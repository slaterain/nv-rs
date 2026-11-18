//! The HDR watch: a diagnostic that reads the game's float render targets (its HDR scene, the
//! luminance down-sample chain, the eye adaptation's targets, the bloom's) on one present in
//! [`EVERY`] and says in the log when one of them goes wrong: pixels that are not finite (an
//! infinity or NaN, which the bloom spreads and the luminance chain can turn into a wrong
//! exposure), lone bright dots in the scene (a pixel over [`DOT_RATIO`] times as bright as every
//! neighbour and over [`DOT_FLOOR`]: the white dots bloom makes into orbs,
//! measure/render-aa/orbs2/verdict), or a small target (the luminance and adaptation targets)
//! whose value jumps by more than [`JUMP`] times or to 0 between two watched presents. One line
//! when such an event starts (each target that shows it, at most [`EVENT_LINES`]), one when a
//! watched present is clean again; the periodic report adds a summary. It only reads: the
//! picture is the same with it on or off (`NV_D3D9_HDR_WATCH=off`).
//!
//! A target is read when the game leaves it as colour target 0 after drawing into it, or when it
//! is a `StretchRect` destination (a multisampled scene resolved there). The results come back a
//! few frames later and are never waited for.

use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};
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
/// Judged presents kept for [`Watch::recent`].
const RECENT: usize = 16;
/// Target lines for one event at most.
const EVENT_LINES: usize = 6;

const SHADER: &str = "
@group(0) @binding(0) var t: texture_2d<f32>;
@group(0) @binding(1) var<storage, read_write> r: array<atomic<u32>, 4>;
var<workgroup> top: atomic<u32>;
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
    if (i == 0u) { atomicStore(&top, 0u); }
    workgroupBarrier();
    let size = vec2<i32>(textureDimensions(t));
    let at = vec2<i32>(g.xy);
    if (at.x < size.x && at.y < size.y) {
        let c = textureLoad(t, at, 0);
        if (!ok(c)) {
            atomicAdd(&r[0], 1u);
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
                if (v > RATIO * around) { atomicAdd(&r[1], 1u); }
            }
        }
    }
    workgroupBarrier();
    if (i == 0u) {
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
    /// The brightest finite pixel of all the targets read.
    pub brightest: f32,
}

struct Pending {
    present: u64,
    target: Target,
    what: &'static str,
    buffer: wgpu::Buffer,
    /// 0 not mapped yet, 1 mapped, 2 failed.
    state: Arc<AtomicU8>,
    asked: bool,
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

pub(crate) struct Watch {
    on: bool,
    pipeline: Option<(wgpu::ComputePipeline, wgpu::BindGroupLayout)>,
    /// Targets are read in this present.
    pub armed: bool,
    /// [`crate::device::Device`]'s draw count when the current colour target 0 was set.
    pub bound_draws: u64,
    /// The textures read in this present.
    read_now: HashSet<u64>,
    pending: Vec<Pending>,
    /// Results back for presents not all back yet.
    back: BTreeMap<u64, Vec<(Target, &'static str, Seen)>>,
    /// Each small target's last value.
    last: HashMap<u64, f32>,
    /// The present an event started in, and the lines printed for it.
    event: Option<(u64, usize)>,
    /// Since the device was made, and since the last report.
    pub totals: Totals,
    /// The last [`RECENT`] judged presents' results ([`Watch::recent`]).
    recent: VecDeque<(u64, Vec<(Target, Seen)>)>,
    since: Totals,
}

impl Default for Watch {
    /// Off (held in place while a read borrows the device).
    fn default() -> Watch {
        Watch {
            on: false,
            pipeline: None,
            armed: false,
            bound_draws: 0,
            read_now: HashSet::new(),
            pending: Vec::new(),
            back: BTreeMap::new(),
            last: HashMap::new(),
            event: None,
            totals: Totals::default(),
            recent: VecDeque::new(),
            since: Totals::default(),
        }
    }
}

impl Watch {
    pub(crate) fn new() -> Watch {
        let on = !matches!(
            std::env::var("NV_D3D9_HDR_WATCH").as_deref(),
            Ok("off" | "0")
        );
        Watch {
            on,
            // The first frame is watched.
            armed: on,
            ..Watch::default()
        }
    }

    /// The next frame is present `next` (counted from 1): watched on one in [`EVERY`].
    pub(crate) fn next_present(&mut self, next: u64) {
        self.armed = self.on && next % EVERY == 1;
        self.read_now.clear();
    }

    /// Whether `target` (its texture's `usage`) is to be read now: a float format the watch reads,
    /// not read yet in this present, the present's reads not used up.
    pub(crate) fn wants(&self, target: &Target, usage: wgpu::TextureUsages) -> bool {
        self.armed
            && matches!(
                target.format,
                wgpu::TextureFormat::Rgba16Float | wgpu::TextureFormat::Rgba32Float
            )
            && usage.contains(wgpu::TextureUsages::TEXTURE_BINDING)
            && !self.read_now.contains(&target.id)
            && self.read_now.len() < READS
    }

    /// Records the read of `view` (a single-level 2D view of `target`, which [`Watch::wants`])
    /// into `encoder`, for the present `present`.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn read(
        &mut self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        target: Target,
        present: u64,
        what: &'static str,
    ) {
        self.read_now.insert(target.id);
        let (pipeline, layout) = self.pipeline.get_or_insert_with(|| make_pipeline(device));
        let words = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("HDR watch"),
            size: 16,
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_SRC
                | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let back = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("HDR watch read-back"),
            size: 16,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: words.as_entire_binding(),
                },
            ],
        });
        encoder.clear_buffer(&words, 0, None);
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("HDR watch"),
                timestamp_writes: None,
            });
            pass.set_pipeline(pipeline);
            pass.set_bind_group(0, &group, &[]);
            pass.dispatch_workgroups(target.width.div_ceil(16), target.height.div_ceil(16), 1);
        }
        encoder.copy_buffer_to_buffer(&words, 0, &back, 0, 16);
        self.pending.push(Pending {
            present,
            target,
            what,
            buffer: back,
            state: Arc::new(AtomicU8::new(0)),
            asked: false,
        });
    }

    /// The maps of the reads recorded since the last call: each job is to run after the submit
    /// that carries its read (on the queue thread, after the submits handed to it before).
    pub(crate) fn maps_to_ask(&mut self) -> Vec<Box<dyn FnOnce() + Send>> {
        let mut jobs: Vec<Box<dyn FnOnce() + Send>> = Vec::new();
        for p in self.pending.iter_mut().filter(|p| !p.asked) {
            p.asked = true;
            let (buffer, state) = (p.buffer.clone(), p.state.clone());
            jobs.push(Box::new(move || {
                buffer.slice(..).map_async(wgpu::MapMode::Read, move |r| {
                    state.store(if r.is_ok() { 1 } else { 2 }, Ordering::Release);
                });
            }));
        }
        jobs
    }

    /// The results that have come back. A present whose reads are all back is judged: the event
    /// lines ([`Watch`]'s module documentation) and the totals. Each judged present's results,
    /// in present order (tests).
    pub(crate) fn collect(&mut self) -> Vec<(u64, Vec<(Target, Seen)>)> {
        let mut keep = Vec::new();
        for p in std::mem::take(&mut self.pending) {
            match p.state.load(Ordering::Acquire) {
                0 => keep.push(p),
                1 => {
                    let mut w = [0u32; 4];
                    if let Ok(data) = p.buffer.slice(..).get_mapped_range() {
                        for (k, c) in data.chunks_exact(4).enumerate().take(4) {
                            w[k] = u32::from_le_bytes([c[0], c[1], c[2], c[3]]);
                        }
                    }
                    p.buffer.unmap();
                    let seen = Seen {
                        not_finite: w[0],
                        dots: w[1],
                        brightest: f32::from_bits(w[2]),
                    };
                    self.back
                        .entry(p.present)
                        .or_default()
                        .push((p.target, p.what, seen));
                }
                // A failed map (a lost device): that read is dropped.
                _ => {}
            }
        }
        self.pending = keep;
        let oldest_waiting = self.pending.iter().map(|p| p.present).min();
        let mut judged = Vec::new();
        while let Some((&present, _)) = self.back.first_key_value() {
            if oldest_waiting.is_some_and(|w| w <= present) {
                break;
            }
            let reads = self.back.remove(&present).unwrap_or_default();
            let results = self.judge(present, &reads);
            if self.recent.len() == RECENT {
                self.recent.pop_front();
            }
            self.recent.push_back((present, results.clone()));
            judged.push((present, results));
        }
        judged
    }

    fn judge(
        &mut self,
        present: u64,
        reads: &[(Target, &'static str, Seen)],
    ) -> Vec<(Target, Seen)> {
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
                bad.push(format!(
                    "{}x{} {:?} ({what}): {why}; the brightest {}",
                    target.width, target.height, target.format, seen.brightest
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
        match (&mut self.event, bad.is_empty()) {
            (None, false) => {
                eprintln!(
                    "NV_D3D9 wgpu: HDR watch: present {present}: a float target went wrong ({} targets read)",
                    reads.len()
                );
                for line in bad.iter().take(EVENT_LINES) {
                    eprintln!("NV_D3D9 wgpu: HDR watch:   {line}");
                }
                self.event = Some((present, bad.len().min(EVENT_LINES)));
            }
            (Some((_, printed)), false) => {
                // Targets that join a running event are named once there is room.
                for line in bad.iter().take(EVENT_LINES.saturating_sub(*printed)) {
                    eprintln!("NV_D3D9 wgpu: HDR watch:   present {present}: {line}");
                    *printed += 1;
                }
            }
            (Some((start, _)), true) => {
                eprintln!(
                    "NV_D3D9 wgpu: HDR watch: present {present}: every float target clean again (the event began at present {start})"
                );
                self.event = None;
            }
            (None, true) => {}
        }
        reads.iter().map(|(t, _, s)| (*t, *s)).collect()
    }

    /// The line for the periodic report (none when nothing was read since the last one).
    pub(crate) fn report(&mut self) -> Option<String> {
        let s = std::mem::take(&mut self.since);
        (s.presents > 0).then(|| {
            format!(
                "NV_D3D9 wgpu: HDR watch since the last report: {} presents read ({} float targets): {} with pixels not finite, {} with lone bright dots, {} with a small target's jump; the brightest pixel {}",
                s.presents, s.reads, s.with_not_finite, s.with_dots, s.with_jumps, s.brightest
            )
        })
    }

    /// The last judged presents' results, oldest first (tests).
    pub(crate) fn recent(&self) -> Vec<(u64, Vec<(Target, Seen)>)> {
        self.recent.iter().cloned().collect()
    }

    /// Whether reads are still on their way.
    pub(crate) fn waiting(&self) -> bool {
        !self.pending.is_empty() || !self.back.is_empty()
    }
}

fn make_pipeline(device: &wgpu::Device) -> (wgpu::ComputePipeline, wgpu::BindGroupLayout) {
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
                    has_dynamic_offset: false,
                    min_binding_size: None,
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
    (pipeline, layout)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_small_targets_jump_and_non_finite_pixels_are_problems_and_a_steady_value_is_not() {
        let seen = |n, d, b| Seen {
            not_finite: n,
            dots: d,
            brightest: b,
        };
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
}
