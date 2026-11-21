//! The text tests the expression evaluator gives each operand token before it reads it as a
//! compiled operand: is it a decimal integer, is it a decimal float. Game-wide helpers (the console
//! and the dialogue's conditions call them too), so every caller by address reaches these.

use engine_core::ptr::Ptr;
use engine_core::Engine;

#[inline]
fn is_digit(c: i8) -> bool {
    (0x30..=0x39).contains(&c)
}

// Typed from 004b1230
/// `IsAnInteger`: whether the text is an optional `-` and digits (a lone `-` or an empty text is
/// not).
pub fn is_an_integer(e: &mut Engine, text: Ptr) -> bool {
    let at = |i: u32| e.mem.i8(text.addr().wrapping_add(i));
    let first = at(0);
    if !is_digit(first) && first != b'-' as i8 {
        return false;
    }
    if at(1) == 0 && first == b'-' as i8 {
        return false;
    }
    let mut i = 1;
    loop {
        let c = at(i);
        if c == 0 {
            return true;
        }
        if !is_digit(c) {
            return false;
        }
        i += 1;
    }
}

// Typed from 004b12d0
/// `IsAFloat`: whether the text is digits with at most one `.` and an optional leading `-` (a
/// lone `-` or `.`, or an empty text, is not).
pub fn is_a_float(e: &mut Engine, text: Ptr) -> bool {
    let at = |i: u32| e.mem.i8(text.addr().wrapping_add(i));
    let first = at(0);
    if first == 0 {
        return false;
    }
    if !is_digit(first) && first != b'-' as i8 && first != b'.' as i8 {
        return false;
    }
    if at(1) == 0 && (first == b'-' as i8 || first == b'.' as i8) {
        return false;
    }
    let mut seen_point = first == b'.' as i8;
    let mut i = 1;
    loop {
        let c = at(i);
        if c == 0 {
            return true;
        }
        if !is_digit(c) && (c != b'.' as i8 || seen_point) {
            return false;
        }
        if c == b'.' as i8 {
            seen_point = true;
        }
        i += 1;
    }
}
