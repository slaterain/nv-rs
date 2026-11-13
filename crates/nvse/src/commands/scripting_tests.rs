//! Tests of the evaluator (I-6) and the loops (I-7), through the commands' records.

use super::super::test_support::*;
use super::*;

/// Tokens of xNVSE's bytecode.
fn var(var_type: u8, idx: u16) -> Vec<u8> {
    let mut b = vec![b'V', var_type, 0, 0];
    b.extend_from_slice(&idx.to_le_bytes());
    b
}
fn byte(v: u8) -> Vec<u8> {
    vec![b'B', v]
}
fn num(v: f64) -> Vec<u8> {
    let mut b = vec![b'Z'];
    b.extend_from_slice(&v.to_le_bytes());
    b
}
fn text(t: &str) -> Vec<u8> {
    let mut b = vec![b'S'];
    b.extend_from_slice(&(t.len() as u16).to_le_bytes());
    b.extend_from_slice(t.as_bytes());
    b
}
fn op(o: u8) -> Vec<u8> {
    vec![o]
}
/// A command token: `X`, no reference, the opcode, then its argument block (count and
/// arguments) with its length.
fn command(opcode: u16, args: &[Vec<u8>]) -> Vec<u8> {
    let mut block = (args.len() as u16).to_le_bytes().to_vec();
    block.extend(args.concat());
    let mut b = vec![b'X', 0, 0];
    b.extend_from_slice(&opcode.to_le_bytes());
    b.extend_from_slice(&(block.len() as u16 + 2).to_le_bytes());
    b.extend(block);
    b
}
/// One expression: its length word (counting itself) and its tokens.
fn expression(tokens: &[Vec<u8>]) -> Vec<u8> {
    let body = tokens.concat();
    let mut b = (body.len() as u16 + 2).to_le_bytes().to_vec();
    b.extend(body);
    b
}
/// The arguments of `Let`/`eval`: a count byte and one expression.
fn line(tokens: &[Vec<u8>]) -> Vec<u8> {
    let mut b = vec![1u8];
    b.extend(expression(tokens));
    b
}

/// Runs `opcode` with `bytes` as its argument data (the evaluator's layout).
fn run_raw(e: &mut Engine, opcode: u32, bytes: &[u8]) -> (u32, f64) {
    e.mem.set_u16(DATA, opcode as u16);
    e.mem.set_u16(DATA + 2, bytes.len() as u16);
    for (i, b) in bytes.iter().enumerate() {
        e.mem.set_u8(DATA + 4 + i as u32, *b);
    }
    e.mem.set_u32(OFFSET, 4);
    e.mem.set_f64(RESULT, 0.0);
    let record = crate::table::record_of(e, opcode);
    let params = e.mem.u32(record + 0x14);
    let execute = e.mem.u32(record + 0x18);
    let ok = e
        .call(execute, &[params, DATA, 0, 0, SCRIPT, LIST, RESULT, OFFSET])
        .eax;
    (ok & 0xff, e.mem.f64(RESULT))
}

fn upper(_: &mut Engine, a: &[u32]) -> Ret {
    Ret {
        eax: u32::from((a[0] as u8).to_ascii_uppercase()),
        ..Ret::default()
    }
}

fn no_form(_: &mut Engine, _: &[u32]) -> Ret {
    Ret::default()
}

fn setup() -> Engine {
    let mut e = engine_with_table();
    e.register(0x00ec_a7f4, upper);
    e.register(0x0048_39c0, no_form);
    // The script's form id: mod 05.
    e.mem.set_u32(SCRIPT + 0xc, 0x0500_0d00);
    e
}

/// Numbers: the compiler's postfix order, integer variables floored, `^` is `pow`, `%` and the
/// bit operators through 64-bit integers, `+=`.
#[test]
fn let_assigns_numbers_as_xnvse() {
    let mut e = setup();
    let x = add_local(&mut e, 1, "x", 0.0);
    let i = add_local(&mut e, 2, "i", 0.0);
    run_raw(
        &mut e,
        0x1539,
        &line(&[var(0, 1), byte(2), byte(3), byte(4), op(16), op(14), op(0)]),
    );
    assert_eq!(local_value(&e, x), 14.0);
    run_raw(
        &mut e,
        0x1539,
        &line(&[var(1, 2), byte(7), byte(2), op(17), op(0)]),
    );
    assert_eq!(local_value(&e, i), 3.0);
    run_raw(
        &mut e,
        0x1539,
        &line(&[var(0, 1), num(-7.0), byte(3), op(18), op(0)]),
    );
    assert_eq!(local_value(&e, x), -1.0);
    run_raw(
        &mut e,
        0x1539,
        &line(&[var(0, 1), byte(2), byte(10), op(19), op(0)]),
    );
    assert_eq!(local_value(&e, x), 1024.0);
    run_raw(
        &mut e,
        0x1539,
        &line(&[var(0, 1), byte(6), byte(9), op(10), op(0)]),
    );
    assert_eq!(local_value(&e, x), 15.0);
    run_raw(&mut e, 0x1539, &line(&[var(0, 1), byte(5), op(28)]));
    assert_eq!(local_value(&e, x), 20.0);
}

/// Strings: concatenation into a string variable owned by the script's mod, comparison without
/// case, `$` of a number as `%g`.
#[test]
fn strings_concatenate_compare_and_convert() {
    let mut e = setup();
    let s = add_local(&mut e, 1, "s", 0.0);
    run_raw(
        &mut e,
        0x1539,
        &line(&[var(2, 1), text("ab"), text("cd"), op(14), op(0)]),
    );
    let id = local_value(&e, s) as u32;
    assert_eq!(crate::vars::string(&e, id), Some(b"abcd".to_vec()));
    assert_eq!(e.ext::<Nvse>().vars.strings[&id].owning_mod, 5);
    assert_eq!(
        run_raw(&mut e, 0x153a, &line(&[var(2, 1), text("ABCD"), op(4)])).1,
        1.0
    );
    run_raw(&mut e, 0x1539, &line(&[var(2, 1), num(2.5), op(27), op(0)]));
    assert_eq!(crate::vars::string(&e, id), Some(b"2.5".to_vec()));
    // The string variable survives the tick; it is not temporary.
    e.site(0x0086_b386, &[]);
    assert!(crate::vars::string(&e, id).is_some());
}

/// Arrays: `&` boxes a value, `[]` reads and writes elements, the variable keeps a reference.
#[test]
fn arrays_box_subscript_and_assign() {
    let mut e = setup();
    let a = add_local(&mut e, 1, "a", 0.0);
    run_raw(&mut e, 0x1539, &line(&[var(3, 1), byte(5), op(37), op(0)]));
    let id = local_value(&e, a) as u32;
    assert!(crate::vars::array_exists(&e, id));
    assert_eq!(crate::vars::array(&e, id).unwrap().refs, vec![5]);
    run_raw(
        &mut e,
        0x1539,
        &line(&[var(3, 1), byte(0), op(24), byte(7), op(0)]),
    );
    assert_eq!(
        run_raw(
            &mut e,
            0x153a,
            &line(&[var(3, 1), byte(0), op(24), byte(7), op(4)])
        )
        .1,
        1.0
    );
    e.site(0x0086_b386, &[]);
    assert!(crate::vars::array_exists(&e, id));
}

/// `&&` and `||` short-circuit: the division by zero on the right is never evaluated;
/// `testexpr` answers 0 for a failing expression.
#[test]
fn logical_operators_short_circuit() {
    let mut e = setup();
    let (ok, v) = run_raw(
        &mut e,
        0x153a,
        &line(&[byte(0), byte(1), byte(0), op(17), op(2)]),
    );
    assert_eq!((ok, v), (1, 0.0));
    assert_eq!(
        run_raw(
            &mut e,
            0x153a,
            &line(&[byte(1), byte(1), byte(0), op(17), op(1)])
        )
        .1,
        1.0
    );
    assert_eq!(
        run_raw(&mut e, 0x1542, &line(&[byte(1), byte(0), op(17)])).1,
        0.0
    );
    assert_eq!(
        run_raw(&mut e, 0x1542, &line(&[byte(1), byte(2), op(17)])).1,
        1.0
    );
}

/// A command inside an expression runs through its record: `Floor 2.7 + 1` is 3.
#[test]
fn commands_run_inside_expressions() {
    let mut e = setup();
    let x = add_local(&mut e, 1, "x", 0.0);
    run_raw(
        &mut e,
        0x1539,
        &line(&[
            var(0, 1),
            command(0x14cb, &[z(2.7)]),
            byte(1),
            op(14),
            op(0),
        ]),
    );
    assert_eq!(local_value(&e, x), 3.0);
}

/// A parenthesised argument (`0xFFFF` and an expression) of a plain command:
/// `Floor (1 + 1.5)` is 2.
#[test]
fn inline_expressions_are_arguments() {
    let mut e = setup();
    let mut arg = vec![0xff, 0xff];
    arg.extend(expression(&[byte(1), num(1.5), op(14)]));
    assert_eq!(call(&mut e, 0x14cb, 0, &[arg]), (1, 2.0));
}

/// `EvaluateLine`'s stand-in for the loop tests: calls the command whose words are stored at
/// `0x0300_e000` (the execute address at `+0x20`).
fn base_evaluate(e: &mut Engine, _: &[u32]) -> Ret {
    let words: Vec<u32> = (0..8).map(|i| e.mem.u32(0x0300_e000 + 4 * i)).collect();
    let execute = e.mem.u32(0x0300_e020);
    e.call(execute, &words)
}

fn nothing(_: &mut Engine, _: &[u32]) -> Ret {
    Ret::default()
}

const RUNNER: u32 = 0x0300_d000;
const LENGTH: u32 = 0x0300_d100;

/// An engine whose `EvaluateLine` runs one stored command line, with the context's wraps.
fn line_engine() -> Engine {
    let mut e = setup();
    for addr in [0x005e_0d20, 0x005e_1160, 0x005e_13a0] {
        e.register(addr, nothing);
    }
    e.register(crate::context::EVALUATE_LINE.addr(), base_evaluate);
    crate::context::install(&mut e);
    e
}

/// Runs `opcode` with `bytes` as one script line through `EvaluateLine`.
fn run_line(e: &mut Engine, opcode: u32, bytes: &[u8]) {
    e.mem.set_u16(DATA, opcode as u16);
    for (i, b) in bytes.iter().enumerate() {
        e.mem.set_u8(DATA + 4 + i as u32, *b);
    }
    e.mem.set_u32(OFFSET, 4);
    let record = crate::table::record_of(e, opcode);
    let words = [
        e.mem.u32(record + 0x14),
        DATA,
        0,
        0,
        SCRIPT,
        LIST,
        RESULT,
        OFFSET,
    ];
    for (i, w) in words.iter().enumerate() {
        e.mem.set_u32(0x0300_e000 + 4 * i as u32, *w);
    }
    e.mem.set_u32(0x0300_e020, e.mem.u32(record + 0x18));
    e.call(
        crate::context::EVALUATE_LINE.addr(),
        &[RUNNER, SCRIPT, opcode, 0, LENGTH, 4, 0, 1, 0, 0],
    );
}

/// `while` with a false condition breaks at once: the running line's length moves the runner
/// to the loop's end and the if stack is put back.
#[test]
fn a_false_while_jumps_to_its_end() {
    let mut e = line_engine();
    e.mem.set_u32(RUNNER + 0x20, 1);
    e.mem.set_u32(RUNNER + 0x24, 0xaa);
    e.mem.set_u32(RUNNER + 0x28, 0xbb);
    e.mem.set_u32(LENGTH, 100);
    // End offset 0x40, count 1, expression `B 0` (4 bytes with its length word).
    let mut bytes = 0x40u32.to_le_bytes().to_vec();
    bytes.push(1);
    bytes.extend(expression(&[byte(0)]));
    run_line(&mut e, 0x153b, &bytes);
    let start = 4 + 5 + 4;
    assert_eq!(e.mem.u32(LENGTH), 100 + 0x40 - start);
    assert_eq!(e.mem.u32(RUNNER + 0x24), 0xaa);
}

/// A true `while` stays in the loop (no jump); `loop` re-tests the condition and, now false,
/// jumps to the end; `break` outside a loop fails the line.
#[test]
fn while_loop_and_break() {
    let mut e = line_engine();
    let x = add_local(&mut e, 1, "x", 1.0);
    e.mem.set_u32(LENGTH, 100);
    let mut bytes = 0x40u32.to_le_bytes().to_vec();
    bytes.push(1);
    bytes.extend(expression(&[var(0, 1)]));
    run_line(&mut e, 0x153b, &bytes);
    assert_eq!(e.mem.u32(LENGTH), 100, "the condition holds: no jump");
    e.mem.set_f64(x + 8, 0.0);
    e.mem.set_u32(LENGTH, 7);
    run_line(&mut e, 0x153c, &[]);
    // `loop` at offset 4: the condition fails, so the runner jumps to the end (0x40).
    assert_eq!(e.mem.u32(LENGTH), 7 + 0x40 - 4);
    assert!(loops(&mut e).is_empty());
    let record = crate::table::record_of(&e, 0x153f);
    let execute = e.mem.u32(record + 0x18);
    let ok = e
        .call(execute, &[0, DATA, 0, 0, SCRIPT, LIST, RESULT, OFFSET])
        .eax;
    assert_eq!(ok & 0xff, 0);
}

/// `ForEachAlt arr fVal`: the value variable takes each element; when the loop ends it is
/// zeroed (the real game: `alt_sum 18 last 0` for 5, 6, 7, `real-results/udf3.txt`).
#[test]
fn foreach_alt_passes_values_and_zeroes_the_variable_at_the_end() {
    let mut e = line_engine();
    let value = add_local(&mut e, 1, "fVal", 0.0);
    let arr_local = add_local(&mut e, 2, "aArr", 0.0);
    let arr = vars::create_array(&mut e, vars::KEY_NUMBER, true, 5);
    for (i, v) in [5.0, 6.0, 7.0].into_iter().enumerate() {
        vars::set_element(&mut e, arr, &Key::Number(i as f64), vars::Value::Number(v));
    }
    e.mem.set_f64(arr_local + 8, f64::from(arr));
    e.mem.set_u32(LENGTH, 100);
    let mut bytes = 0x40u32.to_le_bytes().to_vec();
    bytes.push(2);
    bytes.extend(expression(&[var(3, 2)]));
    bytes.extend(expression(&[var(0, 1)]));
    run_line(&mut e, 0x1670, &bytes);
    assert_eq!(e.mem.u32(LENGTH), 100, "not empty: no jump");
    let mut seen = vec![e.mem.f64(value + 8)];
    for _ in 0..2 {
        run_line(&mut e, 0x153c, &[]);
        seen.push(e.mem.f64(value + 8));
    }
    assert_eq!(seen, [5.0, 6.0, 7.0]);
    run_line(&mut e, 0x153c, &[]);
    assert!(loops(&mut e).is_empty());
    assert_eq!(e.mem.f64(value + 8), 0.0);
}

/// Review item 7: `string * n` repeats an `int` count (`cvttsd2si`); a count past `int` is
/// `INT_MIN`, nothing; a result past 64 MiB is a script error, nothing allocated.
#[test]
fn string_times_number_counts_in_int() {
    let mut e = setup();
    let s = add_local(&mut e, 1, "s", 0.0);
    let repeat = |e: &mut Engine, n: f64| {
        run_raw(
            e,
            0x1539,
            &line(&[var(2, 1), text("ab"), num(n), op(16), op(0)]),
        );
        let id = local_value(e, s) as u32;
        crate::vars::string(e, id)
    };
    assert_eq!(repeat(&mut e, 3.9), Some(b"ababab".to_vec()));
    assert_eq!(repeat(&mut e, -2.0), Some(Vec::new()));
    assert_eq!(repeat(&mut e, 1e12), Some(Vec::new()));
    run_raw(&mut e, 0x1539, &line(&[var(2, 1), text("x"), op(0)]));
    assert_eq!(
        repeat(&mut e, 2e9),
        Some(b"x".to_vec()),
        "too long: not assigned"
    );
}

/// `Run`'s stand-in: a script that begins a loop and returns inside it; the outer one (`a[2]`
/// 0) also calls an inner one first and notes how many loops are left after it at `RESULT`.
fn run_leaving_a_loop(e: &mut Engine, a: &[u32]) -> Ret {
    add_loop(e, Loop::While { expr_offset: 0 }, a[0], 4, 0x40);
    if a[2] == 0 {
        e.call(0x005e_0d20, &[RUNNER + 0x100, SCRIPT, 1]);
        let left = loops(e).len();
        e.mem.set_u32(RESULT, left as u32);
    }
    Ret::default()
}

/// Review item 4: a loop a called script leaves (`return` inside `ForEach`/`while`) ends with
/// that call, so the caller's `loop` finds its own loop; the caller's goes when it ends.
#[test]
fn a_loop_left_inside_a_call_ends_with_the_call() {
    let mut e = setup();
    for addr in [0x005e_1160, 0x005e_13a0, 0x005e_1550] {
        e.register(addr, nothing);
    }
    e.register(0x005e_0d20, run_leaving_a_loop);
    crate::context::install(&mut e);
    e.call(0x005e_0d20, &[RUNNER, SCRIPT, 0]);
    assert_eq!(
        e.mem.u32(RESULT),
        1,
        "only the caller's loop after the inner call"
    );
    assert!(loops(&mut e).is_empty());
}

/// Review item 6, checked against xNVSE 6.4.9: `UpdateForEachAltIterator` stores an inner
/// array's id in the value variable without a reference (`Loops.cpp`, `kDataType_Array`:
/// `GetScriptLocal()->data = elem->m_data.arrID`), and the loop's end zeroes it without one.
/// The port does the same: the inner array's references are the outer element's alone.
#[test]
fn foreach_alt_takes_no_reference_to_an_inner_array() {
    let mut e = line_engine();
    let value = add_local(&mut e, 1, "aVal", 0.0);
    let outer_local = add_local(&mut e, 2, "aOuter", 0.0);
    let outer = vars::create_array(&mut e, vars::KEY_NUMBER, true, 5);
    let inner = vars::create_array(&mut e, vars::KEY_NUMBER, true, 5);
    vars::set_element(&mut e, outer, &Key::Number(0.0), vars::Value::Array(inner));
    e.mem.set_f64(outer_local + 8, f64::from(outer));
    let refs_before = vars::array(&e, inner).unwrap().refs.clone();
    e.mem.set_u32(LENGTH, 100);
    let mut bytes = 0x40u32.to_le_bytes().to_vec();
    bytes.push(2);
    bytes.extend(expression(&[var(3, 2)]));
    bytes.extend(expression(&[var(3, 1)]));
    run_line(&mut e, 0x1670, &bytes);
    assert_eq!(e.mem.f64(value + 8), f64::from(inner));
    assert_eq!(vars::array(&e, inner).unwrap().refs, refs_before);
    run_line(&mut e, 0x153c, &[]);
    assert_eq!(e.mem.f64(value + 8), 0.0);
    assert_eq!(vars::array(&e, inner).unwrap().refs, refs_before);
}

/// The result `run_line` left.
fn line_result(e: &Engine) -> f64 {
    e.mem.f64(RESULT)
}

/// `Jmp 0x30` (xNVSE's compiler's jump): the running line's length becomes the target less the
/// offset the command began at (4), and the result is 1.
#[test]
fn jmp_moves_the_runner_to_its_target() {
    let mut e = line_engine();
    e.mem.set_u32(LENGTH, 100);
    let mut bytes = vec![1u8];
    bytes.extend(expression(&[byte(0x30)]));
    run_line(&mut e, 0x1681, &bytes);
    assert_eq!(e.mem.u32(LENGTH), 0x30 - 4);
    assert_eq!(line_result(&e), 1.0);
    // A target before the start: the length word wraps as xNVSE's signed subtraction.
    e.mem.set_u32(LENGTH, 100);
    let mut bytes = vec![1u8];
    bytes.extend(expression(&[byte(2)]));
    run_line(&mut e, 0x1681, &bytes);
    assert_eq!(e.mem.u32(LENGTH) as i32, 2 - 4);
}

/// `Jmp_If_True cond 0x30` jumps only when the condition holds and `Jmp_If_False` only when it
/// does not; the result says whether it jumped. A fractional target is truncated.
#[test]
fn conditional_jumps_follow_their_condition() {
    let cases = [
        (0x167f, 1u8, true),
        (0x167f, 0, false),
        (0x1680, 0, true),
        (0x1680, 1, false),
    ];
    for (opcode, cond, jumps) in cases {
        let mut e = line_engine();
        e.mem.set_u32(LENGTH, 100);
        let mut bytes = vec![2u8];
        bytes.extend(expression(&[byte(cond)]));
        bytes.extend(expression(&[num(48.75)]));
        run_line(&mut e, opcode, &bytes);
        let (length, result) = if jumps { (48 - 4, 1.0) } else { (100, 0.0) };
        assert_eq!(e.mem.u32(LENGTH), length, "{opcode:x} {cond}");
        assert_eq!(line_result(&e), result, "{opcode:x} {cond}");
    }
}

// ---- The override argument path (`ExtractArgsOverride`, `override_args.rs`) ----
//
// The bytes below follow xNVSE's compiler override (`CompilerOverride`, `Hooks_Script.cpp`, and
// `ExpressionParser::ParseArgs`, `ScriptUtils.cpp`): no plugin here was compiled with it, so they
// are built from those rules, not taken from a real script. A line inside a `Begin _Block`: the
// argument count word `0xFFFF` (`kOverride_Command`; `0xFFFE` for a block type's parameters),
// the count byte, then one expression per argument (its length word counting itself, then its
// tokens).

use crate::args::{v_extract_args_ex, Arg};
use crate::override_args::{self, OVERRIDE_BLOCK_TYPE, OVERRIDE_COMMAND};

const PARAMS: u32 = 0x0300_c000;
const OUT: u32 = 0x0300_c800;
/// `MessageBoxA` through the exe's import, as `g_ErrOut.Show` calls it.
const MESSAGE_BOX: u32 = 0x00fd_f2f4;

thread_local! {
    static BOXES: std::cell::RefCell<Vec<Vec<u8>>> = const { std::cell::RefCell::new(Vec::new()) };
}

fn message_box(e: &mut Engine, a: &[u32]) -> Ret {
    let mut text = Vec::new();
    let mut at = a[1];
    while e.mem.u8(at) != 0 {
        text.push(e.mem.u8(at));
        at += 1;
    }
    BOXES.with(|b| b.borrow_mut().push(text));
    Ret::default()
}

fn boxes() -> Vec<String> {
    BOXES.with(|b| {
        std::mem::take(&mut *b.borrow_mut())
            .into_iter()
            .map(|t| String::from_utf8_lossy(&t).into_owned())
            .collect()
    })
}

thread_local! {
    static CONSOLE_LINES: std::cell::RefCell<Vec<String>> = const { std::cell::RefCell::new(Vec::new()) };
}

/// The game console's `Print` (`0x0071D0A0`): the format it is handed (xNVSE's run-time
/// `ShowError` hands the message as the format).
fn console_print_line(e: &mut Engine, a: &[u32]) -> Ret {
    let text = String::from_utf8_lossy(&crate::table::guest_text(e, a[1])).into_owned();
    CONSOLE_LINES.with(|c| c.borrow_mut().push(text));
    Ret::default()
}

fn console_lines() -> Vec<String> {
    CONSOLE_LINES.with(|c| std::mem::take(&mut *c.borrow_mut()))
}

/// [`setup`] with a console after the first frame, its lines and any message box recorded.
fn override_engine() -> Engine {
    BOXES.with(|b| b.borrow_mut().clear());
    CONSOLE_LINES.with(|c| c.borrow_mut().clear());
    let mut e = setup();
    e.register(MESSAGE_BOX, message_box);
    e.register(0x0071_b160, |_, _| Ret {
        eax: 0x0300_f800,
        ..Ret::default()
    });
    e.register(0x0071_d0a0, console_print_line);
    e.ext::<crate::Nvse>().main_thread_recorded = true;
    override_args::install(&mut e);
    e
}

/// Parameters of the given types at PARAMS (`ParamInfo`: name, type, optional).
fn param_types(e: &mut Engine, kinds: &[u32]) {
    for (i, k) in kinds.iter().enumerate() {
        e.mem.set_u32(PARAMS + 12 * i as u32 + 4, *k);
        e.mem.set_u32(PARAMS + 12 * i as u32 + 8, 0);
    }
}

/// A compiled line at DATA: `opcode`, its length word, then `mode` and the expressions; the
/// offset word at the count word (4). The expressions' total length, for the offset checks.
fn override_line(e: &mut Engine, opcode: u16, mode: u16, exprs: &[Vec<u8>]) -> u32 {
    let mut body = mode.to_le_bytes().to_vec();
    body.push(exprs.len() as u8);
    let mut lengths = 0;
    for x in exprs {
        lengths += x.len() as u32;
        body.extend_from_slice(x);
    }
    e.mem.set_u16(DATA, opcode);
    e.mem.set_u16(DATA + 2, body.len() as u16);
    e.mem.write(DATA + 4, &body);
    e.mem.set_u32(OFFSET, 4);
    lengths
}

/// `@PushExecutionContext` or `@PopExecutionContext` as the compiler override writes them
/// (`OnCompileSuccess`: the opcode and a length of 0; the `End` statement, `0x11` and length 0,
/// moved after the pop) on `this_obj` in `container`: the handler's answer.
fn context_command(e: &mut Engine, opcode: u32, this_obj: u32, container: u32) -> u32 {
    e.mem.set_u16(DATA, opcode as u16);
    e.mem.set_u16(DATA + 2, 0);
    e.mem
        .set_u32(DATA + 4, if opcode == 0x154e { 0x11 } else { 0 });
    e.mem.set_u32(OFFSET, 4);
    let record = crate::table::record_of(e, opcode);
    let execute = e.mem.u32(record + 0x18);
    e.call(
        execute,
        &[0, DATA, this_obj, container, SCRIPT, LIST, RESULT, OFFSET],
    )
    .eax & 0xff
}

/// The compiler's `@PushExecutionContext` after a `Begin`: the calling reference, its container
/// and the pushing command's opcode kept for this thread; `@PopExecutionContext` before the `End`
/// removes the innermost, and fails the line when there is none. Pushes and pops balance.
#[test]
fn execution_contexts_push_and_pop_in_balance() {
    let mut e = override_engine();
    assert_eq!(context_command(&mut e, 0x154d, 0x0500_0001, 0x0500_0002), 1);
    assert_eq!(context_command(&mut e, 0x154d, 0x0500_0003, 0), 1);
    assert_eq!(override_args::depth(&mut e), 2);
    let top = override_args::current_context(&mut e).unwrap();
    assert_eq!(
        (top.calling_ref, top.container_ref, top.opcode),
        (0x0500_0003, 0, 0x154d)
    );
    assert_eq!(context_command(&mut e, 0x154e, 0, 0), 1);
    let top = override_args::current_context(&mut e).unwrap();
    assert_eq!(
        (top.calling_ref, top.container_ref),
        (0x0500_0001, 0x0500_0002)
    );
    assert_eq!(context_command(&mut e, 0x154e, 0, 0), 1);
    assert_eq!(override_args::depth(&mut e), 0);
    assert_eq!(context_command(&mut e, 0x154e, 0, 0), 0, "nothing to pop");
    assert_eq!(override_args::depth(&mut e), 0);
}

/// A command line in a `_` block (`0xFFFF`): every argument evaluated, then each converted to its
/// own parameter's type (an integer, a float, a string, a double); the offset word passes the
/// count word and each expression, not the count byte (xNVSE's `ReadByte` moves only the read
/// position).
#[test]
fn an_override_line_converts_each_argument_to_its_parameter() {
    let mut e = override_engine();
    context_command(&mut e, 0x154d, 0x0500_0001, 0);
    param_types(&mut e, &[0x01, 0x02, 0x00, 0x2c]);
    let lengths = override_line(
        &mut e,
        0x1500,
        OVERRIDE_COMMAND,
        &[
            expression(&[num(7.9)]),
            expression(&[byte(2), byte(3), op(14)]),
            expression(&[text("abc")]),
            expression(&[num(0.25)]),
        ],
    );
    let read = v_extract_args_ex(&mut e, PARAMS, DATA, OFFSET, SCRIPT, LIST, false);
    assert!(read.ok);
    assert_eq!(
        read.args,
        [
            Arg::Int(7),
            Arg::Float(5.0),
            Arg::Str {
                raw: b"abc".to_vec(),
                text: b"abc".to_vec()
            },
            Arg::Double(0.25),
        ]
    );
    assert_eq!(e.mem.u32(OFFSET), 4 + 2 + lengths);
    assert!(boxes().is_empty());
    // A value that does not fit its parameter (a string for a form; a string for a number is
    // xNVSE's new temporary string id instead) fails the whole read.
    param_types(&mut e, &[0x0e]);
    override_line(
        &mut e,
        0x1500,
        OVERRIDE_COMMAND,
        &[expression(&[text("x")])],
    );
    assert!(!v_extract_args_ex(&mut e, PARAMS, DATA, OFFSET, SCRIPT, LIST, false).ok);
}

/// `0xFFFF` needs the context `@PushExecutionContext` keeps: without one, xNVSE's run-time error
/// (one console line, no message box: that is the editor's) and a failed read. A block type's
/// parameters (`0xFFFE`) read without one.
#[test]
fn a_command_needs_a_context_and_a_block_type_does_not() {
    let mut e = override_engine();
    param_types(&mut e, &[0x01]);
    override_line(&mut e, 0x1500, OVERRIDE_COMMAND, &[expression(&[byte(4)])]);
    let read = v_extract_args_ex(&mut e, PARAMS, DATA, OFFSET, SCRIPT, LIST, false);
    assert!(!read.ok);
    assert_eq!(
        console_lines(),
        ["ERROR: Could not get execution context in ExtractArgsOverride::ExtractArgs"]
    );
    assert!(boxes().is_empty(), "no message box in the game");
    override_line(
        &mut e,
        0x1500,
        OVERRIDE_BLOCK_TYPE,
        &[expression(&[byte(4)])],
    );
    let read = v_extract_args_ex(&mut e, PARAMS, DATA, OFFSET, SCRIPT, LIST, false);
    assert!(read.ok);
    assert_eq!(read.args, [Arg::Int(4)]);
    assert!(boxes().is_empty());
    assert!(console_lines().is_empty());
}

/// The exe's own reader as a stand-in: counts its calls at `OUT + 0x100` and writes 99 to the
/// first output.
fn exe_reader(e: &mut Engine, a: &[u32]) -> Ret {
    e.mem.set_u32(OUT + 0x100, e.mem.u32(OUT + 0x100) + 1);
    e.mem.set_u32(a[7], 99);
    Ret {
        eax: 1,
        ..Ret::default()
    }
}

/// The exe's reader (`005accb0`) as the game calls it for a vanilla opcode: an ordinary count goes
/// on to the game's reader untouched; a negative count (a `_` block line) is read by xNVSE's, in
/// every preset, as xNVSE's patch of the exe's reader does.
#[test]
fn a_vanilla_opcode_takes_the_override_reader_only_with_a_negative_count() {
    let mut e = override_engine();
    e.register(crate::args::PARSE_PARAMETERS.addr(), exe_reader);
    crate::args::install(&mut e);
    param_types(&mut e, &[0x01]);
    let call = |e: &mut Engine| {
        e.call(
            crate::args::PARSE_PARAMETERS.addr(),
            &[PARAMS, DATA, OFFSET, 0, 0, SCRIPT, LIST, OUT],
        )
        .eax & 0xff
    };
    // `0x1010` with an ordinary count word (1): the game's reader.
    e.mem.set_u16(DATA, 0x1010);
    e.mem.set_u16(DATA + 4, 1);
    e.mem.set_u32(OFFSET, 4);
    assert_eq!(call(&mut e), 1);
    assert_eq!(e.mem.u32(OUT), 99);
    assert_eq!(e.mem.u32(OUT + 0x100), 1);
    assert_eq!(
        e.mem.u32(OFFSET),
        4,
        "the game's reader is handed it as it was"
    );
    // The same opcode compiled in a `_` block: xNVSE's reader; the game's is not reached.
    context_command(&mut e, 0x154d, 0x0500_0001, 0);
    override_line(&mut e, 0x1010, OVERRIDE_COMMAND, &[expression(&[byte(6)])]);
    assert_eq!(call(&mut e), 1);
    assert_eq!(e.mem.u32(OUT), 6);
    assert_eq!(e.mem.u32(OUT + 0x100), 1);
}

/// A format string in a `_` block (`sv_Construct`): the format and its arguments are the evaluated
/// arguments (`OverriddenScriptFormatStringArgs`); without a context, xNVSE's console error and no
/// text.
#[test]
fn a_format_string_takes_its_arguments_from_the_evaluated_ones() {
    let mut e = override_engine();
    context_command(&mut e, 0x154d, 0x0500_0001, 0);
    let mut bytes = OVERRIDE_COMMAND.to_le_bytes().to_vec();
    bytes.push(3);
    bytes.extend(expression(&[text("%g items, %g left")]));
    bytes.extend(expression(&[byte(3)]));
    bytes.extend(expression(&[num(1.5)]));
    let (ok, id) = run_raw(&mut e, 0x156b, &bytes);
    assert_eq!(ok, 1);
    assert_eq!(
        vars::string(&e, id as u32).as_deref(),
        Some(&b"3 items, 1.5 left"[..])
    );
    context_command(&mut e, 0x154e, 0, 0);
    let (_, id) = run_raw(&mut e, 0x156b, &bytes);
    assert_eq!(
        console_lines(),
        ["ERROR: Could not get execution context in ExtractFormattedString()"]
    );
    assert!(boxes().is_empty());
    assert_eq!(vars::string(&e, id as u32).as_deref(), Some(&b""[..]));
}

/// The form at `0x0200_7000` for id `0x0123_4567`; no other id finds one.
fn lookup_one_form(_: &mut Engine, a: &[u32]) -> Ret {
    Ret {
        eax: if a[0] == 0x0123_4567 { 0x0200_7000 } else { 0 },
        ..Ret::default()
    }
}

/// A form argument read through xNVSE's command reader (`vExtractArgsEx`, `bConvertTESForms`
/// false): the form is passed as it is, whatever its kind; a quest parameter given an NPC still
/// reads, where a cast would refuse it.
#[test]
fn an_override_form_argument_is_passed_unconverted() {
    let mut e = override_engine();
    e.register(0x0048_39c0, lookup_one_form);
    e.mem.set_u8(0x0200_7000 + 4, 0x2a);
    e.mem.set_u32(0x0200_7000 + 0xc, 0x0123_4567);
    set_script_ref(&mut e, 1, 0x0200_7000);
    context_command(&mut e, 0x154d, 0x0500_0001, 0);
    param_types(&mut e, &[0x0e]);
    override_line(
        &mut e,
        0x1500,
        OVERRIDE_COMMAND,
        &[expression(&[vec![b'R', 1, 0]])],
    );
    let read = v_extract_args_ex(&mut e, PARAMS, DATA, OFFSET, SCRIPT, LIST, false);
    assert!(read.ok);
    assert_eq!(read.args, [Arg::Form(0x0200_7000)]);
    // A reference that names no form: passed on as no form.
    set_script_ref(&mut e, 1, 0);
    override_line(
        &mut e,
        0x1500,
        OVERRIDE_COMMAND,
        &[expression(&[vec![b'R', 1, 0]])],
    );
    let read = v_extract_args_ex(&mut e, PARAMS, DATA, OFFSET, SCRIPT, LIST, false);
    assert!(read.ok);
    assert_eq!(read.args, [Arg::Form(0)]);
}

/// A stand-in command: records the calling reference and container it is handed at `OUT + 0x200`.
fn record_refs(e: &mut Engine, a: &[u32]) -> Ret {
    e.mem.set_u32(OUT + 0x200, a[2]);
    e.mem.set_u32(OUT + 0x204, a[3]);
    e.mem.set_f64(a[6], 0.0);
    Ret {
        eax: 1,
        ..Ret::default()
    }
}

/// A command inside an argument with no reference of its own (`GetAV Health` in
/// `SetAV Health (GetAV Health + 1)`) runs on the context's calling reference, in its container.
#[test]
fn the_contexts_reference_and_container_reach_the_argument_commands() {
    let mut e = override_engine();
    let record = crate::table::record_of(&e, 0x1001);
    e.mem.set_u32(record + 0x18, 0x0070_1000);
    e.register(0x0070_1000, record_refs);
    context_command(&mut e, 0x154d, 0x0500_0001, 0x0500_0002);
    param_types(&mut e, &[0x01]);
    override_line(
        &mut e,
        0x1500,
        OVERRIDE_COMMAND,
        &[expression(&[command(0x1001, &[])])],
    );
    let read = v_extract_args_ex(&mut e, PARAMS, DATA, OFFSET, SCRIPT, LIST, false);
    assert!(read.ok);
    assert_eq!(
        (e.mem.u32(OUT + 0x200), e.mem.u32(OUT + 0x204)),
        (0x0500_0001, 0x0500_0002)
    );
}

/// Numbers to number parameters: a float keeps its fraction (rounded to `float`), an integer is
/// truncated towards zero and wraps as MSVC's `__dtoui3` (2^32 + 5 is 5).
#[test]
fn override_numbers_convert_as_xnvse() {
    let mut e = override_engine();
    context_command(&mut e, 0x154d, 0x0500_0001, 0);
    param_types(&mut e, &[0x02, 0x02, 0x01, 0x01]);
    override_line(
        &mut e,
        0x1500,
        OVERRIDE_COMMAND,
        &[
            expression(&[num(2.75)]),
            expression(&[num(0.1)]),
            expression(&[num(-2.5)]),
            expression(&[num(4_294_967_301.0)]),
        ],
    );
    let read = v_extract_args_ex(&mut e, PARAMS, DATA, OFFSET, SCRIPT, LIST, false);
    assert!(read.ok);
    assert_eq!(
        read.args,
        [
            Arg::Float(2.75),
            Arg::Float(0.1f32),
            Arg::Int(-2),
            Arg::Int(5)
        ]
    );
}

/// Strings to number parameters (`handlePrimitiveStringVar`, for the `sv_*` commands): a string
/// variable gives its id, a literal a new temporary string owned by the script's mod; a string
/// variable to a string parameter gives its text.
#[test]
fn override_strings_to_numbers_give_string_ids() {
    let mut e = override_engine();
    let id = vars::add_string(&mut e, 5, b"hello", false);
    add_local(&mut e, 10, "s", f64::from(id));
    context_command(&mut e, 0x154d, 0x0500_0001, 0);
    param_types(&mut e, &[0x01, 0x01, 0x00]);
    override_line(
        &mut e,
        0x1500,
        OVERRIDE_COMMAND,
        &[
            expression(&[var(2, 10)]),
            expression(&[text("q")]),
            expression(&[var(2, 10)]),
        ],
    );
    let read = v_extract_args_ex(&mut e, PARAMS, DATA, OFFSET, SCRIPT, LIST, false);
    assert!(read.ok);
    assert_eq!(read.args[0], Arg::Int(id as i32));
    let Arg::Int(made) = read.args[1] else {
        panic!("{:?}", read.args[1]);
    };
    assert_ne!(made as u32, id);
    let s = e.ext::<crate::Nvse>().vars.strings[&(made as u32)].clone();
    assert_eq!((s.data.as_slice(), s.owning_mod), (&b"q"[..], 5));
    assert_eq!(
        read.args[2],
        Arg::Str {
            raw: b"hello".to_vec(),
            text: b"hello".to_vec()
        }
    );
}

/// `ActorValueInfo`s for codes 0 and 12 (`Agility`), and the first two animation groups.
fn names_tables(e: &mut Engine) {
    e.map(0x011d_6000, 0x1000);
    e.map(0x0119_7000, 0x3000);
    let mut text_at = 0x0200_0800;
    let mut put = |e: &mut Engine, s: &str| {
        let at = text_at;
        e.mem.write(at, s.as_bytes());
        e.mem.set_u8(at + s.len() as u32, 0);
        text_at += 0x20;
        at
    };
    for (code, name) in [(0u32, "Aggression"), (12, "Agility")] {
        let info = 0x0200_0000 + code * 0x40;
        let t = put(e, name);
        e.mem.set_u32(info + 0x38, t);
        e.mem.set_u32(0x011d_61c8 + 4 * code, info);
    }
    for (i, name) in ["Idle", "DynamicIdle"].iter().enumerate() {
        let t = put(e, name);
        e.mem.set_u32(0x0119_77d8 + 0x24 * i as u32, t);
    }
}

/// Actor value, axis, animation group and sex parameters (`SetAV Agility`, `SetPos z`,
/// `PlayGroup Idle`, `GetIsSex Female` in a `_` block): a name of any case, a number where
/// xNVSE takes one, and anything else fails the read.
#[test]
fn override_names_convert_to_their_codes() {
    let mut e = override_engine();
    names_tables(&mut e);
    context_command(&mut e, 0x154d, 0x0500_0001, 0);
    let read = |e: &mut Engine, kind: u32, x: Vec<u8>| {
        param_types(e, &[kind]);
        override_line(e, 0x1500, OVERRIDE_COMMAND, &[expression(&[x])]);
        let r = v_extract_args_ex(e, PARAMS, DATA, OFFSET, SCRIPT, LIST, false);
        r.ok.then(|| r.args[0].clone())
    };
    // Actor value: a name, a number 0..=76, nothing else.
    assert_eq!(read(&mut e, 0x05, text("agility")), Some(Arg::Int(12)));
    assert_eq!(read(&mut e, 0x05, num(76.9)), Some(Arg::Int(76)));
    assert_eq!(read(&mut e, 0x05, num(77.0)), None);
    assert_eq!(read(&mut e, 0x05, num(-1.0)), None);
    assert_eq!(read(&mut e, 0x05, text("Luck")), None);
    // Axis: one letter of either case, as a byte; a number is no axis.
    assert_eq!(read(&mut e, 0x08, text("z")), Some(Arg::Byte(b'Z')));
    assert_eq!(read(&mut e, 0x08, text("X")), Some(Arg::Byte(b'X')));
    assert_eq!(read(&mut e, 0x08, text("xy")), None);
    assert_eq!(read(&mut e, 0x08, byte(0x58)), None);
    // Animation group: a name, any number but 245.
    assert_eq!(read(&mut e, 0x0a, text("DYNAMICIDLE")), Some(Arg::Int(1)));
    assert_eq!(read(&mut e, 0x0a, byte(7)), Some(Arg::Int(7)));
    assert_eq!(read(&mut e, 0x0a, byte(245)), None);
    assert_eq!(read(&mut e, 0x0a, text("Walk")), None);
    // Sex: "male" or "female" of any case.
    assert_eq!(read(&mut e, 0x12, text("Female")), Some(Arg::Int(1)));
    assert_eq!(read(&mut e, 0x12, text("MALE")), Some(Arg::Int(0)));
    assert_eq!(read(&mut e, 0x12, byte(1)), None);
    // A magic effect never converts; an array parameter takes an array's id, 0 for no array.
    assert_eq!(read(&mut e, 0x1f, text("x")), None);
    assert_eq!(read(&mut e, 0x100, byte(3)), Some(Arg::Int(0)));
}

/// The stack is one for every thread, each entry its thread's: a thread sees and pops only its
/// own entries, and a pop with none of its own fails and leaves the other thread's.
#[test]
fn the_context_stack_is_kept_per_thread() {
    let mut e = override_engine();
    context_command(&mut e, 0x154d, 0x0500_0001, 0);
    engine_core::thread_host::create(&mut e, 0x77, 0, 0, true);
    e.kernel.threads.current = 1;
    assert_eq!(override_args::current_context(&mut e), None);
    assert!(!override_args::pop_context(&mut e));
    context_command(&mut e, 0x154d, 0x0500_0009, 0);
    assert_eq!(
        override_args::current_context(&mut e).map(|c| c.calling_ref),
        Some(0x0500_0009)
    );
    assert!(override_args::pop_context(&mut e));
    assert!(!override_args::pop_context(&mut e));
    e.kernel.threads.current = 0;
    assert_eq!(override_args::depth(&mut e), 1);
    assert_eq!(
        override_args::current_context(&mut e).map(|c| c.calling_ref),
        Some(0x0500_0001)
    );
}

/// The format-string reader itself: the arguments before the format (converted), a form read by
/// the format, the parameters after the format's 20 (at `fmt_pos + 21`); a format token that is
/// no string gives an empty format.
#[test]
fn the_format_string_reader_splits_its_arguments() {
    let mut e = override_engine();
    e.register(0x0048_39c0, lookup_one_form);
    e.mem.set_u8(0x0200_7000 + 4, 0x2a);
    e.mem.set_u32(0x0200_7000 + 0xc, 0x0123_4567);
    set_script_ref(&mut e, 1, 0x0200_7000);
    context_command(&mut e, 0x154d, 0x0500_0001, 0);
    // Parameter 0 an integer, 1 the format, 22 (1 + 21) a float.
    param_types(&mut e, &[0x01, 0x00]);
    e.mem.set_u32(PARAMS + 12 * 22 + 4, 0x02);
    let line = |e: &mut Engine, fmt: Vec<u8>| {
        let mut body = OVERRIDE_COMMAND.to_le_bytes().to_vec();
        body.push(4);
        body.extend(expression(&[byte(9)]));
        body.extend(expression(&[fmt]));
        body.extend(expression(&[vec![b'R', 1, 0]]));
        body.extend(expression(&[num(0.5)]));
        e.mem.set_u16(DATA, 0x1500);
        e.mem.write(DATA + 4, &body);
        e.mem.set_u32(OFFSET, 4);
    };
    line(&mut e, text("form %x"));
    let mut seen = Vec::new();
    let out = override_args::formatted(
        &mut e,
        PARAMS,
        DATA,
        OFFSET,
        SCRIPT,
        LIST,
        1,
        |e, fmt, args| {
            seen.push(fmt.to_vec());
            let form = args.form(e)?;
            Some(format!("{form:x}").into_bytes())
        },
    )
    .unwrap();
    assert_eq!(seen, [b"form %x".to_vec()]);
    assert_eq!(out.leading, [Arg::Int(9)]);
    assert_eq!(out.text, b"2007000");
    assert_eq!(out.trailing, [Arg::Float(0.5)]);
    // A number where the format goes: an empty format, the read goes on.
    line(&mut e, byte(3));
    let out = override_args::formatted(
        &mut e,
        PARAMS,
        DATA,
        OFFSET,
        SCRIPT,
        LIST,
        1,
        |_, fmt, args| {
            args.skip(2);
            Some(fmt.to_vec())
        },
    )
    .unwrap();
    assert_eq!(out.text, b"");
}
