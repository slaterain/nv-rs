//! Tests of the inventory commands built here, at the raw offsets the module's doc lists, with
//! expectations read from JIP's source.

use super::*;
use crate::commands::test_support::*;
use nvse::vars::Data::Form;

const ACTOR: u32 = FORMS + 0x1000;
const CONT_REF: u32 = FORMS + 0x1200;
const NPC: u32 = FORMS + 0x1400;
const CONT: u32 = FORMS + 0x1800;
const WEAP: u32 = FORMS + 0x2000;
const ARMO: u32 = FORMS + 0x2400;
const ARMO2: u32 = FORMS + 0x2800;
const AMMO: u32 = FORMS + 0x2c00;
const ALCH: u32 = FORMS + 0x3000;
const POISON: u32 = FORMS + 0x3100;
const LIST: u32 = FORMS + 0x3200;
const MISC: u32 = FORMS + 0x3300;
const PROCESS: u32 = FORMS + 0x4000;
const ENTRY_W: u32 = FORMS + 0x4100;
const ENTRY_A: u32 = FORMS + 0x4140;
const XDL: u32 = FORMS + 0x4200;
const XDL2: u32 = FORMS + 0x4240;
const X1: u32 = FORMS + 0x4300;
const CHANGES_X: u32 = FORMS + 0x4400;
const CHANGES_DATA: u32 = FORMS + 0x4440;
const NODES: u32 = FORMS + 0x5000;
const ACTOR_VT: u32 = FORMS + 0x6000;
const BASE_VT: u32 = FORMS + 0x6400;
const TEMP_REF: u32 = FORMS + 0x7000;
const CAST_FN: u32 = 0x00ec_43fb;

fn setup() -> Engine {
    let mut e = engine();
    extra_stand_ins(&mut e);
    let forms = [
        (ACTOR, 0x3b, 0x201),
        (CONT_REF, 0x3a, 0x202),
        (NPC, 0x2a, 0x203),
        (CONT, 0x1b, 0x204),
        (WEAP, 0x28, 0x205),
        (ARMO, 0x18, 0x206),
        (ARMO2, 0x18, 0x207),
        (AMMO, 0x29, 0x208),
        (ALCH, 0x2f, 0x209),
        (POISON, 0x2f, 0x20a),
        (LIST, 0x55, 0x20b),
        (MISC, 0x1f, 0x20c),
    ];
    for (i, &(at, kind, id)) in forms.iter().enumerate() {
        form(&mut e, at, kind, id);
        set_ref(&mut e, i as u16 + 1, at);
    }
    // Every other form's vtable answers false everywhere (an empty table in test memory).
    for at in [CONT_REF, CONT, WEAP, AMMO, ALCH, POISON, LIST, MISC] {
        e.mem.set_u32(at, FORMS + 0x6800);
    }
    e.mem.set_u32(ARMO, VTBL_ARMOR);
    e.mem.set_u32(ARMO2, VTBL_ARMOR);
    // The actor: IS_ACTOR (+0x100 answers true), its base an NPC (an actor base: +0xF8 true).
    e.mem.set_u32(ACTOR, ACTOR_VT);
    e.mem.set_u32(ACTOR_VT + 0x100, 0x008d_0360);
    e.mem.set_u32(ACTOR + 0x20, NPC);
    e.mem.set_u32(NPC, BASE_VT);
    e.mem.set_u32(CONT_REF + 0x20, CONT);
    // `__RTDynamicCast` to `TESValueForm`: weapons and armour (and, for the base-form test, the
    // container) have it at +0x98.
    e.register(CAST_FN, |e, w| {
        assert_eq!((w[2], w[3]), (RTTI_TES_FORM, RTTI_TES_VALUE_FORM));
        let kind = e.mem.u8(w[0] + 4);
        Ret {
            eax: if kind == 0x28 || kind == 0x18 || kind == 0x1b {
                w[0] + 0x98
            } else {
                0
            },
            ..Ret::default()
        }
    });
    e
}

fn idx(at: u32) -> u16 {
    [
        ACTOR, CONT_REF, NPC, CONT, WEAP, ARMO, ARMO2, AMMO, ALCH, POISON, LIST, MISC,
    ]
    .iter()
    .position(|&f| f == at)
    .expect("a set-up form") as u16
        + 1
}

fn rf(at: u32) -> Vec<u8> {
    r(idx(at))
}

/// A list of `items` from the node at `at` (inline first node), more nodes at `nodes`.
fn put_list(e: &mut Engine, at: u32, nodes: u32, items: &[u32]) {
    e.mem.set_u32(at, 0);
    e.mem.set_u32(at + 4, 0);
    let mut node = at;
    for (i, &item) in items.iter().enumerate() {
        if i > 0 {
            let next = nodes + 8 * i as u32;
            e.mem.set_u32(node + 4, next);
            node = next;
            e.mem.set_u32(node + 4, 0);
        }
        e.mem.set_u32(node, item);
    }
}

/// An inventory reference (the built-in xNVSE's) for a stack with the extra data list `xdata`.
fn inv_ref(e: &mut Engine, item: u32, xdata: u32) -> u32 {
    e.register(0x0040_1000, |_, _| Ret {
        eax: TEMP_REF,
        ..Ret::default()
    });
    e.register(0x0055_a2f0, |e, w| {
        e.mem.set_u32(w[0] + 0xc, 0xff00_0800);
        Ret::default()
    });
    nvse::invref::create(
        e,
        ACTOR,
        nvse::invref::Data {
            item,
            entry: 0,
            xdata,
        },
        false,
    )
}

#[test]
fn values() {
    let mut e = setup();
    e.mem.set_u32(WEAP + 0x98 + 4, 250);
    e.mem.set_u32(ALCH + 0xb8, 0xffff_fff6);
    assert_eq!(call(&mut e, 0x2607, &[rf(WEAP)]).1, 250.0);
    assert_eq!(call(&mut e, 0x2607, &[rf(ALCH)]).1, -10.0, "an int");
    assert_eq!(
        call(&mut e, 0x2607, &[rf(MISC)]).1,
        0.0,
        "no value component"
    );
    // The calling reference's base when it is an inventory item.
    e.mem.set_u32(CONT_REF + 0x20, WEAP);
    assert_eq!(call_on(&mut e, 0x2607, CONT_REF, &[]).1, 250.0);
    e.mem.set_u32(CONT_REF + 0x20, CONT);
    // The stand-in cast gives the container a value too: as the base it is still no item.
    e.mem.set_u32(CONT + 0x98 + 4, 77);
    assert_eq!(
        call_on(&mut e, 0x2607, CONT_REF, &[]).1,
        0.0,
        "a container is no item"
    );
    call(&mut e, 0x26ab, &[rf(ARMO), n(99)]);
    assert_eq!(e.mem.u32(ARMO + 0x98 + 4), 99);
    call(&mut e, 0x26ab, &[rf(ALCH), n(5)]);
    assert_eq!(e.mem.u32(ALCH + 0xb8), 5);
    e.mem.set_u32(MISC + 0xb8, 0xa5a5_a5a5);
    call(&mut e, 0x26ab, &[rf(MISC), n(5)]);
    assert_eq!(e.mem.u32(MISC + 0xb8), 0xa5a5_a5a5, "neither");
}

#[test]
fn unique_items() {
    let mut e = setup();
    assert_eq!(call(&mut e, 0x2934, &[rf(WEAP)]).1, 0.0);
    e.mem.set_u8(WEAP + 6, 0x10);
    assert_eq!(call(&mut e, 0x2934, &[rf(WEAP)]).1, 1.0);
    e.mem.set_u8(WEAP + 6, 0xef);
    assert_eq!(call(&mut e, 0x2934, &[rf(WEAP)]).1, 0.0, "another bit");
}

#[test]
fn base_items() {
    let mut e = setup();
    let (c1, c2) = (FORMS + 0x8000, FORMS + 0x8010);
    e.mem.set_u32(c1 + 4, WEAP);
    e.mem.set_u32(c2 + 4, 0);
    put_list(&mut e, CONT + 0x34, NODES, &[c1, 0, c2]);
    let (_, res) = call(&mut e, 0x28a7, &[rf(CONT)]);
    assert_eq!(array_data(&e, res), vec![Form(0x205), Form(0)]);
    let (_, res) = call_on(&mut e, 0x28a7, CONT_REF, &[]);
    assert_eq!(array_data(&e, res), vec![Form(0x205), Form(0)], "the base");
    put_list(&mut e, CONT + 0x34, NODES, &[]);
    e.mem.set_f64(RESULT, 0.0);
    assert_eq!(
        call(&mut e, 0x28a7, &[rf(CONT)]).1,
        0.0,
        "no entries: no array"
    );
    assert_eq!(call(&mut e, 0x28a7, &[rf(WEAP)]).1, 0.0, "no container");
}

#[test]
fn weapon_poison() {
    let mut e = setup();
    e.mem.set_u32(ACTOR + 0x68, PROCESS);
    e.mem.set_u32(PROCESS + 0x114, ENTRY_W);
    e.mem.set_u32(ENTRY_W + 8, WEAP);
    let lists = FORMS + 0x8100;
    e.mem.set_u32(ENTRY_W, lists);
    put_list(&mut e, lists, NODES, &[XDL]);
    call_on(&mut e, 0x288f, ACTOR, &[]);
    assert_eq!(form_result(&e), 0, "no poison");
    put_extra(&mut e, XDL, X1, 0x3f, POISON);
    call_on(&mut e, 0x288f, ACTOR, &[]);
    assert_eq!(form_result(&e), 0x20a);
    e.mem.set_u32(RESULT, 0);
    e.mem.set_u8(PROCESS + 0x28, 2);
    call_on(&mut e, 0x288f, ACTOR, &[]);
    assert_eq!(form_result(&e), 0, "a low process has no weapon info");
    e.mem.set_u8(PROCESS + 0x28, 1);
    call_on(&mut e, 0x288f, CONT_REF, &[]);
    assert_eq!(form_result(&e), 0, "not an actor");
}

#[test]
fn weapon_mod_flags_of_an_inventory_reference_or_a_reference() {
    let mut e = setup();
    put_extra(&mut e, CONT_REF + 0x44, X1, 0x8d, 0);
    e.mem.set_u8(X1 + 0xc, 5);
    assert_eq!(
        call_on(&mut e, 0x270c, CONT_REF, &[]).1,
        5.0,
        "its own extra data"
    );
    let x2 = FORMS + 0x4380;
    put_extra(&mut e, XDL, x2, 0x8d, 0);
    e.mem.set_u8(x2 + 0xc, 3);
    let refr = inv_ref(&mut e, WEAP, XDL);
    assert_eq!(
        call_on(&mut e, 0x270c, refr, &[]).1,
        3.0,
        "the stack's list"
    );
}

#[test]
fn no_unequip_marks() {
    let mut e = setup();
    let refr = inv_ref(&mut e, ARMO, XDL);
    assert_eq!(call_on(&mut e, 0x2864, refr, &[]).1, 0.0);
    // Not worn: no mark put on.
    call_on(&mut e, 0x2865, refr, &[n(1)]);
    assert!(!extra::has_type(&e, XDL, 0x3e));
    assert!(allocations().is_empty());
    // Worn: a new ExtraCannotWear (0xC bytes, its vtable, type 0x3E).
    e.mem.set_u8(XDL + 8 + 2, 0x40);
    call_on(&mut e, 0x2865, refr, &[n(1)]);
    assert!(extra::has_type(&e, XDL, 0x3e));
    assert_eq!(allocations(), vec![0xc]);
    let made = calls_to(ADD_EXTRA_FN)[0][1];
    assert_eq!((e.mem.u32(made), e.mem.u32(made + 4)), (0x0101_5bf4, 0x3e));
    assert_eq!(call_on(&mut e, 0x2864, refr, &[]).1, 1.0);
    // Already marked: nothing added.
    call_on(&mut e, 0x2865, refr, &[n(1)]);
    assert_eq!(allocations(), vec![0xc]);
    call_on(&mut e, 0x2865, refr, &[n(0)]);
    assert!(!extra::has_type(&e, XDL, 0x3e));
    assert_eq!(calls_to(REMOVE_BY_TYPE_FN), vec![vec![XDL, 0x3e]]);
    // A plain reference is no inventory reference: nothing.
    e.mem.set_u8(CONT_REF + 0x44 + 8 + 7, 0x40);
    assert_eq!(call_on(&mut e, 0x2864, CONT_REF, &[]).1, 0.0);
}

/// The actor's inventory: the NPC's container (actor base: +0x64) with a weapon and ammunition,
/// the changes with an armour (worn) and a quest item.
fn inventory(e: &mut Engine) {
    e.mem.set_u32(BASE_VT + 0xf8, 0x008d_0360);
    let (c1, c2) = (FORMS + 0x8000, FORMS + 0x8010);
    e.mem.set_u32(c1, 1);
    e.mem.set_u32(c1 + 4, WEAP);
    e.mem.set_u32(c2, 20);
    e.mem.set_u32(c2 + 4, AMMO);
    put_list(e, NPC + 0x68, NODES, &[c1, c2]);
    put_extra(e, ACTOR + 0x44, CHANGES_X, 0x15, CHANGES_DATA);
    let entries = FORMS + 0x8200;
    e.mem.set_u32(CHANGES_DATA, entries);
    put_list(e, entries, NODES + 0x100, &[ENTRY_A, ENTRY_W]);
    e.mem.set_u32(ENTRY_A + 4, 1);
    e.mem.set_u32(ENTRY_A + 8, ARMO);
    let lists = FORMS + 0x8300;
    e.mem.set_u32(ENTRY_A, lists);
    put_list(e, lists, NODES + 0x200, &[XDL2, XDL]);
    e.mem.set_u8(XDL + 8 + 2, 0x40);
    e.mem.set_u32(ENTRY_W + 4, 1);
    e.mem.set_u32(ENTRY_W + 8, MISC);
    e.mem.set_u32(MISC + 8, 0x400);
}

#[test]
fn all_items_in_address_order_with_filters() {
    let mut e = setup();
    inventory(&mut e);
    let (_, res) = call_on(&mut e, 0x2766, ACTOR, &[]);
    assert_eq!(
        array_data(&e, res),
        vec![Form(0x205), Form(0x206), Form(0x208), Form(0x20c)]
    );
    let (_, res) = call_on(&mut e, 0x2766, ACTOR, &[n(0x18)]);
    assert_eq!(array_data(&e, res), vec![Form(0x206)], "armour only");
    e.mem.set_u8(ARMO + 0x78, 0x40);
    e.mem.set_u8(WEAP + 0x100, 0x80);
    let (_, res) = call_on(&mut e, 0x2766, ACTOR, &[n(0), n(1)]);
    assert_eq!(
        array_data(&e, res),
        vec![Form(0x208), Form(0x20c)],
        "playable"
    );
    let (_, res) = call_on(&mut e, 0x2766, ACTOR, &[n(0), n(0), n(1)]);
    assert_eq!(
        array_data(&e, res),
        vec![Form(0x205), Form(0x206), Form(0x208)],
        "no quest item"
    );
    let (_, res) = call_on(&mut e, 0x2766, ACTOR, &[n(0), n(0), n(0), n(1)]);
    assert_eq!(
        array_data(&e, res),
        vec![Form(0x205), Form(0x208), Form(0x20c)],
        "the worn armour left out"
    );
    // Its stacks without the worn mark: not equipped.
    e.mem.set_u8(XDL + 8 + 2, 0);
    let (_, res) = call_on(&mut e, 0x2766, ACTOR, &[n(0), n(0), n(0), n(1)]);
    assert_eq!(
        array_data(&e, res),
        vec![Form(0x205), Form(0x206), Form(0x208), Form(0x20c)],
        "stacks, none worn"
    );
    e.mem.set_u8(XDL + 8 + 2, 0x40);
    // The held weapon counts as equipped.
    e.mem.set_u32(ACTOR + 0x68, PROCESS);
    let held = FORMS + 0x8400;
    e.mem.set_u32(PROCESS + 0x114, held);
    e.mem.set_u32(held + 8, WEAP);
    let (_, res) = call_on(&mut e, 0x2766, ACTOR, &[n(0), n(0), n(0), n(1)]);
    assert_eq!(array_data(&e, res), vec![Form(0x208), Form(0x20c)]);
    // A type that is no inventory item: nothing.
    e.mem.set_f64(RESULT, 0.0);
    assert_eq!(call_on(&mut e, 0x2766, ACTOR, &[n(0x39)]).1, 0.0);
    assert_eq!(call_on(&mut e, 0x2766, ACTOR, &[n(500)]).1, 0.0);
    assert_eq!(
        call_on(&mut e, 0x2766, ACTOR, &[n(0x2f)]).1,
        0.0,
        "none of it"
    );
    // A changes entry of a type that is no inventory item (an NPC form): asked for by that type,
    // still nothing (the type test comes first).
    let entry_n = FORMS + 0x4180;
    e.mem.set_u32(entry_n + 4, 1);
    e.mem.set_u32(entry_n + 8, NPC);
    put_list(
        &mut e,
        FORMS + 0x8200,
        NODES + 0x100,
        &[ENTRY_A, ENTRY_W, entry_n],
    );
    assert_eq!(call_on(&mut e, 0x2766, ACTOR, &[n(0x2a)]).1, 0.0);
}

#[test]
fn all_items_into_a_form_list() {
    let mut e = setup();
    inventory(&mut e);
    put_list(&mut e, LIST + 0x18, NODES + 0x300, &[ALCH]);
    let (_, res) = call_on(&mut e, 0x2766, ACTOR, &[n(0), n(0), n(1), n(0), rf(LIST)]);
    assert_eq!(res, 3.0, "the count");
    // Emptied, then each put first: the reverse of the map's order.
    assert_eq!(
        list_items(&e, Ptr::new(LIST + 0x18)),
        vec![AMMO, ARMO, WEAP]
    );
}

#[test]
fn all_items_of_a_non_actor_ignore_equipped() {
    let mut e = setup();
    let c1 = FORMS + 0x8000;
    e.mem.set_u32(c1, 1);
    e.mem.set_u32(c1 + 4, ARMO);
    put_list(&mut e, CONT + 0x34, NODES, &[c1]);
    put_extra(&mut e, CONT_REF + 0x44, CHANGES_X, 0x15, CHANGES_DATA);
    let entries = FORMS + 0x8200;
    e.mem.set_u32(CHANGES_DATA, entries);
    put_list(&mut e, entries, NODES + 0x100, &[ENTRY_A]);
    e.mem.set_u32(ENTRY_A + 8, ARMO);
    let lists = FORMS + 0x8300;
    e.mem.set_u32(ENTRY_A, lists);
    put_list(&mut e, lists, NODES + 0x200, &[XDL]);
    e.mem.set_u8(XDL + 8 + 2, 0x40);
    let (_, res) = call_on(&mut e, 0x2766, CONT_REF, &[n(0), n(0), n(0), n(1)]);
    assert_eq!(array_data(&e, res), vec![Form(0x206)]);
}
