//! Tests of the projectile commands, at raw offsets: the base form's flags +0x60, type +0x62,
//! floats (traits 1-3 from +0x64, 4-6 from +0x78, 7-9 from +0x8C, 10-13 from +0xA4), light
//! +0x70, explosion +0x84, muzzle flash model +0xB4, sound level +0xCC; a fired projectile's
//! impacts +0x88, impact byte +0x90, own flags +0xC8, speed +0xD0, life +0xD8, damage +0xDC,
//! detonation timer +0xE4, weapon +0xF8, source +0xFC, distance +0x110; its base form at +0x20.

use super::*;
use crate::commands::test_support::*;

const PROJ: u32 = FORMS + 0x1000;
const FP: u32 = FORMS + 0x2000;
const NR: u32 = FORMS + 0x2200;
const MISC: u32 = FORMS + 0x2400;
const EXPL: u32 = FORMS + 0x2500;
const LIGH: u32 = FORMS + 0x2600;
const WEAP: u32 = FORMS + 0x2700;
const REF: u32 = FORMS + 0x2800;
const PC: u32 = FORMS + 0x2900;
const IMPACTS: u32 = FORMS + 0x4000;
const NODE: u32 = FORMS + 0x4800;
const VT: u32 = FORMS + 0x7000;
const VT2: u32 = FORMS + 0x7400;
const VB: u32 = FORMS + 0x7800;
/// The base form up to one word past it, the fired projectile the same.
const BSIZE: u32 = 0xD4;
const PSIZE: u32 = 0x154;

fn setup() -> Engine {
    let mut e = engine();
    e.map(0x011d_e000, 0x1000);
    form(&mut e, PROJ, 0x33, 0x0000_0b01);
    form(&mut e, FP, 0x3e, 0xff00_0001);
    form(&mut e, NR, 0x3a, 0xff00_0002);
    form(&mut e, MISC, 0x1f, 0x0000_0999);
    form(&mut e, EXPL, 0x51, 0x0000_0e51);
    form(&mut e, LIGH, 0x1e, 0x0000_011e);
    form(&mut e, WEAP, 0x28, 0x0000_0028);
    form(&mut e, REF, 0x3a, 0xff00_0003);
    form(&mut e, PC, 0x3b, 0x0000_0014);
    for (i, f) in [PROJ, MISC, EXPL, LIGH, WEAP, REF].iter().enumerate() {
        set_ref(&mut e, i as u16 + 1, *f);
    }
    e.mem.set_u32(0x011d_ea3c, PC);
    // Bound objects: a vtable whose `IsBoundObject` (+0xE8) answers true, for the reader's
    // `kParamType_BoundObject` check.
    e.register(0x0050_3000, |_, _| Ret {
        eax: 1,
        ..Ret::default()
    });
    e.mem.set_u32(VB + 0xe8, 0x0050_3000);
    for f in [PROJ, MISC, EXPL, LIGH] {
        e.mem.set_u32(f, VB);
    }
    // The fired grenade and a plain reference: vtables answering true or not at +0x224.
    e.mem.set_u32(VT + 0x224, 0x008d_0360);
    e.mem.set_u32(FP, VT);
    e.mem.set_u32(NR, VT2);
    e.mem.set_u32(FP + 0x20, PROJ);
    e.mem.set_u32(NR + 0x20, PROJ);
    model(&mut e, PROJ + 0xb4, Some(b"flash.nif"));
    CALLS.with(|c| c.borrow_mut().clear());
    e
}

fn with_word(e: &Engine, at: u32, size: u32, off: u32, value: u32) -> Vec<u8> {
    let mut want = e.mem.bytes(at, size);
    let off = off as usize;
    want[off..off + 4].copy_from_slice(&value.to_le_bytes());
    want
}

const TRAIT_OFFSETS: [(i32, u32); 13] = [
    (1, 0x64),
    (2, 0x68),
    (3, 0x6c),
    (4, 0x78),
    (5, 0x7c),
    (6, 0x80),
    (7, 0x8c),
    (8, 0x90),
    (9, 0x94),
    (10, 0xa4),
    (11, 0xa8),
    (12, 0xac),
    (13, 0xb0),
];

#[test]
fn the_numbers() {
    let mut e = setup();
    e.mem.set_u16(PROJ + 0x62, 4);
    e.mem.set_u32(PROJ + 0xcc, 0x0000_0102);
    assert_eq!(call(&mut e, 0x2615, &[r(1), n(0)]).1, 4.0);
    assert_eq!(call(&mut e, 0x2615, &[r(1), n(14)]).1, 2.0, "the low byte");
    for (t, off) in TRAIT_OFFSETS {
        e.mem.set_f32(PROJ + off, t as f32 + 0.5);
        assert_eq!(
            call(&mut e, 0x2615, &[r(1), n(t)]).1,
            f64::from(t) + 0.5,
            "trait {t}"
        );
    }
    assert_eq!(
        call(&mut e, 0x2615, &[r(1), n(15)]).1,
        0.0,
        "trait 15: nothing"
    );
    for (t, off) in TRAIT_OFFSETS {
        let v = -(t as f32) - 0.25;
        let want = with_word(&e, PROJ, BSIZE, off, v.to_bits());
        call(&mut e, 0x2616, &[r(1), n(t), z(f64::from(v))]);
        assert_eq!(e.mem.bytes(PROJ, BSIZE), want, "trait {t} alone");
    }
}

#[test]
fn the_type_and_sound_level() {
    let mut e = setup();
    let set = |e: &mut Engine, t: i32, v: f64| {
        call(e, 0x2616, &[r(1), n(t), z(v)]);
    };
    let ty = |e: &Engine| e.mem.u16(PROJ + 0x62);
    set(&mut e, 0, 8.9);
    assert_eq!(ty(&e), 8, "(int), a power of two");
    set(&mut e, 0, 3.0);
    assert_eq!(ty(&e), 8, "3: refused");
    set(&mut e, 0, -1.5);
    assert_eq!(ty(&e), 8, "-1 is 0xFFFFFFFF: refused");
    set(&mut e, 0, 65536.0 * 2.0);
    assert_eq!(ty(&e), 0, "0x20000: a power of two, its low 16 bits 0");
    set(&mut e, 0, 16.0);
    set(&mut e, 0, f64::NAN);
    assert_eq!(ty(&e), 0, "NaN: 0x80000000, a power of two, low 16 bits 0");
    let before = e.mem.bytes(PROJ, BSIZE);
    let mut want = before.clone();
    want[0xcc] = 2;
    set(&mut e, 14, 2.0);
    assert_eq!(
        e.mem.bytes(PROJ, BSIZE),
        want,
        "the sound level's byte alone"
    );
    for v in [0.0, 3.0, -1.0] {
        set(&mut e, 14, v);
    }
    set(&mut e, 15, 1.0);
    assert_eq!(
        e.mem.bytes(PROJ, BSIZE),
        want,
        "0, 3, -1, trait 15: nothing"
    );
}

#[test]
fn the_flags() {
    let mut e = setup();
    e.mem.set_u16(PROJ + 0x60, 0x8001);
    assert_eq!(call(&mut e, 0x2617, &[r(1), n(15)]).1, 1.0);
    assert_eq!(call(&mut e, 0x2617, &[r(1), n(1)]).1, 0.0);
    assert_eq!(call(&mut e, 0x2617, &[r(1), n(16)]).1, 0.0, "flag 16: no");
    let mut want = e.mem.bytes(PROJ, BSIZE);
    want[0x60..0x62].copy_from_slice(&0x8003u16.to_le_bytes());
    call(&mut e, 0x2618, &[r(1), n(1), n(5)]);
    assert_eq!(e.mem.bytes(PROJ, BSIZE), want, "flag 1 set alone");
    want[0x60..0x62].copy_from_slice(&0x0003u16.to_le_bytes());
    call(&mut e, 0x2618, &[r(1), n(15), n(0)]);
    assert_eq!(e.mem.bytes(PROJ, BSIZE), want, "flag 15 cleared alone");
    call(&mut e, 0x2618, &[r(1), n(16), n(1)]);
    assert_eq!(e.mem.bytes(PROJ, BSIZE), want, "flag 16: nothing");
}

#[test]
fn explosion_and_light() {
    let mut e = setup();
    call(&mut e, 0x2619, &[r(1)]);
    assert_eq!(form_result(&e), 0, "none");
    let want = with_word(&e, PROJ, BSIZE, 0x84, EXPL);
    call(&mut e, 0x261a, &[r(1), r(3)]);
    assert_eq!(e.mem.bytes(PROJ, BSIZE), want, "the explosion alone");
    call(&mut e, 0x2619, &[r(1)]);
    assert_eq!(form_result(&e), 0x0e51);
    let want = with_word(&e, PROJ, BSIZE, 0x84, 0);
    call(&mut e, 0x261a, &[r(1), r(4)]);
    assert_eq!(
        e.mem.bytes(PROJ, BSIZE),
        want,
        "a light is no explosion: none"
    );
    e.mem.set_u32(PROJ + 0x84, EXPL);
    call(&mut e, 0x261a, &[r(1)]);
    assert_eq!(e.mem.u32(PROJ + 0x84), 0, "left out: none");
    let want = with_word(&e, PROJ, BSIZE, 0x70, LIGH);
    call(&mut e, 0x2933, &[r(1), r(4)]);
    assert_eq!(e.mem.bytes(PROJ, BSIZE), want, "the light alone");
    call(&mut e, 0x2932, &[r(1)]);
    assert_eq!(form_result(&e), 0x011e);
    call(&mut e, 0x2933, &[r(1), r(3)]);
    assert_eq!(
        e.mem.bytes(PROJ, BSIZE),
        want,
        "an explosion is no light: nothing"
    );
    call(&mut e, 0x2933, &[r(1)]);
    assert_eq!(e.mem.u32(PROJ + 0x70), 0, "left out: none");
}

#[test]
fn the_muzzle_flash() {
    let mut e = setup();
    let (_, res) = call(&mut e, 0x2831, &[r(1)]);
    assert_eq!(string_result(&e, res), b"flash.nif");
    call(&mut e, 0x2832, &[r(1), s(b"muzzle.nif")]);
    assert_eq!(text_at(&e, PROJ + 0xb8), b"muzzle.nif");
    assert_eq!(allocations(), vec![11], "the game's setter, once");
    let long = vec![b'p'; 0x80];
    let stopped = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        call(&mut e, 0x2832, &[r(1), s(&long)]);
    }));
    assert!(
        stopped.is_err(),
        "0x80 bytes run past JIP's 0x80-byte buffer"
    );
}

#[test]
fn a_fired_projectile() {
    let mut e = setup();
    e.mem.set_f32(FP + 0xd8, 1.25);
    e.mem.set_f32(FP + 0x110, 300.5);
    e.mem.set_f32(FP + 0xdc, 40.0);
    e.mem.set_f32(FP + 0xd0, 0.75);
    call_on(&mut e, 0x26dc, FP, &[]);
    assert_eq!(form_result(&e), 0, "no source");
    assert_eq!(call_on(&mut e, 0x26e0, FP, &[]).1, 1.25);
    assert_eq!(call_on(&mut e, 0x26e1, FP, &[]).1, 300.5);
    assert_eq!(call_on(&mut e, 0x26e2, FP, &[]).1, 40.0);
    assert_eq!(call_on(&mut e, 0x26e4, FP, &[]).1, 0.75);
    let want = with_word(&e, FP, PSIZE, 0xfc, REF);
    call_on(&mut e, 0x26dd, FP, &[r(6)]);
    assert_eq!(e.mem.bytes(FP, PSIZE), want, "the source alone");
    call_on(&mut e, 0x26dc, FP, &[]);
    assert_eq!(form_result(&e), 0xff00_0003);
    let want = with_word(&e, FP, PSIZE, 0xf8, WEAP);
    call_on(&mut e, 0x26df, FP, &[r(5)]);
    assert_eq!(e.mem.bytes(FP, PSIZE), want, "the weapon alone");
    call_on(&mut e, 0x26de, FP, &[]);
    assert_eq!(form_result(&e), 0x0028);
    call_on(&mut e, 0x26df, FP, &[r(2)]);
    assert_eq!(
        e.mem.bytes(FP, PSIZE),
        want,
        "a misc item is no weapon: nothing"
    );
    let want = with_word(&e, FP, PSIZE, 0xdc, 12.5f32.to_bits());
    call_on(&mut e, 0x26e3, FP, &[z(12.5)]);
    assert_eq!(e.mem.bytes(FP, PSIZE), want, "the damage alone");
    let want = with_word(&e, FP, PSIZE, 0xd0, 2.0f32.to_bits());
    call_on(&mut e, 0x26e5, FP, &[z(2.0)]);
    assert_eq!(e.mem.bytes(FP, PSIZE), want, "the speed alone");
    let want = with_word(&e, FP, PSIZE, 0xfc, 0);
    call_on(&mut e, 0x26dd, FP, &[]);
    assert_eq!(e.mem.bytes(FP, PSIZE), want, "left out: no source");
}

/// A reference whose vtable does not answer true at +0x224 (filled the same) is no projectile:
/// the getters answer nothing, the setters change nothing; no reference at all, the same.
#[test]
fn not_a_projectile() {
    let mut e = setup();
    for off in (0x88..PSIZE).step_by(4) {
        e.mem.set_u32(NR + off, 0x4120_0000);
    }
    e.mem.set_u32(NR + 0xfc, REF);
    e.mem.set_u32(NR + 0xf8, WEAP);
    e.mem.set_u8(NR + 0x90, 1);
    let before = e.mem.bytes(NR, PSIZE);
    for this in [NR, 0] {
        for get in [0x26dc, 0x26de, 0x28af] {
            call_on(&mut e, get, this, &[]);
            assert_eq!(form_result(&e), 0, "{get:04x}");
        }
        for get in [0x26e0, 0x26e1, 0x26e2, 0x26e4, 0x28be] {
            assert_eq!(call_on(&mut e, get, this, &[]).1, 0.0, "{get:04x}");
        }
        call_on(&mut e, 0x26dd, this, &[r(6)]);
        call_on(&mut e, 0x26df, this, &[r(5)]);
        call_on(&mut e, 0x26e3, this, &[z(1.0)]);
        call_on(&mut e, 0x26e5, this, &[z(1.0)]);
    }
    assert_eq!(e.mem.bytes(NR, PSIZE), before);
}

#[test]
fn what_it_hit() {
    let mut e = setup();
    // Two impacts: the first without a reference (material 1, cloth), the second with REF.
    e.mem.set_u32(IMPACTS + 0x20, 1);
    e.mem.set_u32(IMPACTS + 0x40, REF);
    e.mem.set_u32(FP + 0x88, IMPACTS);
    e.mem.set_u32(FP + 0x8c, NODE);
    e.mem.set_u32(NODE, IMPACTS + 0x40);
    call_on(&mut e, 0x28af, FP, &[]);
    assert_eq!(form_result(&e), 0, "not hit yet");
    assert_eq!(call_on(&mut e, 0x28be, FP, &[]).1, -1.0, "not hit yet: -1");
    e.mem.set_u8(FP + 0x90, 1);
    call_on(&mut e, 0x28af, FP, &[]);
    assert_eq!(
        form_result(&e),
        0xff00_0003,
        "the first impact with a reference"
    );
    assert_eq!(
        call_on(&mut e, 0x28be, FP, &[]).1,
        7.0,
        "the first impact's: cloth"
    );
    for (m, want) in [(0, 0.0), (16, 9.0), (31, 5.0), (32, -1.0)] {
        e.mem.set_u32(IMPACTS + 0x20, m);
        assert_eq!(call_on(&mut e, 0x28be, FP, &[]).1, want, "material {m}");
    }
    e.mem.set_u32(FP + 0x88, 0);
    assert_eq!(
        call_on(&mut e, 0x28be, FP, &[]).1,
        -1.0,
        "an empty first node: -1"
    );
}

#[test]
fn the_detonation_timer() {
    let mut e = setup();
    e.mem.set_f32(FP + 0xe4, 3.5);
    assert_eq!(
        call_on(&mut e, 0x2772, FP, &[]).1,
        -1.0,
        "no alt-trigger flag: -1"
    );
    call_on(&mut e, 0x2773, FP, &[z(9.0)]);
    assert_eq!(e.mem.f32(FP + 0xe4), 3.5, "no flag: nothing");
    e.mem.set_u16(PROJ + 0x60, 4);
    assert_eq!(call_on(&mut e, 0x2772, FP, &[]).1, 3.5);
    let want = with_word(&e, FP, PSIZE, 0xe4, 1.5f32.to_bits());
    call_on(&mut e, 0x2773, FP, &[z(1.5)]);
    assert_eq!(e.mem.bytes(FP, PSIZE), want, "the timer alone");
    // A missile (0x3D) with the same vtable is no grenade.
    e.mem.set_u8(FP + 4, 0x3d);
    assert_eq!(call_on(&mut e, 0x2772, FP, &[]).1, -1.0);
    call_on(&mut e, 0x2773, FP, &[z(9.0)]);
    assert_eq!(e.mem.f32(FP + 0xe4), 1.5);
}

#[test]
fn an_armed_mine() {
    let mut e = setup();
    e.mem.set_u16(PROJ + 0x60, 0x26);
    assert_eq!(call_on(&mut e, 0x2774, FP, &[]).1, 1.0);
    e.mem.set_u16(PROJ + 0x60, 0x426);
    assert_eq!(call_on(&mut e, 0x2774, FP, &[]).1, 0.0, "detonates: no");
    e.mem.set_u16(PROJ + 0x60, 0x24);
    assert_eq!(
        call_on(&mut e, 0x2774, FP, &[]).1,
        0.0,
        "no explosion flag: no"
    );
    e.mem.set_u16(PROJ + 0x60, 0x1026);
    assert_eq!(
        call_on(&mut e, 0x2774, FP, &[]).1,
        1.0,
        "other flags kept out of the mask"
    );
    e.mem.set_u32(FP + 0xc8, 0x200);
    assert_eq!(
        call_on(&mut e, 0x2774, FP, &[]).1,
        0.0,
        "its own flag 0x200: no"
    );
    e.mem.set_u32(FP + 0xc8, 0x100);
    e.mem.set_u32(FP + 0xfc, PC);
    assert_eq!(call_on(&mut e, 0x2774, FP, &[]).1, 0.0, "the player's: no");
    e.mem.set_u32(FP + 0xfc, REF);
    assert_eq!(call_on(&mut e, 0x2774, FP, &[]).1, 1.0);
    e.mem.set_u8(FP + 4, 0x3d);
    assert_eq!(call_on(&mut e, 0x2774, FP, &[]).1, 0.0, "not a grenade: no");
}

/// The base form commands read the projectile as a bound object or any form, then check
/// `IS_ID`: a misc item filled with a non-zero pattern answers nothing and keeps its bytes.
#[test]
fn another_form_type_is_left_alone() {
    let mut e = setup();
    for off in 0x10..BSIZE {
        e.mem.set_u8(MISC + off, 0xa5);
    }
    let before = e.mem.bytes(MISC, BSIZE);
    for t in [0, 1, 14] {
        assert_eq!(call(&mut e, 0x2615, &[r(2), n(t)]).1, 0.0, "trait {t}");
        call(&mut e, 0x2616, &[r(2), n(t), z(2.0)]);
    }
    assert_eq!(call(&mut e, 0x2617, &[r(2), n(0)]).1, 0.0);
    call(&mut e, 0x2618, &[r(2), n(0), n(0)]);
    for get in [0x2619, 0x2932] {
        call(&mut e, get, &[r(2)]);
        assert_eq!(form_result(&e), 0, "{get:04x}");
    }
    call(&mut e, 0x261a, &[r(2), r(3)]);
    call(&mut e, 0x2933, &[r(2), r(4)]);
    let (_, res) = call(&mut e, 0x2831, &[r(2)]);
    assert_eq!(string_result(&e, res), b"");
    call(&mut e, 0x2832, &[r(2), s(b"x.nif")]);
    assert_eq!(e.mem.bytes(MISC, BSIZE), before);
    assert!(allocations().is_empty());
}
