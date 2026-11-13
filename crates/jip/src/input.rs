//! JIP LN NVSE's input state (`internal/xinput.cpp`, `internal/dinput.cpp`, the control helpers
//! of `internal/patches_cmd.h` (JIP LN NVSE)): the gamepad state JIP keeps and the changes it
//! makes to what the game reads (controls disabled, held or tapped), its reads of xNVSE's
//! keyboard and mouse hook, and the game's control bindings.
//!
//! **Where JIP's hook sits.** JIP jumps from the game's `XInputGetState` thunk (`0x9F996E`) to its
//! own function (`HookXInput`), which every game read of the pad goes through: `Controls::Poll`
//! (`00a23010`) and the interface's reads. The thunk is one `jmp [00fdf394]`, the only use of
//! that import slot, and the port registers its own forwarding of `XInputGetState` at the slot
//! (`engine-core`'s platform layer, which today answers "no controller"; the thunk itself is
//! reached through the port's resolver). So here JIP's hook is a wrap at the slot
//! ([`install`]), in front of the forwarding: every game read goes through it, as through JIP's
//! jump at the thunk, and there is no second reader of the pad. JIP calls `XInputGetStateEx` (ordinal 100, which also reports the
//! guide button); the forwarding is plain `XInputGetState`, so the guide button is never seen and
//! JIP's "guide pressed: no buttons" step never applies.
//!
//! Only while JIP answers "installed" ([`crate::ANSWERS_INSTALLED`]): until then nothing wraps the
//! slot and the game reads the pad as without the DLL.

use crate::game::OSInputGlobals;
use crate::Jip;
use engine_core::extend::{Owner, WrapSlot};
use engine_core::prelude::*;
use engine_core::ptr::Field;

/// The import slot of `XInputGetState` (`00fdf394`), which the game's thunk `009f996e` jumps
/// through (`jmp [00fdf394]`, its only use): JIP's hook site, as the port registers the
/// platform's function there.
pub const XINPUT_SLOT: WrapSlot = WrapSlot::new(Owner::Jip, 0x00fd_f394);
/// The game's pad state (`XINPUT_STATE` at `0x011F35A8`, 16 bytes: the packet number, the
/// buttons, the two triggers, the four stick axes): `Controls::Poll` reads it through the thunk.
pub const GAME_PAD_STATE: u32 = 0x011f_35a8;
/// The byte JIP adds to the "controller ready" test (`ds:0x11D8C50`; the exe's interface reads
/// it beside `bHas360Controller`: engine-bs `tile.rs`): ready only while it is 0.
const CONTROLLER_OFF: u32 = 0x011d_8c50;
/// `ERROR_DEVICE_NOT_CONNECTED`: JIP's last answer before the first read.
const NOT_CONNECTED: u32 = 0x48f;

/// `g_inputGlobals` (JIP LN NVSE, `OSInputGlobals::GetSingleton`): the game's `Controls` object
/// is at `*0x011F35CC`; its bindings through [`OSInputGlobals`]'s layout.
const INPUT_GLOBALS: u32 = 0x011f_35cc;
/// `MAX_CONTROL_BIND`: the game's 28 actions.
pub const MAX_CONTROL_BIND: u32 = 28;
/// `kMaxMacros` (`internal/dinput.h`): 256 keys, 8 mouse buttons, the wheel's 2.
const MAX_MACROS: u32 = 266;
/// `kMacro_MouseButtonOffset`.
const MOUSE_BUTTON_OFFSET: u32 = 0x100;

/// `kXboxCtrl_*` (`nvse/GameOSDepend.h` (JIP LN NVSE)): a controller binding's value.
mod ctrl {
    pub const DPAD_UP: u8 = 1;
    pub const DPAD_DOWN: u8 = 2;
    pub const DPAD_RIGHT: u8 = 4;
    pub const DPAD_LEFT: u8 = 5;
    pub const START: u8 = 6;
    pub const BACK: u8 = 7;
    pub const LS_BUTTON: u8 = 8;
    pub const RS_BUTTON: u8 = 9;
    pub const BUTTON_A: u8 = 10;
    pub const BUTTON_B: u8 = 11;
    pub const BUTTON_X: u8 = 12;
    pub const BUTTON_Y: u8 = 13;
    pub const RB: u8 = 14;
    pub const LB: u8 = 15;
    pub const LT: u8 = 16;
    pub const RT: u8 = 17;
    pub const LS_UP: u8 = 0x13;
    pub const LS_DOWN: u8 = 0x14;
    pub const LS_RIGHT: u8 = 0x16;
    pub const LS_LEFT: u8 = 0x17;
}

/// `kXboxMask_*`: the button bits of `wButtons`.
pub mod mask {
    pub const DPAD_UP: u16 = 1;
    pub const DPAD_DOWN: u16 = 2;
    pub const DPAD_LEFT: u16 = 4;
    pub const DPAD_RIGHT: u16 = 8;
    pub const START: u16 = 0x10;
    pub const BACK: u16 = 0x20;
    pub const LS_BUTTON: u16 = 0x40;
    pub const RS_BUTTON: u16 = 0x80;
    pub const LB: u16 = 0x100;
    pub const RB: u16 = 0x200;
    pub const GUIDE: u16 = 0x400;
    pub const BUTTON_A: u16 = 0x1000;
    pub const BUTTON_B: u16 = 0x2000;
    pub const BUTTON_X: u16 = 0x4000;
    pub const BUTTON_Y: u16 = 0x8000;
}

/// A pad state (`XINPUT_GAMEPAD_EX` (JIP LN NVSE), the 16 bytes of `XINPUT_STATE`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Pad {
    pub packet: u32,
    pub buttons: u16,
    pub left_trigger: u8,
    pub right_trigger: u8,
    pub thumb_lx: i16,
    pub thumb_ly: i16,
    pub thumb_rx: i16,
    pub thumb_ry: i16,
}

impl Pad {
    fn read(e: &Engine, at: u32) -> Pad {
        Pad {
            packet: e.mem.u32(at),
            buttons: e.mem.u16(at + 4),
            left_trigger: e.mem.u8(at + 6),
            right_trigger: e.mem.u8(at + 7),
            thumb_lx: e.mem.i16(at + 8),
            thumb_ly: e.mem.i16(at + 0xa),
            thumb_rx: e.mem.i16(at + 0xc),
            thumb_ry: e.mem.i16(at + 0xe),
        }
    }
}

/// `XInputStateMods`: the changes JIP makes to the game's pad state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Mods {
    /// The buttons that pass (a disabled one's bit clear; the guide's clear from the start).
    pub button_skip: u16,
    /// The buttons held down.
    pub button_hold: u16,
    /// The buttons tapped once, until the next read.
    pub button_tap: u16,
    /// Bits 1, 2: a trigger disabled (left, right); 4, 8: held; 0x10, 0x20: tapped.
    pub trigger_mods: u8,
    /// Bits 1, 2, 4, 8: the left stick's up, down, right, left disabled; 0xF0: the right stick.
    pub stick_skip: u8,
}

impl Default for Mods {
    fn default() -> Self {
        Mods {
            button_skip: 0xfbff,
            button_hold: 0,
            button_tap: 0,
            trigger_mods: 0,
            stick_skip: 0,
        }
    }
}

/// JIP's input state on the engine.
#[derive(Clone, Debug)]
pub struct State {
    /// `s_controllerReady`.
    pub ready: bool,
    /// `s_gamePad`: the last state read from the pad, before JIP's changes.
    pub pad: Pad,
    /// `s_XIStateMods`.
    pub mods: Mods,
    /// The hook's last answer (its `static result`).
    pub last_result: u32,
    /// `s_deadZoneLS`: the left stick's dead zone for a control's state.
    pub dead_zone_ls: i32,
}

impl Default for State {
    fn default() -> Self {
        State {
            ready: false,
            pad: Pad::default(),
            mods: Mods::default(),
            last_result: NOT_CONNECTED,
            dead_zone_ls: 7849,
        }
    }
}

fn state(e: &mut Engine) -> &mut State {
    &mut e.ext::<Jip>().input
}

/// Puts JIP's hook in front of the thunk, when JIP answers "installed" (`HookXInput`, at its
/// start).
pub fn install(e: &mut Engine, answers_installed: bool) {
    if answers_installed {
        e.wrap(XINPUT_SLOT, hook_xinput_get_state);
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/xinput.cpp: Hook_XInputGetState.
/// The game's own read (into `0x011F35A8`): the pad read through the port's forwarding, JIP
/// "ready" when it answered 0 and the byte at `0x011D8C50` is clear; then JIP keeps the state
/// and changes the game's copy: the buttons (none while the guide is down) with the held ones
/// added, the disabled ones taken out and the tapped ones added (the taps then cleared); a
/// disabled trigger released unless held or tapped, a held or tapped one fully down (the taps
/// cleared); a disabled direction of the left stick centred on its axis, the right stick centred
/// when any of its directions is disabled. Any other read gets a copy of the game's state and the
/// last answer.
fn hook_xinput_get_state(e: &mut Engine, a: &[u32]) -> Ret {
    let out = a.get(1).copied().unwrap_or(0);
    if out != GAME_PAD_STATE {
        let bytes = e.mem.bytes(GAME_PAD_STATE, 16);
        e.mem.write(out, &bytes);
        let result = state(e).last_result;
        return Ret {
            eax: result,
            ..Ret::default()
        };
    }
    let result = e.call_next(XINPUT_SLOT.addr(), &args![0u32, out]).u32();
    state(e).last_result = result;
    let ready = (result | u32::from(e.mem.u8(CONTROLLER_OFF))) == 0;
    state(e).ready = ready;
    if ready {
        let pad = Pad::read(e, out);
        state(e).pad = pad;
        apply_mods(e, out);
    }
    Ret {
        eax: result,
        ..Ret::default()
    }
}

/// The mods' part of the hook, on the state at `at`.
fn apply_mods(e: &mut Engine, at: u32) {
    let mut mods = state(e).mods;
    let mut buttons = e.mem.u16(at + 4);
    if buttons & mask::GUIDE != 0 {
        buttons = 0;
    }
    buttons = ((buttons | mods.button_hold) & mods.button_skip) | mods.button_tap;
    e.mem.set_u16(at + 4, buttons);
    mods.button_tap = 0;
    let t = mods.trigger_mods;
    if t != 0 {
        let (mut left, mut right) = (e.mem.u8(at + 6), e.mem.u8(at + 7));
        if t & 0x11 == 1 {
            left = 0;
        } else if t & 0x14 != 0 {
            left = 0xff;
        }
        if t & 0x22 == 2 {
            right = 0;
        } else if t & 0x28 != 0 {
            right = 0xff;
        }
        e.mem.set_u8(at + 6, left);
        e.mem.set_u8(at + 7, right);
        mods.trigger_mods = t & 0xf;
    }
    let s = mods.stick_skip;
    if s & 0xf != 0 {
        let ly = e.mem.i16(at + 0xa);
        if (ly < 0 && s & 2 != 0) || (ly > 0 && s & 1 != 0) {
            e.mem.set_i16(at + 0xa, 0);
        }
        let lx = e.mem.i16(at + 8);
        if (lx < 0 && s & 8 != 0) || (lx > 0 && s & 4 != 0) {
            e.mem.set_i16(at + 8, 0);
        }
    }
    if s & 0xf0 != 0 {
        e.mem.set_u32(at + 0xc, 0);
    }
    state(e).mods = mods;
}

// ---- the game's bindings ----

/// The button a device binds to `control`, 0xFF for none (`KEYBOARD_BIND`, `MOUSE_BIND`,
/// `CONTROLLER_BIND`): byte `control` of the device's array in `OSInputGlobals`.
fn bind(e: &Engine, array: Field<OSInputGlobals, u8>, control: u32) -> u8 {
    let globals = Ptr::<OSInputGlobals>::new(e.mem.u32(INPUT_GLOBALS));
    e.mem.u8(globals.addr() + array.off + control)
}

fn keyboard_bind(e: &Engine, control: u32) -> u8 {
    bind(e, OSInputGlobals::keyBinds, control)
}

fn mouse_bind(e: &Engine, control: u32) -> u8 {
    bind(e, OSInputGlobals::mouseBinds, control)
}

fn controller_bind(e: &Engine, control: u32) -> u8 {
    bind(e, OSInputGlobals::controllerBinds, control)
}

// ---- xNVSE's keyboard and mouse hook, as JIP reads it ----

fn keys(e: &mut Engine) -> &mut [nvse::input::KeyInfo] {
    &mut e.ext::<nvse::Nvse>().input.keys[..]
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/dinput.cpp: DIHookControl::IsKeyPressed.
/// A key down in the sources `flags & 7` names (0 and 1: the game's state, 2: the player's raw
/// one, 4: a script's; their sums), disabled or not: JIP reads xNVSE's key record itself.
pub fn is_key_pressed(e: &mut Engine, key: u32, flags: u32) -> bool {
    if key >= MAX_MACROS {
        return false;
    }
    let k = keys(e)[key as usize];
    let sources = flags & 7;
    let game = sources & 1 != 0 || sources == 0;
    (game && k.game_state)
        || (sources & 2 != 0 && k.raw_state)
        || (sources & 4 != 0 && k.inserted_state)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/dinput.cpp: DIHookControl::IsKeyPressedRaw.
pub fn is_key_pressed_raw(e: &mut Engine, key: u32) -> bool {
    key < MAX_MACROS && keys(e)[key as usize].raw_state
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/dinput.cpp: DIHookControl::IsKeyDisabled.
pub fn is_key_disabled(e: &mut Engine, key: u32) -> bool {
    key < MAX_MACROS && {
        let k = keys(e)[key as usize];
        k.user_disable || k.script_disable
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/dinput.cpp:
// DIHookControl::SetKeyDisableState.
/// Disables or enables a key for the player and scripts at once.
pub fn set_key_disable_state(e: &mut Engine, key: u32, disable: bool) {
    if key < MAX_MACROS {
        let k = &mut keys(e)[key as usize];
        k.user_disable = disable;
        k.script_disable = disable;
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/dinput.cpp: DIHookControl::TapKey.
pub fn tap_key(e: &mut Engine, key: u32) {
    if key < MAX_MACROS {
        keys(e)[key as usize].tap = true;
    }
}

// ---- the pad's controls ----

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/xinput.cpp: GetXIControlPressed.
/// Whether the pad state shows the controller binding of `control` down: its button, a trigger
/// not at rest, the left stick past its dead zone that way.
pub fn xi_control_pressed(e: &Engine, pad: &Pad, control: u32, dead_zone: i32) -> bool {
    let b = |m: u16| pad.buttons & m != 0;
    match controller_bind(e, control) {
        ctrl::DPAD_UP => b(mask::DPAD_UP),
        ctrl::DPAD_DOWN => b(mask::DPAD_DOWN),
        ctrl::DPAD_RIGHT => b(mask::DPAD_RIGHT),
        ctrl::DPAD_LEFT => b(mask::DPAD_LEFT),
        ctrl::START => b(mask::START),
        ctrl::BACK => b(mask::BACK),
        ctrl::LS_BUTTON => b(mask::LS_BUTTON),
        ctrl::RS_BUTTON => b(mask::RS_BUTTON),
        ctrl::BUTTON_A => b(mask::BUTTON_A),
        ctrl::BUTTON_B => b(mask::BUTTON_B),
        ctrl::BUTTON_X => b(mask::BUTTON_X),
        ctrl::BUTTON_Y => b(mask::BUTTON_Y),
        ctrl::RB => b(mask::RB),
        ctrl::LB => b(mask::LB),
        ctrl::LT => pad.left_trigger != 0,
        ctrl::RT => pad.right_trigger != 0,
        ctrl::LS_UP => i32::from(pad.thumb_ly) > dead_zone,
        ctrl::LS_DOWN => i32::from(pad.thumb_ly) < -dead_zone,
        ctrl::LS_RIGHT => i32::from(pad.thumb_lx) > dead_zone,
        ctrl::LS_LEFT => i32::from(pad.thumb_lx) < -dead_zone,
        _ => false,
    }
}

/// The button mask of a controller binding, for the buttons' cases.
fn button_mask(binding: u8) -> Option<u16> {
    Some(match binding {
        ctrl::DPAD_UP => mask::DPAD_UP,
        ctrl::DPAD_DOWN => mask::DPAD_DOWN,
        ctrl::DPAD_RIGHT => mask::DPAD_RIGHT,
        ctrl::DPAD_LEFT => mask::DPAD_LEFT,
        ctrl::START => mask::START,
        ctrl::BACK => mask::BACK,
        ctrl::LS_BUTTON => mask::LS_BUTTON,
        ctrl::RS_BUTTON => mask::RS_BUTTON,
        ctrl::BUTTON_A => mask::BUTTON_A,
        ctrl::BUTTON_B => mask::BUTTON_B,
        ctrl::BUTTON_X => mask::BUTTON_X,
        ctrl::BUTTON_Y => mask::BUTTON_Y,
        ctrl::RB => mask::RB,
        ctrl::LB => mask::LB,
        _ => return None,
    })
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/xinput.cpp: GetXIControlDisabled.
/// JIP's answer for a controller binding: a button's bit in the mask of buttons that pass (JIP's
/// test, as written: set for a button that is not disabled), a trigger's or a left-stick
/// direction's "disabled" bit.
pub fn xi_control_disabled(e: &mut Engine, control: u32) -> bool {
    let binding = controller_bind(e, control);
    let mods = state(e).mods;
    if let Some(m) = button_mask(binding) {
        return mods.button_skip & m != 0;
    }
    match binding {
        ctrl::LT => mods.trigger_mods & 1 != 0,
        ctrl::RT => mods.trigger_mods & 2 != 0,
        ctrl::LS_UP => mods.stick_skip & 1 != 0,
        ctrl::LS_DOWN => mods.stick_skip & 2 != 0,
        ctrl::LS_RIGHT => mods.stick_skip & 4 != 0,
        ctrl::LS_LEFT => mods.stick_skip & 8 != 0,
        _ => false,
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/xinput.cpp: SetXIControlDisabled.
/// A controller binding disabled or enabled: a button taken out of (or put back in) the mask of
/// buttons that pass, a trigger's or a left-stick direction's "disabled" bit set or cleared.
pub fn set_xi_control_disabled(e: &mut Engine, control: u32, disable: bool) {
    let binding = controller_bind(e, control);
    let mods = &mut state(e).mods;
    if let Some(m) = button_mask(binding) {
        mods.button_skip |= m;
        if disable {
            mods.button_skip ^= m;
        }
        return;
    }
    let (field, bit) = match binding {
        ctrl::LT => (&mut mods.trigger_mods, 1),
        ctrl::RT => (&mut mods.trigger_mods, 2),
        ctrl::LS_UP => (&mut mods.stick_skip, 1),
        ctrl::LS_DOWN => (&mut mods.stick_skip, 2),
        ctrl::LS_RIGHT => (&mut mods.stick_skip, 4),
        ctrl::LS_LEFT => (&mut mods.stick_skip, 8),
        _ => return,
    };
    *field &= !bit;
    if disable {
        *field |= bit;
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/xinput.cpp: TapXIControl.
/// A controller binding tapped at the next read: a button or a trigger; not a stick direction.
pub fn tap_xi_control(e: &mut Engine, control: u32) -> bool {
    let binding = controller_bind(e, control);
    let mods = &mut state(e).mods;
    if let Some(m) = button_mask(binding) {
        mods.button_tap |= m;
        return true;
    }
    match binding {
        ctrl::LT => mods.trigger_mods |= 0x10,
        ctrl::RT => mods.trigger_mods |= 0x20,
        _ => return false,
    }
    true
}

// ---- a control, whatever device ----

/// The game's pad state as the hook left it (`0x011F35A8`).
fn game_pad(e: &Engine) -> Pad {
    Pad::read(e, GAME_PAD_STATE)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_cmd.h: IsControlPressed.
/// Without a ready controller, the control's keyboard key or its mouse button down (`flags` as
/// [`is_key_pressed`]'s); with one, its controller binding down in the pad's own state (`flags`
/// bit 2) or in the game's (after JIP's changes).
pub fn is_control_pressed(e: &mut Engine, control: u32, flags: u32) -> bool {
    if !state(e).ready {
        let key = keyboard_bind(e, control);
        if key != 0xff && is_key_pressed(e, u32::from(key), flags) {
            return true;
        }
        let button = mouse_bind(e, control);
        return button != 0xff && is_key_pressed(e, u32::from(button) + MOUSE_BUTTON_OFFSET, flags);
    }
    let pad = if flags & 2 != 0 {
        state(e).pad
    } else {
        game_pad(e)
    };
    let dead_zone = state(e).dead_zone_ls;
    xi_control_pressed(e, &pad, control, dead_zone)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_cmd.h: IsControlPressedRaw.
/// As [`is_control_pressed`] with the player's raw key states and the pad's own state.
pub fn is_control_pressed_raw(e: &mut Engine, control: u32) -> bool {
    if !state(e).ready {
        let key = keyboard_bind(e, control);
        if key != 0xff && is_key_pressed_raw(e, u32::from(key)) {
            return true;
        }
        let button = mouse_bind(e, control);
        return button != 0xff && is_key_pressed_raw(e, u32::from(button) + MOUSE_BUTTON_OFFSET);
    }
    let pad = state(e).pad;
    let dead_zone = state(e).dead_zone_ls;
    xi_control_pressed(e, &pad, control, dead_zone)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_cmd.h: SetControlDisabled.
/// Without a ready controller, the control's keyboard key and mouse button disabled or enabled;
/// with one, its controller binding.
pub fn set_control_disabled(e: &mut Engine, control: u32, disable: bool) {
    if !state(e).ready {
        let key = keyboard_bind(e, control);
        if key != 0xff {
            set_key_disable_state(e, u32::from(key), disable);
        }
        let button = mouse_bind(e, control);
        if button != 0xff {
            set_key_disable_state(e, u32::from(button) + MOUSE_BUTTON_OFFSET, disable);
        }
    } else {
        set_xi_control_disabled(e, control, disable);
    }
}

/// Whether the controller is ready (`s_controllerReady`).
pub fn controller_ready(e: &mut Engine) -> bool {
    state(e).ready
}

/// The keyboard and mouse binding of a control (`KEYBOARD_BIND`, `MOUSE_BIND`), for the commands.
pub fn binds(e: &Engine, control: u32) -> (u8, u8) {
    (keyboard_bind(e, control), mouse_bind(e, control))
}

#[cfg(test)]
#[path = "input_tests.rs"]
mod tests;
