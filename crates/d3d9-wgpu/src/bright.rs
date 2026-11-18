//! The HDR watch: a diagnostic that reads the game's float render targets (its HDR scene, the
//! luminance down-sample chain, the eye adaptation's targets, the bloom's) on one present in
//! [`EVERY`] and says in the log when one of them goes wrong: pixels that are not finite (an
//! infinity or NaN, which the bloom spreads and the luminance chain can turn into a wrong
//! exposure), lone bright dots in the scene (a pixel over [`DOT_RATIO`] times as bright as every
//! neighbour and over [`DOT_FLOOR`]: the white dots bloom makes into orbs,
//! measure/render-aa/orbs2/verdict), or a small target (the luminance and adaptation targets)
//! whose value jumps by more than [`JUMP`] times or to 0 between two watched presents.
//!
//! The log is bounded over time: an event starts with one line (and its targets, at most
//! [`EVENT_LINES`]), ends after [`CLEAN_TO_END`] clean watched presents in a row with one line,
//! and a new event is printed at most once in [`START_GAP`] presents; the periodic report counts
//! every watched present with a problem. It only reads: the picture is the same with it on or off
//! (`NV_D3D9_HDR_WATCH=off`, which also turns off the device's check of render targets sampled
//! before they are drawn).
//!
//! A target is read when the game leaves it as colour target 0 after drawing into it, or when it
//! is a `StretchRect` destination (a multisampled scene resolved there). Each read is a compute
//! pass into one slot of a kept storage buffer, copied into one of [`RING`] kept read-back buffers
//! (one a watched present), which is mapped once after the present's submit and never waited for;
//! a present finding no free read-back buffer is not watched.

use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::Arc;

/// One present in this many is watched.
pub(crate) const EVERY: u64 = 4;
/// A lone dot is brighter than this (the brightest of red, green and blue)...
pub(crate) const DOT_FLOOR: f32 = 4.0;
/// ...and more than this many times as bright as the brightest of its eight neighbours.
pub(crate) const DOT_RATIO: f32 = 8.0;
/// A small target's value moving by more than this factor between watched presents is a jump.
pub(crate) const JUMP: f32 = 8.0;
/// Targets of at most this many pixels are the small ones whose value is followed.
const SMALL: u32 = 64;
/// Reads in one present at most.
const READS: usize = 32;
/// Read-back buffers, one a watched present on its way.
const RING: usize = 4;
/// A read's place in the storage buffer (the storage offset alignment wgpu guarantees).
const SLOT: u64 = 256;
/// Judged presents kept for [`Watch::recent`].
const RECENT: usize = 16;
/// Target lines for one event at most.
pub(crate) const EVENT_LINES: usize = 6;
/// Clean watched presents in a row that end an event.
pub(crate) const CLEAN_TO_END: u32 = 8;
/// Presents between two printed event starts at least.
pub(crate) const START_GAP: u64 = 600;
/// Bind groups kept (one a target texture) before they are all dropped.
const GROUPS: usize = 256;

const SHADER: &str = "
@group(0) @binding(0) var t: texture_2d<f32>;
@group(0) @binding(1) var<storage, read_write> r: array<atomic<u32>, 4>;
var<workgroup> top: atomic<u32>;
var<workgroup> bad: atomic<u32>;
var<workgroup> dots: atomic<u32>;
fn finite(x: f32) -> bool {
    return (bitcast<u32>(x) & 0x7f800000u) != 0x7f800000u;
}
fn level(c: vec4<f32>) -> f32 {
    return max(max(c.x, c.y), c.z);
}
fn ok(c: vec4<f32>) -> bool {
    return finite(c.x) && finite(c.y) && finite(c.z);
}
@compute @workgroup_size(16, 16)
fn main(@builtin(global_invocation_id) g: vec3<u32>, @builtin(local_invocation_index) i: u32) {
    if (i == 0u) {
        atomicStore(&top, 0u);
        atomicStore(&bad, 0u);
        atomicStore(&dots, 0u);
    }
    workgroupBarrier();
    let size = vec2<i32>(textureDimensions(t));
    let at = vec2<i32>(g.xy);
    if (at.x < size.x && at.y < size.y) {
        let c = textureLoad(t, at, 0);
        if (!ok(c)) {
            atomicAdd(&bad, 1u);
        } else {
            let v = level(c);
            if (v > 0.0) { atomicMax(&top, bitcast<u32>(v)); }
            if (v > FLOOR && size.x > 2 && size.y > 2) {
                var around = 0.0;
                for (var dy = -1; dy <= 1; dy++) {
                    for (var dx = -1; dx <= 1; dx++) {
                        if (dx != 0 || dy != 0) {
                            let n = textureLoad(t, clamp(at + vec2<i32>(dx, dy), vec2<i32>(0), size - 1), 0);
                            if (ok(n)) { around = max(around, level(n)); }
                        }
                    }
                }
                if (v > RATIO * around) { atomicAdd(&dots, 1u); }
            }
        }
    }
    workgroupBarrier();
    if (i == 0u) {
        let b = atomicLoad(&bad);
        if (b != 0u) { atomicAdd(&r[0], b); }
        let d = atomicLoad(&dots);
        if (d != 0u) { atomicAdd(&r[1], d); }
        let m = atomicLoad(&top);
        if (m != 0u) { atomicMax(&r[2], m); }
    }
}
";

/// What one read target held.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Seen {
    /// Pixels with a red, green or blue that is infinite or NaN.
    pub not_finite: u32,
    /// Lone bright dots ([`DOT_FLOOR`], [`DOT_RATIO`]; none in a target under 3x3).
    pub dots: u32,
    /// The brightest finite pixel (the brightest of its red, green and blue; 0 when none is
    /// above 0).
    pub brightest: f32,
}

/// Which target a read was of: the device's id of its texture, its level's size and format.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Target {
    pub id: u64,
    pub width: u32,
    pub height: u32,
    pub format: wgpu::TextureFormat,
}

impl Target {
    fn small(&self) -> bool {
        self.width * self.height <= SMALL
    }
}

/// The watch's totals (since the device was made, or since the last report).
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Totals {
    /// Presents watched (all their reads back), targets read.
    pub presents: u64,
    pub reads: u64,
    /// Watched presents with a target holding pixels that are not finite, with lone bright dots,
    /// with a small target's jump.
    pub with_not_finite: u64,
    pub with_dots: u64,
    pub with_jumps: u64,
    /// Events that began, and of them the ones not printed ([`START_GAP`]).
    pub events: u64,
    pub events_unprinted: u64,
    /// Presents not watched for want of a free read-back buffer.
    pub skipped: u64,
    /// The brightest finite pixel of all the targets read.
    pub brightest: f32,
}

/// A target's problem in one read.
fn problem(seen: &Seen, small: bool, before: Option<f32>) -> Option<String> {
    let mut why = Vec::new();
    if seen.not_finite > 0 {
        why.push(format!("{} pixels not finite", seen.not_finite));
    }
    if seen.dots > 0 {
        why.push(format!("{} lone bright dots", seen.dots));
    }
    if let (true, Some(b)) = (small, before) {
        let now = seen.brightest;
        let jumped = if b > 0.0 && now > 0.0 {
            now > b * JUMP || now * JUMP < b
        } else {
            (b > 0.0) != (now > 0.0)
        };
        if jumped {
            why.push(format!("its value went from {b} to {now}"));
        }
    }
    (!why.is_empty()).then(|| why.join(", "))
}

/// Whether the watch is on for `NV_D3D9_HDR_WATCH`'s value (on unless `off` or `0`).
pub(crate) fn on_for(value: Option<&str>) -> bool {
    !matches!(value, Some("off" | "0"))
}

/// A kept read-back buffer and the watched present using it.
struct Back {
    buffer: wgpu::Buffer,
    /// 0 free, 1 taken by the present being drawn, 2 its map asked, 3 mapped, 4 failed.
    state: Arc<AtomicU8>,
    present: u64,
    reads: Vec<(Target, &'static str)>,
}

/// The GPU objects, made on the first read.
struct Gpu {
    pipeline: wgpu::ComputePipeline,
    layout: wgpu::BindGroupLayout,
    storage: wgpu::Buffer,
    backs: Vec<Back>,
    /// A bind group a target texture, by its id: the surface it was made for (its view) and the
    /// group (that view and the storage buffer at a dynamic offset).
    groups: HashMap<u64, (u64, wgpu::BindGroup)>,
}

/// The event being followed: the present it began in, the targets named for it (at most
/// [`EVENT_LINES`]), the clean watched presents in a row since its last bad one.
struct Event {
    start: u64,
    named: Vec<u64>,
    clean: u32,
}

pub(crate) struct Watch {
    on: bool,
    gpu: Option<Gpu>,
    /// Targets are read in this present.
    pub armed: bool,
    /// [`crate::device::Device`]'s draw count when the current colour target 0 was set.
    pub bound_draws: u64,
    /// The read-back buffer of this present's reads.
    now: Option<usize>,
    /// Each small target's last value.
    last: HashMap<u64, f32>,
    event: Option<Event>,
    /// The present of the last printed event start.
    last_start: Option<u64>,
    /// Since the device was made, and since the last report.
    pub totals: Totals,
    since: Totals,
    /// The last [`RECENT`] judged presents' results ([`Watch::recent`]).
    recent: VecDeque<(u64, Vec<(Target, Seen)>)>,
}

impl Default for Watch {
    /// Off (held in place while a read borrows the device).
    fn default() -> Watch {
        Watch::with(false)
    }
}

impl Watch {
    /// On unless `NV_D3D9_HDR_WATCH` is `off` or `0`.
    pub(crate) fn new() -> Watch {
        Watch::with(on_for(std::env::var("NV_D3D9_HDR_WATCH").ok().as_deref()))
    }

    pub(crate) fn with(on: bool) -> Watch {
        Watch {
            on,
            gpu: None,
            // The first frame is watched.
            armed: on,
            bound_draws: 0,
            now: None,
            last: HashMap::new(),
            event: None,
            last_start: None,
            totals: Totals::default(),
            since: Totals::default(),
            recent: VecDeque::new(),
        }
    }

    /// Whether the watch (and the device's check of sampled render targets) is on.
    pub(crate) fn on(&self) -> bool {
        self.on
    }

    /// The next frame is present `next` (counted from 1): watched on one in [`EVERY`].
    pub(crate) fn next_present(&mut self, next: u64) {
        self.armed = self.on && next % EVERY == 1;
        self.now = None;
    }

    /// Whether `target` (its texture's `usage`) is to be read now: a float format the watch reads,
    /// the present's reads not used up, not read yet in this present.
    pub(crate) fn wants(&self, target: &Target, usage: wgpu::TextureUsages) -> bool {
        if !self.armed
            || !matches!(
                target.format,
                wgpu::TextureFormat::Rgba16Float | wgpu::TextureFormat::Rgba32Float
            )
            || !usage.contains(wgpu::TextureUsages::TEXTURE_BINDING)
        {
            return false;
        }
        match (self.now, &self.gpu) {
            (Some(k), Some(g)) => {
                let reads = &g.backs[k].reads;
                reads.len() < READS && !reads.iter().any(|(t, _)| t.id == target.id)
            }
            _ => true,
        }
    }

    /// Records the read of `view` (a single-level 2D view of `target`, which [`Watch::wants`])
    /// into `encoder`, for the present `present`. With no free read-back buffer, the present is
    /// not watched.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn read(
        &mut self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        target: Target,
        present: u64,
        what: &'static str,
        group_key: Option<(u64, u64)>,
    ) {
        let g = self.gpu.get_or_insert_with(|| make_gpu(device));
        let k = match self.now {
            Some(k) => k,
            None => {
                let Some(k) = g
                    .backs
                    .iter()
                    .position(|b| b.state.load(Ordering::Acquire) == 0)
                else {
                    self.armed = false;
                    self.totals.skipped += 1;
                    self.since.skipped += 1;
                    return;
                };
                let b = &mut g.backs[k];
                b.state.store(1, Ordering::Release);
                b.present = present;
                b.reads.clear();
                self.now = Some(k);
                k
            }
        };
        let n = g.backs[k].reads.len() as u64;
        if g.groups.len() >= GROUPS {
            g.groups.clear();
        }
        let make = || {
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("HDR watch"),
                layout: &g.layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                            buffer: &g.storage,
                            offset: 0,
                            size: wgpu::BufferSize::new(16),
                        }),
                    },
                ],
            })
        };
        // A kept target's bind group is kept; a scratch target's is made for this read only (keeping
        // it would keep the scratch texture).
        let fresh;
        let group = match group_key {
            Some((tex, surface)) => {
                let e = g.groups.entry(tex).or_insert_with(|| (surface, make()));
                if e.0 != surface {
                    *e = (surface, make());
                }
                &e.1
            }
            None => {
                fresh = make();
                &fresh
            }
        };
        encoder.clear_buffer(&g.storage, n * SLOT, Some(16));
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("HDR watch"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&g.pipeline);
            pass.set_bind_group(0, group, &[(n * SLOT) as u32]);
            pass.dispatch_workgroups(target.width.div_ceil(16), target.height.div_ceil(16), 1);
        }
        encoder.copy_buffer_to_buffer(&g.storage, n * SLOT, &g.backs[k].buffer, n * 16, 16);
        g.backs[k].reads.push((target, what));
    }

    /// Forgets the bind group of a texture that goes (its id).
    pub(crate) fn forget(&mut self, id: u64) {
        if let Some(g) = &mut self.gpu {
            g.groups.remove(&id);
        }
    }

    /// The map of this present's read-back buffer, to run after the submit that carries its
    /// reads (on the queue thread, after the submits handed to it before).
    pub(crate) fn map_to_ask(&mut self) -> Option<Box<dyn FnOnce() + Send>> {
        let k = self.now.take()?;
        let b = &self.gpu.as_ref()?.backs[k];
        b.state.store(2, Ordering::Release);
        let (buffer, state) = (b.buffer.clone(), b.state.clone());
        Some(Box::new(move || {
            buffer.slice(..).map_async(wgpu::MapMode::Read, move |r| {
                state.store(if r.is_ok() { 3 } else { 4 }, Ordering::Release);
            });
        }))
    }

    /// The watched presents whose reads are back, judged in present order (their log lines
    /// printed); each judged present's results (tests).
    pub(crate) fn collect(&mut self) -> Vec<(u64, Vec<(Target, Seen)>)> {
        let Some(g) = &mut self.gpu else {
            return Vec::new();
        };
        let mut done = Vec::new();
        for b in &mut g.backs {
            match b.state.load(Ordering::Acquire) {
                3 => {
                    let mut reads = Vec::with_capacity(b.reads.len());
                    if let Ok(data) = b.buffer.slice(..).get_mapped_range() {
                        for (n, &(target, what)) in b.reads.iter().enumerate() {
                            let w = |j: usize| {
                                let at = n * 16 + j * 4;
                                u32::from_le_bytes([
                                    data[at],
                                    data[at + 1],
                                    data[at + 2],
                                    data[at + 3],
                                ])
                            };
                            let seen = Seen {
                                not_finite: w(0),
                                dots: w(1),
                                brightest: f32::from_bits(w(2)),
                            };
                            reads.push((target, what, seen));
                        }
                    }
                    b.buffer.unmap();
                    b.state.store(0, Ordering::Release);
                    done.push((b.present, reads));
                }
                // A failed map (a lost device): its reads are dropped, the buffer used again.
                4 => b.state.store(0, Ordering::Release),
                _ => {}
            }
        }
        done.sort_by_key(|(p, _)| *p);
        let mut judged = Vec::new();
        for (present, reads) in done {
            for line in self.judge(present, &reads) {
                eprintln!("NV_D3D9 wgpu: HDR watch: {line}");
            }
            let results: Vec<(Target, Seen)> = reads.iter().map(|(t, _, s)| (*t, *s)).collect();
            if self.recent.len() == RECENT {
                self.recent.pop_front();
            }
            self.recent.push_back((present, results.clone()));
            judged.push((present, results));
        }
        judged
    }

    /// One watched present's reads judged: the totals, and the log lines it gives (bounded over
    /// time: module documentation).
    fn judge(&mut self, present: u64, reads: &[(Target, &'static str, Seen)]) -> Vec<String> {
        let mut bad = Vec::new();
        let (mut not_finite, mut dots, mut jumps) = (false, false, false);
        let mut brightest = 0f32;
        for (target, what, seen) in reads {
            brightest = brightest.max(seen.brightest);
            let before = if target.small() {
                self.last.insert(target.id, seen.brightest)
            } else {
                None
            };
            not_finite |= seen.not_finite > 0;
            dots |= seen.dots > 0;
            if let Some(why) = problem(seen, target.small(), before) {
                jumps |= why.contains("went from");
                bad.push((
                    target.id,
                    format!(
                        "{}x{} {:?} ({what}): {why}; the brightest {}",
                        target.width, target.height, target.format, seen.brightest
                    ),
                ));
            }
        }
        for t in [&mut self.totals, &mut self.since] {
            t.presents += 1;
            t.reads += reads.len() as u64;
            t.with_not_finite += u64::from(not_finite);
            t.with_dots += u64::from(dots);
            t.with_jumps += u64::from(jumps);
            t.brightest = t.brightest.max(brightest);
        }
        let mut lines = Vec::new();
        match (&mut self.event, bad.is_empty()) {
            (None, false) => {
                for t in [&mut self.totals, &mut self.since] {
                    t.events += 1;
                }
                if self.last_start.is_some_and(|s| present < s + START_GAP) {
                    for t in [&mut self.totals, &mut self.since] {
                        t.events_unprinted += 1;
                    }
                    // Not printed, but followed (nothing of it named), so its end is not printed
                    // either.
                    self.event = Some(Event {
                        start: present,
                        named: Vec::new(),
                        clean: 0,
                    });
                    return lines;
                }
                lines.push(format!(
                    "present {present}: a float target went wrong ({} targets read)",
                    reads.len()
                ));
                let mut named = Vec::new();
                for (id, l) in bad.iter().take(EVENT_LINES) {
                    lines.push(format!("  {l}"));
                    named.push(*id);
                }
                self.last_start = Some(present);
                self.event = Some(Event {
                    start: present,
                    named,
                    clean: 0,
                });
            }
            (Some(e), false) => {
                e.clean = 0;
                // Targets that join a printed event are named once, while there is room.
                if self.last_start == Some(e.start) {
                    for (id, l) in &bad {
                        if e.named.len() < EVENT_LINES && !e.named.contains(id) {
                            lines.push(format!("  present {present}: {l}"));
                            e.named.push(*id);
                        }
                    }
                }
            }
            (Some(e), true) => {
                e.clean += 1;
                if e.clean >= CLEAN_TO_END {
                    if self.last_start == Some(e.start) {
                        lines.push(format!(
                            "present {present}: every float target clean for {CLEAN_TO_END} watched presents (the event began at present {})",
                            e.start
                        ));
                    }
                    self.event = None;
                }
            }
            (None, true) => {}
        }
        lines
    }

    /// The line for the periodic report (none when nothing was read since the last one).
    pub(crate) fn report(&mut self) -> Option<String> {
        let s = std::mem::take(&mut self.since);
        (s.presents > 0 || s.skipped > 0).then(|| {
            format!(
                "NV_D3D9 wgpu: HDR watch since the last report: {} presents read ({} float targets): {} with pixels not finite, {} with lone bright dots, {} with a small target's jump ({} events, {} of them not printed); {} presents not watched (no free read-back buffer); the brightest pixel {}",
                s.presents, s.reads, s.with_not_finite, s.with_dots, s.with_jumps, s.events, s.events_unprinted, s.skipped, s.brightest
            )
        })
    }

    /// The last judged presents' results, oldest first (tests).
    pub(crate) fn recent(&self) -> Vec<(u64, Vec<(Target, Seen)>)> {
        self.recent.iter().cloned().collect()
    }

    /// Whether reads are still on their way.
    pub(crate) fn waiting(&self) -> bool {
        self.gpu
            .as_ref()
            .is_some_and(|g| g.backs.iter().any(|b| b.state.load(Ordering::Acquire) != 0))
    }
}

fn make_gpu(device: &wgpu::Device) -> Gpu {
    let source = SHADER
        .replace("FLOOR", &format!("{DOT_FLOOR:?}"))
        .replace("RATIO", &format!("{DOT_RATIO:?}"));
    let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("HDR watch"),
        source: wgpu::ShaderSource::Wgsl(source.into()),
    });
    let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("HDR watch"),
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
                    has_dynamic_offset: true,
                    min_binding_size: wgpu::BufferSize::new(16),
                },
                count: None,
            },
        ],
    });
    let pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: None,
        bind_group_layouts: &[Some(&layout)],
        immediate_size: 0,
    });
    let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("HDR watch"),
        layout: Some(&pl),
        module: &module,
        entry_point: Some("main"),
        compilation_options: Default::default(),
        cache: None,
    });
    let storage = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("HDR watch"),
        size: READS as u64 * SLOT,
        usage: wgpu::BufferUsages::STORAGE
            | wgpu::BufferUsages::COPY_SRC
            | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let backs = (0..RING)
        .map(|_| Back {
            buffer: device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("HDR watch read-back"),
                size: READS as u64 * 16,
                usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                mapped_at_creation: false,
            }),
            state: Arc::new(AtomicU8::new(0)),
            present: 0,
            reads: Vec::with_capacity(READS),
        })
        .collect();
    Gpu {
        pipeline,
        layout,
        storage,
        backs,
        groups: HashMap::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn seen(not_finite: u32, dots: u32, brightest: f32) -> Seen {
        Seen {
            not_finite,
            dots,
            brightest,
        }
    }

    fn scene(id: u64) -> Target {
        Target {
            id,
            width: 64,
            height: 64,
            format: wgpu::TextureFormat::Rgba16Float,
        }
    }

    #[test]
    fn a_small_targets_jump_and_non_finite_pixels_are_problems_and_a_steady_value_is_not() {
        assert_eq!(problem(&seen(0, 0, 1.0), true, Some(1.5)), None);
        assert_eq!(problem(&seen(0, 0, 1.0), true, None), None);
        assert!(problem(&seen(0, 0, 0.1), true, Some(1.0)).is_some());
        assert!(problem(&seen(0, 0, 9.0), true, Some(1.0)).is_some());
        assert!(problem(&seen(0, 0, 0.0), true, Some(1.0)).is_some());
        // A large target's value is not followed.
        assert_eq!(problem(&seen(0, 0, 100.0), false, Some(1.0)), None);
        assert!(problem(&seen(3, 0, 1.0), false, None).is_some());
        assert!(problem(&seen(0, 2, 1.0), false, None).is_some());
    }

    #[test]
    fn the_switch_turns_the_watch_off_and_an_off_watch_never_arms() {
        assert!(on_for(None));
        assert!(on_for(Some("on")));
        assert!(!on_for(Some("off")));
        assert!(!on_for(Some("0")));
        let mut w = Watch::with(false);
        assert!(!w.armed && !w.on());
        for p in 1..=20 {
            w.next_present(p);
            assert!(!w.armed);
        }
        let mut w = Watch::with(true);
        assert!(w.armed);
        w.next_present(2);
        assert!(!w.armed);
        w.next_present(5);
        assert!(w.armed);
    }

    /// Bad, bad, clean, bad: one event, its start and its target lines only; it ends after
    /// enough clean presents, with one line.
    #[test]
    fn an_event_prints_its_start_once_and_its_end_after_clean_presents_in_a_row() {
        let mut w = Watch::with(true);
        let bad = [(scene(1), "test", seen(5, 0, 1.0))];
        let clean = [(scene(1), "test", seen(0, 0, 1.0))];
        let mut lines = Vec::new();
        let mut p = 1;
        for reads in [&bad, &bad, &clean, &bad] {
            lines.extend(w.judge(p, reads));
            p += EVERY;
        }
        assert_eq!(lines.len(), 2, "{lines:?}");
        assert!(lines[0].contains("went wrong"));
        for _ in 0..CLEAN_TO_END {
            lines.extend(w.judge(p, &clean));
            p += EVERY;
        }
        assert_eq!(lines.len(), 3, "{lines:?}");
        assert!(lines[2].contains("clean for"));
        assert_eq!(w.totals.events, 1);
        assert_eq!(w.totals.with_not_finite, 3);
    }

    /// A fault that comes and goes on every other watched present for a long stretch, then
    /// events that end and begin again quickly: the lines stay bounded (one start a
    /// [`START_GAP`] presents at most), and every event is counted.
    #[test]
    fn a_fault_that_comes_and_goes_prints_a_bounded_number_of_lines() {
        let mut w = Watch::with(true);
        let bad = [(scene(1), "test", seen(5, 1, 1.0))];
        let clean = [(scene(1), "test", seen(0, 0, 1.0))];
        let mut lines = 0;
        let mut p = 1;
        // 4000 presents of a flicker: one event that never ends.
        for i in 0..1000 {
            lines += w.judge(p, if i % 2 == 0 { &bad } else { &clean }).len();
            p += EVERY;
        }
        assert_eq!(lines, 2);
        // Events ending and beginning again (bad once, then clean long enough to end):
        // 6000 presents, so at most 6000 / START_GAP + 1 starts and as many ends.
        let mut cycles = 0;
        for _ in 0..(6000 / (EVERY * u64::from(CLEAN_TO_END + 1))) {
            lines += w.judge(p, &bad).len();
            p += EVERY;
            for _ in 0..CLEAN_TO_END {
                lines += w.judge(p, &clean).len();
                p += EVERY;
            }
            cycles += 1;
        }
        // The first cycle's bad present continues the flicker's event; each later cycle is one.
        let starts = 6000 / START_GAP as usize + 1;
        assert!(
            lines <= 2 + 1 + starts * 3,
            "{lines} lines for {cycles} cycles"
        );
        assert_eq!(w.totals.events, cycles);
        assert!(w.totals.events_unprinted >= cycles - 1 - starts as u64);
    }
}
