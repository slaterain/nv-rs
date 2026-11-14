//! Tests of `ln_fn_inventory`: the player's inventory changes built by hand (entries: lists at
//! +0, form at +8; lists' extras walked from +4 with presence bits at +8), the new lists' and
//! hotkeys' exact blocks, and base forms' containers (a container's at +0x30, an actor base's at
//! +0x64; items `{count, form, extra}`).

use super::*;
use crate::commands::test_support::*;
use nvse::vars::{Data, Value as V};

const PC: u32 = FORMS + 0x1000;
const WEAP: u32 = FORMS + 0x1100;
const ARMO: u32 = FORMS + 0x1200;
const ALCH: u32 = FORMS + 0x1300;
const MISC: u32 = FORMS + 0x1400;
const CONT: u32 = FORMS + 0x1500;
const NPC: u32 = FORMS + 0x1600;
const ACTOR: u32 = FORMS + 0x1700;
const ACTOR_VT: u32 = FORMS + 0x1800;
const BASE_VT: u32 = FORMS + 0x1900;
const XC: u32 = FORMS + 0x2000;
const DATA: u32 = FORMS + 0x2100;
const NODES: u32 = FORMS + 0x3000;
const E_W: u32 = FORMS + 0x4000;
const E_A: u32 = FORMS + 0x4100;
const E_L: u32 = FORMS + 0x4200;
const L1: u32 = FORMS + 0x5000;
const L2: u32 = FORMS + 0x5100;
const L3: u32 = FORMS + 0x5200;
const XHK: u32 = FORMS + 0x6000;
const XH: u32 = FORMS + 0x6100;
const XM: u32 = FORMS + 0x6200;
const XHK2: u32 = FORMS + 0x6300;
const FC1: u32 = FORMS + 0x7000;

fn node(e: &mut Engine, at: u32, data: u32, next: u32) {
    e.mem.set_u32(at, data);
    e.mem.set_u32(at + 4, next);
}

/// The player with a weapon entry (two lists: L1 health 0.5 flags 3, L2 plain), an armour entry
/// (one list L3 holding hotkey 4) and an aid item entry with no lists.
fn setup() -> Engine {
    let mut e = engine();
    extra_stand_ins(&mut e);
    game_globals(&mut e);
    form(&mut e, PC, 0x3b, 0x14);
    form(&mut e, WEAP, 0x28, 0x201);
    form(&mut e, ARMO, 0x18, 0x202);
    form(&mut e, ALCH, 0x2f, 0x203);
    form(&mut e, MISC, 0x1f, 0x204);
    form(&mut e, CONT, 0x1b, 0x205);
    form(&mut e, NPC, 0x2a, 0x206);
    for (i, (f, id)) in [(WEAP, 0x201), (ARMO, 0x202), (ALCH, 0x203)]
        .iter()
        .enumerate()
    {
        in_form_map(&mut e, *f, *id, i as u32);
    }
    e.mem.set_u32(0x011d_ea3c, PC);
    put_extra(&mut e, PC + 0x44, XC, 0x15, DATA);
    e.mem.set_u32(DATA, NODES);
    node(&mut e, NODES, E_W, NODES + 8);
    node(&mut e, NODES + 8, E_A, NODES + 0x10);
    node(&mut e, NODES + 0x10, E_L, 0);
    e.mem.set_u32(E_W, NODES + 0x20);
    e.mem.set_u32(E_W + 8, WEAP);
    node(&mut e, NODES + 0x20, L1, NODES + 0x28);
    node(&mut e, NODES + 0x28, L2, 0);
    put_extra(&mut e, L1, XH, 0x25, 0.5f32.to_bits());
    put_extra(&mut e, L1, XM, 0x8d, 3);
    put_extra(&mut e, L2, XHK2 + 0x80, 0x25, 1.0f32.to_bits());
    e.mem.set_u32(E_A, NODES + 0x30);
    e.mem.set_u32(E_A + 8, ARMO);
    node(&mut e, NODES + 0x30, L3, 0);
    put_extra(&mut e, L3, XHK, 0x4a, 4);
    e.mem.set_u32(E_L + 8, ALCH);
    // Base forms and an actor.
    e.mem.set_u32(NPC, BASE_VT);
    e.mem.set_u32(MISC, BASE_VT + 0x200);
    e.mem.set_u32(BASE_VT + 0xf8, 0x008d_0360);
    e.mem.set_u32(ACTOR_VT + 0x100, 0x008d_0360);
    form(&mut e, ACTOR, 0x3b, 0x207);
    e.mem.set_u32(ACTOR, ACTOR_VT);
    e.mem.set_u32(ACTOR + 0x20, NPC);
    for (i, f) in [WEAP, ARMO, ALCH, MISC, CONT, NPC].iter().enumerate() {
        set_ref(&mut e, i as u16 + 1, *f);
    }
    e
}

fn hotkey_of(e: &Engine, xdl: u32) -> Option<u8> {
    let mut x = e.mem.u32(xdl + 4);
    while x != 0 {
        if e.mem.u8(x + 4) == 0x4a {
            return Some(e.mem.u8(x + 0xc));
        }
        x = e.mem.u32(x + 8);
    }
    None
}

#[test]
fn a_hotkey_on_the_matching_list() {
    let mut e = setup();
    let block = heap_next(&e);
    // Hotkey 1 (index 0) on the weapon with health 0.5 and flags 3: L1.
    call(&mut e, 0x2319, &[n(1), r(1), z(0.5), n(3)]);
    assert_eq!(hotkey_of(&e, L1), Some(0));
    assert_eq!(hotkey_of(&e, L2), None);
    assert_eq!(allocations(), vec![0x10]);
    assert_eq!(
        [0, 4, 8, 0xc].map(|o| e.mem.u32(block + o)),
        [0x0101_592c, 0x4a, 0, 0]
    );
    // Health -2 (any) and flags -1 (any): L1 matches first and keeps the hotkey.
    call(&mut e, 0x2319, &[n(1), r(1)]);
    assert_eq!((hotkey_of(&e, L1), hotkey_of(&e, L2)), (Some(0), None));
    // Health 1.0, no flags (0): L2 gets it, L1 loses it.
    call(&mut e, 0x2319, &[n(1), r(1), z(1.0), n(0)]);
    assert_eq!((hotkey_of(&e, L1), hotkey_of(&e, L2)), (None, Some(0)));
    // Index 0 and above 8: nothing.
    call(&mut e, 0x2319, &[n(9), r(1)]);
    call(&mut e, 0x2319, &[n(0), r(1)]);
    assert_eq!(hotkey_of(&e, L2), Some(0));
}

#[test]
fn a_hotkey_moved_from_another_item() {
    let mut e = setup();
    // Hotkey 5 (index 4) set on the weapon: the armour's first list loses it, and that list,
    // left empty, is freed and taken out.
    call(&mut e, 0x2319, &[n(5), r(1), z(0.5), n(3)]);
    assert_eq!(hotkey_of(&e, L1), Some(4));
    assert_eq!(e.mem.u32(NODES + 0x30), 0, "the armour's only node emptied");
    assert_eq!(freed(), vec![L3]);
    assert_eq!(calls_to(REMOVE_BY_TYPE_FN), vec![vec![L3, 0x4a]]);
}

#[test]
fn an_aid_item_gets_a_new_list() {
    let mut e = setup();
    let first = heap_next(&e);
    call(&mut e, 0x2319, &[n(2), r(3), z(-1.0), n(0)]);
    // A list (0x20, zeroed, its vtable), its hotkey (0x10), and the 8-byte node.
    assert_eq!(allocations(), vec![0x20, 0x10, 8]);
    assert_eq!(e.mem.u32(first), 0x0101_43e8);
    assert_eq!(hotkey_of(&e, first), Some(1));
    let nd = e.mem.u32(E_L);
    assert_eq!((e.mem.u32(nd), e.mem.u32(nd + 4)), (first, 0));
    // With a health asked: no new list.
    e.mem.set_u32(E_L, 0);
    call(&mut e, 0x2319, &[n(3), r(3), z(0.5)]);
    assert_eq!(e.mem.u32(E_L), 0);
}

#[test]
fn no_form_clears_the_index() {
    let mut e = setup();
    // ClearAllHotkeys: the armour's list loses its hotkey and goes.
    call(&mut e, 0x222c, &[]);
    assert_eq!(freed(), vec![L3]);
    assert!(hotkey_of(&e, L1).is_none() && hotkey_of(&e, L2).is_none());
}

#[test]
fn a_form_not_carried_still_takes_the_index() {
    let mut e = setup();
    // A misc item the player does not carry: the armour's first list still loses index 4.
    call(&mut e, 0x2319, &[n(5), r(4)]);
    assert_eq!(freed(), vec![L3]);
    assert!(allocations().is_empty());
}

#[test]
fn hotkeys_saved_and_restored() {
    let mut e = setup();
    put_extra(&mut e, L1, XHK2, 0x4a, 0);
    let (_, res) = call(&mut e, 0x2216, &[]);
    let outer = array_data(&e, res);
    assert_eq!(outer.len(), 8);
    let Data::Array(first) = outer[0] else {
        panic!()
    };
    assert_eq!(
        array_data(&e, f64::from(first)),
        vec![Data::Form(0x201), Data::Number(0.5), Data::Number(3.0)]
    );
    let Data::Array(fifth) = outer[4] else {
        panic!()
    };
    assert_eq!(
        array_data(&e, f64::from(fifth)),
        vec![Data::Form(0x202), Data::Number(-1.0), Data::Number(0.0)]
    );
    let Data::Array(empty) = outer[1] else {
        panic!()
    };
    assert_eq!(
        array_data(&e, f64::from(empty)),
        vec![Data::Form(0), Data::Number(-1.0), Data::Number(0.0)]
    );
    // Moved, then restored from the saved ones.
    call(&mut e, 0x2319, &[n(1), r(1), z(1.0), n(0)]);
    assert_eq!(hotkey_of(&e, L2), Some(0));
    call(&mut e, 0x2217, &[]);
    assert_eq!((hotkey_of(&e, L1), hotkey_of(&e, L2)), (Some(0), None));
    // From an array of eight: index 2 to the plain weapon list.
    let mut vals = Vec::new();
    for i in 0..8 {
        let inner = if i == 2 {
            array_of(&mut e, vec![V::Form(0x201), V::Number(1.0), V::Number(0.0)])
        } else {
            array_of(&mut e, vec![V::Number(0.0)])
        };
        vals.push(V::Array(inner));
    }
    let id = array_of(&mut e, vals);
    call(&mut e, 0x2217, &[n(id as i32)]);
    assert_eq!(hotkey_of(&e, L2), Some(2));
    // Not eight: nothing.
    let short = array_of(&mut e, vec![V::Number(0.0)]);
    call(&mut e, 0x2217, &[n(short as i32)]);
}

#[test]
fn base_container_items() {
    let mut e = setup();
    // The container's items at +0x30 + 4: WEAP x2.
    node(&mut e, CONT + 0x34, FC1, 0);
    e.mem.set_u32(FC1, 2);
    e.mem.set_u32(FC1 + 4, WEAP);
    assert_eq!(call(&mut e, 0x22f7, &[r(1), r(5)]).1, 2.0);
    call(&mut e, 0x22f8, &[r(1), n(3), r(5)]);
    assert_eq!(e.mem.u32(FC1), 5);
    // A new item: 0xC for it and 0xC for its extra, put first.
    let fc = heap_next(&e);
    call(&mut e, 0x22fb, &[r(2), n(1), z(0.25), r(5)]);
    assert_eq!(
        allocations(),
        vec![0xc, 0xc, 8],
        "the item, its extra, the node"
    );
    assert_eq!(e.mem.u32(CONT + 0x34), fc);
    assert_eq!((e.mem.u32(fc), e.mem.u32(fc + 4)), (1, ARMO));
    let ce = e.mem.u32(fc + 8);
    assert_eq!(
        (e.mem.u32(ce), e.mem.u32(ce + 4), e.mem.f32(ce + 8)),
        (0, 0, 0.25)
    );
    // An actor base's container (+0x64), by the calling actor's base form.
    call_on(&mut e, 0x22f8, ACTOR, &[r(3), n(4)]);
    assert_eq!(call(&mut e, 0x22f7, &[r(3), r(6)]).1, 4.0);
    // A misc item has no container.
    assert_eq!(call(&mut e, 0x22f7, &[r(1), r(4)]).1, 0.0);
}

#[test]
fn base_items_removed() {
    let mut e = setup();
    node(&mut e, CONT + 0x34, FC1, 0);
    e.mem.set_u32(FC1, 2);
    e.mem.set_u32(FC1 + 4, WEAP);
    call(&mut e, 0x22f9, &[r(1), n(1), r(5)]);
    assert_eq!(e.mem.u32(FC1), 1);
    call(&mut e, 0x22f9, &[r(1), n(1), r(5)]);
    assert_eq!(e.mem.u32(CONT + 0x34), 0, "below 1: the node emptied");
    assert!(freed().is_empty(), "the item itself is not freed");
    // On an actor: the form unequipped first.
    e.register(0x0088_c790, |_, w| {
        CALLS.with(|c| c.borrow_mut().push((0x0088_c790, w.to_vec())));
        Ret::default()
    });
    call_on(&mut e, 0x22f8, ACTOR, &[r(1), n(1)]);
    call_on(&mut e, 0x22f9, ACTOR, &[r(1), n(5)]);
    assert_eq!(
        calls_to(0x0088_c790),
        vec![vec![ACTOR, WEAP, 1, 0, 1, 0, 1]]
    );
}

/// The plain rule for a new list: flags above 0 make none.
#[test]
fn no_new_list_with_flags() {
    let mut e = setup();
    call(&mut e, 0x2319, &[n(2), r(3), z(-1.0), n(3)]);
    assert_eq!(e.mem.u32(E_L), 0);
    assert!(allocations().is_empty());
}

/// The match on weapons: a health no list has, or flags no list has (a list without mod flags
/// matches only flags 0), gives nobody the hotkey.
#[test]
fn no_list_matches() {
    let mut e = setup();
    call(&mut e, 0x2319, &[n(1), r(1), z(0.75)]);
    assert_eq!((hotkey_of(&e, L1), hotkey_of(&e, L2)), (None, None));
    call(&mut e, 0x2319, &[n(1), r(1), z(-2.0), n(5)]);
    assert_eq!((hotkey_of(&e, L1), hotkey_of(&e, L2)), (None, None));
}

/// A list with the item's hotkey that matches keeps it under the new index.
#[test]
fn a_hotkey_reindexed() {
    let mut e = setup();
    call(&mut e, 0x2319, &[n(2), r(2)]);
    assert_eq!(hotkey_of(&e, L3), Some(1));
    assert!(allocations().is_empty(), "the same extra");
}

/// A list of the item left empty by losing the index is freed and taken out.
#[test]
fn an_emptied_list_cleaned() {
    let mut e = setup();
    let l5 = FORMS + 0x5300;
    node(&mut e, NODES + 0x28, L2, NODES + 0x48);
    node(&mut e, NODES + 0x48, l5, 0);
    put_extra(&mut e, l5, XHK + 0x80, 0x4a, 6);
    call(&mut e, 0x2319, &[n(7), r(1), z(0.5), n(3)]);
    assert_eq!(hotkey_of(&e, L1), Some(6));
    assert_eq!(freed(), vec![l5, NODES + 0x48]);
    assert_eq!(e.mem.u32(NODES + 0x2c), 0, "L2's node is the last");
}

/// An actor base's container is the one at +0x64 (its items at +0x68).
#[test]
fn an_actor_base_container_offset() {
    let mut e = setup();
    node(&mut e, NPC + 0x68, FC1, 0);
    e.mem.set_u32(FC1, 6);
    e.mem.set_u32(FC1 + 4, WEAP);
    assert_eq!(call(&mut e, 0x22f7, &[r(1), r(6)]).1, 6.0);
}

const L4: u32 = FORMS + 0x5400;
const L5: u32 = FORMS + 0x5500;
const L6: u32 = FORMS + 0x5600;
const XH4: u32 = FORMS + 0x6400;
const XHK3: u32 = FORMS + 0x6500;
const XH6: u32 = FORMS + 0x6600;
const E_M: u32 = FORMS + 0x4300;
const FC2: u32 = FORMS + 0x7100;
const FC3: u32 = FORMS + 0x7200;

/// SetHotkey 5 on the armour with health 0.5: its only list L3 (hotkey 5, no health) does not
/// match, so it loses the index and is left empty; `CleanEmpty` frees it, and as the first node
/// with nothing after it, that node is emptied (`RemoveMe`). Nobody gets the hotkey.
#[test]
fn the_items_first_list_left_empty_alone() {
    let mut e = setup();
    call(&mut e, 0x2319, &[n(5), r(2), z(0.5)]);
    assert_eq!(calls_to(REMOVE_BY_TYPE_FN), vec![vec![L3, 0x4a]]);
    assert_eq!(freed(), vec![L3]);
    assert_eq!((e.mem.u32(NODES + 0x30), e.mem.u32(NODES + 0x34)), (0, 0));
    assert!(allocations().is_empty());
}

/// The same with a second list L4 (health 0.5) after L3: L4 gets the hotkey (a new 0x10-byte
/// extra), L3 is freed, the first node takes L4 and the second node is freed.
#[test]
fn the_items_first_list_left_empty_with_one_after() {
    let mut e = setup();
    node(&mut e, NODES + 0x30, L3, NODES + 0x50);
    node(&mut e, NODES + 0x50, L4, 0);
    put_extra(&mut e, L4, XH4, 0x25, 0.5f32.to_bits());
    let block = heap_next(&e);
    call(&mut e, 0x2319, &[n(5), r(2), z(0.5)]);
    assert_eq!(allocations(), vec![0x10]);
    assert_eq!(e.mem.u32(block), 0x0101_592c);
    assert_eq!(hotkey_of(&e, L4), Some(4));
    assert_eq!(freed(), vec![L3, NODES + 0x50]);
    assert_eq!((e.mem.u32(NODES + 0x30), e.mem.u32(NODES + 0x34)), (L4, 0));
}

/// Two lists both left empty (each held only a hotkey 5): `CleanEmpty` frees L3, the first node
/// takes L4 (its node freed) and is looked at again: L4, empty too, is freed and the first node
/// emptied.
#[test]
fn the_items_first_two_lists_left_empty() {
    let mut e = setup();
    node(&mut e, NODES + 0x30, L3, NODES + 0x50);
    node(&mut e, NODES + 0x50, L4, 0);
    put_extra(&mut e, L4, XH4, 0x4a, 4);
    call(&mut e, 0x2319, &[n(5), r(2), z(0.5)]);
    assert_eq!(freed(), vec![L3, NODES + 0x50, L4]);
    assert_eq!(
        (e.mem.u32(NODES + 0x30), e.mem.u32(NODES + 0x34)),
        (0, 0)
    );
    assert!(allocations().is_empty());
}

/// RestoreHotkeys with an inner array whose form is none: `SetHotkey` with no form clears the
/// index (`ClearHotkey`): the first list with hotkey 5 (the armour's L3) loses it, and left
/// empty, is taken out and freed. The inner arrays that are not three long are passed over.
#[test]
fn a_hotkey_with_no_form_cleared() {
    let mut e = setup();
    let mut vals = Vec::new();
    for i in 0..8 {
        let inner = if i == 4 {
            array_of(&mut e, vec![V::Form(0), V::Number(0.0), V::Number(0.0)])
        } else {
            array_of(&mut e, vec![V::Number(0.0)])
        };
        vals.push(V::Array(inner));
    }
    let id = array_of(&mut e, vals);
    call(&mut e, 0x2217, &[n(id as i32)]);
    assert_eq!(calls_to(REMOVE_BY_TYPE_FN), vec![vec![L3, 0x4a]]);
    assert_eq!(freed(), vec![L3]);
    assert_eq!(e.mem.u32(NODES + 0x30), 0);
    assert!(allocations().is_empty());
}

/// A misc item's list with hotkey 5 is not one of the three types: SetHotkey 5 on the weapon
/// takes the index from the armour only, ClearAllHotkeys passes it over, and SetHotkey on the
/// misc item itself changes nothing.
#[test]
fn another_types_hotkey_left_alone() {
    let mut e = setup();
    node(&mut e, NODES + 0x10, E_L, NODES + 0x18);
    node(&mut e, NODES + 0x18, E_M, 0);
    e.mem.set_u32(E_M, NODES + 0x58);
    e.mem.set_u32(E_M + 8, MISC);
    node(&mut e, NODES + 0x58, L5, 0);
    put_extra(&mut e, L5, XHK3, 0x4a, 4);
    call(&mut e, 0x2319, &[n(5), r(1), z(0.5), n(3)]);
    assert_eq!(freed(), vec![L3]);
    assert_eq!(hotkey_of(&e, L5), Some(4));
    call(&mut e, 0x222c, &[]);
    assert_eq!(hotkey_of(&e, L5), Some(4));
    call(&mut e, 0x2319, &[n(2), r(4)]);
    assert_eq!(hotkey_of(&e, L5), Some(4));
    assert_eq!(e.mem.u32(NODES + 0x58), L5);
    assert_eq!(allocations(), vec![0x10], "only the weapon's hotkey");
}

/// An aid item's lists all match (health and flags are not compared): its list with health 0.5
/// takes hotkey 2 asked with health 1.0 and flags 0.
#[test]
fn an_aid_items_list_matches_any_health() {
    let mut e = setup();
    e.mem.set_u32(E_L, NODES + 0x60);
    node(&mut e, NODES + 0x60, L6, 0);
    put_extra(&mut e, L6, XH6, 0x25, 0.5f32.to_bits());
    call(&mut e, 0x2319, &[n(2), r(3), z(1.0), n(0)]);
    assert_eq!(hotkey_of(&e, L6), Some(1));
    assert_eq!(allocations(), vec![0x10]);
}

/// An entry whose list of lists is there but empty (its first node holds none): a plain hotkey
/// makes a new list (0x20, then its hotkey 0x10) that goes into that first node, no node made;
/// a hotkey with a health asked makes nothing.
#[test]
fn a_new_list_into_an_empty_first_node() {
    let mut e = setup();
    e.mem.set_u32(E_L, NODES + 0x60);
    node(&mut e, NODES + 0x60, 0, 0);
    call(&mut e, 0x2319, &[n(3), r(3), z(0.5)]);
    assert!(allocations().is_empty(), "a health asked");
    assert_eq!(e.mem.u32(NODES + 0x60), 0);
    let first = heap_next(&e);
    call(&mut e, 0x2319, &[n(3), r(3)]);
    assert_eq!(allocations(), vec![0x20, 0x10]);
    assert_eq!(
        (e.mem.u32(NODES + 0x60), e.mem.u32(NODES + 0x64)),
        (first, 0)
    );
    assert_eq!(e.mem.u32(first), 0x0101_43e8);
    assert_eq!(hotkey_of(&e, first), Some(2));
}

/// The container with WEAP, ARMO (and MISC with `three`), one each, in that order.
fn items(e: &mut Engine, three: bool) {
    node(e, CONT + 0x34, FC1, NODES + 0x70);
    node(e, NODES + 0x70, FC2, if three { NODES + 0x78 } else { 0 });
    node(e, NODES + 0x78, FC3, 0);
    for (fc, f) in [(FC1, WEAP), (FC2, ARMO), (FC3, MISC)] {
        e.mem.set_u32(fc, 1);
        e.mem.set_u32(fc + 4, f);
    }
}

/// BaseRemoveItem taking the last item out: its node freed and unlinked; then the first, now
/// alone: the first node emptied, nothing freed (the items themselves are never freed).
#[test]
fn base_items_removed_at_the_end() {
    let mut e = setup();
    items(&mut e, false);
    call(&mut e, 0x22f9, &[r(2), n(1), r(5)]);
    assert_eq!(freed(), vec![NODES + 0x70]);
    assert_eq!((e.mem.u32(CONT + 0x34), e.mem.u32(CONT + 0x38)), (FC1, 0));
    call(&mut e, 0x22f9, &[r(1), n(1), r(5)]);
    assert_eq!(freed(), vec![NODES + 0x70]);
    assert_eq!((e.mem.u32(CONT + 0x34), e.mem.u32(CONT + 0x38)), (0, 0));
}

/// The first item with one after it: the first node takes the second's item and next, the
/// second node is freed.
#[test]
fn base_items_removed_at_the_head() {
    let mut e = setup();
    items(&mut e, false);
    call(&mut e, 0x22f9, &[r(1), n(1), r(5)]);
    assert_eq!(freed(), vec![NODES + 0x70]);
    assert_eq!((e.mem.u32(CONT + 0x34), e.mem.u32(CONT + 0x38)), (FC2, 0));
}

/// The middle of three: its node freed, the first linked to the third.
#[test]
fn base_items_removed_in_the_middle() {
    let mut e = setup();
    items(&mut e, true);
    call(&mut e, 0x22f9, &[r(2), n(1), r(5)]);
    assert_eq!(freed(), vec![NODES + 0x70]);
    assert_eq!(
        (e.mem.u32(CONT + 0x34), e.mem.u32(CONT + 0x38)),
        (FC1, NODES + 0x78)
    );
    assert_eq!(call(&mut e, 0x22f7, &[r(4), r(5)]).1, 1.0);
}
