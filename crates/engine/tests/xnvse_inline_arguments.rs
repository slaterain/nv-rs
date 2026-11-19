//! A command's argument written in parentheses, compiled through the game's `StandardCompile`
//! (`005b1ba0`, translated) with xNVSE's seam at its word read of each parameter (`0x5B1C65`,
//! `nvse::line_reader`), then read back through the game's argument reader `005accb0` as the
//! command's handler calls it (with xNVSE's wrap over it, `nvse::args`):
//! - a plugin's command of the game's format (opcode `0x1400` up, JIP's for one): a parenthesised
//!   number and a parenthesised string compile to xNVSE's `0xFFFF` form and read back as their
//!   values, through xNVSE's reader;
//! - one of the game's own commands: the seam leaves the word to the game, as before xNVSE's patch
//!   (the exe's reader, which these commands read through, cannot read `0xFFFF`): a parenthesised
//!   number or string is refused when the line compiles ("missing parameter"), nothing written;
//!   the same line written without parentheses compiles and reads back as it always did.
//!
//! Here because this crate registers every translation. The stand-ins are the C runtime calls
//! that read its locale tables (character classes, case-blind compares, `atof`), the float test,
//! the global lookup (no data handler here), the form cast, and the error report's message
//! (counted by its format) and printing.

use engine as _;
use engine_core::prelude::*;
use std::cell::RefCell;

const AREA: u32 = 0x04a0_0000;
/// The compile data, the line, the parameters and their names, the compiled command read back,
/// its offset word, the outputs (a text buffer, a float), the script and its locals.
const BUF: u32 = AREA;
const LINE: u32 = AREA + 0x100;
const PARAMS: u32 = AREA + 0x800;
const NAMES: u32 = AREA + 0x900;
const DATA: u32 = AREA + 0x1000;
const OFFSET: u32 = AREA + 0x1f00;
const OUT_TEXT: u32 = AREA + 0x2000;
const OUT_FLOAT: u32 = AREA + 0x2400;
const SCRIPT: u32 = AREA + 0x3000;
const LOCALS: u32 = AREA + 0x3400;
/// The game's parameter type table (`0118cdd4`, 8 bytes a type: +0 a variable may stand there,
/// +1 it names an object), in the exe's data with the compiler's tables (the keywords, the
/// script and console commands) and the globals the compile reads (the data handler).
const PARAMETER_TYPES: u32 = 0x0118_cdd4;
const TABLES: u32 = 0x0118_0000;
const GLOBALS: u32 = 0x011c_0000;
/// `ScriptError`'s message format (`00407000`, recorded) and its printing.
const VSPRINTF: u32 = 0x0040_7000;
const STRING_PRINTF: u32 = 0x0040_6f60;
const PREPARE_MESSAGE: u32 = 0x0046_4f30;
const CONSOLE_PRINT: u32 = 0x0070_3c00;
const SCRIPT_LOG: u32 = 0x005b_5e40;
/// "Missing parameter %s." (the exe's format).
const TEXT_MISSING_PARAMETER: u32 = 0x0103_9080;
/// The C runtime's locale readers, the float test and `atof`, the global lookup, the form cast.
const ISALNUM: u32 = 0x00ec_89e9;
const ISALPHA: u32 = 0x00ec_86e1;
const TOUPPER: u32 = 0x00ec_a7f4;
const TOLOWER: u32 = 0x00ec_67aa;
const STRICMP: u32 = 0x0040_4dc0;
const STRNICMP: u32 = 0x00ec_7ec0;
const IS_A_FLOAT: u32 = 0x004b_12d0;
const ATOF: u32 = 0x00ec_a573;
const GET_GLOBAL: u32 = 0x0046_1700;
const DYNAMIC_CAST: u32 = 0x00ec_43fb;
/// A plugin's opcode (JIP's block) and one of the game's.
const PLUGIN_COMMAND: u32 = 0x2400;
const GAME_COMMAND: u32 = 0x1004;
/// The parameter types: a string, a float.
const STRING: u32 = 0;
const FLOAT: u32 = 2;

thread_local! {
    static FORMATS: RefCell<Vec<u32>> = const { RefCell::new(Vec::new()) };
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

fn engine() -> Engine {
    let mut e = Engine::new_untyped_for_test();
    for (at, size) in [(AREA, 0x4000), (TABLES, 0x2_0000), (GLOBALS, 0x2_0000)] {
        if !e.mem.is_mapped(at) {
            e.map(at, size);
        }
    }
    nvse::line_reader::install(&mut e);
    nvse::args::install(&mut e);
    // The exe's bytes for the two types: a string is no variable and no object; a float may be
    // a variable.
    e.mem.write(PARAMETER_TYPES + STRING * 8, &[0, 0]);
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
        let t = String::from_utf8_lossy(&text(e, a[0])).into_owned();
        ret(u32::from(t.parse::<f64>().is_ok()))
    });
    e.register(ATOF, |e, a| {
        let t = String::from_utf8_lossy(&text(e, a[0])).into_owned();
        Ret {
            st0: t.parse().unwrap_or(0.0),
            ..Ret::default()
        }
    });
    e.register(GET_GLOBAL, |_, _| ret(0));
    e.register(DYNAMIC_CAST, |_, _| ret(0));
    e
}

/// The parameters (all required) of the given types at `PARAMS`.
fn parameters(e: &mut Engine, kinds: &[u32]) {
    e.mem.write(NAMES, b"argument\0");
    for (i, kind) in kinds.iter().enumerate() {
        let p = PARAMS + 12 * i as u32;
        e.mem.set_u32(p, NAMES);
        e.mem.set_u32(p + 4, *kind);
        e.mem.set_u32(p + 8, 0);
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
    let formats = FORMATS.with(|f| f.borrow().clone());
    (ok, e.mem.bytes(LINE + 0x20c, len), formats)
}

/// `output` read back through `005accb0` for `opcode` as a handler calls it (a compiled script,
/// its locals; the outputs a text buffer and a float): the answer, the text, the float.
fn read_back(e: &mut Engine, opcode: u32, output: &[u8]) -> (bool, Vec<u8>, f32) {
    e.mem.set_u16(DATA, opcode as u16);
    e.mem.set_u16(DATA + 2, output.len() as u16);
    e.mem.write(DATA + 4, output);
    e.mem.set_u32(OFFSET, 4);
    e.mem.write(OUT_TEXT, &[0; 0x200]);
    e.mem.set_f32(OUT_FLOAT, -1.0);
    // The script's form flags: compiled (bit 3; the exe's reader stores nothing without it).
    e.mem.set_u32(SCRIPT + 8, 8);
    let ok = e
        .call(
            0x005a_ccb0,
            &[
                PARAMS, DATA, OFFSET, 0, 0, SCRIPT, LOCALS, OUT_TEXT, OUT_FLOAT,
            ],
        )
        .bool();
    (ok, text(e, OUT_TEXT), e.mem.f32(OUT_FLOAT))
}

/// A plugin's command: `"key" (1 + 2)` and `("k" + "ey") (4 * 2)` compile (each parenthesised
/// argument `0xFFFF` and its expression) and read back as their values.
#[test]
fn a_plugin_commands_parenthesised_arguments_compile_and_read_back() {
    let mut e = engine();
    parameters(&mut e, &[STRING, FLOAT]);
    let (ok, output, errors) = compile(&mut e, PLUGIN_COMMAND, 2, "\"key\" (1 + 2)");
    assert!(ok && errors.is_empty(), "{output:x?} {errors:x?}");
    assert_eq!(
        &output[..7],
        b"\x02\x00\x03\x00key",
        "the string as the game writes it"
    );
    assert_eq!(&output[7..9], &[0xff, 0xff], "xNVSE's expression");
    assert_eq!(
        read_back(&mut e, PLUGIN_COMMAND, &output),
        (true, b"key".to_vec(), 3.0)
    );
    let (ok, output, errors) = compile(&mut e, PLUGIN_COMMAND, 2, "(\"k\" + \"ey\") (4 * 2)");
    assert!(ok && errors.is_empty(), "{output:x?} {errors:x?}");
    assert_eq!(&output[2..4], &[0xff, 0xff]);
    assert_eq!(
        read_back(&mut e, PLUGIN_COMMAND, &output),
        (true, b"key".to_vec(), 8.0)
    );
}

/// One of the game's commands: the seam leaves the word to the game. A parenthesised number and
/// a parenthesised string are each refused ("missing parameter", the line does not compile),
/// and nothing is written as `0xFFFF`. Without parentheses the line compiles and the exe's
/// reader gives the values back.
#[test]
fn a_game_commands_parenthesised_arguments_are_refused() {
    let mut e = engine();
    for (kind, args) in [(FLOAT, "(1 + 2)"), (STRING, "(\"abc\")")] {
        parameters(&mut e, &[kind]);
        let (ok, output, errors) = compile(&mut e, GAME_COMMAND, 1, args);
        assert!(!ok, "{args}: {output:x?}");
        assert_eq!(errors, vec![TEXT_MISSING_PARAMETER], "{args}");
        assert!(
            !output.windows(2).any(|w| w == [0xff, 0xff]),
            "{args}: {output:x?}"
        );
    }
    parameters(&mut e, &[STRING, FLOAT]);
    let (ok, output, errors) = compile(&mut e, GAME_COMMAND, 2, "\"abc\" 5");
    assert!(ok && errors.is_empty(), "{output:x?} {errors:x?}");
    assert_eq!(
        output,
        [&b"\x02\x00\x03\x00abcz"[..], &5.0f64.to_le_bytes()].concat()
    );
    assert_eq!(
        read_back(&mut e, GAME_COMMAND, &output),
        (true, b"abc".to_vec(), 5.0)
    );
}
