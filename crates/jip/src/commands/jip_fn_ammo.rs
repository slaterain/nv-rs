//! `functions_jip/jip_fn_ammo.h` (JIP LN NVSE): an ammo's numbers, projectile and casing. All 5
//! of its commands.
//!
//! Casts of the script's float, from JIP's source (MSVC, SSE2): `projPerShot = (int)val`
//! (`cvttss2si`, stored as the 32-bit word), `clipRounds = val` (a `UInt8` from a float:
//! `cvttss2si`, its low byte), (the effects are `jip_fn_ammo_effect`).

use super::forms::{form_type_more, TESAmmo, INVENTORY_TYPE, TES_AMMO_VTBL};
use super::{cvttss2si, is_type, set_form_result, type_of, Handlers};
use engine_core::prelude::*;
use nvse::commands::Read;
use nvse::{handled, CommandArgs};

#[rustfmt::skip]
pub static COMMANDS: Handlers = &[
    (0x260b, get_ammo_trait_numeric),
    (0x260c, set_ammo_trait_numeric),
    (0x260d, get_ammo_projectile),
    (0x260e, set_ammo_projectile),
    (0x2939, set_ammo_casing),
];

/// Argument 0 when the read succeeded and it is an ammo (`IS_TYPE(ammo, TESAmmo)`: its vtable).
pub(super) fn ammo_arg(e: &Engine, read: &Read) -> Option<Ptr<TESAmmo>> {
    let form = read.form(0);
    (read.ok && is_type(e, form, TES_AMMO_VTBL)).then(|| Ptr::new(form))
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_ammo.h:
// Cmd_GetAmmoTraitNumeric_Execute.
/// Trait 0: speed; 1: projectiles per shot (signed); 2: casing chance; 3: clip rounds.
fn get_ammo_trait_numeric(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if let Some(ammo) = ammo_arg(e, &read) {
        let value = match read.int(1, 0) {
            0 => Some(f64::from(e.get(ammo, TESAmmo::speed))),
            1 => Some(f64::from(e.get(ammo, TESAmmo::projPerShot) as i32)),
            2 => Some(f64::from(e.get(ammo, TESAmmo::casingChance))),
            3 => Some(f64::from(e.get(ammo, TESAmmo::clipRounds))),
            _ => None,
        };
        if let Some(v) = value {
            a.set_result(e, v);
        }
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_ammo.h:
// Cmd_SetAmmoTraitNumeric_Execute.
fn set_ammo_trait_numeric(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if let Some(ammo) = ammo_arg(e, &read) {
        let val = read.float(2, 0.0);
        match read.int(1, 0) {
            0 => e.set(ammo, TESAmmo::speed, val),
            1 => e.set(ammo, TESAmmo::projPerShot, cvttss2si(val) as u32),
            2 => e.set(ammo, TESAmmo::casingChance, val),
            3 => e.set(ammo, TESAmmo::clipRounds, cvttss2si(val) as u8),
            _ => {}
        }
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_ammo.h:
// Cmd_GetAmmoProjectile_Execute.
fn get_ammo_projectile(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if let Some(ammo) = ammo_arg(e, &read) {
        let projectile = e.get(ammo, TESAmmo::projectile);
        if projectile != 0 {
            set_form_result(e, &a, projectile);
        }
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_ammo.h:
// Cmd_SetAmmoProjectile_Execute.
/// The projectile set to a projectile form, or cleared without one; another form, nothing.
fn set_ammo_projectile(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if let Some(ammo) = ammo_arg(e, &read) {
        let proj = read.form(1);
        if proj == 0 || type_of(e, proj) == form_type_more::BGS_PROJECTILE {
            e.set(ammo, TESAmmo::projectile, proj);
        }
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_ammo.h:
// Cmd_SetAmmoCasing_Execute.
/// The casing set to an inventory item (`kInventoryType`), or cleared without one.
fn set_ammo_casing(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if let Some(ammo) = ammo_arg(e, &read) {
        let casing = read.form(1);
        if casing == 0 || INVENTORY_TYPE[usize::from(type_of(e, casing))] {
            e.set(ammo, TESAmmo::casingItem, casing);
        }
    }
    handled(true)
}

#[cfg(test)]
#[path = "jip_fn_ammo_tests.rs"]
mod tests;
