//! The platform's character-type call `GetStringTypeW` (KERNEL32 import), which the C runtime's
//! locale start-up reaches (`__initmbctable` > `__setmbcp`, `__init_ctype`, `__isctype_l`,
//! `__iswctype_l`, through `__crtGetStringTypeA_stat` and `__crtGetStringTypeW`). Every caller in
//! the exe asks for `CT_CTYPE1` with a positive count (`__crtGetStringTypeA_stat` first probes
//! with one unit of the empty wide text at `01011d78`).
//!
//! Measured on this machine (2026-10-27, `build43-scratch/measure/platform5`, a 32-bit program
//! calling the real function):
//! - `CT_CTYPE1` for every UTF-16 code unit of the BMP is `ctype1_measured.tsv` (each unit's
//!   word the same in one call over all of them as in one call per unit); a call writes one word
//!   per unit, answers TRUE and leaves the last error as it was;
//! - a negative count reads through the text's zero, whose word is written too (-1 and -2 alike);
//! - count 0, or a null text, answers FALSE with `ERROR_INVALID_PARAMETER` (87), nothing written;
//! - an info type other than `CT_CTYPE1` (1), `CT_CTYPE2` (2) and `CT_CTYPE3` (4) answers FALSE
//!   with `ERROR_INVALID_FLAGS` (1004), nothing written. `CT_CTYPE2` and `CT_CTYPE3` answer TRUE
//!   but their tables are not measured: not provided (no caller in the exe asks for them).
//!
//! And the code page conversions those same paths make, `MultiByteToWideChar`,
//! `WideCharToMultiByte` and `LCMapStringW`: on Windows the real calls, so every code page and
//! locale answers as it does for the original on that machine (nothing here knows a code page).
//! The guest's text is copied out and the answer back; a failure's last error is Windows' own.
//! A call that fails while sizing its answer leaves the guest's buffer as it was. Elsewhere there
//! is no Windows to ask: the three answer the single-byte pages of the game's languages and
//! Windows' case by tables measured on Windows (`crate::code_page`: 1252 unless `NV_CODE_PAGE`
//! names 1250 or 1251), with Windows' sizing rules below; what those pages do not need (other
//! pages, flags that change a single-byte page's answer, sort keys) stops the run (not provided).
//!
//! Measured on this machine (2026-10-21, `build43-scratch/measure/time-2026-10-21.txt`):
//! `WideCharToMultiByte(CP_ACP (1252 here), 0, text, -1, out, size, NULL, &used)` copies ASCII as
//! is and counts the zero; `used` becomes FALSE; with `size` 0 it answers the size needed and
//! writes nothing; too small a buffer is filled as far as it goes and the call answers 0 with
//! `ERROR_INSUFFICIENT_BUFFER` (122).

use crate::{Engine, Ptr};
use std::sync::OnceLock;

/// `CT_CTYPE1`, `CT_CTYPE2`, `CT_CTYPE3`.
pub const CT_CTYPE1: u32 = 1;
pub const CT_CTYPE2: u32 = 2;
pub const CT_CTYPE3: u32 = 4;
/// `ERROR_INVALID_PARAMETER` and `ERROR_INVALID_FLAGS`.
pub const ERROR_INVALID_PARAMETER: u32 = 87;
pub const ERROR_INVALID_FLAGS: u32 = 1004;

/// The measured `CT_CTYPE1` word of each BMP code unit (`ctype1_measured.tsv`).
pub fn ctype1(unit: u16) -> u16 {
    static TABLE: OnceLock<Vec<u16>> = OnceLock::new();
    let table = TABLE.get_or_init(|| {
        let mut table = vec![0u16; 0x1_0000];
        let mut covered = 0usize;
        for line in include_str!("ctype1_measured.tsv")
            .lines()
            .filter(|l| !l.starts_with('#') && !l.is_empty())
        {
            let f: Vec<u32> = line
                .split('\t')
                .map(|w| u32::from_str_radix(w, 16).expect("a hex word in ctype1_measured.tsv"))
                .collect();
            for unit in f[0]..=f[1] {
                table[unit as usize] = f[2] as u16;
                covered += 1;
            }
        }
        assert_eq!(covered, 0x1_0000, "ctype1_measured.tsv covers the BMP once");
        table
    });
    table[unit as usize]
}

// Platform: replaces KERNEL32.DLL GetStringTypeW (import)
/// `GetStringTypeW(info, text, count, types)`: for `CT_CTYPE1` the measured word of each unit
/// of `text` (`count` units, or through its zero when negative) into `types`, TRUE; the edge
/// cases as measured (module comment).
pub fn get_string_type_w(e: &mut Engine, info: u32, text: Ptr, count: i32, types: Ptr) -> u32 {
    if info != CT_CTYPE1 && info != CT_CTYPE2 && info != CT_CTYPE3 {
        e.kernel.last_error = ERROR_INVALID_FLAGS;
        return 0;
    }
    if count == 0 || text.is_null() {
        e.kernel.last_error = ERROR_INVALID_PARAMETER;
        return 0;
    }
    assert!(
        info == CT_CTYPE1,
        "GetStringTypeW({info}): only CT_CTYPE1 is measured (not provided)"
    );
    let mut i = 0u32;
    loop {
        if count > 0 && i >= count as u32 {
            break;
        }
        let unit = e.mem.u16(text.addr().wrapping_add(2 * i));
        e.mem
            .set_u16(types.addr().wrapping_add(2 * i), ctype1(unit));
        i += 1;
        if count < 0 && unit == 0 {
            break;
        }
    }
    1
}

#[cfg(windows)]
#[link(name = "kernel32")]
extern "system" {
    fn MultiByteToWideChar(
        code_page: u32,
        flags: u32,
        text: *const u8,
        count: i32,
        out: *mut u16,
        size: i32,
    ) -> i32;
    fn WideCharToMultiByte(
        code_page: u32,
        flags: u32,
        wide: *const u16,
        count: i32,
        out: *mut u8,
        size: i32,
        default: *const u8,
        used: *mut i32,
    ) -> i32;
    fn LCMapStringW(
        lcid: u32,
        flags: u32,
        text: *const u16,
        count: i32,
        out: *mut u16,
        size: i32,
    ) -> i32;
    fn GetLastError() -> u32;
}

/// `LCMAP_SORTKEY`: the answer is bytes, not wide characters.
#[cfg(windows)]
const LCMAP_SORTKEY: u32 = 0x400;

/// The guest text a real call reads: `count` bytes, or through its zero for any negative count;
/// nothing for a null text or a count of 0, which Windows refuses without reading.
#[cfg(windows)]
fn guest_bytes(e: &Engine, text: Ptr, count: i32) -> Vec<u8> {
    match count {
        _ if text.is_null() => Vec::new(),
        ..=-1 => {
            let mut bytes = e.mem.cstr(text.addr());
            bytes.push(0);
            bytes
        }
        0 => Vec::new(),
        1.. => e.mem.bytes(text.addr(), count as u32),
    }
}

/// [`guest_bytes`] for wide text.
#[cfg(windows)]
fn guest_wide(e: &Engine, text: Ptr, count: i32) -> Vec<u16> {
    let mut units = Vec::new();
    if text.is_null() || count == 0 {
        return units;
    }
    let mut at = text.addr();
    loop {
        let unit = e.mem.u16(at);
        units.push(unit);
        at = at.wrapping_add(2);
        if (count < 0 && unit == 0) || (count > 0 && units.len() as i32 == count) {
            return units;
        }
    }
}

/// A real text pointer: null where the guest's was.
#[cfg(windows)]
fn real_text<T>(text: Ptr, copy: &[T]) -> *const T {
    if text.is_null() {
        std::ptr::null()
    } else {
        copy.as_ptr()
    }
}

/// Makes a real call whose output is `size` units of `unit` bytes at `out`. `api` is called with
/// a real buffer and its size. A call with no buffer goes through as it came; otherwise it is
/// sized first (the guest's `size` can be far above what is mapped there) and given the guest's
/// bytes up to what it can write, which go back after. A zero answer's last error is Windows'.
#[cfg(windows)]
fn with_output<T: Default + Copy>(
    e: &mut Engine,
    out: Ptr,
    size: i32,
    unit: u32,
    api: impl Fn(*mut T, i32) -> i32,
) -> u32 {
    // The answer and, when it is 0, Windows' last error before anything else can change it.
    let call = |buffer: *mut T, room: i32| {
        let answer = api(buffer, room);
        // SAFETY: a plain query, on the thread of the call just made.
        let error = if answer == 0 {
            unsafe { GetLastError() }
        } else {
            0
        };
        (answer, error)
    };
    let (answer, error) = if size <= 0 || out.is_null() {
        call(std::ptr::null_mut(), size)
    } else {
        match call(std::ptr::null_mut(), 0) {
            (0, error) => (0, error),
            (needed, _) => {
                let room = needed.min(size);
                let bytes = room as usize * unit as usize;
                let mut buffer = vec![T::default(); bytes.div_ceil(std::mem::size_of::<T>())];
                let guest = e.mem.bytes(out.addr(), bytes as u32);
                // SAFETY: `buffer` holds at least `bytes` bytes (`room` units) of a plain integer
                // type: copied in and read back as bytes, and the real call writes at most `room`
                // units.
                unsafe {
                    let raw = buffer.as_mut_ptr().cast::<u8>();
                    std::ptr::copy_nonoverlapping(guest.as_ptr(), raw, bytes);
                    let result = call(buffer.as_mut_ptr(), room);
                    e.mem
                        .write(out.addr(), std::slice::from_raw_parts(raw, bytes));
                    result
                }
            }
        }
    };
    if answer == 0 {
        e.kernel.last_error = error;
    }
    answer as u32
}

// Platform: replaces KERNEL32.DLL MultiByteToWideChar (import)
/// `MultiByteToWideChar(code page, flags, text, count, out, size)`: Windows' answer for the
/// code page it is asked about (module comment).
#[cfg(windows)]
pub fn multi_byte_to_wide_char(
    e: &mut Engine,
    code_page: u32,
    flags: u32,
    text: Ptr,
    count: i32,
    out: Ptr,
    size: i32,
) -> u32 {
    let copy = guest_bytes(e, text, count);
    let source = real_text(text, &copy);
    with_output(e, out, size, 2, |buffer, room| {
        // SAFETY: `source` is a copy of what Windows reads ([`guest_bytes`]), or null, or empty for
        // a count of 0, which it refuses without reading; `buffer` holds `room` units.
        unsafe { MultiByteToWideChar(code_page, flags, source, count, buffer, room) }
    })
}

// Platform: replaces KERNEL32.DLL WideCharToMultiByte (import)
/// `WideCharToMultiByte(code page, flags, wide, count, out, size, default, used)`: Windows'
/// answer for the code page it is asked about (module comment).
#[cfg(windows)]
#[allow(clippy::too_many_arguments)]
pub fn wide_char_to_multi_byte(
    e: &mut Engine,
    code_page: u32,
    flags: u32,
    wide: Ptr,
    count: i32,
    out: Ptr,
    size: i32,
    default: Ptr,
    used: Ptr,
) -> u32 {
    let copy = guest_wide(e, wide, count);
    let source = real_text(wide, &copy);
    let default_copy = if default.is_null() {
        Vec::new()
    } else {
        let mut bytes = e.mem.cstr(default.addr());
        bytes.push(0);
        bytes
    };
    let default_text = real_text(default, &default_copy);
    let used_flag = std::cell::Cell::new(if used.is_null() {
        0
    } else {
        e.mem.u32(used.addr()) as i32
    });
    let used_real = if used.is_null() {
        std::ptr::null_mut()
    } else {
        used_flag.as_ptr()
    };
    let answer = with_output(e, out, size, 1, |buffer, room| {
        // SAFETY: as in `multi_byte_to_wide_char`; `default_text` is a copy of the guest's
        // zero-ended text or null, `used_real` a live word or null.
        unsafe {
            WideCharToMultiByte(
                code_page,
                flags,
                source,
                count,
                buffer,
                room,
                default_text,
                used_real,
            )
        }
    });
    if !used.is_null() {
        e.mem.set_u32(used.addr(), used_flag.get() as u32);
    }
    answer
}

// Platform: replaces KERNEL32.DLL LCMapStringW (import)
/// `LCMapStringW(locale id, flags, text, count, out, size)`: Windows' answer for the locale it
/// is asked about (module comment). `LCMAP_SORTKEY`'s output is counted in bytes.
#[cfg(windows)]
pub fn lc_map_string_w(
    e: &mut Engine,
    lcid: u32,
    flags: u32,
    text: Ptr,
    count: i32,
    out: Ptr,
    size: i32,
) -> u32 {
    let copy = guest_wide(e, text, count);
    let source = real_text(text, &copy);
    let unit = if flags & LCMAP_SORTKEY != 0 { 1 } else { 2 };
    with_output(e, out, size, unit, |buffer, room| {
        // SAFETY: as in `multi_byte_to_wide_char`; `buffer` holds `room` units of the output,
        // two-byte aligned.
        unsafe { LCMapStringW(lcid, flags, source, count, buffer, room) }
    })
}

/// The conversions away from Windows (module comment), compiled on Windows too for the tests that
/// hold them against Windows' own.
#[cfg(any(not(windows), test))]
pub mod measured {
    use super::*;

    /// The page a conversion off Windows is asked for: `CP_ACP` (0) and `CP_THREAD_ACP` (3) are the
    /// game's ([`crate::code_page::game_page`]), else the page by its number when it is provided.
    fn page_of(code_page: u32) -> Option<&'static crate::code_page::Page> {
        match code_page {
            0 | 3 => Some(crate::code_page::game_page()),
            n => crate::code_page::page(n),
        }
    }

    /// `ERROR_INSUFFICIENT_BUFFER`.
    const ERROR_INSUFFICIENT_BUFFER: u32 = 122;

    /// The guest's `count` units of `unit` bytes at `text`, or through its zero (counted) for a
    /// negative count.
    fn guest_units(e: &Engine, text: Ptr, count: i32, unit: u32) -> Vec<u32> {
        let read = |at: u32| {
            if unit == 1 {
                u32::from(e.mem.u8(at))
            } else {
                u32::from(e.mem.u16(at))
            }
        };
        let mut out = Vec::new();
        let mut at = text.addr();
        loop {
            if count >= 0 && out.len() as i32 >= count {
                return out;
            }
            let v = read(at);
            out.push(v);
            at = at.wrapping_add(unit);
            if count < 0 && v == 0 {
                return out;
            }
        }
    }

    /// Hands an answer of `answer.len()` units of `unit` bytes back as Windows does: `size` 0 asks
    /// only its length; too small a buffer is filled as far as it goes and the call answers 0 with
    /// `ERROR_INSUFFICIENT_BUFFER`.
    fn give_back(e: &mut Engine, out: Ptr, size: i32, unit: u32, answer: &[u32]) -> u32 {
        if size == 0 {
            return answer.len() as u32;
        }
        let room = (size.max(0) as usize).min(answer.len());
        for (i, &v) in answer[..room].iter().enumerate() {
            let at = out.addr().wrapping_add(unit * i as u32);
            if unit == 1 {
                e.mem.set_u8(at, v as u8);
            } else {
                e.mem.set_u16(at, v as u16);
            }
        }
        if room < answer.len() {
            e.kernel.last_error = ERROR_INSUFFICIENT_BUFFER;
            return 0;
        }
        answer.len() as u32
    }

    /// The refusals every conversion shares: no text or a count of 0, or a negative size, is
    /// `ERROR_INVALID_PARAMETER` (Windows', measured for the first two: `tests_forwarded`).
    fn refused(e: &mut Engine, text: Ptr, count: i32, size: i32) -> bool {
        if text.is_null() || count == 0 || size < 0 {
            e.kernel.last_error = ERROR_INVALID_PARAMETER;
            return true;
        }
        false
    }

    // Platform: replaces KERNEL32.DLL MultiByteToWideChar (import)
    /// `MultiByteToWideChar(code page, flags, text, count, out, size)` away from Windows: the
    /// single-byte pages of `crate::code_page` by Windows' measured tables, one unit per byte (no byte
    /// of these pages is refused, so `MB_ERR_INVALID_CHARS` changes nothing). Flags
    /// `MB_PRECOMPOSED` (1) and `MB_ERR_INVALID_CHARS` (8) are taken (on a single-byte page neither
    /// changes the answer); any other bit outside Windows' four is `ERROR_INVALID_FLAGS`, as measured;
    /// `MB_COMPOSITE` (2) and `MB_USEGLYPHCHARS` (4), and a page not provided, stop the run (not
    /// provided).
    pub fn multi_byte_to_wide_char(
        e: &mut Engine,
        code_page: u32,
        flags: u32,
        text: Ptr,
        count: i32,
        out: Ptr,
        size: i32,
    ) -> u32 {
        if flags & !0xf != 0 {
            e.kernel.last_error = ERROR_INVALID_FLAGS;
            return 0;
        }
        if refused(e, text, count, size) {
            return 0;
        }
        assert!(
            flags & 6 == 0,
            "MultiByteToWideChar(code page {code_page}, flags {flags:#x}) is not provided"
        );
        let Some(page) = page_of(code_page) else {
            panic!("MultiByteToWideChar: code page {code_page} is not provided");
        };
        let answer: Vec<u32> = guest_units(e, text, count, 1)
            .into_iter()
            .map(|b| u32::from(page.unit(b as u8)))
            .collect();
        give_back(e, out, size, 2, &answer)
    }

    /// `WC_NO_BEST_FIT_CHARS`, and every flag `WideCharToMultiByte` knows (`WC_DISCARDNS`,
    /// `WC_SEPCHARS`, `WC_DEFAULTCHAR`, `WC_ERR_INVALID_CHARS`, `WC_COMPOSITECHECK`, it): a bit
    /// outside them is `ERROR_INVALID_FLAGS`.
    const WC_NO_BEST_FIT_CHARS: u32 = 0x400;
    const WC_KNOWN: u32 = 0x10 | 0x20 | 0x40 | WC_ERR_INVALID_CHARS | 0x200 | WC_NO_BEST_FIT_CHARS;
    /// `WC_ERR_INVALID_CHARS`.
    const WC_ERR_INVALID_CHARS: u32 = 0x80;

    // Platform: replaces KERNEL32.DLL WideCharToMultiByte (import)
    /// `WideCharToMultiByte(code page, flags, wide, count, out, size, default, used)` away from
    /// Windows: the single-byte pages of `crate::code_page` by Windows' measured tables, unit by
    /// unit (the page's own characters and Windows' best fit, or with `WC_NO_BEST_FIT_CHARS` the
    /// page's own only; any other unit, each unit of a surrogate pair included, the default
    /// character: the first byte of `default` when it is given, else `?`, and `used` TRUE).
    /// Measured (2026-10-21): ASCII as is, the zero counted for a count of -1, `used` FALSE when
    /// nothing needed the default. Flags: 0 and `WC_NO_BEST_FIT_CHARS` are provided; a bit
    /// Windows does not know is `ERROR_INVALID_FLAGS`; the other flags Windows knows
    /// (`WC_COMPOSITECHECK` and its companions, `WC_ERR_INVALID_CHARS`), and a page not provided,
    /// stop the run (not provided).
    #[allow(clippy::too_many_arguments)]
    pub fn wide_char_to_multi_byte(
        e: &mut Engine,
        code_page: u32,
        flags: u32,
        wide: Ptr,
        count: i32,
        out: Ptr,
        size: i32,
        default: Ptr,
        used: Ptr,
    ) -> u32 {
        // `WC_ERR_INVALID_CHARS` is taken only by the UTF-8 page: on a single-byte page it is
        // `ERROR_INVALID_FLAGS`, as Windows answers (compared in the tests).
        if flags & !WC_KNOWN != 0 || flags & WC_ERR_INVALID_CHARS != 0 {
            e.kernel.last_error = ERROR_INVALID_FLAGS;
            return 0;
        }
        if refused(e, wide, count, size) {
            return 0;
        }
        assert!(
            flags & !WC_NO_BEST_FIT_CHARS == 0,
            "WideCharToMultiByte(code page {code_page}, flags {flags:#x}) is not provided"
        );
        let Some(page) = page_of(code_page) else {
            panic!("WideCharToMultiByte: code page {code_page} is not provided");
        };
        let units: Vec<u16> = guest_units(e, wide, count, 2)
            .into_iter()
            .map(|u| u as u16)
            .collect();
        let fill = if default.is_null() {
            b'?'
        } else {
            e.mem.u8(default.addr())
        };
        let best_fit = flags & WC_NO_BEST_FIT_CHARS == 0;
        let (bytes, defaulted) = page.to_ansi_units(&units, fill, best_fit);
        // "used" says whether the default stood for a unit Windows turned: all of them for the
        // size query, only those that fit in a buffer too small (compared with Windows).
        let defaulted = match usize::try_from(size) {
            Ok(room) if room > 0 && room < units.len() => {
                page.to_ansi_units(&units[..room], fill, best_fit).1
            }
            _ => defaulted,
        };
        if !used.is_null() {
            e.mem.set_u32(used.addr(), u32::from(defaulted));
        }
        let answer: Vec<u32> = bytes.into_iter().map(u32::from).collect();
        give_back(e, out, size, 1, &answer)
    }

    /// `LCMAP_LOWERCASE`, `LCMAP_UPPERCASE`.
    const LCMAP_LOWERCASE: u32 = 0x100;
    const LCMAP_UPPERCASE: u32 = 0x200;
    /// Every flag `LCMapStringW` knows (the `NORM_`, `LINGUISTIC_`, `SORT_` and `LCMAP_` ones,
    /// `LCMAP_HASH` and `LCMAP_SORTHANDLE` included): a
    /// bit outside them is `ERROR_INVALID_FLAGS`.
    const LCMAP_KNOWN: u32 = 0x0ff3_1f3f | 0x0004_0000 | 0x2000_0000;

    // Platform: replaces KERNEL32.DLL LCMapStringW (import)
    /// `LCMapStringW(locale id, flags, text, count, out, size)` away from Windows: `LCMAP_LOWERCASE`
    /// or `LCMAP_UPPERCASE` alone, each unit by Windows' measured case tables
    /// (`crate::code_page::lower`, `upper`: an English user locale's, whatever locale is named), the
    /// zero counted for a negative count. No flag, or a bit Windows does not know, is
    /// `ERROR_INVALID_FLAGS` (measured); any other flag (a sort key, `LCMAP_LINGUISTIC_CASING`, the
    /// width and kana maps) stops the run (not provided). Not provided either: the case of a
    /// character outside the first 65,536 (a surrogate pair is left as it is, unit by unit).
    pub fn lc_map_string_w(
        e: &mut Engine,
        lcid: u32,
        flags: u32,
        text: Ptr,
        count: i32,
        out: Ptr,
        size: i32,
    ) -> u32 {
        if flags == 0 || flags & !LCMAP_KNOWN != 0 {
            e.kernel.last_error = ERROR_INVALID_FLAGS;
            return 0;
        }
        if refused(e, text, count, size) {
            return 0;
        }
        let map: fn(u16) -> u16 = match flags {
            LCMAP_LOWERCASE => crate::code_page::lower,
            LCMAP_UPPERCASE => crate::code_page::upper,
            _ => panic!("LCMapStringW(locale {lcid:#x}, flags {flags:#x}) is not provided"),
        };
        let answer: Vec<u32> = guest_units(e, text, count, 2)
            .into_iter()
            .map(|u| u32::from(map(u as u16)))
            .collect();
        give_back(e, out, size, 2, &answer)
    }
}

#[cfg(not(windows))]
pub use measured::{lc_map_string_w, multi_byte_to_wide_char, wide_char_to_multi_byte};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ascii_names_convert_and_short_buffers_fail() {
        let mut e = Engine::new();
        let wide = e.mem.alloc(64);
        for (i, c) in "Eastern\0".encode_utf16().enumerate() {
            e.mem.set_u16(wide + 2 * i as u32, c);
        }
        let out = e.mem.alloc(16);
        let used = e.mem.alloc(4);
        e.mem.set_u32(used, 7);
        let call = |e: &mut Engine, size: i32| {
            wide_char_to_multi_byte(
                e,
                1252,
                0,
                Ptr::new(wide),
                -1,
                Ptr::new(out),
                size,
                Ptr::new(0),
                Ptr::new(used),
            )
        };
        assert_eq!(call(&mut e, 0x3f), 8);
        assert_eq!(e.mem.cstr(out), b"Eastern".to_vec());
        assert_eq!(e.mem.u32(used), 0);
        assert_eq!(call(&mut e, 0), 8);
        e.mem.write(out, &[0xaa; 8]);
        assert_eq!(call(&mut e, 4), 0);
        assert_eq!(e.kernel.last_error, 122);
        assert_eq!(e.mem.bytes(out, 5), b"East\xaa".to_vec());
    }

    fn wide(e: &mut Engine, text: &str) -> u32 {
        let units: Vec<u16> = text.encode_utf16().collect();
        let at = e.mem.alloc(2 * units.len() as u32 + 2);
        for (i, u) in units.iter().enumerate() {
            e.mem.set_u16(at + 2 * i as u32, *u);
        }
        at
    }

    /// The measured answers for "Ab 1" and its zero.
    #[test]
    fn ctype1_answers_the_measured_words() {
        let mut e = Engine::new();
        let text = wide(&mut e, "Ab 1\0zz");
        let out = e.mem.alloc(16);
        e.mem.write(out, &[0xaa; 16]);
        e.kernel.last_error = 0x1234;
        assert_eq!(
            get_string_type_w(&mut e, 1, Ptr::new(text), -1, Ptr::new(out)),
            1
        );
        let words: Vec<u16> = (0..6).map(|i| e.mem.u16(out + 2 * i)).collect();
        assert_eq!(words, [0x381, 0x382, 0x248, 0x284, 0x220, 0xaaaa]);
        assert_eq!(e.kernel.last_error, 0x1234);
        assert_eq!(
            get_string_type_w(&mut e, 1, Ptr::new(text), 2, Ptr::new(out)),
            1
        );
        // The table's ends and a run inside it.
        assert_eq!(ctype1(0x0009), 0x268);
        assert_eq!(ctype1(0x000c), 0x228);
        assert_eq!(ctype1(0xfffd), 0x200);
        assert_eq!(ctype1(0xffff), 0);
    }

    #[test]
    fn edge_cases_fail_as_measured() {
        let mut e = Engine::new();
        let text = wide(&mut e, "A");
        let out = e.mem.alloc(4);
        e.mem.set_u16(out, 0xaaaa);
        for (info, at, count, error) in [
            (1u32, text, 0i32, 87u32),
            (1, 0, 1, 87),
            (0, text, 1, 1004),
            (8, text, 1, 1004),
        ] {
            e.kernel.last_error = 0;
            assert_eq!(
                get_string_type_w(&mut e, info, Ptr::new(at), count, Ptr::new(out)),
                0
            );
            assert_eq!(e.kernel.last_error, error);
            assert_eq!(e.mem.u16(out), 0xaaaa);
        }
    }
}

/// The conversions away from Windows: the same answers the forwarded calls' tests hold Windows to
/// (`tests_forwarded`), and on Windows each against Windows' own call.
#[cfg(test)]
mod tests_measured {
    #[cfg(windows)]
    use super::measured;
    use super::measured::{lc_map_string_w, multi_byte_to_wide_char, wide_char_to_multi_byte};
    use crate::{Engine, Ptr};

    fn wide_text(e: &mut Engine, units: &[u16]) -> u32 {
        let at = e.mem.alloc(2 * units.len() as u32 + 2);
        for (i, u) in units.iter().enumerate() {
            e.mem.set_u16(at + 2 * i as u32, *u);
        }
        at
    }

    fn wide_out(e: &Engine, at: u32, count: u32) -> Vec<u16> {
        (0..count).map(|i| e.mem.u16(at + 2 * i)).collect()
    }

    #[test]
    fn multi_byte_answers_as_windows() {
        let mut e = Engine::new();
        let text = e.mem.alloc(8);
        e.mem.write(text, b"A\x80\x81\xe9\0\xff");
        let out = e.mem.alloc(16);
        e.kernel.last_error = 0x1234;
        let call = |e: &mut Engine, count: i32, size: i32| {
            multi_byte_to_wide_char(e, 1252, 1, Ptr::new(text), count, Ptr::new(out), size)
        };
        assert_eq!(call(&mut e, 4, 0), 4);
        assert_eq!(e.mem.u16(out), 0);
        assert_eq!(call(&mut e, 4, 8), 4);
        assert_eq!(wide_out(&e, out, 5), [0x41, 0x20ac, 0x81, 0xe9, 0]);
        assert_eq!(e.kernel.last_error, 0x1234);
        for count in [-1, -2] {
            e.mem.write(out, &[0xaa; 16]);
            assert_eq!(call(&mut e, count, 8), 5);
            assert_eq!(wide_out(&e, out, 6), [0x41, 0x20ac, 0x81, 0xe9, 0, 0xaaaa]);
        }
        e.mem.write(out, &[0xaa; 16]);
        assert_eq!(call(&mut e, 4, 2), 0);
        assert_eq!(e.kernel.last_error, 122);
        assert_eq!(wide_out(&e, out, 3), [0x41, 0x20ac, 0xaaaa]);
        assert_eq!(call(&mut e, 0, 8), 0);
        assert_eq!(e.kernel.last_error, 87);
        assert_eq!(
            multi_byte_to_wide_char(&mut e, 1252, 0x8000, Ptr::new(text), 4, Ptr::new(out), 8),
            0
        );
        assert_eq!(e.kernel.last_error, 1004);
        assert_eq!(
            multi_byte_to_wide_char(&mut e, 1252, 9, Ptr::new(0), 4, Ptr::new(out), 8),
            0
        );
        assert_eq!(e.kernel.last_error, 87);
        // Russian: Zhe. The game's page (`CP_ACP`) is 1252 unless set.
        let zhe = e.mem.alloc(2);
        e.mem.write(zhe, b"\xc6");
        assert_eq!(
            multi_byte_to_wide_char(&mut e, 1251, 9, Ptr::new(zhe), 1, Ptr::new(out), 2),
            1
        );
        assert_eq!(e.mem.u16(out), 0x416);
        assert_eq!(
            multi_byte_to_wide_char(&mut e, 0, 1, Ptr::new(zhe), 1, Ptr::new(out), 2),
            1
        );
        assert_eq!(e.mem.u16(out), 0xc6);
    }

    #[test]
    fn wide_to_multi_byte_answers_as_windows() {
        let mut e = Engine::new();
        let text = wide_text(&mut e, &[0x41, 0x20ac, 0xe9, 0x416]);
        let out = e.mem.alloc(8);
        let used = e.mem.alloc(4);
        e.mem.set_u32(used, 7);
        let call = |e: &mut Engine, page: u32, default: u32, used: u32| {
            wide_char_to_multi_byte(
                e,
                page,
                0,
                Ptr::new(text),
                4,
                Ptr::new(out),
                8,
                Ptr::new(default),
                Ptr::new(used),
            )
        };
        assert_eq!(call(&mut e, 1252, 0, used), 4);
        assert_eq!(e.mem.bytes(out, 4), b"A\x80\xe9?".to_vec());
        assert_eq!(e.mem.u32(used), 1);
        let star = e.mem.alloc(2);
        e.mem.write(star, b"*\0");
        assert_eq!(call(&mut e, 1252, star, 0), 4);
        assert_eq!(e.mem.bytes(out, 4), b"A\x80\xe9*".to_vec());
        assert_eq!(call(&mut e, 1251, 0, used), 4);
        assert_eq!(e.mem.bytes(out, 4), b"A\x88e\xc6".to_vec());
        assert_eq!(e.mem.u32(used), 0, "e acute's best fit is e");
        assert_eq!(call(&mut e, 1252, 0, 0), 4);
        let ascii = wide_text(&mut e, &[0x41, 0x42, 0]);
        e.mem.set_u32(used, 7);
        assert_eq!(
            wide_char_to_multi_byte(
                &mut e,
                0,
                0,
                Ptr::new(ascii),
                -1,
                Ptr::new(out),
                0,
                Ptr::new(0),
                Ptr::new(used)
            ),
            3
        );
        assert_eq!(e.mem.u32(used), 0);
    }

    #[test]
    fn lc_map_maps_case_as_windows() {
        let mut e = Engine::new();
        let text = wide_text(&mut e, &[0x41, 0xc9, 0x416]);
        let out = e.mem.alloc(16);
        let empty = wide_text(&mut e, &[0]);
        assert_eq!(
            lc_map_string_w(&mut e, 0, 0x100, Ptr::new(empty), 1, Ptr::new(0), 0),
            1
        );
        let call = |e: &mut Engine, flags: u32, size: i32| {
            lc_map_string_w(e, 0x409, flags, Ptr::new(text), 3, Ptr::new(out), size)
        };
        assert_eq!(call(&mut e, 0x100, 0), 3);
        assert_eq!(call(&mut e, 0x100, 8), 3);
        assert_eq!(wide_out(&e, out, 3), [0x61, 0xe9, 0x436]);
        let upper = wide_text(&mut e, &[0xff, 0xdf]);
        assert_eq!(
            lc_map_string_w(&mut e, 0x409, 0x200, Ptr::new(upper), 2, Ptr::new(out), 8),
            2
        );
        assert_eq!(wide_out(&e, out, 2), [0x178, 0xdf]);
        assert_eq!(call(&mut e, 0x100, 1), 0);
        assert_eq!(e.kernel.last_error, 122);
        assert_eq!(call(&mut e, 0, 8), 0);
        assert_eq!(e.kernel.last_error, 1004);
    }

    /// Every byte of each page, and a mix of characters with and without a byte, in both
    /// directions and in both cases, the same as Windows' own calls.
    #[cfg(windows)]
    #[test]
    fn the_measured_calls_agree_with_windows() {
        let mut e = Engine::new();
        let bytes: Vec<u8> = (1..=255).collect();
        let text = e.mem.alloc(256);
        e.mem.write(text, &bytes);
        let units: Vec<u16> = (0x20..0x600)
            .chain([0x20ac, 0x2122, 0x2013, 0xfb01, 0xff21, 0x3042])
            .collect();
        let wide = wide_text(&mut e, &units);
        let (a, b) = (e.mem.alloc(0x2000), e.mem.alloc(0x2000));
        let n = units.len() as i32;
        for page in crate::code_page::PAGES {
            let ours = measured::multi_byte_to_wide_char(
                &mut e,
                page,
                1,
                Ptr::new(text),
                255,
                Ptr::new(a),
                512,
            );
            let theirs = super::multi_byte_to_wide_char(
                &mut e,
                page,
                1,
                Ptr::new(text),
                255,
                Ptr::new(b),
                512,
            );
            assert_eq!(
                (ours, wide_out(&e, a, ours)),
                (theirs, wide_out(&e, b, theirs)),
                "{page}"
            );
            let (ua, ub) = (e.mem.alloc(4), e.mem.alloc(4));
            let ours = measured::wide_char_to_multi_byte(
                &mut e,
                page,
                0,
                Ptr::new(wide),
                n,
                Ptr::new(a),
                0x2000,
                Ptr::new(0),
                Ptr::new(ua),
            );
            let theirs = super::wide_char_to_multi_byte(
                &mut e,
                page,
                0,
                Ptr::new(wide),
                n,
                Ptr::new(b),
                0x2000,
                Ptr::new(0),
                Ptr::new(ub),
            );
            assert_eq!(ours, theirs, "{page}");
            assert_eq!(e.mem.bytes(a, ours), e.mem.bytes(b, theirs), "{page}");
            assert_eq!(e.mem.u32(ua), e.mem.u32(ub), "{page}");
        }
        for flags in [0x100, 0x200] {
            let ours =
                measured::lc_map_string_w(&mut e, 0, flags, Ptr::new(wide), n, Ptr::new(a), 0x1000);
            let theirs =
                super::lc_map_string_w(&mut e, 0, flags, Ptr::new(wide), n, Ptr::new(b), 0x1000);
            assert_eq!(
                (ours, wide_out(&e, a, ours)),
                (theirs, wide_out(&e, b, theirs)),
                "{flags:#x}"
            );
        }
        // Bits Windows does not know: the same answer and error from both.
        let pair = wide_text(&mut e, &[0xd83d, 0xde00, 0x41]);
        let answer = |e: &mut Engine, ours: bool, call: u32, flags: u32| {
            e.kernel.last_error = 0;
            let (out, used) = (Ptr::new(a), Ptr::new(0));
            let k = match (call, ours) {
                (0, true) => measured::wide_char_to_multi_byte(
                    e,
                    1252,
                    flags,
                    Ptr::new(pair),
                    3,
                    out,
                    8,
                    Ptr::new(0),
                    used,
                ),
                (0, false) => super::wide_char_to_multi_byte(
                    e,
                    1252,
                    flags,
                    Ptr::new(pair),
                    3,
                    out,
                    8,
                    Ptr::new(0),
                    used,
                ),
                (1, true) => {
                    measured::lc_map_string_w(e, 0x409, flags, Ptr::new(pair), 3, Ptr::new(a), 8)
                }
                (1, false) => {
                    super::lc_map_string_w(e, 0x409, flags, Ptr::new(pair), 3, Ptr::new(a), 8)
                }
                (_, true) => measured::multi_byte_to_wide_char(
                    e,
                    1252,
                    flags,
                    Ptr::new(text),
                    4,
                    Ptr::new(a),
                    8,
                ),
                (_, false) => super::multi_byte_to_wide_char(
                    e,
                    1252,
                    flags,
                    Ptr::new(text),
                    4,
                    Ptr::new(a),
                    8,
                ),
            };
            (k, e.kernel.last_error, e.mem.bytes(a, k.min(8)))
        };
        for (call, flags) in [
            (0, 0),
            (0, 0x400),
            (0, 0x80),
            (0, 1),
            (0, 0x8000),
            (1, 0x4000_0000),
            (1, 0x100),
            (2, 0x8000),
            (2, 0x100),
        ] {
            assert_eq!(
                answer(&mut e, true, call, flags),
                answer(&mut e, false, call, flags),
                "call {call} flags {flags:#x}"
            );
        }
        // The size query and a buffer too small: the answer, the error and the "used" flag (the
        // C runtime's wcstombs reads it from the size query), on each page, with and without
        // best fit, over text with ASCII, a best fit, a character with none and a pair.
        let mixed = wide_text(
            &mut e,
            &[0x41, 0xe9, 0x142, 0x416, 0x3042, 0xd83d, 0xde00, 0x7a],
        );
        for page in crate::code_page::PAGES {
            for flags in [0, 0x400] {
                for size in [0, 2, 4, 6] {
                    let call = |e: &mut Engine, ours: bool| {
                        let used = e.mem.alloc(4);
                        e.mem.set_u32(used, 7);
                        e.kernel.last_error = 0;
                        let out = if size == 0 { Ptr::new(0) } else { Ptr::new(a) };
                        let k = if ours {
                            measured::wide_char_to_multi_byte(
                                e,
                                page,
                                flags,
                                Ptr::new(mixed),
                                8,
                                out,
                                size,
                                Ptr::new(0),
                                Ptr::new(used),
                            )
                        } else {
                            super::wide_char_to_multi_byte(
                                e,
                                page,
                                flags,
                                Ptr::new(mixed),
                                8,
                                out,
                                size,
                                Ptr::new(0),
                                Ptr::new(used),
                            )
                        };
                        (k, e.kernel.last_error, e.mem.u32(used))
                    };
                    assert_eq!(
                        call(&mut e, true),
                        call(&mut e, false),
                        "page {page} flags {flags:#x} size {size}"
                    );
                }
            }
        }
    }
}

/// The forwarded calls: Windows' answers reach the guest's memory, a failure's error its last
/// error. Code pages are named, so the machine's own does not change the answers.
#[cfg(all(test, windows))]
mod tests_forwarded {
    use super::*;

    fn guest_wide_text(e: &mut Engine, units: &[u16]) -> u32 {
        let at = e.mem.alloc(2 * units.len() as u32 + 2);
        for (i, u) in units.iter().enumerate() {
            e.mem.set_u16(at + 2 * i as u32, *u);
        }
        at
    }

    fn wide_out(e: &Engine, at: u32, count: u32) -> Vec<u16> {
        (0..count).map(|i| e.mem.u16(at + 2 * i)).collect()
    }

    #[test]
    fn multi_byte_answers_windows_table_for_the_named_page() {
        let mut e = Engine::new();
        let text = e.mem.alloc(8);
        e.mem.write(text, b"A\x80\x81\xe9\0\xff");
        let out = e.mem.alloc(16);
        e.kernel.last_error = 0x1234;
        let call = |e: &mut Engine, count: i32, size: i32| {
            multi_byte_to_wide_char(e, 1252, 1, Ptr::new(text), count, Ptr::new(out), size)
        };
        assert_eq!(call(&mut e, 4, 0), 4, "the size alone");
        assert_eq!(e.mem.u16(out), 0);
        assert_eq!(call(&mut e, 4, 8), 4);
        assert_eq!(wide_out(&e, out, 5), [0x41, 0x20ac, 0x81, 0xe9, 0]);
        assert_eq!(e.kernel.last_error, 0x1234, "left as it was");
        // A negative count (-2 as -1) runs through the zero, which is counted; the rest stays.
        for count in [-1, -2] {
            e.mem.write(out, &[0xaa; 16]);
            assert_eq!(call(&mut e, count, 8), 5);
            assert_eq!(wide_out(&e, out, 6), [0x41, 0x20ac, 0x81, 0xe9, 0, 0xaaaa]);
        }
        // Too small: filled as far as it goes.
        e.mem.write(out, &[0xaa; 16]);
        assert_eq!(call(&mut e, 4, 2), 0);
        assert_eq!(e.kernel.last_error, 122);
        assert_eq!(wide_out(&e, out, 3), [0x41, 0x20ac, 0xaaaa]);
        // A size far above what is mapped there: only what it needs is touched.
        assert_eq!(call(&mut e, 4, i32::MAX), 4);
        // Windows' own refusals.
        assert_eq!(call(&mut e, 0, 8), 0);
        assert_eq!(e.kernel.last_error, 87);
        assert_eq!(
            multi_byte_to_wide_char(&mut e, 1252, 0x8000, Ptr::new(text), 4, Ptr::new(out), 8),
            0
        );
        assert_eq!(e.kernel.last_error, 1004);
        assert_eq!(
            multi_byte_to_wide_char(&mut e, 1252, 1, Ptr::new(0), 4, Ptr::new(out), 8),
            0
        );
        assert_eq!(e.kernel.last_error, 87);
    }

    #[test]
    fn multi_byte_reads_the_page_it_is_given() {
        let mut e = Engine::new();
        let text = e.mem.alloc(4);
        e.mem.write(text, b"\x83\x5c");
        let out = e.mem.alloc(8);
        // Two bytes, one character, in Japanese.
        assert_eq!(
            multi_byte_to_wide_char(&mut e, 932, 1, Ptr::new(text), 2, Ptr::new(out), 4),
            1
        );
        assert_eq!(e.mem.u16(out), 0x30bd);
    }

    #[test]
    fn wide_to_multi_byte_answers_windows_table_for_the_named_page() {
        let mut e = Engine::new();
        let text = guest_wide_text(&mut e, &[0x41, 0x20ac, 0xe9, 0x416]);
        let out = e.mem.alloc(8);
        let used = e.mem.alloc(4);
        e.mem.set_u32(used, 7);
        let call = |e: &mut Engine, page: u32, default: u32, used: u32| {
            wide_char_to_multi_byte(
                e,
                page,
                0,
                Ptr::new(text),
                4,
                Ptr::new(out),
                8,
                Ptr::new(default),
                Ptr::new(used),
            )
        };
        assert_eq!(call(&mut e, 1252, 0, used), 4);
        assert_eq!(e.mem.bytes(out, 4), b"A\x80\xe9?".to_vec());
        assert_eq!(e.mem.u32(used), 1, "a character outside the page");
        let star = e.mem.alloc(2);
        e.mem.write(star, b"*\0");
        assert_eq!(call(&mut e, 1252, star, 0), 4);
        assert_eq!(e.mem.bytes(out, 4), b"A\x80\xe9*".to_vec());
        assert_eq!(call(&mut e, 65001, 0, 0), 8);
        assert_eq!(e.mem.bytes(out, 8), "A\u{20ac}\u{e9}\u{416}".as_bytes());
    }

    #[test]
    fn lc_map_maps_and_counts_what_it_is_asked_for() {
        let mut e = Engine::new();
        let text = guest_wide_text(&mut e, &[0x41, 0xc9, 0x416]);
        let out = e.mem.alloc(16);
        const ENGLISH: u32 = 0x409;
        // The probe the C runtime makes: one unit of the empty text.
        let empty = guest_wide_text(&mut e, &[0]);
        assert_eq!(
            lc_map_string_w(&mut e, 0, 0x100, Ptr::new(empty), 1, Ptr::new(0), 0),
            1
        );
        let call = |e: &mut Engine, flags: u32, size: i32| {
            lc_map_string_w(e, ENGLISH, flags, Ptr::new(text), 3, Ptr::new(out), size)
        };
        assert_eq!(call(&mut e, 0x100, 0), 3);
        assert_eq!(call(&mut e, 0x100, 8), 3);
        assert_eq!(wide_out(&e, out, 3), [0x61, 0xe9, 0x436]);
        let upper = guest_wide_text(&mut e, &[0xff, 0xdf]);
        assert_eq!(
            lc_map_string_w(&mut e, ENGLISH, 0x200, Ptr::new(upper), 2, Ptr::new(out), 8),
            2
        );
        assert_eq!(wide_out(&e, out, 2), [0x178, 0xdf]);
        // A sort key is bytes.
        let size = call(&mut e, 0x400, 0);
        assert!(size > 0);
        let key = e.mem.alloc(size + 2);
        e.mem.write(key + size, &[0xaa, 0xaa]);
        assert_eq!(
            lc_map_string_w(
                &mut e,
                ENGLISH,
                0x400,
                Ptr::new(text),
                3,
                Ptr::new(key),
                size as i32 + 2
            ),
            size
        );
        assert_eq!(e.mem.u8(key + size - 1), 0, "ends in its zero");
        assert_eq!(e.mem.u8(key + size), 0xaa);
        // Too small, and Windows' own refusals.
        assert_eq!(call(&mut e, 0x100, 1), 0);
        assert_eq!(e.kernel.last_error, 122);
        assert_eq!(call(&mut e, 0, 8), 0);
        assert_eq!(e.kernel.last_error, 1004);
    }
}

/// The C runtime's start-up step `__initmbctable` makes (`__setmbcp(_MB_CP_ANSI)`), run through
/// the exe's translated functions on the real Windows conversions: nothing here is doubled but
/// the thread block's lookup, the allocator and the locks.
#[cfg(all(test, windows))]
mod tests_start_up {
    use crate::exe::Import;
    use crate::prelude::*;

    #[link(name = "kernel32")]
    extern "system" {
        fn IsValidCodePage(page: u32) -> i32;
        fn GetCPInfo(page: u32, info: *mut u8) -> i32;
        fn LoadLibraryA(name: *const u8) -> *mut std::ffi::c_void;
        fn GetProcAddress(module: *mut std::ffi::c_void, name: *const u8) -> *mut std::ffi::c_void;
    }

    /// `__setmbcp`, `_mbctolower`, `_ismbblead`, `_mbsrchr`, `_malloc_crt`, `free`, `_lock`,
    /// `_unlock` and the function that gives `FlsGetValue`'s address.
    const SETMBCP: u32 = 0x00ed_3fe4;
    const MBCTOLOWER: u32 = 0x00ed_761c;
    const ISMBBLEAD: u32 = 0x00ed_5c8d;
    const MBSRCHR: u32 = 0x00ee_248c;
    const STRRCHR: u32 = 0x00ec_6e30;
    const MALLOC_CRT: u32 = 0x00ed_0cdf;
    const FREE: u32 = 0x00ec_d291;
    const LOCK: u32 = 0x00ed_df39;
    const UNLOCK: u32 = 0x00ed_de5f;
    const SET_FLS_GET_VALUE: u32 = 0x00ed_100d;
    /// A function no exe address uses: `FlsGetValue` here.
    const FLS_GET_VALUE: u32 = 0x0e40_0000;
    /// `__initialmbcinfo`, `__ptmbcinfo`, `__ptlocinfo`, `__mbcodepage`, `__ismbcodepage`,
    /// `_mbctype` and `_mbcasemap`.
    const INITIAL_MBCINFO: u32 = 0x011c_1df0;
    const GLOBAL_MBCINFO: u32 = 0x011c_2218;
    const GLOBAL_LOCALE_INFO: u32 = 0x011c_23f8;
    const MB_CODEPAGE: u32 = 0x0127_0478;
    const IS_MB_CODEPAGE: u32 = 0x0127_047c;
    const MB_CTYPE: u32 = 0x011c_2010;
    const MB_CASEMAP: u32 = 0x011c_2118;

    /// The machine table's code page lines as `d3d9-capture codepages` writes them.
    fn page_lines(page: u32) -> String {
        let mut info = [0u8; 20];
        // SAFETY: plain queries; `info` is a 20-byte buffer as `CPINFO` is.
        let (valid, answer) =
            unsafe { (IsValidCodePage(page), GetCPInfo(page, info.as_mut_ptr())) };
        let hex: String = info.iter().map(|b| format!("{b:02x}")).collect();
        format!("IsValidCodePage\t{page:x}\t{valid:x}\nGetCPInfo\t{page:x}\t{answer:x}\t{hex}\n")
    }

    /// An engine as the exe's start-up leaves it before `__initmbctable`, on a machine whose
    /// ANSI code page is `ansi`: the initial multibyte info with its "C" tables, one thread
    /// block on it. Answers the engine and the thread block.
    fn engine(ansi: u32) -> (Engine, u32) {
        let mut e = Engine::new();
        e.map(0x0101_1000, 0x1000);
        e.map(0x011c_1000, 0x2000);
        e.map(0x0126_f000, 0x2000);
        let slots = [
            (0x00fd_f1e4, "GetACP"),
            (0x00fd_f1f0, "IsValidCodePage"),
            (0x00fd_f1d8, "GetCPInfo"),
            (0x00fd_f128, "MultiByteToWideChar"),
            (0x00fd_f170, "WideCharToMultiByte"),
            (0x00fd_f1d4, "LCMapStringW"),
            (0x00fd_f230, "GetStringTypeW"),
            (0x00fd_f094, "GetLastError"),
            (0x00fd_f1dc, "SetLastError"),
            (0x00fd_f1c0, "InterlockedIncrement"),
            (0x00fd_f1c4, "InterlockedDecrement"),
        ];
        let imports: Vec<Import> = slots
            .iter()
            .map(|(slot, name)| Import {
                slot: *slot,
                dll: "KERNEL32.dll".into(),
                name: (*name).into(),
            })
            .collect();
        e.register_imports(&imports);
        let table = format!("GetACP\t\t{ansi:x}\n{}", page_lines(ansi));
        crate::d3d9::load_adapter_table(&mut e, &table);
        let thread = e.mem.alloc(0x214);
        let locale = e.mem.alloc(0x100);
        e.mem.set_u32(thread + 0x68, INITIAL_MBCINFO);
        e.mem.set_u32(thread + 0x6c, locale);
        e.mem.set_u32(GLOBAL_LOCALE_INFO, locale);
        e.mem.set_u32(GLOBAL_MBCINFO, INITIAL_MBCINFO);
        e.mem.set_u32(INITIAL_MBCINFO, 1);
        for c in b'A'..=b'Z' {
            let (upper, lower) = (u32::from(c), u32::from(c.to_ascii_lowercase()));
            e.mem.set_u8(INITIAL_MBCINFO + 0x1d + upper, 0x10);
            e.mem.set_u8(INITIAL_MBCINFO + 0x11d + upper, lower as u8);
            e.mem.set_u8(INITIAL_MBCINFO + 0x1d + lower, 0x20);
            e.mem.set_u8(INITIAL_MBCINFO + 0x11d + lower, upper as u8);
        }
        e.register(SET_FLS_GET_VALUE, |_, _| FLS_GET_VALUE.into_ret());
        e.register_double(FLS_GET_VALUE, move |_, _| thread.into_ret());
        e.register(MALLOC_CRT, |e, a| e.mem.alloc(a[0]).into_ret());
        e.register(FREE, |_, _| Ret::default());
        e.register(LOCK, |_, _| Ret::default());
        e.register(UNLOCK, |_, _| Ret::default());
        (e, thread)
    }

    /// Code page 1252: what `__setmbcp` gives from Windows' answers, and the one function whose
    /// answer it changes on a single-byte page.
    #[test]
    fn the_western_page_fills_the_tables_from_windows() {
        let (mut e, thread) = engine(1252);
        assert_eq!(
            e.call(MBCTOLOWER, &[0xc0]).u32(),
            0xc0,
            "before: letters only"
        );
        assert_eq!(e.call(SETMBCP, &[(-3i32) as u32]).i32(), 0);
        let info = e.mem.u32(GLOBAL_MBCINFO);
        assert_ne!(info, INITIAL_MBCINFO);
        assert_eq!(e.mem.u32(thread + 0x68), info);
        assert_eq!(e.mem.u32(MB_CODEPAGE), 1252);
        assert_eq!(e.mem.u32(IS_MB_CODEPAGE), 0);
        let tables = |e: &Engine, byte: u32| {
            (
                e.mem.u8(MB_CTYPE + 1 + byte),
                e.mem.u8(MB_CASEMAP + byte),
                e.mem.u8(info + 0x1d + byte),
                e.mem.u8(info + 0x11d + byte),
            )
        };
        for (byte, flag, map) in [
            (0x41, 0x10, 0x61),
            (0x61, 0x20, 0x41),
            (0x31, 0, 0),
            // A grave, a grave small, S caron, y diaeresis (the last two outside Latin-1).
            (0xc0, 0x10, 0xe0),
            (0xe0, 0x20, 0xc0),
            (0x8a, 0x10, 0x9a),
            (0xff, 0x20, 0x9f),
            (0x80, 0, 0),
        ] {
            assert_eq!(tables(&e, byte), (flag, map, flag, map), "byte {byte:#x}");
        }
        assert_eq!(e.call(MBCTOLOWER, &[0xc0]).u32(), 0xe0);
        assert_eq!(e.call(MBCTOLOWER, &[0x41]).u32(), 0x61);
        assert_eq!(e.call(ISMBBLEAD, &[0x81]).u32(), 0, "no lead bytes");
    }

    /// Microsoft's own runtime on this machine (`ucrtbase.dll`, the later form of the C runtime
    /// the exe links): `_setmbcp(page)`, then its `_mbctype` (past the end-of-file entry) and
    /// `_mbcasemap`.
    fn microsoft_tables(page: u32) -> (Vec<u8>, Vec<u8>) {
        type SetMbcp = unsafe extern "C" fn(i32) -> i32;
        type Table = unsafe extern "C" fn() -> *const u8;
        // SAFETY: the three exports have these signatures; the two tables are 257 and 256 bytes.
        unsafe {
            let dll = LoadLibraryA(c"ucrtbase.dll".as_ptr().cast());
            assert!(!dll.is_null());
            let proc = |name: &std::ffi::CStr| {
                let f = GetProcAddress(dll, name.as_ptr().cast());
                assert!(!f.is_null(), "{name:?}");
                f
            };
            let set: SetMbcp = std::mem::transmute(proc(c"_setmbcp"));
            let ctype: Table = std::mem::transmute(proc(c"__p__mbctype"));
            let casemap: Table = std::mem::transmute(proc(c"__p__mbcasemap"));
            assert_eq!(set(page as i32), 0);
            (
                std::slice::from_raw_parts(ctype().add(1), 256).to_vec(),
                std::slice::from_raw_parts(casemap(), 256).to_vec(),
            )
        }
    }

    /// Every byte of the tables on the single-byte pages of Western, Central European, Russian,
    /// Greek and Turkish Windows, against a second implementation of the same start-up code.
    #[test]
    fn single_byte_tables_match_microsofts_runtime() {
        for page in [1252, 1250, 1251, 1253, 1254] {
            let (mut e, _) = engine(page);
            let before = e.mem.bytes(INITIAL_MBCINFO + 0x1d, 0x200);
            assert_eq!(e.call(SETMBCP, &[(-3i32) as u32]).i32(), 0);
            let ctype = e.mem.bytes(MB_CTYPE + 1, 256);
            let casemap = e.mem.bytes(MB_CASEMAP, 256);
            let (their_ctype, their_casemap) = microsoft_tables(page);
            assert_eq!(ctype, their_ctype, "_mbctype, code page {page}");
            assert_eq!(casemap, their_casemap, "_mbcasemap, code page {page}");
            let changed = (0..256).filter(|&i| before[i] != ctype[i]).count();
            let mapped = (0..256)
                .filter(|&i| before[0x100 + i] != casemap[i])
                .count();
            println!("code page {page}: {changed} type bytes, {mapped} map bytes differ from C");
            assert!(changed > 0 && mapped > 0);
            assert_eq!(e.mem.u32(IS_MB_CODEPAGE), 0, "single-byte");
        }
    }

    /// Code page 932 (Japanese Windows): lead bytes, and a path whose second byte is a backslash
    /// is one character, as the original reads it; before the step it is cut in two.
    #[test]
    fn a_double_byte_page_reads_a_trail_backslash_as_part_of_its_character() {
        let (mut e, _) = engine(932);
        // `strrchr`: its translation is in another crate.
        e.register(STRRCHR, |e, a| {
            let text = e.mem.cstr(a[0]);
            let found = text.iter().rposition(|b| u32::from(*b) == a[1]);
            found.map_or(0, |i| a[0] + i as u32).into_ret()
        });
        let path = e.mem.alloc(16);
        e.mem.write(path, b"C:\\\x83\x5ca\0");
        assert_eq!(e.call(ISMBBLEAD, &[0x83]).u32(), 0);
        assert_eq!(e.call(MBSRCHR, &[path, 0x5c]).u32(), path + 4);
        assert_eq!(e.call(SETMBCP, &[(-3i32) as u32]).i32(), 0);
        assert_eq!(e.mem.u32(MB_CODEPAGE), 932);
        assert_eq!(e.mem.u32(IS_MB_CODEPAGE), 1);
        assert_ne!(e.call(ISMBBLEAD, &[0x83]).u32(), 0);
        assert_eq!(e.call(ISMBBLEAD, &[0x41]).u32(), 0);
        assert_eq!(e.call(MBSRCHR, &[path, 0x5c]).u32(), path + 2);
    }

    /// The system-wide UTF-8 option: the exe's `__setmbcp` refuses the page, the tables stay
    /// the "C" ones and `errno` says so.
    #[test]
    fn the_utf8_system_page_leaves_the_tables_as_they_were() {
        let (mut e, thread) = engine(65001);
        assert_eq!(e.call(SETMBCP, &[(-3i32) as u32]).i32(), -1);
        assert_eq!(e.mem.u32(GLOBAL_MBCINFO), INITIAL_MBCINFO);
        assert_eq!(e.mem.u32(MB_CODEPAGE), 0);
        assert_eq!(e.mem.u32(thread + 8), 0x16);
    }
}
