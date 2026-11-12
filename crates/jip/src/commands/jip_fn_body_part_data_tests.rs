//! Tests of the body part data commands. The forms are built at the offsets JIP's header gives
//! and the exe's loaders confirm (`005E4D80`: model +0x18, parts from +0x34, ragdoll +0x70;
//! `BGSBodyPart::Load`, `005E4100`: limb replacement +0x2C, `BPND` from +0x5C); the expected
//! answers are worked out from JIP's C++.

use super::*;
use crate::commands::test_support::*;

const DATA: u32 = FORMS + 0x1000;
const PART: u32 = FORMS + 0x2000;
const RAGDOLL: u32 = FORMS + 0x3000;
const EXPLOSION: u32 = FORMS + 0x3100;
const DEBRIS: u32 = FORMS + 0x3200;
const IMPACTS: u32 = FORMS + 0x3300;
const OTHER: u32 = FORMS + 0x3400;

fn setup() -> Engine {
    let mut e = engine();
    form(&mut e, DATA, 0x57, 0x0000_b0d1);
    form(&mut e, RAGDOLL, 0x63, 0x0000_0a61);
    form(&mut e, EXPLOSION, 0x51, 0x0000_0e51);
    form(&mut e, DEBRIS, 0x52, 0x0000_0d52);
    form(&mut e, IMPACTS, 0x5f, 0x0000_015f);
    form(&mut e, OTHER, 0x36, 0x0000_0036);
    model(&mut e, DATA + 0x18, Some(b"skeleton.nif"));
    // Part 1 (head) present, the others absent.
    e.mem.set_u32(DATA + 0x34 + 4, PART);
    model(&mut e, PART + 0x2c, Some(b"head_gore.nif"));
    for (i, f) in [DATA, RAGDOLL, EXPLOSION, DEBRIS, IMPACTS, OTHER]
        .iter()
        .enumerate()
    {
        set_ref(&mut e, i as u16 + 1, *f);
    }
    CALLS.with(|c| c.borrow_mut().clear());
    e
}

const NO_FORM: u16 = 9;

#[test]
fn the_skeleton() {
    let mut e = setup();
    let (_, res) = call(&mut e, 0x26ac, &[r(1)]);
    assert_eq!(string_result(&e, res), b"skeleton.nif");
    let (_, res) = call(&mut e, 0x26ac, &[r(6)]);
    assert_eq!(string_result(&e, res), b"", "not body part data");
    call(&mut e, 0x26ad, &[r(1), s(b"other.nif")]);
    assert_eq!(text_at(&e, DATA + 0x1c), b"other.nif");
    let set = calls()
        .into_iter()
        .find(|c| c.0 == 0x0048_9100)
        .expect("SetModelPath");
    assert_eq!(set.1[0], DATA + 0x18);
}

#[test]
fn the_ragdoll() {
    let mut e = setup();
    call(&mut e, 0x26ae, &[r(1)]);
    assert_eq!(form_result(&e), 0, "none");
    call(&mut e, 0x26af, &[r(1), r(2)]);
    assert_eq!(e.mem.u32(DATA + 0x70), RAGDOLL);
    call(&mut e, 0x26ae, &[r(1)]);
    assert_eq!(form_result(&e), 0x0a61);
    call(&mut e, 0x26af, &[r(1), r(3)]);
    assert_eq!(e.mem.u32(DATA + 0x70), RAGDOLL, "an explosion is no ragdoll");
    call(&mut e, 0x26af, &[r(1)]);
    assert_eq!(e.mem.u32(DATA + 0x70), 0, "cleared without one");
    e.mem.set_u32(OTHER + 0x70, 0x55);
    call(&mut e, 0x26af, &[r(6), r(2)]);
    assert_eq!(e.mem.u32(OTHER + 0x70), 0x55, "not body part data");
}

#[test]
fn which_parts_it_has() {
    let mut e = setup();
    assert_eq!(call(&mut e, 0x26b0, &[r(1), n(1)]).1, 1.0);
    assert_eq!(call(&mut e, 0x26b0, &[r(1), n(0)]).1, 0.0);
    e.mem.set_u32(DATA + 0x34 + 4 * 15, PART);
    assert_eq!(call(&mut e, 0x26b0, &[r(1), n(15)]).1, 0.0, "part 15");
    assert_eq!(call(&mut e, 0x26b0, &[r(6), n(1)]).1, 0.0);
    assert_eq!(call(&mut e, 0x26b0, &[r(NO_FORM), n(1)]).1, 0.0);
}

#[test]
fn the_numeric_traits() {
    let mut e = setup();
    e.mem.set_u8(PART + 0x63, 17); // actor value
    e.mem.set_f32(PART + 0x5c, 2.5); // damage multiplier
    e.mem.set_u8(PART + 0x62, 80); // health percent
    e.mem.set_u8(PART + 0x64, 30); // to-hit chance
    e.mem.set_f32(PART + 0x70, 45.0); // tracking angle
    e.mem.set_u8(PART + 0x78, 3); // severed debris count
    e.mem.set_f32(PART + 0x84, 0.75); // severed debris scale
    e.mem.set_u8(PART + 0xa8, 4); // severed decal count
    e.mem.set_u8(PART + 0x65, 50); // explode chance
    e.mem.set_f32(PART + 0xac, 1.25); // limb replacement scale
    e.mem.set_u8(PART + 0x66, 6); // explode debris count
    e.mem.set_f32(PART + 0x74, 0.5); // explode debris scale
    e.mem.set_u8(PART + 0xa9, 7); // explode decal count
    e.mem.set_f32(PART + 0x88 + 8, -3.0); // gore translate z
    e.mem.set_f32(PART + 0x94 + 4, 1.0); // gore rotation y, radians
    let get = |e: &mut Engine, t: i32| call(e, 0x26b1, &[r(1), n(1), n(t)]).1;
    let expected = [
        17.0, 2.5, 80.0, 30.0, 45.0, 3.0, 0.75, 4.0, 50.0, 1.25, 6.0, 0.5, 7.0, 0.0, 0.0, -3.0,
    ];
    for (t, v) in expected.iter().enumerate() {
        assert_eq!(get(&mut e, t as i32), *v, "trait {t}");
    }
    assert_eq!(get(&mut e, 16), 0.0);
    assert_eq!(get(&mut e, 17), f64::from(1.0f32) * 57.295_779_513_082_32);
    assert_eq!(get(&mut e, 19), 0.0, "past 18");
    assert_eq!(call(&mut e, 0x26b1, &[r(1), n(2), n(1)]).1, 0.0, "no part");
}

#[test]
fn the_numeric_traits_set() {
    let mut e = setup();
    let set = |e: &mut Engine, t: i32, v: f64| {
        call(e, 0x26b2, &[r(1), n(1), n(t), z(v)]);
    };
    set(&mut e, 0, 300.9);
    assert_eq!(e.mem.u8(PART + 0x63), (300 & 0xff) as u8, "the low byte of 300");
    set(&mut e, 1, 1.5);
    assert_eq!(e.mem.f32(PART + 0x5c), 1.5);
    set(&mut e, 7, 9.99);
    assert_eq!(e.mem.u8(PART + 0xa8), 9);
    set(&mut e, 9, 2.0);
    assert_eq!(e.mem.f32(PART + 0xac), 2.0);
    set(&mut e, 12, 12.0);
    assert_eq!(e.mem.u8(PART + 0xa9), 12);
    set(&mut e, 14, 5.0);
    assert_eq!(e.mem.f32(PART + 0x88 + 4), 5.0);
    set(&mut e, 18, 90.0);
    assert_eq!(e.mem.f32(PART + 0x94 + 8), 90.0f32 * 0.017_453_292_38f32);
    // A negative value, or a trait past 18: nothing.
    set(&mut e, 1, -0.5);
    assert_eq!(e.mem.f32(PART + 0x5c), 1.5);
    let before = e.mem.bytes(PART, 0xb0);
    set(&mut e, 19, 1.0);
    assert_eq!(e.mem.bytes(PART, 0xb0), before);
}

#[test]
fn the_form_traits() {
    let mut e = setup();
    let fields = [0x80, 0x6c, 0x7c, 0x68, 0xa0, 0xa4];
    let forms = [3u16, 3, 4, 4, 5, 5];
    let ids = [0x0e51, 0x0e51, 0x0d52, 0x0d52, 0x015f, 0x015f];
    for t in 0..6 {
        call(&mut e, 0x26b4, &[r(1), n(1), n(t as i32), r(forms[t])]);
        assert_eq!(e.mem.u32(PART + fields[t]), FORMS + 0x3000 + 0x100 * u32::from(forms[t] - 2));
        call(&mut e, 0x26b3, &[r(1), n(1), n(t as i32)]);
        assert_eq!(form_result(&e), ids[t], "trait {t}");
        // A form of another type clears it.
        call(&mut e, 0x26b4, &[r(1), n(1), n(t as i32), r(2)]);
        assert_eq!(e.mem.u32(PART + fields[t]), 0, "trait {t}");
        call(&mut e, 0x26b3, &[r(1), n(1), n(t as i32)]);
        assert_eq!(form_result(&e), 0);
    }
    call(&mut e, 0x26b4, &[r(1), n(1), n(0), r(3)]);
    call(&mut e, 0x26b4, &[r(1), n(1), n(0)]);
    assert_eq!(e.mem.u32(PART + 0x80), 0, "no form clears it");
    call(&mut e, 0x26b4, &[r(1), n(1), n(6), r(3)]);
    assert_eq!(e.mem.u32(PART + 0xa4), 0, "trait 6: nothing");
}

#[test]
fn the_part_name() {
    let mut e = setup();
    let (_, res) = call(&mut e, 0x26b5, &[r(1), n(1)]);
    assert_eq!(string_result(&e, res), b"", "no buffer");
    string(&mut e, PART + 0x1c, Some(b"Head"));
    let (_, res) = call(&mut e, 0x26b5, &[r(1), n(1)]);
    assert_eq!(string_result(&e, res), b"Head");
    // A length of 0 answers "" whatever the buffer holds.
    e.mem.set_u16(PART + 0x20, 0);
    let (_, res) = call(&mut e, 0x26b5, &[r(1), n(1)]);
    assert_eq!(string_result(&e, res), b"");
    call(&mut e, 0x26b6, &[r(1), n(1), s(b"Skull")]);
    assert_eq!(text_at(&e, PART + 0x1c), b"Skull");
    assert_eq!(e.mem.u16(PART + 0x20), 5);
}

#[test]
#[should_panic(expected = "runs past JIP's 64-byte buffer")]
fn a_name_longer_than_its_buffer_stops() {
    let mut e = setup();
    call(&mut e, 0x26b6, &[r(1), n(1), s(&[b'n'; 64])]);
}

#[test]
fn the_limb_replacement_model() {
    let mut e = setup();
    let (_, res) = call(&mut e, 0x26b7, &[r(1), n(1)]);
    assert_eq!(string_result(&e, res), b"head_gore.nif");
    let (_, res) = call(&mut e, 0x26b7, &[r(1), n(3)]);
    assert_eq!(string_result(&e, res), b"", "no part");
    call(&mut e, 0x26b8, &[r(1), n(1), s(b"x.nif")]);
    assert_eq!(text_at(&e, PART + 0x30), b"x.nif");
    let set = calls()
        .into_iter()
        .find(|c| c.0 == 0x0048_9100)
        .expect("SetModelPath");
    assert_eq!(set.1[0], PART + 0x2c);
}

#[test]
fn the_flags() {
    let mut e = setup();
    e.mem.set_u8(PART + 0x60, 0x11);
    assert_eq!(call(&mut e, 0x26b9, &[r(1), n(1), n(4)]).1, 1.0);
    assert_eq!(call(&mut e, 0x26b9, &[r(1), n(1), n(1)]).1, 0.0);
    assert_eq!(call(&mut e, 0x26b9, &[r(1), n(1), n(8)]).1, 0.0, "flag 8");
    call(&mut e, 0x26ba, &[r(1), n(1), n(7), n(5)]);
    assert_eq!(e.mem.u8(PART + 0x60), 0x91);
    call(&mut e, 0x26ba, &[r(1), n(1), n(0), n(0)]);
    assert_eq!(e.mem.u8(PART + 0x60), 0x90);
    call(&mut e, 0x26ba, &[r(1), n(1), n(8), n(1)]);
    assert_eq!(e.mem.u8(PART + 0x60), 0x90, "flag 8: nothing");
    call(&mut e, 0x26ba, &[r(1), n(0), n(1), n(1)]);
    assert_eq!(e.mem.u8(PART + 0x60), 0x90, "no part 0");
}
