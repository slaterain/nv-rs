//! Whole levels through D3DX's working colour: decoding, encoding (row by row, or block by
//! block for the DXT formats), the same-size conversion D3DX does when it cannot copy (0x624879)
//! and its box filter for the mip levels it makes (0x624c78).
//!
//! The DXT blocks of a level are independent of each other in D3DX's encoder, so a large level
//! is encoded by several threads, each a band of block rows; every block's bytes are what one
//! thread would make.

use crate::dxt::{self, Color};
use crate::format;
use crate::pixel;

/// One level: format, size and bytes (rows of pixels, or of blocks for DXT, packed).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Level {
    pub format: u32,
    pub width: u32,
    pub height: u32,
    pub bytes: Vec<u8>,
}

/// Whether this crate reads and writes `f`.
pub fn supported(f: u32) -> bool {
    format::is_dxt(f) && f != format::DXT2 && f != format::DXT4 || pixel::supported(f)
}

/// The bytes of one block of the DXT format `f`.
fn block_bytes(f: u32) -> usize {
    if f == format::DXT1 {
        8
    } else {
        16
    }
}

/// One DXT block of `f` to the working colour.
fn decode_block(f: u32, b: &[u8]) -> [Color; 16] {
    match f {
        format::DXT1 => dxt::decode_color(b),
        format::DXT2 | format::DXT3 => dxt::decode_dxt3(b),
        _ => dxt::decode_dxt5(b),
    }
}

/// Block rows `first..first + n` of a DXT level (`w` x `h`) to the working colour: rows
/// `4 first..` of the level, `w` pixels each, into `out` (rows past the level's bottom are
/// left as they are).
fn decode_block_rows(
    f: u32,
    w: usize,
    h: usize,
    bytes: &[u8],
    first: usize,
    n: usize,
    out: &mut [Color],
) {
    let bs = block_bytes(f);
    let bw = w.div_ceil(4);
    for by in first..(first + n).min(h.div_ceil(4)) {
        for bx in 0..bw {
            let o = (by * bw + bx) * bs;
            let px = decode_block(f, &bytes[o..o + bs]);
            for j in 0..4 {
                let y = by * 4 + j;
                if y >= h {
                    break;
                }
                let row = (y - first * 4) * w;
                for i in 0..4 {
                    let x = bx * 4 + i;
                    if x < w {
                        out[row + x] = px[j * 4 + i];
                    }
                }
            }
        }
    }
}

/// A level to the working colour, `width * height` pixels row by row.
pub fn decode(f: u32, w: usize, h: usize, bytes: &[u8]) -> Vec<Color> {
    let mut out = vec![[0.0f32; 4]; w * h];
    if format::is_dxt(f) {
        decode_block_rows(f, w, h, bytes, 0, h.div_ceil(4), &mut out);
    } else {
        let row = format::level_size(f, w as u32, 1);
        for y in 0..h {
            pixel::decode_row(
                f,
                &bytes[y * row..(y + 1) * row],
                w,
                &mut out[y * w..(y + 1) * w],
            );
        }
    }
    out
}

/// The 16 pixels of the block at column `bx` of a band of up to 4 rows (`rows`, `w` pixels
/// each, `vh` of them valid), as D3DX's DXT encoder gathers them: a block reaching past the
/// level's right or bottom edge repeats its pixels (columns, then rows, from the pattern 0, 0,
/// 0, 1).
fn band_block(rows: &[Color], w: usize, vh: usize, bx: usize) -> [Color; 16] {
    const REP: [usize; 4] = [0, 0, 0, 1];
    let mut px = [[0.0f32; 4]; 16];
    let vw = w - bx * 4;
    for j in 0..vh.min(4) {
        for i in 0..vw.min(4) {
            px[j * 4 + i] = rows[j * w + bx * 4 + i];
        }
    }
    if vw < 4 {
        for j in 0..vh.min(4) {
            for i in vw..4 {
                px[j * 4 + i] = px[j * 4 + REP[i]];
            }
        }
    }
    for j in vh.min(4)..4 {
        for i in 0..4 {
            px[j * 4 + i] = px[REP[j] * 4 + i];
        }
    }
    px
}

/// The 16 pixels of block (`bx`, `by`) of a `w` x `h` level ([`band_block`]).
pub fn block_pixels(colors: &[Color], w: usize, h: usize, bx: usize, by: usize) -> [Color; 16] {
    band_block(&colors[by * 4 * w..], w, h - by * 4, bx)
}

/// One DXT block of `f`, into `out`.
fn encode_block_into(f: u32, px: &[Color; 16], dither: bool, out: &mut [u8]) {
    match f {
        format::DXT1 => out.copy_from_slice(&dxt::encode_dxt1(px, dither)),
        format::DXT2 | format::DXT3 => out.copy_from_slice(&dxt::encode_dxt3(px, dither)),
        _ => out.copy_from_slice(&dxt::encode_dxt5(px, dither)),
    }
}

/// One DXT block of `f`.
pub fn encode_block(f: u32, px: &[Color; 16], dither: bool) -> Vec<u8> {
    let mut out = vec![0u8; block_bytes(f)];
    encode_block_into(f, px, dither, &mut out);
    out
}

/// The working colour to a level of `f`: rows, or for DXT blocks ([`block_pixels`]).
/// `dither`: the filter had `D3DX_FILTER_DITHER`.
pub fn encode(f: u32, w: usize, h: usize, colors: &[Color], dither: bool) -> Vec<u8> {
    let mut out = vec![0u8; format::level_size(f, w as u32, h as u32)];
    if format::is_dxt(f) {
        let bs = block_bytes(f);
        let bw = w.div_ceil(4);
        for by in 0..h.div_ceil(4) {
            for bx in 0..bw {
                let px = block_pixels(colors, w, h, bx, by);
                let o = (by * bw + bx) * bs;
                encode_block_into(f, &px, dither, &mut out[o..o + bs]);
            }
        }
    } else {
        let row = format::level_size(f, w as u32, 1);
        for y in 0..h {
            pixel::encode_row(
                f,
                &colors[y * w..(y + 1) * w],
                y as u32,
                dither,
                &mut out[y * row..(y + 1) * row],
            );
        }
    }
    out
}

/// One pixel of the box filter's float path: `(((a + b) + c) + d) * 0.25` per channel, the
/// two pixels of the upper row first (0x624ef2).
#[inline(always)]
fn box4(a: &Color, b: &Color, c: &Color, d: &Color) -> Color {
    std::array::from_fn(|k| (((a[k] + b[k]) + c[k]) + d[k]) * 0.25)
}

/// The box filter's float path for row `y` of the next level: rows `r0`, `r1` of the source
/// (`w` pixels each; the same row for a source 1 high), into `out` (`w2` pixels).
#[inline(always)]
fn box_row(r0: &[Color], r1: &[Color], w: usize, out: &mut [Color]) {
    for (x, o) in out.iter_mut().enumerate() {
        let (x0, x1) = if w == 1 { (0, 0) } else { (2 * x, 2 * x + 1) };
        *o = box4(&r0[x0], &r0[x1], &r1[x0], &r1[x1]);
    }
}

/// Blocks per thread below which a level is not split.
const BLOCKS_PER_THREAD: usize = 256;

/// The most threads one level is encoded by (0: the machine's hardware threads).
static MAX_THREADS: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

/// Caps the threads one level is encoded by (1: the caller's thread only; 0: the machine's
/// hardware threads, the default).
pub fn set_max_threads(n: usize) {
    MAX_THREADS.store(n, std::sync::atomic::Ordering::Relaxed);
}

/// The threads a level of `blocks` blocks is encoded by.
fn threads_for(blocks: usize) -> usize {
    if blocks < 2 * BLOCKS_PER_THREAD {
        return 1;
    }
    let cap = match MAX_THREADS.load(std::sync::atomic::Ordering::Relaxed) {
        0 => std::thread::available_parallelism().map_or(1, |n| n.get()),
        n => n,
    };
    cap.min(blocks / BLOCKS_PER_THREAD).max(1)
}

/// The box filter for a DXT level: band by band of four destination rows, each band from the
/// eight source rows under it, decoded, filtered and encoded; bands split over threads.
fn box_dxt(src: &Level) -> Level {
    let f = src.format;
    let (w, h) = (src.width as usize, src.height as usize);
    let (w2, h2) = ((w / 2).max(1), (h / 2).max(1));
    let bs = block_bytes(f);
    let (bw2, bh2) = (w2.div_ceil(4), h2.div_ceil(4));
    let mut out = vec![0u8; bw2 * bh2 * bs];
    let band_bytes = bw2 * bs;
    let work = |first: usize, out: &mut [u8]| {
        let mut srows = vec![[0.0f32; 4]; 8 * w];
        let mut drows = vec![[0.0f32; 4]; 4 * w2];
        for (k, band) in out.chunks_mut(band_bytes).enumerate() {
            let by = first + k;
            // Source block rows 2 by and 2 by + 1: source rows 8 by .. 8 by + 8.
            decode_block_rows(f, w, h, &src.bytes, 2 * by, 2, &mut srows);
            let vh = (h2 - by * 4).min(4);
            for j in 0..vh {
                let (r0, r1) = if h == 1 { (0, 0) } else { (2 * j, 2 * j + 1) };
                box_row(
                    &srows[r0 * w..(r0 + 1) * w],
                    &srows[r1 * w..(r1 + 1) * w],
                    w,
                    &mut drows[j * w2..(j + 1) * w2],
                );
            }
            for bx in 0..bw2 {
                let px = band_block(&drows, w2, vh, bx);
                encode_block_into(f, &px, false, &mut band[bx * bs..(bx + 1) * bs]);
            }
        }
    };
    let threads = threads_for(bw2 * bh2);
    if threads <= 1 {
        work(0, &mut out);
    } else {
        let per = bh2.div_ceil(threads);
        std::thread::scope(|s| {
            for (i, chunk) in out.chunks_mut(per * band_bytes).enumerate() {
                let work = &work;
                s.spawn(move || work(i * per, chunk));
            }
        });
    }
    Level {
        format: f,
        width: w2 as u32,
        height: h2 as u32,
        bytes: out,
    }
}

/// D3DX's box filter (0x624c78) for the next mip level of `src` (no dither: the mip filter is
/// `D3DX_FILTER_BOX` alone): the integer path for the formats that have one when `src` is at
/// least 2x2, else through the working colour: each pixel
/// `(((r0[2x] + r0[2x+1]) + r1[2x]) + r1[2x+1]) * 0.25`, a side of 1 using its one pixel twice.
pub fn box_next(src: &Level) -> Level {
    let (w, h) = (src.width as usize, src.height as usize);
    let (w2, h2) = ((w / 2).max(1), (h / 2).max(1));
    let f = src.format;
    if format::is_dxt(f) {
        return box_dxt(src);
    }
    if pixel::has_integer_box(f) && w >= 2 && h >= 2 {
        return Level {
            format: f,
            width: w2 as u32,
            height: h2 as u32,
            bytes: pixel::box_integer(f, &src.bytes, w, h),
        };
    }
    let out = box_colors(src);
    Level {
        format: f,
        width: w2 as u32,
        height: h2 as u32,
        bytes: encode(f, w2, h2, &out, false),
    }
}

/// The working colours of the box filter's float path for the next level of `src`.
pub fn box_colors(src: &Level) -> Vec<Color> {
    let (w, h) = (src.width as usize, src.height as usize);
    let (w2, h2) = ((w / 2).max(1), (h / 2).max(1));
    let c = decode(src.format, w, h, &src.bytes);
    let mut out = vec![[0.0f32; 4]; w2 * h2];
    for y in 0..h2 {
        let (r0, r1) = if h == 1 { (0, 0) } else { (2 * y, 2 * y + 1) };
        box_row(
            &c[r0 * w..(r0 + 1) * w],
            &c[r1 * w..(r1 + 1) * w],
            w,
            &mut out[y * w2..(y + 1) * w2],
        );
    }
    out
}

/// What D3DX does with a level of the file it loads into a level of the same size
/// (`D3DXLoadSurfaceFromMemory` with `D3DX_FILTER_TRIANGLE | D3DX_FILTER_DITHER`): the bytes as
/// they are when the formats match and, for DXT, the level is whole blocks (0x624779, 0x6237ae);
/// otherwise through the working colour with dither (0x624879).
pub fn load_same_size(src_format: u32, dst_format: u32, w: u32, h: u32, bytes: &[u8]) -> Vec<u8> {
    if src_format == dst_format && (!format::is_dxt(dst_format) || (w % 4 == 0 && h % 4 == 0)) {
        return bytes.to_vec();
    }
    let c = decode(src_format, w as usize, h as usize, bytes);
    encode(dst_format, w as usize, h as usize, &c, true)
}
