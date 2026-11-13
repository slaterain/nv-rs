//! Tests of the effect commands, at raw offsets: a spell's or enchantment's magic item at +0x18
//! (its effects' list at +0x28 of the form), an ingestible's at +0x30 (list +0x40); a spell's and
//! an enchantment's type +0x34 and flags +0x40; an ingestible's flags +0xBC, addiction +0xC0,
//! chance +0xC4; an effect item's magnitude +0, area +4, duration +8, range +0xC, actor value
//! +0x10, base effect +0x14, cost +0x18, conditions +0x1C; a base effect's flags +0x58,
//! script +0x60, archetype +0x98, actor value +0x9C; an actor base's unarmed effect +0x74 and
//! spells +0x80; an active effect's elapsed +4, magic item +8, item +0xC, active +0x10,
//! terminated +0x13, flags +0x18, magnitude +0x1C, duration +0x20, target +0x24, caster +0x28,
//! object +0x3C; an actor's caster +0x88 and target +0x94; the process array's data +8, first
//! indexes +0x14, past-the-last +0x24 of `011E0E80`.
//!
//! The game functions JIP calls are stand-ins that record: the cast (`00ec43fb`, by a table),
//! `MagicItem::GetType` (the word at +0x700 of the magic item), the target's effect list (the
//! word at +0x700 of the target), the caster's actor and new effect (its words at +0x700 and
//! +0x704), `ApplyEffect`, `Destroy`, `ActiveEffect::Remove`, the game's
//! `MagicTarget::RemoveEffect` (it unlinks the effect from the list as the exe's `00905330`
//! does) and the effect item's delete.

use super::*;
use crate::commands::test_support::*;
use nvse::vars::{Container, Data};
use std::cell::RefCell;

const SPEL: u32 = FORMS + 0x1000;
const SPEL2: u32 = FORMS + 0x2000;
const ENCH: u32 = FORMS + 0x3000;
const ENCH2: u32 = FORMS + 0x4000;
const ALCH: u32 = FORMS + 0x5000;
const ALCH2: u32 = FORMS + 0x6000;
const MGEF: u32 = FORMS + 0x7000;
const MGEF2: u32 = FORMS + 0x8000;
const SCPT: u32 = FORMS + 0x9000;
const MISC: u32 = FORMS + 0xa000;
const NPC: u32 = FORMS + 0xb000;
const NPC2: u32 = FORMS + 0xc000;
const ARMO: u32 = FORMS + 0xd000;
const WEAP: u32 = FORMS + 0xe000;
const FLST: u32 = FORMS + 0xf000;
const PC: u32 = FORMS + 0x1_0000;
const ACT1: u32 = FORMS + 0x1_1000;
const ACT2: u32 = FORMS + 0x1_2000;
const ACT3: u32 = FORMS + 0x1_3000;
const NONACT: u32 = FORMS + 0x1_4000;
const ITEMS: u32 = FORMS + 0x2_0000;
const EFFS: u32 = FORMS + 0x3_0000;
const NODES: u32 = FORMS + 0x5_0000;
const NODE_TOP: u32 = FORMS + 0x5_7ffc;
const ARRAY: u32 = FORMS + 0x5_8000;
const VT: u32 = FORMS + 0x6_0000;
const MI_VT: u32 = VT;
const ACTOR_VT: u32 = VT + 0x200;
const NONACT_VT: u32 = VT + 0x400;
const TARGET_VT: u32 = VT + 0x600;
const CASTER_VT: u32 = VT + 0x800;
const AE_VT: u32 = VT + 0xa00;
const MAGIC_VT: u32 = VT + 0xc00;
const PLAIN_VT: u32 = VT + 0xe00;
const SETTINGS: u32 = FORMS + 0x8_0000;

const PLAYER_PTR: u32 = 0x011d_ea3c;
const PM: u32 = 0x011e_0e80;
const RETURN_TRUE_FN: u32 = 0x008d_0360;
const RETURN_FALSE_FN: u32 = 0x0047_c850;
const SAYS_ACTOR_FN: u32 = 0x0050_2000;
const CAST_FN: u32 = 0x00ec_43fb;
const TYPE_FN: u32 = 0x0050_1000;
const APPLY_FN: u32 = 0x0050_1010;
const LIST_FN: u32 = 0x0050_1020;
const GET_ACTOR_FN: u32 = 0x0050_1030;
const CREATE_FN: u32 = 0x0050_1040;
const DESTROY_FN: u32 = 0x0050_1050;
const REMOVE_FN: u32 = 0x0080_4210;
const MT_REMOVE_FN: u32 = 0x0082_51c0;
const DELETE_FN: u32 = 0x0040_59d0;
const RTTI_TES_FORM: u32 = 0x0118_3028;
const RTTI_MAGIC_ITEM: u32 = 0x0118_3140;
const RTTI_ENCHANTABLE: u32 = 0x0118_39b4;

thread_local! {
    /// The cast stand-in's table: the object, the class asked for, the answer.
    static CASTS: RefCell<Vec<(u32, u32, u32)>> = const { RefCell::new(Vec::new()) };
}

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

fn clear_calls() {
    CALLS.with(|c| c.borrow_mut().clear());
}

fn cast(_: &mut Engine, w: &[u32]) -> Ret {
    let (object, to) = (w[0], w[3]);
    let found = CASTS.with(|c| {
        c.borrow()
            .iter()
            .find(|&&(o, t, _)| o == object && t == to)
            .map(|&(_, _, r)| r)
    });
    eax(found.unwrap_or(0))
}

fn add_cast(object: u32, to: u32, answer: u32) {
    CASTS.with(|c| c.borrow_mut().push((object, to, answer)));
}

/// A new 8-byte list node of the test's own.
fn node(e: &mut Engine) -> u32 {
    let at = e.mem.u32(NODE_TOP);
    e.mem.set_u32(NODE_TOP, at + 8);
    e.mem.set_u32(at, 0);
    e.mem.set_u32(at + 4, 0);
    at
}

/// A list with its first node at `head` holding `items`; the nodes after the first.
fn list(e: &mut Engine, head: u32, items: &[u32]) -> Vec<u32> {
    e.mem.set_u32(head, items.first().copied().unwrap_or(0));
    e.mem.set_u32(head + 4, 0);
    let mut last = head;
    let mut nodes = Vec::new();
    for &item in items.iter().skip(1) {
        let n = node(e);
        e.mem.set_u32(n, item);
        e.mem.set_u32(last + 4, n);
        last = n;
        nodes.push(n);
    }
    nodes
}

/// The items of the list at `head`.
fn items(e: &Engine, head: u32) -> Vec<u32> {
    let mut v = vec![e.mem.u32(head)];
    let mut n = e.mem.u32(head + 4);
    while n != 0 {
        v.push(e.mem.u32(n));
        n = e.mem.u32(n + 4);
    }
    v
}

/// The nodes of a list: its first node, then each next.
fn nodes_of(e: &Engine, head: u32) -> Vec<u32> {
    let mut v = vec![head];
    let mut n = e.mem.u32(head + 4);
    while n != 0 {
        v.push(n);
        n = e.mem.u32(n + 4);
    }
    v
}

/// Effect item `i` (0x40 apart): base effect, magnitude, area, duration, range; its actor value
/// 0x40 + i, cost 1.5, no conditions.
fn item(e: &mut Engine, i: u32, setting: u32, mag: u32, area: u32, dur: u32, range: u32) -> u32 {
    let at = ITEMS + 0x40 * i;
    for (off, v) in [
        (0, mag),
        (4, area),
        (8, dur),
        (0xc, range),
        (0x10, 0x40 + i),
    ] {
        e.mem.set_u32(at + off, v);
    }
    e.mem.set_u32(at + 0x14, setting);
    e.mem.set_f32(at + 0x18, 1.5);
    e.mem.set_u32(at + 0x1c, 0);
    e.mem.set_u32(at + 0x20, 0);
    at
}

/// Active effect `i` (0x100 apart) of `magic_item` and `item`, active and not terminated, with
/// `actor`'s caster and target; elapsed 0.25 + i, flags 0x100 + i, magnitude 10 + i,
/// duration 20 + i, object 0x0A000000 + i.
fn effect(e: &mut Engine, i: u32, magic_item: u32, item: u32, actor: u32) -> u32 {
    let at = EFFS + 0x100 * i;
    e.mem.set_u32(at, AE_VT);
    e.mem.set_f32(at + 4, 0.25 + i as f32);
    e.mem.set_u32(at + 8, magic_item);
    e.mem.set_u32(at + 0xc, item);
    e.mem.set_u8(at + 0x10, 1);
    e.mem.set_u8(at + 0x13, 0);
    e.mem.set_u32(at + 0x18, 0x100 + i);
    e.mem.set_f32(at + 0x1c, 10.0 + i as f32);
    e.mem.set_f32(at + 0x20, 20.0 + i as f32);
    e.mem.set_u32(at + 0x24, actor + 0x94);
    e.mem.set_u32(at + 0x28, actor + 0x88);
    e.mem.set_u32(at + 0x3c, 0x0a00_0000 + i);
    at
}

/// The actor's effect list: a first node of the test's own holding `effects`.
fn effects_of(e: &mut Engine, actor: u32, effects: &[u32]) -> u32 {
    let head = node(e);
    list(e, head, effects);
    e.mem.set_u32(actor + 0x94 + 0x700, head);
    head
}

fn magic_form(e: &mut Engine, at: u32, kind: u8, id: u32, mi: u32, mi_type: u32) {
    form(e, at, kind, id);
    e.mem.set_u32(at, MAGIC_VT);
    e.mem.set_u32(at + mi, MI_VT);
    e.mem.set_u32(at + mi + 0x700, mi_type);
    add_cast(at, RTTI_MAGIC_ITEM, at + mi);
    add_cast(at + mi, RTTI_TES_FORM, at);
}

fn actor_ref(e: &mut Engine, at: u32, id: u32, base: u32) {
    form(e, at, 0x3b, id);
    e.mem.set_u32(at, ACTOR_VT);
    e.mem.set_u32(at + 0x20, base);
    e.mem.set_u32(at + 0x88, CASTER_VT);
    e.mem.set_u32(at + 0x94, TARGET_VT);
}

/// Script references: 1 SPEL, 2 SPEL2, 3 ENCH, 4 ENCH2, 5 ALCH, 6 ALCH2, 7 MGEF, 8 MGEF2,
/// 9 SCPT, 10 MISC, 11 NPC, 12 NPC2, 13 ARMO, 14 WEAP, 15 FLST.
const R_SPEL: u16 = 1;
const R_SPEL2: u16 = 2;
const R_ENCH: u16 = 3;
const R_ENCH2: u16 = 4;
const R_ALCH: u16 = 5;
const R_ALCH2: u16 = 6;
const R_MGEF: u16 = 7;
const R_MGEF2: u16 = 8;
const R_SCPT: u16 = 9;
const R_MISC: u16 = 10;
const R_NPC: u16 = 11;
const R_ARMO: u16 = 13;
const R_WEAP: u16 = 14;
const R_FLST: u16 = 15;

/// The forms (SPEL a spell of type 0, SPEL2 of type 2, ENCH an apparel enchantment, ENCH2 a
/// weapon one, ALCH and ALCH2 ingestibles, MGEF and MGEF2 base effects, SCPT a script, MISC
/// filled with 0xA5, NPC and NPC2 actor bases, ARMO an armour and WEAP a weapon with
/// enchantments, FLST a form list), the player PC (base NPC), the actors ACT1 (high level),
/// ACT2 (middle-high) and ACT3 (middle-low: never visited), all of base NPC2, and NONACT (a
/// reference whose vtable says no actor, in the high level), and the stand-ins. Every list
/// empty.
fn setup() -> Engine {
    let mut e = engine();
    CASTS.with(|c| c.borrow_mut().clear());
    e.map(0x011d_e000, 0x1000);
    e.map(0x011e_0000, 0x2000);
    e.mem.set_u32(NODE_TOP, NODES);
    magic_form(&mut e, SPEL, 0x14, 0x0000_0514, 0x18, 0);
    magic_form(&mut e, SPEL2, 0x14, 0x0000_0524, 0x18, 2);
    e.mem.set_u32(SPEL2 + 0x34, 2);
    magic_form(&mut e, ENCH, 0x13, 0x0000_0613, 0x18, 6);
    e.mem.set_u32(ENCH + 0x34, 3);
    magic_form(&mut e, ENCH2, 0x13, 0x0000_0623, 0x18, 6);
    e.mem.set_u32(ENCH2 + 0x34, 2);
    magic_form(&mut e, ALCH, 0x2f, 0x0000_072f, 0x30, 7);
    magic_form(&mut e, ALCH2, 0x2f, 0x0000_073f, 0x30, 7);
    form(&mut e, MGEF, 0x10, 0x0000_0810);
    form(&mut e, MGEF2, 0x10, 0x0000_0820);
    form(&mut e, SCPT, 0x11, 0x0000_0911);
    for i in 0..0x100 {
        e.mem.set_u32(MISC + 4 * i, 0xa5a5_a5a5);
    }
    form(&mut e, MISC, 0x1f, 0x0000_0a1f);
    e.mem.set_u32(MISC, PLAIN_VT);
    form(&mut e, NPC, 0x2a, 0x0000_0b2a);
    form(&mut e, NPC2, 0x2a, 0x0000_0c2a);
    form(&mut e, ARMO, 0x18, 0x0000_0d18);
    e.mem.set_u32(ARMO, 0x0102_a62c);
    add_cast(ARMO, RTTI_ENCHANTABLE, ARMO + 0x48);
    form(&mut e, WEAP, 0x28, 0x0000_0e28);
    add_cast(WEAP, RTTI_ENCHANTABLE, WEAP + 0x60);
    form(&mut e, FLST, 0x55, 0x0000_0f55);
    for (i, f) in [
        SPEL, SPEL2, ENCH, ENCH2, ALCH, ALCH2, MGEF, MGEF2, SCPT, MISC, NPC, NPC2, ARMO, WEAP, FLST,
    ]
    .iter()
    .enumerate()
    {
        set_ref(&mut e, i as u16 + 1, *f);
    }
    actor_ref(&mut e, PC, 0x0000_0014, NPC);
    actor_ref(&mut e, ACT1, 0xff00_0001, NPC2);
    actor_ref(&mut e, ACT2, 0xff00_0002, NPC2);
    actor_ref(&mut e, ACT3, 0xff00_0003, NPC2);
    actor_ref(&mut e, NONACT, 0xff00_0004, NPC2);
    e.mem.set_u32(NONACT, NONACT_VT);
    for a in [PC, ACT1, ACT2, ACT3, NONACT] {
        effects_of(&mut e, a, &[]);
    }
    e.mem.set_u32(PLAYER_PTR, PC);
    // The process array: high 0..3 (ACT1, a null entry, NONACT), middle-high 3..4 (ACT2),
    // middle-low 4..5 (ACT3).
    for (i, a) in [ACT1, 0, NONACT, ACT2, ACT3].iter().enumerate() {
        e.mem.set_u32(ARRAY + 4 * i as u32, *a);
    }
    e.mem.set_u32(PM + 8, ARRAY);
    for (level, (first, past)) in [(0, 3), (3, 4), (4, 5), (5, 5)].iter().enumerate() {
        e.mem.set_u32(PM + 0x14 + 4 * level as u32, *first);
        e.mem.set_u32(PM + 0x24 + 4 * level as u32, *past);
    }
    e.mem.set_u32(MI_VT + 0x18, TYPE_FN);
    e.mem.set_u32(ACTOR_VT + 0x100, RETURN_TRUE_FN);
    e.mem.set_u32(NONACT_VT + 0x100, RETURN_FALSE_FN);
    e.mem.set_u32(TARGET_VT, APPLY_FN);
    e.mem.set_u32(TARGET_VT + 8, LIST_FN);
    e.mem.set_u32(CASTER_VT + 0x2c, GET_ACTOR_FN);
    e.mem.set_u32(CASTER_VT + 0x4c, CREATE_FN);
    e.mem.set_u32(AE_VT, DESTROY_FN);
    e.mem.set_u32(MAGIC_VT + 0xec, RETURN_TRUE_FN);
    e.mem.set_u32(PLAIN_VT + 0xec, RETURN_FALSE_FN);
    e.mem.set_u32(SCRIPT + 0xc, 0x0600_0001);
    e.register(CAST_FN, cast);
    e.register(RETURN_TRUE_FN, |_, _| eax(1));
    e.register(RETURN_FALSE_FN, |_, _| eax(0));
    e.register(SAYS_ACTOR_FN, |_, _| eax(1));
    e.register(TYPE_FN, |e, w| eax(e.mem.u32(w[0] + 0x700)));
    e.register(APPLY_FN, |_, w| {
        record(APPLY_FN, w);
        eax(1)
    });
    e.register(LIST_FN, |e, w| {
        record(LIST_FN, w);
        eax(e.mem.u32(w[0] + 0x700))
    });
    e.register(GET_ACTOR_FN, |e, w| eax(e.mem.u32(w[0] + 0x700)));
    e.register(CREATE_FN, |e, w| {
        record(CREATE_FN, w);
        eax(e.mem.u32(w[0] + 0x704))
    });
    e.register(DESTROY_FN, |_, w| {
        record(DESTROY_FN, w);
        eax(0)
    });
    e.register(REMOVE_FN, |_, w| {
        record(REMOVE_FN, w);
        eax(0)
    });
    e.register(MT_REMOVE_FN, |e, w| {
        record(MT_REMOVE_FN, w);
        let head = e.mem.u32(w[0] + 0x700);
        crate::commands::list_remove(e, Ptr::new(head), w[1]);
        eax(0)
    });
    e.register(DELETE_FN, |_, w| {
        record(DELETE_FN, w);
        eax(0)
    });
    clear_calls();
    e
}

/// The effects' list (its first node) of a magic item form: +0x28 of a spell or enchantment,
/// +0x40 of an ingestible.
fn effects_head(form: u32) -> u32 {
    if form == ALCH || form == ALCH2 {
        form + 0x40
    } else {
        form + 0x28
    }
}

/// The elements of a result's array (`None` for no array).
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

/// A condition's eval through its record, the result first 0.5.
fn eval(e: &mut Engine, opcode: u32, this: u32, arg1: u32) -> f64 {
    let record = nvse::table::record_of(e, opcode);
    let at = e.mem.u32(record + 0x20);
    assert_ne!(at, 0, "{opcode:04x}: an eval");
    e.mem.set_f64(RESULT, 0.5);
    e.call(at, &[this, arg1, 0, RESULT]);
    e.mem.f64(RESULT)
}

/// The player's, ACT1's and ACT2's targets: the actors JIP's update reaches.
fn reached() -> Vec<Vec<u32>> {
    vec![vec![PC + 0x94], vec![ACT1 + 0x94], vec![ACT2 + 0x94]]
}

// ---- a magic item's effects ----

#[test]
fn counting_and_reading_effects() {
    let mut e = setup();
    let i0 = item(&mut e, 0, MGEF, 1, 2, 3, 0);
    let i1 = item(&mut e, 1, MGEF2, 0xffff_fffe, 7, 9, 2);
    let i2 = item(&mut e, 2, SCPT, 4, 5, 6, 1);
    let i3 = item(&mut e, 3, 0, 4, 5, 6, 1);
    list(&mut e, effects_head(SPEL), &[i0, i1, i2]);
    list(&mut e, effects_head(ALCH), &[i3]);
    list(&mut e, effects_head(SPEL2), &[]);
    let misc = e.mem.bytes(MISC, 0x400);
    assert_eq!(call(&mut e, 0x267b, &[r(R_SPEL)]).1, 3.0);
    assert_eq!(
        call(&mut e, 0x267b, &[r(R_ALCH)]).1,
        1.0,
        "the list at +0x40"
    );
    assert_eq!(call(&mut e, 0x267b, &[r(R_SPEL2)]).1, 0.0);
    assert_eq!(
        call(&mut e, 0x267b, &[r(R_MISC)]).1,
        0.0,
        "no magic item: no read"
    );
    for (idx, want) in [(0, 0x0810), (1, 0x0820), (2, 0x0911), (3, 0), (-1, 0)] {
        call(&mut e, 0x267c, &[r(R_SPEL), n(idx)]);
        assert_eq!(form_result(&e), want, "effect {idx}");
    }
    call(&mut e, 0x267c, &[r(R_ALCH), n(0)]);
    assert_eq!(form_result(&e), 0, "no base effect");
    for (t, want) in [(0, -2.0), (1, 7.0), (2, 9.0), (3, 2.0), (4, 0.0)] {
        assert_eq!(
            call(&mut e, 0x267d, &[r(R_SPEL), n(1), n(t)]).1,
            want,
            "trait {t}"
        );
    }
    assert_eq!(
        call(&mut e, 0x267d, &[r(R_SPEL), n(0), n(2)]).1,
        3.0,
        "the head"
    );
    assert_eq!(
        call(&mut e, 0x267d, &[r(R_SPEL), n(2), n(0)]).1,
        4.0,
        "the tail"
    );
    assert_eq!(call(&mut e, 0x267d, &[r(R_ALCH), n(0), n(3)]).1, 1.0, "one");
    assert_eq!(call(&mut e, 0x267d, &[r(R_SPEL), n(5), n(0)]).1, 0.0);
    assert_eq!(e.mem.bytes(MISC, 0x400), misc);
    assert!(calls_to(LIST_FN).is_empty(), "nothing updated");
}

#[test]
fn setting_numbers_and_the_base_effect() {
    let mut e = setup();
    let i0 = item(&mut e, 0, MGEF, 1, 2, 3, 0);
    let i1 = item(&mut e, 1, MGEF, 11, 12, 13, 2);
    let i2 = item(&mut e, 2, MGEF, 21, 22, 23, 1);
    list(&mut e, effects_head(SPEL), &[i0, i1, i2]);
    let set = |e: &mut Engine, idx: i32, t: i32, v: i32| {
        clear_calls();
        call(e, 0x267e, &[r(R_SPEL), n(idx), n(t), n(v)]);
    };
    let with = |e: &Engine, at: u32, off: usize, v: u32| {
        let mut b = e.mem.bytes(at, 0x24);
        b[off..off + 4].copy_from_slice(&v.to_le_bytes());
        b
    };
    let want = with(&e, i1, 0, 0x1234);
    set(&mut e, 1, 0, 0x1234);
    assert_eq!(e.mem.bytes(i1, 0x24), want, "the magnitude alone");
    assert_eq!(calls_to(LIST_FN), reached(), "the actors updated");
    let want = with(&e, i1, 4, 0xfff0);
    set(&mut e, 1, 1, 0xfff0);
    assert_eq!(e.mem.bytes(i1, 0x24), want, "the area alone");
    assert!(calls_to(LIST_FN).is_empty(), "no update after the area");
    let want = with(&e, i2, 8, 0xffff_ffff);
    set(&mut e, 2, 2, -1);
    assert_eq!(e.mem.bytes(i2, 0x24), want, "the duration of the tail");
    assert_eq!(calls_to(LIST_FN), reached());
    let all = e.mem.bytes(ITEMS, 0xc0);
    set(&mut e, 0, 3, 5);
    set(&mut e, 3, 0, 5);
    assert_eq!(
        e.mem.bytes(ITEMS, 0xc0),
        all,
        "trait 3, past the end: nothing"
    );
    assert!(calls_to(LIST_FN).is_empty());
    // The base effect: its actor value is the byte at +0x9C, widened.
    e.mem.set_u32(MGEF2 + 0x9c, 0xaabb_ccf3);
    let mut want = with(&e, i0, 0x14, MGEF2);
    want[0x10..0x14].copy_from_slice(&0xf3u32.to_le_bytes());
    clear_calls();
    call(&mut e, 0x268b, &[r(R_SPEL), n(0), r(R_MGEF2)]);
    assert_eq!(
        e.mem.bytes(i0, 0x24),
        want,
        "the head's base effect and actor value"
    );
    assert_eq!(calls_to(LIST_FN), reached());
    let all = e.mem.bytes(ITEMS, 0xc0);
    clear_calls();
    call(&mut e, 0x268b, &[r(R_SPEL), n(3), r(R_MGEF2)]);
    call(&mut e, 0x268b, &[r(R_SPEL), n(0), r(R_MISC)]);
    assert_eq!(
        e.mem.bytes(ITEMS, 0xc0),
        all,
        "past the end; no base effect"
    );
    assert!(calls_to(LIST_FN).is_empty());
}

/// JIP's update of the actors under a changed effect: the player's matching effect is replaced
/// (time, active byte and flags carried over); ACT1's is dropped (its time reached the new
/// duration); NONACT, whose virtual `IsActor` says yes, is visited (its caster makes nothing);
/// ACT2's terminated match stops its search; ACT3 (middle-low) is not visited.
#[test]
fn the_actors_get_the_changed_effect() {
    let mut e = setup();
    let i0 = item(&mut e, 0, MGEF, 1, 2, 3, 0);
    let i1 = item(&mut e, 1, MGEF, 11, 12, 13, 2);
    list(&mut e, effects_head(SPEL), &[i0, i1]);
    let mi = SPEL + 0x18;
    let other = effect(&mut e, 0, ALCH + 0x30, i1, PC);
    let first = effect(&mut e, 1, mi, i0, PC);
    let old = effect(&mut e, 2, mi, i1, PC);
    e.mem.set_f32(old + 4, 2.5);
    e.mem.set_u8(old + 0x10, 0);
    e.mem.set_u32(old + 0x18, 0x505);
    effects_of(&mut e, PC, &[other, first, old]);
    let fresh = effect(&mut e, 3, 0, 0, PC);
    e.mem.set_f32(fresh + 0x20, 4.0);
    e.mem.set_u32(fresh + 0x24, 0);
    e.mem.set_u32(PC + 0x88 + 0x704, fresh);
    let old1 = effect(&mut e, 4, mi, i1, ACT1);
    e.mem.set_f32(old1 + 4, 5.0);
    effects_of(&mut e, ACT1, &[old1]);
    let fresh1 = effect(&mut e, 5, 0, 0, ACT1);
    e.mem.set_f32(fresh1 + 0x20, 5.0);
    e.mem.set_u32(ACT1 + 0x88 + 0x704, fresh1);
    let done = effect(&mut e, 6, mi, i1, ACT2);
    e.mem.set_u8(done + 0x13, 1);
    let after = effect(&mut e, 7, mi, i1, ACT2);
    effects_of(&mut e, ACT2, &[done, after]);
    e.mem.set_u32(ACT2 + 0x88 + 0x704, fresh1);
    let skipped = effect(&mut e, 8, mi, i1, NONACT);
    effects_of(&mut e, NONACT, &[skipped]);
    effects_of(&mut e, ACT3, &[skipped]);
    e.mem.set_u32(NONACT_VT + 0x100, SAYS_ACTOR_FN);
    let before = e.mem.bytes(fresh, 0x48);
    call(&mut e, 0x267e, &[r(R_SPEL), n(1), n(2), n(30)]);
    assert_eq!(
        calls_to(CREATE_FN),
        vec![
            vec![PC + 0x88, mi, i1, 0x0a00_0002],
            vec![ACT1 + 0x88, mi, i1, 0x0a00_0004],
            vec![NONACT + 0x88, mi, i1, 0x0a00_0008],
        ]
    );
    let mut want = before;
    want[4..8].copy_from_slice(&2.5f32.to_le_bytes());
    want[0x10] = 0;
    want[0x18..0x1c].copy_from_slice(&0x505u32.to_le_bytes());
    want[0x24..0x28].copy_from_slice(&(PC + 0x94).to_le_bytes());
    assert_eq!(
        e.mem.bytes(fresh, 0x48),
        want,
        "time, active, flags, target"
    );
    assert_eq!(calls_to(REMOVE_FN), vec![vec![old, 1], vec![old1, 1]]);
    assert_eq!(
        calls_to(DESTROY_FN),
        vec![vec![fresh1, 1]],
        "time 5.0 >= 5.0"
    );
    assert_eq!(
        calls_to(APPLY_FN),
        vec![vec![PC + 0x94, PC + 0x88, mi, fresh, 0]]
    );
    assert_eq!(
        calls_to(LIST_FN),
        vec![
            vec![PC + 0x94],
            vec![ACT1 + 0x94],
            vec![NONACT + 0x94],
            vec![ACT2 + 0x94]
        ]
    );
}

#[test]
fn adding_effects() {
    let mut e = setup();
    let i0 = item(&mut e, 0, MGEF, 1, 2, 3, 0);
    let i1 = item(&mut e, 1, MGEF, 11, 12, 13, 2);
    let i2 = item(&mut e, 2, MGEF, 21, 22, 23, 1);
    let nodes = list(&mut e, effects_head(SPEL), &[i0, i1, i2]);
    for f in [SPEL2, ALCH2, ENCH, ENCH2] {
        list(&mut e, effects_head(f), &[]);
    }
    e.mem.set_u32(MGEF + 0x58, 0x70);
    e.mem.set_u32(MGEF + 0x9c, 0x1111_1177);
    e.mem.set_u32(MGEF2 + 0x58, 0x30);
    // The player's first effect of SPEL is terminated: adding still replaces it.
    let mi = SPEL + 0x18;
    let other = effect(&mut e, 0, ALCH + 0x30, i0, PC);
    let first = effect(&mut e, 1, mi, i1, PC);
    e.mem.set_u8(first + 0x13, 1);
    effects_of(&mut e, PC, &[other, first]);
    let fresh = effect(&mut e, 2, 0, 0, PC);
    e.mem.set_u32(PC + 0x88 + 0x704, fresh);
    let act1 = effect(&mut e, 3, mi, i0, ACT1);
    effects_of(&mut e, ACT1, &[act1]);
    let at = heap_next(&e);
    let add = |e: &mut Engine, m: u16, s: u16, args: &[i32]| -> f64 {
        clear_calls();
        let mut v = vec![r(m), r(s)];
        v.extend(args.iter().map(|&a| n(a)));
        call(e, 0x26f7, &v).1
    };
    assert_eq!(
        add(&mut e, R_SPEL, R_MGEF, &[11, 22, 2, 33, 1]),
        3.0,
        "appended"
    );
    let mut want = Vec::new();
    for v in [22u32, 33, 11, 2, 0x77, MGEF, 0, 0, 0] {
        want.extend_from_slice(&v.to_le_bytes());
    }
    assert_eq!(e.mem.bytes(at, 0x24), want, "the new item, every field");
    assert_eq!(allocations(), vec![0x24, 8]);
    assert_eq!(items(&e, effects_head(SPEL)), vec![i0, i1, i2, at]);
    assert_eq!(e.mem.u32(nodes[1] + 4), at + 0x28, "after the old tail");
    assert_eq!(
        calls_to(CREATE_FN),
        vec![
            vec![PC + 0x88, mi, at, 0x0a00_0001],
            vec![ACT1 + 0x88, mi, at, 0x0a00_0003],
        ],
        "each actor's first effect of the item"
    );
    assert!(calls_to(REMOVE_FN).is_empty(), "adding removes nothing");
    assert_eq!(
        calls_to(APPLY_FN),
        vec![vec![PC + 0x94, PC + 0x88, mi, fresh, 0]],
        "ACT1's caster made none"
    );
    assert_eq!(e.mem.u32(fresh + 0x24), PC + 0x94);
    assert_eq!(calls_to(LIST_FN), reached());
    // The head item of a list, its range and area.
    let head = |e: &Engine, f: u32| {
        let it = e.mem.u32(effects_head(f));
        (e.mem.u32(it + 0xc), e.mem.u32(it + 4))
    };
    // Put first: the count; an empty list takes the item into its first node.
    assert_eq!(
        add(&mut e, R_SPEL2, R_MGEF, &[1, 2, 1, 5]),
        1.0,
        "a power: touch kept"
    );
    assert_eq!(allocations(), vec![0x24]);
    assert_eq!(head(&e, SPEL2), (1, 5));
    let first_new = e.mem.u32(effects_head(SPEL2));
    assert_eq!(add(&mut e, R_SPEL2, R_MGEF, &[1, 2, 2, 5]), 2.0);
    assert_eq!(allocations(), vec![0x24, 8]);
    let second = items(&e, effects_head(SPEL2));
    assert_eq!(second[1], first_new, "the newest first");
    assert_eq!(head(&e, SPEL2), (2, 5));
    // Appended to an empty list: the item into the first node, 0 + 1.
    assert_eq!(add(&mut e, R_ALCH2, R_MGEF, &[1, 2, 2, 9, 1]), 1.0);
    assert_eq!(allocations(), vec![0x24]);
    assert_eq!(head(&e, ALCH2), (0, 0), "an ingestible: self, no area");
    assert_eq!(add(&mut e, R_ENCH2, R_MGEF, &[1, 2, 0, 4]), 1.0);
    assert_eq!(head(&e, ENCH2), (1, 4), "a weapon enchantment: touch");
    assert_eq!(add(&mut e, R_ENCH, R_MGEF, &[1, 2, 2, 4]), 1.0);
    assert_eq!(head(&e, ENCH), (0, 0), "an apparel enchantment: self");
    for kind in [1, 4, 10] {
        e.mem.set_u32(SPEL2 + 0x18 + 0x700, kind);
        add(&mut e, R_SPEL2, R_MGEF, &[1, 2, 2, 6]);
        assert_eq!(head(&e, SPEL2), (0, 0), "type {kind}: self");
    }
    e.mem.set_u32(SPEL2 + 0x18 + 0x700, 0);
    add(&mut e, R_SPEL2, R_MGEF, &[1, 2, 0, 8]);
    assert_eq!(head(&e, SPEL2), (0, 0), "self: no area");
    add(&mut e, R_SPEL2, R_MGEF, &[1, 2, 2, 8]);
    assert_eq!(head(&e, SPEL2), (2, 8), "an actor effect: target kept");
    // Refused: range 3, a range the base effect does not allow, no magic item.
    let count = items(&e, effects_head(SPEL2)).len();
    for (m, s, args) in [
        (R_SPEL2, R_MGEF, [1, 2, 3, 0]),
        (R_SPEL2, R_MGEF2, [1, 2, 2, 0]),
        (R_MISC, R_MGEF, [1, 2, 0, 0]),
        (R_SPEL2, R_MISC, [1, 2, 0, 0]),
    ] {
        assert_eq!(add(&mut e, m, s, &args), -1.0, "{m} {s} {args:?}");
        assert!(allocations().is_empty());
        assert!(calls_to(LIST_FN).is_empty());
    }
    assert_eq!(items(&e, effects_head(SPEL2)).len(), count);
    // The base effect allows touch only: self refused, touch taken.
    e.mem.set_u32(MGEF2 + 0x58, 0x20);
    assert_eq!(add(&mut e, R_SPEL2, R_MGEF2, &[1, 2, 0, 0]), -1.0);
    assert_eq!(
        add(&mut e, R_SPEL2, R_MGEF2, &[1, 2, 1, 0]),
        count as f64 + 1.0
    );
}

/// Removing effect 1 of three: each of its active effects on the player (the first two in a
/// row, and one after an effect of another item) and on ACT1 removed and taken out by the game;
/// NONACT (no actor by its vtable word, though its virtual `IsActor` says yes) and ACT3
/// (middle-low) left; the node freed; the item deleted by the game's destructor.
#[test]
fn removing_effects() {
    let mut e = setup();
    let i0 = item(&mut e, 0, MGEF, 1, 2, 3, 0);
    let i1 = item(&mut e, 1, MGEF, 11, 12, 13, 2);
    let i2 = item(&mut e, 2, MGEF, 21, 22, 23, 1);
    let snodes = list(&mut e, effects_head(SPEL), &[i0, i1, i2]);
    let mi = SPEL + 0x18;
    let a = effect(&mut e, 0, mi, i1, PC);
    let b = effect(&mut e, 1, mi, i1, PC);
    let c = effect(&mut e, 2, mi, i0, PC);
    let d = effect(&mut e, 3, mi, i1, PC);
    let head = effects_of(&mut e, PC, &[a, b, c, d]);
    let pnodes = nodes_of(&e, head);
    let f = effect(&mut e, 4, mi, i1, ACT1);
    let act1 = effects_of(&mut e, ACT1, &[f]);
    let g = effect(&mut e, 5, mi, i1, NONACT);
    let nonact = effects_of(&mut e, NONACT, &[g]);
    let act3 = effects_of(&mut e, ACT3, &[g]);
    e.mem.set_u32(NONACT_VT + 0x100, SAYS_ACTOR_FN);
    assert_eq!(call(&mut e, 0x26f8, &[r(R_SPEL), n(1)]).1, 2.0);
    assert_eq!(
        calls_to(REMOVE_FN),
        vec![vec![a, 1], vec![b, 1], vec![d, 1], vec![f, 1]]
    );
    assert_eq!(
        calls_to(MT_REMOVE_FN),
        vec![
            vec![PC + 0x94, a, 1],
            vec![PC + 0x94, b, 1],
            vec![PC + 0x94, d, 1],
            vec![ACT1 + 0x94, f, 1],
        ]
    );
    assert_eq!(items(&e, head), vec![c]);
    assert_eq!(items(&e, act1), vec![0]);
    assert_eq!(items(&e, nonact), vec![g]);
    assert_eq!(items(&e, act3), vec![g]);
    assert_eq!(items(&e, effects_head(SPEL)), vec![i0, i2]);
    assert_eq!(calls_to(DELETE_FN), vec![vec![i1, 1]]);
    assert_eq!(
        freed(),
        vec![pnodes[1], pnodes[2], pnodes[3], snodes[0]],
        "the game's removals, then the effect's node"
    );
    assert!(allocations().is_empty());
    // The head of two.
    clear_calls();
    assert_eq!(call(&mut e, 0x26f8, &[r(R_SPEL), n(0)]).1, 1.0);
    assert_eq!(items(&e, effects_head(SPEL)), vec![i2]);
    assert_eq!(calls_to(DELETE_FN), vec![vec![i0, 1]]);
    assert_eq!(
        freed(),
        vec![snodes[1]],
        "the second node, its item moved up"
    );
    assert_eq!(calls_to(REMOVE_FN), vec![vec![c, 1]]);
    // One left: nothing.
    clear_calls();
    assert_eq!(call(&mut e, 0x26f8, &[r(R_SPEL), n(0)]).1, -1.0);
    assert_eq!(items(&e, effects_head(SPEL)), vec![i2]);
    assert!(calls().is_empty(), "no call at all");
}

#[test]
fn removing_the_tail_and_past_the_end() {
    let mut e = setup();
    let i0 = item(&mut e, 0, MGEF, 1, 2, 3, 0);
    let i1 = item(&mut e, 1, MGEF, 11, 12, 13, 2);
    let i2 = item(&mut e, 2, MGEF, 21, 22, 23, 1);
    let snodes = list(&mut e, effects_head(SPEL), &[i0, i1, i2]);
    for idx in [3, 7, -1] {
        assert_eq!(call(&mut e, 0x26f8, &[r(R_SPEL), n(idx)]).1, -1.0, "{idx}");
    }
    assert!(calls().is_empty());
    assert_eq!(items(&e, effects_head(SPEL)), vec![i0, i1, i2]);
    assert_eq!(call(&mut e, 0x26f8, &[r(R_SPEL), n(2)]).1, 2.0);
    assert_eq!(items(&e, effects_head(SPEL)), vec![i0, i1]);
    assert_eq!(e.mem.u32(snodes[0] + 4), 0);
    assert_eq!(freed(), vec![snodes[1]]);
    assert_eq!(calls_to(DELETE_FN), vec![vec![i2, 1]]);
    let misc = e.mem.bytes(MISC, 0x400);
    assert_eq!(
        call(&mut e, 0x26f8, &[r(R_MISC), n(0)]).1,
        -1.0,
        "no magic item"
    );
    assert_eq!(e.mem.bytes(MISC, 0x400), misc);
    // A list whose second node is empty counts as one.
    clear_calls();
    list(&mut e, effects_head(SPEL2), &[i0, 0]);
    assert_eq!(call(&mut e, 0x26f8, &[r(R_SPEL2), n(0)]).1, -1.0);
    assert_eq!(items(&e, effects_head(SPEL2)), vec![i0, 0]);
    assert!(calls().is_empty());
}

// ---- object, unarmed and actor effects ----

#[test]
fn object_effects() {
    let mut e = setup();
    let slot = |e: &Engine, at: u32| e.mem.u32(at + 4);
    assert_eq!(call(&mut e, 0x26f9, &[r(R_ARMO), r(R_ENCH)]).1, 1.0);
    assert_eq!(slot(&e, ARMO + 0x48), ENCH, "apparel on armour");
    assert_eq!(call(&mut e, 0x26f9, &[r(R_ARMO), r(R_ENCH2)]).1, 0.0);
    assert_eq!(slot(&e, ARMO + 0x48), ENCH, "a weapon enchantment refused");
    assert_eq!(call(&mut e, 0x26f9, &[r(R_WEAP), r(R_ENCH2)]).1, 1.0);
    assert_eq!(slot(&e, WEAP + 0x60), ENCH2, "weapon on anything else");
    assert_eq!(call(&mut e, 0x26f9, &[r(R_WEAP), r(R_ENCH)]).1, 0.0);
    assert_eq!(slot(&e, WEAP + 0x60), ENCH2);
    assert_eq!(call(&mut e, 0x26f9, &[r(R_WEAP), r(R_MISC)]).1, 1.0);
    assert_eq!(slot(&e, WEAP + 0x60), 0, "another form clears");
    assert_eq!(call(&mut e, 0x26f9, &[r(R_ARMO)]).1, 1.0);
    assert_eq!(slot(&e, ARMO + 0x48), 0, "none clears");
    let misc = e.mem.bytes(MISC, 0x400);
    assert_eq!(
        call(&mut e, 0x26f9, &[r(R_MISC), r(R_ENCH)]).1,
        0.0,
        "no enchantment"
    );
    assert_eq!(e.mem.bytes(MISC, 0x400), misc);
}

#[test]
fn an_actor_bases_effects() {
    let mut e = setup();
    list(&mut e, NPC + 0x80, &[SPEL, SPEL2, ENCH]);
    list(&mut e, NPC2 + 0x80, &[ALCH]);
    e.mem.set_u32(NPC + 0x74, ENCH2);
    assert_eq!(call(&mut e, 0x267f, &[r(R_NPC)]).1, 3.0);
    assert_eq!(
        call_on(&mut e, 0x267f, ACT1, &[]).1,
        1.0,
        "the actor's base"
    );
    assert_eq!(call_on(&mut e, 0x267f, NONACT, &[]).1, 0.0, "no actor");
    assert_eq!(call_on(&mut e, 0x267f, 0, &[]).1, 0.0, "no reference");
    assert_eq!(call(&mut e, 0x267f, &[r(R_MISC)]).1, 0.0, "no actor base");
    for (idx, want) in [(0, 0x0514), (1, 0x0524), (2, 0x0613), (3, 0), (-1, 0)] {
        call(&mut e, 0x2680, &[n(idx), r(R_NPC)]);
        assert_eq!(form_result(&e), want, "spell {idx}");
    }
    call_on(&mut e, 0x2680, ACT2, &[n(0)]);
    assert_eq!(form_result(&e), 0x072f);
    call_on(&mut e, 0x2680, NONACT, &[n(0)]);
    assert_eq!(form_result(&e), 0);
    call(&mut e, 0x26fa, &[r(R_NPC)]);
    assert_eq!(form_result(&e), 0x0623);
    call_on(&mut e, 0x26fa, PC, &[]);
    assert_eq!(form_result(&e), 0x0623, "the player's base is NPC");
    call_on(&mut e, 0x26fa, ACT1, &[]);
    assert_eq!(form_result(&e), 0, "NPC2 has none");
    call_on(&mut e, 0x26fa, NONACT, &[]);
    assert_eq!(form_result(&e), 0);
    let npc = e.mem.bytes(NPC, 0x10c);
    let set = |e: &mut Engine, f: u16| call(e, 0x26fb, &[r(f), r(R_NPC)]).1;
    assert_eq!(set(&mut e, R_ENCH), 0.0, "an apparel enchantment refused");
    assert_eq!(set(&mut e, R_SPEL2), 0.0, "a power refused");
    assert_eq!(set(&mut e, R_MISC), 0.0);
    assert_eq!(e.mem.bytes(NPC, 0x10c), npc);
    assert_eq!(set(&mut e, R_SPEL), 1.0);
    let mut want = npc;
    want[0x74..0x78].copy_from_slice(&SPEL.to_le_bytes());
    assert_eq!(e.mem.bytes(NPC, 0x10c), want, "the unarmed effect alone");
    assert_eq!(call_on(&mut e, 0x26fb, ACT1, &[r(R_ENCH2)]).1, 1.0);
    assert_eq!(e.mem.u32(NPC2 + 0x74), ENCH2, "the actor's base");
    assert_eq!(call_on(&mut e, 0x26fb, NONACT, &[r(R_SPEL)]).1, 0.0);
    assert_eq!(e.mem.u32(NPC2 + 0x74), ENCH2);
}

// ---- ingestibles ----

#[test]
fn addiction() {
    let mut e = setup();
    e.mem.set_u32(ALCH + 0xc0, SPEL);
    e.mem.set_f32(ALCH + 0xc4, 0.25);
    e.mem.set_f32(ALCH2 + 0xc4, 0.75);
    call(&mut e, 0x271c, &[r(R_ALCH)]);
    assert_eq!(form_result(&e), 0x0514);
    call(&mut e, 0x271c, &[r(R_ALCH2)]);
    assert_eq!(form_result(&e), 0, "none");
    e.mem.set_u32(SPEL + 0xc0, SPEL2);
    call(&mut e, 0x271c, &[r(R_SPEL)]);
    assert_eq!(form_result(&e), 0, "no ingestible");
    assert_eq!(call(&mut e, 0x271e, &[r(R_ALCH)]).1, 0.25);
    assert_eq!(call(&mut e, 0x271e, &[r(R_ALCH2)]).1, 0.0, "no addiction");
    e.mem.set_f32(SPEL + 0xc4, 0.5);
    assert_eq!(call(&mut e, 0x271e, &[r(R_SPEL)]).1, 0.0);
    let alch = e.mem.bytes(ALCH, 0xd8);
    call(&mut e, 0x271f, &[r(R_ALCH), z(0.125)]);
    let mut want = alch;
    want[0xc4..0xc8].copy_from_slice(&0.125f32.to_le_bytes());
    assert_eq!(e.mem.bytes(ALCH, 0xd8), want, "the chance alone");
    call(&mut e, 0x271f, &[r(R_ALCH2), z(0.5)]);
    assert_eq!(e.mem.f32(ALCH2 + 0xc4), 0.75, "no addiction: left");
    call(&mut e, 0x271f, &[r(R_SPEL), z(0.75)]);
    assert_eq!(e.mem.f32(SPEL + 0xc4), 0.5);
    let before = e.mem.bytes(ALCH, 0xd8);
    call(&mut e, 0x271d, &[r(R_ALCH), r(R_SPEL2)]);
    let mut want = before;
    want[0xc0..0xc4].copy_from_slice(&SPEL2.to_le_bytes());
    assert_eq!(e.mem.bytes(ALCH, 0xd8), want, "the addiction alone");
    call(&mut e, 0x271d, &[r(R_ALCH), r(R_ENCH)]);
    assert_eq!(e.mem.u32(ALCH + 0xc0), 0, "no spell: none");
    call(&mut e, 0x271d, &[r(R_ALCH2), r(R_SPEL)]);
    assert_eq!(e.mem.u32(ALCH2 + 0xc0), SPEL);
    call(&mut e, 0x271d, &[r(R_ALCH2)]);
    assert_eq!(e.mem.u32(ALCH2 + 0xc0), 0, "left out: none");
    call(&mut e, 0x271d, &[r(R_SPEL), r(R_SPEL)]);
    assert_eq!(e.mem.u32(SPEL + 0xc0), SPEL2, "no ingestible");
}

#[test]
fn poison() {
    let mut e = setup();
    e.mem.set_u32(MGEF + 0x58, 0xffff_fffb);
    e.mem.set_u32(MGEF2 + 0x58, 4);
    let bad = item(&mut e, 0, MGEF2, 0, 0, 0, 0);
    let good = item(&mut e, 1, MGEF, 0, 0, 0, 0);
    let none = item(&mut e, 2, 0, 0, 0, 0, 0);
    let poison = |e: &mut Engine| call(e, 0x2720, &[r(R_ALCH)]).1;
    let cases: [(&[u32], f64, &str); 9] = [
        (&[bad, bad, bad], 1.0, "every effect detrimental"),
        (&[bad], 1.0, "one"),
        (&[good, bad, bad], 0.0, "the head is not"),
        (&[bad, good, bad], 0.0, "the middle is not"),
        (&[bad, bad, good], 0.0, "the tail is not"),
        (&[bad, none], 0.0, "the last has no base effect"),
        (&[none, bad], 1.0, "one without a base effect before"),
        (&[bad, 0], 1.0, "an empty node passed over"),
        (&[], 0.0, "no effects"),
    ];
    for (effects, want, what) in cases {
        list(&mut e, effects_head(ALCH), effects);
        assert_eq!(poison(&mut e), want, "{what}");
    }
    list(&mut e, effects_head(SPEL), &[bad]);
    assert_eq!(call(&mut e, 0x2720, &[r(R_SPEL)]).1, 0.0, "no ingestible");
}

// ---- an actor's active effects ----

#[test]
fn active_effects() {
    let mut e = setup();
    let i0 = item(&mut e, 0, MGEF, 0, 0, 0, 0);
    let i1 = item(&mut e, 1, MGEF2, 0, 0, 0, 0);
    e.mem.set_u32(MGEF2 + 0x58, 0x100);
    e.mem.set_u32(MGEF + 0x58, 0xffff_feff);
    // Address order SPEL < ENCH < ALCH, ids the other way round.
    e.mem.set_u32(SPEL + 0xc, 0x0300);
    e.mem.set_u32(ENCH + 0xc, 0x0200);
    e.mem.set_u32(ALCH + 0xc, 0x0100);
    let alch = effect(&mut e, 0, ALCH + 0x30, i0, PC);
    let spel = effect(&mut e, 1, SPEL + 0x18, i0, PC);
    let spel_again = effect(&mut e, 2, SPEL + 0x18, i0, PC);
    let off = effect(&mut e, 3, SPEL2 + 0x18, i0, PC);
    e.mem.set_u8(off + 0x10, 0);
    let ended = effect(&mut e, 4, SPEL2 + 0x18, i0, PC);
    e.mem.set_u8(ended + 0x13, 1);
    let no_item = effect(&mut e, 5, 0, i0, PC);
    let weak = effect(&mut e, 6, SPEL2 + 0x18, i0, PC);
    e.mem.set_f32(weak + 0x1c, 0.0);
    let flagged = effect(&mut e, 7, ENCH + 0x18, i1, PC);
    e.mem.set_f32(flagged + 0x1c, -1.0);
    let no_form = effect(&mut e, 8, MISC + 0x18, i0, PC);
    effects_of(
        &mut e,
        PC,
        &[
            alch, spel, off, ended, no_item, 0, weak, flagged, spel_again, no_form,
        ],
    );
    let get = |e: &mut Engine, f: Option<i32>| {
        let args: Vec<Vec<u8>> = f.map(n).into_iter().collect();
        let v = call_on(e, 0x2733, PC, &args).1;
        elements(e, v)
    };
    let ids = |v: &[u32]| Some(v.iter().map(|&i| Data::Form(i)).collect::<Vec<_>>());
    assert_eq!(
        get(&mut e, None),
        ids(&[0x0300, 0x0200, 0x0100]),
        "by address"
    );
    assert_eq!(get(&mut e, Some(7)), ids(&[0x0300, 0x0200, 0x0100]));
    assert_eq!(get(&mut e, Some(1)), ids(&[0x0200]), "enchantments");
    assert_eq!(get(&mut e, Some(2)), ids(&[0x0300]), "spells");
    assert_eq!(get(&mut e, Some(4)), ids(&[0x0100]), "ingestibles");
    assert_eq!(
        get(&mut e, Some(3)),
        ids(&[0x0300, 0x0200]),
        "0x13 & 0x14 = 0x10"
    );
    assert_eq!(
        get(&mut e, Some(6)),
        ids(&[0x0300, 0x0100]),
        "0x14 & 0x2F = 4"
    );
    assert_eq!(get(&mut e, Some(0)), None, "63: none");
    assert_eq!(call_on(&mut e, 0x2733, NONACT, &[]).1, 0.0, "no actor");
    assert_eq!(call_on(&mut e, 0x2733, 0, &[]).1, 0.0, "no reference");
    effects_of(&mut e, PC, &[off, ended]);
    assert_eq!(call_on(&mut e, 0x2733, PC, &[]).1, 0.0, "none: no array");
    e.mem.set_u32(PC + 0x94 + 0x700, 0);
    assert_eq!(call_on(&mut e, 0x2733, PC, &[]).1, 0.0, "no list");
}

/// A timed effect's item: its base effect (`i`, archetype `arch` in the low byte, `flags`) and
/// its duration; magnitude 0xFFFFFFFD.
fn temp_case(e: &mut Engine, i: u32, arch: u32, flags: u32, duration: u32) -> u32 {
    let s = SETTINGS + 0x100 * i;
    form(e, s, 0x10, 0x0000_9000 + i);
    e.mem.set_u32(s + 0x58, flags);
    e.mem.set_u32(s + 0x98, 0xffff_ff00 | arch);
    item(e, 0x10 + i, s, 0xffff_fffd, 0, duration, 0)
}

#[test]
fn timed_effects() {
    let mut e = setup();
    // Accepted archetypes: 0, 1 with flag 0x2000, 11, 13, 24, 34. Refused: 1 without it, 10,
    // 14, 23, 33, a duration of 0, no base effect.
    let cases: [(u32, u32, u32, bool); 12] = [
        (0, 0, 5, true),
        (1, 0x2000, 5, true),
        (11, 0, 5, true),
        (13, 0, 5, true),
        (24, 0, 5, true),
        (34, 0, 5, true),
        (1, 0xffff_dfff, 5, false),
        (10, 0, 5, false),
        (14, 0, 5, false),
        (23, 0, 5, false),
        (33, 0, 5, false),
        (0, 0, 0, false),
    ];
    let mut all = Vec::new();
    let mut accepted = Vec::new();
    for (i, &(arch, flags, dur, ok)) in cases.iter().enumerate() {
        let i = i as u32;
        let it = temp_case(&mut e, i, arch, flags, dur);
        let ae = effect(&mut e, 0x10 + i, SPEL + 0x18, it, PC);
        // Time left (20 + 16 + i) - (16.25 + 1.5 i): falling, so the sort reverses them.
        e.mem.set_f32(ae + 4, 16.25 + i as f32 * 1.5);
        all.push(ae);
        if ok {
            accepted.push((i, ae));
        }
    }
    let unset = item(&mut e, 0x30, 0, 0, 0, 5, 0);
    all.push(effect(&mut e, 0x30, SPEL + 0x18, unset, PC));
    effects_of(&mut e, PC, &all);
    let v = call_on(&mut e, 0x2739, PC, &[]).1;
    let outer = elements(&e, v).expect("an array");
    assert_eq!(outer.len(), accepted.len());
    for (k, &(i, ae)) in accepted.iter().rev().enumerate() {
        let Data::Array(id) = outer[k] else {
            panic!("an array")
        };
        let duration = 36.0 + i as f32;
        let left = duration - e.mem.f32(ae + 4);
        assert_eq!(
            elements(&e, f64::from(id)),
            Some(vec![
                Data::Form(0x0514),
                Data::Form(0x9000 + i),
                Data::Number(f64::from(26.0 + i as f32)),
                Data::Number(f64::from(left)),
                Data::Form(0),
                Data::Number(f64::from(duration)),
            ]),
            "case {i}"
        );
    }
    assert_eq!(call_on(&mut e, 0x2739, NONACT, &[]).1, 0.0);
    assert_eq!(call_on(&mut e, 0x2739, 0, &[]).1, 0.0);
    effects_of(&mut e, PC, &all[6..]);
    assert_eq!(call_on(&mut e, 0x2739, PC, &[]).1, 0.0, "none: no array");
}

/// A timed effect's numbers: the magnitude by base effect flag 2 (the item's word as an `int`
/// made a float, else the effect's magnitude without its sign), the caster's actor (none
/// without a caster), equal times kept in their order.
#[test]
fn timed_effect_values_and_ties() {
    let mut e = setup();
    let s = SETTINGS;
    form(&mut e, s, 0x10, 0x9000);
    e.mem.set_u32(s + 0x58, 2);
    e.mem.set_u32(s + 0x98, 0);
    let s2 = SETTINGS + 0x100;
    form(&mut e, s2, 0x10, 0x9001);
    e.mem.set_u32(s2 + 0x58, 0xffff_fffd);
    e.mem.set_u32(s2 + 0x98, 0);
    let big = item(&mut e, 0, s, 16_777_217, 0, 1, 0);
    let plain = item(&mut e, 1, s2, 7, 0, 1, 0);
    let a = effect(&mut e, 0, SPEL + 0x18, big, PC);
    let b = effect(&mut e, 1, ENCH + 0x18, plain, PC);
    let c = effect(&mut e, 2, ALCH + 0x30, plain, PC);
    let d = effect(&mut e, 3, ALCH2 + 0x30, plain, PC);
    for (ae, elapsed, dur, mag) in [
        (a, 1.0, 4.0, 2.0),
        (b, 0.5, 3.5, -6.5),
        (c, 2.0, 5.0, 1.0),
        (d, 0.0, 1.0, 1.0),
    ] {
        e.mem.set_f32(ae + 4, elapsed);
        e.mem.set_f32(ae + 0x20, dur);
        e.mem.set_f32(ae + 0x1c, mag);
    }
    e.mem.set_u32(b + 0x28, 0);
    e.mem.set_u32(PC + 0x88 + 0x700, ACT1);
    effects_of(&mut e, PC, &[a, b, c, d]);
    let v = call_on(&mut e, 0x2739, PC, &[]).1;
    let outer = elements(&e, v).expect("an array");
    let inner: Vec<Vec<Data>> = outer
        .iter()
        .map(|el| match el {
            Data::Array(id) => elements(&e, f64::from(*id)).expect("an array"),
            _ => panic!("an array"),
        })
        .collect();
    assert_eq!(inner.len(), 4);
    assert_eq!(inner[0][0], Data::Form(0x073f), "d: 1.0 left, first");
    assert_eq!(
        inner[1][0],
        Data::Form(0x0514),
        "a, b, c: 3.0 left each, in order"
    );
    assert_eq!(
        inner[1][2],
        Data::Number(16_777_216.0),
        "the int made a float"
    );
    assert_eq!(inner[1][4], Data::Form(0xff00_0001), "the caster's actor");
    assert_eq!(inner[2][0], Data::Form(0x0613));
    assert_eq!(inner[2][2], Data::Number(6.5), "no sign");
    assert_eq!(inner[2][4], Data::Form(0), "no caster");
    assert_eq!(inner[3][0], Data::Form(0x072f));
    assert_eq!(inner[3][3], Data::Number(3.0));
}

#[test]
fn removing_a_timed_effect() {
    let mut e = setup();
    let ok = temp_case(&mut e, 0, 0, 0, 5);
    let refused = temp_case(&mut e, 1, 10, 0, 5);
    let a = effect(&mut e, 0, SPEL + 0x18, ok, PC);
    let skip = effect(&mut e, 1, SPEL + 0x18, refused, PC);
    let off = effect(&mut e, 2, SPEL + 0x18, ok, PC);
    e.mem.set_u8(off + 0x10, 0);
    let ended = effect(&mut e, 3, SPEL + 0x18, ok, PC);
    e.mem.set_u8(ended + 0x13, 1);
    let no_item = effect(&mut e, 4, 0, ok, PC);
    let no_form = effect(&mut e, 5, MISC + 0x18, ok, PC);
    let b = effect(&mut e, 6, ENCH + 0x18, ok, PC);
    let c = effect(&mut e, 7, ALCH + 0x30, ok, PC);
    effects_of(
        &mut e,
        PC,
        &[a, skip, off, ended, no_item, no_form, 0, b, c],
    );
    for (idx, want) in [(0, a), (1, b), (2, c)] {
        clear_calls();
        assert_eq!(call_on(&mut e, 0x273a, PC, &[n(idx)]).1, 1.0, "{idx}");
        assert_eq!(calls_to(REMOVE_FN), vec![vec![want, 1]]);
    }
    clear_calls();
    for idx in [3, -1] {
        assert_eq!(call_on(&mut e, 0x273a, PC, &[n(idx)]).1, 0.0, "{idx}");
    }
    assert_eq!(call_on(&mut e, 0x273a, NONACT, &[n(0)]).1, 0.0);
    assert_eq!(call_on(&mut e, 0x273a, 0, &[n(0)]).1, 0.0);
    assert!(calls_to(REMOVE_FN).is_empty());
}

// ---- flags and scripts ----

#[test]
fn effect_flags() {
    let mut e = setup();
    e.mem.set_u8(SPEL + 0x40, 0x81);
    e.mem.set_u8(ENCH + 0x40, 0x02);
    e.mem.set_u8(ALCH + 0xbc, 0x40);
    e.mem.set_u8(ALCH + 0x40, 0xff);
    let get = |e: &mut Engine, f: u16, flag: i32| call(e, 0x2776, &[r(f), n(flag)]).1;
    assert_eq!(get(&mut e, R_SPEL, 0), 1.0);
    assert_eq!(get(&mut e, R_SPEL, 7), 1.0);
    assert_eq!(get(&mut e, R_SPEL, 1), 0.0);
    assert_eq!(get(&mut e, R_ENCH, 1), 1.0);
    assert_eq!(get(&mut e, R_ENCH, 0), 0.0);
    assert_eq!(get(&mut e, R_ALCH, 6), 1.0, "+0xBC");
    assert_eq!(get(&mut e, R_ALCH, 0), 0.0);
    e.mem.set_u8(SPEL + 0x41, 0xff);
    assert_eq!(get(&mut e, R_SPEL, 8), 0.0, "flag 8 refused");
    assert_eq!(get(&mut e, R_MISC, 0), 0.0, "another form");
    for (f, at, size, off) in [
        (R_SPEL, SPEL, 0x44, 0x40),
        (R_ENCH, ENCH, 0x44, 0x40),
        (R_ALCH, ALCH, 0xd8, 0xbc),
    ] {
        let before = e.mem.bytes(at, size);
        call(&mut e, 0x2777, &[r(f), n(4), n(3)]);
        let mut want = before.clone();
        want[off] |= 0x10;
        assert_eq!(e.mem.bytes(at, size), want, "{at:x}: set");
        call(&mut e, 0x2777, &[r(f), n(4), n(0)]);
        assert_eq!(e.mem.bytes(at, size), before, "{at:x}: cleared");
    }
    let spel = e.mem.bytes(SPEL, 0x44);
    call(&mut e, 0x2777, &[r(R_SPEL), n(8), n(1)]);
    call(&mut e, 0x2777, &[r(R_SPEL), n(8), n(0)]);
    assert_eq!(e.mem.bytes(SPEL, 0x44), spel, "flag 8 refused");
    let misc = e.mem.bytes(MISC, 0x400);
    call(&mut e, 0x2777, &[r(R_MISC), n(0), n(0)]);
    assert_eq!(e.mem.bytes(MISC, 0x400), misc);
}

#[test]
fn base_effect_flags_and_scripts() {
    let mut e = setup();
    e.mem.set_u32(MGEF + 0x58, 0x8000_0001);
    let get = |e: &mut Engine, f: u16, flag: i32| call(e, 0x2778, &[r(f), n(flag)]).1;
    assert_eq!(get(&mut e, R_MGEF, 0), 1.0);
    assert_eq!(get(&mut e, R_MGEF, 31), 1.0);
    assert_eq!(get(&mut e, R_MGEF, 1), 0.0);
    assert_eq!(get(&mut e, R_MGEF, 32), 0.0, "flag 32 refused");
    let misc = e.mem.bytes(MISC, 0x400);
    assert_eq!(get(&mut e, R_MISC, 0), 0.0);
    let before = e.mem.bytes(MGEF, 0xb0);
    call(&mut e, 0x2779, &[r(R_MGEF), n(20), n(5)]);
    let mut want = before.clone();
    want[0x5a] |= 0x10;
    assert_eq!(e.mem.bytes(MGEF, 0xb0), want, "bit 20");
    call(&mut e, 0x2779, &[r(R_MGEF), n(20), n(0)]);
    assert_eq!(e.mem.bytes(MGEF, 0xb0), before);
    call(&mut e, 0x2779, &[r(R_MGEF), n(32), n(1)]);
    call(&mut e, 0x2779, &[r(R_MGEF), n(32), n(0)]);
    assert_eq!(e.mem.bytes(MGEF, 0xb0), before, "flag 32 refused");
    call(&mut e, 0x2779, &[r(R_MISC), n(0), n(1)]);
    assert_eq!(e.mem.bytes(MISC, 0x400), misc);
    // Scripts: only a script effect's (archetype 1, its low byte).
    e.mem.set_u32(MGEF + 0x98, 0xffff_ff01);
    e.mem.set_u32(MGEF + 0x60, SCPT);
    e.mem.set_u32(MGEF2 + 0x98, 2);
    e.mem.set_u32(MGEF2 + 0x60, SCPT);
    call(&mut e, 0x27c1, &[r(R_MGEF)]);
    assert_eq!(form_result(&e), 0x0911);
    call(&mut e, 0x27c1, &[r(R_MGEF2)]);
    assert_eq!(form_result(&e), 0, "archetype 2");
    call(&mut e, 0x27c1, &[r(R_MISC)]);
    assert_eq!(form_result(&e), 0);
    e.mem.set_u32(MGEF + 0x60, 0);
    call(&mut e, 0x27c1, &[r(R_MGEF)]);
    assert_eq!(form_result(&e), 0, "no script");
    let before = e.mem.bytes(MGEF, 0xb0);
    call(&mut e, 0x27c2, &[r(R_MGEF), r(R_SCPT)]);
    let mut want = before.clone();
    want[0x60..0x64].copy_from_slice(&SCPT.to_le_bytes());
    assert_eq!(e.mem.bytes(MGEF, 0xb0), want, "the script alone");
    call(&mut e, 0x27c2, &[r(R_MGEF), r(R_MISC)]);
    assert_eq!(e.mem.bytes(MGEF, 0xb0), want, "no script");
    e.mem.set_u32(MGEF2 + 0x60, 0);
    call(&mut e, 0x27c2, &[r(R_MGEF2), r(R_SCPT)]);
    assert_eq!(e.mem.u32(MGEF2 + 0x60), 0, "archetype 2");
    call(&mut e, 0x27c2, &[r(R_MISC), r(R_SCPT)]);
    assert_eq!(e.mem.bytes(MISC, 0x400), misc);
}

// ---- the conditions ----

#[test]
fn is_spell_target_alt() {
    let mut e = setup();
    let i0 = item(&mut e, 0, MGEF, 0, 0, 0, 0);
    let mi = SPEL + 0x18;
    let off = effect(&mut e, 0, mi, i0, PC);
    e.mem.set_u8(off + 0x10, 0);
    let ended = effect(&mut e, 1, mi, i0, PC);
    e.mem.set_u8(ended + 0x13, 1);
    let other = effect(&mut e, 2, ENCH + 0x18, i0, PC);
    let on = effect(&mut e, 3, mi, i0, PC);
    effects_of(&mut e, PC, &[off, ended, 0, other, on]);
    effects_of(&mut e, ACT1, &[off, ended, other]);
    effects_of(&mut e, NONACT, &[on]);
    assert_eq!(call_on(&mut e, 0x27ec, PC, &[r(R_SPEL)]).1, 1.0, "the tail");
    assert_eq!(call_on(&mut e, 0x27ec, ACT1, &[r(R_SPEL)]).1, 0.0);
    assert_eq!(call_on(&mut e, 0x27ec, PC, &[r(R_ALCH)]).1, 0.0);
    assert_eq!(
        call_on(&mut e, 0x27ec, NONACT, &[r(R_SPEL)]).1,
        0.0,
        "no actor"
    );
    assert_eq!(
        call_on(&mut e, 0x27ec, 0, &[r(R_SPEL)]).1,
        0.0,
        "no reference"
    );
    assert_eq!(call_on(&mut e, 0x27ec, PC, &[r(R_MISC)]).1, 0.0, "no read");
    effects_of(&mut e, ACT2, &[on]);
    assert_eq!(call_on(&mut e, 0x27ec, ACT2, &[r(R_SPEL)]).1, 1.0, "one");
    assert_eq!(eval(&mut e, 0x27ec, PC, mi), 1.0);
    assert_eq!(eval(&mut e, 0x27ec, ACT1, mi), 0.5, "false: left");
    assert_eq!(
        eval(&mut e, 0x27ec, PC, SPEL),
        0.5,
        "the form is not its magic item"
    );
    assert_eq!(eval(&mut e, 0x27ec, NONACT, mi), 0.5);
    assert_eq!(eval(&mut e, 0x27ec, 0, mi), 0.5);
    e.mem.set_u32(PC + 0x94 + 0x700, 0);
    assert_eq!(eval(&mut e, 0x27ec, PC, mi), 0.5, "no list");
}

#[test]
fn is_spell_target_list() {
    let mut e = setup();
    let i0 = item(&mut e, 0, MGEF, 0, 0, 0, 0);
    let spel = effect(&mut e, 0, SPEL + 0x18, i0, PC);
    let alch = effect(&mut e, 1, ALCH + 0x30, i0, ACT1);
    let ench_off = effect(&mut e, 2, ENCH + 0x18, i0, ACT2);
    e.mem.set_u8(ench_off + 0x10, 0);
    let ench_ended = effect(&mut e, 3, ENCH + 0x18, i0, ACT2);
    e.mem.set_u8(ench_ended + 0x13, 1);
    let misc = effect(&mut e, 4, MISC + 0x18, i0, ACT3);
    effects_of(&mut e, PC, &[0, spel]);
    effects_of(&mut e, ACT1, &[alch]);
    effects_of(&mut e, ACT2, &[ench_off, ench_ended]);
    effects_of(&mut e, ACT3, &[misc]);
    effects_of(&mut e, NONACT, &[spel]);
    // The list: MISC (no magic item: its vtable's +0xEC), an empty node, ALCH (+0x30), SPEL
    // (+0x18), ENCH.
    list(&mut e, FLST + 0x18, &[MISC, 0, ALCH, SPEL, ENCH]);
    let call_l = |e: &mut Engine, this: u32| call_on(e, 0x291a, this, &[r(R_FLST)]).1;
    assert_eq!(call_l(&mut e, PC), 1.0, "a spell at +0x18");
    assert_eq!(call_l(&mut e, ACT1), 1.0, "an ingestible at +0x30");
    assert_eq!(call_l(&mut e, ACT2), 0.0, "not running");
    assert_eq!(call_l(&mut e, ACT3), 0.0, "MISC gives no magic item");
    assert_eq!(call_l(&mut e, NONACT), 0.0);
    assert_eq!(call_l(&mut e, 0), 0.0);
    assert_eq!(call_on(&mut e, 0x291a, PC, &[r(R_MISC)]).1, 0.0, "no read");
    assert_eq!(eval(&mut e, 0x291a, PC, FLST), 1.0);
    assert_eq!(eval(&mut e, 0x291a, ACT1, FLST), 1.0);
    assert_eq!(eval(&mut e, 0x291a, ACT2, FLST), 0.5, "false: left");
    assert_eq!(eval(&mut e, 0x291a, PC, 0), 0.5, "no list");
    assert_eq!(eval(&mut e, 0x291a, NONACT, FLST), 0.5);
    list(&mut e, FLST + 0x18, &[MISC]);
    assert_eq!(
        eval(&mut e, 0x291a, PC, FLST),
        0.5,
        "no magic item in the list"
    );
    list(&mut e, FLST + 0x18, &[ENCH, SPEL]);
    assert_eq!(eval(&mut e, 0x291a, PC, FLST), 1.0, "the tail");
    e.mem.set_u32(PC + 0x94 + 0x700, 0);
    assert_eq!(eval(&mut e, 0x291a, PC, FLST), 0.5, "no effect list");
}
