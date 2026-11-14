//! Tests of `ln_fn_misc_ref`: references with their extras (activate ref 0x53 flags +0x14,
//! container changes 0x15, action 0x0E, radius 0x5C), the game's weight function and the 3D
//! calls as stand-ins, a light's radius at +0xA0.

use super::*;
use crate::commands::test_support::*;

const REFR: u32 = FORMS + 0x1000;
const OTHER: u32 = FORMS + 0x1100;
const BASE: u32 = FORMS + 0x1200;
const NEW_BASE: u32 = FORMS + 0x1300;
const LIGH: u32 = FORMS + 0x1400;
const MISC: u32 = FORMS + 0x1500;
const PC: u32 = FORMS + 0x1600;
const XA: u32 = FORMS + 0x2000;
const XC: u32 = FORMS + 0x2100;
const CDATA: u32 = FORMS + 0x2200;
const XR: u32 = FORMS + 0x2300;

fn setup() -> Engine {
    let mut e = engine();
    extra_stand_ins(&mut e);
    form(&mut e, REFR, 0x3a, 0x0000_0100);
    form(&mut e, OTHER, 0x3a, 0x0000_0101);
    form(&mut e, BASE, 0x1f, 0x0000_0102);
    form(&mut e, NEW_BASE, 0x1f, 0x0000_0103);
    form(&mut e, LIGH, 0x1e, 0x0000_0104);
    form(&mut e, MISC, 0x1f, 0x0000_0105);
    e.mem.set_u32(REFR + 0x20, BASE);
    e.mem.set_u32(0x011d_ea3c, PC);
    for (i, f) in [OTHER, NEW_BASE, LIGH, MISC].iter().enumerate() {
        set_ref(&mut e, i as u16 + 1, *f);
    }
    // `kParamType_TESObject`: xNVSE's reader asks `IsBoundObject`.
    let vtbl = FORMS + 0x6000;
    e.mem.set_u32(NEW_BASE, vtbl);
    e.mem
        .set_u32(vtbl + nvse::vtable::IS_BOUND_OBJECT, 0x0050_1000);
    e.register(0x0050_1000, |_, _| Ret {
        eax: 1,
        ..Ret::default()
    });
    e
}

#[test]
fn activate_parent_flags() {
    let mut e = setup();
    put_extra(&mut e, OTHER + 0x44, XA, 0x53, 0);
    e.mem.set_u32(XA + 0x14, 3);
    assert_eq!(call(&mut e, 0x2280, &[r(1)]).1, 3.0);
    assert_eq!(call_on(&mut e, 0x2280, OTHER, &[]).1, 3.0);
    assert_eq!(call_on(&mut e, 0x2280, REFR, &[]).1, 0.0);
    assert_eq!(call(&mut e, 0x2280, &[]).1, 0.0, "no reference at all");
}

#[test]
fn a_new_base_form() {
    let mut e = setup();
    // No 3D: only the base form changes.
    call_on(&mut e, 0x2282, REFR, &[r(2)]);
    assert_eq!(e.mem.u32(REFR + 0x20), NEW_BASE);
    assert!(calls().is_empty());
    // Inside an inventory (`containingObj`): nothing.
    let mut words = crate::test_support::line(&mut e, 0x2282, &[r(1)]);
    words[2] = REFR;
    words[3] = OTHER;
    let exec = crate::test_support::execute_of(&e, 0x2282);
    e.call(exec, &words);
    assert_eq!(e.mem.u32(REFR + 0x20), NEW_BASE);
}

#[test]
fn inventory_weight() {
    let mut e = setup();
    e.mem.set_u8(PC + 0x7bc, 1);
    e.register(0x004d_0900, |e, w| {
        assert_eq!(
            e.mem.u8(0x011e_0898),
            1,
            "the game's flag set around the call"
        );
        assert_eq!(e.mem.u32(w[0] + 8), 0xbf80_0000, "the cached weight reset");
        Ret {
            st0: 37.5 + f64::from(w[1]),
            ..Ret::default()
        }
    });
    e.map(0x011e_0000, 0x1000);
    assert_eq!(call_on(&mut e, 0x22c1, REFR, &[]).1, 0.0, "no changes");
    put_extra(&mut e, REFR + 0x44, XC, 0x15, CDATA);
    assert_eq!(call_on(&mut e, 0x22c1, REFR, &[]).1, 38.5);
    assert_eq!(e.mem.u8(0x011e_0898), 0);
    e.mem.set_u32(XC + 0xc, 0);
    assert_eq!(call_on(&mut e, 0x22c1, REFR, &[]).1, 0.0);
}

#[test]
fn reference_flags() {
    let mut e = setup();
    e.mem.set_u32(REFR + 8, 0x0000_0420);
    assert_eq!(call_on(&mut e, 0x22d6, REFR, &[n(0x400)]).1, 1.0);
    assert_eq!(call_on(&mut e, 0x22d6, REFR, &[n(0x800)]).1, 0.0);
    call_on(&mut e, 0x22d7, REFR, &[n(0x801), n(1)]);
    assert_eq!(e.mem.u32(REFR + 8), 0xc21);
    call_on(&mut e, 0x22d7, REFR, &[n(0x420), n(0)]);
    assert_eq!(e.mem.u32(REFR + 8), 0x801);
    assert!(calls().is_empty(), "nothing marked");
    // The condition's eval: the flag as its first parameter.
    let record = nvse::table::record_of(&e, 0x22d6);
    let at = e.mem.u32(record + 0x20);
    e.mem.set_f64(RESULT, 0.0);
    e.call(at, &[REFR, 0x800, 0, RESULT]);
    assert_eq!(e.mem.f64(RESULT), 1.0);
    e.mem.set_f64(RESULT, 0.0);
    e.call(at, &[REFR, 0x400, 0, RESULT]);
    assert_eq!(e.mem.f64(RESULT), 0.0);
}

#[test]
fn action_ref() {
    let mut e = setup();
    assert_eq!(call_on(&mut e, 0x2318, REFR, &[]).1, 0.0);
    // Bit 6 of byte 1 (type 0x0E).
    e.mem.set_u8(REFR + 0x44 + 8 + 1, 0x40);
    assert_eq!(call_on(&mut e, 0x2318, REFR, &[]).1, 1.0);
}

#[test]
fn radius() {
    let mut e = setup();
    e.mem.set_u32(LIGH + 0xa0, 256);
    assert_eq!(call(&mut e, 0x2334, &[r(3)]).1, 256.0);
    call(&mut e, 0x2335, &[z(99.9), r(3)]);
    assert_eq!(e.mem.u32(LIGH + 0xa0), 99, "toward zero");
    call(&mut e, 0x2335, &[z(-5.5), r(3)]);
    assert_eq!(e.mem.u32(LIGH + 0xa0) as i32, -5);
    assert_eq!(call(&mut e, 0x2334, &[r(3)]).1, -5.0);
    // Another form: nothing.
    let before = e.mem.bytes(MISC, 0xc8);
    call(&mut e, 0x2335, &[z(5.0), r(4)]);
    assert_eq!(e.mem.bytes(MISC, 0xc8), before);
    assert_eq!(call(&mut e, 0x2334, &[r(4)]).1, 0.0);
    // No light: the reference's ExtraRadius, made when missing.
    assert_eq!(call_on(&mut e, 0x2334, REFR, &[]).1, 0.0);
    let block = heap_next(&e);
    call_on(&mut e, 0x2335, REFR, &[z(12.5)]);
    assert_eq!(
        [0, 4, 8].map(|o| e.mem.u32(block + o)),
        [0x0101_5208, 0x5c, 0]
    );
    assert_eq!(e.mem.f32(block + 0xc), 12.5);
    assert_eq!(calls_to(ADD_EXTRA_FN), vec![vec![REFR + 0x44, block]]);
    assert_eq!(call_on(&mut e, 0x2334, REFR, &[]).1, 12.5);
    call_on(&mut e, 0x2335, REFR, &[z(3.0)]);
    assert_eq!(e.mem.f32(block + 0xc), 3.0);
    assert_eq!(allocations().len(), 1);
    // No light and no reference: nothing.
    call(&mut e, 0x2335, &[z(3.0)]);
    let _ = XR;
}
