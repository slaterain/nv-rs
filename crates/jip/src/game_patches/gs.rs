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

use super::Area;
use engine_core::extend::{Owner, SiteSlot};
use engine_core::prelude::*;

/// O7's seams: `ScriptRunner::Run`'s failed line, `EvaluateLine`'s set and condition errors.
pub const FAILED_SCRIPT_LOCKS: [SiteSlot; 3] = [
    SiteSlot::new(Owner::Jip, 0x005e_0fbe),
    SiteSlot::new(Owner::Jip, 0x005e_1a23),
    SiteSlot::new(Owner::Jip, 0x005e_1f0b),
];

/// The group's rows and installers.
pub const AREA: Area = Area {
    name: "GS",
    fix_sites: &[
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
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_game.h: SetOptionalPatch case 7
// (the three SafeWrite16 jumps).
/// A failed line's clear of the data length: taken (the jump), so the clear is skipped.
fn keep_failed_script(_: &mut Engine, _: &[u32]) -> Option<u32> {
    Some(1)
}

#[cfg(test)]
#[path = "gs_tests.rs"]
mod tests;
