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
//! - Borderless is winit's borderless full screen on the monitor the machine table describes (the
//!   primary one, else the first: `crate::machine_table::display::described`), in its real
//!   pixels (Windows: a popup window over the primary monitor). A framed window is sized and placed
//!   in the system's scaled pixels, as the Win32 window thread, which is not DPI aware then, is; its
//!   client area is the rectangle the game asks for less the frame the game added for it
//!   (`input::inner_for_game_rect`). Of what the game's style gives its frame on Windows
//!   (`input::frame_of_style`: for the game's window no resizing and no buttons) only the
//!   resizing reaches a Linux window: winit 0.30 cannot take buttons off on X11 or Wayland. So on
//!   X11 the framed window cannot be resized or maximised, and the window manager still draws its
//!   close and minimise buttons. On a Wayland desktop that draws no frames for programs (GNOME)
//!   the window has the plain frame winit draws itself (no title: winit's own title bar is left
//!   out, it brings a font and a rasteriser), with close, and minimise and maximise where the
//!   compositor offers them; a maximise the compositor grants fills the monitor and stretches the
//!   game's picture to it (the Windows window has no such button). Closing ends the run as on
//!   Windows. While the window cannot be resized, its smallest and largest sizes are set to each
//!   size the game asks for, so a compositor that holds a window to them keeps the game's size.
//!   Wayland sends no resize event for a size the program asked for (it applies it at once), so
//!   that size is kept from the request itself ([`SizeKept`]).
//! - The cursor is held (locked, else confined to the window, and hidden) when the window gets the
//!   focus and on a click in it, and let go when it loses the focus: winit does not say whether the
//!   focus came from the keyboard or from a click on the frame, which Windows' window tells apart.
//!   A mouse the game makes non-exclusive (`bBackground Mouse=1`) is never held, only hidden over
//!   the window, and the cursor's place there goes to the game as on Windows (`super::Input::pointer`).
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
//! - macOS: winit's event loop must run on the program's main thread. The viewer hands it there
//!   ([`run_on_main_thread`]): the game runs on a thread of its own (8 MB of stack, as a main
//!   thread has), and the main thread waits until the game first needs a window (or the monitors),
//!   then runs [`run`] for good. A program that does not hand it over gets no window on macOS: the
//!   run ends with a line saying so.
//!   The window's Metal layer is made here too, on the main thread, and the surface on it from the
//!   game's or the render thread (d3d9-wgpu `surface_for`). wgpu itself still reads the window's
//!   occlusion state on the thread that takes each frame, so a Main Thread Checker report from
//!   there is expected.
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
    size: SizeKept,
    /// Whether the player may resize the window (`input::frame_of_style`).
    resizable: bool,
    /// Minimised (looked up on the window thread every [`SERVE_EVERY`] at most).
    minimised: AtomicBool,
    /// Hidden from view, as the system says (X11).
    occluded: AtomicBool,
    /// macOS: the window's `CAMetalLayer`, made here on the main thread, where the window is made
    /// (the loop's thread): AppKit's view, and winit's window handle, are the main thread's only,
    /// while the game's device and its surface are made on the game's or the render thread
    /// (d3d9-wgpu `surface_for` makes the surface on this layer). `None` when the window has no
    /// AppKit view (the device is then refused, naming why).
    #[cfg(target_vendor = "apple")]
    metal: Option<raw_window_metal::Layer>,
}

impl Shown {
    /// Made on the loop's thread (on macOS the main thread: the Metal layer is made here).
    fn new(window: Window, resizable: bool) -> Shown {
        let size = window.inner_size();
        #[cfg(target_vendor = "apple")]
        let metal = metal_layer_of(&window);
        Shown {
            window,
            size: SizeKept::new(size),
            resizable,
            minimised: AtomicBool::new(false),
            occluded: AtomicBool::new(false),
            #[cfg(target_vendor = "apple")]
            metal,
        }
    }

    /// The client area asked to be `size` (the game's): while the window cannot be resized its
    /// smallest and largest sizes become `size` first (winit's Wayland window keeps them at the
    /// size it was made with otherwise; its X11 window moves them itself), and a size the system
    /// applies at once (Wayland) is kept, as no resize event follows it.
    fn ask_size(&self, size: LogicalSize<u32>) {
        if !self.resizable {
            self.window.set_min_inner_size(Some(size));
            self.window.set_max_inner_size(Some(size));
        }
        self.size.asked(self.window.request_inner_size(size));
    }
}

/// The client area's size as the drawing side reads it each frame, kept from the window's resize
/// events ([`SizeKept::resized`]) and from the size the system applied at once when the program
/// asked for one ([`SizeKept::asked`]: Wayland, which sends no resize event for it), so that a read
/// asks nothing of the window system.
struct SizeKept([AtomicU32; 2]);

impl SizeKept {
    fn new(size: PhysicalSize<u32>) -> SizeKept {
        SizeKept([AtomicU32::new(size.width), AtomicU32::new(size.height)])
    }

    fn resized(&self, size: PhysicalSize<u32>) {
        self.0[0].store(size.width, Ordering::Relaxed);
        self.0[1].store(size.height, Ordering::Relaxed);
    }

    /// `request_inner_size`'s answer: the size applied at once, or `None` when a resize event
    /// will bring it (X11).
    fn asked(&self, applied: Option<PhysicalSize<u32>>) {
        if let Some(size) = applied {
            self.resized(size);
        }
    }

    fn get(&self) -> (u32, u32) {
        (
            self.0[0].load(Ordering::Relaxed),
            self.0[1].load(Ordering::Relaxed),
        )
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
        Some(self.size.get())
    }

    fn minimised(&self) -> bool {
        self.minimised.load(Ordering::Relaxed) || self.occluded.load(Ordering::Relaxed)
    }

    fn refresh_millihertz(&self) -> Option<u32> {
        self.window.current_monitor()?.refresh_rate_millihertz()
    }

    #[cfg(target_vendor = "apple")]
    fn metal_layer(&self) -> Option<std::ptr::NonNull<std::ffi::c_void>> {
        self.metal.as_ref().map(raw_window_metal::Layer::as_ptr)
    }
}

/// macOS: `window`'s `CAMetalLayer` (its view made layer-backed, a Metal layer added that follows
/// the view's size and scale), made on the main thread, where winit gives the window's handle and
/// AppKit allows its view to be touched. `None`, with a line, when the window has no AppKit view.
#[cfg(target_vendor = "apple")]
fn metal_layer_of(window: &Window) -> Option<raw_window_metal::Layer> {
    use raw_window_handle::RawWindowHandle;
    match window.window_handle().map(|h| h.as_raw()) {
        // SAFETY: winit's AppKit handle names the window's live NSView, and this runs on the main
        // thread (the loop's: `run_on_main_thread`), as from_ns_view requires.
        Ok(RawWindowHandle::AppKit(h)) => {
            Some(unsafe { raw_window_metal::Layer::from_ns_view(h.ns_view) })
        }
        Ok(other) => {
            eprintln!(
                "NV_D3D9={}: the window is not an AppKit one ({other:?}): no Metal layer",
                tag()
            );
            None
        }
        Err(e) => {
            eprintln!(
                "NV_D3D9={}: the window's handle: {e}: no Metal layer",
                tag()
            );
            None
        }
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
        // macOS: the loop runs on the program's main thread, which the host hands to it
        // ([`run_on_main_thread`]): the requests go there. Without it no window can be shown.
        if cfg!(target_vendor = "apple") {
            if !MAIN_LOOP.load(Ordering::Acquire) {
                no_window(
                    "on macOS the game's window needs the program's main thread, which this \
                     program did not hand to the window layer",
                );
            }
            main_requests().give(rx);
            return Mutex::new(tx);
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

/// Whether the program's main thread waits to run the window layer's loop
/// ([`run_on_main_thread`]).
static MAIN_LOOP: AtomicBool = AtomicBool::new(false);

/// The requests' receiver, handed from the first [`window_thread`] call to the main thread.
fn main_requests() -> &'static Handoff<Receiver<Request>> {
    static SLOT: OnceLock<Handoff<Receiver<Request>>> = OnceLock::new();
    SLOT.get_or_init(Handoff::default)
}

/// One value handed from one thread to another that waits for it: given before or after the
/// waiter starts, it is taken (no lost wake-up: the slot is looked at under its lock before each
/// wait).
struct Handoff<T>(Mutex<Option<T>>, std::sync::Condvar);

impl<T> Default for Handoff<T> {
    fn default() -> Self {
        Handoff(Mutex::new(None), std::sync::Condvar::new())
    }
}

impl<T> Handoff<T> {
    fn give(&self, value: T) {
        *self
            .0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(value);
        self.1.notify_all();
    }

    /// Waits until a value is given, and takes it.
    fn take(&self) -> T {
        let mut slot = self
            .0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        loop {
            if let Some(value) = slot.take() {
                return value;
            }
            slot = self
                .1
                .wait(slot)
                .unwrap_or_else(std::sync::PoisonError::into_inner);
        }
    }
}

/// The process's status when the game's thread ends ([`run_on_main_thread`]): 0 when the game
/// returned, 101 when it stopped with a panic (as Rust's own for a main thread's panic; its message
/// is already printed).
fn game_end_status(ended: &std::thread::Result<()>) -> i32 {
    if ended.is_ok() {
        0
    } else {
        101
    }
}

/// The stack of the thread the game runs on when the main thread is the window layer's: the
/// main thread's own on Linux and macOS (8 MB), not a spawned thread's default (2 MB in Rust).
const GAME_STACK: usize = 8 << 20;

/// Runs `game` on a thread of its own and the window layer's loop on the calling thread, which
/// must be the program's main thread (macOS: winit's loop only runs there). The loop starts when
/// the game first needs a window (or the monitors); until then the main thread waits. The process
/// ends from the game's side (`crate::end_process`, the window closed) or here when `game`
/// returns (status 0) or stops with a panic (status 101, its message already printed).
pub(super) fn run_on_main_thread(game: Box<dyn FnOnce() + Send>) -> ! {
    MAIN_LOOP.store(true, Ordering::Release);
    let spawned = std::thread::Builder::new()
        .name("nv-game".into())
        .stack_size(GAME_STACK)
        .spawn(move || {
            let ended = std::panic::catch_unwind(std::panic::AssertUnwindSafe(game));
            crate::end_process(game_end_status(&ended))
        });
    if let Err(e) = spawned {
        eprintln!("nv-rs: the game's thread could not start ({e})");
        crate::end_process(1);
    }
    run(main_requests().take());
    // `run` ends the process itself (`no_window`); this is never reached.
    crate::end_process(1)
}

/// The monitors as the window thread read them when its loop started: `None` until then.
type Monitors = Result<Vec<crate::machine_table::Monitor>, String>;

fn monitors_read() -> &'static (Mutex<Option<Monitors>>, std::sync::Condvar) {
    static READ: OnceLock<(Mutex<Option<Monitors>>, std::sync::Condvar)> = OnceLock::new();
    READ.get_or_init(Default::default)
}

/// How long [`monitors`] waits for the window thread's loop to start.
const MONITORS_WAIT: Duration = Duration::from_secs(10);

/// The monitors as winit reports them (for the machine table, `crate::machine_table::display`):
/// the window thread is started if it is not running (a run that draws needs it anyway), and its
/// loop reads them once when it starts. Where no window can be shown the run ends there, with the
/// line naming why, as it would when the game makes its window.
pub(super) fn monitors() -> Monitors {
    if cfg!(target_vendor = "apple") && !MAIN_LOOP.load(Ordering::Acquire) {
        return Err(
            "on macOS the window layer needs the program's main thread, which this program did \
             not hand to it"
                .into(),
        );
    }
    let _ = window_thread();
    let (read, ready) = monitors_read();
    let guard = read
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let (guard, _) = ready
        .wait_timeout_while(guard, MONITORS_WAIT, |m| m.is_none())
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    guard.clone().unwrap_or_else(|| {
        Err(format!(
            "the window system did not answer in {} s",
            MONITORS_WAIT.as_secs()
        ))
    })
}

/// A winit monitor as the machine table takes it.
fn monitor_of(m: &winit::monitor::MonitorHandle, primary: bool) -> crate::machine_table::Monitor {
    let size = m.size();
    crate::machine_table::Monitor {
        size: (size.width, size.height),
        scale: m.scale_factor(),
        refresh_millihertz: m.refresh_rate_millihertz(),
        modes: m
            .video_modes()
            .map(|v| crate::machine_table::Mode {
                width: v.size().width,
                height: v.size().height,
                bit_depth: v.bit_depth(),
                refresh_millihertz: v.refresh_rate_millihertz(),
            })
            .collect(),
        primary,
    }
}

/// The monitor the machine table describes ([`read_monitors`]), for both threads: the window
/// thread puts a borderless start on it, the game's thread fits a framed window to it and makes
/// one that does not fit borderless over it (winit's monitor handles may be used from any thread).
static DESCRIBED: Mutex<Option<winit::monitor::MonitorHandle>> = Mutex::new(None);

/// The monitor a borderless window is put on and a framed one fitted to: the one the machine
/// table describes (so the game's screen and its window agree), when the monitors were read;
/// else winit's choice (the current one).
fn described_monitor() -> Option<winit::monitor::MonitorHandle> {
    DESCRIBED
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .clone()
}

/// Where the game's place `(x, y)` (its scaled pixels; the game's 0, 0 is the top left of the
/// screen it was told about) is on the desktop, for a window at `scale`: the described monitor's
/// corner plus the place. X11 puts a primary monitor where the desktop's layout has it, not
/// always at 0, 0; Wayland lets no program place its window (the compositor chooses).
fn on_described(x: i32, y: i32, scale: f64) -> winit::dpi::Position {
    let corner =
        described_monitor().map_or(winit::dpi::PhysicalPosition::new(0, 0), |m| m.position());
    if corner == winit::dpi::PhysicalPosition::new(0, 0) {
        return LogicalPosition::new(x, y).into();
    }
    let place = LogicalPosition::new(x, y).to_physical::<i32>(scale);
    winit::dpi::PhysicalPosition::new(corner.x + place.x, corner.y + place.y).into()
}

/// Reads the monitors for [`monitors`], once, on the window thread.
fn read_monitors(event_loop: &ActiveEventLoop) {
    let (read, ready) = monitors_read();
    let mut slot = read
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if slot.is_some() {
        return;
    }
    let primary = event_loop.primary_monitor();
    let handles: Vec<_> = event_loop.available_monitors().collect();
    // The monitor the table describes (the primary one, else the first) is the one the borderless
    // window is put on ([`described_monitor`]), so the game's screen and its window agree.
    let described = crate::machine_table::display::described(
        handles.iter().map(|m| primary.as_ref() == Some(m)),
    );
    *DESCRIBED
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner) = described.map(|i| handles[i].clone());
    let list: Vec<_> = handles
        .iter()
        .enumerate()
        .map(|(i, m)| monitor_of(m, Some(i) == described))
        .collect();
    *slot = Some(if list.is_empty() {
        Err("the window system reports no monitor".into())
    } else {
        Ok(list)
    });
    ready.notify_all();
}

/// Whether winit runs on Wayland (not X11, not macOS).
fn on_wayland(event_loop: &ActiveEventLoop) -> bool {
    #[cfg(not(target_vendor = "apple"))]
    {
        use winit::platform::wayland::ActiveEventLoopExtWayland;
        event_loop.is_wayland()
    }
    #[cfg(target_vendor = "apple")]
    {
        let _ = event_loop;
        false
    }
}

/// The keyboard library winit needs and cannot load, if one is missing: libxkbcommon always, and
/// libxkbcommon-x11 where winit takes X11 (no Wayland compositor named). winit would stop its
/// thread with a panic instead.
#[cfg(not(target_vendor = "apple"))]
fn missing_keyboard_library() -> Option<&'static str> {
    if xkbcommon_dl::xkbcommon_option().is_none() {
        return Some("libxkbcommon.so.0");
    }
    // As winit chooses: a Wayland compositor named by a variable that is set and not empty.
    let named = |name| std::env::var_os(name).is_some_and(|v| !v.is_empty());
    let wayland = named("WAYLAND_DISPLAY") || named("WAYLAND_SOCKET");
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
        deactivated: false,
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
    /// Whether the game's window lost the focus since it was made: from then on each time it gets
    /// it, the game's window procedure gets the activation too (`input::reactivated`).
    deactivated: bool,
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
        read_monitors(event_loop);
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
                shown.ask_size(LogicalSize::new(size.0, size.1));
            }
            let id = next_id();
            keep(id, &shown, top);
            return Some(id);
        }
        let borderless = borderless();
        let frame = input::frame_of_style(request.style);
        let scale = described_monitor().map_or(1.0, |m| m.scale_factor());
        let mut attributes = Window::default_attributes()
            .with_title(latin1(&request.title))
            .with_visible(false)
            .with_inner_size(LogicalSize::new(size.0, size.1))
            .with_position(on_described(left, top, scale))
            .with_active(request.show != Showing::Background);
        if borderless {
            attributes = attributes
                .with_decorations(false)
                .with_fullscreen(Some(Fullscreen::Borderless(described_monitor())));
        } else {
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
            // The game cannot go on without its window: the run ends, with the line.
            Err(e) => no_window(&format!("winit could not make the game's window ({e})")),
        };
        if request.visible() && request.show != Showing::Never {
            window.set_visible(true);
        }
        let winit_id = window.id();
        let shown = Arc::new(Shown::new(window, borderless || frame.resizable));
        let id = next_id();
        keep(id, &shown, id);
        self.tops.insert(winit_id, id);
        Some(id)
    }

    /// Holds the cursor in the window `id` and hides it there, as Windows does for a program whose
    /// DirectInput mouse is exclusive and in the foreground: locked where the system can, else kept
    /// inside the window; only where `input::cursor_action` says (never for a mouse that is not
    /// exclusive, as on Windows).
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

    /// Does what `input::cursor_action` says for `event` at the window `id` (the rule the Win32
    /// window follows too) and answers it, for the caller to keep the cursor's place.
    fn follow(&mut self, id: RealWindow, event: input::CursorEvent) -> input::CursorAction {
        let now = input::CursorNow {
            exclusive: mouse_exclusive(),
            held: self.held.is_some(),
            active: self.focused,
            hold_wanted: super::hold_wanted(),
        };
        let action = input::cursor_action(now, event);
        if action.let_go {
            self.let_go();
        }
        if action.hold {
            super::take_hold_wanted();
            self.hold(id);
        } else if action.hide {
            if let Some(e) = windows().get(&id) {
                e.shown.window.set_cursor_visible(false);
            }
        }
        action
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

    fn window_event(&mut self, event_loop: &ActiveEventLoop, window: WindowId, event: WindowEvent) {
        let Some(&id) = self.tops.get(&window) else {
            return;
        };
        match event {
            // Closing the game's window ends the run (the player's way out), as on Windows.
            WindowEvent::CloseRequested => window_closed(),
            // Activated, its display's scale changed, shown again: wgpu asks the window for a
            // frame at once rather than at the end of a rest (`d3d9_wgpu::window_changed`).
            // A focus that follows a loss of it is also the game's activation (`WA_ACTIVE`: winit
            // does not say whether a click gave it), which makes its mouse exclusive again.
            WindowEvent::Focused(true) => {
                self.focused = true;
                d3d9_wgpu::window_changed();
                if self.deactivated {
                    input::reactivated(1);
                }
                self.follow(id, input::CursorEvent::Activated);
            }
            // Losing the focus lets go of everything this window held.
            WindowEvent::Focused(false) => {
                self.focused = false;
                self.deactivated = true;
                input::focus_lost();
                self.let_go();
            }
            WindowEvent::ScaleFactorChanged { .. } => {
                if let Some(e) = windows().get(&id) {
                    e.shown.size.resized(e.shown.window.inner_size());
                }
                d3d9_wgpu::window_changed();
            }
            WindowEvent::Resized(size) => {
                if let Some(e) = windows().get(&id) {
                    e.shown.size.resized(size);
                }
                // The full-screen borderless window given its monitor's size turned: Wayland read
                // the monitor turned, so the game was told the wrong shape; said once, with the
                // fix. Against the monitor the window is on (the described one, which it was put
                // on, unless the compositor chose another).
                if borderless() && on_wayland(event_loop) {
                    let monitor = windows().get(&id).and_then(|e| {
                        let w = &e.shown.window;
                        w.fullscreen()?;
                        w.current_monitor().or_else(described_monitor)
                    });
                    if let Some(m) = monitor {
                        let (window, monitor) =
                            ((size.width, size.height), (m.size().width, m.size().height));
                        static SAID: AtomicBool = AtomicBool::new(false);
                        if input::read_turned(window, monitor)
                            && !SAID.swap(true, Ordering::Relaxed)
                        {
                            eprintln!("{}", input::turned_line(window, monitor));
                        }
                    }
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
                    self.follow(id, input::CursorEvent::Pressed);
                }
                input::button(index, down);
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let wheel = input::wheel(&mut self.rest[2], scroll_of(delta));
                if wheel != 0 {
                    input::moved(0, 0, wheel);
                }
            }
            // The game's mouse not exclusive (`bBackground Mouse=1`): the cursor's place over the
            // focused window is the game's `WM_MOUSEMOVE`, as on Windows; for an exclusive one the
            // hold is made again once the game made it exclusive again (`input::cursor_action`).
            WindowEvent::CursorMoved { position, .. } => {
                if !self.follow(id, input::CursorEvent::Moved).record {
                    return;
                }
                let size = windows().get(&id).map(|e| e.shown.window.inner_size());
                if let Some(size) = size {
                    input::pointed(
                        (position.x, position.y),
                        (f64::from(size.width), f64::from(size.height)),
                    );
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
            shown.ask_size(LogicalSize::new(size.0, size.1));
        }
        return;
    }
    let scale = w.scale_factor();
    if flags & input::NO_SIZE == 0 {
        let border = crate::window::game_frame(e, handle).unwrap_or([0; 4]);
        let mut client = input::inner_for_game_rect(rect, border);
        // The monitor the game was told about (the machine table's: the primary one, else the
        // first), in the window's own scaled pixels.
        if let Some(m) = described_monitor() {
            let monitor = (m.size().width, m.size().height);
            let screen = input::scaled_screen(monitor, scale);
            match input::placement(rect, border, monitor, scale) {
                input::Placement::Borderless => {
                    // Framed, it would reach past the screen's edges (report 004, a Steam Deck):
                    // the borderless window over that monitor instead, the game's picture scaled
                    // to it.
                    static SAID: AtomicBool = AtomicBool::new(false);
                    if !SAID.swap(true, Ordering::Relaxed) {
                        let outer = (rect[2].wrapping_sub(rect[0]), rect[3].wrapping_sub(rect[1]));
                        eprintln!("{}", input::too_large_line(outer, screen, client, monitor));
                    }
                    super::want_borderless(true);
                    fill_screen(w, m);
                    return;
                }
                input::Placement::Framed { client: framed } => {
                    static SAID: AtomicBool = AtomicBool::new(false);
                    if !SAID.swap(true, Ordering::Relaxed) {
                        eprintln!("{}", input::framed_line(framed, screen));
                    }
                    client = framed;
                }
            }
        }
        if flags & input::NO_MOVE == 0 {
            w.set_outer_position(on_described(rect[0], rect[1], scale));
        }
        shown.ask_size(LogicalSize::new(client.0, client.1));
    } else if flags & input::NO_MOVE == 0 {
        w.set_outer_position(on_described(rect[0], rect[1], scale));
    }
}

/// The game's framed window made the borderless one over `monitor` (the described one), as a
/// borderless start makes it (`Layer::make`): no frame, winit's borderless full screen on that
/// monitor, no size held (the compositor gives the monitor's).
fn fill_screen(w: &Window, monitor: winit::monitor::MonitorHandle) {
    w.set_resizable(true);
    w.set_min_inner_size(None::<LogicalSize<u32>>);
    w.set_max_inner_size(None::<LogicalSize<u32>>);
    w.set_decorations(false);
    w.set_fullscreen(Some(Fullscreen::Borderless(Some(monitor))));
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

    /// The hand-off of the requests to the main thread ([`run_on_main_thread`]): a value given
    /// before the waiter starts and one given after it waits are both taken (no lost wake-up).
    #[test]
    fn the_handoff_is_taken_whether_given_before_or_after_the_wait() {
        // Given before the waiter starts.
        let before = Handoff::default();
        before.give(7u32);
        assert_eq!(before.take(), 7);
        // Given after the waiter is waiting: the waiter is told it has started, then the value
        // is given a little later (a lost wake-up would hang here; the test harness's time
        // limit, and the channel's, end such a run).
        let after = Arc::new(Handoff::default());
        let (started, has_started) = std::sync::mpsc::channel();
        let (took, has_taken) = std::sync::mpsc::channel();
        let waiter = {
            let after = Arc::clone(&after);
            std::thread::spawn(move || {
                started.send(()).unwrap();
                took.send(after.take()).unwrap();
            })
        };
        has_started.recv().unwrap();
        std::thread::sleep(Duration::from_millis(50));
        assert!(has_taken.try_recv().is_err(), "taken before it was given");
        after.give(9u32);
        assert_eq!(has_taken.recv_timeout(Duration::from_secs(10)), Ok(9));
        waiter.join().unwrap();
        // Taken once: the slot is empty again.
        assert!(after.0.lock().unwrap().is_none());
    }

    /// The process's status when the game's thread ends: 0 when the game returned, 101 when it
    /// stopped with a panic.
    #[test]
    fn the_games_end_gives_0_or_101() {
        assert_eq!(game_end_status(&Ok(())), 0);
        let panicked = std::panic::catch_unwind(|| panic!("the game stopped"));
        assert_eq!(game_end_status(&panicked), 101);
    }

    /// The size the drawing side reads follows the game's own resizes: on Wayland the size applied
    /// at once (no resize event follows it), on X11 the resize event that follows the request.
    #[test]
    fn the_size_read_follows_the_programs_own_resizes() {
        // Wayland: the request answers the size it applied.
        let kept = SizeKept::new(PhysicalSize::new(336, 279));
        kept.asked(Some(PhysicalSize::new(1280, 720)));
        assert_eq!(kept.get(), (1280, 720));
        // X11: the request answers nothing; the resize event brings the size.
        let kept = SizeKept::new(PhysicalSize::new(336, 279));
        kept.asked(None);
        assert_eq!(kept.get(), (336, 279));
        kept.resized(PhysicalSize::new(1280, 720));
        assert_eq!(kept.get(), (1280, 720));
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
