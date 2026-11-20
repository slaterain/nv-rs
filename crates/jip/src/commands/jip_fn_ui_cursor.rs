//! `functions_jip/jip_fn_ui.h` (JIP LN NVSE): `SetCursorPos`, the mouse cursor put at a place on
//! the screen. The rest of the file's commands are in `jip_fn_ui.rs` and the `jip_fn_ui_*` modules.
//!
//! JIP keeps the cursor's node and half the screen's size from its deferred start (`DeferredInit`,
//! internal/patches_game.h: `g_cursorNode` = the interface manager's cursor tile's node,
//! `g_screenWidth` / `g_screenHeight` = `iSize W` / `iSize H` of `[Display]` times half the cached
//! screen ratio). The port reads the same words at the call: the cursor tile is made once with the
//! interface, the display size settings are read at start-up and the ratio (`0x11D8A48`) is a value
//! `00707B60` works out once and keeps, so the numbers are JIP's unless a script changes the display
//! size setting in game (JIP would keep the old size).
//!
//! Threads: on the script's thread, as JIP's.

use super::jip_fn_ui::{interface_manager, menu_mode};
use super::Handlers;
use engine_core::prelude::*;
use nvse::commands::Read;
use nvse::{handled, CommandArgs};

#[rustfmt::skip]
pub static COMMANDS: Handlers = &[
    (0x2866, set_cursor_pos),
];

/// The interface manager's cursor tile (+0x28) and the cursor's x and y (+0x38, +0x40); a tile's
/// node (+0x2C) and a node's local translation (+0x58: x, y, z).
const IM_CURSOR: u32 = 0x28;
const IM_CURSOR_X: u32 = 0x38;
const IM_CURSOR_Y: u32 = 0x40;
const TILE_NODE: u32 = 0x2c;
const LOCAL_X: u32 = 0x58;
const LOCAL_Z: u32 = 0x60;
/// The cached screen ratio (`0x11D8A48`, JIP's `converter`) and the integer values (+4) of the
/// settings `iSize W:Display` (`0x11C73DC`) and `iSize H:Display` (`0x11C718C`).
const SCREEN_RATIO: u32 = 0x011d_8a48;
const SIZE_W: u32 = 0x011c_73dc + 4;
const SIZE_H: u32 = 0x011c_718c + 4;

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_ui.h: Cmd_SetCursorPos_Execute
// (and internal/patches_game.h's DeferredInit for the three values it keeps).
/// `SetCursorPos x y`, in menu mode (JIP's `GetMenuMode`: a menu open or the HUD cursor on): the
/// interface manager's cursor x and y set, and the cursor's node moved there: x `x * ratio - width`,
/// z `height - y * ratio`, where width and height are the display size times half the ratio. All in
/// `f32`, as JIP's SSE code. No result.
fn set_cursor_pos(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if !read.ok || !menu_mode(e) {
        return handled(true);
    }
    // `menu_mode` answers true only with an interface manager or the HUD cursor on; the game
    // always has the manager and its cursor tile and node (JIP writes through them unchecked).
    let im = interface_manager(e);
    if im == 0 {
        return handled(true);
    }
    let (x, y) = (read.float(0, 0.0), read.float(1, 0.0));
    e.mem.set_f32(im + IM_CURSOR_X, x);
    e.mem.set_f32(im + IM_CURSOR_Y, y);
    let tile = e.mem.u32(im + IM_CURSOR);
    let node = if tile == 0 {
        0
    } else {
        e.mem.u32(tile + TILE_NODE)
    };
    if node == 0 {
        return handled(true);
    }
    let ratio = e.mem.f32(SCREEN_RATIO);
    let (width, height) = half_screen(e);
    e.mem.set_f32(node + LOCAL_X, x * ratio - width);
    e.mem.set_f32(node + LOCAL_Z, height - y * ratio);
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_game.h: DeferredInit
// (`g_screenWidth`, `g_screenHeight`).
/// JIP's half screen width and height: `iSize W` / `iSize H` of `[Display]` times half the cached
/// screen ratio (read at the call; see the module doc).
pub(crate) fn half_screen(e: &Engine) -> (f32, f32) {
    let half = e.mem.f32(SCREEN_RATIO) * 0.5;
    let width = e.mem.u32(SIZE_W) as i32 as f32 * half;
    let height = e.mem.u32(SIZE_H) as i32 as f32 * half;
    (width, height)
}

#[cfg(test)]
#[path = "jip_fn_ui_cursor_tests.rs"]
mod tests;
