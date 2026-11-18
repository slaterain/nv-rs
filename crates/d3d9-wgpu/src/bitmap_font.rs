//! The port's own small bitmap font: the overlay text (the F11 frame-rate counter, the start
//! screen's banner, status line and log lines) on systems without Windows' GDI. Windows keeps GDI
//! (`loading::text_image`).
//!
//! Glyphs drawn for this file: 5 by 7 pixels in a 6 by 8 cell (one column and one row of space),
//! every printable ASCII character; any other character is drawn as a box.

/// A glyph's width and height in pixels, and its cell's.
const GLYPH: (u32, u32) = (5, 7);
pub(crate) const CELL: (u32, u32) = (6, 8);

/// Characters 0x20 (space) to 0x7e ('~'), each 7 rows top first, bit 4 the leftmost column.
#[rustfmt::skip]
static ASCII: [[u8; 7]; 95] = [
    [0b00000, 0b00000, 0b00000, 0b00000, 0b00000, 0b00000, 0b00000], // ' '
    [0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b00000, 0b00100], // '!'
    [0b01010, 0b01010, 0b01010, 0b00000, 0b00000, 0b00000, 0b00000], // '"'
    [0b01010, 0b01010, 0b11111, 0b01010, 0b11111, 0b01010, 0b01010], // '#'
    [0b00100, 0b01111, 0b10100, 0b01110, 0b00101, 0b11110, 0b00100], // '$'
    [0b11000, 0b11001, 0b00010, 0b00100, 0b01000, 0b10011, 0b00011], // '%'
    [0b01100, 0b10010, 0b10100, 0b01000, 0b10101, 0b10010, 0b01101], // '&'
    [0b00100, 0b00100, 0b01000, 0b00000, 0b00000, 0b00000, 0b00000], // '\''
    [0b00010, 0b00100, 0b01000, 0b01000, 0b01000, 0b00100, 0b00010], // '('
    [0b01000, 0b00100, 0b00010, 0b00010, 0b00010, 0b00100, 0b01000], // ')'
    [0b00000, 0b00100, 0b10101, 0b01110, 0b10101, 0b00100, 0b00000], // '*'
    [0b00000, 0b00100, 0b00100, 0b11111, 0b00100, 0b00100, 0b00000], // '+'
    [0b00000, 0b00000, 0b00000, 0b00000, 0b01100, 0b00100, 0b01000], // ','
    [0b00000, 0b00000, 0b00000, 0b11111, 0b00000, 0b00000, 0b00000], // '-'
    [0b00000, 0b00000, 0b00000, 0b00000, 0b00000, 0b01100, 0b01100], // '.'
    [0b00000, 0b00001, 0b00010, 0b00100, 0b01000, 0b10000, 0b00000], // '/'
    [0b01110, 0b10001, 0b10011, 0b10101, 0b11001, 0b10001, 0b01110], // '0'
    [0b00100, 0b01100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110], // '1'
    [0b01110, 0b10001, 0b00001, 0b00010, 0b00100, 0b01000, 0b11111], // '2'
    [0b11111, 0b00010, 0b00100, 0b00010, 0b00001, 0b10001, 0b01110], // '3'
    [0b00010, 0b00110, 0b01010, 0b10010, 0b11111, 0b00010, 0b00010], // '4'
    [0b11111, 0b10000, 0b11110, 0b00001, 0b00001, 0b10001, 0b01110], // '5'
    [0b00110, 0b01000, 0b10000, 0b11110, 0b10001, 0b10001, 0b01110], // '6'
    [0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b01000, 0b01000], // '7'
    [0b01110, 0b10001, 0b10001, 0b01110, 0b10001, 0b10001, 0b01110], // '8'
    [0b01110, 0b10001, 0b10001, 0b01111, 0b00001, 0b00010, 0b01100], // '9'
    [0b00000, 0b01100, 0b01100, 0b00000, 0b01100, 0b01100, 0b00000], // ':'
    [0b00000, 0b01100, 0b01100, 0b00000, 0b01100, 0b00100, 0b01000], // ';'
    [0b00010, 0b00100, 0b01000, 0b10000, 0b01000, 0b00100, 0b00010], // '<'
    [0b00000, 0b00000, 0b11111, 0b00000, 0b11111, 0b00000, 0b00000], // '='
    [0b01000, 0b00100, 0b00010, 0b00001, 0b00010, 0b00100, 0b01000], // '>'
    [0b01110, 0b10001, 0b00001, 0b00010, 0b00100, 0b00000, 0b00100], // '?'
    [0b01110, 0b10001, 0b00001, 0b01101, 0b10101, 0b10101, 0b01110], // '@'
    [0b01110, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001], // 'A'
    [0b11110, 0b10001, 0b10001, 0b11110, 0b10001, 0b10001, 0b11110], // 'B'
    [0b01110, 0b10001, 0b10000, 0b10000, 0b10000, 0b10001, 0b01110], // 'C'
    [0b11100, 0b10010, 0b10001, 0b10001, 0b10001, 0b10010, 0b11100], // 'D'
    [0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b11111], // 'E'
    [0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b10000], // 'F'
    [0b01110, 0b10001, 0b10000, 0b10111, 0b10001, 0b10001, 0b01111], // 'G'
    [0b10001, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001], // 'H'
    [0b01110, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110], // 'I'
    [0b00111, 0b00010, 0b00010, 0b00010, 0b00010, 0b10010, 0b01100], // 'J'
    [0b10001, 0b10010, 0b10100, 0b11000, 0b10100, 0b10010, 0b10001], // 'K'
    [0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b11111], // 'L'
    [0b10001, 0b11011, 0b10101, 0b10101, 0b10001, 0b10001, 0b10001], // 'M'
    [0b10001, 0b10001, 0b11001, 0b10101, 0b10011, 0b10001, 0b10001], // 'N'
    [0b01110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110], // 'O'
    [0b11110, 0b10001, 0b10001, 0b11110, 0b10000, 0b10000, 0b10000], // 'P'
    [0b01110, 0b10001, 0b10001, 0b10001, 0b10101, 0b10010, 0b01101], // 'Q'
    [0b11110, 0b10001, 0b10001, 0b11110, 0b10100, 0b10010, 0b10001], // 'R'
    [0b01111, 0b10000, 0b10000, 0b01110, 0b00001, 0b00001, 0b11110], // 'S'
    [0b11111, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100], // 'T'
    [0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110], // 'U'
    [0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01010, 0b00100], // 'V'
    [0b10001, 0b10001, 0b10001, 0b10101, 0b10101, 0b10101, 0b01010], // 'W'
    [0b10001, 0b10001, 0b01010, 0b00100, 0b01010, 0b10001, 0b10001], // 'X'
    [0b10001, 0b10001, 0b10001, 0b01010, 0b00100, 0b00100, 0b00100], // 'Y'
    [0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b10000, 0b11111], // 'Z'
    [0b01110, 0b01000, 0b01000, 0b01000, 0b01000, 0b01000, 0b01110], // '['
    [0b00000, 0b10000, 0b01000, 0b00100, 0b00010, 0b00001, 0b00000], // '\\'
    [0b01110, 0b00010, 0b00010, 0b00010, 0b00010, 0b00010, 0b01110], // ']'
    [0b00100, 0b01010, 0b10001, 0b00000, 0b00000, 0b00000, 0b00000], // '^'
    [0b00000, 0b00000, 0b00000, 0b00000, 0b00000, 0b00000, 0b11111], // '_'
    [0b01000, 0b00100, 0b00010, 0b00000, 0b00000, 0b00000, 0b00000], // '`'
    [0b00000, 0b00000, 0b01110, 0b00001, 0b01111, 0b10001, 0b01111], // 'a'
    [0b10000, 0b10000, 0b10110, 0b11001, 0b10001, 0b10001, 0b11110], // 'b'
    [0b00000, 0b00000, 0b01110, 0b10000, 0b10000, 0b10001, 0b01110], // 'c'
    [0b00001, 0b00001, 0b01101, 0b10011, 0b10001, 0b10001, 0b01111], // 'd'
    [0b00000, 0b00000, 0b01110, 0b10001, 0b11111, 0b10000, 0b01110], // 'e'
    [0b00110, 0b01001, 0b01000, 0b11100, 0b01000, 0b01000, 0b01000], // 'f'
    [0b00000, 0b01111, 0b10001, 0b10001, 0b01111, 0b00001, 0b01110], // 'g'
    [0b10000, 0b10000, 0b10110, 0b11001, 0b10001, 0b10001, 0b10001], // 'h'
    [0b00100, 0b00000, 0b01100, 0b00100, 0b00100, 0b00100, 0b01110], // 'i'
    [0b00010, 0b00000, 0b00110, 0b00010, 0b00010, 0b10010, 0b01100], // 'j'
    [0b10000, 0b10000, 0b10010, 0b10100, 0b11000, 0b10100, 0b10010], // 'k'
    [0b01100, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110], // 'l'
    [0b00000, 0b00000, 0b11010, 0b10101, 0b10101, 0b10001, 0b10001], // 'm'
    [0b00000, 0b00000, 0b10110, 0b11001, 0b10001, 0b10001, 0b10001], // 'n'
    [0b00000, 0b00000, 0b01110, 0b10001, 0b10001, 0b10001, 0b01110], // 'o'
    [0b00000, 0b00000, 0b11110, 0b10001, 0b11110, 0b10000, 0b10000], // 'p'
    [0b00000, 0b00000, 0b01101, 0b10011, 0b01111, 0b00001, 0b00001], // 'q'
    [0b00000, 0b00000, 0b10110, 0b11001, 0b10000, 0b10000, 0b10000], // 'r'
    [0b00000, 0b00000, 0b01110, 0b10000, 0b01110, 0b00001, 0b11110], // 's'
    [0b01000, 0b01000, 0b11100, 0b01000, 0b01000, 0b01001, 0b00110], // 't'
    [0b00000, 0b00000, 0b10001, 0b10001, 0b10001, 0b10011, 0b01101], // 'u'
    [0b00000, 0b00000, 0b10001, 0b10001, 0b10001, 0b01010, 0b00100], // 'v'
    [0b00000, 0b00000, 0b10001, 0b10001, 0b10101, 0b10101, 0b01010], // 'w'
    [0b00000, 0b00000, 0b10001, 0b01010, 0b00100, 0b01010, 0b10001], // 'x'
    [0b00000, 0b00000, 0b10001, 0b10001, 0b01111, 0b00001, 0b01110], // 'y'
    [0b00000, 0b00000, 0b11111, 0b00010, 0b00100, 0b01000, 0b11111], // 'z'
    [0b00010, 0b00100, 0b00100, 0b01000, 0b00100, 0b00100, 0b00010], // '{'
    [0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100], // '|'
    [0b01000, 0b00100, 0b00100, 0b00010, 0b00100, 0b00100, 0b01000], // '}'
    [0b00000, 0b00000, 0b01000, 0b10101, 0b00010, 0b00000, 0b00000], // '~'
];

/// Any character without a glyph of its own.
static BOX: [u8; 7] = [
    0b11111, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b11111,
];

/// `c`'s glyph (a tab as a space; the box for anything outside printable ASCII).
fn glyph(c: char) -> &'static [u8; 7] {
    let c = if c == '\t' { ' ' } else { c };
    match u32::from(c) {
        n @ 0x20..=0x7e => &ASCII[(n - 0x20) as usize],
        _ => &BOX,
    }
}

/// How many times larger than the 8-pixel cell the text is drawn for lines `height` pixels high:
/// the nearest whole number, at least 1 (whole pixels: the glyphs stay sharp).
pub(crate) fn scale(height: u32) -> u32 {
    ((height + CELL.1 / 2) / CELL.1).max(1)
}

/// `text`'s lines (split at '\n') drawn white on transparent, `scale` (from [`scale`]) times the
/// cell's size, B, G, R, A rows top first: the same form as Windows' GDI text in
/// `loading::text_image`. Every character takes one cell, so a line of n characters is n cells
/// wide (the frame-rate counter cuts its glyph strip into equal cells).
pub(crate) fn raster(text: &str, scale: u32) -> (u32, u32, Vec<u8>) {
    let s = scale.max(1);
    let lines: Vec<&str> = text.split('\n').collect();
    let widest = lines.iter().map(|l| l.chars().count()).max().unwrap_or(0) as u32;
    let (cw, ch) = (CELL.0 * s, CELL.1 * s);
    let (w, h) = ((widest * cw).max(1), ch * lines.len() as u32);
    let mut px = vec![0u8; (w * h * 4) as usize];
    for p in px.chunks_exact_mut(4) {
        p[..3].copy_from_slice(&[255, 255, 255]);
    }
    for (row, line) in lines.iter().enumerate() {
        for (col, c) in line.chars().enumerate() {
            let g = glyph(c);
            for (gy, bits) in g.iter().enumerate() {
                for gx in 0..GLYPH.0 {
                    if bits & (1 << (GLYPH.0 - 1 - gx)) == 0 {
                        continue;
                    }
                    let x0 = col as u32 * cw + gx * s;
                    let y0 = row as u32 * ch + gy as u32 * s;
                    for y in y0..y0 + s {
                        for x in x0..x0 + s {
                            px[((y * w + x) * 4 + 3) as usize] = 255;
                        }
                    }
                }
            }
        }
    }
    debug_assert!(GLYPH.1 < CELL.1 && GLYPH.0 < CELL.0);
    (w, h, px)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The alpha of `raster`'s picture as rows of '#' and '.'.
    fn picture(text: &str, scale: u32) -> Vec<String> {
        let (w, h, px) = raster(text, scale);
        (0..h)
            .map(|y| {
                (0..w)
                    .map(|x| match px[((y * w + x) * 4 + 3) as usize] {
                        255 => '#',
                        0 => '.',
                        _ => '?',
                    })
                    .collect()
            })
            .collect()
    }

    #[test]
    fn a_known_string_is_drawn_pixel_for_pixel() {
        assert_eq!(
            picture("1.x", 1),
            [
                "..#...............",
                ".##...............",
                "..#.........#...#.",
                "..#..........#.#..",
                "..#...........#...",
                "..#....##....#.#..",
                ".###...##...#...#.",
                "..................",
            ]
        );
    }

    /// The F11 counter's whole glyph strip, cell by cell, against each glyph's picture written
    /// out here: a wrong, mirrored or shifted counter glyph fails.
    #[test]
    fn the_frame_rate_strip_is_drawn_pixel_for_pixel() {
        #[rustfmt::skip]
        let want: [(char, [&str; 7]); 17] = [
            ('0', [".###.", "#...#", "#..##", "#.#.#", "##..#", "#...#", ".###."]),
            ('1', ["..#..", ".##..", "..#..", "..#..", "..#..", "..#..", ".###."]),
            ('2', [".###.", "#...#", "....#", "...#.", "..#..", ".#...", "#####"]),
            ('3', ["#####", "...#.", "..#..", "...#.", "....#", "#...#", ".###."]),
            ('4', ["...#.", "..##.", ".#.#.", "#..#.", "#####", "...#.", "...#."]),
            ('5', ["#####", "#....", "####.", "....#", "....#", "#...#", ".###."]),
            ('6', ["..##.", ".#...", "#....", "####.", "#...#", "#...#", ".###."]),
            ('7', ["#####", "....#", "...#.", "..#..", ".#...", ".#...", ".#..."]),
            ('8', [".###.", "#...#", "#...#", ".###.", "#...#", "#...#", ".###."]),
            ('9', [".###.", "#...#", "#...#", ".####", "....#", "...#.", ".##.."]),
            ('.', [".....", ".....", ".....", ".....", ".....", ".##..", ".##.."]),
            ('f', ["..##.", ".#..#", ".#...", "###..", ".#...", ".#...", ".#..."]),
            ('p', [".....", ".....", "####.", "#...#", "####.", "#....", "#...."]),
            ('s', [".....", ".....", ".###.", "#....", ".###.", "....#", "####."]),
            ('m', [".....", ".....", "##.#.", "#.#.#", "#.#.#", "#...#", "#...#"]),
            ('a', [".....", ".....", ".###.", "....#", ".####", "#...#", ".####"]),
            ('x', [".....", ".....", "#...#", ".#.#.", "..#..", ".#.#.", "#...#"]),
        ];
        let strip: String = want.iter().map(|(c, _)| *c).collect();
        // The counter's own strip (`frame_rate::GLYPHS`), in its order.
        assert_eq!(strip, "0123456789.fpsmax");
        let rows = picture(&strip, 1);
        assert_eq!(rows.len(), 8);
        for (i, (c, glyph)) in want.iter().enumerate() {
            let cell = |y: usize| &rows[y][6 * i..6 * i + 6];
            for (y, row) in glyph.iter().enumerate() {
                assert_eq!(cell(y), format!("{row}."), "{c:?} row {y}");
            }
            assert_eq!(cell(7), "......", "{c:?} row 7");
        }
    }

    #[test]
    fn a_larger_scale_is_each_pixel_a_square_and_lines_stack() {
        let one = picture("7", 1);
        let two = picture("7", 2);
        assert_eq!(two.len(), 16);
        for (y, row) in two.iter().enumerate() {
            let want: String = one[y / 2].chars().flat_map(|c| [c, c]).collect();
            assert_eq!(row, &want, "row {y}");
        }
        let (w, h, px) = raster("ab\nc", 3);
        assert_eq!((w, h), (2 * 18, 2 * 24));
        assert_eq!(px.len(), (w * h * 4) as usize);
        // White everywhere: only the alpha carries the glyphs, as GDI's picture does.
        assert!(px.chunks_exact(4).all(|p| p[..3] == [255, 255, 255]));
    }

    #[test]
    fn the_overlay_texts_have_glyphs_of_their_own_and_others_a_box() {
        // The F11 counter's strip and the start screen's banner and a status line.
        for c in "0123456789.fpsmax PLEASE STAND BY Preparing shaders 212 of 1,480".chars() {
            assert!(!std::ptr::eq(glyph(c), &BOX), "{c:?}");
        }
        // Every printable character is told apart from every other one.
        for a in 0x21u8..=0x7e {
            for b in a + 1..=0x7e {
                assert_ne!(
                    glyph(a as char),
                    glyph(b as char),
                    "{} {}",
                    a as char,
                    b as char
                );
            }
            assert_ne!(glyph(a as char), &BOX);
        }
        assert!(std::ptr::eq(glyph('é'), &BOX));
        assert_eq!(glyph('\t'), glyph(' '));
    }

    #[test]
    fn the_scale_follows_the_line_height() {
        assert_eq!(scale(0), 1);
        assert_eq!(scale(9), 1);
        assert_eq!(scale(14), 2);
        assert_eq!(scale(24), 3);
        assert_eq!(scale(48), 6);
    }
}
