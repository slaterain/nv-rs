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
    // The test heap fills a new block with 0xCD: the next word must be written 0.
    assert_eq!(e.mem.u32(made + 8), 0, "next");
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
    // One form added by a script (BGSListForm::numAddedObjects): RemoveAll zeroes the count.
    e.mem.set_u32(LIST + 0x20, 1);
    let (_, res) = call_on(&mut e, 0x2766, ACTOR, &[n(0), n(0), n(1), n(0), rf(LIST)]);
    assert_eq!(res, 3.0, "the count");
    // Emptied, then each put first: the reverse of the map's order.
    assert_eq!(
        list_items(&e, Ptr::new(LIST + 0x18)),
        vec![AMMO, ARMO, WEAP]
    );
    assert_eq!(e.mem.u32(LIST + 0x20), 0, "the script-added count");
}

#[test]
fn all_items_of_a_creature_keep_its_worn_armor() {
    // Actor::IsItemEquipped: an armour is never equipped on a creature reference (type 0x3C),
    // worn or not.
    let mut e = setup();
    inventory(&mut e);
    e.mem.set_u8(ACTOR + 4, 0x3c);
    let (_, res) = call_on(&mut e, 0x2766, ACTOR, &[n(0), n(0), n(0), n(1)]);
    assert_eq!(
        array_data(&e, res),
        vec![Form(0x205), Form(0x206), Form(0x208), Form(0x20c)]
    );
}

#[test]
fn weapon_mod_flags_of_an_inventory_reference_without_a_stack_list() {
    // JIP takes the inventory reference's xData (null here) and never its own extra data.
    let mut e = setup();
    let refr = inv_ref(&mut e, WEAP, 0);
    put_extra(&mut e, refr + 0x44, X1, 0x8d, 0);
    e.mem.set_u8(X1 + 0xc, 7);
    e.mem.set_f64(RESULT, 0.0);
    assert_eq!(call_on(&mut e, 0x270c, refr, &[]).1, 0.0);
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

// ---- lane B2: inventory references and the rest ----

const ALLOC: u32 = FORMS + 0x9000;
const ALLOC_AREA: u32 = FORMS + 0xa0000;
const DT_FN: u32 = 0x004b_e0b0;
const DR_FN: u32 = 0x004b_df90;
const BIPED_AT: u32 = FORMS + 0x8800;

fn on_entry(e: &Engine, entry: u32) -> f64 {
    f64::from(e.mem.u32(entry + 4)) + f64::from(e.mem.u32(e.mem.u32(entry)) & 1)
}

/// The form heap as a bump allocator and fresh reference ids, so every inventory reference has
/// its own temporary reference and changes entry; the free a no-op.
fn ref_heap(e: &mut Engine) {
    e.mem.set_u32(ALLOC, ALLOC_AREA);
    e.register(0x0040_1000, |e, w| {
        let at = e.mem.u32(ALLOC);
        e.mem.set_u32(ALLOC, at + w[0].next_multiple_of(0x10));
        Ret {
            eax: at,
            ..Ret::default()
        }
    });
    e.register(0x0040_1030, |_, _| Ret::default());
    e.register(0x0055_a2f0, |e, w| {
        let n = e.mem.u32(ALLOC + 4) + 1;
        e.mem.set_u32(ALLOC + 4, n);
        e.mem.set_u32(w[0] + 0xc, 0xff00_0000 + n);
        Ret {
            eax: w[0],
            ..Ret::default()
        }
    });
}

/// An inventory reference's (container, item, count, stack): its entry's count and item.
fn ir(e: &mut Engine, refr: u32) -> (u32, u32, i32, u32) {
    let id = crate::commands::ref_id(e, refr);
    let r = nvse::invref::get(e, id).expect("an inventory reference");
    let (container, d) = (r.container, r.data);
    let count = e.mem.u32(d.entry + 4) as i32;
    (container, e.mem.u32(d.entry + 8), count, d.xdata)
}

/// The array result's forms as addresses (the form ids of the temporary references).
fn ref_ids(e: &Engine, res: f64) -> Vec<u32> {
    array_data(e, res)
        .into_iter()
        .map(|d| match d {
            Form(id) => id,
            other => panic!("not a form: {other:?}"),
        })
        .collect()
}

/// The temporary reference of an id (the form id is 0xFF00_0000 + n, the n-th made).
fn by_id(e: &mut Engine, id: u32) -> u32 {
    nvse::invref::get_ref(e, id)
}

#[test]
fn all_item_refs_one_per_stack() {
    let mut e = setup();
    ref_heap(&mut e);
    inventory(&mut e);
    // The armour: 3 in all, a stack of 2 (ExtraCount) and the worn one; the rest without data.
    e.mem.set_u32(ENTRY_A + 4, 3);
    put_extra(&mut e, XDL2, X1, 0x24, 0);
    e.mem.set_u16(X1 + 0xc, 2);
    e.mem.set_u8(ARMO + 0x78, 0);
    let (_, res) = call_on(&mut e, 0x2767, ACTOR, &[n(0x18)]);
    let ids = ref_ids(&e, res);
    assert_eq!(ids.len(), 2, "the two stacks cover the count");
    let refs: Vec<_> = ids.iter().map(|&id| by_id(&mut e, id)).collect();
    assert_eq!(ir(&mut e, refs[0]), (ACTOR, ARMO, 2, XDL2));
    assert_eq!(ir(&mut e, refs[1]), (ACTOR, ARMO, 1, XDL));
    // 4 in all: one more without extra data.
    e.mem.set_u32(ENTRY_A + 4, 4);
    let (_, res) = call_on(&mut e, 0x2767, ACTOR, &[n(0x18)]);
    let ids = ref_ids(&e, res);
    let last = by_id(&mut e, ids[2]);
    assert_eq!(ir(&mut e, last), (ACTOR, ARMO, 1, 0), "the rest");
    // No equipped: the worn stack passed over, its count still taken.
    let (_, res) = call_on(&mut e, 0x2767, ACTOR, &[n(0x18), n(0), n(0), n(1)]);
    let ids = ref_ids(&e, res);
    assert_eq!(ids.len(), 2);
    let second = by_id(&mut e, ids[1]);
    assert_eq!(ir(&mut e, second), (ACTOR, ARMO, 1, 0));
    // 1 in all: the first stack takes it (at most what is left), the worn one is not reached.
    e.mem.set_u32(ENTRY_A + 4, 1);
    let (_, res) = call_on(&mut e, 0x2767, ACTOR, &[n(0x18)]);
    let ids = ref_ids(&e, res);
    assert_eq!(ids.len(), 1);
    let only = by_id(&mut e, ids[0]);
    assert_eq!(ir(&mut e, only), (ACTOR, ARMO, 1, XDL2));
}

#[test]
fn all_item_refs_into_a_form_list_and_filters() {
    let mut e = setup();
    ref_heap(&mut e);
    inventory(&mut e);
    put_list(&mut e, LIST + 0x18, NODES + 0x300, &[ALCH]);
    e.mem.set_u32(LIST + 0x20, 1);
    let (_, res) = call_on(&mut e, 0x2767, ACTOR, &[n(0), n(0), n(1), n(0), rf(LIST)]);
    // Weapon, armour (two stacks: XDL2 has no count of its own, so 1 each... the entry's count
    // is 1, the first stack takes it), ammunition; the quest item left out.
    assert_eq!(res, 3.0, "the count");
    assert_eq!(e.mem.u32(LIST + 0x20), 0, "the script-added count");
    let items: Vec<u32> = list_items(&e, Ptr::new(LIST + 0x18));
    assert_eq!(items.len(), 3);
    let kinds: Vec<u32> = items.iter().map(|&r| ir(&mut e, r).1).collect();
    assert_eq!(kinds, vec![AMMO, ARMO, WEAP], "each put first");
    // Unplayable left out.
    e.mem.set_u8(WEAP + 0x100, 0x80);
    e.mem.set_f64(RESULT, 0.0);
    let (_, res) = call_on(&mut e, 0x2767, ACTOR, &[n(0x28), n(1)]);
    assert_eq!(res, 0.0, "no array without references");
}

/// The actor's biped data: slot 2 the armour (upper body, part mask 4), slot 3 the same armour.
fn biped(e: &mut Engine) -> u32 {
    let equipment = BIPED_AT;
    e.mem.set_u32(ACTOR_VT + 0x1e8, 0x0040_2000);
    e.register(0x0040_2000, |_, _| Ret {
        eax: BIPED_AT,
        ..Ret::default()
    });
    e.mem.set_u32(equipment + 0x2c + 0x10 * 2, ARMO);
    e.mem.set_u32(equipment + 0x2c + 0x10 * 3, ARMO);
    e.mem.set_u32(equipment + 0x2c + 0x10 * 5, WEAP);
    e.mem.set_u32(equipment + 0x2c + 0x10 * 6, MISC);
    e.mem.set_u32(ARMO + 0x74, 4);
    equipment
}

#[test]
fn equipped_armor_refs_per_slot() {
    let mut e = setup();
    ref_heap(&mut e);
    inventory(&mut e);
    biped(&mut e);
    // The quest item (no armour) in slot 6 with a worn stack too: passed over.
    e.mem.set_u32(ENTRY_W, FORMS + 0x8300);
    // The entry holds 3, the worn stack no count of its own: the reference gets the entry's 3.
    e.mem.set_u32(ENTRY_A + 4, 3);
    let (_, res) = call_on(&mut e, 0x28d1, ACTOR, &[]);
    let ids = ref_ids(&e, res);
    assert_eq!(
        ids.len(),
        2,
        "the armour in two slots; the weapon and the quest item are no armour"
    );
    let r0 = by_id(&mut e, ids[0]);
    assert_eq!(
        ir(&mut e, r0),
        (ACTOR, ARMO, 3, XDL),
        "its worn stack, the entry's count"
    );
    // Not worn: an empty array still.
    e.mem.set_u8(XDL + 8 + 2, 0);
    let (_, res) = call_on(&mut e, 0x28d1, ACTOR, &[]);
    assert!(array_data(&e, res).is_empty());
    e.mem.set_f64(RESULT, 0.0);
    assert_eq!(
        call_on(&mut e, 0x28d1, CONT_REF, &[]).1,
        0.0,
        "not an actor"
    );
}

#[test]
fn equipped_item_ref_with_and_without_3d() {
    let mut e = setup();
    ref_heap(&mut e);
    inventory(&mut e);
    let equipment = biped(&mut e);
    // Without 3D: the changes entries; slot 2 the worn armour, with the entry's count (3).
    e.mem.set_u32(ENTRY_A + 4, 3);
    call_on(&mut e, 0x2839, ACTOR, &[n(2)]);
    let id = form_result(&e);
    let made = by_id(&mut e, id);
    assert_eq!(ir(&mut e, made), (ACTOR, ARMO, 3, XDL));
    e.mem.set_u32(RESULT, 0);
    call_on(&mut e, 0x2839, ACTOR, &[n(4)]);
    assert_eq!(form_result(&e), 0, "the armour does not cover slot 4");
    call_on(&mut e, 0x2839, ACTOR, &[n(20)]);
    assert_eq!(form_result(&e), 0, "past slot 19");
    // With 3D: the biped slots; the weapon slot reads the held weapon's entry.
    let loaded = FORMS + 0x8900;
    e.mem.set_u32(ACTOR + 0x64, loaded);
    e.mem.set_u32(loaded + 0x14, FORMS + 0x8a00);
    call_on(&mut e, 0x2839, ACTOR, &[n(2)]);
    let id = form_result(&e);
    let made = by_id(&mut e, id);
    assert_eq!(ir(&mut e, made), (ACTOR, ARMO, 3, XDL));
    e.mem.set_u32(RESULT, 0);
    e.mem.set_u32(equipment + 0x2c + 0x10 * 2, 0);
    e.mem.set_u32(equipment + 0x2c + 0x10 * 3, 0);
    call_on(&mut e, 0x2839, ACTOR, &[n(2)]);
    assert_eq!(form_result(&e), 0, "no armour in the biped slots");
    e.mem.set_u32(ACTOR + 0x68, PROCESS);
    let held = FORMS + 0x8400;
    e.mem.set_u32(PROCESS + 0x114, held);
    e.mem.set_u32(held + 4, 4);
    e.mem.set_u32(held + 8, WEAP);
    let lists = FORMS + 0x8b00;
    e.mem.set_u32(held, lists);
    put_list(&mut e, lists, NODES + 0x400, &[XDL2]);
    call_on(&mut e, 0x2839, ACTOR, &[n(5)]);
    let id = form_result(&e);
    let made = by_id(&mut e, id);
    assert_eq!(
        ir(&mut e, made),
        (ACTOR, WEAP, 4, XDL2),
        "the held weapon's first stack, its entry's count"
    );
}

#[test]
fn hotkey_item_ref_of_the_player() {
    let mut e = setup();
    ref_heap(&mut e);
    inventory(&mut e);
    e.mem.set_u32(0x011d_ea3c, ACTOR);
    put_extra(&mut e, XDL2, X1, 0x4a, 3);
    // The stack holds 2 (ExtraCount) of the entry's 3: the reference gets the stack's 2.
    e.mem.set_u32(ENTRY_A + 4, 3);
    put_extra(&mut e, XDL2, FORMS + 0x4380, 0x24, 0);
    e.mem.set_u16(FORMS + 0x4380 + 0xc, 2);
    call(&mut e, 0x28f1, &[n(4)]);
    let id = form_result(&e);
    let made = by_id(&mut e, id);
    assert_eq!(
        ir(&mut e, made),
        (ACTOR, ARMO, 2, XDL2),
        "hotkey 4 is index 3; the stack's count"
    );
    e.mem.set_u32(RESULT, 0);
    call(&mut e, 0x28f1, &[n(3)]);
    assert_eq!(form_result(&e), 0, "no stack on hotkey 3");
    call(&mut e, 0x28f1, &[n(0)]);
    assert_eq!(form_result(&e), 0);
    call(&mut e, 0x28f1, &[n(9)]);
    assert_eq!(form_result(&e), 0);
}

#[test]
fn self_as_inventory_ref_by_the_running_locals() {
    let mut e = setup();
    ref_heap(&mut e);
    inventory(&mut e);
    // The armour's two stacks: XDL2 has a script with other locals, XDL with the running ones.
    let (x2, x3) = (FORMS + 0x4380, FORMS + 0x43c0);
    put_extra(&mut e, XDL2, x2, 0x0d, 0);
    e.mem.set_u32(x2 + 0x10, 0x7777);
    put_extra(&mut e, XDL, x3, 0x0d, 0);
    e.mem.set_u32(x3 + 0x10, EVENTS);
    e.mem.set_u32(TEMP_REF + 0x20, ARMO);
    // The running stack's count 0, then -1, in an entry of 3: at least 1, not the entry's.
    e.mem.set_u32(ENTRY_A + 4, 3);
    let count = FORMS + 0x43e0;
    put_extra(&mut e, XDL, count, 0x24, 0);
    for n in [0u16, 0xffff] {
        e.mem.set_u16(count + 0xc, n);
        call_in(&mut e, 0x2936, TEMP_REF, ACTOR, &[]);
        let id = form_result(&e);
        let made = by_id(&mut e, id);
        assert_eq!(ir(&mut e, made), (ACTOR, ARMO, 1, XDL), "count {n}: 1");
    }
    // A count of 2: the stack's.
    e.mem.set_u16(count + 0xc, 2);
    call_in(&mut e, 0x2936, TEMP_REF, ACTOR, &[]);
    let id = form_result(&e);
    let made = by_id(&mut e, id);
    assert_eq!(ir(&mut e, made), (ACTOR, ARMO, 2, XDL));
    e.mem.set_u32(RESULT, 0);
    call_in(&mut e, 0x2936, TEMP_REF, 0, &[]);
    assert_eq!(form_result(&e), 0, "not inside a container");
}

const EVENTS: u32 = FORMS + 0x9100;

/// A call from an item's script inside `containing` (the handler's `containingObj` and
/// `eventList` words set too).
fn call_in(e: &mut Engine, opcode: u32, this: u32, containing: u32, args: &[Vec<u8>]) {
    let mut words = crate::test_support::line(e, opcode, args);
    words[2] = this;
    words[3] = containing;
    words[5] = EVENTS;
    let execute = crate::test_support::execute_of(e, opcode);
    e.call(execute, &words);
}

#[test]
fn item_ref_current_health() {
    let mut e = setup();
    ref_heap(&mut e);
    // A plain reference of a weapon: health 200, a health mod in slot 2 (+50) when attached.
    e.mem.set_u32(CONT_REF + 0x20, WEAP);
    e.mem.set_u32(WEAP + 0x98, 200);
    e.mem.set_u32(WEAP + 0x184, 10);
    e.mem.set_f32(WEAP + 0x190, 50.0);
    assert_eq!(
        call_on(&mut e, 0x270f, CONT_REF, &[]).1,
        200.0,
        "no ExtraHealth: full"
    );
    assert_eq!(call_on(&mut e, 0x270f, CONT_REF, &[n(1)]).1, 1.0);
    put_extra(&mut e, CONT_REF + 0x44, X1, 0x25, 0);
    e.mem.set_f32(X1 + 0xc, 125.0);
    let x2 = FORMS + 0x4380;
    put_extra(&mut e, CONT_REF + 0x44, x2, 0x8d, 0);
    e.mem.set_u8(x2 + 0xc, 2);
    assert_eq!(call_on(&mut e, 0x270f, CONT_REF, &[]).1, 125.0);
    assert_eq!(call_on(&mut e, 0x270f, CONT_REF, &[n(1)]).1, 0.5, "of 250");
    assert_eq!(
        call_on(&mut e, 0x270f, CONT_REF, &[n(2)]).1,
        250.0,
        "the mod adds"
    );
    e.mem.set_u8(x2 + 0xc, 1);
    assert_eq!(
        call_on(&mut e, 0x270f, CONT_REF, &[n(2)]).1,
        200.0,
        "slot 1 has no health mod"
    );
    // An armour inventory reference: its entry's item and stack.
    e.mem.set_u32(ARMO + 0x6c, 40);
    put_extra(&mut e, XDL, FORMS + 0x43c0, 0x25, 0);
    e.mem.set_f32(FORMS + 0x43c0 + 0xc, 10.0);
    let refr = nvse::invref::create_entry(&mut e, ACTOR, ARMO, 1, XDL);
    assert_eq!(call_on(&mut e, 0x270f, refr, &[n(1)]).1, 0.25);
    assert_eq!(call_on(&mut e, 0x270f, refr, &[n(2)]).1, 40.0);
    // Neither a weapon nor an armour: no full health.
    e.mem.set_u32(CONT_REF + 0x20, MISC);
    assert_eq!(call_on(&mut e, 0x270f, CONT_REF, &[n(2)]).1, 0.0);
}

#[test]
fn armor_effective_dt_and_dr() {
    let mut e = setup();
    ref_heap(&mut e);
    // On the entry: its count and its first list (the low bit).
    // Both take the entry and JIP's 0 (`RET 4`; the translations read two words).
    e.register(DT_FN, |e, w| {
        assert_eq!(w.len(), 2, "the entry and one word");
        assert_eq!(w[1], 0);
        Ret {
            st0: 12.5 + on_entry(e, w[0]),
            ..Ret::default()
        }
    });
    e.register(DR_FN, |e, w| {
        assert_eq!(w.len(), 2, "the entry and one word");
        assert_eq!(w[1], 0);
        Ret {
            st0: 30.0 + on_entry(e, w[0]),
            ..Ret::default()
        }
    });
    e.mem.set_u32(CONT_REF + 0x20, ARMO);
    assert_eq!(
        call_on(&mut e, 0x28e5, CONT_REF, &[]).1,
        13.5,
        "a temporary entry, count 1"
    );
    assert_eq!(call_on(&mut e, 0x28e6, CONT_REF, &[]).1, 31.0);
    let refr = nvse::invref::create_entry(&mut e, ACTOR, ARMO, 3, XDL);
    e.mem.set_u32(refr + 0x20, ARMO);
    assert_eq!(
        call_on(&mut e, 0x28e5, refr, &[]).1,
        15.5,
        "the inventory reference's entry"
    );
    e.mem.set_u32(CONT_REF + 0x20, WEAP);
    e.mem.set_f64(RESULT, 9.0);
    assert_eq!(
        call_on(&mut e, 0x28e5, CONT_REF, &[]).1,
        0.0,
        "not armour: 0"
    );
}

thread_local! {
    static REMOVES: std::cell::RefCell<Vec<Vec<u32>>> = const { std::cell::RefCell::new(Vec::new()) };
}

#[test]
fn remove_item_target_stacks_then_the_rest() {
    let mut e = setup();
    inventory(&mut e);
    e.mem.set_u32(ACTOR_VT + 0x17c, 0x0040_3000);
    e.register(0x0040_3000, |e, w| {
        REMOVES.with(|r| r.borrow_mut().push(w[1..].to_vec()));
        // RemoveItem: the stack's list leaves the entry when whole (stand-in).
        let lists = e.mem.u32(ENTRY_A);
        if w[2] != 0 && e.mem.u32(lists) == w[2] {
            let next = e.mem.u32(lists + 4);
            let after = if next == 0 { 0 } else { e.mem.u32(next) };
            e.mem.set_u32(lists, after);
            e.mem
                .set_u32(lists + 4, if next == 0 { 0 } else { e.mem.u32(next + 4) });
        }
        Ret::default()
    });
    e.mem.set_u32(ENTRY_A + 4, 4);
    put_extra(&mut e, XDL2, X1, 0x24, 0);
    e.mem.set_u16(X1 + 0xc, 2);
    REMOVES.with(|r| r.borrow_mut().clear());
    call_on(&mut e, 0x2688, ACTOR, &[rf(ARMO), rf(CONT_REF)]);
    let removes = REMOVES.with(|r| r.borrow().clone());
    assert_eq!(
        removes,
        vec![
            vec![ARMO, XDL2, 2, 1, 0, CONT_REF, 0, 0, 1, 0],
            vec![ARMO, XDL, 1, 1, 0, CONT_REF, 0, 0, 1, 0],
            vec![ARMO, 0, 1, 1, 0, CONT_REF, 0, 0, 1, 0],
        ]
    );
}

#[test]
fn remove_item_target_through_a_list_with_a_quantity() {
    let mut e = setup();
    inventory(&mut e);
    e.mem.set_u32(ACTOR_VT + 0x17c, 0x0040_3000);
    e.register(0x0040_3000, |_, w| {
        REMOVES.with(|r| r.borrow_mut().push(w[1..].to_vec()));
        Ret::default()
    });
    REMOVES.with(|r| r.borrow_mut().clear());
    e.mem.set_u32(ENTRY_A + 4, 4);
    put_extra(&mut e, XDL2, X1, 0x24, 0);
    e.mem.set_u16(X1 + 0xc, 2);
    put_list(&mut e, LIST + 0x18, NODES + 0x300, &[0, ARMO]);
    // A form list (its empty node passed over), 1 of the armour, the owner cleared.
    call_on(&mut e, 0x2688, ACTOR, &[rf(LIST), rf(CONT_REF), n(1), n(1)]);
    assert_eq!(
        REMOVES.with(|r| r.borrow().clone()),
        vec![vec![ARMO, XDL2, 1, 0, 0, CONT_REF, 0, 0, 1, 0]]
    );
    // More than it holds (10 of 4): its 4, the stack of 2 twice (the stand-in leaves it first).
    REMOVES.with(|r| r.borrow_mut().clear());
    call_on(&mut e, 0x2688, ACTOR, &[rf(ARMO), rf(CONT_REF), n(10)]);
    assert_eq!(
        REMOVES.with(|r| r.borrow().clone()),
        vec![vec![ARMO, XDL2, 2, 1, 0, CONT_REF, 0, 0, 1, 0]; 2]
    );
}

#[test]
fn remove_item_target_a_stack_without_a_count_stays_first() {
    let mut e = setup();
    inventory(&mut e);
    e.mem.set_u32(ACTOR_VT + 0x17c, 0x0040_3000);
    e.register(0x0040_3000, |_, w| {
        REMOVES.with(|r| r.borrow_mut().push(w[1..].to_vec()));
        Ret::default()
    });
    REMOVES.with(|r| r.borrow_mut().clear());
    // 3 of the armour; its first stack's ExtraCount is 0 and RemoveItem leaves it first: JIP
    // removes 1 from it each time until the 3 are done.
    e.mem.set_u32(ENTRY_A + 4, 3);
    put_extra(&mut e, XDL2, X1, 0x24, 0);
    e.mem.set_u16(X1 + 0xc, 0);
    call_on(&mut e, 0x2688, ACTOR, &[rf(ARMO), rf(CONT_REF)]);
    assert_eq!(
        REMOVES.with(|r| r.borrow().clone()),
        vec![vec![ARMO, XDL2, 1, 1, 0, CONT_REF, 0, 0, 1, 0]; 3]
    );
    // A form list holding itself and the armour: the list walked once, the armour once.
    REMOVES.with(|r| r.borrow_mut().clear());
    e.mem.set_u32(ENTRY_A + 4, 1);
    put_list(&mut e, LIST + 0x18, NODES + 0x300, &[LIST, ARMO]);
    call_on(&mut e, 0x2688, ACTOR, &[rf(LIST), rf(CONT_REF)]);
    assert_eq!(
        REMOVES.with(|r| r.borrow().clone()),
        vec![vec![ARMO, XDL2, 1, 1, 0, CONT_REF, 0, 0, 1, 0]]
    );
}

#[test]
fn equipped_item_ref_first_covering_slot_weapon_and_null_forms() {
    let mut e = setup();
    ref_heap(&mut e);
    inventory(&mut e);
    let equipment = biped(&mut e);
    // A third stack list, worn, for the second entry.
    let xdl3 = FORMS + 0x4280;
    e.mem.set_u8(xdl3 + 8 + 2, 0x40);
    let lists = FORMS + 0x8340;
    put_list(&mut e, lists, NODES + 0x280, &[xdl3]);
    e.mem.set_u32(ENTRY_W, lists);
    // Without 3D, slot 5: the first weapon entry with a worn stack, the entry's count.
    e.mem.set_u32(ENTRY_W + 8, WEAP);
    e.mem.set_u32(ENTRY_W + 4, 6);
    // Loaded data with no 3D root is "without 3D".
    let loaded = FORMS + 0x8900;
    e.mem.set_u32(ACTOR + 0x64, loaded);
    e.mem.set_u32(loaded + 0x14, 0);
    call_on(&mut e, 0x2839, ACTOR, &[n(5)]);
    let id = form_result(&e);
    let made = by_id(&mut e, id);
    assert_eq!(ir(&mut e, made), (ACTOR, WEAP, 6, xdl3));
    // An entry with lists but no form first: passed over, the worn armour found after it.
    let null_entry = FORMS + 0x4180;
    e.mem.set_u32(null_entry, lists);
    e.mem.set_u32(null_entry + 4, 1);
    e.mem.set_u32(null_entry + 8, 0);
    put_list(
        &mut e,
        FORMS + 0x8200,
        NODES + 0x100,
        &[null_entry, ENTRY_A, ENTRY_W],
    );
    e.mem.set_u32(RESULT, 0);
    call_on(&mut e, 0x2839, ACTOR, &[n(2)]);
    let id = form_result(&e);
    let made = by_id(&mut e, id);
    assert_eq!(ir(&mut e, made), (ACTOR, ARMO, 1, XDL));
    // With 3D: slot 2 holds the armour, covering part 2 but not worn; slot 3 another armour
    // covering it, worn. The first covering slot ends the search: nothing.
    // (the loaded data from above, now with a 3D root)
    e.mem.set_u32(ACTOR + 0x64, loaded);
    e.mem.set_u32(loaded + 0x14, FORMS + 0x8a00);
    e.mem.set_u8(XDL + 8 + 2, 0);
    e.mem.set_u32(equipment + 0x2c + 0x10 * 3, ARMO2);
    e.mem.set_u32(ARMO2 + 0x74, 4);
    e.mem.set_u32(ENTRY_W + 8, ARMO2);
    e.mem.set_u32(RESULT, 0);
    call_on(&mut e, 0x2839, ACTOR, &[n(2)]);
    assert_eq!(form_result(&e), 0, "stops at the first covering slot");
}

// ---- lane B3: adding, equipping and dropping ----

const LEVI: u32 = FORMS + 0x3400;
const OBJECTS: u32 = FORMS + 0xb000;
const NAMES: u32 = FORMS + 0xb800;
const INIT_FN: u32 = 0x0048_1610;
const DESTROY_FN: u32 = 0x0048_1680;
const ADD_FN: u32 = 0x0048_18e0;
const LEVELED_FN: u32 = 0x0048_7f70;
const MOVE_FN: u32 = 0x0048_21a0;
const LEVEL_FN: u32 = 0x0056_7e10;
const EQUIP_FN: u32 = 0x0088_c650;
const UNEQUIP_FN: u32 = 0x0088_c790;
const MESSAGE_FN: u32 = 0x0070_52f0;
const REMOVE_FN: u32 = 0x0040_3000;

thread_local! {
    static B3: std::cell::RefCell<Vec<(u32, Vec<u32>)>> = const { std::cell::RefCell::new(Vec::new()) };
    static SHOWN: std::cell::RefCell<Vec<Vec<u8>>> = const { std::cell::RefCell::new(Vec::new()) };
}

fn b3_log(addr: u32, w: &[u32]) {
    B3.with(|c| c.borrow_mut().push((addr, w.to_vec())));
}

fn b3_calls(addr: u32) -> Vec<Vec<u32>> {
    B3.with(|c| {
        c.borrow()
            .iter()
            .filter(|x| x.0 == addr)
            .map(|x| x.1.clone())
            .collect()
    })
}

fn b3_clear() {
    B3.with(|c| c.borrow_mut().clear());
    SHOWN.with(|c| c.borrow_mut().clear());
}

/// The game's functions these commands call, as stand-ins that log their words: the temporary
/// container's constructor (a vtable, an empty list) and destructor, its `AddItem` (a
/// `ContainerObject` appended; armour arrives with health extra data owned by no one at 0.25 when
/// `OBJECTS + 0x7f0` is set, a misc item with an owned one), the leveled add, the level, the move,
/// equip and unequip, the corner message (its text kept), and `RemoveItem` on the actor's and the
/// container reference's vtables.
fn b3_stand_ins(e: &mut Engine) {
    b3_clear();
    e.mem.set_u32(OBJECTS + 0x7fc, OBJECTS);
    e.register(INIT_FN, |e, w| {
        b3_log(INIT_FN, w);
        e.mem.set_u32(w[0], 0x0101_c130);
        e.mem.set_u32(w[0] + 4, 0);
        e.mem.set_u32(w[0] + 8, 0);
        Ret::default()
    });
    e.register(DESTROY_FN, |_, w| {
        b3_log(DESTROY_FN, w);
        Ret::default()
    });
    e.register(ADD_FN, |e, w| {
        b3_log(ADD_FN, w);
        let object = e.mem.u32(OBJECTS + 0x7fc);
        e.mem.set_u32(OBJECTS + 0x7fc, object + 0x10);
        e.mem.set_u32(object, w[2]);
        e.mem.set_u32(object + 4, w[1]);
        let extra = match e.mem.u8(w[1] + 4) {
            0x1f => OBJECTS + 0x700,
            0x29 if e.mem.u32(OBJECTS + 0x7f0) != 0 => OBJECTS + 0x720,
            _ => 0,
        };
        e.mem.set_u32(object + 8, extra);
        // Appended: in the head node when it is empty, else in a new node.
        let mut node = w[0] + 4;
        if e.mem.u32(node) != 0 {
            while e.mem.u32(node + 4) != 0 {
                node = e.mem.u32(node + 4);
            }
            let next = object + 0x1000;
            e.mem.set_u32(next, object);
            e.mem.set_u32(next + 4, 0);
            e.mem.set_u32(node + 4, next);
        } else {
            e.mem.set_u32(node, object);
        }
        Ret::default()
    });
    e.register(LEVELED_FN, |_, w| {
        b3_log(LEVELED_FN, w);
        Ret::default()
    });
    e.register(LEVEL_FN, |_, w| {
        b3_log(LEVEL_FN, w);
        Ret {
            eax: 7,
            ..Ret::default()
        }
    });
    e.register(MOVE_FN, |_, w| {
        b3_log(MOVE_FN, w);
        Ret::default()
    });
    e.register(EQUIP_FN, |_, w| {
        b3_log(EQUIP_FN, w);
        Ret::default()
    });
    e.register(UNEQUIP_FN, |_, w| {
        b3_log(UNEQUIP_FN, w);
        Ret::default()
    });
    e.register(MESSAGE_FN, |e, w| {
        b3_log(MESSAGE_FN, w);
        let text = nvse::table::guest_text(e, w[0]);
        SHOWN.with(|c| c.borrow_mut().push(text));
        Ret::default()
    });
    e.mem.set_u32(ACTOR_VT + 0x17c, REMOVE_FN);
    e.mem.set_u32(FORMS + 0x6800 + 0x17c, REMOVE_FN);
    e.register(REMOVE_FN, |_, w| {
        b3_log(REMOVE_FN, w);
        Ret::default()
    });
    // The armour's vtable page (its +0xF0 read for the name: 0, not a reference).
    e.map(0x0102_a000, 0x1000);
    // The game's message texts (filled at start-up) and a name for the armour and the weapon.
    e.map(0x011d_2000, 0x3000);
    e.mem
        .write(NAMES, b"equipped\0unequipped\0Combat Armor\0Laser Rifle\0");
    e.mem.set_u32(0x011d_2a10, NAMES);
    e.mem.set_u32(0x011d_4250, NAMES + 9);
}

fn name_items(e: &mut Engine) {
    e.mem.set_u32(ARMO + 0x34, NAMES + 20);
    e.mem.set_u16(ARMO + 0x38, 12);
    e.mem.set_u32(WEAP + 0x34, NAMES + 33);
    e.mem.set_u16(WEAP + 0x38, 11);
}

fn shown() -> Vec<Vec<u8>> {
    SHOWN.with(|c| c.borrow().clone())
}

/// The temporary container's items: (form, count, health extra) from its list.
fn moved_objects(e: &Engine) -> (u32, Vec<(u32, u32, u32)>) {
    let container = b3_calls(MOVE_FN)[0][0];
    let objects = list_items(e, Ptr::new(container + 4))
        .into_iter()
        .filter(|&o| o != 0)
        .map(|o| (e.mem.u32(o + 4), e.mem.u32(o), e.mem.u32(o + 8)))
        .collect();
    (container, objects)
}

#[test]
fn add_item_alt_an_item_at_a_condition_equipped() {
    let mut e = setup();
    inventory(&mut e);
    b3_stand_ins(&mut e);
    call_on(
        &mut e,
        0x2606,
        ACTOR,
        &[rf(ARMO), n(2), z(50.0), n(1), n(1)],
    );
    let (container, objects) = moved_objects(&e);
    assert_eq!(b3_calls(INIT_FN), vec![vec![container]]);
    assert_eq!(b3_calls(ADD_FN), vec![vec![container, ARMO, 2, 0]]);
    assert_eq!(objects.len(), 1);
    let (_, _, extra) = objects[0];
    assert_ne!(extra, 0, "armour gets health extra data");
    assert_eq!((e.mem.u32(extra), e.mem.u32(extra + 4)), (0, 0));
    assert_eq!(e.mem.f32(extra + 8), 50.0f32 * 0.01, "50 percent");
    assert_eq!(b3_calls(MOVE_FN), vec![vec![container, ACTOR, 1]]);
    assert_eq!(
        b3_calls(EQUIP_FN),
        vec![vec![ACTOR, ARMO, 1, XDL2, 1, 0, 1]],
        "equipped from its entry's first stack"
    );
    assert!(shown().is_empty(), "no message asked");
    assert_eq!(b3_calls(DESTROY_FN), vec![vec![container]]);
    assert!(freed().contains(&container));
    // Equipped with the message (low byte 0), named: the "equipped" message; full condition is
    // kept just under 1.
    b3_clear();
    name_items(&mut e);
    call_on(
        &mut e,
        0x2606,
        ACTOR,
        &[rf(ARMO), n(1), z(100.0), n(0x101), n(0x100)],
    );
    let (_, objects) = moved_objects(&e);
    assert_eq!(
        e.mem.f32(objects[0].2 + 8),
        f32::from_bits(0x3f7f_be77),
        "kMaxHealth"
    );
    assert_eq!(
        b3_calls(MOVE_FN)[0][2],
        0x100,
        "the move gets the whole word"
    );
    assert_eq!(shown(), vec![b"Combat Armor equipped.".to_vec()]);
    assert_eq!(
        b3_calls(MESSAGE_FN)[0][1..],
        [0, 0x0102_08e0, 0, 0x4000_0000, 0]
    );
    // Not equipped (low byte 0); a negative condition is 0.
    b3_clear();
    call_on(
        &mut e,
        0x2606,
        ACTOR,
        &[rf(ARMO), n(1), z(-20.0), n(0x100), n(0)],
    );
    let (_, objects) = moved_objects(&e);
    assert_eq!(e.mem.f32(objects[0].2 + 8), 0.0);
    assert!(b3_calls(EQUIP_FN).is_empty(), "doEquip's low byte only");
}

#[test]
fn add_item_alt_lists_leveled_items_and_health_by_type() {
    let mut e = setup();
    inventory(&mut e);
    b3_stand_ins(&mut e);
    form(&mut e, LEVI, 0x34, 0x20d);
    set_ref(&mut e, 13, LEVI);
    // A form list: an empty node, a misc item (owned extra data), not an item, ammunition (extra
    // data owned by no one), a weapon and a thrown weapon (two adds of the weapon).
    e.mem.set_u32(OBJECTS + 0x700, 0x1234);
    e.mem.set_f32(OBJECTS + 0x708, 0.3);
    e.mem.set_u32(OBJECTS + 0x720, 0);
    e.mem.set_f32(OBJECTS + 0x728, 0.3);
    e.mem.set_u32(OBJECTS + 0x7f0, 1);
    put_list(
        &mut e,
        LIST + 0x18,
        NODES + 0x300,
        &[0, MISC, NPC, AMMO, WEAP],
    );
    call_on(&mut e, 0x2606, CONT_REF, &[rf(LIST), n(3), z(40.0)]);
    assert_eq!(
        b3_calls(ADD_FN)
            .iter()
            .map(|w| (w[1], w[2]))
            .collect::<Vec<_>>(),
        vec![(MISC, 3), (AMMO, 3), (WEAP, 3)]
    );
    let (_, objects) = moved_objects(&e);
    assert_eq!(objects[0].2, OBJECTS + 0x700, "an owned extra stays");
    assert_eq!(e.mem.f32(OBJECTS + 0x708), 1.0, "its health reset to 1");
    assert_eq!(objects[1].2, 0, "an unowned one is freed");
    assert!(freed().contains(&(OBJECTS + 0x720)));
    assert_eq!(
        e.mem.f32(objects[2].2 + 8),
        40.0f32 * 0.01,
        "a weapon has health"
    );
    assert!(b3_calls(EQUIP_FN).is_empty());
    // A thrown weapon has none.
    b3_clear();
    e.mem.set_u8(WEAP + 0xf4, 10);
    call_on(&mut e, 0x2606, CONT_REF, &[rf(WEAP), n(1), z(40.0)]);
    assert_eq!(moved_objects(&e).1[0].2, 0, "thrown: no health");
    // A leveled item: its list (+0x30) adds at the reference's level.
    b3_clear();
    call_on(&mut e, 0x2606, CONT_REF, &[r(13), n(2)]);
    let container = b3_calls(INIT_FN)[0][0];
    assert_eq!(b3_calls(LEVEL_FN), vec![vec![CONT_REF, 0]]);
    assert_eq!(
        b3_calls(LEVELED_FN),
        vec![vec![LEVI + 0x30, 7, 2, container, 0]]
    );
    assert_eq!(b3_calls(MOVE_FN).len(), 1);
    // Not an item (the argument check lets none through; called directly): the container made
    // and destroyed, nothing moved.
    b3_clear();
    add_item_alt(&mut e, CONT_REF, NPC, 1, 1.0, 0, 1);
    assert_eq!(b3_calls(INIT_FN).len(), 1);
    assert!(b3_calls(MOVE_FN).is_empty() && b3_calls(ADD_FN).is_empty());
    assert_eq!(b3_calls(DESTROY_FN).len(), 1);
    // A count of 0, or a reference without a container: nothing at all.
    b3_clear();
    call_on(&mut e, 0x2606, CONT_REF, &[rf(MISC), n(0)]);
    e.mem.set_u32(CONT_REF + 0x20, MISC);
    call_on(&mut e, 0x2606, CONT_REF, &[rf(MISC)]);
    assert!(b3_calls(INIT_FN).is_empty());
}

#[test]
fn container_items_health_keeps_the_lower() {
    let mut e = setup();
    b3_stand_ins(&mut e);
    let container = OBJECTS + 0x600;
    let object = OBJECTS + 0x500;
    let extra = OBJECTS + 0x540;
    e.mem.set_u32(container + 4, object);
    e.mem.set_u32(container + 8, 0);
    e.mem.set_u32(object + 4, ARMO);
    e.mem.set_u32(object + 8, extra);
    e.mem.set_f32(extra + 8, 0.2);
    set_container_items_health(&mut e, container, 0.4);
    assert_eq!(e.mem.f32(extra + 8), 0.2, "the lower stays");
    set_container_items_health(&mut e, container, 0.1);
    assert_eq!(e.mem.f32(extra + 8), 0.1);
    set_container_items_health(&mut e, container, f32::NAN);
    assert_eq!(e.mem.f32(extra + 8), 0.1, "a NaN is kMaxHealth");
}

#[test]
fn equip_item_alt_counts_by_type() {
    let mut e = setup();
    inventory(&mut e);
    b3_stand_ins(&mut e);
    // The second entry as ammunition (20, no lists), then as weapons.
    e.mem.set_u32(ENTRY_W + 8, AMMO);
    e.mem.set_u32(ENTRY_W + 4, 20);
    call_on(&mut e, 0x2717, ACTOR, &[rf(AMMO), n(2)]);
    assert_eq!(b3_calls(EQUIP_FN), vec![vec![ACTOR, AMMO, 20, 0, 1, 2, 1]]);
    b3_clear();
    e.mem.set_u32(ENTRY_W + 8, WEAP);
    e.mem.set_u8(WEAP + 0xf4, 10);
    call_on(&mut e, 0x2717, ACTOR, &[rf(WEAP)]);
    e.mem.set_u8(WEAP + 0xf4, 9);
    call_on(&mut e, 0x2717, ACTOR, &[rf(WEAP)]);
    assert_eq!(
        b3_calls(EQUIP_FN),
        vec![
            vec![ACTOR, WEAP, 20, 0, 1, 0, 1],
            vec![ACTOR, WEAP, 1, 0, 1, 0, 1]
        ],
        "a thrown weapon the entry's count, another one"
    );
    // Aid and books one; a misc item nothing; an item it does not hold nothing; not an actor
    // nothing.
    b3_clear();
    e.mem.set_u32(ENTRY_W + 8, ALCH);
    call_on(&mut e, 0x2717, ACTOR, &[rf(ALCH)]);
    e.mem.set_u32(ENTRY_W + 8, MISC);
    e.mem.set_u8(MISC + 4, 0x19);
    call_on(&mut e, 0x2717, ACTOR, &[rf(MISC)]);
    e.mem.set_u8(MISC + 4, 0x1f);
    call_on(&mut e, 0x2717, ACTOR, &[rf(MISC)]);
    call_on(&mut e, 0x2717, ACTOR, &[rf(POISON)]);
    call_on(&mut e, 0x2717, CONT_REF, &[rf(ARMO)]);
    assert_eq!(
        b3_calls(EQUIP_FN),
        vec![
            vec![ACTOR, ALCH, 1, 0, 1, 0, 1],
            vec![ACTOR, MISC, 1, 0, 1, 0, 1]
        ]
    );
    // The armour from its first stack, with its message (named).
    b3_clear();
    name_items(&mut e);
    call_on(&mut e, 0x2717, ACTOR, &[rf(ARMO), n(0), n(0)]);
    assert_eq!(
        b3_calls(EQUIP_FN),
        vec![vec![ACTOR, ARMO, 1, XDL2, 1, 0, 1]]
    );
    assert_eq!(shown(), vec![b"Combat Armor equipped.".to_vec()]);
}

#[test]
fn unequip_item_alt_the_worn_stack() {
    let mut e = setup();
    inventory(&mut e);
    b3_stand_ins(&mut e);
    call_on(&mut e, 0x2718, ACTOR, &[rf(ARMO)]);
    call_on(&mut e, 0x2718, ACTOR, &[rf(ARMO), n(5), n(7)]);
    assert_eq!(
        b3_calls(UNEQUIP_FN),
        vec![
            vec![ACTOR, ARMO, 1, XDL, 1, 0, 1],
            vec![ACTOR, ARMO, 1, XDL, 1, 1, 1]
        ],
        "the worn stack; the flags as bools"
    );
    assert!(shown().is_empty());
    // With the message (named); a misc item, a weapon it does not hold, an unworn armour and a
    // non-actor: nothing.
    b3_clear();
    name_items(&mut e);
    call_on(&mut e, 0x2718, ACTOR, &[rf(ARMO), n(0), n(0)]);
    assert_eq!(shown(), vec![b"Combat Armor unequipped.".to_vec()]);
    b3_clear();
    call_on(&mut e, 0x2718, ACTOR, &[rf(MISC)]);
    call_on(&mut e, 0x2718, ACTOR, &[rf(WEAP)]);
    call_on(&mut e, 0x2718, CONT_REF, &[rf(ARMO)]);
    e.mem.set_u8(XDL + 8 + 2, 0);
    call_on(&mut e, 0x2718, ACTOR, &[rf(ARMO)]);
    assert!(b3_calls(UNEQUIP_FN).is_empty());
}

#[test]
fn item_message_cut_to_its_buffer() {
    let mut e = setup();
    b3_stand_ins(&mut e);
    let long = FORMS + 0xbc00;
    e.mem.write(long, &[b'x'; 200]);
    e.mem.set_u32(ARMO + 0x34, long);
    e.mem.set_u16(ARMO + 0x38, 200);
    show_item_message(&mut e, ARMO, 0);
    let text = shown()[0].clone();
    assert_eq!(text.len(), 0x6f, "0x70 bytes with the terminator");
    // A null text: the name, a space and the full stop.
    b3_clear();
    name_items(&mut e);
    show_item_message(&mut e, WEAP, 0);
    assert_eq!(shown(), vec![b"Laser Rifle .".to_vec()]);
}

/// `RemoveItem`'s words after the reference: (item, list, count, keep owner, drop, target).
fn removes() -> Vec<Vec<u32>> {
    b3_calls(REMOVE_FN)
        .into_iter()
        .map(|w| w[1..].to_vec())
        .collect()
}

#[test]
fn drop_alt_stacks_one_by_one_then_the_rest() {
    let mut e = setup();
    inventory(&mut e);
    b3_stand_ins(&mut e);
    // The armour: 4, a stack of 2 and the worn one; RemoveItem takes a stack's list out of the
    // entry when it is first (stand-in).
    e.register(REMOVE_FN, |e, w| {
        b3_log(REMOVE_FN, w);
        let lists = e.mem.u32(ENTRY_A);
        if w[2] != 0 && e.mem.u32(lists) == w[2] {
            let next = e.mem.u32(lists + 4);
            e.mem
                .set_u32(lists, if next == 0 { 0 } else { e.mem.u32(next) });
            e.mem
                .set_u32(lists + 4, if next == 0 { 0 } else { e.mem.u32(next + 4) });
        }
        Ret::default()
    });
    e.mem.set_u32(ENTRY_A + 4, 4);
    put_extra(&mut e, XDL2, X1, 0x24, 0);
    e.mem.set_u16(X1 + 0xc, 2);
    call_on(&mut e, 0x2719, ACTOR, &[rf(ARMO), n(3)]);
    assert_eq!(
        removes(),
        vec![
            vec![ARMO, XDL2, 1, 1, 1, 0, 0, 0, 1, 0],
            vec![ARMO, XDL2, 1, 1, 1, 0, 0, 0, 1, 0],
            vec![ARMO, XDL, 1, 1, 1, 0, 0, 0, 1, 0],
        ],
        "armour one by one, 3 of 4"
    );
}

#[test]
fn drop_alt_scripted_stacks_and_large_rests() {
    let mut e = setup();
    inventory(&mut e);
    b3_stand_ins(&mut e);
    // Aid: 3 in one stack of 3 with a script.
    e.mem.set_u32(ENTRY_W + 8, ALCH);
    e.mem.set_u32(ENTRY_W + 4, 3);
    let lists = FORMS + 0x8340;
    e.mem.set_u32(ENTRY_W, lists);
    put_list(&mut e, lists, NODES + 0x280, &[XDL2]);
    put_extra(&mut e, XDL2, X1, 0x24, 0);
    e.mem.set_u16(X1 + 0xc, 3);
    put_extra(&mut e, XDL2, FORMS + 0x4380, 0x0d, 0);
    e.mem.set_u32(ALCH + 0x8c, FORMS + 0x9f00);
    call_on(&mut e, 0x2719, ACTOR, &[rf(ALCH), n(0), n(1)]);
    assert_eq!(
        calls_to(REMOVE_BY_TYPE_FN),
        vec![vec![XDL2, 0x24]],
        "the scripted stack's count dropped"
    );
    assert_eq!(
        removes(),
        vec![vec![ALCH, XDL2, 1, 0, 1, 0, 0, 0, 1, 0]; 3],
        "one at a time, the owner cleared"
    );
    // Without a script on the form: the whole stack at once (aid stacks).
    b3_clear();
    e.mem.set_u32(ALCH + 0x8c, 0);
    put_extra(&mut e, XDL2, X1, 0x24, 0);
    e.mem.set_u16(X1 + 0xc, 3);
    call_on(&mut e, 0x2719, ACTOR, &[rf(ALCH)]);
    assert_eq!(removes(), vec![vec![ALCH, XDL2, 3, 1, 1, 0, 0, 0, 1, 0]]);
    // Ammunition only in the base container (0x9000) through a form list: no entry, drops of at
    // most 0x7FFF.
    b3_clear();
    e.mem.set_u32(FORMS + 0x8010, 0x9000);
    put_list(&mut e, LIST + 0x18, NODES + 0x300, &[0, AMMO]);
    call_on(&mut e, 0x2719, ACTOR, &[rf(LIST)]);
    assert_eq!(
        removes(),
        vec![
            vec![AMMO, 0, 0x7fff, 1, 1, 0, 0, 0, 1, 0],
            vec![AMMO, 0, 0x1001, 1, 1, 0, 0, 0, 1, 0],
        ]
    );
    // A count to drop; none held; a reference without changes.
    b3_clear();
    call_on(&mut e, 0x2719, ACTOR, &[rf(AMMO), n(5)]);
    call_on(&mut e, 0x2719, ACTOR, &[rf(POISON)]);
    call_on(&mut e, 0x2719, CONT_REF, &[rf(AMMO)]);
    assert_eq!(removes(), vec![vec![AMMO, 0, 5, 1, 1, 0, 0, 0, 1, 0]]);
}

#[test]
fn drop_me_alt_and_remove_me_ir_alt_mark_the_reference() {
    let mut e = setup();
    inventory(&mut e);
    ref_heap(&mut e);
    b3_stand_ins(&mut e);
    let refr = nvse::invref::create_entry(&mut e, ACTOR, ARMO, 3, XDL);
    call_on(&mut e, 0x271a, refr, &[n(2), n(1)]);
    assert_eq!(
        removes(),
        vec![vec![ARMO, XDL, 1, 0, 1, 0, 0, 0, 1, 0]; 2],
        "armour one by one from its container"
    );
    // Marked removed: the write-back leaves the stack's list alone.
    let id = crate::commands::ref_id(&e, refr);
    e.mem.set_u32(refr + 0x48, 0xdead);
    let before = e.mem.u32(XDL + 4);
    assert!(nvse::invref::write_ref_data_to_container(&mut e, id));
    assert_eq!(e.mem.u32(XDL + 4), before);
    // Ammunition: one drop of all (0 and more than it has).
    b3_clear();
    let ammo = nvse::invref::create_entry(&mut e, ACTOR, AMMO, 10, 0);
    call_on(&mut e, 0x271a, ammo, &[n(0)]);
    call_on(&mut e, 0x271a, ammo, &[n(11)]);
    assert_eq!(removes(), vec![vec![AMMO, 0, 10, 1, 1, 0, 0, 0, 1, 0]; 2]);
    // RemoveMeIRAlt: all of it to the target, not dropped.
    b3_clear();
    let other = nvse::invref::create_entry(&mut e, ACTOR, ARMO, 3, XDL2);
    call_on(&mut e, 0x27e2, other, &[n(0), n(1), rf(CONT_REF)]);
    call_on(&mut e, 0x27e2, other, &[n(2)]);
    assert_eq!(
        removes(),
        vec![
            vec![ARMO, XDL2, 3, 0, 0, CONT_REF, 0, 0, 1, 0],
            vec![ARMO, XDL2, 2, 1, 0, 0, 0, 0, 1, 0],
        ]
    );
    let other_id = crate::commands::ref_id(&e, other);
    e.mem.set_u32(other + 0x48, 0xbeef);
    let before = e.mem.u32(XDL2 + 4);
    assert!(nvse::invref::write_ref_data_to_container(&mut e, other_id));
    assert_eq!(e.mem.u32(XDL2 + 4), before, "marked removed");
    // A count of 0, or not an inventory reference: nothing.
    b3_clear();
    let none = nvse::invref::create_entry(&mut e, ACTOR, ARMO, 0, XDL);
    call_on(&mut e, 0x271a, none, &[]);
    call_on(&mut e, 0x27e2, none, &[]);
    call_on(&mut e, 0x271a, CONT_REF, &[]);
    call_on(&mut e, 0x27e2, CONT_REF, &[]);
    assert!(removes().is_empty());
}
