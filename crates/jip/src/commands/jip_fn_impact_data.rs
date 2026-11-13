//! `functions_jip/jip_fn_impact_data.h` (JIP LN NVSE): an impact data's model, numbers, forms and
//! decal, and an impact data set's form per material. All 7 of its commands. Forms are read as
//! any form; each command checks `IS_ID` (impact data 0x5E, impact data set 0x5F).
//!
//! JIP addresses the fields as arrays over the form: `((float*)impactData)[12 + traitID]`,
//! `((TESForm**)impactData)[13 + traitID]`, `((float*)impactData)[13 + traitID]` on reads and
//! `[16 + traitID]` on writes. Here each run is a field and an index from its first trait: the
//! duration +0x30, the angle threshold and placement radius +0x38/+0x3C (traits 2, 3), the texture
//! set and sounds +0x48 (traits 5-7), the decal's seven floats from +0x54 (reads 8-14, writes
//! 5-11).
//!
//! Casts of the script's double, from JIP's source: `UInt32 intVal = cvtd2ul(value)` (JIP's helper:
//! the x87's 64-bit truncation, its low word); `effectOrientation`, `soundLevel`,
//! `parallaxPasses` and `flags` are `UInt8`s given that word (its low byte); the floats are
//! `(float)value`. The colour read is `cvtul2d` (unsigned to double).

use super::forms::{form_type, form_type_more, BGSImpactData, BGSImpactDataSet, DecalInfo};
use super::{
    assign_string, buffer_arg, cvtd2ul, model_path, rgb_dec_to_hex, rgb_hex_to_dec,
    set_form_result, set_model_path, type_of, Handlers,
};
use engine_core::prelude::*;
use nvse::commands::Read;
use nvse::{handled, CommandArgs};

#[rustfmt::skip]
pub static COMMANDS: Handlers = &[
    (0x2790, get_impact_data_model),
    (0x2791, set_impact_data_model),
    (0x2792, get_impact_data_trait),
    (0x2793, set_impact_data_trait_numeric),
    (0x2794, set_impact_data_trait_form),
    (0x2886, get_impact_data_set_form),
    (0x2887, set_impact_data_set_form),
];

const PATH_BUFFER: usize = 0x80;
/// The last material of an impact data set (`impactDatas[12]`).
const LAST_MATERIAL: u32 = 11;

fn impact_data(e: &Engine, form: u32) -> Option<Ptr<BGSImpactData>> {
    (type_of(e, form) == form_type_more::BGS_IMPACT_DATA).then(|| Ptr::new(form))
}

fn impact_data_arg(e: &Engine, read: &Read) -> Option<Ptr<BGSImpactData>> {
    if read.ok {
        impact_data(e, read.form(0))
    } else {
        None
    }
}

fn decal(d: Ptr<BGSImpactData>) -> Ptr<DecalInfo> {
    d.at(BGSImpactData::decalInfo)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_impact_data.h:
// Cmd_GetImpactDataModel_Execute.
fn get_impact_data_model(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let text = match impact_data_arg(e, &read) {
        Some(d) => model_path(e, d.at(BGSImpactData::model).addr()),
        None => None,
    };
    assign_string(e, &a, text);
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_impact_data.h:
// Cmd_SetImpactDataModel_Execute.
fn set_impact_data_model(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let path = buffer_arg(e, &read, 1, PATH_BUFFER, "SetImpactDataModel");
    if let Some(d) = impact_data_arg(e, &read) {
        set_model_path(e, d.at(BGSImpactData::model).addr(), &path);
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_impact_data.h:
// Cmd_GetImpactDataTrait_Execute.
/// Traits 0-4: duration, orientation, angle threshold, placement radius, sound level; 5-7: the
/// texture set and the two sounds (their ids); 8-14: the decal's floats; 15: its parallax passes;
/// 16: its flags shifted up one, with "no decal data" in bit 0; 17: its colour as `RRRGGGBBB`.
fn get_impact_data_trait(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let Some(d) = impact_data_arg(e, &read) else {
        return handled(true);
    };
    let t = read.int(1, 0);
    let value = match t {
        1 => f64::from(e.get(d, BGSImpactData::effectOrientation)),
        0 => f64::from(e.get(d, BGSImpactData::effectDuration)),
        2 | 3 => f64::from(e.get(d.index(t - 2, 4), BGSImpactData::angleThreshold)),
        4 => f64::from(e.get(d, BGSImpactData::soundLevel)),
        5..=7 => {
            let form = e.get(d.index(t - 5, 4), BGSImpactData::textureSet);
            if form != 0 {
                set_form_result(e, &a, form);
            }
            return handled(true);
        }
        8..=14 => f64::from(e.get(decal(d).index(t - 8, 4), DecalInfo::minWidth)),
        15 => f64::from(e.get(decal(d), DecalInfo::parallaxPasses)),
        16 => f64::from(
            (u32::from(e.get(decal(d), DecalInfo::flags)) << 1)
                | u32::from(e.get(d, BGSImpactData::noDecalData)),
        ),
        17 => f64::from(rgb_hex_to_dec(e.get(decal(d), DecalInfo::color))),
        _ => return handled(true),
    };
    a.set_result(e, value);
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_impact_data.h:
// Cmd_SetImpactDataTraitNumeric_Execute.
/// Traits 0-4 as the getter's (orientation and sound level only below 3); 5-11: the decal's
/// floats; 12: its parallax passes; 13: "no decal data" from bit 0 and the flags from the rest;
/// 14: the colour from `RRRGGGBBB` (up to 255255255).
fn set_impact_data_trait_numeric(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let Some(d) = impact_data_arg(e, &read) else {
        return handled(true);
    };
    let value = match read.args.get(2).and_then(nvse::args::Arg::double) {
        Some(v) => v,
        None => 0.0,
    };
    let int_val = cvtd2ul(value);
    let t = read.int(1, 0);
    match t {
        1 if int_val < 3 => e.set(d, BGSImpactData::effectOrientation, int_val as u8),
        0 => e.set(d, BGSImpactData::effectDuration, value as f32),
        2 | 3 => e.set(
            d.index(t - 2, 4),
            BGSImpactData::angleThreshold,
            value as f32,
        ),
        4 if int_val < 3 => e.set(d, BGSImpactData::soundLevel, int_val as u8),
        5..=11 => e.set(decal(d).index(t - 5, 4), DecalInfo::minWidth, value as f32),
        12 => e.set(decal(d), DecalInfo::parallaxPasses, int_val as u8),
        13 => {
            e.set(d, BGSImpactData::noDecalData, (int_val & 1) as u8);
            e.set(decal(d), DecalInfo::flags, (int_val >> 1) as u8);
        }
        14 if int_val <= 255_255_255 => {
            e.set(decal(d), DecalInfo::color, rgb_dec_to_hex(int_val));
        }
        _ => {}
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_impact_data.h:
// Cmd_SetImpactDataTraitForm_Execute.
/// 0: the texture set; 1, 2: the sounds. A missing form or one of another type clears it.
fn set_impact_data_trait_form(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let Some(d) = impact_data_arg(e, &read) else {
        return handled(true);
    };
    let t = read.int(1, 0);
    let kind = match t {
        0 => form_type_more::BGS_TEXTURE_SET,
        1 | 2 => form_type_more::TES_SOUND,
        _ => return handled(true),
    };
    let object = read.form(2);
    let value = if object != 0 && type_of(e, object) == kind {
        object
    } else {
        0
    };
    e.set(d.index(t, 4), BGSImpactData::textureSet, value);
    handled(true)
}

fn impact_data_set_arg(e: &Engine, read: &Read) -> Option<Ptr<BGSImpactDataSet>> {
    let form = read.form(0);
    (read.ok && type_of(e, form) == form_type::BGS_IMPACT_DATA_SET).then(|| Ptr::new(form))
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_impact_data.h:
// Cmd_GetImpactDataSetForm_Execute.
fn get_impact_data_set_form(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let material = read.int(1, 0);
    if let Some(s) = impact_data_set_arg(e, &read).filter(|_| material <= LAST_MATERIAL) {
        let data = e.get(s.index(material, 4), BGSImpactDataSet::impactDatas);
        if data != 0 {
            set_form_result(e, &a, data);
        }
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_impact_data.h:
// Cmd_SetImpactDataSetForm_Execute.
/// The material's impact data set, or cleared without one; another form type, nothing.
fn set_impact_data_set_form(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let material = read.int(1, 0);
    let data = read.form(2);
    if let Some(s) = impact_data_set_arg(e, &read).filter(|_| material <= LAST_MATERIAL) {
        if data == 0 || impact_data(e, data).is_some() {
            e.set(s.index(material, 4), BGSImpactDataSet::impactDatas, data);
        }
    }
    handled(true)
}

#[cfg(test)]
#[path = "jip_fn_impact_data_tests.rs"]
mod tests;
