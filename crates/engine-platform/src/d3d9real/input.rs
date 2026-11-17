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

/// A key of the window went down or up (`repeat`: down again while held; `alt`: Alt is held), by
/// DirectInput code: as `win32.rs`'s `window_input` takes a key message ([`key_into`]).
pub(super) fn key(code: u8, down: bool, repeat: bool, alt: bool) {
    let report = {
        let Ok(mut i) = input().lock() else { return };
        key_into(&mut i, code, down, repeat, alt, d3d9_wgpu::frame_rate::key)
    };
    if report {
        shot::report_asked();
    }
}

/// [`key`] into `i`, F11 asked of `counter` (`d3d9_wgpu::frame_rate::key`): true when the report
/// is to be made. F11 goes to the frame-rate counter when `counter` takes it (while wgpu draws the
/// window: its first press toggles, repeats and the release change nothing) and the game never
/// sees it then; F12's first press without Alt asks for the report, as the Win32 window asks only
/// on `WM_KEYDOWN` (with Alt held Windows sends `WM_SYSKEYDOWN`), and the key still reaches the
/// game; every other key is held or let go.
pub(super) fn key_into(
    i: &mut Input,
    code: u8,
    down: bool,
    repeat: bool,
    alt: bool,
    counter: impl FnOnce(bool, bool) -> bool,
) -> bool {
    if code == DIK_F11 && counter(down, repeat) {
        return false;
    }
    i.keys[usize::from(code)] = down;
    down && !repeat && !alt && code == DIK_F12
}

/// A mouse button by DirectInput's order (left 0, right 1, middle 2, X1 3, X2 4) went down or up.
pub(super) fn button(index: usize, down: bool) {
    if let Ok(mut i) = input().lock() {
        button_into(&mut i, index, down);
    }
}

fn button_into(i: &mut Input, index: usize, down: bool) {
    if let Some(b) = i.buttons.get_mut(index) {
        *b = down;
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
/// `release_input` calls this too), the motion kept for the host's next frame.
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

/// The DirectInput code of a key event: `None` when it reaches nothing. A press the system makes
/// up (`synthetic`: X11's for keys already held when the focus comes) is dropped, as Windows sends
/// none; a key with no code of its own (`name` `None`) or not in [`dik_of_code`] reaches nothing.
pub(super) fn key_event(synthetic: bool, name: Option<&str>) -> Option<u8> {
    if synthetic {
        return None;
    }
    dik_of_code(name?)
}

/// A mouse button as a window layer names it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Button {
    Left,
    Right,
    Middle,
    /// The first side button (X1, "back").
    Back,
    /// The second side button (X2, "forward").
    Forward,
    Other,
}

/// The button's place in DirectInput's order, as the Win32 window's messages give it (left 0,
/// right 1, middle 2, X1 3, X2 4); `None` for a button Windows has no message for.
pub(super) fn button_index(button: Button) -> Option<usize> {
    match button {
        Button::Left => Some(0),
        Button::Right => Some(1),
        Button::Middle => Some(2),
        Button::Back => Some(3),
        Button::Forward => Some(4),
        Button::Other => None,
    }
}

/// Whether a press of `button` holds the cursor in the window, as a click in the client area does
/// on Windows (`WM_LBUTTONDOWN`, `WM_RBUTTONDOWN`, `WM_MBUTTONDOWN`; not the side buttons).
pub(super) fn holds_cursor(button: Button) -> bool {
    matches!(button, Button::Left | Button::Right | Button::Middle)
}

/// A turn of the wheel as a window layer gives it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum Scroll {
    /// In notches (lines), away from the player positive.
    Lines(f32),
    /// In pixels: a touchpad, or a high-resolution wheel between its notches.
    Pixels(f64),
}

/// The wheel motion of `scroll` in `WM_MOUSEWHEEL`'s units (120 a notch, `WHEEL_DELTA`), its
/// fractions kept in `rest`. Pixel scrolling gives nothing: Windows hands the game a precision
/// touchpad's scrolling as wheel units its driver chooses, and no stated ratio of pixels to those
/// units was found, so none is made up (a touchpad does not scroll the game here; a mouse wheel
/// does).
pub(super) fn wheel(rest: &mut f64, scroll: Scroll) -> i32 {
    match scroll {
        Scroll::Lines(notches) => whole(rest, f64::from(notches) * WHEEL_NOTCH),
        Scroll::Pixels(_) => 0,
    }
}

/// The mouse's whole counts for a device motion `delta`, its fractions kept in `rest`: none while
/// no window of the game's has the focus, as raw input reaches only the window in front on Windows
/// (and then nothing is kept).
pub(super) fn motion(focused: bool, rest: &mut [f64; 2], delta: (f64, f64)) -> Option<(i32, i32)> {
    if !focused {
        return None;
    }
    let dx = whole(&mut rest[0], delta.0);
    let dy = whole(&mut rest[1], delta.1);
    (dx != 0 || dy != 0).then_some((dx, dy))
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

/// `SWP_NOSIZE`, `SWP_NOMOVE`: the game's `SetWindowPos` flags keeping the size or the place.
pub(super) const NO_SIZE: u32 = 1;
pub(super) const NO_MOVE: u32 = 2;

/// The client area's size for the rectangle `rect` the game asks its top-level window to take
/// (`SetWindowPos`), `border` being the frame the game added to its client area for it (the
/// renderer's start, exe 0x004da951: `AdjustWindowRect` with the window's style, then
/// `SetWindowPos`): the rectangle less that frame, so the client area is the size the game
/// wanted, whatever frame this desktop draws round it. At least 1 by 1.
pub(super) fn inner_for_game_rect(rect: [i32; 4], border: [i32; 4]) -> (u32, u32) {
    let side = |a: i32, b: i32, low: i32, high: i32| {
        let want = i64::from(b) - i64::from(a);
        let frame = i64::from(high) - i64::from(low);
        (want - frame).clamp(1, i64::from(u32::MAX)) as u32
    };
    (
        side(rect[0], rect[2], border[0], border[2]),
        side(rect[1], rect[3], border[1], border[3]),
    )
}

/// What the game's window style gives its frame, as Windows draws it: resizable with
/// `WS_THICKFRAME`; a window menu, so a close button, with `WS_SYSMENU`, and the minimise and
/// maximise buttons only with it and their own `WS_MINIMIZEBOX`, `WS_MAXIMIZEBOX`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Frame {
    pub resizable: bool,
    pub close: bool,
    pub minimise: bool,
    pub maximise: bool,
}

const WS_THICKFRAME: u32 = 0x0004_0000;
const WS_SYSMENU: u32 = 0x0008_0000;
const WS_MINIMIZEBOX: u32 = 0x0002_0000;
const WS_MAXIMIZEBOX: u32 = 0x0001_0000;

pub(super) fn frame_of_style(style: u32) -> Frame {
    let menu = style & WS_SYSMENU != 0;
    Frame {
        resizable: style & WS_THICKFRAME != 0,
        close: menu,
        minimise: menu && style & WS_MINIMIZEBOX != 0,
        maximise: menu && style & WS_MAXIMIZEBOX != 0,
    }
}

/// Runs `f` (a window layer's loop), answering the message of a panic in it instead of passing
/// the panic on: a library the layer loads at run time that is missing, or a window system that
/// lacks what it needs, stops winit with a panic, and the run is then ended with a line naming it.
pub(super) fn guarded(f: impl FnOnce()) -> Result<(), String> {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)).map_err(|p| {
        p.downcast_ref::<&str>()
            .map(|s| s.to_string())
            .or_else(|| p.downcast_ref::<String>().cloned())
            .unwrap_or_else(|| "a panic with no message".into())
    })
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

    /// The client area is the game's rectangle less the frame the game added for it (Windows'
    /// 16 by 39 border of a captioned window: 8 left and right, 31 above, 8 below), so a 1280 by 720
    /// back buffer gets a 1280 by 720 client area, whatever frame this desktop draws; without a
    /// frame the whole rectangle; never below 1 by 1.
    #[test]
    fn the_client_area_is_the_rectangle_less_the_frame_the_game_added() {
        let windows = [-8, -31, 8, 8];
        assert_eq!(inner_for_game_rect([0, 0, 1296, 759], windows), (1280, 720));
        assert_eq!(
            inner_for_game_rect([100, 50, 1396, 809], windows),
            (1280, 720)
        );
        assert_eq!(inner_for_game_rect([10, 20, 810, 620], [0; 4]), (800, 600));
        assert_eq!(inner_for_game_rect([0, 0, 5, 5], windows), (1, 1));
        assert_eq!(inner_for_game_rect([5, 5, 0, 0], [0; 4]), (1, 1));
    }

    /// The game's top-level style (WS_OVERLAPPED with WS_VISIBLE, 0x10000000, the renderer's start)
    /// gives a frame that cannot be resized and has no buttons; each style bit gives what Windows
    /// gives for it, the minimise and maximise buttons only with the window menu.
    #[test]
    fn the_frame_follows_the_games_window_style() {
        let none = Frame {
            resizable: false,
            close: false,
            minimise: false,
            maximise: false,
        };
        assert_eq!(frame_of_style(0x1000_0000), none);
        assert_eq!(frame_of_style(0x00c0_0000), none, "WS_CAPTION alone");
        // WS_OVERLAPPEDWINDOW: caption, window menu, thick frame, both boxes.
        assert_eq!(
            frame_of_style(0x00cf_0000),
            Frame {
                resizable: true,
                close: true,
                minimise: true,
                maximise: true,
            }
        );
        assert_eq!(
            frame_of_style(WS_THICKFRAME),
            Frame {
                resizable: true,
                ..none
            }
        );
        assert_eq!(
            frame_of_style(WS_SYSMENU | WS_MINIMIZEBOX),
            Frame {
                close: true,
                minimise: true,
                ..none
            }
        );
        assert_eq!(
            frame_of_style(WS_MINIMIZEBOX | WS_MAXIMIZEBOX),
            none,
            "no boxes without the window menu"
        );
    }

    /// Keys by the Win32 window's rules: F11 the counter's when it takes it (then the game never
    /// sees it), the game's otherwise; F12's first press without Alt makes the report, its repeats
    /// and Alt+F12 do not, and the game sees F12 either way; other keys held and let go.
    #[test]
    fn keys_follow_the_win32_windows_rules() {
        let mut i = Input::default();
        let taken = |_: bool, _: bool| true;
        let not_taken = |_: bool, _: bool| false;
        assert!(!key_into(&mut i, DIK_F11, true, false, false, taken));
        assert!(!i.keys[0x57], "the counter's F11 never reaches the game");
        let mut asked = None;
        key_into(&mut i, DIK_F11, true, true, false, |d, r| {
            asked = Some((d, r));
            true
        });
        assert_eq!(
            asked,
            Some((true, true)),
            "the counter is told down and repeat"
        );
        assert!(!key_into(&mut i, DIK_F11, true, false, false, not_taken));
        assert!(i.keys[0x57], "no wgpu device: F11 is the game's");
        assert!(key_into(&mut i, DIK_F12, true, false, false, taken));
        assert!(i.keys[0x58]);
        assert!(
            !key_into(&mut i, DIK_F12, true, true, false, taken),
            "a repeat"
        );
        assert!(
            !key_into(&mut i, DIK_F12, false, false, false, taken),
            "the release"
        );
        assert!(!i.keys[0x58]);
        assert!(
            !key_into(&mut i, DIK_F12, true, false, true, taken),
            "Alt+F12"
        );
        assert!(i.keys[0x58], "the game sees Alt+F12");
        assert!(!key_into(&mut i, 0x11, true, false, false, taken));
        assert!(i.keys[0x11]);
        assert!(!key_into(&mut i, 0x11, false, false, false, taken));
        assert!(!i.keys[0x11]);
        button_into(&mut i, 4, true);
        assert!(i.buttons[4]);
        button_into(&mut i, 4, false);
        button_into(&mut i, 99, true);
        assert!(i.buttons.iter().all(|b| !b), "no button past the eighth");
    }

    /// Made-up presses are dropped; a key with no code or one not in the table reaches nothing.
    #[test]
    fn made_up_and_unknown_keys_reach_nothing() {
        assert_eq!(key_event(false, Some("KeyW")), Some(0x11));
        assert_eq!(key_event(true, Some("KeyW")), None);
        assert_eq!(key_event(false, None), None);
        assert_eq!(key_event(false, Some("Fn")), None);
    }

    /// Buttons in DirectInput's order: left, right, middle, then the side buttons X1 (back) and X2
    /// (forward); only the first three hold the cursor, as their clicks do on Windows.
    #[test]
    fn buttons_go_in_directinputs_order() {
        assert_eq!(button_index(Button::Left), Some(0));
        assert_eq!(button_index(Button::Right), Some(1));
        assert_eq!(button_index(Button::Middle), Some(2));
        assert_eq!(button_index(Button::Back), Some(3));
        assert_eq!(button_index(Button::Forward), Some(4));
        assert_eq!(button_index(Button::Other), None);
        for b in [Button::Left, Button::Right, Button::Middle] {
            assert!(holds_cursor(b), "{b:?}");
        }
        for b in [Button::Back, Button::Forward, Button::Other] {
            assert!(!holds_cursor(b), "{b:?}");
        }
    }

    /// The wheel: 120 a notch, away from the player positive, fractions of a notch kept; pixel
    /// scrolling gives nothing.
    #[test]
    fn the_wheel_is_120_a_notch_and_pixels_give_nothing() {
        let mut rest = 0.0;
        assert_eq!(wheel(&mut rest, Scroll::Lines(1.0)), 120);
        assert_eq!(wheel(&mut rest, Scroll::Lines(-2.0)), -240);
        assert_eq!(wheel(&mut rest, Scroll::Lines(0.5)), 60);
        assert_eq!(wheel(&mut rest, Scroll::Pixels(300.0)), 0);
        assert_eq!(rest, 0.0);
    }

    /// The mouse moves the game only while a window of the game's has the focus; nothing is kept
    /// from motion without it.
    #[test]
    fn motion_counts_only_with_the_focus() {
        let mut rest = [0.0; 2];
        assert_eq!(motion(false, &mut rest, (5.0, -3.0)), None);
        assert_eq!(rest, [0.0; 2]);
        assert_eq!(motion(true, &mut rest, (5.0, -3.0)), Some((5, -3)));
        assert_eq!(motion(true, &mut rest, (0.5, 0.0)), None);
        assert_eq!(motion(true, &mut rest, (0.5, 0.0)), Some((1, 0)));
    }

    /// A panic in the layer's loop (winit's when a library it loads is missing) is answered as a
    /// message, so the run can end with a line naming it.
    #[test]
    fn a_panic_in_the_loop_is_answered_with_its_message() {
        assert_eq!(guarded(|| {}), Ok(()));
        assert_eq!(
            guarded(|| panic!("Library libxkbcommon-x11.so could not be loaded.")),
            Err("Library libxkbcommon-x11.so could not be loaded.".to_string())
        );
        let name = "libxkbcommon.so.0";
        assert_eq!(
            guarded(|| panic!("{name} missing")),
            Err("libxkbcommon.so.0 missing".to_string())
        );
    }
}
