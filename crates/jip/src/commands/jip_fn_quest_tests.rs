//! Tests of the quest commands, at raw offsets: a quest's flags +0x3C, objectives and variables
//! +0x4C, current stage +0x60; an objective's vtable `01047088`, id +4, text +8, quest +0x10,
//! targets +0x14, status +0x20; a target's conditions +4, reference +0xC, path data +0x10 with
//! its teleport links at +0x24 (data) and +0x28 (size); the player's active quest +0x6B8,
//! objectives +0x6BC, targets +0x6C4. The game functions JIP calls are stand-ins that record.

use super::*;
use crate::commands::test_support::*;
use nvse::vars::{Container, Data};

const QUEST: u32 = FORMS + 0x1000;
const QUEST2: u32 = FORMS + 0x1100;
const REF1: u32 = FORMS + 0x1200;
const REF2: u32 = FORMS + 0x1300;
const REF3: u32 = FORMS + 0x1400;
const DOOR1: u32 = FORMS + 0x1500;
const DOOR2: u32 = FORMS + 0x1600;
const MISC: u32 = FORMS + 0x1700;
const O1: u32 = FORMS + 0x2000;
const O2: u32 = FORMS + 0x2100;
const O3: u32 = FORMS + 0x2200;
const O4: u32 = FORMS + 0x2300;
const VAR: u32 = FORMS + 0x2400;
const T1: u32 = FORMS + 0x3000;
const T2: u32 = FORMS + 0x3100;
const T3: u32 = FORMS + 0x3200;
const LINKS: u32 = FORMS + 0x3800;
const NODES: u32 = FORMS + 0x4000;
const PC: u32 = FORMS + 0x8000;
const PLAYER_PTR: u32 = 0x011d_ea3c;
const SET_STAGE_FN: u32 = 0x0060_d510;
const EVALUATE_FN: u32 = 0x0068_0c60;
const REFRESH_PATH_FN: u32 = 0x0095_2d60;
const CONSTRUCT_FN: u32 = 0x0060_ff70;
const DESTROY_FN: u32 = 0x005e_c4d0;
const REFRESH_TARGETS_FN: u32 = 0x0060_f110;
const FAIL_FN: u32 = 0x0060_caf0;

fn record(addr: u32, w: &[u32]) {
    CALLS.with(|c| c.borrow_mut().push((addr, w.to_vec())));
}

fn eax(v: u32) -> Ret {
    Ret {
        eax: v,
        ..Ret::default()
    }
}

fn calls_to(addr: u32) -> Vec<Vec<u32>> {
    calls()
        .into_iter()
        .filter(|c| c.0 == addr)
        .map(|c| c.1)
        .collect()
}

/// `node(i)`: list node `i` at NODES (8 bytes each).
fn node(i: u32) -> u32 {
    NODES + 8 * i
}

/// QUEST: a variable, then objectives O1 (id 10, displayed), O2 (20, completed), O3 (30, not
/// displayed). QUEST2 (completed): O4 (5, displayed). O1's targets: T1 (REF1, two doors), T2
/// (REF2, whose conditions fail), T3 (no reference). The player: QUEST active, objectives O1, O2,
/// O4. The stand-ins: SetStage refuses stage 99; conditions fail for REF2.
fn setup() -> Engine {
    let mut e = engine();
    e.map(0x011d_e000, 0x1000);
    form(&mut e, QUEST, 0x47, 0x0000_0a11);
    form(&mut e, QUEST2, 0x47, 0x0000_0a22);
    for (i, r) in [REF1, REF2, REF3, DOOR1, DOOR2].iter().enumerate() {
        form(&mut e, *r, 0x3a, 0x0000_0e01 + i as u32);
    }
    form(&mut e, MISC, 0x1f, 0x0000_0999);
    for (i, f) in [QUEST, QUEST2, REF1, REF2, REF3, MISC].iter().enumerate() {
        set_ref(&mut e, i as u16 + 1, *f);
    }
    e.mem.set_u8(QUEST2 + 0x3c, 2);
    for (o, id, status, quest) in [
        (O1, 10, 1, QUEST),
        (O2, 20, 3, QUEST),
        (O3, 30, 0, QUEST),
        (O4, 5, 1, QUEST2),
    ] {
        e.mem.set_u32(o, 0x0104_7088);
        e.mem.set_u32(o + 4, id);
        e.mem.set_u32(o + 0x10, quest);
        e.mem.set_u32(o + 0x20, status);
    }
    // QUEST's list: VAR, O1, O2, O3 (VAR's vtable is no objective's; it holds id 10 too).
    e.mem.set_u32(VAR + 4, 10);
    e.mem.set_u32(QUEST + 0x4c, VAR);
    e.mem.set_u32(QUEST + 0x50, node(0));
    e.mem.set_u32(node(0), O1);
    e.mem.set_u32(node(0) + 4, node(1));
    e.mem.set_u32(node(1), O2);
    e.mem.set_u32(node(1) + 4, node(2));
    e.mem.set_u32(node(2), O3);
    e.mem.set_u32(QUEST2 + 0x4c, O4);
    // O1's targets: T1, T2, T3.
    e.mem.set_u32(O1 + 0x14, T1);
    e.mem.set_u32(O1 + 0x18, node(3));
    e.mem.set_u32(node(3), T2);
    e.mem.set_u32(node(3) + 4, node(4));
    e.mem.set_u32(node(4), T3);
    e.mem.set_u32(T1 + 0xc, REF1);
    e.mem.set_u32(T2 + 0xc, REF2);
    e.mem.set_u32(T1 + 0x24, LINKS);
    e.mem.set_u32(T1 + 0x28, 2);
    e.mem.set_u32(LINKS, DOOR1);
    e.mem.set_u32(LINKS + 0x10, DOOR2);
    e.mem.set_u32(O4 + 0x14, T3);
    // The player.
    e.mem.set_u32(PLAYER_PTR, PC);
    e.mem.set_u32(PC + 0x6b8, QUEST);
    e.mem.set_u32(PC + 0x6bc, O1);
    e.mem.set_u32(PC + 0x6c0, node(5));
    e.mem.set_u32(node(5), O2);
    e.mem.set_u32(node(5) + 4, node(6));
    e.mem.set_u32(node(6), O4);
    e.mem.set_u32(SCRIPT + 0xc, 0x0600_0001);
    e.register(SET_STAGE_FN, |_, w| {
        record(SET_STAGE_FN, w);
        eax(u32::from(w[1] != 99))
    });
    e.register(EVALUATE_FN, |e, w| {
        record(EVALUATE_FN, w);
        e.mem.set_u8(w[3], 1);
        eax(if w[1] == REF2 {
            0xffff_ff00
        } else {
            0xffff_ff01
        })
    });
    e.register(REFRESH_PATH_FN, |_, w| {
        record(REFRESH_PATH_FN, w);
        eax(0)
    });
    e.register(CONSTRUCT_FN, |e, w| {
        record(CONSTRUCT_FN, w);
        // As the exe's constructor: the byte, the conditions and the reference cleared.
        e.mem.set_u8(w[0], 0);
        for off in [4, 8, 0xc] {
            e.mem.set_u32(w[0] + off, 0);
        }
        eax(w[0])
    });
    e.register(DESTROY_FN, |_, w| {
        record(DESTROY_FN, w);
        eax(0)
    });
    e.register(REFRESH_TARGETS_FN, |_, w| {
        record(REFRESH_TARGETS_FN, w);
        eax(0)
    });
    e.register(FAIL_FN, |_, w| {
        record(FAIL_FN, w);
        eax(0)
    });
    CALLS.with(|c| c.borrow_mut().clear());
    e
}

/// The elements of a result's array as plain values (`None` for no array).
fn elements(e: &Engine, result: f64) -> Option<Vec<Data>> {
    let a = nvse::vars::array(e, result as u32)?;
    assert_eq!(
        (a.packed, a.owning_mod),
        (true, 6),
        "packed, the script's mod"
    );
    let Container::Array(v) = &a.elements else {
        panic!("a packed array")
    };
    Some(v.iter().map(|el| el.data.clone()).collect())
}

fn forms(ids: &[u32]) -> Option<Vec<Data>> {
    Some(ids.iter().map(|&i| Data::Form(i)).collect())
}

fn numbers(v: &[f64]) -> Option<Vec<Data>> {
    Some(v.iter().map(|&n| Data::Number(n)).collect())
}

#[test]
fn the_stage() {
    let mut e = setup();
    assert_eq!(call(&mut e, 0x26da, &[r(1), n(0x105)]).1, 1.0);
    assert_eq!(calls_to(SET_STAGE_FN), vec![vec![QUEST, 5]], "the low byte");
    let mut want = e.mem.bytes(QUEST, 0x70);
    assert_eq!(want[0x60], 5, "the current stage");
    want[0x60] = 0;
    e.mem.set_u8(QUEST + 0x60, 0);
    assert_eq!(call(&mut e, 0x26da, &[r(1), n(99)]).1, 0.0, "refused");
    assert_eq!(e.mem.bytes(QUEST, 0x70), want, "refused: nothing");
}

#[test]
fn the_flags() {
    let mut e = setup();
    assert_eq!(call(&mut e, 0x2889, &[r(2), n(1)]).1, 1.0);
    assert_eq!(call(&mut e, 0x2889, &[r(2), n(0)]).1, 0.0);
    let mut want = e.mem.bytes(QUEST, 0x70);
    want[0x3c] = 0x80;
    call(&mut e, 0x2888, &[r(1), n(7), n(1)]);
    assert_eq!(e.mem.bytes(QUEST, 0x70), want, "flag 7 set");
    assert_eq!(call(&mut e, 0x2889, &[r(1), n(7)]).1, 1.0);
    call(&mut e, 0x2888, &[r(1), n(8), n(1)]);
    assert_eq!(e.mem.bytes(QUEST, 0x70), want, "flag 8: nothing");
    assert_eq!(call(&mut e, 0x2889, &[r(1), n(8)]).1, 0.0);
    e.mem.set_u8(QUEST + 0x3c, 0xff);
    want[0x3c] = 0xfb;
    call(&mut e, 0x2888, &[r(1), n(2), n(0)]);
    assert_eq!(e.mem.bytes(QUEST, 0x70), want, "flag 2 cleared alone");
}

#[test]
fn failing_and_clearing() {
    let mut e = setup();
    call(&mut e, 0x2903, &[r(2)]);
    assert_eq!(calls_to(FAIL_FN), vec![vec![QUEST2, 1]]);
    // The player's targets: REF1's target first, two more nodes.
    e.mem.set_u32(PC + 0x6c4, T1);
    e.mem.set_u32(PC + 0x6c8, node(7));
    e.mem.set_u32(node(7), T2);
    e.mem.set_u32(node(7) + 4, node(8));
    e.mem.set_u32(node(8), T3);
    call(&mut e, 0x2746, &[]);
    assert_eq!(e.mem.u32(PC + 0x6b8), 0, "no active quest");
    assert_eq!((e.mem.u32(PC + 0x6c4), e.mem.u32(PC + 0x6c8)), (0, 0));
    assert_eq!(
        freed(),
        vec![node(7), node(8)],
        "the later nodes, not the targets"
    );
    assert_eq!(e.mem.u32(PC + 0x6bc), O1, "the objectives kept");
}

#[test]
fn the_player_quests() {
    let mut e = setup();
    // Displayed objectives O1 (QUEST), O2 (QUEST again), O4 (QUEST2, completed).
    let (_, res) = call(&mut e, 0x286d, &[]);
    assert_eq!(elements(&e, res), forms(&[0x0a11]), "not completed, once");
    let (_, res) = call(&mut e, 0x286d, &[n(1)]);
    assert_eq!(elements(&e, res), forms(&[0x0a22]), "completed");
    e.mem.set_u32(O4 + 0x20, 2);
    let (_, res) = call(&mut e, 0x286d, &[n(1)]);
    assert_eq!(res, 0.0, "not displayed: none, no array");
    // Two quests come out by address.
    e.mem.set_u8(QUEST2 + 0x3c, 0);
    e.mem.set_u32(O4 + 0x20, 1);
    e.mem.set_u32(PC + 0x6bc, O4);
    e.mem.set_u32(node(6), O1);
    let (_, res) = call(&mut e, 0x286d, &[]);
    assert_eq!(elements(&e, res), forms(&[0x0a11, 0x0a22]));
}

#[test]
fn the_objectives() {
    let mut e = setup();
    let (_, res) = call(&mut e, 0x286e, &[r(1), n(0)]);
    assert_eq!(elements(&e, res), numbers(&[10.0]), "displayed");
    let (_, res) = call(&mut e, 0x286e, &[r(1), n(1)]);
    assert_eq!(elements(&e, res), numbers(&[20.0]), "completed");
    e.mem.set_u32(O2 + 4, 0xffff_fffe);
    e.mem.set_u32(O3 + 0x20, 3);
    let (_, res) = call(&mut e, 0x286e, &[r(1), n(5)]);
    assert_eq!(elements(&e, res), numbers(&[-2.0, 30.0]), "(int), in order");
    let (_, res) = call(&mut e, 0x286f, &[]);
    assert_eq!(elements(&e, res), numbers(&[10.0]), "the active quest's");
    e.mem.set_u32(O1 + 0x20, 0x101);
    let (_, res) = call(&mut e, 0x286f, &[]);
    assert_eq!(elements(&e, res), numbers(&[10.0]), "the low two bits");
    e.mem.set_u32(O1 + 0x20, 0);
    assert_eq!(call(&mut e, 0x286f, &[]).1, 0.0, "none: no array");
    e.mem.set_u32(PC + 0x6b8, 0);
    assert_eq!(call(&mut e, 0x286f, &[]).1, 0.0, "no active quest");
}

#[test]
fn the_targets() {
    let mut e = setup();
    let (_, res) = call(&mut e, 0x2818, &[r(1), n(10)]);
    assert_eq!(
        elements(&e, res),
        forms(&[0x0e01]),
        "REF1; REF2's conditions fail"
    );
    let evaluated = calls_to(EVALUATE_FN);
    assert_eq!(evaluated.len(), 2, "T3 has no reference: not evaluated");
    assert_eq!(
        evaluated[0][..3],
        [T1 + 4, REF1, 0],
        "the conditions, the reference, no second form"
    );
    assert_eq!(evaluated[0][4], 0, "the last argument");
    assert_eq!(call(&mut e, 0x2818, &[r(1), n(20)]).1, 0.0, "completed");
    assert_eq!(
        call(&mut e, 0x2818, &[r(1), n(11)]).1,
        0.0,
        "no such objective"
    );
    e.mem.set_u32(T1 + 0xc, REF2);
    assert_eq!(call(&mut e, 0x2818, &[r(1), n(10)]).1, 0.0, "none live");
    // GetObjectiveHasTarget: any target, live or not.
    assert_eq!(call(&mut e, 0x281a, &[r(1), n(10), r(4)]).1, 1.0);
    assert_eq!(call(&mut e, 0x281a, &[r(1), n(10), r(5)]).1, 0.0);
    assert_eq!(call(&mut e, 0x281a, &[r(1), n(30), r(4)]).1, 0.0);
}

#[test]
fn the_teleport_links() {
    let mut e = setup();
    let (_, res) = call(&mut e, 0x2870, &[r(1), n(10)]);
    let outer = elements(&e, res).expect("the array");
    assert_eq!(outer.len(), 1, "one live target");
    let Data::Array(inner) = outer[0] else {
        panic!("an array of arrays")
    };
    assert_eq!(
        elements(&e, f64::from(inner)),
        forms(&[0x0e04, 0x0e05, 0x0e01]),
        "the doors, then the target"
    );
    assert!(
        calls_to(REFRESH_PATH_FN).is_empty(),
        "the active quest: no refresh"
    );
    // Another quest's objective: the path worked out first, for each live target.
    e.mem.set_u32(PC + 0x6b8, QUEST2);
    call(&mut e, 0x2870, &[r(1), n(10)]);
    assert_eq!(calls_to(REFRESH_PATH_FN), vec![vec![REF1, T1 + 0x10, 1]]);
    // No live target: an empty array all the same.
    e.mem.set_u32(T1 + 0xc, 0);
    let (_, res) = call(&mut e, 0x2870, &[r(1), n(10)]);
    assert_eq!(elements(&e, res), Some(vec![]));
    assert_eq!(
        call(&mut e, 0x2870, &[r(1), n(20)]).1,
        0.0,
        "completed: none"
    );
}

#[test]
fn setting_the_nth_target() {
    let mut e = setup();
    let snap = |e: &Engine| e.mem.bytes(T1, 0x300);
    let mut want = snap(&e);
    want[0x100 + 0xc..0x100 + 0x10].copy_from_slice(&REF3.to_le_bytes());
    call(&mut e, 0x2819, &[r(1), n(10), n(1), r(5)]);
    assert_eq!(snap(&e), want, "T2's reference alone");
    assert_eq!(
        calls_to(REFRESH_PATH_FN),
        vec![vec![REF3, T2 + 0x10, 1]],
        "active"
    );
    CALLS.with(|c| c.borrow_mut().clear());
    call(&mut e, 0x2819, &[r(1), n(10), n(3), r(5)]);
    call(&mut e, 0x2819, &[r(1), n(10), n(-1), r(5)]);
    assert_eq!(snap(&e), want, "past the end, negative: nothing");
    e.mem.set_u32(PC + 0x6b8, QUEST2);
    want[0xc..0x10].copy_from_slice(&REF2.to_le_bytes());
    call(&mut e, 0x2819, &[r(1), n(10), n(0), r(4)]);
    assert_eq!(snap(&e), want);
    assert!(calls_to(REFRESH_PATH_FN).is_empty(), "not the active quest");
}

#[test]
fn adding_a_target() {
    let mut e = setup();
    call(&mut e, 0x281b, &[r(1), n(10), r(3)]);
    assert!(allocations().is_empty(), "REF1 is a target already");
    let block = heap_next(&e);
    call(&mut e, 0x281b, &[r(1), n(10), r(5)]);
    assert_eq!(
        allocations(),
        vec![0x48, 8],
        "the target, then the moved first item's node"
    );
    assert_eq!(calls_to(CONSTRUCT_FN), vec![vec![block]]);
    assert_eq!(e.mem.u32(block + 0xc), REF3);
    assert_eq!(e.mem.u32(O1 + 0x14), block, "first");
    let moved = e.mem.u32(O1 + 0x18);
    assert_eq!((e.mem.u32(moved), e.mem.u32(moved + 4)), (T1, node(3)));
    assert_eq!(
        calls_to(REFRESH_TARGETS_FN),
        vec![vec![QUEST, PC + 0x6c4, PC + 0x6bc]],
        "the active quest's targets worked out again"
    );
    // Another quest: no refresh.
    CALLS.with(|c| c.borrow_mut().clear());
    call(&mut e, 0x281b, &[r(2), n(5), r(4)]);
    assert_eq!(allocations(), vec![0x48, 8]);
    assert!(calls_to(REFRESH_TARGETS_FN).is_empty());
}

#[test]
fn removing_a_target() {
    let mut e = setup();
    call(&mut e, 0x281c, &[r(1), n(10), r(4)]);
    assert_eq!(freed(), vec![node(3)], "T2's node");
    assert_eq!(
        calls_to(DESTROY_FN),
        vec![vec![T2, 1]],
        "T2 destroyed and freed"
    );
    assert_eq!(e.mem.u32(O1 + 0x18), node(4));
    assert_eq!(
        calls_to(REFRESH_TARGETS_FN),
        vec![vec![QUEST, PC + 0x6c4, PC + 0x6bc]]
    );
    // The first: the next node's item moves up, that node freed.
    CALLS.with(|c| c.borrow_mut().clear());
    call(&mut e, 0x281c, &[r(1), n(10), r(3)]);
    assert_eq!(freed(), vec![node(4)]);
    assert_eq!(calls_to(DESTROY_FN), vec![vec![T1, 1]]);
    assert_eq!((e.mem.u32(O1 + 0x14), e.mem.u32(O1 + 0x18)), (T3, 0));
    CALLS.with(|c| c.borrow_mut().clear());
    call(&mut e, 0x281c, &[r(1), n(10), r(5)]);
    assert!(calls().is_empty(), "not a target: nothing");
}

#[test]
fn the_text() {
    let mut e = setup();
    let (_, res) = call(&mut e, 0x281d, &[r(1), n(10)]);
    assert_eq!(string_result(&e, res), b"", "no buffer");
    let first = heap_next(&e);
    call(&mut e, 0x281e, &[r(1), n(10), s(b"Find Doc")]);
    assert_eq!(text_at(&e, O1 + 8), b"Find Doc");
    assert_eq!(allocations(), vec![9]);
    assert!(freed().is_empty());
    assert_eq!(e.mem.u32(O1 + 8), first);
    call(&mut e, 0x281e, &[r(1), n(10), s(b"Go")]);
    assert_eq!(allocations(), vec![9], "shorter: the same buffer");
    let second = heap_next(&e);
    call(&mut e, 0x281e, &[r(1), n(10), s(b"Talk to Doc Mitchell")]);
    assert_eq!(allocations(), vec![9, 21], "longer: a new buffer");
    assert_eq!(freed(), vec![first], "the old buffer freed");
    assert_eq!(e.mem.u32(O1 + 8), second);
    let (_, res) = call(&mut e, 0x281d, &[r(1), n(10)]);
    assert_eq!(string_result(&e, res), b"Talk to Doc Mitchell");
    let before = e.mem.bytes(VAR, 0x24);
    call(&mut e, 0x281e, &[r(1), n(99), s(b"x")]);
    assert_eq!(e.mem.bytes(VAR, 0x24), before, "no such objective");
}

#[test]
fn targets_changed() {
    let mut e = setup();
    assert_eq!(call(&mut e, 0x272f, &[]).1, 1.0, "the first call");
    assert_eq!(e.ext::<crate::Jip>().quest.last_targets, vec![REF1]);
    assert_eq!(call(&mut e, 0x272f, &[]).1, 0.0, "the same");
    // T2's reference now passes: a new set.
    e.mem.set_u32(T2 + 0xc, REF3);
    assert_eq!(call(&mut e, 0x272f, &[]).1, 1.0);
    assert_eq!(e.ext::<crate::Jip>().quest.last_targets, vec![REF1, REF3]);
    // A completed quest's objectives do not count.
    e.mem.set_u8(QUEST + 0x3c, 2);
    assert_eq!(call(&mut e, 0x272f, &[]).1, 1.0);
    assert!(e.ext::<crate::Jip>().quest.last_targets.is_empty());
}

/// Every quest argument is read by type: a misc item stops the read and nothing changes.
#[test]
fn another_form_type_is_left_alone() {
    let mut e = setup();
    for off in 0x10..0x70 {
        e.mem.set_u8(MISC + off, 0xa5);
    }
    let before = (e.mem.bytes(MISC, 0x70), e.mem.bytes(FORMS + 0x2000, 0x2000));
    assert_eq!(call(&mut e, 0x26da, &[r(6), n(1)]).1, 0.0);
    assert_eq!(call(&mut e, 0x2889, &[r(6), n(0)]).1, 0.0);
    call(&mut e, 0x2888, &[r(6), n(0), n(0)]);
    call(&mut e, 0x2903, &[r(6)]);
    assert_eq!(call(&mut e, 0x2818, &[r(6), n(10)]).1, 0.0);
    call(&mut e, 0x2819, &[r(6), n(10), n(0), r(5)]);
    assert_eq!(call(&mut e, 0x281a, &[r(6), n(10), r(3)]).1, 0.0);
    call(&mut e, 0x281b, &[r(6), n(10), r(5)]);
    call(&mut e, 0x281c, &[r(6), n(10), r(3)]);
    let (_, res) = call(&mut e, 0x281d, &[r(6), n(10)]);
    assert_eq!(string_result(&e, res), b"");
    call(&mut e, 0x281e, &[r(6), n(10), s(b"x")]);
    assert_eq!(call(&mut e, 0x286e, &[r(6), n(0)]).1, 0.0);
    assert_eq!(call(&mut e, 0x2870, &[r(6), n(10)]).1, 0.0);
    // A misc item where a reference goes: refused too.
    call(&mut e, 0x2819, &[r(1), n(10), n(0), r(6)]);
    call(&mut e, 0x281b, &[r(1), n(10), r(6)]);
    assert_eq!(
        (e.mem.bytes(MISC, 0x70), e.mem.bytes(FORMS + 0x2000, 0x2000)),
        before
    );
    assert!(calls()
        .iter()
        .all(|c| c.0 != SET_STAGE_FN && c.0 != FAIL_FN));
    assert!(allocations().is_empty() && freed().is_empty());
}
