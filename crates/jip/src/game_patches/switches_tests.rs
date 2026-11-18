//! Tests of JIP's switches at run time, every expectation from `SetOptionalPatch`
//! (internal/patches_game.h) and `Cmd_GetOptionalPatch_Execute` (functions_jip/jip_fn_utility.h).

use super::*;
use crate::test_support::engine;
use engine_core::extend::jip_patches::{optional, FIXES, INSTALLED};

fn world(switches: &[u32]) -> Engine {
    let mut e = engine();
    crate::install_as(&mut e, true);
    let mut word = INSTALLED | FIXES;
    for &n in switches {
        word |= optional(n);
    }
    e.set_jip_patches(word);
    e
}

fn bit(e: &Engine, n: u32) -> bool {
    e.jip_patches() & optional(n) != 0
}

#[test]
fn the_names_are_jips_keys_without_case() {
    assert_eq!(number_of(b"bIgnoreDTDRFix"), 1);
    assert_eq!(number_of(b"BIGNOREDTDRFIX"), 1);
    assert_eq!(number_of(b"bCreatureSpreadFix"), 19);
    assert_eq!(number_of(b"uNPCPerks"), 18);
    assert_eq!(number_of(b"uWMChanceMin"), 21);
    assert_eq!(number_of(b"bNoSuchPatch"), 0);
    assert_eq!(number_of(b""), 0);
}

#[test]
fn the_toggles_turn_on_and_off() {
    for n in [2, 5, 6, 9, 11, 13] {
        let mut e = world(&[]);
        assert!(!is_in(&mut e, n));
        assert!(set(&mut e, n, true), "{n} on");
        assert!(bit(&e, n) && is_in(&mut e, n));
        assert!(!set(&mut e, n, true), "{n} on again: no change");
        assert!(set(&mut e, n, false), "{n} off");
        assert!(!bit(&e, n) && !is_in(&mut e, n));
        assert!(!set(&mut e, n, false));
    }
}

#[test]
fn the_one_way_switches_only_turn_on() {
    for n in [4, 7, 8, 14, 19] {
        let mut e = world(&[]);
        assert!(!set(&mut e, n, false), "{n} off when off");
        assert!(set(&mut e, n, true), "{n} on");
        assert!(is_in(&mut e, n));
        assert!(!set(&mut e, n, true));
        assert!(!set(&mut e, n, false), "{n}: JIP never takes it out");
        assert!(bit(&e, n) && is_in(&mut e, n));
    }
}

#[test]
fn o1_off_takes_o12_and_the_last_threshold_with_it() {
    let mut e = world(&[1, 12]);
    assert!(is_in(&mut e, 1) && is_in(&mut e, 12));
    super::super::gc::damage::set_vats_hit_dt(&mut e, 15.0);
    assert!(set(&mut e, 1, false));
    assert!(!bit(&e, 1) && !bit(&e, 12));
    assert_eq!(super::super::gc::damage::vats_hit_dt(&mut e), -f32::MAX);
    // O12 needs O1 in, to be turned on or off.
    assert!(!set(&mut e, 12, true));
    assert!(!bit(&e, 12));
    assert!(set(&mut e, 1, true));
    assert!(!bit(&e, 12), "O1 back on leaves O12 off");
    assert!(set(&mut e, 12, true));
    assert!(is_in(&mut e, 12));
    assert!(set(&mut e, 12, false));
    assert!(!is_in(&mut e, 12));
}

#[test]
fn o12_is_in_only_with_o1() {
    let mut e = world(&[12]);
    assert!(!is_in(&mut e, 12));
    assert!(!set(&mut e, 12, false), "not with O1 out, even off");
    assert!(bit(&e, 12));
}

#[test]
fn o17_needs_jips_three_numbers() {
    let mut e = world(&[]);
    assert!(!set(&mut e, 17, true));
    assert!(!bit(&e, 17));
    super::super::gh2::set_weapon_mod_chance(&mut e, [2, 0, 60]);
    assert!(!set(&mut e, 17, true));
    super::super::gh2::set_weapon_mod_chance(&mut e, [2, 10, 60]);
    assert!(set(&mut e, 17, true));
    assert!(is_in(&mut e, 17));
    assert!(!set(&mut e, 17, false));
    assert!(is_in(&mut e, 17));
    // The bit alone (numbers since cleared) is not in.
    super::super::gh2::set_weapon_mod_chance(&mut e, [0, 10, 60]);
    assert!(!is_in(&mut e, 17));
}

#[test]
fn nvac_alerts_and_unknown_numbers_are_never_in() {
    let mut e = world(&[15]);
    assert!(!is_in(&mut e, 15), "no NVAC module");
    assert!(!set(&mut e, 15, true));
    for n in [0, 10, 20, 21, 22, 23, 99] {
        assert!(!set(&mut e, n, true), "{n}");
        assert!(!is_in(&mut e, n), "{n}");
    }
}

#[test]
fn o18_only_turns_on_and_is_in_once_set_up() {
    let mut e = world(&[18]);
    assert!(!is_in(&mut e, 18), "the bit alone: not set up");
    assert!(!set(&mut e, 18, false));
}

#[test]
fn without_the_row_nothing_is_in_and_nothing_changes() {
    let mut e = world(&[1, 2, 4]);
    e.set_jip_patches(INSTALLED | optional(1) | optional(2) | optional(4));
    for n in [1, 2, 4] {
        assert!(!is_in(&mut e, n), "{n}");
    }
    for (n, on) in [(2, false), (5, true), (7, true), (1, false)] {
        assert!(!set(&mut e, n, on), "{n}");
    }
    assert_eq!(
        e.jip_patches(),
        INSTALLED | optional(1) | optional(2) | optional(4)
    );
}

/// JIP's file with `bLocalizedDTDR=1` but O1 off: O12 refused at start (JIP's `SetOptionalPatch(12)`
/// needs O1's hook in), and O1 turned on later does not bring it.
#[test]
fn o12_asked_without_o1_stays_off_when_o1_comes_later() {
    let read = settings::switches(&[b"bIgnoreDTDRFix=0".to_vec(), b"bLocalizedDTDR=1".to_vec()]);
    assert_eq!(read.on & optional(12), 0);
    // Listed before O1: still refused (JIP's file order).
    let read = settings::switches(&[b"bLocalizedDTDR=1".to_vec(), b"bIgnoreDTDRFix=1".to_vec()]);
    assert_eq!(read.on, optional(1));
    let read = settings::switches(&[b"bIgnoreDTDRFix=1".to_vec(), b"bLocalizedDTDR=1".to_vec()]);
    assert_eq!(read.on, optional(1) | optional(12));
    let mut e = world(&[]);
    e.jip_patches_or(settings::switches(&[b"bLocalizedDTDR=1".to_vec()]).on);
    assert!(set(&mut e, 1, true));
    assert!(!bit(&e, 12));
    assert!(!is_in(&mut e, 12));
}
