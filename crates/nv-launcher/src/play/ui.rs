//! What Play's steps say and ask, in the launcher's terminal window (`crate::session`): the
//! messages Play.exe showed in Windows' message boxes, its folder picker (the folder is typed at
//! the prompt or dropped onto the window) and its small "getting ready" window (the step's line on
//! the waiting screen). With `NV_RS_NO_WINDOWS=1` (scripted tests, which must not take the focus)
//! there is no window: every message is in `userdata\logs\launcher.log` either way, and the
//! answers are the ones Play.exe's message boxes gave without a window (Yes, no folder).
//!
//! Where the launcher's window cannot be drawn at all (no graphics device takes it), Play goes on
//! as Play.exe did ([`answer_in_boxes`]): Windows' own message boxes with their error, warning and
//! information icons, its folder picker and the small "getting ready" window, so the game can
//! still be started.
use super::win::{from_wide, wide_str};
use crate::session::{self, Event, Tone};
use std::ffi::c_void;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicIsize, Ordering};

#[link(name = "user32")]
extern "system" {
    fn GetSystemMetrics(index: i32) -> i32;
    fn FindWindowW(class: *const u16, title: *const u16) -> *mut c_void;
    fn MessageBoxW(owner: *mut c_void, text: *const u16, title: *const u16, kind: u32) -> i32;
    fn SetThreadDpiAwarenessContext(context: isize) -> isize;
    fn RegisterClassW(class: *const WndClassW) -> u16;
    fn CreateWindowExW(
        ex_style: u32,
        class: *const u16,
        title: *const u16,
        style: u32,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        parent: *mut c_void,
        menu: *mut c_void,
        instance: *mut c_void,
        param: *mut c_void,
    ) -> *mut c_void;
    fn DefWindowProcW(window: *mut c_void, message: u32, w: usize, l: isize) -> isize;
    fn GetMessageW(message: *mut [usize; 8], window: *mut c_void, min: u32, max: u32) -> i32;
    fn TranslateMessage(message: *const [usize; 8]) -> i32;
    fn DispatchMessageW(message: *const [usize; 8]) -> isize;
    fn PostMessageW(window: isize, message: u32, w: usize, l: isize) -> i32;
    fn PostQuitMessage(code: i32);
    fn SetWindowTextW(window: isize, text: *const u16) -> i32;
    fn GetSysColorBrush(index: i32) -> *mut c_void;
}
#[link(name = "kernel32")]
extern "system" {
    fn GetModuleHandleW(name: *const u16) -> *mut c_void;
}
#[link(name = "shell32")]
extern "system" {
    fn SHBrowseForFolderW(info: *mut BrowseInfoW) -> *mut c_void;
    fn SHGetPathFromIDListW(list: *mut c_void, path: *mut u16) -> i32;
    fn ShellExecuteW(
        owner: *mut c_void,
        verb: *const u16,
        file: *const u16,
        parameters: *const u16,
        folder: *const u16,
        show: i32,
    ) -> isize;
}
#[link(name = "ole32")]
extern "system" {
    fn CoInitializeEx(reserved: *mut c_void, flags: u32) -> i32;
    fn CoTaskMemFree(memory: *mut c_void);
}

#[repr(C)]
struct WndClassW {
    style: u32,
    procedure: unsafe extern "system" fn(*mut c_void, u32, usize, isize) -> isize,
    class_extra: i32,
    window_extra: i32,
    instance: *mut c_void,
    icon: *mut c_void,
    cursor: *mut c_void,
    background: *mut c_void,
    menu_name: *const u16,
    class_name: *const u16,
}

#[repr(C)]
struct BrowseInfoW {
    owner: *mut c_void,
    root: *mut c_void,
    display_name: *mut u16,
    title: *const u16,
    flags: u32,
    callback: *mut c_void,
    param: isize,
    image: i32,
}

/// Play's questions go to Windows' message boxes ([`answer_in_boxes`]).
static IN_BOXES: AtomicBool = AtomicBool::new(false);

/// Whether there is no terminal window and Play asks in Windows' message boxes.
pub fn in_boxes() -> bool {
    IN_BOXES.load(Ordering::Relaxed)
}

/// Whether windows may be shown.
pub fn windows_allowed() -> bool {
    std::env::var_os("NV_RS_NO_WINDOWS").is_none_or(|v| v != "1")
}

/// The game cannot start, and why.
pub fn error(text: &str) {
    session::message_as(Tone::Error, text, &["OK"]);
}

/// Something the player should know before the game starts.
pub fn warning(text: &str) {
    session::message_as(Tone::Warning, text, &["OK"]);
}

pub fn info(text: &str) {
    session::message_as(Tone::Info, text, &["OK"]);
}

/// Yes or no; with no window, yes.
pub fn ask(text: &str) -> bool {
    session::message_as(Tone::Question, text, &["Yes", "No"]) == 0
}

/// Opens a folder in Explorer (after an error, so the player finds the log).
pub fn open_folder(path: &std::path::Path) {
    if !windows_allowed() {
        return;
    }
    let (verb, file) = (wide_str("open"), super::win::wide(path.as_os_str()));
    // SAFETY: the strings live through the call; 1 is SW_SHOWNORMAL.
    unsafe {
        ShellExecuteW(
            std::ptr::null_mut(),
            verb.as_ptr(),
            file.as_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            1,
        );
    }
}

/// Asks for a folder; `None` when the player goes back (or no windows are allowed).
pub fn pick_folder(title: &str) -> Option<PathBuf> {
    if !windows_allowed() {
        return None;
    }
    session::folder(title)
}

static STATUS_WINDOW: AtomicIsize = AtomicIsize::new(0);
static STATUS_TEXT: AtomicIsize = AtomicIsize::new(0);
const WM_DESTROY: u32 = 2;
const WM_CLOSE: u32 = 0x10;

/// The player closed the small window: only the window goes, the launcher carries on.
unsafe extern "system" fn status_procedure(
    window: *mut c_void,
    message: u32,
    w: usize,
    l: isize,
) -> isize {
    if message == WM_DESTROY {
        STATUS_TEXT.store(0, Ordering::Relaxed);
        STATUS_WINDOW.store(0, Ordering::Relaxed);
        PostQuitMessage(0);
        return 0;
    }
    DefWindowProcW(window, message, w, l)
}

/// The waiting screen's line while the start gets ready; with Windows' message boxes, Play.exe's
/// small "getting ready" window, on a thread of its own.
pub struct Status;

impl Status {
    pub fn show(first: &str) -> Status {
        if in_boxes() && windows_allowed() {
            let text = first.to_owned();
            std::thread::spawn(move || status_thread(&text));
            // The window appears within moments; `set` before that only loses a line.
            for _ in 0..50 {
                if STATUS_TEXT.load(Ordering::Relaxed) != 0 {
                    break;
                }
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
        } else {
            session::send(Event::Status(first.to_owned()));
        }
        Status
    }

    pub fn set(&self, text: &str) {
        let label = STATUS_TEXT.load(Ordering::Relaxed);
        if label != 0 {
            let text = wide_str(text);
            // SAFETY: the string lives through the call; a window that went only fails the call.
            unsafe { SetWindowTextW(label, text.as_ptr()) };
        } else {
            session::send(Event::Status(text.to_owned()));
        }
    }

    pub fn close(&self) {
        let window = STATUS_WINDOW.load(Ordering::Relaxed);
        if window != 0 {
            // SAFETY: posting to a window that went only fails the call.
            unsafe { PostMessageW(window, WM_CLOSE, 0, 0) };
        }
        session::send(Event::StatusDone);
    }
}

fn status_thread(first: &str) {
    let class_name = wide_str("nv-rs-preview-status");
    let (title, label, text) = (wide_str(TITLE), wide_str("STATIC"), wide_str(first));
    // SAFETY: the strings live for this whole function, and the loop keeps the window alive.
    unsafe {
        // Drawn as Play.exe (DPI-unaware) drew it, Windows scaling it, though winit made the
        // process aware: this thread (only the status window's) stays unaware for its life.
        SetThreadDpiAwarenessContext(DPI_UNAWARE);
        let instance = GetModuleHandleW(std::ptr::null());
        let class = WndClassW {
            style: 0,
            procedure: status_procedure,
            class_extra: 0,
            window_extra: 0,
            instance,
            icon: std::ptr::null_mut(),
            cursor: std::ptr::null_mut(),
            // COLOR_BTNFACE.
            background: GetSysColorBrush(15),
            menu_name: std::ptr::null(),
            class_name: class_name.as_ptr(),
        };
        RegisterClassW(&class);
        let (width, height) = (460, 120);
        let (x, y) = (
            (GetSystemMetrics(0) - width) / 2,
            (GetSystemMetrics(1) - height) / 2,
        );
        // WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_VISIBLE.
        let window = CreateWindowExW(
            0,
            class_name.as_ptr(),
            title.as_ptr(),
            0x10c8_0000,
            x,
            y,
            width,
            height,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            instance,
            std::ptr::null_mut(),
        );
        // WS_CHILD | WS_VISIBLE.
        let text = CreateWindowExW(
            0,
            label.as_ptr(),
            text.as_ptr(),
            0x5000_0000,
            16,
            16,
            width - 48,
            height - 60,
            window,
            std::ptr::null_mut(),
            instance,
            std::ptr::null_mut(),
        );
        STATUS_WINDOW.store(window as isize, Ordering::Relaxed);
        STATUS_TEXT.store(text as isize, Ordering::Relaxed);
        let mut message = [0usize; 8];
        while GetMessageW(&mut message, std::ptr::null_mut(), 0, 0) > 0 {
            TranslateMessage(&message);
            DispatchMessageW(&message);
        }
    }
}

/// Whether the game's own window exists yet (its class in `d3d9real`).
pub fn game_window_is_there() -> bool {
    let class = wide_str("nv-rs-d3d9");
    // The class is registered with the A call; the name compares the same.
    // SAFETY: the string lives through the call.
    unsafe { !FindWindowW(class.as_ptr(), std::ptr::null()).is_null() }
}

/// `DPI_AWARENESS_CONTEXT_UNAWARE`.
pub const DPI_UNAWARE: isize = -1;

/// The primary monitor's size as a program without DPI awareness sees it, as the game does. The
/// launcher's window layer (winit) makes this process aware of each monitor's scale, so the
/// question is asked with this thread unaware for the moment, as Play.exe (unaware) asked it.
pub fn desktop_size() -> (i32, i32) {
    // SAFETY: no pointers.
    desktop_size_with(|| unsafe { (GetSystemMetrics(0), GetSystemMetrics(1)) })
}

/// [`desktop_size`] with `read` asking the size, called with this thread DPI-unaware; the
/// thread's own awareness is put back after.
pub fn desktop_size_with<T>(read: impl FnOnce() -> T) -> T {
    // SAFETY: no pointers; the thread's own context is put back.
    unsafe {
        let before = SetThreadDpiAwarenessContext(DPI_UNAWARE);
        let size = read();
        if before != 0 {
            SetThreadDpiAwarenessContext(before);
        }
        size
    }
}

const TITLE: &str = "nv-rs preview";
const MB_OK: u32 = 0;
const MB_YESNO: u32 = 4;
const MB_ICONERROR: u32 = 0x10;
const MB_ICONWARNING: u32 = 0x30;
const MB_ICONINFORMATION: u32 = 0x40;
const IDYES: i32 = 6;

fn message_box(text: &str, kind: u32) -> i32 {
    let (text, title) = (wide_str(text), wide_str(TITLE));
    // SAFETY: both strings live through the call; no owner window.
    unsafe { MessageBoxW(std::ptr::null_mut(), text.as_ptr(), title.as_ptr(), kind) }
}

/// Windows' folder picker (Play.exe's); `None` when the player cancels.
fn browse_folder(title: &str) -> Option<PathBuf> {
    let title = wide_str(title);
    let mut name = vec![0u16; 1024];
    let mut info = BrowseInfoW {
        owner: std::ptr::null_mut(),
        root: std::ptr::null_mut(),
        display_name: name.as_mut_ptr(),
        title: title.as_ptr(),
        // BIF_RETURNONLYFSDIRS | BIF_NEWDIALOGSTYLE.
        flags: 0x1 | 0x40,
        callback: std::ptr::null_mut(),
        param: 0,
        image: 0,
    };
    // SAFETY: the dialog wants COM on this thread; `info`'s strings live through the calls; the
    // list it hands back is freed here.
    unsafe {
        CoInitializeEx(std::ptr::null_mut(), 2);
        let list = SHBrowseForFolderW(&mut info);
        if list.is_null() {
            return None;
        }
        let mut path = vec![0u16; 1024];
        let ok = SHGetPathFromIDListW(list, path.as_mut_ptr());
        CoTaskMemFree(list);
        (ok != 0).then(|| PathBuf::from(from_wide(&path)))
    }
}

/// A message box's kind for a message of `tone` with `entries` (Play.exe's): one entry is OK with
/// the tone's icon; two are Yes and No.
pub fn box_kind(tone: Tone, entries: usize) -> u32 {
    let icon = match tone {
        Tone::Error => MB_ICONERROR,
        Tone::Warning => MB_ICONWARNING,
        Tone::Info | Tone::Question => MB_ICONINFORMATION,
    };
    icon | if entries >= 2 { MB_YESNO } else { MB_OK }
}

/// A question of Play's as Play.exe asked it, in a message box: the entry chosen. Two entries are
/// Yes and No (an entry other than Yes first is asked as its own question).
fn boxed_answer(tone: Tone, text: &str, entries: &[String]) -> usize {
    let kind = box_kind(tone, entries.len());
    match entries {
        [first, _, ..] => {
            let text = if first == "Yes" {
                text.to_string()
            } else {
                format!("{text}\n\n{first}?")
            };
            if message_box(&text, kind) == IDYES {
                0
            } else {
                1
            }
        }
        _ => {
            message_box(text, kind);
            0
        }
    }
}

/// Play's questions from now on answered in Windows' message boxes, its folder picker and the
/// small "getting ready" window (the launcher's window could not be drawn).
pub fn answer_in_boxes() {
    IN_BOXES.store(true, Ordering::Relaxed);
    session::connect(|event| {
        match event {
            Event::Message {
                tone,
                text,
                entries,
                reply,
            } => {
                let _ = reply.send(if windows_allowed() {
                    boxed_answer(tone, &text, &entries)
                } else {
                    0
                });
            }
            Event::Folder { text, reply } => {
                let _ = reply.send(windows_allowed().then(|| browse_folder(&text)).flatten());
            }
            _ => {}
        }
        true
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[link(name = "user32")]
    extern "system" {
        fn GetThreadDpiAwarenessContext() -> isize;
        fn AreDpiAwarenessContextsEqual(a: isize, b: isize) -> i32;
    }

    /// The desktop's size is read with the thread DPI-unaware (as the game, unaware, sees it),
    /// also once the thread is aware of each monitor's scale (as winit makes the launcher), and
    /// the thread is left as it was; on any display, scaled or not.
    #[test]
    fn the_desktop_size_is_read_unaware_and_the_thread_put_back() {
        const PER_MONITOR_V2: isize = -4;
        // SAFETY: no pointers; this test thread's context is set and put back.
        unsafe {
            let original = SetThreadDpiAwarenessContext(PER_MONITOR_V2);
            let during = desktop_size_with(|| {
                AreDpiAwarenessContextsEqual(GetThreadDpiAwarenessContext(), DPI_UNAWARE) != 0
            });
            assert!(during, "the size was not read DPI-unaware");
            let now = GetThreadDpiAwarenessContext();
            assert!(
                AreDpiAwarenessContextsEqual(now, PER_MONITOR_V2) != 0,
                "the thread's context was not put back"
            );
            let (w, h) = desktop_size();
            assert!(w > 0 && h > 0);
            if original != 0 {
                SetThreadDpiAwarenessContext(original);
            }
        }
    }

    /// The message boxes keep Play.exe's icons: errors, warnings, information; questions Yes/No.
    #[test]
    fn the_message_boxes_keep_play_exes_icons() {
        assert_eq!(box_kind(Tone::Error, 1), MB_ICONERROR | MB_OK);
        assert_eq!(box_kind(Tone::Warning, 1), MB_ICONWARNING | MB_OK);
        assert_eq!(box_kind(Tone::Info, 1), MB_ICONINFORMATION | MB_OK);
        assert_eq!(box_kind(Tone::Question, 2), MB_ICONINFORMATION | MB_YESNO);
    }
}
