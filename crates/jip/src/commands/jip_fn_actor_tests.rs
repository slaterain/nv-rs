//! Tests of `jip_fn_actor` (first half) on actor bases and creatures at the offsets the exe's
//! save and set functions use (base data +0x30: flags +0x34, level +0x3C, calc min/max
//! +0x3E/+0x40, voice +0x50, template +0x54; creature damage +0x110, models +0x118, reach
//! +0x130), actors (vtable +0x100 true) with processes whose virtual functions are stand-ins,
//! and the game calls recorded. Expectations are read from JIP's source.

use super::*;
use crate::commands::test_support::*;
use crate::test_support::short_arg;
use nvse::vars::Data;
use std::cell::RefCell;

const NPC: u32 = FORMS + 0x1000;
const CREA: u32 = FORMS + 0x1400;
const ACTOR: u32 = FORMS + 0x2000;
const CR_ACTOR: u32 = FORMS + 0x2200;
const PLAIN_REF: u32 = FORMS + 0x2400;
const PLAYER_REF: u32 = FORMS + 0x2600;
const ACTI: u32 = FORMS + 0x2800;
const TACT: u32 = FORMS + 0x2900;
const CELL: u32 = FORMS + 0x2a00;
const VTYP: u32 = FORMS + 0x2b00;
const LIST: u32 = FORMS + 0x2c00;
const ITEM1: u32 = FORMS + 0x2d00;
const ITEM2: u32 = FORMS + 0x2e00;
const PACK: u32 = FORMS + 0x2f00;
const AMMO: u32 = FORMS + 0x3000;
const MISC: u32 = FORMS + 0x3100;
const OTHER: u32 = FORMS + 0x3200;
const OTHER2: u32 = FORMS + 0x3400;
const ACTOR_VT: u32 = FORMS + 0x4000;
const BASE_VT: u32 = FORMS + 0x4400;
const TARGET_VT: u32 = FORMS + 0x4800;
const PROC_VT: u32 = FORMS + 0x4c00;
const AVO_VT: u32 = FORMS + 0x5000;
const PROC: u32 = FORMS + 0x6000;
const PROC2: u32 = FORMS + 0x6800;
const CACHED: u32 = FORMS + 0x6f00;
const X1: u32 = FORMS + 0x7000;
const X2: u32 = FORMS + 0x7100;
const NODES: u32 = FORMS + 0x8000;
const BLOCK: u32 = FORMS + 0x9000;
const GROUP: u32 = FORMS + 0xa000;
const TEXTS: u32 = FORMS + 0xb000;
const PM: u32 = 0x011e_0e80;
const PLAYER_PTR: u32 = 0x011d_ea3c;
const F_GET_AV: u32 = 0x0090_0200;
const F_GET_BASE_AV: u32 = 0x0090_0204;
const F_MOD_AV: u32 = 0x0090_0208;
const F_SET_AV_INT: u32 = 0x0090_020c;
const F_EFFECTS: u32 = 0x0090_0210;
const F_STOP_IDLE: u32 = 0x0090_0214;
const F_SET_BREATH: u32 = 0x0090_0218;
const F_GET_BREATH: u32 = 0x0090_021c;
const F_ACTION: u32 = 0x0090_0220;
const F_WEAPON_OUT: u32 = 0x0090_0224;
const F_SET_WEAPON_OUT: u32 = 0x0090_0228;
const F_DETECTION: u32 = 0x0090_022c;
const F_ALPHA: u32 = 0x0090_0230;

thread_local! {
    /// What the process's `GetDetectionState` stand-in answers: (process, target) -> data.
    static DETECTION: RefCell<Vec<(u32, u32, u32)>> = const { RefCell::new(Vec::new()) };
    /// The words of the stand-in calls, by address.
    static SEEN: RefCell<Vec<(u32, Vec<u32>)>> = const { RefCell::new(Vec::new()) };
}

fn seen(addr: u32) -> Vec<Vec<u32>> {
    SEEN.with(|s| {
        s.borrow()
            .iter()
            .filter(|c| c.0 == addr)
            .map(|c| c.1.clone())
            .collect()
    })
}

fn ret(eax: u32) -> Ret {
    Ret {
        eax,
        ..Ret::default()
    }
}

fn fret(st0: f32) -> Ret {
    Ret {
        st0: f64::from(st0),
        ..Ret::default()
    }
}

fn node(e: &mut Engine, at: u32, data: u32, next: u32) {
    e.mem.set_u32(at, data);
    e.mem.set_u32(at + 4, next);
}

fn actor(e: &mut Engine, at: u32, base: u32, id: u32) {
    form(e, at, 0x3b, id);
    e.mem.set_u32(at, ACTOR_VT);
    e.mem.set_u32(at + 0x20, base);
}

fn setup() -> Engine {
    SEEN.with(|s| s.borrow_mut().clear());
    DETECTION.with(|d| d.borrow_mut().clear());
    let mut e = engine();
    extra_stand_ins(&mut e);
    game_globals(&mut e);
    e.map(0x011d_e000, 0x1000);
    e.map(0x011e_0000, 0x1000);
    e.map(0x0119_b000, 0x1000);
    form(&mut e, NPC, 0x2a, 0x100);
    form(&mut e, CREA, 0x2b, 0x101);
    form(&mut e, ACTI, 0x15, 0x102);
    form(&mut e, TACT, 0x16, 0x103);
    e.mem.set_u32(TACT, BASE_VT + 0x200);
    form(&mut e, CELL, 0x39, 0x104);
    form(&mut e, VTYP, 0x5d, 0x105);
    form(&mut e, LIST, 0x55, 0x106);
    form(&mut e, ITEM1, 0x1f, 0x107);
    form(&mut e, ITEM2, 0x1f, 0x108);
    form(&mut e, PACK, 0x49, 0x109);
    form(&mut e, AMMO, 0x29, 0x10a);
    form(&mut e, MISC, 0x1f, 0x10b);
    for b in [NPC, CREA] {
        e.mem.set_u32(b, BASE_VT);
        e.mem.set_u32(b + 0x100, AVO_VT);
    }
    e.mem.set_u32(BASE_VT + 0x198, F_MOD_AV);
    e.mem.set_u32(AVO_VT + 4, F_GET_BASE_AV);
    e.mem.set_u32(AVO_VT + 0xc, F_GET_AV);
    e.register(F_MOD_AV, |_, w| {
        SEEN.with(|s| s.borrow_mut().push((F_MOD_AV, w.to_vec())));
        ret(0)
    });
    e.register(F_GET_AV, |_, w| fret(w[1] as f32 + 0.75));
    e.register(F_GET_BASE_AV, |_, w| fret(-(w[1] as f32) - 0.75));
    // Actors: vtable +0x100 true; SetActorValueInt +0x398; the magic target (+0x94) answers its
    // effect list (vtable +8).
    e.mem.set_u32(ACTOR_VT + 0x100, 0x008d_0360);
    e.mem.set_u32(ACTOR_VT + 0x398, F_SET_AV_INT);
    e.register(F_SET_AV_INT, |_, w| {
        SEEN.with(|s| s.borrow_mut().push((F_SET_AV_INT, w.to_vec())));
        ret(0)
    });
    actor(&mut e, ACTOR, NPC, 0x200);
    actor(&mut e, CR_ACTOR, CREA, 0x201);
    actor(&mut e, PLAYER_REF, NPC, 0x14);
    actor(&mut e, OTHER, NPC, 0x202);
    actor(&mut e, OTHER2, NPC, 0x203);
    e.mem.set_u32(PLAYER_PTR, PLAYER_REF);
    for a in [ACTOR, CR_ACTOR, PLAYER_REF, OTHER, OTHER2] {
        e.mem.set_u32(a + 0xa4, AVO_VT);
        e.mem.set_u32(a + 0x94, TARGET_VT);
    }
    e.mem.set_u32(TARGET_VT + 8, F_EFFECTS);
    e.register(F_EFFECTS, |_, w| {
        ret(if w[0] == ACTOR + 0x94 { NODES } else { 0 })
    });
    form(&mut e, PLAIN_REF, 0x3a, 0x204);
    e.mem.set_u32(PLAIN_REF, ACTOR_VT + 0x300);
    e.mem.set_u32(PLAIN_REF + 0x20, ACTI);
    // Processes: ACTOR's high (level 0), PLAYER_REF's high; the stand-ins.
    for (a, p) in [(ACTOR, PROC), (PLAYER_REF, PROC2)] {
        e.mem.set_u32(a + 0x68, p);
        e.mem.set_u32(p, PROC_VT);
    }
    for (slot, f) in [
        (0xd8, F_STOP_IDLE),
        (0x2fc, F_SET_BREATH),
        (0x300, F_GET_BREATH),
        (0x3e4, F_ACTION),
        (0x454, F_WEAPON_OUT),
        (0x458, F_SET_WEAPON_OUT),
        (0x504, F_DETECTION),
        (0x5ac, F_ALPHA),
    ] {
        e.mem.set_u32(PROC_VT + slot, f);
    }
    e.register(F_STOP_IDLE, |_, w| {
        SEEN.with(|s| s.borrow_mut().push((F_STOP_IDLE, w.to_vec())));
        ret(0)
    });
    e.register(F_SET_BREATH, |_, w| {
        SEEN.with(|s| s.borrow_mut().push((F_SET_BREATH, w.to_vec())));
        ret(0)
    });
    e.register(F_SET_WEAPON_OUT, |_, w| {
        SEEN.with(|s| s.borrow_mut().push((F_SET_WEAPON_OUT, w.to_vec())));
        ret(0)
    });
    e.register(F_GET_BREATH, |e, w| fret(e.mem.f32(w[0] + 0x400)));
    e.register(F_ALPHA, |e, w| fret(e.mem.f32(w[0] + 0x404)));
    e.register(F_ACTION, |e, w| {
        ret(u32::from(e.mem.u16(w[0] + 0x408)) | 0xabcd_0000)
    });
    e.register(F_WEAPON_OUT, |e, w| {
        ret(u32::from(e.mem.u8(w[0] + 0x40c)) | 0x100)
    });
    e.register(F_DETECTION, |_, w| {
        assert_eq!(w[2], 0, "GetDetectionData(target, 0)");
        let d = DETECTION.with(|d| {
            d.borrow()
                .iter()
                .find(|x| x.0 == w[0] && x.1 == w[1])
                .map_or(0, |x| x.2)
        });
        ret(d)
    });
    let refs = [NPC, CREA, VTYP, LIST, ITEM1, OTHER, MISC, ACTI, TACT, CELL];
    for (i, f) in refs.iter().enumerate() {
        set_ref(&mut e, i as u16 + 1, *f);
    }
    e
}

fn eval(e: &mut Engine, opcode: u32, this: u32) -> f64 {
    let record = nvse::table::record_of(e, opcode);
    let at = e.mem.u32(record + 0x20);
    e.mem.set_f64(RESULT, 0.0);
    e.call(at, &[this, 0, 0, RESULT]);
    e.mem.f64(RESULT)
}

fn forms_of(e: &Engine, id: f64) -> Vec<u32> {
    array_data(e, id)
        .into_iter()
        .map(|d| match d {
            Data::Form(f) => f,
            other => panic!("{other:?}"),
        })
        .collect()
}

#[test]
fn templates_voice_types_and_the_leveled_base() {
    let mut e = setup();
    assert_eq!(call(&mut e, 0x2601, &[r(1)]).1, 0.0, "no template");
    e.mem.set_u32(NPC + 0x54, CREA);
    call(&mut e, 0x2601, &[r(1)]);
    assert_eq!(form_result(&e), 0x101);
    call_on(&mut e, 0x2601, ACTOR, &[]);
    assert_eq!(form_result(&e), 0x101, "the calling actor's base");
    e.mem.set_u32(RESULT, 0);
    call_on(&mut e, 0x2601, PLAIN_REF, &[]);
    assert_eq!(form_result(&e), 0, "not an actor");
    // Voice types: only a voice type form is set.
    call(&mut e, 0x26d6, &[r(4), r(1)]);
    assert_eq!(e.mem.u32(NPC + 0x50), 0, "a form list is not a voice type");
    call_on(&mut e, 0x26d6, ACTOR, &[r(3)]);
    assert_eq!(e.mem.u32(NPC + 0x50), VTYP);
    e.mem.set_u32(RESULT, 0);
    call(&mut e, 0x26d5, &[r(2)]);
    assert_eq!(form_result(&e), 0, "the creature has none");
    call(&mut e, 0x26d5, &[r(1)]);
    assert_eq!(form_result(&e), 0x105);
    // The leveled base: a base of file 0xFF answers its ExtraLeveledCreature's base.
    call_on(&mut e, 0x2602, ACTOR, &[]);
    assert_eq!(form_result(&e), 0x100);
    e.mem.set_u32(NPC + 0xc, 0xff00_0100);
    put_extra(&mut e, ACTOR + 0x44, X1, 0x2e, 0);
    e.mem.set_u32(X1 + 0x10, CREA);
    call_on(&mut e, 0x2602, ACTOR, &[]);
    assert_eq!(form_result(&e), 0x101);
    e.mem.set_u32(RESULT, 0);
    call_on(&mut e, 0x2602, PLAIN_REF, &[]);
    call_on(&mut e, 0x2602, 0, &[]);
    assert_eq!(form_result(&e), 0, "no actor: nothing");
}

#[test]
fn creature_damage_reach_and_immobility() {
    let mut e = setup();
    e.mem.set_u16(CREA + 0x110, 35);
    e.mem.set_u8(CREA + 0x130, 200);
    assert_eq!(call(&mut e, 0x2603, &[r(2)]).1, 35.0);
    assert_eq!(call_on(&mut e, 0x2603, CR_ACTOR, &[]).1, 35.0);
    assert_eq!(
        call(&mut e, 0x2603, &[r(1)]).1,
        0.0,
        "an NPC is no creature"
    );
    assert_eq!(call(&mut e, 0x26d7, &[r(2)]).1, 200.0);
    assert_eq!(call(&mut e, 0x26d7, &[r(1)]).1, 0.0);
    call(&mut e, 0x2604, &[n(70_000), r(2)]);
    assert_eq!(e.mem.u16(CREA + 0x110), 0xffff);
    call_on(&mut e, 0x2604, CR_ACTOR, &[n(-4)]);
    assert_eq!(e.mem.u16(CREA + 0x110), 0);
    call(&mut e, 0x2604, &[n(9), r(2)]);
    assert_eq!(e.mem.u16(CREA + 0x110), 9);
    e.mem.set_u16(NPC + 0x110, 7);
    call(&mut e, 0x2604, &[n(9), r(1)]);
    assert_eq!(e.mem.u16(NPC + 0x110), 7, "an NPC is left alone");
    // Immobile: the caller's byte without an argument, a creature's flag 0x800000 with one.
    assert_eq!(call_on(&mut e, 0x26d8, CR_ACTOR, &[]).1, 0.0);
    e.mem.set_u8(CR_ACTOR + 0x1b0, 1);
    assert_eq!(call_on(&mut e, 0x26d8, CR_ACTOR, &[]).1, 1.0);
    assert_eq!(
        call_on(&mut e, 0x26d8, CR_ACTOR, &[r(2)]).1,
        0.0,
        "the flag decides"
    );
    e.mem.set_u32(CREA + 0x34, 0x0080_0000);
    e.mem.set_u32(NPC + 0x34, 0x0080_0000);
    assert_eq!(call(&mut e, 0x26d8, &[r(2)]).1, 1.0);
    assert_eq!(call(&mut e, 0x26d8, &[r(1)]).1, 0.0, "an NPC base: 0");
    assert_eq!(call_on(&mut e, 0x26d8, PLAIN_REF, &[]).1, 0.0);
}

#[test]
fn leveling_data() {
    let mut e = setup();
    e.mem.set_u16(NPC + 0x3c, (-3i16) as u16);
    e.mem.set_u16(NPC + 0x3e, 2);
    e.mem.set_u16(NPC + 0x40, 40);
    assert_eq!(call(&mut e, 0x2689, &[n(0), r(1)]).1, -3.0);
    assert_eq!(call(&mut e, 0x2689, &[n(1), r(1)]).1, 2.0);
    assert_eq!(call_on(&mut e, 0x2689, ACTOR, &[n(2)]).1, 40.0);
    assert_eq!(
        call(&mut e, 0x2689, &[n(3), r(1)]).1,
        0.0,
        "above 2: nothing"
    );
    e.mem.set_u32(NPC + 0x34, 0x80);
    e.mem.set_u16(NPC + 0x3c, 1500);
    assert_eq!(call(&mut e, 0x2689, &[n(0), r(1)]).1, 1500.0 * 0.001);
    // Level with flag 0x80: below 0.01 gives 10, else times 1000.
    call(&mut e, 0x268a, &[n(0), z(0.005), r(1)]);
    assert_eq!(e.mem.u16(NPC + 0x3c), 10);
    call(&mut e, 0x268a, &[n(0), z(1.25), r(1)]);
    assert_eq!(e.mem.u16(NPC + 0x3c), 1250);
    e.mem.set_u32(NPC + 0x34, 0);
    call(&mut e, 0x268a, &[n(0), z(0.5), r(1)]);
    assert_eq!(e.mem.u16(NPC + 0x3c), 1, "at least 1");
    call(&mut e, 0x268a, &[n(0), z(7.9), r(1)]);
    assert_eq!(e.mem.u16(NPC + 0x3c), 7, "toward zero");
    // Calc min raises calc max; calc max lowers calc min.
    call(&mut e, 0x268a, &[n(1), z(50.0), r(1)]);
    assert_eq!((e.mem.u16(NPC + 0x3e), e.mem.u16(NPC + 0x40)), (50, 50));
    call(&mut e, 0x268a, &[n(1), z(30.0), r(1)]);
    assert_eq!((e.mem.u16(NPC + 0x3e), e.mem.u16(NPC + 0x40)), (30, 50));
    call_on(&mut e, 0x268a, ACTOR, &[n(2), z(20.0)]);
    assert_eq!((e.mem.u16(NPC + 0x3e), e.mem.u16(NPC + 0x40)), (20, 20));
    call(&mut e, 0x268a, &[n(2), z(25.0), r(1)]);
    assert_eq!((e.mem.u16(NPC + 0x3e), e.mem.u16(NPC + 0x40)), (20, 25));
    call(&mut e, 0x268a, &[n(2), z(-5.0), r(1)]);
    assert_eq!((e.mem.u16(NPC + 0x3e), e.mem.u16(NPC + 0x40)), (1, 1));
    call(&mut e, 0x268a, &[n(3), z(9.0), r(1)]);
    assert_eq!(
        (e.mem.u16(NPC + 0x3e), e.mem.u16(NPC + 0x40)),
        (1, 1),
        "above 2"
    );
    assert_eq!(at_least_one(f32::NAN), 1.0);
}

/// `count` texts in heap blocks as a creature's model list (first node inline).
fn models(e: &mut Engine, texts: &[&[u8]]) {
    let mut at = CREA + 0x118;
    for (i, t) in texts.iter().enumerate() {
        let text = TEXTS + 0x100 * i as u32;
        e.mem.write(text, t);
        e.mem.set_u8(text + t.len() as u32, 0);
        let next = if i + 1 < texts.len() {
            NODES + 0x800 + 8 * i as u32
        } else {
            0
        };
        node(e, at, text, next);
        at = next;
    }
}

#[test]
fn creature_models() {
    let mut e = setup();
    assert_eq!(
        call(&mut e, 0x26ec, &[r(2)]).1,
        0.0,
        "an empty list: no array"
    );
    models(&mut e, &[b"a\\one.nif", b"B\\Two.NIF"]);
    let id = call_on(&mut e, 0x26ec, CR_ACTOR, &[]).1;
    assert_eq!(
        array_data(&e, id),
        vec![
            Data::Str(b"a\\one.nif".to_vec()),
            Data::Str(b"B\\Two.NIF".to_vec())
        ]
    );
    assert_eq!(call(&mut e, 0x26ec, &[r(1)]).1, 0.0, "an NPC");
    assert_eq!(call(&mut e, 0x26eb, &[s(b"b\\two.nif"), r(2)]).1, 1.0);
    assert_eq!(call(&mut e, 0x26eb, &[s(b"c.nif"), r(2)]).1, 0.0);
    assert_eq!(call(&mut e, 0x26eb, &[s(b"b\\two.nif"), r(1)]).1, 0.0);
    // Enabling one that is there: `return action <= 0` answers false; nothing added.
    assert_eq!(call(&mut e, 0x26ea, &[s(b"A\\ONE.NIF"), n(1), r(2)]).1, 0.0);
    assert!(allocations().is_empty());
    // Enabling a new one: a copy put first (its block, then the node for the old first).
    let copy = heap_next(&e);
    assert_eq!(
        call_on(&mut e, 0x26ea, CR_ACTOR, &[s(b"New.nif"), n(1)]).1,
        1.0
    );
    assert_eq!(allocations(), vec![8, 8]);
    assert_eq!(e.mem.u32(CREA + 0x118), copy);
    assert_eq!(text_at(&e, CREA + 0x118), b"New.nif");
    assert_eq!(
        e.mem.u32(e.mem.u32(CREA + 0x11c)),
        TEXTS,
        "the old first moved"
    );
    // Disabling: the text freed and the node unlinked; again: false.
    assert_eq!(call(&mut e, 0x26ea, &[s(b"b\\two.nif"), n(0), r(2)]).1, 1.0);
    assert_eq!(freed(), vec![TEXTS + 0x100, NODES + 0x800]);
    assert_eq!(call(&mut e, 0x26ea, &[s(b"b\\two.nif"), n(0), r(2)]).1, 0.0);
    assert_eq!(call(&mut e, 0x26eb, &[s(b"b\\two.nif"), r(2)]).1, 0.0);
    assert_eq!(
        call(&mut e, 0x26ea, &[s(b"x.nif"), n(1), r(1)]).1,
        0.0,
        "an NPC"
    );
}

#[test]
fn base_actor_values() {
    let mut e = setup();
    call(&mut e, 0x2723, &[short_arg(12), z(2.5), r(1)]);
    call_on(&mut e, 0x2723, ACTOR, &[short_arg(3), z(-1.0)]);
    call_on(&mut e, 0x2723, PLAIN_REF, &[short_arg(3), z(-1.0)]);
    assert_eq!(
        seen(F_MOD_AV),
        vec![
            vec![NPC, 12, 2.5f32.to_bits()],
            vec![NPC, 3, (-1.0f32).to_bits()]
        ]
    );
    // The leveled base is not used: the caller's base form itself.
    e.mem.set_u32(NPC + 0xc, 0xff00_0100);
    put_extra(&mut e, ACTOR + 0x44, X1, 0x2e, 0);
    e.mem.set_u32(X1 + 0x10, CREA);
    call_on(&mut e, 0x2723, ACTOR, &[short_arg(4), z(1.0)]);
    assert_eq!(seen(F_MOD_AV)[2][0], NPC);
    // An actor base: its actor value (a float); the caller: its base value, a whole number.
    assert_eq!(call(&mut e, 0x27a8, &[short_arg(5), r(1)]).1, 5.75);
    assert_eq!(call_on(&mut e, 0x27a8, ACTOR, &[short_arg(5)]).1, -5.0);
    assert_eq!(call_on(&mut e, 0x27a8, PLAIN_REF, &[short_arg(5)]).1, 0.0);
}

#[test]
fn processes() {
    let mut e = setup();
    assert_eq!(call_on(&mut e, 0x2734, ACTOR, &[]).1, 0.0);
    e.mem.set_u32(PROC + 0x28, 3);
    assert_eq!(call_on(&mut e, 0x2734, ACTOR, &[]).1, 3.0);
    assert_eq!(call_on(&mut e, 0x2734, CR_ACTOR, &[]).1, -1.0, "no process");
    assert_eq!(call_on(&mut e, 0x2734, PLAIN_REF, &[]).1, -1.0);
    // Light: a high process only.
    e.mem.set_f32(PROC + 0x3c4, 0.375);
    assert_eq!(call_on(&mut e, 0x273b, ACTOR, &[]).1, 0.0, "level 3");
    assert_eq!(eval(&mut e, 0x273b, ACTOR), 0.0);
    e.mem.set_u32(PROC + 0x28, 0);
    assert_eq!(call_on(&mut e, 0x273b, ACTOR, &[]).1, 0.375);
    assert_eq!(eval(&mut e, 0x273b, ACTOR), 0.375);
    assert_eq!(eval(&mut e, 0x273b, CR_ACTOR), 0.0);
    // Alpha and dive breath through the process's virtual functions.
    e.mem.set_f32(PROC + 0x404, 0.5);
    e.mem.set_f32(PROC + 0x400, 3.25);
    assert_eq!(call_on(&mut e, 0x273c, ACTOR, &[]).1, 0.5);
    assert_eq!(call_on(&mut e, 0x273d, ACTOR, &[]).1, 3.25);
    assert_eq!(call_on(&mut e, 0x273c, CR_ACTOR, &[]).1, 0.0);
    assert_eq!(call_on(&mut e, 0x273d, CR_ACTOR, &[]).1, 0.0);
    call_on(&mut e, 0x273e, ACTOR, &[z(4.5)]);
    call_on(&mut e, 0x273e, ACTOR, &[z(-2.0)]);
    call_on(&mut e, 0x273e, CR_ACTOR, &[z(1.0)]);
    assert_eq!(
        seen(F_SET_BREATH),
        vec![vec![PROC, 4.5f32.to_bits()], vec![PROC, 0]]
    );
    // Stop idle and holster.
    call_on(&mut e, 0x2741, ACTOR, &[]);
    call_on(&mut e, 0x2741, CR_ACTOR, &[]);
    assert_eq!(seen(F_STOP_IDLE), vec![vec![PROC, ACTOR]]);
    call_on(&mut e, 0x276b, ACTOR, &[]);
    assert!(seen(F_SET_WEAPON_OUT).is_empty(), "the weapon is not out");
    e.mem.set_u8(PROC + 0x40c, 1);
    call_on(&mut e, 0x276b, ACTOR, &[]);
    call_on(&mut e, 0x276b, CR_ACTOR, &[]);
    assert_eq!(seen(F_SET_WEAPON_OUT), vec![vec![PROC, ACTOR, 0]]);
}

#[test]
fn attacking_ragdolled_and_in_water() {
    let mut e = setup();
    for (action, yes) in [(1u16, 0.0), (2, 1.0), (5, 1.0), (6, 0.0), (0xffff, 0.0)] {
        e.mem.set_u16(PROC + 0x408, action);
        assert_eq!(call_on(&mut e, 0x2769, ACTOR, &[]).1, yes, "{action}");
        assert_eq!(eval(&mut e, 0x2769, ACTOR), yes);
    }
    assert_eq!(eval(&mut e, 0x2769, CR_ACTOR), 0.0, "no process");
    // Knocked state 1 on a process of level 0 or 1.
    e.mem.set_u8(PROC + 0x13c, 1);
    assert_eq!(call_on(&mut e, 0x27ad, ACTOR, &[]).1, 1.0);
    e.mem.set_u32(PROC + 0x28, 1);
    assert_eq!(eval(&mut e, 0x27ad, ACTOR), 1.0);
    e.mem.set_u32(PROC + 0x28, 2);
    assert_eq!(eval(&mut e, 0x27ad, ACTOR), 0.0, "level 2");
    e.mem.set_u32(PROC + 0x28, 0);
    e.mem.set_u8(PROC + 0x13c, 2);
    assert_eq!(eval(&mut e, 0x27ad, ACTOR), 0.0);
    assert_eq!(eval(&mut e, 0x27ad, 0), 0.0);
    e.mem.set_u8(ACTOR + 0x14c, 1);
    assert_eq!(call_on(&mut e, 0x277d, ACTOR, &[]).1, 1.0);
    assert_eq!(eval(&mut e, 0x277d, ACTOR), 1.0);
    assert_eq!(eval(&mut e, 0x277d, CR_ACTOR), 0.0);
    e.mem.set_u8(PLAIN_REF + 0x14c, 1);
    assert_eq!(eval(&mut e, 0x277d, PLAIN_REF), 0.0, "not an actor");
}

#[test]
fn actors_by_processing_level() {
    let mut e = setup();
    // Objects at BLOCK: [ACTOR, PLAIN_REF, 0, OTHER]; level 1 is 1..4, level 2 is empty.
    for (i, f) in [ACTOR, PLAIN_REF, 0, OTHER].iter().enumerate() {
        e.mem.set_u32(BLOCK + 4 * i as u32, *f);
    }
    e.mem.set_u32(PM + 8, BLOCK);
    e.mem.set_u32(PM + 0x14 + 4, 0);
    e.mem.set_u32(PM + 0x24 + 4, 4);
    e.mem.set_u32(PM + 0x14 + 8, 4);
    e.mem.set_u32(PM + 0x24 + 8, 4);
    let id = call(&mut e, 0x2735, &[n(1)]).1;
    assert_eq!(forms_of(&e, id), vec![0x200, 0x202]);
    assert_eq!(call(&mut e, 0x2735, &[n(2)]).1, 0.0, "none: no array");
    e.mem.set_u32(PM + 0x14 + 16, 0);
    e.mem.set_u32(PM + 0x24 + 16, 4);
    assert_eq!(call(&mut e, 0x2735, &[n(4)]).1, 0.0, "above 3");
}

#[test]
fn combat_targets_and_allies() {
    let mut e = setup();
    // An actor's arrays (data +4, size +8), itself left out.
    e.mem.set_u32(BLOCK, 0);
    e.mem.set_u32(BLOCK + 4, BLOCK + 0x10);
    e.mem.set_u32(BLOCK + 8, 3);
    for (i, f) in [OTHER, ACTOR, 0].iter().enumerate() {
        e.mem.set_u32(BLOCK + 0x10 + 4 * i as u32, *f);
    }
    assert_eq!(call_on(&mut e, 0x2742, ACTOR, &[]).1, 0.0, "no array");
    e.mem.set_u32(ACTOR + 0x12c, BLOCK);
    let id = call_on(&mut e, 0x2742, ACTOR, &[]).1;
    assert_eq!(forms_of(&e, id), vec![0x202]);
    e.mem.set_u32(ACTOR + 0x130, BLOCK);
    let id = call_on(&mut e, 0x2743, ACTOR, &[]).1;
    assert_eq!(forms_of(&e, id), vec![0x202]);
    assert_eq!(call_on(&mut e, 0x2743, PLAIN_REF, &[]).1, 0.0);
    assert_eq!(call_on(&mut e, 0x2743, 0, &[]).1, 0.0);
    // In combat with: the target array.
    assert_eq!(call_on(&mut e, 0x2761, ACTOR, &[r(6)]).1, 1.0);
    set_ref(&mut e, 11, OTHER2);
    assert_eq!(call_on(&mut e, 0x2761, ACTOR, &[r(11)]).1, 0.0);
    assert_eq!(
        call_on(&mut e, 0x2761, CR_ACTOR, &[r(6)]).1,
        0.0,
        "no array"
    );
    // The player: its combat group's targets (0x68 bytes each, from +0xC, count +0x10) and
    // members (0x14 bytes each, from +0x1C, count +0x20), itself left out of the members only.
    assert_eq!(call_on(&mut e, 0x2742, PLAYER_REF, &[]).1, 0.0, "no group");
    e.mem.set_u32(PLAYER_REF + 0xd64, GROUP);
    e.mem.set_u32(GROUP + 0xc, BLOCK + 0x100);
    e.mem.set_u32(GROUP + 0x10, 2);
    e.mem.set_u32(BLOCK + 0x100, OTHER);
    e.mem.set_u32(BLOCK + 0x168, OTHER2);
    e.mem.set_u32(BLOCK + 0x104, ACTOR);
    e.mem.set_u32(GROUP + 0x1c, BLOCK + 0x200);
    e.mem.set_u32(GROUP + 0x20, 3);
    e.mem.set_u32(BLOCK + 0x200, PLAYER_REF);
    e.mem.set_u32(BLOCK + 0x214, 0);
    e.mem.set_u32(BLOCK + 0x228, ACTOR);
    let id = call_on(&mut e, 0x2742, PLAYER_REF, &[]).1;
    assert_eq!(forms_of(&e, id), vec![0x202, 0x203]);
    let id = call_on(&mut e, 0x2743, PLAYER_REF, &[]).1;
    assert_eq!(forms_of(&e, id), vec![0x200]);
    assert_eq!(call_on(&mut e, 0x2761, PLAYER_REF, &[r(11)]).1, 1.0);
    set_ref(&mut e, 12, ACTOR);
    assert_eq!(call_on(&mut e, 0x2761, PLAYER_REF, &[r(12)]).1, 0.0);
}

fn detection(e: &mut Engine, process: u32, target: u32, at: u32, value: i32, los: bool) {
    e.mem.set_u32(at, target);
    e.mem.set_u32(at + 8, value as u32);
    e.mem.set_u8(at + 0x1e, u8::from(los));
    DETECTION.with(|d| d.borrow_mut().push((process, target, at)));
}

#[test]
fn detection_values_and_actors() {
    let mut e = setup();
    assert_eq!(call_on(&mut e, 0x27a7, ACTOR, &[r(6)]).1, -100.0, "no data");
    detection(&mut e, PROC, OTHER, BLOCK, 55, false);
    assert_eq!(call_on(&mut e, 0x27a7, ACTOR, &[r(6)]).1, 55.0);
    e.mem.set_u32(PROC + 0x28, 1);
    assert_eq!(
        call_on(&mut e, 0x27a7, ACTOR, &[r(6)]).1,
        -100.0,
        "not high"
    );
    e.mem.set_u32(PROC + 0x28, 0);
    assert_eq!(call_on(&mut e, 0x27a7, CR_ACTOR, &[r(6)]).1, -100.0);
    assert_eq!(call_on(&mut e, 0x27a7, PLAIN_REF, &[r(6)]).1, 0.0);
    // An actor: its lists of detection data (value above 0, actor alive).
    detection(&mut e, PROC, OTHER2, BLOCK + 0x40, 0, false);
    detection(&mut e, PROC, CR_ACTOR, BLOCK + 0x80, 9, false);
    node(&mut e, NODES + 0x100, BLOCK, NODES + 0x108);
    node(&mut e, NODES + 0x108, BLOCK + 0x40, NODES + 0x110);
    node(&mut e, NODES + 0x110, BLOCK + 0x80, NODES + 0x118);
    node(&mut e, NODES + 0x118, 0, 0);
    e.mem.set_u32(PROC + 0x25c, NODES + 0x100);
    let id = call_on(&mut e, 0x2744, ACTOR, &[]).1;
    assert_eq!(forms_of(&e, id), vec![0x202, 0x201]);
    e.mem.set_u32(CR_ACTOR + 0x108, 2);
    let id = call_on(&mut e, 0x2744, ACTOR, &[]).1;
    assert_eq!(forms_of(&e, id), vec![0x202], "dead");
    e.mem.set_u32(OTHER + 0x108, 1);
    assert_eq!(call_on(&mut e, 0x2744, ACTOR, &[]).1, 0.0, "dying");
    e.mem.set_u32(OTHER + 0x108, 0);
    assert_eq!(
        call_on(&mut e, 0x2745, ACTOR, &[]).1,
        0.0,
        "no detecting list"
    );
    e.mem.set_u32(PROC + 0x260, NODES + 0x100);
    let id = call_on(&mut e, 0x2745, ACTOR, &[]).1;
    assert_eq!(forms_of(&e, id), vec![0x202]);
    e.mem.set_u32(PROC + 0x28, 1);
    assert_eq!(call_on(&mut e, 0x2745, ACTOR, &[]).1, 0.0, "not high");
    e.mem.set_u32(PROC + 0x28, 0);
    // The player: the high-level actors; detecting: their value of the player above 0;
    // detected: the player's line of sight.
    for (i, f) in [ACTOR, PLAIN_REF, OTHER, CR_ACTOR, 0].iter().enumerate() {
        e.mem.set_u32(BLOCK + 0x400 + 4 * i as u32, *f);
    }
    e.mem.set_u32(PM + 8, BLOCK + 0x400);
    e.mem.set_u32(PM + 0x14, 0);
    e.mem.set_u32(PM + 0x24, 5);
    e.mem.set_u32(CR_ACTOR + 0x108, 0);
    detection(&mut e, PROC, PLAYER_REF, BLOCK + 0xc0, 1, false);
    let id = call_on(&mut e, 0x2745, PLAYER_REF, &[]).1;
    assert_eq!(forms_of(&e, id), vec![0x200]);
    detection(&mut e, PROC2, OTHER, BLOCK + 0x100, 0, true);
    detection(&mut e, PROC2, ACTOR, BLOCK + 0x140, 0, false);
    let id = call_on(&mut e, 0x2744, PLAYER_REF, &[]).1;
    assert_eq!(forms_of(&e, id), vec![0x202]);
    e.mem.set_u32(PROC2 + 0x28, 1);
    assert_eq!(
        call_on(&mut e, 0x2744, PLAYER_REF, &[]).1,
        0.0,
        "no LOS off high"
    );
}

#[test]
fn the_stable_package() {
    let mut e = setup();
    let (_, lines) = echoed(&mut e, 0x2757, ACTOR, &[]);
    assert_eq!(form_result(&e), 0);
    assert_eq!(lines.len(), 1, "echoed even without one");
    e.mem.set_u32(PROC + 4, PACK);
    for (t, stable) in [
        (17u8, true),
        (18, false),
        (26, true),
        (30, true),
        (29, false),
    ] {
        e.mem.set_u8(PACK + 0x20, t);
        e.mem.set_u32(RESULT, 0);
        call_on(&mut e, 0x2757, ACTOR, &[]);
        assert_eq!(form_result(&e), if stable { 0x109 } else { 0 }, "{t}");
    }
    // Not stable: the reference's ExtraPackage.
    put_extra(&mut e, ACTOR + 0x44, X1, 0x19, MISC);
    call_on(&mut e, 0x2757, ACTOR, &[]);
    assert_eq!(form_result(&e), 0x10b);
    e.mem.set_u32(RESULT, 0);
    call_on(&mut e, 0x2757, PLAIN_REF, &[]);
    call_on(&mut e, 0x2757, CR_ACTOR, &[]);
    assert_eq!(form_result(&e), 0);
}

#[test]
fn followers_dropped_items_and_ash_piles() {
    let mut e = setup();
    assert_eq!(call_on(&mut e, 0x2637, ACTOR, &[]).1, 0.0);
    put_extra(&mut e, ACTOR + 0x44, X1, 0x1d, 0);
    assert_eq!(call_on(&mut e, 0x2637, ACTOR, &[]).1, 0.0, "no list");
    node(&mut e, NODES, OTHER, NODES + 8);
    node(&mut e, NODES + 8, 0, NODES + 0x10);
    node(&mut e, NODES + 0x10, OTHER2, 0);
    e.mem.set_u32(X1 + 0xc, NODES);
    let id = call_on(&mut e, 0x2637, ACTOR, &[]).1;
    assert_eq!(forms_of(&e, id), vec![0x202, 0x203]);
    e.mem.set_u32(NODES, 0);
    e.mem.set_u32(NODES + 0x10, 0);
    assert_eq!(
        call_on(&mut e, 0x2637, ACTOR, &[]).1,
        0.0,
        "only empty items"
    );
    put_extra(&mut e, PLAIN_REF + 0x44, X2, 0x1d, NODES + 0x20);
    assert_eq!(
        call_on(&mut e, 0x2637, PLAIN_REF, &[]).1,
        0.0,
        "not an actor"
    );
    // Dropped items: the list inline at +0xC.
    put_extra(&mut e, CR_ACTOR + 0x44, BLOCK, 0x3a, OTHER);
    node(&mut e, BLOCK + 0xc, OTHER, NODES + 0x30);
    node(&mut e, NODES + 0x30, ACTOR, 0);
    let id = call_on(&mut e, 0x2780, CR_ACTOR, &[]).1;
    assert_eq!(forms_of(&e, id), vec![0x202, 0x200]);
    assert_eq!(call_on(&mut e, 0x2780, ACTOR, &[]).1, 0.0);
    // Ash piles: an activator reference's source.
    put_extra(&mut e, PLAIN_REF + 0x44, BLOCK + 0x40, 0x89, OTHER);
    call_on(&mut e, 0x2786, PLAIN_REF, &[]);
    assert_eq!(form_result(&e), 0x202);
    e.mem.set_u32(RESULT, 0);
    e.mem.set_u32(PLAIN_REF + 0x20, MISC);
    call_on(&mut e, 0x2786, PLAIN_REF, &[]);
    assert_eq!(form_result(&e), 0, "not an activator");
    e.mem.set_u32(PLAIN_REF + 0x20, ACTI);
    e.mem.set_u32(BLOCK + 0x4c, 0);
    call_on(&mut e, 0x2786, PLAIN_REF, &[]);
    call_on(&mut e, 0x2786, 0, &[]);
    assert_eq!(form_result(&e), 0, "no source");
}

#[test]
fn ammo() {
    let mut e = setup();
    assert_eq!(call_on(&mut e, 0x27a2, ACTOR, &[]).1, 0.0);
    e.mem.set_u32(PROC + 0x118, BLOCK);
    e.mem.set_u32(BLOCK + 4, (-3i32) as u32);
    assert_eq!(call_on(&mut e, 0x27a2, ACTOR, &[]).1, -3.0);
    call_on(&mut e, 0x27a1, ACTOR, &[]);
    assert_eq!(form_result(&e), 0, "no form");
    e.mem.set_u32(BLOCK + 8, AMMO);
    call_on(&mut e, 0x27a1, ACTOR, &[]);
    assert_eq!(form_result(&e), 0x10a);
    e.mem.set_u32(PROC + 0x28, 1);
    assert_eq!(call_on(&mut e, 0x27a2, ACTOR, &[]).1, -3.0, "middle-high");
    e.mem.set_u32(PROC + 0x28, 2);
    e.mem.set_u32(RESULT, 0);
    assert_eq!(call_on(&mut e, 0x27a2, ACTOR, &[]).1, 0.0, "middle-low");
    call_on(&mut e, 0x27a1, ACTOR, &[]);
    assert_eq!(form_result(&e), 0);
}

#[test]
fn turning() {
    let mut e = setup();
    e.register(0x008b_b520, |_, w| {
        SEEN.with(|s| s.borrow_mut().push((0x008b_b520, w.to_vec())));
        ret(0)
    });
    e.register(0x008b_b5c0, |_, w| {
        SEEN.with(|s| s.borrow_mut().push((0x008b_b5c0, w.to_vec())));
        ret(0)
    });
    e.mem.set_f32(OTHER + 0x30, 1.0);
    e.mem.set_f32(OTHER + 0x34, 2.0);
    e.mem.set_f32(OTHER + 0x38, 3.0);
    call_on(&mut e, 0x2782, ACTOR, &[r(6)]);
    call_on(&mut e, 0x2782, PLAIN_REF, &[r(6)]);
    assert_eq!(
        seen(0x008b_b520),
        vec![vec![
            ACTOR,
            1.0f32.to_bits(),
            2.0f32.to_bits(),
            3.0f32.to_bits(),
            0
        ]]
    );
    e.mem.set_f32(ACTOR + 0x2c, 0.5);
    call_on(&mut e, 0x2783, ACTOR, &[z(90.0)]);
    call_on(&mut e, 0x2783, PLAIN_REF, &[z(90.0)]);
    let want = 90.0f32 * crate::commands::FLT_PI_D_180 + 0.5;
    assert_eq!(seen(0x008b_b5c0), vec![vec![ACTOR, want.to_bits(), 0]]);
}

#[test]
fn names_set_on_bases_and_cells() {
    let mut e = setup();
    string(&mut e, NPC + 0xd4, Some(b"Old"));
    call(&mut e, 0x27a5, &[s(b"Doc"), r(1)]);
    assert_eq!(text_at(&e, NPC + 0xd4), b"Doc");
    assert_eq!(calls_to(ADD_CHANGE_FN), vec![vec![SAVE_LOAD, NPC, 0x20, 0]]);
    // The calling reference's base form.
    string(&mut e, CREA + 0xd4, None);
    call_on(&mut e, 0x27a5, CR_ACTOR, &[s(b"Gecko")]);
    assert_eq!(text_at(&e, CREA + 0xd4), b"Gecko");
    // A talking activator's full name component (+0x30) and a cell's (+0x18): flag 4.
    string(&mut e, TACT + 0x34, None);
    call(&mut e, 0x27a5, &[s(b"Radio"), r(9)]);
    assert_eq!(text_at(&e, TACT + 0x34), b"Radio");
    string(&mut e, CELL + 0x1c, None);
    call(&mut e, 0x27a5, &[s(b"Room"), r(10)]);
    assert_eq!(text_at(&e, CELL + 0x1c), b"Room");
    let changes = calls_to(ADD_CHANGE_FN);
    assert_eq!(changes[2], vec![SAVE_LOAD, TACT, 4, 0]);
    assert_eq!(changes[3], vec![SAVE_LOAD, CELL, 4, 0]);
    // Another type (an activator, the plain reference's base): nothing.
    string(&mut e, ACTI + 0x34, Some(b"Keep"));
    call(&mut e, 0x27a5, &[s(b"No"), r(8)]);
    call_on(&mut e, 0x27a5, PLAIN_REF, &[s(b"No")]);
    call_on(&mut e, 0x27a5, 0, &[s(b"No")]);
    assert_eq!(text_at(&e, ACTI + 0x34), b"Keep");
    assert_eq!(calls_to(ADD_CHANGE_FN).len(), 4);
}

#[test]
fn speed_mult() {
    let mut e = setup();
    e.mem.set_u32(PROC + 0x2c, CACHED);
    e.mem.set_u32(CACHED + 0x44, 0x3105);
    call_on(&mut e, 0x27ac, ACTOR, &[n(120)]);
    assert_eq!(seen(F_SET_AV_INT), vec![vec![ACTOR, 0x15, 120]]);
    assert_eq!(e.mem.u32(CACHED + 0x44), 0x105);
    // No multiplier: only the cached speeds dropped.
    e.mem.set_u32(CACHED + 0x44, 0x3000);
    call_on(&mut e, 0x27ac, ACTOR, &[]);
    assert_eq!(seen(F_SET_AV_INT).len(), 1);
    assert_eq!(e.mem.u32(CACHED + 0x44), 0);
    call_on(&mut e, 0x27ac, CR_ACTOR, &[n(5)]);
    call_on(&mut e, 0x27ac, PLAIN_REF, &[n(5)]);
    assert_eq!(
        seen(F_SET_AV_INT).len(),
        2,
        "no process: the value still set"
    );
}

/// An active effect at `at` (active, an effect item at +0x100 of it with base effect `mgef`).
fn effect(e: &mut Engine, at: u32, mgef: u32, magnitude: f32, duration: f32) {
    e.mem.set_u8(at + 0x10, 1);
    e.mem.set_u32(at + 0xc, at + 0x100);
    e.mem.set_u32(at + 0x100 + 0x14, mgef);
    e.mem.set_f32(at + 0x1c, magnitude);
    e.mem.set_f32(at + 0x20, duration);
}

#[test]
fn poisoned_and_actor_value_modifiers() {
    let mut e = setup();
    let (fx1, fx2, fx3) = (BLOCK, BLOCK + 0x200, BLOCK + 0x400);
    let (m1, m2) = (BLOCK + 0x800, BLOCK + 0x900);
    node(&mut e, NODES, 0, NODES + 8);
    node(&mut e, NODES + 8, fx1, NODES + 0x10);
    node(&mut e, NODES + 0x10, fx2, NODES + 0x18);
    node(&mut e, NODES + 0x18, fx3, 0);
    assert_eq!(eval(&mut e, 0x2605, ACTOR), 0.0);
    e.mem.set_u32(fx2 + 0x2c, 5);
    assert_eq!(call_on(&mut e, 0x2605, ACTOR, &[]).1, 1.0);
    assert_eq!(eval(&mut e, 0x2605, ACTOR), 1.0);
    assert_eq!(eval(&mut e, 0x2605, OTHER), 0.0, "no list");
    assert_eq!(eval(&mut e, 0x2605, PLAIN_REF), 0.0);
    e.mem.set_u32(fx2 + 0x2c, 4);
    assert_eq!(eval(&mut e, 0x2605, ACTOR), 0.0);
    // Modifiers: archetype 0, flag 2, the actor value; duration bits.
    for m in [m1, m2] {
        e.mem.set_u32(m + 0x58, 2);
        e.mem.set_u8(m + 0x9c, 14);
    }
    effect(&mut e, fx1, m1, 1.5, 0.0);
    effect(&mut e, fx2, m1, 2.25, 10.0);
    effect(&mut e, fx3, m2, 4.0, 0.0);
    assert_eq!(call_on(&mut e, 0x27f5, ACTOR, &[short_arg(14)]).1, 7.75);
    assert_eq!(
        call_on(&mut e, 0x27f5, ACTOR, &[short_arg(14), n(1)]).1,
        5.5
    );
    assert_eq!(
        call_on(&mut e, 0x27f5, ACTOR, &[short_arg(14), n(2)]).1,
        2.25
    );
    assert_eq!(call_on(&mut e, 0x27f5, ACTOR, &[short_arg(15)]).1, 0.0);
    e.mem.set_u8(m2 + 0x98, 1);
    assert_eq!(
        call_on(&mut e, 0x27f5, ACTOR, &[short_arg(14)]).1,
        3.75,
        "archetype"
    );
    e.mem.set_u8(m2 + 0x98, 0);
    e.mem.set_u32(m2 + 0x58, 0x5);
    assert_eq!(
        call_on(&mut e, 0x27f5, ACTOR, &[short_arg(14)]).1,
        3.75,
        "no flag 2"
    );
    e.mem.set_u32(m2 + 0x58, 2);
    e.mem.set_u8(fx3 + 0x13, 1);
    assert_eq!(
        call_on(&mut e, 0x27f5, ACTOR, &[short_arg(14)]).1,
        3.75,
        "terminated"
    );
    e.mem.set_u8(fx3 + 0x13, 0);
    e.mem.set_u8(fx3 + 0x10, 0);
    assert_eq!(
        call_on(&mut e, 0x27f5, ACTOR, &[short_arg(14)]).1,
        3.75,
        "inactive"
    );
    e.mem.set_u8(fx3 + 0x10, 1);
    e.mem.set_u32(fx3 + 0x100 + 0x14, 0);
    assert_eq!(
        call_on(&mut e, 0x27f5, ACTOR, &[short_arg(14)]).1,
        3.75,
        "no setting"
    );
    e.mem.set_u32(fx3 + 0xc, 0);
    assert_eq!(
        call_on(&mut e, 0x27f5, ACTOR, &[short_arg(14)]).1,
        3.75,
        "no item"
    );
    e.mem.set_f64(RESULT, 9.0);
    call_on(&mut e, 0x27f5, OTHER, &[short_arg(14)]);
    assert_eq!(e.mem.f64(RESULT), 0.0, "no list: the result left as zeroed");
    let (v, lines) = echoed(&mut e, 0x27f5, ACTOR, &[short_arg(14)]);
    assert_eq!((v, lines.len()), (3.75, 1), "echoed");
}

#[test]
fn perk_modifiers() {
    let mut e = setup();
    e.register(0x005e_58f0, |e, w| {
        SEEN.with(|s| s.borrow_mut().push((0x005e_58f0, w.to_vec())));
        // The value pointer is the last word given: the stand-in doubles it.
        let n = match e.mem.u8(0x0119_6ee0 + 0x10 * w[0] + 4) {
            1 => 2,
            2 => 3,
            _ => 4,
        };
        let v = w[n];
        e.mem.set_f32(v, e.mem.f32(v) * 2.0);
        ret(0)
    });
    for (id, tabs) in [(0u32, 1u8), (5, 2), (9, 3), (12, 0)] {
        e.mem.set_u8(0x0119_6ee0 + 0x10 * id + 4, tabs);
    }
    assert_eq!(call_on(&mut e, 0x27f6, ACTOR, &[n(0), z(1.5)]).1, 3.0);
    assert_eq!(call_on(&mut e, 0x27f6, ACTOR, &[n(5), z(1.5), r(5)]).1, 3.0);
    assert_eq!(
        call_on(&mut e, 0x27f6, ACTOR, &[n(9), z(1.5), r(5), r(6)]).1,
        3.0
    );
    assert_eq!(
        call_on(&mut e, 0x27f6, ACTOR, &[n(12), z(1.5)]).1,
        1.5,
        "no call"
    );
    let c = seen(0x005e_58f0);
    assert_eq!(c.len(), 3);
    assert_eq!(&c[0][..2], &[0, ACTOR]);
    assert_eq!(&c[1][..3], &[5, ACTOR, ITEM1]);
    assert_eq!(&c[2][..4], &[9, ACTOR, ITEM1, OTHER]);
    assert_eq!(
        call_on(&mut e, 0x27f6, ACTOR, &[n(0x4a), z(1.5)]).1,
        0.0,
        "past the max"
    );
    assert_eq!(call_on(&mut e, 0x27f6, PLAIN_REF, &[n(0), z(1.5)]).1, 0.0);
    let (v, lines) = echoed(&mut e, 0x27f6, ACTOR, &[n(12), z(2.0)]);
    assert_eq!((v, lines.len()), (2.0, 1));
}

#[test]
fn hit_health_damage() {
    let mut e = setup();
    assert_eq!(call_on(&mut e, 0x2802, ACTOR, &[]).1, 0.0, "no hit");
    // Rows of settings: the player's at 0119B310, the rest at +0x14; difficulty 2.
    let (s_pc, s_npc) = (BLOCK + 0x100, BLOCK + 0x200);
    e.mem.set_f32(s_pc + 4, 0.5);
    e.mem.set_f32(s_npc + 4, 1.75);
    e.mem.set_u32(0x0119_b310 + 8, s_pc);
    e.mem.set_u32(0x0119_b324 + 8, s_npc);
    e.mem.set_u32(PLAYER_REF + 0x7b8, 2);
    e.mem.set_u32(PROC + 0x240, BLOCK);
    e.mem.set_f32(BLOCK + 0x14, 10.1);
    e.mem.set_u32(BLOCK + 4, OTHER);
    assert_eq!(
        call_on(&mut e, 0x2802, ACTOR, &[]).1,
        f64::from(10.1f32) * 1.75
    );
    e.mem.set_u32(BLOCK + 4, PLAYER_REF);
    assert_eq!(
        call_on(&mut e, 0x2802, ACTOR, &[]).1,
        f64::from(10.1f32) * 0.5
    );
    e.mem.set_u32(PROC + 0x28, 1);
    assert_eq!(
        call_on(&mut e, 0x2802, ACTOR, &[]).1,
        f64::from(10.1f32) * 0.5
    );
    e.mem.set_u32(PROC + 0x28, 2);
    assert_eq!(call_on(&mut e, 0x2802, ACTOR, &[]).1, 0.0, "middle-low");
    assert_eq!(call_on(&mut e, 0x2802, CR_ACTOR, &[]).1, 0.0);
    assert_eq!(call_on(&mut e, 0x2802, 0, &[]).1, 0.0);
}

#[test]
fn pick_from_a_list() {
    let mut e = setup();
    // The list: [ITEM1, MISC, ITEM2]; the base container holds ITEM2 (and MISC); the changes
    // take MISC to 0.
    node(&mut e, LIST + 0x18, ITEM1, NODES);
    node(&mut e, NODES, MISC, NODES + 8);
    node(&mut e, NODES + 8, ITEM2, 0);
    e.mem.set_u32(NPC, BASE_VT);
    e.mem.set_u32(BASE_VT + 0xf8, 0x008d_0360);
    let (o1, o2) = (BLOCK, BLOCK + 0x10);
    e.mem.set_u32(o1, 1);
    e.mem.set_u32(o1 + 4, ITEM2);
    e.mem.set_u32(o2, 2);
    e.mem.set_u32(o2 + 4, MISC);
    node(&mut e, NPC + 0x64 + 4, o1, NODES + 0x20);
    node(&mut e, NODES + 0x20, o2, 0);
    call_on(&mut e, 0x26d9, ACTOR, &[r(4)]);
    assert_eq!(form_result(&e), 0, "no container changes");
    let (data, en) = (BLOCK + 0x100, BLOCK + 0x140);
    put_extra(&mut e, ACTOR + 0x44, X1, 0x15, data);
    e.mem.set_u32(data, NODES + 0x40);
    node(&mut e, NODES + 0x40, en, 0);
    e.mem.set_u32(en + 4, (-2i32) as u32);
    e.mem.set_u32(en + 8, MISC);
    call_on(&mut e, 0x26d9, ACTOR, &[r(4)]);
    assert_eq!(form_result(&e), 0x108, "ITEM2, the first held");
    e.mem.set_u32(RESULT, 0);
    call_on(&mut e, 0x26d9, ACTOR, &[r(4), n(0), n(2)]);
    assert_eq!(form_result(&e), 0, "the first two only");
    call_on(&mut e, 0x26d9, ACTOR, &[r(4), n(2), n(1)]);
    assert_eq!(form_result(&e), 0x108, "from the third");
    e.mem.set_u32(RESULT, 0);
    call_on(&mut e, 0x26d9, ACTOR, &[r(4), n(-1)]);
    assert_eq!(form_result(&e), 0, "a negative start");
    // An empty item ends the walk.
    e.mem.set_u32(NODES, 0);
    call_on(&mut e, 0x26d9, ACTOR, &[r(4)]);
    assert_eq!(form_result(&e), 0);
    e.mem.set_u32(NODES, MISC);
    // A base form without a container: nothing.
    e.mem.set_u32(BASE_VT + 0xf8, 0);
    call_on(&mut e, 0x26d9, ACTOR, &[r(4)]);
    call_on(&mut e, 0x26d9, 0, &[r(4)]);
    assert_eq!(form_result(&e), 0);
}
