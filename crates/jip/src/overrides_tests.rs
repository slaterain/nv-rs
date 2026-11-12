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

/// `GetItemCount`: the base container's entries summed with the reference's changes entry (none
/// below 0), the entry alone when it holds a leveled item or the base has none, a form list's
/// forms summed, 0 for a base without a container.
#[test]
fn item_counts() {
    let mut e = world(true);
    let (apple, pear) = (AREA + 0x9000, AREA + 0x9100);
    // A container base (type 0x1B) with apple x2 and apple x3, pear x1.
    form(&mut e, BASE, 0x0100_0010, CONTAINER, VTABLES + 0x200, &[]);
    form(&mut e, REFR, 0x0100_0011, 0x3a, VTABLES, &[VT_IS_REFERENCE]);
    e.mem.set_u32(REFR + 0x20, BASE);
    let objects = [(2, apple), (3, apple), (1, pear)];
    let list = BASE + 0x30 + 4;
    for (i, (count, item)) in objects.iter().enumerate() {
        let object = LISTS + 0x100 + 8 * i as u32;
        e.mem.set_u32(object, *count as u32);
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
    // No changes yet.
    e.register(GET_BY_TYPE, |_, _| Ret::default());
    assert_eq!(item_count(&mut e, REFR, apple), 5);
    assert_eq!(item_count(&mut e, REFR, 0), 0, "no form");
    // Changes: apple -7 (5 - 7 = -2: 0), pear +2 with a leveled item (2 alone).
    let changes_extra = LISTS + 0x400;
    let data = LISTS + 0x440;
    let entries = [(apple, -7i32, 0u32), (pear, 2, LISTS + 0x500)];
    e.mem.set_u32(changes_extra + 0xc, data);
    e.mem.set_u32(data, LISTS + 0x480);
    for (i, (item, delta, lists)) in entries.iter().enumerate() {
        let entry = LISTS + 0x4c0 + 0x10 * i as u32;
        e.mem.set_u32(entry, *lists);
        e.mem.set_i32(entry + 4, *delta);
        e.mem.set_u32(entry + 8, *item);
        let node = LISTS + 0x480 + 8 * i as u32;
        e.mem.set_u32(node, entry);
        let next = if i == 0 { node + 8 } else { 0 };
        e.mem.set_u32(node + 4, next);
    }
    // The pear's extra data lists: one with the leveled item bit (type 0x2F).
    let xlist = LISTS + 0x600;
    e.mem.set_u32(LISTS + 0x500, LISTS + 0x508);
    e.mem.set_u32(LISTS + 0x508, xlist);
    e.mem.set_u32(LISTS + 0x50c, 0);
    e.mem.set_u8(xlist + 0xd, 0x80);
    e.register(GET_BY_TYPE, |_, a| Ret {
        eax: if a[1] == EXTRA_CONTAINER_CHANGES {
            LISTS + 0x400
        } else {
            0
        },
        ..Ret::default()
    });
    assert_eq!(item_count(&mut e, REFR, apple), 0, "none below 0");
    assert_eq!(item_count(&mut e, REFR, pear), 2, "the leveled entry alone");
    e.mem.set_u8(xlist + 0xd, 0);
    assert_eq!(item_count(&mut e, REFR, pear), 3, "added to the base's");
    // A form list of both.
    let fl = AREA + 0x9200;
    list_form(&mut e, fl, &[apple, 0, pear]);
    assert_eq!(item_count(&mut e, REFR, fl), 3);
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
