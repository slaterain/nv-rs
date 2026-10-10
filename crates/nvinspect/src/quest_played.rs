//! How far each base-game quest has been played in the nv-rs viewer: the
//! hand-kept half of `nvinspect coverage quests` (the rest is read from the
//! data and nv-rs's tables). Add a row, or change one, when a route is
//! played; give the note in your own words (no game text) and where the
//! evidence is. Quests without a row are NOT PLAYED, or NEVER STARTED when
//! nothing in the data starts them.
//!
//! Status words as `docs/DEAD_MONEY_COVERAGE.md` defines them: PLAYED (its
//! end reached by its own scripts, triggers and dialogue), PARTIAL (some of
//! it played, the rest helped along with console lines or not reached),
//! NOT PLAYED, NEVER STARTED (nothing starts it, so the original never
//! plays it either). `[G]` marks a guess, `[C]` something to check against
//! the original game.

/// A quest's played status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum PlayStatus {
    Played,
    Partial,
    NotPlayed,
    NeverStarted,
}

impl PlayStatus {
    pub fn label(self) -> &'static str {
        match self {
            PlayStatus::Played => "PLAYED",
            PlayStatus::Partial => "PARTIAL",
            PlayStatus::NotPlayed => "NOT PLAYED",
            PlayStatus::NeverStarted => "NEVER STARTED",
        }
    }
}

/// One quest's recorded status.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Entry {
    /// The quest's editor ID.
    pub quest: &'static str,
    pub status: PlayStatus,
    pub note: &'static str,
    /// Where the evidence is (a doc, a route in `scripts/acceptance.ps1`).
    pub source: &'static str,
}

const fn e(
    quest: &'static str,
    status: PlayStatus,
    note: &'static str,
    source: &'static str,
) -> Entry {
    Entry {
        quest,
        status,
        note,
        source,
    }
}

use PlayStatus::{NotPlayed, Partial};

pub const ENTRIES: &[Entry] = &[
    e(
        "VCG00",
        Partial,
        "The opening. Nothing in the scripts or the exe's data was found to start it (the two \
         packages that set its stages belong to its own scene); the viewer's `--new-game` sets its \
         stage 0, whose script asks for the intro movie and moves on to Doc Mitchell's quest. The \
         movie plays (`--new-game --movies`: `FNVIntro.bik`, seen in the maintainer's merge check). \
         [C] How the original starts it.",
        "docs/OPENING.md (observed baseline), docs/MOVIES.md, docs/CONTRIB_CHAZM.md, \
         docs/CLAUDE_REFERENCE.md",
    ),
    e(
        "VCG01",
        Partial,
        "Doc Mitchell's opening. Played by its own scripts and dialogue from a new game to stage 55 (Doc \
         asks for the vigor tester; the face menu accepts itself); from a stage 55 save, the tester \
         trigger, stage 60 and the SPECIAL menu; from `--stage VCG01 110`, Doc walking to his door and \
         talking (acceptance route `doc`); the farewell's follow-up chain to the open door was walked \
         on the real data by a throwaway program. Not played in one run end to end: the face editor, \
         the name entry to the door, the questionnaire on the couch.",
        "docs/OPENING.md, docs/PLAYTESTING.md, docs/DIALOGUE.md, scripts/acceptance.ps1 route doc",
    ),
    e(
        "VCG02",
        Partial,
        "Completed once (XP, caps, VCG03 started) with the start, the bottle hits, the gecko kills at the \
         first and third wells, the player's following and the reward request as console lines; \
         Sunny's walks, the patrol trigger, her barks and the stage 30/40 package end actions ran by \
         themselves. Blocker: talking to Sunny can't start it (her greeting comes from this quest, not \
         from VFreeformGoodsprings) [C].",
        "docs/GOODSPRINGS_ROUTE.md, scripts/acceptance.ps1 route vcg02",
    ),
    e(
        "VCG03",
        Partial,
        "Only its start: VCG02's reward sets stage 10 and Sunny sets off on her walk to the campfire. \
         Nothing after that was played.",
        "docs/GOODSPRINGS_ROUTE.md",
    ),
    e(
        "VMS16",
        Partial,
        "Reached stage 100 with Trudy's help (the gangers come in and die, XP, fame and infamy); the \
         dialogue choices that lead there were replayed as console lines, and the player didn't fight \
         (the townspeople killed the gang). Not played: the dialogue picked in the menu, the branch \
         without help, the player's own fight.",
        "docs/GOODSPRINGS_ROUTE.md, scripts/acceptance.ps1 route vms16",
    ),
    e(
        "VMS16b",
        NotPlayed,
        "The Powder Gangers' side of the gunfight: not tried (the gunfight route ran only Trudy's side).",
        "docs/GOODSPRINGS_ROUTE.md",
    ),
    e(
        "CGTutorial",
        Partial,
        "Seen in passing during the VCG02 runs: its stages moved with Sunny's lessons and its V.A.T.S. \
         hint kept repeating while it stayed at stage 70 (the data's own timer [G]). Not watched on \
         purpose.",
        "docs/GOODSPRINGS_ROUTE.md (blockers left, item 5)",
    ),
    e(
        "VDLCPackQuest",
        Partial,
        "In the `--new-game` run its four pack notices came up and were accepted; whether the items \
         arrived wasn't recorded.",
        "docs/OPENING.md (observed baseline)",
    ),
    e(
        "VFreeformGoodsprings",
        NotPlayed,
        "Its variables were set by console lines in the Goodsprings routes. Sunny's line from this quest \
         that should start VCG02 isn't picked (see VCG02).",
        "docs/GOODSPRINGS_ROUTE.md",
    ),
    e(
        "nVPrimmDeputyConv",
        Partial,
        "Only the Primm deputy route in PR #29, which is still open, is recorded: pending review, so \
         its evidence is not merged and the route is not proof of coverage. That route jumps the \
         player with five `player.MoveTo` console lines and opens every conversation with a console \
         `StartConversation`; the quest's own scripts, its stages and the deputy's dialogue were not \
         driven without them. Nothing else of the quest was tried.",
        "PR #29 (open, not merged); docs/GOODSPRINGS_ROUTE.md is the route model",
    ),
];

/// A system nv-rs doesn't have yet, and the script functions that need it.
/// For functions nv-rs answers (so they aren't counted missing) whose
/// screen, playback or game behind them isn't built; functions nv-rs
/// doesn't carry out at all are found by the generator itself. Remove a
/// row when the system is built.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SystemGap {
    pub system: &'static str,
    /// The game's names of the functions that need it.
    pub functions: &'static [&'static str],
    /// Record types that need it for the quest they name (a casino,
    /// `CSNO`, for its winnings quest).
    pub records: &'static [&'static str],
    pub note: &'static str,
    /// Where that is shown (code or doc).
    pub source: &'static str,
}

pub const SYSTEM_GAPS: &[SystemGap] = &[SystemGap {
    system: "face editor",
    functions: &["ShowRaceMenu"],
    records: &[],
    note: "the race and face menu isn't built: it accepts itself and the face is kept",
    source: "docs/TASKS.md B15, viewer/src/scripts.rs",
}];

/// A quest's recorded status, by editor ID (ignoring case).
pub fn entry(editor_id: &str) -> Option<&'static Entry> {
    ENTRIES
        .iter()
        .find(|e| e.quest.eq_ignore_ascii_case(editor_id))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_entry_is_unique_and_has_its_evidence() {
        for (i, a) in ENTRIES.iter().enumerate() {
            assert!(!a.note.is_empty() && !a.source.is_empty(), "{}", a.quest);
            assert!(
                ENTRIES[i + 1..]
                    .iter()
                    .all(|b| !b.quest.eq_ignore_ascii_case(a.quest)),
                "{} twice",
                a.quest
            );
            // Notes are table cells.
            assert!(
                !a.note.contains('|') && !a.note.contains('\n'),
                "{}",
                a.quest
            );
        }
        assert_eq!(entry("vcg02").map(|e| e.status), Some(PlayStatus::Partial));
        // Gap functions are the game's own names.
        for g in SYSTEM_GAPS {
            for f in g.functions {
                assert!(
                    script::function(f).is_some_and(|(_, s)| s.name == *f),
                    "{f}"
                );
            }
            assert!(
                !g.note.contains('|') && !g.source.is_empty(),
                "{}",
                g.system
            );
        }
        assert!(entry("NoSuchQuest").is_none());
    }
}
