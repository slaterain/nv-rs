//! Scripts: JIP LN NVSE's "Cmd_Execute >> *result = 0 by default" (`InitGamePatches`,
//! `internal/patches_game.h` (JIP LN NVSE); step 3.9a, PLAN-3.9 G02), a fix.
//!
//! Before each of the exe's three calls of a script handler (the operand reader `005ac7a0` at
//! `0x5ACB9D`; `ScriptRunner::EvaluateLine`'s block-type handler at `0x5E16C3` and its command
//! handler at `0x5E2329`) JIP writes 0.0 into the result the handler is given (`fldz; fstp qword
//! [ecx]` at the first, `xorps xmm0, xmm0; movq [ecx|edx], xmm0` at the others), for every opcode.
//! The exe leaves whatever the result held before, so a vanilla command that sets no result (a
//! failed lookup, a command that answers nothing) answers the previous command's value; with JIP
//! it answers 0. xNVSE zeroes it only for extender opcodes at the first and third (its own handler
//! at the same seams, `nvse::args::zero_extender_result`, runs before this one): this is JIP as the
//! second owner of xNVSE's three seams, writing the same 0.0 for every opcode.

use super::Area;
use engine_core::extend::{Owner, SiteSlot};
use engine_core::prelude::*;

/// The seams before the handler calls (words: the opcode, the record, the result's address).
pub const OPERAND_HANDLER_CALL: SiteSlot = SiteSlot::new(Owner::Jip, 0x005a_cb9d);
pub const BLOCK_HANDLER_CALL: SiteSlot = SiteSlot::new(Owner::Jip, 0x005e_16c3);
pub const COMMAND_HANDLER_CALL: SiteSlot = SiteSlot::new(Owner::Jip, 0x005e_2329);

/// The area's rows.
pub const AREA: Area = Area {
    name: "script",
    fix_sites: &[
        OPERAND_HANDLER_CALL.addr(),
        BLOCK_HANDLER_CALL.addr(),
        COMMAND_HANDLER_CALL.addr(),
    ],
    fix_wraps: &[],
    install_fixes,
    jip_sites: &[],
    jip_wraps: &[],
    install_jip: super::nothing,
};

fn install_fixes(e: &mut Engine) {
    e.on_site(OPERAND_HANDLER_CALL, zero_result);
    e.on_site(BLOCK_HANDLER_CALL, zero_result);
    e.on_site(COMMAND_HANDLER_CALL, zero_result);
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_game.h: InitGamePatches
// (SAFE_WRITE_BUF(0x5ACB9D, ...), (0x5E16C3, ...), (0x5E2329, ...)).
/// The result starts at 0.0 whatever the opcode; the handler is then called as before. The
/// result's address is never 0 in the exe (a local of the caller); the port passes over a 0 where
/// JIP would stop the game.
fn zero_result(e: &mut Engine, a: &[u32]) -> Option<u32> {
    if a[2] != 0 {
        e.mem.set_f64(a[2], 0.0);
    }
    None
}
