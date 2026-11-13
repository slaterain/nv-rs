//! `functions_jip/jip_fn_class.h` (JIP LN NVSE): a class's tag skills, attributes, flags and
//! services. All 4 of its commands. The class argument is read as `kParamType_Class`, whose type
//! check xNVSE's reader makes. No float is narrowed: the values are the script's integers
//! (`int val` stored into a `UInt32` tag skill or a `UInt8` attribute).

use super::forms::TESClass;
use super::Handlers;
use engine_core::prelude::*;
use nvse::commands::Read;
use nvse::{handled, CommandArgs};

#[rustfmt::skip]
pub static COMMANDS: Handlers = &[
    (0x263e, get_class_trait_numeric),
    (0x263f, set_class_trait_numeric),
    (0x2640, get_class_flag),
    (0x2641, set_class_flag),
];

fn class_arg(read: &Read) -> Option<Ptr<TESClass>> {
    read.ok.then(|| Ptr::new(read.form(0)))
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_class.h:
// Cmd_GetClassTraitNumeric_Execute.
/// Trait 0-3: a tag skill (signed); 4-10: an attribute (strength to luck). Above 10, nothing.
fn get_class_trait_numeric(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let t = read.int(1, 0);
    if let Some(c) = class_arg(&read).filter(|_| t <= 10) {
        let value = if t < 4 {
            f64::from(e.get(c.index(t, 4), TESClass::tagSkills) as i32)
        } else {
            f64::from(e.get(c.index(t - 4, 1), TESClass::attributes))
        };
        a.set_result(e, value);
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_class.h:
// Cmd_SetClassTraitNumeric_Execute.
/// A tag skill set to -1 (none) or an actor value from 32 to 45 but 33; an attribute to 1-10.
fn set_class_trait_numeric(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let t = read.int(1, 0);
    let val = read.int(2, 0) as i32;
    if let Some(c) = class_arg(&read).filter(|_| t <= 10) {
        if t < 4 {
            if val == -1 || (val > 31 && val < 46 && val != 33) {
                e.set(c.index(t, 4), TESClass::tagSkills, val as u32);
            }
        } else if val > 0 && val < 11 {
            e.set(c.index(t - 4, 1), TESClass::attributes, val as u8);
        }
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_class.h:
// Cmd_GetClassFlag_Execute.
/// Flags 0-1: the class flags' bits; 2-19: the services' bits 0-17.
fn get_class_flag(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let flag = read.int(1, 0);
    if let Some(c) = class_arg(&read).filter(|_| flag <= 19) {
        let set = if flag < 2 {
            e.get(c, TESClass::classFlags) & (1 << flag) != 0
        } else {
            e.get(c, TESClass::services) & (1 << (flag - 2)) != 0
        };
        if set {
            a.set_result(e, 1.0);
        }
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_class.h:
// Cmd_SetClassFlag_Execute.
fn set_class_flag(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let flag = read.int(1, 0);
    let on = read.int(2, 0) != 0;
    if let Some(c) = class_arg(&read).filter(|_| flag <= 19) {
        let (field, bit) = if flag < 2 {
            (TESClass::classFlags, 1u32 << flag)
        } else {
            (TESClass::services, 1u32 << (flag - 2))
        };
        let v = e.get(c, field);
        e.set(c, field, if on { v | bit } else { v & !bit });
    }
    handled(true)
}

#[cfg(test)]
#[path = "jip_fn_class_tests.rs"]
mod tests;
