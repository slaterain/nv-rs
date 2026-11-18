//! Tests of `SetInternalMarker` by its opcode (functions_jip/jip_fn_miscellaneous.h), and of the
//! set reaching JIP's `IsMarker` (H22's wrap at `0050F9E0`, internal/hooks.h:
//! `GetIsInternalMarkerHook`).

use crate::commands::test_support::*;
use engine_core::prelude::*;

const SET_INTERNAL_MARKER: u32 = 0x2902;
const IS_MARKER: u32 = 0x0050_f9e0;
const STATIC: u32 = FORMS + 0x1000;
const REFR: u32 = FORMS + 0x2000;
const PLAIN_VT: u32 = FORMS + 0x8000;
const REF_VT: u32 = FORMS + 0x8800;

fn setup() -> Engine {
    let mut e = engine();
    // A static (type 0x20) with id 0x0012_3456, and a reference to it.
    e.mem.set_u32(STATIC, PLAIN_VT);
    e.mem.set_u8(STATIC + 4, 0x20);
    e.mem.set_u32(STATIC + 0xc, 0x0012_3456);
    e.mem.set_u32(REFR, REF_VT);
    e.mem.set_u8(REFR + 4, 0x3a);
    e.mem.set_u32(REFR + 0xc, 0x0012_9999);
    e.mem.set_u32(REFR + 0x20, STATIC);
    e.mem.set_u32(REF_VT + 0xf0, 0x008d_0360);
    set_ref(&mut e, 1, STATIC);
    set_ref(&mut e, 2, REFR);
    e
}

fn ask(e: &mut Engine, r_index: u16) -> f64 {
    call(e, SET_INTERNAL_MARKER, &[r(r_index)]).1
}

fn is_marker(e: &mut Engine, form: u32) -> u32 {
    e.call(IS_MARKER, &[form]).eax & 0xff
}

#[test]
fn set_internal_marker_asks_adds_and_removes_by_id() {
    let mut e = setup();
    assert_eq!(ask(&mut e, 1), 0.0);
    assert_eq!(is_marker(&mut e, STATIC), 0);
    call(&mut e, SET_INTERNAL_MARKER, &[r(1), n(1)]);
    assert_eq!(ask(&mut e, 1), 1.0);
    assert_eq!(is_marker(&mut e, STATIC), 1, "IsMarker reads the set");
    // A reference stands for its base form.
    assert_eq!(ask(&mut e, 2), 1.0);
    call(&mut e, SET_INTERNAL_MARKER, &[r(2), n(0)]);
    assert_eq!(ask(&mut e, 1), 0.0);
    assert_eq!(is_marker(&mut e, STATIC), 0);
    // A negative number only asks.
    call(&mut e, SET_INTERNAL_MARKER, &[r(1), n(-1)]);
    assert_eq!(ask(&mut e, 1), 0.0);
    call(&mut e, SET_INTERNAL_MARKER, &[r(1), n(5)]);
    assert_eq!(ask(&mut e, 1), 1.0);
}

#[test]
fn jips_own_ids_start_in_the_set_and_can_be_taken_out() {
    let mut e = setup();
    // The id 0x10 (one of the exe's marker statics) is in at the start.
    e.mem.set_u32(STATIC + 0xc, 0x10);
    assert_eq!(ask(&mut e, 1), 1.0);
    assert_eq!(is_marker(&mut e, STATIC), 1);
    call(&mut e, SET_INTERNAL_MARKER, &[r(1), n(0)]);
    assert_eq!(ask(&mut e, 1), 0.0);
    assert_eq!(is_marker(&mut e, STATIC), 0);
}
