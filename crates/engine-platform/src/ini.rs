//! The profile (INI) file imports, `GetPrivateProfileIntA`, `GetPrivateProfileStringA` and
//! `WritePrivateProfileStringA`: they read the files the game made in this run ([`files`]) and
//! the Windows directory of the player's table ([`d3d9::windows_directory`]), so they are in the
//! platform layer rather than the core (moved from `engine-core`'s `units/platform.rs`).

#[allow(unused_imports)]
use crate::prelude::*;

use super::{d3d9, files};

/// ASCII white space taken off both ends (`<[u8]>::trim_ascii` is newer than the MSRV).
trait TrimSpaces {
    fn trim_spaces(&self) -> &Self;
}

impl TrimSpaces for [u8] {
    fn trim_spaces(&self) -> &[u8] {
        let start = self
            .iter()
            .position(|b| !b.is_ascii_whitespace())
            .unwrap_or(self.len());
        let end = self
            .iter()
            .rposition(|b| !b.is_ascii_whitespace())
            .map_or(start, |i| i + 1);
        &self[start..end.max(start)]
    }
}

/// The value of `key` in section `section` of the INI text `text` (names compared without case,
/// spaces around names and values trimmed, one pair of surrounding quotes taken off), as
/// `GetPrivateProfileStringA` finds it: only in the first section of that name, as Windows reads
/// a file with a repeated section name (measured on scratch files, 2026-11-09; the test
/// `only_the_first_section_of_a_name_is_read` holds the answers).
fn ini_value(text: &[u8], section: &[u8], key: &[u8]) -> Option<Vec<u8>> {
    let section = section.trim_spaces();
    let mut inside = false;
    for line in text.split(|&b| b == b'\n') {
        let line = line.trim_spaces();
        if let Some(rest) = line.strip_prefix(b"[") {
            // The first section of the name ends here: Windows reads no later one.
            if inside {
                return None;
            }
            let name = rest.split(|&b| b == b']').next().unwrap_or(rest);
            inside = name.trim_spaces().eq_ignore_ascii_case(section);
            continue;
        }
        if !inside {
            continue;
        }
        let Some(at) = line.iter().position(|&b| b == b'=') else {
            continue;
        };
        if line[..at].trim_spaces().eq_ignore_ascii_case(key) {
            let value = line[at + 1..].trim_spaces();
            let value = match value {
                [b'"', middle @ .., b'"'] => middle,
                _ => value,
            };
            return Some(value.to_vec());
        }
    }
    None
}

/// The text of the profile file `file` names: a name without a full path is in the Windows
/// directory, as the profile functions look for it (`d3d9::windows_directory`, from the
/// player's table); the file as the game made it in this run when it did (`files`), else the
/// player's file, read only. `None` when there is none.
fn profile_text(e: &mut Engine, file: Ptr) -> Option<Vec<u8>> {
    let name = e.mem.cstr(file.addr());
    let path = if files::is_full_path(&name) {
        name
    } else {
        let mut path = d3d9::windows_directory(e);
        path.push(b'\\');
        path.extend_from_slice(&name);
        path
    };
    files::read_whole(e, &path)
}

// Platform: replaces KERNEL32.DLL GetPrivateProfileIntA (import)
/// `GetPrivateProfileIntA(section, key, default, file)`: the key's value as an integer, or
/// `default` when the file, the section or the key isn't there ([`profile_text`]). The value is
/// read as [`profile_int`] does, which follows Windows' answers measured on the player's machine
/// (2026-10-21, 38 values).
pub fn get_private_profile_int_a(
    e: &mut Engine,
    section: Ptr,
    key: Ptr,
    default: u32,
    file: Ptr,
) -> u32 {
    let section = e.mem.cstr(section.addr());
    let key = e.mem.cstr(key.addr());
    let Some(value) = profile_text(e, file).and_then(|text| ini_value(&text, &section, &key))
    else {
        return default;
    };
    profile_int(&value)
}

/// A profile value as Windows' `GetPrivateProfileIntA` reads it (measured): leading spaces and
/// tabs, then one optional `+` or `-`, then an optional lower-case base prefix `0x` (16), `0o`
/// (8) or `0b` (2), else base 10; digits of that base up to the first other character, kept
/// modulo 2^32; a `-` negates the result modulo 2^32. No digits (`-`, `0x`, `0b2`, `x5`) give 0;
/// upper-case prefixes are not prefixes (`0X1F` is 0).
fn profile_int(value: &[u8]) -> u32 {
    let mut rest = value;
    while let [b' ' | b'\t', tail @ ..] = rest {
        rest = tail;
    }
    let negative = match rest {
        [b'-', tail @ ..] => {
            rest = tail;
            true
        }
        [b'+', tail @ ..] => {
            rest = tail;
            false
        }
        _ => false,
    };
    let base = match rest {
        [b'0', b'x', tail @ ..] => {
            rest = tail;
            16
        }
        [b'0', b'o', tail @ ..] => {
            rest = tail;
            8
        }
        [b'0', b'b', tail @ ..] => {
            rest = tail;
            2
        }
        _ => 10,
    };
    let mut n = 0u32;
    for &c in rest {
        let Some(digit) = char::from(c).to_digit(base) else {
            break;
        };
        n = n.wrapping_mul(base).wrapping_add(digit);
    }
    if negative {
        n.wrapping_neg()
    } else {
        n
    }
}

// Platform: replaces KERNEL32.DLL GetPrivateProfileStringA (import)
/// The value of `key` in `section` of the INI file `file` ([`profile_text`]), or
/// `default` (none: empty) when the file, the section or the key isn't there: copied into
/// `buffer`, cut to `size - 1` bytes and zero-terminated; answers the bytes copied. Listing a
/// file's sections or keys (a null section or key) is not provided.
pub fn get_private_profile_string_a(
    e: &mut Engine,
    section: Ptr,
    key: Ptr,
    default: Ptr,
    buffer: Ptr,
    size: u32,
    file: Ptr,
) -> u32 {
    if section.is_null() || key.is_null() {
        panic!("GetPrivateProfileStringA with a null section or key is not provided");
    }
    let section = e.mem.cstr(section.addr());
    let key = e.mem.cstr(key.addr());
    let found = profile_text(e, file).and_then(|text| ini_value(&text, &section, &key));
    let value = match found {
        Some(value) => value,
        None if default.is_null() => Vec::new(),
        None => e.mem.cstr(default.addr()),
    };
    if size == 0 {
        return 0;
    }
    let copied = value.len().min(size as usize - 1);
    e.mem.set_cstr(buffer.addr(), &value[..copied]);
    copied as u32
}

// Platform: replaces KERNEL32.DLL WritePrivateProfileStringA (import)
/// Answers success without writing: the platform never changes the player's files.
pub fn write_private_profile_string_a(
    _e: &mut Engine,
    _section: Ptr,
    _key: Ptr,
    _value: Ptr,
    _file: Ptr,
) -> u32 {
    1
}

#[cfg(test)]
mod tests {
    use super::*;

    fn engine() -> Engine {
        let mut e = Engine::new();
        e.mem.map(0x1000, 0x1000);
        e
    }

    #[test]
    fn ini_values_are_found_by_section_and_key() {
        let text = b"; comment\r\n[General]\r\n sLanguage = ENGLISH \r\n[Controls]\r\nForward=\"0011FF00\"\r\nBack=1F\r\n";
        assert_eq!(
            ini_value(text, b"controls", b"forward").unwrap(),
            b"0011FF00"
        );
        assert_eq!(ini_value(text, b"Controls", b"Back").unwrap(), b"1F");
        assert_eq!(
            ini_value(text, b"General", b"sLanguage").unwrap(),
            b"ENGLISH"
        );
        assert_eq!(ini_value(text, b"General", b"Back"), None);
    }

    /// A `;` line is never found: a key nv-rs's `Fallout.ini` writes commented out
    /// (`distribution/nv-rs-sections.ini`) or a player commented out is not the key, and the
    /// `[nv-rs ...]` sections are sections like any other.
    #[test]
    fn a_commented_key_is_never_found() {
        let text = b"[nv-rs Fixes]\r\n; Uncapped frame rate: a note\r\n;bUncap=1\r\n\
                     [Display]\r\n;iPresentInterval=0\r\niPresentInterval=1\r\n";
        assert_eq!(ini_value(text, b"nv-rs Fixes", b"bUncap"), None);
        assert_eq!(
            ini_value(text, b"Display", b"iPresentInterval").unwrap(),
            b"1"
        );
    }

    /// A file that starts with a UTF-8 byte order mark, as Windows' `GetPrivateProfileStringA`
    /// reads it (measured on scratch files, 2026-11-09, measure/ini/DESIGN.md): a heading glued
    /// to the mark is no heading, so the keys under it are in no section and not found, and the
    /// sections after it read as usual. Play.exe puts nv-rs's block at the end of such a file
    /// (scripts/launcher `game_ini::with_sections`), where it changes none of the player's keys.
    #[test]
    fn a_heading_glued_to_a_byte_order_mark_is_no_heading() {
        let marked = b"\xef\xbb\xbf[General]\r\nsA=1\r\nsB=two\r\n[Display]\r\nfB=2\r\n\
                       \r\n[nv-rs General]\r\nsPreset=fixes\r\n";
        assert_eq!(ini_value(marked, b"General", b"sA"), None);
        assert_eq!(ini_value(marked, b"General", b"sB"), None);
        assert_eq!(ini_value(marked, b"Display", b"fB").unwrap(), b"2");
        assert_eq!(
            ini_value(marked, b"nv-rs General", b"sPreset").unwrap(),
            b"fixes"
        );
        // The mark alone on the first line: the next line is a heading.
        let alone = b"\xef\xbb\xbf\r\n[General]\r\nsA=1\r\n";
        assert_eq!(ini_value(alone, b"General", b"sA").unwrap(), b"1");
    }

    /// Windows' answers on a file with a repeated section name, measured on scratch files on the
    /// player's machine (2026-11-09; `GetPrivateProfileStringA` and `GetPrivateProfileIntA` alike):
    /// only the first section of a name is read, the name compared without case and with the
    /// spaces in the brackets and in the caller's name trimmed. An empty first section hides a
    /// later one; an empty key in the first one is found, empty.
    #[test]
    fn only_the_first_section_of_a_name_is_read() {
        let text = b"[A]\r\nx=1\r\n[B]\r\nz=3\r\n[a]\r\ny=2\r\nx=9\r\n[ A ]\r\nw=6\r\n";
        assert_eq!(ini_value(text, b"A", b"x").unwrap(), b"1");
        assert_eq!(ini_value(text, b"A", b"y"), None);
        assert_eq!(ini_value(text, b"A", b"w"), None);
        assert_eq!(ini_value(text, b"B", b"z").unwrap(), b"3");
        assert_eq!(ini_value(text, b" A ", b"x").unwrap(), b"1");
        let text = b"[a]\r\nx=1\r\n[A]\r\ny=2\r\n[C]\r\nq=1\r\n[C]\r\nq=2\r\nr=3\r\n";
        assert_eq!(ini_value(text, b"A", b"x").unwrap(), b"1");
        assert_eq!(ini_value(text, b"A", b"y"), None);
        assert_eq!(ini_value(text, b"C", b"q").unwrap(), b"1");
        assert_eq!(ini_value(text, b"C", b"r"), None);
        assert_eq!(ini_value(b"[A]\r\n[A]\r\nx=1\r\n", b"A", b"x"), None);
        assert_eq!(
            ini_value(b"[A]\r\n; note\r\n\r\n[A]\r\nx=1\r\n", b"A", b"x"),
            None
        );
        assert_eq!(
            ini_value(b"[A]\r\nx=\r\n[A]\r\nx=1\r\n", b"A", b"x").unwrap(),
            b""
        );
    }

    /// Windows' `GetPrivateProfileIntA` answers, measured on the player's machine (2026-10-21).
    #[test]
    fn profile_ints_read_as_windows_does() {
        let measured: &[(&[u8], u32)] = &[
            (b"-5", 0xffff_fffb),
            (b"-0", 0),
            (b"0x10", 16),
            (b"0X1F", 0),
            (b"-0x10", 0xffff_fff0),
            (b"12abc", 12),
            (b"  7", 7),
            (b"+3", 3),
            (b"4294967295", 0xffff_ffff),
            (b"4294967296", 0),
            (b"99999999999", 1_215_752_191),
            (b"1e3", 1),
            (b"0", 0),
            (b"08", 8),
            (b"010", 10),
            (b"-", 0),
            (b"x5", 0),
            (b"0b101", 5),
            (b"0o17", 15),
            (b"0x", 0),
            (b"0xg", 0),
            (b"-  5", 0),
            (b" -5", 0xffff_fffb),
            (b"0b2", 0),
            (b"0x10abc", 0x10abc),
            (b"0xFFFFFFFF1", 0xffff_fff1),
            (b"0B11", 0),
            (b"0O7", 0),
            (b"\t9", 9),
            (b"--5", 0),
            (b"+-5", 0),
            (b"- 5", 0),
            (b"00x10", 0),
            (b"0x-5", 0),
            (b"0b", 0),
            (b"0o8", 0),
            (b"5 6", 5),
        ];
        for &(value, want) in measured {
            assert_eq!(profile_int(value), want, "{:?}", std::str::from_utf8(value));
        }
    }

    /// A missing file gives the default, cut to the buffer; writing changes nothing and answers 1.
    #[test]
    fn profile_strings_fall_back_to_the_default() {
        let mut e = engine();
        e.mem.set_cstr(0x1000, b"Controls");
        e.mem.set_cstr(0x1020, b"Forward");
        e.mem.set_cstr(0x1040, b"default");
        e.mem.set_cstr(0x1060, br"Z:\no\such\file.ini");
        let n = get_private_profile_string_a(
            &mut e,
            Ptr::new(0x1000),
            Ptr::new(0x1020),
            Ptr::new(0x1040),
            Ptr::new(0x1100),
            4,
            Ptr::new(0x1060),
        );
        assert_eq!(n, 3);
        assert_eq!(e.mem.cstr(0x1100), b"def");
        let n = get_private_profile_string_a(
            &mut e,
            Ptr::new(0x1000),
            Ptr::new(0x1020),
            Ptr::NULL,
            Ptr::new(0x1100),
            0x40,
            Ptr::new(0x1060),
        );
        assert_eq!((n, e.mem.u8(0x1100)), (0, 0));
        assert_eq!(
            write_private_profile_string_a(
                &mut e,
                Ptr::new(0x1000),
                Ptr::new(0x1020),
                Ptr::new(0x1040),
                Ptr::new(0x1060)
            ),
            1
        );
    }
}
