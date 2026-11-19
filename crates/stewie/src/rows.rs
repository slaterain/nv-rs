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
/// its `patch` prefix; a row that is one block of a function holding several (`MenuFixes`) adds
/// a word for the block after it.
pub const ROWS: &[Row] = &[
    Row {
        bit: stewie::UGRIDS_LOAD_NULL_CELL,
        function: "fixUGridsCrashAndAllow3AsMinimum",
        plan: "S0018",
        key: "bFixUGridsCrashAndAllow3AsMinimum",
        tweaks: None,
        summary: "loading a save made with more grids than the game now loads no longer crashes; uGridsToLoad 3 is kept",
    },
    Row {
        bit: stewie::INVENTORY_USE_NO_ITEM_JUMP,
        function: "MenuFixes",
        plan: "S0022.14",
        key: "bMenuFixesInventoryUseNoItem",
        tweaks: None,
        summary: "clicking Use in the inventory with nothing selected no longer crashes",
    },
    Row {
        bit: stewie::QUANTITY_MENU_CLOSE_NO_ITEM,
        function: "MenuFixes",
        plan: "S0022.15",
        key: "bMenuFixesQuantityMenuCloseNoItem",
        tweaks: None,
        summary: "closing the quantity menu after the item to drop is gone no longer crashes",
    },
    Row {
        bit: stewie::INVENTORY_SELECT_NO_ITEM,
        function: "MenuFixes",
        plan: "S0022.16",
        key: "bMenuFixesInventorySelectNoItem",
        tweaks: None,
        summary: "equipping from the inventory with nothing selected no longer crashes",
    },
    Row {
        bit: stewie::CONTAINER_MOUSEOVER_NO_FORM,
        function: "MenuFixes",
        plan: "S0022.17",
        key: "bMenuFixesContainerMouseoverNoForm",
        tweaks: None,
        summary: "pointing at an entry with no item in the container menu (keys) no longer crashes",
    },
    Row {
        bit: stewie::CONSOLE_WHILE_MENUS_LOAD,
        function: "MenuFixes",
        plan: "S0022.30",
        key: "bMenuFixesConsoleWhileMenusLoad",
        tweaks: None,
        summary: "opening the console while the menus load no longer crashes",
    },
    Row {
        bit: stewie::LIST_BOX_GET_AT_NULL_ITEM,
        function: "MenuFixes",
        plan: "S0022.36",
        key: "bMenuFixesListBoxNullItem",
        tweaks: None,
        summary: "a list in a menu with an empty entry no longer crashes when an entry is looked up",
    },
    Row {
        bit: stewie::TILE_LABEL_NULL_NAME,
        function: "MenuFixes",
        plan: "S0022.56",
        key: "bMenuFixesTileLabelNullName",
        tweaks: None,
        summary: "a menu tile with no name no longer crashes the labels' refresh",
    },
    Row {
        bit: stewie::PIPBOY_EQUIP_CRASH,
        function: "patchFixPipboyEquipCrash",
        plan: "S0027",
        key: "bFixPipboyEquipCrash",
        tweaks: None,
        summary: "equipping a Pip-Boy item no longer crashes: the Pip-Boy is reset a moment later (the next frame, or when it opens), not in the middle of the equip",
    },
    Row {
        bit: stewie::EXPLOSION_WATER_NULL_CALL,
        function: "fixExplosionWaterNullCall",
        plan: "S0042",
        key: "bFixExplosionWaterNullCall",
        tweaks: None,
        summary: "water reflecting an explosion no longer crashes on an explosion that is being taken apart",
    },
    Row {
        bit: stewie::RELOAD_LOOP_NULL_WEAPON,
        function: "patchFixNullWeaponOrWeaponInfoReloadLoopCrash",
        plan: "S0043",
        key: "bFixNullWeaponOrWeaponInfoReloadLoopCrash",
        tweaks: None,
        summary: "an actor in a reload loop, or with a reload queued, that has lost its weapon no longer crashes",
    },
    Row {
        bit: stewie::ITEM_VALUE_NULL_WEAPON_MOD,
        function: "patchFixGetItemValueCrashForModdedWeapons",
        plan: "S0045",
        key: "bFixGetItemValueCrashForModdedWeapons",
        tweaks: None,
        summary: "a weapon whose mod flags name a mod the weapon no longer has no longer crashes when its value is asked",
    },
    Row {
        bit: stewie::UNRESOLVED_RACE,
        function: "patchFixUnresolvedRaceCrash",
        plan: "S0049",
        key: "bFixUnresolvedRaceCrash",
        tweaks: None,
        summary: "an NPC whose race is missing (a removed plugin's) gets CaucasianVanilla instead of crashing",
    },
    Row {
        bit: stewie::LEVEL_GREATER_THAN_99,
        function: "patchFixLevelGreaterThan99Crash",
        plan: "S0057",
        key: "bFixLevelGreaterThan99Crash",
        tweaks: None,
        summary: "levelling up past level 99 no longer crashes the HUD's level display",
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
        bit: stewie::THROWING_WEAPON_AMMO_EFFECTS,
        function: "fixGetWeaponDamageCrashOnThrowingWeapons",
        plan: "S0067",
        key: "bFixGetWeaponDamageCrashOnThrowingWeapons",
        tweaks: None,
        summary: "a throwing weapon's damage shown in a menu no longer crashes",
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
        bit: stewie::BODY_PART_NULL_PARENT,
        function: "patchBodyPartDataNullChecks",
        plan: "S0077",
        key: "bBodyPartDataNullChecks",
        tweaks: None,
        summary: "hiding or showing limbs whose node chain runs out before the root no longer crashes",
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
        bit: stewie::RELOAD_NULL_AMMO_ENTRY,
        function: "RememberAmmoCountOnLoad",
        plan: "S0089.04",
        key: "bRememberAmmoCountOnLoadReloadNoAmmoEntry",
        tweaks: None,
        summary: "the player reloading with no ammunition entry equipped no longer crashes",
    },
    Row {
        bit: stewie::DROP_AMMO_NULL_ENTRY,
        function: "RememberAmmoCountOnLoad",
        plan: "S0089.05",
        key: "bRememberAmmoCountOnLoadDropNoAmmoEntry",
        tweaks: None,
        summary: "dropping the weapon's ammunition with no ammunition entry equipped no longer crashes",
    },
    Row {
        bit: stewie::PERK_DESCRIPTION_TOO_LONG,
        function: "PerkMenuDescriptionTypoFix",
        plan: "S0093.07",
        key: "bPerkMenuDescriptionTypoFixTooLong",
        tweaks: None,
        summary: "a perk description too long for the menu's buffer is cut to fit instead of crashing",
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
        bit: stewie::COMBAT_STATE_NULL_PROJECTILE,
        function: "patchFixNullProjectileInCombatStateUpdateInventory",
        plan: "S0126",
        key: "bFixNullProjectileInCombatStateUpdateInventory",
        tweaks: None,
        summary: "an NPC weighing a weapon with no projectile in combat no longer crashes; it weighs the weapon's own projectile's explosion, as when it picks the weapon (changes which explosion an NPC with special ammunition considers)",
    },
    Row {
        bit: stewie::RECIPE_MENU_NULL_AMMO_INFO,
        function: "patchFixRecipeMenuMissingAmmoInfoNullCheck",
        plan: "S0127",
        key: "bFixRecipeMenuMissingAmmoInfoNullCheck",
        tweaks: None,
        summary: "crafting ammunition while no ammunition is equipped no longer crashes",
    },
    Row {
        bit: stewie::CREDITS_NO_LOADED_SAVE,
        function: "patchFixCreditsCrashIfNoLoadedSave",
        plan: "S0139",
        key: "bFixCreditsCrashIfNoLoadedSave",
        tweaks: None,
        summary: "closing the credits from the main menu before any save was loaded no longer crashes",
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
        bit: stewie::MISSING_ANIM_NULL_CHECK,
        function: "patchFixMissingAnimNullCheck",
        plan: "S0150",
        key: "bFixMissingAnimNullCheck",
        tweaks: None,
        summary: "an actor, or the player's first-person view, with no attack animation playing no longer crashes",
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
        bit: stewie::ANIM_DATA_RELOAD_LOOP_NULL,
        function: "patchFixAnimDataUpdateReloadLoopNullCheck",
        plan: "S0172",
        key: "bFixAnimDataUpdateReloadLoopNullCheck",
        tweaks: None,
        summary: "a queued reload for an actor with no weapon no longer crashes",
    },
    Row {
        bit: stewie::ANIM_DATA_MOVEMENT_SEQUENCE,
        function: "patchAnimDataUpdateCrashCheck",
        plan: "S0173",
        key: "bAnimDataUpdateCrashCheck",
        tweaks: None,
        summary: "a movement animation that animates no bones no longer crashes; the console names it, once a game",
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
        bit: stewie::NULL_ANIM_REMOVE_RECURSE,
        function: "NullAnimRemoveRecurseFix",
        plan: "S0181",
        key: "bNullAnimRemoveRecurseFix",
        tweaks: None,
        summary: "an idle animation's held object that is already gone no longer crashes when the idle ends",
    },
    Row {
        bit: stewie::HOLOTAPE_EMPTY_SUBTITLE,
        function: "HolotapeEmptySubtitleCrashFix",
        plan: "S0182",
        key: "bHolotapeEmptySubtitleCrashFix",
        tweaks: None,
        summary: "playing a holotape that has sound but no subtitles no longer crashes",
    },
    Row {
        bit: stewie::BIPED_ANIM_NULL_ITEM,
        function: "BipedAnimNullItemError",
        plan: "S0186",
        key: "bBipedAnimNullItemError",
        tweaks: None,
        summary: "a helmet whose item is gone is not attached instead of crashing; the console names the slot and the actor",
    },
    Row {
        bit: stewie::NPC_ACTIVATE_NULL_PROCESS,
        function: "MiscCrashFixes",
        plan: "S0199.05",
        key: "bMiscCrashFixesNpcActivate",
        tweaks: None,
        summary: "an NPC activated by something that is not an actor, or by an actor with no AI process, no longer crashes",
    },
    Row {
        bit: stewie::REMOVE_WORN_NULL_ACTOR,
        function: "MiscCrashFixes",
        plan: "S0199.06",
        key: "bMiscCrashFixesRemoveWorn",
        tweaks: None,
        summary: "taking off a worn item that may not be removed, with no actor given, no longer crashes",
    },
    Row {
        bit: stewie::SHOW_SCENE_GRAPH_NULL,
        function: "MiscCrashFixes",
        plan: "S0199.09",
        key: "bMiscCrashFixesShowSceneGraph",
        tweaks: None,
        summary: "the console's ShowSceneGraph no longer crashes on an empty property entry",
    },
    Row {
        bit: stewie::EQUIP_AMMO_NULL_PROCESS,
        function: "patchEquipAmmoOnNullBaseProcessCrashFix",
        plan: "S0202",
        key: "bEquipAmmoOnNullBaseProcessCrashFix",
        tweaks: None,
        summary: "equipping ammunition on an actor with no AI process no longer crashes",
    },
    Row {
        bit: stewie::SAVING_NULL_CELL,
        function: "patchFixSavingNullCellCrash",
        plan: "S0881",
        key: "bFixSavingNullCellCrash",
        tweaks: None,
        summary: "saving with a placed object that has no cell no longer crashes",
    },
    Row {
        bit: stewie::CHANGES_MAP_BUCKETS,
        function: "Inlines",
        plan: "S1016.845558",
        key: "bInlinesChangesMapBuckets",
        tweaks: Some(("Inlines", "bSaveLoad")),
        summary: "a save's changed-forms map gets 10,037 buckets (Engine Optimizations): shorter saves late in a playthrough; changed forms are written in another order (saves load either way)",
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
