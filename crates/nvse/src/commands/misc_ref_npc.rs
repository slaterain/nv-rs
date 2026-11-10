//! xNVSE's `Commands_MiscRef.cpp` (xNVSE), second batch: an NPC base form's face and size (its
//! eyes, hair, hair length and colour, weight and height) and the flags of eyes and hair forms.

use super::{console_print, double_words, dynamic_cast, is_console_mode, rtti, Read};
use crate::table::{Command, Param, Returns};
use crate::{handled, CommandArgs};
use engine_core::prelude::*;

const fn p(name: &'static str, kind: u32, optional: bool) -> Param {
    Param {
        name,
        kind,
        optional,
    }
}
/// Parameter types (xNVSE `CommandTable.h`): `kParamType_Integer`, `_Float`, `_NPC`, `_AnyForm`.
const INT: u32 = 0x01;
const FLOAT: u32 = 0x02;
const NPC: u32 = 0x22;
const ANY_FORM: u32 = 0x3d;
/// `kParams_SetNPCBodyData`, `kParams_OneNPC`, `kParams_SetHairLength`, `kParams_SetHairColor`,
/// `kParams_GetHairColor`, `kParams_SetNPCWeight` (also `SetNPCHeight`'s),
/// `kParams_OneEyes_OneOptionalMask` and `kParams_OneHair_OneOptionalMask` (xNVSE).
const BODY_DATA: &[Param] = &[p("body data", ANY_FORM, false), p("base NPC", NPC, true)];
const ONE_NPC: &[Param] = &[p("NPC", NPC, true)];
const FLOAT_NPC: &[Param] = &[p("value", FLOAT, false), p("base NPC", NPC, true)];
const INT_NPC: &[Param] = &[p("value", INT, false), p("base NPC", NPC, true)];
const FORM_MASK: &[Param] = &[p("form", ANY_FORM, false), p("mask", INT, true)];

/// `TESForm::refID`; `TESObjectREFR::baseForm`.
const FORM_ID: u32 = 0xc;
const REFR_BASE_FORM: u32 = 0x20;
/// `TESNPC`: `hair` (`+0x1B8`), `hairLength` (`+0x1BC`, a `float`), `eyes` (`+0x1C0`),
/// `hairColor` (`+0x1D8`), `height` (`+0x1F4`) and `weight` (`+0x1F8`), both `float`s (the
/// game's own `TESNPC` load and save read and write them there: `engine-fallout-forms`'
/// `tesnpc.rs`). `TESEyes::eyeFlags` (a byte at `+0x30`), `TESHair::hairFlags` (a byte at `+0x48`).
const NPC_HAIR: u32 = 0x1b8;
const NPC_HAIR_LENGTH: u32 = 0x1bc;
const NPC_EYES: u32 = 0x1c0;
const NPC_HAIR_COLOR: u32 = 0x1d8;
const NPC_HEIGHT: u32 = 0x1f4;
const NPC_WEIGHT: u32 = 0x1f8;
const EYES_FLAGS: u32 = 0x30;
const HAIR_FLAGS: u32 = 0x48;

const fn cmd(
    opcode: u32,
    name: &'static str,
    help: &'static str,
    params: &'static [Param],
    execute: AbiFn,
    returns: Returns,
) -> Command {
    Command {
        opcode,
        name,
        short: "",
        help,
        needs_ref: false,
        params,
        execute,
        eval: None,
        returns,
    }
}

pub static COMMANDS: &[Command] = &[
    cmd(
        0x1521,
        "SetEyes",
        "set an NPCs eyes",
        BODY_DATA,
        set_eyes,
        Returns::Default,
    ),
    cmd(
        0x1522,
        "GetEyes",
        "get an NPCs eyes",
        ONE_NPC,
        get_eyes,
        Returns::Form,
    ),
    cmd(
        0x1523,
        "SetHair",
        "set an NPCs hair",
        BODY_DATA,
        set_hair,
        Returns::Default,
    ),
    cmd(
        0x1524,
        "GetHair",
        "set an NPCs hair",
        ONE_NPC,
        get_hair,
        Returns::Form,
    ),
    cmd(
        0x1525,
        "GetHairLength",
        "get an NPCs hair length",
        ONE_NPC,
        get_hair_length,
        Returns::Default,
    ),
    cmd(
        0x1526,
        "SetHairLength",
        "set an NPCs hair length",
        FLOAT_NPC,
        set_hair_length,
        Returns::Default,
    ),
    cmd(
        0x1527,
        "GetHairColor",
        "get an NPCs hair color (code: 1=R, 2=G, 3=B)",
        INT_NPC,
        get_hair_color,
        Returns::Default,
    ),
    cmd(
        0x1528,
        "SetHairColor",
        "set an NPCs hair color (RGB bytes)",
        INT_NPC,
        set_hair_color,
        Returns::Default,
    ),
    cmd(
        0x1529,
        "GetNPCWeight",
        "get an NPCs weight",
        ONE_NPC,
        get_npc_weight,
        Returns::Default,
    ),
    cmd(
        0x152a,
        "SetNPCWeight",
        "set an NPCs weight",
        FLOAT_NPC,
        set_npc_weight,
        Returns::Default,
    ),
    cmd(
        0x152b,
        "GetNPCHeight",
        "get an NPCs height",
        ONE_NPC,
        get_npc_height,
        Returns::Default,
    ),
    cmd(
        0x152c,
        "SetNPCHeight",
        "set an NPCs height",
        FLOAT_NPC,
        set_npc_height,
        Returns::Default,
    ),
    cmd(
        0x1600,
        "GetEyesFlags",
        "get the flags of eyes",
        FORM_MASK,
        get_eyes_flags,
        Returns::Default,
    ),
    cmd(
        0x1601,
        "SetEyesFlags",
        "set the flags of eyes",
        FORM_MASK,
        set_eyes_flags,
        Returns::Default,
    ),
    cmd(
        0x1602,
        "GetHairFlags",
        "get the flags of an hair",
        FORM_MASK,
        get_hair_flags,
        Returns::Default,
    ),
    cmd(
        0x1603,
        "SetHairFlags",
        "set the flags of an hair",
        FORM_MASK,
        set_hair_flags,
        Returns::Default,
    ),
];

// ---- the NPC a command works on ----

/// The NPC a setter changes: the given base NPC, else the calling reference's base form, as an
/// NPC (`DYNAMIC_CAST(target, TESForm, TESNPC)`); 0 for none.
fn target_npc(e: &mut Engine, a: &CommandArgs, given: u32) -> u32 {
    let mut target = given;
    if target == 0 {
        if a.this_obj == 0 {
            return 0;
        }
        target = e.mem.u32(a.this_obj + REFR_BASE_FORM);
    }
    if target == 0 {
        return 0;
    }
    dynamic_cast(e, target, rtti::TES_FORM, rtti::TES_NPC)
}

/// The NPC a getter reads: the given one, else the calling reference's base form as an NPC.
fn source_npc(e: &mut Engine, a: &CommandArgs, given: u32) -> u32 {
    if given == 0 && a.this_obj != 0 {
        let base = e.mem.u32(a.this_obj + REFR_BASE_FORM);
        return dynamic_cast(e, base, rtti::TES_FORM, rtti::TES_NPC);
    }
    given
}

// ---- eyes and hair ----

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_MiscRef.cpp:
// Cmd_SetEyes_Execute, Cmd_SetHair_Execute.
/// The NPC's eyes (or hair) set to the given form when it is eyes (hair); 1 when set.
fn set_part(e: &mut Engine, w: &[u32], class: u32, field: u32) -> Ret {
    let a = CommandArgs::new(w);
    a.set_result(e, 0.0);
    let read = Read::of(e, &a);
    if !read.ok {
        return handled(true);
    }
    let part = dynamic_cast(e, read.form(0), rtti::TES_FORM, class);
    if part == 0 {
        return handled(true);
    }
    let npc = target_npc(e, &a, read.form(1));
    if npc != 0 {
        e.mem.set_u32(npc + field, part);
        a.set_result(e, 1.0);
    }
    handled(true)
}

fn set_eyes(e: &mut Engine, w: &[u32]) -> Ret {
    set_part(e, w, rtti::TES_EYES, NPC_EYES)
}

fn set_hair(e: &mut Engine, w: &[u32]) -> Ret {
    set_part(e, w, rtti::TES_HAIR, NPC_HAIR)
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_MiscRef.cpp:
// Cmd_GetEyes_Execute, Cmd_GetHair_Execute.
/// The NPC's eyes (hair) form; in the console its id, 0 for none.
fn get_part(e: &mut Engine, w: &[u32], field: u32, text: &str) -> Ret {
    let a = CommandArgs::new(w);
    a.set_form_result(e, 0);
    let read = Read::of(e, &a);
    if !read.ok {
        return handled(true);
    }
    let npc = source_npc(e, &a, read.form(0));
    let mut id = 0;
    if npc != 0 {
        let part = e.mem.u32(npc + field);
        if part != 0 {
            id = e.mem.u32(part + FORM_ID);
            a.set_form_result(e, id);
        }
    }
    if is_console_mode(e) {
        console_print(e, text, &[id]);
    }
    handled(true)
}

fn get_eyes(e: &mut Engine, w: &[u32]) -> Ret {
    get_part(e, w, NPC_EYES, "GetEyes: %08X")
}

fn get_hair(e: &mut Engine, w: &[u32]) -> Ret {
    get_part(e, w, NPC_HAIR, "GetHair: %08X")
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_MiscRef.cpp:
// Cmd_GetEyesFlags_Execute, Cmd_GetHairFlags_Execute.
/// The flags byte of an eyes (hair) form, masked (all bits when no mask is given); in the console
/// the result, 0 for another form.
fn get_flags(e: &mut Engine, w: &[u32], class: u32, field: u32, text: &str) -> Ret {
    let a = CommandArgs::new(w);
    a.set_result(e, 0.0);
    let read = Read::of(e, &a);
    let mut value = 0u32;
    let form = read.form(0);
    if read.ok && form != 0 {
        let part = dynamic_cast(e, form, rtti::TES_FORM, class);
        if part != 0 {
            value = u32::from(e.mem.u8(part + field)) & read.int(1, 0xffff_ffff);
            a.set_result(e, f64::from(value));
        }
    }
    if is_console_mode(e) {
        console_print(e, text, &[value]);
    }
    handled(true)
}

fn get_eyes_flags(e: &mut Engine, w: &[u32]) -> Ret {
    get_flags(e, w, rtti::TES_EYES, EYES_FLAGS, "GetEyesFlags: %08X")
}

fn get_hair_flags(e: &mut Engine, w: &[u32]) -> Ret {
    get_flags(e, w, rtti::TES_HAIR, HAIR_FLAGS, "GetHairFlags: %08X")
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_MiscRef.cpp:
// Cmd_SetEyesFlags_Execute, Cmd_SetHairFlags_Execute.
/// The flags byte of an eyes (hair) form set to the "mask", below 0xFF only (none given is all
/// bits, refused).
fn set_flags(e: &mut Engine, w: &[u32], class: u32, field: u32) -> Ret {
    let a = CommandArgs::new(w);
    a.set_result(e, 0.0);
    let read = Read::of(e, &a);
    let (form, flags) = (read.form(0), read.int(1, 0xffff_ffff));
    if read.ok && form != 0 && flags < 0xff {
        let part = dynamic_cast(e, form, rtti::TES_FORM, class);
        if part != 0 {
            e.mem.set_u8(part + field, flags as u8);
        }
    }
    handled(true)
}

fn set_eyes_flags(e: &mut Engine, w: &[u32]) -> Ret {
    set_flags(e, w, rtti::TES_EYES, EYES_FLAGS)
}

fn set_hair_flags(e: &mut Engine, w: &[u32]) -> Ret {
    set_flags(e, w, rtti::TES_HAIR, HAIR_FLAGS)
}

// ---- hair length and colour, weight and height ----

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_MiscRef.cpp:
// Cmd_SetHairLength_Execute, Cmd_SetNPCWeight_Execute, Cmd_SetNPCHeight_Execute.
/// The NPC's `float` at `field` set; 1 when set.
fn set_float(e: &mut Engine, w: &[u32], field: u32) -> Ret {
    let a = CommandArgs::new(w);
    a.set_result(e, 0.0);
    let read = Read::of(e, &a);
    if !read.ok {
        return handled(true);
    }
    let value = read.float(0, 0.0);
    let npc = target_npc(e, &a, read.form(1));
    if npc != 0 {
        e.mem.set_f32(npc + field, value);
        a.set_result(e, 1.0);
    }
    handled(true)
}

fn set_hair_length(e: &mut Engine, w: &[u32]) -> Ret {
    set_float(e, w, NPC_HAIR_LENGTH)
}

fn set_npc_weight(e: &mut Engine, w: &[u32]) -> Ret {
    set_float(e, w, NPC_WEIGHT)
}

fn set_npc_height(e: &mut Engine, w: &[u32]) -> Ret {
    set_float(e, w, NPC_HEIGHT)
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_MiscRef.cpp:
// Cmd_GetHairLength_Execute, Cmd_GetNPCWeight_Execute, Cmd_GetNPCHeight_Execute.
/// The NPC's `float` at `field`; in the console the result, 0 for no NPC.
fn get_float(e: &mut Engine, w: &[u32], field: u32, text: &str) -> Ret {
    let a = CommandArgs::new(w);
    a.set_result(e, 0.0);
    let read = Read::of(e, &a);
    if !read.ok {
        return handled(true);
    }
    let npc = source_npc(e, &a, read.form(0));
    let mut value = 0.0;
    if npc != 0 {
        value = f64::from(e.mem.f32(npc + field));
        a.set_result(e, value);
    }
    if is_console_mode(e) {
        console_print(e, text, &double_words(value));
    }
    handled(true)
}

fn get_hair_length(e: &mut Engine, w: &[u32]) -> Ret {
    get_float(e, w, NPC_HAIR_LENGTH, "GetHairLength: %f")
}

fn get_npc_weight(e: &mut Engine, w: &[u32]) -> Ret {
    get_float(e, w, NPC_WEIGHT, "GetNPCweight: %f")
}

fn get_npc_height(e: &mut Engine, w: &[u32]) -> Ret {
    get_float(e, w, NPC_HEIGHT, "GetNPCheight: %f")
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_MiscRef.cpp:
// Cmd_SetHairColor_Execute.
/// The NPC's hair colour set to the low three bytes of the number; 1 when set.
fn set_hair_color(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    a.set_result(e, 0.0);
    let read = Read::of(e, &a);
    if !read.ok {
        return handled(true);
    }
    let color = read.int(0, 0);
    let npc = target_npc(e, &a, read.form(1));
    if npc != 0 {
        e.mem.set_u32(npc + NPC_HAIR_COLOR, color & 0x00ff_ffff);
        a.set_result(e, 1.0);
    }
    handled(true)
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_MiscRef.cpp:
// Cmd_GetHairColor_Execute.
/// The NPC's hair colour word, or one of its bytes: code 1 the lowest (`R`), 2 the next, 3 the
/// third. In the console the word, the code, the value and the result.
fn get_hair_color(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    a.set_result(e, 0.0);
    let read = Read::of(e, &a);
    if !read.ok {
        return handled(true);
    }
    let code = read.int(0, 0);
    let npc = source_npc(e, &a, read.form(1));
    if npc != 0 {
        let word = e.mem.u32(npc + NPC_HAIR_COLOR);
        let color = match code {
            3 => (word >> 16) & 0xff,
            2 => (word >> 8) & 0xff,
            1 => word & 0xff,
            _ => word,
        };
        let value = f64::from(color);
        a.set_result(e, value);
        if is_console_mode(e) {
            let [lo, hi] = double_words(value);
            console_print(
                e,
                "GetHairColor: (%08X) Code=%d Color=%02x [%f]",
                &[word, code, color, lo, hi],
            );
        }
    }
    handled(true)
}

#[cfg(test)]
#[path = "misc_ref_npc_tests.rs"]
mod tests;
