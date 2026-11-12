//! What `D3DXCreateTextureFromFileInMemory` and `D3DXCreateCubeTextureFromFileInMemory` do
//! (`d3dx9_38.dll` 0x611266 / 0x611479, both through 0x61025e) with the defaults those calls
//! pass: size and level count `D3DX_DEFAULT` (the file's size rounded up to a power of two,
//! the whole mip chain), usage 0, the file's format, `D3DPOOL_MANAGED`, filter
//! `D3DX_FILTER_TRIANGLE | D3DX_FILTER_DITHER`, mip filter `D3DX_FILTER_BOX`, no colour key.
//!
//! The texture's size and format go through `D3DXCheckTextureRequirements` (0x60d6b8): DXT
//! sizes round up to whole blocks, a cube is square, the device's limits and the formats it takes
//! ([`Caps`]; the format D3DX picks: [`crate::device`]). Each level the file has is loaded into
//! the texture's level (`D3DXLoadSurfaceFromMemory`): as it is or converted when the sizes match
//! ([`filter::load_same_size`]), resampled with the triangle filter when they do not (a side that
//! is not a power of two, a DXT side not a multiple of 4: [`crate::triangle`]); the rest of the
//! chain comes from `D3DXFilterTexture` with the box filter, each level from the one before
//! ([`filter::box_next`]).

use crate::dds::{self, RTYPE_CUBE, RTYPE_TEXTURE};
use crate::filter::{self, Level};
use crate::format;
use crate::num::{Precision, P24, P53};
use crate::triangle;

/// What the device lets D3DX make (`IDirect3DDevice9::GetDeviceCaps` and `CheckDeviceFormat`
/// answers D3DX asks for).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Caps {
    /// `MaxTextureWidth`, `MaxTextureHeight`.
    pub max_width: u32,
    pub max_height: u32,
    /// `D3DPTEXTURECAPS_POW2`: texture sides must be powers of two (D3DX rounds them up).
    pub pow2: bool,
    /// The formats the device takes for a texture and for a cube texture: of D3DX's format table,
    /// those `IDirect3D9::CheckDeviceFormat` answers yes for (usage 0, on the display mode's
    /// format), as D3DX asks (0x60d069).
    pub texture_formats: Vec<u32>,
    pub cube_formats: Vec<u32>,
}

impl Default for Caps {
    /// The Direct3D 9 GPUs the game runs on today: 16384 texels a side, any size; the formats the
    /// reference machine's device takes ([`REFERENCE_TEXTURE_FORMATS`]).
    fn default() -> Self {
        Caps {
            max_width: 16384,
            max_height: 16384,
            pow2: false,
            texture_formats: REFERENCE_TEXTURE_FORMATS.to_vec(),
            cube_formats: REFERENCE_CUBE_FORMATS.to_vec(),
        }
    }
}

/// The formats of D3DX's table the reference machine's Direct3D 9 device takes for a texture
/// (an NVIDIA RTX 5070 Ti, measured 2026-11-11 with `d3dx-measure formats`:
/// `data/device_formats_32bit.tsv`). Not taken, among others: R8G8B8, X4R4G4B4, A8B8G8R8,
/// X8B8G8R8, R3G3B2, A8R3G3B2, A4L4, the palette and YUV formats; D3DX then picks the nearest
/// ([`crate::device`]).
pub const REFERENCE_TEXTURE_FORMATS: &[u32] = &[
    format::A8R8G8B8,
    format::X8R8G8B8,
    format::R5G6B5,
    format::X1R5G5B5,
    format::A1R5G5B5,
    format::A4R4G4B4,
    format::A8,
    format::A2B10G10R10,
    format::G16R16,
    format::A2R10G10B10,
    format::A16B16G16R16,
    format::L8,
    format::A8L8,
    format::L16,
    format::V8U8,
    format::X8L8V8U8,
    format::Q8W8V8U8,
    format::V16U16,
    format::DXT1,
    format::DXT2,
    format::DXT3,
    format::DXT4,
    format::DXT5,
    // D32F_LOCKABLE, D24S8, D16, D24X8.
    82,
    75,
    80,
    77,
    format::R16F,
    format::G16R16F,
    format::A16B16G16R16F,
    format::R32F,
    format::G32R32F,
    format::A32B32G32R32F,
];

/// The same for a cube texture: [`REFERENCE_TEXTURE_FORMATS`] without V8U8, X8L8V8U8 and the depth
/// formats.
pub const REFERENCE_CUBE_FORMATS: &[u32] = &[
    format::A8R8G8B8,
    format::X8R8G8B8,
    format::R5G6B5,
    format::X1R5G5B5,
    format::A1R5G5B5,
    format::A4R4G4B4,
    format::A8,
    format::A2B10G10R10,
    format::G16R16,
    format::A2R10G10B10,
    format::A16B16G16R16,
    format::L8,
    format::A8L8,
    format::L16,
    format::Q8W8V8U8,
    format::V16U16,
    format::DXT1,
    format::DXT2,
    format::DXT3,
    format::DXT4,
    format::DXT5,
    format::R16F,
    format::G16R16F,
    format::A16B16G16R16F,
    format::R32F,
    format::G32R32F,
    format::A32B32G32R32F,
];

/// Why a texture could not be made.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// D3DX's own answer for this file (the game gets it).
    D3dx(u32),
    /// Something D3DX does that this crate does not translate (yet): its kind and the details;
    /// the caller decides (on Windows the player's DLL can still answer).
    NotProvided(Untranslated, String),
}

/// What D3DX does here that this crate leaves to it, by kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Untranslated {
    /// An image file that is not DDS (BMP, TGA, PNG, JPG, ...).
    ImageFile,
    /// A volume texture.
    Volume,
    /// A DDS pixel format this crate does not read or make.
    PixelFormat,
}

/// `D3DXIMAGE_INFO`'s seven words.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ImageInfo {
    pub width: u32,
    pub height: u32,
    pub depth: u32,
    pub mip_levels: u32,
    pub format: u32,
    pub resource_type: u32,
    pub image_file_format: u32,
}

impl ImageInfo {
    pub fn words(&self) -> [u32; 7] {
        [
            self.width,
            self.height,
            self.depth,
            self.mip_levels,
            self.format,
            self.resource_type,
            self.image_file_format,
        ]
    }
}

/// One level of one face: format, size and bytes, rows packed (rows of blocks for DXT).
pub type Surface = Level;

/// The texture D3DX makes: its level count, face count (1, or 6 for a cube) and surfaces face
/// by face, level by level (the DDS order for a cube: +X, -X, +Y, -Y, +Z, -Z).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Texture {
    pub levels: u32,
    pub faces: u32,
    pub surfaces: Vec<Surface>,
}

/// `D3DXIFF_DDS`.
const IFF_DDS: u32 = 4;

fn is_dds(data: &[u8]) -> bool {
    data.len() >= 4 && &data[..4] == b"DDS "
}

/// Reads a file the way D3DX's image reader does. Only DDS files are translated; any other
/// image file format D3DX reads is [`Error::NotProvided`].
fn read(data: &[u8]) -> Result<dds::Image, Error> {
    if data.len() >= 4 && &data[1..4] == b"DS " && !is_dds(data) {
        // A broken magic: D3DX's other readers refuse it too (measured, 2026-10-22).
        return Err(Error::D3dx(crate::D3DXERR_INVALIDDATA));
    }
    if !is_dds(data) {
        if data.len() < 4 {
            return Err(Error::D3dx(crate::D3DXERR_INVALIDDATA));
        }
        return Err(Error::NotProvided(
            Untranslated::ImageFile,
            "an image file that is not DDS (BMP, TGA, PNG, JPG, ...)".into(),
        ));
    }
    dds::parse(data).ok_or(Error::D3dx(crate::D3DXERR_INVALIDDATA))
}

/// `D3DXGetImageInfoFromFileInMemory`.
pub fn get_image_info_from_file_in_memory(data: &[u8]) -> Result<ImageInfo, Error> {
    if data.is_empty() {
        return Err(Error::D3dx(crate::D3DERR_INVALIDCALL));
    }
    let img = read(data)?;
    Ok(ImageInfo {
        width: img.width,
        height: img.height,
        depth: img.depth,
        mip_levels: img.mip_levels,
        format: img.format,
        resource_type: img.resource_type,
        image_file_format: IFF_DDS,
    })
}

/// `D3DXCreateTextureFromFileInMemory(device, data, size, &texture)`, called on a thread at
/// `precision` (the texture's bytes depend on it: [`Precision`]).
pub fn create_texture_from_file_in_memory(
    data: &[u8],
    caps: &Caps,
    precision: Precision,
) -> Result<Texture, Error> {
    create(data, caps, RTYPE_TEXTURE, precision)
}

/// `D3DXCreateCubeTextureFromFileInMemory(device, data, size, &texture)`, called on a thread
/// at `precision`.
pub fn create_cube_texture_from_file_in_memory(
    data: &[u8],
    caps: &Caps,
    precision: Precision,
) -> Result<Texture, Error> {
    create(data, caps, RTYPE_CUBE, precision)
}

/// A side as `D3DX_DEFAULT` makes it (0x61025e): 0 and 1 give 1, a side up to 2^31 the next
/// power of two at or above it, a larger one (negative as D3DX's signed test reads it) itself.
fn default_side(v: u32) -> u32 {
    if (v as i32) < 0 {
        v
    } else {
        v.max(1).next_power_of_two()
    }
}

/// The number of bits in `v` (the mip chain's length for a side).
fn bit_count(mut v: u32) -> u32 {
    let mut n = 0;
    while v != 0 {
        n += 1;
        v >>= 1;
    }
    n
}

fn create(data: &[u8], caps: &Caps, rtype: u32, precision: Precision) -> Result<Texture, Error> {
    if data.is_empty() {
        return Err(Error::D3dx(crate::D3DERR_INVALIDCALL));
    }
    let img = read(data)?;
    if img.resource_type == dds::RTYPE_VOLUME {
        return Err(Error::NotProvided(
            Untranslated::Volume,
            "a volume DDS file".into(),
        ));
    }
    // 0x61025e counts the image's faces only for a cube texture; one that is not a whole cube
    // is E_FAIL. A texture takes the first face of a cube file.
    let faces = if rtype == RTYPE_CUBE {
        if img.faces != 6 {
            return Err(Error::D3dx(0x8000_4005));
        }
        6
    } else {
        1
    };
    // The format (0x61025e): R8G8B8 becomes X8R8G8B8 outside D3DPOOL_SCRATCH; then the one the
    // device takes (0x60d069), the file's or the nearest.
    let file_format = img.format;
    let not_provided = |why: String| Err(Error::NotProvided(Untranslated::PixelFormat, why));
    if !filter::reads(file_format) {
        return not_provided(format!("DDS pixel format {file_format:#x}"));
    }
    let wanted = if file_format == format::R8G8B8 {
        format::X8R8G8B8
    } else {
        file_format
    };
    let takes = if rtype == RTYPE_CUBE {
        &caps.cube_formats
    } else {
        &caps.texture_formats
    };
    let tex_format = crate::device::choose(wanted, |f| takes.contains(&f));
    if tex_format == 0 {
        return Err(Error::D3dx(crate::D3DERR_NOTAVAILABLE));
    }
    // A texture format this crate does not make (D3DX's pick on a device that takes other formats
    // than the reference one): left to the caller.
    if !filter::makes(tex_format) {
        return not_provided(format!(
            "texture format {tex_format:#x} (D3DX's pick for DDS pixel format {file_format:#x})"
        ));
    }
    // A level whose size the loader's 32-bit arithmetic wrapped into one the file holds: the
    // 32-bit DLL reads past the file and crashes (measured); refused as a file shorter than its
    // levels instead.
    if img.past_file {
        return Err(Error::D3dx(crate::D3DXERR_INVALIDDATA));
    }
    // A side of 0 (levels of no bytes): the image info takes it, the create calls answer E_FAIL
    // (measured, `data/headers_32bit.tsv`).
    if img.width == 0 || img.height == 0 {
        return Err(Error::D3dx(0x8000_4005));
    }
    // The size: D3DX_DEFAULT rounds the file's up to powers of two; then the requirements.
    let mut w = default_side(img.width);
    let mut h = default_side(img.height);
    if rtype == RTYPE_CUBE {
        let s = w.max(h);
        w = s;
        h = s;
    }
    w = w.min(caps.max_width);
    h = h.min(caps.max_height);
    if caps.pow2 {
        // Within the caps' limits, which are powers of two (16384 and below).
        w = default_side(w);
        h = default_side(h);
    }
    if format::is_dxt(tex_format) {
        w = w.saturating_add(3) & !3;
        h = h.saturating_add(3) & !3;
    }
    let levels = bit_count(w).max(bit_count(h));
    // A file level of another size than the texture's (a side that is not a power of two, a DXT
    // side that is not a multiple of 4) is resampled with D3DX's triangle filter; its edges wrap
    // round, except in a cube map, for which D3DX adds the mirror flags (0x61025e).
    let wrap = rtype != RTYPE_CUBE;
    let mut surfaces = Vec::with_capacity((levels * faces) as usize);
    for face in 0..faces {
        let first = surfaces.len();
        for l in 0..levels {
            if l < img.stored {
                let lv = &img.levels[(face * img.stored + l) as usize];
                let size = format::level_size(file_format, lv.width, lv.height);
                let bytes = &data[lv.offset..lv.offset + size];
                let (tw, th) = ((w >> l).max(1), (h >> l).max(1));
                let (fw, fh) = (lv.width, lv.height);
                let (ff, tf) = (file_format, tex_format);
                let bytes = match (precision, (tw, th) == (fw, fh)) {
                    (Precision::Single, true) => {
                        filter::load_same_size::<P24>(ff, tf, fw, fh, bytes)
                    }
                    (Precision::Double, true) => {
                        filter::load_same_size::<P53>(ff, tf, fw, fh, bytes)
                    }
                    (Precision::Single, false) => {
                        triangle::triangle::<P24>(ff, fw, fh, bytes, tf, tw, th, wrap)
                    }
                    (Precision::Double, false) => {
                        triangle::triangle::<P53>(ff, fw, fh, bytes, tf, tw, th, wrap)
                    }
                };
                surfaces.push(Level {
                    format: tex_format,
                    width: tw,
                    height: th,
                    bytes,
                });
            } else {
                let prev = &surfaces[first + l as usize - 1];
                let next = match precision {
                    Precision::Single => filter::box_next::<P24>(prev),
                    Precision::Double => filter::box_next::<P53>(prev),
                };
                surfaces.push(next);
            }
        }
    }
    Ok(Texture {
        levels,
        faces,
        surfaces,
    })
}
