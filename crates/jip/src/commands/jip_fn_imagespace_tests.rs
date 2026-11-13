//! Tests of the image space commands, at raw offsets: the image space's 33 floats from +0x18
//! (its loader `0052B440`); the modifier's bytes and floats (+0x20, +0x24, +0xE8, +0xEC, +0xF0,
//! +0x100, +0x101), its interpolators' values (`fltIntrpl1` +0x114 + 0x18 * i + 0xC, `clrIntrpl`
//! +0x534 + 0x24 * c + 0xC, `fltIntrpl2` +0x57C + 0x18 * i + 0xC) and its key arrays (+0x654,
//! +0x704, +0x70C, the modifier loader `0052D510`), each key's value at +4.

use super::*;
use crate::commands::test_support::*;

const IMGS: u32 = FORMS + 0x1000;
const IMAD: u32 = FORMS + 0x2000;
const KEYS: u32 = FORMS + 0x4000;

fn setup() -> Engine {
    let mut e = engine();
    form(&mut e, IMGS, 0x53, 0x0000_1350);
    form(&mut e, IMAD, 0x54, 0x0000_1354);
    set_ref(&mut e, 1, IMGS);
    set_ref(&mut e, 2, IMAD);
    // One key block per key array entry: 44 float keys, 2 colour keys, 9 float keys.
    for i in 0..44 {
        e.mem.set_u32(IMAD + 0x654 + 4 * i, KEYS + 0x10 * i);
    }
    for c in 0..2 {
        e.mem.set_u32(IMAD + 0x704 + 4 * c, KEYS + 0x400 + 0x20 * c);
    }
    for i in 0..9 {
        e.mem.set_u32(IMAD + 0x70c + 4 * i, KEYS + 0x500 + 0x10 * i);
    }
    e
}

#[test]
fn image_space_values() {
    let mut e = setup();
    let get = |e: &mut Engine, t: i32| call(e, 0x27cc, &[r(1), n(t)]).1;
    let set = |e: &mut Engine, t: i32, v: f64| {
        call(e, 0x27cd, &[r(1), n(t), z(v)]);
    };
    for t in 0..=32u32 {
        e.mem.set_f32(IMGS + 0x18 + 4 * t, 0.5);
        let colour = (21..=23).contains(&t) || (29..=31).contains(&t);
        let want = if colour { 0.5f32 * 255.0 } else { 0.5 };
        assert_eq!(get(&mut e, t as i32), f64::from(want), "trait {t}");
        set(&mut e, t as i32, 51.0);
        let want = if colour {
            51.0f32 * (1.0 / 255.0)
        } else {
            51.0
        };
        assert_eq!(e.mem.f32(IMGS + 0x18 + 4 * t), want, "trait {t} set");
    }
    e.mem.set_f32(IMGS + 0x18 + 4 * 33, 7.0);
    assert_eq!(get(&mut e, 33), 0.0, "trait 33");
    set(&mut e, 33, 1.0);
    assert_eq!(e.mem.f32(IMGS + 0x18 + 4 * 33), 7.0, "trait 33: nothing");
    // The float products, as JIP's float maths gives them (not the doubles').
    e.mem.set_f32(IMGS + 0x18 + 4 * 21, 0.1);
    assert_eq!(get(&mut e, 21), f64::from(0.1f32 * 255.0));
}

#[test]
fn modifier_numbers_and_bytes() {
    let mut e = setup();
    let get = |e: &mut Engine, t: i32| call(e, 0x27ce, &[r(2), n(t)]).1;
    let set = |e: &mut Engine, t: i32, v: f64| {
        call(e, 0x27cf, &[r(2), n(t), z(v)]);
    };
    for (t, off) in [(0, 0x24), (1, 0xec), (2, 0xf0)] {
        e.mem.set_f32(IMAD + off, 1.25);
        assert_eq!(get(&mut e, t), 1.25, "trait {t}");
        set(&mut e, t, -4.5);
        assert_eq!(e.mem.f32(IMAD + off), -4.5, "trait {t} set");
    }
    for (t, off) in [(64, 0x20), (65, 0xe8), (66, 0x100), (67, 0x101)] {
        e.mem.set_u8(IMAD + off, 2);
        assert_eq!(get(&mut e, t), 2.0, "trait {t}");
        // UInt8(value) & 3: toward zero, low byte, low two bits.
        set(&mut e, t, 7.9);
        assert_eq!(e.mem.u8(IMAD + off), 3, "trait {t}: 7 & 3");
        set(&mut e, t, -2.5);
        assert_eq!(e.mem.u8(IMAD + off), 2, "trait {t}: 0xFE & 3");
        set(&mut e, t, f64::NAN);
        assert_eq!(e.mem.u8(IMAD + off), 0, "trait {t}: NaN");
        set(&mut e, t, 3e9);
        assert_eq!(e.mem.u8(IMAD + off), 0, "trait {t}: past the range");
    }
    let before = e.mem.bytes(IMAD, 0x730);
    assert_eq!(get(&mut e, 68), 0.0, "trait 68");
    set(&mut e, 68, 1.0);
    assert_eq!(e.mem.bytes(IMAD, 0x730), before, "trait 68: nothing");
}

#[test]
fn modifier_animated_values() {
    let mut e = setup();
    let get = |e: &mut Engine, t: i32| call(e, 0x27ce, &[r(2), n(t)]).1;
    let set = |e: &mut Engine, t: i32, v: f64| {
        call(e, 0x27cf, &[r(2), n(t), z(v)]);
    };
    // data654 / fltIntrpl1: traits 3-46.
    for t in [3u32, 20, 46] {
        let i = t - 3;
        e.mem.set_f32(KEYS + 0x10 * i + 4, 0.75);
        assert_eq!(get(&mut e, t as i32), 0.75, "trait {t}");
        set(&mut e, t as i32, 2.5);
        assert_eq!(e.mem.f32(KEYS + 0x10 * i + 4), 2.5, "trait {t}: the key");
        assert_eq!(
            e.mem.f32(IMAD + 0x114 + 0x18 * i + 0xc),
            2.5,
            "trait {t}: interpolator"
        );
        assert_eq!(
            e.mem.f32(KEYS + 0x10 * i),
            0.0,
            "trait {t}: the key's time kept"
        );
    }
    // The two colours: traits 47-50 and 51-54, 0-255 shown, 0-1 kept.
    for (t, c, k) in [(47u32, 0u32, 0u32), (50, 0, 3), (51, 1, 0), (54, 1, 3)] {
        let key = KEYS + 0x400 + 0x20 * c + 4 + 4 * k;
        e.mem.set_f32(key, 0.2);
        assert_eq!(
            get(&mut e, t as i32),
            f64::from(0.2f32 * 255.0),
            "trait {t}"
        );
        set(&mut e, t as i32, 102.0);
        let want = 102.0f32 * (1.0 / 255.0);
        assert_eq!(e.mem.f32(key), want, "trait {t}: the key");
        assert_eq!(
            e.mem.f32(IMAD + 0x534 + 0x24 * c + 0xc + 4 * k),
            want,
            "trait {t}: interpolator"
        );
    }
    // data70C / fltIntrpl2: traits 55-63.
    for t in [55u32, 63] {
        let i = t - 55;
        e.mem.set_f32(KEYS + 0x500 + 0x10 * i + 4, -1.0);
        assert_eq!(get(&mut e, t as i32), -1.0, "trait {t}");
        set(&mut e, t as i32, 9.0);
        assert_eq!(
            e.mem.f32(KEYS + 0x500 + 0x10 * i + 4),
            9.0,
            "trait {t}: the key"
        );
        assert_eq!(
            e.mem.f32(IMAD + 0x57c + 0x18 * i + 0xc),
            9.0,
            "trait {t}: interpolator"
        );
    }
}

/// Both forms are read by type (`kParamType_ImageSpace`, `kParamType_ImageSpaceModifier`): a
/// misc item holding the same bytes, and each form where the other is wanted, are refused by
/// xNVSE's reader, so every command answers 0 and leaves the bytes.
#[test]
fn other_form_types_are_left_alone() {
    let mut e = setup();
    const MISC: u32 = FORMS + 0x8000;
    let bytes = e.mem.bytes(IMAD, 0x730);
    e.mem.write(MISC, &bytes);
    form(&mut e, MISC, 0x1f, 0x0000_0999);
    e.mem.set_f32(MISC + 0x24, 1.0);
    set_ref(&mut e, 3, MISC);
    let misc = e.mem.bytes(MISC, 0x730);
    let keys = e.mem.bytes(KEYS, 0x600);
    let imgs = e.mem.bytes(IMGS, 0xb0);
    let imad = e.mem.bytes(IMAD, 0x730);
    for (who, other) in [(3u16, 3u16), (2, 1)] {
        for t in [0, 21, 32] {
            assert_eq!(
                call(&mut e, 0x27cc, &[r(who), n(t)]).1,
                0.0,
                "image space {t}"
            );
            call(&mut e, 0x27cd, &[r(who), n(t), z(5.0)]);
        }
        for t in [0, 3, 47, 55, 64] {
            assert_eq!(
                call(&mut e, 0x27ce, &[r(other), n(t)]).1,
                0.0,
                "modifier {t}"
            );
            call(&mut e, 0x27cf, &[r(other), n(t), z(5.0)]);
        }
    }
    assert_eq!(e.mem.bytes(MISC, 0x730), misc);
    assert_eq!(e.mem.bytes(KEYS, 0x600), keys);
    assert_eq!(e.mem.bytes(IMGS, 0xb0), imgs);
    assert_eq!(e.mem.bytes(IMAD, 0x730), imad);
}

/// GetActiveIMODs: the TES (`*0x011DEA10`) and its running modifiers (+0x6C): form instances
/// (vtable `0102D12C`) not hidden (+8), their modifier at +0x1C; another instance kind, a hidden
/// one and one without a modifier are left out.
#[test]
fn the_running_modifiers() {
    use nvse::vars::{Container, Data};
    let mut e = setup();
    e.map(0x011d_e000, 0x1000);
    const TES: u32 = FORMS + 0x8000;
    const INST: u32 = FORMS + 0x9000;
    const NODES: u32 = FORMS + 0xa000;
    const IMAD2: u32 = FORMS + 0x3000;
    form(&mut e, IMAD2, 0x54, 0x0000_2354);
    e.mem.set_u32(SCRIPT + 0xc, 0x0400_0001);
    assert_eq!(call(&mut e, 0x28a0, &[]).1, 0.0, "no TES: none");
    e.mem.set_u32(0x011d_ea10, TES);
    assert_eq!(call(&mut e, 0x28a0, &[]).1, 0.0, "an empty list: no array");
    // Instances 0-4: IMAD2, hidden IMAD, a temporary instance with IMAD, no modifier, IMAD.
    let inst = |i: u32| INST + 0x40 * i;
    for (i, (vtbl, hidden, imod)) in [
        (0x0102_d12c, 0u8, IMAD2),
        (0x0102_d12c, 1, IMAD),
        (0x0102_d000, 0, IMAD),
        (0x0102_d12c, 0, 0),
        (0x0102_d12c, 0, IMAD),
    ]
    .iter()
    .enumerate()
    {
        let at = inst(i as u32);
        e.mem.set_u32(at, *vtbl);
        e.mem.set_u8(at + 8, *hidden);
        e.mem.set_u32(at + 0x1c, *imod);
    }
    e.mem.set_u32(TES + 0x6c, inst(0));
    e.mem.set_u32(TES + 0x70, NODES);
    for i in 1..5u32 {
        let node = NODES + 8 * (i - 1);
        e.mem.set_u32(node, inst(i));
        e.mem.set_u32(node + 4, if i < 4 { node + 8 } else { 0 });
    }
    let (_, res) = call(&mut e, 0x28a0, &[]);
    let a = nvse::vars::array(&e, res as u32).expect("an array");
    assert_eq!((a.packed, a.owning_mod), (true, 4));
    let Container::Array(v) = &a.elements else {
        panic!("a packed array")
    };
    let got: Vec<Data> = v.iter().map(|el| el.data.clone()).collect();
    assert_eq!(
        got,
        vec![Data::Form(0x2354), Data::Form(0x1354)],
        "in order"
    );
}
