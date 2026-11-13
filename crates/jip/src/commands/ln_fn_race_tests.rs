//! Tests of the race commands, at the raw offsets of the race loader (`00610CD0`: voices +0x4FC,
//! older race +0x504, younger +0x508) and the NPC loader (`00602190`: the race +0x110).

use super::*;
use crate::commands::test_support::*;

const RACE: u32 = FORMS + 0x1000;
const OLD: u32 = FORMS + 0x2000;
const VOICE: u32 = FORMS + 0x3000;
const NPC: u32 = FORMS + 0x3100;
const MISC: u32 = FORMS + 0x3200;
const PLAYER_REF: u32 = FORMS + 0x4000;

fn setup() -> Engine {
    let mut e = engine();
    form(&mut e, RACE, 0x0c, 0x0000_9ace);
    form(&mut e, OLD, 0x0c, 0x0000_01d0);
    form(&mut e, VOICE, 0x5d, 0x0000_0c0e);
    form(&mut e, NPC, 0x2a, 0x0000_0a9c);
    form(&mut e, MISC, 0x1f, 0x0000_0999);
    for (i, f) in [RACE, OLD, VOICE, NPC, MISC].iter().enumerate() {
        set_ref(&mut e, i as u16 + 1, *f);
    }
    e.map(0x011d_e000, 0x1000);
    e.mem.set_u32(0x011d_ea3c, PLAYER_REF);
    e
}

#[test]
fn the_voices() {
    let mut e = setup();
    call(&mut e, 0x2287, &[r(1), n(0), r(3)]);
    assert_eq!(
        (e.mem.u32(RACE + 0x4fc), e.mem.u32(RACE + 0x500)),
        (VOICE, 0)
    );
    call(&mut e, 0x2287, &[r(1), n(-5), r(3)]);
    assert_eq!(e.mem.u32(RACE + 0x500), VOICE, "any non-zero is female");
    call(&mut e, 0x2286, &[r(1), n(2)]);
    assert_eq!(form_result(&e), 0x0c0e, "2 is female too (`gender != 0`)");
    e.mem.set_u32(RACE + 0x500, 0);
    call(&mut e, 0x2286, &[r(1), n(1)]);
    assert_eq!(form_result(&e), 0, "none");
    call(&mut e, 0x2286, &[r(1), n(0)]);
    assert_eq!(form_result(&e), 0x0c0e);
    call(&mut e, 0x2287, &[r(1), n(1), r(5)]);
    assert_eq!(e.mem.u32(RACE + 0x500), 0, "a misc item is no voice");
}

#[test]
fn the_age_races() {
    let mut e = setup();
    call(&mut e, 0x2289, &[r(1), n(0), r(2)]);
    call(&mut e, 0x2289, &[r(1), n(9), r(1)]);
    assert_eq!(
        (e.mem.u32(RACE + 0x504), e.mem.u32(RACE + 0x508)),
        (OLD, RACE)
    );
    call(&mut e, 0x2288, &[r(1), n(0)]);
    assert_eq!(form_result(&e), 0x01d0);
    call(&mut e, 0x2288, &[r(2), n(1)]);
    assert_eq!(form_result(&e), 0, "none");
    call(&mut e, 0x2289, &[r(1), n(0), r(3)]);
    assert_eq!(e.mem.u32(RACE + 0x504), OLD, "a voice is no race");
}

#[test]
fn an_npcs_race() {
    let mut e = setup();
    let calls_to = |addr: u32| {
        calls()
            .into_iter()
            .filter(|c| c.0 == addr)
            .map(|c| c.1)
            .collect::<Vec<_>>()
    };
    e.register(0x0060_b240, |e, w| {
        CALLS.with(|c| c.borrow_mut().push((0x0060_b240, w.to_vec())));
        e.mem.set_u32(w[0] + 0x110, w[1]);
        Ret::default()
    });
    e.mem.set_u32(NPC + 0x110, OLD);
    call(&mut e, 0x22a2, &[r(4), r(1)]);
    assert_eq!(
        e.mem.u32(NPC + 0x110),
        RACE,
        "another NPC: the word written"
    );
    assert!(calls_to(0x0060_b240).is_empty());
    call(&mut e, 0x22a2, &[r(4), r(5)]);
    assert_eq!(e.mem.u32(NPC + 0x110), RACE, "a misc item is no race");
    // The player's own base form: the game's change, with 0; the same race: nothing.
    e.mem.set_u32(PLAYER_REF + 0x20, NPC);
    call(&mut e, 0x22a2, &[r(4), r(1)]);
    assert!(calls_to(0x0060_b240).is_empty(), "the same race");
    call(&mut e, 0x22a2, &[r(4), r(2)]);
    assert_eq!(calls_to(0x0060_b240), vec![vec![NPC, OLD, 0]]);
    let before = e.mem.bytes(MISC, 0x120);
    call(&mut e, 0x22a2, &[r(5), r(1)]);
    assert_eq!(e.mem.bytes(MISC, 0x120), before, "a misc item is no NPC");
}

/// Every command reads its forms as any form: a misc item holding the same words answers 0 and
/// keeps them.
#[test]
fn another_form_type_is_left_alone() {
    let mut e = setup();
    for off in [0x4fc, 0x500, 0x504, 0x508] {
        e.mem.set_u32(MISC + off, OLD);
    }
    let before = e.mem.bytes(MISC, 0x510);
    for get in [0x2286, 0x2288] {
        call(&mut e, get, &[r(5), n(0)]);
        assert_eq!(form_result(&e), 0, "{get:04x}");
    }
    call(&mut e, 0x2287, &[r(5), n(0), r(3)]);
    call(&mut e, 0x2289, &[r(5), n(0), r(1)]);
    assert_eq!(e.mem.bytes(MISC, 0x510), before);
}
