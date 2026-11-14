//! `functions_ln/ln_fn_gamepad.h` (JIP LN NVSE): the XInput pad as JIP sees it and the changes
//! scripts make to what the game reads. 27 of its 31 commands (the core track built
//! `SetOnTriggerDownEventHandler` and `SetOnTriggerUpEventHandler`, `ui_hooks.rs`), over its pad state
//! (`crate::input`: `s_controllerReady`, `s_gamePad` (the pad's own state before JIP's changes),
//! `s_XIStateMods`, the dead zones).
//!
//! Left for later: `SetDeadZoneLS` and `SetDeadZoneRS` write the new dead zone into the game's own
//! code (`SafeWrite32` at thirteen and eighteen immediates in the game's code,
//! `0x717B90` ... `0xA23E9C`): the translated functions hold those numbers as constants, so each
//! site needs a seam first. Until then the dead zones keep JIP's first values (7849 and 8689),
//! which the getters here read.

use super::Handlers;
use crate::input::{Mods, Pad};
use crate::Jip;
use engine_core::prelude::*;
use nvse::commands::Read;
use nvse::{handled, CommandArgs, EvalArgs};

#[rustfmt::skip]
pub static COMMANDS: Handlers = &[
    (0x2203, get_controller),
    (0x220b, get_dead_zone_ls),
    (0x220c, get_dead_zone_rs),
    (0x220f, get_pressed_buttons),
    (0x2294, enable_button),
    (0x2295, disable_button),
    (0x2297, hold_button),
    (0x2298, release_button),
    (0x22a4, enable_trigger),
    (0x22a5, disable_trigger),
    (0x22a7, hold_trigger),
    (0x22a8, release_trigger),
    (0x27fd, tap_button),
    (0x27fe, tap_trigger),
    (0x289e, set_stick_disabled),
];

#[rustfmt::skip]
pub static CONDITIONS: super::Conditions = &[
    (0x2204, is_button_pressed, is_button_pressed_eval),
    (0x2205, get_left_stick_x, get_left_stick_x_eval),
    (0x2206, get_left_stick_y, get_left_stick_y_eval),
    (0x2207, get_right_stick_x, get_right_stick_x_eval),
    (0x2208, get_right_stick_y, get_right_stick_y_eval),
    (0x2209, get_left_trigger, get_left_trigger_eval),
    (0x220a, get_right_trigger, get_right_trigger_eval),
    (0x2296, is_button_disabled, is_button_disabled_eval),
    (0x2299, is_button_held, is_button_held_eval),
    (0x22a6, is_trigger_disabled, is_trigger_disabled_eval),
    (0x22a9, is_trigger_held, is_trigger_held_eval),
    (0x289d, is_stick_disabled, is_stick_disabled_eval),
];

fn ready(e: &mut Engine) -> bool {
    e.ext::<Jip>().input.ready
}

fn pad(e: &mut Engine) -> Pad {
    e.ext::<Jip>().input.pad
}

fn mods(e: &mut Engine) -> &mut Mods {
    &mut e.ext::<Jip>().input.mods
}

/// The one integer argument, when read.
fn int_arg(e: &mut Engine, a: &CommandArgs) -> Option<u32> {
    let read = Read::of(e, a);
    read.ok.then(|| read.int(0, 0))
}

fn set_if(e: &mut Engine, result: u32, on: bool) {
    if on {
        e.mem.set_f64(result, 1.0);
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_gamepad.h:
// Cmd_GetController_Execute.
/// 1 while JIP reads a ready controller, echoed in the console.
fn get_controller(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let on = ready(e);
    a.set_result(e, if on { 1.0 } else { 0.0 });
    crate::console::print_result(e, &a);
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_gamepad.h:
// Cmd_IsButtonPressed_Execute, Cmd_IsButtonPressed_Eval.
/// 1 when one of the buttons (the low 16 bits) is down on the pad (its own state).
fn is_button_pressed(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    if let Some(button) = int_arg(e, &a) {
        let on = pad(e).buttons & button as u16 != 0;
        set_if(e, a.result, on);
    }
    handled(true)
}

fn is_button_pressed_eval(e: &mut Engine, w: &[u32]) -> Ret {
    let a = EvalArgs::new(w);
    let on = pad(e).buttons & a.arg1 as u16 != 0;
    set_if(e, a.result, on);
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_gamepad.h:
// Cmd_GetPressedButtons_Execute.
fn get_pressed_buttons(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let buttons = pad(e).buttons;
    a.set_result(e, f64::from(buttons));
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_gamepad.h: GetLeftStick,
// GetRightStick (asm).
/// A stick axis past its dead zone, scaled: 0 inside it; above, `(value - dz) * d`; below,
/// `(value + 1 + dz) * d` (`d = 1 / (32767 - dz)`, JIP's `s_deadZoneLSd`/`RSd`, kept with the
/// dead zone: `1.0 / 24918` and `1.0 / 24078` at the start); the products in doubles
/// (`cvtsi2sd`, `mulsd`).
pub fn stick(value: i16, dead_zone: i32) -> f64 {
    let v = i32::from(value);
    let scale = 1.0 / f64::from(32767 - dead_zone);
    if v == 0 {
        return 0.0;
    }
    let past = if v > 0 {
        v.wrapping_sub(dead_zone)
    } else {
        v.wrapping_add(1).wrapping_add(dead_zone)
    };
    if (v > 0 && past <= 0) || (v < 0 && past >= 0) {
        return 0.0;
    }
    f64::from(past) * scale
}

fn left(e: &mut Engine, axis: fn(&Pad) -> i16) -> f64 {
    let p = pad(e);
    let dz = e.ext::<Jip>().input.dead_zone_ls;
    stick(axis(&p), dz)
}

fn right(e: &mut Engine, axis: fn(&Pad) -> i16) -> f64 {
    let p = pad(e);
    let dz = e.ext::<Jip>().input.dead_zone_rs;
    stick(axis(&p), dz)
}

/// A getter's execute and eval both write the value.
fn put(e: &mut Engine, result: u32, value: f64) -> Ret {
    e.mem.set_f64(result, value);
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_gamepad.h:
// Cmd_GetLeftStickX_Execute/_Eval and the three others.
fn get_left_stick_x(e: &mut Engine, w: &[u32]) -> Ret {
    let v = left(e, |p| p.thumb_lx);
    put(e, CommandArgs::new(w).result, v)
}
fn get_left_stick_x_eval(e: &mut Engine, w: &[u32]) -> Ret {
    let v = left(e, |p| p.thumb_lx);
    put(e, EvalArgs::new(w).result, v)
}
fn get_left_stick_y(e: &mut Engine, w: &[u32]) -> Ret {
    let v = left(e, |p| p.thumb_ly);
    put(e, CommandArgs::new(w).result, v)
}
fn get_left_stick_y_eval(e: &mut Engine, w: &[u32]) -> Ret {
    let v = left(e, |p| p.thumb_ly);
    put(e, EvalArgs::new(w).result, v)
}
fn get_right_stick_x(e: &mut Engine, w: &[u32]) -> Ret {
    let v = right(e, |p| p.thumb_rx);
    put(e, CommandArgs::new(w).result, v)
}
fn get_right_stick_x_eval(e: &mut Engine, w: &[u32]) -> Ret {
    let v = right(e, |p| p.thumb_rx);
    put(e, EvalArgs::new(w).result, v)
}
fn get_right_stick_y(e: &mut Engine, w: &[u32]) -> Ret {
    let v = right(e, |p| p.thumb_ry);
    put(e, CommandArgs::new(w).result, v)
}
fn get_right_stick_y_eval(e: &mut Engine, w: &[u32]) -> Ret {
    let v = right(e, |p| p.thumb_ry);
    put(e, EvalArgs::new(w).result, v)
}

/// A trigger's travel: `value * (1 / 255.0)` in doubles.
pub fn trigger(value: u8) -> f64 {
    f64::from(value) * (1.0 / 255.0)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_gamepad.h:
// Cmd_GetLeftTrigger_Execute/_Eval, Cmd_GetRightTrigger_Execute/_Eval.
fn get_left_trigger(e: &mut Engine, w: &[u32]) -> Ret {
    let v = trigger(pad(e).left_trigger);
    put(e, CommandArgs::new(w).result, v)
}
fn get_left_trigger_eval(e: &mut Engine, w: &[u32]) -> Ret {
    let v = trigger(pad(e).left_trigger);
    put(e, EvalArgs::new(w).result, v)
}
fn get_right_trigger(e: &mut Engine, w: &[u32]) -> Ret {
    let v = trigger(pad(e).right_trigger);
    put(e, CommandArgs::new(w).result, v)
}
fn get_right_trigger_eval(e: &mut Engine, w: &[u32]) -> Ret {
    let v = trigger(pad(e).right_trigger);
    put(e, EvalArgs::new(w).result, v)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_gamepad.h:
// Cmd_GetDeadZoneLS_Execute, Cmd_GetDeadZoneRS_Execute.
/// The dead zone (`s_deadZoneLSg`, a double equal to the integer one) times `1 / 32767.0`,
/// echoed in the console.
fn get_dead_zone_ls(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let dz = e.ext::<Jip>().input.dead_zone_ls;
    a.set_result(e, f64::from(dz) * (1.0 / 32767.0));
    crate::console::print_result(e, &a);
    handled(true)
}

fn get_dead_zone_rs(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let dz = e.ext::<Jip>().input.dead_zone_rs;
    a.set_result(e, f64::from(dz) * (1.0 / 32767.0));
    crate::console::print_result(e, &a);
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_gamepad.h:
// Cmd_EnableButton_Execute .. Cmd_TapButton_Execute.
/// The buttons (stored as 16 bits) back in the mask of buttons that pass.
fn enable_button(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    if let Some(b) = int_arg(e, &a) {
        mods(e).button_skip |= b as u16;
    }
    handled(true)
}

fn disable_button(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    if let Some(b) = int_arg(e, &a) {
        mods(e).button_skip &= !(b as u16);
    }
    handled(true)
}

/// 1 when none of the buttons passes (the whole 32-bit argument against the 16-bit mask; the
/// eval takes the low 16 bits, which is the same).
fn is_button_disabled(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    if let Some(b) = int_arg(e, &a) {
        let on = u32::from(mods(e).button_skip) & b == 0;
        set_if(e, a.result, on);
    }
    handled(true)
}

fn is_button_disabled_eval(e: &mut Engine, w: &[u32]) -> Ret {
    let a = EvalArgs::new(w);
    let on = mods(e).button_skip & a.arg1 as u16 == 0;
    set_if(e, a.result, on);
    handled(true)
}

fn hold_button(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    if let Some(b) = int_arg(e, &a) {
        mods(e).button_hold |= b as u16;
    }
    handled(true)
}

fn release_button(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    if let Some(b) = int_arg(e, &a) {
        mods(e).button_hold &= !(b as u16);
    }
    handled(true)
}

fn is_button_held(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    if let Some(b) = int_arg(e, &a) {
        let on = u32::from(mods(e).button_hold) & b != 0;
        set_if(e, a.result, on);
    }
    handled(true)
}

fn is_button_held_eval(e: &mut Engine, w: &[u32]) -> Ret {
    let a = EvalArgs::new(w);
    let on = mods(e).button_hold & a.arg1 as u16 != 0;
    set_if(e, a.result, on);
    handled(true)
}

fn tap_button(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    if let Some(b) = int_arg(e, &a) {
        mods(e).button_tap |= b as u16;
    }
    handled(true)
}

/// A trigger's bit: `right` (any non-zero value) the second of the pair.
fn trigger_bit(right: bool, left_bit: u8) -> u8 {
    if right {
        left_bit << 1
    } else {
        left_bit
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_gamepad.h:
// Cmd_EnableTrigger_Execute .. Cmd_TapTrigger_Execute.
/// Bits 1/2 disabled, 4/8 held, 0x10/0x20 tapped (left/right).
fn enable_trigger(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    if let Some(t) = int_arg(e, &a) {
        mods(e).trigger_mods &= if t != 0 { 0xfd } else { 0xfe };
    }
    handled(true)
}

fn disable_trigger(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    if let Some(t) = int_arg(e, &a) {
        mods(e).trigger_mods |= trigger_bit(t != 0, 1);
    }
    handled(true)
}

fn is_trigger_disabled(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    if let Some(t) = int_arg(e, &a) {
        let on = mods(e).trigger_mods & trigger_bit(t != 0, 1) != 0;
        set_if(e, a.result, on);
    }
    handled(true)
}

fn is_trigger_disabled_eval(e: &mut Engine, w: &[u32]) -> Ret {
    let a = EvalArgs::new(w);
    let on = mods(e).trigger_mods & trigger_bit(a.arg1 != 0, 1) != 0;
    set_if(e, a.result, on);
    handled(true)
}

fn hold_trigger(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    if let Some(t) = int_arg(e, &a) {
        mods(e).trigger_mods |= trigger_bit(t != 0, 4);
    }
    handled(true)
}

fn release_trigger(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    if let Some(t) = int_arg(e, &a) {
        mods(e).trigger_mods &= if t != 0 { 0xf7 } else { 0xfb };
    }
    handled(true)
}

fn is_trigger_held(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    if let Some(t) = int_arg(e, &a) {
        let on = mods(e).trigger_mods & trigger_bit(t != 0, 4) != 0;
        set_if(e, a.result, on);
    }
    handled(true)
}

fn is_trigger_held_eval(e: &mut Engine, w: &[u32]) -> Ret {
    let a = EvalArgs::new(w);
    let on = mods(e).trigger_mods & trigger_bit(a.arg1 != 0, 4) != 0;
    set_if(e, a.result, on);
    handled(true)
}

fn tap_trigger(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    if let Some(t) = int_arg(e, &a) {
        mods(e).trigger_mods |= trigger_bit(t != 0, 0x10);
    }
    handled(true)
}

/// A stick's bits: 0x0F the left one, 0xF0 the right (any non-zero value).
fn stick_bits(right: bool) -> u8 {
    if right {
        0xf0
    } else {
        0x0f
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_gamepad.h:
// Cmd_IsStickDisabled_Execute/_Eval, Cmd_SetStickDisabled_Execute.
/// 1 when any direction of the stick is disabled.
fn is_stick_disabled(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    if let Some(s) = int_arg(e, &a) {
        let on = mods(e).stick_skip & stick_bits(s != 0) != 0;
        set_if(e, a.result, on);
    }
    handled(true)
}

fn is_stick_disabled_eval(e: &mut Engine, w: &[u32]) -> Ret {
    let a = EvalArgs::new(w);
    let on = mods(e).stick_skip & stick_bits(a.arg1 != 0) != 0;
    set_if(e, a.result, on);
    handled(true)
}

/// Every direction of the stick disabled, or every one enabled.
fn set_stick_disabled(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if read.ok {
        let bits = stick_bits(read.int(0, 0) != 0);
        if read.int(1, 0) != 0 {
            mods(e).stick_skip |= bits;
        } else {
            mods(e).stick_skip &= !bits;
        }
    }
    handled(true)
}

#[cfg(test)]
#[path = "ln_fn_gamepad_tests.rs"]
mod tests;
