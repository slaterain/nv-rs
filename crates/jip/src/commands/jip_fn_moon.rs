//! `SetMoonTexture` (`functions_jip/jip_fn_miscellaneous.h`, JIP LN NVSE): one of the moon's
//! eight phase textures replaced. JIP rebuilds the sky's moon (`Sky::RefreshMoon`) with a jump
//! written into the moon's constructor (`0x634B30`, `InitMoonHook`) for just that call, so the
//! new moon takes JIP's eight names instead of the game's. The port has a seam there
//! (engine-fallout-env's `moon.rs`), asked while JIP is installed, answered only while this
//! command rebuilds the moon. The names JIP set stay for its next calls (never saved: a new
//! session starts with the game's moon).

use super::Handlers;
use crate::game_patches::Area;
use engine_core::extend::{Owner, SiteSlot};
use engine_core::prelude::*;
use nvse::commands::Read;
use nvse::{handled, CommandArgs};
use std::sync::Mutex;

#[rustfmt::skip]
pub static COMMANDS: Handlers = &[
    (0x27a4, set_moon_texture),
];

/// The moon constructor's names (`0x634B30`).
pub const MOON_NAMES: SiteSlot = SiteSlot::new(Owner::Jip, 0x0063_4b30);

/// JIP's `kMoonTexturesDefault` (phase 4, no moon, has none).
const DEFAULTS: [Option<&[u8]>; 8] = [
    Some(b"textures\\sky\\masser_full.dds"),
    Some(b"textures\\sky\\masser_three_wan.dds"),
    Some(b"textures\\sky\\masser_half_wan.dds"),
    Some(b"textures\\sky\\masser_one_wan.dds"),
    None,
    Some(b"textures\\sky\\masser_one_wax.dds"),
    Some(b"textures\\sky\\masser_half_wax.dds"),
    Some(b"textures\\sky\\masser_three_wax.dds"),
];

/// Eight names, a phase's `None` for none.
type Names = Vec<Option<Vec<u8>>>;

/// JIP's `s_moonTexturesOverride` (the names the command gave) and `s_moonTexturesSet` (the
/// names the moon being built takes; `Some` only while the command rebuilds it).
#[derive(Default)]
pub struct MoonTextures {
    state: Mutex<(Names, Option<Names>)>,
}

fn textures(e: &mut Engine) -> &MoonTextures {
    e.ext_free::<MoonTextures>()
}

/// The slot (made before the threads start).
pub fn install(e: &mut Engine) {
    e.ext_free::<MoonTextures>();
}

/// The moon's seam: in with JIP's `INSTALLED`, as JIP's other patches.
pub const AREA: Area = Area {
    name: "moon textures",
    fix_sites: &[],
    fix_wraps: &[],
    install_fixes: crate::game_patches::nothing,
    jip_sites: &[MOON_NAMES.addr()],
    jip_wraps: &[],
    install_jip,
};

fn install_jip(e: &mut Engine) {
    e.on_site(MOON_NAMES, moon_names);
}

/// A moon's eight names (`+0x18`, 8 bytes apart).
const MOON_TEXTURES: u32 = 0x18;
/// A game string's text (`+0`), its length (`+4`, 16 bits) and buffer size (`+6`).
const STRING_DATA: u32 = 0;
const STRING_LEN: u32 = 4;
const STRING_MAX: u32 = 6;

// Translated from JIP LN NVSE 57.30 (5a30ac4), nvse/GameTypes.cpp: String::Set (a null text).
/// The string emptied and its buffer freed (`String::Set(nullptr)`).
fn string_clear(e: &mut Engine, string: u32) {
    e.mem.set_u16(string + STRING_LEN, 0);
    let data = e.mem.u32(string + STRING_DATA);
    if data != 0 {
        super::heap_free(e, data);
        e.mem.set_u32(string + STRING_DATA, 0);
    }
    e.mem.set_u16(string + STRING_MAX, 0);
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_miscellaneous.h: InitMoonHook
// (asm).
/// The moon being built (`a`: the moon), while SetMoonTexture rebuilds it: its eight names set to
/// JIP's (`String::Set` each; phase 4's default is none). Answers 0; nothing otherwise.
fn moon_names(e: &mut Engine, a: &[u32]) -> Option<u32> {
    let set = textures(e)
        .state
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .1
        .clone()?;
    let moon = a[0];
    for (i, name) in set.iter().enumerate() {
        let string = moon + MOON_TEXTURES + 8 * i as u32;
        match name {
            Some(text) => super::string_set(e, string, text),
            None => string_clear(e, string),
        }
    }
    Some(0)
}

/// `Sky::Get` (`*011DEA20`), its node `+8` and its moon (`masserMoon`, `+0x30`); a moon's size
/// (0x7C), its constructor (`00634A70`, thiscall: the moon, the name prefix, five `float`s and
/// an int), its `Destroy` (slot 0) and `Refresh` (slot `0x10`: a node, the prefix); the prefix
/// `"Masser"` (`0104EEB0`) and the moon's game settings (their values at +4).
const SKY: u32 = 0x011d_ea20;
const SKY_NODE: u32 = 0x08;
const SKY_MOON: u32 = 0x30;
const MOON_SIZE: u32 = 0x7c;
const MOON_CONSTRUCTOR: u32 = 0x0063_4a70;
const MOON_PREFIX: u32 = 0x0104_eeb0;
const F_MASSER_ANGLE_FADE_START: u32 = 0x011c_ccb8 + 4;
const F_MASSER_ANGLE_FADE_END: u32 = 0x011c_cc94 + 4;
const F_MASSER_ANGLE_SHADOW_EARLY_FADE: u32 = 0x011c_cba4 + 4;
const F_MASSER_SPEED: u32 = 0x011c_cbfc + 4;
const F_MASSER_Z_OFFSET: u32 = 0x011c_cc54 + 4;
const I_MASSER_SIZE: u32 = 0x011c_cc18 + 4;

// Translated from JIP LN NVSE 57.30 (5a30ac4), nvse/GameData.cpp: Sky::RefreshMoon.
/// The sky's moon destroyed (when it has one) and a new one built from the game settings and
/// refreshed under the sky's node.
fn refresh_moon(e: &mut Engine, sky: u32) {
    let old = e.mem.u32(sky + SKY_MOON);
    if old != 0 {
        e.vcall(old, 0, &[1]);
    }
    let block = super::heap_alloc(e, MOON_SIZE);
    let words = [
        block,
        MOON_PREFIX,
        e.mem.u32(F_MASSER_ANGLE_FADE_START),
        e.mem.u32(F_MASSER_ANGLE_FADE_END),
        e.mem.u32(F_MASSER_ANGLE_SHADOW_EARLY_FADE),
        e.mem.u32(F_MASSER_SPEED),
        e.mem.u32(F_MASSER_Z_OFFSET),
        e.mem.u32(I_MASSER_SIZE),
    ];
    let moon = e.call(MOON_CONSTRUCTOR, &words).eax;
    e.mem.set_u32(sky + SKY_MOON, moon);
    let node = e.mem.u32(sky + SKY_NODE);
    e.vcall(moon, 0x10, &[node, MOON_PREFIX]);
}

/// JIP's path buffer (`char path[0x80]`).
const PATH_BUFFER: usize = 0x80;

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_miscellaneous.h:
// Cmd_SetMoonTexture_Execute.
/// `SetMoonTexture phase "path"` (phase 0 to 7, with a sky): the phase's texture replaced (the
/// others keep what earlier calls gave, else JIP's defaults) and the moon rebuilt with them;
/// answers 1.
fn set_moon_texture(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let sky = e.mem.u32(SKY);
    let phase = read.int(0, 0) as usize;
    if !read.ok || phase > 7 || sky == 0 {
        return handled(true);
    }
    let path = super::buffer_arg(e, &read, 1, PATH_BUFFER, "SetMoonTexture");
    {
        let mut state = textures(e).state.lock().unwrap_or_else(|p| p.into_inner());
        if state.0.is_empty() {
            state.0 = vec![None; 8];
        }
        state.0[phase] = Some(path);
        let set = (0..8)
            .map(|i| {
                state.0[i]
                    .clone()
                    .or_else(|| DEFAULTS[i].map(<[u8]>::to_vec))
            })
            .collect();
        state.1 = Some(set);
    }
    refresh_moon(e, sky);
    textures(e)
        .state
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .1 = None;
    a.set_result(e, 1.0);
    handled(true)
}

#[cfg(test)]
#[path = "jip_fn_moon_tests.rs"]
mod tests;
