//! `functions_jip/jip_fn_texture_set.h` (JIP LN NVSE): a texture set's six textures, its decal
//! numbers and its flags. All 6 of its commands.
//!
//! Casts in `SetTextureSetTraitNumeric`, from JIP's source (MSVC, SSE2): a value below 0 (or NaN:
//! `value >= 0` is false) is refused; traits 0-6 store the double as a float (`cvtsd2ss`, to
//! nearest); the parallax passes `(int)value` (`cvttsd2si`) into a byte, its low byte; the colour
//! through `cvtd2ul` (the x87's 64-bit truncation, low word) when its decimal form is at most
//! 255255255.

use super::forms::{form_type_more, BGSTextureSet, DecalInfo, TESTexture};
use super::{
    assign_string, buffer_arg, cvtd2ul, cvttsd2si, rgb_dec_to_hex, rgb_hex_to_dec, string_set,
    string_text, type_of, Handlers,
};
use engine_core::prelude::*;
use engine_core::ptr::Layout;
use nvse::commands::Read;
use nvse::{handled, CommandArgs};

#[rustfmt::skip]
pub static COMMANDS: Handlers = &[
    (0x2665, get_texture_set_texture),
    (0x2666, set_texture_set_texture),
    (0x2667, get_texture_set_trait_numeric),
    (0x2668, set_texture_set_trait_numeric),
    (0x2669, get_texture_set_flag),
    (0x266a, set_texture_set_flag),
];

const PATH_BUFFER: usize = 0x80;

fn set_arg(e: &Engine, read: &Read) -> Option<Ptr<BGSTextureSet>> {
    let form = read.form(0);
    (read.ok && type_of(e, form) == form_type_more::BGS_TEXTURE_SET).then(|| Ptr::new(form))
}

fn texture_path(set: Ptr<BGSTextureSet>, map: u32) -> u32 {
    set.index(map, TESTexture::SIZE)
        .at(BGSTextureSet::textures)
        .at(TESTexture::ddsPath)
        .addr()
}

fn decal(e: &Engine, set: Ptr<BGSTextureSet>) -> Option<Ptr<DecalInfo>> {
    let d = e.get(set, BGSTextureSet::decalInfo);
    (d != 0).then(|| Ptr::new(d))
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_texture_set.h:
// Cmd_GetTextureSetTexture_Execute.
/// Texture map `0` to `5`'s path.
fn get_texture_set_texture(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let map = read.int(1, 0);
    let text = set_arg(e, &read)
        .filter(|_| map <= 5)
        .and_then(|s| string_text(e, texture_path(s, map)));
    assign_string(e, &a, text);
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_texture_set.h:
// Cmd_SetTextureSetTexture_Execute.
fn set_texture_set_texture(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let map = read.int(1, 0);
    let path = buffer_arg(e, &read, 2, PATH_BUFFER, "SetTextureSetTexture");
    if let Some(s) = set_arg(e, &read).filter(|_| map <= 5) {
        string_set(e, texture_path(s, map), &path);
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_texture_set.h:
// Cmd_GetTextureSetTraitNumeric_Execute.
/// With decal data: traits 0-6 its seven floats; 7 the parallax passes; 8 the colour as decimal
/// `RRRGGGBBB`.
fn get_texture_set_trait_numeric(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let Some(d) = set_arg(e, &read).and_then(|s| decal(e, s)) else {
        return handled(true);
    };
    let value = match read.int(1, 0) {
        t @ 0..=6 => Some(f64::from(e.get(d.index(t, 4), DecalInfo::minWidth))),
        7 => Some(f64::from(e.get(d, DecalInfo::parallaxPasses))),
        8 => Some(f64::from(rgb_hex_to_dec(e.get(d, DecalInfo::color)))),
        _ => None,
    };
    if let Some(v) = value {
        a.set_result(e, v);
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_texture_set.h:
// Cmd_SetTextureSetTraitNumeric_Execute.
fn set_texture_set_trait_numeric(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let value = match read.args.get(2) {
        Some(nvse::args::Arg::Double(v)) => *v,
        _ => 0.0,
    };
    let Some(s) = set_arg(e, &read) else {
        return handled(true);
    };
    let Some(d) = decal(e, s).filter(|_| value >= 0.0) else {
        return handled(true);
    };
    match read.int(1, 0) {
        t @ 0..=6 => e.set(d.index(t, 4), DecalInfo::minWidth, value as f32),
        7 => e.set(d, DecalInfo::parallaxPasses, cvttsd2si(value) as u8),
        8 => {
            let dec = cvtd2ul(value);
            if dec <= 255_255_255 {
                e.set(d, DecalInfo::color, rgb_dec_to_hex(dec));
            }
        }
        _ => {}
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_texture_set.h:
// Cmd_GetTextureSetFlag_Execute.
/// Flag 0: the texture set's own flags word is not 0; 1-3: the decal flags' bits 0-2.
fn get_texture_set_flag(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let flag = read.int(1, 0);
    if let Some(s) = set_arg(e, &read).filter(|_| flag <= 3) {
        let set = if flag == 0 {
            e.get(s, BGSTextureSet::texFlags) != 0
        } else {
            decal(e, s).is_some_and(|d| e.get(d, DecalInfo::flags) & (1 << (flag - 1)) != 0)
        };
        if set {
            a.set_result(e, 1.0);
        }
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_texture_set.h:
// Cmd_SetTextureSetFlag_Execute.
/// Flag 0: the flags word set to 1 or 0; 1-3: a decal flag bit, with decal data.
fn set_texture_set_flag(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let flag = read.int(1, 0);
    let on = read.int(2, 0) != 0;
    if let Some(s) = set_arg(e, &read).filter(|_| flag <= 3) {
        if flag == 0 {
            e.set(s, BGSTextureSet::texFlags, u16::from(on));
        } else if let Some(d) = decal(e, s) {
            let bit = 1u8 << (flag - 1);
            let v = e.get(d, DecalInfo::flags);
            e.set(d, DecalInfo::flags, if on { v | bit } else { v & !bit });
        }
    }
    handled(true)
}

#[cfg(test)]
#[path = "jip_fn_texture_set_tests.rs"]
mod tests;
