//! Synthetic DDS files (built here from the documented layout, no game files) for checking this
//! crate against the 32-bit `d3dx9_38.dll`: `scripts/d3dx-tex-verify --capture-synthetic` runs
//! them through the DLL once and writes `data/synthetic_32bit.tsv`, which the tests compare with.
//! They cover what the game's textures need: DXT1/3/5 with the whole chain in the file (levels
//! under a block re-encoded with dither), with one level (the chain made by the box filter),
//! non-square sizes, transparent DXT1 pixels, A8R8G8B8 and A4R4G4B4 chains (integer and float box
//! paths), R8G8B8 converted to X8R8G8B8 with ordered dither, a DXT1 cube map, and files D3DX
//! refuses. [`formats`] adds the formats translated for mods' textures, in answers of their own.

/// A file: its name, whether it goes to the cube call, and its bytes.
pub struct File {
    pub name: String,
    pub cube: bool,
    pub data: Vec<u8>,
}

/// xorshift64: the same bytes on every run.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    fn byte(&mut self) -> u8 {
        (self.next() >> 24) as u8
    }
}

/// A DDS pixel format: its flags, FourCC, bit count and four masks.
type DdsPixelFormat = (u32, u32, u32, [u32; 4]);

/// The 128-byte DDS header (with the magic) for a `w` x `h` file of `mips` levels; `pf` is the
/// pixel format's flags, FourCC, bit count and four masks; `caps2` the cube bits.
fn header(w: u32, h: u32, mips: u32, pf: (u32, u32, u32, [u32; 4]), caps2: u32) -> Vec<u8> {
    let mut d = vec![0u8; 128];
    let mut put = |at: usize, v: u32| d[at..at + 4].copy_from_slice(&v.to_le_bytes());
    put(0, u32::from_le_bytes(*b"DDS "));
    put(4, 124);
    put(8, 0x1007 | if mips > 1 { 0x2_0000 } else { 0 });
    put(12, h);
    put(16, w);
    put(28, mips);
    put(76, 32);
    put(80, pf.0);
    put(84, pf.1);
    put(88, pf.2);
    for (i, m) in pf.3.iter().enumerate() {
        put(92 + 4 * i, *m);
    }
    put(108, 0x1000 | if mips > 1 { 0x40_0008 } else { 0 });
    put(112, caps2);
    d
}

fn fourcc(f: &[u8; 4]) -> (u32, u32, u32, [u32; 4]) {
    (4, u32::from_le_bytes(*f), 0, [0; 4])
}

/// DXT blocks with varied content: smooth gradients, edges, noise and flat runs, endpoints of
/// either order (DXT1's three-colour mode included), alpha in every form.
fn dxt_blocks(rng: &mut Rng, n: usize, bytes: usize, transparent: bool) -> Vec<u8> {
    let mut out = Vec::with_capacity(n * bytes);
    for i in 0..n {
        let mut b = vec![0u8; bytes];
        for x in b.iter_mut() {
            *x = rng.byte();
        }
        let c = bytes - 8;
        match i % 5 {
            // A flat block.
            0 => {
                b[c + 2] = b[c];
                b[c + 3] = b[c + 1];
            }
            // Close endpoints (the 5:6:5 rounding cases).
            1 => {
                b[c + 2] = b[c].wrapping_add(1);
                b[c + 3] = b[c + 1];
            }
            // Endpoints in the order DXT1 reads as three colours and transparent black.
            2 if transparent && b[c + 1] > b[c + 3] => {
                b.swap(c, c + 2);
                b.swap(c + 1, c + 3);
            }
            _ => {}
        }
        if bytes == 16 && i % 7 == 3 {
            // DXT5: the six-value alpha mode, or opaque.
            b[0] = b[1] / 2;
        }
        out.extend_from_slice(&b);
    }
    out
}

/// A DXT file of `fmt` (`w` x `h`, `mips` levels stored).
fn dxt(rng: &mut Rng, fmt: &[u8; 4], w: u32, h: u32, mips: u32) -> Vec<u8> {
    let bytes = if fmt == b"DXT1" { 8 } else { 16 };
    let mut d = header(w, h, mips, fourcc(fmt), 0);
    for l in 0..mips {
        let (lw, lh) = ((w >> l).max(1), (h >> l).max(1));
        let n = (lw.div_ceil(4) * lh.div_ceil(4)) as usize;
        d.extend(dxt_blocks(rng, n, bytes, fmt == b"DXT1"));
    }
    d
}

/// An uncompressed file: `bpp` bytes per pixel, a smooth picture with noise.
fn plain(rng: &mut Rng, pf: (u32, u32, u32, [u32; 4]), w: u32, h: u32, mips: u32) -> Vec<u8> {
    let bpp = (pf.2 / 8) as usize;
    let mut d = header(w, h, mips, pf, 0);
    for l in 0..mips {
        let (lw, lh) = ((w >> l).max(1), (h >> l).max(1));
        for y in 0..lh {
            for x in 0..lw {
                for k in 0..bpp {
                    let v = (x * 255 / lw.max(1)) as u8 ^ (y * 97 / lh.max(1)) as u8;
                    d.push(v.wrapping_add((k as u8) * 61).wrapping_add(rng.byte() & 7));
                }
            }
        }
    }
    d
}

/// The synthetic set.
pub fn files() -> Vec<File> {
    let mut rng = Rng(0x5eed_d3d8_0909_2008);
    let mut v = Vec::new();
    let mut add = |name: &str, cube: bool, data: Vec<u8>| {
        v.push(File {
            name: name.into(),
            cube,
            data,
        })
    };
    // The whole chain stored: levels under a block re-encoded with dither.
    add("dxt1-64-full", false, dxt(&mut rng, b"DXT1", 64, 64, 7));
    add("dxt3-32-full", false, dxt(&mut rng, b"DXT3", 32, 32, 6));
    add("dxt5-32-full", false, dxt(&mut rng, b"DXT5", 32, 32, 6));
    add("dxt1-64x16-full", false, dxt(&mut rng, b"DXT1", 64, 16, 7));
    add("dxt5-16x64-full", false, dxt(&mut rng, b"DXT5", 16, 64, 7));
    // One level: the chain from the box filter.
    add("dxt1-128-one", false, dxt(&mut rng, b"DXT1", 128, 128, 1));
    add("dxt3-64-one", false, dxt(&mut rng, b"DXT3", 64, 64, 1));
    add("dxt5-128-one", false, dxt(&mut rng, b"DXT5", 128, 128, 1));
    add("dxt5-256x32-one", false, dxt(&mut rng, b"DXT5", 256, 32, 1));
    // Some levels stored, the rest made.
    add("dxt1-64-three", false, dxt(&mut rng, b"DXT1", 64, 64, 3));
    add("dxt5-64-four", false, dxt(&mut rng, b"DXT5", 64, 64, 4));
    // Uncompressed.
    let argb = (0x41, 0, 32, [0xff_0000, 0xff00, 0xff, 0xff00_0000]);
    let xrgb = (0x40, 0, 32, [0xff_0000, 0xff00, 0xff, 0]);
    let rgb = (0x40, 0, 24, [0xff_0000, 0xff00, 0xff, 0]);
    let argb4 = (0x41, 0, 16, [0xf00, 0xf0, 0xf, 0xf000]);
    add("a8r8g8b8-32-one", false, plain(&mut rng, argb, 32, 32, 1));
    add("a8r8g8b8-64x4-one", false, plain(&mut rng, argb, 64, 4, 1));
    add("a8r8g8b8-16-full", false, plain(&mut rng, argb, 16, 16, 5));
    add("x8r8g8b8-32x8-one", false, plain(&mut rng, xrgb, 32, 8, 1));
    add("a4r4g4b4-32-one", false, plain(&mut rng, argb4, 32, 32, 1));
    add("a4r4g4b4-8x32-one", false, plain(&mut rng, argb4, 8, 32, 1));
    add("r8g8b8-16-one", false, plain(&mut rng, rgb, 16, 16, 1));
    add("r8g8b8-8x2-full", false, plain(&mut rng, rgb, 8, 2, 4));
    // A DXT1 cube map with its chain down to 4x4, as the game's.
    {
        let mut d = header(16, 16, 3, fourcc(b"DXT1"), 0xfe00);
        for _ in 0..6 {
            for l in 0..3u32 {
                let s = (16u32 >> l).max(1);
                let n = (s.div_ceil(4) * s.div_ceil(4)) as usize;
                d.extend(dxt_blocks(&mut rng, n, 8, true));
            }
        }
        add("dxt1-cube-16-three", true, d);
    }
    // Refused: cut one byte short; the cube call on a file with one face.
    let mut short = dxt(&mut rng, b"DXT5", 16, 16, 5);
    short.pop();
    add("dxt5-16-short", false, short);
    add(
        "dxt1-16-not-a-cube",
        true,
        dxt(&mut rng, b"DXT1", 16, 16, 5),
    );
    v
}

/// A cube map of an uncompressed format: six faces of `plain` pictures, `s` a side, `mips`
/// levels each.
fn plain_cube(rng: &mut Rng, pf: (u32, u32, u32, [u32; 4]), s: u32, mips: u32) -> Vec<u8> {
    let mut d = header(s, s, mips, pf, 0xfe00);
    for _ in 0..6 {
        d.extend_from_slice(&plain(rng, pf, s, s, mips)[128..]);
    }
    d
}

/// Files of the formats translated for mods' textures (2026-11-11), through the DLL once like
/// [`files`] (`d3dx-tex-verify --capture-formats`, `data/formats_32bit.tsv` and
/// `formats_32bit_double.tsv`). For each format: the whole chain stored (copied as it is), one
/// level (the box filter's integer path where the format has one), levels 4 and 2 high (down to
/// levels one pixel high: the box filter's path through the working colour, the codec's decoder
/// and encoder without dither), some levels stored, and a cube map; then files of edge values
/// ([`edges`]). The 16-bit formats first, then the luminance and alpha ones. X4R4G4B4 and A4L4
/// are read into X1R5G5B5 and A4R4G4B4 (the formats D3DX picks on the reference device) with the
/// ordered dither. The pixels are noise over gradients, so the X bits are set in some.
pub fn formats() -> Vec<File> {
    let mut rng = Rng(0x5eed_d3d8_16b1_2026);
    let mut v = Vec::new();
    let sixteen = [
        ("r5g6b5", (0x40, 0, 16, [0xf800, 0x7e0, 0x1f, 0])),
        ("x1r5g5b5", (0x40, 0, 16, [0x7c00, 0x3e0, 0x1f, 0])),
        ("a1r5g5b5", (0x41, 0, 16, [0x7c00, 0x3e0, 0x1f, 0x8000])),
        ("x4r4g4b4", (0x40, 0, 16, [0xf00, 0xf0, 0xf, 0])),
    ];
    let luminance = [
        ("l8", (0x2_0000, 0, 8, [0xff, 0, 0, 0])),
        ("a8l8", (0x2_0001, 0, 16, [0xff, 0, 0, 0xff00])),
        ("a8", (0x2, 0, 8, [0, 0, 0, 0xff])),
        ("a4l4", (0x2_0001, 0, 8, [0xf, 0, 0, 0xf0])),
        ("l16", (0x2_0000, 0, 16, [0xffff, 0, 0, 0])),
    ];
    // In this order, so each file's bytes stay those its answer was captured for.
    format_group(&mut rng, &mut v, &sixteen, false);
    format_group(&mut rng, &mut v, &sixteen, true);
    format_group(&mut rng, &mut v, &luminance, false);
    format_group(&mut rng, &mut v, &luminance, true);
    v
}

/// The files of [`formats`] for a group of formats: the plain ones, or (`edge`) the edge-value
/// ones in levels one pixel high or wide that the file stores, and in a square one.
fn format_group(rng: &mut Rng, v: &mut Vec<File>, group: &[(&str, DdsPixelFormat)], edge: bool) {
    let mut add = |name: String, cube: bool, data: Vec<u8>| v.push(File { name, cube, data });
    for &(name, pf) in group {
        if edge {
            add(
                format!("{name}-64x1-edges-one"),
                false,
                edges(rng, pf, 64, 1, 1),
            );
            add(
                format!("{name}-1x32-edges-one"),
                false,
                edges(rng, pf, 1, 32, 1),
            );
            add(
                format!("{name}-16-edges-one"),
                false,
                edges(rng, pf, 16, 16, 1),
            );
            continue;
        }
        add(format!("{name}-16-full"), false, plain(rng, pf, 16, 16, 5));
        add(format!("{name}-32-one"), false, plain(rng, pf, 32, 32, 1));
        add(format!("{name}-64x4-one"), false, plain(rng, pf, 64, 4, 1));
        add(
            format!("{name}-256x2-one"),
            false,
            plain(rng, pf, 256, 2, 1),
        );
        add(
            format!("{name}-8x32-three"),
            false,
            plain(rng, pf, 8, 32, 3),
        );
        add(
            format!("{name}-cube-8-one"),
            true,
            plain_cube(rng, pf, 8, 1),
        );
    }
}

/// Edge values of the 16-bit formats: every bit, none, the top bit (an X bit) alone and with the
/// rest, each field full alone, one bit of each field, and others near the ends.
const EDGES16: [u16; 14] = [
    0xffff, 0x0000, 0x8000, 0x7fff, 0xf800, 0x07e0, 0x001f, 0x7c00, 0x03e0, 0x8421, 0x7bde, 0xfffe,
    0x0001, 0x8001,
];

/// Edge values of the 8-bit formats: every bit, none, the top bit alone and with the rest, a
/// nibble full alone, and others near the ends.
const EDGES8: [u8; 10] = [0xff, 0x00, 0x80, 0x7f, 0xf0, 0x0f, 0x01, 0xfe, 0x81, 0x7e];

/// A file of edge values ([`EDGES16`], [`EDGES8`] by the format's bit count) and some noise, one
/// value per 2x2 cell (a box of a cell averages it unchanged): levels one pixel high or wide
/// reach the box filter's path through the working colour with their X bits set and their fields
/// at the ends.
fn edges(rng: &mut Rng, pf: (u32, u32, u32, [u32; 4]), w: u32, h: u32, mips: u32) -> Vec<u8> {
    let wide = pf.2 == 16;
    let mut d = header(w, h, mips, pf, 0);
    for l in 0..mips {
        let (lw, lh) = ((w >> l).max(1), (h >> l).max(1));
        let cw = lw.div_ceil(2);
        let cells: Vec<u16> = (0..cw * lh.div_ceil(2))
            .map(|i| match (i % 4 == 3, wide) {
                (true, true) => rng.next() as u16,
                (true, false) => u16::from(rng.next() as u8),
                (false, true) => EDGES16[i as usize % EDGES16.len()],
                (false, false) => u16::from(EDGES8[i as usize % EDGES8.len()]),
            })
            .collect();
        for y in 0..lh {
            for x in 0..lw {
                let c = cells[((y / 2) * cw + x / 2) as usize].to_le_bytes();
                d.extend_from_slice(if wide { &c } else { &c[..1] });
            }
        }
    }
    d
}

/// Headers whose sizes push D3DX's DDS loader (`data/headers_32bit.tsv`: the 32-bit DLL's image
/// info and create answers, `d3dx-tex-verify --capture-headers`): sides of 0 (levels of no
/// bytes), levels between 2^31 and 2^32 bytes, levels past 2^32 bytes (the loader's 32-bit
/// arithmetic wraps them, into the file or not), and many levels.
pub fn header_files() -> Vec<File> {
    let mut rng = Rng(0x0dd5_17e5_0000_0001);
    let dxt1 = fourcc(b"DXT1");
    let argb = (0x41, 0, 32, [0xff_0000, 0xff00, 0xff, 0xff00_0000]);
    let f128 = (4, 116, 0, [0; 4]);
    let fourcc117 = (4, 117, 0, [0; 4]);
    let cases: Vec<(&str, Vec<u8>, usize)> = vec![
        ("dxt1-8x0", header(8, 0, 1, dxt1, 0), 64),
        ("dxt1-8x0-4-levels", header(8, 0, 4, dxt1, 0), 64),
        ("dxt1-0x8", header(0, 8, 1, dxt1, 0), 64),
        ("argb-8x0", header(8, 0, 1, argb, 0), 64),
        ("argb-0x0", header(0, 0, 1, argb, 0), 64),
        ("dxt1-0x80000001x0", header(0x8000_0001, 0, 1, dxt1, 0), 64),
        (
            "argb-32768x24576-3gib",
            header(32768, 24576, 1, argb, 0),
            4096,
        ),
        (
            "dxt1-65536x98304-3gib",
            header(65536, 98304, 1, dxt1, 0),
            4096,
        ),
        (
            "argb-65536x16385-wraps-into-the-file",
            header(65536, 16385, 1, argb, 0),
            0x4_1000,
        ),
        (
            "argb-65536x16385-wraps-past-the-file",
            header(65536, 16385, 1, argb, 0),
            4096,
        ),
        (
            "argb-65536x65536-wraps-to-0",
            header(65536, 65536, 1, argb, 0),
            4096,
        ),
        (
            "dxt1-131072-wraps-to-0",
            header(131072, 131072, 1, dxt1, 0),
            4096,
        ),
        (
            "argb-2^31-square",
            header(0x8000_0000, 0x8000_0000, 1, argb, 0),
            64,
        ),
        (
            "f128-max-square",
            header(u32::MAX, u32::MAX, 1, f128, 0),
            64,
        ),
        ("dxt1-8x0-65536-levels", header(8, 0, 1 << 16, dxt1, 0), 64),
        ("dxt1-8x0-262144-levels", header(8, 0, 1 << 18, dxt1, 0), 64),
        (
            "argb-1x1-65536-levels",
            header(1, 1, 1 << 16, argb, 0),
            4 << 16,
        ),
        ("argb-1x1-max-levels", header(1, 1, u32::MAX, argb, 0), 64),
        ("dxt1-4x4-max-levels", header(4, 4, u32::MAX, dxt1, 0), 64),
        (
            "fourcc117-4x4-50m-levels",
            header(4, 4, 50_000_000, fourcc117, 0),
            64,
        ),
    ];
    cases
        .into_iter()
        .map(|(name, mut data, extra)| {
            data.extend((0..extra).map(|_| rng.byte()));
            File {
                name: name.into(),
                cube: false,
                data,
            }
        })
        .collect()
}

/// FNV-1a, 64 bits (the test data's level hashes).
pub fn fnv(b: &[u8]) -> u64 {
    b.iter().fold(0xcbf2_9ce4_8422_2325u64, |h, &x| {
        (h ^ u64::from(x)).wrapping_mul(0x100_0000_01b3)
    })
}
