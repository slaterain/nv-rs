//! JIP LN NVSE's own text helpers the built-in uses so far (`internal/utility.cpp` (JIP LN
//! NVSE)): its case-blind compare of two texts and its number-to-text conversions. JIP writes
//! them in SSE2 and x87 assembly; these give the same answers, byte for byte.

use engine_core::x87::{fistp, fistp_chop64, x87_sub};

/// A byte as JIP's case-blind compares see it (`kStringMasks`' range test: subtract `A`, keep
/// seven bits, at most 25): `A` to `Z`, and also 0xC1 to 0xDA, have bit 5 flipped; every other
/// byte stays.
fn fold(b: u8) -> u8 {
    if b.wrapping_sub(b'A') & 0x7f <= 25 {
        b ^ 0x20
    } else {
        b
    }
}

/// The byte at `i`, 0 past the end (the texts are compared up to their terminator).
fn at(text: &[u8], i: usize) -> u8 {
    text.get(i).copied().unwrap_or(0)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/utility.cpp: StrCompareCI.
/// Two texts compared without case as JIP compares them: 0 when they are equal, else -1 or 1 by
/// the first byte that differs once folded (compared as signed bytes), with that byte's index.
pub fn str_compare_ci(left: &[u8], right: &[u8]) -> (i8, usize) {
    let mut i = 0;
    loop {
        let (l, r) = (fold(at(left, i)), fold(at(right, i)));
        if l != r {
            return (if l as i8 > r as i8 { 1 } else { -1 }, i);
        }
        if l == 0 {
            return (0, i);
        }
        i += 1;
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/utility.cpp: StrBeginsCI.
/// Whether `left` begins with `right`, without case: 2 when they are equal, 1 when `left` goes on
/// past `right` (its next byte comparing above the terminator, so not a byte from 0x80 up), else 0.
pub fn str_begins_ci(left: &[u8], right: &[u8]) -> u8 {
    match str_compare_ci(left, right) {
        (0, _) => 2,
        (c, _) if c < 0 => 0,
        (_, i) => u8::from(at(right, i) == 0),
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/utility.cpp: IntToStr.
/// A signed integer in decimal: a minus sign, then the digits of its magnitude (`-2147483648` too).
pub fn int_to_str(num: i32) -> Vec<u8> {
    let mut out = Vec::new();
    if num < 0 {
        out.push(b'-');
    }
    let start = out.len();
    let mut rest = num.unsigned_abs();
    loop {
        out.push(b'0' + (rest % 10) as u8);
        rest /= 10;
        if rest == 0 {
            break;
        }
    }
    out[start..].reverse();
    out
}

/// `cvtsd2si` under the default MXCSR: to the nearest integer, ties to even, the integer
/// indefinite (`i32::MIN`) for NaN and past the range; the x87 `FISTP` rounds the same way.
fn cvtsd2si(value: f64) -> i32 {
    fistp(value)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/utility.cpp: FltToStr.
/// A double as JIP writes it for the console: `0` for zero and NaN; else a minus sign for a
/// negative value, the integer part, and up to six decimals with the trailing zeros left out,
/// rounded to the nearest millionth (a fraction that rounds up to a whole one carries into the
/// integer part). The integer part is the low 32 bits of the truncated value, written signed, as
/// JIP's code does (from 2^31 up it is not the value's). The fraction is the x87's subtraction
/// of the integer part, so it is rounded to the running thread's precision: on the game's main
/// thread, 24 bits once the renderer has made its device (`engine_core::x87`).
pub fn flt_to_str(value: f64) -> Vec<u8> {
    // `comisd value, 0`: equal, or unordered (NaN), writes the one digit.
    if value == 0.0 || value.is_nan() {
        return vec![b'0'];
    }
    let mut out = Vec::new();
    let mut value = value;
    if value < 0.0 {
        value = -value;
        out.push(b'-');
    }
    // `fisttp qword`, `fild`, `fsubp`, `fstp qword`: the integer part toward zero, and what is
    // left of the value.
    let int = fistp_chop64(value);
    let mut whole = int as u32;
    let fraction = x87_sub(value, int as f64);
    let mut millionths = 0i32;
    if !(fraction == 0.0 || fraction.is_nan()) {
        millionths = cvtsd2si(fraction * 1_000_000.0);
        if millionths == 1_000_000 {
            millionths = 0;
            whole = whole.wrapping_add(1);
        }
    }
    if whole != 0 {
        out.extend(int_to_str(whole as i32));
    } else {
        out.push(b'0');
    }
    if millionths != 0 {
        out.push(b'.');
        // `kIntDivisors` from the top: each digit, until what is left is 0; the last one is the
        // remainder of the division by 10.
        let mut rest = millionths as u32;
        for divisor in [100_000u32, 10_000, 1_000, 100, 10] {
            out.push(((rest / divisor) as u8).wrapping_add(b'0'));
            rest %= divisor;
            if rest == 0 {
                return out;
            }
        }
        out.push((rest as u8).wrapping_add(b'0'));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// JIP's case folding takes `A`-`Z` and 0xC1-0xDA: not lower case, digits, 0xC0 or 0xDB.
    #[test]
    fn the_case_folding() {
        assert_eq!(fold(b'A'), b'a');
        assert_eq!(fold(b'Z'), b'z');
        assert_eq!(fold(b'a'), b'a');
        assert_eq!(fold(b'@'), b'@');
        assert_eq!(fold(b'['), b'[');
        assert_eq!(fold(0xc1), 0xe1);
        assert_eq!(fold(0xda), 0xfa);
        assert_eq!(fold(0xc0), 0xc0);
        assert_eq!(fold(0xdb), 0xdb);
        assert_eq!(fold(0xe1), 0xe1);
        assert_eq!(fold(0), 0);
    }

    /// Equal without case; otherwise ordered by the first folded byte that differs, as signed
    /// bytes, with its index; a text that goes on is greater (unless its next byte is from 0x80).
    #[test]
    fn texts_compare_without_case() {
        assert_eq!(str_compare_ci(b"JIP LN NVSE", b"jip ln nvse"), (0, 11));
        assert_eq!(str_compare_ci(b"", b""), (0, 0));
        assert_eq!(str_compare_ci(b"abc", b"abd"), (-1, 2));
        assert_eq!(str_compare_ci(b"abd", b"ABC"), (1, 2));
        assert_eq!(str_compare_ci(b"abcd", b"abc"), (1, 3));
        assert_eq!(str_compare_ci(b"abc", b"abcd"), (-1, 3));
        assert_eq!(
            str_compare_ci(b"ab\xe9", b"ab"),
            (-1, 2),
            "0xE9 is a negative byte"
        );
        assert_eq!(str_compare_ci(b"\xc1", b"\xe1"), (0, 1));
        assert_eq!(str_compare_ci(b"\xc0", b"\xe0"), (-1, 0));
        // A text stops at its terminator.
        assert_eq!(str_compare_ci(b"ab\0x", b"ab\0y"), (0, 2));
    }

    #[test]
    fn a_text_begins_with_another() {
        assert_eq!(str_begins_ci(b"JIP LN NVSE", b"JIP LN NVSE"), 2);
        assert_eq!(str_begins_ci(b"jip ln nvse plugin", b"JIP LN NVSE"), 1);
        assert_eq!(str_begins_ci(b"JIP LN NVS", b"JIP LN NVSE"), 0);
        assert_eq!(str_begins_ci(b"JIP LN NVSF", b"JIP LN NVSE"), 0);
        assert_eq!(str_begins_ci(b"JIP LN NVSE\xe9", b"JIP LN NVSE"), 0);
        assert_eq!(str_begins_ci(b"JIQ", b"JIP LN NVSE"), 0);
    }

    #[test]
    fn integers_in_decimal() {
        assert_eq!(int_to_str(0), b"0");
        assert_eq!(int_to_str(5730), b"5730");
        assert_eq!(int_to_str(-12), b"-12");
        assert_eq!(int_to_str(i32::MAX), b"2147483647");
        assert_eq!(int_to_str(i32::MIN), b"-2147483648");
    }

    /// The console's numbers: up to six decimals without trailing zeros, rounded to the nearest
    /// millionth; zero and NaN as `0`; the integer part's low 32 bits, signed.
    #[test]
    fn doubles_as_jip_writes_them() {
        let s = |v: f64| String::from_utf8(flt_to_str(v)).unwrap();
        assert_eq!(s(57.0 + 30.0 * 0.01), "57.3");
        assert_eq!(s(6.0 + 4.0 * 0.1 + 9.0 * 0.01), "6.49");
        assert_eq!(s(1.0), "1");
        assert_eq!(s(-1.0), "-1");
        assert_eq!(s(0.0), "0");
        assert_eq!(s(-0.0), "0");
        assert_eq!(s(f64::NAN), "0");
        assert_eq!(s(0.5), "0.5");
        assert_eq!(s(-1.5), "-1.5");
        assert_eq!(s(0.1), "0.1");
        assert_eq!(s(123.000001), "123.000001");
        assert_eq!(s(1e-7), "0");
        assert_eq!(s(0.9999996), "1");
        assert_eq!(s(41.9999996), "42");
        assert_eq!(s(2147483647.0), "2147483647");
        assert_eq!(s(2147483648.0), "-2147483648", "the low 32 bits, signed");
        assert_eq!(s(4294967296.25), "0.25", "the low 32 bits");
    }

    /// The fraction is the x87's: under 24-bit precision (the main thread once the renderer's
    /// device is made) it is rounded to 24 bits before it is scaled, which can move the last
    /// decimal.
    #[test]
    fn the_fraction_follows_the_x87_precision() {
        struct Restore;
        impl Drop for Restore {
            fn drop(&mut self) {
                engine_core::x87::set_control_word(engine_core::x87::DEFAULT_CONTROL_WORD);
            }
        }
        let value = 7.499_500_499;
        assert_eq!(flt_to_str(value), b"7.4995");
        let _restore = Restore;
        engine_core::x87::set_control_word(0x007f);
        assert_eq!(flt_to_str(value), b"7.499501");
        assert_eq!(flt_to_str(57.0 + 30.0 * 0.01), b"57.3");
    }
}
