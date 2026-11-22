//! `ScriptRunner`: the line loop of a script's run (`Run`), the action list's run, the read of one
//! compiled line's opcode and length (`ParseLine`) and its evaluation (`EvaluateLine`). The runner
//! (0xa4 bytes): the container (+0), the reference's form (+4), the locals (+8), the line's
//! reference flag (+0x10), the error code (+0x18), the expression error (+0x1c), the if-stack's
//! depth (+0x20) and levels (+0x24, a word each), the running script (+0x14), the error byte
//! (+0xa0) and the "references changed" byte (+0xa1).

use crate::scripts;
use engine_core::extend::Hook;
use engine_core::ptr::Ptr;
use engine_core::typed::leaf;
use engine_core::x87::x87_quiet;
use engine_core::{args, typed_call, Engine};

const RUNNING: u32 = 0x14;
/// `QueryPerformanceFrequency`'s result the runner keeps (two words), and the two imports.
const PROFILER_FREQUENCY: u32 = 0x011c_aec8;
const QUERY_PERFORMANCE_FREQUENCY: u32 = 0x00fd_f0a4;
const QUERY_PERFORMANCE_COUNTER: u32 = 0x00fd_f0a0;
/// Whether the interface's debug text is shown (the profiler runs then).
const DEBUG_TEXT_SHOWN: u32 = 0x0070_2fc0;
/// The script's compiled-data flag test, its header, the locals made for a run, the locals'
/// delete, the profiler timer's add.
const FORM_FLAG_BIT_3: u32 = 0x0040_13e0;
const SCRIPT_HEADER_OF: u32 = 0x0050_0940;
const NEW_LOCALS: u32 = 0x005a_bf60;
const DELETE_LOCALS: u32 = 0x0041_af70;
const ADD_PROFILER_TIME: u32 = 0x005e_1140;
const ALLOCATE: u32 = 0x0040_1000;
/// `CExpression`'s destructor.
const EXPRESSION_DESTRUCT: u32 = 0x0048_3710;
/// The wrapped pair, by address (xNVSE).
const PARSE_LINE: u32 = 0x005e_13a0;
const EVALUATE_LINE: u32 = 0x005e_1550;
/// The script log, the console, the runner's error report and error code, the command table, the
/// cast, the parameter parser and the operand reader (`overridable.tsv`).
const SCRIPT_LOG: u32 = 0x005b_5e40;
const CONSOLE_PRINT: u32 = 0x0070_3c00;
const REPORT_ERROR: u32 = 0x005e_2410;
const SET_ERROR_CODE: u32 = 0x005e_24b0;
const GET_FUNCTION_DEFINITION: u32 = 0x005b_1120;
const DYNAMIC_CAST: u32 = 0x00ec_43fb;
const PARSE_PARAMETERS: u32 = 0x005a_ccb0;
const READ_OPERAND: u32 = 0x005a_c7a0;
/// The RTTI descriptors of the cast: `TESForm`, `TESObjectREFR`, `TESQuest`.
const RTTI_FORM: u32 = 0x0118_3028;
const RTTI_REFERENCE: u32 = 0x0118_41cc;
const RTTI_QUEST: u32 = 0x0118_6500;
/// The block types' table (0x28 bytes each: needs a reference, parameters +4, handler +8).
const BLOCK_TYPES: u32 = 0x0118_e300;
/// `CExpression`'s error names, and the double an `if` condition is compared with (0.0).
const EXPRESSION_ERROR_NAMES: u32 = 0x0118_c594;
const CONDITION_FALSE: u32 = 0x0101_2060;
/// The log formats.
const NESTED_RUN_MESSAGE: u32 = 0x0103_d498;
const REFERENCE_COMMAND_UNKNOWN: u32 = 0x0103_d538;
const REFERENCE_UNKNOWN: u32 = 0x0103_d4f4;
const BLOCK_TYPE_INVALID: u32 = 0x0103_d8d0;
const BLOCK_NEEDS_REFERENCE: u32 = 0x0103_d890;
const BLOCK_FAILED: u32 = 0x0103_d848;
const SET_NO_REFERENCE: u32 = 0x0103_d7b8;
const SET_NO_LOCALS: u32 = 0x0103_d7f8;
const SET_EXPRESSION_ERROR: u32 = 0x0103_d770;
const SET_NO_GLOBAL: u32 = 0x0103_d718;
const SET_GLOBAL_ECHO: u32 = 0x0103_d758;
const SET_NO_TARGET_LOCALS: u32 = 0x0103_d6a8;
const SET_FLOAT_ECHO: u32 = 0x0103_d6f4;
const SET_INTEGER_ECHO: u32 = 0x0103_d704;
const CONDITION_ERROR: u32 = 0x0103_d664;
const COMMAND_UNKNOWN: u32 = 0x0103_d618;
const COMMAND_NEEDS_REFERENCE: u32 = 0x0103_d5d0;
const COMMAND_FAILED: u32 = 0x0103_d588;
/// The global's set, a variable's name, the reference's changed test and its
/// process.
const SET_GLOBAL: u32 = 0x0046_dce0;
const VARIABLE_NAME: u32 = 0x005a_bed0;
const REFERENCE_SCRIPT_VARIABLES: u32 = 0x0056_73e0;
const QUEST_SCRIPT_LOCALS: u32 = 0x0059_e300;
const IS_DELETED: u32 = 0x0040_77c0;
const PROCESS_OF: u32 = 0x008d_6f30;
const PROCESS_KEEPS: u32 = 0x0045_0ff0;
const FLOAT_TO_LONG: u32 = 0x00ec_62c0;
/// The temporary string of a report: its constructor, printf, text and destructor.
const STRING_CONSTRUCT: u32 = 0x0040_37b0;
const STRING_PRINTF: u32 = 0x0040_6f60;
const STRING_TEXT: u32 = 0x0046_4f30;
const STRING_DESTRUCT: u32 = 0x0040_37d0;
const PARSE_COUNTED_COMMAND: i32 = 0x1059;

/// `ScriptLocals::SetVariable` (the scripts family's) on the locals at `locals`.
#[inline]
fn set_variable(e: &mut Engine, locals: u32, index: i32, value: f64) {
    typed_call!(
        scripts::FAMILY,
        0x005a_9290,
        scripts::run::set_variable(e, Ptr::new(locals), index as u32, value)
    );
}

/// The thread's console-echo byte (TLS +0x268, the inline body of `005e23f0`).
#[inline]
fn console_echo(e: &mut Engine) -> bool {
    let tls = e.tls();
    e.mem.u8(tls + 0x268) != 0
}

/// A script's type name (its slot +0x130).
#[inline]
fn name_of(e: &mut Engine, form: u32) -> u32 {
    e.vcall(form, 0x130, &args![]).u32()
}

#[inline]
fn header_of(e: &mut Engine, script: Ptr) -> u32 {
    e.call(SCRIPT_HEADER_OF, &args![script]).u32()
}

/// JIP LN NVSE's `bNoFailedScriptLocks` (O7): its three jumps skip the clear of a failed script's
/// data length. The switch's bit is tested first; true when JIP's handler at `site` takes it.
#[inline]
fn jip_keeps_failed_script(e: &mut Engine, site: u32) -> bool {
    e.jip_patches() & engine_core::extend::jip_patches::NO_FAILED_SCRIPT_LOCKS != 0
        && e.site(site, &[]).is_some()
}

/// The address of if-stack level `depth`.
#[inline]
fn if_level(this: u32, depth: u32) -> u32 {
    this.wrapping_add(0x24).wrapping_add(depth.wrapping_mul(4))
}

/// A fresh `CExpression` at `expression` (the scripts family's constructor).
#[inline]
fn new_expression(e: &mut Engine, expression: Ptr) {
    typed_call!(
        scripts::FAMILY,
        0x0059_3490,
        scripts::expression::construct(e, expression)
    );
}

// Typed from 005e0d20
/// The runner's line loop: runs `script`'s compiled lines through `ParseLine` and `EvaluateLine`
/// (both by address, where xNVSE wraps them) until the data ends, a parse fails (the script's data
/// size is then cleared, unless JIP keeps it), a line fails, or a `return` (opcode 0x1e). The runner
/// keeps the container, the reference's form and the locals (fresh ones, deleted afterwards, when
/// none are given). A magic-effect script stores `effect_start`, `effect_finish` and `elapsed` in
/// the locals' effect block (made when missing). Unless `keep_actions` or a line failed, the
/// locals' action list is processed. With the debug text shown the run's time goes to the script's
/// profiler timer. Answers the "references changed" byte, cleared on entry; a busy runner logs and
/// runs nothing.
#[allow(clippy::too_many_arguments)]
pub fn run_lines(
    e: &mut Engine,
    this: Ptr,
    script: Ptr,
    reference: Ptr,
    locals: Ptr,
    container: Ptr,
    keep_actions: bool,
    effect_start: u8,
    effect_finish: u8,
    elapsed: f32,
) -> bool {
    let me = this.addr();
    let profile = e.call(DEBUG_TEXT_SHOWN, &args![]).bool();
    e.with_stack(8, |e, start| {
        if profile {
            if e.mem.u32(PROFILER_FREQUENCY) | e.mem.u32(PROFILER_FREQUENCY + 4) == 0 {
                e.call(QUERY_PERFORMANCE_FREQUENCY, &args![PROFILER_FREQUENCY]);
            }
            e.call(QUERY_PERFORMANCE_COUNTER, &args![start]);
        }
        e.mem.set_u8(me + 0xa1, 0);
        if script.is_null() || !e.call(FORM_FLAG_BIT_3, &args![script]).bool() {
            return e.mem.u8(me + 0xa1) != 0;
        }
        let running = e.mem.u32(me + RUNNING);
        if running != 0 {
            let requested_name = name_of(e, script.addr());
            let running = e.mem.u32(me + RUNNING);
            let running_name = name_of(e, running);
            e.call(
                SCRIPT_LOG,
                &args![NESTED_RUN_MESSAGE, running_name, requested_name],
            );
            return e.mem.u8(me + 0xa1) != 0;
        }
        e.mem.set_u32(me + RUNNING, script.addr());
        e.with_stack(0x710, |e, expression| {
            new_expression(e, expression);
            let mut created_locals = 0u32;
            e.mem.set_u32(me + 0x24, 0);
            e.mem.set_u32(me + 0x20, 0);
            e.with_stack(20, |e, work| {
                let opcode = work.addr();
                let cursor = opcode + 4;
                let length = opcode + 8;
                let target = opcode + 12;
                let skip = opcode + 16;
                e.mem.set_u32(opcode, 0xffff);
                e.mem.set_u32(cursor, 0);
                e.mem.set_u32(me, container.addr());
                let form = if reference.is_null() {
                    0
                } else {
                    typed_call!(leaf::FAMILY, 0x007a_f430, leaf::word_at_20(e, reference))
                };
                e.mem.set_u32(me + 4, form);
                if locals.is_null() {
                    created_locals = e.call(NEW_LOCALS, &args![script]).u32();
                    e.mem.set_u32(me + 8, created_locals);
                } else {
                    e.mem.set_u32(me + 8, locals.addr());
                }
                // The seam at `005e0ec9` (JIP LN NVSE's script wait: which wait this run is),
                // before the header read; the answer is not used.
                if e.hook_users(Hook::ScriptWait) != 0 {
                    e.site(0x005e_0ec9, &[script.addr(), reference.addr()]);
                }
                let header = header_of(e, script);
                if e.mem.u8(header + 0x11) != 0 {
                    let runner_locals = e.mem.u32(me + 8);
                    if e.mem.u32(runner_locals + 0x10) == 0 {
                        let block = e.call(ALLOCATE, &args![8u32]).u32();
                        let runner_locals = e.mem.u32(me + 8);
                        e.mem.set_u32(runner_locals + 0x10, block);
                    }
                    let effect = e.mem.u32(e.mem.u32(me + 8) + 0x10);
                    e.mem.set_u8(effect, effect_start);
                    let effect = e.mem.u32(e.mem.u32(me + 8) + 0x10);
                    e.mem.set_u8(effect + 1, effect_finish);
                    let effect = e.mem.u32(e.mem.u32(me + 8) + 0x10);
                    e.mem.set_f32(effect + 4, x87_quiet(elapsed));
                }
                e.mem.set_u32(length, 0);
                e.mem.set_u32(target, 0);
                e.mem.set_u32(skip, 0);
                let mut failed = false;
                let mut line = 0u32;
                loop {
                    let header = header_of(e, script);
                    if e.mem.u32(cursor) >= e.mem.u32(header + 8) {
                        break;
                    }
                    line = line.wrapping_add(1);
                    e.mem.set_u32(me + 0x10, 0);
                    e.mem.set_u32(length, 0);
                    let skipping = e.mem.u32(skip) != 0;
                    if !e
                        .call(
                            PARSE_LINE,
                            &args![this, script, opcode, cursor, length, target, skipping],
                        )
                        .bool()
                    {
                        // 005e0fbe: JIP LN NVSE's O7 seam jumps past the clear.
                        if !jip_keeps_failed_script(e, 0x005e_0fbe) {
                            let header = header_of(e, script);
                            e.mem.set_u32(header + 8, 0);
                        }
                        break;
                    }
                    let remaining = e.mem.u32(skip);
                    if remaining != 0 {
                        e.mem.set_u32(skip, remaining.wrapping_sub(1));
                    } else {
                        if e.mem.u32(opcode) == 0x1e {
                            break;
                        }
                        if e.mem.u32(target) == 0 {
                            e.mem.set_u32(target, reference.addr());
                        }
                        let command = e.mem.u32(opcode);
                        let target_ref = e.mem.u32(target);
                        let position = e.mem.u32(cursor);
                        if !e
                            .call(
                                EVALUATE_LINE,
                                &args![
                                    this, script, command, target_ref, length, position, skip,
                                    line, 0u32, 0u32
                                ],
                            )
                            .bool()
                        {
                            failed = true;
                            break;
                        }
                    }
                    let next = e.mem.u32(cursor).wrapping_add(e.mem.u32(length));
                    e.mem.set_u32(cursor, next);
                }
                if !keep_actions && !failed {
                    let runner_locals = e.mem.u32(me + 8);
                    typed_call!(
                        scripts::FAMILY,
                        0x005a_8ea0,
                        scripts::run::clear_action_flags(e, Ptr::new(runner_locals))
                    );
                }
            });
            if created_locals != 0 {
                e.call(DELETE_LOCALS, &args![created_locals, 1u32]);
            }
            e.mem.set_u32(me + RUNNING, 0);
            if profile {
                e.with_stack(8, |e, now| {
                    e.call(QUERY_PERFORMANCE_COUNTER, &args![now]);
                    let ticks =
                        (e.mem.u64(now.addr()) as i64).wrapping_sub(e.mem.u64(start.addr()) as i64);
                    let frequency = e.mem.u64(PROFILER_FREQUENCY) as i64;
                    let seconds = seconds_of(ticks, frequency);
                    e.call(ADD_PROFILER_TIME, &args![script, seconds]);
                });
            }
            let changed = e.mem.u8(me + 0xa1);
            e.call(EXPRESSION_DESTRUCT, &args![expression]);
            changed != 0
        })
    })
}

/// The profiler's seconds: the tick count over the frequency, stored as a float.
#[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
fn seconds_of(ticks: i64, frequency: i64) -> f32 {
    (ticks as f64 / frequency as f64) as f32
}

// Typed from 005e1160
/// Builds a script's action list: runs its lines through `ParseLine` and `EvaluateLine` (by
/// address) with the action-list flag, so every line but a `BEGIN` is accepted unread. A parse
/// failure clears the script's data size; a busy runner logs and builds nothing.
pub fn build_action_list(
    e: &mut Engine,
    this: Ptr,
    script: Ptr,
    reference: Ptr,
    locals: Ptr,
) -> bool {
    let me = this.addr();
    if script.is_null() || !e.call(FORM_FLAG_BIT_3, &args![script]).bool() {
        return false;
    }
    if e.mem.u32(me + RUNNING) != 0 {
        let requested_name = name_of(e, script.addr());
        let running = e.mem.u32(me + RUNNING);
        let running_name = name_of(e, running);
        e.call(
            SCRIPT_LOG,
            &args![NESTED_RUN_MESSAGE, running_name, requested_name],
        );
        return false;
    }
    e.mem.set_u32(me + RUNNING, script.addr());
    e.with_stack(0x710, |e, expression| {
        new_expression(e, expression);
        let object = if reference.is_null() {
            0
        } else {
            typed_call!(leaf::FAMILY, 0x007a_f430, leaf::word_at_20(e, reference))
        };
        e.mem.set_u32(me + 4, object);
        e.mem.set_u32(me + 8, locals.addr());
        e.with_stack(20, |e, work| {
            let opcode = work.addr();
            let cursor = opcode + 4;
            let length = opcode + 8;
            let target = opcode + 12;
            let skip = opcode + 16;
            e.mem.set_u32(opcode, 0xffff);
            for word in [cursor, length, target, skip] {
                e.mem.set_u32(word, 0);
            }
            let mut line = 0u32;
            loop {
                let header = header_of(e, script);
                if e.mem.u32(cursor) >= e.mem.u32(header + 8) {
                    break;
                }
                line = line.wrapping_add(1);
                e.mem.set_u32(me + 0x10, 0);
                e.mem.set_u32(length, 0);
                let skipping = e.mem.u32(skip) != 0;
                if !e
                    .call(
                        PARSE_LINE,
                        &args![this, script, opcode, cursor, length, target, skipping],
                    )
                    .bool()
                {
                    let header = header_of(e, script);
                    e.mem.set_u32(header + 8, 0);
                    break;
                }
                let remaining = e.mem.u32(skip);
                if remaining != 0 {
                    e.mem.set_u32(skip, remaining.wrapping_sub(1));
                } else {
                    if e.mem.u32(target) == 0 {
                        e.mem.set_u32(target, reference.addr());
                    }
                    let command = e.mem.u32(opcode);
                    let target_ref = e.mem.u32(target);
                    let position = e.mem.u32(cursor);
                    if !e
                        .call(
                            EVALUATE_LINE,
                            &args![
                                this, script, command, target_ref, length, position, skip, line,
                                0u32, 1u32
                            ],
                        )
                        .bool()
                    {
                        break;
                    }
                }
                let next = e.mem.u32(cursor).wrapping_add(e.mem.u32(length));
                e.mem.set_u32(cursor, next);
            }
        });
        e.mem.set_u32(me + RUNNING, 0);
        e.call(EXPRESSION_DESTRUCT, &args![expression]);
    });
    true
}

/// The signed 16-bit word of the script's data at `*cursor`.
#[inline]
fn script_word(e: &mut Engine, script: Ptr, cursor: u32) -> i32 {
    let data = typed_call!(leaf::FAMILY, 0x0067_1d10, leaf::word_at_30(e, script));
    i32::from(e.mem.i16(data.wrapping_add(e.mem.u32(cursor))))
}

#[inline]
fn advance_word(e: &Engine, cursor: u32) {
    e.mem.set_u32(cursor, e.mem.u32(cursor).wrapping_add(2));
}

// Typed from 005e13a0
/// Reads a line's signed opcode and length words, the opcode first; a reference prefix (opcode
/// 0x1c) names the referenced object whose reference is the line's target, then the real opcode.
/// A skipped prefix leaves the target as it is; an ordinary line clears it. A prefix that names no
/// reference (and no variable holding one) is reported and fails.
#[allow(clippy::too_many_arguments)]
pub fn read_line(
    e: &mut Engine,
    this: Ptr,
    script: Ptr,
    opcode: Ptr,
    cursor: Ptr,
    length: Ptr,
    reference: Ptr,
    skip_reference: bool,
) -> bool {
    let cursor = cursor.addr();
    e.mem.set_u32(length.addr(), 0);
    let command = script_word(e, script, cursor);
    e.mem.set_i32(opcode.addr(), command);
    advance_word(e, cursor);
    if e.mem.u32(opcode.addr()) == 0x1c {
        let index = script_word(e, script, cursor);
        advance_word(e, cursor);
        let mut variable = false;
        if !skip_reference {
            let locals = e.mem.u32(this.addr() + 8);
            let found = typed_call!(
                scripts::FAMILY,
                0x005a_c4f0,
                scripts::value::referenced_object(e, script, index as u32, locals)
            );
            if found != 0 {
                variable = e.mem.u32(found + 0xc) != 0;
                let object = e.mem.u32(found + 8);
                if object != 0 {
                    let target = e
                        .call(
                            DYNAMIC_CAST,
                            &args![object, 0u32, RTTI_FORM, RTTI_REFERENCE, 0u32],
                        )
                        .u32();
                    e.mem.set_u32(reference.addr(), target);
                }
            }
        }
        let command = script_word(e, script, cursor);
        e.mem.set_i32(opcode.addr(), command);
        advance_word(e, cursor);
        if !skip_reference && e.mem.u32(reference.addr()) == 0 && !variable {
            let command = e.mem.u32(opcode.addr());
            let descriptor = e.call(GET_FUNCTION_DEFINITION, &args![command]).u32();
            let script_name = name_of(e, script.addr());
            if descriptor != 0 {
                let command_name = e.mem.u32(descriptor);
                e.call(
                    REPORT_ERROR,
                    &args![
                        this,
                        script,
                        REFERENCE_COMMAND_UNKNOWN,
                        command_name,
                        script_name
                    ],
                );
            } else {
                e.call(
                    REPORT_ERROR,
                    &args![this, script, REFERENCE_UNKNOWN, script_name],
                );
            }
            return false;
        }
    } else {
        e.mem.set_u32(reference.addr(), 0);
    }
    let size = script_word(e, script, cursor);
    e.mem.set_i32(length.addr(), size);
    advance_word(e, cursor);
    true
}

/// The words of one `EvaluateLine` call that its branches share.
struct Line {
    this: u32,
    script: Ptr,
    reference: Ptr,
    data: u32,
    position: u32,
    skip: u32,
    length: u32,
    line: u32,
    parse_only: bool,
}

// Typed from 005e1550
/// `ScriptRunner::EvaluateLine`: one compiled line at `position` (after its opcode and length
/// words). `BEGIN` (0x10) runs its block type's handler and adds the block's span to `*length`
/// when the block does not apply; with `action_list` every other line is accepted unread.
/// Otherwise: `set` (0x15), `if` and `elseif` (0x16, 0x18: the if-stack's levels, bit 1 open, bit 2
/// a branch taken; a false condition adds the jump count to `*skip`), `else` (0x17), `endif`
/// (0x19), `end`, a reference prefix and `scriptname` (0x11, 0x1c, 0x1d: nothing), and every other
/// opcode a command through its record. `parse_only` reads parameters and expressions without
/// effects. False when the line failed (logged).
///
/// Seams: `005e16c3` before the block type's handler; `005e1716` (JIP LN's script wait) in place of
/// the block test; `005e1a23` and `005e1f0b` (JIP's kept failed scripts); `005e2329` before a
/// command's handler.
#[allow(clippy::too_many_arguments)]
pub fn run_line(
    e: &mut Engine,
    this: Ptr,
    script: Ptr,
    command: i32,
    reference: Ptr,
    length: Ptr,
    position: u32,
    skip: Ptr,
    line: u32,
    parse_only: bool,
    action_list: bool,
) -> bool {
    let me = this.addr();
    let data = typed_call!(leaf::FAMILY, 0x0067_1d10, leaf::word_at_30(e, script));
    if command != 0x10 {
        if action_list {
            return true;
        }
        let at = Line {
            this: me,
            script,
            reference,
            data,
            position,
            skip: skip.addr(),
            length: length.addr(),
            line,
            parse_only,
        };
        return match command.wrapping_sub(0x10) as u32 {
            0x01 | 0x0c | 0x0d => true,
            0x05 => evaluate_set(e, &at),
            0x06 => {
                let depth = e.mem.u32(me + 0x20);
                if e.mem.u32(if_level(me, depth)) != 0 {
                    e.mem.set_u32(me + 0x20, depth.wrapping_add(1));
                }
                let depth = e.mem.u32(me + 0x20);
                e.mem.set_u32(if_level(me, depth), 1);
                evaluate_condition(e, &at)
            }
            0x08 => evaluate_condition(e, &at),
            0x07 => {
                let count = i32::from(e.mem.i16(data.wrapping_add(position)));
                let depth = e.mem.u32(me + 0x20);
                let level = e.mem.u32(if_level(me, depth));
                if level & 2 == 0 {
                    e.mem.set_u32(if_level(me, depth), level | 2);
                } else if !parse_only {
                    let skipped = e.mem.u32(skip.addr()).wrapping_add(count as u32);
                    e.mem.set_u32(skip.addr(), skipped);
                }
                true
            }
            0x09 => {
                let depth = e.mem.u32(me + 0x20);
                e.mem.set_u32(if_level(me, depth), 0);
                let depth = e.mem.u32(me + 0x20);
                if depth != 0 {
                    e.mem.set_u32(me + 0x20, depth.wrapping_sub(1));
                }
                true
            }
            0x0e if parse_only => true,
            _ => evaluate_command(e, &at, command),
        };
    }
    e.with_stack(12, |e, work| {
        let cursor = work.addr();
        let result = cursor + 4;
        let kind = i32::from(e.mem.i16(data.wrapping_add(position)));
        e.mem.set_u32(cursor, position.wrapping_add(2));
        if !(0..0x26).contains(&kind) {
            let form_id = typed_call!(leaf::FAMILY, 0x0084_e3a0, leaf::word_at_c(e, script));
            let name = name_of(e, script.addr());
            e.call(SCRIPT_LOG, &args![BLOCK_TYPE_INVALID, line, name, form_id]);
            return false;
        }
        let span = e.mem.u32(data.wrapping_add(e.mem.u32(cursor)));
        e.mem.set_u32(cursor, position.wrapping_add(6));
        let entry = BLOCK_TYPES + kind as u32 * 0x28;
        let parameters = e.mem.u32(entry + 4);
        let handler = e.mem.u32(entry + 8);
        if parse_only {
            if parameters != 0 && e.mem.u32(length.addr()) != 0 {
                let locals = e.mem.u32(me + 8);
                let container = e.mem.u32(me);
                e.call(
                    PARSE_PARAMETERS,
                    &args![parameters, data, cursor, reference, container, script, locals],
                );
            }
            return true;
        }
        if e.mem.u8(entry) != 0 && reference.is_null() {
            let name = name_of(e, script.addr());
            e.call(SCRIPT_LOG, &args![BLOCK_NEEDS_REFERENCE, name, line]);
            return false;
        }
        let succeeded = handler != 0 && {
            let locals = e.mem.u32(me + 8);
            let container = e.mem.u32(me);
            // The seam before the block type's handler (`005e16c3`, ADR-0016): the block record's
            // opcode, the record and the result's address.
            let record = entry - 0x10;
            e.site(0x005e_16c3, &[e.mem.u32(record + 8), record, result]);
            e.call(
                handler,
                &args![parameters, data, reference, container, script, locals, result, cursor],
            )
            .bool()
        };
        if !succeeded {
            let failed_at = e.mem.u32(cursor);
            let name = name_of(e, script.addr());
            e.call(SCRIPT_LOG, &args![BLOCK_FAILED, name, line, failed_at]);
            return false;
        }
        // The seam at `005e1716` (JIP LN NVSE's script wait: a waiting script resumes in its
        // block), in place of the block test: an answer means its handler made the test.
        if e.hook_users(Hook::ScriptWait) != 0
            && e.site(
                0x005e_1716,
                &[
                    me,
                    kind as u32,
                    e.mem.u32(cursor),
                    e.mem.u32(result + 4),
                    u32::from(action_list),
                    length.addr(),
                    span,
                ],
            )
            .is_some()
        {
            return true;
        }
        if e.mem.f64(result) == 0.0 || action_list {
            e.mem
                .set_u32(length.addr(), e.mem.u32(length.addr()).wrapping_add(span));
        }
        true
    })
}

/// Logs `format` with the line, the script's name and its form id (asked first).
fn log_line_error(e: &mut Engine, at: &Line, format: u32) {
    let form_id = typed_call!(leaf::FAMILY, 0x0084_e3a0, leaf::word_at_c(e, at.script));
    let name = name_of(e, at.script.addr());
    e.call(SCRIPT_LOG, &args![format, at.line, name, form_id]);
}

/// The value of `size` expression bytes at `cursor`, with a fresh `CExpression` on the stack; `f`
/// sees the expression and the value before its destructor runs.
fn with_expression<R>(
    e: &mut Engine,
    at: &Line,
    cursor: u32,
    size: i16,
    f: impl FnOnce(&mut Engine, Ptr, f64) -> R,
) -> R {
    e.with_stack(0x710, |e, expression| {
        new_expression(e, expression);
        let container = e.mem.u32(at.this);
        let locals = e.mem.u32(at.this + 8);
        let value = typed_call!(
            scripts::FAMILY,
            0x0059_3db0,
            scripts::expression::eval(
                e,
                expression,
                Ptr::new(at.data.wrapping_add(cursor)),
                at.reference.addr(),
                container,
                at.script.addr(),
                locals,
                i32::from(size),
                u8::from(at.parse_only)
            )
        );
        let result = f(e, expression, value);
        e.call(EXPRESSION_DESTRUCT, &args![expression]);
        result
    })
}

/// `FLD` and `FSTP qword`: a signalling NaN comes out quiet.
#[inline]
fn reload_f64(value: f64) -> f64 {
    let bits = value.to_bits();
    if value.is_nan() && bits & 0x0008_0000_0000_0000 == 0 {
        f64::from_bits(bits | 0x0008_0000_0000_0000)
    } else {
        value
    }
}

/// A double stored as a float (`FSTP dword`).
#[allow(clippy::cast_possible_truncation)]
#[inline]
fn to_float(value: f64) -> f32 {
    value as f32
}

/// The low word of a `_ftol2` result.
#[inline]
fn truncated(e: &mut Engine, value: f64) -> u32 {
    let long = e.call(FLOAT_TO_LONG, &args![value]).u64();
    long_low(long)
}

#[allow(clippy::cast_possible_truncation)]
#[inline]
fn long_low(long: u64) -> u32 {
    long as u32
}

/// `set` (0x15): the variable's kind (`G` a global; `r` and an index first name the referenced
/// object whose variable it is), its index (not for `G`), the expression's length and bytes. A
/// reference's variables are its script variables, a quest's its locals. Floats (`f`) take the
/// value, integers (`l`, `s`) its truncation; a global takes it as a float. A variable set on a
/// reference, or on the line's reference, marks it changed (its slot +0x48 with 0x80000000) unless
/// it is deleted or its process keeps it. While a console command runs, the new value is echoed.
fn evaluate_set(e: &mut Engine, at: &Line) -> bool {
    let data = at.data;
    let mut cursor = at.position;
    let mut kind = e.mem.u8(data.wrapping_add(cursor));
    cursor = cursor.wrapping_add(1);
    let mut variable = 0i16;
    let mut target_locals = e.mem.u32(at.this + 8);
    let mut object = 0u32;
    if kind == b'G' || kind == b'r' {
        let index = e.mem.i16(data.wrapping_add(cursor));
        cursor = cursor.wrapping_add(2);
        if kind == b'r' {
            kind = e.mem.u8(data.wrapping_add(cursor));
            cursor = cursor.wrapping_add(1);
        }
        let locals = e.mem.u32(at.this + 8);
        object = typed_call!(
            scripts::FAMILY,
            0x005a_c4f0,
            scripts::value::referenced_object(e, at.script, i32::from(index) as u32, locals)
        );
        if !at.parse_only {
            if kind == b'r' || object == 0 || e.mem.u32(object + 8) == 0 {
                log_line_error(e, at, SET_NO_REFERENCE);
                return false;
            }
            let form = e.mem.u32(object + 8);
            let as_reference = e
                .call(
                    DYNAMIC_CAST,
                    &args![form, 0u32, RTTI_FORM, RTTI_REFERENCE, 0u32],
                )
                .u32();
            if as_reference != 0 {
                target_locals = e
                    .call(REFERENCE_SCRIPT_VARIABLES, &args![as_reference])
                    .u32();
            } else {
                let form = e.mem.u32(object + 8);
                let as_quest = e
                    .call(
                        DYNAMIC_CAST,
                        &args![form, 0u32, RTTI_FORM, RTTI_QUEST, 0u32],
                    )
                    .u32();
                if as_quest != 0 {
                    target_locals = e.call(QUEST_SCRIPT_LOCALS, &args![as_quest]).u32();
                }
            }
            if target_locals == 0 {
                log_line_error(e, at, SET_NO_LOCALS);
                return false;
            }
        }
    }
    if kind != b'G' {
        variable = e.mem.i16(data.wrapping_add(cursor));
        cursor = cursor.wrapping_add(2);
    }
    let size = e.mem.i16(data.wrapping_add(cursor));
    cursor = cursor.wrapping_add(2);
    with_expression(e, at, cursor, size, |e, expression, value| {
        let code = typed_call!(leaf::FAMILY, 0x0055_9450, leaf::word_at_0(e, expression));
        if code != 0 {
            let code = typed_call!(leaf::FAMILY, 0x0055_9450, leaf::word_at_0(e, expression));
            let error_name = e
                .mem
                .u32(EXPRESSION_ERROR_NAMES.wrapping_add(code.wrapping_mul(4)));
            let name = name_of(e, at.script.addr());
            e.call(
                SCRIPT_LOG,
                &args![SET_EXPRESSION_ERROR, name, at.line, error_name],
            );
            // 005e1a23: JIP LN NVSE's O7 seam jumps past the clear.
            if !jip_keeps_failed_script(e, 0x005e_1a23) {
                let header = header_of(e, at.script);
                e.mem.set_u32(header + 8, 0);
            }
            return false;
        }
        if at.parse_only {
            return true;
        }
        if kind == b'G' {
            if object == 0 || e.mem.u32(object + 8) == 0 {
                log_line_error(e, at, SET_NO_GLOBAL);
                return false;
            }
            let single = to_float(value);
            let global = e.mem.u32(object + 8);
            e.call(SET_GLOBAL, &args![global, single]);
            if console_echo(e) {
                e.call(CONSOLE_PRINT, &args![SET_GLOBAL_ECHO, f64::from(single)]);
            }
            return true;
        }
        if target_locals == 0 {
            log_line_error(e, at, SET_NO_TARGET_LOCALS);
            return false;
        }
        let index = i32::from(variable);
        match kind {
            b'f' => {
                let stored = reload_f64(value);
                set_variable(e, target_locals, index, stored);
                if console_echo(e) {
                    let master = e.mem.u32(target_locals);
                    let name = e.call(VARIABLE_NAME, &args![master, index]).u32();
                    e.call(CONSOLE_PRINT, &args![SET_FLOAT_ECHO, name, stored]);
                }
            }
            b'l' | b's' => {
                let integer = truncated(e, value) as i32;
                set_variable(e, target_locals, index, f64::from(integer));
                if console_echo(e) {
                    let integer = truncated(e, value);
                    let master = e.mem.u32(target_locals);
                    let name = e.call(VARIABLE_NAME, &args![master, index]).u32();
                    e.call(CONSOLE_PRINT, &args![SET_INTEGER_ECHO, name, integer]);
                }
            }
            _ => {}
        }
        let has_form = object != 0 && e.mem.u32(object + 8) != 0;
        if has_form || !at.reference.is_null() {
            let mut target = 0u32;
            if object != 0 {
                let form = e.mem.u32(object + 8);
                if form != 0 && e.vcall(form, 0xf0, &args![]).bool() {
                    target = e.mem.u32(object + 8);
                }
            } else {
                target = at.reference.addr();
            }
            if target != 0 && !e.call(IS_DELETED, &args![target]).bool() {
                let process = e.call(PROCESS_OF, &args![target]).u32();
                let keep = process != 0 && {
                    let process = e.call(PROCESS_OF, &args![target]).u32();
                    e.call(PROCESS_KEEPS, &args![process]).bool()
                };
                if !keep {
                    e.vcall(target, 0x48, &args![0x8000_0000u32]);
                }
            }
        }
        true
    })
}

/// The condition of `if` and `elseif` (0x16, 0x18): the jump count and the expression's length,
/// then the expression. When no branch of this level was taken yet, a condition other than 0.0
/// takes this one (bit 2); otherwise the jump count is added to `*skip`. `parse_only` reads the
/// expression and marks the level taken. An expression error reports, stores error code 6 and the
/// expression's code (+0x1c), clears the script's data size (unless JIP keeps it) and fails.
fn evaluate_condition(e: &mut Engine, at: &Line) -> bool {
    let data = at.data;
    let count = e.mem.i16(data.wrapping_add(at.position));
    let size = e.mem.i16(data.wrapping_add(at.position).wrapping_add(2));
    let cursor = at.position.wrapping_add(4);
    let this = at.this;
    let level = |e: &Engine| if_level(this, e.mem.u32(this + 0x20));
    let add_skip = |e: &Engine| {
        let skipped = e.mem.u32(at.skip).wrapping_add(i32::from(count) as u32);
        e.mem.set_u32(at.skip, skipped);
    };
    let parse = at.parse_only;
    e.with_stack(0x710, |e, expression| {
        new_expression(e, expression);
        let container = e.mem.u32(this);
        let locals = e.mem.u32(this + 8);
        let evaluate = |e: &mut Engine| {
            typed_call!(
                scripts::FAMILY,
                0x0059_3db0,
                scripts::expression::eval(
                    e,
                    expression,
                    Ptr::new(data.wrapping_add(cursor)),
                    at.reference.addr(),
                    container,
                    at.script.addr(),
                    locals,
                    i32::from(size),
                    u8::from(parse)
                )
            )
        };
        if parse {
            evaluate(e);
            let slot = level(e);
            e.mem.set_u32(slot, e.mem.u32(slot) | 2);
        } else if e.mem.u32(level(e)) & 2 == 0 {
            let value = evaluate(e);
            if value == e.mem.f64(CONDITION_FALSE) {
                add_skip(e);
            } else {
                let slot = level(e);
                e.mem.set_u32(slot, e.mem.u32(slot) | 2);
            }
        } else {
            add_skip(e);
        }
        let succeeded =
            if typed_call!(leaf::FAMILY, 0x0055_9450, leaf::word_at_0(e, expression)) != 0 {
                e.call(SET_ERROR_CODE, &args![this, at.script, 6u32]);
                let name = name_of(e, at.script.addr());
                e.call(SCRIPT_LOG, &args![CONDITION_ERROR, name, at.line]);
                // 005e1f0b: JIP LN NVSE's O7 seam jumps past the clear.
                if !jip_keeps_failed_script(e, 0x005e_1f0b) {
                    let header = header_of(e, at.script);
                    e.mem.set_u32(header + 8, 0);
                }
                let code = typed_call!(leaf::FAMILY, 0x0055_9450, leaf::word_at_0(e, expression));
                e.mem.set_u32(this + 0x1c, code);
                false
            } else {
                true
            };
        e.call(EXPRESSION_DESTRUCT, &args![expression]);
        succeeded
    })
}

/// Formats a message into a temporary string and prints it on the console while a console command
/// runs, or logs it, then frees the string.
fn report_through_string(e: &mut Engine, words: &[u32]) {
    e.with_stack(8, |e, string| {
        e.call(STRING_CONSTRUCT, &args![string]);
        let mut all = vec![string.addr()];
        all.extend_from_slice(words);
        e.call(STRING_PRINTF, &all);
        let printer = if console_echo(e) {
            CONSOLE_PRINT
        } else {
            SCRIPT_LOG
        };
        let characters = typed_call!(leaf::FAMILY, 0x0055_9450, leaf::word_at_0(e, string));
        let text = e.call(STRING_TEXT, &args![characters, 0u32]).u32();
        e.call(printer, &args![text]);
        e.call(STRING_DESTRUCT, &args![string]);
    });
}

/// A command line: its record (unknown is reported), the needs-a-reference byte (+0x10), the
/// parameters (+0x14). `parse_only` parses the parameters when the line has any, and for opcode
/// 0x1059 the counted operands after them and four more bytes. Otherwise the handler (+0x18) runs
/// with the eight command words, the "references changed" byte (+0x25) copied to the runner first.
/// A missing or failing handler is reported.
fn evaluate_command(e: &mut Engine, at: &Line, command: i32) -> bool {
    let definition = e.call(GET_FUNCTION_DEFINITION, &args![command]).u32();
    if definition == 0 {
        let name = name_of(e, at.script.addr());
        report_through_string(e, &[COMMAND_UNKNOWN, command as u32, name]);
        return false;
    }
    let parameters = e.mem.u32(definition + 0x14);
    if e.mem.u8(definition + 0x10) != 0 && at.reference.is_null() && !at.parse_only {
        let command_name = e.mem.u32(definition);
        let name = name_of(e, at.script.addr());
        report_through_string(e, &[COMMAND_NEEDS_REFERENCE, name, at.line, command_name]);
        return false;
    }
    let data = at.data;
    e.with_stack(16, |e, work| {
        let cursor = work.addr();
        let result = cursor + 8;
        e.mem.set_u32(cursor, at.position);
        let container = e.mem.u32(at.this);
        let locals = e.mem.u32(at.this + 8);
        if at.parse_only {
            if parameters != 0 && e.mem.u32(at.length) != 0 {
                e.call(
                    PARSE_PARAMETERS,
                    &args![
                        parameters,
                        data,
                        cursor,
                        at.reference,
                        container,
                        at.script,
                        locals
                    ],
                );
                if command == PARSE_COUNTED_COMMAND {
                    let at_count = e.mem.u32(cursor);
                    let count = e.mem.i16(data.wrapping_add(at_count));
                    e.mem.set_u32(cursor, at_count.wrapping_add(2));
                    let mut operand = 0i16;
                    while operand < count {
                        let container = e.mem.u32(at.this);
                        let locals = e.mem.u32(at.this + 8);
                        e.call(
                            READ_OPERAND,
                            &args![
                                result,
                                data,
                                cursor,
                                at.reference,
                                container,
                                at.script,
                                locals,
                                1u32
                            ],
                        );
                        operand = operand.wrapping_add(1);
                    }
                    let after = e.mem.u32(cursor).wrapping_add(4);
                    e.mem.set_u32(cursor, after);
                }
            }
            return true;
        }
        let handler = e.mem.u32(definition + 0x18);
        if e.mem.u8(definition + 0x25) != 0 {
            e.mem.set_u8(at.this + 0xa1, 1);
        }
        let succeeded = handler != 0 && {
            // The seam before the handler (`005e2329`, ADR-0016): the opcode, the record and the
            // result's address.
            e.site(0x005e_2329, &[command as u32, definition, result]);
            e.call(
                handler,
                &args![
                    parameters,
                    data,
                    at.reference,
                    container,
                    at.script,
                    locals,
                    result,
                    cursor
                ],
            )
            .bool()
        };
        if !succeeded {
            let form_id = typed_call!(leaf::FAMILY, 0x0084_e3a0, leaf::word_at_c(e, at.script));
            let name = name_of(e, at.script.addr());
            let command_name = e.mem.u32(definition);
            e.call(
                SCRIPT_LOG,
                &args![COMMAND_FAILED, command_name, at.line, name, form_id],
            );
            return false;
        }
        true
    })
}
