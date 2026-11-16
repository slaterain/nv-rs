//! How Windows' ANSI profile calls (`GetPrivateProfileStringA` and the others) see a file's
//! encoding, measured on scratch files with those calls (2026-11-16,
//! `build43-scratch/measure/jip-cmds/files-measure`, `ini-measure6/7/8.ps1`, results
//! `ini6-`, `ini7-` and `ini8-2026-11-16.txt`, ANSI code page 1252):
//! - A file that starts with the UTF-16 LE byte-order mark (`FF FE`) and has an even length is
//!   Unicode text: the calls read and edit it as text, with the same line rules as an ANSI file,
//!   and write it back as UTF-16 LE with its mark (ini6 `U16LE ...`, ini7 `lone cr`, `lf only`,
//!   `no eol trailing spaces`). The calls' arguments and answers are ANSI: an argument byte is the
//!   code page's character (`x=\xe4\xc4` writes U+00E4 U+00C4); an answer's character the code
//!   page lacks is a byte Windows chooses (ini7: U+3042 reads `?`; ini8: Windows' "best fit",
//!   U+0100 reads `A`, U+03A9 `O`). Characters a write does not touch stay as they were (ini7
//!   `change other key keeps them`).
//! - A file with that mark and an odd length is not Unicode (ini7 `odd byte count`), nor is one
//!   with the UTF-16 BE mark (`FE FF`, ini6 `U16BE ...`); a UTF-8 mark (`EF BB BF`) is three ANSI
//!   bytes of the first line (ini6 `U8BOM ...`: the first heading is not found, a write of its
//!   section adds another at the end). A UTF-16 file without a mark is read as Unicode by
//!   Windows' own guess (ini6 `U16LE no BOM`).
//! - Names with bytes above 0x7F are compared with their case (ini6 `ansi other case ...`: `[\xc4rger]`
//!   is not `[\xe4rger]`), as the ASCII-only comparison here does.
//!
//! What the port does with that:
//! - ANSI files (no 0 byte), UTF-8 marked ones among them: as they are.
//! - UTF-16 LE marked files of an even length: decoded to UTF-8 for the same byte rules (UTF-8
//!   puts no ASCII byte inside a character, so every rule sees the same lines, `[`, `=`, `;` and
//!   blanks), the arguments taken from code page 1252, the answers given in it, the file written
//!   back as UTF-16 LE with its mark.
//! - Deliberate differences: a character code page 1252 lacks reads as `?` (Windows' best-fit
//!   table is not copied: a letter like U+0100 reads `A` there); a file with a 0 byte that is not
//!   such a UTF-16 LE file (UTF-16 BE, UTF-16 without a mark, an odd length) is read as ANSI bytes
//!   and never written ([`Decoded::writable`]: Windows' own guess decides there); a UTF-16 LE
//!   file that is only its mark is written only by `WritePrivateProfileStringA` (measured, ini7
//!   `bom only`: a CR LF goes first), the other writes refuse it.

/// The characters code page 1252 has at 0x80-0x9F (the five it leaves out, 0x81, 0x8D, 0x8F, 0x90
/// and 0x9D, map to the same code points, as Windows' `MultiByteToWideChar` maps them).
const CP1252_HIGH: [u16; 32] = [
    0x20ac, 0x0081, 0x201a, 0x0192, 0x201e, 0x2026, 0x2020, 0x2021, 0x02c6, 0x2030, 0x0160, 0x2039,
    0x0152, 0x008d, 0x017d, 0x008f, 0x0090, 0x2018, 0x2019, 0x201c, 0x201d, 0x2022, 0x2013, 0x2014,
    0x02dc, 0x2122, 0x0161, 0x203a, 0x0153, 0x009d, 0x017e, 0x0178,
];

/// A code page 1252 byte as its character.
fn cp1252_char(b: u8) -> char {
    match b {
        0x80..=0x9f => char::from_u32(u32::from(CP1252_HIGH[usize::from(b - 0x80)])).unwrap_or('?'),
        _ => char::from(b),
    }
}

/// A character as its code page 1252 byte, `?` when the code page lacks it.
fn cp1252_byte(c: char) -> u8 {
    let u = u32::from(c);
    if u < 0x80 || (0xa0..=0xff).contains(&u) {
        return u as u8;
    }
    CP1252_HIGH
        .iter()
        .position(|&h| u32::from(h) == u)
        .map_or(b'?', |i| 0x80 + i as u8)
}

/// How a file's bytes are held while the profile rules work on them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Encoding {
    /// ANSI bytes as they are.
    Ansi,
    /// UTF-16 LE with its mark, held as UTF-8.
    Utf16Le,
}

/// A file's text as the profile rules see it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Decoded {
    pub encoding: Encoding,
    /// The text: the file's bytes (ANSI), or its characters as UTF-8 (UTF-16 LE, without the mark).
    pub text: Vec<u8>,
    /// Whether a change may be written back: false for a file with a 0 byte that is not a UTF-16
    /// LE file with its mark (see the module).
    pub writable: bool,
}

impl Decoded {
    /// The file's bytes as the profile calls see them.
    pub fn of(bytes: &[u8]) -> Decoded {
        if let Some(units) = bytes.strip_prefix(&[0xff, 0xfe]) {
            if units.len() % 2 == 0 {
                let units: Vec<u16> = units
                    .chunks_exact(2)
                    .map(|p| u16::from_le_bytes([p[0], p[1]]))
                    .collect();
                if let Ok(text) = String::from_utf16(&units) {
                    return Decoded {
                        encoding: Encoding::Utf16Le,
                        text: text.into_bytes(),
                        writable: true,
                    };
                }
                // A lone surrogate: not text Windows' calls were measured on; read as bytes.
            }
        }
        Decoded {
            encoding: Encoding::Ansi,
            text: bytes.to_vec(),
            writable: !bytes.contains(&0),
        }
    }

    /// A call's ANSI argument in this text's encoding.
    pub fn arg(&self, ansi: &[u8]) -> Vec<u8> {
        match self.encoding {
            Encoding::Ansi => ansi.to_vec(),
            Encoding::Utf16Le => ansi
                .iter()
                .map(|&b| cp1252_char(b))
                .collect::<String>()
                .into_bytes(),
        }
    }

    /// Text found in this file as a call's ANSI answer.
    pub fn answer(&self, found: &[u8]) -> Vec<u8> {
        match self.encoding {
            Encoding::Ansi => found.to_vec(),
            Encoding::Utf16Le => String::from_utf8_lossy(found)
                .chars()
                .map(cp1252_byte)
                .collect(),
        }
    }

    /// A changed text as the file's bytes (UTF-16 LE: its mark first).
    pub fn encode(&self, text: &[u8]) -> Vec<u8> {
        match self.encoding {
            Encoding::Ansi => text.to_vec(),
            Encoding::Utf16Le => {
                let mut out = vec![0xff, 0xfe];
                for unit in String::from_utf8_lossy(text).encode_utf16() {
                    out.extend_from_slice(&unit.to_le_bytes());
                }
                out
            }
        }
    }

    /// A UTF-16 LE file that is only its mark (`FF FE`).
    pub fn mark_only(&self) -> bool {
        self.encoding == Encoding::Utf16Le && self.text.is_empty()
    }
}

#[cfg(test)]
#[path = "mod_files_text_tests.rs"]
mod tests;
