//! Tests of JIP's world hooks: nothing put in while JIP answers "not installed"; installed, each
//! seam's handler as JIP's patched code decides, the commands through their records with the
//! hooks' user counts, the forms' flag bytes and JIP's maps, and the pre-load housekeeping.
//! Layouts at literal offsets, not the code's: a form's vtable `+0`, type `+4`, JIP's flag bytes
//! `+5`/`+6`, id `+0xC`; a reference's base `+0x20`, position `+0x30`, extra list `+0x44`, JIP's
//! byte `+0x5F`, teammate byte `+0x18D`; a script's id `+0xC`, data length `+0x20`, quest and
//! effect bytes `+0x28`/`+0x29`, quest `+0x40`; an extra script's script `+0xC` and event list
//! `+0x10`; a lock extra's data `+0xC` and its flag byte `+8`; a quest's stage list `+0x44` and
//! flags `+0x3C`; a stage's id `+0` and done byte `+1`; a runner's `if` stack `+0x20` (12
//! words); the player's target actor `+0xD44`; a note's text `+0x6C`.

use super::*;
use crate::test_support::{
    engine, form_arg, int_arg, line, s, script_ref, with_jips_slot_held as held, RESULT,
    SCRIPT as CALLER,
};

const AREA: u32 = 0x0500_0000;
const REF: u32 = AREA + 0x1000;
const BASE: u32 = AREA + 0x1200;
const ITEM: u32 = AREA + 0x1400;
const ITEM_BASE: u32 = AREA + 0x1600;
const QUEST_FORM: u32 = AREA + 0x1800;
const STAGE: u32 = AREA + 0x1900;
const SCRIPT_ONE: u32 = AREA + 0x2000;
const SCRIPT_TWO: u32 = AREA + 0x2100;
const TOPIC_FORM: u32 = AREA + 0x2200;
const INFO: u32 = AREA + 0x2300;
const WORLD: u32 = AREA + 0x2400;
const ARMOR_FORM: u32 = AREA + 0x2500;
const WEAPON_FORM: u32 = AREA + 0x2600;
const LIST_FORM_F: u32 = AREA + 0x2700;
const NODES: u32 = AREA + 0x2780;
const XSCRIPT: u32 = AREA + 0x2800;
const OBJ_SCRIPT: u32 = AREA + 0x2900;
const LOCK: u32 = AREA + 0x2a00;
const LOCK_DATA: u32 = AREA + 0x2b00;
const RUNNER: u32 = AREA + 0x3000;
const LENGTH: u32 = AREA + 0x3100;
const PLAYER: u32 = AREA + 0x3200;
const NOTE_FORM: u32 = AREA + 0x4000;
const DESC: u32 = AREA + 0x4100;
const BUFFER: u32 = AREA + 0x5000;
const STRINGS: u32 = AREA + 0x6000;
const EXTRAS: u32 = AREA + 0x8000;
/// The game's map of forms by id (`0x011C54C0` points at it): one bucket.
const FORM_MAP: u32 = AREA + 0xd000;
const MAP_ENTRIES: u32 = AREA + 0xd100;
const HEAP: u32 = AREA + 0xe000;
/// The vtables: a reference's (`IsReference` true; at a game vtable's address, its low byte's
/// bit 2 set, as JIP's `IsValidReference` wants), a plain form's (`IsBoundObject` true).
const REF_VT: u32 = 0x0108_9f04;
const FORM_VT: u32 = AREA + 0xa00;

thread_local! {
    static CALLS: std::cell::RefCell<Vec<(u32, Vec<u32>)>> = const { std::cell::RefCell::new(Vec::new()) };
}

fn calls() -> Vec<(u32, Vec<u32>)> {
    CALLS.with(|c| std::mem::take(&mut *c.borrow_mut()))
}

/// Extras found by `GetByType` (stand-in): (extra list, type) -> extra.
fn set_extra(e: &mut Engine, list: u32, kind: u32, extra: u32) {
    e.mem.set_u32(extra_slot(list, kind), extra);
}

fn extra_slot(list: u32, kind: u32) -> u32 {
    EXTRAS + (list - AREA) * 2 + kind * 4
}

fn form(e: &mut Engine, at: u32, kind: u8, id: u32, reference: bool) {
    e.mem.set_u32(at, if reference { REF_VT } else { FORM_VT });
    e.mem.set_u8(at + 4, kind);
    e.mem.set_u32(at + 0xc, id);
}

fn world_as(installed: bool) -> Engine {
    calls();
    crate::script_call::calls();
    let mut e = engine();
    e.map(AREA, 0x1_0000);
    e.map(0x011c_5000, 0x1000);
    e.mem.set_u32(REF_VT + 0xf0, 0x008d_0360);
    e.mem.set_u32(FORM_VT + 0xe8, 0x00de_00e8);
    e.register(0x00de_00e8, |_, _| Ret {
        eax: 1,
        ..Ret::default()
    });
    form(&mut e, REF, 0x3a, 0x0100_0001, true);
    form(&mut e, BASE, 0x15, 0x0100_0002, false);
    e.mem.set_u32(REF + 0x20, BASE);
    form(&mut e, ITEM, 0x3a, 0x0100_0003, true);
    form(&mut e, ITEM_BASE, 0x18, 0x0100_0004, false);
    e.mem.set_u32(ITEM + 0x20, ITEM_BASE);
    form(&mut e, QUEST_FORM, QUEST, 0x0100_0005, false);
    for (script, id) in [(SCRIPT_ONE, 0x0100_0006), (SCRIPT_TWO, 0x0100_0007)] {
        form(&mut e, script, SCRIPT, id, false);
    }
    form(&mut e, TOPIC_FORM, TOPIC, 0x0100_0008, false);
    form(&mut e, INFO, TOPIC_INFO, 0x0100_0009, false);
    form(&mut e, WORLD, WORLD_SPACE, 0x0100_000a, false);
    form(&mut e, ARMOR_FORM, ARMOR, 0x0100_000b, false);
    form(&mut e, WEAPON_FORM, WEAPON, 0x0100_000c, false);
    form(&mut e, LIST_FORM_F, LIST_FORM, 0x0100_000d, false);
    e.mem.set_u32(LIST_FORM_F + 0x18, ARMOR_FORM);
    e.mem.set_u32(LIST_FORM_F + 0x1c, NODES);
    e.mem.set_u32(NODES, WEAPON_FORM);
    form(&mut e, NOTE_FORM, NOTE, 0x0100_000e, false);
    e.mem.set_u32(NOTE_FORM + 0x6c, DESC);
    // The quest's stage list: one stage, 10.
    e.mem.set_u8(STAGE, 10);
    e.mem.set_u32(QUEST_FORM + 0x44, STAGE);
    e.register(GET_BY_TYPE, |e, w| Ret {
        eax: e.mem.u32(extra_slot(w[0], w[1])),
        ..Ret::default()
    });
    e.register(GET_OWNER_TOPIC, |_, w| Ret {
        eax: if w[0] == INFO { TOPIC_FORM } else { 0 },
        ..Ret::default()
    });
    // `__RTDynamicCast`: no form here has a `TESDescription`.
    e.register(0x00ec_43fb, |_, _| Ret::default());
    // The game's heap (`MemoryManager::Allocate`, `::Free`): a bump allocator.
    e.mem.set_u32(HEAP, HEAP + 0x10);
    e.register(0x00aa_3e40, |e, w| {
        let at = e.mem.u32(HEAP);
        e.mem.set_u32(HEAP, at + w[1].div_ceil(4) * 4);
        Ret {
            eax: at,
            ..Ret::default()
        }
    });
    e.register(0x00aa_4060, |_, _| Ret::default());
    e.mem.set_u32(0x011c_54c0, FORM_MAP);
    e.mem.set_u32(FORM_MAP + 4, 1);
    e.mem.set_u32(FORM_MAP + 8, MAP_ENTRIES);
    for (i, f) in [REF, QUEST_FORM].into_iter().enumerate() {
        let entry = MAP_ENTRIES + 0x10 + 0x10 * i as u32;
        e.mem.set_u32(entry, e.mem.u32(MAP_ENTRIES));
        e.mem.set_u32(entry + 4, e.mem.u32(f + 0xc));
        e.mem.set_u32(entry + 8, f);
        e.mem.set_u32(MAP_ENTRIES, entry);
    }
    crate::install_as(&mut e, installed);
    for (i, f) in [
        REF,
        SCRIPT_ONE,
        SCRIPT_TWO,
        QUEST_FORM,
        TOPIC_FORM,
        INFO,
        WORLD,
        ARMOR_FORM,
        WEAPON_FORM,
        LIST_FORM_F,
        NOTE_FORM,
        ITEM,
        BASE,
    ]
    .into_iter()
    .enumerate()
    {
        script_ref(&mut e, i as u16 + 1, f);
    }
    e
}

fn world() -> Engine {
    world_as(true)
}

/// Form argument indices of `world`.
const R_REF: u16 = 1;
const R_S1: u16 = 2;
const R_S2: u16 = 3;
const R_QUEST: u16 = 4;
const R_TOPIC: u16 = 5;
const R_INFO: u16 = 6;
const R_WORLD: u16 = 7;
const R_ARMOR: u16 = 8;
const R_LIST: u16 = 10;
const R_NOTE: u16 = 11;
const R_BASE: u16 = 13;

/// Runs `opcode` with `args` through its record on `this`; the result.
fn run(e: &mut Engine, opcode: u32, this: u32, args: &[Vec<u8>]) -> f64 {
    let mut words = line(e, opcode, args);
    words[2] = this;
    let at = crate::test_support::execute_of(e, opcode);
    assert_eq!(e.call(at, &words).eax & 0xff, 1);
    e.mem.f64(RESULT)
}

fn site(e: &mut Engine, addr: u32, args: &[u32]) -> Option<u32> {
    e.site(addr, args)
}

fn users(e: &Engine, hook: Hook) -> u32 {
    e.hook_users(hook)
}

/// While JIP answers "not installed": no seam has a handler of its, the commands have no handler.
#[test]
fn not_installed_nothing_is_put_in() {
    let e = world_as(false);
    for addr in SITES {
        assert!(e.site_owners(addr).is_empty(), "{addr:08x}");
    }
    for op in COMMANDS {
        assert_eq!(crate::test_support::execute_of(&e, op), 0, "{op:04x}");
    }
}

/// A command adds users only while every site of its hooks has a JIP seam, and every handler
/// here is the only one at its seam.
#[test]
fn every_command_hook_has_all_its_seams() {
    let e = world();
    for addr in SITES {
        assert_eq!(e.site_owners(addr), [Owner::Jip], "{addr:08x}");
    }
    for (op, sites) in COMMAND_SITES {
        assert_ne!(crate::test_support::execute_of(&e, op), 0, "{op:04x}");
        for &addr in sites {
            assert!(
                engine_core::extend::SITES
                    .iter()
                    .any(|s| s.addr == addr && s.owners.iter().any(|o| matches!(o, Owner::Jip))),
                "{op:04x}: {addr:08x} has no JIP seam"
            );
        }
    }
}

/// SetFormDescription and the GetDescription seam: a description's own text sets the shared text
/// and ends the load; an empty text removes it; a change empties the owner.
#[test]
fn form_descriptions() {
    let mut e = world();
    assert_eq!(site(&mut e, 0x0048_2f7f, &[DESC]), None);
    e.mem.set_u32(DESCRIPTION_OWNER, 0x1234);
    assert_eq!(
        run(
            &mut e,
            SET_FORM_DESCRIPTION,
            0,
            &[form_arg(R_NOTE), s(b"Read me")]
        ),
        1.0
    );
    assert_eq!(users(&e, Hook::GetDescription), 1);
    assert_eq!(e.mem.u32(DESCRIPTION_OWNER), 0, "the owner emptied");
    assert_eq!(site(&mut e, 0x0048_2f7f, &[DESC]), Some(1));
    let text = e.mem.u32(DESCRIPTION_TEXT);
    assert_eq!(nvse::table::guest_text(&e, text), b"Read me");
    assert_eq!(e.mem.u16(DESCRIPTION_TEXT + 4), 7);
    // Again: the text replaced, no second user.
    run(
        &mut e,
        SET_FORM_DESCRIPTION,
        0,
        &[form_arg(R_NOTE), s(b"Other")],
    );
    assert_eq!(users(&e, Hook::GetDescription), 1);
    site(&mut e, 0x0048_2f7f, &[DESC]);
    assert_eq!(
        nvse::table::guest_text(&e, e.mem.u32(DESCRIPTION_TEXT)),
        b"Other"
    );
    // A form without a description (a quest here: the cast stand-in answers none).
    assert_eq!(
        run(
            &mut e,
            SET_FORM_DESCRIPTION,
            0,
            &[form_arg(R_QUEST), s(b"x")]
        ),
        0.0
    );
    // Empty: removed with its user; removing again changes nothing.
    e.mem.set_u32(DESCRIPTION_OWNER, 0x1234);
    run(&mut e, SET_FORM_DESCRIPTION, 0, &[form_arg(R_NOTE), s(b"")]);
    assert_eq!(users(&e, Hook::GetDescription), 0);
    assert_eq!(e.mem.u32(DESCRIPTION_OWNER), 0);
    e.mem.set_u32(DESCRIPTION_OWNER, 0x1234);
    run(&mut e, SET_FORM_DESCRIPTION, 0, &[form_arg(R_NOTE), s(b"")]);
    assert_eq!(
        e.mem.u32(DESCRIPTION_OWNER),
        0x1234,
        "nothing removed: the owner kept"
    );
    assert_eq!(users(&e, Hook::GetDescription), 0);
    assert_eq!(site(&mut e, 0x0048_2f7f, &[DESC]), None);
}

/// ToggleItemUnique and the CheckUniqueItem seam.
#[test]
fn unique_items() {
    let mut e = world();
    assert_eq!(site(&mut e, 0x0047_bbec, &[ARMOR_FORM, 0x18]), None);
    run(
        &mut e,
        TOGGLE_ITEM_UNIQUE,
        0,
        &[form_arg(R_LIST), int_arg(1)],
    );
    assert_eq!(
        users(&e, Hook::CheckUniqueItem),
        2,
        "the list's armour and weapon"
    );
    assert_eq!(site(&mut e, 0x0047_bbec, &[ARMOR_FORM, 0x18]), Some(0));
    assert_eq!(
        site(&mut e, 0x0047_bbec, &[ARMOR_FORM, 0x28]),
        None,
        "other type"
    );
    assert_eq!(site(&mut e, 0x0047_bbec, &[WEAPON_FORM, 0x28]), Some(0));
    run(
        &mut e,
        TOGGLE_ITEM_UNIQUE,
        0,
        &[form_arg(R_ARMOR), int_arg(1)],
    );
    assert_eq!(users(&e, Hook::CheckUniqueItem), 2, "already unique");
    run(
        &mut e,
        TOGGLE_ITEM_UNIQUE,
        0,
        &[form_arg(R_ARMOR), int_arg(0)],
    );
    assert_eq!(users(&e, Hook::CheckUniqueItem), 1);
    assert_eq!(site(&mut e, 0x0047_bbec, &[ARMOR_FORM, 0x18]), None);
    run(
        &mut e,
        TOGGLE_ITEM_UNIQUE,
        0,
        &[form_arg(R_ARMOR), int_arg(0)],
    );
    assert_eq!(
        users(&e, Hook::CheckUniqueItem),
        1,
        "not unique: nothing back"
    );
    // Not a weapon, armour or list: nothing.
    run(
        &mut e,
        TOGGLE_ITEM_UNIQUE,
        0,
        &[form_arg(R_QUEST), int_arg(1)],
    );
    assert_eq!(users(&e, Hook::CheckUniqueItem), 1);
    let id = e.mem.u32(WEAPON_FORM + 0xc);
    assert_eq!(state(&mut e).form_flags.get(&id), Some(&UNIQUE_ITEM));
}

/// Many distinct names on one reference take one block of the game's heap, rewritten in place:
/// the heap does not grow, and a pointer handed out before reads the newer name. A removed
/// name's block, and the model paths' blocks dropped before a load, serve the next texts.
#[test]
fn many_names_keep_the_memory_bounded() {
    let mut e = world();
    let heap = e.mem.u32(HEAP);
    run(&mut e, SET_REF_NAME, REF, &[s(b"start")]);
    let first = site(&mut e, 0x0055_d520, &[REF]).expect("named");
    assert_eq!(
        e.mem.u32(HEAP),
        heap + 0x80,
        "one block of JIP's buffer size"
    );
    for i in 0..5000u32 {
        let name = format!("Raider {i} hp {}", 5000 - i);
        run(&mut e, SET_REF_NAME, REF, &[s(name.as_bytes())]);
        assert_eq!(
            site(&mut e, 0x0055_d520, &[REF]),
            Some(first),
            "the same block"
        );
    }
    assert_eq!(e.mem.u32(HEAP), heap + 0x80, "no more of the heap taken");
    assert_eq!(nvse::table::guest_text(&e, first), b"Raider 4999 hp 1");
    let full = vec![b'x'; 0x7f];
    run(&mut e, SET_REF_NAME, REF, &[s(&full)]);
    assert_eq!(
        site(&mut e, 0x0055_d520, &[REF]),
        Some(first),
        "0x7F and its 0 fit"
    );
    assert_eq!(nvse::table::guest_text(&e, first), full);
    // Removed: the block waits; the next reference's text takes it.
    run(&mut e, SET_REF_NAME, REF, &[s(b"")]);
    assert!(state(&mut e).ref_names.is_empty());
    run(&mut e, SET_REFR_MODEL_PATH, ITEM, &[s(b"b.nif")]);
    assert_eq!(site(&mut e, 0x0050_fe8b, &[BASE, ITEM]), Some(first));
    assert_eq!(e.mem.u32(HEAP), heap + 0x80);
    // Before a load the paths' blocks go back; a name takes it again.
    crate::world_hooks::pre_load(&mut e);
    assert_eq!(state(&mut e).free_texts.len(), 1);
    run(&mut e, SET_REF_NAME, REF, &[s(b"again")]);
    assert_eq!(site(&mut e, 0x0055_d520, &[REF]), Some(first));
    assert_eq!(e.mem.u32(HEAP), heap + 0x80);
    assert!(state(&mut e).free_texts.is_empty());
}

/// A text longer than its block (read whole with the list guard row) moves to a block twice the
/// size; the old block waits for the next text.
#[test]
fn a_longer_text_moves_to_a_bigger_block() {
    let mut e = world();
    // The fixes preset's list guard row (the host turns it on): a longer text is read whole.
    nvse::commands::array::set_fix(&mut e, nvse::commands::array::LIST_GUARD, true);
    run(&mut e, SET_REF_NAME, REF, &[s(b"short")]);
    let small = site(&mut e, 0x0055_d520, &[REF]).expect("named");
    let long = vec![b'y'; 0x90];
    run(&mut e, SET_REF_NAME, REF, &[s(&long)]);
    let big = site(&mut e, 0x0055_d520, &[REF]).expect("named");
    assert_ne!(big, small);
    assert_eq!(nvse::table::guest_text(&e, big), long);
    assert_eq!(
        state(&mut e).ref_names[&REF].capacity,
        0x100,
        "the next power of two"
    );
    assert_eq!(
        state(&mut e).free_texts,
        vec![TextBlock {
            at: small,
            capacity: 0x80
        }]
    );
    run(&mut e, SET_REFR_MODEL_PATH, REF, &[s(b"c.nif")]);
    assert_eq!(
        site(&mut e, 0x0050_fe8b, &[BASE, REF]),
        Some(small),
        "reused"
    );
}

/// SetRefName and SetRefrModelPath with their seams; the pre-load keeps the names (their flag
/// set again) and drops the paths.
#[test]
fn reference_names_and_model_paths() {
    let mut e = world();
    assert_eq!(site(&mut e, 0x0055_d520, &[REF]), None);
    run(&mut e, SET_REF_NAME, REF, &[s(b"Bob")]);
    run(&mut e, SET_REFR_MODEL_PATH, REF, &[s(b"a.nif")]);
    assert_eq!(users(&e, Hook::GetRefName), 1);
    assert_eq!(users(&e, Hook::GetModelPath), 1);
    assert_eq!(e.mem.u8(REF + 0x5f), ALT_REF_NAME | REFR_MODEL_PATH);
    // On the loader and AI threads: both answer while another engine holds JIP's shared slot.
    let name = held(&mut e, |e| site(e, 0x0055_d520, &[REF])).expect("named");
    assert_eq!(nvse::table::guest_text(&e, name), b"Bob");
    let path = held(&mut e, |e| site(e, 0x0050_fe8b, &[BASE, REF])).expect("a path");
    assert_eq!(nvse::table::guest_text(&e, path), b"a.nif");
    assert_eq!(site(&mut e, 0x0050_fe8b, &[BASE, 0]), None, "no reference");
    run(&mut e, SET_REF_NAME, REF, &[s(b"Al")]);
    assert_eq!(users(&e, Hook::GetRefName), 1, "replaced: no second user");
    // The flag decides even with a name kept.
    e.mem.set_u8(REF + 0x5f, REFR_MODEL_PATH);
    assert_eq!(site(&mut e, 0x0055_d520, &[REF]), None);
    e.mem.set_u8(REF + 0x5f, ALT_REF_NAME);
    assert_eq!(site(&mut e, 0x0050_fe8b, &[BASE, REF]), None);
    crate::world_hooks::pre_load(&mut e);
    assert_eq!(e.mem.u8(REF + 0x5f), ALT_REF_NAME);
    assert_eq!(users(&e, Hook::GetRefName), 1);
    assert_eq!(users(&e, Hook::GetModelPath), 0);
    assert!(state(&mut e).model_paths.is_empty());
    e.mem.set_u8(REF + 0x5f, 0);
    crate::world_hooks::pre_load(&mut e);
    assert_eq!(e.mem.u8(REF + 0x5f), ALT_REF_NAME, "set again");
    run(&mut e, SET_REF_NAME, REF, &[s(b"")]);
    assert_eq!(users(&e, Hook::GetRefName), 0);
    assert_eq!(e.mem.u8(REF + 0x5f), 0);
    run(&mut e, SET_REF_NAME, REF, &[s(b"")]);
    assert_eq!(users(&e, Hook::GetRefName), 0, "nothing to remove");
}

/// The DoActivate and ActivateDoor seams, with ToggleIgnoreLockedDoors.
#[test]
fn activations_and_locked_doors() {
    let mut e = world();
    assert_eq!(site(&mut e, 0x0057_3512, &[REF]), None);
    e.mem.set_u8(BASE + 6, ACTIVATE_DISABLED);
    assert_eq!(
        site(&mut e, 0x0057_3512, &[REF]),
        Some(1),
        "the base's flag"
    );
    e.mem.set_u8(BASE + 6, 0);
    e.mem.set_u8(REF + 6, ACTIVATE_DISABLED);
    assert_eq!(site(&mut e, 0x0057_3512, &[REF]), Some(1));
    // A teammate at a locked door leading nowhere.
    let teammate = ITEM;
    e.mem.set_u8(teammate + 0x18d, 1);
    set_extra(&mut e, REF + 0x44, EXTRA_LOCK, LOCK);
    e.mem.set_u32(LOCK + 0xc, LOCK_DATA);
    e.mem.set_u8(LOCK_DATA + 8, 1);
    assert_eq!(site(&mut e, 0x0057_3673, &[REF, teammate, 0]), Some(1));
    assert_eq!(
        site(&mut e, 0x0057_3673, &[REF, teammate, 0x99]),
        None,
        "it leads somewhere"
    );
    assert_eq!(
        site(&mut e, 0x0057_3673, &[REF, 0, 0]),
        None,
        "no activator"
    );
    e.mem.set_u8(teammate + 0x18d, 0);
    assert_eq!(
        site(&mut e, 0x0057_3673, &[REF, teammate, 0]),
        None,
        "not a teammate"
    );
    e.mem.set_u8(teammate + 0x18d, 1);
    e.mem.set_u8(LOCK_DATA + 8, 0);
    assert_eq!(
        site(&mut e, 0x0057_3673, &[REF, teammate, 0]),
        None,
        "unlocked"
    );
    e.mem.set_u8(LOCK_DATA + 8, 1);
    e.mem.set_u32(LOCK + 0xc, 0);
    assert_eq!(
        site(&mut e, 0x0057_3673, &[REF, teammate, 0]),
        None,
        "no lock data"
    );
    set_extra(&mut e, REF + 0x44, EXTRA_LOCK, 0);
    assert_eq!(
        site(&mut e, 0x0057_3673, &[REF, teammate, 0]),
        None,
        "no lock"
    );
    run(&mut e, TOGGLE_IGNORE_LOCKED_DOORS, 0, &[int_arg(1)]);
    run(&mut e, TOGGLE_IGNORE_LOCKED_DOORS, 0, &[int_arg(1)]);
    assert_eq!(users(&e, Hook::ActivateDoor), 1, "HOOK_SET: on once");
    run(&mut e, TOGGLE_IGNORE_LOCKED_DOORS, 0, &[int_arg(0)]);
    assert_eq!(users(&e, Hook::ActivateDoor), 0);
}

/// The fast travel, cell change, target change and note events, with GetPCFastTravelled and
/// GetPCMovedCell (and their pre-load part).
#[test]
fn player_events() {
    let mut e = world();
    run(&mut e, SET_ON_FAST_TRAVEL, 0, &[form_arg(R_S1), int_arg(1)]);
    run(&mut e, SET_ON_FAST_TRAVEL, 0, &[form_arg(R_S1), int_arg(1)]);
    assert_eq!(users(&e, Hook::PCFastTravel), 1);
    // Asked from the calling reference (the script has no quest): not yet informed.
    assert_eq!(run(&mut e, GET_PC_FAST_TRAVELLED, REF, &[]), 0.0);
    assert_eq!(
        users(&e, Hook::PCFastTravel),
        2,
        "the first asker is a user"
    );
    assert_eq!(e.mem.u8(REF + 5), FAST_TRAVEL_INFORMED);
    assert_eq!(
        run(&mut e, GET_PC_FAST_TRAVELLED, REF, &[]),
        0.0,
        "still in the set"
    );
    crate::script_call::calls();
    assert_eq!(site(&mut e, 0x0093_bf22, &[0x7000]), None);
    assert_eq!(
        crate::script_call::calls(),
        vec![(SCRIPT_ONE, 0, vec![0x7000])]
    );
    assert_eq!(
        run(&mut e, GET_PC_FAST_TRAVELLED, REF, &[]),
        1.0,
        "travelled since"
    );
    assert_eq!(users(&e, Hook::PCFastTravel), 3, "a first asker again");
    run(&mut e, SET_ON_FAST_TRAVEL, 0, &[form_arg(R_S1), int_arg(0)]);
    run(&mut e, SET_ON_FAST_TRAVEL, 0, &[form_arg(R_S1), int_arg(0)]);
    assert_eq!(
        users(&e, Hook::PCFastTravel),
        2,
        "removing what is not set gives nothing back"
    );
    // The cell change: switched on, the set forgotten at a new cell.
    e.mem.set_u32(CALLER + 0x40, QUEST_FORM);
    assert_eq!(
        run(&mut e, GET_PC_MOVED_CELL, 0, &[]),
        0.0,
        "the quest asks"
    );
    assert_eq!(users(&e, Hook::PCCellChange), 1);
    assert_eq!(e.mem.u8(QUEST_FORM + 5), CELL_CHANGE_INFORMED);
    site(&mut e, 0x0094_cb35, &[0x9100]);
    assert_eq!(run(&mut e, GET_PC_MOVED_CELL, 0, &[]), 1.0);
    site(&mut e, 0x0094_cb35, &[0x9100]);
    assert_eq!(
        run(&mut e, GET_PC_MOVED_CELL, 0, &[]),
        0.0,
        "the same cell: kept"
    );
    site(&mut e, 0x0094_cb35, &[0]);
    assert_eq!(run(&mut e, GET_PC_MOVED_CELL, 0, &[]), 0.0, "no cell: kept");
    // The pre-load: flags cleared, sets gone with their users.
    crate::world_hooks::pre_load(&mut e);
    assert_eq!(e.mem.u8(REF + 5), 0);
    assert_eq!(e.mem.u8(QUEST_FORM + 5), 0);
    assert_eq!(users(&e, Hook::PCFastTravel), 1, "one user less");
    assert_eq!(users(&e, Hook::PCCellChange), 0);
    // Target change: a new target only.
    run(
        &mut e,
        SET_ON_PC_TARGET_CHANGE,
        0,
        &[form_arg(R_S2), int_arg(1)],
    );
    assert_eq!(users(&e, Hook::SetPCTarget), 1);
    e.mem.set_u32(PLAYER + 0xd44, 0x8800);
    crate::script_call::calls();
    site(&mut e, 0x0096_4675, &[PLAYER]);
    site(&mut e, 0x0096_4675, &[PLAYER]);
    assert_eq!(
        crate::script_call::calls(),
        vec![(SCRIPT_TWO, 0, vec![0x8800])]
    );
    run(
        &mut e,
        SET_ON_PC_TARGET_CHANGE,
        0,
        &[form_arg(R_S2), int_arg(0)],
    );
    assert_eq!(users(&e, Hook::SetPCTarget), 0);
    // Notes.
    run(&mut e, SET_ON_NOTE_ADDED, 0, &[form_arg(R_S1), int_arg(1)]);
    run(&mut e, SET_ON_NOTE_ADDED, 0, &[form_arg(R_S2), int_arg(1)]);
    assert_eq!(users(&e, Hook::AddNote), 2);
    site(&mut e, 0x0096_6af9, &[NOTE_FORM]);
    assert_eq!(
        crate::script_call::calls(),
        vec![
            (SCRIPT_ONE, 0, vec![NOTE_FORM]),
            (SCRIPT_TWO, 0, vec![NOTE_FORM])
        ]
    );
    // Not a script: nothing.
    run(
        &mut e,
        SET_ON_NOTE_ADDED,
        0,
        &[form_arg(R_QUEST), int_arg(1)],
    );
    assert_eq!(users(&e, Hook::AddNote), 2);
}

/// SetOnDialogTopicEventHandler and the RunResultScript seam.
/// An info a mod links under two topics: the handler calls the scripts of the topic
/// `GetOwnerTopic` answers (the exe's walk: the first topic holding it; with JIP's step 3.9
/// patch, the last linked), never both. The stand-in walks two topics' info lists.
#[test]
fn an_info_under_two_topics_takes_get_owner_topics_answer() {
    let mut e = world();
    const SECOND_TOPIC: u32 = AREA + 0x4800;
    form(&mut e, SECOND_TOPIC, TOPIC, 0x0100_0010, false);
    // Each topic's info list (one info each; the same info in both).
    const LISTS: u32 = AREA + 0x4900;
    e.mem.set_u32(LISTS, INFO);
    e.mem.set_u32(LISTS + 8, INFO);
    e.register(GET_OWNER_TOPIC, |e, w| {
        let topic = [(TOPIC_FORM, LISTS), (SECOND_TOPIC, LISTS + 8)]
            .into_iter()
            .find(|(_, list)| e.mem.u32(*list) == w[0])
            .map_or(0, |(t, _)| t);
        Ret {
            eax: topic,
            ..Ret::default()
        }
    });
    run(
        &mut e,
        SET_ON_DIALOG_TOPIC,
        0,
        &[form_arg(R_S1), int_arg(1), form_arg(R_TOPIC)],
    );
    // The second topic's handler (no script argument index of its own here).
    state(&mut e)
        .dialog_topic
        .entry(SECOND_TOPIC)
        .or_default()
        .insert(SCRIPT_TWO);
    crate::script_call::calls();
    // On the AI threads (CreateConversation): it answers while another engine holds JIP's slot.
    assert_eq!(
        held(&mut e, |e| site(e, 0x0061_f184, &[INFO, 0, REF])),
        None
    );
    assert_eq!(
        crate::script_call::calls(),
        vec![(SCRIPT_ONE, REF, vec![TOPIC_FORM])]
    );
}

/// With JIP's game patch "Store parent TESTopic" on (step 3.9a: the host's row), the handler reads
/// the info's recorded topic, the last its load linked (here the second topic, where the exe's
/// walk answers the first), as JIP reads its word: no call of `GetOwnerTopic`.
#[test]
fn an_info_under_two_topics_takes_the_last_linked_with_jips_patch() {
    let mut e = world();
    const SECOND_TOPIC: u32 = AREA + 0x4800;
    form(&mut e, SECOND_TOPIC, TOPIC, 0x0100_0010, false);
    // No jip_nvse.ini (`CreateFileA` finds none): JIP's defaults.
    e.register(0x00fd_f088, |_, _| Ret {
        eax: 0xffff_ffff,
        ..Ret::default()
    });
    crate::game_patches::set_row(&mut e, true);
    e.site(0x0061_e086, &[TOPIC_FORM, INFO]);
    e.site(0x0061_e086, &[SECOND_TOPIC, INFO]);
    run(
        &mut e,
        SET_ON_DIALOG_TOPIC,
        0,
        &[form_arg(R_S1), int_arg(1), form_arg(R_TOPIC)],
    );
    state(&mut e)
        .dialog_topic
        .entry(SECOND_TOPIC)
        .or_default()
        .insert(SCRIPT_TWO);
    crate::script_call::calls();
    e.call_log = Some(vec![]);
    // The fixes branch reads the dialogue patch's lock-free record: it answers while another
    // engine holds JIP's shared slot.
    assert_eq!(
        held(&mut e, |e| site(e, 0x0061_f184, &[INFO, 0, REF])),
        None
    );
    let log = e.call_log.take().unwrap();
    assert_eq!(
        crate::script_call::calls(),
        vec![(SCRIPT_TWO, REF, vec![SECOND_TOPIC])]
    );
    assert!(log.iter().all(|c| c.0 != GET_OWNER_TOPIC), "no call");
}

#[test]
fn dialogue_topic_events() {
    let mut e = world();
    run(
        &mut e,
        SET_ON_DIALOG_TOPIC,
        0,
        &[form_arg(R_S1), int_arg(1), form_arg(R_INFO)],
    );
    run(
        &mut e,
        SET_ON_DIALOG_TOPIC,
        0,
        &[form_arg(R_S2), int_arg(1), form_arg(R_INFO)],
    );
    run(
        &mut e,
        SET_ON_DIALOG_TOPIC,
        0,
        &[form_arg(R_S2), int_arg(1), form_arg(R_TOPIC)],
    );
    assert_eq!(users(&e, Hook::RunResultScript), 2, "one per key");
    run(
        &mut e,
        SET_ON_DIALOG_TOPIC,
        0,
        &[form_arg(R_S1), int_arg(1), form_arg(R_QUEST)],
    );
    assert_eq!(users(&e, Hook::RunResultScript), 2, "not a topic");
    crate::script_call::calls();
    assert_eq!(site(&mut e, 0x0061_f184, &[INFO, 1, REF]), None);
    assert!(
        crate::script_call::calls().is_empty(),
        "not the first result script"
    );
    assert_eq!(site(&mut e, 0x0061_f184, &[INFO, 0, REF]), None);
    assert_eq!(
        crate::script_call::calls(),
        vec![
            (SCRIPT_ONE, REF, vec![INFO]),
            (SCRIPT_TWO, REF, vec![INFO]),
            (SCRIPT_TWO, REF, vec![TOPIC_FORM]),
        ]
    );
    run(
        &mut e,
        SET_ON_DIALOG_TOPIC,
        0,
        &[form_arg(R_S2), int_arg(0), form_arg(R_TOPIC)],
    );
    assert_eq!(users(&e, Hook::RunResultScript), 1);
    run(
        &mut e,
        SET_ON_DIALOG_TOPIC,
        0,
        &[form_arg(R_S2), int_arg(0), form_arg(R_TOPIC)],
    );
    run(
        &mut e,
        SET_ON_DIALOG_TOPIC,
        0,
        &[form_arg(R_S1), int_arg(0), form_arg(R_INFO)],
    );
    assert_eq!(
        users(&e, Hook::RunResultScript),
        1,
        "one script left for the info"
    );
}

/// SetOnQuestStageEventHandler and the SetQuestStage seam.
#[test]
fn quest_stage_events() {
    let mut e = world();
    let add = |e: &mut Engine, script: u16, stage: i32, skip: i32| {
        run(
            e,
            SET_ON_QUEST_STAGE,
            0,
            &[
                form_arg(script),
                int_arg(1),
                form_arg(R_QUEST),
                int_arg(stage),
                int_arg(skip),
            ],
        )
    };
    assert_eq!(add(&mut e, R_S1, 11, 0), 0.0, "no such stage");
    assert_eq!(add(&mut e, R_S1, 10, 1), 1.0);
    assert_eq!(add(&mut e, R_S1, 10, 0), 0.0, "already there");
    assert_eq!(users(&e, Hook::SetQuestStage), 1);
    assert_eq!(e.mem.u8(QUEST_FORM + 6), SET_STAGE_HANDLERS);
    // A stage not done: marked done at once; the script asked to skip: 0.
    crate::script_call::calls();
    assert_eq!(site(&mut e, 0x0060_f4a9, &[STAGE, QUEST_FORM]), Some(0));
    assert_eq!(e.mem.u8(STAGE + 1), 1);
    assert_eq!(
        crate::script_call::calls(),
        vec![(SCRIPT_ONE, 0, vec![QUEST_FORM, 10])]
    );
    // Done, the quest's stages do not repeat: 0 without scripts.
    assert_eq!(site(&mut e, 0x0060_f4a9, &[STAGE, QUEST_FORM]), Some(0));
    assert!(crate::script_call::calls().is_empty());
    e.mem.set_u8(QUEST_FORM + 0x3c, 8);
    assert_eq!(add(&mut e, R_S2, 10, 0), 1.0);
    assert_eq!(users(&e, Hook::SetQuestStage), 1, "one per quest");
    // On an AI thread (a result script): it answers while another engine holds JIP's slot.
    assert_eq!(
        held(&mut e, |e| site(e, 0x0060_f4a9, &[STAGE, QUEST_FORM])),
        Some(1),
        "1 | 2: set"
    );
    assert_eq!(crate::script_call::calls().len(), 2);
    // Without the flag: 1, no scripts.
    e.mem.set_u8(QUEST_FORM + 6, 0);
    assert_eq!(site(&mut e, 0x0060_f4a9, &[STAGE, QUEST_FORM]), Some(1));
    assert!(crate::script_call::calls().is_empty());
    e.mem.set_u8(QUEST_FORM + 6, SET_STAGE_HANDLERS);
    let remove = |e: &mut Engine, script: u16| {
        run(
            e,
            SET_ON_QUEST_STAGE,
            0,
            &[
                form_arg(script),
                int_arg(0),
                form_arg(R_QUEST),
                int_arg(10),
                int_arg(0),
            ],
        )
    };
    assert_eq!(remove(&mut e, R_S1), 1.0);
    assert_eq!(remove(&mut e, R_S1), 0.0, "gone already");
    assert_eq!(users(&e, Hook::SetQuestStage), 1);
    assert_eq!(remove(&mut e, R_S2), 1.0);
    assert_eq!(users(&e, Hook::SetQuestStage), 0);
    assert_eq!(e.mem.u8(QUEST_FORM + 6), 0);
}

/// SetWeatherTransitionTimeOverride and the UpdateWeather seam.
#[test]
fn weather_transition_rate() {
    let mut e = world();
    let hours = 3.0f64.to_bits();
    let ask = |e: &mut Engine| {
        f32::from_bits(site(e, 0x0063_d4f5, &[hours as u32, (hours >> 32) as u32]).unwrap())
    };
    assert_eq!(ask(&mut e), 12.0, "the default rate 4");
    run(
        &mut e,
        SET_WEATHER_TRANSITION_TIME_OVERRIDE,
        0,
        &[int_arg(-60)],
    );
    assert_eq!(users(&e, Hook::UpdateWeather), 1);
    assert_eq!(ask(&mut e), -6.0, "120 / -60");
    run(
        &mut e,
        SET_WEATHER_TRANSITION_TIME_OVERRIDE,
        0,
        &[int_arg(0)],
    );
    assert_eq!(users(&e, Hook::UpdateWeather), 0);
    assert_eq!(ask(&mut e), -6.0, "the rate kept");
}

fn message(e: &mut Engine, kind: u32, data: u32) {
    crate::cosave::listen(e, &nvse::plugins::Message { kind, data, len: 0 });
}

/// The flag bytes SetJIPFlag recorded come back after a load that succeeded and at a new game
/// (`RestoreJIPFormFlags`), onto the form the load rebuilt with its bytes cleared; a failed load
/// leaves them; an id no form has any more leaves the record, as does a byte of 0.
#[test]
fn recorded_form_flags_come_back_after_a_load() {
    use nvse::plugins::msg;
    let mut e = world();
    run(&mut e, SET_INTERACTION_DISABLED, REF, &[int_arg(1)]);
    run(&mut e, SET_INTERACTION_DISABLED, ITEM, &[int_arg(1)]);
    let (ref_id, item_id) = (0x0100_0001, 0x0100_0003);
    assert_eq!(
        state(&mut e).form_flags.get(&ref_id),
        Some(&NO_PC_ACTIVATION)
    );
    // The load rebuilds the forms: their bytes are 0 again.
    e.mem.set_u8(REF + 6, 0);
    message(&mut e, msg::POST_LOAD_GAME, 0);
    assert_eq!(e.mem.u8(REF + 6), 0, "a failed load restores nothing");
    assert_eq!(state(&mut e).form_flags.len(), 2);
    message(&mut e, msg::POST_LOAD_GAME, 1);
    assert_eq!(e.mem.u8(REF + 6), NO_PC_ACTIVATION);
    // The item is not in the game's form map: its record goes.
    assert_eq!(state(&mut e).form_flags.get(&item_id), None);
    assert_eq!(state(&mut e).form_flags.len(), 1);
    e.mem.set_u8(REF + 6, 0);
    message(&mut e, msg::NEW_GAME, 0);
    assert_eq!(e.mem.u8(REF + 6), NO_PC_ACTIVATION, "a new game too");
    // A byte of 0 (every flag cleared) leaves the record.
    run(&mut e, SET_INTERACTION_DISABLED, REF, &[int_arg(0)]);
    assert_eq!(state(&mut e).form_flags.get(&ref_id), Some(&0));
    message(&mut e, msg::POST_LOAD_GAME, 1);
    assert!(state(&mut e).form_flags.is_empty());
}

/// A form type byte past the game's 121 types (bad mod data) answers "not disabled", "no script"
/// and "not an inventory item" instead of stopping: the activation seam, SetScriptEventDisabled
/// on a reference with such a base and on such a form.
#[test]
fn a_form_type_past_the_tables_stops_nothing() {
    let mut e = world();
    e.mem.set_u8(BASE + 4, 0xff);
    assert_eq!(site(&mut e, 0x0094_3279, &[REF]), None);
    run(
        &mut e,
        SET_SCRIPT_EVENT_DISABLED,
        0,
        &[form_arg(R_REF), s(b"OnAdd"), int_arg(1)],
    );
    run(
        &mut e,
        SET_SCRIPT_EVENT_DISABLED,
        0,
        &[form_arg(R_BASE), s(b"OnAdd"), int_arg(1)],
    );
    assert!(state(&mut e).disabled_events.is_empty());
    assert_eq!(users(&e, Hook::MarkScriptEvent), 0);
    // SetInteractionDisabledType's own check passes only the filter types.
    run(
        &mut e,
        SET_INTERACTION_DISABLED_TYPE,
        0,
        &[int_arg(1), int_arg(0xff)],
    );
    assert_eq!(users(&e, Hook::InteractionDisabled), 0);
}

/// A swapped cell of a world space with an editor id of `len` characters: the seam's answer and
/// the buffer.
fn lod_name_with_editor_id(len: usize) -> (Option<u32>, Vec<u8>) {
    let mut e = world();
    e.mem.set_u32(FORM_VT + 0x130, 0x00de_0130);
    e.register_double(0x00de_0130, |_, _| Ret {
        eax: STRINGS,
        ..Ret::default()
    });
    let mut edid = vec![b'W'; len];
    edid.push(0);
    e.mem.write(STRINGS, &edid);
    run(
        &mut e,
        SWAP_OBJECT_LOD,
        0,
        &[form_arg(R_WORLD), int_arg(-2), int_arg(3)],
    );
    let answer = site(&mut e, 0x006f_6df1, &[WORLD, 8, 0xffff_fffe, 3, BUFFER]);
    (answer, nvse::table::guest_text(&e, BUFFER))
}

/// The longest editor id whose AltLOD name fits JIP's 0x104 bytes (100 characters: 259 bytes and
/// the 0) is written whole.
#[test]
fn a_long_editor_id_that_fits() {
    let (answer, name) = lod_name_with_editor_id(100);
    assert_eq!(answer, Some(1));
    assert_eq!(name.len(), 259);
    assert!(name.ends_with(b".Level8.X-2.Y3.AltLOD.nif"));
}

/// One character more: JIP's `sprintf_s` ends the game; the port stops at the same place.
#[test]
#[should_panic(expected = "a LOD block name of 261 bytes does not fit JIP's 0x104-byte buffer")]
fn a_too_long_editor_id_stops_as_jip_does() {
    lod_name_with_editor_id(101);
}

/// SwapObjectLOD and the MakeObjLODPath seam (and the pre-load).
#[test]
fn swapped_lod_blocks() {
    let mut e = world();
    e.mem.set_u32(FORM_VT + 0x130, 0x00de_0130);
    e.register_double(0x00de_0130, |_, _| Ret {
        eax: STRINGS,
        ..Ret::default()
    });
    e.mem.write(STRINGS, b"Wasteland\0");
    assert_eq!(
        site(&mut e, 0x006f_6df1, &[WORLD, 8, 0xffff_fffe, 3, BUFFER]),
        None
    );
    run(
        &mut e,
        SWAP_OBJECT_LOD,
        0,
        &[form_arg(R_WORLD), int_arg(-2), int_arg(3)],
    );
    run(
        &mut e,
        SWAP_OBJECT_LOD,
        0,
        &[form_arg(R_WORLD), int_arg(-2), int_arg(3)],
    );
    assert_eq!(users(&e, Hook::MakeObjLODPath), 1);
    run(
        &mut e,
        SWAP_OBJECT_LOD,
        0,
        &[form_arg(R_QUEST), int_arg(1), int_arg(1)],
    );
    assert_eq!(
        site(&mut e, 0x006f_6df1, &[WORLD, 8, 0xffff_fffe, 3, BUFFER]),
        Some(1)
    );
    assert_eq!(
        nvse::table::guest_text(&e, BUFFER),
        b"Data\\Meshes\\Landscape\\LOD\\Wasteland\\Blocks\\Wasteland.Level8.X-2.Y3.AltLOD.nif"
    );
    // The key is the low 16 bits of each.
    assert_eq!(
        site(&mut e, 0x006f_6df1, &[WORLD, 8, 0x0001_fffe, 3, BUFFER]),
        Some(1)
    );
    assert_eq!(
        site(&mut e, 0x006f_6df1, &[WORLD, 8, 0xfffe, 4, BUFFER]),
        None
    );
    crate::world_hooks::pre_load(&mut e);
    assert_eq!(users(&e, Hook::MakeObjLODPath), 0);
    assert_eq!(
        site(&mut e, 0x006f_6df1, &[WORLD, 8, 0xffff_fffe, 3, BUFFER]),
        None
    );
}

/// The PlayerMinHealth seam (its command waits for step 3.9).
#[test]
fn player_min_health() {
    let mut e = world();
    let ask = |e: &mut Engine, health: f32, change: f32| {
        site(e, 0x0093_b80a, &[health.to_bits(), change.to_bits()]).map(f32::from_bits)
    };
    assert_eq!(ask(&mut e, 10.0, 5.0), None, "a gain");
    assert_eq!(ask(&mut e, 10.0, -5.0), None, "above 1 after");
    assert_eq!(ask(&mut e, 10.0, -9.0), None, "exactly 1 after");
    assert_eq!(ask(&mut e, 10.0, -9.5), Some(-9.0));
    assert_eq!(ask(&mut e, 10.0, -0.0), None, "-0 sum 10");
    assert_eq!(ask(&mut e, 0.5, -0.0), Some(0.5), "-0 has the sign bit");
    assert_eq!(
        ask(&mut e, f32::NAN, -1.0).map(f32::is_nan),
        Some(true),
        "unordered"
    );
}

/// SetInteractionDisabled(Type) and the activation seam (the HUD half is step 3.5a's).
#[test]
fn interaction_disabled() {
    let mut e = world();
    assert_eq!(site(&mut e, 0x0094_3279, &[REF]), None);
    run(&mut e, SET_INTERACTION_DISABLED, REF, &[int_arg(1)]);
    run(&mut e, SET_INTERACTION_DISABLED, REF, &[int_arg(1)]);
    assert_eq!(users(&e, Hook::InteractionDisabled), 1);
    assert_eq!(site(&mut e, 0x0094_3279, &[REF]), Some(1));
    run(&mut e, SET_INTERACTION_DISABLED, REF, &[int_arg(0)]);
    assert_eq!(users(&e, Hook::InteractionDisabled), 0);
    assert_eq!(site(&mut e, 0x0094_3279, &[REF]), None);
    run(&mut e, SET_INTERACTION_DISABLED, 0, &[int_arg(1)]);
    assert_eq!(
        users(&e, Hook::InteractionDisabled),
        0,
        "no form, no reference"
    );
    // The base by its form (a reference given would win over the calling one).
    run(
        &mut e,
        SET_INTERACTION_DISABLED,
        REF,
        &[int_arg(1), form_arg(R_BASE)],
    );
    assert_eq!(users(&e, Hook::InteractionDisabled), 1);
    assert_eq!(e.mem.u8(BASE + 6), NO_PC_ACTIVATION);
    assert_eq!(e.mem.u8(REF + 6), 0);
    assert_eq!(site(&mut e, 0x0094_3279, &[REF]), Some(1));
    run(
        &mut e,
        SET_INTERACTION_DISABLED,
        0,
        &[int_arg(0), form_arg(R_BASE)],
    );
    assert_eq!(users(&e, Hook::InteractionDisabled), 0);
    assert_eq!(site(&mut e, 0x0094_3279, &[REF]), None);
    // A type: 0x15 (an activator) is a filter type; 0x11 (script) is not.
    run(
        &mut e,
        SET_INTERACTION_DISABLED_TYPE,
        0,
        &[int_arg(1), int_arg(0x15)],
    );
    run(
        &mut e,
        SET_INTERACTION_DISABLED_TYPE,
        0,
        &[int_arg(1), int_arg(0x15)],
    );
    assert_eq!(users(&e, Hook::InteractionDisabled), 1);
    assert_eq!(site(&mut e, 0x0094_3279, &[REF]), Some(1));
    run(
        &mut e,
        SET_INTERACTION_DISABLED_TYPE,
        0,
        &[int_arg(1), int_arg(200)],
    );
    assert_eq!(users(&e, Hook::InteractionDisabled), 1, "out of range");
    run(
        &mut e,
        SET_INTERACTION_DISABLED_TYPE,
        0,
        &[int_arg(0), int_arg(0x15)],
    );
    assert_eq!(users(&e, Hook::InteractionDisabled), 0);
    assert_eq!(site(&mut e, 0x0094_3279, &[REF]), None);
}

/// SetScriptEventDisabled and the MarkScriptEvent and DoActivate seams.
#[test]
fn script_events_disabled() {
    let mut e = world();
    // REF's base has a script (an activator: `TESScriptableForm` at word 0x13 of its type 0x15).
    let scriptable =
        BASE + 4 * ((crate::commands::forms::BASE_COMPONENT_OFFSETS[0x15] >> 8) as u8 as i8 as u32);
    e.mem.set_u32(scriptable + 4, OBJ_SCRIPT);
    run(
        &mut e,
        SET_SCRIPT_EVENT_DISABLED,
        0,
        &[form_arg(R_REF), s(b"OnHit  ONLOAD OnActivate"), int_arg(1)],
    );
    assert_eq!(users(&e, Hook::MarkScriptEvent), 1);
    assert_eq!(users(&e, Hook::DoActivate), 1);
    assert_eq!(e.mem.u8(REF + 6), EVENT_DISABLED | ACTIVATE_DISABLED);
    assert_eq!(state(&mut e).disabled_events[&REF], 0x1080);
    assert_eq!(site(&mut e, 0x0057_3512, &[REF]), Some(1));
    // The seam: the list's extra script with an event list and a script with data.
    let list = REF + 0x44;
    set_extra(&mut e, list, EXTRA_SCRIPT, XSCRIPT);
    e.mem.set_u32(XSCRIPT + 0xc, OBJ_SCRIPT);
    e.mem.set_u32(XSCRIPT + 0x10, 0x7700);
    e.mem.set_u32(OBJ_SCRIPT + 0x20, 20);
    // On the AI threads: it answers while another engine holds JIP's shared slot.
    assert_eq!(
        held(&mut e, |e| site(e, 0x005a_c763, &[0, list, 0x80])),
        Some(1),
        "OnHit disabled"
    );
    assert_eq!(site(&mut e, 0x005a_c763, &[0, list, 0x10]), None, "OnDeath");
    assert_eq!(site(&mut e, 0x005a_c763, &[0, list, 0x1000]), Some(1));
    // No event list: false; a disabled script (data length 0): true.
    e.mem.set_u32(XSCRIPT + 0x10, 0);
    assert_eq!(site(&mut e, 0x005a_c763, &[0, list, 0x10]), Some(0));
    e.mem.set_u32(XSCRIPT + 0x10, 0x7700);
    e.mem.set_u32(OBJ_SCRIPT + 0x20, 0);
    assert_eq!(site(&mut e, 0x005a_c763, &[0, list, 0x10]), Some(1));
    e.mem.set_u32(OBJ_SCRIPT + 0x20, 20);
    set_extra(&mut e, list, EXTRA_SCRIPT, 0);
    assert_eq!(
        site(&mut e, 0x005a_c763, &[0, list, 0x10]),
        Some(0),
        "no script"
    );
    set_extra(&mut e, list, EXTRA_SCRIPT, XSCRIPT);
    // The base's events: OnDeath disabled for the base.
    state(&mut e).disabled_events.insert(BASE, 0x10);
    e.mem.set_u8(BASE + 6, EVENT_DISABLED);
    assert_eq!(site(&mut e, 0x005a_c763, &[0, list, 0x10]), Some(1));
    // Item events (OnAdd, 1): the form is a reference, so the caller's word decides.
    assert_eq!(
        site(&mut e, 0x005a_c763, &[REF, list, 1]),
        None,
        "no caller word"
    );
    state(&mut e).disabled_events.insert(ITEM_BASE, 1);
    e.mem.set_u8(ITEM_BASE + 6, EVENT_DISABLED);
    {
        let _w = e.mark_caller_word(Hook::MarkScriptEvent, ITEM);
        assert_eq!(
            site(&mut e, 0x005a_c763, &[REF, list, 1]),
            Some(1),
            "the item's base"
        );
        // OnDrop with a reference item: skipped at once.
        assert_eq!(site(&mut e, 0x005a_c763, &[REF, list, 4]), Some(1));
    }
    {
        // OnEquip: the item word itself is the form tested.
        let _w = e.mark_caller_word(Hook::MarkScriptEvent, ITEM_BASE);
        state(&mut e).disabled_events.insert(ITEM_BASE, 2);
        assert_eq!(site(&mut e, 0x005a_c763, &[REF, list, 2]), Some(1));
        assert_eq!(
            site(&mut e, 0x005a_c763, &[REF, list, 8]),
            None,
            "OnUnequip not disabled"
        );
    }
    // OnFire: the form's own events, no caller word.
    state(&mut e).disabled_events.insert(ITEM_BASE, 0x40_0000);
    assert_eq!(
        site(&mut e, 0x005a_c763, &[ITEM_BASE, list, 0x40_0000]),
        Some(1)
    );
    // An item event from a form that is not a reference: the list's reference.
    assert_eq!(site(&mut e, 0x005a_c763, &[ITEM_BASE, list, 1]), None);
    // Enabling: OnHit then OnLoad; the last clears the entry, the flag and the user.
    run(
        &mut e,
        SET_SCRIPT_EVENT_DISABLED,
        0,
        &[form_arg(R_REF), s(b"OnHit"), int_arg(0)],
    );
    assert_eq!(users(&e, Hook::MarkScriptEvent), 1);
    run(
        &mut e,
        SET_SCRIPT_EVENT_DISABLED,
        0,
        &[form_arg(R_REF), s(b"OnLoad OnActivate"), int_arg(0)],
    );
    assert_eq!(users(&e, Hook::MarkScriptEvent), 0);
    assert_eq!(users(&e, Hook::DoActivate), 0);
    assert_eq!(e.mem.u8(REF + 6), 0);
    // Unknown names, or a form with no script: nothing.
    run(
        &mut e,
        SET_SCRIPT_EVENT_DISABLED,
        0,
        &[form_arg(R_REF), s(b"OnNothing"), int_arg(1)],
    );
    run(
        &mut e,
        SET_SCRIPT_EVENT_DISABLED,
        0,
        &[form_arg(R_QUEST), s(b"OnHit"), int_arg(1)],
    );
    e.mem.set_u32(scriptable + 4, 0);
    run(
        &mut e,
        SET_SCRIPT_EVENT_DISABLED,
        0,
        &[form_arg(R_REF), s(b"OnHit"), int_arg(1)],
    );
    assert_eq!(users(&e, Hook::MarkScriptEvent), 0);
}

/// The ScriptWait seams: which wait a run is, and a waiting block counting down and resuming.
#[test]
fn script_wait_seams() {
    let mut e = world();
    // REF waits two passes of the block at offset 0x40, to resume 0x10 into it.
    let wait = ScriptWait {
        owner: REF,
        ref_id: 0x0100_0001,
        iterations: 2,
        block_offset: 0x40,
        saved_offset: 0x10,
        saved_if_stack: [7; 12],
    };
    state(&mut e).script_waits.insert(REF, wait);
    e.mem.set_u8(REF + 5, SCRIPT_ON_WAIT);
    // On the AI threads too (package scripts): the runner's and the block's seams answer while
    // another engine holds JIP's shared slot.
    held(&mut e, |e| site(e, 0x005e_0ec9, &[OBJ_SCRIPT, REF]));
    assert_eq!(current_wait(), Some(REF));
    // Another thread's run start leaves this thread's wait (JIP's one global would not).
    std::thread::spawn(crate::world_hooks::clear_current_wait)
        .join()
        .expect("ran");
    assert_eq!(current_wait(), Some(REF), "per thread");
    e.mem.set_u16(OBJ_SCRIPT + 0x28, 0x100);
    site(&mut e, 0x005e_0ec9, &[OBJ_SCRIPT, REF]);
    assert_eq!(current_wait(), None, "an effect script");
    e.mem.set_u16(OBJ_SCRIPT + 0x28, 1);
    e.mem.set_u32(OBJ_SCRIPT + 0x40, REF);
    site(&mut e, 0x005e_0ec9, &[OBJ_SCRIPT, 0]);
    assert_eq!(current_wait(), Some(REF), "a quest script's quest");
    e.mem.set_u8(REF + 5, 0);
    site(&mut e, 0x005e_0ec9, &[OBJ_SCRIPT, 0]);
    assert_eq!(current_wait(), None, "not flagged");
    e.mem.set_u8(REF + 5, SCRIPT_ON_WAIT);
    site(&mut e, 0x005e_0ec9, &[OBJ_SCRIPT, 0]);
    let eval = |e: &mut Engine, kind: u32, cursor: u32, high: u32, action: u32| {
        e.mem.set_u32(LENGTH, 100);
        let r = held(e, |e| {
            site(
                e,
                0x005e_1716,
                &[RUNNER, kind, cursor, high, action, LENGTH, 0x20],
            )
        });
        assert_eq!(r, Some(1));
        e.mem.u32(LENGTH)
    };
    // Another block: the exe's test (applies: kept; not: skipped).
    assert_eq!(eval(&mut e, 0, 0x50, 0x3ff0_0000, 0), 100);
    assert_eq!(eval(&mut e, 0, 0x50, 0, 0), 0x84);
    assert_eq!(
        eval(&mut e, 0, 0x50, 0x3ff0_0000, 1),
        0x84,
        "the action list"
    );
    // The waiting block: a GameMode pass that does not apply skips without counting.
    assert_eq!(eval(&mut e, 0, 0x40, 0, 0), 0x84);
    assert_eq!(state(&mut e).script_waits[&REF].iterations, 2);
    // One that applies counts down and skips.
    assert_eq!(eval(&mut e, 0, 0x40, 0x3ff0_0000, 0), 0x84);
    assert_eq!(state(&mut e).script_waits[&REF].iterations, 1);
    // An event block (type above 1) counts even when it does not apply: resumes.
    assert_eq!(eval(&mut e, 2, 0x40, 0, 0), 100 + 0x10);
    for i in 0..12 {
        assert_eq!(e.mem.u32(RUNNER + 0x20 + 4 * i), 7);
    }
    assert_eq!(current_wait(), None);
    assert_eq!(e.mem.u8(REF + 5), 0);
    // The pre-load: the waits go with the hook, and this thread's current wait (JIP nulls
    // `s_scriptWaitInfo`).
    e.mem.set_u8(REF + 5, SCRIPT_ON_WAIT);
    site(&mut e, 0x005e_0ec9, &[OBJ_SCRIPT, REF]);
    assert_eq!(current_wait(), Some(REF));
    e.set_hook(Hook::ScriptWait, true);
    crate::world_hooks::pre_load(&mut e);
    assert_eq!(users(&e, Hook::ScriptWait), 0);
    assert!(state(&mut e).script_waits.is_empty());
    assert_eq!(current_wait(), None, "cleared on the loading thread");
}

/// The block iterator over a compiled script: `scn`, then blocks; the last block before the
/// length.
#[test]
fn block_iterator_walks_to_the_offset() {
    let e = world();
    let data = AREA + 0xc000;
    // scn (4 bytes), Begin GameMode (opcode 0x10, size 6: type 0, length 0x0C, 2 more bytes),
    // 0x0C bytes of lines, End (0x11); then a second block of type 0xD.
    e.mem.set_u32(data, 0x1d);
    let mut at = 4;
    for kind in [0u16, 0xd] {
        e.mem.set_u16(data + at, 0x10);
        e.mem.set_u16(data + at + 2, 6);
        e.mem.set_u16(data + at + 4, kind);
        e.mem.set_u16(data + at + 6, 0x0c);
        at += 4 + 6 + 0x0c - 6 + 6;
        e.mem.set_u32(data + at - 4, 0x11);
    }
    let b = block_iterator(&e, data, 8);
    assert_eq!(b.type_at, data + 8);
    assert_eq!(b.next_op, 8 + 6);
    let b = block_iterator(&e, data, 40);
    assert_eq!(b.type_at, data + 30, "the second block");
    assert_eq!((b.next_op, b.next_block), (36, 48));
    assert_eq!(at, 48);
    assert_eq!(
        block_iterator(&e, data, 0x40).type_at,
        0,
        "past the last block: none"
    );
    e.mem.set_u32(data, 0x1e);
    assert_eq!(block_iterator(&e, data, 8).type_at, 0, "no scn");
}

/// Before a game loads: the parts of this step only (the others are their modules').
#[test]
fn pre_load_leaves_the_events() {
    let mut e = world();
    run(&mut e, SET_ON_NOTE_ADDED, 0, &[form_arg(R_S1), int_arg(1)]);
    crate::world_hooks::pre_load(&mut e);
    assert_eq!(users(&e, Hook::AddNote), 1);
}

thread_local! {
    /// The handler words `EvaluateLine`'s stand-in hands `ScriptWait`.
    static WAIT_WORDS: std::cell::Cell<[u32; 8]> = const { std::cell::Cell::new([0; 8]) };
    static WAIT_FROM_COMMAND: std::cell::Cell<bool> = const { std::cell::Cell::new(true) };
}

/// `EvaluateLine`'s stand-in under xNVSE's wrap (which records the line): its command handler call
/// (the seam xNVSE notes it at, unless told not to), then `ScriptWait` through its record.
fn evaluate_line_stand_in(e: &mut Engine, _: &[u32]) -> Ret {
    let record = nvse::table::record_of(e, SCRIPT_WAIT);
    if WAIT_FROM_COMMAND.with(|c| c.get()) {
        e.site(0x005e_2329, &[SCRIPT_WAIT, record, RESULT]);
    }
    let at = crate::test_support::execute_of(e, SCRIPT_WAIT);
    let words = WAIT_WORDS.with(|c| c.get());
    e.call(at, &words)
}

/// ScriptWait from a script's block: the line past the command, the wait (the block's offset,
/// the line to resume at, the runner's `if` stack), the owner flagged, the hook on, and `Run`'s
/// length word set to jump to the block's end. Not from the command call, a count of 0, or an
/// effect script: nothing.
#[test]
fn script_wait_command() {
    let mut e = world();
    // A thread's engine, made right after install: it reads the waits the command writes.
    let mut t = Engine::new_untyped_for_test();
    e.share_slots_for_test(&mut t);
    engine_core::extend::release_held_for_test();
    for addr in [0x005e_0d20u32, 0x005e_1160, 0x005e_13a0] {
        e.register(addr, |_, _| Ret::default());
    }
    e.register(0x005e_1550, evaluate_line_stand_in);
    nvse::context::install(&mut e);
    // `scn`; Begin (0x10, size 6, type 0, block length 15 at +10); the ScriptWait line at 14
    // (opcode, length 7, one argument: `n` 3); End (0x11) at 25.
    let data = AREA + 0xc800;
    e.mem.set_u32(data, 0x1d);
    e.mem.set_u16(data + 4, 0x10);
    e.mem.set_u16(data + 6, 6);
    e.mem.set_u16(data + 8, 0);
    e.mem.set_u16(data + 10, 15);
    e.mem.set_u16(data + 14, SCRIPT_WAIT as u16);
    e.mem.set_u16(data + 16, 7);
    e.mem.set_u16(data + 18, 1);
    e.mem.set_u8(data + 20, b'n');
    e.mem.set_u32(data + 21, 3);
    e.mem.set_u32(data + 25, 0x11);
    e.mem.set_u32(CALLER + 0xc, 0x0100_0100);
    for i in 0..12 {
        e.mem.set_u32(RUNNER + 0x20 + 4 * i, 0x50 + i);
    }
    let go = |e: &mut Engine, count: i32| {
        let mut words = line(e, SCRIPT_WAIT, &[int_arg(count)]);
        words[1] = data;
        words[2] = REF;
        e.mem.set_u32(words[7], 18);
        e.mem.set_u8(data + 20, b'n');
        e.mem.set_u32(data + 21, count as u32);
        WAIT_WORDS.with(|c| c.set(words));
        e.mem.set_u32(LENGTH, 0x77);
        // EvaluateLine's words: the runner, script, command, reference, length, position, skip,
        // line, parse only, action list.
        e.call(
            0x005e_1550,
            &[RUNNER, CALLER, SCRIPT_WAIT, REF, LENGTH, 18, 0, 1, 0, 0],
        );
        e.mem.u32(words[7])
    };
    WAIT_FROM_COMMAND.with(|c| c.set(false));
    assert_eq!(go(&mut e, 3), 18, "not the command call: nothing");
    WAIT_FROM_COMMAND.with(|c| c.set(true));
    assert_eq!(go(&mut e, 0), 18, "a count of 0");
    e.mem.set_u8(CALLER + 0x29, 1);
    assert_eq!(go(&mut e, 3), 18, "an effect script");
    e.mem.set_u8(CALLER + 0x29, 0);
    assert!(state(&mut e).script_waits.is_empty());
    assert_eq!(users(&e, Hook::ScriptWait), 0);
    assert_eq!(go(&mut e, 3), 25, "past the command");
    let wait = state(&mut e).script_waits[&REF];
    assert_eq!(
        read(&mut t).script_waits.get(&REF),
        Some(&wait),
        "a thread's engine reads it"
    );
    assert_eq!(
        (
            wait.owner,
            wait.ref_id,
            wait.iterations,
            wait.block_offset,
            wait.saved_offset
        ),
        (REF, 0x0100_0001, 3, 14, 11)
    );
    assert_eq!(
        wait.saved_if_stack,
        std::array::from_fn(|i| 0x50 + i as u32)
    );
    assert_eq!(e.mem.u8(REF + 5), SCRIPT_ON_WAIT);
    assert_eq!(users(&e, Hook::ScriptWait), 1);
    assert_eq!(
        e.mem.u32(LENGTH),
        29 - 4 - 18,
        "Run jumps to the block's end"
    );
    // A quest script waits for its quest.
    e.mem.set_u8(CALLER + 0x28, 1);
    e.mem.set_u32(CALLER + 0x40, QUEST_FORM);
    go(&mut e, 2);
    assert_eq!(state(&mut e).script_waits[&QUEST_FORM].iterations, 2);
    crate::world_hooks::pre_load(&mut e);
    assert!(read(&mut t).script_waits.is_empty(), "and their removal");
    engine_core::extend::release_held_for_test();
}

/// A thread's engine (`share_slots_for_test`, as `ExtendState::for_thread`) reads the names the
/// commands write and remove on the main engine: the world hooks' state is one run-wide slot.
#[test]
fn a_threads_engine_reads_the_names_the_commands_write() {
    let mut e = world();
    let mut t = Engine::new_untyped_for_test();
    e.share_slots_for_test(&mut t);
    engine_core::extend::release_held_for_test();
    run(&mut e, SET_REF_NAME, REF, &[s(b"Bob")]);
    assert!(read(&mut t).ref_names.contains_key(&REF), "written");
    run(&mut e, SET_REF_NAME, REF, &[s(b"")]);
    assert!(!read(&mut t).ref_names.contains_key(&REF), "removed");
    engine_core::extend::release_held_for_test();
}

/// One engine run on two OS threads one at a time, as stepped threads hand it over (the main
/// thread waits on the channel while the other runs it).
struct OneAtATime(*mut Engine);
// SAFETY: the two threads use the engine strictly in turn, ordered by the channels below.
unsafe impl Send for OneAtATime {}

/// A wait a thread took before a load counts nothing after it, also when the same owner waits
/// again at the same block: JIP's pre-load nulls its one `s_scriptWaitInfo` for every thread;
/// the port's pre-load (on the loading thread) raises the waits' generation, and the other
/// thread's older value counts as none and is cleared.
#[test]
fn a_wait_taken_before_a_load_counts_nothing_after_it() {
    let mut e = world();
    let wait = ScriptWait {
        owner: REF,
        ref_id: 0x0100_0001,
        iterations: 2,
        block_offset: 0x40,
        saved_offset: 0x10,
        saved_if_stack: [7; 12],
    };
    state(&mut e).script_waits.insert(REF, wait);
    e.mem.set_u8(REF + 5, SCRIPT_ON_WAIT);
    engine_core::extend::release_held_for_test();
    let raw: *mut Engine = &mut e;
    let other = OneAtATime(raw);
    let (go, wait_go) = std::sync::mpsc::channel::<()>();
    let (tell, heard) = std::sync::mpsc::channel::<(u32, Option<u32>)>();
    std::thread::scope(|scope| {
        scope.spawn(move || {
            let other = other;
            // SAFETY: the main thread does not touch the engine until `tell` (see OneAtATime).
            let e = unsafe { &mut *other.0 };
            site(e, 0x005e_0ec9, &[OBJ_SCRIPT, REF]);
            tell.send((0, current_wait())).expect("told");
            wait_go.recv().expect("go");
            e.mem.set_u32(LENGTH, 100);
            site(
                e,
                0x005e_1716,
                &[RUNNER, 0, 0x40, 0x3ff0_0000, 0, LENGTH, 0x20],
            );
            tell.send((e.mem.u32(LENGTH), current_wait()))
                .expect("told");
        });
        assert_eq!(heard.recv().expect("heard").1, Some(REF), "it waits");
        // SAFETY: the other thread waits on `go` and does not touch the engine meanwhile.
        let e = unsafe { &mut *raw };
        e.set_hook(Hook::ScriptWait, true);
        crate::world_hooks::pre_load(e);
        // The same owner waits again at the same block.
        state(e).script_waits.insert(REF, wait);
        e.mem.set_u8(REF + 5, SCRIPT_ON_WAIT);
        go.send(()).expect("sent");
        let (length, now) = heard.recv().expect("heard");
        assert_eq!(length, 100, "the block applies: not counted, not skipped");
        assert_eq!(now, None, "the old wait is cleared");
    });
    assert_eq!(
        state(&mut e).script_waits[&REF].iterations,
        2,
        "nothing counted"
    );
}
