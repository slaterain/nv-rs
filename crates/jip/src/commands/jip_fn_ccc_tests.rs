//! Tests of `jip_fn_ccc` on actors (vtable +0x100 answering true), actor bases (factions +0x5C,
//! script +0xF8), extra data lists (the stand-ins of `test_support`), cells and worldspaces at
//! JIP's header offsets, map markers (base id 0x10, `ExtraMapMarker` 0x2C), and the game
//! functions the commands call as recording stand-ins.

use super::*;
use crate::commands::test_support::*;
use engine_core::extend::Hook;

const ACTOR: u32 = FORMS + 0x100;
const ACTOR2: u32 = FORMS + 0x200;
const PLAIN: u32 = FORMS + 0x300;
const NPC: u32 = FORMS + 0x400;
const NPC2: u32 = FORMS + 0x600;
const FACTION: u32 = FORMS + 0x800;
const FACTION2: u32 = FORMS + 0x880;
const REP: u32 = FORMS + 0x900;
const REP2: u32 = FORMS + 0x980;
const SCRIPT: u32 = FORMS + 0xa00;
const SCRIPT2: u32 = FORMS + 0xa80;
const PACKAGE: u32 = FORMS + 0xb00;
const TOPIC: u32 = FORMS + 0xb80;
const INFO: u32 = FORMS + 0xc00;
const CELL: u32 = FORMS + 0x1000;
const CELL2: u32 = FORMS + 0x1100;
const PCELL: u32 = FORMS + 0x1200;
const WORLD: u32 = FORMS + 0x1300;
const MARKER_STATIC: u32 = FORMS + 0x1400;
const M1: u32 = FORMS + 0x1500;
const M2: u32 = FORMS + 0x1600;
const M3: u32 = FORMS + 0x1700;
const MD1: u32 = FORMS + 0x1800;
const MD2: u32 = FORMS + 0x1840;
const MD3: u32 = FORMS + 0x1880;
const X1: u32 = FORMS + 0x2000;
const X2: u32 = FORMS + 0x2040;
const X3: u32 = FORMS + 0x2080;
const X4: u32 = FORMS + 0x20c0;
const X5: u32 = FORMS + 0x2100;
const NODES: u32 = FORMS + 0x3000;
const DATA: u32 = FORMS + 0x3400;
const TEXT: u32 = FORMS + 0x3800;
const CHANGES: u32 = FORMS + 0x3c00;
const SDATA: u32 = FORMS + 0x4000;
const VTBL: u32 = FORMS + 0x6000;
const PLAYER_AT: u32 = FORMS + 0x7000;

fn reply(st0: f64) -> Ret {
    Ret {
        st0,
        ..Ret::default()
    }
}

fn setup() -> Engine {
    let mut e = engine();
    extra_stand_ins(&mut e);
    e.mem.set_u32(VTBL + 0x100, 0x008d_0360);
    e.mem.set_u32(VTBL + 0xf0, 0x008d_0360);
    for (f, t, id) in [
        (ACTOR, 0x3b, 0x0100_0100),
        (ACTOR2, 0x3b, 0x0100_0200),
        (PLAIN, 0x3a, 0x0100_0300),
        (NPC, 0x2a, 0x0100_0400),
        (NPC2, 0x2a, 0x0100_0600),
        (FACTION, 0x08, 0x0100_0800),
        (FACTION2, 0x08, 0x0100_0880),
        (REP, 0x68, 0x0100_0900),
        (REP2, 0x68, 0x0100_0980),
        (SCRIPT, 0x11, 0x0100_0a00),
        (SCRIPT2, 0x11, 0x0100_0a80),
        (PACKAGE, 0x49, 0x0100_0b00),
        (TOPIC, 0x45, 0x0100_0b80),
        (INFO, 0x46, 0x0100_0c00),
        (CELL, 0x39, 0x0100_1000),
        (CELL2, 0x39, 0x0100_1100),
        (PCELL, 0x39, 0x0100_1200),
        (WORLD, 0x41, 0x0100_1300),
        (MARKER_STATIC, 0x20, 0x10),
        (M1, 0x3a, 0x0100_1500),
        (M2, 0x3a, 0x0100_1600),
        (M3, 0x3a, 0x0100_1700),
    ] {
        form(&mut e, f, t, id);
    }
    e.mem.set_u32(ACTOR, VTBL);
    e.mem.set_u32(ACTOR2, VTBL);
    e.mem.set_u32(PLAIN, VTBL + 0x400);
    e.map(0x011e_0000, 0x1000);
    e.mem.set_u32(ACTOR + 0x20, NPC);
    e.mem.set_u32(ACTOR2 + 0x20, NPC2);
    e.mem.set_u32(PLAIN + 0x20, NPC);
    e.mem.set_u32(0x011d_ea3c, PLAYER_AT);
    for (i, f) in [ACTOR, FACTION, FACTION2, PACKAGE, TOPIC]
        .iter()
        .enumerate()
    {
        set_ref(&mut e, i as u16 + 1, *f);
    }
    e.register(MAX_CARRY_WEIGHT, |e, w| {
        CALLS.with(|c| c.borrow_mut().push((MAX_CARRY_WEIGHT, w.to_vec())));
        reply(f64::from(e.mem.f32(w[0] + 0x64)))
    });
    e.register(0x004d_0900, |e, w| {
        CALLS.with(|c| c.borrow_mut().push((0x004d_0900, w.to_vec())));
        reply(f64::from(e.mem.f32(w[0] + 0x20)))
    });
    e.register(SAY_TO, |e, w| {
        CALLS.with(|c| c.borrow_mut().push((SAY_TO, w.to_vec())));
        let actor = w[2];
        let info = e.mem.u32(PLAYER_AT + 0x100);
        if info != u32::MAX {
            put_extra(e, actor + 0x44, X5, 0x75, info);
        }
        Ret::default()
    });
    e.register(RUN_RESULT_SCRIPT, |_, w| {
        CALLS.with(|c| c.borrow_mut().push((RUN_RESULT_SCRIPT, w.to_vec())));
        Ret::default()
    });
    e.register(GET_TOPIC_INFO, |e, w| {
        let mut words = w.to_vec();
        words.push(e.mem.u32(w[1]));
        CALLS.with(|c| c.borrow_mut().push((GET_TOPIC_INFO, words)));
        Ret {
            eax: e.mem.u32(PLAYER_AT + 0x104),
            ..Ret::default()
        }
    });
    e
}

fn run(e: &mut Engine, opcode: u32, this: u32, args: &[Vec<u8>]) -> f64 {
    e.mem.set_f64(RESULT, 0.0);
    call_on(e, opcode, this, args).1
}

fn eval(e: &mut Engine, opcode: u32, this: u32) -> f64 {
    let record = nvse::table::record_of(e, opcode);
    let at = e.mem.u32(record + 0x20);
    e.mem.set_f64(RESULT, -5.0);
    e.call(at, &[this, 0, 0, RESULT]);
    e.mem.f64(RESULT)
}

fn place(e: &mut Engine, refr: u32, x: f32, y: f32, z: f32) {
    e.mem.set_f32(refr + 0x30, x);
    e.mem.set_f32(refr + 0x34, y);
    e.mem.set_f32(refr + 0x38, z);
}

fn node(e: &mut Engine, at: u32, data: u32, next: u32) {
    e.mem.set_u32(at, data);
    e.mem.set_u32(at + 4, next);
}

/// A game string at `at` (text pointer, length) holding `text` at `buffer`.
fn text(e: &mut Engine, at: u32, buffer: u32, text: &[u8]) {
    e.mem.write(buffer, text);
    e.mem.set_u8(buffer + text.len() as u32, 0);
    e.mem.set_u32(at, buffer);
    e.mem.set_u16(at + 4, text.len() as u16);
    e.mem.set_u16(at + 6, text.len() as u16);
}

#[test]
fn distance() {
    let mut e = setup();
    place(&mut e, ACTOR, 1.5, -2.0, 100.25);
    place(&mut e, ACTOR2, -1.5, 2.0, -0.5);
    set_ref(&mut e, 6, ACTOR2);
    let want = super::super::ln_fn_math::point2_distance([1.5, -2.0, 100.25], [-1.5, 2.0, -0.5]);
    assert_eq!(run(&mut e, 0x2701, ACTOR, &[r(6)]), f64::from(want));
    assert_eq!(run(&mut e, 0x2701, ACTOR, &[r(6), n(3)]), f64::from(want));
    assert_eq!(
        run(&mut e, 0x2701, ACTOR, &[r(6), n(0)]),
        f64::from(want),
        "any axis but 4"
    );
    assert_eq!(run(&mut e, 0x2701, ACTOR, &[r(6), n(4)]), 100.75);
    assert_eq!(
        run(&mut e, 0x2701, ACTOR2, &[r(1), n(4)]),
        100.75,
        "absolute"
    );
    assert_eq!(run(&mut e, 0x2701, ACTOR, &[r(6), n(5)]), f64::from(want));
}

/// Faction membership: the base's list (raw base form) only with the second argument, the
/// reference's own changes always; the first entry of the faction decides; empty entries
/// skipped; a non-actor nothing.
#[test]
fn in_faction() {
    let mut e = setup();
    let ask = |e: &mut Engine, this: u32, a: &[Vec<u8>]| run(e, 0x2704, this, a);
    assert_eq!(ask(&mut e, ACTOR, &[r(2), n(1)]), 0.0);
    node(&mut e, NPC + 0x5c, 0, NODES);
    node(&mut e, NODES, DATA, 0);
    e.mem.set_u32(DATA, FACTION);
    e.mem.set_u8(DATA + 4, 0);
    assert_eq!(ask(&mut e, ACTOR, &[r(2), n(1)]), 1.0, "rank 0 counts");
    assert_eq!(ask(&mut e, ACTOR, &[r(2)]), 0.0, "the base only when asked");
    assert_eq!(ask(&mut e, ACTOR, &[r(3), n(1)]), 0.0, "another faction");
    e.mem.set_u8(DATA + 4, 0xff);
    assert_eq!(ask(&mut e, ACTOR, &[r(2), n(1)]), 0.0, "rank -1");
    assert_eq!(ask(&mut e, PLAIN, &[r(2), n(1)]), 0.0, "not an actor");
    // The reference's own list: an empty entry, then FACTION2 at -1, then FACTION2 at 3.
    put_extra(&mut e, ACTOR + 0x44, X1, 0x5e, NODES + 0x10);
    node(&mut e, NODES + 0x10, 0, NODES + 0x18);
    node(&mut e, NODES + 0x18, DATA + 8, NODES + 0x20);
    node(&mut e, NODES + 0x20, DATA + 0x10, 0);
    e.mem.set_u32(DATA + 8, FACTION2);
    e.mem.set_u8(DATA + 0xc, 0xff);
    e.mem.set_u32(DATA + 0x10, FACTION2);
    e.mem.set_u8(DATA + 0x14, 3);
    assert_eq!(
        ask(&mut e, ACTOR, &[r(3)]),
        0.0,
        "the first entry's -1 decides"
    );
    e.mem.set_u8(DATA + 0xc, 2);
    assert_eq!(ask(&mut e, ACTOR, &[r(3)]), 1.0);
    assert_eq!(ask(&mut e, ACTOR, &[r(3), n(1)]), 1.0, "or");
    e.mem.set_u32(X1 + 0xc, 0);
    assert_eq!(ask(&mut e, ACTOR, &[r(3)]), 0.0, "no list");
}

/// A faction list from a bad save that runs back into itself (the reference's own changes, and
/// its base's list), the faction not in it: with the list guard row the walks end, not in.
#[test]
fn in_faction_with_a_list_that_loops_back() {
    let mut e = setup();
    nvse::commands::array::set_fix(&mut e, nvse::commands::array::LIST_GUARD, true);
    let ask = |e: &mut Engine, a: &[Vec<u8>]| run(e, 0x2704, ACTOR, a);
    e.mem.set_u32(DATA, FACTION2);
    e.mem.set_u8(DATA + 4, 1);
    put_extra(&mut e, ACTOR + 0x44, X1, 0x5e, NODES);
    node(&mut e, NODES, 0, NODES + 8);
    node(&mut e, NODES + 8, DATA, NODES);
    assert_eq!(ask(&mut e, &[r(2)]), 0.0, "own changes loop");
    node(&mut e, NPC + 0x5c, DATA, NODES + 0x10);
    node(&mut e, NODES + 0x10, 0, NPC + 0x5c);
    assert_eq!(ask(&mut e, &[r(2), n(1)]), 0.0, "the base's list loops too");
    assert_eq!(ask(&mut e, &[r(3)]), 1.0, "a faction in it is still found");
}

fn script_with_data(e: &mut Engine, script: u32, data: u32) {
    // ScriptName, one block (Begin 6: type 0, length 4), End: 4 + 10 + 4 bytes.
    let d: [u8; 18] = [
        0x1d, 0, 0, 0, 0x10, 0, 6, 0, 0, 0, 4, 0, 0, 0, 0x11, 0, 0, 0,
    ];
    e.mem.write(data, &d);
    e.mem.set_u32(script + 0x30, data);
    e.mem.set_u32(script + 0x20, 18);
}

/// NCCS: an actor base with a script has the script disabled (length 0, one MergeEventMask user)
/// and its activation disabled (the flag, one DoActivate user), once per base; set back by
/// CCCSetNCCS 0 or CCCLoadNCCS (every base, the set emptied).
#[test]
fn nccs() {
    let mut e = setup();
    script_with_data(&mut e, SCRIPT, SDATA);
    script_with_data(&mut e, SCRIPT2, SDATA + 0x40);
    e.mem.set_u32(NPC + 0xf8, SCRIPT);
    e.mem.set_u32(NPC2 + 0xf8, SCRIPT2);
    run(&mut e, 0x2705, ACTOR, &[]);
    assert_eq!(e.mem.u32(SCRIPT + 0x20), 0);
    assert_eq!(e.mem.u8(NPC + 6) & 0x08, 0x08);
    assert_eq!(e.hook_users(Hook::MergeEventMask), 1);
    assert_eq!(e.hook_users(Hook::DoActivate), 1);
    run(&mut e, 0x2705, ACTOR, &[n(5)]);
    assert_eq!(e.hook_users(Hook::DoActivate), 1, "already in the set");
    run(&mut e, 0x2705, ACTOR2, &[n(1)]);
    assert_eq!(e.hook_users(Hook::DoActivate), 2);
    run(&mut e, 0x2705, ACTOR, &[n(0)]);
    assert_eq!(e.mem.u32(SCRIPT + 0x20), 18, "the length counted again");
    assert_eq!(e.mem.u8(NPC + 6) & 0x08, 0);
    assert_eq!(e.hook_users(Hook::DoActivate), 1);
    run(&mut e, 0x2705, ACTOR, &[n(0)]);
    assert_eq!(
        e.hook_users(Hook::MergeEventMask),
        1,
        "not in the set: nothing"
    );
    run(&mut e, 0x2705, ACTOR, &[n(1)]);
    run(&mut e, 0x2707, 0, &[]);
    assert_eq!(e.mem.u32(SCRIPT + 0x20), 18);
    assert_eq!(e.mem.u32(SCRIPT2 + 0x20), 18);
    assert_eq!(e.hook_users(Hook::MergeEventMask), 0);
    assert_eq!(e.hook_users(Hook::DoActivate), 0);
    assert!(e.ext::<Jip>().ccc.nccs_actors.is_empty());
    // A base with no script joins the set and nothing else; a non-actor nothing.
    e.mem.set_u32(NPC + 0xf8, 0);
    run(&mut e, 0x2705, ACTOR, &[]);
    run(&mut e, 0x2705, PLAIN, &[]);
    assert_eq!(
        e.ext::<Jip>()
            .ccc
            .nccs_actors
            .iter()
            .copied()
            .collect::<Vec<_>>(),
        vec![NPC]
    );
    assert_eq!(e.hook_users(Hook::DoActivate), 0);
}

/// `100 * weight / max` in doubles; 0 for a non-actor, no changes, no data, a maximum of 0 or
/// below (and then the weight is not asked); the execute and the eval.
#[test]
fn encumbrance() {
    let mut e = setup();
    assert_eq!(run(&mut e, 0x2706, ACTOR, &[]), 0.0);
    put_extra(&mut e, ACTOR + 0x44, X1, 0x15, 0);
    assert_eq!(run(&mut e, 0x2706, ACTOR, &[]), 0.0, "no data");
    e.mem.set_u32(X1 + 0xc, CHANGES);
    e.mem.set_f32(CHANGES + 0x20, 37.5);
    e.mem.set_f32(ACTOR + 0x64, 0.0);
    assert_eq!(run(&mut e, 0x2706, ACTOR, &[]), 0.0);
    e.mem.set_f32(ACTOR + 0x64, -1.0);
    assert_eq!(eval(&mut e, 0x2706, ACTOR), 0.0, "the eval writes 0");
    assert!(calls_to(0x004d_0900).is_empty(), "no weight asked");
    e.mem.set_f32(ACTOR + 0x64, 300.0);
    let want = 100.0 * 37.5 / 300.0;
    assert_eq!(run(&mut e, 0x2706, ACTOR, &[]), want);
    assert_eq!(eval(&mut e, 0x2706, ACTOR), want);
    assert_eq!(
        e.mem.u32(CHANGES + 8),
        0xbf80_0000,
        "the cached weight reset"
    );
    assert_eq!(calls_to(MAX_CARRY_WEIGHT)[0], vec![ACTOR]);
    e.mem.set_f32(ACTOR + 0x64, 7.0);
    e.mem.set_f32(CHANGES + 0x20, 1.0);
    assert_eq!(run(&mut e, 0x2706, ACTOR, &[]), 100.0 * 1.0 / 7.0);
    assert_eq!(eval(&mut e, 0x2706, PLAIN), 0.0);
    assert_eq!(eval(&mut e, 0x2706, 0), 0.0);
}

#[test]
fn reputation() {
    let mut e = setup();
    run(&mut e, 0x270a, ACTOR, &[]);
    assert_eq!(form_result(&e), 0);
    e.mem.set_u32(FACTION2 + 0x38, REP2);
    node(&mut e, NPC + 0x5c, 0, NODES);
    node(&mut e, NODES, DATA, NODES + 8);
    node(&mut e, NODES + 8, DATA + 8, 0);
    e.mem.set_u32(DATA, FACTION);
    e.mem.set_u32(DATA + 8, FACTION2);
    run(&mut e, 0x270a, ACTOR, &[]);
    assert_eq!(form_result(&e), 0x0100_0980, "the first with one");
    e.mem.set_u32(FACTION + 0x38, REP);
    run(&mut e, 0x270a, ACTOR, &[]);
    assert_eq!(form_result(&e), 0x0100_0900);
    // Nothing in the base's list: the reference's own changes.
    node(&mut e, NPC + 0x5c, 0, 0);
    put_extra(&mut e, ACTOR + 0x44, X1, 0x5e, NODES + 0x10);
    node(&mut e, NODES + 0x10, 0, NODES + 0x18);
    node(&mut e, NODES + 0x18, DATA + 0x10, 0);
    e.mem.set_u32(DATA + 0x10, FACTION2);
    run(&mut e, 0x270a, ACTOR, &[]);
    assert_eq!(form_result(&e), 0x0100_0980);
    // An entry with no faction is passed over.
    e.mem.set_u32(DATA + 0x10, 0);
    run(&mut e, 0x270a, ACTOR, &[]);
    assert_eq!(form_result(&e), 0);
    run(&mut e, 0x270a, PLAIN, &[]);
    assert_eq!(form_result(&e), 0);
}

#[test]
fn say_to_runs_the_end_script() {
    let mut e = setup();
    e.mem.set_u32(PLAYER_AT + 0x100, INFO);
    run(&mut e, 0x2752, PLAIN, &[]);
    assert!(calls_to(SAY_TO).is_empty(), "not an actor");
    set_ref(&mut e, 6, ACTOR2);
    run(&mut e, 0x2752, ACTOR, &[r(6), r(5)]);
    let say = calls_to(SAY_TO);
    assert_eq!(say.len(), 1);
    assert_eq!(say[0].len(), 8, "the command's eight words");
    assert_eq!(say[0][2], ACTOR);
    assert_eq!(calls_to(RUN_RESULT_SCRIPT), vec![vec![INFO, 1, ACTOR]]);
    assert_eq!(calls_to(REMOVE_BY_TYPE_FN), vec![vec![ACTOR + 0x44, 0x75]]);
    // An extra with no info: removed, no script.
    e.mem.set_u32(PLAYER_AT + 0x100, 0);
    run(&mut e, 0x2752, ACTOR, &[r(6), r(5)]);
    assert_eq!(calls_to(RUN_RESULT_SCRIPT).len(), 1);
    assert_eq!(calls_to(REMOVE_BY_TYPE_FN).len(), 2);
    // No extra left by the game: nothing more.
    e.mem.set_u32(PLAYER_AT + 0x100, u32::MAX);
    run(&mut e, 0x2752, ACTOR2, &[r(1), r(5)]);
    assert_eq!(calls_to(REMOVE_BY_TYPE_FN).len(), 2);
}

#[test]
fn result_scripts() {
    let mut e = setup();
    run(&mut e, 0x2753, ACTOR, &[r(5)]);
    let asked = calls_to(GET_TOPIC_INFO);
    assert_eq!(asked.len(), 1);
    let w = &asked[0];
    assert_eq!(
        (w[0], w[2], w[3], w[4], w[5], w[6], w[7]),
        (TOPIC, ACTOR, PLAYER_AT, 1, 0, 0, TOPIC),
        "the topic's variable as the result's place"
    );
    assert!(calls_to(RUN_RESULT_SCRIPT).is_empty(), "no info");
    e.mem.set_u32(PLAYER_AT + 0x104, INFO);
    run(&mut e, 0x2753, ACTOR, &[r(5)]);
    assert_eq!(
        calls_to(RUN_RESULT_SCRIPT),
        vec![vec![INFO, 0, ACTOR], vec![INFO, 1, ACTOR]]
    );
    run(&mut e, 0x2753, PLAIN, &[r(5)]);
    assert_eq!(calls_to(GET_TOPIC_INFO).len(), 2, "not an actor");
}

#[test]
fn position_text() {
    let mut e = setup();
    let pos = |e: &mut Engine, this: u32| {
        let v = run(e, 0x2762, this, &[]);
        string_result(e, v)
    };
    assert_eq!(pos(&mut e, PLAIN), b"");
    place(&mut e, PLAIN, -12.7, 4095.99, 5.9);
    e.mem.set_u32(PLAIN + 0x40, CELL);
    assert_eq!(pos(&mut e, PLAIN), b"1001000 -12 4095 6");
    e.mem.set_u32(CELL + 0xc0, WORLD);
    place(&mut e, PLAIN, 0.5, -0.5, -1.5);
    assert_eq!(
        pos(&mut e, PLAIN),
        b"1001300 0 0 0",
        "the worldspace's id; z+1 after the cut"
    );
    // The persistent cell's extra when the reference has no parent cell.
    e.mem.set_u32(PLAIN + 0x40, 0);
    put_extra(&mut e, PLAIN + 0x44, X1, 0x0c, CELL2);
    form(&mut e, CELL2, 0x39, 0xab);
    assert_eq!(pos(&mut e, PLAIN), b"AB 0 0 0");
}

#[test]
fn task_package_flags() {
    let mut e = setup();
    let base: u32 = 0x8000_0001;
    let set = |e: &mut Engine, t: i32, v1: i32, v2: i32| {
        e.mem.set_u32(PACKAGE + 0x1c, base | 0x3fff_fffe);
        run(e, 0x2702, 0, &[r(4), n(t), n(v1), n(v2)]);
        e.mem.u32(PACKAGE + 0x1c)
    };
    let all = base | 0x3fff_fffe;
    let (run_, sneak, drawn, idle, cont, alert, walk) = (
        1 << 13,
        1 << 17,
        1 << 23,
        1 << 24,
        1 << 26,
        1 << 27,
        1 << 29,
    );
    assert_eq!(
        set(&mut e, 1, 1, 0),
        (all | idle | alert | walk) & !(run_ | drawn | cont) & !sneak
    );
    assert_eq!(
        set(&mut e, 2, 0, 1),
        (all | run_ | idle | alert) & !(drawn | cont | walk) | sneak
    );
    assert_eq!(
        set(&mut e, 0, 3, 0),
        (all | run_ | drawn | cont) & !(idle | alert | walk) & !sneak
    );
    assert_eq!(
        set(&mut e, 0, 0, -1),
        (all | run_ | drawn) & !(idle | cont | alert | walk) | sneak
    );
    // From all clear, each case's set bits.
    e.mem.set_u32(PACKAGE + 0x1c, 0);
    run(&mut e, 0x2702, 0, &[r(4), n(0), n(0), n(0)]);
    assert_eq!(e.mem.u32(PACKAGE + 0x1c), run_ | drawn);
    // The second variable sets AlwaysSneak where it was clear.
    e.mem.set_u32(PACKAGE + 0x1c, 0);
    run(&mut e, 0x2702, 0, &[r(4), n(0), n(0), n(1)]);
    assert_eq!(e.mem.u32(PACKAGE + 0x1c), run_ | drawn | sneak);
}

/// Interior cells, no cell, the worldspace fallback, and the map marker search: the grid built
/// from the persistent cell's named markers (unnamed and other references left out; a later
/// marker in a grid cell replaces an earlier), the nearest in the first ring with any, "at"
/// within 4096 units, the direction words beyond, and the grid kept from the first call.
#[test]
fn location_names() {
    let mut e = setup();
    let name = |e: &mut Engine| {
        let v = run(e, 0x2709, PLAIN, &[]);
        string_result(e, v)
    };
    assert_eq!(name(&mut e), b"at an unknown location");
    e.mem.set_u32(PLAIN + 0x40, CELL);
    text(&mut e, CELL + 0x1c, TEXT, b"Doc's House");
    assert_eq!(name(&mut e), b"in Doc's House");
    e.mem.set_u32(CELL + 0x1c, 0);
    assert_eq!(name(&mut e), b"in ", "no name");
    // An exterior: worldspace with no persistent cell: its name.
    e.mem.set_u32(CELL + 0xc0, WORLD);
    text(&mut e, WORLD + 0x1c, TEXT + 0x40, b"Mojave");
    assert_eq!(name(&mut e), b"at Mojave");
    // Markers (the grid is built at first use, so a new worldspace state is needed: clear it).
    e.ext::<Jip>().ccc.marker_grids.clear();
    e.mem.set_u32(WORLD + 0x34, PCELL);
    e.mem.set_u16(WORLD + 0x2c, 1);
    for (m, md, x) in [(M1, MD1, X2), (M2, MD2, X3), (M3, MD3, X4)] {
        e.mem.set_u32(m + 0x20, MARKER_STATIC);
        put_extra(&mut e, m + 0x44, x, 0x2c, md);
    }
    text(&mut e, MD1 + 4, TEXT + 0x80, b"Goodsprings");
    text(&mut e, MD2 + 4, TEXT + 0xc0, b"Primm");
    e.mem.set_u32(MD3 + 4, 0);
    node(&mut e, PCELL + 0xac, PLAIN, NODES);
    node(&mut e, NODES, M1, NODES + 8);
    node(&mut e, NODES + 8, M2, NODES + 0x10);
    node(&mut e, NODES + 0x10, M3, 0);
    place(&mut e, M1, 100.0, 100.0, 0.0);
    place(&mut e, M2, 9000.0, 100.0, 0.0);
    place(&mut e, M3, 50.0, 50.0, 0.0);
    place(&mut e, PLAIN, 10.0, 10.0, 0.0);
    assert_eq!(
        name(&mut e),
        b"at Goodsprings",
        "same grid cell, within 4096"
    );
    let grid = e.ext::<Jip>().ccc.marker_grids[&WORLD].clone();
    assert_eq!(
        grid.len(),
        2,
        "the unnamed marker and the plain reference left out"
    );
    assert_eq!(grid[&grid_cell(100.0, 100.0)], M1);
    assert_eq!(grid[&grid_cell(9000.0, 100.0)], M2);
    // Two cells east of Primm's: the ring at radius 2 finds it, east.
    place(&mut e, PLAIN, 9000.0 + 8192.0, 100.0, 0.0);
    assert_eq!(name(&mut e), b"East of Primm");
    place(&mut e, PLAIN, 9000.0 - 8192.0 + 4096.0 * 0.0, 100.0, 0.0);
    // 808 east of Goodsprings' cell... the nearest in ring 0 is Goodsprings (cell 0,0).
    assert_eq!(name(&mut e), b"at Goodsprings");
    place(&mut e, PLAIN, 9000.0 - 4500.0, 100.0 + 4500.0 * 3.0, 0.0);
    let n = name(&mut e);
    assert!(
        n.ends_with(b"of Goodsprings") || n.ends_with(b"of Primm"),
        "{n:?}"
    );
    // The grid is kept: a marker added later is not seen.
    e.mem.set_u32(NODES + 0x14, NODES + 0x18);
    node(&mut e, NODES + 0x18, M3, 0);
    text(&mut e, MD3 + 4, TEXT + 0x100, b"Nipton");
    place(&mut e, PLAIN, 50.0, 50.0, 0.0);
    assert_eq!(name(&mut e), b"at Goodsprings");
    // Beyond 8 rings: nothing found, the worldspace's name.
    place(&mut e, PLAIN, 4096.0 * 20.0, 4096.0 * 20.0, 0.0);
    assert_eq!(name(&mut e), b"at Mojave");
}

/// The direction words by the x component of the unit vector to the reference (`>= 0.97`
/// east, `<= -0.97` west, within 0.26 north or south by y, else the quarter), from one marker
/// at the origin with the reference 5000 units out at each angle.
#[test]
fn location_directions() {
    let mut e = setup();
    e.mem.set_u32(PLAIN + 0x40, CELL);
    e.mem.set_u32(CELL + 0xc0, WORLD);
    e.mem.set_u32(WORLD + 0x34, PCELL);
    e.mem.set_u32(WORLD + 0x70, WORLD);
    e.mem.set_u32(M1 + 0x20, MARKER_STATIC);
    put_extra(&mut e, M1 + 0x44, X2, 0x2c, MD1);
    text(&mut e, MD1 + 4, TEXT, b"Here");
    node(&mut e, PCELL + 0xac, M1, 0);
    place(&mut e, M1, 0.0, 0.0, 0.0);
    let at = |e: &mut Engine, x: f32, y: f32| {
        place(e, PLAIN, x, y, 0.0);
        let v = run(e, 0x2709, PLAIN, &[]);
        string_result(e, v)
    };
    assert_eq!(at(&mut e, 5000.0, 0.0), b"East of Here");
    assert_eq!(at(&mut e, -5000.0, 0.0), b"West of Here");
    assert_eq!(at(&mut e, 0.0, 5000.0), b"North of Here");
    assert_eq!(at(&mut e, 0.0, -5000.0), b"South of Here");
    assert_eq!(at(&mut e, 3536.0, 3536.0), b"NE of Here");
    assert_eq!(at(&mut e, 3536.0, -3536.0), b"SE of Here");
    assert_eq!(at(&mut e, -3536.0, 3536.0), b"NW of Here");
    assert_eq!(at(&mut e, -3536.0, -3536.0), b"SW of Here");
    assert_eq!(at(&mut e, 4000.0, 0.0), b"at Here", "within 4096");
    // The edges: x/d of 0.97 exactly is east (computed as the command does: float division).
    let d = super::super::ln_fn_math::point3_distance([4850.0, 1215.6, 0.0], [0.0; 3]);
    let x = 4850.0f32 / d;
    let want: &[u8] = if (0.97..=1.0).contains(&x) {
        b"East of Here"
    } else {
        b"NE of Here"
    };
    assert_eq!(at(&mut e, 4850.0, 1215.6), want);
}

/// Two named markers in the same ring (radius 2 around the reference's cell 0,0) at different
/// distances. JIP's ring walk starts at (-2,-2) going +y, then +x along y=2, -y along x=2, -x
/// along y=-2: cell (2,0) is checked 11th, (0,-2) 15th; every hit nearer than the best so far
/// replaces it, so the nearer one wins whichever is met first.
#[test]
fn location_two_markers_in_one_ring() {
    let mut e = setup();
    e.mem.set_u32(PLAIN + 0x40, CELL);
    e.mem.set_u32(CELL + 0xc0, WORLD);
    e.mem.set_u32(WORLD + 0x34, PCELL);
    e.mem.set_u32(WORLD + 0x70, WORLD);
    for (m, md, x) in [(M1, MD1, X2), (M2, MD2, X3)] {
        e.mem.set_u32(m + 0x20, MARKER_STATIC);
        put_extra(&mut e, m + 0x44, x, 0x2c, md);
    }
    text(&mut e, MD1 + 4, TEXT, b"Goodsprings");
    text(&mut e, MD2 + 4, TEXT + 0x40, b"Primm");
    node(&mut e, PCELL + 0xac, M1, NODES);
    node(&mut e, NODES, M2, 0);
    place(&mut e, PLAIN, 2048.0, 2048.0, 0.0);
    // Goodsprings in cell (2,0), 8144 east; Primm in cell (0,-2), 7240 south: Primm, met
    // second, is nearer. x/d = 0 (within 0.26) and y - y' = 7240 > 0: "North of".
    place(&mut e, M1, 10192.0, 2048.0, 0.0);
    place(&mut e, M2, 2048.0, -5192.0, 0.0);
    let name = |e: &mut Engine| {
        let v = run(e, 0x2709, PLAIN, &[]);
        string_result(e, v)
    };
    assert_eq!(name(&mut e), b"North of Primm");
    // Goodsprings moved to 6244 east (still cell 2,0): now nearer and met first; x/d = -1:
    // "West of".
    place(&mut e, M1, 8292.0, 2048.0, 0.0);
    assert_eq!(name(&mut e), b"West of Goodsprings");
}

/// The grid cell: truncation, then an arithmetic shift by 12 (so -1 is cell -1), x high, y low.
#[test]
fn grid_cells() {
    assert_eq!(grid_cell(0.0, 0.0), 0);
    assert_eq!(grid_cell(4095.9, 4096.0), 1);
    assert_eq!(grid_cell(4096.0, 0.0), 0x0001_0000);
    assert_eq!(
        grid_cell(-0.5, -1.0),
        0x0000_ffff,
        "-0.5 truncates to 0; -1 >> 12 is -1"
    );
    assert_eq!(grid_cell(-4096.0, -4097.0), 0xffff_fffe);
    assert_eq!(
        grid_cell(f32::NAN, 1e10),
        0,
        "the indefinite >> 12: 0xFFF80000, low 16 bits 0"
    );
}

/// A leveled actor (its base made in the game, mod index 0xFF, with an `ExtraLeveledCreature`
/// naming another base): JIP's CC&C commands read the raw `baseForm`, not `GetActorBase`.
#[test]
fn a_leveled_actor_uses_its_raw_base() {
    let mut e = setup();
    form(&mut e, NPC, 0x2a, 0xff00_0400);
    put_extra(&mut e, ACTOR + 0x44, X4, 0x2e, NPC2);
    e.mem.set_u32(X4 + 0x10, NPC2);
    assert_eq!(super::super::ln_fn_actor::actor_base(&mut e, ACTOR), NPC2);
    node(&mut e, NPC + 0x5c, DATA, 0);
    e.mem.set_u32(DATA, FACTION);
    e.mem.set_u8(DATA + 4, 1);
    assert_eq!(
        run(&mut e, 0x2704, ACTOR, &[r(2), n(1)]),
        1.0,
        "the raw base's faction"
    );
    e.mem.set_u32(FACTION + 0x38, REP);
    run(&mut e, 0x270a, ACTOR, &[]);
    assert_eq!(form_result(&e), 0x0100_0900, "the raw base's reputation");
    script_with_data(&mut e, SCRIPT, SDATA);
    e.mem.set_u32(NPC + 0xf8, SCRIPT);
    run(&mut e, 0x2705, ACTOR, &[]);
    assert_eq!(
        e.ext::<Jip>()
            .ccc
            .nccs_actors
            .iter()
            .copied()
            .collect::<Vec<_>>(),
        vec![NPC]
    );
    assert_eq!(
        e.mem.u32(SCRIPT + 0x20),
        0,
        "the raw base's script disabled"
    );
}
