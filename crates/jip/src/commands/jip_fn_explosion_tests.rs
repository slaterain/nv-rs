//! Tests of the explosion commands, at the raw offsets of the explosion loader (`004FB0D0`):
//! enchantment +0x58, image space modifier +0x6C, placed object +0x70, `DATA` from +0x74 (force,
//! damage, radius, light +0x80, sound +0x84, flags +0x88, image space radius +0x8C, impact data
//! set +0x90, sound +0x94, radiation level, dissipation, radiation radius, sound level +0xA4).

use super::*;
use crate::commands::test_support::*;

const EXPL: u32 = FORMS + 0x1000;
const LIGHT: u32 = FORMS + 0x2000;
const SOUND: u32 = FORMS + 0x2100;
const IPDS: u32 = FORMS + 0x2200;
const ENCH: u32 = FORMS + 0x2300;
const IMAD: u32 = FORMS + 0x2400;
const MISC: u32 = FORMS + 0x2500;

fn setup() -> Engine {
    let mut e = engine();
    form(&mut e, EXPL, 0x51, 0x0000_e491);
    form(&mut e, LIGHT, 0x1e, 0x0000_0111);
    form(&mut e, SOUND, 0x0d, 0x0000_0222);
    form(&mut e, IPDS, 0x5f, 0x0000_0333);
    form(&mut e, ENCH, 0x13, 0x0000_0444);
    form(&mut e, IMAD, 0x54, 0x0000_0555);
    form(&mut e, MISC, 0x1f, 0x0000_0666);
    for (i, f) in [EXPL, LIGHT, SOUND, IPDS, ENCH, IMAD, MISC]
        .iter()
        .enumerate()
    {
        set_ref(&mut e, i as u16 + 1, *f);
    }
    // `kParamType_TESObject` (bound object): xNVSE's reader asks `IsBoundObject`.
    let vtbl = FORMS + 0x6000;
    for f in [EXPL, MISC] {
        e.mem.set_u32(f, vtbl);
    }
    e.mem
        .set_u32(vtbl + nvse::vtable::IS_BOUND_OBJECT, 0x0050_1000);
    e.register(0x0050_1000, |_, _| Ret {
        eax: 1,
        ..Ret::default()
    });
    e
}

const FLOATS: [(i32, u32); 7] = [
    (0, 0x74),
    (1, 0x78),
    (2, 0x7c),
    (3, 0x8c),
    (4, 0x98),
    (5, 0x9c),
    (6, 0xa0),
];

#[test]
fn the_numbers_at_their_offsets() {
    let mut e = setup();
    let get = |e: &mut Engine, t: i32| call(e, 0x261b, &[r(1), n(t)]).1;
    for (t, off) in FLOATS {
        e.mem.set_f32(EXPL + off, 1.5 + t as f32);
        assert_eq!(get(&mut e, t), 1.5 + f64::from(t), "trait {t}");
    }
    e.mem.set_u32(EXPL + 0xa4, 0xffff_ff02);
    assert_eq!(get(&mut e, 7), 2.0, "the sound level's byte");
    assert_eq!(get(&mut e, 8), 0.0);
    // Another form type: nothing.
    e.mem.set_f32(MISC + 0x74, 9.0);
    assert_eq!(call(&mut e, 0x261b, &[r(7), n(0)]).1, 0.0);
}

#[test]
fn the_numbers_set_with_jips_casts() {
    let mut e = setup();
    let set = |e: &mut Engine, t: i32, v: f64| {
        call(e, 0x261c, &[r(1), n(t), z(v)]);
    };
    for (t, off) in FLOATS {
        set(&mut e, t, -2.25 - f64::from(t));
        assert_eq!(e.mem.f32(EXPL + off), -2.25 - t as f32, "trait {t}");
    }
    // soundLevel (a byte) = val: (int) toward zero, then the low byte.
    e.mem.set_u32(EXPL + 0xa4, 0xaaaa_aaaa);
    set(&mut e, 7, 2.9);
    assert_eq!(e.mem.u32(EXPL + 0xa4), 0xaaaa_aa02, "one byte written");
    set(&mut e, 7, -1.5);
    assert_eq!(e.mem.u8(EXPL + 0xa4), 0xff);
    set(&mut e, 7, 257.0);
    assert_eq!(e.mem.u8(EXPL + 0xa4), 1);
    set(&mut e, 7, f64::NAN);
    assert_eq!(e.mem.u8(EXPL + 0xa4), 0, "NaN: INT_MIN's low byte");
    set(&mut e, 7, 3e9);
    assert_eq!(
        e.mem.u8(EXPL + 0xa4),
        0,
        "past the range: INT_MIN's low byte"
    );
    let before = e.mem.bytes(EXPL, 0xa8);
    set(&mut e, 8, 1.0);
    assert_eq!(e.mem.bytes(EXPL, 0xa8), before, "trait 8: nothing");
    call(&mut e, 0x261c, &[r(7), n(0), z(4.0)]);
    assert_eq!(e.mem.f32(MISC + 0x74), 0.0, "not an explosion");
}

#[test]
fn the_forms() {
    let mut e = setup();
    let fields = [
        (0, 0x80, 2u16, 0x111u32),
        (1, 0x84, 3, 0x222),
        (2, 0x94, 3, 0x222),
        (3, 0x90, 4, 0x333),
        (4, 0x70, 7, 0x666),
        (5, 0x58, 5, 0x444),
        (6, 0x6c, 6, 0x555),
    ];
    for (t, off, rf, id) in fields {
        call(&mut e, 0x261d, &[r(1), n(t)]);
        assert_eq!(form_result(&e), 0, "trait {t}: none");
        call(&mut e, 0x261e, &[r(1), n(t), r(rf)]);
        let form = e.mem.u32(EXPL + off);
        assert_ne!(form, 0, "trait {t} set");
        call(&mut e, 0x261d, &[r(1), n(t)]);
        assert_eq!(form_result(&e), id, "trait {t}");
        // A misc item: cleared (any form for the placed object).
        call(&mut e, 0x261e, &[r(1), n(t), r(7)]);
        let want = if t == 4 { MISC } else { 0 };
        assert_eq!(e.mem.u32(EXPL + off), want, "trait {t}: a misc item");
        call(&mut e, 0x261e, &[r(1), n(t), r(rf)]);
        call(&mut e, 0x261e, &[r(1), n(t)]);
        assert_eq!(e.mem.u32(EXPL + off), 0, "trait {t}: no form");
    }
    let before = e.mem.bytes(EXPL, 0xa8);
    call(&mut e, 0x261e, &[r(1), n(7), r(2)]);
    assert_eq!(e.mem.bytes(EXPL, 0xa8), before, "trait 7: nothing");
    e.mem.set_u32(EXPL + 0x80, LIGHT);
    call(&mut e, 0x261d, &[r(1), n(7)]);
    assert_eq!(form_result(&e), 0);
}

#[test]
fn the_flags() {
    let mut e = setup();
    let get = |e: &mut Engine, f: i32| call(e, 0x261f, &[r(1), n(f)]).1;
    let set = |e: &mut Engine, f: i32, v: i32| {
        call(e, 0x2620, &[r(1), n(f), n(v)]);
    };
    e.mem.set_u32(EXPL + 0x88, 0x8000_0003);
    assert_eq!(get(&mut e, 0), 0.0, "flag 0 is never answered");
    assert_eq!(get(&mut e, 1), 1.0);
    assert_eq!(get(&mut e, 31), 1.0);
    assert_eq!(get(&mut e, 2), 0.0);
    assert_eq!(get(&mut e, 32), 0.0);
    set(&mut e, 4, 7);
    assert_eq!(e.mem.u32(EXPL + 0x88), 0x8000_0013);
    set(&mut e, 31, 0);
    assert_eq!(e.mem.u32(EXPL + 0x88), 0x13);
    set(&mut e, 0, 0);
    assert_eq!(e.mem.u32(EXPL + 0x88), 0x13, "flag 0: nothing");
    set(&mut e, 32, 1);
    assert_eq!(e.mem.u32(EXPL + 0x88), 0x13, "flag 32: nothing");
}

/// The explosion is read as any form (the setters) or bound object (the getters): a misc item with
/// the same words answers 0 and keeps its bytes.
#[test]
fn another_form_type_is_left_alone() {
    let mut e = setup();
    for off in (0x58..0xa8).step_by(4) {
        e.mem.set_u32(MISC + off, LIGHT);
    }
    let before = e.mem.bytes(MISC, 0xa8);
    for t in 0..8 {
        assert_eq!(call(&mut e, 0x261b, &[r(7), n(t)]).1, 0.0, "number {t}");
        call(&mut e, 0x261c, &[r(7), n(t), z(1.0)]);
    }
    for t in 0..7 {
        call(&mut e, 0x261d, &[r(7), n(t)]);
        assert_eq!(form_result(&e), 0, "form {t}");
        call(&mut e, 0x261e, &[r(7), n(t), r(2)]);
        call(&mut e, 0x261e, &[r(7), n(t)]);
    }
    for f in 1..32 {
        call(&mut e, 0x2620, &[r(7), n(f), n(0)]);
    }
    assert_eq!(call(&mut e, 0x261f, &[r(7), n(1)]).1, 0.0);
    assert_eq!(e.mem.bytes(MISC, 0xa8), before);
}
