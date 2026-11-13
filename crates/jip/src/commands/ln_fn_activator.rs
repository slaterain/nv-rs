//! `functions_ln/ln_fn_activator.h` (JIP LN NVSE): an activator's radio station, prompt, water,
//! looping and activation sounds. All 10 of its commands. The activator is an optional form
//! argument; without one, the calling reference's base form. Each command then checks `IS_ID`
//! (activator 0x15; the water commands also take a placeable water, 0x23).
//!
//! The activator loader (`00510A90`) stores `SNAM` through `004FEBB0` (+0x74), `VNAM` through
//! `00442A80` (+0x78), `WNAM` through `008AC890` (+0x80), `RNAM` through `004FB090` (+0x84) and
//! `XATO` through `00510EA0` (the prompt, a game string at +0x88, JIP's header); a placeable
//! water's water is at +0x4C (JIP's header).

use super::forms::{form_type_ln, form_type_more, BGSPlaceableWater, TESObjectACTI};
use super::{
    assign_string, buffer_arg, set_form_result, string_set, string_text, type_of, Handlers,
};
use crate::game::TESObjectREFR;
use engine_core::prelude::*;
use nvse::commands::Read;
use nvse::{handled, CommandArgs};

#[rustfmt::skip]
pub static COMMANDS: Handlers = &[
    (0x2273, get_activator_radio_station),
    (0x2274, set_activator_radio_station),
    (0x2275, get_activator_prompt),
    (0x2276, set_activator_prompt),
    (0x2277, get_activator_water_type),
    (0x2278, set_activator_water_type),
    (0x2290, get_activator_sound_looping),
    (0x2291, set_activator_sound_looping),
    (0x2292, get_activator_sound_activate),
    (0x2293, set_activator_sound_activate),
];

const PROMPT_BUFFER: usize = 0x80;

/// The form argument `i`, or without one the calling reference's base form (0 with neither).
fn form_or_base(e: &Engine, read: &Read, i: usize, this_obj: u32) -> u32 {
    let form = read.form(i);
    if form != 0 {
        form
    } else if this_obj != 0 {
        e.get(
            Ptr::<TESObjectREFR>::new(this_obj),
            TESObjectREFR::pObjectReference,
        )
    } else {
        0
    }
}

fn activator(e: &Engine, form: u32) -> Option<Ptr<TESObjectACTI>> {
    (form != 0 && type_of(e, form) == form_type_ln::TES_OBJECT_ACTI).then(|| Ptr::new(form))
}

/// A getter of a form field of the activator (argument 0 or the caller's base form).
fn get_form(e: &mut Engine, w: &[u32], field: Field<TESObjectACTI, u32>) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if read.ok {
        let form = form_or_base(e, &read, 0, a.this_obj);
        if let Some(act) = activator(e, form) {
            let value = e.get(act, field);
            if value != 0 {
                set_form_result(e, &a, value);
            }
        }
    }
    handled(true)
}

/// A setter of a form field: argument 0 must be of `kind`; the activator is argument 1 or the
/// caller's base form.
fn set_form(e: &mut Engine, w: &[u32], field: Field<TESObjectACTI, u32>, kind: u8) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let value = read.form(0);
    if read.ok && type_of(e, value) == kind {
        let form = form_or_base(e, &read, 1, a.this_obj);
        if let Some(act) = activator(e, form) {
            e.set(act, field, value);
        }
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_activator.h:
// Cmd_GetActivatorRadioStation_Execute.
fn get_activator_radio_station(e: &mut Engine, w: &[u32]) -> Ret {
    get_form(e, w, TESObjectACTI::radioStation)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_activator.h:
// Cmd_SetActivatorRadioStation_Execute.
fn set_activator_radio_station(e: &mut Engine, w: &[u32]) -> Ret {
    set_form(
        e,
        w,
        TESObjectACTI::radioStation,
        form_type_ln::BGS_TALKING_ACTIVATOR,
    )
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_activator.h:
// Cmd_GetActivatorPrompt_Execute.
fn get_activator_prompt(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let mut text = None;
    if read.ok {
        let form = form_or_base(e, &read, 0, a.this_obj);
        if let Some(act) = activator(e, form) {
            text = string_text(e, act.at(TESObjectACTI::activationPrompt).addr());
        }
    }
    assign_string(e, &a, text);
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_activator.h:
// Cmd_SetActivatorPrompt_Execute.
fn set_activator_prompt(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if !read.ok {
        return handled(true);
    }
    let prompt = buffer_arg(e, &read, 0, PROMPT_BUFFER, "SetActivatorPrompt");
    let form = form_or_base(e, &read, 1, a.this_obj);
    if let Some(act) = activator(e, form) {
        string_set(e, act.at(TESObjectACTI::activationPrompt).addr(), &prompt);
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_activator.h:
// Cmd_GetActivatorWaterType_Execute.
/// An activator's water, or a placeable water's.
fn get_activator_water_type(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if !read.ok {
        return handled(true);
    }
    let form = form_or_base(e, &read, 0, a.this_obj);
    if form == 0 {
        return handled(true);
    }
    let water = match type_of(e, form) {
        form_type_ln::TES_OBJECT_ACTI => {
            e.get(Ptr::<TESObjectACTI>::new(form), TESObjectACTI::waterType)
        }
        form_type_ln::BGS_PLACEABLE_WATER => e.get(
            Ptr::<BGSPlaceableWater>::new(form),
            BGSPlaceableWater::water,
        ),
        _ => 0,
    };
    if water != 0 {
        set_form_result(e, &a, water);
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_activator.h:
// Cmd_SetActivatorWaterType_Execute.
fn set_activator_water_type(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let water = read.form(0);
    if !read.ok || type_of(e, water) != form_type_more::TES_WATER_FORM {
        return handled(true);
    }
    let form = form_or_base(e, &read, 1, a.this_obj);
    if form == 0 {
        return handled(true);
    }
    match type_of(e, form) {
        form_type_ln::TES_OBJECT_ACTI => e.set(
            Ptr::<TESObjectACTI>::new(form),
            TESObjectACTI::waterType,
            water,
        ),
        form_type_ln::BGS_PLACEABLE_WATER => e.set(
            Ptr::<BGSPlaceableWater>::new(form),
            BGSPlaceableWater::water,
            water,
        ),
        _ => {}
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_activator.h:
// Cmd_GetActivatorSoundLooping_Execute.
fn get_activator_sound_looping(e: &mut Engine, w: &[u32]) -> Ret {
    get_form(e, w, TESObjectACTI::loopingSound)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_activator.h:
// Cmd_SetActivatorSoundLooping_Execute.
fn set_activator_sound_looping(e: &mut Engine, w: &[u32]) -> Ret {
    set_form(e, w, TESObjectACTI::loopingSound, form_type_more::TES_SOUND)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_activator.h:
// Cmd_GetActivatorSoundActivate_Execute.
fn get_activator_sound_activate(e: &mut Engine, w: &[u32]) -> Ret {
    get_form(e, w, TESObjectACTI::activationSound)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_activator.h:
// Cmd_SetActivatorSoundActivate_Execute.
fn set_activator_sound_activate(e: &mut Engine, w: &[u32]) -> Ret {
    set_form(
        e,
        w,
        TESObjectACTI::activationSound,
        form_type_more::TES_SOUND,
    )
}

#[cfg(test)]
#[path = "ln_fn_activator_tests.rs"]
mod tests;
