//! Tests of [`super`] (Tweaks' co-save section).

use super::*;
use std::sync::atomic::{AtomicU32, Ordering::Relaxed};

/// The rounds the player carries of the special round and of the default one.
static SPECIAL: AtomicU32 = AtomicU32::new(0);
static DEFAULT: AtomicU32 = AtomicU32::new(0);
/// One test at a time: the counts above are shared.
static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

struct World {
    e: Engine,
    ammo_info: u32,
    weapon_info: u32,
    special: u32,
    default: u32,
}

/// The player (`0x11DEA3C`) whose process's ammunition entry holds 30 of the default round and
/// whose weapon entry is a gun with a clip of 12 (`+0xB0`) and a clip mod (slot 1: effect 2, +6);
/// the special round (type 0x29, form id 0x0001_4400) and the default one (0x0001_4500); the form
/// lookup (`004839c0`), the extra-data lookup (`00410220`: the container changes, and the weapon
/// entry's mod flags `mod_flags`) and the inventory count (`004c8f30`) stand in. The save's file
/// list maps index 0 to 0.
fn world(mod_flags: Option<u8>) -> World {
    let mut e = Engine::new();
    e.map(0x011d_e000, 0x1000);
    // No data handler (`011c3f2c` 0): no files loaded, as the co-save lists none.
    e.map(0x011c_3000, 0x1000);
    e.ext::<nvse::Nvse>().saveload.preload_mods = vec![0];
    let player = e.mem.alloc(0x100);
    e.set_global(PLAYER, player);
    let process = e.mem.alloc(0x200);
    e.mem.set_u32(player + 0x68, process);
    let special = e.mem.alloc(0x20);
    e.mem.set_u8(special + 4, 0x29);
    e.mem.set_u32(special + 0xc, 0x0001_4400);
    let default = e.mem.alloc(0x20);
    e.mem.set_u8(default + 4, 0x29);
    e.mem.set_u32(default + 0xc, 0x0001_4500);
    let ammo_info = e.mem.alloc(0x10);
    e.mem.set_u32(ammo_info + 4, 30);
    e.mem.set_u32(ammo_info + 8, default);
    e.mem.set_u32(process + 0x118, ammo_info);
    let weapon = e.mem.alloc(0x200);
    e.mem.set_u8(weapon + 0xb0, 12);
    e.mem.set_u32(weapon + 0x184, 2);
    e.mem.set_f32(weapon + 0x190, 6.0);
    let weapon_info = e.mem.alloc(0x10);
    e.mem.set_u32(weapon_info + 8, weapon);
    let lists = e.mem.alloc(8);
    let first = e.mem.alloc(0x20);
    e.mem.set_u32(lists, first);
    e.mem.set_u32(weapon_info, lists);
    e.mem.set_u32(process + 0x114, weapon_info);
    let changes = e.mem.alloc(0x10);
    e.mem.set_u32(changes + 0xc, 0x4100_0000);
    let flags = e.mem.alloc(0x10);
    e.mem.set_u8(flags + 0xc, mod_flags.unwrap_or(0));
    let has_flags = mod_flags.is_some();
    // xNVSE's kExtraData_ContainerChanges (0x15) and kExtraData_WeaponModFlags (0x8D) as numbers.
    e.register_double(0x0041_0220, move |_, a| Ret {
        eax: match a[1] {
            0x15 => changes,
            0x8d if a[0] == first && has_flags => flags,
            _ => 0,
        },
        ..Ret::default()
    });
    e.register_double(0x004c_8f30, move |_, a| Ret {
        eax: if a[1] == special {
            SPECIAL.load(Relaxed)
        } else {
            DEFAULT.load(Relaxed)
        },
        ..Ret::default()
    });
    // The form ids: the special round as 0x0001_4400 (and as 0x0301_4400, its file at index 3 in
    // a load order where it moved), a weapon (type 0x28) as 0x0001_4600.
    let not_ammo = e.mem.alloc(0x20);
    e.mem.set_u8(not_ammo + 4, 0x28);
    e.register_double(0x0048_39c0, move |_, a| Ret {
        eax: match a[0] {
            0x0001_4400 | 0x0301_4400 => special,
            0x0001_4600 => not_ammo,
            _ => 0,
        },
        ..Ret::default()
    });
    e.set_stewie(engine_core::extend::stewie::LOAD_GAME_AMMO_COUNT, true);
    World {
        e,
        ammo_info,
        weapon_info,
        special,
        default,
    }
}

fn keep_count(e: &mut Engine, count: Option<u32>) {
    let kept = e.stewie_kept(engine_core::extend::stewie::kept::SAVED_AMMO_COUNT, 4);
    e.mem.set_u32(kept, count.map_or(0, |c| c + 1));
}

fn entry(w: &World) -> (u32, u32) {
    (w.e.mem.u32(w.ammo_info + 4), w.e.mem.u32(w.ammo_info + 8))
}

/// The save writes Tweaks' record: `AMID` (stored as the bytes "DIMA"), version 0, the current
/// ammunition's form id; with row S0089.07 off, none.
#[test]
fn the_save_writes_the_current_ammunitions_form_id() {
    let _one = LOCK.lock().unwrap_or_else(|p| p.into_inner());
    let mut w = world(None);
    let mut records = Records::default();
    save(&mut w.e, &mut records);
    assert_eq!(records.chunks.len(), 1);
    assert_eq!(
        (records.chunks[0].kind, records.chunks[0].version),
        (AMMO_ID, 0)
    );
    assert_eq!(AMMO_ID.to_le_bytes(), *b"DIMA");
    assert_eq!(records.chunks[0].data, 0x0001_4500u32.to_le_bytes());
    w.e.set_stewie(engine_core::extend::stewie::LOAD_GAME_AMMO_COUNT, false);
    let mut records = Records::default();
    save(&mut w.e, &mut records);
    assert!(records.chunks.is_empty());
}

/// A case: the clip mod flags, the rounds carried, the saved count, whether there is a weapon, and
/// the entry after (its count, whether the special round is its type).
type Case = (Option<u8>, u32, Option<u32>, bool, (u32, bool));

/// The `AMID` step: a special round chosen without a hotkey, saved with 7 loaded and 20 carried,
/// loads as 7 of that round (where S0089.07 had left the default round); with only 5 carried, 5;
/// a save without a count: the rounds carried capped at the clip (12, or 18 with the clip mod
/// fitted); the round not carried: the entry as it was; no weapon and no count: 0.
#[test]
fn the_load_restores_the_chosen_round_and_its_count() {
    let _one = LOCK.lock().unwrap_or_else(|p| p.into_inner());
    let cases: [Case; 6] = [
        (None, 20, Some(7), true, (7, true)),
        (None, 5, Some(7), true, (5, true)),
        (None, 20, None, true, (12, true)),
        (Some(0b010), 20, None, true, (18, true)),
        (None, 0, Some(7), true, (30, false)),
        (None, 20, None, false, (0, true)),
    ];
    for (flags, carried, count, weapon, (rounds, special)) in cases {
        let mut w = world(flags);
        SPECIAL.store(carried, Relaxed);
        keep_count(&mut w.e, count);
        if !weapon {
            w.e.mem.set_u32(w.weapon_info + 8, 0);
        }
        restore_ammo(&mut w.e, 0x0001_4400);
        let kind = if special { w.special } else { w.default };
        assert_eq!(
            entry(&w),
            (rounds, kind),
            "carried {carried} count {count:?}"
        );
    }
}

/// A co-save section in Tweaks' own format (xNVSE's file, opcode base 0x2132): its `AMID` is
/// applied after the load and its other records (here a `PINH`) are written back on the next save
/// as they were; the PreLoadGame message clears the saved count and the kept records.
#[test]
fn a_tweaks_section_is_read_and_its_other_records_kept() {
    let _one = LOCK.lock().unwrap_or_else(|p| p.into_inner());
    let mut w = world(None);
    SPECIAL.store(20, Relaxed);
    keep_count(&mut w.e, Some(7));
    let file = nvse::cosave::write(&[nvse::cosave::Plugin {
        opcode_base: crate::OPCODE_BASE,
        chunks: vec![
            nvse::cosave::Chunk {
                kind: tag(b"PINH"),
                version: 0,
                data: b"75\0".to_vec(),
            },
            nvse::cosave::Chunk {
                kind: AMMO_ID,
                version: 0,
                data: 0x0001_4400u32.to_le_bytes().to_vec(),
            },
        ],
    }]);
    let mut r = Reader::open(&file).unwrap();
    let plugin = r.next_plugin().unwrap().unwrap();
    assert_eq!(plugin.opcode_base, crate::OPCODE_BASE);
    load(&mut w.e, &mut r).unwrap();
    assert_eq!(entry(&w), (7, w.special));
    let mut records = Records::default();
    save(&mut w.e, &mut records);
    let kinds: Vec<u32> = records.chunks.iter().map(|c| c.kind).collect();
    assert_eq!(kinds, vec![AMMO_ID, tag(b"PINH")]);
    assert_eq!(records.chunks[0].data, 0x0001_4400u32.to_le_bytes());
    assert_eq!(records.chunks[1].data, b"75\0");
    let message = nvse::plugins::Message {
        kind: nvse::plugins::msg::PRE_LOAD_GAME,
        data: 0,
        len: 0,
    };
    listen(&mut w.e, &message);
    let kept =
        w.e.stewie_kept(engine_core::extend::stewie::kept::SAVED_AMMO_COUNT, 4);
    assert_eq!(w.e.mem.u32(kept), 0);
    assert!(w.e.ext::<crate::Stewie>().kept.records.is_empty());
}

/// The record's checks before it is applied: a form id whose file is no longer loaded (the save's
/// index 1 maps to nothing) leaves the entry as it was; a file that moved in the load order (the
/// save's index 1 is today's 3) resolves to the moved id; a looked-up form that is no ammunition
/// (a weapon, type 0x28) leaves the entry as it was.
#[test]
fn the_record_needs_a_loaded_ammunition_form() {
    let _one = LOCK.lock().unwrap_or_else(|p| p.into_inner());
    let mut w = world(None);
    SPECIAL.store(20, Relaxed);
    keep_count(&mut w.e, Some(7));
    restore_ammo(&mut w.e, 0x0101_4400);
    assert_eq!(entry(&w), (30, w.default), "index 1 not loaded");

    let mut w = world(None);
    w.e.ext::<nvse::Nvse>().saveload.preload_mods = vec![0, 3];
    SPECIAL.store(20, Relaxed);
    keep_count(&mut w.e, Some(7));
    restore_ammo(&mut w.e, 0x0101_4400);
    assert_eq!(entry(&w), (7, w.special), "index 1 is today's 3");

    let mut w = world(None);
    SPECIAL.store(20, Relaxed);
    DEFAULT.store(20, Relaxed);
    keep_count(&mut w.e, Some(7));
    restore_ammo(&mut w.e, 0x0001_4600);
    assert_eq!(entry(&w), (30, w.default), "not ammunition");
}
