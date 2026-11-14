//! `functions_jip/jip_fn_input.h` (JIP LN NVSE): controls held by a script and the mouse wheel.
//! 3 of its 9 commands; the core track built the four `SetOn...EventHandler` commands
//! (`ui_hooks.rs`).
//!
//! Left for later: `ToggleVanityWheel` writes a byte of the game's code (`SafeWrite8(0x945A29,
//! 0x8B or 0x89)`, inside `009445B0`), which the translated code has no seam for;
//! `ToggleMouseMovement` sets `s_mouseMovementState`, which only JIP's `GetMouseMovementHook`
//! reads (`internal/hooks.h`: its jump replaces the game's mouse axis read `00A239E0`): the hook
//! is the core track's, and the command comes with it.

use super::Handlers;
use crate::input::MAX_CONTROL_BIND;
use engine_core::prelude::*;
use nvse::commands::Read;
use nvse::{handled, CommandArgs};

#[rustfmt::skip]
pub static COMMANDS: Handlers = &[
    (0x2805, hold_control),
    (0x2806, release_control),
    (0x292c, scroll_mouse_wheel),
];

/// `g_inputGlobals` (`*0x011F35CC`, the game's `Controls`) and its mouse wheel word
/// (`currMouseWheelScroll`, +0x1B2C: the exe's mouse axis read `00A239E0` answers it for axis 3).
const INPUT_GLOBALS: u32 = 0x011f_35cc;
const MOUSE_WHEEL: u32 = 0x1b2c;
/// `kMacro_MouseButtonOffset`.
const MOUSE_BUTTON_OFFSET: u32 = 0x100;

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_input.h: SetCtrlHeldState.
/// Without a ready controller, the control's keyboard key and mouse button held (or let go) in
/// xNVSE's key states (JIP's `DIHookControl::SetKeyHeldState`); with one, its controller binding.
fn set_ctrl_held_state(e: &mut Engine, control: u32, hold: bool) {
    if !crate::input::controller_ready(e) {
        let (key, button) = crate::input::binds(e, control);
        if key != 0xff {
            crate::input::set_key_held_state(e, u32::from(key), hold);
        }
        if button != 0xff {
            crate::input::set_key_held_state(e, u32::from(button) + MOUSE_BUTTON_OFFSET, hold);
        }
    } else {
        crate::input::set_xi_control_held(e, control, hold);
    }
}

/// HoldControl and ReleaseControl: a control below 28.
fn control_held(e: &mut Engine, w: &[u32], hold: bool) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let control = read.int(0, 0);
    if read.ok && control < MAX_CONTROL_BIND {
        set_ctrl_held_state(e, control, hold);
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_input.h:
// Cmd_HoldControl_Execute.
fn hold_control(e: &mut Engine, w: &[u32]) -> Ret {
    control_held(e, w, true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_input.h:
// Cmd_ReleaseControl_Execute.
fn release_control(e: &mut Engine, w: &[u32]) -> Ret {
    control_held(e, w, false)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_input.h:
// Cmd_ScrollMouseWheel_Execute.
/// The value added to this frame's mouse wheel movement (a 32-bit sum that wraps).
///
/// JIP leaves its `value` uninitialised and the parameter is optional: with no argument it adds
/// whatever its stack slot held. The port adds 0 then (no change).
fn scroll_mouse_wheel(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if read.ok {
        let value = read.int(0, 0);
        let globals = e.mem.u32(INPUT_GLOBALS);
        if globals != 0 {
            let at = globals + MOUSE_WHEEL;
            let sum = e.mem.u32(at).wrapping_add(value);
            e.mem.set_u32(at, sum);
        }
    }
    handled(true)
}

#[cfg(test)]
#[path = "jip_fn_input_tests.rs"]
mod tests;
