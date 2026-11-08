//! Tests of the inventory references (I-12), with the game's functions they call stood in by
//! small test functions; each against the xNVSE 6.4.9 source it translates.

use super::*;
use std::cell::RefCell;

const AREA: u32 = 0x0200_0000;
/// A bump allocator's next block, and its blocks.
const HEAP_TOP: u32 = AREA + 0xfffc;
const HEAP: u32 = AREA + 0x8000;
/// The temporary references' vtable, the container's, an item's.
const REF_VTABLE: u32 = AREA + 0x100;
const CONTAINER_VTABLE: u32 = AREA + 0x600;
const ITEM_VTABLE: u32 = AREA + 0xb00;
/// A container reference (an actor) with its base form (an actor base: its container at
/// `+0x64`), two items, extra data lists and changes.
const CONTAINER: u32 = AREA + 0x1000;
const BASE: u32 = AREA + 0x1100;
const ITEM_A: u32 = AREA + 0x2000;
const ITEM_B: u32 = AREA + 0x3000;
const CHANGES: u32 = AREA + 0x4000;
const DATA: u32 = AREA + 0x4100;
const ENTRY_A: u32 = AREA + 0x4200;
const ENTRY_B: u32 = AREA + 0x4300;
const LISTS_A: u32 = AREA + 0x4400;
const X_A: u32 = AREA + 0x4500;
const X_B: u32 = AREA + 0x4600;
const COUNT_A: u32 = AREA + 0x4700;
const WORN_B: u32 = AREA + 0x4800;
const FORM_COUNTS: u32 = AREA + 0x4900;
const NEXT_ID: u32 = AREA + 0xfff8;

thread_local! {
    static LOG: RefCell<Vec<(&'static str, Vec<u32>)>> = const { RefCell::new(Vec::new()) };
}

fn log(name: &'static str, a: &[u32]) {
    LOG.with(|l| l.borrow_mut().push((name, a.to_vec())));
}

fn calls() -> Vec<(&'static str, Vec<u32>)> {
    LOG.with(|l| l.borrow_mut().drain(..).collect())
}

fn eax(v: u32) -> Ret {
    Ret {
        eax: v,
        ..Ret::default()
    }
}

fn alloc(e: &mut Engine, a: &[u32]) -> Ret {
    let at = e.mem.u32(HEAP_TOP).max(HEAP);
    e.mem.set_u32(HEAP_TOP, at + a[0].next_multiple_of(8));
    for i in 0..a[0] {
        e.mem.set_u8(at + i, 0);
    }
    eax(at)
}

fn free(_: &mut Engine, a: &[u32]) -> Ret {
    log("free", a);
    Ret::default()
}

/// `TESObjectREFR`'s constructor: the vtable and a new id.
fn refr_init(e: &mut Engine, a: &[u32]) -> Ret {
    let id = e.mem.u32(NEXT_ID).max(0xff00_0800);
    e.mem.set_u32(NEXT_ID, id + 1);
    e.mem.set_u32(a[0], REF_VTABLE);
    e.mem.set_u32(a[0] + 0xc, id);
    eax(a[0])
}

fn destroy(_: &mut Engine, a: &[u32]) -> Ret {
    log("destroy", a);
    Ret::default()
}

fn yes(_: &mut Engine, _: &[u32]) -> Ret {
    eax(1)
}

fn remove_item(_: &mut Engine, a: &[u32]) -> Ret {
    log("remove", a);
    Ret::default()
}

/// `BaseExtraList::GetByType` over the list's chain (`BSExtraData`: type at `+4`, next at `+8`).
fn get_by_type(e: &mut Engine, a: &[u32]) -> Ret {
    let mut x = e.mem.u32(a[0] + 4);
    while x != 0 {
        if u32::from(e.mem.u8(x + 4)) == a[1] {
            return eax(x);
        }
        x = e.mem.u32(x + 8);
    }
    eax(0)
}

/// A `BSExtraData` of `kind` at `at`, put at the head of `list` and marked present.
fn add_extra(e: &mut Engine, list: u32, at: u32, kind: u32) {
    e.mem.set_u8(at + 4, kind as u8);
    e.mem.set_u32(at + 8, e.mem.u32(list + 4));
    e.mem.set_u32(list + 4, at);
    let bits = e.mem.u8(list + 8 + (kind >> 3));
    e.mem.set_u8(list + 8 + (kind >> 3), bits | (1 << (kind & 7)));
}

/// The container: a base count of 3 of item A; changes: item A with -1 and two lists (A with an
/// `ExtraCount` of 1, B without one, worn), item B (only in the changes) with 2.
fn engine() -> Engine {
    LOG.with(|l| l.borrow_mut().clear());
    let mut e = Engine::new();
    e.map(AREA, 0x1_0000);
    e.register(FORM_HEAP_ALLOCATE, alloc);
    e.register(FORM_HEAP_FREE, free);
    e.register(REFR_INIT, refr_init);
    e.register(EXTRA_GET_BY_TYPE, get_by_type);
    e.register(AREA + 0xf00, destroy);
    e.register(AREA + 0xf10, yes);
    e.register(AREA + 0xf20, remove_item);
    e.register(EXTRA_REMOVE_ALL, free);
    e.mem.set_u32(REF_VTABLE + FORM_DESTROY, AREA + 0xf00);
    e.mem.set_u32(CONTAINER_VTABLE + IS_ACTOR, AREA + 0xf10);
    e.mem.set_u32(CONTAINER_VTABLE + REFR_REMOVE_ITEM, AREA + 0xf20);
    for (item, id) in [(ITEM_A, 0x0001_0a00u32), (ITEM_B, 0x0001_0b00)] {
        e.mem.set_u32(item, ITEM_VTABLE);
        e.mem.set_u8(item + 4, 0x28);
        e.mem.set_u32(item + 0xc, id);
    }
    e.mem.set_u32(CONTAINER, CONTAINER_VTABLE);
    e.mem.set_u32(CONTAINER + 0x20, BASE);
    // The base container's list: one entry, 3 of item A.
    e.mem.set_u32(FORM_COUNTS, 3);
    e.mem.set_u32(FORM_COUNTS + 4, ITEM_A);
    e.mem.set_u32(BASE + 0x64 + 4, FORM_COUNTS);
    // The changes.
    add_extra(&mut e, CONTAINER + REFR_EXTRA, CHANGES, EXTRA_CONTAINER_CHANGES);
    e.mem.set_u32(CHANGES + 0xc, DATA);
    let objects = alloc(&mut e, &[8]).eax;
    e.mem.set_u32(DATA, objects);
    let second = alloc(&mut e, &[8]).eax;
    e.mem.set_u32(objects, ENTRY_A);
    e.mem.set_u32(objects + 4, second);
    e.mem.set_u32(second, ENTRY_B);
    e.mem.set_u32(ENTRY_A, LISTS_A);
    e.mem.set_u32(ENTRY_A + 4, (-1i32) as u32);
    e.mem.set_u32(ENTRY_A + 8, ITEM_A);
    e.mem.set_u32(ENTRY_B + 4, 2);
    e.mem.set_u32(ENTRY_B + 8, ITEM_B);
    let node = alloc(&mut e, &[8]).eax;
    e.mem.set_u32(LISTS_A, X_A);
    e.mem.set_u32(LISTS_A + 4, node);
    e.mem.set_u32(node, X_B);
    add_extra(&mut e, X_A, COUNT_A, EXTRA_COUNT);
    e.mem.set_u16(COUNT_A + 0xc, 1);
    add_extra(&mut e, X_B, WORN_B, EXTRA_WORN);
    e
}

fn id_of(e: &Engine, refr: u32) -> u32 {
    e.mem.u32(refr + 0xc)
}

/// `CreateInventoryRefEntry`: the temporary reference stands for the item with the list's data;
/// the clean-up destroys it (its base and data cleared first) and frees the entry it made.
#[test]
fn a_reference_with_an_entry_of_its_own_is_made_and_cleaned_up() {
    let mut e = engine();
    let refr = create_entry(&mut e, CONTAINER, ITEM_A, 1, X_A);
    let id = id_of(&e, refr);
    assert_eq!(e.mem.u32(refr + REFR_BASE_FORM), ITEM_A);
    assert_eq!(e.mem.u32(refr + REFR_EXTRA + 4), COUNT_A, "the list's data copied");
    assert!(for_ref_id(&mut e, id));
    let entry = get(&mut e, id).unwrap().data.entry;
    assert_eq!(count(&mut e, id), 1);
    clear(&mut e);
    assert!(!any(&mut e));
    assert_eq!(e.mem.u32(refr + REFR_BASE_FORM), 0);
    assert_eq!(e.mem.u32(refr + REFR_EXTRA + 4), 0);
    let log = calls();
    assert_eq!(log[0], ("destroy", vec![refr, 1]));
    let extend = e.mem.u32(entry);
    assert_eq!(&log[1..], [("free", vec![extend]), ("free", vec![entry])]);
}

/// The stacks of an item (`GetInvRefsForItem`): the base count with the changes' (3 - 1), the
/// lists with their counts first (A's 1, B's 1), nothing left for the rest; an item only in the
/// changes is all one stack.
#[test]
fn an_items_stacks_are_its_lists_then_the_rest() {
    let mut e = engine();
    let ids = crate::commands::inventory_ref::stacks_for_item(&mut e, CONTAINER, ITEM_A);
    assert_eq!(ids.len(), 2);
    let stacks: Vec<_> = ids
        .iter()
        .map(|&id| {
            let r = get(&mut e, id).unwrap().data;
            (r.xdata, e.mem.u32(r.entry + 4))
        })
        .collect();
    assert_eq!(stacks, [(X_A, 1), (X_B, 1)]);
    let ids = crate::commands::inventory_ref::stacks_for_item(&mut e, CONTAINER, ITEM_B);
    let r = get(&mut e, ids[0]).unwrap().data;
    assert_eq!((ids.len(), r.xdata, e.mem.u32(r.entry + 4)), (1, 0, 2));
}

/// `GetInventoryItems`: the base container's items, then the changes' own, in the order of an
/// `UnorderedMap` of 0x40 buckets by address: both items fall in bucket 0, so the one added
/// last comes first.
#[test]
fn a_containers_items_come_in_the_maps_order() {
    let mut e = engine();
    assert_eq!(
        inventory_items(&mut e, CONTAINER),
        [(ITEM_B, 2, ENTRY_B), (ITEM_A, 2, ENTRY_A)]
    );
}

/// A worn stack's removal waits for the clean-up (`kAction_Remove`), which removes the stack's
/// count of the item from the container; an unworn one goes from its entry at once.
#[test]
fn a_worn_stacks_removal_is_deferred() {
    let mut e = engine();
    let ids = crate::commands::inventory_ref::stacks_for_item(&mut e, CONTAINER, ITEM_A);
    let (a, b) = (ids[0], ids[1]);
    calls();
    // B is worn: deferred.
    assert!(remove_from_container(&mut e, b));
    assert!(calls().is_empty());
    // A is not: its list leaves the entry, its count with it (A's entry is the temporary one).
    assert!(remove_from_container(&mut e, a));
    clear(&mut e);
    let removes: Vec<_> = calls().into_iter().filter(|(n, _)| *n == "remove").collect();
    assert_eq!(
        removes,
        [(
            "remove",
            vec![CONTAINER, ITEM_A, X_B, 1, 0, 0, 0, 0, 0, 1, 0]
        )]
    );
}

/// With validation, a stack whose list left the container's entry is no longer valid.
#[test]
fn validation_follows_the_containers_lists() {
    let mut e = engine();
    let refr = create(
        &mut e,
        CONTAINER,
        Data {
            item: ITEM_A,
            entry: ENTRY_A,
            xdata: X_B,
        },
        true,
    );
    let id = id_of(&e, refr);
    assert!(for_ref_id(&mut e, id));
    assert_eq!(get_ref(&mut e, id), refr);
    // X_B leaves A's lists.
    e.mem.set_u32(e.mem.u32(LISTS_A + 4), 0);
    assert!(!for_ref_id(&mut e, id));
    assert_eq!(get_ref(&mut e, id), 0);
}

/// OnSell's item asked for during the handlers: the first list with a count (A's, 1), made once.
#[test]
fn the_sold_item_is_its_first_stack() {
    let mut e = engine();
    assert_eq!(sold_item_ref(&mut e), 0);
    set_sold(&mut e, ENTRY_A, CONTAINER);
    let refr = sold_item_ref(&mut e);
    let r = get(&mut e, id_of(&e, refr)).unwrap().data;
    assert_eq!((r.item, r.xdata, e.mem.u32(r.entry + 4)), (ITEM_A, X_A, 1));
    assert_eq!(sold_item_ref(&mut e), refr);
    clear_sold(&mut e);
    assert_eq!(sold_item_ref(&mut e), 0);
}
