//! xNVSE's override argument path (`ExtractArgsOverride` and `CompilerOverride`,
//! `Hooks_Script.cpp` (xNVSE)).
//!
//! The GECK, with xNVSE loaded, compiles a block whose name starts with `_` (`Begin _GameMode`)
//! through xNVSE's expression parser: every command line in it gets the argument count word
//! [`OVERRIDE_COMMAND`] (`0xFFFF`), a block's own parameters [`OVERRIDE_BLOCK_TYPE`] (`0xFFFE`),
//! then a count byte and one expression per argument (`ExpressionParser::ParseArgs`). The
//! compiler also writes `@PushExecutionContext` after the `Begin` line and `@PopExecutionContext`
//! before the `End` line: they keep the calling reference and its container on a stack, per
//! thread, for the commands of the block, whose reader is not handed them. xNVSE's runtime
//! compiler (lambdas, `CompileScript`) never writes this form: it writes the per-argument inline
//! expression ([`crate::expr::inline_argument`]).
//!
//! Reading: every argument is evaluated first, then each is converted to its parameter's type
//! (`ExtractDefaultArgs`, `ConvertDefaultArg`); a format string's arguments come from the
//! evaluated arguments (`OverriddenScriptFormatStringArgs`).

use crate::args::Arg;
use crate::expr::{self, Eval, Tok};
use engine_core::prelude::*;
use engine_core::thread_host::current_id;

/// `CompilerOverride::kOverride_Command`: an ordinary command inside a block.
pub const OVERRIDE_COMMAND: u16 = 0xffff;
/// `CompilerOverride::kOverride_BlockType`: a block type (`GameMode`'s parameters).
pub const OVERRIDE_BLOCK_TYPE: u16 = 0xfffe;

/// One entry of the stack (`ExecutingScriptContext` (xNVSE)).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ExecContext {
    pub calling_ref: u32,
    pub container_ref: u32,
    /// The opcode of the command that pushed it.
    pub opcode: u16,
    /// The game thread that pushed it.
    pub thread: u32,
}

/// The stack, shared by every thread as xNVSE's (each entry names its thread), behind a lock as
/// xNVSE's critical section.
#[derive(Default)]
pub struct State {
    stack: std::sync::Mutex<Vec<ExecContext>>,
}

/// Makes the stack's slot at the extender's install, before the game's threads start, so every
/// thread's engine shares the one stack (as [`crate::events::install`] does its lock-free state).
pub fn install(e: &mut Engine) {
    e.ext_free::<State>();
}

fn stack(e: &mut Engine) -> std::sync::MutexGuard<'_, Vec<ExecContext>> {
    e.ext_free::<State>()
        .stack
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Hooks_Script.cpp:
// ExtractArgsOverride::PushContext.
/// The calling reference and container, and the opcode of the command at the offset word (the
/// word 4 bytes before its arguments), pushed for this thread.
pub fn push_context(e: &mut Engine, this_obj: u32, container: u32, data: u32, offset_ptr: u32) {
    let offset = e.mem.u32(offset_ptr);
    let opcode = e.mem.u16(data.wrapping_add(offset).wrapping_sub(4));
    let thread = current_id(e);
    stack(e).push(ExecContext {
        calling_ref: this_obj,
        container_ref: container,
        opcode,
        thread,
    });
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Hooks_Script.cpp:
// ExtractArgsOverride::PopContext.
/// This thread's innermost entry removed; false when it has none.
pub fn pop_context(e: &mut Engine) -> bool {
    let thread = current_id(e);
    let mut s = stack(e);
    match s.iter().rposition(|c| c.thread == thread) {
        Some(i) => {
            s.remove(i);
            true
        }
        None => false,
    }
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Hooks_Script.cpp:
// ExtractArgsOverride::GetCurrentContext.
/// This thread's innermost entry.
pub fn current_context(e: &mut Engine) -> Option<ExecContext> {
    let thread = current_id(e);
    stack(e).iter().rev().find(|c| c.thread == thread).copied()
}

/// The entries on the stack, every thread's (tests).
pub fn depth(e: &mut Engine) -> usize {
    stack(e).len()
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), ScriptUtils.cpp: ShowError (the
// `RUNTIME` one; the message box is the editor build's).
/// `g_ErrOut.Show`: `Console_Print(msg)`, the text being the format, and the game carries on.
/// xNVSE also writes it to `nvse.log` (`_MESSAGE`); the port keeps no such log, as for every other
/// error it prints ([`crate::errors::show`]).
fn show_error(e: &mut Engine, text: &str) {
    if crate::Nvse::prints(e) {
        crate::commands::console_print(e, text, &[]);
    }
}

/// The calling reference and container a reader in `mode` gets: the current context's for a
/// command ([`OVERRIDE_COMMAND`]), none for a block type; `None` (after xNVSE's console error) when a
/// command has no context.
fn references(e: &mut Engine, mode: u16, what: &str) -> Option<(u32, u32)> {
    if mode != OVERRIDE_COMMAND {
        return Some((0, 0));
    }
    match current_context(e) {
        Some(c) => Some((c.calling_ref, c.container_ref)),
        None => {
            show_error(
                e,
                &format!("ERROR: Could not get execution context in {what}"),
            );
            None
        }
    }
}

/// An evaluator over the arguments at the offset word (`ExpressionEvaluator`'s constructor with
/// no result).
fn evaluator(
    e: &Engine,
    params: u32,
    data: u32,
    offset_ptr: u32,
    script: u32,
    event_list: u32,
    refs: (u32, u32),
) -> Eval {
    let offset = e.mem.u32(offset_ptr);
    Eval {
        params,
        data,
        this_obj: refs.0,
        containing_obj: refs.1,
        script,
        event_list,
        result: 0,
        offset_ptr,
        at: data.wrapping_add(offset),
        errors: Vec::new(),
        suppress_errors: false,
        base_offset: Some(offset.wrapping_sub(4)),
        inline: false,
    }
}

/// The type of parameter `i` (`ParamInfo`, 12 bytes each: the type at `+4`).
fn param_kind(e: &Engine, params: u32, i: u32) -> u32 {
    e.mem.u32(params.wrapping_add(12 * i + 4))
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Hooks_Script.cpp:
// ExtractArgsOverride::ExtractArgs, and ScriptUtils.cpp: ExpressionEvaluator::ExtractDefaultArgs.
/// The arguments of a line compiled in `mode` (the count word, already passed: the offset word is
/// at the count byte): all evaluated, then each converted to its parameter's type
/// (`convert_forms` as xNVSE's `bConvertTESForms`). The offset word moves past each expression as
/// the evaluator reads it. `None` when one fails, or a command has no context.
#[allow(clippy::too_many_arguments)]
pub fn arguments(
    e: &mut Engine,
    params: u32,
    data: u32,
    offset_ptr: u32,
    script: u32,
    event_list: u32,
    mode: u16,
    convert_forms: bool,
) -> Option<Vec<Arg>> {
    let refs = references(e, mode, "ExtractArgsOverride::ExtractArgs")?;
    let mut ev = evaluator(e, params, data, offset_ptr, script, event_list, refs);
    let toks = expr::with_evaluator(e, |e| ev.extract_args(e));
    let out = toks.and_then(|toks| convert_all(e, &ev, params, 0, &toks, script, convert_forms));
    expr::report(e, &ev);
    out
}

/// `toks` converted to the parameters from `first_param` on.
fn convert_all(
    e: &mut Engine,
    ev: &Eval,
    params: u32,
    first_param: u32,
    toks: &[Tok],
    script: u32,
    convert_forms: bool,
) -> Option<Vec<Arg>> {
    let mut out = Vec::with_capacity(toks.len());
    for (i, tok) in toks.iter().enumerate() {
        let kind = param_kind(e, params, first_param + i as u32);
        out.push(expr::convert_default_arg(
            e,
            ev,
            kind,
            tok,
            script,
            convert_forms,
        )?);
    }
    Some(out)
}

/// What a format string read through the override gives (`ExtractFormatStringArgs`).
pub struct Formatted {
    /// The arguments before the format string.
    pub leading: Vec<Arg>,
    pub text: Vec<u8>,
    /// The ordinary parameters after the format string's arguments.
    pub trailing: Vec<Arg>,
}

/// The format string's arguments as the evaluated arguments
/// (`OverriddenScriptFormatStringArgs`): `cur` the next one.
pub struct TokenArgs<'a> {
    pub ev: &'a Eval,
    pub toks: &'a [Tok],
    pub cur: usize,
}

impl TokenArgs<'_> {
    // Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), ScriptUtils.cpp:
    // OverriddenScriptFormatStringArgs::Arg (kArgType_Float).
    pub fn float(&mut self, e: &mut Engine) -> Option<f64> {
        let tok = self.toks.get(self.cur)?;
        let v = self.ev.get_number(e, tok);
        self.cur += 1;
        Some(v)
    }

    // Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), ScriptUtils.cpp:
    // OverriddenScriptFormatStringArgs::Arg (kArgType_Form).
    pub fn form(&mut self, e: &mut Engine) -> Option<u32> {
        let tok = self.toks.get(self.cur)?;
        let v = self.ev.get_form(e, tok);
        self.cur += 1;
        Some(v)
    }

    // Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), ScriptUtils.cpp:
    // OverriddenScriptFormatStringArgs::SkipArgs and HasMoreArgs.
    pub fn skip(&mut self, count: u32) {
        self.cur += count as usize;
    }

    pub fn has_more(&self) -> bool {
        self.cur < self.toks.len()
    }
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Hooks_Script.cpp:
// ExtractArgsOverride::ExtractFormattedString, and ScriptUtils.cpp:
// ExpressionEvaluator::ExtractFormatStringArgs.
/// A format string at parameter `fmt_pos` of a line compiled with the override (the offset word
/// at the count word): the context first (a command without one fails), the count word passed,
/// every argument evaluated, those before the format string converted (forms not cast), the
/// format string made by `format` from the arguments after it, then any arguments left converted
/// to the parameters after the format string's 20.
#[allow(clippy::too_many_arguments)]
pub fn formatted(
    e: &mut Engine,
    params: u32,
    data: u32,
    offset_ptr: u32,
    script: u32,
    event_list: u32,
    fmt_pos: u32,
    format: impl FnOnce(&mut Engine, &[u8], &mut TokenArgs) -> Option<Vec<u8>>,
) -> Option<Formatted> {
    let refs = references(e, OVERRIDE_COMMAND, "ExtractFormattedString()")?;
    let offset = e.mem.u32(offset_ptr);
    e.mem.set_u32(offset_ptr, offset.wrapping_add(2));
    let mut ev = evaluator(e, params, data, offset_ptr, script, event_list, refs);
    let toks = expr::with_evaluator(e, |e| ev.extract_args(e));
    let out = toks.and_then(|toks| {
        if (toks.len() as u32) < fmt_pos {
            return None;
        }
        let leading = convert_all(e, &ev, params, 0, &toks[..fmt_pos as usize], script, false)?;
        // A missing format token: xNVSE reads through a null there. Any other token gives its
        // `GetString`, empty for one that is no string.
        let fmt_tok = toks.get(fmt_pos as usize)?;
        let fmt = ev.get_string(e, fmt_tok);
        let mut args = TokenArgs {
            ev: &ev,
            toks: &toks,
            cur: fmt_pos as usize + 1,
        };
        let text = format(e, &fmt, &mut args)?;
        let rest = args.cur;
        let trailing = if rest < toks.len() {
            convert_all(e, &ev, params, fmt_pos + 21, &toks[rest..], script, false)?
        } else {
            Vec::new()
        };
        Some(Formatted {
            leading,
            text,
            trailing,
        })
    });
    expr::report(e, &ev);
    out
}
