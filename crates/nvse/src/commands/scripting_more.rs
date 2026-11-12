//! The rest of xNVSE's `Commands_Scripting.cpp` (xNVSE): expressions printed to the console
//! (`Print`, `PrintDebug`, `PrintVar`, `DebugPrintVar`) or to a file (`PrintF`, `PrintDebugF`),
//! turned into a string (`ToString`) and named by type (`TypeOf`); and the string representation
//! they print (`ScriptToken`, `ArrayVar`, `ArrayElement` and `TESForm::GetStringRepresentation`).
//!
//! Read from xNVSE's source (xNVSE 6.4.9, `0ccd23a`).

use super::console::{mod_debug_state, print_str};
use super::printf::ucrt_format;
use super::string::{assign_to_string_var, with_eval};
use crate::expr::{Eval, Tok, Ty};
use crate::table::{guest_text, Command, Param, Returns};
use crate::vars::{self, Container, Data};
use crate::{handled, CommandArgs};
use engine_core::prelude::*;

const fn p(name: &'static str, kind: u32, optional: bool) -> Param {
    Param {
        name,
        kind,
        optional,
    }
}
/// `kNVSEParamType_BasicType`, `_String` and `_Variable` (xNVSE `ScriptUtils.h`): token type
/// bits, `Ambiguous` (bit 15) with the first two.
const BASIC_TYPE: u32 = 0x804d;
const NVSE_STRING: u32 = 0x8004;
const VARIABLE: u32 = 0x400;
/// `kParams_OneBasicType`, `kParams_OneNVSEString`, `kParams_TwoNVSEString` and
/// `kParams_OneNVSEVariable_OptionalNVSEVariables` (fifteen variables, the first required).
const ONE_BASIC_TYPE: &[Param] = &[p("expression", BASIC_TYPE, false)];
const ONE_NVSE_STRING: &[Param] = &[p("string", NVSE_STRING, false)];
const TWO_NVSE_STRINGS: &[Param] = &[
    p("filename", NVSE_STRING, false),
    p("string", NVSE_STRING, false),
];
const VARIABLES: &[Param] = &[
    p("variable", VARIABLE, false),
    p("variable", VARIABLE, true),
    p("variable", VARIABLE, true),
    p("variable", VARIABLE, true),
    p("variable", VARIABLE, true),
    p("variable", VARIABLE, true),
    p("variable", VARIABLE, true),
    p("variable", VARIABLE, true),
    p("variable", VARIABLE, true),
    p("variable", VARIABLE, true),
    p("variable", VARIABLE, true),
    p("variable", VARIABLE, true),
    p("variable", VARIABLE, true),
    p("variable", VARIABLE, true),
    p("variable", VARIABLE, true),
];

const fn cmd(
    opcode: u32,
    name: &'static str,
    short: &'static str,
    help: &'static str,
    params: &'static [Param],
    execute: AbiFn,
    returns: Returns,
) -> Command {
    Command {
        opcode,
        name,
        short,
        help,
        needs_ref: false,
        params,
        execute,
        eval: None,
        returns,
    }
}

const RD: Returns = Returns::Default;
const RS: Returns = Returns::String;

#[rustfmt::skip]
pub static COMMANDS: &[Command] = &[
    cmd(0x1540, "ToString", "", "attempts to convert an expression to a string", ONE_BASIC_TYPE, to_string, RS),
    cmd(0x1541, "Print", "", "prints a string expression to the console", ONE_NVSE_STRING, print, RD),
    cmd(0x1543, "TypeOf", "", "returns a string representing the type of the expression", ONE_BASIC_TYPE, type_of, RS),
    cmd(0x15bd, "PrintDebug", "PrintD", "prints a string expression to the console in debug mode", ONE_NVSE_STRING, print_debug, RD),
    cmd(0x15f6, "PrintF", "", "prints a string expression to a file", TWO_NVSE_STRINGS, print_f, RD),
    cmd(0x15f7, "PrintDebugF", "PrintDF", "prints a string expression to a file in debug mode", TWO_NVSE_STRINGS, print_debug_f, RD),
    cmd(0x1626, "PrintVar", "PrintV", "", VARIABLES, print_var, RD),
    cmd(0x165c, "DebugPrintVar", "DPrintV", "", VARIABLES, debug_print_var, RD),
];

/// `LookupFormByID` (`0x004839C0`).
const LOOKUP_FORM: u32 = 0x0048_39c0;
/// `TESForm::refID` (`+0xC`) and its name (`GetName`, the `+0x130` virtual function: the editor
/// id the game keeps, or none).
const FORM_ID: u32 = 0xc;
const FORM_NAME: u32 = 0x130;

/// `FormatString("%g", v)`: the C runtime's shortest form of a number.
fn g(e: &Engine, v: f64) -> Vec<u8> {
    ucrt_format(e, b"%g", &v.to_le_bytes())
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), GameForms.cpp:
// TESForm::GetStringRepresentation.
/// `[id: <id in hex>, edid: "<editor id>", name: "<name>"]`; the name is "<no name>" for a form
/// without the name component.
pub(crate) fn form_representation(e: &mut Engine, form: u32) -> Vec<u8> {
    let id = e.mem.u32(form + FORM_ID);
    let edid_at = e.vcall(form, FORM_NAME, &args![]).u32();
    let edid = if edid_at == 0 {
        Vec::new()
    } else {
        guest_text(e, edid_at)
    };
    let full = super::string::tes_full_name(e, form);
    let name = if full == 0 {
        b"<no name>".to_vec()
    } else {
        let data = e.mem.u32(full + 4);
        if data == 0 {
            Vec::new()
        } else {
            guest_text(e, data)
        }
    };
    let mut out = format!("[id: {id:X}, edid: \"").into_bytes();
    out.extend_from_slice(&edid);
    out.extend_from_slice(b"\", name: \"");
    out.extend_from_slice(&name);
    out.extend_from_slice(b"\"]");
    out
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), ArrayVar.cpp:
// ArrayElement::GetStringRepresentation.
/// An element: its number, its form (or "null" for an id no form has), its string in quotes or
/// its array (or "invalid array").
fn element_representation(e: &mut Engine, data: &Data) -> Vec<u8> {
    match data {
        Data::Invalid => b"invalid".to_vec(),
        Data::Number(v) => g(e, *v),
        Data::Form(id) => {
            let form = e.call(LOOKUP_FORM, &args![*id]).u32();
            if form == 0 {
                b"null".to_vec()
            } else {
                form_representation(e, form)
            }
        }
        Data::Str(s) => {
            let mut out = vec![b'"'];
            out.extend_from_slice(s);
            out.push(b'"');
            out
        }
        Data::Array(id) => {
            if vars::array_exists(e, *id) {
                array_representation(e, *id)
            } else {
                b"invalid array".to_vec()
            }
        }
    }
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), ArrayVar.cpp:
// ArrayVar::GetStringRepresentation.
/// `[a, b]` for an array; `[key: a, ...]` for a map, a number key as `std::to_string` writes it
/// (`%f`) and a string key in quotes.
pub(crate) fn array_representation(e: &mut Engine, id: u32) -> Vec<u8> {
    let Some(elements) = vars::array(e, id).map(|a| a.elements.clone()) else {
        return b"invalid array".to_vec();
    };
    let mut parts: Vec<Vec<u8>> = Vec::new();
    match &elements {
        Container::Array(v) => {
            for el in v {
                parts.push(element_representation(e, &el.data));
            }
        }
        Container::NumMap(v) => {
            for (key, el) in v {
                let mut part = ucrt_format(e, b"%f", &key.to_le_bytes());
                part.extend_from_slice(b": ");
                part.extend(element_representation(e, &el.data));
                parts.push(part);
            }
        }
        Container::StrMap(v) => {
            for (key, el) in v {
                let mut part = vec![b'"'];
                part.extend_from_slice(key);
                part.extend_from_slice(b"\": ");
                part.extend(element_representation(e, &el.data));
                parts.push(part);
            }
        }
    }
    let mut out = vec![b'['];
    out.extend(parts.join(&b", "[..]));
    out.push(b']');
    out
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), ScriptTokens.cpp:
// ScriptToken::GetStringRepresentation.
/// A token as a string, a number, a form or an array would print; empty for anything else.
pub(crate) fn token_representation(e: &mut Engine, ev: &Eval, t: &Tok) -> Vec<u8> {
    if ev.can_convert(e, t, Ty::String) {
        return ev.get_string(e, t);
    }
    if ev.can_convert(e, t, Ty::Number) {
        let v = ev.get_number(e, t);
        return g(e, v);
    }
    if ev.can_convert(e, t, Ty::Form) {
        let form = ev.get_form(e, t);
        if form != 0 {
            return form_representation(e, form);
        }
    }
    if ev.can_convert(e, t, Ty::Array) {
        let id = ev.get_array_id(e, t);
        if vars::array_exists(e, id) {
            return array_representation(e, id);
        }
    }
    Vec::new()
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_Scripting.cpp:
// Cmd_ToString_Execute.
/// The expression as a string variable: a string as it is, a number in `%g`, a form's name;
/// "NULL" for anything else.
fn to_string(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    a.set_result(e, 0.0);
    let mut text = b"NULL".to_vec();
    with_eval(e, &a, |e, ev, toks| {
        let Some(t) = toks.and_then(|t| t.into_iter().next()) else {
            return;
        };
        if ev.can_convert(e, &t, Ty::String) {
            text = ev.get_string(e, &t);
        } else if ev.can_convert(e, &t, Ty::Number) {
            let v = ev.get_number(e, &t);
            text = g(e, v);
        } else if ev.can_convert(e, &t, Ty::Form) {
            let form = ev.get_form(e, &t);
            let name = super::misc_ref::full_name_text(e, form);
            text = guest_text(e, name);
        }
    });
    assign_to_string_var(e, &a, &text);
    handled(true)
}

/// `Print`'s run: the expression's string representation to the console.
fn print_expression(e: &mut Engine, a: &CommandArgs) {
    with_eval(e, a, |e, ev, toks| {
        if let Some(t) = toks.and_then(|t| t.into_iter().next()) {
            let text = token_representation(e, ev, &t);
            print_str(e, &text);
        }
    });
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_Scripting.cpp:
// Cmd_Print_Execute.
/// The expression's string representation to the console (in lines of 500 when it is long).
fn print(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    a.set_result(e, 0.0);
    print_expression(e, &a);
    handled(true)
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_Scripting.cpp:
// Cmd_PrintDebug_Execute.
/// `Print` when the calling script's mod has debug mode on (`SetDebugMode`).
fn print_debug(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    a.set_result(e, 0.0);
    if mod_debug_state(e, a.script) {
        print_expression(e, &a);
    }
    handled(true)
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_Scripting.cpp: Cmd_PrintF.
/// The second string and a new line at the end of the file the first names (`fopen_s` `"a+"`,
/// text mode: a new line is `\r\n`), through the game's own file calls; a file that cannot be
/// opened is passed over. With `debug`, only when the calling script's mod has debug mode on.
fn print_to_file(e: &mut Engine, w: &[u32], debug: bool) -> Ret {
    let a = CommandArgs::new(w);
    a.set_result(e, 0.0);
    if debug && !mod_debug_state(e, a.script) {
        return handled(true);
    }
    with_eval(e, &a, |e, ev, toks| {
        let Some(toks) = toks else {
            return;
        };
        let (Some(path), Some(text)) = (toks.first(), toks.get(1)) else {
            return;
        };
        if !ev.can_convert(e, path, Ty::String) || !ev.can_convert(e, text, Ty::String) {
            return;
        }
        let path = ev.get_string(e, path);
        let text = ev.get_string(e, text);
        let mut out = Vec::with_capacity(text.len() + 2);
        for &b in &text {
            if b == b'\n' {
                out.extend_from_slice(b"\r\n");
            } else {
                out.push(b);
            }
        }
        out.extend_from_slice(b"\r\n");
        super::array::write_text_file(e, &path, true, &out);
    });
    handled(true)
}

fn print_f(e: &mut Engine, w: &[u32]) -> Ret {
    print_to_file(e, w, false)
}

fn print_debug_f(e: &mut Engine, w: &[u32]) -> Ret {
    print_to_file(e, w, true)
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_Scripting.cpp:
// Cmd_TypeOf_Execute.
/// "Number", "String", "Form", "Array", "Map" or "StringMap" (or "<Bad Array>" for an array id
/// with no array), as a string variable; "NULL" for anything else.
fn type_of(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let mut name = "NULL";
    with_eval(e, &a, |e, ev, toks| {
        let Some(t) = toks.and_then(|t| t.into_iter().next()) else {
            return;
        };
        if ev.can_convert(e, &t, Ty::Number) {
            name = "Number";
        } else if ev.can_convert(e, &t, Ty::String) {
            name = "String";
        } else if ev.can_convert(e, &t, Ty::Form) {
            name = "Form";
        } else if ev.can_convert(e, &t, Ty::Array) {
            let id = ev.get_array_id(e, &t);
            name = match vars::array(e, id) {
                None => "<Bad Array>",
                Some(arr) if arr.key_type == vars::KEY_NUMBER && arr.packed => "Array",
                Some(arr) if arr.key_type == vars::KEY_NUMBER => "Map",
                Some(arr) if arr.key_type == vars::KEY_STRING => "StringMap",
                Some(_) => "NULL",
            };
        }
    });
    assign_to_string_var(e, &a, name.as_bytes());
    handled(true)
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), GameAPI.cpp: GetParentScript and
// GetVariableName.
/// The name of a variable token's local: in the script of the reference it is read through
/// (`ref.var`: the reference's own script), else in `script`; empty when no variable of that id
/// is there.
fn variable_name(e: &mut Engine, t: &Tok, script: u32, event_list: u32) -> Vec<u8> {
    let Tok::Var { local, ref_idx, .. } = *t else {
        return Vec::new();
    };
    if local == 0 || script == 0 {
        return Vec::new();
    }
    let mut owner = script;
    if ref_idx != 0 {
        let ref_var = crate::args::script_ref(e, script, u32::from(ref_idx));
        if ref_var != 0 {
            crate::args::resolve_ref_variable(e, ref_var, event_list);
            let form = e.mem.u32(ref_var + 8);
            if form != 0 {
                let list = crate::expr::event_list_from_form(e, form);
                if list != 0 && e.mem.u32(list) != 0 {
                    owner = e.mem.u32(list);
                }
            }
        }
    }
    let id = e.mem.u32(local);
    let mut node = owner + 0x4c;
    while node != 0 {
        let info = e.mem.u32(node);
        if info != 0 && e.mem.u32(info) == id {
            return guest_text(e, e.mem.u32(info + 0x18));
        }
        node = e.mem.u32(node + 4);
    }
    Vec::new()
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_Scripting.cpp:
// Cmd_PrintVar_Execute.
/// `name: value` to the console for each variable: a number in `%g`, a string in quotes, an
/// array's representation (or "uninitialized or invalid array"), a reference's form (or
/// "invalid form", and then nothing). Nothing when the first argument is not a variable.
fn print_var(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    with_eval(e, &a, |e, ev, toks| {
        let Some(toks) = toks else {
            return;
        };
        match toks.first() {
            Some(Tok::Var { local, .. }) if *local != 0 => {}
            _ => return,
        }
        for t in &toks {
            let value = match t.ty() {
                Ty::NumericVar => {
                    let v = ev.get_number(e, t);
                    g(e, v)
                }
                Ty::StringVar => {
                    let mut out = vec![b'"'];
                    out.extend(ev.get_string(e, t));
                    out.push(b'"');
                    out
                }
                Ty::ArrayVar => {
                    let id = ev.get_array_id(e, t);
                    if vars::array_exists(e, id) {
                        array_representation(e, id)
                    } else {
                        b"uninitialized or invalid array".to_vec()
                    }
                }
                Ty::RefVar => {
                    let form = ev.get_form(e, t);
                    if form == 0 {
                        b"invalid form".to_vec()
                    } else {
                        form_representation(e, form)
                    }
                }
                _ => Vec::new(),
            };
            let mut line = variable_name(e, t, a.script, a.event_list);
            line.extend_from_slice(b": ");
            line.extend(value);
            print_str(e, &line);
        }
    });
    handled(true)
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_Scripting.cpp:
// Cmd_DebugPrintVar_Execute.
/// `PrintVar` when the calling script's mod has debug mode on.
fn debug_print_var(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    if mod_debug_state(e, a.script) {
        return print_var(e, w);
    }
    handled(true)
}
