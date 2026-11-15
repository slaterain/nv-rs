//! Group GH2's seams and wraps (step 3.9: references, models, cells, ash piles). Only group GH2's PR edits this file; rows by
//! address, owner `super::JIP`, step "3.9-GH2". Rows of the patches this group
//! writes the handlers of (its home rows), seams in other groups' files included.

use super::super::Entry;
use super::JIP;

/// The group's seams.
pub(in super::super) const SITES: &[Entry] = &[
    // H24: the reference's destructor body 0055a430, its call of TESObjectREFR::IsFurniture:
    // JIP's DestroyRefrHook (InitJIPHooks) drops the reference's own name and model path and a
    // temporary reference's aux variables, then answers the furniture test.
    Entry {
        addr: 0x0055_a488,
        owners: JIP,
        step: "3.9-GH2",
    },
];

/// The group's wraps.
pub(in super::super) const WRAPS: &[Entry] = &[];
