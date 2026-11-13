//! Tests of the challenge commands, at the raw offsets of the challenge loader (`005F51A0`:
//! `DATA` at +0x54: type, threshold +0x58, flags +0x5C, interval +0x60, values +0x64/+0x66
//! (16 bits) and +0x68) and the forms at +0x74/+0x78 (JIP's header).

use super::*;
use crate::commands::test_support::*;

const CHAL: u32 = FORMS + 0x1000;
const OTHER: u32 = FORMS + 0x2000;
const MISC: u32 = FORMS + 0x3000;

fn setup() -> Engine {
    let mut e = engine();
    form(&mut e, CHAL, 0x71, 0x0000_c4a1);
    form(&mut e, OTHER, 0x36, 0x0000_0036);
    form(&mut e, MISC, 0x1f, 0x0000_0999);
    for (i, f) in [CHAL, OTHER, MISC].iter().enumerate() {
        set_ref(&mut e, i as u16 + 1, *f);
    }
    e
}

#[test]
fn the_type() {
    let mut e = setup();
    e.mem.set_u32(CHAL + 0x54, 13);
    assert_eq!(call(&mut e, 0x22c2, &[r(1)]).1, 13.0);
    assert_eq!(
        call(&mut e, 0x22c2, &[r(3)]).1,
        -1.0,
        "the reader refuses a misc item: -1"
    );
}

#[test]
fn the_words_each_in_place() {
    let mut e = setup();
    for (get, set, off) in [
        (0x231a, 0x231b, 0x5cusize),
        (0x22c3, 0x22c4, 0x58),
        (0x22c5, 0x22c6, 0x60),
        (0x22cb, 0x22cc, 0x68),
    ] {
        e.mem.set_u32(CHAL + off as u32, 0xffff_fff0);
        assert_eq!(call(&mut e, get, &[r(1)]).1, -16.0, "{get:04x}: (int)");
        let mut want = e.mem.bytes(CHAL, 0x7c);
        call(&mut e, set, &[r(1), n(0x1234_5678)]);
        want[off..off + 4].copy_from_slice(&0x1234_5678u32.to_le_bytes());
        assert_eq!(e.mem.bytes(CHAL, 0x7c), want, "{set:04x}: only its word");
    }
}

#[test]
fn the_16_bit_values() {
    let mut e = setup();
    for (get, set, off) in [(0x22c7, 0x22c8, 0x64usize), (0x22c9, 0x22ca, 0x66)] {
        e.mem.set_u16(CHAL + off as u32, 0xffff);
        assert_eq!(call(&mut e, get, &[r(1)]).1, 65535.0, "{get:04x}: unsigned");
        let mut want = e.mem.bytes(CHAL, 0x7c);
        call(&mut e, set, &[r(1), n(0x0001_0302)]);
        want[off..off + 2].copy_from_slice(&0x0302u16.to_le_bytes());
        assert_eq!(
            e.mem.bytes(CHAL, 0x7c),
            want,
            "{set:04x}: the low half only"
        );
    }
}

#[test]
fn the_forms() {
    let mut e = setup();
    for (get, set, off) in [(0x22cd, 0x22ce, 0x74), (0x22cf, 0x22d0, 0x78)] {
        call(&mut e, get, &[r(1)]);
        assert_eq!(form_result(&e), 0, "{get:04x}: none");
        call(&mut e, set, &[r(1), r(2)]);
        assert_eq!(e.mem.u32(CHAL + off), OTHER, "{set:04x}: any form");
        call(&mut e, get, &[r(1)]);
        assert_eq!(form_result(&e), 0x36);
    }
}

/// The challenge is read as `kParamType_Challenge`: a misc item holding the same bytes is refused
/// by xNVSE's reader, so every command answers 0 (-1 for the type) and leaves its bytes.
#[test]
fn another_form_type_is_left_alone() {
    let mut e = setup();
    for off in (0x54..0x7c).step_by(4) {
        e.mem.set_u32(MISC + off, OTHER);
    }
    let before = e.mem.bytes(MISC, 0x7c);
    for get in [0x231a, 0x22c3, 0x22c5, 0x22c7, 0x22c9, 0x22cb] {
        assert_eq!(call(&mut e, get, &[r(3)]).1, 0.0, "{get:04x}");
    }
    for get in [0x22cd, 0x22cf] {
        call(&mut e, get, &[r(3)]);
        assert_eq!(form_result(&e), 0, "{get:04x}");
    }
    for set in [0x231b, 0x22c4, 0x22c6, 0x22c8, 0x22ca, 0x22cc] {
        call(&mut e, set, &[r(3), n(5)]);
    }
    for set in [0x22ce, 0x22d0] {
        call(&mut e, set, &[r(3), r(1)]);
    }
    assert_eq!(e.mem.bytes(MISC, 0x7c), before);
}
