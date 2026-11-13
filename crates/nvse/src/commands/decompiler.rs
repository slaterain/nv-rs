//! xNVSE's commands over its decompiler (`ScriptAnalyzer.cpp`): `Assert` (`Commands_Scripting.cpp`),
//! `DecompileScript` and `HasScriptCommand` (`Commands_Script.cpp`).
//!
//! Read from xNVSE's source (xNVSE 6.4.9, `0ccd23a`).

use super::{arg_in_buffer, console_print, is_console_mode, Read};
use crate::decompile;
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

/// `kNVSEParamType_Boolean` (a boolean or an ambiguous token).
const NVSE_BOOLEAN: u32 = 0x8002;
/// `kParamType_Integer`, `kParamType_String`, `kParamType_AnyForm`.
const INTEGER: u32 = 0x01;
const STRING: u32 = 0x00;
const ANY_FORM: u32 = 0x3d;

const ONE_BOOLEAN: &[Param] = &[p("boolean expression", NVSE_BOOLEAN, false)];
const ONE_FORM_ONE_OPTIONAL_STRING: &[Param] =
    &[p("form", ANY_FORM, false), p("string", STRING, true)];
const HAS_SCRIPT_COMMAND: &[Param] = &[
    p("command opcode", INTEGER, false),
    p("form", ANY_FORM, true),
    p("event block id", INTEGER, true),
];

const fn cmd(
    opcode: u32,
    name: &'static str,
    short: &'static str,
    help: &'static str,
    params: &'static [Param],
    execute: AbiFn,
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
        returns: Returns::Default,
    }
}

#[rustfmt::skip]
pub static COMMANDS: &[Command] = &[
    cmd(0x162a, "DecompileScript", "", "decompiles a script to file", ONE_FORM_ONE_OPTIONAL_STRING, decompile_script),
    cmd(0x162b, "HasScriptCommand", "", "returns 1 if script contains call to a command", HAS_SCRIPT_COMMAND, has_script_command),
    cmd(0x1646, "Assert", "AssertTrue", "", ONE_BOOLEAN, assert),
];

/// `Console_Print(format, text)` with the text in guest memory; nothing before the first frame.
fn print_with(e: &mut Engine, format: &str, text: &[u8]) {
    if !crate::Nvse::prints(e) {
        return;
    }
    let size = (text.len() as u32 + 1).next_multiple_of(4);
    e.with_stack(size, |e, buffer| {
        e.mem.write(buffer.addr(), text);
        e.mem.set_u8(buffer.addr() + text.len() as u32, 0);
        console_print(e, format, &[buffer.addr()]);
    });
}

fn print_plain(e: &mut Engine, text: &str) {
    if crate::Nvse::prints(e) {
        console_print(e, text, &[]);
    }
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_Scripting.cpp:
// Cmd_Assert_Execute.
/// When the expression does not read or is false: "Assertion failed!", then the line as the
/// evaluator writes it back and its variables' values, each on a console line after a tab.
fn assert(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    // `with_eval` with the evaluator asked to keep the variables it resolves (for "Where").
    let mut ev = crate::expr::Eval::new(e, &a);
    ev.resolved_vars = Some(Vec::new());
    crate::expr::with_evaluator(e, |e| {
        let ev = &mut ev;
        let toks = ev.extract_args(e);
        let holds = toks
            .and_then(|t| t.into_iter().next())
            .is_some_and(|t| ev.get_number(e, &t) != 0.0);
        if holds {
            return;
        }
        let line = decompile::evaluator_line_text(e, ev);
        let vars = decompile::evaluator_variables_text(e, ev);
        print_plain(e, "Assertion failed!");
        if !line.is_empty() {
            print_with(e, "\t%s", &line);
        }
        if !vars.is_empty() {
            print_with(e, "\tWhere %s", &vars);
        }
    });
    crate::expr::report(e, &ev);
    handled(true)
}

/// `CreateDirectoryA` and the file calls, through the exe's imports (relative paths are the game
/// process's working folder's, as xNVSE's `std::filesystem` and `std::ofstream` take them).
const CREATE_DIRECTORY_A: u32 = 0x00fd_f0b8;

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_Script.cpp:
// DecompileScriptToFolder.
/// `DecompiledScripts/<mod>/<name>.<extension>` with the script's text (a text-mode stream: each
/// new line written `\r\n`); in the console, a line naming the file, or that the script is not
/// compiled.
fn decompile_to_folder(e: &mut Engine, name: &[u8], script: u32, extension: &[u8], modname: &[u8]) {
    if !decompile::parses(e, script) {
        if is_console_mode(e) {
            print_with(e, "Script %s is not compiled", name);
        }
        return;
    }
    let dir = b"DecompiledScripts".to_vec();
    let mut mod_dir = dir.clone();
    mod_dir.push(b'/');
    mod_dir.extend_from_slice(modname);
    for d in [&dir, &mod_dir] {
        crate::events::with_guest_text(e, d, |e, p| {
            e.call(CREATE_DIRECTORY_A, &args![p, 0u32]);
        });
    }
    let mut path = mod_dir;
    path.push(b'/');
    path.extend_from_slice(name);
    path.push(b'.');
    path.extend_from_slice(extension);
    let text = decompile::decompile(e, script);
    let mut out = Vec::with_capacity(text.len() + text.len() / 16);
    for b in text {
        if b == b'\n' {
            out.push(b'\r');
        }
        out.push(b);
    }
    super::array::write_text_file(e, &path, false, &out);
    if is_console_mode(e) {
        print_with(e, "Decompiled script to '%s'", &path);
    }
}

/// `tList` items from the inline first node at `head`.
fn list_items(e: &Engine, head: u32) -> Vec<u32> {
    let mut out = Vec::new();
    let mut node = head;
    while node != 0 {
        out.push(e.mem.u32(node));
        node = e.mem.u32(node + 4);
    }
    out
}

/// `TESForm::GetIsReference`, the `+0xF0` virtual function.
const GET_IS_REFERENCE: u32 = 0xf0;

/// `TESTopicInfo::GetResultScript` (`0061EB60`).
const TOPIC_INFO_RESULT_SCRIPT: u32 = 0x0061_eb60;

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_Script.cpp:
// Cmd_DecompileScript_Execute.
/// Writes the script of a script, or each result script of a quest's stages, a package's three
/// actions, a dialogue response's two, or a terminal's menu entries; 1 for a form of those kinds.
fn decompile_script(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    a.set_result(e, 0.0);
    let read = Read::of(e, &a);
    if !read.ok {
        return handled(true);
    }
    let form = read.form(0);
    let ext = arg_in_buffer(e, &read, 1, 0x100, "DecompileScript");
    let ext = if ext.is_empty() { b"gek".to_vec() } else { ext };
    let name_at = e.vcall(form, 0x130, &args![]).u32();
    let mut name = if name_at == 0 {
        Vec::new()
    } else {
        crate::table::guest_text(e, name_at)
    };
    if name.is_empty() {
        name = format!("{:08X}", e.mem.u32(form + 0xc) & 0x00ff_ffff).into_bytes();
    }
    let modname = crate::errors::mod_name(e, form);
    let suffixed = |n: &[u8], s: String| {
        let mut v = n.to_vec();
        v.extend(s.into_bytes());
        v
    };
    match e.mem.u8(form + 4) {
        0x11 => decompile_to_folder(e, &name, form, &ext, &modname),
        0x47 => {
            for stage in list_items(e, form + 0x44) {
                if stage == 0 {
                    continue;
                }
                let entry = e.mem.u32(stage + 4);
                if entry != 0 && e.mem.u32(entry + 0xc + 0x20) != 0 {
                    let n = suffixed(&name, format!(" #{}", e.mem.u8(stage)));
                    decompile_to_folder(e, &n, entry + 0xc, &ext, &modname);
                }
            }
        }
        0x49 => {
            for (suffix, at) in [(" OnBegin", 0x50), (" OnEnd", 0x60), (" OnChange", 0x70)] {
                let script = e.mem.u32(form + at);
                if script != 0 {
                    let n = suffixed(&name, suffix.to_string());
                    decompile_to_folder(e, &n, script, &ext, &modname);
                }
            }
        }
        0x46 => {
            for (end, suffix) in [(0u32, " Begin"), (1, " End")] {
                let script = e.call(TOPIC_INFO_RESULT_SCRIPT, &args![form, end]).u32();
                if script != 0 {
                    let n = suffixed(&name, suffix.to_string());
                    decompile_to_folder(e, &n, script, &ext, &modname);
                }
            }
        }
        0x17 => {
            let mut index = 0;
            for entry in list_items(e, form + 0x98) {
                if entry == 0 {
                    continue;
                }
                index += 1;
                if e.mem.u32(entry + 0x10 + 0x20) != 0 {
                    let n = suffixed(&name, format!(" #{index}"));
                    decompile_to_folder(e, &n, entry + 0x10, &ext, &modname);
                }
            }
        }
        _ => return handled(true),
    }
    a.set_result(e, 1.0);
    handled(true)
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_Script.cpp:
// Cmd_HasScriptCommand_Execute.
/// 1 when the script (the form's, or a reference's script; the calling reference's without a
/// form) calls the command of that opcode, within the given block type's blocks when one is given.
fn has_script_command(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    a.set_result(e, 0.0);
    let read = Read::of(e, &a);
    if !read.ok {
        return handled(true);
    }
    let opcode = read.int(0, 0);
    let mut form = read.form(1);
    let event_op = read.int(2, u32::MAX);
    if form == 0 {
        form = a.this_obj;
    }
    if form == 0 {
        return handled(true);
    }
    let script = if e.mem.u8(form + 4) == 0x11 {
        form
    } else if e.vcall(form, GET_IS_REFERENCE, &args![]).bool() {
        const EXTRA_GET_BY_TYPE: u32 = 0x0041_0220;
        let extra = e
            .call(EXTRA_GET_BY_TYPE, &args![form + 0x44, 0x0du32])
            .u32();
        if extra == 0 {
            0
        } else {
            e.mem.u32(extra + 0xc)
        }
    } else {
        0
    };
    if script == 0 {
        return handled(true);
    }
    let record = crate::table::record_of(e, opcode);
    if record == 0 {
        return handled(true);
    }
    let event = if event_op == u32::MAX {
        0
    } else {
        decompile::event_block(event_op)
    };
    if decompile::contains_command(e, script, record, event) {
        a.set_result(e, 1.0);
    }
    handled(true)
}

#[cfg(test)]
#[path = "decompiler_tests.rs"]
mod tests;
