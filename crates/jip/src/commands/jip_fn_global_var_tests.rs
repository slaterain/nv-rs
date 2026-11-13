//! Tests of the global commands, at the raw offsets of the global loader (`005A64B0`: the value
//! +0x24) and the form's flags (+8, JIP's mark 0x200).

use super::*;
use crate::commands::test_support::*;

const GLOB: u32 = FORMS + 0x1000;
const MISC: u32 = FORMS + 0x2000;
/// `g_BGSSaveLoadGame` and a stand-in for the object it points to.
const SAVE_LOAD_GAME: u32 = 0x011d_df38;
const SL: u32 = FORMS + 0x8000;

fn setup() -> Engine {
    let mut e = engine();
    form(&mut e, GLOB, 0x06, 0x0000_0610);
    form(&mut e, MISC, 0x1f, 0x0700_0999);
    set_ref(&mut e, 1, GLOB);
    set_ref(&mut e, 2, MISC);
    e.map(0x011d_d000, 0x1000);
    e.mem.set_u32(SAVE_LOAD_GAME, SL);
    for i in 0..255 {
        e.mem.set_u8(SL + 0x44 + i, 0xff);
    }
    e
}

#[test]
fn the_value() {
    let mut e = setup();
    e.mem.set_f32(GLOB + 0x24, 2.5);
    assert_eq!(call(&mut e, 0x2626, &[r(1)]).1, 2.5);
    call(&mut e, 0x2627, &[r(1), z(-0.1)]);
    assert_eq!(
        e.mem.f32(GLOB + 0x24),
        -0.1f32,
        "the double read as a float"
    );
    assert_eq!(call(&mut e, 0x2626, &[r(1)]).1, f64::from(-0.1f32));
    // A misc item is refused by the reader (`kParamType_Global`).
    assert_eq!(call(&mut e, 0x2626, &[r(2)]).1, 0.0);
}

#[test]
fn a_ref_set_and_read() {
    let mut e = setup();
    e.mem.set_u32(GLOB + 8, 0x8000_0041);
    call(&mut e, 0x27d6, &[r(1), r(2)]);
    assert_eq!(e.mem.u32(GLOB + 0x24), 0x0700_0999);
    assert_eq!(
        e.mem.u32(GLOB + 8),
        0x8000_0241,
        "marked, the other flags kept"
    );
    call(&mut e, 0x27d5, &[r(1)]);
    assert_eq!(form_result(&e), 0x0700_0999, "marked: read as it is");
    call(&mut e, 0x27d6, &[r(1)]);
    assert_eq!(e.mem.u32(GLOB + 0x24), 0, "no form: 0");
}

#[test]
fn a_ref_from_a_save_is_mapped_once() {
    let mut e = setup();
    // The save's file 7 is file 9 today.
    e.mem.set_u8(SL + 0x44 + 7, 9);
    e.mem.set_u32(GLOB + 0x24, 0x0700_1234);
    e.mem.set_u32(GLOB + 8, 0x40);
    call(&mut e, 0x27d5, &[r(1)]);
    assert_eq!(form_result(&e), 0x0900_1234);
    assert_eq!(e.mem.u32(GLOB + 0x24), 0x0900_1234, "rewritten in place");
    assert_eq!(e.mem.u32(GLOB + 8), 0x240, "marked, other flags kept");
    // Marked: not mapped again (file 9 of the save is not loaded, but nothing is asked).
    call(&mut e, 0x27d5, &[r(1)]);
    assert_eq!(form_result(&e), 0x0900_1234);
}

#[test]
fn a_ref_of_a_file_no_longer_loaded_becomes_0() {
    let mut e = setup();
    e.mem.set_u32(GLOB + 0x24, 0x0500_1234);
    call(&mut e, 0x27d5, &[r(1)]);
    assert_eq!(form_result(&e), 0, "0 answered");
    assert_eq!(e.mem.u32(GLOB + 0x24), 0, "and written into the global");
    assert_eq!(e.mem.u32(GLOB + 8), 0x200);
    // An id made in the game (file 0xFF) and 0 stay too.
    e.mem.set_u32(GLOB + 8, 0);
    e.mem.set_u32(GLOB + 0x24, 0xff00_0001);
    call(&mut e, 0x27d5, &[r(1)]);
    assert_eq!(form_result(&e), 0xff00_0001);
    e.mem.set_u32(GLOB + 8, 0);
    e.mem.set_u32(GLOB + 0x24, 0);
    call(&mut e, 0x27d5, &[r(1)]);
    assert_eq!(form_result(&e), 0);
}

/// The global is read as `kParamType_Global`: a misc item holding the same bytes is refused by
/// xNVSE's reader, so every command answers 0 and leaves its bytes.
#[test]
fn another_form_type_is_left_alone() {
    let mut e = setup();
    e.mem.set_u32(MISC + 8, 0x40);
    e.mem.set_u32(MISC + 0x24, 0x0700_1234);
    let before = e.mem.bytes(MISC, 0x28);
    assert_eq!(call(&mut e, 0x2626, &[r(2)]).1, 0.0);
    call(&mut e, 0x2627, &[r(2), z(5.0)]);
    call(&mut e, 0x27d5, &[r(2)]);
    assert_eq!(form_result(&e), 0);
    call(&mut e, 0x27d6, &[r(2), r(1)]);
    assert_eq!(e.mem.bytes(MISC, 0x28), before);
}
