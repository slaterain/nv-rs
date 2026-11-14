//! `functions_ln/ln_fn_load_screen.h` (JIP LN NVSE): a load screen's texture and type, and a load
//! screen type's text colour. All 6 of its commands.
//!
//! `TESLoadScreen` (form type 0x4B, 0x40 bytes): the constructor `005A66E0` builds its
//! `TESTexture` at +0x18 (the path, a game string, at +0x1C) and zeroes the type at +0x34.
//! `TESLoadScreenType` (0x6E): its loader `005A72F0` reads `DATA`, 0x58 bytes, at +0x18; the
//! first font colour (`fontColor1`, three floats) is at +0x34 of the form.

use super::arrays::{create_string_map, Element};
use super::{
    assign_string, buffer_arg, set_form_result, string_set, string_text, type_of, Handlers,
};
use engine_core::prelude::*;
use nvse::commands::Read;
use nvse::{handled, CommandArgs};

#[rustfmt::skip]
pub static COMMANDS: Handlers = &[
    (0x229b, get_load_screen_texture),
    (0x229c, set_load_screen_texture),
    (0x229d, get_load_screen_type),
    (0x229e, set_load_screen_type),
    (0x229f, get_load_screen_type_text_rgb),
    (0x22a0, set_load_screen_type_text_rgb),
];

/// `kFormType_TESLoadScreen`, `kFormType_TESLoadScreenType`.
pub const TES_LOAD_SCREEN: u8 = 0x4b;
pub const TES_LOAD_SCREEN_TYPE: u8 = 0x6e;
/// `texture.ddsPath` (+0x1C) and `type` (+0x34) of a load screen; `fontColor1` (+0x34) of a
/// load screen type.
const TEXTURE_PATH: u32 = 0x1c;
const SCREEN_TYPE: u32 = 0x34;
const FONT_COLOR_1: u32 = 0x34;
/// JIP's `char path[0x80]`.
const PATH_BUFFER: usize = 0x80;

fn of_type(e: &Engine, read: &Read, i: usize, kind: u8) -> Option<u32> {
    let f = read.form(i);
    (read.ok && type_of(e, f) == kind).then_some(f)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_load_screen.h:
// Cmd_GetLoadScreenTexture_Execute.
/// The texture's path (`""` without a buffer or for another form).
fn get_load_screen_texture(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let text = of_type(e, &read, 0, TES_LOAD_SCREEN).and_then(|f| string_text(e, f + TEXTURE_PATH));
    assign_string(e, &a, text);
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_load_screen.h:
// Cmd_SetLoadScreenTexture_Execute.
fn set_load_screen_texture(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    // The reader fills JIP's buffer before the type is checked.
    let path = read
        .ok
        .then(|| buffer_arg(e, &read, 1, PATH_BUFFER, "SetLoadScreenTexture"));
    if let (Some(f), Some(path)) = (of_type(e, &read, 0, TES_LOAD_SCREEN), path) {
        string_set(e, f + TEXTURE_PATH, &path);
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_load_screen.h:
// Cmd_GetLoadScreenType_Execute.
fn get_load_screen_type(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if let Some(f) = of_type(e, &read, 0, TES_LOAD_SCREEN) {
        let t = e.mem.u32(f + SCREEN_TYPE);
        if t != 0 {
            set_form_result(e, &a, t);
        }
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_load_screen.h:
// Cmd_SetLoadScreenType_Execute.
fn set_load_screen_type(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if let (Some(f), Some(t)) = (
        of_type(e, &read, 0, TES_LOAD_SCREEN),
        of_type(e, &read, 1, TES_LOAD_SCREEN_TYPE),
    ) {
        e.mem.set_u32(f + SCREEN_TYPE, t);
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_load_screen.h:
// Cmd_GetLoadScreenTypeTextRGB_Execute.
/// A map `r`, `g`, `b` of the first font colour's floats.
fn get_load_screen_type_text_rgb(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if let Some(t) = of_type(e, &read, 0, TES_LOAD_SCREEN_TYPE) {
        let values: Vec<Element> = (0..3)
            .map(|i| Element::Number(f64::from(e.mem.f32(t + FONT_COLOR_1 + 4 * i))))
            .collect();
        let keys = [b"r".to_vec(), b"g".to_vec(), b"b".to_vec()];
        let id = create_string_map(e, a.script, &keys, &values);
        a.set_result(e, f64::from(id as i32));
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_load_screen.h:
// Cmd_SetLoadScreenTypeTextRGB_Execute.
fn set_load_screen_type_text_rgb(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if let Some(t) = of_type(e, &read, 0, TES_LOAD_SCREEN_TYPE) {
        for i in 0..3 {
            let v = read.float(1 + i, 0.0);
            e.mem.set_f32(t + FONT_COLOR_1 + 4 * i as u32, v);
        }
    }
    handled(true)
}

#[cfg(test)]
#[path = "ln_fn_load_screen_tests.rs"]
mod tests;
