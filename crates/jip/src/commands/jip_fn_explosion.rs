//! `functions_jip/jip_fn_explosion.h` (JIP LN NVSE): an explosion's numbers, forms and flags. All
//! 6 of its commands. The explosion is any form or bound object; each command checks
//! `IS_ID(explosion, BGSExplosion)` (form type 0x51).
//!
//! Casts of the script's float, from JIP's source (MSVC, SSE2): `soundLevel = val` (a `UInt8`
//! from a float: `cvttss2si`, its low byte); the other numbers are floats stored as they are.

use super::forms::{form_type, form_type_more, BGSExplosion};
use super::{cvttss2si, set_form_result, type_of, Handlers};
use engine_core::prelude::*;
use nvse::commands::Read;
use nvse::{handled, CommandArgs};

#[rustfmt::skip]
pub static COMMANDS: Handlers = &[
    (0x261b, get_explosion_trait_numeric),
    (0x261c, set_explosion_trait_numeric),
    (0x261d, get_explosion_trait_form),
    (0x261e, set_explosion_trait_form),
    (0x261f, get_explosion_flag),
    (0x2620, set_explosion_flag),
];

fn explosion_arg(e: &Engine, read: &Read) -> Option<Ptr<BGSExplosion>> {
    let form = read.form(0);
    (read.ok && type_of(e, form) == form_type::BGS_EXPLOSION).then(|| Ptr::new(form))
}

/// The float field of numeric trait `t` (0 force, 1 damage, 2 radius, 3 image space radius,
/// 4 radiation level, 5 dissipation time, 6 radiation radius); 7 is the sound level byte.
fn float_trait(t: u32) -> Option<Field<BGSExplosion, f32>> {
    Some(match t {
        0 => BGSExplosion::force,
        1 => BGSExplosion::damage,
        2 => BGSExplosion::radius,
        3 => BGSExplosion::ISradius,
        4 => BGSExplosion::RADlevel,
        5 => BGSExplosion::dissipationTime,
        6 => BGSExplosion::RADradius,
        _ => return None,
    })
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_explosion.h:
// Cmd_GetExplosionTraitNumeric_Execute.
fn get_explosion_trait_numeric(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if let Some(x) = explosion_arg(e, &read) {
        let t = read.int(1, 0);
        let value = match float_trait(t) {
            Some(field) => Some(f64::from(e.get(x, field))),
            None if t == 7 => Some(f64::from(e.get(x, BGSExplosion::soundLevel))),
            None => None,
        };
        if let Some(v) = value {
            a.set_result(e, v);
        }
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_explosion.h:
// Cmd_SetExplosionTraitNumeric_Execute.
fn set_explosion_trait_numeric(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if let Some(x) = explosion_arg(e, &read) {
        let t = read.int(1, 0);
        let val = read.float(2, 0.0);
        match float_trait(t) {
            Some(field) => e.set(x, field, val),
            None if t == 7 => e.set(x, BGSExplosion::soundLevel, cvttss2si(val) as u8),
            None => {}
        }
    }
    handled(true)
}

/// The form field of form trait `t` and the form type a set needs (`None`: any form, the placed
/// object): 0 light, 1 and 2 the sounds, 3 the impact data set, 4 the placed object, 5 the
/// enchantment (`enchantable.enchantItem`), 6 the image space modifier
/// (`imageSpaceModForm.imod`).
fn form_trait(t: u32) -> Option<(Field<BGSExplosion, u32>, Option<u8>)> {
    Some(match t {
        0 => (BGSExplosion::light, Some(form_type_more::TES_OBJECT_LIGH)),
        1 => (BGSExplosion::sound1, Some(form_type_more::TES_SOUND)),
        2 => (BGSExplosion::sound2, Some(form_type_more::TES_SOUND)),
        3 => (
            BGSExplosion::impactDataSet,
            Some(form_type::BGS_IMPACT_DATA_SET),
        ),
        4 => (BGSExplosion::placedObj, None),
        5 => (
            BGSExplosion::enchantItem,
            Some(form_type_more::ENCHANTMENT_ITEM),
        ),
        6 => (
            BGSExplosion::imod,
            Some(form_type_more::TES_IMAGE_SPACE_MODIFIER),
        ),
        _ => return None,
    })
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_explosion.h:
// Cmd_GetExplosionTraitForm_Execute.
fn get_explosion_trait_form(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if let Some(x) = explosion_arg(e, &read) {
        if let Some((field, _)) = form_trait(read.int(1, 0)) {
            let form = e.get(x, field);
            if form != 0 {
                set_form_result(e, &a, form);
            }
        }
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_explosion.h:
// Cmd_SetExplosionTraitForm_Execute.
/// The form set, or the field cleared when the form is missing or of another type.
fn set_explosion_trait_form(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if let Some(x) = explosion_arg(e, &read) {
        if let Some((field, kind)) = form_trait(read.int(1, 0)) {
            let object = read.form(2);
            let value = match kind {
                Some(k) if object != 0 && type_of(e, object) == k => object,
                Some(_) => 0,
                None => object,
            };
            e.set(x, field, value);
        }
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_explosion.h:
// Cmd_GetExplosionFlag_Execute.
/// Flags 1-31 (`explFlags`' bits); flag 0 is never answered.
fn get_explosion_flag(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let flag = read.int(1, 0);
    if let Some(x) = explosion_arg(e, &read).filter(|_| flag != 0 && flag <= 31) {
        if e.get(x, BGSExplosion::explFlags) & (1 << flag) != 0 {
            a.set_result(e, 1.0);
        }
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_explosion.h:
// Cmd_SetExplosionFlag_Execute (with BGSExplosion::SetFlag).
fn set_explosion_flag(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let flag = read.int(1, 0);
    if let Some(x) = explosion_arg(e, &read).filter(|_| flag != 0 && flag <= 31) {
        let flags = e.get(x, BGSExplosion::explFlags);
        let bit = 1u32 << flag;
        let flags = if read.int(2, 0) != 0 {
            flags | bit
        } else {
            flags & !bit
        };
        e.set(x, BGSExplosion::explFlags, flags);
    }
    handled(true)
}

#[cfg(test)]
#[path = "jip_fn_explosion_tests.rs"]
mod tests;
