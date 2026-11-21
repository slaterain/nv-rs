//! The differential test of every typed function of the scripts family (`engine_typed::scripts`)
//! against its translation: the expression evaluator, the number tests and the operand reader with
//! its two lookups.
//!
//! The expressions are generated: every operator on integer, double and mixed operands (doubles
//! from the harness's list and its float sweep), the text literals, every compiled operand kind
//! (`s l f r G Z n z X`, quoted texts), the errors, the long tokens, parse mode, xNVSE's two
//! changes to this code (its 0x500-byte token buffer with the moved frame, and its command call at
//! the seam `005949d4`), and JIP LN's double precision operators (the seam `00593fbc` behind its
//! patch bit). `real_scda_expressions` (opt-in, needs the game's data) runs every expression of
//! `FalloutNV.esm`'s compiled scripts the same way.
//!
//! What stands in: the CRT's `atol` and `atof` (decimal parsers), `strlen`, the script log, and
//! what the operand reader calls by address (the form lookups, the locals' owners, the inventory,
//! the command table and a command's handler, the parameter parser).

use engine as _;
use engine_core::extend::jip_patches;
use engine_core::prelude::*;
use engine_core::typed::diff::{Case, Diff, Input};
use engine_core::typed::leaf;
use engine_core::typed::{MAIN_THREAD, OTHER_THREADS};
use engine_typed::crt;
use engine_typed::scripts::{self, expression, numbers, value};

/// The exe's data the code reads: the operator table and error names (`0118c000` page) and the
/// CRT's SSE2 switch.
const TABLE_PAGE: u32 = 0x0118_c000;
const OPERATOR_TABLE: u32 = 0x0118_cad0;
const ERROR_NAMES: u32 = 0x0118_c594;
const SSE2_PAGE: u32 = 0x0127_0000;
const SSE2_AVAILABLE: u32 = 0x0127_0a6c;
/// The line being run (the "variable not found" report reads it).
const LINE_PAGE: u32 = 0x011c_a000;
const SCRIPT_LINE: u32 = 0x011c_ae50;

/// Where a case's answers for the stand-ins live (read by them, so they need no state).
const CONFIG: u32 = 0x0300_0000;
const ANSWER_LOOKUP: u32 = CONFIG;
const ANSWER_REF_VARS: u32 = CONFIG + 4;
const ANSWER_EXTRA: u32 = CONFIG + 8;
const ANSWER_ITEM: u32 = CONFIG + 12;
const ANSWER_ITEM_LOCALS: u32 = CONFIG + 16;
const ANSWER_QUEST_LOCALS: u32 = CONFIG + 20;
const ANSWER_HANDLER: u32 = CONFIG + 24;
const DEFINITION: u32 = CONFIG + 0x40;
const DEFINITION_FOR: u32 = CONFIG + 28;
/// A command handler's address: new in the setup, the same function on every side.
const HANDLER: u32 = 0x00ff_3000;
/// The script's vtable, its name slot's function and the name it answers.
const SCRIPT_VTABLE: u32 = CONFIG + 0x100;
const NAME_SLOT_FN: u32 = 0x00ff_3010;

const ATOL: u32 = 0x00ec_a6bd;
const ATOF: u32 = 0x00ec_a573;
const STRLEN: u32 = 0x0044_a670;
const SCRIPT_LOG: u32 = 0x005b_5e40;
const SCRIPT_REF_OBJECT_LIST: u32 = 0x005d_43c0;
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
const PARSE_PARAMETERS: u32 = 0x005a_ccb0;

fn ret(eax: u32) -> Ret {
    Ret {
        eax,
        ..Ret::default()
    }
}

fn ret_f(st0: f64) -> Ret {
    Ret {
        st0,
        ..Ret::default()
    }
}

/// A decimal parser over the guest text (`atol`'s and `atof`'s stand-ins).
fn text_of(e: &Engine, at: u32) -> String {
    String::from_utf8_lossy(&e.mem.cstr(at))
        .trim_start()
        .to_string()
}

fn number_end(text: &str, point: bool) -> usize {
    text.char_indices()
        .find(|&(i, c)| {
            !(c.is_ascii_digit() || (point && c == '.') || (i == 0 && (c == '-' || c == '+')))
        })
        .map_or(text.len(), |(i, _)| i)
}

fn atol(e: &mut Engine, a: &[u32]) -> Ret {
    let text = text_of(e, a[0]);
    let end = number_end(&text, false);
    ret(text[..end].parse::<i64>().unwrap_or(0) as i32 as u32)
}

fn atof(e: &mut Engine, a: &[u32]) -> Ret {
    let text = text_of(e, a[0]);
    let end = number_end(&text, true);
    ret_f(text[..end].parse::<f64>().unwrap_or(0.0))
}

/// The exe's data and the stand-ins every case has.
fn world(e: &mut Engine) {
    for page in [TABLE_PAGE, SSE2_PAGE, LINE_PAGE] {
        e.map(page, 0x1000);
    }
    e.map(CONFIG, 0x1000);
    let operators: [(u8, &[u8]); 16] = [
        (0, b"("),
        (0, b")"),
        (1, b"&&"),
        (2, b"||"),
        (3, b"<="),
        (3, b"<"),
        (3, b">="),
        (3, b">"),
        (3, b"=="),
        (3, b"!="),
        (4, b"-"),
        (4, b"+"),
        (5, b"*"),
        (5, b"/"),
        (5, b"%"),
        (6, b"~"),
    ];
    for (number, (precedence, text)) in (0u32..).zip(operators) {
        let entry = OPERATOR_TABLE + number * 8;
        e.mem.set_u32(entry, number);
        e.mem.set_u8(entry + 4, precedence);
        e.mem.write(entry + 5, text);
    }
    for code in 0..6 {
        e.mem.set_u32(ERROR_NAMES + code * 4, 0x5000 + code);
    }
    e.mem.set_u32(SSE2_AVAILABLE, 1);
    e.mem.set_u32(SCRIPT_LINE, 17);
    e.register(ATOL, atol);
    e.register(ATOF, atof);
    e.register(STRLEN, |e, a| ret(e.mem.cstr(a[0]).len() as u32));
    e.register(SCRIPT_LOG, |_, _| ret(0));
    e.register(SCRIPT_REF_OBJECT_LIST, |e, a| ret(e.mem.u32(a[0] + 0x44)));
    e.register(LOOKUP_FORM_BY_ID, |e, a| {
        ret(if a[0] == 0x77 {
            e.mem.u32(ANSWER_LOOKUP)
        } else {
            0
        })
    });
    e.register(FORM_TYPE_OF, |e, a| {
        ret(if a[0] == 0 {
            0
        } else {
            u32::from(e.mem.u8(a[0] + 4))
        })
    });
    e.register(FORM_FLAG_BIT_3, |e, a| ret((e.mem.u32(a[0] + 8) >> 3) & 1));
    e.register(GLOBAL_VALUE_OF, |e, a| {
        ret_f(if a[0] == 0 {
            0.0
        } else {
            f64::from(f32::from_bits(e.mem.u32(a[0] + 0x24)))
        })
    });
    e.register(QUEST_SCRIPT_LOCALS, |e, _| {
        ret(e.mem.u32(ANSWER_QUEST_LOCALS))
    });
    e.register(REFERENCE_SCRIPT_VARIABLES, |e, _| {
        ret(e.mem.u32(ANSWER_REF_VARS))
    });
    e.register(EXTRA_LIST_REFERENCE_POINTER, |e, _| {
        ret(e.mem.u32(ANSWER_EXTRA))
    });
    e.register(INVENTORY_ITEM_OF, |e, _| ret(e.mem.u32(ANSWER_ITEM)));
    e.register(ITEM_SCRIPT_LOCALS, |e, _| {
        ret(e.mem.u32(ANSWER_ITEM_LOCALS))
    });
    e.register(RELEASE_ITEM, |_, _| ret(0));
    e.register(GET_FUNCTION_DEFINITION, |e, a| {
        ret(if a[0] == e.mem.u32(DEFINITION_FOR) {
            DEFINITION
        } else {
            0
        })
    });
    e.register(PARSE_PARAMETERS, |_, _| ret(1));
    e.register(HANDLER, |e, a| {
        e.mem.set_f64(a[6], 6.25);
        ret(e.mem.u32(ANSWER_HANDLER))
    });
    e.register(NAME_SLOT_FN, |_, _| ret(0x0050_0000));
    for slot in (0..0x140).step_by(4) {
        e.mem.set_u32(SCRIPT_VTABLE + slot, NAME_SLOT_FN);
    }
}

/// A fresh `CExpression` (0x710 bytes), constructed by the translation.
fn new_expression(e: &mut Engine) -> u32 {
    let this = e.mem.alloc(0x710);
    e.call(0x0059_3490, &args![this]);
    this
}

/// A BSSimpleList of `items` (nodes of two words; an empty list is one zeroed node).
fn list(e: &mut Engine, items: &[u32]) -> u32 {
    if items.is_empty() {
        return e.mem.alloc(8);
    }
    let mut next = 0;
    for item in items.iter().rev() {
        let node = e.mem.alloc(8);
        e.mem.set_u32(node, *item);
        e.mem.set_u32(node + 4, next);
        next = node;
    }
    next
}

/// A script variable (id, value).
fn variable(e: &mut Engine, id: u32, value: f64) -> u32 {
    let v = e.mem.alloc(0x10);
    e.mem.set_u32(v, id);
    e.mem.set_f64(v + 8, value);
    v
}

/// Locals with variables 1 (2.5), 2 (-4) and 3 (holding the numeric id 0x77), owned by nothing.
fn locals(e: &mut Engine) -> u32 {
    let one = variable(e, 1, 2.5);
    let two = variable(e, 2, -4.0);
    let three = variable(e, 3, f64::from_bits(0x77));
    let vars = list(e, &[one, two, three]);
    let l = e.mem.alloc(0x14);
    e.mem.set_u32(l + 0xc, vars);
    l
}

/// A form of `kind` with flags.
fn form(e: &mut Engine, kind: u8, flags: u32) -> u32 {
    let f = e.mem.alloc(0x40);
    e.mem.set_u8(f + 4, kind);
    e.mem.set_u32(f + 8, flags);
    e.mem.set_u32(f + 0xc, 0x0012_3456);
    e.mem.set_u32(f + 0x20, 0x9);
    e.mem.set_u32(f + 0x24, 0x4049_0fdb);
    f
}

/// A referenced object: its form (+8) and its variable id (+0xc).
fn referenced(e: &mut Engine, form: u32, variable_id: u32) -> u32 {
    let o = e.mem.alloc(0x10);
    e.mem.set_u32(o + 8, form);
    e.mem.set_u32(o + 0xc, variable_id);
    o
}

/// A script with `objects` as its referenced objects, `flags` its form flags.
fn script(e: &mut Engine, objects: &[u32], flags: u32) -> u32 {
    let s = e.mem.alloc(0x54);
    e.mem.set_u32(s, SCRIPT_VTABLE);
    e.mem.set_u8(s + 4, 0x11);
    e.mem.set_u32(s + 8, flags);
    e.mem.set_u32(s + 0x1c, objects.len() as u32);
    let l = list(e, objects);
    e.mem.set_u32(s + 0x44, l);
    s
}

/// A command record: needs a reference (+0x10), its parameters (+0x14), its handler (+0x18).
fn command(e: &mut Engine, opcode: u32, needs_reference: bool, parameters: u32, handler: u32) {
    e.mem.set_u32(DEFINITION_FOR, opcode);
    e.mem.set_u8(DEFINITION + 0x10, u8::from(needs_reference));
    e.mem.set_u32(DEFINITION + 0x14, parameters);
    e.mem.set_u32(DEFINITION + 0x18, handler);
}

/// A script world for the operand reader: the script with five referenced objects (1 a global, 2
/// a quest, 3 a reference with a variable id, 4 one with no form but a variable id, 5 an actor),
/// the locals, and the command 0x1010.
struct ScriptWorld {
    script: u32,
    locals: u32,
}

fn script_world(e: &mut Engine, script_flags: u32) -> ScriptWorld {
    let locals = locals(e);
    let global = form(e, 0x06, 0);
    let quest = form(e, 0x47, 0);
    let reference = form(e, 0x3b, 0);
    let actor = form(e, 0x3c, 0);
    let objects = [
        referenced(e, global, 0),
        referenced(e, quest, 0),
        referenced(e, reference, 3),
        referenced(e, 0, 2),
        referenced(e, actor, 0),
    ];
    let script = script(e, &objects, script_flags);
    let other_locals = self::locals(e);
    e.mem.set_u32(ANSWER_LOOKUP, actor);
    e.mem.set_u32(ANSWER_QUEST_LOCALS, other_locals);
    e.mem.set_u32(ANSWER_REF_VARS, other_locals);
    e.mem.set_u32(ANSWER_ITEM_LOCALS, other_locals);
    command(e, 0x1010, false, 0x0060_0000, HANDLER);
    ScriptWorld { script, locals }
}

// ---- Expressions ---------------------------------------------------------------------------------

/// The token of `kind` for template letter `c`: `A`, `B` a double `z` token from the case's floats,
/// `I`, `J` an `n` integer token from its words; anything else as written.
fn token(c: &str, case: &Case, floats: [usize; 2], ints: [usize; 2]) -> Vec<u8> {
    let float = |i: usize| {
        let mut t = vec![b'z'];
        t.extend(case.f64(i).to_bits().to_le_bytes());
        t
    };
    let int = |i: usize| {
        let mut t = vec![b'n'];
        t.extend(case.word(i).to_le_bytes());
        t
    };
    match c {
        "A" => float(floats[0]),
        "B" => float(floats[1]),
        "I" => int(ints[0]),
        "J" => int(ints[1]),
        _ => c.as_bytes().to_vec(),
    }
}

/// The postfix bytes of `template` (tokens separated by spaces), each token after a space.
fn postfix(template: &str, case: &Case, floats: [usize; 2], ints: [usize; 2]) -> Vec<u8> {
    let mut out = Vec::new();
    for t in template.split(' ') {
        out.push(b' ');
        out.extend(token(t, case, floats, ints));
    }
    out
}

/// One evaluation: the expression's bytes copied in, the call's words.
fn eval_words(e: &mut Engine, bytes: &[u8], context: [u32; 4], parse: u32) -> Vec<u32> {
    let this = new_expression(e);
    let data = e.mem.alloc(bytes.len() as u32 + 0x10);
    e.mem.write(data, bytes);
    vec![
        this,
        data,
        context[0],
        context[1],
        context[2],
        context[3],
        bytes.len() as u32,
        parse,
    ]
}

/// xNVSE's two changes to the evaluator, when `on`: its 0x500-byte token buffer (which also moves
/// `Eval`'s token buffer down its grown frame) and its command call at seam `005949d4`.
fn xnvse(e: &mut Engine, on: bool) {
    if on {
        let buffer = e.mem.alloc(0x500);
        e.set_expression_alt_buffer(buffer);
        e.on_site_for_test(0x0059_49d4, xnvse_command_call);
    }
}

/// xNVSE's command call: the operand read on the script's own bytes (here by the game's reader, by
/// address), so a typed and a translated evaluator hand it the same words.
fn xnvse_command_call(e: &mut Engine, w: &[u32]) -> Option<u32> {
    let offset = w[1];
    let at = w[6];
    e.mem.set_u32(offset, 0);
    let ok = e
        .call(
            0x005a_c7a0,
            &args![w[0], at, offset, w[2], w[3], w[4], w[5], 0u32],
        )
        .u32();
    Some(ok)
}

/// JIP LN's double precision operators, when `on`: the patch bits and its operator seam, which
/// here does `+` and `*` itself in doubles (as JIP's handler) and leaves the rest to the exe.
fn jip(e: &mut Engine, on: bool) {
    if on {
        e.set_jip_patches(
            jip_patches::INSTALLED | jip_patches::FIXES | jip_patches::DOUBLE_PRECISION,
        );
        e.on_site_for_test(0x0059_3fbc, jip_operator);
    }
}

fn jip_operator(e: &mut Engine, w: &[u32]) -> Option<u32> {
    let value = |kind: u32, low: u32, high: u32| {
        if kind & 0xff != 0 {
            f64::from_bits(u64::from(low) | u64::from(high) << 32)
        } else {
            f64::from(low as i32)
        }
    };
    let l = value(w[1], w[2], w[3]);
    let r = value(w[4], w[5], w[6]);
    let result = match w[0] + 2 {
        11 => l + r,
        12 => l * r,
        _ => return None,
    };
    e.mem.set_u8(w[7], 1);
    e.mem.set_f64(w[7] + 8, result);
    Some(1)
}

/// What every `Eval` comparison names: the families and the NaN payloads.
trait Common {
    fn common(self) -> Diff;
}

impl Common for Diff {
    fn common(self) -> Diff {
        self.family(&scripts::SCRIPTS)
            .family(&crt::CRT)
            .family(&leaf::LEAVES)
            .allow_nan_payloads()
    }
}

/// Every operator on two doubles, a double and an integer, and two integers that cannot fault.
const BINARY: &[&str] = &[
    "A B &&",
    "A B ||",
    "A B <=",
    "A B <",
    "A B >=",
    "A B >",
    "A B ==",
    "A B !=",
    "A B -",
    "A B +",
    "A B *",
    "A B /",
    "A ~",
    "A I +",
    "I A -",
    "A I *",
    "I A /",
    "A I <",
    "I A ==",
    "I A &&",
    "A B + I *",
    "A B ~ -",
    "A B * B /",
    "I J +",
    "I J -",
    "I J *",
    "I J <",
    "I J ==",
    "I ~",
    "I I <",
    "I I <=",
    "I I >",
    "I I >=",
    "I I !=",
    "A A <",
    "A A ==",
    "A A >=",
];

#[test]
fn scripts_eval_operators_on_doubles_and_integers() {
    let templates: Vec<u32> = (0..BINARY.len() as u32).collect();
    let templates: &'static [u32] = templates.leak();
    Diff::new(entry!(
        0x0059_3db0,
        expression::eval(Ptr<_>, Ptr<_>, u32, u32, u32, u32, i32, u8) -> f64
    ))
    .common()
    .inputs(&[
        Input::Of(templates),
        Input::F64,
        Input::F64,
        Input::Of(&[3, 0x8000_0000, 0xffff_fff9, 0x7fff_ffff, 0]),
        Input::Of(&[1, 5, 0xffff_ffd8]),
        Input::Flag,
    ])
    .max_cases(512)
    .float_sweep(96)
    .setup(|e, case| {
        world(e);
        let bytes = postfix(BINARY[case.word(0) as usize], case, [1, 2], [3, 4]);
        {
            let w = script_world(e, 0);
            eval_words(
                e,
                &bytes,
                [0x0070_0000, 0, w.script, w.locals],
                case.word(5),
            )
        }
    })
    .run()
    .assert_same();
}

/// Steps whose `f64` result is exactly halfway between two 24-bit values while the exact result is
/// not (the main thread's x87 rounds the exact result once; rounding the `f64` again would make the
/// tie): `(left bits, right bits, operator)`. 1 + 2^-24 (kept bit even) and 1 + 3 * 2^-24 (odd) with
/// a residual of either sign; a product and a quotient with a residual far below the double's.
const MIDPOINTS: &[(u64, u64, &str)] = &[
    (0x3FF0_0000_1000_0000, 0x3C32_725D_D1D2_43AC, "+"), // 1 + 2^-24 and 1e-18
    (0x3FF0_0000_1000_0000, 0xBC32_725D_D1D2_43AC, "+"), // and -1e-18
    (0x3FF0_0000_1000_0000, 0x3C32_725D_D1D2_43AC, "-"),
    (0x3FF0_0000_1000_0000, 0xBC32_725D_D1D2_43AC, "-"),
    (0x3FF0_0000_3000_0000, 0x3C32_725D_D1D2_43AC, "+"),
    (0x3FF0_0000_3000_0000, 0x3C32_725D_D1D2_43AC, "-"),
    (0x3FF0_0000_0FFF_FFFF, 0x3FF0_0000_0000_0001, "*"), // 1 + 2^-24 + 2^-76 - 2^-104
    (0x3FF0_0000_3000_0001, 0x3FF0_0000_0000_0001, "/"), // 1 + 3 * 2^-24 - about 3 * 2^-76
    (0x3FF0_0000_0000_0000, 0x4008_0000_0000_0000, "/"), // 1 / 3: no tie, every rounding mode
];

/// The midpoint steps under every control word the exe's arithmetic reads: 53 bits, and 24 bits
/// under each rounding mode (to nearest as on the main thread, down, up, toward zero).
#[test]
fn scripts_eval_midpoints_and_rounding_modes() {
    let cases: Vec<u32> = (0..MIDPOINTS.len() as u32).collect();
    let cases: &'static [u32] = cases.leak();
    Diff::new(entry!(
        0x0059_3db0,
        expression::eval(Ptr<_>, Ptr<_>, u32, u32, u32, u32, i32, u8) -> f64
    ))
    .common()
    .inputs(&[Input::Of(cases)])
    .control_words(&[
        OTHER_THREADS,
        MAIN_THREAD,
        MAIN_THREAD | 0x0400,
        MAIN_THREAD | 0x0800,
        MAIN_THREAD | 0x0c00,
    ])
    .setup(|e, case| {
        world(e);
        let (left, right, operator) = MIDPOINTS[case.word(0) as usize];
        let mut bytes = Vec::new();
        for value in [left, right] {
            bytes.extend(b" z");
            bytes.extend(value.to_le_bytes());
        }
        bytes.push(b' ');
        bytes.extend(operator.as_bytes());
        let w = script_world(e, 0);
        eval_words(e, &bytes, [0x0070_0000, 0, w.script, w.locals], 0)
    })
    .run()
    .assert_same();
}

/// Texts, literals, the modulo and division cases that do not fault, the errors and the long
/// tokens; parse mode, xNVSE and JIP on and off.
const TEXTS: &[&str] = &[
    "3 4 +",
    "3 4 -",
    "6 7 *",
    "7 2 /",
    "7 2 ~ /",
    "7 2 ~ %",
    "7 ~ 2 %",
    "1.5 2 *",
    "1 4.0 /",
    "7.9 2 %",
    "7 2.5 %",
    "7.5 2.5 %",
    "-3 2 %",
    "2 3 4 * +",
    "2147483647 1 +",
    "0.5 ~",
    "1 2 < 3 4 > ||",
    // Equal integer literals, so each comparison's equal case on two integers is seen (the
    // generated `I I` cases did not reach the integer comparisons: a planted `<` to `<=` passed).
    "2 2 <",
    "2 2 <=",
    "2 2 >",
    "2 2 >=",
    "2 2 ==",
    "2 2 !=",
    "2 2.0 ==",
    "1 0 /",
    "1 0 %",
    "1.0 0.0 /",
    "3 0.0 /",
    "3 0.0 %",
    "3.5 0.0 /",
    "3.5 0 /",
    "1 +",
    "+",
    "~",
    "1 2",
    "",
    "1 )",
    "( 1",
    "1 2 (",
    "1.2.3 1 +",
    "-.5 .5 +",
    "12a 1 +",
    "\"text\" 1 +",
    "\"open",
    "1 2 &",
    "1 2 !",
    "1 2 =",
    "1 2 |",
    "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa 1 +",
    "1 2 3",
];

/// Which text cases stop: an integer divided by an integer zero in parse mode (it replaces only
/// the divisor's double value, and the integer division reads its low word, 0).
fn text_stops(template: &str, parse: bool) -> bool {
    parse && matches!(template, "1 0 /" | "1 0 %")
}

#[test]
fn scripts_eval_texts_errors_and_extenders() {
    let templates: Vec<u32> = (0..TEXTS.len() as u32).collect();
    let templates: &'static [u32] = templates.leak();
    Diff::new(entry!(
        0x0059_3db0,
        expression::eval(Ptr<_>, Ptr<_>, u32, u32, u32, u32, i32, u8) -> f64
    ))
    .common()
    .inputs(&[Input::Of(templates), Input::Flag, Input::Flag, Input::Flag])
    .max_cases(512)
    .expect_stops(|case| text_stops(TEXTS[case.word(0) as usize], case.word(1) != 0))
    .setup(|e, case| {
        world(e);
        xnvse(e, case.word(2) != 0);
        jip(e, case.word(3) != 0);
        let template = TEXTS[case.word(0) as usize];
        let mut bytes = Vec::new();
        for t in template.split(' ').filter(|t| !t.is_empty()) {
            bytes.push(b' ');
            bytes.extend(t.as_bytes());
        }
        if bytes.is_empty() {
            bytes.push(b' ');
        }
        {
            let w = script_world(e, 0);
            eval_words(
                e,
                &bytes,
                [0x0070_0000, 0, w.script, w.locals],
                case.word(1),
            )
        }
    })
    .run()
    .assert_same();
}

/// Expressions of compiled operands, read through the operand reader (typed too).
const COMPILED: &[&[u8]] = &[
    b" s\x01\x00 1 +",
    b" f\x02\x00 s\x01\x00 *",
    b" l\x09\x00 1 +",
    b" r\x03\x00s\x01\x00 2 *",
    b" r\x02\x00f\x02\x00 1 -",
    b" G\x01\x00 2 +",
    b" Z\x05\x00 1 +",
    b" X\x10\x10\x00\x00 1 +",
    b" r\x05\x00X\x10\x10\x00\x00 1 +",
    b" X\x11\x10\x00\x00 1 +",
    b" n\x07\x00\x00\x00 z\x00\x00\x00\x00\x00\x00\xe0\x3f +",
    b" s\x01\x00 s\x02\x00 + s\x01\x00 * 3 <",
    b" r\x09\x00s\x01\x00 1 +",
    b" q\x01\x00 1 +",
];

#[test]
fn scripts_eval_compiled_operands_with_extenders() {
    let templates: Vec<u32> = (0..COMPILED.len() as u32).collect();
    let templates: &'static [u32] = templates.leak();
    Diff::new(entry!(
        0x0059_3db0,
        expression::eval(Ptr<_>, Ptr<_>, u32, u32, u32, u32, i32, u8) -> f64
    ))
    .common()
    .inputs(&[
        Input::Of(templates),
        Input::Flag,
        Input::Flag,
        Input::Flag,
        Input::Flag,
    ])
    .max_cases(512)
    .setup(|e, case| {
        world(e);
        xnvse(e, case.word(2) != 0);
        jip(e, case.word(3) != 0);
        let w = script_world(e, 0);
        e.mem.set_u32(ANSWER_HANDLER, case.word(4));
        eval_words(
            e,
            COMPILED[case.word(0) as usize],
            [0x0070_0000, 0, w.script, w.locals],
            case.word(1),
        )
    })
    .run()
    .assert_same();
}

// ---- The tokenizer and the postfix conversion -------------------------------------------------

const SOURCES: &[&[u8]] = &[
    b"  12 + 3",
    b"\t\x01\x80abc def",
    b"<=1",
    b"<1",
    b"!x",
    b"&&&",
    b"a&b",
    b"s\x01\x00rest",
    b"r\x02\x00s\x01\x00 1",
    b"r\x02\x00X\x10\x10\x02\x00\x01\x02 9",
    b"X\x10\x10\x02\x00\x01\x02 9",
    b"X\x00\x05\x02\x00 9",
    b"n\x01\x02\x03\x04 5",
    b"z\x01\x02\x03\x04\x05\x06\x07\x08 5",
    b"\"quoted text\" 5",
    b"\"unclosed",
    b"",
    b"   ",
    b"~5",
    b"5-3",
    b"Gab",
    b"Z\xff\xff",
];

#[test]
fn scripts_next_token() {
    let sources: Vec<u32> = (0..SOURCES.len() as u32).collect();
    let sources: &'static [u32] = sources.leak();
    Diff::new(entry!(
        0x0059_3870,
        expression::next_token(Ptr<_>, Ptr<_>, Ptr<_>, Ptr<_>, bool, Ptr<_>) -> u32
    ))
    .family(&scripts::SCRIPTS)
    .family(&crt::CRT)
    .family(&leaf::LEAVES)
    .inputs(&[Input::Of(sources), Input::Flag, Input::Flag])
    .setup(|e, case| {
        world(e);
        let this = new_expression(e);
        let source = SOURCES[case.word(0) as usize];
        let text = e.mem.alloc(source.len() as u32 + 0x10);
        e.mem.write(text, source);
        let cursor = e.mem.alloc(4);
        e.mem.set_u32(cursor, text);
        let out = e.mem.alloc(0x80);
        let kind = e.mem.alloc(4);
        let spaces = if case.word(2) != 0 {
            let s = e.mem.alloc(4);
            e.mem.set_u32(s, 3);
            s
        } else {
            0
        };
        vec![this, cursor, out, kind, case.word(1), spaces]
    })
    .run()
    .assert_same();
}

const INFIX: &[&[u8]] = &[
    b"1 + 2 * 3",
    b"(1 + 2) * 3",
    b"-1 - -2",
    b"a && b || c",
    b"x <= 3 && y != 4",
    b"((((1))))",
    b"(1 + 2",
    b"1 + 2)",
    b"s\x01\x00 * 2",
    b"",
    b"1 2 3",
    b"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa + 1",
];

#[test]
fn scripts_to_postfix() {
    let sources: Vec<u32> = (0..INFIX.len() as u32 + 1).collect();
    let sources: &'static [u32] = sources.leak();
    Diff::new(entry!(
        0x0059_3500,
        expression::to_postfix(Ptr<_>, Ptr<_>, Ptr<_>) -> Ptr<_>
    ))
    .family(&scripts::SCRIPTS)
    .family(&crt::CRT)
    .family(&leaf::LEAVES)
    .inputs(&[Input::Of(sources), Input::Flag])
    .setup(|e, case| {
        world(e);
        xnvse(e, case.word(1) != 0);
        let this = new_expression(e);
        let index = case.word(0) as usize;
        // The last case: 65 open parentheses, past the operator stack's 64 words.
        let deep = [b'('; 65];
        let source: &[u8] = if index < INFIX.len() {
            INFIX[index]
        } else {
            &deep
        };
        let text = e.mem.alloc(source.len() as u32 + 0x10);
        e.mem.write(text, source);
        let length = e.mem.alloc(4);
        e.mem.set_u32(length, source.len() as u32);
        vec![this, text, length]
    })
    .run()
    .assert_same();
}

// ---- The small functions ------------------------------------------------------------------------

#[test]
fn scripts_construct_and_error() {
    Diff::new(entry!(0x0059_3490, expression::construct(Ptr<_>) -> Ptr<_>))
        .family(&scripts::SCRIPTS)
        .family(&leaf::LEAVES)
        .inputs(&[Input::Word])
        .setup(|e, case| {
            world(e);
            let this = e.mem.alloc(0x710);
            e.mem.set_u32(this, case.word(0));
            e.mem.set_u32(this + 0x504, case.word(0));
            e.mem.set_u32(this + 0x708, case.word(0));
            vec![this]
        })
        .run()
        .assert_same();
    Diff::new(entry!(0x0059_34d0, expression::set_error(Ptr<_>, u32)))
        .family(&scripts::SCRIPTS)
        .family(&leaf::LEAVES)
        .inputs(&[Input::Of(&[0, 1, 2, 3, 4, 5])])
        .setup(|e, case| {
            world(e);
            let this = new_expression(e);
            vec![this, case.word(0)]
        })
        .run()
        .assert_same();
    Diff::new(entry!(
        0x0059_4b60,
        expression::operator_stack_construct(Ptr<_>) -> Ptr<_>
    ))
    .family(&scripts::SCRIPTS)
    .inputs(&[Input::Word])
    .setup(|e, case| {
        let this = e.mem.alloc(0x210);
        e.mem.set_u32(this + 0x100, case.word(0));
        e.mem.set_u32(this + 0x200, case.word(0));
        vec![this]
    })
    .run()
    .assert_same();
    Diff::new(entry!(
        0x0059_4c60,
        expression::operand_stack_construct(Ptr<_>) -> Ptr<_>
    ))
    .family(&scripts::SCRIPTS)
    .inputs(&[Input::Word])
    .setup(|e, case| {
        let this = e.mem.alloc(0x210);
        e.mem.set_u32(this + 0x100, case.word(0));
        e.mem.set_u32(this + 0x200, case.word(0));
        vec![this]
    })
    .run()
    .assert_same();
}

/// A stack of either kind with its index at `index`.
fn stack(e: &mut Engine, index: u32) -> u32 {
    let this = e.mem.alloc(0x210);
    for i in 0..0x84 {
        e.mem.set_u32(this + 4 * i, 0x1000 + i);
    }
    e.mem.set_u32(this + 0x100, index);
    e.mem.set_u32(this + 0x200, index);
    this
}

const INDICES: &[u32] = &[u32::MAX, 0, 1, 0x1e, 0x3e, 0x3f];

#[test]
fn scripts_operator_stack() {
    Diff::new(entry!(0x0059_4b80, expression::operator_push(Ptr<_>, u32)))
        .family(&scripts::SCRIPTS)
        .inputs(&[Input::Of(INDICES), Input::Word])
        .setup(|e, case| vec![stack(e, case.word(0)), case.word(1)])
        .run()
        .assert_same();
    Diff::new(entry!(0x0059_4bc0, expression::operator_pop(Ptr<_>) -> u32))
        .family(&scripts::SCRIPTS)
        .inputs(&[Input::Of(&[0, 1, 0x1e, 0x3f])])
        .setup(|e, case| vec![stack(e, case.word(0))])
        .run()
        .assert_same();
    Diff::new(entry!(
        0x0059_4c00,
        expression::operator_is_empty(Ptr<_>) -> bool
    ))
    .family(&scripts::SCRIPTS)
    .inputs(&[Input::Of(INDICES)])
    .setup(|e, case| vec![stack(e, case.word(0))])
    .run()
    .assert_same();
    Diff::new(entry!(
        0x0059_4c20,
        expression::operator_is_full(Ptr<_>) -> bool
    ))
    .family(&scripts::SCRIPTS)
    .inputs(&[Input::Of(INDICES)])
    .setup(|e, case| vec![stack(e, case.word(0))])
    .run()
    .assert_same();
    Diff::new(entry!(
        0x0059_4d20,
        expression::operand_is_empty(Ptr<_>) -> bool
    ))
    .family(&scripts::SCRIPTS)
    .inputs(&[Input::Of(INDICES)])
    .setup(|e, case| vec![stack(e, case.word(0))])
    .run()
    .assert_same();
    Diff::new(entry!(0x0059_4c40, expression::operator_clear(Ptr<_>)))
        .family(&scripts::SCRIPTS)
        .inputs(&[Input::Of(INDICES)])
        .setup(|e, case| vec![stack(e, case.word(0))])
        .run()
        .assert_same();
    Diff::new(entry!(0x0059_4d40, expression::operand_clear(Ptr<_>)))
        .family(&scripts::SCRIPTS)
        .inputs(&[Input::Of(INDICES)])
        .setup(|e, case| vec![stack(e, case.word(0))])
        .run()
        .assert_same();
}

/// An operand stack big enough for 32 operands and the index.
fn operand_stack(e: &mut Engine, index: u32) -> u32 {
    let this = e.mem.alloc(0x210);
    for i in 0..0x84 {
        e.mem.set_u32(this + 4 * i, 0x2000 + i);
    }
    e.mem.set_u32(this + 0x200, index);
    this
}

#[test]
fn scripts_operand_stack() {
    Diff::new(entry!(
        0x0059_4c80,
        expression::operand_push(Ptr<_>, u32, u32, u32, u32)
    ))
    .family(&scripts::SCRIPTS)
    .inputs(&[
        Input::Of(&[u32::MAX, 0, 0x1e]),
        Input::Word,
        Input::Of(&[0, 7]),
        Input::Word,
        Input::Of(&[0x3ff0_0000]),
    ])
    .max_cases(128)
    .setup(|e, case| {
        vec![
            operand_stack(e, case.word(0)),
            case.word(1),
            case.word(2),
            case.word(3),
            case.word(4),
        ]
    })
    .run()
    .assert_same();
    Diff::new(entry!(
        0x0059_4cd0,
        expression::operand_pop(Ptr<_>, Ptr<_>) -> Ptr<_>
    ))
    .family(&scripts::SCRIPTS)
    .inputs(&[Input::Of(&[0, 1, 0x1e])])
    .setup(|e, case| {
        let stack = operand_stack(e, case.word(0));
        let out = e.mem.alloc(0x10);
        vec![stack, out]
    })
    .run()
    .assert_same();
}

#[test]
fn scripts_number_tests() {
    const TEXTS: &[&[u8]] = &[
        b"0", b"123", b"-5", b"-", b"", b"12a", b"1.5", b"+1", b"1-", b"-.5", b"5.", b".5", b".",
        b"1.2.3", b"-0.25", b"\x80", b"9\xff",
    ];
    let texts: Vec<u32> = (0..TEXTS.len() as u32).collect();
    let texts: &'static [u32] = texts.leak();
    Diff::new(entry!(0x004b_1230, numbers::is_an_integer(Ptr<_>) -> bool))
        .family(&scripts::SCRIPTS)
        .inputs(&[Input::Of(texts)])
        .setup(|e, case| {
            let text = TEXTS[case.word(0) as usize];
            let at = e.mem.alloc(text.len() as u32 + 4);
            e.mem.write(at, text);
            vec![at]
        })
        .run()
        .assert_same();
    Diff::new(entry!(0x004b_12d0, numbers::is_a_float(Ptr<_>) -> bool))
        .family(&scripts::SCRIPTS)
        .inputs(&[Input::Of(texts)])
        .setup(|e, case| {
            let text = TEXTS[case.word(0) as usize];
            let at = e.mem.alloc(text.len() as u32 + 4);
            e.mem.write(at, text);
            vec![at]
        })
        .run()
        .assert_same();
}

// ---- The operand reader and its lookups -------------------------------------------------------

/// Operands for the reader: kind, index or id, and what follows.
const OPERANDS: &[&[u8]] = &[
    b"s\x01\x00",
    b"f\x02\x00",
    b"l\x09\x00",
    b"q\x01\x00",
    b"G\x01\x00",
    b"G\x04\x00",
    b"Z\x05\x00",
    b"Z\x00\x00",
    b"r\x02\x00s\x01\x00",
    b"r\x03\x00f\x02\x00",
    b"r\x04\x00s\x01\x00",
    b"r\x05\x00s\x01\x00",
    b"r\x05\x00X\x10\x10\x00\x00",
    b"r\x02\x00X\x10\x10\x00\x00",
    b"r\x09\x00s\x01\x00",
    b"X\x10\x10\x00\x00",
    b"X\x11\x10\x00\x00",
    b"n\x07\x00\x00\x80",
    b"z\x00\x00\x00\x00\x00\x00\xe0\x3f",
];

#[test]
fn scripts_read_operand() {
    let operands: Vec<u32> = (0..OPERANDS.len() as u32).collect();
    let operands: &'static [u32] = operands.leak();
    Diff::new(entry!(
        0x005a_c7a0,
        value::read_operand(u32, u32, u32, u32, u32, u32, u32, u8) -> bool
    ))
    .family(&scripts::SCRIPTS)
    .family(&crt::CRT)
    .family(&leaf::LEAVES)
    .inputs(&[
        Input::Of(operands),
        Input::Flag,
        Input::Of(&[0, 8]),
        Input::Of(&[0, 1, 2]),
        Input::Flag,
        Input::Flag,
        Input::Flag,
        Input::Flag,
    ])
    .max_cases(512)
    .setup(|e, case| {
        world(e);
        let w = script_world(e, case.word(2));
        e.mem.set_u32(ANSWER_HANDLER, case.word(4));
        // The reference's owner: an inventory (with or without the item) or its own variables.
        let item = if case.word(3) == 2 { 0 } else { 0x0080_0000 };
        e.mem.set_u32(
            ANSWER_EXTRA,
            if case.word(3) != 0 { 0x0081_0000 } else { 0 },
        );
        e.mem.set_u32(ANSWER_ITEM, item);
        command(
            e,
            0x1010,
            case.word(5) != 0,
            if case.word(6) != 0 { 0x0060_0000 } else { 0 },
            HANDLER,
        );
        let operand = OPERANDS[case.word(0) as usize];
        let code = e.mem.alloc(0x20);
        e.mem.write(code + 2, operand);
        let offset = e.mem.alloc(4);
        e.mem.set_u32(offset, 2);
        let out = e.mem.alloc(8);
        e.mem.set_u64(out, 0x1111_2222_3333_4444);
        let locals = if case.word(1) != 0 { w.locals } else { 0 };
        vec![
            out,
            code,
            offset,
            0x0070_0000,
            0,
            w.script,
            locals,
            case.word(7),
        ]
    })
    .run()
    .assert_same();
}

#[test]
fn scripts_referenced_object() {
    Diff::new(entry!(
        0x005a_c4f0,
        value::referenced_object(Ptr<_>, u32, u32) -> u32
    ))
    .family(&scripts::SCRIPTS)
    .family(&crt::CRT)
    .family(&leaf::LEAVES)
    .inputs(&[
        Input::Of(&[0, 1, 2, 3, 4, 5, 6]),
        Input::Flag,
        Input::Of(&[0, 1, 2]),
        Input::Flag,
    ])
    .setup(|e, case| {
        world(e);
        let w = script_world(e, 0);
        let index = case.word(0);
        let locals = if case.word(1) != 0 { w.locals } else { 0 };
        let tls = e.tls();
        // The cache: empty, holding this very lookup, or another index.
        match case.word(2) {
            1 => {
                e.mem.set_u32(tls + 0x288, w.script);
                e.mem.set_u32(tls + 0x280, index);
                e.mem.set_u32(tls + 0x284, locals);
                e.mem.set_u32(tls + 0x27c, 0x0abc_0000);
            }
            2 => {
                e.mem.set_u32(tls + 0x288, w.script);
                e.mem.set_u32(tls + 0x280, index + 1);
                e.mem.set_u32(tls + 0x284, locals);
            }
            _ => {}
        }
        if case.word(3) == 0 {
            e.mem.set_u32(ANSWER_LOOKUP, 0);
        }
        vec![w.script, index, locals]
    })
    .run()
    .assert_same();
}

#[test]
fn scripts_get_variable() {
    Diff::new(entry!(
        0x005a_9140,
        value::get_variable(Ptr<_>, u32, Ptr<_>) -> f64
    ))
    .family(&scripts::SCRIPTS)
    .family(&leaf::LEAVES)
    .inputs(&[
        Input::Of(&[0, 1, 2, 3, 4]),
        Input::Of(&[0, 1, 2, 3]),
        Input::Flag,
    ])
    .setup(|e, case| {
        world(e);
        let w = script_world(e, 0);
        let id = case.word(0);
        let tls = e.tls();
        match case.word(1) {
            // The cache holds this lookup, another id, or a missing variable for this one.
            1 => {
                e.mem.set_u32(tls + 0x278, w.locals);
                e.mem.set_u32(tls + 0x274, id);
                let v = variable(e, id, 9.5);
                e.mem.set_u32(tls + 0x270, v);
            }
            2 => {
                e.mem.set_u32(tls + 0x278, w.locals);
                e.mem.set_u32(tls + 0x274, id + 1);
                e.mem.set_u32(tls + 0x270, 0x0abc_0000);
            }
            3 => {
                e.mem.set_u32(tls + 0x278, w.locals);
                e.mem.set_u32(tls + 0x274, id);
                e.mem.set_u32(tls + 0x270, 0);
            }
            _ => {}
        }
        let script = if case.word(2) != 0 { w.script } else { 0 };
        vec![w.locals, id, script]
    })
    .run()
    .assert_same();
}

// ---- The game's own compiled expressions (opt-in) -----------------------------------------------

/// Every expression of the compiled scripts' `if`, `elseif` and `set` lines (opcodes 0x16, 0x18,
/// 0x15): its bytes after the 16-bit length.
fn expressions_of(code: &[u8], out: &mut Vec<Vec<u8>>) {
    let word = |at: usize| {
        code.get(at..at + 2)
            .map(|b| u16::from_le_bytes([b[0], b[1]]) as usize)
    };
    let mut at = 0;
    while let (Some(opcode), Some(length)) = (word(at), word(at + 2)) {
        let body = at + 4;
        let end = body + length;
        if end > code.len() {
            break;
        }
        let expression_at = match opcode {
            0x16 | 0x18 => Some(body),
            // `set`: the variable (`s`/`f`/`l` and an id, or `r` and an index first), then the
            // expression's length.
            0x15 => match code.get(body) {
                Some(b'r') => Some(body + 6),
                Some(_) => Some(body + 3),
                None => None,
            },
            _ => None,
        };
        if let Some(start) = expression_at {
            if let Some(size) = word(start) {
                if start + 2 + size <= end && size > 0 {
                    out.push(code[start + 2..start + 2 + size].to_vec());
                }
            }
        }
        at = end;
    }
}

/// `NV_DATA="...\Data" cargo test -p engine --test typed_scripts -- --ignored real_scda`: every
/// expression of `FalloutNV.esm`'s compiled scripts (up to `NV_EXPRESSIONS`, default 3000), typed
/// against translated in the same script world, xNVSE and JIP on and off. Nothing of the game's is
/// stored; the expressions' referenced objects and commands beyond the world's answer as missing.
#[test]
#[ignore = "needs the player's FalloutNV.esm (set NV_DATA)"]
fn real_scda_expressions() {
    use esm::{FourCC, Plugin};
    let data = std::path::PathBuf::from(std::env::var("NV_DATA").expect("NV_DATA"));
    let limit: usize = std::env::var("NV_EXPRESSIONS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(3000);
    let plugin = Plugin::open(data.join("FalloutNV.esm")).expect("FalloutNV.esm");
    let mut all = Vec::new();
    for entry in plugin.records_of_type(FourCC(*b"SCPT")) {
        let record = plugin.record(entry).expect("SCPT");
        if let Some(code) = record.get(FourCC(*b"SCDA")) {
            expressions_of(&code.data, &mut all);
        }
        if all.len() >= limit {
            break;
        }
    }
    all.truncate(limit);
    assert!(!all.is_empty(), "no expressions found");
    let count = all.len();
    let all: &'static [Vec<u8>] = all.leak();
    let indices: &'static [u32] = (0..count as u32).collect::<Vec<_>>().leak();
    Diff::new(entry!(
        0x0059_3db0,
        expression::eval(Ptr<_>, Ptr<_>, u32, u32, u32, u32, i32, u8) -> f64
    ))
    .common()
    .inputs(&[Input::Of(indices), Input::Flag, Input::Flag])
    .max_cases(count * 4)
    .setup(move |e, case| {
        world(e);
        xnvse(e, case.word(1) != 0);
        jip(e, case.word(2) != 0);
        let w = script_world(e, 0);
        eval_words(
            e,
            &all[case.word(0) as usize],
            [0x0070_0000, 0, w.script, w.locals],
            0,
        )
    })
    .run()
    .assert_same();
    eprintln!("{count} real expressions compared");
}
