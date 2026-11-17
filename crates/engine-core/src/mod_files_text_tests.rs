//! Tests of [`super`]: the encodings Windows' ANSI profile calls see, each case as measured on
//! scratch files (`build43-scratch/measure/jip-cmds/files-measure`, `ini6-`, `ini7-`,
//! `ini8-2026-11-16.txt`).

use super::*;
use crate::mod_files::{ini_value, removed_key, removed_section, section_names, written_profile};

/// UTF-16 LE text with its mark, as a test writes a file.
fn u16le(text: &str) -> Vec<u8> {
    let mut out = vec![0xff, 0xfe];
    for unit in text.encode_utf16() {
        out.extend_from_slice(&unit.to_le_bytes());
    }
    out
}

const BASE: &str = "[A]\r\nx=1\r\ny=2\r\n[B]\r\nz=3\r\n";

/// ini6 `U16LE ...`: read as text, changed as an ANSI file is, written back as UTF-16 LE with its
/// mark; the A calls' arguments and answers in the ANSI code page.
#[test]
fn a_utf16_le_file_is_read_and_written_as_text() {
    let file = Decoded::of(&u16le(BASE));
    assert_eq!(file.encoding, Encoding::Utf16Le);
    assert!(file.writable);
    let read = |s: &[u8], k: &[u8]| {
        ini_value(&file.text, &file.arg(s), &file.arg(k)).map(|v| file.answer(&v))
    };
    assert_eq!(read(b"A", b"y").unwrap(), b"2");
    assert_eq!(read(b"B", b"z").unwrap(), b"3");
    let names: Vec<Vec<u8>> = section_names(&file.text)
        .iter()
        .map(|n| file.answer(n))
        .collect();
    assert_eq!(names, [b"A".to_vec(), b"B".to_vec()]);
    let put = |k: &[u8], v: &[u8]| {
        file.encode(&written_profile(
            &file.text,
            &file.arg(b"A"),
            &file.arg(k),
            &file.arg(v),
        ))
    };
    assert_eq!(
        put(b"x", b"5"),
        u16le("[A]\r\nx=5\r\ny=2\r\n[B]\r\nz=3\r\n")
    );
    assert_eq!(
        put(b"w", b"7"),
        u16le("[A]\r\nx=1\r\ny=2\r\nw=7\r\n[B]\r\nz=3\r\n")
    );
    // `x=\xe4\xc4` writes U+00E4 U+00C4 (ini6 `U16LE high value`).
    assert_eq!(
        put(b"x", b"\xe4\xc4"),
        u16le("[A]\r\nx=\u{e4}\u{c4}\r\ny=2\r\n[B]\r\nz=3\r\n")
    );
    let deleted = removed_key(&file.text, &file.arg(b"A"), &file.arg(b"x")).unwrap();
    assert_eq!(file.encode(&deleted), u16le("[A]\r\ny=2\r\n[B]\r\nz=3\r\n"));
    let deleted = removed_section(&file.text, &file.arg(b"A")).unwrap();
    assert_eq!(file.encode(&deleted), u16le("[B]\r\nz=3\r\n"));
}

/// ini7: a character the code page lacks reads `?` (Windows' best fit is not copied: ini8's
/// U+0100 reads `A` there), the euro reads 0x80; a write of another key keeps them all.
#[test]
fn characters_the_code_page_lacks() {
    let text = "[A]\r\nx=1\r\nj=\u{3042}\r\ne=\u{20ac}\r\n[B\u{3042}]\r\nz=3\r\n";
    let file = Decoded::of(&u16le(text));
    let read = |k: &[u8]| file.answer(&ini_value(&file.text, b"A", k).unwrap());
    assert_eq!(read(b"j"), b"?");
    assert_eq!(read(b"e"), b"\x80");
    assert_eq!(file.answer(&section_names(&file.text)[1]), b"B?");
    let changed = written_profile(&file.text, b"A", b"x", &file.arg(b"5"));
    assert_eq!(
        file.encode(&changed),
        u16le("[A]\r\nx=5\r\nj=\u{3042}\r\ne=\u{20ac}\r\n[B\u{3042}]\r\nz=3\r\n")
    );
    // The euro as an argument byte (0x80) is U+20AC in the file.
    let euro = written_profile(&file.text, b"A", b"x", &file.arg(b"\x80"));
    assert!(file
        .encode(&euro)
        .starts_with(&u16le("[A]\r\nx=\u{20ac}\r\n")));
}

/// ini7 `lone cr`, `lf only`, `no eol trailing spaces`: a UTF-16 file keeps the ANSI line rules.
#[test]
fn a_utf16_file_keeps_the_line_rules() {
    let cr = Decoded::of(&u16le("[A]\rx=1\ry=2\r"));
    assert_eq!(ini_value(&cr.text, b"A", b"y").unwrap(), b"2");
    assert_eq!(
        cr.encode(&written_profile(&cr.text, b"A", b"x", b"5")),
        u16le("[A]\rx=5\ry=2\r\r\n")
    );
    let lf = Decoded::of(&u16le("[A]\nx=1\ny=2\n"));
    assert_eq!(
        lf.encode(&written_profile(&lf.text, b"A", b"x", b"5")),
        u16le("[A]\nx=5\ny=2\n")
    );
    let spaces = Decoded::of(&u16le("[A]\r\nx=1  "));
    assert_eq!(
        spaces.encode(&written_profile(&spaces.text, b"A", b"y", b"2")),
        u16le("[A]\r\nx=1\r\ny=2\r\n")
    );
}

/// What is not such a UTF-16 file: an odd length (ini7 `odd byte count`), the UTF-16 BE mark and
/// UTF-16 without a mark (ini6) are ANSI bytes that are never written; a UTF-8 mark is three ANSI
/// bytes of the first line (ini6 `U8BOM ...`: its first section is not found, and a write of it
/// adds another at the end); a lone surrogate is not text.
#[test]
fn files_that_are_not_utf16_le_text() {
    let mut odd = u16le("[A]\r\nx=1\r\n");
    odd.push(b'A');
    let be: Vec<u8> = [0xfe, 0xff]
        .into_iter()
        .chain(BASE.encode_utf16().flat_map(|u| u.to_be_bytes()))
        .collect();
    let no_mark = u16le(BASE)[2..].to_vec();
    let lone = vec![0xff, 0xfe, 0x00, 0xd8, b'A', 0];
    for bytes in [&odd, &be, &no_mark, &lone] {
        let file = Decoded::of(bytes);
        assert_eq!(file.encoding, Encoding::Ansi);
        assert!(!file.writable);
        assert_eq!(file.text, *bytes);
    }
    let utf8: Vec<u8> = [0xef, 0xbb, 0xbf]
        .iter()
        .chain(BASE.as_bytes())
        .copied()
        .collect();
    let file = Decoded::of(&utf8);
    assert_eq!(file.encoding, Encoding::Ansi);
    assert!(file.writable);
    assert_eq!(ini_value(&file.text, b"A", b"y"), None);
    assert_eq!(ini_value(&file.text, b"B", b"z").unwrap(), b"3");
    let mut want = utf8.clone();
    want.extend_from_slice(b"[A]\r\nx=5\r\n");
    assert_eq!(written_profile(&file.text, b"A", b"x", b"5"), want);
    assert_eq!(removed_key(&file.text, b"A", b"x"), None);
    assert_eq!(removed_section(&file.text, b"A"), None);
    let plain = Decoded::of(BASE.as_bytes());
    assert!(plain.writable && !plain.mark_only());
    assert!(Decoded::of(&[0xff, 0xfe]).mark_only());
}

/// Code page 1252 both ways: every byte's character and back; the five bytes it leaves out map
/// to the same code points.
#[test]
fn code_page_1252_both_ways() {
    for b in 0u8..=255 {
        assert_eq!(cp1252_byte(cp1252_char(b)), b, "{b:#04x}");
    }
    assert_eq!(cp1252_char(0x80), '\u{20ac}');
    assert_eq!(cp1252_char(0x9f), '\u{178}');
    assert_eq!(cp1252_char(0x81), '\u{81}');
    assert_eq!(cp1252_byte('\u{3042}'), b'?');
    assert_eq!(cp1252_byte('\u{100}'), b'?');
}
