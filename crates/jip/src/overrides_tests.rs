//! Tests of JIP's versions of xNVSE's and the game's commands: in the records only while JIP
//! answers "installed" (the vanilla ones also only with the row), and each one's answers from the
//! C++ over stand-ins of the game's objects. None has been run in the real game here.

use super::*;
use crate::test_support::{call, engine, s};
use std::cell::RefCell;

const AREA: u32 = 0x0500_0000;
const REFR: u32 = AREA;
const BASE: u32 = AREA + 0x200;
const LOADED: u32 = AREA + 0x400;
const NODE: u32 = AREA + 0x500;
const CHILDREN: u32 = AREA + 0x600;
const LIGHT: u32 = AREA + 0x700;
const SUB_NODE: u32 = AREA + 0x800;
const VTABLES: u32 = AREA + 0x1000;
const LISTS: u32 = AREA + 0x2000;
const NAMES: u32 = AREA + 0x4000;
const SETTINGS: u32 = AREA + 0x5000;
const MANAGER: u32 = AREA + 0x6000;

thread_local! {
    static CALLS: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
}

fn calls() -> Vec<String> {
    CALLS.with(|c| std::mem::take(&mut *c.borrow_mut()))
}

fn log(s: String) {
    CALLS.with(|c| c.borrow_mut().push(s));
}

fn world(installed: bool) -> Engine {
    calls();
    let mut e = engine();
    e.map(AREA, 0x1_0000);
    for page in [
        0x011d_8000,
        0x011f_3000,
        0x011c_a000,
        0x011c_8000,
        0x011f_9000,
        0x011c_c000,
        0x011c_3000,
    ] {
        e.map(page, 0x1000);
    }
    e.register(SET_3D, |_, a| {
        log(format!("set3d {:x} {} {}", a[0], a[1], a[2]));
        Ret::default()
    });
    e.register(QUEUE_REFERENCE, |_, a| {
        log(format!("queue {:x} {:x} {} {}", a[0], a[1], a[2], a[3]));
        Ret::default()
    });
    e.register(PLAYER_UPDATE_3D, |_, a| {
        log(format!("player3d {:x}", a[0]));
        Ret::default()
    });
    e.mem.set_u32(MODEL_LOADER, 0x0bad_0001);
    e.mem.set_u32(INTERFACE_MANAGER, MANAGER);
    crate::install_as(&mut e, installed);
    e
}

/// A form at `at` with a vtable whose `slots` answer true.
fn form(e: &mut Engine, at: u32, id: u32, kind: u8, vtable: u32, slots: &[u32]) {
    for slot in slots {
        e.mem.set_u32(vtable + slot, RETURN_TRUE);
    }
    e.mem.set_u32(at, vtable);
    e.mem.set_u8(at + 4, kind);
    e.mem.set_u32(at + 0xc, id);
}

/// In the records only while JIP answers "installed"; the vanilla ones also only with the row,
/// given before JIP's start or after it; the two setters read a double then.
#[test]
fn in_the_records_only_when_installed() {
    let record = |e: &Engine, op: u32| e.script_tables().record(op);
    let before = engine();
    let words = |e: &Engine, op: u32| {
        (
            e.mem.u32(record(e, op) + 0x18),
            e.mem.u32(record(e, op) + 0x20),
        )
    };
    for row_first in [false, true] {
        let mut e = engine();
        if row_first {
            set_vanilla_commands(&mut e, true);
        }
        crate::install_as(&mut e, false);
        set_vanilla_commands(&mut e, true);
        for o in XNVSE.iter().chain(VANILLA) {
            assert_eq!(
                words(&e, o.opcode),
                words(&before, o.opcode),
                "{:04x}",
                o.opcode
            );
        }
    }
    let mut e = engine();
    crate::install_as(&mut e, true);
    for (i, o) in XNVSE.iter().enumerate() {
        let at = ROUTINES + 8 * i as u32;
        assert_eq!(words(&e, o.opcode).0, at, "{:04x}", o.opcode);
        if o.eval.is_some() {
            assert_eq!(words(&e, o.opcode).1, at + 4);
        } else {
            assert_eq!(words(&e, o.opcode).1, words(&before, o.opcode).1);
        }
    }
    for o in VANILLA {
        assert_eq!(
            words(&e, o.opcode),
            words(&before, o.opcode),
            "no row: {:04x}",
            o.opcode
        );
    }
    let params = e.mem.u32(record(&e, 0x149b) + 0x14);
    assert_eq!(e.mem.u32(params + 4), 0);
    assert_eq!(
        e.mem.u32(params + 16),
        0x2c,
        "the second parameter a double"
    );
    assert_eq!(e.mem.u32(record(&e, 0x149d) + 0x14), params);
    // The row, before and after JIP's start.
    for row_first in [false, true] {
        let mut e = engine();
        if row_first {
            set_vanilla_commands(&mut e, true);
        }
        crate::install_as(&mut e, true);
        set_vanilla_commands(&mut e, true);
        for (i, o) in VANILLA.iter().enumerate() {
            let at = ROUTINES + 8 * (XNVSE.len() + i) as u32;
            assert_eq!(words(&e, o.opcode).0, at, "{:04x}", o.opcode);
        }
    }
}

/// A reference with 3D (its root node at `NODE`).
fn with_3d(e: &mut Engine, id: u32) {
    form(e, REFR, id, 0x3a, VTABLES, &[VT_IS_REFERENCE]);
    e.mem.set_u32(REFR + 0x64, LOADED);
    e.mem.set_u32(LOADED + 0x14, NODE);
}

/// `Update3D`: another reference lets go of its 3D (`Set3D(0, 1)`), is marked and queued; with
/// the root's flag bit `0x20000000` its point lights are hidden first, a light below a child node
/// too; the player through the game's own update; the result stays 0 (xNVSE's answers 1).
#[test]
fn update_3d_as_jip() {
    let mut e = world(true);
    with_3d(&mut e, 0x0100_0001);
    // The root has two children: a node (with a light below it) and a light, both flagged; an
    // empty slot; an unflagged light.
    e.mem.set_u32(NODE + 0x30, 0x2000_0000);
    e.mem.set_u32(NODE + 0xa0, CHILDREN);
    e.mem.set_u16(NODE + 0xa6, 4);
    let node_vt = VTABLES + 0x400;
    e.mem.set_u32(node_vt + VT_IS_NODE, RETURN_THIS);
    for (i, child) in [SUB_NODE, LIGHT, 0, LIGHT + 0x80].into_iter().enumerate() {
        e.mem.set_u32(CHILDREN + 4 * i as u32, child);
    }
    e.mem.set_u32(SUB_NODE, node_vt);
    e.mem.set_u32(SUB_NODE + 0x30, 0x2000_0000);
    e.mem.set_u32(SUB_NODE + 0xa0, CHILDREN + 0x40);
    e.mem.set_u16(SUB_NODE + 0xa6, 1);
    e.mem.set_u32(CHILDREN + 0x40, LIGHT + 0x40);
    for light in [LIGHT, LIGHT + 0x40] {
        e.mem.set_u32(light, VTBL_POINT_LIGHT);
        e.mem.set_u32(light + 0x30, 0x2000_0000);
    }
    e.mem.set_u32(LIGHT + 0x80, VTBL_POINT_LIGHT);
    let refr = REFR;
    let (_, result) = {
        let words = crate::test_support::line(&mut e, 0x152d, &[]);
        let mut words = words;
        words[2] = refr;
        let at = e.mem.u32(e.script_tables().record(0x152d) + 0x18);
        let ok = e.call(at, &words).eax & 0xff;
        (ok, e.mem.f64(crate::test_support::RESULT))
    };
    assert_eq!(result, 0.0, "left as it was");
    assert_eq!(
        calls(),
        [
            format!("set3d {refr:x} 0 1"),
            format!("queue bad0001 {refr:x} 1 0")
        ]
    );
    assert_eq!(e.mem.u8(REFR + 0x5f), 1, "marked");
    assert_eq!(e.mem.u32(LIGHT + 0x30), 0x2000_0001);
    assert_eq!(e.mem.u32(LIGHT + 0x70), 0x2000_0001, "below a child node");
    assert_eq!(e.mem.u32(LIGHT + 0xb0), 0, "not flagged: left");
    // Without the root's flag bit no light is touched.
    e.mem.set_u32(NODE + 0x30, 0);
    e.mem.set_u32(LIGHT + 0x30, 0x2000_0000);
    ref_update_3d(&mut e, REFR);
    assert_eq!(e.mem.u32(LIGHT + 0x30), 0x2000_0000);
    calls();
    // The player: marked once, queued without letting go; marked already, nothing.
    let mut e = world(true);
    with_3d(&mut e, PLAYER_ID);
    ref_update_3d(&mut e, REFR);
    assert_eq!(calls(), [format!("queue bad0001 {refr:x} 1 0")]);
    ref_update_3d(&mut e, REFR);
    assert!(calls().is_empty(), "marked already");
    // No 3D: nothing.
    e.mem.set_u32(LOADED + 0x14, 0);
    e.mem.set_u8(REFR + 0x5f, 0);
    ref_update_3d(&mut e, REFR);
    assert!(calls().is_empty());
    // The command on the player calls the game's own update.
    let mut words = crate::test_support::line(&mut e, 0x152d, &[]);
    words[2] = REFR;
    e.mem.set_u32(REFR + 0xc, PLAYER_ID);
    let at = e.mem.u32(e.script_tables().record(0x152d) + 0x18);
    e.call(at, &words);
    assert_eq!(calls(), [format!("player3d {refr:x}")]);
}

/// `IsMenuMode`'s cases.
#[test]
fn menu_modes() {
    let e = world(true);
    e.mem.set_u32(MANAGER + 0xc, 1);
    assert!(!is_menu_mode(&e, 0));
    e.mem.set_u32(MANAGER + 0xc, 2);
    assert!(is_menu_mode(&e, 0));
    e.mem.set_u32(MANAGER + 0x4bc, 3);
    assert!(is_menu_mode(&e, 1), "the Pip-Boy mode");
    e.mem.set_u8(PIPBOY_FLAG, 1);
    assert!(!is_menu_mode(&e, 1), "with the flag: the three menus");
    e.mem.set_u8(MENU_VISIBILITY + MENU_MAP, 1);
    assert!(is_menu_mode(&e, 1));
    // The stack: a menu id at or above 0x3E9 before a 0.
    e.mem.set_u32(MANAGER + 0x114, 5);
    e.mem.set_u32(MANAGER + 0x118, 0);
    e.mem.set_u32(MANAGER + 0x11c, 0x3f0);
    assert!(!is_menu_mode(&e, 2), "stops at the 0");
    e.mem.set_u32(MANAGER + 0x118, 0x3e9);
    assert!(is_menu_mode(&e, 2));
    assert!(!is_menu_mode(&e, 3));
    e.mem.set_u8(CONSOLE_OPEN, 1);
    assert!(is_menu_mode(&e, 3));
    e.mem.set_u8(MENU_VISIBILITY + MENU_LOADING, 1);
    e.mem.set_u8(MENU_VISIBILITY + MENU_START, 1);
    assert!(!is_menu_mode(&e, 4));
    e.mem.set_u8(MENU_VISIBILITY + MENU_HUD_MAIN, 1);
    assert!(is_menu_mode(&e, 4));
    assert!(is_menu_mode(&e, MENU_MAP));
    assert!(!is_menu_mode(&e, 0x43d), "past the last menu");
    assert!(!is_menu_mode(&e, 5));
}

/// A list's nodes at `at`: the first inline in a form at `form_at` (`+0x18`), the rest after it.
fn list_form(e: &mut Engine, form_at: u32, items: &[u32]) {
    e.mem.set_u8(form_at + 4, LIST_FORM);
    let head = form_at + 0x18;
    let mut node = head;
    for (i, item) in items.iter().enumerate() {
        e.mem.set_u32(node, *item);
        let next = if i + 1 < items.len() {
            LISTS + 0x800 + 8 * i as u32
        } else {
            0
        };
        e.mem.set_u32(node + 4, next);
        node = next;
    }
}

/// A container base with `objects` (count, form) and a reference of it, as the exe lays them out:
/// `TESContainer::objectList` is a `BSSimpleList<ContainerObject *>` with its first node inline at
/// the container's `+4` (engine-fallout-forms `tescontainer.rs`: `ContainerObject` is count `+0`,
/// form `+4`, extra `+8`); a container base's `TESContainer` is at `+0x30` (`tesobjectcont.rs`).
fn container_world(e: &mut Engine, objects: &[(i32, u32)]) {
    form(e, BASE, 0x0100_0010, CONTAINER, VTABLES + 0x200, &[]);
    form(e, REFR, 0x0100_0011, 0x3a, VTABLES, &[VT_IS_REFERENCE]);
    e.mem.set_u32(REFR + 0x20, BASE);
    let list = BASE + 0x30 + 4;
    for (i, (count, item)) in objects.iter().enumerate() {
        let object = LISTS + 0x100 + 0x10 * i as u32;
        e.mem.set_i32(object, *count);
        e.mem.set_u32(object + 4, *item);
        let node = if i == 0 {
            list
        } else {
            LISTS + 0x200 + 8 * i as u32
        };
        e.mem.set_u32(node, object);
        let next = if i + 1 < objects.len() {
            LISTS + 0x200 + 8 * (i as u32 + 1)
        } else {
            0
        };
        e.mem.set_u32(node + 4, next);
    }
}

/// The reference's container changes, as the exe lays them out: `ExtraContainerChanges::pChanges`
/// (`+0xC`) is an `InventoryChanges*` whose `pListofChanges` (`+0`) is a
/// `BSSimpleList<ItemChange *>*`, the list's first node itself; an `ItemChange` is
/// `pExtraObjectList` `+0` (a `BSSimpleList<ExtraDataList *>*`, again the first node itself:
/// `ItemChange::GetExtraTotalDefaultCount`, `004bc980`, walks from `*this`), `iNumber` `+4`,
/// `pContainerObj` `+8` (engine-fallout-forms `inventorychanges.rs`). `entries` are (form, count
/// change, the entry's extra data lists).
fn changes(e: &mut Engine, entries: &[(u32, i32, &[u32])]) {
    let extra = LISTS + 0x400;
    let inventory = LISTS + 0x440;
    e.mem.set_u32(extra + 0xc, inventory);
    e.mem.set_u32(inventory, LISTS + 0x480);
    let mut spare = LISTS + 0x700;
    for (i, (item, delta, lists)) in entries.iter().enumerate() {
        let entry = LISTS + 0x4c0 + 0x10 * i as u32;
        let head = if lists.is_empty() { 0 } else { spare };
        for (j, list) in lists.iter().enumerate() {
            let node = spare;
            spare += 8;
            e.mem.set_u32(node, *list);
            let next = if j + 1 < lists.len() { spare } else { 0 };
            e.mem.set_u32(node + 4, next);
        }
        e.mem.set_u32(entry, head);
        e.mem.set_i32(entry + 4, *delta);
        e.mem.set_u32(entry + 8, *item);
        let node = LISTS + 0x480 + 8 * i as u32;
        e.mem.set_u32(node, entry);
        let next = if i + 1 < entries.len() { node + 8 } else { 0 };
        e.mem.set_u32(node + 4, next);
    }
    e.register(GET_BY_TYPE, |_, a| Ret {
        eax: if a[1] == EXTRA_CONTAINER_CHANGES {
            LISTS + 0x400
        } else {
            0
        },
        ..Ret::default()
    });
}

/// An `ExtraDataList` at `at`: a vtable word, its first extra (`+4`) and the presence bits from
/// `+8`, with type 0x2F (`ExtraLeveledItem`, byte `+0xD` bit `0x80`) as given.
fn extra_list(e: &mut Engine, at: u32, vtable: u32, leveled: bool) {
    e.mem.set_u32(at, vtable);
    e.mem.set_u32(at + 4, 0);
    e.mem.set_u8(at + 0xd, if leveled { 0x80 } else { 0 });
}

/// `GetItemCount`: the base container's entries summed with the reference's changes entry (none
/// below 0), the entry alone when one of its extra data lists holds a leveled item (the first or a
/// later one) or the base has none, a form list's forms summed, 0 for a base without a
/// container. The extra data lists are walked from the node `pExtraObjectList` points to: a list
/// whose own vtable word points at a word with the leveled bit is not taken for a leveled one (a
/// walk that reads one pointer too deep would).
#[test]
fn item_counts() {
    let mut e = world(true);
    let (apple, pear) = (AREA + 0x9000, AREA + 0x9100);
    container_world(&mut e, &[(2, apple), (3, apple), (1, pear)]);
    e.register(GET_BY_TYPE, |_, _| Ret::default());
    assert_eq!(item_count(&mut e, REFR, apple), 5, "no changes");
    assert_eq!(item_count(&mut e, REFR, 0), 0, "no form");
    let (leveled, plain, decoy) = (AREA + 0xc000, AREA + 0xc100, AREA + 0xc200);
    extra_list(&mut e, leveled, VTABLES + 0xa00, true);
    // A list whose vtable word points at a "list" with the leveled bit.
    extra_list(&mut e, plain, decoy, false);
    extra_list(&mut e, decoy, 0, true);
    // apple -7: 5 - 7 below 0: 0. pear +2 with the leveled list first: 2 alone.
    changes(&mut e, &[(apple, -7, &[]), (pear, 2, &[leveled])]);
    assert_eq!(item_count(&mut e, REFR, apple), 0, "none below 0");
    assert_eq!(
        item_count(&mut e, REFR, pear),
        2,
        "the leveled list first: alone"
    );
    // The leveled list second.
    changes(&mut e, &[(pear, 2, &[plain, leveled])]);
    assert_eq!(
        item_count(&mut e, REFR, pear),
        2,
        "the leveled list later: alone"
    );
    // No leveled list (the decoy only through its vtable word): added to the base's.
    changes(&mut e, &[(pear, 2, &[plain])]);
    assert_eq!(item_count(&mut e, REFR, pear), 3, "added to the base's");
    // The base has none of the form: the entry alone, none below 0.
    let plum = AREA + 0x9400;
    changes(&mut e, &[(plum, 4, &[plain]), (pear, -1, &[])]);
    assert_eq!(item_count(&mut e, REFR, plum), 4, "the base has none");
    changes(&mut e, &[(plum, -4, &[])]);
    assert_eq!(item_count(&mut e, REFR, plum), 0);
    // A form list of both.
    changes(&mut e, &[(pear, 2, &[plain])]);
    let fl = AREA + 0x9200;
    list_form(&mut e, fl, &[apple, 0, pear]);
    assert_eq!(item_count(&mut e, REFR, fl), 5 + 3);
    // A base without a container.
    e.mem.set_u8(BASE + 4, 0x28);
    assert_eq!(item_count(&mut e, REFR, apple), 0);
}

/// `IsInList` with the reference's base form (a temporary actor's leveled base), `IsRefInList`'s
/// index of the form or its base form.
#[test]
fn lists() {
    let mut e = world(true);
    let (a, b, leveled) = (AREA + 0x9000, AREA + 0x9100, AREA + 0x9300);
    let fl = AREA + 0x9200;
    list_form(&mut e, fl, &[a, b]);
    form(&mut e, REFR, 0x0100_0011, 0x3a, VTABLES, &[VT_IS_REFERENCE]);
    e.mem.set_u32(REFR + 0x20, b);
    e.mem.set_u32(b + 0xc, 0x0100_0020);
    assert!(ref_in_form_list(&mut e, fl, REFR));
    assert_eq!(ref_index_in_list(&mut e, fl, b), 1);
    assert_eq!(ref_index_in_list(&mut e, fl, REFR), 1, "by its base form");
    form(&mut e, leveled, 0x0100_0021, 0x1f, VTABLES + 0x300, &[]);
    assert_eq!(ref_index_in_list(&mut e, fl, leveled), -1);
    // A temporary actor base (id 0xFF......, `+0xF8` true) with an ExtraLeveledCreature.
    form(
        &mut e,
        BASE,
        0xff00_0001,
        0x2a,
        VTABLES + 0x200,
        &[VT_IS_ACTOR_BASE],
    );
    e.mem.set_u32(REFR + 0x20, BASE);
    let extra = AREA + 0x9400;
    e.mem.set_u32(extra + 0xc, a);
    e.register(GET_BY_TYPE, |_, args| Ret {
        eax: if args[1] == EXTRA_LEVELED_CREATURE {
            AREA + 0x9400
        } else {
            0
        },
        ..Ret::default()
    });
    assert_eq!(ref_base_form(&mut e, REFR), a);
    assert_eq!(ref_index_in_list(&mut e, fl, REFR), 0);
    // A placeable water's water form.
    e.mem.set_u32(BASE, VTBL_PLACEABLE_WATER);
    e.mem.set_u32(BASE + 0x4c, b);
    assert_eq!(ref_base_form(&mut e, REFR), b);
}

/// A setting at `at` named `name`, holding `bits`.
fn setting(e: &mut Engine, at: u32, name: &[u8], bits: u32) {
    let text = NAMES + 0x40 * ((at - SETTINGS) / 0x10);
    e.mem.write(text, name);
    e.mem.set_u8(text + name.len() as u32, 0);
    e.mem.set_u32(at + 4, bits);
    e.mem.set_u32(at + 8, text);
}

/// The settings map: game settings by name without case, the INI collections' numeric and string
/// ones after them (a later replacing an earlier); get and set by type; a string setting's name
/// refused; "SETTING NOT FOUND" when there is none.
#[test]
fn numeric_settings() {
    let mut e = world(true);
    let s1 = SETTINGS;
    let s2 = SETTINGS + 0x10;
    let s3 = SETTINGS + 0x20;
    let s4 = SETTINGS + 0x30;
    setting(&mut e, s1, b"fJumpHeight", 2.5f32.to_bits());
    setting(&mut e, s2, b"iCount", (-3i32) as u32);
    setting(&mut e, s3, b"uBig", 7);
    setting(&mut e, s4, b"fJumpHeight", 9.0f32.to_bits());
    // The game settings' map: 2 buckets, s1 in one, s2 and s3 in the other.
    let games = AREA + 0x7000;
    e.mem.set_u32(GAME_SETTINGS, games);
    e.mem.set_u32(games + 0x110, 2);
    e.mem.set_u32(games + 0x114, games + 0x200);
    let item = |e: &mut Engine, at: u32, next: u32, value: u32| {
        e.mem.set_u32(at, next);
        e.mem.set_u32(at + 8, value);
    };
    item(&mut e, games + 0x300, 0, s1);
    item(&mut e, games + 0x310, games + 0x320, s2);
    item(&mut e, games + 0x320, 0, s3);
    e.mem.set_u32(games + 0x200, games + 0x300);
    e.mem.set_u32(games + 0x204, games + 0x310);
    // The INI collections: the first holds s4 (replacing s1's name), the others are empty.
    for (i, at) in INI_COLLECTIONS.iter().enumerate() {
        let coll = AREA + 0x8000 + 0x200 * i as u32;
        e.mem.set_u32(*at, coll);
        e.mem.set_u32(coll + 0x10c, if i == 0 { s4 } else { 0 });
    }
    assert_eq!(find_setting(&mut e, b"FJUMPHEIGHT"), Some(s4));
    assert_eq!(find_setting(&mut e, b"icount"), Some(s2));
    assert_eq!(setting_get(&e, s2), Some(-3.0));
    assert_eq!(setting_get(&e, s3), Some(7.0));
    setting_set(&mut e, s1, 0.1);
    assert_eq!(e.mem.f32(s1 + 4), 0.1f32);
    setting_set(&mut e, s2, 1e10);
    assert_eq!(e.mem.i32(s2 + 4), i32::MIN, "out of range: 0x80000000");
    setting_set(&mut e, s3, -1.0);
    assert_eq!(e.mem.u32(s3 + 4), 0xffff_ffff, "the low word of -1");
    // The command: by name; a string setting's name and an unknown one answer -1.
    let (ok, v) = call(&mut e, 0x149a, &[s(b"fjumpheight")]);
    assert_eq!((ok, v), (1, 9.0));
    assert_eq!(call(&mut e, 0x149a, &[s(b"sName")]).1, -1.0);
    assert_eq!(call(&mut e, 0x149c, &[s(b"fNone")]).1, -1.0);
}

/// The conversions: `(int)` gives 0x80000000 out of range, `cvtd2ul` the low word of the 64-bit
/// truncation, 0 outside it.
#[test]
fn conversions() {
    assert_eq!(to_i32(-2.9), -2);
    assert_eq!(to_i32(2_147_483_647.9), i32::MAX);
    assert_eq!(to_i32(2_147_483_648.0), i32::MIN);
    assert_eq!(to_i32(f64::NAN), i32::MIN);
    assert_eq!(cvtd2ul(4_294_967_297.5), 1);
    assert_eq!(cvtd2ul(-2.5), 0xffff_fffe);
    assert_eq!(cvtd2ul(1e19), 0);
    assert_eq!(cvtd2ul(f64::NAN), 0);
}

/// `GetModelPath`'s component: by form type, the model's text, none for an empty one or a type
/// without a model.
#[test]
fn model_paths() {
    let e = world(true);
    // Type 0x28 (a weapon: model at word 0x0F, its text at +0x40).
    assert_eq!((BASE_COMPONENT_OFFSETS[0x28] >> 24) as i8, 0x0f);
    e.mem.set_u8(BASE + 4, 0x28);
    let text = AREA + 0x9800;
    e.mem.write(text, b"Weapons\\gun.nif\0");
    e.mem.set_u32(BASE + 0x40, text);
    assert_eq!(form_model_path(&e, BASE), text);
    e.mem.set_u8(text, 0);
    assert_eq!(form_model_path(&e, BASE), 0, "empty");
    e.mem.set_u8(BASE + 4, 0x00);
    assert_eq!(form_model_path(&e, BASE), 0, "no model");
}

/// `HasPerk`'s rank without the NPC perks patch: the player's for the player and a teammate (as
/// for teammates), none for another actor or a non-actor.
#[test]
fn perk_ranks() {
    let mut e = world(true);
    let player = AREA + 0xa000;
    let player_vt = VTABLES + 0x600;
    let rank_at = 0x0700_1000;
    form(&mut e, player, PLAYER_ID, 0x3b, player_vt, &[VT_IS_ACTOR]);
    e.mem.set_u32(player_vt + VT_GET_PERK_RANK, rank_at);
    e.register(rank_at, |_, a| {
        log(format!("rank {:x} {:x} {}", a[0], a[1], a[2]));
        Ret {
            eax: 0x102,
            ..Ret::default()
        }
    });
    e.mem.set_u32(PLAYER, player);
    let perk = 0x0500_9f00;
    assert_eq!(
        perk_rank(&mut e, player, perk, false),
        2,
        "a rank's low byte"
    );
    assert_eq!(calls(), [format!("rank {player:x} {perk:x} 0")]);
    let npc = AREA + 0xa100;
    form(&mut e, npc, 0x0100_0050, 0x2b, player_vt, &[VT_IS_ACTOR]);
    assert_eq!(perk_rank(&mut e, npc, perk, false), 0, "not a teammate");
    e.mem.set_u8(npc + 0x18d, 1);
    assert_eq!(perk_rank(&mut e, npc, perk, false), 2);
    assert_eq!(
        calls(),
        [format!("rank {player:x} {perk:x} 1")],
        "the player's, for teammates"
    );
    let thing = AREA + 0xa200;
    form(&mut e, thing, 0x0100_0051, 0x1f, VTABLES + 0x700, &[]);
    assert_eq!(perk_rank(&mut e, thing, perk, false), 0);
}

/// A handler's eight words with `this` and `containing`, the result at the test result.
fn words(this: u32, containing: u32) -> [u32; 8] {
    use crate::test_support::RESULT;
    [0, 0, this, containing, 0, 0, RESULT, 0]
}

fn result(e: &Engine) -> f64 {
    e.mem.f64(crate::test_support::RESULT)
}

/// `GetContainer`: the container the script runs in (its id in the result's low word); without
/// one, and with no inventory reference for the calling reference, the result left as it was.
#[test]
fn containers() {
    let mut e = world(true);
    form(&mut e, REFR, 0x0100_0011, 0x3a, VTABLES, &[VT_IS_REFERENCE]);
    form(&mut e, BASE, 0x0100_0077, 0x3a, VTABLES, &[VT_IS_REFERENCE]);
    e.mem.set_f64(crate::test_support::RESULT, 0.0);
    get_container(&mut e, &words(REFR, BASE));
    assert_eq!(e.mem.u32(crate::test_support::RESULT), 0x0100_0077);
    e.mem.set_f64(crate::test_support::RESULT, 0.0);
    get_container(&mut e, &words(REFR, 0));
    assert_eq!(result(&e), 0.0, "no inventory reference: left");
}

/// `GetHitLocation`: the actor's process's hit data's location; -1 without a process, hit data
/// or for a non-actor.
#[test]
fn hit_locations() {
    let mut e = world(true);
    let process = AREA + 0xb000;
    let process_vt = VTABLES + 0x800;
    let hit = AREA + 0xb100;
    let get_hit = 0x0700_2000;
    form(&mut e, REFR, 0x0100_0011, 0x3b, VTABLES, &[VT_IS_ACTOR]);
    e.mem.set_u32(process, process_vt);
    e.mem.set_u32(process_vt + VT_GET_HIT_DATA, get_hit);
    e.register(get_hit, |_, _| Ret {
        eax: AREA + 0xb100,
        ..Ret::default()
    });
    e.mem.set_i32(hit + 0x60, 3);
    get_hit_location(&mut e, &words(REFR, 0));
    assert_eq!(result(&e), -1.0, "no process");
    e.mem.set_u32(REFR + 0x68, process);
    get_hit_location(&mut e, &words(REFR, 0));
    assert_eq!(result(&e), 3.0);
    let thing = AREA + 0xb200;
    form(&mut e, thing, 0x0100_0012, 0x1f, VTABLES + 0x900, &[]);
    get_hit_location(&mut e, &words(thing, 0));
    assert_eq!(result(&e), -1.0, "not an actor");
}

/// The evals: `MenuMode` writes 1 only when true (the condition code's 0 stays otherwise);
/// `IsInList` writes 0 or 1; `IsRefInList` the index.
#[test]
fn evals() {
    let mut e = world(true);
    let res = crate::test_support::RESULT;
    e.mem.set_u32(MANAGER + 0xc, 1);
    e.mem.set_f64(res, 0.5);
    menu_mode_eval(&mut e, &[0, 0, 0, res]);
    assert_eq!(result(&e), 0.5, "false: left");
    e.mem.set_u32(MANAGER + 0xc, 2);
    menu_mode_eval(&mut e, &[0, 0, 0, res]);
    assert_eq!(result(&e), 1.0);
    let (a, b) = (AREA + 0x9000, AREA + 0x9100);
    let fl = AREA + 0x9200;
    list_form(&mut e, fl, &[a, b]);
    form(&mut e, REFR, 0x0100_0011, 0x3a, VTABLES, &[VT_IS_REFERENCE]);
    e.mem.set_u32(REFR + 0x20, a);
    e.mem.set_u32(a + 0xc, 0x0100_0020);
    e.mem.set_f64(res, 0.5);
    is_in_list_eval(&mut e, &[REFR, fl, 0, res]);
    assert_eq!(result(&e), 1.0);
    e.mem.set_u32(REFR + 0x20, AREA + 0x9300);
    is_in_list_eval(&mut e, &[REFR, fl, 0, res]);
    assert_eq!(result(&e), 0.0);
    is_ref_in_list_eval(&mut e, &[0, fl, b, res]);
    assert_eq!(result(&e), 1.0);
}

// ---- through the commands' records ----

thread_local! {
    static LINES: RefCell<Vec<(String, Vec<u32>)>> = const { RefCell::new(Vec::new()) };
}

fn lines() -> Vec<(String, Vec<u32>)> {
    LINES.with(|l| std::mem::take(&mut *l.borrow_mut()))
}

const CONSOLE: u32 = 0x0500_f000;

/// The game's records of the seven vanilla commands as the exe has them (read from
/// FalloutNV.exe's table at `0x01190910`): names, parameter lists (`MenuMode`: an optional
/// Integer; `GetItemCount`: an ObjectID, type 0x32; `IsInList`: a Form List, 0x25; `HasPerk`: a
/// Perk, 0x27, and an optional Integer `Alt`; `GetContainer`, `GetHitLocation`,
/// `ClearTrackedCells` at 0x127D: none). The stand-in exe table has empty records.
/// A record's opcode, name and parameters (type, optional).
type VanillaRecord = (u32, &'static str, &'static [(u32, u32)]);

fn vanilla_records(e: &mut Engine) {
    let lists: [VanillaRecord; 7] = [
        (0x1024, "MenuMode", &[(0x01, 1)]),
        (0x102f, "GetItemCount", &[(0x32, 0)]),
        (0x10cf, "GetContainer", &[]),
        (0x1174, "IsInList", &[(0x25, 0)]),
        (0x1187, "GetHitLocation", &[]),
        (0x11c1, "HasPerk", &[(0x27, 0), (0x01, 1)]),
        (0x127d, "ClearTrackedCells", &[]),
    ];
    for (i, (opcode, name, params)) in lists.iter().enumerate() {
        let at = AREA + 0xd000 + 0x80 * i as u32;
        e.mem.write(at + 0x40, name.as_bytes());
        e.mem.set_u8(at + 0x40 + name.len() as u32, 0);
        for (j, (kind, optional)) in params.iter().enumerate() {
            let p = at + 12 * j as u32;
            e.mem.set_u32(p + 4, *kind);
            e.mem.set_u32(p + 8, *optional);
        }
        let record = e.script_tables().record(*opcode);
        e.mem.set_u32(record, at + 0x40);
        e.mem.set_u16(record + 0x12, params.len() as u16);
        e.mem
            .set_u32(record + 0x14, if params.is_empty() { 0 } else { at });
    }
}

/// JIP answering "installed" with the row on, the vanilla records filled first, the console's
/// print kept, a stand-in `EvaluateLine` (the echo prints only for a line of its own).
fn commands_world() -> Engine {
    LINES.with(|l| l.borrow_mut().clear());
    calls();
    let mut e = engine();
    e.map(AREA, 0x1_0000);
    for page in [
        0x011d_8000,
        0x011f_3000,
        0x011c_a000,
        0x011c_8000,
        0x011f_9000,
        0x011c_c000,
        0x011c_3000,
    ] {
        e.map(page, 0x1000);
    }
    e.mem.set_u32(INTERFACE_MANAGER, MANAGER);
    vanilla_records(&mut e);
    set_vanilla_commands(&mut e, true);
    crate::install_as(&mut e, true);
    e.register(0x0071_b160, |_, _| Ret {
        eax: CONSOLE,
        ..Ret::default()
    });
    e.register(0x0071_d0a0, |e, a| {
        let format = String::from_utf8_lossy(&nvse::table::guest_text(e, a[1])).into_owned();
        let words = if a[2] == a[1] {
            Vec::new()
        } else {
            vec![e.mem.u32(a[2]), e.mem.u32(a[2] + 4)]
        };
        LINES.with(|l| l.borrow_mut().push((format, words)));
        Ret::default()
    });
    e.register(0x005e_1550, |e, w| e.call(w[0], &w[1..9]));
    e
}

/// Script reference `index` (from 1) is `form`: the reference list at the script's `+0x44`, each
/// entry's form at `+8` (as `nvse::args` reads an `r` argument).
fn set_script_ref(e: &mut Engine, index: u16, form: u32) {
    let script = crate::test_support::SCRIPT;
    let i = u32::from(index);
    let ref_var = 0x0300_a000 + i * 0x10;
    e.mem.set_u32(ref_var + 8, form);
    e.mem.set_u32(ref_var + 0xc, 0);
    let mut node = script + 0x44;
    for at in 1..i {
        let next = e.mem.u32(node + 4);
        node = if next == 0 {
            let made = 0x0300_b000 + (at + 1) * 8;
            e.mem.set_u32(node + 4, made);
            made
        } else {
            next
        };
    }
    e.mem.set_u32(node, ref_var);
}

fn form_arg(index: u16) -> Vec<u8> {
    let mut b = vec![b'r'];
    b.extend_from_slice(&index.to_le_bytes());
    b
}

fn int_arg(v: i32) -> Vec<u8> {
    let mut b = vec![b'n'];
    b.extend_from_slice(&v.to_le_bytes());
    b
}

fn double_arg(v: f64) -> Vec<u8> {
    let mut b = vec![b'z'];
    b.extend_from_slice(&v.to_le_bytes());
    b
}

/// Runs `opcode` through its record's execute on `this`, as a line of its own, in the console
/// when `console` (the echo's condition): the handler's `al` and the result (`start` first).
fn run(
    e: &mut Engine,
    opcode: u32,
    this: u32,
    args: &[Vec<u8>],
    console: bool,
    start: f64,
) -> (u32, f64) {
    crate::test_support::console(e, console);
    let mut words = crate::test_support::line(e, opcode, args);
    words[2] = this;
    e.mem.set_f64(crate::test_support::RESULT, start);
    let execute = crate::test_support::execute_of(e, opcode);
    let mut w = vec![execute];
    w.extend_from_slice(&words);
    let ok = e.call(0x005e_1550, &w).eax & 0xff;
    crate::test_support::console(e, false);
    (ok, e.mem.f64(crate::test_support::RESULT))
}

/// `MenuMode` through its record: the mode asked (no argument: 0), the result 1 or left; the
/// console line `MenuMode %d >> %d`.
#[test]
fn menu_mode_command() {
    let mut e = commands_world();
    e.mem.set_u32(MANAGER + 0xc, 2);
    assert_eq!(
        run(&mut e, 0x1024, 0, &[], false, 0.0),
        (1, 1.0),
        "no argument: 0"
    );
    e.mem.set_u32(MANAGER + 0xc, 1);
    assert_eq!(
        run(&mut e, 0x1024, 0, &[], false, 0.5),
        (1, 0.5),
        "false: left"
    );
    // In the console (its open byte set): mode 3 is true.
    assert_eq!(run(&mut e, 0x1024, 0, &[int_arg(3)], true, 0.0), (1, 1.0));
    assert_eq!(lines(), [("MenuMode %d >> %d".to_string(), vec![3, 1])]);
}

/// `GetItemCount` through its record: the count; a form the parameter refuses (not an inventory
/// type) leaves the result; the console echo; the eval.
#[test]
fn get_item_count_command() {
    let mut e = commands_world();
    let apple = AREA + 0x9000;
    form(&mut e, apple, 0x0100_0030, 0x2f, VTABLES + 0xb00, &[]);
    container_world(&mut e, &[(2, apple)]);
    e.register(GET_BY_TYPE, |_, _| Ret::default());
    set_script_ref(&mut e, 1, apple);
    assert_eq!(
        run(&mut e, 0x102f, REFR, &[form_arg(1)], true, 0.0),
        (1, 2.0)
    );
    assert_eq!(lines(), [("GetItemCount >> 2".to_string(), vec![])]);
    let race = AREA + 0x9500;
    form(&mut e, race, 0x0100_0031, 0x0c, VTABLES + 0xb00, &[]);
    set_script_ref(&mut e, 2, race);
    assert_eq!(
        run(&mut e, 0x102f, REFR, &[form_arg(2)], false, 0.5),
        (1, 0.5),
        "refused: left"
    );
    let res = crate::test_support::RESULT;
    get_item_count_eval(&mut e, &[REFR, apple, 0, res]);
    assert_eq!(result(&e), 2.0);
}

/// `GetContainer` through its record: the container the script runs in; an inventory
/// reference's container (`nvse::invref`); neither: left.
#[test]
fn get_container_command() {
    let mut e = commands_world();
    form(&mut e, REFR, 0x0100_0011, 0x3a, VTABLES, &[VT_IS_REFERENCE]);
    form(&mut e, BASE, 0x0100_0077, 0x3a, VTABLES, &[VT_IS_REFERENCE]);
    let mut words = crate::test_support::line(&mut e, 0x10cf, &[]);
    words[2] = REFR;
    words[3] = BASE;
    let execute = crate::test_support::execute_of(&e, 0x10cf);
    e.mem.set_f64(crate::test_support::RESULT, 0.0);
    e.call(execute, &words);
    assert_eq!(
        e.mem.u32(crate::test_support::RESULT),
        0x0100_0077,
        "the containing reference"
    );
    assert_eq!(
        run(&mut e, 0x10cf, REFR, &[], false, 0.0),
        (1, 0.0),
        "neither: left"
    );
    // An inventory reference: its temporary reference (the game's allocator and constructor,
    // stood in) stands for a stack in BASE.
    let temp = AREA + 0xe000;
    e.register(0x0040_1000, |_, _| Ret {
        eax: AREA + 0xe000,
        ..Ret::default()
    });
    e.register(0x0055_a2f0, |e, a| {
        e.mem.set_u32(a[0] + 0xc, 0xff00_0100);
        Ret::default()
    });
    let refr = nvse::invref::create(&mut e, BASE, nvse::invref::Data::default(), false);
    assert_eq!(refr, temp);
    assert_eq!(run(&mut e, 0x10cf, temp, &[], false, 0.0).0, 1);
    assert_eq!(
        e.mem.u32(crate::test_support::RESULT),
        0x0100_0077,
        "the stack's container"
    );
}

/// `IsInList` through its record: 1 for the reference's base form in the list, 0 otherwise; a
/// form the parameter refuses (not a form list) leaves the result; the console echo.
#[test]
fn is_in_list_command() {
    let mut e = commands_world();
    let (a, b) = (AREA + 0x9000, AREA + 0x9100);
    let fl = AREA + 0x9200;
    list_form(&mut e, fl, &[a, b]);
    e.mem.set_u32(fl + 0xc, 0x0100_0040);
    form(&mut e, REFR, 0x0100_0011, 0x3a, VTABLES, &[VT_IS_REFERENCE]);
    e.mem.set_u32(REFR + 0x20, b);
    e.mem.set_u32(b + 0xc, 0x0100_0020);
    set_script_ref(&mut e, 1, fl);
    assert_eq!(
        run(&mut e, 0x1174, REFR, &[form_arg(1)], true, 0.0),
        (1, 1.0)
    );
    assert_eq!(lines(), [("IsInList >> 1".to_string(), vec![])]);
    e.mem.set_u32(REFR + 0x20, AREA + 0x9300);
    assert_eq!(
        run(&mut e, 0x1174, REFR, &[form_arg(1)], false, 0.5),
        (1, 0.5)
    );
    set_script_ref(&mut e, 2, b);
    e.mem.set_u8(b + 4, 0x28);
    assert_eq!(
        run(&mut e, 0x1174, REFR, &[form_arg(2)], false, 0.5),
        (1, 0.5),
        "refused: left"
    );
}

/// `GetHitLocation` through its record: the hit data's word; -1 without a process.
#[test]
fn get_hit_location_command() {
    let mut e = commands_world();
    form(&mut e, REFR, 0x0100_0011, 0x3b, VTABLES, &[VT_IS_ACTOR]);
    assert_eq!(run(&mut e, 0x1187, REFR, &[], false, 0.0), (1, -1.0));
    let process = AREA + 0xb000;
    e.mem.set_u32(process, VTABLES + 0x800);
    e.mem
        .set_u32(VTABLES + 0x800 + VT_GET_HIT_DATA, 0x0700_2000);
    e.register(0x0700_2000, |_, _| Ret {
        eax: AREA + 0xb100,
        ..Ret::default()
    });
    e.mem.set_i32(AREA + 0xb100 + 0x60, 7);
    e.mem.set_u32(REFR + 0x68, process);
    assert_eq!(run(&mut e, 0x1187, REFR, &[], false, 0.0), (1, 7.0));
}

/// The player at `PLAYER`, whose `GetPerkRank` (slot `+0x4A0`, `(perk, alt)`, `AL`: the exe's
/// `LevelUpMenu` calls it so) logs its arguments and answers rank 2; an actor whose own answers 1.
fn perk_world(e: &mut Engine) -> (u32, u32) {
    let player = AREA + 0xa000;
    let npc = AREA + 0xa100;
    let (player_vt, npc_vt) = (VTABLES + 0x600, VTABLES + 0xc00);
    form(e, player, PLAYER_ID, 0x3b, player_vt, &[VT_IS_ACTOR]);
    form(e, npc, 0x0100_0050, 0x2b, npc_vt, &[VT_IS_ACTOR]);
    e.mem.set_u32(player_vt + VT_GET_PERK_RANK, 0x0700_1000);
    e.mem.set_u32(npc_vt + VT_GET_PERK_RANK, 0x0700_1100);
    e.register(0x0700_1000, |_, a| {
        log(format!("player {:x} {}", a[1], a[2]));
        Ret {
            eax: 0x102,
            ..Ret::default()
        }
    });
    e.register(0x0700_1100, |_, a| {
        log(format!("own {:x} {}", a[1], a[2]));
        Ret {
            eax: 1,
            ..Ret::default()
        }
    });
    e.mem.set_u32(PLAYER, player);
    (player, npc)
}

/// `HasPerk`: the player's rank for the player and for a teammate, `alt` or-ed with the teammate
/// flag both ways; with the NPC perks patch the actor's own; through its record the result and
/// the game's console line (its format at `0x01035F1C`, `Perk Rank >> %d` in the exe); the eval.
#[test]
fn has_perk_command() {
    let mut e = commands_world();
    let (player, npc) = perk_world(&mut e);
    let perk = AREA + 0x9600;
    form(&mut e, perk, 0x0100_0060, 0x56, VTABLES + 0xd00, &[]);
    calls();
    assert_eq!(perk_rank(&mut e, player, perk, true), 2);
    e.mem.set_u8(npc + 0x18d, 1);
    assert_eq!(perk_rank(&mut e, npc, perk, false), 2);
    assert_eq!(perk_rank(&mut e, player, perk, false), 2);
    assert_eq!(
        calls(),
        [
            format!("player {perk:x} 1"),
            format!("player {perk:x} 1"),
            format!("player {perk:x} 0")
        ]
    );
    e.ext::<Jip>().saved.npc_perks_patch = true;
    assert_eq!(perk_rank(&mut e, npc, perk, false), 1);
    assert_eq!(calls(), [format!("own {perk:x} 0")]);
    e.ext::<Jip>().saved.npc_perks_patch = false;
    e.map(0x0103_5000, 0x1000);
    e.mem.write(0x0103_5f1c, b"Perk Rank >> %d\0");
    set_script_ref(&mut e, 1, perk);
    assert_eq!(
        run(&mut e, 0x11c1, player, &[form_arg(1)], true, 0.0),
        (1, 1.0)
    );
    assert_eq!(lines(), [("Perk Rank >> %d".to_string(), vec![2, 0])]);
    calls();
    let res = crate::test_support::RESULT;
    e.mem.set_f64(res, 0.0);
    has_perk_eval(&mut e, &[npc, perk, 0, res]);
    assert_eq!(result(&e), 1.0);
    e.mem.set_u8(npc + 0x18d, 0);
    e.mem.set_f64(res, 0.0);
    has_perk_eval(&mut e, &[npc, perk, 0, res]);
    assert_eq!(result(&e), 0.0, "not a teammate");
}

/// `GetNVSEVersionFull` at the game's `0x127D`: 6.49.
#[test]
fn nvse_version_at_0x127d() {
    let mut e = commands_world();
    assert_eq!(run(&mut e, 0x127d, 0, &[], false, 0.0), (1, 6.49));
}

/// The setting commands through their records: get by name, -1 for a string setting's name or an
/// unknown one, "SETTING NOT FOUND" in the console for an unknown one; set with a double (1), an
/// unknown one or a string setting's name leaving the result.
#[test]
fn setting_commands() {
    let mut e = commands_world();
    let s1 = SETTINGS;
    setting(&mut e, s1, b"fJumpHeight", 2.5f32.to_bits());
    let games = AREA + 0x7000;
    e.mem.set_u32(GAME_SETTINGS, games);
    e.mem.set_u32(games + 0x110, 1);
    e.mem.set_u32(games + 0x114, games + 0x200);
    e.mem.set_u32(games + 0x200, games + 0x300);
    e.mem.set_u32(games + 0x300, 0);
    e.mem.set_u32(games + 0x308, s1);
    for (i, at) in INI_COLLECTIONS.iter().enumerate() {
        let coll = AREA + 0x8000 + 0x200 * i as u32;
        e.mem.set_u32(*at, coll);
        e.mem.set_u32(coll + 0x10c, 0);
    }
    for op in [0x149a, 0x149c] {
        assert_eq!(
            run(&mut e, op, 0, &[s(b"fJumpHeight")], false, 0.0),
            (1, 2.5)
        );
        assert_eq!(run(&mut e, op, 0, &[s(b"sJump")], true, 0.0), (1, -1.0));
        assert!(lines().is_empty(), "a string setting's name: no line");
        assert_eq!(run(&mut e, op, 0, &[s(b"fNone")], true, 0.0), (1, -1.0));
        let formats: Vec<String> = lines().into_iter().map(|l| l.0).collect();
        assert_eq!(formats, ["SETTING NOT FOUND"]);
    }
    for (op, v) in [(0x149b, 4.0), (0x149d, 0.25)] {
        let args = [s(b"fjumpheight"), double_arg(v)];
        assert_eq!(run(&mut e, op, 0, &args, false, 0.0), (1, 1.0));
        assert_eq!(e.mem.f32(s1 + 4), v as f32);
        let args = [s(b"fNone"), double_arg(v)];
        assert_eq!(run(&mut e, op, 0, &args, false, 0.5), (1, 0.5));
        let args = [s(b"sJump"), double_arg(v)];
        assert_eq!(run(&mut e, op, 0, &args, false, 0.5), (1, 0.5));
    }
}

/// `IsRefInList` through its record: the index; a read that fails (no arguments): -1.
#[test]
fn is_ref_in_list_command() {
    let mut e = commands_world();
    let (a, b) = (AREA + 0x9000, AREA + 0x9100);
    let fl = AREA + 0x9200;
    list_form(&mut e, fl, &[a, b]);
    e.mem.set_u32(fl + 0xc, 0x0100_0040);
    form(&mut e, b, 0x0100_0020, 0x28, VTABLES + 0xe00, &[]);
    set_script_ref(&mut e, 1, fl);
    set_script_ref(&mut e, 2, b);
    let args = [form_arg(1), form_arg(2)];
    assert_eq!(run(&mut e, 0x1511, 0, &args, false, 0.0), (1, 1.0));
    assert_eq!(
        run(&mut e, 0x1511, 0, &[], false, 0.0),
        (1, -1.0),
        "no list: -1"
    );
}

/// The string a command answered (its id in the result).
fn result_text(e: &Engine) -> Vec<u8> {
    let id = result(e) as u32;
    nvse::vars::string(e, id).unwrap_or_default()
}

/// `GetModelPath` through its record: the calling reference's base form's path; a reference
/// given: its base form's; a base form given: its own; neither: empty. (A weapon's model, its
/// `TESModelTextureSwap`, is at `+0x3C`, its text at `+0x40`: engine-fallout-forms
/// `tesobjectweap.rs`, `tesmodel.rs`.)
#[test]
fn get_model_path_command() {
    let mut e = commands_world();
    let (weapon, other) = (AREA + 0x9700, AREA + 0x9800);
    for (at, id, text) in [
        (weapon, 0x0100_0070, b"gun.nif\0"),
        (other, 0x0100_0071, b"bat.nif\0"),
    ] {
        form(&mut e, at, id, 0x28, VTABLES + 0xf00, &[]);
        e.mem.write(at + 0x80, text);
        e.mem.set_u32(at + 0x40, at + 0x80);
    }
    form(&mut e, REFR, 0x0100_0011, 0x3a, VTABLES, &[VT_IS_REFERENCE]);
    e.mem.set_u32(REFR + 0x20, weapon);
    let refr2 = AREA + 0x9900;
    form(
        &mut e,
        refr2,
        0x0100_0012,
        0x3a,
        VTABLES,
        &[VT_IS_REFERENCE],
    );
    e.mem.set_u32(refr2 + 0x20, other);
    run(&mut e, 0x158e, REFR, &[], false, 0.0);
    assert_eq!(result_text(&e), b"gun.nif", "the calling reference's base");
    set_script_ref(&mut e, 1, refr2);
    run(&mut e, 0x158e, REFR, &[form_arg(1)], false, 0.0);
    assert_eq!(result_text(&e), b"bat.nif", "a reference given: its base");
    set_script_ref(&mut e, 2, weapon);
    run(&mut e, 0x158e, 0, &[form_arg(2)], false, 0.0);
    assert_eq!(result_text(&e), b"gun.nif", "a base form given");
    run(&mut e, 0x158e, 0, &[], false, 0.0);
    assert_eq!(result_text(&e), b"", "neither: empty");
}

/// The whole records (0x28 bytes and the parameter list) of every touched opcode while JIP does
/// not answer "installed", row on and off (given before and after JIP's start), against xNVSE's
/// table alone; with JIP answering "installed" but the row off, the seven vanilla records
/// unchanged.
#[test]
fn records_untouched_when_off() {
    let snapshot = |e: &Engine| -> Vec<Vec<u8>> {
        XNVSE
            .iter()
            .chain(VANILLA)
            .map(|o| {
                let record = e.script_tables().record(o.opcode);
                let mut bytes = e.mem.bytes(record, 0x28);
                let (count, params) = (e.mem.u16(record + 0x12), e.mem.u32(record + 0x14));
                if params != 0 {
                    bytes.extend(e.mem.bytes(params, 12 * u32::from(count)));
                }
                bytes
            })
            .collect()
    };
    let mut plain = engine();
    plain.map(AREA, 0x1_0000);
    vanilla_records(&mut plain);
    let want = snapshot(&plain);
    for row in [false, true] {
        let mut e = engine();
        e.map(AREA, 0x1_0000);
        vanilla_records(&mut e);
        set_vanilla_commands(&mut e, row);
        crate::install_as(&mut e, false);
        set_vanilla_commands(&mut e, row);
        assert_eq!(snapshot(&e), want, "not installed, row {row}");
    }
    let mut e = engine();
    e.map(AREA, 0x1_0000);
    vanilla_records(&mut e);
    crate::install_as(&mut e, true);
    let got = snapshot(&e);
    assert_eq!(
        got[XNVSE.len()..],
        want[XNVSE.len()..],
        "installed, row off: vanilla untouched"
    );
}
