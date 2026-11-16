//! The keyboard and mouse of a window layer other than Win32's (`winit_window.rs`: Linux, macOS)
//! into the DirectInput state the host merges into the game's devices ([`super::Input`]), as the
//! Win32 window's messages put them there (`win32.rs`: `window_input`, `raw_mouse`, `dik`): the
//! same DirectInput key codes, F11 the frame-rate counter's while wgpu draws the window, F12's first
//! press the report, the buttons in DirectInput's order, the wheel 120 a notch, the motion in whole
//! counts. Every system compiles this file, so its tables and counts are tested everywhere.

// On Windows the Win32 window calls only `focus_lost` (it puts its key and mouse messages into
// `Input` itself, `win32.rs`); the rest is used there by the tests.
#![cfg_attr(windows, allow(dead_code))]

use super::{input, shot, Input};

/// `DIK_F11`, `DIK_F12`.
const DIK_F11: u8 = 0x57;
const DIK_F12: u8 = 0x58;

/// The DirectInput key code of a key named by its UI Events `code` value (winit's `KeyCode`
/// names: `KeyA`, `ArrowUp`, `NumpadEnter`): the code Windows' key messages give the game for the
/// same key (`win32.rs`'s `dik`: the keyboard's set-1 scan code, 0x80 added for an extended key;
/// NumLock 0x45, Pause 0xC5). `None` for a key not in the table: it reaches nothing, as a key
/// message with scan code 0 does on Windows.
pub(super) fn dik_of_code(code: &str) -> Option<u8> {
    Some(match code {
        "Escape" => 0x01,
        "Digit1" => 0x02,
        "Digit2" => 0x03,
        "Digit3" => 0x04,
        "Digit4" => 0x05,
        "Digit5" => 0x06,
        "Digit6" => 0x07,
        "Digit7" => 0x08,
        "Digit8" => 0x09,
        "Digit9" => 0x0a,
        "Digit0" => 0x0b,
        "Minus" => 0x0c,
        "Equal" => 0x0d,
        "Backspace" => 0x0e,
        "Tab" => 0x0f,
        "KeyQ" => 0x10,
        "KeyW" => 0x11,
        "KeyE" => 0x12,
        "KeyR" => 0x13,
        "KeyT" => 0x14,
        "KeyY" => 0x15,
        "KeyU" => 0x16,
        "KeyI" => 0x17,
        "KeyO" => 0x18,
        "KeyP" => 0x19,
        "BracketLeft" => 0x1a,
        "BracketRight" => 0x1b,
        "Enter" => 0x1c,
        "ControlLeft" => 0x1d,
        "KeyA" => 0x1e,
        "KeyS" => 0x1f,
        "KeyD" => 0x20,
        "KeyF" => 0x21,
        "KeyG" => 0x22,
        "KeyH" => 0x23,
        "KeyJ" => 0x24,
        "KeyK" => 0x25,
        "KeyL" => 0x26,
        "Semicolon" => 0x27,
        "Quote" => 0x28,
        "Backquote" => 0x29,
        "ShiftLeft" => 0x2a,
        "Backslash" => 0x2b,
        "KeyZ" => 0x2c,
        "KeyX" => 0x2d,
        "KeyC" => 0x2e,
        "KeyV" => 0x2f,
        "KeyB" => 0x30,
        "KeyN" => 0x31,
        "KeyM" => 0x32,
        "Comma" => 0x33,
        "Period" => 0x34,
        "Slash" => 0x35,
        "ShiftRight" => 0x36,
        "NumpadMultiply" => 0x37,
        "AltLeft" => 0x38,
        "Space" => 0x39,
        "CapsLock" => 0x3a,
        "F1" => 0x3b,
        "F2" => 0x3c,
        "F3" => 0x3d,
        "F4" => 0x3e,
        "F5" => 0x3f,
        "F6" => 0x40,
        "F7" => 0x41,
        "F8" => 0x42,
        "F9" => 0x43,
        "F10" => 0x44,
        // Reported extended (0x45): no 0x80 added (`dik`).
        "NumLock" => 0x45,
        "ScrollLock" => 0x46,
        "Numpad7" => 0x47,
        "Numpad8" => 0x48,
        "Numpad9" => 0x49,
        "NumpadSubtract" => 0x4a,
        "Numpad4" => 0x4b,
        "Numpad5" => 0x4c,
        "Numpad6" => 0x4d,
        "NumpadAdd" => 0x4e,
        "Numpad1" => 0x4f,
        "Numpad2" => 0x50,
        "Numpad3" => 0x51,
        "Numpad0" => 0x52,
        "NumpadDecimal" => 0x53,
        "IntlBackslash" => 0x56,
        "F11" => DIK_F11,
        "F12" => DIK_F12,
        "F13" => 0x64,
        "F14" => 0x65,
        "F15" => 0x66,
        "KanaMode" => 0x70,
        "IntlRo" => 0x73,
        "Convert" => 0x79,
        "NonConvert" => 0x7b,
        "IntlYen" => 0x7d,
        // Extended keys: 0x80 added to the scan code.
        "MediaTrackPrevious" => 0x90,
        "MediaTrackNext" => 0x99,
        "NumpadEnter" => 0x9c,
        "ControlRight" => 0x9d,
        "AudioVolumeMute" => 0xa0,
        "LaunchApp2" => 0xa1,
        "MediaPlayPause" => 0xa2,
        "MediaStop" => 0xa4,
        "AudioVolumeDown" => 0xae,
        "AudioVolumeUp" => 0xb0,
        "BrowserHome" => 0xb2,
        "NumpadDivide" => 0xb5,
        "PrintScreen" => 0xb7,
        "AltRight" => 0xb8,
        // Pause's scan code is 0x45 without the extended bit (`dik`: DIK_PAUSE).
        "Pause" => 0xc5,
        "Home" => 0xc7,
        "ArrowUp" => 0xc8,
        "PageUp" => 0xc9,
        "ArrowLeft" => 0xcb,
        "ArrowRight" => 0xcd,
        "End" => 0xcf,
        "ArrowDown" => 0xd0,
        "PageDown" => 0xd1,
        "Insert" => 0xd2,
        "Delete" => 0xd3,
        "SuperLeft" => 0xdb,
        "SuperRight" => 0xdc,
        "ContextMenu" => 0xdd,
        "Power" => 0xde,
        "Sleep" => 0xdf,
        "WakeUp" => 0xe3,
        "BrowserSearch" => 0xe5,
        "BrowserFavorites" => 0xe6,
        "BrowserRefresh" => 0xe7,
        "BrowserStop" => 0xe8,
        "BrowserForward" => 0xe9,
        "BrowserBack" => 0xea,
        "LaunchApp1" => 0xeb,
        "LaunchMail" => 0xec,
        "MediaSelect" => 0xed,
        _ => return None,
    })
}

/// A key of the window went down or up (`repeat`: down again while held), by DirectInput code:
/// as `win32.rs`'s `window_input` takes a key message. F11 goes to the frame-rate counter, by the
/// rules the Win32 window's F11 follows (`d3d9_wgpu::frame_rate::key`: its first press toggles,
/// repeats and the release change nothing, and the game never sees it while wgpu draws the
/// window); F12's first press asks for the report (the key still reaches the game); every other
/// key is held or let go.
pub(super) fn key(code: u8, down: bool, repeat: bool) {
    let Ok(mut i) = input().lock() else { return };
    if code == DIK_F11 && d3d9_wgpu::frame_rate::key(down, repeat) {
        return;
    }
    if down && !repeat && code == DIK_F12 {
        shot::report_asked();
    }
    i.keys[usize::from(code)] = down;
}

/// A mouse button by DirectInput's order (left 0, right 1, middle 2, X1 3, X2 4) went down or up.
pub(super) fn button(index: usize, down: bool) {
    if let Ok(mut i) = input().lock() {
        if let Some(b) = i.buttons.get_mut(index) {
            *b = down;
        }
    }
}

/// The mouse moved by `dx`, `dy` counts and the wheel by `wheel` (120 a notch), relative, as the
/// raw input Windows hands an exclusive mouse (`win32.rs`'s `raw_mouse`).
pub(super) fn moved(dx: i32, dy: i32, wheel: i32) {
    if let Ok(mut i) = input().lock() {
        add_motion(&mut i, dx, dy, wheel);
    }
}

/// The focus left the window: every key and button it held is let go (`win32.rs`'s
/// `release_input`), the motion kept for the host's next frame.
pub(super) fn focus_lost() {
    if let Ok(mut i) = input().lock() {
        release(&mut i);
    }
}

fn add_motion(i: &mut Input, dx: i32, dy: i32, wheel: i32) {
    i.motion[0] = i.motion[0].wrapping_add(dx);
    i.motion[1] = i.motion[1].wrapping_add(dy);
    i.motion[2] = i.motion[2].wrapping_add(wheel);
}

fn release(i: &mut Input) {
    i.keys = [false; 256];
    i.buttons = [false; 8];
}

/// Whole counts of a motion given in fractions (a system that reports sub-count motion, Wayland's
/// relative pointer): `delta` added to the `rest` kept from earlier ones, the whole part (towards
/// zero) given and the rest kept, so no motion is lost or doubled.
pub(super) fn whole(rest: &mut f64, delta: f64) -> i32 {
    if !delta.is_finite() {
        return 0;
    }
    let sum = *rest + delta;
    let counts = sum.trunc();
    *rest = sum - counts;
    counts as i32
}

/// `WHEEL_DELTA`: a notch of the wheel in `WM_MOUSEWHEEL`'s units.
pub(super) const WHEEL_NOTCH: f64 = 120.0;

/// The size of the window's client area for a frame (outer) rectangle `rect` asked by the game
/// (`SetWindowPos`), the window's frame now being `outer - inner` (the frame a system draws round
/// a window is its own: the game's rectangle counts Windows' frame, and the client area takes what
/// is left once this system's frame is taken off). At least 1 by 1.
pub(super) fn inner_for_outer(rect: [i32; 4], outer: (u32, u32), inner: (u32, u32)) -> (u32, u32) {
    let frame = (
        i64::from(outer.0.saturating_sub(inner.0)),
        i64::from(outer.1.saturating_sub(inner.1)),
    );
    let want = (
        i64::from(rect[2]) - i64::from(rect[0]),
        i64::from(rect[3]) - i64::from(rect[1]),
    );
    let fit = |w: i64, f: i64| (w - f).clamp(1, i64::from(u32::MAX)) as u32;
    (fit(want.0, frame.0), fit(want.1, frame.1))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The table gives the key codes Windows' key messages give (`win32.rs`'s `dik` of the same
    /// keys' scan codes): plain keys their scan code, extended ones 0x80 added, NumLock and Pause
    /// as `dik` has them.
    #[test]
    fn key_names_give_the_codes_windows_gives() {
        for (name, code) in [
            ("KeyW", 0x11),
            ("Escape", 0x01),
            ("ControlLeft", 0x1d),
            ("AltLeft", 0x38),
            ("ControlRight", 0x9d),
            ("AltRight", 0xb8),
            ("ArrowUp", 0xc8),
            ("ArrowDown", 0xd0),
            ("ArrowLeft", 0xcb),
            ("ArrowRight", 0xcd),
            ("NumpadEnter", 0x9c),
            ("Numpad8", 0x48),
            ("NumLock", 0x45),
            ("Pause", 0xc5),
            ("F11", 0x57),
            ("F12", 0x58),
            ("Digit0", 0x0b),
            ("Digit1", 0x02),
            ("Space", 0x39),
            ("Enter", 0x1c),
            ("Tab", 0x0f),
            ("Backquote", 0x29),
            ("ShiftRight", 0x36),
            ("NumpadDivide", 0xb5),
            ("Delete", 0xd3),
            ("SuperLeft", 0xdb),
        ] {
            assert_eq!(dik_of_code(name), Some(code), "{name}");
        }
        assert_eq!(dik_of_code("Fn"), None);
        assert_eq!(dik_of_code(""), None);
    }

    /// Every key in the table has a code of its own: no two keys read as one.
    #[test]
    fn no_two_keys_share_a_code() {
        let names = [
            "Escape",
            "Digit1",
            "Digit2",
            "Digit3",
            "Digit4",
            "Digit5",
            "Digit6",
            "Digit7",
            "Digit8",
            "Digit9",
            "Digit0",
            "Minus",
            "Equal",
            "Backspace",
            "Tab",
            "KeyQ",
            "KeyW",
            "KeyE",
            "KeyR",
            "KeyT",
            "KeyY",
            "KeyU",
            "KeyI",
            "KeyO",
            "KeyP",
            "BracketLeft",
            "BracketRight",
            "Enter",
            "ControlLeft",
            "KeyA",
            "KeyS",
            "KeyD",
            "KeyF",
            "KeyG",
            "KeyH",
            "KeyJ",
            "KeyK",
            "KeyL",
            "Semicolon",
            "Quote",
            "Backquote",
            "ShiftLeft",
            "Backslash",
            "KeyZ",
            "KeyX",
            "KeyC",
            "KeyV",
            "KeyB",
            "KeyN",
            "KeyM",
            "Comma",
            "Period",
            "Slash",
            "ShiftRight",
            "NumpadMultiply",
            "AltLeft",
            "Space",
            "CapsLock",
            "F1",
            "F2",
            "F3",
            "F4",
            "F5",
            "F6",
            "F7",
            "F8",
            "F9",
            "F10",
            "NumLock",
            "ScrollLock",
            "Numpad7",
            "Numpad8",
            "Numpad9",
            "NumpadSubtract",
            "Numpad4",
            "Numpad5",
            "Numpad6",
            "NumpadAdd",
            "Numpad1",
            "Numpad2",
            "Numpad3",
            "Numpad0",
            "NumpadDecimal",
            "IntlBackslash",
            "F11",
            "F12",
            "F13",
            "F14",
            "F15",
            "KanaMode",
            "IntlRo",
            "Convert",
            "NonConvert",
            "IntlYen",
            "MediaTrackPrevious",
            "MediaTrackNext",
            "NumpadEnter",
            "ControlRight",
            "AudioVolumeMute",
            "LaunchApp2",
            "MediaPlayPause",
            "MediaStop",
            "AudioVolumeDown",
            "AudioVolumeUp",
            "BrowserHome",
            "NumpadDivide",
            "PrintScreen",
            "AltRight",
            "Pause",
            "Home",
            "ArrowUp",
            "PageUp",
            "ArrowLeft",
            "ArrowRight",
            "End",
            "ArrowDown",
            "PageDown",
            "Insert",
            "Delete",
            "SuperLeft",
            "SuperRight",
            "ContextMenu",
            "Power",
            "Sleep",
            "WakeUp",
            "BrowserSearch",
            "BrowserFavorites",
            "BrowserRefresh",
            "BrowserStop",
            "BrowserForward",
            "BrowserBack",
            "LaunchApp1",
            "LaunchMail",
            "MediaSelect",
        ];
        let mut seen = std::collections::HashMap::new();
        for name in names {
            let code = dik_of_code(name).unwrap_or_else(|| panic!("{name} not in the table"));
            assert_ne!(code, 0, "{name}");
            if let Some(other) = seen.insert(code, name) {
                panic!("{name} and {other} both {code:#x}");
            }
        }
        assert_eq!(seen.len(), names.len());
    }

    /// Motion in fractions comes out in whole counts, none lost or doubled; the rest carries over,
    /// in either direction; a motion that is not a number moves nothing.
    #[test]
    fn fractions_of_a_count_are_kept_for_the_next_motion() {
        let mut rest = 0.0;
        assert_eq!(whole(&mut rest, 0.4), 0);
        assert_eq!(whole(&mut rest, 0.4), 0);
        assert_eq!(whole(&mut rest, 0.4), 1);
        assert!((rest - 0.2).abs() < 1e-9, "{rest}");
        assert_eq!(whole(&mut rest, 2.9), 3);
        assert!((rest - 0.1).abs() < 1e-9, "{rest}");
        assert_eq!(whole(&mut rest, -1.6), -1);
        assert!((rest + 0.5).abs() < 1e-9, "{rest}");
        assert_eq!(whole(&mut rest, -0.5), -1);
        assert!(rest.abs() < 1e-9, "{rest}");
        let mut total = 0;
        let mut rest = 0.0;
        for _ in 0..1000 {
            total += whole(&mut rest, 0.25);
        }
        assert_eq!(total, 250);
        assert_eq!(whole(&mut rest, f64::NAN), 0);
        assert_eq!(whole(&mut rest, f64::INFINITY), 0);
        assert_eq!(rest, 0.0);
    }

    /// The motion adds up until the host takes it; letting go of the focus lets go of every key
    /// and button and keeps the motion.
    #[test]
    fn the_motion_adds_up_and_focus_loss_lets_go() {
        let mut i = Input::default();
        add_motion(&mut i, 3, -2, 120);
        add_motion(&mut i, 4, 5, -240);
        assert_eq!(i.motion, [7, 3, -120]);
        i.keys[0x11] = true;
        i.keys[0xc8] = true;
        i.buttons[0] = true;
        i.buttons[4] = true;
        release(&mut i);
        assert!(i.keys.iter().all(|k| !k) && i.buttons.iter().all(|b| !b));
        assert_eq!(i.motion, [7, 3, -120]);
    }

    /// The client area the game's frame rectangle leaves once this system's frame is taken off;
    /// without a frame the whole rectangle; never below 1 by 1.
    #[test]
    fn the_client_area_is_the_rectangle_less_this_systems_frame() {
        assert_eq!(
            inner_for_outer([0, 0, 1296, 759], (1300, 760), (1280, 720)),
            (1276, 719)
        );
        assert_eq!(
            inner_for_outer([10, 20, 810, 620], (800, 600), (800, 600)),
            (800, 600)
        );
        assert_eq!(inner_for_outer([0, 0, 5, 5], (40, 40), (20, 20)), (1, 1));
        assert_eq!(inner_for_outer([5, 5, 0, 0], (0, 0), (0, 0)), (1, 1));
        // A frame that reads larger inside than out (a system mid-change) counts as none.
        assert_eq!(
            inner_for_outer([0, 0, 640, 480], (600, 400), (640, 480)),
            (640, 480)
        );
    }
}
