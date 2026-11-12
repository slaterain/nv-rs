//! xNVSE's `Commands_Script.cpp` (xNVSE): checks of the forms a script holds (`IsFormValid`,
//! `IsReference`).
//!
//! Read from xNVSE's source (xNVSE 6.4.9, `0ccd23a`).

use super::{base_form, console_print, is_console_mode, try_get_refr_parent, Read};
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
/// `kParamType_ObjectRef` and `kParamType_AnyForm` (xNVSE `CommandTable.h`).
const OBJECT_REF: u32 = 0x04;
const ANY_FORM: u32 = 0x3d;
/// `kParams_OneOptionalForm` and `kParams_OneReference` (xNVSE `ParamInfos.h`).
const ONE_OPTIONAL_FORM: &[Param] = &[p("form", ANY_FORM, true)];
const ONE_REFERENCE: &[Param] = &[p("reference", OBJECT_REF, false)];

const fn cmd(
    opcode: u32,
    name: &'static str,
    help: &'static str,
    params: &'static [Param],
    execute: AbiFn,
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
        returns: Returns::Default,
    }
}

#[rustfmt::skip]
pub static COMMANDS: &[Command] = &[
    cmd(0x14ba, "IsFormValid", "returns 1 if the reference or passed object is valid.", ONE_OPTIONAL_FORM, is_form_valid),
    cmd(0x14bb, "IsReference", "returns 1 if the passed refVar is a reference.", ONE_REFERENCE, is_reference),
];

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_Script.cpp:
// Cmd_IsFormValid_Execute.
/// 1 when the form given (a reference's base form for a reference), or else the calling
/// reference's base form, is there; the console says which. Without either, 0 and no line.
fn is_form_valid(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    a.set_result(e, 0.0);
    let read = Read::of(e, &a);
    if !read.ok {
        return handled(true);
    }
    let mut form = try_get_refr_parent(e, read.form(0));
    if form == 0 {
        if a.this_obj == 0 {
            return handled(true);
        }
        form = base_form(e, a.this_obj);
    }
    let valid = form != 0;
    if valid {
        a.set_result(e, 1.0);
    }
    if is_console_mode(e) {
        let line = if valid { "Valid Form!" } else { "Invalid Form" };
        console_print(e, line, &[]);
    }
    handled(true)
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_Script.cpp:
// Cmd_IsReference_Execute.
/// 1 when the argument reads as a reference (the reader refuses anything else); the console says
/// which.
fn is_reference(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    a.set_result(e, 0.0);
    let reference = Read::of(e, &a).ok;
    if reference {
        a.set_result(e, 1.0);
    }
    if is_console_mode(e) {
        let line = if reference {
            "IsReference"
        } else {
            "Not reference!"
        };
        console_print(e, line, &[]);
    }
    handled(true)
}
