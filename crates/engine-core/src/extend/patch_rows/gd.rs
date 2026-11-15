//! Group GD's seams and wraps (step 3.9: projectiles, explosions, combat AI, packages). Only group GD's PR edits this file; rows by
//! address, owner `super::JIP`, step "3.9-GD". Rows of the patches this group
//! writes the handlers of (its home rows), seams in other groups' files included.

use super::super::Entry;
use super::JIP;

/// The group's seams.
pub(in super::super) const SITES: &[Entry] = &[
    // G117, the projectile's launch constructor 009bbef0: its projectile count, damage division
    // and mod scale (009bc241..009bc293), replaced by JIP's InitProjectileSetAmmoHook, which also
    // records the ammo, the split-beam flag and the count in the projectile.
    Entry {
        addr: 0x009b_c241,
        owners: JIP,
        step: "3.9-GD",
    },
];

/// The group's wraps.
pub(in super::super) const WRAPS: &[Entry] = &[];
