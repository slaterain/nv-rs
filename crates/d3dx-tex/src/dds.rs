//! Reading a DDS file, as D3DX's DDS loader does (`d3dx9_38.dll` 0x621052, called from its image
//! reader 0x622d99, which tries the image file formats in turn).
//!
//! What it checks and gives:
//! - at least 4 + 124 bytes and the magic `DDS `;
//! - width (+16), height (+12); depth (+24) only when `DDSD_DEPTH` is set, 0 counting as 1;
//! - a cube map when all six face bits of `dwCaps2` are set (the `DDSCAPS2_CUBEMAP` bit itself is
//!   not looked at); some face bits but not all: refused;
//! - the mip count (+28), 0 counting as 1, whatever `DDSD_MIPMAPCOUNT` says;
//! - the pixel format by D3DX's table ([`PIXEL_FORMATS`], `pfSize` must be 32; with
//!   `DDPF_FOURCC` set every other flag is dropped);
//! - every level of every face must be in the data, or the file is refused.
//!
//! A refused file is tried by D3DX's other image readers and ends as `D3DXERR_INVALIDDATA`
//! ([`crate::D3DXERR_INVALIDDATA`]) for anything that is not one of those formats.

use crate::format;

/// One row of D3DX's DDS pixel format table (0x787758): the `D3DFORMAT`, the `DDPF_*` flags, the
/// FourCC, the bit count and the red, green, blue and alpha masks.
pub struct PixelFormat {
    pub format: u32,
    pub flags: u32,
    pub fourcc: u32,
    pub bits: u32,
    pub masks: [u32; 4],
}

const fn pf(format: u32, flags: u32, fourcc: u32, bits: u32, masks: [u32; 4]) -> PixelFormat {
    PixelFormat {
        format,
        flags,
        fourcc,
        bits,
        masks,
    }
}

const fn cc(f: u32) -> PixelFormat {
    pf(f, 4, f, 0, [0; 4])
}

/// D3DX's table, in its order (the first match wins).
pub const PIXEL_FORMATS: &[PixelFormat] = &[
    pf(format::R8G8B8, 0x40, 0, 24, [0xff_0000, 0xff00, 0xff, 0]),
    pf(
        format::A8R8G8B8,
        0x41,
        0,
        32,
        [0xff_0000, 0xff00, 0xff, 0xff00_0000],
    ),
    pf(format::X8R8G8B8, 0x40, 0, 32, [0xff_0000, 0xff00, 0xff, 0]),
    pf(format::R5G6B5, 0x40, 0, 16, [0xf800, 0x7e0, 0x1f, 0]),
    pf(format::X1R5G5B5, 0x40, 0, 16, [0x7c00, 0x3e0, 0x1f, 0]),
    pf(format::A1R5G5B5, 0x41, 0, 16, [0x7c00, 0x3e0, 0x1f, 0x8000]),
    pf(format::A4R4G4B4, 0x41, 0, 16, [0xf00, 0xf0, 0xf, 0xf000]),
    pf(format::R3G3B2, 0x40, 0, 8, [0xe0, 0x1c, 0x3, 0]),
    pf(format::A8, 0x2, 0, 8, [0, 0, 0, 0xff]),
    pf(format::A8R3G3B2, 0x41, 0, 16, [0xe0, 0x1c, 0x3, 0xff00]),
    pf(format::X4R4G4B4, 0x40, 0, 16, [0xf00, 0xf0, 0xf, 0]),
    pf(
        format::A2B10G10R10,
        0x41,
        0,
        32,
        [0x3ff0_0000, 0xffc00, 0x3ff, 0xc000_0000],
    ),
    pf(
        format::A8B8G8R8,
        0x41,
        0,
        32,
        [0xff, 0xff00, 0xff_0000, 0xff00_0000],
    ),
    pf(format::X8B8G8R8, 0x40, 0, 32, [0xff, 0xff00, 0xff_0000, 0]),
    pf(format::G16R16, 0x40, 0, 32, [0xffff, 0xffff_0000, 0, 0]),
    pf(
        format::A2R10G10B10,
        0x41,
        0,
        32,
        [0x3ff, 0xffc00, 0x3ff0_0000, 0xc000_0000],
    ),
    pf(format::A8P8, 0x21, 0, 16, [0, 0, 0, 0xff00]),
    pf(format::P8, 0x20, 0, 8, [0; 4]),
    pf(format::L8, 0x2_0000, 0, 8, [0xff, 0, 0, 0]),
    pf(format::A8L8, 0x2_0001, 0, 16, [0xff, 0, 0, 0xff00]),
    pf(format::A4L4, 0x2_0001, 0, 8, [0xf, 0, 0, 0xf0]),
    pf(format::L16, 0x2_0000, 0, 16, [0xffff, 0, 0, 0]),
    pf(format::V8U8, 0x8_0000, 0, 16, [0xff, 0xff00, 0, 0]),
    pf(format::L6V5U5, 0x4_0000, 0, 16, [0x1f, 0x3e0, 0xfc00, 0]),
    pf(
        format::X8L8V8U8,
        0x4_0000,
        0,
        32,
        [0xff, 0xff00, 0xff_0000, 0],
    ),
    pf(
        format::Q8W8V8U8,
        0x8_0000,
        0,
        32,
        [0xff, 0xff00, 0xff_0000, 0xff00_0000],
    ),
    pf(format::V16U16, 0x8_0000, 0, 32, [0xffff, 0xffff_0000, 0, 0]),
    pf(
        format::A2W10V10U10,
        0x8_0001,
        0,
        32,
        [0x3ff0_0000, 0xffc00, 0x3ff, 0xc000_0000],
    ),
    cc(u32::from_le_bytes(*b"UYVY")),
    cc(u32::from_le_bytes(*b"RGBG")),
    cc(u32::from_le_bytes(*b"YUY2")),
    cc(u32::from_le_bytes(*b"GRGB")),
    cc(format::DXT1),
    cc(format::DXT2),
    cc(format::DXT3),
    cc(format::DXT4),
    cc(format::DXT5),
    pf(70, 0x400, 0, 16, [0, 0xffff, 0, 0]),
    cc(format::L16 + 1),
    cc(format::A16B16G16R16),
    cc(format::Q16W16V16U16),
    cc(format::R16F),
    cc(format::G16R16F),
    cc(format::A16B16G16R16F),
    cc(format::R32F),
    cc(format::G32R32F),
    cc(format::A32B32G32R32F),
    cc(117),
];

/// `D3DRESOURCETYPE`: a texture, a volume texture, a cube texture.
pub const RTYPE_TEXTURE: u32 = 3;
pub const RTYPE_VOLUME: u32 = 4;
pub const RTYPE_CUBE: u32 = 5;

/// One level of one face in the file: its size and where its bytes are.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Level {
    pub width: u32,
    pub height: u32,
    pub depth: u32,
    /// Offset of the level's bytes in the file.
    pub offset: usize,
    /// Bytes per row (of blocks, for the block formats).
    pub pitch: usize,
    /// Bytes per slice.
    pub slice: usize,
}

/// A DDS file as D3DX reads it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Image {
    pub width: u32,
    pub height: u32,
    pub depth: u32,
    /// The file's level count (its mip count, 0 counting as 1).
    pub mip_levels: u32,
    /// The pixel format's `D3DFORMAT` (from [`PIXEL_FORMATS`]).
    pub format: u32,
    /// [`RTYPE_TEXTURE`], [`RTYPE_VOLUME`] or [`RTYPE_CUBE`].
    pub resource_type: u32,
    /// 1, or 6 for a cube map.
    pub faces: u32,
    /// Face by face, level by level.
    pub levels: Vec<Level>,
}

fn word(d: &[u8], at: usize) -> u32 {
    u32::from_le_bytes(d[at..at + 4].try_into().unwrap())
}

/// The row and slice bytes of a level of `format` (D3DX's DDS loader's sizes).
fn level_bytes(format: u32, bits: u32, w: u32, h: u32) -> (usize, usize) {
    let (w, h) = (w as usize, h as usize);
    match format {
        format::DXT1 => {
            let row = w.div_ceil(4) * 8;
            (row, h.div_ceil(4) * row)
        }
        format::DXT2 | format::DXT3 | format::DXT4 | format::DXT5 => {
            let row = w.div_ceil(4) * 16;
            (row, h.div_ceil(4) * row)
        }
        f if f == u32::from_le_bytes(*b"UYVY")
            || f == u32::from_le_bytes(*b"YUY2")
            || f == u32::from_le_bytes(*b"RGBG")
            || f == u32::from_le_bytes(*b"GRGB") =>
        {
            let row = w.div_ceil(2) * 4;
            (row, row * h)
        }
        _ => {
            let row = (bits as usize >> 3) * w;
            (row, row * h)
        }
    }
}

/// The bits per pixel D3DX's format table gives the formats it reads; 0 for the block formats.
fn format_bits(format: u32) -> u32 {
    match format {
        format::R8G8B8 => 24,
        format::A8R8G8B8
        | format::X8R8G8B8
        | format::A2B10G10R10
        | format::A8B8G8R8
        | format::X8B8G8R8
        | format::G16R16
        | format::A2R10G10B10
        | format::X8L8V8U8
        | format::Q8W8V8U8
        | format::V16U16
        | format::A2W10V10U10
        | format::G16R16F
        | format::R32F => 32,
        format::R5G6B5
        | format::X1R5G5B5
        | format::A1R5G5B5
        | format::A4R4G4B4
        | format::A8R3G3B2
        | format::X4R4G4B4
        | format::A8P8
        | format::A8L8
        | format::V8U8
        | format::L6V5U5
        | format::L16
        | format::R16F
        | 70
        | 82 => 16,
        format::R3G3B2 | format::A8 | format::P8 | format::L8 | format::A4L4 => 8,
        format::A16B16G16R16 | format::Q16W16V16U16 | format::A16B16G16R16F | format::G32R32F => 64,
        format::A32B32G32R32F => 128,
        _ => 0,
    }
}

/// Reads `data` as D3DX's DDS loader does; `None` when the loader refuses it (D3DX then answers
/// `D3DXERR_INVALIDDATA`, its other image readers refusing it too).
pub fn parse(data: &[u8]) -> Option<Image> {
    if data.len() < 4 || word(data, 0) != u32::from_le_bytes(*b"DDS ") || data.len() - 4 < 0x7c {
        return None;
    }
    let width = word(data, 16);
    let height = word(data, 12);
    let flags = word(data, 8);
    let depth = if flags & 0x80_0000 == 0 {
        1
    } else {
        word(data, 24).max(1)
    };
    let caps2 = word(data, 112);
    let faces = match caps2 & 0xfc00 {
        0xfc00 => 6,
        0 => 1,
        _ => return None,
    };
    let mips = word(data, 28).max(1);
    let mut pf_flags = word(data, 80);
    if pf_flags & 4 != 0 {
        pf_flags = 4;
    }
    let (pf_size, fourcc, bits) = (word(data, 76), word(data, 84), word(data, 88));
    let masks = [92, 96, 100, 104].map(|a| word(data, a));
    let entry = PIXEL_FORMATS.iter().find(|e| {
        !(pf_size != 0x20
            || e.flags != pf_flags
            || (pf_flags & 4 != 0 && fourcc != e.fourcc)
            || (pf_flags & 0xc4462 != 0 && bits != e.bits)
            || (pf_flags & 0xe4040 != 0 && masks[0] != e.masks[0])
            || (pf_flags & 0xc4440 != 0 && masks[1] != e.masks[1])
            || (pf_flags & 0xc4040 != 0 && masks[2] != e.masks[2])
            || (pf_flags & 0x80003 != 0 && masks[3] != e.masks[3]))
    })?;
    let format = entry.format;
    let resource_type = if flags & 0x80_0000 != 0 {
        RTYPE_VOLUME
    } else if faces == 6 {
        RTYPE_CUBE
    } else {
        RTYPE_TEXTURE
    };
    // A palette of 256 colours follows the header for the palette formats.
    let mut at = 128usize;
    let mut left = data.len() - 128;
    if format == format::P8 || format == format::A8P8 {
        if left < 0x400 {
            return None;
        }
        at += 0x400;
        left -= 0x400;
    }
    let bits = format_bits(format);
    let mut levels = Vec::with_capacity((faces * mips) as usize);
    for _ in 0..faces {
        let (mut w, mut h, mut d) = (width, height, depth);
        for _ in 0..mips {
            let (pitch, slice) = level_bytes(format, bits, w, h);
            let size = slice * d as usize;
            if left < size {
                return None;
            }
            levels.push(Level {
                width: w,
                height: h,
                depth: d,
                offset: at,
                pitch,
                slice,
            });
            at += size;
            left -= size;
            w = (w >> 1).max(1);
            h = (h >> 1).max(1);
            d = (d >> 1).max(1);
        }
    }
    Some(Image {
        width,
        height,
        depth,
        mip_levels: mips,
        format,
        resource_type,
        faces,
        levels,
    })
}
