//! Tests of the expression compiler and its hooks, from xNVSE 6.4.9's rules (`ScriptUtils.cpp`,
//! `ScriptTokens.cpp`, `Hooks_Script.cpp`): a line compiled through its command's parse word
//! on the decompiler tests' engine (the script `Test` with locals `x` float, `i` int, `rx` a
//! reference variable, and its reference list), its bytes compared with the bytes the rules
//! give for each token, operator and error, then read back by the decompiler and run by the
//! evaluator.

use super::*;
use crate::commands::test_support::{local_value, DATA, LIST, OFFSET, RESULT, SCRIPT};
use crate::decompile::tests::{engine as decompile_engine, line as script_line, set_script};
use crate::decompile::{decompile, parses};
use std::cell::RefCell;

/// The compile's own memory: the script buffer, the line, the heap, a lambda's stand-in data.
const AREA: u32 = 0x0500_0000;
const BUF: u32 = AREA;
const LINE: u32 = AREA + 0x100;
const BUF_TEXT: u32 = AREA + 0x800;
const HEAP_TOP: u32 = AREA + 0xffc;
const HEAP: u32 = AREA + 0x1000;
const CONSOLE: u32 = AREA + 0x3_f000;
const COMPILER: u32 = AREA + 0x3_f800;
/// A command record of the game's syntax (`GetV`, one optional integer).
const GET_V: u32 = 0x1002;
/// A test record parsed by xNVSE's expression parser: two parameters.
const TWO: u32 = 0x1003;
/// A reference that is not persistent (`GetFormByID("LooseRef")`).
const LOOSE_REF: u32 = AREA + 0x2_a000;

thread_local! {
    static CONSOLE_LINES: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
    static SEEN: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
}

fn console_print(e: &mut Engine, a: &[u32]) -> Ret {
    let text = guest_text(e, e.mem.u32(a[2]));
    CONSOLE_LINES.with(|c| {
        c.borrow_mut()
            .push(String::from_utf8_lossy(&text).into_owned())
    });
    Ret::default()
}

fn printed() -> Vec<String> {
    CONSOLE_LINES.with(|c| std::mem::take(&mut *c.borrow_mut()))
}

fn seen() -> Vec<String> {
    SEEN.with(|c| std::mem::take(&mut *c.borrow_mut()))
}

fn heap(e: &mut Engine, a: &[u32]) -> Ret {
    let at = e.mem.u32(HEAP_TOP).max(HEAP);
    e.mem
        .set_u32(HEAP_TOP, at + a[0].max(1).next_multiple_of(8));
    Ret {
        eax: at,
        ..Ret::default()
    }
}

fn upper(_: &mut Engine, a: &[u32]) -> Ret {
    Ret {
        eax: u32::from((a[0] as u8).to_ascii_uppercase()),
        ..Ret::default()
    }
}

/// `GetFormByID(editorID)`: `PlayerRef` (the stand-in form A) and `TestQuest` (form B).
fn form_by_id(e: &mut Engine, a: &[u32]) -> Ret {
    let name = guest_text(e, a[0]);
    Ret {
        eax: if name == b"LooseRef" {
            LOOSE_REF
        } else if name == b"PlayerRef" {
            crate::decompile::tests::FORM_A
        } else if name == b"TestQuest" {
            crate::decompile::tests::FORM_B
        } else {
            0
        },
        ..Ret::default()
    }
}

/// The game's default parser as a stand-in: it records the text it was given (from its
/// offset) and whether the end-of-line check was off, takes one number, writes the game's
/// form (`count`, `'n'`, the integer) and moves the offset past the number.
fn default_parse(e: &mut Engine, a: &[u32]) -> Ret {
    let line = a[2];
    let off = e.mem.u32(line + ln::OFFSET);
    let text = guest_text(e, line + ln::TEXT + off);
    let eol_off = state(e).eol_check_off;
    SEEN.with(|s| {
        s.borrow_mut()
            .push(format!("{}|{eol_off}", String::from_utf8_lossy(&text)))
    });
    let digits: Vec<u8> = text
        .iter()
        .copied()
        .skip_while(|c| *c == b' ')
        .take_while(u8::is_ascii_digit)
        .collect();
    let skipped = text.iter().take_while(|c| **c == b' ').count();
    let mut out = vec![u8::from(!digits.is_empty()), 0];
    if !digits.is_empty() {
        let v: i32 = String::from_utf8_lossy(&digits).parse().unwrap();
        out.push(b'n');
        out.extend_from_slice(&v.to_le_bytes());
    }
    let len = e.mem.u32(line + ln::DATA_LEN);
    e.mem.write(line + ln::DATA + len, &out);
    e.mem.set_u32(line + ln::DATA_LEN, len + out.len() as u32);
    e.mem
        .set_u32(line + ln::OFFSET, off + (skipped + digits.len()) as u32);
    Ret {
        eax: 1,
        ..Ret::default()
    }
}

fn nothing(_: &mut Engine, _: &[u32]) -> Ret {
    Ret::default()
}

/// The text of the script buffer: the declarations `GetDeclaredVariableType` reads.
const DECLARATIONS: &str =
    "scn Test\r\nfloat x\r\nint i\r\nref rx\r\nstring_var s\r\narray_var arr\r\n";

/// The engine: the decompiler tests' (the table, the stand-in forms, `Test` and its locals),
/// a string variable `s` (local 4), the compile's stand-ins, and a script buffer whose lists
/// are the script's (the head nodes copied, so what the compile appends both see).
fn engine() -> Engine {
    let mut e = decompile_engine();
    crate::decompile::tests::local(&mut e, 4, "s", 0);
    crate::decompile::tests::local(&mut e, 5, "arr", 0);
    e.map(0x011c_a000, 0x1000);
    e.register(0x005a_a0f0, |_, a| Ret {
        eax: a[0],
        ..Ret::default()
    });
    e.map(AREA, 0x4_0000);
    e.register(FORM_HEAP_ALLOCATE, heap);
    e.register(FORM_HEAP_FREE, nothing);
    e.register(0x00ec_a7f4, upper);
    e.register(FORM_BY_EDITOR_ID, form_by_id);
    e.register(0x005b_1ba0, default_parse);
    e.register(0x0071_b160, |_, _| Ret {
        eax: CONSOLE,
        ..Ret::default()
    });
    e.register(0x0071_d0a0, console_print);
    e.mem.set_u32(CONSOLE, COMPILER);
    e.ext::<Nvse>().main_thread_recorded = true;
    // GetV's record parses with the game's default parser (the stand-in).
    let get_v = record_of(&e, GET_V);
    e.mem.set_u32(get_v + 0x1c, 0x005b_1ba0);
    e.mem.set_u32(get_v + 8, GET_V);
    two_param_record(&mut e);
    e.mem.write(BUF_TEXT, DECLARATIONS.as_bytes());
    e.mem.set_u32(BUF + sb::TEXT, BUF_TEXT);
    e.mem.set_u32(BUF + sb::CURRENT_SCRIPT, SCRIPT);
    e.mem.set_u32(BUF + sb::VAR_COUNT, 5);
    let vars = e.mem.bytes(SCRIPT + sc::VARS, 8);
    e.mem.write(BUF + sb::VARS, &vars);
    let refs = e.mem.bytes(SCRIPT + sc::REFS, 8);
    e.mem.write(BUF + sb::REFS, &refs);
    printed();
    seen();
    e
}

/// `TWO`: an expression-parsed command with a number and an optional string.
fn two_param_record(e: &mut Engine) {
    let record = record_of(e, TWO);
    let at = AREA + 0x700;
    let names = AREA + 0x780;
    e.mem.write(names, b"TwoArgs\0num\0str\0");
    for (i, (name, kind, optional)) in [
        (names + 8, NVSE_NUMBER, false),
        (names + 12, NVSE_STRING, true),
    ]
    .into_iter()
    .enumerate()
    {
        let p = at + 12 * i as u32;
        e.mem.set_u32(p, name);
        e.mem.set_u32(p + 4, kind);
        e.mem.set_u32(p + 8, u32::from(optional));
    }
    e.mem.set_u32(record, names);
    e.mem.set_u32(record + 8, TWO);
    e.mem.set_u16(record + 0x12, 2);
    e.mem.set_u32(record + 0x14, at);
    e.mem.set_u32(record + 0x1c, crate::table::EXPRESSION_PARSE);
}

/// Compiles `text` (the line after its command word) as line 7 with `opcode`'s parse word, as
/// the game's `CompileLine` calls it: its answer and the line's output.
fn compile(e: &mut Engine, opcode: u32, text: &str) -> (bool, Vec<u8>) {
    e.mem.write(LINE, &[0; ln::STRUCT as usize]);
    e.mem.set_u32(LINE + ln::NUMBER, 7);
    e.mem.write(LINE + ln::TEXT, text.as_bytes());
    e.mem.set_u32(LINE + ln::OPCODE, opcode);
    let record = record_of(e, opcode);
    let parse = e.mem.u32(record + 0x1c);
    let count = u32::from(e.mem.u16(record + 0x12));
    let params = e.mem.u32(record + 0x14);
    let ok = e.call(parse, &[count, params, LINE, BUF]).bool();
    let len = e.mem.u32(LINE + ln::DATA_LEN).min(ln::SIZE);
    (ok, e.mem.bytes(LINE + ln::DATA, len))
}

fn compiled(e: &mut Engine, opcode: u32, text: &str) -> Vec<u8> {
    let (ok, bytes) = compile(e, opcode, text);
    assert!(ok, "{text}: {:?}", printed());
    bytes
}

/// The decompiled text of a script of these lines (the variables' lines left out).
fn decompiled(e: &mut Engine, lines: &[(u32, Vec<u8>)]) -> String {
    let script: Vec<Vec<u8>> = lines
        .iter()
        .map(|(op, b)| script_line(*op as u16, b))
        .collect();
    set_script(e, &script);
    assert!(parses(e, SCRIPT));
    let text = String::from_utf8(decompile(e, SCRIPT)).unwrap();
    text.lines()
        .filter(|l| {
            ["Float ", "Int ", "Ref ", "String_var ", "Array_var "]
                .iter()
                .all(|d| !l.starts_with(d))
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Runs a compiled line's command as `EvaluateLine` does; its `al` and result.
fn run(e: &mut Engine, opcode: u32, bytes: &[u8]) -> (u32, f64) {
    e.mem.set_u16(DATA, opcode as u16);
    e.mem.set_u16(DATA + 2, bytes.len() as u16);
    e.mem.write(DATA + 4, bytes);
    e.mem.set_u32(OFFSET, 4);
    e.mem.set_f64(RESULT, 0.0);
    e.mem.set_u32(SCRIPT + 0x30, DATA);
    let record = record_of(e, opcode);
    let params = e.mem.u32(record + 0x14);
    let execute = e.mem.u32(record + 0x18);
    let ok = e
        .call(execute, &[params, DATA, 0, 0, SCRIPT, LIST, RESULT, OFFSET])
        .eax;
    (ok & 0xff, e.mem.f64(RESULT))
}

const LET: u32 = 0x1539;
const EVAL: u32 = 0x153a;
const WHILE: u32 = 0x153b;
const LOOP: u32 = 0x153c;
const FUNCTION: u32 = 0x1544;
const CALL: u32 = 0x1545;

fn v(var_type: u8, idx: u16) -> Vec<u8> {
    let mut b = vec![b'V', var_type, 0, 0];
    b.extend_from_slice(&idx.to_le_bytes());
    b
}

/// One argument (its length word counting itself) of the given tokens.
fn arg(tokens: &[&[u8]]) -> Vec<u8> {
    let body = tokens.concat();
    let mut b = (body.len() as u16 + 2).to_le_bytes().to_vec();
    b.extend(body);
    b
}

/// A count byte and the arguments.
fn args_of(args: &[Vec<u8>]) -> Vec<u8> {
    let mut b = vec![args.len() as u8];
    b.extend(args.concat());
    b
}

// ---- (b) bytes by the rules ----

/// Numbers by the rules of `ScriptToken::Write`: a whole number below 0x100 as `B`, below
/// 0x10000 as `I`, else `L`; a fraction as `Z` and its double; hexadecimal and binary forms;
/// operators in postfix order, `*` before `+`.
#[test]
fn numbers_and_precedence_compile_as_the_rules_write_them() {
    let mut e = engine();
    assert_eq!(
        compiled(&mut e, EVAL, "1 + 2 * 3"),
        args_of(&[arg(&[
            b"B\x01",
            b"B\x02",
            b"B\x03",
            &[op::MULTIPLY],
            &[op::ADD]
        ])])
    );
    let mut z = vec![b'Z'];
    z.extend_from_slice(&2.5f64.to_le_bytes());
    assert_eq!(
        compiled(&mut e, EVAL, "300 + 70000 + 2.5 + 0x1F + 0b101"),
        args_of(&[arg(&[
            b"I\x2c\x01",
            b"L\x70\x11\x01\x00",
            &[op::ADD],
            &z,
            &[op::ADD],
            b"B\x1f",
            &[op::ADD],
            b"B\x05",
            &[op::ADD],
        ])])
    );
    // `-` after an operand subtracts, else negates; `^` before the unary minus.
    assert_eq!(
        compiled(&mut e, EVAL, "-x - 2 ^ 3"),
        args_of(&[arg(&[
            &v(0, 1),
            &[op::NEGATION],
            b"B\x02",
            b"B\x03",
            &[op::EXPONENT],
            &[op::SUBTRACT],
        ])])
    );
    assert!(printed().is_empty());
}

/// `Let`: the variable (`V`, its type, reference 0, its index), the assignment last, right to
/// left (`x := i := 2`); the compound assignments; parentheses group.
#[test]
fn let_writes_the_assignment_last() {
    let mut e = engine();
    assert_eq!(
        compiled(&mut e, LET, "x := (1 + 2) * 3"),
        args_of(&[arg(&[
            &v(0, 1),
            b"B\x01",
            b"B\x02",
            &[op::ADD],
            b"B\x03",
            &[op::MULTIPLY],
            &[op::ASSIGNMENT],
        ])])
    );
    assert_eq!(
        compiled(&mut e, LET, "x := i := 2"),
        args_of(&[arg(&[
            &v(0, 1),
            &v(1, 2),
            b"B\x02",
            &[op::ASSIGNMENT],
            &[op::ASSIGNMENT],
        ])])
    );
    assert_eq!(
        compiled(&mut e, LET, "i += 4"),
        args_of(&[arg(&[&v(1, 2), b"B\x04", &[op::PLUS_EQUALS]])])
    );
    // A lone `=` is the assignment, and takes two characters as `:=` does.
    assert_eq!(
        compiled(&mut e, LET, "x = 5"),
        args_of(&[arg(&[&v(0, 1), b"B\x05", &[op::ASSIGNMENT]])])
    );
    // No assignment last: Let refuses the line, with no message at run time.
    let (ok, _) = compile(&mut e, LET, "x + 1");
    assert!(!ok);
    assert!(printed().is_empty());
}

/// Strings: a literal with `%q` and `%r` made a quote and a line break (`%%` kept); a string
/// variable through `$`; an unquoted word that names nothing is a string, with xNVSE's warning.
#[test]
fn strings_and_unquoted_words() {
    let mut e = engine();
    assert_eq!(
        compiled(&mut e, EVAL, "\"a%qb%r%%\" == $s"),
        args_of(&[arg(&[
            b"S\x06\x00a\"b\n%%",
            &v(2, 4),
            &[op::TO_STRING],
            &[op::EQUALS],
        ])])
    );
    assert!(printed().is_empty());
    assert_eq!(
        compiled(&mut e, EVAL, "xyz"),
        args_of(&[arg(&[b"S\x03\x00xyz"])])
    );
    assert_eq!(
        printed(),
        ["Unquoted argument 'xyz' will be treated as string by default. Check spelling if a form or variable was intended."]
    );
    // `->` takes a word as a string literal, without the warning.
    assert_eq!(
        compiled(&mut e, EVAL, "arr->key"),
        args_of(&[arg(&[&v(3, 5), b"S\x03\x00key", &[op::MEMBER_ACCESS]])])
    );
    assert!(printed().is_empty());
}

/// References: the player resolves to the game's player reference (appended to the list as
/// reference 4), the reference variable is a variable of type `ref`, `_` the empty argument.
#[test]
fn references_variables_and_the_empty_argument() {
    let mut e = engine();
    assert_eq!(
        compiled(&mut e, EVAL, "player"),
        args_of(&[arg(&[b"R\x04\x00"])])
    );
    assert_eq!(e.mem.u32(BUF + sb::NUM_REFS), 1);
    // A second use finds the same entry.
    assert_eq!(
        compiled(&mut e, EVAL, "PLAYER"),
        args_of(&[arg(&[b"R\x04\x00"])])
    );
    assert_eq!(e.mem.u32(BUF + sb::NUM_REFS), 1);
    assert_eq!(compiled(&mut e, EVAL, "rx"), args_of(&[arg(&[&v(4, 3)])]));
    // `_` is an empty argument: only an unchecked parameter takes it.
    assert_eq!(
        compiled(&mut e, CALL, "rx _"),
        [&[1u8][..], &arg(&[&v(4, 3)]), &args_of(&[arg(&[b"K"])])].concat()
    );
}

/// A command inside an expression: `X`, no reference, its opcode, then its arguments' length
/// word and the arguments its own parser wrote, on the text after it with the end-of-line
/// check off; the expression goes on after what it took.
#[test]
fn a_command_inside_an_expression_parses_with_its_own_parser() {
    let mut e = engine();
    let mut get_v = vec![b'X', 0, 0];
    get_v.extend_from_slice(&(GET_V as u16).to_le_bytes());
    get_v.extend_from_slice(&[9, 0, 1, 0, b'n', 7, 0, 0, 0]);
    assert_eq!(
        compiled(&mut e, EVAL, "GetV 7 + 1"),
        args_of(&[arg(&[&get_v, b"B\x01", &[op::ADD]])])
    );
    assert_eq!(seen(), [" 7 + 1|true"]);
    assert!(!state(&mut e).eol_check_off, "the check is back on");
    // An xNVSE command inside: its count byte and its argument.
    let mut type_of = vec![b'X', 0, 0, 0x43, 0x15];
    let inner = args_of(&[arg(&[&v(0, 1)])]);
    type_of.extend_from_slice(&(inner.len() as u16 + 2).to_le_bytes());
    type_of.extend(inner);
    assert_eq!(
        compiled(&mut e, EVAL, "TypeOf x"),
        args_of(&[arg(&[&type_of])])
    );
}

/// Arguments: two parameters each an expression; a comparison outside parentheses ends the
/// arguments (from the start of the text, the last character before the operator left out, as
/// xNVSE cuts it), so `TwoArgs 1 "b" == 0` takes `"b"` (a string reads to its quote) while
/// `TwoArgs 1 2 == 0` loses the 2.
#[test]
fn arguments_end_at_a_comparison() {
    let mut e = engine();
    assert_eq!(
        compiled(&mut e, TWO, "1 \"bc\""),
        args_of(&[arg(&[b"B\x01"]), arg(&[b"S\x02\x00bc"])])
    );
    assert_eq!(
        compiled(&mut e, TWO, "1 \"bc\" == 0"),
        args_of(&[arg(&[b"B\x01"]), arg(&[b"S\x02\x00bc"])])
    );
    // A number must not be given for the string.
    let (ok, _) = compile(&mut e, TWO, "1 2");
    assert!(!ok);
    assert_eq!(
        printed(),
        ["Error on line 7\n\nInvalid expression for parameter 2. Expected str."]
    );
    assert_eq!(
        compiled(&mut e, TWO, "(4 == 5)"),
        args_of(&[arg(&[b"B\x04", b"B\x05", &[op::EQUALS]])])
    );
    assert_eq!(
        compiled(&mut e, TWO, "1 2 == 0"),
        args_of(&[arg(&[b"B\x01"])])
    );
}

/// Variables declared inside an expression get the next index, join the buffer's list and keep
/// their type; the same name again with another type is refused.
#[test]
fn variables_declared_in_an_expression() {
    let mut e = engine();
    assert_eq!(
        compiled(&mut e, LET, "int j := 5"),
        args_of(&[arg(&[&v(1, 6), b"B\x05", &[op::ASSIGNMENT]])])
    );
    assert_eq!(e.mem.u32(BUF + sb::VAR_COUNT), 6);
    let vars: Vec<(u32, Vec<u8>)> = list_items(&e, BUF + sb::VARS)
        .into_iter()
        .map(|info| (e.mem.u32(info), string_cstr(&e, info + 0x18)))
        .collect();
    assert_eq!(vars.last(), Some(&(6, b"j".to_vec())));
    assert_eq!(
        compiled(&mut e, LET, "int j := 6"),
        args_of(&[arg(&[&v(1, 6), b"B\x06", &[op::ASSIGNMENT]])])
    );
    let (ok, _) = compile(&mut e, LET, "float j := 6");
    assert!(!ok);
    assert_eq!(
        printed(),
        ["Variable redefinition with a different type (saw first 'int', then 'float')"]
    );
    let (ok, _) = compile(&mut e, LET, "int Let := 1");
    assert!(!ok);
    assert_eq!(
        printed(),
        ["Error on line 7\n\nInvalid variable name Let: Command with that name already exists."]
    );
}

/// The errors' texts, line number first (`Error line 7`), and the line refused.
#[test]
fn errors_print_xnvses_texts() {
    let mut e = engine();
    for (text, want) in [
        ("(1 + 2", "Error line 7\n\nMismatched brackets."),
        ("\"abc", "Error line 7\n\nMismatched quotes."),
        (
            "\"a\" + 1",
            "Error line 7\n\nInvalid operands for operator +",
        ),
        ("1 )", "Error line 7\n\nMismatched brackets."),
        ("rx.zz", "Error line 7\n\nUnknown variable 'zz'."),
        (
            "1 2",
            "Error line 7\n\nMore args provided than expected by function or command.",
        ),
        ("x->", "Error line 7\n\nToo many operators."),
    ] {
        let (ok, _) = compile(&mut e, EVAL, text);
        assert!(!ok, "{text}");
        let lines = printed();
        assert_eq!(
            lines.first().map(String::as_str),
            Some(want),
            "{text}: {lines:?}"
        );
    }
    let (ok, _) = compile(&mut e, LET, "");
    assert!(!ok);
    assert_eq!(
        printed(),
        ["Error line 7\n\nMissing required parameter 'expression' for parameter #'1'."]
    );
    for (text, want) in [
        ("0x", "Error on line 7\n\nInvalid hexadecimal syntax"),
        ("0x1G", "Error on line 7\n\nInvalid hexadecimal syntax"),
        ("0b", "Error on line 7\n\nInvalid binary syntax"),
        ("0b102", "Error on line 7\n\nInvalid binary syntax"),
        (
            "0x100000000",
            "Error on line 7\n\nInvalid hexadecimal syntax",
        ),
    ] {
        let (ok, _) = compile(&mut e, EVAL, text);
        assert!(!ok, "{text}");
        assert_eq!(printed().first().map(String::as_str), Some(want), "{text}");
    }
}

/// The output is held to its 0x200 bytes: a line that would pass it fails with xNVSE's
/// message, never past the buffer.
#[test]
fn the_output_is_held_to_its_buffer() {
    let mut e = engine();
    let long = vec!["x"; 100].join("+");
    let (ok, bytes) = compile(&mut e, EVAL, &long);
    assert!(!ok);
    assert!(bytes.len() < 0x200);
    let lines = printed();
    assert!(lines
        .iter()
        .any(|l| l == "Line data buffer overflow! To fix this make the line shorter in length."));
    assert_eq!(
        e.mem.u32(LINE + ln::NUMBER),
        7,
        "the line's fields are untouched"
    );
}

/// `while`: room for the jump word (where the script's data will hold it is kept), then the
/// condition; `loop` writes the offset after itself there. A `loop` with no open loop fails.
#[test]
fn loops_record_their_jump() {
    let mut e = engine();
    let data = AREA + 0x2_0000;
    e.mem.set_u32(BUF + sb::DATA, data);
    e.mem.set_u32(BUF + sb::DATA_OFFSET, 0x10);
    let bytes = compiled(&mut e, WHILE, "x < 3");
    assert_eq!(bytes[..4], [0; 4]);
    assert_eq!(
        bytes[4..],
        args_of(&[arg(&[&v(0, 1), b"B\x03", &[op::LESS_THAN]])])[..]
    );
    e.mem.set_u32(BUF + sb::DATA_OFFSET, 0x40);
    compiled(&mut e, LOOP, "");
    // The jump word sits after the while line's opcode and length (0x10 + 4).
    assert_eq!(e.mem.u32(data + 0x14), 0x44);
    let (ok, _) = compile(&mut e, LOOP, "");
    assert!(!ok);
}

/// `Begin Function {i, float y}`: the version, the parameters by index and type (`y` declared
/// there, a new local), no local arrays.
#[test]
fn a_function_definition_lists_its_parameters() {
    let mut e = engine();
    let text = format!("{DECLARATIONS}begin function {{i, float y}}\r\nend\r\n");
    e.mem.write(BUF_TEXT, text.as_bytes());
    e.mem.set_u8(BUF_TEXT + text.len() as u32, 0);
    assert_eq!(
        compiled(&mut e, FUNCTION, "{i, float y}"),
        [1, 2, 2, 0, vt::INTEGER, 6, 0, vt::FLOAT, 0]
    );
    // An undeclared parameter.
    let text = format!("{DECLARATIONS}begin function {{nope}}\r\n");
    e.mem.write(BUF_TEXT, text.as_bytes());
    e.mem.set_u8(BUF_TEXT + text.len() as u32, 0);
    let (ok, _) = compile(&mut e, FUNCTION, "{nope}");
    assert!(!ok);
    assert!(
        printed()[0].starts_with("Error line 7\n\nCould not parse user function parameter list")
    );
}

/// `Call`: a function in a reference variable has no text to check its arguments with: ten
/// unchecked arguments after the version and the function's expression.
#[test]
fn call_through_a_variable_takes_unchecked_arguments() {
    let mut e = engine();
    assert_eq!(
        compiled(&mut e, CALL, "rx 1, \"a\""),
        [
            &[1u8][..],
            &arg(&[&v(4, 3)]),
            &args_of(&[arg(&[b"B\x01"]), arg(&[b"S\x01\x00a"])]),
        ]
        .concat()
    );
}

/// A lambda: `begin ... end` compiled by the game's compiler as a script of its own (the
/// stand-in writes its data), its token `F`, the data's length and the data; the one-line form
/// `{} => 5` is made three lines first; its script is deleted when the outermost parse ends.
#[test]
fn lambdas_compile_as_scripts_of_their_own() {
    thread_local! {
        static LAMBDA: RefCell<(String, String, u32)> = const { RefCell::new((String::new(), String::new(), 0)) };
    }
    fn compile_stand_in(e: &mut Engine, a: &[u32]) -> Ret {
        let (this, script, buf) = (a[0], a[1], a[2]);
        assert_eq!(this, COMPILER);
        let text = String::from_utf8_lossy(&guest_text(e, e.mem.u32(buf + sb::TEXT))).into_owned();
        let name = String::from_utf8_lossy(&string_cstr(e, buf + sb::NAME)).into_owned();
        LAMBDA.with(|l| *l.borrow_mut() = (text, name, e.mem.u32(buf + sb::CUR_LINE)));
        let data = AREA + 0x3_0000;
        e.mem.write(data, &[0x1d, 0, 0, 0, 0xab, 0xcd]);
        e.mem.set_u32(script + sc::DATA, data);
        e.mem.set_u32(script + sc::DATA_LEN, 6);
        Ret {
            eax: 1,
            ..Ret::default()
        }
    }
    thread_local! {
        static DELETED: RefCell<Vec<u32>> = const { RefCell::new(Vec::new()) };
    }
    let mut e = engine();
    e.register(0x005a_e490, |e, a| {
        let data = heap(e, &[0x4000]).eax;
        e.mem.set_u32(a[0] + sb::DATA, data);
        Ret::default()
    });
    e.register(0x005a_e5c0, nothing);
    e.register(0x005a_eb90, compile_stand_in);
    e.register(0x005a_a1a0, |_, a| {
        DELETED.with(|d| d.borrow_mut().push(a[0]));
        Ret::default()
    });
    let f = [&b"F"[..], &6u32.to_le_bytes(), &[0x1d, 0, 0, 0, 0xab, 0xcd]].concat();
    e.mem.set_u32(0x011c_adbc, 0x1234);
    // A lambda is a form: a reference variable takes it.
    assert_eq!(
        compiled(
            &mut e,
            LET,
            "rx := begin function {} SetFunctionValue 5 end"
        ),
        args_of(&[arg(&[&v(4, 3), &f, &[op::ASSIGNMENT]])])
    );
    let (text, name, line) = LAMBDA.with(|l| l.borrow().clone());
    assert_eq!(text, "begin function {} SetFunctionValue 5 end");
    assert_eq!((name.as_str(), line), ("LambdaAtLine6", 6));
    assert_eq!(DELETED.with(|d| d.borrow().len()), 1);
    assert!(state(&mut e).lambda_parents.is_empty());
    assert_eq!(e.mem.u32(0x011c_adbc), 0x1234, "the block jump is back");
    assert!(!state(&mut e).skip_ref_check);
    // The one-line form.
    assert_eq!(
        compiled(&mut e, LET, "rx := {} => 5"),
        args_of(&[arg(&[&v(4, 3), &f, &[op::ASSIGNMENT]])])
    );
    let (text, name, line) = LAMBDA.with(|l| l.borrow().clone());
    assert_eq!(text, "begin function {}\r\nSetFunctionValue 5\r\nend");
    assert_eq!((name.as_str(), line), ("LambdaAtLine7", 5));
    // A lambda whose compile fails: the line fails, its script is deleted at once.
    e.register(0x005a_eb90, nothing);
    let (ok, _) = compile(&mut e, LET, "rx := begin function {} end");
    assert!(!ok);
    assert_eq!(DELETED.with(|d| d.borrow().len()), 3);
}

// ---- (a) round trips ----

/// Each line compiled, read back by the decompiler, and the text compiled again gives the same
/// bytes; the decompiler's text is xNVSE's form of the line.
#[test]
fn compiled_lines_decompile_and_compile_back() {
    let mut e = engine();
    for (opcode, text, want) in [
        (LET, "x := 1 + 2 * 3", "Let x := 1 + 2 * 3"),
        (LET, "x := (1 + 2) * 3", "Let x := (1 + 2) * 3"),
        (EVAL, "x > 1 && i < 2", "eval x > 1 && i < 2"),
        (EVAL, "\"a\" == $s", "eval \"a\" == $s"),
        (EVAL, "-x", "eval -x"),
        (EVAL, "GetV 7 + 1", "eval (GetV 7) + 1"),
        (EVAL, "PlayerRef", "eval PlayerRef"),
    ] {
        let bytes = compiled(&mut e, opcode, text);
        let got = decompiled(&mut e, &[(opcode, bytes.clone())]);
        assert_eq!(got, want, "{text}");
        let again = want.split_once(' ').unwrap().1;
        assert_eq!(compiled(&mut e, opcode, again), bytes, "{again}");
    }
}

/// Compiled lines run: `Let` assigns what the expression gives, `eval` answers it.
#[test]
fn compiled_lines_evaluate() {
    let mut e = engine();
    // Local 1's `ScriptLocal` (`add_local`'s place for it).
    let x = 0x0300_6000 + 0x20;
    let bytes = compiled(&mut e, LET, "x := 1 + 2 * 3 ^ 2");
    run(&mut e, LET, &bytes);
    assert_eq!(local_value(&e, x), 19.0);
    let bytes = compiled(&mut e, LET, "x -= 0x10 | 0b1");
    run(&mut e, LET, &bytes);
    assert_eq!(local_value(&e, x), 2.0);
    let bytes = compiled(&mut e, EVAL, "x == 2 && (x + 1) * 2 == 6");
    assert_eq!(run(&mut e, EVAL, &bytes).1, 1.0);
}

// ---- the hooks ----

fn installed() -> Engine {
    let mut e = engine();
    crate::compile_hooks::install(&mut e);
    e
}

fn set_buf_text(e: &mut Engine, text: &str) {
    e.mem.write(BUF_TEXT, text.as_bytes());
    e.mem.set_u8(BUF_TEXT + text.len() as u32, 0);
}

/// The start and end of a compile: the script on the stack, its error code and compiled flag
/// cleared, `#version` tags made comments and given their `#` back in the script's text at the
/// end; the variable types forgotten when the stack empties.
#[test]
fn a_compile_starts_and_ends() {
    let mut e = installed();
    let text = "scn Test\r\n;#version(\"nvse\", 6, 4)\r\nint i\r\n";
    set_buf_text(&mut e, text);
    e.mem.set_u32(SCRIPT + sc::TEXT, BUF_TEXT);
    e.mem.set_u32(BUF + sb::ERROR, 3);
    e.mem.set_u8(SCRIPT + sc::COMPILED, 1);
    assert_eq!(e.site(0x005a_ebb1, &[SCRIPT, BUF]), Some(1));
    assert_eq!(crate::compile_hooks::compiling(&mut e), [SCRIPT]);
    assert_eq!(e.mem.u32(BUF + sb::ERROR), 0);
    assert_eq!(e.mem.u8(SCRIPT + sc::COMPILED), 0);
    let at = text.find('#').unwrap() as u32;
    assert_eq!(e.mem.u8(BUF_TEXT + at), b';');
    // The preprocessor saw `int i`.
    assert_eq!(
        crate::compile_hooks::saved_var_type(&mut e, SCRIPT, b"i"),
        vt::INTEGER
    );
    assert_eq!(e.site(0x005a_eda0, &[1, BUF]), Some(1));
    assert!(crate::compile_hooks::compiling(&mut e).is_empty());
    assert_eq!(e.mem.u8(BUF_TEXT + at), b'#');
    assert_eq!(
        crate::compile_hooks::saved_var_type(&mut e, SCRIPT, b"i"),
        vt::INVALID
    );
    // A tag naming a plugin that is not built in fails the compile.
    set_buf_text(&mut e, "#version(\"Nope\", 1)\r\n");
    assert_eq!(e.site(0x005a_ebb1, &[SCRIPT, BUF]), Some(0));
    assert_eq!(
        printed(),
        ["Error in script 00000000 in mod Unknown or deleted script\nScript compilation failed: No plugin with name nope could be found."]
    );
    assert_eq!(e.site(0x005a_eda0, &[0, BUF]), Some(0));
}

/// The preprocessor's checks and their texts.
#[test]
fn the_preprocessor_checks_blocks_loops_and_declarations() {
    let mut e = installed();
    for (text, want) in [
        (
            "if x\r\nendif\r\nendif\r\n",
            Some("Invalid If/EndIf block structure on line 3."),
        ),
        (
            "while x\r\nendif\r\n",
            Some("Invalid Loop block structure on line 2."),
        ),
        (
            "break\r\n",
            Some("Error line 1:\nFunction break must be called from within a loop."),
        ),
        (
            "while x\r\nreturn\r\nloop\r\n",
            Some("Error line 2:\nReturn cannot be called within the body of a loop."),
        ),
        ("if x\r\n", Some("Error: Mismatched block structure.")),
        (
            "array_var a\r\nset a to 1\r\n",
            Some("Error line 2:\nSet may not be used to assign to an array variable"),
        ),
        (
            "int a\r\nfloat a\r\n",
            Some("Variable redefinition with a different type (saw first 'int', then 'float')"),
        ),
        (
            "if x ; endif\r\n\r\nendif\r\nwhile x\r\ncontinue\r\nloop\r\nint b = 1\r\nfloat b\r\n",
            None,
        ),
        ("/* if x */ int c\r\n", None),
    ] {
        set_buf_text(&mut e, text);
        let answer = e.site(0x005a_ebb1, &[SCRIPT, BUF]);
        let lines = printed();
        match want {
            Some(w) => {
                assert_eq!(answer, Some(0), "{text}");
                assert_eq!(lines, [w], "{text}");
                assert_eq!(e.mem.u32(BUF + sb::ERROR), 1);
            }
            None => {
                assert_eq!(answer, Some(1), "{text}: {lines:?}");
                assert!(lines.is_empty(), "{text}: {lines:?}");
            }
        }
        e.site(0x005a_eda0, &[0, BUF]);
    }
}

/// The game's compiler finds xNVSE's commands by name (`Let`, `ar_Size`, a short name), a word
/// that names none falls to the console commands; the end-of-line check is off only during a
/// nested parse; variables may share a form's name; the `if` parentheses check is passed over.
#[test]
fn the_compilers_seams_answer_as_xnvses_hooks() {
    let mut e = installed();
    let word = AREA + 0x2_8000;
    for (name, want) in [
        ("Let", Some(0x1539)),
        ("LET", Some(0x1539)),
        ("nothing", None),
    ] {
        e.mem.write(word, &[0; 0x214]);
        e.mem.write(word, name.as_bytes());
        let answer = e.site(0x005b_19ba, &[word]);
        match want {
            Some(op) => {
                assert_eq!(answer, Some(1), "{name}");
                assert_eq!(e.mem.u8(word + 0x204), b'X');
                assert_eq!(e.mem.u32(word + 0x208), op);
            }
            None => {
                assert_eq!(answer, Some(0));
                assert_eq!(e.mem.u8(word + 0x204), 0);
            }
        }
    }
    assert_eq!(e.site(0x005b_3a8d, &[]), None);
    state(&mut e).eol_check_off = true;
    assert_eq!(e.site(0x005b_3a8d, &[]), Some(1));
    assert_eq!(e.site(0x005a_fedb, &[word]), Some(0));
    assert_eq!(e.site(0x005b_0a3f, &[]), Some(1));
    // The double type: the float case, and its type table entry made a variable's.
    assert_eq!(e.site(0x005b_2fa2, &[]), Some(1));
    assert_eq!(e.mem.u16(0x0118_cf34), 1);
    assert_eq!(e.site(0x005a_ed78, &[BUF]), None);
    state(&mut e).skip_ref_check = true;
    assert_eq!(e.site(0x005a_ed78, &[BUF]), Some(1));
}

/// The script data grows (doubled, zeroed, the data so far copied) when a line and ten bytes
/// would reach the heap block's size, and stays below.
#[test]
fn the_script_data_grows_past_its_block() {
    let mut e = installed();
    e.register(0x00aa_44c0, |_, _| Ret {
        eax: 0x20,
        ..Ret::default()
    });
    let data = AREA + 0x2_0000;
    e.mem.write(data, &[7; 0x20]);
    e.mem.set_u32(BUF + sb::DATA, data);
    e.mem.set_u32(BUF + sb::DATA_OFFSET, 0x10);
    e.mem.set_u32(LINE + ln::DATA_LEN, 5);
    assert_eq!(e.site(0x005b_0f68, &[BUF, LINE]), Some(1));
    assert_eq!(e.mem.u32(BUF + sb::DATA), data, "0x10 + 5 + 10 < 0x20");
    e.mem.set_u32(LINE + ln::DATA_LEN, 6);
    assert_eq!(e.site(0x005b_0f68, &[BUF, LINE]), Some(1));
    let grown = e.mem.u32(BUF + sb::DATA);
    assert_ne!(grown, data);
    assert_eq!(
        e.mem.bytes(grown, 0x40),
        [vec![7; 0x10], vec![0; 0x30]].concat()
    );
}

// ---- the text readers ----

/// `GetDeclaredVariableType`: declarations with commas, a function's typed parameters, the
/// first found kept; a lambda's parent's declarations.
#[test]
fn declared_types_come_from_the_text() {
    let mut e = engine();
    let text = b"scn A\nshort a, b ; comment\nbegin function {ref r, c}\nstring_var d\nfloat p q\n";
    let t =
        |e: &mut Engine, n: &[u8]| crate::compile_hooks::declared_variable_type(e, n, text, 0x77);
    assert_eq!(t(&mut e, b"b"), vt::INTEGER);
    assert_eq!(t(&mut e, b"r"), vt::REF);
    assert_eq!(t(&mut e, b"c"), vt::INVALID);
    assert_eq!(t(&mut e, b"comment"), vt::INVALID);
    assert_eq!(t(&mut e, b"D"), vt::STRING);
    // Only a comma goes on to the next name.
    assert_eq!(t(&mut e, b"p"), vt::FLOAT);
    assert_eq!(t(&mut e, b"q"), vt::INVALID);
    // A lambda's variable from its parent's text.
    let parent_text = AREA + 0x2_9000;
    e.mem.write(parent_text, b"float pv\n\0");
    e.mem.set_u32(SCRIPT + sc::TEXT, parent_text);
    state(&mut e).lambda_parents.push((0x99, SCRIPT));
    assert_eq!(
        crate::compile_hooks::declared_variable_type(&mut e, b"pv", b"", 0x99),
        vt::FLOAT
    );
}

/// `ScriptTokenizer`: `;` and `/* */` comments, strings as one token, blank lines passed.
#[test]
fn the_script_tokenizer_leaves_comments_out() {
    use crate::compile_hooks::ScriptTokenizer;
    let text = b"  a b;c

/* x
 y */ d \"e f\"
; g
h/* i */ j
/*;*/ k
m n;
";
    let mut t = ScriptTokenizer::new(text);
    let mut lines = Vec::new();
    while t.load_next_line() {
        let mut words = Vec::new();
        loop {
            let w = t.next_token();
            if w.is_empty() {
                break;
            }
            words.push(String::from_utf8(w).unwrap());
        }
        lines.push(words.join(","));
    }
    assert_eq!(lines, ["a,b;c", "d,\"e f\"", "h,j", "m,n"]);
}

// ---- (c) the player's plugins ----

/// A compiled script's lines (`SCDA`): the opcode, whether it is called on a reference (`0x1C`
/// first), the bytes after the length word.
fn compiled_lines(data: &[u8]) -> Vec<(u32, bool, Vec<u8>)> {
    let word = |i: usize| usize::from(u16::from_le_bytes([data[i], data[i + 1]]));
    let mut out = Vec::new();
    let mut i = 0;
    while i + 4 <= data.len() {
        let (op, len, at, on_ref) = if word(i) == 0x1c {
            if i + 8 > data.len() {
                break;
            }
            (word(i + 4), word(i + 6), i + 8, true)
        } else {
            (word(i), word(i + 2), i + 4, false)
        };
        let end = (at + len).min(data.len());
        out.push((op as u32, on_ref, data[at..end].to_vec()));
        i = at + len;
    }
    out
}

/// A script source's lines that compile to a line of data: comments left out, blank lines and
/// declarations passed over.
fn code_lines(text: &[u8]) -> Vec<String> {
    let text = crate::compile_hooks::remove_multi_line_comments(text);
    let mut out = Vec::new();
    for raw in text.split(|&c| c == b'\n') {
        let mut line = Vec::new();
        let mut quoted = false;
        for &c in raw {
            if c == b'"' {
                quoted = !quoted;
            }
            if c == b';' && !quoted {
                break;
            }
            line.push(c);
        }
        let line = String::from_utf8_lossy(&line).trim().to_string();
        let first = line
            .split([' ', '\t'])
            .next()
            .unwrap_or("")
            .to_ascii_lowercase();
        let declaration = [
            "short",
            "long",
            "float",
            "int",
            "ref",
            "reference",
            "string_var",
            "array_var",
        ]
        .contains(&first.as_str());
        if !line.is_empty() && !declaration {
            out.push(line);
        }
    }
    out
}

thread_local! {
    static NEEDS_GAME_PARSER: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    static PLUGIN_NAMES: RefCell<Vec<(Vec<u8>, u32)>> = const { RefCell::new(Vec::new()) };
}

/// The lines of the player's scripts that xNVSE's compiler wrote (the commands parsed by its
/// parse routines), compiled again here from their source (`SCTX`) and compared byte for byte
/// with the plugin's compiled data (`SCDA`). The plugins are `NV_COMPILE_PLUGINS` (names in
/// `NV_DATA`, separated by commas); the exe beside `NV_DATA` gives the game's records. A line
/// called on a reference, or whose command calls the game's own parser inside it, is passed
/// over (counted). With `NV_COMPILE_DUMP` (a file path) every compared line is written there.
/// Nothing of the files is kept in the repository.
#[test]
#[ignore = "needs the player's FalloutNV.exe and plugins compiled with xNVSE (NV_DATA, NV_COMPILE_PLUGINS)"]
fn the_players_xnvse_lines_compile_to_their_bytes() {
    use crate::decompile::tests::{real_cast, real_lookup, real_script, RealForms};
    let data = std::path::PathBuf::from(std::env::var("NV_DATA").expect("NV_DATA"));
    let plugins = std::env::var("NV_COMPILE_PLUGINS").expect("NV_COMPILE_PLUGINS");
    let exe = data
        .parent()
        .expect("the game's folder")
        .join("FalloutNV.exe");
    let mut e = Engine::with_exe(&exe).expect("FalloutNV.exe");
    crate::table::install(&mut e);
    let vtable = e.mem.alloc(0x200);
    e.mem.write(vtable, &[0; 0x200]);
    e.mem
        .set_u32(vtable + 0x130, crate::decompile::tests::NAME_FN);
    e.register(
        crate::decompile::tests::NAME_FN,
        crate::decompile::tests::name_fn,
    );
    let is_ref = 0x7eff_e200;
    e.mem.set_u32(vtable + 0xf0, is_ref);
    e.register(is_ref, |e, a| Ret {
        eax: u32::from((0x3a..=0x40).contains(&e.mem.u8(a[0] + 4))),
        ..Ret::default()
    });
    e.register(DYNAMIC_CAST, real_cast);
    e.register(LOOKUP_FORM, real_lookup);
    e.register(0x0041_0220, |_, _| Ret::default());
    e.register(FORM_HEAP_ALLOCATE, |e, a| Ret {
        eax: e.mem.alloc(a[0].max(1)),
        ..Ret::default()
    });
    e.register(FORM_HEAP_FREE, nothing);
    e.register(0x00ec_a7f4, upper);
    e.register(0x0071_b160, nothing);
    e.register(0x005b_1ba0, |_, _| {
        NEEDS_GAME_PARSER.with(|n| n.set(true));
        Ret::default()
    });
    e.register(FORM_BY_EDITOR_ID, |e, a| {
        let name = guest_text(e, a[0]);
        let id = PLUGIN_NAMES.with(|p| {
            p.borrow()
                .iter()
                .find(|(n, _)| n.eq_ignore_ascii_case(&name))
                .map(|(_, id)| *id)
        });
        Ret {
            eax: id.map_or(0, |id| real_lookup(e, &[id]).eax),
            ..Ret::default()
        }
    });
    let mut types = std::collections::HashMap::new();
    for t in 0..0x79u32 {
        let at = e
            .mem
            .u32(crate::decompile::tests::FORM_TYPE_TABLE + 12 * t + 4);
        if at != 0 {
            let s = e.mem.bytes(at, 4);
            types.insert([s[0], s[1], s[2], s[3]], t as u8);
        }
    }
    let parse_words = [
        crate::table::EXPRESSION_PARSE,
        crate::table::LET_PARSE,
        crate::table::BEGIN_LOOP_PARSE,
        crate::table::LOOP_PARSE,
        crate::table::NULL_PARSE,
        crate::table::FUNCTION_PARSE,
        crate::table::CALL_PARSE,
    ];
    let ours = |e: &Engine, op: u32| {
        let r = record_of(e, op);
        r != 0 && parse_words.contains(&e.mem.u32(r + 0x1c))
    };
    let buf = e.mem.alloc(sb::SIZE);
    let line = e.mem.alloc(ln::STRUCT);
    let (mut compared, mut passed_over) = (0, 0);
    let mut mismatches = Vec::new();
    let mut dump = String::new();
    for file in plugins.split(',').map(str::trim).filter(|n| !n.is_empty()) {
        let plugin = esm::Plugin::open(data.join(file)).expect("the plugin");
        let mut forms = RealForms {
            by_id: std::collections::HashMap::new(),
            vtable,
        };
        for entry in plugin.records_of_type(esm::FourCC(*b"SCPT")) {
            let scpt = plugin.record(entry).expect("SCPT");
            let sub = |k: &[u8; 4]| {
                scpt.get(esm::FourCC(*k))
                    .map(|s| s.data.clone())
                    .unwrap_or_default()
            };
            let (source, code) = (sub(b"SCTX"), sub(b"SCDA"));
            let lines = compiled_lines(&code);
            if !lines.iter().any(|(op, _, _)| ours(&e, *op)) {
                continue;
            }
            let texts = code_lines(&source);
            if texts.len() != lines.len() {
                passed_over += 1;
                continue;
            }
            let script_name = scpt.editor_id().unwrap_or_default();
            let script = real_script(&mut e, &mut forms, &plugin, &types, &scpt);
            // The reference list's names: the forms' editor ids (the player's as the source
            // writes it).
            let lower = String::from_utf8_lossy(&source).to_ascii_lowercase();
            let mut names = Vec::new();
            for r in list_items(&e, script + sc::REFS) {
                let form = e.mem.u32(r + 8);
                if form == 0 {
                    continue;
                }
                let id = e.mem.u32(form + 0xc);
                let edid = plugin
                    .get(esm::FormId(id))
                    .and_then(|x| plugin.editor_id_of(x).ok().flatten())
                    .unwrap_or_default();
                let name = if id == 0x14 && lower.contains("player") && !lower.contains("playerref")
                {
                    "player".to_string()
                } else {
                    edid
                };
                string_set(&mut e, r, name.as_bytes());
                names.push((name.into_bytes(), id));
            }
            PLUGIN_NAMES.with(|p| *p.borrow_mut() = names);
            e.mem.write(buf, &[0; sb::SIZE as usize]);
            let text = heap_text(&mut e, &source);
            e.mem.set_u32(buf + sb::TEXT, text);
            e.mem.set_u32(script + sc::TEXT, text);
            e.mem.set_u32(buf + sb::CURRENT_SCRIPT, script);
            for at in [0u32, 4, 8, 0xc] {
                let v = e.mem.u32(script + sc::UNUSED_VARS + at);
                e.mem.set_u32(buf + sb::UNUSED_VARS + at, v);
            }
            let kind = e.mem.u16(script + 0x28);
            e.mem.set_u16(buf + sb::TYPE, kind);
            let refs = e.mem.bytes(script + sc::REFS, 8);
            e.mem.write(buf + sb::REFS, &refs);
            let vars = e.mem.bytes(script + sc::VARS, 8);
            e.mem.write(buf + sb::VARS, &vars);
            for (number, (text, (op, on_ref, want))) in texts.iter().zip(&lines).enumerate() {
                if !ours(&e, *op) {
                    continue;
                }
                let record = record_of(&e, *op);
                let (first, rest) = text.split_once([' ', '\t']).unwrap_or((text.as_str(), ""));
                if *on_ref || crate::table::opcode_by_name(&e, first.as_bytes()) != Some(*op) {
                    passed_over += 1;
                    continue;
                }
                e.mem.write(line, &[0; ln::STRUCT as usize]);
                e.mem.set_u32(line + ln::NUMBER, number as u32 + 1);
                let rest = rest.as_bytes();
                e.mem.write(line + ln::TEXT, &rest[..rest.len().min(0x1ff)]);
                e.mem.set_u32(line + ln::OPCODE, *op);
                NEEDS_GAME_PARSER.with(|n| n.set(false));
                let parse = e.mem.u32(record + 0x1c);
                let count = u32::from(e.mem.u16(record + 0x12));
                let params = e.mem.u32(record + 0x14);
                let ok = e.call(parse, &[count, params, line, buf]).bool();
                if NEEDS_GAME_PARSER.with(std::cell::Cell::get) {
                    passed_over += 1;
                    continue;
                }
                let len = e.mem.u32(line + ln::DATA_LEN).min(ln::SIZE);
                let got = e.mem.bytes(line + ln::DATA, len);
                compared += 1;
                dump.push_str(&format!(
                    "{script_name}: {text}\n  want {want:02x?}\n  got  {got:02x?}\n"
                ));
                if !ok || got != *want {
                    mismatches.push(format!("{script_name}: {text}"));
                }
            }
        }
    }
    if let Some(path) = std::env::var_os("NV_COMPILE_DUMP") {
        std::fs::write(path, &dump).expect("the dump file");
    }
    eprintln!("compared {compared} lines, passed over {passed_over}");
    assert!(compared > 0, "no line of xNVSE's compiler in {plugins}");
    assert!(mismatches.is_empty(), "{mismatches:#?}");
}

// ---- more rules ----

/// The number widths' edges: 255 is a byte, 256 a short, 65535 a short, 65536 an int.
#[test]
fn number_widths_change_at_their_edges() {
    let mut e = engine();
    assert_eq!(
        compiled(&mut e, EVAL, "255 + 256 + 65535 + 65536"),
        args_of(&[arg(&[
            b"B\xff",
            b"I\x00\x01",
            &[op::ADD],
            b"I\xff\xff",
            &[op::ADD],
            b"L\x00\x00\x01\x00",
            &[op::ADD],
        ])])
    );
}

/// An operator's rules: an asymmetric rule only one way round (`2 := x` is refused), the
/// logical operators give their right operand's type (a string variable takes `x && "a"`).
#[test]
fn operator_rules_choose_the_result_type() {
    let mut e = engine();
    let (ok, _) = compile(&mut e, LET, "2 := x");
    assert!(!ok);
    assert_eq!(
        printed(),
        ["Error line 7\n\nInvalid operands for operator :="]
    );
    assert_eq!(
        compiled(&mut e, LET, "s := x && \"a\""),
        args_of(&[arg(&[
            &v(2, 4),
            &v(0, 1),
            b"S\x01\x00a",
            &[op::LOGICAL_AND],
            &[op::ASSIGNMENT],
        ])])
    );
    assert!(printed().is_empty());
}

/// A command inside an expression takes the rest of the text: what its own arguments leave is
/// not a second argument's error inside (`TypeOf x 5` is `TypeOf x`); the line goes on past it.
#[test]
fn a_nested_command_takes_the_rest_of_the_line() {
    let mut e = engine();
    let short = compiled(&mut e, EVAL, "TypeOf x");
    assert_eq!(compiled(&mut e, EVAL, "TypeOf x 5"), short);
    assert!(printed().is_empty());
}

/// A reference that is not persistent cannot be named in a script: the word is a string, with
/// the warning.
#[test]
fn a_reference_that_is_not_persistent_is_a_string() {
    let mut e = engine();
    e.mem.write(LOOSE_REF, &[0; 0x800]);
    crate::commands::test_support::set_classes(&mut e, LOOSE_REF, 1);
    assert_eq!(
        compiled(&mut e, EVAL, "LooseRef"),
        args_of(&[arg(&[b"S\x08\x00LooseRef"])])
    );
    assert_eq!(printed().len(), 1);
    e.mem.set_u32(LOOSE_REF + 8, 0x400);
    assert_eq!(
        compiled(&mut e, EVAL, "LooseRef"),
        args_of(&[arg(&[b"R\x04\x00"])])
    );
}

/// The dot syntax on a command's result: the right-hand command takes its calling reference
/// from the stack (`x`), the dot is written after it, and the type is the right-hand command's.
#[test]
fn a_dot_after_a_result_calls_on_the_stack() {
    let mut e = engine();
    e.ext::<Nvse>().table.returns[(GET_V - 0x1000) as usize] = Returns::Form;
    let mut first = vec![b'X', 0, 0];
    first.extend_from_slice(&(GET_V as u16).to_le_bytes());
    first.extend_from_slice(&[9, 0, 1, 0, b'n', 1, 0, 0, 0]);
    let mut second = vec![b'x', 0, 0];
    second.extend_from_slice(&(GET_V as u16).to_le_bytes());
    second.extend_from_slice(&[9, 0, 1, 0, b'n', 2, 0, 0, 0]);
    assert_eq!(
        compiled(&mut e, EVAL, "(GetV 1).GetV 2"),
        args_of(&[arg(&[&first, &second, &[op::DOT]])])
    );
    assert!(printed().is_empty());
}

/// While a script compiles the command names come from one map, made as xNVSE's name map is
/// (the first record to bring a name keeps it): the table walk's answers, a short name that
/// another record has as its long name included; the map is kept, and made again only after
/// records are written in.
#[test]
fn a_compile_finds_names_through_one_map() {
    let mut e = installed();
    let short = AREA + 0x7c0;
    e.mem.write(short, b"Let\0");
    let two = record_of(&e, TWO);
    e.mem.set_u32(two + 4, short);
    set_buf_text(&mut e, "int i\r\n");
    assert_eq!(e.site(0x005a_ebb1, &[SCRIPT, BUF]), Some(1));
    for name in ["Let", "TwoArgs", "eval", "nothing"] {
        assert_eq!(
            command_by_name(&mut e, name.as_bytes()),
            crate::table::opcode_by_name(&e, name.as_bytes()),
            "{name}"
        );
    }
    assert_eq!(command_by_name(&mut e, b"let"), Some(TWO));
    assert!(e.ext::<Nvse>().table.names.is_some());
    e.site(0x005a_eda0, &[1, BUF]);
    // The map stays for the next compile, and is made again once records are written in.
    assert!(e.ext::<Nvse>().table.names.is_some());
    e.mem.set_u32(two + 4, 0);
    assert_eq!(e.site(0x005a_ebb1, &[SCRIPT, BUF]), Some(1));
    assert_eq!(command_by_name(&mut e, b"let"), Some(TWO), "the same map");
    e.ext::<Nvse>().table.registrations += 1;
    assert_eq!(command_by_name(&mut e, b"let"), Some(0x1539), "made again");
    e.site(0x005a_eda0, &[1, BUF]);
}

/// A loop open while the script's data grows (xNVSE's size seam moves it to a new block): the
/// `loop` writes its jump word in the new block, at the offset the `while` left room at, and
/// the old block is not written (xNVSE would write the old block; see `begin_loop_parse`).
#[test]
fn a_loop_over_the_datas_growth_writes_the_new_block() {
    let mut e = installed();
    e.register(0x00aa_44c0, |_, _| Ret {
        eax: 0x20,
        ..Ret::default()
    });
    let old = AREA + 0x2_0000;
    e.mem.write(old, &[7; 0x20]);
    e.mem.set_u32(BUF + sb::DATA, old);
    e.mem.set_u32(BUF + sb::DATA_OFFSET, 0x10);
    compiled(&mut e, WHILE, "x < 3");
    // The while line goes into the data (as the game's compiler copies it), then the data grows.
    e.mem.write(old + 0x14, &[0; 4]);
    e.mem.set_u32(LINE + ln::DATA_LEN, 6);
    assert_eq!(e.site(0x005b_0f68, &[BUF, LINE]), Some(1));
    let new = e.mem.u32(BUF + sb::DATA);
    assert_ne!(new, old);
    let before = e.mem.bytes(old, 0x20);
    e.mem.set_u32(BUF + sb::DATA_OFFSET, 0x18);
    compiled(&mut e, LOOP, "");
    assert_eq!(e.mem.u32(new + 0x14), 0x1c);
    assert_eq!(e.mem.bytes(old, 0x20), before, "the old block is untouched");
}

/// The game's own names through xNVSE's name lookup: a script command's long and short names
/// find it; a name a console command brought first (the console table comes in first) finds
/// nothing, so `GetFunction` goes on to the console loop; a record with no short name adds only
/// the empty name. The map, the table walk and the seam agree.
#[test]
fn vanilla_names_through_xnvses_lookup() {
    let mut e = installed();
    let console = 0x0118_e8e0;
    let texts = AREA + 0x7d0;
    e.mem.write(texts, b"GetV\0tgm\0gv\0");
    // Console record 0 shares GetV's long name; console record 1 has no short name.
    e.mem.set_u32(console, texts);
    e.mem.set_u32(console + 0x28, texts + 5);
    e.mem.set_u32(console + 0x28 + 8, 0x105);
    let get_v = record_of(&e, GET_V);
    e.mem.set_u32(get_v + 4, texts + 9);
    let word = AREA + 0x2_8000;
    for (name, want) in [("GetV", None), ("gv", Some(GET_V)), ("tgm", None), ("Let", Some(0x1539))] {
        assert_eq!(crate::table::opcode_by_name(&e, name.as_bytes()), want, "{name}");
        e.ext::<Nvse>().table.registrations += 1;
        assert_eq!(
            crate::table::opcode_by_name_mapped(&mut e, name.as_bytes()),
            want,
            "{name}"
        );
        e.mem.write(word, &[0; 0x214]);
        e.mem.write(word, name.as_bytes());
        set_buf_text(&mut e, "");
        assert_eq!(e.site(0x005a_ebb1, &[SCRIPT, BUF]), Some(1));
        let answer = e.site(0x005b_19ba, &[word]);
        e.site(0x005a_eda0, &[1, BUF]);
        assert_eq!(answer, Some(u32::from(want.is_some())), "{name}");
        assert_eq!(e.mem.u32(word + 0x208), want.unwrap_or(0), "{name}");
    }
}

/// `OptArgs` (0x1004): an expression-parsed command with two optional numbers.
fn optional_args_record(e: &mut Engine) -> u32 {
    let record = record_of(e, 0x1004);
    let at = AREA + 0x7e0;
    let names = AREA + 0x7f8;
    e.mem.write(names, b"OptArgs\0");
    for i in 0..2u32 {
        let p = at + 12 * i;
        e.mem.set_u32(p, names);
        e.mem.set_u32(p + 4, NVSE_NUMBER);
        e.mem.set_u32(p + 8, 1);
    }
    e.mem.set_u32(record, names);
    e.mem.set_u32(record + 8, 0x1004);
    e.mem.set_u16(record + 0x12, 2);
    e.mem.set_u32(record + 0x14, at);
    e.mem.set_u32(record + 0x1c, crate::table::EXPRESSION_PARSE);
    record
}

/// `Peek(-1)` in the argument-end trim. `eval OptArgs == 0`: the nested command's text is
/// " == 0"; its comparison scan puts the end at 0, the trim steps over the blank to -1, where
/// `Peek` reads the cursor's byte (the blank again) and steps to -2. The arguments end, empty,
/// and the cursor is left at -2 + 1 = -1, which the outer line's offset (7) brings back to 6:
/// the outer argument stops at the `s` of `OptArgs`, which is left over, so the line is refused.
#[test]
fn the_argument_end_trim_reads_peeks_default_index() {
    let mut e = engine();
    optional_args_record(&mut e);
    let (ok, _) = compile(&mut e, EVAL, "OptArgs == 0");
    assert!(!ok);
    assert_eq!(
        printed(),
        ["Error line 7\n\nMore args provided than expected by function or command."]
    );
    // With a word before the comparison the trim stops on it, and the line compiles.
    let mut opt = vec![b'X', 0, 0, 4, 0x10];
    let inner = args_of(&[arg(&[b"B\x01"])]);
    opt.extend_from_slice(&(inner.len() as u16 + 2).to_le_bytes());
    opt.extend(inner);
    assert_eq!(
        compiled(&mut e, EVAL, "(OptArgs 1) == 0"),
        args_of(&[arg(&[&opt, b"B\x00", &[op::EQUALS]])])
    );
}
