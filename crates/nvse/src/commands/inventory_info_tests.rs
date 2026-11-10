//! Tests of the batch through the commands' records: a container reference whose base form's
//! container and changes are laid out by hand (the test's cast puts the components), extra data
//! lists answered by a stand-in of `BaseExtraList::GetByType` (a list's data of type `t` at
//! `+0x100 + 4t`), and the game functions the commands call recorded.

use super::super::test_support::*;
use super::*;
use std::cell::RefCell;

const ARMOR: u32 = 0x0200_1000;
const WEAPON: u32 = 0x0200_2000;
const AMMO: u32 = 0x0200_3000;
const LEVELED: u32 = 0x0200_3800;
const REF: u32 = 0x0200_4000;
const BASE: u32 = 0x0200_4800;
const ACTOR: u32 = 0x0200_5000;
const CHANGES: u32 = 0x0200_6000;
const XDATA: [u32; 3] = [0x0200_6400, 0x0200_6800, 0x0200_6c00];
const NODES: u32 = 0x0200_7400;
const OWNER: u32 = 0x0200_8000;
const LIST_FORM: u32 = 0x0200_8800;
const VTABLE: u32 = 0x0200_9000;
const HEAP: u32 = 0x0300_c000;
const FN_ADD_ITEM: u32 = 0x0060_0190;
const FN_MARK: u32 = 0x0060_0048;
const FN_GET_HEALTH: u32 = 0x0060_0010;

thread_local! {
    static CALLS: RefCell<Vec<(u32, Vec<u32>)>> = const { RefCell::new(Vec::new()) };
}

fn record(at: u32, a: &[u32]) {
    CALLS.with(|c| c.borrow_mut().push((at, a.to_vec())));
}

fn calls_to(at: u32) -> Vec<Vec<u32>> {
    CALLS.with(|c| {
        let mut c = c.borrow_mut();
        let out = c
            .iter()
            .filter(|(f, _)| *f == at)
            .map(|(_, a)| a.clone())
            .collect();
        c.retain(|(f, _)| *f != at);
        out
    })
}

fn eax(v: u32) -> Ret {
    Ret {
        eax: v,
        ..Ret::default()
    }
}

fn get_by_type(e: &mut Engine, a: &[u32]) -> Ret {
    eax(e.mem.u32(a[0] + 0x100 + 4 * a[1]))
}

fn put_extra(e: &mut Engine, list: u32, kind: u32, data: u32) {
    let at = list + 8 + (kind >> 3);
    let bits = e.mem.u8(at) | (1 << (kind & 7));
    e.mem.set_u8(at, bits);
    e.mem.set_u32(list + 0x100 + 4 * kind, data);
}

fn extra_of(e: &Engine, list: u32, kind: u32) -> u32 {
    e.mem.u32(list + 0x100 + 4 * kind)
}

/// Writes a `tList` of `items` at `at` (nodes 8 bytes apart).
fn tlist(e: &mut Engine, at: u32, items: &[u32]) {
    for (i, item) in items.iter().enumerate() {
        let node = at + 8 * i as u32;
        e.mem.set_u32(node, *item);
        let next = if i + 1 < items.len() { node + 8 } else { 0 };
        e.mem.set_u32(node + 4, next);
    }
}

/// An entry {extend list, count delta, item}.
fn entry(e: &mut Engine, at: u32, extend: u32, delta: i32, item: u32) -> u32 {
    e.mem.set_u32(at, extend);
    e.mem.set_u32(at + 4, delta as u32);
    e.mem.set_u32(at + 8, item);
    at
}

/// The forms (ids `0x0001000N`; the weapon and armor with health 250 and 100), a container
/// reference `REF` (an actor) whose base form's container holds 2 armor, a leveled list and 3
/// ammo, and whose changes hold the ammo (-3: all gone), the weapon (+1, its one list with health
/// 50 and the owner) and the armor (+1, no lists).
fn setup() -> Engine {
    CALLS.with(|c| c.borrow_mut().clear());
    let mut e = engine_with_table();
    e.register(0x0041_0220, get_by_type);
    for (form, kind, id) in [
        (ARMOR, 0x18, 1),
        (WEAPON, 0x28, 2),
        (AMMO, 0x29, 3),
        (LEVELED, 0x34, 4),
        (OWNER, 0x2a, 5),
    ] {
        e.mem.set_u8(form + 4, kind);
        e.mem.set_u32(form + 0xc, 0x0001_0000 + id);
    }
    e.mem.set_u32(WEAPON + 0x7f0, 4);
    e.mem.set_u32(WEAPON + 0x200 + 4, 250);
    e.mem.set_u32(ARMOR + 0x7f0, 4);
    e.mem.set_u32(ARMOR + 0x200 + 4, 100);
    e.mem.set_u32(LEVELED + 0x7f0, 1 << 24);
    e.mem.set_u8(LIST_FORM + 4, 0x55);
    e.mem.set_u8(REF + 4, 0x3b);
    e.mem.set_u32(REF + 0x7f0, 1 | (1 << 21));
    e.mem.set_u32(REF + 0xc, 0x0001_0010);
    e.mem.set_u32(REF + 0x20, BASE);
    e.mem.set_u32(REF, VTABLE);
    e.mem.set_u32(BASE + 0x7f0, 1 << 23);
    let fcs = [NODES + 0x200, NODES + 0x210, NODES + 0x220];
    for (fc, (count, form)) in fcs.iter().zip([(2, ARMOR), (1, LEVELED), (3, AMMO)]) {
        e.mem.set_u32(*fc, count);
        e.mem.set_u32(fc + 4, form);
    }
    tlist(&mut e, BASE + 0x580 + 4, &fcs);
    put_extra(&mut e, REF + 0x44, 0x15, CHANGES);
    e.mem.set_u32(CHANGES + 0xc, CHANGES + 0x20);
    e.mem.set_u32(CHANGES + 0x20, NODES);
    let ammo = entry(&mut e, NODES + 0x100, 0, -3, AMMO);
    let weapon = entry(&mut e, NODES + 0x110, NODES + 0x40, 1, WEAPON);
    let armor = entry(&mut e, NODES + 0x120, 0, 1, ARMOR);
    tlist(&mut e, NODES, &[ammo, weapon, armor]);
    tlist(&mut e, NODES + 0x40, &[XDATA[0]]);
    put_extra(&mut e, XDATA[0], 0x25, XDATA[0] + 0x380);
    e.mem.set_f32(XDATA[0] + 0x38c, 50.0);
    put_extra(&mut e, XDATA[0], 0x21, XDATA[0] + 0x390);
    e.mem.set_u32(XDATA[0] + 0x39c, OWNER);
    for (i, form) in [ARMOR, WEAPON, AMMO, OWNER, LIST_FORM, REF]
        .into_iter()
        .enumerate()
    {
        set_script_ref(&mut e, i as u16 + 1, form);
    }
    e.register(0x0040_1000, |e, a| {
        let at = e.mem.u32(HEAP).max(HEAP + 0x400);
        e.mem.set_u32(HEAP, at + a[0].next_multiple_of(0x400));
        eax(at)
    });
    e.register(0x0040_1030, |_, _| Ret::default());
    e.register(0x0040_ff60, |e, a| {
        let kind = u32::from(e.mem.u8(a[1] + 4));
        put_extra(e, a[0], kind, a[1]);
        Ret::default()
    });
    e.register(0x0041_0020, |e, a| {
        record(0x0041_0020, a);
        let kind = u32::from(e.mem.u8(a[1] + 4));
        e.mem.set_u32(a[0] + 0x100 + 4 * kind, 0);
        Ret::default()
    });
    e.register(0x0048_39c0, |_, a| {
        eax(match a[0] {
            0x0001_0001 => ARMOR,
            0x0001_0002 => WEAPON,
            0x0001_0003 => AMMO,
            _ => 0,
        })
    });
    e.mem.set_u32(VTABLE + 0x190, FN_ADD_ITEM);
    e.mem.set_u32(VTABLE + 0x48, FN_MARK);
    e.register(FN_ADD_ITEM, |_, a| {
        record(FN_ADD_ITEM, a);
        Ret::default()
    });
    e.register(FN_MARK, |_, a| {
        record(FN_MARK, a);
        Ret::default()
    });
    e
}

fn run(e: &mut Engine, opcode: u32, this: u32, args: &[Vec<u8>]) -> f64 {
    let (ok, result) = call(e, opcode, this, args);
    assert_eq!(ok, 1);
    result
}

fn form_of(v: f64) -> u32 {
    v.to_bits() as u32
}

/// `GetNumItems`: the armor (2 in the base, 1 more in the changes), not the leveled list nor the
/// ammo that is all gone, and the weapon only in the changes; `GetInventoryObject` in that order.
#[test]
fn items_are_counted_base_container_first() {
    let mut e = setup();
    assert_eq!(run(&mut e, 0x144e, REF, &[]), 2.0);
    assert_eq!(eval(&mut e, 0x144e, REF, 0, 0), 2.0);
    assert_eq!(eval(&mut e, 0x144e, 0, 0, 0), 0.0);
    assert_eq!(form_of(run(&mut e, 0x144f, REF, &[n(0)])), 0x0001_0001);
    assert_eq!(form_of(run(&mut e, 0x144f, REF, &[n(1)])), 0x0001_0002);
    assert_eq!(run(&mut e, 0x144f, REF, &[n(2)]), 0.0);
    // Without changes: the base container's entries that hold any.
    e.mem.set_u32(REF + 0x44 + 0x100 + 4 * 0x15, 0);
    assert_eq!(run(&mut e, 0x144e, REF, &[]), 2.0);
    assert_eq!(form_of(run(&mut e, 0x144f, REF, &[n(1)])), 0x0001_0003);
}

/// `PickOneOf`: of the list's forms, those the actor holds (the ammo is all gone, so only the
/// weapon); of two, the one MT19937's next draw picks; none for a non-actor.
#[test]
fn pick_one_of_draws_among_held_forms() {
    let mut e = setup();
    tlist(&mut e, LIST_FORM + 0x18, &[AMMO, WEAPON]);
    assert_eq!(form_of(run(&mut e, 0x159f, REF, &[r(5)])), 0x0001_0002);
    tlist(&mut e, LIST_FORM + 0x18, &[ARMOR, WEAPON, AMMO]);
    e.ext::<crate::Nvse>().mt.init_genrand(5489);
    // genrand_real2 of seed 5489 starts 3499211612 / 2^32 = 0.81..., times 2 is place 1.
    assert_eq!(form_of(run(&mut e, 0x159f, REF, &[r(5)])), 0x0001_0002);
    e.mem.set_u32(REF + 0x7f0, 1);
    assert_eq!(run(&mut e, 0x159f, REF, &[r(5)]), 0.0);
}

/// The token getters: the weapon's first stack's health and owner (also as conditions); the ammo
/// (all gone) and an item the reference does not hold answer 0.
#[test]
fn token_values_and_owners() {
    let mut e = setup();
    assert_eq!(run(&mut e, 0x1506, REF, &[r(2)]), 50.0);
    assert_eq!(eval(&mut e, 0x1506, REF, WEAPON, 0), 50.0);
    assert_eq!(form_of(run(&mut e, 0x1508, REF, &[r(2)])), 0x0001_0005);
    assert_eq!(form_of(eval(&mut e, 0x1508, REF, WEAPON, 0)), 0x0001_0005);
    assert_eq!(run(&mut e, 0x1506, REF, &[r(3)]), 0.0);
    assert_eq!(run(&mut e, 0x1508, REF, &[r(3)]), 0.0);
    assert_eq!(eval(&mut e, 0x1506, 0, WEAPON, 0), 0.0);
    // Without its ExtraHealth the stack answers the weapon's base health.
    e.mem.set_u32(XDATA[0] + 0x100 + 4 * 0x25, 0);
    assert_eq!(run(&mut e, 0x1506, REF, &[r(2)]), 250.0);
}

/// Whether a token command stops the game for the armor, which is in the base container: with a
/// changes entry that has no stacks, or (`entry_missing`) with none.
fn armor_token_stops(entry_missing: bool, opcode: u32, args: &[Vec<u8>]) -> bool {
    let mut e = setup();
    if entry_missing {
        e.mem.set_u32(NODES + 0x10, 0);
    }
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        call(&mut e, opcode, REF, args);
    }))
    .is_err()
}

/// An item of the base container whose changes entry has no stacks, or is missing: xNVSE counts
/// the base container's stack on through the index `GetTokenValue`, `GetTokenRef` and the setters
/// do not pass, or reads the item's health through the missing entry, and the game stops (the
/// real game: `player.GetTokenValue` of the Pip-Boy ends it with an access violation, run
/// `inv6r1`). `GetPaired` passes its index, so only the missing entry stops it.
#[test]
fn a_base_container_token_stops_the_game() {
    for entry_missing in [false, true] {
        assert!(armor_token_stops(entry_missing, 0x1506, &[r(1)]));
        assert!(armor_token_stops(entry_missing, 0x1508, &[r(1)]));
        assert!(armor_token_stops(entry_missing, 0x1507, &[r(1), z(40.0)]));
        assert!(armor_token_stops(entry_missing, 0x1509, &[r(1), r(4)]));
        assert!(armor_token_stops(
            entry_missing,
            0x150a,
            &[r(1), z(40.0), r(4)]
        ));
    }
    assert!(!armor_token_stops(false, 0x150b, &[r(1), r(6)]));
    assert!(armor_token_stops(true, 0x150b, &[r(1), r(6)]));
}

/// The armor's health form answers `GetHealth` 100 (for `AddItem`'s `ExtraHealth`).
fn armor_health(e: &mut Engine) {
    e.mem.set_u32(ARMOR + 0x200, VTABLE + 0x400);
    e.mem.set_u32(VTABLE + 0x400 + 0x10, FN_GET_HEALTH);
    e.register(FN_GET_HEALTH, |_, _| eax(100));
}

/// The same with the list guard row: the search goes on without xNVSE's null reads. The getters
/// answer for the base container's armor (the armor's own base health, 100, read from the found
/// form; no owner), each setter sets the armor's first stack as xNVSE would have, which with no
/// stack is `AddItem` of one at the value (`AddItemHealthPercentOwner` takes it as a share of the
/// base health, as xNVSE does), then marks the reference; `GetPaired` answers 0 (no armor is
/// owned). The weapon, which has a stack, is not touched by any of it.
#[test]
fn a_base_container_token_answers_with_the_list_guard() {
    for entry_missing in [false, true] {
        let mut e = setup();
        crate::commands::array::set_fix(&mut e, crate::commands::array::LIST_GUARD, true);
        armor_health(&mut e);
        if entry_missing {
            e.mem.set_u32(NODES + 0x10, 0);
        }
        assert_eq!(run(&mut e, 0x1506, REF, &[r(1)]), 100.0);
        assert_eq!(eval(&mut e, 0x1506, REF, ARMOR, 0), 100.0);
        assert_eq!(run(&mut e, 0x1508, REF, &[r(1)]), 0.0);
        assert_eq!(form_of(eval(&mut e, 0x1508, REF, ARMOR, 0)), 0);
        run(&mut e, 0x1507, REF, &[r(1), z(0.4)]);
        let added = calls_to(FN_ADD_ITEM);
        assert_eq!(added.len(), 1);
        assert_eq!((added[0][1], added[0][3]), (ARMOR, 1));
        assert_eq!(e.mem.f32(extra_of(&e, added[0][2], 0x25) + 0xc), 40.0);
        assert_eq!(calls_to(FN_MARK), vec![vec![REF, 0x0800_0000]]);
        run(&mut e, 0x1509, REF, &[r(1), r(4)]);
        let added = calls_to(FN_ADD_ITEM);
        assert_eq!(added.len(), 1);
        assert_eq!(e.mem.u32(extra_of(&e, added[0][2], 0x21) + 0xc), OWNER);
        run(&mut e, 0x150a, REF, &[r(1), z(0.5), r(4)]);
        assert_eq!(calls_to(FN_ADD_ITEM).len(), 1);
        assert_eq!(calls_to(FN_MARK).len(), 2);
        assert_eq!(e.mem.u32(NODES + 0x124), 1, "the armor's count delta");
        assert_eq!(run(&mut e, 0x150b, REF, &[r(1), r(6)]), 0.0);
        assert_eq!(eval(&mut e, 0x150b, REF, ARMOR, REF), 0.0);
        assert_eq!(
            run(&mut e, 0x1506, REF, &[r(2)]),
            50.0,
            "the weapon's stack"
        );
    }
}

/// Review of #384, point A: container changes without data (what xNVSE itself makes after
/// `CopyIR` or `RemoveMeIR` into a container that had none) are no changes to every finder, as
/// `GetEntryDataList` gives them: the base container's items are counted and found, an item only
/// the changes could hold is not there, and a setter adds it (twice: the set finds no entry
/// either, as with no changes).
#[test]
fn changes_without_data_are_no_changes() {
    let mut e = setup();
    e.mem.set_u32(CHANGES + 0xc, 0);
    e.mem.set_u32(WEAPON + 0x200, VTABLE + 0x400);
    e.mem.set_u32(VTABLE + 0x400 + 0x10, FN_GET_HEALTH);
    e.register(FN_GET_HEALTH, |_, _| eax(250));
    assert_eq!(run(&mut e, 0x144e, REF, &[]), 2.0, "the armor and the ammo");
    assert_eq!(eval(&mut e, 0x144e, REF, 0, 0), 2.0);
    assert_eq!(form_of(run(&mut e, 0x144f, REF, &[n(1)])), 0x0001_0003);
    tlist(&mut e, LIST_FORM + 0x18, &[AMMO, WEAPON]);
    assert_eq!(form_of(run(&mut e, 0x159f, REF, &[r(5)])), 0x0001_0003);
    assert_eq!(run(&mut e, 0x1506, REF, &[r(2)]), 0.0);
    assert_eq!(run(&mut e, 0x1508, REF, &[r(2)]), 0.0);
    assert_eq!(run(&mut e, 0x150b, REF, &[r(2), r(6)]), 0.0);
    run(&mut e, 0x1507, REF, &[r(2), z(0.5)]);
    let added = calls_to(FN_ADD_ITEM);
    assert_eq!(added.len(), 2);
    assert_eq!((added[0][1], added[0][3]), (WEAPON, 1));
}

/// Review of #384, point C: a changes entry list that runs back into itself stops the game
/// without the list guard row, and with it ends the walk: the commands answer 0.
#[test]
fn a_cyclic_entry_list_stops_the_game_or_ends_the_walk() {
    for guard in [false, true] {
        let mut e = setup();
        crate::commands::array::set_fix(&mut e, crate::commands::array::LIST_GUARD, guard);
        e.mem.set_u32(NODES + 0x14, NODES);
        let answers = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            [
                run(&mut e, 0x144e, REF, &[]),
                run(&mut e, 0x144f, REF, &[n(1)]),
                run(&mut e, 0x1506, REF, &[r(2)]),
                run(&mut e, 0x150b, REF, &[r(2), r(6)]),
            ]
        }));
        if guard {
            assert_eq!(answers.ok(), Some([0.0; 4]));
        } else {
            assert!(answers.is_err());
        }
    }
}

/// Review of #384, point C: `GetPaired` over a stack list with an empty node after the first (a
/// stack without an owner, then the empty node) finds the item with its index not counted on,
/// and xNVSE would ask for the same stack for ever (the game hangs). It ends at the first such
/// answer: with the list guard row as not paired, without it the game stops there.
#[test]
fn get_paired_ends_where_the_index_stays() {
    for guard in [false, true] {
        let mut e = setup();
        crate::commands::array::set_fix(&mut e, crate::commands::array::LIST_GUARD, guard);
        tlist(&mut e, NODES + 0x40, &[XDATA[1], 0, XDATA[0]]);
        let answer = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            run(&mut e, 0x150b, REF, &[r(2), r(6)])
        }));
        if guard {
            assert_eq!(answer.ok(), Some(0.0));
        } else {
            assert!(answer.is_err());
        }
    }
}

/// Review of #384, point D: a base container entry without a form. xNVSE counts it, gives it as
/// an item without a form, and its finders by id read through it (the game stops); with the
/// inventory reference checks row it is passed over, as in `GetInventoryItems`.
#[test]
fn a_base_entry_without_a_form() {
    for checks in [false, true] {
        let mut e = setup();
        invref::set_checks(&mut e, checks);
        e.mem.set_u32(NODES + 0x210 + 4, 0);
        tlist(&mut e, LIST_FORM + 0x18, &[WEAPON]);
        if checks {
            assert_eq!(run(&mut e, 0x144e, REF, &[]), 2.0);
            assert_eq!(form_of(run(&mut e, 0x144f, REF, &[n(1)])), 0x0001_0002);
            assert_eq!(form_of(run(&mut e, 0x159f, REF, &[r(5)])), 0x0001_0002);
            assert_eq!(run(&mut e, 0x1506, REF, &[r(2)]), 50.0);
        } else {
            assert_eq!(run(&mut e, 0x144e, REF, &[]), 3.0);
            assert_eq!(run(&mut e, 0x144f, REF, &[n(1)]), 0.0);
            let stop = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                call(&mut e, 0x159f, REF, &[r(5)]);
            }));
            assert!(stop.is_err());
        }
    }
}

/// `PickOneOf` of an empty list answers 0; of the armor, held in the base container and in the
/// changes, the armor.
#[test]
fn pick_one_of_an_empty_list_and_an_item_held_twice() {
    let mut e = setup();
    assert_eq!(run(&mut e, 0x159f, REF, &[r(5)]), 0.0);
    tlist(&mut e, LIST_FORM + 0x18, &[ARMOR]);
    assert_eq!(form_of(run(&mut e, 0x159f, REF, &[r(5)])), 0x0001_0001);
}

/// The token getters and setters on what the review listed: a stack list whose head node is
/// empty (`tList::Count` 0, so no stack is read and the weapon, not in the base container,
/// answers 0); `GetTokenRef` on a stack without an owner (0); `SetTokenValue` at the
/// holder's base form's health (the `ExtraHealth` removed, as xNVSE compares with the container's
/// health) and at -1 (the health kept); `SetTokenRef` with a new owner (the rank removed);
/// `SetTokenValueAndRef` on a stack with an `ExtraCount` (set to 1).
#[test]
fn token_edge_cases() {
    let mut e = setup();
    tlist(&mut e, NODES + 0x40, &[0, XDATA[0]]);
    assert_eq!(run(&mut e, 0x1506, REF, &[r(2)]), 0.0);
    tlist(&mut e, NODES + 0x40, &[XDATA[0]]);
    let owner = XDATA[0] + 0x100 + 4 * 0x21;
    e.mem.set_u32(owner, 0);
    assert_eq!(run(&mut e, 0x1508, REF, &[r(2)]), 0.0);
    e.mem.set_u32(owner, XDATA[0] + 0x390);
    // The holder's base form has health 60: setting 60 removes the stack's ExtraHealth.
    e.mem.set_u32(BASE + 0x7f0, (1 << 23) | 4);
    e.mem.set_u32(BASE + 0x200 + 4, 60);
    run(&mut e, 0x1507, REF, &[r(2), z(60.0)]);
    assert_eq!(
        calls_to(0x0041_0020),
        vec![vec![XDATA[0], XDATA[0] + 0x380, 1]]
    );
    put_extra(&mut e, XDATA[0], 0x25, XDATA[0] + 0x380);
    e.mem.set_f32(XDATA[0] + 0x38c, 50.0);
    e.mem.set_u32(NODES + 0x114, 3);
    run(&mut e, 0x1507, REF, &[r(2), z(-1.0)]);
    assert_eq!(e.mem.f32(XDATA[0] + 0x38c), 50.0, "kept");
    assert_eq!(e.mem.u32(NODES + 0x114), 1);
    // A rank with the owner: a new owner drops it.
    put_extra(&mut e, XDATA[0], 0x23, XDATA[0] + 0x3a0);
    e.mem.set_u32(XDATA[0] + 0x3ac, 2);
    run(&mut e, 0x1509, REF, &[r(2), r(1)]);
    assert_eq!(e.mem.u32(XDATA[0] + 0x39c), ARMOR);
    assert_eq!(
        calls_to(0x0041_0020),
        vec![vec![XDATA[0], XDATA[0] + 0x3a0, 1]]
    );
    // An ExtraCount of 5 on the stack becomes 1.
    put_extra(&mut e, XDATA[0], 0x24, XDATA[0] + 0x3b0);
    e.mem.set_u16(XDATA[0] + 0x3bc, 5);
    run(&mut e, 0x150a, REF, &[r(2), z(10.0), r(4)]);
    assert_eq!(e.mem.u16(XDATA[0] + 0x3bc), 1);
    assert_eq!(e.mem.f32(XDATA[0] + 0x38c), 10.0);
    assert_eq!(e.mem.u32(XDATA[0] + 0x39c), OWNER);
}

/// `AddItemOwnership` and `AddItemHealthPercentOwner` of a scripted item: the new list also gets
/// an `ExtraScript` with the item's script and a new event list (`CreateEventList`); the count is
/// the float truncated; a rank is added with `AddItemOwnership` too, and no health.
#[test]
fn a_scripted_item_is_added_with_its_script() {
    const ITEM_SCRIPT: u32 = 0x0200_9800;
    const EVENTS: u32 = 0x0200_9900;
    let mut e = setup();
    e.mem
        .set_u32(WEAPON + 0x7f0, e.mem.u32(WEAPON + 0x7f0) | (1 << 25));
    e.mem.set_u32(WEAPON + 0x5c0 + 4, ITEM_SCRIPT);
    e.register(0x005a_bf60, |_, a| {
        record(0x005a_bf60, a);
        eax(EVENTS)
    });
    run(&mut e, 0x1504, REF, &[r(2), z(2.7), r(4), n(3)]);
    let added = calls_to(FN_ADD_ITEM);
    let (list, count) = (added[0][2], added[0][3]);
    assert_eq!(count, 2, "2.7 truncated");
    let script = extra_of(&e, list, 0x0d);
    assert_eq!(e.mem.u32(script), 0x0101_5914);
    assert_eq!(e.mem.u32(script + 0xc), ITEM_SCRIPT);
    assert_eq!(e.mem.u32(script + 0x10), EVENTS);
    assert_eq!(calls_to(0x005a_bf60), vec![vec![ITEM_SCRIPT]]);
    assert_eq!(e.mem.u32(extra_of(&e, list, 0x21) + 0xc), OWNER);
    assert_eq!(e.mem.u32(extra_of(&e, list, 0x23) + 0xc), 3);
    assert_eq!(extra_of(&e, list, 0x25), 0, "no health");
    e.mem.set_u32(WEAPON + 0x200, VTABLE + 0x400);
    e.mem.set_u32(VTABLE + 0x400 + 0x10, FN_GET_HEALTH);
    e.register(FN_GET_HEALTH, |_, _| eax(250));
    run(&mut e, 0x1505, REF, &[r(2), z(1.0), z(1.0)]);
    let list = calls_to(FN_ADD_ITEM)[0][2];
    assert_ne!(list, 0, "a list for the script alone");
    assert_eq!(e.mem.u32(extra_of(&e, list, 0x0d) + 0xc), ITEM_SCRIPT);
    assert_eq!(extra_of(&e, list, 0x21), 0);
}

/// The token setters on the weapon's stack: a new health (the stack's `ExtraHealth` changed), a new
/// owner (changed); `SetTokenValueAndRef` both; each time the count is set to 1 and the reference
/// marked as changed in its inventory.
#[test]
fn token_setters_change_the_first_stack() {
    let mut e = setup();
    e.mem.set_u32(NODES + 0x114, 3);
    run(&mut e, 0x1507, REF, &[r(2), z(75.0)]);
    assert_eq!(e.mem.f32(XDATA[0] + 0x38c), 75.0);
    assert_eq!(e.mem.u32(NODES + 0x114), 1, "the count delta");
    assert_eq!(calls_to(FN_MARK), vec![vec![REF, 0x0800_0000]]);
    run(&mut e, 0x1509, REF, &[r(2), r(1)]);
    assert_eq!(e.mem.u32(XDATA[0] + 0x39c), ARMOR, "the new owner");
    assert_eq!(e.mem.f32(XDATA[0] + 0x38c), 75.0, "the health kept");
    run(&mut e, 0x150a, REF, &[r(2), z(10.0), r(4)]);
    assert_eq!(e.mem.f32(XDATA[0] + 0x38c), 10.0);
    assert_eq!(e.mem.u32(XDATA[0] + 0x39c), OWNER);
    assert_eq!(calls_to(FN_MARK).len(), 2);
}

/// A token the reference does not hold (an item with health) is added at that health and owner,
/// then set: `AddItem` gets a new list with an `ExtraHealth` of the base health times the value.
#[test]
fn a_missing_token_is_added() {
    let mut e = setup();
    e.mem.set_u32(NODES + 0x10, 0);
    e.mem.set_u32(ARMOR, VTABLE + 0x400);
    e.mem.set_u32(ARMOR + 0x200, VTABLE + 0x400);
    e.mem.set_u32(VTABLE + 0x400 + 0x10, FN_GET_HEALTH);
    e.register(FN_GET_HEALTH, |_, _| eax(100));
    // No armor entry and none in the base container: the lookup finds nothing.
    e.mem.set_u32(NODES + 0x200, 0);
    run(&mut e, 0x1507, REF, &[r(1), z(0.5)]);
    // The stand-in AddItem makes no entry, so the set that follows finds none and adds again.
    let added = calls_to(FN_ADD_ITEM);
    assert_eq!(added.len(), 2);
    let (form, list, count) = (added[0][1], added[0][2], added[0][3]);
    assert_eq!((form, count), (ARMOR, 1));
    assert_eq!(e.mem.u32(list), 0x0101_43e8);
    let health = extra_of(&e, list, 0x25);
    assert_eq!(e.mem.f32(health + 0xc), 50.0);
    assert_eq!(e.mem.u32(health), 0x0101_58e4);
    assert_eq!(calls_to(FN_MARK).len(), 2, "added, then set");
}

/// `AddItemHealthPercentOwner` and `AddItemOwnership`: the count, an `ExtraHealth` of the health
/// share (none for a health of 1, or with `AddItemOwnership`), the owner and the rank on a new list;
/// an item without health takes no health other than -1.
#[test]
fn items_are_added_with_health_and_owner() {
    let mut e = setup();
    e.mem.set_u32(WEAPON + 0x200, VTABLE + 0x400);
    e.mem.set_u32(VTABLE + 0x400 + 0x10, FN_GET_HEALTH);
    e.register(FN_GET_HEALTH, |_, _| eax(250));
    run(&mut e, 0x1505, REF, &[r(2), z(2.0), z(0.2), r(4), n(3)]);
    let added = calls_to(FN_ADD_ITEM);
    let (list, count) = (added[0][2], added[0][3]);
    assert_eq!(count, 2);
    assert_eq!(e.mem.f32(extra_of(&e, list, 0x25) + 0xc), 50.0);
    assert_eq!(e.mem.u32(extra_of(&e, list, 0x21) + 0xc), OWNER);
    assert_eq!(e.mem.u32(extra_of(&e, list, 0x23) + 0xc), 3);
    assert_eq!(e.mem.u32(extra_of(&e, list, 0x23)), 0x0101_58cc);
    run(&mut e, 0x1505, REF, &[r(2), z(1.0), z(1.0), r(4)]);
    let list = calls_to(FN_ADD_ITEM)[0][2];
    assert_eq!(extra_of(&e, list, 0x25), 0, "a health of 1");
    assert_eq!(extra_of(&e, list, 0x23), 0, "no rank");
    run(&mut e, 0x1504, REF, &[r(3), z(5.0), r(4)]);
    let added = calls_to(FN_ADD_ITEM);
    assert_eq!((added[0][1], added[0][3]), (AMMO, 5));
    assert_eq!(extra_of(&e, added[0][2], 0x25), 0);
    run(&mut e, 0x1505, REF, &[r(3), z(1.0), z(0.5), r(4)]);
    assert!(calls_to(FN_ADD_ITEM).is_empty(), "ammo has no health");
}

/// `GetPaired`: the reference's weapon stack is owned by the actor and the actor's by the
/// reference; not when either owner differs.
#[test]
fn paired_owners_cross_reference() {
    let mut e = setup();
    // The actor: changes with one weapon stack owned by REF.
    e.mem.set_u8(ACTOR + 4, 0x3b);
    e.mem.set_u32(ACTOR + 0x7f0, 1 | (1 << 21));
    e.mem.set_u32(ACTOR + 0x20, BASE + 0x40);
    let changes = CHANGES + 0x80;
    put_extra(&mut e, ACTOR + 0x44, 0x15, changes);
    e.mem.set_u32(changes + 0xc, changes + 0x20);
    e.mem.set_u32(changes + 0x20, NODES + 0x80);
    let theirs = entry(&mut e, NODES + 0x130, NODES + 0x90, 1, WEAPON);
    tlist(&mut e, NODES + 0x80, &[theirs]);
    tlist(&mut e, NODES + 0x90, &[XDATA[1]]);
    put_extra(&mut e, XDATA[1], 0x21, XDATA[1] + 0x390);
    e.mem.set_u32(XDATA[1] + 0x39c, REF);
    e.mem.set_u32(XDATA[0] + 0x39c, ACTOR);
    set_script_ref(&mut e, 7, ACTOR);
    assert_eq!(run(&mut e, 0x150b, REF, &[r(2), r(7)]), 1.0);
    assert_eq!(eval(&mut e, 0x150b, REF, WEAPON, ACTOR), 1.0);
    e.mem.set_u32(XDATA[1] + 0x39c, OWNER);
    assert_eq!(eval(&mut e, 0x150b, REF, WEAPON, ACTOR), 0.0);
    e.mem.set_u32(XDATA[1] + 0x39c, REF);
    e.mem.set_u32(XDATA[0] + 0x39c, OWNER);
    assert_eq!(eval(&mut e, 0x150b, REF, WEAPON, ACTOR), 0.0);
    assert_eq!(eval(&mut e, 0x150b, REF, 0, ACTOR), 0.0);
}

/// The real game's answers (xNVSE 6.4.9 with JIP LN 57.30, `real-tests/gr_s_tokens.txt` and
/// `gr_t_pick.txt`, run `inv6r1`, after `nvsecos4` loads, on the player) on stand-ins laid out
/// the same way.
/// - The 9mm pistol (base health 150): its one stack reads 51; after the game's `AddItem` of
///   another the first stack reads 149.85 and there are 2. `SetTokenValue 50` reads back 50 and
///   leaves 1 (the count is set, not kept), 75 the same. `SetTokenRef` with the holder itself reads
///   the holder back, keeps 75, and `GetPaired` with the holder answers 1.
/// - `AddItemHealthPercentOwner` of 2 at 0.5 adds 2 (1 became 3) at health 75 with the owner;
///   `AddItemOwnership` of 5 rounds adds 5 (20 became 25) with the owner and no health.
/// - A Varmint Rifle (base health 120) added at 0.5: its stack reads 60, and no owner, the game
///   having taken the holder's own ownership off. `SetTokenValueAndRef 30` with another actor reads
///   30 and that actor; `GetPaired` with that actor, who holds none, answers 0.
/// - `PickOneOf` over a list of one form the holder has answers that form.
#[test]
fn the_real_games_answers() {
    let mut e = setup();
    let delta = NODES + 0x114;
    e.mem.set_u32(WEAPON + 0x200 + 4, 150);
    e.mem.set_u32(WEAPON + 0x200, VTABLE + 0x400);
    e.mem.set_u32(VTABLE + 0x400 + 0x10, FN_GET_HEALTH);
    e.register(FN_GET_HEALTH, |_, _| eax(150));
    e.mem.set_f32(XDATA[0] + 0x38c, 51.0);
    e.mem.set_u32(XDATA[0] + 0x100 + 4 * 0x21, 0);
    assert_eq!(run(&mut e, 0x1506, REF, &[r(2)]), 51.0);
    // What the game's AddItem left: 2 of them, and a first stack that reads 149.85.
    put_extra(&mut e, XDATA[1], 0x25, XDATA[1] + 0x380);
    e.mem.set_f32(XDATA[1] + 0x38c, 149.85);
    tlist(&mut e, NODES + 0x40, &[XDATA[1], XDATA[0]]);
    e.mem.set_u32(delta, 2);
    assert_eq!(run(&mut e, 0x1506, REF, &[r(2)]), f64::from(149.85f32));
    assert_eq!(run(&mut e, 0x1508, REF, &[r(2)]), 0.0);
    for value in [50.0, 75.0] {
        run(&mut e, 0x1507, REF, &[r(2), z(value)]);
        assert_eq!(run(&mut e, 0x1506, REF, &[r(2)]), value);
        assert_eq!(e.mem.u32(delta), 1);
    }
    run(&mut e, 0x1509, REF, &[r(2), r(6)]);
    assert_eq!(form_of(run(&mut e, 0x1508, REF, &[r(2)])), 0x0001_0010);
    assert_eq!(run(&mut e, 0x1506, REF, &[r(2)]), 75.0);
    assert_eq!(run(&mut e, 0x150b, REF, &[r(2), r(6)]), 1.0);
    assert_eq!(e.mem.u32(delta), 1);
    calls_to(FN_ADD_ITEM);
    run(&mut e, 0x1505, REF, &[r(2), z(2.0), z(0.5), r(6)]);
    let added = calls_to(FN_ADD_ITEM);
    assert_eq!((added[0][1], added[0][3]), (WEAPON, 2));
    assert_eq!(e.mem.f32(extra_of(&e, added[0][2], 0x25) + 0xc), 75.0);
    assert_eq!(e.mem.u32(extra_of(&e, added[0][2], 0x21) + 0xc), REF);
    run(&mut e, 0x1504, REF, &[r(3), z(5.0), r(6)]);
    let added = calls_to(FN_ADD_ITEM);
    assert_eq!((added[0][1], added[0][3]), (AMMO, 5));
    assert_eq!(extra_of(&e, added[0][2], 0x25), 0);
    assert_eq!(e.mem.u32(extra_of(&e, added[0][2], 0x21) + 0xc), REF);
    // The rifle: the armor stands in, with a base health of 120.
    e.mem.set_u32(ARMOR + 0x200 + 4, 120);
    e.mem.set_u32(ARMOR + 0x200, VTABLE + 0x500);
    e.mem.set_u32(VTABLE + 0x500 + 0x10, FN_GET_HEALTH + 4);
    e.register(FN_GET_HEALTH + 4, |_, _| eax(120));
    run(&mut e, 0x1505, REF, &[r(1), z(1.0), z(0.5), r(6)]);
    let added = calls_to(FN_ADD_ITEM);
    let list = added[0][2];
    assert_eq!((added[0][1], added[0][3]), (ARMOR, 1));
    assert_eq!(e.mem.f32(extra_of(&e, list, 0x25) + 0xc), 60.0);
    // Its stack as the game left it: the health kept, the holder's own ownership taken off.
    e.mem.set_u32(list + 0x100 + 4 * 0x21, 0);
    tlist(&mut e, NODES + 0x60, &[list]);
    e.mem.set_u32(NODES + 0x120, NODES + 0x60);
    assert_eq!(run(&mut e, 0x1506, REF, &[r(1)]), 60.0);
    assert_eq!(run(&mut e, 0x1508, REF, &[r(1)]), 0.0);
    e.mem.set_u8(ACTOR + 4, 0x3b);
    e.mem.set_u32(ACTOR + 0x7f0, 1 | (1 << 21));
    e.mem.set_u32(ACTOR + 0xc, 0x0001_0020);
    e.mem.set_u32(ACTOR + 0x20, BASE + 0x40);
    set_script_ref(&mut e, 7, ACTOR);
    run(&mut e, 0x150a, REF, &[r(1), z(30.0), r(7)]);
    assert_eq!(run(&mut e, 0x1506, REF, &[r(1)]), 30.0);
    assert_eq!(form_of(run(&mut e, 0x1508, REF, &[r(1)])), 0x0001_0020);
    assert_eq!(run(&mut e, 0x150b, REF, &[r(1), r(7)]), 0.0);
    assert_eq!(e.mem.u32(NODES + 0x124), 1);
    tlist(&mut e, LIST_FORM + 0x18, &[WEAPON]);
    assert_eq!(form_of(run(&mut e, 0x159f, REF, &[r(5)])), 0x0001_0002);
}
