//! Tests of the texture set commands, at the raw offsets of the texture set loader (`005928A0`:
//! textures +0x38 in 0xC steps, flags word +0x84) and JIP's `DecalInfo` (pointer +0x80; floats
//! +0x00..+0x18, passes +0x1C, flags +0x1D, colour +0x20). Casts as MSVC compiles JIP's source.

use super::*;
use crate::commands::test_support::*;

const SET: u32 = FORMS + 0x1000;
const DECAL: u32 = FORMS + 0x2000;

fn setup() -> Engine {
    let mut e = engine();
    form(&mut e, SET, 0x04, 0x0000_7e57);
    set_ref(&mut e, 1, SET);
    e
}

#[test]
fn the_textures() {
    let mut e = setup();
    string(&mut e, SET + 0x38 + 0xc * 5 + 4, Some(b"t5.dds"));
    let (_, res) = call(&mut e, 0x2665, &[r(1), n(5)]);
    assert_eq!(string_result(&e, res), b"t5.dds");
    let (_, res) = call(&mut e, 0x2665, &[r(1), n(6)]);
    assert_eq!(string_result(&e, res), b"", "map 6");
    call(&mut e, 0x2666, &[r(1), n(0), s(b"t0.dds")]);
    assert_eq!(text_at(&e, SET + 0x3c), b"t0.dds");
    call(&mut e, 0x2666, &[r(1), n(6), s(b"x")]);
    assert_eq!(e.mem.u32(SET + 0x38 + 0xc * 6 + 4), 0, "map 6: nothing");
}

#[test]
fn the_decal_numbers() {
    let mut e = setup();
    let get = |e: &mut Engine, t: i32| call(e, 0x2667, &[r(1), n(t)]).1;
    e.mem.set_f32(DECAL + 0x18, 9.0);
    assert_eq!(get(&mut e, 6), 0.0, "no decal data");
    e.mem.set_u32(SET + 0x80, DECAL);
    e.mem.set_f32(DECAL + 0x4, 2.5);
    e.mem.set_u8(DECAL + 0x1c, 3);
    e.mem.set_u32(DECAL + 0x20, 0x0003_0201);
    assert_eq!(get(&mut e, 1), 2.5);
    assert_eq!(get(&mut e, 6), 9.0);
    assert_eq!(get(&mut e, 7), 3.0);
    assert_eq!(get(&mut e, 8), 1_002_003.0);
    assert_eq!(get(&mut e, 9), 0.0);
    let set = |e: &mut Engine, t: i32, v: f64| {
        call(e, 0x2668, &[r(1), n(t), z(v)]);
    };
    set(&mut e, 0, 0.1);
    assert_eq!(e.mem.f32(DECAL), 0.1f32, "the double to the nearest float");
    set(&mut e, 7, 300.7);
    assert_eq!(e.mem.u8(DECAL + 0x1c), 44, "(int)300.7 = 300, its low byte");
    set(&mut e, 7, -0.5);
    assert_eq!(e.mem.u8(DECAL + 0x1c), 44, "below 0: nothing");
    set(&mut e, 7, f64::NAN);
    assert_eq!(e.mem.u8(DECAL + 0x1c), 44, "NaN fails value >= 0");
    set(&mut e, 8, 255_000_001.0);
    assert_eq!(e.mem.u32(DECAL + 0x20), 0x0001_00ff);
    set(&mut e, 8, 3e10);
    assert_eq!(
        e.mem.u32(DECAL + 0x20),
        0x0001_00ff,
        "low word 4230196224: too big"
    );
}

#[test]
fn the_flags() {
    let mut e = setup();
    let get = |e: &mut Engine, f: i32| call(e, 0x2669, &[r(1), n(f)]).1;
    e.mem.set_u16(SET + 0x84, 0x100);
    assert_eq!(get(&mut e, 0), 1.0, "any bit of the word");
    assert_eq!(get(&mut e, 1), 0.0, "no decal data");
    e.mem.set_u32(SET + 0x80, DECAL);
    e.mem.set_u8(DECAL + 0x1d, 4);
    assert_eq!((get(&mut e, 3), get(&mut e, 2)), (1.0, 0.0));
    assert_eq!(get(&mut e, 4), 0.0);
    let set = |e: &mut Engine, f: i32, v: i32| {
        call(e, 0x266a, &[r(1), n(f), n(v)]);
    };
    set(&mut e, 0, 5);
    assert_eq!(e.mem.u16(SET + 0x84), 1);
    set(&mut e, 0, 0);
    assert_eq!(e.mem.u16(SET + 0x84), 0);
    set(&mut e, 1, 1);
    assert_eq!(e.mem.u8(DECAL + 0x1d), 5);
    set(&mut e, 3, 0);
    assert_eq!(e.mem.u8(DECAL + 0x1d), 1);
    set(&mut e, 4, 1);
    assert_eq!(e.mem.u8(DECAL + 0x1d), 1, "flag 4: nothing");
}
