//! Tests of JIP's input state: the hook at the `XInputGetState` thunk over a stand-in of the
//! port's forwarding, the control helpers over the game's bindings and xNVSE's key records, and
//! the six input commands through their records. Layouts from the exe: the pad state the game
//! reads at `0x011F35A8` (`XINPUT_STATE`, 16 bytes, `Controls::Poll` `00a23010`), the bindings at
//! `+0x1B94` of the `Controls` object (`*0x011F35CC`), 0x1C bytes a device (engine-bs
//! `xcontrols.rs`).

use super::*;
use crate::test_support::{engine, line, RESULT};
use std::cell::RefCell;

const CONTROLS_AT: u32 = 0x0500_0000;
const OTHER_STATE: u32 = 0x0500_2000;
/// The exe's addresses and offsets, written out (not the code's constants): the pad state the game
/// reads, the controller-off byte, `ERROR_DEVICE_NOT_CONNECTED`, the `XInputGetState` import slot;
/// `OSInputGlobals`' four binding arrays (`keyBinds` `+0x1B94`, `mouseBinds` `+0x1BB0`,
/// `joystickBinds` `+0x1BCC`, `controllerBinds` `+0x1BE8`: JIP's `GameOSDepend.h`; the exe's
/// `00a25410` reads `+0x1b94 + device * 0x1c`).
const PAD_STATE: u32 = 0x011f_35a8;
const OFF_BYTE: u32 = 0x011d_8c50;
const NOT_CONN: u32 = 0x48f;
const SLOT: u32 = 0x00fd_f394;
const BIND_ARRAYS: [u32; 4] = [0x1b94, 0x1bb0, 0x1bcc, 0x1be8];

thread_local! {
    static FORWARD: RefCell<(u32, [u8; 16])> = const { RefCell::new((0, [0; 16])) };
}

/// The port's forwarding stood in: answers `result` and writes `state` into the buffer given.
fn set_forward(result: u32, state: [u8; 16]) {
    FORWARD.with(|f| *f.borrow_mut() = (result, state));
}

fn forward(e: &mut Engine, a: &[u32]) -> Ret {
    let (result, state) = FORWARD.with(|f| *f.borrow());
    e.mem.write(a[1], &state);
    Ret {
        eax: result,
        ..Ret::default()
    }
}

fn pad_bytes(buttons: u16, lt: u8, rt: u8, lx: i16, ly: i16, rx: i16, ry: i16) -> [u8; 16] {
    let mut b = [0u8; 16];
    b[0..4].copy_from_slice(&7u32.to_le_bytes());
    b[4..6].copy_from_slice(&buttons.to_le_bytes());
    b[6] = lt;
    b[7] = rt;
    b[8..10].copy_from_slice(&lx.to_le_bytes());
    b[10..12].copy_from_slice(&ly.to_le_bytes());
    b[12..14].copy_from_slice(&rx.to_le_bytes());
    b[14..16].copy_from_slice(&ry.to_le_bytes());
    b
}

/// The thunk with the forwarding behind it, JIP installed as `installed` says, the bindings of
/// control 4 set: keyboard key 0x11, mouse button 1, controller button A (10); control 5: no key,
/// mouse button 2, controller LT (16); control 6: key 0x12, no mouse button, left stick up (0x13).
fn world(installed: bool) -> Engine {
    let mut e = engine();
    e.map(0x011f_3000, 0x1000);
    e.map(0x011d_8000, 0x1000);
    e.map(CONTROLS_AT, 0x1_0000);
    e.mem.set_u32(0x011f_35cc, CONTROLS_AT);
    for device in 0..4 {
        for control in 0..28 {
            e.mem
                .set_u8(CONTROLS_AT + BIND_ARRAYS[device as usize] + control, 0xff);
        }
    }
    let set = |e: &mut Engine, device: u32, control: u32, v: u8| {
        e.mem
            .set_u8(CONTROLS_AT + BIND_ARRAYS[device as usize] + control, v);
    };
    set(&mut e, 0, 4, 0x11);
    set(&mut e, 1, 4, 1);
    set(&mut e, 3, 4, ctrl::BUTTON_A);
    set(&mut e, 1, 5, 2);
    set(&mut e, 3, 5, ctrl::LT);
    set(&mut e, 0, 6, 0x12);
    set(&mut e, 3, 6, ctrl::LS_UP);
    e.register(SLOT, forward);
    crate::install_as(&mut e, installed);
    e
}

fn read_game(e: &mut Engine) -> u32 {
    e.call(SLOT, &args![0u32, PAD_STATE]).u32()
}

/// Not installed: nothing in front of the forwarding.
#[test]
fn no_hook_when_not_installed() {
    let mut e = world(false);
    set_forward(0, pad_bytes(mask::BUTTON_A, 0, 0, 0, 0, 0, 0));
    state(&mut e).mods.button_skip = 0;
    assert_eq!(read_game(&mut e), 0);
    assert_eq!(e.mem.u16(PAD_STATE + 4), mask::BUTTON_A, "untouched");
    assert!(!state(&mut e).ready);
}

/// The game's read: ready when the forwarding answers 0 and the byte at `0x011D8C50` is clear;
/// the raw state kept; the buttons held, disabled and tapped (the taps cleared after one read);
/// the guide clears every button; triggers and sticks.
#[test]
fn the_hook_changes_the_games_copy() {
    let mut e = world(true);
    let raw = pad_bytes(mask::BUTTON_A | mask::BUTTON_B, 0x40, 0x80, 100, -200, 5, 6);
    set_forward(0, raw);
    {
        let m = &mut state(&mut e).mods;
        m.button_hold = mask::BUTTON_X;
        m.button_skip = 0xfbff & !mask::BUTTON_B;
        m.button_tap = mask::START;
        m.trigger_mods = 0x01 | 0x20;
        m.stick_skip = 0x02 | 0x04 | 0x10;
    }
    assert_eq!(read_game(&mut e), 0);
    assert!(state(&mut e).ready);
    assert_eq!(state(&mut e).pad, pad_of(&raw));
    let g = |e: &Engine, at: u32| e.mem.u16(PAD_STATE + at);
    assert_eq!(
        g(&e, 4),
        mask::BUTTON_A | mask::BUTTON_X | mask::START,
        "held added, disabled out, tapped added"
    );
    assert_eq!(e.mem.u8(PAD_STATE + 6), 0, "left trigger disabled");
    assert_eq!(e.mem.u8(PAD_STATE + 7), 0xff, "right trigger tapped");
    assert_eq!(e.mem.i16(PAD_STATE + 0xa), 0, "down disabled: y centred");
    assert_eq!(e.mem.i16(PAD_STATE + 8), 0, "right disabled: x centred");
    assert_eq!(e.mem.u32(PAD_STATE + 0xc), 0, "the right stick centred");
    let m = state(&mut e).mods;
    assert_eq!(
        (m.button_tap, m.trigger_mods),
        (0, 0x01),
        "the taps cleared"
    );
    // The next read: no tap; a held trigger fully down; the guide clears every button.
    state(&mut e).mods.trigger_mods = 0x04;
    set_forward(
        0,
        pad_bytes(mask::GUIDE | mask::BUTTON_A, 0, 0, 0, 300, 0, 0),
    );
    state(&mut e).mods.stick_skip = 0x01;
    read_game(&mut e);
    assert_eq!(g(&e, 4), mask::BUTTON_X, "the guide: only the held button");
    assert_eq!(e.mem.u8(PAD_STATE + 6), 0xff, "held");
    assert_eq!(e.mem.i16(PAD_STATE + 0xa), 0, "up disabled: y centred");
    // A trigger disabled and tapped: the tap wins (fully down), the disable kept.
    state(&mut e).mods.trigger_mods = 0x33;
    set_forward(0, pad_bytes(0, 0x40, 0x40, 0, 0, 0, 0));
    read_game(&mut e);
    assert_eq!(e.mem.u8(PAD_STATE + 6), 0xff, "left disabled and tapped");
    assert_eq!(e.mem.u8(PAD_STATE + 7), 0xff, "right disabled and tapped");
    assert_eq!(state(&mut e).mods.trigger_mods, 0x03);
    // Not ready: the byte set, or the forwarding failing: no change, the state not kept.
    e.mem.set_u8(OFF_BYTE, 1);
    set_forward(0, pad_bytes(mask::BUTTON_B, 0, 0, 0, 0, 0, 0));
    assert_eq!(read_game(&mut e), 0);
    assert!(!state(&mut e).ready);
    assert_eq!(g(&e, 4), mask::BUTTON_B, "untouched");
    e.mem.set_u8(OFF_BYTE, 0);
    set_forward(NOT_CONN, pad_bytes(mask::BUTTON_Y, 0, 0, 0, 0, 0, 0));
    assert_eq!(read_game(&mut e), NOT_CONN);
    assert!(!state(&mut e).ready);
    // Another buffer: the game's copy and the last answer, the forwarding not asked (it would
    // answer 0 and button A now).
    set_forward(0, pad_bytes(mask::BUTTON_A, 0, 0, 0, 0, 0, 0));
    let answer = e.call(SLOT, &args![0u32, OTHER_STATE]).u32();
    assert_eq!(answer, NOT_CONN);
    assert_eq!(e.mem.u16(OTHER_STATE + 4), mask::BUTTON_Y);
    assert!(!state(&mut e).ready, "the game's state not touched");
    assert_eq!(e.mem.bytes(OTHER_STATE, 16), e.mem.bytes(PAD_STATE, 16));
}

/// The pad state of raw bytes.
fn pad_of(raw: &[u8; 16]) -> Pad {
    {
        let w = |i: usize| i16::from_le_bytes([raw[i], raw[i + 1]]);
        Pad {
            packet: u32::from_le_bytes([raw[0], raw[1], raw[2], raw[3]]),
            buttons: u16::from_le_bytes([raw[4], raw[5]]),
            left_trigger: raw[6],
            right_trigger: raw[7],
            thumb_lx: w(8),
            thumb_ly: w(10),
            thumb_rx: w(12),
            thumb_ry: w(14),
        }
    }
}

fn key(e: &mut Engine, k: u32) -> &mut nvse::input::KeyInfo {
    &mut e.ext::<nvse::Nvse>().input.keys[k as usize]
}

/// JIP's reads of xNVSE's key records: the flags' sources (0 and 1 the game's state), disabled or
/// not; past the last key: false.
#[test]
fn key_reads() {
    let mut e = world(true);
    key(&mut e, 0x11).game_state = true;
    key(&mut e, 0x11).user_disable = true;
    key(&mut e, 0x12).raw_state = true;
    key(&mut e, 0x13).inserted_state = true;
    for (k, flags, want) in [
        (0x11, 0, true),
        (0x11, 1, true),
        (0x11, 2, false),
        (0x12, 1, false),
        (0x12, 2, true),
        (0x12, 3, true),
        (0x13, 3, false),
        (0x13, 4, true),
        (0x13, 0x1c, true),
        (266, 7, false),
    ] {
        assert_eq!(is_key_pressed(&mut e, k, flags), want, "{k:x} {flags}");
    }
    assert!(is_key_pressed_raw(&mut e, 0x12));
    assert!(!is_key_pressed_raw(&mut e, 0x11));
    set_key_disable_state(&mut e, 0x14, true);
    let k = *key(&mut e, 0x14);
    assert!(k.user_disable && k.script_disable, "both");
    assert!(is_key_disabled(&mut e, 0x14));
    key(&mut e, 0x15).script_disable = true;
    assert!(is_key_disabled(&mut e, 0x15), "a script's disable counts");
    tap_key(&mut e, 0x14);
    assert!(key(&mut e, 0x14).tap);
}

/// A control without a ready controller: its key or its mouse button (offset 0x100); with one:
/// its controller binding in the game's state, or (flags bit 2) the pad's own.
#[test]
fn controls_by_device() {
    let mut e = world(true);
    assert!(!is_control_pressed(&mut e, 4, 1));
    key(&mut e, 0x101).game_state = true;
    assert!(is_control_pressed(&mut e, 4, 1), "the mouse button");
    key(&mut e, 0x101).game_state = false;
    key(&mut e, 0x11).raw_state = true;
    assert!(is_control_pressed_raw(&mut e, 4));
    // Disabled through the keyboard and the mouse.
    set_control_disabled(&mut e, 4, true);
    assert!(is_key_disabled(&mut e, 0x11) && is_key_disabled(&mut e, 0x101));
    set_control_disabled(&mut e, 4, false);
    assert!(!is_key_disabled(&mut e, 0x11));
    // A ready controller: A down in the game's copy, not in the pad's own.
    state(&mut e).ready = true;
    e.mem.set_u16(PAD_STATE + 4, mask::BUTTON_A);
    assert!(is_control_pressed(&mut e, 4, 1));
    assert!(!is_control_pressed(&mut e, 4, 2), "the pad's own state");
    state(&mut e).pad.left_trigger = 1;
    assert!(is_control_pressed(&mut e, 5, 2));
    // The left stick up past the dead zone (7849).
    state(&mut e).pad.thumb_ly = 7849;
    assert!(!is_control_pressed_raw(&mut e, 6));
    state(&mut e).pad.thumb_ly = 7850;
    assert!(is_control_pressed_raw(&mut e, 6));
    // Disabling a button takes it out of the mask of buttons that pass; JIP's "disabled" answer
    // reads that mask as written (set: not disabled).
    assert!(xi_control_disabled(&mut e, 4));
    set_xi_control_disabled(&mut e, 4, true);
    assert_eq!(state(&mut e).mods.button_skip & mask::BUTTON_A, 0);
    assert!(!xi_control_disabled(&mut e, 4));
    set_xi_control_disabled(&mut e, 4, false);
    assert_ne!(state(&mut e).mods.button_skip & mask::BUTTON_A, 0);
    set_xi_control_disabled(&mut e, 5, true);
    assert!(xi_control_disabled(&mut e, 5), "a trigger's bit");
    set_xi_control_disabled(&mut e, 6, true);
    assert_eq!(state(&mut e).mods.stick_skip, 1);
    assert!(tap_xi_control(&mut e, 4));
    assert!(tap_xi_control(&mut e, 5));
    assert!(
        !tap_xi_control(&mut e, 6),
        "a stick direction is not tapped"
    );
    assert_eq!(state(&mut e).mods.button_tap, mask::BUTTON_A);
    assert_eq!(state(&mut e).mods.trigger_mods & 0x10, 0x10);
}

fn int_arg(v: i32) -> Vec<u8> {
    let mut b = vec![b'n'];
    b.extend_from_slice(&v.to_le_bytes());
    b
}

/// Runs `opcode` through its record with integer arguments: `al` and the result (0 first).
fn run(e: &mut Engine, opcode: u32, ints: &[i32]) -> (u32, f64) {
    let args: Vec<Vec<u8>> = ints.iter().map(|v| int_arg(*v)).collect();
    let words = line(e, opcode, &args);
    let execute = crate::test_support::execute_of(e, opcode);
    let ok = e.call(execute, &words).eax & 0xff;
    (ok, e.mem.f64(RESULT))
}

/// The six commands through their records: `IsKeyPressed` (default flags 1, written 0 or 1),
/// `Disable`/`EnableControl`, `TapControl` (the key, else the mouse button; a controller's
/// binding when ready), `IsControlDisabled` (the key and the button), `IsControlPressed`; a
/// control past the 28 does nothing; the two evals.
#[test]
fn input_commands() {
    let mut e = world(true);
    key(&mut e, 0x11).game_state = true;
    assert_eq!(run(&mut e, 0x1453, &[0x11]), (1, 1.0));
    assert_eq!(run(&mut e, 0x1453, &[0x11, 2]), (1, 0.0), "the raw state");
    assert_eq!(run(&mut e, 0x146b, &[4]), (1, 1.0));
    // Control 28's keyboard byte would be the mouse's first: bound to the pressed key here, so
    // only the refusal answers 0.
    e.mem.set_u8(CONTROLS_AT + 0x1b94 + 28, 0x11);
    assert_eq!(run(&mut e, 0x146b, &[28]), (1, 0.0), "past the 28");
    assert_eq!(run(&mut e, 0x1462, &[4, 0]).0, 1);
    assert!(is_key_disabled(&mut e, 0x11) && is_key_disabled(&mut e, 0x101));
    assert_eq!(
        run(&mut e, 0x146a, &[4]),
        (1, 1.0),
        "key and button disabled"
    );
    set_key_disable_state(&mut e, 0x101, false);
    assert_eq!(run(&mut e, 0x146a, &[4]), (1, 0.0), "the button not");
    set_key_disable_state(&mut e, 0x102, true);
    assert_eq!(run(&mut e, 0x146a, &[5]), (1, 1.0), "no key: the button");
    assert_eq!(run(&mut e, 0x1463, &[4, 0]).0, 1);
    assert!(!is_key_disabled(&mut e, 0x11));
    assert_eq!(run(&mut e, 0x1464, &[4]), (1, 1.0));
    assert!(key(&mut e, 0x11).tap, "the key");
    assert_eq!(run(&mut e, 0x1464, &[5]), (1, 1.0));
    assert!(key(&mut e, 0x102).tap, "no key: the mouse button");
    assert_eq!(run(&mut e, 0x1464, &[7]), (1, 0.0), "nothing bound");
    state(&mut e).ready = true;
    assert_eq!(run(&mut e, 0x1464, &[4]), (1, 1.0));
    assert_eq!(state(&mut e).mods.button_tap, mask::BUTTON_A);
    assert_eq!(
        run(&mut e, 0x146a, &[4]),
        (1, 1.0),
        "JIP's controller answer"
    );
    state(&mut e).ready = false;
    let r = RESULT;
    e.mem.set_f64(r, 0.0);
    eval(&mut e, 0x1453, &[0, 0x11, 1, r]);
    assert_eq!(e.mem.f64(r), 1.0);
    e.mem.set_f64(r, 0.0);
    eval(&mut e, 0x146b, &[0, 4, 1, r]);
    assert_eq!(e.mem.f64(r), 1.0);
    e.mem.set_f64(r, 0.0);
    eval(&mut e, 0x146b, &[0, 28, 1, r]);
    assert_eq!(e.mem.f64(r), 0.0, "past the 28");
}

/// Calls `opcode`'s eval word with the condition's four words.
fn eval(e: &mut Engine, opcode: u32, words: &[u32]) {
    let at = e.mem.u32(e.script_tables().record(opcode) + 0x20);
    e.call(at, words);
}
