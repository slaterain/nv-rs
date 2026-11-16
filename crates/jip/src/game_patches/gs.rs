//! Group GS's game patches (step 3.9: script engine): its handlers, wraps and slot routines. Only group
//! GS's PR edits this file (and its own tests file beside it).
//!
//! **O7, `bNoFailedScriptLocks`** (`SetOptionalPatch(7)`, `internal/patches_game.h` (JIP LN NVSE); on
//! by JIP's default): `SafeWrite16(0x5E0FBE, 0x7AEB)`, `SafeWrite16(0x5E1A23, 0x0DEB)`,
//! `SafeWrite16(0x5E1F0B, 0x0DEB)`: short jumps over `MOV ECX,[EBP+8]; CALL 00500940; MOV
//! [EAX+8],0` (the jump at `0x5E0FBE` lands on the `JMP`'s target `0x5E103A`, the others right after
//! the clear), so a script whose line fails keeps its data length and runs again next time,
//! where the exe sets it to 0 and the script never runs again. JIP turns it on once and never off
//! (`!bEnable` answers false); the port's seams test the switch's bit, so a later
//! `SetOptionalPatch(7, 0)` would turn it off: not reachable while that command is not built.
//!
//! **G51** (`InitGamePatches`): `AddItem` gives every added form the container items' health call
//! ([`every_item_health`]). **D07** (`DeferredInit`): one word of "JIP Companions Command & Control.esp"'s
//! main script ([`deferred_init`]). Both are fixes (the row on).

use super::Area;
use engine_core::extend::{Owner, SiteSlot};
use engine_core::prelude::*;

/// O7's seams: `ScriptRunner::Run`'s failed line, `EvaluateLine`'s set and condition errors.
pub const FAILED_SCRIPT_LOCKS: [SiteSlot; 3] = [
    SiteSlot::new(Owner::Jip, 0x005e_0fbe),
    SiteSlot::new(Owner::Jip, 0x005e_1a23),
    SiteSlot::new(Owner::Jip, 0x005e_1f0b),
];
/// G51: `AddItem`'s tests before the container items' health call (the jump at `0x5B4DBC`).
pub const EVERY_ITEM_HEALTH: SiteSlot = SiteSlot::new(Owner::Jip, 0x005b_4dbc);

/// The group's rows and installers.
pub const AREA: Area = Area {
    name: "GS",
    fix_sites: &[
        EVERY_ITEM_HEALTH.addr(),
        FAILED_SCRIPT_LOCKS[0].addr(),
        FAILED_SCRIPT_LOCKS[1].addr(),
        FAILED_SCRIPT_LOCKS[2].addr(),
    ],
    fix_wraps: &[],
    install_fixes,
    jip_sites: &[],
    jip_wraps: &[],
    install_jip: super::nothing,
};

fn install_fixes(e: &mut Engine) {
    for slot in FAILED_SCRIPT_LOCKS {
        e.on_site(slot, keep_failed_script);
    }
    e.on_site(EVERY_ITEM_HEALTH, every_item_health);
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_game.h: SetOptionalPatch case 7
// (the three SafeWrite16 jumps).
/// A failed line's clear of the data length: taken (the jump), so the clear is skipped.
fn keep_failed_script(_: &mut Engine, _: &[u32]) -> Option<u32> {
    Some(1)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_game.h: InitGamePatches
// (SafeWrite16(0x5B4DBC, 0x19EB)).
/// G51 (`0x5B4DBC`, no words): `AddItem`'s jump over its two tests (a weapon form, a weapon type
/// below 10) to the container items' health call: the call is made for every added form, a
/// leveled list's and a form list's too, and when nothing could be added. With G50 (`00482090`
/// rewritten, GH1) armour and real weapons get the 0.999 health, other items none. Main thread
/// and AI threads (scripts); no state.
fn every_item_health(_: &mut Engine, _: &[u32]) -> Option<u32> {
    Some(1)
}

/// D07: "JIP Companions Command & Control.esp"'s main script (`0xADE` in that file).
const CCC_PLUGIN: &[u8] = b"JIP Companions Command & Control.esp";
const CCC_SCRIPT: u32 = 0xade;
/// `ModInfo::modIndex` (+0x40C), `Script::data` (+0x30), the patched word's place in the data, the
/// word before and after (MSVC's multi-character constants `' 931'` and `' 552'`: the bytes "139 "
/// and "255 ").
const MOD_INDEX: u32 = 0x40c;
const SCRIPT_DATA: u32 = 0x30;
const CCC_WORD_AT: u32 = 0x29c;
const CCC_WORD_BEFORE: u32 = 0x2039_3331;
const CCC_WORD_AFTER: u32 = 0x2035_3532;

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_game.h: DeferredInit (the
// "JIP Companions Command & Control.esp" lines).
/// D07, once at `DeferredInit` (the main thread, before the AI threads run scripts), with the
/// fixes: when that plugin is loaded (`DataHandler::GetModIndex` not 0xFF) and its form `0xADE`
/// holds `' 931'` at its data +0x29C, the word becomes `' 552'`. JIP reads the data with no test;
/// a form with no data (JIP would stop the game) is left alone.
pub fn deferred_init(e: &mut Engine) {
    if e.jip_patches() & engine_core::extend::jip_patches::FIXES == 0 {
        return;
    }
    let info = crate::commands::jip_fn_utility::lookup_mod_by_name(e, CCC_PLUGIN);
    if info == 0 {
        return;
    }
    let index = u32::from(e.mem.u8(info + MOD_INDEX));
    let script = crate::cosave::lookup_form(e, index << 24 | CCC_SCRIPT);
    if script == 0 {
        return;
    }
    let data = e.mem.u32(script + SCRIPT_DATA);
    if data == 0 {
        return;
    }
    if e.mem.u32(data + CCC_WORD_AT) == CCC_WORD_BEFORE {
        e.mem.set_u32(data + CCC_WORD_AT, CCC_WORD_AFTER);
    }
}

#[cfg(test)]
#[path = "gs_tests.rs"]
mod tests;
