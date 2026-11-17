//! The game's code page away from Windows: the single-byte pages of the game's official languages
//! as Windows converts them, and Windows' lower and upper case, from tables measured on Windows.
//!
//! On Windows every conversion is Windows' own (`super::nls`, `super::files`): the page is the
//! PC's language setting's. Linux and macOS have no ANSI code page (a program's text there is
//! UTF-8 whatever the language), so the page is the game's: 1252, Windows' Western page, for the
//! English, French, German, Italian and Spanish games, unless `NV_CODE_PAGE` names another
//! ([`chosen`]): 1250 (Central European: the Polish and Czech games) or 1251 (Cyrillic: the
//! Russian game). The system's locale plays no part (Wine's default for a C or English locale is
//! 1252 too). The Japanese game needs Windows' double-byte page 932, which is not provided here.
//!
//! Each page is measured with Windows' own functions (`build43-scratch/measure/platforms/lm-run/
//! codepages.ps1`, Windows 11, 2026-11-17; 1252's table the same as the one measured on
//! 2026-11-16): every byte above 0x7F as `MultiByteToWideChar` turns it (no byte of these pages is
//! refused, not even with `MB_ERR_INVALID_CHARS`), and every character `WideCharToMultiByte` gives
//! a byte for, its best fit included; any other is the default character (`?`). The case tables
//! are `LCMapStringW`'s `LCMAP_LOWERCASE` and `LCMAP_UPPERCASE` of every unit of the first 65,536
//! (user locale en-US: a Turkish or Azeri locale's dotted and dotless i differ there).
//!
//! Each page pairs with the OEM page Windows of that language has ([`oem_of`]: 437, 852, 866), and
//! the machine table answers both and their `IsValidCodePage` and `GetCPInfo` (single-byte pages,
//! default character `?`, measured with the others) for the chosen page.
// Only Linux and macOS use the conversions in a run; on Windows the tests hold them against
// Windows' own.
#![cfg_attr(windows, allow(dead_code))]

use std::collections::HashMap;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::OnceLock;

/// The pages provided: 1252 (the default), 1250, 1251.
pub const PAGES: [u32; 3] = [1252, 1250, 1251];

/// A single-byte code page as Windows converts it.
pub struct Page {
    pub number: u32,
    /// Byte 0x80..0xFF to its unit.
    wide: [u16; 128],
    /// Unit to byte: the page's own characters and Windows' best fit.
    ansi: HashMap<u16, u8>,
}

fn table_text(number: u32) -> Option<&'static str> {
    Some(match number {
        1252 => include_str!("cp1252_measured.tsv"),
        1250 => include_str!("cp1250_measured.tsv"),
        1251 => include_str!("cp1251_measured.tsv"),
        _ => return None,
    })
}

fn hex(s: &str) -> u32 {
    u32::from_str_radix(s, 16).expect("a code page table's hex")
}

fn parse(number: u32, text: &str) -> Page {
    let mut page = Page {
        number,
        wide: [0; 128],
        ansi: HashMap::new(),
    };
    let mut bytes = 0;
    for line in text.lines() {
        let f: Vec<&str> = line.split('\t').collect();
        match f.as_slice() {
            ["w", b, u] => {
                page.wide[hex(b) as usize - 0x80] = hex(u) as u16;
                bytes += 1;
            }
            ["a", u, b] => {
                page.ansi.insert(hex(u) as u16, hex(b) as u8);
            }
            _ => {}
        }
    }
    assert_eq!(bytes, 128, "code page {number}: every byte above 0x7f once");
    page
}

/// The page `number`, if it is provided.
pub fn page(number: u32) -> Option<&'static Page> {
    static PAGES_READ: OnceLock<Vec<Page>> = OnceLock::new();
    PAGES_READ
        .get_or_init(|| {
            PAGES
                .iter()
                .map(|&n| parse(n, table_text(n).expect("a provided page")))
                .collect()
        })
        .iter()
        .find(|p| p.number == number)
}

impl Page {
    /// One byte's unit.
    pub fn unit(&self, byte: u8) -> u16 {
        if byte < 0x80 {
            u16::from(byte)
        } else {
            self.wide[usize::from(byte - 0x80)]
        }
    }

    /// Bytes as UTF-16, one unit each.
    pub fn to_wide(&self, bytes: &[u8]) -> Vec<u16> {
        bytes.iter().map(|&b| self.unit(b)).collect()
    }

    /// One character's byte, if the page has one for it (its own or a best fit).
    pub fn byte(&self, c: u32) -> Option<u8> {
        if c < 0x80 {
            return Some(c as u8);
        }
        u16::try_from(c)
            .ok()
            .and_then(|u| self.ansi.get(&u).copied())
    }

    /// UTF-16 as the page's bytes, unit by unit, `default` for a unit it has none for (each unit of
    /// a surrogate pair is one, as Windows gives a pair two default bytes), and whether `default`
    /// was used. With `best_fit` false (`WC_NO_BEST_FIT_CHARS`) only the page's own characters (a
    /// byte that turns back into the same unit) have a byte.
    pub fn to_ansi_units(&self, units: &[u16], default: u8, best_fit: bool) -> (Vec<u8>, bool) {
        let mut used = false;
        let bytes = units
            .iter()
            .map(|&u| {
                let byte = self
                    .byte(u32::from(u))
                    .filter(|&b| best_fit || self.unit(b) == u)
                    .filter(|_| !(0xd800..=0xdfff).contains(&u));
                byte.unwrap_or_else(|| {
                    used = true;
                    default
                })
            })
            .collect();
        (bytes, used)
    }

    /// [`Page::to_ansi_units`] with Windows' best fit.
    pub fn to_ansi_with(&self, units: &[u16], default: u8) -> (Vec<u8>, bool) {
        self.to_ansi_units(units, default, true)
    }

    /// [`Page::to_ansi_with`] with `?`.
    pub fn to_ansi(&self, units: &[u16]) -> Vec<u8> {
        self.to_ansi_with(units, b'?').0
    }
}

/// The measured case maps: lower and upper by unit, for the units that change.
fn case_maps() -> &'static (HashMap<u16, u16>, HashMap<u16, u16>) {
    static MAPS: OnceLock<(HashMap<u16, u16>, HashMap<u16, u16>)> = OnceLock::new();
    MAPS.get_or_init(|| {
        let (mut lower, mut upper) = (HashMap::new(), HashMap::new());
        for line in include_str!("case_measured.tsv").lines() {
            let f: Vec<&str> = line.split('\t').collect();
            match f.as_slice() {
                ["l", a, b] => {
                    lower.insert(hex(a) as u16, hex(b) as u16);
                }
                ["u", a, b] => {
                    upper.insert(hex(a) as u16, hex(b) as u16);
                }
                _ => {}
            }
        }
        (lower, upper)
    })
}

/// `LCMapStringW`'s `LCMAP_LOWERCASE` of one unit.
pub fn lower(unit: u16) -> u16 {
    *case_maps().0.get(&unit).unwrap_or(&unit)
}

/// `LCMapStringW`'s `LCMAP_UPPERCASE` of one unit.
pub fn upper(unit: u16) -> u16 {
    *case_maps().1.get(&unit).unwrap_or(&unit)
}

/// The OEM page of a Windows whose ANSI page is `ansi`.
pub fn oem_of(ansi: u32) -> u32 {
    match ansi {
        1250 => 852,
        1251 => 866,
        _ => 437,
    }
}

/// The page `NV_CODE_PAGE`'s `value` names (unset or empty: 1252), or why it cannot be used.
pub fn chosen(value: Option<&str>) -> Result<u32, String> {
    let value = value.map(str::trim).unwrap_or_default();
    if value.is_empty() {
        return Ok(1252);
    }
    match value.parse::<u32>() {
        Ok(n) if PAGES.contains(&n) => Ok(n),
        _ => Err(format!(
            "NV_CODE_PAGE={value}: give 1252 (English, French, German, Italian, Spanish), 1250 \
             (Polish, Czech) or 1251 (Russian); the Japanese game's 932 is not provided"
        )),
    }
}

/// The game's page this run (off Windows): set with the machine table ([`set_game_page`]), 1252
/// until then.
static GAME_PAGE: AtomicU32 = AtomicU32::new(1252);

/// Makes `number` (a provided page) the game's page: `CP_ACP` and the file names' page.
pub fn set_game_page(number: u32) {
    assert!(page(number).is_some(), "code page {number} is not provided");
    GAME_PAGE.store(number, Ordering::Relaxed);
}

/// The game's page ([`set_game_page`]).
pub fn game_page() -> &'static Page {
    page(GAME_PAGE.load(Ordering::Relaxed)).expect("a provided page")
}

/// The machine table's code page lines for `ansi`: `GetACP`, `GetOEMCP`, and `IsValidCodePage` and
/// `GetCPInfo` for both (measured with the tables; the same for 1252 and 437 as the fixed answers).
pub fn table_lines(ansi: u32) -> String {
    let oem = oem_of(ansi);
    let mut t = format!("GetACP\t\t{ansi:x}\nGetOEMCP\t\t{oem:x}\n");
    for p in [ansi, oem] {
        t += &format!(
            "IsValidCodePage\t{p:x}\t1\nGetCPInfo\t{p:x}\t1\t010000003f000000000000000000000000000000\n"
        );
    }
    t
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_pages_turn_bytes_as_windows_does() {
        let western = page(1252).unwrap();
        assert_eq!(
            western.to_wide(b"A\x80\x81\xe9"),
            [0x41, 0x20ac, 0x81, 0xe9]
        );
        assert_eq!(western.to_ansi(&[0x41, 0x20ac, 0xe9, 0x416]), b"A\x80\xe9?");
        // The best fit: l with stroke is l.
        assert_eq!(western.to_ansi(&[0x142]), b"l");
        let central = page(1250).unwrap();
        // Polish: l with stroke, s acute, z dot above.
        assert_eq!(central.to_wide(b"\xb3\x9c\xbf"), [0x142, 0x15b, 0x17c]);
        assert_eq!(central.to_ansi(&[0x142, 0x15b, 0x17c]), b"\xb3\x9c\xbf");
        let cyrillic = page(1251).unwrap();
        // Russian: Zhe, ya; yo; 0x98 unassigned stays its own unit.
        assert_eq!(
            cyrillic.to_wide(b"\xc6\xff\xa8\x98"),
            [0x416, 0x44f, 0x401, 0x98]
        );
        assert_eq!(cyrillic.to_ansi(&[0x416, 0x44f, 0x401]), b"\xc6\xff\xa8");
        // A character the page lacks (and has no best fit for) is the default, and says so; e
        // acute's best fit on the Cyrillic page is e; each unit of a pair is one default.
        assert_eq!(
            cyrillic.to_ansi_with(&[0x3042, 0x41], b'*'),
            (b"*A".to_vec(), true)
        );
        assert_eq!(cyrillic.to_ansi_with(&[0xe9], b'*'), (b"e".to_vec(), false));
        assert_eq!(
            western.to_ansi_with(&[0xd83d, 0xde00], b'?'),
            (b"??".to_vec(), true)
        );
        // A lone surrogate is one default too; without best fit, l with stroke has no byte.
        assert_eq!(
            western.to_ansi_with(&[0xdc00, 0x41], b'?'),
            (b"?A".to_vec(), true)
        );
        assert_eq!(
            western.to_ansi_units(&[0x142], b'?', false),
            (b"?".to_vec(), true)
        );
        assert_eq!(
            western.to_ansi_units(&[0xe9], b'?', false),
            (vec![0xe9], false)
        );
        assert!(
            !western
                .to_ansi_with(b"ok".map(u16::from).as_slice(), b'?')
                .1
        );
        assert!(page(932).is_none());
    }

    /// Every byte of each page goes there and back (the page's own characters).
    #[test]
    fn every_byte_comes_back() {
        for n in PAGES {
            let p = page(n).unwrap();
            let all: Vec<u8> = (0..=255).collect();
            assert_eq!(p.to_ansi(&p.to_wide(&all)), all, "code page {n}");
        }
    }

    #[test]
    fn the_case_maps_are_windows() {
        assert_eq!((lower(0x41), upper(0x61)), (0x61, 0x41));
        assert_eq!((lower(0xc9), upper(0xe9)), (0xe9, 0xc9));
        assert_eq!((lower(0x416), upper(0x44f)), (0x436, 0x42f));
        // As measured on Windows (the C runtime's start-up test asks the same): y diaeresis upper
        // is U+0178, sharp s has no single upper case.
        assert_eq!((upper(0xff), upper(0xdf)), (0x178, 0xdf));
        assert_eq!(lower(0x31), 0x31);
    }

    #[test]
    fn the_page_is_chosen_by_the_setting() {
        assert_eq!(chosen(None), Ok(1252));
        assert_eq!(chosen(Some(" ")), Ok(1252));
        assert_eq!(chosen(Some("1251")), Ok(1251));
        assert_eq!(chosen(Some("1250 ")), Ok(1250));
        assert!(chosen(Some("932"))
            .unwrap_err()
            .contains("932 is not provided"));
        assert!(chosen(Some("utf8")).is_err());
        assert_eq!(oem_of(1252), 437);
        assert!(table_lines(1251).starts_with("GetACP\t\t4e3\nGetOEMCP\t\t362\n"));
    }

    /// The tables against Windows' own conversion, on Windows: every byte above 0x7F both without
    /// flags and with `MB_ERR_INVALID_CHARS` (none refused); every unit of the first 65,536 but the
    /// surrogates to bytes, with best fit and with `WC_NO_BEST_FIT_CHARS`, with the default's
    /// "used" flag; a surrogate pair and a lone surrogate; and both cases of every such unit.
    #[cfg(windows)]
    #[test]
    fn the_tables_are_windows_own() {
        extern "system" {
            fn MultiByteToWideChar(
                p: u32,
                f: u32,
                t: *const u8,
                n: i32,
                o: *mut u16,
                c: i32,
            ) -> i32;
            fn WideCharToMultiByte(
                p: u32,
                f: u32,
                t: *const u16,
                n: i32,
                o: *mut u8,
                c: i32,
                d: *const u8,
                u: *mut i32,
            ) -> i32;
            fn LCMapStringW(l: u32, f: u32, t: *const u16, n: i32, o: *mut u16, c: i32) -> i32;
        }
        let units = || (0u16..=0xffff).filter(|u| !(0xd800..=0xdfff).contains(u));
        let mut differences = 0usize;
        for n in PAGES {
            let p = page(n).unwrap();
            for flags in [0u32, 8] {
                for b in 0x80u8..=0xff {
                    let mut out = [0u16; 2];
                    // SAFETY: one byte in, room for two units.
                    let k = unsafe { MultiByteToWideChar(n, flags, &b, 1, out.as_mut_ptr(), 2) };
                    assert_eq!(
                        (k, out[0]),
                        (1, p.unit(b)),
                        "code page {n} byte {b:#x} flags {flags}"
                    );
                }
            }
            let to_bytes = |text: &[u16], flags: u32| {
                let mut out = [0u8; 8];
                let mut used = 0i32;
                // SAFETY: `text` is read for its length; room for eight bytes; no default.
                let k = unsafe {
                    WideCharToMultiByte(
                        n,
                        flags,
                        text.as_ptr(),
                        text.len() as i32,
                        out.as_mut_ptr(),
                        8,
                        std::ptr::null(),
                        &mut used,
                    )
                };
                (out[..k.max(0) as usize].to_vec(), used != 0)
            };
            for best_fit in [true, false] {
                let flags = if best_fit { 0 } else { 0x400 };
                for u in units() {
                    let theirs = to_bytes(&[u], flags);
                    if p.to_ansi_units(&[u], b'?', best_fit) != theirs {
                        differences += 1;
                        eprintln!("code page {n} unit {u:#x} flags {flags:#x}: theirs {theirs:?}");
                    }
                }
                for text in [&[0xd83d, 0xde00][..], &[0xdc00, 0x41], &[0x41, 0xd800]] {
                    assert_eq!(
                        p.to_ansi_units(text, b'?', best_fit),
                        to_bytes(text, flags),
                        "code page {n} {text:x?} flags {flags:#x}"
                    );
                }
            }
        }
        for u in units() {
            for (flags, ours) in [(0x100, lower(u)), (0x200, upper(u))] {
                let mut out = [0u16; 2];
                // SAFETY: one unit in, room for two.
                let k = unsafe { LCMapStringW(0, flags, &u, 1, out.as_mut_ptr(), 2) };
                if (k, out[0]) != (1, ours) {
                    differences += 1;
                    eprintln!("unit {u:#x} flags {flags:#x}: theirs {:#x}", out[0]);
                }
            }
        }
        assert_eq!(
            differences, 0,
            "the tables differ from Windows (lines above)"
        );
    }
}
