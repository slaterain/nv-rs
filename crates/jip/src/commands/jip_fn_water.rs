//! `functions_jip/jip_fn_water.h` (JIP LN NVSE): a water form's noise texture, drink effect and
//! sound. All 5 of its commands. Nothing is narrowed (paths and forms only).

use super::forms::{form_type_more, TESTexture, TESWaterForm};
use super::{
    assign_string, buffer_arg, set_form_result, string_set, string_text, type_of, Handlers,
};
use engine_core::prelude::*;
use nvse::commands::Read;
use nvse::{handled, CommandArgs};

#[rustfmt::skip]
pub static COMMANDS: Handlers = &[
    (0x27c3, get_water_noise_texture),
    (0x27c4, set_water_noise_texture),
    (0x27c5, set_water_form_effect),
    (0x27c6, get_water_sound),
    (0x27c7, set_water_sound),
];

const PATH_BUFFER: usize = 0x80;

fn water_arg(e: &Engine, read: &Read) -> Option<Ptr<TESWaterForm>> {
    let form = read.form(0);
    (read.ok && type_of(e, form) == form_type_more::TES_WATER_FORM).then(|| Ptr::new(form))
}

fn noise_path(w: Ptr<TESWaterForm>) -> u32 {
    w.at(TESWaterForm::noiseMap).at(TESTexture::ddsPath).addr()
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_water.h:
// Cmd_GetWaterNoiseTexture_Execute.
fn get_water_noise_texture(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let text = water_arg(e, &read).and_then(|water| string_text(e, noise_path(water)));
    assign_string(e, &a, text);
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_water.h:
// Cmd_SetWaterNoiseTexture_Execute.
fn set_water_noise_texture(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let path = buffer_arg(e, &read, 1, PATH_BUFFER, "SetWaterNoiseTexture");
    if let Some(water) = water_arg(e, &read) {
        string_set(e, noise_path(water), &path);
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_water.h:
// Cmd_SetWaterFormEffect_Execute.
/// The drink effect set to a spell, or cleared without one; another form, nothing.
fn set_water_form_effect(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if let Some(water) = water_arg(e, &read) {
        let effect = read.form(1);
        if effect == 0 || type_of(e, effect) == form_type_more::SPELL_ITEM {
            e.set(water, TESWaterForm::drinkEffect, effect);
        }
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_water.h:
// Cmd_GetWaterSound_Execute.
fn get_water_sound(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if let Some(water) = water_arg(e, &read) {
        let sound = e.get(water, TESWaterForm::sound);
        if sound != 0 {
            set_form_result(e, &a, sound);
        }
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_water.h:
// Cmd_SetWaterSound_Execute.
fn set_water_sound(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if let Some(water) = water_arg(e, &read) {
        let sound = read.form(1);
        if sound == 0 || type_of(e, sound) == form_type_more::TES_SOUND {
            e.set(water, TESWaterForm::sound, sound);
        }
    }
    handled(true)
}

#[cfg(test)]
#[path = "jip_fn_water_tests.rs"]
mod tests;
