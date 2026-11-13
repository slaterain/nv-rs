//! Tests of the combat style commands. Each trait's offset is worked out here from JIP's own
//! expressions (`((float*)cStyle)[6 + t]` and the rest), not from the layout, and written at that
//! raw address. Casts as MSVC compiles JIP's source.

use super::*;
use crate::commands::test_support::*;

const CSTY: u32 = FORMS + 0x1000;

fn setup() -> Engine {
    let mut e = engine();
    form(&mut e, CSTY, 0x4a, 0x0000_c571);
    set_ref(&mut e, 1, CSTY);
    e
}

/// JIP's address of trait `t` and whether it is a byte.
fn jip_offset(t: u32) -> (u32, bool) {
    match t {
        0..=8 => (4 * (6 + t), false),
        9 => (0x40, true),
        10..=14 => (4 * (7 + t), false),
        15 | 16 => (0x49 + t, true),
        17..=24 => (4 * (6 + t), false),
        25 | 26 => (0x63 + t, true),
        27..=29 | 31 | 32 => (4 * (5 + t), false),
        30 => (0x8c, true),
        33..=37 => (0x77 + t, true),
        38 | 39 => (4 * (2 + t), false),
        40 | 41 => (0x84 + t, true),
        _ => (4 * (2 + t), false),
    }
}

#[test]
fn every_trait_at_jips_offset() {
    let mut e = setup();
    for t in 0..=63u32 {
        let (off, byte) = jip_offset(t);
        assert!(off + 4 <= 0x108, "trait {t} inside the form");
        let get = |e: &mut Engine| call(e, 0x2630, &[r(1), n(t as i32)]).1;
        let set = |e: &mut Engine, v: f64| {
            call(e, 0x2631, &[r(1), n(t as i32), z(v)]);
        };
        if byte {
            e.mem.set_u8(CSTY + off, 2);
            assert_eq!(get(&mut e), 2.0, "trait {t} read");
            set(&mut e, 1.0);
            assert_eq!(e.mem.u8(CSTY + off), 1, "trait {t} written");
        } else {
            e.mem.set_f32(CSTY + off, t as f32 + 0.5);
            assert_eq!(get(&mut e), f64::from(t) + 0.5, "trait {t} read");
            set(&mut e, -3.25);
            assert_eq!(e.mem.f32(CSTY + off), -3.25, "trait {t} written");
        }
    }
    let before = e.mem.bytes(CSTY, 0x108);
    assert_eq!(call(&mut e, 0x2630, &[r(1), n(64)]).1, 0.0, "trait 64");
    call(&mut e, 0x2631, &[r(1), n(64), z(1.0)]);
    assert_eq!(e.mem.bytes(CSTY, 0x108), before, "trait 64: nothing");
}

#[test]
fn bytes_narrowed_as_jip_does() {
    let mut e = setup();
    let set = |e: &mut Engine, t: i32, v: f64| {
        call(e, 0x2631, &[r(1), n(t), z(v)]);
    };
    // bVal = (int)fVal, its low byte.
    set(&mut e, 15, 99.9);
    assert_eq!(e.mem.u8(CSTY + 0x58), 99, "toward zero");
    set(&mut e, 15, -1.5);
    assert_eq!(e.mem.u8(CSTY + 0x58), 0xff, "-1's low byte");
    set(&mut e, 15, 256.0 + 7.0);
    assert_eq!(e.mem.u8(CSTY + 0x58), 7);
    set(&mut e, 15, f64::NAN);
    assert_eq!(e.mem.u8(CSTY + 0x58), 0, "NaN: INT_MIN's low byte");
    set(&mut e, 15, 3e9);
    assert_eq!(
        e.mem.u8(CSTY + 0x58),
        0,
        "past the range: INT_MIN's low byte"
    );
    // Weapon restrictions: only 1 or 2.
    e.mem.set_u8(CSTY + 0x40, 1);
    set(&mut e, 9, 2.7);
    assert_eq!(e.mem.u8(CSTY + 0x40), 2);
    set(&mut e, 9, 3.0);
    assert_eq!(e.mem.u8(CSTY + 0x40), 2, "3 refused");
    set(&mut e, 9, 0.5);
    assert_eq!(e.mem.u8(CSTY + 0x40), 2, "0 refused");
    set(&mut e, 9, 257.0);
    assert_eq!(e.mem.u8(CSTY + 0x40), 1, "257's low byte is 1");
    // A float trait keeps the float as it is (NaN included).
    set(&mut e, 0, f64::NAN);
    assert!(e.mem.f32(CSTY + 0x18).is_nan());
}

#[test]
fn the_flags() {
    let mut e = setup();
    let get = |e: &mut Engine, f: i32| call(e, 0x2632, &[r(1), n(f)]).1;
    let set = |e: &mut Engine, f: i32, v: i32| {
        call(e, 0x2633, &[r(1), n(f), n(v)]);
    };
    e.mem.set_u32(CSTY + 0xa8, 0xffff_8001);
    assert_eq!(get(&mut e, 0), 1.0);
    assert_eq!(get(&mut e, 15), 1.0);
    assert_eq!(get(&mut e, 1), 0.0);
    assert_eq!(get(&mut e, 16), 0.0, "flag 16 is past the word");
    set(&mut e, 3, 1);
    assert_eq!(e.mem.u32(CSTY + 0xa8), 0xffff_8009, "one word written");
    set(&mut e, 15, 0);
    assert_eq!(e.mem.u32(CSTY + 0xa8), 0xffff_0009);
    set(&mut e, 16, 0);
    assert_eq!(e.mem.u32(CSTY + 0xa8), 0xffff_0009, "flag 16: nothing");
}

/// The style is read as `kParamType_CombatStyle`: a misc item holding the same bytes is refused
/// by xNVSE's reader, so every command answers 0 and leaves its bytes.
#[test]
fn another_form_type_is_left_alone() {
    let mut e = setup();
    const MISC: u32 = FORMS + 0x3000;
    let bytes = e.mem.bytes(CSTY, 0x108);
    e.mem.write(MISC, &bytes);
    form(&mut e, MISC, 0x1f, 0x0000_0999);
    for i in 0x18..0x108 {
        e.mem.set_u8(MISC + i, 0x01);
    }
    set_ref(&mut e, 2, MISC);
    let before = e.mem.bytes(MISC, 0x108);
    for t in 0..64 {
        assert_eq!(call(&mut e, 0x2630, &[r(2), n(t)]).1, 0.0, "get {t}");
        call(&mut e, 0x2631, &[r(2), n(t), z(2.0)]);
    }
    for f in 0..16 {
        assert_eq!(call(&mut e, 0x2632, &[r(2), n(f)]).1, 0.0, "flag {f}");
        call(&mut e, 0x2633, &[r(2), n(f), n(0)]);
    }
    assert_eq!(e.mem.bytes(MISC, 0x108), before);
}
