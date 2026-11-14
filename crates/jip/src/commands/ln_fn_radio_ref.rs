//! `functions_ln/ln_fn_radio_ref.h` (JIP LN NVSE): a radio reference's broadcast type, radius,
//! static and position reference. All 9 of its commands, on the calling reference.
//!
//! `ExtraRadioData` (type 0x68, JIP's header): radius (a float) +0xC, range type +0x10, static
//! percentage (a float) +0x14, position reference +0x18. The radius, static and position
//! reference only count for range type 0 (radius); the getters and setters leave other types
//! alone.

use super::extra::{self, kind, REFR_EXTRA_LIST};
use super::{set_form_result, Handlers};
use engine_core::prelude::*;
use nvse::commands::Read;
use nvse::{handled, CommandArgs};

#[rustfmt::skip]
pub static COMMANDS: Handlers = &[
    (0x2245, is_radio_ref),
    (0x223d, get_radio_broadcast_type),
    (0x223e, set_radio_broadcast_type),
    (0x223f, get_radio_radius),
    (0x2240, set_radio_radius),
    (0x2241, get_radio_static),
    (0x2242, set_radio_static),
    (0x2243, get_radio_pos_ref),
    (0x2244, set_radio_pos_ref),
];

const RADIUS: u32 = 0x0c;
const RANGE_TYPE: u32 = 0x10;
const STATIC_PERC: u32 = 0x14;
const POSITION_REF: u32 = 0x18;

fn radio(e: &mut Engine, refr: u32) -> u32 {
    extra::refr_extra(e, refr, kind::RADIO_DATA)
}

/// The radio data when its range type is 0 (a radius), else 0.
fn radius_radio(e: &mut Engine, refr: u32) -> u32 {
    match radio(e, refr) {
        0 => 0,
        x if e.mem.u32(x + RANGE_TYPE) == 0 => x,
        _ => 0,
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_radio_ref.h:
// Cmd_IsRadioRef_Execute.
/// 1 when the reference's list has the radio data's presence bit.
fn is_radio_ref(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    if extra::has_type(e, a.this_obj + REFR_EXTRA_LIST, kind::RADIO_DATA) {
        a.set_result(e, 1.0);
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_radio_ref.h:
// Cmd_GetRadioBroadcastType_Execute.
fn get_radio_broadcast_type(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let x = radio(e, a.this_obj);
    if x != 0 {
        let t = e.mem.u32(x + RANGE_TYPE) as i32;
        a.set_result(e, f64::from(t));
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_radio_ref.h:
// Cmd_SetRadioBroadcastType_Execute.
/// Types 0 to 4; any but 0 also zeroes the radius and the static.
fn set_radio_broadcast_type(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let t = read.int(0, 0);
    if read.ok && t <= 4 {
        let x = radio(e, a.this_obj);
        if x != 0 {
            e.mem.set_u32(x + RANGE_TYPE, t);
            if t != 0 {
                e.mem.set_f32(x + RADIUS, 0.0);
                e.mem.set_f32(x + STATIC_PERC, 0.0);
            }
        }
    }
    handled(true)
}

/// A float field of a radius-type radio data as the result.
fn get_float(e: &mut Engine, w: &[u32], field: u32) -> Ret {
    let a = CommandArgs::new(w);
    let x = radius_radio(e, a.this_obj);
    if x != 0 {
        let v = e.mem.f32(x + field);
        a.set_result(e, f64::from(v));
    }
    handled(true)
}

/// A float field of a radius-type radio data set, for a value of 0 or more (not NaN).
fn set_float(e: &mut Engine, w: &[u32], field: u32) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let v = read.float(0, 0.0);
    if read.ok && v >= 0.0 {
        let x = radius_radio(e, a.this_obj);
        if x != 0 {
            e.mem.set_f32(x + field, v);
        }
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_radio_ref.h:
// Cmd_GetRadioRadius_Execute.
fn get_radio_radius(e: &mut Engine, w: &[u32]) -> Ret {
    get_float(e, w, RADIUS)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_radio_ref.h:
// Cmd_SetRadioRadius_Execute.
fn set_radio_radius(e: &mut Engine, w: &[u32]) -> Ret {
    set_float(e, w, RADIUS)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_radio_ref.h:
// Cmd_GetRadioStatic_Execute.
fn get_radio_static(e: &mut Engine, w: &[u32]) -> Ret {
    get_float(e, w, STATIC_PERC)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_radio_ref.h:
// Cmd_SetRadioStatic_Execute.
fn set_radio_static(e: &mut Engine, w: &[u32]) -> Ret {
    set_float(e, w, STATIC_PERC)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_radio_ref.h:
// Cmd_GetRadioPosRef_Execute.
fn get_radio_pos_ref(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let x = radius_radio(e, a.this_obj);
    if x != 0 {
        let r = e.mem.u32(x + POSITION_REF);
        if r != 0 {
            set_form_result(e, &a, r);
        }
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_radio_ref.h:
// Cmd_SetRadioPosRef_Execute.
fn set_radio_pos_ref(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if read.ok {
        let r = read.form(0);
        let x = radius_radio(e, a.this_obj);
        if x != 0 {
            e.mem.set_u32(x + POSITION_REF, r);
        }
    }
    handled(true)
}

#[cfg(test)]
#[path = "ln_fn_radio_ref_tests.rs"]
mod tests;
