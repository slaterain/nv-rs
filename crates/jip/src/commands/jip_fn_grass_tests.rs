//! Tests of the grass commands, at the raw offsets the grass loader (`00509F80`) gives (model
//! +0x30, `DATA` from +0x48: density, slopes, water distance +0x4C, its type +0x50, ranges
//! +0x54..+0x60, flags +0x64). The casts are worked out from JIP's source as MSVC compiles it:
//! `int iVal = fVal` (`INT_MIN` for NaN and 2^31 up) and `GetMin`/`GetMax` answering their first
//! argument's type.

use super::*;
use crate::commands::test_support::*;

const GRASS: u32 = FORMS + 0x1000;
const OTHER: u32 = FORMS + 0x2000;

fn setup() -> Engine {
    let mut e = engine();
    form(&mut e, GRASS, 0x24, 0x0000_9a55);
    form(&mut e, OTHER, 0x36, 0x0000_0036);
    model(&mut e, GRASS + 0x30, Some(b"grass.nif"));
    set_ref(&mut e, 1, GRASS);
    set_ref(&mut e, 2, OTHER);
    e
}

#[test]
fn the_numbers_at_their_offsets() {
    let mut e = setup();
    for (off, v) in [(0x48, 60u8), (0x49, 5), (0x4a, 40), (0x50, 3), (0x64, 6)] {
        e.mem.set_u8(GRASS + off, v);
    }
    e.mem.set_u16(GRASS + 0x4c, 1234);
    for (i, v) in [1.5f32, 2.5, 0.125, 9.0].iter().enumerate() {
        e.mem.set_f32(GRASS + 0x54 + 4 * i as u32, *v);
    }
    let get = |e: &mut Engine, t: i32| call(e, 0x2724, &[r(1), n(t)]).1;
    let want = [60.0, 5.0, 40.0, 1234.0, 3.0, 1.5, 2.5, 0.125, 9.0, 6.0];
    for (t, v) in want.iter().enumerate() {
        assert_eq!(get(&mut e, t as i32), *v, "trait {t}");
    }
    assert_eq!(get(&mut e, 10), 0.0);
    assert_eq!(call(&mut e, 0x2724, &[r(2), n(0)]).1, 0.0, "not a grass");
}

#[test]
fn the_numbers_set_with_jips_casts() {
    let mut e = setup();
    let set = |e: &mut Engine, t: i32, v: f64| {
        call(e, 0x2725, &[r(1), n(t), z(v)]);
    };
    set(&mut e, 0, 150.9);
    assert_eq!(e.mem.u8(GRASS + 0x48), 100, "GetMin(150, 100)");
    set(&mut e, 0, 42.9);
    assert_eq!(e.mem.u8(GRASS + 0x48), 42);
    set(&mut e, 0, 3e9);
    assert_eq!(
        e.mem.u8(GRASS + 0x48),
        0,
        "INT_MIN is below 100: its low byte"
    );
    set(&mut e, 0, f64::NAN);
    assert_eq!(
        e.mem.u8(GRASS + 0x48),
        0,
        "NaN passes fVal < 0, then INT_MIN"
    );
    e.mem.set_u8(GRASS + 0x48, 9);
    set(&mut e, 0, -1.0);
    assert_eq!(e.mem.u8(GRASS + 0x48), 9, "a negative value: nothing");
    // Minimum slope: GetMin(maxSlope, iVal), a byte.
    e.mem.set_u8(GRASS + 0x4a, 40);
    set(&mut e, 1, 50.0);
    assert_eq!(e.mem.u8(GRASS + 0x49), 40);
    set(&mut e, 1, 30.7);
    assert_eq!(e.mem.u8(GRASS + 0x49), 30);
    // Maximum slope: GetMax(minSlope, GetMin(iVal, 90)).
    e.mem.set_u8(GRASS + 0x49, 20);
    set(&mut e, 2, 95.0);
    assert_eq!(e.mem.u8(GRASS + 0x4a), 90);
    set(&mut e, 2, 10.0);
    assert_eq!(e.mem.u8(GRASS + 0x4a), 20);
    set(&mut e, 3, 70000.0);
    assert_eq!(e.mem.u16(GRASS + 0x4c), 0xffff);
    set(&mut e, 3, 3e9);
    assert_eq!(e.mem.u16(GRASS + 0x4c), 0, "INT_MIN's low word");
    set(&mut e, 4, 9.9);
    assert_eq!(e.mem.u8(GRASS + 0x50), 7);
    set(&mut e, 6, 2.75);
    assert_eq!(e.mem.f32(GRASS + 0x58), 2.75);
    set(&mut e, 9, 5.5);
    assert_eq!(e.mem.u8(GRASS + 0x64), 5);
    let before = e.mem.bytes(GRASS, 0x68);
    set(&mut e, 10, 1.0);
    assert_eq!(e.mem.bytes(GRASS, 0x68), before, "trait 10: nothing");
}

#[test]
fn the_model() {
    let mut e = setup();
    let (_, res) = call(&mut e, 0x2726, &[r(1)]);
    assert_eq!(string_result(&e, res), b"grass.nif");
    call(&mut e, 0x2727, &[r(1), s(b"other.nif")]);
    assert_eq!(text_at(&e, GRASS + 0x34), b"other.nif");
    let (_, res) = call(&mut e, 0x2726, &[r(2)]);
    assert_eq!(string_result(&e, res), b"");
}
