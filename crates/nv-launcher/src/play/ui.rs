//! What Play's steps say and ask, in the launcher's terminal window (`crate::session`): the
//! messages Play.exe showed in Windows' message boxes, its folder picker (the folder is typed at
//! the prompt or dropped onto the window) and its small "getting ready" window (the step's line on
//! the waiting screen). With `NV_RS_NO_WINDOWS=1` (scripted tests, which must not take the focus)
//! there is no window: every message is in `userdata\logs\launcher.log` either way, and the
//! answers are the ones Play.exe's message boxes gave without a window (Yes, no folder).
//!
//! Where the launcher's window cannot be drawn at all (no graphics device takes it), Play goes on
//! with Windows' own message boxes and folder picker, as Play.exe did ([`answer_in_boxes`]), so the
//! game can still be started.
use super::win::{from_wide, wide_str};
use crate::session::{self, Event};
use std::ffi::c_void;
use std::path::PathBuf;

#[link(name = "user32")]
extern "system" {
    fn GetSystemMetrics(index: i32) -> i32;
    fn FindWindowW(class: *const u16, title: *const u16) -> *mut c_void;
    fn MessageBoxW(owner: *mut c_void, text: *const u16, title: *const u16, kind: u32) -> i32;
    fn SetThreadDpiAwarenessContext(context: isize) -> isize;
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

/// Whether windows may be shown.
pub fn windows_allowed() -> bool {
    std::env::var_os("NV_RS_NO_WINDOWS").is_none_or(|v| v != "1")
}

/// The game cannot start, and why.
pub fn error(text: &str) {
    session::message(text, &["OK"]);
}

/// Something the player should know before the game starts.
pub fn warning(text: &str) {
    session::message(text, &["OK"]);
}

pub fn info(text: &str) {
    session::message(text, &["OK"]);
}

/// Yes or no; with no window, yes.
pub fn ask(text: &str) -> bool {
    session::message(text, &["Yes", "No"]) == 0
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

/// The waiting screen's line while the start gets ready.
pub struct Status;

impl Status {
    pub fn show(first: &str) -> Status {
        session::send(Event::Status(first.to_owned()));
        Status
    }

    pub fn set(&self, text: &str) {
        session::send(Event::Status(text.to_owned()));
    }

    pub fn close(&self) {
        session::send(Event::StatusDone);
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
    // SAFETY: no pointers; the thread's own context is put back.
    unsafe {
        let before = SetThreadDpiAwarenessContext(DPI_UNAWARE);
        let size = (GetSystemMetrics(0), GetSystemMetrics(1));
        if before != 0 {
            SetThreadDpiAwarenessContext(before);
        }
        size
    }
}

const TITLE: &str = "nv-rs preview";
const MB_OK: u32 = 0;
const MB_YESNO: u32 = 4;
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

/// A question of Play's as Play.exe asked it, in a message box: the entry chosen. One entry is
/// OK; two are Yes and No (the second entry's text, "Back" or "No", is No).
fn boxed_answer(text: &str, entries: &[String]) -> usize {
    match entries {
        [_] | [] => {
            message_box(text, MB_OK | MB_ICONINFORMATION);
            0
        }
        [first, ..] => {
            let text = if first == "Yes" {
                text.to_string()
            } else {
                format!("{text}\n\n{first}?")
            };
            if message_box(&text, MB_YESNO | MB_ICONINFORMATION) == IDYES {
                0
            } else {
                1
            }
        }
    }
}

/// Play's questions from now on answered in Windows' message boxes and folder picker (the
/// launcher's window could not be drawn).
pub fn answer_in_boxes() {
    session::connect(|event| {
        match event {
            Event::Message {
                text,
                entries,
                reply,
            } => {
                let _ = reply.send(boxed_answer(&text, &entries));
            }
            Event::Folder { text, reply } => {
                let _ = reply.send(browse_folder(&text));
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

    /// The desktop's size is the one a DPI-unaware program (the game) sees, also once this
    /// thread is aware of each monitor's scale (as winit makes the launcher), and the thread is
    /// left as it was.
    #[test]
    fn the_desktop_size_is_the_one_the_unaware_game_sees() {
        const PER_MONITOR_V2: isize = -4;
        // SAFETY: no pointers; this test thread's context is set and put back.
        unsafe {
            let original = SetThreadDpiAwarenessContext(PER_MONITOR_V2);
            let size = desktop_size();
            let now = GetThreadDpiAwarenessContext();
            assert!(
                AreDpiAwarenessContextsEqual(now, PER_MONITOR_V2) != 0,
                "the thread's context was not put back"
            );
            let unaware_before = SetThreadDpiAwarenessContext(DPI_UNAWARE);
            let unaware = (GetSystemMetrics(0), GetSystemMetrics(1));
            SetThreadDpiAwarenessContext(unaware_before);
            assert_eq!(size, unaware);
            if original != 0 {
                SetThreadDpiAwarenessContext(original);
            }
        }
    }
}
