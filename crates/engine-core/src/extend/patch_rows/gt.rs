//! Group GT's wraps (step 3.9: the fixed strings, JIP LN NVSE's G43). Only group GT's PR edits this file;
//! rows by address, owner `super::JIP`, step "3.9-GT".

use super::super::Entry;
use super::JIP;

const fn row(addr: u32) -> Entry {
    Entry {
        addr,
        owners: JIP,
        step: "3.9-GT",
    }
}

/// The group's seams: none.
pub(in super::super) const SITES: &[Entry] = &[];

/// The group's wraps.
pub(in super::super) const WRAPS: &[Entry] = &[
    // G43: the clean-up of unused strings 00a5b460 (its one call, 0x878203), JIP's
    // FreeUnusedNiFixedStrings in its place.
    row(0x00a5_b460),
    // G43: the table's making 00a5b630, made `ret` by JIP.
    row(0x00a5_b630),
    // G43: "make this string" 00a5b690, JIP's GetNiFixedString (its pool) in its place.
    row(0x00a5_b690),
];
