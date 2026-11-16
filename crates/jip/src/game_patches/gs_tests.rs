//! Group GS's game patches: O7, G51, D07.

use super::*;
use crate::test_support::engine;
use engine_core::extend::jip_patches;

/// JIP's addresses (`SetOptionalPatch` case 7; Ghidra: `8B 4D 08` at each, the start of `MOV
/// ECX,[EBP+8]; CALL 00500940; MOV [EAX+8],0`).
#[test]
fn o7_seams_are_jips_addresses() {
    let sites: Vec<u32> = FAILED_SCRIPT_LOCKS.iter().map(|s| s.addr()).collect();
    assert_eq!(sites, [0x005e_0fbe, 0x005e_1a23, 0x005e_1f0b]);
    assert_eq!(AREA.fix_sites, sites);
}

/// Not installed: nothing registered at O7's seams.
#[test]
fn o7_not_installed_nothing_answers() {
    let mut e = engine();
    crate::install_as(&mut e, false);
    for slot in FAILED_SCRIPT_LOCKS {
        assert_eq!(e.site(slot.addr(), &[]), None);
    }
}

/// Installed with the row off: no handler; with the area's fixes in, every seam takes the call
/// (the seams in the translation test the switch's bit before asking).
#[test]
fn o7_handlers_take_the_call_with_the_fixes() {
    let mut e = engine();
    crate::install_as(&mut e, true);
    for slot in FAILED_SCRIPT_LOCKS {
        assert_eq!(e.site(slot.addr(), &[]), None, "row off");
    }
    install_fixes(&mut e);
    for slot in FAILED_SCRIPT_LOCKS {
        assert_eq!(e.site(slot.addr(), &[]), Some(1));
    }
}

// ---- G51, D07 ----

/// G51 at JIP's address (`SafeWrite16(0x5B4DBC, 0x19EB)`: Ghidra `0F B6 45 D3` there, the read
/// of the weapon flag), first in the area's seams.
#[test]
fn g51_row_is_jips_address() {
    assert_eq!(EVERY_ITEM_HEALTH.addr(), 0x005b_4dbc);
    assert_eq!(
        AREA.fix_sites,
        [0x005b_4dbc, 0x005e_0fbe, 0x005e_1a23, 0x005e_1f0b]
    );
}

/// G51: not installed or the row off, no handler; with the fixes in, the jump is taken (no
/// words).
#[test]
fn g51_handler_takes_the_jump_with_the_fixes() {
    let mut e = engine();
    crate::install_as(&mut e, false);
    assert_eq!(e.site(EVERY_ITEM_HEALTH.addr(), &[]), None);
    let mut e = engine();
    crate::install_as(&mut e, true);
    assert_eq!(e.site(EVERY_ITEM_HEALTH.addr(), &[]), None, "row off");
    install_fixes(&mut e);
    assert_eq!(e.site(EVERY_ITEM_HEALTH.addr(), &[]), Some(1));
}

/// The plugin's data: the script form at `SCRIPT_FORM` (its data +0x30 at `DATA`), the word at
/// data +0x29C (its neighbours set), the plugin loaded at index 5 under `name`, the form in the
/// form map under `id`.
const SCRIPT_FORM: u32 = crate::commands::test_support::FORMS + 0x100;
const SCRIPT_DATA_AT: u32 = crate::commands::test_support::FORMS + 0x1000;
const MOD_INFO: u32 = crate::commands::test_support::FORMS + 0x4000;

fn ccc(name: &[u8], id: u32, word: u32, data: bool, word_bits: u32) -> Engine {
    use crate::commands::test_support::{engine, game_globals, in_form_map, loaded_mod};
    let mut e = engine();
    game_globals(&mut e);
    loaded_mod(&mut e, 5, MOD_INFO, name);
    in_form_map(&mut e, SCRIPT_FORM, id, 0);
    e.mem.set_u32(SCRIPT_FORM + 0x2c, 0x1111_1111);
    e.mem
        .set_u32(SCRIPT_FORM + 0x30, if data { SCRIPT_DATA_AT } else { 0 });
    e.mem.set_u32(SCRIPT_FORM + 0x34, 0x2222_2222);
    e.mem.set_u32(SCRIPT_DATA_AT + 0x298, 0x3333_3333);
    e.mem.set_u32(SCRIPT_DATA_AT + 0x29c, word);
    e.mem.set_u32(SCRIPT_DATA_AT + 0x2a0, 0x4444_4444);
    e.set_jip_patches(word_bits);
    e
}

fn word(e: &Engine) -> [u32; 3] {
    [
        e.mem.u32(SCRIPT_DATA_AT + 0x298),
        e.mem.u32(SCRIPT_DATA_AT + 0x29c),
        e.mem.u32(SCRIPT_DATA_AT + 0x2a0),
    ]
}

const NAME: &[u8] = b"JIP Companions Command & Control.esp";
const ON: u32 = jip_patches::INSTALLED | jip_patches::FIXES;
/// `' 931'` ("139 ") and `' 552'` ("255 ").
const BEFORE: u32 = u32::from_le_bytes(*b"139 ");
const AFTER: u32 = u32::from_le_bytes(*b"255 ");

/// D07 with the fixes: the plugin's form `(index << 24) | 0xADE` has its word at data +0x29C
/// changed from "139 " to "255 ", only that word.
#[test]
fn d07_patches_the_plugins_word() {
    assert_eq!((BEFORE, AFTER), (0x2039_3331, 0x2035_3532));
    let mut e = ccc(NAME, 0x0500_0ade, BEFORE, true, ON);
    deferred_init(&mut e);
    assert_eq!(word(&e), [0x3333_3333, AFTER, 0x4444_4444]);
    // The name as JIP's hash compares it (case-insensitive).
    let mut e = ccc(
        b"jip companions command & control.ESP",
        0x0500_0ade,
        BEFORE,
        true,
        ON,
    );
    deferred_init(&mut e);
    assert_eq!(word(&e)[1], AFTER);
}

/// D07 leaves the word alone: the row off or JIP not installed (the word without `FIXES`), another
/// word there, another plugin, the form under another index, a form with no data.
#[test]
fn d07_leaves_everything_else() {
    for bits in [0, jip_patches::INSTALLED] {
        let mut e = ccc(NAME, 0x0500_0ade, BEFORE, true, bits);
        deferred_init(&mut e);
        assert_eq!(word(&e)[1], BEFORE, "{bits:x}");
    }
    let other = u32::from_le_bytes(*b"138 ");
    let mut e = ccc(NAME, 0x0500_0ade, other, true, ON);
    deferred_init(&mut e);
    assert_eq!(word(&e)[1], other);
    let mut e = ccc(
        b"JIP Companions Command & Control.esm",
        0x0500_0ade,
        BEFORE,
        true,
        ON,
    );
    deferred_init(&mut e);
    assert_eq!(word(&e)[1], BEFORE);
    let mut e = ccc(NAME, 0x0400_0ade, BEFORE, true, ON);
    deferred_init(&mut e);
    assert_eq!(word(&e)[1], BEFORE);
    let mut e = ccc(NAME, 0x0500_0adf, BEFORE, true, ON);
    deferred_init(&mut e);
    assert_eq!(word(&e)[1], BEFORE);
    let mut e = ccc(NAME, 0x0500_0ade, BEFORE, false, ON);
    deferred_init(&mut e);
    assert_eq!(word(&e)[1], BEFORE);
}

/// D07 runs at xNVSE's `DeferredInit` message (JIP's listener).
#[test]
fn d07_runs_at_deferred_init() {
    use nvse::plugins::{dispatch, msg};
    let mut e = ccc(NAME, 0x0500_0ade, BEFORE, true, ON);
    dispatch(&mut e, msg::MAIN_GAME_LOOP, 0, 0);
    assert_eq!(word(&e)[1], BEFORE, "only DeferredInit");
    dispatch(&mut e, msg::DEFERRED_INIT, 0, 0);
    assert_eq!(word(&e)[1], AFTER);
}
