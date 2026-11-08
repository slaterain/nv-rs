//! D3DX's DXT block decoder and encoder (`d3dx9_38.dll`: decoders 0x62990d DXT1 colour,
//! 0x629a3d DXT3, 0x629ac7 DXT5; encoders 0x629c16 DXT1, 0x629d52 DXT3, 0x629e94 DXT5, with the
//! colour block encoder 0x628f03, its endpoint search 0x6288d0, the alpha endpoint search
//! 0x62858a and the 5:6:5 conversions 0x628493 / 0x627a50).
//!
//! A block is 16 pixels, row by row, each D3DX's working colour: red, green, blue, alpha as
//! floats. Arithmetic rounds to nearest `f32`, except in the loops where the DLL truncates
//! ([`Z`]; see [`crate::num`]).
// The comparisons keep the DLL's NaN behaviour: `!(a < b)` is not `a >= b` for a NaN.
#![allow(clippy::neg_cmp_op_on_partial_ord)]

use crate::num::{
    fistp, Rz, INV15, INV255, INV31, INV5, INV63, INV7, THIRD, TWO_THIRDS, W1, W3, W5, W7, Z,
};

/// D3DX's working colour: red, green, blue, alpha.
pub type Color = [f32; 4];

/// The weights D3DX gives red and blue against green while it fits a colour block (0.2125 and
/// 0.0721 over 0.7154; the DLL's floats at 0x787e44..) and their inverses (0x787e54..).
#[inline(always)]
fn lum() -> [f32; 3] {
    [
        f32::from_bits(0x3e98_1530),
        1.0,
        f32::from_bits(0x3dce_6734),
    ]
}
#[inline(always)]
fn lum_inv() -> [f32; 3] {
    [
        f32::from_bits(0x4057_762e),
        1.0,
        f32::from_bits(0x411e_c1dd),
    ]
}

/// 0x627a50: a 5:6:5 colour to the working colour.
#[inline]
pub fn decode565(w: u16) -> Color {
    let w = u32::from(w);
    [
        ((w >> 11) & 31) as f32 * INV31,
        ((w >> 5) & 63) as f32 * INV63,
        (w & 31) as f32 * INV31,
        1.0,
    ]
}

/// The clamp of 0x628493 and 0x62858a: below 0 gives 0, above 1 gives 1, anything else (a NaN
/// too) is kept.
#[inline(always)]
fn clamp01(x: f32) -> f32 {
    x.clamp(0.0, 1.0)
}

/// `trunc(x * scale + 0.5)` with truncating arithmetic: the encoders' quantization (inside an
/// [`Rz`] region).
#[inline(always)]
fn quant(x: Z, scale: f32) -> i32 {
    fistp((x * Z(scale) + Z(0.5)).0)
}

/// [`quant`] of three values in their own truncating region (0x62851c to 0x628576, and the
/// DXT5 endpoints' 0x62a06e to 0x62a0ad).
#[inline(never)]
fn quant3(v: [f32; 3], scale: [f32; 3]) -> [i32; 3] {
    let _rz = Rz::new();
    [
        quant(Z(v[0]), scale[0]),
        quant(Z(v[1]), scale[1]),
        quant(Z(v[2]), scale[2]),
    ]
}

/// 0x628493: the working colour to 5:6:5.
#[inline]
pub fn encode565(c: &[f32; 3]) -> u16 {
    let [r, g, b] = quant3(
        [clamp01(c[0]), clamp01(c[1]), clamp01(c[2])],
        [31.0, 63.0, 31.0],
    );
    (((r << 6 | g) << 5) | b) as u16
}

/// `c0 + (c1 - c0) * t` for each channel, as the decoder and encoder make their palettes.
#[inline(always)]
fn lerp(c0: &[f32], c1: &[f32], t: f32) -> [f32; 4] {
    let mut o = [0.0f32; 4];
    for (k, v) in o.iter_mut().enumerate().take(c0.len()) {
        *v = c0[k] + (c1[k] - c0[k]) * t;
    }
    o
}

/// 0x62990d: a DXT1 colour block (also the colour half of DXT2..5: D3DX decodes those with the
/// DXT1 rule, three colours and transparent black when the first endpoint is not the larger).
pub fn decode_color(b: &[u8]) -> [Color; 16] {
    let w0 = u16::from_le_bytes([b[0], b[1]]);
    let w1 = u16::from_le_bytes([b[2], b[3]]);
    let c0 = decode565(w0);
    let c1 = decode565(w1);
    let mut pal = [c0, c1, [0.0; 4], [0.0; 4]];
    if w0 > w1 {
        pal[2] = lerp(&c0, &c1, THIRD);
        pal[3] = lerp(&c0, &c1, TWO_THIRDS);
    } else {
        pal[2] = lerp(&c0, &c1, 0.5);
    }
    let idx = u32::from_le_bytes([b[4], b[5], b[6], b[7]]);
    std::array::from_fn(|i| pal[((idx >> (2 * i)) & 3) as usize])
}

/// 0x629a3d: a DXT2/DXT3 block (explicit 4-bit alpha, then the colour block).
pub fn decode_dxt3(b: &[u8]) -> [Color; 16] {
    let mut px = decode_color(&b[8..16]);
    for half in 0..2 {
        let mut a = u32::from_le_bytes(b[half * 4..half * 4 + 4].try_into().unwrap());
        for i in 0..8 {
            px[half * 8 + i][3] = (a & 0xf) as f32 * INV15;
            a >>= 4;
        }
    }
    px
}

/// The eight alpha values of a DXT4/DXT5 alpha block (0x629ac7), also the encoder's steps:
/// `s[i + 1] = (i s1 + (n - 1 - i) s0) / (n - 1)` (times the DLL's 1/7 or 1/5), then for six
/// steps 0 and 1.
fn alpha_steps(s0: f32, s1: f32, n: u32) -> [f32; 8] {
    let mut s = [0.0f32; 8];
    s[0] = s0;
    s[1] = s1;
    let inv = if n == 8 { INV7 } else { INV5 };
    for i in 1..n - 1 {
        s[i as usize + 1] = (i as f32 * s1 + (n - 1 - i) as f32 * s0) * inv;
    }
    if n == 6 {
        s[6] = 0.0;
        s[7] = 1.0;
    }
    s
}

/// 0x629ac7: a DXT4/DXT5 block (interpolated alpha, then the colour block).
pub fn decode_dxt5(b: &[u8]) -> [Color; 16] {
    let mut px = decode_color(&b[8..16]);
    let a0 = f32::from(b[0]) * INV255;
    let a1 = INV255 * f32::from(b[1]);
    let pal = alpha_steps(a0, a1, if b[1] < b[0] { 8 } else { 6 });
    for half in 0..2 {
        let o = 2 + half * 3;
        let mut bits = u32::from(b[o]) | u32::from(b[o + 1]) << 8 | u32::from(b[o + 2]) << 16;
        for i in 0..8 {
            px[half * 8 + i][3] = pal[(bits & 7) as usize];
            bits >>= 3;
        }
    }
    px
}

/// The colour lookup tables of the endpoint search (0x4080d4.., 0x4080f4..): for three steps
/// `pC3`, `pD3`; for four `pC4`, `pD4`.
const PC3: [f32; 3] = [1.0, 0.5, 0.0];
const PD3: [f32; 3] = [0.0, 0.5, 1.0];
const PC4: [f32; 4] = [1.0, 2.0 / 3.0, 1.0 / 3.0, 0.0];
const PD4: [f32; 4] = [0.0, 1.0 / 3.0, 2.0 / 3.0, 1.0];

/// 0x6288d0: the colour endpoints of a block of quantized, weighted colours (D3DX's search: the
/// box's corners, the diagonal that fits best, then up to eight Newton steps with truncating
/// arithmetic). `steps` is 3 or 4.
pub fn optimize_rgb(points: &[Color; 16], steps: usize) -> ([f32; 3], [f32; 3]) {
    let mut x = lum();
    let mut y = [0.0f32; 3];
    for p in points {
        for c in 0..3 {
            if p[c] < x[c] {
                x[c] = p[c];
            }
            if y[c] < p[c] {
                y[c] = p[c];
            }
        }
    }
    let ab = [y[0] - x[0], y[1] - x[1], y[2] - x[2]];
    let f_ab = ab[2] * ab[2] + (ab[0] * ab[0] + ab[1] * ab[1]);
    if f_ab < f32::MIN_POSITIVE {
        return (x, y);
    }
    let inv = 1.0 / f_ab;
    let dir = [inv * ab[0], inv * ab[1], inv * ab[2]];
    let mid: [f32; 3] = std::array::from_fn(|c| (y[c] + x[c]) * 0.5);
    let mut f_dir = [0.0f32; 4];
    for p in points {
        let pt: [f32; 3] = std::array::from_fn(|c| (p[c] - mid[c]) * dir[c]);
        let gr = pt[1] + pt[0];
        let rg = pt[0] - pt[1];
        let f = [pt[2] + gr, gr - pt[2], pt[2] + rg, rg - pt[2]];
        for k in 0..4 {
            f_dir[k] += f[k] * f[k];
        }
    }
    let mut best = f_dir[0];
    let mut i_best = 0;
    for (i, &v) in f_dir.iter().enumerate().skip(1) {
        if best < v {
            best = v;
            i_best = i;
        }
    }
    if i_best & 2 != 0 {
        std::mem::swap(&mut x[1], &mut y[1]);
    }
    if i_best & 1 != 0 {
        std::mem::swap(&mut x[2], &mut y[2]);
    }
    if f_ab < 1.0 / 4096.0 {
        return (x, y);
    }
    newton_rgb(points, x, y, steps)
}

/// The Newton steps of the endpoint search (truncating, 0x628bb8 to 0x628ec9).
#[inline(never)]
fn newton_rgb(
    points: &[Color; 16],
    x: [f32; 3],
    y: [f32; 3],
    steps: usize,
) -> ([f32; 3], [f32; 3]) {
    let _rz = Rz::new();
    let (pc, pd): (&[f32], &[f32]) = if steps == 3 {
        (&PC3, &PD3)
    } else {
        (&PC4, &PD4)
    };
    let f_steps = Z((steps - 1) as f32);
    let mut x = x.map(Z);
    let mut y = y.map(Z);
    for _ in 0..8 {
        let mut s = [[Z(0.0); 3]; 4];
        for i in 0..steps {
            for c in 0..3 {
                s[i][c] = y[c] * Z(pd[i]) + Z(pc[i]) * x[c];
            }
        }
        let dv = [y[0] - x[0], y[1] - x[1], y[2] - x[2]];
        let len = dv[2] * dv[2] + (dv[0] * dv[0] + dv[1] * dv[1]);
        if len.0 < 1.0 / 4096.0 {
            break;
        }
        let scale = f_steps / len;
        let dir = [scale * dv[0], scale * dv[1], scale * dv[2]];
        let (mut d2x, mut d2y) = (Z(0.0), Z(0.0));
        let (mut dx, mut dy) = ([Z(0.0); 3], [Z(0.0); 3]);
        for p in points {
            let p = [Z(p[0]), Z(p[1]), Z(p[2])];
            let dot = (p[2] - x[2]) * dir[2] + ((p[0] - x[0]) * dir[0] + (p[1] - x[1]) * dir[1]);
            let i = if f_steps.0 > dot.0 {
                fistp((dot + Z(0.5)).0)
            } else {
                steps as i32 - 1
            };
            let i = step_index(i);
            let diff = [s[i][0] - p[0], s[i][1] - p[1], s[i][2] - p[2]];
            let fc = Z(pc[i]) * Z(0.125);
            let fd = Z(0.125) * Z(pd[i]);
            d2x = d2x + fc * Z(pc[i]);
            for c in 0..3 {
                dx[c] = fc * diff[c] + dx[c];
            }
            d2y = fd * Z(pd[i]) + d2y;
            for c in 0..3 {
                dy[c] = fd * diff[c] + dy[c];
            }
        }
        if 0.0 < d2x.0 {
            let f = Z(-1.0) / d2x;
            for c in 0..3 {
                x[c] = f * dx[c] + x[c];
            }
        }
        if 0.0 < d2y.0 {
            let f = Z(-1.0) / d2y;
            for c in 0..3 {
                y[c] = f * dy[c] + y[c];
            }
        }
        const EPS: f32 = (0.25 / 64.0) * (0.25 / 64.0);
        if dx.iter().chain(dy.iter()).all(|&v| (v * v).0 < EPS) {
            break;
        }
    }
    (x.map(|v| v.0), y.map(|v| v.0))
}

/// Counts the step indices the endpoint search's Newton loop takes below 0 (a point projecting
/// more than half a step before the first endpoint). The DLL then reads memory before its tables;
/// none of the game's textures does this (the verification tool checks the count stays 0).
pub static NEGATIVE_STEPS: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

#[inline(always)]
fn step_index(i: i32) -> usize {
    if i < 0 {
        NEGATIVE_STEPS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        0
    } else {
        i as usize
    }
}

/// Floyd-Steinberg error diffusion within a 4x4 block (the DLL's encoders, pixel by pixel in
/// row order, truncating): `err[i + k] = err[i + k] + e * w`.
#[inline(always)]
fn diffuse<const N: usize>(err: &mut [[Z; N]; 16], i: usize, e: [Z; N]) {
    let mut add = |k: usize, w: f32| {
        for c in 0..N {
            err[k][c] = err[k][c] + e[c] * Z(w);
        }
    };
    if i & 3 != 3 {
        add(i + 1, W7);
    }
    if i < 12 {
        if i & 3 != 0 {
            add(i + 3, W3);
        }
        add(i + 4, W5);
        if i & 3 != 3 {
            add(i + 5, W1);
        }
    }
}

/// The quantization of a colour block to 5:6:5 with error diffusion when dithering
/// (truncating, 0x628f94 to 0x6292c8): each pixel's quantized colour weighted by `lum`, alpha 1.
#[inline(never)]
fn quantize_colors(c: &[Color; 16], dither: bool, lum: [f32; 3]) -> [Color; 16] {
    let _rz = Rz::new();
    let zl = lum.map(Z);
    let mut err = [[Z(0.0); 3]; 16];
    let mut color = [[0.0f32; 4]; 16];
    for i in 0..16 {
        let mut v = [Z(c[i][0]), Z(c[i][1]), Z(c[i][2])];
        if dither {
            for k in 0..3 {
                v[k] = err[i][k] + v[k];
            }
        }
        let q = [
            Z(quant(v[0], 31.0) as f32) * Z(INV31),
            Z(quant(v[1], 63.0) as f32) * Z(INV63),
            Z(quant(v[2], 31.0) as f32) * Z(INV31),
        ];
        if dither {
            diffuse(&mut err, i, [v[0] - q[0], v[1] - q[1], v[2] - q[2]]);
        }
        color[i] = [(q[0] * zl[0]).0, (q[1] * zl[1]).0, (q[2] * zl[2]).0, 1.0];
    }
    color
}

/// What the colour block's index loop needs: the palette (`s`, weighted), its first colour,
/// the direction scaled to steps, the step count and its index table.
struct Palette<'a> {
    s: [[f32; 4]; 4],
    dir: [f32; 3],
    steps: usize,
    p_steps: &'a [u32],
}

/// The colour block's indices (truncating, 0x629636 to 0x6298f3).
#[inline(never)]
fn color_indices(c: &[Color; 16], pal: &Palette, dither: bool, lum: [f32; 3]) -> u32 {
    let _rz = Rz::new();
    let zl = lum.map(Z);
    let s0 = [Z(pal.s[0][0]), Z(pal.s[0][1]), Z(pal.s[0][2])];
    let dir = pal.dir.map(Z);
    let f_steps = (pal.steps - 1) as f32;
    let mut err = [[Z(0.0); 3]; 16];
    let mut bitmap = 0u32;
    for i in 0..16 {
        if pal.steps == 3 && c[i][3] < 0.5 {
            bitmap = (bitmap >> 2) | (3 << 30);
            continue;
        }
        let mut cl = [Z(c[i][0]) * zl[0], zl[1] * Z(c[i][1]), Z(c[i][2]) * zl[2]];
        if dither {
            cl = [cl[0] + err[i][0], err[i][1] + cl[1], err[i][2] + cl[2]];
        }
        let dot = (cl[2] - s0[2]) * dir[2] + ((cl[0] - s0[0]) * dir[0] + (cl[1] - s0[1]) * dir[1]);
        let step = if !(0.0 < dot.0) {
            0
        } else if !(f_steps > dot.0) {
            1
        } else {
            pal.p_steps[fistp((dot + Z(0.5)).0) as usize]
        };
        bitmap = (step << 30) | (bitmap >> 2);
        if dither {
            let t = &pal.s[step as usize];
            // The quantized colour's alpha, which is 1.
            let w = Z(1.0);
            let e: [Z; 3] = std::array::from_fn(|k| (cl[k] - Z(t[k])) * w);
            diffuse(&mut err, i, e);
        }
    }
    bitmap
}

/// 0x628f03: a colour block. `dxt1`: a DXT1 block, which may use three colours and transparent
/// black for pixels with alpha below 0.5; `dither`: D3DX's error diffusion (filters with
/// `D3DX_FILTER_DITHER`).
pub fn encode_color(c: &[Color; 16], dxt1: bool, dither: bool) -> [u8; 8] {
    let mut steps = 4usize;
    if dxt1 {
        let n = c.iter().filter(|p| p[3] < 0.5).count();
        if n == 16 {
            return [0, 0, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff];
        }
        if n > 0 {
            steps = 3;
        }
    }
    let lum = lum();
    let color = quantize_colors(c, dither, lum);
    let (a, b) = optimize_rgb(&color, steps);
    let inv = lum_inv();
    let wa = encode565(&[inv[0] * a[0], inv[1] * a[1], inv[2] * a[2]]);
    let wb = encode565(&[b[0] * inv[0], inv[1] * b[1], inv[2] * b[2]]);
    if steps == 4 && wa == wb {
        let mut out = [0u8; 8];
        out[0..2].copy_from_slice(&wa.to_le_bytes());
        out[2..4].copy_from_slice(&wb.to_le_bytes());
        return out;
    }
    let ca = decode565(wa);
    let cb = decode565(wb);
    let a = [lum[0] * ca[0], lum[1] * ca[1], lum[2] * ca[2]];
    let b = [lum[0] * cb[0], lum[1] * cb[1], lum[2] * cb[2]];
    let (w0, w1, s0, s1) = if (steps == 3) == (wa <= wb) {
        (wa, wb, a, b)
    } else {
        (wb, wa, b, a)
    };
    let mut s = [[0.0f32; 4]; 4];
    s[0] = [s0[0], s0[1], s0[2], 0.0];
    s[1] = [s1[0], s1[1], s1[2], 0.0];
    let p_steps: &[u32] = if steps == 3 {
        s[2] = lerp(&s0, &s1, 0.5);
        &[0, 2, 1]
    } else {
        s[2] = lerp(&s0, &s1, THIRD);
        s[3] = lerp(&s0, &s1, TWO_THIRDS);
        &[0, 2, 3, 1]
    };
    let dv = [s1[0] - s0[0], s1[1] - s0[1], s1[2] - s0[2]];
    let f_steps = (steps - 1) as f32;
    let scale = if wa == wb {
        0.0
    } else {
        f_steps / ((dv[0] * dv[0] + dv[1] * dv[1]) + dv[2] * dv[2])
    };
    let pal = Palette {
        s,
        dir: [scale * dv[0], scale * dv[1], scale * dv[2]],
        steps,
        p_steps,
    };
    let bitmap = color_indices(c, &pal, dither, lum);
    let mut out = [0u8; 8];
    out[0..2].copy_from_slice(&w0.to_le_bytes());
    out[2..4].copy_from_slice(&w1.to_le_bytes());
    out[4..8].copy_from_slice(&bitmap.to_le_bytes());
    out
}

/// The DXT1 encoder's alpha dither: each alpha to 0 or 1 with error diffusion (truncating,
/// 0x629c4c to 0x629d25).
#[inline(never)]
fn dither_alpha_dxt1(c: &[Color; 16]) -> [Color; 16] {
    let _rz = Rz::new();
    let mut q = *c;
    let mut err = [[Z(0.0); 1]; 16];
    for i in 0..16 {
        let a = err[i][0] + Z(c[i][3]);
        let v = Z(fistp((a + Z(0.5)).0) as f32);
        q[i][3] = v.0;
        diffuse(&mut err, i, [a - v]);
    }
    q
}

/// 0x629c16: a DXT1 block. With dither the alpha is first quantized to 0 or 1 with error
/// diffusion.
pub fn encode_dxt1(c: &[Color; 16], dither: bool) -> [u8; 8] {
    if !dither {
        return encode_color(c, true, false);
    }
    encode_color(&dither_alpha_dxt1(c), true, true)
}

/// The DXT3 encoder's 4-bit alphas (truncating, 0x629d89 to 0x629e75).
#[inline(never)]
fn alpha_dxt3(c: &[Color; 16], dither: bool) -> [u32; 2] {
    let _rz = Rz::new();
    let mut dw = [0u32; 2];
    let mut err = [[Z(0.0); 1]; 16];
    for i in 0..16 {
        let mut a = Z(c[i][3]);
        if dither {
            a = err[i][0] + a;
        }
        let q = quant(a, 15.0);
        dw[i >> 3] = (q as u32).wrapping_shl(28) | (dw[i >> 3] >> 4);
        if dither {
            // FILD of the unsigned value: a negative one gets 2^32 added.
            let qf = if q < 0 {
                Z(q as f32) + Z(4_294_967_296.0)
            } else {
                Z(q as f32)
            };
            diffuse(&mut err, i, [a - qf * Z(INV15)]);
        }
    }
    dw
}

/// 0x629d52: a DXT2/DXT3 block: explicit 4-bit alpha (with error diffusion when dithering), then
/// the colour block.
pub fn encode_dxt3(c: &[Color; 16], dither: bool) -> [u8; 16] {
    let dw = alpha_dxt3(c, dither);
    let mut out = [0u8; 16];
    out[0..4].copy_from_slice(&dw[0].to_le_bytes());
    out[4..8].copy_from_slice(&dw[1].to_le_bytes());
    out[8..16].copy_from_slice(&encode_color(c, false, dither));
    out
}

/// The alpha lookup tables of 0x62858a: `pC6`/`pD6` (0x4080b8, 0x4080a0) and `pC8`/`pD8`
/// (0x408080, 0x408060). `pD8`'s last value is 8/7 in the DLL, not 1.
fn alpha_tables(steps: usize) -> ([f32; 8], [f32; 8]) {
    if steps == 6 {
        (
            [1.0, 0.8, 0.6, 0.4, 0.2, 0.0, 0.0, 0.0],
            [0.0, 0.2, 0.4, 0.6, 0.8, 1.0, 0.0, 0.0],
        )
    } else {
        (
            [
                1.0,
                6.0 / 7.0,
                5.0 / 7.0,
                4.0 / 7.0,
                3.0 / 7.0,
                2.0 / 7.0,
                1.0 / 7.0,
                0.0,
            ],
            [
                0.0,
                1.0 / 7.0,
                2.0 / 7.0,
                3.0 / 7.0,
                4.0 / 7.0,
                5.0 / 7.0,
                6.0 / 7.0,
                8.0 / 7.0,
            ],
        )
    }
}

/// 0x62858a: the alpha endpoints of a block of quantized alphas, `steps` 6 or 8.
pub fn optimize_alpha(points: &[f32; 16], steps: usize) -> (f32, f32) {
    let mut x = 1.0f32;
    let mut y = 0.0f32;
    if steps == 8 {
        for &p in points {
            if p < x {
                x = p;
            }
            if y < p {
                y = p;
            }
        }
    } else {
        for &p in points {
            if p < x && 0.0 < p {
                x = p;
            }
            if y < p && p < 1.0 {
                y = p;
            }
        }
        if x == y {
            y = 1.0;
        }
    }
    let (x, y) = newton_alpha(points, x, y, steps);
    (clamp01(x), clamp01(y))
}

/// The Newton steps of the alpha endpoint search (truncating, 0x628680 to 0x628864).
#[inline(never)]
fn newton_alpha(points: &[f32; 16], x: f32, y: f32, steps: usize) -> (f32, f32) {
    let _rz = Rz::new();
    let (pc, pd) = alpha_tables(steps);
    let f_steps = Z((steps - 1) as f32);
    let (mut x, mut y) = (Z(x), Z(y));
    for _ in 0..8 {
        if (y - x).0 < 1.0 / 256.0 {
            break;
        }
        let scale = f_steps / (y - x);
        let mut s = [Z(0.0); 8];
        for i in 0..steps {
            s[i] = Z(pc[i]) * x + Z(pd[i]) * y;
        }
        if steps == 6 {
            s[6] = Z(0.0);
            s[7] = Z(1.0);
        }
        let (mut dx, mut dy, mut d2x, mut d2y) = (Z(0.0), Z(0.0), Z(0.0), Z(0.0));
        let lo = x * Z(0.5);
        let hi = (y + Z(1.0)) * Z(0.5);
        for &p in points {
            let p = Z(p);
            let dot = (p - x) * scale;
            let i = if !(0.0 < dot.0) {
                if steps == 6 && !(lo.0 < p.0) {
                    continue;
                }
                0
            } else if f_steps.0 > dot.0 {
                fistp((dot + Z(0.5)).0) as usize
            } else {
                if steps == 6 && hi.0 <= p.0 {
                    continue;
                }
                steps - 1
            };
            if i < steps {
                let diff = p - s[i];
                dx = diff * Z(pc[i]) + dx;
                d2x = Z(pc[i]) * Z(pc[i]) + d2x;
                dy = Z(pd[i]) * diff + dy;
                d2y = Z(pd[i]) * Z(pd[i]) + d2y;
            }
        }
        if 0.0 < d2x.0 {
            x = x - dx / d2x;
        }
        if 0.0 < d2y.0 {
            y = y - dy / d2y;
        }
        if y.0 < x.0 {
            std::mem::swap(&mut x, &mut y);
        }
        if (dx * dx).0 < 1.0 / 64.0 && (dy * dy).0 < 1.0 / 64.0 {
            break;
        }
    }
    (x.0, y.0)
}

/// The DXT5 encoder's alphas to 8 bits (truncating, 0x629edc to 0x629fd8): the quantized
/// alphas and their smallest and largest (both starting from the first pixel's alpha as given).
#[inline(never)]
fn alpha_dxt5(c: &[Color; 16], dither: bool) -> ([f32; 16], f32, f32) {
    let _rz = Rz::new();
    let mut lo = c[0][3];
    let mut hi = c[0][3];
    let mut q = [0.0f32; 16];
    let mut err = [[Z(0.0); 1]; 16];
    for i in 0..16 {
        let mut a = Z(c[i][3]);
        if dither {
            a = err[i][0] + a;
        }
        let v = Z(quant(a, 255.0) as f32) * Z(INV255);
        q[i] = v.0;
        if v.0 < lo {
            lo = v.0;
        } else if hi < v.0 {
            hi = v.0;
        }
        if dither {
            diffuse(&mut err, i, [a - v]);
        }
    }
    (q, lo, hi)
}

/// The DXT5 encoder's alpha indices (truncating, 0x62a208 to 0x62a38e): two 24-bit words.
#[inline(never)]
fn alpha_indices_dxt5(
    c: &[Color; 16],
    s: &[f32; 8],
    scale: f32,
    steps: u32,
    p_steps: &[u32],
    dither: bool,
) -> [u32; 2] {
    let _rz = Rz::new();
    let f_steps = (steps - 1) as f32;
    let scale = Z(scale);
    let (s0, lo_t, hi_t) = (Z(s[0]), Z(s[0]) * Z(0.5), (Z(s[1]) + Z(1.0)) * Z(0.5));
    let mut err = [[Z(0.0); 1]; 16];
    let mut out = [0u32; 2];
    for (set, word) in out.iter_mut().enumerate() {
        let mut dw = 0u32;
        for i in set * 8..set * 8 + 8 {
            let mut a = Z(c[i][3]);
            if dither {
                a = err[i][0] + a;
            }
            let dot = (a - s0) * scale;
            let step = if !(0.0 < dot.0) {
                if steps == 6 && !(lo_t.0 < a.0) {
                    6
                } else {
                    0
                }
            } else if f_steps > dot.0 {
                p_steps[fistp((dot + Z(0.5)).0) as usize]
            } else if steps == 6 && hi_t.0 <= a.0 {
                7
            } else {
                1
            };
            dw = (dw >> 3) | (step << 21);
            if dither {
                diffuse(&mut err, i, [a - Z(s[step as usize])]);
            }
        }
        *word = dw;
    }
    out
}

/// 0x629e94: a DXT4/DXT5 block: interpolated alpha, then the colour block.
pub fn encode_dxt5(c: &[Color; 16], dither: bool) -> [u8; 16] {
    let mut out = [0u8; 16];
    let (q, lo, hi) = alpha_dxt5(c, dither);
    out[8..16].copy_from_slice(&encode_color(c, false, dither));
    if lo == 1.0 {
        out[0] = 0xff;
        out[1] = 0xff;
        return out;
    }
    let steps: u32 = if lo == 0.0 || hi == 1.0 { 6 } else { 8 };
    let (fa, fb) = optimize_alpha(&q, steps as usize);
    let [ba, bb, _] = quant3([fa, fb, 0.0], [255.0, 255.0, 0.0]);
    let fa = (ba & 0xff) as f32 * INV255;
    let fb = (bb & 0xff) as f32 * INV255;
    let (ca, cb) = (ba as u8, bb as u8);
    let (s, p_steps): ([f32; 8], &[u32]) = if steps == 8 {
        if ca == cb {
            out[0] = ca;
            out[1] = cb;
            return out;
        }
        out[1] = ca;
        out[0] = cb;
        (alpha_steps(fb, fa, 8), &[0, 2, 3, 4, 5, 6, 7, 1])
    } else {
        out[0] = ca;
        out[1] = cb;
        (alpha_steps(fa, fb, 6), &[0, 2, 3, 4, 5, 1])
    };
    let f_steps = (steps - 1) as f32;
    let scale = if s[1] == s[0] {
        0.0
    } else {
        f_steps / (s[1] - s[0])
    };
    let words = alpha_indices_dxt5(c, &s, scale, steps, p_steps, dither);
    for (set, dw) in words.iter().enumerate() {
        out[2 + set * 3..5 + set * 3].copy_from_slice(&dw.to_le_bytes()[..3]);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode565_and_decode565_round_trip() {
        for w in [0u16, 0xffff, 0x1234, 0xf81f, 0x07e0] {
            let c = decode565(w);
            assert_eq!(encode565(&[c[0], c[1], c[2]]), w);
        }
    }

    #[test]
    fn a_transparent_dxt1_block_is_the_dlls_constant() {
        let px = [[0.5, 0.5, 0.5, 0.0]; 16];
        assert_eq!(
            encode_dxt1(&px, false),
            [0, 0, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff]
        );
    }

    /// The lookup tables are the DLL's floats (0x4080d4.., 0x408060..).
    #[test]
    fn tables_are_the_dlls() {
        let bits = |v: &[f32]| v.iter().map(|x| x.to_bits()).collect::<Vec<_>>();
        assert_eq!(bits(&PD4), [0, 0x3eaa_aaab, 0x3f2a_aaab, 0x3f80_0000]);
        assert_eq!(bits(&PC4), [0x3f80_0000, 0x3f2a_aaab, 0x3eaa_aaab, 0]);
        let (pc8, pd8) = alpha_tables(8);
        assert_eq!(
            bits(&pd8),
            [
                0,
                0x3e12_4925,
                0x3e92_4925,
                0x3edb_6db7,
                0x3f12_4925,
                0x3f36_db6e,
                0x3f5b_6db7,
                0x3f92_4925
            ]
        );
        assert_eq!(
            bits(&pc8),
            [
                0x3f80_0000,
                0x3f5b_6db7,
                0x3f36_db6e,
                0x3f12_4925,
                0x3edb_6db7,
                0x3e92_4925,
                0x3e12_4925,
                0
            ]
        );
        let (pc6, pd6) = alpha_tables(6);
        assert_eq!(
            bits(&pc6[..6]),
            [
                0x3f80_0000,
                0x3f4c_cccd,
                0x3f19_999a,
                0x3ecc_cccd,
                0x3e4c_cccd,
                0
            ]
        );
        assert_eq!(
            bits(&pd6[..6]),
            [
                0,
                0x3e4c_cccd,
                0x3ecc_cccd,
                0x3f19_999a,
                0x3f4c_cccd,
                0x3f80_0000
            ]
        );
    }
}
