//! `functions_jip/jip_fn_ammo_effect.h` (JIP LN NVSE): an ammo's effect list and an ammo effect's
//! numbers. All 6 of its commands.
//!
//! Cast of the script's float, from JIP's source (MSVC, SSE2): `UInt32 iVal = (int)fVal`
//! (`cvttss2si`) for the type and the operation; the value is stored as the float.

use super::forms::{form_type_more, TESAmmo, TESAmmoEffect};
use super::jip_fn_ammo::ammo_arg;
use super::{
    cvttss2si, list_contains, list_count, list_prepend, list_remove, nth_item, set_form_result,
    type_of, Handlers,
};
use engine_core::prelude::*;
use nvse::commands::Read;
use nvse::{handled, CommandArgs};

#[rustfmt::skip]
pub static COMMANDS: Handlers = &[
    (0x260f, get_num_ammo_effects),
    (0x2610, get_nth_ammo_effect),
    (0x2611, add_ammo_effect),
    (0x2612, remove_ammo_effect),
    (0x2613, get_ammo_effect_trait_numeric),
    (0x2614, set_ammo_effect_trait_numeric),
];

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_ammo_effect.h:
// Cmd_GetNumAmmoEffects_Execute.
fn get_num_ammo_effects(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if let Some(ammo) = ammo_arg(e, &read) {
        let count = list_count(e, ammo.at(TESAmmo::effectList));
        a.set_result(e, f64::from(count as i32));
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_ammo_effect.h:
// Cmd_GetNthAmmoEffect_Execute.
fn get_nth_ammo_effect(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if let Some(ammo) = ammo_arg(e, &read) {
        let effect = nth_item(e, ammo.at(TESAmmo::effectList), read.int(1, 0) as i32);
        if effect != 0 {
            set_form_result(e, &a, effect);
        }
    }
    handled(true)
}

/// An ammo argument 0 and an ammo effect argument 1.
fn ammo_and_effect(e: &Engine, read: &Read) -> Option<(Ptr<TESAmmo>, u32)> {
    let ammo = ammo_arg(e, read)?;
    let effect = read.form(1);
    (type_of(e, effect) == form_type_more::TES_AMMO_EFFECT).then_some((ammo, effect))
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_ammo_effect.h:
// Cmd_AddAmmoEffect_Execute.
/// The effect put first in the list when it is not there yet; the result 1.
fn add_ammo_effect(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if let Some((ammo, effect)) = ammo_and_effect(e, &read) {
        let list = ammo.at(TESAmmo::effectList);
        if !list_contains(e, list, effect) {
            list_prepend(e, list, effect);
            a.set_result(e, 1.0);
        }
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_ammo_effect.h:
// Cmd_RemoveAmmoEffect_Execute.
fn remove_ammo_effect(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if let Some((ammo, effect)) = ammo_and_effect(e, &read) {
        if list_remove(e, ammo.at(TESAmmo::effectList), effect) {
            a.set_result(e, 1.0);
        }
    }
    handled(true)
}

/// An ammo effect argument 0 (`IS_ID(effect, TESAmmoEffect)`).
fn effect_arg(e: &Engine, read: &Read) -> Option<Ptr<TESAmmoEffect>> {
    let form = read.form(0);
    (read.ok && type_of(e, form) == form_type_more::TES_AMMO_EFFECT).then(|| Ptr::new(form))
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_ammo_effect.h:
// Cmd_GetAmmoEffectTraitNumeric_Execute.
/// Trait 0: type; 1: operation (both signed); 2: value.
fn get_ammo_effect_trait_numeric(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if let Some(effect) = effect_arg(e, &read) {
        let value = match read.int(1, 0) {
            0 => Some(f64::from(e.get(effect, TESAmmoEffect::effectType) as i32)),
            1 => Some(f64::from(e.get(effect, TESAmmoEffect::operation) as i32)),
            2 => Some(f64::from(e.get(effect, TESAmmoEffect::value))),
            _ => None,
        };
        if let Some(v) = value {
            a.set_result(e, v);
        }
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_ammo_effect.h:
// Cmd_SetAmmoEffectTraitNumeric_Execute.
/// From a value of at least 0: the type when below 6, the operation when below 3 (both from
/// `UInt32 iVal = (int)fVal`), the value as given.
fn set_ammo_effect_trait_numeric(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let f_val = read.float(2, 0.0);
    if let Some(effect) = effect_arg(e, &read).filter(|_| f_val >= 0.0) {
        let i_val = cvttss2si(f_val) as u32;
        match read.int(1, 0) {
            0 if i_val < 6 => e.set(effect, TESAmmoEffect::effectType, i_val),
            1 if i_val < 3 => e.set(effect, TESAmmoEffect::operation, i_val),
            2 => e.set(effect, TESAmmoEffect::value, f_val),
            _ => {}
        }
    }
    handled(true)
}

#[cfg(test)]
#[path = "jip_fn_ammo_effect_tests.rs"]
mod tests;
