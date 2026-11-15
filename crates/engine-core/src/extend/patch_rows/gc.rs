//! Group GC's seams and wraps (step 3.9: hit damage, detection, combat formulas). Only group GC's PR edits this file; rows by
//! address, owner `super::JIP`, step "3.9-GC". Rows of the patches this group
//! writes the handlers of (its home rows), seams in other groups' files included.

use super::super::Entry;
use super::JIP;

/// The group's seams.
pub(in super::super) const SITES: &[Entry] = &[
    // G53, Actor::GetDetectionLevelAgainstActor 008a0d10: its three calls of the detected
    // actor's teammate test (0056_6950), replaced by JIP's DetectionTeammateHook.
    Entry {
        addr: 0x008a_0dab,
        owners: JIP,
        step: "3.9-GC",
    },
    Entry {
        addr: 0x008a_1291,
        owners: JIP,
        step: "3.9-GC",
    },
    Entry {
        addr: 0x008a_131d,
        owners: JIP,
        step: "3.9-GC",
    },
    // G53, the same function: the detected actor's level word (its actor value owner's virtual
    // +0x28), replaced by JIP's DetectionGetLevelHook.
    Entry {
        addr: 0x008a_147c,
        owners: JIP,
        step: "3.9-GC",
    },
];

/// The group's wraps.
pub(in super::super) const WRAPS: &[Entry] = &[];
