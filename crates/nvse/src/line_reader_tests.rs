//! Tests of xNVSE's run-time line reader and line macros, from xNVSE 6.4.9's `ParseNextLine`,
//! `g_lineMacros` and `EvalMacro` (`Hooks_Editor.cpp`, `ScriptUtils.cpp`): each rule at its edges,
//! the two check files of the JIP in-game pair as they were written (line feeds only, a lambda
//! over three lines), on the compiler tests' engine (its console and its script `Test`).

use super::*;
use crate::commands::test_support::SCRIPT;
use crate::compiler::tests::{engine, printed, BUF, LINE};

/// Where the tests' script texts go (inside the compiler tests' memory).
const TEXT_AT: u32 = crate::compiler::tests::AREA + 0x3_0000;

/// Every line [`parse_next_line`] reads from `text` as the game's `GetNextExpression` asks for
/// them: the buffer's offset moved on by each read, each line numbered after the last line read,
/// until the text is used up or a read answers 0 (its error code then the last item, number 0).
fn read_all(e: &mut Engine, text: &[u8]) -> Vec<(u32, String)> {
    e.mem.write(TEXT_AT, text);
    e.mem.set_u8(TEXT_AT + text.len() as u32, 0);
    e.mem.set_u32(BUF + sb::TEXT, TEXT_AT);
    e.mem.set_u32(BUF + TEXT_OFFSET, 0);
    e.mem.set_u32(BUF + sb::CUR_LINE, 0);
    state(e).next_line_increment = 0;
    let mut lines = Vec::new();
    while (e.mem.u32(BUF + TEXT_OFFSET) as usize) < text.len() {
        e.mem.write(LINE, &[0; ln::STRUCT as usize]);
        let number = e.mem.u32(BUF + sb::CUR_LINE) + 1;
        e.mem.set_u32(LINE + ln::NUMBER, number);
        let read = parse_next_line(e, BUF, LINE);
        if read == 0 {
            let code = e.mem.u32(LINE + LINE_ERROR);
            if code != 0 {
                lines.push((0, format!("error {code}")));
            }
            break;
        }
        let offset = e.mem.u32(BUF + TEXT_OFFSET) + read;
        e.mem.set_u32(BUF + TEXT_OFFSET, offset);
        let number = e.mem.u32(LINE + ln::NUMBER);
        e.mem.set_u32(BUF + sb::CUR_LINE, number);
        let len = e.mem.u32(LINE + ln::TEXT_LEN);
        let text = e.mem.bytes(LINE + ln::TEXT, len.min(ln::SIZE));
        lines.push((number, String::from_utf8_lossy(&text).into_owned()));
    }
    lines
}

fn lines(items: &[(u32, &str)]) -> Vec<(u32, String)> {
    items.iter().map(|(n, t)| (*n, t.to_string())).collect()
}

// ---- line ends, blanks, comments ----

/// A line feed ends a line, with or without a carriage return before it (cut as a trailing
/// blank); a carriage return alone is a blank inside the line (xNVSE ends lines at line feeds
/// only); the text's end ends the last line.
#[test]
fn lines_end_at_line_feeds() {
    let mut e = engine();
    assert_eq!(
        read_all(&mut e, b"a 1\nb 2\r\nc 3"),
        lines(&[(1, "a 1"), (2, "b 2"), (3, "c 3")])
    );
    assert_eq!(read_all(&mut e, b"a\rb\r"), lines(&[(1, "a\rb")]));
    assert_eq!(read_all(&mut e, b"a\r\n"), lines(&[(1, "a")]));
}

/// Blank lines and lines with only a comment count their numbers; leading blanks and line feeds
/// are skipped (counted); a `;` comment cuts the rest of its line, the blanks before it too.
#[test]
fn blank_and_comment_lines_count_their_numbers() {
    let mut e = engine();
    assert_eq!(
        read_all(
            &mut e,
            b"\r\n\r\n  scn Test\r\n; a note\r\n\r\n\tset x to 1 ; why\r\n   \r\nend"
        ),
        lines(&[(3, "scn Test"), (6, "set x to 1"), (8, "end")])
    );
}

/// `/*` to `*/` is left out (outside strings), its line feeds counted at once (the line it is in
/// takes the number its end has); inside a string it is text. An unclosed block is an error at
/// the text's end.
#[test]
fn block_comments() {
    let mut e = engine();
    assert_eq!(
        read_all(&mut e, b"a /* one\ntwo */ b\nc \"/* x */\"\nd"),
        lines(&[(2, "a b"), (3, "c \"/* x */\""), (4, "d")])
    );
    printed();
    assert_eq!(
        read_all(&mut e, b"a\nb /* open"),
        lines(&[(1, "a"), (0, "error 1")])
    );
    assert_eq!(
        printed(),
        vec!["Error on line 2\n\nMismatched comment block (missing '*/' for a present '/*')"]
    );
    assert_eq!(
        e.mem.u32(BUF + sb::CUR_LINE),
        2,
        "the buffer takes the line's number"
    );
}

/// A run of blanks outside a string is one blank (its first); inside a string it is kept.
#[test]
fn blanks_collapse_outside_strings() {
    let mut e = engine();
    assert_eq!(
        read_all(&mut e, b"a  \t b   \"x  \t y\"   c\t\t"),
        lines(&[(1, "a b \"x  \t y\" c")])
    );
}

/// `;` starts a comment even inside a string (xNVSE does not test for one there): at a line feed
/// the line is what came before; at the text's end the string is open (an error).
#[test]
fn a_semicolon_cuts_even_in_a_string() {
    let mut e = engine();
    assert_eq!(
        read_all(&mut e, b"print \"a;b\"\nnext"),
        lines(&[(1, "print \"a"), (2, "next")])
    );
    printed();
    assert_eq!(read_all(&mut e, b"print \"a;b\""), lines(&[(0, "error 1")]));
    assert_eq!(
        printed(),
        vec!["Error on line 1\n\nMismatched quotes. A string literal was not closed."]
    );
}

// ---- parentheses ----

/// Inside open parentheses a line feed does not end the line: it stays in the text and the
/// lines it held are added to the next line's number. Blanks after it collapse with it.
#[test]
fn parentheses_join_lines_and_number_the_next() {
    let mut e = engine();
    assert_eq!(
        read_all(&mut e, b"a (b\n   c\n)\nd\n\ne"),
        lines(&[(1, "a (b\nc\n)"), (4, "d"), (6, "e")])
    );
    // Parentheses inside strings do not count.
    assert_eq!(
        read_all(&mut e, b"a \"(\"\nb \")\""),
        lines(&[(1, "a \"(\""), (2, "b \")\"")])
    );
}

/// A `)` with none open is an error at once; parentheses open at the text's end too.
#[test]
fn mismatched_parentheses() {
    let mut e = engine();
    printed();
    assert_eq!(read_all(&mut e, b"a )\n"), lines(&[(0, "error 1")]));
    assert_eq!(printed(), vec!["Error on line 1\n\nMismatched parenthesis"]);
    assert_eq!(
        read_all(&mut e, b"x\na (b\nc"),
        lines(&[(1, "x"), (0, "error 1")])
    );
    assert_eq!(
        printed(),
        vec!["Error on line 2\n\nMismatched parentheses. Parentheses were not closed."]
    );
    assert_eq!(state(&mut e).next_line_increment, 0);
}

// ---- the 512-character limit ----

/// 512 characters are read (the game's own reader stops at 511); the 513th is an error
/// ("Max script line length", error 16), inside parentheses the other message.
#[test]
fn the_512_character_limit() {
    let mut e = engine();
    let line_of = |n: usize| -> Vec<u8> {
        let mut t = b"a".to_vec();
        t.extend(std::iter::repeat_n(b'b', n - 1));
        t
    };
    let ok = read_all(&mut e, &line_of(512));
    assert_eq!(ok.len(), 1);
    assert_eq!(ok[0].1.len(), 512);
    assert_eq!(
        e.mem.u32(LINE + ln::TEXT_LEN),
        512,
        "the length's own low byte is the end"
    );
    printed();
    assert_eq!(read_all(&mut e, &line_of(513)), lines(&[(0, "error 16")]));
    assert_eq!(
        printed(),
        vec!["Error on line 1\n\nMax script line length (512 characters) exceeded."]
    );
    let mut inside = b"a (".to_vec();
    inside.extend(std::iter::repeat_n(b'b', 510));
    inside.push(b')');
    assert_eq!(read_all(&mut e, &inside), lines(&[(0, "error 16")]));
    assert_eq!(
        printed(),
        vec![
            "Error on line 1\n\nMax script expression length inside parenthesis (512 characters) exceeded."
        ]
    );
    // 511 and a line feed: read whole.
    let mut text = line_of(511);
    text.extend_from_slice(b"\nnext");
    assert_eq!(read_all(&mut e, &text).len(), 2);
}

// ---- the JIP in-game pair's check files, as written ----

/// The first check file as the pair's first run had it (line feeds only, `fW` and the others):
/// every line read whole with its own number, where the game's reader read one line of 600
/// characters and refused it.
#[test]
fn the_first_check_file_with_line_feeds() {
    let text = "float fCheck\nfloat fW\nfloat fH\nfloat fL\nSetINIFloat \"check:ran\" 1 \"nvrs_jipcheck\"\n\
set fCheck to GetLNVersion\nSetINIFloat \"check:lnversion\" fCheck \"nvrs_jipcheck\"\n\
set fCheck to IsPluginInstalled \"JIP LN NVSE\"\nSetINIFloat \"check:installed\" fCheck \"nvrs_jipcheck\"\n\
AuxiliaryVariableSetFloat \"*nvrscheck\" 42 0 PlayerRef\n\
set fCheck to AuxiliaryVariableGetFloat \"*nvrscheck\" 0 PlayerRef\n\
SetINIFloat \"check:auxvar\" fCheck \"nvrs_jipcheck\"\nRefMapArraySetFloat \"*nvrsmap\" 7 PlayerRef\n\
set fCheck to RefMapArrayGetFloat \"*nvrsmap\" PlayerRef\nSetINIFloat \"check:refmap\" fCheck \"nvrs_jipcheck\"\n\
GetStringUIDimensions \"abc\" 89 0 fW fH fL\nSetINIFloat \"check:font89width\" fW \"nvrs_jipcheck\"\n\
GetStringUIDimensions \"abc\" 9 0 fW fH fL\nSetINIFloat \"check:font9width\" fW \"nvrs_jipcheck\"\n\
SetINIFloat \"check:end\" 1 \"nvrs_jipcheck\"\n";
    let mut e = engine();
    let read = read_all(&mut e, text.as_bytes());
    let want: Vec<(u32, String)> = text
        .lines()
        .enumerate()
        .map(|(i, l)| (i as u32 + 1, l.to_string()))
        .collect();
    assert_eq!(read, want);
    assert!(printed().is_empty());
}

/// The second check file (line feeds, a lambda over three lines): the lambda's line is read whole,
/// its line feeds kept (the lambda's compile counts them) and the indent collapsed; the next line
/// is numbered 4.
#[test]
fn the_second_check_file_lambda_is_one_line() {
    let text = b"SetEventHandler \"OnCellEnter\" (begin function {ref rCell}\n  SetINIFloat \"check:cellenter\" 1 \"nvrs_jipcheck\"\nend)\nSetINIFloat \"check:handlerset\" 1 \"nvrs_jipcheck\"\n";
    let mut e = engine();
    assert_eq!(
        read_all(&mut e, text),
        lines(&[
            (
                1,
                "SetEventHandler \"OnCellEnter\" (begin function {ref rCell}\nSetINIFloat \"check:cellenter\" 1 \"nvrs_jipcheck\"\nend)"
            ),
            (4, "SetINIFloat \"check:handlerset\" 1 \"nvrs_jipcheck\""),
        ])
    );
}

// ---- the assignment shorthand ----

/// Each line against what xNVSE's regex `^(\w+(\.\w+)*(\[.*\])*)\s*OP([^=][\S\s]*)` (ECMAScript,
/// anchored, the shorthands in order) makes of it, worked out by its rules: the target is the
/// first group (brackets included, the farthest `]` first), the rest from the byte after the
/// operator (blanks kept, so `x = 5` gets two).
#[test]
fn the_assignment_shorthand_at_its_edges() {
    for (line, want) in [
        ("x = 5", Some("let x :=  5")),
        ("x=5", Some("let x := 5")),
        ("x := 5", Some("let x :=  5")),
        ("x += 1", Some("let x +=  1")),
        ("x -= 1", Some("let x -=  1")),
        ("x *= 2", Some("let x *=  2")),
        ("x /= 2", Some("let x /=  2")),
        ("x |= 1", Some("let x |=  1")),
        ("x &= 1", Some("let x &=  1")),
        ("x %= 2", Some("let x %=  2")),
        ("x\t+=\t1", Some("let x += \t1")),
        ("x == 1", None),
        ("x = y == z", Some("let x :=  y == z")),
        ("MessageEx \"a = b\"", None),
        ("x = \"a = b\"", Some("let x :=  \"a = b\"")),
        ("a[(Rand 1, 3)] = 1", Some("let a[(Rand 1, 3)] :=  1")),
        ("a[1][2] = 3", Some("let a[1][2] :=  3")),
        ("a[x] = b[y] = c", Some("let a[x] = b[y] :=  c")),
        ("a[x\n] = 1", None),
        ("a[x\r] = 1", None),
        ("player.x = 5", Some("let player.x :=  5")),
        ("_x1 = 2", Some("let _x1 :=  2")),
        (".x = 5", None),
        ("int x = 5", Some("let int x :=  5")),
        ("int\tx = 5", Some("let int x :=  5")),
        ("Int x = 5", Some("let int x :=  5")),
        ("reference r = player", Some("let reference r :=  player")),
        ("int  x = 5", Some("let int x :=  5")),
        ("intx = 5", Some("let intx :=  5")),
        ("float", None),
        ("let x = 5", None),
        ("if x = 5", None),
        ("set x to 5", None),
        ("x =", None),
        ("x = ", Some("let x :=  ")),
        ("x = = 1", Some("let x :=  = 1")),
    ] {
        let got = assignment_shorthand(line.as_bytes()).map(|v| String::from_utf8(v).unwrap());
        assert_eq!(got.as_deref(), want, "{line:?}");
    }
}

/// Through the reader: a shorthand line becomes its `let` (the length with it); one too long for
/// the line's 512 bytes is refused with xNVSE's message and the read fails.
#[test]
fn the_shorthand_through_the_reader() {
    let mut e = engine();
    assert_eq!(
        read_all(&mut e, b"x = 5\nx  +=  (1 +\n2)"),
        lines(&[(1, "let x :=  5"), (2, "let x +=  (1 +\n2)")])
    );
    assert_eq!(e.mem.u32(LINE + ln::TEXT_LEN), 17);
    printed();
    let mut long = b"x = ".to_vec();
    // 509 bytes: read whole, but its `let` would be 515.
    long.extend(std::iter::repeat_n(b'1', 505));
    assert_eq!(read_all(&mut e, &long), lines(&[]));
    assert_eq!(
        printed(),
        vec!["Line length limit reached, could not translate macro."]
    );
}

// ---- the multiple declaration ----

/// `int a, b, c`: `b` and `c` are made variables of the current script (`CreateVariable`, each
/// numbered after the last), the line itself left as it was; a single name, a part with another
/// byte, or no current script make nothing; a part naming a command is refused with its
/// message.
#[test]
fn the_multiple_declaration() {
    let mut e = engine();
    state(&mut e).scripts.push(SCRIPT);
    let count = e.mem.u32(BUF + sb::VAR_COUNT);
    assert_eq!(
        read_all(&mut e, b"int nA, n B ,nC"),
        lines(&[(1, "int nA, n B ,nC")])
    );
    let made = |e: &mut Engine, name: &[u8]| -> u32 {
        crate::compiler::list_items(e, BUF + sb::VARS)
            .into_iter()
            .find(|&info| crate::compiler::string_cstr(e, info + 0x18) == name)
            .map_or(0, |info| e.mem.u32(info))
    };
    assert_eq!(made(&mut e, b"nB"), count + 1, "blanks taken out of a part");
    assert_eq!(made(&mut e, b"nC"), count + 2);
    assert_eq!(made(&mut e, b"nA"), 0, "the first is the game's to declare");
    for text in [&b"int nD"[..], b"int nE, nF!", b"integer nG, nH"] {
        read_all(&mut e, text);
    }
    for name in [&b"nE"[..], b"nF", b"nF!", b"nH"] {
        assert_eq!(made(&mut e, name), 0, "{}", String::from_utf8_lossy(name));
    }
    printed();
    read_all(&mut e, b"float fA, Let");
    assert_eq!(
        printed(),
        vec![
            "Error on line 1\n\nInvalid variable name Let: Command with that name already exists."
        ]
    );
    state(&mut e).scripts.clear();
    read_all(&mut e, b"float fB, fC");
    assert_eq!(made(&mut e, b"fC"), 0, "no compile open");
}

// ---- the type keywords ----

/// At install, `Long`'s and `Float`'s alias slots in the keyword table name `string_var` and
/// `array_var` (xNVSE's `SafeWrite32`s); without the table's page nothing is written.
#[test]
fn the_type_keyword_aliases() {
    let mut e = engine();
    if !e.mem.is_mapped(LONG_ALIAS) {
        install(&mut e);
        e.map(0x0118_c000, 0x1000);
        assert_eq!(e.mem.u32(LONG_ALIAS), 0, "nothing written without the page");
        e = engine();
        e.map(0x0118_c000, 0x1000);
    }
    install(&mut e);
    let alias = |e: &Engine, at: u32| crate::table::guest_text(e, e.mem.u32(at));
    assert_eq!(alias(&e, LONG_ALIAS), b"string_var");
    assert_eq!(alias(&e, FLOAT_ALIAS), b"array_var");
    assert_eq!(LONG_ALIAS, 0x0118_cb50 + 3 * 0x28 + 4);
    assert_eq!(FLOAT_ALIAS, 0x0118_cb50 + 4 * 0x28 + 4);
}

// ---- the seams ----

/// The line reader's seam answers with the reader's count; the parameter seam answers nothing
/// for a word, 1 for an expression and 0 for a wrong one.
#[test]
fn the_seams_answer() {
    let mut e = engine();
    install(&mut e);
    e.mem.write(TEXT_AT, b"a b\nc\0");
    e.mem.set_u32(BUF + sb::TEXT, TEXT_AT);
    e.mem.set_u32(BUF + TEXT_OFFSET, 0);
    e.mem.write(LINE, &[0; ln::STRUCT as usize]);
    assert_eq!(e.site(LINE_READER.addr(), &[BUF, LINE]), Some(4));
    assert_eq!(e.mem.bytes(LINE + ln::TEXT, 4), b"a b\0");
    assert_eq!(LINE_READER.addr(), 0x005a_f3f0);
    assert_eq!(PARAMETER_WORD.addr(), 0x005b_1c65);
}

// ---- the port's log ----

/// An xNVSE message's line in the port's own log: its lines trimmed and joined.
#[test]
fn the_log_line() {
    assert_eq!(
        crate::compiler::log_line(
            b"Error on line 3\n\nMax script line length (512 characters) exceeded."
        ),
        b"xNVSE: Error on line 3: Max script line length (512 characters) exceeded."
    );
    assert_eq!(
        crate::compiler::log_line(b"  one  \r\n\r\n two\n"),
        b"xNVSE: one: two"
    );
    assert_eq!(crate::compiler::log_line(b""), b"xNVSE: ");
}

/// A compile error is also written to `nvse.log` (xNVSE's `_MESSAGE` after its `Console_Print`),
/// through the game's file calls: the log made at its first message, each line ended `\r\n`.
#[test]
fn compile_errors_reach_nvse_log() {
    use std::cell::RefCell;
    thread_local! {
        static LOG: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
    }
    fn eax(v: u32) -> Ret {
        Ret {
            eax: v,
            ..Ret::default()
        }
    }
    let mut e = engine();
    LOG.with(|l| l.borrow_mut().clear());
    e.register(crate::commands::array::CREATE_FILE_A, |e, a| {
        assert_eq!(crate::table::guest_text(e, a[0]), b"nvse.log");
        eax(0x44)
    });
    e.register(0x00fd_f240, |_, _| eax(0));
    e.register(0x00fd_f0dc, |e, a| {
        let bytes = e.mem.bytes(a[1], a[2]);
        LOG.with(|l| l.borrow_mut().extend(bytes));
        eax(1)
    });
    e.register(0x00fd_f07c, |_, _| eax(1));
    printed();
    read_all(&mut e, b"a )\n");
    assert_eq!(printed(), vec!["Error on line 1\n\nMismatched parenthesis"]);
    assert_eq!(
        LOG.with(|l| l.borrow().clone()),
        b"Error on line 1\r\n\r\nMismatched parenthesis\r\n"
    );
}

// ---- the review round (REVIEW-792): nested parentheses, the seam's glue, the macro room ----

/// Nested and repeated parentheses: `if ((a) && (b))` is one line; a group held over three line
/// feeds is one line and the next is numbered after them; a lambda whose body has `if (x)` on its
/// own line is one line up to its `end)`.
#[test]
fn nested_parentheses_through_the_reader() {
    let mut e = engine();
    assert_eq!(
        read_all(&mut e, b"if ((a) && (b))\nnext"),
        lines(&[(1, "if ((a) && (b))"), (2, "next")])
    );
    assert_eq!(
        read_all(&mut e, b"x ((a\n(b)\n)\nc)\nd"),
        lines(&[(1, "x ((a\n(b)\n)\nc)"), (5, "d")])
    );
    assert_eq!(
        read_all(
            &mut e,
            b"SetEventHandler \"OnHit\" (begin function {ref r}\nif (r)\nPrint \"hi\"\nendif\nend)\nnext"
        ),
        lines(&[
            (
                1,
                "SetEventHandler \"OnHit\" (begin function {ref r}\nif (r)\nPrint \"hi\"\nendif\nend)"
            ),
            (6, "next"),
        ])
    );
}

/// A parameter list of one float parameter at `at` (xNVSE's `ParamInfo`: the name, the type, the
/// optional flag).
fn one_float_parameter(e: &mut Engine, at: u32) -> u32 {
    e.mem.write(at + 0x40, b"value\0");
    e.mem.set_u32(at, at + 0x40);
    e.mem.set_u32(at + 4, 2);
    e.mem.set_u32(at + 8, 0);
    at
}

/// `StandardCompile`'s seam as the translation asks it (words: the line, the script buffer, the
/// parameters, the index): for an extender's command (`0x1400` up) a `(` compiles the expression
/// (1, `0xFFFF` and the expression in the output), an unclosed one is an error (0), a plain word
/// is the game's (none); for the game's own commands nothing is answered.
#[test]
fn the_parameter_seam_answers() {
    let mut e = engine();
    install(&mut e);
    let params = one_float_parameter(&mut e, TEXT_AT + 0x2000);
    let ask = |e: &mut Engine, opcode: u32, text: &str| {
        e.mem.write(LINE, &[0; ln::STRUCT as usize]);
        e.mem.set_u32(LINE + ln::NUMBER, 3);
        e.mem.write(LINE + ln::TEXT, text.as_bytes());
        e.mem.set_u32(LINE + ln::TEXT_LEN, text.len() as u32);
        e.mem.set_u32(LINE + ln::OPCODE, opcode);
        let answer = e.site(PARAMETER_WORD.addr(), &[LINE, BUF, params, 0]);
        let len = e.mem.u32(LINE + ln::DATA_LEN).min(ln::SIZE);
        (answer, e.mem.bytes(LINE + ln::DATA, len))
    };
    let (answer, bytes) = ask(&mut e, 0x2200, "(1 + 2)");
    assert_eq!(answer, Some(1), "{:?}", printed());
    assert_eq!(&bytes[..2], &[0xff, 0xff], "the expression's mark");
    assert!(bytes.len() > 4);
    assert_eq!(ask(&mut e, 0x2200, "5"), (None, vec![]));
    printed();
    assert_eq!(ask(&mut e, 0x2200, "(1 + 2").0, Some(0));
    assert_eq!(printed(), vec!["Error line 3\n\nMismatched brackets."]);
    assert_eq!(
        ask(&mut e, 0x1004, "(1 + 2)"),
        (None, vec![]),
        "a game command's"
    );
    assert_eq!(
        ask(&mut e, 0x13ff, "(1 + 2)"),
        (None, vec![]),
        "the last game opcode"
    );
    assert_eq!(
        ask(&mut e, 0x1400, "(1 + 2)").0,
        Some(1),
        "the first extender opcode"
    );
}

/// The shorthand's room: a `let` of 511 bytes fits the line (its length 511); one of 512 is
/// refused (the port's own difference: xNVSE's `strcpy_s` would end the game there).
#[test]
fn the_shorthand_room_at_511_and_512() {
    let mut e = engine();
    let line_of = |ones: usize| {
        let mut t = b"x = ".to_vec();
        t.extend(std::iter::repeat_n(b'1', ones));
        t
    };
    let read = read_all(&mut e, &line_of(501));
    assert_eq!(read.len(), 1);
    assert_eq!(read[0].1.len(), 511);
    assert_eq!(e.mem.u32(LINE + ln::TEXT_LEN), 511);
    printed();
    assert_eq!(read_all(&mut e, &line_of(502)), lines(&[]));
    assert_eq!(
        printed(),
        vec!["Line length limit reached, could not translate macro."]
    );
}
