//! The game's window on Windows (the window layer of `super`, ADR-0008): a real Win32 window for
//! each of the game's modelled windows, made and pumped on a thread of its own, its keyboard and
//! mouse (window messages, raw input) into the DirectInput state `super` keeps ([`super::Input`]).
//!
//! Moved here unchanged from `d3d9real.rs` (Linux and macOS PR 3a): the drawing hand-over, the
//! render thread and the renderer choice stayed in `super`, which every system compiles; this file
//! is Windows only. What `super` takes from a window layer: [`window_thread`] (the real windows are
//! made by `Request`s sent to it), [`window_titled`] and [`window_moved`].

use super::*;
use windows::core::PCSTR;
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, RECT, WPARAM};
use windows::Win32::Graphics::Gdi::{
    ClientToScreen, EnumDisplaySettingsW, GetMonitorInfoW, MonitorFromWindow, DEVMODEW,
    ENUM_CURRENT_SETTINGS, MONITORINFOEXW, MONITOR_DEFAULTTOPRIMARY,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleA;
use windows::Win32::UI::HiDpi::{
    GetAwarenessFromDpiAwarenessContext, GetThreadDpiAwarenessContext,
    SetThreadDpiAwarenessContext, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
};
use windows::Win32::UI::Input::{
    GetRawInputData, RegisterRawInputDevices, HRAWINPUT, MOUSE_MOVE_ABSOLUTE, RAWINPUT,
    RAWINPUTDEVICE, RAWINPUTDEVICE_FLAGS, RAWINPUTHEADER, RID_INPUT, RIM_TYPEMOUSE,
};
use windows::Win32::UI::WindowsAndMessaging::*;

// The window.

const CLASS_NAME: PCSTR = PCSTR(b"nv-rs-d3d9\0".as_ptr());

extern "system" fn window_procedure(
    window: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    // Closing the game's window ends the run (the player's way out, also when the game has
    // stopped); the window itself is not destroyed first.
    if message == WM_CLOSE {
        eprintln!(
            "NV_D3D9={}: the game's window was closed: the run ends",
            tag()
        );
        // What the host asked to finish first (the copy-out of saves, `crate::on_exit`). The window
        // runs on its own thread, so this covers only the jobs main has already handed on: a save
        // written in the frame still running on main when the window closes can be lost.
        crate::run_exit_hooks();
        crate::end_process(0);
    }
    // Activated (Alt+Tab back, a click, the screen unlocked), the display changed, or the machine
    // woke: wgpu asks its window for a frame at once rather than at the end of a rest
    // (`d3d9_wgpu::window_changed`); a display change also refits the borderless window.
    let woke = message == WM_POWERBROADCAST
        && matches!(
            wparam.0 as u32,
            PBT_APMRESUMEAUTOMATIC | PBT_APMRESUMESUSPEND
        );
    let activated = message == WM_ACTIVATE && wparam.0 & 0xffff != 0;
    if activated || woke || message == WM_DISPLAYCHANGE || message == WM_DPICHANGED {
        d3d9_wgpu::window_changed();
    }
    if message == WM_DISPLAYCHANGE {
        refit_borderless(window);
    }
    match message {
        WM_KEYDOWN | WM_SYSKEYDOWN | WM_KEYUP | WM_SYSKEYUP | WM_LBUTTONDOWN | WM_LBUTTONUP
        | WM_RBUTTONDOWN | WM_RBUTTONUP | WM_MBUTTONDOWN | WM_MBUTTONUP | WM_XBUTTONDOWN
        | WM_XBUTTONUP | WM_MOUSEWHEEL => {
            if matches!(message, WM_LBUTTONDOWN | WM_RBUTTONDOWN | WM_MBUTTONDOWN) {
                hold_cursor(window);
            }
            window_input(window, message, wparam, lparam)
        }
        WM_INPUT => raw_mouse(lparam),
        // While the window is active the cursor stays in it, hidden (the game's DirectInput
        // mouse is exclusive and foreground: Windows hides and holds the cursor for it). It is
        // taken on a keyboard activation (Alt+Tab, WA_ACTIVE) or a click in the client area
        // (above); a click that activates the window elsewhere (WA_CLICKACTIVE, e.g. on the
        // title bar) leaves it free, and so does moving or sizing the window: holding it in the
        // client area there pulled the cursor below the title bar, the move loop followed it,
        // and the window slid down the screen (playtest, 2026-10-30).
        WM_ACTIVATE if wparam.0 & 0xffff == WA_ACTIVE as usize => hold_cursor(window),
        WM_ENTERSIZEMOVE => let_go_of_cursor(),
        WM_SIZE | WM_MOVE if cursor_held() => hold_cursor(window),
        WM_SETCURSOR if cursor_held() && lparam.0 as u32 & 0xffff == HTCLIENT => {
            unsafe { SetCursor(None) };
            return LRESULT(1);
        }
        // Losing the focus lets go of everything this window held.
        WM_ACTIVATE if wparam.0 & 0xffff == 0 => release_input(),
        WM_KILLFOCUS => release_input(),
        _ => {}
    }
    // Alt and F10 are the game's keys: they do not open the window menu (Alt+F4 still closes
    // nothing, as above).
    if message == WM_SYSKEYDOWN || message == WM_SYSKEYUP {
        return LRESULT(0);
    }
    unsafe { DefWindowProcA(window, message, wparam, lparam) }
}

fn release_input() {
    if let Ok(mut i) = input().lock() {
        i.keys = [false; 256];
        i.buttons = [false; 8];
    }
    let_go_of_cursor();
}

/// Frees the cursor [`hold_cursor`] held (no clip; Windows shows it again).
fn let_go_of_cursor() {
    if CURSOR_HELD.swap(false, std::sync::atomic::Ordering::Relaxed) {
        unsafe {
            let _ = ClipCursor(None);
            ShowCursor(true);
        }
    }
}

/// The centre of the held client area, in screen coordinates: the cursor is put back there after
/// each raw mouse motion, so it never rests on the area's edge (where it showed, playtest
/// 2026-11-03). The motion the game reads comes from raw input, not the cursor's position.
static CURSOR_CENTRE: [std::sync::atomic::AtomicI32; 2] = [
    std::sync::atomic::AtomicI32::new(0),
    std::sync::atomic::AtomicI32::new(0),
];

/// Whether the window holds the cursor (clipped to its client area, hidden over it).
static CURSOR_HELD: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

fn cursor_held() -> bool {
    CURSOR_HELD.load(std::sync::atomic::Ordering::Relaxed)
}

/// Holds the cursor in the window's client area and hides it there, as Windows does for a
/// program whose DirectInput mouse is exclusive and in the foreground (the game's:
/// `SetCooperativeLevel` with `DISCL_EXCLUSIVE | DISCL_FOREGROUND`).
fn hold_cursor(window: HWND) {
    unsafe {
        let mut rect = RECT::default();
        if GetClientRect(window, &mut rect).is_err() {
            return;
        }
        let mut corners = [
            POINT {
                x: rect.left,
                y: rect.top,
            },
            POINT {
                x: rect.right,
                y: rect.bottom,
            },
        ];
        for corner in &mut corners {
            let _ = ClientToScreen(window, corner);
        }
        let screen = RECT {
            left: corners[0].x,
            top: corners[0].y,
            right: corners[1].x,
            bottom: corners[1].y,
        };
        if ClipCursor(Some(&screen)).is_ok() {
            let centre = [
                (screen.left + screen.right) / 2,
                (screen.top + screen.bottom) / 2,
            ];
            for (slot, value) in CURSOR_CENTRE.iter().zip(centre) {
                slot.store(value, std::sync::atomic::Ordering::Relaxed);
            }
            if !CURSOR_HELD.swap(true, std::sync::atomic::Ordering::Relaxed) {
                ShowCursor(false);
            }
            SetCursor(None);
            let _ = SetCursorPos(centre[0], centre[1]);
        }
    }
}

/// Registers the window for the mouse's raw input: its relative counts are what DirectInput
/// hands an exclusive mouse (no pointer acceleration, not stopped at the screen's edge).
fn register_raw_mouse(window: HWND) {
    let device = RAWINPUTDEVICE {
        usUsagePage: 1,
        usUsage: 2,
        dwFlags: RAWINPUTDEVICE_FLAGS(0),
        hwndTarget: window,
    };
    let _ =
        unsafe { RegisterRawInputDevices(&[device], std::mem::size_of::<RAWINPUTDEVICE>() as u32) };
}

/// A raw input message: a mouse's relative motion is added to the frame's.
fn raw_mouse(lparam: LPARAM) {
    let mut raw = RAWINPUT::default();
    let mut size = std::mem::size_of::<RAWINPUT>() as u32;
    let read = unsafe {
        GetRawInputData(
            HRAWINPUT(lparam.0 as *mut _),
            RID_INPUT,
            Some(&mut raw as *mut RAWINPUT as *mut core::ffi::c_void),
            &mut size,
            std::mem::size_of::<RAWINPUTHEADER>() as u32,
        )
    };
    if read == u32::MAX || raw.header.dwType != RIM_TYPEMOUSE.0 {
        return;
    }
    let mouse = unsafe { raw.data.mouse };
    if mouse.usFlags.0 & MOUSE_MOVE_ABSOLUTE.0 != 0 {
        return;
    }
    if let Ok(mut i) = input().lock() {
        i.motion[0] += mouse.lLastX;
        i.motion[1] += mouse.lLastY;
    }
    if cursor_held() {
        let x = CURSOR_CENTRE[0].load(std::sync::atomic::Ordering::Relaxed);
        let y = CURSOR_CENTRE[1].load(std::sync::atomic::Ordering::Relaxed);
        let _ = unsafe { SetCursorPos(x, y) };
    }
}

/// The DirectInput key code of a key message (`lParam`: scan code in bits 16..23, the extended
/// bit 24); `None` for the fake shifts Windows sends around some extended keys.
fn dik(lparam: isize) -> Option<u8> {
    let scan = ((lparam >> 16) & 0xff) as u8;
    let extended = lparam & (1 << 24) != 0;
    if scan == 0 {
        return None;
    }
    if extended {
        if scan == 0x2a || scan == 0x36 {
            return None;
        }
        // NumLock reports as extended 0x45 (DIK_NUMLOCK); others get 0x80 (DIK_RCONTROL,
        // DIK_RMENU, the arrows, DIK_NUMPADENTER, DIK_DIVIDE, ...).
        if scan == 0x45 {
            return Some(0x45);
        }
        return Some((scan & 0x7f) | 0x80);
    }
    // Pause reports as 0x45 without the extended bit (DIK_PAUSE 0xC5).
    if scan == 0x45 {
        return Some(0xc5);
    }
    Some(scan & 0x7f)
}

/// A message of the real window that is input (the mouse's motion comes from its raw input,
/// [`raw_mouse`]).
fn window_input(_window: HWND, message: u32, wparam: WPARAM, lparam: LPARAM) {
    let Ok(mut i) = input().lock() else { return };
    let button = |i: &mut Input, b: usize, down: bool| i.buttons[b] = down;
    match message {
        WM_KEYDOWN | WM_SYSKEYDOWN | WM_KEYUP | WM_SYSKEYUP => {
            // F11: the port's frame-rate counter on or off (its first press; `d3d9_wgpu::frame_rate`).
            // While wgpu draws the window the key is the port's, so the game never sees it go down
            // or up; on the real Direct3D 9 (no counter) it is the game's.
            if d3d9_wgpu::frame_rate::window_key(message, wparam.0, lparam.0) {
                return;
            }
            // F12 (the first press, not its repeats): a report for the player to send
            // ([`shot::report_at_present`]). The key still reaches the game, as every key does.
            if message == WM_KEYDOWN && wparam.0 == 0x7b && lparam.0 & (1 << 30) == 0 {
                shot::report_asked();
            }
            if let Some(code) = dik(lparam.0) {
                i.keys[code as usize] = message == WM_KEYDOWN || message == WM_SYSKEYDOWN;
            }
        }
        WM_LBUTTONDOWN | WM_LBUTTONUP => button(&mut i, 0, message == WM_LBUTTONDOWN),
        WM_RBUTTONDOWN | WM_RBUTTONUP => button(&mut i, 1, message == WM_RBUTTONDOWN),
        WM_MBUTTONDOWN | WM_MBUTTONUP => button(&mut i, 2, message == WM_MBUTTONDOWN),
        WM_XBUTTONDOWN | WM_XBUTTONUP => {
            let b = if (wparam.0 >> 16) & 0xffff == 2 { 4 } else { 3 };
            button(&mut i, b, message == WM_XBUTTONDOWN);
        }
        WM_MOUSEWHEEL => i.motion[2] += ((wparam.0 >> 16) & 0xffff) as i16 as i32,
        _ => {}
    }
}

/// The window thread: it makes every real window (a child window's messages go to the thread
/// that made it, so parent and children share it) and pumps their messages.
pub(super) fn window_thread() -> &'static std::sync::Mutex<std::sync::mpsc::Sender<Request>> {
    static THREAD: std::sync::OnceLock<std::sync::Mutex<std::sync::mpsc::Sender<Request>>> =
        std::sync::OnceLock::new();
    THREAD.get_or_init(|| {
        let (tx, rx) = std::sync::mpsc::channel::<Request>();
        let _ = std::thread::Builder::new()
            .name("nv-d3d9-window".into())
            .spawn(move || unsafe { run_window_thread(rx) });
        std::sync::Mutex::new(tx)
    })
}

/// The borderless window ([`borderless`]), which keeps the monitor's size when the game moves or
/// sizes its window (0: none), and the game's window that fills it.
static BORDERLESS_WINDOW: std::sync::atomic::AtomicIsize = std::sync::atomic::AtomicIsize::new(0);
static BORDERLESS_CHILD: std::sync::atomic::AtomicIsize = std::sync::atomic::AtomicIsize::new(0);

/// `PBT_APMRESUMESUSPEND`, `PBT_APMRESUMEAUTOMATIC`: the machine woke from sleep.
const PBT_APMRESUMESUSPEND: u32 = 7;
const PBT_APMRESUMEAUTOMATIC: u32 = 0x12;

/// The display changed (`WM_DISPLAYCHANGE`, on the window thread, which is per-monitor DPI aware
/// with the borderless window): when `window` is the borderless window, it and the game's window
/// that fills it take the primary monitor's new size, so the picture covers the screen at a new
/// resolution as it did at the old one (each backend fits its frames to the window).
fn refit_borderless(window: HWND) {
    let borderless = BORDERLESS_WINDOW.load(std::sync::atomic::Ordering::Relaxed);
    if borderless == 0 || window.0 as isize != borderless {
        return;
    }
    let child = BORDERLESS_CHILD.load(std::sync::atomic::Ordering::Relaxed);
    unsafe {
        let (width, height) = (GetSystemMetrics(SM_CXSCREEN), GetSystemMetrics(SM_CYSCREEN));
        for w in [borderless, child].into_iter().filter(|&w| w != 0) {
            let _ = SetWindowPos(
                HWND(w as *mut _),
                None,
                0,
                0,
                width,
                height,
                SWP_NOZORDER | SWP_NOACTIVATE | SWP_ASYNCWINDOWPOS,
            );
        }
    }
}

/// The game's top-level real window (0: none yet), whose placement the log reports
/// ([`report_placement`]).
static TOP_WINDOW: std::sync::atomic::AtomicIsize = std::sync::atomic::AtomicIsize::new(0);

/// The placement lines printed so far and what the last one found (its monitor and [`Verdict`]):
/// a line is printed when the window is made, after the game first places it, and again only when
/// it is then on another monitor or found otherwise, [`MOST_PLACEMENT_LINES`] at most.
static PLACEMENT_SAID: std::sync::Mutex<(u32, Option<Said>)> = std::sync::Mutex::new((0, None));
/// What a placement line found: the monitor's rectangle and how the window lies on it.
type Said = ([i32; 4], Verdict);
const MOST_PLACEMENT_LINES: u32 = 4;

/// One log line on where the game's top-level window `window` is ([`placement_line`]), when it is
/// that window and the line would say something new ([`PLACEMENT_SAID`]). Only reads: no call
/// here changes a window, the thread's DPI awareness, the activation or the messages, and a call
/// that fails only leaves its part out.
fn report_placement(window: HWND, borderless: bool, when: &str) {
    if window.0 as isize != TOP_WINDOW.load(std::sync::atomic::Ordering::Relaxed) {
        return;
    }
    if let Some(placement) = next_placement(&PLACEMENT_SAID, || read_placement(window, borderless))
    {
        eprintln!("NV_D3D9={}: {when}: {}", tag(), placement_line(&placement));
    }
}

/// The placement `read` gives when a line on it is to be printed, counted in `said`: none once
/// [`MOST_PLACEMENT_LINES`] are out, and then `read` (the Win32 calls) is not made at all; none
/// either when it would say what the last line said. The lock is held over `read`, whose calls
/// send no message to any window.
fn next_placement(
    said: &std::sync::Mutex<(u32, Option<Said>)>,
    read: impl FnOnce() -> Option<Placement>,
) -> Option<Placement> {
    let Ok(mut said) = said.lock() else {
        return None;
    };
    if said.0 >= MOST_PLACEMENT_LINES {
        return None;
    }
    let placement = read()?;
    let key = (placement.judged().1, placement.verdict());
    if said.1 == Some(key) {
        return None;
    }
    *said = (said.0 + 1, Some(key));
    Some(placement)
}

/// Where `window` is, as the calling thread reads it, and in physical pixels where that thread is
/// not per-monitor aware: the window's visible frame from DWM's frame bounds (never scaled; they
/// leave out a framed window's invisible resize border, which `GetWindowRect` includes), its
/// monitor's from the display's current mode. `None` when the window or its monitor cannot be read.
fn read_placement(window: HWND, borderless: bool) -> Option<Placement> {
    let rect = |r: RECT| [r.left, r.top, r.right, r.bottom];
    let mut at = RECT::default();
    let mut info = MONITORINFOEXW::default();
    info.monitorInfo.cbSize = std::mem::size_of::<MONITORINFOEXW>() as u32;
    // SAFETY: read-only queries of a window handle and the calling thread, into buffers sized
    // for them.
    unsafe {
        let awareness = GetAwarenessFromDpiAwarenessContext(GetThreadDpiAwarenessContext()).0;
        let monitor = MonitorFromWindow(window, MONITOR_DEFAULTTOPRIMARY);
        if GetWindowRect(window, &mut at).is_err()
            || !GetMonitorInfoW(monitor, &mut info.monitorInfo).as_bool()
        {
            return None;
        }
        let physical = if awareness == 2 {
            None
        } else {
            let mut frame = RECT::default();
            let mut mode = DEVMODEW {
                dmSize: std::mem::size_of::<DEVMODEW>() as u16,
                ..Default::default()
            };
            let framed = DwmGetWindowAttribute(
                window.0 as isize,
                DWMWA_EXTENDED_FRAME_BOUNDS,
                &mut frame as *mut RECT as *mut std::ffi::c_void,
                std::mem::size_of::<RECT>() as u32,
            ) == 0;
            let moded = EnumDisplaySettingsW(
                windows::core::PCWSTR(info.szDevice.as_ptr()),
                ENUM_CURRENT_SETTINGS,
                &mut mode,
            )
            .as_bool();
            (framed && moded).then(|| {
                let at = mode.Anonymous1.Anonymous2.dmPosition;
                let (w, h) = (mode.dmPelsWidth as i32, mode.dmPelsHeight as i32);
                (rect(frame), [at.x, at.y, at.x + w, at.y + h])
            })
        };
        Some(Placement {
            borderless,
            awareness,
            window: rect(at),
            monitor: rect(info.monitorInfo.rcMonitor),
            physical,
            primary: info.monitorInfo.dwFlags & MONITORINFOF_PRIMARY != 0,
        })
    }
}

/// `DWMWA_EXTENDED_FRAME_BOUNDS`: the window's visible frame on the screen (no invisible resize
/// border, no shadow), in physical pixels whatever the caller's DPI awareness.
const DWMWA_EXTENDED_FRAME_BOUNDS: u32 = 9;

#[link(name = "dwmapi")]
extern "system" {
    fn DwmGetWindowAttribute(
        hwnd: isize,
        attribute: u32,
        value: *mut std::ffi::c_void,
        size: u32,
    ) -> i32;
}

/// Where the game's top-level window is ([`read_placement`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Placement {
    borderless: bool,
    /// The reading thread's DPI awareness: 0 not aware (scaled pixels on a scaled monitor), 1
    /// system aware, 2 per-monitor aware (physical pixels).
    awareness: i32,
    /// The window's rectangle and its monitor's, as that thread reads them.
    window: [i32; 4],
    monitor: [i32; 4],
    /// The same two in physical pixels, read where the thread is not per-monitor aware: the
    /// window's visible frame (without a framed window's invisible resize border) and the monitor.
    physical: Option<([i32; 4], [i32; 4])>,
    /// Whether the monitor is the primary one.
    primary: bool,
}

/// How the window lies on its monitor: exactly over it, inside it, or reaching beyond its left,
/// top, right and bottom edges by so many pixels (0 where it does not).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Verdict {
    Covers,
    Inside,
    Beyond([i32; 4]),
}

impl Placement {
    /// The pair judged ([`Placement::verdict`]): the physical one when there is one, and its unit.
    fn judged(&self) -> ([i32; 4], [i32; 4], &'static str) {
        match self.physical {
            Some((window, monitor)) => (window, monitor, "physical pixels"),
            None => (self.window, self.monitor, unit(self.awareness)),
        }
    }

    fn verdict(&self) -> Verdict {
        let (window, monitor, _) = self.judged();
        if window == monitor {
            return Verdict::Covers;
        }
        let over = [
            (monitor[0] - window[0]).max(0),
            (monitor[1] - window[1]).max(0),
            (window[2] - monitor[2]).max(0),
            (window[3] - monitor[3]).max(0),
        ];
        if over == [0; 4] {
            Verdict::Inside
        } else {
            Verdict::Beyond(over)
        }
    }
}

/// The unit a thread of DPI awareness `awareness` reads rectangles in.
fn unit(awareness: i32) -> &'static str {
    match awareness {
        0 => "scaled pixels: the thread is not DPI aware",
        1 => "system-scaled pixels: the thread is system DPI aware",
        2 => "physical pixels",
        _ => "pixels of an unknown DPI awareness",
    }
}

/// The placement line: borderless or framed (and why a window can be framed), the window's and its
/// monitor's rectangles in the reading thread's unit and in physical pixels where read, and how the
/// window lies on the monitor ([`Verdict`]), plainly when it reaches beyond it.
fn placement_line(p: &Placement) -> String {
    let kind = if p.borderless {
        "borderless"
    } else {
        "framed, not borderless: bBorderless=0 or NV_BORDERLESS=0 without bFull Screen=1 (which gives the borderless window over either), \
         a run no launcher started, or a launcher's play run that a test's or a fix's variable made a test (its own line says so)"
    };
    let which = if p.primary { "primary" } else { "other" };
    let (window, monitor) = (p.window, p.monitor);
    let (physical_window, physical_monitor) = match p.physical {
        Some((w, m)) => (
            format!(", visible frame, physical {w:?}"),
            format!(", physical {m:?}"),
        ),
        None => (String::new(), String::new()),
    };
    let unit_read = unit(p.awareness);
    let (_, _, judged_in) = p.judged();
    let lies = match (p.verdict(), p.borderless) {
        (Verdict::Covers, _) => "covers it exactly".to_string(),
        (Verdict::Inside, true) => "does NOT cover it exactly (it lies inside it)".to_string(),
        (Verdict::Inside, false) => "lies inside it".to_string(),
        (Verdict::Beyond(over), borderless) => {
            let edges: Vec<String> = ["left", "top", "right", "bottom"]
                .iter()
                .zip(over)
                .filter(|(_, by)| *by > 0)
                .map(|(edge, by)| format!("{by} past its {edge} edge"))
                .collect();
            format!(
                "{}reaches BEYOND it onto what is next to it: {} ({judged_in})",
                if borderless {
                    "does NOT cover it exactly and "
                } else {
                    ""
                },
                edges.join(", ")
            )
        }
    };
    format!(
        "the game's window ({kind}) is at {window:?} ({unit_read}){physical_window} on the {which} monitor {monitor:?}{physical_monitor}: it {lies}"
    )
}

unsafe fn run_window_thread(rx: std::sync::mpsc::Receiver<Request>) {
    let module = GetModuleHandleA(None).unwrap_or_default();
    let class = WNDCLASSA {
        lpfnWndProc: Some(window_procedure),
        hInstance: module.into(),
        hCursor: LoadCursorW(None, IDC_ARROW).unwrap_or_default(),
        lpszClassName: CLASS_NAME,
        ..Default::default()
    };
    RegisterClassA(&class);
    if borderless() {
        SetThreadDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
    }
    loop {
        while let Ok(request) = rx.try_recv() {
            let mut title = request.title;
            title.push(0);
            let [left, top, right, bottom] = request.rect;
            let child = request.parent != 0;
            // A top-level window is shown afterwards (activated, in the background for the
            // scripted runs, or never: [`Showing`]).
            let borderless = !child && borderless();
            let fills = child
                && request.parent != 0
                && request.parent == BORDERLESS_WINDOW.load(std::sync::atomic::Ordering::Relaxed);
            let style = if child {
                request.style
            } else if borderless {
                WS_POPUP.0
            } else {
                request.style & !WS_VISIBLE.0
            };
            let (left, top, right, bottom) = if borderless || fills {
                (
                    0,
                    0,
                    GetSystemMetrics(SM_CXSCREEN),
                    GetSystemMetrics(SM_CYSCREEN),
                )
            } else {
                (left, top, right, bottom)
            };
            let ex_style = if borderless { 0 } else { request.ex_style };
            let window = CreateWindowExA(
                WINDOW_EX_STYLE(ex_style),
                CLASS_NAME,
                PCSTR(title.as_ptr()),
                WINDOW_STYLE(style),
                left,
                top,
                right.wrapping_sub(left),
                bottom.wrapping_sub(top),
                HWND(request.parent as *mut _),
                None,
                module,
                None,
            );
            let handle = match window {
                Ok(w) => {
                    if !child {
                        register_raw_mouse(w);
                        // The first top-level window is the game's.
                        let _ = TOP_WINDOW.compare_exchange(
                            0,
                            w.0 as isize,
                            std::sync::atomic::Ordering::Relaxed,
                            std::sync::atomic::Ordering::Relaxed,
                        );
                        report_placement(w, borderless, "made");
                    }
                    if borderless {
                        BORDERLESS_WINDOW.store(w.0 as isize, std::sync::atomic::Ordering::Relaxed);
                    }
                    if fills {
                        BORDERLESS_CHILD.store(w.0 as isize, std::sync::atomic::Ordering::Relaxed);
                    }
                    if !child && request.style & WS_VISIBLE.0 != 0 {
                        match request.show {
                            Showing::Activated => {
                                let _ = ShowWindow(w, SW_SHOW);
                            }
                            Showing::Background => {
                                let _ = ShowWindow(w, SW_SHOWNOACTIVATE);
                                // In the background a new window still lands on top of the
                                // stack, in front of whatever the person at the machine is
                                // using: put it at the bottom instead (it keeps drawing there;
                                // the game's own `SetWindowPos` calls keep the order,
                                // `SWP_NOZORDER` in `window_moved`).
                                let _ = SetWindowPos(
                                    w,
                                    HWND_BOTTOM,
                                    0,
                                    0,
                                    0,
                                    0,
                                    SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
                                );
                            }
                            // Nothing shows it later either: `window_moved` never passes
                            // `SWP_SHOWWINDOW`, and the windowed device does not show its window.
                            Showing::Never => {}
                        }
                    }
                    w.0 as isize
                }
                Err(_) => 0,
            };
            let _ = request.reply.send(handle);
        }
        MsgWaitForMultipleObjects(None, false, 10, QS_ALLINPUT);
        let mut msg = MSG::default();
        while PeekMessageA(&mut msg, None, 0, 0, PM_REMOVE).as_bool() {
            let _ = TranslateMessage(&msg);
            DispatchMessageA(&msg);
        }
    }
}

/// The real window takes the title the game gave its record (`SetWindowTextA`).
pub fn window_titled(e: &mut Engine, handle: u32, title: &[u8]) {
    let Some(window) = real_window(e, handle) else {
        return;
    };
    let mut text = title.to_vec();
    text.push(0);
    unsafe {
        let _ = SetWindowTextA(HWND(window as *mut _), PCSTR(text.as_ptr()));
    }
}

/// `SetWindowPos` moved or sized the modelled window `handle` (`SWP_NOSIZE`, `SWP_NOMOVE` in
/// `flags`): the real window follows.
pub fn window_moved(e: &mut Engine, handle: u32, rect: [i32; 4], flags: u32) {
    let Some(window) = real_window(e, handle) else {
        return;
    };
    let borderless = BORDERLESS_WINDOW.load(std::sync::atomic::Ordering::Relaxed);
    // The borderless window and the child filling it keep the monitor's size.
    if borderless != 0
        && (window == borderless
            || unsafe { GetParent(HWND(window as *mut _)) }
                .is_ok_and(|p| p.0 as isize == borderless))
    {
        // Where the game's placing left it: where it was made, so a line only when that changed.
        report_placement(
            HWND(window as *mut _),
            window == borderless,
            "after the game placed it",
        );
        return;
    }
    let [left, top, right, bottom] = rect;
    unsafe {
        let _ = SetWindowPos(
            HWND(window as *mut _),
            None,
            left,
            top,
            right.wrapping_sub(left),
            bottom.wrapping_sub(top),
            SET_WINDOW_POS_FLAGS(flags & 3) | SWP_NOZORDER | SWP_NOACTIVATE,
        );
    }
    report_placement(HWND(window as *mut _), false, "after the game placed it");
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A placement as the per-monitor aware window thread reads it (physical pixels, nothing more).
    fn aware(borderless: bool, window: [i32; 4], monitor: [i32; 4], primary: bool) -> Placement {
        Placement {
            borderless,
            awareness: 2,
            window,
            monitor,
            physical: None,
            primary,
        }
    }

    /// The placement line names the window's rectangle and its monitor's in the reading thread's
    /// unit (and in physical pixels where that is another), and says how the window lies on the
    /// monitor. The maintainer's desk of 2026-11-16 (physical pixels): the primary 2560x1440 at
    /// 0,0 (2048x1152 scaled at 125%), another to its right at 2560,2 and the laptop's 1800x1125
    /// panel at -2880,0. The borderless window over the primary covers it; one of the monitor's
    /// size at another origin does not; tonight's framed window, 2576x1479 scaled pixels at 5,5
    /// (about 3220x1849 physical from 6,6), reaches beyond it, read either way. Its physical
    /// figures here are illustrative (the scaled rectangle times 1.25): DWM's visible frame leaves
    /// out the invisible resize border, so the real line's overhang is a few pixels smaller.
    #[test]
    fn the_window_line_says_where_the_window_went() {
        let primary = [0, 0, 2560, 1440];
        let right = [2560, 2, 5120, 1442];
        let laptop = [-2880, 0, -1080, 1125];
        let line = placement_line(&aware(true, primary, primary, true));
        assert_eq!(
            line,
            "the game's window (borderless) is at [0, 0, 2560, 1440] (physical pixels) on the primary monitor [0, 0, 2560, 1440]: it covers it exactly"
        );
        // Another monitor, also at negative coordinates: named as it is.
        let other = placement_line(&aware(true, laptop, laptop, false));
        assert!(
            other.ends_with("on the other monitor [-2880, 0, -1080, 1125]: it covers it exactly"),
            "{other}"
        );
        assert!(placement_line(&aware(true, right, right, false)).ends_with("covers it exactly"));
        // The monitor's size at another origin (the right-hand monitor's, 2 pixels lower) is no
        // cover, and neither is off by one on any side.
        let shifted = placement_line(&aware(true, [0, 2, 2560, 1442], primary, true));
        assert!(
            shifted.ends_with("it does NOT cover it exactly and reaches BEYOND it onto what is next to it: 2 past its bottom edge (physical pixels)"),
            "{shifted}"
        );
        for side in 0..4 {
            let mut near = primary;
            near[side] += 1;
            let line = placement_line(&aware(true, near, primary, true));
            assert!(line.contains("does NOT cover it exactly"), "{side}: {line}");
        }
        let inside = placement_line(&aware(true, [1, 1, 2559, 1439], primary, true));
        assert!(
            inside.ends_with("does NOT cover it exactly (it lies inside it)"),
            "{inside}"
        );

        // Tonight's framed window, read on the game's thread (not DPI aware) with the physical
        // rectangles beside: judged in physical pixels. The physical window is illustrative (the
        // scaled one times 1.25, border included), not what DWM's visible frame would return.
        let tonight = Placement {
            borderless: false,
            awareness: 0,
            window: [5, 5, 2581, 1484],
            monitor: [0, 0, 2048, 1152],
            physical: Some(([6, 6, 3226, 1855], primary)),
            primary: true,
        };
        assert_eq!(tonight.verdict(), Verdict::Beyond([0, 0, 666, 415]));
        let line = placement_line(&tonight);
        assert!(
            line.starts_with("the game's window (framed, not borderless: bBorderless=0 or NV_BORDERLESS=0 without bFull Screen=1"),
            "{line}"
        );
        assert!(
            line.contains("a launcher's play run that a test's or a fix's variable made a test")
        );
        assert!(
            line.ends_with(
                "is at [5, 5, 2581, 1484] (scaled pixels: the thread is not DPI aware), visible frame, physical [6, 6, 3226, 1855] \
                 on the primary monitor [0, 0, 2048, 1152], physical [0, 0, 2560, 1440]: \
                 it reaches BEYOND it onto what is next to it: 666 past its right edge, 415 past its bottom edge (physical pixels)"
            ),
            "{line}"
        );
        // The same read without the physical pair: judged in scaled pixels, and the unit says so.
        let scaled = Placement {
            physical: None,
            ..tonight
        };
        assert_eq!(scaled.verdict(), Verdict::Beyond([0, 0, 533, 332]));
        assert!(
            placement_line(&scaled).ends_with(
                "533 past its right edge, 332 past its bottom edge (scaled pixels: the thread is not DPI aware)"
            ),
            "{}",
            placement_line(&scaled)
        );
        // Made, before the game places it: inside the monitor, framed.
        let made = Placement {
            awareness: 0,
            window: [0, 0, 320, 240],
            monitor: [0, 0, 2048, 1152],
            physical: None,
            ..tonight
        };
        assert!(placement_line(&made).ends_with(": it lies inside it"));
        // Above and left of the monitor too.
        let up_left = aware(false, [-10, -20, 100, 100], primary, true);
        assert_eq!(up_left.verdict(), Verdict::Beyond([10, 20, 0, 0]));
        assert!(placement_line(&up_left)
            .ends_with("10 past its left edge, 20 past its top edge (physical pixels)"));
        assert_eq!(
            unit(1),
            "system-scaled pixels: the thread is system DPI aware"
        );
        assert_eq!(unit(7), "pixels of an unknown DPI awareness");
    }

    /// A placement line is printed when what it finds is new, [`MOST_PLACEMENT_LINES`] at most;
    /// once they are out, the placement is not even read (its Win32 calls are not made), and a
    /// read that fails prints nothing and counts nothing.
    #[test]
    fn placement_lines_stop_reading_at_the_cap() {
        let said = std::sync::Mutex::new((0, None));
        let reads = std::cell::Cell::new(0);
        let primary = [0, 0, 2560, 1440];
        let at = |window: [i32; 4]| {
            let reads = &reads;
            move || {
                reads.set(reads.get() + 1);
                Some(aware(true, window, primary, true))
            }
        };
        assert!(next_placement(&said, at(primary)).is_some());
        // The same finding again: read, not printed, not counted.
        assert!(next_placement(&said, at(primary)).is_none());
        assert_eq!((reads.get(), said.lock().unwrap().0), (2, 1));
        // A failed read: nothing printed or counted.
        assert!(next_placement(&said, || None).is_none());
        assert_eq!(said.lock().unwrap().0, 1);
        // Something new each time until the cap.
        for k in 1..MOST_PLACEMENT_LINES as i32 {
            assert!(
                next_placement(&said, at([0, k, 2560, 1440 + k])).is_some(),
                "{k}"
            );
        }
        assert_eq!(said.lock().unwrap().0, MOST_PLACEMENT_LINES);
        let before = reads.get();
        assert!(next_placement(&said, at([0, 0, 1, 1])).is_none());
        assert_eq!(reads.get(), before, "read after the cap");
    }

    /// F11 is the port's frame-rate counter key: its press toggles the counter once, its repeats
    /// and release change nothing, and the game's keyboard never holds it (DIK_F11, 0x57), in the
    /// game's menus, Pip-Boy and console alike (they all read these keys). Another key still
    /// reaches the game.
    /// On the real Direct3D 9, which has no counter, F11 is the game's like any key.
    #[test]
    fn f11_toggles_the_frame_rate_counter_and_never_reaches_the_game() {
        let window = HWND(std::ptr::null_mut());
        // Scan code 0x57 (F11) and 0x1e (A) in bits 16..23; bit 30 set on a repeat.
        let (f11, a, repeat) = (0x0057_0001isize, 0x001e_0001isize, 1isize << 30);
        let send = |message: u32, vk: usize, lparam: isize| {
            window_input(window, message, WPARAM(vk), LPARAM(lparam))
        };
        // The real Direct3D 9 draws (no wgpu device on the window, no counter): F11 is the game's.
        d3d9_wgpu::frame_rate::set_window_device(false);
        d3d9_wgpu::frame_rate::show(false);
        send(WM_KEYDOWN, 0x7a, f11);
        assert!(held_keys()[0x57], "Direct3D 9: the game sees F11 down");
        assert!(!d3d9_wgpu::frame_rate::shown());
        send(WM_KEYUP, 0x7a, f11 | repeat | (1 << 31));
        assert!(!held_keys()[0x57]);
        // wgpu on the window: F11 is the counter's.
        d3d9_wgpu::frame_rate::set_window_device(true);
        d3d9_wgpu::frame_rate::show(false);
        send(WM_KEYDOWN, 0x7a, f11);
        assert!(d3d9_wgpu::frame_rate::shown(), "pressed: on");
        assert!(!held_keys()[0x57], "the game does not see F11 down");
        for _ in 0..10 {
            send(WM_KEYDOWN, 0x7a, f11 | repeat);
        }
        assert!(d3d9_wgpu::frame_rate::shown(), "held: still on");
        assert!(!held_keys()[0x57]);
        send(WM_KEYUP, 0x7a, f11 | repeat | (1 << 31));
        assert!(d3d9_wgpu::frame_rate::shown(), "released: still on");
        send(WM_KEYDOWN, 0x7a, f11);
        send(WM_KEYUP, 0x7a, f11 | repeat | (1 << 31));
        assert!(!d3d9_wgpu::frame_rate::shown(), "pressed again: off");
        assert!(!held_keys()[0x57]);
        send(WM_KEYDOWN, 0x41, a);
        assert!(held_keys()[0x1e], "A reaches the game");
        send(WM_KEYUP, 0x41, a | repeat | (1 << 31));
        assert!(!held_keys()[0x1e]);
        assert!(!d3d9_wgpu::frame_rate::shown());
        d3d9_wgpu::frame_rate::set_window_device(false);
    }

    /// A game window that is never shown ([`Showing::Never`]): the top-level window, asked for
    /// with `WS_VISIBLE` as the game asks (its styles as its runs print them), and the child window
    /// in it are made, the child keeping the `WS_VISIBLE` the game gave it, and neither is visible.
    /// A windowed real device on the child, the top-level window its focus window, clears and
    /// presents with `D3D_OK` (the model's answers: nothing is printed as differing) and shows no
    /// window. Skipped where Direct3D 9 has no device (no display adapter, as on CI).
    #[test]
    fn a_window_never_shown_stays_hidden_and_its_device_presents() {
        use d3d9_stream::Backend as _;
        let _turn = real_turn();
        let make = |style: u32, rect: [i32; 4], parent: isize| -> Option<isize> {
            let (reply, answer) = std::sync::mpsc::channel();
            let sent = window_thread()
                .lock()
                .map(|tx| {
                    tx.send(Request {
                        title: b"nv-rs hidden test".to_vec(),
                        style,
                        ex_style: 0,
                        rect,
                        parent,
                        show: Showing::Never,
                        reply,
                    })
                    .is_ok()
                })
                .unwrap_or(false);
            answer.recv().ok().filter(|&w| sent && w != 0)
        };
        let top = make(0x14c0_0000, [0, 0, 320, 240], 0).expect("the top-level window");
        let child = make(0x5000_0000, [0, 0, 64, 64], top).expect("the child window");
        let visible = |w: isize| unsafe { IsWindowVisible(HWND(w as *mut _)) }.as_bool();
        let style = |w: isize| unsafe { GetWindowLongW(HWND(w as *mut _), GWL_STYLE) } as u32;
        assert!(!visible(top) && !visible(child));
        assert_eq!(
            style(top) & WS_VISIBLE.0,
            0,
            "the top-level window never shown"
        );
        assert_ne!(
            style(child) & WS_VISIBLE.0,
            0,
            "the child as the game made it"
        );
        let mut b = crate::d3d9backend::RealBackend::new();
        let device = 0x1000;
        let present = test_present();
        let made = b.execute(Cmd::CreateDevice {
            device,
            focus: top,
            window: child,
            behavior: 0x54,
            present: Data::from_words(&present),
            back_buffers: Data::from_words(&[]),
            depth_stencil: 0,
        });
        if !matches!(made, Reply::Device { created: true, .. }) {
            eprintln!("no Direct3D 9 device here: the device's half of this test is skipped");
            return;
        }
        for frame in 1..=3 {
            b.execute(Cmd::Clear {
                device,
                count: 0,
                rects: None,
                flags: 1,
                colour: 0xff20_4060,
                z: 0,
                stencil: 0,
                model: 0,
            });
            b.execute(Cmd::Present {
                device,
                source: None,
                dest: None,
                window: 0,
                dirty: false,
                model: 0,
                frame,
            });
        }
        assert!(b.logged.is_empty(), "differing answers: {:?}", b.logged);
        assert!(
            !visible(top) && !visible(child),
            "the device shows no window"
        );
    }

    /// A key message's `lParam` with this scan code (extended or not).
    fn key(scan: isize, extended: bool) -> isize {
        (scan << 16) | if extended { 1 << 24 } else { 0 }
    }

    #[test]
    fn key_messages_give_directinput_key_codes() {
        // W, Escape, left Ctrl and Alt: the scan code.
        assert_eq!(dik(key(0x11, false)), Some(0x11));
        assert_eq!(dik(key(0x01, false)), Some(0x01));
        assert_eq!(dik(key(0x1d, false)), Some(0x1d));
        assert_eq!(dik(key(0x38, false)), Some(0x38));
        // Right Ctrl and Alt, the arrows, numpad Enter: extended, 0x80 added.
        assert_eq!(dik(key(0x1d, true)), Some(0x9d));
        assert_eq!(dik(key(0x38, true)), Some(0xb8));
        assert_eq!(dik(key(0x48, true)), Some(0xc8));
        assert_eq!(dik(key(0x50, true)), Some(0xd0));
        assert_eq!(dik(key(0x1c, true)), Some(0x9c));
        // The numpad's own keys are not extended (DIK_NUMPAD8 0x48).
        assert_eq!(dik(key(0x48, false)), Some(0x48));
        // NumLock (extended 0x45) and Pause (plain 0x45).
        assert_eq!(dik(key(0x45, true)), Some(0x45));
        assert_eq!(dik(key(0x45, false)), Some(0xc5));
        // The fake shifts around extended keys are dropped.
        assert_eq!(dik(key(0x2a, true)), None);
        assert_eq!(dik(key(0, false)), None);
    }
}
