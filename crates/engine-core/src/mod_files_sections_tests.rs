//! Tests of [`super`] against Windows' bytes, measured on scratch files (2026-11-16,
//! `build43-scratch/measure/jip-cmds/files-measure`, `ini3-`, `ini4-` and `ini5-2026-11-16.txt`):
//! every case of those runs, as `(file before, call, file after)`.

use super::*;

fn list(items: &[&str]) -> Vec<Vec<u8>> {
    items.iter().map(|s| s.as_bytes().to_vec()).collect()
}

/// `GetPrivateProfileSectionA` (buffer 0x1000): the entries, spaces and tabs off, comments and
/// blank lines left out, `#` lines and lines without `=` kept, the first section only.
#[test]
fn a_section_reads_as_windows_reads_it() {
    let mixed = b"[A]\r\nx=1\r\n; c\r\n\r\n  y  =  2  \r\njunk\r\nz=\"q\"\r\n# h\r\n=5\r\nk=\r\n\tt\t=\t3\t\r\n  ;s=1\r\n[B]\r\nw=3\r\n";
    let cases: &[(&[u8], &str, Option<&[&str]>)] = &[
        (
            mixed,
            "A",
            Some(&["x=1", "y=2", "junk", "z=\"q\"", "# h", "=5", "k=", "t=3"]),
        ),
        (mixed, "b", Some(&["w=3"])),
        (mixed, "C", None),
        (b"", "A", None),
        (
            b"[A]\r\nx=1\r\n[B]\r\nv=0\r\n[A]\r\ny=2\r\n",
            "A",
            Some(&["x=1"]),
        ),
        (b"[A]\nx=1\ny=2\n[B]\n", "A", Some(&["x=1", "y=2"])),
        (b"[ a ]\r\nx=1\r\n", "A", Some(&["x=1"])),
        (b"[A\r\nx=1\r\n", "A", Some(&["x=1"])),
        (b"[A] junk\r\nx=1\r\n", "A", Some(&["x=1"])),
        (b"[A]\r\n[B]\r\nx=1\r\n", "A", Some(&[])),
        (b"[A]\r\nx=1", "A", Some(&["x=1"])),
        (
            b"[A]\r\nx=\"a b\"\r\ny= \"c\" \r\nz=\"d\r\n",
            "A",
            Some(&["x=\"a b\"", "y=\"c\"", "z=\"d"]),
        ),
        (b"[A]\rx=1\ry=2\r", "A", Some(&["x=1", "y=2"])),
        (b"[A]\r\n  junk  \r\n\tx\t\r\n", "A", Some(&["junk", "x"])),
        (b"[A]\r\nk = a b \r\n", "A", Some(&["k=a b"])),
        (b"[A]\r\nx=1\r\r\ny=2\r\n", "A", Some(&["x=1", "y=2"])),
        (b"[A]\nx=1\n\ry=2\n", "A", Some(&["x=1", "y=2"])),
    ];
    for (text, section, want) in cases {
        assert_eq!(
            section_lines(text, section.as_bytes()),
            want.map(list),
            "{:?} [{section}]",
            String::from_utf8_lossy(text)
        );
    }
}

/// `GetPrivateProfileSectionNamesA`: every heading, trimmed, repeats and `[]` included.
#[test]
fn section_names_read_as_windows_reads_them() {
    let cases: &[(&[u8], &[&str])] = &[
        (
            b"[A]\r\n[ B ]\r\n[C] junk\r\n[D\r\n;[E]\r\n  [F]\r\n[]\r\n[A]\r\nx=1\r\n",
            &["A", "B", "C", "D", "F", "", "A"],
        ),
        (b"x=1\r\n", &[]),
        (b"[A]\n[B]\n", &["A", "B"]),
        (b"[ a b ]\r\n[c]d]\r\n", &["a b", "c"]),
        (b"[A]\r[B]\r", &["A", "B"]),
    ];
    for (text, want) in cases {
        assert_eq!(section_names(text), list(want));
    }
}

/// The buffer fill: the list and a 0 when it fits with a byte to spare, else the first
/// `size - 2` bytes and two 0s (`trunc` cases, sections and names).
#[test]
fn a_list_fills_a_buffer_as_windows_fills_it() {
    let section = list(&["abc=123", "def=456"]);
    let cases: &[(usize, &[u8], usize)] = &[
        (1, b"\0", 0),
        (2, b"\0\0", 0),
        (3, b"a\0\0", 1),
        (5, b"abc\0\0", 3),
        (7, b"abc=1\0\0", 5),
        (8, b"abc=12\0\0", 6),
        (9, b"abc=123\0\0", 7),
        (10, b"abc=123\0\0\0", 8),
        (12, b"abc=123\0de\0\0", 10),
        (15, b"abc=123\0def=4\0\0", 13),
        (16, b"abc=123\0def=45\0\0", 14),
        (17, b"abc=123\0def=456\0\0", 15),
        (18, b"abc=123\0def=456\0\0", 16),
        (0x1000, b"abc=123\0def=456\0\0", 16),
    ];
    for &(size, buf, count) in cases {
        assert_eq!(
            profile_list(&section, size),
            (buf.to_vec(), count),
            "{size}"
        );
    }
    let names = list(&["A", "BB", "CCC"]);
    let cases: &[(usize, &[u8], usize)] = &[
        (1, b"\0", 0),
        (2, b"\0\0", 0),
        (3, b"A\0\0", 1),
        (4, b"A\0\0\0", 2),
        (5, b"A\0B\0\0", 3),
        (6, b"A\0BB\0\0", 4),
        (7, b"A\0BB\0\0\0", 5),
        (8, b"A\0BB\0C\0\0", 6),
        (9, b"A\0BB\0CC\0\0", 7),
        (10, b"A\0BB\0CCC\0\0", 8),
        (11, b"A\0BB\0CCC\0\0", 9),
    ];
    for &(size, buf, count) in cases {
        assert_eq!(profile_list(&names, size), (buf.to_vec(), count), "{size}");
    }
    // An empty section: one 0, counted 0.
    assert_eq!(profile_list(&[], 0x1000), (vec![0], 0));
    assert_eq!(profile_list(&[], 0), (vec![], 0));
}

/// `WritePrivateProfileStringA(section, key, NULL)`: `None` is a file Windows left as it was.
#[test]
fn a_key_is_removed_as_windows_removes_it() {
    let two: &[u8] = b"[A]\r\nx=1\r\ny=2\r\n[B]\r\nx=3\r\n";
    let cases: &[(&[u8], &str, &str, Option<&[u8]>)] = &[
        (two, "A", "x", Some(b"[A]\r\ny=2\r\n[B]\r\nx=3\r\n")),
        (two, "A", "y", Some(b"[A]\r\nx=1\r\n[B]\r\nx=3\r\n")),
        (two, "a", "Y", Some(b"[A]\r\nx=1\r\n[B]\r\nx=3\r\n")),
        (two, "A", "q", None),
        (two, "C", "x", None),
        (
            b"[A]\r\nx=1\r\n[B]\r\ny=2\r\n",
            "A",
            "x",
            Some(b"[A]\r\n[B]\r\ny=2\r\n"),
        ),
        (b"[A]\r\nx=1\r\nx=2\r\n", "A", "x", Some(b"[A]\r\nx=2\r\n")),
        (
            b"[A]\r\n  x  = 1\r\ny=2\r\n",
            "A",
            "x",
            Some(b"[A]\r\n  y=2\r\n"),
        ),
        (b"[A]\nx=1\ny=2\n", "A", "x", Some(b"[A]\ny=2\n")),
        (b"[A]\r\nx=1\r\ny=2", "A", "y", Some(b"[A]\r\nx=1\r\n")),
        (b"[A]\r\n;x=1\r\nx=2\r\n", "A", ";x", None),
        (b"[A]\r\ny=0\r\n[B]\r\n[A]\r\nx=1\r\n", "A", "x", None),
        (b"[A]\r\nx=1\r\n\x08", "A", "q", None),
        (b"[A]\r\nx=1\r\n\x08", "A", "x", Some(b"[A]\r\n")),
        (b"[A]\r\nx=1", "A", "q", Some(b"[A]\r\nx=1\r\n")),
        (b"[A]\rx=1\ry=2\r", "A", "x", Some(b"[A]\ry=2\r\r\n")),
        (
            b"[A]\r\n\tx=1\r\ny=2\r\n",
            "A",
            "x",
            Some(b"[A]\r\n\ty=2\r\n"),
        ),
        (
            b"[A]\r\njunk\r\nx=1\r\n",
            "A",
            "x",
            Some(b"[A]\r\njunk\r\n"),
        ),
        (b"[A]\r\njunk\r\nx=1\r\n", "A", "junk", None),
        (b"[A]\r\nx=1\r\n ", "A", "q", None),
        (b"[A]\r\nx=1\r\n\t", "A", "q", None),
        (b"[A]\r\nx=1\r\n\x08\x08", "A", "q", None),
        (b"[A]\r\nx=1", "B", "q", Some(b"[A]\r\nx=1\r\n")),
        (
            b"[A]\r\nx=1 ;c\r\ny=2\r\n",
            "A",
            "x",
            Some(b"[A]\r\ny=2\r\n"),
        ),
        (b"[A]\r\n  x=1", "A", "x", Some(b"[A]\r\n  ")),
        (
            b"[A]\r\nx=1   \r\ny=2\r\n",
            "A",
            "x",
            Some(b"[A]\r\ny=2\r\n"),
        ),
        (b"[A]\r\n=1\r\nx=2\r\n", "A", "", None),
        (b"[A]\r\nx=\r\ny=2\r\n", "A", "x", Some(b"[A]\r\ny=2\r\n")),
        (b"[A]\r\nx=1  ", "A", "q", Some(b"[A]\r\nx=1\r\n")),
        (b"[A]\r\nx=1 \x08", "A", "q", Some(b"[A]\r\nx=1\r\n")),
        (b"[A]\r\nx=1\r\ny=2  ", "A", "x", Some(b"[A]\r\ny=2\r\n")),
        (b"[A]\r\nx=1\r\n\r\n", "A", "x", Some(b"[A]\r\n\r\n")),
        (
            b"[A]\r\nx=1\r\r\ny=2\r\n",
            "A",
            "x",
            Some(b"[A]\r\n\r\ny=2\r\n"),
        ),
        (b"[A]\nx=1\n\ry=2\n", "A", "x", Some(b"[A]\n\ry=2\n")),
    ];
    for (old, section, key, want) in cases {
        assert_eq!(
            removed_key(old, section.as_bytes(), key.as_bytes()).as_deref(),
            *want,
            "{:?} [{section}] {key}",
            String::from_utf8_lossy(old)
        );
    }
}

/// `WritePrivateProfileStructA(section, NULL, NULL, 0)` (the same bytes as
/// `WritePrivateProfileStringA(section, NULL, NULL)`, measured side by side).
#[test]
fn a_section_is_removed_as_windows_removes_it() {
    let three: &[u8] = b"[A]\r\nx=1\r\n;c\r\n\r\n[B]\r\ny=2\r\n\r\n[C]\r\nz=3\r\n";
    let cases: &[(&[u8], &str, Option<&[u8]>)] = &[
        (
            three,
            "A",
            Some(b";c\r\n\r\n[B]\r\ny=2\r\n\r\n[C]\r\nz=3\r\n"),
        ),
        (
            three,
            "b",
            Some(b"[A]\r\nx=1\r\n;c\r\n\r\n\r\n[C]\r\nz=3\r\n"),
        ),
        (
            three,
            "C",
            Some(b"[A]\r\nx=1\r\n;c\r\n\r\n[B]\r\ny=2\r\n\r\n"),
        ),
        (three, "D", None),
        (
            b"[A]\r\nx=1\r\n[B]\r\n[A]\r\ny=2\r\n",
            "A",
            Some(b"[B]\r\n[A]\r\ny=2\r\n"),
        ),
        (b"[A]\nx=1\n[B]\ny=2\n", "A", Some(b"[B]\ny=2\n")),
        (
            b"top=1\r\n[A]\r\nx=1\r\n[B]\r\n",
            "A",
            Some(b"top=1\r\n[B]\r\n"),
        ),
        (b"[A]\r\nx=1\r\n[B]\r\ny=2", "B", Some(b"[A]\r\nx=1\r\n")),
        (b"[A]\r\nx=1\r\n\x08", "A", Some(b"")),
        (b"[ a ]\r\nx=1\r\n[B]\r\n", "A", Some(b"[B]\r\n")),
        (
            b"[A]\r\nx=1\r\n;c\r\ny=2\r\n\r\n[B]\r\n",
            "A",
            Some(b"\r\n[B]\r\n"),
        ),
        (
            b"[A]\r\nx=1\r\njunk\r\n\r\n[B]\r\n",
            "A",
            Some(b"\r\n[B]\r\n"),
        ),
        (
            b"[A]\r\nx=1\r\njunk\r\ny=2\r\n[B]\r\n",
            "A",
            Some(b"[B]\r\n"),
        ),
        (
            b"[A]\r\nx=1\r\n# h\r\n\r\n[B]\r\n",
            "A",
            Some(b"\r\n[B]\r\n"),
        ),
        (b"[A]\r\n;c\r\n\r\n[B]\r\n", "A", Some(b";c\r\n\r\n[B]\r\n")),
        (b"  [A]  \r\nx=1\r\n[B]\r\n", "A", Some(b"  [B]\r\n")),
        (
            b"[A]\r\n  x=1  \r\n  ;c\r\n[B]\r\n",
            "A",
            Some(b"  ;c\r\n[B]\r\n"),
        ),
        (b"[A]\r\nx=1\r\n\r\n\r\n", "A", Some(b"")),
        (b"[A]\r\njunk\r\n[B]\r\n", "A", Some(b"[B]\r\n")),
        (b"[A]\r\nx=1", "B", Some(b"[A]\r\nx=1\r\n")),
        (b"[A]\r\nx=1\r\n\x08", "B", None),
        (b"[A]\rx=1\r[B]\ry=2\r", "A", Some(b"[B]\ry=2\r\r\n")),
        (b"[A]\r\n=1\r\n;c\r\n[B]\r\n", "A", Some(b";c\r\n[B]\r\n")),
        (b"[A]\r\nx=1\r\n;c\r\n", "A", Some(b";c\r\n")),
        (b"[A]\r\nx=1\r\n;c\r\n\r\n", "A", Some(b"")),
        (b"[A]\r\nx=1\r\n\r\n;c\r\n", "A", Some(b"\r\n;c\r\n")),
        (b"[A]\r\n\r\n", "A", Some(b"")),
        (b"[A]\r\n;c\r\n", "A", Some(b";c\r\n")),
        (b"[A]\r\nx=1\r\n  \r\n\t\r\n", "A", Some(b"")),
        (
            b"[Z]\r\nq=1\r\n[A]\r\nx=1\r\n\r\n",
            "A",
            Some(b"[Z]\r\nq=1\r\n"),
        ),
    ];
    for (old, section, want) in cases {
        assert_eq!(
            removed_section(old, section.as_bytes()).as_deref(),
            *want,
            "{:?} [{section}]",
            String::from_utf8_lossy(old)
        );
    }
}

/// `WritePrivateProfileSectionA`: the section's key lines replaced, entries without `=` and
/// comments after the last key kept, a new section at the end.
#[test]
fn a_section_is_written_as_windows_writes_it() {
    let three: &[u8] = b"[A]\r\nx=1\r\n;c\r\n\r\n[B]\r\ny=2\r\n\r\n[C]\r\nz=3\r\n";
    let ab = list(&["a=1", "b=2"]);
    let a = list(&["a=1"]);
    let cases: &[(&[u8], &str, &[Vec<u8>], &[u8])] = &[
        (
            three,
            "A",
            &ab,
            b"[A]\r\na=1\r\nb=2\r\n;c\r\n\r\n[B]\r\ny=2\r\n\r\n[C]\r\nz=3\r\n",
        ),
        (
            three,
            "B",
            &ab,
            b"[A]\r\nx=1\r\n;c\r\n\r\n[B]\r\na=1\r\nb=2\r\n\r\n[C]\r\nz=3\r\n",
        ),
        (
            three,
            "C",
            &ab,
            b"[A]\r\nx=1\r\n;c\r\n\r\n[B]\r\ny=2\r\n\r\n[C]\r\na=1\r\nb=2\r\n",
        ),
        (
            three,
            "D",
            &ab,
            b"[A]\r\nx=1\r\n;c\r\n\r\n[B]\r\ny=2\r\n\r\n[C]\r\nz=3\r\n[D]\r\na=1\r\nb=2\r\n",
        ),
        (b"", "A", &ab, b"[A]\r\na=1\r\nb=2\r\n"),
        (
            b"[A]\r\nx=1\r\n[B]\r\n[A]\r\ny=2\r\n",
            "A",
            &ab,
            b"[A]\r\na=1\r\nb=2\r\n[B]\r\n[A]\r\ny=2\r\n",
        ),
        (
            b"[A]\r\nx=1\r\n[B]\r\ny=2",
            "B",
            &ab,
            b"[A]\r\nx=1\r\n[B]\r\na=1\r\nb=2\r\n",
        ),
        (
            b"[A]\r\nx=1",
            "B",
            &ab,
            b"[A]\r\nx=1\r\n[B]\r\na=1\r\nb=2\r\n",
        ),
        (
            b"[A]\r\nx=1\r\n",
            "A",
            &list(&[" a = 1 "]),
            b"[A]\r\n a = 1 \r\n",
        ),
        (
            b"[A]\r\nx=1\r\n",
            "A",
            &list(&["junk", "b=2"]),
            b"[A]\r\njunk\r\nb=2\r\n",
        ),
        (b"[A]\nx=1\n[B]\n", "A", &ab, b"[A]\na=1\r\nb=2\r\n[B]\n"),
        (b"[A]\r\n[B]\r\n", "A", &ab, b"[A]\r\na=1\r\nb=2\r\n[B]\r\n"),
        (b"[a]\r\nx=1\r\n", "A", &ab, b"[a]\r\na=1\r\nb=2\r\n"),
        (b"\x08", "A", &ab, b"[A]\r\na=1\r\nb=2\r\n"),
        (b"[A]\r\nx=1\r\n", "A", &[], b"[A]\r\n"),
        (
            b"[A]\r\n;c\r\n\r\n[B]\r\n",
            "A",
            &ab,
            b"[A]\r\na=1\r\nb=2\r\n;c\r\n\r\n[B]\r\n",
        ),
        (
            b"[A]\r\nx=1\r\n;c\r\ny=2\r\n\r\n[B]\r\n",
            "A",
            &a,
            b"[A]\r\na=1\r\n\r\n[B]\r\n",
        ),
        (
            b"[A]\r\nx=1\r\njunk\r\n\r\n[B]\r\n",
            "A",
            &a,
            b"[A]\r\na=1\r\njunk\r\n\r\n[B]\r\n",
        ),
        (
            b"[A]\r\nx=1\r\njunk\r\ny=2\r\n[B]\r\n",
            "A",
            &a,
            b"[A]\r\na=1\r\n[B]\r\n",
        ),
        (
            b"[A]\r\nx=1\r\n# h\r\n\r\n[B]\r\n",
            "A",
            &a,
            b"[A]\r\na=1\r\n# h\r\n\r\n[B]\r\n",
        ),
        (
            b"  [A]  \r\nx=1\r\n[B]\r\n",
            "A",
            &a,
            b"  [A]  \r\na=1\r\n[B]\r\n",
        ),
        (
            b"[A]\r\n  x=1  \r\n  y=2\r\n[B]\r\n",
            "A",
            &a,
            b"[A]\r\na=1\r\n[B]\r\n",
        ),
        (b"[A]\r\nx=1\r\n\r\n\r\n", "A", &a, b"[A]\r\na=1\r\n"),
        (
            b"[A]\r\njunk\r\n[B]\r\n",
            "A",
            &a,
            b"[A]\r\na=1\r\njunk\r\n[B]\r\n",
        ),
        (
            b"[A]\rx=1\r[B]\ry=2\r",
            "A",
            &a,
            b"[A]\ra=1\r\n[B]\ry=2\r\r\n",
        ),
        (b"[A]", "A", &a, b"[A]\r\na=1\r\n"),
        (b"[A\r\nx=1\r\n", "A", &a, b"[A\r\na=1\r\n"),
        (
            b"[A]\r\n=1\r\n;c\r\n[B]\r\n",
            "A",
            &a,
            b"[A]\r\na=1\r\n;c\r\n[B]\r\n",
        ),
        (
            b"[A]\r\nx=1\r\n",
            "A",
            &list(&["a=1\r\nb=2"]),
            b"[A]\r\na=1\r\nb=2\r\n",
        ),
        (b"[A]\r\nx=1\r\n;c\r\n", "A", &a, b"[A]\r\na=1\r\n;c\r\n"),
        (
            b"[A]\r\nx=1\r\n\r\n;c\r\n",
            "A",
            &a,
            b"[A]\r\na=1\r\n\r\n;c\r\n",
        ),
        (b"[A]\r\n\r\n", "A", &a, b"[A]\r\na=1\r\n"),
        (b"[A]\r\nx=1\r\njunk\r\n\r\n", "A", &a, b"[A]\r\na=1\r\n"),
        (
            b"[A]\r\nx=1\r\n\r\n[B]\r\n",
            "A",
            &a,
            b"[A]\r\na=1\r\n\r\n[B]\r\n",
        ),
        (
            b"[A]\r\nx=1\r\n\r\n",
            "B",
            &a,
            b"[A]\r\nx=1\r\n\r\n[B]\r\na=1\r\n",
        ),
    ];
    for (old, section, lines, want) in cases {
        assert_eq!(
            written_section(old, section.as_bytes(), lines),
            want.to_vec(),
            "{:?} [{section}]",
            String::from_utf8_lossy(old)
        );
    }
}

/// Without the platform layer (this crate's tests install none) nothing is read or written.
#[test]
fn without_the_platform_the_section_calls_do_nothing() {
    let mut e = Engine::new();
    assert_eq!(profile_section(&e, br"Data\Config\x.ini", b"A"), None);
    assert_eq!(profile_section_names(&e, br"Data\Config\x.ini"), None);
    assert!(!remove_profile_key(&mut e, b"x.ini", b"A", b"x"));
    assert!(!remove_profile_section(&mut e, b"x.ini", b"A"));
    assert!(!write_profile_section(
        &mut e,
        b"x.ini",
        b"A",
        &list(&["a=1"])
    ));
}
