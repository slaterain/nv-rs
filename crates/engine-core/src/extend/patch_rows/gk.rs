//! Group GK's seams and wraps (step 3.9: lights in combat). Only group GK's PR edits this file; rows by address,
//! owner `super::JIP`, step "3.9-GK". Rows of the patches this group writes the handlers of (its home rows), seams
//! in other groups' files included.

use super::super::Entry;
use super::JIP;

const fn row(addr: u32) -> Entry {
    Entry {
        addr,
        owners: JIP,
        step: "3.9-GK",
    }
}

/// The group's seams.
pub(in super::super) const SITES: &[Entry] = &[
    // G97 (fix row, `WriteRelCall(0x9AE68C, SetExplosionLightFadeHook)`): the explosion's update 009ae5d0, its
    // call of the light fade 009b1320: the fade times the explosion form's light's fade (seam in `explosion.rs`).
    row(0x009a_e68c),
    // G96 (fix row, `WriteRelJump(0x9BB906, SetMuzzleFlashFadeHook)`): `MuzzleFlash::UpdateLight` 009bb8a0's
    // dimmer: the muzzle flash light form's fade while on (seam in `muzzleflash.rs`).
    row(0x009b_b906),
    // G95 (fix row, `WriteRelCall(0x9BC6D9, RemoveProjectileLightHook)`): `Projectile`'s destructor 009bc470, its
    // call of 009c3e10: JIP's light taken off its node (seam in `projectile.rs`).
    row(0x009b_c6d9),
    // G95 (fix row, `WriteRelCall(0x9BDC92, AddProjectileLightHook)`): the projectile's start in the world
    // 009bda10, its call of 009c3d90: one of JIP's point lights from the form's light (seam in `projectile.rs`).
    row(0x009b_dc92),
];

/// The group's wraps.
pub(in super::super) const WRAPS: &[Entry] = &[];
