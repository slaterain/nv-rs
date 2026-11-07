//! xNVSE's scripting core (`Commands_Scripting.cpp`, `Loops.cpp` (xNVSE)): `Let`, `eval` and
//! `testexpr` on the expression evaluator (I-6), and the loops and labels (I-7): `while`,
//! `ForEach`, `loop`, `continue`, `break`, `Label`, `Goto`.
//!
//! The loops move the script runner as xNVSE does: through the running line's `length` word
//! (`GetCalculatedOpLength`, here the line record of [`crate::context`]), and they save and
//! restore the runner's if/else stack (`ifStackDepth` `+0x20`, `ifStack` `+0x24`, ten deep;
//! `LoopManager::RestoreStack`). The loop stack is per game thread, as xNVSE's thread-local
//! `LoopManager`.
//!
//! Not here yet: `ForEach` over a container's inventory (inventory references, I-12),
//! `ForEachAlt`, user functions and lambdas.

use crate::expr::{self, Eval, Tok};
use crate::table::{Command, Param, Returns};
use crate::vars::{self, Key};
use crate::{handled, CommandArgs, Nvse};
use engine_core::prelude::*;
use engine_core::thread_host::current_id;
use std::collections::HashMap;

/// `kNVSEParamType_BasicType`, `_Boolean`, `_NoTypeCheck` (xNVSE `ScriptUtils.h`): bits of
/// token types, with `Ambiguous` (bit 15).
const BASIC_TYPE: u32 = 0x804d;
const BOOLEAN: u32 = 0x8002;
const NO_TYPE_CHECK: u32 = 0;

const fn p(name: &'static str, kind: u32) -> Param {
    Param {
        name,
        kind,
        optional: false,
    }
}
const ONE_BASIC_TYPE: &[Param] = &[p("expression", BASIC_TYPE)];
const ONE_BOOLEAN: &[Param] = &[p("boolean expression", BOOLEAN)];
const NO_TYPE_CHECKING: &[Param] = &[p("expression", NO_TYPE_CHECK)];
const FOREACH_CONTEXT: &[Param] = &[p("ForEach expression", 1 << 17)];
const ONE_INT: &[Param] = &[Param {
    name: "int",
    kind: 0x01,
    optional: true,
}];

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

pub static COMMANDS: &[Command] = &[
    cmd(0x149e, "Label", "set a label", ONE_INT, label),
    cmd(0x149f, "Goto", "branch to a label", ONE_INT, goto),
    cmd(
        0x1539,
        "Let",
        "assigns the value of an expression to a variable",
        ONE_BASIC_TYPE,
        let_execute,
    ),
    cmd(
        0x153a,
        "eval",
        "evaluates an expression and returns a boolean result.",
        ONE_BOOLEAN,
        eval_execute,
    ),
    cmd(
        0x153b,
        "while",
        "loops until the given condition evaluates to false",
        ONE_BOOLEAN,
        while_execute,
    ),
    cmd(
        0x153c,
        "loop",
        "marks the end of a While or ForEach loop",
        &[],
        loop_execute,
    ),
    cmd(
        0x153d,
        "ForEach",
        "iterates over the elements of an array",
        FOREACH_CONTEXT,
        foreach_execute,
    ),
    cmd(
        0x153e,
        "continue",
        "returns control to the top of a loop",
        &[],
        continue_execute,
    ),
    cmd(0x153f, "break", "exits a loop", &[], break_execute),
    cmd(
        0x1542,
        "testexpr",
        "returns false if errors occur while evaluating expression, true otherwise",
        NO_TYPE_CHECKING,
        testexpr_execute,
    ),
];

/// Runs the evaluator over a command's arguments (`ExpressionEvaluator(PASS_COMMAND_ARGS)` and
/// `ExtractArgs`), reporting its errors when it ends.
fn evaluate_args(e: &mut Engine, a: &CommandArgs, suppress: bool) -> (Option<Vec<Tok>>, Eval) {
    let mut ev = Eval::new(e, a);
    ev.suppress_errors = suppress;
    let toks = expr::with_evaluator(e, |e| ev.extract_args(e));
    expr::report(e, &ev);
    (toks, ev)
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_Scripting.cpp: Cmd_Let_Execute.
fn let_execute(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    evaluate_args(e, &a, false);
    handled(true)
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_Scripting.cpp: Cmd_eval_Execute.
fn eval_execute(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    a.set_result(e, 0.0);
    let (toks, ev) = evaluate_args(e, &a, false);
    if let Some(first) = toks.as_ref().and_then(|t| t.first()) {
        let v = ev.get_bool(e, first);
        a.set_result(e, f64::from(u8::from(v)));
    }
    handled(true)
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_Scripting.cpp:
// Cmd_testexpr_Execute.
fn testexpr_execute(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    a.set_result(e, 0.0);
    let (toks, ev) = evaluate_args(e, &a, true);
    if let Some(first) = toks.as_ref().and_then(|t| t.first()) {
        if ev.errors.is_empty() {
            let v = match first {
                Tok::Element { array, key } => {
                    let upper = vars::upper_table(e);
                    vars::array(e, *array).is_some_and(|arr| arr.get(&upper, key).is_some())
                }
                _ => true,
            };
            a.set_result(e, f64::from(u8::from(v)));
        }
    }
    handled(true)
}

/// The running script line's runner and `length` word (`GetScriptRunner`,
/// `GetCalculatedOpLength`).
fn runner_and_length(e: &Engine) -> Option<(u32, u32)> {
    crate::context::current_line(e).map(|l| (l.runner, l.length))
}

/// The runner's if/else stack (`SavedIPInfo`): its depth and the words up to it.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SavedIp {
    pub ip: u32,
    pub depth: u32,
    pub stack: Vec<u32>,
}

fn save_stack(e: &Engine, runner: u32, ip: u32) -> SavedIp {
    let depth = e.mem.u32(runner + 0x20);
    let stack = (0..=depth.min(10))
        .map(|i| e.mem.u32(runner + 0x24 + 4 * i))
        .collect();
    SavedIp { ip, depth, stack }
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Loops.cpp: LoopManager::RestoreStack.
fn restore_stack(e: &mut Engine, runner: u32, info: &SavedIp) {
    e.mem.set_u32(runner + 0x20, info.depth);
    for (i, w) in info.stack.iter().enumerate() {
        e.mem.set_u32(runner + 0x24 + 4 * i as u32, *w);
    }
}

/// Moves the runner to `target`: `*calculatedOpLength += target - *opcodeOffsetPtr`.
fn jump(e: &mut Engine, length: u32, offset_ptr: u32, target: u32) {
    let now = e.mem.u32(offset_ptr);
    let v = e.mem.u32(length).wrapping_add(target.wrapping_sub(now));
    e.mem.set_u32(length, v);
}

/// A loop (`Loop` and its kinds, xNVSE `Loops.h`).
#[derive(Clone, Debug, PartialEq)]
pub enum Loop {
    /// `WhileLoop`: the offset of its condition.
    While { expr_offset: u32 },
    /// `ArrayIterLoop` with an iterator array (`iter["key"]`, `iter["value"]`): the source,
    /// the iterator, the iterator variable's local and the current key.
    Array {
        source: u32,
        iter: u32,
        local: u32,
        key: Option<Key>,
        owner_mod: u8,
    },
    /// `StringIterLoop`: the source text, the iterator string, the index.
    Str {
        text: Vec<u8>,
        iter: u32,
        index: usize,
    },
    /// `FormListIterLoop`: the list node (0 at the end) and the reference variable's local.
    FormList { node: u32, local: u32 },
}

/// `LoopInfo`.
#[derive(Clone, Debug, PartialEq)]
pub struct LoopInfo {
    pub kind: Loop,
    pub saved: SavedIp,
    pub end_ip: u32,
}

/// The loop managers (per thread) and the labels (`s_savedIPTable`, by script form id).
#[derive(Default)]
pub struct State {
    loops: HashMap<u32, Vec<LoopInfo>>,
    labels: HashMap<u32, HashMap<u32, SavedIp>>,
}

fn loops(e: &mut Engine) -> &mut Vec<LoopInfo> {
    let thread = current_id(e);
    e.ext::<Nvse>().loops.loops.entry(thread).or_default()
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Loops.cpp: LoopManager::Add.
fn add_loop(e: &mut Engine, kind: Loop, runner: u32, start: u32, end: u32) {
    let saved = save_stack(e, runner, start);
    loops(e).push(LoopInfo {
        kind,
        saved,
        end_ip: end,
    });
}

/// Ends a loop: `ArrayIterLoop`'s destructor gives up the iterator variable's reference.
fn drop_loop(e: &mut Engine, info: &LoopInfo) {
    match info.kind {
        Loop::Array {
            iter,
            local,
            owner_mod,
            ..
        } if iter != 0 => vars::remove_reference_double(e, local + 8, owner_mod),
        _ => {}
    }
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Loops.cpp: LoopManager::Break.
fn break_loop(e: &mut Engine, a: &CommandArgs) -> bool {
    let Some(info) = loops(e).pop() else {
        return false;
    };
    if let Some((runner, length)) = runner_and_length(e) {
        restore_stack(e, runner, &info.saved);
        jump(e, length, a.opcode_offset, info.end_ip);
    }
    drop_loop(e, &info);
    true
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Loops.cpp: LoopManager::Continue.
fn continue_loop(e: &mut Engine, a: &CommandArgs) -> bool {
    let Some(mut info) = loops(e).last().cloned() else {
        return false;
    };
    if !update(e, a, &mut info.kind) {
        break_loop(e, a);
        return true;
    }
    if let Some(top) = loops(e).last_mut() {
        top.kind = info.kind.clone();
    }
    if let Some((runner, length)) = runner_and_length(e) {
        restore_stack(e, runner, &info.saved);
        jump(e, length, a.opcode_offset, info.saved.ip);
    }
    true
}

/// `Loop::Update`: the next pass, false at the end.
fn update(e: &mut Engine, a: &CommandArgs, kind: &mut Loop) -> bool {
    match kind {
        // `WhileLoop::Update`: the condition at its offset, the offset word put back.
        Loop::While { expr_offset } => {
            let original = e.mem.u32(a.opcode_offset);
            e.mem.set_u32(a.opcode_offset, *expr_offset);
            let (toks, ev) = evaluate_args(e, a, false);
            e.mem.set_u32(a.opcode_offset, original);
            match toks.as_ref().and_then(|t| t.first()) {
                Some(first) => ev.get_bool(e, first),
                None => false,
            }
        }
        Loop::Array {
            source, iter, key, ..
        } => {
            let Some(cur) = key.clone() else {
                return false;
            };
            let Some(arr) = vars::array(e, *source) else {
                return false;
            };
            let keys = arr.keys();
            // `GetNextElement`: the first key after the current one.
            let upper = vars::upper_table(e);
            let next = keys.into_iter().find(|k| key_less(&upper, &cur, k));
            let Some(next) = next else {
                return false;
            };
            *key = Some(next.clone());
            update_iterator(e, *source, *iter, &next)
        }
        Loop::Str { text, iter, index } => {
            if vars::string(e, *iter).is_none() {
                return false;
            }
            *index += 1;
            if *index < text.len() {
                vars::set_string(e, *iter, &text[*index..*index + 1]);
                true
            } else {
                false
            }
        }
        Loop::FormList { node, local } => {
            if *node == 0 {
                return false;
            }
            *node = e.mem.u32(*node + 4);
            while *node != 0 {
                let form = e.mem.u32(*node);
                if form != 0 {
                    e.mem.set_u32(*local + 8, e.mem.u32(form + 0xc));
                    e.mem.set_u32(*local + 12, 0);
                    return true;
                }
                *node = e.mem.u32(*node + 4);
            }
            false
        }
    }
}

/// `ArrayKey::operator<` for keys of one type.
fn key_less(upper: &[u8; 256], a: &Key, b: &Key) -> bool {
    match (a, b) {
        (Key::Number(x), Key::Number(y)) => x < y,
        (Key::Str(x), Key::Str(y)) => vars::str_compare(upper, x, y).is_lt(),
        _ => true,
    }
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Loops.cpp: ArrayIterLoop::UpdateIterator
// (the iterator-array form).
fn update_iterator(e: &mut Engine, source: u32, iter: u32, key: &Key) -> bool {
    let Some(elem) = vars::get_element(e, source, key) else {
        return false;
    };
    let key_value = match key {
        Key::Str(s) => vars::Value::Str(s.clone()),
        Key::Number(n) => vars::Value::Number(*n),
    };
    if !vars::set_element(e, iter, &Key::Str(b"key".to_vec()), key_value) {
        return false;
    }
    let value = match elem.data {
        vars::Data::Number(n) => vars::Value::Number(n),
        vars::Data::Form(f) => vars::Value::Form(f),
        vars::Data::Str(s) => vars::Value::Str(s),
        vars::Data::Array(a) => vars::Value::Array(a),
        vars::Data::Invalid => return false,
    };
    vars::set_element(e, iter, &Key::Str(b"value".to_vec()), value)
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_Scripting.cpp:
// Cmd_While_Execute.
/// The data: a `UInt32` offset to the loop's end, the argument count, the condition.
fn while_execute(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let Some((runner, _)) = runner_and_length(e) else {
        return handled(true);
    };
    let offset = e.mem.u32(a.opcode_offset);
    let at = a.data.wrapping_add(offset);
    let end = e.mem.u32(at);
    let expr_len = u32::from(e.mem.u16(at + 5));
    let start = offset + 5 + expr_len;
    add_loop(
        e,
        Loop::While {
            expr_offset: offset + 4,
        },
        runner,
        start,
        end,
    );
    e.mem.set_u32(a.opcode_offset, start);
    let mut kind = Loop::While {
        expr_offset: offset + 4,
    };
    if !update(e, &a, &mut kind) {
        break_loop(e, &a);
    }
    handled(true)
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_Scripting.cpp:
// Cmd_ForEach_Execute, with the ArrayIterLoop, StringIterLoop and FormListIterLoop constructors
// (Loops.cpp).
fn foreach_execute(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let Some((runner, _)) = runner_and_length(e) else {
        return handled(true);
    };
    let offset = e.mem.u32(a.opcode_offset);
    let at = a.data.wrapping_add(offset);
    let end = e.mem.u32(at);
    let expr_len = u32::from(e.mem.u16(at + 5));
    let start = offset + 5 + expr_len;
    e.mem.set_u32(a.opcode_offset, offset + 4);
    let (toks, _) = evaluate_args(e, &a, false);
    let now = e.mem.u32(a.opcode_offset);
    e.mem.set_u32(a.opcode_offset, now.wrapping_sub(4));
    let Some(Tok::ForEach {
        source,
        iter,
        local,
        var_type,
    }) = toks.and_then(|t| t.into_iter().next())
    else {
        // `ForEach expression failed to return a value`: the line fails.
        return handled(false);
    };
    let modi = vars::script_mod_index(e, a.script);
    let (kind, empty) = match var_type {
        3 => {
            // Clear the iterator variable, then refer it to the iterator array.
            vars::remove_reference_double(e, local + 8, modi);
            vars::add_reference_double(e, local + 8, iter, modi);
            let mut key = None;
            let mut ok_source = source;
            if source != 0 {
                match vars::array(e, source).map(|arr| arr.keys()) {
                    Some(keys) => {
                        if let Some(first) = keys.into_iter().next() {
                            if !update_iterator(e, source, iter, &first) {
                                ok_source = 0;
                            }
                            key = Some(first);
                        }
                    }
                    None => ok_source = 0,
                }
            }
            vars::collect(e, a.event_list, local, vars::VarType::Array);
            let empty =
                ok_source == 0 || !vars::array(e, ok_source).is_some_and(|arr| !arr.is_empty());
            (
                Loop::Array {
                    source: ok_source,
                    iter,
                    local,
                    key,
                    owner_mod: modi,
                },
                empty,
            )
        }
        2 => {
            let text = vars::string(e, source).unwrap_or_default();
            if !text.is_empty() {
                vars::set_string(e, iter, &text[..1]);
            }
            let empty = text.is_empty();
            (
                Loop::Str {
                    text,
                    iter,
                    index: 0,
                },
                empty,
            )
        }
        _ => {
            // A form list (a container needs inventory references: I-12).
            if e.mem.u8(source + 4) != 0x55 {
                return handled(false);
            }
            let mut node = source + 0x18;
            while node != 0 && e.mem.u32(node) == 0 {
                node = e.mem.u32(node + 4);
            }
            if node != 0 {
                let form = e.mem.u32(node);
                e.mem.set_u32(local + 8, e.mem.u32(form + 0xc));
                e.mem.set_u32(local + 12, 0);
            }
            (Loop::FormList { node, local }, node == 0)
        }
    };
    add_loop(e, kind, runner, start, end);
    if empty {
        e.mem.set_u32(a.opcode_offset, start);
        break_loop(e, &a);
    }
    handled(true)
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_Scripting.cpp:
// Cmd_Loop_Execute, Cmd_Continue_Execute, Cmd_Break_Execute. Outside a loop each is a runtime
// error and the line fails.
fn loop_execute(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    handled(continue_loop(e, &a))
}

fn continue_execute(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    if continue_loop(e, &a) {
        let now = e.mem.u32(a.opcode_offset);
        e.mem.set_u32(a.opcode_offset, now.wrapping_add(4));
        return handled(true);
    }
    handled(false)
}

fn break_execute(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    handled(break_loop(e, &a))
}

/// `Label`'s and `Goto`'s index (`ExtractArgs(&idx)`), and the offset word moved past the
/// arguments by their length (the word just before them).
fn label_index(e: &mut Engine, a: &CommandArgs) -> Option<u32> {
    let read = super::Read::of(e, a);
    if !read.ok {
        return None;
    }
    let offset = e.mem.u32(a.opcode_offset);
    let len = u32::from(e.mem.u16(a.data.wrapping_add(offset).wrapping_sub(2)));
    e.mem.set_u32(a.opcode_offset, offset.wrapping_add(len));
    Some(read.int(0, 0))
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_Scripting.cpp: Cmd_Label_Execute.
fn label(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let Some(idx) = label_index(e, &a) else {
        return handled(true);
    };
    let Some((runner, _)) = runner_and_length(e) else {
        return handled(true);
    };
    let ip = e.mem.u32(a.opcode_offset);
    let saved = save_stack(e, runner, ip);
    let script_id = e.mem.u32(a.script + 0xc);
    e.ext::<Nvse>()
        .loops
        .labels
        .entry(script_id)
        .or_default()
        .insert(idx, saved);
    handled(true)
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_Scripting.cpp: Cmd_Goto_Execute.
fn goto(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = super::Read::of(e, &a);
    if !read.ok {
        return handled(true);
    }
    let script_id = e.mem.u32(a.script + 0xc);
    let Some(info) = e
        .ext::<Nvse>()
        .loops
        .labels
        .get(&script_id)
        .and_then(|m| m.get(&read.int(0, 0)))
        .cloned()
    else {
        return handled(true);
    };
    let offset = e.mem.u32(a.opcode_offset);
    let len = u32::from(e.mem.u16(a.data.wrapping_add(offset).wrapping_sub(2)));
    e.mem.set_u32(a.opcode_offset, offset.wrapping_add(len));
    if let Some((runner, length)) = runner_and_length(e) {
        jump(e, length, a.opcode_offset, info.ip);
        restore_stack(e, runner, &info);
    }
    handled(true)
}

/// The `<-` operator's result (`Eval_In`, a `ForEachContext`), made by [`crate::expr`].
pub fn foreach_context(e: &mut Engine, ev: &Eval, lh: &Tok, rh: &Tok) -> Result<Tok, &'static str> {
    let Tok::Var {
        local, var_type, ..
    } = lh
    else {
        return Err("Unsupported variable type to iterate over");
    };
    let modi = vars::script_mod_index(e, ev.script);
    match *var_type {
        3 => {
            let iter = vars::create_array(e, vars::KEY_STRING, false, modi);
            let source = ev.get_array_id(e, rh);
            Ok(Tok::ForEach {
                source,
                iter,
                local: *local,
                var_type: 3,
            })
        }
        2 => {
            let text = ev.get_string(e, rh);
            let source = vars::add_string(e, modi, &text, true);
            let mut iter = crate::args::f64_to_i32(e.mem.f64(local + 8)) as u32;
            if vars::string(e, iter).is_none() {
                vars::collect(e, ev.event_list, *local, vars::VarType::String);
                iter = vars::add_string(e, modi, b"", false);
                e.mem.set_f64(local + 8, f64::from(iter));
            }
            Ok(Tok::ForEach {
                source,
                iter,
                local: *local,
                var_type: 2,
            })
        }
        4 => {
            let form = ev.get_form(e, rh);
            if form == 0 {
                return Err("Source is a base form (must be a reference)");
            }
            if e.mem.u8(form + 4) != 0x55 {
                // A container reference: inventory references come with I-12.
                return Err("ForEach over a container is not supported yet");
            }
            Ok(Tok::ForEach {
                source: form,
                iter: 0,
                local: *local,
                var_type: 4,
            })
        }
        _ => Err("Unsupported variable type to iterate over (only array, string and ref variables supported for non-alt ForEach)"),
    }
}

#[cfg(test)]
#[path = "scripting_tests.rs"]
mod tests;
