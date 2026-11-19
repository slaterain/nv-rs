//! What Play's steps say and ask, in the launcher's terminal window (`crate::session`): the
//! messages Play.exe showed in Windows' message boxes, its folder picker (the folder is typed at
//! the prompt or dropped onto the window) and its small "getting ready" window (the step's line on
//! the waiting screen). With `NV_RS_NO_WINDOWS=1` (scripted tests, which must not take the focus)
//! there is no window: every message is in `userdata\logs\launcher.log` either way, and the
//! answers are the ones Play.exe's message boxes gave without a window (Yes, no folder).
use super::win::wide_str;
use crate::session;
use std::ffi::c_void;
use std::path::PathBuf;

#[link(name = "user32")]
extern "system" {
    fn GetSystemMetrics(index: i32) -> i32;
    fn FindWindowW(class: *const u16, title: *const u16) -> *mut c_void;
}
#[link(name = "shell32")]
extern "system" {
    fn ShellExecuteW(
        owner: *mut c_void,
        verb: *const u16,
        file: *const u16,
        parameters: *const u16,
        folder: *const u16,
        show: i32,
    ) -> isize;
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
        session::send(session::Event::Status(first.to_owned()));
        Status
    }

    pub fn set(&self, text: &str) {
        session::send(session::Event::Status(text.to_owned()));
    }

    pub fn close(&self) {
        session::send(session::Event::StatusDone);
    }
}

/// Whether the game's own window exists yet (its class in `d3d9real`).
pub fn game_window_is_there() -> bool {
    let class = wide_str("nv-rs-d3d9");
    // The class is registered with the A call; the name compares the same.
    // SAFETY: the string lives through the call.
    unsafe { !FindWindowW(class.as_ptr(), std::ptr::null()).is_null() }
}

/// The primary monitor's size as a program without DPI awareness sees it, as the game does.
pub fn desktop_size() -> (i32, i32) {
    // SAFETY: no pointers.
    unsafe { (GetSystemMetrics(0), GetSystemMetrics(1)) }
}
