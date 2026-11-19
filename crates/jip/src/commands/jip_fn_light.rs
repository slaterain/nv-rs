//! `functions_jip/jip_fn_light.h` (JIP LN NVSE): a light's numbers and flags, all 4 of its
//! commands.
//!
//! The setters end with JIP's `MarkLightModified` ([`gl_lights::mark_modified`]): every point
//! light of JIP's own list (`s_activePtLights`, group GL part 2) made from the form is marked to be
//! set from it again, which JIP's animated-lights step (H12, in with JIP installed) does at its next
//! pass, so the lights already in the world follow the form at the next frame. `SetLightTraitNumeric`'s
//! falloff exponent and field of view (4, 5) return before the mark, as in JIP: they change the form
//! only (new lights).

use super::forms::{form_type_more, TESObjectLIGH};
use super::{cvttss2si, type_of, Handlers};
use crate::game_patches::gl::lights as gl_lights;
use engine_core::prelude::*;
use nvse::commands::Read;
use nvse::{handled, CommandArgs};

#[rustfmt::skip]
pub static COMMANDS: Handlers = &[
    (0x2642, get_light_trait_numeric),
    (0x2643, set_light_trait_numeric),
    (0x2644, get_light_flag),
    (0x2645, set_light_flag),
];

fn light_arg(e: &Engine, read: &Read) -> Option<Ptr<TESObjectLIGH>> {
    let form = read.form(0);
    (read.ok && type_of(e, form) == form_type_more::TES_OBJECT_LIGH).then(|| Ptr::new(form))
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_light.h:
// Cmd_GetLightTraitNumeric_Execute.
/// Trait 0: radius (signed); 1-3: red, green, blue; 4: falloff exponent; 5: field of view;
/// 6: fade.
fn get_light_trait_numeric(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if let Some(l) = light_arg(e, &read) {
        let value = match read.int(1, 0) {
            0 => Some(f64::from(e.get(l, TESObjectLIGH::radius) as i32)),
            1 => Some(f64::from(e.get(l, TESObjectLIGH::red))),
            2 => Some(f64::from(e.get(l, TESObjectLIGH::green))),
            3 => Some(f64::from(e.get(l, TESObjectLIGH::blue))),
            4 => Some(f64::from(e.get(l, TESObjectLIGH::falloffExp))),
            5 => Some(f64::from(e.get(l, TESObjectLIGH::fov))),
            6 => Some(f64::from(e.get(l, TESObjectLIGH::fadeValue))),
            _ => None,
        };
        if let Some(v) = value {
            a.set_result(e, v);
        }
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_light.h:
// Cmd_SetLightTraitNumeric_Execute.
/// A light form, a trait and a value: 0 the radius and 1-3 the red, green, blue byte from
/// `(int)value` (truncated toward zero; the colour its low byte), 6 the fade, each followed by JIP's
/// `MarkLightModified`; 4 the falloff exponent and 5 the field of view written without the mark
/// (JIP returns first); any other trait changes nothing.
fn set_light_trait_numeric(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let Some(l) = light_arg(e, &read) else {
        return handled(true);
    };
    let value = read.float(2, 0.0);
    let int = cvttss2si(value) as u32;
    match read.int(1, 0) {
        0 => e.set(l, TESObjectLIGH::radius, int),
        1 => e.set(l, TESObjectLIGH::red, int as u8),
        2 => e.set(l, TESObjectLIGH::green, int as u8),
        3 => e.set(l, TESObjectLIGH::blue, int as u8),
        4 => {
            e.set(l, TESObjectLIGH::falloffExp, value);
            return handled(true);
        }
        5 => {
            e.set(l, TESObjectLIGH::fov, value);
            return handled(true);
        }
        6 => e.set(l, TESObjectLIGH::fadeValue, value),
        _ => return handled(true),
    }
    gl_lights::mark_modified(e, l.addr());
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_light.h:
// Cmd_GetLightFlag_Execute.
/// 1 when flag bit `0` to `31` is set.
fn get_light_flag(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let flag = read.int(1, 0);
    if let Some(l) = light_arg(e, &read).filter(|_| flag <= 31) {
        if e.get(l, TESObjectLIGH::lightFlags) & (1 << flag) != 0 {
            a.set_result(e, 1.0);
        }
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_light.h:
// Cmd_SetLightFlag_Execute.
/// A light form, a flag bit `0` to `31` and a value: the bit set (any value but 0) or cleared,
/// then JIP's `MarkLightModified`. Bit 32 and up: nothing.
fn set_light_flag(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let flag = read.int(1, 0);
    let Some(l) = light_arg(e, &read).filter(|_| flag <= 31) else {
        return handled(true);
    };
    let bit = 1u32 << flag;
    let flags = e.get(l, TESObjectLIGH::lightFlags);
    let flags = if read.int(2, 0) != 0 {
        flags | bit
    } else {
        flags & !bit
    };
    e.set(l, TESObjectLIGH::lightFlags, flags);
    gl_lights::mark_modified(e, l.addr());
    handled(true)
}

#[cfg(test)]
#[path = "jip_fn_light_tests.rs"]
mod tests;
