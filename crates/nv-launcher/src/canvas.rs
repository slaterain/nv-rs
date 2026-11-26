//! The terminal's picture, drawn on the processor: 8-bit RGBA, blended as Direct3D 9 blends the
//! game's menus (straight alpha, in the stored values, no sRGB step). Small enough to draw whole
//! (960 by 720) whenever something on it changes; the graphics card only scales it to the window.

pub struct Canvas {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
}

impl Canvas {
    pub fn new(width: u32, height: u32) -> Canvas {
        let mut pixels = vec![0u8; (width * height * 4) as usize];
        for p in pixels.as_chunks_mut::<4>().0 {
            p[3] = 255;
        }
        Canvas {
            width,
            height,
            pixels,
        }
    }

    /// This picture on a black one of `size` with its top-left corner at `at` (the terminal's
    /// 4:3 screen in a window of another shape: `crate::gpu::Framing`), the part outside cut off;
    /// itself when it is that size already.
    pub fn placed(self, size: (u32, u32), at: (i32, i32)) -> Canvas {
        if (self.width, self.height) == size && at == (0, 0) {
            return self;
        }
        let mut out = Canvas::new(size.0, size.1);
        let x0 = at.0.max(0);
        let x1 = (at.0 + self.width as i32).min(size.0 as i32);
        if x1 <= x0 {
            return out;
        }
        for y in 0..self.height as i32 {
            let ty = y + at.1;
            if ty < 0 || ty >= size.1 as i32 {
                continue;
            }
            let from = ((y as u32 * self.width + (x0 - at.0) as u32) * 4) as usize;
            let to = ((ty as u32 * size.0 + x0 as u32) * 4) as usize;
            let n = ((x1 - x0) * 4) as usize;
            out.pixels[to..to + n].copy_from_slice(&self.pixels[from..from + n]);
        }
        out
    }

    /// `colour` over the pixel at (`x`, `y`) with `coverage` (0 to 1) times its alpha.
    pub fn blend(&mut self, x: i32, y: i32, colour: [f32; 4], coverage: f32) {
        if x < 0 || y < 0 || x >= self.width as i32 || y >= self.height as i32 {
            return;
        }
        let a = (colour[3] * coverage).clamp(0.0, 1.0);
        let at = ((y as u32 * self.width + x as u32) * 4) as usize;
        for (px, over) in self.pixels[at..at + 3].iter_mut().zip(colour) {
            let under = *px as f32 / 255.0;
            let over = over.clamp(0.0, 1.0);
            *px = ((over * a + under * (1.0 - a)) * 255.0).round() as u8;
        }
    }

    /// A filled rectangle.
    pub fn fill(&mut self, x: i32, y: i32, w: u32, h: u32, colour: [f32; 4]) {
        for yy in y..y + h as i32 {
            for xx in x..x + w as i32 {
                self.blend(xx, yy, colour, 1.0);
            }
        }
    }

    /// `image` (RGBA, `iw` by `ih`) stretched over the rectangle, each channel times `tint`.
    pub fn stretch(
        &mut self,
        image: (&[u8], u32, u32),
        rect: (i32, i32, u32, u32),
        tint: [f32; 4],
    ) {
        let (px, iw, ih) = image;
        let (x, y, w, h) = rect;
        for yy in 0..h {
            // The texel under the pixel's centre (the game samples with bilinear filtering; at
            // this size the difference is a fraction of a texel).
            let ty = (((yy as f32 + 0.5) * ih as f32 / h as f32) as u32).min(ih - 1);
            for xx in 0..w {
                let tx = (((xx as f32 + 0.5) * iw as f32 / w as f32) as u32).min(iw - 1);
                let t = &px[((ty * iw + tx) * 4) as usize..][..4];
                let c = [
                    t[0] as f32 / 255.0 * tint[0],
                    t[1] as f32 / 255.0 * tint[1],
                    t[2] as f32 / 255.0 * tint[2],
                    t[3] as f32 / 255.0 * tint[3],
                ];
                self.blend(x + xx as i32, y + yy as i32, c, 1.0);
            }
        }
    }
}

/// A DXT1 (BC1) image's pixels as RGBA, `w` by `h` (the first level of a DDS file's data).
pub fn decode_dxt1(data: &[u8], w: u32, h: u32) -> Option<Vec<u8>> {
    let mut out = vec![0u8; (w * h * 4) as usize];
    let (bw, bh) = (w.div_ceil(4), h.div_ceil(4));
    for by in 0..bh {
        for bx in 0..bw {
            let at = ((by * bw + bx) * 8) as usize;
            let b = data.get(at..at + 8)?;
            let c0 = u16::from_le_bytes([b[0], b[1]]);
            let c1 = u16::from_le_bytes([b[2], b[3]]);
            let rgb = |c: u16| {
                let r = ((c >> 11) & 31) as u32;
                let g = ((c >> 5) & 63) as u32;
                let bl = (c & 31) as u32;
                [
                    (r * 255 + 15) / 31,
                    (g * 255 + 31) / 63,
                    (bl * 255 + 15) / 31,
                ]
            };
            let (p0, p1) = (rgb(c0), rgb(c1));
            let mix = |a: u32, b: u32, n: u32, d: u32| (a * (d - n) + b * n) / d;
            let pal: [[u32; 4]; 4] = if c0 > c1 {
                [
                    [p0[0], p0[1], p0[2], 255],
                    [p1[0], p1[1], p1[2], 255],
                    [
                        mix(p0[0], p1[0], 1, 3),
                        mix(p0[1], p1[1], 1, 3),
                        mix(p0[2], p1[2], 1, 3),
                        255,
                    ],
                    [
                        mix(p0[0], p1[0], 2, 3),
                        mix(p0[1], p1[1], 2, 3),
                        mix(p0[2], p1[2], 2, 3),
                        255,
                    ],
                ]
            } else {
                [
                    [p0[0], p0[1], p0[2], 255],
                    [p1[0], p1[1], p1[2], 255],
                    [
                        mix(p0[0], p1[0], 1, 2),
                        mix(p0[1], p1[1], 1, 2),
                        mix(p0[2], p1[2], 1, 2),
                        255,
                    ],
                    [0, 0, 0, 0],
                ]
            };
            let bits = u32::from_le_bytes([b[4], b[5], b[6], b[7]]);
            for i in 0..16 {
                let (x, y) = (bx * 4 + i % 4, by * 4 + i / 4);
                if x >= w || y >= h {
                    continue;
                }
                let p = pal[((bits >> (2 * i)) & 3) as usize];
                let o = ((y * w + x) * 4) as usize;
                for c in 0..4 {
                    out[o + c] = p[c] as u8;
                }
            }
        }
    }
    Some(out)
}

/// A DDS file's first level: width, height and RGBA pixels, for the DXT1 files the terminal uses.
pub fn dds_rgba(dds: &[u8]) -> Option<(u32, u32, Vec<u8>)> {
    let u32_at = |o: usize| Some(u32::from_le_bytes(dds.get(o..o + 4)?.try_into().ok()?));
    if dds.get(0..4)? != b"DDS " {
        return None;
    }
    let (h, w) = (u32_at(12)?, u32_at(16)?);
    if dds.get(84..88)? != b"DXT1" {
        return None;
    }
    Some((w, h, decode_dxt1(dds.get(128..)?, w, h)?))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A picture placed on a larger one keeps every pixel, moved by `at`, with black (opaque)
    /// around it; one placed on its own size is itself.
    #[test]
    fn a_picture_is_placed_whole_with_black_around_it() {
        let mut small = Canvas::new(3, 2);
        for (i, p) in small.pixels.as_chunks_mut::<4>().0.iter_mut().enumerate() {
            *p = [i as u8 + 1, 7, 9, 255];
        }
        let big = Canvas {
            width: 3,
            height: 2,
            pixels: small.pixels.clone(),
        }
        .placed((6, 4), (2, 1));
        let at = |x: u32, y: u32| &big.pixels[((y * 6 + x) * 4) as usize..][..4];
        for y in 0..4 {
            for x in 0..6 {
                if (2..5).contains(&x) && (1..3).contains(&y) {
                    let i = ((y - 1) * 3 + (x - 2)) as u8;
                    assert_eq!(at(x, y), [i + 1, 7, 9, 255], "{x},{y}");
                } else {
                    assert_eq!(at(x, y), [0, 0, 0, 255], "{x},{y}");
                }
            }
        }
        let same = small.placed((3, 2), (0, 0));
        assert_eq!(same.pixels[0], 1);
        assert_eq!(
            Canvas::new(2, 2).placed((1, 1), (5, 5)).pixels,
            [0, 0, 0, 255]
        );
    }
}
