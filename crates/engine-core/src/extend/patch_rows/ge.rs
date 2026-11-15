//! Group GE's seams and wraps (step 3.9: actors: movement, life, NPC perks). Only group GE's PR edits this file; rows by
//! address, owner `super::JIP`, step "3.9-GE". Rows of the patches this group
//! writes the handlers of (its home rows), seams in other groups' files included.

use super::super::Entry;
use super::JIP;

/// The group's seams.
pub(in super::super) const SITES: &[Entry] = &[
    // G63: the actor's load-game routine 008ab3a0, its two calls that clear bits 0x80000000
    // (008c1ac0) and 0x40000000 (008c1b50) of the flag word +0x140 after it is read: JIP's 26
    // bytes clear its own flag bytes +0x105/+0x106 and only bit 0x40000000 instead.
    Entry {
        addr: 0x008a_b6d9,
        owners: JIP,
        step: "3.9-GE",
    },
];

/// The group's wraps.
pub(in super::super) const WRAPS: &[Entry] = &[];
