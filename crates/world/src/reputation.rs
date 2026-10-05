//! Reputation (`REPU`) and karma, read from the game's code
//! (`%USERPROFILE%\nv-re\findings\reputation.md`).
//!
//! A reputation's `DATA` f32 is its most (NCR 80, Legion 100, Goodsprings
//! 15); the game keeps fame and infamy for each, both from 0. Scripts:
//! `AddReputation rep type n` / `RemoveReputation` move fame (type 1) or
//! infamy (type 0) by `fReputationBump…` (n 1–5: 1, 2, 4, 7, 12), clamped
//! to 0..most; `…Exact` by the amount given; `SetReputation` sets it
//! (no clamp, no notice); `GetReputation` the value, `GetReputationPct`
//! value / most (a fraction). Each axis's level from value / most: 1 from
//! 0.15, 2 from 0.5, 3 at the most (`fReputationThreshold…`); the title is
//! by infamy level × 4 + fame level (`sRepTitlePos<F>Neg<I>`), and
//! `GetReputationThreshold` answers per axis as `00616a90` does. Every
//! change shows "<name>\n<Fame Gained!…>"; a changed level shows the new
//! title.
//!
//! Karma (actor value 23): `RewardKarma` (always the player) clamped to
//! ±1000 (`iKarmaMin`/`Max`), with the game's notice; bands very evil ≤
//! −750, evil ≤ −250, good ≥ 250, very good ≥ 750. The player's kills of
//! anyone in a faction that tracks crime (`FACT` `DATA` flag 0x100) change
//! it by the victim's own karma: good −50, very good −100, evil +100, very
//! evil +2 (`fKarmaMod…`).

use esm::{FormId, FourCC, LoadOrder};

use crate::cell::{le_f32, le_u32};
use crate::dialogue::PLAYER_REF;
use crate::scripting::{game_setting, game_setting_text, Event, GameState};

const REPU: FourCC = FourCC::new(b"REPU");

/// The karma actor value.
pub const KARMA: u16 = 23;

/// The axes: type 1 fame, 0 infamy.
pub const FAME: u8 = 1;
pub const INFAMY: u8 = 0;

/// A reputation's most and name.
pub fn reputation_max(order: &LoadOrder, rep: FormId) -> Option<f32> {
    let record = order
        .get(rep)
        .filter(|r| r.entry.header.kind == REPU)?
        .record()
        .ok()?;
    record
        .get(esm::sig::DATA)
        .filter(|s| s.data.len() >= 4)
        .map(|s| le_f32(&s.data, 0))
}

fn reputation_name(order: &LoadOrder, rep: FormId) -> String {
    order
        .get(rep)
        .and_then(|r| r.record().ok())
        .and_then(|r| r.full_name())
        .unwrap_or_else(|| rep.to_string())
}

/// Fame (type 1) or infamy (type 0) now.
pub fn get(state: &GameState, rep: FormId, kind: u8) -> f32 {
    let (fame, infamy) = state.reputations.get(&rep).copied().unwrap_or((0.0, 0.0));
    if kind == FAME {
        fame
    } else {
        infamy
    }
}

fn set_raw(state: &mut GameState, rep: FormId, kind: u8, value: f32) {
    let e = state.reputations.entry(rep).or_insert((0.0, 0.0));
    if kind == FAME {
        e.0 = value;
    } else {
        e.1 = value;
    }
}

/// An axis's level (0–3) for a value (`00616950`).
pub fn level(order: &LoadOrder, value: f32, max: f32) -> u8 {
    let x = if max > 0.0 { value / max } else { 0.0 };
    let t = |n: &str, d: f32| game_setting(order, n).unwrap_or(d);
    if x >= t("fReputationThresholdThree", 1.0) {
        3
    } else if x >= t("fReputationThresholdTwo", 0.5) {
        2
    } else if x >= t("fReputationThresholdOne", 0.15) {
        1
    } else {
        0
    }
}

/// Both levels of a reputation now: (fame, infamy).
pub fn levels(order: &LoadOrder, state: &GameState, rep: FormId) -> (u8, u8) {
    let max = reputation_max(order, rep).unwrap_or(1.0);
    (
        level(order, get(state, rep, FAME), max),
        level(order, get(state, rep, INFAMY), max),
    )
}

/// The title for fame and infamy levels (`sRepTitlePos<F>Neg<I>`; the
/// exe's own names where the data has none).
pub fn title(order: &LoadOrder, fame: u8, infamy: u8) -> String {
    const WORDS: [&str; 4] = ["None", "One", "Two", "Three"];
    // The exe's defaults, by infamy × 4 + fame.
    const DEFAULTS: [&str; 16] = [
        "Neutral",
        "Accepted",
        "Liked",
        "Idolized",
        "Shunned",
        "Mixed",
        "Smiling Troublemaker",
        "Good Natured Rascal",
        "Hated",
        "Sneering Punk",
        "Unpredictable",
        "Dark Hero",
        "Vilified",
        "Merciful Thug",
        "Soft-Hearted Devil",
        "Wild Child",
    ];
    let (f, i) = (fame.min(3), infamy.min(3));
    game_setting_text(
        order,
        &format!(
            "sRepTitlePos{}Neg{}",
            WORDS[usize::from(f)],
            WORDS[usize::from(i)]
        ),
    )
    .unwrap_or_else(|| DEFAULTS[usize::from(i) * 4 + usize::from(f)].to_string())
}

/// `GetReputationThreshold rep axis` (`00616a90`): 1 when neither is
/// known; else by axis 0 (mixed), 1 (good), 2 (bad) as the game's table
/// has it, 0 off the axis.
pub fn threshold(order: &LoadOrder, state: &GameState, rep: FormId, axis: u8) -> f32 {
    let (f, i) = levels(order, state, rep);
    if (f, i) == (0, 0) {
        return 1.0;
    }
    let v = match axis {
        0 => match (f, i) {
            (1, 1) => 3,
            (2, 2) => 4,
            (3, 3) => 5,
            (3, 2) | (2, 3) => 2,
            _ => 0,
        },
        1 => match (f, i) {
            (1, 0) => 4,
            (2, 0) => 5,
            (3, 0) => 6,
            (2, 1) => 2,
            (3, 1) => 3,
            _ => 0,
        },
        2 => match (f, i) {
            (0, 1) => 4,
            (0, 2) => 5,
            (0, 3) => 6,
            (1, 2) => 2,
            (1, 3) => 3,
            _ => 0,
        },
        _ => 0,
    };
    v as f32
}

/// Moves fame or infamy by `by` (clamped to 0..most), with the game's
/// notice, and the new title when the axis's level changed.
pub fn change(order: &LoadOrder, state: &mut GameState, rep: FormId, kind: u8, by: f32) {
    let Some(max) = reputation_max(order, rep) else {
        return;
    };
    let before = levels(order, state, rep);
    let now = (get(state, rep, kind) + by).clamp(0.0, max);
    set_raw(state, rep, kind, now);
    let words = match (kind == FAME, by >= 0.0) {
        (true, true) => ("sRepPositiveGain", "Fame Gained!"),
        (false, true) => ("sRepNegativeGain", "Infamy Gained!"),
        (true, false) => ("sRepPositiveLoss", "Fame Reduced"),
        (false, false) => ("sRepNegativeLoss", "Infamy Reduced"),
    };
    let name = reputation_name(order, rep);
    let text = game_setting_text(order, words.0).unwrap_or_else(|| words.1.into());
    state.events.push(Event::Message {
        title: None,
        text: format!("{name}\n{text}"),
        buttons: Vec::new(),
    });
    let after = levels(order, state, rep);
    if after != before {
        let title = title(order, after.0, after.1);
        state.events.push(Event::Message {
            title: Some(name),
            text: title,
            buttons: Vec::new(),
        });
    }
}

/// `AddReputation` / `RemoveReputation`'s size (1–5) as points
/// (`fReputationBump…`: 1, 2, 4, 7, 12).
pub fn bump(order: &LoadOrder, size: i32) -> Option<f32> {
    let (name, default) = match size {
        1 => ("fReputationBumpVeryMinor", 1.0),
        2 => ("fReputationBumpMinor", 2.0),
        3 => ("fReputationBumpAverage", 4.0),
        4 => ("fReputationBumpMajor", 7.0),
        5 => ("fReputationBumpVeryMajor", 12.0),
        _ => return None,
    };
    Some(game_setting(order, name).unwrap_or(default))
}

/// `SetReputation`: the value as given, no clamp, no notice.
pub fn set(state: &mut GameState, rep: FormId, kind: u8, value: f32) {
    set_raw(state, rep, kind, value);
}

// ---------------------------------------------------------------------------
// Karma

/// Someone's karma band (`0047e040`): 4 very evil (≤ −750), 2 evil (≤
/// −250), 1 neutral, 0 good (≥ 250), 3 very good (≥ 750).
pub fn alignment(order: &LoadOrder, karma: f32) -> u8 {
    let s = |n: &str, d: f32| game_setting(order, n).unwrap_or(d);
    if karma <= s("fAlignVeryEvilMaxKarma", -750.0) {
        4
    } else if karma <= s("fAlignEvilMaxKarma", -250.0) {
        2
    } else if karma < s("fAlignGoodMinKarma", 250.0) {
        1
    } else if karma < s("fAlignVeryGoodMinKarma", 750.0) {
        0
    } else {
        3
    }
}

/// The player's karma.
pub fn karma(order: &LoadOrder, state: &GameState) -> f32 {
    crate::scripting::Facts {
        order,
        state,
        speaker: None,
    }
    .current_actor_value(PLAYER_REF, KARMA)
    .unwrap_or(0.0) as f32
}

/// The player's karma changes (`0094fd30`): clamped to `iKarmaMin` …
/// `iKarmaMax` (±1000), with the game's notice (`sKarmaMajor/Minor
/// Gained/Lost`, major past ±`iKarmaChangeThreshold` 250).
pub fn reward_karma(order: &LoadOrder, state: &mut GameState, amount: i32) {
    let s = |n: &str, d: f32| game_setting(order, n).unwrap_or(d) as i32;
    let k = karma(order, state) as i32;
    let (min, max) = (s("iKarmaMin", -1000.0), s("iKarmaMax", 1000.0));
    let mut a = amount;
    if a < 0 && k + a < min {
        a = min - k;
    }
    if a >= 0 && k + a > max {
        a = max - k;
    }
    let major = s("iKarmaChangeThreshold", 250.0);
    let (name, fallback) = if a < -major {
        ("sKarmaMajorLost", "You've lost Karma!")
    } else if a < 0 {
        ("sKarmaMinorLost", "You've lost Karma!")
    } else if a < major {
        ("sKarmaMinorGained", "You've gained Karma!")
    } else {
        ("sKarmaMajorGained", "You've gained Karma!")
    };
    state.events.push(Event::Message {
        title: None,
        text: game_setting_text(order, name).unwrap_or_else(|| fallback.into()),
        buttons: Vec::new(),
    });
    if (a > 0 && k >= max) || (a < 0 && k <= min) {
        return;
    }
    *state
        .actor_values
        .entry((PLAYER_REF, KARMA))
        .or_insert(k as f64) += f64::from(a);
}

/// The karma for the player killing someone (`0089e242`): only when they
/// are in a faction that tracks crime (`FACT` `DATA` flag 0x100), by the
/// victim's own record karma's band.
pub fn kill_karma(order: &LoadOrder, state: &GameState, victim: FormId) -> i32 {
    let tracks_crime = crate::factions::factions_of(order, state, victim)
        .into_iter()
        .any(|f| {
            order
                .get(f)
                .and_then(|r| r.record().ok())
                .and_then(|r| r.get(esm::sig::DATA).map(|s| s.data.clone()))
                .is_some_and(|d| d.len() >= 4 && le_u32(&d, 0) & 0x100 != 0)
        });
    if !tracks_crime {
        return 0;
    }
    let base = crate::scripting::Facts {
        order,
        state,
        speaker: None,
    }
    .base_actor_value(victim, KARMA)
    .unwrap_or(0.0) as f32;
    let s = |n: &str, d: f32| game_setting(order, n).unwrap_or(d);
    let creature = crate::combat::is_creature(order, victim);
    let amount = match alignment(order, base) {
        0 => s("fKarmaModMurderingGoodNPC", -50.0),
        3 => s("fKarmaModMurderingVeryGoodNPC", -100.0),
        2 => s("fKarmaModKillingEvilActor", 1.0),
        4 => s("fKarmaModKillingVeryEvilActor", 2.0),
        _ if creature => s("fKarmaModMurderingNonEvilCreature", 0.0),
        _ => s("fKarmaModMurderingNonEvilNPC", 0.0),
    };
    amount.trunc() as i32
}

/// The player's karmic title at their level (`0047e0e0`): the band's row
/// (very good uses good's, very evil evil's), `sKarmicTitle<Good|Neutral|
/// Evil><NN>` by level (30 and past the same); only the titles the data
/// sets are known here (the rest are the exe's own).
pub fn karmic_title(order: &LoadOrder, state: &GameState) -> Option<String> {
    let row = match alignment(order, karma(order, state)) {
        0 | 3 => "Good",
        2 | 4 => "Evil",
        _ => "Neutral",
    };
    let level = state.player_level.clamp(1, 30);
    game_setting_text(order, &format!("sKarmicTitle{row}{level:02}"))
}
