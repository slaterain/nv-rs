//! The game's window as wgpu shows frames in it (`window.rs`): the swap chain follows the
//! window's size whatever the back buffer's, the back buffer covers the whole client area, and a
//! minimised window takes no frame. On a real (hidden) window of this process, through the whole
//! backend; the window's frames are read from the `NV_SHOT` captures (`wgpu-window-N.bmp`).
//!
//! Windows only (the window layer's other platforms come with its port). A machine without a GPU
//! adapter skips with a message. This file is its own test binary: it sets `NV_SHOT` for its
//! process.

#![cfg(windows)]

use d3d9_stream::{Backend, Cmd, Data};
use d3d9_wgpu::{Output, WgpuBackend};

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

/// The window's frame `frame` as captured (`wgpu-window-N.bmp`): its width, height and the
/// distinct pixels (B, G, R, A), or `None` when no frame was shown then.
fn window_frame(dir: &std::path::Path, frame: u32) -> Option<(u32, u32, Vec<[u8; 4]>)> {
    let bytes = std::fs::read(dir.join(format!("wgpu-window-{frame:06}.bmp"))).ok()?;
    let (width, height, pixels) = d3d9_wgpu::shot::read_bmp(&bytes)?;
    let mut seen: Vec<[u8; 4]> = Vec::new();
    for p in pixels.chunks_exact(4) {
        let p = [p[0], p[1], p[2], p[3]];
        if !seen.contains(&p) {
            seen.push(p);
        }
    }
    Some((width, height, seen))
}

/// The back buffer (32x24) cleared to one colour, shown in a window of another size: the swap
/// chain is the window's size and the colour covers all of it; the window sized again, the next
/// frame is the new size; sized to nothing, no frame is shown and the game's `Present` goes on;
/// sized again, frames are shown again; after a `Reset` to another back buffer size, still the
/// window's size; minimised, no frame.
#[test]
fn the_swap_chain_follows_the_window_and_a_minimised_window_takes_no_frame() {
    let dir = std::env::temp_dir().join(format!("nv-rs-wgpu-window-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("the shots' folder");
    std::env::set_var("NV_SHOT", &dir);
    std::env::set_var("NV_SHOT_LIST", "1,2,3,4,5,6");
    let hwnd = hidden_window(64, 48);
    assert_ne!(hwnd, 0, "the test window");
    let mut b = WgpuBackend::new(Output::Window);
    let present = words(&[32, 24, A8R8G8B8, 1, 0, 0, 1, 0, 1, 0, 0, 0, 0, 0]);
    let back = words(&[BACK]);
    b.execute(Cmd::CreateDevice {
        device: DEVICE,
        focus: hwnd,
        window: hwnd,
        behavior: 0x54,
        present: Data::Bytes(&present),
        back_buffers: Data::Bytes(&back),
        depth_stencil: 0,
    });
    if b.device().is_none() {
        eprintln!("no GPU adapter: the window test is skipped");
        // SAFETY: the test's own window.
        unsafe { DestroyWindow(hwnd) };
        let _ = std::fs::remove_dir_all(&dir);
        return;
    }
    let show = |b: &mut WgpuBackend, frame: u32, colour: u32| {
        b.execute(Cmd::SetRenderTarget {
            device: DEVICE,
            index: 0,
            surface: BACK,
        });
        b.execute(Cmd::Clear {
            device: DEVICE,
            count: 0,
            rects: None,
            flags: 1,
            colour,
            z: 1f32.to_bits(),
            stencil: 0,
            model: 0,
        });
        b.execute(Cmd::Present {
            device: DEVICE,
            source: None,
            dest: None,
            window: 0,
            dirty: false,
            model: 0,
            frame,
        });
    };
    // B, G, R, A of 0xAARRGGBB.
    let bgra = |c: u32| [c as u8, (c >> 8) as u8, (c >> 16) as u8, (c >> 24) as u8];
    show(&mut b, 1, 0xFF20_4060);
    assert_eq!(
        window_frame(&dir, 1),
        Some((64, 48, vec![bgra(0xFF20_4060)])),
        "the window's size, the colour over all of it"
    );
    // SAFETY: the test's own window.
    unsafe {
        SetWindowPos(
            hwnd,
            0,
            0,
            0,
            100,
            30,
            SWP_NOMOVE | SWP_NOZORDER | SWP_NOACTIVATE,
        )
    };
    show(&mut b, 2, 0xFF80_A0C0);
    assert_eq!(
        window_frame(&dir, 2),
        Some((100, 30, vec![bgra(0xFF80_A0C0)])),
        "the window sized again"
    );
    // No area (sized to nothing): no frame; sized again, the frames come again.
    let size = |w: i32, h: i32| {
        // SAFETY: the test's own window.
        unsafe {
            SetWindowPos(
                hwnd,
                0,
                0,
                0,
                w,
                h,
                SWP_NOMOVE | SWP_NOZORDER | SWP_NOACTIVATE,
            )
        };
    };
    size(0, 0);
    assert_eq!(state(hwnd), (false, (0, 0)), "the test window has no area");
    show(&mut b, 3, 0xFF11_2233);
    assert_eq!(window_frame(&dir, 3), None, "no area, no frame");
    size(100, 30);
    show(&mut b, 4, 0xFF44_5566);
    assert_eq!(
        window_frame(&dir, 4),
        Some((100, 30, vec![bgra(0xFF44_5566)])),
        "sized again, the frames come again"
    );
    // A Reset to another back buffer size: the swap chain stays the window's size.
    let present = words(&[16, 16, A8R8G8B8, 1, 0, 0, 1, 0, 1, 0, 0, 0, 0, 0]);
    b.execute(Cmd::Reset {
        device: DEVICE,
        window: hwnd,
        present: Data::Bytes(&present),
        back_buffers: Data::Bytes(&back),
        depth_stencil: 0,
    });
    show(&mut b, 5, 0xFF77_8899);
    assert_eq!(
        window_frame(&dir, 5),
        Some((100, 30, vec![bgra(0xFF77_8899)])),
        "after a Reset"
    );
    // Minimised (the style bit IsIconic reads, set by hand: the window stays hidden; Windows
    // keeps it minimised from then on, so this comes last): no frame, and the game goes on.
    // SAFETY: the test's own window.
    unsafe {
        let style = GetWindowLongW(hwnd, GWL_STYLE) as u32;
        SetWindowLongW(hwnd, GWL_STYLE, (style | WS_MINIMIZE) as i32);
    }
    assert!(state(hwnd).0, "the test window reads as minimised");
    show(&mut b, 6, 0xFF11_2233);
    assert_eq!(
        window_frame(&dir, 6),
        None,
        "a minimised window takes no frame"
    );
    let d = b.device().expect("the device");
    assert_eq!(d.wgpu_errors(), 0, "wgpu reported no error");
    drop(b);
    // SAFETY: the test's own window.
    unsafe { DestroyWindow(hwnd) };
    let _ = std::fs::remove_dir_all(&dir);
}
