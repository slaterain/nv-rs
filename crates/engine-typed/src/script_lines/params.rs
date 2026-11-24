//! The script functions xNVSE wraps that every command line and every script event goes through:
//! a command's parameter read (`Script::ParseParameters`, `005accb0`), the script event mark
//! (`Script::SetActionFlag`, `005ac750`) and the locals' deleting destructor (`0041af70`). In this
//! family because a wrap over any of them clears its family's direct bit: every call into them
//! stays by address, and the wrap's own call reaches the typed body here.

use crate::crt;
use crate::scripts;
use engine_core::abi::Ret;
use engine_core::extend::Hook;
use engine_core::ptr::Ptr;
use engine_core::{args, typed_call, Engine};

/// The parameter types' table (8 bytes each: whether the type is a referenced object).
const PARAMETER_TYPE_TABLE: u32 = 0x0118_cdd5;
const FORM_TYPE_OF: u32 = 0x0040_1170;
const FORM_FLAG_BIT_3: u32 = 0x0040_13e0;
const MEMCPY: u32 = 0x0040_1460;
const CONTAINER_CAN_HOLD_TYPE: u32 = 0x0048_1f30;
const DYNAMIC_CAST: u32 = 0x00ec_43fb;
const RTTI_FORM: u32 = 0x0118_3028;
const RTTI_TARGET_FORM_7: u32 = 0x0118_3060;
const RTTI_TARGET_FORM_11: u32 = 0x0118_3140;
const RTTI_TARGET_FORM_31: u32 = 0x0118_37c4;
/// The operand reader, by address (`overridable.tsv`).
const READ_OPERAND: u32 = 0x005a_c7a0;
const SCRIPT_LOG: u32 = 0x005b_5e40;
const FORMAT_PARAMETER_TYPE_UNIMPLEMENTED: u32 = 0x0103_73a8;
/// The extra data list's script locals, the locals' destructor and `operator delete`.
const EXTRA_LIST_SCRIPT_LOCALS: u32 = 0x0041_8830;
const SCRIPT_LOCALS_DESTRUCTOR: u32 = 0x005a_8bc0;
const OPERATOR_DELETE: u32 = 0x0040_1030;

#[inline]
fn form_type(e: &mut Engine, form: u32) -> u32 {
    u32::from(e.call(FORM_TYPE_OF, &args![form]).u8())
}

#[inline]
fn compiled(e: &mut Engine, script: u32) -> bool {
    e.call(FORM_FLAG_BIT_3, &args![script]).bool()
}

#[inline]
fn take_i16(e: &Engine, code: u32, offset: u32) -> i16 {
    let at = e.mem.u32(offset);
    let value = e.mem.i16(code.wrapping_add(at));
    e.mem.set_u32(offset, at.wrapping_add(2));
    value
}

#[inline]
fn take_u8(e: &Engine, code: u32, offset: u32) -> u8 {
    let at = e.mem.u32(offset);
    let value = e.mem.u8(code.wrapping_add(at));
    e.mem.set_u32(offset, at.wrapping_add(1));
    value
}

/// A 16-bit value widened as the exe's `MOVSX` does.
#[inline]
fn widen(value: i16) -> u32 {
    i32::from(value) as u32
}

/// Whether a form-type byte is one of the 0x3A..=0x40 and 0x69 types that carry script variables.
#[inline]
fn is_scripted_reference_type(form_type: u32) -> bool {
    form_type >= 0x3a && (form_type <= 0x40 || form_type == 0x69)
}

/// The next output pointer of the caller's (the exe reads the next stack word; a caller always
/// pushes one per parameter, so a missing one is a stop on both sides).
#[inline]
fn next_output(outputs: &[u32], used: &mut usize) -> u32 {
    let target = *outputs
        .get(*used)
        .unwrap_or_else(|| panic!("ParseParameters: output pointer {} missing", *used));
    *used += 1;
    target
}

#[inline]
fn store_form(e: &mut Engine, outputs: &[u32], used: &mut usize, form: u32) -> bool {
    let target = next_output(outputs, used);
    e.mem.set_u32(target, form);
    e.mem.u32(target) != 0
}

#[inline]
fn cast_form(e: &mut Engine, form: u32, target: u32) -> u32 {
    e.call(DYNAMIC_CAST, &args![form, 0u32, RTTI_FORM, target, 0u32])
        .u32()
}

/// The form types a referenced-object parameter type accepts by a plain test of the form type.
#[inline]
fn exact_reference_type(parameter: u32) -> Option<(u32, u32)> {
    let exact = match parameter {
        0x9 => 0x39,
        0xc => 0xd,
        0xd => 0x45,
        0xe => 0x47,
        0xf => 0xc,
        0x10 => 0x7,
        0x11 => 0x8,
        0x13 => 0x6,
        0x1b => 0x41,
        0x1d => 0x49,
        0x1e => 0x4a,
        0x21 => 0x35,
        0x22 => 0x2a,
        0x24 => 0x4f,
        0x25 => 0x55,
        0x26 => 0x5,
        0x27 => 0x56,
        0x28 => 0x31,
        0x2a => 0x54,
        0x2b => 0x53,
        0x2f => 0x61,
        0x31 => 0x62,
        0x36 => 0x66,
        0x3a => 0x2d,
        0x3b => 0x2c,
        0x3c => 0x34,
        0x3e => 0x68,
        0x3f => 0x6d,
        0x40 => 0x6c,
        0x41 => 0x71,
        0x42 => 0x74,
        0x43 => 0x73,
        0x44 => 0x75,
        0x45 => 0x37,
        0x6 => return Some((0x3b, 0x3c)),
        _ => return None,
    };
    Some((exact, exact))
}

/// Clears the next output pointer's word, then stores `form` there when `accepts` says so
/// (asked after the clear, as the exe asks), and answers whether a form was stored.
#[inline]
fn store_if(
    e: &mut Engine,
    outputs: &[u32],
    used: &mut usize,
    form: u32,
    accepts: impl FnOnce(&mut Engine) -> bool,
) -> bool {
    let target = next_output(outputs, used);
    e.mem.set_u32(target, 0);
    if accepts(e) {
        e.mem.set_u32(target, form);
    }
    e.mem.u32(target) != 0
}

/// One referenced-object parameter: stores the (non-null) form through the next output pointer
/// when the type accepts it and says whether it did; `None` for a type the game does not implement.
fn reference_parameter(
    e: &mut Engine,
    parameter: u32,
    form: u32,
    outputs: &[u32],
    used: &mut usize,
) -> Option<bool> {
    if let Some((low, high)) = exact_reference_type(parameter) {
        let kind = form_type(e, form);
        if kind >= low && kind <= high {
            return Some(store_form(e, outputs, used, form));
        }
        return Some(false);
    }
    let accepted = match parameter {
        3 => {
            let kind = form_type(e, form);
            if e.call(CONTAINER_CAN_HOLD_TYPE, &args![kind]).bool() {
                store_if(e, outputs, used, form, |e| {
                    form != 0 && e.vcall(form, 0xe8, &args![]).bool()
                })
            } else {
                false
            }
        }
        4 | 0x18 | 0x1a => {
            if is_scripted_reference_type(form_type(e, form)) {
                store_if(e, outputs, used, form, |e| {
                    form != 0 && is_scripted_reference_type(form_type(e, form))
                })
            } else {
                false
            }
        }
        7 => {
            let target = next_output(outputs, used);
            let cast = cast_form(e, form, RTTI_TARGET_FORM_7);
            e.mem.set_u32(target, cast);
            if cast != 0 {
                true
            } else {
                let quest = if form != 0 && form_type(e, form) == 0x19 {
                    form
                } else {
                    0
                };
                e.mem.set_u32(target, quest);
                quest != 0
                    && typed_call!(
                        engine_core::typed::leaf::FAMILY,
                        0x0072_6070,
                        engine_core::typed::leaf::word_at_4(e, quest + 0x74)
                    ) != 0
            }
        }
        0xb | 0x1f => {
            let target = next_output(outputs, used);
            let wanted = if parameter == 0xb {
                RTTI_TARGET_FORM_11
            } else {
                RTTI_TARGET_FORM_31
            };
            let cast = cast_form(e, form, wanted);
            e.mem.set_u32(target, cast);
            cast != 0
        }
        0x14 => {
            let kind = form_type(e, form);
            if kind == 0x27 || form_type(e, form) == 0x55 {
                store_form(e, outputs, used, form)
            } else {
                false
            }
        }
        0x15 => store_if(e, outputs, used, form, |e| {
            form != 0 && e.vcall(form, 0xe8, &args![]).bool()
        }),
        0x19 => {
            let target = next_output(outputs, used);
            e.mem.set_u32(target, 0);
            if form != 0 && (0x2a..=0x2b).contains(&form_type(e, form)) {
                e.mem.set_u32(target, form);
            }
            e.mem.u32(target) != 0
        }
        0x23 | 0x38 | 0x39 => {
            let target = next_output(outputs, used);
            e.mem.set_u32(target, 0);
            if form != 0 {
                let kind = form_type(e, form);
                let (first, second) = match parameter {
                    0x23 => (8, 0x2a),
                    0x38 => (0x2a, 0x2d),
                    _ => (0x2b, 0x2c),
                };
                if kind == first || kind == second {
                    e.mem.set_u32(target, form);
                }
            }
            e.mem.u32(target) != 0
        }
        0x32 => {
            let kind = form_type(e, form);
            if e.call(CONTAINER_CAN_HOLD_TYPE, &args![kind]).bool() || form_type(e, form) == 0x55 {
                store_form(e, outputs, used, form)
            } else {
                false
            }
        }
        0x35 => {
            let target = next_output(outputs, used);
            e.mem.set_u32(target, 0);
            if form != 0 && (e.vcall(form, 0xe8, &args![]).bool() || form_type(e, form) == 0x55) {
                e.mem.set_u32(target, form);
            }
            e.mem.u32(target) != 0
        }
        0x3d => form != 0 && store_form(e, outputs, used, form),
        _ => return None,
    };
    Some(accepted)
}

/// An operand read into a fresh double (the operand reader, by address), `None` when it fails.
#[inline]
#[allow(clippy::too_many_arguments)]
fn read_value(
    e: &mut Engine,
    code: u32,
    offset: u32,
    reference: u32,
    container: u32,
    script: u32,
    locals: u32,
) -> Option<f64> {
    e.with_stack(8, |e, slot| {
        e.mem.set_f64(slot.addr(), 0.0);
        let ok = e
            .call(
                READ_OPERAND,
                &args![slot, code, offset, reference, container, script, locals, 1u32],
            )
            .bool();
        ok.then(|| e.mem.f64(slot.addr()))
    })
}

/// A double stored as a float (`FSTP dword`).
#[allow(clippy::cast_possible_truncation)]
#[inline]
fn to_float(value: f64) -> f32 {
    value as f32
}

/// The low byte of a 16-bit value (`MOV byte`).
#[allow(clippy::cast_possible_truncation)]
#[inline]
fn low_byte(value: i16) -> u8 {
    value as u8
}

/// The low word of `_ftol2`'s answer.
#[allow(clippy::cast_possible_truncation)]
#[inline]
fn low_word(value: u64) -> u32 {
    value as u32
}

/// `Script::ParseParameters` (`cdecl`, varargs): reads a command's parameters from the compiled
/// data at `*offset` (`code`): a 16-bit count, then each parameter by the type in the command's
/// list (12 bytes an entry, the type at +4), stored through the next of the caller's output
/// pointers, but only when the script is compiled (its form flag bit 3; otherwise only skipped).
/// Referenced-object types (the table's byte) are an `r` marker and an index into the script's
/// referenced objects, whose form must exist and suit the type; the rest are a string (0), an
/// integer or float operand (1, 0x17, 2), a 16-bit integer (5, 0xa, 0x12, 0x1c, 0x29, 0x33, 0x34,
/// 0x37), a byte (8) or a 16-bit value stored as a byte (0x20). Any other type, a missing marker or
/// a rejected form answers false (an unimplemented referenced-object type is logged).
///
/// The seam at `005acd0d` (xNVSE's inline expression, the parameter starting with `0xFFFF`): an
/// answer ends the read with false for xNVSE's reader to take the command.
#[allow(clippy::too_many_arguments)]
pub fn parse_parameters(
    e: &mut Engine,
    parameters: u32,
    code: u32,
    offset: u32,
    reference: u32,
    container: u32,
    script: u32,
    locals: u32,
    outputs: &[u32],
) -> bool {
    if parameters == 0 {
        return false;
    }
    let mut used = 0usize;
    let count = take_i16(e, code, offset);
    let mut index: i16 = 0;
    while index < count {
        if e.mem.u16(code.wrapping_add(e.mem.u32(offset))) == 0xffff
            && e.site(0x005a_cd0d, &[code, offset]).is_some()
        {
            return false;
        }
        let entry = parameters.wrapping_add(widen(index).wrapping_mul(12));
        let parameter = e.mem.u32(entry.wrapping_add(4));
        let is_reference = e
            .mem
            .u8(PARAMETER_TYPE_TABLE.wrapping_add(parameter.wrapping_mul(8)))
            != 0;
        if is_reference {
            if take_u8(e, code, offset) != b'r' {
                return false;
            }
            let object_index = take_i16(e, code, offset);
            let object = typed_call!(
                scripts::FAMILY,
                0x005a_c4f0,
                scripts::value::referenced_object(e, Ptr::new(script), widen(object_index), locals)
            );
            if !compiled(e, script) || object == 0 || e.mem.u32(object + 8) == 0 {
                return false;
            }
            let form = e.mem.u32(object + 8);
            match reference_parameter(e, parameter, form, outputs, &mut used) {
                Some(true) => {}
                Some(false) => return false,
                None => {
                    e.call(
                        SCRIPT_LOG,
                        &args![FORMAT_PARAMETER_TYPE_UNIMPLEMENTED, parameter],
                    );
                    return false;
                }
            }
        } else {
            match parameter {
                0 => {
                    let length = take_i16(e, code, offset);
                    if compiled(e, script) {
                        let target = next_output(outputs, &mut used);
                        let at = e.mem.u32(offset);
                        e.call(MEMCPY, &args![target, code.wrapping_add(at), widen(length)]);
                        e.mem.set_u8(target.wrapping_add(widen(length)), 0);
                    }
                    let at = e.mem.u32(offset);
                    e.mem.set_u32(offset, at.wrapping_add(widen(length)));
                }
                1 | 0x17 => {
                    let Some(value) =
                        read_value(e, code, offset, reference, container, script, locals)
                    else {
                        return false;
                    };
                    if compiled(e, script) {
                        let target = next_output(outputs, &mut used);
                        let integer =
                            typed_call!(crt::FAMILY, 0x00ec_62c0, crt::math::ftol2_sse(e, value));
                        e.mem.set_u32(target, low_word(integer));
                    }
                }
                2 => {
                    let Some(value) =
                        read_value(e, code, offset, reference, container, script, locals)
                    else {
                        return false;
                    };
                    if compiled(e, script) {
                        let target = next_output(outputs, &mut used);
                        e.mem.set_f32(target, to_float(value));
                    }
                }
                5 | 0xa | 0x12 | 0x1c | 0x29 | 0x33 | 0x34 | 0x37 => {
                    let value = take_i16(e, code, offset);
                    if compiled(e, script) {
                        let target = next_output(outputs, &mut used);
                        e.mem.set_i32(target, i32::from(value));
                    }
                }
                8 => {
                    if compiled(e, script) {
                        let target = next_output(outputs, &mut used);
                        let at = e.mem.u32(offset);
                        let value = e.mem.u8(code.wrapping_add(at));
                        e.mem.set_u8(target, value);
                    }
                    let at = e.mem.u32(offset);
                    e.mem.set_u32(offset, at.wrapping_add(1));
                }
                0x20 => {
                    let value = take_i16(e, code, offset);
                    if compiled(e, script) {
                        let target = next_output(outputs, &mut used);
                        e.mem.set_u8(target, low_byte(value));
                    }
                }
                _ => return false,
            }
        }
        index = index.wrapping_add(1);
    }
    true
}

// Typed from 005accb0
/// The registered form of [`parse_parameters`]: seven fixed words, then the output pointers,
/// however many the caller pushed.
pub fn parse_parameters_entry(e: &mut Engine, words: &[u32]) -> Ret {
    assert!(
        words.len() >= 7,
        "ParseParameters needs 7 argument words, got {}",
        words.len()
    );
    let ok = parse_parameters(
        e,
        words[0],
        words[1],
        words[2],
        words[3],
        words[4],
        words[5],
        words[6],
        &words[7..],
    );
    Ret {
        eax: u32::from(ok),
        ..Ret::default()
    }
}

// Typed from 005ac750
/// `Script::SetActionFlag`: for an extra data list with script locals, sets `flags` on the
/// action of `form` there (`ScriptLocals::SetActionFlag`, the scripts family's) and answers its
/// result; otherwise false. JIP LN's three seams stay at their points: before the locals are read
/// (`005ac763`, its answer is the result), at the locals' call (`005ac782`, its answer replaces the
/// call) and before the result returns on every path (`005ac78a`, the hit events).
pub fn set_action_flag(e: &mut Engine, form: u32, extra_list: Ptr, flags: u32) -> bool {
    let mut result = false;
    'body: {
        if extra_list.is_null() {
            break 'body;
        }
        if e.hook_users(Hook::MarkScriptEvent) != 0 {
            if let Some(answer) = e.site(0x005a_c763, &[form, extra_list.addr(), flags]) {
                result = answer != 0;
                break 'body;
            }
        }
        if e.call(EXTRA_LIST_SCRIPT_LOCALS, &args![extra_list]).u32() != 0 {
            let locals = e.call(EXTRA_LIST_SCRIPT_LOCALS, &args![extra_list]).u32();
            let merged = if e.hook_users(Hook::MergeEventMask) != 0 {
                e.site(0x005a_c782, &[locals, form, flags])
            } else {
                None
            };
            result = match merged {
                Some(answer) => answer != 0,
                None => typed_call!(
                    scripts::FAMILY,
                    0x005a_8e20,
                    scripts::run::set_locals_action_flag(e, Ptr::new(locals), form, flags)
                ),
            };
        }
    }
    if e.hook_users(Hook::OnHitEvent) != 0 {
        e.site(
            0x005a_c78a,
            &[form, extra_list.addr(), flags, u32::from(result)],
        );
    }
    result
}

// Typed from 0041af70
/// `ScriptLocals`' deleting destructor: the destructor, then `operator delete` when bit 0 of
/// `flags` is set. Answers `this`.
pub fn delete_locals(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    e.call(SCRIPT_LOCALS_DESTRUCTOR, &args![this]);
    if flags & 1 != 0 {
        e.call(OPERATOR_DELETE, &args![this]);
    }
    this
}
