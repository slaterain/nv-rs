//! Tests of the impact data commands, at the raw offsets of the impact data loader (`0058DEA0`:
//! model +0x18, `DATA` +0x30: duration, orientation +0x34, angle +0x38, radius +0x3C, sound level
//! +0x40, no decal +0x44; texture set +0x48, sounds +0x4C/+0x50; the decal from +0x54: seven
//! floats, passes +0x70, flags +0x71, colour +0x74) and of the set's loader (`0058EC00`: twelve
//! pointers from +0x1C). Casts as MSVC compiles JIP's source.

use super::*;
use crate::commands::test_support::*;

const IPCT: u32 = FORMS + 0x1000;
const IPDS: u32 = FORMS + 0x2000;
const TXST: u32 = FORMS + 0x3000;
const SOUND: u32 = FORMS + 0x3100;
const MISC: u32 = FORMS + 0x3200;

fn setup() -> Engine {
    let mut e = engine();
    form(&mut e, IPCT, 0x5e, 0x0000_1c70);
    form(&mut e, IPDS, 0x5f, 0x0000_1d50);
    form(&mut e, TXST, 0x04, 0x0000_0777);
    form(&mut e, SOUND, 0x0d, 0x0000_0888);
    form(&mut e, MISC, 0x1f, 0x0000_0999);
    for (i, f) in [IPCT, IPDS, TXST, SOUND, MISC].iter().enumerate() {
        set_ref(&mut e, i as u16 + 1, *f);
    }
    model(&mut e, IPCT + 0x18, Some(b"impact.nif"));
    e
}

#[test]
fn the_model() {
    let mut e = setup();
    let (_, res) = call(&mut e, 0x2790, &[r(1)]);
    assert_eq!(string_result(&e, res), b"impact.nif");
    call(&mut e, 0x2791, &[r(1), s(b"other.nif")]);
    assert_eq!(text_at(&e, IPCT + 0x1c), b"other.nif");
    let (_, res) = call(&mut e, 0x2790, &[r(2)]);
    assert_eq!(string_result(&e, res), b"", "a set has no model");
    call(&mut e, 0x2791, &[r(5), s(b"x.nif")]);
    assert_eq!(e.mem.u32(MISC + 0x1c), 0);
}

#[test]
fn the_traits_read_at_their_offsets() {
    let mut e = setup();
    let get = |e: &mut Engine, t: i32| call(e, 0x2792, &[r(1), n(t)]).1;
    e.mem.set_f32(IPCT + 0x30, 0.5);
    e.mem.set_u8(IPCT + 0x34, 2);
    e.mem.set_f32(IPCT + 0x38, 30.0);
    e.mem.set_f32(IPCT + 0x3c, 4.0);
    e.mem.set_u8(IPCT + 0x40, 1);
    e.mem.set_u8(IPCT + 0x44, 1);
    for i in 0..7 {
        e.mem.set_f32(IPCT + 0x54 + 4 * i, 10.0 + i as f32);
    }
    e.mem.set_u8(IPCT + 0x70, 6);
    e.mem.set_u8(IPCT + 0x71, 5);
    e.mem.set_u32(IPCT + 0x74, 0x0003_0201);
    assert_eq!(get(&mut e, 0), 0.5);
    assert_eq!(get(&mut e, 1), 2.0);
    assert_eq!(get(&mut e, 2), 30.0);
    assert_eq!(get(&mut e, 3), 4.0);
    assert_eq!(get(&mut e, 4), 1.0);
    for t in 8..=14 {
        assert_eq!(get(&mut e, t), f64::from(10 + t - 8), "decal float {t}");
    }
    assert_eq!(get(&mut e, 15), 6.0);
    assert_eq!(get(&mut e, 16), 11.0, "flags 5 << 1 | no decal 1");
    assert_eq!(get(&mut e, 17), 1_002_003.0);
    // The colour word unsigned: a top byte past 0x7F stays positive.
    e.mem.set_u32(IPCT + 0x74, 0xffff_ffff);
    assert_eq!(get(&mut e, 17), f64::from(rgb_hex_to_dec(0xffff_ffff)));
    assert_eq!(get(&mut e, 18), 0.0);
    // Forms: their ids, none when empty.
    for (t, off) in [(5, 0x48), (6, 0x4c), (7, 0x50)] {
        call(&mut e, 0x2792, &[r(1), n(t)]);
        assert_eq!(form_result(&e), 0, "trait {t}: none");
        e.mem.set_u32(IPCT + off, SOUND);
        call(&mut e, 0x2792, &[r(1), n(t)]);
        assert_eq!(form_result(&e), 0x888, "trait {t}");
    }
    assert_eq!(call(&mut e, 0x2792, &[r(2), n(0)]).1, 0.0, "a set");
}

#[test]
fn the_traits_set_with_jips_casts() {
    let mut e = setup();
    let set = |e: &mut Engine, t: i32, v: f64| {
        call(e, 0x2793, &[r(1), n(t), z(v)]);
    };
    // Orientation and sound level: cvtd2ul(value), below 3, its low byte.
    set(&mut e, 1, 2.9);
    assert_eq!(e.mem.u8(IPCT + 0x34), 2);
    set(&mut e, 1, 3.0);
    assert_eq!(e.mem.u8(IPCT + 0x34), 2, "3 refused");
    set(&mut e, 1, -1.0);
    assert_eq!(e.mem.u8(IPCT + 0x34), 2, "-1 is 0xFFFFFFFF: refused");
    set(&mut e, 1, f64::NAN);
    assert_eq!(
        e.mem.u8(IPCT + 0x34),
        0,
        "NaN: the indefinite's low word, 0"
    );
    set(&mut e, 4, 1.5);
    assert_eq!(e.mem.u8(IPCT + 0x40), 1);
    set(&mut e, 4, 4_294_967_296.0);
    assert_eq!(e.mem.u8(IPCT + 0x40), 0, "2^32: low word 0");
    // Floats: (float)value.
    set(&mut e, 0, 0.1);
    assert_eq!(e.mem.f32(IPCT + 0x30), 0.1f32);
    set(&mut e, 2, -45.5);
    assert_eq!(e.mem.f32(IPCT + 0x38), -45.5);
    set(&mut e, 3, 1e40);
    assert_eq!(e.mem.f32(IPCT + 0x3c), f32::INFINITY);
    for t in 5..=11 {
        set(&mut e, t, f64::from(t) + 0.25);
        assert_eq!(
            e.mem.f32(IPCT + 0x54 + 4 * (t as u32 - 5)),
            t as f32 + 0.25,
            "decal float {t}"
        );
    }
    // Passes and flags: the word's low byte.
    set(&mut e, 12, 300.0);
    assert_eq!(e.mem.u8(IPCT + 0x70), 44);
    set(&mut e, 13, 11.0);
    assert_eq!((e.mem.u8(IPCT + 0x44), e.mem.u8(IPCT + 0x71)), (1, 5));
    set(&mut e, 13, 0x1fe as f64);
    assert_eq!((e.mem.u8(IPCT + 0x44), e.mem.u8(IPCT + 0x71)), (0, 0xff));
    set(&mut e, 14, 1_002_003.0);
    assert_eq!(e.mem.u32(IPCT + 0x74), 0x0003_0201);
    set(&mut e, 14, 255_255_256.0);
    assert_eq!(e.mem.u32(IPCT + 0x74), 0x0003_0201, "past 255255255");
    set(&mut e, 14, 255_255_255.0);
    assert_eq!(e.mem.u32(IPCT + 0x74), 0x00ff_ffff);
    let before = e.mem.bytes(IPCT, 0x78);
    set(&mut e, 15, 1.0);
    assert_eq!(e.mem.bytes(IPCT, 0x78), before, "trait 15: nothing");
}

#[test]
fn the_forms_set() {
    let mut e = setup();
    let set = |e: &mut Engine, t: i32, f: Option<u16>| {
        let mut args = vec![r(1), n(t)];
        if let Some(f) = f {
            args.push(r(f));
        }
        call(e, 0x2794, &args);
    };
    set(&mut e, 0, Some(3));
    assert_eq!(e.mem.u32(IPCT + 0x48), TXST);
    set(&mut e, 0, Some(4));
    assert_eq!(e.mem.u32(IPCT + 0x48), 0, "a sound is no texture set");
    set(&mut e, 1, Some(4));
    set(&mut e, 2, Some(4));
    assert_eq!(
        (e.mem.u32(IPCT + 0x4c), e.mem.u32(IPCT + 0x50)),
        (SOUND, SOUND)
    );
    set(&mut e, 2, None);
    assert_eq!(e.mem.u32(IPCT + 0x50), 0);
    set(&mut e, 1, Some(3));
    assert_eq!(e.mem.u32(IPCT + 0x4c), 0, "a texture set is no sound");
    let before = e.mem.bytes(IPCT, 0x78);
    set(&mut e, 3, Some(4));
    assert_eq!(e.mem.bytes(IPCT, 0x78), before, "trait 3: nothing");
}

#[test]
fn the_set_per_material() {
    let mut e = setup();
    call(&mut e, 0x2887, &[r(2), n(11), r(1)]);
    assert_eq!(e.mem.u32(IPDS + 0x1c + 4 * 11), IPCT);
    call(&mut e, 0x2886, &[r(2), n(11)]);
    assert_eq!(form_result(&e), 0x1c70);
    call(&mut e, 0x2886, &[r(2), n(0)]);
    assert_eq!(form_result(&e), 0, "material 0: none");
    // Material 12 would be the word after the array.
    e.mem.set_u32(IPDS + 0x1c + 4 * 12, IPCT);
    call(&mut e, 0x2886, &[r(2), n(12)]);
    assert_eq!(form_result(&e), 0, "material 12");
    call(&mut e, 0x2887, &[r(2), n(12)]);
    assert_eq!(
        e.mem.u32(IPDS + 0x1c + 4 * 12),
        IPCT,
        "material 12: nothing"
    );
    call(&mut e, 0x2887, &[r(2), n(11), r(5)]);
    assert_eq!(
        e.mem.u32(IPDS + 0x1c + 4 * 11),
        IPCT,
        "a misc item: nothing"
    );
    call(&mut e, 0x2887, &[r(2), n(11)]);
    assert_eq!(e.mem.u32(IPDS + 0x1c + 4 * 11), 0, "cleared");
    let before = e.mem.bytes(IPCT, 0x78);
    call(&mut e, 0x2887, &[r(1), n(0), r(1)]);
    assert_eq!(e.mem.bytes(IPCT, 0x78), before, "an impact data is no set");
}
