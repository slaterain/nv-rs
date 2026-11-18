//! Tweaks' rows: each change it makes to the game's code, its bit
//! ([`engine_core::extend::stewie`]), its number in `measure/stewie/PLAN.md`, its off key in
//! Fallout.ini's `[nv-rs Stewie]`, and Tweaks' own `[Tweaks]` key when Tweaks gives it one.
//!
//! A row is on when the host's row `NV_STEWIE` is on, its own key is not `0`, and Tweaks' key (if
//! any) reads as a number other than 0 (`GetLongValue`, [`crate::settings::Settings::long`]).

use crate::settings::Settings;
use engine_core::extend::stewie;

/// One row.
#[derive(Clone, Copy, Debug)]
pub struct Row {
    pub bit: stewie::Row,
    /// Tweaks' function that makes the change (`patches.h`, `main.cpp`'s `WritePatches`).
    pub function: &'static str,
    /// Its row in `measure/stewie/PLAN.md`.
    pub plan: &'static str,
    /// Its key in Fallout.ini's `[nv-rs Stewie]`: `0` turns it off.
    pub key: &'static str,
    /// Tweaks' own switch for it (section, key), when Tweaks gives it one; an always-on fix has none.
    pub tweaks: Option<(&'static str, &'static str)>,
    /// What a player gets, one line. A row that changes how the game plays (not only a crash
    /// removed) says so here.
    pub summary: &'static str,
}

/// Every row built, in PLAN.md's order. A row's key is `b` and Tweaks' function's name without
/// its `patch` prefix.
pub const ROWS: &[Row] = &[
    Row {
        bit: stewie::EXPLOSION_WATER_NULL_CALL,
        function: "fixExplosionWaterNullCall",
        plan: "S0042",
        key: "bFixExplosionWaterNullCall",
        tweaks: None,
        summary: "water reflecting an explosion no longer crashes on an explosion that is being taken apart",
    },
    Row {
        bit: stewie::HANDLE_MULTIBOUNDS_NULL_CELL,
        function: "patchFixHandleMultiboundsCrashWithNullCell",
        plan: "S0064",
        key: "bFixHandleMultiboundsCrashWithNullCell",
        tweaks: None,
        summary: "a reference in a multibound with no cell (being moved or removed) is passed over instead of crashing",
    },
    Row {
        bit: stewie::HANDLE_STEALING_CRASH,
        function: "fixHandleStealingCrash",
        plan: "S0068",
        key: "bFixHandleStealingCrash",
        tweaks: None,
        summary: "taking an item marked stolen that has no owner no longer crashes",
    },
    Row {
        bit: stewie::NULL_PLAYER_CELL_IMPOSTER_CHECK,
        function: "patchNullPlayerCellImposterCheck",
        plan: "S0078",
        key: "bNullPlayerCellImposterCheck",
        tweaks: None,
        summary: "the regions at the player's place are looked up without crashing when no cell is there yet",
    },
    Row {
        bit: stewie::DIALOG_SPEECH_NULL_CHECK,
        function: "patchDialogSpeechNullCheck",
        plan: "S0079",
        key: "bDialogSpeechNullCheck",
        tweaks: None,
        summary: "clearing the speaker after dialogue no longer crashes when the speaker's 3D is not loaded",
    },
    Row {
        bit: stewie::NULL_HIT_LOCATION_NODE_EXPLOSION,
        function: "patchFixNullHitLocationNodeExplosionCrash",
        plan: "S0106",
        key: "bFixNullHitLocationNodeExplosionCrash",
        tweaks: None,
        summary: "a limb blown off a body whose model lacks that limb's node no longer crashes",
    },
    Row {
        bit: stewie::PROCESS_ACTIVATE_NULL_PACKAGE,
        function: "patchProcessActivateAddNullCheckForPackage",
        plan: "S0124",
        key: "bProcessActivateAddNullCheckForPackage",
        tweaks: None,
        summary: "an NPC using something while it has no package no longer crashes",
    },
    Row {
        bit: stewie::PATROL_MISSING_NULL_CHECK,
        function: "patchFixPatrolMissingNullCheck",
        plan: "S0140",
        key: "bFixPatrolMissingNullCheck",
        tweaks: None,
        summary: "an NPC on patrol whose next patrol point is missing waits instead of crashing",
    },
    Row {
        bit: stewie::RARE_NULL_TELEPORT_DOOR,
        function: "patchFixRareNullTeleportDoorCrash",
        plan: "S0169",
        key: "bFixRareNullTeleportDoorCrash",
        tweaks: None,
        summary: "an NPC finding its way through doors no longer crashes on an empty door entry",
    },
    Row {
        bit: stewie::SET_ACTOR_REF_ESSENTIAL_NULL_CHECK,
        function: "patchFixSetActorRefEssentialNullCheck",
        plan: "S0174",
        key: "bFixSetActorRefEssentialNullCheck",
        tweaks: None,
        summary: "a script calling SetActorRefEssential without a reference no longer crashes",
    },
    Row {
        bit: stewie::SAVING_NULL_CELL,
        function: "patchFixSavingNullCellCrash",
        plan: "S0881",
        key: "bFixSavingNullCellCrash",
        tweaks: None,
        summary: "saving with a placed object that has no cell no longer crashes",
    },
];

/// The keys of `[nv-rs Stewie]` and how they turn the rows on.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Keys {
    /// The rows' keys the file sets, as `1` (on) or `0` (off).
    pub rows: Vec<(&'static str, bool)>,
    /// `bMissingKeysAsTweaks=1`: a key the player's Tweaks INI lacks takes Tweaks' code default,
    /// not VNV's value.
    pub as_tweaks: bool,
    /// Lines whose value is not `0` or `1` (`key=value`): the default stands for each.
    pub bad: Vec<String>,
}

/// The switch of [`Keys::as_tweaks`].
pub const AS_TWEAKS_KEY: &str = "bMissingKeysAsTweaks";

impl Keys {
    /// The keys, each looked up with `value` (the section's value of a key, as the host finds a
    /// Fallout.ini key: `game_ini::value`); an empty value is the default.
    pub fn read(value: impl Fn(&str) -> Option<String>) -> Keys {
        let mut keys = Keys::default();
        let mut flag = |key: &'static str| -> Option<bool> {
            let raw = value(key)?;
            match raw.as_str() {
                "" => None,
                "1" => Some(true),
                "0" => Some(false),
                other => {
                    keys.bad.push(format!("{key}={other}"));
                    None
                }
            }
        };
        let as_tweaks = flag(AS_TWEAKS_KEY);
        let rows: Vec<(&'static str, bool)> = ROWS
            .iter()
            .filter_map(|r| flag(r.key).map(|on| (r.key, on)))
            .collect();
        keys.as_tweaks = as_tweaks.unwrap_or(false);
        keys.rows = rows;
        keys
    }

    /// Whether `row`'s own key leaves it on (no key: on).
    pub fn row_on(&self, row: &Row) -> bool {
        self.rows
            .iter()
            .find(|(k, _)| *k == row.key)
            .map_or(true, |(_, on)| *on)
    }
}

/// Whether `row` is on with these keys and settings (the host's row being on).
pub fn on(row: &Row, keys: &Keys, settings: &Settings) -> bool {
    keys.row_on(row)
        && row
            .tweaks
            .map_or(true, |(section, key)| settings.long(section, key, 0) != 0)
}
