//! Tests of the ammo effect commands: the ammo's effect list at its raw offset +0xD4 (first node
//! inline), an effect's type +0x24, operation +0x28 and value +0x2C (JIP's header). Casts as MSVC
//! compiles JIP's `UInt32 iVal = (int)fVal`.

use super::*;
use crate::commands::test_support::*;

const AMMO: u32 = FORMS + 0x1000;
const FX_A: u32 = FORMS + 0x2000;
const FX_B: u32 = FORMS + 0x2100;
const FX_C: u32 = FORMS + 0x2200;
const OTHER: u32 = FORMS + 0x2300;
const NODES: u32 = FORMS + 0x3000;

fn setup() -> Engine {
    let mut e = engine();
    form(&mut e, AMMO, 0x29, 0x0000_a330);
    e.mem.set_u32(AMMO, 0x0102_6064);
    form(&mut e, FX_A, 0x72, 0x0000_0fa1);
    form(&mut e, FX_B, 0x72, 0x0000_0fb2);
    form(&mut e, FX_C, 0x72, 0x0000_0fc3);
    form(&mut e, OTHER, 0x36, 0x0000_0036);
    for (i, f) in [AMMO, FX_A, FX_B, FX_C, OTHER].iter().enumerate() {
        set_ref(&mut e, i as u16 + 1, *f);
    }
    CALLS.with(|c| c.borrow_mut().clear());
    e
}

/// The effect list from the inline first node at +0xD4.
fn set_list(e: &mut Engine, items: &[u32]) {
    e.mem.set_u32(AMMO + 0xd4, 0);
    e.mem.set_u32(AMMO + 0xd8, 0);
    let mut prev = AMMO + 0xd4;
    for (i, &item) in items.iter().enumerate() {
        let node = if i == 0 {
            AMMO + 0xd4
        } else {
            NODES + 8 * i as u32
        };
        e.mem.set_u32(node, item);
        e.mem.set_u32(node + 4, 0);
        if i > 0 {
            e.mem.set_u32(prev + 4, node);
        }
        prev = node;
    }
}

fn items(e: &Engine) -> Vec<u32> {
    let (mut out, mut node) = (Vec::new(), AMMO + 0xd4);
    while node != 0 {
        out.push(e.mem.u32(node));
        node = e.mem.u32(node + 4);
    }
    out
}

#[test]
fn the_effect_list() {
    let mut e = setup();
    assert_eq!(call(&mut e, 0x260f, &[r(1)]).1, 0.0, "empty");
    set_list(&mut e, &[FX_A, FX_B]);
    assert_eq!(call(&mut e, 0x260f, &[r(1)]).1, 2.0);
    call(&mut e, 0x2610, &[r(1), n(1)]);
    assert_eq!(form_result(&e), 0x0fb2);
    call(&mut e, 0x2610, &[r(1), n(2)]);
    assert_eq!(form_result(&e), 0);
    call(&mut e, 0x2610, &[r(1), n(-1)]);
    assert_eq!(form_result(&e), 0);
    assert_eq!(call(&mut e, 0x260f, &[r(5)]).1, 0.0, "not an ammo");
}

#[test]
fn effects_added_and_removed() {
    let mut e = setup();
    set_list(&mut e, &[FX_A, FX_B]);
    assert_eq!(call(&mut e, 0x2611, &[r(1), r(2)]).1, 0.0, "already there");
    assert_eq!(call(&mut e, 0x2611, &[r(1), r(4)]).1, 1.0);
    assert_eq!(items(&e), vec![FX_C, FX_A, FX_B], "put first");
    assert_eq!(call(&mut e, 0x2611, &[r(1), r(5)]).1, 0.0, "not an effect");
    // Removed: the first (the next node's item moves in), a later one, then the last one.
    assert_eq!(call(&mut e, 0x2612, &[r(1), r(4)]).1, 1.0);
    assert_eq!(items(&e), vec![FX_A, FX_B]);
    assert_eq!(call(&mut e, 0x2612, &[r(1), r(3)]).1, 1.0);
    assert_eq!(items(&e), vec![FX_A]);
    assert_eq!(call(&mut e, 0x2612, &[r(1), r(3)]).1, 0.0, "not there");
    assert_eq!(call(&mut e, 0x2612, &[r(1), r(2)]).1, 1.0);
    assert_eq!(items(&e), vec![0], "the first node emptied");
    assert_eq!(freed().len(), 2, "the two nodes taken out");
    // Into an empty list: the first node.
    assert_eq!(call(&mut e, 0x2611, &[r(1), r(2)]).1, 1.0);
    assert_eq!(items(&e), vec![FX_A]);
}

#[test]
fn the_effect_numbers() {
    let mut e = setup();
    e.mem.set_u32(FX_A + 0x24, 3);
    e.mem.set_u32(FX_A + 0x28, 0xffff_ffff);
    e.mem.set_f32(FX_A + 0x2c, 0.5);
    let get = |e: &mut Engine, t: i32| call(e, 0x2613, &[r(2), n(t)]).1;
    assert_eq!(
        (get(&mut e, 0), get(&mut e, 1), get(&mut e, 2)),
        (3.0, -1.0, 0.5)
    );
    assert_eq!(get(&mut e, 3), 0.0);
    let set = |e: &mut Engine, t: i32, v: f64| {
        call(e, 0x2614, &[r(2), n(t), z(v)]);
    };
    set(&mut e, 0, 5.9);
    assert_eq!(e.mem.u32(FX_A + 0x24), 5, "(int)5.9");
    set(&mut e, 0, 6.0);
    assert_eq!(e.mem.u32(FX_A + 0x24), 5, "6 is not below 6");
    set(&mut e, 1, 2.99);
    assert_eq!(e.mem.u32(FX_A + 0x28), 2);
    set(&mut e, 1, 3.0);
    assert_eq!(e.mem.u32(FX_A + 0x28), 2);
    set(&mut e, 2, 7.25);
    assert_eq!(e.mem.f32(FX_A + 0x2c), 7.25);
    set(&mut e, 2, -0.5);
    assert_eq!(e.mem.f32(FX_A + 0x2c), 7.25, "below 0: nothing");
    set(&mut e, 2, f64::NAN);
    assert_eq!(e.mem.f32(FX_A + 0x2c), 7.25, "NaN fails fVal >= 0");
    // 3e9 is not below 0; (int)3e9 is INT_MIN, as UInt32 0x80000000: not below 6.
    set(&mut e, 0, 3e9);
    assert_eq!(e.mem.u32(FX_A + 0x24), 5);
}
