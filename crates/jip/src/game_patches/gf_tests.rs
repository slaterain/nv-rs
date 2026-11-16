//! Group GF's game patches: G39 and G40 (JIP's own: the NPC skeleton model), G07, G16, G20, G83
//! (fixes).

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
