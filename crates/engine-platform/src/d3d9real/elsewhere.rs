//! The window layer off Windows (`super`'s contract): none yet. Every real window asked for is
//! refused with a line naming what is missing; the game's modelled window stays as it is. (A
//! winit window comes next, Linux and macOS PR 3b; the host refuses a start off Windows until
//! then.)

use super::*;

/// The thread that would make the real windows: here a channel whose other end is gone, so a
/// request is refused at once (`super::window_created` prints that the real window could not be
/// made), after one line saying why.
pub(super) fn window_thread() -> &'static std::sync::Mutex<std::sync::mpsc::Sender<Request>> {
    static THREAD: std::sync::OnceLock<std::sync::Mutex<std::sync::mpsc::Sender<Request>>> =
        std::sync::OnceLock::new();
    THREAD.get_or_init(|| {
        eprintln!(
            "NV_D3D9={}: the game's window is not provided on this system yet (no window layer)",
            tag()
        );
        let (tx, _) = std::sync::mpsc::channel::<Request>();
        std::sync::Mutex::new(tx)
    })
}

/// No real window here, so nothing follows the title.
pub fn window_titled(e: &mut Engine, handle: u32, title: &[u8]) {
    let _ = (e, handle, title);
}

/// No real window here, so nothing follows the position.
pub fn window_moved(e: &mut Engine, handle: u32, rect: [i32; 4], flags: u32) {
    let _ = (e, handle, rect, flags);
}
