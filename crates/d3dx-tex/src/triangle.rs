//! D3DX's triangle filter, for a level it loads at another size (`D3DXLoadSurfaceFromMemory` with
//! `D3DX_FILTER_TRIANGLE | D3DX_FILTER_DITHER`, its two-dimensional filter 0x626872): the create
//! calls round a side that is not a power of two up to one (and a DXT side up to a multiple of 4),
//! and resample each level the file has into the texture's level of the same index.
//!
//! The filter is separable: per axis a list of weights for each source pixel (0x6234ab: the
//! pixel's triangle, 1 at its centre and 0 a pixel to either side, taken in its two halves, each a
//! pixel wide; each destination pixel a half overlaps gets the overlap times the triangle at the
//! overlap's middle). Each source row is decoded and added, weighted, into the float rows of the
//! destination it reaches; a destination row is clamped to [0, 1] and encoded (with the ordered
//! dither, or a DXT block row once its four rows are there) when its last source row has been
//! added. Without `D3DX_FILTER_MIRROR_*` (a texture) the edges wrap round; with them (D3DX adds
//! them for cube maps) an edge pixel's outer half is cut at the edge and weighs 1 throughout, as if
//! the pixel were mirrored past the edge.
//!
//! Arithmetic: round to nearest throughout. What the DLL stores to floats rounds to a float there
//! (at either precision); what it keeps in registers between stores is at the thread's precision
//! (`P::N`), so the weights and the sums come out differently at 24 and 53 bits.

use crate::dxt::Color;
use crate::filter;
use crate::format;
use crate::num::{Fpu, Reg};
use crate::pixel;

/// A weight at or under this is dropped (0x4064a0).
const EPSILON: f32 = 1e-5;

/// One axis's weights (0x6234ab): for each of the `src` source pixels, the destination pixels
/// (of `dst`) it adds to and with which weight, in the DLL's order. `wrap`: the edges wrap round
/// (no `D3DX_FILTER_MIRROR` flag for the axis); else an edge pixel's outer half is cut at the edge
/// and weighs 1.
pub(crate) fn weights<P: Fpu>(src: usize, dst: usize, wrap: bool) -> Vec<Vec<(usize, f32)>> {
    let n = <P::N as Reg>::ld;
    let (fsrc, fdst) = (src as f32, dst as f32);
    let scale = (n(fdst) / n(fsrc)).st();
    let inv = (n(0.5) / n(scale)).st();
    let mut out = vec![Vec::new(); src];
    // The destination pixel being summed and its sum, carried from one source pixel to the next.
    let (mut cur, mut acc) = (0usize, 0.0f32);
    for (i, list) in out.iter_mut().enumerate() {
        for k in 0..2u32 {
            // Half `k` of the pixel's triangle: `[t, t + 1)` in source units (t = i - 0.5 + k),
            // `[lo, hi)` in destination units.
            let t = ((n(k as f32) + n(i as f32)) - n(0.5)).st();
            let mut lo = (n(t) * n(scale)).st();
            let mut hi = (n(lo) + n(scale)).st();
            if !wrap {
                if 0.0 > lo {
                    lo = 0.0;
                }
                if fdst < hi {
                    hi = fdst;
                }
            }
            // `floor` (0x4f7d20) and the conversion to an integer: exact.
            let mut j = lo.floor() as i64;
            let mut a = j as f32;
            while a < hi {
                let next = (n(a) + n(1.0)).st();
                let d = if j < 0 {
                    j + dst as i64
                } else if j >= dst as i64 {
                    j - dst as i64
                } else {
                    j
                } as usize;
                if d != cur {
                    if EPSILON < acc {
                        list.push((cur, acc));
                    }
                    acc = 0.0;
                    cur = d;
                }
                // The overlap of the half-pixel with destination pixel j: [s, e).
                let s = if lo > a { lo } else { a };
                let e = if next > hi { hi } else { next };
                // (`t` is a half-integer, never NaN: the DLL's "not greater" is "at most".)
                let tri = if !wrap && t < 0.0 {
                    1.0
                } else if !wrap && fsrc <= t + 1.0 {
                    0.0
                } else {
                    (((n(e) + n(s)) * n(inv)) - n(t)).st()
                };
                let v = if k == 0 { n(tri) } else { n(1.0) - n(tri) };
                acc = ((n(e) - n(s)) * v + n(acc)).st();
                j += 1;
                a = j as f32;
            }
        }
        if EPSILON < acc {
            list.push((cur, acc));
        }
        acc = 0.0;
    }
    out
}

/// D3DX's clamp of a finished row of a format of the first kind (every format this crate makes):
/// each channel into [0, 1], a NaN to 0.
fn clamp01(v: f32) -> f32 {
    if 0.0 <= v {
        if 1.0 <= v {
            1.0
        } else {
            v
        }
    } else {
        0.0
    }
}

/// D3DX's triangle filter (0x626872) from a `w` x `h` level of `src_format` (`bytes`, rows packed)
/// into a `dw` x `dh` level of `dst_format`, with its dither, at the thread's precision `P`.
#[allow(clippy::too_many_arguments)]
pub(crate) fn triangle<P: Fpu>(
    src_format: u32,
    w: u32,
    h: u32,
    bytes: &[u8],
    dst_format: u32,
    dw: u32,
    dh: u32,
    wrap: bool,
) -> Vec<u8> {
    let n = <P::N as Reg>::ld;
    let (w, h, dw, dh) = (w as usize, h as usize, dw as usize, dh as usize);
    let wx = weights::<P>(w, dw, wrap);
    let wy = weights::<P>(h, dh, wrap);
    // How many source rows each destination row still waits for.
    let mut pending = vec![0usize; dh];
    for list in &wy {
        for &(dy, _) in list {
            pending[dy] += 1;
        }
    }
    let mut sums: Vec<Option<Vec<Color>>> = vec![None; dh];
    let mut out = vec![0u8; format::level_size(dst_format, dw as u32, dh as u32)];
    let dst_dxt = format::is_dxt(dst_format);
    // DXT: the finished rows of each block row until all of its rows are there.
    let mut finished: Vec<Option<Vec<Color>>> = vec![None; if dst_dxt { dh } else { 0 }];
    let src_dxt = format::is_dxt(src_format);
    let mut band = vec![[0.0f32; 4]; if src_dxt { 4 * w } else { 0 }];
    let mut row = vec![[0.0f32; 4]; w];
    let src_row = format::level_size(src_format, w as u32, 1);
    for (sy, ylist) in wy.iter().enumerate() {
        for &(dy, _) in ylist {
            sums[dy].get_or_insert_with(|| vec![[0.0; 4]; dw]);
        }
        if src_dxt {
            if sy % 4 == 0 {
                filter::decode_block_rows::<P>(src_format, w, h, bytes, sy / 4, 1, &mut band);
            }
            row.copy_from_slice(&band[(sy % 4) * w..(sy % 4 + 1) * w]);
        } else {
            pixel::decode_row(
                src_format,
                &bytes[sy * src_row..(sy + 1) * src_row],
                w,
                &mut row,
            );
        }
        for (c, xlist) in row.iter().zip(&wx) {
            for &(dy, wyv) in ylist {
                let sum = sums[dy].as_mut().expect("allocated for this source row");
                for &(dx, wxv) in xlist {
                    let weight = wxv * wyv;
                    let o = &mut sum[dx];
                    for k in 0..4 {
                        o[k] = (n(o[k]) + n(weight) * n(c[k])).st();
                    }
                }
            }
        }
        for &(dy, _) in ylist {
            pending[dy] -= 1;
            if pending[dy] != 0 {
                continue;
            }
            let mut done = sums[dy].take().expect("a row with its sums");
            for o in done.iter_mut() {
                for v in o.iter_mut() {
                    *v = clamp01(*v);
                }
            }
            if dst_dxt {
                finished[dy] = Some(done);
                encode_block_row::<P>(dst_format, dw, dh, dy / 4, &mut finished, &mut out);
            } else {
                let rb = format::level_size(dst_format, dw as u32, 1);
                pixel::encode_row::<P>(
                    dst_format,
                    &done,
                    dy as u32,
                    true,
                    &mut out[dy * rb..(dy + 1) * rb],
                );
            }
        }
    }
    out
}

/// A DXT destination's block row `by`, once every row of it within the level is finished (the
/// DXT codec 0x61d2bc keeps a block row's rows until then): its blocks with the dither.
fn encode_block_row<P: Fpu>(
    f: u32,
    dw: usize,
    dh: usize,
    by: usize,
    finished: &mut [Option<Vec<Color>>],
    out: &mut [u8],
) {
    let rows = 4 * by..(4 * by + 4).min(dh);
    if finished[rows.clone()].iter().any(Option::is_none) {
        return;
    }
    let vh = rows.len();
    let mut band = vec![[0.0f32; 4]; 4 * dw];
    for (j, y) in rows.clone().enumerate() {
        band[j * dw..(j + 1) * dw].copy_from_slice(finished[y].as_ref().unwrap());
    }
    let bs = filter::block_bytes(f);
    let bw = dw.div_ceil(4);
    for bx in 0..bw {
        let px = filter::band_block(&band, dw, vh, bx);
        let o = (by * bw + bx) * bs;
        filter::encode_block_into::<P>(f, &px, true, &mut out[o..o + bs]);
    }
    for y in rows {
        finished[y] = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::num::P24;

    /// D3DX's clamp of a finished row (0x626872, for a format of the first kind): each channel into
    /// [0, 1], a NaN to 0. The weights are not negative and sum to about 1, so a sum passes 1 by a
    /// rounding at most, and the encoders' own integer clamps hide that in the DLL's answers for the
    /// formats this crate makes: the clamp is pinned here.
    #[test]
    fn a_finished_row_is_clamped_to_0_and_1() {
        assert_eq!(clamp01(1.5), 1.0);
        assert_eq!(clamp01(f32::from_bits(0x3f80_0001)), 1.0);
        assert_eq!(clamp01(1.0), 1.0);
        assert_eq!(clamp01(0.75), 0.75);
        assert_eq!(clamp01(-0.25), 0.0);
        assert_eq!(clamp01(f32::NAN), 0.0);
    }

    /// Every synthetic file D3DX resamples is made (its bytes are checked against the DLL's in
    /// `tests/dll_answers.rs`), at both precisions, texture or cube.
    #[test]
    fn every_resampled_file_is_made() {
        let caps = crate::Caps::default();
        for f in crate::synthetic::npot() {
            for p in [crate::Precision::Single, crate::Precision::Double] {
                let made = if f.cube {
                    crate::create_cube_texture_from_file_in_memory(&f.data, &caps, p)
                } else {
                    crate::create_texture_from_file_in_memory(&f.data, &caps, p)
                };
                assert!(made.is_ok(), "{}: {:?}", f.name, made.err());
            }
        }
    }

    /// The weights of a source pixel sum to about the scale (the destination pixels per source
    /// pixel), and every destination pixel gets some.
    #[test]
    fn the_weights_spread_each_source_pixel_over_the_scale() {
        for (src, dst) in [(24, 32), (3, 4), (48, 64), (1, 2), (7, 8), (5, 4)] {
            for wrap in [true, false] {
                let w = weights::<P24>(src, dst, wrap);
                let scale = dst as f32 / src as f32;
                let mut reached = vec![false; dst];
                for list in &w {
                    let total: f32 = list.iter().map(|&(_, v)| v).sum();
                    assert!((total - scale).abs() < 1e-3, "{src}->{dst} {wrap}: {total}");
                    for &(d, _) in list {
                        reached[d] = true;
                    }
                }
                assert!(reached.iter().all(|&r| r), "{src}->{dst} {wrap}");
            }
        }
    }
}
