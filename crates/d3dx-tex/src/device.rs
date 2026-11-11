//! The texture format D3DX makes for a file's format on a device (`d3dx9_38.dll` 0x60d069,
//! called by `D3DXCheckTextureRequirements` 0x60d6b8 for every texture the create calls make):
//! the file's format when the device takes it, else the nearest format of D3DX's table the device
//! takes, by D3DX's own measure (0x60cf77). What the device takes is its
//! `IDirect3D9::CheckDeviceFormat` answer for each format (usage 0, the display mode's format),
//! [`crate::Caps`]'s format lists.

/// One row of D3DX's format table (0x406b10, in its order): the format, its kind (0 colour, 1
/// palette, 2 luminance, 3 signed, 4 depth), its bits per pixel, the bits of its five channels as
/// the table keeps them (a fifth channel, then alpha, red or luminance, green, blue; the signed
/// and depth formats keep theirs in the same places), and whether D3DX picks it in place of
/// another format.
struct Row {
    format: u32,
    kind: usize,
    bits: u32,
    channels: [u32; 5],
    fallback: bool,
}

const fn row(format: u32, kind: usize, bits: u32, channels: [u32; 5], fallback: bool) -> Row {
    Row {
        format,
        kind,
        bits,
        channels,
        fallback,
    }
}

const fn cc(s: &[u8; 4]) -> u32 {
    u32::from_le_bytes(*s)
}

/// D3DX's format table (0x406b10).
const TABLE: &[Row] = &[
    row(20, 0, 24, [0, 0, 8, 8, 8], false),
    row(21, 0, 32, [0, 8, 8, 8, 8], true),
    row(22, 0, 32, [0, 0, 8, 8, 8], true),
    row(23, 0, 16, [0, 0, 5, 6, 5], true),
    row(24, 0, 16, [0, 0, 5, 5, 5], true),
    row(40, 1, 16, [0, 8, 8, 8, 8], true),
    row(41, 1, 8, [0, 8, 8, 8, 8], true),
    row(25, 0, 16, [0, 1, 5, 5, 5], true),
    row(26, 0, 16, [0, 4, 4, 4, 4], true),
    row(27, 0, 8, [0, 0, 3, 3, 2], true),
    row(28, 0, 8, [0, 8, 0, 0, 0], true),
    row(29, 0, 16, [0, 8, 3, 3, 2], true),
    row(30, 0, 16, [0, 0, 4, 4, 4], true),
    row(31, 0, 32, [0, 2, 10, 10, 10], true),
    row(32, 0, 32, [0, 8, 8, 8, 8], true),
    row(33, 0, 32, [0, 0, 8, 8, 8], true),
    row(34, 0, 32, [0, 0, 16, 16, 0], true),
    row(35, 0, 32, [0, 2, 10, 10, 10], true),
    row(36, 0, 64, [0, 16, 16, 16, 16], true),
    row(50, 2, 8, [0, 0, 8, 8, 8], true),
    row(51, 2, 16, [0, 8, 8, 8, 8], true),
    row(52, 2, 8, [0, 4, 4, 4, 4], true),
    row(81, 2, 16, [0, 0, 16, 16, 16], true),
    row(60, 3, 16, [0, 0, 0, 8, 8], true),
    row(61, 3, 16, [6, 0, 0, 5, 5], true),
    row(62, 3, 32, [8, 0, 0, 8, 8], true),
    row(63, 3, 32, [0, 8, 8, 8, 8], true),
    row(64, 3, 32, [0, 0, 0, 16, 16], true),
    row(67, 3, 32, [2, 0, 10, 10, 10], true),
    row(110, 3, 64, [0, 16, 16, 16, 16], true),
    row(117, 3, 16, [8, 0, 0, 8, 8], false),
    row(cc(b"UYVY"), 0, 16, [0, 0, 8, 8, 8], false),
    row(cc(b"RGBG"), 0, 16, [0, 0, 8, 8, 8], false),
    row(cc(b"YUY2"), 0, 16, [0, 0, 8, 8, 8], false),
    row(cc(b"GRGB"), 0, 16, [0, 0, 8, 8, 8], false),
    row(cc(b"DXT1"), 0, 16, [0, 1, 5, 6, 5], false),
    row(cc(b"DXT2"), 0, 16, [0, 4, 5, 6, 5], false),
    row(cc(b"DXT3"), 0, 16, [0, 4, 5, 6, 5], false),
    row(cc(b"DXT4"), 0, 16, [0, 8, 5, 6, 5], false),
    row(cc(b"DXT5"), 0, 16, [0, 8, 5, 6, 5], false),
    row(70, 4, 16, [1, 0, 0, 0, 16], true),
    row(82, 4, 32, [1, 0, 0, 0, 32], false),
    row(71, 4, 32, [0, 0, 0, 0, 32], true),
    row(73, 4, 16, [0, 0, 0, 1, 15], true),
    row(75, 4, 32, [0, 0, 0, 8, 24], true),
    row(80, 4, 16, [0, 0, 0, 0, 16], true),
    row(77, 4, 32, [0, 0, 0, 0, 24], true),
    row(79, 4, 32, [0, 0, 0, 4, 24], true),
    row(83, 4, 32, [0, 0, 0, 8, 24], false),
    row(111, 0, 16, [0, 0, 16, 0, 0], false),
    row(112, 0, 32, [0, 0, 16, 16, 0], false),
    row(113, 0, 64, [0, 16, 16, 16, 16], false),
    row(114, 0, 32, [0, 0, 32, 0, 0], false),
    row(115, 0, 64, [0, 0, 32, 32, 0], false),
    row(116, 0, 128, [0, 32, 32, 32, 32], false),
    row(cc(b"AL16"), 2, 32, [0, 16, 16, 16, 16], false),
    row(cc(b" R16"), 0, 48, [0, 0, 16, 16, 16], false),
];

/// The formats of D3DX's format table, in its order: the ones a device is asked about (a caller
/// builds [`crate::Caps`]' lists from its device's answers for these).
pub fn table_formats() -> impl Iterator<Item = u32> {
    TABLE.iter().map(|r| r.format)
}

/// The row D3DX uses for a format its table does not have (0x406ae8).
const OTHER: Row = row(0, 0, 32, [0, 8, 8, 8, 8], false);

/// Which kinds may stand in for which (0x407318): `KINDS[wanted][candidate]`. Colour only for
/// colour; colour or palette for palette; colour or luminance for luminance; signed and depth
/// formats only for their own kind.
const KINDS: [[bool; 5]; 5] = [
    [true, false, false, false, false],
    [true, true, false, false, false],
    [true, false, true, false, false],
    [false, false, false, true, false],
    [false, false, false, false, true],
];

/// How far `candidate` is from `want` (0x60cf77, `want` in ECX): per channel the bits added
/// (0x100 for a channel `want` does not have), the bits lost times 0x10000 (0x1000000 for a
/// channel `candidate` does not have); `None` when the kinds do not match or every channel of
/// `want` is lost.
fn distance(want: &Row, candidate: &Row) -> Option<u32> {
    if !KINDS[want.kind][candidate.kind] {
        return None;
    }
    let (mut d, mut have) = (0u32, 0u32);
    for (&w, &c) in want.channels.iter().zip(&candidate.channels) {
        if w != 0 {
            have += 1;
        }
        if w < c {
            d += if w == 0 { 0x100 } else { c - w };
        } else if c < w {
            d += if c == 0 {
                0x100_0000
            } else {
                (w - c) * 0x1_0000
            };
        }
    }
    (have != d >> 24).then_some(d)
}

/// The format D3DX makes a texture of `wanted` in on a device that takes the formats `takes`
/// says (0x60d069): `wanted` when taken; else, in the table's order, the taken row that may stand
/// in with the smallest distance, a tie going to fewer bits per pixel; 0 when none (D3DX then
/// answers `D3DERR_NOTAVAILABLE`).
pub(crate) fn choose(wanted: u32, takes: impl Fn(u32) -> bool) -> u32 {
    let want = TABLE.iter().find(|r| r.format == wanted).unwrap_or(&OTHER);
    let (mut best, mut chosen) = (u32::MAX, &OTHER);
    for row in TABLE {
        if !takes(row.format) {
            continue;
        }
        if row.format == want.format {
            return row.format;
        }
        if !row.fallback {
            continue;
        }
        let Some(d) = distance(want, row) else {
            continue;
        };
        if d < best || (d == best && row.bits < chosen.bits) {
            best = d;
            chosen = row;
        }
    }
    chosen.format
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::format;
    use crate::{REFERENCE_CUBE_FORMATS, REFERENCE_TEXTURE_FORMATS};

    /// `data/device_formats_32bit.tsv` (`d3dx-measure formats` on the reference machine): for
    /// every format of the table, the device's `CheckDeviceFormat` answers for a texture and a
    /// cube texture, and the format the 32-bit DLL's `D3DXCheckTextureRequirements` and
    /// `D3DXCheckCubeTextureRequirements` picked (or `D3DERR_NOTAVAILABLE`). The reference lists
    /// are those answers, and on them [`choose`] picks what the DLL picked, every one.
    #[test]
    fn the_picks_are_the_32_bit_dlls_on_the_reference_device() {
        let data = include_str!("../data/device_formats_32bit.tsv");
        let hex = |s: &str| u32::from_str_radix(s.trim_start_matches("0x"), 16).unwrap();
        let mut n = 0;
        for line in data.lines().filter(|l| !l.starts_with('#')) {
            let f: Vec<u32> = line.split('\t').map(hex).collect();
            let format = f[0];
            assert_eq!(TABLE[n].format, format, "the table's order");
            for (list, answer, hr, pick) in [
                (REFERENCE_TEXTURE_FORMATS, f[1], f[3], f[4]),
                (REFERENCE_CUBE_FORMATS, f[2], f[5], f[6]),
            ] {
                assert_eq!(list.contains(&format), answer == 0, "{format:#x}");
                let ours = choose(format, |x| list.contains(&x));
                if hr == 0 {
                    assert_eq!(ours, pick, "{format:#x}");
                } else {
                    assert_eq!((ours, hr), (0, crate::D3DERR_NOTAVAILABLE), "{format:#x}");
                }
            }
            n += 1;
        }
        assert_eq!(n, TABLE.len());
    }

    /// On the reference device every format this crate reads becomes a texture of a format it
    /// makes (R8G8B8 asked as X8R8G8B8, as 0x61025e does), as a texture and as a cube.
    #[test]
    fn every_format_read_becomes_one_this_crate_makes() {
        for r in TABLE.iter().filter(|r| crate::filter::reads(r.format)) {
            let wanted = if r.format == format::R8G8B8 {
                format::X8R8G8B8
            } else {
                r.format
            };
            for list in [REFERENCE_TEXTURE_FORMATS, REFERENCE_CUBE_FORMATS] {
                let made = choose(wanted, |x| list.contains(&x));
                assert!(crate::filter::makes(made), "{:#x} -> {made:#x}", r.format);
            }
        }
    }

    /// A device that takes everything keeps every format of the table.
    #[test]
    fn a_format_the_device_takes_is_kept() {
        for r in TABLE {
            assert_eq!(choose(r.format, |_| true), r.format);
        }
    }

    /// The measure's pieces: a wider channel costs its extra bits, a missing one 0x100; a narrower
    /// one 0x10000 a bit, a lost one 0x1000000; other kinds and losing everything are refused.
    #[test]
    fn the_distance_is_d3dxs() {
        let find = |f: u32| TABLE.iter().find(|r| r.format == f).unwrap();
        // X4R4G4B4 -> R5G6B5: 1 + 2 + 1 bits more.
        assert_eq!(
            distance(find(format::X4R4G4B4), find(format::R5G6B5)),
            Some(4)
        );
        // X4R4G4B4 -> A4R4G4B4: an alpha channel it does not have.
        assert_eq!(
            distance(find(format::X4R4G4B4), find(format::A4R4G4B4)),
            Some(0x100)
        );
        // A8R8G8B8 -> R5G6B5: alpha lost, 3 + 2 + 3 bits fewer.
        assert_eq!(
            distance(find(format::A8R8G8B8), find(format::R5G6B5)),
            Some(0x100_0000 + 8 * 0x1_0000)
        );
        // Luminance may become colour, colour never luminance.
        assert!(distance(find(format::L8), find(format::X8R8G8B8)).is_some());
        assert_eq!(distance(find(format::X8R8G8B8), find(format::L8)), None);
        // A8 -> R5G6B5 loses its only channel.
        assert_eq!(distance(find(format::A8), find(format::R5G6B5)), None);
    }

    /// Without the format asked for, the nearest; a tie goes to the row with fewer bits.
    #[test]
    fn the_nearest_taken_format_stands_in() {
        let takes = |list: &'static [u32]| move |f: u32| list.contains(&f);
        // X4R4G4B4 on a device without it: R5G6B5 (4) before A4R4G4B4 (0x100) and X8R8G8B8 (12).
        assert_eq!(
            choose(
                format::X4R4G4B4,
                takes(&[
                    format::A8R8G8B8,
                    format::X8R8G8B8,
                    format::R5G6B5,
                    format::A4R4G4B4
                ])
            ),
            format::R5G6B5
        );
        // X1R5G5B5 between X8R8G8B8 (9) and A8B8G8R8's twin X8B8G8R8 (9): the first in the table.
        assert_eq!(
            choose(
                format::X1R5G5B5,
                takes(&[format::X8B8G8R8, format::X8R8G8B8])
            ),
            format::X8R8G8B8
        );
        // A1R5G5B5 between A8R8G8B8 (7 + 9) and A4R4G4B4 (3 + 3 * 0x10000): the nearer.
        assert_eq!(
            choose(
                format::A1R5G5B5,
                takes(&[format::A8R8G8B8, format::A4R4G4B4])
            ),
            format::A8R8G8B8
        );
        // Nothing taken that may stand in: none.
        assert_eq!(choose(format::R5G6B5, takes(&[format::L8])), 0);
    }
}
