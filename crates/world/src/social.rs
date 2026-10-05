//! People talking without the player starting it, read from
//! `FalloutNV.exe` with Ghidra (`%USERPROFILE%\nv-re\findings\
//! ai_rules.md` §5b and §6):
//!
//! - Greetings ([`Social::greets`]): someone who notices the player within
//!   `fAIMinGreetingDistance` (150) says a line from `HELLO`, no dialogue
//!   menu, and turns to them; then not again for `fAIGreetingTimer` (20 s).
//! - Idle chatter ([`Social::chatter_due`]): a line from `IdleChatter`
//!   every 15–60 s (the data's `fIdleChatterCommentTimer…`).
//! - Conversations between two people ([`Social::start_conversation`],
//!   [`conversation`]): within 200, when their timer (30–60 s) has run out,
//!   a 25 % chance each; the lines are worked out at the start from
//!   `HELLO`, following each line's topics at random, who speaks next by the
//!   line's own rule.
//!
//! Saying the lines (the voice, the face) is the viewer's.

use esm::{FormId, LoadOrder};

use crate::dialogue::{topic_lines, Info, Speaker};
use crate::scripting::{game_setting, Facts, GameState};

/// The default topics of kind 1 (conversation), by the game's table
/// (`0061a2d0(1, n)`, tables `0119a41c`/`0119a43c`): fixed forms in every
/// game.
pub mod topics {
    use esm::FormId;
    /// 0: `HELLO` ("Hello").
    pub const HELLO: FormId = FormId(0xD2);
    /// 1: `ANY`.
    pub const ANY: FormId = FormId(0xD3);
    /// 2: `GOODBYE` ("Goodbye.").
    pub const GOODBYE: FormId = FormId(0xD4);
    /// 3: `IdleChatter`.
    pub const IDLE_CHATTER: FormId = FormId(0xD5);
}

/// The settings greetings and conversations read: the data's (`GMST`), else
/// the exe's defaults.
#[derive(Debug, Clone, PartialEq)]
pub struct SocialSettings {
    /// `fAIMinGreetingDistance` (150).
    pub greeting_distance: f32,
    /// `fAIGreetingTimer` (20 s).
    pub greeting_timer: f32,
    /// `fIdleChatterCommentTimer` and `…Max` (exe 5 and 30, data 15 and 60).
    pub chatter: (f32, f32),
    /// `fAISocialTimerForConversationsMin` and `…Max` (exe 5 and 30, data 30
    /// and 60).
    pub conversation_timer: (f32, f32),
    /// `fAItalktosameNPCtimer` (120 s).
    pub same_npc: f32,
    /// `fAISocialRadiusToTriggerConversation` (2000) and `…Interior` (exe
    /// 500, data 1000).
    pub radius: f32,
    pub radius_interior: f32,
    /// `fAISocialchanceForConversation` (exe 60, data 25) and `…Interior`
    /// (25).
    pub chance: f32,
    pub chance_interior: f32,
}

impl SocialSettings {
    pub fn read(order: &LoadOrder) -> SocialSettings {
        let g = |name: &str, default: f32| game_setting(order, name).unwrap_or(default);
        SocialSettings {
            greeting_distance: g("fAIMinGreetingDistance", 150.0),
            greeting_timer: g("fAIGreetingTimer", 20.0),
            chatter: (
                g("fIdleChatterCommentTimer", 5.0),
                g("fIdleChatterCommentTimerMax", 30.0),
            ),
            conversation_timer: (
                g("fAISocialTimerForConversationsMin", 5.0),
                g("fAISocialTimerForConversationsMax", 30.0),
            ),
            same_npc: g("fAItalktosameNPCtimer", 120.0),
            radius: g("fAISocialRadiusToTriggerConversation", 2000.0),
            radius_interior: g("fAISocialRadiusToTriggerConversationInterior", 500.0),
            chance: g("fAISocialchanceForConversation", 60.0),
            chance_interior: g("fAISocialchanceForConversationInterior", 25.0),
        }
    }

    /// How near two people must be to start talking (`00904800`): 200,
    /// unless the place's social radius (outdoors' or indoors') is below
    /// 200, when it's the outdoor setting's value (as the code reads it).
    pub fn trigger_distance(&self, interior: bool) -> f32 {
        let place = if interior {
            self.radius_interior
        } else {
            self.radius
        };
        if place < 200.0 {
            self.radius
        } else {
            200.0
        }
    }

    /// The chance in 100 of starting a conversation (`00904800`).
    pub fn chance(&self, interior: bool) -> f32 {
        if interior {
            self.chance_interior
        } else {
            self.chance
        }
    }
}

/// A random number between two (`006465f0`), from `unit` in 0..1.
fn between((lo, hi): (f32, f32), unit: f32) -> f32 {
    lo + (hi - lo) * unit
}

/// One person's social timers (their high process): the greeting timer
/// (+0x330), idle chatter (+0x298), conversations (+0x2c8), the same-pair
/// timer (+0x2a0) and the people they've talked with lately.
#[derive(Debug, Clone, PartialEq)]
pub struct Social {
    pub greeting: f32,
    pub chatter: f32,
    pub conversation: f32,
    pub pair: f32,
    pub partners: Vec<FormId>,
}

impl Social {
    /// As a process starts (`008d7510`): the chatter and conversation
    /// timers at random within their settings, the pair timer at
    /// `fAItalktosameNPCtimer`, no greeting wait.
    pub fn new(settings: &SocialSettings, unit: &mut dyn FnMut() -> f32) -> Social {
        Social {
            greeting: 0.0,
            chatter: between(settings.chatter, unit()),
            conversation: between(settings.conversation_timer, unit()),
            pair: settings.same_npc,
            partners: Vec::new(),
        }
    }

    /// Whether someone greets the player now (`008eeec0`): they detect the
    /// player (above 0), are free to (not fighting, not in a conversation,
    /// … the caller's `free`), the player is within `fAIMinGreetingDistance`
    /// and their greeting timer has run out. The line is from `HELLO`; then
    /// the timer is set ([`Self::greeted`]).
    pub fn greets(
        &self,
        detection: i32,
        free: bool,
        distance: f32,
        settings: &SocialSettings,
    ) -> bool {
        free && detection > 0 && distance <= settings.greeting_distance && self.greeting <= 0.0
    }

    /// A line was said to the player, or the player's dialogue with them
    /// ended (`008dbe30`, `00762160`): no greeting for `fAIGreetingTimer`.
    pub fn greeted(&mut self, settings: &SocialSettings) {
        self.greeting = settings.greeting_timer;
    }

    /// The frame's time passes on the greeting timer.
    pub fn tick(&mut self, dt: f32) {
        if self.greeting > 0.0 {
            self.greeting -= dt;
        }
    }

    /// The idle chatter timer (`008eeec0`): while it runs it counts down;
    /// once out, if they have no package or it isn't a dialogue package
    /// (type 15), a line from `IdleChatter` is due and the timer starts again
    /// at random within `fIdleChatterCommentTimer…`. (The code also asks the
    /// package whether it allows chatter, `0067abd0`, not traced: taken as
    /// yes.) Whether a line is due.
    pub fn chatter_due(
        &mut self,
        dt: f32,
        package_kind: Option<u8>,
        settings: &SocialSettings,
        unit: &mut dyn FnMut() -> f32,
    ) -> bool {
        if self.chatter > 0.0 {
            self.chatter -= dt;
            return false;
        }
        if package_kind == Some(crate::ai::kinds::DIALOGUE) {
            return false;
        }
        self.chatter = between(settings.chatter, unit());
        true
    }

    /// Someone looks for another to talk to (`00904800`, each high-process
    /// update): `allowed` says their package lets them (not sleep, use item
    /// at, ambush, guard, dialogue or use weapon packages; a sandbox only
    /// when its "no conversation" flag is clear; the caller's) and `only`
    /// limits them to their follow, escort or accompany target. When the
    /// conversation timer has run out, each candidate (others who can talk
    /// both ways, not the player, a sandboxer only if its sandbox allows
    /// it) within the trigger distance, in order: the same-pair timer (reset
    /// to `fAItalktosameNPCtimer` while nobody is remembered, counting down
    /// by the frame for each candidate in reach otherwise) clears the
    /// remembered partners when it runs out; someone remembered is passed
    /// over; else a roll (`rand() % 100`) below the chance starts it, and
    /// both remember each other. Then the timer is set at random within
    /// `fAISocialTimerForConversations…`. Whom they start talking to.
    #[allow(clippy::too_many_arguments)]
    pub fn start_conversation(
        &mut self,
        me: FormId,
        allowed: bool,
        only: Option<FormId>,
        candidates: &[(FormId, f32)],
        interior: bool,
        dt: f32,
        settings: &SocialSettings,
        roll: &mut dyn FnMut() -> u64,
        unit: &mut dyn FnMut() -> f32,
    ) -> Option<FormId> {
        if self.conversation > 0.0 {
            self.conversation -= dt;
            return None;
        }
        if !allowed {
            return None;
        }
        let reach = settings.trigger_distance(interior);
        let chance = settings.chance(interior);
        let mut chosen = None;
        if let Some(target) = only {
            chosen = Some(target);
        } else {
            for &(other, distance) in candidates {
                if other == me || other == crate::dialogue::PLAYER_REF || distance > reach {
                    continue;
                }
                if self.partners.is_empty() {
                    self.pair = settings.same_npc;
                } else {
                    self.pair -= dt;
                }
                if self.pair <= 0.0 {
                    self.partners.clear();
                }
                let rolled = (roll() % 100) as f32;
                if self.partners.contains(&other) {
                    continue;
                }
                if rolled < chance {
                    self.partners.push(other);
                    chosen = Some(other);
                    break;
                }
            }
        }
        let other = chosen?;
        self.conversation = between(settings.conversation_timer, unit());
        Some(other)
    }
}

/// A line's "next speaker" (`INFO` `DATA` byte 1, the line's +0x24 read by
/// `0061b9f0`): 0 the other, 1 the same, 2 either (a coin flip).
pub fn next_speaker(order: &LoadOrder, info: FormId) -> u8 {
    order
        .get(info)
        .and_then(|rr| rr.record().ok())
        .and_then(|r| r.get(esm::sig::DATA).and_then(|s| s.data.get(1).copied()))
        .unwrap_or(0)
}

/// The first line of a topic `speaker` can say to `listener` now
/// (`0061b320`): as [`crate::dialogue::pick`], its conditions asked with
/// the listener as their target; lines already in this conversation
/// (`said`) aren't said again (the code is given the list; that it skips
/// them is inferred).
pub fn pick_for(
    order: &LoadOrder,
    topic: FormId,
    speaker: &Speaker,
    listener: FormId,
    state: &GameState,
    said: &[FormId],
) -> Option<Info> {
    let facts = Facts {
        order,
        state,
        speaker: Some(speaker),
    };
    let mut quest_ok: std::collections::HashMap<FormId, bool> = Default::default();
    topic_lines(order, topic).into_iter().find(|info| {
        if info.responses.is_empty()
            || said.contains(&info.form_id)
            || (info.flags & crate::dialogue::SAY_ONCE != 0 && state.said.contains(&info.form_id))
        {
            return false;
        }
        if let Some(q) = info.quest {
            let ok = *quest_ok.entry(q).or_insert_with(|| {
                state.running.contains(&q)
                    && facts.conditions_pass(
                        &crate::quest::quest_conditions(order, q),
                        speaker.reference,
                        listener,
                    )
            });
            if !ok {
                return false;
            }
        }
        facts.conditions_pass(&info.conditions, speaker.reference, listener)
    })
}

/// One line of a conversation between two people: who says it, to whom.
#[derive(Debug, Clone, PartialEq)]
pub struct Line {
    pub speaker: FormId,
    pub listener: FormId,
    pub info: Info,
}

/// The most lines a conversation holds (`0061b440` stops at 100).
pub const MOST_LINES: usize = 100;

/// A conversation's lines, worked out as it starts (`0061b440`): from the
/// topic given, else `HELLO`, `a` speaking first. Each step: the first line
/// the speaker can say on the topic ([`pick_for`]); with none, when no topic
/// was given or the last line keeps the same speaker, `GOODBYE` instead;
/// still none, the end. Then one of the line's follow-ups (`TCLT`) at random
/// (`rand() % count`, each tried once), the speakers swapped by the line's
/// rule ([`next_speaker`]: 0 swap, 1 keep, 2 a coin flip, at every try) until
/// one has a line; a line without follow-ups ends it, and when none of them
/// has a line the last tried is looked at again (and so to `GOODBYE`). At
/// most [`MOST_LINES`].
pub fn conversation(
    order: &LoadOrder,
    state: &GameState,
    a: &Speaker,
    b: &Speaker,
    topic: Option<FormId>,
    roll: &mut dyn FnMut() -> u64,
) -> Vec<Line> {
    let mut lines: Vec<Line> = Vec::new();
    let (mut speaker, mut listener) = (a, b);
    let mut current = topic.unwrap_or(topics::HELLO);
    let mut last_rule: Option<u8> = None;
    let mut said: Vec<FormId> = Vec::new();
    loop {
        if lines.len() >= MOST_LINES {
            break;
        }
        let mut line = pick_for(order, current, speaker, listener.reference, state, &said);
        if line.is_none() && (topic.is_none() || last_rule == Some(1)) {
            current = topics::GOODBYE;
            line = pick_for(order, current, speaker, listener.reference, state, &said);
        }
        let Some(info) = line else {
            break;
        };
        let rule = next_speaker(order, info.form_id);
        let follow_ups = info.choices.clone();
        said.push(info.form_id);
        lines.push(Line {
            speaker: speaker.reference,
            listener: listener.reference,
            info,
        });
        last_rule = Some(rule);
        if follow_ups.is_empty() {
            break;
        }
        let mut tried = vec![false; follow_ups.len()];
        let mut next: Option<FormId> = None;
        let mut last_tried = None;
        loop {
            // The code rolls again until it meets one not yet tried: one of
            // those, evenly.
            let untried: Vec<usize> = (0..tried.len()).filter(|&i| !tried[i]).collect();
            if untried.is_empty() {
                break;
            }
            let k = untried[(roll() % untried.len() as u64) as usize];
            tried[k] = true;
            let swap = match rule {
                0 => true,
                2 => roll() & 1 == 0,
                _ => false,
            };
            if swap {
                std::mem::swap(&mut speaker, &mut listener);
            }
            last_tried = Some(follow_ups[k]);
            if pick_for(
                order,
                follow_ups[k],
                speaker,
                listener.reference,
                state,
                &said,
            )
            .is_some()
            {
                next = Some(follow_ups[k]);
                break;
            }
        }
        match next.or(last_tried) {
            Some(t) => current = t,
            None => break,
        }
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    fn settings() -> SocialSettings {
        SocialSettings {
            greeting_distance: 150.0,
            greeting_timer: 20.0,
            chatter: (15.0, 60.0),
            conversation_timer: (30.0, 60.0),
            same_npc: 120.0,
            radius: 2000.0,
            radius_interior: 1000.0,
            chance: 25.0,
            chance_interior: 25.0,
        }
    }

    #[test]
    fn greetings_wait_for_the_timer_and_the_distance() {
        let s = settings();
        let mut social = Social::new(&s, &mut || 0.5);
        assert_eq!(social.chatter, 37.5);
        assert_eq!(social.conversation, 45.0);
        assert!(social.greets(10, true, 140.0, &s));
        assert!(!social.greets(0, true, 140.0, &s));
        assert!(!social.greets(10, true, 160.0, &s));
        assert!(!social.greets(10, false, 140.0, &s));
        social.greeted(&s);
        assert!(!social.greets(10, true, 140.0, &s));
        for _ in 0..200 {
            social.tick(0.1);
        }
        assert!(social.greets(10, true, 140.0, &s));
    }

    #[test]
    fn idle_chatter_comes_round_at_random_but_not_in_dialogue_packages() {
        let s = settings();
        let mut social = Social::new(&s, &mut || 0.0);
        assert_eq!(social.chatter, 15.0);
        let mut t = 0.0f32;
        while !social.chatter_due(0.5, None, &s, &mut || 1.0) {
            t += 0.5;
        }
        assert!((t - 15.0).abs() < 0.6, "{t}");
        assert_eq!(social.chatter, 60.0);
        social.chatter = 0.0;
        assert!(!social.chatter_due(0.5, Some(crate::ai::kinds::DIALOGUE), &s, &mut || 1.0));
    }

    #[test]
    fn conversations_start_within_200_by_chance_and_not_twice_with_one_person() {
        let s = settings();
        assert_eq!(s.trigger_distance(true), 200.0);
        let me = FormId(1);
        let mut social = Social::new(&s, &mut || 0.0);
        social.conversation = 0.0;
        let mut rolls = [80u64, 10, 10, 10].into_iter();
        let mut roll = || rolls.next().unwrap_or(99);
        // The first candidate is too far; the second's roll fails (80); the
        // third's passes (10).
        let candidates = [(FormId(2), 250.0), (FormId(3), 150.0), (FormId(4), 150.0)];
        let got = social.start_conversation(
            me,
            true,
            None,
            &candidates,
            true,
            0.1,
            &s,
            &mut roll,
            &mut || 0.0,
        );
        assert_eq!(got, Some(FormId(4)));
        assert_eq!(social.conversation, 30.0);
        assert_eq!(social.partners, vec![FormId(4)]);
        // Not again for the timer; then not with the same person.
        social.conversation = 0.0;
        let mut always = || 0u64;
        let got = social.start_conversation(
            me,
            true,
            None,
            &[(FormId(4), 100.0)],
            true,
            0.1,
            &s,
            &mut always,
            &mut || 0.0,
        );
        assert_eq!(got, None);
        // A package that forbids it.
        let got = social.start_conversation(
            me,
            false,
            None,
            &[(FormId(5), 100.0)],
            true,
            0.1,
            &s,
            &mut always,
            &mut || 0.0,
        );
        assert_eq!(got, None);
    }
}
