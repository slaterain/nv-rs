//! xNVSE's run-time line reader (`nvse::line_reader::parse_next_line`, its `ParseNextLine`, which
//! xNVSE puts in place of the game's `ScriptCompiler::GetNextLine` `005AF3F0` for every script
//! compiled while the game runs, the console's lines and the old syntax's plain scripts
//! included) against the game's own reader (the translation), on scripts written as the game's
//! own and most mods' are: carriage returns and line feeds, tabs for indents, `;` comments, blank
//! lines, a last line without a line end. Every line must come out with the same number (the
//! number every compile error of the line names) and the same text, but for runs of blanks
//! between words, which xNVSE makes one (the game's word reader, `GetNextWord`, steps over any
//! run, so the line compiles to the same bytes); and both readers must use up the same text.
//! Here because this crate registers every translation; the stand-ins are the C runtime's
//! `isalnum` and `memset`, which the game's reader calls.

use engine as _;
use engine_core::prelude::*;

const AREA: u32 = 0x0480_0000;
/// The script buffer (its text and offset), the line, the text.
const BUF: u32 = AREA;
const LINE: u32 = AREA + 0x100;
const TEXT: u32 = AREA + 0x1000;
/// The C runtime's `isalnum` and `memset` as the game's reader calls them.
const ISALNUM: u32 = 0x00ec_89e9;
const MEMSET: u32 = 0x0040_3d30;
/// `ScriptBuffer`: the text, its offset, the current line. `ScriptLineBuffer`: the number, the
/// text, its length, its error.
const BUF_TEXT: u32 = 0;
const BUF_OFFSET: u32 = 4;
const BUF_LINE: u32 = 0x1c;
const LINE_TEXT: u32 = 4;
const LINE_LEN: u32 = 0x204;
const LINE_ERROR: u32 = 0x418;

fn ret(v: u32) -> Ret {
    Ret {
        eax: v,
        ..Ret::default()
    }
}

fn engine() -> Engine {
    let mut e = Engine::new_untyped_for_test();
    if !e.mem.is_mapped(AREA) {
        e.map(AREA, 0x1_0000);
    }
    e.register(ISALNUM, |_, a| {
        ret(u32::from((a[0] as u8).is_ascii_alphanumeric()))
    });
    e.register(MEMSET, |e, a| {
        for i in 0..a[2] {
            e.mem.set_u8(a[0] + i, a[1] as u8);
        }
        ret(a[0])
    });
    e
}

/// Every line one reader gives from `text`, as `GetNextExpression` asks for them (the offset
/// moved on by each read, each line numbered after the last line read): its number and text,
/// and the offset where reading ended.
fn read_all(
    e: &mut Engine,
    text: &[u8],
    read: fn(&mut Engine) -> u32,
) -> (Vec<(u32, Vec<u8>)>, u32) {
    e.mem.write(TEXT, text);
    e.mem.set_u8(TEXT + text.len() as u32, 0);
    e.mem.set_u32(BUF + BUF_TEXT, TEXT);
    e.mem.set_u32(BUF + BUF_OFFSET, 0);
    e.mem.set_u32(BUF + BUF_LINE, 0);
    let mut lines = Vec::new();
    while (e.mem.u32(BUF + BUF_OFFSET) as usize) < text.len() {
        e.mem.write(LINE, &[0; 0x41c]);
        let number = e.mem.u32(BUF + BUF_LINE) + 1;
        e.mem.set_u32(LINE, number);
        let n = read(e);
        assert_eq!(e.mem.u32(LINE + LINE_ERROR), 0, "no error on these scripts");
        if n == 0 {
            break;
        }
        let offset = e.mem.u32(BUF + BUF_OFFSET) + n;
        e.mem.set_u32(BUF + BUF_OFFSET, offset);
        let number = e.mem.u32(LINE);
        e.mem.set_u32(BUF + BUF_LINE, number);
        let len = e.mem.u32(LINE + LINE_LEN).min(0x200);
        lines.push((number, e.mem.bytes(LINE + LINE_TEXT, len)));
    }
    (lines, e.mem.u32(BUF + BUF_OFFSET))
}

fn game(e: &mut Engine) -> u32 {
    e.call(0x005a_f3f0, &[BUF, LINE]).u32()
}

fn xnvse(e: &mut Engine) -> u32 {
    nvse::line_reader::parse_next_line(e, BUF, LINE)
}

/// A run of blanks (space, tab) outside strings made its first blank, as xNVSE's reader does.
fn one_blank_per_run(text: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut in_string = false;
    let mut last_blank = false;
    for &c in text {
        let blank = c == b' ' || c == b'\t';
        if blank && last_blank && !in_string {
            continue;
        }
        if c == b'"' {
            in_string = !in_string;
        }
        last_blank = blank;
        out.push(c);
    }
    out
}

const SCRIPTS: [&[u8]; 6] = [
    b"scn VMS01Script\r\n\r\nshort DoOnce\r\nfloat\tfTimer\r\nref\t\trTarget\r\n\r\nbegin GameMode\r\n\r\n\tif DoOnce == 0\r\n\t\tset fTimer to fTimer + GetSecondsPassed ; count up\r\n\t\tif fTimer >= 5\r\n\t\t\tplayer.additem Caps001 10\r\n\t\t\tShowMessage VMS01Msg\r\n\t\t\tset DoOnce to 1\r\n\t\tendif\r\n\tendif\r\n\r\nend\r\n\r\nbegin OnActivate\r\n\tif IsActionRef player == 1\r\n\t\tActivate\r\n\tendif\r\nend\r\n",
    b"ScriptName NoLineEnd\r\n; only notes here\r\n;\r\n   ; and here\r\nshort  x\r\nbegin  OnLoad\r\n  set x   to  x + 1\r\nend",
    b"scn Strings\r\nbegin GameMode\r\n\tMessageBoxEx \"Two  spaces  kept\"\r\n\tPrintToConsole \"a\tb\"\r\n\tif 1 ; x\r\n\tendif ;\r\nend\r\n\r\n\r\n",
    b"scn Q\r\n\r\n\r\nbegin\tMenuMode 1007\r\n\tset x to GetGameSetting fJumpHeightMin\r\n\t\r\n \t \r\nend\r\n",
    b"scn Paren\r\nshort a\r\nshort b\r\nfloat f\r\nbegin GameMode\r\n\tif (GetDead == 0) && (IsInCombat == 1)\r\n\t\tset f to ((a + b) * 2)\r\n\tendif\r\n\tif ((a == 1) || (b == 2)) && (f > 0)\r\n\t\tset a to (b)\r\n\tendif\r\nend\r\n",
    b"scn ParenRuns\r\nbegin OnLoad\r\n\tset  f  to  ( ( a + 1 )  *  ( b - 1 ) ) ; nested, blank runs\r\nend",
];

/// Each script read by both readers: the same lines, numbers and text (blank runs aside), the
/// same text used up.
#[test]
fn plain_scripts_read_the_same_by_both_readers() {
    for script in SCRIPTS {
        let mut e = engine();
        let (by_game, game_end) = read_all(&mut e, script, game);
        let mut e = engine();
        let (by_xnvse, xnvse_end) = read_all(&mut e, script, xnvse);
        let name = String::from_utf8_lossy(&script[..script.len().min(20)]).into_owned();
        assert!(!by_game.is_empty(), "{name}");
        assert_eq!(by_game.len(), by_xnvse.len(), "{name}: the lines");
        for ((gn, gt), (xn, xt)) in by_game.iter().zip(&by_xnvse) {
            assert_eq!(gn, xn, "{name}: {}", String::from_utf8_lossy(gt));
            assert_eq!(
                one_blank_per_run(gt),
                *xt,
                "{name}: line {gn}: {:?} against {:?}",
                String::from_utf8_lossy(gt),
                String::from_utf8_lossy(xt)
            );
        }
        // Both stop where only blanks are left (a read of 0), but the game's reader counts the
        // end byte of a last line without a line end and xNVSE's does not.
        let ends_with_text = !script.ends_with(b"\n");
        assert_eq!(
            game_end,
            xnvse_end + u32::from(ends_with_text),
            "{name}: the text used up"
        );
        assert!(xnvse_end <= script.len() as u32);
    }
}

/// Lines without blank runs come out byte for byte the same.
#[test]
fn lines_without_blank_runs_are_byte_for_byte_the_same() {
    let script = SCRIPTS[0];
    let mut e = engine();
    let (by_game, _) = read_all(&mut e, script, game);
    let mut e = engine();
    let (by_xnvse, _) = read_all(&mut e, script, xnvse);
    let same = by_game
        .iter()
        .zip(&by_xnvse)
        .filter(|((_, g), _)| one_blank_per_run(g) == *g)
        .inspect(|((_, g), (_, x))| assert_eq!(g, x))
        .count();
    assert!(same >= by_game.len() - 3, "{same} of {}", by_game.len());
}
