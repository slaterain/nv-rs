//! Tests of `SetCursorPos`, called by its opcode. Expectations from JIP's body
//! (functions_jip/jip_fn_ui.h), its `GetMenuMode` (internal/jip_core.cpp) and the three values its
//! `DeferredInit` keeps (internal/patches_game.h), never from the port's output.

use crate::commands::test_support::*;
use engine_core::prelude::*;

const SET_CURSOR_POS: u32 = 0x2866;
const TOGGLE_HUD_CURSOR: u32 = 0x28b5;
const INTERFACE_MANAGER: u32 = 0x011d_8a80;
const IM_AT: u32 = FORMS + 0x1_0000;
const CURSOR: u32 = FORMS + 0x3_0000;
const CURSOR_NODE: u32 = FORMS + 0x3_1000;

/// A 1920 x 1080 display, the ratio 0.75, the interface manager with its cursor tile and node;
/// `mode` the interface's mode (1 the game, 2 or more a menu).
fn setup(mode: u32) -> Engine {
    let mut e = engine();
    for page in [0x011c_7000, 0x011d_8000] {
        if !e.mem.is_mapped(page) {
            e.map(page, 0x1000);
        }
    }
    e.mem.set_u32(INTERFACE_MANAGER, IM_AT);
    e.mem.set_u32(IM_AT + 0xc, mode);
    e.mem.set_u32(IM_AT + 0x28, CURSOR);
    e.mem.set_u32(CURSOR + 0x2c, CURSOR_NODE);
    e.mem.set_f32(0x011d_8a48, 0.75);
    e.mem.set_u32(0x011c_73dc + 4, 1920);
    e.mem.set_u32(0x011c_718c + 4, 1080);
    for at in [0x38, 0x40] {
        e.mem.set_f32(IM_AT + at, -1.0);
    }
    for at in [0x58, 0x5c, 0x60] {
        e.mem.set_f32(CURSOR_NODE + at, -2.0);
    }
    e
}

fn cursor(e: &Engine) -> (f32, f32) {
    (e.mem.f32(IM_AT + 0x38), e.mem.f32(IM_AT + 0x40))
}

fn node(e: &Engine) -> [f32; 3] {
    [0x58, 0x5c, 0x60].map(|at| e.mem.f32(CURSOR_NODE + at))
}

/// In menu mode: the manager's x and y, the node's x `x * 0.75 - 1920 * 0.375` and z
/// `1080 * 0.375 - y * 0.75` (y untouched); no result.
#[test]
fn set_cursor_pos_moves_the_cursor_in_menu_mode() {
    let mut e = setup(2);
    let r = call(&mut e, SET_CURSOR_POS, &[z(400.0), z(300.5)]).1;
    assert_eq!(r, 0.0);
    assert_eq!(cursor(&e), (400.0, 300.5));
    let half = 0.75f32 * 0.5;
    let want_x = 400.0f32 * 0.75 - 1920.0 * half;
    let want_z = 1080.0f32 * half - 300.5 * 0.75;
    assert_eq!(node(&e), [want_x, -2.0, want_z]);
    assert_eq!(node(&e), [-420.0, -2.0, 179.625]);
    // Any menu mode above 1.
    let mut e = setup(3);
    call(&mut e, SET_CURSOR_POS, &[z(0.0), z(0.0)]);
    assert_eq!(cursor(&e), (0.0, 0.0));
    assert_eq!(node(&e), [-720.0, -2.0, 405.0]);
}

/// In the game (mode 1, or 0) with the HUD cursor off: nothing moves.
#[test]
fn set_cursor_pos_does_nothing_outside_menu_mode() {
    for mode in [0, 1] {
        let mut e = setup(mode);
        call(&mut e, SET_CURSOR_POS, &[z(400.0), z(300.0)]);
        assert_eq!(cursor(&e), (-1.0, -1.0), "mode {mode}");
        assert_eq!(node(&e), [-2.0; 3], "mode {mode}");
    }
}

/// JIP's `GetMenuMode` is also true with its HUD cursor on (`ToggleHUDCursor 1` in the game):
/// then the cursor moves in mode 1.
#[test]
fn set_cursor_pos_works_with_the_hud_cursor_on() {
    let mut e = setup(1);
    // `ToggleHUDCursor` is registered with its patch H02 (on main).
    call(&mut e, TOGGLE_HUD_CURSOR, &[n(1)]);
    assert!(super::super::jip_fn_ui::hud_cursor_mode(&mut e));
    call(&mut e, SET_CURSOR_POS, &[z(10.0), z(20.0)]);
    assert_eq!(cursor(&e), (10.0, 20.0));
    assert_eq!(node(&e), [10.0 * 0.75 - 720.0, -2.0, 405.0 - 20.0 * 0.75]);
}

/// The cursor's x and y are set before the node is read: with no cursor tile or node (JIP writes
/// through them unchecked) the manager still takes the place and nothing else is written.
#[test]
fn set_cursor_pos_without_a_cursor_node() {
    let mut e = setup(2);
    e.mem.set_u32(CURSOR + 0x2c, 0);
    call(&mut e, SET_CURSOR_POS, &[z(5.0), z(6.0)]);
    assert_eq!(cursor(&e), (5.0, 6.0));
    assert_eq!(node(&e), [-2.0; 3]);
    let mut e = setup(2);
    e.mem.set_u32(IM_AT + 0x28, 0);
    call(&mut e, SET_CURSOR_POS, &[z(7.0), z(8.0)]);
    assert_eq!(cursor(&e), (7.0, 8.0));
    assert_eq!(node(&e), [-2.0; 3]);
}
