//! Tests of `ln_fn_actor` on actor bases at JIP's header offsets (karma +0x44, template flags
//! +0x4A, template actor +0x54, factions +0x5C, services +0x9C, aggro +0xA3/+0xA4; creature type
//! +0x12C and body part data +0x144; NPC race +0x110), actors (vtable +0x100 true) with their
//! extras, the actor bases' virtual functions and the game's equip call as stand-ins.

use super::*;
use crate::commands::test_support::*;
use nvse::vars::{Data, Value as V};

const NPC: u32 = FORMS + 0x1000;
const CREA: u32 = FORMS + 0x1400;
const LVL_BASE: u32 = FORMS + 0x1800;
const ACTOR: u32 = FORMS + 0x2000;
const ACTOR2: u32 = FORMS + 0x2200;
const PLAIN_REF: u32 = FORMS + 0x2400;
const FACTION: u32 = FORMS + 0x2600;
const FACTION2: u32 = FORMS + 0x2700;
const RACE: u32 = FORMS + 0x2800;
const LIST: u32 = FORMS + 0x2900;
const CSTY: u32 = FORMS + 0x2a00;
const CSTY2: u32 = FORMS + 0x2b00;
const BPD: u32 = FORMS + 0x2c00;
const MGEF: u32 = FORMS + 0x2d00;
const WEAP: u32 = FORMS + 0x2e00;
const ARMO: u32 = FORMS + 0x2f00;
const MISC: u32 = FORMS + 0x3000;
const ACTOR_VT: u32 = FORMS + 0x4000;
const BASE_VT: u32 = FORMS + 0x4400;
const TARGET_VT: u32 = FORMS + 0x4800;
const X1: u32 = FORMS + 0x5000;
const X2: u32 = FORMS + 0x5100;
const X3: u32 = FORMS + 0x5200;
const NODES: u32 = FORMS + 0x6000;
const FX: u32 = FORMS + 0x6400;
const FX_ITEM: u32 = FORMS + 0x6500;
const DATA: u32 = FORMS + 0x7000;
const ENTRY_W: u32 = FORMS + 0x7100;
const ENTRY_A: u32 = FORMS + 0x7200;
const XDL1: u32 = FORMS + 0x7300;
const XDL2: u32 = FORMS + 0x7400;
const XH: u32 = FORMS + 0x7500;
const XM: u32 = FORMS + 0x7600;
const XW: u32 = FORMS + 0x7700;
const XDL3: u32 = FORMS + 0x7800;
const XW3: u32 = FORMS + 0x7900;
const CALL_BPD: u32 = 0x0090_0100;
const CALL_CSTY: u32 = 0x0090_0110;
const CALL_IS_CHAR: u32 = 0x0090_0120;
const CALL_EFFECTS: u32 = 0x0090_0130;

fn node(e: &mut Engine, at: u32, data: u32, next: u32) {
    e.mem.set_u32(at, data);
    e.mem.set_u32(at + 4, next);
}

fn setup() -> Engine {
    let mut e = engine();
    extra_stand_ins(&mut e);
    game_globals(&mut e);
    form(&mut e, NPC, 0x2a, 0x0000_0100);
    form(&mut e, CREA, 0x2b, 0x0000_0101);
    form(&mut e, LVL_BASE, 0x2a, 0x0000_0102);
    form(&mut e, FACTION, 0x08, 0x0000_0103);
    form(&mut e, FACTION2, 0x08, 0x0000_0104);
    form(&mut e, RACE, 0x0c, 0x0000_0105);
    form(&mut e, LIST, 0x55, 0x0000_0106);
    form(&mut e, CSTY, 0x4a, 0x0000_0107);
    form(&mut e, CSTY2, 0x4a, 0x0000_0108);
    form(&mut e, BPD, 0x57, 0x0000_0109);
    form(&mut e, MGEF, 0x10, 0x0000_010a);
    form(&mut e, WEAP, 0x28, 0x0000_010b);
    form(&mut e, ARMO, 0x18, 0x0000_010c);
    form(&mut e, MISC, 0x1f, 0x0000_010d);
    e.mem.set_u32(ARMO, TES_OBJECT_ARMO_VTBL);
    in_form_map(&mut e, WEAP, 0x10b, 0);
    for b in [NPC, CREA, LVL_BASE] {
        e.mem.set_u32(b, BASE_VT);
    }
    e.mem.set_u32(BASE_VT + 0x180, CALL_BPD);
    e.mem.set_u32(BASE_VT + 0x188, CALL_CSTY);
    e.register(CALL_BPD, |_, _| Ret {
        eax: BPD,
        ..Ret::default()
    });
    e.register(CALL_CSTY, |_, w| Ret {
        eax: if w[0] == NPC { CSTY } else { 0 },
        ..Ret::default()
    });
    // Actors: vtable +0x100 true, IsCharacter (+0x218) a stand-in answering true.
    e.mem.set_u32(ACTOR_VT + 0x100, 0x008d_0360);
    e.mem.set_u32(ACTOR_VT + 0x218, CALL_IS_CHAR);
    e.register(CALL_IS_CHAR, |_, _| Ret {
        eax: 1,
        ..Ret::default()
    });
    for (r, base, id) in [(ACTOR, NPC, 0x200), (ACTOR2, LVL_BASE, 0xff00_0201)] {
        form(&mut e, r, 0x3b, id);
        e.mem.set_u32(r, ACTOR_VT);
        e.mem.set_u32(r + 0x20, base);
    }
    form(&mut e, PLAIN_REF, 0x3a, 0x202);
    e.mem.set_u32(PLAIN_REF, ACTOR_VT + 0x300);
    e.mem.set_u32(PLAIN_REF + 0x20, MISC);
    for (i, f) in [NPC, CREA, FACTION, FACTION2, LIST, MISC, BPD, MGEF, WEAP]
        .iter()
        .enumerate()
    {
        set_ref(&mut e, i as u16 + 1, *f);
    }
    e
}

#[test]
fn the_actor_base_of_a_leveled_actor() {
    let mut e = setup();
    assert_eq!(actor_base(&mut e, ACTOR), NPC);
    // A base of file 0xFF: its ExtraLeveledCreature's actor base (+0x10).
    e.mem.set_u32(LVL_BASE + 0xc, 0xff00_0102);
    assert_eq!(actor_base(&mut e, ACTOR2), LVL_BASE, "no extra yet");
    put_extra(&mut e, ACTOR2 + 0x44, X1, 0x2e, 0);
    e.mem.set_u32(X1 + 0x10, CREA);
    assert_eq!(actor_base(&mut e, ACTOR2), CREA);
    e.mem.set_u32(X1 + 0x10, 0);
    assert_eq!(actor_base(&mut e, ACTOR2), LVL_BASE);
}

#[test]
fn aggro_radius() {
    let mut e = setup();
    e.mem.set_u32(NPC + 0xa4, 400);
    assert_eq!(call(&mut e, 0x2247, &[r(1)]).1, 0.0, "behaviour off");
    call(&mut e, 0x2248, &[n(250), r(1)]);
    assert_eq!((e.mem.u8(NPC + 0xa3), e.mem.u32(NPC + 0xa4)), (1, 250));
    assert_eq!(call(&mut e, 0x2247, &[r(1)]).1, 250.0);
    assert_eq!(
        call_on(&mut e, 0x2247, ACTOR, &[]).1,
        250.0,
        "the calling actor's base"
    );
    call_on(&mut e, 0x2248, ACTOR, &[n(-5)]);
    assert_eq!((e.mem.u8(NPC + 0xa3), e.mem.u32(NPC + 0xa4)), (0, 0));
    // A reference that is not an actor: nothing.
    e.mem.set_u32(NPC + 0xa4, 9);
    call_on(&mut e, 0x2248, PLAIN_REF, &[n(30)]);
    assert_eq!(e.mem.u32(NPC + 0xa4), 9);
}

#[test]
fn services_and_template() {
    let mut e = setup();
    call(&mut e, 0x22e7, &[r(1), n(0x404), n(1)]);
    assert_eq!(e.mem.u32(NPC + 0x9c), 0x404);
    assert_eq!(call(&mut e, 0x22e6, &[r(1), n(0x400)]).1, 1.0);
    call(&mut e, 0x22e7, &[r(1), n(0x400), n(0)]);
    assert_eq!(call(&mut e, 0x22e6, &[r(1), n(0x400)]).1, 0.0);
    assert_eq!(e.mem.u32(NPC + 0x9c), 4);
    call(&mut e, 0x22f6, &[n(0x1_0003), n(1), r(1)]);
    assert_eq!(e.mem.u16(NPC + 0x4a), 3, "16 bits");
    assert_eq!(e.mem.u16(NPC + 0x4c), 0);
    assert_eq!(call_on(&mut e, 0x22f5, ACTOR, &[n(2)]).1, 1.0);
    call(&mut e, 0x22f6, &[n(1), n(0), r(1)]);
    assert_eq!(e.mem.u16(NPC + 0x4a), 2);
    call(&mut e, 0x22fa, &[r(2), r(1)]);
    assert_eq!(e.mem.u32(NPC + 0x54), CREA);
}

#[test]
fn base_faction_ranks() {
    let mut e = setup();
    assert_eq!(call(&mut e, 0x2314, &[r(3), r(1)]).1, -1.0);
    let first = heap_next(&e);
    call(&mut e, 0x2315, &[r(3), n(2), r(1)]);
    assert_eq!(allocations(), vec![8]);
    assert_eq!(e.mem.u32(NPC + 0x5c), first, "into the empty first node");
    assert_eq!((e.mem.u32(first), e.mem.u8(first + 4)), (FACTION, 2));
    assert_eq!(e.mem.u8(first + 5), 0xcd, "the pad left as allocated");
    call(&mut e, 0x2315, &[r(4), n(1), r(1)]);
    assert_eq!(call(&mut e, 0x2314, &[r(3), r(1)]).1, 2.0);
    assert_eq!(call_on(&mut e, 0x2314, ACTOR, &[r(4)]).1, 1.0);
    // Not a faction: -1.
    assert_eq!(call(&mut e, 0x2314, &[r(6), r(1)]).1, -1.0);
    // An existing entry: its rank changed.
    call(&mut e, 0x2315, &[r(3), n(5), r(1)]);
    assert_eq!(call(&mut e, 0x2314, &[r(3), r(1)]).1, 5.0);
    // 128 is -128 as a char: the entry goes (its 8 bytes freed, and the node after the first,
    // where it sits since FACTION2 went first).
    call(&mut e, 0x2315, &[r(3), n(128), r(1)]);
    assert_eq!(freed(), vec![first, first + 0x10]);
    assert_eq!(call(&mut e, 0x2314, &[r(3), r(1)]).1, -1.0);
    // -7 is -1: removing one that is not there does nothing.
    call(&mut e, 0x2315, &[r(3), n(-7), r(1)]);
    assert_eq!(freed().len(), 2);
    assert_eq!(allocations().len(), 3, "no new entry for a negative rank");
}

#[test]
fn karma() {
    let mut e = setup();
    call(&mut e, 0x2320, &[n(-750), r(1)]);
    assert_eq!(e.mem.f32(NPC + 0x44), -750.0);
    assert_eq!(call_on(&mut e, 0x231f, ACTOR, &[]).1, -750.0);
    assert_eq!(call_on(&mut e, 0x231f, PLAIN_REF, &[]).1, 0.0);
}

#[test]
fn creature_type_and_body_part_data() {
    let mut e = setup();
    e.mem.set_u8(CREA + 0x12c, 3);
    assert_eq!(call(&mut e, 0x224d, &[r(2)]).1, 3.0);
    assert_eq!(call(&mut e, 0x224d, &[r(1)]).1, -1.0, "an NPC");
    assert_eq!(call(&mut e, 0x224d, &[]).1, -1.0);
    call(&mut e, 0x2333, &[r(1)]);
    assert_eq!(form_result(&e), 0x109);
    call(&mut e, 0x2848, &[r(2), r(7)]);
    assert_eq!(e.mem.u32(CREA + 0x144), BPD);
    call(&mut e, 0x2848, &[r(1), r(7)]);
    assert_eq!(e.mem.u32(NPC + 0x144), 0, "an NPC has no such field");
    e.mem.set_u32(CREA + 0x144, 0);
    call(&mut e, 0x2848, &[r(2), r(6)]);
    assert_eq!(e.mem.u32(CREA + 0x144), 0, "not body part data");
}

#[test]
fn combat_style() {
    let mut e = setup();
    call(&mut e, 0x2214, &[r(1)]);
    assert_eq!(form_result(&e), 0x107);
    e.mem.set_f64(RESULT, 0.0);
    call(&mut e, 0x2214, &[r(2)]);
    assert_eq!(form_result(&e), 0, "the creature has none");
    call_on(&mut e, 0x2214, ACTOR, &[]);
    assert_eq!(form_result(&e), 0x107, "the actor's base form's");
    put_extra(&mut e, ACTOR + 0x44, X2, 0x69, CSTY2);
    call_on(&mut e, 0x2214, ACTOR, &[]);
    assert_eq!(form_result(&e), 0x108, "its ExtraCombatStyle first");
}

#[test]
fn merchant_container() {
    let mut e = setup();
    call_on(&mut e, 0x223c, ACTOR, &[]);
    assert_eq!(form_result(&e), 0);
    put_extra(&mut e, ACTOR + 0x44, X3, 0x3c, PLAIN_REF);
    call_on(&mut e, 0x223c, ACTOR, &[]);
    assert_eq!(form_result(&e), 0x202);
}

fn eval(e: &mut Engine, opcode: u32, this: u32, arg1: u32) -> f64 {
    let record = nvse::table::record_of(e, opcode);
    let at = e.mem.u32(record + 0x20);
    e.mem.set_f64(RESULT, 0.0);
    e.call(at, &[this, arg1, 0, RESULT]);
    e.mem.f64(RESULT)
}

#[test]
fn race_in_a_list() {
    let mut e = setup();
    node(&mut e, LIST + 0x18, MISC, NODES);
    node(&mut e, NODES, RACE, 0);
    e.mem.set_u32(NPC + 0x110, RACE);
    assert_eq!(call_on(&mut e, 0x2265, ACTOR, &[r(5)]).1, 1.0);
    assert_eq!(eval(&mut e, 0x2265, ACTOR, LIST), 1.0);
    e.mem.set_u32(NODES, FACTION);
    assert_eq!(eval(&mut e, 0x2265, ACTOR, LIST), 0.0);
    // Not a character: false.
    e.mem.set_u32(NODES, RACE);
    e.register(CALL_IS_CHAR, |_, _| Ret::default());
    assert_eq!(eval(&mut e, 0x2265, ACTOR, LIST), 0.0);
}

#[test]
fn an_active_effect() {
    let mut e = setup();
    // The actor's magic target (+0x94) answers its effect list (vtable +8).
    e.mem.set_u32(ACTOR + 0x94, TARGET_VT);
    e.mem.set_u32(TARGET_VT + 8, CALL_EFFECTS);
    e.register(CALL_EFFECTS, |_, _| Ret {
        eax: NODES + 0x10,
        ..Ret::default()
    });
    node(&mut e, NODES + 0x10, FX, 0);
    e.mem.set_u32(FX + 0xc, FX_ITEM);
    e.mem.set_u32(FX_ITEM + 0x14, MGEF);
    e.mem.set_u32(FX + 8, MISC);
    e.mem.set_u8(FX + 0x10, 1);
    assert_eq!(call_on(&mut e, 0x2321, ACTOR, &[r(8)]).1, 1.0);
    assert_eq!(eval(&mut e, 0x2321, ACTOR, MGEF), 1.0);
    e.mem.set_u8(FX + 0x13, 1);
    assert_eq!(eval(&mut e, 0x2321, ACTOR, MGEF), 0.0, "terminated");
    e.mem.set_u8(FX + 0x13, 0);
    e.mem.set_u8(FX + 0x10, 0);
    assert_eq!(eval(&mut e, 0x2321, ACTOR, MGEF), 0.0, "not active");
    e.mem.set_u8(FX + 0x10, 1);
    e.mem.set_u32(FX + 8, 0);
    assert_eq!(eval(&mut e, 0x2321, ACTOR, MGEF), 0.0, "no magic item");
    e.mem.set_u32(FX + 8, MISC);
    assert_eq!(eval(&mut e, 0x2321, PLAIN_REF, MGEF), 0.0, "not an actor");
}

/// The actor's inventory: a weapon entry (two lists: a worn one with health 0.5 and mod flags 3,
/// and an unworn plain one) and an armour entry (part mask bit 2, one worn list).
fn inventory(e: &mut Engine) {
    put_extra(e, ACTOR + 0x44, X1 + 0x40, 0x15, DATA);
    e.mem.set_u32(DATA, NODES + 0x20);
    node(e, NODES + 0x20, ENTRY_W, NODES + 0x28);
    node(e, NODES + 0x28, ENTRY_A, 0);
    e.mem.set_u32(ENTRY_W, NODES + 0x30);
    e.mem.set_u32(ENTRY_W + 8, WEAP);
    node(e, NODES + 0x30, XDL2, NODES + 0x38);
    node(e, NODES + 0x38, XDL1, 0);
    put_extra(e, XDL1, XW, 0x16, 0);
    put_extra(e, XDL1, XH, 0x25, 0.5f32.to_bits());
    put_extra(e, XDL1, XM, 0x8d, 3);
    e.mem.set_u32(ENTRY_A, NODES + 0x40);
    e.mem.set_u32(ENTRY_A + 8, ARMO);
    e.mem.set_u32(ARMO + 0x74, 4);
    node(e, NODES + 0x40, XDL3, 0);
    put_extra(e, XDL3, XW3, 0x16, 0);
}

#[test]
fn equipped_data() {
    let mut e = setup();
    inventory(&mut e);
    let (_, res) = call_on(&mut e, 0x2323, ACTOR, &[n(5)]);
    assert_eq!(
        array_data(&e, res),
        vec![Data::Form(0x10b), Data::Number(0.5), Data::Number(3.0)]
    );
    let (_, res) = call_on(&mut e, 0x2323, ACTOR, &[n(2)]);
    assert_eq!(
        array_data(&e, res),
        vec![Data::Form(0x10c), Data::Number(-1.0), Data::Number(0.0)]
    );
    assert_eq!(
        call_on(&mut e, 0x2323, ACTOR, &[n(3)]).1,
        0.0,
        "nothing in slot 3"
    );
    // Slot 20 and up: every bit (the first worn item, the weapon).
    let (_, res) = call_on(&mut e, 0x2323, ACTOR, &[n(25)]);
    assert_eq!(array_data(&e, res)[0], Data::Form(0x10b));
    assert_eq!(call_on(&mut e, 0x2323, PLAIN_REF, &[n(5)]).1, 0.0);
}

#[test]
fn equip_by_data() {
    let mut e = setup();
    inventory(&mut e);
    e.register(0x0088_c650, |_, w| {
        CALLS.with(|c| c.borrow_mut().push((0x0088_c650, w.to_vec())));
        Ret::default()
    });
    // Health 0.5 and flags 3: the worn list.
    call_on(&mut e, 0x2325, ACTOR, &[r(9), z(0.5), n(3), n(1), n(0)]);
    assert_eq!(
        calls_to(0x0088_c650),
        vec![vec![ACTOR, WEAP, 1, XDL1, 1, 1, 0]]
    );
    // No health, no flags: the first list without mod flags (the plain one).
    call_on(&mut e, 0x2325, ACTOR, &[r(9)]);
    assert_eq!(
        calls_to(0x0088_c650)[1],
        vec![ACTOR, WEAP, 1, XDL2, 1, 0, 0]
    );
    // A health that matches no list: nothing.
    call_on(&mut e, 0x2325, ACTOR, &[r(9), z(0.75)]);
    assert_eq!(calls_to(0x0088_c650).len(), 2);
    // From an array (silent defaults to 1 there).
    let id = array_of(&mut e, vec![V::Form(0x10b), V::Number(0.5), V::Number(3.0)]);
    call_on(&mut e, 0x2324, ACTOR, &[n(id as i32)]);
    assert_eq!(
        calls_to(0x0088_c650)[2],
        vec![ACTOR, WEAP, 1, XDL1, 1, 0, 1]
    );
    let short = array_of(&mut e, vec![V::Form(0x10b), V::Number(0.5)]);
    call_on(&mut e, 0x2324, ACTOR, &[n(short as i32)]);
    assert_eq!(calls_to(0x0088_c650).len(), 3, "not three elements");
}

/// The equip match: a health that differs from the list's (even with matching flags) equips
/// nothing; with no health and no flags the first list WITHOUT mod flags is taken, even when a
/// modded list comes first.
#[test]
fn equip_match_rules() {
    let mut e = setup();
    inventory(&mut e);
    e.register(0x0088_c650, |_, w| {
        CALLS.with(|c| c.borrow_mut().push((0x0088_c650, w.to_vec())));
        Ret::default()
    });
    call_on(&mut e, 0x2325, ACTOR, &[r(9), z(0.75), n(3)]);
    assert!(calls_to(0x0088_c650).is_empty(), "health bits differ");
    // The modded list first, then the plain one.
    node(&mut e, NODES + 0x30, XDL1, NODES + 0x38);
    node(&mut e, NODES + 0x38, XDL2, 0);
    call_on(&mut e, 0x2325, ACTOR, &[r(9)]);
    assert_eq!(
        calls_to(0x0088_c650),
        vec![vec![ACTOR, WEAP, 1, XDL2, 1, 0, 0]]
    );
}

/// Rank 0 on an existing entry is a rank, not a removal.
#[test]
fn faction_rank_zero_kept() {
    let mut e = setup();
    call(&mut e, 0x2315, &[r(3), n(4), r(1)]);
    call(&mut e, 0x2315, &[r(3), n(0), r(1)]);
    assert_eq!(call(&mut e, 0x2314, &[r(3), r(1)]).1, 0.0);
    assert!(freed().is_empty());
}
