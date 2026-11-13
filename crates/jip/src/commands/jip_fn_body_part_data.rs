//! `functions_jip/jip_fn_body_part_data.h` (JIP LN NVSE): a body part data form's skeleton and
//! ragdoll, and its parts' numbers, forms, names, limb replacement models and flags. All 15 of
//! its commands. The game reads these fields when it uses a part (a hit, a dismemberment), so a
//! set takes effect from then on, as with JIP.
//!
//! The float-to-byte stores of `SetBodyPartTraitNumeric` are the compiled C++'s conversion:
//! `cvttss2si` to a 32-bit integer, its low byte stored.

use super::forms::{form_type, BGSBodyPart, BGSBodyPartData};
use super::{
    assign_string, buffer_arg, cvttss2si, model_path, set_form_result, set_model_path, string_set,
    string_text, type_of, Handlers, DBL_180_D_PI, FLT_PI_D_180,
};
use engine_core::prelude::*;
use engine_core::ptr::Field;
use nvse::commands::Read;
use nvse::{handled, CommandArgs};

#[rustfmt::skip]
pub static COMMANDS: Handlers = &[
    (0x26ac, get_body_part_data_skeleton),
    (0x26ad, set_body_part_data_skeleton),
    (0x26ae, get_body_part_data_ragdoll),
    (0x26af, set_body_part_data_ragdoll),
    (0x26b0, get_body_part_data_has_part),
    (0x26b1, get_body_part_trait_numeric),
    (0x26b2, set_body_part_trait_numeric),
    (0x26b3, get_body_part_trait_form),
    (0x26b4, set_body_part_trait_form),
    (0x26b5, get_body_part_name),
    (0x26b6, set_body_part_name),
    (0x26b7, get_body_part_replacement_model),
    (0x26b8, set_body_part_replacement_model),
    (0x26b9, get_body_part_flag),
    (0x26ba, set_body_part_flag),
];

/// JIP's buffers: `char path[0x80]`, `char nameStr[0x40]`.
const PATH_BUFFER: usize = 0x80;
const NAME_BUFFER: usize = 0x40;
/// The last part index (`eBodyPart_Weapon`, 14).
const LAST_PART: u32 = 14;

/// The body part data argument 0 when the read succeeded and it is one.
fn data_arg(e: &Engine, read: &Read) -> Option<Ptr<BGSBodyPartData>> {
    let form = read.form(0);
    (read.ok && type_of(e, form) == form_type::BGS_BODY_PART_DATA).then(|| Ptr::new(form))
}

/// `bpData->bodyParts[part]` (which may be null).
fn part_of(e: &Engine, data: Ptr<BGSBodyPartData>, part: u32) -> u32 {
    e.get(data.index(part, 4), BGSBodyPartData::bodyParts)
}

/// The part argument 1 of a body part data argument 0, when it is at most 14 and `ok` holds
/// (the command's further range checks, made before the part is read, as JIP's `&&` order).
fn part_arg(e: &Engine, read: &Read, ok: bool) -> Option<Ptr<BGSBodyPart>> {
    let data = data_arg(e, read)?;
    let part = read.int(1, 0);
    if part > LAST_PART || !ok {
        return None;
    }
    let p = part_of(e, data, part);
    (p != 0).then(|| Ptr::new(p))
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_body_part_data.h:
// Cmd_GetBodyPartDataSkeleton_Execute.
fn get_body_part_data_skeleton(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let text = match data_arg(e, &read) {
        Some(d) => model_path(e, d.at(BGSBodyPartData::model).addr()),
        None => None,
    };
    assign_string(e, &a, text);
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_body_part_data.h:
// Cmd_SetBodyPartDataSkeleton_Execute.
fn set_body_part_data_skeleton(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let path = buffer_arg(e, &read, 1, PATH_BUFFER, "SetBodyPartDataSkeleton");
    if let Some(d) = data_arg(e, &read) {
        set_model_path(e, d.at(BGSBodyPartData::model).addr(), &path);
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_body_part_data.h:
// Cmd_GetBodyPartDataRagdoll_Execute.
fn get_body_part_data_ragdoll(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if let Some(d) = data_arg(e, &read) {
        let ragdoll = e.get(d, BGSBodyPartData::ragDoll);
        if ragdoll != 0 {
            set_form_result(e, &a, ragdoll);
        }
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_body_part_data.h:
// Cmd_SetBodyPartDataRagdoll_Execute.
/// The ragdoll set to a ragdoll form, or cleared without one; another form changes nothing.
fn set_body_part_data_ragdoll(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if let Some(d) = data_arg(e, &read) {
        let ragdoll = read.form(1);
        if ragdoll == 0 || type_of(e, ragdoll) == form_type::BGS_RAGDOLL {
            e.set(d, BGSBodyPartData::ragDoll, ragdoll);
        }
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_body_part_data.h:
// Cmd_GetBodyPartDataHasPart_Execute.
fn get_body_part_data_has_part(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if part_arg(e, &read, true).is_some() {
        a.set_result(e, 1.0);
    }
    handled(true)
}

/// A numeric trait's field: 0 actor value, 1 damage multiplier, 2 health percent, 3 to-hit
/// chance, 4 tracking angle, 5 severed debris count, 6 severed debris scale, 7 severed decal
/// count, 8 explode chance, 9 limb replacement scale, 10 explode debris count, 11 explode debris
/// scale, 12 explode decal count, 13-15 gore translation, 16-18 gore rotation.
enum Numeric {
    Byte(Field<BGSBodyPart, u8>),
    Float(Field<BGSBodyPart, f32>),
    /// `goreEffTranslate[i]`.
    Translate(u32),
    /// `goreEffRotation[i]`, radians.
    Rotate(u32),
}

fn numeric(trait_id: u32) -> Numeric {
    use Numeric::*;
    match trait_id {
        0 => Byte(BGSBodyPart::actorValue),
        1 => Float(BGSBodyPart::damageMult),
        2 => Byte(BGSBodyPart::healthPercent),
        3 => Byte(BGSBodyPart::toHitChance),
        4 => Float(BGSBodyPart::trackingMaxAngle),
        5 => Byte(BGSBodyPart::sevrDebrisCount),
        6 => Float(BGSBodyPart::sevrDebrisScale),
        7 => Byte(BGSBodyPart::sevrDecalCount),
        8 => Byte(BGSBodyPart::explChance),
        9 => Float(BGSBodyPart::limbRepScale),
        10 => Byte(BGSBodyPart::explDebrisCount),
        11 => Float(BGSBodyPart::explDebrisScale),
        12 => Byte(BGSBodyPart::explDecalCount),
        13..=15 => Translate(trait_id - 13),
        _ => Rotate(trait_id - 16),
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_body_part_data.h:
// Cmd_GetBodyPartTraitNumeric_Execute.
/// A part's numeric trait (0 to 18; see [`Numeric`]); the gore rotation in degrees (the float
/// times `Dbl180dPI`, in doubles).
fn get_body_part_trait_numeric(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let trait_id = read.int(2, 0);
    if let Some(p) = part_arg(e, &read, trait_id <= 18) {
        let value = match numeric(trait_id) {
            Numeric::Byte(f) => f64::from(e.get(p, f)),
            Numeric::Float(f) => f64::from(e.get(p, f)),
            Numeric::Translate(i) => f64::from(e.get(p.index(i, 4), BGSBodyPart::goreEffTranslate)),
            Numeric::Rotate(i) => {
                f64::from(e.get(p.index(i, 4), BGSBodyPart::goreEffRotation)) * DBL_180_D_PI
            }
        };
        a.set_result(e, value);
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_body_part_data.h:
// Cmd_SetBodyPartTraitNumeric_Execute.
/// A part's numeric trait set from a value of at least 0 (a negative one changes nothing); a
/// byte field takes the float converted toward zero, the gore rotation the degrees times
/// `FltPId180` (in floats).
fn set_body_part_trait_numeric(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let trait_id = read.int(2, 0);
    let val = read.float(3, 0.0);
    if let Some(p) = part_arg(e, &read, trait_id <= 18 && val >= 0.0) {
        match numeric(trait_id) {
            Numeric::Byte(f) => e.set(p, f, cvttss2si(val) as u8),
            Numeric::Float(f) => e.set(p, f, val),
            Numeric::Translate(i) => e.set(p.index(i, 4), BGSBodyPart::goreEffTranslate, val),
            Numeric::Rotate(i) => e.set(
                p.index(i, 4),
                BGSBodyPart::goreEffRotation,
                val * FLT_PI_D_180,
            ),
        }
    }
    handled(true)
}

/// A form trait's field and the form type it takes: 0 severed explosion, 1 explode explosion,
/// 2 severed debris, 3 explode debris, 4 severed impact data set, 5 explode impact data set.
fn form_trait(trait_id: u32) -> (Field<BGSBodyPart, u32>, u8) {
    match trait_id {
        0 => (BGSBodyPart::sevrExplosion, form_type::BGS_EXPLOSION),
        1 => (BGSBodyPart::explExplosion, form_type::BGS_EXPLOSION),
        2 => (BGSBodyPart::sevrDebris, form_type::BGS_DEBRIS),
        3 => (BGSBodyPart::explDebris, form_type::BGS_DEBRIS),
        4 => (BGSBodyPart::sevrImpactDS, form_type::BGS_IMPACT_DATA_SET),
        _ => (BGSBodyPart::explImpactDS, form_type::BGS_IMPACT_DATA_SET),
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_body_part_data.h:
// Cmd_GetBodyPartTraitForm_Execute.
fn get_body_part_trait_form(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let trait_id = read.int(2, 0);
    if let Some(p) = part_arg(e, &read, trait_id <= 5) {
        let form = e.get(p, form_trait(trait_id).0);
        if form != 0 {
            set_form_result(e, &a, form);
        }
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_body_part_data.h:
// Cmd_SetBodyPartTraitForm_Execute.
/// A form trait set to a form of its type; no form, or one of another type, clears it.
fn set_body_part_trait_form(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let trait_id = read.int(2, 0);
    if let Some(p) = part_arg(e, &read, trait_id <= 5) {
        let (field, kind) = form_trait(trait_id);
        let object = read.form(3);
        let value = if object != 0 && type_of(e, object) == kind {
            object
        } else {
            0
        };
        e.set(p, field, value);
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_body_part_data.h:
// Cmd_GetBodyPartName_Execute.
/// The part's name, when its length is not 0.
fn get_body_part_name(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let text = part_arg(e, &read, true).and_then(|p| {
        let name = p.at(BGSBodyPart::partName);
        if e.get(name, super::forms::BSStringT::sLen) != 0 {
            string_text(e, name.addr())
        } else {
            None
        }
    });
    assign_string(e, &a, text);
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_body_part_data.h:
// Cmd_SetBodyPartName_Execute.
fn set_body_part_name(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let name = buffer_arg(e, &read, 2, NAME_BUFFER, "SetBodyPartName");
    if let Some(p) = part_arg(e, &read, true) {
        string_set(e, p.at(BGSBodyPart::partName).addr(), &name);
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_body_part_data.h:
// Cmd_GetBodyPartReplacementModel_Execute.
fn get_body_part_replacement_model(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let text = match part_arg(e, &read, true) {
        Some(p) => model_path(e, p.at(BGSBodyPart::limbReplacement).addr()),
        None => None,
    };
    assign_string(e, &a, text);
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_body_part_data.h:
// Cmd_SetBodyPartReplacementModel_Execute.
fn set_body_part_replacement_model(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let path = buffer_arg(e, &read, 2, PATH_BUFFER, "SetBodyPartReplacementModel");
    if let Some(p) = part_arg(e, &read, true) {
        set_model_path(e, p.at(BGSBodyPart::limbReplacement).addr(), &path);
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_body_part_data.h:
// Cmd_GetBodyPartFlag_Execute.
/// 1 when flag bit `0` to `7` is set.
fn get_body_part_flag(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let flag = read.int(2, 0);
    if let Some(p) = part_arg(e, &read, flag <= 7) {
        if e.get(p, BGSBodyPart::flags) & (1 << flag) != 0 {
            a.set_result(e, 1.0);
        }
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_body_part_data.h:
// Cmd_SetBodyPartFlag_Execute; BGSBodyPart::SetFlag.
/// Flag bit `0` to `7` set for a nonzero value, cleared for 0.
fn set_body_part_flag(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let flag = read.int(2, 0);
    if let Some(p) = part_arg(e, &read, flag <= 7) {
        let flags = e.get(p, BGSBodyPart::flags);
        let bit = 1u8 << flag;
        let flags = if read.int(3, 0) != 0 {
            flags | bit
        } else {
            flags & !bit
        };
        e.set(p, BGSBodyPart::flags, flags);
    }
    handled(true)
}

#[cfg(test)]
#[path = "jip_fn_body_part_data_tests.rs"]
mod tests;
