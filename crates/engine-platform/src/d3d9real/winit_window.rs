//! The game's window on Linux (X11 and Wayland) and macOS: the window layer of `super` (ADR-0008)
//! from winit, as `win32.rs` is Windows'. A real window for each top-level window the game makes,
//! made and pumped on a thread of its own ([`window_thread`]), its keyboard and mouse into the
//! DirectInput state `super` keeps (through `input.rs`, with the codes and rules of the Win32
//! window's messages), and each window handed to the drawing side by its id
//! (`d3d9_wgpu::register_window`: its surface, client size, minimised, refresh rate).
//!
//! What differs from the Win32 window, and why:
//! - winit makes no child windows. The game's child window (the one it draws in) fills its parent's
//!   client area on Windows, so here it is its parent's window under an id of its own: the parent's
//!   client area takes the child's size (unless borderless), and the device draws in the parent.
//! - Borderless is winit's borderless full screen on the monitor the window is on, in its real
//!   pixels (Windows: a popup window over the primary monitor). A framed window is sized and placed
//!   in the system's scaled pixels, as the Win32 window thread, which is not DPI aware then, is.
//! - The cursor is held (locked, else confined to the window, and hidden) when the window gets the
//!   focus and on a click in it, and let go when it loses the focus: winit does not say whether the
//!   focus came from the keyboard or from a click on the frame, which Windows' window tells apart.
//! - The mouse's motion is winit's device motion (X11's raw motion, Wayland's relative pointer,
//!   macOS's mouse deltas), counted only while the window has the focus, as raw input reaches only
//!   the window in front on Windows. Fractions of a count (Wayland) carry over to the next motion.
//! - Key presses winit makes up for keys already held when the focus comes (X11) are not passed on:
//!   Windows sends none.
//! - The system's own keys stay the system's: a desktop that closes a window on Alt+F4 closes the
//!   game's (the run ends as with its close button), where the Win32 window keeps Alt for the game.
//! - macOS: winit's event loop must run on the program's main thread, which the port does not hand
//!   to the window layer yet (the game's own main thread runs there). [`run`] is the loop, on
//!   whatever thread calls it; until a host calls it on the main thread, macOS has no window, and
//!   says so.

use super::*;
use raw_window_handle::{
    DisplayHandle, HandleError, HasDisplayHandle, HasWindowHandle, WindowHandle,
};
use std::sync::atomic::{AtomicBool, AtomicIsize, Ordering};
use std::sync::mpsc::Receiver;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};
use winit::application::ApplicationHandler;
use winit::dpi::{LogicalPosition, LogicalSize};
use winit::event::{
    DeviceEvent, DeviceId, ElementState, MouseButton, MouseScrollDelta, StartCause, WindowEvent,
};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::PhysicalKey;
use winit::window::{CursorGrabMode, Fullscreen, Window, WindowId};

/// `WS_VISIBLE`: the game asks for its top-level window shown.
const WS_VISIBLE: u32 = 0x1000_0000;

/// How often the window thread looks for windows to make between the system's events (as the
/// Win32 window thread waits for messages at most 10 ms before it looks).
const SERVE_EVERY: Duration = Duration::from_millis(10);

/// A real window: winit's, and whether the system says it is hidden from view.
struct Shown {
    window: Window,
    occluded: AtomicBool,
}

impl HasWindowHandle for Shown {
    fn window_handle(&self) -> Result<WindowHandle<'_>, HandleError> {
        self.window.window_handle()
    }
}

impl HasDisplayHandle for Shown {
    fn display_handle(&self) -> Result<DisplayHandle<'_>, HandleError> {
        self.window.display_handle()
    }
}

impl d3d9_wgpu::GameWindow for Shown {
    fn client_size(&self) -> Option<(u32, u32)> {
        let size = self.window.inner_size();
        Some((size.width, size.height))
    }

    fn minimised(&self) -> bool {
        self.window.is_minimized() == Some(true) || self.occluded.load(Ordering::Relaxed)
    }

    fn refresh_millihertz(&self) -> Option<u32> {
        self.window.current_monitor()?.refresh_rate_millihertz()
    }
}

/// A real window by the id `super` keeps for it (never 0), with its top-level window's id (its
/// own for a top-level window).
struct Entry {
    shown: Arc<Shown>,
    top: isize,
}

fn windows() -> std::sync::MutexGuard<'static, HashMap<isize, Entry>> {
    static WINDOWS: OnceLock<Mutex<HashMap<isize, Entry>>> = OnceLock::new();
    WINDOWS
        .get_or_init(Default::default)
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// The next window's id.
fn next_id() -> isize {
    static NEXT: AtomicIsize = AtomicIsize::new(1);
    NEXT.fetch_add(1, Ordering::Relaxed)
}

/// `id`'s window as made known to `super` and to the drawing side.
fn keep(id: isize, shown: &Arc<Shown>, top: isize) {
    windows().insert(
        id,
        Entry {
            shown: Arc::clone(shown),
            top,
        },
    );
    d3d9_wgpu::register_window(id, Arc::clone(shown) as Arc<dyn d3d9_wgpu::GameWindow>);
}

/// The thread that makes every real window and pumps the system's events ([`run`]).
pub(super) fn window_thread() -> &'static std::sync::Mutex<std::sync::mpsc::Sender<Request>> {
    static THREAD: OnceLock<Mutex<std::sync::mpsc::Sender<Request>>> = OnceLock::new();
    THREAD.get_or_init(|| {
        let (tx, rx) = std::sync::mpsc::channel::<Request>();
        if cfg!(target_vendor = "apple") {
            // The receiver goes: each request is refused (`super::window_created` says so).
            eprintln!(
                "NV_D3D9={}: on macOS the game's window needs the program's main thread, which is not handed to the window layer yet: no window",
                tag()
            );
            drop(rx);
        } else if let Err(e) = std::thread::Builder::new()
            .name("nv-d3d9-window".into())
            .spawn(move || run(rx))
        {
            eprintln!("NV_D3D9={}: the window thread could not start: {e}", tag());
        }
        Mutex::new(tx)
    })
}

/// The window layer's loop on the calling thread: the windows asked for in `requests` are made,
/// the system's events pumped, until the run ends. Returns at once when there is no window system
/// to show a window in (no X server or Wayland compositor), with a line saying so; the requests
/// are refused then.
pub(super) fn run(requests: Receiver<Request>) {
    let mut builder = EventLoop::builder();
    // The game's window has a thread of its own, not the program's main thread (X11 and Wayland
    // allow it; macOS does not, [`window_thread`]).
    #[cfg(any(
        target_os = "linux",
        target_os = "dragonfly",
        target_os = "freebsd",
        target_os = "netbsd",
        target_os = "openbsd"
    ))]
    winit::platform::x11::EventLoopBuilderExtX11::with_any_thread(&mut builder, true);
    let event_loop = match builder.build() {
        Ok(l) => l,
        Err(e) => {
            eprintln!(
                "NV_D3D9={}: no window system to show the game's window in ({e}): no window",
                tag()
            );
            return;
        }
    };
    let mut layer = Layer {
        requests,
        tops: HashMap::new(),
        focused: false,
        held: None,
        rest: [0.0; 3],
    };
    if let Err(e) = event_loop.run_app(&mut layer) {
        eprintln!("NV_D3D9={}: the window's event loop ended: {e}", tag());
    }
}

/// The loop's state on the window thread.
struct Layer {
    requests: Receiver<Request>,
    /// The top-level windows by winit's id: their ids.
    tops: HashMap<WindowId, isize>,
    /// Whether a window of the game's has the focus.
    focused: bool,
    /// The window holding the cursor, if one does.
    held: Option<isize>,
    /// The motion's fractions not given yet (x, y, the wheel in its 120ths).
    rest: [f64; 3],
}

impl Layer {
    /// Makes the windows asked for since the last look.
    fn serve(&mut self, event_loop: &ActiveEventLoop) {
        while let Ok(request) = self.requests.try_recv() {
            let id = self.make(event_loop, &request).unwrap_or(0);
            let _ = request.reply.send(id);
        }
        event_loop.set_control_flow(ControlFlow::WaitUntil(Instant::now() + SERVE_EVERY));
    }

    /// The real window of `request`: its id, `None` when it could not be made.
    fn make(&mut self, event_loop: &ActiveEventLoop, request: &Request) -> Option<isize> {
        let [left, top, right, bottom] = request.rect;
        let size = (
            right.wrapping_sub(left).max(1) as u32,
            bottom.wrapping_sub(top).max(1) as u32,
        );
        if request.parent != 0 {
            // The game's child window: its parent's window under an id of its own; the parent's
            // client area its size (the borderless window keeps the monitor's).
            let (shown, top) = {
                let all = windows();
                let parent = all.get(&request.parent)?;
                (Arc::clone(&parent.shown), parent.top)
            };
            if !borderless() {
                let _ = shown
                    .window
                    .request_inner_size(LogicalSize::new(size.0, size.1));
            }
            let id = next_id();
            keep(id, &shown, top);
            return Some(id);
        }
        let borderless = borderless();
        let mut attributes = Window::default_attributes()
            .with_title(latin1(&request.title))
            .with_visible(false)
            .with_inner_size(LogicalSize::new(size.0, size.1))
            .with_position(LogicalPosition::new(left, top))
            .with_active(request.show != Showing::Background);
        if borderless {
            attributes = attributes
                .with_decorations(false)
                .with_fullscreen(Some(Fullscreen::Borderless(None)));
        }
        let window = match event_loop.create_window(attributes) {
            Ok(w) => w,
            Err(e) => {
                eprintln!("NV_D3D9={}: winit could not make the window: {e}", tag());
                return None;
            }
        };
        if request.style & WS_VISIBLE != 0 && request.show != Showing::Never {
            window.set_visible(true);
        }
        let winit_id = window.id();
        let shown = Arc::new(Shown {
            window,
            occluded: AtomicBool::new(false),
        });
        let id = next_id();
        keep(id, &shown, id);
        self.tops.insert(winit_id, id);
        Some(id)
    }

    /// Holds the cursor in the window `id` and hides it there, as Windows does for a program whose
    /// DirectInput mouse is exclusive and in the foreground: locked where the system can, else kept
    /// inside the window.
    fn hold(&mut self, id: isize) {
        let Some(shown) = windows().get(&id).map(|e| Arc::clone(&e.shown)) else {
            return;
        };
        let w = &shown.window;
        if w.set_cursor_grab(CursorGrabMode::Locked).is_err() {
            let _ = w.set_cursor_grab(CursorGrabMode::Confined);
        }
        w.set_cursor_visible(false);
        self.held = Some(id);
    }

    /// Lets the cursor go (free, shown).
    fn let_go(&mut self) {
        let Some(id) = self.held.take() else { return };
        if let Some(shown) = windows().get(&id).map(|e| Arc::clone(&e.shown)) {
            let _ = shown.window.set_cursor_grab(CursorGrabMode::None);
            shown.window.set_cursor_visible(true);
        }
    }
}

impl ApplicationHandler for Layer {
    fn new_events(&mut self, event_loop: &ActiveEventLoop, _cause: StartCause) {
        self.serve(event_loop);
    }

    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        self.serve(event_loop);
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        self.serve(event_loop);
    }

    fn window_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        window: WindowId,
        event: WindowEvent,
    ) {
        let Some(&id) = self.tops.get(&window) else {
            return;
        };
        match event {
            // Closing the game's window ends the run (the player's way out), as on Windows.
            WindowEvent::CloseRequested => {
                eprintln!(
                    "NV_D3D9={}: the game's window was closed: the run ends",
                    tag()
                );
                crate::run_exit_hooks();
                crate::end_process(0);
            }
            // Activated, its display's scale changed, shown again: wgpu asks the window for a
            // frame at once rather than at the end of a rest (`d3d9_wgpu::window_changed`).
            WindowEvent::Focused(true) => {
                self.focused = true;
                d3d9_wgpu::window_changed();
                self.hold(id);
            }
            // Losing the focus lets go of everything this window held.
            WindowEvent::Focused(false) => {
                self.focused = false;
                input::focus_lost();
                self.let_go();
            }
            WindowEvent::ScaleFactorChanged { .. } => d3d9_wgpu::window_changed(),
            WindowEvent::Occluded(hidden) => {
                if let Some(e) = windows().get(&id) {
                    e.shown.occluded.store(hidden, Ordering::Relaxed);
                }
                if !hidden {
                    d3d9_wgpu::window_changed();
                }
            }
            WindowEvent::KeyboardInput {
                event,
                is_synthetic,
                ..
            } => {
                if is_synthetic {
                    return;
                }
                let PhysicalKey::Code(code) = event.physical_key else {
                    return;
                };
                if let Some(dik) = input::dik_of_code(&format!("{code:?}")) {
                    input::key(dik, event.state == ElementState::Pressed, event.repeat);
                }
            }
            WindowEvent::MouseInput { state, button, .. } => {
                let down = state == ElementState::Pressed;
                let index = match button {
                    MouseButton::Left => 0,
                    MouseButton::Right => 1,
                    MouseButton::Middle => 2,
                    MouseButton::Back => 3,
                    MouseButton::Forward => 4,
                    MouseButton::Other(_) => return,
                };
                if down && index < 3 {
                    self.hold(id);
                }
                input::button(index, down);
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let notches = match delta {
                    MouseScrollDelta::LineDelta(_, y) => f64::from(y) * input::WHEEL_NOTCH,
                    // A touchpad's smooth scrolling in pixels: taken as wheel units.
                    MouseScrollDelta::PixelDelta(p) => p.y,
                };
                let wheel = input::whole(&mut self.rest[2], notches);
                if wheel != 0 {
                    input::moved(0, 0, wheel);
                }
            }
            _ => {}
        }
    }

    fn device_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        _device: DeviceId,
        event: DeviceEvent,
    ) {
        if let DeviceEvent::MouseMotion { delta } = event {
            if !self.focused {
                return;
            }
            let dx = input::whole(&mut self.rest[0], delta.0);
            let dy = input::whole(&mut self.rest[1], delta.1);
            if dx != 0 || dy != 0 {
                input::moved(dx, dy, 0);
            }
        }
    }
}

/// The game's title bytes (its code page's; the game's titles are ASCII) as text.
fn latin1(bytes: &[u8]) -> String {
    bytes.iter().map(|&b| char::from(b)).collect()
}

/// The real window of the game's window `handle` takes the title the game gave its record
/// (`SetWindowTextA`).
pub fn window_titled(e: &mut Engine, handle: u32, title: &[u8]) {
    let Some(window) = real_window(e, handle) else {
        return;
    };
    let shown = windows().get(&window).map(|e| Arc::clone(&e.shown));
    if let Some(shown) = shown {
        shown.window.set_title(&latin1(title));
    }
}

/// `SetWindowPos` moved or sized the game's window `handle` (`SWP_NOSIZE` 1, `SWP_NOMOVE` 2 in
/// `flags`): the real window follows (in the system's scaled pixels, as the framed Win32 window).
/// The borderless window keeps the monitor; the game's child window, which is its parent's window
/// here, gives the parent's client area its size.
pub fn window_moved(e: &mut Engine, handle: u32, rect: [i32; 4], flags: u32) {
    let Some(window) = real_window(e, handle) else {
        return;
    };
    if borderless() {
        return;
    }
    let Some((shown, top)) = windows()
        .get(&window)
        .map(|e| (Arc::clone(&e.shown), e.top))
    else {
        return;
    };
    let w = &shown.window;
    let scale = w.scale_factor();
    if top != window {
        if flags & 1 == 0 {
            let [left, top, right, bottom] = rect;
            let size = (
                right.wrapping_sub(left).max(1) as u32,
                bottom.wrapping_sub(top).max(1) as u32,
            );
            let _ = w.request_inner_size(LogicalSize::new(size.0, size.1));
        }
        return;
    }
    if flags & 2 == 0 {
        w.set_outer_position(LogicalPosition::new(rect[0], rect[1]));
    }
    if flags & 1 == 0 {
        let outer = w.outer_size().to_logical::<u32>(scale);
        let inner = w.inner_size().to_logical::<u32>(scale);
        let (width, height) = input::inner_for_outer(
            rect,
            (outer.width, outer.height),
            (inner.width, inner.height),
        );
        let _ = w.request_inner_size(LogicalSize::new(width, height));
    }
}
