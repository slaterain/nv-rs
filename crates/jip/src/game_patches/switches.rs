//! JIP's `[GamePatches]` switches at run time: `SetOptionalPatch` (`internal/patches_game.h`
//! (JIP LN NVSE)) and the "is it in" answers `GetOptionalPatch` gives (`HOOK_INSTALLED`,
//! `s_patchInstallState`, `functions_jip/jip_fn_utility.h`).
//!
//! In the port every switch's seams and wraps are put in with the fixes (`put_fixes_in`) and
//! each one tests its bit of the run-wide word ([`Engine::jip_patches`]) when it is reached, so
//! JIP's hook set and its `s_patchInstallState` flag are both that bit: setting or clearing it
//! is JIP's `HOOK_SET` and its flag write. What JIP does besides, per switch, is done here:
//! - 1 (`bIgnoreDTDRFix`) off: 12 (`bLocalizedDTDR`) off too and the last hit's damage threshold
//!   (`s_VATSHitDT`, G121) back to `-FLT_MAX`;
//! - 3 (`bEnableBigGunsSkill`): Big Guns' actor value flags (0x410 on, 0x2000 off) and its
//!   description (JIP's text on, the alternative text dropped off: [`super::gb::set_big_guns_skill`]);
//! - 12 only while 1 is in; 17 only with JIP's three weapon mod numbers each non-zero;
//! - 16 on: the load screen lists made when not made yet ([`super::ga::deferred_init`]);
//! - 18 on: JIP's NPC perk set-up ([`super::ge::perks::switch_on`]);
//! - 4, 7, 8, 14, 15, 17, 18, 19 can only be turned on (JIP writes code it never takes back);
//! - 15 (`bEnableNVACAlerts`) patches NVAC's DLL, which the port does not load: never in, as JIP
//!   with no `nvac` module (`GetModuleHandle` answers null).
//!
//! The game patches row (`NV_JIP_GAME_PATCHES`, the word's `FIXES` bit) holds every switch's
//! seams: with it off (the vanilla preset) no switch is in, `GetOptionalPatch` answers 0 and
//! `SetOptionalPatch` changes nothing (a preset's choice: JIP itself has no such row).
//!
//! Threads: the word is one atomic, changed with one `fetch_or` / `fetch_and` (as JIP's hook
//! writes, from the script's thread); the seams read it as they are reached.

use super::settings;
use crate::Jip;
use engine_core::extend::jip_patches;
use engine_core::prelude::*;

/// `s_optionalHacks->Get(name)`: the switch's number (20 to 22 the weapon mod numbers), 0 for a
/// name it does not hold. JIP's map compares the names' `StrHashCI` (as [`settings::switches`]).
pub fn number_of(name: &[u8]) -> u32 {
    let hash = crate::containers::str_hash_ci(name);
    settings::KEYS
        .iter()
        .find(|(key, _)| crate::containers::str_hash_ci(key.as_bytes()) == hash)
        .map_or(0, |&(_, number)| number)
}

/// Switch `n`'s bit is set and the row's patches are in.
fn bit_in(e: &Engine, n: u32) -> bool {
    let word = e.jip_patches();
    word & jip_patches::FIXES != 0 && word & jip_patches::optional(n) != 0
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_utility.h:
// Cmd_GetOptionalPatch_Execute (its switch).
/// Whether switch `number` is in (`GetOptionalPatch`'s answer): see the module's doc.
pub fn is_in(e: &mut Engine, number: u32) -> bool {
    match number {
        1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 | 9 | 11 | 13 | 14 | 16 | 19 => bit_in(e, number),
        12 => bit_in(e, 12) && bit_in(e, 1),
        17 => bit_in(e, 17) && super::gh2::weapon_mods_on(e),
        // `NPCPerks + s_NPCPerksAutoAdd`: the patch is marked in at its set-up; the flag only
        // comes with it in the port (the player's file).
        18 => e.jip_patches() & jip_patches::FIXES != 0 && crate::cosave::npc_perks_patch(e),
        _ => false,
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_game.h: SetOptionalPatch.
/// Switch `number` turned on or off as JIP's `SetOptionalPatch` does (see the module's doc);
/// whether anything changed (JIP's answer, which the command does not use).
pub fn set(e: &mut Engine, number: u32, enable: bool) -> bool {
    // 0, 10, 20-22 and others: JIP's `default` (and no bit of the word).
    if e.jip_patches() & jip_patches::FIXES == 0 || !matches!(number, 1..=9 | 11..=19) {
        return false;
    }
    let bit = jip_patches::optional(number);
    let on = |e: &Engine| e.jip_patches() & bit != 0;
    match number {
        1 => {
            if on(e) == enable {
                return false;
            }
            if enable {
                e.jip_patches_or(bit);
            } else {
                e.jip_patches_and_not(bit | jip_patches::LOCALIZED_DTDR);
                super::gc::damage::reset_vats_hit_dt(e);
            }
            true
        }
        2 | 5 | 6 | 9 | 11 | 13 => toggle(e, bit, enable),
        3 => {
            if !toggle(e, bit, enable) {
                return false;
            }
            super::gb::set_big_guns_skill(e, enable);
            true
        }
        12 => {
            if on(e) == enable || !bit_in(e, 1) {
                return false;
            }
            toggle(e, bit, enable)
        }
        16 => {
            if !toggle(e, bit, enable) {
                return false;
            }
            if enable {
                super::ga::deferred_init(e);
            }
            true
        }
        4 | 7 | 8 | 14 | 19 => enable && (e.jip_patches_or(bit) & bit) == 0,
        17 => {
            if !enable || on(e) || !super::gh2::weapon_mod_numbers_set(e) {
                return false;
            }
            e.jip_patches_or(bit);
            true
        }
        18 => {
            if !enable || crate::cosave::npc_perks_patch(e) {
                return false;
            }
            e.jip_patches_or(bit);
            let auto_add = e.ext::<Jip>().game_patches.settings.npc_perks_auto_add;
            super::ge::perks::switch_on(e, auto_add);
            true
        }
        // 15: NVAC's DLL is not loaded (JIP's `GetModuleHandle("nvac")` answers null).
        _ => false,
    }
}

/// JIP's `HOOK_SET(hook, enable)`: the bit set or cleared; whether it changed.
fn toggle(e: &mut Engine, bit: u32, enable: bool) -> bool {
    let before = if enable {
        e.jip_patches_or(bit)
    } else {
        e.jip_patches_and_not(bit)
    };
    (before & bit != 0) != enable
}

#[cfg(test)]
#[path = "switches_tests.rs"]
mod tests;
