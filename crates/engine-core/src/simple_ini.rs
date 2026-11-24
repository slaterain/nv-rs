//! Tweaks' INI files read as Tweaks reads them: `CSimpleIniA` (SimpleIni 4.x as Tweaks ships it,
//! `libs/SimpleINILibrary/SimpleIni.h`), made with its defaults (no multi-key, no multi-line values)
//! and `SetUnicode()`, then `LoadFile`. The rules, from `LoadData`, `FindEntry`, `AddEntry` and
//! `GetLongValue` there:
//!
//! - A UTF-8 byte order mark is passed over. The text ends at its first NUL byte (a C string): a
//!   UTF-16 file ends after its first character, so it gives nothing.
//! - Spaces, tabs, CR and LF between entries are passed over. A line whose first other character is
//!   `;` or `#` is a note. A line ends at LF or CR.
//! - `[name]`: the spaces after `[` (line ends too) are passed over, the name runs to `]`, its
//!   trailing spaces are cut, and the rest of the line is passed over. A `[` line with no `]` is
//!   refused, but the section is changed all the same, to one no name finds (SimpleIni keeps a
//!   pointer to the broken text), so the keys under it are never found ([`Entry::section`] `None`).
//! - `key = value`: the key runs to the first `=`, its trailing spaces cut; a line with no `=` or
//!   an empty key is passed over. The value runs to the line's end, spaces and tabs around it cut.
//!   Quotes are kept and a `;` after the value is part of it (SimpleIni has no inline notes).
//! - Keys before any section are in the section with no name. Section and key names are compared
//!   without case (ASCII; `_mbsicmp`), and a key given again in a section replaces the value (the
//!   last wins); sections of one name are one.
//! - `GetLongValue`: a missing or empty value is the default, as is one longer than 63 bytes (its
//!   64-byte buffer); `0x` or `0X` then hexadecimal through `strtol` (`0x` alone is the default),
//!   else decimal through `strtol`; anything left after the number makes the default.

/// One `key=value` of a file, in the file's order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    /// `None` after a broken `[` line (no name finds it).
    pub section: Option<Vec<u8>>,
    pub key: Vec<u8>,
    pub value: Vec<u8>,
}

fn is_space(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\r' | b'\n')
}

fn is_new_line(b: u8) -> bool {
    matches!(b, b'\r' | b'\n')
}

fn trim_end(s: &[u8]) -> &[u8] {
    let end = s.iter().rposition(|&b| !is_space(b)).map_or(0, |i| i + 1);
    &s[..end]
}

/// The file's entries, as `LoadData` adds them.
pub fn entries(file: &[u8]) -> Vec<Entry> {
    let file = file.strip_prefix(b"\xef\xbb\xbf").unwrap_or(file);
    let text = &file[..file.iter().position(|&b| b == 0).unwrap_or(file.len())];
    let mut out = Vec::new();
    let mut section: Option<Vec<u8>> = Some(Vec::new());
    let mut at = 0;
    let n = text.len();
    while at < n {
        while at < n && is_space(text[at]) {
            at += 1;
        }
        if at >= n {
            break;
        }
        match text[at] {
            b';' | b'#' => {
                while at < n && !is_new_line(text[at]) {
                    at += 1;
                }
            }
            b'[' => {
                at += 1;
                while at < n && is_space(text[at]) {
                    at += 1;
                }
                let start = at;
                while at < n && text[at] != b']' && !is_new_line(text[at]) {
                    at += 1;
                }
                if at >= n || text[at] != b']' {
                    section = None;
                    continue;
                }
                section = Some(trim_end(&text[start..at]).to_vec());
                while at < n && !is_new_line(text[at]) {
                    at += 1;
                }
            }
            _ => {
                let start = at;
                while at < n && text[at] != b'=' && !is_new_line(text[at]) {
                    at += 1;
                }
                if at >= n || text[at] != b'=' {
                    continue;
                }
                if at == start {
                    while at < n && !is_new_line(text[at]) {
                        at += 1;
                    }
                    continue;
                }
                let key = trim_end(&text[start..at]).to_vec();
                at += 1;
                while at < n && matches!(text[at], b' ' | b'\t') {
                    at += 1;
                }
                let value_start = at;
                while at < n && !is_new_line(text[at]) {
                    at += 1;
                }
                let value = trim_end(&text[value_start..at]).to_vec();
                out.push(Entry {
                    section: section.clone(),
                    key,
                    value,
                });
            }
        }
    }
    out
}

/// The file's values by section and key, both lowercase (the last of a key wins); entries under a
/// broken `[` line are left out.
pub fn values(file: &[u8]) -> std::collections::HashMap<(Vec<u8>, Vec<u8>), Vec<u8>> {
    entries(file)
        .into_iter()
        .filter_map(|e| {
            let section = e.section?.to_ascii_lowercase();
            Some(((section, e.key.to_ascii_lowercase()), e.value))
        })
        .collect()
}

/// `GetLongValue(section, key, default)` on a value (`None`: the key is missing).
pub fn long_value(value: Option<&[u8]>, default: i32) -> i32 {
    let Some(value) = value.filter(|v| !v.is_empty()) else {
        return default;
    };
    // `ConvertToStore` into `char szValue[64]`: the value and its NUL.
    if value.len() + 1 > 64 {
        return default;
    }
    let (digits, base) = match value {
        [b'0', b'x' | b'X', rest @ ..] => {
            if rest.is_empty() {
                return default;
            }
            (rest, 16)
        }
        _ => (value, 10),
    };
    match strtol(digits, base) {
        (n, used) if used == digits.len() => n,
        _ => default,
    }
}

/// The C runtime's `strtol` on a 32-bit `long`: the number and how many bytes it used (0 when none
/// was read). White space and a sign first; base 16 takes a `0x` of its own; a value out of range
/// is `LONG_MAX` or `LONG_MIN`.
pub fn strtol(s: &[u8], base: u32) -> (i32, usize) {
    let mut at = 0;
    while at < s.len() && matches!(s[at], b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r') {
        at += 1;
    }
    let negative = match s.get(at) {
        Some(b'-') => {
            at += 1;
            true
        }
        Some(b'+') => {
            at += 1;
            false
        }
        _ => false,
    };
    if base == 16
        && s.get(at) == Some(&b'0')
        && matches!(s.get(at + 1), Some(b'x' | b'X'))
        && s.get(at + 2).is_some_and(u8::is_ascii_hexdigit)
    {
        at += 2;
    }
    let start = at;
    let mut value: i64 = 0;
    let mut overflow = false;
    while let Some(d) = s.get(at).and_then(|&b| (b as char).to_digit(base)) {
        value = value * i64::from(base) + i64::from(d);
        if value > i64::from(i32::MAX) + 1 {
            overflow = true;
            value = i64::from(i32::MAX) + 1;
        }
        at += 1;
    }
    if at == start {
        return (0, 0);
    }
    let n = if negative { -value } else { value };
    let n = if overflow || n > i64::from(i32::MAX) || n < i64::from(i32::MIN) {
        if negative {
            i32::MIN
        } else {
            i32::MAX
        }
    } else {
        n as i32
    };
    (n, at)
}
