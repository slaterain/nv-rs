//! Group GF's game patches: G39 and G40 (JIP's own: the NPC skeleton model), G07, G16, G20, G83
//! (fixes); part 2: G110, G57, G130, D01 (fixes); G48 (JIP's own: setting a form's type).

use super::*;
use crate::test_support::engine;

const AREA_BASE: u32 = 0x0536_0000;
/// An NPC (its model at `+0xDC`), the model's characters, an item, the player and its base.
const NPC: u32 = AREA_BASE;
const MODEL: u32 = NPC + 0xdc;
const OWN_PATH: u32 = AREA_BASE + 0x400;
const ITEM: u32 = AREA_BASE + 0x800;
const PLAYER: u32 = AREA_BASE + 0xc00;
const PLAYER_BASE: u32 = AREA_BASE + 0xd00;
/// The settings' texts (distinct, so a swap shows).
const FEMALE_TEXT: u32 = 0x0abc_0010;
const MALE_TEXT: u32 = 0x0def_0020;
/// What the exe's `TESModel::SetModelPath` (`0x489100`) held in the slot.
const SET_MODEL_PATH: u32 = 0x0048_9100;
/// The model vtable and its `GetModel` slot word (`0x104A1B8`).
const MODEL_VTABLE: u32 = 0x0104_a1a4;

fn world_as(installed: bool) -> Engine {
    let mut e = engine();
    e.map(AREA_BASE, 0x1000);
    e.map(THE_PLAYER & !0xfff, 0x1000);
    e.mem.set_u32(FEMALE_MODEL_VALUE, FEMALE_TEXT);
    e.mem.set_u32(MALE_MODEL_VALUE, MALE_TEXT);
    // The exe's vtable words: GetModel at +0x14, SetModelPath at +0x18.
    e.mem.set_u32(MODEL_VTABLE + 0x14, 0x0060_1c30);
    e.mem.set_u32(NPC_SET_MODEL_SLOT, SET_MODEL_PATH);
    e.mem.set_u32(MODEL, MODEL_VTABLE);
    // The NPC: form type 0x2A at +4 with JIP's flag bytes beside it, female (+0x34 bit 0).
    e.mem.set_u32(NPC + 4, 0x0040_402a);
    e.mem.set_u8(NPC + 0x34, 1);
    e.mem.set_u32(THE_PLAYER, PLAYER);
    e.mem.set_u32(PLAYER + 0x20, PLAYER_BASE);
    crate::install_as(&mut e, installed);
    e
}

fn fixes_world() -> Engine {
    let mut e = world_as(true);
    // The group's installer (what the row puts in; the framework's tests hold every area to the
    // row, and the row reads the player's jip_nvse.ini through the platform).
    install_fixes(&mut e);
    e
}

/// While JIP answers "not installed": no seam or wrap of the group has JIP's handler, row or
/// not; `00601c30` answers the exe's (the female setting for the female NPC) and the set slot
/// keeps `0x489100`.
#[test]
fn not_installed_nothing_is_put_in() {
    let mut e = world_as(false);
    super::super::set_row(&mut e, true);
    for addr in AREA.fix_sites.iter().chain(AREA.jip_sites) {
        assert!(!e.site_owners(*addr).contains(&Owner::Jip), "{addr:08x}");
        assert_eq!(e.site(*addr, &[ITEM, 0]), None, "{addr:08x}");
    }
    for addr in AREA.fix_wraps.iter().chain(AREA.jip_wraps) {
        assert!(e.wraps_at(*addr).is_empty(), "{addr:08x}");
    }
    assert_eq!(e.call(NPC_MODEL.addr(), &[MODEL]).eax, FEMALE_TEXT);
    assert_eq!(e.mem.u32(NPC_SET_MODEL_SLOT), SET_MODEL_PATH);
}

/// Installed with the row off: G39's wrap and G40's slot (JIP's own) are in, the fixes are not;
/// the group's fix installer (what the row runs) puts the fixes in and changes nothing of G39 and G40.
#[test]
fn installed_row_off_puts_only_jips_own_in() {
    let mut e = world_as(true);
    super::super::set_row(&mut e, false);
    assert_eq!(e.wraps_at(NPC_MODEL.addr()), [Owner::Jip]);
    assert_eq!(e.mem.u32(NPC_SET_MODEL_SLOT), SET_MODEL_STUB);
    for addr in AREA.fix_sites {
        assert!(e.site_owners(*addr).is_empty(), "{addr:08x}");
        assert_eq!(e.site(*addr, &[ITEM, 0]), None, "{addr:08x}");
    }
    install_fixes(&mut e);
    for addr in AREA.fix_sites {
        assert_eq!(
            e.site_owners(*addr).to_vec(),
            vec![Owner::Jip],
            "{addr:08x}"
        );
    }
    assert_eq!(e.wraps_at(NPC_MODEL.addr()), [Owner::Jip]);
    assert_eq!(e.mem.u32(NPC_SET_MODEL_SLOT), SET_MODEL_STUB);
}

/// G39 through the NPC model's vtable (slot `+0x14`) and by address: the model's own path when
/// set; else, with the word at the NPC's `+4` even (the form type 0x2A), the male setting's text
/// although the NPC is female; with that word odd, the female setting's text although the NPC
/// is male (JIP's test reads the word, not the sex). The exe's function is not called.
#[test]
fn g39_answers_as_jips_bytes() {
    let mut e = world_as(true);
    let get = |e: &mut Engine| e.vcall(MODEL, 0x14, &[]).eax;
    assert_eq!(get(&mut e), MALE_TEXT);
    e.mem.set_u32(MODEL + 4, OWN_PATH);
    assert_eq!(get(&mut e), OWN_PATH);
    assert_eq!(e.call(NPC_MODEL.addr(), &[MODEL]).eax, OWN_PATH);
    e.mem.set_u32(MODEL + 4, 0);
    e.mem.set_u8(NPC + 0x34, 0);
    e.mem.set_u32(NPC + 4, 0x0040_402b);
    assert_eq!(get(&mut e), FEMALE_TEXT);
    // Only bit 0 of the word counts: the other bits set, bit 0 clear.
    e.mem.set_u32(NPC + 4, 0xffff_fffe);
    e.mem.set_u8(NPC + 0x34, 0xff);
    assert_eq!(get(&mut e), MALE_TEXT);
    // The NPC's word at +0 (the model's -0xDC) is not the one read.
    e.mem.set_u32(NPC, 1);
    assert_eq!(get(&mut e), MALE_TEXT);
}

/// G40: once installed, the NPC model's set slot (`+0x18`) is the exe's stub `0x401290`, so a
/// model path set through the vtable (what loading an NPC record does) reaches the stub and is
/// not stored; `0x489100` is not called.
#[test]
fn g40_the_npc_model_path_is_not_stored() {
    let mut e = world_as(true);
    e.map(LOG & !0xfff, 0x1000);
    e.register(SET_MODEL_STUB, |e, a| {
        e.mem.set_u32(LOG, a[0]);
        e.mem.set_u32(LOG + 4, a[1]);
        Ret {
            eax: 1,
            ..Ret::default()
        }
    });
    e.register(SET_MODEL_PATH, |e, a| {
        e.mem.set_u32(a[0] + 4, a[1]);
        Ret::default()
    });
    e.vcall(MODEL, 0x18, &[OWN_PATH]);
    assert_eq!((e.mem.u32(LOG), e.mem.u32(LOG + 4)), (MODEL, OWN_PATH));
    assert_eq!(e.mem.u32(MODEL + 4), 0);
    // Not installed: the exe's setter stores it.
    let mut e = world_as(false);
    e.register(SET_MODEL_PATH, |e, a| {
        e.mem.set_u32(a[0] + 4, a[1]);
        Ret::default()
    });
    e.vcall(MODEL, 0x18, &[OWN_PATH]);
    assert_eq!(e.mem.u32(MODEL + 4), OWN_PATH);
}

/// Where the G40 test's stand-in writes its words.
const LOG: u32 = 0x0537_0000;

/// G07, G16, G20: JIP's jumps, taken on any call.
#[test]
fn jumps_are_taken() {
    let mut e = fixes_world();
    for slot in [
        GRASS_EDGE,
        PLAYER_CASING,
        FIRE_NO_PROCESS,
        PLAYER_PERK_ENTRY,
    ] {
        assert_eq!(e.site(slot.addr(), &[]), Some(1), "{:08x}", slot.addr());
    }
}

/// An armour with distinct icon words and its female icon's length (`+0x110`, a word: its low
/// byte 0 so a byte read shows).
fn armour(e: &mut Engine, female_length: u16) {
    // The type byte 0x18 with JIP's flag bytes beside it.
    e.mem.set_u32(ITEM + 4, 0x2040_6018);
    e.mem.set_u32(ITEM + 0x100, 0x0aaa_0100);
    e.mem.set_u32(ITEM + 0x10c, 0x0bbb_010c);
    e.mem.set_u16(ITEM + 0x110, female_length);
    e.mem.set_u16(ITEM + 0x112, 0xffff);
}

/// G83, case by case: not an armour (the neighbouring types) 0; an armour: the male icon for a
/// male player; the female icon for a female player with a female icon; the male icon for a
/// female player without one; only bit 0 of the base's flags counts; no player or no base: the
/// male icon; no item: 0. The owner word is not read.
#[test]
fn g83_answers_as_jip() {
    let mut e = fixes_world();
    armour(&mut e, 0x0100);
    for kind in [0x17u8, 0x19] {
        e.mem.set_u8(ITEM + 4, kind);
        assert_eq!(icon_path(&mut e, &[ITEM, PLAYER]), Some(0), "{kind:02x}");
    }
    e.mem.set_u8(ITEM + 4, 0x18);
    e.mem.set_u8(PLAYER_BASE + 0x34, 0xfe);
    assert_eq!(icon_path(&mut e, &[ITEM, PLAYER]), Some(0x0aaa_0100));
    e.mem.set_u8(PLAYER_BASE + 0x34, 1);
    assert_eq!(icon_path(&mut e, &[ITEM, 0]), Some(0x0bbb_010c));
    assert_eq!(icon_path(&mut e, &[ITEM, 0x1234]), Some(0x0bbb_010c));
    armour(&mut e, 0);
    assert_eq!(icon_path(&mut e, &[ITEM, 0]), Some(0x0aaa_0100));
    armour(&mut e, 1);
    assert_eq!(icon_path(&mut e, &[ITEM, 0]), Some(0x0bbb_010c));
    e.mem.set_u32(PLAYER + 0x20, 0);
    assert_eq!(icon_path(&mut e, &[ITEM, 0]), Some(0x0aaa_0100));
    e.mem.set_u32(PLAYER + 0x20, PLAYER_BASE);
    e.mem.set_u32(THE_PLAYER, 0);
    assert_eq!(icon_path(&mut e, &[ITEM, 0]), Some(0x0aaa_0100));
    assert_eq!(icon_path(&mut e, &[0, 0]), Some(0));
}

/// Each handler at its own seam after the row is in: at G83's seam a form that is not an
/// armour answers 0 (the jumps' handler would answer 1), and an armour its icon; the three jump
/// seams answer 1 with words on which G83's handler would answer otherwise.
#[test]
fn every_handler_is_at_its_row() {
    let mut e = fixes_world();
    armour(&mut e, 0);
    assert_eq!(e.site(ICON_PATH.addr(), &[ITEM, 0]), Some(0x0aaa_0100));
    e.mem.set_u8(ITEM + 4, 0x19);
    assert_eq!(e.site(ICON_PATH.addr(), &[ITEM, 0]), Some(0));
    for slot in [
        GRASS_EDGE,
        PLAYER_CASING,
        FIRE_NO_PROCESS,
        PLAYER_PERK_ENTRY,
    ] {
        assert_eq!(
            e.site(slot.addr(), &[ITEM, 0]),
            Some(1),
            "{:08x}",
            slot.addr()
        );
    }
}

// ---- G48: TESForm::SetFormType ----

/// A form whose bytes `+4`..`+7` and the word at `+0x104` are pre-filled (what reused memory
/// can hold).
const G48_FORM: u32 = 0x0539_0000;

/// `004f15a0` called by address with `word` on the pre-filled form: the bytes `+4`..`+7`, the word
/// at `+0x104`, the byte past it and `EAX`.
fn g48_set(e: &mut Engine, word: u32) -> ([u8; 4], u32, u8, u32) {
    if !e.mem.is_mapped(G48_FORM) {
        e.map(G48_FORM, 0x1000);
    }
    e.mem.set_u32(G48_FORM + 4, 0xa5a5_a5a5);
    e.mem.set_u32(G48_FORM + 0x104, 0x5a5a_5a5a);
    e.mem.set_u8(G48_FORM + 0x108, 0x77);
    let eax = e.call(SET_FORM_TYPE.addr(), &[G48_FORM, word]).eax;
    let b = |i: u32| e.mem.u8(G48_FORM + 4 + i);
    (
        [b(0), b(1), b(2), b(3)],
        e.mem.u32(G48_FORM + 0x104),
        e.mem.u8(G48_FORM + 0x108),
        eax,
    )
}

/// G48 with JIP not installed (the row on or off): the exe's `004f15a0`, only the type byte is
/// stored and the form answered; the flag bytes and `+0x104` keep what they held, for an actor
/// type too.
#[test]
fn g48_not_installed_keeps_the_exes_byte_store() {
    for row in [false, true] {
        let mut e = world_as(false);
        super::super::set_row(&mut e, row);
        assert!(e.wraps_at(SET_FORM_TYPE.addr()).is_empty());
        for kind in [0x3bu32, 0x3c, 0x2a] {
            assert_eq!(
                g48_set(&mut e, kind),
                ([kind as u8, 0xa5, 0xa5, 0xa5], 0x5a5a_5a5a, 0x77, G48_FORM),
                "{kind:02x} row {row}"
            );
        }
    }
}

/// G48 with JIP installed (the row off, on, and on with an optional switch: JIP's own, in every
/// preset): the type stored as a whole word (JIP's flag bytes `+5`..`+7` cleared); for a
/// character (0x3B) or a creature (0x3C) the word at `+0x104` cleared too, nothing past it; any
/// other type (the neighbours 0x3A and 0x3D, an NPC base 0x2A) leaves `+0x104` alone. Only the
/// stack word's low byte counts. `EAX` is the type, as JIP's bytes leave it.
#[test]
fn g48_installed_clears_jips_flag_bytes() {
    use engine_core::extend::jip_patches;
    for (row, switch) in [
        (false, 0),
        (true, 0),
        (true, jip_patches::FO3_WPN_DEGRADATION),
    ] {
        let mut e = world_as(true);
        // The row on: the word's bit and the group's fixes (what the row puts in; the row
        // itself reads the player's jip_nvse.ini through the platform).
        if row {
            e.jip_patches_or(jip_patches::FIXES | switch);
            install_fixes(&mut e);
        }
        assert_eq!(e.wraps_at(SET_FORM_TYPE.addr()), [Owner::Jip]);
        for kind in [0x3bu32, 0x3c] {
            assert_eq!(
                g48_set(&mut e, kind),
                ([kind as u8, 0, 0, 0], 0, 0x77, kind),
                "{kind:02x} row {row}"
            );
        }
        for kind in [0x3au32, 0x3d, 0x2a] {
            assert_eq!(
                g48_set(&mut e, kind),
                ([kind as u8, 0, 0, 0], 0x5a5a_5a5a, 0x77, kind),
                "{kind:02x} row {row}"
            );
        }
        assert_eq!(
            g48_set(&mut e, 0x1234_563b),
            ([0x3b, 0, 0, 0], 0, 0x77, 0x3b),
            "the low byte only"
        );
        assert_eq!(
            g48_set(&mut e, 0x0000_3b2a),
            ([0x2a, 0, 0, 0], 0x5a5a_5a5a, 0x77, 0x2a),
            "the low byte only"
        );
    }
}
// ---- part 2: G110, G57, G130, D01 ----

const PART2: u32 = 0x0538_0000;
/// A faction, the other form, a reference, its cell, an armour, a file and the form map.
const FACTION: u32 = PART2;
const OTHER: u32 = PART2 + 0x100;
const REFERENCE: u32 = PART2 + 0x200;
const CELL: u32 = PART2 + 0x300;
const ARMOUR: u32 = PART2 + 0x400;
const FILE: u32 = PART2 + 0x500;
const MAP: u32 = PART2 + 0x600;
const BUCKETS: u32 = PART2 + 0x640;
const ITEMS: u32 = PART2 + 0x680;
/// Where the stand-ins write what they were asked (a count, then words).
const ASKED: u32 = PART2 + 0x800;
/// `TESObjectREFR`'s vtable and what its slot `+0x48` held (`TESForm::MarkAsModified`).
const REFR_VTABLE: u32 = 0x0102_f55c;
const MARK_AS_MODIFIED: u32 = 0x0048_4b60;

fn part2_world() -> Engine {
    let mut e = fixes_world();
    e.map(PART2, 0x1000);
    e
}

fn asked(e: &Engine) -> Vec<u32> {
    let n = e.mem.u32(ASKED);
    (0..n).map(|i| e.mem.u32(ASKED + 4 + 4 * i)).collect()
}

fn record(e: &mut Engine, words: &[u32]) {
    let mut n = e.mem.u32(ASKED);
    for w in words {
        e.mem.set_u32(ASKED + 4 + 4 * n, *w);
        n += 1;
    }
    e.mem.set_u32(ASKED, n);
}

fn answer(eax: u32) -> Ret {
    Ret {
        eax,
        ..Ret::default()
    }
}

/// G110: the faction's reaction form (`+0x24`) and the other form go to `0x48C1B0` (the entry's
/// group combat reaction), whose answer is the seam's; `0x5FD3B0`'s modifier is not asked.
#[test]
fn g110_answers_the_combat_reaction() {
    let mut e = part2_world();
    e.register(0x0048_c1b0, |e, a| {
        record(e, &[a[0], a[1]]);
        answer(2)
    });
    // The exe's modifier walk (`0x48BF50`, what `0x5FD3B0` calls) answers otherwise.
    e.register(0x0048_bf50, |_, _| answer(0x55));
    assert_eq!(e.site(FACTION_REACTION.addr(), &[FACTION, OTHER]), Some(2));
    assert_eq!(asked(&e), vec![FACTION + 0x24, OTHER]);
    assert_eq!(faction_reaction(&mut e, &[0, OTHER]), Some(2));
    assert_eq!(asked(&e)[2..], [0x24, OTHER]);
}

/// The game's form map with the forms `id -> id + 0x5000_0000` for `ids` (one bucket each).
fn form_map(e: &mut Engine, ids: &[u32]) {
    if !e.mem.is_mapped(0x011c_54c0) {
        e.map(0x011c_5000, 0x1000);
    }
    e.mem.set_u32(0x011c_54c0, MAP);
    e.mem.set_u32(MAP + 4, 16);
    e.mem.set_u32(MAP + 8, BUCKETS);
    for (i, &id) in ids.iter().enumerate() {
        let item = ITEMS + 12 * i as u32;
        e.mem.set_u32(item + 4, id);
        e.mem.set_u32(item + 8, id.wrapping_add(0x5000_0000));
        let bucket = BUCKETS + 4 * (id % 16);
        let first = e.mem.u32(bucket);
        e.mem.set_u32(item, first);
        e.mem.set_u32(bucket, item);
    }
}

/// `0x485D50`'s stand-in: the id's mod index made 3.
fn resolve_to_mod_3(e: &mut Engine, a: &[u32]) -> Ret {
    let id = e.mem.u32(a[0]);
    record(e, &[id, a[1]]);
    e.mem.set_u32(a[0], (id & 0x00ff_ffff) | 0x0300_0000);
    Ret::default()
}

/// G57: the armour's last file (`0x484E60` with -1), the id resolved in place through it
/// (`0x485D50`, here: the mod index made 3), then the resolved id's form from the game's map (not
/// the exe's `004839c0`); an id the map has not answers 0.
#[test]
fn g57_resolves_the_id_through_the_armours_file() {
    let mut e = part2_world();
    form_map(&mut e, &[0x0300_0042, 0x0100_0042]);
    e.register(FORM_FILE, |e, a| {
        record(e, &[a[0], a[1]]);
        answer(FILE)
    });
    e.register(RESOLVE_FORM_ID, resolve_to_mod_3);
    for slot in [ARMOR_SOUND_TEMPLATE, ARMOR_SOUND_ENTRY] {
        e.mem.set_u32(ASKED, 0);
        assert_eq!(
            e.site(slot.addr(), &[0x0100_0042, ARMOUR]),
            Some(0x5300_0042),
            "{:08x}",
            slot.addr()
        );
        assert_eq!(asked(&e), vec![ARMOUR, u32::MAX, 0x0100_0042, FILE]);
    }
    assert_eq!(armor_sound_form(&mut e, &[0x0100_0077, ARMOUR]), Some(0));
}

/// G130's names: `Data\` + the name less its last 8 characters + `bsa`.
#[test]
fn g130_archive_names_as_jip_builds_them() {
    assert_eq!(
        archive_name(b"Foo - Main.override"),
        b"Data\\Foo - Main.bsa"
    );
    assert_eq!(archive_name(b"X.OVERRIDE"), b"Data\\X.bsa");
}

/// One find entry for the stand-ins: `Foo.override` (a file), `Dir.override` (a folder),
/// `Bar.OVERRIDE` (a file), then no more (0).
fn find_entry(e: &mut Engine, data: u32, n: u32) -> u32 {
    let (name, attributes): (&[u8], u32) = match n {
        0 => (b"Foo.override", 0x20),
        1 => (b"Dir.override", 0x10),
        2 => (b"Bar.OVERRIDE", 0x80),
        _ => return 0,
    };
    e.mem.set_u32(data, attributes);
    e.mem.set_cstr(data + 0x2c, name);
    1
}

/// The find stand-ins over [`find_entry`]; the search text and the closed handle are kept.
fn find_stand_ins(e: &mut Engine) {
    e.register(FIND_FIRST_FILE_A, |e, a| {
        let text = e.mem.cstr(a[0]);
        e.mem.set_cstr(ASKED + 0x100, &text);
        find_entry(e, a[1], 0);
        e.mem.set_u32(ASKED + 0x80, 1);
        answer(7)
    });
    e.register(FIND_NEXT_FILE_A, |e, a| {
        assert_eq!(a[0], 7);
        let n = e.mem.u32(ASKED + 0x80);
        e.mem.set_u32(ASKED + 0x80, n + 1);
        answer(find_entry(e, a[1], n))
    });
    e.register(FIND_CLOSE, |e, a| {
        e.mem.set_u32(ASKED + 0x84, a[0]);
        Ret::default()
    });
}

/// G130's answer for the archive path `path`.
fn g130_answer(e: &mut Engine, path: &[u8]) -> Option<u32> {
    e.mem.set_cstr(ASKED + 0x200, path);
    e.site(ARCHIVE_OVERRIDE.addr(), &[ASKED + 0x200])
}

/// G130: the names read as the fixes go in (`Data\*.override`, files only, the search closed);
/// an archive whose path is one of them (any case, `DATA\` as the exe writes it) answers 1, any
/// other 0; after JIP's deferred start every archive answers 0 (the exe's word). No files: 0.
#[test]
fn g130_answers_whether_the_archive_has_an_override_file() {
    let mut e = part2_world();
    find_stand_ins(&mut e);
    read_override_archives(&mut e);
    assert_eq!(e.mem.cstr(ASKED + 0x100), b"Data\\*.override");
    assert_eq!(e.mem.u32(ASKED + 0x84), 7);
    assert_eq!(g130_answer(&mut e, b"DATA\\Foo.bsa"), Some(1));
    assert_eq!(g130_answer(&mut e, b"DATA\\bar.bsa"), Some(1));
    assert_eq!(g130_answer(&mut e, b"DATA\\Dir.bsa"), Some(0));
    assert_eq!(g130_answer(&mut e, b"DATA\\Foo - Main.bsa"), Some(0));
    e.set_jip_patches(engine_core::extend::jip_patches::FIXES);
    deferred_init(&mut e);
    assert_eq!(g130_answer(&mut e, b"DATA\\Foo.bsa"), Some(0));
    // No files at all.
    let mut e = part2_world();
    e.register(FIND_FIRST_FILE_A, |_, _| answer(u32::MAX));
    read_override_archives(&mut e);
    assert_eq!(g130_answer(&mut e, b"DATA\\Foo.bsa"), Some(0));
}

/// A reference in `CELL` (the vtable `0x102F55C` with `0x484B60` at `+0x48`) and the save's
/// add-change stand-in.
fn d01_world() -> Engine {
    let mut e = part2_world();
    if !e.mem.is_mapped(REFR_VTABLE) {
        e.map(REFR_VTABLE & !0xfff, 0x1000);
    }
    e.mem.set_u32(REFR_VTABLE + 0x48, MARK_AS_MODIFIED);
    e.mem.set_u32(REFERENCE, REFR_VTABLE);
    e.mem.set_u32(REFERENCE + 0x40, CELL);
    if !e.mem.is_mapped(SAVE_LOAD_GAME) {
        e.map(SAVE_LOAD_GAME & !0xfff, 0x1000);
    }
    e.mem.set_u32(SAVE_LOAD_GAME, 0x0abc_0000);
    e.register(ADD_CHANGE, |e, a| {
        record(e, &[a[0], a[1], a[2], a[3]]);
        Ret::default()
    });
    e.register(MARK_AS_MODIFIED, |e, a| {
        record(e, &[MARK_AS_MODIFIED, a[0], a[1]]);
        Ret::default()
    });
    e
}

/// D01 is put in at JIP's deferred start, only with the fixes' bit: without the bit the slot
/// keeps `0x484B60`.
#[test]
fn d01_slot_written_at_the_deferred_start_with_the_fixes() {
    use engine_core::extend::jip_patches;
    let mut e = d01_world();
    for word in [0, jip_patches::INSTALLED] {
        e.set_jip_patches(word);
        deferred_init(&mut e);
        assert_eq!(e.mem.u32(REFR_VTABLE + 0x48), MARK_AS_MODIFIED);
    }
    for word in [
        jip_patches::FIXES,
        jip_patches::INSTALLED | jip_patches::FIXES | jip_patches::FO3_WPN_DEGRADATION,
    ] {
        let mut e = d01_world();
        e.set_jip_patches(word);
        deferred_init(&mut e);
        assert_eq!(e.mem.u32(REFR_VTABLE + 0x48), MARK_MODIFIED_ROUTINE);
        assert!(slots::in_block(MARK_MODIFIED_ROUTINE));
    }
}

/// The game's data handler pointer (`0x11C3F2C`) and a handler with no files loaded (zeroed).
const GAME_DATA_HANDLER: u32 = 0x011c_3f2c;
const NO_FILES_HANDLER: u32 = 0x053a_0000;

/// JIP's deferred start through the co-save listener (xNVSE's `DEFERRED_INIT` message, as the
/// game sends it): with the fixes' bit, D01's slot holds the routine and a G130 name read before
/// answers 0 after; without the bit, the slot keeps `0x484B60` and the name still answers 1.
#[test]
fn deferred_start_through_the_listener_puts_group_gfs_part_in() {
    use engine_core::extend::jip_patches;
    let message = nvse::plugins::Message {
        kind: nvse::plugins::msg::DEFERRED_INIT,
        data: 0,
        len: 0,
    };
    for (word, slot, name) in [
        (
            jip_patches::INSTALLED | jip_patches::FIXES,
            MARK_MODIFIED_ROUTINE,
            0,
        ),
        (jip_patches::INSTALLED, MARK_AS_MODIFIED, 1),
    ] {
        let mut e = d01_world();
        find_stand_ins(&mut e);
        read_override_archives(&mut e);
        assert_eq!(g130_answer(&mut e, b"DATA\\Foo.bsa"), Some(1));
        e.set_jip_patches(word);
        e.ext::<crate::Jip>().pos_marker.marker = 1;
        // The listener runs the other groups' deferred parts too: group GS's (D07) looks a
        // plugin up through the game's data handler, here one with no files loaded.
        if !e.mem.is_mapped(GAME_DATA_HANDLER) {
            e.map(GAME_DATA_HANDLER & !0xfff, 0x1000);
        }
        e.map(NO_FILES_HANDLER, 0x1000);
        e.mem.set_u32(GAME_DATA_HANDLER, NO_FILES_HANDLER);
        crate::cosave::listen(&mut e, &message);
        assert_eq!(e.mem.u32(REFR_VTABLE + 0x48), slot, "{word:08x}");
        assert_eq!(
            g130_answer(&mut e, b"DATA\\Foo.bsa"),
            Some(name),
            "{word:08x}"
        );
    }
}

/// The words the save's add-change was asked with (or `0x484B60`'s, which must not be).
fn d01_mark(e: &mut Engine, flag: u32) -> Vec<u32> {
    e.mem.set_u32(ASKED, 0);
    e.vcall(REFERENCE, 0x48, &[flag]);
    asked(e)
}

/// D01 through the reference's vtable: the moved flag (exactly 4) is dropped when the parent
/// cell has form flag 0x400000 (the byte `+0xA`, bit 0x40); every other case (another flag, 4
/// with other bits, no cell, the cell without the bit but every other bit) goes to the save's
/// add-change with the reference, the flag and 0, never through `0x484B60`.
#[test]
fn d01_drops_the_moved_mark_in_a_flagged_cell() {
    let mut e = d01_world();
    e.set_jip_patches(engine_core::extend::jip_patches::FIXES);
    deferred_init(&mut e);
    let passed = |flag: u32| vec![0x0abc_0000, REFERENCE, flag, 0];
    e.mem.set_u32(CELL + 8, 0x0040_0000);
    assert!(d01_mark(&mut e, 4).is_empty());
    assert_eq!(d01_mark(&mut e, 5), passed(5));
    assert_eq!(d01_mark(&mut e, 0x8000_0004), passed(0x8000_0004));
    assert_eq!(d01_mark(&mut e, 1), passed(1));
    e.mem.set_u32(CELL + 8, 0xffbf_ffff);
    assert_eq!(d01_mark(&mut e, 4), passed(4));
    e.mem.set_u32(CELL + 8, 0x0040_0000);
    e.mem.set_u32(REFERENCE + 0x40, 0);
    assert_eq!(d01_mark(&mut e, 4), passed(4));
}

/// Each new handler at its own row: words on which another handler would answer otherwise.
#[test]
fn part2_handlers_are_at_their_rows() {
    let mut e = part2_world();
    e.register(0x0048_c1b0, |_, _| answer(9));
    form_map(&mut e, &[0x0300_0042]);
    e.register(FORM_FILE, |_, _| answer(FILE));
    e.register(RESOLVE_FORM_ID, resolve_to_mod_3);
    e.mem.set_cstr(ASKED + 0x200, b"DATA\\x.bsa");
    assert_eq!(
        e.site(FACTION_REACTION.addr(), &[ASKED + 0x200, 0]),
        Some(9)
    );
    assert_eq!(
        e.site(ARMOR_SOUND_TEMPLATE.addr(), &[0x0100_0042, ARMOUR]),
        Some(0x5300_0042)
    );
    assert_eq!(
        e.site(ARMOR_SOUND_ENTRY.addr(), &[0x0100_0042, ARMOUR]),
        Some(0x5300_0042)
    );
    assert_eq!(e.site(ARCHIVE_OVERRIDE.addr(), &[ASKED + 0x200]), Some(0));
}
