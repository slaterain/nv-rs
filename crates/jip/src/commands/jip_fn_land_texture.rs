//! `functions_jip/jip_fn_land_texture.h` (JIP LN NVSE): a land texture's texture set, numbers and
//! grass list. All 8 of its commands. Forms are read as any form; each command checks `IS_ID`
//! (land texture 0x12, texture set 0x04, grass 0x24).
//!
//! The numbers are bytes set from the script's integer (`UInt32 val` stored into a `UInt8`: its
//! low byte; the material type only below 32). Nothing is narrowed from a float.

use super::forms::{form_type_more, TESLandTexture};
use super::{list_contains, list_count, list_prepend, list_remove, nth_item, set_form_result};
use super::{type_of, Handlers};
use engine_core::prelude::*;
use nvse::commands::Read;
use nvse::{handled, CommandArgs};

#[rustfmt::skip]
pub static COMMANDS: Handlers = &[
    (0x265d, get_land_texture_texture_set),
    (0x265e, set_land_texture_texture_set),
    (0x265f, get_land_texture_trait_numeric),
    (0x2660, set_land_texture_trait_numeric),
    (0x2661, get_land_texture_num_grasses),
    (0x2662, get_land_texture_nth_grass),
    (0x2663, land_texture_add_grass),
    (0x2664, land_texture_remove_grass),
];

fn land_texture_arg(e: &Engine, read: &Read) -> Option<Ptr<TESLandTexture>> {
    let form = read.form(0);
    (read.ok && type_of(e, form) == form_type_more::TES_LAND_TEXTURE).then(|| Ptr::new(form))
}

/// Argument 1 when it is of `kind`.
fn second_of(e: &Engine, read: &Read, kind: u8) -> Option<u32> {
    let form = read.form(1);
    (type_of(e, form) == kind).then_some(form)
}

/// The byte of trait `t`: 0 material type, 1 friction, 2 restitution, 3 specular exponent.
fn byte_trait(t: u32) -> Option<Field<TESLandTexture, u8>> {
    Some(match t {
        0 => TESLandTexture::materialType,
        1 => TESLandTexture::friction,
        2 => TESLandTexture::restitution,
        3 => TESLandTexture::specularExponent,
        _ => return None,
    })
}

fn grasses(l: Ptr<TESLandTexture>) -> Ptr<super::forms::ListNode> {
    l.at(TESLandTexture::grasses)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_land_texture.h:
// Cmd_GetLandTextureTextureSet_Execute.
fn get_land_texture_texture_set(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if let Some(l) = land_texture_arg(e, &read) {
        let set = e.get(l, TESLandTexture::textureSet);
        if set != 0 {
            set_form_result(e, &a, set);
        }
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_land_texture.h:
// Cmd_SetLandTextureTextureSet_Execute.
fn set_land_texture_texture_set(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if let Some(l) = land_texture_arg(e, &read) {
        if let Some(set) = second_of(e, &read, form_type_more::BGS_TEXTURE_SET) {
            e.set(l, TESLandTexture::textureSet, set);
        }
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_land_texture.h:
// Cmd_GetLandTextureTraitNumeric_Execute.
fn get_land_texture_trait_numeric(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if let Some(l) = land_texture_arg(e, &read) {
        if let Some(field) = byte_trait(read.int(1, 0)) {
            let v = e.get(l, field);
            a.set_result(e, f64::from(v));
        }
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_land_texture.h:
// Cmd_SetLandTextureTraitNumeric_Execute.
fn set_land_texture_trait_numeric(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if let Some(l) = land_texture_arg(e, &read) {
        let t = read.int(1, 0);
        let val = read.int(2, 0);
        if let Some(field) = byte_trait(t).filter(|_| t != 0 || val < 32) {
            e.set(l, field, val as u8);
        }
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_land_texture.h:
// Cmd_GetLandTextureNumGrasses_Execute.
fn get_land_texture_num_grasses(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if let Some(l) = land_texture_arg(e, &read) {
        let count = list_count(e, grasses(l));
        a.set_result(e, f64::from(count as i32));
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_land_texture.h:
// Cmd_GetLandTextureNthGrass_Execute.
/// The grass of node `idx` (`GetNthItem` takes the script's `UInt32` as a signed index).
fn get_land_texture_nth_grass(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if let Some(l) = land_texture_arg(e, &read) {
        let grass = nth_item(e, grasses(l), read.int(1, 0) as i32);
        if grass != 0 {
            set_form_result(e, &a, grass);
        }
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_land_texture.h:
// Cmd_LandTextureAddGrass_Execute.
/// A grass not yet in the list put first (1); one already there, nothing.
fn land_texture_add_grass(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if let Some(l) = land_texture_arg(e, &read) {
        if let Some(grass) = second_of(e, &read, form_type_more::TES_GRASS) {
            if !list_contains(e, grasses(l), grass) {
                list_prepend(e, grasses(l), grass);
                a.set_result(e, 1.0);
            }
        }
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_land_texture.h:
// Cmd_LandTextureRemoveGrass_Execute.
fn land_texture_remove_grass(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if let Some(l) = land_texture_arg(e, &read) {
        if let Some(grass) = second_of(e, &read, form_type_more::TES_GRASS) {
            if list_remove(e, grasses(l), grass) {
                a.set_result(e, 1.0);
            }
        }
    }
    handled(true)
}

#[cfg(test)]
#[path = "jip_fn_land_texture_tests.rs"]
mod tests;
