//! `functions_ln/ln_fn_weapon.h` (JIP LN NVSE): a weapon's kill impulse, impulse distance, VATS
//! effect and critical flags. All 8 of its commands. The weapon is read as an inventory object
//! (`kParamType_ObjectID`) or any form; each command checks `IS_ID(weapon, TESObjectWEAP)` (type
//! 0x28).
//!
//! The weapon loader (`0051F630`) reads `DNAM` (0xCC bytes) at +0xF4 (the kill impulse +0x1A8 and
//! the impulse distance +0x1B8 inside it), `CRDT` (0x10 bytes) at +0x1C0 (the critical flags byte
//! +0x1C8) and `VATS` (0x14 bytes) at +0x370 (the effect first).
//!
//! Cast: `critDamageFlags = flags` (the script's `UInt32` into a `UInt8`: its low byte).

use super::forms::{form_type_ln, form_type_more, TESObjectWEAP};
use super::{set_form_result, type_of, Handlers};
use engine_core::prelude::*;
use nvse::commands::Read;
use nvse::{handled, CommandArgs};

#[rustfmt::skip]
pub static COMMANDS: Handlers = &[
    (0x232b, get_weapon_kill_impulse),
    (0x232c, set_weapon_kill_impulse),
    (0x232d, get_weapon_impulse_distance),
    (0x232e, set_weapon_impulse_distance),
    (0x232f, get_weapon_vats_effect),
    (0x2330, set_weapon_vats_effect),
    (0x2331, get_weapon_crit_flags),
    (0x2332, set_weapon_crit_flags),
];

fn weapon_arg(e: &Engine, read: &Read) -> Option<Ptr<TESObjectWEAP>> {
    let form = read.form(0);
    (read.ok && type_of(e, form) == form_type_ln::TES_OBJECT_WEAP).then(|| Ptr::new(form))
}

fn get_float(e: &mut Engine, w: &[u32], field: Field<TESObjectWEAP, f32>) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if let Some(wp) = weapon_arg(e, &read) {
        let v = e.get(wp, field);
        a.set_result(e, f64::from(v));
    }
    handled(true)
}

fn set_float(e: &mut Engine, w: &[u32], field: Field<TESObjectWEAP, f32>) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if let Some(wp) = weapon_arg(e, &read) {
        e.set(wp, field, read.float(1, 0.0));
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_weapon.h:
// Cmd_GetWeaponKillImpulse_Execute.
fn get_weapon_kill_impulse(e: &mut Engine, w: &[u32]) -> Ret {
    get_float(e, w, TESObjectWEAP::killImpulse)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_weapon.h:
// Cmd_SetWeaponKillImpulse_Execute.
fn set_weapon_kill_impulse(e: &mut Engine, w: &[u32]) -> Ret {
    set_float(e, w, TESObjectWEAP::killImpulse)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_weapon.h:
// Cmd_GetWeaponImpulseDistance_Execute.
fn get_weapon_impulse_distance(e: &mut Engine, w: &[u32]) -> Ret {
    get_float(e, w, TESObjectWEAP::impulseDist)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_weapon.h:
// Cmd_SetWeaponImpulseDistance_Execute.
fn set_weapon_impulse_distance(e: &mut Engine, w: &[u32]) -> Ret {
    set_float(e, w, TESObjectWEAP::impulseDist)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_weapon.h:
// Cmd_GetWeaponVATSEffect_Execute.
fn get_weapon_vats_effect(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if let Some(wp) = weapon_arg(e, &read) {
        let effect = e.get(wp, TESObjectWEAP::VATSEffect);
        if effect != 0 {
            set_form_result(e, &a, effect);
        }
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_weapon.h:
// Cmd_SetWeaponVATSEffect_Execute.
/// The effect set to a spell; another form type, nothing (both forms are required).
fn set_weapon_vats_effect(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let effect = read.form(1);
    if let Some(wp) = weapon_arg(e, &read) {
        if type_of(e, effect) == form_type_more::SPELL_ITEM {
            e.set(wp, TESObjectWEAP::VATSEffect, effect);
        }
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_weapon.h:
// Cmd_GetWeaponCritFlags_Execute.
fn get_weapon_crit_flags(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if let Some(wp) = weapon_arg(e, &read) {
        let v = e.get(wp, TESObjectWEAP::critDamageFlags);
        a.set_result(e, f64::from(v));
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_weapon.h:
// Cmd_SetWeaponCritFlags_Execute.
fn set_weapon_crit_flags(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if let Some(wp) = weapon_arg(e, &read) {
        e.set(wp, TESObjectWEAP::critDamageFlags, read.int(1, 0) as u8);
    }
    handled(true)
}

#[cfg(test)]
#[path = "ln_fn_weapon_tests.rs"]
mod tests;
