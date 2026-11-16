//! Group GS's seams and wraps (step 3.9: script engine). Only group GS's PR edits this file; rows by
//! address, owner `super::JIP`, step "3.9-GS". Rows of the patches this group
//! writes the handlers of (its home rows), seams in other groups' files included.

use super::super::Entry;
use super::JIP;

const fn row(addr: u32) -> Entry {
    Entry {
        addr,
        owners: JIP,
        step: "3.9-GS",
    }
}

/// The group's seams.
pub(in super::super) const SITES: &[Entry] = &[
    // O8 `bDoublePrecision` (`SetOptionalPatch(8)`): `CExpression::Eval` 00593db0's operator
    // switch, JIP's DoOperatorHook in its place.
    row(0x0059_3fbc),
    // G70: `GetLockedConditionFunction` 0059d010's call of `BGSTerminal::IsUnlocked`, JIP's
    // TerminalGetLockedHook in its place.
    row(0x0059_d0b3),
    // G71: `IsCurrentFurnitureObjConditionFunction` 0059dfe0's process read, JIP's 24 bytes test
    // the process for null.
    row(0x0059_e034),
    // G51: `AddItem` 005b4be0's tests before the container items' health call, JIP's two-byte jump.
    row(0x005b_4dbc),
    // G34: `PlaySound3D` 005c22f0's object to follow, JIP's 8 bytes read the reference's node.
    row(0x005c_240d),
    // G03: `AddAchievement` 005d3b20's console-used test, JIP's two-byte NOP.
    row(0x005d_3b69),
    // O7 `bNoFailedScriptLocks` (`SetOptionalPatch(7)`: three `SafeWrite16` jumps past the clear of a
    // failed script's data length): `ScriptRunner::Run` 005e0d20, then `ScriptRunner::EvaluateLine`
    // 005e1550's set and condition errors.
    row(0x005e_0fbe),
    row(0x005e_1a23),
    row(0x005e_1f0b),
];

/// The group's wraps.
pub(in super::super) const WRAPS: &[Entry] = &[
    // G123: `TESImageSpaceModifier::GetGetHit` 005d2860 replaced by JIP's GetHitIMODHook.
    row(0x005d_2860),
];
