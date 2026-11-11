//! The Direct3D 9 device on wgpu: resources by the game's handles, the device state, render
//! passes, pipelines made from the state and draws. Each method is one recorded call
//! (`d3d9-stream`'s `Cmd`), with the meaning the DirectX 9 SDK gives it; the forwarding layer
//! (`engine-platform` `d3d9real.rs`) is the reference for which calls reach a backend.
//!
//! Correctness first (ADR-0007's phase 1): every draw sets its whole state; uploads into a
//! resource the frame's unsent commands still use send those commands first. The caches keyed by
//! state (pipelines, shader modules, samplers, bind groups) are where the speed work starts.

use crate::answers::QueryAnswers;
use crate::cache;
use crate::format::{self, d3dfmt, Format};
use crate::fxhash::Map;
use crate::loading;
use crate::prepare;
use crate::shot::{self, Shots};
use crate::sm::{self, wgsl, Stage};
use crate::state::{self, rs, SamplerKey};
use d3d9_stream::{Handle, Rect};
use std::collections::HashSet;
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::sync::Arc;

/// Where the device's frames go.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Output {
    /// To the game's window (`D3DPRESENT_PARAMETERS.hDeviceWindow`), as Direct3D 9 presents.
    Window,
    /// Nowhere but `NV_SHOT` captures: the shadow of another backend that owns the window.
    Offscreen,
}

/// A texture (2D, or a cube's six faces as layers) and the batch that last used it.
pub(crate) struct Tex {
    texture: wgpu::Texture,
    view: wgpu::TextureView,
    format: Format,
    width: u32,
    height: u32,
    levels: u32,
    cube: bool,
    /// Samples a pixel (`D3DMULTISAMPLE_TYPE`; 1 without multisampling).
    samples: u32,
    id: u64,
    batch: AtomicU64,
}

/// A surface: a level (and face) of a texture, or a standalone render target or depth surface
/// (a texture of one level), or system memory only.
pub(crate) struct Surf {
    tex: Option<Arc<Tex>>,
    level: u32,
    layer: u32,
    width: u32,
    height: u32,
    format: Format,
    samples: u32,
    /// The view a render pass draws into (one level, one layer).
    view: Option<wgpu::TextureView>,
    id: u64,
}

pub(crate) struct Buf {
    buffer: wgpu::Buffer,
    index32: bool,
    batch: AtomicU64,
}

pub(crate) struct Shader {
    program: sm::Program,
    sig: wgsl::Signature,
    id: u64,
    /// The float constants its uniform block must carry ([`wgsl::float_constants_read`]).
    fregs: u32,
    /// Its inputs' varyings (a pixel shader's link to the vertex shader).
    varyings: Vec<wgsl::Semantic>,
    /// Its bytecode and the bytecode's hash, the same in every run ([`prepare::bytecode_hash`]).
    tokens: Vec<u32>,
    stable: u64,
}

/// A `D3DVERTEXELEMENT9`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct Element {
    stream: u16,
    offset: u16,
    ty: u8,
    usage: u8,
    index: u8,
}

pub(crate) struct Decl {
    elements: Vec<Element>,
    id: u64,
}

enum Obj {
    Texture(Arc<Tex>, Vec<Handle>),
    Surface(Arc<Surf>),
    Buffer(Arc<Buf>, Vec<u8>),
    Shader(Arc<Shader>),
    Decl(Arc<Decl>),
    Query(Query),
}

/// A query (`CreateQuery`): an event's answer comes from the model; an occlusion query counts
/// the samples its draws pass ([`Device::query_issue`]).
#[derive(Clone, Debug, Default)]
struct Query {
    /// `D3DQUERYTYPE`: 8 event, 9 occlusion.
    kind: u32,
    /// Its count's slot in the count buffer, from its last `D3DISSUE_BEGIN`.
    slot: Option<u32>,
    /// The batch that ended it (its count is in the GPU's hands once that batch is sent).
    batch: u64,
    /// The count's read-back on its way (asked for by a `GetData` after the batch was sent).
    read: Option<CountRead>,
    /// The count, once read back.
    result: Option<u32>,
}

/// An occlusion count being read back: its 4 bytes, and the map's state (0 on its way, 1 mapped,
/// 2 failed), which wgpu sets from a poll.
#[derive(Clone, Debug)]
struct CountRead {
    buffer: wgpu::Buffer,
    state: Arc<std::sync::atomic::AtomicU8>,
}

/// The occlusion count's target of the open pass ([`wgsl::COUNT_TARGET`]): each sample a draw
/// passes adds 1 there; summed into `slot` of the count buffer when the pass ends.
struct CountSegment {
    view: wgpu::TextureView,
    samples: u32,
    width: u32,
    height: u32,
    slot: u32,
}

#[derive(Clone, Copy, Debug, Default)]
struct Viewport {
    x: u32,
    y: u32,
    w: u32,
    h: u32,
    min_z: f32,
    max_z: f32,
}

#[derive(Clone, Default)]
struct Stream {
    buffer: Option<Arc<Buf>>,
    offset: u32,
    stride: u32,
}

/// The D3D9 device state.
struct State {
    rt: [Option<Arc<Surf>>; 4],
    ds: Option<Arc<Surf>>,
    viewport: Viewport,
    scissor: Rect,
    rs: Box<[u32; 256]>,
    /// 16 pixel samplers, then the 4 vertex samplers (`D3DVERTEXTEXTURESAMPLER0` = 257).
    samplers: Box<[[u32; 14]; 20]>,
    textures: [Option<Arc<Tex>>; 20],
    vs: Option<Arc<Shader>>,
    ps: Option<Arc<Shader>>,
    decl: Option<Arc<Decl>>,
    streams: [Stream; 16],
    indices: Option<Arc<Buf>>,
    /// Shader constants as the model holds them: per stage float (256 registers of 4 words, as
    /// the register block carries them: little-endian bytes), int (4 words), bool (1).
    vsf: Box<[u8; 4096]>,
    vsi: [[u32; 4]; 16],
    vsb: [u32; 16],
    psf: Box<[u8; 4096]>,
    psi: [[u32; 4]; 16],
    psb: [u32; 16],
    /// The user clip planes (`SetClipPlane`), float bits.
    clip: [[u32; 4]; wgsl::CLIP_PLANES],
    /// The vertex and pixel stage's uniform blocks need writing again.
    dirty: [bool; 2],
    /// ... and their headers, everything but the float constants, need making again
    /// ([`Device::write_blocks`]): set with `dirty` by every change but a float constant's.
    header_dirty: [bool; 2],
    /// The handles the bindings were last set from ([`Bound`]).
    bound: Bound,
}

/// A handle no binding holds: the object at a handle may have changed since it was bound
/// ([`Bound::forget`]).
const UNKNOWN: Handle = Handle::MAX;

/// The handles `SetTexture`, `SetStreamSource` (with its offset and stride) and `SetIndices` last
/// bound, so binding the same again changes nothing and looks nothing up (the game binds many again
/// each draw). 0 is no object.
#[derive(Clone, Default)]
struct Bound {
    textures: [Handle; 20],
    streams: [(Handle, u32, u32); 16],
    indices: Handle,
    /// The vertex and pixel shader, the declaration.
    shaders: [Handle; 2],
    declaration: Handle,
}

impl Bound {
    /// The object at `h` is gone or new: a binding of `h` looks it up again.
    fn forget(&mut self, h: Handle) {
        for t in &mut self.textures {
            if *t == h {
                *t = UNKNOWN;
            }
        }
        for s in &mut self.streams {
            if s.0 == h {
                s.0 = UNKNOWN;
            }
        }
        if self.indices == h {
            self.indices = UNKNOWN;
        }
        for s in &mut self.shaders {
            if *s == h {
                *s = UNKNOWN;
            }
        }
        if self.declaration == h {
            self.declaration = UNKNOWN;
        }
    }
}

impl State {
    /// Something a stage's block header carries changed (`stage` 0 vertex, 1 pixel).
    fn touch(&mut self, stage: usize) {
        self.dirty[stage] = true;
        self.header_dirty[stage] = true;
    }
}

/// What a render pass draws into.
#[derive(Clone, PartialEq, Eq)]
struct Targets {
    colours: [Option<u64>; 4],
    depth: Option<u64>,
    /// An occlusion query counts in this pass ([`CountSegment`]).
    counting: bool,
}

#[derive(Clone, PartialEq, Eq, Hash)]
struct PipeKey {
    vs: u64,
    ps: u64,
    decl: u64,
    strides: [u32; 8],
    topology: u8,
    index32: bool,
    colours: [Option<(wgpu::TextureFormat, bool)>; 4],
    depth: Option<wgpu::TextureFormat>,
    states: [u32; 30],
    samples: u32,
    /// Whether each of the shaders' samplers ([`Device::sampler_bindings`], in order) filters:
    /// with the two shaders, what the bind group's layout is made from.
    filterable: u32,
    /// NVIDIA's alpha to coverage in place of the alpha test ([`ATOC`]).
    atoc: bool,
    /// The pass has the occlusion count's target.
    count: bool,
    /// The user clip planes' code ([`Device::clip_link`]).
    clip: wgsl::Clip,
}

/// The samplers a draw's two shaders use ([`Device::sampler_bindings`]): (binding, dimension,
/// filterable) and the device's sampler slot; at most 16 pixel and 4 vertex samplers.
#[derive(Clone, Copy, Default)]
struct Bindings {
    n: usize,
    b: [((u32, u8, bool), usize); 20],
}

impl Bindings {
    fn as_slice(&self) -> &[((u32, u8, bool), usize)] {
        &self.b[..self.n]
    }
}

/// A bind group's key: each binding's number, the dimension its shader declares (2D, cube, volume:
/// the layout the group is made for, and the dummy a texture of the other kind is replaced by), its
/// texture and its sampler.
type GroupKey = Vec<(u32, u8, u64, Option<SamplerKey>)>;

/// A vertex shader's inputs read from a declaration ([`Device::vertex_link`]).
struct VertexLink {
    inputs: Vec<wgsl::Input>,
    attributes: [Vec<wgpu::VertexAttribute>; 8],
}

/// What making pipelines cost since the last report (printed with the present counters every
/// 600 presents): a pipeline made on first use holds the draw, and so the frame, until it is done.
#[derive(Default)]
struct Made {
    pipelines: u32,
    modules: u32,
    convert: std::time::Duration,
    module: std::time::Duration,
    pipeline: std::time::Duration,
    longest: std::time::Duration,
    /// The pipelines' whole times (their modules included), and how many took over 16 ms.
    whole: std::time::Duration,
    over_16ms: u32,
    /// The longest time between two presents on the device's thread (the render thread).
    longest_frame: std::time::Duration,
}

impl Made {
    fn add_module(&mut self, convert: std::time::Duration, module: std::time::Duration) {
        self.modules += 1;
        self.convert += convert;
        self.module += module;
    }
    fn add_pipeline(&mut self, pipeline: std::time::Duration, all: std::time::Duration) {
        self.pipelines += 1;
        self.pipeline += pipeline;
        self.longest = self.longest.max(all);
        self.whole += all;
        if all.as_millis() > 16 {
            self.over_16ms += 1;
        }
    }
}

/// The list of pipelines to make before play and what of it is done ([`prepare`]).
struct Listed {
    list: prepare::List,
    /// Entry by entry, the sample counts (as bits, as [`Device::sample_mask`]) it was made at or
    /// tried and given up at (its shaders collide, a format is unknown); none yet: 0.
    done: Vec<u32>,
    /// The file and its lines, shared with the writer threads and the exit hooks.
    shared: Arc<ListFile>,
    /// Entries this run added that the file does not have yet, and the present of the last.
    dirty: bool,
    added_at: u64,
    /// The present of the first entry the file does not have yet.
    first_added_at: u64,
}

/// The list's file: its lines as the device has them, written on a thread of its own or by the
/// exit hooks ([`save_lists_now`]).
struct ListFile {
    file: std::path::PathBuf,
    lines: std::sync::Mutex<Vec<String>>,
    /// How many lines the file has (none written yet: the count read).
    written: std::sync::atomic::AtomicUsize,
    writing: std::sync::Mutex<()>,
    /// A write failed: none more this run (said once).
    failed: std::sync::atomic::AtomicBool,
}

impl ListFile {
    fn save(&self) {
        if self.failed.load(Ordering::Relaxed) {
            return;
        }
        let _one = self.writing.lock().unwrap_or_else(|e| e.into_inner());
        let text = {
            let lines = self.lines.lock().unwrap_or_else(|e| e.into_inner());
            if lines.len() == self.written.load(Ordering::Relaxed) {
                return;
            }
            self.written.store(lines.len(), Ordering::Relaxed);
            let mut t = String::new();
            for l in lines.iter() {
                t.push_str(l);
                t.push('\n');
            }
            t
        };
        if let Err(e) = cache::write_replacing(&self.file, text.as_bytes()) {
            self.failed.store(true, Ordering::Relaxed);
            eprintln!(
                "NV_D3D9 wgpu: pipeline list {}: not written: {e}: none written this run",
                self.file.display()
            );
        }
    }
}

/// The devices' list files, for [`save_lists_now`].
static LISTS: std::sync::Mutex<Vec<std::sync::Weak<ListFile>>> = std::sync::Mutex::new(Vec::new());

/// Writes every live device's pipeline list now, when it has new entries (the end of the run:
/// the process ends with `std::process::exit`, so no device is dropped).
pub fn save_lists_now() {
    let all: Vec<Arc<ListFile>> = LISTS
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .iter()
        .filter_map(std::sync::Weak::upgrade)
        .collect();
    for l in all {
        l.save();
    }
}

/// The list shipped with the port (`data/pipelines.txt`), recorded on the maintainer's machine:
/// four recordings of the game's Direct3D 9 calls (the intro and the main menu; Save 138's load
/// and Doc Mitchell's house; the race menu; the Goodsprings street and a full turn there, at 4x
/// antialiasing) replayed into this backend on Vulkan, each with an empty folder, their lists
/// joined in that order, each entry once. It holds no game file's bytes: hashes of the game's
/// shaders, format numbers and render-state values.
const SHIPPED_LIST: &str = include_str!("../data/pipelines.txt");

impl Listed {
    fn load(dir: &std::path::Path) -> Listed {
        cache::remove_stale_temporaries(dir, &["pipelines"]);
        let file = dir.join("pipelines.txt");
        let mut list = prepare::List::default();
        let (shipped, s1) = list.merge(SHIPPED_LIST);
        let (local, s2) = match std::fs::read_to_string(&file) {
            Ok(text) => list.merge(&text),
            Err(_) => (0, 0),
        };
        eprintln!(
            "NV_D3D9 wgpu: pipeline list: {} entries ({shipped} shipped, {local} more from {}; {} lines not read)",
            list.entries.len(),
            file.display(),
            s1 + s2
        );
        let lines: Vec<String> = list.entries.iter().map(prepare::Entry::line).collect();
        let shared = Arc::new(ListFile {
            file,
            // Written when this run adds a line (the file then holds the shipped ones too).
            written: std::sync::atomic::AtomicUsize::new(lines.len()),
            lines: std::sync::Mutex::new(lines),
            writing: std::sync::Mutex::new(()),
            failed: Default::default(),
        });
        let mut all = LISTS.lock().unwrap_or_else(|e| e.into_inner());
        all.retain(|w| w.strong_count() > 0);
        all.push(Arc::downgrade(&shared));
        Listed {
            done: vec![0; list.entries.len()],
            list,
            shared,
            dirty: false,
            added_at: 0,
            first_added_at: 0,
        }
    }

    /// `e` listed (it was made on a draw at present `presents`), unless it is already.
    fn add(&mut self, e: prepare::Entry, presents: u64) {
        let line = e.line();
        let samples = 1u32 << e.samples.min(31);
        if self.list.add(e) {
            self.done.push(samples);
            if !self.dirty {
                self.first_added_at = presents;
            }
            self.dirty = true;
            self.added_at = presents;
            self.shared
                .lines
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .push(line);
        }
    }

    /// Writes the list 300 presents after the last new entry (or every 3600 while they keep
    /// coming; at once with `now`), on a thread of its own.
    fn save_if_due(&mut self, presents: u64, now: bool) {
        if !self.dirty {
            return;
        }
        let quiet = presents.saturating_sub(self.added_at) >= 300;
        let long = presents.saturating_sub(self.first_added_at) >= 3600;
        if !(now || quiet || long) {
            return;
        }
        self.dirty = false;
        let shared = self.shared.clone();
        if now {
            shared.save();
        } else {
            std::thread::spawn(move || shared.save());
        }
    }
}

/// What the open render pass was last given, so a draw sends only what changed.
#[derive(Default)]
struct PassBinds {
    pipeline: Option<wgpu::RenderPipeline>,
    blocks: Option<(u32, u32)>,
    group: Option<wgpu::BindGroup>,
    viewport: Option<[u32; 6]>,
    scissor: Option<(u32, u32, u32, u32)>,
    stencil: Option<u32>,
    blend: Option<u32>,
    streams: [Option<(wgpu::Buffer, u64)>; 8],
    index: Option<(wgpu::Buffer, bool)>,
}

/// The render states a pipeline is made from.
const PIPELINE_STATES: [usize; 30] = [
    rs::ZENABLE,
    rs::ZWRITEENABLE,
    rs::ZFUNC,
    rs::CULLMODE,
    rs::ALPHABLENDENABLE,
    rs::SRCBLEND,
    rs::DESTBLEND,
    rs::BLENDOP,
    rs::SEPARATEALPHABLENDENABLE,
    rs::SRCBLENDALPHA,
    rs::DESTBLENDALPHA,
    rs::BLENDOPALPHA,
    rs::COLORWRITEENABLE,
    rs::COLORWRITEENABLE1,
    rs::COLORWRITEENABLE2,
    rs::COLORWRITEENABLE3,
    rs::STENCILENABLE,
    rs::STENCILFAIL,
    rs::STENCILZFAIL,
    rs::STENCILPASS,
    rs::STENCILFUNC,
    rs::STENCILMASK,
    rs::STENCILWRITEMASK,
    rs::TWOSIDEDSTENCILMODE,
    rs::CCW_STENCILFAIL,
    rs::CCW_STENCILZFAIL,
    rs::CCW_STENCILFUNC,
    rs::CCW_STENCILPASS,
    rs::DEPTHBIAS,
    rs::SLOPESCALEDEPTHBIAS,
];

/// `D3DRS_ADAPTIVETESS_Y` set to `MAKEFOURCC('A','T','O','C')`: NVIDIA's Direct3D 9 driver's
/// switch from the alpha test to alpha to coverage ("transparency multisampling"), in effect while
/// the alpha test is on and the target is multisampled. The game's renderer sets it on this card
/// (Doc's house: about 14,000 times in a minute, around its alpha-tested draws). What the driver
/// does with it is NVIDIA's documented behaviour, which DXVK applies the same way: coverage from
/// the pixel's alpha, the alpha test itself off. Measured on the real Direct3D 9 here (RTX 5070
/// Ti laptop, 4x and 8x, alpha in 64 steps, every comparison, references 0, 0x80 and 0xFF;
/// DESIGN.md section 10): the samples covered for each alpha are the ones wgpu's alpha to
/// coverage gives on the same card, the reference value plays no part, GREATER, NOTEQUAL,
/// GREATEREQUAL and ALWAYS cover from the alpha, and NEVER, LESS, EQUAL and LESSEQUAL draw
/// nothing. Which samples another vendor's hardware covers for an alpha is its own.
const ATOC: u32 = u32::from_le_bytes(*b"ATOC");
/// `D3DRS_CLIPPING` and `D3DRS_ADAPTIVETESS_Y`.
const RS_CLIPPING: usize = 136;
const RS_ADAPTIVETESS_Y: usize = 181;
/// The occlusion counts' slots: a ring (a query's count is read long before its slot comes round
/// again).
const COUNT_SLOTS: u32 = 4096;
/// The occlusion count's target formats ([`Device::count_format`]): 32-bit float, whose whole
/// counts are exact up to 16,777,216 passes of one sample in one pass (more than a pass can draw
/// over one sample), where the adapter blends it; else 16-bit float, exact up to 2,048.
const COUNT_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::R32Float;
const COUNT_FORMAT_16: wgpu::TextureFormat = wgpu::TextureFormat::R16Float;
/// The formats the game makes multisampled surfaces in (`d3d9-stats` of the recordings in Doc's
/// house and the race menu): `D3DFMT_A8R8G8B8`, `D3DFMT_A16B16G16R16F` render targets and their
/// `D3DFMT_D24S8` depth surfaces. One sample count serves them all ([`Device::sample_count`]).
const MULTISAMPLED_FORMATS: [u32; 3] = [21, 113, 75];

/// [`Device::sample_count`] with the adapter's answer `supported(format, count)`.
fn common_sample_count(
    format: wgpu::TextureFormat,
    asked: u32,
    supported: impl Fn(wgpu::TextureFormat, u32) -> bool,
) -> u32 {
    let family: Vec<wgpu::TextureFormat> = MULTISAMPLED_FORMATS
        .iter()
        .filter_map(|&d3d| format::format(d3d).map(|f| f.wgpu))
        .chain([format, COUNT_FORMAT_16])
        .collect();
    [16u32, 8, 4, 2]
        .into_iter()
        .filter(|&c| c <= asked)
        .find(|&c| family.iter().all(|&f| supported(f, c)))
        .unwrap_or(1)
}

/// The uniform block of one stage in the ring, rounded up to the offset alignment.
const BLOCK: u64 = wgsl::REGS_SIZE.div_ceil(256) * 256;
/// The ring of uniform blocks and `DrawPrimitiveUP` vertices sent with each batch.
const RING: u64 = 32 << 20;
const UP_RING: u64 = 8 << 20;
/// The upload stage: buffer uploads into buffers the open batch draws from, copied in among the
/// batch's commands ([`Device::upload_buffer`]).
const STAGE: u64 = 8 << 20;

pub struct Device {
    pub(crate) device: wgpu::Device,
    pub(crate) queue: wgpu::Queue,
    adapter: wgpu::Adapter,
    adapter_name: String,
    /// The game's window and its swap chain ([`crate::window::Window`]), with [`Output::Window`].
    surface: Option<crate::window::Window>,
    objects: Map<Handle, Obj>,
    back_buffers: Vec<Handle>,
    auto_depth: Handle,
    st: State,
    encoder: Option<wgpu::CommandEncoder>,
    /// The batch's earlier encoders, in order: finished on the finishing threads (or here), their
    /// command buffers sent with the batch before the open encoder's ([`Device::send_encoder`]).
    finishing: Vec<Finishing>,
    finisher: Option<Finisher>,
    /// Draws recorded into the open encoder.
    encoder_draws: u32,
    pass: Option<wgpu::RenderPass<'static>>,
    pass_targets: Option<Targets>,
    batch: u64,
    ring: Vec<u8>,
    ring_buffer: wgpu::Buffer,
    ring_group: wgpu::BindGroup,
    uniform_layout: wgpu::BindGroupLayout,
    up: Vec<u8>,
    up_buffer: wgpu::Buffer,
    stage: Vec<u8>,
    stage_buffer: wgpu::Buffer,
    blocks: (u32, u32),
    /// The float constants each stage's last written block carries.
    block_fregs: [u32; 2],
    #[allow(clippy::type_complexity)]
    sampler_layouts: Map<Vec<(u32, u8, bool)>, (wgpu::BindGroupLayout, wgpu::PipelineLayout)>,
    pipelines: Map<PipeKey, Option<(wgpu::RenderPipeline, wgpu::BindGroupLayout)>>,
    /// The last draw's pipeline key and pipeline, its bind group's key and group, its vertex
    /// link; the vertex links by shader and declaration; what the open pass was given.
    #[allow(clippy::type_complexity)]
    last_pipeline: Option<(
        PipeKey,
        Option<(wgpu::RenderPipeline, wgpu::BindGroupLayout)>,
    )>,
    last_group: (GroupKey, Option<wgpu::BindGroup>),
    last_link: Option<((u64, u64), Arc<VertexLink>)>,
    vertex_links: Map<(u64, u64), Arc<VertexLink>>,
    binds: PassBinds,
    /// The driver's compiled pipelines kept between runs ([`cache`]), where the backend has them.
    pipeline_cache: Option<cache::PipelineCacheFile>,
    /// When the last `Present` began ([`Made::longest_frame`]).
    last_present: Option<std::time::Instant>,
    /// The screen of the backend's own long steps ([`Device::show_progress`]), made when first shown.
    loading: Option<loading::Screen>,
    /// The shaders by bytecode hash; the hashes two bytecodes share; the declarations by their
    /// elements ([`Device::create_shader`], [`Device::decl_of`]).
    shader_bytecode: Map<u64, Vec<Arc<Shader>>>,
    collided: HashSet<u64>,
    decls_by_elements: Map<Vec<Element>, Arc<Decl>>,
    /// The pipelines to make before play ([`prepare`]): the list, which of its entries are made
    /// (or tried), whether a new shader or sample count may have made more of them ready, the
    /// local list's file and whether it has entries it has not written. None without a cache folder.
    list: Option<Listed>,
    /// The sample counts of the surfaces made so far, as bits (1 always).
    sample_mask: u32,
    listed_new: bool,
    made: Made,
    #[allow(clippy::type_complexity)]
    vs_modules: Map<
        (
            u64,
            Vec<wgsl::Input>,
            Vec<wgsl::Semantic>,
            Vec<bool>,
            wgsl::Clip,
        ),
        Option<wgpu::ShaderModule>,
    >,
    #[allow(clippy::type_complexity)]
    ps_modules: Map<(u64, Vec<wgsl::Semantic>, bool, wgsl::Clip), Option<wgpu::ShaderModule>>,
    samplers: Map<SamplerKey, wgpu::Sampler>,
    groups: Map<GroupKey, wgpu::BindGroup>,
    blit: Map<(wgpu::TextureFormat, bool), wgpu::RenderPipeline>,
    blit_layout: Option<wgpu::BindGroupLayout>,
    #[allow(clippy::type_complexity)]
    gamma_resolves:
        Map<(wgpu::TextureFormat, u32, bool), (wgpu::RenderPipeline, wgpu::BindGroupLayout)>,
    /// Depth stand-ins for targets smaller than the bound depth surface ([`Device::usable_depth`]).
    depth_parts: Map<(u64, u32, u32), Arc<Surf>>,
    #[allow(clippy::type_complexity)]
    clears: Map<
        (
            [Option<(wgpu::TextureFormat, bool)>; 4],
            Option<wgpu::TextureFormat>,
            u32,
        ),
        (wgpu::RenderPipeline, wgpu::BindGroupLayout),
    >,
    dummy: [wgpu::TextureView; 3],
    border: bool,
    float_filterable: bool,
    /// Whether the adapter has clip distances for the user clip planes ([`Device::clip_link`]).
    clip_distances: bool,
    /// The pixel-centre move the last written vertex block carries ([`Device::write_blocks`]).
    block_centre: [f32; 2],
    /// Each stage's block header as last made, and the pixel-centre move and alpha to coverage
    /// switch they were made with ([`Device::write_blocks`]).
    headers: [[u8; HEADER]; 2],
    header_centre: [f32; 2],
    header_atoc: bool,
    /// The levels whose bytes the backend keeps (managed and system-memory pools: what
    /// `ReadSurface` answers), by the texture or surface they belong to; and those bytes, as the
    /// game last gave them (`UploadSurface`) or `GetRenderTargetData` wrote them.
    keeps: Map<Handle, Vec<Handle>>,
    kept: Map<Handle, Vec<u8>>,
    /// The occlusion query counting now (its `D3DISSUE_BEGIN` until its `END`) and its slot.
    counting: Option<(Handle, u32)>,
    pass_count: Option<CountSegment>,
    count_targets: Map<(u32, u32, u32), wgpu::TextureView>,
    count_buffer: wgpu::Buffer,
    next_count_slot: u32,
    reduce: Map<bool, (wgpu::ComputePipeline, wgpu::BindGroupLayout)>,
    /// The copies of textures a draw samples while drawing into them, by the texture's id
    /// ([`Device::copy_for_sampling`]).
    target_copies: Map<u64, Arc<Tex>>,
    /// Textures and surfaces destroyed since the last `Present`, by id: the cached bind groups and
    /// depth stand-ins that hold them are dropped then ([`Device::purge_destroyed`]).
    destroyed: HashSet<u64>,
    next_id: u64,
    presents: u64,
    shots: Option<Shots>,
    warned: HashSet<String>,
    errors: Arc<AtomicU64>,
    pub(crate) draws: u64,
    pub(crate) skipped: u64,
    /// Pipelines made on the draw that needed them (not ahead from the list).
    made_on_draws: u64,
    /// Where the occlusion counts are posted for the game's threads ([`QueryAnswers`]), and the
    /// `END`s (query, number) whose counts are awaited.
    pub(crate) answers: Option<Arc<QueryAnswers>>,
    awaiting: Vec<(Handle, u64)>,
}

pub(crate) fn block_on<F: std::future::Future>(f: F) -> F::Output {
    // wgpu's native futures are ready when polled after the work they wait for (a `poll`).
    struct Noop;
    impl std::task::Wake for Noop {
        fn wake(self: Arc<Self>) {}
    }
    let waker = std::task::Waker::from(Arc::new(Noop));
    let mut cx = std::task::Context::from_waker(&waker);
    let mut f = std::pin::pin!(f);
    loop {
        if let std::task::Poll::Ready(v) = f.as_mut().poll(&mut cx) {
            return v;
        }
        std::thread::yield_now();
    }
}

/// The present parameters' words (`D3DPRESENT_PARAMETERS`).
pub(crate) struct PresentParameters {
    pub width: u32,
    pub height: u32,
    pub format: u32,
    pub auto_depth: bool,
    pub depth_format: u32,
    pub interval: u32,
}

impl PresentParameters {
    pub fn from_words(w: &[u32]) -> Self {
        let at = |i: usize| w.get(i).copied().unwrap_or(0);
        PresentParameters {
            width: at(0),
            height: at(1),
            format: at(2),
            auto_depth: at(9) != 0,
            depth_format: at(10),
            interval: at(13),
        }
    }
}

/// `D3DPOOL_MANAGED`, `D3DPOOL_SYSTEMMEM`, `D3DPOOL_SCRATCH`.
const POOL_MANAGED: u32 = 1;
const POOL_SYSTEMMEM: u32 = 2;
const POOL_SCRATCH: u32 = 3;
/// `D3DQUERYTYPE_OCCLUSION`; `D3DISSUE_END`, `D3DISSUE_BEGIN`.
const QUERY_OCCLUSION: u32 = 9;
const ISSUE_END: u32 = 1;
const ISSUE_BEGIN: u32 = 2;
/// `S_FALSE`: a query's result is not in yet.
const S_FALSE: u32 = 1;
/// `D3DGETDATA_FLUSH`.
const GETDATA_FLUSH: u32 = 1;

impl Device {
    /// The wgpu device for the game's (`CreateDevice`): on the game's real window `window` when
    /// `output` is [`Output::Window`], with the window's swap chain configured. Why none, when no
    /// graphics API has a device that can show the game in the window (the platform then draws
    /// with Direct3D 9).
    pub(crate) fn new(
        output: Output,
        window: isize,
        pp: &PresentParameters,
        back_buffers: &[Handle],
        depth: Handle,
        cache_dir: Option<&std::path::Path>,
    ) -> Result<Device, String> {
        // The graphics APIs in [`apis`]'s order: Vulkan, then Direct3D 12 for a card without a
        // Vulkan driver, or whose Vulkan device or swap chain fails; `NV_WGPU_BACKEND` names one.
        let asked = std::env::var("NV_WGPU_BACKEND").ok();
        let (tries, software) = apis(asked.as_deref());
        let shots = Shots::from_env();
        let copies = shots.is_some();
        let mut opened = None;
        let mut software_one = None;
        let mut failed = Vec::new();
        for backends in tries {
            let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
                backends,
                flags: if std::env::var_os("NV_WGPU_DEBUG").is_some() {
                    wgpu::InstanceFlags::debugging()
                } else {
                    wgpu::InstanceFlags::empty()
                },
                ..wgpu::InstanceDescriptor::new_without_display_handle()
            });
            let surface = match output {
                Output::Window if window != 0 => match surface_for(&instance, window) {
                    Ok(s) => Some(s),
                    // The window on this API failed: the next API is tried.
                    Err(e) => {
                        eprintln!("NV_D3D9 wgpu: {backends:?}: {e}");
                        continue;
                    }
                },
                _ => None,
            };
            let Ok(adapter) = block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                force_fallback_adapter: software,
                compatible_surface: surface.as_ref(),
                apply_limit_buckets: false,
            })) else {
                continue;
            };
            // A graphics card's device and swap chain end the search; when either fails, the next
            // API is tried. A software adapter (a CPU one) stands only until a later API has a
            // card, or is asked for: with none, the last API's (WARP, Direct3D 12's, on Windows,
            // as CI's runners had before Vulkan came first).
            let cpu = adapter.get_info().device_type == wgpu::DeviceType::Cpu;
            if cpu && !software {
                software_one = Some((backends, instance, surface, adapter));
                continue;
            }
            match open(instance, surface, adapter, window, pp, copies) {
                Ok(o) => {
                    opened = Some(o);
                    break;
                }
                Err(e) => {
                    eprintln!("NV_D3D9 wgpu: {backends:?}: {e}");
                    failed.push(format!("{backends:?}"));
                }
            }
        }
        // A software adapter only on a machine without a graphics card: after a card's device
        // failed, the platform's Direct3D 9 draws on that card (not the CPU at a few frames a
        // second, which the hardware check of #422 keeps players from).
        if opened.is_none() && failed.is_empty() {
            if let Some((backends, instance, surface, adapter)) = software_one {
                match open(instance, surface, adapter, window, pp, copies) {
                    Ok(o) => opened = Some(o),
                    Err(e) => {
                        eprintln!("NV_D3D9 wgpu: {backends:?}: {e}");
                        failed.push(format!("{backends:?}"));
                    }
                }
            }
        }
        // None: the platform draws with Direct3D 9 (`engine-platform`'s `d3d9real`).
        let Some(Opened {
            adapter,
            device,
            queue,
            features,
            window: shown_in,
        }) = opened
        else {
            return Err(if failed.is_empty() {
                "no wgpu adapter".into()
            } else {
                format!(
                    "no device on {} could show the game (why above)",
                    failed.join(", ")
                )
            });
        };
        // The driver's compiled pipelines from the last run, where the backend keeps them (Vulkan).
        let pipeline_cache =
            cache_dir.and_then(|d| cache::PipelineCacheFile::open(&device, &adapter.get_info(), d));
        // The pipelines to make before play: the shipped list and this machine's.
        let list = cache_dir.map(Listed::load);
        let errors = Arc::new(AtomicU64::new(0));
        {
            let errors = errors.clone();
            device.on_uncaptured_error(Arc::new(move |e: wgpu::Error| {
                if errors.fetch_add(1, Ordering::Relaxed) < 20 {
                    eprintln!("NV_D3D9 wgpu: {e}");
                }
            }));
        }
        let info = adapter.get_info();
        let adapter_name = format!("{} ({:?})", info.name, info.backend);
        let uniform_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("registers"),
            entries: &[0, 1].map(|binding| wgpu::BindGroupLayoutEntry {
                binding,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: true,
                    min_binding_size: std::num::NonZeroU64::new(wgsl::REGS_SIZE),
                },
                count: None,
            }),
        });
        let ring_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("register ring"),
            size: RING,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let ring_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("registers"),
            layout: &uniform_layout,
            entries: &[0, 1].map(|binding| wgpu::BindGroupEntry {
                binding,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: &ring_buffer,
                    offset: 0,
                    size: std::num::NonZeroU64::new(wgsl::REGS_SIZE),
                }),
            }),
        });
        let stage_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("upload stage"),
            size: STAGE,
            usage: wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let up_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("DrawPrimitiveUP vertices"),
            size: UP_RING,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let dummy = dummies(&device, &queue);
        let count_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("occlusion counts"),
            size: u64::from(COUNT_SLOTS) * 4,
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_SRC
                | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let mut d = Device {
            device,
            queue,
            adapter: adapter.clone(),
            adapter_name,
            surface: None,
            objects: Map::default(),
            back_buffers: Vec::new(),
            auto_depth: 0,
            st: State {
                rt: Default::default(),
                ds: None,
                viewport: Viewport::default(),
                scissor: [0; 4],
                rs: state::default_render_states(pp.auto_depth),
                samplers: Box::new([state::default_sampler_states(); 20]),
                textures: Default::default(),
                vs: None,
                ps: None,
                decl: None,
                streams: Default::default(),
                indices: None,
                vsf: Box::new([0; 4096]),
                vsi: [[0; 4]; 16],
                vsb: [0; 16],
                psf: Box::new([0; 4096]),
                psi: [[0; 4]; 16],
                psb: [0; 16],
                clip: [[0; 4]; wgsl::CLIP_PLANES],
                dirty: [true; 2],
                header_dirty: [true; 2],
                bound: Bound::default(),
            },
            encoder: None,
            finishing: Vec::new(),
            finisher: Finisher::for_this_machine(),
            encoder_draws: 0,
            pass: None,
            pass_targets: None,
            batch: 1,
            ring: Vec::with_capacity(RING as usize),
            ring_buffer,
            ring_group,
            uniform_layout,
            up: Vec::new(),
            up_buffer,
            stage: Vec::new(),
            stage_buffer,
            blocks: (0, 0),
            block_fregs: [0; 2],
            sampler_layouts: Map::default(),
            pipelines: Map::default(),
            last_pipeline: None,
            last_group: (Vec::new(), None),
            last_link: None,
            vertex_links: Map::default(),
            binds: PassBinds::default(),
            made_on_draws: 0,
            pipeline_cache,
            last_present: None,
            loading: None,
            shader_bytecode: Map::default(),
            collided: HashSet::new(),
            decls_by_elements: Map::default(),
            list,
            sample_mask: 1 << 1,
            listed_new: false,
            made: Made::default(),
            vs_modules: Map::default(),
            ps_modules: Map::default(),
            samplers: Map::default(),
            groups: Map::default(),
            blit: Map::default(),
            blit_layout: None,
            gamma_resolves: Map::default(),
            depth_parts: Map::default(),
            clears: Map::default(),
            dummy,
            border: features.contains(wgpu::Features::ADDRESS_MODE_CLAMP_TO_BORDER),
            float_filterable: features.contains(wgpu::Features::FLOAT32_FILTERABLE),
            clip_distances: features.contains(wgpu::Features::CLIP_DISTANCES),
            block_centre: [0.0; 2],
            headers: [[0; HEADER]; 2],
            header_centre: [0.0; 2],
            header_atoc: false,
            keeps: Map::default(),
            kept: Map::default(),
            counting: None,
            pass_count: None,
            count_targets: Map::default(),
            count_buffer,
            next_count_slot: 0,
            reduce: Map::default(),
            target_copies: Map::default(),
            destroyed: HashSet::new(),
            next_id: 1,
            presents: 0,
            shots,
            warned: HashSet::new(),
            errors,
            draws: 0,
            skipped: 0,
            answers: None,
            awaiting: Vec::new(),
        };
        d.surface = shown_in;
        assert!(
            FAIL_FOR_TESTS.load(Ordering::Relaxed) & PANIC_FOR_TESTS == 0,
            "a device made to panic (tests)"
        );
        d.make_back_buffers(pp, back_buffers, depth);
        let shown = match (&d.surface, output) {
            (Some(w), _) => format!(
                "on the game's window: {}x{}, {:?}",
                w.config.width, w.config.height, w.config.present_mode
            ),
            (None, Output::Window) => "with no window".to_string(),
            (None, Output::Offscreen) => "offscreen".to_string(),
        };
        eprintln!(
            "NV_D3D9 wgpu: the game's device on {} ({}x{}, format {}, {shown})",
            d.adapter_name, pp.width, pp.height, pp.format,
        );
        Ok(d)
    }

    fn id(&mut self) -> u64 {
        self.next_id += 1;
        self.next_id
    }

    /// Prints once that a call the game made is not carried out.
    pub(crate) fn not_provided(&mut self, what: &str) {
        self.warn(what);
    }

    /// Prints a message once.
    fn warn(&mut self, what: impl Into<String>) {
        let what = what.into();
        if self.warned.insert(what.clone()) {
            eprintln!("NV_D3D9 wgpu: {what}");
        }
    }

    fn make_back_buffers(
        &mut self,
        pp: &PresentParameters,
        back_buffers: &[Handle],
        depth: Handle,
    ) {
        let format = if pp.format == 0 {
            d3dfmt::X8R8G8B8
        } else {
            pp.format
        };
        self.back_buffers = back_buffers.to_vec();
        for &h in back_buffers {
            let s = self.new_surface(pp.width, pp.height, format, false, 1);
            if let Some(s) = s {
                self.objects.insert(h, Obj::Surface(s));
            }
        }
        if let Some(&first) = back_buffers.first() {
            self.st.rt[0] = self.surface_of(first);
        }
        self.auto_depth = depth;
        if depth != 0 && pp.auto_depth {
            if let Some(s) = self.new_surface(pp.width, pp.height, pp.depth_format, true, 1) {
                self.objects.insert(depth, Obj::Surface(s.clone()));
                self.st.ds = Some(s);
            }
        }
        self.st.viewport = Viewport {
            x: 0,
            y: 0,
            w: pp.width,
            h: pp.height,
            min_z: 0.0,
            max_z: 1.0,
        };
        self.st.scissor = [0, 0, pp.width as i32, pp.height as i32];
    }

    fn texture_usage(format: &Format, target: bool) -> wgpu::TextureUsages {
        let mut u = wgpu::TextureUsages::TEXTURE_BINDING
            | wgpu::TextureUsages::COPY_DST
            | wgpu::TextureUsages::COPY_SRC;
        if target || format.depth {
            u |= wgpu::TextureUsages::RENDER_ATTACHMENT;
        }
        u
    }

    #[allow(clippy::too_many_arguments)]
    fn new_texture(
        &mut self,
        width: u32,
        height: u32,
        levels: u32,
        d3d_format: u32,
        cube: bool,
        target: bool,
        samples: u32,
    ) -> Option<Arc<Tex>> {
        let Some(format) = format::format(d3d_format) else {
            self.warn(format!("texture format {d3d_format:#x} is not provided"));
            return None;
        };
        if format.block && (width % 4 != 0 || height % 4 != 0) {
            self.warn(format!(
                "a {width}x{height} block-compressed texture: wgpu wants whole blocks (skipped)"
            ));
            return None;
        }
        // A new sample count may make more listed pipelines ready ([`Device::prepare_listed`]).
        let bit = 1u32 << samples.min(31);
        if self.sample_mask & bit == 0 {
            self.sample_mask |= bit;
            self.listed_new = true;
        }
        let layers = if cube { 6 } else { 1 };
        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: None,
            size: wgpu::Extent3d {
                width: width.max(1),
                height: height.max(1),
                depth_or_array_layers: layers,
            },
            mip_level_count: levels.max(1),
            sample_count: samples,
            dimension: wgpu::TextureDimension::D2,
            format: format.wgpu,
            // A multisampled surface is only drawn into and resolved (`StretchRect`; an 8-bit
            // colour one by a shader that reads its samples).
            usage: if samples > 1 && gamma_resolved(format.wgpu) {
                wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING
            } else if samples > 1 {
                wgpu::TextureUsages::RENDER_ATTACHMENT
            } else {
                Self::texture_usage(&format, target)
            },
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor {
            dimension: Some(if cube {
                wgpu::TextureViewDimension::Cube
            } else {
                wgpu::TextureViewDimension::D2
            }),
            aspect: if format.depth {
                wgpu::TextureAspect::DepthOnly
            } else {
                wgpu::TextureAspect::All
            },
            ..Default::default()
        });
        let id = self.id();
        Some(Arc::new(Tex {
            texture,
            view,
            format,
            width,
            height,
            levels: levels.max(1),
            cube,
            samples,
            id,
            batch: AtomicU64::new(0),
        }))
    }

    fn level_surface(&mut self, tex: &Arc<Tex>, level: u32, layer: u32) -> Arc<Surf> {
        let view = tex.texture.create_view(&wgpu::TextureViewDescriptor {
            dimension: Some(wgpu::TextureViewDimension::D2),
            base_mip_level: level,
            mip_level_count: Some(1),
            base_array_layer: layer,
            array_layer_count: Some(1),
            ..Default::default()
        });
        let id = self.id();
        Arc::new(Surf {
            tex: Some(tex.clone()),
            level,
            layer,
            width: (tex.width >> level).max(1),
            height: (tex.height >> level).max(1),
            format: tex.format,
            samples: tex.samples,
            view: Some(view),
            id,
        })
    }

    fn new_surface(
        &mut self,
        width: u32,
        height: u32,
        d3d: u32,
        depth: bool,
        samples: u32,
    ) -> Option<Arc<Surf>> {
        if d3d == d3dfmt::NULL {
            return None;
        }
        let tex = self.new_texture(width, height, 1, d3d, false, !depth, samples)?;
        Some(self.level_surface(&tex, 0, 0))
    }

    fn surface_of(&self, h: Handle) -> Option<Arc<Surf>> {
        match self.objects.get(&h) {
            Some(Obj::Surface(s)) => Some(s.clone()),
            _ => None,
        }
    }

    fn texture_of(&self, h: Handle) -> Option<Arc<Tex>> {
        match self.objects.get(&h) {
            Some(Obj::Texture(t, _)) => Some(t.clone()),
            _ => None,
        }
    }

    fn buffer_of(&self, h: Handle) -> Option<Arc<Buf>> {
        match self.objects.get(&h) {
            Some(Obj::Buffer(b, _)) => Some(b.clone()),
            _ => None,
        }
    }

    // Resources.

    /// Marks `levels` (of `owner`) as levels whose bytes the backend keeps when `pool` is the
    /// managed or system-memory pool, which the game reads back through `ReadSurface`; kept even
    /// when the backend's own texture could not be made.
    fn keep_levels(&mut self, owner: Handle, levels: &[Handle], pool: u32) {
        if pool != POOL_MANAGED && pool != POOL_SYSTEMMEM {
            return;
        }
        for &h in levels {
            self.kept.insert(h, Vec::new());
        }
        self.keeps.insert(owner, levels.to_vec());
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn create_texture(
        &mut self,
        texture: Handle,
        levels: &[Handle],
        width: u32,
        height: u32,
        usage: u32,
        format: u32,
        cube: bool,
        pool: u32,
    ) {
        self.keep_levels(texture, levels, pool);
        self.st.bound.forget(texture);
        let count = if cube {
            levels.len() as u32 / 6
        } else {
            levels.len() as u32
        };
        let target = usage & 1 != 0;
        let Some(tex) = self.new_texture(width, height, count.max(1), format, cube, target, 1)
        else {
            return;
        };
        let mut surfaces = Vec::with_capacity(levels.len());
        for (k, &h) in levels.iter().enumerate() {
            let (layer, level) = if cube {
                ((k as u32) / count, (k as u32) % count)
            } else {
                (0, k as u32)
            };
            let s = self.level_surface(&tex, level, layer);
            self.objects.insert(h, Obj::Surface(s));
            surfaces.push(h);
        }
        self.objects.insert(texture, Obj::Texture(tex, surfaces));
    }

    pub(crate) fn create_surface(
        &mut self,
        surface: Handle,
        depth: bool,
        width: u32,
        height: u32,
        format: u32,
        multisample: u32,
    ) {
        // `D3DMULTISAMPLE_2_SAMPLES` and up are the sample counts; NONMASKABLE (1) has none here.
        let asked = if multisample >= 2 { multisample } else { 1 };
        let samples = format::format(format).map_or(1, |f| self.sample_count(f.wgpu, asked));
        if samples != asked {
            self.warn(format!(
                "{asked}x multisampling is not supported here for every format the game draws multisampled: {samples}x"
            ));
        }
        if let Some(s) = self.new_surface(width, height, format, depth, samples) {
            self.objects.insert(surface, Obj::Surface(s));
        } else if format != d3dfmt::NULL {
            self.warn(format!("a surface of format {format:#x} is not provided"));
        }
    }

    /// `CreateOffscreenPlainSurface`: a surface of one level; a system-memory one keeps its
    /// bytes here (`GetRenderTargetData`'s destination, read back by `ReadSurface`).
    pub(crate) fn create_offscreen(
        &mut self,
        surface: Handle,
        width: u32,
        height: u32,
        format: u32,
        pool: u32,
    ) {
        let pool = if pool == POOL_SCRATCH {
            POOL_SYSTEMMEM
        } else {
            pool
        };
        self.keep_levels(surface, &[surface], pool);
        self.create_surface(surface, false, width, height, format, 0);
    }

    pub(crate) fn create_buffer(&mut self, buffer: Handle, index: bool, length: u32, format: u32) {
        self.st.bound.forget(buffer);
        let size = u64::from(length.max(4).div_ceil(4) * 4);
        let wgpu_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size,
            usage: if index {
                wgpu::BufferUsages::INDEX
            } else {
                wgpu::BufferUsages::VERTEX
            } | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let b = Arc::new(Buf {
            buffer: wgpu_buffer,
            index32: index && format == 102,
            batch: AtomicU64::new(0),
        });
        self.objects
            .insert(buffer, Obj::Buffer(b, vec![0; size as usize]));
    }

    pub(crate) fn create_shader(&mut self, shader: Handle, tokens: &[u32]) {
        self.st.bound.forget(shader);
        // One shader object for one bytecode: a shader made again (another handle, the same
        // tokens) shares its modules and pipelines, and the list ([`prepare`]) finds it by hash.
        let stable = prepare::bytecode_hash(tokens);
        if let Some(same) = self.shader_bytecode.get(&stable) {
            if let Some(s) = same.iter().find(|s| s.tokens[..] == tokens[..]) {
                let s = s.clone();
                self.objects.insert(shader, Obj::Shader(s));
                return;
            }
            // Two bytecodes, one hash: the list's entries with it are never prepared (a
            // draw still finds its own pipeline by the exact key).
            if self.collided.insert(stable) {
                eprintln!("NV_D3D9 wgpu: two shaders share the bytecode hash {stable:016x}: its listed pipelines are not prepared");
            }
        }
        match sm::parse(tokens) {
            Ok(program) => {
                let sig = wgsl::signature(&program);
                let id = self.id();
                let fregs = wgsl::float_constants_read(&program);
                let varyings = sig.inputs.iter().map(|(_, s)| *s).collect();
                let s = Arc::new(Shader {
                    program,
                    sig,
                    id,
                    fregs,
                    varyings,
                    tokens: tokens.to_vec(),
                    stable,
                });
                self.shader_bytecode
                    .entry(stable)
                    .or_default()
                    .push(s.clone());
                self.objects.insert(shader, Obj::Shader(s));
                self.listed_new = true;
            }
            Err(e) => self.warn(format!("a shader the converter cannot read: {e}")),
        }
    }

    pub(crate) fn create_declaration(&mut self, declaration: Handle, bytes: &[u8]) {
        self.st.bound.forget(declaration);
        let mut elements = Vec::new();
        for e in bytes.chunks_exact(8) {
            if e[0] == 0xFF || e[4] == 17 {
                break;
            }
            elements.push(Element {
                stream: u16::from_le_bytes([e[0], e[1]]),
                offset: u16::from_le_bytes([e[2], e[3]]),
                ty: e[4],
                usage: e[6],
                index: e[7],
            });
        }
        let d = self.decl_of(elements);
        self.objects.insert(declaration, Obj::Decl(d));
    }

    /// One declaration object for one list of elements, whether the game made it (again) or the
    /// list ([`prepare`]) names it.
    fn decl_of(&mut self, elements: Vec<Element>) -> Arc<Decl> {
        if let Some(d) = self.decls_by_elements.get(&elements) {
            return d.clone();
        }
        let id = self.id();
        let d = Arc::new(Decl {
            elements: elements.clone(),
            id,
        });
        self.decls_by_elements.insert(elements, d.clone());
        d
    }

    pub(crate) fn create_query(&mut self, query: Handle, kind: u32) {
        self.objects.insert(
            query,
            Obj::Query(Query {
                kind,
                ..Query::default()
            }),
        );
        if let Some(a) = &self.answers {
            a.made(query, kind == QUERY_OCCLUSION);
        }
    }

    pub(crate) fn destroy(&mut self, object: Handle) {
        self.st.bound.forget(object);
        if let Some(levels) = self.keeps.remove(&object) {
            for h in levels {
                self.kept.remove(&h);
            }
        }
        if self.counting.is_some_and(|(q, _)| q == object) {
            self.end_pass();
            self.counting = None;
        }
        match self.objects.remove(&object) {
            Some(Obj::Texture(t, surfaces)) => {
                if let Some(c) = self.target_copies.remove(&t.id) {
                    self.destroyed.insert(c.id);
                }
                self.destroyed.insert(t.id);
                for s in surfaces {
                    self.objects.remove(&s);
                }
            }
            Some(Obj::Surface(s)) => {
                self.destroyed.insert(s.id);
            }
            _ => {}
        }
    }

    /// The cached bind groups that sample a texture destroyed since the last `Present`, and the depth
    /// stand-ins made for a destroyed depth surface, dropped: they held the texture (its video
    /// memory) until the cache filled up (8,192 groups), so over a long walk the textures of
    /// unloaded cells stayed. Once a frame, so a cell change that destroys hundreds of textures
    /// costs one pass over the cache.
    fn purge_destroyed(&mut self) {
        if self.destroyed.is_empty() {
            return;
        }
        let gone = std::mem::take(&mut self.destroyed);
        let holds = |k: &GroupKey| k.iter().any(|e| gone.contains(&e.2));
        self.groups.retain(|k, _| !holds(k));
        if holds(&self.last_group.0) {
            self.last_group = (Vec::new(), None);
        }
        self.depth_parts.retain(|k, _| !gone.contains(&k.0));
    }

    /// How many bind groups are cached (tests).
    pub fn cached_bind_groups(&self) -> usize {
        self.groups.len()
    }

    /// Sends the frame's commands so far, so a later upload cannot change what they draw.
    fn flush(&mut self) {
        self.end_pass();
        if !self.ring.is_empty() {
            self.queue.write_buffer(&self.ring_buffer, 0, &self.ring);
            self.ring.clear();
        }
        if !self.up.is_empty() {
            self.queue.write_buffer(&self.up_buffer, 0, &self.up);
            self.up.clear();
        }
        if !self.stage.is_empty() {
            self.queue.write_buffer(&self.stage_buffer, 0, &self.stage);
            self.stage.clear();
        }
        // The batch's encoders in order: the earlier ones as their finishing threads finish them,
        // the open one finished here.
        let mut buffers: Vec<wgpu::CommandBuffer> = self
            .finishing
            .drain(..)
            .map(|f| match f {
                Finishing::Ready(b) => b,
                Finishing::Pending(r) => match r.recv() {
                    Ok(Ok(b)) => b,
                    Ok(Err(why)) => panic!("a finishing thread could not finish an encoder: {why}"),
                    Err(_) => panic!("a finishing thread stopped before it finished an encoder"),
                },
            })
            .collect();
        if let Some(e) = self.encoder.take() {
            buffers.push(e.finish());
        }
        if !buffers.is_empty() {
            self.queue.submit(buffers);
        }
        self.encoder_draws = 0;
        self.batch += 1;
        self.st.dirty = [true; 2];
    }

    pub(crate) fn upload_surface(&mut self, surface: Handle, data: &[u8]) {
        // The stream carries whole levels (`UploadSurface`, checked when read); one shorter all
        // the same changes only the bytes it carries, so the rest still reads back.
        if self.kept.contains_key(&surface) {
            let level = self.surface_of(surface).map_or(0, |s| {
                let (row, rows) = format::level_layout(&s.format, s.width, s.height);
                (row * rows) as usize
            });
            if let Some(bytes) = self.kept.get_mut(&surface) {
                if bytes.len() < data.len().max(level) {
                    bytes.resize(data.len().max(level), 0);
                }
                bytes[..data.len()].copy_from_slice(data);
            }
        }
        let Some(s) = self.surface_of(surface) else {
            return;
        };
        let Some(tex) = s.tex.clone() else { return };
        if tex.batch.load(Ordering::Relaxed) == self.batch {
            self.flush();
        }
        let (row, rows) = format::level_layout(&s.format, s.width, s.height);
        if (data.len() as u64) < u64::from(row) * u64::from(rows) {
            self.warn("an upload shorter than its level (skipped)");
            return;
        }
        let bytes = format::convert(&s.format, &data[..(row * rows) as usize]);
        let (row, rows) = format::uploaded_layout(&s.format, s.width, s.height);
        let (w, h) = if s.format.block {
            (s.width.div_ceil(4) * 4, s.height.div_ceil(4) * 4)
        } else {
            (s.width, s.height)
        };
        self.queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &tex.texture,
                mip_level: s.level,
                origin: wgpu::Origin3d {
                    x: 0,
                    y: 0,
                    z: s.layer,
                },
                aspect: wgpu::TextureAspect::All,
            },
            &bytes,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(row),
                rows_per_image: Some(rows),
            },
            wgpu::Extent3d {
                width: w,
                height: h,
                depth_or_array_layers: 1,
            },
        );
    }

    pub(crate) fn upload_buffer(
        &mut self,
        buffer: Handle,
        offset: u32,
        data: &d3d9_stream::Data<'_>,
    ) {
        let Some(Obj::Buffer(b, shadow)) = self.objects.get_mut(&buffer) else {
            return;
        };
        let b = b.clone();
        let len = (data.len() as usize).min(shadow.len().saturating_sub(offset as usize));
        if len == 0 {
            return;
        }
        data.copy_to(0, &mut shadow[offset as usize..offset as usize + len]);
        let start = (offset as usize) & !3;
        let end = ((offset as usize + len).div_ceil(4) * 4).min(shadow.len());
        let in_batch = b.batch.load(Ordering::Relaxed) == self.batch;
        if in_batch && self.stage.len() + (end - start) <= STAGE as usize {
            // The open batch draws from this buffer already: the new bytes go in between, as a
            // copy among the batch's commands from its upload stage (written before the batch
            // runs, as the register ring is), so the draws before read the old bytes and the draws
            // after the new, and the batch is not sent for it (it was: five times a frame in the
            // crowded street, each a submit and a new encoder).
            // The pass ends first (a copy cannot be recorded inside one), then the bytes are staged.
            self.end_pass();
            let Some(Obj::Buffer(_, shadow)) = self.objects.get(&buffer) else {
                return;
            };
            let at = self.stage.len() as u64;
            self.stage.extend_from_slice(&shadow[start..end]);
            let stage = self.stage_buffer.clone();
            self.encoder().copy_buffer_to_buffer(
                &stage,
                at,
                &b.buffer,
                start as u64,
                (end - start) as u64,
            );
            return;
        }
        if in_batch {
            self.flush();
        }
        // Not drawn from by the open batch: written before the batch's commands run.
        if let Some(Obj::Buffer(_, shadow)) = self.objects.get(&buffer) {
            self.queue
                .write_buffer(&b.buffer, start as u64, &shadow[start..end]);
        }
    }

    // State.

    pub(crate) fn set_render_target(&mut self, index: u32, surface: Handle) {
        let s = if surface == 0 {
            None
        } else {
            self.surface_of(surface)
        };
        if surface != 0 && s.is_none() {
            self.warn("a render target without a counterpart");
        }
        if index == 0 {
            // SetRenderTarget(0) sets the viewport and scissor to the whole target.
            if let Some(s) = &s {
                self.st.viewport = Viewport {
                    x: 0,
                    y: 0,
                    w: s.width,
                    h: s.height,
                    min_z: 0.0,
                    max_z: 1.0,
                };
                self.st.scissor = [0, 0, s.width as i32, s.height as i32];
            }
        }
        // The same target again (the viewport and scissor reset above, as Direct3D 9 does) changes
        // nothing a block carries; another may change the samples, and with them the alpha to
        // coverage switch ([`ATOC`]).
        let same = |a: &Option<Arc<Surf>>, b: &Option<Arc<Surf>>| match (a, b) {
            (Some(a), Some(b)) => a.id == b.id,
            (None, None) => true,
            _ => false,
        };
        if self.st.rt.get(index as usize).is_some_and(|r| same(r, &s)) {
            return;
        }
        if let Some(slot) = self.st.rt.get_mut(index as usize) {
            *slot = s;
        }
        self.st.touch(0);
        self.st.touch(1);
    }

    pub(crate) fn set_depth_stencil(&mut self, surface: Handle) {
        let ds = if surface == 0 {
            None
        } else {
            self.surface_of(surface)
        };
        // The same depth surface again changes nothing a block carries.
        if ds.as_ref().map(|d| d.id) == self.st.ds.as_ref().map(|d| d.id) {
            return;
        }
        self.st.ds = ds;
        // The targets' sample count decides the alpha to coverage switch ([`ATOC`]).
        self.st.touch(0);
        self.st.touch(1);
    }

    /// `SetClipPlane(index, plane)`: in clip space, as Direct3D 9 takes them with a vertex
    /// shader ([`wgsl::Clip`]).
    pub(crate) fn set_clip_plane(&mut self, index: u32, plane: [u32; 4]) {
        if let Some(p) = self.st.clip.get_mut(index as usize) {
            *p = plane;
            self.st.touch(0);
        }
    }

    pub(crate) fn set_viewport(&mut self, v: [u32; 6]) {
        self.st.viewport = Viewport {
            x: v[0],
            y: v[1],
            w: v[2],
            h: v[3],
            min_z: f32::from_bits(v[4]),
            max_z: f32::from_bits(v[5]),
        };
        // Nothing to mark: the vertex block carries the viewport's pixel-centre move, which
        // [`Device::write_blocks`] compares at each draw.
    }

    pub(crate) fn set_scissor(&mut self, r: Rect) {
        self.st.scissor = r;
    }

    pub(crate) fn set_render_state(&mut self, state: u32, value: u32) {
        // A state set to the value it has changes nothing (the game sets many again each draw).
        let Some(s) = self.st.rs.get_mut(state as usize) else {
            return;
        };
        if *s == value {
            return;
        }
        *s = value;
        match state as usize {
            // The pixel block's alpha test and alpha to coverage; the vertex block's clip planes.
            rs::ALPHATESTENABLE | rs::ALPHAREF | rs::ALPHAFUNC | RS_ADAPTIVETESS_Y => {
                self.st.touch(1)
            }
            rs::CLIPPLANEENABLE | RS_CLIPPING => self.st.touch(0),
            rs::FOGENABLE if value != 0 => {
                self.warn("D3DRS_FOGENABLE (fixed-function fog) is not provided")
            }
            rs::SRGBWRITEENABLE if value != 0 => self.warn("D3DRS_SRGBWRITEENABLE is not provided"),
            rs::FILLMODE if value != 3 => {
                self.warn("wireframe and point fill modes are not provided")
            }
            _ => {}
        }
    }

    fn sampler_slot(sampler: u32) -> Option<usize> {
        match sampler {
            0..=15 => Some(sampler as usize),
            257..=260 => Some(16 + (sampler - 257) as usize),
            _ => None,
        }
    }

    pub(crate) fn set_sampler_state(&mut self, sampler: u32, kind: u32, value: u32) {
        if let Some(slot) = Self::sampler_slot(sampler) {
            let Some(s) = self.st.samplers[slot].get_mut(kind as usize) else {
                return;
            };
            if *s == value {
                return;
            }
            *s = value;
            // Of the sampler states only the LOD bias is in the stage's block; the rest go into
            // the draw's sampler, made from them at the draw.
            if kind as usize == state::ss::MIPMAPLODBIAS {
                self.st.touch(usize::from(slot < 16));
            }
            if kind as usize == state::ss::SRGBTEXTURE && value != 0 {
                self.warn("D3DSAMP_SRGBTEXTURE is not provided yet");
            }
        }
    }

    pub(crate) fn set_texture(&mut self, sampler: u32, texture: Handle) {
        if let Some(slot) = Self::sampler_slot(sampler) {
            // The texture bound there already (the game binds many again each draw): nothing
            // to look up ([`State::bound`]).
            if self.st.bound.textures[slot] == texture {
                return;
            }
            let t = if texture == 0 {
                None
            } else {
                self.texture_of(texture)
            };
            // The stage's block carries each sampler's fill mode (its texture's format).
            let fix = |t: &Option<Arc<Tex>>| t.as_ref().map_or(0, |t| t.format.fix);
            if fix(&t) != fix(&self.st.textures[slot]) {
                self.st.touch(usize::from(slot < 16));
            }
            self.st.textures[slot] = t;
            self.st.bound.textures[slot] = texture;
        }
    }

    pub(crate) fn set_shader(&mut self, pixel: bool, shader: Handle) {
        // The shader bound there already: nothing to look up ([`State::bound`]).
        if self.st.bound.shaders[usize::from(pixel)] == shader {
            return;
        }
        self.st.bound.shaders[usize::from(pixel)] = shader;
        let s = match self.objects.get(&shader) {
            Some(Obj::Shader(s)) => Some(s.clone()),
            _ => None,
        };
        if pixel {
            self.st.ps = s;
        } else {
            self.st.vs = s;
        }
    }

    pub(crate) fn set_declaration(&mut self, declaration: Handle) {
        if self.st.bound.declaration == declaration {
            return;
        }
        self.st.bound.declaration = declaration;
        self.st.decl = match self.objects.get(&declaration) {
            Some(Obj::Decl(d)) => Some(d.clone()),
            _ => None,
        };
    }

    /// `Set*ShaderConstant*` (`kind`: vertex float, int, bool, then the same for the pixel stage)
    /// of `count` registers from `start`, from the command's bytes. Float registers are copied
    /// as bytes (a draw's block carries them so); registers past the file's end are not kept.
    pub(crate) fn set_constants(
        &mut self,
        kind: u8,
        start: u32,
        count: u32,
        data: &d3d9_stream::Data<'_>,
    ) {
        let st = &mut self.st;
        let s = start as usize;
        let stage = usize::from(kind >= 3);
        if kind == 0 || kind == 3 {
            let regs = if kind == 0 { &mut st.vsf } else { &mut st.psf };
            let n = (count as usize)
                .min(data.len() as usize / 16)
                .min(256usize.saturating_sub(s));
            if n > 0 {
                data.copy_to(0, &mut regs[16 * s..16 * (s + n)]);
            }
            st.dirty[stage] = true;
            return;
        }
        let words = data.to_words();
        match kind {
            1 | 4 => {
                let regs = if kind == 1 { &mut st.vsi } else { &mut st.psi };
                for (k, r) in words.chunks_exact(4).take(count as usize).enumerate() {
                    if let Some(slot) = regs.get_mut(s + k) {
                        slot.copy_from_slice(r);
                    }
                }
            }
            _ => {
                let regs = if kind == 2 { &mut st.vsb } else { &mut st.psb };
                for (k, &v) in words.iter().take(count as usize).enumerate() {
                    if let Some(slot) = regs.get_mut(s + k) {
                        *slot = v;
                    }
                }
            }
        }
        st.touch(stage);
    }

    pub(crate) fn set_stream(&mut self, stream: u32, buffer: Handle, offset: u32, stride: u32) {
        let Some(&(h, o, s)) = self.st.bound.streams.get(stream as usize) else {
            return;
        };
        if (h, o, s) == (buffer, offset, stride) {
            return;
        }
        self.st.bound.streams[stream as usize] = (buffer, offset, stride);
        let b = if buffer == 0 {
            None
        } else {
            self.buffer_of(buffer)
        };
        if let Some(s) = self.st.streams.get_mut(stream as usize) {
            *s = Stream {
                buffer: b,
                offset,
                stride,
            };
        }
    }

    pub(crate) fn set_indices(&mut self, buffer: Handle) {
        if self.st.bound.indices == buffer {
            return;
        }
        self.st.bound.indices = buffer;
        self.st.indices = if buffer == 0 {
            None
        } else {
            self.buffer_of(buffer)
        };
    }

    // Passes.

    fn encoder(&mut self) -> &mut wgpu::CommandEncoder {
        self.encoder.get_or_insert_with(|| {
            self.device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None })
        })
    }

    /// The open encoder (its pass ended) to a finishing thread, its command buffer sent with the
    /// batch after the encoders before it ([`Device::flush`]).
    fn send_encoder(&mut self) {
        self.end_pass();
        if let Some(e) = self.encoder.take() {
            self.finishing.push(match &self.finisher {
                Some(f) => f.send(e),
                None => Finishing::Ready(e.finish()),
            });
        }
        self.encoder_draws = 0;
    }

    fn end_pass(&mut self) {
        self.pass = None;
        self.pass_targets = None;
        if let Some(c) = self.pass_count.take() {
            self.reduce_count(&c);
        }
    }

    /// Whether a level or a face of `t` is one of the current colour targets.
    fn is_colour_target(&self, t: &Tex) -> bool {
        self.st
            .rt
            .iter()
            .flatten()
            .any(|s| s.tex.as_ref().is_some_and(|rt| rt.id == t.id))
    }

    /// A copy of `t` (every level and face) as it is now, for a draw that samples `t` while it
    /// draws into it: the open pass ends, so the copy holds every draw before this one, and the
    /// next pass loads the targets again. One scratch texture is kept for each such texture.
    fn copy_for_sampling(&mut self, t: &Arc<Tex>) -> Option<Arc<Tex>> {
        self.end_pass();
        let copy = match self.target_copies.get(&t.id) {
            Some(c) => c.clone(),
            None => {
                let c = self.new_texture(
                    t.width,
                    t.height,
                    t.levels,
                    t.format.d3d,
                    t.cube,
                    false,
                    t.samples,
                )?;
                self.target_copies.insert(t.id, c.clone());
                c
            }
        };
        let layers = if t.cube { 6 } else { 1 };
        for level in 0..t.levels {
            let size = wgpu::Extent3d {
                width: (t.width >> level).max(1),
                height: (t.height >> level).max(1),
                depth_or_array_layers: layers,
            };
            self.encoder().copy_texture_to_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: &t.texture,
                    mip_level: level,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                wgpu::TexelCopyTextureInfo {
                    texture: &copy.texture,
                    mip_level: level,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                size,
            );
        }
        for x in [t, &copy] {
            x.batch.store(self.batch, Ordering::Relaxed);
        }
        Some(copy)
    }

    fn current_targets(&mut self) -> Targets {
        Targets {
            colours: std::array::from_fn(|i| self.st.rt[i].as_ref().map(|s| s.id)),
            depth: self.usable_depth().map(|s| s.id),
            counting: self.counting.is_some(),
        }
    }

    /// The size and sample count of what the current targets draw into.
    fn target_shape(&mut self) -> (u32, u32, u32) {
        match self.st.rt.iter().flatten().next() {
            Some(s) => (s.width, s.height, s.samples),
            None => self
                .usable_depth()
                .map_or((1, 1, 1), |d| (d.width, d.height, d.samples)),
        }
    }

    /// The clip code a draw takes on a target of `samples`: none without a plane on; the
    /// hardware's clip distances where the adapter has them; else the pixel shader's discard,
    /// per sample on a multisampled target, which is where the hardware cuts.
    fn clip_link(&self, samples: u32) -> wgsl::Clip {
        let on = self.st.rs[RS_CLIPPING] != 0
            && self.st.rs[rs::CLIPPLANEENABLE] & ((1 << wgsl::CLIP_PLANES) - 1) != 0;
        if !on {
            wgsl::Clip::Off
        } else if self.clip_distances {
            wgsl::Clip::Distances
        } else {
            wgsl::Clip::Discard {
                per_sample: samples > 1,
            }
        }
    }

    /// Whether NVIDIA's alpha to coverage stands in for the alpha test ([`ATOC`]).
    fn atoc(&mut self) -> bool {
        self.st.rs[RS_ADAPTIVETESS_Y] == ATOC
            && self.st.rs[rs::ALPHATESTENABLE] != 0
            && self.target_shape().2 > 1
    }

    /// Whether the device can make `format` with `samples` per pixel: what the adapter reports
    /// where the device has its formats' own features (2x and 8x need them), else what WebGPU
    /// guarantees.
    fn samples_supported(&self, format: wgpu::TextureFormat, samples: u32) -> bool {
        let features = if self
            .device
            .features()
            .contains(wgpu::Features::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES)
        {
            self.adapter.get_texture_format_features(format)
        } else {
            format.guaranteed_format_features(self.device.features())
        };
        features.flags.sample_count_supported(samples)
    }

    /// The sample count a surface of `format` asked with `asked` samples gets: the largest count
    /// at or below it that `format` and every format of [`MULTISAMPLED_FORMATS`] support, and the
    /// occlusion count's 16-bit target. A render target and its depth surface are made apart
    /// with the same request, so both get the same count and draw together (a count for each
    /// format alone left an 8x colour target with a 4x depth surface: every draw without depth);
    /// and the occlusion count's target can always join their passes ([`Device::count_format`]).
    fn sample_count(&self, format: wgpu::TextureFormat, asked: u32) -> u32 {
        common_sample_count(format, asked, |f, c| self.samples_supported(f, c))
    }

    /// The occlusion count's target format for `samples` ([`COUNT_FORMAT`]): 32-bit where the
    /// adapter blends it at that count, else 16-bit, which every multisampled surface's count
    /// supports ([`Device::sample_count`] chose the count so).
    fn count_format(&self, samples: u32) -> wgpu::TextureFormat {
        if self
            .device
            .features()
            .contains(wgpu::Features::FLOAT32_BLENDABLE)
            && self.samples_supported(COUNT_FORMAT, samples)
        {
            COUNT_FORMAT
        } else {
            COUNT_FORMAT_16
        }
    }

    /// The occlusion count's target of `(width, height, samples)`, made once.
    fn count_target(&mut self, width: u32, height: u32, samples: u32) -> wgpu::TextureView {
        if let Some(v) = self.count_targets.get(&(width, height, samples)) {
            return v.clone();
        }
        let t = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("occlusion count"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: samples,
            dimension: wgpu::TextureDimension::D2,
            format: self.count_format(samples),
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let v = t.create_view(&Default::default());
        self.count_targets
            .insert((width, height, samples), v.clone());
        v
    }

    /// Adds the samples a pass's count target holds into its query's slot (a compute pass after
    /// the render pass, in the same encoder).
    fn reduce_count(&mut self, c: &CountSegment) {
        let multisampled = c.samples > 1;
        if !self.reduce.contains_key(&multisampled) {
            let texture = if multisampled {
                "texture_multisampled_2d<f32>"
            } else {
                "texture_2d<f32>"
            };
            let source = format!(
                "struct P {{ slot: u32, samples: u32, w: u32, h: u32 }}
@group(0) @binding(0) var t: {texture};
@group(0) @binding(1) var<storage, read_write> counts: array<atomic<u32>>;
@group(0) @binding(2) var<uniform> p: P;
var<workgroup> sum: atomic<u32>;
@compute @workgroup_size(16, 16)
fn main(@builtin(global_invocation_id) g: vec3<u32>, @builtin(local_invocation_index) i: u32) {{
    if (i == 0u) {{ atomicStore(&sum, 0u); }}
    workgroupBarrier();
    if (g.x < p.w && g.y < p.h) {{
        var n = 0u;
        for (var s = 0u; s < p.samples; s++) {{
            n += u32(textureLoad(t, vec2<i32>(g.xy), {sample}).x + 0.5);
        }}
        if (n != 0u) {{ atomicAdd(&sum, n); }}
    }}
    workgroupBarrier();
    if (i == 0u) {{
        let total = atomicLoad(&sum);
        if (total != 0u) {{ atomicAdd(&counts[p.slot], total); }}
    }}
}}
",
                sample = if multisampled { "i32(s)" } else { "0" },
            );
            let module = self
                .device
                .create_shader_module(wgpu::ShaderModuleDescriptor {
                    label: Some("occlusion count"),
                    source: wgpu::ShaderSource::Wgsl(source.into()),
                });
            let layout = self
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("occlusion count"),
                    entries: &[
                        wgpu::BindGroupLayoutEntry {
                            binding: 0,
                            visibility: wgpu::ShaderStages::COMPUTE,
                            ty: wgpu::BindingType::Texture {
                                sample_type: wgpu::TextureSampleType::Float { filterable: false },
                                view_dimension: wgpu::TextureViewDimension::D2,
                                multisampled,
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 1,
                            visibility: wgpu::ShaderStages::COMPUTE,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Storage { read_only: false },
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 2,
                            visibility: wgpu::ShaderStages::COMPUTE,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Uniform,
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                    ],
                });
            let pl = self
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: None,
                    bind_group_layouts: &[Some(&layout)],
                    immediate_size: 0,
                });
            let pipeline = self
                .device
                .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                    label: Some("occlusion count"),
                    layout: Some(&pl),
                    module: &module,
                    entry_point: Some("main"),
                    compilation_options: Default::default(),
                    cache: None,
                });
            self.reduce.insert(multisampled, (pipeline, layout));
        }
        let (pipeline, layout) = self.reduce[&multisampled].clone();
        let words = [c.slot, c.samples, c.width, c.height];
        let ub = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: 16,
            usage: wgpu::BufferUsages::UNIFORM,
            mapped_at_creation: true,
        });
        if let Ok(mut mapped) = ub.slice(..).get_mapped_range_mut() {
            let bytes: Vec<u8> = words.iter().flat_map(|w| w.to_le_bytes()).collect();
            mapped.copy_from_slice(&bytes);
        }
        ub.unmap();
        let group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&c.view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: self.count_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: ub.as_entire_binding(),
                },
            ],
        });
        let mut pass = self
            .encoder()
            .begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("occlusion count"),
                timestamp_writes: None,
            });
        pass.set_pipeline(&pipeline);
        pass.set_bind_group(0, &group, &[]);
        pass.dispatch_workgroups(c.width.div_ceil(16), c.height.div_ceil(16), 1);
    }

    /// The depth surface when it can be used with the colour targets (wgpu wants one size).
    /// The depth surface to draw with: the bound one, or when it is larger than the colour
    /// target (Direct3D 9 then uses its top-left part; wgpu wants one size) a stand-in of the
    /// target's size kept for that pair, whose contents are its own (the game clears it). None
    /// when the sample counts differ (Direct3D 9 refuses to draw so).
    fn usable_depth(&mut self) -> Option<Arc<Surf>> {
        let ds = self.st.ds.clone()?;
        let Some(rt) = self.st.rt.iter().flatten().next().cloned() else {
            return Some(ds);
        };
        if rt.samples != ds.samples {
            return None;
        }
        if rt.width == ds.width && rt.height == ds.height {
            return Some(ds);
        }
        if rt.width > ds.width || rt.height > ds.height {
            return None;
        }
        let key = (ds.id, rt.width, rt.height);
        if let Some(s) = self.depth_parts.get(&key) {
            return Some(s.clone());
        }
        let tex = self.new_texture(
            rt.width,
            rt.height,
            1,
            ds.format.d3d,
            false,
            false,
            ds.samples,
        )?;
        let s = self.level_surface(&tex, 0, 0);
        self.depth_parts.insert(key, s.clone());
        Some(s)
    }

    /// Opens a render pass on the current targets (loading what they hold), or clears them as it
    /// opens (`clear`: colour, depth, stencil).
    fn begin_pass(&mut self, clear: Option<(wgpu::Color, Option<f32>, Option<u32>, bool)>) {
        let targets = self.current_targets();
        if clear.is_none() && self.pass.is_some() && self.pass_targets.as_ref() == Some(&targets) {
            return;
        }
        self.end_pass();
        if let (Some(ds), Some(rt)) = (&self.st.ds, self.st.rt.iter().flatten().next()) {
            if ds.width < rt.width || ds.height < rt.height || ds.samples != rt.samples {
                let what = format!(
                    "a {}x{} depth surface ({} samples) with a {}x{} render target ({} samples): drawn without depth",
                    ds.width, ds.height, ds.samples, rt.width, rt.height, rt.samples
                );
                self.warn(what);
            }
        }
        // The views and their load operations, owned here so the encoder can be borrowed.
        let colour_views: Vec<Option<(wgpu::TextureView, wgpu::LoadOp<wgpu::Color>)>> = self
            .st
            .rt
            .iter()
            .map(|rt| {
                rt.as_ref().and_then(|s| {
                    s.view.clone().map(|view| {
                        let load = match clear {
                            Some((c, _, _, true)) => wgpu::LoadOp::Clear(if s.format.opaque {
                                wgpu::Color { a: 1.0, ..c }
                            } else {
                                c
                            }),
                            _ => wgpu::LoadOp::Load,
                        };
                        (view, load)
                    })
                })
            })
            .collect();
        let colours: Vec<Option<wgpu::RenderPassColorAttachment>> = colour_views
            .iter()
            .map(|c| {
                c.as_ref()
                    .map(|(view, load)| wgpu::RenderPassColorAttachment {
                        view,
                        depth_slice: None,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: *load,
                            store: wgpu::StoreOp::Store,
                        },
                    })
            })
            .collect();
        // As many attachments as the pipelines have targets: up to the last bound one, then the
        // occlusion count's while a query counts.
        let mut colours = colours;
        colours.truncate(
            colours
                .iter()
                .rposition(|c| c.is_some())
                .map_or(0, |i| i + 1),
        );
        let count = match self.counting {
            Some((_, slot)) => {
                let (width, height, samples) = self.target_shape();
                let view = self.count_target(width, height, samples);
                Some(CountSegment {
                    view,
                    samples,
                    width,
                    height,
                    slot,
                })
            }
            None => None,
        };
        if let Some(c) = &count {
            colours.resize_with(wgsl::COUNT_TARGET as usize, || None);
            colours.push(Some(wgpu::RenderPassColorAttachment {
                view: &c.view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                    store: wgpu::StoreOp::Store,
                },
            }));
        }
        let colours = &colours[..];
        let depth = self.usable_depth();
        let depth_view = depth.as_ref().and_then(|s| s.view.clone());
        let depth_attachment = depth.as_ref().zip(depth_view.as_ref()).map(|(s, view)| {
            let stencil = s.format.wgpu.has_stencil_aspect();
            wgpu::RenderPassDepthStencilAttachment {
                view,
                depth_ops: Some(wgpu::Operations {
                    load: match clear {
                        Some((_, Some(z), _, _)) => wgpu::LoadOp::Clear(z),
                        _ => wgpu::LoadOp::Load,
                    },
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: stencil.then_some(wgpu::Operations {
                    load: match clear {
                        Some((_, _, Some(v), _)) => wgpu::LoadOp::Clear(v),
                        _ => wgpu::LoadOp::Load,
                    },
                    store: wgpu::StoreOp::Store,
                }),
            }
        });
        let pass = self
            .encoder()
            .begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: colours,
                depth_stencil_attachment: depth_attachment,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            })
            .forget_lifetime();
        self.pass = Some(pass);
        self.binds = PassBinds::default();
        self.pass_targets = Some(targets);
        self.pass_count = count;
        // The surfaces' textures are in use by this batch.
        for s in self.st.rt.iter().flatten() {
            if let Some(t) = &s.tex {
                t.batch.store(self.batch, Ordering::Relaxed);
            }
        }
        if let Some(t) = depth.as_ref().and_then(|d| d.tex.as_ref()) {
            t.batch.store(self.batch, Ordering::Relaxed);
        }
    }

    pub(crate) fn clear(
        &mut self,
        rects: Option<&[i32]>,
        flags: u32,
        colour: u32,
        z: f32,
        stencil: u32,
    ) {
        let target = self.st.rt.iter().flatten().next().cloned();
        let (w, h) = match (&target, &self.st.ds) {
            (Some(t), _) => (t.width, t.height),
            (None, Some(d)) => (d.width, d.height),
            _ => return,
        };
        let v = self.st.viewport;
        let scissored =
            self.st.rs[rs::SCISSORTESTENABLE] != 0 && self.st.scissor != [0, 0, w as i32, h as i32];
        let whole = v.x == 0 && v.y == 0 && v.w >= w && v.h >= h && !scissored;
        let rects_whole = match rects {
            None => true,
            Some(r) => r
                .chunks_exact(4)
                .any(|r| r[0] <= 0 && r[1] <= 0 && r[2] >= w as i32 && r[3] >= h as i32),
        };
        let c = wgpu::Color {
            r: f64::from((colour >> 16) & 0xFF) / 255.0,
            g: f64::from((colour >> 8) & 0xFF) / 255.0,
            b: f64::from(colour & 0xFF) / 255.0,
            a: f64::from(colour >> 24) / 255.0,
        };
        if !(whole && rects_whole) {
            // The viewport (and the scissor rectangle when on), with each rectangle given.
            let mut area = [
                v.x as i32,
                v.y as i32,
                (v.x + v.w).min(w) as i32,
                (v.y + v.h).min(h) as i32,
            ];
            if self.st.rs[rs::SCISSORTESTENABLE] != 0 {
                let s = self.st.scissor;
                area = [
                    area[0].max(s[0]),
                    area[1].max(s[1]),
                    area[2].min(s[2]),
                    area[3].min(s[3]),
                ];
            }
            let parts: Vec<[i32; 4]> = match rects {
                None => vec![area],
                Some(r) => r
                    .chunks_exact(4)
                    .map(|r| {
                        [
                            area[0].max(r[0]),
                            area[1].max(r[1]),
                            area[2].min(r[2]),
                            area[3].min(r[3]),
                        ]
                    })
                    .collect(),
            };
            self.clear_draw(&parts, flags, c, z.clamp(0.0, 1.0), stencil & 0xFF);
            return;
        }
        self.begin_pass(Some((
            c,
            (flags & 2 != 0).then_some(z.clamp(0.0, 1.0)),
            (flags & 4 != 0).then_some(stencil & 0xFF),
            flags & 1 != 0,
        )));
    }

    /// A clear of rectangles of the targets: a draw writing the colour, depth and stencil the
    /// flags name, cut to each rectangle by the scissor.
    fn clear_draw(&mut self, parts: &[[i32; 4]], flags: u32, c: wgpu::Color, z: f32, stencil: u32) {
        let colours: [Option<(wgpu::TextureFormat, bool)>; 4] = std::array::from_fn(|i| {
            self.st.rt[i]
                .as_ref()
                .map(|s| (s.format.wgpu, s.format.opaque))
        });
        let depth_surface = self.usable_depth();
        let depth = depth_surface.as_ref().map(|d| d.format.wgpu);
        let (tw, th, samples) = self.target_shape();
        let count = self.counting.is_some();
        let key = (
            colours,
            depth,
            (flags & 7) | (samples << 8) | (u32::from(count) << 16),
        );
        if !self.clears.contains_key(&key) {
            let n = colours
                .iter()
                .rposition(|c| c.is_some())
                .map_or(0, |i| i + 1);
            let mut outputs = String::new();
            let mut values = Vec::new();
            for i in 0..n {
                let _ = std::fmt::Write::write_fmt(
                    &mut outputs,
                    format_args!("@location({i}) c{i}: vec4<f32>, "),
                );
                values.push("k.colour");
            }
            let fs = if n == 0 {
                "@fragment fn fs() {}".to_string()
            } else {
                format!(
                    "struct O {{ {outputs} }}\n@fragment fn fs() -> O {{ return O({}); }}",
                    values.join(", ")
                )
            };
            let source = format!(
                "struct K {{ colour: vec4<f32>, z: vec4<f32> }}\n@group(0) @binding(0) var<uniform> k: K;\n@vertex fn vs(@builtin(vertex_index) i: u32) -> @builtin(position) vec4<f32> {{\n    let p = vec2<f32>(f32((i << 1u) & 2u), f32(i & 2u));\n    return vec4<f32>(p * 2.0 - 1.0, k.z.x, 1.0);\n}}\n{fs}\n"
            );
            let module = self
                .device
                .create_shader_module(wgpu::ShaderModuleDescriptor {
                    label: Some("clear"),
                    source: wgpu::ShaderSource::Wgsl(source.into()),
                });
            let layout = self
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("clear"),
                    entries: &[wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    }],
                });
            let pl = self
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: None,
                    bind_group_layouts: &[Some(&layout)],
                    immediate_size: 0,
                });
            let mut targets: Vec<Option<wgpu::ColorTargetState>> = colours[..n]
                .iter()
                .map(|c| {
                    c.map(|(format, opaque)| wgpu::ColorTargetState {
                        format,
                        blend: None,
                        write_mask: if flags & 1 != 0 {
                            state::write_mask(0xF, opaque)
                        } else {
                            wgpu::ColorWrites::empty()
                        },
                    })
                })
                .collect();
            // A clear adds nothing to an occlusion count.
            if count {
                targets.resize(wgsl::COUNT_TARGET as usize, None);
                targets.push(Some(wgpu::ColorTargetState {
                    format: self.count_format(samples),
                    blend: None,
                    write_mask: wgpu::ColorWrites::empty(),
                }));
            }
            let stencil_face = wgpu::StencilFaceState {
                compare: wgpu::CompareFunction::Always,
                fail_op: wgpu::StencilOperation::Replace,
                depth_fail_op: wgpu::StencilOperation::Replace,
                pass_op: wgpu::StencilOperation::Replace,
            };
            let pipeline = self
                .device
                .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                    label: Some("clear"),
                    layout: Some(&pl),
                    vertex: wgpu::VertexState {
                        module: &module,
                        entry_point: Some("vs"),
                        compilation_options: Default::default(),
                        buffers: &[],
                    },
                    primitive: wgpu::PrimitiveState::default(),
                    depth_stencil: depth.map(|format| wgpu::DepthStencilState {
                        format,
                        depth_write_enabled: Some(flags & 2 != 0),
                        depth_compare: Some(wgpu::CompareFunction::Always),
                        stencil: if flags & 4 != 0 && format.has_stencil_aspect() {
                            wgpu::StencilState {
                                front: stencil_face,
                                back: stencil_face,
                                read_mask: 0xFF,
                                write_mask: 0xFF,
                            }
                        } else {
                            wgpu::StencilState::default()
                        },
                        bias: wgpu::DepthBiasState::default(),
                    }),
                    multisample: wgpu::MultisampleState {
                        count: samples,
                        ..Default::default()
                    },
                    fragment: Some(wgpu::FragmentState {
                        module: &module,
                        entry_point: Some("fs"),
                        compilation_options: Default::default(),
                        targets: &targets,
                    }),
                    multiview_mask: None,
                    cache: None,
                });
            self.clears.insert(key, (pipeline, layout));
        }
        let (pipeline, layout) = self.clears[&key].clone();
        let values: [f32; 8] = [
            c.r as f32, c.g as f32, c.b as f32, c.a as f32, z, 0.0, 0.0, 0.0,
        ];
        let ub = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: 32,
            usage: wgpu::BufferUsages::UNIFORM,
            mapped_at_creation: true,
        });
        if let Ok(mut mapped) = ub.slice(..).get_mapped_range_mut() {
            let bytes: Vec<u8> = values.iter().flat_map(|v| v.to_le_bytes()).collect();
            mapped.copy_from_slice(&bytes);
        }
        ub.unmap();
        let group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: ub.as_entire_binding(),
            }],
        });
        self.begin_pass(None);
        // The clear's own pipeline, group, viewport and scissor: the next draw sets its own.
        self.binds = PassBinds::default();
        let pass = self.pass.as_mut().expect("a pass is open");
        pass.set_pipeline(&pipeline);
        pass.set_bind_group(0, &group, &[]);
        pass.set_stencil_reference(stencil);
        // The whole target, depth range 0 to 1: the pass keeps the last draw's viewport, whose
        // depth range would move the cleared depth and whose area may not cover the rectangles.
        pass.set_viewport(0.0, 0.0, tw as f32, th as f32, 0.0, 1.0);
        for p in parts {
            if p[2] <= p[0] || p[3] <= p[1] || p[0] < 0 || p[1] < 0 {
                continue;
            }
            pass.set_scissor_rect(
                p[0] as u32,
                p[1] as u32,
                (p[2] - p[0]) as u32,
                (p[3] - p[1]) as u32,
            );
            pass.draw(0..3, 0..1);
        }
    }

    // Draws.

    fn input_of(ty: u8) -> Option<(wgpu::VertexFormat, wgsl::Input)> {
        use wgpu::VertexFormat as V;
        use wgsl::Input as I;
        Some(match ty {
            0 => (V::Float32, I::Float(1)),
            1 => (V::Float32x2, I::Float(2)),
            2 => (V::Float32x3, I::Float(3)),
            3 => (V::Float32x4, I::Float(4)),
            4 => (V::Unorm8x4Bgra, I::Float(4)),
            5 => (V::Uint8x4, I::Uint4),
            6 => (V::Sint16x2, I::Sint(2)),
            7 => (V::Sint16x4, I::Sint(4)),
            8 => (V::Unorm8x4, I::Float(4)),
            9 => (V::Snorm16x2, I::Float(2)),
            10 => (V::Snorm16x4, I::Float(4)),
            11 => (V::Unorm16x2, I::Float(2)),
            12 => (V::Unorm16x4, I::Float(4)),
            15 => (V::Float16x2, I::Float(2)),
            16 => (V::Float16x4, I::Float(4)),
            _ => return None,
        })
    }

    fn sampler_layout(&mut self, key: &[(u32, u8, bool)]) -> wgpu::PipelineLayout {
        if let Some((_, p)) = self.sampler_layouts.get(key) {
            return p.clone();
        }
        let mut entries = Vec::new();
        for &(binding, dim, filterable) in key {
            entries.push(wgpu::BindGroupLayoutEntry {
                binding,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable },
                    view_dimension: match dim {
                        1 => wgpu::TextureViewDimension::Cube,
                        2 => wgpu::TextureViewDimension::D3,
                        _ => wgpu::TextureViewDimension::D2,
                    },
                    multisampled: false,
                },
                count: None,
            });
            entries.push(wgpu::BindGroupLayoutEntry {
                binding: binding + 1,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Sampler(if filterable {
                    wgpu::SamplerBindingType::Filtering
                } else {
                    wgpu::SamplerBindingType::NonFiltering
                }),
                count: None,
            });
        }
        let layout = self
            .device
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: None,
                entries: &entries,
            });
        let pipeline_layout = self
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: None,
                bind_group_layouts: &[Some(&self.uniform_layout), Some(&layout)],
                immediate_size: 0,
            });
        self.sampler_layouts
            .insert(key.to_vec(), (layout, pipeline_layout.clone()));
        pipeline_layout
    }

    fn filterable(&self, t: Option<&Arc<Tex>>) -> bool {
        match t {
            None => true,
            Some(t) => {
                !t.format.depth
                    && (self.float_filterable
                        || !matches!(
                            t.format.wgpu,
                            wgpu::TextureFormat::R32Float
                                | wgpu::TextureFormat::Rg32Float
                                | wgpu::TextureFormat::Rgba32Float
                        ))
            }
        }
    }

    /// The samplers both shaders use, as (binding, dimension, filterable) and the device sampler
    /// slot.
    fn sampler_bindings(&self, vs: &Shader, ps: &Shader) -> Bindings {
        let mut out = Bindings::default();
        for (stage, shader) in [(Stage::Vertex, vs), (Stage::Pixel, ps)] {
            for &(n, dim) in &shader.sig.samplers {
                let slot = match stage {
                    Stage::Pixel => n as usize,
                    Stage::Vertex => 16 + n as usize,
                };
                if slot >= 20 {
                    continue;
                }
                let d = match dim {
                    wgsl::Dim::D2 => 0,
                    wgsl::Dim::Cube => 1,
                    wgsl::Dim::D3 => 2,
                };
                let filterable = self.filterable(self.st.textures[slot].as_ref());
                if out.n < out.b.len() {
                    out.b[out.n] = ((wgsl::texture_binding(stage, n), d, filterable), slot);
                    out.n += 1;
                }
            }
        }
        out
    }

    /// The vertex inputs of `vs` read from `decl`'s elements: each shader input from the
    /// declaration's element of its usage, the attributes by stream. Made once for each pair.
    fn vertex_link(&mut self, vs: &Shader, decl: &Decl) -> Arc<VertexLink> {
        let key = (vs.id, decl.id);
        if let Some((k, l)) = &self.last_link {
            if *k == key {
                return l.clone();
            }
        }
        let link = match self.vertex_links.get(&key) {
            Some(l) => l.clone(),
            None => {
                let mut inputs = vec![wgsl::Input::Missing; 16];
                let mut attributes: [Vec<wgpu::VertexAttribute>; 8] = Default::default();
                for (r, sem) in &vs.sig.inputs {
                    if r.n >= 16 {
                        continue;
                    }
                    let Some(e) = decl
                        .elements
                        .iter()
                        .find(|e| e.usage == sem.usage && e.index == sem.index)
                    else {
                        continue;
                    };
                    let Some((format, input)) = Self::input_of(e.ty) else {
                        self.warn(format!("vertex element type {} is not provided", e.ty));
                        continue;
                    };
                    if e.stream >= 8 {
                        continue;
                    }
                    inputs[r.n as usize] = input;
                    attributes[e.stream as usize].push(wgpu::VertexAttribute {
                        format,
                        offset: u64::from(e.offset),
                        shader_location: r.n,
                    });
                }
                if self.vertex_links.len() >= 8192 {
                    self.vertex_links.clear();
                }
                let l = Arc::new(VertexLink { inputs, attributes });
                self.vertex_links.insert(key, l.clone());
                l
            }
        };
        self.last_link = Some((key, link.clone()));
        link
    }

    fn topology(kind: u32) -> Option<wgpu::PrimitiveTopology> {
        use wgpu::PrimitiveTopology as P;
        Some(match kind {
            1 => P::PointList,
            2 => P::LineList,
            3 => P::LineStrip,
            4 => P::TriangleList,
            5 => P::TriangleStrip,
            _ => return None,
        })
    }

    /// Vertices a draw of `count` primitives reads.
    fn vertex_count(kind: u32, count: u32) -> u32 {
        match kind {
            1 => count,
            2 => count * 2,
            3 => count + 1,
            4 => count * 3,
            5 | 6 => count + 2,
            _ => 0,
        }
    }

    /// The uniform blocks of the next draw; `centre`: the vertex shaders' half-pixel move in clip
    /// space per unit of `w` (`1 / width`, `-1 / height` of the draw's viewport); `fregs`: the
    /// float constants the vertex and pixel shader read ([`Shader::fregs`]). Each stage's block is
    /// written again only when what it carries changed (or its shader reads constants beyond the
    /// ones it carries), and carries only the constants up to the highest its shader reads: the
    /// bytes a shader reads are the same as with whole blocks.
    fn write_blocks(&mut self, centre: [f32; 2], fregs: [u32; 2]) {
        if self.ring.len() as u64 + 2 * BLOCK > RING {
            self.flush();
        }
        let fresh = self.ring.is_empty();
        let vs_due = fresh
            || self.st.dirty[0]
            || centre != self.block_centre
            || fregs[0] > self.block_fregs[0];
        let ps_due = fresh || self.st.dirty[1] || fregs[1] > self.block_fregs[1];
        if !vs_due && !ps_due {
            return;
        }
        let atoc = ps_due && self.atoc();
        let st = &self.st;
        // Each sampler's fill mode (its texture's format) and LOD bias.
        let per_sampler = |range: std::ops::Range<usize>| -> ([u32; 16], [f32; 16]) {
            let mut fix = [0u32; 16];
            let mut bias = [0f32; 16];
            for (k, slot) in range.enumerate() {
                fix[k] = st.textures[slot].as_ref().map_or(0, |t| t.format.fix);
                bias[k] = f32::from_bits(st.samplers[slot][state::ss::MIPMAPLODBIAS]);
            }
            (fix, bias)
        };
        // A stage's header (everything before its float constants) is made again only when
        // something it carries changed ([`State::header_dirty`]); the float constants, which
        // most draws change, are copied as they are held.
        if vs_due {
            if st.header_dirty[0] || centre != self.header_centre {
                // The planes `D3DRS_CLIPPLANEENABLE` turns on (all off without `D3DRS_CLIPPING`);
                // a plane that is off is zero, which keeps everything.
                let mut clip = [[0u32; 4]; wgsl::CLIP_PLANES];
                if st.rs[RS_CLIPPING] != 0 {
                    for (k, plane) in clip.iter_mut().enumerate() {
                        if st.rs[rs::CLIPPLANEENABLE] & (1 << k) != 0 {
                            *plane = st.clip[k];
                        }
                    }
                }
                // The vertex stage's system values: the half-pixel move of Direct3D 9's pixel
                // centres.
                let vs_sys = [centre[0], centre[1], 0.0, 0.0];
                self.headers[0] =
                    block_header(&st.vsi, &st.vsb, vs_sys, per_sampler(16..20), &clip);
                self.header_centre = centre;
            }
            self.blocks.0 = write_block(
                &mut self.ring,
                &self.headers[0],
                &self.st.vsf[..16 * fregs[0] as usize],
            );
            self.block_centre = centre;
            self.block_fregs[0] = fregs[0];
        }
        let st = &self.st;
        if ps_due {
            if st.header_dirty[1] || atoc != self.header_atoc {
                // The alpha test, unless alpha to coverage stands in for it ([`ATOC`]: then the
                // comparisons NEVER, LESS, EQUAL and LESSEQUAL draw nothing, the others pass
                // everything and the coverage comes from the alpha alone).
                let alpha_func = match (st.rs[rs::ALPHATESTENABLE] != 0, atoc) {
                    (false, _) => 8.0,
                    (true, false) => st.rs[rs::ALPHAFUNC] as f32,
                    (true, true) if (1..=4).contains(&st.rs[rs::ALPHAFUNC]) => 1.0,
                    (true, true) => 8.0,
                };
                let (above, from) = alpha_bounds(st.rs[rs::ALPHAREF] & 0xFF);
                let ps_sys = [above, alpha_func, from, 0.0];
                self.headers[1] = block_header(
                    &st.psi,
                    &st.psb,
                    ps_sys,
                    per_sampler(0..16),
                    &[[0; 4]; wgsl::CLIP_PLANES],
                );
                self.header_atoc = atoc;
            }
            self.blocks.1 = write_block(
                &mut self.ring,
                &self.headers[1],
                &self.st.psf[..16 * fregs[1] as usize],
            );
            self.block_fregs[1] = fregs[1];
        }
        if vs_due {
            self.st.header_dirty[0] = false;
        }
        if ps_due {
            self.st.header_dirty[1] = false;
        }
        self.st.dirty = [false; 2];
    }

    fn sampler(&mut self, key: SamplerKey) -> wgpu::Sampler {
        if let Some(s) = self.samplers.get(&key) {
            return s.clone();
        }
        let s = self.device.create_sampler(&key.descriptor(self.border));
        self.samplers.insert(key, s.clone());
        s
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn draw(
        &mut self,
        kind: u32,
        indexed: bool,
        base_vertex: i32,
        start: u32,
        count: u32,
        up: Option<(&[u8], u32)>,
    ) {
        // Listed pipelines whose shaders are now made, also when the game draws before its first
        // `Present` (its first frame).
        if self.listed_new {
            self.prepare_listed();
        }
        let (Some(vs), Some(ps)) = (self.st.vs.clone(), self.st.ps.clone()) else {
            self.warn("a draw without both shaders (fixed function is not provided): skipped");
            self.skipped += 1;
            return;
        };
        let Some(topology) = Self::topology(kind) else {
            self.warn(format!("primitive type {kind} is not provided: skipped"));
            self.skipped += 1;
            return;
        };
        let Some(decl) = self.st.decl.clone() else {
            self.warn("a draw without a vertex declaration (FVF is not provided): skipped");
            self.skipped += 1;
            return;
        };
        if self.st.rt.iter().all(|r| r.is_none()) && self.st.ds.is_none() {
            self.skipped += 1;
            return;
        }
        // The vertex inputs: each shader input from the declaration's element of its usage
        // (linked once for each vertex shader and declaration).
        let link = self.vertex_link(&vs, &decl);
        let attributes = &link.attributes;
        let mut strides = [0u32; 8];
        for (k, a) in attributes.iter().enumerate() {
            if !a.is_empty() {
                strides[k] = if up.is_some() && k == 0 {
                    up.map_or(0, |(_, s)| s)
                } else {
                    self.st.streams[k].stride
                };
            }
        }
        let samplers = self.sampler_bindings(&vs, &ps);
        let samplers = samplers.as_slice();
        let filterable = samplers
            .iter()
            .enumerate()
            .fold(0u32, |m, (i, &((_, _, f), _))| m | (u32::from(f) << i));
        let index32 = indexed && self.st.indices.as_ref().is_some_and(|b| b.index32);
        let depth = self.usable_depth();
        let atoc = self.atoc();
        let samples = self
            .st
            .rt
            .iter()
            .flatten()
            .next()
            .map(|s| s.samples)
            .or(depth.as_ref().map(|d| d.samples))
            .unwrap_or(1);
        let key = PipeKey {
            vs: vs.id,
            ps: ps.id,
            decl: decl.id,
            strides,
            topology: kind as u8,
            index32,
            colours: std::array::from_fn(|i| {
                self.st.rt[i]
                    .as_ref()
                    .map(|s| (s.format.wgpu, s.format.opaque))
            }),
            depth: depth.as_ref().map(|d| d.format.wgpu),
            states: PIPELINE_STATES.map(|s| self.st.rs[s]),
            samples,
            filterable,
            atoc,
            count: self.counting.is_some(),
            clip: self.clip_link(samples),
        };
        // The last draw's pipeline again (most draws), else the cache's, else a new one.
        let made = match &self.last_pipeline {
            Some((k, p)) if *k == key => p.clone(),
            _ => {
                if !self.pipelines.contains_key(&key) {
                    // Bounded: render states with continuous values (depth bias) could grow it
                    // without end.
                    if self.pipelines.len() >= 4096 {
                        self.pipelines.clear();
                    }
                    let sampler_keys: Vec<(u32, u8, bool)> =
                        samplers.iter().map(|(k, _)| *k).collect();
                    let pipeline = self.make_pipeline(
                        &key,
                        &vs,
                        &ps,
                        &link.inputs,
                        &ps.varyings,
                        attributes,
                        topology,
                        &sampler_keys,
                    );
                    self.pipelines.insert(key.clone(), pipeline);
                    // Listed, so the next start makes it before play ([`prepare`]).
                    if self.list.is_some() {
                        let entry = prepare::Entry {
                            vs: vs.stable,
                            ps: ps.stable,
                            decl: decl
                                .elements
                                .iter()
                                .map(|e| (e.stream, e.offset, e.ty, e.usage, e.index))
                                .collect(),
                            strides,
                            topology: kind as u8,
                            index32,
                            colours: std::array::from_fn(|i| {
                                self.st.rt[i].as_ref().map(|s| s.format.d3d)
                            }),
                            depth: depth.as_ref().map(|d| d.format.d3d),
                            states: key.states,
                            samples,
                            filterable,
                            atoc,
                            count: key.count,
                            clip: clip_code(key.clip),
                        };
                        let presents = self.presents;
                        if let Some(l) = &mut self.list {
                            l.add(entry, presents);
                        }
                    }
                }
                let p = self.pipelines.get(&key).cloned().flatten();
                self.last_pipeline = Some((key, p.clone()));
                p
            }
        };
        let Some((pipeline, group_layout)) = made else {
            self.skipped += 1;
            return;
        };
        // A texture the draw samples that is also one of its colour targets (a level or a face of
        // it): Direct3D 9 draws so, and wgpu refuses such a pass and with it the whole submit. The
        // game does it when the water displacement is set up near water: its texture-offset pass
        // (ISWATERDISPLACEMENTTEXOFFSET) and then an image-space copy with an offset (ISCOPY) draw
        // into the 512x512 displacement texture while sampling it, and that frame lost every draw
        // (a repeatable run's recording, the route's run near x -49,100, y 10,400). The draw
        // samples a copy of that texture instead, made just before it
        // ([`Device::copy_for_sampling`]): what the target held before this draw.
        let mut copies: [Option<Arc<Tex>>; 20] = Default::default();
        for &(_, slot) in samplers {
            if let Some(t) = self.st.textures[slot].clone() {
                if self.is_colour_target(&t) {
                    copies[slot] = self.copy_for_sampling(&t);
                }
            }
        }
        // The textures and samplers: the last draw's bind group again, else the cache's.
        let mut group_key = [(0u32, 0u8, 0u64, None::<SamplerKey>); 20];
        for (g, &((binding, dim, filterable), slot)) in group_key.iter_mut().zip(samplers) {
            let tex_id = copies[slot]
                .as_ref()
                .or(self.st.textures[slot].as_ref())
                .map_or(0, |t| t.id);
            let sk = SamplerKey::new(&self.st.samplers[slot], filterable);
            *g = (binding, dim, tex_id, Some(sk));
        }
        let group_key = &group_key[..samplers.len()];
        let group = match &self.last_group {
            (k, Some(g)) if k.as_slice() == group_key => g.clone(),
            _ => {
                if !self.groups.contains_key(group_key) {
                    if self.groups.len() > 8192 {
                        self.groups.clear();
                    }
                    let mut views = Vec::new();
                    let mut samps = Vec::new();
                    for &((binding, dim, filterable), slot) in samplers {
                        let view = match copies[slot].as_ref().or(self.st.textures[slot].as_ref()) {
                            Some(t) if (dim == 1) == t.cube => t.view.clone(),
                            _ => self.dummy[dim as usize].clone(),
                        };
                        let sk = SamplerKey::new(&self.st.samplers[slot], filterable);
                        views.push((binding, view));
                        samps.push((binding + 1, self.sampler(sk)));
                    }
                    let mut entries = Vec::new();
                    for (b, v) in &views {
                        entries.push(wgpu::BindGroupEntry {
                            binding: *b,
                            resource: wgpu::BindingResource::TextureView(v),
                        });
                    }
                    for (b, s) in &samps {
                        entries.push(wgpu::BindGroupEntry {
                            binding: *b,
                            resource: wgpu::BindingResource::Sampler(s),
                        });
                    }
                    let group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                        label: None,
                        layout: &group_layout,
                        entries: &entries,
                    });
                    self.groups.insert(group_key.to_vec(), group);
                }
                let group = self.groups[group_key].clone();
                self.last_group.0.clear();
                self.last_group.0.extend_from_slice(group_key);
                self.last_group.1 = Some(group.clone());
                group
            }
        };
        // Vertices from DrawPrimitiveUP go into the batch's ring.
        let up_offset = if let Some((bytes, _)) = up {
            if self.up.len() + bytes.len() > UP_RING as usize {
                self.flush();
            }
            let at = self.up.len();
            self.up.extend_from_slice(bytes);
            self.up.resize(self.up.len().div_ceil(4) * 4, 0);
            Some(at as u64)
        } else {
            None
        };
        let v = self.st.viewport;
        let (tw, th) = self
            .st
            .rt
            .iter()
            .flatten()
            .next()
            .map(|s| (s.width, s.height))
            .or(depth.as_ref().map(|d| (d.width, d.height)))
            .unwrap_or((1, 1));
        let vx = v.x.min(tw.saturating_sub(1));
        let vy = v.y.min(th.saturating_sub(1));
        let vw = v.w.min(tw - vx).max(1);
        let vh = v.h.min(th - vy).max(1);
        // Direct3D 9's pixel centres: each vertex shader moves its position half a pixel right
        // and down, by this viewport's size (the converter's module documentation).
        self.write_blocks([1.0 / vw as f32, -1.0 / vh as f32], [vs.fregs, ps.fregs]);
        self.begin_pass(None);
        let scissor = if self.st.rs[rs::SCISSORTESTENABLE] != 0 {
            let r = self.st.scissor;
            let x0 = r[0].clamp(0, tw as i32) as u32;
            let y0 = r[1].clamp(0, th as i32) as u32;
            let x1 = r[2].clamp(x0 as i32, tw as i32) as u32;
            let y1 = r[3].clamp(y0 as i32, th as i32) as u32;
            (x0, y0, x1 - x0, y1 - y0)
        } else {
            (0, 0, tw, th)
        };
        if scissor.2 == 0 || scissor.3 == 0 {
            return;
        }
        let blocks = self.blocks;
        let stencil_ref = self.st.rs[rs::STENCILREF] & 0xFF;
        let bf = self.st.rs[rs::BLENDFACTOR];
        let mut streams: [Option<(wgpu::Buffer, u64)>; 8] = Default::default();
        for (k, a) in attributes.iter().enumerate() {
            if a.is_empty() {
                continue;
            }
            if up.is_some() && k == 0 {
                streams[k] = Some((self.up_buffer.clone(), up_offset.unwrap_or(0)));
                continue;
            }
            match &self.st.streams[k].buffer {
                Some(b) => {
                    let offset = u64::from(self.st.streams[k].offset);
                    // An offset past the buffer's end (wgpu would refuse the whole pass): the draw
                    // is skipped, as a bad call is.
                    if offset >= b.buffer.size() {
                        self.warn("a stream offset past its buffer's end: the draw is skipped");
                        self.skipped += 1;
                        return;
                    }
                    b.batch.store(self.batch, Ordering::Relaxed);
                    streams[k] = Some((b.buffer.clone(), offset));
                }
                None => {
                    self.warn("a draw with a stream the declaration uses unbound: skipped");
                    self.skipped += 1;
                    return;
                }
            }
        }
        let n = Self::vertex_count(kind, count);
        let index = if indexed {
            match &self.st.indices {
                Some(b) => {
                    // Indices past the buffer's end: skipped (wgpu would refuse the pass).
                    let size = if b.index32 { 4 } else { 2 };
                    if (u64::from(start) + u64::from(n)) * size > b.buffer.size() {
                        self.warn("a draw reading indices past its buffer's end: skipped");
                        self.skipped += 1;
                        return;
                    }
                    b.batch.store(self.batch, Ordering::Relaxed);
                    Some((b.buffer.clone(), b.index32))
                }
                None => {
                    self.skipped += 1;
                    return;
                }
            }
        } else {
            None
        };
        // The textures are this batch's from here (after any send above), so an upload into one
        // later in the batch sends the batch first.
        for &(_, slot) in samplers {
            for t in [&self.st.textures[slot], &copies[slot]]
                .into_iter()
                .flatten()
            {
                t.batch.store(self.batch, Ordering::Relaxed);
            }
        }
        // The viewport is Direct3D 9's own rectangle of whole pixels, so drawing is cut at its
        // pixels' edges and every sample of its edge pixels can be covered (the pixel centres
        // are the vertex shaders' move, above: a viewport moved half a pixel would cut through
        // the edge pixels on an adapter that clips at the viewport's exact edge: WARP, the
        // software adapter, does, and lost 15 of the 64 samples of a 4 by 4 target at 4x).
        // Each of the pass's settings is sent only when it differs from what the pass has.
        let viewport = [
            vx as f32,
            vy as f32,
            vw as f32,
            vh as f32,
            v.min_z.clamp(0.0, 1.0),
            v.max_z.clamp(0.0, 1.0),
        ]
        .map(f32::to_bits);
        let b = &mut self.binds;
        let pass = self.pass.as_mut().expect("a pass is open");
        if b.pipeline.as_ref() != Some(&pipeline) {
            pass.set_pipeline(&pipeline);
            b.pipeline = Some(pipeline);
        }
        if b.blocks != Some(blocks) {
            pass.set_bind_group(0, &self.ring_group, &[blocks.0, blocks.1]);
            b.blocks = Some(blocks);
        }
        if b.group.as_ref() != Some(&group) {
            pass.set_bind_group(1, &group, &[]);
            b.group = Some(group);
        }
        if b.viewport != Some(viewport) {
            let f = viewport.map(f32::from_bits);
            pass.set_viewport(f[0], f[1], f[2], f[3], f[4], f[5]);
            b.viewport = Some(viewport);
        }
        if b.scissor != Some(scissor) {
            pass.set_scissor_rect(scissor.0, scissor.1, scissor.2, scissor.3);
            b.scissor = Some(scissor);
        }
        if b.stencil != Some(stencil_ref) {
            pass.set_stencil_reference(stencil_ref);
            b.stencil = Some(stencil_ref);
        }
        if b.blend != Some(bf) {
            pass.set_blend_constant(wgpu::Color {
                r: f64::from((bf >> 16) & 0xFF) / 255.0,
                g: f64::from((bf >> 8) & 0xFF) / 255.0,
                b: f64::from(bf & 0xFF) / 255.0,
                a: f64::from(bf >> 24) / 255.0,
            });
            b.blend = Some(bf);
        }
        for (slot, s) in streams.into_iter().enumerate() {
            if s.is_none() || b.streams[slot] == s {
                continue;
            }
            if let Some((buffer, offset)) = &s {
                pass.set_vertex_buffer(slot as u32, buffer.slice(*offset..));
            }
            b.streams[slot] = s;
        }
        match index {
            Some(i) => {
                if b.index.as_ref() != Some(&i) {
                    pass.set_index_buffer(
                        i.0.slice(..),
                        if i.1 {
                            wgpu::IndexFormat::Uint32
                        } else {
                            wgpu::IndexFormat::Uint16
                        },
                    );
                    b.index = Some(i);
                }
                pass.draw_indexed(start..start + n, base_vertex, 0..1);
            }
            None => pass.draw(start..start + n, 0..1),
        }
        self.draws += 1;
        // Every [`SPLIT_DRAWS`] draws the open encoder goes to a finishing thread: wgpu does its
        // checking and the driver's recording when an encoder is finished, and those of different
        // encoders run side by side. The next draw opens a new pass (it loads what this one
        // stored) in a new encoder.
        self.encoder_draws += 1;
        if self.encoder_draws >= SPLIT_DRAWS && self.finisher.is_some() {
            self.send_encoder();
        }
    }

    /// The one shader with bytecode hash `hash`, unless two bytecodes share it.
    fn shader_by_hash(&self, hash: u64) -> Option<Arc<Shader>> {
        if self.collided.contains(&hash) {
            return None;
        }
        match self.shader_bytecode.get(&hash).map(Vec::as_slice) {
            Some([one]) => Some(one.clone()),
            _ => None,
        }
    }

    /// The live key and the parts a pipeline of the listed `e` is made from, its two shaders
    /// made; None when the entry names something this backend cannot make (a format it lacks).
    fn key_of_entry(
        &mut self,
        e: &prepare::Entry,
        vs: Arc<Shader>,
        ps: Arc<Shader>,
    ) -> Option<Ready> {
        let topology = Self::topology(u32::from(e.topology))?;
        let elements = e
            .decl
            .iter()
            .map(|&(stream, offset, ty, usage, index)| Element {
                stream,
                offset,
                ty,
                usage,
                index,
            })
            .collect();
        let decl = self.decl_of(elements);
        let link = self.vertex_link(&vs, &decl);
        let mut colours = [None; 4];
        for (c, d) in colours.iter_mut().zip(e.colours) {
            if let Some(d) = d {
                let f = format::format(d)?;
                *c = Some((f.wgpu, f.opaque));
            }
        }
        let depth = match e.depth {
            Some(d) => Some(format::format(d)?.wgpu),
            None => None,
        };
        let mut bindings = self.sampler_bindings(&vs, &ps);
        for (i, b) in bindings.b[..bindings.n].iter_mut().enumerate() {
            b.0 .2 = e.filterable & (1 << i) != 0;
        }
        let samplers = bindings.as_slice().iter().map(|(k, _)| *k).collect();
        let key = PipeKey {
            vs: vs.id,
            ps: ps.id,
            decl: decl.id,
            strides: e.strides,
            topology: e.topology,
            index32: e.index32,
            colours,
            depth,
            states: e.states,
            samples: e.samples,
            filterable: e.filterable,
            atoc: e.atoc,
            count: e.count,
            // Planes on or off is the entry's; how they are cut is this adapter's (a list made on
            // Direct3D 12 serves Vulkan too).
            clip: if e.clip == 0 {
                wgsl::Clip::Off
            } else if self.clip_distances {
                wgsl::Clip::Distances
            } else {
                wgsl::Clip::Discard {
                    per_sample: e.samples > 1,
                }
            },
        };
        Some(Ready {
            key,
            vs,
            ps,
            link,
            topology,
            samplers,
        })
    }

    /// Makes, before the game goes on, every listed pipeline whose two shaders the game has made
    /// ([`prepare`]), at the sample counts its surfaces have: a single-sampled entry when it is
    /// listed so, a multisampled one at each multisample count the surfaces have (a list
    /// recorded at 4x serves a player at 8x: the same draws on that player's targets), where
    /// its formats take that count. The modules first, then the pipelines, each on the worker
    /// threads. At a `Present`, after a new shader or sample count; the game waits for it (its
    /// next frame comes after), so no listed pipeline is ever made during play.
    fn prepare_listed(&mut self) {
        if !self.listed_new {
            return;
        }
        self.listed_new = false;
        let Some(mut l) = self.list.take() else {
            return;
        };
        let started = std::time::Instant::now();
        let mut ready = Vec::new();
        let mut ahead = HashSet::new();
        for i in 0..l.list.entries.len() {
            let e = &l.list.entries[i];
            let want = if e.samples > 1 {
                self.sample_mask & !0b11
            } else {
                self.sample_mask & (1u32 << e.samples.min(31))
            };
            let todo = want & !l.done[i];
            if todo == 0 {
                continue;
            }
            let (Some(vs), Some(ps)) = (self.shader_by_hash(e.vs), self.shader_by_hash(e.ps))
            else {
                continue;
            };
            l.done[i] |= todo;
            let e = e.clone();
            for samples in (1..32u32).filter(|n| todo & (1 << n) != 0) {
                let formats_take_it = samples == e.samples
                    || e.colours.iter().flatten().chain(&e.depth).all(|&d| {
                        format::format(d).is_some_and(|f| self.samples_supported(f.wgpu, samples))
                    });
                if !formats_take_it {
                    continue;
                }
                let e = prepare::Entry {
                    samples,
                    ..e.clone()
                };
                if let Some(r) = self.key_of_entry(&e, vs.clone(), ps.clone()) {
                    // Two entries can name one pipeline (one recorded at 4x and one at 8x).
                    if !self.pipelines.contains_key(&r.key) && ahead.insert(r.key.clone()) {
                        ready.push(r);
                    }
                }
            }
        }
        if ready.is_empty() {
            self.list = Some(l);
            return;
        }
        let threads = worker_threads();
        let mut tasks = Vec::new();
        let mut keys = Vec::new();
        for r in &ready {
            let (vk, pk, t) =
                self.module_tasks(&r.key, &r.vs, &r.ps, &r.link.inputs, &r.ps.varyings);
            tasks.extend(t);
            keys.push((vk, pk));
        }
        let modules = {
            let mut seen = HashSet::new();
            tasks.iter().filter(|t| seen.insert(t.key.clone())).count()
        };
        // The loading screen while it runs, when it is a long step (the start's; a few pipelines
        // ready later, after a new sample count, are made without it).
        let total = modules + ready.len();
        let shown = total >= 32;
        let mut surface = self.surface.take();
        let mut screen = self.loading.take();
        let (device, queue) = (self.device.clone(), self.queue.clone());
        let mut step = |done: usize| {
            if shown {
                let p = loading::Progress {
                    title: "Loading".into(),
                    status: format!("Preparing shaders {} of {}", done.min(total), total),
                    done: done as f32 / total.max(1) as f32,
                };
                show_on(&mut surface, &mut screen, &device, &queue, &p);
            }
        };
        let t = std::time::Instant::now();
        self.make_modules(tasks, threads, false, &mut step);
        let modules_took = t.elapsed();
        let mut jobs = Vec::new();
        for (r, (vk, pk)) in ready.iter().zip(&keys) {
            if let Some(j) = self.pipeline_job(
                &r.key,
                &r.ps,
                vk,
                pk,
                &r.link.attributes,
                r.topology,
                &r.samplers,
            ) {
                jobs.push((r.key.clone(), j));
            }
        }
        let cache = self.pipeline_cache.as_ref().map(|c| &c.cache);
        let built = on_workers(
            &jobs,
            threads,
            |(_, j)| build_pipeline(&device, cache, j),
            &mut |d| step(modules + d),
        );
        step(total);
        drop(step);
        self.surface = surface;
        self.loading = screen;
        let mut refused = 0;
        let made = jobs.len();
        for ((key, j), b) in jobs.into_iter().zip(built) {
            match b {
                Ok(p) => {
                    self.pipelines.insert(key, Some((p, j.group_layout)));
                }
                Err(e) => {
                    refused += 1;
                    self.warn(format!("a pipeline wgpu refuses: {e}"));
                    self.pipelines.insert(key, None);
                }
            }
        }
        if let Some(c) = &self.pipeline_cache {
            c.touched(self.presents);
        }
        let left = l.done.iter().filter(|&&d| d == 0).count();
        eprintln!(
            "NV_D3D9 wgpu: made ahead: {made} pipelines and {modules} shader modules in {:.0} ms (modules {:.0} ms) on {threads} threads, {refused} refused; {left} listed not yet possible (present {})",
            started.elapsed().as_secs_f64() * 1000.0,
            modules_took.as_secs_f64() * 1000.0,
            self.presents
        );
        self.list = Some(l);
    }

    /// The cache keys of the two modules a pipeline of `key` takes, and the work to make each
    /// one not made yet (its WGSL converted here, the module made by [`Device::make_modules`]).
    fn module_tasks(
        &self,
        key: &PipeKey,
        vs: &Shader,
        ps: &Shader,
        inputs: &[wgsl::Input],
        varyings: &[wgsl::Semantic],
    ) -> (VKey, PKey, Vec<ModuleTask>) {
        let vkey = (
            vs.id,
            inputs.to_vec(),
            varyings.to_vec(),
            ps.sig.centroid.clone(),
            key.clip,
        );
        let pkey = (ps.id, varyings.to_vec(), key.count, key.clip);
        let mut tasks = Vec::new();
        if !self.vs_modules.contains_key(&vkey) {
            let link = wgsl::Link {
                inputs: inputs.to_vec(),
                varyings: varyings.to_vec(),
                centroid: ps.sig.centroid.clone(),
                clip: key.clip,
                count: false,
            };
            let t = std::time::Instant::now();
            let source = wgsl::convert(&vs.program, &link);
            tasks.push(ModuleTask {
                key: ModKey::V(vkey.clone()),
                source,
                convert: t.elapsed(),
            });
        }
        if !self.ps_modules.contains_key(&pkey) {
            let link = wgsl::Link {
                inputs: Vec::new(),
                varyings: varyings.to_vec(),
                centroid: ps.sig.centroid.clone(),
                clip: key.clip,
                count: key.count,
            };
            let t = std::time::Instant::now();
            let source = wgsl::convert(&ps.program, &link);
            tasks.push(ModuleTask {
                key: ModKey::P(pkey.clone()),
                source,
                convert: t.elapsed(),
            });
        }
        (vkey, pkey, tasks)
    }

    /// Makes the modules of `tasks` (each once, on `threads` worker threads; 1: here) and keeps
    /// them; a module wgpu or the converter refuses is kept as none and said once. `on_draw`: for
    /// a draw's pipeline, counted in the pipeline report (modules made ahead have a line of their
    /// own, [`Device::prepare_listed`]).
    fn make_modules(
        &mut self,
        mut tasks: Vec<ModuleTask>,
        threads: usize,
        on_draw: bool,
        tick: &mut dyn FnMut(usize),
    ) {
        let mut seen = HashSet::new();
        tasks.retain(|t| seen.insert(t.key.clone()));
        let device = self.device.clone();
        let made = on_workers(
            &tasks,
            threads,
            |t| {
                let start = std::time::Instant::now();
                let m = match &t.source {
                    Ok(s) => create_module(&device, s.clone()),
                    Err(e) => Err(e.to_string()),
                };
                (m, start.elapsed())
            },
            tick,
        );
        for (t, (m, took)) in tasks.into_iter().zip(made) {
            if on_draw {
                self.made.add_module(t.convert, took);
            }
            let what = match t.key {
                ModKey::V(_) => "a vertex shader",
                ModKey::P(_) => "a pixel shader",
            };
            let m = match m {
                Ok(m) => Some(m),
                Err(e) => {
                    self.warn(format!("{what}: the converted shader is not valid: {e}"));
                    None
                }
            };
            match t.key {
                ModKey::V(k) => {
                    self.vs_modules.insert(k, m);
                }
                ModKey::P(k) => {
                    self.ps_modules.insert(k, m);
                }
            }
        }
    }

    /// What making a pipeline of `key` takes, its modules made ([`Device::make_modules`]); None
    /// when one of them was refused.
    #[allow(clippy::too_many_arguments)]
    fn pipeline_job(
        &mut self,
        key: &PipeKey,
        ps: &Shader,
        vkey: &VKey,
        pkey: &PKey,
        attributes: &[Vec<wgpu::VertexAttribute>; 8],
        topology: wgpu::PrimitiveTopology,
        samplers: &[(u32, u8, bool)],
    ) -> Option<PipelineJob> {
        let vm = self.vs_modules.get(vkey)?.clone()?;
        let pm = self.ps_modules.get(pkey)?.clone()?;
        let layout = self.sampler_layout(samplers);
        let group_layout = self.sampler_layouts[samplers].0.clone();
        // One layout per stream slot up to the last used (a slot's index is its stream's number).
        let last = attributes
            .iter()
            .rposition(|a| !a.is_empty())
            .map_or(0, |i| i + 1);
        let buffers = attributes[..last]
            .iter()
            .enumerate()
            .map(|(k, a)| (!a.is_empty()).then(|| (u64::from(key.strides[k]), a.clone())))
            .collect();
        // The render states the pipeline is made from: the key's own, so a pipeline made ahead
        // ([`prepare`]) is the one a draw with those states would make.
        let mut s = [0u32; 256];
        for (k, &state) in PIPELINE_STATES.iter().enumerate() {
            s[state] = key.states[k];
        }
        let blend = state::blend(&s);
        let colour_states = [
            rs::COLORWRITEENABLE,
            rs::COLORWRITEENABLE1,
            rs::COLORWRITEENABLE2,
            rs::COLORWRITEENABLE3,
        ];
        let written = ps.sig.colours;
        let mut targets: Vec<Option<wgpu::ColorTargetState>> = (0..4)
            .map(|i| {
                key.colours[i].map(|(format, opaque)| wgpu::ColorTargetState {
                    format,
                    blend,
                    write_mask: if written & (1 << i) != 0 {
                        state::write_mask(s[colour_states[i]], opaque)
                    } else {
                        wgpu::ColorWrites::empty()
                    },
                })
            })
            .collect();
        while targets.last().is_some_and(|t| t.is_none()) {
            targets.pop();
        }
        // The occlusion count: one added for each sample the draw passes.
        if key.count {
            let add = wgpu::BlendComponent {
                src_factor: wgpu::BlendFactor::One,
                dst_factor: wgpu::BlendFactor::One,
                operation: wgpu::BlendOperation::Add,
            };
            targets.resize(wgsl::COUNT_TARGET as usize, None);
            targets.push(Some(wgpu::ColorTargetState {
                format: self.count_format(key.samples),
                blend: Some(wgpu::BlendState {
                    color: add,
                    alpha: add,
                }),
                write_mask: wgpu::ColorWrites::RED,
            }));
        }
        let strip = matches!(
            topology,
            wgpu::PrimitiveTopology::TriangleStrip | wgpu::PrimitiveTopology::LineStrip
        );
        Some(PipelineJob {
            layout,
            group_layout,
            vm,
            pm,
            buffers,
            primitive: wgpu::PrimitiveState {
                topology,
                strip_index_format: strip.then_some(if key.index32 {
                    wgpu::IndexFormat::Uint32
                } else {
                    wgpu::IndexFormat::Uint16
                }),
                front_face: wgpu::FrontFace::Cw,
                cull_mode: state::cull(s[rs::CULLMODE]),
                unclipped_depth: false,
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: key.depth.map(|f| state::depth_stencil(&s, f)),
            multisample: wgpu::MultisampleState {
                count: key.samples,
                mask: !0,
                alpha_to_coverage_enabled: key.atoc,
            },
            targets,
        })
    }

    /// A pipeline made on a draw that needs it (not made ahead): it holds that draw, and so its
    /// frame, until it is done; one that takes over a millisecond is said with its shaders.
    #[allow(clippy::too_many_arguments)]
    fn make_pipeline(
        &mut self,
        key: &PipeKey,
        vs: &Shader,
        ps: &Shader,
        inputs: &[wgsl::Input],
        varyings: &[wgsl::Semantic],
        attributes: &[Vec<wgpu::VertexAttribute>; 8],
        topology: wgpu::PrimitiveTopology,
        samplers: &[(u32, u8, bool)],
    ) -> Option<(wgpu::RenderPipeline, wgpu::BindGroupLayout)> {
        self.made_on_draws += 1;
        let started = std::time::Instant::now();
        let (vkey, pkey, tasks) = self.module_tasks(key, vs, ps, inputs, varyings);
        self.make_modules(tasks, 1, true, &mut |_| {});
        let job = self.pipeline_job(key, ps, &vkey, &pkey, attributes, topology, samplers)?;
        let t = std::time::Instant::now();
        let made = build_pipeline(
            &self.device,
            self.pipeline_cache.as_ref().map(|c| &c.cache),
            &job,
        );
        self.made.add_pipeline(t.elapsed(), started.elapsed());
        if let Some(c) = &self.pipeline_cache {
            c.touched(self.presents);
        }
        let took = started.elapsed();
        if self.list.is_some() && took.as_secs_f64() > 0.001 {
            eprintln!(
                "NV_D3D9 wgpu: a pipeline made on first use: {:.1} ms (shaders {:016x} {:016x}, present {})",
                took.as_secs_f64() * 1000.0,
                vs.stable,
                ps.stable,
                self.presents
            );
        }
        match made {
            Ok(p) => Some((p, job.group_layout)),
            Err(e) => {
                self.warn(format!("a pipeline wgpu refuses: {e}"));
                None
            }
        }
    }

    // Copies.

    /// The copy pipeline into `format` (`opaque`: a target without alpha, whose alpha stays 1).
    fn blit_pipeline(
        &mut self,
        (format, opaque): (wgpu::TextureFormat, bool),
    ) -> (wgpu::RenderPipeline, wgpu::BindGroupLayout) {
        let layout = self
            .blit_layout
            .get_or_insert_with(|| {
                self.device
                    .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                        label: Some("blit"),
                        entries: &[
                            wgpu::BindGroupLayoutEntry {
                                binding: 0,
                                visibility: wgpu::ShaderStages::FRAGMENT,
                                ty: wgpu::BindingType::Texture {
                                    sample_type: wgpu::TextureSampleType::Float {
                                        filterable: true,
                                    },
                                    view_dimension: wgpu::TextureViewDimension::D2,
                                    multisampled: false,
                                },
                                count: None,
                            },
                            wgpu::BindGroupLayoutEntry {
                                binding: 1,
                                visibility: wgpu::ShaderStages::FRAGMENT,
                                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                                count: None,
                            },
                            wgpu::BindGroupLayoutEntry {
                                binding: 2,
                                visibility: wgpu::ShaderStages::VERTEX,
                                ty: wgpu::BindingType::Buffer {
                                    ty: wgpu::BufferBindingType::Uniform,
                                    has_dynamic_offset: false,
                                    min_binding_size: None,
                                },
                                count: None,
                            },
                        ],
                    })
            })
            .clone();
        if let Some(p) = self.blit.get(&(format, opaque)) {
            return (p.clone(), layout);
        }
        let module = self
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("blit"),
                source: wgpu::ShaderSource::Wgsl(BLIT.into()),
            });
        let pl = self
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: None,
                bind_group_layouts: &[Some(&layout)],
                immediate_size: 0,
            });
        let p = self
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("blit"),
                layout: Some(&pl),
                vertex: wgpu::VertexState {
                    module: &module,
                    entry_point: Some("vs"),
                    compilation_options: Default::default(),
                    buffers: &[],
                },
                primitive: wgpu::PrimitiveState::default(),
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                fragment: Some(wgpu::FragmentState {
                    module: &module,
                    entry_point: Some("fs"),
                    compilation_options: Default::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format,
                        blend: None,
                        write_mask: state::write_mask(0xF, opaque),
                    })],
                }),
                multiview_mask: None,
                cache: None,
            });
        self.blit.insert((format, opaque), p.clone());
        (p, layout)
    }

    /// Draws `source` (its rectangle in texels) into `target`'s rectangle, filtered or not.
    #[allow(clippy::too_many_arguments)]
    fn blit_into(
        &mut self,
        source: &wgpu::TextureView,
        source_size: (u32, u32),
        source_rect: [f32; 4],
        target: &wgpu::TextureView,
        format: (wgpu::TextureFormat, bool),
        target_rect: [f32; 4],
        linear: bool,
    ) {
        self.end_pass();
        let (pipeline, layout) = self.blit_pipeline(format);
        let sampler = self.sampler(SamplerKey {
            address: [3, 3, 3],
            mag: if linear { 2 } else { 1 },
            min: if linear { 2 } else { 1 },
            mip: 0,
            max_level: 0,
            anisotropy: 1,
            border: 0,
            filterable: true,
        });
        let (w, h) = (source_size.0.max(1) as f32, source_size.1.max(1) as f32);
        let uv = [
            source_rect[0] / w,
            source_rect[1] / h,
            source_rect[2] / w,
            source_rect[3] / h,
        ];
        let bytes: Vec<u8> = uv.iter().flat_map(|v| v.to_le_bytes()).collect();
        let ub = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: 16,
            usage: wgpu::BufferUsages::UNIFORM,
            mapped_at_creation: true,
        });
        if let Ok(mut mapped) = ub.slice(..).get_mapped_range_mut() {
            mapped.copy_from_slice(&bytes);
        }
        ub.unmap();
        let group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(source),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: ub.as_entire_binding(),
                },
            ],
        });
        let encoder = self.encoder();
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("blit"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: target,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        pass.set_pipeline(&pipeline);
        pass.set_bind_group(0, &group, &[]);
        pass.set_viewport(
            target_rect[0],
            target_rect[1],
            (target_rect[2] - target_rect[0]).max(1.0),
            (target_rect[3] - target_rect[1]).max(1.0),
            0.0,
            1.0,
        );
        pass.draw(0..3, 0..1);
    }

    pub(crate) fn stretch_rect(
        &mut self,
        source: Handle,
        source_rect: Option<Rect>,
        destination: Handle,
        destination_rect: Option<Rect>,
        filter: u32,
    ) {
        let (Some(src), Some(dst)) = (self.surface_of(source), self.surface_of(destination)) else {
            self.warn("StretchRect of a surface without a counterpart");
            return;
        };
        let (Some(sv), Some(dv)) = (src.view.clone(), dst.view.clone()) else {
            return;
        };
        if src.format.depth || dst.format.depth {
            if src.width == dst.width
                && src.height == dst.height
                && src.format.wgpu == dst.format.wgpu
            {
                self.end_pass();
                let (st, dt) = (
                    src.tex.clone().expect("a texture"),
                    dst.tex.clone().expect("a texture"),
                );
                self.encoder().copy_texture_to_texture(
                    wgpu::TexelCopyTextureInfo {
                        texture: &st.texture,
                        mip_level: src.level,
                        origin: wgpu::Origin3d::ZERO,
                        aspect: wgpu::TextureAspect::All,
                    },
                    wgpu::TexelCopyTextureInfo {
                        texture: &dt.texture,
                        mip_level: dst.level,
                        origin: wgpu::Origin3d::ZERO,
                        aspect: wgpu::TextureAspect::All,
                    },
                    wgpu::Extent3d {
                        width: src.width,
                        height: src.height,
                        depth_or_array_layers: 1,
                    },
                );
            } else {
                self.warn("StretchRect of depth surfaces of different sizes is not provided");
            }
            return;
        }
        let sr = source_rect.unwrap_or([0, 0, src.width as i32, src.height as i32]);
        let dr = destination_rect.unwrap_or([0, 0, dst.width as i32, dst.height as i32]);
        if dst.samples > 1 {
            self.warn("StretchRect into a multisampled surface is not provided");
            return;
        }
        // A multisampled source is resolved first (the samples averaged, as Direct3D 9's
        // StretchRect does): straight into the destination when it is the same whole surface,
        // else into a scratch texture that is then copied as any other source.
        let (sv, src_size) = if src.samples > 1 {
            let whole_src = sr == [0, 0, src.width as i32, src.height as i32];
            let whole_dst = dr == [0, 0, dst.width as i32, dst.height as i32];
            if whole_src
                && whole_dst
                && (src.width, src.height) == (dst.width, dst.height)
                && src.format.wgpu == dst.format.wgpu
            {
                self.resolve(&sv, &dv, &src);
                for s in [&src, &dst] {
                    if let Some(t) = &s.tex {
                        t.batch.store(self.batch, Ordering::Relaxed);
                    }
                }
                return;
            }
            let Some(scratch) =
                self.new_texture(src.width, src.height, 1, src.format.d3d, false, true, 1)
            else {
                return;
            };
            let resolved = self.level_surface(&scratch, 0, 0);
            let rv = resolved.view.clone().expect("a view");
            self.resolve(&sv, &rv, &src);
            (rv, (src.width, src.height))
        } else {
            (sv, (src.width, src.height))
        };
        let f = |r: Rect| r.map(|v| v as f32);
        self.blit_into(
            &sv,
            src_size,
            f(sr),
            &dv,
            (dst.format.wgpu, dst.format.opaque),
            f(dr),
            filter >= 2,
        );
        for s in [&src, &dst] {
            if let Some(t) = &s.tex {
                t.batch.store(self.batch, Ordering::Relaxed);
            }
        }
    }

    /// Resolves the multisampled `source` (the view of `surface`) into `target` (same size and
    /// format). An 8-bit colour surface is resolved as the reference machine's driver does,
    /// gamma-corrected ([`GAMMA_RESOLVE`]); others by the hardware's average: a render pass that
    /// only loads the samples and resolves them at its end.
    fn resolve(&mut self, source: &wgpu::TextureView, target: &wgpu::TextureView, surface: &Surf) {
        self.end_pass();
        if gamma_resolved(surface.format.wgpu) {
            self.gamma_resolve(source, target, surface);
            return;
        }
        let encoder = self.encoder();
        let _ = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("resolve"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: source,
                depth_slice: None,
                resolve_target: Some(target),
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
    }

    /// [`Device::resolve`] of an 8-bit colour surface: each sample's colour taken out of the sRGB
    /// curve, averaged, put back; alpha averaged as it is ([`GAMMA_RESOLVE`]).
    fn gamma_resolve(&mut self, source: &wgpu::TextureView, target: &wgpu::TextureView, s: &Surf) {
        let key = (s.format.wgpu, s.samples, s.format.opaque);
        if !self.gamma_resolves.contains_key(&key) {
            let layout = self
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("gamma resolve"),
                    entries: &[wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: false },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: true,
                        },
                        count: None,
                    }],
                });
            let module = self
                .device
                .create_shader_module(wgpu::ShaderModuleDescriptor {
                    label: Some("gamma resolve"),
                    source: wgpu::ShaderSource::Wgsl(
                        GAMMA_RESOLVE
                            .replace("SAMPLES", &format!("{}u", s.samples))
                            .into(),
                    ),
                });
            let pl = self
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: None,
                    bind_group_layouts: &[Some(&layout)],
                    immediate_size: 0,
                });
            let p = self
                .device
                .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                    label: Some("gamma resolve"),
                    layout: Some(&pl),
                    vertex: wgpu::VertexState {
                        module: &module,
                        entry_point: Some("vs"),
                        compilation_options: Default::default(),
                        buffers: &[],
                    },
                    primitive: wgpu::PrimitiveState::default(),
                    depth_stencil: None,
                    multisample: wgpu::MultisampleState::default(),
                    fragment: Some(wgpu::FragmentState {
                        module: &module,
                        entry_point: Some("fs"),
                        compilation_options: Default::default(),
                        targets: &[Some(wgpu::ColorTargetState {
                            format: s.format.wgpu,
                            blend: None,
                            write_mask: state::write_mask(0xF, s.format.opaque),
                        })],
                    }),
                    multiview_mask: None,
                    cache: None,
                });
            self.gamma_resolves.insert(key, (p, layout));
        }
        let (pipeline, layout) = self.gamma_resolves[&key].clone();
        let group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(source),
            }],
        });
        let encoder = self.encoder();
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("gamma resolve"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: target,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        pass.set_pipeline(&pipeline);
        pass.set_bind_group(0, &group, &[]);
        pass.draw(0..3, 0..1);
    }

    pub(crate) fn update_texture(&mut self, source: Handle, destination: Handle) {
        let (Some(s), Some(d)) = (self.texture_of(source), self.texture_of(destination)) else {
            return;
        };
        // Direct3D 9 matches the levels by size: a source with more (larger) levels starts at the
        // one of the destination's top size.
        let Some(skip) = (0..s.levels)
            .find(|&l| ((s.width >> l).max(1), (s.height >> l).max(1)) == (d.width, d.height))
        else {
            self.warn("UpdateTexture between textures of no common level size: skipped");
            return;
        };
        if s.format.wgpu != d.format.wgpu || s.cube != d.cube {
            self.warn("UpdateTexture between different formats: skipped");
            return;
        }
        self.end_pass();
        let levels = (s.levels - skip).min(d.levels);
        let layers = if s.cube { 6 } else { 1 };
        for level in 0..levels {
            let w = (d.width >> level).max(1);
            let h = (d.height >> level).max(1);
            let (w, h) = if s.format.block {
                (w.div_ceil(4) * 4, h.div_ceil(4) * 4)
            } else {
                (w, h)
            };
            self.encoder().copy_texture_to_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: &s.texture,
                    mip_level: level + skip,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                wgpu::TexelCopyTextureInfo {
                    texture: &d.texture,
                    mip_level: level,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                wgpu::Extent3d {
                    width: w,
                    height: h,
                    depth_or_array_layers: layers,
                },
            );
        }
    }

    // Queries.

    /// `Issue(flags)` of `query`. An occlusion query's `D3DISSUE_BEGIN` starts its count: the
    /// open pass ends, and every pass until its `END` carries the count's target
    /// ([`CountSegment`]), to which each draw adds one for each sample it passes (after the
    /// depth and stencil tests, the alpha test and `texkill`: Direct3D 9's count); each pass's
    /// samples are summed into the query's slot as it ends. An event query has nothing here.
    pub(crate) fn query_issue(&mut self, query: Handle, flags: u32) {
        let Some(Obj::Query(q)) = self.objects.get(&query) else {
            return;
        };
        if q.kind != QUERY_OCCLUSION {
            return;
        }
        if flags & ISSUE_BEGIN != 0 {
            if self.counting.is_some_and(|(other, _)| other != query) {
                self.warn("an occlusion query begun while another counts: the other stops there");
            }
            self.end_pass();
            let slot = self.next_count_slot;
            self.next_count_slot = (slot + 1) % COUNT_SLOTS;
            let counts = self.count_buffer.clone();
            self.encoder()
                .clear_buffer(&counts, u64::from(slot) * 4, Some(4));
            self.counting = Some((query, slot));
            if let Some(Obj::Query(q)) = self.objects.get_mut(&query) {
                q.slot = Some(slot);
                q.result = None;
                q.read = None;
            }
        }
        if flags & ISSUE_END != 0 {
            if self.counting.is_some_and(|(q, _)| q == query) {
                self.end_pass();
                self.counting = None;
            }
            let batch = self.batch;
            if let Some(Obj::Query(q)) = self.objects.get_mut(&query) {
                q.batch = batch;
                q.result = None;
                q.read = None;
            }
            // Its count is posted for the game's threads once the GPU has it
            // ([`Device::post_answers`]).
            if let Some(a) = &self.answers {
                let ends = a.ended(query);
                self.awaiting.push((query, ends));
            }
        }
    }

    /// The occlusion counts the game's threads wait for ([`QueryAnswers`]): each awaited `END`'s
    /// count read as `GetData` with `D3DGETDATA_FLUSH` reads it (its batch sent if it is not yet),
    /// never waiting for the GPU, and posted once it is in. Whether any is still awaited.
    fn post_answers(&mut self) -> bool {
        if self.answers.is_none() || self.awaiting.is_empty() {
            return false;
        }
        let mut awaiting = std::mem::take(&mut self.awaiting);
        awaiting.retain(|&(query, ends)| {
            let Some(a) = self.answers.clone() else {
                return false;
            };
            // A later `END` of the same query supersedes this one.
            if !a.awaits(query, ends) {
                return false;
            }
            match self.query_get_data(query, GETDATA_FLUSH) {
                Some((S_FALSE, _)) => true,
                Some((_, count)) => {
                    a.post(query, ends, count);
                    false
                }
                None => false,
            }
        });
        self.awaiting.extend(awaiting);
        !self.awaiting.is_empty()
    }

    /// The render thread has nothing queued ([`d3d9_stream::Backend::idle`]): the awaited counts
    /// polled and posted. Whether any is still awaited.
    pub(crate) fn idle(&mut self) -> bool {
        self.post_answers()
    }

    /// `GetData(flags)` of an occlusion query after its `END`, as Direct3D 9 answers it: `S_FALSE`
    /// (not ready) until the GPU has counted, then `S_OK` and the samples its draws passed; it
    /// never waits. While the batch that counted is not yet sent, `D3DGETDATA_FLUSH` sends it
    /// (without the flag the answer stays `S_FALSE` until something else sends it: a `Present`, a
    /// full batch, a read-back; measured so on the real Direct3D 9 here, which answered `S_FALSE`
    /// to 20,000 `GetData`s without the flag and `S_OK` within milliseconds with it). Once sent,
    /// each call polls the device without waiting. `None` for an event query, whose answer is the
    /// model's.
    pub(crate) fn query_get_data(&mut self, query: Handle, flags: u32) -> Option<(u32, u32)> {
        use std::sync::atomic::AtomicU8;
        let q = match self.objects.get(&query) {
            Some(Obj::Query(q)) if q.kind == QUERY_OCCLUSION => q.clone(),
            _ => return None,
        };
        if let Some(n) = q.result {
            return Some((0, n));
        }
        let Some(slot) = q.slot else {
            // Ended without a begin: nothing was counted.
            return Some((0, 0));
        };
        if self.counting.is_some_and(|(c, _)| c == query) {
            // Still counting (asked before its END): not done, as Direct3D 9 answers.
            return Some((S_FALSE, 0));
        }
        let read = match q.read {
            Some(read) => read,
            None => {
                if q.batch == self.batch {
                    if flags & GETDATA_FLUSH == 0 {
                        return Some((S_FALSE, 0));
                    }
                    self.flush();
                }
                let buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("occlusion count read-back"),
                    size: 4,
                    usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                    mapped_at_creation: false,
                });
                let mut e = self
                    .device
                    .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
                e.copy_buffer_to_buffer(&self.count_buffer, u64::from(slot) * 4, &buffer, 0, 4);
                self.queue.submit([e.finish()]);
                let state = Arc::new(AtomicU8::new(0));
                let mapped = state.clone();
                buffer.slice(..).map_async(wgpu::MapMode::Read, move |r| {
                    mapped.store(if r.is_ok() { 1 } else { 2 }, Ordering::Release);
                });
                let read = CountRead { buffer, state };
                if let Some(Obj::Query(q)) = self.objects.get_mut(&query) {
                    q.read = Some(read.clone());
                }
                read
            }
        };
        let _ = self.device.poll(wgpu::PollType::Poll);
        let mapped = match read.state.load(Ordering::Acquire) {
            0 => return Some((S_FALSE, 0)),
            1 => read
                .buffer
                .slice(..)
                .get_mapped_range()
                .map(|data| u32::from_le_bytes([data[0], data[1], data[2], data[3]]))
                .map_err(|e| e.to_string()),
            _ => Err("the map failed".to_string()),
        };
        let n = match mapped {
            Ok(n) => {
                read.buffer.unmap();
                n
            }
            Err(e) => {
                // (A lost device.) Not ready; asked again, the count is read again.
                self.warn(format!(
                    "an occlusion count not read back ({e}): read again"
                ));
                if let Some(Obj::Query(q)) = self.objects.get_mut(&query) {
                    q.read = None;
                }
                return Some((S_FALSE, 0));
            }
        };
        if let Some(Obj::Query(q)) = self.objects.get_mut(&query) {
            q.result = Some(n);
            q.read = None;
        }
        Some((0, n))
    }

    // Read-backs.

    /// `ReadSurface`: the bytes of a managed or system-memory level as the backend keeps them
    /// (`size` of them, rows packed); none for another level or another size.
    pub(crate) fn read_surface(&self, surface: Handle, size: u32) -> Option<Vec<u8>> {
        self.kept
            .get(&surface)
            .filter(|b| b.len() == size as usize)
            .cloned()
    }

    /// The pixels of a level as wgpu holds them, rows packed (a multisampled surface resolved
    /// first, as Direct3D 9 resolves for a read); none for block-compressed levels.
    fn read_level(&mut self, s: &Arc<Surf>) -> Option<Vec<u8>> {
        let tex = s.tex.clone()?;
        self.end_pass();
        if s.format.block || s.format.depth {
            return None;
        }
        let bpp = s.format.wgpu.block_copy_size(None)?;
        let (texture, level, layer) = if s.samples > 1 {
            let scratch = self.new_texture(s.width, s.height, 1, s.format.d3d, false, true, 1)?;
            let resolved = self.level_surface(&scratch, 0, 0);
            let (from, to) = (s.view.clone()?, resolved.view.clone()?);
            self.resolve(&from, &to, s);
            (scratch.texture.clone(), 0, 0)
        } else {
            (tex.texture.clone(), s.level, s.layer)
        };
        let row = (s.width * bpp).div_ceil(256) * 256;
        let buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: u64::from(row * s.height),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        self.encoder().copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: level,
                origin: wgpu::Origin3d {
                    x: 0,
                    y: 0,
                    z: layer,
                },
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(row),
                    rows_per_image: Some(s.height),
                },
            },
            wgpu::Extent3d {
                width: s.width,
                height: s.height,
                depth_or_array_layers: 1,
            },
        );
        self.flush();
        buffer.slice(..).map_async(wgpu::MapMode::Read, |_| {});
        let _ = self.device.poll(wgpu::PollType::wait_indefinitely());
        let mut out = Vec::with_capacity((s.width * bpp * s.height) as usize);
        {
            let data = buffer.slice(..).get_mapped_range().ok()?;
            for y in 0..s.height {
                out.extend_from_slice(
                    &data[(y * row) as usize..(y * row + s.width * bpp) as usize],
                );
            }
        }
        buffer.unmap();
        Some(out)
    }

    /// `GetRenderTargetData(source, destination)`: the render target's pixels into the
    /// destination (a system-memory surface or level of the same size and format, as the model
    /// checked): on the GPU, and as the level's kept bytes (`ReadSurface`); with `pitch` (not 0),
    /// also its rows, `row` bytes each (the narrower of `pitch` and the level's row). `None` when
    /// either surface has no counterpart or the format's bytes are not kept as Direct3D 9 lays
    /// them out.
    pub(crate) fn get_render_target_data(
        &mut self,
        source: Handle,
        destination: Handle,
        pitch: u32,
    ) -> Option<d3d9_stream::Rows> {
        let src = self.surface_of(source)?;
        let read = if src.samples > 1 {
            // Direct3D 9 refuses a multisampled source (D3DERR_INVALIDCALL, as documented and
            // measured on the real one here): nothing is copied.
            self.warn(
                "GetRenderTargetData of a multisampled render target: refused, as Direct3D 9 does",
            );
            None
        } else if src.format.upload != format::Upload::Copy {
            self.warn(format!(
                "GetRenderTargetData of format {:#x} is not provided: the destination keeps what it had",
                src.format.d3d
            ));
            None
        } else {
            let read = self.read_level(&src);
            if read.is_none() {
                // The read-back failed: the device is lost (a driver reset) or the format is not
                // read back.
                self.warn(
                    "GetRenderTargetData: the render target could not be read back (a lost device?): the destination keeps what it had",
                );
            }
            read
        };
        let Some(bytes) = read else {
            // Nothing copied: the destination keeps what it had, which its rows then give, as the
            // real backend reads them back after a refused or failed copy. So the caller goes on
            // (a save's thumbnail after a driver reset: the model would stop the run on no answer).
            return Some(self.rows_kept(destination, &src, pitch));
        };
        if let Some(dst) = self.surface_of(destination) {
            if let (Some(st), Some(dt)) = (&src.tex, &dst.tex) {
                if src.samples == 1
                    && dst.samples == 1
                    && (src.width, src.height, src.format.wgpu)
                        == (dst.width, dst.height, dst.format.wgpu)
                {
                    self.encoder().copy_texture_to_texture(
                        wgpu::TexelCopyTextureInfo {
                            texture: &st.texture,
                            mip_level: src.level,
                            origin: wgpu::Origin3d {
                                x: 0,
                                y: 0,
                                z: src.layer,
                            },
                            aspect: wgpu::TextureAspect::All,
                        },
                        wgpu::TexelCopyTextureInfo {
                            texture: &dt.texture,
                            mip_level: dst.level,
                            origin: wgpu::Origin3d {
                                x: 0,
                                y: 0,
                                z: dst.layer,
                            },
                            aspect: wgpu::TextureAspect::All,
                        },
                        wgpu::Extent3d {
                            width: src.width,
                            height: src.height,
                            depth_or_array_layers: 1,
                        },
                    );
                    dt.batch.store(self.batch, Ordering::Relaxed);
                }
            }
        }
        let natural = (bytes.len() as u32) / src.height.max(1);
        let rows = if pitch == 0 {
            d3d9_stream::Rows {
                row: 0,
                bytes: Vec::new(),
            }
        } else {
            let row = pitch.min(natural);
            let mut out = Vec::with_capacity((row * src.height) as usize);
            for r in bytes.chunks(natural.max(1) as usize) {
                out.extend_from_slice(&r[..row as usize]);
            }
            d3d9_stream::Rows { row, bytes: out }
        };
        if let Some(k) = self.kept.get_mut(&destination) {
            *k = bytes;
        }
        Some(rows)
    }

    /// The rows of `destination` (a system-memory level of `source`'s size and format, as the
    /// model checked) as it holds them, when nothing was copied into it: its kept bytes, made
    /// zeros of the level's size if it holds none yet (never written, or by a GPU that is lost),
    /// so the game's lock of it reads them back as the real backend's does; `pitch` as in
    /// [`Device::get_render_target_data`].
    fn rows_kept(&mut self, destination: Handle, source: &Surf, pitch: u32) -> d3d9_stream::Rows {
        let (natural, height) = format::level_layout(&source.format, source.width, source.height);
        let size = (natural * height) as usize;
        let mut zeros = Vec::new();
        let kept = match self.kept.get_mut(&destination) {
            Some(k) => {
                if k.len() != size {
                    k.clear();
                    k.resize(size, 0);
                }
                k
            }
            None => {
                zeros.resize(size, 0);
                &mut zeros
            }
        };
        let row = if pitch == 0 { 0 } else { pitch.min(natural) };
        let bytes = if row == 0 {
            Vec::new()
        } else {
            kept.chunks(natural.max(1) as usize)
                .flat_map(|r| r[..row as usize].iter().copied())
                .collect()
        };
        d3d9_stream::Rows { row, bytes }
    }

    // Present.

    /// Shows `p` in the game's window ([`loading`]): for a long step of the backend's own before
    /// play, which calls it as it goes. Only a device on the game's window draws it; every call
    /// also writes one log line with the same numbers (for a launcher). The game's own frames
    /// are not touched: the next `Present` shows the game again.
    pub fn show_progress(&mut self, p: &loading::Progress) {
        show_on(
            &mut self.surface,
            &mut self.loading,
            &self.device,
            &self.queue,
            p,
        );
    }

    /// `Present` of the stream's `frame` (numbered as the forwarding layer's captures are).
    pub(crate) fn present(&mut self, frame: u32) {
        let now = std::time::Instant::now();
        if let Some(t) = self.last_present {
            self.made.longest_frame = self.made.longest_frame.max(now - t);
        }
        self.last_present = Some(now);
        // Listed pipelines whose shaders the game has now made, made before its next frame.
        self.prepare_listed();
        self.end_pass();
        self.presents += 1;
        let frame = u64::from(frame);
        let back = self.back_buffers.first().and_then(|&h| self.surface_of(h));
        let shot = match (&mut self.shots, &back) {
            (Some(s), Some(_)) => s.take(frame),
            _ => false,
        };
        let mut readback = None;
        if let (true, Some(b)) = (shot, &back) {
            let tex = b.tex.clone().expect("the back buffer's texture");
            let row = (b.width * 4).div_ceil(256) * 256;
            let buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("shot"),
                size: u64::from(row * b.height),
                usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                mapped_at_creation: false,
            });
            self.encoder().copy_texture_to_buffer(
                wgpu::TexelCopyTextureInfo {
                    texture: &tex.texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                wgpu::TexelCopyBufferInfo {
                    buffer: &buffer,
                    layout: wgpu::TexelCopyBufferLayout {
                        offset: 0,
                        bytes_per_row: Some(row),
                        rows_per_image: Some(b.height),
                    },
                },
                wgpu::Extent3d {
                    width: b.width,
                    height: b.height,
                    depth_or_array_layers: 1,
                },
            );
            readback = Some((buffer, row, b.width, b.height, b.format));
        }
        let mut frame_texture = None;
        let mut window_shot = None;
        // The window's frame at its size now ([`crate::window`]); none while it is minimised,
        // gives none in time or is lost (the frame is skipped, as Direct3D 9's Present then shows
        // nothing, and the game goes on).
        let taken = match (&mut self.surface, &back) {
            (Some(w), Some(_)) => w.frame(&self.device).map(|t| (t, w.config.usage)),
            _ => None,
        };
        if let (Some((t, usage)), Some(b)) = (taken, &back) {
            // The frame's own size and format, as the swap chain gave it.
            let (format, cw, ch) = (t.texture.format(), t.texture.width(), t.texture.height());
            let view = t.texture.create_view(&Default::default());
            let copyable = usage.contains(wgpu::TextureUsages::COPY_SRC);
            let sv = b.view.clone().expect("the back buffer's view");
            let size = (b.width, b.height);
            // Over the whole client area, as Direct3D 9's windowed Present without
            // rectangles: a copy at the same size, filtered when the window's differs.
            self.blit_into(
                &sv,
                size,
                [0.0, 0.0, size.0 as f32, size.1 as f32],
                &view,
                (format, false),
                [0.0, 0.0, cw as f32, ch as f32],
                size != (cw, ch),
            );
            // With a capture, the frame the window gets too (`wgpu-window-N.bmp`): the
            // swapchain's own texture, read back before it is presented.
            if shot && copyable && format == wgpu::TextureFormat::Bgra8Unorm {
                let row = (cw * 4).div_ceil(256) * 256;
                let buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("window shot"),
                    size: u64::from(row * ch),
                    usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                    mapped_at_creation: false,
                });
                self.encoder().copy_texture_to_buffer(
                    t.texture.as_image_copy(),
                    wgpu::TexelCopyBufferInfo {
                        buffer: &buffer,
                        layout: wgpu::TexelCopyBufferLayout {
                            offset: 0,
                            bytes_per_row: Some(row),
                            rows_per_image: Some(ch),
                        },
                    },
                    wgpu::Extent3d {
                        width: cw,
                        height: ch,
                        depth_or_array_layers: 1,
                    },
                );
                window_shot = Some((buffer, row, cw, ch));
            }
            frame_texture = Some(t);
        }
        self.flush();
        if let Some(t) = frame_texture {
            self.queue.present(t);
        }
        if let Some((buffer, row, width, height)) = window_shot {
            buffer.slice(..).map_async(wgpu::MapMode::Read, |_| {});
            let _ = self.device.poll(wgpu::PollType::wait_indefinitely());
            let mut pixels = Vec::with_capacity((width * height * 4) as usize);
            if let Ok(data) = buffer.slice(..).get_mapped_range() {
                for y in 0..height {
                    let r = &data[(y * row) as usize..(y * row + width * 4) as usize];
                    pixels.extend_from_slice(r);
                }
            }
            buffer.unmap();
            if let (Some(s), true) = (&self.shots, pixels.len() == (width * height * 4) as usize) {
                let path = s.dir.join(format!("wgpu-window-{frame:06}.bmp"));
                if let Err(e) = std::fs::write(&path, shot::bmp(width, height, &pixels)) {
                    eprintln!("NV_SHOT wgpu: {} not written: {e}", path.display());
                }
            }
        }
        if let Some((buffer, row, width, height, format)) = readback {
            buffer.slice(..).map_async(wgpu::MapMode::Read, |_| {});
            let _ = self.device.poll(wgpu::PollType::wait_indefinitely());
            let mut pixels = Vec::with_capacity((width * height * 4) as usize);
            match buffer.slice(..).get_mapped_range() {
                Ok(data) => {
                    for y in 0..height {
                        let r = &data[(y * row) as usize..(y * row + width * 4) as usize];
                        pixels.extend_from_slice(r);
                    }
                }
                Err(e) => eprintln!("NV_SHOT wgpu: frame {frame} not read back: {e}"),
            }
            buffer.unmap();
            if format.wgpu == wgpu::TextureFormat::Rgba8Unorm {
                for p in pixels.chunks_exact_mut(4) {
                    p.swap(0, 2);
                }
            }
            if let (Some(s), true) = (&self.shots, pixels.len() == (width * height * 4) as usize) {
                let path = s.path(frame);
                if let Err(e) = std::fs::write(&path, shot::bmp(width, height, &pixels)) {
                    eprintln!("NV_SHOT wgpu: {} not written: {e}", path.display());
                }
            }
        }
        // Bind groups of textures destroyed this frame dropped.
        self.purge_destroyed();
        // The counts the GPU has now posted for the game's threads.
        self.post_answers();
        if let Some(c) = &mut self.pipeline_cache {
            c.save_if_due(self.presents, false);
        }
        if let Some(l) = &mut self.list {
            l.save_if_due(self.presents, false);
        }
        if self.presents % 600 == 1 {
            // The bytes kept for read-backs (`ReadSurface`), printed for their memory cost.
            let kept: usize = self.kept.values().map(Vec::len).sum();
            eprintln!(
                "NV_D3D9 wgpu: present {frame}: {} draws, {} skipped, {} pipelines, {} bind groups, {} wgpu errors, {:.1} MB kept for read-backs ({} levels){}",
                self.draws,
                self.skipped,
                self.pipelines.len(),
                self.groups.len(),
                self.errors.load(Ordering::Relaxed),
                kept as f64 / 1_048_576.0,
                self.kept.values().filter(|k| !k.is_empty()).count(),
                self.surface.as_ref().map_or(String::new(), |w| format!(
                    ", the window's swap chain configured {} times ({} refused)",
                    w.configures, w.refused
                ))
            );
            self.report_made();
        }
    }

    /// The pipeline report (with the present counters, and once more when the device goes): the
    /// game's pipelines made on the draws that needed them since the last report (the backend's
    /// own few for clears, copies and counts are not counted, nor kept in the pipeline cache), what
    /// that took, and the longest time between two presents on this thread.
    fn report_made(&mut self) {
        let m = std::mem::take(&mut self.made);
        let ms = |d: std::time::Duration| d.as_secs_f64() * 1000.0;
        eprintln!(
            "NV_D3D9 wgpu: made since the last report: {} pipelines ({} shader modules) in {:.0} ms on the draws that needed them (of it converting {:.0}, modules {:.0}, pipelines {:.0}); the longest pipeline {:.0} ms, {} pipelines over 16 ms; the longest time between two presents here {:.0} ms",
            m.pipelines,
            m.modules,
            ms(m.whole),
            ms(m.convert),
            ms(m.module),
            ms(m.pipeline),
            ms(m.longest),
            m.over_16ms,
            ms(m.longest_frame)
        );
    }

    /// What the device made of its pipeline cache file ("N KB read", "none yet...", "...: ignored...");
    /// None without a cache (no folder, or a backend without one). Tests.
    pub fn pipeline_cache_note(&self) -> Option<String> {
        self.pipeline_cache.as_ref().map(|c| c.note.clone())
    }

    /// The errors wgpu reported so far (a refused pass or submit among them; tests).
    pub fn wgpu_errors(&self) -> u64 {
        self.errors.load(Ordering::Relaxed)
    }

    /// The adapter and its graphics API, as the log names them ("NVIDIA ... (Vulkan)"; tests).
    pub fn adapter_name(&self) -> &str {
        &self.adapter_name
    }

    /// Loses the device, as a driver reset (TDR) or a removed graphics card does: from now on
    /// nothing reaches the GPU and nothing is read back from it (tests).
    pub fn lose(&self) {
        self.device.destroy();
    }

    /// The window's swap chain asked in `format` from the next frame on, on a surface made anew
    /// and never configured with `new_surface` (as after a lost one); a format the surface does not
    /// offer is refused by wgpu, as a configuration a driver refuses (tests).
    pub fn reconfigure_window_for_tests(&mut self, format: wgpu::TextureFormat, new_surface: bool) {
        if let Some(w) = &mut self.surface {
            w.retarget_for_tests(format, new_surface);
        }
    }

    /// The screen of [`Device::show_progress`] drawn into a `width` by `height` texture of its own,
    /// its pixels B, G, R, A rows packed (tests: an offscreen device has no window to show it in).
    pub fn progress_pixels(&mut self, width: u32, height: u32, p: &loading::Progress) -> Vec<u8> {
        let format = wgpu::TextureFormat::Bgra8Unorm;
        let tex = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("loading screen test"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let view = tex.create_view(&Default::default());
        let mut screen = loading::Screen::new(&self.device, &self.queue, format);
        screen.draw(&self.device, &self.queue, &view, (width, height), p);
        let row = (width * 4).div_ceil(256) * 256;
        let buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: u64::from(row * height),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut encoder = self.device.create_command_encoder(&Default::default());
        encoder.copy_texture_to_buffer(
            tex.as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(row),
                    rows_per_image: Some(height),
                },
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );
        self.queue.submit([encoder.finish()]);
        buffer.slice(..).map_async(wgpu::MapMode::Read, |_| {});
        let _ = self.device.poll(wgpu::PollType::wait_indefinitely());
        let mut out = Vec::with_capacity((width * height * 4) as usize);
        if let Ok(data) = buffer.slice(..).get_mapped_range() {
            for y in 0..height {
                out.extend_from_slice(&data[(y * row) as usize..(y * row + width * 4) as usize]);
            }
        }
        buffer.unmap();
        out
    }

    /// How many pipelines were made on the draw that needed them, not ahead from the list (tests).
    pub fn pipelines_made_on_draws(&self) -> u64 {
        self.made_on_draws
    }

    /// The pixels of a back buffer or render target as wgpu holds them, rows packed (tests).
    pub fn read_surface_pixels(&mut self, surface: Handle) -> Option<(u32, u32, Vec<u8>)> {
        let s = self.surface_of(surface)?;
        let pixels = self.read_level(&s)?;
        Some((s.width, s.height, pixels))
    }

    pub(crate) fn reset(&mut self, pp: &PresentParameters, back_buffers: &[Handle], depth: Handle) {
        self.flush();
        // The swap chain stays the window's size; configured again before the next frame, in the
        // new present interval's mode.
        if let Some(w) = &mut self.surface {
            w.reset(pp.interval);
        }
        // Reset returns the device to its defaults: every state, binding and register; the caches
        // that hold old resources go.
        let st = &mut self.st;
        st.rt = Default::default();
        st.ds = None;
        st.rs = state::default_render_states(pp.auto_depth);
        *st.samplers = [state::default_sampler_states(); 20];
        st.textures = Default::default();
        st.vs = None;
        st.ps = None;
        st.decl = None;
        st.streams = Default::default();
        st.indices = None;
        *st.vsf = [0; 4096];
        st.vsi = [[0; 4]; 16];
        st.vsb = [0; 16];
        *st.psf = [0; 4096];
        st.psi = [[0; 4]; 16];
        st.psb = [0; 16];
        st.clip = [[0; 4]; wgsl::CLIP_PLANES];
        st.dirty = [true; 2];
        st.header_dirty = [true; 2];
        st.bound = Bound::default();
        self.counting = None;
        self.groups.clear();
        self.depth_parts.clear();
        for h in std::mem::take(&mut self.back_buffers) {
            self.objects.remove(&h);
        }
        if self.auto_depth != 0 {
            self.objects.remove(&self.auto_depth);
        }
        self.make_back_buffers(pp, back_buffers, depth);
    }
}

/// The bytes of a register block before its float constants (`Regs` up to `c`).
const HEADER: usize = wgsl::REGS_C_OFFSET as usize;

/// One stage's block header (`Regs` of the converted shaders up to `c`): the integer and boolean
/// registers, the system values, each sampler's fill mode and LOD bias, the clip planes.
fn block_header(
    i: &[[u32; 4]; 16],
    b: &[u32; 16],
    sys: [f32; 4],
    (fix, bias): ([u32; 16], [f32; 16]),
    clip: &[[u32; 4]; wgsl::CLIP_PLANES],
) -> [u8; HEADER] {
    let words = i
        .iter()
        .flatten()
        .copied()
        .chain(b.iter().copied())
        .chain(sys.iter().map(|v| v.to_bits()))
        .chain([0u32; 12])
        .chain(fix)
        .chain(bias.iter().map(|v| v.to_bits()))
        .chain(clip.iter().flatten().copied());
    let mut out = [0u8; HEADER];
    let mut n = 0;
    for (d, w) in out.chunks_exact_mut(4).zip(words) {
        d.copy_from_slice(&w.to_le_bytes());
        n += 4;
    }
    debug_assert_eq!(n, HEADER);
    out
}

/// One stage's register block at the end of the ring: its header ([`block_header`]), then the
/// float registers' bytes `f` (those the stage's shader reads: the rest of the block's range is
/// never read). Its offset; the ring grows by the block rounded up to the offset alignment.
fn write_block(ring: &mut Vec<u8>, header: &[u8; HEADER], f: &[u8]) -> u32 {
    let at = ring.len();
    ring.extend_from_slice(header);
    ring.extend_from_slice(f);
    ring.resize(at + (HEADER + f.len()).div_ceil(256) * 256, 0);
    at as u32
}

/// The alpha test's two bounds for the reference `aref` (0..=255), in steps of 1/4096: the
/// pixel's alpha `a` taken as `x = floor(a * 4096)` is above the reference when `x > above`
/// and at or above it when `x > from`. So the reference machine's alpha test answers (measured
/// on the real Direct3D 9 here, DESIGN.md section 11: the exact alpha at which GREATER,
/// GREATEREQUAL, LESS and LESSEQUAL change their answer, for every reference, 1,020 of 1,020):
/// as if the alpha were rounded to 8 bits, with the halfway points `(aref + 0.5) / 255` cut to
/// 12 bits. `from` is -1 for reference 0 (every alpha is at or above it). What was measured:
/// GREATER, GREATEREQUAL, LESS and LESSEQUAL, at one sample, on an A8R8G8B8 target, on one
/// NVIDIA card. Not measured: EQUAL and NOTEQUAL (derived here from the same two bounds), and
/// every other target format (the same rule is applied there without a measurement). The game
/// uses only GREATER and ALWAYS in all three recordings.
fn alpha_bounds(aref: u32) -> (f32, f32) {
    let above = |r: u32| ((4096 * (2 * r + 1)) / 510) as f32;
    (above(aref), if aref == 0 { -1.0 } else { above(aref - 1) })
}

/// Whether a multisampled surface of `format` is resolved gamma-corrected ([`GAMMA_RESOLVE`]):
/// the 8-bit colour formats (`D3DFMT_A8R8G8B8` and `X8R8G8B8`, the race menu's head target).
fn gamma_resolved(format: wgpu::TextureFormat) -> bool {
    format == wgpu::TextureFormat::Bgra8Unorm
}

/// The multisample resolve of an 8-bit colour surface as the reference machine's driver does it
/// (NVIDIA's "Antialiasing - Gamma correction", on by default; measured on the real Direct3D 9
/// here, DESIGN.md section 11): each sample's red, green and blue taken out of the sRGB curve,
/// averaged, put back on it; alpha averaged as it is. (One sample of four white on black: 137,
/// where the plain average gives 64. Over 2,080 measured resolves at 4x and 8x this curve gives
/// the driver's byte or one off in 238, at most 1.4 levels.) A16B16G16R16F targets the driver
/// averages plainly (measured: identical to wgpu's hardware resolve). `SAMPLES` is replaced by
/// the sample count.
const GAMMA_RESOLVE: &str = "
@group(0) @binding(0) var src: texture_multisampled_2d<f32>;

@vertex fn vs(@builtin(vertex_index) i: u32) -> @builtin(position) vec4<f32> {
    let uv = vec2<f32>(f32((i << 1u) & 2u), f32(i & 2u));
    return vec4<f32>(uv * 2.0 - 1.0, 0.0, 1.0);
}

fn to_linear(c: vec3<f32>) -> vec3<f32> {
    return select(pow((c + 0.055) / 1.055, vec3<f32>(2.4)), c / 12.92, c <= vec3<f32>(0.04045));
}

fn to_srgb(l: vec3<f32>) -> vec3<f32> {
    return select(1.055 * pow(l, vec3<f32>(1.0 / 2.4)) - 0.055, l * 12.92, l <= vec3<f32>(0.0031308));
}

@fragment fn fs(@builtin(position) p: vec4<f32>) -> @location(0) vec4<f32> {
    let xy = vec2<i32>(p.xy);
    var rgb = vec3<f32>(0.0);
    var a = 0.0;
    for (var s = 0u; s < SAMPLES; s++) {
        let c = textureLoad(src, xy, i32(s));
        rgb += to_linear(saturate(c.rgb));
        a += c.a;
    }
    return vec4<f32>(to_srgb(rgb / f32(SAMPLES)), a / f32(SAMPLES));
}
";

/// The copy of one texture's rectangle into another's (`Present`, `StretchRect`).
const BLIT: &str = "
struct Rect { uv: vec4<f32> }
@group(0) @binding(0) var t: texture_2d<f32>;
@group(0) @binding(1) var s: sampler;
@group(0) @binding(2) var<uniform> r: Rect;
struct V { @builtin(position) pos: vec4<f32>, @location(0) uv: vec2<f32> }
@vertex fn vs(@builtin(vertex_index) i: u32) -> V {
    let c = vec2<f32>(f32((i << 1u) & 2u), f32(i & 2u));
    var v: V;
    v.pos = vec4<f32>(c.x * 2.0 - 1.0, 1.0 - c.y * 2.0, 0.0, 1.0);
    v.uv = mix(r.uv.xy, r.uv.zw, c);
    return v;
}
@fragment fn fs(v: V) -> @location(0) vec4<f32> {
    return textureSample(t, s, v.uv);
}
";

/// The textures an unbound sampler reads: 2D, cube and volume, each (0, 0, 0, 1) (a guess,
/// DESIGN.md).
fn dummies(device: &wgpu::Device, queue: &wgpu::Queue) -> [wgpu::TextureView; 3] {
    let make = |dimension, layers, view_dimension| {
        let t = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("unbound sampler"),
            size: wgpu::Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: layers,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        for layer in 0..layers {
            queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: &t,
                    mip_level: 0,
                    origin: wgpu::Origin3d {
                        x: 0,
                        y: 0,
                        z: layer,
                    },
                    aspect: wgpu::TextureAspect::All,
                },
                &[0, 0, 0, 255],
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(4),
                    rows_per_image: Some(1),
                },
                wgpu::Extent3d {
                    width: 1,
                    height: 1,
                    depth_or_array_layers: 1,
                },
            );
        }
        t.create_view(&wgpu::TextureViewDescriptor {
            dimension: Some(view_dimension),
            ..Default::default()
        })
    };
    [
        make(
            wgpu::TextureDimension::D2,
            1,
            wgpu::TextureViewDimension::D2,
        ),
        make(
            wgpu::TextureDimension::D2,
            6,
            wgpu::TextureViewDimension::Cube,
        ),
        make(
            wgpu::TextureDimension::D3,
            1,
            wgpu::TextureViewDimension::D3,
        ),
    ]
}

/// Tests only ([`fail_devices_for_tests`]): the graphics APIs whose swap chains are refused, as
/// `wgpu::Backends` bits; [`LOST_FOR_TESTS`] for a device lost before its swap chain is made;
/// [`PANIC_FOR_TESTS`] for a panic once a device and its swap chain are made.
static FAIL_FOR_TESTS: AtomicU32 = AtomicU32::new(0);
const PANIC_FOR_TESTS: u32 = 1 << 31;
const LOST_FOR_TESTS: u32 = 1 << 30;

/// Tests only: from now on the swap chains of devices made on the `refused` graphics APIs are
/// asked in a format no window offers (wgpu refuses each); with `lost` every device is destroyed
/// before its swap chain is made (as a driver that loses it there); with `panic` making a device
/// panics once it and its swap chain are made. `(wgpu::Backends::empty(), false, false)` makes
/// them as before.
#[doc(hidden)]
pub fn fail_devices_for_tests(refused: wgpu::Backends, lost: bool, panic: bool) {
    let bits = refused.bits() & !(PANIC_FOR_TESTS | LOST_FOR_TESTS);
    FAIL_FOR_TESTS.store(
        bits | if lost { LOST_FOR_TESTS } else { 0 } | if panic { PANIC_FOR_TESTS } else { 0 },
        Ordering::Relaxed,
    );
}

/// Whether the tests asked for the swap chains of devices on `backend` to be refused.
pub(crate) fn refused_for_tests(backend: wgpu::Backend) -> bool {
    let bits = FAIL_FOR_TESTS.load(Ordering::Relaxed) & !(PANIC_FOR_TESTS | LOST_FOR_TESTS);
    wgpu::Backends::from_bits_truncate(bits).contains(wgpu::Backends::from(backend))
}

/// What [`open`] made on one graphics API's adapter.
struct Opened {
    adapter: wgpu::Adapter,
    device: wgpu::Device,
    queue: wgpu::Queue,
    features: wgpu::Features,
    /// The game's window and its swap chain ([`crate::window`]); none offscreen.
    window: Option<crate::window::Window>,
}

/// The device on `adapter` with the features the backend uses where it has them, and on
/// `surface` (the game's window `window`) its swap chain at the window's size, configured
/// ([`crate::window::Window::new`]; with `copies`, its frames can be copied out:
/// `wgpu-window-N.bmp`); why not, when wgpu refused the device or the swap chain twice, or lost
/// the device while the swap chain was made.
fn open(
    instance: wgpu::Instance,
    surface: Option<wgpu::Surface<'static>>,
    adapter: wgpu::Adapter,
    window: isize,
    pp: &PresentParameters,
    copies: bool,
) -> Result<Opened, String> {
    let wanted = wgpu::Features::TEXTURE_COMPRESSION_BC
        | wgpu::Features::FLOAT32_FILTERABLE
        | wgpu::Features::TEXTURE_FORMAT_16BIT_NORM
        | wgpu::Features::ADDRESS_MODE_CLAMP_TO_BORDER
        | wgpu::Features::DEPTH32FLOAT_STENCIL8
        | wgpu::Features::CLIP_DISTANCES
        | wgpu::Features::FLOAT32_BLENDABLE
        | wgpu::Features::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES
        | wgpu::Features::PIPELINE_CACHE;
    let features = adapter.features() & wanted;
    let (device, queue) = block_on(adapter.request_device(&wgpu::DeviceDescriptor {
        label: Some("d3d9-wgpu"),
        required_features: features,
        required_limits: adapter.limits(),
        memory_hints: wgpu::MemoryHints::Performance,
        experimental_features: Default::default(),
        trace: Default::default(),
    }))
    .map_err(|e| format!("request_device: {e}"))?;
    // A driver can refuse the swap chain in a way wgpu takes for a lost device (Vulkan: out of
    // memory, a lost device or a code wgpu does not know): wgpu gives that to no error scope, so
    // the window's configuration reads as taken. The device's lost callback says it (wgpu calls
    // it inside that configuration, or at the poll below for an earlier loss).
    let lost = Arc::new(std::sync::atomic::AtomicBool::new(false));
    {
        let lost = lost.clone();
        device.set_device_lost_callback(move |_, _| lost.store(true, Ordering::Release));
    }
    if FAIL_FOR_TESTS.load(Ordering::Relaxed) & LOST_FOR_TESTS != 0 {
        device.destroy();
    }
    let window = match surface {
        Some(surface) => Some(crate::window::Window::new(
            window,
            instance,
            adapter.clone(),
            surface,
            &device,
            (pp.width, pp.height),
            pp.interval,
            copies,
        )?),
        None => None,
    };
    let _ = device.poll(wgpu::PollType::Poll);
    if lost.load(Ordering::Acquire) {
        return Err("the device was lost while the window's swap chain was made".into());
    }
    Ok(Opened {
        adapter,
        device,
        queue,
        features,
        window,
    })
}

/// The graphics APIs the device is made on, in the order tried, and whether the software adapter
/// is asked for, by `NV_WGPU_BACKEND`'s value (`None` unset; case and spaces aside):
/// - unset: Vulkan, then Direct3D 12 for a card without a Vulkan driver (Windows; elsewhere the
///   platform's own). Vulkan is the renderer (the maintainer, 2026-11-11: "we are trying to focus
///   solely on wgpu and vulkan, as those brings the most improvements and compatability"). Its
///   pictures differ from the real Direct3D 9's in faint texture filtering far off (NVIDIA filters
///   anisotropically otherwise on Vulkan than on its Direct3D: town1, 10-26% of pixels by a mean
///   of 0.2-0.8 levels against 0.3-0.6% on Direct3D 12; accepted by the lead, 2026-11-10).
/// - `vulkan`, `dx12`, `gl`, `metal`: that one alone.
/// - `software`: the platform's software adapter (WARP on Windows, the one CI's runners have),
///   for checking that the backend does not depend on one card's leniency.
pub(crate) fn apis(asked: Option<&str>) -> (Vec<wgpu::Backends>, bool) {
    let asked = asked.map(|v| v.trim().to_ascii_lowercase());
    let one = match asked.as_deref() {
        Some("vulkan") => Some(wgpu::Backends::VULKAN),
        Some("dx12") => Some(wgpu::Backends::DX12),
        Some("gl") => Some(wgpu::Backends::GL),
        Some("metal") => Some(wgpu::Backends::METAL),
        _ => None,
    };
    let software = asked.as_deref() == Some("software");
    let tries = match one {
        Some(b) => vec![b],
        None if software && cfg!(windows) => vec![wgpu::Backends::DX12, wgpu::Backends::PRIMARY],
        None if cfg!(windows) => vec![wgpu::Backends::VULKAN, wgpu::Backends::DX12],
        None => vec![wgpu::Backends::PRIMARY],
    };
    (tries, software)
}

/// A graphics card wgpu can draw with (not a software adapter) on the APIs `NV_WGPU_BACKEND` allows,
/// in [`apis`]' order: its name and API, or none. `NV_WGPU_BACKEND=software` asks for the software
/// adapter, so it answers that one.
pub fn hardware_adapter() -> Option<String> {
    let (tries, software) = apis(std::env::var("NV_WGPU_BACKEND").ok().as_deref());
    if software {
        return Some("the software adapter (NV_WGPU_BACKEND=software)".into());
    }
    tries.into_iter().find_map(|backends| {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends,
            ..wgpu::InstanceDescriptor::new_without_display_handle()
        });
        block_on(instance.enumerate_adapters(backends))
            .into_iter()
            .map(|a| a.get_info())
            .find(|i| i.device_type != wgpu::DeviceType::Cpu)
            .map(|i| format!("{} ({:?})", i.name, i.backend))
    })
}

/// The surface of a real window (Windows: an `HWND`).
#[cfg(windows)]
pub(crate) fn surface_for(
    instance: &wgpu::Instance,
    window: isize,
) -> Result<wgpu::Surface<'static>, String> {
    use raw_window_handle::{
        RawDisplayHandle, RawWindowHandle, Win32WindowHandle, WindowsDisplayHandle,
    };
    let hwnd = std::num::NonZeroIsize::new(window).ok_or("no window")?;
    let mut handle = Win32WindowHandle::new(hwnd);
    // SAFETY: GetModuleHandleW(null) is this process's own module.
    let module = unsafe { GetModuleHandleW(std::ptr::null()) };
    handle.hinstance = std::num::NonZeroIsize::new(module);
    // SAFETY: the window lives as long as the game's device (the forwarding layer's window
    // thread owns it; it is never destroyed while the game runs).
    unsafe {
        instance
            .create_surface_unsafe(wgpu::SurfaceTargetUnsafe::RawHandle {
                raw_display_handle: Some(RawDisplayHandle::Windows(WindowsDisplayHandle::new())),
                raw_window_handle: RawWindowHandle::Win32(handle),
            })
            .map_err(|e| format!("create_surface: {e}"))
    }
}

#[cfg(windows)]
#[link(name = "kernel32")]
extern "system" {
    fn GetModuleHandleW(name: *const u16) -> isize;
}

#[cfg(not(windows))]
pub(crate) fn surface_for(
    _instance: &wgpu::Instance,
    _window: isize,
) -> Result<wgpu::Surface<'static>, String> {
    Err("windows of other platforms come with the window layer's port".into())
}

/// A vertex module's cache key: the shader, the inputs its declaration gives, the varyings and
/// centroid flags of the pixel shader it feeds, the clip code.
type VKey = (
    u64,
    Vec<wgsl::Input>,
    Vec<wgsl::Semantic>,
    Vec<bool>,
    wgsl::Clip,
);
/// A pixel module's cache key: the shader, its varyings, the occlusion count, the clip code.
type PKey = (u64, Vec<wgsl::Semantic>, bool, wgsl::Clip);

#[derive(Clone, PartialEq, Eq, Hash)]
enum ModKey {
    V(VKey),
    P(PKey),
}

/// A listed pipeline's live key and parts ([`Device::key_of_entry`]).
struct Ready {
    key: PipeKey,
    vs: Arc<Shader>,
    ps: Arc<Shader>,
    link: Arc<VertexLink>,
    topology: wgpu::PrimitiveTopology,
    samplers: Vec<(u32, u8, bool)>,
}

/// The clip code as the list writes it ([`prepare::Entry::clip`]).
fn clip_code(c: wgsl::Clip) -> u8 {
    match c {
        wgsl::Clip::Off => 0,
        wgsl::Clip::Distances => 1,
        wgsl::Clip::Discard { per_sample: false } => 2,
        wgsl::Clip::Discard { per_sample: true } => 3,
    }
}

/// One module to make: its WGSL (or why there is none) and what converting it took.
struct ModuleTask {
    key: ModKey,
    source: Result<String, sm::Error>,
    convert: std::time::Duration,
}

/// Everything a render pipeline is made from, owned, so worker threads can make it.
struct PipelineJob {
    layout: wgpu::PipelineLayout,
    group_layout: wgpu::BindGroupLayout,
    vm: wgpu::ShaderModule,
    pm: wgpu::ShaderModule,
    /// Each stream slot up to the last used: its stride and attributes.
    buffers: Vec<Option<(u64, Vec<wgpu::VertexAttribute>)>>,
    primitive: wgpu::PrimitiveState,
    depth_stencil: Option<wgpu::DepthStencilState>,
    multisample: wgpu::MultisampleState,
    targets: Vec<Option<wgpu::ColorTargetState>>,
}

/// [`Device::show_progress`] on the device's parts (so it can run while other parts are lent out).
fn show_on(
    surface: &mut Option<crate::window::Window>,
    screen: &mut Option<loading::Screen>,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    p: &loading::Progress,
) {
    eprintln!(
        "NV_D3D9 wgpu: {}: {} ({:.0}%)",
        p.title,
        p.status,
        p.done.clamp(0.0, 1.0) * 100.0
    );
    let Some(window) = surface else {
        return;
    };
    // As a `Present` does ([`crate::window`]): the window's frame at its size now, none while it
    // is minimised, gives none in time or is lost (not drawn then).
    let Some(frame) = window.frame(device) else {
        return;
    };
    let config = &window.config;
    let view = frame.texture.create_view(&Default::default());
    // Made again if the window's format changed since (its pipeline is made for one).
    if screen.as_ref().is_some_and(|s| s.format != config.format) {
        *screen = None;
    }
    let screen = screen.get_or_insert_with(|| loading::Screen::new(device, queue, config.format));
    screen.draw(device, queue, &view, (config.width, config.height), p);
    queue.present(frame);
}

/// Makes a module from WGSL: read and checked once, by wgpu (naga's reader and validator, with
/// the device's own capabilities); what it refuses, as text.
fn create_module(device: &wgpu::Device, source: String) -> Result<wgpu::ShaderModule, String> {
    let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
    let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: None,
        source: wgpu::ShaderSource::Wgsl(source.into()),
    });
    match block_on(scope.pop()) {
        None => Ok(module),
        Some(e) => Err(e.to_string()),
    }
}

/// Makes the pipeline `job` describes (with the driver's cache, where there is one).
fn build_pipeline(
    device: &wgpu::Device,
    cache: Option<&wgpu::PipelineCache>,
    job: &PipelineJob,
) -> Result<wgpu::RenderPipeline, String> {
    let buffers: Vec<Option<wgpu::VertexBufferLayout>> = job
        .buffers
        .iter()
        .map(|b| {
            b.as_ref().map(|(stride, a)| wgpu::VertexBufferLayout {
                array_stride: *stride,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: a,
            })
        })
        .collect();
    let desc = wgpu::RenderPipelineDescriptor {
        label: None,
        layout: Some(&job.layout),
        vertex: wgpu::VertexState {
            module: &job.vm,
            entry_point: Some("main"),
            compilation_options: Default::default(),
            buffers: &buffers,
        },
        primitive: job.primitive,
        depth_stencil: job.depth_stencil.clone(),
        multisample: job.multisample,
        fragment: Some(wgpu::FragmentState {
            module: &job.pm,
            entry_point: Some("main"),
            compilation_options: Default::default(),
            targets: &job.targets,
        }),
        multiview_mask: None,
        cache,
    };
    let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
    let pipeline = device.create_render_pipeline(&desc);
    match block_on(scope.pop()) {
        None => Ok(pipeline),
        Some(e) => Err(e.to_string()),
    }
}

/// `f` of each of `items`, on up to `threads` worker threads below normal priority (1: on this
/// thread), the results in the items' order. `tick` runs on this thread meanwhile, about four
/// times a second, with how many are done (the loading screen).
fn on_workers<T: Sync, R: Send>(
    items: &[T],
    threads: usize,
    f: impl Fn(&T) -> R + Sync,
    tick: &mut dyn FnMut(usize),
) -> Vec<R> {
    if threads <= 1 || items.len() <= 1 {
        let mut last: Option<std::time::Instant> = None;
        return items
            .iter()
            .enumerate()
            .map(|(i, item)| {
                if !matches!(last, Some(t) if t.elapsed().as_millis() < 250) {
                    tick(i);
                    last = Some(std::time::Instant::now());
                }
                f(item)
            })
            .collect();
    }
    let next = std::sync::atomic::AtomicUsize::new(0);
    let done = std::sync::atomic::AtomicUsize::new(0);
    let results: Vec<std::sync::Mutex<Option<R>>> =
        items.iter().map(|_| std::sync::Mutex::new(None)).collect();
    std::thread::scope(|s| {
        let mut workers = Vec::new();
        for _ in 0..threads.min(items.len()) {
            workers.push(s.spawn(|| {
                lower_priority();
                loop {
                    let i = next.fetch_add(1, Ordering::Relaxed);
                    let Some(item) = items.get(i) else { break };
                    let r = f(item);
                    *results[i].lock().unwrap_or_else(|e| e.into_inner()) = Some(r);
                    done.fetch_add(1, Ordering::Relaxed);
                }
            }));
        }
        // Until every worker is out (also one that panicked: the scope then passes it on).
        let mut last: Option<std::time::Instant> = None;
        loop {
            let d = done.load(Ordering::Relaxed);
            if d >= items.len() || workers.iter().all(|w| w.is_finished()) {
                break;
            }
            // Four times a second: each call presents (with the display's sync, a frame's wait).
            if !matches!(last, Some(t) if t.elapsed().as_millis() < 250) {
                tick(d);
                last = Some(std::time::Instant::now());
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
    });
    results
        .into_iter()
        .map(|m| {
            m.into_inner()
                .unwrap_or_else(|e| e.into_inner())
                .expect("every item done")
        })
        .collect()
}

/// The worker threads making pipelines ahead run below normal priority, so the game's own
/// threads and the menus keep theirs.
fn lower_priority() {
    #[cfg(windows)]
    {
        #[link(name = "kernel32")]
        extern "system" {
            fn GetCurrentThread() -> isize;
            fn SetThreadPriority(thread: isize, priority: i32) -> i32;
        }
        /// `THREAD_PRIORITY_BELOW_NORMAL`.
        const BELOW_NORMAL: i32 = -1;
        // SAFETY: the current thread's pseudo handle and a documented priority value.
        unsafe {
            SetThreadPriority(GetCurrentThread(), BELOW_NORMAL);
        }
    }
}

/// How many worker threads make pipelines ahead: fewer than the machine's cores (two left for
/// the game's main thread and the render thread), at least one.
fn worker_threads() -> usize {
    std::thread::available_parallelism()
        .map_or(1, |n| n.get())
        .saturating_sub(2)
        .clamp(1, 16)
}

/// Draws an encoder takes before it goes to a finishing thread ([`Device::send_encoder`]).
const SPLIT_DRAWS: u32 = 256;

/// An earlier encoder of the batch: finished, or on its way from a finishing thread.
enum Finishing {
    Ready(wgpu::CommandBuffer),
    Pending(std::sync::mpsc::Receiver<Result<wgpu::CommandBuffer, String>>),
}

/// A job for a finishing thread: the encoder, and where its command buffer goes.
type FinishJob = (
    wgpu::CommandEncoder,
    std::sync::mpsc::SyncSender<Result<wgpu::CommandBuffer, String>>,
);

/// The threads that finish the render thread's encoders ([`Device::send_encoder`]). wgpu records a
/// pass as a list of commands and does all of its checking, resource tracking and the driver's
/// recording when the encoder is finished (most of what wgpu and the driver cost a crowded frame,
/// measure/wgpu/CROWD.md); finishing takes the device's resource lock only for reading, so
/// encoders are finished side by side while the render thread goes on.
struct Finisher {
    jobs: Option<std::sync::mpsc::Sender<FinishJob>>,
    threads: Vec<std::thread::JoinHandle<()>>,
}

impl Finisher {
    /// Two threads where the machine has six or more logical processors, one with four or five,
    /// none below (the encoders are then finished at the submit, as before).
    fn for_this_machine() -> Option<Finisher> {
        let n = std::thread::available_parallelism().map_or(1, |n| n.get());
        let threads = match n {
            6.. => 2,
            4..=5 => 1,
            _ => 0,
        };
        (threads > 0).then(|| Finisher::new(threads))
    }

    fn new(threads: usize) -> Finisher {
        let (jobs, queue) = std::sync::mpsc::channel::<FinishJob>();
        let queue = Arc::new(std::sync::Mutex::new(queue));
        let threads = (0..threads)
            .map(|k| {
                let queue = Arc::clone(&queue);
                std::thread::Builder::new()
                    .name(format!("nv-wgpu-finish {k}"))
                    .spawn(move || loop {
                        let job = queue.lock().unwrap_or_else(|e| e.into_inner()).recv();
                        let Ok((encoder, back)) = job else { break };
                        // The render thread waits for it at the submit (a panic's message too, so
                        // it can say why); gone (the device dropped), the buffer is dropped too.
                        let done = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                            encoder.finish()
                        }))
                        .map_err(|p| {
                            p.downcast_ref::<&str>()
                                .map(|s| s.to_string())
                                .or_else(|| p.downcast_ref::<String>().cloned())
                                .unwrap_or_else(|| "a panic without a message".to_string())
                        });
                        let _ = back.send(done);
                    })
                    .expect("a finishing thread starts")
            })
            .collect();
        Finisher {
            jobs: Some(jobs),
            threads,
        }
    }

    fn send(&self, encoder: wgpu::CommandEncoder) -> Finishing {
        let (back, buffer) = std::sync::mpsc::sync_channel(1);
        match self.jobs.as_ref().map(|j| j.send((encoder, back))) {
            Some(Ok(())) => Finishing::Pending(buffer),
            // The threads are gone: finished here.
            Some(Err(std::sync::mpsc::SendError((encoder, _)))) => {
                Finishing::Ready(encoder.finish())
            }
            None => unreachable!("the jobs' sender lives until the finisher is dropped"),
        }
    }
}

impl Drop for Finisher {
    fn drop(&mut self) {
        self.jobs = None;
        for t in self.threads.drain(..) {
            let _ = t.join();
        }
    }
}

impl Drop for Device {
    /// The pipeline cache written once more when pipelines were made since its last save.
    fn drop(&mut self) {
        self.report_made();
        if let Some(c) = &mut self.pipeline_cache {
            c.save_if_due(self.presents, true);
        }
        if let Some(l) = &mut self.list {
            l.save_if_due(self.presents, true);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wgpu::TextureFormat as F;

    /// A render target and its depth surface, asked for 8x apart, get the same count: the
    /// largest that both (and the rest of the game's multisampled formats and the occlusion
    /// count's target) support.
    #[test]
    fn a_target_and_its_depth_surface_get_one_sample_count() {
        // Colour 8x, depth only up to 4x.
        let depth_4x = |f: F, c: u32| !(f == F::Depth24PlusStencil8 && c > 4);
        assert_eq!(common_sample_count(F::Bgra8Unorm, 8, depth_4x), 4);
        assert_eq!(common_sample_count(F::Depth24PlusStencil8, 8, depth_4x), 4);
        // The other way round: colour 4x, depth 8x.
        let colour_4x = |f: F, c: u32| !(f == F::Bgra8Unorm && c > 4);
        assert_eq!(common_sample_count(F::Bgra8Unorm, 8, colour_4x), 4);
        assert_eq!(common_sample_count(F::Depth24PlusStencil8, 8, colour_4x), 4);
        // Everything supported: as asked; nothing: one sample.
        assert_eq!(common_sample_count(F::Bgra8Unorm, 8, |_, _| true), 8);
        assert_eq!(common_sample_count(F::Bgra8Unorm, 2, |_, _| true), 2);
        assert_eq!(common_sample_count(F::Bgra8Unorm, 8, |_, c| c == 1), 1);
        // The occlusion count's 16-bit target lacks 8x: 4x, so counting still joins the pass.
        let count_4x = |f: F, c: u32| !(f == COUNT_FORMAT_16 && c > 4);
        assert_eq!(common_sample_count(F::Bgra8Unorm, 8, count_4x), 4);
    }

    /// The alpha test's bounds give the alphas at which the real Direct3D 9 here changes its
    /// answers (`probe-1109/k-probe4.log`: the smallest alpha passing GREATER is (bound + 1) / 4096).
    #[test]
    fn the_alpha_bounds_are_the_measured_ones() {
        // Reference: GREATER from x = 9, 25, 266, 2049, 2306 (measured, 255 * alpha - ref =
        // +0.56030, +0.55640, +0.56006, +0.56226, +0.56201).
        for (r, from) in [
            (0, 9.0),
            (1, 25.0),
            (16, 266.0),
            (127, 2049.0),
            (143, 2306.0),
        ] {
            assert_eq!(alpha_bounds(r).0 + 1.0, from, "reference {r}");
            assert_eq!(alpha_bounds(r + 1).1 + 1.0, from, "reference {}", r + 1);
        }
        // 255: nothing is above it; 0: everything is at or above it.
        assert!(alpha_bounds(255).0 >= 4096.0);
        assert!(alpha_bounds(0).1 < 0.0);
    }

    /// Vulkan first, Direct3D 12 for a card without a Vulkan driver; a named API alone; the
    /// software adapter on Direct3D 12 (WARP, CI's) as before.
    #[test]
    fn vulkan_is_tried_first_and_a_named_api_alone() {
        use wgpu::Backends as B;
        if cfg!(windows) {
            assert_eq!(apis(None), (vec![B::VULKAN, B::DX12], false));
            assert_eq!(apis(Some("")), (vec![B::VULKAN, B::DX12], false));
            assert_eq!(apis(Some("bogus")), (vec![B::VULKAN, B::DX12], false));
            assert_eq!(apis(Some("software")), (vec![B::DX12, B::PRIMARY], true));
        }
        assert_eq!(apis(Some("vulkan")), (vec![B::VULKAN], false));
        assert_eq!(apis(Some(" Vulkan ")), (vec![B::VULKAN], false));
        assert_eq!(apis(Some("dx12")), (vec![B::DX12], false));
        assert_eq!(apis(Some("gl")), (vec![B::GL], false));
        assert_eq!(apis(Some("metal")), (vec![B::METAL], false));
    }
}
