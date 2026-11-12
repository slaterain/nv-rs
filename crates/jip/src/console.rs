//! JIP LN NVSE's console echo (`DoConsolePrint` (JIP LN NVSE), `internal/jip_core.cpp`): a JIP
//! command run from the console as a line of its own prints its name and its result there,
//! `GetLNVersion >> 57.3`. Inside an expression, a condition or a script it prints nothing.

use crate::utility::flt_to_str;
use engine_core::prelude::*;
use nvse::CommandArgs;

/// `ScriptRunner::EvaluateLine` (`005e1550`). JIP prints only when the command's execute was
/// called by it for a line of its own: the handler's return address is `005E234B`, after the call
/// at `005E2345`.
const EVALUATE_LINE: u32 = 0x005e_1550;
/// `ConsoleManager_GetSingleton` (`0071b160`, with true: `g_consoleManager`, which JIP takes at
/// its deferred start) and `ConsoleManager::Print` (`0071d0a0`, `thiscall`: a format and a
/// `va_list`).
const CONSOLE_INSTANCE: u32 = 0x0071_b160;
const CONSOLE_PRINT: u32 = 0x0071_d0a0;

/// Whether the running handler was called by `EvaluateLine`: the caller on the engine's call
/// stack, below the handler's own entry (`[ebp+4]` in JIP's code). A wrap at `EvaluateLine`
/// runs as part of the same entry.
fn called_by_evaluate_line(e: &Engine) -> bool {
    let stack = &e.call_stack;
    stack.len() >= 2 && stack[stack.len() - 2] == EVALUATE_LINE
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/jip_core.cpp: DoConsolePrint (the one
// for a double result).
/// With the console open (`IsConsoleOpen`, xNVSE's console-mode test) and the command run by
/// `EvaluateLine`, prints the name of the record `EvaluateLine` runs (its local at `[ebp-0x30]`:
/// the record of the line's opcode), ` >> ` and the result as [`flt_to_str`] writes it, through
/// the console's print with the line as its format (it holds no `%`).
pub fn print_result(e: &mut Engine, a: &CommandArgs) {
    if !nvse::commands::is_console_mode(e) || !called_by_evaluate_line(e) {
        return;
    }
    let record = nvse::table::record_of(e, a.opcode(e));
    let mut line = nvse::table::guest_text(e, e.mem.u32(record));
    line.extend_from_slice(b" >> ");
    line.extend(flt_to_str(e.mem.f64(a.result)));
    console_print_line(e, &line);
}

/// The console's print with `line` as its format (it holds no `%` JIP would expand: JIP passes
/// its buffer as both the format and the argument).
fn console_print_line(e: &mut Engine, line: &[u8]) {
    let console = e.call(CONSOLE_INSTANCE, &args![1u32]).u32();
    if console == 0 {
        return;
    }
    let size = (line.len() as u32 + 1).next_multiple_of(4);
    e.with_stack(size, |e, text| {
        for (i, b) in line.iter().enumerate() {
            e.mem.set_u8(text.addr() + i as u32, *b);
        }
        e.mem.set_u8(text.addr() + line.len() as u32, 0);
        e.call(CONSOLE_PRINT, &args![console, text, text]);
    });
}

/// `UIntToHex` (JIP LN NVSE, `internal/utility.cpp`): a number in upper-case hexadecimal without
/// leading zeros, `0` for zero.
pub fn uint_to_hex(num: u32) -> Vec<u8> {
    format!("{num:X}").into_bytes()
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/jip_core.cpp: DoConsolePrint (the one
// for a form result).
/// As [`print_result`], with the form instead of a number: its id in hexadecimal, then its
/// editor ID in quotes when the form answers a non-empty one; `0` for no form.
pub fn print_form(e: &mut Engine, a: &CommandArgs, form: u32) {
    if !nvse::commands::is_console_mode(e) || !called_by_evaluate_line(e) {
        return;
    }
    let record = nvse::table::record_of(e, a.opcode(e));
    let mut line = nvse::table::guest_text(e, e.mem.u32(record));
    line.extend_from_slice(b" >> ");
    if form == 0 {
        line.push(b'0');
    } else {
        line.extend(uint_to_hex(crate::commands::ref_id(e, form)));
        let edid = e.vcall(form, crate::commands::forms::GET_EDITOR_ID, &[]).u32();
        if edid != 0 && e.mem.u8(edid) != 0 {
            line.extend_from_slice(b" \"");
            line.extend(nvse::table::guest_text(e, edid));
            line.push(b'"');
        }
    }
    console_print_line(e, &line);
}
