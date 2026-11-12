//! The screen the backend shows while it does a long step of its own before play (the first
//! start's pipelines, [`crate::prepare`]), and over the game's start until its first picture
//! ([`crate::device::Device`]'s start): a television test card in the style of the 1950s (our
//! own, drawn here: sepia rings, crosshairs with tick marks, corner targets, a light film grain
//! and a vignette), the nv-rs logo in its centre, a PLEASE STAND BY banner, a bar with the step's
//! progress, the step's line and the last lines the run printed, scaled to the window. Whoever
//! does the long step gives it a [`Progress`]; the same numbers go to the log, for a launcher to
//! read.
//!
//! The logo is the 256 by 256 image in the port's icon (`viewer/nv-rs.ico`, the exe's icon: a PNG
//! there, read with the plugin reader's zlib decoder), else its 128 by 128 one, toned sepia and
//! drawn at a whole multiple of its size (no blur). The text is drawn by Windows' GDI into a
//! bitmap at the size it is shown, in fonts Windows has (Arial Black, Arial, Consolas); elsewhere
//! the card shows no text. The run's lines come from its standard output and error while the
//! screen shows ([`capture_output`]). Only a backend that owns the game's window draws it:
//! offscreen backends (the shadow of the real one, replays, tests) never do.

use std::collections::VecDeque;
use std::sync::Mutex;

/// What a long step says about itself.
#[derive(Debug, Clone, PartialEq)]
pub struct Progress {
    /// The step's kind ("Loading"), for the log.
    pub title: String,
    /// The step and its count ("Preparing shaders 212 of 1,480").
    pub status: String,
    /// How much is done, 0 to 1; none for a step of no known length, which shows the bar empty.
    pub done: Option<f32>,
}

/// The icon's `size` by `size` 32-bit image: rows top first, B, G, R, A bytes.
fn icon_image(size: u32) -> Option<(u32, u32, Vec<u8>)> {
    const ICO: &[u8] = include_bytes!("../../../viewer/nv-rs.ico");
    let count = u16::from_le_bytes([*ICO.get(4)?, *ICO.get(5)?]) as usize;
    for i in 0..count {
        let e = ICO.get(6 + 16 * i..6 + 16 * i + 16)?;
        // The directory's width byte: 0 is 256.
        let width = if e[0] == 0 { 256 } else { u32::from(e[0]) };
        if width != size {
            continue;
        }
        let len = u32::from_le_bytes(e[8..12].try_into().ok()?) as usize;
        let at = u32::from_le_bytes(e[12..16].try_into().ok()?) as usize;
        let img = ICO.get(at..at + len)?;
        if img.starts_with(b"\x89PNG\r\n\x1a\n") {
            match png_bgra(img, size) {
                Some(out) => return Some(out),
                None => continue,
            }
        }
        // A BITMAPINFOHEADER image (its height counts the mask too), 32 bits.
        let word = |o: usize| u32::from_le_bytes(img[o..o + 4].try_into().expect("4 bytes"));
        if img.len() < 40 || word(0) != 40 || word(4) != size || word(8) != 2 * size {
            continue;
        }
        if u16::from_le_bytes([img[14], img[15]]) != 32 {
            continue;
        }
        let row = size as usize * 4;
        let pixels = img.get(40..40 + row * size as usize)?;
        // Bottom row first in the file.
        let mut out = Vec::with_capacity(pixels.len());
        for r in (0..size as usize).rev() {
            out.extend_from_slice(&pixels[r * row..r * row + row]);
        }
        return Some((size, size, out));
    }
    None
}

/// A PNG of `size` by `size` 8-bit RGBA pixels, not interlaced (what an icon holds), as B, G, R,
/// A rows top first; None for anything else.
fn png_bgra(png: &[u8], size: u32) -> Option<(u32, u32, Vec<u8>)> {
    let (mut at, mut idat, mut header) = (8, Vec::new(), None);
    while at + 12 <= png.len() {
        let len = u32::from_be_bytes(png[at..at + 4].try_into().ok()?) as usize;
        let data = png.get(at + 8..at + 8 + len)?;
        match &png[at + 4..at + 8] {
            b"IHDR" => header = Some(data),
            b"IDAT" => idat.extend_from_slice(data),
            b"IEND" => break,
            _ => {}
        }
        at += 12 + len;
    }
    let h = header?;
    let word = |o: usize| u32::from_be_bytes(h[o..o + 4].try_into().expect("4 bytes"));
    if h.len() < 13 || word(0) != size || word(4) != size || h[8] != 8 || h[9] != 6 || h[12] != 0 {
        return None;
    }
    let raw = esm::inflate::zlib_decompress(&idat, None).ok()?;
    let row = size as usize * 4;
    if raw.len() < (row + 1) * size as usize {
        return None;
    }
    let mut out = vec![0u8; row * size as usize];
    let mut prev = vec![0u8; row];
    for y in 0..size as usize {
        let (filter, src) = (
            raw[y * (row + 1)],
            &raw[y * (row + 1) + 1..(y + 1) * (row + 1)],
        );
        let cur = &mut out[y * row..(y + 1) * row];
        for x in 0..row {
            let a = if x >= 4 { cur[x - 4] } else { 0 };
            let (b, c) = (prev[x], if x >= 4 { prev[x - 4] } else { 0 });
            let p = match filter {
                0 => 0,
                1 => a,
                2 => b,
                3 => ((u16::from(a) + u16::from(b)) / 2) as u8,
                4 => {
                    let p = i16::from(a) + i16::from(b) - i16::from(c);
                    let (pa, pb, pc) = (
                        (p - i16::from(a)).abs(),
                        (p - i16::from(b)).abs(),
                        (p - i16::from(c)).abs(),
                    );
                    if pa <= pb && pa <= pc {
                        a
                    } else if pb <= pc {
                        b
                    } else {
                        c
                    }
                }
                _ => return None,
            };
            cur[x] = src[x].wrapping_add(p);
        }
        prev.copy_from_slice(cur);
    }
    for p in out.chunks_exact_mut(4) {
        p.swap(0, 2);
    }
    Some((size, size, out))
}

/// The logo's images from the icon, largest first, toned to the card's sepia (its ink to its
/// paper by brightness; the alpha kept).
fn logos() -> Vec<(u32, u32, Vec<u8>)> {
    let mut out: Vec<_> = [256, 128].into_iter().filter_map(icon_image).collect();
    for (_, _, px) in &mut out {
        for p in px.chunks_exact_mut(4) {
            let l = (0.114 * f32::from(p[0]) + 0.587 * f32::from(p[1]) + 0.299 * f32::from(p[2]))
                / 255.0;
            for (c, (dark, light)) in p.iter_mut().zip([(0.12, 0.66), (0.16, 0.80), (0.20, 0.87)]) {
                *c = (255.0 * (dark + (light - dark) * l)).round() as u8;
            }
        }
    }
    out
}

/// A GDI font: its face and weight.
type Font = (&'static str, i32);
const BANNER_FONT: Font = ("Arial Black", 900);
const STATUS_FONT: Font = ("Arial", 700);
const LOG_FONT: Font = ("Consolas", 400);

/// `text`'s lines (split at '\n') drawn white on transparent at `height` pixels a line, B, G, R,
/// A rows top first (Windows' GDI); None elsewhere or when GDI fails.
#[cfg(windows)]
fn text_image(text: &str, height: u32, (face, weight): Font) -> Option<(u32, u32, Vec<u8>)> {
    use std::ffi::c_void;
    #[repr(C)]
    struct BitmapInfoHeader {
        size: u32,
        width: i32,
        height: i32,
        planes: u16,
        bit_count: u16,
        compression: u32,
        size_image: u32,
        x_ppm: i32,
        y_ppm: i32,
        clr_used: u32,
        clr_important: u32,
    }
    #[repr(C)]
    struct Size {
        cx: i32,
        cy: i32,
    }
    #[link(name = "gdi32")]
    extern "system" {
        fn CreateCompatibleDC(dc: isize) -> isize;
        fn DeleteDC(dc: isize) -> i32;
        fn CreateDIBSection(
            dc: isize,
            info: *const BitmapInfoHeader,
            usage: u32,
            bits: *mut *mut c_void,
            section: isize,
            offset: u32,
        ) -> isize;
        fn SelectObject(dc: isize, object: isize) -> isize;
        fn DeleteObject(object: isize) -> i32;
        fn CreateFontW(
            height: i32,
            width: i32,
            escapement: i32,
            orientation: i32,
            weight: i32,
            italic: u32,
            underline: u32,
            strike: u32,
            charset: u32,
            precision: u32,
            clip: u32,
            quality: u32,
            pitch: u32,
            face: *const u16,
        ) -> isize;
        fn SetTextColor(dc: isize, colour: u32) -> u32;
        fn SetBkMode(dc: isize, mode: i32) -> i32;
        fn TextOutW(dc: isize, x: i32, y: i32, text: *const u16, len: i32) -> i32;
        fn GetTextExtentPoint32W(dc: isize, text: *const u16, len: i32, size: *mut Size) -> i32;
    }
    let lines: Vec<Vec<u16>> = text
        .split('\n')
        .map(|l| l.encode_utf16().collect())
        .collect();
    let face: Vec<u16> = face.encode_utf16().chain([0]).collect();
    // SAFETY: plain GDI calls on objects made here and deleted before returning; the DIB's bits
    // are read while the bitmap is alive, within its size.
    unsafe {
        let dc = CreateCompatibleDC(0);
        if dc == 0 {
            return None;
        }
        // DEFAULT_CHARSET, ANTIALIASED_QUALITY (grey levels: the alpha below).
        let font = CreateFontW(
            -(height as i32),
            0,
            0,
            0,
            weight,
            0,
            0,
            0,
            1,
            0,
            0,
            4,
            0,
            face.as_ptr(),
        );
        let old_font = SelectObject(dc, font);
        let (mut w, mut line_h) = (1u32, 1u32);
        for l in &lines {
            let mut size = Size { cx: 0, cy: 0 };
            GetTextExtentPoint32W(dc, l.as_ptr(), l.len() as i32, &mut size);
            w = w.max(size.cx.max(1) as u32);
            line_h = line_h.max(size.cy.max(1) as u32);
        }
        let h = line_h * lines.len() as u32;
        let info = BitmapInfoHeader {
            size: 40,
            width: w as i32,
            height: -(h as i32),
            planes: 1,
            bit_count: 32,
            compression: 0,
            size_image: 0,
            x_ppm: 0,
            y_ppm: 0,
            clr_used: 0,
            clr_important: 0,
        };
        let mut bits: *mut c_void = std::ptr::null_mut();
        let bitmap = CreateDIBSection(dc, &info, 0, &mut bits, 0, 0);
        let mut out = None;
        if bitmap != 0 && !bits.is_null() {
            let old_bitmap = SelectObject(dc, bitmap);
            SetBkMode(dc, 1);
            SetTextColor(dc, 0x00FF_FFFF);
            for (i, l) in lines.iter().enumerate() {
                TextOutW(
                    dc,
                    0,
                    (i as u32 * line_h) as i32,
                    l.as_ptr(),
                    l.len() as i32,
                );
            }
            let px = std::slice::from_raw_parts(bits as *const u8, (w * h * 4) as usize);
            // GDI writes the colour, not an alpha: the grey level is the coverage.
            let mut v = Vec::with_capacity(px.len());
            for p in px.chunks_exact(4) {
                let a = p[0].max(p[1]).max(p[2]);
                v.extend_from_slice(&[255, 255, 255, a]);
            }
            out = Some((w, h, v));
            SelectObject(dc, old_bitmap);
            DeleteObject(bitmap);
        }
        SelectObject(dc, old_font);
        DeleteObject(font);
        DeleteDC(dc);
        out
    }
}

#[cfg(not(windows))]
fn text_image(_text: &str, _height: u32, _font: Font) -> Option<(u32, u32, Vec<u8>)> {
    None
}

/// How many of the run's last lines the card shows.
const LOG_LINES: usize = 8;

/// The run's last lines while its output is captured ([`capture_output`]).
static RECENT: Mutex<VecDeque<String>> = Mutex::new(VecDeque::new());

/// Sets the lines the card shows as if the run had printed them (a picture of the card in a
/// test); none clears them.
#[doc(hidden)]
pub fn show_lines(lines: &[&str]) {
    if let Ok(mut r) = RECENT.lock() {
        r.clear();
        r.extend(lines.iter().map(|l| l.to_string()));
    }
}

/// The run's last printed lines, oldest first.
fn recent_lines() -> Vec<String> {
    RECENT
        .lock()
        .map(|r| r.iter().cloned().collect())
        .unwrap_or_default()
}

/// The capture ([`capture_output`]): one pipe for each place the output went (one when standard
/// output and error go to the same file or console, so their lines keep their order).
struct Pipe {
    /// The pipe's write end, standing in for the std handles `which`.
    write: isize,
    which: Vec<u32>,
    /// Where they went before: the reader passes everything on to it.
    original: isize,
    reader: std::thread::JoinHandle<()>,
    done: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

/// The pipes while the output is captured; `true` once the std handles are back.
static CAPTURED: Mutex<Option<(Vec<Pipe>, bool)>> = Mutex::new(None);

/// While it lives (the start screen's [`crate::device::Device`] start owns it) the run's output
/// goes through the capture; dropped, the std handles are their own again ([`release_output`]).
pub(crate) struct Capture(());

impl Drop for Capture {
    fn drop(&mut self) {
        release_output();
    }
}

#[cfg(windows)]
mod win {
    use std::ffi::c_void;
    #[link(name = "kernel32")]
    extern "system" {
        pub fn CreatePipe(
            read: *mut isize,
            write: *mut isize,
            attrs: *const c_void,
            size: u32,
        ) -> i32;
        pub fn GetStdHandle(which: u32) -> isize;
        pub fn SetStdHandle(which: u32, handle: isize) -> i32;
        pub fn ReadFile(h: isize, buf: *mut u8, len: u32, read: *mut u32, ov: *mut c_void) -> i32;
        pub fn WriteFile(
            h: isize,
            buf: *const u8,
            len: u32,
            wrote: *mut u32,
            ov: *mut c_void,
        ) -> i32;
        pub fn CloseHandle(h: isize) -> i32;
        pub fn GetFileType(h: isize) -> u32;
        pub fn GetFileInformationByHandle(h: isize, info: *mut [u32; 13]) -> i32;
    }
    /// `STD_OUTPUT_HANDLE`, `STD_ERROR_HANDLE`.
    pub const OUT: u32 = -11i32 as u32;
    pub const ERR: u32 = -12i32 as u32;

    /// Whether handles `a` and `b` lead to the same place: the same handle, both a console, or
    /// the same file (its volume and index).
    pub fn same_place(a: isize, b: isize) -> bool {
        if a == b {
            return true;
        }
        // SAFETY: plain kernel32 queries on the process's own handles, into a local.
        unsafe {
            let (ta, tb) = (GetFileType(a), GetFileType(b));
            if ta == 2 && tb == 2 {
                return true;
            }
            let (mut ia, mut ib) = ([0u32; 13], [0u32; 13]);
            ta == 1
                && tb == 1
                && GetFileInformationByHandle(a, &mut ia) != 0
                && GetFileInformationByHandle(b, &mut ib) != 0
                && (ia[7], ia[11], ia[12]) == (ib[7], ib[11], ib[12])
        }
    }
}

/// From here until [`release_output`], what the run prints to its standard output and error
/// (Rust's `print!` and `eprint!`, which look the handles up on each write) goes through a pipe,
/// whose thread passes it on unchanged to where it went before (the console, the log file) and
/// keeps the last lines for the card. A panic, and the end of the process
/// (`engine_platform::end_process`), hand everything still in a pipe on first
/// ([`finish_output`]). Windows only; once.
#[cfg(windows)]
pub(crate) fn capture_output() -> Option<Capture> {
    use std::sync::atomic::{AtomicBool, Ordering};
    use win::*;
    let mut captured = CAPTURED.lock().ok()?;
    if captured.is_some() {
        return None;
    }
    // SAFETY: plain kernel32 calls.
    let (out, err) = unsafe { (GetStdHandle(OUT), GetStdHandle(ERR)) };
    let groups: Vec<(Vec<u32>, isize)> = if same_place(out, err) {
        vec![(vec![OUT, ERR], out)]
    } else {
        vec![(vec![OUT], out), (vec![ERR], err)]
    };
    let mut pipes = Vec::new();
    for (which, original) in groups {
        let (mut read, mut write) = (0isize, 0isize);
        // SAFETY: as above; the handles stay open until `finish_output`.
        if unsafe { CreatePipe(&mut read, &mut write, std::ptr::null(), 1 << 16) } == 0 {
            continue;
        }
        let done = std::sync::Arc::new(AtomicBool::new(false));
        let (served, ended) = (which.clone(), done.clone());
        let reader = std::thread::Builder::new()
            .name("nv-rs output".into())
            .spawn(move || {
                let (mut buf, mut line) = ([0u8; 4096], Vec::new());
                loop {
                    let mut n = 0u32;
                    // SAFETY: reads into `buf` within its length; writes what was read.
                    let ok = unsafe {
                        ReadFile(read, buf.as_mut_ptr(), 4096, &mut n, std::ptr::null_mut())
                    };
                    if ok == 0 || n == 0 {
                        break;
                    }
                    let data = &buf[..n as usize];
                    if original != 0 && original != -1 {
                        let mut wrote = 0u32;
                        // SAFETY: as above.
                        unsafe {
                            WriteFile(original, data.as_ptr(), n, &mut wrote, std::ptr::null_mut())
                        };
                    }
                    for &b in data {
                        match b {
                            b'\n' => {
                                let s = String::from_utf8_lossy(&line).into_owned();
                                line.clear();
                                if let Ok(mut r) = RECENT.lock() {
                                    if r.len() == LOG_LINES {
                                        r.pop_front();
                                    }
                                    r.push_back(s);
                                }
                            }
                            b'\r' => {}
                            _ => line.push(b),
                        }
                    }
                }
                // Ended or failed: a std handle still on this pipe goes back to its own, so no
                // print waits on a pipe nobody reads.
                for &w in &served {
                    // SAFETY: as above.
                    unsafe {
                        if GetStdHandle(w) == write {
                            SetStdHandle(w, original);
                        }
                    }
                }
                // SAFETY: the read end, closed once, by its reader.
                unsafe { CloseHandle(read) };
                ended.store(true, Ordering::Release);
            });
        match reader {
            Ok(reader) => {
                for &w in &which {
                    // SAFETY: as above.
                    unsafe { SetStdHandle(w, write) };
                }
                pipes.push(Pipe {
                    write,
                    which,
                    original,
                    reader,
                    done,
                });
            }
            // SAFETY: as above.
            Err(_) => unsafe {
                CloseHandle(read);
                CloseHandle(write);
            },
        }
    }
    *captured = Some((pipes, false));
    drop(captured);
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        finish_output();
        previous(info);
    }));
    Some(Capture(()))
}

#[cfg(not(windows))]
pub(crate) fn capture_output() -> Option<Capture> {
    None
}

/// The standard output and error back to their own handles (the screen is gone); the pipes stay
/// open, so a print that had already looked its handle up still gets passed on.
pub(crate) fn release_output() {
    #[cfg(windows)]
    if let Ok(mut c) = CAPTURED.lock() {
        if let Some((pipes, restored)) = c.as_mut() {
            for p in pipes.iter() {
                for &w in &p.which {
                    // SAFETY: a plain kernel32 call with the handle it had before.
                    unsafe { win::SetStdHandle(w, p.original) };
                }
            }
            *restored = true;
        }
    }
}

/// The capture ended for the end of the process or a panic: the std handles back, each pipe's
/// write end closed so its reader passes on everything left and ends, and the readers waited
/// for (at most about 200 ms in all). Nothing when nothing was captured.
pub fn finish_output() {
    #[cfg(windows)]
    {
        use std::sync::atomic::Ordering;
        release_output();
        let Some((pipes, _)) = CAPTURED.lock().ok().and_then(|mut c| c.take()) else {
            return;
        };
        for p in &pipes {
            // SAFETY: the write end made by `capture_output`, closed once (taken out above).
            unsafe { win::CloseHandle(p.write) };
        }
        let until = std::time::Instant::now() + std::time::Duration::from_millis(200);
        for p in pipes {
            while !p.done.load(Ordering::Acquire) && std::time::Instant::now() < until {
                std::thread::sleep(std::time::Duration::from_millis(1));
            }
            if p.done.load(Ordering::Acquire) {
                let _ = p.reader.join();
            }
        }
    }
}

const SHADER: &str = "
struct Q { rect: vec4<f32>, colour: vec4<f32> }
@group(0) @binding(0) var t: texture_2d<f32>;
@group(0) @binding(1) var s: sampler;
@group(0) @binding(2) var<uniform> q: Q;
@group(0) @binding(3) var<storage, read> gate: array<u32>;
struct V { @builtin(position) p: vec4<f32>, @location(0) uv: vec2<f32> }
@vertex fn vs(@builtin(vertex_index) i: u32) -> V {
    let c = vec2<f32>(f32(i & 1u), f32((i >> 1u) & 1u));
    var v: V;
    v.p = vec4<f32>(q.rect.xy + c * q.rect.zw, 0.0, 1.0);
    // Over a game frame: nothing once the game has shown a picture (gate[0] open), every quad
    // then outside the target.
    if (gate[0] != 0u) {
        v.p = vec4<f32>(2.0, 2.0, 0.0, 1.0);
    }
    v.uv = vec2<f32>(c.x, 1.0 - c.y);
    return v;
}
@fragment fn fs(v: V) -> @location(0) vec4<f32> {
    return textureSample(t, s, v.uv) * q.colour;
}
";

/// The card itself, over the whole target: in units of half the window's shorter side from its
/// centre (`centre`, pixels), the rings (the logo inside the first), the crosshairs with tick
/// marks between the rings, a target in each corner, the banner's band (rows `top` to `bottom`),
/// then a fixed film grain and a vignette. Sepia paper, brown-grey ink.
const CARD: &str = "
struct Card { size: vec2<f32>, centre: vec2<f32>, unit: f32, top: f32, bottom: f32, pad: f32 }
@group(0) @binding(0) var<uniform> c: Card;
@group(0) @binding(1) var<storage, read> gate: array<u32>;
@vertex fn vs(@builtin(vertex_index) i: u32) -> @builtin(position) vec4<f32> {
    if (gate[0] != 0u) {
        return vec4<f32>(2.0, 2.0, 0.0, 1.0);
    }
    return vec4<f32>(f32(i & 1u) * 2.0 - 1.0, f32((i >> 1u) & 1u) * 2.0 - 1.0, 0.0, 1.0);
}
fn ring(d: f32, r: f32, w: f32, px: f32) -> f32 {
    return 1.0 - smoothstep(0.5 * w, 0.5 * w + px, abs(d - r));
}
fn line(d: f32, w: f32, px: f32) -> f32 {
    return 1.0 - smoothstep(0.5 * w, 0.5 * w + px, abs(d));
}
fn grain(p: vec2<f32>) -> f32 {
    var q = fract(p * vec2<f32>(0.1031, 0.1030));
    q = q + dot(q, q.yx + 33.33);
    return fract((q.x + q.y) * q.x);
}
@fragment fn fs(@builtin(position) pos: vec4<f32>) -> @location(0) vec4<f32> {
    let px = 1.0 / c.unit;
    let p = (pos.xy - c.centre) * px;
    let r = length(p);
    var ink = ring(r, 0.36, 0.016, px);
    ink = max(ink, ring(r, 0.42, 0.006, px));
    ink = max(ink, ring(r, 0.64, 0.010, px));
    ink = max(ink, ring(r, 0.86, 0.020, px));
    // The upright arm stops at the banner: the bar and the step's line are under it.
    let above = step(pos.y, c.top);
    ink = max(ink, max(line(p.x, 0.008, px) * above, line(p.y, 0.008, px)) * step(0.36, r));
    let between = step(0.42, r) * step(r, 0.86);
    let tx = abs(fract(p.x / 0.04 + 0.5) - 0.5) * 0.04;
    let ty = abs(fract(p.y / 0.04 + 0.5) - 0.5) * 0.04;
    ink = max(ink, line(tx, 0.006, px) * step(abs(p.y), 0.024) * between);
    ink = max(ink, line(ty, 0.006, px) * step(abs(p.x), 0.024) * between * above);
    let corner = abs((pos.xy - 0.5 * c.size) * px) - (0.5 * c.size * px - vec2<f32>(0.24, 0.24));
    let rc = length(corner);
    ink = max(ink, ring(rc, 0.15, 0.010, px));
    ink = max(ink, ring(rc, 0.08, 0.006, px));
    ink = max(ink, max(line(corner.x, 0.005, px), line(corner.y, 0.005, px)) * step(rc, 0.19));
    let paper = vec3<f32>(0.78, 0.69, 0.53);
    let light = vec3<f32>(0.87, 0.80, 0.66);
    let dark = vec3<f32>(0.20, 0.16, 0.12);
    var col = mix(paper, light, 0.6 * (1.0 - smoothstep(0.0, 0.86, r)));
    // Clear paper under the banner for the bar and the step's line.
    let under = step(c.bottom, pos.y) * step(pos.y, c.bottom + 0.2 * c.unit) * step(abs(p.x), 0.66);
    col = mix(col, dark, ink * (1.0 - under));
    let band = step(c.top, pos.y) * step(pos.y, c.bottom) * step(abs(p.x), 0.98);
    col = mix(col, dark, band);
    col = col + vec3<f32>(0.05 * (grain(floor(pos.xy)) - 0.5));
    let e = (pos.xy / c.size - 0.5) * 2.0;
    col = col * (1.0 - 0.35 * pow(min(length(e) / 1.414, 1.0), 2.2));
    return vec4<f32>(clamp(col, vec3<f32>(0.0), vec3<f32>(1.0)), 1.0);
}
";

/// The drawing of the screen on one device and window format.
pub(crate) struct Screen {
    pipeline: wgpu::RenderPipeline,
    card: wgpu::RenderPipeline,
    /// The window format the pipelines are made for.
    pub(crate) format: wgpu::TextureFormat,
    layout: wgpu::BindGroupLayout,
    card_layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    /// The logo's images, largest first.
    logos: Vec<(u32, u32, wgpu::TextureView)>,
    white: wgpu::TextureView,
    /// The last text drawn and its texture, by line.
    text: Vec<(String, u32, (u32, u32, wgpu::TextureView))>,
    /// The share the bar last showed: it never shows less.
    shown: f32,
    /// A gate that never stops the screen (zero: the game has shown no picture): the screen
    /// drawn on its own.
    ungated: wgpu::Buffer,
}

fn texture(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    (w, h, px): (u32, u32, &[u8]),
) -> wgpu::TextureView {
    let size = wgpu::Extent3d {
        width: w,
        height: h,
        depth_or_array_layers: 1,
    };
    let t = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("loading screen"),
        size,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Bgra8Unorm,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    queue.write_texture(
        t.as_image_copy(),
        px,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(w * 4),
            rows_per_image: Some(h),
        },
        size,
    );
    t.create_view(&Default::default())
}

fn buffer_entry(
    binding: u32,
    visibility: wgpu::ShaderStages,
    ty: wgpu::BufferBindingType,
    min: u64,
) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility,
        ty: wgpu::BindingType::Buffer {
            ty,
            has_dynamic_offset: false,
            min_binding_size: std::num::NonZeroU64::new(min),
        },
        count: None,
    }
}

fn pipeline(
    device: &wgpu::Device,
    source: &str,
    layout: &wgpu::BindGroupLayout,
    format: wgpu::TextureFormat,
) -> wgpu::RenderPipeline {
    let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("loading screen"),
        source: wgpu::ShaderSource::Wgsl(source.into()),
    });
    let pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: None,
        bind_group_layouts: &[Some(layout)],
        immediate_size: 0,
    });
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("loading screen"),
        layout: Some(&pl),
        vertex: wgpu::VertexState {
            module: &module,
            entry_point: Some("vs"),
            compilation_options: Default::default(),
            buffers: &[],
        },
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleStrip,
            ..Default::default()
        },
        depth_stencil: None,
        multisample: Default::default(),
        fragment: Some(wgpu::FragmentState {
            module: &module,
            entry_point: Some("fs"),
            compilation_options: Default::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format,
                blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        multiview_mask: None,
        cache: None,
    })
}

impl Screen {
    pub(crate) fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        format: wgpu::TextureFormat,
    ) -> Screen {
        use wgpu::{BufferBindingType as B, ShaderStages as S};
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("loading screen"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: S::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: S::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                buffer_entry(2, S::VERTEX_FRAGMENT, B::Uniform, 0),
                buffer_entry(3, S::VERTEX, B::Storage { read_only: true }, 4),
            ],
        });
        let card_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("loading screen card"),
            entries: &[
                buffer_entry(0, S::VERTEX_FRAGMENT, B::Uniform, 0),
                buffer_entry(1, S::VERTEX, B::Storage { read_only: true }, 4),
            ],
        });
        // Nearest: the logo is drawn at a whole multiple of its size, the text at its own.
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor::default());
        let logos = logos()
            .into_iter()
            .map(|(w, h, px)| (w, h, texture(device, queue, (w, h, &px))))
            .collect();
        let white = texture(device, queue, (1, 1, &[255, 255, 255, 255]));
        // wgpu gives a new buffer zeros.
        let ungated = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("loading screen gate"),
            size: 4,
            usage: wgpu::BufferUsages::STORAGE,
            mapped_at_creation: false,
        });
        Screen {
            pipeline: pipeline(device, SHADER, &layout, format),
            card: pipeline(device, CARD, &card_layout, format),
            format,
            layout,
            card_layout,
            sampler,
            logos,
            white,
            text: Vec::new(),
            ungated,
            shown: 0.0,
        }
    }

    /// The text line `i` as a texture at `height` pixels, made again only when it changed.
    fn line(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        i: usize,
        text: &str,
        height: u32,
        font: Font,
    ) -> Option<(u32, u32, wgpu::TextureView)> {
        if let Some((t, h, v)) = self.text.get(i) {
            if t == text && *h == height {
                return Some(v.clone());
            }
        }
        let (w, h, px) = text_image(text, height, font)?;
        let v = (w, h, texture(device, queue, (w, h, &px)));
        let e = (text.to_string(), height, v.clone());
        while self.text.len() <= i {
            self.text.push((String::new(), 0, v.clone()));
        }
        self.text[i] = e;
        Some(v)
    }

    /// Draws `p` over all of `target` (`width` by `height` pixels) into `encoder`: on its own; or
    /// with `over` (a gate: non-zero once the game has shown a picture), over the game's frame
    /// already there, and only while the gate is shut.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn draw(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        (width, height): (u32, u32),
        p: &Progress,
        over: Option<&wgpu::Buffer>,
    ) {
        let (fw, fh) = (width.max(1) as f32, height.max(1) as f32);
        // The card's unit (half the shorter side) and centre, the logo's ring around it.
        let unit = fw.min(fh) / 2.0;
        let u = |v: f32| (v * unit).round();
        let centre = [(fw / 2.0).round(), (fh * 0.44).round()];
        let banner = [centre[1] + u(0.46), centre[1] + u(0.62)];
        let paper = [0.87, 0.80, 0.66, 1.0];
        let ink = [0.20, 0.16, 0.12, 1.0];
        let mut quads: Vec<(wgpu::TextureView, [f32; 4], [f32; 4])> = Vec::new();
        // The logo at the largest whole multiple of one of its sizes that fits the ring.
        let room = u(0.66).max(1.0) as u32;
        let logo = self
            .logos
            .iter()
            .find(|(w, _, _)| *w <= room)
            .or(self.logos.last())
            .map(|(w, h, v)| ((room / w).max(1), *w, *h, v.clone()));
        if let Some((scale, w, h, v)) = logo {
            let (w, h) = ((w * scale) as f32, (h * scale) as f32);
            let at = [centre[0] - (w / 2.0).round(), centre[1] - (h / 2.0).round()];
            quads.push((v, [at[0], at[1], w, h], [1.0; 4]));
        }
        let banner_px = u(0.10).max(8.0) as u32;
        if let Some((w, h, v)) =
            self.line(device, queue, 0, "PLEASE STAND BY", banner_px, BANNER_FONT)
        {
            let y = ((banner[0] + banner[1]) / 2.0 - h as f32 / 2.0).round();
            let x = centre[0] - (w as f32 / 2.0).round();
            quads.push((v, [x, y, w as f32, h as f32], paper));
        }
        // The bar under the banner: an ink frame, the paper inside, the ink fill.
        let (bw, bh) = (u(1.1).min(fw - 16.0).max(8.0), u(0.045).max(5.0));
        let (bx, by) = ((fw - bw) / 2.0, banner[1] + u(0.05));
        quads.push((self.white.clone(), [bx, by, bw, bh], ink));
        quads.push((
            self.white.clone(),
            [bx + 2.0, by + 2.0, bw - 4.0, bh - 4.0],
            paper,
        ));
        // Never less than the last frame drew: one bar through the whole start.
        let done = p
            .done
            .map_or(self.shown, |d| d.clamp(0.0, 1.0).max(self.shown));
        self.shown = done;
        if done > 0.0 {
            let fill = ((bw - 6.0) * done).round();
            quads.push((
                self.white.clone(),
                [bx + 3.0, by + 3.0, fill, bh - 6.0],
                ink,
            ));
        }
        let mut y = by + bh + u(0.03);
        let status_px = u(0.05).max(8.0) as u32;
        if let Some((w, h, v)) = self.line(device, queue, 1, &p.status, status_px, STATUS_FONT) {
            let x = centre[0] - (w as f32 / 2.0).round();
            quads.push((v, [x, y.round(), w as f32, h as f32], ink));
            y += h as f32;
        }
        // The run's last lines, in a dark strip along the bottom, as many as fit under the rest.
        let log_px = (fh / 80.0).max(9.0).round() as u32;
        let line_h = (log_px as f32 * 1.2).ceil();
        let fit = ((fh - y - u(0.03)) / line_h).floor().max(0.0) as usize;
        let lines = recent_lines();
        let lines = &lines[lines.len().saturating_sub(fit.min(LOG_LINES))..];
        if !lines.is_empty() {
            let chars = ((fw - 2.0 * u(0.04)) / (log_px as f32 * 0.55)).max(8.0) as usize;
            let text: Vec<String> = lines
                .iter()
                .map(|l| l.chars().take(chars).collect())
                .collect();
            if let Some((w, h, v)) = self.line(device, queue, 2, &text.join("\n"), log_px, LOG_FONT)
            {
                let strip = h as f32 + 2.0 * u(0.015);
                quads.push((
                    self.white.clone(),
                    [0.0, fh - strip, fw, strip],
                    [0.08, 0.07, 0.06, 0.85],
                ));
                quads.push((
                    v,
                    [u(0.04), fh - strip + u(0.015), w as f32, h as f32],
                    [0.93, 0.86, 0.70, 1.0],
                ));
            }
        }
        let gate = over.unwrap_or(&self.ungated);
        let uniform = |device: &wgpu::Device, words: &[f32]| {
            let bytes: Vec<u8> = words.iter().flat_map(|v| v.to_le_bytes()).collect();
            let ub = device.create_buffer(&wgpu::BufferDescriptor {
                label: None,
                size: bytes.len() as u64,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            queue.write_buffer(&ub, 0, &bytes);
            ub
        };
        let card_words = [
            fw, fh, centre[0], centre[1], unit, banner[0], banner[1], 0.0,
        ];
        let card_ub = uniform(device, &card_words);
        let card_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &self.card_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: card_ub.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: gate.as_entire_binding(),
                },
            ],
        });
        let mut groups = Vec::new();
        for (view, [x, y, w, h], colour) in &quads {
            // Pixels to clip space, rounded to whole pixels.
            let rect = [
                x.round() / fw * 2.0 - 1.0,
                1.0 - (y.round() + h) / fh * 2.0,
                w / fw * 2.0,
                h / fh * 2.0,
            ];
            let words: Vec<f32> = rect.iter().chain(colour.iter()).copied().collect();
            let ub = uniform(device, &words);
            groups.push(device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: None,
                layout: &self.layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&self.sampler),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: ub.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: gate.as_entire_binding(),
                    },
                ],
            }));
        }
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("loading screen"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: target,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: if over.is_some() {
                        wgpu::LoadOp::Load
                    } else {
                        wgpu::LoadOp::Clear(wgpu::Color::BLACK)
                    },
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        pass.set_pipeline(&self.card);
        pass.set_bind_group(0, &card_group, &[]);
        pass.draw(0..4, 0..1);
        pass.set_pipeline(&self.pipeline);
        for g in &groups {
            pass.set_bind_group(0, g, &[]);
            pass.draw(0..4, 0..1);
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_icon_holds_the_256_pixel_logo_and_the_128_one() {
        let logos = super::logos();
        let sizes: Vec<(u32, u32, usize)> =
            logos.iter().map(|(w, h, p)| (*w, *h, p.len())).collect();
        assert_eq!(
            sizes,
            [(256, 256, 256 * 256 * 4), (128, 128, 128 * 128 * 4)]
        );
        for (_, _, px) in &logos {
            // Not all transparent: the picture is there.
            assert!(px.chunks_exact(4).filter(|p| p[3] > 0).count() > 1000);
        }
    }

    #[cfg(windows)]
    #[test]
    fn gdi_draws_the_status_line_and_the_log() {
        let (w, h, px) =
            super::text_image("Preparing shaders 212 of 1,480", 24, super::STATUS_FONT)
                .expect("GDI");
        assert!(w > 100 && h >= 20, "{w}x{h}");
        assert!(px.chunks_exact(4).any(|p| p[3] == 255));
        let (_, two, _) = super::text_image("one\ntwo", 12, super::LOG_FONT).expect("GDI");
        let (_, one, _) = super::text_image("one", 12, super::LOG_FONT).expect("GDI");
        assert_eq!(two, 2 * one);
    }

    /// A run that ends while its output is captured loses none of it: the child (this test run
    /// again) captures, prints 2,000 lines of 100 bytes on each stream in turn (more than a pipe holds, so a
    /// pipe is full when it ends) and ends as
    /// `engine_platform::end_process` does; each line reaches its own stream exactly once.
    #[cfg(windows)]
    #[test]
    fn a_run_ending_while_captured_loses_no_line() {
        if std::env::var_os("NV_CAPTURE_CHILD").is_some() {
            let _capture = super::capture_output();
            // The test harness's "test ... " has no line end yet.
            println!();
            let pad = "x".repeat(90);
            for i in 0..2000 {
                println!("out {i} {pad}");
                eprintln!("err {i} {pad}");
            }
            // Last a line of 200,000 bytes on each: most of it still in the pipe at the end.
            println!("{}", "y".repeat(200_000));
            eprintln!("{}", "z".repeat(200_000));
            super::finish_output();
            std::process::exit(0);
        }
        let me = std::env::current_exe().expect("the test binary");
        let out = std::process::Command::new(me)
            .args([
                "loading::tests::a_run_ending_while_captured_loses_no_line",
                "--exact",
                "--nocapture",
                "--test-threads",
                "1",
            ])
            .env("NV_CAPTURE_CHILD", "1")
            .output()
            .expect("the child");
        let (o, e) = (
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr),
        );
        let (y, z) = ("y".repeat(200_000), "z".repeat(200_000));
        assert_eq!(
            o.lines().filter(|l| *l == y).count(),
            1,
            "the last line out"
        );
        assert_eq!(
            e.lines().filter(|l| *l == z).count(),
            1,
            "the last line err"
        );
        let pad = "x".repeat(90);
        for i in 0..2000 {
            let (want_o, want_e) = (format!("out {i} {pad}"), format!("err {i} {pad}"));
            assert_eq!(o.lines().filter(|l| *l == want_o).count(), 1, "{want_o}");
            assert_eq!(e.lines().filter(|l| *l == want_e).count(), 1, "{want_e}");
        }
    }
}
