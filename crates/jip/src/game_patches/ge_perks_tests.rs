//! O18 (`uNPCPerks`): the NPC and player perk routines, the teammate wrap and the seams' handlers,
//! case by case from JIP's source.

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
    e.map(0x0108_6000, 0x5000);
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

// ---- part 2: the player's routines, the teammate wrap, the seams, the lock-free reads ----

/// The ability entries' vtable page (the stand-ins of `GetType`, `AddEntry` and `RemoveEntry`)
/// and the stats menu's refresh (counted in [`REFRESHES`]).
fn ability_world() -> Engine {
    let mut e = world();
    REFRESHES.with(|r| r.set(0));
    e.map(0x0104_6000, 0x1000);
    e.mem.set_u32(ABILITY_ENTRY_VTABLE + 0x10, GET_TYPE_AT);
    e.mem.set_u32(ABILITY_ENTRY_VTABLE + 0x30, ADD_ENTRY_AT);
    e.mem.set_u32(ABILITY_ENTRY_VTABLE + 0x34, REMOVE_ENTRY_AT);
    e.register(REFRESH_STATS_MENU, |_, _| {
        REFRESHES.with(|r| r.set(r.get() + 1));
        ret(0)
    });
    e
}

/// An ability entry at `n` (the entries' area, as [`entry`]) of `rank`.
fn ability(e: &mut Engine, n: u32, rank: u8) -> u32 {
    let at = entry(e, n, 1, rank, 0);
    e.mem.set_u32(at, ABILITY_ENTRY_VTABLE);
    at
}

/// A second actor (a teammate) and its id.
const MATE: u32 = AREA + 0x1800;
const MATE_ID: u32 = 0x0700_0002;

thread_local! {
    static REFRESHES: Cell<u32> = const { Cell::new(0) };
    static BASE_TEAMMATE: RefCell<Vec<Vec<u32>>> = const { RefCell::new(Vec::new()) };
}

/// The stats menu refreshes since the last look.
fn refreshes() -> u32 {
    REFRESHES.with(|r| r.replace(0))
}

/// `SetPerkRankPlayerHook`, the player's own list: a new `PerkRank` put first; every entry of the
/// rank added and of another rank removed (an entry point entry too), the stats menu refreshed,
/// the reload speed updated; the same rank again nothing; another rank changes the one kept.
#[test]
fn the_players_set_rank_puts_its_entries_on() {
    let mut e = ability_world();
    let point = entry(&mut e, 0, 2, 0, 4);
    let other = ability(&mut e, 1, 1);
    let p = perk(&mut e, 0, [0, 0, 2, 1, 0], &[point, other]);
    let q = AREA + 0x3800;
    let old = AREA + 0x7000;
    e.mem.set_u32(old, q);
    e.mem.set_u8(old + 4, 1);
    list(&mut e, PLAYER + 0x87c, AREA + 0x7100, &[old]);
    set_perk_rank_player(&mut e, &[PLAYER, p, 1, 0]);
    let items = list_items(&e, Ptr::new(PLAYER + 0x87c));
    assert_eq!(items.len(), 2);
    let made = items[0];
    assert_eq!((e.mem.u32(made), e.mem.u8(made + 4)), (p, 1), "put first");
    assert_eq!(items[1], old);
    assert_eq!(calls(), [(1, point, PLAYER), (0, other, PLAYER)]);
    assert_eq!((refreshes(), RELOADS.with(|r| r.take())), (1, vec![PLAYER]));
    set_perk_rank_player(&mut e, &[PLAYER, p, 1, 0]);
    assert!(calls().is_empty() && refreshes() == 0, "same rank: nothing");
    set_perk_rank_player(&mut e, &[PLAYER, p, 2, 0]);
    assert_eq!(e.mem.u8(made + 4), 2);
    assert_eq!(
        list_items(&e, Ptr::new(PLAYER + 0x87c)).len(),
        2,
        "no new one"
    );
    assert_eq!(calls(), [(0, point, PLAYER), (1, other, PLAYER)]);
    // A null perk: nothing (the exe's own test).
    set_perk_rank_player(&mut e, &[PLAYER, 0, 1, 0]);
    assert_eq!(list_items(&e, Ptr::new(PLAYER + 0x87c)).len(), 2);
}

/// `SetPerkRankPlayerHook` for the teammates: the teammates' list keeps the rank; each teammate
/// gets the perk's ability entries of the rank (others removed; entry point entries are left to
/// the teammate lists), unless it has the perk of its own; no menu refresh, no reload update.
#[test]
fn the_teammates_set_rank_puts_abilities_on_each_teammate() {
    let mut e = ability_world();
    e.mem.set_u32(MATE + 0xc, MATE_ID);
    let point = entry(&mut e, 0, 2, 0, 4);
    let at_rank = ability(&mut e, 1, 0);
    let other = ability(&mut e, 2, 1);
    let p = perk(&mut e, 0, [0, 0, 2, 1, 0], &[point, at_rank, other]);
    list(&mut e, PLAYER + 0x5fc, AREA + 0x7100, &[ACTOR, 0, MATE]);
    crate::cosave::npc_perks(&mut e)
        .emplace(MATE_ID)
        .ranks
        .insert(p, 1);
    set_perk_rank_player(&mut e, &[PLAYER, p, 1, 1]);
    let items = list_items(&e, Ptr::new(PLAYER + 0xad4));
    assert_eq!((e.mem.u32(items[0]), e.mem.u8(items[0] + 4)), (p, 1));
    assert_eq!(
        calls(),
        [(1, at_rank, ACTOR), (0, other, ACTOR)],
        "not MATE"
    );
    assert_eq!((refreshes(), RELOADS.with(|r| r.take())), (0, vec![]));
    assert!(
        list_items(&e, Ptr::new(PLAYER + 0x87c)) == [0],
        "not the player's own list"
    );
}

/// `RemovePerkPlayerHook`: the `PerkRank` unlinked and freed; for the player every entry removed,
/// the menu refreshed and the reload speed updated; for the teammates each teammate's abilities
/// removed unless it has the perk; a perk not had: nothing.
#[test]
fn the_players_remove_takes_the_entries_off() {
    let mut e = ability_world();
    let point = entry(&mut e, 0, 2, 0, 4);
    let skill = ability(&mut e, 1, 0);
    let p = perk(&mut e, 0, [0, 0, 1, 1, 0], &[point, skill]);
    let q = AREA + 0x3800;
    let (rank_q, rank_p) = (AREA + 0x7000, AREA + 0x7010);
    e.mem.set_u32(rank_q, q);
    e.mem.set_u32(rank_p, p);
    list(&mut e, PLAYER + 0x87c, AREA + 0x7100, &[rank_q, rank_p]);
    remove_perk_player(&mut e, &[PLAYER, p, 0]);
    assert_eq!(list_items(&e, Ptr::new(PLAYER + 0x87c)), [rank_q]);
    assert!(FREED.with(|f| f.take()).contains(&rank_p), "freed");
    assert_eq!(calls(), [(0, point, PLAYER), (0, skill, PLAYER)]);
    assert_eq!((refreshes(), RELOADS.with(|r| r.take())), (1, vec![PLAYER]));
    remove_perk_player(&mut e, &[PLAYER, p, 0]);
    assert!(calls().is_empty() && refreshes() == 0, "not had: nothing");
    // The teammates' list: the abilities off each teammate without the perk of its own.
    e.mem.set_u32(MATE + 0xc, MATE_ID);
    list(&mut e, PLAYER + 0xad4, AREA + 0x7200, &[rank_p]);
    list(&mut e, PLAYER + 0x5fc, AREA + 0x7300, &[MATE, ACTOR]);
    crate::cosave::npc_perks(&mut e)
        .emplace(MATE_ID)
        .ranks
        .insert(p, 1);
    remove_perk_player(&mut e, &[PLAYER, p, 1]);
    assert!(list_items(&e, Ptr::new(PLAYER + 0xad4)) == [0]);
    assert_eq!(calls(), [(0, skill, ACTOR)]);
    assert_eq!((refreshes(), RELOADS.with(|r| r.take())), (0, vec![]));
}

fn base_set_player_teammate(_: &mut Engine, a: &[u32]) -> Ret {
    BASE_TEAMMATE.with(|b| b.borrow_mut().push(a.to_vec()));
    ret(0)
}

/// `SetPlayerTeammateHook` through the wrap: before the switch the exe's function; then the
/// player, or an unchanged flag, nothing; joining counts the teammate, puts it first and puts
/// the teammates' perks' abilities on at each perk's rank; leaving uncounts, takes it out and
/// takes those abilities off only when it has perks of its own (JIP's test).
#[test]
fn the_teammate_wrap() {
    let mut e = ability_world();
    e.register(0x008b_ca90, base_set_player_teammate);
    BASE_TEAMMATE.with(|b| b.borrow_mut().clear());
    e.wrap(SET_PLAYER_TEAMMATE, set_player_teammate);
    e.call(0x008b_ca90, &[ACTOR, 1]);
    assert_eq!(
        BASE_TEAMMATE.with(|b| b.take()),
        [vec![ACTOR, 1]],
        "not on yet"
    );
    assert_eq!(e.mem.u8(ACTOR + 0x18d), 0);
    set_npc_perks_patch(&mut e, true);
    let skill = ability(&mut e, 0, 1);
    let low = ability(&mut e, 1, 0);
    let p = perk(&mut e, 0, [0, 0, 2, 1, 0], &[skill, low]);
    let rank = AREA + 0x7000;
    e.mem.set_u32(rank, p);
    e.mem.set_u8(rank + 4, 2);
    list(&mut e, PLAYER + 0xad4, AREA + 0x7100, &[rank]);
    // (`tList::Empty` looks at the first node only: a list starting empty counts as empty.)
    list(&mut e, PLAYER + 0x5fc, AREA + 0x7200, &[MATE]);
    e.mem.set_u32(PLAYER + 0xd68, 1);
    e.call(0x008b_ca90, &[PLAYER, 1]);
    assert_eq!(e.mem.u32(PLAYER + 0xd68), 1, "the player: nothing");
    e.call(0x008b_ca90, &[ACTOR, 1]);
    assert!(BASE_TEAMMATE.with(|b| b.take()).is_empty(), "JIP's");
    assert_eq!(e.mem.u8(ACTOR + 0x18d), 1);
    assert_eq!(e.mem.u32(PLAYER + 0xd68), 2);
    assert_eq!(list_items(&e, Ptr::new(PLAYER + 0x5fc)), [ACTOR, MATE]);
    assert_eq!(calls(), [(1, skill, ACTOR), (0, low, ACTOR)]);
    e.call(0x008b_ca90, &[ACTOR, 1]);
    assert_eq!(e.mem.u32(PLAYER + 0xd68), 2, "already one: nothing");
    // Leaving with no perks of its own: the abilities stay (JIP's test).
    e.call(0x008b_ca90, &[ACTOR, 0]);
    assert_eq!(e.mem.u8(ACTOR + 0x18d), 0);
    assert_eq!(e.mem.u32(PLAYER + 0xd68), 1);
    assert_eq!(list_items(&e, Ptr::new(PLAYER + 0x5fc)), [MATE]);
    assert!(calls().is_empty());
    // With perks of its own (not this one): taken off.
    e.call(0x008b_ca90, &[ACTOR, 1]);
    calls();
    crate::cosave::npc_perks(&mut e)
        .emplace(ACTOR_ID)
        .ranks
        .insert(AREA + 0x3800, 1);
    e.call(0x008b_ca90, &[ACTOR, 0]);
    assert_eq!(calls(), [(0, skill, ACTOR), (0, low, ACTOR)]);
}

/// The three seams' handlers: nothing until the patch is marked installed; then the teammate
/// kept, the damage entry point run, and the attacker's current weapon (0 without a process or a
/// weapon).
#[test]
fn the_seams_answer_once_switched_on() {
    let mut e = world();
    let weapon_at = 0x0040_1f10;
    let process = AREA + 0x7000;
    let process_vt = AREA + 0x7100;
    let inventory_entry = AREA + 0x7200;
    e.mem.set_u32(process, process_vt);
    e.mem.set_u32(process_vt + 0x148, weapon_at);
    e.register(weapon_at, |e, a| ret(e.mem.u32(a[0] + 4)));
    e.mem.set_u32(inventory_entry + 8, 0x6400);
    assert_eq!(teammate_own_entries(&mut e, &[ACTOR]), None);
    assert_eq!(weapon_damage_perks(&mut e, &[ACTOR]), None);
    assert_eq!(kill_effect_weapon(&mut e, &[ACTOR]), None);
    set_npc_perks_patch(&mut e, true);
    assert_eq!(teammate_own_entries(&mut e, &[ACTOR]), Some(1));
    assert_eq!(weapon_damage_perks(&mut e, &[ACTOR]), Some(1));
    assert_eq!(kill_effect_weapon(&mut e, &[ACTOR]), Some(0), "no process");
    e.mem.set_u32(ACTOR + 0x68, process);
    assert_eq!(kill_effect_weapon(&mut e, &[ACTOR]), Some(0), "no weapon");
    e.mem.set_u32(process + 4, inventory_entry);
    assert_eq!(kill_effect_weapon(&mut e, &[ACTOR]), Some(0x6400));
}

/// The handlers and the wrap go in with the fixes.
#[test]
fn installed_with_the_fixes() {
    let mut e = world();
    install_fixes(&mut e);
    for site in [
        DESTROY_ACTOR,
        TEAMMATE_OWN_ENTRIES,
        WEAPON_DAMAGE_PERKS,
        KILL_EFFECT_WEAPON,
    ] {
        assert_eq!(e.site_owners(site.addr()), [Owner::Jip]);
    }
    assert_eq!(e.wraps_at(SET_PLAYER_TEAMMATE.addr()), [Owner::Jip]);
}

/// O18 on: the player's two routines in its slots too.
#[test]
fn switching_on_writes_the_players_slots() {
    let mut e = ability_world();
    e.map(0x0108_6000, 0x5000);
    e.jip_patches_or(engine_core::extend::jip_patches::NPC_PERKS);
    e.mem.set_u32(PLAYER_SLOTS[0], 0x0096_3790);
    e.mem.set_u32(PLAYER_SLOTS[1], 0x0096_3900);
    switch_on(&mut e, false);
    assert_eq!(e.mem.u32(PLAYER_SLOTS[0]), PLAYER_ROUTINES[0]);
    assert_eq!(e.mem.u32(PLAYER_SLOTS[1]), PLAYER_ROUTINES[1]);
    // Through the slot: the player's set rank.
    let p = perk(&mut e, 0, [0, 0, 1, 1, 0], &[]);
    e.call(PLAYER_ROUTINES[0], &args![PLAYER, p, 1u32, 0u32]);
    let items = list_items(&e, Ptr::new(PLAYER + 0x87c));
    assert_eq!(e.mem.u32(items[0]), p);
    assert_eq!(refreshes(), 1);
}

/// The map's counters: reads that took the lock, copies made.
fn read_counts(e: &mut Engine) -> (u32, u32) {
    let m = e.ext_free::<crate::cosave::NpcPerksMap>();
    (
        m.locked_reads.load(Ordering::Relaxed),
        m.copies_made.load(Ordering::Relaxed),
    )
}

/// The rank and list reads: after a change the first read is answered under the lock (no copy),
/// the next one, the version unchanged, makes the copy; then no lock while nothing changes; a
/// look through the map that changes nothing makes no new version; each change is seen by the
/// next read; another engine's map is never taken for this one's.
#[test]
fn the_reads_take_no_lock_while_nothing_changes() {
    let mut e = world();
    let p = AREA + 0x3000;
    crate::cosave::npc_perks(&mut e)
        .emplace(ACTOR_ID)
        .ranks
        .insert(p, 2);
    let (locked, made) = read_counts(&mut e);
    assert_eq!(get_perk_rank_npc(&mut e, &[ACTOR, p, 0]).eax, 2);
    assert_eq!(
        read_counts(&mut e),
        (locked + 1, made),
        "the first read: under the lock, no copy"
    );
    assert_eq!(get_perk_rank_npc(&mut e, &[ACTOR, p, 0]).eax, 2);
    assert_eq!(
        read_counts(&mut e),
        (locked + 2, made + 1),
        "the version stayed: copied"
    );
    for _ in 0..3 {
        assert_eq!(get_perk_rank_npc(&mut e, &[ACTOR, p, 0]).eax, 2);
        assert_eq!(get_perk_entry_point_list_npc(&mut e, &[ACTOR, 0, 0]).eax, 0);
    }
    assert!(crate::cosave::npc_perks(&mut e).get(&ACTOR_ID).is_some());
    assert_eq!(
        read_counts(&mut e),
        (locked + 2, made + 1),
        "no change: no lock"
    );
    crate::cosave::npc_perks(&mut e)
        .emplace(ACTOR_ID)
        .ranks
        .insert(p, 3);
    assert_eq!(get_perk_rank_npc(&mut e, &[ACTOR, p, 0]).eax, 3, "seen");
    assert_eq!(read_counts(&mut e), (locked + 3, made + 1));
    crate::cosave::npc_perks(&mut e)
        .emplace(ACTOR_ID)
        .entry_points = AREA + 0x7000;
    e.mem.set_u32(AREA + 0x7000, 0x55);
    assert_eq!(
        get_perk_entry_point_list_npc(&mut e, &[ACTOR, 0, 0]).eax,
        AREA + 0x7000
    );
    let mut other = world();
    crate::cosave::npc_perks(&mut other)
        .emplace(ACTOR_ID)
        .ranks
        .insert(p, 7);
    assert_eq!(get_perk_rank_npc(&mut other, &[ACTOR, p, 0]).eax, 7);
    assert_eq!(get_perk_rank_npc(&mut e, &[ACTOR, p, 0]).eax, 3);
}

/// A set at an equal or lower rank and a remove of a perk the actor lacks change nothing: the
/// reads' copy stays current (no lock taken afterwards).
#[test]
fn writes_that_change_nothing_keep_the_reads_copy() {
    let mut e = world();
    let point = entry(&mut e, 0, 2, 0, 0);
    let p = perk(&mut e, 0, [0, 0, 3, 1, 0], &[point]);
    let q = perk(&mut e, 1, [0, 0, 1, 1, 0], &[]);
    set_perk_rank_npc(&mut e, &[ACTOR, p, 2, 0]);
    for _ in 0..2 {
        assert_eq!(get_perk_rank_npc(&mut e, &[ACTOR, p, 0]).eax, 2);
    }
    let counts = read_counts(&mut e);
    set_perk_rank_npc(&mut e, &[ACTOR, p, 2, 0]);
    set_perk_rank_npc(&mut e, &[ACTOR, p, 1, 0]);
    remove_perk_npc(&mut e, &[ACTOR, q, 0]);
    assert_eq!(get_perk_rank_npc(&mut e, &[ACTOR, p, 0]).eax, 2);
    assert_eq!(read_counts(&mut e), counts, "the copy stayed current");
    // A real change is seen.
    set_perk_rank_npc(&mut e, &[ACTOR, p, 3, 0]);
    assert_eq!(get_perk_rank_npc(&mut e, &[ACTOR, p, 0]).eax, 3);
}

/// A burst of changes with reads between them: each read answered under the lock, right, and no
/// copy made until the version holds still for two reads.
#[test]
fn a_burst_of_changes_makes_no_copies() {
    let mut e = world();
    let p = AREA + 0x3000;
    let (_, made) = read_counts(&mut e);
    for rank in 1..=5u8 {
        crate::cosave::npc_perks(&mut e)
            .emplace(ACTOR_ID)
            .ranks
            .insert(p, rank);
        assert_eq!(
            get_perk_rank_npc(&mut e, &[ACTOR, p, 0]).eax,
            u32::from(rank)
        );
    }
    assert_eq!(read_counts(&mut e).1, made, "no copies during the burst");
    assert_eq!(get_perk_rank_npc(&mut e, &[ACTOR, p, 0]).eax, 5);
    assert_eq!(read_counts(&mut e).1, made + 1, "copied once it held still");
}

/// The switch goes on at xNVSE's `DEFERRED_INIT` (through the co-save listener, as JIP's
/// `s_deferrSetOptional`), with the settings' `uNPCPerks=2` flag; without the switch's bit the
/// message turns nothing on.
#[test]
fn the_deferred_init_message_switches_it_on() {
    let mut e = world();
    e.map(0x0108_6000, 0x5000);
    // The same message makes JIP's position marker: its game calls as stand-ins.
    e.register(crate::commands::pos_marker::REFR_CONSTRUCTOR, |_, w| {
        ret(w[0])
    });
    e.register(crate::commands::pos_marker::SET_TEMPORARY, |_, _| ret(0));
    e.ext::<crate::Jip>()
        .game_patches
        .settings
        .npc_perks_auto_add = true;
    let deferred = |e: &mut Engine| {
        crate::cosave::listen(
            e,
            &nvse::plugins::Message {
                kind: nvse::plugins::msg::DEFERRED_INIT,
                data: 0,
                len: 0,
            },
        );
    };
    deferred(&mut e);
    assert!(!crate::cosave::npc_perks_patch(&mut e), "switch off");
    assert_eq!(e.mem.u32(PLAYER_SLOTS[0]), 0);
    e.jip_patches_or(engine_core::extend::jip_patches::NPC_PERKS);
    deferred(&mut e);
    assert!(crate::cosave::npc_perks_patch(&mut e));
    assert_eq!(e.mem.u32(PLAYER_SLOTS[0]), PLAYER_ROUTINES[0]);
    assert_eq!(e.mem.u32(CHARACTER_SLOTS[0]), ROUTINES[0]);
    assert!(e
        .ext_free::<NpcPerkLists>()
        .auto_add
        .load(Ordering::Relaxed));
}

/// `DoLoadGameHousekeeping`'s teammate step: with teammates, each teammate perk of the player put
/// on again at its rank (the game's `005eb6a0` with the player, 0, the rank, 1); an empty item and
/// a rank with no perk passed over; with no teammates, nothing.
#[test]
fn a_load_puts_the_teammate_perks_on_again() {
    thread_local! {
        static APPLIED: RefCell<Vec<Vec<u32>>> = const { RefCell::new(Vec::new()) };
    }
    let mut e = world();
    e.register(PERK_APPLY_RANKS, |_, a| {
        APPLIED.with(|c| c.borrow_mut().push(a[..5].to_vec()));
        ret(0)
    });
    let (p1, p2) = (AREA + 0x3000, AREA + 0x3100);
    let (r1, r2, r3) = (AREA + 0x7300, AREA + 0x7308, AREA + 0x7310);
    e.mem.set_u32(r1, p1);
    e.mem.set_u8(r1 + 4, 2);
    e.mem.set_u32(r2, 0);
    e.mem.set_u32(r3, p2);
    e.mem.set_u8(r3 + 4, 1);
    list(&mut e, PLAYER + 0xad4, AREA + 0x7400, &[r1, 0, r2, r3]);
    e.mem.set_u32(PLAYER + 0xd68, 0);
    teammate_perks_after_load(&mut e);
    assert!(APPLIED.with(|c| c.borrow().is_empty()), "no teammates");
    e.mem.set_u32(PLAYER + 0xd68, 1);
    teammate_perks_after_load(&mut e);
    assert_eq!(
        APPLIED.with(|c| c.borrow().clone()),
        [vec![p1, PLAYER, 0, 2, 1], vec![p2, PLAYER, 0, 1, 1]]
    );
}
