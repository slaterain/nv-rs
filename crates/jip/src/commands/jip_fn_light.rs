//! `functions_jip/jip_fn_light.h` (JIP LN NVSE): a light's numbers and flags. 2 of its 4
//! commands: the getters.
//!
//! Not built: `SetLightTraitNumeric` (`0x2643`) and `SetLightFlag` (`0x2645`). `SetLightFlag` and
//! `SetLightTraitNumeric`'s radius, colour and fade traits (0-3, 6) end with JIP's
//! `MarkLightModified`, which marks the lights made from the form in `s_activePtLights`, the list
//! JIP's point light hooks keep (`internal/hooks.h`; hook sites, steps 3.5/3.9); its falloff and
//! field of view traits (4, 5) only write the form. The command waits whole for that list: without
//! it a set would change the form but not the lights already in the world.

use super::forms::{form_type_more, TESObjectLIGH};
use super::{type_of, Handlers};
use engine_core::prelude::*;
use nvse::commands::Read;
use nvse::{handled, CommandArgs};

#[rustfmt::skip]
pub static COMMANDS: Handlers = &[
    (0x2642, get_light_trait_numeric),
    (0x2644, get_light_flag),
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

#[cfg(test)]
#[path = "jip_fn_light_tests.rs"]
mod tests;
