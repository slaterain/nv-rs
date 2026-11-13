//! The frame-rate counter: a small readout in the top-right corner of the game's window, the
//! port's own (not the game's), off until the player presses F11 or `Fallout.ini`'s
//! `[nv-rs Display] bShowFrameRate=1` turns it on from the start ([`show`]).
//!
//! It shows two lines: the frames a second averaged over about half a second, and the longest
//! time between two frames in about the last second ("max 9.3 ms": a stutter shows there even
//! when the average is high). It counts the frames the window was given: the time is taken on
//! the queue thread right after each frame's `present` returns, where the frame reaches the swap
//! chain ([`Meter`]), not when the game's loop turns or the render thread hands the frame on.
//!
//! It is drawn by the wgpu backend into the window's frame after the game's frame is in it and
//! after the window's capture is taken, just before the present ([`Overlay`]): it is never in the
//! back buffer, so never in an F12 report's picture, an `NV_SHOT` capture of the game's frame or
//! of the window's (`wgpu-window-N.bmp`), nor in anything the comparison tools read. Only a
//! backend on the game's window draws it (offscreen ones never do), and the real Direct3D 9
//! fallback has none.
//!
//! Off, a present costs one atomic read and a branch: nothing is measured, made or written. On,
//! each present takes one timestamp under an uncontended lock and draws one instanced quad list
//! from buffers made once; the text and its quads are made again about four times a second.

use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::time::{Duration, Instant};

/// Whether the counter shows ([`show`], [`window_key`]).
static SHOWN: AtomicBool = AtomicBool::new(false);

/// Shows or hides the counter: the run's default (`Fallout.ini`'s `bShowFrameRate`), set by the
/// host before the game starts.
pub fn show(on: bool) {
    SHOWN.store(on, Ordering::Relaxed);
}

/// Whether the counter shows now.
pub fn shown() -> bool {
    SHOWN.load(Ordering::Relaxed)
}

/// Whether a wgpu device draws the game's window, so the counter can show: set by the device that
/// owns the window ([`window_device`]). Without one (the real Direct3D 9 fallback, offscreen
/// runs) F11 is the game's like any key ([`window_key`]).
static WINDOW_DEVICES: AtomicUsize = AtomicUsize::new(0);

/// A wgpu device on the game's window was made (`true`) or dropped (`false`): counted, as a
/// device made again (a new device before the old one is dropped) keeps the counter.
pub(crate) fn window_device(made: bool) {
    if made {
        WINDOW_DEVICES.fetch_add(1, Ordering::Relaxed);
    } else {
        let _ =
            WINDOW_DEVICES.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |n| n.checked_sub(1));
    }
}

/// As if a wgpu device drew the game's window (`true`) or none did (tests).
#[doc(hidden)]
pub fn set_window_device(on: bool) {
    WINDOW_DEVICES.store(usize::from(on), Ordering::Relaxed);
}

/// The window was activated, the display changed or the machine woke (`crate::window_changed`):
/// the time until the next frame is no frame's, as after a minimised window gave none
/// ([`Meter::no_frame`]).
static GAP: AtomicBool = AtomicBool::new(false);

pub(crate) fn gap() {
    GAP.store(true, Ordering::Relaxed);
}

/// Whether a gap ([`gap`]) came since the last call.
pub(crate) fn take_gap() -> bool {
    GAP.swap(false, Ordering::Relaxed)
}

/// `VK_F11`: the counter's key. The game has no default use for it (its quick save and load are
/// F5 and F9, the Pip-Boy's pages F1 to F3), and Windows does not take it as it takes F10.
pub const KEY: usize = 0x7a;

/// `WM_KEYDOWN`, `WM_KEYUP`, `WM_SYSKEYDOWN`, `WM_SYSKEYUP`.
const KEY_MESSAGES: [u32; 4] = [0x100, 0x101, 0x104, 0x105];

/// A key message of the game's window (`message`, `wparam`, `lparam` as the window procedure
/// gets them): true when it is the counter's key, which then goes no further (the game never
/// sees it go down or up). Its first press toggles the counter; the repeats Windows sends while
/// it is held (bit 30 of `lparam`, the key was already down) and its release change nothing.
/// While no wgpu device draws the game's window ([`window_device`]: the real Direct3D 9
/// fallback has no counter) F11 is not taken: false, and it reaches the game.
pub fn window_key(message: u32, wparam: usize, lparam: isize) -> bool {
    if wparam != KEY
        || !KEY_MESSAGES.contains(&message)
        || WINDOW_DEVICES.load(Ordering::Relaxed) == 0
    {
        return false;
    }
    let down = message == 0x100 || message == 0x104;
    if down && lparam & (1 << 30) == 0 {
        SHOWN.fetch_xor(true, Ordering::Relaxed);
    }
    true
}

/// How often the numbers are made again; the average is over the last two such periods, the
/// slowest frame over the last four.
const PERIOD: Duration = Duration::from_millis(250);
const AVERAGED: usize = 2;
const KEPT: usize = 4;

/// One period's frames: how many, their time in all, the longest.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct Period {
    frames: u32,
    time: Duration,
    longest: Duration,
}

/// What the counter shows: frames a second (whole) and the slowest frame in tenths of a
/// millisecond.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Readout {
    pub fps: u32,
    pub slowest_tenths_ms: u32,
}

/// The presented frames' times: no allocation, a fixed amount of work per frame.
#[derive(Debug, Default)]
pub struct Meter {
    last: Option<Instant>,
    open: Period,
    /// The closed periods, newest at `at - 1` (a ring), `filled` of them.
    done: [Period; KEPT],
    at: usize,
    filled: usize,
    readout: Option<Readout>,
    /// Counts the readouts made: the drawing makes its text again when this changes.
    generation: u64,
    /// Counts the presents timed, kept through a reset (tests: none while the counter is off).
    presents: u64,
}

impl Meter {
    /// A frame reached the swap chain at `now`.
    pub fn presented(&mut self, now: Instant) {
        self.presents += 1;
        if let Some(last) = self.last {
            self.frame(now.saturating_duration_since(last));
        }
        self.last = Some(now);
    }

    /// A frame `time` after the one before.
    pub fn frame(&mut self, time: Duration) {
        let p = &mut self.open;
        p.frames += 1;
        p.time += time;
        p.longest = p.longest.max(time);
        if p.time < PERIOD {
            return;
        }
        self.done[self.at] = std::mem::take(&mut self.open);
        self.at = (self.at + 1) % KEPT;
        self.filled = (self.filled + 1).min(KEPT);
        let newest =
            |n: usize| (1..=n.min(self.filled)).map(|i| self.done[(self.at + KEPT - i) % KEPT]);
        let (frames, nanos) = newest(AVERAGED).fold((0u128, 0u128), |(f, t), p| {
            (f + u128::from(p.frames), t + p.time.as_nanos())
        });
        let longest = newest(KEPT).map(|p| p.longest).max().unwrap_or_default();
        // Rounded to the nearest, halves up.
        let fps = (frames * 2_000_000_000 + nanos) / (2 * nanos.max(1));
        let tenths = (longest.as_nanos() + 50_000) / 100_000;
        self.readout = Some(Readout {
            fps: fps.min(99_999) as u32,
            slowest_tenths_ms: tenths.min(99_999) as u32,
        });
        self.generation += 1;
    }

    /// Everything forgotten: the counter was just turned on, and the time since the last frame
    /// it saw is not a frame's.
    pub fn reset(&mut self) {
        let generation = self.generation + 1;
        *self = Meter {
            generation,
            presents: self.presents,
            ..Meter::default()
        };
    }

    /// The window took no frame (minimised, sized to nothing, lost) or was away ([`gap`]): the
    /// next present only starts the clock, so the time away is not shown as a slow frame. A frame
    /// the game itself was slow to give (a load, a stall) still counts in full: the player saw it.
    pub fn no_frame(&mut self) {
        self.last = None;
    }

    /// The presents timed so far (tests).
    pub fn presents(&self) -> u64 {
        self.presents
    }

    /// The numbers to show (none until the first period ends) and their generation.
    pub fn readout(&self) -> (Option<Readout>, u64) {
        (self.readout, self.generation)
    }
}

/// One line of the readout, made without allocating.
#[derive(Clone, Copy, Default)]
pub struct Line {
    bytes: [u8; 16],
    len: usize,
}

impl Line {
    pub fn as_str(&self) -> &str {
        std::str::from_utf8(&self.bytes[..self.len]).unwrap_or("")
    }
}

impl std::fmt::Write for Line {
    fn write_str(&mut self, s: &str) -> std::fmt::Result {
        let end = self.len + s.len();
        if end > self.bytes.len() {
            return Err(std::fmt::Error);
        }
        self.bytes[self.len..end].copy_from_slice(s.as_bytes());
        self.len = end;
        Ok(())
    }
}

/// The readout's two lines: "144 fps" and "max 9.3 ms".
pub fn lines(r: Readout) -> [Line; 2] {
    use std::fmt::Write;
    let (mut a, mut b) = (Line::default(), Line::default());
    let _ = write!(a, "{} fps", r.fps);
    let t = r.slowest_tenths_ms;
    let _ = write!(b, "max {}.{} ms", t / 10, t % 10);
    [a, b]
}

/// The characters the readout uses, in the glyph strip's order (a space is drawn as nothing).
const GLYPHS: &str = "0123456789.fpsmax";

/// The most quads one readout takes: the plate and two lines of at most 16 characters.
const MAX_QUADS: usize = 1 + 2 * 16;
/// A quad: its rectangle in clip space (x, y, width, height), the glyph strip's part it shows
/// (u, v, width, height; u below 0: no texture, the plate), its colour.
const QUAD_FLOATS: usize = 12;
const QUAD_BYTES: usize = QUAD_FLOATS * 4;

/// The quads for `lines` in a `width` by `height` window with glyph cells `cell` (width, height
/// in pixels) of the strip: the dark plate, then each character right-aligned in the top-right
/// corner. Returns how many of `out` were written.
fn layout(
    lines: &[Line; 2],
    (width, height): (u32, u32),
    cell: (u32, u32),
    out: &mut [[f32; QUAD_FLOATS]; MAX_QUADS],
) -> usize {
    let (fw, fh) = (width.max(1) as f32, height.max(1) as f32);
    let (cw, ch) = (cell.0 as f32, cell.1 as f32);
    let count = GLYPHS.chars().count() as f32;
    let pad = (ch / 3.0).round();
    let margin = (ch / 2.0).round();
    let widest = lines.iter().map(|l| l.len).max().unwrap_or(0) as f32;
    let plate = [
        fw - margin - widest * cw - 2.0 * pad,
        margin,
        widest * cw + 2.0 * pad,
        2.0 * ch + 2.0 * pad,
    ];
    let clip = |[x, y, w, h]: [f32; 4]| {
        [
            x / fw * 2.0 - 1.0,
            1.0 - (y + h) / fh * 2.0,
            w / fw * 2.0,
            h / fh * 2.0,
        ]
    };
    let mut n = 0;
    let mut push = |rect: [f32; 4], uv: [f32; 4], colour: [f32; 4]| {
        let mut q = [0.0; QUAD_FLOATS];
        q[..4].copy_from_slice(&clip(rect));
        q[4..8].copy_from_slice(&uv);
        q[8..].copy_from_slice(&colour);
        out[n] = q;
        n += 1;
    };
    push(plate, [-1.0, 0.0, 0.0, 0.0], [0.0, 0.0, 0.0, 0.6]);
    for (row, line) in lines.iter().enumerate() {
        let right = fw - margin - pad;
        let y = margin + pad + row as f32 * ch;
        for (i, c) in line.as_str().chars().enumerate() {
            let Some(g) = GLYPHS.chars().position(|x| x == c) else {
                continue;
            };
            let x = right - (line.len - i) as f32 * cw;
            push(
                [x, y, cw, ch],
                [g as f32 / count, 0.0, 1.0 / count, 1.0],
                [1.0, 1.0, 0.9, 1.0],
            );
        }
    }
    n
}

/// The glyphs' height for a window `height` pixels high: about a 45th of it, from 14 to 48
/// pixels, whatever the game's own interface size.
fn glyph_height(height: u32) -> u32 {
    (height / 45).clamp(14, 48)
}

const SHADER: &str = "
struct Q { rect: vec4<f32>, uv: vec4<f32>, colour: vec4<f32> }
@group(0) @binding(0) var t: texture_2d<f32>;
@group(0) @binding(1) var s: sampler;
@group(0) @binding(2) var<storage, read> qs: array<Q>;
struct V {
    @builtin(position) p: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) colour: vec4<f32>,
    @location(2) plate: f32,
}
@vertex fn vs(@builtin(vertex_index) i: u32, @builtin(instance_index) n: u32) -> V {
    let q = qs[n];
    let c = vec2<f32>(f32(i & 1u), f32((i >> 1u) & 1u));
    var v: V;
    v.p = vec4<f32>(q.rect.xy + c * q.rect.zw, 0.0, 1.0);
    v.uv = q.uv.xy + vec2<f32>(c.x, 1.0 - c.y) * q.uv.zw;
    v.colour = q.colour;
    v.plate = select(0.0, 1.0, q.uv.x < 0.0);
    return v;
}
@fragment fn fs(v: V) -> @location(0) vec4<f32> {
    let a = textureSample(t, s, v.uv).a;
    return vec4<f32>(v.colour.rgb, v.colour.a * mix(a, 1.0, v.plate));
}
";

/// The counter's drawing on one device and window format: its pipeline, the quads' buffer and
/// the glyph strip, made once (the strip again when the window's height changes its size).
pub(crate) struct Overlay {
    pipeline: wgpu::RenderPipeline,
    /// The window format the pipeline is made for.
    pub(crate) format: wgpu::TextureFormat,
    layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    quads: wgpu::Buffer,
    /// The glyph strip's height and cell, and the bind group on it.
    strip: Option<(u32, (u32, u32), wgpu::BindGroup)>,
    /// What the quads were last made for: the readout's generation and the window's size.
    made_for: Option<(u64, (u32, u32))>,
    count: u32,
    /// Times drawn (tests).
    pub(crate) drawn: u64,
}

impl Overlay {
    pub(crate) fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Overlay {
        use wgpu::ShaderStages as S;
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("frame rate"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: S::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: S::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: S::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: std::num::NonZeroU64::new(QUAD_BYTES as u64),
                    },
                    count: None,
                },
            ],
        });
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("frame rate"),
            source: wgpu::ShaderSource::Wgsl(SHADER.into()),
        });
        let pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("frame rate"),
            layout: Some(&pl),
            vertex: wgpu::VertexState {
                module: &module,
                entry_point: Some("vs"),
                compilation_options: Default::default(),
                buffers: &[],
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleStrip,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: Default::default(),
            fragment: Some(wgpu::FragmentState {
                module: &module,
                entry_point: Some("fs"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview_mask: None,
            cache: None,
        });
        let quads = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("frame rate"),
            size: (MAX_QUADS * QUAD_BYTES) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        Overlay {
            pipeline,
            format,
            layout,
            // Nearest: the glyphs are drawn at their own size.
            sampler: device.create_sampler(&wgpu::SamplerDescriptor::default()),
            quads,
            strip: None,
            made_for: None,
            count: 0,
            drawn: 0,
        }
    }

    /// The glyph strip at `height` pixels: its cell and bind group, made when the height changed;
    /// none where Windows' GDI draws no text (other platforms).
    fn strip(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        height: u32,
    ) -> Option<(u32, u32)> {
        if let Some((h, cell, _)) = &self.strip {
            if *h == height {
                return Some(*cell);
            }
        }
        self.strip = None;
        // Consolas: every glyph the same width, so the strip's cells are equal.
        let (w, h, px) = crate::loading::text_image(GLYPHS, height, ("Consolas", 700))?;
        let n = GLYPHS.chars().count() as u32;
        let view = crate::loading::texture(device, queue, (w, h, &px));
        let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("frame rate"),
            layout: &self.layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: self.quads.as_entire_binding(),
                },
            ],
        });
        // The strip is drawn whole; each cell a whole number of pixels wide.
        let cell = (w / n, h);
        self.strip = Some((height, cell, group));
        self.made_for = None;
        Some(cell)
    }

    /// Draws `readout` (with its generation) into `target`, a `width` by `height` window frame
    /// that already holds the game's frame: after everything recorded into `encoder` so far.
    /// Nothing before the first readout. The quads are made again only when the readout or the
    /// window's size changed.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn draw(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        size: (u32, u32),
        (readout, generation): (Option<Readout>, u64),
    ) {
        let Some(readout) = readout else { return };
        let Some(cell) = self.strip(device, queue, glyph_height(size.1)) else {
            return;
        };
        if self.made_for != Some((generation, size)) {
            let mut quads = [[0.0f32; QUAD_FLOATS]; MAX_QUADS];
            let n = layout(&lines(readout), size, cell, &mut quads);
            let mut bytes = [0u8; MAX_QUADS * QUAD_BYTES];
            for (b, v) in bytes.chunks_exact_mut(4).zip(quads.iter().flatten()) {
                b.copy_from_slice(&v.to_le_bytes());
            }
            queue.write_buffer(&self.quads, 0, &bytes[..n * QUAD_BYTES]);
            self.count = n as u32;
            self.made_for = Some((generation, size));
        }
        let Some((_, _, group)) = &self.strip else {
            return;
        };
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("frame rate"),
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
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, group, &[]);
        pass.draw(0..4, 0..self.count);
        self.drawn += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ms(v: u64) -> Duration {
        Duration::from_millis(v)
    }

    fn text(m: &Meter) -> Option<(String, String)> {
        m.readout()
            .0
            .map(lines)
            .map(|[a, b]| (a.as_str().to_string(), b.as_str().to_string()))
    }

    /// Nothing until the first quarter second is over; then the average over the last two
    /// periods and the slowest frame over the last four, as text.
    #[test]
    fn the_average_and_the_slowest_frame_from_fixed_frame_times() {
        let mut m = Meter::default();
        for _ in 0..24 {
            m.frame(ms(10));
        }
        assert_eq!(text(&m), None, "the first period is not over");
        m.frame(ms(10));
        assert_eq!(text(&m), Some(("100 fps".into(), "max 10.0 ms".into())));
        // A 40 ms stutter among 10 ms frames: the average barely moves, the slowest shows it.
        m.frame(ms(40));
        for _ in 0..22 {
            m.frame(ms(10));
        }
        // The second period ends at 250 ms (22 frames; the last 10 ms frame starts the third):
        // 47 frames in 500 ms.
        assert_eq!(text(&m), Some(("94 fps".into(), "max 40.0 ms".into())));
        // Two more periods of 10 ms frames: the stutter has left the average, not the slowest.
        for _ in 0..50 {
            m.frame(ms(10));
        }
        assert_eq!(text(&m), Some(("100 fps".into(), "max 40.0 ms".into())));
        // A third: still within the last four periods (the last second).
        for _ in 0..25 {
            m.frame(ms(10));
        }
        assert_eq!(text(&m), Some(("100 fps".into(), "max 40.0 ms".into())));
        // A fourth: the stutter's period is the fifth back, out of the last second.
        for _ in 0..25 {
            m.frame(ms(10));
        }
        assert_eq!(text(&m), Some(("100 fps".into(), "max 10.0 ms".into())));
    }

    /// Whole frames a second rounded to the nearest, the slowest frame to a tenth of a ms.
    #[test]
    fn the_numbers_are_rounded_to_whole_frames_and_tenths_of_a_millisecond() {
        let mut m = Meter::default();
        // 6.944 ms frames (144 a second): 37 of them make 256.9 ms.
        for _ in 0..37 {
            m.frame(Duration::from_micros(6_944));
        }
        assert_eq!(text(&m), Some(("144 fps".into(), "max 6.9 ms".into())));
        let mut m = Meter::default();
        // 16.65 ms frames: 16 of them in 266.4 ms, 60.06 a second, show 60; the frame 16.7
        // (16.65 rounds up).
        for _ in 0..16 {
            m.frame(Duration::from_micros(16_650));
        }
        assert_eq!(text(&m), Some(("60 fps".into(), "max 16.7 ms".into())));
        // A long hitch: one frame of 1.5 s.
        let mut m = Meter::default();
        m.frame(ms(1500));
        assert_eq!(text(&m), Some(("1 fps".into(), "max 1500.0 ms".into())));
        let [a, b] = lines(Readout {
            fps: 99_999,
            slowest_tenths_ms: 99_999,
        });
        assert_eq!((a.as_str(), b.as_str()), ("99999 fps", "max 9999.9 ms"));
    }

    /// The meter takes its frames from the presents' times; a reset forgets the time since the
    /// last one (the counter was off), and every readout has a new generation.
    #[test]
    fn presents_make_frames_and_a_reset_forgets_the_gap() {
        let mut m = Meter::default();
        let t0 = Instant::now();
        for i in 0..=25u64 {
            m.presented(t0 + ms(10 * i));
        }
        let (r, g) = m.readout();
        assert_eq!(r.map(|r| r.fps), Some(100));
        m.reset();
        assert_eq!(m.readout(), (None, g + 1));
        // The first present after the reset only starts the clock: the 10 s gap is no frame.
        for i in 0..=25u64 {
            m.presented(t0 + Duration::from_secs(10) + ms(20 * i));
        }
        let (r, g2) = m.readout();
        assert_eq!(
            r,
            Some(Readout {
                fps: 50,
                slowest_tenths_ms: 200
            })
        );
        assert!(g2 > g + 1);
    }

    /// A window that took no frame (minimised) or was away (Alt-Tab): the time until the next
    /// present is not shown as a slow frame; a slow frame the game itself gave still is.
    #[test]
    fn time_away_is_no_frame_but_a_slow_frame_is() {
        let t0 = Instant::now();
        let run = |away: bool| {
            let mut m = Meter::default();
            let mut t = t0;
            for _ in 0..=10 {
                m.presented(t);
                t += ms(10);
            }
            // 8 s without a frame, then 10 ms frames until a period ends.
            if away {
                m.no_frame();
            }
            t += Duration::from_secs(8);
            for _ in 0..=30 {
                m.presented(t);
                t += ms(10);
            }
            m.readout().0.map(|r| r.slowest_tenths_ms)
        };
        assert_eq!(run(true), Some(100), "away: the slowest is a 10 ms frame");
        assert_eq!(
            run(false),
            Some(80_100),
            "a stall the player saw shows in full"
        );
    }

    /// F11 toggles once per press: its repeats while held and its release change nothing, and
    /// every F11 message is taken (the game never sees it); other keys pass and change nothing.
    #[test]
    fn f11_toggles_once_per_press_and_never_reaches_the_game() {
        show(false);
        let (down, up, sys_down) = (0x100, 0x101, 0x104);
        let repeat = 1 << 30;
        // No wgpu device on the game's window (the real Direct3D 9 draws): F11 is the game's.
        set_window_device(false);
        assert!(!window_key(down, KEY, 0x0057_0001));
        assert!(!window_key(up, KEY, 0x0057_0001 | repeat | (1 << 31)));
        assert!(!shown(), "no counter to show");
        set_window_device(true);
        assert!(window_key(down, KEY, 0x0057_0001));
        assert!(shown());
        for _ in 0..20 {
            assert!(window_key(down, KEY, 0x0057_0001 | repeat));
        }
        assert!(shown(), "held: no flutter");
        assert!(window_key(up, KEY, 0x0057_0001 | repeat | (1 << 31)));
        assert!(shown(), "the release changes nothing");
        assert!(window_key(sys_down, KEY, 0x0057_0001));
        assert!(!shown(), "pressed again (with Alt too): off");
        // F12 (the report), F10, Escape and a character pass, and change nothing.
        for vk in [0x7b, 0x79, 0x1b, 0x41] {
            assert!(!window_key(down, vk, 0x0001_0001));
        }
        // Not a key message.
        assert!(!window_key(0x102, KEY, 0));
        assert!(!shown());
        set_window_device(false);
    }

    /// The quads: a plate in the top-right corner, then each character right-aligned on its line,
    /// a space drawn as nothing; all inside the window.
    #[test]
    fn the_readout_sits_in_the_top_right_corner() {
        let mut out = [[0.0; QUAD_FLOATS]; MAX_QUADS];
        let l = lines(Readout {
            fps: 144,
            slowest_tenths_ms: 93,
        });
        let n = layout(&l, (1920, 1080), (12, 24), &mut out);
        // "144 fps" has 6 drawn characters, "max 9.3 ms" 8.
        assert_eq!(n, 1 + 6 + 8);
        let px = |q: &[f32; QUAD_FLOATS]| {
            (
                ((q[0] + 1.0) / 2.0 * 1920.0).round(),
                ((1.0 - q[1] - q[3]) / 2.0 * 1080.0).round(),
                (q[2] / 2.0 * 1920.0).round(),
                (q[3] / 2.0 * 1080.0).round(),
            )
        };
        // The plate: 10 characters wide plus 8 pixels each side, 12 pixels from the edges.
        assert_eq!(
            px(&out[0]),
            (1920.0 - 12.0 - 120.0 - 16.0, 12.0, 136.0, 64.0)
        );
        assert!(out[0][4] < 0.0, "the plate takes no glyph");
        // The last character of each line ends 8 pixels inside the plate's right edge.
        assert_eq!(px(&out[6]), (1920.0 - 20.0 - 12.0, 20.0, 12.0, 24.0));
        assert_eq!(px(&out[14]), (1920.0 - 20.0 - 12.0, 44.0, 12.0, 24.0));
        // The 's' of "fps" is the strip's 14th glyph.
        let count = GLYPHS.len() as f32;
        assert_eq!(out[6][4], 13.0 / count);
        assert_eq!(glyph_height(1080), 24);
        assert_eq!(glyph_height(480), 14);
        assert_eq!(glyph_height(2160), 48);
    }
}
