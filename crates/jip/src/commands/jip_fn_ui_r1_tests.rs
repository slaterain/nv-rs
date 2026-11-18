//! `RefreshItemsList` (with JIP's `RefreshItemListBox` and `RefreshRecipeMenu`, and the main
//! loop's refresh step G60 asks for) and `ToggleHUDCursor` (with `GetMenuMode` and the main loop's
//! end of the mode), called by their opcodes. Expectations from JIP's bodies
//! (functions_jip/jip_fn_ui.h, internal/jip_core.cpp, jip_nvse.cpp's main loop message,
//! internal/dinput.cpp's `SetLMBDisabled`) and the exe's own recipe refresh at `00727620`,
//! never from the port's output. The game functions are stand-ins that record.

use super::*;
use crate::commands::test_support::*;

const IM_AT: u32 = FORMS + 0x1_0000;
const MENU: u32 = FORMS + 0x2_0000;
const LIST_VT: u32 = FORMS + 0x2_1000;
const CURSOR: u32 = FORMS + 0x3_0000;
const CURSOR_NODE: u32 = FORMS + 0x3_1000;
const ACTIVE: u32 = FORMS + 0x3_2000;
const VALUES: u32 = FORMS + 0x3_3000;
const LIST_VFNS: [u32; 4] = [0x0dea_1000, 0x0dea_1014, 0x0dea_1010, 0x0dea_101c];
const REFRESH_FN: u32 = 0x00a0_9410;
const ALL_FNS: [u32; 9] = [
    0x0078_2a90,
    0x007d_f230,
    0x0075_c280,
    0x0079_dbb0,
    0x0073_0690,
    0x0072_7680,
    0x0073_12e0,
    0x0072_9fe0,
    0x0072_a660,
];

fn record(addr: u32, w: &[u32]) {
    CALLS.with(|c| c.borrow_mut().push((addr, w.to_vec())));
}

fn recorded() -> Vec<(u32, Vec<u32>)> {
    calls()
        .into_iter()
        .filter(|c| c.0 != 0x00aa_3e40 && c.0 != 0x00aa_4060)
        .collect()
}

fn clear() {
    CALLS.with(|c| c.borrow_mut().clear());
}

fn setup() -> Engine {
    let mut e = engine();
    for page in [0x011d_8000, 0x011d_9000, 0x011d_a000, 0x011f_3000] {
        if !e.mem.is_mapped(page) {
            e.map(page, 0x1000);
        }
    }
    e.mem.set_u32(INTERFACE_MANAGER, IM_AT);
    macro_rules! stand_ins {
        ($($f:expr),*) => {$(
            e.register($f, |_, w| {
                record($f, w);
                Ret {
                    eax: 0x0dea_beef,
                    ..Ret::default()
                }
            });
        )*};
    }
    stand_ins!(
        ALL_FNS[0],
        ALL_FNS[1],
        ALL_FNS[2],
        ALL_FNS[3],
        ALL_FNS[4],
        ALL_FNS[5],
        ALL_FNS[6],
        ALL_FNS[7],
        ALL_FNS[8],
        LIST_VFNS[0],
        LIST_VFNS[1],
        LIST_VFNS[2],
        LIST_VFNS[3]
    );
    e.register(REFRESH_FN, |_, w| {
        record(REFRESH_FN, w);
        Ret::default()
    });
    // The recipe list box's virtual functions +0, +0x10, +0x14, +0x1C.
    for (slot, f) in [(0, 0), (0x10, 2), (0x14, 1), (0x1c, 3)] {
        e.mem.set_u32(LIST_VT + slot, LIST_VFNS[f]);
    }
    e.mem.set_u32(MENU + 0x6c, LIST_VT);
    e.mem.set_u32(MENU + 0x64, 0x0c47_0001);
    // `ToggleHUDCursor` is registered with its patch (H02, `game_patches::gp`).
    assert_ne!(crate::test_support::execute_of(&e, 0x28b5), 0);
    clear();
    e
}

fn show(e: &mut Engine, id: u32, on: bool) {
    e.mem.set_u8(MENU_SHOWN + id, u8::from(on));
}

// ---- RefreshItemsList ----

#[test]
fn refresh_calls_the_first_shown_menus_refresh() {
    let mut e = setup();
    assert_eq!(
        call_on(&mut e, 0x27e8, 0, &[]),
        (1, 0.0),
        "no menu: nothing"
    );
    assert!(recorded().is_empty());
    for (id, at) in [
        (STATS, STATS_MENU),
        (CONTAINER, CONTAINER_MENU),
        (MAP, MAP_MENU),
        (RECIPE, RECIPE_MENU),
    ] {
        e.mem.set_u32(at, MENU);
        show(&mut e, id, true);
    }
    show(&mut e, BARTER, true);
    show(&mut e, INVENTORY, true);
    call_on(&mut e, 0x27e8, 0, &[]);
    assert_eq!(
        recorded(),
        vec![(0x0078_2a90, vec![])],
        "the inventory first"
    );
    clear();
    show(&mut e, INVENTORY, false);
    call_on(&mut e, 0x27e8, 0, &[]);
    assert_eq!(recorded(), vec![(0x007d_f230, vec![MENU, 4])], "then stats");
    clear();
    show(&mut e, STATS, false);
    call_on(&mut e, 0x27e8, 0, &[]);
    assert_eq!(
        recorded(),
        vec![(0x0075_c280, vec![MENU, 0])],
        "then the container"
    );
    clear();
    show(&mut e, CONTAINER, false);
    // The map: only on its world map tab, and the next centring skipped.
    e.mem.set_u8(MENU + 0x80, 0x20);
    call_on(&mut e, 0x27e8, 0, &[]);
    assert!(
        recorded().is_empty(),
        "the local map tab: nothing, barter not reached"
    );
    assert!(!crate::game_patches::gb::map_skip_centring_pending(&mut e));
    e.mem.set_u8(MENU + 0x80, 0x21);
    call_on(&mut e, 0x27e8, 0, &[]);
    assert_eq!(recorded(), vec![(0x0079_dbb0, vec![MENU])]);
    assert!(crate::game_patches::gb::map_skip_centring_pending(&mut e));
    clear();
    show(&mut e, MAP, false);
    call_on(&mut e, 0x27e8, 0, &[]);
    assert_eq!(recorded(), vec![(0x0073_0690, vec![1])], "then barter");
    clear();
    show(&mut e, BARTER, false);
    call_on(&mut e, 0x27e8, 0, &[]);
    let list = MENU + 0x6c;
    assert_eq!(
        recorded(),
        vec![
            (0x0dea_101c, vec![list]),
            (0x0072_7680, vec![MENU, 0, 0x0c47_0001]),
            (0x0dea_1014, vec![list, 0, 0]),
            (0x0dea_1000, vec![list, 0x0dea_beef]),
            (0x0dea_1010, vec![list]),
            (0x0073_12e0, vec![list]),
            (0x0072_9fe0, vec![list, 0x0072_8d60]),
            (0x0072_a660, vec![list, 1]),
        ],
        "the recipe menu, as the exe's own refresh"
    );
}

#[test]
fn a_refresh_asked_for_is_made_at_the_next_main_loop() {
    let mut e = setup();
    show(&mut e, INVENTORY, true);
    main_loop(&mut e);
    assert!(recorded().is_empty(), "nothing asked");
    request_item_list_refresh(&mut e);
    main_loop(&mut e);
    assert_eq!(recorded(), vec![(0x0078_2a90, vec![])]);
    main_loop(&mut e);
    assert_eq!(recorded().len(), 1, "once");
}

// ---- ToggleHUDCursor ----

/// The cursor tile with its `visible` value (1.0) and node, the active tile with `mouseover`
/// (1.0), game mode.
fn cursor_scene(e: &mut Engine) {
    e.mem.set_u32(IM_AT + IM_CURRENT_MODE, 1);
    e.mem.set_u32(IM_AT + IM_CURSOR, CURSOR);
    e.mem.set_u32(CURSOR + 0x2c, CURSOR_NODE);
    e.mem.set_u8(CURSOR_NODE + 0x30, 0x40);
    for (tile, id, value) in [
        (CURSOR, TRAIT_VISIBLE, VALUES),
        (ACTIVE, TRAIT_MOUSEOVER, VALUES + 0x40),
    ] {
        e.mem.set_u32(tile + 0x14, value + 0x20);
        e.mem.set_u32(tile + 0x18, 1);
        e.mem.set_u32(value + 0x20, value);
        e.mem.set_u32(value, id);
        e.mem.set_f32(value + 8, 1.0);
    }
    e.mem.set_u32(IM_AT + IM_ACTIVE_TILE, ACTIVE);
    e.mem.set_u32(IM_AT + IM_ACTIVE_MENU, MENU);
    e.mem.set_u8(IM_AT + 0x170, 7);
    e.mem.set_u8(IM_AT + 0x4b4, 9);
}

fn lmb_disabled(e: &mut Engine) -> (bool, bool) {
    let k = e.ext::<nvse::Nvse>().input.keys[0x100];
    (k.user_disable, k.script_disable)
}

#[test]
fn hud_cursor_on_and_off() {
    let mut e = setup();
    cursor_scene(&mut e);
    assert_eq!(call_on(&mut e, 0x28b5, 0, &[]), (1, 0.0));
    assert!(!hud_cursor_mode(&mut e), "no argument: only the answer");
    set_last_lmb_state(&mut e, true);
    assert_eq!(
        call_on(&mut e, 0x28b5, 0, &[n(3)]),
        (1, 0.0),
        "the mode before"
    );
    assert!(hud_cursor_mode(&mut e));
    assert_eq!((e.mem.u8(IM_AT + 0x170), e.mem.u8(IM_AT + 0x4b4)), (0, 0));
    assert_eq!(e.mem.u32(IM_AT + IM_ACTIVE_TILE), 0);
    assert_eq!(e.mem.u32(IM_AT + IM_ACTIVE_MENU), 0);
    assert_eq!(
        lmb_disabled(&mut e),
        (true, false),
        "the player's left button only"
    );
    assert!(!last_lmb_state(&mut e));
    assert_eq!(e.mem.f32(VALUES + 8), 1.0, "turning on leaves the cursor");
    // `GetMenuMode` sees it.
    assert!(menu_mode(&mut e));
    // Already on: nothing.
    e.mem.set_u8(IM_AT + 0x170, 5);
    assert_eq!(call_on(&mut e, 0x28b5, 0, &[n(1)]), (1, 1.0));
    assert_eq!(e.mem.u8(IM_AT + 0x170), 5);
    // Off: the cursor hidden, the active tile's mouse-over off.
    e.mem.set_u32(IM_AT + IM_ACTIVE_TILE, ACTIVE);
    clear();
    assert_eq!(call_on(&mut e, 0x28b5, 0, &[n(0)]), (1, 1.0));
    assert!(!hud_cursor_mode(&mut e));
    assert_eq!(e.mem.f32(VALUES + 8), 0.0, "visible 0");
    assert_eq!(e.mem.f32(VALUES + 0x48), 0.0, "mouseover 0");
    assert_eq!(
        recorded(),
        vec![
            (REFRESH_FN, vec![VALUES, 1]),
            (REFRESH_FN, vec![VALUES + 0x40, 1])
        ]
    );
    assert_eq!(e.mem.u8(CURSOR_NODE + 0x30), 0x41, "the node hidden");
    assert_eq!(e.mem.u8(IM_AT + 0x170), 5, "off leaves the bytes");
    assert_eq!(e.mem.u32(IM_AT + IM_ACTIVE_TILE), 0);
    assert_eq!(lmb_disabled(&mut e), (false, false));
}

#[test]
fn hud_cursor_needs_game_mode_and_no_controller() {
    let mut e = setup();
    cursor_scene(&mut e);
    e.mem.set_u32(IM_AT + IM_CURRENT_MODE, 2);
    call_on(&mut e, 0x28b5, 0, &[n(1)]);
    assert!(!hud_cursor_mode(&mut e), "menu mode");
    e.mem.set_u32(IM_AT + IM_CURRENT_MODE, 1);
    crate::input::set_ready_for_test(&mut e, true);
    call_on(&mut e, 0x28b5, 0, &[n(1)]);
    assert!(!hud_cursor_mode(&mut e), "a controller in use");
    assert_eq!(lmb_disabled(&mut e), (false, false));
    assert_eq!(e.mem.u8(IM_AT + 0x170), 7);
}

#[test]
fn the_main_loop_ends_the_mode_in_a_menu() {
    let mut e = setup();
    cursor_scene(&mut e);
    call_on(&mut e, 0x28b5, 0, &[n(1)]);
    main_loop(&mut e);
    assert!(hud_cursor_mode(&mut e), "still game mode");
    e.mem.set_u32(IM_AT + IM_CURRENT_MODE, 2);
    main_loop(&mut e);
    assert!(!hud_cursor_mode(&mut e));
    assert_eq!(lmb_disabled(&mut e), (false, false));
    assert!(menu_mode(&mut e), "menu mode itself");
    e.mem.set_u32(IM_AT + IM_CURRENT_MODE, 1);
    assert!(!menu_mode(&mut e));
}

/// The recipe filter is the word the exe's code holds at `00727637` when the guest has it.
#[test]
fn the_recipe_refresh_takes_the_filter_word_in_the_code() {
    let mut e = setup();
    if !e.mem.is_mapped(0x0072_7000) {
        e.map(0x0072_7000, 0x1000);
    }
    e.mem.set_u32(0x0072_7637, 0x00ab_cdef);
    e.mem.set_u32(RECIPE_MENU, MENU);
    show(&mut e, RECIPE, true);
    call_on(&mut e, 0x27e8, 0, &[]);
    let list = MENU + 0x6c;
    assert!(recorded().contains(&(0x0072_9fe0, vec![list, 0x00ab_cdef])));
}

// ---- H02: the HUD cursor patch with ToggleHUDCursor ----

/// The HUD mouse handling's game calls as stand-ins: the tile under the cursor (`HUD_TILE`), a
/// trait's number set recorded, sounds recorded, the menu's click handler recorded.
const HUD_TILE: u32 = FORMS + 0x3_4000;
const HUD_MENU: u32 = FORMS + 0x3_5000;
const HUD_MENU_VT: u32 = FORMS + 0x3_6000;
const CLICK_FN: u32 = 0x0dea_2000;
const PLAYER_AT: u32 = FORMS + 0x3_7000;
const INPUT_AT: u32 = FORMS + 0x3_8000;

fn hud_stand_ins(e: &mut Engine) {
    e.mem.set_u32(0x011d_96c0, HUD_MENU);
    e.mem.set_u32(0x011f_35cc, INPUT_AT);
    e.mem.set_u8(INPUT_AT + 4, 0xff);
    e.mem.set_u32(HUD_MENU, HUD_MENU_VT);
    e.mem.set_u32(HUD_MENU_VT + 0xc, CLICK_FN);
    e.register(CLICK_FN, |_, w| {
        record(CLICK_FN, w);
        Ret::default()
    });
    e.register(0x0071_18d0, |_, w| {
        record(0x0071_18d0, w);
        Ret::default()
    });
    e.register(0x0071_26c0, |_, _| Ret {
        eax: HUD_TILE,
        ..Ret::default()
    });
    e.register(0x00a0_b110, |_, w| {
        record(0x00a0_b110, w);
        Ret::default()
    });
    e.register(0x00a0_12d0, |_, w| {
        record(0x00a0_12d0, w);
        Ret::default()
    });
}

/// H02 with its command: before `ToggleHUDCursor 1` the look handling's vanity test is the
/// player's flag; after it, the test answers true and runs JIP's HUD mouse handling: the tile under
/// the cursor gets the mouse-over (its sound, `mouseover` 1, the HUD as the active menu), and a
/// press and release of the left button over it is a click (its sound, `clicked` 1 then 0, the
/// menu's click handler with the tile's id, -1 without one).
#[test]
fn h02_the_hud_cursor_handles_the_mouse_over_the_hud() {
    let mut e = setup();
    cursor_scene(&mut e);
    hud_stand_ins(&mut e);
    let seam = crate::game_patches::gp::VANITY_DISABLED.addr();
    e.mem.set_u8(PLAYER_AT + 0x680, 2);
    assert_eq!(e.site(seam, &[PLAYER_AT]), Some(1), "the player's flag");
    e.mem.set_u8(PLAYER_AT + 0x680, 0);
    assert_eq!(e.site(seam, &[PLAYER_AT]), Some(0));
    assert!(
        recorded().is_empty(),
        "no mouse handling while the cursor is off"
    );
    call_on(&mut e, 0x28b5, 0, &[n(1)]);
    clear();
    // The first frame: the tile under the cursor gets the mouse-over.
    assert_eq!(e.site(seam, &[PLAYER_AT]), Some(1));
    assert_eq!(
        e.mem.u8(INPUT_AT + 4),
        0xf7,
        "the input flag's bit 3 cleared"
    );
    assert_eq!(e.mem.u32(IM_AT + IM_ACTIVE_TILE), HUD_TILE);
    assert_eq!(e.mem.u32(IM_AT + IM_ACTIVE_MENU), HUD_MENU);
    let one = 1.0f32.to_bits();
    assert_eq!(
        recorded(),
        vec![
            (0x0071_18d0, vec![IM_AT]),
            (0x00a0_b110, vec![HUD_TILE, 0xfe8]),
            (0x00a0_12d0, vec![HUD_TILE, 0xfc3, one, 0]),
        ]
    );
    // Pressed: only remembered.
    crate::test_support::key(&mut e, 0x100).raw_state = true;
    clear();
    e.site(seam, &[PLAYER_AT]);
    assert!(last_lmb_state(&mut e));
    assert_eq!(recorded(), vec![(0x0071_18d0, vec![IM_AT])]);
    // Released over the tile: the click.
    crate::test_support::key(&mut e, 0x100).raw_state = false;
    clear();
    e.site(seam, &[PLAYER_AT]);
    assert!(!last_lmb_state(&mut e));
    assert_eq!(
        recorded(),
        vec![
            (0x0071_18d0, vec![IM_AT]),
            (0x00a0_b110, vec![HUD_TILE, 0xfcb]),
            (0x00a0_12d0, vec![HUD_TILE, 0xfc7, one, 0]),
            (0x00a0_12d0, vec![HUD_TILE, 0xfc7, 0, 0]),
            (CLICK_FN, vec![HUD_MENU, 0xffff_ffff, HUD_TILE]),
        ]
    );
}

thread_local! {
    /// The trait numbers the HUD stand-ins keep: (tile, trait) and its number.
    static TRAITS: std::cell::RefCell<Vec<((u32, u32), f32)>> = const { std::cell::RefCell::new(Vec::new()) };
}

fn trait_of(tile: u32, id: u32) -> Option<f32> {
    TRAITS.with(|t| {
        t.borrow()
            .iter()
            .rev()
            .find(|(k, _)| *k == (tile, id))
            .map(|(_, v)| *v)
    })
}

/// The HUD stand-ins with a cursor: the scale 2, the tile's absolute place (10, 20), its traits
/// kept (`x` 3, `y` 4 at the start; what is set is read back), `draggable` and `wheelable` by the
/// traits kept (1.0 true).
fn drag_stand_ins(e: &mut Engine) {
    TRAITS.with(|t| t.borrow_mut().clear());
    hud_stand_ins(e);
    for (id, v) in [(0xfa1, 3.0), (0xfa2, 4.0)] {
        TRAITS.with(|t| t.borrow_mut().push(((HUD_TILE, id), v)));
    }
    e.register(0x0070_ecb0, |_, _| Ret {
        st0: 2.0,
        ..Ret::default()
    });
    e.register(0x00a0_13d0, |_, _| Ret {
        st0: 10.0,
        ..Ret::default()
    });
    e.register(0x00a0_1440, |_, _| Ret {
        st0: 20.0,
        ..Ret::default()
    });
    e.register(0x00a0_11b0, |_, w| Ret {
        st0: f64::from(trait_of(w[0], w[1]).unwrap_or(0.0)),
        ..Ret::default()
    });
    e.register(0x00a0_12d0, |_, w| {
        record(0x00a0_12d0, w);
        TRAITS.with(|t| t.borrow_mut().push(((w[0], w[1]), f32::from_bits(w[2]))));
        Ret::default()
    });
    e.register(0x00a0_1230, |_, w| Ret {
        eax: u32::from(trait_of(w[0], w[1]) == Some(1.0)),
        ..Ret::default()
    });
}

fn cursor_at(e: &mut Engine, x: f32, y: f32) {
    e.mem.set_f32(IM_AT + 0x38, x);
    e.mem.set_f32(IM_AT + 0x40, y);
}

fn set_floats() -> Vec<(u32, f32)> {
    recorded()
        .into_iter()
        .filter(|(f, _)| *f == 0x00a0_12d0)
        .map(|(_, w)| (w[1], f32::from_bits(w[2])))
        .collect()
}

/// H02's drag: a press over a draggable tile is remembered; held the next frame it starts the drag
/// (the interface's drag words and the tile's drag values); held with the cursor moved the tile
/// follows; released, the drag ends (its starts -1, the words cleared, the button state false).
#[test]
fn h02_a_draggable_tile_is_dragged_and_let_go() {
    let mut e = setup();
    cursor_scene(&mut e);
    drag_stand_ins(&mut e);
    TRAITS.with(|t| t.borrow_mut().push(((HUD_TILE, 0xfe9), 1.0)));
    let seam = crate::game_patches::gp::VANITY_DISABLED.addr();
    call_on(&mut e, 0x28b5, 0, &[n(1)]);
    cursor_at(&mut e, 100.0, 50.0);
    e.site(seam, &[PLAYER_AT]); // the mouse-over
    crate::test_support::key(&mut e, 0x100).raw_state = true;
    e.site(seam, &[PLAYER_AT]); // the press, remembered
    for at in [0x54, 0x58, 0x60, 0x64] {
        e.mem.set_u32(IM_AT + at, 0x77);
    }
    clear();
    e.site(seam, &[PLAYER_AT]); // held: the drag starts
    assert_eq!(e.mem.u32(IM_AT + 0x4c), HUD_TILE);
    assert_eq!(e.mem.u32(IM_AT + 0xd4), HUD_TILE);
    assert_eq!(e.mem.u32(IM_AT + 0xd8), HUD_MENU);
    assert_eq!((e.mem.u32(IM_AT + 0x50), e.mem.u32(IM_AT + 0x5c)), (50, 25));
    for at in [0x54, 0x58, 0x60, 0x64] {
        assert_eq!(e.mem.u32(IM_AT + at), 0, "{at:x}");
    }
    assert_eq!(
        set_floats(),
        vec![
            (0xff0, 43.0),
            (0xff1, 9.0),
            (0xfea, 50.0),
            (0xfeb, 25.0),
            (0xfec, 40.0),
            (0xfed, 5.0)
        ]
    );
    // Held and moved: the tile follows.
    cursor_at(&mut e, 120.0, 60.0);
    clear();
    e.site(seam, &[PLAYER_AT]);
    assert_eq!((e.mem.u32(IM_AT + 0x60), e.mem.u32(IM_AT + 0x64)), (10, 5));
    assert_eq!(set_floats(), vec![(0xff0, 53.0), (0xff1, 14.0)]);
    // Released: let go.
    crate::test_support::key(&mut e, 0x100).raw_state = false;
    clear();
    e.site(seam, &[PLAYER_AT]);
    assert_eq!(e.mem.u32(IM_AT + 0x4c), 0);
    assert_eq!(e.mem.u32(IM_AT + 0xd4), 0);
    assert_eq!(e.mem.u32(IM_AT + 0xd8), 0);
    assert_eq!(set_floats()[..2], [(0xfea, -1.0), (0xfeb, -1.0)]);
    assert!(!last_lmb_state(&mut e));
}

/// H02's wheel: over a tile that is not wheelable, its wheelable parent gets `wheelmoved` set to the
/// wheel's steps (the delta over -120) and back to 0: 240 is -2, -120 is 1.
#[test]
fn h02_the_wheel_moves_the_nearest_wheelable_parent() {
    let mut e = setup();
    cursor_scene(&mut e);
    drag_stand_ins(&mut e);
    const PARENT: u32 = FORMS + 0x3_9000;
    e.mem.set_u32(HUD_TILE + 0x28, PARENT);
    TRAITS.with(|t| t.borrow_mut().push(((PARENT, 0xff2), 1.0)));
    let seam = crate::game_patches::gp::VANITY_DISABLED.addr();
    call_on(&mut e, 0x28b5, 0, &[n(1)]);
    e.site(seam, &[PLAYER_AT]); // the mouse-over: the tile is active
    for (wheel, steps) in [(240i32, -2.0f32), (-120, 1.0)] {
        e.mem.set_u32(INPUT_AT + 0x1b2c, wheel as u32);
        clear();
        e.site(seam, &[PLAYER_AT]);
        let moved: Vec<(u32, u32, f32)> = recorded()
            .into_iter()
            .filter(|(f, _)| *f == 0x00a0_12d0)
            .map(|(_, w)| (w[0], w[1], f32::from_bits(w[2])))
            .collect();
        assert_eq!(
            moved,
            vec![(PARENT, 0xff3, steps), (PARENT, 0xff3, 0.0)],
            "{wheel}"
        );
    }
}
