//! The game's window as wgpu shows frames in it ([`Window`]).
//!
//! The swap chain is the window's own size, read in its real pixels before each frame, whatever
//! the game's back buffer is: `Present` draws the back buffer over the whole client area, as
//! Direct3D 9's windowed `Present` without rectangles does (filtered when the sizes differ, a
//! copy when they match). Vulkan's rule on Windows is a swap chain of the window's size (its
//! `currentExtent`). NVIDIA's driver also took another size and stretched it (run wl3, a 1280x720
//! window); other drivers need not, and a swap chain of another size than the window's keeps a
//! frame from going to the screen without the compositor. That is every game resolution other
//! than the window's: the borderless window over a monitor of another resolution, a window sized.
//!
//! The window's life (measure/wgpu/MAIN-RENDERER.md, B): minimised, no frame is asked for or
//! drawn (the game runs on); a frame the window cannot give in time (`Timeout`, `Occluded`) is
//! skipped, and none is asked for again for a while ([`RETRY_AFTER`], doubling while it gives none,
//! up to [`RETRY_MOST`]), so a window that gives none (a locked screen, a display switched off)
//! does not hold the game in wgpu's one-second wait for a frame on every frame;
//! an outdated swap chain (the window sized, the display changed) is made again at the window's
//! size; a lost surface (the display or its driver gone) is made anew on the same window. Each
//! stretch of frames not shown is one log line when it starts and one when frames come again.
//!
//! The thread that shows a frame is in the window's DPI awareness while it does ([`DpiScope`]):
//! the borderless window is per-monitor aware (`engine-platform`'s window thread) and the game's
//! threads are not, and a thread that is not reads a scaled size (2048x1152 for a 2560x1440 window
//! at 125% scaling), the graphics driver's reads for the swap chain on that thread included. The
//! thread is in it from the frame's size to its present and back in its own after: the render
//! thread, or with it off (`NV_RENDER_THREAD=0`) the game's thread that presents, whose own
//! awareness the game's other calls keep. The present itself runs on the device's queue thread
//! (after the frame's submit), which takes the window's awareness for it the same way.

use std::time::{Duration, Instant};

/// How long a window that gave no frame (`Timeout`, `Occluded`, a swap chain or surface that could
/// not be made again) is first left alone before a frame is asked of it again: the frames between
/// are skipped. Each time in a row it gives none, twice as long, up to [`RETRY_MOST`]: wgpu waits up
/// to a second for a frame (`Timeout`), and a window that keeps giving none must not hold the game
/// that long every time.
pub(crate) const RETRY_AFTER: Duration = Duration::from_millis(100);
pub(crate) const RETRY_MOST: Duration = Duration::from_secs(2);

/// `D3DPRESENT_INTERVAL_IMMEDIATE`.
const INTERVAL_IMMEDIATE: u32 = 0x8000_0000;

/// The present mode for the game's present interval: `D3DPRESENT_INTERVAL_IMMEDIATE` (the game's
/// `iPresentInterval=0`) or `NV_D3D9_PRESENT=immediate` without waiting for the display
/// (`Immediate` where the surface has it), anything else with the display (`Fifo`: the game asks
/// for no other interval than one).
pub(crate) fn present_mode(interval: u32, caps: &wgpu::SurfaceCapabilities) -> wgpu::PresentMode {
    let immediate = interval == INTERVAL_IMMEDIATE
        || std::env::var("NV_D3D9_PRESENT").is_ok_and(|v| v.eq_ignore_ascii_case("immediate"));
    if immediate && caps.present_modes.contains(&wgpu::PresentMode::Immediate) {
        wgpu::PresentMode::Immediate
    } else if immediate {
        wgpu::PresentMode::AutoNoVsync
    } else {
        wgpu::PresentMode::Fifo
    }
}

/// The rest after a window gave no frame: none is asked for until it ends ([`RETRY_AFTER`] the
/// first time, twice as long each time in a row, up to [`RETRY_MOST`]; short again once a frame
/// is shown). The clock is the caller's, so tests drive it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Backoff {
    next: Duration,
    until: Option<Instant>,
}

impl Backoff {
    pub(crate) fn new() -> Backoff {
        Backoff {
            next: RETRY_AFTER,
            until: None,
        }
    }

    /// Whether no frame is asked for at `now`.
    pub(crate) fn resting(&mut self, now: Instant) -> bool {
        match self.until {
            Some(until) if now < until => true,
            _ => {
                self.until = None;
                false
            }
        }
    }

    /// No frame could be had at `now`: a rest, twice as long as the last one in a row.
    pub(crate) fn failed(&mut self, now: Instant) {
        self.until = Some(now + self.next);
        self.next = (self.next * 2).min(RETRY_MOST);
    }

    /// A frame was shown: the next rest is short again.
    pub(crate) fn shown(&mut self) {
        self.next = RETRY_AFTER;
        self.until = None;
    }
}

/// How many times the game's window was activated, its display changed or the machine woke
/// ([`window_changed`]): a window resting after it gave no frame is asked at once after one.
static CHANGED: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// The game's window was activated (Alt+Tab back, a click, the screen unlocked), its display
/// changed, or the machine woke from sleep (the window layer calls it from its window
/// procedure): a window resting after it gave no frame is asked for one at once, not at the end
/// of its rest (up to [`RETRY_MOST`] away, and then wgpu's wait for a frame).
pub fn window_changed() {
    CHANGED.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    // The frame-rate counter does not take the time away for a slow frame.
    crate::frame_rate::gap();
}

/// The game's window and its swap chain.
pub(crate) struct Window {
    /// The window (an `HWND`).
    hwnd: isize,
    /// The instance and adapter the surface was made on: a lost surface is made again on them.
    instance: wgpu::Instance,
    adapter: wgpu::Adapter,
    surface: wgpu::Surface<'static>,
    pub(crate) config: wgpu::SurfaceConfiguration,
    /// The game's present interval and whether frames are copied out (`NV_SHOT`): what the
    /// configuration is chosen from again for a new surface or a `Reset`.
    interval: u32,
    copies: bool,
    /// Whether wgpu took the surface's last configuration. A frame is asked only of a surface
    /// that holds one: wgpu gives a surface its error handler only when a configuration succeeds,
    /// and asks of one without it stop the process.
    configured: bool,
    /// The swap chain is configured again before the next frame (a `Reset`, a configuration wgpu
    /// refused, a first suboptimal frame at this size).
    stale: bool,
    /// The swap chain size a suboptimal frame was answered at (configured again once): a driver
    /// that keeps answering suboptimal must not have the swap chain made again, and the GPU waited
    /// for, on every frame.
    suboptimal_at: Option<(u32, u32)>,
    rest: Backoff,
    /// The [`CHANGED`] count last seen.
    seen_changes: u64,
    /// While frames are not shown, the time between them is at least `refresh` (the display's
    /// refresh interval) and `paced` the last one: a minimised window does not run the game as
    /// fast as the machine can, for nothing on screen.
    refresh: Duration,
    paced: Option<Instant>,
    /// Frames asked for; those not shown since the last one shown, and why the last was not.
    asked: u64,
    skipped: u64,
    last_why: Option<NoFrame>,
    /// Configurations made, and those wgpu refused (the device's line every 600 presents).
    pub(crate) configures: u64,
    pub(crate) refused: u64,
}

/// Why no frame was shown ([`Window::frame`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum NoFrame {
    Minimised,
    NoArea,
    Resting,
    TimedOut,
    Occluded,
    Outdated,
    Lost,
    NotConfigured,
    Refused,
}

impl NoFrame {
    fn says(self) -> &'static str {
        match self {
            NoFrame::Minimised => "the window is minimised",
            NoFrame::NoArea => "the window has no area to show a frame in",
            NoFrame::Resting => "the window is left alone a while after it gave none",
            NoFrame::TimedOut => "the window gave no frame in time",
            NoFrame::Occluded => "the window is hidden from view",
            NoFrame::Outdated => "the window's swap chain stays out of date at its size",
            NoFrame::Lost => "the window's surface is lost and could not be made again",
            NoFrame::NotConfigured => "wgpu refused the swap chain's configuration",
            NoFrame::Refused => "wgpu refused the frame",
        }
    }
}

impl Window {
    /// The swap chain of `surface` (made on `instance` for `adapter`) on the window `hwnd`, at the
    /// window's size, in the present mode for the game's present `interval`; with `copies`, its
    /// frames can be copied out (`NV_SHOT`'s captures of the window's frame). The calling thread,
    /// the one that shows the frames, takes the window's DPI awareness. A window minimised or
    /// without an area now gets its swap chain at its first frame with one (`fallback`, the back
    /// buffer's size, until then). A configuration wgpu refuses is asked once more, from what the
    /// surface offers then; refused again, this device cannot show the game in the window: why,
    /// for [`crate::device::Device::new`] to try the next graphics API (after the last, the
    /// platform draws with Direct3D 9). Later refusals (a resized or restored window) leave the
    /// window without frames until a configuration is taken (tried again after each rest).
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        hwnd: isize,
        instance: wgpu::Instance,
        adapter: wgpu::Adapter,
        surface: wgpu::Surface<'static>,
        device: &wgpu::Device,
        fallback: (u32, u32),
        interval: u32,
        copies: bool,
    ) -> Result<Window, String> {
        let _dpi = DpiScope::of(hwnd);
        let area = client_size(hwnd).filter(|&(w, h)| w > 0 && h > 0);
        let (width, height) = area.unwrap_or(fallback);
        let mut w = Window {
            hwnd,
            instance,
            adapter,
            surface,
            config: wgpu::SurfaceConfiguration {
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                format: wgpu::TextureFormat::Bgra8Unorm,
                width: width.max(1),
                height: height.max(1),
                present_mode: wgpu::PresentMode::Fifo,
                color_space: wgpu::SurfaceColorSpace::Auto,
                desired_maximum_frame_latency: 2,
                alpha_mode: wgpu::CompositeAlphaMode::Auto,
                view_formats: vec![],
            },
            interval,
            copies,
            configured: false,
            stale: true,
            suboptimal_at: None,
            rest: Backoff::new(),
            seen_changes: CHANGED.load(std::sync::atomic::Ordering::Relaxed),
            refresh: refresh_interval(),
            paced: None,
            asked: 0,
            skipped: 0,
            last_why: None,
            configures: 0,
            refused: 0,
        };
        if !w.choose() {
            return Err("the window's surface offers this adapter no format".into());
        }
        if area.is_none() || minimised(hwnd) {
            eprintln!(
                "NV_D3D9 wgpu: the window is minimised or has no area: its swap chain is made at its first frame with one"
            );
            return Ok(w);
        }
        if let Err(first) = w.configure(device) {
            w.choose();
            if let Err(again) = w.configure(device) {
                return Err(format!(
                    "the window's swap chain ({}x{}, {:?}) was refused twice: {again}",
                    w.config.width, w.config.height, w.config.present_mode
                ));
            }
            eprintln!(
                "NV_D3D9 wgpu: the window's swap chain was refused once, then taken: {first}"
            );
            // The first refusal in play is logged again.
            w.refused = 0;
        }
        Ok(w)
    }

    /// The configuration's format, present mode, alpha and usage chosen from what the surface
    /// offers now (a new surface's may differ from the lost one's); false when it offers no format.
    fn choose(&mut self) -> bool {
        let caps = self.surface.get_capabilities(&self.adapter);
        let Some(&first) = caps.formats.first() else {
            return false;
        };
        let c = &mut self.config;
        c.format = caps
            .formats
            .iter()
            .copied()
            .find(|f| *f == wgpu::TextureFormat::Bgra8Unorm)
            .or_else(|| caps.formats.iter().copied().find(|f| !f.is_srgb()))
            .unwrap_or(first);
        if crate::device::refused_for_tests(self.adapter.get_info().backend) {
            c.format = wgpu::TextureFormat::R8Unorm;
        }
        c.present_mode = present_mode(self.interval, &caps);
        c.alpha_mode = caps
            .alpha_modes
            .first()
            .copied()
            .unwrap_or(wgpu::CompositeAlphaMode::Auto);
        c.usage = wgpu::TextureUsages::RENDER_ATTACHMENT
            | if self.copies {
                caps.usages & wgpu::TextureUsages::COPY_SRC
            } else {
                wgpu::TextureUsages::empty()
            };
        true
    }

    /// The swap chain configured with [`Window::config`], wgpu's errors caught here (not passed to
    /// the device's handler): what wgpu said when it refused it. Refused, it is stale (configured
    /// again before the next frame) and no frame is asked of the surface until a configuration is
    /// taken.
    fn configure(&mut self, device: &wgpu::Device) -> Result<(), String> {
        let scopes = [
            wgpu::ErrorFilter::Internal,
            wgpu::ErrorFilter::OutOfMemory,
            wgpu::ErrorFilter::Validation,
        ]
        .map(|filter| device.push_error_scope(filter));
        self.surface.configure(device, &self.config);
        let mut refused = None;
        for scope in scopes.into_iter().rev() {
            if let Some(e) = crate::device::block_on(scope.pop()) {
                refused.get_or_insert(e);
            }
        }
        self.configures += 1;
        self.configured = refused.is_none();
        self.stale = !self.configured;
        let Some(e) = refused else {
            return Ok(());
        };
        self.refused += 1;
        Err(e.to_string())
    }

    /// The device's `Reset` with the game's present `interval`: the swap chain is configured again
    /// before the next frame, at the window's size, in that interval's present mode.
    pub(crate) fn reset(&mut self, interval: u32) {
        self.interval = interval;
        self.choose();
        self.stale = true;
    }

    /// The calling thread in the window's DPI awareness until the scope is dropped: held from
    /// before [`Window::frame`] to after the frame's present.
    pub(crate) fn dpi_scope(&self) -> DpiScope {
        DpiScope::of(self.hwnd)
    }

    /// The window's handle (the queue thread takes its DPI awareness for the present).
    pub(crate) fn hwnd(&self) -> isize {
        self.hwnd
    }

    /// The window's next frame, at its size now; `None` when it gives none now (the start and the
    /// end of a stretch of frames not shown are each one log line).
    pub(crate) fn frame(&mut self, device: &wgpu::Device) -> Option<wgpu::SurfaceTexture> {
        self.asked += 1;
        match self.take(device) {
            Ok(t) => {
                self.rest.shown();
                self.paced = None;
                if self.skipped > 0 {
                    eprintln!(
                        "NV_D3D9 wgpu: frames shown again in the window ({} not shown; the last because {})",
                        self.skipped,
                        self.last_why.map_or("", NoFrame::says)
                    );
                    self.skipped = 0;
                }
                Some(t)
            }
            Err(why) => {
                if self.skipped == 0 {
                    eprintln!(
                        "NV_D3D9 wgpu: frames not shown from the window's frame {}: {} (the game goes on)",
                        self.asked,
                        why.says()
                    );
                }
                self.skipped += 1;
                self.last_why = Some(why);
                self.pace();
                None
            }
        }
    }

    /// A frame not shown: the game is held until a display refresh has passed since the last one,
    /// as a shown frame waits for the display.
    fn pace(&mut self) {
        let now = Instant::now();
        if let Some(last) = self.paced {
            let since = now.saturating_duration_since(last);
            if since < self.refresh {
                std::thread::sleep(self.refresh - since);
            }
        }
        self.paced = Some(Instant::now());
    }

    fn take(&mut self, device: &wgpu::Device) -> Result<wgpu::SurfaceTexture, NoFrame> {
        // Activated, the display changed or the machine woke: asked at once, rest or not.
        let changes = CHANGED.load(std::sync::atomic::Ordering::Relaxed);
        if changes != self.seen_changes {
            self.seen_changes = changes;
            self.rest.shown();
        }
        if self.rest.resting(Instant::now()) {
            return Err(NoFrame::Resting);
        }
        if minimised(self.hwnd) {
            return Err(NoFrame::Minimised);
        }
        match self.fit(device, false) {
            Ok(()) => {}
            // No area is the window's state, as minimised is: asked again at the next frame.
            Err(NoFrame::NoArea) => return Err(NoFrame::NoArea),
            Err(why) => return Err(self.rested(why)),
        }
        // An outdated swap chain is made again at the window's size and asked once more; a lost
        // surface is made anew on the same window.
        for _ in 0..2 {
            let why = match self.surface.get_current_texture() {
                wgpu::CurrentSurfaceTexture::Success(t) => return Ok(t),
                wgpu::CurrentSurfaceTexture::Suboptimal(t) => {
                    let size = (self.config.width, self.config.height);
                    if self.suboptimal_at != Some(size) {
                        self.suboptimal_at = Some(size);
                        self.stale = true;
                    }
                    return Ok(t);
                }
                wgpu::CurrentSurfaceTexture::Outdated => match self.fit(device, true) {
                    Ok(()) => continue,
                    Err(why) => why,
                },
                wgpu::CurrentSurfaceTexture::Lost => match self.make_again(device) {
                    Ok(()) => continue,
                    Err(why) => why,
                },
                wgpu::CurrentSurfaceTexture::Timeout => NoFrame::TimedOut,
                wgpu::CurrentSurfaceTexture::Occluded => NoFrame::Occluded,
                wgpu::CurrentSurfaceTexture::Validation => NoFrame::Refused,
            };
            return Err(self.rested(why));
        }
        // Out of date twice in a row: made again at the window's size, still out of date.
        Err(self.rested(NoFrame::Outdated))
    }

    /// `why` no frame was had: none is asked for a while ([`Backoff`]).
    fn rested(&mut self, why: NoFrame) -> NoFrame {
        self.rest.failed(Instant::now());
        why
    }

    /// The swap chain at the window's size now, configured again when the size changed, when it
    /// is stale (or wgpu refused the last configuration), or with `always`; why not when the
    /// window has no area (nothing configured) or wgpu refused the configuration.
    fn fit(&mut self, device: &wgpu::Device, always: bool) -> Result<(), NoFrame> {
        let (width, height) =
            client_size(self.hwnd).unwrap_or((self.config.width, self.config.height));
        if width == 0 || height == 0 {
            return Err(NoFrame::NoArea);
        }
        let resized = (width, height) != (self.config.width, self.config.height);
        if always || resized || self.stale || !self.configured {
            self.config.width = width;
            self.config.height = height;
            if let Err(e) = self.configure(device) {
                if self.refused == 1 {
                    eprintln!(
                        "NV_D3D9 wgpu: the window's swap chain ({}x{}, {:?}) was refused: {e}",
                        self.config.width, self.config.height, self.config.present_mode
                    );
                }
                return Err(NoFrame::NotConfigured);
            }
        }
        Ok(())
    }

    /// [`crate::device::Device::reconfigure_window_for_tests`]: `format` asked from the next frame
    /// on, on a new surface never configured with `new_surface`.
    pub(crate) fn retarget_for_tests(&mut self, format: wgpu::TextureFormat, new_surface: bool) {
        if new_surface {
            if let Ok(surface) = crate::device::surface_for(&self.instance, self.hwnd) {
                self.surface = surface;
                self.configured = false;
            }
        }
        self.config.format = format;
        self.stale = true;
    }

    /// A new surface on the same window, its configuration chosen again from what it offers, and
    /// configured at the window's size; why not when it could not be made or configured (the
    /// window then rests).
    fn make_again(&mut self, device: &wgpu::Device) -> Result<(), NoFrame> {
        let Ok(surface) = crate::device::surface_for(&self.instance, self.hwnd) else {
            return Err(NoFrame::Lost);
        };
        self.surface = surface;
        self.configured = false;
        self.stale = true;
        self.suboptimal_at = None;
        if !self.choose() {
            return Err(NoFrame::Lost);
        }
        eprintln!("NV_D3D9 wgpu: the window's surface was lost: made again");
        self.fit(device, true)
    }
}

/// The client area's size of `hwnd` in the calling thread's pixels (the window's own, once
/// [`Window::new`] gave the thread the window's DPI awareness); `None` where it cannot be read.
#[cfg(windows)]
pub(crate) fn client_size(hwnd: isize) -> Option<(u32, u32)> {
    let mut rect = [0i32; 4];
    // SAFETY: a query of a window handle the forwarding layer's window thread keeps for the whole
    // run (a handle no longer valid fails it); `rect` is a RECT's four longs.
    let read = unsafe { GetClientRect(hwnd, rect.as_mut_ptr()) } != 0;
    read.then(|| {
        (
            (rect[2] - rect[0]).max(0) as u32,
            (rect[3] - rect[1]).max(0) as u32,
        )
    })
}

#[cfg(not(windows))]
pub(crate) fn client_size(_hwnd: isize) -> Option<(u32, u32)> {
    None
}

/// The primary display's refresh interval (the borderless window covers it), 60 a second where
/// the display does not say.
#[cfg(windows)]
fn refresh_interval() -> Duration {
    // A DEVMODEW (220 bytes) as 16-bit words: dmSize at byte 68, dmDisplayFrequency at 184.
    let mut mode = [0u16; 110];
    mode[34] = 220;
    // SAFETY: `mode` is a DEVMODEW-sized buffer with its dmSize set, as the call requires.
    let read =
        unsafe { EnumDisplaySettingsW(std::ptr::null(), ENUM_CURRENT_SETTINGS, mode.as_mut_ptr()) }
            != 0;
    let hz = u32::from(mode[92]) | u32::from(mode[93]) << 16;
    // 0 and 1 mean the display's default rate, which it does not name.
    let hz = if read && hz > 1 { hz } else { 60 };
    Duration::from_secs_f64(1.0 / f64::from(hz))
}

#[cfg(not(windows))]
fn refresh_interval() -> Duration {
    Duration::from_secs_f64(1.0 / 60.0)
}

/// Whether the window, or the top-level window it is in, is minimised.
#[cfg(windows)]
fn minimised(hwnd: isize) -> bool {
    // SAFETY: queries of a window handle (see [`client_size`]).
    unsafe {
        let root = GetAncestor(hwnd, GA_ROOT);
        IsIconic(if root != 0 { root } else { hwnd }) != 0
    }
}

#[cfg(not(windows))]
fn minimised(_hwnd: isize) -> bool {
    false
}

/// The calling thread in a window's DPI awareness until this is dropped, then back in its own
/// (nothing done when it already is in it).
pub(crate) struct DpiScope {
    /// The thread's own awareness to go back to; 0: nothing was changed.
    #[cfg_attr(not(windows), allow(dead_code))]
    before: isize,
}

impl DpiScope {
    /// The calling thread in `hwnd`'s DPI awareness.
    #[cfg(windows)]
    pub(crate) fn of(hwnd: isize) -> DpiScope {
        // SAFETY: the calling thread's own setting, from a window handle's.
        unsafe {
            let window = GetWindowDpiAwarenessContext(hwnd);
            if window == 0
                || AreDpiAwarenessContextsEqual(window, GetThreadDpiAwarenessContext()) != 0
            {
                return DpiScope { before: 0 };
            }
            DpiScope {
                before: SetThreadDpiAwarenessContext(window),
            }
        }
    }

    #[cfg(not(windows))]
    pub(crate) fn of(_hwnd: isize) -> DpiScope {
        DpiScope { before: 0 }
    }
}

#[cfg(windows)]
impl Drop for DpiScope {
    fn drop(&mut self) {
        if self.before != 0 {
            // SAFETY: the calling thread's own setting, put back.
            unsafe {
                SetThreadDpiAwarenessContext(self.before);
            }
        }
    }
}

/// `GA_ROOT`, `ENUM_CURRENT_SETTINGS`.
#[cfg(windows)]
const GA_ROOT: u32 = 2;
#[cfg(windows)]
const ENUM_CURRENT_SETTINGS: u32 = 0xFFFF_FFFF;

#[cfg(windows)]
#[link(name = "user32")]
extern "system" {
    fn GetClientRect(hwnd: isize, rect: *mut i32) -> i32;
    fn GetAncestor(hwnd: isize, flags: u32) -> isize;
    fn IsIconic(hwnd: isize) -> i32;
    fn GetWindowDpiAwarenessContext(hwnd: isize) -> isize;
    fn GetThreadDpiAwarenessContext() -> isize;
    fn SetThreadDpiAwarenessContext(context: isize) -> isize;
    fn AreDpiAwarenessContextsEqual(a: isize, b: isize) -> i32;
    fn EnumDisplaySettingsW(device: *const u16, mode: u32, devmode: *mut u16) -> i32;
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The game's interval one waits for the display; `D3DPRESENT_INTERVAL_IMMEDIATE` does not,
    /// `Immediate` where the surface has it.
    #[test]
    fn the_present_interval_picks_the_present_mode() {
        let caps = |modes: Vec<wgpu::PresentMode>| wgpu::SurfaceCapabilities {
            present_modes: modes,
            ..Default::default()
        };
        let all = caps(vec![
            wgpu::PresentMode::Fifo,
            wgpu::PresentMode::Immediate,
            wgpu::PresentMode::Mailbox,
        ]);
        if std::env::var_os("NV_D3D9_PRESENT").is_none() {
            assert_eq!(present_mode(1, &all), wgpu::PresentMode::Fifo);
            assert_eq!(present_mode(0, &all), wgpu::PresentMode::Fifo);
        }
        assert_eq!(
            present_mode(INTERVAL_IMMEDIATE, &all),
            wgpu::PresentMode::Immediate
        );
        let fifo = caps(vec![wgpu::PresentMode::Fifo]);
        assert_eq!(
            present_mode(INTERVAL_IMMEDIATE, &fifo),
            wgpu::PresentMode::AutoNoVsync
        );
    }

    /// A window that gives no frame rests: 100 ms, then twice as long each time in a row, up to
    /// 2 s; nothing is asked during a rest; a frame shown makes the next rest short again.
    #[test]
    fn a_window_that_gives_no_frame_rests_longer_each_time() {
        let ms = Duration::from_millis;
        let t0 = Instant::now();
        let mut b = Backoff::new();
        assert!(!b.resting(t0));
        b.failed(t0);
        assert!(b.resting(t0 + ms(99)));
        assert!(!b.resting(t0 + ms(100)));
        let mut at = t0 + ms(100);
        for rest in [200, 400, 800, 1600, 2000, 2000] {
            b.failed(at);
            assert!(b.resting(at + ms(rest - 1)), "{rest}");
            assert!(!b.resting(at + ms(rest)), "{rest}");
            at += ms(rest);
        }
        b.shown();
        assert!(!b.resting(at));
        b.failed(at);
        assert!(b.resting(at + ms(99)));
        assert!(!b.resting(at + ms(100)), "short again after a frame");
    }
}
