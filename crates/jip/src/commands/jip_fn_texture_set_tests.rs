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
    // A text where a seventh map would lie (+0x84), so reading map 6 would show it.
    string(&mut e, SET + 0x38 + 0xc * 6 + 4, Some(b"t6.dds"));
    let (_, res) = call(&mut e, 0x2665, &[r(1), n(6)]);
    assert_eq!(string_result(&e, res), b"", "map 6");
    e.mem.set_u32(SET + 0x38 + 0xc * 6 + 4, 0);
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

/// The texture set is read as any form; only the commands' type check (a texture set, 0x04)
/// guards the fields. A climate with a word at +0x80 and a string at +0x3C: the getters answer
/// ""/0 and the setters leave its bytes, and the block its word points to, as they were.
#[test]
fn another_form_type_is_left_alone() {
    let mut e = setup();
    const OTHER: u32 = FORMS + 0x3000;
    const BLOCK: u32 = FORMS + 0x4000;
    form(&mut e, OTHER, 0x36, 0x0000_0036);
    set_ref(&mut e, 2, OTHER);
    string(&mut e, OTHER + 0x3c, Some(b"t0.dds"));
    e.mem.set_u32(OTHER + 0x80, BLOCK);
    e.mem.set_u16(OTHER + 0x84, 0xffff);
    for i in 0..0x24 {
        e.mem.set_u8(BLOCK + i, 0x40 + i as u8);
    }
    let (_, res) = call(&mut e, 0x2665, &[r(2), n(0)]);
    assert_eq!(string_result(&e, res), b"");
    for t in 0..=8 {
        assert_eq!(call(&mut e, 0x2667, &[r(2), n(t)]).1, 0.0, "trait {t}");
    }
    for f in 0..=3 {
        assert_eq!(call(&mut e, 0x2669, &[r(2), n(f)]).1, 0.0, "flag {f}");
    }
    let before = (e.mem.bytes(OTHER, 0xa0), e.mem.bytes(BLOCK, 0x24));
    let path_before = text_at(&e, OTHER + 0x3c);
    call(&mut e, 0x2666, &[r(2), n(0), s(b"x.dds")]);
    for t in 0..=8 {
        call(&mut e, 0x2668, &[r(2), n(t), z(3.0)]);
    }
    for f in 0..=3 {
        call(&mut e, 0x266a, &[r(2), n(f), n(0)]);
    }
    assert_eq!((e.mem.bytes(OTHER, 0xa0), e.mem.bytes(BLOCK, 0x24)), before);
    assert_eq!(text_at(&e, OTHER + 0x3c), path_before);
}

/// Decal traits 0-6, each a distinct value: each lands in its own float and nothing else of the
/// decal changes.
#[test]
fn the_decal_floats_each_in_place() {
    let mut e = setup();
    e.mem.set_u32(SET + 0x80, DECAL);
    for i in 0..0x24 {
        e.mem.set_u8(DECAL + i, 0x5a);
    }
    let mut want = e.mem.bytes(DECAL, 0x24);
    for t in 0..7u32 {
        let v = 1.5 + f64::from(t);
        call(&mut e, 0x2668, &[r(1), n(t as i32), z(v)]);
        let at = 4 * t as usize;
        want[at..at + 4].copy_from_slice(&(v as f32).to_le_bytes());
        assert_eq!(e.mem.bytes(DECAL, 0x24), want, "trait {t}");
    }
}
