//! Tests of `GetOptionalPatch` and `SetOptionalPatch` by their opcodes (JIP's command bodies,
//! functions_jip/jip_fn_utility.h); what each switch does is tested in `game_patches::switches`.

use crate::commands::test_support::*;
use engine_core::extend::jip_patches::{optional, FIXES, INSTALLED};
use engine_core::prelude::*;

const GET: u32 = 0x27ee;
const SET: u32 = 0x27ef;

fn world(word: u32) -> Engine {
    let e = engine();
    e.set_jip_patches(word);
    e
}

#[test]
fn get_optional_patch_answers_whether_the_switch_is_in() {
    let mut e = world(INSTALLED | FIXES | optional(1) | optional(13));
    assert_eq!(call(&mut e, GET, &[s(b"bIgnoreDTDRFix")]).1, 1.0);
    assert_eq!(call(&mut e, GET, &[s(b"bvoicemodulationfix")]).1, 1.0);
    assert_eq!(call(&mut e, GET, &[s(b"bEnableFO3Repair")]).1, 0.0);
    assert_eq!(call(&mut e, GET, &[s(b"uWMChanceMin")]).1, 0.0);
    assert_eq!(call(&mut e, GET, &[s(b"nothing")]).1, 0.0);
    // Without the row's patches: 0.
    let mut e = world(INSTALLED | optional(1));
    assert_eq!(call(&mut e, GET, &[s(b"bIgnoreDTDRFix")]).1, 0.0);
}

#[test]
fn set_optional_patch_switches_by_name() {
    let mut e = world(INSTALLED | FIXES);
    call(&mut e, SET, &[s(b"bEnableFO3Repair"), n(1)]);
    assert_ne!(e.jip_patches() & optional(2), 0);
    assert_eq!(call(&mut e, GET, &[s(b"bEnableFO3Repair")]).1, 1.0);
    call(&mut e, SET, &[s(b"BENABLEFO3REPAIR"), n(0)]);
    assert_eq!(e.jip_patches() & optional(2), 0);
    // Any number but 0 turns on.
    call(&mut e, SET, &[s(b"bQttSelectShortKeys"), n(-3)]);
    assert_ne!(e.jip_patches() & optional(9), 0);
    // An unknown name changes nothing.
    let before = e.jip_patches();
    call(&mut e, SET, &[s(b"bNothing"), n(1)]);
    assert_eq!(e.jip_patches(), before);
}
