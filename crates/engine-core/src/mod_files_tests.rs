//! Tests of [`super`]: the INI write against Windows' bytes, measured on scratch files
//! (2026-11-16, `build43-scratch/measure/mcm/ini-measure`), and the seam without a platform layer.

use super::*;

fn w(old: &[u8], section: &str, key: &str, value: &str) -> Vec<u8> {
    written_profile(old, section.as_bytes(), key.as_bytes(), value.as_bytes())
}

/// Windows' bytes, case by case (`ini-2026-11-16.txt`, `ini2-2026-11-16.txt`).
#[test]
fn writes_make_windows_bytes() {
    // A new file, then more writes: keys in order, the key's first spelling kept.
    let mut f = w(b"", "Messages", "Initial", "1.0000");
    assert_eq!(f, b"[Messages]\r\nInitial=1.0000\r\n");
    for (s, k, v) in [
        ("Messages", "Tutorial", "0.0000"),
        ("List", "MaxWidth", "350.0000"),
        ("messages", "initial", "2.0000"),
    ] {
        f = w(&f, s, k, v);
    }
    assert_eq!(
        f,
        b"[Messages]\r\nInitial=2.0000\r\nTutorial=0.0000\r\n[List]\r\nMaxWidth=350.0000\r\n"
    );
    // MCM's new file: one backspace byte, gone at the first write.
    assert_eq!(
        w(b"\x08", "Messages", "Initial", "1.0000"),
        b"[Messages]\r\nInitial=1.0000\r\n"
    );
    // The shipped MCM.ini: a replaced value takes its comment with it; new keys after the
    // section's last key, before its blank line; a new section at the end.
    let mut f = b"[List]\r\nMaxWidth=350\t;Max width of mod list.\r\n\r\n[Messages]\r\nInitial=1\t;Initial message\r\nTutorial=1\t;Tutorial\r\n".to_vec();
    for (s, k, v) in [
        ("List", "MaxWidth", "200.0000"),
        ("Messages", "New", "3.0000"),
        ("List", "Added", "4.0000"),
        ("Other", "K", "5.0000"),
    ] {
        f = w(&f, s, k, v);
    }
    assert_eq!(
        f,
        b"[List]\r\nMaxWidth=200.0000\r\nAdded=4.0000\r\n\r\n[Messages]\r\nInitial=1\t;Initial message\r\nTutorial=1\t;Tutorial\r\nNew=3.0000\r\n[Other]\r\nK=5.0000\r\n".to_vec()
    );
    // Each case one write from a measured start to Windows' bytes; the chains (no final line
    // end, spaced key, LF-only file, two backspace writes) measured one write at a time too
    // (`ini3-2026-11-16.txt`).
    let cases: &[(&[u8], (&str, &str, &str), &[u8])] = &[
        (b"\x08", ("B", "y", "2"), b"[B]\r\ny=2\r\n"),
        (b"[B]\r\ny=2\r\n", ("B", "z", "3"), b"[B]\r\ny=2\r\nz=3\r\n"),
        (b"[A]\r\nx=1", ("A", "y", "2"), b"[A]\r\nx=1\r\ny=2\r\n"),
        (
            b"[A]\r\nx=1\r\ny=2\r\n",
            ("B", "z", "3"),
            b"[A]\r\nx=1\r\ny=2\r\n[B]\r\nz=3\r\n",
        ),
        (
            b"[ A ]\r\n  x  =  1  \r\nY = 2\r\n",
            ("A", "X", "9"),
            b"[ A ]\r\n  x  =9  \r\nY = 2\r\n",
        ),
        (
            b"[ A ]\r\n  x  =9  \r\nY = 2\r\n",
            ("a", "y", "8"),
            b"[ A ]\r\n  x  =9  \r\nY =8\r\n",
        ),
        (
            b"[A]\nx=1\n\n[B]\ny=2\n",
            ("A", "z", "3"),
            b"[A]\nx=1\nz=3\r\n\n[B]\ny=2\n",
        ),
        (
            b"[A]\nx=1\nz=3\r\n\n[B]\ny=2\n",
            ("A", "x", "4"),
            b"[A]\nx=4\nz=3\r\n\n[B]\ny=2\n",
        ),
        (
            b"[A]\r\nx=1\r\n\r\n\r\n[B]\r\ny=2\r\n",
            ("A", "z", "3"),
            b"[A]\r\nx=1\r\nz=3\r\n\r\n\r\n[B]\r\ny=2\r\n",
        ),
        (
            b"[A]\r\nx=1\r\n[A]\r\nx=2\r\n",
            ("A", "x", "7"),
            b"[A]\r\nx=7\r\n[A]\r\nx=2\r\n",
        ),
        (
            b"[A]\r\n;x=1\r\nx=2 ; note\r\n",
            ("A", "x", "5"),
            b"[A]\r\n;x=1\r\nx=5\r\n",
        ),
        (
            b"\x08\r\n[A]\r\nx=1\r\n",
            ("A", "y", "2"),
            b"\x08\r\n[A]\r\nx=1\r\ny=2\r\n",
        ),
        (
            b"\x08\r\n[A]\r\nx=1\r\n",
            ("B", "y", "2"),
            b"\x08\r\n[A]\r\nx=1\r\n[B]\r\ny=2\r\n",
        ),
        (b"\x08\x08", ("B", "y", "2"), b"[B]\r\ny=2\r\n"),
        (b" ", ("B", "y", "2"), b"[B]\r\ny=2\r\n"),
        (b"x", ("B", "y", "2"), b"x\r\n[B]\r\ny=2\r\n"),
        (b"\r\n", ("B", "y", "2"), b"\r\n[B]\r\ny=2\r\n"),
        (
            b"[A]\r\nx=1\r\n; c\r\n\r\n[B]\r\n",
            ("A", "y", "2"),
            b"[A]\r\nx=1\r\ny=2\r\n; c\r\n\r\n[B]\r\n",
        ),
        (
            b"[A]\r\nx=1\r\n\r\nw=0\r\n\r\n[B]\r\n",
            ("A", "y", "2"),
            b"[A]\r\nx=1\r\n\r\nw=0\r\ny=2\r\n\r\n[B]\r\n",
        ),
        (
            b"[A]\r\nx=1\r\njunk\r\n[B]\r\n",
            ("A", "y", "2"),
            b"[A]\r\nx=1\r\ny=2\r\njunk\r\n[B]\r\n",
        ),
        (b"[A]", ("A", "y", "2"), b"[A]\r\ny=2\r\n"),
        (b"[A]\r\n", ("A", "y", "2"), b"[A]\r\ny=2\r\n"),
        (
            b"[A]\r\nx=\t1\t;c\t\r\n",
            ("A", "x", "5"),
            b"[A]\r\nx=5\t\r\n",
        ),
        (b"[A]\r\nx=\r\n", ("A", "x", "5"), b"[A]\r\nx=5\r\n"),
        (b"[A]\r\nx=\"1\"\r\n", ("A", "x", "5"), b"[A]\r\nx=5\r\n"),
        (b"", ("A", "x", " 5 "), b"[A]\r\nx= 5 \r\n"),
        (
            b"x=1\r\n[A]\r\ny=2\r\n",
            ("A", "x", "5"),
            b"x=1\r\n[A]\r\ny=2\r\nx=5\r\n",
        ),
        (
            b"[A]\r\nx=1\r\n\r\n\r\n",
            ("B", "y", "2"),
            b"[A]\r\nx=1\r\n\r\n\r\n[B]\r\ny=2\r\n",
        ),
        (b"[A]\nx=1\n", ("B", "y", "2"), b"[A]\nx=1\n[B]\r\ny=2\r\n"),
        (b"[A]\rx=1\r", ("A", "y", "2"), b"[A]\rx=1\r\r\ny=2\r\n"),
        (
            b"[A]\r\nx=1\r\n;c=2\r\n[B]\r\n",
            ("A", "y", "2"),
            b"[A]\r\nx=1\r\ny=2\r\n;c=2\r\n[B]\r\n",
        ),
        (
            b"[A]\r\n;x=1\r\n[B]\r\n",
            ("A", ";x", "2"),
            b"[A]\r\n;x=2\r\n;x=1\r\n[B]\r\n",
        ),
        (
            b"[A]\r\nx=1\r\n\t",
            ("B", "y", "2"),
            b"[A]\r\nx=1\r\n[B]\r\ny=2\r\n",
        ),
        (
            b"[A]\r\nx=1\r\n\r",
            ("B", "y", "2"),
            b"[A]\r\nx=1\r\n\r\r\n[B]\r\ny=2\r\n",
        ),
        (
            b"[A]\r\n=1\r\nx=1\r\n",
            ("A", "", "2"),
            b"[A]\r\n=2\r\nx=1\r\n",
        ),
    ];
    for &(old, (s, k, v), want) in cases {
        assert_eq!(
            w(old, s, k, v),
            want,
            "{:?} [{s}] {k}={v}",
            String::from_utf8_lossy(old)
        );
    }
}

/// A CR not followed by LF ends a line; spaces, tabs and backspaces at the very end of the file go
/// before it is split (`ini-measure/ini-cr-2026-11-16.txt`, every case of it).
#[test]
fn writes_split_at_lone_crs_and_trim_the_files_end() {
    let cases: &[(&[u8], (&str, &str, &str), &[u8])] = &[
        (b"[A]\rx=1\ry=2\r", ("A", "x", "5"), b"[A]\rx=5\ry=2\r\r\n"),
        (b"[A]\rx=1\ry=2\r", ("A", "y", "5"), b"[A]\rx=1\ry=5\r\r\n"),
        (
            b"[A]\rx=1\ry=2\r",
            ("A", "z", "3"),
            b"[A]\rx=1\ry=2\r\r\nz=3\r\n",
        ),
        (
            b"[A]\rx=1\r[B]\ry=2\r",
            ("B", "y", "7"),
            b"[A]\rx=1\r[B]\ry=7\r\r\n",
        ),
        (
            b"[A]\rx=1\r[B]\ry=2\r",
            ("A", "q", "1"),
            b"[A]\rx=1\rq=1\r\n[B]\ry=2\r\r\n",
        ),
        (
            b"[A]\rx=1\r",
            ("C", "q", "1"),
            b"[A]\rx=1\r\r\n[C]\r\nq=1\r\n",
        ),
        (
            b"[A]\rx=1  \ry=2\r",
            ("A", "x", "5"),
            b"[A]\rx=5  \ry=2\r\r\n",
        ),
        (
            b"[A]\r\nx=1\r\r\ny=2\r\n",
            ("A", "y", "9"),
            b"[A]\r\nx=1\r\r\ny=9\r\n",
        ),
        (b"[A]\nx=1\n\ry=2\n", ("A", "x", "4"), b"[A]\nx=4\n\ry=2\n"),
        (b"[A]\r\nx=1  ", ("A", "y", "2"), b"[A]\r\nx=1\r\ny=2\r\n"),
        (
            b"[A]\r\nx=1 \x08",
            ("A", "y", "2"),
            b"[A]\r\nx=1\r\ny=2\r\n",
        ),
        (b"[A]\r\nx=1\t", ("A", "y", "2"), b"[A]\r\nx=1\r\ny=2\r\n"),
        (
            b"[A]\r\nx=1\r\ny=2  ",
            ("A", "y", "5"),
            b"[A]\r\nx=1\r\ny=5\r\n",
        ),
        (
            b"[A]\r\nx=1\r\ny=2  ",
            ("A", "x", "5"),
            b"[A]\r\nx=5\r\ny=2\r\n",
        ),
        (b"[A]\r\n  x=1  ", ("A", "x", "3"), b"[A]\r\n  x=3\r\n"),
        (
            b"[A]\r\nx=1 \r",
            ("A", "y", "2"),
            b"[A]\r\nx=1 \r\r\ny=2\r\n",
        ),
        (
            b"[A]\r\nx=1\r ",
            ("A", "y", "2"),
            b"[A]\r\nx=1\r\r\ny=2\r\n",
        ),
        (b"[A]  ", ("A", "y", "2"), b"[A]\r\ny=2\r\n"),
        (
            b"[A]\r\nx=1  ",
            ("B", "y", "2"),
            b"[A]\r\nx=1\r\n[B]\r\ny=2\r\n",
        ),
    ];
    for &(old, (s, k, v), want) in cases {
        assert_eq!(
            String::from_utf8_lossy(&w(old, s, k, v)),
            String::from_utf8_lossy(want),
            "{:?} [{s}] {k}={v}",
            String::from_utf8_lossy(old)
        );
    }
}

/// A read ends lines at CR alone too (`jip-cmds/files-measure/ini4-2026-11-16.txt`, `STRING cr
/// only`; `ini9-2026-11-16.txt`, `cr only two sections`: A/y none, B/y 2, A/x 1, and `cr cr lf`;
/// `ini5-...`, `SECTION cr cr lf` and `lf cr`, `STRING trailing spaces no eol`).
#[test]
fn a_read_ends_lines_at_a_lone_cr() {
    let cr_only = b"[A]\rx=1\ry=2\r";
    assert_eq!(ini_value(cr_only, b"A", b"y").unwrap(), b"2");
    assert_eq!(ini_value(cr_only, b"A", b"x").unwrap(), b"1");
    assert_eq!(ini_value(b"[A]\rx=1\r[B]\ry=2\r", b"A", b"y"), None);
    assert_eq!(
        ini_value(b"[A]\rx=1\r[B]\ry=2\r", b"B", b"y").unwrap(),
        b"2"
    );
    for text in [&b"[A]\r\nx=1\r\r\ny=2\r\n"[..], b"[A]\nx=1\n\ry=2\n"] {
        assert_eq!(ini_value(text, b"A", b"x").unwrap(), b"1");
        assert_eq!(ini_value(text, b"A", b"y").unwrap(), b"2");
    }
    assert_eq!(ini_value(b"[A]\r\nx=1  ", b"A", b"x").unwrap(), b"1");
    assert_eq!(ini_value(b"[A]\r\r\nx=1\r\r\n", b"A", b"x").unwrap(), b"1");
}

/// The four cases REVIEW-642 named, against the measurements: a CR-only file read and changed in
/// place (`ini-measure/ini-cr-2026-11-16.txt`; `jip-cmds/files-measure/ini4-2026-11-16.txt`,
/// `STRING cr only`), and blanks at the end of a file with no last line end trimmed before a
/// change (`ini5-2026-11-16.txt`, `trailing spaces no eol`), the section heading's too.
#[test]
fn the_reviewed_cases_read_and_write_as_measured() {
    let cr_only = b"[A]\rx=1\ry=2\r";
    assert_eq!(ini_value(cr_only, b"A", b"y").unwrap(), b"2");
    assert_eq!(
        written_profile(cr_only, b"A", b"x", b"5"),
        b"[A]\rx=5\ry=2\r\r\n"
    );
    assert_eq!(
        written_profile(b"[A]\r\n  x=1  ", b"A", b"x", b"3"),
        b"[A]\r\n  x=3\r\n"
    );
    assert_eq!(
        written_profile(b"[A]  ", b"A", b"y", b"2"),
        b"[A]\r\ny=2\r\n"
    );
    assert_eq!(
        written_profile(b"[A]\r\nx=1  ", b"A", b"y", b"2"),
        b"[A]\r\nx=1\r\ny=2\r\n"
    );
}

/// A new heading and a new key are written without the spaces around their names
/// (`jip-cmds/files-measure/ini6-2026-11-16.txt`, `PUT spaced section arg new` and `PUT spaced
/// key arg new`); the spaced names find their section and key (`STRING spaced ...`).
#[test]
fn spaced_names_are_trimmed_when_written() {
    let text = b"[A]\r\nx=1\r\ny=2\r\n[B]\r\nz=3\r\n";
    assert_eq!(
        written_profile(text, b" C ", b"k", b"v"),
        b"[A]\r\nx=1\r\ny=2\r\n[B]\r\nz=3\r\n[C]\r\nk=v\r\n"
    );
    assert_eq!(
        written_profile(text, b"A", b" w ", b"v"),
        b"[A]\r\nx=1\r\ny=2\r\nw=v\r\n[B]\r\nz=3\r\n"
    );
    assert_eq!(ini_value(text, b" A ", b"x").unwrap(), b"1");
    assert_eq!(ini_value(text, b"A", b" x ").unwrap(), b"1");
    // A form feed before a heading or a key (`STRING form feed heading`, `... before key`).
    assert_eq!(ini_value(b"\x0c[A]\r\nx=1\r\n", b"A", b"x").unwrap(), b"1");
    assert_eq!(ini_value(b"[A]\r\n\x0cx=1\r\n", b"A", b"x").unwrap(), b"1");
}

/// A comment after a value is part of what a read gives (measured: `350\t;Max width...`).
#[test]
fn a_read_keeps_a_comment_after_the_value() {
    let text = b"[List]\r\nMaxWidth=350\t;Max width of mod list.\r\n";
    assert_eq!(
        ini_value(text, b"List", b"MaxWidth").unwrap(),
        b"350\t;Max width of mod list."
    );
}

/// Without the platform layer (this crate's tests install none) nothing is read or written.
#[test]
fn without_the_platform_nothing_is_read_or_written() {
    let mut e = Engine::new();
    assert_eq!(read(&e, br"Data\Config\MCM.ini"), None);
    assert!(!exists(&e, br"Data\Config\MCM.ini"));
    assert!(!write(&mut e, br"Data\Config\MCM.ini", b"x"));
    assert_eq!(
        write_profile_string(&mut e, b"MCM.ini", b"A", b"x", b"1"),
        Err(ERROR_NOT_SUPPORTED)
    );
    assert_eq!(read_or_error(&e, b"MCM.ini"), Err(ERROR_NOT_SUPPORTED));
    assert_eq!(
        profile_string(&e, br"Data\Config\MCM.ini", b"A", b"x"),
        None
    );
}
