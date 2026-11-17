//! `functions_jip/jip_fn_string.h` (JIP LN NVSE): `GetStringHash` (JIP's hash of a text, with or
//! without case, and its split mode, which writes `GetStringHash.txt` through the core's seam
//! `engine_core::mod_files`: in the port's own folder for mods' files, never the game folder).
//! 1 of its commands; the others come later.

use super::jip_fn_ui::next_token;
use super::Handlers;
use crate::containers::{str_hash_ci, str_hash_cs};
use engine_core::prelude::*;
use nvse::commands::Read;
use nvse::{handled, CommandArgs};

#[rustfmt::skip]
pub static COMMANDS: Handlers = &[
    (0x293f, get_string_hash),
];

/// JIP's string argument buffer (`GetStrArgBuffer`, 0x40000 bytes).
const STR_ARG_BUFFER: usize = 0x4_0000;

/// `GetStringHash.txt`: the file the split mode writes, in the game's folder for JIP (here the
/// port's folder for mods' files).
const STRING_HASH_FILE: &[u8] = b"GetStringHash.txt";

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_string.h:
// Cmd_GetStringHash_Execute.
/// JIP's hash of the text (`StrHashCI`, or `StrHashCS` when `case` is set) as a signed number,
/// printed in the console when it is open; with `split`, each `|`-separated part (runs of `|`
/// one separator) and its hash written to `GetStringHash.txt` as `"part"`, the number and a
/// blank line, and no result. Nothing for an empty text.
fn get_string_hash(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if !read.ok {
        return handled(true);
    }
    let text = super::buffer_arg(e, &read, 0, STR_ARG_BUFFER, "GetStringHash");
    if text.is_empty() {
        return handled(true);
    }
    let hash: fn(&[u8]) -> u32 = if read.int(2, 0) == 0 {
        str_hash_ci
    } else {
        str_hash_cs
    };
    if read.int(1, 0) == 0 {
        a.set_result(e, f64::from(hash(&text) as i32));
        crate::console::print_result(e, &a);
        return handled(true);
    }
    let mut out = Vec::new();
    let mut rest: &[u8] = &text;
    loop {
        let (part, next) = next_token(rest, b'|');
        out.push(b'"');
        out.extend_from_slice(part);
        out.extend_from_slice(format!("\"\n{}\n\n", hash(part) as i32).as_bytes());
        if next.is_empty() {
            break;
        }
        rest = next;
    }
    engine_core::mod_files::write(e, STRING_HASH_FILE, &out);
    handled(true)
}

#[cfg(test)]
#[path = "jip_fn_string_tests.rs"]
mod tests;
