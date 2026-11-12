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
    /// A pixel shader whose varyings all have a fixed location ([`wgsl::fixed_slot`]): its
    /// module serves every vertex shader ([`Device::module_tasks`]). Every pixel shader of the
    /// game's packages.
    fixed: bool,
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
    /// The pipeline work on the device's thread since the last present: pipelines made on draws
    /// with their modules, and listed ones made ahead ([`Device::prepare_listed`]).
    work: std::time::Duration,
    /// The most of it one present held, and that present; the presents that held more than
    /// 20 ms of it; the pipelines made on draws that took more than 1 ms. The first-use gate
    /// (measure/wgpu/FAST-LINK.md): none over 20 ms in a present, none over 1 ms on a draw.
    worst_work: std::time::Duration,
    worst_at: u64,
    work_over_20ms: u32,
    over_1ms: u32,
    /// Shaders the game created after its start ([`Device::wait_for_start`]): made in the
    /// background or on the draw that needs them (a mod's, mid-game). One made again with
    /// bytecode the device has makes nothing and is not counted.
    late_shaders: u32,
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
        self.work += all;
        if all.as_millis() > 16 {
            self.over_16ms += 1;
        }
        if all > std::time::Duration::from_millis(1) {
            self.over_1ms += 1;
        }
    }
    /// Present `present` is done: the pipeline work it held counted.
    fn end_present(&mut self, present: u64) {
        let work = std::mem::take(&mut self.work);
        if work > self.worst_work {
            self.worst_work = work;
            self.worst_at = present;
        }
        if work > std::time::Duration::from_millis(20) {
            self.work_over_20ms += 1;
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
        let samples = 1u32 << e.samples.min(31);
        if !self.list.add(e) {
            return;
        }
        // Its line, formatted only for a new entry (the list's last).
        let line = self
            .list
            .entries
            .last()
            .map_or_else(String::new, prepare::Entry::line);
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
    /// The pixel shader's sampler group (bind group 1) and the vertex shader's (2).
    groups: [Option<wgpu::BindGroup>; 2],
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
    /// A pair's sampler layouts by its samplers' key ([`Device::sampler_layout`]): the pixel
    /// shader's group and the vertex shader's, and the pipeline layout of the two.
    #[allow(clippy::type_complexity)]
    sampler_layouts: Map<Vec<(u32, u8, bool)>, ([wgpu::BindGroupLayout; 2], wgpu::PipelineLayout)>,
    /// The pipelines by key (none: refused), each with the plane states its draws were listed
    /// with this run (bit 0 none on, bit 1 one on; [`clip_code`]).
    pipelines: Map<PipeKey, (Option<Pipe>, u8)>,
    /// The last draw's pipeline key and pipeline, each stage's sampler group's key and group
    /// (the pixel shader's first), its vertex link; the vertex links by shader and declaration;
    /// what the open pass was given.
    last_pipeline: Option<(PipeKey, Option<Pipe>, bool)>,
    last_groups: [(GroupKey, Option<wgpu::BindGroup>); 2],
    /// Each stage's empty sampler group (a stage without samplers).
    empty_groups: [Option<wgpu::BindGroup>; 2],
    last_link: Option<((u64, u64), Arc<VertexLink>)>,
    vertex_links: Map<(u64, u64), Arc<VertexLink>>,
    binds: PassBinds,
    /// The driver's compiled pipelines kept between runs ([`cache`]), where the backend has them.
    pipeline_cache: Option<cache::PipelineCacheFile>,
    /// When the last `Present` began ([`Made::longest_frame`]).
    last_present: Option<std::time::Instant>,
    /// The screen of the backend's own long steps ([`Device::show_progress`]), made when first shown.
    loading: Option<loading::Screen>,
    /// The game's start ([`Start`]): until the game draws anything, its black frames show the
    /// loading screen. None offscreen and once the game has drawn.
    start: Option<Start>,
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
    vs_modules: Map<VKey, Option<wgpu::ShaderModule>>,
    ps_modules: Map<PKey, Option<wgpu::ShaderModule>>,
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
    /// Entries the draws offered the pipeline list: each pipeline once for each plane state it
    /// was drawn with this run.
    list_offers: u64,
    /// Shader modules made on the device's thread for a draw or a listed pipeline (not ahead on
    /// the module threads; tests).
    modules_here: u64,
    /// The module threads: each shader's modules made when the game creates it
    /// ([`Device::queue_modules`]); started with the first.
    modules_ahead: Option<crate::ahead::Queue<ModKey, ModuleMade>>,
    /// The modules the module threads made since their queue last ran empty, and their time
    /// (said in one line when it does: [`Device::collect_modules`]); of them, those whose
    /// pipeline library was made too.
    ahead_burst: (u32, std::time::Duration, u32),
    /// Whether each shader's pipeline libraries are made with its modules (FAST-LINK step 3):
    /// where wgpu-hal links render pipelines from libraries (Vulkan with
    /// `VK_EXT_graphics_pipeline_library`; [`Device::library_ahead`]).
    libraries: bool,
    /// The pipeline cache file was read (a warm start, [`cache::PipelineCacheFile::warm`]): only
    /// the shaders the pipeline list names get their libraries made ahead ([`listed_shaders`]);
    /// the rest come from the driver's cache on first use, about a millisecond each.
    ///
    /// [`listed_shaders`]: Device::listed_shaders
    warm_start: bool,
    /// The shaders (bytecode hashes) the pipeline list names (shipped and this player's).
    listed_shaders: HashSet<u64>,
    /// The game's start is over: its first draw or `Present` after its first shaders
    /// ([`Device::wait_for_start`]); shaders created after it are counted as late.
    start_done: bool,
    late_shaders: u64,
    /// How many module jobs the start waited for ([`Device::start_waited_for`]).
    start_jobs: usize,
    /// Libraries are still made ahead: a refused one stops it (said once), so no shader makes
    /// a pipeline for nothing. Shared with the module jobs.
    libraries_ahead: Arc<std::sync::atomic::AtomicBool>,
    /// The stand-in partners those are made with (or why wgpu refused one): a vertex shader for
    /// each set of varyings and clip code ([`wgsl::stand_in_vertex`]), and the pixel shader
    /// ([`wgsl::STAND_IN_PIXEL`]). Kept: wgpu-hal forgets a module's libraries when it goes.
    stand_in_vertex: Map<(Vec<wgsl::Semantic>, wgsl::Clip), Result<wgpu::ShaderModule, String>>,
    stand_in_pixel: Option<Result<wgpu::ShaderModule, String>>,
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
                    // The window on this API failed: the next API is tried. A card on this API
                    // that cannot take the window counts as failed (no software adapter then).
                    Err(e) => {
                        eprintln!("NV_D3D9 wgpu: {backends:?}: {e}");
                        if has_card(&instance, backends) {
                            failed.push(format!("{backends:?}"));
                        }
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
                if surface.is_some() && has_card(&instance, backends) {
                    eprintln!("NV_D3D9 wgpu: {backends:?}: no adapter can show the game's window");
                    failed.push(format!("{backends:?}"));
                }
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
            lost,
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
        let warm_start = pipeline_cache.as_ref().is_some_and(|c| c.warm);
        let listed_shaders: HashSet<u64> = list
            .as_ref()
            .map(|l| l.list.entries.iter().flat_map(|e| [e.vs, e.ps]).collect())
            .unwrap_or_default();
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
        // SAFETY: the hal device is only asked a capability it keeps (no Vulkan call).
        let libraries = unsafe { device.as_hal::<wgpu::hal::api::Vulkan>() }
            .is_some_and(|hal| hal.uses_pipeline_libraries());
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
            last_groups: [(Vec::new(), None), (Vec::new(), None)],
            empty_groups: [None, None],
            last_link: None,
            vertex_links: Map::default(),
            binds: PassBinds::default(),
            made_on_draws: 0,
            list_offers: 0,
            modules_here: 0,
            modules_ahead: None,
            ahead_burst: (0, std::time::Duration::ZERO, 0),
            libraries,
            warm_start,
            listed_shaders,
            start_done: false,
            late_shaders: 0,
            start_jobs: 0,
            libraries_ahead: Arc::new(std::sync::atomic::AtomicBool::new(libraries)),
            stand_in_vertex: Map::default(),
            stand_in_pixel: None,
            pipeline_cache,
            last_present: None,
            loading: None,
            start: None,
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
        // Lost while the rest was made: refused as well (the platform's Direct3D 9 draws).
        if lost.load(Ordering::Acquire) {
            return Err("the device was lost while it was made".into());
        }
        // The loading screen from the start (the maintainer's rule for the port's waits: the
        // logo, "Loading" and the step), until the game's first picture ([`Start`]), with the
        // lines the run prints meanwhile; its bar through the game's start against how long the
        // last start took (a file beside the pipeline cache).
        if d.surface.is_some() {
            let mut start = Start::new(&d.device);
            start.capture = loading::capture_output();
            start.record = cache_dir.map(|c| c.join("start-time.txt"));
            start.expected = start
                .record
                .as_ref()
                .and_then(|f| std::fs::read_to_string(f).ok())
                .and_then(|s| s.trim().parse().ok())
                .map(std::time::Duration::from_millis);
            let p = start.progress();
            d.start = Some(start);
            d.show_progress(&p);
        }
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
                let varyings: Vec<wgsl::Semantic> = sig.inputs.iter().map(|(_, s)| *s).collect();
                let fixed = varyings.iter().all(|s| wgsl::fixed_slot(*s).is_some());
                let s = Arc::new(Shader {
                    program,
                    sig,
                    id,
                    fregs,
                    varyings,
                    fixed,
                    tokens: tokens.to_vec(),
                    stable,
                });
                self.shader_bytecode
                    .entry(stable)
                    .or_default()
                    .push(s.clone());
                self.objects.insert(shader, Obj::Shader(s.clone()));
                self.listed_new = true;
                self.queue_modules(&s);
                if self.start_done {
                    self.made.late_shaders += 1;
                    self.late_shaders += 1;
                }
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
        for last in &mut self.last_groups {
            if holds(&last.0) {
                *last = (Vec::new(), None);
            }
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
            depth: self.usable_depth_with(|s| s.id),
            counting: self.counting.is_some(),
        }
    }

    /// The size and sample count of what the current targets draw into.
    fn target_shape(&mut self) -> (u32, u32, u32) {
        match self.st.rt.iter().flatten().next() {
            Some(s) => (s.width, s.height, s.samples),
            None => self
                .usable_depth_with(|d| (d.width, d.height, d.samples))
                .unwrap_or((1, 1, 1)),
        }
    }

    /// The clip code a draw takes on a target of `samples`: the hardware's clip distances where
    /// the adapter has them, planes on or off (a plane that is off is zero and cuts nothing), so
    /// a shader's modules and pipelines do not depend on the planes; else none without a plane
    /// on, and the pixel shader's discard with one, per sample on a multisampled target, which
    /// is where the hardware cuts.
    fn clip_link(&self, samples: u32) -> wgsl::Clip {
        if self.clip_distances {
            return wgsl::Clip::Distances;
        }
        if !self.planes_on() {
            wgsl::Clip::Off
        } else {
            wgsl::Clip::Discard {
                per_sample: samples > 1,
            }
        }
    }

    /// Whether a user clip plane is on (`D3DRS_CLIPPING` and a bit of `D3DRS_CLIPPLANEENABLE`).
    fn planes_on(&self) -> bool {
        self.st.rs[RS_CLIPPING] != 0
            && self.st.rs[rs::CLIPPLANEENABLE] & ((1 << wgsl::CLIP_PLANES) - 1) != 0
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
        self.usable_depth_with(Arc::clone)
    }

    /// `f` of [`Device::usable_depth`]'s surface, read in place: each draw asks two or three
    /// times, and no reference to a surface is taken and dropped for it.
    fn usable_depth_with<R>(&mut self, f: impl FnOnce(&Arc<Surf>) -> R) -> Option<R> {
        let ds = self.st.ds.as_ref()?;
        let Some(rt) = self.st.rt.iter().flatten().next() else {
            return Some(f(ds));
        };
        if rt.samples != ds.samples {
            return None;
        }
        if rt.width == ds.width && rt.height == ds.height {
            return Some(f(ds));
        }
        if rt.width > ds.width || rt.height > ds.height {
            return None;
        }
        let key = (ds.id, rt.width, rt.height);
        if let Some(s) = self.depth_parts.get(&key) {
            return Some(f(s));
        }
        let (format, samples) = (ds.format.d3d, ds.samples);
        let tex = self.new_texture(key.1, key.2, 1, format, false, false, samples)?;
        let s = self.level_surface(&tex, 0, 0);
        self.depth_parts.insert(key, s.clone());
        Some(f(&s))
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

    /// A `D3DDECLTYPE`'s vertex format, and how the vertex shader takes it: by its kind only
    /// (float, unsigned, signed), always four components. A format with fewer is filled with
    /// 0, 0 and 1 by the hardware (Vulkan's and Direct3D 12's vertex input rule), which is
    /// Direct3D 9's own expansion of each type (`FLOAT2` to (x, y, 0, 1), `SHORT2` likewise), and
    /// wgpu matches a format to the shader by kind alone; so a vertex shader has one module for
    /// every declaration of the same kinds (FAST-LINK step 1A).
    fn input_of(ty: u8) -> Option<(wgpu::VertexFormat, wgsl::Input)> {
        use wgpu::VertexFormat as V;
        use wgsl::Input as I;
        Some(match ty {
            0 => (V::Float32, I::Float(4)),
            1 => (V::Float32x2, I::Float(4)),
            2 => (V::Float32x3, I::Float(4)),
            3 => (V::Float32x4, I::Float(4)),
            4 => (V::Unorm8x4Bgra, I::Float(4)),
            5 => (V::Uint8x4, I::Uint4),
            6 => (V::Sint16x2, I::Sint(4)),
            7 => (V::Sint16x4, I::Sint(4)),
            8 => (V::Unorm8x4, I::Float(4)),
            9 => (V::Snorm16x2, I::Float(4)),
            10 => (V::Snorm16x4, I::Float(4)),
            11 => (V::Unorm16x2, I::Float(4)),
            12 => (V::Unorm16x4, I::Float(4)),
            15 => (V::Float16x2, I::Float(4)),
            16 => (V::Float16x4, I::Float(4)),
            _ => return None,
        })
    }

    /// The bind group of one stage's samplers (`k`: 0 the pixel shader's, 1 the vertex
    /// shader's) as `key` and `samplers` give them: the last draw's again, else the cache's, else
    /// a new one in `layout`, each binding numbered within the stage's group.
    fn stage_group(
        &mut self,
        k: usize,
        key: &[(u32, u8, u64, Option<SamplerKey>)],
        samplers: &[((u32, u8, bool), usize)],
        copies: &[Option<Arc<Tex>>; 20],
    ) {
        if let (last, Some(_)) = &self.last_groups[k] {
            if last.as_slice() == key {
                return;
            }
        }
        let group = if key.is_empty() {
            // A stage without samplers (most vertex shaders): one empty group, made once.
            if self.empty_groups[k].is_none() {
                self.empty_groups[k] =
                    Some(self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                        label: None,
                        layout: draw_layout(&self.last_pipeline, k),
                        entries: &[],
                    }));
            }
            self.empty_groups[k].clone().expect("made above")
        } else if let Some(g) = self.groups.get(key) {
            g.clone()
        } else {
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
                views.push((binding % 32, view));
                samps.push((binding % 32 + 1, self.sampler(sk)));
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
                layout: draw_layout(&self.last_pipeline, k),
                entries: &entries,
            });
            self.groups.insert(key.to_vec(), group.clone());
            group
        };
        self.last_groups[k].0.clear();
        self.last_groups[k].0.extend_from_slice(key);
        self.last_groups[k].1 = Some(group);
    }

    /// The pipeline layout of a pair whose samplers are `key` ([`Device::sampler_bindings`]):
    /// the registers' group (0), the pixel shader's samplers' (1) and the vertex shader's (2),
    /// each stage's numbered from 0 in its own group ([`wgsl::sampler_group`]).
    fn sampler_layout(&mut self, key: &[(u32, u8, bool)]) -> wgpu::PipelineLayout {
        if let Some((_, p)) = self.sampler_layouts.get(key) {
            return p.clone();
        }
        let layouts = [false, true].map(|vertex| {
            let mut entries = Vec::new();
            for &(binding, dim, filterable) in key.iter().filter(|k| (k.0 >= 32) == vertex) {
                let binding = binding % 32;
                let visibility = if vertex {
                    wgpu::ShaderStages::VERTEX
                } else {
                    wgpu::ShaderStages::FRAGMENT
                };
                entries.push(wgpu::BindGroupLayoutEntry {
                    binding,
                    visibility,
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
                    visibility,
                    ty: wgpu::BindingType::Sampler(if filterable {
                        wgpu::SamplerBindingType::Filtering
                    } else {
                        wgpu::SamplerBindingType::NonFiltering
                    }),
                    count: None,
                });
            }
            self.device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: None,
                    entries: &entries,
                })
        });
        let pipeline_layout = self
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: None,
                bind_group_layouts: &[
                    Some(&self.uniform_layout),
                    Some(&layouts[0]),
                    Some(&layouts[1]),
                ],
                immediate_size: 0,
            });
        self.sampler_layouts
            .insert(key.to_vec(), (layouts, pipeline_layout.clone()));
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
        // The depth surface's format (wgpu's, Direct3D 9's), samples and size.
        let depth =
            self.usable_depth_with(|d| (d.format.wgpu, d.format.d3d, d.samples, d.width, d.height));
        let atoc = self.atoc();
        let samples = self
            .st
            .rt
            .iter()
            .flatten()
            .next()
            .map(|s| s.samples)
            .or(depth.map(|d| d.2))
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
            depth: depth.map(|d| d.0),
            states: PIPELINE_STATES.map(|s| self.st.rs[s]),
            samples,
            filterable,
            atoc,
            count: self.counting.is_some(),
            clip: self.clip_link(samples),
        };
        // The last draw's pipeline again (most draws), else the cache's, else a new one.
        // With a list, the plane state too: one pipeline with the clip distances serves draws
        // with a plane on and with none, but an adapter without them needs one for each, so each
        // state its draws have is listed ([`clip_code`]).
        let planes = self.list.is_some() && self.planes_on();
        if !matches!(&self.last_pipeline, Some((k, _, on)) if *k == key && *on == planes) {
            // Listed, so the next start makes it before play ([`prepare`]): once for each
            // plane state its draws have (a bit for each beside the pipeline). One lookup
            // when the cache has it.
            let bit = 1u8 << u8::from(planes);
            let (p, unlisted) = match self.pipelines.get_mut(&key) {
                Some((p, m)) => {
                    let new = *m & bit == 0;
                    *m |= bit;
                    (p.clone(), new)
                }
                None => {
                    // Bounded: render states with continuous values (depth bias) could grow
                    // it without end.
                    if self.pipelines.len() >= 4096 {
                        self.pipelines.clear();
                    }
                    let sampler_keys: Vec<(u32, u8, bool)> =
                        samplers.iter().map(|(k, _)| *k).collect();
                    let p = self.make_pipeline(
                        &key,
                        &vs,
                        &ps,
                        &link.inputs,
                        attributes,
                        topology,
                        &sampler_keys,
                    );
                    self.pipelines.insert(key.clone(), (p.clone(), bit));
                    (p, true)
                }
            };
            if unlisted && self.list.is_some() {
                self.list_offers += 1;
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
                    colours: std::array::from_fn(|i| self.st.rt[i].as_ref().map(|s| s.format.d3d)),
                    depth: depth.map(|d| d.1),
                    states: key.states,
                    samples,
                    filterable,
                    atoc,
                    count: key.count,
                    clip: clip_code(key.clip, planes),
                };
                let presents = self.presents;
                if let Some(l) = &mut self.list {
                    l.add(entry, presents);
                }
            }
            self.last_pipeline = Some((key, p, planes));
        }
        // The draw's pipeline and its group layouts are the last pipeline's from here, read in
        // place (no reference to them taken and dropped on each draw).
        if !self
            .last_pipeline
            .as_ref()
            .is_some_and(|(_, p, _)| p.is_some())
        {
            self.skipped += 1;
            return;
        }
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
            let t = match &self.st.textures[slot] {
                Some(t) if self.is_colour_target(t) => t.clone(),
                _ => continue,
            };
            copies[slot] = self.copy_for_sampling(&t);
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
        // Each stage's own group ([`wgsl::sampler_group`]): the vertex shader's samplers come
        // first in the pair's list ([`Device::sampler_bindings`]).
        let vertex = samplers
            .iter()
            .take_while(|((b, _, _), _)| *b >= 32)
            .count();
        self.stage_group(0, &group_key[vertex..], &samplers[vertex..], &copies);
        self.stage_group(1, &group_key[..vertex], &samplers[..vertex], &copies);
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
            .or(depth.map(|d| (d.3, d.4)))
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
                    // Bound already (most draws): nothing to set and no reference taken.
                    if !self.binds.streams[k]
                        .as_ref()
                        .is_some_and(|(x, o)| *x == b.buffer && *o == offset)
                    {
                        streams[k] = Some((b.buffer.clone(), offset));
                    }
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
                    // The buffer to set, None when it is bound already (most draws).
                    let bound = self
                        .binds
                        .index
                        .as_ref()
                        .is_some_and(|(x, i)| *x == b.buffer && *i == b.index32);
                    Some((!bound).then(|| (b.buffer.clone(), b.index32)))
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
        let pipeline = &self
            .last_pipeline
            .as_ref()
            .and_then(|(_, p, _)| p.as_ref())
            .expect("the draw's pipeline, checked above")
            .0;
        if b.pipeline.as_ref() != Some(pipeline) {
            pass.set_pipeline(pipeline);
            b.pipeline = Some(pipeline.clone());
        }
        if b.blocks != Some(blocks) {
            pass.set_bind_group(0, &self.ring_group, &[blocks.0, blocks.1]);
            b.blocks = Some(blocks);
        }
        for (k, (_, group)) in self.last_groups.iter().enumerate() {
            let group = group.as_ref().expect("the draw's groups, set above");
            if b.groups[k].as_ref() != Some(group) {
                pass.set_bind_group(1 + k as u32, group, &[]);
                b.groups[k] = Some(group.clone());
            }
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
            Some(new) => {
                if let Some(i) = new {
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
            // How the planes are cut is this adapter's ([`Device::clip_link`]: with clip distances
            // every draw takes them, planes on or off); without them, planes on or off is the
            // entry's (a list made on one API serves the other).
            clip: if self.clip_distances {
                wgsl::Clip::Distances
            } else if e.clip == 0 {
                wgsl::Clip::Off
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
        let mut list = self.list.take();
        let started = std::time::Instant::now();
        let mut ready = Vec::new();
        let mut ahead = HashSet::new();
        for i in 0..list.as_ref().map_or(0, |l| l.list.entries.len()) {
            let Some(l) = &mut list else { break };
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
        // The game's start (once): every shader's module and library first, the listed
        // pipelines below then only link; one progress bar for both.
        let before = self.wait_for_start(ready.len());
        let Some(l) = list else {
            return;
        };
        if ready.is_empty() {
            self.list = Some(l);
            return;
        }
        let threads = worker_threads();
        // The listed pipelines' modules the module threads have not made yet (the game's
        // shaders, created just before): those still queued are taken back and made on the
        // worker threads here, all at once; those a thread is making are waited for, each alone.
        // The game's other shaders' modules stay with the module threads.
        self.collect_modules();
        let mut wanted = Vec::new();
        let mut taken = Vec::new();
        {
            let mut seen = HashSet::new();
            for r in &ready {
                let (vk, pk) = self.module_keys(&r.key, &r.vs, &r.ps, &r.link.inputs);
                for k in [ModKey::V(vk), ModKey::P(pk)] {
                    if seen.insert(k.clone()) {
                        wanted.push(k);
                    }
                }
            }
            if let Some(q) = &mut self.modules_ahead {
                for k in &wanted {
                    if let Some(job) = q.take_queued(k) {
                        taken.push((k.clone(), std::sync::Mutex::new(Some(job))));
                    }
                }
            }
        }
        // The loading screen while it runs, when it is a long step (the start's; a few pipelines
        // ready later, after a new sample count, are made without it): the modules taken back,
        // then any other the pipelines need, then the pipelines.
        let total = std::cell::Cell::new(before + taken.len() + ready.len());
        let shown = before > 0 || total.get() >= 32;
        let mut surface = self.surface.take();
        let mut screen = self.loading.take();
        let (device, queue) = (self.device.clone(), self.queue.clone());
        let mut shots = self.shots.take();
        let mut step = |done: usize| {
            if shown {
                let p = shader_progress(before + done, total.get());
                show_on(
                    &mut surface,
                    &mut screen,
                    &device,
                    &queue,
                    shots.as_mut(),
                    &p,
                );
            }
        };
        let t = std::time::Instant::now();
        let back = taken.len();
        if back > 0 {
            self.modules_here += back as u64;
            let made = on_workers(
                &taken,
                threads,
                |(_, job)| {
                    let job = job.lock().unwrap_or_else(|e| e.into_inner()).take();
                    job.map(|job| job())
                },
                &mut step,
            );
            for ((k, _), m) in taken.into_iter().zip(made) {
                if let Some(m) = m {
                    self.library_made(&m.library);
                    self.keep_module(k, m.module);
                }
            }
        }
        for k in &wanted {
            self.settle_module(k);
        }
        let mut tasks = Vec::new();
        let mut keys = Vec::new();
        for r in &ready {
            let (vk, pk, t) = self.module_tasks(&r.key, &r.vs, &r.ps, &r.link.inputs);
            tasks.extend(t);
            keys.push((vk, pk));
        }
        let modules = {
            let mut seen = HashSet::new();
            tasks.iter().filter(|t| seen.insert(t.key.clone())).count()
        };
        total.set(total.get() + modules);
        self.make_modules(tasks, threads, false, &mut |d| step(back + d));
        let modules = back + modules;
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
        step(total.get() - before);
        drop(step);
        self.surface = surface;
        self.loading = screen;
        self.shots = shots;
        let mut refused = 0;
        let made = jobs.len();
        for ((key, j), b) in jobs.into_iter().zip(built) {
            match b {
                // No draw has listed it in a plane state yet: the first draw in each state adds
                // that state's line if the list lacks it (a list may name it in one state only:
                // the shipped list was written before both were listed).
                Ok(p) => {
                    self.pipelines.insert(key, (Some((p, j.group_layouts)), 0));
                }
                Err(e) => {
                    refused += 1;
                    self.warn(format!("a pipeline wgpu refuses: {e}"));
                    self.pipelines.insert(key, (None, 0));
                }
            }
        }
        if let Some(c) = &self.pipeline_cache {
            c.touched(self.presents);
        }
        self.made.work += started.elapsed();
        let left = l.done.iter().filter(|&&d| d == 0).count();
        eprintln!(
            "NV_D3D9 wgpu: made ahead: {made} pipelines and {modules} shader modules in {:.0} ms (modules {:.0} ms) on {threads} threads, {refused} refused; {left} listed not yet possible (present {})",
            started.elapsed().as_secs_f64() * 1000.0,
            modules_took.as_secs_f64() * 1000.0,
            self.presents
        );
        self.list = Some(l);
    }

    /// The game's start, its first draw or `Present` after its first shaders: with pipeline
    /// libraries (FAST-LINK step 3), every shader's module and library the module threads have
    /// queued is waited for, behind the loading screen and with the threads at normal priority,
    /// so none is made during play. A library made in the background during play cost frames
    /// up to about 130 ms while the driver made it (d1, driver cold), whatever the number of
    /// threads. Shaders the game creates later are made in the background or on the draw that
    /// needs them, and counted ([`Made::late_shaders`]).
    ///
    /// The start uses every core at normal priority: the jobs no module thread has started are
    /// taken back and made here on [`all_cores`] threads (the game waits at its next frame
    /// anyway), the ones a module thread is doing are waited for with the threads raised. The
    /// loading screen counts them with the `then` listed pipelines that follow, one bar. How
    /// many it waited for (`0`: none).
    fn wait_for_start(&mut self, then: usize) -> usize {
        // Not before the game has a shader (a multisampled target made first asks for this too).
        if self.start_done || self.shader_bytecode.is_empty() {
            return 0;
        }
        self.start_done = true;
        let waiting = self.modules_ahead.as_ref().map_or(0, |q| q.pending());
        if !self.libraries || waiting == 0 {
            return 0;
        }
        self.start_jobs = waiting;
        let started = std::time::Instant::now();
        let total = waiting + then;
        let cores = all_cores();
        let mut surface = self.surface.take();
        let mut screen = self.loading.take();
        let mut shots = self.shots.take();
        let (device, queue) = (self.device.clone(), self.queue.clone());
        let mut show = |done: usize| {
            let p = shader_progress(done, total);
            show_on(
                &mut surface,
                &mut screen,
                &device,
                &queue,
                shots.as_mut(),
                &p,
            );
        };
        let taken: Vec<(
            ModKey,
            std::sync::Mutex<Option<crate::ahead::Job<ModuleMade>>>,
        )> = match &mut self.modules_ahead {
            Some(q) => {
                q.boost(true);
                q.take_all_queued()
                    .into_iter()
                    .map(|(k, job)| (k, std::sync::Mutex::new(Some(job))))
                    .collect()
            }
            None => Vec::new(),
        };
        let made = on_workers(
            &taken,
            cores,
            |(_, job)| {
                let job = job.lock().unwrap_or_else(|e| e.into_inner()).take();
                job.map(|job| job())
            },
            &mut show,
        );
        // Where the threads' time went (the log line below): converting, modules, libraries.
        let mut spent = [std::time::Duration::ZERO; 3];
        let mut libraries = 0usize;
        let mut add = |m: &ModuleMade| {
            spent[0] += m.convert;
            spent[1] += m.made;
            if let Some(l) = &m.library {
                spent[2] += l.0;
                libraries += 1;
            }
        };
        for ((k, _), m) in taken.into_iter().zip(made) {
            if let Some(m) = m {
                add(&m);
                self.library_made(&m.library);
                self.keep_module(k, m.module);
            }
        }
        // The ones the module threads were doing; the loading screen about four times a second.
        let deadline = START_DEADLINE_FOR_TESTS.load(Ordering::Relaxed);
        let mut last: Option<std::time::Instant> = None;
        loop {
            let (done, left) = match &mut self.modules_ahead {
                Some(q) => (q.finished(), q.pending()),
                None => (Vec::new(), 0),
            };
            for (k, m) in done {
                add(&m);
                self.ahead_burst.0 += 1;
                self.ahead_burst.1 += m.convert + m.made;
                self.keep_library(&m.library);
                self.keep_module(k, m.module);
            }
            if !matches!(last, Some(t) if t.elapsed().as_millis() < 250) {
                show(waiting - left);
                last = Some(std::time::Instant::now());
            }
            if left == 0 {
                break;
            }
            assert!(
                deadline == 0 || started.elapsed().as_millis() < u128::from(deadline),
                "the start waited over {deadline} ms (tests)"
            );
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        if let Some(q) = &self.modules_ahead {
            q.boost(false);
        }
        drop(show);
        self.surface = surface;
        self.loading = screen;
        self.shots = shots;
        self.made.work += started.elapsed();
        let ms = |d: std::time::Duration| d.as_secs_f64() * 1000.0;
        eprintln!(
            "NV_D3D9 wgpu: the start: {waiting} shader modules, {libraries} with their pipeline library ({}), {:.0} ms on {cores} cores (threads' time: converting {:.0} ms, modules {:.0} ms, libraries {:.0} ms)",
            if self.warm_start {
                "the pipeline cache warm: the listed shaders'"
            } else {
                "the pipeline cache cold: every shader's"
            },
            ms(started.elapsed()),
            ms(spent[0]),
            ms(spent[1]),
            ms(spent[2])
        );
        waiting
    }

    /// How many shader modules (with their libraries) the game's start waited for (tests).
    pub fn start_waited_for(&self) -> usize {
        self.start_jobs
    }

    /// How many shader modules the module threads have not handed out yet (tests).
    pub fn modules_pending(&self) -> usize {
        self.modules_ahead.as_ref().map_or(0, |q| q.pending())
    }

    /// The module threads' priorities now (`0` normal, `-1` below normal; tests).
    #[cfg(windows)]
    pub fn module_thread_priorities(&self) -> Vec<i32> {
        self.modules_ahead
            .as_ref()
            .map_or_else(Vec::new, |q| q.priorities())
    }

    /// Shaders the game created after its start (tests; the report says how many in each
    /// stretch).
    pub fn shaders_created_after_start(&self) -> u64 {
        self.late_shaders
    }

    /// Queues the modules a draw of shader `s` will most likely take, for the module threads to
    /// make before any draw needs them (FAST-LINK step 1A; the game creates its shaders before
    /// its first frame): a pixel shader's own module where its varyings have fixed locations,
    /// and a vertex shader's for float inputs and every pixel shader that reads only what it
    /// writes; each with the clip code the adapter gives a draw without planes, no occlusion
    /// count. Any other variant is made when a draw first needs it.
    fn queue_modules(&mut self, s: &Arc<Shader>) {
        let clip = if self.clip_distances {
            wgsl::Clip::Distances
        } else {
            wgsl::Clip::Off
        };
        let (key, link) = match s.program.stage {
            Stage::Pixel => {
                if !s.fixed {
                    return;
                }
                let link = wgsl::Link {
                    inputs: Vec::new(),
                    centroid: wgsl::centroid_everywhere(s.varyings.len()),
                    varyings: s.varyings.clone(),
                    clip,
                    count: false,
                    fixed: true,
                };
                let key = ModKey::P((s.id, s.varyings.clone(), false, clip, true));
                (key, link)
            }
            Stage::Vertex => {
                let mut inputs = vec![wgsl::Input::Missing; 16];
                for (r, _) in &s.sig.inputs {
                    if let Some(i) = inputs.get_mut(r.n as usize) {
                        *i = wgsl::Input::Float(4);
                    }
                }
                let varyings = own_varyings(s);
                let link = wgsl::Link {
                    inputs: inputs.clone(),
                    centroid: wgsl::centroid_everywhere(varyings.len()),
                    varyings: varyings.clone(),
                    clip,
                    count: false,
                    fixed: true,
                };
                (ModKey::V((s.id, inputs, varyings, clip, true)), link)
            }
        };
        let made = match &key {
            ModKey::V(k) => self.vs_modules.contains_key(k),
            ModKey::P(k) => self.ps_modules.contains_key(k),
        };
        if made {
            return;
        }
        // Its library too, until the start: after it, a shader's library is made on the draw that
        // first needs it, never in the background during play (the driver held frames up to about
        // 130 ms while making one); none once wgpu refused one.
        // On a warm start only a listed shader's: the others' libraries come from the driver's
        // cache on first use, about a millisecond each (cold, about 100 ms: then all are made).
        let library = if self.libraries
            && !self.start_done
            && self.libraries_ahead.load(Ordering::Relaxed)
            && (!self.warm_start || self.listed_shaders.contains(&s.stable))
        {
            self.library_ahead(s, &link)
        } else {
            None
        };
        let device = self.device.clone();
        let q = self
            .modules_ahead
            .get_or_insert_with(|| crate::ahead::Queue::new(worker_threads(), "nv-wgpu-modules"));
        let ahead = self.libraries_ahead.clone();
        q.push(key, module_job(device, s.clone(), link, library, ahead));
    }

    /// What making `s`'s pipeline library takes, right after its module (made with `link`) on
    /// the module thread (FAST-LINK step 3, DXVK's way): a pipeline of the module and a stand-in
    /// for the other stage, made and dropped. wgpu-hal keeps the shader's library (it depends on
    /// the module, its entry point and the groups its stage sees), so the draws' pipelines with
    /// that module link it, about a command's cost, instead of compiling it. The layout is a
    /// draw's when the shader's textures are filterable (most draws; a draw with another takes a
    /// library of its own). The stand-in's library is made once.
    fn library_ahead(&mut self, s: &Shader, link: &wgsl::Link) -> Option<LibraryAhead> {
        let stage = s.program.stage;
        // The shader's samplers in its stage's group, as [`Device::sampler_bindings`] numbers them.
        let samplers: Vec<(u32, u8, bool)> = s
            .sig
            .samplers
            .iter()
            .filter(|&&(n, _)| match stage {
                Stage::Pixel => n < 16,
                Stage::Vertex => n < 4,
            })
            .map(|&(n, dim)| (wgsl::texture_binding(stage, n), dim_code(dim), true))
            .collect();
        let layout = self.sampler_layout(&samplers);
        let group_layouts = self.sampler_layouts[&samplers[..]].0.clone();
        let target = Some(wgpu::ColorTargetState {
            format: wgpu::TextureFormat::Rgba8Unorm,
            blend: None,
            write_mask: wgpu::ColorWrites::ALL,
        });
        let (partner, buffers, targets, depth_stencil): (Result<_, String>, _, _, _) = match stage {
            Stage::Vertex => {
                let device = &self.device;
                let partner = self
                    .stand_in_pixel
                    .get_or_insert_with(|| create_module(device, wgsl::STAND_IN_PIXEL.into()))
                    .clone();
                // Each input the module reads (all of them, as floats: [`Device::queue_modules`]).
                let attributes: Vec<wgpu::VertexAttribute> = s
                    .sig
                    .inputs
                    .iter()
                    .filter(|(r, _)| r.n < 16)
                    .enumerate()
                    .map(|(i, (r, _))| wgpu::VertexAttribute {
                        format: wgpu::VertexFormat::Float32x4,
                        offset: 16 * i as u64,
                        shader_location: r.n,
                    })
                    .collect();
                let buffers = if attributes.is_empty() {
                    Vec::new()
                } else {
                    vec![Some((16 * attributes.len() as u64, attributes))]
                };
                (partner, buffers, vec![target], None)
            }
            Stage::Pixel => {
                let device = &self.device;
                let partner = self
                    .stand_in_vertex
                    .entry((link.varyings.clone(), link.clip))
                    .or_insert_with(|| create_module(device, wgsl::stand_in_vertex(link)))
                    .clone();
                // A target for each colour it writes (the module writes target 0 when it writes
                // none and no depth), a depth target when it writes depth.
                let mut targets: Vec<Option<wgpu::ColorTargetState>> = (0..4)
                    .map(|k| {
                        if s.sig.colours & (1 << k) != 0 {
                            target.clone()
                        } else {
                            None
                        }
                    })
                    .collect();
                if s.sig.colours == 0 && !s.sig.depth {
                    targets[0] = target;
                }
                while targets.last().is_some_and(|t| t.is_none()) {
                    targets.pop();
                }
                let depth_stencil = s.sig.depth.then(|| wgpu::DepthStencilState {
                    format: wgpu::TextureFormat::Depth32Float,
                    depth_write_enabled: Some(true),
                    depth_compare: Some(wgpu::CompareFunction::Always),
                    stencil: wgpu::StencilState::default(),
                    bias: wgpu::DepthBiasState::default(),
                });
                (partner, Vec::new(), targets, depth_stencil)
            }
        };
        // A stand-in wgpu refused (said once): the draws make the library.
        let partner = match partner {
            Ok(p) => p,
            Err(e) => {
                self.warn(format!("a stand-in shader for pipeline libraries: {e}"));
                return None;
            }
        };
        Some(LibraryAhead {
            layout,
            group_layouts,
            partner,
            buffers,
            targets,
            depth_stencil,
            cache: self.pipeline_cache.as_ref().map(|c| c.cache.clone()),
        })
    }

    /// The modules the module threads have made so far, kept ([`Device::queue_modules`]).
    fn collect_modules(&mut self) {
        let Some(q) = &mut self.modules_ahead else {
            return;
        };
        let done = q.finished();
        let empty = q.pending() == 0;
        for (k, m) in done {
            self.ahead_burst.0 += 1;
            self.ahead_burst.1 += m.convert + m.made;
            self.keep_library(&m.library);
            self.keep_module(k, m.module);
        }
        // One line for each burst (the game's shaders at its start; a mod's later).
        if empty && self.ahead_burst.0 > 0 {
            let (n, t, libraries) = std::mem::take(&mut self.ahead_burst);
            let libraries = if self.libraries {
                format!(", {libraries} with their pipeline library")
            } else {
                String::new()
            };
            eprintln!(
                "NV_D3D9 wgpu: {n} shader modules made ahead on the module threads{libraries} ({:.0} ms of their time; present {})",
                t.as_secs_f64() * 1000.0,
                self.presents
            );
        }
    }

    /// Notes a shader's pipeline library the module threads made with its module
    /// ([`Device::library_ahead`]): its time in this burst's, then as [`Device::library_made`].
    fn keep_library(&mut self, library: &Option<(std::time::Duration, Result<(), String>)>) {
        if let Some((took, made)) = library {
            self.ahead_burst.1 += *took;
            if made.is_ok() {
                self.ahead_burst.2 += 1;
            }
        }
        self.library_made(library);
    }

    /// Notes a shader's pipeline library made with its module, wherever: the driver's cache to
    /// be saved (it holds the library), a pipeline wgpu refused said once (the draws then make
    /// the library).
    fn library_made(&mut self, library: &Option<(std::time::Duration, Result<(), String>)>) {
        match library {
            Some((_, Ok(()))) => {
                if let Some(c) = &self.pipeline_cache {
                    c.touched(self.presents);
                }
            }
            Some((_, Err(e))) => {
                self.libraries_ahead.store(false, Ordering::Relaxed);
                self.warn(format!(
                    "a shader's pipeline library made ahead: {e}; no more are made ahead"
                ));
            }
            None => {}
        }
    }

    /// The module of `k` when the module threads have it: taken back and made here if no thread
    /// has started it, else waited for (that one alone), then kept.
    fn settle_module(&mut self, k: &ModKey) {
        let Some(q) = &mut self.modules_ahead else {
            return;
        };
        if !q.has(k) {
            return;
        }
        match q.take(k) {
            // Its library too, if any: its time is the draw's, as the draw's pipeline would
            // have made it.
            crate::ahead::Taken::Job(job) => {
                let m = job();
                self.modules_here += 1;
                self.made.add_module(m.convert, m.made);
                self.library_made(&m.library);
                self.keep_module(k.clone(), m.module);
            }
            crate::ahead::Taken::Done(m) => {
                self.ahead_burst.0 += 1;
                self.ahead_burst.1 += m.convert + m.made;
                self.keep_library(&m.library);
                self.keep_module(k.clone(), m.module);
            }
            // Its job panicked on a thread: made the usual way.
            crate::ahead::Taken::None => {}
        }
    }

    /// Keeps a module made (none: refused, said once).
    fn keep_module(&mut self, k: ModKey, m: Result<wgpu::ShaderModule, String>) {
        let what = match k {
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
        match k {
            ModKey::V(k) => {
                self.vs_modules.insert(k, m);
            }
            ModKey::P(k) => {
                self.ps_modules.insert(k, m);
            }
        }
    }

    /// The module threads start no module while `on`, then go on (tests: a draw that needs a
    /// module no thread has started takes it back).
    pub fn hold_modules(&mut self, on: bool) {
        self.modules_ahead
            .get_or_insert_with(|| crate::ahead::Queue::new(worker_threads(), "nv-wgpu-modules"))
            .hold(on);
    }

    /// Waits until the module threads have made every module queued, and keeps them (tests).
    pub fn wait_for_modules(&mut self) {
        if let Some(q) = &mut self.modules_ahead {
            let pending = q.wait_all();
            for (k, m) in pending {
                self.ahead_burst.0 += 1;
                self.ahead_burst.1 += m.convert + m.made;
                self.keep_library(&m.library);
                self.keep_module(k, m.module);
            }
        }
    }

    /// Shader modules made on the device's thread (for a draw or a listed pipeline) rather than
    /// ahead on the module threads (tests).
    pub fn modules_made_here(&self) -> u64 {
        self.modules_here
    }

    /// The cache keys of the two modules a pipeline of `key` takes. With fixed varying locations
    /// (a pixel shader whose varyings all have one, [`wgsl::fixed_slot`]), the pixel module is
    /// the shader's own whatever its partner, and the vertex module the shader's own for every
    /// pixel shader that reads only what it writes ([`pair_varyings`]); else both are made for
    /// the pair, as the pixel shader's varyings.
    fn module_keys(
        &self,
        key: &PipeKey,
        vs: &Shader,
        ps: &Shader,
        inputs: &[wgsl::Input],
    ) -> (VKey, PKey) {
        let fixed = ps.fixed;
        let ps_varyings = ps.varyings.clone();
        let vs_varyings = if fixed {
            pair_varyings(vs, ps)
        } else {
            ps_varyings.clone()
        };
        (
            (vs.id, inputs.to_vec(), vs_varyings, key.clip, fixed),
            (ps.id, ps_varyings, key.count, key.clip, fixed),
        )
    }

    /// The modules of a pipeline of `key`, as [`Device::module_keys`] keys them, and the work to
    /// make each one not made yet (its WGSL converted here, the module made by
    /// [`Device::make_modules`]).
    fn module_tasks(
        &mut self,
        key: &PipeKey,
        vs: &Shader,
        ps: &Shader,
        inputs: &[wgsl::Input],
    ) -> (VKey, PKey, Vec<ModuleTask>) {
        let fixed = ps.fixed;
        let (vkey, pkey) = self.module_keys(key, vs, ps, inputs);
        let (vs_varyings, ps_varyings) = (vkey.2.clone(), pkey.1.clone());
        // Made ahead, or being made, on the module threads ([`Device::queue_modules`]).
        self.collect_modules();
        self.settle_module(&ModKey::V(vkey.clone()));
        self.settle_module(&ModKey::P(pkey.clone()));
        let mut tasks = Vec::new();
        if !self.vs_modules.contains_key(&vkey) {
            // Every varying at the centroid, whatever the pixel shader declares: no varying is
            // extrapolated outside a thin triangle on a multisampled target (the white orbs; the
            // converter's module documentation).
            let link = wgsl::Link {
                inputs: inputs.to_vec(),
                centroid: wgsl::centroid_everywhere(vs_varyings.len()),
                varyings: vs_varyings,
                clip: key.clip,
                count: false,
                fixed,
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
                centroid: wgsl::centroid_everywhere(ps_varyings.len()),
                varyings: ps_varyings,
                clip: key.clip,
                count: key.count,
                fixed,
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
        self.modules_here += tasks.len() as u64;
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
        let group_layouts = self.sampler_layouts[samplers].0.clone();
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
            group_layouts,
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
        attributes: &[Vec<wgpu::VertexAttribute>; 8],
        topology: wgpu::PrimitiveTopology,
        samplers: &[(u32, u8, bool)],
    ) -> Option<Pipe> {
        self.made_on_draws += 1;
        let started = std::time::Instant::now();
        let (vkey, pkey, tasks) = self.module_tasks(key, vs, ps, inputs);
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
            Ok(p) => Some((p, job.group_layouts)),
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

    /// The game's start ([`Start`]) in the window's frame `view` (`frame`: its format and size), after
    /// the game's back buffer `back` (`size`) was copied into it: the back buffer's lit pixels
    /// counted into the start's gate, then the loading screen over the frame unless the gate says
    /// the game has shown a picture (all on the GPU: no frame waits).
    fn start_over(
        &mut self,
        back: &wgpu::TextureView,
        size: (u32, u32),
        view: &wgpu::TextureView,
        (format, width, height): (wgpu::TextureFormat, u32, u32),
    ) {
        let Some(mut start) = self.start.take() else {
            return;
        };
        self.end_pass();
        let group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("start"),
            layout: &start.layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(back),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: start.lit.as_entire_binding(),
                },
            ],
        });
        let (device, queue) = (self.device.clone(), self.queue.clone());
        let encoder = self.encoder();
        // This frame's count from zero; the gate itself (word 0) is kept.
        encoder.clear_buffer(&start.lit, 4, Some(4));
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("start"),
                timestamp_writes: None,
            });
            pass.set_bind_group(0, &group, &[]);
            pass.set_pipeline(&start.count);
            pass.dispatch_workgroups(size.0.div_ceil(16), size.1.div_ceil(16), 1);
            pass.set_pipeline(&start.decide);
            pass.dispatch_workgroups(1, 1, 1);
        }
        // Made again if the window's format changed since (its pipeline is made for one).
        let mut screen = match self.loading.take() {
            Some(s) if s.format == format => s,
            _ => loading::Screen::new(&device, &queue, format),
        };
        let p = start.progress();
        let encoder = self.encoder();
        screen.draw(
            &device,
            &queue,
            encoder,
            view,
            (width, height),
            &p,
            Some(&start.lit),
        );
        // What the gate says, for the CPU (read after the frame is sent: [`Start::drawn`]).
        let copy = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("start gate read"),
            size: 4,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        encoder.copy_buffer_to_buffer(&start.lit, 0, &copy, 0, 4);
        start.unsent = Some(copy);
        self.loading = Some(screen);
        self.start = Some(start);
    }

    /// Shows `p` in the game's window ([`loading`]): for a long step of the backend's own before
    /// play, which calls it as it goes. Only a device on the game's window draws it; every call
    /// also writes one log line with the same numbers (for a launcher). The game's own frames
    /// are not touched: the next `Present` shows the game again.
    pub fn show_progress(&mut self, p: &loading::Progress) {
        // Through the game's start the step's share fills the bar's first part only ([`Start`]).
        let mapped;
        let p = match (&mut self.start, p.done) {
            (Some(s), Some(d)) => {
                if d >= 1.0 {
                    s.prepared.get_or_insert_with(std::time::Instant::now);
                }
                mapped = loading::Progress {
                    done: Some(d.min(1.0) * PREPARED),
                    ..p.clone()
                };
                &mapped
            }
            _ => p,
        };
        show_on(
            &mut self.surface,
            &mut self.loading,
            &self.device,
            &self.queue,
            self.shots.as_mut(),
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
        self.made.end_present(self.presents);
        self.collect_modules();
        // The start is over once the game has shown a picture (the GPU's gate already stopped
        // the screen), or at the backstop.
        if let Some(why) = self
            .start
            .as_mut()
            .and_then(|s| s.over(&self.device, self.presents))
        {
            eprintln!("NV_D3D9 wgpu: the start's loading screen ended: {why}");
            if let (Some(s), true) = (self.start.take(), why.starts_with("the game's first")) {
                if let Some(f) = &s.record {
                    let _ = std::fs::write(
                        f,
                        format!("{}\n", s.prepared.unwrap_or(s.began).elapsed().as_millis()),
                    );
                }
            }
        }
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
        // In the window's DPI awareness from the frame's size to its present ([`crate::window`]).
        let _dpi = self.surface.as_ref().map(crate::window::Window::dpi_scope);
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
            // The start: the loading screen over the frame while the game has drawn nothing.
            if self.start.is_some() {
                // The count runs over the back buffer's own texture.
                let tex = b.tex.as_ref().map(|t| &t.texture);
                let full = tex.map_or(size, |t| (t.width(), t.height()));
                self.start_over(&sv, full, &view, (format, cw, ch));
            }
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
        let presents = self.presents;
        if let Some(s) = &mut self.start {
            s.read_after_submit(presents);
        }
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
        eprintln!(
            "NV_D3D9 wgpu: pipeline work since the last report: at most {:.1} ms in one present (present {}), {} presents over 20 ms, {} pipelines made on draws over 1 ms; {} shaders the game created after its start",
            ms(m.worst_work),
            m.worst_at,
            m.work_over_20ms,
            m.over_1ms,
            m.late_shaders
        );
        if let Some(line) = self.pipeline_library_report() {
            eprintln!("NV_D3D9 wgpu: {line}");
        }
    }

    /// The Vulkan backend's pipeline libraries (vendor/wgpu-hal, pipeline_library.rs) in one line,
    /// where it links pipelines from them: what was made since the device was, and what linking
    /// cost. `None` where render pipelines are made whole (another backend, a driver without
    /// libraries, or `NV_WGPU_LIBRARIES=off`).
    pub fn pipeline_library_report(&self) -> Option<String> {
        // SAFETY: the hal device is only read (its counters), on the device's own thread.
        unsafe { self.device.as_hal::<wgpu::hal::api::Vulkan>() }
            .and_then(|hal| hal.pipeline_library_report())
    }

    /// Whether the Vulkan backend links this device's render pipelines from pipeline libraries
    /// ([`Device::pipeline_library_report`]).
    pub fn uses_pipeline_libraries(&self) -> bool {
        // SAFETY: as in `pipeline_library_report`: a capability read on the device's own thread.
        unsafe { self.device.as_hal::<wgpu::hal::api::Vulkan>() }
            .is_some_and(|hal| hal.uses_pipeline_libraries())
    }

    /// What the device made of its pipeline cache file ("N KB read", "none yet...", "...: ignored...");
    /// None without a cache (no folder, or a backend without one). Tests.
    pub fn pipeline_cache_note(&self) -> Option<String> {
        self.pipeline_cache.as_ref().map(|c| c.note.clone())
    }

    /// Whether the start's loading screen still stands in for the game's frames ([`Start`]; tests).
    pub fn starting(&self) -> bool {
        self.start.is_some()
    }

    /// The errors wgpu reported so far (a refused pass or submit among them; tests).
    pub fn wgpu_errors(&self) -> u64 {
        self.errors.load(Ordering::Relaxed)
    }

    /// The adapter and its graphics API, as the log names them ("NVIDIA ... (Vulkan)"; tests).
    pub fn adapter_name(&self) -> &str {
        &self.adapter_name
    }

    /// Whether the adapter has the hardware's clip distances, which every draw then takes
    /// ([`Device::clip_link`]; tests).
    pub fn has_clip_distances(&self) -> bool {
        self.clip_distances
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
        let mut encoder = self.device.create_command_encoder(&Default::default());
        screen.draw(
            &self.device,
            &self.queue,
            &mut encoder,
            &view,
            (width, height),
            p,
            None,
        );
        let row = (width * 4).div_ceil(256) * 256;
        let buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: u64::from(row * height),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
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

    /// How many entries the draws offered the pipeline list: each pipeline once for each plane
    /// state it was drawn with this run, however often (tests).
    pub fn list_offers(&self) -> u64 {
        self.list_offers
    }

    /// How many vertex and pixel shader modules the device has made, not counting any wgpu or
    /// the converter refused (tests).
    pub fn module_counts(&self) -> (usize, usize) {
        let made = |m: &Option<wgpu::ShaderModule>| m.is_some();
        (
            self.vs_modules.values().filter(|m| made(m)).count(),
            self.ps_modules.values().filter(|m| made(m)).count(),
        )
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
    /// Set by the device's lost callback: checked again once the whole device is made.
    lost: Arc<std::sync::atomic::AtomicBool>,
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
    // Lost inside the configuration (the callback has run): refused without a poll, which on a
    // truly lost Vulkan device reads a fence that fails, and wgpu then panics. Otherwise a poll
    // for an earlier loss.
    if !lost.load(Ordering::Acquire) {
        let _ = device.poll(wgpu::PollType::Poll);
    }
    if lost.load(Ordering::Acquire) {
        return Err("the device was lost while the window's swap chain was made".into());
    }
    Ok(Opened {
        adapter,
        device,
        queue,
        features,
        window,
        lost,
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

/// Whether `instance` has a graphics card (not a software adapter) on `backends`.
fn has_card(instance: &wgpu::Instance, backends: wgpu::Backends) -> bool {
    block_on(instance.enumerate_adapters(backends))
        .iter()
        .any(|a| a.get_info().device_type != wgpu::DeviceType::Cpu)
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

/// A pipeline and the layouts of its two sampler groups (the pixel shader's, the vertex
/// shader's).
type Pipe = (wgpu::RenderPipeline, [wgpu::BindGroupLayout; 2]);

/// Stage `k`'s group layout of the draw's pipeline, the last one ([`Device::draw`] makes sure of
/// one before it asks for groups).
fn draw_layout(last: &Option<(PipeKey, Option<Pipe>, bool)>, k: usize) -> &wgpu::BindGroupLayout {
    &last
        .as_ref()
        .and_then(|(_, p, _)| p.as_ref())
        .expect("the draw's pipeline")
        .1[k]
}

/// A vertex module's cache key: the shader, the inputs its declaration gives, the varyings it
/// writes (each at the centroid; [`Device::module_tasks`]), the clip code, whether at fixed
/// locations.
type VKey = (u64, Vec<wgsl::Input>, Vec<wgsl::Semantic>, wgsl::Clip, bool);
/// A pixel module's cache key: the shader, its varyings, the occlusion count, the clip code,
/// whether at fixed locations.
type PKey = (u64, Vec<wgsl::Semantic>, bool, wgsl::Clip, bool);

/// The varyings a vertex shader writes for a pixel shader with fixed varying locations
/// ([`Shader::fixed`]), in location order: every one it writes that has a fixed location (the
/// same for every pixel shader that reads only those), and any the pixel shader reads that it
/// does not write, written with the real Direct3D 9's value for it ([`wgsl::unwritten`]; the pair's
/// own module then).
fn pair_varyings(vs: &Shader, ps: &Shader) -> Vec<wgsl::Semantic> {
    let mut v = own_varyings(vs);
    for s in &ps.varyings {
        if !v.contains(s) {
            v.push(*s);
        }
    }
    v.sort_by_key(|s| wgsl::fixed_slot(*s));
    v.dedup();
    v
}

/// The varyings a vertex shader writes that have a fixed location, in location order: its
/// varyings for every pixel shader that reads only what it writes ([`pair_varyings`]). One of
/// shader model 1 or 2 always writes COLOR0 and COLOR1, as Direct3D 9's value where it does not
/// ([`wgsl::unwritten`]): its two colours are there whatever it writes, so a pixel shader that
/// reads a colour it does not write takes its own module, not one for the pair.
fn own_varyings(vs: &Shader) -> Vec<wgsl::Semantic> {
    let mut v: Vec<wgsl::Semantic> = vs
        .sig
        .outputs
        .iter()
        .map(|(_, s)| *s)
        .filter(|s| wgsl::fixed_slot(*s).is_some())
        .collect();
    if vs.program.major < 3 {
        v.extend((0..2).map(|index| wgsl::Semantic {
            usage: sm::usage::COLOR,
            index,
        }));
    }
    v.sort_by_key(|s| wgsl::fixed_slot(*s));
    v.dedup();
    v
}

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

/// The clip code as the list writes it ([`prepare::Entry::clip`]) for a draw cut as `c`, with a
/// plane on or not (`on`): 0 for none on, also with the clip distances every draw takes where
/// the adapter has them, so a list written there predicts right on an adapter without them.
fn clip_code(c: wgsl::Clip, on: bool) -> u8 {
    match c {
        _ if !on => 0,
        wgsl::Clip::Off => 0,
        wgsl::Clip::Distances => 1,
        wgsl::Clip::Discard { per_sample: false } => 2,
        wgsl::Clip::Discard { per_sample: true } => 3,
    }
}

/// A module made on a module thread ([`Device::queue_modules`]): the module or why there is
/// none, and what converting and making it took; with pipeline libraries, what making the
/// shader's library took and whether wgpu made its pipeline ([`Device::library_ahead`]).
struct ModuleMade {
    module: Result<wgpu::ShaderModule, String>,
    convert: std::time::Duration,
    made: std::time::Duration,
    library: Option<(std::time::Duration, Result<(), String>)>,
}

/// What making a shader's pipeline library takes besides its module ([`Device::library_ahead`]):
/// the pipeline's layout, the stand-in for the other stage, a vertex shader's inputs, a pixel
/// shader's targets.
struct LibraryAhead {
    layout: wgpu::PipelineLayout,
    group_layouts: [wgpu::BindGroupLayout; 2],
    partner: wgpu::ShaderModule,
    buffers: Vec<Option<(u64, Vec<wgpu::VertexAttribute>)>>,
    targets: Vec<Option<wgpu::ColorTargetState>>,
    depth_stencil: Option<wgpu::DepthStencilState>,
    cache: Option<wgpu::PipelineCache>,
}

/// A sampler's dimension as [`Device::sampler_layout`]'s keys code it.
fn dim_code(dim: wgsl::Dim) -> u8 {
    match dim {
        wgsl::Dim::D2 => 0,
        wgsl::Dim::Cube => 1,
        wgsl::Dim::D3 => 2,
    }
}

/// The job of making `program`'s module linked as `link`, on a module thread.
fn module_job(
    device: wgpu::Device,
    shader: Arc<Shader>,
    link: wgsl::Link,
    library: Option<LibraryAhead>,
    ahead: Arc<std::sync::atomic::AtomicBool>,
) -> crate::ahead::Job<ModuleMade> {
    Box::new(move || {
        let delay = MODULE_DELAY_FOR_TESTS.load(Ordering::Relaxed);
        if delay > 0 {
            std::thread::sleep(std::time::Duration::from_millis(u64::from(delay)));
        }
        let t = std::time::Instant::now();
        let source = wgsl::convert(&shader.program, &link);
        let convert = t.elapsed();
        let t = std::time::Instant::now();
        let module = match source {
            Ok(s) => create_module(&device, s),
            Err(e) => Err(e.to_string()),
        };
        let made = t.elapsed();
        // The shader's pipeline library: a pipeline of the module and the stand-in, dropped
        // (wgpu-hal keeps the library).
        // None once one was refused (said once by the device): no pipeline made for nothing.
        let library = match (&module, library) {
            (Ok(m), Some(l)) if ahead.load(Ordering::Relaxed) => {
                let t = std::time::Instant::now();
                let (vm, pm) = match shader.program.stage {
                    Stage::Vertex => (m.clone(), l.partner),
                    Stage::Pixel => (l.partner, m.clone()),
                };
                let job = PipelineJob {
                    layout: l.layout,
                    group_layouts: l.group_layouts,
                    vm,
                    pm,
                    buffers: l.buffers,
                    primitive: wgpu::PrimitiveState::default(),
                    depth_stencil: l.depth_stencil,
                    multisample: wgpu::MultisampleState::default(),
                    targets: l.targets,
                };
                let made = build_pipeline(&device, l.cache.as_ref(), &job).map(drop);
                if made.is_err() {
                    ahead.store(false, Ordering::Relaxed);
                }
                Some((t.elapsed(), made))
            }
            _ => None,
        };
        ModuleMade {
            module,
            convert,
            made,
            library,
        }
    })
}

/// The loading screen's words while the start prepares the shaders: `done` of `total`.
fn shader_progress(done: usize, total: usize) -> loading::Progress {
    loading::Progress {
        title: "Loading".into(),
        status: format!("Preparing shaders {} of {}", done.min(total), total),
        done: Some(done as f32 / total.max(1) as f32),
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
    /// The pixel shader's and the vertex shader's sampler group layouts.
    group_layouts: [wgpu::BindGroupLayout; 2],
    vm: wgpu::ShaderModule,
    pm: wgpu::ShaderModule,
    /// Each stream slot up to the last used: its stride and attributes.
    buffers: Vec<Option<(u64, Vec<wgpu::VertexAttribute>)>>,
    primitive: wgpu::PrimitiveState,
    depth_stencil: Option<wgpu::DepthStencilState>,
    multisample: wgpu::MultisampleState,
    targets: Vec<Option<wgpu::ColorTargetState>>,
}

/// The game's start on a device in the game's window: from the device's creation until the game's
/// first picture, the window shows the loading screen ([`loading`]) instead of the game's black
/// frames. A picture: a frame with at least 1% of its pixels lit (a channel above 24 of 255,
/// [`PICTURE`]). The game's own start is about 5 s of frames that are none: black, then its legal
/// screen fading in from almost black (its first frames have no pixel above 24; the logo stays
/// over them). The game's frames are never held back: each is copied into the window's frame as
/// always; two compute passes then count its lit pixels and open the gate once a frame is a
/// picture (on the GPU); the screen is drawn over the frame only while the gate is shut, its quads
/// taken off the target in the vertex shader once it is open. The gate stays open, so a black
/// frame after the first picture stays black. The CPU reads it a frame or so later
/// ([`Start::over`]) and ends the start, logging the present of the first picture. A backstop
/// ends it all the same [`BACKSTOP_PRESENTS`] presents or [`BACKSTOP`] after the game's first
/// present (the game's own start needs about 8 presents), so that a first picture under the
/// rule (a mod's dark menu; a window minimised through the first pictures) never keeps
/// "Loading" over a playable game.
struct Start {
    count: wgpu::ComputePipeline,
    decide: wgpu::ComputePipeline,
    layout: wgpu::BindGroupLayout,
    /// Two words: the gate (non-zero once a frame was a picture) and this frame's lit pixels.
    lit: wgpu::Buffer,
    /// The copy of `lit` recorded with this frame, mapped once the frame is sent.
    unsent: Option<wgpu::Buffer>,
    /// Copies being mapped: each with its present and its map's state ([`MAP_PENDING`],
    /// [`MAP_DONE`], [`MAP_FAILED`]).
    reading: Vec<(wgpu::Buffer, u64, Arc<std::sync::atomic::AtomicU8>)>,
    /// When the game's first present came, and which it was.
    first: Option<(std::time::Instant, u64)>,
    /// When the device was made, how long the last start took to its first picture, and the
    /// file that keeps it ([`Start::progress`]).
    began: std::time::Instant,
    expected: Option<std::time::Duration>,
    record: Option<std::path::PathBuf>,
    /// When the start's shader preparation ended (its bar full): the time part of the bar, and the
    /// time kept, count from here.
    prepared: Option<std::time::Instant>,
    /// The run's output on the screen while the start lasts ([`loading::capture_output`]).
    capture: Option<loading::Capture>,
}

/// The start ends without a picture this many presents after the game's first ([`Start`]).
/// The share of the start screen's bar the shader preparation fills; the time estimate fills the rest.
const PREPARED: f32 = 0.5;
const BACKSTOP_PRESENTS: u64 = 600;
/// ... or this long after it.
const BACKSTOP: std::time::Duration = std::time::Duration::from_secs(10);
/// A read-back's map: not answered yet, done, or failed (a lost device): then dropped unread.
const MAP_PENDING: u8 = 0;
const MAP_DONE: u8 = 1;
const MAP_FAILED: u8 = 2;

/// A frame's lit pixels (a channel above 24 of 255: plainly not black on a screen) counted into
/// `gate[1]` (per 16x16 block, then all), then the gate (`gate[0]`) opened when they are at least
/// 1% of the frame: a picture.
const PICTURE: &str = "
@group(0) @binding(0) var frame: texture_2d<f32>;
@group(0) @binding(1) var<storage, read_write> gate: array<atomic<u32>, 2>;
var<workgroup> here: atomic<u32>;
@compute @workgroup_size(16, 16)
fn count(@builtin(global_invocation_id) id: vec3<u32>, @builtin(local_invocation_index) i: u32) {
    let size = textureDimensions(frame);
    if (id.x < size.x && id.y < size.y) {
        let c = textureLoad(frame, vec2<i32>(id.xy), 0);
        if (max(c.r, max(c.g, c.b)) * 255.0 > 24.5) {
            atomicAdd(&here, 1u);
        }
    }
    workgroupBarrier();
    if (i == 0u) {
        let n = atomicLoad(&here);
        if (n != 0u) {
            atomicAdd(&gate[1], n);
        }
    }
}
@compute @workgroup_size(1)
fn decide() {
    let size = textureDimensions(frame);
    // 1% of the pixels, rounded up (no product that can overflow).
    if (atomicLoad(&gate[1]) >= (size.x * size.y + 99u) / 100u) {
        atomicMax(&gate[0], 1u);
    }
}
";

impl Start {
    fn new(device: &wgpu::Device) -> Start {
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("start"),
            source: wgpu::ShaderSource::Wgsl(PICTURE.into()),
        });
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("start"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: std::num::NonZeroU64::new(8),
                    },
                    count: None,
                },
            ],
        });
        let pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("start"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let pipeline = |entry: &str| {
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("start"),
                layout: Some(&pl),
                module: &module,
                entry_point: Some(entry),
                compilation_options: Default::default(),
                cache: None,
            })
        };
        let (count, decide) = (pipeline("count"), pipeline("decide"));
        // wgpu gives a new buffer zeros: no picture yet.
        let lit = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("start gate"),
            size: 8,
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_SRC
                | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        Start {
            count,
            decide,
            layout,
            lit,
            unsent: None,
            reading: Vec::new(),
            first: None,
            began: std::time::Instant::now(),
            expected: None,
            record: None,
            prepared: None,
            capture: None,
        }
    }

    /// What the screen says through the start: the bar at the time since the device was made
    /// against the last start's time to its first picture (short of full until it shows), empty
    /// on a first start.
    fn progress(&self) -> loading::Progress {
        // The shader preparation fills the first part ([`PREPARED`], [`Device::show_progress`]);
        // after it, the time since it ended against the last start's time fills the rest.
        let done = match (self.prepared, self.expected) {
            (Some(at), Some(e)) => {
                let t = (at.elapsed().as_secs_f32() / e.as_secs_f32().max(0.001)).min(0.97);
                PREPARED + (1.0 - PREPARED) * t
            }
            (Some(_), None) => PREPARED,
            _ => 0.0,
        };
        let done = Some(done);
        loading::Progress {
            title: "Loading".into(),
            status: "Starting the game".into(),
            done,
        }
    }

    /// The copy recorded with present `present`, just sent, mapped.
    fn read_after_submit(&mut self, present: u64) {
        if let Some(copy) = self.unsent.take() {
            let state = Arc::new(std::sync::atomic::AtomicU8::new(MAP_PENDING));
            let answered = state.clone();
            copy.slice(..).map_async(wgpu::MapMode::Read, move |r| {
                let s = if r.is_ok() { MAP_DONE } else { MAP_FAILED };
                answered.store(s, Ordering::Release);
            });
            self.reading.push((copy, present, state));
        }
    }

    /// At present `present` (counted before it is made): why the start is over, if it is (the
    /// game's first picture, or the backstop); polls `device` without waiting.
    fn over(&mut self, device: &wgpu::Device, present: u64) -> Option<String> {
        if let Some(at) = self.drawn(device) {
            return Some(format!("the game's first picture at present {at}"));
        }
        let (since, first) = *self
            .first
            .get_or_insert((std::time::Instant::now(), present));
        let shown = present.saturating_sub(first);
        (shown >= BACKSTOP_PRESENTS || since.elapsed() >= BACKSTOP).then(|| {
            format!(
                "no picture in {shown} presents and {:.1} s after the game's first: the backstop",
                since.elapsed().as_secs_f64()
            )
        })
    }

    /// The first present whose copy, mapped so far, says the game has shown a picture (polls
    /// `device` without waiting). A copy whose map failed is dropped unread.
    fn drawn(&mut self, device: &wgpu::Device) -> Option<u64> {
        if self.reading.is_empty() {
            return None;
        }
        let _ = device.poll(wgpu::PollType::Poll);
        let mut drawn: Option<u64> = None;
        self.reading.retain(
            |(copy, present, state)| match state.load(Ordering::Acquire) {
                MAP_PENDING => true,
                MAP_DONE => {
                    if let Ok(data) = copy.slice(..).get_mapped_range() {
                        if data.iter().any(|&b| b != 0) {
                            drawn = Some(drawn.map_or(*present, |d| d.min(*present)));
                        }
                    }
                    copy.unmap();
                    false
                }
                _ => false,
            },
        );
        drawn
    }
}

/// [`Device::show_progress`] on the device's parts (so it can run while other parts are lent out);
/// with `shots` (`NV_SHOT`) each frame it shows is also captured in their folder
/// (`wgpu-loading-N.bmp`, N counting these frames from 1).
fn show_on(
    surface: &mut Option<crate::window::Window>,
    screen: &mut Option<loading::Screen>,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    shots: Option<&mut Shots>,
    p: &loading::Progress,
) {
    match p.done {
        Some(done) => eprintln!(
            "NV_D3D9 wgpu: {}: {} ({:.0}%)",
            p.title,
            p.status,
            done.clamp(0.0, 1.0) * 100.0
        ),
        None => eprintln!("NV_D3D9 wgpu: {}: {}", p.title, p.status),
    }
    let Some(window) = surface else {
        return;
    };
    // As a `Present` does ([`crate::window`]): the window's frame at its size now, none while it
    // is minimised, gives none in time or is lost (not drawn then), in the window's DPI awareness
    // to its present.
    let _dpi = window.dpi_scope();
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
    let mut encoder = device.create_command_encoder(&Default::default());
    screen.draw(
        device,
        queue,
        &mut encoder,
        &view,
        (config.width, config.height),
        p,
        None,
    );
    queue.submit([encoder.finish()]);
    if let Some(s) = shots {
        if config.usage.contains(wgpu::TextureUsages::COPY_SRC)
            && config.format == wgpu::TextureFormat::Bgra8Unorm
        {
            s.loading += 1;
            let path = s.dir.join(format!("wgpu-loading-{:06}.bmp", s.loading));
            capture(device, queue, &frame.texture, &path);
        }
    }
    queue.present(frame);
}

/// `texture` (B8G8R8A8, a window's frame before it is presented) read back and written to `path`
/// as an `NV_SHOT` capture.
fn capture(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    texture: &wgpu::Texture,
    path: &std::path::Path,
) {
    let (width, height) = (texture.width(), texture.height());
    let row = (width * 4).div_ceil(256) * 256;
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("loading screen shot"),
        size: u64::from(row * height),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&Default::default());
    encoder.copy_texture_to_buffer(
        texture.as_image_copy(),
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
    queue.submit([encoder.finish()]);
    buffer.slice(..).map_async(wgpu::MapMode::Read, |_| {});
    let _ = device.poll(wgpu::PollType::wait_indefinitely());
    let mut pixels = Vec::with_capacity((width * height * 4) as usize);
    if let Ok(data) = buffer.slice(..).get_mapped_range() {
        for y in 0..height {
            pixels.extend_from_slice(&data[(y * row) as usize..(y * row + width * 4) as usize]);
        }
    }
    buffer.unmap();
    if pixels.len() == (width * height * 4) as usize {
        if let Err(e) = std::fs::write(path, shot::bmp(width, height, &pixels)) {
            eprintln!("NV_SHOT wgpu: {} not written: {e}", path.display());
        }
    }
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

/// `f` of each of `items`, on up to `threads` worker threads (1: on this thread), the results in
/// the items' order. `tick` runs on this thread meanwhile, about four times a second, with how
/// many are done (the loading screen). The threads keep normal priority: the game waits for them
/// (its start, behind the loading screen).
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
            workers.push(s.spawn(|| loop {
                let i = next.fetch_add(1, Ordering::Relaxed);
                let Some(item) = items.get(i) else { break };
                let r = f(item);
                *results[i].lock().unwrap_or_else(|e| e.into_inner()) = Some(r);
                done.fetch_add(1, Ordering::Relaxed);
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

/// How many worker threads make pipelines ahead: fewer than the machine's cores (two left for
/// the game's main thread and the render thread), at least one.
/// Every core (the logical processors), for the game's start while it waits ([`Device::wait_for_start`]).
fn all_cores() -> usize {
    std::thread::available_parallelism().map_or(1, |n| n.get())
}

/// Tests only: each module job waits this many milliseconds before it starts (jobs in flight
/// across the game's start), and the start fails past this many (a wait that would not end).
static MODULE_DELAY_FOR_TESTS: AtomicU32 = AtomicU32::new(0);
static START_DEADLINE_FOR_TESTS: AtomicU32 = AtomicU32::new(0);

/// Tests only: [`MODULE_DELAY_FOR_TESTS`] and [`START_DEADLINE_FOR_TESTS`] in milliseconds
/// (0: none).
#[doc(hidden)]
pub fn module_timing_for_tests(delay_ms: u32, start_deadline_ms: u32) {
    MODULE_DELAY_FOR_TESTS.store(delay_ms, Ordering::Relaxed);
    START_DEADLINE_FOR_TESTS.store(start_deadline_ms, Ordering::Relaxed);
}

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

    /// The list records whether a plane was on, not only how the adapter cut: the clip
    /// distances every draw takes where the adapter has them write 0 with no plane on, so a list
    /// written there predicts the right pipelines on an adapter without them (Direct3D 12).
    #[test]
    fn the_list_records_a_plane_on_whatever_cuts_it() {
        use wgsl::Clip;
        assert_eq!(clip_code(Clip::Distances, false), 0);
        assert_eq!(clip_code(Clip::Distances, true), 1);
        assert_eq!(clip_code(Clip::Off, false), 0);
        assert_eq!(clip_code(Clip::Discard { per_sample: false }, true), 2);
        assert_eq!(clip_code(Clip::Discard { per_sample: true }, true), 3);
    }

    /// The first-use gate's counts: the pipeline work a present held (pipelines made on its draws,
    /// with their modules, and listed ones made ahead), the most in one present and where, the
    /// presents over 20 ms, the pipelines over 1 ms; a present with none counts nothing.
    #[test]
    fn the_pipeline_work_each_present_held_is_counted_for_the_gate() {
        let ms = std::time::Duration::from_millis;
        let mut m = Made::default();
        m.add_pipeline(ms(10), ms(12));
        m.add_pipeline(ms(9), ms(12));
        m.end_present(5);
        m.add_pipeline(ms(0), std::time::Duration::from_micros(500));
        m.end_present(6);
        m.work += ms(21);
        m.end_present(7);
        m.end_present(8);
        assert_eq!(m.worst_work, ms(24));
        assert_eq!(m.worst_at, 5);
        assert_eq!(m.work_over_20ms, 2, "presents 5 and 7");
        assert_eq!(m.over_1ms, 2, "the two 12 ms pipelines");
        assert_eq!(m.work, std::time::Duration::ZERO);
        assert_eq!(m.pipelines, 3);
    }

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
