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
//!   in the system's scaled pixels, as the Win32 window thread, which is not DPI aware then, is; its
//!   client area is the rectangle the game asks for less the frame the game added for it
//!   (`input::inner_for_game_rect`), and its frame has the buttons and resizing the game's style
//!   gives it on Windows (`input::frame_of_style`: none of either for the game's window). On a
//!   Wayland desktop that draws no frames for programs (GNOME) the framed window has a plain frame
//!   with no title: winit's own title bar is left out (it brings a font and a rasteriser).
//! - The cursor is held (locked, else confined to the window, and hidden) when the window gets the
//!   focus and on a click in it, and let go when it loses the focus: winit does not say whether the
//!   focus came from the keyboard or from a click on the frame, which Windows' window tells apart.
//! - The mouse's motion is winit's device motion (X11's raw motion, Wayland's relative pointer,
//!   macOS's mouse deltas), counted only while the window has the focus, as raw input reaches only
//!   the window in front on Windows. Fractions of a count (Wayland) carry over to the next motion.
//!   Not done: Windows drops the raw input of a pointer that gives absolute positions (a tablet, a
//!   virtual machine's mouse); winit does not say which device is such a one, so on X11 its
//!   positions would be taken as motion.
//! - Pixel scrolling (a touchpad) does not reach the game: no stated ratio to Windows' wheel units
//!   was found (`input::wheel`). A mouse wheel does, 120 a notch.
//! - Key presses winit makes up for keys already held when the focus comes (X11) are not passed on:
//!   Windows sends none.
//! - The system's own keys stay the system's: a desktop that closes a window on Alt+F4 closes the
//!   game's (the run ends as with its close button), where the Win32 window keeps Alt for the game.
//! - Minimised or hidden: X11 says so (the drawing side then asks no frame of the window); winit's
//!   Wayland says neither, so there the drawing side keeps asking a hidden window for frames.
//! - macOS: winit's event loop must run on the program's main thread, which the port does not hand
//!   to the window layer yet (the game's own main thread runs there). [`run`] is the loop, on
//!   whatever thread calls it; until a host calls it on the main thread, macOS has no window, and
//!   the run ends with a line saying so.
//! - Where no window can be shown the run ends with a line naming why (a run without its window
//!   would go on drawing nothing): no X server or Wayland compositor, a missing library, winit's
//!   loop stopping. The libraries a Linux player needs, all loaded at run time: on X11 libX11.so.6,
//!   libX11-xcb.so.1, libXcursor.so.1, libXi.so.6, libxkbcommon.so.0 and libxkbcommon-x11.so.0; on
//!   Wayland libwayland-client.so.0 and libxkbcommon.so.0; and libvulkan.so.1 with a Vulkan driver
//!   for the drawing (Debian and Ubuntu: libx11-6 libx11-xcb1 libxcursor1 libxi6 libxkbcommon0
//!   libxkbcommon-x11-0 libwayland-client0 libvulkan1).
//! - The contract with `super` and the drawing side (`super`'s `const _` check): the [`RealWindow`]
//!   number, the game's own `CreateWindowExA` and `SetWindowPos` arguments (style, rectangle,
//!   flags: the exe's, by design) and portable calls cross; nothing of winit's.

use super::*;
use raw_window_handle::{
    DisplayHandle, HandleError, HasDisplayHandle, HasWindowHandle, WindowHandle,
};
use std::sync::atomic::{AtomicBool, AtomicIsize, AtomicU32, Ordering};
use std::sync::mpsc::Receiver;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};
use winit::application::ApplicationHandler;
use winit::dpi::{LogicalPosition, LogicalSize, PhysicalSize};
use winit::event::{
    DeviceEvent, DeviceId, ElementState, MouseButton, MouseScrollDelta, StartCause, WindowEvent,
};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::PhysicalKey;
use winit::window::{CursorGrabMode, Fullscreen, Window, WindowButtons, WindowId};

/// How often the window thread looks for windows to make between the system's events (as the
/// Win32 window thread waits for messages at most 10 ms before it looks).
const SERVE_EVERY: Duration = Duration::from_millis(10);

/// A real window: winit's, and what the drawing side reads of it, kept here from the window's own
/// events and looked up on the window thread, so that the drawing side's reads each frame ask
/// nothing of the window system (on X11 each would be a round trip to the server).
struct Shown {
    window: Window,
    /// The client area's size in real pixels.
    size: [AtomicU32; 2],
    /// Minimised (looked up on the window thread every [`SERVE_EVERY`] at most).
    minimised: AtomicBool,
    /// Hidden from view, as the system says (X11).
    occluded: AtomicBool,
}

impl Shown {
    fn new(window: Window) -> Shown {
        let size = window.inner_size();
        Shown {
            window,
            size: [AtomicU32::new(size.width), AtomicU32::new(size.height)],
            minimised: AtomicBool::new(false),
            occluded: AtomicBool::new(false),
        }
    }

    fn resized(&self, size: PhysicalSize<u32>) {
        self.size[0].store(size.width, Ordering::Relaxed);
        self.size[1].store(size.height, Ordering::Relaxed);
    }
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
        Some((
            self.size[0].load(Ordering::Relaxed),
            self.size[1].load(Ordering::Relaxed),
        ))
    }

    fn minimised(&self) -> bool {
        self.minimised.load(Ordering::Relaxed) || self.occluded.load(Ordering::Relaxed)
    }

    fn refresh_millihertz(&self) -> Option<u32> {
        self.window.current_monitor()?.refresh_rate_millihertz()
    }
}

/// A real window by the id `super` keeps for it (never 0), with its top-level window's id (its
/// own for a top-level window).
struct Entry {
    shown: Arc<Shown>,
    top: RealWindow,
}

fn windows() -> std::sync::MutexGuard<'static, HashMap<RealWindow, Entry>> {
    static WINDOWS: OnceLock<Mutex<HashMap<RealWindow, Entry>>> = OnceLock::new();
    WINDOWS
        .get_or_init(Default::default)
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// The next window's id.
fn next_id() -> RealWindow {
    static NEXT: AtomicIsize = AtomicIsize::new(1);
    NEXT.fetch_add(1, Ordering::Relaxed)
}

/// `id`'s window as made known to `super` and to the drawing side.
fn keep(id: RealWindow, shown: &Arc<Shown>, top: RealWindow) {
    windows().insert(
        id,
        Entry {
            shown: Arc::clone(shown),
            top,
        },
    );
    d3d9_wgpu::register_window(id, Arc::clone(shown) as Arc<dyn d3d9_wgpu::GameWindow>);
}

/// No window can be shown: the run ends with a line saying why (a run without its window would go
/// on drawing nothing).
fn no_window(why: &str) -> ! {
    eprintln!("NV_D3D9={}: {why}: no window, the run ends", tag());
    crate::end_process(1)
}

/// The thread that makes every real window and pumps the system's events ([`run`]).
pub(super) fn window_thread() -> &'static std::sync::Mutex<std::sync::mpsc::Sender<Request>> {
    static THREAD: OnceLock<Mutex<std::sync::mpsc::Sender<Request>>> = OnceLock::new();
    THREAD.get_or_init(|| {
        let (tx, rx) = std::sync::mpsc::channel::<Request>();
        if cfg!(target_vendor = "apple") {
            no_window(
                "on macOS the game's window needs the program's main thread, which is not handed \
                 to the window layer yet",
            );
        }
        if let Err(e) = std::thread::Builder::new()
            .name("nv-d3d9-window".into())
            .spawn(move || run(rx))
        {
            no_window(&format!("the window thread could not start ({e})"));
        }
        Mutex::new(tx)
    })
}

/// The keyboard library winit needs and cannot load, if one is missing: libxkbcommon always, and
/// libxkbcommon-x11 where winit takes X11 (no Wayland compositor named). winit would stop its
/// thread with a panic instead.
#[cfg(not(target_vendor = "apple"))]
fn missing_keyboard_library() -> Option<&'static str> {
    if xkbcommon_dl::xkbcommon_option().is_none() {
        return Some("libxkbcommon.so.0");
    }
    let wayland = std::env::var_os("WAYLAND_DISPLAY").is_some()
        || std::env::var_os("WAYLAND_SOCKET").is_some();
    if !wayland && xkbcommon_dl::x11::xkbcommon_x11_option().is_none() {
        return Some("libxkbcommon-x11.so.0");
    }
    None
}

/// The window layer's loop on the calling thread: the windows asked for in `requests` are made,
/// the system's events pumped, until the run ends. Where no window can be shown (no X server or
/// Wayland compositor, a library missing, the loop stopping) the run ends with a line naming why
/// ([`no_window`]), also when winit stops with a panic.
pub(super) fn run(requests: Receiver<Request>) {
    #[cfg(not(target_vendor = "apple"))]
    if let Some(library) = missing_keyboard_library() {
        no_window(&format!(
            "the keyboard library {library} is not installed (a Linux player needs libxkbcommon, \
             and libxkbcommon-x11 on X11)"
        ));
    }
    match input::guarded(|| run_loop(requests)) {
        Ok(()) => no_window("the window's event loop ended"),
        Err(message) => no_window(&format!("the window layer stopped ({message})")),
    }
}

fn run_loop(requests: Receiver<Request>) {
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
        Err(e) => no_window(&format!(
            "no window system to show the game's window in ({e})"
        )),
    };
    let mut layer = Layer {
        requests,
        tops: HashMap::new(),
        focused: false,
        alt: false,
        held: None,
        rest: [0.0; 3],
    };
    if let Err(e) = event_loop.run_app(&mut layer) {
        no_window(&format!("the window's event loop ended ({e})"));
    }
}

/// The loop's state on the window thread.
struct Layer {
    requests: Receiver<Request>,
    /// The top-level windows by winit's id: their ids.
    tops: HashMap<WindowId, RealWindow>,
    /// Whether a window of the game's has the focus.
    focused: bool,
    /// Whether Alt is held.
    alt: bool,
    /// The window holding the cursor, if one does.
    held: Option<RealWindow>,
    /// The motion's fractions not given yet (x, y, the wheel in its 120ths).
    rest: [f64; 3],
}

impl Layer {
    /// Makes the windows asked for since the last look, and looks up whether each top-level window
    /// is minimised (for the drawing side, [`Shown::minimised`]).
    fn serve(&mut self, event_loop: &ActiveEventLoop) {
        while let Ok(request) = self.requests.try_recv() {
            let id = self.make(event_loop, &request).unwrap_or(0);
            let _ = request.reply.send(id);
        }
        let tops: Vec<Arc<Shown>> = {
            let all = windows();
            self.tops
                .values()
                .filter_map(|id| all.get(id).map(|e| Arc::clone(&e.shown)))
                .collect()
        };
        for shown in tops {
            let minimised = shown.window.is_minimized() == Some(true);
            shown.minimised.store(minimised, Ordering::Relaxed);
        }
        event_loop.set_control_flow(ControlFlow::WaitUntil(Instant::now() + SERVE_EVERY));
    }

    /// The real window of `request`: its id, `None` when it could not be made.
    fn make(&mut self, event_loop: &ActiveEventLoop, request: &Request) -> Option<RealWindow> {
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
        } else {
            let frame = input::frame_of_style(request.style);
            let mut buttons = WindowButtons::empty();
            buttons.set(WindowButtons::CLOSE, frame.close);
            buttons.set(WindowButtons::MINIMIZE, frame.minimise);
            buttons.set(WindowButtons::MAXIMIZE, frame.maximise);
            attributes = attributes
                .with_resizable(frame.resizable)
                .with_enabled_buttons(buttons);
        }
        let window = match event_loop.create_window(attributes) {
            Ok(w) => w,
            Err(e) => {
                eprintln!("NV_D3D9={}: winit could not make the window: {e}", tag());
                return None;
            }
        };
        if request.visible() && request.show != Showing::Never {
            window.set_visible(true);
        }
        let winit_id = window.id();
        let shown = Arc::new(Shown::new(window));
        let id = next_id();
        keep(id, &shown, id);
        self.tops.insert(winit_id, id);
        Some(id)
    }

    /// Holds the cursor in the window `id` and hides it there, as Windows does for a program whose
    /// DirectInput mouse is exclusive and in the foreground: locked where the system can, else kept
    /// inside the window.
    fn hold(&mut self, id: RealWindow) {
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

/// winit's mouse button as `input.rs` names it.
fn button_of(button: MouseButton) -> input::Button {
    match button {
        MouseButton::Left => input::Button::Left,
        MouseButton::Right => input::Button::Right,
        MouseButton::Middle => input::Button::Middle,
        MouseButton::Back => input::Button::Back,
        MouseButton::Forward => input::Button::Forward,
        MouseButton::Other(_) => input::Button::Other,
    }
}

/// winit's wheel motion as `input.rs` takes it.
fn scroll_of(delta: MouseScrollDelta) -> input::Scroll {
    match delta {
        MouseScrollDelta::LineDelta(_, y) => input::Scroll::Lines(y),
        MouseScrollDelta::PixelDelta(p) => input::Scroll::Pixels(p.y),
    }
}

/// The name `input::dik_of_code` looks a key up by: winit's `KeyCode` name (the UI Events `code`
/// value), `None` for a key winit has no code for.
fn key_name(key: PhysicalKey) -> Option<String> {
    match key {
        PhysicalKey::Code(code) => Some(format!("{code:?}")),
        PhysicalKey::Unidentified(_) => None,
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
            WindowEvent::CloseRequested => window_closed(),
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
            WindowEvent::Resized(size) => {
                if let Some(e) = windows().get(&id) {
                    e.shown.resized(size);
                }
            }
            WindowEvent::Occluded(hidden) => {
                if let Some(e) = windows().get(&id) {
                    e.shown.occluded.store(hidden, Ordering::Relaxed);
                }
                if !hidden {
                    d3d9_wgpu::window_changed();
                }
            }
            WindowEvent::ModifiersChanged(modifiers) => self.alt = modifiers.state().alt_key(),
            WindowEvent::KeyboardInput {
                event,
                is_synthetic,
                ..
            } => {
                let name = key_name(event.physical_key);
                if let Some(dik) = input::key_event(is_synthetic, name.as_deref()) {
                    input::key(
                        dik,
                        event.state == ElementState::Pressed,
                        event.repeat,
                        self.alt,
                    );
                }
            }
            WindowEvent::MouseInput { state, button, .. } => {
                let down = state == ElementState::Pressed;
                let button = button_of(button);
                let Some(index) = input::button_index(button) else {
                    return;
                };
                if down && input::holds_cursor(button) {
                    self.hold(id);
                }
                input::button(index, down);
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let wheel = input::wheel(&mut self.rest[2], scroll_of(delta));
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
            let mut rest = [self.rest[0], self.rest[1]];
            if let Some((dx, dy)) = input::motion(self.focused, &mut rest, delta) {
                input::moved(dx, dy, 0);
            }
            self.rest[0] = rest[0];
            self.rest[1] = rest[1];
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

/// `SetWindowPos` moved or sized the game's window `handle` (`input::NO_SIZE`, `input::NO_MOVE` in
/// `flags`): the real window follows (in the system's scaled pixels, as the framed Win32 window),
/// its client area the rectangle less the frame the game added for it
/// (`crate::window::game_frame`), never this desktop's own frame. The borderless window keeps the
/// monitor; the game's child window, which is its parent's window here, gives the parent's client
/// area its size.
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
    if top != window {
        if flags & input::NO_SIZE == 0 {
            let [left, top, right, bottom] = rect;
            let size = (
                right.wrapping_sub(left).max(1) as u32,
                bottom.wrapping_sub(top).max(1) as u32,
            );
            let _ = w.request_inner_size(LogicalSize::new(size.0, size.1));
        }
        return;
    }
    if flags & input::NO_MOVE == 0 {
        w.set_outer_position(LogicalPosition::new(rect[0], rect[1]));
    }
    if flags & input::NO_SIZE == 0 {
        let border = crate::window::game_frame(e, handle).unwrap_or([0; 4]);
        let (width, height) = input::inner_for_game_rect(rect, border);
        let _ = w.request_inner_size(LogicalSize::new(width, height));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use winit::keyboard::KeyCode;

    /// winit's own key codes, named as `input::dik_of_code` looks them up, give the codes
    /// Windows' key messages give (a winit upgrade that renamed one would show here).
    #[test]
    fn winits_key_codes_give_the_codes_windows_gives() {
        for (code, dik) in [
            (KeyCode::KeyW, 0x11),
            (KeyCode::Escape, 0x01),
            (KeyCode::ArrowUp, 0xc8),
            (KeyCode::NumpadEnter, 0x9c),
            (KeyCode::Pause, 0xc5),
            (KeyCode::NumLock, 0x45),
            (KeyCode::ControlRight, 0x9d),
            (KeyCode::AltRight, 0xb8),
            (KeyCode::F11, 0x57),
            (KeyCode::F12, 0x58),
            (KeyCode::Backquote, 0x29),
            (KeyCode::SuperLeft, 0xdb),
            (KeyCode::IntlBackslash, 0x56),
            (KeyCode::MediaSelect, 0xed),
        ] {
            let name = key_name(PhysicalKey::Code(code));
            assert_eq!(
                input::key_event(false, name.as_deref()),
                Some(dik),
                "{code:?}"
            );
        }
        let unknown = PhysicalKey::Unidentified(winit::keyboard::NativeKeyCode::Unidentified);
        assert_eq!(key_name(unknown), None);
    }

    /// winit's buttons in DirectInput's order (Back is X1, Forward X2) and its wheel motion.
    #[test]
    fn winits_buttons_and_wheel_map_as_windows_gives_them() {
        let index = |b| input::button_index(button_of(b));
        assert_eq!(index(MouseButton::Left), Some(0));
        assert_eq!(index(MouseButton::Right), Some(1));
        assert_eq!(index(MouseButton::Middle), Some(2));
        assert_eq!(index(MouseButton::Back), Some(3));
        assert_eq!(index(MouseButton::Forward), Some(4));
        assert_eq!(index(MouseButton::Other(9)), None);
        assert_eq!(
            scroll_of(MouseScrollDelta::LineDelta(0.0, -1.0)),
            input::Scroll::Lines(-1.0)
        );
        assert_eq!(
            scroll_of(MouseScrollDelta::PixelDelta(
                winit::dpi::PhysicalPosition::new(0.0, 12.5)
            )),
            input::Scroll::Pixels(12.5)
        );
    }
}
