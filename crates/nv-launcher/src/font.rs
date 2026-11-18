//! The terminal's text: the game's own terminal font when the game is found (font 5 of the game's
//! `[Fonts]`, `Fixedsys_Comp_uniform_width`, which `menus\computers_menu.xml` gives every text of
//! the terminal), read from the player's archives; before that, the port's own 5 by 7 bitmap font
//! (`crates/d3d9-wgpu/src/bitmap_font.rs`, drawn for the project) at three times its size.
//!
//! The game's font file (`textures\fonts\<name>.fnt`, `Font::Load` 00a12020 and `Font::AddChar`
//! 00a142d0 in `crates/engine-bs/src/units/bsmenu/font.rs`): the base line (float), the texture
//! count, eight texture names of 0x24 bytes, then 256 letters of 0x38 bytes from +0x128: texture
//! index, the four corners' texture coordinates (top left, top right, bottom left, bottom right),
//! width, height, leading edge, spacing and top edge. A letter is drawn from the pen moved by its
//! leading edge, `width` wide, its top `top edge` above the pen's line and `height` tall; the pen
//! then moves by the width and the spacing (no spacing for a letter of no width). The texture is
//! `textures\fonts\<texture name>.tex`: width, height, then 32-bit pixels.

#[path = "../../d3d9-wgpu/src/bitmap_font.rs"]
#[allow(dead_code)]
mod bitmap_font;

use crate::canvas::Canvas;

/// One letter of the game's font.
#[derive(Clone, Copy, Default, Debug)]
pub struct Letter {
    /// Texture pixels: left, top, width, height.
    cell: (u32, u32, u32, u32),
    width: f32,
    height: f32,
    leading: f32,
    spacing: f32,
    top: f32,
}

pub enum Font {
    /// The game's font: its letters and the texture's coverage (alpha), `tex_w` wide.
    Game {
        base_line: f32,
        height: u32,
        letters: Box<[Letter; 256]>,
        tex_w: u32,
        coverage: Vec<u8>,
    },
    /// The port's own bitmap font, `scale` times its 6 by 8 cell.
    Pixel { scale: u32 },
}

impl Font {
    /// The game's font from its `.fnt` and its first texture's `.tex` bytes.
    pub fn game(fnt: &[u8], tex: &[u8]) -> Option<Font> {
        let f32_at = |o: usize| Some(f32::from_le_bytes(fnt.get(o..o + 4)?.try_into().ok()?));
        let u32_at = |b: &[u8], o: usize| Some(u32::from_le_bytes(b.get(o..o + 4)?.try_into().ok()?));
        let base_line = f32_at(0)?;
        let (tw, th) = (u32_at(tex, 0)?, u32_at(tex, 4)?);
        let pixels = tex.get(8..8 + (tw * th * 4) as usize)?;
        // The letters are white; the alpha is the coverage.
        let coverage: Vec<u8> = pixels.chunks_exact(4).map(|p| p[3]).collect();
        let mut letters = Box::new([Letter::default(); 256]);
        let mut height = 0f32;
        for (i, letter) in letters.iter_mut().enumerate() {
            let at = 0x128 + i * 0x38;
            let uv = |k: usize| f32_at(at + 4 + 4 * k);
            let (u0, v0, u1, v1) = (uv(0)?, uv(1)?, uv(6)?, uv(7)?);
            let px = |u: f32, size: u32| (u * size as f32).round().max(0.0) as u32;
            let (x0, y0) = (px(u0, tw), px(v0, th));
            let (x1, y1) = (px(u1, tw), px(v1, th));
            *letter = Letter {
                cell: (x0, y0, x1.saturating_sub(x0), y1.saturating_sub(y0)),
                width: f32_at(at + 0x24)?,
                height: f32_at(at + 0x28)?,
                leading: f32_at(at + 0x2c)?,
                spacing: f32_at(at + 0x30)?,
                top: f32_at(at + 0x34)?,
            };
            // `Font::Load`: the font's height is the most any letter reaches below the line's
            // top, (base line - top edge) + height.
            height = height.max(base_line - letter.top + letter.height);
        }
        Some(Font::Game {
            base_line,
            height: height.ceil() as u32,
            letters,
            tex_w: tw,
            coverage,
        })
    }

    /// The port's own font, at three times its cell (18 by 24 pixels a character).
    pub fn pixel() -> Font {
        Font::Pixel { scale: 3 }
    }

    /// A line's height in pixels.
    pub fn line_height(&self) -> u32 {
        match self {
            Font::Game { height, .. } => *height,
            Font::Pixel { scale } => bitmap_font::CELL.1 * scale,
        }
    }

    /// How far the pen moves for `c`.
    fn advance(&self, c: char) -> f32 {
        match self {
            Font::Game { letters, .. } => {
                let l = &letters[byte_of(c) as usize];
                l.leading + l.width + if l.width > 0.0 { l.spacing } else { 0.0 }
            }
            Font::Pixel { scale } => (bitmap_font::CELL.0 * scale) as f32,
        }
    }

    /// `text`'s width in pixels.
    pub fn width(&self, text: &str) -> u32 {
        text.chars().map(|c| self.advance(c)).sum::<f32>().ceil() as u32
    }

    /// `text` drawn with its line's top at (`x`, `y`) in `colour` (straight RGBA, 0 to 1).
    pub fn draw(&self, canvas: &mut Canvas, x: i32, y: i32, text: &str, colour: [f32; 4]) {
        match self {
            Font::Game {
                base_line,
                letters,
                tex_w,
                coverage,
                ..
            } => {
                let mut pen = x as f32;
                for c in text.chars() {
                    let l = &letters[byte_of(c) as usize];
                    pen += l.leading;
                    let (cx, cy, cw, ch) = l.cell;
                    let top = y as f32 + (base_line - l.top);
                    for gy in 0..ch {
                        for gx in 0..cw {
                            let a = coverage[((cy + gy) * tex_w + cx + gx) as usize];
                            if a != 0 {
                                canvas.blend(
                                    pen as i32 + gx as i32,
                                    top as i32 + gy as i32,
                                    colour,
                                    a as f32 / 255.0,
                                );
                            }
                        }
                    }
                    pen += l.width + if l.width > 0.0 { l.spacing } else { 0.0 };
                }
            }
            Font::Pixel { scale } => {
                let (w, h, px) = bitmap_font::raster(text, *scale);
                for gy in 0..h {
                    for gx in 0..w {
                        let a = px[((gy * w + gx) * 4 + 3) as usize];
                        if a != 0 {
                            canvas.blend(x + gx as i32, y + gy as i32, colour, a as f32 / 255.0);
                        }
                    }
                }
            }
        }
    }
}

/// The game's text is one byte a character (Windows-1252): anything outside it as '?'.
fn byte_of(c: char) -> u8 {
    u8::try_from(u32::from(c)).unwrap_or(b'?')
}
