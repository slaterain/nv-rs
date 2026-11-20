//! The text of an installer's XML file from its bytes.
//!
//! Met in mods: UTF-16 little-endian with a byte order mark (the FOMOD editors wrote it), UTF-16
//! big-endian with one (rare), UTF-8 with or without one, and Windows-1252 text in a file that
//! says it is UTF-8. The `encoding=` a file declares is often wrong (UTF-16 declared on UTF-8 and
//! the reverse), so it is never read: Vortex removes it before parsing for the same reason, and
//! Mod Organizer 2 falls back through UTF-16, UTF-8 and Latin-1 when the declared one fails.
//! Here: the byte order mark decides; without one, UTF-16 is told by its zero bytes (`<` and `?`
//! are ASCII, so the first two characters give `<\0?\0` or `\0<\0?`); otherwise UTF-8 when the
//! bytes are valid UTF-8, else Windows-1252.

/// The text of `bytes`, by the rules above. Never fails: bytes that do not decode become U+FFFD
/// (UTF-16) or their Windows-1252 character.
pub fn decode(bytes: &[u8]) -> String {
    let text = if let Some(rest) = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]) {
        utf8_or_1252(rest)
    } else if let Some(rest) = bytes.strip_prefix(&[0xFF, 0xFE]) {
        utf16(rest, false)
    } else if let Some(rest) = bytes.strip_prefix(&[0xFE, 0xFF]) {
        utf16(rest, true)
    } else if bytes.len() >= 4 && bytes[0] != 0 && bytes[1] == 0 && bytes[3] == 0 {
        utf16(bytes, false)
    } else if bytes.len() >= 4 && bytes[0] == 0 && bytes[1] != 0 && bytes[2] == 0 {
        utf16(bytes, true)
    } else {
        utf8_or_1252(bytes)
    };
    match text.strip_prefix('\u{FEFF}') {
        Some(rest) => rest.to_owned(),
        None => text,
    }
}

fn utf16(bytes: &[u8], big_endian: bool) -> String {
    let units: Vec<u16> = bytes
        .chunks_exact(2)
        .map(|p| {
            if big_endian {
                u16::from_be_bytes([p[0], p[1]])
            } else {
                u16::from_le_bytes([p[0], p[1]])
            }
        })
        .collect();
    String::from_utf16_lossy(&units)
}

/// Windows-1252's characters for 0x80 to 0x9F; the five it leaves undefined keep their own code
/// (as Windows' own conversion does).
const CP1252_HIGH: [char; 32] = [
    '\u{20AC}', '\u{81}', '\u{201A}', '\u{0192}', '\u{201E}', '\u{2026}', '\u{2020}', '\u{2021}',
    '\u{02C6}', '\u{2030}', '\u{0160}', '\u{2039}', '\u{0152}', '\u{8D}', '\u{017D}', '\u{8F}',
    '\u{90}', '\u{2018}', '\u{2019}', '\u{201C}', '\u{201D}', '\u{2022}', '\u{2013}', '\u{2014}',
    '\u{02DC}', '\u{2122}', '\u{0161}', '\u{203A}', '\u{0153}', '\u{9D}', '\u{017E}', '\u{0178}',
];

fn utf8_or_1252(bytes: &[u8]) -> String {
    match std::str::from_utf8(bytes) {
        Ok(text) => text.to_owned(),
        Err(_) => bytes
            .iter()
            .map(|&b| match b {
                0x80..=0x9F => CP1252_HIGH[usize::from(b - 0x80)],
                _ => char::from(b),
            })
            .collect(),
    }
}
