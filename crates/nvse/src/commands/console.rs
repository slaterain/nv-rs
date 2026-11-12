//! xNVSE's `Commands_Console.cpp` (xNVSE): a formatted line printed to the console, the per-mod
//! debug mode and the lines it lets through, the game's echo of its own lines to the console, and
//! the console's output file (`SCOF`).
//!
//! Read from xNVSE's source (xNVSE 6.4.9, `0ccd23a`). The lines go through the game's console
//! (`ConsoleManager::Print`), and xNVSE prints nothing before the first frame recorded the main
//! thread (`Console_Print`'s `s_recordedMainThreadID`).

use super::string::{assign_to_string_var, format_string_args};
use super::{console_print, is_console_mode, Read};
use crate::table::{Command, Param, Returns};
use crate::{handled, CommandArgs, Nvse};
use engine_core::prelude::*;

const fn p(name: &'static str, kind: u32, optional: bool) -> Param {
    Param {
        name,
        kind,
        optional,
    }
}
/// `kParamType_Integer` (xNVSE `CommandTable.h`).
const INT: u32 = 0x01;
/// `kParams_OneInt_OneOptionalInt`, `kParams_OneOptionalInt`, `kParams_OneInt`,
/// `kParams_FormatString` (xNVSE `ParamInfos.h`).
const ONE_INT_ONE_OPTIONAL_INT: &[Param] = &[p("int", INT, false), p("int", INT, true)];
const ONE_OPTIONAL_INT: &[Param] = &[p("int", INT, true)];
const ONE_INT: &[Param] = &[p("int", INT, false)];
const FORMAT_STRING: [Param; 21] = super::string::fmt_params(&[]);

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
    cmd(0x14a0, "PrintToConsole", "printc", "Print formatted string to console", &FORMAT_STRING, print_to_console, RD),
    cmd(0x14a1, "DebugPrint", "dbprintc", "prints a formatted string to the console if debug mode is enabled for calling script", &FORMAT_STRING, debug_print, RD),
    cmd(0x14a2, "SetDebugMode", "dbmode", "toggles debug mode for all scripts belonging to a mod", ONE_INT_ONE_OPTIONAL_INT, set_debug_mode, RD),
    cmd(0x14a3, "GetDebugMode", "GetDbMode", "returns whether debug mode is set for the mod", ONE_OPTIONAL_INT, get_debug_mode, RD),
    cmd(0x15b1, "GetConsoleEcho", "", "returns whether the engine echoes to the console", ONE_OPTIONAL_INT, get_console_echo, RD),
    cmd(0x15b2, "SetConsoleEcho", "", "toggles wether the engine echoes to the console and return previous state", ONE_INT, set_console_echo, RD),
    cmd(0x15f4, "HasConsoleOutputFilename", "HasCOF", "\"return if there is a Console Output Filename active\"", &[], has_console_output_filename, RD),
    cmd(0x15f5, "GetConsoleOutputFilename", "GetCOF", "\"returns the name of the Console Output Filename\"", &[], get_console_output_filename, RS),
];

/// The mods whose scripts' debug lines print: a bit per mod index (`ModDebugStates`, eight
/// 32-bit fields).
#[derive(Default)]
pub struct State {
    pub debug: [u32; 8],
}

/// `ConsoleManager_GetSingleton` (`0x0071B160`, created when absent) and its
/// `COFileName` (`+0x810`, `char[260]`).
const CONSOLE_INSTANCE: u32 = 0x0071_b160;
const OUTPUT_FILENAME: u32 = 0x810;
/// `bEchoConsole` (xNVSE `GameAPI.cpp`), the byte at `0x011F158C`.
const ECHO_CONSOLE: u32 = 0x011f_158c;

/// `Console_Print("%s", text)` (xNVSE): the text as it is, nothing before the first frame.
pub(crate) fn print_text(e: &mut Engine, text: &[u8]) {
    if !Nvse::prints(e) {
        return;
    }
    let size = (text.len() as u32 + 1).next_multiple_of(4);
    e.with_stack(size, |e, buffer| {
        e.mem.write(buffer.addr(), text);
        e.mem.set_u8(buffer.addr() + text.len() as u32, 0);
        console_print(e, "%s", &[buffer.addr()]);
    });
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Utilities.cpp: Console_Print_Str and
// Console_Print_Long.
/// A text under 512 bytes in one line; a longer one in lines of 500 bytes, each followed by " ...",
/// then the rest.
pub(crate) fn print_str(e: &mut Engine, text: &[u8]) {
    if text.len() < 512 {
        print_text(e, text);
        return;
    }
    let lines = text.len() / 500;
    for i in 0..lines {
        let mut line = text[i * 500..(i + 1) * 500].to_vec();
        line.extend_from_slice(b" ...");
        print_text(e, &line);
    }
    print_text(e, &text[lines * 500..]);
}

/// `Script::GetModIndex` (xNVSE): the top byte of the script's form id; a call without a script
/// counts as mod 0 (xNVSE reads through the null pointer).
fn mod_index(e: &Engine, script: u32) -> u8 {
    if script == 0 {
        0
    } else {
        (e.mem.u32(script + 0xc) >> 24) as u8
    }
}

/// Whether mod `index`'s debug lines print.
fn debug_on(e: &mut Engine, index: u8) -> bool {
    e.ext::<Nvse>().console.debug[usize::from(index / 32)] & (1 << (index % 32)) != 0
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_Console.cpp: ModDebugState.
/// Whether the debug lines of `script`'s mod print.
pub(crate) fn mod_debug_state(e: &mut Engine, script: u32) -> bool {
    let index = mod_index(e, script);
    debug_on(e, index)
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_Console.cpp:
// Cmd_PrintToConsole_Execute.
/// The formatted line to the console, and 1, when it is under 512 bytes; else a line saying so.
fn print_to_console(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    a.set_result(e, 0.0);
    let formatted = format_string_args(e, &a, 21);
    if formatted.ok {
        if formatted.text.len() < 512 {
            a.set_result(e, 1.0);
            print_text(e, &formatted.text);
        } else {
            print_text(e, b"PrintToConsole >> Max length exceeded (512 chars)");
        }
    }
    handled(true)
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_Console.cpp:
// Cmd_DebugPrint_Execute.
/// `PrintToConsole` when the calling script's mod has debug mode on; 0.
fn debug_print(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    a.set_result(e, 0.0);
    if mod_debug_state(e, a.script) {
        return print_to_console(e, w);
    }
    handled(true)
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_Console.cpp:
// Cmd_SetDebugMode_Execute.
/// Debug mode on or off for a mod (the calling script's without the second argument); mod 0 and
/// `0xFF` are never toggled. The console says what changed.
fn set_debug_mode(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    a.set_result(e, 0.0);
    let read = Read::of(e, &a);
    if !read.ok {
        return handled(true);
    }
    let enable = read.int(0, 0);
    let mut index_arg = read.int(1, 0xffff);
    if index_arg == 0xffff {
        index_arg = u32::from(mod_index(e, a.script));
    }
    let index = index_arg as u8;
    if index > 0 && index < 0xff {
        let bit = 1 << (index % 32);
        let field = &mut e.ext::<Nvse>().console.debug[usize::from(index / 32)];
        if enable != 0 {
            *field |= bit;
        } else {
            *field &= !bit;
        }
        if is_console_mode(e) {
            let state = crate::table::guest_str(e, if enable != 0 { b"on" } else { b"off" });
            console_print(
                e,
                "Debug statements toggled %s for mod %02X",
                &[state, index_arg],
            );
        }
    }
    handled(true)
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_Console.cpp:
// Cmd_GetDebugMode_Execute.
/// 1 when a mod's debug mode is on (the calling script's without an argument); the console says.
fn get_debug_mode(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    a.set_result(e, 0.0);
    let read = Read::of(e, &a);
    if !read.ok {
        return handled(true);
    }
    let mut index_arg = read.int(0, 0xffff);
    if index_arg == 0xffff {
        index_arg = u32::from(mod_index(e, a.script));
    }
    let on = debug_on(e, index_arg as u8);
    if is_console_mode(e) {
        let state = crate::table::guest_str(e, if on { b"on" } else { b"off" });
        console_print(e, "Debug statements %s for mod %02X", &[state, index_arg]);
    }
    if on {
        a.set_result(e, 1.0);
    }
    handled(true)
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_Console.cpp:
// Cmd_GetConsoleEcho_Execute.
/// 1 while the game echoes its lines to the console.
fn get_console_echo(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let echo = e.mem.u8(ECHO_CONSOLE) != 0;
    a.set_result(e, f64::from(u8::from(echo)));
    handled(true)
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_Console.cpp:
// Cmd_SetConsoleEcho_Execute.
/// The echo as it was, then set from the argument (left as it was when it cannot be read).
fn set_console_echo(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let echo = e.mem.u8(ECHO_CONSOLE) != 0;
    a.set_result(e, f64::from(u8::from(echo)));
    let read = Read::of(e, &a);
    if !read.ok {
        return handled(true);
    }
    e.mem.set_u8(ECHO_CONSOLE, u8::from(read.int(0, 0) != 0));
    handled(true)
}

/// `ConsoleManager::GetConsoleOutputFilename` (xNVSE): the console's output file's name field.
fn output_filename(e: &mut Engine) -> u32 {
    e.call(CONSOLE_INSTANCE, &args![1u32]).u32() + OUTPUT_FILENAME
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_Console.cpp:
// Cmd_HasConsoleOutputFilename_Execute.
/// 1 while the console writes to an output file (`SCOF`).
fn has_console_output_filename(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let name = output_filename(e);
    a.set_result(e, f64::from(u8::from(e.mem.u8(name) != 0)));
    handled(true)
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_Console.cpp:
// Cmd_GetConsoleOutputFilename_Execute.
/// The console's output file's name, as a string variable (empty without one).
fn get_console_output_filename(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    a.set_result(e, 0.0);
    let name = output_filename(e);
    let text = e.mem.bytes(name, 260);
    let end = text.iter().position(|&b| b == 0).unwrap_or(text.len());
    assign_to_string_var(e, &a, &text[..end]);
    handled(true)
}

#[cfg(test)]
#[path = "console_tests.rs"]
mod tests;
