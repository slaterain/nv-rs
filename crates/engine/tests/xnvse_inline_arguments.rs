//! A command's argument written as an xNVSE expression, compiled through the game's
//! `StandardCompile` (`005b1ba0`, translated) with xNVSE's seam at its word read of each parameter
//! (`0x5B1C65`, `nvse::line_reader`), and read back through the game's argument reader
//! `005accb0` as the command's handler calls it (with xNVSE's wrap over it, `nvse::args`):
//! - a plugin's command of the game's format (opcode `0x1400` up, JIP's for one): in every preset
//!   a parenthesised number and a parenthesised string compile to xNVSE's `0xFFFF` form and read
//!   back as their values, through xNVSE's reader;
//! - one of the game's own commands, with the fix row `xnvse.extract_args`
//!   (`nvse::args::set_game_inline`): the same, at a required or an optional parameter. The exe's
//!   reader reads the line until the expression, then xNVSE's reader reads it again from its start
//!   (its outputs stored again, the offset past the arguments); a line without one is the exe's
//!   read, the same with the row and without;
//! - one of the game's own commands without the row (`vanilla`): the seam leaves the word to the
//!   game, as without xNVSE's patch. A parenthesised argument is refused at a required parameter
//!   and dropped at an optional one, and a line carrying `0xFFFF` (the new script language's, or a
//!   mod's compiled with xNVSE's GECK) is the exe's read: a string's length -1, a copy of 4 GB,
//!   the game's own crash;
//! - inside `set` and `if` (`ReplaceVariablesAndFunctions`, `005b1570`, which compiles the command
//!   on a line of its own without its command word; seam `005B17DD` gives it).
//!
//! Here because this crate registers every translation. The stand-ins are the C runtime calls
//! that read its locale tables (character classes, case-blind compares, `atof`, `strtol`), the
//! number tests, the global lookup (no data handler here), the form cast, the error report's
//! message (recorded by its format) and printing, and the copy (`00401460`), which records a
//! copy of more than 64 KB instead of making it.

use engine as _;
use engine_core::extend::ScriptTables;
use engine_core::prelude::*;
use std::cell::RefCell;

const AREA: u32 = 0x04a0_0000;
/// The compile data, the line, the parameters and their names, the compiled command read back,
/// its offset word, the outputs (a text buffer, an integer, a float), the script and its locals,
/// the expression text and output of `005b1570`, the line number it reads.
const BUF: u32 = AREA;
const LINE: u32 = AREA + 0x100;
const PARAMS: u32 = AREA + 0x800;
const NAMES: u32 = AREA + 0x900;
const DATA: u32 = AREA + 0x1000;
const OFFSET: u32 = AREA + 0x1f00;
const OUT_TEXT: u32 = AREA + 0x2000;
const OUT_INT: u32 = AREA + 0x2400;
const OUT_FLOAT: u32 = AREA + 0x2408;
const SCRIPT: u32 = AREA + 0x3000;
const LOCALS: u32 = AREA + 0x3400;
const EXPR_TEXT: u32 = AREA + 0x3800;
const EXPR_OUT: u32 = AREA + 0x3a00;
const EXPR_LINE: u32 = AREA + 0x3ffc;
/// The exe's data: the compiler's tables (the parameter types at `0118cdd4`, 8 bytes a type: +0
/// a variable may stand there, +1 it names an object; the keywords; the script commands from
/// `01190910`, here reaching past the game's own to a plugin opcode's record) and the globals the
/// compile reads (the data handler).
const EXE_DATA: u32 = 0x0118_0000;
const PARAMETER_TYPES: u32 = 0x0118_cdd4;
const SCRIPT_COMMANDS: u32 = 0x0119_0910;
/// `ScriptError`'s message format (`00407000`, recorded) and its printing.
const VSPRINTF: u32 = 0x0040_7000;
const STRING_PRINTF: u32 = 0x0040_6f60;
const PREPARE_MESSAGE: u32 = 0x0046_4f30;
const CONSOLE_PRINT: u32 = 0x0070_3c00;
const SCRIPT_LOG: u32 = 0x005b_5e40;
/// "Missing parameter %s." (the exe's format).
const TEXT_MISSING_PARAMETER: u32 = 0x0103_9080;
/// The C runtime's locale readers, the number tests, `atof`, `strtol`, the global lookup, the
/// form cast, the copy.
const ISALNUM: u32 = 0x00ec_89e9;
const ISALPHA: u32 = 0x00ec_86e1;
const TOUPPER: u32 = 0x00ec_a7f4;
const TOLOWER: u32 = 0x00ec_67aa;
const STRICMP: u32 = 0x0040_4dc0;
const STRNICMP: u32 = 0x00ec_7ec0;
const IS_A_FLOAT: u32 = 0x004b_12d0;
const IS_AN_INTEGER: u32 = 0x004b_1230;
const ATOF: u32 = 0x00ec_a573;
const TEXT_TO_INTEGER: u32 = 0x00ec_a6bd;
const GET_GLOBAL: u32 = 0x0046_1700;
const DYNAMIC_CAST: u32 = 0x00ec_43fb;
const MEMCPY: u32 = 0x0040_1460;
/// A plugin's opcode (JIP's block) and two of the game's (the second the new language's
/// stand-in's, as `nvse`'s code generation test compiles it).
const PLUGIN_COMMAND: u32 = 0x2400;
const GAME_COMMAND: u32 = 0x1004;
const NEW_SYNTAX_COMMAND: u32 = 0x1104;
/// The parameter types: a string, an integer, a float.
const STRING: u32 = 0;
const INTEGER: u32 = 1;
const FLOAT: u32 = 2;

/// `PlayFileAt("abc", 5);` as the new script language compiles it, after its opcode and length:
/// the count, then each argument after `0xFFFF` with its length word and expression (pinned by
/// `nvse`'s test `a_game_commands_string_and_number_arguments`).
#[rustfmt::skip]
const NEW_SYNTAX_CALL: [u8; 18] = [
    0x02, 0x00,
    0xff, 0xff, 0x08, 0x00, b'S', 0x03, 0x00, b'a', b'b', b'c',
    0xff, 0xff, 0x04, 0x00, b'B', 0x05,
];

thread_local! {
    static FORMATS: RefCell<Vec<u32>> = const { RefCell::new(Vec::new()) };
    static BIG_COPIES: RefCell<Vec<u32>> = const { RefCell::new(Vec::new()) };
}

fn ret(v: u32) -> Ret {
    Ret {
        eax: v,
        ..Ret::default()
    }
}

fn nothing(_: &mut Engine, _: &[u32]) -> Ret {
    Ret::default()
}

fn text(e: &Engine, at: u32) -> Vec<u8> {
    let mut v = Vec::new();
    let mut a = at;
    while a != 0 && e.mem.u8(a) != 0 {
        v.push(e.mem.u8(a));
        a += 1;
    }
    v
}

fn lower(e: &Engine, at: u32) -> Vec<u8> {
    text(e, at).to_ascii_lowercase()
}

fn number(e: &Engine, at: u32) -> String {
    String::from_utf8_lossy(&text(e, at)).into_owned()
}

fn formats() -> Vec<u32> {
    FORMATS.with(|f| f.borrow().clone())
}

fn engine() -> Engine {
    let mut e = Engine::new_untyped_for_test();
    if !e.mem.is_mapped(AREA) {
        e.map(AREA, 0x4000);
    }
    // Installed before the exe's data is mapped: no keyword aliases written (none needed here).
    nvse::line_reader::install(&mut e);
    nvse::args::install(&mut e);
    if !e.mem.is_mapped(EXE_DATA) {
        e.map(EXE_DATA, 0x6_0000);
    }
    // The script command table reaching the plugin opcode (its records past the game's own).
    e.set_script_tables(ScriptTables {
        start: SCRIPT_COMMANDS,
        end: SCRIPT_COMMANDS + (PLUGIN_COMMAND + 1 - 0x1000) * 0x28,
        max_opcode: PLUGIN_COMMAND + 1,
    });
    // The exe's bytes for the three types: a string is no variable and no object; an integer and
    // a float may be a variable.
    e.mem.write(PARAMETER_TYPES + STRING * 8, &[0, 0]);
    e.mem.write(PARAMETER_TYPES + INTEGER * 8, &[1, 0]);
    e.mem.write(PARAMETER_TYPES + FLOAT * 8, &[1, 0]);
    e.register(VSPRINTF, |e, a| {
        FORMATS.with(|f| f.borrow_mut().push(a[1]));
        e.mem.set_u8(a[0], 0);
        ret(0)
    });
    for quiet in [STRING_PRINTF, CONSOLE_PRINT, SCRIPT_LOG] {
        e.register(quiet, nothing);
    }
    e.register(PREPARE_MESSAGE, |_, a| ret(a[0]));
    e.register(ISALNUM, |_, a| {
        ret(u32::from(
            (a[0] as u8).is_ascii_alphanumeric() && a[0] < 0x80,
        ))
    });
    e.register(ISALPHA, |_, a| {
        ret(u32::from((a[0] as u8).is_ascii_alphabetic() && a[0] < 0x80))
    });
    e.register(TOUPPER, |_, a| {
        ret(u32::from((a[0] as u8).to_ascii_uppercase()))
    });
    e.register(TOLOWER, |_, a| {
        ret(u32::from((a[0] as u8).to_ascii_lowercase()))
    });
    e.register(STRICMP, |e, a| {
        if a[0] == 0 || a[1] == 0 {
            return ret(1);
        }
        ret(u32::from(lower(e, a[0]) != lower(e, a[1])))
    });
    e.register(STRNICMP, |e, a| {
        let n = a[2] as usize;
        let (x, y) = (lower(e, a[0]), lower(e, a[1]));
        let cut = |v: &[u8]| v.iter().take(n).copied().collect::<Vec<u8>>();
        ret(u32::from(cut(&x) != cut(&y)))
    });
    e.register(IS_A_FLOAT, |e, a| {
        ret(u32::from(number(e, a[0]).parse::<f64>().is_ok()))
    });
    e.register(IS_AN_INTEGER, |e, a| {
        ret(u32::from(number(e, a[0]).parse::<i32>().is_ok()))
    });
    e.register(ATOF, |e, a| Ret {
        st0: number(e, a[0]).parse().unwrap_or(0.0),
        ..Ret::default()
    });
    e.register(TEXT_TO_INTEGER, |e, a| {
        ret(number(e, a[0]).parse::<i32>().unwrap_or(0) as u32)
    });
    e.register(GET_GLOBAL, |_, _| ret(0));
    e.register(DYNAMIC_CAST, |_, _| ret(0));
    e.register(MEMCPY, |e, a| {
        if a[2] > 0x1_0000 {
            BIG_COPIES.with(|c| c.borrow_mut().push(a[2]));
            return ret(a[0]);
        }
        let bytes = e.mem.bytes(a[1], a[2]);
        e.mem.write(a[0], &bytes);
        ret(a[0])
    });
    e
}

/// The parameters of the given types at `PARAMS`, optional from index `optional_from` on.
fn parameters(e: &mut Engine, kinds: &[u32], optional_from: usize) {
    e.mem.write(NAMES, b"argument\0");
    for (i, kind) in kinds.iter().enumerate() {
        let p = PARAMS + 12 * i as u32;
        e.mem.set_u32(p, NAMES);
        e.mem.set_u32(p + 4, *kind);
        e.mem.set_u32(p + 8, u32::from(i >= optional_from));
    }
}

/// The arguments `args` of a line of `opcode` compiled for `count` parameters: the answer, the
/// output, the formats of the errors reported.
fn compile(e: &mut Engine, opcode: u32, count: u32, args: &str) -> (bool, Vec<u8>, Vec<u32>) {
    FORMATS.with(|f| f.borrow_mut().clear());
    e.mem.write(BUF, &[0; 0x58]);
    e.mem.write(LINE, &[0; 0x41c]);
    e.mem.set_u32(LINE, 1);
    e.mem.write(LINE + 4, args.as_bytes());
    e.mem.set_u32(LINE + 0x204, args.len() as u32);
    e.mem.set_u32(LINE + 0x410, opcode);
    let ok = e.call(0x005b_1ba0, &[count, PARAMS, LINE, BUF]).bool();
    let len = e.mem.u32(LINE + 0x40c).min(0x200);
    (ok, e.mem.bytes(LINE + 0x20c, len), formats())
}

/// The compiled command `output` of `opcode` put where a handler reads it (its opcode and length
/// before it, the offset word at its count) for a compiled script.
fn put_command(e: &mut Engine, opcode: u32, output: &[u8]) {
    e.mem.set_u16(DATA, opcode as u16);
    e.mem.set_u16(DATA + 2, output.len() as u16);
    e.mem.write(DATA + 4, output);
    e.mem.set_u32(OFFSET, 4);
    // The script's form flags: compiled (bit 3; the exe's reader stores nothing without it).
    e.mem.set_u32(SCRIPT + 8, 8);
}

/// What a read back of three gave: the answer, the text, the integer, the float, the offset
/// word after.
#[derive(Debug, PartialEq)]
struct Read {
    ok: bool,
    text: Vec<u8>,
    int: i32,
    float: f32,
    offset: u32,
}

/// `output` read back through `005accb0` for `opcode` as a handler calls it (the outputs a text
/// buffer, an integer and a float).
fn read_three(e: &mut Engine, opcode: u32, output: &[u8]) -> Read {
    put_command(e, opcode, output);
    e.mem.write(OUT_TEXT, &[0x55; 0x10]);
    e.mem.set_u8(OUT_TEXT + 0x10, 0);
    e.mem.set_i32(OUT_INT, -1);
    e.mem.set_f32(OUT_FLOAT, -1.0);
    let words = [
        PARAMS, DATA, OFFSET, 0, 0, SCRIPT, LOCALS, OUT_TEXT, OUT_INT, OUT_FLOAT,
    ];
    let ok = e.call(0x005a_ccb0, &words).bool();
    Read {
        ok,
        text: text(e, OUT_TEXT),
        int: e.mem.i32(OUT_INT),
        float: e.mem.f32(OUT_FLOAT),
        offset: e.mem.u32(OFFSET),
    }
}

/// `output` read back for two parameters (the outputs a text buffer and a float): the answer,
/// the text, the float.
fn read_two(e: &mut Engine, opcode: u32, output: &[u8]) -> (bool, Vec<u8>, f32) {
    put_command(e, opcode, output);
    e.mem.write(OUT_TEXT, &[0; 0x20]);
    e.mem.set_f32(OUT_FLOAT, -1.0);
    let words = [
        PARAMS, DATA, OFFSET, 0, 0, SCRIPT, LOCALS, OUT_TEXT, OUT_FLOAT,
    ];
    let ok = e.call(0x005a_ccb0, &words).bool();
    (ok, text(e, OUT_TEXT), e.mem.f32(OUT_FLOAT))
}

/// A plugin's command: `"key" (1 + 2)` and `("k" + "ey") (4 * 2)` compile (each parenthesised
/// argument `0xFFFF` and its expression) and read back as their values, with the row and
/// without.
#[test]
fn a_plugin_commands_parenthesised_arguments_compile_and_read_back() {
    for row in [false, true] {
        let mut e = engine();
        nvse::args::set_game_inline(&mut e, row);
        parameters(&mut e, &[STRING, FLOAT], 2);
        let (ok, output, errors) = compile(&mut e, PLUGIN_COMMAND, 2, "\"key\" (1 + 2)");
        assert!(ok && errors.is_empty(), "{output:x?} {errors:x?}");
        assert_eq!(
            &output[..7],
            b"\x02\x00\x03\x00key",
            "the string as the game writes it"
        );
        assert_eq!(&output[7..9], &[0xff, 0xff], "xNVSE's expression");
        assert_eq!(
            read_two(&mut e, PLUGIN_COMMAND, &output),
            (true, b"key".to_vec(), 3.0)
        );
        let (ok, output, errors) = compile(&mut e, PLUGIN_COMMAND, 2, "(\"k\" + \"ey\") (4 * 2)");
        assert!(ok && errors.is_empty(), "{output:x?} {errors:x?}");
        assert_eq!(&output[2..4], &[0xff, 0xff]);
        assert_eq!(
            read_two(&mut e, PLUGIN_COMMAND, &output),
            (true, b"key".to_vec(), 8.0)
        );
    }
}

/// One of the game's commands without the row: the seam leaves the word to the game. A
/// parenthesised number and a parenthesised string at a required parameter are each refused
/// ("missing parameter", the line does not compile), and nothing is written as `0xFFFF`; at an
/// optional parameter the argument is dropped (the line compiles with one argument fewer). The
/// line without parentheses compiles and the exe's reader gives the values back.
#[test]
fn without_the_row_a_game_commands_parenthesised_arguments_are_the_games_words() {
    let mut e = engine();
    for (kind, args) in [(FLOAT, "(1 + 2)"), (STRING, "(\"abc\")")] {
        parameters(&mut e, &[kind], 1);
        let (ok, output, errors) = compile(&mut e, GAME_COMMAND, 1, args);
        assert!(!ok, "{args}: {output:x?}");
        assert_eq!(errors, vec![TEXT_MISSING_PARAMETER], "{args}");
        assert!(
            !output.windows(2).any(|w| w == [0xff, 0xff]),
            "{args}: {output:x?}"
        );
    }
    parameters(&mut e, &[STRING, FLOAT], 1);
    let (ok, output, errors) = compile(&mut e, GAME_COMMAND, 2, "\"abc\" (1 + 2)");
    assert!(ok && errors.is_empty(), "{output:x?} {errors:x?}");
    assert_eq!(
        output, b"\x01\x00\x03\x00abc",
        "the optional argument dropped"
    );
    parameters(&mut e, &[STRING, FLOAT], 2);
    let (ok, output, errors) = compile(&mut e, GAME_COMMAND, 2, "\"abc\" 5");
    assert!(ok && errors.is_empty(), "{output:x?} {errors:x?}");
    assert_eq!(
        output,
        [&b"\x02\x00\x03\x00abcz"[..], &5.0f64.to_le_bytes()].concat()
    );
    assert_eq!(
        read_two(&mut e, GAME_COMMAND, &output),
        (true, b"abc".to_vec(), 5.0)
    );
}

/// With the row, one of the game's commands as xNVSE: `"%e" 7 (2 * 3)` compiles to a string, an
/// integer and the expression. Read back, the exe's reader takes the string and the integer
/// (storing `%e` and 7), meets the expression and stops; xNVSE's reader reads the line again from
/// its start, storing every output again (its string is `%e` read as an empty text, so the exe's
/// `%e` is overwritten), and the offset ends past the arguments. The same line without the
/// parentheses is the exe's read, `%e` kept, the same with the row and without. At an optional
/// parameter the parenthesised argument compiles too.
#[test]
fn with_the_row_a_game_commands_expression_is_read_by_xnvses_reader() {
    let mut e = engine();
    nvse::args::set_game_inline(&mut e, true);
    parameters(&mut e, &[STRING, INTEGER, FLOAT], 3);
    let (ok, output, errors) = compile(&mut e, GAME_COMMAND, 3, "\"%e\" 7 (2 * 3)");
    assert!(ok && errors.is_empty(), "{output:x?} {errors:x?}");
    assert_eq!(&output[..11], b"\x03\x00\x02\x00%en\x07\x00\x00\x00");
    assert_eq!(&output[11..13], &[0xff, 0xff]);
    let end = 4 + output.len() as u32;
    assert_eq!(
        read_three(&mut e, GAME_COMMAND, &output),
        Read {
            ok: true,
            text: vec![],
            int: 7,
            float: 6.0,
            offset: end
        }
    );
    let (ok, plain, _) = compile(&mut e, GAME_COMMAND, 3, "\"%e\" 7 8");
    assert!(ok);
    let exes = Read {
        ok: true,
        text: b"%e".to_vec(),
        int: 7,
        float: 8.0,
        offset: 4 + plain.len() as u32,
    };
    assert_eq!(read_three(&mut e, GAME_COMMAND, &plain), exes);
    nvse::args::set_game_inline(&mut e, false);
    assert_eq!(
        read_three(&mut e, GAME_COMMAND, &plain),
        exes,
        "without the row"
    );
    nvse::args::set_game_inline(&mut e, true);
    parameters(&mut e, &[STRING, FLOAT], 1);
    let (ok, output, errors) = compile(&mut e, GAME_COMMAND, 2, "\"abc\" (1 + 2)");
    assert!(ok && errors.is_empty(), "{output:x?} {errors:x?}");
    assert_eq!(
        &output[..9],
        b"\x02\x00\x03\x00abc\xff\xff",
        "the optional one"
    );
    assert_eq!(
        read_two(&mut e, GAME_COMMAND, &output),
        (true, b"abc".to_vec(), 3.0)
    );
}

/// A call of a game command with a string and a number in the new script language (its bytes as
/// `nvse`'s code generation writes them): with the row read by xNVSE's reader, "abc" and 5.
/// Without it the port's own compiler refuses such a call (`nvse`'s test
/// `without_the_row_a_game_commands_expression_argument_is_refused`), so the bytes come only from
/// a mod compiled with xNVSE's GECK: the exe's reader takes the first `0xFFFF` as a string's
/// length, -1, and copies 4 GB, as the game without xNVSE's run-time half would (its crash; the
/// copy is recorded here, not made).
#[test]
fn a_new_syntax_call_of_a_game_command_is_read_with_the_row() {
    let mut e = engine();
    parameters(&mut e, &[STRING, FLOAT], 2);
    nvse::args::set_game_inline(&mut e, true);
    assert_eq!(
        read_two(&mut e, NEW_SYNTAX_COMMAND, &NEW_SYNTAX_CALL),
        (true, b"abc".to_vec(), 5.0)
    );
    assert_eq!(e.mem.u32(OFFSET), 4 + NEW_SYNTAX_CALL.len() as u32);
    assert_eq!(BIG_COPIES.with(|c| c.borrow().clone()), vec![]);
    nvse::args::set_game_inline(&mut e, false);
    read_two(&mut e, NEW_SYNTAX_COMMAND, &NEW_SYNTAX_CALL);
    assert_eq!(
        BIG_COPIES.with(|c| c.borrow().clone()),
        vec![0xffff_ffff],
        "the original's copy of 4 GB"
    );
}

/// A command for `ReplaceVariablesAndFunctions`: its name in the game's script table (slot
/// `slot`, its opcode word `opcode`), and its record where the table gives that opcode's (one
/// float parameter, compiled by `StandardCompile`, run by [`handler`]).
fn table_command(e: &mut Engine, slot: u32, name: &[u8], opcode: u32) {
    let name_at = NAMES + 0x20 + 0x10 * slot;
    e.mem.write(name_at, name);
    e.mem.set_u8(name_at + name.len() as u32, 0);
    let entry = SCRIPT_COMMANDS + 0x28 * slot;
    e.mem.set_u32(entry, name_at);
    e.mem.set_u32(entry + 8, opcode);
    let record = e.script_tables().record(opcode);
    e.mem.set_u32(record, name_at);
    e.mem.set_u32(record + 8, opcode);
    e.mem.set_u16(record + 0x12, 1);
    e.mem.set_u32(record + 0x14, PARAMS);
    e.mem.set_u32(record + 0x18, HANDLER);
    e.mem.set_u32(record + 0x1c, 0x005b_1ba0);
}

/// `expression` as `set` and `if` compile it (`ReplaceVariablesAndFunctions`): the answer
/// (the output's length, 0 on an error) and the output.
fn replace(e: &mut Engine, expression: &str) -> (u32, Vec<u8>) {
    FORMATS.with(|f| f.borrow_mut().clear());
    e.mem.write(BUF, &[0; 0x58]);
    e.mem.write(EXPR_TEXT, expression.as_bytes());
    e.mem.set_u8(EXPR_TEXT + expression.len() as u32, 0);
    e.mem.write(EXPR_OUT, &[0; 0x200]);
    e.mem.set_u32(EXPR_LINE, 1);
    let n = e
        .call(0x005b_1570, &[0, BUF, EXPR_LINE, EXPR_TEXT, EXPR_OUT])
        .u32();
    (n, e.mem.bytes(EXPR_OUT, n.min(0x200)))
}

/// Inside `set` and `if`: `set f to PlugCmd (1 + 2)` compiles in both presets (seam `005B17DD`
/// gives the line its command's opcode) and its arguments read back as 3; `if GameCmd (2 * 3)`
/// compiles with the row and reads back as 6, and without it is refused with nothing written as
/// `0xFFFF`.
#[test]
fn inside_set_and_if_a_commands_expression_compiles_by_its_own_opcode() {
    for row in [false, true] {
        let mut e = engine();
        nvse::args::set_game_inline(&mut e, row);
        parameters(&mut e, &[FLOAT], 1);
        table_command(&mut e, 4, b"GameCmd", GAME_COMMAND);
        table_command(&mut e, 5, b"PlugCmd", PLUGIN_COMMAND);
        for (expression, opcode, value) in [
            ("PlugCmd (1 + 2)", PLUGIN_COMMAND, 3.0),
            ("GameCmd (2 * 3)", GAME_COMMAND, 6.0),
        ] {
            let (n, output) = replace(&mut e, expression);
            if opcode == GAME_COMMAND && !row {
                assert_eq!(n, 0, "{expression}: refused without the row");
                assert_eq!(formats(), vec![TEXT_MISSING_PARAMETER]);
                assert!(!output.windows(2).any(|w| w == [0xff, 0xff]));
                continue;
            }
            assert_ne!(n, 0, "{expression} (row {row}): {:x?}", formats());
            assert_eq!(output[0], b'X', "{expression}");
            assert_eq!(
                u32::from(u16::from_le_bytes([output[1], output[2]])),
                opcode
            );
            let size = usize::from(u16::from_le_bytes([output[3], output[4]]));
            let arguments = output[5..5 + size].to_vec();
            assert_eq!(&arguments[..4], &[1, 0, 0xff, 0xff], "{expression}");
            put_command(&mut e, opcode, &arguments);
            e.mem.set_f32(OUT_FLOAT, -1.0);
            let words = [PARAMS, DATA, OFFSET, 0, 0, SCRIPT, LOCALS, OUT_FLOAT];
            assert!(e.call(0x005a_ccb0, &words).bool(), "{expression}");
            assert_eq!(e.mem.f32(OUT_FLOAT), value, "{expression} (row {row})");
        }
    }
}

/// Where `set` and `if` lines compile and run: the compile data, a line and the `endif` after
/// it, the script's data (the compile's output), the variable `f` and its name, an expression.
const AREA2: u32 = 0x04a1_0000;
const C_DATA: u32 = AREA2;
const C_LINE: u32 = AREA2 + 0x100;
const C_ENDIF: u32 = AREA2 + 0x600;
const C_OUTPUT: u32 = AREA2 + 0xc00;
const C_VAR: u32 = AREA2 + 0x1000;
const C_VAR_NAME: u32 = AREA2 + 0x1040;
const C_EXPR: u32 = AREA2 + 0x1100;
/// The test commands' handler (a stand-in's address).
const HANDLER: u32 = 0x0103_f100;
/// The expression operators' table (`0118cad0`, 8 bytes each: the number, the precedence, the
/// text), as the exe holds it.
const OPERATOR_TABLE: u32 = 0x0118_cad0;
const OPERATORS: [(u8, &[u8]); 16] = [
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

thread_local! {
    static CALLS: RefCell<Vec<(u32, u32)>> = const { RefCell::new(Vec::new()) };
}

/// A command's handler as `005ac7a0` calls it (words: the parameters, the data, the reference,
/// the container, the script, the locals, the result, the offset's address): where it was given
/// its arguments (the data and the offset) recorded, its one float read through `005accb0` as a
/// handler reads it, and given as the result.
fn handler(e: &mut Engine, a: &[u32]) -> Ret {
    let (params, data, reference, container) = (a[0], a[1], a[2], a[3]);
    let (script, locals, result, offset) = (a[4], a[5], a[6], a[7]);
    let at = e.mem.u32(offset);
    CALLS.with(|c| c.borrow_mut().push((data, at)));
    let value = e.with_stack(8, |e, out| {
        let words = [
            params,
            data,
            offset,
            reference,
            container,
            script,
            locals,
            out.addr(),
        ];
        if e.call(0x005a_ccb0, &words).bool() {
            e.mem.f32(out.addr())
        } else {
            f32::NAN
        }
    });
    e.mem.set_f64(result, f64::from(value));
    ret(1)
}

/// The engine for whole lines: the operators, the two commands (one float parameter), the
/// variable `f` (a float, id 1), a compiled script whose data is the compile's output.
fn line_engine(row: bool) -> Engine {
    let mut e = engine();
    if !e.mem.is_mapped(AREA2) {
        e.map(AREA2, 0x2000);
    }
    nvse::args::set_game_inline(&mut e, row);
    for (number, (precedence, text)) in OPERATORS.iter().enumerate() {
        let entry = OPERATOR_TABLE + number as u32 * 8;
        e.mem.set_u32(entry, number as u32);
        e.mem.set_u8(entry + 4, *precedence);
        e.mem.write(entry + 5, text);
    }
    parameters(&mut e, &[FLOAT], 1);
    table_command(&mut e, 4, b"GameCmd", GAME_COMMAND);
    table_command(&mut e, 5, b"PlugCmd", PLUGIN_COMMAND);
    e.register(HANDLER, handler);
    e.mem.write(C_VAR_NAME, b"f\0");
    e.mem.set_u32(C_VAR, 1);
    e.mem.set_u32(C_VAR + 0x18, C_VAR_NAME);
    e.mem.set_u32(SCRIPT + 8, 8);
    e.mem.set_u32(SCRIPT + 0x30, C_OUTPUT);
    e.mem.set_u32(C_DATA + 0x20, C_OUTPUT);
    e
}

/// One line (`opcode`, `text` after its command word, number 1, an `endif` numbered 2 after
/// it) compiled by `CompileLine` (`005aff40`) onto the script's data: where it starts there.
fn compile_line(e: &mut Engine, opcode: u32, text: &str) -> Option<u32> {
    FORMATS.with(|f| f.borrow_mut().clear());
    e.mem.set_u32(C_DATA + 0x3c, C_VAR);
    e.mem.set_u32(C_DATA + 0x40, 0);
    e.mem.set_u32(C_DATA + 0x50, C_ENDIF);
    e.mem.set_u32(C_DATA + 0x54, 0);
    e.mem.write(C_ENDIF, &[0; 0x41c]);
    e.mem.set_u32(C_ENDIF, 2);
    e.mem.set_u32(C_ENDIF + 0x410, 0x19);
    e.mem.write(C_LINE, &[0; 0x41c]);
    e.mem.set_u32(C_LINE, 1);
    e.mem.write(C_LINE + 4, text.as_bytes());
    e.mem.set_u32(C_LINE + 0x204, text.len() as u32);
    e.mem.set_u32(C_LINE + 0x410, opcode);
    let start = e.mem.u32(C_DATA + 0x24);
    let ok = e.call(0x005a_ff40, &[0, SCRIPT, C_DATA, C_LINE]).bool();
    ok.then_some(C_OUTPUT + start)
}

/// A compiled line's expression (`len` bytes at `at`) evaluated by `CExpression::Eval`
/// (`00593db0`) as the script runner does.
fn evaluate(e: &mut Engine, at: u32) -> f64 {
    let len = u32::from(e.mem.u16(at - 2));
    e.mem.write(C_EXPR, &[0; 0x720]);
    e.call(0x0059_3490, &[C_EXPR]);
    e.call(0x0059_3db0, &[C_EXPR, at, 0, 0, SCRIPT, LOCALS, len, 0])
        .f64()
}

/// Whole `set` and `if` lines, compiled by `CompileLine` and run by `CExpression::Eval`: two
/// `set f to PlugCmd (...)` lines and `if PlugCmd (2 * 3) == 6` give 3, 100 and true in both
/// presets; `set f to GameCmd (1 + 2)` compiles and gives 3 with the row and is refused without
/// it. Each command reads its arguments where they are, on the script's data at its own line's
/// offset (seam `005949D4`: `Eval` would hand it a copy of the token on its stack, the same
/// address for every line, and lambdas and error offsets are known by that address), and no
/// copy of more than 64 KB is made.
#[test]
fn set_and_if_lines_run_their_commands_on_the_scripts_data() {
    for row in [false, true] {
        let mut e = line_engine(row);
        let first = compile_line(&mut e, 0x15, " f to PlugCmd (1 + 2)");
        let second = compile_line(&mut e, 0x15, " f to PlugCmd (10 * 10)");
        let condition = compile_line(&mut e, 0x16, " PlugCmd (2 * 3) == 6");
        let game = compile_line(&mut e, 0x15, " f to GameCmd (1 + 2)");
        let (Some(first), Some(second), Some(condition)) = (first, second, condition) else {
            panic!("row {row}: {:x?}", formats());
        };
        assert_eq!(game.is_some(), row, "the game's command: {:x?}", formats());
        CALLS.with(|c| c.borrow_mut().clear());
        // A set line: the opcode, the size, the variable (`f` and its id), the expression's
        // length and bytes; an if line: the opcode, the size, the line count, the length.
        assert_eq!(evaluate(&mut e, first + 4 + 3 + 2), 3.0, "row {row}");
        assert_eq!(evaluate(&mut e, second + 4 + 3 + 2), 100.0, "row {row}");
        assert_eq!(evaluate(&mut e, condition + 4 + 2 + 2), 1.0, "row {row}");
        let mut opcodes = vec![PLUGIN_COMMAND; 3];
        if let Some(game) = game {
            assert_eq!(evaluate(&mut e, game + 4 + 3 + 2), 3.0);
            opcodes.push(GAME_COMMAND);
        }
        let calls = CALLS.with(|c| c.borrow().clone());
        assert_eq!(calls.len(), opcodes.len(), "row {row}: {calls:x?}");
        let mut offsets = Vec::new();
        for ((data, offset), opcode) in calls.iter().zip(&opcodes) {
            assert_eq!(*data, C_OUTPUT, "row {row}: the script's data");
            assert_eq!(
                u32::from(e.mem.u16(data + offset - 4)),
                *opcode,
                "row {row}: the line's own command"
            );
            offsets.push(*offset);
        }
        offsets.dedup();
        assert_eq!(offsets.len(), opcodes.len(), "each line at its own offset");
        assert_eq!(BIG_COPIES.with(|c| c.borrow().clone()), vec![]);
    }
}

/// `kParamType_AnyForm`: any form, a lambda's script included.
const ANY_FORM: u32 = 0x3d;
/// What a lambda's compile and its script at run time need, stood in: the heap's block for the
/// script buffer's data, `ScriptBuffer`'s constructor and destructor, `Script`'s constructor and
/// destructor (a test vtable whose `SetFormID` at `+0x128` stores the id), the console manager
/// (its first word the compiler `Script::Compile` runs on), the game's compile of the lambda's
/// text (`005aeb90`), `DisableAssignFormIDs`, `CopyVarList`, `CopyRefList` and the form lookup
/// (no form has an id here).
const BUFFER_CONSTRUCTOR: u32 = 0x005a_e490;
const BUFFER_DESTRUCTOR: u32 = 0x005a_e5c0;
const SCRIPT_CONSTRUCTOR: u32 = 0x005a_a0f0;
const SCRIPT_DESTRUCTOR: u32 = 0x005a_a1a0;
const CONSOLE_MANAGER: u32 = 0x0071_b160;
const SCRIPT_COMPILE: u32 = 0x005a_eb90;
const DISABLE_ASSIGN_FORM_IDS: u32 = 0x0046_4d30;
const COPY_VAR_LIST: u32 = 0x005a_b930;
const COPY_REF_LIST: u32 = 0x005a_b7f0;
const LOOKUP_FORM: u32 = 0x0048_39c0;
const FORM_HEAP_ALLOCATE: u32 = 0x0040_1000;
/// The test's own: the scripts' vtable and its `SetFormID`, the console manager and its compiler
/// word, the lambdas' compiled data (16 bytes each), the handler of the command taking a form.
const LAMBDA_VTABLE: u32 = AREA2 + 0x1a00;
const SET_FORM_ID: u32 = 0x0103_f200;
const MANAGER: u32 = AREA2 + 0x1c00;
const LAMBDA_DATA: u32 = AREA2 + 0x1d00;
const FORM_HANDLER: u32 = 0x0103_f300;

thread_local! {
    static LAMBDAS: RefCell<u32> = const { RefCell::new(0) };
}

/// The game's compile of a lambda's text, stood in: `begin function {} SetFunctionValue N end`
/// compiles to a script whose data is `scn` and the number `N` (the data `005aeb90` would write
/// is not needed here, only that each lambda's is its own).
fn compile_lambda(e: &mut Engine, a: &[u32]) -> Ret {
    let (script, buf) = (a[1], a[2]);
    let source = String::from_utf8_lossy(&text(e, e.mem.u32(buf))).into_owned();
    let value: u8 = source
        .split_once("SetFunctionValue ")
        .and_then(|(_, rest)| rest.split_whitespace().next())
        .and_then(|n| n.parse().ok())
        .unwrap_or(0);
    let n = LAMBDAS.with(|l| {
        let n = *l.borrow();
        *l.borrow_mut() = n + 1;
        n
    });
    let data = LAMBDA_DATA + 0x10 * n;
    e.mem.write(data, &[0x1d, 0, 0, 0, value, 0xab]);
    e.mem.set_u32(script + 0x30, data);
    e.mem.set_u32(script + 0x20, 6);
    ret(1)
}

/// The command taking one form as `005ac7a0` calls it: the form read through `005accb0` (the
/// lambda's script, from xNVSE's reader), the number in its data given as the result.
fn form_handler(e: &mut Engine, a: &[u32]) -> Ret {
    let (params, data, reference, container) = (a[0], a[1], a[2], a[3]);
    let (script, locals, result, offset) = (a[4], a[5], a[6], a[7]);
    let value = e.with_stack(8, |e, out| {
        e.mem.set_u32(out.addr(), 0);
        let words = [
            params,
            data,
            offset,
            reference,
            container,
            script,
            locals,
            out.addr(),
        ];
        if !e.call(0x005a_ccb0, &words).bool() {
            return f64::NAN;
        }
        let form = e.mem.u32(out.addr());
        if form == 0 {
            return f64::NAN;
        }
        f64::from(e.mem.u8(e.mem.u32(form + 0x30) + 4))
    });
    e.mem.set_f64(result, value);
    ret(1)
}

/// The engine for lines whose command takes a lambda: [`line_engine`], `PlugCmd` taking one form
/// (`kParamType_AnyForm`) run by [`form_handler`], and the stand-ins above.
fn lambda_engine(row: bool) -> Engine {
    let mut e = line_engine(row);
    LAMBDAS.with(|l| *l.borrow_mut() = 0);
    parameters(&mut e, &[ANY_FORM], 1);
    let record = e.script_tables().record(PLUGIN_COMMAND);
    e.mem.set_u32(record + 0x18, FORM_HANDLER);
    e.register(FORM_HANDLER, form_handler);
    e.register(BUFFER_CONSTRUCTOR, |e, a| {
        let data = e.call(FORM_HEAP_ALLOCATE, &[0x4000]).u32();
        e.mem.set_u32(a[0] + 0x20, data);
        ret(a[0])
    });
    e.register(SCRIPT_CONSTRUCTOR, |e, a| {
        e.mem.write(a[0], &[0; 0x54]);
        e.mem.set_u32(a[0], LAMBDA_VTABLE);
        e.mem.set_u8(a[0] + 4, 0x11);
        ret(a[0])
    });
    e.mem.set_u32(LAMBDA_VTABLE + 0x128, SET_FORM_ID);
    e.register(SET_FORM_ID, |e, a| {
        e.mem.set_u32(a[0] + 0xc, a[1]);
        Ret::default()
    });
    e.mem.set_u32(MANAGER, MANAGER + 0x10);
    e.register(CONSOLE_MANAGER, |_, _| ret(MANAGER));
    e.register(SCRIPT_COMPILE, compile_lambda);
    for quiet in [
        BUFFER_DESTRUCTOR,
        SCRIPT_DESTRUCTOR,
        DISABLE_ASSIGN_FORM_IDS,
        COPY_VAR_LIST,
        COPY_REF_LIST,
        LOOKUP_FORM,
    ] {
        e.register(quiet, nothing);
    }
    e
}

/// Two `set` lines whose command's argument is a different lambda (`SetFunctionValue 1` and `2`),
/// compiled by `CompileLine` and run by `CExpression::Eval`: the command gets each line's own
/// lambda (1, then 2), in both presets. Lambdas are known by their token's address and their
/// position on the script's data, so a command handed the copy of its token on `Eval`'s stack
/// (seam `005949D4` gone) would get the first line's lambda on the second line.
#[test]
fn two_lines_with_different_lambdas_get_their_own() {
    for row in [false, true] {
        let mut e = lambda_engine(row);
        let lines: Vec<u32> = [1, 2]
            .iter()
            .map(|n| {
                let text = format!(" f to PlugCmd (begin function {{}} SetFunctionValue {n} end)");
                compile_line(&mut e, 0x15, &text)
                    .unwrap_or_else(|| panic!("row {row}, lambda {n}: {:x?}", formats()))
            })
            .collect();
        for (line, want) in lines.iter().zip([1.0, 2.0]) {
            assert_eq!(evaluate(&mut e, line + 4 + 3 + 2), want, "row {row}");
        }
        assert_eq!(BIG_COPIES.with(|c| c.borrow().clone()), vec![]);
    }
}
