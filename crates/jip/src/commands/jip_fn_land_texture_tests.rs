//! Tests of the land texture commands, at the raw offsets of the land texture loader (`005412A0`:
//! texture set +0x18, material +0x1C, friction +0x1D, restitution +0x1E, specular +0x1F) and the
//! grass list's first node at +0x20.

use super::*;
use crate::commands::test_support::*;

const LTEX: u32 = FORMS + 0x1000;
const TXST: u32 = FORMS + 0x2000;
const GRASS1: u32 = FORMS + 0x2100;
const GRASS2: u32 = FORMS + 0x2200;
const MISC: u32 = FORMS + 0x2300;

fn setup() -> Engine {
    let mut e = engine();
    form(&mut e, LTEX, 0x12, 0x0000_1e70);
    form(&mut e, TXST, 0x04, 0x0000_0777);
    form(&mut e, GRASS1, 0x24, 0x0000_0601);
    form(&mut e, GRASS2, 0x24, 0x0000_0602);
    form(&mut e, MISC, 0x1f, 0x0000_0999);
    for (i, f) in [LTEX, TXST, GRASS1, GRASS2, MISC].iter().enumerate() {
        set_ref(&mut e, i as u16 + 1, *f);
    }
    e
}

#[test]
fn the_texture_set() {
    let mut e = setup();
    call(&mut e, 0x265d, &[r(1)]);
    assert_eq!(form_result(&e), 0, "none");
    call(&mut e, 0x265e, &[r(1), r(2)]);
    assert_eq!(e.mem.u32(LTEX + 0x18), TXST);
    call(&mut e, 0x265d, &[r(1)]);
    assert_eq!(form_result(&e), 0x777);
    call(&mut e, 0x265e, &[r(1), r(5)]);
    assert_eq!(e.mem.u32(LTEX + 0x18), TXST, "a misc item: nothing");
    call(&mut e, 0x265e, &[r(5), r(2)]);
    assert_eq!(e.mem.u32(MISC + 0x18), 0, "not a land texture");
}

#[test]
fn the_numbers() {
    let mut e = setup();
    let get = |e: &mut Engine, t: i32| call(e, 0x265f, &[r(1), n(t)]).1;
    let set = |e: &mut Engine, t: i32, v: i32| {
        call(e, 0x2660, &[r(1), n(t), n(v)]);
    };
    e.mem.set_u32(LTEX + 0x1c, 0x0403_0201);
    for t in 0..4 {
        assert_eq!(get(&mut e, t), f64::from(t + 1), "trait {t}");
    }
    e.mem.set_u8(LTEX + 0x20, 9);
    assert_eq!(get(&mut e, 4), 0.0, "trait 4: nothing");
    set(&mut e, 0, 31);
    assert_eq!(e.mem.u8(LTEX + 0x1c), 31);
    set(&mut e, 0, 32);
    assert_eq!(e.mem.u8(LTEX + 0x1c), 31, "material 32 refused");
    set(&mut e, 0, -1);
    assert_eq!(e.mem.u8(LTEX + 0x1c), 31, "-1 is 0xFFFFFFFF: refused");
    set(&mut e, 1, 300);
    assert_eq!(e.mem.u8(LTEX + 0x1d), 44, "the low byte");
    set(&mut e, 2, -1);
    assert_eq!(e.mem.u8(LTEX + 0x1e), 0xff);
    set(&mut e, 3, 77);
    assert_eq!(e.mem.u8(LTEX + 0x1f), 77);
    set(&mut e, 4, 5);
    assert_eq!(e.mem.u8(LTEX + 0x20), 9, "trait 4: nothing");
}

#[test]
fn the_grasses() {
    let mut e = setup();
    let count = |e: &mut Engine| call(e, 0x2661, &[r(1)]).1;
    assert_eq!(count(&mut e), 0.0);
    assert_eq!(call(&mut e, 0x2663, &[r(1), r(3)]).1, 1.0);
    assert_eq!(e.mem.u32(LTEX + 0x20), GRASS1, "into the empty first node");
    assert_eq!(call(&mut e, 0x2663, &[r(1), r(3)]).1, 0.0, "already there");
    assert_eq!(call(&mut e, 0x2663, &[r(1), r(5)]).1, 0.0, "a misc item");
    assert_eq!(call(&mut e, 0x2663, &[r(1), r(4)]).1, 1.0);
    assert_eq!(count(&mut e), 2.0);
    // Prepended: grass 2 first.
    call(&mut e, 0x2662, &[r(1), n(0)]);
    assert_eq!(form_result(&e), 0x602);
    call(&mut e, 0x2662, &[r(1), n(1)]);
    assert_eq!(form_result(&e), 0x601);
    call(&mut e, 0x2662, &[r(1), n(2)]);
    assert_eq!(form_result(&e), 0, "past the end");
    call(&mut e, 0x2662, &[r(1), n(-1)]);
    assert_eq!(form_result(&e), 0, "a negative index");
    assert_eq!(call(&mut e, 0x2664, &[r(1), r(5)]).1, 0.0, "a misc item");
    assert_eq!(call(&mut e, 0x2664, &[r(1), r(3)]).1, 1.0);
    assert_eq!(call(&mut e, 0x2664, &[r(1), r(3)]).1, 0.0, "gone");
    assert_eq!(count(&mut e), 1.0);
    assert_eq!(call(&mut e, 0x2661, &[r(5)]).1, 0.0, "not a land texture");
}
