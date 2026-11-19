//! xNVSE's run-time patch of the game's script command parser (`PatchGameCommandParser`,
//! `Hooks_Editor.cpp` (xNVSE), the part outside the editor build; the compat audit's B14). It
//! changes how every script compiled while the game runs is read: the console's lines, JIP LN's
//! script runner files, `RunBatchScript` and `RunScriptSnippet`, the user function files and
//! `CompileScript`. The game's own scripts come compiled in the plugins and are not read again.
//! - The line reader (`ScriptCompiler::GetNextLine`, `005AF3F0`, replaced whole by
//!   `ParseNextLine`): a line ends at a line feed (a carriage return is a blank, cut at the end),
//!   lines inside open parentheses are one line (a lambda or an expression over several lines),
//!   `;` comments and `/* */` blocks are left out, runs of blanks outside strings are one, and its
//!   own errors (quotes, parentheses, comments, the 512-character limit). Then the line macros
//!   (`HandleLineBufMacros`): `x = 5`, `x += 1` and the other shorthands become `let`, and
//!   `int a, b, c` declares the others too ([`assignment_shorthand`],
//!   [`multiple_declaration`]).
//! - The type keywords: `string_var` and `array_var` as aliases of the compiler's `Long` and
//!   `Float` (their keyword table's alias slots, `0118CBCC` and `0118CBF4`), so a run-time script
//!   can declare them.
//! - A parenthesised argument of a command of the game's format (`InlineExpressionHook` at
//!   `005B1C65`, `StandardCompile`'s word read of each parameter): an xNVSE expression
//!   ([`crate::compiler::handle_parameter_parenthesis`]), for an extender's command (opcode
//!   `0x1400` up: JIP's and the other plugins' commands of the game's format). For the game's own
//!   commands the port does not answer (the game's word read runs, as without the patch), because
//!   their arguments are read at run time by the exe's reader (`005ACCB0`), which cannot read the
//!   expression's `0xFFFF`. xNVSE's entry jump sends every opcode to its own reader; in the port
//!   that waits for the fix row `xnvse.extract_args` (`crate::args`). Until then, three gaps:
//!   - A call of a game command with arguments in the new script language does not run yet, and a
//!     string argument can crash the game. This was so before this seam: the new language's code
//!     generation writes every argument as `0xFFFF` and an expression, which the exe's reader
//!     misreads (a string's length read as -1). Plugins compiled with xNVSE's editor that use
//!     parenthesised arguments on game commands carry the same bytes.
//!   - At an optional parameter of a game command, an argument in parentheses is silently dropped
//!     (the game's word read finds no word, so the line compiles without it and the rest of the
//!     line); at a required one the line is refused ("missing parameter").
//!   - A plugin command's argument in parentheses works on a statement line, not yet inside `set`,
//!     `if` or `elseif`: there the game compiles the command on a line it builds on the stack
//!     (`005b1570`), which leaves the opcode word (`+0x410`) unset, so the seam sees no plugin
//!     command and the argument is refused.
//!
//! Each is a seam in the translated compiler (`engine-fallout-script`'s `tesscriptcompiler.rs`),
//! answered here, so the port's xNVSE reads a run-time script as xNVSE 6.4.9 does on every system.
//!
//! The byte tests (`isspace`, `isalnum`, `\w`, `\s`) are the C locale's: a byte above 0x7F is
//! none of them (xNVSE hands such a `char` to `isspace` and `isalnum` as a negative number, which
//! the C runtime does not define).

use crate::compiler::{self, is_space, ln, sb, show_error, state};

use engine_core::extend::{Owner, SiteSlot};
use engine_core::prelude::*;

/// The game's line reader, xNVSE's in its place.
const LINE_READER: SiteSlot = SiteSlot::new(Owner::Nvse, 0x005a_f3f0);
/// `StandardCompile`'s word read of a parameter (`CALL 005AF5F0` at `005B1C65`).
const PARAMETER_WORD: SiteSlot = SiteSlot::new(Owner::Nvse, 0x005b_1c65);
/// The compiler's keyword table (`0118CB50`, 0x28-byte entries: the name, an alias, the code):
/// the alias slots of `Long` (entry 3, code 0x13) and `Float` (entry 4, code 0x14).
const LONG_ALIAS: u32 = 0x0118_cbcc;
const FLOAT_ALIAS: u32 = 0x0118_cbf4;
/// `ScriptBuffer::textOffset`.
const TEXT_OFFSET: u32 = 4;
/// `ScriptLineBuffer::errorCode`.
const LINE_ERROR: u32 = 0x418;

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Hooks_Editor.cpp: PatchGameCommandParser
// (the run-time build's).
/// The seams and the two keyword aliases (stand-in engines map no keyword table).
pub fn install(e: &mut Engine) {
    e.on_site(LINE_READER, |e, w| {
        let (buf, line) = (word(w, 0), word(w, 1));
        Some(parse_next_line(e, buf, line))
    });
    e.on_site(PARAMETER_WORD, parameter_word);
    if e.mem.is_mapped(LONG_ALIAS) && e.mem.is_mapped(FLOAT_ALIAS) {
        let string_var = crate::table::guest_str(e, b"string_var");
        let array_var = crate::table::guest_str(e, b"array_var");
        e.mem.set_u32(FLOAT_ALIAS, array_var);
        e.mem.set_u32(LONG_ALIAS, string_var);
    }
}

fn word(w: &[u32], i: usize) -> u32 {
    w.get(i).copied().unwrap_or(0)
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Hooks_Editor.cpp: InlineExpressionHook.
/// `005B1C65`, words (the line, the script buffer, the parameters, the index): a parameter in
/// parentheses is compiled as an expression (answer 1: the next parameter follows) or was wrong
/// (answer 0: the compile of the line fails); anything else is the game's word (no answer). Only
/// for an extender's opcode (the line's `+0x410`, `0x1400` up); for the game's own commands no
/// answer (the module's note: their run-time reader is the exe's).
fn parameter_word(e: &mut Engine, w: &[u32]) -> Option<u32> {
    let (line, buf, params, index) = (word(w, 0), word(w, 1), word(w, 2), word(w, 3));
    if e.mem.u32(line + ln::OPCODE) < crate::args::FIRST_EXTENDER_OPCODE {
        return None;
    }
    match compiler::handle_parameter_parenthesis(e, line, buf, params, index) {
        compiler::Paren::NoParam => None,
        compiler::Paren::Success => Some(1),
        compiler::Paren::SyntaxError => Some(0),
    }
}

/// `ParseNextLine`'s errors (`lineError`): the next line's count dropped, the buffer's line made
/// this one's, the message (`ShowCompilerError` with the line), error 1; nothing read.
fn line_error(e: &mut Engine, buf: u32, line: u32, message: &[u8]) -> u32 {
    state(e).next_line_increment = 0;
    let number = e.mem.u32(line + ln::NUMBER);
    e.mem.set_u32(buf + sb::CUR_LINE, number);
    compiler::compiler_error(e, line, message);
    e.mem.set_u32(line + LINE_ERROR, 1);
    0
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Hooks_Editor.cpp: ParseNextLine.
/// `ParseNextLine(scriptBuf, lineBuf)`: the next line of the buffer's text (from its offset) into
/// the line's text; the answer is the bytes read (0 at the text's end or after an error).
/// - The line number first takes the lines the last line's parentheses held, then each line feed
///   among the leading blanks; a blank line or one with only a comment counts one more.
/// - A line ends at a line feed (or the text's end) outside parentheses; inside them a line feed
///   is kept in the text and counted for the next line's number.
/// - `/*` (outside a string) to `*/` is left out, its line feeds counted; `;` (anywhere, a string
///   too, as xNVSE reads it) leaves out the rest of the line.
/// - A blank after a blank is left out outside strings; blanks before the first other byte too;
///   trailing blanks (a carriage return among them) are cut.
/// - Errors: a comment, a string or parentheses left open at the text's end, a `)` with none open,
///   and a line of 512 characters ("Max script line length (512 characters) exceeded.", or
///   "Max script expression length inside parenthesis (512 characters) exceeded." inside
///   parentheses; error 16): the line is not read.
/// - The line macros then run on the line ([`handle_line_macros`]); one that fails ends the read
///   (0).
pub fn parse_next_line(e: &mut Engine, buf: u32, line: u32) -> u32 {
    e.mem.set_u32(line + ln::TEXT_LEN, 0);
    e.mem.write(line + ln::TEXT, &[0; ln::SIZE as usize]);
    let start = e
        .mem
        .u32(buf + sb::TEXT)
        .wrapping_add(e.mem.u32(buf + TEXT_OFFSET));
    let mut cur = start;
    let mut brackets: i32 = 0;
    let mut captured = false;
    let mut newlines_in_parentheses = 0u32;
    let mut in_string = false;
    let mut in_comment = false;
    let increment = std::mem::take(&mut state(e).next_line_increment);
    let number = e.mem.u32(line + ln::NUMBER).wrapping_add(increment);
    e.mem.set_u32(line + ln::NUMBER, number);
    let count_line = |e: &mut Engine| {
        let n = e.mem.u32(line + ln::NUMBER).wrapping_add(1);
        e.mem.set_u32(line + ln::NUMBER, n);
    };
    while is_space(e.mem.u8(cur)) {
        if e.mem.u8(cur) == b'\n' {
            count_line(e);
        }
        cur = cur.wrapping_add(1);
    }
    let mut last = 0u8;
    loop {
        if e.mem.u8(cur) != 0 {
            let two = [e.mem.u8(cur), e.mem.u8(cur.wrapping_add(1))];
            if &two == b"/*" && !in_string && !in_comment {
                in_comment = true;
                cur = cur.wrapping_add(2);
                continue;
            }
            if &two == b"*/" && !in_string && in_comment {
                in_comment = false;
                cur = cur.wrapping_add(2);
                continue;
            }
        }
        let c = e.mem.u8(cur);
        cur = cur.wrapping_add(1);
        if c == 0 {
            if in_comment {
                return line_error(
                    e,
                    buf,
                    line,
                    b"Mismatched comment block (missing '*/' for a present '/*')",
                );
            }
            if in_string {
                return line_error(
                    e,
                    buf,
                    line,
                    b"Mismatched quotes. A string literal was not closed.",
                );
            }
            if brackets != 0 {
                return line_error(
                    e,
                    buf,
                    line,
                    b"Mismatched parentheses. Parentheses were not closed.",
                );
            }
        }
        if in_comment {
            if c == b'\n' {
                count_line(e);
            }
            continue;
        }
        match c {
            b'(' => {
                if !in_string {
                    brackets += 1;
                }
            }
            b')' => {
                if !in_string {
                    brackets -= 1;
                    if brackets < 0 {
                        return line_error(e, buf, line, b"Mismatched parenthesis");
                    }
                }
            }
            b'"' => in_string = !in_string,
            0 | b'\n' => {
                if c == 0 {
                    cur = cur.wrapping_sub(1);
                    if !captured {
                        return 0;
                    }
                }
                if brackets == 0 && captured {
                    let mut len = e.mem.u32(line + ln::TEXT_LEN);
                    while len > 0 && is_space(e.mem.u8(line + ln::TEXT + len - 1)) {
                        len -= 1;
                        e.mem.set_u8(line + ln::TEXT + len, 0);
                    }
                    e.mem.set_u32(line + ln::TEXT_LEN, len);
                    // `paramText[paramTextLen] = 0`: at 512 characters that is the length's own
                    // low byte, as in xNVSE.
                    e.mem.set_u8(line + ln::TEXT + len, 0);
                    state(e).next_line_increment = newlines_in_parentheses;
                    if !handle_line_macros(e, line, buf) {
                        return 0;
                    }
                    return cur.wrapping_sub(start);
                }
                if brackets != 0 {
                    newlines_in_parentheses += 1;
                } else if !captured {
                    count_line(e);
                }
            }
            b';' => {
                while e.mem.u8(cur) != 0 && e.mem.u8(cur) != b'\n' {
                    cur = cur.wrapping_add(1);
                }
                continue;
            }
            _ => {
                if is_space(last) && is_space(c) && !in_string {
                    continue;
                }
            }
        }
        let len = e.mem.u32(line + ln::TEXT_LEN);
        if len >= ln::SIZE {
            let message: &[u8] = if brackets != 0 {
                b"Max script expression length inside parenthesis (512 characters) exceeded."
            } else {
                b"Max script line length (512 characters) exceeded."
            };
            compiler::compiler_error(e, line, message);
            e.mem.set_u32(line + LINE_ERROR, 16);
            return 0;
        }
        if !is_space(c) {
            captured = true;
        }
        if captured {
            e.mem.set_u8(line + ln::TEXT + len, c);
            e.mem.set_u32(line + ln::TEXT_LEN, len + 1);
        }
        last = c;
    }
}

/// `ScriptLineMacro::MacroResult`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum MacroResult {
    Error,
    Skipped,
    Applied,
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Hooks_Editor.cpp: HandleLineBufMacros,
// g_lineMacros; ScriptUtils.cpp: ScriptLineMacro::EvalMacro.
/// `HandleLineBufMacros`: the assignment shorthand, then the multiple declaration, each on the
/// line's text from its offset; false when one failed (its message given).
fn handle_line_macros(e: &mut Engine, line: u32, buf: u32) -> bool {
    let shorthand = eval_macro(e, line, |_, text| assignment_shorthand(text));
    if shorthand == MacroResult::Error {
        return false;
    }
    let declaration = eval_macro(e, line, |e, text| {
        multiple_declaration(e, text, buf, line);
        None
    });
    declaration != MacroResult::Error
}

/// `EvalMacro`: `f` given the text from the line's offset; a new text replaces it (with the
/// length) unless it does not fit the 512 bytes from the offset ("Line length limit reached, could
/// not translate macro.", an error). Where the new text is exactly as long as that room, xNVSE's
/// `strcpy_s` has no room for the end and the game closes; here that is the same error.
fn eval_macro(
    e: &mut Engine,
    line: u32,
    f: impl FnOnce(&mut Engine, &[u8]) -> Option<Vec<u8>>,
) -> MacroResult {
    let offset = e.mem.u32(line + ln::OFFSET);
    let start = line + ln::TEXT + offset.min(ln::SIZE);
    // The text to its end byte: at 512 characters that is the length's low byte, past the text.
    let mut text = Vec::new();
    let mut at = start;
    while at < line + ln::STRUCT && e.mem.u8(at) != 0 {
        text.push(e.mem.u8(at));
        at += 1;
    }
    let Some(new) = f(e, &text) else {
        return MacroResult::Skipped;
    };
    let room = ln::SIZE.saturating_sub(offset) as usize;
    if new.len() >= room {
        show_error(e, b"Line length limit reached, could not translate macro.");
        return MacroResult::Error;
    }
    e.mem.write(start, &new);
    e.mem.set_u8(start + new.len() as u32, 0);
    e.mem
        .set_u32(line + ln::TEXT_LEN, new.len() as u32 + offset);
    MacroResult::Applied
}

/// `g_validVariableTypeNames`, in xNVSE's order.
const TYPE_NAMES: [&[u8]; 8] = [
    b"float",
    b"int",
    b"string_var",
    b"array_var",
    b"ref",
    b"short",
    b"long",
    b"reference",
];

/// The shorthands in xNVSE's order: the operator written and the one `let` gets.
const SHORTHANDS: [(&[u8], &[u8]); 9] = [
    (b"=", b":="),
    (b":=", b":="),
    (b"+=", b"+="),
    (b"-=", b"-="),
    (b"*=", b"*="),
    (b"/=", b"/="),
    (b"|=", b"|="),
    (b"&=", b"&="),
    (b"%=", b"%="),
];

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Hooks_Editor.cpp: g_lineMacros (the
// assignment shorthand, MacroType::AssignmentShortHand).
/// The assignment shorthand: a line starting with a type name and a blank loses them (and its
/// blanks) and keeps the type for `let`; then for each shorthand in order, a line matching
/// `^(\w+(\.\w+)*(\[.*\])*)\s*` + the operator + `([^=][\S\s]*)` becomes
/// `let <type> <target> <op> <rest>` (the target the first group, the rest the last, from the
/// byte after the operator; none when no shorthand matches).
pub(crate) fn assignment_shorthand(text: &[u8]) -> Option<Vec<u8>> {
    let mut line = text.to_vec();
    let mut declaration = Vec::new();
    for name in TYPE_NAMES {
        if line.len() >= name.len()
            && line[..name.len()].eq_ignore_ascii_case(name)
            && line.get(name.len()).is_some_and(|&c| is_space(c))
        {
            declaration = name.to_vec();
            line.drain(..name.len());
            declaration.push(b' ');
            let blanks = line.iter().take_while(|&&c| is_space(c)).count();
            line.drain(..blanks);
            break;
        }
    }
    for (written, real) in SHORTHANDS {
        if let Some((target_end, rest)) = match_assignment(&line, written) {
            let mut out = b"let ".to_vec();
            out.extend_from_slice(&declaration);
            out.extend_from_slice(&line[..target_end]);
            out.push(b' ');
            out.extend_from_slice(real);
            out.push(b' ');
            out.extend_from_slice(&line[rest..]);
            return Some(out);
        }
    }
    None
}

/// `\w`: a letter, a digit or `_` (the C locale's).
fn is_word(c: u8) -> bool {
    c.is_ascii_alphanumeric() || c == b'_'
}

/// The assignment pattern `^(\w+(\.\w+)*(\[.*\])*)\s*OP([^=][\S\s]*)` matched as xNVSE's
/// `std::regex_search` (ECMAScript, `match_continuous`) matches it: the end of the first group and
/// the start of the last, or `None`.
///
/// The identifier part is the longest run (a shorter one leaves a word byte next, which no
/// bracket, blank or operator can follow). The brackets are tried as the regex backtracks: from
/// a `[`, each `]` before the next line break, the farthest first, each followed again by more
/// brackets before stopping there; zero brackets last. After them only the longest run of blanks
/// can be followed by the operator (none starts with a blank), and the operator must have a byte
/// after it that is not `=`. `.` takes no line feed or carriage return (ECMAScript's line
/// terminators).
fn match_assignment(line: &[u8], op: &[u8]) -> Option<(usize, usize)> {
    let n = line.len();
    let mut i = 0;
    while i < n && is_word(line[i]) {
        i += 1;
    }
    if i == 0 {
        return None;
    }
    while i + 1 < n && line[i] == b'.' && is_word(line[i + 1]) {
        i += 1;
        while i < n && is_word(line[i]) {
            i += 1;
        }
    }
    let mut failed = vec![false; n + 1];
    brackets_then_operator(line, i, op, &mut failed)
}

/// The brackets from `p` (see [`match_assignment`]), `failed` holding the starts already tried
/// in vain (the rest depends only on the start).
fn brackets_then_operator(
    line: &[u8],
    p: usize,
    op: &[u8],
    failed: &mut [bool],
) -> Option<(usize, usize)> {
    if failed[p] {
        return None;
    }
    let n = line.len();
    if p < n && line[p] == b'[' {
        let mut limit = p + 1;
        while limit < n && line[limit] != b'\n' && line[limit] != b'\r' {
            limit += 1;
        }
        for j in (p + 1..limit).rev() {
            if line[j] == b']' {
                if let Some(found) = brackets_then_operator(line, j + 1, op, failed) {
                    return Some(found);
                }
            }
        }
    }
    let mut k = p;
    while k < n && is_space(line[k]) {
        k += 1;
    }
    let after = k + op.len();
    if line[k..].starts_with(op) && after < n && line[after] != b'=' {
        return Some((p, after));
    }
    failed[p] = true;
    None
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Hooks_Editor.cpp: g_lineMacros (the
// multiple declaration, MacroType::MultipleVariableDeclaration).
/// `int a, b, c` and the other type names (the first one the line starts with, in xNVSE's order,
/// any case, a blank after it): the rest split at commas, each part's blanks taken out; with more
/// than one part and only letters, digits and `_` in each, the second and later parts are made
/// variables of the compile's current script now (`CreateVariable`, its errors given). xNVSE
/// leaves the line itself as it was (its macro answers "not applied"), so the game's compiler
/// then reads the declaration of the first name.
fn multiple_declaration(e: &mut Engine, text: &[u8], buf: u32, line: u32) {
    let Some(name) = TYPE_NAMES.iter().find(|name| {
        text.len() > name.len()
            && text[..name.len()].eq_ignore_ascii_case(name)
            && text[name.len()] == b' '
    }) else {
        return;
    };
    let rest = &text[name.len() + 1..];
    let parts: Vec<Vec<u8>> = rest
        .split(|&c| c == b',')
        .map(|part| part.iter().copied().filter(|&c| !is_space(c)).collect())
        .collect();
    if parts.len() <= 1 {
        return;
    }
    if !parts.iter().all(|part| part.iter().all(|&c| is_word(c))) {
        return;
    }
    let script = state(e).scripts.last().copied().unwrap_or(0);
    let var_type = compiler::variable_type_by_name(e, name);
    for part in &parts[1..] {
        compiler::create_variable(e, script, buf, part, var_type, line);
    }
}

#[cfg(test)]
#[path = "line_reader_tests.rs"]
mod tests;
