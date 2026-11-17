//! Group GA's game patches: G08, G11, G17, G21, G35, G37, G58, H21. Each handler's branches over
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
    assert_eq!(
        AREA.fix_sites,
        [
            sites,
            corner,
            vec![ROLLOVER_WEAPON.addr(), VATS_TARGET_DT.addr()]
        ]
        .concat()
    );
    assert!(AREA.fix_wraps.is_empty() && AREA.jip_sites.is_empty() && AREA.jip_wraps.is_empty());
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
