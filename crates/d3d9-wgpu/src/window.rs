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
//! The thread that shows the frames takes the window's DPI awareness: the borderless window is
//! per-monitor aware (`engine-platform`'s window thread) and the game's threads are not, and a
//! thread that is not reads a scaled size (2048x1152 for a 2560x1440 window at 125% scaling),
//! the graphics driver's reads for the swap chain on that thread included.

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

/// The game's window and its swap chain.
pub(crate) struct Window {
    /// The window (an `HWND`).
    hwnd: isize,
    /// The instance and adapter the surface was made on: a lost surface is made again on them.
    instance: wgpu::Instance,
    adapter: wgpu::Adapter,
    surface: wgpu::Surface<'static>,
    pub(crate) config: wgpu::SurfaceConfiguration,
    /// The swap chain is configured again before the next frame (a frame came back suboptimal,
    /// or the device's `Reset`).
    stale: bool,
    /// No frame is asked for before this (a window that gave none: [`RETRY_AFTER`]), and how long
    /// the next rest is.
    rest_until: Option<Instant>,
    rest: Duration,
    /// Frames asked for, and those not shown since the last one shown.
    asked: u64,
    skipped: u64,
}

/// Why no frame was shown ([`Window::frame`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum NoFrame {
    Minimised,
    NoArea,
    TimedOut,
    Occluded,
    Outdated,
    Lost,
    Refused,
}

impl NoFrame {
    fn says(self) -> &'static str {
        match self {
            NoFrame::Minimised => "the window is minimised",
            NoFrame::NoArea => "the window has no area to show a frame in",
            NoFrame::TimedOut => "the window gave no frame in time",
            NoFrame::Occluded => "the window is hidden from view",
            NoFrame::Outdated => "the window's swap chain could not be made for its size",
            NoFrame::Lost => "the window's surface is lost and could not be made again",
            NoFrame::Refused => "wgpu refused the frame",
        }
    }
}

impl Window {
    /// The swap chain of `surface` (made on `instance` for `adapter`) on the window `hwnd`, at the
    /// window's size, or at `fallback` (the back buffer's) when the window has none to read, in
    /// the present mode for the game's present `interval`; with `copies`, its frames can be copied
    /// out (`NV_SHOT`'s captures of the window's frame). The calling thread, the one that shows
    /// the frames, takes the window's DPI awareness.
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
    ) -> Window {
        take_dpi_awareness(hwnd);
        let caps = surface.get_capabilities(&adapter);
        let format = caps
            .formats
            .iter()
            .copied()
            .find(|f| *f == wgpu::TextureFormat::Bgra8Unorm)
            .or_else(|| caps.formats.iter().copied().find(|f| !f.is_srgb()))
            .unwrap_or(caps.formats[0]);
        let copies = if copies {
            caps.usages & wgpu::TextureUsages::COPY_SRC
        } else {
            wgpu::TextureUsages::empty()
        };
        let (width, height) = client_size(hwnd)
            .filter(|&(w, h)| w > 0 && h > 0)
            .unwrap_or(fallback);
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | copies,
            format,
            width: width.max(1),
            height: height.max(1),
            present_mode: present_mode(interval, &caps),
            color_space: wgpu::SurfaceColorSpace::Auto,
            desired_maximum_frame_latency: 2,
            alpha_mode: caps.alpha_modes[0],
            view_formats: vec![],
        };
        surface.configure(device, &config);
        Window {
            hwnd,
            instance,
            adapter,
            surface,
            config,
            stale: false,
            rest_until: None,
            rest: RETRY_AFTER,
            asked: 0,
            skipped: 0,
        }
    }

    /// The device's `Reset` with the game's present `interval`: the swap chain is configured again
    /// before the next frame, at the window's size, in that interval's present mode.
    pub(crate) fn reset(&mut self, interval: u32) {
        let caps = self.surface.get_capabilities(&self.adapter);
        self.config.present_mode = present_mode(interval, &caps);
        self.stale = true;
    }

    /// The window's next frame, at its size now; `None` when it gives none now (the start and the
    /// end of a stretch of frames not shown are each one log line).
    pub(crate) fn frame(&mut self, device: &wgpu::Device) -> Option<wgpu::SurfaceTexture> {
        self.asked += 1;
        match self.take(device) {
            Ok(t) => {
                self.rest = RETRY_AFTER;
                if self.skipped > 0 {
                    eprintln!(
                        "NV_D3D9 wgpu: frames shown again in the window ({} not shown)",
                        self.skipped
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
                None
            }
        }
    }

    fn take(&mut self, device: &wgpu::Device) -> Result<wgpu::SurfaceTexture, NoFrame> {
        if let Some(until) = self.rest_until {
            if Instant::now() < until {
                return Err(NoFrame::TimedOut);
            }
            self.rest_until = None;
        }
        if minimised(self.hwnd) {
            return Err(NoFrame::Minimised);
        }
        if !self.fit(device, false) {
            return Err(NoFrame::NoArea);
        }
        // An outdated swap chain is made again at the window's size and asked once more; a lost
        // surface is made anew on the same window.
        for _ in 0..2 {
            let rest = match self.surface.get_current_texture() {
                wgpu::CurrentSurfaceTexture::Success(t) => return Ok(t),
                wgpu::CurrentSurfaceTexture::Suboptimal(t) => {
                    self.stale = true;
                    return Ok(t);
                }
                wgpu::CurrentSurfaceTexture::Outdated => {
                    if self.fit(device, true) {
                        continue;
                    }
                    return Err(NoFrame::NoArea);
                }
                wgpu::CurrentSurfaceTexture::Lost => {
                    if self.make_again(device) {
                        continue;
                    }
                    NoFrame::Lost
                }
                wgpu::CurrentSurfaceTexture::Timeout => NoFrame::TimedOut,
                wgpu::CurrentSurfaceTexture::Occluded => NoFrame::Occluded,
                wgpu::CurrentSurfaceTexture::Validation => return Err(NoFrame::Refused),
            };
            return Err(self.rest_for(rest));
        }
        // Out of date twice in a row: made again at the window's size, still out of date.
        Err(self.rest_for(NoFrame::Outdated))
    }

    /// `why` no frame was had, after which none is asked for a while ([`RETRY_AFTER`], twice as
    /// long each time in a row up to [`RETRY_MOST`]).
    fn rest_for(&mut self, why: NoFrame) -> NoFrame {
        self.rest_until = Some(Instant::now() + self.rest);
        self.rest = (self.rest * 2).min(RETRY_MOST);
        why
    }

    /// The swap chain at the window's size now: configured again when the size changed, when
    /// stale, or with `always`; false when the window has no area to show a frame in.
    fn fit(&mut self, device: &wgpu::Device, always: bool) -> bool {
        let (width, height) =
            client_size(self.hwnd).unwrap_or((self.config.width, self.config.height));
        if width == 0 || height == 0 {
            return false;
        }
        if always || self.stale || (width, height) != (self.config.width, self.config.height) {
            self.config.width = width;
            self.config.height = height;
            self.surface.configure(device, &self.config);
            self.stale = false;
        }
        true
    }

    /// A new surface on the same window, configured as the lost one was (its format chosen again
    /// where the new one lacks it); false when it could not be made.
    fn make_again(&mut self, device: &wgpu::Device) -> bool {
        let Ok(surface) = crate::device::surface_for(&self.instance, self.hwnd) else {
            return false;
        };
        let caps = surface.get_capabilities(&self.adapter);
        if caps.formats.is_empty() {
            return false;
        }
        if !caps.formats.contains(&self.config.format) {
            self.config.format = caps
                .formats
                .iter()
                .copied()
                .find(|f| !f.is_srgb())
                .unwrap_or(caps.formats[0]);
        }
        self.surface = surface;
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

/// The calling thread takes `hwnd`'s DPI awareness.
#[cfg(windows)]
fn take_dpi_awareness(hwnd: isize) {
    // SAFETY: the calling thread's own setting, from a window handle's.
    unsafe {
        let context = GetWindowDpiAwarenessContext(hwnd);
        if context != 0 {
            SetThreadDpiAwarenessContext(context);
        }
    }
}

#[cfg(not(windows))]
fn take_dpi_awareness(_hwnd: isize) {}

/// `GA_ROOT`.
#[cfg(windows)]
const GA_ROOT: u32 = 2;

#[cfg(windows)]
#[link(name = "user32")]
extern "system" {
    fn GetClientRect(hwnd: isize, rect: *mut i32) -> i32;
    fn GetAncestor(hwnd: isize, flags: u32) -> isize;
    fn IsIconic(hwnd: isize) -> i32;
    fn GetWindowDpiAwarenessContext(hwnd: isize) -> isize;
    fn SetThreadDpiAwarenessContext(context: isize) -> isize;
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
}
