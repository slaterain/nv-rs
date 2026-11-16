//! The window layer off Windows (`super`'s contract): none yet. A real window asked for stops the
//! run with a line naming what is missing (a run without its window would go on drawing nothing).
//! Unreachable today: the host refuses a start off Windows before the game makes its window, and
//! the callers of `super` are Windows-only until the winit window comes (Linux and macOS PR 3b).

use super::*;

/// The thread that would make the real windows: none, so asking for it stops the run.
pub(super) fn window_thread() -> &'static std::sync::Mutex<std::sync::mpsc::Sender<Request>> {
    eprintln!(
        "NV_D3D9={}: the game's window is not provided on this system yet (no window layer): the run ends",
        tag()
    );
    crate::end_process(1)
}

/// No real window here (none can be made: [`window_thread`]), so nothing follows the title.
pub fn window_titled(e: &mut Engine, handle: u32, title: &[u8]) {
    let _ = (e, handle, title);
}

/// No real window here (none can be made: [`window_thread`]), so nothing follows the position.
pub fn window_moved(e: &mut Engine, handle: u32, rect: [i32; 4], flags: u32) {
    let _ = (e, handle, rect, flags);
}
