//! The terminal's text: the game's own terminal font when the game is found (font 5 of the game's
//! `[Fonts]`, `Fixedsys_Comp_uniform_width`, which `menus\computers_menu.xml` gives every text of
//! the terminal), read from the player's archives; before that, the port's own 5 by 7 bitmap font
//! (`crates/d3d9-wgpu/src/bitmap_font.rs`, drawn for the project) at three times its size, in a
//! taller cell with g, j, p, q and y given tails below the line ([`PixelGlyphs`]).
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
#[allow(dead_code, clippy::chunks_exact_to_as_chunks)]
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
    /// The port's own bitmap font ([`PixelGlyphs`]), `scale` times its cell.
    Pixel {
        scale: u32,
        glyphs: Box<PixelGlyphs>,
    },
}

/// The port's own 5 by 7 glyphs (`bitmap_font`) in a taller cell, 6 by 10: two rows under the
/// line, so the letters that reach below it (g, j, p, q, y) are drawn with their tails there
/// instead of squeezed above the line, where they read as 9, r and ?. Each glyph is 9 rows, top
/// first, bit 4 the leftmost column; printable ASCII, anything else the box.
pub struct PixelGlyphs([[u8; 9]; 96]);

/// The cell: one column and one row of space beside the 5 by 9 glyph.
const PIXEL_CELL: (u32, u32) = (6, 10);

/// The letters drawn again with their tails below the line (rows 7 and 8).
const DESCENDERS: [(char, [u8; 9]); 5] = [
    (
        'g',
        [
            0, 0, 0b01111, 0b10001, 0b10001, 0b10001, 0b01111, 0b00001, 0b01110,
        ],
    ),
    (
        'j',
        [
            0b00010, 0, 0b00110, 0b00010, 0b00010, 0b00010, 0b00010, 0b10010, 0b01100,
        ],
    ),
    (
        'p',
        [
            0, 0, 0b11110, 0b10001, 0b10001, 0b10001, 0b11110, 0b10000, 0b10000,
        ],
    ),
    (
        'q',
        [
            0, 0, 0b01111, 0b10001, 0b10001, 0b10001, 0b01111, 0b00001, 0b00001,
        ],
    ),
    (
        'y',
        [
            0, 0, 0b10001, 0b10001, 0b10001, 0b10001, 0b01111, 0b00001, 0b01110,
        ],
    ),
];

impl PixelGlyphs {
    /// The port's glyphs, read back from its own drawing at size 1, with [`DESCENDERS`].
    fn new() -> PixelGlyphs {
        let mut table = [[0u8; 9]; 96];
        for (i, rows) in table.iter_mut().enumerate() {
            // 0x20..=0x7e, then the box (any character outside them) at 95.
            let c = if i < 95 {
                char::from(0x20 + i as u8)
            } else {
                '\u{80}'
            };
            let (w, _, px) = bitmap_font::raster(&c.to_string(), 1);
            for (y, row) in rows.iter_mut().take(7).enumerate() {
                for x in 0..5u32 {
                    if px[((y as u32 * w + x) * 4 + 3) as usize] != 0 {
                        *row |= 1 << (4 - x);
                    }
                }
            }
        }
        for (c, rows) in DESCENDERS {
            table[c as usize - 0x20] = rows;
        }
        PixelGlyphs(table)
    }

    fn glyph(&self, c: char) -> &[u8; 9] {
        let c = if c == '\t' { ' ' } else { c };
        match u32::from(c) {
            n @ 0x20..=0x7e => &self.0[(n - 0x20) as usize],
            _ => &self.0[95],
        }
    }
}

impl Font {
    /// The game's font from its `.fnt` and its first texture's `.tex` bytes.
    pub fn game(fnt: &[u8], tex: &[u8]) -> Option<Font> {
        let f32_at = |o: usize| Some(f32::from_le_bytes(fnt.get(o..o + 4)?.try_into().ok()?));
        let u32_at =
            |b: &[u8], o: usize| Some(u32::from_le_bytes(b.get(o..o + 4)?.try_into().ok()?));
        let base_line = f32_at(0)?;
        let (tw, th) = (u32_at(tex, 0)?, u32_at(tex, 4)?);
        let pixels = tex.get(8..8 + (tw * th * 4) as usize)?;
        // The letters are white; the alpha is the coverage.
        let coverage: Vec<u8> = pixels.as_chunks::<4>().0.iter().map(|p| p[3]).collect();
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

    /// The port's own font, at three times its cell (18 by 30 pixels a character).
    pub fn pixel() -> Font {
        Font::Pixel {
            scale: 3,
            glyphs: Box::new(PixelGlyphs::new()),
        }
    }

    /// A line's height in pixels.
    pub fn line_height(&self) -> u32 {
        match self {
            Font::Game { height, .. } => *height,
            Font::Pixel { scale, .. } => PIXEL_CELL.1 * scale,
        }
    }

    /// How far the pen moves for `c`.
    fn advance(&self, c: char) -> f32 {
        match self {
            Font::Game { letters, .. } => {
                let l = &letters[byte_of(c) as usize];
                l.leading + l.width + if l.width > 0.0 { l.spacing } else { 0.0 }
            }
            Font::Pixel { scale, .. } => (PIXEL_CELL.0 * scale) as f32,
        }
    }

    /// `text`'s width in pixels.
    pub fn width(&self, text: &str) -> u32 {
        text.chars().map(|c| self.advance(c)).sum::<f32>().ceil() as u32
    }

    /// `text` drawn with its line's top at (`x`, `y`) in `colour` (straight RGBA, 0 to 1): how
    /// many of its characters were drawn (a line reaching past the canvas's right edge stops
    /// there).
    pub fn draw(&self, canvas: &mut Canvas, x: i32, y: i32, text: &str, colour: [f32; 4]) -> usize {
        let mut drawn = 0;
        match self {
            Font::Game {
                base_line,
                letters,
                tex_w,
                coverage,
                ..
            } => {
                let mut pen = x as f32;
                // Nothing is drawn past the canvas's right edge: a long line stops there.
                let edge = canvas.width as f32;
                for c in text.chars() {
                    if pen >= edge {
                        break;
                    }
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
                    drawn += 1;
                }
            }
            Font::Pixel { scale, glyphs } => {
                let s = *scale as i32;
                for (col, c) in text.chars().enumerate() {
                    let left = x + col as i32 * PIXEL_CELL.0 as i32 * s;
                    if left >= canvas.width as i32 {
                        break;
                    }
                    for (gy, bits) in glyphs.glyph(c).iter().enumerate() {
                        for gx in 0..5 {
                            if bits & (1 << (4 - gx)) == 0 {
                                continue;
                            }
                            canvas.fill(
                                left + gx * s,
                                y + gy as i32 * s,
                                s as u32,
                                s as u32,
                                colour,
                            );
                        }
                    }
                    drawn += 1;
                }
            }
        }
        drawn
    }
}

/// The game's text is one byte a character (Windows-1252): anything outside it as '?'.
fn byte_of(c: char) -> u8 {
    u8::try_from(u32::from(c)).unwrap_or(b'?')
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The letters with tails draw them below the line (rows 7 and 8); the others keep the
    /// port's glyph as it is, with those two rows empty.
    #[test]
    fn descenders_go_below_the_line() {
        let g = PixelGlyphs::new();
        for c in ['g', 'j', 'p', 'q', 'y'] {
            let rows = g.glyph(c);
            assert!(rows[7] != 0 && rows[8] != 0, "{c}: {rows:?}");
            assert_eq!(rows[0] | rows[1], if c == 'j' { 0b00010 } else { 0 }, "{c}");
        }
        assert_eq!(
            *g.glyph('1'),
            [0b00100, 0b01100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110, 0, 0]
        );
        for c in ['a', 'x', 'A', ',', '?'] {
            assert_eq!(g.glyph(c)[7] | g.glyph(c)[8], 0, "{c}");
        }
        // Outside printable ASCII: the box.
        assert_eq!(g.glyph('\u{e9}')[0], 0b11111);
    }

    /// A made-up game font: every letter a solid 4 by 4 square of an 8 by 8 texture, 4 wide with
    /// 1 of spacing (5 a character), its top on the line's top.
    fn square_font() -> Font {
        let mut fnt = vec![0u8; 0x128 + 256 * 0x38];
        fnt[0..4].copy_from_slice(&16f32.to_le_bytes());
        for i in 0..256 {
            let at = 0x128 + i * 0x38;
            let mut put =
                |o: usize, v: f32| fnt[at + o..at + o + 4].copy_from_slice(&v.to_le_bytes());
            // Corners: top left (0, 0), top right, bottom left, bottom right (0.5, 0.5).
            for (k, v) in [0.0, 0.0, 0.5, 0.0, 0.0, 0.5, 0.5, 0.5]
                .into_iter()
                .enumerate()
            {
                put(4 + 4 * k, v);
            }
            put(0x24, 4.0);
            put(0x28, 4.0);
            put(0x2c, 0.0);
            put(0x30, 1.0);
            put(0x34, 16.0);
        }
        let mut tex = Vec::new();
        tex.extend(8u32.to_le_bytes());
        tex.extend(8u32.to_le_bytes());
        tex.extend([255u8; 8 * 8 * 4]);
        Font::game(&fnt, &tex).expect("the made-up font reads")
    }

    /// Whether the pixel at (`x`, `y`) was drawn on (the canvas starts black; text is white).
    fn lit(c: &Canvas, x: u32, y: u32) -> bool {
        c.pixels[((y * c.width + x) * 4) as usize] != 0
    }

    /// Both fonts stop a line at the canvas's right edge: the characters starting before it are
    /// drawn (the last one up to the edge), the rest are not, nothing is written past the canvas
    /// and nothing panics.
    #[test]
    fn a_line_wider_than_the_canvas_stops_at_its_right_edge() {
        let white = [1.0; 4];
        let text = "MMMMMMMMMM";

        // The game's font, 5 a character: on 23 pixels the letters at 0, 5, 10, 15 and 20.
        let font = square_font();
        let mut c = Canvas::new(23, 8);
        assert_eq!(font.draw(&mut c, 0, 0, text, white), 5);
        assert_eq!(c.pixels.len(), 23 * 8 * 4);
        for x in [0, 3, 20, 22] {
            assert!(lit(&c, x, 0), "x {x}");
        }
        assert!(!lit(&c, 4, 0), "the spacing");
        // Started further in: fewer letters reach the canvas.
        let mut c = Canvas::new(23, 8);
        assert_eq!(font.draw(&mut c, 12, 0, text, white), 3);
        assert!(lit(&c, 22, 0) && !lit(&c, 11, 0));

        // The port's font, 18 a character: on 40 pixels the letters at 0, 18 and 36.
        let font = Font::pixel();
        let mut c = Canvas::new(40, font.line_height());
        assert_eq!(font.draw(&mut c, 0, 0, text, white), 3);
        assert_eq!(c.pixels.len(), (40 * font.line_height() * 4) as usize);
        // M's left column is lit from its top row: at 0, 18 and 36 (three pixels wide).
        for x in [0, 18, 36, 38] {
            assert!(lit(&c, x, 0), "x {x}");
        }
        // M's right column, 12 into its cell: the second letter's is drawn (the last one's, at
        // 48, is past the edge, where the canvas keeps nothing).
        assert!(lit(&c, 18 + 12, 0));

        // A line wholly past the edge draws nothing.
        let mut c = Canvas::new(23, 8);
        assert_eq!(square_font().draw(&mut c, 23, 0, text, white), 0);
        assert!(c.pixels.as_chunks::<4>().0.iter().all(|p| p[0] == 0));
    }
}
