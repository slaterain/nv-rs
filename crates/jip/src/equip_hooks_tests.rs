//! Tests of `CCCSetEquipped` and JIP's `GetCannotWear` wrap: nothing put in while JIP answers "not
//! installed"; installed, the wrap's answer with and without JIP's flag, and the command through
//! its record on a companion and from the container menu, with the game's functions it calls as
//! recording stand-ins. Layouts at literal offsets, not the code's: a form's vtable `+0` and type
//! `+4`; a reference's base `+0x20`, extra list `+0x44`, process `+0x68`; `Character`'s
//! `BipedAnim` `+0x1B4` (slots at `+0x2C`, 0x10 bytes each), DR `+0x1B8`, DT `+0x1BC`; a process's
//! level `+0x28` and weapon entry `+0x114`; an actor base's container at `+0x64` (list first node
//! inline at `+4`, each entry a count and a form) when its vtable's `+0xF8` holds `0x008D0360`, its
//! combat style slot `+0x188`; an `ExtraContainerChanges`' data `+0xC`, whose list is at `+0`; an
//! inventory entry's lists `+0`, count `+4`, form `+8`; an extra list's worn byte `+0xA` (`0x40`),
//! leveled-item byte `+0xD` (`0x80`) and cannot-wear byte `+0xF` (`0x40`); a weapon's type `+0xF4`,
//! flags `+0x100` (`0x80`) and `+0x12C` (bit 0); an armour's part mask `+0x74` and flags `+0x78`
//! (`0x40`); a combat style's restrictions `+0x40`; the container menu `*0x011D93F8`, its selection
//! `*0x011D93FC`, the menu's reference `+0x74` and selected tile `+0xD8`; the interface manager
//! `*0x011D8A80`, its tiles `+0xCC` and `+0xBC`; a tile's values `+0x14`, count `+0x18`, a value's
//! id `+0` and number `+8`; `Character`'s vtable `0x01086A6C`, `TESObjectARMO`'s `0x0102A62C`; the
//! player `*0x011DEA3C`.

use super::*;
use crate::test_support::{engine, line, RESULT};
use std::cell::RefCell;
use std::collections::HashMap;

const AREA: u32 = 0x0500_0000;
const CHAR: u32 = AREA + 0x1000;
const OTHER_REF: u32 = AREA + 0x1400;
const BASE: u32 = AREA + 0x2000;
const BASE_VT: u32 = AREA + 0x100;
const OBJECTS: u32 = AREA + 0x3000;
const NODES: u32 = AREA + 0x4000;
const ENTRIES: u32 = AREA + 0x5000;
const LISTS: u32 = AREA + 0x6000;
const LIST_NODES: u32 = AREA + 0x6800;
const XCHANGES: u32 = AREA + 0x7000;
const CDATA: u32 = AREA + 0x7100;
const XSTYLE: u32 = AREA + 0x7200;
const ITEMS: u32 = AREA + 0x8000;
const BIPED: u32 = AREA + 0xc000;
const PROCESS: u32 = AREA + 0xd000;
const MENU: u32 = AREA + 0xe000;
const TILE: u32 = AREA + 0xe200;
const TILE_ALT: u32 = AREA + 0xe280;
const VALUES: u32 = AREA + 0xe300;
const VALUE: u32 = AREA + 0xe400;
const MANAGER: u32 = AREA + 0xf000;
const STYLE: u32 = AREA + 0xf800;
const BASE_STYLE: u32 = AREA + 0xf900;
const PLAYER_REF: u32 = AREA + 0xfa00;
const CHARACTER_VT: u32 = 0x0108_6a6c;
const ARMOR_VT: u32 = 0x0102_a62c;
const BASE_GET_STYLE: u32 = 0x00de_0188;

thread_local! {
    /// The game functions' calls the stand-ins saw: (function, words, JIP's flag then).
    static CALLS: RefCell<Vec<(u32, Vec<u32>, bool)>> = const { RefCell::new(Vec::new()) };
    /// `GetByType`'s answers: (list, type) -> extra.
    static EXTRAS: RefCell<HashMap<(u32, u32), u32>> = RefCell::new(HashMap::new());
    /// What `GetCanNotWear` answered when the equip stand-ins asked it of their extra list.
    static ASKED: RefCell<Vec<u32>> = const { RefCell::new(Vec::new()) };
}

fn calls() -> Vec<(u32, Vec<u32>, bool)> {
    CALLS.with(|c| std::mem::take(&mut *c.borrow_mut()))
}

fn called(f: u32) -> Vec<Vec<u32>> {
    calls()
        .into_iter()
        .filter(|(g, _, _)| *g == f)
        .map(|(_, w, _)| w)
        .collect()
}

fn asked() -> Vec<u32> {
    ASKED.with(|c| std::mem::take(&mut *c.borrow_mut()))
}

fn set_extra(list: u32, kind: u32, extra: u32) {
    EXTRAS.with(|x| x.borrow_mut().insert((list, kind), extra));
}

/// The n-th item's address (0x200 bytes each).
fn item(n: u32) -> u32 {
    ITEMS + n * 0x200
}

fn weapon(e: &mut Engine, n: u32, kind: u8, playable: bool) -> u32 {
    let w = item(n);
    e.mem.set_u8(w + 4, 0x28);
    e.mem.set_u8(w + 0xf4, kind);
    e.mem.set_u8(w + 0x100, if playable { 0 } else { 0x80 });
    w
}

fn armor(e: &mut Engine, n: u32, mask: u32, playable: bool) -> u32 {
    let a = item(n);
    e.mem.set_u32(a, ARMOR_VT);
    e.mem.set_u8(a + 4, 0x18);
    e.mem.set_u32(a + 0x74, mask);
    e.mem.set_u32(a + 0x78, if playable { 0 } else { 0x40 });
    a
}

/// The base container's entries (count, form), in list order.
fn set_container(e: &mut Engine, objects: &[(i32, u32)]) {
    let first = BASE + 0x64 + 4;
    e.mem.set_u32(first, 0);
    e.mem.set_u32(first + 4, 0);
    let mut node = first;
    for (i, &(count, form)) in objects.iter().enumerate() {
        let object = OBJECTS + i as u32 * 0x10;
        e.mem.set_u32(object, count as u32);
        e.mem.set_u32(object + 4, form);
        if i > 0 {
            let next = OBJECTS + 0x800 + i as u32 * 8;
            e.mem.set_u32(node + 4, next);
            node = next;
        }
        e.mem.set_u32(node, object);
        e.mem.set_u32(node + 4, 0);
    }
}

/// The changes entries (form, count, extra list or 0), in list order; answers their addresses.
fn set_changes(e: &mut Engine, entries: &[(u32, i32, u32)]) -> Vec<u32> {
    let mut made = vec![];
    e.mem.set_u32(NODES, 0);
    e.mem.set_u32(NODES + 4, 0);
    for (i, &(form, count, list)) in entries.iter().enumerate() {
        let entry = ENTRIES + i as u32 * 0x10;
        let lists = if list == 0 {
            0
        } else {
            let node = LIST_NODES + i as u32 * 8;
            e.mem.set_u32(node, list);
            e.mem.set_u32(node + 4, 0);
            node
        };
        e.mem.set_u32(entry, lists);
        e.mem.set_u32(entry + 4, count as u32);
        e.mem.set_u32(entry + 8, form);
        let node = NODES + i as u32 * 8;
        e.mem.set_u32(node, entry);
        e.mem.set_u32(node + 4, 0);
        if i > 0 {
            e.mem.set_u32(node - 8 + 4, node);
        }
        made.push(entry);
    }
    made
}

fn world_as(installed: bool) -> Engine {
    calls();
    asked();
    EXTRAS.with(|x| x.borrow_mut().clear());
    let mut e = engine();
    e.map(AREA, 0x1_0000);
    for page in [0x0108_6000u32, 0x0102_a000, 0x011d_8000, 0x011d_9000] {
        if !e.mem.is_mapped(page) {
            e.map(page, 0x1000);
        }
    }
    e.mem.set_u32(CHAR, CHARACTER_VT);
    e.mem.set_u32(OTHER_REF, AREA + 0x900);
    e.mem.set_u32(CHAR + 0x20, BASE);
    e.mem.set_u32(BASE, BASE_VT);
    e.mem.set_u32(BASE_VT + 0xf8, 0x008d_0360);
    e.mem.set_u32(BASE_VT + 0x188, BASE_GET_STYLE);
    e.register(BASE_GET_STYLE, |e, a| Ret {
        eax: e.mem.u32(a[0] + 0x1d4),
        ..Ret::default()
    });
    e.register(0x0041_0220, |_, w| Ret {
        eax: EXTRAS.with(|x| x.borrow().get(&(w[0], w[1])).copied().unwrap_or(0)),
        ..Ret::default()
    });
    set_extra(CHAR + 0x44, 0x15, XCHANGES);
    e.mem.set_u32(XCHANGES + 0xc, CDATA);
    e.mem.set_u32(CDATA, NODES);
    for f in [
        0x0088_c650u32,
        0x008d_2110,
        0x008d_22b0,
        0x008a_ded0,
        0x0075_c280,
    ] {
        e.register_double(f, move |e, w| {
            let forced = e.ext::<Jip>().ai_hooks.force_equip_call;
            CALLS.with(|c| c.borrow_mut().push((f, w.to_vec(), forced)));
            Ret::default()
        });
    }
    // EquipObject and UnEquipObject ask GetCanNotWear of their extra list (as 0088c947 and
    // 0088d80d do), through the address.
    for f in [0x0088_c830u32, 0x0088_d7d0] {
        e.register_double(f, move |e, w| {
            let forced = e.ext::<Jip>().ai_hooks.force_equip_call;
            CALLS.with(|c| c.borrow_mut().push((f, w.to_vec(), forced)));
            if w[3] != 0 {
                let answer = e.call(0x0041_8b10, &args![w[3]]).eax & 0xff;
                ASKED.with(|c| c.borrow_mut().push(answer));
            }
            Ret::default()
        });
    }
    e.mem.set_u32(0x011d_ea3c, PLAYER_REF);
    crate::install_as(&mut e, installed);
    e
}

fn world() -> Engine {
    world_as(true)
}

/// Runs the command through its record on `this`; the result.
fn run(e: &mut Engine, this: u32) -> f64 {
    let mut words = line(e, CCC_SET_EQUIPPED, &[]);
    words[2] = this;
    let at = crate::test_support::execute_of(e, CCC_SET_EQUIPPED);
    assert_eq!(e.call(at, &words).eax & 0xff, 1);
    e.mem.f64(RESULT)
}

fn cannot_wear(e: &mut Engine, list: u32) -> u32 {
    e.call(0x0041_8b10, &args![list]).eax & 0xff
}

/// While JIP answers "not installed": no wrap at `0x418B10` (the game's function answers alone)
/// and the command has no handler.
#[test]
fn not_installed_nothing_is_put_in() {
    let mut e = world_as(false);
    assert!(e.wraps_at(0x0041_8b10).is_empty());
    assert_eq!(crate::test_support::execute_of(&e, CCC_SET_EQUIPPED), 0);
    e.mem.set_u8(LISTS + 0xf, 0x40);
    e.ext::<Jip>().ai_hooks.force_equip_call = true;
    assert_eq!(cannot_wear(&mut e, LISTS), 1, "the flag means nothing");
}

/// Installed: JIP's one wrap at the address, listed in WRAPS; with the flag clear the function
/// there answers (both ways), with it set the answer is false.
#[test]
fn get_cannot_wear_is_false_only_while_the_command_equips() {
    let mut e = world();
    assert_eq!(e.wraps_at(0x0041_8b10), [Owner::Jip]);
    assert!(e.unlisted_wraps().is_empty());
    e.mem.set_u8(LISTS + 0xf, 0x40);
    assert_eq!(cannot_wear(&mut e, LISTS), 1);
    assert_eq!(cannot_wear(&mut e, LISTS + 0x40), 0);
    e.ext::<Jip>().ai_hooks.force_equip_call = true;
    assert_eq!(cannot_wear(&mut e, LISTS), 0);
    assert_eq!(cannot_wear(&mut e, LISTS + 0x40), 0);
    e.ext::<Jip>().ai_hooks.force_equip_call = false;
    assert_eq!(cannot_wear(&mut e, LISTS), 1, "back to the game's answer");
}

/// A reference that is not a `Character` does nothing (no game function is called), even with
/// gear the companion form would equip.
#[test]
fn a_reference_that_is_not_a_character_does_nothing() {
    let mut e = world();
    let w = weapon(&mut e, 1, 0, false);
    set_container(&mut e, &[(1, w)]);
    set_changes(&mut e, &[]);
    run(&mut e, OTHER_REF);
    assert!(calls().is_empty());
}

/// The weapon pass: with no weapon in hand, the first weapon by address that is not playable and
/// that the combat style allows (here melee only: types below 3) is equipped once, with its
/// entry's first extra list, count 1, words 1, 0, 0; the actor's own combat style comes before
/// its base's.
#[test]
fn a_companion_with_no_weapon_equips_its_first_allowed_non_playable_one() {
    let mut e = world();
    let playable = weapon(&mut e, 1, 0, true);
    let ranged = weapon(&mut e, 2, 3, false);
    let melee = weapon(&mut e, 3, 2, false);
    let second = weapon(&mut e, 4, 1, false);
    set_container(
        &mut e,
        &[(1, second), (1, melee), (1, ranged), (1, playable)],
    );
    let list = LISTS + 0x100;
    set_changes(&mut e, &[(melee, 0, list)]);
    // The base's style allows anything; the actor's own (preferred) only melee.
    e.mem.set_u32(BASE + 0x1d4, BASE_STYLE);
    set_extra(CHAR + 0x44, 0x69, XSTYLE);
    e.mem.set_u32(XSTYLE + 0xc, STYLE);
    e.mem.set_u8(STYLE + 0x40, 1);
    run(&mut e, CHAR);
    assert_eq!(
        called(0x0088_c650),
        vec![vec![CHAR, melee, 1, list, 1, 0, 0]]
    );
    // Ranged only, from the base when the actor has none: the ranged one.
    calls();
    set_extra(CHAR + 0x44, 0x69, 0);
    e.mem.set_u8(BASE_STYLE + 0x40, 2);
    run(&mut e, CHAR);
    assert_eq!(called(0x0088_c650), vec![vec![CHAR, ranged, 1, 0, 1, 0, 0]]);
    // An extra combat style with no style falls back to the base's (melee only here).
    set_extra(CHAR + 0x44, 0x69, XSTYLE);
    e.mem.set_u32(XSTYLE + 0xc, 0);
    e.mem.set_u8(BASE_STYLE + 0x40, 1);
    run(&mut e, CHAR);
    assert_eq!(
        called(0x0088_c650),
        vec![vec![CHAR, melee, 1, list, 1, 0, 0]]
    );
    // No style at all allows anything.
    e.mem.set_u32(BASE + 0x1d4, 0);
    run(&mut e, CHAR);
    assert_eq!(called(0x0088_c650), vec![vec![CHAR, ranged, 1, 0, 1, 0, 0]]);
}

/// A companion holding a weapon (its process at a high level names one) skips the weapon pass; a
/// process at a low level holds none.
#[test]
fn a_companion_holding_a_weapon_keeps_it() {
    let mut e = world();
    let w = weapon(&mut e, 1, 0, false);
    set_container(&mut e, &[(1, w)]);
    set_changes(&mut e, &[]);
    let held = ENTRIES + 0x400;
    let in_hand = weapon(&mut e, 2, 0, true);
    e.mem.set_u32(held + 8, in_hand);
    e.mem.set_u32(CHAR + 0x68, PROCESS);
    e.mem.set_u32(PROCESS + 0x114, held);
    e.mem.set_u8(PROCESS + 0x28, 1);
    run(&mut e, CHAR);
    assert!(called(0x0088_c650).is_empty());
    e.mem.set_u8(PROCESS + 0x28, 2);
    run(&mut e, CHAR);
    assert_eq!(called(0x0088_c650), vec![vec![CHAR, w, 1, 0, 1, 0, 0]]);
}

/// JIP's inventory: the base container's count of an item summed over its entries, plus its
/// changes entry's count; an entry holding a leveled item counts alone; a total of 0 or below,
/// a leveled list form and an entry with no form stay out; changes-only items with a count above
/// 0 come in. The pass goes by address, whatever the lists' order.
#[test]
fn the_inventory_counts_as_jips_does() {
    let mut e = world();
    let sold = weapon(&mut e, 1, 0, false);
    let leveled = weapon(&mut e, 2, 0, false);
    let added = weapon(&mut e, 3, 0, false);
    let first = weapon(&mut e, 4, 0, false);
    let list_form = item(5);
    e.mem.set_u8(list_form + 4, 0x34);
    e.mem.set_u8(list_form + 0x100, 0x80);
    // `sold`: 1 + 1 in the base, -2 changed: gone. `leveled`: 3 in the base, its entry (a leveled
    // item, count 1) counts alone. `first` (highest address): base 1.
    set_container(
        &mut e,
        &[
            (1, first),
            (1, sold),
            (3, leveled),
            (1, sold),
            (1, list_form),
            (1, 0),
        ],
    );
    let lev_list = LISTS + 0x200;
    e.mem.set_u8(lev_list + 0xd, 0x80);
    set_changes(
        &mut e,
        &[
            (sold, -2, 0),
            (leveled, 1, lev_list),
            (added, 2, 0),
            (0, 5, 0),
        ],
    );
    let mut items = Items::new();
    assert!(inventory_items(&mut e, CHAR, WEAPON, &mut items));
    let got: Vec<(u32, (i32, u32))> = items.into_iter().collect();
    assert_eq!(
        got,
        vec![
            (leveled, (1, ENTRIES + 0x10)),
            (added, (2, ENTRIES + 0x20)),
            (first, (1, 0)),
        ]
    );
    // A leveled entry whose count is 0, and an ordinary one brought to 0: out; `sold` (2 in the
    // base, no entry now) in.
    set_changes(&mut e, &[(leveled, 0, lev_list), (first, -1, 0)]);
    let mut items = Items::new();
    assert!(inventory_items(&mut e, CHAR, WEAPON, &mut items));
    assert_eq!(items.into_iter().collect::<Vec<_>>(), vec![(sold, (2, 0))]);
    // Items already in the map are kept as they were.
    let mut items = Items::from([(first, (9, 7))]);
    set_changes(&mut e, &[]);
    assert!(inventory_items(&mut e, CHAR, WEAPON, &mut items));
    assert_eq!(
        items.into_iter().collect::<Vec<_>>(),
        vec![(sold, (2, 0)), (leveled, (3, 0)), (first, (9, 7))]
    );
    // Another type finds nothing; all types (0) find the three weapons, not the leveled list.
    let mut items = Items::new();
    assert!(!inventory_items(&mut e, CHAR, ARMOR, &mut items));
    assert!(inventory_items(&mut e, CHAR, 0, &mut items));
    assert_eq!(
        items.keys().copied().collect::<Vec<_>>(),
        vec![sold, leveled, first]
    );
}

/// No base container (a base whose vtable does not answer "actor base") or no changes list: no
/// items, nothing equipped and no armour pass.
#[test]
fn no_container_or_no_changes_means_no_items() {
    let mut e = world();
    let w = weapon(&mut e, 1, 0, false);
    set_container(&mut e, &[(1, w)]);
    set_changes(&mut e, &[]);
    e.mem.set_u32(CHAR + 0x1b4, BIPED);
    set_extra(CHAR + 0x44, 0x15, 0);
    run(&mut e, CHAR);
    assert!(calls().is_empty());
    set_extra(CHAR + 0x44, 0x15, XCHANGES);
    e.mem.set_u32(BASE_VT + 0xf8, 0);
    run(&mut e, CHAR);
    assert!(calls().is_empty());
}

/// The armour pass: worn armours (by `TESObjectARMO`'s vtable) leave the map and take their
/// slots; each other armour, by address, that is not playable and whose slots are free is equipped
/// and takes them (a later one wanting a slot taken here is skipped); then DR and DT are set to -1
/// and summed again (DT first, then DR).
#[test]
fn a_companion_equips_its_non_playable_armour_in_free_slots() {
    let mut e = world();
    let worn = armor(&mut e, 1, 0x4, false);
    let playable = armor(&mut e, 2, 0x10, true);
    let clash = armor(&mut e, 3, 0x6, false);
    let hat = armor(&mut e, 4, 0x400, false);
    let second_hat = armor(&mut e, 5, 0x400, false);
    let gloves = armor(&mut e, 6, 0x8, false);
    // Worn, with no slot bits: only its leaving the map keeps it from being equipped again.
    let worn_ring = armor(&mut e, 7, 0, false);
    set_container(
        &mut e,
        &[
            (1, worn_ring),
            (1, gloves),
            (1, second_hat),
            (1, hat),
            (1, clash),
            (1, playable),
            (1, worn),
        ],
    );
    let list = LISTS + 0x300;
    set_changes(&mut e, &[(hat, 0, list)]);
    // A process holding a weapon: no weapon pass.
    let held = ENTRIES + 0x400;
    e.mem.set_u32(held + 8, item(9));
    e.mem.set_u32(CHAR + 0x68, PROCESS);
    e.mem.set_u32(PROCESS + 0x114, held);
    e.mem.set_u32(CHAR + 0x1b4, BIPED);
    e.mem.set_u32(BIPED + 0x2c + 2 * 0x10, worn);
    e.mem.set_u32(BIPED + 0x2c + 8 * 0x10, worn_ring);
    // A slot holding a form that is not an armour (a race, say) is passed over.
    e.mem.set_u32(BIPED + 0x2c + 5 * 0x10, item(9));
    e.mem.set_f32(CHAR + 0x1b8, 30.0);
    e.mem.set_f32(CHAR + 0x1bc, 7.0);
    e.register_double(0x008d_2110, |e, w| {
        CALLS.with(|c| {
            c.borrow_mut().push((
                0x008d_2110,
                vec![w[0], e.mem.u32(w[0] + 0x1b8), e.mem.u32(w[0] + 0x1bc)],
                false,
            ))
        });
        Ret::default()
    });
    run(&mut e, CHAR);
    let all = calls();
    let equipped: Vec<Vec<u32>> = all
        .iter()
        .filter(|(f, _, _)| *f == 0x0088_c650)
        .map(|(_, w, _)| w.clone())
        .collect();
    assert_eq!(
        equipped,
        vec![
            vec![CHAR, hat, 1, list, 1, 0, 0],
            vec![CHAR, gloves, 1, 0, 1, 0, 0],
        ]
    );
    let tail: Vec<u32> = all.iter().skip(2).map(|(f, _, _)| *f).collect();
    assert_eq!(tail, vec![0x008d_2110, 0x008d_22b0]);
    let minus_one = (-1.0f32).to_bits();
    assert_eq!(all[2].1, vec![CHAR, minus_one, minus_one]);
}

/// No `BipedAnim`: the weapon pass still runs, then the command ends (no armour, no DR/DT).
#[test]
fn no_biped_anim_ends_after_the_weapon_pass() {
    let mut e = world();
    let w = weapon(&mut e, 1, 0, false);
    let a = armor(&mut e, 2, 0x4, false);
    set_container(&mut e, &[(1, w), (1, a)]);
    set_changes(&mut e, &[]);
    run(&mut e, CHAR);
    assert_eq!(
        calls()
            .into_iter()
            .map(|(f, w, _)| (f, w[1]))
            .collect::<Vec<_>>(),
        vec![(0x0088_c650, w)]
    );
}

/// JIP's map is not emptied between the passes: the weapons found by the first stay in it for the
/// armour pass, tested with the words at `+0x74` and `+0x78` as an armour's mask and flags (in a
/// weapon, its enchantment component's vtable and its enchantment), and equipped by it when those
/// pass. With a weapon in hand the first pass does not run and the weapons never enter the map.
#[test]
fn weapons_of_the_first_pass_stay_in_the_armour_pass_as_in_jip() {
    let mut e = world();
    let first = weapon(&mut e, 1, 0, false);
    let other = weapon(&mut e, 2, 0, false);
    let hat = armor(&mut e, 3, 0x400, false);
    // `first` (equipped by the weapon pass): no 0x40 at +0x78, read as "playable": skipped here.
    e.mem.set_u32(first + 0x74, 0x0101_0000);
    e.mem.set_u32(first + 0x78, 0x0500_0000);
    // `other`: 0x40 at +0x78 (an enchantment address with that bit), read as "not playable", and
    // a word at +0x74 sharing no bit with the hat's slot: equipped, and the hat after it.
    e.mem.set_u32(other + 0x74, 0x0101_0000);
    e.mem.set_u32(other + 0x78, 0x0500_0040);
    set_container(&mut e, &[(1, hat), (1, other), (1, first)]);
    set_changes(&mut e, &[]);
    e.mem.set_u32(CHAR + 0x1b4, BIPED);
    run(&mut e, CHAR);
    let equipped: Vec<u32> = called(0x0088_c650).into_iter().map(|w| w[1]).collect();
    assert_eq!(equipped, vec![first, other, hat]);
    // A weapon in hand: only the hat.
    let held = ENTRIES + 0x400;
    e.mem.set_u32(held + 8, first);
    e.mem.set_u32(CHAR + 0x68, PROCESS);
    e.mem.set_u32(PROCESS + 0x114, held);
    run(&mut e, CHAR);
    let equipped: Vec<u32> = called(0x0088_c650).into_iter().map(|w| w[1]).collect();
    assert_eq!(equipped, vec![hat]);
}

// ---- the container menu ----

/// A menu open on `CHAR` with `entry` selected, its list's tile selected and the cursor on the
/// tile with id 20 (`activeTile`).
fn menu_world(e: &mut Engine, entry: u32) {
    e.mem.set_u32(0x011d_93f8, MENU);
    e.mem.set_u32(0x011d_93fc, entry);
    e.mem.set_u32(MENU + 0x74, CHAR);
    e.mem.set_u32(MENU + 0xd8, AREA + 0xe100);
    e.mem.set_u32(0x011d_8a80, MANAGER);
    e.mem.set_u32(MANAGER + 0xcc, TILE);
    e.mem.set_u32(MANAGER + 0xbc, 0);
    // Values sorted by id: 0xFA1 (x), 0xFAA (id), 0xFB0.
    tile_values(e, TILE, &[(0xfa1, 3.0), (0xfaa, 20.0), (0xfb0, 1.0)]);
}

fn tile_values(e: &mut Engine, tile: u32, values: &[(u32, f32)]) {
    let base = if tile == TILE { VALUES } else { VALUES + 0x80 };
    e.mem.set_u32(tile + 0x14, base);
    e.mem.set_u32(tile + 0x18, values.len() as u32);
    for (i, &(id, num)) in values.iter().enumerate() {
        let v = VALUE + (base - VALUES) * 4 + i as u32 * 0x14;
        e.mem.set_u32(base + i as u32 * 4, v);
        e.mem.set_u32(v, id);
        e.mem.set_f32(v + 8, num);
    }
}

/// The selection's entry: (form, count, extra list or 0).
fn selection(e: &mut Engine, form: u32, count: i32, list: u32) -> u32 {
    set_changes(e, &[(form, count, list)])[0]
}

/// An armour not worn: EquipObject on the companion (the item, count 1, its extra list, 1, 0, 0)
/// with JIP's flag set for the call only, so the "cannot wear" extra does not stop it; then the
/// player hears the equip sound (equip, equip) and the menu's lists are updated.
#[test]
fn the_menu_equips_an_armour_the_companion_cannot_wear() {
    let mut e = world();
    let a = armor(&mut e, 1, 4, true);
    let list = LISTS + 0x400;
    e.mem.set_u8(list + 0xf, 0x40);
    let entry = selection(&mut e, a, 1, list);
    menu_world(&mut e, entry);
    assert_eq!(cannot_wear(&mut e, list), 1, "before");
    assert_eq!(run(&mut e, 0), 0.0);
    assert_eq!(
        calls(),
        vec![
            (0x0088_c830, vec![CHAR, a, 1, list, 1, 0, 0], true),
            (0x008a_ded0, vec![PLAYER_REF, a, 1, 1], false),
            (0x0075_c280, vec![MENU, a], false),
        ]
    );
    assert_eq!(asked(), vec![0], "the game's equip saw no cannot-wear");
    assert_eq!(cannot_wear(&mut e, list), 1, "after");
    assert!(!e.ext::<Jip>().ai_hooks.force_equip_call);
}

/// A worn armour (its extra list's worn bit) is taken off: UnEquipObject, count 1, sound (0, 0).
#[test]
fn the_menu_takes_off_a_worn_armour() {
    let mut e = world();
    let a = armor(&mut e, 1, 4, true);
    let list = LISTS + 0x400;
    e.mem.set_u8(list + 0xa, 0x40);
    let entry = selection(&mut e, a, 3, list);
    menu_world(&mut e, entry);
    run(&mut e, 0);
    assert_eq!(
        calls(),
        vec![
            (0x0088_d7d0, vec![CHAR, a, 1, list, 1, 0, 0], true),
            (0x008a_ded0, vec![PLAYER_REF, a, 0, 0], false),
            (0x0075_c280, vec![MENU, a], false),
        ]
    );
}

/// Weapons: a worn one is taken off as the whole stack; a grenade or mine (types 11, 12) or one
/// with the second flags' bit 0 answers -1 and nothing runs; one the companion's combat style
/// refuses answers the restriction; a thrown weapon (type 10) is equipped as the whole stack, any
/// other one by one.
#[test]
fn the_menu_and_weapons() {
    let mut e = world();
    let list = LISTS + 0x400;
    let w = weapon(&mut e, 1, 5, true);
    e.mem.set_u8(list + 0xa, 0x40);
    let entry = selection(&mut e, w, 4, list);
    menu_world(&mut e, entry);
    run(&mut e, 0);
    assert_eq!(called(0x0088_d7d0), vec![vec![CHAR, w, 4, list, 1, 0, 0]]);
    e.mem.set_u8(list + 0xa, 0);
    for (kind, flags2) in [(11u8, 0u32), (12, 0), (5, 1)] {
        e.mem.set_u8(w + 0xf4, kind);
        e.mem.set_u32(w + 0x12c, flags2);
        assert_eq!(run(&mut e, 0), -1.0, "type {kind}");
        assert!(calls().is_empty());
    }
    e.mem.set_u32(w + 0x12c, 0);
    // Melee only (1) refuses a ranged weapon (type 5); ranged only (2) a melee one (type 0).
    e.mem.set_u32(BASE + 0x1d4, STYLE);
    for (restriction, kind) in [(1u8, 5u8), (2, 0)] {
        e.mem.set_u8(STYLE + 0x40, restriction);
        e.mem.set_u8(w + 0xf4, kind);
        assert_eq!(run(&mut e, 0), f64::from(restriction));
        assert!(calls().is_empty());
    }
    e.mem.set_u8(STYLE + 0x40, 0);
    e.mem.set_u8(w + 0xf4, 10);
    run(&mut e, 0);
    assert_eq!(called(0x0088_c830), vec![vec![CHAR, w, 4, list, 1, 0, 0]]);
    e.mem.set_u8(w + 0xf4, 9);
    run(&mut e, 0);
    assert_eq!(called(0x0088_c830), vec![vec![CHAR, w, 1, list, 1, 0, 0]]);
}

/// An aid item is used (EquipObject, count 1, no extra list read: its lists are not passed); any
/// other kind of item does nothing. An armour with no extra lists is equipped with none.
#[test]
fn the_menu_and_other_items() {
    let mut e = world();
    let list = LISTS + 0x400;
    let aid = item(1);
    e.mem.set_u8(aid + 4, 0x2f);
    let entry = selection(&mut e, aid, 5, list);
    menu_world(&mut e, entry);
    run(&mut e, 0);
    assert_eq!(called(0x0088_c830), vec![vec![CHAR, aid, 1, 0, 1, 0, 0]]);
    e.mem.set_u8(aid + 4, 0x2e);
    run(&mut e, 0);
    assert!(calls().is_empty());
    let a = armor(&mut e, 2, 4, true);
    let entry = selection(&mut e, a, 1, 0);
    menu_world(&mut e, entry);
    run(&mut e, 0);
    assert_eq!(called(0x0088_c830), vec![vec![CHAR, a, 1, 0, 1, 0, 0]]);
}

/// Nothing happens without a selection, a menu, the list's selected tile, the cursor on tile 20
/// (`activeTile`, else `activeTileAlt`; a tile with no id value is -1), an item, or a `Character`
/// as the menu's reference (none counts as not one).
#[test]
fn the_menu_needs_its_list_selected_and_the_cursor_on_it() {
    let mut e = world();
    let a = armor(&mut e, 1, 4, true);
    let entry = selection(&mut e, a, 1, 0);
    let check = |e: &mut Engine, change: &dyn Fn(&mut Engine), equips: bool| {
        menu_world(e, entry);
        e.mem.set_u32(entry + 8, a);
        change(e);
        run(e, 0);
        assert_eq!(!called(0x0088_c830).is_empty(), equips);
    };
    check(&mut e, &|_| {}, true);
    check(&mut e, &|e| e.mem.set_u32(0x011d_93fc, 0), false);
    check(&mut e, &|e| e.mem.set_u32(0x011d_93f8, 0), false);
    check(&mut e, &|e| e.mem.set_u32(MENU + 0xd8, 0), false);
    check(&mut e, &|e| e.mem.set_u32(entry + 8, 0), false);
    check(&mut e, &|e| e.mem.set_u32(MENU + 0x74, OTHER_REF), false);
    check(&mut e, &|e| e.mem.set_u32(MENU + 0x74, 0), false);
    check(&mut e, &|e| e.mem.set_u32(0x011d_8a80, 0), false);
    check(
        &mut e,
        &|e| tile_values(e, TILE, &[(0xfa1, 20.0), (0xfaa, 19.0)]),
        false,
    );
    check(&mut e, &|e| tile_values(e, TILE, &[(0xfaa, 21.0)]), false);
    check(
        &mut e,
        &|e| tile_values(e, TILE, &[(0xfa1, 20.0), (0xfb0, 20.0)]),
        false,
    );
    check(&mut e, &|e| tile_values(e, TILE, &[(0xfaa, 20.9)]), true);
    check(
        &mut e,
        &|e| tile_values(e, TILE, &[(0xfaa, f32::NAN)]),
        false,
    );
    check(&mut e, &|e| e.mem.set_u32(MANAGER + 0xcc, 0), false);
    check(
        &mut e,
        &|e| {
            e.mem.set_u32(MANAGER + 0xcc, 0);
            e.mem.set_u32(MANAGER + 0xbc, TILE_ALT);
            tile_values(e, TILE_ALT, &[(0xfaa, 20.0)]);
        },
        true,
    );
    check(
        &mut e,
        &|e| {
            // activeTile wins over activeTileAlt.
            e.mem.set_u32(MANAGER + 0xbc, TILE_ALT);
            tile_values(e, TILE_ALT, &[(0xfaa, 7.0)]);
        },
        true,
    );
}

/// The tile value search finds every id of a longer sorted list (a binary search, as JIP's).
#[test]
fn the_tile_id_is_found_wherever_it_sits() {
    let mut e = world();
    for at in 0..7u32 {
        let values: Vec<(u32, f32)> = (0..7u32)
            .map(|i| {
                if i == at {
                    (0xfaa, 20.0)
                } else if i < at {
                    (0xf00 + i, 0.0)
                } else {
                    (0xfb0 + i, 0.0)
                }
            })
            .collect();
        menu_world(&mut e, 0);
        tile_values(&mut e, TILE, &values);
        assert_eq!(active_tile_id(&e), 20, "at {at}");
    }
}

/// A guest exception (a panic) in the game's equip still puts JIP's flag down.
#[test]
fn the_flag_goes_down_when_the_equip_stops() {
    let mut e = world();
    let a = armor(&mut e, 1, 4, true);
    let entry = selection(&mut e, a, 1, 0);
    menu_world(&mut e, entry);
    e.register_double(0x0088_c830, |e, _| {
        assert!(e.ext::<Jip>().ai_hooks.force_equip_call);
        panic!("a guest exception");
    });
    let at = crate::test_support::execute_of(&e, CCC_SET_EQUIPPED);
    let words = line(&mut e, CCC_SET_EQUIPPED, &[]);
    let stop = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| e.call(at, &words)));
    assert!(stop.is_err());
    assert!(!e.ext::<Jip>().ai_hooks.force_equip_call);
}
