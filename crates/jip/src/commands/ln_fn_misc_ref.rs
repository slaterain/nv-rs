//! `functions_ln/ln_fn_misc_ref.h` (JIP LN NVSE): a reference's activate-parent flags, base form,
//! inventory weight, form flags (also as a condition), action, and a light's radius. All 8 of its
//! commands.
//!
//! On references: the form flags at +8, the base form at +0x20, the extra data list at +0x44,
//! the 3D redone by the core track's `Update3D` ([`crate::overrides::ref_update_3d`]). Extras: `ExtraActivateRef`
//! (type 0x53: the parents list at +0xC, the flags at +0x14), `ExtraContainerChanges` (0x15: its
//! data at +0xC), `ExtraAction` (0x0E), `ExtraRadius` (0x5C: a float at +0xC, made by JIP's
//! `Create` with the exe's vtable `01015208`). A light's radius is the word at +0xA0 (forms.rs
//! `TESObjectLIGH`).

use super::extra::{self, kind, vtbl, REFR_EXTRA_LIST};
use super::forms::TESObjectLIGH;
use super::{cvttss2si, type_of, Handlers};
use engine_core::prelude::*;
use nvse::commands::Read;
use nvse::{handled, CommandArgs, EvalArgs};

#[rustfmt::skip]
pub static COMMANDS: Handlers = &[
    (0x2280, is_parent_activate_only),
    (0x2282, set_base_form),
    (0x22c1, get_inventory_weight),
    (0x22d7, set_reference_flag),
    (0x2318, has_action_ref),
    (0x2334, get_radius),
    (0x2335, set_radius),
];

#[rustfmt::skip]
pub static CONDITIONS: super::Conditions = &[
    (0x22d6, get_reference_flag, get_reference_flag_eval),
];

const FLAGS: u32 = 0x08;
const BASE: u32 = 0x20;
const EXTRA_WORD: u32 = 0x0c;
const ACTIVATE_FLAGS: u32 = 0x14;
/// `kFormType_TESObjectLIGH`.
const TES_OBJECT_LIGH: u8 = 0x1e;
/// `g_thePlayer` (`*011DEA3C`); its byte at +0x7BC the weight call takes; the game's flag byte
/// `011E0898` JIP sets around the call; `ExtraContainerChanges::Data::GetInventoryWeight`'s
/// game function `004D0900` (`thiscall`: the data, the byte; the weight in `ST0`).
const PLAYER: u32 = 0x011d_ea3c;
const PLAYER_BYTE_7BC: u32 = 0x7bc;
const WEIGHT_FLAG: u32 = 0x011e_0898;
const INVENTORY_WEIGHT: u32 = 0x004d_0900;

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_misc_ref.h:
// Cmd_IsParentActivateOnly_Execute.
/// The flags of the reference's (else the caller's) `ExtraActivateRef`.
fn is_parent_activate_only(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if read.ok {
        let r = match read.form(0) {
            0 => a.this_obj,
            r => r,
        };
        if r != 0 {
            let x = extra::refr_extra(e, r, kind::ACTIVATE_REF);
            if x != 0 {
                let f = e.mem.u32(x + ACTIVATE_FLAGS) as i32;
                a.set_result(e, f64::from(f));
            }
        }
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_misc_ref.h:
// Cmd_SetBaseForm_Execute.
/// Not inside an inventory (`containingObj`): the base form replaced, the 3D redone.
fn set_base_form(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    if a.containing_obj != 0 {
        return handled(true);
    }
    let read = Read::of(e, &a);
    if read.ok {
        let form = read.form(0);
        e.mem.set_u32(a.this_obj + BASE, form);
        crate::overrides::ref_update_3d(e, a.this_obj);
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_misc_ref.h:
// Cmd_GetInventoryWeight_Execute; nvse/GameExtraData.cpp:
// ExtraContainerChanges::Data::GetInventoryWeight (asm).
/// The weight the game's function gives for the reference's container changes data (its cached
/// weight +8 set to -1 first, the game's flag byte set around the call), echoed in the console.
fn get_inventory_weight(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let x = extra::refr_extra(e, a.this_obj, kind::CONTAINER_CHANGES);
    if x != 0 {
        let data = e.mem.u32(x + EXTRA_WORD);
        if data != 0 {
            e.mem.set_u32(data + 8, 0xbf80_0000);
            let byte = e.mem.u8(e.mem.u32(PLAYER) + PLAYER_BYTE_7BC);
            e.mem.set_u8(WEIGHT_FLAG, 1);
            let weight = e.call(INVENTORY_WEIGHT, &args![data, u32::from(byte)]).st0;
            e.mem.set_u8(WEIGHT_FLAG, 0);
            a.set_result(e, weight);
        }
    }
    crate::console::print_result(e, &a);
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_misc_ref.h:
// Cmd_GetReferenceFlag_Execute.
/// 1 when any of the bits is set in the reference's form flags; echoed in the console.
fn get_reference_flag(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if read.ok && e.mem.u32(a.this_obj + FLAGS) & read.int(0, 0) != 0 {
        a.set_result(e, 1.0);
    }
    crate::console::print_result(e, &a);
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_misc_ref.h:
// Cmd_GetReferenceFlag_Eval.
fn get_reference_flag_eval(e: &mut Engine, w: &[u32]) -> Ret {
    let a = EvalArgs::new(w);
    if e.mem.u32(a.this_obj + FLAGS) & a.arg1 != 0 {
        e.mem.set_f64(a.result, 1.0);
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_misc_ref.h:
// Cmd_SetReferenceFlag_Execute.
/// The bits set or cleared in the reference's form flags (nothing marked changed: JIP's call is
/// commented out).
fn set_reference_flag(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if read.ok {
        let (flag, on) = (read.int(0, 0), read.int(1, 0));
        let f = e.mem.u32(a.this_obj + FLAGS);
        e.mem.set_u32(
            a.this_obj + FLAGS,
            if on != 0 { f | flag } else { f & !flag },
        );
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_misc_ref.h:
// Cmd_HasActionRef_Execute.
fn has_action_ref(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    if extra::has_type(e, a.this_obj + REFR_EXTRA_LIST, kind::ACTION) {
        a.set_result(e, 1.0);
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_misc_ref.h:
// Cmd_GetRadius_Execute.
/// With a light: its radius (a whole number); without: the calling reference's `ExtraRadius`.
/// Another form type: nothing.
fn get_radius(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if !read.ok {
        return handled(true);
    }
    let light = read.form(0);
    if light == 0 {
        if a.this_obj != 0 {
            let x = extra::refr_extra(e, a.this_obj, kind::RADIUS);
            if x != 0 {
                let v = e.mem.f32(x + EXTRA_WORD);
                a.set_result(e, f64::from(v));
            }
        }
    } else if type_of(e, light) == TES_OBJECT_LIGH {
        let r = e.get(Ptr::<TESObjectLIGH>::new(light), TESObjectLIGH::radius) as i32;
        a.set_result(e, f64::from(r));
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_misc_ref.h:
// Cmd_SetRadius_Execute.
/// With a light: its radius (`(int)value`); without: the calling reference's `ExtraRadius`, made
/// when missing.
fn set_radius(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if !read.ok {
        return handled(true);
    }
    let value = read.float(0, 0.0);
    let light = read.form(1);
    if light == 0 {
        if a.this_obj != 0 {
            let x = extra::refr_extra(e, a.this_obj, kind::RADIUS);
            if x != 0 {
                e.mem.set_f32(x + EXTRA_WORD, value);
            } else {
                let made = extra::create(e, vtbl::RADIUS, kind::RADIUS);
                e.mem.set_f32(made + EXTRA_WORD, value);
                extra::add_extra(e, a.this_obj + REFR_EXTRA_LIST, made);
            }
        }
    } else if type_of(e, light) == TES_OBJECT_LIGH {
        let r = cvttss2si(value) as u32;
        e.set(Ptr::<TESObjectLIGH>::new(light), TESObjectLIGH::radius, r);
    }
    handled(true)
}

#[cfg(test)]
#[path = "ln_fn_misc_ref_tests.rs"]
mod tests;
