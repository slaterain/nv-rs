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
    // O7 `bNoFailedScriptLocks` (`SetOptionalPatch(7)`: three `SafeWrite16` jumps past the clear of a
    // failed script's data length): `ScriptRunner::Run` 005e0d20, then `ScriptRunner::EvaluateLine`
    // 005e1550's set and condition errors.
    row(0x005e_0fbe),
    row(0x005e_1a23),
    row(0x005e_1f0b),
];

/// The group's wraps.
pub(in super::super) const WRAPS: &[Entry] = &[];
