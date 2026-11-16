//! Group GA's seams and wraps (step 3.9: UI: HUD, messages, text, start-up INI). Only group GA's PR edits this file; rows by
//! address, owner `super::JIP`, step "3.9-GA". Rows of the patches this group
//! writes the handlers of (its home rows), seams in other groups' files included.

use super::super::Entry;
use super::JIP;

const fn row(addr: u32) -> Entry {
    Entry {
        addr,
        owners: JIP,
        step: "3.9-GA",
    }
}

/// The group's seams.
pub(in super::super) const SITES: &[Entry] = &[
    // G10 (fix row): the HUD corner message record's layout (text 0x104 -> 0x204, record 0x310 ->
    // 0x410), one seam per function for JIP's 16 bytes: DoIdle 00770430 (0x7714DA) ...
    row(0x0077_14da),
    // ... fn_00775380, the message's queueing (0x7753B8 and its ten neighbours to 0x775600) ...
    row(0x0077_53b8),
    // ... and HideMessage 007756e0 (0x7757B1, 0x7757D3, 0x77582B).
    row(0x0077_57b1),
];

/// The group's wraps.
pub(in super::super) const WRAPS: &[Entry] = &[];
