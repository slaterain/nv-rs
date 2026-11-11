//! The game's window as wgpu shows frames in it (`window.rs`): the swap chain follows the
//! window's size whatever the back buffer's, the back buffer covers the whole client area (copied
//! at the same size, filtered when stretched), a minimised window takes no frame, and a refused
//! configuration never has a frame asked of it. On a real (hidden) window of this process, through
//! the whole backend; the window's frames are read from the `NV_SHOT` captures
//! (`wgpu-window-N.bmp`, beside the back buffer's `wgpu-frame-N.bmp`).
//!
//! Windows only (the window layer's other platforms come with its port). A machine without a GPU
//! adapter skips with a message. This file is its own test binary: it sets `NV_SHOT` for its
//! process, and its tests take turns (one device and window at a time).

#![cfg(windows)]

use d3d9_stream::{Backend, Cmd, Data, Reply};
use d3d9_wgpu::{Output, WgpuBackend};
use std::path::{Path, PathBuf};

const DEVICE: u32 = 0x1000;
const BACK: u32 = 0x1100;
/// `D3DFMT_A8R8G8B8`.
const A8R8G8B8: u32 = 21;

fn words(w: &[u32]) -> Vec<u8> {
    w.iter().flat_map(|w| w.to_le_bytes()).collect()
}

const WS_POPUP: u32 = 0x8000_0000;
const WS_MINIMIZE: u32 = 0x2000_0000;
const GWL_STYLE: i32 = -16;
const SWP_NOMOVE: u32 = 0x2;
const SWP_NOZORDER: u32 = 0x4;
const SWP_NOACTIVATE: u32 = 0x10;

#[link(name = "user32")]
extern "system" {
    fn CreateWindowExW(
        ex_style: u32,
        class: *const u16,
        title: *const u16,
        style: u32,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        parent: isize,
        menu: isize,
        instance: isize,
        param: *const core::ffi::c_void,
    ) -> isize;
    fn DestroyWindow(hwnd: isize) -> i32;
    fn SetWindowPos(hwnd: isize, after: isize, x: i32, y: i32, w: i32, h: i32, flags: u32) -> i32;
    fn GetWindowLongW(hwnd: isize, index: i32) -> i32;
    fn SetWindowLongW(hwnd: isize, index: i32, value: i32) -> i32;
    fn IsIconic(hwnd: isize) -> i32;
    fn GetClientRect(hwnd: isize, rect: *mut i32) -> i32;
}

/// Whether the window reads as minimised, and its client size.
fn state(hwnd: isize) -> (bool, (i32, i32)) {
    let mut r = [0i32; 4];
    // SAFETY: the test's own window; `r` is a RECT's four longs.
    unsafe {
        GetClientRect(hwnd, r.as_mut_ptr());
        (IsIconic(hwnd) != 0, (r[2] - r[0], r[3] - r[1]))
    }
}

/// The captures' folder, `NV_SHOT` for this process (set once, before any device reads it), every
/// present the tests make captured (`NV_SHOT_LIST`, 1 to 100), however soon after the last it
/// comes: by `NV_SHOT`'s interval alone (1 ms at least) a present within 1 ms of the one before
/// was not (CI's run of #461: the frame after the one with no area).
fn shots_dir() -> &'static Path {
    static DIR: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();
    DIR.get_or_init(|| {
        let dir = std::env::temp_dir().join(format!("nv-rs-wgpu-window-{}", std::process::id()));
        std::env::set_var("NV_SHOT", format!("{},1", dir.display()));
        let every: Vec<String> = (1..=100).map(|n: u32| n.to_string()).collect();
        std::env::set_var("NV_SHOT_LIST", every.join(","));
        dir
    })
}

/// One device and window at a time (a failed test leaves the lock poisoned; the next still runs).
static TURN: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// A hidden test window and a wgpu device on it with a back buffer of `back` (B8G8R8A8).
struct Rig {
    b: WgpuBackend,
    hwnd: isize,
    dir: &'static Path,
    _turn: std::sync::MutexGuard<'static, ()>,
}

impl Rig {
    /// `None` (with a line) where there is no GPU adapter.
    fn new(window: (i32, i32), back: (u32, u32)) -> Option<Rig> {
        let turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
        let dir = shots_dir();
        let _ = std::fs::remove_dir_all(dir);
        std::fs::create_dir_all(dir).expect("the captures' folder");
        let hwnd = hidden_window(window.0, window.1);
        assert_ne!(hwnd, 0, "the test window");
        let mut b = WgpuBackend::new(Output::Window);
        let present = words(&[back.0, back.1, A8R8G8B8, 1, 0, 0, 1, 0, 1, 0, 0, 0, 0, 0]);
        b.execute(Cmd::CreateDevice {
            device: DEVICE,
            focus: hwnd,
            window: hwnd,
            behavior: 0x54,
            present: Data::Bytes(&present),
            back_buffers: Data::Bytes(&words(&[BACK])),
            depth_stencil: 0,
        });
        if b.device().is_none() {
            eprintln!("no GPU adapter: the window test is skipped");
            // SAFETY: the test's own window.
            unsafe { DestroyWindow(hwnd) };
            return None;
        }
        Some(Rig {
            b,
            hwnd,
            dir,
            _turn: turn,
        })
    }

    /// The back buffer cleared to `colour` (0xAARRGGBB) in `rect` (all of it for `None`).
    fn clear(&mut self, rect: Option<[i32; 4]>, colour: u32) {
        let rect = rect.map(|r| words(&r.map(|v| v as u32)));
        self.b.execute(Cmd::SetRenderTarget {
            device: DEVICE,
            index: 0,
            surface: BACK,
        });
        self.b.execute(Cmd::Clear {
            device: DEVICE,
            count: u32::from(rect.is_some()),
            rects: rect.as_deref().map(Data::Bytes),
            flags: 1,
            colour,
            z: 1f32.to_bits(),
            stencil: 0,
            model: 0,
        });
    }

    /// `Present` number `frame`.
    fn present(&mut self, frame: u32) {
        self.b.execute(Cmd::Present {
            device: DEVICE,
            source: None,
            dest: None,
            window: 0,
            dirty: false,
            model: 0,
            frame,
        });
    }

    /// The back buffer cleared to `colour`, then presented as `frame`.
    fn show(&mut self, frame: u32, colour: u32) {
        self.clear(None, colour);
        self.present(frame);
    }

    /// The capture `name-N.bmp` of `frame`: width, height and pixels (B, G, R, X rows); `None`
    /// when none was taken (no frame shown).
    fn capture(&self, name: &str, frame: u32) -> Option<(u32, u32, Vec<u8>)> {
        let bytes = std::fs::read(self.dir.join(format!("{name}-{frame:06}.bmp"))).ok()?;
        d3d9_wgpu::shot::read_bmp(&bytes)
    }

    /// The window's frame `frame`: its size and its distinct pixels (B, G, R, A).
    fn window_frame(&self, frame: u32) -> Option<(u32, u32, Vec<[u8; 4]>)> {
        let (width, height, pixels) = self.capture("wgpu-window", frame)?;
        let mut seen: Vec<[u8; 4]> = Vec::new();
        for p in pixels.chunks_exact(4) {
            let p = [p[0], p[1], p[2], p[3]];
            if !seen.contains(&p) {
                seen.push(p);
            }
        }
        Some((width, height, seen))
    }

    fn size(&self, w: i32, h: i32) {
        // SAFETY: the test's own window.
        unsafe {
            SetWindowPos(
                self.hwnd,
                0,
                0,
                0,
                w,
                h,
                SWP_NOMOVE | SWP_NOZORDER | SWP_NOACTIVATE,
            )
        };
    }
}

impl Drop for Rig {
    fn drop(&mut self) {
        // SAFETY: the test's own window, destroyed after the device (fields drop after this).
        unsafe { DestroyWindow(self.hwnd) };
    }
}

/// A popup window of the `STATIC` class, `width` by `height`, never shown (no `WS_VISIBLE`).
fn hidden_window(width: i32, height: i32) -> isize {
    let class: Vec<u16> = "STATIC\0".encode_utf16().collect();
    let title: Vec<u16> = "nv-rs window test\0".encode_utf16().collect();
    // SAFETY: a plain window of a system class, destroyed by the test.
    unsafe {
        CreateWindowExW(
            0,
            class.as_ptr(),
            title.as_ptr(),
            WS_POPUP,
            0,
            0,
            width,
            height,
            0,
            0,
            0,
            std::ptr::null(),
        )
    }
}

/// B, G, R, A of 0xAARRGGBB.
fn bgra(c: u32) -> [u8; 4] {
    [c as u8, (c >> 8) as u8, (c >> 16) as u8, (c >> 24) as u8]
}

/// The back buffer (32x24) cleared to one colour, shown in a window of another size: the swap
/// chain is the window's size and the colour covers all of it; the window sized again, the next
/// frame is the new size; sized to nothing, no frame is shown and the game's `Present` goes on;
/// sized again, frames are shown again; after a `Reset` to another back buffer size, still the
/// window's size; minimised, no frame.
#[test]
fn the_swap_chain_follows_the_window_and_a_minimised_window_takes_no_frame() {
    let Some(mut g) = Rig::new((64, 48), (32, 24)) else {
        return;
    };
    g.show(1, 0xFF20_4060);
    assert_eq!(
        g.window_frame(1),
        Some((64, 48, vec![bgra(0xFF20_4060)])),
        "the window's size, the colour over all of it"
    );
    g.size(100, 30);
    g.show(2, 0xFF80_A0C0);
    assert_eq!(
        g.window_frame(2),
        Some((100, 30, vec![bgra(0xFF80_A0C0)])),
        "the window sized again"
    );
    // No area (sized to nothing): no frame; sized again, the frames come again.
    g.size(0, 0);
    assert_eq!(
        state(g.hwnd),
        (false, (0, 0)),
        "the test window has no area"
    );
    g.show(3, 0xFF11_2233);
    assert_eq!(g.window_frame(3), None, "no area, no frame");
    g.size(100, 30);
    g.show(4, 0xFF44_5566);
    assert_eq!(
        g.window_frame(4),
        Some((100, 30, vec![bgra(0xFF44_5566)])),
        "sized again, the frames come again"
    );
    // A Reset to another back buffer size: the swap chain stays the window's size.
    let present = words(&[16, 16, A8R8G8B8, 1, 0, 0, 1, 0, 1, 0, 0, 0, 0, 0]);
    g.b.execute(Cmd::Reset {
        device: DEVICE,
        window: g.hwnd,
        present: Data::Bytes(&present),
        back_buffers: Data::Bytes(&words(&[BACK])),
        depth_stencil: 0,
    });
    g.show(5, 0xFF77_8899);
    assert_eq!(
        g.window_frame(5),
        Some((100, 30, vec![bgra(0xFF77_8899)])),
        "after a Reset"
    );
    // Minimised (the style bit IsIconic reads, set by hand: the window stays hidden; Windows
    // keeps it minimised from then on, so this comes last): no frame, and the game goes on.
    // SAFETY: the test's own window.
    unsafe {
        let style = GetWindowLongW(g.hwnd, GWL_STYLE) as u32;
        SetWindowLongW(g.hwnd, GWL_STYLE, (style | WS_MINIMIZE) as i32);
    }
    assert!(state(g.hwnd).0, "the test window reads as minimised");
    g.show(6, 0xFF11_2233);
    assert_eq!(g.window_frame(6), None, "a minimised window takes no frame");
    let d = g.b.device().expect("the device");
    assert_eq!(d.wgpu_errors(), 0, "wgpu reported no error");
}

/// A back buffer of two colours (the left half one, the right half the other). In a window of
/// its own size the window's frame is the back buffer byte for byte (a copy). Stretched to twice
/// the width, each half keeps its colour up to the seam, and the seam only blends the two (a
/// filtered stretch: every pixel between them, changing one way along each row).
#[test]
fn the_same_size_copies_exactly_and_a_stretch_blends_only_at_the_seam() {
    let Some(mut g) = Rig::new((32, 24), (32, 24)) else {
        return;
    };
    let (left, right) = (0xFF10_2030u32, 0xFFF0_D0B0u32);
    let two = |g: &mut Rig, frame| {
        g.clear(Some([0, 0, 16, 24]), left);
        g.clear(Some([16, 0, 32, 24]), right);
        g.present(frame);
    };
    two(&mut g, 1);
    let back = g
        .capture("wgpu-frame", 1)
        .expect("the back buffer's capture");
    let window = g.capture("wgpu-window", 1).expect("the window's frame");
    assert_eq!((window.0, window.1), (32, 24));
    // B, G, R the same in every pixel (the window's fourth byte is its own alpha).
    let rgb =
        |p: &[u8]| -> Vec<[u8; 3]> { p.chunks_exact(4).map(|c| [c[0], c[1], c[2]]).collect() };
    assert_eq!(rgb(&window.2), rgb(&back.2), "the same size: a copy");
    g.size(64, 24);
    two(&mut g, 2);
    let (w, h, px) = g.capture("wgpu-window", 2).expect("the stretched frame");
    assert_eq!((w, h), (64, 24));
    let (l, r) = (bgra(left), bgra(right));
    for y in 0..h as usize {
        let row: Vec<[u8; 3]> = px[y * w as usize * 4..(y + 1) * w as usize * 4]
            .chunks_exact(4)
            .map(|c| [c[0], c[1], c[2]])
            .collect();
        assert_eq!(row[0], [l[0], l[1], l[2]], "row {y}: the left edge");
        assert_eq!(row[63], [r[0], r[1], r[2]], "row {y}: the right edge");
        assert!(
            row[..28].iter().all(|p| *p == [l[0], l[1], l[2]]),
            "row {y}: the left half"
        );
        assert!(
            row[36..].iter().all(|p| *p == [r[0], r[1], r[2]]),
            "row {y}: the right half"
        );
        for x in 1..64 {
            for c in 0..3 {
                assert!(
                    row[x][c] >= row[x - 1][c],
                    "row {y}, x {x}: one way across the seam"
                );
                assert!(
                    (l[c]..=r[c]).contains(&row[x][c]),
                    "row {y}, x {x}: between the two"
                );
            }
        }
    }
}

/// A configuration wgpu refuses (a format the surface does not offer), also on a surface made
/// anew and never configured (as after a lost one): no frame is asked of it (wgpu gives such a
/// surface no error handler, and asking it stops the process), the game's `Present` goes on, and
/// once a configuration is taken again (after the rest) the frames come back.
#[test]
fn a_refused_configuration_never_has_a_frame_asked_of_it() {
    let Some(mut g) = Rig::new((40, 30), (40, 30)) else {
        return;
    };
    g.show(1, 0xFF30_3030);
    assert!(g.window_frame(1).is_some());
    for (frame, new_surface) in [(2, false), (4, true)] {
        let d = g.b.device().expect("the device");
        d.reconfigure_window_for_tests(wgpu::TextureFormat::R8Unorm, new_surface);
        g.show(frame, 0xFF40_4040);
        assert_eq!(
            g.window_frame(frame),
            None,
            "refused, no frame ({new_surface})"
        );
        let d = g.b.device().expect("the device");
        d.reconfigure_window_for_tests(wgpu::TextureFormat::Bgra8Unorm, false);
        // The rest after the refusal (100 ms the first time, 200 the second), then a frame.
        std::thread::sleep(std::time::Duration::from_millis(250));
        g.show(frame + 1, 0xFF50_5050);
        assert_eq!(
            g.window_frame(frame + 1),
            Some((40, 30, vec![bgra(0xFF50_5050)])),
            "taken again, frames again ({new_surface})"
        );
    }
}

/// `CreateDevice` on a fresh hidden window with a 32x24 back buffer: the reply, and the backend.
fn create(window: isize) -> (Reply, WgpuBackend) {
    let mut b = WgpuBackend::new(Output::Window);
    let present = words(&[32, 24, A8R8G8B8, 1, 0, 0, 1, 0, 1, 0, 0, 0, 0, 0]);
    let reply = b.execute(Cmd::CreateDevice {
        device: DEVICE,
        focus: window,
        window,
        behavior: 0x54,
        present: Data::Bytes(&present),
        back_buffers: Data::Bytes(&words(&[BACK])),
        depth_stencil: 0,
    });
    (reply, b)
}

/// A device that cannot show the game in its window is not made, and a graphics API whose device
/// or swap chain fails gives way to the next. With the swap chain refused twice on every API (a
/// format no window offers), with every device lost before its swap chain is made (as a driver
/// that refuses the swap chain with out of memory does: wgpu takes that for a lost device), and
/// with a panic once the device and its swap chain are made, `CreateDevice` answers not created
/// (the platform then draws with Direct3D 9) and the window is left free: a device made on it
/// afterwards shows frames in it. Refused on Vulkan alone (on a machine with a Vulkan card), the
/// device is made on Direct3D 12.
#[test]
fn a_device_that_cannot_show_frames_is_not_made_and_the_next_api_is_tried() {
    // A machine with an adapter only (a device made as usual first).
    drop(match Rig::new((40, 30), (40, 30)) {
        Some(g) => g,
        None => return,
    });
    let turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    let hwnd = hidden_window(48, 36);
    assert_ne!(hwnd, 0, "the test window");
    let fail = d3d9_wgpu::device::fail_devices_for_tests;
    for (refused, lost, panic) in [
        (wgpu::Backends::all(), false, false),
        (wgpu::Backends::empty(), true, false),
        (wgpu::Backends::empty(), false, true),
    ] {
        fail(refused, lost, panic);
        let (reply, mut b) = create(hwnd);
        fail(wgpu::Backends::empty(), false, false);
        assert!(
            matches!(reply, Reply::Device { created: false, .. }),
            "refused {refused:?}, lost {lost}, panic {panic}: {reply:?}"
        );
        assert!(b.device().is_none());
    }
    // Refused on Vulkan only: Direct3D 12, where the run tries it after Vulkan. Without a Vulkan
    // card (CI's runners), or with NV_WGPU_BACKEND naming one API, Vulkan then Direct3D 12 is
    // never tried: said so.
    let card = d3d9_wgpu::hardware_adapter();
    if std::env::var_os("NV_WGPU_BACKEND").is_none()
        && card.as_deref().is_some_and(|n| n.ends_with("(Vulkan)"))
    {
        fail(wgpu::Backends::VULKAN, false, false);
        let (reply, mut b) = create(hwnd);
        fail(wgpu::Backends::empty(), false, false);
        assert!(
            matches!(reply, Reply::Device { created: true, .. }),
            "{reply:?}"
        );
        let name = b.device().expect("the device").adapter_name().to_string();
        assert!(name.ends_with("(Dx12)"), "{name}");
    } else {
        eprintln!("no Vulkan card: the Vulkan-then-Direct3D 12 part is skipped");
    }
    // As usual on the same window afterwards, on the run's first API (a failed attempt that
    // kept the window would push it onto the next): the window's size, the colour over all of it.
    let (reply, mut b) = create(hwnd);
    assert!(
        matches!(reply, Reply::Device { created: true, .. }),
        "{reply:?}"
    );
    let api = |n: &str| n.rsplit_once(" (").map(|(_, a)| a.to_string());
    let name = b.device().expect("the device").adapter_name().to_string();
    if let Some(card) = &card {
        assert_eq!(api(&name), api(card), "{name} against {card}");
    }
    let dir = shots_dir();
    let _ = std::fs::remove_dir_all(dir);
    std::fs::create_dir_all(dir).expect("the captures' folder");
    let mut g = Rig {
        b,
        hwnd,
        dir,
        _turn: turn,
    };
    g.show(1, 0xFF60_7080);
    assert_eq!(
        g.window_frame(1),
        Some((48, 36, vec![bgra(0xFF60_7080)])),
        "a device on the window the failed ones were made on"
    );
}

/// A window with no area when the device is made (sized to nothing) still gets the device: its
/// swap chain is made at its first frame with an area, and frames are shown at that size.
#[test]
fn a_window_without_an_area_at_create_device_still_gets_the_device() {
    // A machine with an adapter only (a device made as usual first).
    drop(match Rig::new((40, 30), (40, 30)) {
        Some(g) => g,
        None => return,
    });
    let turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    let dir = shots_dir();
    let _ = std::fs::remove_dir_all(dir);
    std::fs::create_dir_all(dir).expect("the captures' folder");
    let hwnd = hidden_window(0, 0);
    assert_ne!(hwnd, 0, "the test window");
    assert_eq!(state(hwnd), (false, (0, 0)), "the test window has no area");
    let (reply, b) = create(hwnd);
    assert!(
        matches!(reply, Reply::Device { created: true, .. }),
        "no area: {reply:?}"
    );
    let mut g = Rig {
        b,
        hwnd,
        dir,
        _turn: turn,
    };
    g.show(1, 0xFF11_2233);
    assert_eq!(g.window_frame(1), None, "no area, no frame");
    g.size(50, 20);
    g.show(2, 0xFF44_5566);
    assert_eq!(
        g.window_frame(2),
        Some((50, 20, vec![bgra(0xFF44_5566)])),
        "sized, the swap chain is made and the frames come"
    );
}

/// A window minimised when the device is made still gets the device (its swap chain is made at
/// its first frame with an area): never Direct3D 9 for a player who minimises the game while it
/// starts. Minimised, the game's frames go on unshown.
#[test]
fn a_window_minimised_at_create_device_still_gets_the_device() {
    // A machine with an adapter only (a device made as usual first).
    drop(match Rig::new((40, 30), (40, 30)) {
        Some(g) => g,
        None => return,
    });
    let turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    let dir = shots_dir();
    let _ = std::fs::remove_dir_all(dir);
    std::fs::create_dir_all(dir).expect("the captures' folder");
    // Minimised: the style bit IsIconic reads, set by hand (the window stays hidden).
    let hwnd = hidden_window(48, 36);
    assert_ne!(hwnd, 0, "the test window");
    // SAFETY: the test's own window.
    unsafe {
        let style = GetWindowLongW(hwnd, GWL_STYLE) as u32;
        SetWindowLongW(hwnd, GWL_STYLE, (style | WS_MINIMIZE) as i32);
    }
    assert!(state(hwnd).0, "the test window reads as minimised");
    let (reply, b) = create(hwnd);
    assert!(
        matches!(reply, Reply::Device { created: true, .. }),
        "minimised: {reply:?}"
    );
    let mut g = Rig {
        b,
        hwnd,
        dir,
        _turn: turn,
    };
    g.show(1, 0xFF77_8899);
    assert_eq!(g.window_frame(1), None, "minimised, no frame");
    let d = g.b.device().expect("the device");
    assert_eq!(d.wgpu_errors(), 0, "wgpu reported no error");
}
