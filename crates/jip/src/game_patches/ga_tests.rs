//! Group GA's game patches: G08, G11, G17, G21, G35, G37, G55, G58, H21. Each handler's branches over
//! stand-ins of what JIP's bytes read, values from JIP's assembly (`internal/patches_game.h`,
//! `internal/hooks.h`) and the exe's (Ghidra).

use super::*;
use crate::test_support::engine;

const AREA_AT: u32 = 0x0500_0000;

/// An engine with JIP installed (the row off) and a test page mapped.
fn installed() -> Engine {
    let mut e = engine();
    e.map(AREA_AT, 0x4000);
    crate::install_as(&mut e, true);
    e
}

/// JIP's addresses (the first byte JIP writes at each; G11's one seam stands for its four
/// writes), rising, and the area lists them as fixes only.
#[test]
fn ga_seams_are_jips_addresses() {
    let sites: Vec<u32> = FIX_SITES.iter().map(|s| s.addr()).collect();
    assert_eq!(
        sites,
        [
            0x0070_e5fe,
            0x0071_d0d2,
            0x0077_04ae,
            0x007a_a80f,
            0x007d_333b,
            0x007e_6f04,
            0x00a1_f035,
            0x00c4_4405
        ]
    );
    let corner: Vec<u32> = CORNER_MESSAGE.iter().map(|s| s.addr()).collect();
    assert_eq!(ROLLOVER_WEAPON.addr(), 0x0077_7123);
    assert_eq!(VATS_TARGET_DT.addr(), 0x007e_ce32);
    assert_eq!(MESSAGE_TIME.addr(), 0x0070_535c);
    assert_eq!(
        AREA.fix_sites,
        [
            sites,
            corner,
            vec![
                ROLLOVER_WEAPON.addr(),
                VATS_TARGET_DT.addr(),
                MESSAGE_TIME.addr(),
                PICK_LOAD_SCREEN.addr(),
            ]
        ]
        .concat()
    );
    assert_eq!(PICK_LOAD_SCREEN.addr(), 0x0078_a79b);
    assert_eq!(AREA.fix_wraps, [0x0078_ac60]);
    assert_eq!(
        AREA.jip_sites,
        [0x007a_9aee, 0x007a_9c45],
        "G55 (JIP's own)"
    );
    assert!(AREA.jip_wraps.is_empty());
}

/// Not installed: nothing answers at any seam.
#[test]
fn ga_not_installed_nothing_answers() {
    let mut e = engine();
    e.map(AREA_AT, 0x4000);
    crate::install_as(&mut e, false);
    for slot in FIX_SITES {
        assert_eq!(e.site(slot.addr(), &[AREA_AT, 0x1d, 1]), None);
    }
    for slot in MAX_MESSAGE_WIDTH {
        assert_eq!(e.site(slot.addr(), &[AREA_AT, 0x1d]), None);
    }
}

/// G55 is in with JIP installed, the row off: no box waiting, it does not answer (the game's
/// width; the command's tests drive a box).
#[test]
fn g55_in_with_jip_installed() {
    let mut e = installed();
    for slot in MAX_MESSAGE_WIDTH {
        assert_eq!(e.site_owners(slot.addr()).to_vec(), vec![Owner::Jip]);
        assert_eq!(e.site(slot.addr(), &[AREA_AT, 0x1d]), None);
    }
}

/// Installed with the row off: nothing answers; with the fixes in, each seam answers with its
/// own handler, told apart by its answer to the same stand-ins (a handler at another row's
/// address fails here; the two size rows and the two jump rows answer alike, as JIP's bytes do).
#[test]
fn ga_each_handler_answers_at_its_own_seam() {
    let mut e = installed();
    let node = AREA_AT + 0x100;
    let item = AREA_AT + 0x200;
    // G21's node with an item whose text is null; G58's input with the key down now only; H21's
    // HUD with no tile.
    e.mem.set_u32(node, item);
    e.mem.set_u8(node + 0x1921, 0x80);
    for slot in FIX_SITES {
        assert_eq!(e.site(slot.addr(), &[node, 0x1d, 1]), None, "row off");
    }
    install_fixes(&mut e);
    let answers: Vec<Option<u32>> = FIX_SITES
        .iter()
        .map(|slot| e.site(slot.addr(), &[node, 0x1d, 1]))
        .collect();
    assert_eq!(
        answers,
        [
            Some(1),     // G58: down now, not before
            Some(1),     // G35: the jump
            Some(0),     // H21: walked (no tile)
            Some(1),     // G21: no text, passed over
            Some(node),  // G37: a quest found is kept
            Some(0x400), // G11: the size
            Some(1),     // G17: the jump
            Some(0x400), // G08: the size
        ]
    );
    // A text, the key held from the frame before, no quest (none in the form map): G21, G58 and
    // G37 answer otherwise.
    forms_map(&mut e);
    e.mem.set_u32(item + 4, AREA_AT + 0x300);
    // G21 and G58 disagree here (a text: G21 goes on; the key down now only: G58 toggles), so
    // the two swapped in `install_fixes` fail; so do H21 and G37 against either.
    assert_eq!(e.site(CONSOLE_KEY.addr(), &[node, 0x1d, 1]), Some(1));
    assert_eq!(e.site(ACTIVATION_TEXT.addr(), &[node]), Some(0));
    e.mem.set_u8(node + 0x1a21, 0x80);
    assert_eq!(e.site(CHARGEN_QUEST.addr(), &[0]), Some(0));
    assert_eq!(e.site(ACTIVATION_TEXT.addr(), &[node]), Some(0));
    assert_eq!(e.site(CONSOLE_KEY.addr(), &[node, 0x1d, 1]), Some(0));
}

/// G58 (`ToggleConsoleHook`): `xor eax,eax; test [ecx+0x1A21],0x80; jnz done; test
/// [ecx+0x1921],0x80; setnz al`: only bit 0x80 of each byte counts, the frame before first; the
/// key and the 1 (the `retn 8` words) are not read.
#[test]
fn g58_console_key_is_down_only_on_its_first_frame() {
    let mut e = engine();
    e.map(AREA_AT, 0x4000);
    let input = AREA_AT;
    for (before, now, want) in [
        (0x00u8, 0x00u8, 0u32),
        (0x00, 0x80, 1),
        (0x80, 0x80, 0),
        (0x80, 0x00, 0),
        (0x7f, 0x7f, 0),
        (0x7f, 0xff, 1),
        (0xff, 0xff, 0),
    ] {
        e.mem.set_u8(input + 0x1a21, before);
        e.mem.set_u8(input + 0x1921, now);
        // Neighbouring bytes set, so a wrong offset reads a different answer.
        e.mem.set_u8(input + 0x1920, 0x80);
        e.mem.set_u8(input + 0x1922, 0x80);
        e.mem.set_u8(input + 0x1a20, 0x80);
        e.mem.set_u8(input + 0x1a22, 0x80);
        for key in [0x1d, 0x29] {
            assert_eq!(
                console_key(&mut e, &[input, key, 1]),
                Some(want),
                "{before:#x} {now:#x} {key:#x}"
            );
        }
    }
}

/// G21: the node's item (`[node]`) and its text (`[item+4]`); a null text passes the node over,
/// any other goes on; the item's other words are not read.
#[test]
fn g21_a_choice_without_text_is_passed_over() {
    let mut e = engine();
    e.map(AREA_AT, 0x4000);
    let node = AREA_AT + 0x10;
    let item = AREA_AT + 0x40;
    e.mem.set_u32(node, item);
    e.mem.set_u32(node + 4, AREA_AT + 0x80);
    e.mem.set_u32(item, 0x1234);
    e.mem.set_u32(item + 8, 0x5678);
    assert_eq!(activation_text(&mut e, &[node]), Some(1));
    e.mem.set_u32(item + 4, AREA_AT + 0x300);
    assert_eq!(activation_text(&mut e, &[node]), Some(0));
    e.mem.set_u32(item + 4, 1);
    assert_eq!(activation_text(&mut e, &[node]), Some(0));
}

/// G37 (`InvalidChargenQuestFixHook`): a quest found is kept; none is looked up as form
/// `0x102037` in the game's form map (`*0x11C54C0`), 0 when the map has none.
#[test]
fn g37_no_quest_falls_back_to_the_vanilla_one() {
    use crate::commands::test_support::in_form_map;
    let mut e = engine();
    forms_map(&mut e);
    assert_eq!(chargen_quest(&mut e, &[0]), Some(0), "not in the map");
    in_form_map(&mut e, 0x0abc_0000, 0x0010_2036, 0);
    in_form_map(&mut e, 0x0def_0000, 0x0010_2038, 1);
    assert_eq!(chargen_quest(&mut e, &[0]), Some(0), "only its neighbours");
    in_form_map(&mut e, 0x0123_4560, VANILLA_CHARGEN_QUEST, 2);
    assert_eq!(chargen_quest(&mut e, &[0]), Some(0x0123_4560));
    assert_eq!(chargen_quest(&mut e, &[0x0777_0000]), Some(0x0777_0000));
}

/// The two sizes and the two jumps.
#[test]
fn g08_g11_g17_g35_answers() {
    let mut e = engine();
    assert_eq!(ini_string_buffer(&mut e, &[0x100]), Some(0x400));
    assert_eq!(trait_description(&mut e, &[0x200]), Some(0x400));
    assert_eq!(keep_menu_open_byte(&mut e, &[1, 2]), Some(1));
    assert_eq!(console_while_loading(&mut e, &[1]), Some(1));
}

const DESTROY: u32 = 0x00ab_c000;

/// H21 (`ClearHUDOrphanedTiles`): the HUD's tile (`[hud+4]`), its child list's head
/// (`[tile+4]`), each node's tile (`+8`) and next (`+0`); a child whose scene node (`+0x2C`) has
/// no parent (`+0x18`) is destroyed through its virtual slot 0 with 1, in list order; a null
/// child, a child with no scene node and a child whose node has a parent stay. The next node is
/// read before the destroy (here the destroy clears the node).
#[test]
fn h21_orphaned_tiles_are_destroyed_in_order() {
    let mut e = engine();
    e.map(AREA_AT, 0x4000);
    let vtable = AREA_AT + 0x3000;
    e.mem.set_u32(vtable, DESTROY);
    let hud = AREA_AT;
    let tile = AREA_AT + 0x100;
    e.mem.set_u32(hud + 4, tile);
    // Children: 0 orphan, 1 null tile, 2 no scene node, 3 with a parent, 4 orphan.
    let nodes: Vec<u32> = (0..5).map(|i| AREA_AT + 0x200 + 0x10 * i).collect();
    let children: Vec<u32> = (0..5).map(|i| AREA_AT + 0x400 + 0x40 * i).collect();
    let scenes: Vec<u32> = (0..5).map(|i| AREA_AT + 0x800 + 0x40 * i).collect();
    e.mem.set_u32(tile + 4, nodes[0]);
    for i in 0..5 {
        e.mem
            .set_u32(nodes[i], if i < 4 { nodes[i + 1] } else { 0 });
        e.mem
            .set_u32(nodes[i] + 8, if i == 1 { 0 } else { children[i] });
        e.mem.set_u32(children[i], vtable);
        e.mem
            .set_u32(children[i] + 0x2c, if i == 2 { 0 } else { scenes[i] });
        e.mem
            .set_u32(scenes[i] + 0x18, if i == 3 { AREA_AT + 0xf00 } else { 0 });
        // Words beside the ones JIP reads, so a wrong offset reads another answer.
        e.mem.set_u32(children[i] + 0x28, scenes[i]);
        e.mem.set_u32(children[i] + 0x30, scenes[i]);
        e.mem.set_u32(scenes[i] + 0x14, AREA_AT + 0xf00);
        e.mem.set_u32(scenes[i] + 0x1c, AREA_AT + 0xf00);
    }
    let first = nodes[0];
    e.register_double(DESTROY, move |e, _| {
        e.mem.set_u32(first, 0);
        Ret::default()
    });
    e.call_log = Some(vec![]);
    assert_eq!(clear_orphaned_tiles(&mut e, &[hud]), Some(0));
    let destroyed: Vec<Vec<u32>> = e
        .call_log
        .take()
        .unwrap()
        .into_iter()
        .filter(|(at, _)| *at == DESTROY)
        .map(|(_, words)| words)
        .collect();
    assert_eq!(destroyed, [vec![children[0], 1], vec![children[4], 1]]);

    // A HUD with no tile walks nothing.
    e.mem.set_u32(hud + 4, 0);
    e.call_log = Some(vec![]);
    assert_eq!(clear_orphaned_tiles(&mut e, &[hud]), Some(0));
    assert!(e.call_log.take().unwrap().is_empty());
}

/// The game's form map in test memory (empty).
fn forms_map(e: &mut Engine) {
    use crate::commands::test_support::{game_globals, FORMS};
    e.map(FORMS, 0x10_0000);
    game_globals(e);
}

/// G10: JIP's first byte in each of the three functions, rising.
#[test]
fn g10_seams_are_jips_addresses() {
    let sites: Vec<u32> = CORNER_MESSAGE.iter().map(|s| s.addr()).collect();
    assert_eq!(sites, [0x0077_14da, 0x0077_53b8, 0x0077_57b1]);
}

/// G10 not installed: nothing answers.
#[test]
fn g10_not_installed_nothing_answers() {
    let mut e = engine();
    crate::install_as(&mut e, false);
    for slot in CORNER_MESSAGE {
        assert_eq!(e.site(slot.addr(), &[0x104]), None);
    }
}

/// G10 installed with the row off: nothing; with the fixes in, each of the three seams answers
/// JIP's 0x204 (0x203 letters: JIP's `SafeWrite8(0x7753D2, 2)` makes `PUSH 0x104` `PUSH 0x204`),
/// a different answer from every other GA seam given the same word. The handlers alone (before
/// `FIXES` is set) do not change `corner_message_text_size`: it reads the queue's own bit.
#[test]
fn g10_each_seam_answers_jips_text_buffer() {
    let mut e = engine();
    crate::install_as(&mut e, true);
    for slot in CORNER_MESSAGE {
        assert_eq!(e.site(slot.addr(), &[0x104]), None, "row off");
    }
    install_fixes(&mut e);
    for slot in CORNER_MESSAGE {
        assert_eq!(e.site(slot.addr(), &[0x104]), Some(0x204));
    }
    assert_eq!(corner_message_text(&mut e, &[0x104]), Some(0x204));
    assert_eq!(
        e.jip_patches() & engine_core::extend::jip_patches::FIXES,
        0,
        "only the handlers are in"
    );
    assert_eq!(
        corner_message_text_size(&mut e),
        0x104,
        "the queue reads FIXES too"
    );
}

/// G80's stand-ins: a weapon at `AREA_AT + 0x400` with flag byte +0x100 `flags`, a reference at
/// `AREA_AT + 0x800` whose list holds `ExtraWeaponModFlags` with the flags `mods` (none when
/// `None`), and the crosshair name `name`.
fn g80_world(flags: u8, mods: Option<u32>, name: &[u8]) -> (Engine, u32, u32) {
    let mut e = crate::commands::test_support::engine();
    crate::commands::test_support::extra_stand_ins(&mut e);
    e.map(AREA_AT, 0x1000);
    e.map(0x011d_9000, 0x1000);
    let (weapon, reference) = (AREA_AT + 0x400, AREA_AT + 0x800);
    e.mem.set_u8(weapon + 0x100, flags);
    // Another extra kind first, so the lookup walks.
    crate::commands::test_support::put_extra(&mut e, reference + 0x44, AREA_AT + 0xc00, 0x8C, 7);
    if let Some(mods) = mods {
        crate::commands::test_support::put_extra(
            &mut e,
            reference + 0x44,
            AREA_AT + 0xc40,
            0x8D,
            mods,
        );
    }
    e.mem.set_cstr(0x011d_9c48, name);
    // The byte after the buffer, to see a write past it.
    e.mem.set_u8(0x011d_9c48 + 0x104, 0xAA);
    (e, weapon, reference)
}

/// G80 not installed, or installed with the row off: nothing answers at its seam; with the
/// fixes in, its handler answers there.
#[test]
fn g80_seam_answers_only_with_the_fixes() {
    let (mut e, weapon, reference) = g80_world(0, None, b"Gun");
    crate::install_as(&mut e, false);
    assert_eq!(
        e.site(0x0077_7123, &[weapon, reference]),
        None,
        "not installed"
    );
    crate::install_as(&mut e, true);
    assert_eq!(e.site(0x0077_7123, &[weapon, reference]), None, "row off");
    install_fixes(&mut e);
    assert_eq!(e.site(0x0077_7123, &[weapon, reference]), Some(1));
}

/// JIP's `RolloverWeaponHook`: a weapon not playable (+0x100 bit 0x80) answers 0 and its name
/// stays, mods or not; a playable one answers 1, and its name gets "+" only when the reference
/// has `ExtraWeaponModFlags` with flags not 0 (low byte +0xC; a higher byte alone is no mod).
#[test]
fn g80_playable_weapon_with_mods_gets_a_plus() {
    let cases: [(u8, Option<u32>, u32, &[u8]); 6] = [
        (0x80, Some(1), 0, b"Gun"),
        (0x81, None, 0, b"Gun"),
        (0x7F, None, 1, b"Gun"),
        (0, Some(0x100), 1, b"Gun"),
        (0, Some(1), 1, b"Gun+"),
        (0x7F, Some(4), 1, b"Gun+"),
    ];
    for (flags, mods, answer, name) in cases {
        let (mut e, weapon, reference) = g80_world(flags, mods, b"Gun");
        assert_eq!(
            rollover_weapon(&mut e, &[weapon, reference]),
            Some(answer),
            "{flags:#x} {mods:?}"
        );
        assert_eq!(e.mem.cstr(0x011d_9c48), name, "{flags:#x} {mods:?}");
    }
}

/// The "+" and its end stay inside the 0x104-byte name buffer: a 0x102-byte name gets it (its
/// end at the last byte), a 0x103-byte one keeps its name (JIP would write one byte past it).
#[test]
fn g80_plus_only_within_the_name_buffer() {
    let fits = vec![b'a'; 0x102];
    let (mut e, weapon, reference) = g80_world(0, Some(1), &fits);
    assert_eq!(rollover_weapon(&mut e, &[weapon, reference]), Some(1));
    assert_eq!(
        e.mem.cstr(0x011d_9c48),
        [fits.clone(), b"+".to_vec()].concat()
    );
    assert_eq!(e.mem.u8(0x011d_9c48 + 0x104), 0xAA);
    let full = vec![b'a'; 0x103];
    let (mut e, weapon, reference) = g80_world(0, Some(1), &full);
    assert_eq!(rollover_weapon(&mut e, &[weapon, reference]), Some(1));
    assert_eq!(e.mem.cstr(0x011d_9c48), full);
    assert_eq!(e.mem.u8(0x011d_9c48 + 0x104), 0xAA);
}

/// G121 not installed, or installed with the row off: nothing answers at its seam; with the
/// fixes in and no hit reckoned by JIP's hit damage yet (`s_VATSHitDT` still `-FLT_MAX`), no
/// answer either: the exe's call runs, as JIP jumps to it.
#[test]
fn g121_no_answer_until_a_hit_is_reckoned() {
    let mut e = engine();
    crate::install_as(&mut e, false);
    assert_eq!(
        e.site(0x007e_ce32, &[0x5000, 0x6000]),
        None,
        "not installed"
    );
    crate::install_as(&mut e, true);
    assert_eq!(e.site(0x007e_ce32, &[0x5000, 0x6000]), None, "row off");
    install_fixes(&mut e);
    assert_eq!(
        vats_target_dt(&mut e, &[0x5000, 0x6000]),
        None,
        "no hit yet"
    );
    assert_eq!(
        e.site(0x007e_ce32, &[0x5000, 0x6000]),
        None,
        "the exe's call"
    );
}

/// JIP's `GetVATSTargetDTHook` on the recorded threshold: `-FLT_MAX` gives no answer; a
/// threshold with its sign bit set (negative, -0.0, the next float above `-FLT_MAX`) gives 0.0;
/// any other (0.0, a fraction, `FLT_MAX`) its own bits.
#[test]
fn g121_negative_thresholds_show_as_zero() {
    assert_eq!(vats_dt_answer(-f32::MAX), None);
    for dt in [-0.5f32, -0.0, -1.0e30, f32::from_bits(0xFF7F_FFFE)] {
        assert_eq!(vats_dt_answer(dt), Some(0), "{dt}");
    }
    for dt in [0.0f32, 6.0, 0.25, f32::MAX] {
        assert_eq!(vats_dt_answer(dt), Some(dt.to_bits()), "{dt}");
    }
}

/// G121 with a hit recorded by JIP's hit damage: the seam answers through the real handler, a
/// positive threshold as its bits (6.0) and a negative one as 0 (-0.5).
#[test]
fn g121_answers_the_recorded_threshold() {
    let mut e = engine();
    crate::install_as(&mut e, true);
    install_fixes(&mut e);
    crate::game_patches::gc::damage::set_vats_hit_dt(&mut e, 6.0);
    assert_eq!(
        e.site(0x007e_ce32, &[0x5000, 0x6000]),
        Some(6.0f32.to_bits())
    );
    crate::game_patches::gc::damage::set_vats_hit_dt(&mut e, -0.5);
    assert_eq!(e.site(0x007e_ce32, &[0x5000, 0x6000]), Some(0));
}

/// G09 not installed, or installed with the row off: nothing answers at its seam; with the fixes
/// in, its handler answers there (the default time, 2.0, for the game's call).
#[test]
fn g09_seam_answers_only_with_the_fixes() {
    let mut e = engine();
    crate::install_as(&mut e, false);
    assert_eq!(
        e.site(0x0070_535c, &[2.5f32.to_bits()]),
        None,
        "not installed"
    );
    crate::install_as(&mut e, true);
    assert_eq!(e.site(0x0070_535c, &[2.5f32.to_bits()]), None, "row off");
    install_fixes(&mut e);
    assert_eq!(
        e.site(0x0070_535c, &[2.5f32.to_bits()]),
        Some(2.0f32.to_bits())
    );
}

/// JIP's `QueueUIMessageHook`: the game's own call (unmarked) gets the default time, 2.0 and
/// then what `SetDefaultMessageTime` set; a built-in plugin's call (marked while JIP's fixes are
/// in) keeps the time it passed, and only while its mark lives.
#[test]
fn g09_game_calls_get_the_default_time_plugin_calls_keep_theirs() {
    let mut e = engine();
    // install_as runs every area's install_jip once (GA's: the time slot and G55's seams; a
    // second run would register G55's seams twice, which on_site refuses).
    crate::install_as(&mut e, true);
    e.set_jip_patches(
        engine_core::extend::jip_patches::INSTALLED | engine_core::extend::jip_patches::FIXES,
    );
    let passed = 2.5f32.to_bits();
    assert_eq!(message_time(&mut e, &[passed]), Some(2.0f32.to_bits()));
    set_default_message_time(&mut e, 5.0);
    assert_eq!(message_time(&mut e, &[passed]), Some(5.0f32.to_bits()));
    let mark = e.mark_plugin_ui_message();
    assert_eq!(message_time(&mut e, &[passed]), Some(passed), "a plugin's");
    drop(mark);
    assert_eq!(message_time(&mut e, &[passed]), Some(5.0f32.to_bits()));
}

// ---- O16, bLoadScreenFix ----

const O16_HANDLER: u32 = AREA_AT + 0x2000;
const O16_NODES: u32 = AREA_AT + 0x2200;
const O16_SCREENS: u32 = AREA_AT + 0x1000;
const O16_TYPE: u32 = AREA_AT + 0x1800;
const O16_RESULTS: u32 = AREA_AT + 0x3000;
const O16_RESULT_DATA: u32 = AREA_AT + 0x3100;
const O16_MENU: u32 = AREA_AT + 0x3400;
const O16_OS_GLOBALS: u32 = AREA_AT + 0x3800;
const O16_LOCATION: u32 = 0x55;
const O16_NEXT: u32 = 0x0078_ac60;

fn screen(i: u32) -> u32 {
    O16_SCREENS + i * 0x100
}

thread_local! {
    /// The screens `CanDisplayForForm` answers true for.
    static SHOWS_HERE: std::cell::RefCell<Vec<u32>> = const { std::cell::RefCell::new(Vec::new()) };
}

/// O16's world: the switch on (JIP installed, the row off), the data handler's load screen list
/// `screen(1)` (locations unless `no_locations`), a null item, `screen(2)` (generic, type 5),
/// `screen(3)` (locations unless `no_locations`), `screen(4)` (generic, flag 0x400, no type); the
/// menu with the location; the candidates' array (size 5 before); `CanDisplayForForm` true for the
/// screens in [`SHOWS_HERE`]; the array's add appending the word it is given; the exe's
/// `BuildCandidateList` a marker (`0x7777` in the size); the lists made (`DeferredInit`).
fn o16_world_with(shows: &[u32], no_locations: bool) -> Engine {
    let mut e = o16_world_unmade(shows, no_locations);
    deferred_init(&mut e);
    e
}

/// [`o16_world_with`] before the lists are made.
fn o16_world_unmade(shows: &[u32], no_locations: bool) -> Engine {
    let mut e = installed();
    e.map(0x011c_3000, 0x1000);
    e.map(0x011d_8000, 0x1000);
    e.map(0x011d_e000, 0x1000);
    e.jip_patches_or(engine_core::extend::jip_patches::LOAD_SCREEN_FIX);
    e.mem.set_u32(0x011c_3f2c, O16_HANDLER);
    e.mem.set_u32(0x011d_ea0c, O16_OS_GLOBALS);
    let items = [screen(1), 0, screen(2), screen(3), screen(4)];
    let mut node = O16_HANDLER + 0x128;
    for (i, &item) in items.iter().enumerate() {
        e.mem.set_u32(node, item);
        let next = if i + 1 < items.len() {
            O16_NODES + 8 * i as u32
        } else {
            0
        };
        e.mem.set_u32(node + 4, next);
        node = next;
    }
    if !no_locations {
        e.mem.set_u32(screen(1) + 0x2c, 0x1111);
        e.mem.set_u32(screen(3) + 0x2c, 0x3333);
    }
    e.mem.set_u32(screen(2) + 0x34, O16_TYPE);
    e.mem.set_u32(O16_TYPE + 0x18, 5);
    e.mem.set_u8(screen(4) + 9, 4);
    e.mem.set_u32(O16_MENU + 0x1e8, O16_LOCATION);
    e.mem.set_u32(O16_RESULTS + 4, O16_RESULT_DATA);
    e.mem.set_u32(O16_RESULTS + 8, 5);
    SHOWS_HERE.with(|s| *s.borrow_mut() = shows.to_vec());
    e.register(0x005a_6ec0, |_, a| Ret {
        eax: u32::from(SHOWS_HERE.with(|s| s.borrow().contains(&a[0]))),
        ..Ret::default()
    });
    e.register(0x007c_b2e0, |e, a| {
        let n = e.mem.u32(a[0] + 8);
        let data = e.mem.u32(a[0] + 4);
        e.mem.set_u32(data + 4 * n, e.mem.u32(a[1]));
        e.mem.set_u32(a[0] + 8, n + 1);
        Ret::default()
    });
    e.register(O16_NEXT, |e, a| {
        e.mem.set_u32(a[1] + 8, 0x7777);
        Ret::default()
    });
    e.call_log = Some(vec![]);
    e
}

fn o16_world(shows: &[u32]) -> Engine {
    o16_world_with(shows, false)
}

fn candidates(e: &mut Engine) -> Vec<u32> {
    let n = e.mem.u32(O16_RESULTS + 8);
    (0..n).map(|i| e.mem.u32(O16_RESULT_DATA + 4 * i)).collect()
}

fn shown_asked(e: &Engine) -> Vec<Vec<u32>> {
    e.call_log
        .as_ref()
        .unwrap()
        .iter()
        .filter(|c| c.0 == 0x005a_6ec0)
        .map(|c| c.1.clone())
        .collect()
}

/// O16's lists (`SetOptionalPatch` case 16, at `DeferredInit`): the data's screens with
/// locations and the generic ones, in order, null items passed over; with the switch off, or no
/// data handler, none are made.
#[test]
fn o16_the_lists_are_made_once_the_data_is_in() {
    let mut e = o16_world(&[]);
    assert_eq!(
        e.ext_free::<LoadScreenLists>().0.get(),
        Some(&(vec![screen(1), screen(3)], vec![screen(2), screen(4)]))
    );
    let mut e = installed();
    e.map(0x011c_3000, 0x1000);
    e.mem.set_u32(0x011c_3f2c, O16_HANDLER);
    deferred_init(&mut e);
    assert_eq!(e.ext_free::<LoadScreenLists>().0.get(), None, "switch off");
    e.jip_patches_or(engine_core::extend::jip_patches::LOAD_SCREEN_FIX);
    e.mem.set_u32(0x011c_3f2c, 0);
    deferred_init(&mut e);
    assert_eq!(e.ext_free::<LoadScreenLists>().0.get(), None, "no data");
}

/// O16 `GetSuitableLoadScreensHook` with a location: the array emptied, the location screens
/// asked in order with the location, those that can show added; when none can, the generic
/// ones; with no location, the generic ones without asking.
#[test]
fn o16_the_location_screens_else_the_generic_ones() {
    let mut e = o16_world(&[screen(3)]);
    load_screen_candidates(&mut e, &[O16_MENU, O16_RESULTS]);
    assert_eq!(candidates(&mut e), [screen(3)]);
    assert_eq!(
        shown_asked(&e),
        [vec![screen(1), O16_LOCATION], vec![screen(3), O16_LOCATION]]
    );
    let mut e = o16_world(&[]);
    load_screen_candidates(&mut e, &[O16_MENU, O16_RESULTS]);
    assert_eq!(candidates(&mut e), [screen(2), screen(4)], "none can show");
    let mut e = o16_world(&[screen(1), screen(3)]);
    e.mem.set_u32(O16_MENU + 0x1e8, 0);
    load_screen_candidates(&mut e, &[O16_MENU, O16_RESULTS]);
    assert_eq!(candidates(&mut e), [screen(2), screen(4)], "no location");
    assert!(shown_asked(&e).is_empty());
}

/// O16 in the overlay mode (the menu's flag bit 5, `+0x222` bit 0x20): only the generic screens
/// with flag 0x400, the location screens not asked even with a location.
#[test]
fn o16_the_overlay_mode_takes_flagged_generic_screens() {
    let mut e = o16_world(&[screen(1), screen(3)]);
    e.mem.set_u8(O16_MENU + 0x222, 0x20);
    load_screen_candidates(&mut e, &[O16_MENU, O16_RESULTS]);
    assert_eq!(candidates(&mut e), [screen(4)]);
    assert!(shown_asked(&e).is_empty());
}

/// O16 with `0x11D8907` or the OS globals' byte `+2` set: a screen of no type, type 0 or 3 is
/// added, any other type not (screen 2's type 5).
#[test]
fn o16_only_type_3_screens_while_the_menu_is_replaced() {
    for at in [0x011d_8907, O16_OS_GLOBALS + 2] {
        let mut e = o16_world(&[]);
        e.mem.set_u8(at, 1);
        load_screen_candidates(&mut e, &[O16_MENU, O16_RESULTS]);
        assert_eq!(candidates(&mut e), [screen(4)], "{at:#x}");
        for (value, kept) in [(0u32, true), (3, true), (2, false)] {
            let mut e = o16_world(&[]);
            e.mem.set_u8(at, 1);
            e.mem.set_u32(O16_TYPE + 0x18, value);
            load_screen_candidates(&mut e, &[O16_MENU, O16_RESULTS]);
            assert_eq!(
                candidates(&mut e).contains(&screen(2)),
                kept,
                "{at:#x} {value}"
            );
        }
    }
    // A location screen of type 5 that can show is held back too, and the generic ones follow.
    let mut e = o16_world(&[screen(1)]);
    e.mem.set_u32(screen(1) + 0x34, O16_TYPE);
    e.mem.set_u8(0x011d_8907, 1);
    load_screen_candidates(&mut e, &[O16_MENU, O16_RESULTS]);
    assert_eq!(candidates(&mut e), [screen(4)]);
}

/// O16 with no screens with locations at all and a location: none asked or added from them and
/// the generic ones follow (JIP's walk would run on past its empty list).
#[test]
fn o16_no_location_screens_at_all() {
    let mut e = o16_world_with(&[screen(1)], true);
    assert_eq!(
        e.ext_free::<LoadScreenLists>().0.get(),
        Some(&(vec![], vec![screen(1), screen(2), screen(3), screen(4)]))
    );
    load_screen_candidates(&mut e, &[O16_MENU, O16_RESULTS]);
    assert!(shown_asked(&e).is_empty());
    assert_eq!(
        candidates(&mut e),
        [screen(1), screen(2), screen(3), screen(4)]
    );
}

/// O16's wrap through the dispatch: with the switch on JIP's (the next link not run); off, the
/// exe's `BuildCandidateList` (the next link).
#[test]
fn o16_the_wrap_runs_the_exes_with_the_switch_off() {
    let mut e = o16_world(&[screen(3)]);
    install_fixes(&mut e);
    assert_eq!(e.wraps_at(0x0078_ac60), [Owner::Jip]);
    e.call(0x0078_ac60, &[O16_MENU, O16_RESULTS]);
    assert_eq!(candidates(&mut e), [screen(3)]);
    e.set_jip_patches(engine_core::extend::jip_patches::INSTALLED);
    e.call(0x0078_ac60, &[O16_MENU, O16_RESULTS]);
    assert_eq!(e.mem.u32(O16_RESULTS + 8), 0x7777, "the exe's");
}

/// O16 with the switch on but the lists not made yet (JIP puts its hooks in only with them): the
/// wrap runs the exe's `BuildCandidateList`, and the second-pass seam gives no answer.
#[test]
fn o16_before_the_lists_are_made_the_exes_runs() {
    let mut e = o16_world_unmade(&[screen(3)], false);
    install_fixes(&mut e);
    assert_eq!(e.ext_free::<LoadScreenLists>().0.get(), None);
    e.call(0x0078_ac60, &[O16_MENU, O16_RESULTS]);
    assert_eq!(e.mem.u32(O16_RESULTS + 8), 0x7777, "the exe's");
    e.mem.set_u32(O16_MENU + 0x1d0, screen(2));
    assert_eq!(pick_load_screen(&mut e, &[O16_MENU]), None);
    deferred_init(&mut e);
    assert_eq!(pick_load_screen(&mut e, &[O16_MENU]), Some(1), "made");
}

/// O16 `PickLoadScreenHook`: the second pass held (1) while the menu's list has a first screen,
/// not (0) when it has none; the switch off: no answer.
#[test]
fn o16_the_second_pass_only_for_an_empty_list() {
    let mut e = o16_world(&[]);
    assert_eq!(pick_load_screen(&mut e, &[O16_MENU]), Some(0));
    e.mem.set_u32(O16_MENU + 0x1d0, screen(2));
    assert_eq!(pick_load_screen(&mut e, &[O16_MENU]), Some(1));
    e.set_jip_patches(engine_core::extend::jip_patches::INSTALLED);
    assert_eq!(pick_load_screen(&mut e, &[O16_MENU]), None);
}

/// O16's lists are made by JIP's `DeferredInit` message (xNVSE's `kMessage_DeferredInit`).
#[test]
fn o16_the_deferred_init_message_makes_the_lists() {
    let mut e = o16_world_unmade(&[], false);
    // The same message makes JIP's position marker: its game calls as stand-ins.
    e.register(0x00aa_3e40, |_, _| Ret {
        eax: AREA_AT + 0x3a00,
        ..Ret::default()
    });
    e.register(crate::commands::pos_marker::REFR_CONSTRUCTOR, |_, w| Ret {
        eax: w[0],
        ..Ret::default()
    });
    e.register(crate::commands::pos_marker::SET_TEMPORARY, |_, _| {
        Ret::default()
    });
    assert_eq!(e.ext_free::<LoadScreenLists>().0.get(), None);
    crate::cosave::listen(
        &mut e,
        &nvse::plugins::Message {
            kind: nvse::plugins::msg::DEFERRED_INIT,
            data: 0,
            len: 0,
        },
    );
    assert_eq!(
        e.ext_free::<LoadScreenLists>().0.get(),
        Some(&(vec![screen(1), screen(3)], vec![screen(2), screen(4)]))
    );
}

/// O16 switched on while the game runs (`SetOptionalPatch(16, true)`): the lists made then, as
/// JIP makes them when its location list is empty; off and on again keeps them.
#[test]
fn o16_switched_on_at_run_time_makes_the_lists() {
    use engine_core::extend::jip_patches::{FIXES, LOAD_SCREEN_FIX};
    let mut e = o16_world_unmade(&[], false);
    e.jip_patches_and_not(LOAD_SCREEN_FIX);
    e.jip_patches_or(FIXES);
    assert_eq!(e.ext_free::<LoadScreenLists>().0.get(), None);
    assert!(crate::game_patches::switches::set(&mut e, 16, true));
    let made = Some((vec![screen(1), screen(3)], vec![screen(2), screen(4)]));
    assert_eq!(e.ext_free::<LoadScreenLists>().0.get(), made.as_ref());
    assert!(crate::game_patches::switches::set(&mut e, 16, false));
    assert!(!crate::game_patches::switches::is_in(&mut e, 16));
    assert!(crate::game_patches::switches::set(&mut e, 16, true));
    assert_eq!(e.ext_free::<LoadScreenLists>().0.get(), made.as_ref());
}
