//! Group GP's seams and wraps (step 3.9: player character). Only group GP's PR edits this file; rows by
//! address, owner `super::JIP`, step "3.9-GP". Rows of the patches this group
//! writes the handlers of (its home rows), seams in other groups' files included.

use super::super::Entry;
use super::JIP;

/// The group's seams.
pub(in super::super) const SITES: &[Entry] = &[
    // The player's float actor value change 0093b7a0, the value read before the change: JIP's
    // DamagePCFatigueMinFixHook (G105) stops fatigue at fMinFatigue.
    Entry {
        addr: 0x0093_b87f,
        owners: JIP,
        step: "3.9-GP",
    },
    // The music marker list's rebuild 00947d80, the world space's loop: JIP's
    // BuildMusicMarkerListHook (G72) puts in its sorted cache of the world space's markers.
    Entry {
        addr: 0x0094_7dd2,
        owners: JIP,
        step: "3.9-GP",
    },
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
    // The closest music marker's search 00969930, the marker's position: JIP's 20 bytes (G26)
    // pass over a disabled marker and read the position at +0x30.
    Entry {
        addr: 0x0096_99c5,
        owners: JIP,
        step: "3.9-GP",
    },
    // PlayerCharacter::UpdateHardcoreMode 00969c30, its IsInState(4) call: JIP's
    // ModHardcoreNeedsHook (G114, with O6 inside) adds the pending needs and keeps tracking off.
    Entry {
        addr: 0x0096_9c41,
        owners: JIP,
        step: "3.9-GP",
    },
];

/// The group's wraps.
pub(in super::super) const WRAPS: &[Entry] = &[];
