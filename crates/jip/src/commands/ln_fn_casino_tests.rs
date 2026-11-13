//! Tests of the casino commands, at the raw offsets of the casino loader (`005048D0`: `DATA` at
//! +0x1E8: shuffle percent, blackjack payout +0x1EC, reel stops +0x1F0.., decks +0x20C, the
//! winnings cap +0x210, flags +0x21C).

use super::*;
use crate::commands::test_support::*;

const CASINO: u32 = FORMS + 0x1000;
const MISC: u32 = FORMS + 0x2000;

fn setup() -> Engine {
    let mut e = engine();
    form(&mut e, CASINO, 0x6d, 0x0000_ca51);
    form(&mut e, MISC, 0x1f, 0x0000_0999);
    set_ref(&mut e, 1, CASINO);
    set_ref(&mut e, 2, MISC);
    e
}

#[test]
fn the_integers() {
    let mut e = setup();
    for (get, set, off) in [(0x22b1, 0x22b2, 0x210), (0x22b7, 0x22b8, 0x20c)] {
        e.mem.set_u32(CASINO + off, 0xffff_fffe);
        assert_eq!(
            call(&mut e, get, &[r(1)]).1,
            -2.0,
            "{get:04x}: read as (int)"
        );
        call(&mut e, set, &[r(1), n(1234)]);
        assert_eq!(e.mem.u32(CASINO + off), 1234, "{set:04x}");
        call(&mut e, set, &[r(1), n(-1)]);
        assert_eq!(
            e.mem.u32(CASINO + off),
            0xffff_ffff,
            "{set:04x}: the word whole"
        );
    }
}

#[test]
fn the_floats() {
    let mut e = setup();
    for (get, set, off) in [(0x22b3, 0x22b4, 0x1ec), (0x22b9, 0x22ba, 0x1e8)] {
        e.mem.set_f32(CASINO + off, 1.5);
        assert_eq!(call(&mut e, get, &[r(1)]).1, 1.5, "{get:04x}");
        call(&mut e, set, &[r(1), z(-0.25)]);
        assert_eq!(e.mem.f32(CASINO + off), -0.25, "{set:04x}");
        call(&mut e, set, &[r(1), z(0.1)]);
        assert_eq!(
            e.mem.f32(CASINO + off),
            0.1f32,
            "{set:04x}: the nearest float"
        );
    }
}

#[test]
fn the_soft_17_flag() {
    let mut e = setup();
    e.mem.set_u32(CASINO + 0x21c, 0x8000_0003);
    assert_eq!(
        call(&mut e, 0x22b5, &[r(1)]).1,
        f64::from(0x8000_0003u32 as i32),
        "the word"
    );
    call(&mut e, 0x22b6, &[r(1), n(7)]);
    assert_eq!(e.mem.u32(CASINO + 0x21c), 1, "the whole word 1");
    call(&mut e, 0x22b6, &[r(1), n(0)]);
    assert_eq!(e.mem.u32(CASINO + 0x21c), 0);
}

#[test]
fn the_reel_stops() {
    let mut e = setup();
    for i in 0..8u32 {
        e.mem.set_u32(CASINO + 0x1f0 + 4 * i, 10 + i);
    }
    let get = |e: &mut Engine, s: i32| call(e, 0x22bb, &[r(1), n(s)]).1;
    assert_eq!(get(&mut e, 1), 10.0);
    assert_eq!(get(&mut e, 7), 16.0);
    assert_eq!(get(&mut e, 0), -1.0, "slot 0");
    assert_eq!(get(&mut e, 8), -1.0, "slot 8 (the decks word)");
    assert_eq!(
        call(&mut e, 0x22bb, &[r(2), n(1)]).1,
        -1.0,
        "the reader refuses a misc item"
    );
    let before = e.mem.bytes(CASINO, 0x228);
    call(&mut e, 0x22bc, &[r(1), n(0), n(5)]);
    call(&mut e, 0x22bc, &[r(1), n(8), n(5)]);
    assert_eq!(e.mem.bytes(CASINO, 0x228), before, "slots 0 and 8: nothing");
    call(&mut e, 0x22bc, &[r(1), n(7), n(-3)]);
    assert_eq!(e.mem.u32(CASINO + 0x1f0 + 24), 0xffff_fffd);
    assert_eq!(e.mem.u32(CASINO + 0x20c), 17, "the decks untouched");
}

/// The casino is read as `kParamType_Casino` (refused by xNVSE's reader for another form type),
/// or as any form by `SetCasinoReelStops` (refused by `IS_ID`): a misc item holding the same
/// bytes answers 0 (-1 for the reel stops) and keeps them.
#[test]
fn another_form_type_is_left_alone() {
    let mut e = setup();
    for off in (0x1e8..0x228).step_by(4) {
        e.mem.set_u32(MISC + off, 3);
    }
    let before = e.mem.bytes(MISC, 0x228);
    for get in [0x22b1, 0x22b3, 0x22b5, 0x22b7, 0x22b9] {
        assert_eq!(call(&mut e, get, &[r(2)]).1, 0.0, "{get:04x}");
    }
    for set in [0x22b2, 0x22b6, 0x22b8] {
        call(&mut e, set, &[r(2), n(9)]);
    }
    for set in [0x22b4, 0x22ba] {
        call(&mut e, set, &[r(2), z(9.0)]);
    }
    call(&mut e, 0x22bc, &[r(2), n(1), n(9)]);
    assert_eq!(e.mem.bytes(MISC, 0x228), before);
}
