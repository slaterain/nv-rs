//! Tests of `Assert`, `DecompileScript` and `HasScriptCommand` through their records, on the
//! decompiler tests' engine (`crate::decompile::tests`).

use super::super::test_support::*;
use crate::decompile::tests::{
    engine, form, game_expr, line, nvse_expr, sample_lines, set_script, AREA, VTABLE,
};
use engine_core::prelude::*;
use std::cell::RefCell;

thread_local! {
    /// Console lines: the format and, for one with `%s`, the text it is given.
    static CONSOLE: RefCell<Vec<(String, String)>> = const { RefCell::new(Vec::new()) };
    /// Directories made and files written (path, disposition, bytes).
    static DIRS: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
    static FILES: RefCell<Vec<(String, u32, Vec<u8>)>> = const { RefCell::new(Vec::new()) };
}

fn s(e: &Engine, at: u32) -> String {
    String::from_utf8_lossy(&crate::table::guest_text(e, at)).into_owned()
}

fn console_print(e: &mut Engine, a: &[u32]) -> Ret {
    let format = s(e, a[1]);
    let arg = if format.contains("%s") {
        s(e, e.mem.u32(a[2]))
    } else {
        String::new()
    };
    CONSOLE.with(|c| c.borrow_mut().push((format, arg)));
    Ret::default()
}

fn console() -> Vec<(String, String)> {
    CONSOLE.with(|c| std::mem::take(&mut *c.borrow_mut()))
}

fn create_directory(e: &mut Engine, a: &[u32]) -> Ret {
    let path = s(e, a[0]);
    DIRS.with(|d| d.borrow_mut().push(path));
    Ret {
        eax: 1,
        ..Ret::default()
    }
}

fn create_file(e: &mut Engine, a: &[u32]) -> Ret {
    let path = s(e, a[0]);
    FILES.with(|f| f.borrow_mut().push((path, a[4], Vec::new())));
    Ret {
        eax: 0x44,
        ..Ret::default()
    }
}

fn write_file(e: &mut Engine, a: &[u32]) -> Ret {
    let bytes = e.mem.bytes(a[1], a[2]);
    FILES.with(|f| {
        if let Some(last) = f.borrow_mut().last_mut() {
            last.2.extend(bytes);
        }
    });
    Ret {
        eax: 1,
        ..Ret::default()
    }
}

/// Files written: path, disposition, bytes.
type Files = Vec<(String, u32, Vec<u8>)>;

fn written() -> (Vec<String>, Files) {
    (
        DIRS.with(|d| std::mem::take(&mut *d.borrow_mut())),
        FILES.with(|f| std::mem::take(&mut *f.borrow_mut())),
    )
}

/// The decompiler tests' engine with a console after the first frame (closed: not console mode)
/// and the file calls recorded, nothing written to a disk.
fn setup() -> Engine {
    console();
    written();
    let mut e = engine();
    e.register(0x0071_b160, |_, _| Ret {
        eax: 0x0300_f800,
        ..Ret::default()
    });
    e.register(0x0071_d0a0, console_print);
    e.ext::<crate::Nvse>().main_thread_recorded = true;
    e.register(0x00fd_f0b8, create_directory);
    e.register(0x00fd_f088, create_file);
    e.register(0x00fd_f0dc, write_file);
    e.register(0x00fd_f07c, |_, _| Ret::default());
    // A command line runs from its script's data (the evaluator reads tokens there).
    e.mem.set_u32(SCRIPT + 0x30, DATA);
    e
}

/// Runs `opcode` with `args` as its argument bytes, in the script, on `this_obj`: the result.
fn run(e: &mut Engine, opcode: u32, args: &[u8], this_obj: u32) -> f64 {
    e.mem.set_u16(DATA, opcode as u16);
    e.mem.set_u16(DATA + 2, args.len() as u16);
    e.mem.write(DATA + 4, args);
    e.mem.set_u32(OFFSET, 4);
    e.mem.set_f64(RESULT, 7.0);
    let record = crate::table::record_of(e, opcode);
    let words = [
        e.mem.u32(record + 0x14),
        DATA,
        this_obj,
        0,
        SCRIPT,
        LIST,
        RESULT,
        OFFSET,
    ];
    e.call(e.mem.u32(record + 0x18), &words);
    e.mem.f64(RESULT)
}

fn assert_args(tokens: &[&[u8]]) -> Vec<u8> {
    let mut b = vec![1u8];
    b.extend(nvse_expr(tokens));
    b
}

fn v(var_idx: u16) -> Vec<u8> {
    let mut b = vec![b'V', 0, 0, 0];
    b.extend_from_slice(&var_idx.to_le_bytes());
    b
}

/// `Assert`: nothing when the expression holds; when it does not, "Assertion failed!", the line
/// as the evaluator writes it back (the command's name, then its expression) and each variable it
/// read once with its value; a line that does not evaluate prints too.
#[test]
fn assert_prints_the_line_and_its_variables_when_false() {
    use crate::expr::op;
    let mut e = setup();
    let x = crate::args::event_list_variable(&e, LIST, 1);
    let i = crate::args::event_list_variable(&e, LIST, 2);
    e.mem.set_f64(x + 8, 1.0);
    e.mem.set_f64(i + 8, 2.0);
    // x + i > 1: holds.
    let holds = assert_args(&[&v(1), &v(2), &[op::ADD], &[b'B', 1], &[op::GREATER_THAN]]);
    run(&mut e, 0x1646, &holds, 0);
    assert_eq!(console(), Vec::<(String, String)>::new());
    // x + x + i > 5: does not.
    let fails = assert_args(&[
        &v(1),
        &v(1),
        &[op::ADD],
        &v(2),
        &[op::ADD],
        &[b'B', 5],
        &[op::GREATER_THAN],
    ]);
    assert_eq!(
        run(&mut e, 0x1646, &fails, 0),
        7.0,
        "the result is left as it was"
    );
    assert_eq!(
        console(),
        [
            ("Assertion failed!".to_string(), String::new()),
            ("\t%s".to_string(), "Assert x + x + i > 5".to_string()),
            ("\tWhere %s".to_string(), "x=1, i=2".to_string()),
        ]
    );
    // (x + 1) * 2 < 0.5, the stronger operator's operand in parentheses.
    let fails = assert_args(&[
        &v(1),
        &[b'B', 1],
        &[op::ADD],
        &[b'B', 2],
        &[op::MULTIPLY],
        &[b'Z', 0, 0, 0, 0, 0, 0, 0xe0, 0x3f],
        &[op::LESS_THAN],
    ]);
    run(&mut e, 0x1646, &fails, 0);
    let lines = console();
    assert_eq!(lines[1].1, "Assert (x + 1) * 2 < 0.5");
    assert_eq!(lines[2].1, "x=1");
}

/// A command called on a reference variable (`rx.GetV 7`): the line names the variable and the
/// command (in parentheses, its parameter being optional, with an operator after it); the
/// variables part gives the variable's form as its string representation, or that it holds none.
#[test]
fn assert_names_a_reference_variables_form() {
    use crate::expr::op;
    let mut e = setup();
    let rx = crate::args::event_list_variable(&e, LIST, 3);
    let mut call = vec![b'X', 3, 0, 2, 0x10];
    call.extend_from_slice(&9u16.to_le_bytes());
    call.extend_from_slice(&[1, 0, b'n', 7, 0, 0, 0]);
    let fails = assert_args(&[&call, &[b'B', 100], &[op::GREATER_THAN]]);
    run(&mut e, 0x1646, &fails, 0);
    let lines = console();
    assert_eq!(lines[1].1, "Assert (rx.GetV 7) > 100");
    assert_eq!(lines[2].1, "rx=uninitialized form (0)");
    e.mem.set_u32(rx + 8, 0x14);
    run(&mut e, 0x1646, &fails, 0);
    let lines = console();
    assert_eq!(lines[2].1, "rx=[id: 14, edid: \"\", name: \"<no name>\"]");
    e.mem.set_u32(rx + 8, 0x77);
    run(&mut e, 0x1646, &fails, 0);
    assert_eq!(console()[2].1, "rx=invalid form (77)");
}

/// A line that does not evaluate (a variable the list does not have) fails the assertion; its
/// variable has no name and reads as 0.
#[test]
fn assert_prints_a_line_that_does_not_evaluate() {
    use crate::expr::op;
    let mut e = setup();
    let fails = assert_args(&[&v(9), &[b'B', 1], &[op::GREATER_THAN]]);
    run(&mut e, 0x1646, &fails, 0);
    let lines = console();
    assert_eq!(lines[0].0, "Assertion failed!");
    assert_eq!(lines[1].1, "Assert <failed to get var name 0 9> > 1");
    assert_eq!(lines[2].1, "=0");
}

/// `DecompileScript` on a script: `DecompiledScripts`, then `DecompiledScripts/<mod>`, then the
/// file `<name>.<extension>` written whole (new lines as `\r\n`); 1. A script whose lines do not
/// read writes nothing (and still answers 1); a form of another kind answers 0.
#[test]
fn decompile_script_writes_the_script_to_its_file() {
    let mut e = setup();
    set_script(&mut e, &sample_lines());
    set_script_ref(&mut e, 5, SCRIPT);
    let mut args = vec![2, 0, b'r', 5, 0];
    args.extend_from_slice(&[3, 0, b't', b'x', b't']);
    assert_eq!(run(&mut e, 0x162a, &args, 0), 1.0);
    let (dirs, files) = written();
    assert_eq!(dirs, ["DecompiledScripts", "DecompiledScripts/Unknown"]);
    assert_eq!(files.len(), 1);
    assert_eq!(files[0].0, "DecompiledScripts/Unknown/Test.txt");
    assert_eq!(files[0].1, 2, "CREATE_ALWAYS: the file starts over");
    // The sample script's text (worked out from the rules in the decompiler's own test), each new
    // line written `\r\n`.
    let want = "ScriptName Test\r\n\r\nFloat x\r\nInt i\r\nRef rx\r\n\r\nBegin GameMode\r\n\
        \tSet x To 1 + 2 * 3\r\n\teval x + 1\r\n\r\n\tIf x > 1\r\n\
        \t\tV1Cmd 5 2.5 \"a%qb\" PlayerRef X Female\r\n\tElseIf x < 2\r\n\tElse\r\n\
        \t\t<ABC123>.GetV 3\r\n\tEndIf\r\n\r\n\tSet x To (GetV 7) + 1\r\n\tReturn\r\nEnd\r\n";
    assert_eq!(String::from_utf8(files[0].2.clone()).unwrap(), want);
    // No extension given: "gek".
    assert_eq!(run(&mut e, 0x162a, &[1, 0, b'r', 5, 0], 0), 1.0);
    assert_eq!(written().1[0].0, "DecompiledScripts/Unknown/Test.gek");
    // Lines that do not read: nothing written, still 1.
    set_script(&mut e, &[line(0x0f, &[])]);
    assert_eq!(run(&mut e, 0x162a, &[1, 0, b'r', 5, 0], 0), 1.0);
    assert_eq!(written(), (vec![], vec![]));
    // Another kind of form: 0.
    e.mem.set_u8(SCRIPT + 4, 0x2a);
    assert_eq!(run(&mut e, 0x162a, &[1, 0, b'r', 5, 0], 0), 0.0);
    assert_eq!(written(), (vec![], vec![]));
}

/// A script embedded in another form (a quest stage's result script, a terminal entry's) with its
/// vtable, data and the main script's locals.
fn embedded_script(e: &mut Engine, at: u32, lines: &[Vec<u8>]) {
    let data = lines.concat();
    let d = AREA + 0xa000 + (at & 0xfff);
    e.mem.write(d, &data);
    e.mem.write(d + data.len() as u32, &[0; 16]);
    e.mem.set_u32(at, VTABLE);
    e.mem.set_u32(at + 0x30, d);
    e.mem.set_u32(at + 0x20, data.len() as u32);
    for off in [0x44, 0x48, 0x4c, 0x50] {
        e.mem.set_u32(at + off, e.mem.u32(SCRIPT + off));
    }
}

/// A quest writes each stage's first log entry's result script that has data, named
/// `<quest> #<stage>`; a package its three actions' scripts, named with the action.
#[test]
fn decompile_script_writes_quest_stages_and_package_actions() {
    let mut e = setup();
    let body = [
        line(0x10, &[0, 0, 0, 0, 0, 0]),
        line(0x15, &[&[b'f', 1, 0][..], &game_expr(&[b"5"])].concat()),
        line(0x11, &[]),
    ];
    // Quest "Q": stage 10 with an entry whose script has data, stage 20 whose script has none.
    let quest = AREA + 0xc000;
    form(&mut e, quest, 0x0500_0001, Some("Q"));
    e.mem.set_u8(quest + 4, 0x47);
    let (stage_a, stage_b) = (AREA + 0xc100, AREA + 0xc200);
    let (entry_a, entry_b) = (AREA + 0xc300, AREA + 0xc400);
    e.mem.set_u32(quest + 0x44, stage_a);
    e.mem.set_u32(quest + 0x48, AREA + 0xc0f0);
    e.mem.set_u32(AREA + 0xc0f0, stage_b);
    e.mem.set_u8(stage_a, 10);
    e.mem.set_u32(stage_a + 4, entry_a);
    e.mem.set_u8(stage_b, 20);
    e.mem.set_u32(stage_b + 4, entry_b);
    embedded_script(&mut e, entry_a + 0xc, &body);
    // Stage 20's script has data but a length of 0: passed over.
    embedded_script(&mut e, entry_b + 0xc, &body);
    e.mem.set_u32(entry_b + 0xc + 0x20, 0);
    set_script_ref(&mut e, 5, quest);
    assert_eq!(run(&mut e, 0x162a, &[1, 0, b'r', 5, 0], 0), 1.0);
    let (_, files) = written();
    assert_eq!(files.len(), 1);
    assert_eq!(files[0].0, "DecompiledScripts/Unknown/Q #10.gek");
    assert_eq!(
        String::from_utf8(files[0].2.clone()).unwrap(),
        "Float x\r\nInt i\r\nRef rx\r\n\r\nBegin GameMode\r\n\tSet x To 5\r\nEnd\r\n"
    );
    // Package "P": OnBegin and OnChange scripts, no OnEnd.
    let package = AREA + 0xd000;
    form(&mut e, package, 0x0500_0002, Some("P"));
    e.mem.set_u8(package + 4, 0x49);
    let (on_begin, on_change) = (AREA + 0xd100, AREA + 0xd200);
    embedded_script(&mut e, on_begin, &body);
    embedded_script(&mut e, on_change, &body);
    e.mem.set_u32(package + 0x50, on_begin);
    e.mem.set_u32(package + 0x70, on_change);
    set_script_ref(&mut e, 5, package);
    assert_eq!(run(&mut e, 0x162a, &[1, 0, b'r', 5, 0], 0), 1.0);
    let names: Vec<String> = written().1.into_iter().map(|f| f.0).collect();
    assert_eq!(
        names,
        [
            "DecompiledScripts/Unknown/P OnBegin.gek",
            "DecompiledScripts/Unknown/P OnChange.gek"
        ]
    );
}

/// In the console, a line names the file written, or says the script is not compiled.
#[test]
fn decompile_script_tells_the_console() {
    let mut e = setup();
    e.mem.set_u8(0x011d_ea2e, 1);
    let tls = e.tls();
    e.mem.set_u8(tls + 0x268, 1);
    set_script(&mut e, &sample_lines());
    set_script_ref(&mut e, 5, SCRIPT);
    run(&mut e, 0x162a, &[1, 0, b'r', 5, 0], 0);
    assert_eq!(
        console(),
        [(
            "Decompiled script to '%s'".to_string(),
            "DecompiledScripts/Unknown/Test.gek".to_string()
        )]
    );
    set_script(&mut e, &[line(0x0f, &[])]);
    run(&mut e, 0x162a, &[1, 0, b'r', 5, 0], 0);
    assert_eq!(
        console(),
        [("Script %s is not compiled".to_string(), "Test".to_string())]
    );
}

/// `HasScriptCommand opcode form block`: 1 when the script calls the command, within the block
/// type's blocks when one is given; 0 for an opcode no record has or without a script.
#[test]
fn has_script_command_answers_through_the_analyzer() {
    let mut e = setup();
    set_script(&mut e, &sample_lines());
    set_script_ref(&mut e, 5, SCRIPT);
    let args = |opcode: i32, block: Option<i32>| {
        let mut b = vec![if block.is_some() { 3 } else { 2 }, 0, b'n'];
        b.extend_from_slice(&opcode.to_le_bytes());
        b.extend_from_slice(&[b'r', 5, 0]);
        if let Some(k) = block {
            b.push(b'n');
            b.extend_from_slice(&k.to_le_bytes());
        }
        b
    };
    assert_eq!(run(&mut e, 0x162b, &args(0x1001, None), 0), 1.0);
    assert_eq!(run(&mut e, 0x162b, &args(0x1002, None), 0), 1.0);
    assert_eq!(run(&mut e, 0x162b, &args(0x1003, None), 0), 0.0);
    assert_eq!(run(&mut e, 0x162b, &args(0x1001, Some(0)), 0), 1.0);
    assert_eq!(run(&mut e, 0x162b, &args(0x1001, Some(1)), 0), 0.0);
    // A block type past the table: no filter.
    assert_eq!(run(&mut e, 0x162b, &args(0x1001, Some(38)), 0), 1.0);
    assert_eq!(run(&mut e, 0x162b, &args(0x9999, None), 0), 0.0);
    // No form and no calling reference.
    let mut only = vec![1, 0, b'n'];
    only.extend_from_slice(&0x1001i32.to_le_bytes());
    assert_eq!(run(&mut e, 0x162b, &only, 0), 0.0);
}

thread_local! {
    static RESULT_SCRIPTS: RefCell<(u32, u32)> = const { RefCell::new((0, 0)) };
}

/// `TESTopicInfo::GetResultScript(begin/end)`: the two scripts the test sets.
fn result_script(_: &mut Engine, a: &[u32]) -> Ret {
    let (begin, end) = RESULT_SCRIPTS.with(|r| *r.borrow());
    Ret {
        eax: if a[1] == 0 { begin } else { end },
        ..Ret::default()
    }
}

/// A dialogue response writes its Begin and End result scripts (`<name> Begin`, `<name> End`); a
/// terminal its menu entries' scripts that have data, numbered among the entries that exist
/// (`<name> #2`); a form with no editor id is named by its id's low six digits (`%08X`).
#[test]
fn decompile_script_names_topic_infos_terminals_and_unnamed_forms() {
    let mut e = setup();
    e.register(0x0061_eb60, result_script);
    let body = [line(0x10, &[0, 0, 0, 0, 0, 0]), line(0x11, &[])];
    let info = AREA + 0xc000;
    form(&mut e, info, 0x0500_0010, Some("Hi"));
    e.mem.set_u8(info + 4, 0x46);
    let (begin, end) = (AREA + 0xc100, AREA + 0xc200);
    embedded_script(&mut e, begin, &body);
    embedded_script(&mut e, end, &body);
    RESULT_SCRIPTS.with(|r| *r.borrow_mut() = (begin, end));
    set_script_ref(&mut e, 5, info);
    assert_eq!(run(&mut e, 0x162a, &[1, 0, b'r', 5, 0], 0), 1.0);
    let names: Vec<String> = written().1.into_iter().map(|f| f.0).collect();
    assert_eq!(
        names,
        [
            "DecompiledScripts/Unknown/Hi Begin.gek",
            "DecompiledScripts/Unknown/Hi End.gek"
        ]
    );
    // A terminal: no entry first, then an entry with no data, then one with data.
    let terminal = AREA + 0xd000;
    form(&mut e, terminal, 0x0500_0011, Some("T"));
    e.mem.set_u8(terminal + 4, 0x17);
    let (first, second) = (AREA + 0xd100, AREA + 0xd200);
    e.mem.set_u32(terminal + 0x98, 0);
    e.mem.set_u32(terminal + 0x9c, AREA + 0xd0f0);
    e.mem.set_u32(AREA + 0xd0f0, first);
    e.mem.set_u32(AREA + 0xd0f4, AREA + 0xd0f8);
    e.mem.set_u32(AREA + 0xd0f8, second);
    e.mem.set_u32(AREA + 0xd0fc, 0);
    embedded_script(&mut e, first + 0x10, &body);
    e.mem.set_u32(first + 0x10 + 0x20, 0);
    embedded_script(&mut e, second + 0x10, &body);
    set_script_ref(&mut e, 5, terminal);
    assert_eq!(run(&mut e, 0x162a, &[1, 0, b'r', 5, 0], 0), 1.0);
    let names: Vec<String> = written().1.into_iter().map(|f| f.0).collect();
    assert_eq!(names, ["DecompiledScripts/Unknown/T #2.gek"]);
    // A script with no editor id.
    set_script(&mut e, &body);
    form(&mut e, SCRIPT, 0x0500_0d00, None);
    e.mem.set_u8(SCRIPT + 4, 0x11);
    set_script_ref(&mut e, 5, SCRIPT);
    assert_eq!(run(&mut e, 0x162a, &[1, 0, b'r', 5, 0], 0), 1.0);
    assert_eq!(written().1[0].0, "DecompiledScripts/Unknown/00000D00.gek");
}

/// `Assert`'s variables of every kind: a string variable in quotes; an array's keys (up to 10, as
/// numbers or quoted strings) or its size, of each kind; a global and a form literal in the line
/// (neither is a variable); a variable a short circuit passed over prints as never resolved.
#[test]
fn assert_prints_every_kind_of_variable() {
    use crate::expr::op;
    use crate::vars::{self, Key, Value};
    let mut e = setup();
    let string_local = super::super::test_support::add_local(&mut e, 4, "sv", 0.0);
    let info = crate::decompile::tests::local_info(&mut e, 4, "sv");
    let _ = info;
    let id = vars::add_string(&mut e, 5, b"hello", false);
    e.mem.set_f64(string_local + 8, f64::from(id));
    // A string variable compared with a string.
    let fails = assert_args(&[&[b'V', 2, 0, 0, 4, 0], &[b'S', 1, 0, b'x'], &[op::EQUALS]]);
    run(&mut e, 0x1646, &fails, 0);
    let lines = console();
    assert_eq!(lines[1].1, "Assert sv == \"x\"");
    assert_eq!(lines[2].1, "sv=\"hello\"");
    // Arrays: an array of 0, 3 and 11 elements, a number map of 3, a string map of 3.
    let array_local = super::super::test_support::add_local(&mut e, 5, "av", 0.0);
    crate::decompile::tests::local_info(&mut e, 5, "av");
    let make = |e: &mut Engine, key: u8, packed: bool, keys: &[Key]| {
        let id = vars::create_array(e, key, packed, 5);
        for k in keys {
            vars::set_element(e, id, k, Value::Number(1.0));
        }
        id
    };
    let n = |k: f64| Key::Number(k);
    let s = |k: &str| Key::Str(k.as_bytes().to_vec());
    let empty = make(&mut e, vars::KEY_NUMBER, true, &[]);
    let three = make(&mut e, vars::KEY_NUMBER, true, &[n(0.0), n(1.0), n(2.0)]);
    let eleven: Vec<Key> = (0..11).map(|k| n(f64::from(k))).collect();
    let eleven = make(&mut e, vars::KEY_NUMBER, true, &eleven);
    let num_map = make(&mut e, vars::KEY_NUMBER, false, &[n(-1.5), n(2.0), n(10.0)]);
    let str_map = make(&mut e, vars::KEY_STRING, false, &[s("a"), s("b"), s("c")]);
    let big_map: Vec<Key> = (0..11).map(|k| s(&format!("k{k:02}"))).collect();
    let big_map = make(&mut e, vars::KEY_STRING, false, &big_map);
    let fails = assert_args(&[&[b'V', 3, 0, 0, 5, 0], &[b'B', 1], &[op::EQUALS]]);
    for (array, want) in [
        (0, "av=uninitialized array"),
        (empty, "av=array keys []"),
        (three, "av=array keys [0, 1, 2]"),
        (eleven, "av=array size 11"),
        (num_map, "av=array keys [-1.5, 2, 10]"),
        (str_map, "av=array keys [\"a\", \"b\", \"c\"]"),
        (big_map, "av=array size 11"),
    ] {
        e.mem.set_f64(array_local + 8, f64::from(array));
        run(&mut e, 0x1646, &fails, 0);
        let lines = console();
        assert_eq!(lines[1].1, "Assert av == 1", "{want}");
        assert_eq!(lines[2].1, want);
    }
    // A global and a form literal: the line names them, no variables follow.
    let global = AREA + 0xe000;
    form(&mut e, global, 0x0500_0020, None);
    let gname = crate::decompile::tests::text_at(&mut e, "TimeScale");
    e.mem.set_u32(global + 0x18, gname);
    e.mem.set_u16(global + 0x1c, 9);
    e.mem.set_f32(global + 0x24, 30.0);
    super::super::test_support::set_classes(&mut e, global, 1 << 49);
    set_script_ref(&mut e, 6, global);
    let fails = assert_args(&[&[b'G', 6, 0], &[b'B', 100], &[op::GREATER_THAN]]);
    run(&mut e, 0x1646, &fails, 0);
    let lines = console();
    assert_eq!(lines.len(), 2, "{lines:?}");
    assert_eq!(lines[1].1, "Assert TimeScale > 100");
    let fails = assert_args(&[&[b'R', 1, 0], &[b'R', 2, 0], &[op::EQUALS]]);
    run(&mut e, 0x1646, &fails, 0);
    assert_eq!(console()[1].1, "Assert PlayerRef == <ABC123>");
    // `0 && x`: the short circuit passes over x, which prints as never resolved.
    let x = crate::args::event_list_variable(&e, LIST, 1);
    e.mem.set_f64(x + 8, 9.0);
    let fails = assert_args(&[&[b'B', 0], &v(1), &[op::LOGICAL_AND]]);
    run(&mut e, 0x1646, &fails, 0);
    let lines = console();
    assert_eq!(lines[1].1, "Assert 0 && x");
    assert_eq!(lines[2].1, "x=0");
    // An ordinary evaluation (not Assert's) records nothing: no list is kept.
    e.mem.write(DATA + 4, &assert_args(&[&v(1)]));
    e.mem.set_u32(OFFSET, 4);
    let a = crate::CommandArgs::new(&[0, DATA, 0, 0, SCRIPT, LIST, RESULT, OFFSET]);
    let mut ev = crate::expr::Eval::new(&e, &a);
    let toks = crate::expr::with_evaluator(&mut e, |e| ev.extract_args(e));
    assert!(toks.is_some());
    assert!(ev.resolved_vars.is_none());
}

/// `Assert` with a lambda: the line writes the lambda back through the analyzer.
#[test]
fn assert_writes_a_lambda_back() {
    let mut e = setup();
    crate::decompile::tests::lambda_stand_ins(&mut e);
    e.register(0x0046_9800, |_, _| Ret {
        eax: 0x0500_0400,
        ..Ret::default()
    });
    e.register(0x7eff_e200, |_, _| Ret::default());
    e.mem.set_u32(VTABLE + 0x128, 0x7eff_e200);
    let body = [
        line(0x10, &[0, 0, 0, 0, 0, 0]),
        line(0x15, &[&[b'f', 1, 0][..], &game_expr(&[b"5"])].concat()),
        line(0x11, &[]),
    ]
    .concat();
    let mut token = vec![b'F'];
    token.extend_from_slice(&(body.len() as u32).to_le_bytes());
    token.extend_from_slice(&body);
    let fails = assert_args(&[&token]);
    run(&mut e, 0x1646, &fails, 0);
    let lines = console();
    assert_eq!(lines[0].0, "Assertion failed!");
    assert_eq!(lines[1].1, "Assert Begin GameMode\n\tSet x To 5\nEnd");
}
