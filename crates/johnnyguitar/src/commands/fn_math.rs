//! `JG/functions/fn_math.cpp` (JohnnyGuitar NVSE 5.28): the arithmetic commands (`Clamp`, `Remap`,
//! `Lerp`, `Sign`), the colour conversions (`GetRGBColor`, `HSVtoRGB`, `RGBtoHSV`), the distance
//! between two points (`GetVector3DDistance`), the player's fields of view (`GetPlayerCamFOV`,
//! `GetPackedPlayerFOV`), and the world-to-screen projection with its camera copy
//! (`WorldToScreen`, `JGLegacyWorldToScreen`, `GetCameraTranslation`).

use super::{console_print, Handlers};
use engine_core::prelude::*;
use nvse::commands::Read;
use nvse::{handled, CommandArgs};

#[rustfmt::skip]
pub static COMMANDS: Handlers = &[
    (CLAMP, clamp, None),
    (REMAP, remap, None),
    (LERP, lerp, None),
    (SIGN, sign, None),
    (GET_RGB_COLOR, get_rgb_color, None),
    (HSV_TO_RGB, hsv_to_rgb, None),
    (RGB_TO_HSV, rgb_to_hsv, None),
    (GET_VECTOR_3D_DISTANCE, get_vector_3d_distance, None),
    (GET_PLAYER_CAM_FOV, get_player_cam_fov, None),
    (GET_PACKED_PLAYER_FOV, get_packed_player_fov, None),
    (WORLD_TO_SCREEN, world_to_screen, None),
];

// The opcodes (`JohnnyCommands.cpp`'s order from 0x3100; each pinned to its slot's name by a test).
pub(crate) const GET_VECTOR_3D_DISTANCE: u32 = 0x3145;
pub(crate) const CLAMP: u32 = 0x3155;
pub(crate) const REMAP: u32 = 0x3156;
pub(crate) const LERP: u32 = 0x3157;
pub(crate) const SIGN: u32 = 0x315a;
pub(crate) const WORLD_TO_SCREEN: u32 = 0x31a0;
pub(crate) const RGB_TO_HSV: u32 = 0x31ad;
pub(crate) const HSV_TO_RGB: u32 = 0x31ae;
pub(crate) const GET_RGB_COLOR: u32 = 0x31be;
pub(crate) const GET_PACKED_PLAYER_FOV: u32 = 0x31e9;
pub(crate) const GET_PLAYER_CAM_FOV: u32 = 0x31ec;

// Translated from JohnnyGuitar NVSE 5.28 (c87905e), JG/functions/fn_math.cpp: Cmd_Clamp_Execute.
/// The value, or the lower bound below it, or the upper bound above it (floats).
fn clamp(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    a.set_result(e, 0.0);
    let read = Read::of(e, &a);
    if read.ok {
        let (value, min, max) = (read.float(0, 0.0), read.float(1, 0.0), read.float(2, 0.0));
        let r = if value < min {
            min
        } else if value > max {
            max
        } else {
            value
        };
        a.set_result(e, f64::from(r));
    }
    handled(true)
}

// Translated from JohnnyGuitar NVSE 5.28 (c87905e), JG/functions/fn_math.cpp: Cmd_Remap_Execute.
/// `(value - minA) / (maxA - minA) * (maxB - minB) + minB`, in floats.
fn remap(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    a.set_result(e, 0.0);
    let read = Read::of(e, &a);
    if read.ok {
        let v: [f32; 5] = std::array::from_fn(|i| read.float(i, 0.0));
        let r = (v[0] - v[1]) / (v[2] - v[1]) * (v[4] - v[3]) + v[3];
        a.set_result(e, f64::from(r));
    }
    handled(true)
}

// Translated from JohnnyGuitar NVSE 5.28 (c87905e), JG/functions/fn_math.cpp: Cmd_Lerp_Execute.
/// `(1 - t) * start + t * end`, in floats.
fn lerp(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    a.set_result(e, 0.0);
    let read = Read::of(e, &a);
    if read.ok {
        let (start, end, t) = (read.float(0, 0.0), read.float(1, 0.0), read.float(2, 0.0));
        a.set_result(e, f64::from((1.0 - t) * start + t * end));
    }
    handled(true)
}

// Translated from JohnnyGuitar NVSE 5.28 (c87905e), JG/functions/fn_math.cpp: Cmd_Sign_Execute.
/// 1 for a positive value, -1 for a negative one, 0 for zero (or a NaN: neither compare holds,
/// and `fValue != 0` does, so `fValue > 0` gives -1).
fn sign(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    a.set_result(e, 0.0);
    let read = Read::of(e, &a);
    if read.ok {
        let value = read.float(0, 0.0);
        if value != 0.0 {
            a.set_result(e, if value > 0.0 { 1.0 } else { -1.0 });
        }
    }
    handled(true)
}

// Translated from JohnnyGuitar NVSE 5.28 (c87905e), JG/functions/fn_math.cpp:
// Cmd_GetRGBColor_Execute.
/// Three components of 0 to 255 packed as `0xRRGGBB` (any above 255: 0), echoed `0x%X` in the
/// console.
fn get_rgb_color(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    a.set_result(e, 0.0);
    let read = Read::of(e, &a);
    let (r, g, b) = (read.int(0, 0), read.int(1, 0), read.int(2, 0));
    if read.ok && r <= 0xff && g <= 0xff && b <= 0xff {
        let packed = (r << 16) + (g << 8) + b;
        a.set_result(e, f64::from(packed));
        console_print(e, b"0x%X", &[packed]);
    }
    handled(true)
}

/// A script variable's value (`ScriptVar::data`, the double at +8).
fn var(e: &Engine, local: u32) -> f64 {
    e.mem.f64(local + 8)
}

fn set_var(e: &mut Engine, local: u32, value: f64) {
    e.mem.set_f64(local + 8, value);
}

// Translated from JohnnyGuitar NVSE 5.28 (c87905e), JG/functions/fn_math.cpp:
// Cmd_HSVtoRGB_Execute.
/// Hue (degrees), saturation and value (0 to 1) from three variables into red, green and blue
/// (0 to 255, rounded half away from zero) in three more; the result 0.
fn hsv_to_rgb(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    a.set_result(e, 0.0);
    let read = Read::of(e, &a);
    if !read.ok {
        return handled(true);
    }
    let locals: [u32; 6] = std::array::from_fn(|i| read.local(i));
    let (h, s, v) = (var(e, locals[0]), var(e, locals[1]), var(e, locals[2]));
    let (r, g, b) = if s <= 0.0 {
        (v, v, v)
    } else {
        let mut hh = h;
        if hh >= 360.0 {
            hh = 0.0;
        }
        hh /= 60.0;
        // `(long)hh`: truncation toward zero (a negative hue gives a negative sector: the default).
        let i = hh as i32;
        let ff = hh - f64::from(i);
        let p = v * (1.0 - s);
        let q = v * (1.0 - (s * ff));
        let t = v * (1.0 - (s * (1.0 - ff)));
        match i {
            0 => (v, t, p),
            1 => (q, v, p),
            2 => (p, v, t),
            3 => (p, q, v),
            4 => (t, p, v),
            _ => (v, p, q),
        }
    };
    set_var(e, locals[3], (r * 255.0).round());
    set_var(e, locals[4], (g * 255.0).round());
    set_var(e, locals[5], (b * 255.0).round());
    handled(true)
}

// Translated from JohnnyGuitar NVSE 5.28 (c87905e), JG/functions/fn_math.cpp:
// Cmd_RGBtoHSV_Execute.
/// Red, green and blue (0 to 255) from three variables into hue (degrees), saturation and value
/// (0 to 1) in three more; the result 0.
fn rgb_to_hsv(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    a.set_result(e, 0.0);
    let read = Read::of(e, &a);
    if !read.ok {
        return handled(true);
    }
    let locals: [u32; 6] = std::array::from_fn(|i| read.local(i));
    let r = var(e, locals[0]) / 255.0;
    let g = var(e, locals[1]) / 255.0;
    let b = var(e, locals[2]) / 255.0;
    let min = if r < g { r } else { g };
    let min = if min < b { min } else { b };
    let max = if r > g { r } else { g };
    let max = if max > b { max } else { b };
    let v = max;
    let delta = max - min;
    let (h, s) = if delta < 0.00001 || max <= 0.0 {
        (0.0, 0.0)
    } else {
        let s = delta / max;
        let mut h = if r >= max {
            (g - b) / delta
        } else if g >= max {
            2.0 + (b - r) / delta
        } else {
            4.0 + (r - g) / delta
        };
        h *= 60.0;
        if h < 0.0 {
            h += 360.0;
        }
        (h, s)
    };
    set_var(e, locals[3], h);
    set_var(e, locals[4], s);
    set_var(e, locals[5], v);
    handled(true)
}

// Translated from JohnnyGuitar NVSE 5.28 (c87905e), JG/functions/fn_math.cpp:
// Cmd_GetVector3DDistance_Execute.
/// The distance between two points (`NiPoint3::Distance`: the floats' differences, their squares'
/// sum's square root, in floats), echoed `Get3DDistance >> %f` in the console.
fn get_vector_3d_distance(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    a.set_result(e, 0.0);
    let read = Read::of(e, &a);
    if read.ok {
        let p: [f32; 6] = std::array::from_fn(|i| read.float(i, 0.0));
        let distance = point_distance([p[0], p[1], p[2]], [p[3], p[4], p[5]]);
        a.set_result(e, f64::from(distance));
        console_print(e, b"Get3DDistance >> %f", &args![f64::from(distance)]);
    }
    handled(true)
}

/// `NiPoint3::Distance` (JohnnyGuitar's `Gamebryo/NiPoint3.hpp`): to check against its header.
fn point_distance(a: [f32; 3], b: [f32; 3]) -> f32 {
    let (dx, dy, dz) = (a[0] - b[0], a[1] - b[1], a[2] - b[2]);
    (dx * dx + dy * dy + dz * dz).sqrt()
}

// ---- the fields of view ----

/// The player (`*011DEA3C`); `PlayerCharacter::Get1stPersonFOV` (`00874900`, +0x674) and
/// `GetWorldFOV` (`00710AB0`, +0x670), both `thiscall`; `TESMain::GetWorldSceneGraph`
/// (`0045C670`) and `BSSceneGraph::GetCameraFOV` (`0099E040`, `thiscall`, +0xBC).
const PLAYER: u32 = 0x011d_ea3c;
const FIRST_PERSON_FOV: u32 = 0x0087_4900;
const WORLD_FOV: u32 = 0x0071_0ab0;
const WORLD_SCENE_GRAPH: u32 = 0x0045_c670;
const CAMERA_FOV: u32 = 0x0099_e040;

fn first_person_fov(e: &mut Engine, player: u32) -> f64 {
    f64::from(e.call(FIRST_PERSON_FOV, &args![player]).f32())
}

fn world_fov(e: &mut Engine, player: u32) -> f64 {
    f64::from(e.call(WORLD_FOV, &args![player]).f32())
}

fn current_fov(e: &mut Engine) -> f64 {
    let graph = e.call(WORLD_SCENE_GRAPH, &args![]).u32();
    f64::from(e.call(CAMERA_FOV, &args![graph]).f32())
}

// Translated from JohnnyGuitar NVSE 5.28 (c87905e), JG/functions/fn_math.cpp:
// Cmd_GetPlayerCamFOV_Eval.
/// 0: the player's first-person field of view; 1: the world one; any other: the world scene
/// graph's camera's (JG reads the player and the scene graph without a null check).
fn player_cam_fov(e: &mut Engine, kind: u32, result: u32) {
    let player = e.mem.u32(PLAYER);
    let v = match kind {
        0 => first_person_fov(e, player),
        1 => world_fov(e, player),
        _ => current_fov(e),
    };
    e.mem.set_f64(result, v);
}

// Translated from JohnnyGuitar NVSE 5.28 (c87905e), JG/functions/fn_math.cpp:
// Cmd_GetPlayerCamFOV_Execute.
/// The eval with the integer read (0, the first-person one, when not read: the read's answer is
/// not looked at).
fn get_player_cam_fov(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    e.mem.set_f64(a.result, 0.0);
    player_cam_fov(e, read.int(0, 0), a.result);
    handled(true)
}

// Translated from JohnnyGuitar NVSE 5.28 (c87905e), JG/functions/fn_math.cpp:
// Cmd_GetPackedPlayerFOV_Execute.
/// The first-person and world fields of view into two variables, and the camera's into a third
/// when given; the result 1. A failed read or no player: 0, nothing written.
fn get_packed_player_fov(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    a.set_result(e, 0.0);
    let read = Read::of(e, &a);
    let player = e.mem.u32(PLAYER);
    if !read.ok || player == 0 {
        return handled(true);
    }
    let (first, world, current) = (read.local(0), read.local(1), read.local(2));
    let v = first_person_fov(e, player);
    set_var(e, first, v);
    let v = world_fov(e, player);
    set_var(e, world, v);
    if current != 0 {
        let v = current_fov(e);
        set_var(e, current, v);
    }
    a.set_result(e, 1.0);
    handled(true)
}

// ---- the projection ----

/// `TESObjectREFR::GetPosition`: the reference's position (+0x30, three floats).
const REF_POSITION: u32 = 0x30;

// Translated from JohnnyGuitar NVSE 5.28 (c87905e), JG/functions/fn_math.cpp:
// Cmd_WorldToScreen_Execute.
/// The point (plus the reference's position when one is given) projected with the camera copy
/// ([`crate::world_to_screen`]): 1 when on screen, else 0; the projected x, y, z into the three
/// variables either way. A failed read writes nothing.
fn world_to_screen(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    a.set_result(e, 0.0);
    let read = Read::of(e, &a);
    if !read.ok {
        return handled(true);
    }
    let outs = [read.local(0), read.local(1), read.local(2)];
    let mut p = [read.float(3, 0.0), read.float(4, 0.0), read.float(5, 0.0)];
    let handling = read.int(6, 0) as i32;
    let reference = read.form(7);
    if reference != 0 {
        for (i, v) in p.iter_mut().enumerate() {
            *v += e.mem.f32(reference + REF_POSITION + 4 * i as u32);
        }
    }
    let camera = crate::world_to_screen::camera(e);
    let (on, out) = crate::world_to_screen::project(
        &camera,
        p,
        handling,
        crate::world_to_screen::ZERO_TOLERANCE,
    );
    a.set_result(e, if on { 1.0 } else { 0.0 });
    for (local, v) in outs.into_iter().zip(out) {
        set_var(e, local, f64::from(v));
    }
    handled(true)
}

#[cfg(test)]
#[path = "fn_math_tests.rs"]
mod tests;
