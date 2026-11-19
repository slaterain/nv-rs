//! A player's menus did not follow the mouse at all while it still turned the camera (report 002,
//! the Windows preview of 2026-11-17). The game moves its menu cursor in one of two ways, chosen
//! by `Fallout.ini`'s `[Controls] bBackground Mouse` (the input start `00a22660` sets bit 3 of the
//! controls' flags for it, `00711e00` reads it): without the key from the mouse's DirectInput
//! motion (`007118d0`, which returns early with the key), with it from each `WM_MOUSEMOVE` its
//! window procedure gets (`0086a0a0` hands it to `00871d10`). The port never handed the game's
//! procedure a `WM_MOUSEMOVE`, so with the key the menus' cursor never moved. Now the platform
//! posts one for each move of the cursor over the game's window while the game's mouse is not
//! exclusive (`engine_platform::window::post_mouse_move`, tested there); here the game's own
//! procedure and handler, translated, take it, with the key on and off. Here because this crate
//! registers every translation.

use engine as _;
use engine_core::prelude::*;
use std::cell::Cell;

/// The game's window procedure and the `Main` singleton (its window at `+0x08`).
const WINDOW_PROCEDURE: u32 = 0x0086_a0a0;
const MAIN: u32 = 0x011d_ea0c;
/// The import slots of `GetActiveWindow` and `DefWindowProcA`.
const GET_ACTIVE_WINDOW: u32 = 0x00fd_f2fc;
const DEF_WINDOW_PROC: u32 = 0x00fd_f2e0;
/// `Interface::IsInMenuMode`, `Interface::IsTopMenuID`, `Controls::GetInstance`: stood in (menu
/// mode, no menu `0x41e` on top, the controls object at [`CONTROLS`]).
const IS_IN_MENU_MODE: u32 = 0x0070_2360;
const IS_TOP_MENU_ID: u32 = 0x0070_2450;
const CONTROLS_GET_INSTANCE: u32 = 0x007f_df30;
/// Where the menus' cursor is placed from a window position (fractions across and down): stood in,
/// to see the call.
const PLACE_MENU_CURSOR: u32 = 0x0070_3080;
/// The renderer's width and height (`004dc1f0`, `004dc200` read them).
const RENDER_WIDTH: u32 = 0x0118_947c;
const RENDER_HEIGHT: u32 = 0x0118_9480;
/// The test's own memory: `Main`, the controls object.
const AREA: u32 = 0x0400_0000;
const MAIN_OBJECT: u32 = AREA;
const CONTROLS: u32 = AREA + 0x100;
/// The game's main window and the renderer's child of it, which gets the moves.
const WINDOW: u32 = 0x94;
const CHILD: u32 = 0xc0;
/// `bBackground Mouse`'s bit in the controls' flags (`+4`).
const BACKGROUND_MOUSE: u32 = 8;
/// `Main::SetActive`, `00877720` (`Main`'s controls) and `00a23c00` (the controls make the mouse
/// again, exclusive for 0): stood in, to see an activation's calls.
const MAIN_SET_ACTIVE: u32 = 0x0087_1c90;
const MAIN_CONTROLS: u32 = 0x0087_7720;
const REMAKE_MOUSE: u32 = 0x00a2_3c00;
/// `WM_ACTIVATE` and its `WA_ACTIVE`, `WA_CLICKACTIVE`.
const WM_ACTIVATE: u32 = 6;

thread_local! {
    /// The menu cursor's placing call (fractions across and down, as `f32` bits), if made.
    static PLACED: Cell<Option<[u32; 2]>> = const { Cell::new(None) };
    /// Whether `DefWindowProcA` got the message.
    static DEFAULTED: Cell<bool> = const { Cell::new(false) };
    /// `Main::SetActive`'s word, if called.
    static SET_ACTIVE: Cell<Option<u32>> = const { Cell::new(None) };
    /// `00a23c00`'s two words (the controls, the word), if called.
    static REMADE: Cell<Option<[u32; 2]>> = const { Cell::new(None) };
}

fn ret(eax: u32) -> Ret {
    Ret {
        eax,
        ..Ret::default()
    }
}

/// An engine with every translation, the stand-ins named above, `Main` and a 1920 by 1080
/// renderer (the player's), the controls' flags `flags`.
fn engine(flags: u32) -> Engine {
    let mut e = Engine::new_untyped_for_test();
    for (at, len) in [
        (AREA, 0x1000),
        (MAIN & !0xfff, 0x1000),
        (RENDER_WIDTH & !0xfff, 0x1000),
    ] {
        if !e.mem.is_mapped(at) {
            e.map(at, len);
        }
    }
    e.mem.set_u32(MAIN, MAIN_OBJECT);
    e.mem.set_u32(MAIN_OBJECT + 8, WINDOW);
    e.mem.set_u32(CONTROLS + 4, flags);
    e.mem.set_u32(RENDER_WIDTH, 1920);
    e.mem.set_u32(RENDER_HEIGHT, 1080);
    e.register(GET_ACTIVE_WINDOW, |_, _| ret(WINDOW));
    e.register(DEF_WINDOW_PROC, |_, a| {
        assert_eq!(a.len(), 4, "DefWindowProcA's four words");
        DEFAULTED.with(|d| d.set(true));
        ret(0)
    });
    e.register(IS_IN_MENU_MODE, |_, _| ret(1));
    e.register(IS_TOP_MENU_ID, |_, _| ret(0));
    e.register(CONTROLS_GET_INSTANCE, |_, _| ret(CONTROLS));
    e.register(PLACE_MENU_CURSOR, |_, a| {
        PLACED.with(|p| p.set(Some([a[0], a[1]])));
        ret(0)
    });
    e.register(MAIN_SET_ACTIVE, |_, a| {
        SET_ACTIVE.with(|s| s.set(Some(a[1])));
        ret(0)
    });
    e.register(MAIN_CONTROLS, |_, _| ret(CONTROLS));
    e.register(REMAKE_MOUSE, |_, a| {
        REMADE.with(|r| r.set(Some([a[0], a[1]])));
        ret(0)
    });
    PLACED.with(|p| p.set(None));
    DEFAULTED.with(|d| d.set(false));
    SET_ACTIVE.with(|s| s.set(None));
    REMADE.with(|r| r.set(None));
    e
}

/// The `WM_ACTIVATE` the platform posts when the real window becomes active again.
fn activation(e: &mut Engine, wparam: u32) -> u32 {
    e.call(WINDOW_PROCEDURE, &args![WINDOW, WM_ACTIVATE, wparam, 0u32])
        .u32()
}

/// The `WM_MOUSEMOVE` the platform posts for the cursor at (x, y) of the child's client area.
fn mouse_move(e: &mut Engine, x: u32, y: u32) -> u32 {
    e.call(
        WINDOW_PROCEDURE,
        &args![CHILD, 0x200u32, 0u32, (y << 16) | x],
    )
    .u32()
}

/// With `bBackground Mouse=1`, a move to the middle of a 1920 by 1080 window places the menu
/// cursor at a half each way, and the procedure answers 0 (handled); a move to the window's top
/// left corner at 0, 0.
#[test]
fn with_background_mouse_the_menu_cursor_follows_the_window_moves() {
    let mut e = engine(BACKGROUND_MOUSE);
    assert_eq!(mouse_move(&mut e, 960, 540), 0);
    assert_eq!(
        PLACED.with(Cell::get),
        Some([0.5f32.to_bits(), 0.5f32.to_bits()]),
        "the middle of the window"
    );
    assert!(!DEFAULTED.with(Cell::get), "handled by the game");
    mouse_move(&mut e, 0, 0);
    assert_eq!(PLACED.with(Cell::get), Some([0, 0]), "the top left corner");
}

/// Without the key (the game's default), a move is not the menus' (their cursor follows the
/// mouse's DirectInput motion): the game passes it to `DefWindowProcA`, which answers 0.
#[test]
fn without_background_mouse_a_window_move_is_left_to_the_default_procedure() {
    let mut e = engine(0);
    assert_eq!(mouse_move(&mut e, 960, 540), 0);
    assert_eq!(
        PLACED.with(Cell::get),
        None,
        "the menu cursor is not placed"
    );
    assert!(DEFAULTED.with(Cell::get), "DefWindowProcA answered it");
}

/// The game takes a window move only for its active window (`00871d10` asks `GetActiveWindow`):
/// with another window active the menu cursor is not placed and the move is left to
/// `DefWindowProcA`. (The window layers also keep no place while another program is active.)
#[test]
fn a_move_while_another_window_is_active_places_nothing() {
    let mut e = engine(BACKGROUND_MOUSE);
    e.register(GET_ACTIVE_WINDOW, |_, _| ret(0x1234));
    assert_eq!(mouse_move(&mut e, 960, 540), 0);
    assert_eq!(PLACED.with(Cell::get), None, "not the active window");
    assert!(DEFAULTED.with(Cell::get));
}

/// The real window becoming active again (Alt+Tab back, a click on it) reaches the game's
/// procedure as `WM_ACTIVATE`, which, once `Main` exists and the window is not minimised, sets
/// `Main` active and has the controls make the mouse again with 0 (`00a23c00`: exclusive, level
/// 5, `bBackground Mouse`'s flag cleared), so the menus follow the mouse's motion again, as in the
/// original. A minimised activation, or one before `Main`, changes nothing.
#[test]
fn an_activation_makes_the_games_mouse_exclusive_again() {
    for state in [1u32, 2] {
        let mut e = engine(BACKGROUND_MOUSE);
        assert_eq!(activation(&mut e, state), 0);
        assert_eq!(SET_ACTIVE.with(Cell::get), Some(1), "WA {state}");
        assert_eq!(REMADE.with(Cell::get), Some([CONTROLS, 0]), "WA {state}");
    }
    let mut e = engine(BACKGROUND_MOUSE);
    assert_eq!(activation(&mut e, 0x0001_0001), 0);
    assert_eq!(REMADE.with(Cell::get), None, "minimised");
    let mut e = engine(BACKGROUND_MOUSE);
    e.mem.set_u32(MAIN, 0);
    assert_eq!(activation(&mut e, 1), 0);
    assert_eq!(REMADE.with(Cell::get), None, "before Main");
}
