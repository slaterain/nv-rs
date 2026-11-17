//! O18 part 1 (`uNPCPerks`): the NPC perk routines, case by case from JIP's source.

use super::*;
use crate::cosave::{changed, changed_flags, set_changed, set_npc_perks_patch, NpcPerks};
use std::cell::{Cell, RefCell};

/// The test's objects (64 KB) and the heap stand-in's blocks after them.
const AREA: u32 = 0x0400_0000;
const HEAP: u32 = 0x0401_0000;
/// An entry's vtable and the stand-ins it points to; the entry's kind for `GetType` is kept at
/// the entry's `+0x10` (the test's own word).
const ENTRY_VT: u32 = AREA;
const GET_TYPE_AT: u32 = 0x0040_1f00;
const ADD_ENTRY_AT: u32 = 0x0040_1f04;
const REMOVE_ENTRY_AT: u32 = 0x0040_1f08;
const ACTOR: u32 = AREA + 0x1000;
const ACTOR_ID: u32 = 0x0700_0001;
const BASE: u32 = AREA + 0x1400;
const RENDER: u32 = AREA + 0x1500;
const PLAYER: u32 = AREA + 0x2000;
const HANDLER: u32 = AREA + 0x6000;

thread_local! {
    static NEXT: Cell<u32> = const { Cell::new(HEAP) };
    static FREED: RefCell<Vec<u32>> = const { RefCell::new(Vec::new()) };
    static ENTRY_CALLS: RefCell<Vec<(u32, u32, u32)>> = const { RefCell::new(Vec::new()) };
    static DRAWS: RefCell<Vec<u32>> = const { RefCell::new(Vec::new()) };
    static RELOADS: RefCell<Vec<u32>> = const { RefCell::new(Vec::new()) };
}

fn ret(eax: u32) -> Ret {
    Ret {
        eax,
        ..Ret::default()
    }
}

/// An engine with the heap, an entry's virtual calls, the random draws (taken in order, 0 when
/// none are left), the reload speed update and the level (`GetLevel`: 12) as stand-ins; the actor
/// (a `Character`, alive, its 3D loaded) and the player.
fn world() -> Engine {
    let mut e = crate::test_support::engine();
    e.map(AREA, 0x2_0000);
    e.map(0x011c_3000, 0x1000);
    e.map(0x011d_e000, 0x1000);
    e.map(0x011e_0000, 0x1000);
    NEXT.with(|n| n.set(HEAP));
    FREED.with(|f| f.borrow_mut().clear());
    ENTRY_CALLS.with(|c| c.borrow_mut().clear());
    DRAWS.with(|d| d.borrow_mut().clear());
    RELOADS.with(|r| r.borrow_mut().clear());
    e.register(0x00aa_3e40, |_, a| {
        let at = NEXT.with(|n| n.get());
        NEXT.with(|n| n.set(at + a[1].next_multiple_of(8)));
        ret(at)
    });
    e.register(0x00aa_4060, |_, a| {
        FREED.with(|f| f.borrow_mut().push(a[1]));
        ret(0)
    });
    e.register(GET_TYPE_AT, |e, a| ret(e.mem.u32(a[0] + 0x10)));
    e.register(ADD_ENTRY_AT, |_, a| {
        ENTRY_CALLS.with(|c| c.borrow_mut().push((1, a[0], a[1])));
        ret(0)
    });
    e.register(REMOVE_ENTRY_AT, |_, a| {
        ENTRY_CALLS.with(|c| c.borrow_mut().push((0, a[0], a[1])));
        ret(0)
    });
    e.register(crate::game_patches::gh1::GET_RANDOM_INT, |_, _| {
        ret(DRAWS.with(|d| {
            let mut d = d.borrow_mut();
            if d.is_empty() {
                0
            } else {
                d.remove(0)
            }
        }))
    });
    e.register(0x008c_17c0, |_, a| {
        RELOADS.with(|r| r.borrow_mut().push(a[0]));
        ret(0)
    });
    e.register(0x0047_ded0, |_, _| ret(12));
    e.mem.set_u32(ENTRY_VT + 0x10, GET_TYPE_AT);
    e.mem.set_u32(ENTRY_VT + 0x30, ADD_ENTRY_AT);
    e.mem.set_u32(ENTRY_VT + 0x34, REMOVE_ENTRY_AT);
    e.mem.set_u8(ACTOR + 4, 0x3b);
    e.mem.set_u32(ACTOR + 0xc, ACTOR_ID);
    e.mem.set_u32(ACTOR + 0x20, BASE);
    e.mem.set_u32(ACTOR + 0x64, RENDER);
    e.mem.set_u32(RENDER + 0x14, 0x7777);
    e.mem.set_u32(0x011d_ea3c, PLAYER);
    e.mem.set_u32(0x011c_3f2c, HANDLER);
    e
}

/// A perk at `n` (0x100 bytes each from `AREA + 0x3000`): trait, minimum level, ranks, playable,
/// hidden, and its entries (a list of them, nodes after the perk).
fn perk(e: &mut Engine, n: u32, data: [u8; 5], entries: &[u32]) -> u32 {
    let p = AREA + 0x3000 + 0x100 * n;
    for (i, b) in data.iter().enumerate() {
        e.mem.set_u8(p + 0x38 + i as u32, *b);
    }
    list(e, p + 0x48, p + 0x80, entries);
    p
}

/// A list at `head` with its later nodes from `nodes`.
fn list(e: &mut Engine, head: u32, nodes: u32, items: &[u32]) {
    let mut node = head;
    for (i, &item) in items.iter().enumerate() {
        e.mem.set_u32(node, item);
        let next = if i + 1 < items.len() {
            nodes + 8 * i as u32
        } else {
            0
        };
        e.mem.set_u32(node + 4, next);
        node = next;
    }
}

/// An entry at `n` (0x20 bytes each from `AREA + 0x5000`): its kind (0 quest, 1 ability, 2 entry
/// point), rank and entry point (or spell).
fn entry(e: &mut Engine, n: u32, kind: u32, rank: u8, point: u32) -> u32 {
    let at = AREA + 0x5000 + 0x20 * n;
    e.mem.set_u32(at, ENTRY_VT);
    e.mem.set_u8(at + 4, rank);
    e.mem.set_u32(at + 8, point);
    e.mem.set_u32(at + 0x10, kind);
    at
}

fn calls() -> Vec<(u32, u32, u32)> {
    ENTRY_CALLS.with(|c| c.take())
}

fn ranks(e: &mut Engine) -> Option<NpcPerks> {
    crate::cosave::npc_perks(e).get(&ACTOR_ID).cloned()
}

fn items_of(e: &Engine, block: u32, point: u8) -> Vec<u32> {
    let list = entry_list(block, point).unwrap();
    list_items(e, list)
}

/// `BuildValidNPCPerks`: playable, not hidden, entries none empty or a quest's, abilities of
/// non-script effects only, and an entry point NPCs get; traits apart.
#[test]
fn the_valid_perks_and_traits() {
    let mut e = world();
    let (good_point, bad_point) = (entry(&mut e, 0, 2, 0, 0), entry(&mut e, 1, 2, 0, 3));
    let spell = AREA + 0x7000;
    let (effect, setting) = (AREA + 0x7100, AREA + 0x7200);
    e.mem.set_u32(spell + 0x28, effect);
    e.mem.set_u32(effect + 0x14, setting);
    let ability = entry(&mut e, 2, 1, 0, spell);
    let quest = entry(&mut e, 3, 0, 0, 0);
    let ok = perk(&mut e, 0, [0, 0, 1, 1, 0], &[ability, good_point]);
    let tr = perk(&mut e, 1, [1, 0, 1, 1, 0], &[good_point]);
    let hidden = perk(&mut e, 2, [0, 0, 1, 1, 1], &[good_point]);
    let unplayable = perk(&mut e, 3, [0, 0, 1, 0, 0], &[good_point]);
    let no_point = perk(&mut e, 4, [0, 0, 1, 1, 0], &[bad_point]);
    let with_quest = perk(&mut e, 5, [0, 0, 1, 1, 0], &[good_point, quest]);
    let with_null = perk(&mut e, 6, [0, 0, 1, 1, 0], &[good_point, 0]);
    let empty = perk(&mut e, 7, [0, 0, 1, 1, 0], &[]);
    let all = [
        ok, tr, hidden, unplayable, no_point, with_quest, with_null, empty, 0,
    ];
    list(&mut e, HANDLER + 0x178, AREA + 0x6100, &all);
    assert_eq!(build_valid_npc_perks(&mut e), (vec![ok], vec![tr]));
    // A script effect, or none, makes the ability invalid.
    e.mem.set_u8(setting + 0x98, 1);
    assert_eq!(build_valid_npc_perks(&mut e), (vec![], vec![tr]));
    e.mem.set_u8(setting + 0x98, 2);
    e.mem.set_u32(effect + 0x14, 0);
    assert_eq!(build_valid_npc_perks(&mut e).0, Vec::<u32>::new());
}

/// `SetPerkRankNPCHook`: the rank kept and the entries of that rank put on (the entry point's
/// list, the ability added; another rank's ability removed; a quest entry passed over; a reload
/// speed entry updates the reload speed); a lower rank changes nothing; a dead actor nothing.
#[test]
fn set_rank_puts_the_entries_on() {
    let mut e = world();
    let point = entry(&mut e, 0, 2, 1, 0);
    let reload = entry(&mut e, 1, 2, 1, 25);
    let ability = entry(&mut e, 2, 1, 1, 0);
    let other = entry(&mut e, 3, 1, 0, 0);
    let quest = entry(&mut e, 4, 0, 1, 0);
    let p = perk(
        &mut e,
        0,
        [0, 0, 3, 1, 0],
        &[point, reload, ability, other, quest],
    );
    set_changed(&mut e, 0);
    set_perk_rank_npc(&mut e, &[ACTOR, p, 2, 0]);
    let info = ranks(&mut e).unwrap();
    assert_eq!(info.ranks.into_iter().collect::<Vec<_>>(), [(p, 2)]);
    assert_ne!(info.entry_points, 0);
    assert_eq!(items_of(&e, info.entry_points, 0), [point]);
    assert_eq!(items_of(&e, info.entry_points, 25), [reload]);
    assert_eq!(calls(), [(1, ability, ACTOR), (0, other, ACTOR)]);
    assert_eq!(RELOADS.with(|r| r.take()), [ACTOR]);
    assert_eq!(
        changed_flags(&mut e) & changed::NPC_PERKS,
        changed::NPC_PERKS
    );
    // Again at the same or a lower rank: nothing.
    set_perk_rank_npc(&mut e, &[ACTOR, p, 1, 0]);
    assert!(calls().is_empty());
    // A higher rank: the rank-1 entries come off their lists.
    set_perk_rank_npc(&mut e, &[ACTOR, p, 3, 0]);
    let info = ranks(&mut e).unwrap();
    assert_eq!(info.ranks[&p], 3);
    assert!(items_of(&e, info.entry_points, 0) == [0]);
    assert_eq!(calls(), [(0, ability, ACTOR), (0, other, ACTOR)]);
    // A dead actor: nothing.
    let mut e = world();
    let p = perk(&mut e, 0, [0, 0, 3, 1, 0], &[]);
    e.mem.set_u32(ACTOR + 0x108, 2);
    set_perk_rank_npc(&mut e, &[ACTOR, p, 1, 0]);
    assert_eq!(ranks(&mut e), None);
}

/// Without its 3D the actor keeps the rank, with no lists and no entries put on.
#[test]
fn set_rank_without_3d_keeps_the_rank_only() {
    let mut e = world();
    e.mem.set_u32(ACTOR + 0x64, 0);
    let ability = entry(&mut e, 0, 1, 0, 0);
    let p = perk(&mut e, 0, [0, 0, 1, 1, 0], &[ability]);
    set_perk_rank_npc(&mut e, &[ACTOR, p, 1, 0]);
    let info = ranks(&mut e).unwrap();
    assert_eq!((info.ranks[&p], info.entry_points), (1, 0));
    assert!(calls().is_empty());
}

/// `RemovePerkNPCHook`: the entries taken off (lists, abilities, the reload speed) and, with no
/// perk left, the entry and its lists gone; a perk not had: nothing.
#[test]
fn remove_takes_the_entries_off() {
    let mut e = world();
    let point = entry(&mut e, 0, 2, 0, 25);
    let ability = entry(&mut e, 1, 1, 0, 0);
    let p = perk(&mut e, 0, [0, 0, 1, 1, 0], &[point, ability]);
    let q = perk(&mut e, 1, [0, 0, 1, 1, 0], &[]);
    set_perk_rank_npc(&mut e, &[ACTOR, p, 1, 0]);
    set_perk_rank_npc(&mut e, &[ACTOR, q, 1, 0]);
    calls();
    RELOADS.with(|r| r.take());
    let block = ranks(&mut e).unwrap().entry_points;
    remove_perk_npc(&mut e, &[ACTOR, p, 0]);
    assert_eq!(items_of(&e, block, 25), [0]);
    assert_eq!(calls(), [(0, ability, ACTOR)]);
    assert_eq!(RELOADS.with(|r| r.take()), [ACTOR]);
    assert_eq!(ranks(&mut e).unwrap().ranks.len(), 1);
    remove_perk_npc(&mut e, &[ACTOR, p, 0]);
    assert!(calls().is_empty(), "not had");
    remove_perk_npc(&mut e, &[ACTOR, q, 0]);
    assert_eq!(ranks(&mut e), None);
    assert_eq!(FREED.with(|f| f.take()).last(), Some(&block));
}

/// `GetPerkRankNPCHook`: the actor's own rank; with none, a teammate's from the player's
/// teammates' ranks; anyone else 0.
#[test]
fn the_rank_is_the_actors_or_a_teammates() {
    let mut e = world();
    let (p, q) = (AREA + 0x3000, AREA + 0x3100);
    crate::cosave::npc_perks(&mut e)
        .emplace(ACTOR_ID)
        .ranks
        .insert(p, 2);
    let rank_item = AREA + 0x7000;
    e.mem.set_u32(rank_item, q);
    e.mem.set_u8(rank_item + 4, 3);
    list(&mut e, PLAYER + 0xad4, AREA + 0x7100, &[0, rank_item]);
    assert_eq!(get_perk_rank_npc(&mut e, &[ACTOR, p, 0]).eax, 2);
    assert_eq!(get_perk_rank_npc(&mut e, &[ACTOR, q, 1]).eax, 0);
    e.mem.set_u8(ACTOR + 0x18d, 1);
    assert_eq!(get_perk_rank_npc(&mut e, &[ACTOR, q, 0]).eax, 3);
    assert_eq!(get_perk_rank_npc(&mut e, &[ACTOR, AREA, 0]).eax, 0);
}

/// `GetPerkEntryPointListNPCHook`: the actor's list with an entry; else a teammate's from the
/// player's teammates' lists with an entry; else none.
#[test]
fn the_entry_point_list_is_the_actors_or_a_teammates() {
    let mut e = world();
    let point = entry(&mut e, 0, 2, 0, 4);
    let p = perk(&mut e, 0, [0, 0, 1, 1, 0], &[point]);
    assert_eq!(get_perk_entry_point_list_npc(&mut e, &[ACTOR, 4, 0]).eax, 0);
    set_perk_rank_npc(&mut e, &[ACTOR, p, 1, 0]);
    assert!(
        RELOADS.with(|r| r.take()).is_empty(),
        "no reload speed entry: no update"
    );
    let block = ranks(&mut e).unwrap().entry_points;
    assert_eq!(
        get_perk_entry_point_list_npc(&mut e, &[ACTOR, 4, 0]).eax,
        block + 8 * 3
    );
    assert_eq!(get_perk_entry_point_list_npc(&mut e, &[ACTOR, 0, 0]).eax, 0);
    assert_eq!(get_perk_entry_point_list_npc(&mut e, &[ACTOR, 3, 0]).eax, 0);
    e.mem.set_u8(ACTOR + 0x18d, 1);
    assert_eq!(get_perk_entry_point_list_npc(&mut e, &[ACTOR, 0, 0]).eax, 0);
    e.mem.set_u32(PLAYER + 0xadc, point);
    assert_eq!(
        get_perk_entry_point_list_npc(&mut e, &[ACTOR, 0, 0]).eax,
        PLAYER + 0xadc
    );
}

/// `InitNPCPerks`: an actor with ranks and no lists gets them with each perk's entries; one with
/// lists, or with no entry and no starting perks, nothing.
#[test]
fn init_makes_the_lists_once() {
    let mut e = world();
    let point = entry(&mut e, 0, 2, 1, 0);
    let p = perk(&mut e, 0, [0, 0, 2, 1, 0], &[point]);
    init_npc_perks(&mut e, ACTOR);
    assert_eq!(ranks(&mut e), None, "no entry, no starting perks");
    crate::cosave::npc_perks(&mut e)
        .emplace(ACTOR_ID)
        .ranks
        .insert(p, 2);
    init_npc_perks(&mut e, ACTOR);
    let block = ranks(&mut e).unwrap().entry_points;
    assert_eq!(items_of(&e, block, 0), [point]);
    init_npc_perks(&mut e, ACTOR);
    assert_eq!(ranks(&mut e).unwrap().entry_points, block, "made once");
}

/// `AddStartingPerks` (`uNPCPerks=2`): a `Character` gets a random trait at rank 1 and, at level
/// 12, four random ranks of the perks of minimum level up to 18 (one ticket a rank), shuffled;
/// a creature nothing; without the flag nothing.
#[test]
fn starting_perks() {
    let mut e = world();
    let point = entry(&mut e, 0, 2, 0, 0);
    let t0 = perk(&mut e, 0, [1, 0, 1, 1, 0], &[point]);
    let t1 = perk(&mut e, 1, [1, 0, 1, 1, 0], &[point]);
    let a = perk(&mut e, 2, [0, 2, 2, 1, 0], &[point]);
    let b = perk(&mut e, 3, [0, 18, 3, 1, 0], &[point]);
    let c = perk(&mut e, 4, [0, 19, 1, 1, 0], &[point]);
    let lists = e.ext_free::<NpcPerkLists>();
    let _ = lists.lists.set((vec![a, b, c], vec![t0, t1]));
    assert!(!add_starting_perks(&mut e, ACTOR, false), "flag off");
    e.ext_free::<NpcPerkLists>()
        .auto_add
        .store(true, Ordering::Relaxed);
    // The trait draw 1; the shuffle of [a, a, b, b, b] (5 > 4): draws 0, 2, 0, 1.
    DRAWS.with(|d| *d.borrow_mut() = vec![1, 0, 2, 0, 1]);
    set_changed(&mut e, 0);
    assert!(add_starting_perks(&mut e, ACTOR, false));
    // [a, a, b, b, b] -> i0 r0, i1 r2 swaps 1 and 3 -> [a, b, b, a, b], i2 r0, i3 r1 swaps 3, 4
    // -> [a, b, b, b, a]; the first four: a once, b three times.
    let info = ranks(&mut e).unwrap();
    assert_eq!(
        info.ranks.into_iter().collect::<Vec<_>>(),
        [(t1, 1), (a, 1), (b, 3)]
    );
    assert_eq!(
        changed_flags(&mut e) & changed::NPC_PERKS,
        changed::NPC_PERKS
    );
    let mut e2 = world();
    e2.mem.set_u8(ACTOR + 4, 0x3c);
    e2.ext_free::<NpcPerkLists>()
        .auto_add
        .store(true, Ordering::Relaxed);
    assert!(!add_starting_perks(&mut e2, ACTOR, false), "a creature");
}

/// `DestroyActorHook`: with the switch the actor's entry dropped (its lists freed, the data
/// marked changed), no answer; without it nothing.
#[test]
fn the_destructor_drops_the_entry() {
    let mut e = world();
    let p = perk(&mut e, 0, [0, 0, 1, 1, 0], &[]);
    set_perk_rank_npc(&mut e, &[ACTOR, p, 1, 0]);
    let block = ranks(&mut e).unwrap().entry_points;
    assert_eq!(destroy_actor(&mut e, &[ACTOR]), None);
    assert!(ranks(&mut e).is_some(), "switch off");
    e.jip_patches_or(engine_core::extend::jip_patches::NPC_PERKS);
    set_changed(&mut e, 0);
    assert_eq!(destroy_actor(&mut e, &[ACTOR]), None);
    assert_eq!(ranks(&mut e), None);
    assert_eq!(FREED.with(|f| f.take()), [block]);
    assert_eq!(changed_flags(&mut e), changed::NPC_PERKS);
}

/// After a load (the patch on, the NPC perks changed) each high actor's lists are made; with
/// the perks not changed, or the patch off, nothing.
#[test]
fn a_load_puts_the_high_actors_perks_on() {
    let mut e = world();
    list(&mut e, 0x011e_0f00, AREA + 0x7800, &[0, ACTOR]);
    crate::cosave::npc_perks(&mut e).emplace(ACTOR_ID);
    set_changed(&mut e, changed::NPC_PERKS);
    post_load_game(&mut e);
    assert_eq!(ranks(&mut e).unwrap().entry_points, 0, "patch off");
    set_npc_perks_patch(&mut e, true);
    set_changed(&mut e, 0);
    post_load_game(&mut e);
    assert_eq!(ranks(&mut e).unwrap().entry_points, 0, "not changed");
    set_changed(&mut e, changed::NPC_PERKS);
    post_load_game(&mut e);
    assert_ne!(ranks(&mut e).unwrap().entry_points, 0);
}

/// O18 on: the four routines in both vtables, the patch marked, the flag kept; off, or a second
/// time, nothing written.
#[test]
fn switching_on_writes_the_slots() {
    let mut e = world();
    e.map(0x0108_6000, 0x2000);
    switch_on(&mut e, true);
    assert_eq!(e.mem.u32(CHARACTER_SLOTS[0]), 0, "switch off");
    assert!(!crate::cosave::npc_perks_patch(&mut e));
    e.jip_patches_or(engine_core::extend::jip_patches::NPC_PERKS);
    switch_on(&mut e, true);
    for slots in [CHARACTER_SLOTS, CREATURE_SLOTS] {
        for (slot, at) in slots.into_iter().zip(ROUTINES) {
            assert_eq!(e.mem.u32(slot), at);
        }
    }
    assert!(crate::cosave::npc_perks_patch(&mut e));
    assert!(e
        .ext_free::<NpcPerkLists>()
        .auto_add
        .load(Ordering::Relaxed));
    // Through the slot: the actor's rank routine.
    crate::cosave::npc_perks(&mut e)
        .emplace(ACTOR_ID)
        .ranks
        .insert(AREA, 4);
    assert_eq!(e.call(ROUTINES[2], &args![ACTOR, AREA, 0u32]).u32(), 4);
    switch_on(&mut e, false);
    assert!(
        e.ext_free::<NpcPerkLists>()
            .auto_add
            .load(Ordering::Relaxed),
        "once"
    );
}
