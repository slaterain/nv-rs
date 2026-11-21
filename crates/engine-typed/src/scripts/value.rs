//! The operand reader (`005ac7a0`) and the two lookups it makes for each operand: the script's
//! referenced object by index (`Script::GetReferencedObject`) and a variable of a script's locals
//! by id (`ScriptLocals::GetVariable`). Both keep their last answer in the running thread's TLS
//! block, as the exe does.

use super::FAMILY;
use crate::crt;
use engine_core::ptr::Ptr;
use engine_core::typed::leaf;
use engine_core::{args, typed_call, Engine};

/// `Script`'s referenced-object count (its header's +4, at +0x1c).
const REF_OBJECT_COUNT: u32 = 0x1c;
/// The TLS block's caches: the last variable (its address, id and locals) and the last referenced
/// object (the object, its index, the locals and the script).
const TLS_LAST_VARIABLE: u32 = 0x270;
const TLS_LAST_VARIABLE_ID: u32 = 0x274;
const TLS_LAST_LOCALS: u32 = 0x278;
const TLS_LAST_REF_OBJECT: u32 = 0x27c;
const TLS_LAST_REF_INDEX: u32 = 0x280;
const TLS_LAST_REF_LOCALS: u32 = 0x284;
const TLS_LAST_REF_SCRIPT: u32 = 0x288;

/// The script's referenced-object list (`Script`'s +0x44 list, by address).
const SCRIPT_REF_OBJECT_LIST: u32 = 0x005d_43c0;
const GET_NUMERIC_ID_FROM_DOUBLE: u32 = 0x005a_cc90;
const PUT_NUMERIC_ID_IN_DOUBLE: u32 = 0x005a_cc70;
const LOOKUP_FORM_BY_ID: u32 = 0x0048_39c0;
const FORM_TYPE_OF: u32 = 0x0040_1170;
const FORM_FLAG_BIT_3: u32 = 0x0040_13e0;
const GLOBAL_VALUE_OF: u32 = 0x0052_6ac0;
const QUEST_SCRIPT_LOCALS: u32 = 0x0059_e300;
const REFERENCE_SCRIPT_VARIABLES: u32 = 0x0056_73e0;
const EXTRA_LIST_REFERENCE_POINTER: u32 = 0x0041_c8d0;
const INVENTORY_ITEM_OF: u32 = 0x0057_6260;
const ITEM_SCRIPT_LOCALS: u32 = 0x004b_dea0;
const RELEASE_ITEM: u32 = 0x0044_59e0;
const GET_FUNCTION_DEFINITION: u32 = 0x005b_1120;
/// `Script::ParseParameters`, by address: a built-in extender wraps it (ADR-0016).
const PARSE_PARAMETERS: u32 = 0x005a_ccb0;
/// The script log, the "variable not found" format, `"UNKNOWN"` and the line being run.
const SCRIPT_LOG: u32 = 0x005b_5e40;
const FORMAT_VARIABLE_NOT_FOUND: u32 = 0x0103_6d98;
const UNKNOWN_SCRIPT_NAME: u32 = 0x0101_5890;
const SCRIPT_LINE: u32 = 0x011c_ae50;

#[inline]
fn list_item(e: &mut Engine, node: u32) -> u32 {
    let slot = typed_call!(leaf::FAMILY, 0x0068_15c0, leaf::same_word(e, node));
    e.mem.u32(slot)
}

#[inline]
fn list_next(e: &mut Engine, node: u32) -> u32 {
    typed_call!(leaf::FAMILY, 0x0072_6070, leaf::word_at_4(e, node))
}

#[inline]
fn list_is_empty(e: &mut Engine, node: u32) -> bool {
    typed_call!(leaf::FAMILY, 0x0082_56d0, leaf::both_words_zero(e, node))
}

#[inline]
fn form_type(e: &mut Engine, form: u32) -> u32 {
    u32::from(e.call(FORM_TYPE_OF, &args![form]).u8())
}

#[inline]
fn form_id_of(e: &mut Engine, form: u32) -> u32 {
    typed_call!(leaf::FAMILY, 0x0084_e3a0, leaf::word_at_c(e, form))
}

/// Whether a form-type byte is one of the 0x3A..=0x40 and 0x69 types that carry script
/// variables.
#[inline]
fn is_scripted_reference_type(form_type: u32) -> bool {
    form_type >= 0x3a && (form_type <= 0x40 || form_type == 0x69)
}

/// Reads the 16-bit word at the parse offset and moves the offset past it.
#[inline]
fn take_i16(e: &Engine, code: u32, offset: u32) -> i16 {
    let at = e.mem.u32(offset);
    let value = e.mem.i16(code.wrapping_add(at));
    e.mem.set_u32(offset, at.wrapping_add(2));
    value
}

/// Reads the byte at the parse offset and moves the offset past it.
#[inline]
fn take_u8(e: &Engine, code: u32, offset: u32) -> u8 {
    let at = e.mem.u32(offset);
    let value = e.mem.u8(code.wrapping_add(at));
    e.mem.set_u32(offset, at.wrapping_add(1));
    value
}

// Typed from 005a9140
/// `ScriptLocals::GetVariable`: the value of the variable `id`. The last lookup is kept in the
/// TLS block (locals, id, variable); otherwise the locals' variable list (+0xc) is searched. A
/// missing variable is logged (with the name of `script`, `"UNKNOWN"` when null, and the line being
/// run) and reads 0.
pub fn get_variable(e: &mut Engine, this: Ptr, id: u32, script: Ptr) -> f64 {
    let tls = e.tls();
    let mut variable = 0u32;
    if e.mem.u32(tls + TLS_LAST_LOCALS) == this.addr()
        && e.mem.u32(tls + TLS_LAST_VARIABLE_ID) == id
    {
        variable = e.mem.u32(tls + TLS_LAST_VARIABLE);
    }
    if variable == 0 {
        let mut node = typed_call!(leaf::FAMILY, 0x0084_e3a0, leaf::word_at_c(e, this));
        while node != 0 && !list_is_empty(e, node) {
            let candidate = list_item(e, node);
            if e.mem.u32(candidate) == id {
                variable = candidate;
                break;
            }
            node = list_next(e, node);
        }
    }
    e.mem.set_u32(tls + TLS_LAST_LOCALS, this.addr());
    e.mem.set_u32(tls + TLS_LAST_VARIABLE_ID, id);
    e.mem.set_u32(tls + TLS_LAST_VARIABLE, variable);
    if variable != 0 {
        return e.mem.f64(variable + 8);
    }
    let name = if script.is_null() {
        UNKNOWN_SCRIPT_NAME
    } else {
        e.vcall(script.addr(), 0x130, &args![]).u32()
    };
    let line: u32 = e.global(SCRIPT_LINE);
    e.call(
        SCRIPT_LOG,
        &args![FORMAT_VARIABLE_NOT_FOUND, id, name, line],
    );
    0.0
}

// Typed from 005ac4f0
/// `Script::GetReferencedObject`: the `index`th (1-based) referenced object of the script, null
/// for 0 or past the count. The last answer is kept in the TLS block (script, index, locals,
/// object). For a new lookup, an object with a variable id has its form looked up again from the
/// numeric id stored in that variable of `locals`.
pub fn referenced_object(e: &mut Engine, this: Ptr, index: u32, locals: u32) -> u32 {
    if index == 0 {
        return 0;
    }
    if e.mem.u32(this.addr() + REF_OBJECT_COUNT) < index {
        return 0;
    }
    let tls = e.tls();
    if e.mem.u32(tls + TLS_LAST_REF_SCRIPT) == this.addr()
        && e.mem.u32(tls + TLS_LAST_REF_INDEX) == index
        && e.mem.u32(tls + TLS_LAST_REF_LOCALS) == locals
    {
        return e.mem.u32(tls + TLS_LAST_REF_OBJECT);
    }
    let mut position = 1u32;
    let mut node = e.call(SCRIPT_REF_OBJECT_LIST, &args![this]).u32();
    while node != 0 {
        if list_is_empty(e, node) || position >= index {
            break;
        }
        position += 1;
        node = list_next(e, node);
    }
    if node == 0 {
        return 0;
    }
    let object = list_item(e, node);
    let variable_id = e.mem.u32(object + 0xc);
    if variable_id != 0 && locals != 0 {
        let value = typed_call!(
            FAMILY,
            0x005a_9140,
            get_variable(e, Ptr::new(locals), variable_id, this)
        );
        let id = e.with_stack(12, |e, buffer| {
            e.mem.set_f64(buffer.addr(), value);
            e.mem.set_u32(buffer.addr() + 8, 0);
            e.call(
                GET_NUMERIC_ID_FROM_DOUBLE,
                &args![buffer.addr() + 8, buffer],
            );
            e.mem.u32(buffer.addr() + 8)
        });
        if id != 0 {
            let form = e.call(LOOKUP_FORM_BY_ID, &args![id]).u32();
            e.mem.set_u32(object + 8, form);
        }
    }
    e.mem.set_u32(tls + TLS_LAST_REF_SCRIPT, this.addr());
    e.mem.set_u32(tls + TLS_LAST_REF_LOCALS, locals);
    e.mem.set_u32(tls + TLS_LAST_REF_INDEX, index);
    e.mem.set_u32(tls + TLS_LAST_REF_OBJECT, object);
    object
}

// Typed from 005ac7a0
/// Reads one operand of a script's compiled data at `*offset` into the double at `out` (`cdecl`).
/// The first byte is its kind: `r`, `G` and `Z` name a referenced object (a 16-bit index): `G`
/// stores the object's global value, `Z` its form id as a numeric id, and `r` is followed by a
/// command marker `X` or by a variable, and makes the locals those of the referenced object (a
/// quest, a reference, or an inventory item of one). `n` is an immediate integer (4 bytes) and `z`
/// an immediate double (8). `X` is a command: a 16-bit opcode (and 16 bits skipped) looked up in the
/// command table; with `parse` set only its parameters are parsed and the answer is false;
/// otherwise its handler runs with the command signature's eight words. Any other marker (`f`, `l`
/// or `s`) is a 16-bit variable id read from the locals. False when the data cannot be read.
///
/// Seam `005acb9d` before a command's handler (ADR-0016): the opcode, the record and the result's
/// address.
#[allow(clippy::too_many_arguments)]
pub fn read_operand(
    e: &mut Engine,
    out: u32,
    code: u32,
    offset: u32,
    reference: u32,
    container: u32,
    script: u32,
    locals: u32,
    parse: u8,
) -> bool {
    let mut locals_to_use = locals;
    if locals_to_use == 0 {
        return false;
    }
    let mut object = 0u32;
    let mut parse = parse;
    if parse != 0 && e.call(FORM_FLAG_BIT_3, &args![script]).bool() {
        parse = 0;
    }
    let mut marker = take_u8(e, code, offset);
    match marker {
        b'G' | b'Z' | b'r' => {
            let index = i32::from(take_i16(e, code, offset)) as u32;
            object = typed_call!(
                FAMILY,
                0x005a_c4f0,
                referenced_object(e, Ptr::new(script), index, locals)
            );
            if object == 0 {
                return false;
            }
            let form = e.mem.u32(object + 8);
            if form == 0 && parse == 0 {
                if e.mem.u32(object + 0xc) != 0 {
                    e.mem.set_u64(out, 0);
                    return true;
                }
                return false;
            }
            if marker == b'G' {
                let value = e.call(GLOBAL_VALUE_OF, &args![form]).f64();
                e.mem.set_f64(out, value);
                return true;
            }
            if marker == b'Z' {
                let id = form_id_of(e, form);
                e.with_stack(4, |e, slot| {
                    e.mem.set_u32(slot.addr(), id);
                    e.call(PUT_NUMERIC_ID_IN_DOUBLE, &args![slot, out]);
                });
                return true;
            }
            marker = take_u8(e, code, offset);
            if marker != b'X' {
                let quest = if form != 0 && form_type(e, form) == 0x47 {
                    form
                } else {
                    0
                };
                if quest != 0 {
                    locals_to_use = e.call(QUEST_SCRIPT_LOCALS, &args![quest]).u32();
                } else {
                    let holder = if form != 0 && is_scripted_reference_type(form_type(e, form)) {
                        form
                    } else {
                        0
                    };
                    if holder != 0 {
                        let list = e.call(SCRIPT_REF_OBJECT_LIST, &args![holder]).u32();
                        if e.call(EXTRA_LIST_REFERENCE_POINTER, &args![list]).u32() != 0 {
                            let list = e.call(SCRIPT_REF_OBJECT_LIST, &args![holder]).u32();
                            let pointer = e.call(EXTRA_LIST_REFERENCE_POINTER, &args![list]).u32();
                            let id = form_id_of(e, holder);
                            let key =
                                typed_call!(leaf::FAMILY, 0x007a_f430, leaf::word_at_20(e, holder));
                            let item = e.call(INVENTORY_ITEM_OF, &args![pointer, key, id]).u32();
                            if item != 0 {
                                locals_to_use = e.call(ITEM_SCRIPT_LOCALS, &args![item]).u32();
                                e.call(RELEASE_ITEM, &args![item, 1u32]);
                            }
                        } else {
                            locals_to_use =
                                e.call(REFERENCE_SCRIPT_VARIABLES, &args![holder]).u32();
                        }
                    }
                }
                if locals_to_use == 0 {
                    return false;
                }
            }
        }
        b'n' => {
            let at = e.mem.u32(offset);
            let value = e.mem.i32(code.wrapping_add(at));
            e.mem.set_f64(out, f64::from(value));
            e.mem.set_u32(offset, at.wrapping_add(4));
            return true;
        }
        b'z' => {
            let at = e.mem.u32(offset);
            typed_call!(
                crt::FAMILY,
                0x0040_1460,
                crt::mem::memcpy_wrapper(e, Ptr::new(out), Ptr::new(code.wrapping_add(at)), 8)
            );
            e.mem.set_u32(offset, at.wrapping_add(8));
            return true;
        }
        _ => {}
    }
    if marker == b'X' {
        let function_id = i32::from(take_i16(e, code, offset)) as u32;
        let at = e.mem.u32(offset);
        e.mem.set_u32(offset, at.wrapping_add(2));
        let definition = e.call(GET_FUNCTION_DEFINITION, &args![function_id]).u32();
        if definition == 0 {
            return false;
        }
        let mut target = reference;
        if object != 0 && e.mem.u32(object + 8) != 0 {
            let form = e.mem.u32(object + 8);
            target = if is_scripted_reference_type(form_type(e, form)) {
                form
            } else {
                0
            };
        }
        // The command's record: needs-reference byte (+0x10), parameters (+0x14), handler (+0x18).
        let parameters = e.mem.u32(definition + 0x14);
        if parse != 0 {
            if parameters != 0 {
                e.call(
                    PARSE_PARAMETERS,
                    &args![
                        parameters,
                        code,
                        offset,
                        target,
                        container,
                        script,
                        locals_to_use
                    ],
                );
            }
            return false;
        }
        let needs_reference = e.mem.u8(definition + 0x10) != 0;
        if needs_reference && target == 0 {
            return false;
        }
        let handler = e.mem.u32(definition + 0x18);
        if handler == 0 {
            return false;
        }
        e.site(0x005a_cb9d, &[function_id, definition, out]);
        e.call(
            handler,
            &args![
                parameters,
                code,
                target,
                container,
                script,
                locals_to_use,
                out,
                offset
            ],
        )
        .bool()
    } else {
        let id = i32::from(take_i16(e, code, offset)) as u32;
        if matches!(marker, b'f' | b'l' | b's') {
            let value = typed_call!(
                FAMILY,
                0x005a_9140,
                get_variable(e, Ptr::new(locals_to_use), id, Ptr::new(script))
            );
            e.mem.set_f64(out, value);
            true
        } else {
            false
        }
    }
}
