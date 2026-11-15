//! Group GP's seams and wraps (step 3.9: player character). Only group GP's PR edits this file; rows by
//! address, owner `super::JIP`, step "3.9-GP". Rows of the patches this group
//! writes the handlers of (its home rows), seams in other groups' files included.

use super::super::Entry;
use super::JIP;

/// The group's seams.
pub(in super::super) const SITES: &[Entry] = &[
    // PlayerCharacter::Load3D 0094e1d0, the first-person model's load (LoadFile and the flag
    // call): JIP's GeneratePlayerNodeHook (G38) copies the model loader's cached model instead.
    Entry {
        addr: 0x0094_e4a1,
        owners: JIP,
        step: "3.9-GP",
    },
    // PlayerCharacter::Load3D 0094e1d0, the eye height read from the first-person camera node:
    // JIP's two-byte jump (G38) skips it.
    Entry {
        addr: 0x0094_e5f6,
        owners: JIP,
        step: "3.9-GP",
    },
];

/// The group's wraps.
pub(in super::super) const WRAPS: &[Entry] = &[];
