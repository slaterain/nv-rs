//! `functions_jip/jip_fn_string.h` (JIP LN NVSE): `sv_RegexMatch`, `sv_RegexSearch` and
//! `sv_RegexReplace`, MSVC's `std::regex` on a string variable's text ([`crate::ecma_regex`], the
//! same engine JIP's DLL carries). `GetStringHash` is in `jip_fn_string.rs`.
//!
//! Where the original stops the game, the port goes on with the row `NV_NVSE_LIST_GUARD` (on in
//! the fixes preset; docs/FIXES.md):
//! - a pattern `std::regex` refuses (an unclosed group, a bad escape, `a**` ...) throws out of the
//!   command, and so does a match past MSVC's limits (1000 nested steps or 10,000,000 steps from
//!   one start place, a runaway pattern such as `(a*)*b` on a long text): nothing catches it and the
//!   game stops. With the row the command answers as for no match: `sv_RegexMatch` 0,
//!   `sv_RegexSearch` the matches found before it, `sv_RegexReplace` an empty string;
//! - `sv_RegexSearch` searches the text after each match as a new text, starting where the match
//!   ended, so a match of the empty text at the start of the text searched (`a*` on `b`, `$` once
//!   the text is used up) is found there again and again: the loop never ends and the array grows
//!   until the game runs out of memory. (An empty match further in moves the start on and is
//!   JIP's answer: `\B` on `abc` gives two.) With the row, the next search after that empty match
//!   starts one byte later (as JavaScript's global search does), the array takes it once (not a
//!   second time when it is the place an empty match further in already gave), and at the end of
//!   the text the search ends.
//!
//! MSVC's runaway limit is per start place (each search tries every place of its text, each with
//! its own 10,000,000 steps), so with the row a slow pattern on a long text can still hold the game
//! for a long time: only a start place that alone passes the limit stops the command.
//!
//! Off (vanilla) the port stops the game in those places with a message, as the original does.
//!
//! Threads: on the script's thread, as JIP's; the pattern is compiled at each call, as JIP does.

use super::arrays::{append_element, create_array, Element};
use super::jip_fn_ui::next_token;
use super::Handlers;
use crate::ecma_regex::{self, Regex};
use engine_core::prelude::*;
use nvse::commands::array::{fix_on, LIST_GUARD};
use nvse::{handled, vars, CommandArgs};

#[rustfmt::skip]
pub static COMMANDS: Handlers = &[
    (0x2795, sv_regex_match),
    (0x2796, sv_regex_search),
    (0x2797, sv_regex_replace),
];

/// `kParams_OneInt_OneFormatString`'s count (the integer, the format string and its 20).
const REGEX_PARAMS: u32 = 22;
/// JIP's `rgxStr[0x80]` and its string argument buffer (`GetStrArgBuffer`, 0x40000 bytes).
const RGX_BUFFER: usize = 0x80;
const STR_ARG_BUFFER: usize = 0x4_0000;

/// The string variable's text up to its first 0 (JIP reads it as a C string); `None` when there
/// is no such variable.
fn source(e: &Engine, id: u32) -> Option<Vec<u8>> {
    vars::string(e, id).map(|mut text| {
        if let Some(end) = text.iter().position(|&b| b == 0) {
            text.truncate(end);
        }
        text
    })
}

/// `std::regex`'s error, which the original throws out of the command: with the row the caller
/// goes on as for no match, without it the port stops the game.
fn regex_error(e: &Engine, command: &str, what: ecma_regex::Error) {
    if !fix_on(e, LIST_GUARD) {
        panic!(
            "{command}: std::regex throws regex_error ({what:?}) out of JIP's command and the game \
             stops: the port stops here (NV_NVSE_LIST_GUARD goes on as for no match)"
        );
    }
}

/// The format string's text and the string variable's id (the integer before it), or `None` when
/// the arguments do not read.
fn args(e: &mut Engine, a: &CommandArgs, size: usize, command: &str) -> Option<(Vec<u8>, u32)> {
    let f = nvse::commands::string::format_string_args_at(e, a, 1, REGEX_PARAMS);
    if !f.ok {
        return None;
    }
    let id = f
        .leading
        .first()
        .and_then(nvse::args::Arg::int)
        .unwrap_or(0) as u32;
    Some((super::formatted_in_buffer(e, &f.text, size, command), id))
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_string.h:
// Cmd_sv_RegexMatch_Execute.
/// `sv_RegexMatch strID "pattern"`: 1 when the whole text of the string variable matches
/// (`std::regex_match`).
fn sv_regex_match(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let Some((pattern, id)) = args(e, &a, RGX_BUFFER, "sv_RegexMatch") else {
        return handled(true);
    };
    let Some(text) = source(e, id) else {
        return handled(true);
    };
    let answer = Regex::new(&pattern).and_then(|re| ecma_regex::full_match(&re, &text));
    match answer {
        Ok(true) => a.set_result(e, 1.0),
        Ok(false) => {}
        Err(what) => regex_error(e, "sv_RegexMatch", what),
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_string.h:
// Cmd_sv_RegexSearch_Execute.
/// `sv_RegexSearch strID "pattern"`: a new array (always, the result even when nothing reads)
/// with each match in turn: a search of the text, then of the text after the match, and so on
/// (each search sees its text as a new one, so `^` matches at its start).
fn sv_regex_search(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let array = create_array(e, a.script, &[]);
    a.set_result(e, f64::from(array));
    let Some((pattern, id)) = args(e, &a, RGX_BUFFER, "sv_RegexSearch") else {
        return handled(true);
    };
    let Some(text) = source(e, id).filter(|t| !t.is_empty()) else {
        return handled(true);
    };
    let re = match Regex::new(&pattern) {
        Ok(re) => re,
        Err(what) => {
            regex_error(e, "sv_RegexSearch", what);
            return handled(true);
        }
    };
    let mut pos = 0;
    // Where the last empty match the array took is (from the text's start): with the row, the
    // same place found again at the start of the next text is not taken twice.
    let mut last_empty = None;
    loop {
        let rest = &text[pos..];
        let (start, end) = match ecma_regex::search(&re, rest, 0, 0) {
            Ok(Some(m)) => m.whole(),
            Ok(None) => break,
            Err(what) => {
                regex_error(e, "sv_RegexSearch", what);
                break;
            }
        };
        if end == 0 {
            // An empty match at the start of the text searched: JIP's next text starts at the
            // match's end, the same place, so the same search finds it again for ever (an empty
            // match further in moves the start on, as below, and ends).
            if !fix_on(e, LIST_GUARD) {
                panic!(
                    "sv_RegexSearch: an empty match at the start of the text searched is found \
                     again there for ever (JIP's loop never ends): the port stops here \
                     (NV_NVSE_LIST_GUARD searches on one byte later)"
                );
            }
            if last_empty != Some(pos) {
                append_element(e, array, &Element::Str(Vec::new()));
                last_empty = Some(pos);
            }
            if pos == text.len() {
                break;
            }
            pos += 1;
            continue;
        }
        append_element(e, array, &Element::Str(rest[start..end].to_vec()));
        if start == end {
            last_empty = Some(pos + end);
        }
        // JIP's next start: the text's end less the suffix's length, the match's end.
        pos += end;
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_string.h:
// Cmd_sv_RegexReplace_Execute.
/// `sv_RegexReplace strID "format|pattern"`: the string variable's text with every match of the
/// pattern (the part after the first run of `|`) replaced by the format (the part before it, with
/// `std::regex_replace`'s `$` parts); an empty string when the variable is missing or no pattern
/// follows a `|`.
fn sv_regex_replace(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let replaced = args(e, &a, STR_ARG_BUFFER, "sv_RegexReplace").and_then(|(buffer, id)| {
        let text = source(e, id)?;
        let (fmt, pattern) = next_token(&buffer, b'|');
        if pattern.is_empty() {
            return None;
        }
        let answer = Regex::new(pattern).and_then(|re| ecma_regex::replace(&re, &text, fmt));
        match answer {
            Ok(out) => Some(out),
            Err(what) => {
                regex_error(e, "sv_RegexReplace", what);
                None
            }
        }
    });
    super::assign_string(e, &a, replaced);
    handled(true)
}

#[cfg(test)]
#[path = "jip_fn_string_regex_tests.rs"]
mod tests;
