//! The platform's Direct3D 9 device (ADR-0007): `IDirect3D9::CreateDevice` and the
//! `IDirect3DDevice9` it makes, with its surfaces, following the documented Direct3D 9
//! semantics (the DirectX 9.0c SDK documentation of each method). Answers that depend on the
//! player's hardware come from the machine table (`super::d3d9`, `scripts/d3d9-capture`).
//!
//! As in `super::d3d9`, each object lives in engine memory with its vtable pointer at `+0`, each
//! provided slot a platform function at a code address from [`DEVICE_CODE`] (the device) or
//! [`SURFACE_CODE`] (surfaces); a slot not provided is left unregistered, so a call to it stops
//! the run as an open function with its address (slot = (address - the code base) / 4, named
//! in [`DEVICE_SLOTS`] and [`SURFACE_SLOTS`]). Nothing is drawn yet: the viewer still draws.
//!
//! Provided so far, as the game's start-up reaches them: windowed devices of the HAL type with
//! hardware or mixed vertex processing, without multisampling (anything else stops the run as
//! not provided); the device's reference counting, `GetDirect3D`, `GetDeviceCaps`,
//! `GetDisplayMode`, `GetCreationParameters`, `GetBackBuffer`, the render target and
//! depth-stencil slots, the viewport and `CreateTexture`; a surface's reference counting,
//! `GetDevice`, `GetType`, `GetContainer`, `GetDesc` and `LockRect`/`UnlockRect`; a texture's
//! reference counting, priority, LOD, levels, their descriptions, surfaces and locks.
//! Measured with the retail runtime (`scripts/d3d9-capture device fixed`): `SetMaterial`, `SetLight`,
//! `LightEnable`, `SetSoftwareVertexProcessing`, `UpdateTexture` and `StretchRect`.

use super::d3d9::{CODE_BASE, D3DERR_INVALIDCALL, D3D_OK, E_NOINTERFACE};
use crate::{Engine, Ptr};

/// Where the device's slots sit (after `IDirect3D9`'s).
pub const DEVICE_CODE: u32 = CODE_BASE + 0x0200;
/// Where a surface's slots sit.
pub const SURFACE_CODE: u32 = CODE_BASE + 0x0400;
/// Where a texture's slots sit.
pub const TEXTURE_CODE: u32 = CODE_BASE + 0x0500;
/// `IDirect3DCubeTexture9`'s slots (the texture's names, with `GetCubeMapSurface` at 18).
pub const CUBE_CODE: u32 = CODE_BASE + 0x0900;

/// `D3DERR_NOTFOUND` (d3d9.h).
pub const D3DERR_NOTFOUND: u32 = 0x8876_0866;

/// `IDirect3DDevice9`'s slots (d3d9.h order).
pub const DEVICE_SLOTS: [&str; 119] = [
    "QueryInterface",
    "AddRef",
    "Release",
    "TestCooperativeLevel",
    "GetAvailableTextureMem",
    "EvictManagedResources",
    "GetDirect3D",
    "GetDeviceCaps",
    "GetDisplayMode",
    "GetCreationParameters",
    "SetCursorProperties",
    "SetCursorPosition",
    "ShowCursor",
    "CreateAdditionalSwapChain",
    "GetSwapChain",
    "GetNumberOfSwapChains",
    "Reset",
    "Present",
    "GetBackBuffer",
    "GetRasterStatus",
    "SetDialogBoxMode",
    "SetGammaRamp",
    "GetGammaRamp",
    "CreateTexture",
    "CreateVolumeTexture",
    "CreateCubeTexture",
    "CreateVertexBuffer",
    "CreateIndexBuffer",
    "CreateRenderTarget",
    "CreateDepthStencilSurface",
    "UpdateSurface",
    "UpdateTexture",
    "GetRenderTargetData",
    "GetFrontBufferData",
    "StretchRect",
    "ColorFill",
    "CreateOffscreenPlainSurface",
    "SetRenderTarget",
    "GetRenderTarget",
    "SetDepthStencilSurface",
    "GetDepthStencilSurface",
    "BeginScene",
    "EndScene",
    "Clear",
    "SetTransform",
    "GetTransform",
    "MultiplyTransform",
    "SetViewport",
    "GetViewport",
    "SetMaterial",
    "GetMaterial",
    "SetLight",
    "GetLight",
    "LightEnable",
    "GetLightEnable",
    "SetClipPlane",
    "GetClipPlane",
    "SetRenderState",
    "GetRenderState",
    "CreateStateBlock",
    "BeginStateBlock",
    "EndStateBlock",
    "SetClipStatus",
    "GetClipStatus",
    "GetTexture",
    "SetTexture",
    "GetTextureStageState",
    "SetTextureStageState",
    "GetSamplerState",
    "SetSamplerState",
    "ValidateDevice",
    "SetPaletteEntries",
    "GetPaletteEntries",
    "SetCurrentTexturePalette",
    "GetCurrentTexturePalette",
    "SetScissorRect",
    "GetScissorRect",
    "SetSoftwareVertexProcessing",
    "GetSoftwareVertexProcessing",
    "SetNPatchMode",
    "GetNPatchMode",
    "DrawPrimitive",
    "DrawIndexedPrimitive",
    "DrawPrimitiveUP",
    "DrawIndexedPrimitiveUP",
    "ProcessVertices",
    "CreateVertexDeclaration",
    "SetVertexDeclaration",
    "GetVertexDeclaration",
    "SetFVF",
    "GetFVF",
    "CreateVertexShader",
    "SetVertexShader",
    "GetVertexShader",
    "SetVertexShaderConstantF",
    "GetVertexShaderConstantF",
    "SetVertexShaderConstantI",
    "GetVertexShaderConstantI",
    "SetVertexShaderConstantB",
    "GetVertexShaderConstantB",
    "SetStreamSource",
    "GetStreamSource",
    "SetStreamSourceFreq",
    "GetStreamSourceFreq",
    "SetIndices",
    "GetIndices",
    "CreatePixelShader",
    "SetPixelShader",
    "GetPixelShader",
    "SetPixelShaderConstantF",
    "GetPixelShaderConstantF",
    "SetPixelShaderConstantI",
    "GetPixelShaderConstantI",
    "SetPixelShaderConstantB",
    "GetPixelShaderConstantB",
    "DrawRectPatch",
    "DrawTriPatch",
    "DeletePatch",
    "CreateQuery",
];

/// `IDirect3DSurface9`'s slots (d3d9.h order: `IUnknown`, `IDirect3DResource9`, its own).
pub const SURFACE_SLOTS: [&str; 17] = [
    "QueryInterface",
    "AddRef",
    "Release",
    "GetDevice",
    "SetPrivateData",
    "GetPrivateData",
    "FreePrivateData",
    "SetPriority",
    "GetPriority",
    "PreLoad",
    "GetType",
    "GetContainer",
    "GetDesc",
    "LockRect",
    "UnlockRect",
    "GetDC",
    "ReleaseDC",
];

/// `IDirect3DTexture9`'s slots (d3d9.h order: `IUnknown`, `IDirect3DResource9`,
/// `IDirect3DBaseTexture9`, its own).
pub const TEXTURE_SLOTS: [&str; 22] = [
    "QueryInterface",
    "AddRef",
    "Release",
    "GetDevice",
    "SetPrivateData",
    "GetPrivateData",
    "FreePrivateData",
    "SetPriority",
    "GetPriority",
    "PreLoad",
    "GetType",
    "SetLOD",
    "GetLOD",
    "GetLevelCount",
    "SetAutoGenFilterType",
    "GetAutoGenFilterType",
    "GenerateMipSubLevels",
    "GetLevelDesc",
    "GetSurfaceLevel",
    "LockRect",
    "UnlockRect",
    "AddDirtyRect",
];

/// `D3DDEVTYPE_HAL`, the vertex-processing behaviour flags (`D3DCREATE_SOFTWARE_`,
/// `HARDWARE_`, `MIXED_VERTEXPROCESSING`), `D3DFMT_UNKNOWN`, `D3DRTYPE_SURFACE`, the surface
/// usages `D3DUSAGE_RENDERTARGET` and `D3DUSAGE_DEPTHSTENCIL`, `D3DPRESENT_BACK_BUFFERS_MAX`.
const D3DDEVTYPE_HAL: u32 = 1;
const SOFTWARE_VERTEXPROCESSING: u32 = 0x20;
const HARDWARE_VERTEXPROCESSING: u32 = 0x40;
const MIXED_VERTEXPROCESSING: u32 = 0x80;
const D3DFMT_UNKNOWN: u32 = 0;
const D3DRTYPE_SURFACE: u32 = 1;
const D3DUSAGE_RENDERTARGET: u32 = 1;
const D3DUSAGE_DEPTHSTENCIL: u32 = 2;
const BACK_BUFFERS_MAX: u32 = 3;
/// `D3DPRESENT_PARAMETERS` (0x38 bytes) fields.
const PP_WIDTH: u32 = 0x0;
const PP_HEIGHT: u32 = 0x4;
const PP_FORMAT: u32 = 0x8;
const PP_COUNT: u32 = 0xc;
const PP_MULTISAMPLE: u32 = 0x10;
const PP_QUALITY: u32 = 0x14;
const PP_WINDOW: u32 = 0x1c;
const PP_WINDOWED: u32 = 0x20;
const PP_AUTO_DEPTH: u32 = 0x24;
const PP_DEPTH_FORMAT: u32 = 0x28;
const PP_REFRESH: u32 = 0x30;
const PRESENT_SIZE: u32 = 0x38;
/// `D3DCAPS9::NumSimultaneousRTs`.
const CAPS_SIMULTANEOUS_RTS: usize = 0xf0;
/// The render target slots a device keeps (`D3D_MAX_SIMULTANEOUS_RENDERTARGETS`).
const RENDER_TARGETS: usize = 4;

/// A GUID's 16 bytes as they lie in memory (`Data1`..`Data3` little-endian, then `Data4`).
pub(super) const fn guid(d1: u32, d2: u16, d3: u16, d4: [u8; 8]) -> [u8; 16] {
    let a = d1.to_le_bytes();
    let b = d2.to_le_bytes();
    let c = d3.to_le_bytes();
    [
        a[0], a[1], a[2], a[3], b[0], b[1], c[0], c[1], d4[0], d4[1], d4[2], d4[3], d4[4], d4[5],
        d4[6], d4[7],
    ]
}

/// The interface identifiers (unknwn.h, d3d9.h).
pub(super) const IID_IUNKNOWN: [u8; 16] = guid(0, 0, 0, [0xc0, 0, 0, 0, 0, 0, 0, 0x46]);
const IID_DEVICE9: [u8; 16] = guid(
    0xd022_3b96,
    0xbf7a,
    0x43fd,
    [0x92, 0xbd, 0xa4, 0x3b, 0x0d, 0x82, 0xb9, 0xeb],
);
const IID_RESOURCE9: [u8; 16] = guid(
    0x05ee_c05d,
    0x8f7d,
    0x4362,
    [0xb9, 0x99, 0xd1, 0xba, 0xf3, 0x57, 0xc7, 0x04],
);
const IID_BASETEXTURE9: [u8; 16] = guid(
    0x580c_a87e,
    0x1d3c,
    0x4d54,
    [0x99, 0x1d, 0xb7, 0xd3, 0xe3, 0xc2, 0x98, 0xce],
);
const IID_TEXTURE9: [u8; 16] = guid(
    0x85c3_1227,
    0x3de5,
    0x4f00,
    [0x9b, 0x3a, 0xf1, 0x1a, 0xc3, 0x8c, 0x18, 0xb5],
);
const IID_SURFACE9: [u8; 16] = guid(
    0x0cfb_af3a,
    0x9ff6,
    0x429a,
    [0x99, 0xb3, 0xa2, 0x79, 0x6a, 0xf8, 0xb8, 0x9b],
);

/// A device of this layer.
#[derive(Debug, Clone)]
struct Device {
    references: u32,
    direct3d: u32,
    /// `D3DDEVICE_CREATION_PARAMETERS`: adapter, device type, focus window, behaviour flags.
    creation: [u32; 4],
    /// The present parameters as the device took them (after filling in the defaults).
    present: Vec<u8>,
    back_buffers: Vec<u32>,
    render_targets: [u32; RENDER_TARGETS],
    depth_stencil: u32,
    /// `D3DVIEWPORT9`: X, Y, Width, Height, then MinZ and MaxZ as `float` bits.
    viewport: [u32; 6],
    /// The states the program set: render states by type, texture stage states and sampler
    /// states by (stage or sampler, type), the textures by sampler (held, counted).
    /// The last gamma ramp `SetGammaRamp` was given (three 256-entry WORD tables, 0x600 bytes).
    gamma_ramp: Option<Vec<u16>>,
    render_states: States<RENDER_STATES>,
    stage_states: States<STAGE_STATES>,
    sampler_states: States<SAMPLER_STATES>,
    textures: engine_core::AddrMap<u32, u32>,
    /// Transforms by `D3DTRANSFORMSTATETYPE`, the scissor rectangle, user clip planes, and
    /// whether a scene is open.
    transforms: engine_core::AddrMap<u32, [u32; 16]>,
    scissor: [u32; 4],
    clip_planes: engine_core::AddrMap<u32, [u32; 4]>,
    in_scene: bool,
    /// The fixed-function material (`D3DMATERIAL9`, 17 words), the lights by index
    /// (`D3DLIGHT9`, 26 words), the enabled ones, and the software vertex processing switch.
    material: Option<[u32; 17]>,
    lights: engine_core::AddrMap<u32, [u32; 26]>,
    enabled_lights: std::collections::BTreeSet<u32>,
    software_vertex_processing: bool,
    /// After a `Reset` that failed: the device needs a successful `Reset` (`TestCooperativeLevel`
    /// answers `D3DERR_DEVICENOTRESET`; scenes and `Present` are refused).
    lost: bool,
}

/// The render states kept (numbers 0 to 255), the texture stage states (8 stages of types 1 to
/// 32) and the sampler states (21 samplers, [`sampler_index`], of types 1 to 13).
const RENDER_STATES: usize = 256;
const STAGE_STATES: usize = 8 * 32;
const SAMPLER_STATES: usize = 21 * 13;

/// States by number: each value the program set, `None` for one never set. A table rather than
/// a map: these are set thousands of times a frame.
#[derive(Debug, Clone)]
struct States<const N: usize>(Box<[Option<u32>; N]>);

impl<const N: usize> Default for States<N> {
    fn default() -> Self {
        States(Box::new([None; N]))
    }
}

impl<const N: usize> States<N> {
    fn get(&self, index: Option<usize>) -> Option<u32> {
        index.and_then(|i| self.0.get(i).copied().flatten())
    }
    fn set(&mut self, index: usize, value: u32) {
        self.0[index] = Some(value);
    }
    fn clear(&mut self) {
        self.0.fill(None);
    }
    #[cfg(test)]
    fn is_empty(&self) -> bool {
        self.0.iter().all(Option::is_none)
    }
}

/// The place of texture stage state `kind` of `stage` (0 to 7) in [`Device::stage_states`];
/// `None` for a type past 1 to 32.
fn stage_index(stage: u32, kind: u32) -> Option<usize> {
    (1..=32)
        .contains(&kind)
        .then(|| stage as usize * 32 + kind as usize - 1)
}

/// The place of sampler state `kind` of `sampler` (one [`is_sampler`] names) in
/// [`Device::sampler_states`]: samplers 0 to 15 then 256 to 260; `None` for a type past 1 to 13.
fn sampler_index(sampler: u32, kind: u32) -> Option<usize> {
    let s = if sampler < 16 { sampler } else { sampler - 240 };
    (1..=13)
        .contains(&kind)
        .then(|| s as usize * 13 + kind as usize - 1)
}

/// A surface of this layer.
#[derive(Debug, Clone)]
struct Surface {
    /// References the program holds (`AddRef`/`Release`).
    references: u32,
    device: u32,
    /// Whether the device owns it (the back buffers and the automatic depth-stencil surface):
    /// it stays while the device lives, whatever the program's count.
    implicit: bool,
    /// `D3DSURFACE_DESC`: Format, Type, Usage, Pool, MultiSampleType, MultiSampleQuality,
    /// Width, Height.
    desc: [u32; 8],
    /// The texture whose level this is (0 for none): its references are the texture's.
    container: u32,
    /// The surface's memory for `LockRect` in the game's address space (0 for none) and its
    /// pitch (the row's bytes, rows packed, for every lockable surface).
    bits: u32,
    pitch: u32,
    locked: bool,
    /// A lockable texture level's pixels kept outside the game's address space (`bits` is then
    /// 0): see [`Pixels`].
    pixels: Option<Pixels>,
    /// While such a level is locked: the block of the game's memory its lock handed out.
    staging: u32,
}

/// Where a lockable texture level keeps its pixels between locks (ADR-0015, the bulk buffers
/// first): not in the game's address space, which holds them only while the game has the level
/// locked. `LockRect` copies them into a block of the game's memory (the staging block, rows
/// packed, the pitch unchanged) and answers its address; `UnlockRect` copies the block back
/// (and to the real level, as before); the block is freed at the next `Present`, so code that
/// still reads through the lock's address just after the unlock reads the level's bytes, not
/// freed memory (at most [`KEPT_STAGING_MAX`] bytes wait, the oldest freed first). What the game
/// sees through its locks is the
/// same as with the pixels in its memory: the pitch, the bytes (zero until written), the
/// rectangle's offset; only the block's address changes from lock to lock, as Direct3D leaves
/// it to the runtime. `NV_D3D9_TEXTURES=check` also reports a staging block written after its
/// `UnlockRect` ([`Store`]).
#[derive(Debug, Clone)]
enum Pixels {
    /// In host memory; empty while every byte is 0.
    Host(Vec<u8>),
    /// In the real level behind this one (`NV_D3D9=real`; a managed or system-memory texture,
    /// whose real levels Direct3D keeps in system memory and hands back as written); `written`
    /// false while every byte is 0 (nothing unlocked yet).
    #[cfg_attr(not(windows), allow(dead_code))]
    Real { written: bool },
}

/// Where lockable texture levels keep their pixels (`NV_D3D9_TEXTURES`), and static vertex and
/// index buffers their bytes (`NV_D3D9_BUFFERS`, `super::d3d9resource`): outside the game's
/// address space ([`Pixels`], the default), or outside with checks (`check`, a diagnostic): at each `UnlockRect` (a
/// buffer's last `Unlock`) the level is read back from where it keeps its bytes (host memory or
/// the real level) and compared with the staging block, and when the block is freed (at the
/// next `Present`, or earlier past [`KEPT_STAGING_MAX`]) it is compared with what it held at
/// the unlock (a difference is game code writing through a lock's address after the unlock,
/// which nothing would carry to the texture); a summary is printed every 2,000
/// unlocks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Store {
    Host,
    Check,
}

impl Store {
    fn from_env(setting: &str) -> Store {
        match std::env::var(setting).as_deref() {
            Ok("check") => Store::Check,
            _ => Store::Host,
        }
    }
}

/// The devices and their resources (`d3d9::DeviceState`).
#[derive(Debug, Default)]
pub struct Objects {
    device_vtable: u32,
    surface_vtable: u32,
    texture_vtable: u32,
    cube_vtable: u32,
    devices: engine_core::AddrMap<u32, Device>,
    surfaces: engine_core::AddrMap<u32, Surface>,
    textures: engine_core::AddrMap<u32, Texture>,
    /// The buffers, shaders and declarations (`super::d3d9resource`).
    pub(super) resources: super::d3d9resource::Resources,
    /// The queries (`super::d3d9query`).
    pub(super) queries: super::d3d9query::Queries,
    /// Where texture levels keep their pixels: read from `NV_D3D9_TEXTURES` at the first texture.
    texture_store: Option<Store>,
    /// Where static buffers keep their bytes: read from `NV_D3D9_BUFFERS` at the first buffer.
    buffer_store: Option<Store>,
    /// The staging blocks unlocked since the last `Present`, still in the game's memory
    /// ([`unstage`], [`wait_for_present`]): block, level or buffer, size, with `check` the bytes
    /// each held at its unlock, and whether it is a buffer's; and their bytes together.
    unlocked_staging: std::collections::VecDeque<(u32, u32, u32, Vec<u8>, bool)>,
    unlocked_staging_bytes: u64,
    /// `check`: the levels and buffers already reported.
    written_after_unlock: engine_core::AddrSet<u32>,
    /// `NV_D3D9_TEXTURES=check`: the unlocks whose level read back the same as its staging
    /// block, and those that did not; the bytes compared.
    round_trips: [u64; 2],
    round_trip_bytes: u64,
    /// The open cut of step 2's two producers (`super::d3d9draw`), and the drawing side's copies
    /// of the devices' state while it is open.
    pub(super) draw: super::d3d9draw::DrawSide,
    draw_devices: engine_core::AddrMap<u32, Device>,
    /// Whether states set again to the value they hold still reach the backend ([`skip_repeats`]):
    /// `NV_D3D9_REPEATS=keep`, read at the first such set; tests set it ([`keep_repeats`]).
    keep_repeats: Option<bool>,
}

/// Where texture levels keep their pixels in this run ([`Store`]).
fn texture_store(e: &mut Engine) -> Store {
    *device!(e)
        .device_objects
        .texture_store
        .get_or_insert_with(|| Store::from_env("NV_D3D9_TEXTURES"))
}

/// Where static vertex and index buffers keep their bytes in this run ([`Store`]).
pub(super) fn buffer_store(e: &mut Engine) -> Store {
    *device!(e)
        .device_objects
        .buffer_store
        .get_or_insert_with(|| Store::from_env("NV_D3D9_BUFFERS"))
}

/// Sets where static buffers keep their bytes (tests), before the first buffer.
#[cfg(test)]
pub(crate) fn set_buffer_store(e: &mut Engine, store: Store) {
    device!(e).device_objects.buffer_store = Some(store);
}

/// Sets where texture levels keep their pixels (tests), before the first texture.
#[cfg(test)]
pub(crate) fn set_texture_store(e: &mut Engine, store: Store) {
    device!(e).device_objects.texture_store = Some(store);
}

fn vtable(e: &mut Engine, base: u32, slots: usize) -> u32 {
    let table = e.mem.alloc(4 * slots as u32);
    for i in 0..slots as u32 {
        e.mem.set_u32(table + 4 * i, base + 4 * i);
    }
    table
}

/// A word of a call's arguments, as the real side (`super::d3d9real`) takes it.
pub(super) trait Word: Copy {
    fn word(self) -> u32;
}
impl Word for u32 {
    fn word(self) -> u32 {
        self
    }
}
impl Word for Ptr {
    fn word(self) -> u32 {
        self.addr()
    }
}

/// The slots that change what is drawn: the model's answer, and with `NV_D3D9=real` the same
/// call made on the real device (ADR-0008, `super::d3d9real`), which gets the model's answer
/// to compare.
macro_rules! forwarded {
    ($($vis:vis fn $name:ident = $model:ident($($a:ident: $t:ty),*) => $real:ident;)*) => {$(
        $vis fn $name(e: &mut Engine, this: Ptr, $($a: $t),*) -> u32 {
            let model = $model(e, this, $($a),*);
            // A call the model refuses changes nothing, so the real device does not get it.
            if model == D3D_OK {
                super::d3d9real::$real(e, this.addr(), $(Word::word($a),)* model);
            }
            model
        }
    )*};
}
pub(super) use forwarded;

/// Whether a state set to the value it already has is left out of what the backend gets
/// ([`fwd_set_sampler_state`], `super::d3d9resource`'s constants): on, unless `NV_D3D9_REPEATS=keep`
/// (a check: the runs that show the stream's draws see the same state either way). While a drawing
/// cut is open (`super::d3d9draw`) only the drawing side leaves them out: its copy starts as main's
/// state at the cut and takes only its own sets, and its commands go into the cut's slot right after
/// the commands before the cut, so its copy is the stream's state where its next command lands. Main's
/// model is not: the drawing side's commands come between, so main sends every set until the join.
/// Direct3D 9 makes such a set a no-op: the draws see the same state. (A gate of 2026-11-21 with the
/// drawing thread on: both sides sending every repeat made the stream about 9.5 million constant sets
/// and 4.6 million sampler states longer on the short route, the only commands whose counts differed.)
#[inline]
pub(super) fn skip_repeats(e: &mut Engine) -> bool {
    skip_repeats_in(&mut device!(e).device_objects)
}

/// [`skip_repeats`] for code that holds the devices' objects already (`super::d3d9resource`'s
/// constant sets in one pass).
#[inline]
pub(super) fn skip_repeats_in(objects: &mut Objects) -> bool {
    static KEEP: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    let keep = *objects.keep_repeats.get_or_insert_with(|| {
        *KEEP.get_or_init(|| std::env::var("NV_D3D9_REPEATS").is_ok_and(|v| v == "keep"))
    });
    !keep
        && (!super::d3d9draw::cut_open_in(&objects.draw)
            || super::d3d9draw::drawing_side(&objects.draw))
}

/// Tests: whether states set again to the value they hold still reach the backend on `e`
/// ([`skip_repeats`]), in place of `NV_D3D9_REPEATS`.
#[cfg(test)]
pub(crate) fn keep_repeats(e: &mut Engine, keep: bool) {
    device!(e).device_objects.keep_repeats = Some(keep);
}

/// `SetSamplerState` forwarded as the others ([`forwarded`]), except that a value the sampler
/// already holds is not passed on ([`skip_repeats`]): in a crowded street about 1,900 of the
/// frame's 2,200 sampler sets (measured 2026-11-20).
fn fwd_set_sampler_state(e: &mut Engine, this: Ptr, sampler: u32, kind: u32, value: u32) -> u32 {
    if let Some(model) = state_one_pass(e, this, StateSet::Sampler, sampler, kind, value) {
        return model;
    }
    let repeat = skip_repeats(e)
        && is_sampler(sampler)
        && device(e, this)
            .sampler_states
            .get(sampler_index(sampler, kind))
            == Some(value);
    let model = set_sampler_state(e, this, sampler, kind, value);
    if model == D3D_OK && !repeat {
        super::d3d9real::set_sampler_state(e, this.addr(), sampler, kind, value, model);
    }
    model
}

/// `SetRenderState` forwarded as the others ([`forwarded`]), in one pass when it can
/// ([`state_one_pass`]).
fn fwd_set_render_state(e: &mut Engine, this: Ptr, state: u32, value: u32) -> u32 {
    if let Some(model) = state_one_pass(e, this, StateSet::Render, state, 0, value) {
        return model;
    }
    let model = set_render_state(e, this, state, value);
    if model == D3D_OK {
        super::d3d9real::set_render_state(e, this.addr(), state, value, model);
    }
    model
}

/// `SetTextureStageState` forwarded as the others ([`forwarded`]), in one pass when it can
/// ([`state_one_pass`]).
fn fwd_set_texture_stage_state(
    e: &mut Engine,
    this: Ptr,
    stage: u32,
    kind: u32,
    value: u32,
) -> u32 {
    if let Some(model) = state_one_pass(e, this, StateSet::Stage, stage, kind, value) {
        return model;
    }
    let model = set_texture_stage_state(e, this, stage, kind, value);
    if model == D3D_OK {
        super::d3d9real::set_texture_stage_state(e, this.addr(), stage, kind, value, model);
    }
    model
}

/// `SetTexture` forwarded as the others ([`forwarded`]), in one pass when it can
/// ([`texture_one_pass`]).
fn fwd_set_texture(e: &mut Engine, this: Ptr, sampler: u32, texture: u32) -> u32 {
    if let Some(model) = texture_one_pass(e, this, sampler, texture) {
        return model;
    }
    let model = set_texture(e, this, sampler, texture);
    if model == D3D_OK {
        super::d3d9real::set_texture(e, this.addr(), sampler, texture, model);
    }
    model
}

/// A forwarded `SetTexture` ([`set_texture`], then `super::d3d9real::set_texture`) in one pass over
/// the device's state: the device and the texture found once, the sampler's texture replaced and
/// the counts kept there, and the command sent with no second look-up of the devices or the
/// backend's state. The same model state, answer and commands as the two steps: when the texture
/// let go of ends (neither the program nor a device holds it any more), its end comes first and
/// the command after it, as in the two steps. `None` (nothing done) when `NV_D3D9_ONE_PASS=0` or
/// for a texture not of this layer (the two steps then stop the run as before).
#[inline]
fn texture_one_pass(e: &mut Engine, this: Ptr, sampler: u32, texture: u32) -> Option<u32> {
    let held = crate::device_held(e);
    let crate::d3d9::DeviceState {
        device_objects: objects,
        real,
        ..
    } = crate::device_mut(&mut e.device, held);
    if !super::d3d9resource::one_pass_in(&mut objects.resources) {
        return None;
    }
    if !is_sampler(sampler) {
        return Some(D3DERR_INVALIDCALL);
    }
    let device = this.addr();
    let cmd = d3d9_stream::Cmd::SetTexture {
        device,
        sampler,
        texture,
    };
    super::d3d9draw::main_set(&mut objects.draw, super::d3d9draw::Key::Texture(sampler));
    let Objects {
        devices,
        draw_devices,
        textures,
        draw,
        ..
    } = objects;
    let devices = if super::d3d9draw::drawing_side(draw) && draw_devices.contains_key(&device) {
        draw_devices
    } else {
        devices
    };
    let d = devices
        .get_mut(&device)
        .expect("an IDirect3DDevice9 of this layer");
    // The texture the sampler holds already: counting it once more and letting go of it once
    // changes nothing (as [`set_texture`]).
    if texture != 0 && d.textures.get(&sampler) == Some(&texture) {
        super::d3d9real::record_in(real, device, cmd);
        return Some(D3D_OK);
    }
    let old = if texture != 0 {
        textures.get_mut(&texture)?.bound += 1;
        d.textures.insert(sampler, texture)
    } else {
        d.textures.remove(&sampler)
    };
    if let Some(old) = old {
        // As [`texture_unbind`]: the texture ends once neither the program nor a device holds it.
        let t = textures
            .get_mut(&old)
            .expect("an IDirect3DTexture9 of this layer");
        t.bound -= 1;
        if t.bound == 0 && t.references == 0 {
            texture_destroy(e, Ptr::new(old));
            super::d3d9real::set_texture(e, device, sampler, texture, D3D_OK);
            return Some(D3D_OK);
        }
    }
    super::d3d9real::record_in(real, device, cmd);
    Some(D3D_OK)
}

/// [`in_scene`] for code that holds the devices' objects already (`super::d3d9resource`'s draws in
/// one pass).
#[inline]
pub(super) fn in_scene_in(objects: &mut Objects, this: Ptr) -> bool {
    device_in(objects, this).in_scene
}

/// Which of the device's state tables a set in one pass writes ([`state_one_pass`]).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum StateSet {
    Render,
    Stage,
    Sampler,
}

/// A forwarded `SetRenderState(unit, value)`, `SetTextureStageState(unit, kind, value)` or
/// `SetSamplerState(unit, kind, value)` (the model's [`set_render_state`] /
/// [`set_texture_stage_state`] / [`set_sampler_state`], then `super::d3d9real`'s command) in one
/// pass over the device's state: the device found once, the state noted for an open cut, stored
/// and (a sampler state, [`fwd_set_sampler_state`]'s repeat test) compared there, and the command
/// sent with no second look-up of the devices or the backend's state. The same model state, answer
/// and command as the two steps. `None` (nothing done) when `NV_D3D9_ONE_PASS=0` or for a call the
/// model does not provide (the two steps then stop the run as before).
#[inline]
fn state_one_pass(
    e: &mut Engine,
    this: Ptr,
    set: StateSet,
    unit: u32,
    kind: u32,
    value: u32,
) -> Option<u32> {
    use super::d3d9draw::Key;
    use d3d9_stream::Cmd;
    let held = crate::device_held(e);
    let crate::d3d9::DeviceState {
        device_objects: objects,
        real,
        ..
    } = crate::device_mut(&mut e.device, held);
    if !super::d3d9resource::one_pass_in(&mut objects.resources) {
        return None;
    }
    let device = this.addr();
    let (key, index, cmd) = match set {
        StateSet::Render => {
            if unit >= 256 {
                return None;
            }
            let cmd = Cmd::SetRenderState {
                device,
                state: unit,
                value,
            };
            (Key::Render(unit), unit as usize, cmd)
        }
        StateSet::Stage => {
            if unit >= 8 {
                return Some(D3DERR_INVALIDCALL);
            }
            let index = stage_index(unit, kind)?;
            let cmd = Cmd::SetTextureStageState {
                device,
                stage: unit,
                kind,
                value,
            };
            (Key::Stage(index as u32), index, cmd)
        }
        StateSet::Sampler => {
            if !is_sampler(unit) {
                return Some(D3DERR_INVALIDCALL);
            }
            let index = sampler_index(unit, kind)?;
            let cmd = Cmd::SetSamplerState {
                device,
                sampler: unit,
                kind,
                value,
            };
            (Key::Sampler(index as u32), index, cmd)
        }
    };
    // Only sampler states leave repeats out ([`fwd_set_sampler_state`]); read before the device
    // is borrowed, as the two steps read it before the model's set.
    let skip = set == StateSet::Sampler && skip_repeats_in(objects);
    if super::d3d9draw::any_cut() {
        super::d3d9draw::main_set(&mut objects.draw, key);
    }
    let d = device_in(objects, this);
    let table: &mut [Option<u32>] = match set {
        StateSet::Render => &mut d.render_states.0[..],
        StateSet::Stage => &mut d.stage_states.0[..],
        StateSet::Sampler => &mut d.sampler_states.0[..],
    };
    let old = table[index].replace(value);
    if !(skip && old == Some(value)) {
        super::d3d9real::record_in(real, device, cmd);
    }
    Some(D3D_OK)
}

forwarded! {
    fn fwd_set_render_target = set_render_target(index: u32, surface: u32) => set_render_target;
    fn fwd_set_depth_stencil_surface = set_depth_stencil_surface(surface: u32)
        => set_depth_stencil_surface;
    fn fwd_set_viewport = set_viewport(viewport: Ptr) => set_viewport;
    fn fwd_set_transform = set_transform(state: u32, matrix: Ptr) => set_transform;
    fn fwd_set_scissor_rect = set_scissor_rect(rect: Ptr) => set_scissor_rect;
    fn fwd_set_clip_plane = set_clip_plane(index: u32, plane: Ptr) => set_clip_plane;
    fn fwd_set_gamma_ramp = set_gamma_ramp(swap_chain: u32, flags: u32, ramp: Ptr)
        => set_gamma_ramp;
    fn fwd_set_material = set_material(material: Ptr) => set_material;
    fn fwd_set_light = set_light(index: u32, light: Ptr) => set_light;
    fn fwd_light_enable = light_enable(index: u32, enable: u32) => light_enable;
    fn fwd_set_software_vertex_processing = set_software_vertex_processing(software: u32)
        => set_software_vertex_processing;
    fn fwd_update_texture = update_texture(source: u32, destination: u32) => update_texture;
    fn fwd_stretch_rect = stretch_rect(
        source: u32, source_rect: u32, destination: u32, destination_rect: u32, filter: u32
    ) => stretch_rect;
}

/// `CreateTexture`, then (`NV_D3D9=real`) the real texture of the same arguments.
#[allow(clippy::too_many_arguments)]
fn fwd_create_texture(
    e: &mut Engine,
    this: Ptr,
    width: u32,
    height: u32,
    levels: u32,
    usage: u32,
    format: u32,
    pool: u32,
    out: Ptr,
    shared: u32,
) -> u32 {
    let model = create_texture(
        e, this, width, height, levels, usage, format, pool, out, shared,
    );
    if model == D3D_OK {
        let texture = e.mem.u32(out.addr());
        let surfaces = device!(e).device_objects.textures[&texture].levels.clone();
        super::d3d9real::texture_created(
            e,
            this.addr(),
            texture,
            &surfaces,
            width,
            height,
            usage,
            format,
            pool,
        );
        pixels_to_real_levels(e, pool, &surfaces);
    }
    model
}

/// `CreateCubeTexture`, then (`NV_D3D9=real`) the real cube texture of the same arguments.
#[allow(clippy::too_many_arguments)]
fn fwd_create_cube_texture(
    e: &mut Engine,
    this: Ptr,
    edge: u32,
    levels: u32,
    usage: u32,
    format: u32,
    pool: u32,
    out: Ptr,
    shared: u32,
) -> u32 {
    let model = create_cube_texture(e, this, edge, levels, usage, format, pool, out, shared);
    if model == D3D_OK {
        let texture = e.mem.u32(out.addr());
        let surfaces = device!(e).device_objects.textures[&texture].levels.clone();
        super::d3d9real::cube_created(
            e,
            this.addr(),
            texture,
            &surfaces,
            edge,
            usage,
            format,
            pool,
        );
        pixels_to_real_levels(e, pool, &surfaces);
    }
    model
}

/// `CreateRenderTarget` / `CreateDepthStencilSurface`, then (`NV_D3D9=real`) the real surface.
#[allow(clippy::too_many_arguments)]
fn fwd_create_surface(
    e: &mut Engine,
    this: Ptr,
    depth: bool,
    width: u32,
    height: u32,
    format: u32,
    multisample: u32,
    quality: u32,
    flag: u32,
    out: Ptr,
    shared: u32,
) -> u32 {
    let model = if depth {
        create_depth_stencil_surface(
            e,
            this,
            width,
            height,
            format,
            multisample,
            quality,
            flag,
            out,
            shared,
        )
    } else {
        create_render_target(
            e,
            this,
            width,
            height,
            format,
            multisample,
            quality,
            flag,
            out,
            shared,
        )
    };
    if model == D3D_OK {
        let surface = e.mem.u32(out.addr());
        super::d3d9real::surface_created(
            e,
            this.addr(),
            surface,
            depth,
            width,
            height,
            format,
            multisample,
            quality,
            flag,
        );
    }
    model
}

#[allow(clippy::too_many_arguments)]
fn fwd_create_render_target(
    e: &mut Engine,
    this: Ptr,
    width: u32,
    height: u32,
    format: u32,
    multisample: u32,
    quality: u32,
    lockable: u32,
    out: Ptr,
    shared: u32,
) -> u32 {
    fwd_create_surface(
        e,
        this,
        false,
        width,
        height,
        format,
        multisample,
        quality,
        lockable,
        out,
        shared,
    )
}

#[allow(clippy::too_many_arguments)]
fn fwd_create_depth_stencil_surface(
    e: &mut Engine,
    this: Ptr,
    width: u32,
    height: u32,
    format: u32,
    multisample: u32,
    quality: u32,
    discard: u32,
    out: Ptr,
    shared: u32,
) -> u32 {
    fwd_create_surface(
        e,
        this,
        true,
        width,
        height,
        format,
        multisample,
        quality,
        discard,
        out,
        shared,
    )
}

/// The device functions the world draw calls most, for the typed draw code's writer
/// (`engine_core::real_threads::DeviceWriter`, `NV_DRAW_WRITER`): each the function registered at
/// its slot below (the words form) and the same one taking its arguments as they are.
static WRITER: engine_core::real_threads::DeviceWriter = {
    use super::d3d9resource as r;
    engine_core::real_threads::DeviceWriter {
        code: DEVICE_CODE,
        render_state: (
            crate::entry!(0, fwd_set_render_state(Ptr, u32, u32) -> u32).1,
            |e, this, state, value| fwd_set_render_state(e, Ptr::new(this), state, value),
        ),
        stage_state: (
            crate::entry!(0, fwd_set_texture_stage_state(Ptr, u32, u32, u32) -> u32).1,
            |e, this, stage, kind, value| {
                fwd_set_texture_stage_state(e, Ptr::new(this), stage, kind, value)
            },
        ),
        sampler_state: (
            crate::entry!(0, fwd_set_sampler_state(Ptr, u32, u32, u32) -> u32).1,
            |e, this, sampler, kind, value| {
                fwd_set_sampler_state(e, Ptr::new(this), sampler, kind, value)
            },
        ),
        texture: (
            crate::entry!(0, fwd_set_texture(Ptr, u32, u32) -> u32).1,
            |e, this, sampler, texture| fwd_set_texture(e, Ptr::new(this), sampler, texture),
        ),
        stream_source: (
            crate::entry!(0, r::fwd_set_stream_source(Ptr, u32, u32, u32, u32) -> u32).1,
            |e, this, stream, buffer, offset, stride| {
                r::fwd_set_stream_source(e, Ptr::new(this), stream, buffer, offset, stride)
            },
        ),
        draw_primitive: (
            crate::entry!(0, r::fwd_draw_primitive(Ptr, u32, u32, u32) -> u32).1,
            |e, this, kind, start, count| r::fwd_draw_primitive(e, Ptr::new(this), kind, start, count),
        ),
        draw_indexed_primitive: (
            crate::entry!(0, r::fwd_draw_indexed_primitive(Ptr, u32, u32, u32, u32, u32, u32) -> u32).1,
            |e, this, kind, base, min, vertices, start, count| {
                r::fwd_draw_indexed_primitive(
                    e,
                    Ptr::new(this),
                    kind,
                    base,
                    min,
                    vertices,
                    start,
                    count,
                )
            },
        ),
        vertex_constants: (
            crate::entry!(0, r::fwd_set_vertex_constant_f(Ptr, u32, Ptr, u32) -> u32).1,
            |e, this, start, data, count| {
                r::fwd_set_vertex_constant_f(e, Ptr::new(this), start, Ptr::new(data), count)
            },
        ),
        pixel_constants: (
            crate::entry!(0, r::fwd_set_pixel_constant_f(Ptr, u32, Ptr, u32) -> u32).1,
            |e, this, start, data, count| {
                r::fwd_set_pixel_constant_f(e, Ptr::new(this), start, Ptr::new(data), count)
            },
        ),
    }
};

fn register_code(e: &mut Engine) {
    engine_core::real_threads::install_device_writer(&WRITER);
    let device: [(u32, crate::AbiFn); 48] = [
        (16, crate::entry!(0, reset(Ptr, Ptr) -> u32).1),
        (
            32,
            crate::entry!(0, get_render_target_data(Ptr, u32, u32) -> u32).1,
        ),
        (35, crate::entry!(0, color_fill(Ptr, u32, Ptr, u32) -> u32).1),
        (
            36,
            crate::entry!(0, create_offscreen_plain_surface(Ptr, u32, u32, u32, u32, Ptr, u32) -> u32).1,
        ),
        (
            118,
            crate::entry!(0, super::d3d9query::create_query(Ptr, u32, Ptr) -> u32).1,
        ),
        (
            0,
            crate::entry!(0, device_query_interface(Ptr, Ptr, Ptr) -> u32).1,
        ),
        (1, crate::entry!(0, device_add_ref(Ptr) -> u32).1),
        (2, crate::entry!(0, device_release(Ptr) -> u32).1),
        (6, crate::entry!(0, get_direct3d(Ptr, Ptr) -> u32).1),
        (7, crate::entry!(0, get_device_caps(Ptr, Ptr) -> u32).1),
        (
            8,
            crate::entry!(0, get_display_mode(Ptr, u32, Ptr) -> u32).1,
        ),
        (
            9,
            crate::entry!(0, get_creation_parameters(Ptr, Ptr) -> u32).1,
        ),
        (
            18,
            crate::entry!(0, get_back_buffer(Ptr, u32, u32, u32, Ptr) -> u32).1,
        ),
        (
            23,
            crate::entry!(0, fwd_create_texture(Ptr, u32, u32, u32, u32, u32, u32, Ptr, u32) -> u32).1,
        ),
        (
            25,
            crate::entry!(0, fwd_create_cube_texture(Ptr, u32, u32, u32, u32, u32, Ptr, u32) -> u32).1,
        ),
        (
            28,
            crate::entry!(0, fwd_create_render_target(Ptr, u32, u32, u32, u32, u32, u32, Ptr, u32) -> u32).1,
        ),
        (
            29,
            crate::entry!(0, fwd_create_depth_stencil_surface(Ptr, u32, u32, u32, u32, u32, u32, Ptr, u32) -> u32).1,
        ),
        (
            37,
            crate::entry!(0, fwd_set_render_target(Ptr, u32, u32) -> u32).1,
        ),
        (
            38,
            crate::entry!(0, get_render_target(Ptr, u32, Ptr) -> u32).1,
        ),
        (
            39,
            crate::entry!(0, fwd_set_depth_stencil_surface(Ptr, u32) -> u32).1,
        ),
        (
            40,
            crate::entry!(0, get_depth_stencil_surface(Ptr, Ptr) -> u32).1,
        ),
        (47, crate::entry!(0, fwd_set_viewport(Ptr, Ptr) -> u32).1),
        (
            21,
            crate::entry!(0, fwd_set_gamma_ramp(Ptr, u32, u32, Ptr) -> u32).1,
        ),
        (3, crate::entry!(0, test_cooperative_level(Ptr) -> u32).1),
        (
            17,
            crate::entry!(0, present(Ptr, u32, u32, u32, u32) -> u32).1,
        ),
        (41, crate::entry!(0, begin_scene(Ptr) -> u32).1),
        (42, crate::entry!(0, end_scene(Ptr) -> u32).1),
        (
            43,
            crate::entry!(0, clear(Ptr, u32, Ptr, u32, u32, u32, u32) -> u32).1,
        ),
        (44, crate::entry!(0, fwd_set_transform(Ptr, u32, Ptr) -> u32).1),
        (45, crate::entry!(0, get_transform(Ptr, u32, Ptr) -> u32).1),
        (55, crate::entry!(0, fwd_set_clip_plane(Ptr, u32, Ptr) -> u32).1),
        (75, crate::entry!(0, fwd_set_scissor_rect(Ptr, Ptr) -> u32).1),
        (76, crate::entry!(0, get_scissor_rect(Ptr, Ptr) -> u32).1),
        (
            57,
            WRITER.render_state.0,
        ),
        (
            58,
            crate::entry!(0, get_render_state(Ptr, u32, Ptr) -> u32).1,
        ),
        (64, crate::entry!(0, get_texture(Ptr, u32, Ptr) -> u32).1),
        (65, WRITER.texture.0),
        (
            66,
            crate::entry!(0, get_texture_stage_state(Ptr, u32, u32, Ptr) -> u32).1,
        ),
        (
            67,
            WRITER.stage_state.0,
        ),
        (
            68,
            crate::entry!(0, get_sampler_state(Ptr, u32, u32, Ptr) -> u32).1,
        ),
        (
            69,
            WRITER.sampler_state.0,
        ),
        (48, crate::entry!(0, get_viewport(Ptr, Ptr) -> u32).1),
        (49, crate::entry!(0, fwd_set_material(Ptr, Ptr) -> u32).1),
        (51, crate::entry!(0, fwd_set_light(Ptr, u32, Ptr) -> u32).1),
        (53, crate::entry!(0, fwd_light_enable(Ptr, u32, u32) -> u32).1),
        (
            77,
            crate::entry!(0, fwd_set_software_vertex_processing(Ptr, u32) -> u32).1,
        ),
        (
            31,
            crate::entry!(0, fwd_update_texture(Ptr, u32, u32) -> u32).1,
        ),
        (
            34,
            crate::entry!(0, fwd_stretch_rect(Ptr, u32, u32, u32, u32, u32) -> u32).1,
        ),
    ];
    for (slot, f) in device {
        e.register(DEVICE_CODE + 4 * slot, f);
    }
    use super::d3d9resource as r;
    let resource: [(u32, crate::AbiFn); 32] = [
        (
            26,
            crate::entry!(0, r::create_vertex_buffer(Ptr, u32, u32, u32, u32, Ptr, u32) -> u32).1,
        ),
        (
            27,
            crate::entry!(0, r::create_index_buffer(Ptr, u32, u32, u32, u32, Ptr, u32) -> u32).1,
        ),
        (
            86,
            crate::entry!(0, r::create_vertex_declaration(Ptr, Ptr, Ptr) -> u32).1,
        ),
        (
            87,
            crate::entry!(0, r::fwd_set_vertex_declaration(Ptr, u32) -> u32).1,
        ),
        (89, crate::entry!(0, r::fwd_set_fvf(Ptr, u32) -> u32).1),
        (90, crate::entry!(0, r::get_fvf(Ptr, Ptr) -> u32).1),
        (
            91,
            crate::entry!(0, r::create_vertex_shader(Ptr, Ptr, Ptr) -> u32).1,
        ),
        (
            92,
            crate::entry!(0, r::fwd_set_vertex_shader(Ptr, u32) -> u32).1,
        ),
        (
            93,
            crate::entry!(0, r::get_vertex_shader(Ptr, Ptr) -> u32).1,
        ),
        (94, WRITER.vertex_constants.0),
        (
            95,
            crate::entry!(0, r::get_vertex_constant_f(Ptr, u32, Ptr, u32) -> u32).1,
        ),
        (
            96,
            crate::entry!(0, r::fwd_set_vertex_constant_i(Ptr, u32, Ptr, u32) -> u32).1,
        ),
        (
            97,
            crate::entry!(0, r::get_vertex_constant_i(Ptr, u32, Ptr, u32) -> u32).1,
        ),
        (
            98,
            crate::entry!(0, r::fwd_set_vertex_constant_b(Ptr, u32, Ptr, u32) -> u32).1,
        ),
        (
            99,
            crate::entry!(0, r::get_vertex_constant_b(Ptr, u32, Ptr, u32) -> u32).1,
        ),
        (100, WRITER.stream_source.0),
        (
            101,
            crate::entry!(0, r::get_stream_source(Ptr, u32, Ptr, Ptr, Ptr) -> u32).1,
        ),
        (81, WRITER.draw_primitive.0),
        (82, WRITER.draw_indexed_primitive.0),
        (
            83,
            crate::entry!(0, r::fwd_draw_primitive_up(Ptr, u32, u32, u32, u32) -> u32).1,
        ),
        (
            88,
            crate::entry!(0, r::get_vertex_declaration(Ptr, Ptr) -> u32).1,
        ),
        (104, crate::entry!(0, r::fwd_set_indices(Ptr, u32) -> u32).1),
        (105, crate::entry!(0, r::get_indices(Ptr, Ptr) -> u32).1),
        (
            106,
            crate::entry!(0, r::create_pixel_shader(Ptr, Ptr, Ptr) -> u32).1,
        ),
        (
            107,
            crate::entry!(0, r::fwd_set_pixel_shader(Ptr, u32) -> u32).1,
        ),
        (
            108,
            crate::entry!(0, r::get_pixel_shader(Ptr, Ptr) -> u32).1,
        ),
        (109, WRITER.pixel_constants.0),
        (
            110,
            crate::entry!(0, r::get_pixel_constant_f(Ptr, u32, Ptr, u32) -> u32).1,
        ),
        (
            111,
            crate::entry!(0, r::fwd_set_pixel_constant_i(Ptr, u32, Ptr, u32) -> u32).1,
        ),
        (
            112,
            crate::entry!(0, r::get_pixel_constant_i(Ptr, u32, Ptr, u32) -> u32).1,
        ),
        (
            113,
            crate::entry!(0, r::fwd_set_pixel_constant_b(Ptr, u32, Ptr, u32) -> u32).1,
        ),
        (
            114,
            crate::entry!(0, r::get_pixel_constant_b(Ptr, u32, Ptr, u32) -> u32).1,
        ),
    ];
    for (slot, f) in resource {
        e.register(DEVICE_CODE + 4 * slot, f);
    }
    let surface: [(u32, crate::AbiFn); 9] = [
        (
            0,
            crate::entry!(0, surface_query_interface(Ptr, Ptr, Ptr) -> u32).1,
        ),
        (1, crate::entry!(0, surface_add_ref(Ptr) -> u32).1),
        (2, crate::entry!(0, surface_release(Ptr) -> u32).1),
        (3, crate::entry!(0, surface_get_device(Ptr, Ptr) -> u32).1),
        (10, crate::entry!(0, surface_get_type(Ptr) -> u32).1),
        (
            11,
            crate::entry!(0, surface_get_container(Ptr, Ptr, Ptr) -> u32).1,
        ),
        (12, crate::entry!(0, surface_get_desc(Ptr, Ptr) -> u32).1),
        (
            13,
            crate::entry!(0, surface_lock_rect(Ptr, Ptr, Ptr, u32) -> u32).1,
        ),
        (14, crate::entry!(0, surface_unlock_rect(Ptr) -> u32).1),
    ];
    for (slot, f) in surface {
        e.register(SURFACE_CODE + 4 * slot, f);
    }
    let texture: [(u32, crate::AbiFn); 15] = [
        (
            0,
            crate::entry!(0, texture_query_interface(Ptr, Ptr, Ptr) -> u32).1,
        ),
        (1, crate::entry!(0, texture_add_ref(Ptr) -> u32).1),
        (2, crate::entry!(0, texture_release(Ptr) -> u32).1),
        (3, crate::entry!(0, texture_get_device(Ptr, Ptr) -> u32).1),
        (7, crate::entry!(0, texture_set_priority(Ptr, u32) -> u32).1),
        (8, crate::entry!(0, texture_get_priority(Ptr) -> u32).1),
        (9, crate::entry!(0, texture_pre_load(Ptr)).1),
        (10, crate::entry!(0, texture_get_type(Ptr) -> u32).1),
        (11, crate::entry!(0, fwd_texture_set_lod(Ptr, u32) -> u32).1),
        (12, crate::entry!(0, texture_get_lod(Ptr) -> u32).1),
        (13, crate::entry!(0, texture_get_level_count(Ptr) -> u32).1),
        (
            17,
            crate::entry!(0, texture_get_level_desc(Ptr, u32, Ptr) -> u32).1,
        ),
        (
            18,
            crate::entry!(0, texture_get_surface_level(Ptr, u32, Ptr) -> u32).1,
        ),
        (
            19,
            crate::entry!(0, texture_lock_rect(Ptr, u32, Ptr, Ptr, u32) -> u32).1,
        ),
        (20, crate::entry!(0, texture_unlock_rect(Ptr, u32) -> u32).1),
    ];
    for (slot, f) in texture {
        e.register(TEXTURE_CODE + 4 * slot, f);
    }
    // `IDirect3DCubeTexture9`: the base-texture slots are the texture's (they read the same
    // object kind); its own slots take a face.
    let cube: [(u32, crate::AbiFn); 15] = [
        (
            0,
            crate::entry!(0, cube_query_interface(Ptr, Ptr, Ptr) -> u32).1,
        ),
        (1, crate::entry!(0, texture_add_ref(Ptr) -> u32).1),
        (2, crate::entry!(0, texture_release(Ptr) -> u32).1),
        (3, crate::entry!(0, texture_get_device(Ptr, Ptr) -> u32).1),
        (7, crate::entry!(0, texture_set_priority(Ptr, u32) -> u32).1),
        (8, crate::entry!(0, texture_get_priority(Ptr) -> u32).1),
        (9, crate::entry!(0, texture_pre_load(Ptr)).1),
        (10, crate::entry!(0, cube_get_type(Ptr) -> u32).1),
        (11, crate::entry!(0, fwd_texture_set_lod(Ptr, u32) -> u32).1),
        (12, crate::entry!(0, texture_get_lod(Ptr) -> u32).1),
        (13, crate::entry!(0, texture_get_level_count(Ptr) -> u32).1),
        (
            17,
            crate::entry!(0, texture_get_level_desc(Ptr, u32, Ptr) -> u32).1,
        ),
        (
            18,
            crate::entry!(0, cube_get_surface(Ptr, u32, u32, Ptr) -> u32).1,
        ),
        (
            19,
            crate::entry!(0, cube_lock_rect(Ptr, u32, u32, Ptr, Ptr, u32) -> u32).1,
        ),
        (
            20,
            crate::entry!(0, cube_unlock_rect(Ptr, u32, u32) -> u32).1,
        ),
    ];
    for (slot, f) in cube {
        e.register(CUBE_CODE + 4 * slot, f);
    }
    if device!(e).device_objects.device_vtable == 0 {
        let device = vtable(e, DEVICE_CODE, DEVICE_SLOTS.len());
        let surface = vtable(e, SURFACE_CODE, SURFACE_SLOTS.len());
        let texture = vtable(e, TEXTURE_CODE, TEXTURE_SLOTS.len());
        let cube = vtable(e, CUBE_CODE, TEXTURE_SLOTS.len());
        let objects = &mut device!(e).device_objects;
        objects.device_vtable = device;
        objects.surface_vtable = surface;
        objects.texture_vtable = texture;
        objects.cube_vtable = cube;
    }
}

fn not_provided(what: &str) -> ! {
    panic!("IDirect3D9::CreateDevice: {what} is not provided")
}

fn new_surface(e: &mut Engine, device: u32, implicit: bool, desc: [u32; 8]) -> u32 {
    let object = e.mem.alloc(8);
    let table = device!(e).device_objects.surface_vtable;
    e.mem.set_u32(object, table);
    let references = u32::from(!implicit);
    device!(e).device_objects.surfaces.insert(
        object,
        Surface {
            references,
            device,
            implicit,
            desc,
            container: 0,
            bits: 0,
            pitch: 0,
            locked: false,
            pixels: None,
            staging: 0,
        },
    );
    object
}

// Platform: replaces D3D9.DLL IDirect3D9::CreateDevice (vtable +0x40)
/// `CreateDevice(adapter, device type, focus window, behaviour flags, present parameters,
/// device out)`. A null present parameters or out, another adapter, or behaviour flags without
/// exactly one of the vertex-processing flags answer `D3DERR_INVALIDCALL`; so does a windowed
/// device with neither a device window nor a focus window, or with a full-screen refresh rate,
/// and a back buffer count above `D3DPRESENT_BACK_BUFFERS_MAX` (3; the count is set to 3, so a
/// second call can succeed). A count of 0 becomes 1 and the format `D3DFMT_UNKNOWN` the
/// display mode's format, written back into the caller's parameters as Direct3D does. The
/// device gets its back buffers (render-target surfaces of the parameters' size and format;
/// the first is render target 0), the automatic depth-stencil surface when asked for, the
/// viewport of the whole back buffer (depth 0 to 1), a reference to this `IDirect3D9`, and its
/// creation runs [`crate::direct3d_device_created`] (the calling thread's
/// x87 precision). Not provided (the run stops): other device types, software vertex
/// processing, full-screen devices, a zero width or height (the window's client area),
/// multisampling, and a back buffer format the table's `CheckDeviceType` refuses.
#[allow(clippy::too_many_arguments)]
pub fn create_device(
    e: &mut Engine,
    this: Ptr,
    adapter: u32,
    device_type: u32,
    focus: u32,
    behavior: u32,
    present: Ptr,
    out: Ptr,
) -> u32 {
    if present.is_null() || out.is_null() || adapter != 0 {
        return D3DERR_INVALIDCALL;
    }
    let processing =
        behavior & (SOFTWARE_VERTEXPROCESSING | HARDWARE_VERTEXPROCESSING | MIXED_VERTEXPROCESSING);
    if processing.count_ones() != 1 {
        return D3DERR_INVALIDCALL;
    }
    if device_type != D3DDEVTYPE_HAL {
        not_provided(&format!("device type {device_type}"));
    }
    if processing == SOFTWARE_VERTEXPROCESSING {
        not_provided("software vertex processing");
    }
    let p = present.addr();
    let field = |e: &Engine, offset: u32| e.mem.u32(p + offset);
    if field(e, PP_WINDOWED) == 0 {
        not_provided("a full-screen device");
    }
    if field(e, PP_WINDOW) == 0 && focus == 0 {
        return D3DERR_INVALIDCALL;
    }
    if field(e, PP_REFRESH) != 0 {
        return D3DERR_INVALIDCALL;
    }
    if field(e, PP_COUNT) > BACK_BUFFERS_MAX {
        e.mem.set_u32(p + PP_COUNT, BACK_BUFFERS_MAX);
        return D3DERR_INVALIDCALL;
    }
    if field(e, PP_WIDTH) == 0 || field(e, PP_HEIGHT) == 0 {
        not_provided("a back buffer size taken from the window");
    }
    if field(e, PP_MULTISAMPLE) != 0 {
        not_provided("multisampling");
    }
    // The display mode's format (`D3DDISPLAYMODE` +0xC).
    let display_format = e.with_stack(0x10, |e, mode| {
        let hr = e.vcall(this.addr(), 4 * 8, &[0, mode.addr()]).u32();
        assert_eq!(hr, D3D_OK, "GetAdapterDisplayMode for CreateDevice");
        e.mem.u32(mode.addr() + 0xc)
    });
    if field(e, PP_COUNT) == 0 {
        e.mem.set_u32(p + PP_COUNT, 1);
    }
    if field(e, PP_FORMAT) == D3DFMT_UNKNOWN {
        e.mem.set_u32(p + PP_FORMAT, display_format);
    }
    let format = field(e, PP_FORMAT);
    let check = e
        .vcall(
            this.addr(),
            4 * 9,
            &[adapter, device_type, display_format, format, 1],
        )
        .u32();
    if check != D3D_OK {
        not_provided(&format!(
            "a windowed back buffer of format {format} (CheckDeviceType answers {check:#x})"
        ));
    }
    register_code(e);
    let object = e.mem.alloc(8);
    let table = device!(e).device_objects.device_vtable;
    e.mem.set_u32(object, table);
    let (width, height) = (field(e, PP_WIDTH), field(e, PP_HEIGHT));
    let quality = field(e, PP_QUALITY);
    let back_buffers: Vec<u32> = (0..field(e, PP_COUNT))
        .map(|_| {
            let desc = [
                format,
                D3DRTYPE_SURFACE,
                D3DUSAGE_RENDERTARGET,
                0,
                0,
                quality,
                width,
                height,
            ];
            new_surface(e, object, true, desc)
        })
        .collect();
    let depth_stencil = if field(e, PP_AUTO_DEPTH) != 0 {
        let depth_format = field(e, PP_DEPTH_FORMAT);
        let desc = [
            depth_format,
            D3DRTYPE_SURFACE,
            D3DUSAGE_DEPTHSTENCIL,
            0,
            0,
            quality,
            width,
            height,
        ];
        new_surface(e, object, true, desc)
    } else {
        0
    };
    let mut render_targets = [0; RENDER_TARGETS];
    render_targets[0] = back_buffers[0];
    // The device holds its render target and depth surface (counted, as `SetRenderTarget` and
    // `SetDepthStencilSurface` do).
    surface_add_ref(e, Ptr::new(back_buffers[0]));
    if depth_stencil != 0 {
        surface_add_ref(e, Ptr::new(depth_stencil));
    }
    e.vcall(this.addr(), 4, &[]);
    let device = Device {
        references: 1,
        direct3d: this.addr(),
        creation: [adapter, device_type, focus, behavior],
        present: e.mem.bytes(p, PRESENT_SIZE),
        back_buffers,
        render_targets,
        depth_stencil,
        viewport: [0, 0, width, height, 0f32.to_bits(), 1f32.to_bits()],
        gamma_ramp: None,
        render_states: Default::default(),
        stage_states: Default::default(),
        sampler_states: Default::default(),
        textures: Default::default(),
        transforms: Default::default(),
        scissor: [0, 0, width, height],
        clip_planes: Default::default(),
        in_scene: false,
        material: None,
        lights: Default::default(),
        enabled_lights: std::collections::BTreeSet::new(),
        software_vertex_processing: false,
        lost: false,
    };
    device!(e).device_objects.devices.insert(object, device);
    crate::direct3d_device_created(e, behavior);
    {
        let d = &device!(e).device_objects.devices[&object];
        let (present, back_buffers, depth) =
            (d.present.clone(), d.back_buffers.clone(), d.depth_stencil);
        super::d3d9real::device_created(e, object, focus, behavior, &present, &back_buffers, depth);
    }
    e.mem.set_u32(out.addr(), object);
    D3D_OK
}

/// The device's state as this call sees it: the drawing side's copy for a call from the drawing
/// side of an open cut (`super::d3d9draw`), main's otherwise.
fn device(e: &mut Engine, this: Ptr) -> &mut Device {
    device_in(&mut device!(e).device_objects, this)
}

/// [`device`] for code that holds the devices' objects already (the state sets in one pass,
/// [`state_one_pass`]).
#[inline]
fn device_in(objects: &mut Objects, this: Ptr) -> &mut Device {
    if super::d3d9draw::drawing_side(&objects.draw) {
        if let Some(d) = objects.draw_devices.get_mut(&this.addr()) {
            return d;
        }
    }
    objects
        .devices
        .get_mut(&this.addr())
        .expect("an IDirect3DDevice9 of this layer")
}

/// The device itself (its references, its present parameters, whether it is lost): main's, from
/// either side.
fn main_device(e: &mut Engine, this: Ptr) -> &mut Device {
    device!(e)
        .device_objects
        .devices
        .get_mut(&this.addr())
        .expect("an IDirect3DDevice9 of this layer")
}

/// Notes that main set `key` while a cut is open (`super::d3d9draw::main_set`).
fn mark(e: &mut Engine, key: super::d3d9draw::Key) {
    if super::d3d9draw::any_cut() {
        super::d3d9draw::main_set(&mut device!(e).device_objects.draw, key);
    }
}

/// Whether the device is between `BeginScene` and `EndScene`.
pub(super) fn in_scene(e: &mut Engine, this: Ptr) -> bool {
    device(e, this).in_scene
}

pub(super) fn iid_is(e: &Engine, iid: Ptr, ids: &[[u8; 16]]) -> bool {
    if iid.is_null() || ids.is_empty() {
        return false;
    }
    let mut asked = [0u8; 16];
    e.mem.read(iid.addr(), &mut asked);
    ids.contains(&asked)
}

/// `QueryInterface(iid, out)`: `IUnknown` and `IDirect3DDevice9` answer the device (counted);
/// others `E_NOINTERFACE` with a null out.
fn device_query_interface(e: &mut Engine, this: Ptr, iid: Ptr, out: Ptr) -> u32 {
    if out.is_null() {
        return D3DERR_INVALIDCALL;
    }
    if iid_is(e, iid, &[IID_IUNKNOWN, IID_DEVICE9]) {
        device_add_ref(e, this);
        e.mem.set_u32(out.addr(), this.addr());
        return D3D_OK;
    }
    e.mem.set_u32(out.addr(), 0);
    E_NOINTERFACE
}

pub(super) fn device_add_ref(e: &mut Engine, this: Ptr) -> u32 {
    let d = main_device(e, this);
    d.references += 1;
    d.references
}

/// `Release()`: the last reference destroys the device: its implicit surfaces go, the surfaces
/// it holds in its render target and depth slots are released, and its `IDirect3D9` reference.
pub(super) fn device_release(e: &mut Engine, this: Ptr) -> u32 {
    let d = main_device(e, this);
    d.references -= 1;
    let left = d.references;
    if left == 0 {
        let d = device!(e)
            .device_objects
            .devices
            .remove(&this.addr())
            .expect("the device");
        for held in d.render_targets.iter().chain([&d.depth_stencil]) {
            if *held != 0 {
                surface_release(e, Ptr::new(*held));
            }
        }
        for texture in d.textures.values() {
            texture_unbind(e, Ptr::new(*texture));
        }
        super::d3d9resource::release_bindings(e, this.addr());
        let implicit: Vec<u32> = device!(e)
            .device_objects
            .surfaces
            .iter()
            .filter(|(_, s)| s.device == this.addr() && s.implicit)
            .map(|(a, _)| *a)
            .collect();
        for surface in implicit {
            device!(e).device_objects.surfaces.remove(&surface);
            super::d3d9real::forget(e, surface);
            e.mem.free(surface);
        }
        super::d3d9real::forget(e, this.addr());
        e.vcall(d.direct3d, 4 * 2, &[]);
        e.mem.free(this.addr());
    }
    left
}

/// `GetDirect3D(out)`: the `IDirect3D9` that made the device, counted.
fn get_direct3d(e: &mut Engine, this: Ptr, out: Ptr) -> u32 {
    if out.is_null() {
        return D3DERR_INVALIDCALL;
    }
    let direct3d = device(e, this).direct3d;
    e.vcall(direct3d, 4, &[]);
    e.mem.set_u32(out.addr(), direct3d);
    D3D_OK
}

/// `GetDeviceCaps(out)`: the adapter's caps for the device's type
/// (`IDirect3D9::GetDeviceCaps`, from the table); with hardware or mixed vertex processing the
/// device reports the same caps.
fn get_device_caps(e: &mut Engine, this: Ptr, out: Ptr) -> u32 {
    if out.is_null() {
        return D3DERR_INVALIDCALL;
    }
    let d = device(e, this);
    let (direct3d, [adapter, device_type, ..]) = (d.direct3d, d.creation);
    e.vcall(direct3d, 4 * 14, &[adapter, device_type, out.addr()])
        .u32()
}

/// `GetDisplayMode(swap chain, out)`: for the implicit swap chain (0) of a windowed device the
/// adapter's display mode; another swap chain `D3DERR_INVALIDCALL`.
fn get_display_mode(e: &mut Engine, this: Ptr, swap_chain: u32, out: Ptr) -> u32 {
    if swap_chain != 0 || out.is_null() {
        return D3DERR_INVALIDCALL;
    }
    let d = device(e, this);
    let (direct3d, adapter) = (d.direct3d, d.creation[0]);
    e.vcall(direct3d, 4 * 8, &[adapter, out.addr()]).u32()
}

/// `GetCreationParameters(out)`: the adapter, device type, focus window and behaviour flags
/// `CreateDevice` was given.
fn get_creation_parameters(e: &mut Engine, this: Ptr, out: Ptr) -> u32 {
    if out.is_null() {
        return D3DERR_INVALIDCALL;
    }
    let creation = device(e, this).creation;
    for (i, word) in creation.iter().enumerate() {
        e.mem.set_u32(out.addr() + 4 * i as u32, *word);
    }
    D3D_OK
}

/// `GetBackBuffer(swap chain, index, type, out)`: back buffer `index` of the implicit swap
/// chain (0), counted; another swap chain, an index past the count, a type other than
/// `D3DBACKBUFFER_TYPE_MONO` (0) or a null out answer `D3DERR_INVALIDCALL`.
fn get_back_buffer(
    e: &mut Engine,
    this: Ptr,
    swap_chain: u32,
    index: u32,
    kind: u32,
    out: Ptr,
) -> u32 {
    if out.is_null() {
        return D3DERR_INVALIDCALL;
    }
    let buffers = device(e, this).back_buffers.clone();
    if swap_chain != 0 || kind != 0 || index as usize >= buffers.len() {
        return D3DERR_INVALIDCALL;
    }
    let surface = buffers[index as usize];
    surface_add_ref(e, Ptr::new(surface));
    e.mem.set_u32(out.addr(), surface);
    D3D_OK
}

fn simultaneous_render_targets(e: &mut Engine, this: Ptr) -> u32 {
    e.with_stack(0x130, |e, caps| {
        let hr = get_device_caps(e, this, caps);
        assert_eq!(hr, D3D_OK, "the device's caps");
        e.mem.u32(caps.addr() + CAPS_SIMULTANEOUS_RTS as u32)
    })
}

/// `SetRenderTarget(index, surface)`: an index at or past the caps' `NumSimultaneousRTs`, a
/// null surface for index 0, or a surface without `D3DUSAGE_RENDERTARGET` answer
/// `D3DERR_INVALIDCALL`. The device holds the new surface (counted) and releases the old one;
/// setting index 0 resets the viewport to the whole surface (depth 0 to 1).
fn set_render_target(e: &mut Engine, this: Ptr, index: u32, surface: u32) -> u32 {
    if index >= simultaneous_render_targets(e, this) || index as usize >= RENDER_TARGETS {
        return D3DERR_INVALIDCALL;
    }
    if surface == 0 && index == 0 {
        return D3DERR_INVALIDCALL;
    }
    let desc = if surface != 0 {
        match device!(e).device_objects.surfaces.get(&surface) {
            Some(s) if s.desc[2] & D3DUSAGE_RENDERTARGET != 0 => Some(s.desc),
            _ => return D3DERR_INVALIDCALL,
        }
    } else {
        None
    };
    if surface != 0 {
        surface_add_ref(e, Ptr::new(surface));
    }
    mark(e, super::d3d9draw::Key::RenderTarget(index));
    let old = std::mem::replace(&mut device(e, this).render_targets[index as usize], surface);
    if old != 0 {
        surface_release(e, Ptr::new(old));
    }
    if let (0, Some(desc)) = (index, desc) {
        mark(e, super::d3d9draw::Key::Viewport);
        mark(e, super::d3d9draw::Key::Scissor);
        device(e, this).viewport = [0, 0, desc[6], desc[7], 0f32.to_bits(), 1f32.to_bits()];
        device(e, this).scissor = [0, 0, desc[6], desc[7]];
    }
    D3D_OK
}

/// `GetRenderTarget(index, out)`: the surface in that slot, counted; an empty slot answers
/// `D3DERR_NOTFOUND` (out null); an index at or past `NumSimultaneousRTs` or a null out
/// `D3DERR_INVALIDCALL`.
fn get_render_target(e: &mut Engine, this: Ptr, index: u32, out: Ptr) -> u32 {
    if out.is_null()
        || index >= simultaneous_render_targets(e, this)
        || index as usize >= RENDER_TARGETS
    {
        return D3DERR_INVALIDCALL;
    }
    let surface = device(e, this).render_targets[index as usize];
    e.mem.set_u32(out.addr(), surface);
    if surface == 0 {
        return D3DERR_NOTFOUND;
    }
    surface_add_ref(e, Ptr::new(surface));
    D3D_OK
}

/// `SetDepthStencilSurface(surface)`: null removes the depth buffer; a surface without
/// `D3DUSAGE_DEPTHSTENCIL` answers `D3DERR_INVALIDCALL`. The device holds the new surface
/// (counted) and releases the old one.
fn set_depth_stencil_surface(e: &mut Engine, this: Ptr, surface: u32) -> u32 {
    if surface != 0 {
        match device!(e).device_objects.surfaces.get(&surface) {
            Some(s) if s.desc[2] & D3DUSAGE_DEPTHSTENCIL != 0 => {}
            _ => return D3DERR_INVALIDCALL,
        }
        surface_add_ref(e, Ptr::new(surface));
    }
    mark(e, super::d3d9draw::Key::DepthStencil);
    let old = std::mem::replace(&mut device(e, this).depth_stencil, surface);
    if old != 0 {
        surface_release(e, Ptr::new(old));
    }
    D3D_OK
}

/// `GetDepthStencilSurface(out)`: the depth surface, counted; none answers `D3DERR_NOTFOUND`
/// (out null).
fn get_depth_stencil_surface(e: &mut Engine, this: Ptr, out: Ptr) -> u32 {
    if out.is_null() {
        return D3DERR_INVALIDCALL;
    }
    let surface = device(e, this).depth_stencil;
    e.mem.set_u32(out.addr(), surface);
    if surface == 0 {
        return D3DERR_NOTFOUND;
    }
    surface_add_ref(e, Ptr::new(surface));
    D3D_OK
}

/// `SetViewport(viewport)`: keeps the six words of the `D3DVIEWPORT9`; a null pointer, or a
/// rectangle outside render target 0, answers `D3DERR_INVALIDCALL`.
fn set_viewport(e: &mut Engine, this: Ptr, viewport: Ptr) -> u32 {
    if viewport.is_null() {
        return D3DERR_INVALIDCALL;
    }
    let words: [u32; 6] = std::array::from_fn(|i| e.mem.u32(viewport.addr() + 4 * i as u32));
    let target = device(e, this).render_targets[0];
    let desc = device!(e).device_objects.surfaces[&target].desc;
    let right = u64::from(words[0]) + u64::from(words[2]);
    let bottom = u64::from(words[1]) + u64::from(words[3]);
    if right > u64::from(desc[6]) || bottom > u64::from(desc[7]) {
        return D3DERR_INVALIDCALL;
    }
    mark(e, super::d3d9draw::Key::Viewport);
    let d = device(e, this);
    d.viewport.copy_from_slice(&words);
    D3D_OK
}

/// `GetViewport(out)`: the viewport's six words.
fn get_viewport(e: &mut Engine, this: Ptr, out: Ptr) -> u32 {
    if out.is_null() {
        return D3DERR_INVALIDCALL;
    }
    let viewport = device(e, this).viewport;
    for (i, word) in viewport.iter().enumerate() {
        e.mem.set_u32(out.addr() + 4 * i as u32, *word);
    }
    D3D_OK
}

fn surface(e: &mut Engine, this: Ptr) -> &mut Surface {
    device!(e)
        .device_objects
        .surfaces
        .get_mut(&this.addr())
        .expect("an IDirect3DSurface9 of this layer")
}

/// `QueryInterface(iid, out)`: `IUnknown`, `IDirect3DResource9` and `IDirect3DSurface9` answer
/// the surface (counted); others `E_NOINTERFACE` with a null out.
fn surface_query_interface(e: &mut Engine, this: Ptr, iid: Ptr, out: Ptr) -> u32 {
    if out.is_null() {
        return D3DERR_INVALIDCALL;
    }
    if iid_is(e, iid, &[IID_IUNKNOWN, IID_RESOURCE9, IID_SURFACE9]) {
        surface_add_ref(e, this);
        e.mem.set_u32(out.addr(), this.addr());
        return D3D_OK;
    }
    e.mem.set_u32(out.addr(), 0);
    E_NOINTERFACE
}

/// `AddRef()`; a texture's level counts its texture (Direct3D 9 sub-objects share their
/// container's count).
fn surface_add_ref(e: &mut Engine, this: Ptr) -> u32 {
    let s = surface(e, this);
    if s.container != 0 {
        let container = s.container;
        return texture_add_ref(e, Ptr::new(container));
    }
    s.references += 1;
    s.references
}

/// `Release()`: the count falls; a surface the program made goes at 0, one the device owns
/// (back buffers, the automatic depth surface) stays until the device goes, and a texture's
/// level releases its texture.
fn surface_release(e: &mut Engine, this: Ptr) -> u32 {
    let s = surface(e, this);
    if s.container != 0 {
        let container = s.container;
        return texture_release(e, Ptr::new(container));
    }
    let s = surface(e, this);
    s.references = s.references.saturating_sub(1);
    let (left, implicit) = (s.references, s.implicit);
    let surface_owner = s.device;
    if left == 0 && !implicit {
        let removed = device!(e).device_objects.surfaces.remove(&this.addr());
        if let Some(s) = removed {
            if s.bits != 0 {
                e.mem.free(s.bits);
            }
        }
        super::d3d9real::forget(e, this.addr());
        e.mem.free(this.addr());
        // A surface the program made (CreateRenderTarget, CreateDepthStencilSurface) held its
        // device.
        let owner = surface_owner;
        device_release(e, Ptr::new(owner));
    }
    left
}

/// `GetDevice(out)`: the surface's device, counted.
fn surface_get_device(e: &mut Engine, this: Ptr, out: Ptr) -> u32 {
    if out.is_null() {
        return D3DERR_INVALIDCALL;
    }
    let owner = surface(e, this).device;
    device_add_ref(e, Ptr::new(owner));
    e.mem.set_u32(out.addr(), owner);
    D3D_OK
}

/// `GetType()`: `D3DRTYPE_SURFACE`.
fn surface_get_type(_e: &mut Engine, _this: Ptr) -> u32 {
    D3DRTYPE_SURFACE
}

/// `GetDesc(out)`: the 0x20-byte `D3DSURFACE_DESC`.
fn surface_get_desc(e: &mut Engine, this: Ptr, out: Ptr) -> u32 {
    if out.is_null() {
        return D3DERR_INVALIDCALL;
    }
    let desc = surface(e, this).desc;
    for (i, word) in desc.iter().enumerate() {
        e.mem.set_u32(out.addr() + 4 * i as u32, *word);
    }
    D3D_OK
}

/// The 16 words of the identity matrix (`float` 1.0 on the diagonal).
fn identity() -> [u32; 16] {
    let mut m = [0u32; 16];
    for i in 0..4 {
        m[5 * i] = 1f32.to_bits();
    }
    m
}

/// Whether `state` names a transform: `D3DTS_VIEW` (2), `D3DTS_PROJECTION` (3),
/// `D3DTS_TEXTURE0..7` (16 to 23) or `D3DTS_WORLDMATRIX(0..255)` (256 to 511).
fn is_transform(state: u32) -> bool {
    matches!(state, 2 | 3 | 16..=23 | 256..=511)
}

// Platform: replaces D3D9.DLL IDirect3DDevice9::SetTransform (vtable +0xB0)
/// `SetTransform(state, matrix)`: keeps the 16 words of the `D3DMATRIX`; a null matrix or a
/// state that names no transform answers `D3DERR_INVALIDCALL`.
fn set_transform(e: &mut Engine, this: Ptr, state: u32, matrix: Ptr) -> u32 {
    if matrix.is_null() || !is_transform(state) {
        return D3DERR_INVALIDCALL;
    }
    let mut m = [0u32; 16];
    for (i, word) in m.iter_mut().enumerate() {
        *word = e.mem.u32(matrix.addr() + 4 * i as u32);
    }
    mark(e, super::d3d9draw::Key::Transform(state));
    device(e, this).transforms.insert(state, m);
    D3D_OK
}

/// `GetTransform(state, out)`: the matrix set, or the identity (every transform starts as
/// the identity).
fn get_transform(e: &mut Engine, this: Ptr, state: u32, out: Ptr) -> u32 {
    if out.is_null() || !is_transform(state) {
        return D3DERR_INVALIDCALL;
    }
    let m = device(e, this)
        .transforms
        .get(&state)
        .copied()
        .unwrap_or_else(identity);
    for (i, word) in m.iter().enumerate() {
        e.mem.set_u32(out.addr() + 4 * i as u32, *word);
    }
    D3D_OK
}

/// `SetScissorRect(rect)`: keeps the `RECT`; null answers `D3DERR_INVALIDCALL`.
fn set_scissor_rect(e: &mut Engine, this: Ptr, rect: Ptr) -> u32 {
    if rect.is_null() {
        return D3DERR_INVALIDCALL;
    }
    let mut r = [0u32; 4];
    for (i, word) in r.iter_mut().enumerate() {
        *word = e.mem.u32(rect.addr() + 4 * i as u32);
    }
    mark(e, super::d3d9draw::Key::Scissor);
    device(e, this).scissor = r;
    D3D_OK
}

/// `GetScissorRect(out)`: the rectangle set (at first, and after `SetRenderTarget` on index 0,
/// the whole render target).
fn get_scissor_rect(e: &mut Engine, this: Ptr, out: Ptr) -> u32 {
    if out.is_null() {
        return D3DERR_INVALIDCALL;
    }
    let r = device(e, this).scissor;
    for (i, word) in r.iter().enumerate() {
        e.mem.set_u32(out.addr() + 4 * i as u32, *word);
    }
    D3D_OK
}

/// `SetClipPlane(index, plane)`: keeps the plane's four words for indices 0 to 5
/// (`D3DMAXUSERCLIPPLANES` 32 is the API's limit; the caps' `MaxUserClipPlanes` the device's);
/// a null plane or an index at or past the caps' count answers `D3DERR_INVALIDCALL`.
fn set_clip_plane(e: &mut Engine, this: Ptr, index: u32, plane: Ptr) -> u32 {
    if plane.is_null() || index >= max_user_clip_planes(e, this) {
        return D3DERR_INVALIDCALL;
    }
    let mut p = [0u32; 4];
    for (i, word) in p.iter_mut().enumerate() {
        *word = e.mem.u32(plane.addr() + 4 * i as u32);
    }
    mark(e, super::d3d9draw::Key::ClipPlane(index));
    device(e, this).clip_planes.insert(index, p);
    D3D_OK
}

/// `D3DCAPS9::MaxUserClipPlanes` (+0xA4).
fn max_user_clip_planes(e: &mut Engine, this: Ptr) -> u32 {
    e.with_stack(0x130, |e, caps| {
        assert_eq!(get_device_caps(e, this, caps), D3D_OK, "the device's caps");
        e.mem.u32(caps.addr() + 0xa4)
    })
}

/// `BeginScene()`: starts a scene; inside one already, `D3DERR_INVALIDCALL`.
fn begin_scene(e: &mut Engine, this: Ptr) -> u32 {
    let model = {
        let d = device(e, this);
        if d.in_scene || d.lost {
            D3DERR_INVALIDCALL
        } else {
            d.in_scene = true;
            D3D_OK
        }
    };
    mark(e, super::d3d9draw::Key::InScene);
    super::d3d9real::begin_scene(e, this.addr(), model);
    model
}

/// `EndScene()`: ends the scene; outside one, `D3DERR_INVALIDCALL`.
fn end_scene(e: &mut Engine, this: Ptr) -> u32 {
    let model = {
        let d = device(e, this);
        if !d.in_scene || d.lost {
            D3DERR_INVALIDCALL
        } else {
            d.in_scene = false;
            D3D_OK
        }
    };
    mark(e, super::d3d9draw::Key::InScene);
    super::d3d9real::end_scene(e, this.addr(), model);
    model
}

/// `D3DCLEAR_TARGET`, `D3DCLEAR_ZBUFFER`, `D3DCLEAR_STENCIL`.
const D3DCLEAR_TARGET: u32 = 1;
const D3DCLEAR_ZBUFFER: u32 = 2;
const D3DCLEAR_STENCIL: u32 = 4;

// Platform: replaces D3D9.DLL IDirect3DDevice9::Clear (vtable +0xAC)
/// `Clear(count, rects, flags, colour, z, stencil)`: nothing is drawn yet (the viewer draws), so
/// only the call's validity is answered: rectangles without a count or a count without
/// rectangles, flags other than the three clear bits or none of them, and a depth or stencil
/// clear without a depth surface answer `D3DERR_INVALIDCALL`.
#[allow(clippy::too_many_arguments)]
fn clear(
    e: &mut Engine,
    this: Ptr,
    count: u32,
    rects: Ptr,
    flags: u32,
    colour: u32,
    z: u32,
    stencil: u32,
) -> u32 {
    let model = clear_answer(e, this, count, rects, flags);
    super::d3d9real::clear(
        e,
        this.addr(),
        count,
        rects.addr(),
        flags,
        colour,
        z,
        stencil,
        model,
    );
    let _ = (colour, z, stencil);
    model
}

fn clear_answer(e: &mut Engine, this: Ptr, count: u32, rects: Ptr, flags: u32) -> u32 {
    if (count == 0) != rects.is_null() {
        return D3DERR_INVALIDCALL;
    }
    let all = D3DCLEAR_TARGET | D3DCLEAR_ZBUFFER | D3DCLEAR_STENCIL;
    if flags == 0 || flags & !all != 0 {
        return D3DERR_INVALIDCALL;
    }
    if flags & (D3DCLEAR_ZBUFFER | D3DCLEAR_STENCIL) != 0 && device(e, this).depth_stencil == 0 {
        return D3DERR_INVALIDCALL;
    }
    D3D_OK
}

// Platform: replaces D3D9.DLL IDirect3DDevice9::Present (vtable +0x44)
/// `Present(source, dest, window, dirty region)`: shows the back buffer; nothing is shown yet
/// (the viewer draws), so it answers `D3D_OK` outside a scene and `D3DERR_INVALIDCALL` inside
/// one.
fn present(e: &mut Engine, this: Ptr, source: u32, dest: u32, window: u32, dirty: u32) -> u32 {
    if crate::queue_check::enabled() {
        // Diagnostic only (`NV_QUEUE_CHECK`, not exe behaviour): the frame ends here.
        crate::queue_check::frame(e);
    }
    let objects = &mut device!(e).device_objects;
    if objects.draw.device.is_some() {
        // A cut is open: the drawing side may still be in the frame's blocks; they are freed at
        // the join (`super::d3d9draw::draw_join`).
        objects.draw.presented = true;
    } else if !objects.unlocked_staging.is_empty() {
        // The texture levels' and buffers' staging blocks unlocked in the frame.
        free_all_unlocked_staging(e);
    }
    let model = if device(e, this).in_scene || device(e, this).lost {
        D3DERR_INVALIDCALL
    } else {
        D3D_OK
    };
    super::d3d9real::present(e, this.addr(), source, dest, window, dirty, model);
    let _ = (source, dest, window, dirty);
    model
}

/// `TestCooperativeLevel()`: a windowed device is not lost here: `D3D_OK`, or
/// `D3DERR_DEVICENOTRESET` after a `Reset` that failed until one succeeds (measured, 2026-10-27).
const D3DERR_DEVICENOTRESET: u32 = 0x8876_0869;
fn test_cooperative_level(e: &mut Engine, this: Ptr) -> u32 {
    if device(e, this).lost {
        return D3DERR_DEVICENOTRESET;
    }
    D3D_OK
}

/// Whether `sampler` names a sampler: 0 to 15, `D3DDMAPSAMPLER` (256) or the vertex samplers
/// `D3DVERTEXTEXTURESAMPLER0..3` (257 to 260).
fn is_sampler(sampler: u32) -> bool {
    sampler < 16 || (256..=260).contains(&sampler)
}

// Platform: replaces D3D9.DLL IDirect3DDevice9::SetGammaRamp (vtable +0x54)
/// `SetGammaRamp(swap_chain, flags, ramp)`: the ramp (`D3DGAMMARAMP`, three tables of 256
/// WORDs) is kept for the swap chain 0, the only one the game makes. The method is `void` in
/// the SDK, so the word in `EAX` is not part of its contract; the game ignores it (004dcef0, the
/// Display menu's brightness). Swap chains other than 0 and a null ramp are not provided.
fn set_gamma_ramp(e: &mut Engine, this: Ptr, swap_chain: u32, flags: u32, ramp: Ptr) -> u32 {
    let _ = flags;
    assert!(
        swap_chain == 0 && !ramp.is_null(),
        "IDirect3DDevice9::SetGammaRamp({swap_chain}, .., {:08x}) is not provided",
        ramp.addr()
    );
    let table = (0..0x300).map(|i| e.mem.u16(ramp.addr() + 2 * i)).collect();
    mark(e, super::d3d9draw::Key::GammaRamp);
    device(e, this).gamma_ramp = Some(table);
    D3D_OK
}

// Platform: replaces D3D9.DLL IDirect3DDevice9::SetRenderState (vtable +0xE4)
/// `SetRenderState(state, value)`: kept; answers `D3D_OK`. A state number above 255
/// (`D3DRENDERSTATETYPE` ends at 209) is not provided.
fn set_render_state(e: &mut Engine, this: Ptr, state: u32, value: u32) -> u32 {
    assert!(
        state < 256,
        "IDirect3DDevice9::SetRenderState({state}) is not provided"
    );
    mark(e, super::d3d9draw::Key::Render(state));
    device(e, this).render_states.set(state as usize, value);
    D3D_OK
}

/// `GetRenderState(state, out)`: the value set. A state the program never set (its default)
/// is not provided yet.
fn get_render_state(e: &mut Engine, this: Ptr, state: u32, out: Ptr) -> u32 {
    if out.is_null() {
        return D3DERR_INVALIDCALL;
    }
    let Some(value) = device(e, this).render_states.get(Some(state as usize)) else {
        panic!("IDirect3DDevice9::GetRenderState({state}) before a SetRenderState: its default is not provided");
    };
    e.mem.set_u32(out.addr(), value);
    D3D_OK
}

/// `SetTextureStageState(stage, type, value)`: kept for stages 0 to 7
/// (`D3DTEXTURESTAGESTATETYPE` 1 to 32); another stage answers `D3DERR_INVALIDCALL`.
fn set_texture_stage_state(e: &mut Engine, this: Ptr, stage: u32, kind: u32, value: u32) -> u32 {
    if stage >= 8 {
        return D3DERR_INVALIDCALL;
    }
    assert!(
        (1..=32).contains(&kind),
        "IDirect3DDevice9::SetTextureStageState(type {kind}) is not provided"
    );
    let index = stage_index(stage, kind).expect("a type 1 to 32");
    mark(e, super::d3d9draw::Key::Stage(index as u32));
    device(e, this).stage_states.set(index, value);
    D3D_OK
}

/// `GetTextureStageState(stage, type, out)`: the value set; one never set is not provided yet.
fn get_texture_stage_state(e: &mut Engine, this: Ptr, stage: u32, kind: u32, out: Ptr) -> u32 {
    if stage >= 8 || out.is_null() {
        return D3DERR_INVALIDCALL;
    }
    let Some(value) = device(e, this).stage_states.get(stage_index(stage, kind)) else {
        panic!("IDirect3DDevice9::GetTextureStageState({stage}, {kind}) before a set: its default is not provided");
    };
    e.mem.set_u32(out.addr(), value);
    D3D_OK
}

/// `SetSamplerState(sampler, type, value)`: kept for the samplers [`is_sampler`] names
/// (`D3DSAMPLERSTATETYPE` 1 to 13); another sampler answers `D3DERR_INVALIDCALL`.
fn set_sampler_state(e: &mut Engine, this: Ptr, sampler: u32, kind: u32, value: u32) -> u32 {
    if !is_sampler(sampler) {
        return D3DERR_INVALIDCALL;
    }
    assert!(
        (1..=13).contains(&kind),
        "IDirect3DDevice9::SetSamplerState(type {kind}) is not provided"
    );
    let index = sampler_index(sampler, kind).expect("a type 1 to 13");
    mark(e, super::d3d9draw::Key::Sampler(index as u32));
    device(e, this).sampler_states.set(index, value);
    D3D_OK
}

/// `GetSamplerState(sampler, type, out)`: the value set; one never set is not provided yet.
fn get_sampler_state(e: &mut Engine, this: Ptr, sampler: u32, kind: u32, out: Ptr) -> u32 {
    if !is_sampler(sampler) || out.is_null() {
        return D3DERR_INVALIDCALL;
    }
    let Some(value) = device(e, this)
        .sampler_states
        .get(sampler_index(sampler, kind))
    else {
        panic!("IDirect3DDevice9::GetSamplerState({sampler}, {kind}) before a set: its default is not provided");
    };
    e.mem.set_u32(out.addr(), value);
    D3D_OK
}

// Platform: replaces D3D9.DLL IDirect3DDevice9::SetTexture (vtable +0x104)
/// `SetTexture(sampler, texture)`: the device holds the texture (counted) for the sampler and
/// releases the one it held; null clears it. Another sampler answers `D3DERR_INVALIDCALL`;
/// a texture not of this layer's kinds (cube and volume textures are not provided yet) stops.
fn set_texture(e: &mut Engine, this: Ptr, sampler: u32, texture: u32) -> u32 {
    if !is_sampler(sampler) {
        return D3DERR_INVALIDCALL;
    }
    // The texture and the device are looked up once each (they are apart in `Objects`), the
    // device's state as this call sees it (main's, or the drawing side's copy).
    let objects = &mut device!(e).device_objects;
    super::d3d9draw::main_set(&mut objects.draw, super::d3d9draw::Key::Texture(sampler));
    let devices = if super::d3d9draw::drawing_side(&objects.draw)
        && objects.draw_devices.contains_key(&this.addr())
    {
        &mut objects.draw_devices
    } else {
        &mut objects.devices
    };
    // The texture the sampler holds already: it is a texture of this layer (a bound texture lives
    // on), and counting it once more and letting go of it once changes nothing, as below, without
    // looking it up.
    if texture != 0
        && devices
            .get(&this.addr())
            .is_some_and(|d| d.textures.get(&sampler) == Some(&texture))
    {
        return D3D_OK;
    }
    let held = if texture != 0 {
        let Some(t) = objects.textures.get_mut(&texture) else {
            panic!(
                "IDirect3DDevice9::SetTexture({sampler}, {texture:#x}): not a texture of this layer"
            );
        };
        Some(t)
    } else {
        None
    };
    let d = devices
        .get_mut(&this.addr())
        .expect("an IDirect3DDevice9 of this layer");
    let old = if let Some(t) = held {
        // The texture the sampler holds already: counting it once more and letting go of it
        // once changes nothing.
        if d.textures.get(&sampler) == Some(&texture) {
            return D3D_OK;
        }
        t.bound += 1;
        d.textures.insert(sampler, texture)
    } else {
        d.textures.remove(&sampler)
    };
    if let Some(old) = old {
        texture_unbind(e, Ptr::new(old));
    }
    D3D_OK
}

/// `GetTexture(sampler, out)`: the texture held for the sampler, counted, or null.
fn get_texture(e: &mut Engine, this: Ptr, sampler: u32, out: Ptr) -> u32 {
    if !is_sampler(sampler) || out.is_null() {
        return D3DERR_INVALIDCALL;
    }
    let texture = device(e, this).textures.get(&sampler).copied().unwrap_or(0);
    if texture != 0 {
        texture_add_ref(e, Ptr::new(texture));
    }
    e.mem.set_u32(out.addr(), texture);
    D3D_OK
}

/// `GetContainer(iid, out)`: a texture's level answers its texture (counted) for `IUnknown`,
/// `IDirect3DResource9`, `IDirect3DBaseTexture9` or `IDirect3DTexture9`, `E_NOINTERFACE` (out
/// null) for other interfaces; the container of other surfaces is not provided.
fn surface_get_container(e: &mut Engine, this: Ptr, iid: Ptr, out: Ptr) -> u32 {
    if out.is_null() {
        return D3DERR_INVALIDCALL;
    }
    let container = surface(e, this).container;
    if container != 0
        && iid_is(
            e,
            iid,
            &[IID_IUNKNOWN, IID_RESOURCE9, IID_BASETEXTURE9, IID_TEXTURE9],
        )
    {
        texture_add_ref(e, Ptr::new(container));
        e.mem.set_u32(out.addr(), container);
        return D3D_OK;
    }
    if container != 0 {
        e.mem.set_u32(out.addr(), 0);
        return E_NOINTERFACE;
    }
    panic!("IDirect3DSurface9::GetContainer of a surface without a texture is not provided")
}

/// The block width (1, or 4 for the DXT formats) and the bytes of one block (one pixel for the
/// others) of a format; `None` for formats this layer doesn't size yet.
pub(crate) fn format_blocks(format: u32) -> Option<(u32, u32)> {
    const DXT1: u32 = u32::from_le_bytes(*b"DXT1");
    const DXT2: u32 = u32::from_le_bytes(*b"DXT2");
    const DXT3: u32 = u32::from_le_bytes(*b"DXT3");
    const DXT4: u32 = u32::from_le_bytes(*b"DXT4");
    const DXT5: u32 = u32::from_le_bytes(*b"DXT5");
    Some(match format {
        DXT1 => (4, 8),
        DXT2 | DXT3 | DXT4 | DXT5 => (4, 16),
        // R8G8B8.
        20 => (1, 3),
        // A8R8G8B8, X8R8G8B8, A2B10G10R10, A8B8G8R8, X8B8G8R8, G16R16, A2R10G10B10,
        // X8L8V8U8, Q8W8V8U8, V16U16, A2W10V10U10, G16R16F, R32F, D32, D24S8, D24X8, D24X4S4.
        21 | 22 | 31 | 32 | 33 | 34 | 35 | 62 | 63 | 64 | 67 | 112 | 114 | 71 | 75 | 77 | 79 => {
            (1, 4)
        }
        // R5G6B5, X1R5G5B5, A1R5G5B5, A4R4G4B4, A8R3G3B2, X4R4G4B4, A8P8, A8L8, V8U8, L6V5U5,
        // L16, R16F, D16, D15S1.
        23 | 24 | 25 | 26 | 29 | 30 | 40 | 51 | 60 | 61 | 81 | 111 | 80 | 73 => (1, 2),
        // R3G3B2, A8, P8, L8, A4L4.
        27 | 28 | 41 | 50 | 52 => (1, 1),
        // A16B16G16R16, A16B16G16R16F, G32R32F.
        36 | 113 | 115 => (1, 8),
        // A32B32G32R32F.
        116 => (1, 16),
        _ => return None,
    })
}

/// A level's pitch (bytes per row of blocks) and size, for a format [`format_blocks`] sizes.
fn level_layout(format: u32, width: u32, height: u32) -> (u32, u32) {
    let Some((block, bytes)) = format_blocks(format) else {
        panic!("IDirect3DDevice9: the memory layout of format {format:#x} is not provided");
    };
    let pitch = width.div_ceil(block) * bytes;
    (pitch, pitch * height.div_ceil(block))
}

/// A level of the texture `texture` described by `desc` (a `D3DSURFACE_DESC`): a surface of
/// `device` whose references are the texture's; a `lockable` one gets its pitch and pixels
/// (zero), kept outside the game's address space ([`Pixels`]).
fn new_level(e: &mut Engine, device: u32, texture: u32, desc: [u32; 8], lockable: bool) -> u32 {
    let surface = new_surface(e, device, false, desc);
    let pitch = if lockable {
        level_layout(desc[0], desc[6], desc[7]).0
    } else {
        0
    };
    let s = surface_mut(e, surface);
    s.container = texture;
    s.bits = 0;
    s.pitch = pitch;
    if lockable {
        s.pixels = Some(Pixels::Host(Vec::new()));
    }
    surface
}

/// The surface at `surface`.
fn surface_mut(e: &mut Engine, surface: u32) -> &mut Surface {
    device!(e)
        .device_objects
        .surfaces
        .get_mut(&surface)
        .expect("a surface of this layer")
}

/// A managed or system-memory texture's levels kept in host memory, still all zero, whose real
/// levels exist (`NV_D3D9=real` made them): from now on the real levels keep their pixels.
fn pixels_to_real_levels(e: &mut Engine, pool: u32, levels: &[u32]) {
    if pool != D3DPOOL_MANAGED && pool != D3DPOOL_SYSTEMMEM {
        return;
    }
    for &level in levels {
        if !super::d3d9real::has_surface(e, level) {
            continue;
        }
        let s = surface_mut(e, level);
        if matches!(&s.pixels, Some(Pixels::Host(v)) if v.is_empty()) && !s.locked {
            s.pixels = Some(Pixels::Real { written: false });
        }
    }
}

/// A texture of this layer.
#[derive(Debug, Clone)]
struct Texture {
    /// The program's references (what `AddRef` and `Release` answer).
    references: u32,
    /// The device's samplers holding it: apart from the program's count, as the retail
    /// runtime keeps them (`d3d9-capture device refs`, 2026-10-27).
    bound: u32,
    device: u32,
    pool: u32,
    /// 1, or 6 for a cube texture.
    faces: u32,
    /// The surfaces, face by face (each face's chain largest first).
    levels: Vec<u32>,
    lod: u32,
    priority: u32,
}

fn texture(e: &mut Engine, this: Ptr) -> &mut Texture {
    device!(e)
        .device_objects
        .textures
        .get_mut(&this.addr())
        .expect("an IDirect3DTexture9 of this layer")
}

/// `D3DPOOL_DEFAULT`, `D3DPOOL_MANAGED`; the usages `D3DUSAGE_DYNAMIC`, `D3DUSAGE_AUTOGENMIPMAP`;
/// `D3DRTYPE_TEXTURE`; the texture caps `D3DPTEXTURECAPS_POW2` (`D3DCAPS9::TextureCaps` +0x3C).
const D3DPOOL_DEFAULT: u32 = 0;
const D3DPOOL_MANAGED: u32 = 1;
const D3DUSAGE_DYNAMIC: u32 = 0x200;
const D3DUSAGE_AUTOGENMIPMAP: u32 = 0x400;
const D3DRTYPE_TEXTURE: u32 = 3;
const CAPS_TEXTURE_CAPS: u32 = 0x3c;
const D3DPTEXTURECAPS_POW2: u32 = 0x2;

// Platform: replaces D3D9.DLL IDirect3DDevice9::CreateTexture (vtable +0x5C)
/// `CreateTexture(width, height, levels, usage, format, pool, out, shared handle)`. A zero
/// size, a null out, a shared handle (Direct3D 9Ex only), more levels than the full chain, a
/// render-target or depth-stencil usage outside `D3DPOOL_DEFAULT`, `D3DUSAGE_DYNAMIC` in
/// `D3DPOOL_MANAGED`, or a format the table's `CheckDeviceFormat` (the device's adapter and
/// type, the display format, the usage, `D3DRTYPE_TEXTURE`) refuses answer
/// `D3DERR_INVALIDCALL`. Levels 0 is the full chain down to 1 by 1 (sides halved, rounding
/// down, at least 1). Each level is a surface of the texture (`GetSurfaceLevel`, counted with
/// the texture); levels the program can lock (any pool but `D3DPOOL_DEFAULT`, or dynamic) get
/// zeroed pixels, rows packed tightly (the pitch is the row's bytes: a value Direct3D leaves to
/// the runtime; the game copies by it), kept outside the game's address space between locks
/// ([`Pixels`]). Not provided (the run stops):
/// `D3DUSAGE_AUTOGENMIPMAP`, a side that is not a power of two on a device whose caps require
/// them, and formats [`format_blocks`] doesn't size.
#[allow(clippy::too_many_arguments)]
fn create_texture(
    e: &mut Engine,
    this: Ptr,
    width: u32,
    height: u32,
    levels: u32,
    usage: u32,
    format: u32,
    pool: u32,
    out: Ptr,
    shared: u32,
) -> u32 {
    if width == 0 || height == 0 || out.is_null() || shared != 0 {
        return D3DERR_INVALIDCALL;
    }
    if usage & D3DUSAGE_AUTOGENMIPMAP != 0 {
        panic!("IDirect3DDevice9::CreateTexture: D3DUSAGE_AUTOGENMIPMAP is not provided");
    }
    let full = 32 - width.max(height).leading_zeros();
    if levels > full {
        return D3DERR_INVALIDCALL;
    }
    if usage & (D3DUSAGE_RENDERTARGET | D3DUSAGE_DEPTHSTENCIL) != 0 && pool != D3DPOOL_DEFAULT {
        return D3DERR_INVALIDCALL;
    }
    if usage & D3DUSAGE_DYNAMIC != 0 && pool == D3DPOOL_MANAGED {
        return D3DERR_INVALIDCALL;
    }
    let d = device(e, this);
    let (direct3d, [adapter, device_type, ..]) = (d.direct3d, d.creation);
    let display_format = e.with_stack(0x10, |e, mode| {
        let hr = e.vcall(direct3d, 4 * 8, &[adapter, mode.addr()]).u32();
        assert_eq!(hr, D3D_OK, "GetAdapterDisplayMode for CreateTexture");
        e.mem.u32(mode.addr() + 0xc)
    });
    let check = e
        .vcall(
            direct3d,
            4 * 10,
            &[
                adapter,
                device_type,
                display_format,
                usage,
                D3DRTYPE_TEXTURE,
                format,
            ],
        )
        .u32();
    if check != D3D_OK {
        return D3DERR_INVALIDCALL;
    }
    if !(width.is_power_of_two() && height.is_power_of_two()) {
        let caps = e.with_stack(0x130, |e, caps| {
            assert_eq!(get_device_caps(e, this, caps), D3D_OK, "the device's caps");
            e.mem.u32(caps.addr() + CAPS_TEXTURE_CAPS)
        });
        if caps & D3DPTEXTURECAPS_POW2 != 0 {
            panic!(
                "IDirect3DDevice9::CreateTexture: a {width}x{height} texture on a device that \
                 wants powers of two is not provided"
            );
        }
    }
    let count = if levels == 0 { full } else { levels };
    let lockable = pool != D3DPOOL_DEFAULT || usage & D3DUSAGE_DYNAMIC != 0;
    let object = e.mem.alloc(8);
    let table = device!(e).device_objects.texture_vtable;
    e.mem.set_u32(object, table);
    let device_addr = this.addr();
    let surfaces: Vec<u32> = (0..count)
        .map(|level| {
            let (w, h) = ((width >> level).max(1), (height >> level).max(1));
            let desc = [format, D3DRTYPE_SURFACE, usage, pool, 0, 0, w, h];
            new_level(e, device_addr, object, desc, lockable)
        })
        .collect();
    device!(e).device_objects.textures.insert(
        object,
        Texture {
            references: 1,
            bound: 0,
            device: device_addr,
            pool,
            faces: 1,
            levels: surfaces,
            lod: 0,
            priority: 0,
        },
    );
    device_add_ref(e, this);
    note_reference(e, Ptr::new(object), "Create", 1);
    e.mem.set_u32(out.addr(), object);
    D3D_OK
}

// Platform: replaces D3D9.DLL IDirect3DDevice9::CreateCubeTexture (vtable +0x64)
/// `CreateCubeTexture(edge, levels, usage, format, pool, out, shared handle)`: as
/// [`create_texture`] for six square faces (`D3DRTYPE_CUBETEXTURE` in the table's
/// `CheckDeviceFormat`), each face with its own chain of surfaces (counted with the cube
/// texture). Not provided (the run stops): `D3DUSAGE_AUTOGENMIPMAP`, an edge that is not a
/// power of two on a device whose caps want powers of two for cube maps, and formats
/// [`format_blocks`] doesn't size.
#[allow(clippy::too_many_arguments)]
fn create_cube_texture(
    e: &mut Engine,
    this: Ptr,
    edge: u32,
    levels: u32,
    usage: u32,
    format: u32,
    pool: u32,
    out: Ptr,
    shared: u32,
) -> u32 {
    if edge == 0 || out.is_null() || shared != 0 {
        return D3DERR_INVALIDCALL;
    }
    if usage & D3DUSAGE_AUTOGENMIPMAP != 0 {
        panic!("IDirect3DDevice9::CreateCubeTexture: D3DUSAGE_AUTOGENMIPMAP is not provided");
    }
    let full = 32 - edge.leading_zeros();
    if levels > full {
        return D3DERR_INVALIDCALL;
    }
    if usage & (D3DUSAGE_RENDERTARGET | D3DUSAGE_DEPTHSTENCIL) != 0 && pool != D3DPOOL_DEFAULT {
        return D3DERR_INVALIDCALL;
    }
    if usage & D3DUSAGE_DYNAMIC != 0 && pool == D3DPOOL_MANAGED {
        return D3DERR_INVALIDCALL;
    }
    let d = device(e, this);
    let (direct3d, [adapter, device_type, ..]) = (d.direct3d, d.creation);
    let display_format = e.with_stack(0x10, |e, mode| {
        let hr = e.vcall(direct3d, 4 * 8, &[adapter, mode.addr()]).u32();
        assert_eq!(hr, D3D_OK, "GetAdapterDisplayMode for CreateCubeTexture");
        e.mem.u32(mode.addr() + 0xc)
    });
    let check = e
        .vcall(
            direct3d,
            4 * 10,
            &[
                adapter,
                device_type,
                display_format,
                usage,
                D3DRTYPE_CUBETEXTURE,
                format,
            ],
        )
        .u32();
    if check != D3D_OK {
        return D3DERR_INVALIDCALL;
    }
    if !edge.is_power_of_two() {
        let caps = e.with_stack(0x130, |e, caps| {
            assert_eq!(get_device_caps(e, this, caps), D3D_OK, "the device's caps");
            e.mem.u32(caps.addr() + CAPS_TEXTURE_CAPS)
        });
        if caps & D3DPTEXTURECAPS_CUBEMAP_POW2 != 0 {
            panic!("IDirect3DDevice9::CreateCubeTexture: an edge of {edge} on a device that wants powers of two is not provided");
        }
    }
    let count = if levels == 0 { full } else { levels };
    let lockable = pool != D3DPOOL_DEFAULT || usage & D3DUSAGE_DYNAMIC != 0;
    let object = e.mem.alloc(8);
    let table = device!(e).device_objects.cube_vtable;
    e.mem.set_u32(object, table);
    let device_addr = this.addr();
    let mut surfaces = Vec::new();
    for _face in 0..6 {
        for level in 0..count {
            let side = (edge >> level).max(1);
            let desc = [format, D3DRTYPE_SURFACE, usage, pool, 0, 0, side, side];
            surfaces.push(new_level(e, device_addr, object, desc, lockable));
        }
    }
    device!(e).device_objects.textures.insert(
        object,
        Texture {
            references: 1,
            bound: 0,
            device: device_addr,
            pool,
            faces: 6,
            levels: surfaces,
            lod: 0,
            priority: 0,
        },
    );
    device_add_ref(e, this);
    note_reference(e, Ptr::new(object), "CreateCube", 1);
    e.mem.set_u32(out.addr(), object);
    D3D_OK
}

/// Whether a surface of `format` may have `multisample` samples at `quality`: `D3DMULTISAMPLE_NONE`
/// (0) always; otherwise the device's `IDirect3D9::CheckDeviceMultiSampleType` (its adapter and
/// type, windowed as the device is) answers `D3D_OK` with more quality levels than `quality`.
fn multisample_supported(
    e: &mut Engine,
    this: Ptr,
    format: u32,
    multisample: u32,
    quality: u32,
) -> bool {
    if multisample == 0 {
        return true;
    }
    let d = device(e, this);
    let (direct3d, [adapter, device_type, ..]) = (d.direct3d, d.creation);
    let windowed = u32::from_le_bytes(
        d.present[PP_WINDOWED as usize..PP_WINDOWED as usize + 4]
            .try_into()
            .unwrap(),
    );
    e.with_stack(4, |e, levels| {
        let hr = e
            .vcall(
                direct3d,
                4 * 11,
                &[
                    adapter,
                    device_type,
                    format,
                    windowed,
                    multisample,
                    levels.addr(),
                ],
            )
            .u32();
        hr == D3D_OK && quality < e.mem.u32(levels.addr())
    })
}

// Platform: replaces D3D9.DLL IDirect3DDevice9::CreateRenderTarget (vtable +0x70)
/// `CreateRenderTarget(width, height, format, multisample type, quality, lockable, out, shared
/// handle)`. A zero size, a null out, a shared handle (Direct3D 9Ex only), a format the table's
/// `CheckDeviceFormat` (the device's adapter and type, the display format,
/// `D3DUSAGE_RENDERTARGET`, `D3DRTYPE_SURFACE`) refuses, or a multisample type and quality its
/// `CheckDeviceMultiSampleType` refuses answer `D3DERR_INVALIDCALL`. The surface
/// (`D3DPOOL_DEFAULT`, usage `D3DUSAGE_RENDERTARGET`, reference count 1) holds its device until
/// released; a lockable one gets memory in the game's address space (rows packed). Not
/// provided (the run stops): a lockable multisampled target (Direct3D refuses it) and formats
/// [`format_blocks`] doesn't size.
#[allow(clippy::too_many_arguments)]
fn create_render_target(
    e: &mut Engine,
    this: Ptr,
    width: u32,
    height: u32,
    format: u32,
    multisample: u32,
    quality: u32,
    lockable: u32,
    out: Ptr,
    shared: u32,
) -> u32 {
    if width == 0 || height == 0 || out.is_null() || shared != 0 {
        return D3DERR_INVALIDCALL;
    }
    let d = device(e, this);
    let (direct3d, [adapter, device_type, ..]) = (d.direct3d, d.creation);
    let display_format = e.with_stack(0x10, |e, mode| {
        let hr = e.vcall(direct3d, 4 * 8, &[adapter, mode.addr()]).u32();
        assert_eq!(hr, D3D_OK, "GetAdapterDisplayMode for CreateRenderTarget");
        e.mem.u32(mode.addr() + 0xc)
    });
    let check = e
        .vcall(
            direct3d,
            4 * 10,
            &[
                adapter,
                device_type,
                display_format,
                D3DUSAGE_RENDERTARGET,
                D3DRTYPE_SURFACE,
                format,
            ],
        )
        .u32();
    if check != D3D_OK || !multisample_supported(e, this, format, multisample, quality) {
        return D3DERR_INVALIDCALL;
    }
    if lockable != 0 && multisample != 0 {
        panic!(
            "IDirect3DDevice9::CreateRenderTarget: a lockable multisampled target is not provided"
        );
    }
    let desc = [
        format,
        D3DRTYPE_SURFACE,
        D3DUSAGE_RENDERTARGET,
        D3DPOOL_DEFAULT,
        multisample,
        quality,
        width,
        height,
    ];
    let surface = new_surface(e, this.addr(), false, desc);
    if lockable != 0 {
        let (pitch, size) = level_layout(format, width, height);
        let bits = e.mem.alloc(size);
        let s = device!(e)
            .device_objects
            .surfaces
            .get_mut(&surface)
            .expect("the target");
        s.bits = bits;
        s.pitch = pitch;
    }
    device_add_ref(e, this);
    e.mem.set_u32(out.addr(), surface);
    D3D_OK
}

// Platform: replaces D3D9.DLL IDirect3DDevice9::CreateDepthStencilSurface (vtable +0x74)
/// `CreateDepthStencilSurface(width, height, format, multisample type, quality, discard, out,
/// shared handle)`. A zero size, a null out, a shared handle (Direct3D 9Ex only) or a format
/// the table's `CheckDeviceFormat` (the device's adapter and type, the display format,
/// `D3DUSAGE_DEPTHSTENCIL`, `D3DRTYPE_SURFACE`) refuses answer `D3DERR_INVALIDCALL`. The
/// surface (`D3DPOOL_DEFAULT`, usage `D3DUSAGE_DEPTHSTENCIL`, reference count 1) holds its
/// device until released; it has no memory the program can lock (the depth formats the game
/// uses are not lockable), so `LockRect` answers `D3DERR_INVALIDCALL`. A multisample type and
/// quality the table's `CheckDeviceMultiSampleType` refuses answer `D3DERR_INVALIDCALL`.
#[allow(clippy::too_many_arguments)]
fn create_depth_stencil_surface(
    e: &mut Engine,
    this: Ptr,
    width: u32,
    height: u32,
    format: u32,
    multisample: u32,
    quality: u32,
    _discard: u32,
    out: Ptr,
    shared: u32,
) -> u32 {
    if width == 0 || height == 0 || out.is_null() || shared != 0 {
        return D3DERR_INVALIDCALL;
    }
    if !multisample_supported(e, this, format, multisample, quality) {
        return D3DERR_INVALIDCALL;
    }
    let d = device(e, this);
    let (direct3d, [adapter, device_type, ..]) = (d.direct3d, d.creation);
    let display_format = e.with_stack(0x10, |e, mode| {
        let hr = e.vcall(direct3d, 4 * 8, &[adapter, mode.addr()]).u32();
        assert_eq!(
            hr, D3D_OK,
            "GetAdapterDisplayMode for CreateDepthStencilSurface"
        );
        e.mem.u32(mode.addr() + 0xc)
    });
    let check = e
        .vcall(
            direct3d,
            4 * 10,
            &[
                adapter,
                device_type,
                display_format,
                D3DUSAGE_DEPTHSTENCIL,
                D3DRTYPE_SURFACE,
                format,
            ],
        )
        .u32();
    if check != D3D_OK {
        return D3DERR_INVALIDCALL;
    }
    let desc = [
        format,
        D3DRTYPE_SURFACE,
        D3DUSAGE_DEPTHSTENCIL,
        D3DPOOL_DEFAULT,
        multisample,
        quality,
        width,
        height,
    ];
    let surface = new_surface(e, this.addr(), false, desc);
    device_add_ref(e, this);
    e.mem.set_u32(out.addr(), surface);
    D3D_OK
}

// Platform: replaces D3D9.DLL IDirect3DDevice9::CreateOffscreenPlainSurface (vtable +0x90)
/// `CreateOffscreenPlainSurface(width, height, format, pool, out, shared handle)`, as the retail
/// runtime answered (`d3d9-capture device extra`, 2026-10-27): a zero side, a null out, a shared
/// handle, `D3DPOOL_MANAGED` or a format the table's `CheckDeviceFormat` (usage 0,
/// `D3DRTYPE_SURFACE`) refuses answer `D3DERR_INVALIDCALL`; in the other pools (default, system
/// memory, scratch) the surface has memory the program can lock (rows packed tightly), and holds
/// the device.
#[allow(clippy::too_many_arguments)]
fn create_offscreen_plain_surface(
    e: &mut Engine,
    this: Ptr,
    width: u32,
    height: u32,
    format: u32,
    pool: u32,
    out: Ptr,
    shared: u32,
) -> u32 {
    if width == 0 || height == 0 || out.is_null() || shared != 0 || pool == D3DPOOL_MANAGED {
        return D3DERR_INVALIDCALL;
    }
    let d = device(e, this);
    let (direct3d, [adapter, device_type, ..]) = (d.direct3d, d.creation);
    let display_format = e.with_stack(0x10, |e, mode| {
        let hr = e.vcall(direct3d, 4 * 8, &[adapter, mode.addr()]).u32();
        assert_eq!(
            hr, D3D_OK,
            "GetAdapterDisplayMode for CreateOffscreenPlainSurface"
        );
        e.mem.u32(mode.addr() + 0xc)
    });
    let check = e
        .vcall(
            direct3d,
            4 * 10,
            &[
                adapter,
                device_type,
                display_format,
                0,
                D3DRTYPE_SURFACE,
                format,
            ],
        )
        .u32();
    if check != D3D_OK {
        return D3DERR_INVALIDCALL;
    }
    let desc = [format, D3DRTYPE_SURFACE, 0, pool, 0, 0, width, height];
    let surface = new_surface(e, this.addr(), false, desc);
    let (pitch, size) = level_layout(format, width, height);
    let bits = e.mem.alloc(size);
    let s = device!(e)
        .device_objects
        .surfaces
        .get_mut(&surface)
        .expect("the surface");
    s.bits = bits;
    s.pitch = pitch;
    device_add_ref(e, this);
    super::d3d9real::offscreen_created(e, this.addr(), surface, width, height, format, pool);
    e.mem.set_u32(out.addr(), surface);
    D3D_OK
}

/// A rectangle (`RECT`) inside a `width` by `height` surface, or the whole surface for null.
fn rect_inside(e: &Engine, rect: Ptr, width: u32, height: u32) -> Option<[u32; 4]> {
    if rect.is_null() {
        return Some([0, 0, width, height]);
    }
    let r: [i32; 4] = std::array::from_fn(|i| e.mem.i32(rect.addr() + 4 * i as u32));
    let inside = r[0] >= 0
        && r[1] >= 0
        && r[0] <= r[2]
        && r[1] <= r[3]
        && r[2] as u32 <= width
        && r[3] as u32 <= height;
    inside.then(|| r.map(|v| v as u32))
}

// Platform: replaces D3D9.DLL IDirect3DDevice9::ColorFill (vtable +0x8C)
/// `ColorFill(surface, rect, colour)`, as the retail runtime answered (2026-10-27): a surface
/// outside `D3DPOOL_DEFAULT` or a rectangle past its edges answers `D3DERR_INVALIDCALL`; in a
/// scene too it answers `D3D_OK`. The surface's memory, when it has some here, takes the colour
/// as the runtime wrote it: `X8R8G8B8` with the unused byte 0, `A8R8G8B8` the colour; other
/// formats with memory are not provided. A render target's pixels are the real device's
/// (`NV_D3D9=real`).
fn color_fill(e: &mut Engine, this: Ptr, surface: u32, rect: Ptr, colour: u32) -> u32 {
    let Some(s) = device!(e).device_objects.surfaces.get(&surface).cloned() else {
        return D3DERR_INVALIDCALL;
    };
    if s.desc[3] != D3DPOOL_DEFAULT {
        return D3DERR_INVALIDCALL;
    }
    let Some([left, top, right, bottom]) = rect_inside(e, rect, s.desc[6], s.desc[7]) else {
        return D3DERR_INVALIDCALL;
    };
    if s.bits != 0 || s.pixels.is_some() {
        let word = match s.desc[0] {
            22 => colour & 0x00ff_ffff,
            21 => colour,
            f => panic!("IDirect3DDevice9::ColorFill of a surface of format {f} is not provided"),
        };
        // A level kept outside the game's memory that is locked has its bytes in the lock's
        // staging block.
        let bits = if s.bits != 0 { s.bits } else { s.staging };
        if bits != 0 {
            for y in top..bottom {
                for x in left..right {
                    e.mem.set_u32(bits + y * s.pitch + 4 * x, word);
                }
            }
        } else {
            // A dynamic texture's level kept in host memory (the default pool is never the
            // real level's to keep).
            let size = level_size(&s) as usize;
            let level = surface_mut(e, surface);
            let Some(Pixels::Host(bytes)) = &mut level.pixels else {
                unreachable!("a default-pool level keeps its pixels in host memory")
            };
            bytes.resize(size, 0);
            for y in top..bottom {
                for x in left..right {
                    let at = (y * s.pitch + 4 * x) as usize;
                    bytes[at..at + 4].copy_from_slice(&word.to_le_bytes());
                }
            }
        }
    }
    super::d3d9real::color_fill(e, this.addr(), surface, rect.addr(), colour, D3D_OK);
    let _ = this;
    D3D_OK
}

// Platform: replaces D3D9.DLL IDirect3DDevice9::GetRenderTargetData (vtable +0x80)
/// `GetRenderTargetData(render target, destination)`, as the retail runtime answered
/// (2026-10-27): the source must be a render target in `D3DPOOL_DEFAULT`, the destination a
/// `D3DPOOL_SYSTEMMEM` surface of the same size and format, else `D3DERR_INVALIDCALL`; in a
/// scene too it answers `D3D_OK`. The pixels are the real device's: with `NV_D3D9=real` they are
/// read back into the destination's memory; without it nothing is drawn and the copy is not
/// provided (the run stops).
fn get_render_target_data(e: &mut Engine, this: Ptr, source: u32, destination: u32) -> u32 {
    let surfaces = &device!(e).device_objects.surfaces;
    let (Some(from), Some(to)) = (
        surfaces.get(&source).cloned(),
        surfaces.get(&destination).cloned(),
    ) else {
        return D3DERR_INVALIDCALL;
    };
    let ok = from.desc[2] & D3DUSAGE_RENDERTARGET != 0
        && from.desc[3] == D3DPOOL_DEFAULT
        && to.desc[3] == D3DPOOL_SYSTEMMEM
        && (from.desc[0], from.desc[6], from.desc[7]) == (to.desc[0], to.desc[6], to.desc[7]);
    if !ok {
        return D3DERR_INVALIDCALL;
    }
    if super::d3d9real::get_render_target_data(
        e,
        this.addr(),
        source,
        destination,
        to.bits,
        to.pitch,
    ) {
        // A texture level kept outside the game's memory takes the real level's bytes.
        match &to.pixels {
            Some(Pixels::Real { .. }) => {
                surface_mut(e, destination).pixels = Some(Pixels::Real { written: true });
            }
            Some(Pixels::Host(_)) => {
                let size = level_size(&to);
                let block = e.mem.alloc(size);
                let read = super::d3d9real::read_surface(e, destination, block, size);
                assert!(read, "GetRenderTargetData: the real level read back");
                let bytes = e.mem.bytes(block, size);
                e.mem.free(block);
                surface_mut(e, destination).pixels = Some(Pixels::Host(bytes));
            }
            None => {}
        }
        return D3D_OK;
    }
    let _ = this;
    panic!(
        "IDirect3DDevice9::GetRenderTargetData without NV_D3D9=real (nothing is drawn, so no \
         pixels to read back) is not provided"
    )
}

/// Whether the device has a resource of its own in `D3DPOOL_DEFAULT` alive (a texture, a
/// surface it did not make itself, a buffer). Under the device lock: its one caller is `Reset`,
/// a device function.
fn default_pool_alive(e: &Engine, device: u32) -> bool {
    let o = &device_ref!(e).device_objects;
    o.textures
        .values()
        .any(|t| t.device == device && t.pool == D3DPOOL_DEFAULT)
        || o.surfaces.values().any(|s| {
            s.device == device && !s.implicit && s.container == 0 && s.desc[3] == D3DPOOL_DEFAULT
        })
        || super::d3d9resource::default_pool_alive(e, device)
}

// Platform: replaces D3D9.DLL IDirect3DDevice9::Reset (vtable +0x40)
/// `Reset(present parameters)`, as the retail runtime answered on a windowed device
/// (`d3d9-capture device reset`, 2026-10-27):
/// - with a resource of the device's own in `D3DPOOL_DEFAULT` alive, or more than three back
///   buffers (the count left as given), it answers `D3DERR_INVALIDCALL` and the device needs a
///   successful `Reset`: `TestCooperativeLevel` answers `D3DERR_DEVICENOTRESET`, `BeginScene`,
///   `EndScene` and `Present` are refused, also after the resource is released;
/// - else, in a scene too (which it ends), it answers `D3D_OK` with a count of 0 made 1 and
///   `D3DFMT_UNKNOWN` the display format (written back); the old back buffers and automatic
///   depth surface go (a reference the program still holds no longer names a live surface) and
///   new ones of the parameters are made, render target 0 the first back buffer, the depth
///   surface the automatic one; every state the program set returns to its default (render,
///   texture stage and sampler states, transforms, clip planes, material and lights, textures,
///   streams, indices, declaration, FVF, shaders, constants), the viewport and scissor the whole
///   back buffer.
///
/// Not provided (the run stops): a full-screen device, a size taken from the window (0),
/// multisampling.
fn reset(e: &mut Engine, this: Ptr, present: Ptr) -> u32 {
    // The drawing thread's cut is joined before a reset (piece 8 waits for the job first): the
    // drawing side's copy holds the surfaces a reset frees. One still open is joined here.
    if device!(e).device_objects.draw.device.is_some() && !super::d3d9draw::drawing_thread() {
        static NOTED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
        if !NOTED.swap(true, std::sync::atomic::Ordering::Relaxed) {
            eprintln!(
                "NV_DRAW_THREAD: IDirect3DDevice9::Reset while a drawing cut is open: joined first"
            );
        }
        super::d3d9draw::draw_finish(e);
    }
    if present.is_null() {
        return D3DERR_INVALIDCALL;
    }
    let p = present.addr();
    let field = |e: &Engine, offset: u32| e.mem.u32(p + offset);
    if field(e, PP_WINDOWED) == 0 {
        panic!("IDirect3DDevice9::Reset to a full-screen device is not provided");
    }
    if default_pool_alive(e, this.addr()) || field(e, PP_COUNT) > BACK_BUFFERS_MAX {
        device(e, this).lost = true;
        return D3DERR_INVALIDCALL;
    }
    if field(e, PP_WIDTH) == 0 || field(e, PP_HEIGHT) == 0 {
        panic!(
            "IDirect3DDevice9::Reset with a back buffer size taken from the window is not provided"
        );
    }
    if field(e, PP_MULTISAMPLE) != 0 {
        panic!("IDirect3DDevice9::Reset with multisampling is not provided");
    }
    let (direct3d, adapter) = {
        let d = device(e, this);
        (d.direct3d, d.creation[0])
    };
    let display_format = e.with_stack(0x10, |e, mode| {
        let hr = e.vcall(direct3d, 4 * 8, &[adapter, mode.addr()]).u32();
        assert_eq!(hr, D3D_OK, "GetAdapterDisplayMode for Reset");
        e.mem.u32(mode.addr() + 0xc)
    });
    if field(e, PP_COUNT) == 0 {
        e.mem.set_u32(p + PP_COUNT, 1);
    }
    if field(e, PP_FORMAT) == D3DFMT_UNKNOWN {
        e.mem.set_u32(p + PP_FORMAT, display_format);
    }
    // The device's bindings and states go back to their defaults.
    let old = {
        let d = device(e, this);
        let held: Vec<u32> = d
            .render_targets
            .iter()
            .chain([&d.depth_stencil])
            .copied()
            .filter(|&s| s != 0)
            .collect();
        let textures: Vec<u32> = d.textures.values().copied().collect();
        d.render_targets = [0; RENDER_TARGETS];
        d.depth_stencil = 0;
        d.textures.clear();
        d.render_states.clear();
        d.stage_states.clear();
        d.sampler_states.clear();
        d.transforms.clear();
        d.clip_planes.clear();
        d.material = None;
        d.lights.clear();
        d.enabled_lights.clear();
        d.in_scene = false;
        (held, textures)
    };
    for surface in old.0 {
        surface_release(e, Ptr::new(surface));
    }
    for texture in old.1 {
        texture_unbind(e, Ptr::new(texture));
    }
    super::d3d9resource::release_bindings(e, this.addr());
    let implicit: Vec<u32> = device!(e)
        .device_objects
        .surfaces
        .iter()
        .filter(|(_, s)| s.device == this.addr() && s.implicit)
        .map(|(a, _)| *a)
        .collect();
    for surface in implicit {
        device!(e).device_objects.surfaces.remove(&surface);
        super::d3d9real::forget(e, surface);
        e.mem.free(surface);
    }
    // The new swap chain.
    let format = field(e, PP_FORMAT);
    let (width, height, quality) = (
        field(e, PP_WIDTH),
        field(e, PP_HEIGHT),
        field(e, PP_QUALITY),
    );
    let back_buffers: Vec<u32> = (0..field(e, PP_COUNT))
        .map(|_| {
            let desc = [
                format,
                D3DRTYPE_SURFACE,
                D3DUSAGE_RENDERTARGET,
                0,
                0,
                quality,
                width,
                height,
            ];
            new_surface(e, this.addr(), true, desc)
        })
        .collect();
    let depth_stencil = if field(e, PP_AUTO_DEPTH) != 0 {
        let desc = [
            field(e, PP_DEPTH_FORMAT),
            D3DRTYPE_SURFACE,
            D3DUSAGE_DEPTHSTENCIL,
            0,
            0,
            quality,
            width,
            height,
        ];
        new_surface(e, this.addr(), true, desc)
    } else {
        0
    };
    surface_add_ref(e, Ptr::new(back_buffers[0]));
    if depth_stencil != 0 {
        surface_add_ref(e, Ptr::new(depth_stencil));
    }
    let bytes = e.mem.bytes(p, PRESENT_SIZE);
    {
        let d = device(e, this);
        d.present = bytes.clone();
        d.back_buffers = back_buffers.clone();
        d.render_targets[0] = back_buffers[0];
        d.depth_stencil = depth_stencil;
        d.viewport = [0, 0, width, height, 0f32.to_bits(), 1f32.to_bits()];
        d.scissor = [0, 0, width, height];
        d.lost = false;
    }
    super::d3d9real::device_reset(e, this.addr(), &bytes, &back_buffers, depth_stencil);
    D3D_OK
}

/// `QueryInterface(iid, out)`: `IUnknown`, `IDirect3DResource9`, `IDirect3DBaseTexture9` and
/// `IDirect3DTexture9` answer the texture (counted); others `E_NOINTERFACE` with a null out.
fn texture_query_interface(e: &mut Engine, this: Ptr, iid: Ptr, out: Ptr) -> u32 {
    if out.is_null() {
        return D3DERR_INVALIDCALL;
    }
    if iid_is(
        e,
        iid,
        &[IID_IUNKNOWN, IID_RESOURCE9, IID_BASETEXTURE9, IID_TEXTURE9],
    ) {
        texture_add_ref(e, this);
        e.mem.set_u32(out.addr(), this.addr());
        return D3D_OK;
    }
    e.mem.set_u32(out.addr(), 0);
    E_NOINTERFACE
}

fn texture_add_ref(e: &mut Engine, this: Ptr) -> u32 {
    let t = texture(e, this);
    t.references += 1;
    let left = t.references;
    note_reference(e, this, "AddRef", left);
    left
}

/// `NV_D3D9_REFS=FILE` (a diagnostic, not Direct3D behaviour): every count change of a texture is
/// appended to `FILE` with the count after it and the exe functions it came from (innermost
/// first), to find a reference given back once too often. Off by default.
fn note_reference(e: &Engine, this: Ptr, what: &str, left: u32) {
    static FILE: std::sync::OnceLock<Option<std::sync::Mutex<std::fs::File>>> =
        std::sync::OnceLock::new();
    let Some(file) = FILE.get_or_init(|| {
        let path = std::env::var_os("NV_D3D9_REFS")?;
        std::fs::File::create(path).ok().map(std::sync::Mutex::new)
    }) else {
        return;
    };
    let calls: Vec<String> = e
        .call_stack
        .iter()
        .rev()
        .take(14)
        .map(|a| format!("{a:08x}"))
        .collect();
    if let Ok(mut f) = file.lock() {
        use std::io::Write;
        let _ = writeln!(f, "{:08x}	{what}	{left}	{}", this.addr(), calls.join(" < "));
    }
}

/// `Release()`: the last reference frees the texture, its levels and their memory, and
/// releases the device's reference the texture held.
fn texture_release(e: &mut Engine, this: Ptr) -> u32 {
    let t = texture(e, this);
    t.references -= 1;
    let (left, bound) = (t.references, t.bound);
    note_reference(e, this, "Release", left);
    if left == 0 && bound == 0 {
        texture_destroy(e, this);
    }
    left
}

/// A sampler lets go of `this`; the texture goes once neither the program nor a sampler holds it.
fn texture_unbind(e: &mut Engine, this: Ptr) {
    let t = texture(e, this);
    t.bound -= 1;
    if t.bound == 0 && t.references == 0 {
        texture_destroy(e, this);
    }
}

fn texture_destroy(e: &mut Engine, this: Ptr) {
    // On the drawing side of an open cut the texture waits for the join (`super::d3d9draw`).
    let objects = &mut device!(e).device_objects;
    if super::d3d9draw::drawing_side(&objects.draw) {
        objects.draw.ending.push(this.addr());
        return;
    }
    note_reference(e, this, "destroyed", 0);
    {
        let t = device!(e)
            .device_objects
            .textures
            .remove(&this.addr())
            .expect("the texture");
        for level in t.levels {
            let s = device!(e)
                .device_objects
                .surfaces
                .remove(&level)
                .expect("the level");
            e.mem.free(s.bits);
            // A level destroyed while locked: its staging block goes with it.
            e.mem.free(s.staging);
            super::d3d9real::forget(e, level);
            e.mem.free(level);
        }
        super::d3d9real::forget(e, this.addr());
        e.mem.free(this.addr());
        device_release(e, Ptr::new(t.device));
    }
}

/// `GetDevice(out)`: the texture's device, counted.
fn texture_get_device(e: &mut Engine, this: Ptr, out: Ptr) -> u32 {
    if out.is_null() {
        return D3DERR_INVALIDCALL;
    }
    let owner = texture(e, this).device;
    device_add_ref(e, Ptr::new(owner));
    e.mem.set_u32(out.addr(), owner);
    D3D_OK
}

/// `SetPriority(priority)`: kept for a managed texture (others keep 0); answers the old one.
fn texture_set_priority(e: &mut Engine, this: Ptr, priority: u32) -> u32 {
    let t = texture(e, this);
    if t.pool != D3DPOOL_MANAGED {
        return 0;
    }
    std::mem::replace(&mut t.priority, priority)
}

/// `GetPriority()`.
fn texture_get_priority(e: &mut Engine, this: Ptr) -> u32 {
    texture(e, this).priority
}

/// `PreLoad()`: a hint about managed resources; nothing to do here.
fn texture_pre_load(_e: &mut Engine, _this: Ptr) {}

/// `GetType()`: `D3DRTYPE_TEXTURE`.
fn texture_get_type(_e: &mut Engine, _this: Ptr) -> u32 {
    D3DRTYPE_TEXTURE
}

/// `SetLOD(level)`: for a managed texture the most detailed level kept (at most the last);
/// answers the old one. Other pools keep 0 and answer 0.
fn texture_set_lod(e: &mut Engine, this: Ptr, lod: u32) -> u32 {
    let t = texture(e, this);
    if t.pool != D3DPOOL_MANAGED {
        return 0;
    }
    let last = t.levels.len() as u32 / t.faces - 1;
    std::mem::replace(&mut t.lod, lod.min(last))
}

/// `SetLOD`, then (`NV_D3D9=real`) the same on the real texture, whose sampling starts at that
/// level too (a managed texture; the model's answer stands).
fn fwd_texture_set_lod(e: &mut Engine, this: Ptr, lod: u32) -> u32 {
    let previous = texture_set_lod(e, this, lod);
    if texture(e, this).pool == D3DPOOL_MANAGED {
        super::d3d9real::texture_set_lod(e, this.addr(), lod, previous);
    }
    previous
}

/// `GetLOD()`.
fn texture_get_lod(e: &mut Engine, this: Ptr) -> u32 {
    texture(e, this).lod
}

/// `GetLevelCount()`.
fn texture_get_level_count(e: &mut Engine, this: Ptr) -> u32 {
    let t = texture(e, this);
    t.levels.len() as u32 / t.faces
}

/// A level's surface (face 0's for a cube texture: its levels all have the same description).
fn texture_level(e: &mut Engine, this: Ptr, level: u32) -> Option<u32> {
    let t = texture(e, this);
    if level >= t.levels.len() as u32 / t.faces {
        return None;
    }
    t.levels.get(level as usize).copied()
}

/// A cube texture's surface for (face, level).
fn cube_level(e: &mut Engine, this: Ptr, face: u32, level: u32) -> Option<u32> {
    let t = texture(e, this);
    let count = t.levels.len() as u32 / t.faces;
    if face >= t.faces || level >= count {
        return None;
    }
    t.levels.get((face * count + level) as usize).copied()
}

/// `IID_IDirect3DCubeTexture9` (d3d9.h).
const IID_CUBETEXTURE9: [u8; 16] = guid(
    0xfff3_2f81,
    0xd953,
    0x473a,
    [0x92, 0x23, 0x93, 0xd6, 0x52, 0xab, 0xa9, 0x3f],
);
/// `D3DRTYPE_CUBETEXTURE`; the cube-map caps `D3DPTEXTURECAPS_CUBEMAP_POW2`.
const D3DRTYPE_CUBETEXTURE: u32 = 5;
const D3DPTEXTURECAPS_CUBEMAP_POW2: u32 = 0x2_0000;

/// `QueryInterface(iid, out)`: `IUnknown`, `IDirect3DResource9`, `IDirect3DBaseTexture9` and
/// `IDirect3DCubeTexture9` answer the cube texture (counted); others `E_NOINTERFACE`.
fn cube_query_interface(e: &mut Engine, this: Ptr, iid: Ptr, out: Ptr) -> u32 {
    if out.is_null() {
        return D3DERR_INVALIDCALL;
    }
    if iid_is(
        e,
        iid,
        &[
            IID_IUNKNOWN,
            IID_RESOURCE9,
            IID_BASETEXTURE9,
            IID_CUBETEXTURE9,
        ],
    ) {
        texture_add_ref(e, this);
        e.mem.set_u32(out.addr(), this.addr());
        return D3D_OK;
    }
    e.mem.set_u32(out.addr(), 0);
    E_NOINTERFACE
}

/// `GetType()`: `D3DRTYPE_CUBETEXTURE`.
fn cube_get_type(_e: &mut Engine, _this: Ptr) -> u32 {
    D3DRTYPE_CUBETEXTURE
}

/// `GetCubeMapSurface(face, level, out)`: that surface, counted (with the texture).
fn cube_get_surface(e: &mut Engine, this: Ptr, face: u32, level: u32, out: Ptr) -> u32 {
    match cube_level(e, this, face, level) {
        Some(surface) if !out.is_null() => {
            texture_add_ref(e, this);
            e.mem.set_u32(out.addr(), surface);
            D3D_OK
        }
        _ => D3DERR_INVALIDCALL,
    }
}

/// `LockRect(face, level, locked out, rectangle, flags)`: [`surface_lock_rect`] on it.
fn cube_lock_rect(
    e: &mut Engine,
    this: Ptr,
    face: u32,
    level: u32,
    locked: Ptr,
    rect: Ptr,
    flags: u32,
) -> u32 {
    match cube_level(e, this, face, level) {
        Some(surface) => surface_lock_rect(e, Ptr::new(surface), locked, rect, flags),
        None => D3DERR_INVALIDCALL,
    }
}

/// `UnlockRect(face, level)`: [`surface_unlock_rect`] on it.
fn cube_unlock_rect(e: &mut Engine, this: Ptr, face: u32, level: u32) -> u32 {
    match cube_level(e, this, face, level) {
        Some(surface) => surface_unlock_rect(e, Ptr::new(surface)),
        None => D3DERR_INVALIDCALL,
    }
}

/// `GetLevelDesc(level, out)`: that level's `D3DSURFACE_DESC`; a level past the count or a
/// null out answers `D3DERR_INVALIDCALL`.
fn texture_get_level_desc(e: &mut Engine, this: Ptr, level: u32, out: Ptr) -> u32 {
    match texture_level(e, this, level) {
        Some(surface) if !out.is_null() => surface_get_desc(e, Ptr::new(surface), out),
        _ => D3DERR_INVALIDCALL,
    }
}

/// `GetSurfaceLevel(level, out)`: that level's surface, counted (with the texture).
fn texture_get_surface_level(e: &mut Engine, this: Ptr, level: u32, out: Ptr) -> u32 {
    match texture_level(e, this, level) {
        Some(surface) if !out.is_null() => {
            texture_add_ref(e, this);
            e.mem.set_u32(out.addr(), surface);
            D3D_OK
        }
        _ => D3DERR_INVALIDCALL,
    }
}

/// `LockRect(level, locked out, rectangle, flags)`: [`surface_lock_rect`] on that level.
fn texture_lock_rect(
    e: &mut Engine,
    this: Ptr,
    level: u32,
    locked: Ptr,
    rect: Ptr,
    flags: u32,
) -> u32 {
    match texture_level(e, this, level) {
        Some(surface) => surface_lock_rect(e, Ptr::new(surface), locked, rect, flags),
        None => D3DERR_INVALIDCALL,
    }
}

/// `UnlockRect(level)`: [`surface_unlock_rect`] on that level.
fn texture_unlock_rect(e: &mut Engine, this: Ptr, level: u32) -> u32 {
    match texture_level(e, this, level) {
        Some(surface) => surface_unlock_rect(e, Ptr::new(surface)),
        None => D3DERR_INVALIDCALL,
    }
}

/// `LockRect(locked out, rectangle, flags)`: a surface with memory, not locked, answers its
/// `D3DLOCKED_RECT` (pitch, then the address of the rectangle's first block: the whole surface
/// for a null rectangle); one already locked, one without memory (`D3DPOOL_DEFAULT` and not
/// dynamic, the back buffers) or a null out answers `D3DERR_INVALIDCALL`. The flags
/// (`D3DLOCK_READONLY`, `D3DLOCK_DISCARD`, `D3DLOCK_NO_DIRTY_UPDATE`, ...) change nothing here:
/// the level's bytes are handed out as they are and every byte is kept at the unlock. A texture
/// level kept outside the game's memory ([`Pixels`]) is handed out in a staging block.
fn surface_lock_rect(e: &mut Engine, this: Ptr, locked: Ptr, rect: Ptr, _flags: u32) -> u32 {
    let s = surface(e, this);
    if locked.is_null() || (s.bits == 0 && s.pixels.is_none()) || s.locked {
        return D3DERR_INVALIDCALL;
    }
    let (bits, pitch, format) = (s.bits, s.pitch, s.desc[0]);
    s.locked = true;
    let bits = if bits != 0 { bits } else { stage(e, this) };
    let at = if rect.is_null() {
        bits
    } else {
        let (block, bytes) = format_blocks(format).expect("a sized format");
        let left = e.mem.u32(rect.addr());
        let top = e.mem.u32(rect.addr() + 4);
        bits + (top / block) * pitch + (left / block) * bytes
    };
    e.mem.set_u32(locked.addr(), pitch);
    e.mem.set_u32(locked.addr() + 4, at);
    D3D_OK
}

/// `UnlockRect()`: a locked surface is unlocked; one not locked answers `D3DERR_INVALIDCALL`.
/// (`NV_D3D9=real`) the level's bytes go to the real surface; a texture level kept outside the
/// game's memory takes its staging block's bytes back and the block is freed.
fn surface_unlock_rect(e: &mut Engine, this: Ptr) -> u32 {
    let s = surface(e, this);
    if !s.locked {
        return D3DERR_INVALIDCALL;
    }
    s.locked = false;
    let staging = std::mem::take(&mut s.staging);
    let bits = if staging != 0 { staging } else { s.bits };
    let uploaded = {
        let (pitch, desc) = (s.pitch, s.desc);
        let (_, size) = level_layout(desc[0], desc[6], desc[7]);
        super::d3d9real::surface_unlocked(e, this.addr(), bits, pitch, pitch, size / pitch.max(1))
    };
    if staging != 0 {
        unstage(e, this, staging, uploaded);
    }
    let _ = (bits, uploaded);
    D3D_OK
}

/// The bytes of a level kept outside the game's memory (rows packed, `size` of them).
fn level_size(s: &Surface) -> u32 {
    level_layout(s.desc[0], s.desc[6], s.desc[7]).1
}

/// A lock of a level kept outside the game's memory: a staging block of the game's memory
/// (zeroed) takes the level's bytes; its address, kept until the unlock.
fn stage(e: &mut Engine, this: Ptr) -> u32 {
    let size = level_size(surface(e, this));
    let block = e.mem.alloc(size);
    let s = device!(e)
        .device_objects
        .surfaces
        .get_mut(&this.addr())
        .expect("the level");
    s.staging = block;
    let from_real = match &s.pixels {
        Some(Pixels::Host(bytes)) => {
            if !bytes.is_empty() {
                e.mem.write(block, bytes);
            }
            false
        }
        Some(Pixels::Real { written }) => *written,
        // Zero until written: the block is zeroed.
        None => false,
    };
    if from_real && !super::d3d9real::read_surface(e, this.addr(), block, size) {
        panic!(
            "IDirect3DSurface9::LockRect: the real level {:#x} could not be read back (not \
             provided)",
            this.addr()
        );
    }
    let _ = from_real;
    block
}

/// The staging blocks' bytes that may wait for the next `Present` after their unlock (16 MB, the
/// largest level the game makes: 2048 by 2048 A8R8G8B8); past it the oldest are freed first,
/// the newest block always waits. A load that unlocks many levels in one frame so holds at most
/// this much more of the game's memory than with each block freed at its unlock.
const KEPT_STAGING_MAX: u64 = 16 << 20;

/// The unlock of a level kept outside the game's memory: the staging block's bytes are kept
/// (in host memory, or by the real level when `uploaded` put them there), and the block waits
/// for the next `Present` (or for [`KEPT_STAGING_MAX`]) to be freed, so code reading through the
/// lock's address just after the unlock still reads the level's bytes
/// (`NV_D3D9_TEXTURES=check`: the store read back and compared with the block, and the block
/// compared again when it is freed).
fn unstage(e: &mut Engine, this: Ptr, staging: u32, uploaded: bool) {
    let size = level_size(surface(e, this));
    let check = texture_store(e) == Store::Check;
    let s = device!(e)
        .device_objects
        .surfaces
        .get_mut(&this.addr())
        .expect("the level");
    match &mut s.pixels {
        Some(Pixels::Real { written }) if uploaded => *written = true,
        // A real level that did not take the bytes: they stay in host memory from now on.
        Some(p @ Pixels::Real { .. }) => *p = Pixels::Host(e.mem.bytes(staging, size)),
        Some(Pixels::Host(bytes)) => {
            bytes.resize(size as usize, 0);
            e.mem.read(staging, bytes);
        }
        None => unreachable!("a staged level"),
    }
    let held = if check {
        check_round_trip(e, this, staging, size)
    } else {
        Vec::new()
    };
    wait_for_present(e, (staging, this.addr(), size, held, false));
}

/// A staging block just unlocked (block, level or buffer, size, with `check` the bytes it held,
/// whether a buffer's) waits for the next `Present` to be freed; past [`KEPT_STAGING_MAX`]
/// bytes waiting the oldest are freed now (the newest always waits).
pub(super) fn wait_for_present(e: &mut Engine, unlocked: (u32, u32, u32, Vec<u8>, bool)) {
    let objects = &mut device!(e).device_objects;
    objects.unlocked_staging_bytes += u64::from(unlocked.2);
    objects.unlocked_staging.push_back(unlocked);
    loop {
        let objects = &mut device!(e).device_objects;
        if objects.unlocked_staging_bytes <= KEPT_STAGING_MAX || objects.unlocked_staging.len() < 2
        {
            break;
        }
        let oldest = objects
            .unlocked_staging
            .pop_front()
            .expect("more than one block");
        objects.unlocked_staging_bytes -= u64::from(oldest.2);
        free_unlocked_staging(e, oldest);
    }
}

/// `NV_D3D9_TEXTURES=check`, at an unlock: the level read back from where it keeps its bytes and
/// compared with the staging block; the block's bytes, to compare again when it is freed.
fn check_round_trip(e: &mut Engine, this: Ptr, staging: u32, size: u32) -> Vec<u8> {
    let held = e.mem.bytes(staging, size);
    let kept = stored_bytes(e, this, size);
    let objects = &mut device!(e).device_objects;
    let same = kept == held;
    objects.round_trips[usize::from(!same)] += 1;
    objects.round_trip_bytes += u64::from(size);
    let [same_count, differ] = objects.round_trips;
    if !same {
        eprintln!(
            "NV_D3D9_TEXTURES=check: the texture level {:#x} kept other bytes than its lock \
             handed back ({size} bytes)",
            this.addr()
        );
    }
    if (same_count + differ) % 2000 == 0 {
        eprintln!(
            "NV_D3D9_TEXTURES=check: {} unlocks ({} MB) read back from where the levels keep \
             them: {differ} differ; {} levels written after an unlock",
            same_count + differ,
            objects.round_trip_bytes >> 20,
            objects.written_after_unlock.len()
        );
    }
    held
}

/// `NV_D3D9_TEXTURES=check`: the bytes a level kept outside the game's memory holds, read back
/// from where it keeps them (`size` bytes).
fn stored_bytes(e: &mut Engine, this: Ptr, size: u32) -> Vec<u8> {
    match &surface(e, this).pixels {
        Some(Pixels::Host(bytes)) => bytes.clone(),
        Some(Pixels::Real { .. }) => {
            let block = e.mem.alloc(size);
            let read = super::d3d9real::read_surface(e, this.addr(), block, size);
            let bytes = e.mem.bytes(block, size);
            e.mem.free(block);
            if read {
                bytes
            } else {
                Vec::new()
            }
        }
        None => Vec::new(),
    }
}

/// At each `Present`: the staging blocks unlocked since the last one are freed
/// ([`free_unlocked_staging`]).
fn free_all_unlocked_staging(e: &mut Engine) {
    let objects = &mut device!(e).device_objects;
    let kept = std::mem::take(&mut objects.unlocked_staging);
    objects.unlocked_staging_bytes = 0;
    for block in kept {
        free_unlocked_staging(e, block);
    }
}

/// At the join: `object` (a texture a release or unbinding on the drawing side would have ended)
/// ends now if nothing holds it.
pub(super) fn end_if_unheld(e: &mut Engine, object: u32) {
    if let Some(t) = device!(e).device_objects.textures.get(&object) {
        if t.references == 0 && t.bound == 0 {
            texture_destroy(e, Ptr::new(object));
        }
    }
}

/// Whether `device` is a device of this layer.
pub(super) fn has_device(e: &mut Engine, device: u32) -> bool {
    device!(e).device_objects.devices.contains_key(&device)
}

/// The join of a cut during which main presented: the frame's unlocked staging blocks freed, as
/// that `Present` would have.
pub(super) fn free_frame_staging(e: &mut Engine) {
    free_all_unlocked_staging(e);
}

/// The cut (`super::d3d9draw::draw_cut`): main's state of `device` copied for the drawing side,
/// what it binds held once more by the copy: its textures (their samplers' count, which the
/// program never sees) and its render targets and depth surface (a reference each, as
/// `SetRenderTarget` takes one: while the cut is open, `AddRef` and `Release` of those surfaces
/// answer one more).
pub(super) fn copy_for_drawing(e: &mut Engine, device: u32) {
    let copy = device!(e).device_objects.devices[&device].clone();
    for &t in copy.textures.values() {
        texture(e, Ptr::new(t)).bound += 1;
    }
    for &s in copy.render_targets.iter().chain([&copy.depth_stencil]) {
        if s != 0 {
            surface_add_ref(e, Ptr::new(s));
        }
    }
    device!(e).device_objects.draw_devices.insert(device, copy);
}

/// `main`'s value for each key `take` refuses, `draw`'s for the others.
fn merge_map<V: Copy>(
    main: &mut engine_core::AddrMap<u32, V>,
    draw: &engine_core::AddrMap<u32, V>,
    take: impl Fn(u32) -> bool,
) {
    let keys: std::collections::BTreeSet<u32> = main.keys().chain(draw.keys()).copied().collect();
    for k in keys {
        if take(k) {
            match draw.get(&k) {
                Some(v) => {
                    main.insert(k, *v);
                }
                None => {
                    main.remove(&k);
                }
            }
        }
    }
}

/// The join (`super::d3d9draw::draw_join`): main's state of `device` becomes the serial order's,
/// main's value for each piece in `set` (set by main after the cut), the drawing side's copy's for
/// every other; the copy's holds are let go of, and main's for what the copy's values replace.
pub(super) fn merge_drawing(
    e: &mut Engine,
    device: u32,
    set: &std::collections::HashSet<super::d3d9draw::Key>,
) {
    use super::d3d9draw::Key;
    let Some(draw) = device!(e).device_objects.draw_devices.remove(&device) else {
        return;
    };
    let take = |k: Key| !set.contains(&k);
    let mut unbind = Vec::new();
    let mut targets = Vec::new();
    {
        let main = device!(e)
            .device_objects
            .devices
            .get_mut(&device)
            .expect("the device of the cut");
        for i in 0..RENDER_STATES {
            if take(Key::Render(i as u32)) {
                main.render_states.0[i] = draw.render_states.0[i];
            }
        }
        for i in 0..STAGE_STATES {
            if take(Key::Stage(i as u32)) {
                main.stage_states.0[i] = draw.stage_states.0[i];
            }
        }
        for i in 0..SAMPLER_STATES {
            if take(Key::Sampler(i as u32)) {
                main.sampler_states.0[i] = draw.sampler_states.0[i];
            }
        }
        merge_map(&mut main.transforms, &draw.transforms, |s| {
            take(Key::Transform(s))
        });
        merge_map(&mut main.clip_planes, &draw.clip_planes, |i| {
            take(Key::ClipPlane(i))
        });
        merge_map(&mut main.lights, &draw.lights, |i| take(Key::Light(i)));
        let lights: std::collections::BTreeSet<u32> = main
            .enabled_lights
            .union(&draw.enabled_lights)
            .copied()
            .collect();
        for i in lights {
            if take(Key::LightEnable(i)) {
                if draw.enabled_lights.contains(&i) {
                    main.enabled_lights.insert(i);
                } else {
                    main.enabled_lights.remove(&i);
                }
            }
        }
        // An index main enabled or disabled after the cut that neither side had a light for: the
        // default its LightEnable made.
        for k in set {
            if let Key::LightEnable(i) = *k {
                main.lights.entry(i).or_insert_with(default_light);
            }
        }
        if take(Key::Viewport) {
            main.viewport = draw.viewport;
        }
        if take(Key::Scissor) {
            main.scissor = draw.scissor;
        }
        if take(Key::InScene) {
            main.in_scene = draw.in_scene;
        }
        if take(Key::Material) {
            main.material = draw.material;
        }
        if take(Key::SoftwareVertexProcessing) {
            main.software_vertex_processing = draw.software_vertex_processing;
        }
        if take(Key::GammaRamp) {
            main.gamma_ramp.clone_from(&draw.gamma_ramp);
        }
        // The targets: each held by main and by the copy; the one not kept lets go.
        for i in 0..RENDER_TARGETS {
            if take(Key::RenderTarget(i as u32)) {
                targets.push(main.render_targets[i]);
                main.render_targets[i] = draw.render_targets[i];
            } else {
                targets.push(draw.render_targets[i]);
            }
        }
        if take(Key::DepthStencil) {
            targets.push(main.depth_stencil);
            main.depth_stencil = draw.depth_stencil;
        } else {
            targets.push(draw.depth_stencil);
        }
        let samplers: std::collections::BTreeSet<u32> = main
            .textures
            .keys()
            .chain(draw.textures.keys())
            .copied()
            .collect();
        for s in samplers {
            let (old, copy) = (
                main.textures.get(&s).copied(),
                draw.textures.get(&s).copied(),
            );
            if take(Key::Texture(s)) {
                match copy {
                    Some(t) => main.textures.insert(s, t),
                    None => main.textures.remove(&s),
                };
                unbind.extend(old);
            } else {
                unbind.extend(copy);
            }
        }
    }
    for surface in targets {
        if surface != 0 {
            surface_release(e, Ptr::new(surface));
        }
    }
    for t in unbind {
        texture_unbind(e, Ptr::new(t));
    }
}

/// Frees a staging block waiting since its unlock; with `check` it is first compared with what
/// it held at the unlock (a difference is printed once per level or buffer).
fn free_unlocked_staging(
    e: &mut Engine,
    (block, level, size, held, buffer): (u32, u32, u32, Vec<u8>, bool),
) {
    if !held.is_empty()
        && e.mem.bytes(block, size) != held
        && device!(e).device_objects.written_after_unlock.insert(level)
    {
        if buffer {
            eprintln!(
                "NV_D3D9_BUFFERS=check: the buffer {level:#x} was written through its lock's \
                 address after its last Unlock (staging block {block:#x}, {size} bytes)"
            );
        } else {
            eprintln!(
                "NV_D3D9_TEXTURES=check: the texture level {level:#x} was written through its \
                 lock's address after UnlockRect (staging block {block:#x}, {size} bytes)"
            );
        }
    }
    e.mem.free(block);
}

/// `check`: whether a level or buffer was written through a lock's address after its unlock
/// (tests).
#[cfg(test)]
pub(crate) fn written_after_unlock(e: &mut Engine, object: u32) -> bool {
    device!(e)
        .device_objects
        .written_after_unlock
        .contains(&object)
}

/// The present parameters a device took (tests, with no real threads: `device_ref!` checks).
#[cfg(test)]
pub(crate) fn device_present_parameters(e: &Engine, device: u32) -> Option<Vec<u8>> {
    device_ref!(e)
        .device_objects
        .devices
        .get(&device)
        .map(|d| d.present.clone())
}

// The fixed-function and copy calls below answer as the retail runtime did on this machine
// (2026-10-27, `scripts/d3d9-capture device fixed`, windowed HAL devices with hardware and with
// mixed vertex processing; cases that faulted the runtime were run one per process). Nothing is
// drawn yet: the states are kept, the copies only answered.

// Platform: replaces D3D9.DLL IDirect3DDevice9::SetMaterial (vtable +0xC4)
/// `SetMaterial(material)`: keeps the 17 words of the `D3DMATERIAL9` and answers `D3D_OK` (the
/// runtime checks nothing in it). A null material faulted the runtime: not provided.
fn set_material(e: &mut Engine, this: Ptr, material: Ptr) -> u32 {
    if material.is_null() {
        panic!("IDirect3DDevice9::SetMaterial(NULL) is not provided (the runtime faulted)");
    }
    let words: [u32; 17] = std::array::from_fn(|i| e.mem.u32(material.addr() + 4 * i as u32));
    mark(e, super::d3d9draw::Key::Material);
    device(e, this).material = Some(words);
    D3D_OK
}

// Platform: replaces D3D9.DLL IDirect3DDevice9::SetLight (vtable +0xCC)
/// `SetLight(index, light)`: keeps the 26 words of the `D3DLIGHT9` for the index and answers
/// `D3D_OK` whatever they hold: the runtime took types 0 and 4, zero attenuation, negative or
/// zero ranges, a zero direction, theta above phi, phi above pi and NaN colours alike. A null
/// light faulted the runtime: not provided.
fn set_light(e: &mut Engine, this: Ptr, index: u32, light: Ptr) -> u32 {
    if light.is_null() {
        panic!("IDirect3DDevice9::SetLight({index}, NULL) is not provided (the runtime faulted)");
    }
    let words: [u32; 26] = std::array::from_fn(|i| e.mem.u32(light.addr() + 4 * i as u32));
    mark(e, super::d3d9draw::Key::Light(index));
    device(e, this).lights.insert(index, words);
    D3D_OK
}

/// The light `LightEnable` makes for an index never set (read back with `GetLight`): a
/// directional light (type 3) with diffuse (1, 1, 1, 0) and direction (0, 0, 1), everything
/// else 0.
fn default_light() -> [u32; 26] {
    let mut l = [0u32; 26];
    l[0] = 3;
    l[1] = 1f32.to_bits();
    l[2] = 1f32.to_bits();
    l[3] = 1f32.to_bits();
    l[18] = 1f32.to_bits();
    l
}

// Platform: replaces D3D9.DLL IDirect3DDevice9::LightEnable (vtable +0xD4)
/// `LightEnable(index, enable)`: enables (any nonzero value) or disables the light and answers
/// `D3D_OK`; an index never set first gets [`default_light`]. The runtime set no limit (ten more
/// lights than any caps count were taken).
fn light_enable(e: &mut Engine, this: Ptr, index: u32, enable: u32) -> u32 {
    // Not the light itself: the default it may make here comes after a light the drawing side set
    // for the index in the serial order, and the join makes the default where neither side has
    // one (`merge_drawing`).
    mark(e, super::d3d9draw::Key::LightEnable(index));
    let d = device(e, this);
    d.lights.entry(index).or_insert_with(default_light);
    if enable != 0 {
        d.enabled_lights.insert(index);
    } else {
        d.enabled_lights.remove(&index);
    }
    D3D_OK
}

/// `D3DCREATE_MIXED_VERTEXPROCESSING`.
const D3DCREATE_MIXED_VERTEXPROCESSING: u32 = 0x80;

// Platform: replaces D3D9.DLL IDirect3DDevice9::SetSoftwareVertexProcessing (vtable +0x134)
/// `SetSoftwareVertexProcessing(software)`: on a device with mixed vertex processing any value
/// is taken (nonzero switches to software) and `D3D_OK` answered; on one with hardware vertex
/// processing FALSE answers `D3D_OK` and anything else `D3DERR_INVALIDCALL` (measured with 1
/// and 2).
fn set_software_vertex_processing(e: &mut Engine, this: Ptr, software: u32) -> u32 {
    let d = device(e, this);
    if d.creation[3] & D3DCREATE_MIXED_VERTEXPROCESSING != 0 {
        d.software_vertex_processing = software != 0;
        mark(e, super::d3d9draw::Key::SoftwareVertexProcessing);
        return D3D_OK;
    }
    if software == 0 {
        D3D_OK
    } else {
        D3DERR_INVALIDCALL
    }
}

/// `D3DPOOL_SYSTEMMEM`.
const D3DPOOL_SYSTEMMEM: u32 = 2;

// Platform: replaces D3D9.DLL IDirect3DDevice9::UpdateTexture (vtable +0x7C)
/// `UpdateTexture(source, destination)` for textures of this layer: a system-memory source and a
/// default-pool destination answer `D3D_OK` (the runtime took mismatched sizes, formats and
/// level counts there, in a scene and outside one); any other pair of pools answers
/// `D3DERR_INVALIDCALL`. Nothing is drawn, so no level is copied (a default-pool texture is not
/// read back here). A null source faulted the runtime: not provided; so is a destination that
/// is not a texture of this layer.
fn update_texture(e: &mut Engine, _this: Ptr, source: u32, destination: u32) -> u32 {
    let pools = |e: &Engine, t: u32| {
        device_ref!(e)
            .device_objects
            .textures
            .get(&t)
            .map(|t| t.pool)
    };
    let (Some(from), Some(to)) = (pools(e, source), pools(e, destination)) else {
        panic!("IDirect3DDevice9::UpdateTexture({source:#x}, {destination:#x}) on objects that are not textures of this layer is not provided");
    };
    if from == D3DPOOL_SYSTEMMEM && to == D3DPOOL_DEFAULT {
        D3D_OK
    } else {
        D3DERR_INVALIDCALL
    }
}

// Platform: replaces D3D9.DLL IDirect3DDevice9::StretchRect (vtable +0x88)
/// `StretchRect(source, source rect, destination, destination rect, filter)` for surfaces of
/// this layer: the destination must be a render target (a render-target surface or a level of a
/// render-target texture) and the source a default-pool surface (a render target, a level of a
/// render-target or default-pool texture) other than the destination; then `D3D_OK` (stretching,
/// rectangles, a format conversion and each filter were taken, in a scene and outside one),
/// else `D3DERR_INVALIDCALL` (a system-memory source, a destination that is no render target,
/// the same surface, a null destination). Nothing is drawn, so nothing is copied. A source that
/// is not a surface of this layer is not provided.
fn stretch_rect(
    e: &mut Engine,
    _this: Ptr,
    source: u32,
    _source_rect: u32,
    destination: u32,
    _destination_rect: u32,
    _filter: u32,
) -> u32 {
    let desc = |e: &Engine, s: u32| {
        device_ref!(e)
            .device_objects
            .surfaces
            .get(&s)
            .map(|s| (s.desc, s.container))
    };
    let Some((from, from_texture)) = desc(e, source) else {
        panic!("IDirect3DDevice9::StretchRect from {source:#x}, which is not a surface of this layer, is not provided");
    };
    let Some((to, _)) = desc(e, destination) else {
        return D3DERR_INVALIDCALL;
    };
    // A depth-stencil surface on either side was not measured.
    if (from[2] | to[2]) & D3DUSAGE_DEPTHSTENCIL != 0 {
        panic!("IDirect3DDevice9::StretchRect with a depth-stencil surface is not provided");
    }
    let source_kind_measured =
        from[2] & D3DUSAGE_RENDERTARGET != 0 || from_texture != 0 || from[3] == D3DPOOL_SYSTEMMEM;
    if !source_kind_measured {
        panic!("IDirect3DDevice9::StretchRect from a kind of surface not measured is not provided");
    }
    // `D3DSURFACE_DESC`: Usage +2, Pool +3.
    let ok = source != destination
        && to[2] & D3DUSAGE_RENDERTARGET != 0
        && to[3] == D3DPOOL_DEFAULT
        && from[3] == D3DPOOL_DEFAULT;
    if ok {
        D3D_OK
    } else {
        D3DERR_INVALIDCALL
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::d3d9;

    /// TEST DATA, not a capture: a small adapter table in `scripts/d3d9-capture`'s format (a
    /// 640x480 X8R8G8B8 desktop, HAL caps with four simultaneous render targets, a windowed
    /// X8R8G8B8 back buffer accepted).
    fn test_table() -> String {
        let hex = |b: &[u8]| -> String { b.iter().map(|x| format!("{x:02x}")).collect() };
        let mut mode = vec![0u8; 16];
        mode[..4].copy_from_slice(&640u32.to_le_bytes());
        mode[4..8].copy_from_slice(&480u32.to_le_bytes());
        mode[8..12].copy_from_slice(&60u32.to_le_bytes());
        mode[12..].copy_from_slice(&0x16u32.to_le_bytes());
        let mut caps = vec![0u8; 304];
        caps[..4].copy_from_slice(&1u32.to_le_bytes());
        caps[CAPS_SIMULTANEOUS_RTS..CAPS_SIMULTANEOUS_RTS + 4].copy_from_slice(&4u32.to_le_bytes());
        [
            format!("GetAdapterDisplayMode\t0\t0\t{}", hex(&mode)),
            format!("GetDeviceCaps\t0 1\t0\t{}", hex(&caps)),
            "Domain\td0\t0,1".to_string(),
            "CheckDeviceType\t0 1 16 16\t0\t\td0".to_string(),
        ]
        .join("\n")
    }

    /// An engine with the table and an `IDirect3D9`, and windowed present parameters (640x480,
    /// format and count left for the defaults, device window 0x11c).
    fn d3d() -> (Engine, u32, u32) {
        let mut e = Engine::new();
        d3d9::load_adapter_table(&mut e, &test_table());
        let create = d3d9::proc_address(&mut e, b"Direct3DCreate9").unwrap();
        let d3d = e.call(create, &[d3d9::D3D_SDK_VERSION]).u32();
        let present = e.mem.alloc(PRESENT_SIZE);
        e.mem.set_u32(present + PP_WIDTH, 640);
        e.mem.set_u32(present + PP_HEIGHT, 480);
        e.mem.set_u32(present + PP_WINDOW, 0x11c);
        e.mem.set_u32(present + PP_WINDOWED, 1);
        (e, d3d, present)
    }

    const CREATE_DEVICE: u32 = 0x40;

    #[test]
    fn create_device_fills_the_defaults_and_makes_the_back_buffer_render_target_0() {
        let (mut e, d3d, present) = d3d();
        let out = e.mem.alloc(4);
        let hr = e
            .vcall(d3d, CREATE_DEVICE, &[0, 1, 0x11c, 0x40, present, out])
            .u32();
        assert_eq!(hr, D3D_OK);
        let device = e.mem.u32(out);
        assert_eq!(e.mem.u32(device), device!(e).device_objects.device_vtable);
        assert_eq!(e.mem.u32(present + PP_COUNT), 1);
        assert_eq!(e.mem.u32(present + PP_FORMAT), 0x16);
        // The device holds a reference to the IDirect3D9.
        assert_eq!(e.vcall(d3d, 4, &[]).u32(), 3);
        e.vcall(d3d, 8, &[]);
        let slot = e.mem.alloc(4);
        assert_eq!(e.vcall(device, 4 * 38, &[0, slot]).u32(), D3D_OK);
        let target = e.mem.u32(slot);
        assert_eq!(e.vcall(device, 4 * 18, &[0, 0, 0, slot]).u32(), D3D_OK);
        assert_eq!(e.mem.u32(slot), target);
        assert_eq!(
            e.vcall(device, 4 * 18, &[0, 1, 0, slot]).u32(),
            D3DERR_INVALIDCALL
        );
        let desc = e.mem.alloc(0x20);
        assert_eq!(e.vcall(target, 4 * 12, &[desc]).u32(), D3D_OK);
        let words: Vec<u32> = (0..8).map(|i| e.mem.u32(desc + 4 * i)).collect();
        assert_eq!(words, [0x16, 1, 1, 0, 0, 0, 640, 480]);
        assert_eq!(e.vcall(target, 4 * 10, &[]).u32(), D3DRTYPE_SURFACE);
        // Two references taken, two released; the back buffer stays with the device.
        assert_eq!(e.vcall(target, 8, &[]).u32(), 2);
        assert_eq!(e.vcall(target, 8, &[]).u32(), 1);
        assert!(device!(e).device_objects.surfaces.contains_key(&target));
        assert_eq!(e.vcall(device, 4 * 40, &[slot]).u32(), D3DERR_NOTFOUND);
        assert_eq!(e.mem.u32(slot), 0);
        assert_eq!(e.vcall(device, 4 * 38, &[1, slot]).u32(), D3DERR_NOTFOUND);
        assert_eq!(
            e.vcall(device, 4 * 38, &[4, slot]).u32(),
            D3DERR_INVALIDCALL
        );
        let viewport = e.mem.alloc(0x18);
        assert_eq!(e.vcall(device, 4 * 48, &[viewport]).u32(), D3D_OK);
        assert_eq!(
            (e.mem.u32(viewport + 8), e.mem.u32(viewport + 0xc)),
            (640, 480)
        );
        assert_eq!(e.mem.u32(viewport + 0x14), 1f32.to_bits());
        e.mem.set_u32(viewport + 8, 641);
        assert_eq!(
            e.vcall(device, 4 * 47, &[viewport]).u32(),
            D3DERR_INVALIDCALL
        );
        let params = e.mem.alloc(0x10);
        assert_eq!(e.vcall(device, 4 * 9, &[params]).u32(), D3D_OK);
        assert_eq!(e.mem.u32(params + 0xc), 0x40);
        let caps = e.mem.alloc(304);
        assert_eq!(e.vcall(device, 4 * 7, &[caps]).u32(), D3D_OK);
        assert_eq!(e.mem.u32(caps), 1);
        assert_eq!(e.vcall(device, 4 * 2, &[]).u32(), 0);
        assert!(device!(e).device_objects.devices.is_empty());
        assert!(device!(e).device_objects.surfaces.is_empty());
        assert_eq!(e.vcall(d3d, 4, &[]).u32(), 2);
    }

    #[test]
    fn create_device_refuses_what_direct3d_refuses() {
        let (mut e, d3d, present) = d3d();
        let out = e.mem.alloc(4);
        let create = |e: &mut Engine, behavior: u32| {
            e.vcall(d3d, CREATE_DEVICE, &[0, 1, 0x11c, behavior, present, out])
                .u32()
        };
        assert_eq!(create(&mut e, 0), D3DERR_INVALIDCALL);
        assert_eq!(create(&mut e, 0x40 | 0x80), D3DERR_INVALIDCALL);
        e.mem.set_u32(present + PP_COUNT, 4);
        assert_eq!(create(&mut e, 0x40), D3DERR_INVALIDCALL);
        assert_eq!(e.mem.u32(present + PP_COUNT), 3);
        e.mem.set_u32(present + PP_REFRESH, 60);
        assert_eq!(create(&mut e, 0x40), D3DERR_INVALIDCALL);
        e.mem.set_u32(present + PP_REFRESH, 0);
        assert_eq!(create(&mut e, 0x40), D3D_OK);
        let device = e.mem.u32(out);
        let slot = e.mem.alloc(4);
        for i in 0..3 {
            assert_eq!(e.vcall(device, 4 * 18, &[0, i, 0, slot]).u32(), D3D_OK);
        }
        let hr = e
            .vcall(d3d, CREATE_DEVICE, &[0, 1, 0x11c, 0x40, 0, out])
            .u32();
        assert_eq!(hr, D3DERR_INVALIDCALL);
    }

    #[test]
    fn render_target_and_depth_slots_hold_their_surfaces() {
        let (mut e, d3d, present) = d3d();
        e.mem.set_u32(present + PP_AUTO_DEPTH, 1);
        e.mem.set_u32(present + PP_DEPTH_FORMAT, 0x4b);
        let out = e.mem.alloc(4);
        e.vcall(d3d, CREATE_DEVICE, &[0, 1, 0x11c, 0x40, present, out]);
        let device = e.mem.u32(out);
        let slot = e.mem.alloc(4);
        assert_eq!(e.vcall(device, 4 * 40, &[slot]).u32(), D3D_OK);
        let depth = e.mem.u32(slot);
        assert_eq!(e.vcall(device, 4 * 38, &[0, slot]).u32(), D3D_OK);
        let target = e.mem.u32(slot);
        // The depth surface is no render target, the back buffer no depth surface.
        assert_eq!(
            e.vcall(device, 4 * 37, &[0, depth]).u32(),
            D3DERR_INVALIDCALL
        );
        assert_eq!(e.vcall(device, 4 * 39, &[target]).u32(), D3DERR_INVALIDCALL);
        assert_eq!(e.vcall(device, 4 * 37, &[0, 0]).u32(), D3DERR_INVALIDCALL);
        assert_eq!(e.vcall(device, 4 * 37, &[1, target]).u32(), D3D_OK);
        assert_eq!(e.vcall(device, 4 * 37, &[1, 0]).u32(), D3D_OK);
        assert_eq!(e.vcall(device, 4 * 39, &[0]).u32(), D3D_OK);
        assert_eq!(e.vcall(device, 4 * 40, &[slot]).u32(), D3DERR_NOTFOUND);
        assert_eq!(e.vcall(device, 4 * 39, &[depth]).u32(), D3D_OK);
        assert_eq!(e.vcall(device, 4 * 40, &[slot]).u32(), D3D_OK);
        assert_eq!(e.mem.u32(slot), depth);
        let got = e.mem.alloc(4);
        assert_eq!(e.vcall(depth, 4 * 3, &[got]).u32(), D3D_OK);
        assert_eq!(e.mem.u32(got), device);
        assert_eq!(e.vcall(device, 4 * 2, &[]).u32(), 1);
    }

    #[test]
    fn cube_textures_have_six_faces_of_lockable_levels() {
        let (mut e, d3d, present) = d3d();
        let out = e.mem.alloc(4);
        e.vcall(d3d, CREATE_DEVICE, &[0, 1, 0x11c, 0x40, present, out]);
        let device = e.mem.u32(out);
        // TEST DATA: the table accepts A8R8G8B8 (0x15) cube textures with usage 0.
        let table =
            test_table() + "\nDomain\td1\t15,16\nCheckDeviceFormat\t0 1 16 0 5\t0\t16=8876086a\td1";
        d3d9::load_adapter_table(&mut e, &table);
        assert_eq!(
            e.vcall(device, 4 * 25, &[4, 0, 0, 0x16, 1, out, 0]).u32(),
            D3DERR_INVALIDCALL
        );
        assert_eq!(
            e.vcall(device, 4 * 25, &[4, 0, 0, 0x15, 1, out, 0]).u32(),
            D3D_OK
        );
        let cube = e.mem.u32(out);
        assert_eq!(e.vcall(cube, 4 * 10, &[]).u32(), D3DRTYPE_CUBETEXTURE);
        assert_eq!(e.vcall(cube, 4 * 13, &[]).u32(), 3, "4, 2, 1");
        let desc = e.mem.alloc(0x20);
        assert_eq!(e.vcall(cube, 4 * 17, &[1, desc]).u32(), D3D_OK);
        assert_eq!((e.mem.u32(desc + 0x18), e.mem.u32(desc + 0x1c)), (2, 2));
        assert_eq!(e.vcall(cube, 4 * 17, &[3, desc]).u32(), D3DERR_INVALIDCALL);
        let slot = e.mem.alloc(4);
        assert_eq!(e.vcall(cube, 4 * 18, &[5, 2, slot]).u32(), D3D_OK);
        let last = e.mem.u32(slot);
        assert_eq!(e.vcall(cube, 4 * 18, &[0, 2, slot]).u32(), D3D_OK);
        assert_ne!(e.mem.u32(slot), last, "each face its own surface");
        assert_eq!(
            e.vcall(cube, 4 * 18, &[6, 0, slot]).u32(),
            D3DERR_INVALIDCALL
        );
        let locked = e.mem.alloc(8);
        assert_eq!(e.vcall(cube, 4 * 19, &[5, 0, locked, 0, 0]).u32(), D3D_OK);
        assert_eq!(e.mem.u32(locked), 16, "4 pixels of 4 bytes");
        assert_eq!(e.vcall(cube, 4 * 20, &[5, 0]).u32(), D3D_OK);
        // Two surfaces taken, the texture's own reference: three releases free it.
        e.vcall(cube, 8, &[]);
        e.vcall(cube, 8, &[]);
        assert_eq!(e.vcall(cube, 8, &[]).u32(), 0);
        assert!(!device!(e).device_objects.surfaces.contains_key(&last));
    }

    #[test]
    fn render_targets_check_their_format_and_multisampling() {
        let (mut e, d3d, present) = d3d();
        let out = e.mem.alloc(4);
        e.vcall(d3d, CREATE_DEVICE, &[0, 1, 0x11c, 0x40, present, out]);
        let device = e.mem.u32(out);
        // TEST DATA: A8R8G8B8 render targets accepted; 4 samples with 2 quality levels.
        let table = test_table()
            + "\nDomain\td1\t15,16\nCheckDeviceFormat\t0 1 16 1 1\t8876086a\t15=0\td1"
            + "\nDomain\td2\t0,1,2,3,4\nCheckDeviceMultiSampleType\t0 1 15 1\t8876086a/0\t0=0/1,4=0/2\td2";
        d3d9::load_adapter_table(&mut e, &table);
        let create = |e: &mut Engine, format: u32, ms: u32, q: u32, lockable: u32| {
            e.vcall(device, 4 * 28, &[640, 480, format, ms, q, lockable, out, 0])
                .u32()
        };
        assert_eq!(create(&mut e, 0x16, 0, 0, 0), D3DERR_INVALIDCALL);
        assert_eq!(create(&mut e, 0x15, 2, 0, 0), D3DERR_INVALIDCALL);
        assert_eq!(create(&mut e, 0x15, 4, 2, 0), D3DERR_INVALIDCALL);
        assert_eq!(create(&mut e, 0x15, 4, 1, 0), D3D_OK);
        let target = e.mem.u32(out);
        let desc = e.mem.alloc(0x20);
        assert_eq!(e.vcall(target, 4 * 12, &[desc]).u32(), D3D_OK);
        let words: Vec<u32> = (0..8).map(|i| e.mem.u32(desc + 4 * i)).collect();
        assert_eq!(words, [0x15, 1, 1, 0, 4, 1, 640, 480]);
        assert_eq!(create(&mut e, 0x15, 0, 0, 1), D3D_OK);
        let lockable = e.mem.u32(out);
        let locked = e.mem.alloc(8);
        assert_eq!(e.vcall(lockable, 4 * 13, &[locked, 0, 0]).u32(), D3D_OK);
        assert_eq!(e.mem.u32(locked), 640 * 4);
        assert_eq!(e.vcall(lockable, 4 * 14, &[]).u32(), D3D_OK);
        assert_eq!(e.vcall(lockable, 8, &[]).u32(), 0);
        assert_eq!(e.vcall(target, 8, &[]).u32(), 0);
    }

    #[test]
    fn depth_stencil_surfaces_are_made_unlockable_and_hold_their_device() {
        let (mut e, d3d, present) = d3d();
        let out = e.mem.alloc(4);
        e.vcall(d3d, CREATE_DEVICE, &[0, 1, 0x11c, 0x40, present, out]);
        let device = e.mem.u32(out);
        // TEST DATA: the table accepts D24S8 (0x4b) as a depth-stencil surface, refuses D16 (0x50).
        let table =
            test_table() + "\nDomain\td1\t4b,50\nCheckDeviceFormat\t0 1 16 2 1\t0\t50=8876086a\td1";
        d3d9::load_adapter_table(&mut e, &table);
        let refs = |e: &mut Engine| {
            e.vcall(device, 4, &[]);
            e.vcall(device, 8, &[]).u32()
        };
        let before = refs(&mut e);
        let create = |e: &mut Engine, format: u32| {
            e.vcall(device, 4 * 29, &[1920, 1080, format, 0, 0, 0, out, 0])
                .u32()
        };
        assert_eq!(create(&mut e, 0x50), D3DERR_INVALIDCALL);
        assert_eq!(
            e.vcall(device, 4 * 29, &[0, 1080, 0x4b, 0, 0, 0, out, 0])
                .u32(),
            D3DERR_INVALIDCALL
        );
        assert_eq!(create(&mut e, 0x4b), D3D_OK);
        let surface = e.mem.u32(out);
        assert_eq!(refs(&mut e), before + 1);
        let desc = e.mem.alloc(0x20);
        assert_eq!(e.vcall(surface, 4 * 12, &[desc]).u32(), D3D_OK);
        let words: Vec<u32> = (0..8).map(|i| e.mem.u32(desc + 4 * i)).collect();
        assert_eq!(words, [0x4b, 1, 2, 0, 0, 0, 1920, 1080]);
        let locked = e.mem.alloc(8);
        assert_eq!(
            e.vcall(surface, 4 * 13, &[locked, 0, 0]).u32(),
            D3DERR_INVALIDCALL
        );
        assert_eq!(e.vcall(surface, 8, &[]).u32(), 0);
        assert!(!device!(e).device_objects.surfaces.contains_key(&surface));
        assert_eq!(refs(&mut e), before);
    }

    #[test]
    fn textures_have_lockable_levels_that_count_with_the_texture() {
        let (mut e, d3d, present) = d3d();
        let out = e.mem.alloc(4);
        e.vcall(d3d, CREATE_DEVICE, &[0, 1, 0x11c, 0x40, present, out]);
        let device = e.mem.u32(out);
        // TEST DATA: the table accepts R5G6B5 (0x17) and DXT1 textures with usage 0.
        let dxt1 = u32::from_le_bytes(*b"DXT1");
        let table = test_table()
            + "\nDomain\td1\t16,17,31545844\nCheckDeviceFormat\t0 1 16 0 3\t0\t16=8876086a\td1";
        d3d9::load_adapter_table(&mut e, &table);
        let hr = e
            .vcall(device, 4 * 23, &[4, 2, 0, 0, 0x17, 1, out, 0])
            .u32();
        assert_eq!(hr, D3D_OK);
        let texture = e.mem.u32(out);
        assert_eq!(e.vcall(texture, 4 * 13, &[]).u32(), 3, "4x2, 2x1, 1x1");
        assert_eq!(e.vcall(texture, 4 * 10, &[]).u32(), D3DRTYPE_TEXTURE);
        let desc = e.mem.alloc(0x20);
        assert_eq!(e.vcall(texture, 4 * 17, &[1, desc]).u32(), D3D_OK);
        assert_eq!((e.mem.u32(desc + 0x18), e.mem.u32(desc + 0x1c)), (2, 1));
        assert_eq!(e.mem.u32(desc + 0xc), 1, "managed");
        assert_eq!(
            e.vcall(texture, 4 * 17, &[3, desc]).u32(),
            D3DERR_INVALIDCALL
        );
        let slot = e.mem.alloc(4);
        assert_eq!(e.vcall(texture, 4 * 18, &[0, slot]).u32(), D3D_OK);
        let level = e.mem.u32(slot);
        // The level's reference is the texture's.
        assert_eq!(e.vcall(texture, 4, &[]).u32(), 3);
        assert_eq!(e.vcall(level, 8, &[]).u32(), 2);
        let locked = e.mem.alloc(8);
        assert_eq!(e.vcall(level, 4 * 13, &[locked, 0, 0]).u32(), D3D_OK);
        assert_eq!(e.mem.u32(locked), 8, "4 pixels of 2 bytes");
        let bits = e.mem.u32(locked + 4);
        e.mem.set_u32(bits + 12, 0x1234_5678);
        assert_eq!(
            e.vcall(level, 4 * 13, &[locked, 0, 0]).u32(),
            D3DERR_INVALIDCALL
        );
        assert_eq!(e.vcall(level, 4 * 14, &[]).u32(), D3D_OK);
        assert_eq!(e.vcall(level, 4 * 14, &[]).u32(), D3DERR_INVALIDCALL);
        let rect = e.mem.alloc(16);
        e.mem.set_u32(rect, 2);
        e.mem.set_u32(rect + 4, 1);
        assert_eq!(
            e.vcall(texture, 4 * 19, &[0, locked, rect, 0]).u32(),
            D3D_OK
        );
        // The rectangle starts 12 bytes in (row 1 of 8 bytes, pixel 2 of 2 bytes): the word
        // written there through the first lock (the lock's address itself is the runtime's).
        let at = e.mem.u32(locked + 4);
        assert_eq!(e.mem.u32(at), 0x1234_5678);
        assert_eq!(e.vcall(texture, 4 * 20, &[0]).u32(), D3D_OK);
        assert_eq!(e.vcall(level, 4 * 11, &[0, slot]).u32(), E_NOINTERFACE);
        // DXT1: blocks of 4 by 4 pixels, 8 bytes.
        let hr = e
            .vcall(device, 4 * 23, &[8, 8, 1, 0, dxt1, 1, out, 0])
            .u32();
        assert_eq!(hr, D3D_OK);
        let compressed = e.mem.u32(out);
        assert_eq!(
            e.vcall(compressed, 4 * 19, &[0, locked, 0, 0]).u32(),
            D3D_OK
        );
        assert_eq!(e.mem.u32(locked), 16);
        // Refusals: an unsupported format, too many levels, dynamic managed, a shared handle.
        let refused = |e: &mut Engine, words: [u32; 8]| e.vcall(device, 4 * 23, &words).u32();
        assert_eq!(
            refused(&mut e, [4, 4, 0, 0, 0x16, 1, out, 0]),
            D3DERR_INVALIDCALL
        );
        assert_eq!(
            refused(&mut e, [4, 4, 4, 0, 0x17, 1, out, 0]),
            D3DERR_INVALIDCALL
        );
        assert_eq!(
            refused(&mut e, [4, 4, 1, 0x200, 0x17, 1, out, 0]),
            D3DERR_INVALIDCALL
        );
        assert_eq!(
            refused(&mut e, [4, 4, 1, 0, 0x17, 1, out, 4]),
            D3DERR_INVALIDCALL
        );
        // The textures hold the device; releasing them lets it go.
        assert_eq!(e.vcall(texture, 8, &[]).u32(), 1);
        assert_eq!(e.vcall(texture, 8, &[]).u32(), 0);
        assert_eq!(e.vcall(compressed, 8, &[]).u32(), 0);
        assert!(device!(e).device_objects.textures.is_empty());
        assert_eq!(e.vcall(device, 8, &[]).u32(), 0);
    }

    /// The Display menu's brightness (004dcef0) sets the gamma ramp through slot 21 (+0x54);
    /// without it the game's main loop stopped on the open slot (the maintainer's freeze).
    #[test]
    fn the_gamma_ramp_is_kept() {
        let (mut e, d3d, present) = d3d();
        let out = e.mem.alloc(4);
        e.vcall(d3d, CREATE_DEVICE, &[0, 1, 0x11c, 0x40, present, out]);
        let device = e.mem.u32(out);
        let ramp = e.mem.alloc(0x600);
        for i in 0..0x300 {
            e.mem.set_u16(ramp + 2 * i, (i * 85) as u16);
        }
        assert_eq!(e.vcall(device, 4 * 21, &[0, 1, ramp]).u32(), D3D_OK);
        let kept = super::device(&mut e, Ptr::new(device))
            .gamma_ramp
            .clone()
            .unwrap();
        assert_eq!(kept.len(), 0x300);
        assert_eq!(kept[0x201], (0x201u32 * 85) as u16);
    }

    #[test]
    fn states_are_kept_and_textures_held_by_their_samplers() {
        let (mut e, d3d, present) = d3d();
        let out = e.mem.alloc(4);
        e.vcall(d3d, CREATE_DEVICE, &[0, 1, 0x11c, 0x40, present, out]);
        let device = e.mem.u32(out);
        let value = e.mem.alloc(4);
        assert_eq!(e.vcall(device, 4 * 57, &[7, 1]).u32(), D3D_OK);
        assert_eq!(e.vcall(device, 4 * 58, &[7, value]).u32(), D3D_OK);
        assert_eq!(e.mem.u32(value), 1);
        assert_eq!(e.vcall(device, 4 * 67, &[1, 1, 4]).u32(), D3D_OK);
        assert_eq!(e.vcall(device, 4 * 66, &[1, 1, value]).u32(), D3D_OK);
        assert_eq!(e.mem.u32(value), 4);
        assert_eq!(
            e.vcall(device, 4 * 67, &[8, 1, 4]).u32(),
            D3DERR_INVALIDCALL
        );
        assert_eq!(e.vcall(device, 4 * 69, &[257, 5, 2]).u32(), D3D_OK);
        assert_eq!(e.vcall(device, 4 * 68, &[257, 5, value]).u32(), D3D_OK);
        assert_eq!(e.mem.u32(value), 2);
        assert_eq!(
            e.vcall(device, 4 * 69, &[16, 5, 2]).u32(),
            D3DERR_INVALIDCALL
        );
        assert_eq!(e.vcall(device, 4 * 107, &[0]).u32(), D3D_OK);
        // TEST DATA: the table accepts R5G6B5 (0x17) textures with usage 0.
        let table = test_table() + "\nDomain\td1\t17\nCheckDeviceFormat\t0 1 16 0 3\t0\t\td1";
        d3d9::load_adapter_table(&mut e, &table);
        e.vcall(device, 4 * 23, &[4, 4, 1, 0, 0x17, 1, out, 0]);
        let texture = e.mem.u32(out);
        assert_eq!(e.vcall(device, 4 * 65, &[0, texture]).u32(), D3D_OK);
        assert_eq!(e.vcall(device, 4 * 64, &[0, value]).u32(), D3D_OK);
        assert_eq!(e.mem.u32(value), texture);
        // The program's reference and GetTexture's: the sampler's binding is not counted (the
        // retail runtime, `d3d9-capture device refs`, 2026-10-27); released to 0 while bound,
        // the texture lives on until the sampler lets go.
        assert_eq!(e.vcall(texture, 8, &[]).u32(), 1);
        assert_eq!(e.vcall(texture, 8, &[]).u32(), 0);
        assert!(!device!(e).device_objects.textures.is_empty());
        assert_eq!(e.vcall(device, 4 * 65, &[0, 0]).u32(), D3D_OK);
        assert!(device!(e).device_objects.textures.is_empty());
        assert_eq!(e.vcall(device, 4 * 64, &[0, value]).u32(), D3D_OK);
        assert_eq!(e.mem.u32(value), 0);
    }

    #[test]
    fn buffers_shaders_and_declarations_are_made_bound_and_released() {
        let (mut e, d3d, present) = d3d();
        let out = e.mem.alloc(4);
        e.vcall(d3d, CREATE_DEVICE, &[0, 1, 0x11c, 0x40, present, out]);
        let device = e.mem.u32(out);
        // A vertex buffer of 64 bytes, locked twice, its description.
        assert_eq!(
            e.vcall(device, 4 * 26, &[64, 0, 0x142, 1, out, 0]).u32(),
            D3D_OK
        );
        let vb = e.mem.u32(out);
        let data = e.mem.alloc(4);
        assert_eq!(e.vcall(vb, 4 * 11, &[16, 0, data, 0]).u32(), D3D_OK);
        let bits = e.mem.u32(data) - 16;
        assert_eq!(e.vcall(vb, 4 * 11, &[0, 0, data, 0]).u32(), D3D_OK);
        assert_eq!(e.mem.u32(data), bits);
        assert_eq!(
            e.vcall(vb, 4 * 11, &[60, 8, data, 0]).u32(),
            D3DERR_INVALIDCALL
        );
        assert_eq!(e.vcall(vb, 4 * 12, &[]).u32(), D3D_OK);
        assert_eq!(e.vcall(vb, 4 * 12, &[]).u32(), D3D_OK);
        assert_eq!(e.vcall(vb, 4 * 12, &[]).u32(), D3DERR_INVALIDCALL);
        let desc = e.mem.alloc(24);
        assert_eq!(e.vcall(vb, 4 * 13, &[desc]).u32(), D3D_OK);
        let words: Vec<u32> = (0..6).map(|i| e.mem.u32(desc + 4 * i)).collect();
        assert_eq!(words, [100, 6, 0, 1, 64, 0x142]);
        assert_eq!(
            e.vcall(device, 4 * 26, &[64, 0x200, 0, 1, out, 0]).u32(),
            D3DERR_INVALIDCALL
        );
        // An index buffer; a wrong format is refused.
        assert_eq!(
            e.vcall(device, 4 * 27, &[12, 0, 101, 1, out, 0]).u32(),
            D3D_OK
        );
        let ib = e.mem.u32(out);
        assert_eq!(
            e.vcall(device, 4 * 27, &[12, 0, 0x16, 1, out, 0]).u32(),
            D3DERR_INVALIDCALL
        );
        // A shader: its tokens up to the end token, given back by GetFunction.
        let function = e.mem.alloc(16);
        for (i, token) in [0xfffe_0300u32, 0x1234, 0x0000_ffff].iter().enumerate() {
            e.mem.set_u32(function + 4 * i as u32, *token);
        }
        assert_eq!(e.vcall(device, 4 * 91, &[function, out]).u32(), D3D_OK);
        let vs = e.mem.u32(out);
        let size = e.mem.alloc(4);
        assert_eq!(e.vcall(vs, 4 * 4, &[0, size]).u32(), D3D_OK);
        assert_eq!(e.mem.u32(size), 12);
        // A declaration of one element and the end.
        let elements = e.mem.alloc(16);
        e.mem.set_u16(elements + 8, 0xff);
        e.mem.set_u8(elements + 12, 17);
        assert_eq!(e.vcall(device, 4 * 86, &[elements, out]).u32(), D3D_OK);
        let declaration = e.mem.u32(out);
        assert_eq!(e.vcall(declaration, 4 * 4, &[0, size]).u32(), D3D_OK);
        assert_eq!(e.mem.u32(size), 2);
        // Binding: kinds are checked, objects are held.
        assert_eq!(e.vcall(device, 4 * 100, &[0, vb, 0, 32]).u32(), D3D_OK);
        assert_eq!(
            e.vcall(device, 4 * 100, &[0, ib, 0, 32]).u32(),
            D3DERR_INVALIDCALL
        );
        assert_eq!(
            e.vcall(device, 4 * 100, &[16, vb, 0, 32]).u32(),
            D3DERR_INVALIDCALL
        );
        assert_eq!(e.vcall(device, 4 * 104, &[ib]).u32(), D3D_OK);
        assert_eq!(e.vcall(device, 4 * 104, &[vb]).u32(), D3DERR_INVALIDCALL);
        assert_eq!(e.vcall(device, 4 * 92, &[vs]).u32(), D3D_OK);
        assert_eq!(e.vcall(device, 4 * 107, &[vs]).u32(), D3DERR_INVALIDCALL);
        assert_eq!(e.vcall(device, 4 * 87, &[declaration]).u32(), D3D_OK);
        let got = e.mem.alloc(12);
        assert_eq!(
            e.vcall(device, 4 * 101, &[0, got, got + 4, got + 8]).u32(),
            D3D_OK
        );
        assert_eq!((e.mem.u32(got), e.mem.u32(got + 8)), (vb, 32));
        // GetStreamSource's reference and the program's; the stream's binding is not counted.
        assert_eq!(e.vcall(vb, 8, &[]).u32(), 1, "the program's");
        assert_eq!(e.vcall(vb, 8, &[]).u32(), 0, "none: still bound");
        assert!(device!(e).device_objects.resources_hold(vb));
        // Constants.
        let four = e.mem.alloc(32);
        e.mem.set_u32(four + 16, 7);
        assert_eq!(
            e.vcall(device, 4 * 94, &[255, four, 2]).u32(),
            D3DERR_INVALIDCALL
        );
        assert_eq!(e.vcall(device, 4 * 109, &[3, four, 2]).u32(), D3D_OK);
        let back = e.mem.alloc(32);
        assert_eq!(e.vcall(device, 4 * 110, &[4, back, 1]).u32(), D3D_OK);
        assert_eq!(e.mem.u32(back), 7);
        // Unbinding releases: the stream's buffer goes.
        assert_eq!(e.vcall(device, 4 * 100, &[0, 0, 0, 0]).u32(), D3D_OK);
        assert!(!device!(e).device_objects.resources_hold(vb));
    }

    /// Binding what a stream, the indices, a shader or a sampler holds already (the calls that
    /// skip looking the object up) changes only a stream's offset and stride: the object stays
    /// held once, so it goes at the one unbinding after the program let go of it, and a wrong
    /// kind is still refused.
    #[test]
    fn binding_again_what_is_bound_changes_nothing_but_the_stream_offset() {
        let (mut e, d3d, present) = d3d();
        let out = e.mem.alloc(4);
        e.vcall(d3d, CREATE_DEVICE, &[0, 1, 0x11c, 0x40, present, out]);
        let device = e.mem.u32(out);
        e.vcall(device, 4 * 26, &[64, 0, 0x142, 1, out, 0]);
        let vb = e.mem.u32(out);
        e.vcall(device, 4 * 27, &[12, 0, 101, 1, out, 0]);
        let ib = e.mem.u32(out);
        let function = e.mem.alloc(16);
        for (i, token) in [0xfffe_0300u32, 0x1234, 0x0000_ffff].iter().enumerate() {
            e.mem.set_u32(function + 4 * i as u32, *token);
        }
        e.vcall(device, 4 * 91, &[function, out]);
        let vs = e.mem.u32(out);
        let table = test_table() + "\nDomain\td1\t17\nCheckDeviceFormat\t0 1 16 0 3\t0\t\td1";
        d3d9::load_adapter_table(&mut e, &table);
        e.vcall(device, 4 * 23, &[4, 4, 1, 0, 0x17, 1, out, 0]);
        let texture = e.mem.u32(out);
        for _ in 0..3 {
            assert_eq!(e.vcall(device, 4 * 100, &[2, vb, 0, 32]).u32(), D3D_OK);
            assert_eq!(e.vcall(device, 4 * 104, &[ib]).u32(), D3D_OK);
            assert_eq!(e.vcall(device, 4 * 92, &[vs]).u32(), D3D_OK);
            assert_eq!(e.vcall(device, 4 * 65, &[1, texture]).u32(), D3D_OK);
        }
        // Again with another offset and stride: kept for GetStreamSource.
        assert_eq!(e.vcall(device, 4 * 100, &[2, vb, 8, 16]).u32(), D3D_OK);
        let got = e.mem.alloc(12);
        assert_eq!(
            e.vcall(device, 4 * 101, &[2, got, got + 4, got + 8]).u32(),
            D3D_OK
        );
        assert_eq!(
            (e.mem.u32(got), e.mem.u32(got + 4), e.mem.u32(got + 8)),
            (vb, 8, 16)
        );
        assert_eq!(e.vcall(vb, 8, &[]).u32(), 1, "GetStreamSource's reference");
        // A wrong kind is refused as before, and leaves what is bound.
        assert_eq!(
            e.vcall(device, 4 * 100, &[2, ib, 0, 32]).u32(),
            D3DERR_INVALIDCALL
        );
        assert_eq!(e.vcall(device, 4 * 104, &[vb]).u32(), D3DERR_INVALIDCALL);
        // The program lets go of each: all live on, bound once.
        for object in [vb, ib, vs, texture] {
            assert_eq!(e.vcall(object, 8, &[]).u32(), 0, "{object:#x}");
        }
        let objects = &device!(e).device_objects;
        assert!(objects.resources_hold(vb) && objects.resources_hold(ib));
        assert!(objects.resources_hold(vs) && !objects.textures.is_empty());
        // One unbinding each lets go of them.
        assert_eq!(e.vcall(device, 4 * 100, &[2, 0, 0, 0]).u32(), D3D_OK);
        assert_eq!(e.vcall(device, 4 * 104, &[0]).u32(), D3D_OK);
        assert_eq!(e.vcall(device, 4 * 92, &[0]).u32(), D3D_OK);
        assert_eq!(e.vcall(device, 4 * 65, &[1, 0]).u32(), D3D_OK);
        let objects = &device!(e).device_objects;
        assert!(!objects.resources_hold(vb) && !objects.resources_hold(ib));
        assert!(!objects.resources_hold(vs) && objects.textures.is_empty());
    }

    #[test]
    fn draws_answer_inside_a_scene_with_their_bindings() {
        let (mut e, d3d, present) = d3d();
        let out = e.mem.alloc(4);
        e.vcall(d3d, CREATE_DEVICE, &[0, 1, 0x11c, 0x40, present, out]);
        let device = e.mem.u32(out);
        e.vcall(device, 4 * 26, &[64, 0, 0x142, 1, out, 0]);
        let vb = e.mem.u32(out);
        e.vcall(device, 4 * 27, &[12, 0, 101, 1, out, 0]);
        let ib = e.mem.u32(out);
        let elements = e.mem.alloc(16);
        e.mem.set_u16(elements + 8, 0xff);
        e.mem.set_u8(elements + 12, 17);
        e.vcall(device, 4 * 86, &[elements, out]);
        let declaration = e.mem.u32(out);
        e.vcall(device, 4 * 100, &[0, vb, 0, 32]);
        e.vcall(device, 4 * 104, &[ib]);
        e.vcall(device, 4 * 87, &[declaration]);
        assert_eq!(e.vcall(device, 4 * 41, &[]).u32(), D3D_OK);
        assert_eq!(e.vcall(device, 4 * 81, &[4, 0, 2]).u32(), D3D_OK);
        assert_eq!(e.vcall(device, 4 * 82, &[4, 0, 0, 4, 0, 2]).u32(), D3D_OK);
        let held = e.mem.alloc(4);
        assert_eq!(e.vcall(device, 4 * 88, &[held]).u32(), D3D_OK);
        assert_eq!(e.mem.u32(held), declaration);
        assert_eq!(e.vcall(device, 4 * 88, &[0]).u32(), D3DERR_INVALIDCALL);
        assert_eq!(e.vcall(device, 4 * 42, &[]).u32(), D3D_OK);
        let outside = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            e.vcall(device, 4 * 81, &[4, 0, 2]);
        }));
        assert!(outside.is_err(), "a draw outside a scene stops the run");
    }

    #[test]
    fn transforms_scenes_and_clears_follow_direct3d() {
        let (mut e, d3d, present) = d3d();
        let out = e.mem.alloc(4);
        e.vcall(d3d, CREATE_DEVICE, &[0, 1, 0x11c, 0x40, present, out]);
        let device = e.mem.u32(out);
        let m = e.mem.alloc(64);
        assert_eq!(e.vcall(device, 4 * 45, &[256, m]).u32(), D3D_OK);
        assert_eq!(e.mem.u32(m), 1f32.to_bits());
        assert_eq!(e.mem.u32(m + 4), 0);
        e.mem.set_u32(m + 4, 2f32.to_bits());
        assert_eq!(e.vcall(device, 4 * 44, &[2, m]).u32(), D3D_OK);
        let back = e.mem.alloc(64);
        assert_eq!(e.vcall(device, 4 * 45, &[2, back]).u32(), D3D_OK);
        assert_eq!(e.mem.u32(back + 4), 2f32.to_bits());
        assert_eq!(e.vcall(device, 4 * 44, &[4, m]).u32(), D3DERR_INVALIDCALL);
        let rect = e.mem.alloc(16);
        assert_eq!(e.vcall(device, 4 * 76, &[rect]).u32(), D3D_OK);
        assert_eq!((e.mem.u32(rect + 8), e.mem.u32(rect + 12)), (640, 480));
        assert_eq!(e.vcall(device, 4 * 41, &[]).u32(), D3D_OK);
        assert_eq!(e.vcall(device, 4 * 41, &[]).u32(), D3DERR_INVALIDCALL);
        assert_eq!(
            e.vcall(device, 4 * 17, &[0, 0, 0, 0]).u32(),
            D3DERR_INVALIDCALL
        );
        assert_eq!(e.vcall(device, 4 * 42, &[]).u32(), D3D_OK);
        assert_eq!(e.vcall(device, 4 * 42, &[]).u32(), D3DERR_INVALIDCALL);
        assert_eq!(e.vcall(device, 4 * 17, &[0, 0, 0, 0]).u32(), D3D_OK);
        assert_eq!(e.vcall(device, 4 * 43, &[0, 0, 1, 0, 0, 0]).u32(), D3D_OK);
        assert_eq!(
            e.vcall(device, 4 * 43, &[0, 0, 2, 0, 0, 0]).u32(),
            D3DERR_INVALIDCALL
        );
        assert_eq!(
            e.vcall(device, 4 * 43, &[1, 0, 1, 0, 0, 0]).u32(),
            D3DERR_INVALIDCALL
        );
        assert_eq!(e.vcall(device, 4 * 3, &[]).u32(), D3D_OK);
    }

    /// `DrawPrimitiveUP` as the retail runtime answered it (`d3d9_resource::draw_primitive_up`):
    /// refused outside a scene until one has succeeded in a scene, then taken anywhere; stride
    /// 0 refused; stream 0 left as measured.
    #[test]
    fn draws_from_memory_answer_and_leave_stream_zero_as_measured() {
        let (mut e, d3d, present) = d3d();
        let out = e.mem.alloc(4);
        e.vcall(d3d, CREATE_DEVICE, &[0, 1, 0x11c, 0x40, present, out]);
        let device = e.mem.u32(out);
        let vertices = e.mem.alloc(0x90);
        let stream = |e: &mut Engine| {
            let words = e.mem.alloc(12);
            assert_eq!(
                e.vcall(device, 4 * 101, &[0, words, words + 4, words + 8])
                    .u32(),
                D3D_OK
            );
            let held = e.mem.u32(words);
            if held != 0 {
                e.vcall(held, 8, &[]);
            }
            (held, e.mem.u32(words + 4), e.mem.u32(words + 8))
        };
        // Without an FVF or a declaration the run stops.
        let bare = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            e.vcall(device, 4 * 83, &[5, 2, vertices, 0x18]);
        }));
        assert!(bare.is_err());
        e.vcall(device, 4 * 89, &[0x104]);
        e.vcall(device, 4 * 26, &[0x90, 8, 0x104, 0, out, 0]);
        let vb = e.mem.u32(out);
        // Outside a scene on a fresh device: refused, stream 0 emptied with the stride kept.
        e.vcall(device, 4 * 100, &[0, vb, 0x18, 0x18]);
        assert_eq!(
            e.vcall(device, 4 * 83, &[5, 2, vertices, 0x18]).u32(),
            D3DERR_INVALIDCALL
        );
        assert_eq!(stream(&mut e), (0, 0, 0x18));
        // An empty scene does not change that.
        e.vcall(device, 4 * 41, &[]);
        e.vcall(device, 4 * 42, &[]);
        assert_eq!(
            e.vcall(device, 4 * 83, &[5, 2, vertices, 0x18]).u32(),
            D3DERR_INVALIDCALL
        );
        // In a scene: taken whatever the type, count and pointer; stream 0 emptied.
        e.vcall(device, 4 * 41, &[]);
        e.vcall(device, 4 * 100, &[0, vb, 0x18, 0x18]);
        for (kind, count, data) in [
            (5u32, 2u32, vertices),
            (2, 6, vertices),
            (0, 2, vertices),
            (9, 2, vertices),
            (5, 0, vertices),
            (5, 2, 0),
        ] {
            assert_eq!(
                e.vcall(device, 4 * 83, &[kind, count, data, 0x18]).u32(),
                D3D_OK
            );
        }
        assert_eq!(stream(&mut e), (0, 0, 0));
        // Stride 0: refused, the buffer kept with offset and stride 0.
        e.vcall(device, 4 * 100, &[0, vb, 0x18, 0x18]);
        assert_eq!(
            e.vcall(device, 4 * 83, &[5, 2, vertices, 0]).u32(),
            D3DERR_INVALIDCALL
        );
        assert_eq!(stream(&mut e), (vb, 0, 0));
        e.vcall(device, 4 * 42, &[]);
        // After a draw from memory in a scene, taken outside one too.
        assert_eq!(
            e.vcall(device, 4 * 83, &[5, 2, vertices, 0x18]).u32(),
            D3D_OK
        );
        assert_eq!(stream(&mut e), (0, 0, 0));
        // The stream's buffer was released by the draws: only the creator's reference is left.
        assert_eq!(e.vcall(vb, 8, &[]).u32(), 0);
    }

    /// The fixed-function states and the copies as the retail runtime answered them
    /// (`set_material` .. `stretch_rect`).
    #[test]
    fn lights_materials_and_copies_answer_as_measured() {
        let (mut e, d3d, present) = d3d();
        let out = e.mem.alloc(4);
        e.vcall(d3d, CREATE_DEVICE, &[0, 1, 0x11c, 0x40, present, out]);
        let device = e.mem.u32(out);
        let material = e.mem.alloc(68);
        assert_eq!(e.vcall(device, 4 * 49, &[material]).u32(), D3D_OK);
        let light = e.mem.alloc(104);
        e.mem.set_u32(light, 4); // a type the runtime took anyway
        assert_eq!(e.vcall(device, 4 * 51, &[3, light]).u32(), D3D_OK);
        assert_eq!(e.vcall(device, 4 * 53, &[3, 2]).u32(), D3D_OK);
        assert_eq!(e.vcall(device, 4 * 53, &[7, 1]).u32(), D3D_OK);
        let d = device_state(&e, device);
        assert_eq!(d.lights[&7], default_light());
        assert_eq!(d.lights[&3][0], 4);
        assert!(d.enabled_lights.contains(&3) && d.enabled_lights.contains(&7));
        assert_eq!(e.vcall(device, 4 * 53, &[7, 0]).u32(), D3D_OK);
        assert!(!device_state(&e, device).enabled_lights.contains(&7));
        for null in [4 * 49, 4 * 51] {
            let stopped = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                let args: &[u32] = if null == 4 * 49 { &[0] } else { &[0, 0] };
                e.vcall(device, null, args);
            }));
            assert!(stopped.is_err(), "a null pointer faulted the runtime");
        }
        // Software vertex processing: refused on a hardware device unless FALSE.
        assert_eq!(e.vcall(device, 4 * 77, &[1]).u32(), D3DERR_INVALIDCALL);
        assert_eq!(e.vcall(device, 4 * 77, &[0]).u32(), D3D_OK);
        let mixed = {
            e.vcall(d3d, CREATE_DEVICE, &[0, 1, 0x11c, 0x80, present, out]);
            e.mem.u32(out)
        };
        assert_eq!(e.vcall(mixed, 4 * 77, &[2]).u32(), D3D_OK);
        assert!(device_state(&e, mixed).software_vertex_processing);
        // TEST DATA: A8R8G8B8 (0x15) textures and render targets accepted.
        let table = test_table()
            + "\nDomain\td1\t15\nCheckDeviceFormat\t0 1 16 0 3\t0\t\td1"
            + "\nCheckDeviceFormat\t0 1 16 1 3\t0\t\td1"
            + "\nCheckDeviceFormat\t0 1 16 1 1\t0\t\td1"
            + "\nDomain\td2\t0\nCheckDeviceMultiSampleType\t0 1 15 1\t0/1\t\td2";
        d3d9::load_adapter_table(&mut e, &table);
        let texture = |e: &mut Engine, levels: u32, usage: u32, pool: u32| {
            let hr = e
                .vcall(device, 4 * 23, &[64, 64, levels, usage, 0x15, pool, out, 0])
                .u32();
            assert_eq!(hr, D3D_OK);
            e.mem.u32(out)
        };
        let system = texture(&mut e, 0, 0, 2);
        let default = texture(&mut e, 1, 0, 0);
        let managed = texture(&mut e, 0, 0, 1);
        assert_eq!(e.vcall(device, 4 * 31, &[system, default]).u32(), D3D_OK);
        assert_eq!(
            e.vcall(device, 4 * 31, &[default, default]).u32(),
            D3DERR_INVALIDCALL
        );
        assert_eq!(
            e.vcall(device, 4 * 31, &[system, managed]).u32(),
            D3DERR_INVALIDCALL
        );
        let level = |e: &mut Engine, t: u32| {
            assert_eq!(e.vcall(t, 4 * 18, &[0, out]).u32(), D3D_OK);
            let s = e.mem.u32(out);
            e.vcall(s, 8, &[]);
            s
        };
        let target = |e: &mut Engine| {
            assert_eq!(
                e.vcall(device, 4 * 28, &[64, 64, 0x15, 0, 0, 0, out, 0])
                    .u32(),
                D3D_OK
            );
            e.mem.u32(out)
        };
        let (a, b) = (target(&mut e), target(&mut e));
        let render_texture = texture(&mut e, 1, 1, 0);
        let rt_level = level(&mut e, render_texture);
        let default_level = level(&mut e, default);
        let system_level = level(&mut e, system);
        let stretch = |e: &mut Engine, from: u32, to: u32| {
            e.vcall(device, 4 * 34, &[from, 0, to, 0, 2]).u32()
        };
        assert_eq!(stretch(&mut e, a, b), D3D_OK);
        assert_eq!(stretch(&mut e, a, rt_level), D3D_OK);
        assert_eq!(stretch(&mut e, default_level, a), D3D_OK);
        assert_eq!(stretch(&mut e, a, default_level), D3DERR_INVALIDCALL);
        assert_eq!(stretch(&mut e, system_level, a), D3DERR_INVALIDCALL);
        assert_eq!(stretch(&mut e, a, a), D3DERR_INVALIDCALL);
        assert_eq!(stretch(&mut e, a, 0), D3DERR_INVALIDCALL);
    }

    fn device_state(e: &Engine, device: u32) -> &Device {
        &device_ref!(e).device_objects.devices[&device]
    }

    /// The test table with what the offscreen, fill and query calls ask: `CheckDeviceFormat` of
    /// an X8R8G8B8 plain surface (usage 0, `D3DRTYPE_SURFACE`) and the query types this machine's
    /// device took (`d3d9-capture device queries`, 2026-10-27: event, occlusion and a few more).
    fn extra_device() -> (Engine, u32) {
        device_with(
            "\nDomain\td1\t15,16\nCheckDeviceFormat\t0 1 16 0 1\t0\t\td1\
             \nCreateQuery\t1\t8876086a\nCreateQuery\t8\t0\nCreateQuery\t9\t0\nCreateQuery\ta\t0",
            0x40,
        )
    }

    /// A modelled device (adapter 0, HAL, windowed 640x480 on window 0x11c, `behavior` its
    /// creation flags) over the test table with the lines `extra` added (each after a newline).
    pub(crate) fn device_with(extra: &str, behavior: u32) -> (Engine, u32) {
        let mut e = Engine::new();
        let table = test_table() + extra;
        d3d9::load_adapter_table(&mut e, &table);
        let create = d3d9::proc_address(&mut e, b"Direct3DCreate9").unwrap();
        let d3d = e.call(create, &[d3d9::D3D_SDK_VERSION]).u32();
        let present = e.mem.alloc(PRESENT_SIZE);
        e.mem.set_u32(present + PP_WIDTH, 640);
        e.mem.set_u32(present + PP_HEIGHT, 480);
        e.mem.set_u32(present + PP_WINDOW, 0x11c);
        e.mem.set_u32(present + PP_WINDOWED, 1);
        let out = e.mem.alloc(4);
        let hr = e
            .vcall(d3d, CREATE_DEVICE, &[0, 1, 0x11c, behavior, present, out])
            .u32();
        assert_eq!(hr, D3D_OK);
        let device = e.mem.u32(out);
        (e, device)
    }

    /// Not a check: the host heap allocations of the device's per-frame calls as the game makes
    /// them (through its vtable), for the frame-rate work (`cargo test --release -p engine-platform
    /// --lib allocations_per -- --ignored --nocapture`). The real device's side is measured in
    /// `d3d9real`.
    #[test]
    #[ignore]
    fn allocations_per_operation_of_the_device() {
        use crate::alloc_count::{per_run, row};
        let (mut e, device) = extra_device();
        let table = test_table()
            + "\nDomain\td1\t15,16\nCheckDeviceFormat\t0 1 16 0 1\t0\t\td1"
            + "\nCreateQuery\t1\t8876086a\nCreateQuery\t8\t0\nCreateQuery\t9\t0\nCreateQuery\ta\t0"
            + "\nDomain\td2\t17\nCheckDeviceFormat\t0 1 16 0 3\t0\t\td2";
        d3d9::load_adapter_table(&mut e, &table);
        let out = e.mem.alloc(4);
        let scratch = e.mem.alloc(0x400);
        e.vcall(device, 4 * 23, &[4, 4, 1, 0, 0x17, 1, out, 0]);
        let texture = e.mem.u32(out);
        e.vcall(device, 4 * 26, &[0x900, 0x208, 0, 0, out, 0]);
        let vb = e.mem.u32(out);
        e.vcall(device, 4 * 27, &[0x300, 0x208, 101, 0, out, 0]);
        let ib = e.mem.u32(out);
        let elements = e.mem.alloc(16);
        e.mem.set_u16(elements + 8, 0xff);
        e.mem.set_u8(elements + 12, 17);
        e.vcall(device, 4 * 86, &[elements, out]);
        let declaration = e.mem.u32(out);
        let function = e.mem.alloc(16);
        for (i, token) in [0xfffe_0300u32, 0x1234, 0x0000_ffff].iter().enumerate() {
            e.mem.set_u32(function + 4 * i as u32, *token);
        }
        e.vcall(device, 4 * 91, &[function, out]);
        let vs = e.mem.u32(out);
        e.vcall(device, 0x1d8, &[8, out]);
        let query = e.mem.u32(out);
        let viewport = e.mem.alloc(24);
        for (i, w) in [0u32, 0, 640, 480, 0, 1f32.to_bits()].iter().enumerate() {
            e.mem.set_u32(viewport + 4 * i as u32, *w);
        }
        let matrix = e.mem.alloc(64);
        let constants = e.mem.alloc(16 * 64);
        let rect = e.mem.alloc(16);
        e.mem.set_u32(rect + 8, 64);
        e.mem.set_u32(rect + 12, 64);
        let iid = e.mem.alloc(16);
        e.mem.write(iid, &IID_DEVICE9);
        e.vcall(device, 4 * 41, &[]);
        let mut calls: Vec<(&str, u32, Vec<u32>)> = vec![
            ("SetRenderState", device, vec![4 * 57, 7, 1]),
            ("SetTextureStageState", device, vec![4 * 67, 1, 1, 4]),
            ("SetSamplerState", device, vec![4 * 69, 0, 5, 2]),
            ("SetTransform", device, vec![4 * 44, 256, matrix]),
            ("SetViewport", device, vec![4 * 47, viewport]),
            ("SetScissorRect", device, vec![4 * 75, rect]),
            (
                "SetTexture (bind, unbind)",
                device,
                vec![4 * 65, 0, texture],
            ),
            ("SetStreamSource", device, vec![4 * 100, 0, vb, 0, 32]),
            ("SetIndices", device, vec![4 * 104, ib]),
            ("SetVertexDeclaration", device, vec![4 * 87, declaration]),
            ("SetVertexShader", device, vec![4 * 92, vs]),
            (
                "SetVertexShaderConstantF, 16",
                device,
                vec![4 * 94, 0, constants, 16],
            ),
            (
                "SetPixelShaderConstantF, 4",
                device,
                vec![4 * 109, 0, constants, 4],
            ),
            (
                "DrawIndexedPrimitive",
                device,
                vec![4 * 82, 4, 0, 0, 4, 0, 2],
            ),
            ("DrawPrimitive", device, vec![4 * 81, 4, 0, 2]),
            ("Clear", device, vec![4 * 43, 0, 0, 1, 0, 0, 0]),
            ("QueryInterface + Release", device, vec![0, iid, out]),
            (
                "vertex buffer Lock + Unlock",
                vb,
                vec![4 * 11, 0, 0x40, out, 0x2000],
            ),
            ("query Issue(END) + GetData", query, vec![0x18, 1]),
        ];
        for (what, object, words) in calls.iter_mut() {
            let (slot, args) = (words[0], words[1..].to_vec());
            let object = *object;
            let what = *what;
            row(
                &format!("IDirect3DDevice9 {what}"),
                per_run(1000, || {
                    e.vcall(object, slot, &args);
                    match what {
                        "SetTexture (bind, unbind)" => {
                            e.vcall(device, 4 * 65, &[0, 0]);
                        }
                        "QueryInterface + Release" => {
                            e.vcall(device, 8, &[]);
                        }
                        "vertex buffer Lock + Unlock" => {
                            e.vcall(vb, 4 * 12, &[]);
                        }
                        "query Issue(END) + GetData" => {
                            e.vcall(query, 0x1c, &[scratch, 4, 1]);
                        }
                        _ => {}
                    }
                }),
            );
        }
        row(
            "IDirect3DDevice9 EndScene + Present + BeginScene",
            per_run(1000, || {
                e.vcall(device, 4 * 42, &[]);
                e.vcall(device, 4 * 17, &[0, 0, 0, 0]);
                e.vcall(device, 4 * 41, &[]);
            }),
        );
    }

    /// `CreateOffscreenPlainSurface` (+0x90) and `ColorFill` (+0x8C) as the retail runtime
    /// answered (`d3d9-capture device extra`, 2026-10-27).
    #[test]
    fn offscreen_surfaces_and_fills_answer_as_measured() {
        let (mut e, device) = extra_device();
        let out = e.mem.alloc(4);
        let make = |e: &mut Engine, w: u32, pool: u32| {
            let hr = e.vcall(device, 0x90, &[w, 64, 0x16, pool, out, 0]).u32();
            (hr, e.mem.u32(out))
        };
        assert_eq!(make(&mut e, 64, 1).0, D3DERR_INVALIDCALL, "managed");
        assert_eq!(make(&mut e, 0, 2).0, D3DERR_INVALIDCALL, "a zero side");
        let (hr, system) = make(&mut e, 64, 2);
        assert_eq!(hr, D3D_OK);
        let (hr, default) = make(&mut e, 64, 0);
        assert_eq!(hr, D3D_OK);
        // Outside the default pool a fill is refused; X8R8G8B8 takes the colour with byte 3 zero.
        assert_eq!(
            e.vcall(device, 0x8c, &[system, 0, 0xff11_2233]).u32(),
            D3DERR_INVALIDCALL
        );
        assert_eq!(
            e.vcall(device, 0x8c, &[default, 0, 0xff11_2233]).u32(),
            D3D_OK
        );
        let locked = e.mem.alloc(8);
        assert_eq!(e.vcall(default, 4 * 13, &[locked, 0, 0]).u32(), D3D_OK);
        let bits = e.mem.u32(locked + 4);
        assert_eq!(e.mem.u32(bits), 0x0011_2233);
        assert_eq!(e.vcall(default, 4 * 14, &[]).u32(), D3D_OK);
        // A rectangle inside fills; past the edge is refused.
        let rect = e.mem.alloc(16);
        for (i, v) in [8u32, 8, 16, 16].iter().enumerate() {
            e.mem.set_u32(rect + 4 * i as u32, *v);
        }
        assert_eq!(
            e.vcall(device, 0x8c, &[default, rect, 0x0044_5566]).u32(),
            D3D_OK
        );
        e.mem.set_u32(rect + 8, 100);
        assert_eq!(
            e.vcall(device, 0x8c, &[default, rect, 0]).u32(),
            D3DERR_INVALIDCALL
        );
        // GetRenderTargetData (+0x80): a system-memory destination of the back buffer's size and
        // format is taken (the pixels are the real device's); another size or pool is refused.
        let back = device_state(&e, device).render_targets[0];
        let (_, small) = make(&mut e, 32, 2);
        assert_eq!(
            e.vcall(device, 0x80, &[back, small]).u32(),
            D3DERR_INVALIDCALL
        );
        assert_eq!(
            e.vcall(device, 0x80, &[back, default]).u32(),
            D3DERR_INVALIDCALL
        );
        assert_eq!(
            e.vcall(device, 0x80, &[system, system]).u32(),
            D3DERR_INVALIDCALL
        );
    }

    /// `CreateQuery` (+0x1D8) and the event query's life as the retail runtime answered
    /// (2026-10-27); without `NV_D3D9=real` an ended event query is done at once.
    #[test]
    fn queries_answer_as_measured() {
        let (mut e, device) = extra_device();
        assert_eq!(e.vcall(device, 0x1d8, &[1, 0]).u32(), 0x8876_086a);
        assert_eq!(e.vcall(device, 0x1d8, &[8, 0]).u32(), D3D_OK);
        let out = e.mem.alloc(4);
        assert_eq!(e.vcall(device, 0x1d8, &[8, out]).u32(), D3D_OK);
        let q = e.mem.u32(out);
        assert_eq!(e.vcall(q, 0x10, &[]).u32(), 8, "GetType");
        assert_eq!(e.vcall(q, 0x14, &[]).u32(), 4, "GetDataSize");
        let data = e.mem.alloc(8);
        e.mem.set_u32(data, 0xaaaa_aaaa);
        // Never issued: S_OK and dddddddd.
        assert_eq!(e.vcall(q, 0x1c, &[data, 4, 1]).u32(), D3D_OK);
        assert_eq!(e.mem.u32(data), 0xdddd_dddd);
        assert_eq!(e.vcall(q, 0x18, &[2]).u32(), D3D_OK, "BEGIN");
        e.mem.set_u32(data, 0xaaaa_aaaa);
        assert_eq!(
            e.vcall(q, 0x1c, &[data, 4, 1]).u32(),
            1,
            "S_FALSE after BEGIN"
        );
        assert_eq!(e.mem.u32(data), 0xaaaa_aaaa);
        assert_eq!(e.vcall(q, 0x18, &[1]).u32(), D3D_OK, "END");
        assert_eq!(e.vcall(q, 0x1c, &[data, 4, 1]).u32(), D3D_OK);
        assert_eq!(e.mem.u32(data), 1);
        // Size 2 writes two bytes; size 8 zero-extends.
        e.mem.set_u32(data, 0xaaaa_aaaa);
        assert_eq!(e.vcall(q, 0x1c, &[data, 2, 0]).u32(), D3D_OK);
        assert_eq!(e.mem.u32(data), 0xaaaa_0001);
        e.mem.set_u32(data + 4, 0xaaaa_aaaa);
        assert_eq!(e.vcall(q, 0x1c, &[data, 8, 0]).u32(), D3D_OK);
        assert_eq!([e.mem.u32(data), e.mem.u32(data + 4)], [1, 0]);
        assert_eq!(e.vcall(q, 0x1c, &[0, 0, 0]).u32(), D3D_OK);
        assert_eq!(e.vcall(q, 8, &[]).u32(), 0, "Release");
    }

    /// `Reset` (+0x40) as the retail runtime answered (`d3d9-capture device reset`, 2026-10-27).
    #[test]
    fn reset_answers_as_measured() {
        let (mut e, device) = extra_device();
        let pp = e.mem.alloc(PRESENT_SIZE);
        let fill = |e: &mut Engine, count: u32, width: u32| {
            for i in 0..PRESENT_SIZE / 4 {
                e.mem.set_u32(pp + 4 * i, 0);
            }
            e.mem.set_u32(pp + PP_WIDTH, width);
            e.mem.set_u32(pp + PP_HEIGHT, 480);
            e.mem.set_u32(pp + PP_COUNT, count);
            e.mem.set_u32(pp + PP_WINDOW, 0x11c);
            e.mem.set_u32(pp + PP_WINDOWED, 1);
        };
        // States set before Reset.
        assert_eq!(e.vcall(device, 0xe4, &[7, 0]).u32(), D3D_OK);
        // A default-pool surface alive: refused, and the device needs a Reset.
        let out = e.mem.alloc(4);
        assert_eq!(
            e.vcall(device, 0x90, &[64, 64, 0x16, 0, out, 0]).u32(),
            D3D_OK
        );
        let surface = e.mem.u32(out);
        fill(&mut e, 0, 640);
        assert_eq!(e.vcall(device, 0x40, &[pp]).u32(), D3DERR_INVALIDCALL);
        assert_eq!(e.vcall(device, 0xc, &[]).u32(), D3DERR_DEVICENOTRESET);
        assert_eq!(
            e.vcall(device, 0xa4, &[]).u32(),
            D3DERR_INVALIDCALL,
            "BeginScene"
        );
        assert_eq!(
            e.vcall(device, 0x44, &[0, 0, 0, 0]).u32(),
            D3DERR_INVALIDCALL,
            "Present"
        );
        assert_eq!(e.vcall(surface, 8, &[]).u32(), 0);
        assert_eq!(
            e.vcall(device, 0xc, &[]).u32(),
            D3DERR_DEVICENOTRESET,
            "still"
        );
        // Four back buffers: refused, the count left as given.
        fill(&mut e, 4, 640);
        assert_eq!(e.vcall(device, 0x40, &[pp]).u32(), D3DERR_INVALIDCALL);
        assert_eq!(e.mem.u32(pp + PP_COUNT), 4);
        // A good Reset: defaults filled in, the device usable again, its states cleared, the
        // render target the new back buffer of the new size.
        fill(&mut e, 0, 800);
        assert_eq!(e.vcall(device, 0x40, &[pp]).u32(), D3D_OK);
        assert_eq!(e.mem.u32(pp + PP_COUNT), 1);
        assert_eq!(e.mem.u32(pp + PP_FORMAT), 0x16);
        assert_eq!(e.vcall(device, 0xc, &[]).u32(), D3D_OK);
        let d = device_state(&e, device);
        assert!(d.render_states.is_empty());
        assert_eq!(d.viewport[2..4], [800, 480]);
        let back = d.back_buffers[0];
        assert_eq!(d.render_targets[0], back);
        assert_eq!(
            device!(e).device_objects.surfaces[&back].desc[6..8],
            [800, 480]
        );
        // In a scene too; the scene ends.
        assert_eq!(e.vcall(device, 0xa4, &[]).u32(), D3D_OK);
        assert_eq!(e.vcall(device, 0x40, &[pp]).u32(), D3D_OK);
        assert_eq!(
            e.vcall(device, 0xa8, &[]).u32(),
            D3DERR_INVALIDCALL,
            "EndScene"
        );
    }

    /// A device with what a frame of the game binds (TEST DATA, not a capture): `n` textures,
    /// vertex and index buffers, vertex and pixel shaders and declarations (each a power of
    /// two), and 4 KB of shader constants (distinct floats).
    #[cfg(windows)]
    struct Scene {
        e: Engine,
        device: u32,
        textures: Vec<u32>,
        vertex_buffers: Vec<u32>,
        index_buffers: Vec<u32>,
        declarations: Vec<u32>,
        vertex_shaders: Vec<u32>,
        pixel_shaders: Vec<u32>,
        constants: u32,
    }

    #[cfg(windows)]
    fn scene(n: [u32; 4]) -> Scene {
        let (mut e, d3d, present) = d3d();
        let table = test_table() + "\nDomain\td1\t17\nCheckDeviceFormat\t0 1 16 0 3\t0\t\td1";
        d3d9::load_adapter_table(&mut e, &table);
        let out = e.mem.alloc(4);
        e.vcall(d3d, CREATE_DEVICE, &[0, 1, 0x11c, 0x40, present, out]);
        let device = e.mem.u32(out);
        let make = |e: &mut Engine, n: u32, slot: u32, args: &[u32]| -> Vec<u32> {
            (0..n)
                .map(|_| {
                    let mut words = args.to_vec();
                    words.push(out);
                    words.push(0);
                    assert_eq!(e.vcall(device, 4 * slot, &words).u32(), D3D_OK);
                    e.mem.u32(out)
                })
                .collect()
        };
        let textures = make(&mut e, n[0], 23, &[4, 4, 1, 0, 0x17, 1]);
        let vertex_buffers = make(&mut e, n[1], 26, &[64, 0, 0x142, 1]);
        let index_buffers = make(&mut e, n[1], 27, &[12, 0, 101, 1]);
        let function = |e: &mut Engine, version: u32| {
            let at = e.mem.alloc(8);
            e.mem.set_u32(at, version);
            e.mem.set_u32(at + 4, 0x0000_ffff);
            at
        };
        let shaders = |e: &mut Engine, slot: u32, version: u32| -> Vec<u32> {
            (0..n[2])
                .map(|_| {
                    let f = function(e, version);
                    assert_eq!(e.vcall(device, 4 * slot, &[f, out]).u32(), D3D_OK);
                    e.mem.u32(out)
                })
                .collect()
        };
        let vertex_shaders = shaders(&mut e, 91, 0xfffe_0300);
        let pixel_shaders = shaders(&mut e, 106, 0xffff_0300);
        let elements = e.mem.alloc(16);
        e.mem.set_u16(elements + 8, 0xff);
        e.mem.set_u8(elements + 12, 17);
        let declarations = (0..n[3])
            .map(|_| {
                assert_eq!(e.vcall(device, 4 * 86, &[elements, out]).u32(), D3D_OK);
                e.mem.u32(out)
            })
            .collect();
        let constants = e.mem.alloc(0x1100);
        for i in 0..0x440 {
            e.mem
                .set_u32(constants + 4 * i, (i as f32 * 1.5 - 7.25).to_bits());
        }
        Scene {
            e,
            device,
            textures,
            vertex_buffers,
            index_buffers,
            declarations,
            vertex_shaders,
            pixel_shaders,
            constants,
        }
    }

    /// A backend whose objects are stand-ins (`d3d9backend::stub`) for every object of the scene:
    /// the device `#0`, textures `#1000..`, vertex buffers `#2000..`, index buffers `#3000..`,
    /// declarations `#4000..`, vertex shaders `#5000..`, pixel shaders `#6000..`, their calls
    /// noted in `log`; and the scene's objects.
    #[cfg(windows)]
    fn stand_ins(
        s: &Scene,
        log: &std::sync::Arc<super::super::d3d9backend::stub::Log>,
    ) -> (super::super::d3d9backend::RealBackend, Vec<u32>) {
        use super::super::d3d9backend::stub::{install, Kind};
        let mut b = super::super::d3d9backend::RealBackend::new();
        install(&mut b, s.device, Kind::Device, 0, log);
        let mut all = Vec::new();
        for (kind, objects, base) in [
            (Kind::Texture, &s.textures, 1000),
            (Kind::VertexBuffer, &s.vertex_buffers, 2000),
            (Kind::IndexBuffer, &s.index_buffers, 3000),
            (Kind::Declaration, &s.declarations, 4000),
            (Kind::VertexShader, &s.vertex_shaders, 5000),
            (Kind::PixelShader, &s.pixel_shaders, 6000),
        ] {
            for (i, &o) in objects.iter().enumerate() {
                install(&mut b, o, kind, base + i as u32, log);
                all.push(o);
            }
        }
        (b, all)
    }

    /// Gives every object of the scene a stand-in real counterpart ([`stand_ins`]) as the
    /// engine's forwarding, carried out at once or on the render thread (`threaded`): the log of
    /// the calls that reach them, and the backend (shared, to look at afterwards).
    #[cfg(windows)]
    fn stand_in(
        s: &mut Scene,
        threaded: bool,
    ) -> (
        std::sync::Arc<super::super::d3d9backend::stub::Log>,
        std::sync::Arc<std::sync::Mutex<super::super::d3d9backend::RealBackend>>,
    ) {
        let log = std::sync::Arc::default();
        let (b, objects) = stand_ins(s, &log);
        let shared = std::sync::Arc::new(std::sync::Mutex::new(b));
        super::super::d3d9real::test_forwarding(
            &mut s.e,
            Box::new(super::super::d3d9real::SharedBackend(
                std::sync::Arc::clone(&shared),
            )),
            threaded,
            &[s.device],
            &objects,
        );
        (log, shared)
    }

    /// The groups of calls a [`draw`] makes, with how many calls each makes.
    #[cfg(windows)]
    const GROUPS: [(&str, u32); 9] = [
        ("SetVertexDeclaration", 1),
        ("SetVertexShader/SetPixelShader", 2),
        ("SetVertexShaderConstantF", 8),
        ("SetPixelShaderConstantF", 4),
        ("SetTexture", 2),
        ("SetSamplerState", 4),
        ("SetRenderState", 4),
        ("SetStreamSource/SetIndices", 2),
        ("DrawIndexedPrimitive", 1),
    ];

    /// One draw as the game's renderer makes them (TEST DATA, the shape of a lit, textured
    /// mesh's pass): declaration, shaders, eight vertex and four pixel float constant ranges,
    /// two textures, four sampler and four render states, stream 0, indices and the indexed
    /// draw: 28 calls, the [`GROUPS`] in order. `device` is the object called (the scene's
    /// device, or another object with the same slots). The objects are picked spread over the
    /// scene's (as a frame's meshes are).
    #[cfg(windows)]
    fn draw(s: &mut Scene, device: u32, i: usize) {
        for group in 0..GROUPS.len() {
            draw_group(s, device, i, group);
        }
    }

    #[cfg(windows)]
    fn draw_group(s: &mut Scene, device: u32, i: usize, group: usize) {
        let e = &mut s.e;
        let data = |k: usize| s.constants + 16 * (k % 64) as u32;
        let pick = |objects: &[u32], k: usize| objects[k.wrapping_mul(769) % objects.len()];
        let i32 = i as u32;
        match group {
            0 => {
                e.vcall(device, 4 * 87, &[pick(&s.declarations, i)]);
            }
            1 => {
                e.vcall(device, 4 * 92, &[pick(&s.vertex_shaders, i)]);
                e.vcall(device, 4 * 107, &[pick(&s.pixel_shaders, i / 2)]);
            }
            2 => {
                for (k, (start, count)) in [
                    (0, 4),
                    (4, 4),
                    (8, 3),
                    (12, 1),
                    (13, 1),
                    (20, 2),
                    (30, 1),
                    (40, 1),
                ]
                .into_iter()
                .enumerate()
                {
                    e.vcall(device, 4 * 94, &[start, data(3 * i + k), count]);
                }
            }
            3 => {
                for (k, (start, count)) in [(0, 1), (1, 1), (2, 2), (5, 1)].into_iter().enumerate()
                {
                    e.vcall(device, 4 * 109, &[start, data(5 * i + k), count]);
                }
            }
            4 => {
                e.vcall(device, 4 * 65, &[0, pick(&s.textures, i)]);
                e.vcall(device, 4 * 65, &[1, pick(&s.textures, 7 * i + 3)]);
            }
            5 => {
                e.vcall(device, 4 * 69, &[0, 1, 1 + (i32 & 1)]);
                e.vcall(device, 4 * 69, &[0, 2, 1]);
                e.vcall(device, 4 * 69, &[1, 5, 2]);
                e.vcall(device, 4 * 69, &[1, 6, 2]);
            }
            6 => {
                e.vcall(device, 4 * 57, &[7, 1]);
                e.vcall(device, 4 * 57, &[27, i32 & 1]);
                e.vcall(device, 4 * 57, &[22, 2 + (i32 & 1)]);
                e.vcall(device, 4 * 57, &[14, (i32 >> 1) & 1]);
            }
            7 => {
                e.vcall(device, 4 * 100, &[0, pick(&s.vertex_buffers, i), 0, 32]);
                e.vcall(device, 4 * 104, &[pick(&s.index_buffers, i)]);
            }
            _ => {
                e.vcall(device, 4 * 82, &[4, 0, 0, 100, 0, 50]);
            }
        }
    }

    /// The calls of one draw that reach the real device, as forwarded to a stand-in (the
    /// first draw of [`draw`]: index 0).
    #[cfg(windows)]
    const FIRST_DRAW: [&str; 28] = [
        "SetVertexDeclaration(#4000)",
        "SetVertexShader(#5000)",
        "SetPixelShader(#6000)",
        "vertex_constant_f(0, [c0e80000 c0b80000 c0880000 c0300000 bfa00000 3e800000 \
         3fe00000 40500000 40980000 40c80000 40f80000 41140000 412c0000 41440000 415c0000 \
         41740000], 4)",
        "vertex_constant_f(4, [bfa00000 3e800000 3fe00000 40500000 40980000 40c80000 \
         40f80000 41140000 412c0000 41440000 415c0000 41740000 41860000 41920000 419e0000 \
         41aa0000], 4)",
        "vertex_constant_f(8, [40980000 40c80000 40f80000 41140000 412c0000 41440000 \
         415c0000 41740000 41860000 41920000 419e0000 41aa0000], 3)",
        "vertex_constant_f(12, [412c0000 41440000 415c0000 41740000], 1)",
        "vertex_constant_f(13, [41860000 41920000 419e0000 41aa0000], 1)",
        "vertex_constant_f(20, [41b60000 41c20000 41ce0000 41da0000 41e60000 41f20000 \
         41fe0000 42050000], 2)",
        "vertex_constant_f(30, [41e60000 41f20000 41fe0000 42050000], 1)",
        "vertex_constant_f(40, [420b0000 42110000 42170000 421d0000], 1)",
        "pixel_constant_f(0, [c0e80000 c0b80000 c0880000 c0300000], 1)",
        "pixel_constant_f(1, [bfa00000 3e800000 3fe00000 40500000], 1)",
        "pixel_constant_f(2, [40980000 40c80000 40f80000 41140000 412c0000 41440000 \
         415c0000 41740000], 2)",
        "pixel_constant_f(5, [412c0000 41440000 415c0000 41740000], 1)",
        "SetTexture(0, #1000)",
        "SetTexture(1, #1003)",
        "SetSamplerState(0, 1, 0x1)",
        "SetSamplerState(0, 2, 0x1)",
        "SetSamplerState(1, 5, 0x2)",
        "SetSamplerState(1, 6, 0x2)",
        "SetRenderState(7, 0x1)",
        "SetRenderState(27, 0x0)",
        "SetRenderState(22, 0x2)",
        "SetRenderState(14, 0x0)",
        "SetStreamSource(0, #2000, 0, 32)",
        "SetIndices(#3000)",
        "DrawIndexedPrimitive(4, 0, 0, 100, 0, 50)",
    ];

    /// What reaches the real device is the game's calls as given, in the game's order, with
    /// the data behind their pointers as engine memory holds it; a call the model refuses does
    /// not reach it. Pins the forwarding against a stand-in device (`d3d9backend::stub`), carried
    /// out directly or from the render thread.
    #[cfg(windows)]
    #[test]
    fn forwarded_calls_reach_the_real_device_as_given() {
        forwarded_calls_through(false);
    }

    /// [`forwarded_calls_reach_the_real_device_as_given`] with the render thread: the same calls
    /// reach the real device in the same order, with the same data (the same digest), and the
    /// model answers the same.
    #[cfg(windows)]
    #[test]
    fn forwarded_calls_reach_the_real_device_as_given_from_the_render_thread() {
        forwarded_calls_through(true);
    }

    #[cfg(windows)]
    fn forwarded_calls_through(threaded: bool) {
        use super::super::d3d9real::test_finish;
        let mut s = scene([64, 16, 8, 8]);
        // Every call as given, repeats too (`NV_D3D9_REPEATS=keep`): what is left out by default
        // is pinned by `repeated_states_do_not_reach_the_real_device`.
        keep_repeats(&mut s.e, true);
        let (calls, backend) = stand_in(&mut s, threaded);
        let device = s.device;
        calls.record();
        assert_eq!(s.e.vcall(device, 4 * 41, &[]).u32(), D3D_OK);
        draw(&mut s, device, 0);
        test_finish(&mut s.e);
        let log = calls.take();
        assert_eq!(log[0], "BeginScene");
        let first: Vec<String> = log[1..]
            .iter()
            .map(|l| l.split_whitespace().collect::<Vec<_>>().join(" "))
            .collect();
        let expected: Vec<String> = FIRST_DRAW
            .iter()
            .map(|l| l.split_whitespace().collect::<Vec<_>>().join(" "))
            .collect();
        assert_eq!(first, expected);
        // Calls the model refuses or takes at their edges, then many draws.
        calls.record();
        let e = &mut s.e;
        let data = s.constants;
        let (t0, t1) = (s.textures[0], s.textures[1]);
        assert_eq!(
            e.vcall(device, 4 * 69, &[16, 1, 1]).u32(),
            D3DERR_INVALIDCALL
        );
        assert_eq!(e.vcall(device, 4 * 65, &[16, t0]).u32(), D3DERR_INVALIDCALL);
        assert_eq!(
            e.vcall(device, 4 * 94, &[255, data, 2]).u32(),
            D3DERR_INVALIDCALL
        );
        assert_eq!(
            e.vcall(device, 4 * 109, &[223, data, 2]).u32(),
            D3DERR_INVALIDCALL
        );
        assert_eq!(
            e.vcall(device, 4 * 94, &[3, 0, 1]).u32(),
            D3DERR_INVALIDCALL
        );
        assert_eq!(e.vcall(device, 4 * 94, &[3, data, 0]).u32(), D3D_OK);
        assert_eq!(e.vcall(device, 4 * 94, &[0, data, 256]).u32(), D3D_OK);
        assert_eq!(e.vcall(device, 4 * 109, &[0, data + 4, 224]).u32(), D3D_OK);
        assert_eq!(e.vcall(device, 4 * 96, &[2, data + 8, 3]).u32(), D3D_OK);
        assert_eq!(e.vcall(device, 4 * 98, &[1, data + 12, 5]).u32(), D3D_OK);
        assert_eq!(e.vcall(device, 4 * 111, &[15, data, 1]).u32(), D3D_OK);
        assert_eq!(e.vcall(device, 4 * 113, &[0, data, 16]).u32(), D3D_OK);
        assert_eq!(
            e.vcall(device, 4 * 113, &[1, data, 16]).u32(),
            D3DERR_INVALIDCALL
        );
        assert_eq!(e.vcall(device, 4 * 65, &[2, t1]).u32(), D3D_OK);
        assert_eq!(e.vcall(device, 4 * 65, &[2, t1]).u32(), D3D_OK);
        assert_eq!(e.vcall(device, 4 * 65, &[2, 0]).u32(), D3D_OK);
        assert_eq!(e.vcall(device, 4 * 65, &[257, t0]).u32(), D3D_OK);
        assert_eq!(e.vcall(device, 4 * 67, &[1, 1, 4]).u32(), D3D_OK);
        assert_eq!(
            e.vcall(device, 4 * 67, &[8, 1, 4]).u32(),
            D3DERR_INVALIDCALL
        );
        assert_eq!(e.vcall(device, 4 * 100, &[1, 0, 0, 0]).u32(), D3D_OK);
        assert_eq!(
            e.vcall(device, 4 * 100, &[16, 0, 0, 0]).u32(),
            D3DERR_INVALIDCALL
        );
        assert_eq!(e.vcall(device, 4 * 92, &[0]).u32(), D3D_OK);
        assert_eq!(
            e.vcall(device, 4 * 92, &[s.vertex_shaders[3]]).u32(),
            D3D_OK
        );
        // Transforms, the viewport, a material and a light, a clear and a draw from memory.
        assert_eq!(e.vcall(device, 4 * 44, &[256, data]).u32(), D3D_OK);
        assert_eq!(e.vcall(device, 4 * 44, &[3, data + 68]).u32(), D3D_OK);
        assert_eq!(
            e.vcall(device, 4 * 44, &[4, data]).u32(),
            D3DERR_INVALIDCALL
        );
        let viewport = e.mem.alloc(24);
        for (i, w) in [8, 4, 600, 400, 0.25f32.to_bits(), 1f32.to_bits()]
            .into_iter()
            .enumerate()
        {
            e.mem.set_u32(viewport + 4 * i as u32, w);
        }
        assert_eq!(e.vcall(device, 4 * 47, &[viewport]).u32(), D3D_OK);
        assert_eq!(e.vcall(device, 4 * 49, &[data + 20]).u32(), D3D_OK);
        assert_eq!(e.vcall(device, 4 * 51, &[1, data + 36]).u32(), D3D_OK);
        let rect = e.mem.alloc(16);
        e.mem.set_u32(rect + 8, 10);
        e.mem.set_u32(rect + 12, 12);
        let clear = [1, rect, 1, 0xff10_2030, 1f32.to_bits(), 0];
        assert_eq!(e.vcall(device, 4 * 43, &clear).u32(), D3D_OK);
        assert_eq!(e.vcall(device, 4 * 83, &[4, 2, data + 2, 16]).u32(), D3D_OK);
        for i in 1..200 {
            draw(&mut s, device, i);
        }
        assert_eq!(s.e.vcall(device, 4 * 42, &[]).u32(), D3D_OK);
        test_finish(&mut s.e);
        let log = calls.take();
        assert_eq!(log.len(), 22 + 199 * 28 + 1);
        assert!(!log.iter().any(|l| l.contains("unexpected")));
        let digest = log.iter().fold(0xcbf2_9ce4_8422_2325u64, |h, line| {
            line.bytes()
                .chain(*b"\n")
                .fold(h, |h, b| (h ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3))
        });
        if std::env::var_os("NV_D3D9_STUB_LOG").is_some() {
            for line in &log {
                eprintln!("{line}");
            }
        }
        assert_eq!(
            digest, 5_257_225_520_366_858_290,
            "the forwarded calls' digest"
        );
        assert_eq!(
            super::super::d3d9backend::stub::logged(&backend.lock().unwrap()),
            0
        );
        // What the model answers the game afterwards: its getters' answers and words, and the
        // objects' counts.
        let answers = model_answers(&mut s);
        if std::env::var_os("NV_D3D9_STUB_LOG").is_some() {
            eprintln!("{answers:08x?}");
        }
        let digest = answers.iter().fold(0xcbf2_9ce4_8422_2325u64, |h, &w| {
            (h ^ u64::from(w)).wrapping_mul(0x0100_0000_01b3)
        });
        assert_eq!(answers.len(), 2303);
        // The answers include the objects' addresses (GetTexture, GetStreamSource, ...), which
        // depend on what the layer allocates in the game's memory; the heap holds each block at its
        // exe pool's size (`engine_core::mem::block_bytes`), so a change there re-pins the digest.
        assert_eq!(
            digest, 6_478_585_233_084_394_086,
            "the model's answers' digest"
        );
    }

    /// By default a sampler state or a constant range set again to what it holds does not reach
    /// the real device (`skip_repeats`): the second of two identical draws sends its textures,
    /// render states, shaders, streams and the draw, not its sampler states or constants. A changed
    /// value, one register changed in a range, and a sampler state never set before still go, and
    /// the model answers the game the same as when every call goes.
    #[cfg(windows)]
    #[test]
    fn repeated_states_do_not_reach_the_real_device() {
        use super::super::d3d9real::test_finish;
        let mut s = scene([64, 16, 8, 8]);
        let (calls, _backend) = stand_in(&mut s, false);
        let device = s.device;
        assert_eq!(s.e.vcall(device, 4 * 41, &[]).u32(), D3D_OK);
        draw(&mut s, device, 0);
        test_finish(&mut s.e);
        calls.record();
        draw(&mut s, device, 0);
        test_finish(&mut s.e);
        let log = calls.take();
        let names: Vec<&str> = log
            .iter()
            .map(|l| l.split('(').next().unwrap_or(""))
            .collect();
        assert!(!names.contains(&"SetSamplerState"), "{log:?}");
        assert!(!names.iter().any(|n| n.contains("constant")), "{log:?}");
        for kept in [
            "SetVertexDeclaration",
            "SetVertexShader",
            "SetPixelShader",
            "SetTexture",
            "SetRenderState",
            "SetStreamSource",
            "SetIndices",
            "DrawIndexedPrimitive",
        ] {
            assert!(names.contains(&kept), "{kept} missing: {log:?}");
        }
        assert_eq!(log.len(), 28 - 4 - 12, "{log:?}");
        // A changed sampler value goes, and so does the same value set again after it changed.
        calls.record();
        let e = &mut s.e;
        assert_eq!(e.vcall(device, 4 * 69, &[0, 1, 7]).u32(), D3D_OK);
        assert_eq!(e.vcall(device, 4 * 69, &[0, 1, 7]).u32(), D3D_OK);
        assert_eq!(e.vcall(device, 4 * 69, &[0, 1, 1]).u32(), D3D_OK);
        // A sampler state the device never had set goes.
        assert_eq!(e.vcall(device, 4 * 69, &[2, 3, 1]).u32(), D3D_OK);
        // One register changed in a range: the whole range goes, as given.
        let data = e.mem.alloc(32);
        for i in 0..8 {
            e.mem.set_u32(data + 4 * i, 0x3f80_0000 + i);
        }
        assert_eq!(e.vcall(device, 4 * 94, &[60, data, 2]).u32(), D3D_OK);
        assert_eq!(e.vcall(device, 4 * 94, &[60, data, 2]).u32(), D3D_OK);
        e.mem.set_u32(data + 20, 0x4000_0000);
        assert_eq!(e.vcall(device, 4 * 94, &[60, data, 2]).u32(), D3D_OK);
        // Registers set before, then a range reaching one never set: it goes.
        assert_eq!(e.vcall(device, 4 * 94, &[61, data, 2]).u32(), D3D_OK);
        // A register never set, set to zeros (what the model's empty store holds): it goes, then not again.
        let zeros = e.mem.alloc(16);
        assert_eq!(e.vcall(device, 4 * 94, &[70, zeros, 1]).u32(), D3D_OK);
        assert_eq!(e.vcall(device, 4 * 94, &[70, zeros, 1]).u32(), D3D_OK);
        test_finish(&mut s.e);
        let log: Vec<String> = calls
            .take()
            .iter()
            .map(|l| l.split_whitespace().collect::<Vec<_>>().join(" "))
            .collect();
        assert_eq!(
            log,
            [
                "SetSamplerState(0, 1, 0x7)",
                "SetSamplerState(0, 1, 0x1)",
                "SetSamplerState(2, 3, 0x1)",
                "vertex_constant_f(60, [3f800000 3f800001 3f800002 3f800003 3f800004 3f800005 3f800006 3f800007], 2)",
                "vertex_constant_f(60, [3f800000 3f800001 3f800002 3f800003 3f800004 40000000 3f800006 3f800007], 2)",
                "vertex_constant_f(61, [3f800000 3f800001 3f800002 3f800003 3f800004 40000000 3f800006 3f800007], 2)",
                "vertex_constant_f(70, [00000000 00000000 00000000 00000000], 1)",
            ]
        );
        // The model holds what the game set, whatever was sent.
        let out = s.e.mem.alloc(32);
        assert_eq!(s.e.vcall(device, 4 * 68, &[0, 1, out]).u32(), D3D_OK);
        assert_eq!(s.e.mem.u32(out), 1);
        assert_eq!(s.e.vcall(device, 4 * 95, &[60, out, 1]).u32(), D3D_OK);
        assert_eq!(s.e.mem.u32(out + 4), 0x3f80_0001);
    }

    /// What one engine did over [`constant_sets_run`]: each call's answer, the stream's commands
    /// as the wire encodes them, and every constant register as the device's bindings hold it.
    type ConstantsRun = (Vec<u32>, Vec<u8>, Vec<Vec<u32>>);

    /// Drives a device through `steps` generated calls (seed `seed`): constant sets of all six
    /// kinds (ranges in and past the registers, null data, exact repeats, ranges that partly
    /// repeat), the data's words changed now and then, render states between them, and drawing cuts
    /// with sets from both sides; the constant sets in one pass (`NV_D3D9_ONE_PASS`) as `one_pass`
    /// says, repeats kept as `keep`.
    fn constant_sets_run(one_pass: bool, keep: bool, seed: u64, steps: u32) -> ConstantsRun {
        use super::super::d3d9draw::{as_drawing_side, draw_cut, draw_join};
        use super::super::d3d9resource::{binding, one_pass_for_tests};
        use d3d9_stream::{Backend, Cmd, Data, Reply, Stream};
        struct Wire(std::sync::Arc<std::sync::Mutex<Vec<u8>>>);
        impl Backend for Wire {
            fn execute(&mut self, cmd: Cmd<Data<'_>>) -> Reply {
                d3d9_stream::wire::encode(&cmd, &mut self.0.lock().unwrap());
                Reply::None
            }
        }
        let (mut e, dev) = texture_device(Store::Host);
        let wire = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        super::super::d3d9real::test_stream(
            &mut e,
            dev,
            Stream::direct(Box::new(Wire(wire.clone()))),
        );
        keep_repeats(&mut e, keep);
        one_pass_for_tests(&mut e, one_pass);
        // xorshift64*: the same calls for both engines.
        let mut state = seed | 1;
        let mut next = move |n: u32| -> u32 {
            state ^= state >> 12;
            state ^= state << 25;
            state ^= state >> 27;
            ((state.wrapping_mul(0x2545_f491_4f6c_dd1d) >> 32) % u64::from(n.max(1))) as u32
        };
        const SLOTS: [u32; 6] = [94, 96, 98, 109, 111, 113];
        const REGISTERS: [u32; 6] = [256, 16, 16, 224, 16, 16];
        const VALUES: [u32; 5] = [0, 0x3f80_0000, 0x4000_0000, 0xbf80_0000, 7];
        let block = e.mem.alloc(4096 + 1024);
        for i in 0..(4096 + 1024) / 4 {
            e.mem.set_u32(block + 4 * i, VALUES[next(5) as usize]);
        }
        // Zeros never written: registers set for the first time to what the model's empty
        // store already holds (they must still go).
        let zeros = e.mem.alloc(4096);
        let mut answers = Vec::new();
        let mut last = None;
        let mut in_cut = false;
        for _ in 0..steps {
            let pick = next(100);
            let call = match pick {
                0..=69 => {
                    let k = next(6) as usize;
                    let start = next(REGISTERS[k] + 2);
                    let count = if next(50) == 0 { REGISTERS[k] } else { next(9) };
                    let data = match next(40) {
                        0 => 0,
                        1..=4 => zeros,
                        _ => block + 4 * next(256),
                    };
                    Some((SLOTS[k], [start, data, count]))
                }
                70..=84 => {
                    for _ in 0..1 + next(4) {
                        e.mem
                            .set_u32(block + 4 * next(1024 + 256), VALUES[next(5) as usize]);
                    }
                    None
                }
                85..=91 => {
                    let (state, value) = (next(200), next(3));
                    answers.push(e.vcall(dev, 4 * 57, &[state, value]).u32());
                    None
                }
                92..=96 => last,
                _ => {
                    if in_cut {
                        draw_join(&mut e);
                    } else {
                        assert!(draw_cut(&mut e, dev));
                    }
                    in_cut = !in_cut;
                    None
                }
            };
            if let Some((slot, args)) = call {
                let answer = if in_cut && next(2) == 0 {
                    as_drawing_side(|| e.vcall(dev, 4 * slot, &args).u32())
                } else {
                    e.vcall(dev, 4 * slot, &args).u32()
                };
                answers.push(answer);
                last = Some((slot, args));
            }
        }
        if in_cut {
            draw_join(&mut e);
        }
        let mut registers = Vec::new();
        for (k, count) in REGISTERS.iter().enumerate() {
            for r in 0..*count {
                registers.push(binding(&mut e, dev, &format!("constant {k} {r}")));
            }
        }
        let bytes = wire.lock().unwrap().clone();
        (answers, bytes, registers)
    }

    /// The constant sets in one pass (`NV_D3D9_ONE_PASS`, on by default) against the model then
    /// the forwarding as two steps, over generated calls with repeats left out and kept: the same
    /// answers, the same commands to the backend byte for byte, and the same registers in the
    /// model.
    #[test]
    fn constant_sets_in_one_pass_match_the_two_steps() {
        for keep in [false, true] {
            for seed in [1, 0x9e37_79b9_7f4a_7c15, 0x00c0_ffee] {
                let two = constant_sets_run(false, keep, seed, 3000);
                let one = constant_sets_run(true, keep, seed, 3000);
                assert_eq!(one.0, two.0, "answers, keep {keep}, seed {seed:#x}");
                assert_eq!(one.2, two.2, "registers, keep {keep}, seed {seed:#x}");
                assert!(
                    one.1 == two.1,
                    "the stream's bytes differ (keep {keep}, seed {seed:#x}): {} against {}",
                    one.1.len(),
                    two.1.len()
                );
                // The run reached what it is for: sets sent, sets left out, sets refused.
                assert!(two.1.len() > 10_000, "{}", two.1.len());
                assert!(two.0.contains(&D3DERR_INVALIDCALL));
            }
        }
    }

    /// One constant set in one pass, by hand: sent once, left out when repeated, sent again when a
    /// word changes; refused (nothing sent, nothing kept) past the registers.
    #[test]
    fn a_constant_set_in_one_pass_sends_what_the_two_steps_send() {
        use super::super::d3d9resource::{binding, one_pass_for_tests};
        use d3d9_stream::{Backend, Cmd, Data, Reply, Stream};
        type Sent = std::sync::Arc<std::sync::Mutex<Vec<(u8, u32, u32, Vec<u32>)>>>;
        struct Log(Sent);
        impl Backend for Log {
            fn execute(&mut self, cmd: Cmd<Data<'_>>) -> Reply {
                if let Cmd::SetConstants {
                    kind,
                    start,
                    count,
                    data,
                    ..
                } = cmd
                {
                    let words = data
                        .bytes()
                        .chunks(4)
                        .map(|c| u32::from_le_bytes(c.try_into().unwrap()))
                        .collect();
                    self.0.lock().unwrap().push((kind, start, count, words));
                }
                Reply::None
            }
        }
        let (mut e, dev) = texture_device(Store::Host);
        let sent: Sent = Default::default();
        super::super::d3d9real::test_stream(
            &mut e,
            dev,
            Stream::direct(Box::new(Log(sent.clone()))),
        );
        keep_repeats(&mut e, false);
        one_pass_for_tests(&mut e, true);
        let data = e.mem.alloc(32);
        for i in 0..8 {
            e.mem.set_u32(data + 4 * i, 0x3f80_0000 + i);
        }
        assert_eq!(e.vcall(dev, 4 * 109, &[3, data, 2]).u32(), D3D_OK);
        assert_eq!(e.vcall(dev, 4 * 109, &[3, data, 2]).u32(), D3D_OK);
        e.mem.set_u32(data + 4, 9);
        assert_eq!(e.vcall(dev, 4 * 109, &[3, data, 2]).u32(), D3D_OK);
        // Booleans: one word a register, packed.
        assert_eq!(e.vcall(dev, 4 * 113, &[14, data, 2]).u32(), D3D_OK);
        // A register never set, set to zeros (what the model's empty store holds): it goes once.
        let zeros = e.mem.alloc(16);
        assert_eq!(e.vcall(dev, 4 * 94, &[70, zeros, 1]).u32(), D3D_OK);
        assert_eq!(e.vcall(dev, 4 * 94, &[70, zeros, 1]).u32(), D3D_OK);
        // Past the pixel float registers (224): refused.
        assert_eq!(
            e.vcall(dev, 4 * 109, &[223, data, 2]).u32(),
            D3DERR_INVALIDCALL
        );
        assert_eq!(e.vcall(dev, 4 * 109, &[0, 0, 1]).u32(), D3DERR_INVALIDCALL);
        let first: Vec<u32> = (0..8).map(|i| 0x3f80_0000 + i).collect();
        let mut changed = first.clone();
        changed[1] = 9;
        assert_eq!(
            *sent.lock().unwrap(),
            [
                (3, 3, 2, first),
                (3, 3, 2, changed),
                (5, 14, 2, vec![0x3f80_0000, 9]),
                (0, 70, 1, vec![0; 4]),
            ]
        );
        assert_eq!(
            binding(&mut e, dev, "constant 3 4"),
            [0x3f80_0004, 0x3f80_0005, 0x3f80_0006, 0x3f80_0007, 1]
        );
        assert_eq!(binding(&mut e, dev, "constant 5 15"), [9, 0, 0, 0, 1]);
        assert_eq!(binding(&mut e, dev, "constant 3 223"), [0, 0, 0, 0, 0]);
    }

    /// Every render, texture stage and sampler state of `dev` as main's device holds it, then as
    /// the drawing side's copy holds it (only while a cut is open).
    fn state_tables(e: &mut Engine, dev: u32) -> Vec<Option<u32>> {
        let objects = &device!(e).device_objects;
        let mut out = Vec::new();
        for d in [objects.devices.get(&dev), objects.draw_devices.get(&dev)]
            .into_iter()
            .flatten()
        {
            out.extend_from_slice(&d.render_states.0[..]);
            out.extend_from_slice(&d.stage_states.0[..]);
            out.extend_from_slice(&d.sampler_states.0[..]);
        }
        out
    }

    /// What one engine did over [`state_sets_run`]: each call's answer, the stream's commands as
    /// the wire encodes them, and the state tables ([`state_tables`]) before each join and at the
    /// end.
    type StatesRun = (Vec<u32>, Vec<u8>, Vec<Vec<Option<u32>>>);

    /// Drives a device through `steps` generated calls (seed `seed`): render states, texture stage
    /// states (stages 8 and 9 refused) and sampler states (samplers past the named ones refused),
    /// few values so that many sets repeat, exact repeats of the last call, constant sets between
    /// them, and drawing cuts with sets from both sides; the state sets in one pass
    /// (`NV_D3D9_ONE_PASS`) as `one_pass` says, repeats kept as `keep`.
    fn state_sets_run(one_pass: bool, keep: bool, seed: u64, steps: u32) -> StatesRun {
        use super::super::d3d9draw::{as_drawing_side, draw_cut, draw_join};
        use super::super::d3d9resource::one_pass_for_tests;
        use d3d9_stream::{Backend, Cmd, Data, Reply, Stream};
        struct Wire(std::sync::Arc<std::sync::Mutex<Vec<u8>>>);
        impl Backend for Wire {
            fn execute(&mut self, cmd: Cmd<Data<'_>>) -> Reply {
                d3d9_stream::wire::encode(&cmd, &mut self.0.lock().unwrap());
                Reply::None
            }
        }
        let (mut e, dev) = texture_device(Store::Host);
        let wire = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        super::super::d3d9real::test_stream(
            &mut e,
            dev,
            Stream::direct(Box::new(Wire(wire.clone()))),
        );
        keep_repeats(&mut e, keep);
        one_pass_for_tests(&mut e, one_pass);
        // xorshift64*: the same calls for both engines.
        let mut state = seed | 1;
        let mut next = move |n: u32| -> u32 {
            state ^= state >> 12;
            state ^= state << 25;
            state ^= state >> 27;
            ((state.wrapping_mul(0x2545_f491_4f6c_dd1d) >> 32) % u64::from(n.max(1))) as u32
        };
        const SAMPLERS: [u32; 8] = [0, 1, 7, 15, 16, 256, 260, 261];
        const VALUES: [u32; 4] = [0, 1, 2, 0x3f80_0000];
        let data = e.mem.alloc(64);
        e.mem.set_u32(data, 0x3f80_0000);
        let mut answers = Vec::new();
        let mut tables = Vec::new();
        let mut last = None;
        let mut in_cut = false;
        for _ in 0..steps {
            let call = match next(100) {
                0..=29 => Some((57, [next(210), VALUES[next(4) as usize], 0])),
                30..=54 => Some((67, [next(10), 1 + next(32), VALUES[next(4) as usize]])),
                55..=84 => Some((
                    69,
                    [
                        SAMPLERS[next(8) as usize],
                        1 + next(13),
                        VALUES[next(4) as usize],
                    ],
                )),
                85..=87 => Some((94, [next(8), data, 1])),
                88..=95 => last,
                _ => {
                    if in_cut {
                        tables.push(state_tables(&mut e, dev));
                        draw_join(&mut e);
                    } else {
                        assert!(draw_cut(&mut e, dev));
                    }
                    in_cut = !in_cut;
                    None
                }
            };
            if let Some((slot, args)) = call {
                let args = if slot == 57 { &args[..2] } else { &args[..] };
                let answer = if in_cut && next(2) == 0 {
                    as_drawing_side(|| e.vcall(dev, 4 * slot, args).u32())
                } else {
                    e.vcall(dev, 4 * slot, args).u32()
                };
                answers.push(answer);
                last = call;
            }
        }
        if in_cut {
            tables.push(state_tables(&mut e, dev));
            draw_join(&mut e);
        }
        tables.push(state_tables(&mut e, dev));
        let bytes = wire.lock().unwrap().clone();
        (answers, bytes, tables)
    }

    /// The render, texture stage and sampler state sets in one pass (`NV_D3D9_ONE_PASS`, on by
    /// default) against the model then the forwarding as two steps, over generated calls with
    /// repeats left out and kept: the same answers, the same commands to the backend byte for
    /// byte, and the same states in main's device and the drawing side's copy.
    #[test]
    fn state_sets_in_one_pass_match_the_two_steps() {
        for seed in [1, 0x9e37_79b9_7f4a_7c15, 0x00c0_ffee] {
            let mut lengths = Vec::new();
            for keep in [false, true] {
                let two = state_sets_run(false, keep, seed, 3000);
                let one = state_sets_run(true, keep, seed, 3000);
                assert_eq!(one.0, two.0, "answers, keep {keep}, seed {seed:#x}");
                assert!(
                    one.2 == two.2,
                    "the states differ (keep {keep}, seed {seed:#x})"
                );
                assert!(
                    one.1 == two.1,
                    "the stream's bytes differ (keep {keep}, seed {seed:#x}): {} against {}",
                    one.1.len(),
                    two.1.len()
                );
                // The run reached what it is for: sets refused, cuts made.
                assert!(two.0.contains(&D3DERR_INVALIDCALL));
                assert!(two.2.len() > 10, "{}", two.2.len());
                lengths.push(two.1.len());
            }
            // Sampler states set again were left out unless kept.
            assert!(lengths[0] < lengths[1], "{lengths:?}");
        }
    }

    /// The state sets in one pass, by hand: a render or texture stage state set again still goes,
    /// a sampler state set again does not (unless it changed); a stage or sampler past the named
    /// ones is refused and nothing goes; the getters answer what was set; and a state main sets
    /// while a cut is open keeps main's value at the join.
    #[test]
    fn a_state_set_in_one_pass_sends_what_the_two_steps_send() {
        use super::super::d3d9draw::{as_drawing_side, draw_cut, draw_join};
        use super::super::d3d9resource::one_pass_for_tests;
        use d3d9_stream::{Backend, Cmd, Data, Reply, Stream};
        type Sent = std::sync::Arc<std::sync::Mutex<Vec<(char, u32, u32, u32)>>>;
        struct Log(Sent);
        impl Backend for Log {
            fn execute(&mut self, cmd: Cmd<Data<'_>>) -> Reply {
                let call = match cmd {
                    Cmd::SetRenderState { state, value, .. } => ('r', state, 0, value),
                    Cmd::SetTextureStageState {
                        stage, kind, value, ..
                    } => ('t', stage, kind, value),
                    Cmd::SetSamplerState {
                        sampler,
                        kind,
                        value,
                        ..
                    } => ('s', sampler, kind, value),
                    _ => return Reply::None,
                };
                self.0.lock().unwrap().push(call);
                Reply::None
            }
        }
        let (mut e, dev) = texture_device(Store::Host);
        let sent: Sent = Default::default();
        super::super::d3d9real::test_stream(
            &mut e,
            dev,
            Stream::direct(Box::new(Log(sent.clone()))),
        );
        keep_repeats(&mut e, false);
        one_pass_for_tests(&mut e, true);
        for _ in 0..2 {
            assert_eq!(e.vcall(dev, 4 * 57, &[7, 1]).u32(), D3D_OK);
            assert_eq!(e.vcall(dev, 4 * 67, &[1, 4, 2]).u32(), D3D_OK);
            assert_eq!(e.vcall(dev, 4 * 69, &[256, 5, 3]).u32(), D3D_OK);
        }
        assert_eq!(e.vcall(dev, 4 * 69, &[256, 5, 0]).u32(), D3D_OK);
        // A sampler state never set before, set to zero: it goes.
        assert_eq!(e.vcall(dev, 4 * 69, &[3, 1, 0]).u32(), D3D_OK);
        assert_eq!(e.vcall(dev, 4 * 67, &[8, 4, 2]).u32(), D3DERR_INVALIDCALL);
        assert_eq!(e.vcall(dev, 4 * 69, &[16, 5, 3]).u32(), D3DERR_INVALIDCALL);
        assert_eq!(
            *sent.lock().unwrap(),
            [
                ('r', 7, 0, 1),
                ('t', 1, 4, 2),
                ('s', 256, 5, 3),
                ('r', 7, 0, 1),
                ('t', 1, 4, 2),
                ('s', 256, 5, 0),
                ('s', 3, 1, 0),
            ]
        );
        let out = e.mem.alloc(4);
        assert_eq!(e.vcall(dev, 4 * 58, &[7, out]).u32(), D3D_OK);
        assert_eq!(e.mem.u32(out), 1);
        assert_eq!(e.vcall(dev, 4 * 66, &[1, 4, out]).u32(), D3D_OK);
        assert_eq!(e.mem.u32(out), 2);
        assert_eq!(e.vcall(dev, 4 * 68, &[256, 5, out]).u32(), D3D_OK);
        assert_eq!(e.mem.u32(out), 0);
        // A cut: main sets render state 7, the drawing side sets 7 and 8: main's 7 and the
        // drawing side's 8 hold after the join.
        assert!(draw_cut(&mut e, dev));
        assert_eq!(e.vcall(dev, 4 * 57, &[7, 5]).u32(), D3D_OK);
        as_drawing_side(|| {
            assert_eq!(e.vcall(dev, 4 * 57, &[7, 6]).u32(), D3D_OK);
            assert_eq!(e.vcall(dev, 4 * 57, &[8, 6]).u32(), D3D_OK);
        });
        draw_join(&mut e);
        assert_eq!(e.vcall(dev, 4 * 58, &[7, out]).u32(), D3D_OK);
        assert_eq!(e.mem.u32(out), 5);
        assert_eq!(e.vcall(dev, 4 * 58, &[8, out]).u32(), D3D_OK);
        assert_eq!(e.mem.u32(out), 6);
    }

    /// The textures: each one's address, references and samplers holding it (address order), then
    /// each sampler's texture on main's device and on the drawing side's copy (while a cut is
    /// open).
    fn textures_held(e: &mut Engine, dev: u32) -> Vec<u32> {
        let objects = &device!(e).device_objects;
        let mut textures: Vec<_> = objects
            .textures
            .iter()
            .map(|(a, t)| [*a, t.references, t.bound])
            .collect();
        textures.sort_unstable();
        let mut out: Vec<u32> = textures.into_iter().flatten().collect();
        for d in [objects.devices.get(&dev), objects.draw_devices.get(&dev)]
            .into_iter()
            .flatten()
        {
            let mut bound: Vec<_> = d.textures.iter().map(|(s, t)| [*s, *t]).collect();
            bound.sort_unstable();
            out.push(u32::MAX);
            out.extend(bound.into_iter().flatten());
        }
        out
    }

    /// A device for [`binding_sets_run`] and the hand test: six managed textures, four vertex
    /// buffers and an index buffer made before the stream (so their making is not in it), then the
    /// wire stream (`wire`, the commands as `d3d9_stream::wire` encodes them) with every object
    /// forwarded (so an object's end is a command too), the FVF, the index buffer, `vertex[0]` on
    /// stream 0 and a scene begun; the sets and draws in one pass as `one_pass` says.
    #[allow(clippy::type_complexity)]
    fn binding_device(
        one_pass: bool,
        wire: Box<dyn d3d9_stream::Backend>,
    ) -> (Engine, u32, Vec<u32>, Vec<u32>, u32) {
        use super::super::d3d9resource::one_pass_for_tests;
        let (mut e, dev) = texture_device(Store::Host);
        let out = e.mem.alloc(4);
        let textures: Vec<u32> = (0..6)
            .map(|_| new_texture(&mut e, dev, [4, 4, 1, 0, A8R8G8B8, 1]))
            .collect();
        let make = |e: &mut Engine, slot: u32, [a, b, c, d]: [u32; 4]| {
            assert_eq!(e.vcall(dev, 4 * slot, &[a, b, c, d, out, 0]).u32(), D3D_OK);
            e.mem.u32(out)
        };
        let vertex: Vec<u32> = (0..4)
            .map(|_| make(&mut e, 26, [64, 0, 0x142, 1]))
            .collect();
        let index = make(&mut e, 27, [12, 0, 101, 1]);
        super::super::d3d9real::test_stream(&mut e, dev, d3d9_stream::Stream::direct(wire));
        for &o in textures.iter().chain(&vertex).chain([&index]) {
            super::super::d3d9real::test_forwarded(&mut e, o);
        }
        one_pass_for_tests(&mut e, one_pass);
        assert_eq!(e.vcall(dev, 4 * 89, &[0x142]).u32(), D3D_OK);
        assert_eq!(e.vcall(dev, 4 * 104, &[index]).u32(), D3D_OK);
        assert_eq!(e.vcall(dev, 4 * 100, &[0, vertex[0], 0, 16]).u32(), D3D_OK);
        assert_eq!(e.vcall(dev, 4 * 41, &[]).u32(), D3D_OK);
        (e, dev, textures, vertex, index)
    }

    /// What one engine did over [`binding_sets_run`]: each call's answer, the stream's commands as
    /// the wire encodes them, and what the model held (textures, buffers, streams) before each
    /// join and at the end.
    type BindingsRun = (Vec<u32>, Vec<u8>, Vec<Vec<u32>>);

    /// Drives a device ([`binding_device`]) through `steps` generated calls (seed `seed`):
    /// `SetTexture` (samplers past the named ones refused, null), `SetStreamSource` (streams past
    /// 15 and index buffers refused, null on streams other than 0, the same buffer with another
    /// offset), `DrawPrimitive` and `DrawIndexedPrimitive`, the program letting go of textures and
    /// buffers (so a later set ends them: their end, then the command), exact repeats, and drawing
    /// cuts with calls from both sides; the sets and draws in one pass (`NV_D3D9_ONE_PASS`) as
    /// `one_pass` says.
    fn binding_sets_run(one_pass: bool, seed: u64, steps: u32) -> BindingsRun {
        use super::super::d3d9draw::{as_drawing_side, draw_cut, draw_join};
        use super::super::d3d9resource::held_for_tests;
        use d3d9_stream::{Backend, Cmd, Data, Reply};
        struct Wire(std::sync::Arc<std::sync::Mutex<Vec<u8>>>);
        impl Backend for Wire {
            fn execute(&mut self, cmd: Cmd<Data<'_>>) -> Reply {
                d3d9_stream::wire::encode(&cmd, &mut self.0.lock().unwrap());
                Reply::None
            }
        }
        let wire = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let (mut e, dev, mut textures, mut vertex, index) =
            binding_device(one_pass, Box::new(Wire(wire.clone())));
        // Stream 0 always holds a buffer (a draw without one is not provided): `keep`, which the
        // program never lets go of, or another set over it.
        let keep = vertex[0];
        // xorshift64*: the same calls for both engines.
        let mut state = seed | 1;
        let mut next = move |n: u32| -> u32 {
            state ^= state >> 12;
            state ^= state << 25;
            state ^= state >> 27;
            ((state.wrapping_mul(0x2545_f491_4f6c_dd1d) >> 32) % u64::from(n.max(1))) as u32
        };
        const SAMPLERS: [u32; 6] = [0, 1, 2, 15, 16, 256];
        const STREAMS: [u32; 5] = [0, 1, 2, 15, 16];
        let mut answers = Vec::new();
        let mut held = Vec::new();
        let mut last: Option<(u32, [u32; 6])> = None;
        let mut in_cut = false;
        for _ in 0..steps {
            let call = match next(100) {
                0..=29 => {
                    let t = match next(8) {
                        0 => 0,
                        _ => textures[next(textures.len() as u32) as usize],
                    };
                    Some((65, [SAMPLERS[next(6) as usize], t, 0, 0, 0, 0]))
                }
                30..=54 => {
                    let stream = STREAMS[next(5) as usize];
                    let buffer = match next(10) {
                        0 if stream != 0 => 0,
                        1 => index,
                        2 => keep,
                        _ => vertex[next(vertex.len() as u32) as usize],
                    };
                    Some((100, [stream, buffer, 4 * next(3), 16, 0, 0]))
                }
                55..=64 => Some((81, [1 + next(6), next(4), 1 + next(3), 0, 0, 0])),
                65..=74 => Some((82, [2 + next(5), 0, 0, 4, next(3), 1])),
                75..=79 => {
                    // The program lets go of a texture or a buffer, and sets it no more.
                    if next(2) == 0 && textures.len() > 2 {
                        let t = textures.swap_remove(next(textures.len() as u32) as usize);
                        answers.push(e.vcall(t, 4 * 2, &[]).u32());
                    } else if vertex.len() > 2 {
                        let b = vertex.swap_remove(1 + next(vertex.len() as u32 - 1) as usize);
                        answers.push(e.vcall(b, 4 * 2, &[]).u32());
                    }
                    last = None;
                    None
                }
                80..=93 => last,
                _ => {
                    if in_cut {
                        held.push(textures_held(&mut e, dev));
                        held.push(held_for_tests(&mut e));
                        draw_join(&mut e);
                    } else {
                        assert!(draw_cut(&mut e, dev));
                    }
                    in_cut = !in_cut;
                    None
                }
            };
            if let Some((slot, args)) = call {
                let n = match slot {
                    65 => 2,
                    81 => 3,
                    100 => 4,
                    _ => 6,
                };
                let answer = if in_cut && next(2) == 0 {
                    as_drawing_side(|| e.vcall(dev, 4 * slot, &args[..n]).u32())
                } else {
                    e.vcall(dev, 4 * slot, &args[..n]).u32()
                };
                answers.push(answer);
                last = call;
            }
        }
        if in_cut {
            held.push(textures_held(&mut e, dev));
            held.push(held_for_tests(&mut e));
            draw_join(&mut e);
        }
        held.push(textures_held(&mut e, dev));
        held.push(held_for_tests(&mut e));
        let bytes = wire.lock().unwrap().clone();
        (answers, bytes, held)
    }

    /// `SetTexture`, `SetStreamSource` and the draws in one pass (`NV_D3D9_ONE_PASS`, on by
    /// default) against the model then the forwarding as two steps, over generated calls: the same
    /// answers, the same commands to the backend byte for byte (objects' ends included), and the
    /// same textures, buffers, counts and streams in the model.
    #[test]
    fn binding_sets_and_draws_in_one_pass_match_the_two_steps() {
        for seed in [1, 0x9e37_79b9_7f4a_7c15, 0x00c0_ffee] {
            let two = binding_sets_run(false, seed, 3000);
            let one = binding_sets_run(true, seed, 3000);
            assert_eq!(one.0, two.0, "answers, seed {seed:#x}");
            assert!(
                one.2 == two.2,
                "what the model holds differs (seed {seed:#x})"
            );
            assert!(
                one.1 == two.1,
                "the stream's bytes differ (seed {seed:#x}): {} against {}",
                one.1.len(),
                two.1.len()
            );
            // The run reached what it is for: calls refused, cuts made.
            assert!(two.0.contains(&D3DERR_INVALIDCALL));
            assert!(two.2.len() > 20, "{}", two.2.len());
        }
    }

    /// `SetTexture` and `SetStreamSource` in one pass, by hand: each set sent; a texture or a
    /// buffer the program let go of ends when the device lets go of it, and its end reaches the
    /// backend before the set that replaced it; refused calls send nothing; the draws are sent.
    #[test]
    fn a_binding_set_in_one_pass_sends_what_the_two_steps_send() {
        use d3d9_stream::{Backend, Cmd, Data, Reply};
        type Sent = std::sync::Arc<std::sync::Mutex<Vec<String>>>;
        struct Log(Sent);
        impl Backend for Log {
            fn execute(&mut self, cmd: Cmd<Data<'_>>) -> Reply {
                let line = match cmd {
                    Cmd::SetTexture {
                        sampler, texture, ..
                    } => format!("texture {sampler} {texture:#x}"),
                    Cmd::SetStreamSource {
                        stream,
                        buffer,
                        offset,
                        ..
                    } => format!("stream {stream} {buffer:#x} {offset}"),
                    Cmd::Destroy { object } => format!("destroy {object:#x}"),
                    Cmd::DrawPrimitive { .. } => "draw".into(),
                    Cmd::DrawIndexedPrimitive { .. } => "indexed draw".into(),
                    _ => return Reply::None,
                };
                self.0.lock().unwrap().push(line);
                Reply::None
            }
        }
        let sent: Sent = Default::default();
        let (mut e, dev, textures, vertex, index) =
            binding_device(true, Box::new(Log(sent.clone())));
        sent.lock().unwrap().clear();
        let (t0, t1, v1) = (textures[0], textures[1], vertex[1]);
        assert_eq!(e.vcall(dev, 4 * 65, &[0, t0]).u32(), D3D_OK);
        assert_eq!(e.vcall(dev, 4 * 65, &[0, t0]).u32(), D3D_OK);
        // The program lets go of t0 while sampler 0 holds it: it lives until replaced.
        assert_eq!(e.vcall(t0, 4 * 2, &[]).u32(), 0);
        assert_eq!(e.vcall(dev, 4 * 65, &[0, t1]).u32(), D3D_OK);
        assert_eq!(e.vcall(dev, 4 * 65, &[16, t1]).u32(), D3DERR_INVALIDCALL);
        assert_eq!(e.vcall(dev, 4 * 100, &[1, v1, 8, 16]).u32(), D3D_OK);
        assert_eq!(e.vcall(dev, 4 * 100, &[1, v1, 12, 16]).u32(), D3D_OK);
        assert_eq!(e.vcall(v1, 4 * 2, &[]).u32(), 0);
        assert_eq!(e.vcall(dev, 4 * 100, &[1, 0, 0, 0]).u32(), D3D_OK);
        assert_eq!(
            e.vcall(dev, 4 * 100, &[16, v1, 0, 16]).u32(),
            D3DERR_INVALIDCALL
        );
        assert_eq!(
            e.vcall(dev, 4 * 100, &[2, index, 0, 16]).u32(),
            D3DERR_INVALIDCALL
        );
        assert_eq!(e.vcall(dev, 4 * 81, &[4, 0, 1]).u32(), D3D_OK);
        assert_eq!(e.vcall(dev, 4 * 82, &[4, 0, 0, 4, 0, 1]).u32(), D3D_OK);
        assert_eq!(
            *sent.lock().unwrap(),
            [
                format!("texture 0 {t0:#x}"),
                format!("texture 0 {t0:#x}"),
                format!("destroy {t0:#x}"),
                format!("texture 0 {t1:#x}"),
                format!("stream 1 {v1:#x} 8"),
                format!("stream 1 {v1:#x} 12"),
                format!("destroy {v1:#x}"),
                "stream 1 0x0 0".to_string(),
                "draw".to_string(),
                "indexed draw".to_string(),
            ]
        );
        let out = e.mem.alloc(12);
        assert_eq!(e.vcall(dev, 4 * 64, &[0, out]).u32(), D3D_OK);
        assert_eq!(e.mem.u32(out), t1);
        assert_eq!(
            e.vcall(dev, 4 * 101, &[1, out, out + 4, out + 8]).u32(),
            D3D_OK
        );
        assert_eq!([e.mem.u32(out), e.mem.u32(out + 4)], [0, 0]);
    }

    /// What one engine did over [`writer_run`]: each call's answer, the stream's bytes, what the
    /// model holds (states, textures, buffers, streams, constant registers) at the end, and how
    /// many device calls went directly.
    type WriterRun = (Vec<u32>, Vec<u8>, Vec<Vec<u32>>, u64);

    /// Drives a device ([`binding_device`], stepped threads, direct calls on) through `steps`
    /// generated calls (seed `seed`) of every kind the typed draw code's writer takes (render,
    /// stage and sampler states, textures, stream sources, both draws, float constants of both
    /// kinds, the integer ones beside them), refused ones among them: through the writer's
    /// functions (`engine_core::real_threads::device_set_render_state` and its kin) when `writer`,
    /// else through `device_vcall` with the same words.
    fn writer_run(writer: bool, seed: u64, steps: u32) -> WriterRun {
        use super::super::d3d9resource::{binding, held_for_tests};
        use d3d9_stream::{Backend, Cmd, Data, Reply};
        use engine_core::real_threads as rt;
        struct Wire(std::sync::Arc<std::sync::Mutex<Vec<u8>>>);
        impl Backend for Wire {
            fn execute(&mut self, cmd: Cmd<Data<'_>>) -> Reply {
                d3d9_stream::wire::encode(&cmd, &mut self.0.lock().unwrap());
                Reply::None
            }
        }
        let wire = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let (mut e, dev, textures, vertex, index) =
            binding_device(true, Box::new(Wire(wire.clone())));
        let data = e.mem.alloc(16 * 64);
        for i in 0..4 * 64 {
            e.mem.set_u32(
                data + 4 * i,
                [0, 0x3f80_0000, 7, 0xbf80_0000][(i % 7 % 4) as usize],
            );
        }
        rt::force_direct_for_tests();
        assert!(rt::direct_begin(&e));
        let before = rt::direct_calls();
        // xorshift64*: the same calls for both engines.
        let mut state = seed | 1;
        let mut next = move |n: u32| -> u32 {
            state ^= state >> 12;
            state ^= state << 25;
            state ^= state >> 27;
            ((state.wrapping_mul(0x2545_f491_4f6c_dd1d) >> 32) % u64::from(n.max(1))) as u32
        };
        let mut answers = Vec::new();
        for _ in 0..steps {
            let v = [0, 1, 2, 0x3f80_0000][next(4) as usize];
            let (slot, args): (u32, Vec<u32>) = match next(10) {
                0 => (0xe4, vec![next(210), v]),
                1 => (0x10c, vec![next(10), 1 + next(32), v]),
                2 => (
                    0x114,
                    vec![[0, 1, 15, 16, 256][next(5) as usize], 1 + next(13), v],
                ),
                3 => {
                    let t = [0, textures[next(6) as usize]][usize::from(next(8) != 0)];
                    (0x104, vec![[0, 1, 16][next(3) as usize], t])
                }
                4 => {
                    let b = [vertex[next(4) as usize], index][usize::from(next(8) == 0)];
                    (0x190, vec![next(17), b, 4 * next(2), 16])
                }
                5 => (0x144, vec![1 + next(6), next(4), 1 + next(3)]),
                6 => (0x148, vec![2 + next(5), 0, 0, 4, next(3), 1]),
                7 => (0x178, vec![next(260), data + 16 * next(8), 1 + next(4)]),
                8 => (0x1b4, vec![next(230), data + 16 * next(8), 1 + next(4)]),
                _ => (0x180, vec![next(17), data, 1 + next(2)]),
            };
            let a = &args;
            let ret = if !writer {
                rt::device_vcall(&mut e, dev, slot, a)
            } else {
                match slot {
                    0xe4 => rt::device_set_render_state(&mut e, dev, a[0], a[1]),
                    0x10c => rt::device_set_stage_state(&mut e, dev, a[0], a[1], a[2]),
                    0x114 => rt::device_set_sampler_state(&mut e, dev, a[0], a[1], a[2]),
                    0x104 => rt::device_set_texture(&mut e, dev, a[0], a[1]),
                    0x190 => rt::device_set_stream_source(&mut e, dev, a[0], a[1], a[2], a[3]),
                    0x144 => rt::device_draw_primitive(&mut e, dev, a[0], a[1], a[2]),
                    0x148 => rt::device_draw_indexed_primitive(
                        &mut e, dev, a[0], a[1], a[2], a[3], a[4], a[5],
                    ),
                    _ => rt::device_set_constants(&mut e, dev, slot, a[0], a[1], a[2]),
                }
            };
            answers.push(ret.u32());
        }
        let calls = rt::direct_calls() - before;
        rt::direct_end();
        let mut held = vec![
            state_tables(&mut e, dev)
                .iter()
                .map(|s| s.unwrap_or(u32::MAX))
                .collect(),
            textures_held(&mut e, dev),
            held_for_tests(&mut e),
        ];
        for (k, count) in [(0, 256), (1, 16), (3, 224)] {
            for r in 0..count {
                held.push(binding(&mut e, dev, &format!("constant {k} {r}")));
            }
        }
        let bytes = wire.lock().unwrap().clone();
        (answers, bytes, held, calls)
    }

    /// The typed draw code's writer against `device_vcall` (stepped threads, direct calls on):
    /// over generated calls of every kind it takes, the same answers, the same commands to the
    /// backend byte for byte, the same model state and the same count of direct calls.
    #[test]
    fn the_writer_matches_device_calls_by_slot() {
        for seed in [1, 0x9e37_79b9_7f4a_7c15, 0x00c0_ffee] {
            let by_slot = writer_run(false, seed, 2000);
            let writer = writer_run(true, seed, 2000);
            assert_eq!(writer.0, by_slot.0, "answers, seed {seed:#x}");
            assert!(writer.2 == by_slot.2, "the model differs (seed {seed:#x})");
            assert!(
                writer.1 == by_slot.1,
                "the stream's bytes differ (seed {seed:#x}): {} against {}",
                writer.1.len(),
                by_slot.1.len()
            );
            assert_eq!(writer.3, by_slot.3, "direct calls, seed {seed:#x}");
            assert_eq!(by_slot.3, 2000, "every call went directly");
            assert!(by_slot.0.contains(&D3DERR_INVALIDCALL));
            assert!(by_slot.1.len() > 10_000, "{}", by_slot.1.len());
        }
    }

    /// Calls the writer must not take: a device vtable word replaced (as a mod's hook does) sends
    /// `SetRenderState` to what the word names, at the very next call and back to the layer's own
    /// once the word is put back; a function registered over the layer's at the slot's address (as
    /// a wrap does) runs in its place; with direct calls off the call goes by the vtable.
    #[test]
    fn the_writer_sends_replaced_slots_where_they_point() {
        use engine_core::real_threads as rt;
        use std::sync::atomic::{AtomicU32, Ordering};
        static HOOKED: AtomicU32 = AtomicU32::new(0);
        let (mut e, dev) = texture_device(Store::Host);
        let hook: crate::AbiFn = |_, a| {
            HOOKED.fetch_add(a[2], Ordering::Relaxed);
            engine_core::abi::Ret::default()
        };
        let elsewhere = CODE_BASE + 0xf000;
        e.register(elsewhere, hook);
        rt::force_direct_for_tests();
        assert!(rt::direct_begin(&e));
        let table = e.mem.u32(dev);
        let word = table + 0xe4;
        assert_eq!(e.mem.u32(word), DEVICE_CODE + 0xe4);
        assert_eq!(rt::device_set_render_state(&mut e, dev, 7, 1).u32(), D3D_OK);
        assert_eq!(render_state(&mut e, dev, 7), 1);
        // A hook in the vtable: the hook gets the call, the model does not.
        e.mem.set_u32(word, elsewhere);
        assert_eq!(rt::device_set_render_state(&mut e, dev, 7, 5).u32(), 0);
        assert_eq!(HOOKED.load(Ordering::Relaxed), 5);
        assert_eq!(render_state(&mut e, dev, 7), 1);
        e.mem.set_u32(word, DEVICE_CODE + 0xe4);
        assert_eq!(rt::device_set_render_state(&mut e, dev, 7, 2).u32(), D3D_OK);
        assert_eq!(render_state(&mut e, dev, 7), 2);
        // A function registered over the layer's at the slot's address.
        e.register(DEVICE_CODE + 0xe4, hook);
        assert_eq!(rt::device_set_render_state(&mut e, dev, 7, 3).u32(), 0);
        assert_eq!(HOOKED.load(Ordering::Relaxed), 8);
        assert_eq!(render_state(&mut e, dev, 7), 2);
        rt::direct_end();
        // Direct calls off: by the vtable (still the registered function).
        assert_eq!(rt::device_set_render_state(&mut e, dev, 7, 4).u32(), 0);
        assert_eq!(HOOKED.load(Ordering::Relaxed), 12);
    }

    /// With real threads (direct calls on, under a device lock the caller holds, as
    /// `direct_device_calls_under_a_held_device_lock_answer_the_same`): the writer's calls answer
    /// and change the model as the calls by slot do.
    #[test]
    fn the_writer_with_real_threads_answers_the_same() {
        use engine_core::real_threads as rt;
        let (mut e, dev) = texture_device(Store::Host);
        rt::start(&mut e).expect("real threads on");
        rt::force_direct_for_tests();
        assert!(rt::direct_begin(&e));
        let before = rt::direct_calls();
        {
            let _device = rt::device_guard(&e);
            assert_eq!(rt::device_set_render_state(&mut e, dev, 8, 2).u32(), D3D_OK);
            assert_eq!(
                rt::device_set_sampler_state(&mut e, dev, 16, 1, 1).u32(),
                D3DERR_INVALIDCALL
            );
            assert_eq!(
                rt::device_set_sampler_state(&mut e, dev, 1, 1, 3).u32(),
                D3D_OK
            );
            let out = e.mem.alloc(4);
            assert_eq!(
                rt::device_vcall(&mut e, dev, 4 * 58, &[8, out]).u32(),
                D3D_OK
            );
            assert_eq!(e.mem.u32(out), 2);
            assert_eq!(
                rt::device_vcall(&mut e, dev, 4 * 68, &[1, 1, out]).u32(),
                D3D_OK
            );
            assert_eq!(e.mem.u32(out), 3);
        }
        assert_eq!(rt::direct_calls() - before, 5);
        rt::direct_end();
        assert!(!rt::direct_holds() && !rt::device_lock_held(&e));
    }

    /// The scene's device state as the game can read it back: each getter's answer and the
    /// words it writes, and each object's reference count (`AddRef` then `Release`).
    #[cfg(windows)]
    fn model_answers(s: &mut Scene) -> Vec<u32> {
        let device = s.device;
        let e = &mut s.e;
        let out = e.mem.alloc(0x1000);
        let mut answers = Vec::new();
        let mut get = |e: &mut Engine, slot: u32, args: &[u32], words: u32| {
            e.mem.zero(out, 4 * words);
            answers.push(e.vcall(device, 4 * slot, args).u32());
            answers.extend((0..words).map(|i| e.mem.u32(out + 4 * i)));
        };
        for state in [7, 14, 22, 27] {
            get(e, 58, &[state, out], 1);
        }
        for (sampler, kind) in [(0, 1), (0, 2), (1, 5), (1, 6)] {
            get(e, 68, &[sampler, kind, out], 1);
        }
        get(e, 66, &[1, 1, out], 1);
        for sampler in [0, 1, 2, 257] {
            get(e, 64, &[sampler, out], 1);
        }
        for stream in [0, 1] {
            get(e, 101, &[stream, out, out + 4, out + 8], 3);
        }
        for slot in [105, 88, 93, 108] {
            get(e, slot, &[out], 1);
        }
        get(e, 95, &[0, out, 256], 1024);
        get(e, 110, &[0, out, 224], 896);
        get(e, 97, &[2, out, 3], 12);
        get(e, 99, &[1, out, 5], 5);
        get(e, 112, &[15, out, 1], 4);
        get(e, 114, &[0, out, 16], 16);
        for state in [256, 3, 2] {
            get(e, 45, &[state, out], 16);
        }
        get(e, 48, &[out], 6);
        let objects = [
            &s.textures,
            &s.vertex_buffers,
            &s.index_buffers,
            &s.declarations,
            &s.vertex_shaders,
            &s.pixel_shaders,
        ];
        for &o in objects.iter().flat_map(|o| o.iter()) {
            answers.push(e.vcall(o, 4, &[]).u32());
            answers.push(e.vcall(o, 8, &[]).u32());
        }
        answers
    }

    /// The cost of the platform's Direct3D 9 calls as the game makes them (through
    /// `Engine::vcall`), per call: the call machinery alone (the same calls to an object whose
    /// slots do nothing), the model alone, and the model forwarding to a stand-in real device
    /// (`d3d9backend::stub`); then the forwarded cost by group of calls. The scene has 2048
    /// textures, 1024 vertex and index buffers, 64 shaders of each kind and 32 declarations.
    /// `cargo test --release -p engine-platform --lib cost_of_d3d9 -- --ignored --nocapture`.
    #[cfg(windows)]
    #[test]
    #[ignore]
    fn cost_of_d3d9_calls() {
        const SIZE: [u32; 4] = [2048, 1024, 64, 32];
        let draws = 2000;
        let frames = 10;
        // Nanoseconds per call of `group` (all of a draw's for `None`), the best of five rounds
        // after a warm-up frame.
        let run = |s: &mut Scene, device: u32, group: Option<usize>| -> f64 {
            let calls = group.map_or(28, |g| GROUPS[g].1);
            let mut best = f64::MAX;
            for round in 0..6 {
                let t = std::time::Instant::now();
                for _ in 0..if round == 0 { 1 } else { frames } {
                    s.e.vcall(s.device, 4 * 41, &[]);
                    for i in 0..draws {
                        match group {
                            None => draw(s, device, i),
                            Some(_) if round == 0 => draw(s, device, i),
                            Some(g) => draw_group(s, device, i, g),
                        }
                    }
                    s.e.vcall(s.device, 4 * 42, &[]);
                }
                let ns = t.elapsed().as_nanos() as f64 / (frames * draws) as f64 / f64::from(calls);
                if round > 0 {
                    best = best.min(ns);
                }
            }
            best
        };
        // The floor: an object whose every slot is a function that does nothing.
        let mut s = scene(SIZE);
        fn nothing(_: &mut Engine, _: &[u32]) -> crate::Ret {
            crate::Ret::default()
        }
        let empty = 0x7e0f_0000;
        s.e.register(empty, nothing);
        let table = s.e.mem.alloc(4 * 128);
        for i in 0..128 {
            s.e.mem.set_u32(table + 4 * i, empty);
        }
        let object = s.e.mem.alloc(8);
        s.e.mem.set_u32(object, table);
        let floor = run(&mut s, object, None);
        let device = s.device;
        let model = run(&mut s, device, None);
        let model_groups: Vec<f64> = (0..GROUPS.len())
            .map(|g| run(&mut s, device, Some(g)))
            .collect();
        let log = std::sync::Arc::default();
        let forwarding = |threaded: bool| {
            let mut s = scene(SIZE);
            let (b, objects) = stand_ins(&s, &log);
            let device = s.device;
            super::super::d3d9real::test_forwarding(
                &mut s.e,
                Box::new(b),
                threaded,
                &[device],
                &objects,
            );
            s
        };
        // The calls recorded for the render thread, which carries them out on its own core.
        let mut t = forwarding(true);
        let device = t.device;
        let recorded = run(&mut t, device, None);
        drop(t);
        let mut s = forwarding(false);
        let device = s.device;
        let forwarded = run(&mut s, device, None);
        eprintln!(
            "cost of a Direct3D 9 call: machinery {floor:.1} ns, model {model:.1} ns, \
             model + forwarding to a stand-in device {forwarded:.1} ns \
             ({draws} draws x 28 calls a frame: {:.2} ms), model + recording for the render \
             thread {recorded:.1} ns ({:.2} ms)",
            forwarded * f64::from(draws as u32 * 28) / 1e6,
            recorded * f64::from(draws as u32 * 28) / 1e6
        );
        for (g, (name, _)) in GROUPS.iter().enumerate() {
            let ns = run(&mut s, device, Some(g));
            eprintln!(
                "  {name}: model {:.1} ns, forwarded {ns:.1} ns a call",
                model_groups[g]
            );
        }
        // A skinned mesh's bone palette: 90 registers in one call.
        let calls = 200_000u32;
        let mut best = f64::MAX;
        for _ in 0..5 {
            let t = std::time::Instant::now();
            for i in 0..calls {
                let data = s.constants + 16 * (i % 64);
                s.e.vcall(device, 4 * 94, &[30, data, 90]);
            }
            best = best.min(t.elapsed().as_nanos() as f64 / f64::from(calls));
        }
        eprintln!("  SetVertexShaderConstantF of 90 registers: forwarded {best:.1} ns a call");
    }

    // Texture levels kept outside the game's memory (`Store`, ADR-0015 step 3).

    /// `D3DFMT_A8R8G8B8`, `DXT1`; `D3DLOCK_READONLY`, `D3DLOCK_DISCARD`,
    /// `D3DLOCK_NO_DIRTY_UPDATE`, `D3DLOCK_NOSYSLOCK`.
    const A8R8G8B8: u32 = 0x15;
    const DXT1: u32 = u32::from_le_bytes(*b"DXT1");
    const READONLY: u32 = 0x10;
    const DISCARD: u32 = 0x2000;
    const NO_DIRTY_UPDATE: u32 = 0x8000;
    const NOSYSLOCK: u32 = 0x800;

    /// A device whose levels are kept as `store` says, on the test table with A8R8G8B8 and DXT1
    /// textures (managed, system memory or dynamic) and cube textures accepted (TEST DATA).
    fn texture_device(store: Store) -> (Engine, u32) {
        let (mut e, d3d, present) = d3d();
        set_texture_store(&mut e, store);
        let out = e.mem.alloc(4);
        e.vcall(d3d, CREATE_DEVICE, &[0, 1, 0x11c, 0x40, present, out]);
        let device = e.mem.u32(out);
        let table = test_table()
            + "\nDomain\td1\t15,31545844"
            + "\nCheckDeviceFormat\t0 1 16 0 3\t0\t\td1"
            + "\nCheckDeviceFormat\t0 1 16 200 3\t0\t\td1"
            + "\nCheckDeviceFormat\t0 1 16 0 5\t0\t\td1";
        d3d9::load_adapter_table(&mut e, &table);
        (e, device)
    }

    /// `CreateTexture(width, height, levels, usage, format, pool)`: the texture.
    fn new_texture(e: &mut Engine, device: u32, words: [u32; 6]) -> u32 {
        let out = e.mem.alloc(4);
        let [w, h, levels, usage, format, pool] = words;
        let hr = e
            .vcall(device, 4 * 23, &[w, h, levels, usage, format, pool, out, 0])
            .u32();
        assert_eq!(hr, D3D_OK);
        let texture = e.mem.u32(out);
        e.mem.free(out);
        texture
    }

    /// `LockRect` of a texture's level (`face` `Some` for a cube texture's), the rectangle
    /// `rect` or the whole level: the answer, the pitch and the address handed out.
    fn lock(
        e: &mut Engine,
        texture: u32,
        face: Option<u32>,
        level: u32,
        rect: Option<[u32; 4]>,
        flags: u32,
    ) -> (u32, u32, u32) {
        let locked = e.mem.alloc(8);
        let r = rect.map_or(0, |r| {
            let at = e.mem.alloc(16);
            for (i, v) in r.iter().enumerate() {
                e.mem.set_u32(at + 4 * i as u32, *v);
            }
            at
        });
        let hr = match face {
            Some(f) => e.vcall(texture, 4 * 19, &[f, level, locked, r, flags]),
            None => e.vcall(texture, 4 * 19, &[level, locked, r, flags]),
        }
        .u32();
        let answer = (hr, e.mem.u32(locked), e.mem.u32(locked + 4));
        e.mem.free(locked);
        e.mem.free(r);
        answer
    }

    fn unlock(e: &mut Engine, texture: u32, face: Option<u32>, level: u32) -> u32 {
        match face {
            Some(f) => e.vcall(texture, 4 * 20, &[f, level]),
            None => e.vcall(texture, 4 * 20, &[level]),
        }
        .u32()
    }

    /// The bytes a whole-level `LockRect(D3DLOCK_READONLY)` hands out (`size` of them).
    fn read_level(
        e: &mut Engine,
        texture: u32,
        face: Option<u32>,
        level: u32,
        size: u32,
    ) -> Vec<u8> {
        let (hr, _, at) = lock(e, texture, face, level, None, READONLY);
        assert_eq!(hr, D3D_OK);
        let bytes = e.mem.bytes(at, size);
        assert_eq!(unlock(e, texture, face, level), D3D_OK);
        bytes
    }

    /// A pattern of `size` bytes from `seed`.
    fn pattern(seed: u32, size: u32) -> Vec<u8> {
        (0..size)
            .map(|i| (i.wrapping_mul(31) ^ seed) as u8)
            .collect()
    }

    #[test]
    fn a_level_kept_outside_is_in_the_games_memory_only_while_locked() {
        let (mut e, device) = texture_device(Store::Host);
        let before = e.mem.heap_available();
        // 256 by 256 A8R8G8B8, the full chain (9 levels, 349,524 bytes), managed.
        let texture = new_texture(&mut e, device, [256, 256, 0, 0, A8R8G8B8, 1]);
        let taken = before - e.mem.heap_available();
        assert!(taken < 4096, "only the objects: {taken} bytes");
        let (hr, pitch, at) = lock(&mut e, texture, None, 0, None, 0);
        assert_eq!((hr, pitch), (D3D_OK, 1024));
        assert_eq!(e.mem.block_size(at), Some(256 * 1024), "a staging block");
        assert_eq!(
            e.mem.bytes(at, 256 * 1024),
            vec![0; 256 * 1024],
            "zero until written"
        );
        let bytes = pattern(7, 256 * 1024);
        e.mem.write(at, &bytes);
        // Locked once already: refused, the block stays.
        assert_eq!(
            lock(&mut e, texture, None, 0, None, 0).0,
            D3DERR_INVALIDCALL
        );
        assert_eq!(e.mem.block_size(at), Some(256 * 1024));
        assert_eq!(unlock(&mut e, texture, None, 0), D3D_OK);
        // Until the next Present the block still holds the level's bytes (code reading through
        // the lock's address just after the unlock reads them); then it is freed.
        assert_eq!(e.mem.bytes(at, 256 * 1024), bytes);
        assert_eq!(unlock(&mut e, texture, None, 0), D3DERR_INVALIDCALL);
        assert_eq!(present(&mut e, device), D3D_OK);
        assert_eq!(e.mem.block_size(at), None, "freed at the Present");
        assert_eq!(before - e.mem.heap_available(), taken, "nothing kept");
        assert_eq!(read_level(&mut e, texture, None, 0, 256 * 1024), bytes);
        // The other levels are their own.
        assert_eq!(
            read_level(&mut e, texture, None, 1, 128 * 512),
            vec![0; 128 * 512]
        );
        // A rectangle: (16, 8) is 8 rows of 1024 bytes and 16 pixels of 4 bytes in.
        let (hr, pitch, at) = lock(&mut e, texture, None, 0, Some([16, 8, 32, 24]), 0);
        assert_eq!((hr, pitch), (D3D_OK, 1024));
        let start = 8 * 1024 + 16 * 4;
        assert_eq!(e.mem.bytes(at, 64), bytes[start..start + 64]);
        e.mem.write(at, &[0xee; 4]);
        assert_eq!(unlock(&mut e, texture, None, 0), D3D_OK);
        let mut expected = bytes.clone();
        expected[start..start + 4].copy_from_slice(&[0xee; 4]);
        assert_eq!(read_level(&mut e, texture, None, 0, 256 * 1024), expected);
        // Released: every block it had is free.
        assert_eq!(e.vcall(texture, 8, &[]).u32(), 0);
        assert_eq!(present(&mut e, device), D3D_OK);
        assert!(before - e.mem.heap_available() < taken);
    }

    /// `Present()` on the device (outside a scene).
    fn present(e: &mut Engine, device: u32) -> u32 {
        e.vcall(device, 4 * 17, &[0, 0, 0, 0]).u32()
    }

    #[test]
    fn unlocked_staging_blocks_wait_for_the_present_up_to_16_mb() {
        let (mut e, device) = texture_device(Store::Host);
        // Five 4 MB levels unlocked in one frame: 20 MB, so the oldest goes before the Present.
        let mut blocks = Vec::new();
        for i in 0..5 {
            let texture = new_texture(&mut e, device, [1024, 1024, 1, 0, A8R8G8B8, 1]);
            let (_, _, at) = lock(&mut e, texture, None, 0, None, 0);
            e.mem.write(at, &pattern(i, 64));
            assert_eq!(unlock(&mut e, texture, None, 0), D3D_OK);
            assert_eq!(e.mem.bytes(at, 64), pattern(i, 64), "readable just after");
            blocks.push(at);
        }
        assert_eq!(
            e.mem.block_size(blocks[0]),
            None,
            "the oldest freed past 16 MB"
        );
        for &at in &blocks[1..] {
            assert_eq!(e.mem.block_size(at), Some(4 << 20));
        }
        assert_eq!(present(&mut e, device), D3D_OK);
        for &at in &blocks {
            assert_eq!(e.mem.block_size(at), None);
        }
        // One level bigger than 16 MB still waits for the Present.
        let big = new_texture(&mut e, device, [4096, 2048, 1, 0, A8R8G8B8, 1]);
        let (_, _, at) = lock(&mut e, big, None, 0, None, 0);
        assert_eq!(unlock(&mut e, big, None, 0), D3D_OK);
        assert!(e.mem.block_size(at).is_some());
        assert_eq!(present(&mut e, device), D3D_OK);
        assert_eq!(e.mem.block_size(at), None);
    }

    #[test]
    fn lock_flags_change_nothing_the_game_sees() {
        for store in [Store::Host] {
            let (mut e, device) = texture_device(store);
            // Dynamic in the default pool, and managed.
            for (usage, pool) in [(0x200, 0), (0, 1)] {
                let texture = new_texture(&mut e, device, [8, 8, 1, usage, A8R8G8B8, pool]);
                let size = 8 * 32;
                for (i, flags) in [
                    0,
                    READONLY,
                    DISCARD,
                    NO_DIRTY_UPDATE,
                    NOSYSLOCK,
                    READONLY | NO_DIRTY_UPDATE,
                ]
                .into_iter()
                .enumerate()
                {
                    let before = read_level(&mut e, texture, None, 0, size);
                    let (hr, pitch, at) = lock(&mut e, texture, None, 0, None, flags);
                    assert_eq!((hr, pitch), (D3D_OK, 32), "{store:?} flags {flags:#x}");
                    // The bytes as they were (a discard too), and what is written is kept (a
                    // read-only lock's too: the flags change nothing here).
                    assert_eq!(e.mem.bytes(at, size), before, "{store:?} flags {flags:#x}");
                    let bytes = pattern(i as u32, size);
                    e.mem.write(at, &bytes);
                    assert_eq!(unlock(&mut e, texture, None, 0), D3D_OK);
                    assert_eq!(read_level(&mut e, texture, None, 0, size), bytes);
                }
            }
        }
    }

    #[test]
    fn mip_levels_cube_faces_and_blocks_are_kept_apart() {
        let (mut e, device) = texture_device(Store::Host);
        // DXT1 16 by 16, three levels: 4 rows of 4 blocks, 2 of 2, 1 of 1 (8 bytes a block).
        let dxt = new_texture(&mut e, device, [16, 16, 3, 0, DXT1, 1]);
        let sizes = [128, 32, 8];
        for (level, size) in sizes.iter().enumerate() {
            let (hr, pitch, at) = lock(&mut e, dxt, None, level as u32, None, 0);
            assert_eq!((hr, pitch), (D3D_OK, [32, 16, 8][level]));
            e.mem.write(at, &pattern(level as u32, *size));
            assert_eq!(unlock(&mut e, dxt, None, level as u32), D3D_OK);
        }
        for (level, size) in sizes.iter().enumerate() {
            assert_eq!(
                read_level(&mut e, dxt, None, level as u32, *size),
                pattern(level as u32, *size)
            );
        }
        // A rectangle of a DXT level starts at its block: (8, 4) is block row 1, block 2.
        let (_, _, at) = lock(&mut e, dxt, None, 0, Some([8, 4, 12, 8]), 0);
        assert_eq!(e.mem.bytes(at, 8), pattern(0, 128)[32 + 16..32 + 24]);
        assert_eq!(unlock(&mut e, dxt, None, 0), D3D_OK);
        // A cube texture of edge 4, two levels: twelve levels of their own.
        let out = e.mem.alloc(4);
        assert_eq!(
            e.vcall(device, 4 * 25, &[4, 2, 0, A8R8G8B8, 1, out, 0])
                .u32(),
            D3D_OK
        );
        let cube = e.mem.u32(out);
        for face in 0..6 {
            for level in 0..2 {
                let (hr, _, at) = lock(&mut e, cube, Some(face), level, None, 0);
                assert_eq!(hr, D3D_OK);
                e.mem
                    .write(at, &pattern(face * 2 + level, [64, 16][level as usize]));
                assert_eq!(unlock(&mut e, cube, Some(face), level), D3D_OK);
            }
        }
        for face in 0..6 {
            for level in 0..2 {
                let size = [64, 16][level as usize];
                assert_eq!(
                    read_level(&mut e, cube, Some(face), level, size),
                    pattern(face * 2 + level, size)
                );
            }
        }
        assert_eq!(
            lock(&mut e, cube, Some(6), 0, None, 0).0,
            D3DERR_INVALIDCALL
        );
    }

    #[test]
    fn default_pool_levels_have_nothing_to_lock_and_a_destroyed_lock_frees_its_block() {
        let (mut e, device) = texture_device(Store::Host);
        let target = new_texture(&mut e, device, [8, 8, 1, 0, A8R8G8B8, 0]);
        assert_eq!(lock(&mut e, target, None, 0, None, 0).0, D3DERR_INVALIDCALL);
        let texture = new_texture(&mut e, device, [64, 64, 1, 0, A8R8G8B8, 1]);
        let (_, _, at) = lock(&mut e, texture, None, 0, None, 0);
        assert!(e.mem.block_size(at).is_some());
        assert_eq!(e.vcall(texture, 8, &[]).u32(), 0);
        assert_eq!(e.mem.block_size(at), None);
    }

    #[test]
    fn managed_levels_keep_their_bytes_across_a_reset() {
        let (mut e, d3d, present) = d3d();
        set_texture_store(&mut e, Store::Host);
        let out = e.mem.alloc(4);
        e.vcall(d3d, CREATE_DEVICE, &[0, 1, 0x11c, 0x40, present, out]);
        let device = e.mem.u32(out);
        let table = test_table() + "\nDomain\td1\t15\nCheckDeviceFormat\t0 1 16 0 3\t0\t\td1";
        d3d9::load_adapter_table(&mut e, &table);
        let texture = new_texture(&mut e, device, [4, 4, 1, 0, A8R8G8B8, 1]);
        let (_, _, at) = lock(&mut e, texture, None, 0, None, 0);
        e.mem.write(at, &pattern(3, 64));
        assert_eq!(unlock(&mut e, texture, None, 0), D3D_OK);
        assert_eq!(e.vcall(device, 4 * 16, &[present]).u32(), D3D_OK);
        assert_eq!(read_level(&mut e, texture, None, 0, 64), pattern(3, 64));
    }

    #[test]
    fn color_fill_reaches_a_dynamic_level_kept_outside() {
        for store in [Store::Host] {
            let (mut e, device) = texture_device(store);
            let texture = new_texture(&mut e, device, [4, 4, 1, 0x200, A8R8G8B8, 0]);
            let out = e.mem.alloc(4);
            assert_eq!(e.vcall(texture, 4 * 18, &[0, out]).u32(), D3D_OK);
            let level = e.mem.u32(out);
            let rect = e.mem.alloc(16);
            for (i, v) in [1u32, 1, 3, 2].iter().enumerate() {
                e.mem.set_u32(rect + 4 * i as u32, *v);
            }
            assert_eq!(
                e.vcall(device, 4 * 35, &[level, rect, 0x1122_3344]).u32(),
                D3D_OK
            );
            let bytes = read_level(&mut e, texture, None, 0, 64);
            let mut expected = vec![0u8; 64];
            for x in 1..3 {
                expected[16 + 4 * x..16 + 4 * x + 4].copy_from_slice(&0x1122_3344u32.to_le_bytes());
            }
            assert_eq!(bytes, expected, "{store:?}");
        }
    }

    /// On a real device (`NV_D3D9=real`), a managed texture's levels keep their bytes in the real
    /// levels: what the game wrote comes back through its next locks (a rectangle's too), the real
    /// level holds it, and a level never written comes back zero. Skipped where Direct3D 9 has no
    /// device (no display adapter, as on CI).
    #[cfg(windows)]
    #[test]
    fn real_levels_keep_the_bytes_of_managed_textures() {
        real_levels_keep_the_bytes(false);
    }

    /// [`real_levels_keep_the_bytes_of_managed_textures`] with the real device on the render
    /// thread: the reads wait for the uploads before them.
    #[cfg(windows)]
    #[test]
    fn real_levels_keep_the_bytes_of_managed_textures_from_the_render_thread() {
        real_levels_keep_the_bytes(true);
    }

    #[cfg(windows)]
    fn real_levels_keep_the_bytes(threaded: bool) {
        let _turn = super::super::d3d9real::real_turn();
        let extra = "\nDomain\td2\t15,31545844\nCheckDeviceFormat\t0 1 16 0 3\t0\t\td2\
                     \nCheckDeviceFormat\t0 1 16 0 5\t0\t\td2";
        let (mut e, device) = device_with(extra, 0x54);
        set_texture_store(&mut e, Store::Host);
        if super::super::d3d9real::test_real_device_on(&mut e, device, 0x54, threaded).is_none() {
            return;
        }
        let texture = new_texture(&mut e, device, [64, 32, 0, 0, A8R8G8B8, 1]);
        let levels = device!(e).device_objects.textures[&texture].levels.clone();
        for &level in &levels {
            assert!(matches!(
                surface_mut(&mut e, level).pixels,
                Some(Pixels::Real { written: false })
            ));
        }
        let bytes = pattern(9, 64 * 32 * 4);
        let (hr, pitch, at) = lock(&mut e, texture, None, 0, None, 0);
        assert_eq!((hr, pitch), (D3D_OK, 256));
        e.mem.write(at, &bytes);
        assert_eq!(unlock(&mut e, texture, None, 0), D3D_OK);
        assert!(matches!(
            surface_mut(&mut e, levels[0]).pixels,
            Some(Pixels::Real { written: true })
        ));
        assert_eq!(read_level(&mut e, texture, None, 0, 64 * 32 * 4), bytes);
        assert_eq!(
            read_level(&mut e, texture, None, 1, 32 * 16 * 4),
            vec![0; 2048]
        );
        let (_, _, at) = lock(&mut e, texture, None, 0, Some([4, 2, 8, 4]), 0);
        assert_eq!(e.mem.bytes(at, 16), bytes[2 * 256 + 16..2 * 256 + 32]);
        assert_eq!(unlock(&mut e, texture, None, 0), D3D_OK);
        // The real level itself.
        let block = e.mem.alloc(64 * 32 * 4);
        assert!(super::super::d3d9real::read_surface(
            &mut e,
            levels[0],
            block,
            64 * 32 * 4
        ));
        assert_eq!(e.mem.bytes(block, 64 * 32 * 4), bytes);
        // A cube texture's faces, DXT1.
        let out = e.mem.alloc(4);
        assert_eq!(
            e.vcall(device, 4 * 25, &[8, 1, 0, A8R8G8B8, 1, out, 0])
                .u32(),
            D3D_OK
        );
        let cube = e.mem.u32(out);
        for face in 0..6 {
            let (_, _, at) = lock(&mut e, cube, Some(face), 0, None, 0);
            e.mem.write(at, &pattern(face, 256));
            assert_eq!(unlock(&mut e, cube, Some(face), 0), D3D_OK);
        }
        for face in 0..6 {
            assert_eq!(
                read_level(&mut e, cube, Some(face), 0, 256),
                pattern(face, 256)
            );
        }
        let dxt = new_texture(&mut e, device, [8, 8, 1, 0, DXT1, 1]);
        let (_, pitch, at) = lock(&mut e, dxt, None, 0, None, 0);
        assert_eq!(pitch, 16);
        e.mem.write(at, &pattern(5, 32));
        assert_eq!(unlock(&mut e, dxt, None, 0), D3D_OK);
        assert_eq!(read_level(&mut e, dxt, None, 0, 32), pattern(5, 32));
        assert!(
            device!(e).real.logged_is_empty(),
            "no answer of the real device differed"
        );
    }

    #[test]
    fn the_check_store_reports_a_write_after_the_unlock() {
        let (mut e, device) = texture_device(Store::Check);
        let texture = new_texture(&mut e, device, [4, 4, 1, 0, A8R8G8B8, 1]);
        let (_, _, at) = lock(&mut e, texture, None, 0, None, 0);
        e.mem.write(at, &pattern(1, 64));
        assert_eq!(unlock(&mut e, texture, None, 0), D3D_OK);
        assert_eq!(
            device!(e).device_objects.round_trips,
            [1, 0],
            "read back the same"
        );
        assert!(e.mem.block_size(at).is_some(), "kept until the Present");
        e.mem.set_u32(at + 8, 0xdead_beef);
        let level = device!(e).device_objects.textures[&texture].levels[0];
        assert_eq!(e.vcall(device, 4 * 17, &[0, 0, 0, 0]).u32(), D3D_OK);
        assert!(device!(e)
            .device_objects
            .written_after_unlock
            .contains(&level));
        assert_eq!(e.mem.block_size(at), None, "freed at the Present");
        // The texture has what it held at the unlock.
        assert_eq!(read_level(&mut e, texture, None, 0, 64), pattern(1, 64));
    }

    // Static buffers kept outside the game's memory (`d3d9resource::Kept`, ADR-0015 step 3).

    /// `D3DLOCK_NOOVERWRITE`; `D3DUSAGE_WRITEONLY`, `D3DUSAGE_DYNAMIC`.
    const NOOVERWRITE: u32 = 0x1000;
    const WRITEONLY: u32 = 0x8;
    const DYNAMIC: u32 = 0x200;

    /// A device whose static buffers are kept as `store` says.
    fn buffer_device(store: Store) -> (Engine, u32) {
        let (mut e, d3d, present) = d3d();
        set_buffer_store(&mut e, store);
        let out = e.mem.alloc(4);
        e.vcall(d3d, CREATE_DEVICE, &[0, 1, 0x11c, 0x40, present, out]);
        let device = e.mem.u32(out);
        (e, device)
    }

    /// `CreateVertexBuffer` (FVF 0x142) or `CreateIndexBuffer` (16-bit): the buffer.
    fn new_buffer(
        e: &mut Engine,
        device: u32,
        index: bool,
        length: u32,
        usage: u32,
        pool: u32,
    ) -> u32 {
        let out = e.mem.alloc(4);
        let hr = if index {
            e.vcall(device, 4 * 27, &[length, usage, 101, pool, out, 0])
        } else {
            e.vcall(device, 4 * 26, &[length, usage, 0x142, pool, out, 0])
        }
        .u32();
        assert_eq!(hr, D3D_OK);
        let buffer = e.mem.u32(out);
        e.mem.free(out);
        buffer
    }

    /// `Lock(offset, size, flags)` of a buffer: the answer and the address handed out.
    fn lock_buffer(e: &mut Engine, buffer: u32, offset: u32, size: u32, flags: u32) -> (u32, u32) {
        let out = e.mem.alloc(4);
        let hr = e.vcall(buffer, 4 * 11, &[offset, size, out, flags]).u32();
        let at = e.mem.u32(out);
        e.mem.free(out);
        (hr, at)
    }

    fn unlock_buffer(e: &mut Engine, buffer: u32) -> u32 {
        e.vcall(buffer, 4 * 12, &[]).u32()
    }

    /// The bytes a whole-buffer `Lock(D3DLOCK_READONLY)` hands out (`length` of them).
    fn read_buffer_bytes(e: &mut Engine, buffer: u32, length: u32) -> Vec<u8> {
        let (hr, at) = lock_buffer(e, buffer, 0, 0, READONLY);
        assert_eq!(hr, D3D_OK);
        let bytes = e.mem.bytes(at, length);
        assert_eq!(unlock_buffer(e, buffer), D3D_OK);
        bytes
    }

    fn buffer_kept(e: &mut Engine, buffer: u32) -> &'static str {
        device!(e).device_objects.buffer_kept(buffer).0
    }

    #[test]
    fn a_static_buffer_is_in_the_games_memory_only_while_locked() {
        let (mut e, device) = buffer_device(Store::Host);
        for index in [false, true] {
            let before = e.mem.heap_available();
            let buffer = new_buffer(&mut e, device, index, 0x10000, WRITEONLY, 1);
            let taken = before - e.mem.heap_available();
            assert!(taken < 0x100, "only the object: {taken}");
            assert_eq!(buffer_kept(&mut e, buffer), "host");
            let (hr, at) = lock_buffer(&mut e, buffer, 0, 0, 0);
            assert_eq!(hr, D3D_OK);
            assert_eq!(e.mem.block_size(at), Some(0x10000), "a staging block");
            assert_eq!(
                e.mem.bytes(at, 0x10000),
                vec![0; 0x10000],
                "zero until written"
            );
            e.mem.write(at, &pattern(7, 0x10000));
            assert_eq!(unlock_buffer(&mut e, buffer), D3D_OK);
            assert_eq!(unlock_buffer(&mut e, buffer), D3DERR_INVALIDCALL);
            // Readable just after the unlock, freed at the Present.
            assert_eq!(e.mem.bytes(at, 64), pattern(7, 64));
            assert_eq!(present(&mut e, device), D3D_OK);
            assert_eq!(e.mem.block_size(at), None);
            assert_eq!(before - e.mem.heap_available(), taken, "nothing kept");
            assert_eq!(
                read_buffer_bytes(&mut e, buffer, 0x10000),
                pattern(7, 0x10000)
            );
            assert_eq!(present(&mut e, device), D3D_OK);
            assert_eq!(e.vcall(buffer, 8, &[]).u32(), 0);
        }
        // A dynamic buffer stays in the game's memory: the same address each lock.
        let dynamic = new_buffer(&mut e, device, false, 256, DYNAMIC | WRITEONLY, 0);
        assert_eq!(buffer_kept(&mut e, dynamic), "guest");
        let (_, first) = lock_buffer(&mut e, dynamic, 0, 0, DISCARD);
        assert_eq!(unlock_buffer(&mut e, dynamic), D3D_OK);
        let (_, again) = lock_buffer(&mut e, dynamic, 64, 32, NOOVERWRITE);
        assert_eq!(again, first + 64);
        assert_eq!(unlock_buffer(&mut e, dynamic), D3D_OK);
    }

    #[test]
    fn buffer_ranges_nested_locks_and_flags_change_nothing_the_game_sees() {
        for store in [Store::Host] {
            let (mut e, device) = buffer_device(store);
            // Managed, default write-only, system memory, dynamic; vertex and index.
            for (index, usage, pool) in [
                (false, 0, 1),
                (false, WRITEONLY, 0),
                (true, 0, 2),
                (false, DYNAMIC | WRITEONLY, 0),
                (true, WRITEONLY, 1),
            ] {
                let what = format!("{store:?} index {index} usage {usage:#x} pool {pool}");
                let length = 256;
                let buffer = new_buffer(&mut e, device, index, length, usage, pool);
                let mut expected = vec![0u8; length as usize];
                for (i, flags) in [
                    0,
                    READONLY,
                    DISCARD,
                    NOOVERWRITE,
                    NOSYSLOCK,
                    DISCARD | NOSYSLOCK,
                ]
                .into_iter()
                .enumerate()
                {
                    // A range, the whole buffer (0, 0), and from an offset to the end (size 0).
                    for (offset, size) in [(16, 32), (0, 0), (200, 0), (0, length)] {
                        let what = format!("{what} flags {flags:#x} range {offset} {size}");
                        let (hr, at) = lock_buffer(&mut e, buffer, offset, size, flags);
                        assert_eq!(hr, D3D_OK, "{what}");
                        let end = if size == 0 { length } else { offset + size };
                        let range = offset as usize..end as usize;
                        // The bytes as they were (after a discard too), and what is written is
                        // kept (through a read-only lock too: the flags change nothing here).
                        assert_eq!(
                            e.mem.bytes(at, end - offset),
                            expected[range.clone()],
                            "{what}"
                        );
                        let bytes = pattern(i as u32 * 7 + offset, end - offset);
                        e.mem.write(at, &bytes);
                        expected[range].copy_from_slice(&bytes);
                        assert_eq!(unlock_buffer(&mut e, buffer), D3D_OK);
                        assert_eq!(
                            read_buffer_bytes(&mut e, buffer, length),
                            expected,
                            "{what}"
                        );
                    }
                }
                // Nested locks share one block: the inner one at its offset in it, every byte
                // written through either kept at the last unlock.
                let (_, outer) = lock_buffer(&mut e, buffer, 0, 128, 0);
                let (_, inner) = lock_buffer(&mut e, buffer, 96, 64, NOOVERWRITE);
                assert_eq!(inner, outer + 96, "{what}");
                e.mem.write(inner, &pattern(1, 64));
                assert_eq!(unlock_buffer(&mut e, buffer), D3D_OK);
                assert!(e.mem.block_size(outer).is_some());
                e.mem.write(outer, &pattern(2, 16));
                assert_eq!(unlock_buffer(&mut e, buffer), D3D_OK);
                expected[96..160].copy_from_slice(&pattern(1, 64));
                expected[..16].copy_from_slice(&pattern(2, 16));
                assert_eq!(
                    read_buffer_bytes(&mut e, buffer, length),
                    expected,
                    "{what}"
                );
                // A range past the end and a null out are refused and leave nothing locked.
                assert_eq!(lock_buffer(&mut e, buffer, 250, 8, 0).0, D3DERR_INVALIDCALL);
                assert_eq!(
                    e.vcall(buffer, 4 * 11, &[0, 0, 0, 0]).u32(),
                    D3DERR_INVALIDCALL
                );
                assert_eq!(unlock_buffer(&mut e, buffer), D3DERR_INVALIDCALL);
                assert_eq!(present(&mut e, device), D3D_OK);
            }
        }
    }

    #[test]
    fn a_buffer_released_while_locked_frees_its_block_and_managed_ones_survive_a_reset() {
        let (mut e, device) = buffer_device(Store::Host);
        let buffer = new_buffer(&mut e, device, false, 4096, 0, 1);
        let (_, at) = lock_buffer(&mut e, buffer, 0, 0, 0);
        assert!(e.mem.block_size(at).is_some());
        assert_eq!(e.vcall(buffer, 8, &[]).u32(), 0);
        assert_eq!(e.mem.block_size(at), None);
        // A managed index buffer's bytes across a Reset.
        let buffer = new_buffer(&mut e, device, true, 64, WRITEONLY, 1);
        let (_, at) = lock_buffer(&mut e, buffer, 0, 64, 0);
        e.mem.write(at, &pattern(3, 64));
        assert_eq!(unlock_buffer(&mut e, buffer), D3D_OK);
        let present_parameters = e.mem.alloc(PRESENT_SIZE);
        let parameters = device_present_parameters(&e, device).unwrap();
        e.mem.write(present_parameters, &parameters);
        assert_eq!(e.vcall(device, 4 * 16, &[present_parameters]).u32(), D3D_OK);
        assert_eq!(read_buffer_bytes(&mut e, buffer, 64), pattern(3, 64));
    }

    #[test]
    fn the_buffer_check_store_reports_a_write_after_the_last_unlock() {
        let (mut e, device) = buffer_device(Store::Check);
        let buffer = new_buffer(&mut e, device, false, 64, WRITEONLY, 1);
        let (_, at) = lock_buffer(&mut e, buffer, 0, 0, 0);
        e.mem.write(at, &pattern(1, 64));
        assert_eq!(unlock_buffer(&mut e, buffer), D3D_OK);
        assert_eq!(
            device!(e).device_objects.buffer_kept(buffer).1,
            [1, 0],
            "read back the same"
        );
        assert!(e.mem.block_size(at).is_some(), "kept until the Present");
        e.mem.set_u32(at + 8, 0xdead_beef);
        assert_eq!(present(&mut e, device), D3D_OK);
        assert!(written_after_unlock(&mut e, buffer));
        assert_eq!(e.mem.block_size(at), None, "freed at the Present");
        // The buffer has what it held at the unlock.
        assert_eq!(read_buffer_bytes(&mut e, buffer, 64), pattern(1, 64));
    }

    /// On a real device (`NV_D3D9=real`), static buffers of every pool keep their bytes in host
    /// memory, never read back from the real buffer (the game's are write-only, whose reads
    /// Direct3D leaves undefined): what the game wrote comes back through its next locks, a
    /// whole buffer, a range, nested locks and a write through a read-only lock alike, and
    /// nothing the real device answered differed. Skipped where Direct3D 9 has no device (no
    /// display adapter, as on CI).
    #[cfg(windows)]
    #[test]
    fn static_buffers_on_a_real_device_keep_their_bytes_in_host_memory() {
        static_buffers_keep_their_bytes(false);
    }

    /// The same with the real device on the render thread.
    #[cfg(windows)]
    #[test]
    fn static_buffers_on_a_real_device_keep_their_bytes_from_the_render_thread() {
        static_buffers_keep_their_bytes(true);
    }

    #[cfg(windows)]
    fn static_buffers_keep_their_bytes(threaded: bool) {
        let _turn = super::super::d3d9real::real_turn();
        let (mut e, device) = device_with("", 0x54);
        set_buffer_store(&mut e, Store::Host);
        if super::super::d3d9real::test_real_device_on(&mut e, device, 0x54, threaded).is_none() {
            return;
        }
        for index in [false, true] {
            // Write-only and managed, as the game's geometry chips; system memory; default.
            for (usage, pool) in [(WRITEONLY, 1), (0, 2), (WRITEONLY, 0)] {
                let length = 0x4000;
                let buffer = new_buffer(&mut e, device, index, length, usage, pool);
                assert_eq!(buffer_kept(&mut e, buffer), "host");
                assert_eq!(
                    read_buffer_bytes(&mut e, buffer, length),
                    vec![0; length as usize]
                );
                let mut expected = pattern(4, length);
                let (_, at) = lock_buffer(&mut e, buffer, 0, 0, 0);
                e.mem.write(at, &expected);
                assert_eq!(unlock_buffer(&mut e, buffer), D3D_OK);
                assert_eq!(read_buffer_bytes(&mut e, buffer, length), expected);
                let (_, outer) = lock_buffer(&mut e, buffer, 0x100, 0x40, 0);
                let (_, inner) = lock_buffer(&mut e, buffer, 0x120, 0x10, NOSYSLOCK);
                assert_eq!(inner, outer + 0x20);
                e.mem.write(outer, &pattern(5, 0x40));
                assert_eq!(unlock_buffer(&mut e, buffer), D3D_OK);
                assert_eq!(unlock_buffer(&mut e, buffer), D3D_OK);
                expected[0x100..0x140].copy_from_slice(&pattern(5, 0x40));
                let (_, at) = lock_buffer(&mut e, buffer, 0, 0, READONLY);
                e.mem.write(at, &[1, 2, 3]);
                assert_eq!(unlock_buffer(&mut e, buffer), D3D_OK);
                expected[..3].copy_from_slice(&[1, 2, 3]);
                assert_eq!(read_buffer_bytes(&mut e, buffer, length), expected);
                assert_eq!(present(&mut e, device), D3D_OK);
                assert_eq!(e.vcall(buffer, 8, &[]).u32(), 0);
            }
        }
        assert!(
            device!(e).real.logged_is_empty(),
            "no answer of the real device differed"
        );
        eprintln!(
            "static_buffers_on_a_real_device_keep_their_bytes_in_host_memory: ran on a real device"
        );
    }

    // --- Step 2's two producers (`super::super::d3d9draw`) ---

    /// `GetRenderState(state)` as the calling side sees it.
    fn render_state(e: &mut Engine, device: u32, state: u32) -> u32 {
        let out = e.mem.alloc(4);
        assert_eq!(
            get_render_state(e, Ptr::new(device), state, Ptr::new(out)),
            D3D_OK
        );
        let v = e.mem.u32(out);
        e.mem.free(out);
        v
    }

    /// `GetTexture(sampler)` as the calling side sees it (the count it adds given back).
    fn texture_at(e: &mut Engine, device: u32, sampler: u32) -> u32 {
        let out = e.mem.alloc(4);
        assert_eq!(
            get_texture(e, Ptr::new(device), sampler, Ptr::new(out)),
            D3D_OK
        );
        let t = e.mem.u32(out);
        e.mem.free(out);
        if t != 0 {
            texture_release(e, Ptr::new(t));
        }
        t
    }

    /// While a cut is open on the engine's devices only the drawing side leaves repeats out (main's
    /// model and the stream's order differ then), and nobody with `NV_D3D9_REPEATS=keep`.
    #[test]
    fn repeats_are_kept_by_main_while_a_cut_is_open() {
        use super::super::d3d9draw::{as_drawing_side, draw_cut, draw_join};
        let (mut e, dev) = texture_device(Store::Host);
        keep_repeats(&mut e, false);
        assert!(skip_repeats(&mut e));
        assert!(draw_cut(&mut e, dev));
        assert!(!skip_repeats(&mut e), "main, cut open");
        as_drawing_side(|| assert!(skip_repeats(&mut e), "the drawing side, cut open"));
        keep_repeats(&mut e, true);
        as_drawing_side(|| assert!(!skip_repeats(&mut e), "the drawing side, kept"));
        keep_repeats(&mut e, false);
        draw_join(&mut e);
        assert!(skip_repeats(&mut e), "after the join");
        keep_repeats(&mut e, true);
        assert!(!skip_repeats(&mut e), "kept");
    }

    /// The drawing side of a cut starts from main's state, changes only its own, and the join
    /// gives main the serial order's state: the drawing side's value where main set nothing after
    /// the cut, main's where it did (also when it set a state again to the value it had). A second
    /// cut while one is open is refused, and so is a cut from the drawing side.
    #[test]
    fn the_drawing_side_has_its_own_state_and_the_join_gives_the_serial_one() {
        use super::super::d3d9draw::{as_drawing_side, draw_cut, draw_join, draw_open};
        let (mut e, dev) = texture_device(Store::Host);
        let d = Ptr::new(dev);
        for (state, value) in [(7, 1), (8, 1), (9, 1), (10, 1)] {
            set_render_state(&mut e, d, state, value);
        }
        set_viewport_words(&mut e, dev, [0, 0, 320, 240]);
        assert!(draw_cut(&mut e, dev));
        assert!(draw_open(&mut e));
        assert!(!draw_cut(&mut e, dev), "one cut at a time");
        as_drawing_side(|| {
            assert!(!draw_cut(&mut e, dev), "not from the drawing side");
            assert_eq!(render_state(&mut e, dev, 7), 1, "main's state at the cut");
            set_render_state(&mut e, d, 7, 2);
            set_render_state(&mut e, d, 8, 2);
            set_render_state(&mut e, d, 10, 2);
            set_viewport_words(&mut e, dev, [0, 0, 100, 100]);
            assert_eq!(render_state(&mut e, dev, 7), 2);
        });
        // Main, meanwhile: its own state, the drawing side's changes unseen.
        assert_eq!(render_state(&mut e, dev, 7), 1);
        assert_eq!(render_state(&mut e, dev, 10), 1);
        assert_eq!(device(&mut e, d).viewport[2], 320);
        set_render_state(&mut e, d, 8, 3);
        // Set again to the value it has: still main's after the join (it came after the draw).
        set_render_state(&mut e, d, 10, 1);
        as_drawing_side(|| set_render_state(&mut e, d, 9, 5));
        draw_join(&mut e);
        assert!(!draw_open(&mut e));
        assert_eq!(
            render_state(&mut e, dev, 7),
            2,
            "the drawing side's: main left it"
        );
        assert_eq!(render_state(&mut e, dev, 8), 3, "main's: set after the cut");
        assert_eq!(render_state(&mut e, dev, 9), 5, "the drawing side's");
        assert_eq!(
            render_state(&mut e, dev, 10),
            1,
            "main's: set again after the cut"
        );
        assert_eq!(
            device(&mut e, d).viewport[2],
            100,
            "the drawing side's viewport"
        );
        // The next cut starts from the joined state.
        assert!(draw_cut(&mut e, dev));
        as_drawing_side(|| assert_eq!(render_state(&mut e, dev, 8), 3));
        draw_join(&mut e);
    }

    /// A texture both sides bind is held by each: main unbinding and releasing it during the cut
    /// leaves it alive for the drawing side, which still answers it; at the join main's choice
    /// (no texture, set after the cut) stands and the texture goes. One the drawing side binds
    /// alone becomes main's at the join, held once.
    #[test]
    fn a_texture_the_drawing_side_binds_lives_until_the_join() {
        use super::super::d3d9draw::{as_drawing_side, draw_cut, draw_join};
        let (mut e, dev) = texture_device(Store::Host);
        let d = Ptr::new(dev);
        let first = new_texture(&mut e, dev, [16, 16, 1, 0, A8R8G8B8, 1]);
        let second = new_texture(&mut e, dev, [16, 16, 1, 0, A8R8G8B8, 1]);
        assert_eq!(set_texture(&mut e, d, 0, first), D3D_OK);
        assert!(draw_cut(&mut e, dev));
        assert_eq!(
            texture(&mut e, Ptr::new(first)).bound,
            2,
            "held by main and the copy"
        );
        // Main lets go of it altogether.
        assert_eq!(set_texture(&mut e, d, 0, 0), D3D_OK);
        assert_eq!(texture_release(&mut e, Ptr::new(first)), 0);
        assert!(
            device!(e).device_objects.textures.contains_key(&first),
            "alive"
        );
        as_drawing_side(|| {
            assert_eq!(texture_at(&mut e, dev, 0), first);
            assert_eq!(set_texture(&mut e, d, 1, second), D3D_OK);
        });
        draw_join(&mut e);
        assert!(
            !device!(e).device_objects.textures.contains_key(&first),
            "main's choice stands and nothing holds it any more"
        );
        assert_eq!(texture_at(&mut e, dev, 0), 0);
        assert_eq!(
            texture_at(&mut e, dev, 1),
            second,
            "the drawing side's binding"
        );
        assert_eq!(texture(&mut e, Ptr::new(second)).bound, 1, "held once");
    }

    /// The bindings at a cut, as the states: the drawing side starts from main's (stream 0, the
    /// indices), its own changes are unseen by main, and the join gives the serial order's: the
    /// drawing side's stream where main set none after the cut, main's indices set after it.
    #[test]
    fn the_drawing_side_has_its_own_bindings_and_the_join_gives_the_serial_ones() {
        use super::super::d3d9draw::{as_drawing_side, draw_cut, draw_join};
        let (mut e, dev) = texture_device(Store::Host);
        let a = new_buffer(&mut e, dev, false, 0x100, 0, 0);
        let b = new_buffer(&mut e, dev, false, 0x100, 0, 0);
        let i1 = new_buffer(&mut e, dev, true, 0x100, 0, 0);
        let i2 = new_buffer(&mut e, dev, true, 0x100, 0, 0);
        // `GetStreamSource(0)` and `GetIndices` as the calling side sees them (the count they add
        // given back).
        let bound = |e: &mut Engine, slot: u32| {
            let out = e.mem.alloc(12);
            let args: &[u32] = if slot == 101 {
                &[0, out, out + 4, out + 8]
            } else {
                &[out]
            };
            assert_eq!(e.vcall(dev, 4 * slot, args).u32(), D3D_OK);
            let buffer = e.mem.u32(out);
            e.mem.free(out);
            if buffer != 0 {
                e.vcall(buffer, 4 * 2, &[]);
            }
            buffer
        };
        assert_eq!(e.vcall(dev, 4 * 100, &[0, a, 0, 32]).u32(), D3D_OK);
        assert_eq!(e.vcall(dev, 4 * 104, &[i1]).u32(), D3D_OK);
        assert!(draw_cut(&mut e, dev));
        as_drawing_side(|| {
            assert_eq!(bound(&mut e, 101), a, "main's stream at the cut");
            assert_eq!(bound(&mut e, 105), i1, "main's indices at the cut");
            assert_eq!(e.vcall(dev, 4 * 100, &[0, b, 0, 32]).u32(), D3D_OK);
            assert_eq!(bound(&mut e, 101), b);
        });
        assert_eq!(
            bound(&mut e, 101),
            a,
            "main's own, the drawing side's unseen"
        );
        assert_eq!(e.vcall(dev, 4 * 104, &[i2]).u32(), D3D_OK);
        draw_join(&mut e);
        assert_eq!(
            bound(&mut e, 101),
            b,
            "the drawing side's: main set none after the cut"
        );
        assert_eq!(bound(&mut e, 105), i2, "main's: set after the cut");
    }

    /// A buffer whose last reference the drawing side lets go of during a cut ends only at the
    /// join: main's calls meanwhile still find it alive.
    #[test]
    fn a_buffer_the_drawing_side_releases_ends_at_the_join() {
        use super::super::d3d9draw::{as_drawing_side, draw_cut, draw_join};
        use super::super::d3d9resource::alive;
        let (mut e, dev) = texture_device(Store::Host);
        let x = new_buffer(&mut e, dev, false, 0x100, 0, 0);
        assert!(draw_cut(&mut e, dev));
        as_drawing_side(|| assert_eq!(e.vcall(x, 4 * 2, &[]).u32(), 0, "its last reference"));
        assert!(alive(&mut e, x), "kept until the join");
        draw_join(&mut e);
        assert!(!alive(&mut e, x), "ended at the join");
    }

    /// A lock from the drawing side gets a block of its own holding the range's bytes as they are:
    /// its writes do not reach the buffer's own bytes during the cut, which a lock of main's that
    /// reads nothing (NOOVERWRITE, as the game's buffer manager locks without reading back) finds.
    #[test]
    fn the_drawing_side_locks_a_buffer_in_a_block_of_its_own() {
        use super::super::d3d9draw::{as_drawing_side, draw_cut, draw_join};
        let (mut e, dev) = texture_device(Store::Host);
        let x = new_buffer(&mut e, dev, false, 0x100, 0, 0);
        let (hr, main_at) = lock_buffer(&mut e, x, 0, 16, 0);
        assert_eq!(hr, D3D_OK);
        e.mem.set_u32(main_at, 0x1234_5678);
        assert_eq!(e.vcall(x, 4 * 12, &[]).u32(), D3D_OK);
        assert!(draw_cut(&mut e, dev));
        let draw_at = as_drawing_side(|| {
            let (hr, at) = lock_buffer(&mut e, x, 0, 16, 0);
            assert_eq!(hr, D3D_OK);
            assert_eq!(e.mem.u32(at), 0x1234_5678, "the range's bytes as they are");
            e.mem.set_u32(at, 0x9abc_def0);
            assert_eq!(e.vcall(x, 4 * 12, &[]).u32(), D3D_OK);
            at
        });
        let (hr, again) = lock_buffer(&mut e, x, 0, 16, 0x1000);
        assert_eq!(hr, D3D_OK);
        assert_ne!(
            again, draw_at,
            "main's lock never meets the drawing side's block"
        );
        assert_eq!(e.mem.u32(again), 0x1234_5678, "the buffer's own bytes");
        assert_eq!(e.vcall(x, 4 * 12, &[]).u32(), D3D_OK);
        draw_join(&mut e);
    }

    /// The calls of both sides reach the backend in the serial order: main's before the cut, the
    /// drawing side's (through the cut's slot, while main goes on), then main's after the cut, held
    /// until the join, and main's after the join.
    #[cfg(windows)]
    #[test]
    fn the_backend_sees_the_two_sides_calls_in_the_serial_order() {
        use super::super::d3d9draw::{as_drawing_side, draw_cut, draw_join};
        use d3d9_stream::{Backend, Cmd, Data, Reply, Stream};
        struct States(std::sync::Arc<std::sync::Mutex<Vec<String>>>);
        impl Backend for States {
            fn execute(&mut self, cmd: Cmd<Data<'_>>) -> Reply {
                if let Cmd::SetRenderState { state, value, .. } = cmd {
                    self.0.lock().unwrap().push(format!("{state}={value}"));
                }
                Reply::None
            }
        }
        let (mut e, dev) = texture_device(Store::Host);
        let seen = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let backend = States(std::sync::Arc::clone(&seen));
        super::super::d3d9real::test_stream(&mut e, dev, Stream::direct(Box::new(backend)));
        // `SetRenderState` through the device's table: the model, then the backend's stream.
        let state = |e: &mut Engine, s: u32, v: u32| {
            assert_eq!(e.vcall(dev, 4 * 57, &[s, v]).u32(), D3D_OK);
        };
        state(&mut e, 7, 1);
        assert!(draw_cut(&mut e, dev));
        state(&mut e, 8, 3);
        as_drawing_side(|| state(&mut e, 9, 2));
        assert_eq!(
            *seen.lock().unwrap(),
            ["7=1", "9=2"],
            "main's after the cut held"
        );
        draw_join(&mut e);
        state(&mut e, 10, 4);
        assert_eq!(*seen.lock().unwrap(), ["7=1", "9=2", "8=3", "10=4"]);
    }

    /// With a cut open the drawing side leaves out a sampler state or constant range set again to what
    /// ITS copy holds (that copy is the stream's state where its commands land), even where main's
    /// model holds something else; main sends every set until the join, a repeat of its own model
    /// too (the drawing side's commands come between in the stream); after the join repeats of the
    /// serial state are left out again.
    #[cfg(windows)]
    #[test]
    fn the_drawing_side_leaves_out_repeats_of_its_own_copy() {
        use super::super::d3d9draw::{as_drawing_side, draw_cut, draw_join};
        use d3d9_stream::{Backend, Cmd, Data, Reply, Stream};
        struct Seen(std::sync::Arc<std::sync::Mutex<Vec<String>>>);
        impl Backend for Seen {
            fn execute(&mut self, cmd: Cmd<Data<'_>>) -> Reply {
                let seen = match cmd {
                    Cmd::SetSamplerState {
                        sampler,
                        kind,
                        value,
                        ..
                    } => format!("s{sampler}/{kind}={value}"),
                    Cmd::SetConstants { start, count, .. } => format!("c{start}x{count}"),
                    _ => return Reply::None,
                };
                self.0.lock().unwrap().push(seen);
                Reply::None
            }
        }
        let (mut e, dev) = texture_device(Store::Host);
        keep_repeats(&mut e, false);
        let seen = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        super::super::d3d9real::test_stream(
            &mut e,
            dev,
            Stream::direct(Box::new(Seen(std::sync::Arc::clone(&seen)))),
        );
        let sampler = |e: &mut Engine, kind: u32, v: u32| {
            assert_eq!(e.vcall(dev, 4 * 69, &[0, kind, v]).u32(), D3D_OK);
        };
        let data = e.mem.alloc(16);
        for i in 0..4 {
            e.mem.set_u32(data + 4 * i, 0x3f80_0000 + i);
        }
        let constant = |e: &mut Engine| {
            assert_eq!(e.vcall(dev, 4 * 94, &[60, data, 1]).u32(), D3D_OK);
        };
        sampler(&mut e, 1, 7);
        constant(&mut e);
        assert!(draw_cut(&mut e, dev));
        as_drawing_side(|| {
            // Repeats of the state at the cut: left out.
            sampler(&mut e, 1, 7);
            constant(&mut e);
            // A change goes, then its repeat does not.
            sampler(&mut e, 1, 3);
            sampler(&mut e, 1, 3);
        });
        // Main: a repeat of its own model goes (the stream holds the drawing side's 3 there), and so
        // does every set of a state either side never changed.
        sampler(&mut e, 1, 7);
        sampler(&mut e, 2, 5);
        sampler(&mut e, 2, 5);
        constant(&mut e);
        // The drawing side, after main's sets: still against its own copy (3, not main's 7).
        as_drawing_side(|| sampler(&mut e, 1, 3));
        assert_eq!(
            *seen.lock().unwrap(),
            ["s0/1=7", "c60x1", "s0/1=3"],
            "main's after the cut held; the drawing side's repeats left out"
        );
        draw_join(&mut e);
        // After the join the serial state (main's 7 and 5, the constant) is the model: repeats left out.
        sampler(&mut e, 1, 7);
        sampler(&mut e, 2, 5);
        constant(&mut e);
        assert_eq!(
            *seen.lock().unwrap(),
            ["s0/1=7", "c60x1", "s0/1=3", "s0/1=7", "s0/2=5", "s0/2=5", "c60x1"]
        );
    }

    /// With real threads, a sync call of main's while a cut is open (a surface read back, under
    /// the device lock as a device function holds it) waits for the drawing job, which runs on the
    /// drawing helper and needs that lock for its own calls: the lock is given back meanwhile, and
    /// the reply comes after the drawing side's commands and main's held ones, on the direct and
    /// the threaded runner. A job that ends without closing its slot (one that stopped, say) does
    /// not leave main's call stuck or refused: the slot is closed for it. Under a time limit: a
    /// lock kept through the wait would leave both threads waiting for ever.
    #[cfg(windows)]
    #[test]
    fn mains_sync_call_during_a_cut_waits_for_the_drawing_job_with_real_threads() {
        let (done, finished) = std::sync::mpsc::channel();
        let body = std::thread::Builder::new()
            .name("sync-during-cut".into())
            .stack_size(16 << 20)
            .spawn(move || {
                sync_call_during_a_cut();
                let _ = done.send(());
            })
            .expect("the test's thread");
        match finished.recv_timeout(std::time::Duration::from_secs(30)) {
            Ok(()) => body.join().expect("the test's thread"),
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                if let Err(payload) = body.join() {
                    std::panic::resume_unwind(payload);
                }
            }
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                panic!("main's sync call and the drawing job waited for each other (30 s)")
            }
        }
    }

    /// Stepped threads (no device lock): with direct calls on, a call still reaches the device function by its
    /// address and the model takes it.
    #[test]
    fn direct_device_calls_on_stepped_threads_reach_the_device_functions() {
        use engine_core::real_threads::{
            device_vcall, direct_begin, direct_calls, direct_end, direct_holds,
            force_direct_for_tests,
        };
        let (mut e, dev) = texture_device(Store::Host);
        force_direct_for_tests();
        let before = direct_calls();
        assert!(direct_begin(&e));
        assert_eq!(device_vcall(&mut e, dev, 4 * 57, &[7, 5]).u32(), D3D_OK);
        assert_eq!(device_vcall(&mut e, dev, 4 * 57, &[8, 6]).u32(), D3D_OK);
        assert!(!direct_holds(), "no device lock with stepped threads");
        let out = e.mem.alloc(4);
        assert_eq!(device_vcall(&mut e, dev, 4 * 58, &[8, out]).u32(), D3D_OK);
        assert_eq!(e.mem.u32(out), 6);
        assert_eq!(direct_calls() - before, 3, "made directly");
        direct_end();
        assert!(direct_begin(&e), "on again after the end");
        direct_end();
        assert_eq!(render_state(&mut e, dev, 7), 5);
    }

    /// With direct calls on and real threads, under a device lock the caller already holds (this test's device was
    /// made before the threads started, so its functions are behind the kernel lock, as in
    /// `sync_call_during_a_cut`): the same answers, no direct hold taken, nothing left held after.
    #[test]
    fn direct_device_calls_under_a_held_device_lock_answer_the_same() {
        use engine_core::real_threads::{
            device_guard, device_lock_held, device_vcall, direct_begin, direct_end, direct_holds,
            force_direct_for_tests,
        };
        let (mut e, dev) = texture_device(Store::Host);
        engine_core::real_threads::start(&mut e).expect("real threads on");
        force_direct_for_tests();
        assert!(direct_begin(&e));
        assert!(!direct_begin(&e), "one at a time");
        {
            let _device = device_guard(&e);
            assert_eq!(device_vcall(&mut e, dev, 4 * 57, &[8, 2]).u32(), D3D_OK);
            assert_eq!(
                device_vcall(&mut e, dev, 4 * 69, &[16, 1, 1]).u32(),
                D3DERR_INVALIDCALL
            );
            let out = e.mem.alloc(4);
            assert_eq!(device_vcall(&mut e, dev, 4 * 58, &[8, out]).u32(), D3D_OK);
            assert_eq!(e.mem.u32(out), 2);
            assert!(
                !direct_holds(),
                "the caller's own hold: no direct hold taken"
            );
        }
        direct_end();
        assert!(!direct_holds() && !device_lock_held(&e));
    }

    #[cfg(windows)]
    fn sync_call_during_a_cut() {
        use super::super::d3d9draw::{draw_close, draw_cut, draw_join};
        use d3d9_stream::{Backend, Cmd, Data, Reply, Settings, Stream};
        use engine_core::real_threads::{device_guard, join_draw, post_draw};
        use std::sync::atomic::{AtomicBool, Ordering};
        use std::sync::{Arc, Mutex};
        struct Log(Arc<Mutex<Vec<String>>>);
        impl Backend for Log {
            fn execute(&mut self, cmd: Cmd<Data<'_>>) -> Reply {
                match cmd {
                    Cmd::SetRenderState { state, value, .. } => {
                        self.0.lock().unwrap().push(format!("{state}={value}"));
                        Reply::None
                    }
                    Cmd::ReadSurface { size, .. } => {
                        self.0.lock().unwrap().push("read".into());
                        Reply::Bytes(Some(vec![0x5a; size as usize]))
                    }
                    _ => Reply::None,
                }
            }
        }
        const SURFACE: u32 = 0x0bad_5000;
        let (mut e, dev) = texture_device(Store::Host);
        engine_core::real_threads::start(&mut e).expect("real threads on");
        let at = e.mem.alloc(16);
        // `SetRenderState` through the device's table, under the device lock as the game's call
        // takes it (`real_threads::device_call`; this test's table is not in the layer's ranges).
        fn state(e: &mut Engine, dev: u32, s: u32, v: u32) {
            let _device = device_guard(e);
            assert_eq!(e.vcall(dev, 4 * 57, &[s, v]).u32(), D3D_OK);
        }
        for threaded in [false, true] {
            let seen = Arc::new(Mutex::new(Vec::new()));
            let backend = Box::new(Log(Arc::clone(&seen)));
            let stream = if threaded {
                Stream::threaded(backend, Settings::default())
            } else {
                Stream::direct(backend)
            };
            {
                let _device = device_guard(&e);
                super::super::d3d9real::test_stream(&mut e, dev, stream);
                super::super::d3d9real::test_forwarded(&mut e, SURFACE);
            }
            // Twice: a job that closes its slot, and one that ends without closing it.
            for closes in [true, false] {
                seen.lock().unwrap().clear();
                state(&mut e, dev, 7, 1);
                assert!(draw_cut(&mut e, dev));
                let go = Arc::new(AtomicBool::new(false));
                let job_go = Arc::clone(&go);
                assert!(post_draw(
                    &mut e,
                    Box::new(move |w: &mut Engine| {
                        // Only once main is about to make its sync call, and late enough that a
                        // reply not waiting for the job would come first.
                        while !job_go.load(Ordering::SeqCst) {
                            std::thread::yield_now();
                        }
                        std::thread::sleep(std::time::Duration::from_millis(30));
                        state(w, dev, 9, 2);
                        if closes {
                            draw_close(w);
                        }
                    })
                ));
                state(&mut e, dev, 8, 3);
                let read = {
                    let _device = device_guard(&e);
                    go.store(true, Ordering::SeqCst);
                    let read = super::super::d3d9real::read_surface(&mut e, SURFACE, at, 16);
                    assert!(crate::device_held(&e), "the device lock is taken again");
                    read
                };
                let runner = if threaded { "threaded" } else { "direct" };
                let how = if closes { "closed" } else { "left open" };
                assert!(read, "{runner}, slot {how}: the surface read back");
                assert_eq!(e.mem.u32(at), 0x5a5a_5a5a, "{runner}, slot {how}");
                assert_eq!(
                    *seen.lock().unwrap(),
                    ["7=1", "9=2", "8=3", "read"],
                    "{runner}, slot {how}: the drawing side's commands, main's held ones, then \
                     the read"
                );
                assert!(join_draw(&e));
                draw_join(&mut e);
                let _device = device_guard(&e);
                assert_eq!(render_state(&mut e, dev, 9), 2, "the serial order's state");
                assert_eq!(render_state(&mut e, dev, 8), 3);
            }
        }
        draw_finish_and_the_end_of_the_process(&mut e, dev);
    }

    /// The rest of the real-threads test (the drawing helper is one per process, so one test
    /// posts): `draw_finish` joins a job and the cut in one call; at the end of the process a slow
    /// job is waited for, one that never closes its slot is joined, and one that cannot end in
    /// time is left behind after the limit, so the end never hangs.
    #[cfg(windows)]
    fn draw_finish_and_the_end_of_the_process(e: &mut Engine, dev: u32) {
        use super::super::d3d9draw::{draw_close, draw_cut, draw_finish, draw_join, draw_open};
        use engine_core::real_threads::{draw_posted, join_draw, post_draw, DrawAtExit};
        use std::sync::atomic::{AtomicBool, Ordering};
        use std::sync::Arc;
        use std::time::{Duration, Instant};
        fn state(e: &mut Engine, dev: u32, s: u32, v: u32) {
            let _device = engine_core::real_threads::device_guard(e);
            assert_eq!(e.vcall(dev, 4 * 57, &[s, v]).u32(), D3D_OK);
        }
        let read_state = |e: &mut Engine, s: u32| {
            let _device = engine_core::real_threads::device_guard(e);
            render_state(e, dev, s)
        };
        // draw_finish: the job's change is main's after one call, the cut closed.
        assert!(draw_cut(e, dev));
        assert!(post_draw(
            e,
            Box::new(move |w: &mut Engine| {
                std::thread::sleep(Duration::from_millis(20));
                state(w, dev, 11, 6);
                draw_close(w);
            })
        ));
        draw_finish(e);
        assert!(!draw_posted() && !draw_open(e), "joined, cut closed");
        assert_eq!(read_state(e, 11), 6);
        // A job posted and taken by the helper (one not taken yet is dropped unrun at the end).
        fn post_running(e: &mut Engine, job: impl FnOnce(&mut Engine) + Send + 'static) {
            let running = Arc::new(AtomicBool::new(false));
            let flag = Arc::clone(&running);
            assert!(post_draw(
                e,
                Box::new(move |w: &mut Engine| {
                    flag.store(true, Ordering::SeqCst);
                    job(w)
                })
            ));
            while !running.load(Ordering::SeqCst) {
                std::thread::yield_now();
            }
        }
        // The end of the process, a slow job: waited for, joined.
        assert!(draw_cut(e, dev));
        post_running(e, move |w: &mut Engine| {
            std::thread::sleep(Duration::from_millis(150));
            state(w, dev, 12, 7);
            draw_close(w);
        });
        let started = Instant::now();
        assert_eq!(
            crate::finish_drawing_at_exit(Duration::from_secs(5)),
            DrawAtExit::Joined
        );
        assert!(
            started.elapsed() >= Duration::from_millis(100),
            "it waited for the job"
        );
        assert!(!draw_posted());
        assert_eq!(
            crate::finish_drawing_at_exit(Duration::from_secs(5)),
            DrawAtExit::None,
            "a second call finds nothing"
        );
        draw_join(e);
        // A job that never closes its slot: joined all the same.
        assert!(draw_cut(e, dev));
        post_running(e, move |w: &mut Engine| state(w, dev, 13, 8));
        assert_eq!(
            crate::finish_drawing_at_exit(Duration::from_secs(5)),
            DrawAtExit::Joined
        );
        draw_join(e);
        assert!(!draw_open(e));
        // A job that cannot end in time: left after the limit, no hang.
        let release = Arc::new(AtomicBool::new(false));
        let job_release = Arc::clone(&release);
        post_running(e, move |_| {
            while !job_release.load(Ordering::SeqCst) {
                std::thread::sleep(Duration::from_millis(1));
            }
        });
        let started = Instant::now();
        assert_eq!(
            crate::finish_drawing_at_exit(Duration::from_millis(100)),
            DrawAtExit::StillRunning
        );
        assert!(
            started.elapsed() < Duration::from_secs(2),
            "the limit holds"
        );
        release.store(true, Ordering::SeqCst);
        assert!(join_draw(e), "still posted, joined once it ends");
    }

    /// A device with mixed vertex processing (so `SetSoftwareVertexProcessing` takes both
    /// values) on the texture table: three vertex buffers, index buffers, declarations, vertex
    /// and pixel shaders and textures to bind (TEST DATA).
    struct Bound {
        vbs: [u32; 3],
        ibs: [u32; 3],
        decls: [u32; 3],
        vss: [u32; 3],
        pss: [u32; 3],
        textures: [u32; 3],
    }

    fn mixed_device() -> (Engine, u32, Bound) {
        let (mut e, d3d, present) = d3d();
        set_texture_store(&mut e, Store::Host);
        let out = e.mem.alloc(4);
        let mixed = D3DCREATE_MIXED_VERTEXPROCESSING;
        assert_eq!(
            e.vcall(d3d, CREATE_DEVICE, &[0, 1, 0x11c, mixed, present, out])
                .u32(),
            D3D_OK
        );
        let dev = e.mem.u32(out);
        // The test table's caps with six user clip planes (`MaxUserClipPlanes`, +0xA4).
        let hex = |b: &[u8]| -> String { b.iter().map(|x| format!("{x:02x}")).collect() };
        let mut caps = vec![0u8; 304];
        caps[..4].copy_from_slice(&1u32.to_le_bytes());
        caps[CAPS_SIMULTANEOUS_RTS..CAPS_SIMULTANEOUS_RTS + 4].copy_from_slice(&4u32.to_le_bytes());
        let plain = hex(&caps);
        caps[0xa4..0xa8].copy_from_slice(&6u32.to_le_bytes());
        let table = test_table().replace(&plain, &hex(&caps))
            + "\nDomain\td1\t15,31545844"
            + "\nCheckDeviceFormat\t0 1 16 0 3\t0\t\td1";
        assert!(table.contains(&hex(&caps)), "the caps replaced");
        d3d9::load_adapter_table(&mut e, &table);
        let made = |e: &mut Engine, slot: u32, args: &[u32]| {
            let mut a = args.to_vec();
            a.push(out);
            if matches!(slot, 23 | 26 | 27) {
                // The shared handle.
                a.push(0);
            }
            assert_eq!(e.vcall(dev, 4 * slot, &a).u32(), D3D_OK, "slot {slot}");
            e.mem.u32(out)
        };
        let tokens = |e: &mut Engine, version: u32, v: u32| {
            let f = e.mem.alloc(16);
            for (i, t) in [version, 0x1000 + v, 0x0000_ffff].iter().enumerate() {
                e.mem.set_u32(f + 4 * i as u32, *t);
            }
            f
        };
        let element = |e: &mut Engine, v: u32| {
            let el = e.mem.alloc(16);
            e.mem.set_u16(el + 4, v as u16);
            e.mem.set_u16(el + 8, 0xff);
            e.mem.set_u8(el + 12, 17);
            el
        };
        let mut bound = Bound {
            vbs: [0; 3],
            ibs: [0; 3],
            decls: [0; 3],
            vss: [0; 3],
            pss: [0; 3],
            textures: [0; 3],
        };
        for i in 0..3 {
            bound.vbs[i] = made(&mut e, 26, &[64, 0, 0x142, 1]);
            bound.ibs[i] = made(&mut e, 27, &[64, 0, 101, 1]);
            let el = element(&mut e, i as u32);
            bound.decls[i] = made(&mut e, 86, &[el]);
            let f = tokens(&mut e, 0xfffe_0300, i as u32);
            bound.vss[i] = made(&mut e, 91, &[f]);
            let f = tokens(&mut e, 0xffff_0300, i as u32);
            bound.pss[i] = made(&mut e, 106, &[f]);
            bound.textures[i] = made(&mut e, 23, &[4, 4, 1, 0, A8R8G8B8, 1]);
        }
        (e, dev, bound)
    }

    /// The words a getter writes: `slot` called with `args` and then `out` (where the getter
    /// takes its out pointer last), `n` words read back.
    fn got(e: &mut Engine, dev: u32, slot: u32, args: &[u32], n: u32) -> Vec<u32> {
        let out = e.mem.alloc(4 * n);
        let mut a = args.to_vec();
        a.push(out);
        assert_eq!(
            e.vcall(dev, 4 * slot, &a).u32(),
            D3D_OK,
            "getter slot {slot}"
        );
        let words = (0..n).map(|i| e.mem.u32(out + 4 * i)).collect();
        e.mem.free(out);
        words
    }

    /// `slot` called with `words` written to memory and handed by address after `args`.
    fn with_words(e: &mut Engine, dev: u32, slot: u32, args: &[u32], words: &[u32]) -> u32 {
        let at = e.mem.alloc(4 * words.len().max(1) as u32);
        for (i, w) in words.iter().enumerate() {
            e.mem.set_u32(at + 4 * i as u32, *w);
        }
        let mut a = args.to_vec();
        a.push(at);
        let hr = e.vcall(dev, 4 * slot, &a).u32();
        e.mem.free(at);
        hr
    }

    type Setter = fn(&mut Engine, u32, &Bound, u32);
    type Getter = fn(&mut Engine, u32, &Bound) -> Vec<u32>;

    /// Every piece of the device's state the join merges, set through the device's own calls
    /// (so each setter's note of what main set is tested too) and read from the model as the
    /// calling side sees it: for each, the value set before the cut (A), the drawing side's
    /// change (B) and main's change after the cut (C). Main keeps A during the cut; with no change
    /// of main's the join gives B (the drawing side's, the serial order's); with main's change it
    /// gives C. Where a piece can be absent before the cut (a transform, a clip plane, a texture,
    /// a stream, a constant never set), a pass without A checks the drawing side's value
    /// arriving where main had none.
    #[test]
    fn every_kind_of_state_merges_at_the_join_as_the_serial_order() {
        use super::super::d3d9draw::{as_drawing_side, draw_cut, draw_join};
        use super::super::d3d9resource::binding;
        fn float_words(v: u32, n: usize) -> Vec<u32> {
            (0..n)
                .map(|i| (v as f32 + i as f32 / 8.0).to_bits())
                .collect()
        }
        fn ok(hr: u32) {
            assert_eq!(hr, D3D_OK);
        }
        fn model(e: &mut Engine, d: u32) -> &mut Device {
            device(e, Ptr::new(d))
        }
        fn constant(e: &mut Engine, d: u32, slot: u32, start: u32, words: &[u32]) {
            let at = e.mem.alloc(16);
            for (i, w) in words.iter().enumerate() {
                e.mem.set_u32(at + 4 * i as u32, *w);
            }
            ok(e.vcall(d, 4 * slot, &[start, at, 1]).u32());
            e.mem.free(at);
        }
        fn read(e: &mut Engine, f: impl FnOnce(&mut Engine, Ptr) -> u32) -> Vec<u32> {
            let out = e.mem.alloc(4);
            ok(f(e, Ptr::new(out)));
            let v = e.mem.u32(out);
            e.mem.free(out);
            vec![v]
        }
        let cases: Vec<(&str, bool, Setter, Getter)> = vec![
            (
                "render state",
                false,
                |e, d, _, v| ok(e.vcall(d, 4 * 57, &[7, v]).u32()),
                |e, d, _| vec![model(e, d).render_states.0[7].unwrap_or(u32::MAX)],
            ),
            (
                "stage state",
                false,
                |e, d, _, v| ok(e.vcall(d, 4 * 67, &[1, 1, v + 1]).u32()),
                |e, d, _| read(e, |e, o| get_texture_stage_state(e, Ptr::new(d), 1, 1, o)),
            ),
            (
                "sampler state",
                false,
                |e, d, _, v| ok(e.vcall(d, 4 * 69, &[2, 5, v]).u32()),
                |e, d, _| read(e, |e, o| get_sampler_state(e, Ptr::new(d), 2, 5, o)),
            ),
            (
                "transform",
                true,
                |e, d, _, v| ok(with_words(e, d, 44, &[17], &float_words(v, 16))),
                |e, d, _| {
                    model(e, d)
                        .transforms
                        .get(&17)
                        .map_or(Vec::new(), |m| m.to_vec())
                },
            ),
            (
                "clip plane",
                true,
                |e, d, _, v| ok(with_words(e, d, 55, &[1], &float_words(v, 4))),
                |e, d, _| {
                    model(e, d)
                        .clip_planes
                        .get(&1)
                        .map_or(Vec::new(), |p| p.to_vec())
                },
            ),
            (
                "light",
                true,
                |e, d, _, v| {
                    let mut l = float_words(v, 26);
                    l[0] = 1 + v % 3;
                    ok(with_words(e, d, 51, &[2], &l))
                },
                |e, d, _| {
                    model(e, d)
                        .lights
                        .get(&2)
                        .map_or(Vec::new(), |l| l.to_vec())
                },
            ),
            (
                "light enable",
                false,
                |e, d, _, v| ok(e.vcall(d, 4 * 53, &[3, v % 2]).u32()),
                |e, d, _| vec![u32::from(model(e, d).enabled_lights.contains(&3))],
            ),
            (
                "material",
                false,
                |e, d, _, v| ok(with_words(e, d, 49, &[], &float_words(v, 17))),
                |e, d, _| model(e, d).material.map_or(Vec::new(), |m| m.to_vec()),
            ),
            (
                "viewport",
                false,
                |e, d, _, v| set_viewport_words(e, d, [0, 0, 100 * v, 50 * v]),
                |e, d, _| model(e, d).viewport.to_vec(),
            ),
            (
                "scissor",
                false,
                |e, d, _, v| ok(with_words(e, d, 75, &[], &[0, 0, 10 * v, 10 * v])),
                |e, d, _| model(e, d).scissor.to_vec(),
            ),
            (
                "gamma ramp",
                false,
                |e, d, _, v| {
                    let ramp: Vec<u32> = (0..384).map(|i| i * v).collect();
                    with_words(e, d, 21, &[0, 0], &ramp);
                },
                |e, d, _| {
                    model(e, d)
                        .gamma_ramp
                        .as_ref()
                        .map_or(Vec::new(), |r| r.iter().map(|&w| u32::from(w)).collect())
                },
            ),
            (
                "software vertex processing",
                false,
                |e, d, _, v| ok(e.vcall(d, 4 * 77, &[v % 2]).u32()),
                |e, d, _| vec![u32::from(model(e, d).software_vertex_processing)],
            ),
            (
                // A: in a scene; B: the drawing side ends it; C: main ends it and begins another.
                "in scene",
                false,
                |e, d, _, v| {
                    if v != 1 {
                        ok(e.vcall(d, 4 * 42, &[]).u32());
                    }
                    if v != 2 {
                        ok(e.vcall(d, 4 * 41, &[]).u32());
                    }
                },
                |e, d, _| vec![u32::from(model(e, d).in_scene)],
            ),
            (
                "texture",
                true,
                |e, d, b, v| ok(e.vcall(d, 4 * 65, &[3, b.textures[v as usize - 1]]).u32()),
                |e, d, _| vec![texture_at(e, d, 3)],
            ),
            (
                "stream",
                true,
                |e, d, b, v| {
                    ok(e.vcall(d, 4 * 100, &[1, b.vbs[v as usize - 1], 4 * v, 16])
                        .u32())
                },
                |e, d, _| binding(e, d, "stream 1"),
            ),
            (
                "indices",
                false,
                |e, d, b, v| ok(e.vcall(d, 4 * 104, &[b.ibs[v as usize - 1]]).u32()),
                |e, d, _| binding(e, d, "indices"),
            ),
            (
                "declaration",
                false,
                |e, d, b, v| ok(e.vcall(d, 4 * 87, &[b.decls[v as usize - 1]]).u32()),
                |e, d, _| binding(e, d, "declaration"),
            ),
            (
                "FVF",
                false,
                |e, d, _, v| {
                    ok(e.vcall(d, 4 * 89, &[[0x2, 0x12, 0x42][v as usize - 1]])
                        .u32())
                },
                |e, d, _| binding(e, d, "fvf"),
            ),
            (
                "vertex shader",
                false,
                |e, d, b, v| ok(e.vcall(d, 4 * 92, &[b.vss[v as usize - 1]]).u32()),
                |e, d, _| binding(e, d, "vertex shader"),
            ),
            (
                "pixel shader",
                false,
                |e, d, b, v| ok(e.vcall(d, 4 * 107, &[b.pss[v as usize - 1]]).u32()),
                |e, d, _| binding(e, d, "pixel shader"),
            ),
            (
                "vertex float constant",
                true,
                |e, d, _, v| constant(e, d, 94, 10, &float_words(v, 4)),
                |e, d, _| binding(e, d, "constant 0 10"),
            ),
            (
                "vertex integer constant",
                true,
                |e, d, _, v| constant(e, d, 96, 2, &[v, v + 1, v + 2, v + 3]),
                |e, d, _| binding(e, d, "constant 1 2"),
            ),
            (
                "pixel float constant",
                true,
                |e, d, _, v| constant(e, d, 109, 5, &float_words(v, 4)),
                |e, d, _| binding(e, d, "constant 3 5"),
            ),
        ];
        for (name, can_be_absent, set, get) in cases {
            for (before, main_after) in [(true, false), (true, true), (false, false), (false, true)]
            {
                if !before && !can_be_absent {
                    continue;
                }
                let (mut e, dev, b) = mixed_device();
                let what = format!("{name}, set before: {before}, main after: {main_after}");
                if before {
                    set(&mut e, dev, &b, 1);
                }
                let at_cut = get(&mut e, dev, &b);
                assert!(draw_cut(&mut e, dev), "{what}");
                let drawn = as_drawing_side(|| {
                    assert_eq!(get(&mut e, dev, &b), at_cut, "{what}: main's at the cut");
                    set(&mut e, dev, &b, 2);
                    get(&mut e, dev, &b)
                });
                assert_ne!(drawn, at_cut, "{what}: the drawing side's value differs");
                assert_eq!(get(&mut e, dev, &b), at_cut, "{what}: main keeps its own");
                let want = if main_after {
                    set(&mut e, dev, &b, 3);
                    let mine = get(&mut e, dev, &b);
                    assert_ne!(mine, drawn, "{what}: main's value differs");
                    mine
                } else {
                    drawn
                };
                draw_join(&mut e);
                assert_eq!(get(&mut e, dev, &b), want, "{what}: after the join");
            }
        }
    }

    /// The references of `object` (an `AddRef` and its `Release`).
    fn references(e: &mut Engine, object: u32) -> u32 {
        e.vcall(object, 4, &[]);
        e.vcall(object, 4 * 2, &[]).u32()
    }

    /// Render targets and the depth surface at the join, with their references: the copy holds
    /// one of each target it binds; at the join the target not kept lets go of it, so a target
    /// the program released while bound goes exactly when nothing binds it, in both branches
    /// (the drawing side's kept where main set none, main's kept where it set one).
    #[test]
    fn render_targets_and_depth_merge_with_their_references() {
        use super::super::d3d9draw::{as_drawing_side, draw_cut, draw_join};
        let (mut e, d3d, present) = d3d();
        let out = e.mem.alloc(4);
        e.vcall(d3d, CREATE_DEVICE, &[0, 1, 0x11c, 0x40, present, out]);
        let dev = e.mem.u32(out);
        // TEST DATA: A8R8G8B8 render targets (no multisampling) and D24S8 depth surfaces.
        let table = test_table()
            + "\nDomain\td1\t15,16\nCheckDeviceFormat\t0 1 16 1 1\t8876086a\t15=0\td1"
            + "\nDomain\td2\t0,1,2,3,4\nCheckDeviceMultiSampleType\t0 1 15 1\t8876086a/0\t0=0/1,4=0/2\td2"
            + "\nDomain\td3\t4b,50\nCheckDeviceFormat\t0 1 16 2 1\t0\t50=8876086a\td3";
        d3d9::load_adapter_table(&mut e, &table);
        let make = |e: &mut Engine, slot: u32, format: u32| {
            assert_eq!(
                e.vcall(dev, 4 * slot, &[640, 480, format, 0, 0, 0, out, 0])
                    .u32(),
                D3D_OK
            );
            e.mem.u32(out)
        };
        let [t1, t2, t3, t4] = [0; 4].map(|_| make(&mut e, 28, A8R8G8B8));
        let [z1, z2] = [0; 2].map(|_| make(&mut e, 29, 0x4b));
        let surface_alive =
            |e: &mut Engine, s: u32| device!(e).device_objects.surfaces.contains_key(&s);
        // Pass 1: the drawing side switches target 1 and the depth away from main's; main sets
        // none. Main's (t1, z1), released by the program while bound, go at the join.
        assert_eq!(e.vcall(dev, 4 * 37, &[1, t1]).u32(), D3D_OK);
        assert_eq!(e.vcall(dev, 4 * 39, &[z1]).u32(), D3D_OK);
        assert_eq!(
            references(&mut e, t1),
            2,
            "the program's and main's binding"
        );
        assert!(draw_cut(&mut e, dev));
        assert_eq!(references(&mut e, t1), 3, "and the copy's");
        assert_eq!(references(&mut e, z1), 3);
        e.vcall(t1, 4 * 2, &[]);
        e.vcall(z1, 4 * 2, &[]);
        as_drawing_side(|| {
            assert_eq!(e.vcall(dev, 4 * 37, &[1, t2]).u32(), D3D_OK);
            assert_eq!(e.vcall(dev, 4 * 39, &[z2]).u32(), D3D_OK);
            assert_eq!(got(&mut e, dev, 38, &[1], 1)[0], t2);
            e.vcall(t2, 4 * 2, &[]);
            assert_eq!(got(&mut e, dev, 40, &[], 1)[0], z2);
            e.vcall(z2, 4 * 2, &[]);
        });
        assert_eq!(got(&mut e, dev, 38, &[1], 1)[0], t1, "main's own");
        e.vcall(t1, 4 * 2, &[]);
        draw_join(&mut e);
        assert_eq!(got(&mut e, dev, 38, &[1], 1)[0], t2, "the drawing side's");
        e.vcall(t2, 4 * 2, &[]);
        assert_eq!(got(&mut e, dev, 40, &[], 1)[0], z2);
        e.vcall(z2, 4 * 2, &[]);
        assert!(
            !surface_alive(&mut e, t1),
            "main's target goes: nothing holds it"
        );
        assert!(!surface_alive(&mut e, z1), "main's depth goes");
        assert_eq!(
            references(&mut e, t2),
            2,
            "the program's and main's binding, once"
        );
        assert_eq!(references(&mut e, z2), 2);
        // Pass 2: both switch target 1; main's (t3) stands, the drawing side's (t4), released by
        // the program, goes at the join; main unbinding the depth stands too.
        assert!(draw_cut(&mut e, dev));
        as_drawing_side(|| {
            assert_eq!(e.vcall(dev, 4 * 37, &[1, t4]).u32(), D3D_OK);
        });
        e.vcall(t4, 4 * 2, &[]);
        assert!(surface_alive(&mut e, t4), "the copy holds it");
        assert_eq!(e.vcall(dev, 4 * 37, &[1, t3]).u32(), D3D_OK);
        assert_eq!(e.vcall(dev, 4 * 39, &[0]).u32(), D3D_OK);
        draw_join(&mut e);
        assert_eq!(
            got(&mut e, dev, 38, &[1], 1)[0],
            t3,
            "main's: set after the cut"
        );
        e.vcall(t3, 4 * 2, &[]);
        assert!(!surface_alive(&mut e, t4), "the drawing side's goes");
        assert!(surface_alive(&mut e, t2), "the program's still");
        assert_eq!(
            references(&mut e, t2),
            1,
            "nothing binds it: the program's alone"
        );
        assert_eq!(
            references(&mut e, t3),
            2,
            "the program's and main's binding"
        );
        assert!(surface_alive(&mut e, z2), "still the program's");
        assert_eq!(
            references(&mut e, z2),
            1,
            "main unbound it: the program's alone"
        );
    }

    /// Constants where both sides set overlapping registers: each register is main's where main
    /// set it after the cut, the drawing side's where only it did, and the value before the cut
    /// where neither did.
    #[test]
    fn overlapping_constants_merge_register_by_register() {
        use super::super::d3d9draw::{as_drawing_side, draw_cut, draw_join};
        let (mut e, dev, _) = mixed_device();
        let set = |e: &mut Engine, start: u32, count: u32, v: u32| {
            let at = e.mem.alloc(16 * count);
            for i in 0..4 * count {
                e.mem.set_u32(at + 4 * i, v);
            }
            assert_eq!(e.vcall(dev, 4 * 94, &[start, at, count]).u32(), D3D_OK);
        };
        let get = |e: &mut Engine| {
            let at = e.mem.alloc(16 * 8);
            assert_eq!(e.vcall(dev, 4 * 95, &[0, at, 8]).u32(), D3D_OK);
            (0..8).map(|r| e.mem.u32(at + 16 * r)).collect::<Vec<_>>()
        };
        set(&mut e, 0, 8, 1);
        assert!(draw_cut(&mut e, dev));
        as_drawing_side(|| set(&mut e, 2, 5, 2));
        set(&mut e, 4, 2, 3);
        draw_join(&mut e);
        assert_eq!(get(&mut e), [1, 1, 2, 2, 3, 3, 2, 1]);
    }

    /// Main enabling a light only the drawing side set during the cut: the drawing side's light
    /// stands (main's `LightEnable` made a default only in its own copy, and came after the
    /// drawing side's `SetLight` in the serial order); one neither side set gets the default.
    #[test]
    fn mains_light_enable_keeps_the_drawing_sides_light() {
        use super::super::d3d9draw::{as_drawing_side, draw_cut, draw_join};
        let (mut e, dev, _) = mixed_device();
        let light: Vec<u32> = (0..26)
            .map(|i| if i == 0 { 1 } else { 0x4000_0000 + i })
            .collect();
        assert!(draw_cut(&mut e, dev));
        as_drawing_side(|| assert_eq!(with_words(&mut e, dev, 51, &[4], &light), D3D_OK));
        assert_eq!(e.vcall(dev, 4 * 53, &[4, 1]).u32(), D3D_OK);
        assert_eq!(e.vcall(dev, 4 * 53, &[6, 1]).u32(), D3D_OK);
        draw_join(&mut e);
        let d = device(&mut e, Ptr::new(dev));
        assert_eq!(
            d.lights.get(&4).map(|l| l.to_vec()),
            Some(light),
            "the drawing side's light"
        );
        assert!(d.enabled_lights.contains(&4), "main's enable");
        assert_eq!(
            d.lights.get(&6),
            Some(&default_light()),
            "the default main's LightEnable made"
        );
        assert!(d.enabled_lights.contains(&6));
    }

    /// A draw from memory on the drawing side (stream 0 left empty with the stride, the device
    /// marked as having drawn from memory) reaches main at the join; one of main's after the cut
    /// keeps main's stream 0.
    #[test]
    fn a_draw_from_memory_on_the_drawing_side_merges_at_the_join() {
        use super::super::d3d9draw::{as_drawing_side, draw_cut, draw_join};
        let (mut e, dev, b) = mixed_device();
        let stream0 = |e: &mut Engine| {
            let out = e.mem.alloc(12);
            assert_eq!(
                e.vcall(dev, 4 * 101, &[0, out, out + 4, out + 8]).u32(),
                D3D_OK
            );
            let w = [e.mem.u32(out), e.mem.u32(out + 8)];
            if w[0] != 0 {
                e.vcall(w[0], 4 * 2, &[]);
            }
            w
        };
        assert_eq!(e.vcall(dev, 4 * 89, &[0x2]).u32(), D3D_OK);
        assert_eq!(e.vcall(dev, 4 * 100, &[0, b.vbs[0], 0, 12]).u32(), D3D_OK);
        assert!(draw_cut(&mut e, dev));
        as_drawing_side(|| {
            // Outside a scene before any draw from memory: refused, stream 0 empty, the stride kept.
            assert_eq!(
                e.vcall(dev, 4 * 83, &[4, 1, 0x1000, 24]).u32(),
                D3DERR_INVALIDCALL
            );
            assert_eq!(stream0(&mut e), [0, 24]);
            assert_eq!(e.vcall(dev, 4 * 41, &[]).u32(), D3D_OK);
            assert_eq!(e.vcall(dev, 4 * 83, &[4, 1, 0x1000, 24]).u32(), D3D_OK);
            assert_eq!(e.vcall(dev, 4 * 42, &[]).u32(), D3D_OK);
        });
        assert_eq!(stream0(&mut e), [b.vbs[0], 12], "main's own");
        draw_join(&mut e);
        assert_eq!(stream0(&mut e), [0, 0], "the drawing side's stream 0");
        // Drawn from memory: outside a scene it is taken now.
        assert_eq!(e.vcall(dev, 4 * 83, &[4, 1, 0x1000, 24]).u32(), D3D_OK);
        // Main's stream 0 set after the cut stands.
        assert!(draw_cut(&mut e, dev));
        as_drawing_side(|| {
            assert_eq!(e.vcall(dev, 4 * 83, &[4, 1, 0x1000, 20]).u32(), D3D_OK);
        });
        assert_eq!(e.vcall(dev, 4 * 100, &[0, b.vbs[1], 0, 16]).u32(), D3D_OK);
        draw_join(&mut e);
        assert_eq!(stream0(&mut e), [b.vbs[1], 16]);
    }

    /// The drawing side's writes into a buffer, for static (kept outside the game's memory) and
    /// dynamic buffers: a later lock of its own in the same cut starts from them (a partial write
    /// keeps the earlier bytes, not the buffer's old ones); at the join they become the buffer's,
    /// except where main locked it after the cut (main's bytes come later in the serial order).
    #[test]
    fn the_drawing_sides_buffer_writes_reach_its_later_locks_and_the_join() {
        use super::super::d3d9draw::{as_drawing_side, draw_cut, draw_join};
        for dynamic in [false, true] {
            let (mut e, dev) = texture_device(Store::Host);
            let (usage, pool) = if dynamic { (0x200, 0) } else { (0, 1) };
            let x = new_buffer(&mut e, dev, false, 0x100, usage, pool);
            let word = |e: &mut Engine, at: u32| {
                let (hr, p) = lock_buffer(e, x, at, 4, 0);
                assert_eq!(hr, D3D_OK);
                let w = e.mem.u32(p);
                assert_eq!(e.vcall(x, 4 * 12, &[]).u32(), D3D_OK);
                w
            };
            let put = |e: &mut Engine, at: u32, w: u32| {
                let (hr, p) = lock_buffer(e, x, at, 4, 0);
                assert_eq!(hr, D3D_OK);
                e.mem.set_u32(p, w);
                assert_eq!(e.vcall(x, 4 * 12, &[]).u32(), D3D_OK);
            };
            for at in [0, 0x10, 0x20] {
                put(&mut e, at, 0x1111_1111);
            }
            assert!(draw_cut(&mut e, dev));
            as_drawing_side(|| {
                // A range lock, then a whole-buffer lock that writes only elsewhere.
                put(&mut e, 0x10, 0x2222_2222);
                put(&mut e, 0x20, 0x2222_2222);
                let (hr, p) = lock_buffer(&mut e, x, 0, 0, 0);
                assert_eq!(hr, D3D_OK);
                assert_eq!(e.mem.u32(p + 0x10), 0x2222_2222, "its own earlier write");
                assert_eq!(e.mem.u32(p), 0x1111_1111, "the buffer's bytes elsewhere");
                e.mem.set_u32(p + 0x30, 0x3333_3333);
                assert_eq!(e.vcall(x, 4 * 12, &[]).u32(), D3D_OK);
                assert_eq!(word(&mut e, 0x30), 0x3333_3333);
                assert_eq!(
                    word(&mut e, 0x10),
                    0x2222_2222,
                    "kept through the whole lock"
                );
            });
            // (A lock of main's counts as main's write: main reads where the drawing side did not.)
            assert_eq!(word(&mut e, 0), 0x1111_1111, "main: the buffer's own bytes");
            // Main writes 0x20 after the cut: its bytes come later.
            put(&mut e, 0x20, 0x4444_4444);
            draw_join(&mut e);
            let kind = if dynamic { "dynamic" } else { "static" };
            assert_eq!(
                word(&mut e, 0x10),
                0x2222_2222,
                "{kind}: the drawing side's"
            );
            assert_eq!(
                word(&mut e, 0x30),
                0x3333_3333,
                "{kind}: the drawing side's"
            );
            assert_eq!(
                word(&mut e, 0x20),
                0x4444_4444,
                "{kind}: main's, locked later"
            );
            assert_eq!(word(&mut e, 0), 0x1111_1111, "{kind}: untouched");
            // The next cut's drawing side starts from the joined bytes.
            assert!(draw_cut(&mut e, dev));
            as_drawing_side(|| assert_eq!(word(&mut e, 0x30), 0x3333_3333));
            draw_join(&mut e);
        }
    }

    /// A `Present` of main's during the cut leaves the frame's unlocked staging blocks (the
    /// drawing side may still read them) to the join, which frees them.
    #[test]
    fn a_present_during_a_cut_leaves_the_staging_blocks_to_the_join() {
        use super::super::d3d9draw::{draw_cut, draw_join};
        let (mut e, dev) = texture_device(Store::Host);
        let x = new_buffer(&mut e, dev, false, 0x100, 0, 1);
        let (hr, _) = lock_buffer(&mut e, x, 0, 16, 0);
        assert_eq!(hr, D3D_OK);
        assert_eq!(e.vcall(x, 4 * 12, &[]).u32(), D3D_OK);
        let waiting = |e: &mut Engine| device!(e).device_objects.unlocked_staging.len();
        assert_eq!(
            waiting(&mut e),
            1,
            "the static buffer's block waits for a Present"
        );
        assert!(draw_cut(&mut e, dev));
        assert_eq!(present(&mut e, dev), D3D_OK);
        assert_eq!(waiting(&mut e), 1, "kept through main's Present");
        draw_join(&mut e);
        assert_eq!(waiting(&mut e), 0, "freed at the join");
        // Without a Present in the cut, the join frees nothing.
        let (hr, _) = lock_buffer(&mut e, x, 0, 16, 0);
        assert_eq!(hr, D3D_OK);
        assert_eq!(e.vcall(x, 4 * 12, &[]).u32(), D3D_OK);
        assert!(draw_cut(&mut e, dev));
        draw_join(&mut e);
        assert_eq!(waiting(&mut e), 1);
    }

    /// A texture whose last reference the drawing side lets go of (bound nowhere) ends at the
    /// join, not during the cut, where main may still use it.
    #[test]
    fn a_texture_the_drawing_side_releases_ends_at_the_join() {
        use super::super::d3d9draw::{as_drawing_side, draw_cut, draw_join};
        let (mut e, dev) = texture_device(Store::Host);
        let t = new_texture(&mut e, dev, [16, 16, 1, 0, A8R8G8B8, 1]);
        assert!(draw_cut(&mut e, dev));
        as_drawing_side(|| assert_eq!(e.vcall(t, 4 * 2, &[]).u32(), 0, "its last reference"));
        assert!(
            device!(e).device_objects.textures.contains_key(&t),
            "kept until the join"
        );
        draw_join(&mut e);
        assert!(
            !device!(e).device_objects.textures.contains_key(&t),
            "ended at the join"
        );
    }

    /// A Reset while a cut is open joins it first (piece 8 waits before a reset; this covers a
    /// reset it missed, in release builds too): the copy's holds of the back buffer go before the
    /// reset frees it, so the reset succeeds and the device has its new back buffer.
    #[test]
    fn a_reset_during_a_cut_joins_it_first() {
        use super::super::d3d9draw::{as_drawing_side, draw_cut, draw_open};
        let (mut e, d3d, present) = d3d();
        let out = e.mem.alloc(4);
        e.vcall(d3d, CREATE_DEVICE, &[0, 1, 0x11c, 0x40, present, out]);
        let dev = e.mem.u32(out);
        let old = got(&mut e, dev, 38, &[0], 1)[0];
        e.vcall(old, 4 * 2, &[]);
        assert!(draw_cut(&mut e, dev));
        as_drawing_side(|| assert_eq!(e.vcall(dev, 4 * 57, &[9, 5]).u32(), D3D_OK));
        assert_eq!(e.vcall(dev, 4 * 16, &[present]).u32(), D3D_OK, "the reset");
        assert!(!draw_open(&mut e), "joined");
        let new = got(&mut e, dev, 38, &[0], 1)[0];
        assert!(new != 0 && device!(e).device_objects.surfaces.contains_key(&new));
        e.vcall(new, 4 * 2, &[]);
    }

    /// Main's locks of a buffer the drawing side wrote in the cut, by how they read: one that reads
    /// the buffer (no flags, or READONLY) waits for the drawing job and sees the drawing side's
    /// bytes; a READONLY one does not count as main's write, so the join keeps the drawing side's
    /// bytes there; one with DISCARD or NOOVERWRITE reads nothing and does not wait, and its bytes
    /// stand at the join (main's came later), the drawing side's elsewhere.
    #[test]
    fn mains_locks_during_a_cut_wait_only_when_they_read() {
        use super::super::d3d9draw::{as_drawing_side, draw_cut, draw_join};
        use super::super::d3d9resource::main_lock_waits;
        for dynamic in [false, true] {
            let (mut e, dev) = texture_device(Store::Host);
            let (usage, pool) = if dynamic { (0x200, 0) } else { (0, 1) };
            let x = new_buffer(&mut e, dev, false, 0x100, usage, pool);
            let word = |e: &mut Engine, at: u32, flags: u32| {
                let (hr, p) = lock_buffer(e, x, at, 4, flags);
                assert_eq!(hr, D3D_OK);
                let w = e.mem.u32(p);
                assert_eq!(e.vcall(x, 4 * 12, &[]).u32(), D3D_OK);
                w
            };
            let put = |e: &mut Engine, at: u32, w: u32, flags: u32| {
                let (hr, p) = lock_buffer(e, x, at, 4, flags);
                assert_eq!(hr, D3D_OK);
                e.mem.set_u32(p, w);
                assert_eq!(e.vcall(x, 4 * 12, &[]).u32(), D3D_OK);
            };
            // READONLY: waits, reads the drawing side's bytes, and is no write of main's.
            assert!(draw_cut(&mut e, dev));
            as_drawing_side(|| put(&mut e, 0x10, 0x2222_2222, 0));
            let waits = main_lock_waits();
            assert_eq!(
                word(&mut e, 0x10, 0x10),
                0x2222_2222,
                "main reads the drawing side's"
            );
            assert!(main_lock_waits() > waits, "it waited");
            draw_join(&mut e);
            assert_eq!(word(&mut e, 0x10, 0), 0x2222_2222);
            // A READONLY lock of main's before the drawing side writes there: no write of main's,
            // so the join keeps the drawing side's bytes.
            assert!(draw_cut(&mut e, dev));
            assert_eq!(
                word(&mut e, 0x18, 0x10),
                0,
                "main reads the buffer's own bytes"
            );
            as_drawing_side(|| put(&mut e, 0x18, 0x5555_5555, 0x1000));
            draw_join(&mut e);
            assert_eq!(word(&mut e, 0x18, 0), 0x5555_5555, "the drawing side's");
            // DISCARD and NOOVERWRITE: no wait; main's bytes stand in its range.
            for flags in [0x2000, 0x1000] {
                assert!(draw_cut(&mut e, dev));
                as_drawing_side(|| {
                    put(&mut e, 0x20, 0x3333_3333, 0);
                    put(&mut e, 0x30, 0x3333_3333, 0);
                });
                let before = word(&mut e, 0, 0x1000);
                put(&mut e, 0x20, 0x4444_4444, flags);
                assert_eq!(
                    before, 0,
                    "a lock that reads nothing sees the buffer's own bytes"
                );
                draw_join(&mut e);
                assert_eq!(word(&mut e, 0x20, 0), 0x4444_4444, "main's, locked later");
                assert_eq!(word(&mut e, 0x30, 0), 0x3333_3333, "the drawing side's");
            }
        }
    }

    /// The drawing side locking, to read, bytes main locked after the cut would read main's later
    /// bytes: a debug assertion names it (piece 4 keeps the two sides' locks apart).
    #[cfg(debug_assertions)]
    #[test]
    #[should_panic(expected = "that main locked after the cut")]
    fn a_drawing_lock_reading_mains_later_bytes_is_caught_in_debug_builds() {
        use super::super::d3d9draw::{as_drawing_side, draw_cut};
        let (mut e, dev) = texture_device(Store::Host);
        let x = new_buffer(&mut e, dev, false, 0x100, 0x200, 0);
        assert!(draw_cut(&mut e, dev));
        let (hr, _) = lock_buffer(&mut e, x, 0x40, 16, 0x1000);
        assert_eq!(hr, D3D_OK);
        assert_eq!(e.vcall(x, 4 * 12, &[]).u32(), D3D_OK);
        as_drawing_side(|| {
            // Reading nothing (NOOVERWRITE) is fine; reading (no flags) is not.
            let (hr, _) = lock_buffer(&mut e, x, 0x40, 16, 0x1000);
            assert_eq!(hr, D3D_OK);
            assert_eq!(e.vcall(x, 4 * 12, &[]).u32(), D3D_OK);
            lock_buffer(&mut e, x, 0x48, 4, 0);
        });
    }

    /// The drawing side's upload of a buffer range reaches the slot with its bytes at the lock's
    /// offset, from its own lock stack: main's lock of the same buffer, open meanwhile, uploads
    /// main's bytes at main's offset afterwards.
    #[cfg(windows)]
    #[test]
    fn the_drawing_sides_buffer_upload_carries_its_bytes_at_its_offset() {
        use super::super::d3d9draw::{as_drawing_side, draw_cut, draw_join};
        use d3d9_stream::{Backend, Cmd, Data, Reply, Stream};
        type Uploads = std::sync::Arc<std::sync::Mutex<Vec<(u32, Vec<u8>)>>>;
        struct Log(Uploads);
        impl Backend for Log {
            fn execute(&mut self, cmd: Cmd<Data<'_>>) -> Reply {
                if let Cmd::UploadBuffer { offset, data, .. } = cmd {
                    self.0.lock().unwrap().push((offset, data.bytes().to_vec()));
                }
                Reply::None
            }
        }
        let (mut e, dev) = texture_device(Store::Host);
        let x = new_buffer(&mut e, dev, false, 0x100, 0, 1);
        let seen: Uploads = Default::default();
        super::super::d3d9real::test_stream(
            &mut e,
            dev,
            Stream::direct(Box::new(Log(seen.clone()))),
        );
        super::super::d3d9real::test_forwarded(&mut e, x);
        let (hr, main_at) = lock_buffer(&mut e, x, 0x40, 8, 0);
        assert_eq!(hr, D3D_OK);
        e.mem.set_u32(main_at, 0xaaaa_aaaa);
        e.mem.set_u32(main_at + 4, 0xbbbb_bbbb);
        assert!(draw_cut(&mut e, dev));
        as_drawing_side(|| {
            let (hr, at) = lock_buffer(&mut e, x, 0x18, 8, 0);
            assert_eq!(hr, D3D_OK);
            e.mem.set_u32(at, 0x0102_0304);
            e.mem.set_u32(at + 4, 0x0506_0708);
            assert_eq!(e.vcall(x, 4 * 12, &[]).u32(), D3D_OK);
        });
        assert_eq!(
            *seen.lock().unwrap(),
            [(0x18, vec![4, 3, 2, 1, 8, 7, 6, 5])],
            "the drawing side's bytes at its offset"
        );
        assert_eq!(
            e.vcall(x, 4 * 12, &[]).u32(),
            D3D_OK,
            "main's lock still open"
        );
        draw_join(&mut e);
        let seen = seen.lock().unwrap();
        assert_eq!(seen.len(), 2);
        assert_eq!(
            seen[1],
            (0x40, vec![0xaa, 0xaa, 0xaa, 0xaa, 0xbb, 0xbb, 0xbb, 0xbb])
        );
    }

    /// A sync call of main's during a cut with no drawing job posted (the poster did the drawing
    /// itself): the open slot is closed and the call answered, after the slot's commands.
    #[cfg(windows)]
    #[test]
    fn mains_sync_call_during_a_cut_with_no_job_posted_is_answered() {
        use super::super::d3d9draw::{as_drawing_side, draw_cut, draw_join};
        use d3d9_stream::{Backend, Cmd, Data, Reply, Stream};
        struct Log(std::sync::Arc<std::sync::Mutex<Vec<String>>>);
        impl Backend for Log {
            fn execute(&mut self, cmd: Cmd<Data<'_>>) -> Reply {
                match cmd {
                    Cmd::SetRenderState { state, value, .. } => {
                        self.0.lock().unwrap().push(format!("{state}={value}"));
                        Reply::None
                    }
                    Cmd::ReadSurface { size, .. } => {
                        self.0.lock().unwrap().push("read".into());
                        Reply::Bytes(Some(vec![1; size as usize]))
                    }
                    _ => Reply::None,
                }
            }
        }
        let (mut e, dev) = texture_device(Store::Host);
        let seen = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        super::super::d3d9real::test_stream(
            &mut e,
            dev,
            Stream::direct(Box::new(Log(seen.clone()))),
        );
        super::super::d3d9real::test_forwarded(&mut e, 0x0bad_5000);
        let at = e.mem.alloc(8);
        assert!(draw_cut(&mut e, dev));
        as_drawing_side(|| assert_eq!(e.vcall(dev, 4 * 57, &[9, 2]).u32(), D3D_OK));
        assert_eq!(e.vcall(dev, 4 * 57, &[8, 3]).u32(), D3D_OK);
        assert!(super::super::d3d9real::read_surface(
            &mut e,
            0x0bad_5000,
            at,
            8
        ));
        assert_eq!(*seen.lock().unwrap(), ["9=2", "8=3", "read"]);
        draw_join(&mut e);
    }

    /// The viewport's words through `SetViewport`.
    fn set_viewport_words(e: &mut Engine, device: u32, rect: [u32; 4]) {
        let v = e.mem.alloc(24);
        for (i, w) in rect
            .iter()
            .chain(&[0f32.to_bits(), 1f32.to_bits()])
            .enumerate()
        {
            e.mem.set_u32(v + 4 * i as u32, *w);
        }
        assert_eq!(set_viewport(e, Ptr::new(device), Ptr::new(v)), D3D_OK);
        e.mem.free(v);
    }
}
