//! The game's brightness in nv-rs's window. **Not 1:1 with FalloutNV.exe**: asked for by the
//! maintainer (2026-11-23).
//!
//! **What the game does.** The Display menu's brightness slider (`007d19c0`, 16 steps from
//! `fGammaMin:Display`, 1.4, to `fGammaMax:Display`, 0.6) sets `fGamma` (`004dc2d0`), and the
//! main loop then builds Direct3D 9's gamma ramp from it (`004dcef0`): for each level `i` of 256,
//! `(i / 255)^fGamma * 65535 + 0.5`, truncated, the same for red, green and blue, given to
//! `SetGammaRamp`. Direct3D 9 applies a ramp only to an exclusive full-screen swap chain; in a
//! window it changes nothing. nv-rs shows the game in a window (its full screen is the borderless
//! window over the monitor), so the slider did nothing.
//!
//! **What this does.** The frame shown in the window goes through the game's last ramp, as the
//! display hardware applies it in exclusive full screen: each colour's 8-bit level picks its
//! entry, shown as the 8-bit level nearest entry / 65535 ([`level`]). It is done in the copy of
//! the back buffer into the window's frame (`Present`), not in a pass of its own. With no ramp
//! set, or a ramp that gives every level back ([`Ramp::is_identity`]: the game's at `fGamma`
//! 1.0), the copy is the plain one, so the picture is the same bit for bit. The back buffer is
//! never changed: its captures (`NV_SHOT`'s `wgpu-frame-N.bmp`, F12 reports) stay the game's
//! picture; the window's capture (`wgpu-window-N.bmp`) has the ramp.
//!
//! **What it says.** The brightness the game's window takes is said in the log ([`change_line`]).
//! The game sends a ramp whenever its brightness changes: the slider, the start-up INI value, and
//! the `SetGamma` console and script command, which a mod may send every frame; so a line is
//! printed only when its value to two places differs from the last one printed, at most once a
//! second, and a value held back is printed once it has settled ([`Said`]). The first ramp that
//! changes the picture also says once that a brightness saved earlier now shows ([`FIRST_LINE`]);
//! an F12 report's `place.txt` says the brightness the window showed ([`place_line`]).

use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{Duration, Instant};

/// Entries per colour.
pub const ENTRIES: usize = 256;

/// A `D3DGAMMARAMP`: 256 16-bit entries for red, then green, then blue.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ramp(pub [[u16; ENTRIES]; 3]);

impl Ramp {
    /// The ramp in `SetGammaRamp`'s bytes: 0x600, little-endian, red's entries first; `None`
    /// for any other length.
    pub fn from_bytes(bytes: &[u8]) -> Option<Ramp> {
        if bytes.len() != 3 * ENTRIES * 2 {
            return None;
        }
        let mut ramp = [[0u16; ENTRIES]; 3];
        for (at, pair) in bytes.chunks_exact(2).enumerate() {
            ramp[at / ENTRIES][at % ENTRIES] = u16::from_le_bytes([pair[0], pair[1]]);
        }
        Some(Ramp(ramp))
    }

    /// The ramp as `SetGammaRamp`'s bytes ([`Ramp::from_bytes`]).
    pub fn to_bytes(&self) -> Vec<u8> {
        self.0
            .iter()
            .flatten()
            .flat_map(|entry| entry.to_le_bytes())
            .collect()
    }

    /// Whether every entry is shown as its own level ([`level`]): the picture through the ramp
    /// is the picture, so none is applied. Direct3D 9's own ramp (`i * 257`, which is also the
    /// game's at `fGamma` 1.0) is one; `i * 256` is not (its last entry shows as 254).
    pub fn is_identity(&self) -> bool {
        self.0.iter().all(|colour| {
            colour
                .iter()
                .enumerate()
                .all(|(i, &entry)| usize::from(level(entry)) == i)
        })
    }

    /// The entries as [`SHADER`] reads them: for each level red, green, blue and 0 as `u32`
    /// words (a uniform array's elements are 16 bytes apart).
    pub(crate) fn uniform_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(ENTRIES * 16);
        for i in 0..ENTRIES {
            for word in [self.0[0][i], self.0[1][i], self.0[2][i], 0] {
                bytes.extend_from_slice(&u32::from(word).to_le_bytes());
            }
        }
        bytes
    }

    /// The brightness the ramp stands for, as the game's `fGamma`: worked out from red's middle
    /// entry, which the game makes `(128 / 255)^fGamma * 65535` ([`games_ramp`]). `None` where that
    /// entry is 0 or the full 65535: no curve of the game's gives either.
    pub fn gamma(&self) -> Option<f32> {
        let entry = self.0[0][128];
        if entry == 0 || entry == u16::MAX {
            return None;
        }
        Some(((f64::from(entry) / 65535.0).ln() / (128.0f64 / 255.0).ln()) as f32)
    }
}

/// The brightness the game's window shows ([`shown`]): `f32` bits, 0 for none.
static SHOWN: AtomicU32 = AtomicU32::new(0);

/// The brightness the game's window shows now, as the game's `fGamma` ([`Ramp::gamma`]; NaN for a
/// ramp not of the game's curve), or `None` while the window shows the picture as drawn (no ramp
/// set, or one that changes no level). Set by the device on the game's window (never by one
/// without a window: a replay, the shadow of the real Direct3D 9); the F12 report's place says it
/// ([`place_line`]).
pub fn shown() -> Option<f32> {
    match SHOWN.load(Ordering::Relaxed) {
        0 => None,
        bits => Some(f32::from_bits(bits)),
    }
}

/// [`shown`] from the ramp the window now applies (`None`: none).
pub(crate) fn set_shown(ramp: Option<&Ramp>) {
    let bits = ramp.map_or(0, |r| r.gamma().unwrap_or(f32::NAN).to_bits());
    SHOWN.store(bits, Ordering::Relaxed);
}

/// Said once a run, at the first ramp the game's window applies: why a brightness the player
/// saved long ago may change the picture now.
pub const FIRST_LINE: &str = "the game's brightness is now shown in its window (new in nv-rs: \
the original shows it only in exclusive full screen), so a brightness saved in FalloutPrefs.ini \
(fGamma), also one saved while playing the original in a window, applies from now on";

/// The log line for a ramp the game's window takes: its brightness in plain words.
pub fn change_line(ramp: &Ramp) -> String {
    match (ramp.is_identity(), ramp.gamma()) {
        (true, _) => {
            "brightness 1.00 (the default): off, the window shows the picture as drawn".to_string()
        }
        (false, Some(gamma)) => format!("brightness {gamma:.2} applied in the window"),
        (false, None) => {
            "brightness applied in the window (a ramp not of the game's curve)".to_string()
        }
    }
}

/// The shortest time between two brightness lines ([`Said`]).
pub const SAY_EVERY: Duration = Duration::from_secs(1);

/// Which brightness lines ([`change_line`]) are printed: a line only when it differs from the last
/// one printed (its value to two places), at most once in [`SAY_EVERY`]; a line held back is
/// printed at the first `Present` once [`SAY_EVERY`] has passed without a new ramp (the value it
/// settled at). `SetGamma` sent every frame then prints about a line a second while the value keeps
/// changing, and none while it stays: never a line every frame on the render thread.
#[derive(Debug, Default)]
pub struct Said {
    /// The last line printed, and when.
    last: Option<(String, Instant)>,
    /// The newest line not printed yet, and when its ramp came.
    held: Option<(String, Instant)>,
}

impl Said {
    /// A ramp's line at `now`: the line to print now, if any.
    pub fn ramp(&mut self, line: String, now: Instant) -> Option<String> {
        if self.last.as_ref().is_some_and(|(said, _)| *said == line) {
            // Back at the value printed last: nothing new to say.
            self.held = None;
            return None;
        }
        let due = self.last.as_ref().map_or(true, |(_, at)| {
            now.saturating_duration_since(*at) >= SAY_EVERY
        });
        if due {
            self.held = None;
            self.last = Some((line.clone(), now));
            Some(line)
        } else {
            self.held = Some((line, now));
            None
        }
    }

    /// At a `Present` at `now`: the line held back, once its value has stood for [`SAY_EVERY`].
    pub fn present(&mut self, now: Instant) -> Option<String> {
        let settled = self
            .held
            .as_ref()
            .is_some_and(|(_, at)| now.saturating_duration_since(*at) >= SAY_EVERY);
        if !settled {
            return None;
        }
        let (line, _) = self.held.take()?;
        self.last = Some((line.clone(), now));
        Some(line)
    }
}

/// The F12 report's place line for the brightness the window shows ([`shown`]): the report's
/// picture is the back buffer, which never has it.
pub fn place_line() -> String {
    match shown() {
        None => "brightness in the window: none (the picture as drawn)\n".to_string(),
        Some(gamma) if gamma.is_nan() => {
            "brightness in the window: a ramp not of the game's curve (picture.bmp is without it)\n"
                .to_string()
        }
        Some(gamma) => {
            format!("brightness in the window: {gamma:.2} (fGamma; picture.bmp is without it)\n")
        }
    }
}

/// The 8-bit level an entry is shown as: entry / 65535, to the nearest of 255 steps. [`SHADER`]
/// works it out in integers and writes it as level / 255, which every GPU stores in an 8-bit frame
/// as that level (as the plain copy stores a level it read); a float written as entry / 65535 may
/// be stored as either of the two nearest levels (a GPU here stored 3.53 as 3).
pub fn level(entry: u16) -> u8 {
    ((u32::from(entry) * 255 + 32767) / 65535) as u8
}

/// The copy of the back buffer into the window's frame through the ramp: the device's plain copy
/// (`device.rs`'s `BLIT`: the same rectangle and filtering) with the ramp as a fourth binding.
/// A level is the nearest of 256 to the sampled colour (exact for a copy at the same size; a
/// stretched copy's blend is taken to its nearest level first, as an 8-bit frame would hold it).
pub(crate) const SHADER: &str = "
struct Rect { uv: vec4<f32> }
@group(0) @binding(0) var t: texture_2d<f32>;
@group(0) @binding(1) var s: sampler;
@group(0) @binding(2) var<uniform> r: Rect;
@group(0) @binding(3) var<uniform> ramp: array<vec4<u32>, 256>;
struct V { @builtin(position) pos: vec4<f32>, @location(0) uv: vec2<f32> }
@vertex fn vs(@builtin(vertex_index) i: u32) -> V {
    let c = vec2<f32>(f32((i << 1u) & 2u), f32(i & 2u));
    var v: V;
    v.pos = vec4<f32>(c.x * 2.0 - 1.0, 1.0 - c.y * 2.0, 0.0, 1.0);
    v.uv = mix(r.uv.xy, r.uv.zw, c);
    return v;
}
@fragment fn fs(v: V) -> @location(0) vec4<f32> {
    let c = textureSample(t, s, v.uv);
    let k = min(vec3<u32>(round(saturate(c.rgb) * 255.0)), vec3<u32>(255u));
    let w = vec3<u32>(ramp[k.r].x, ramp[k.g].y, ramp[k.b].z);
    let l = (w * 255u + vec3<u32>(32767u)) / vec3<u32>(65535u);
    return vec4<f32>(vec3<f32>(l) / 255.0, c.a);
}
";

/// The bind group layout of [`SHADER`]: the plain copy's texture, sampler and rectangle, then
/// the ramp.
pub(crate) fn layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    let uniform = |binding, visibility| wgpu::BindGroupLayoutEntry {
        binding,
        visibility,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Uniform,
            has_dynamic_offset: false,
            min_binding_size: None,
        },
        count: None,
    };
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("brightness"),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
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
            uniform(2, wgpu::ShaderStages::VERTEX),
            uniform(3, wgpu::ShaderStages::FRAGMENT),
        ],
    })
}

/// [`SHADER`]'s pipeline into a frame of `format` (every channel written).
pub(crate) fn pipeline(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    format: wgpu::TextureFormat,
) -> wgpu::RenderPipeline {
    let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("brightness"),
        source: wgpu::ShaderSource::Wgsl(SHADER.into()),
    });
    let pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: None,
        bind_group_layouts: &[Some(layout)],
        immediate_size: 0,
    });
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("brightness"),
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
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        multiview_mask: None,
        cache: None,
    })
}

/// The ramp's uniform buffer ([`Ramp::uniform_bytes`]), filled at its making: a ramp changes
/// rarely (the player moving the slider), and a new buffer never changes a frame already on its
/// way to the GPU.
pub(crate) fn buffer(device: &wgpu::Device, ramp: &Ramp) -> wgpu::Buffer {
    let bytes = ramp.uniform_bytes();
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("brightness ramp"),
        size: bytes.len() as u64,
        usage: wgpu::BufferUsages::UNIFORM,
        mapped_at_creation: true,
    });
    if let Ok(mut mapped) = buffer.slice(..).get_mapped_range_mut() {
        mapped.copy_from_slice(&bytes);
    }
    buffer.unmap();
    buffer
}

/// The game's ramp for `gamma` as `004dcef0` builds it: entry `i` is
/// `(i / 255)^gamma * 65535 + 0.5`, truncated (`i / 255` rounded to a float, the power a float,
/// the rest in double), the same for red, green and blue. For tests (the platform passes the
/// game's own ramp on).
pub fn games_ramp(gamma: f32) -> Ramp {
    let colour: [u16; ENTRIES] = std::array::from_fn(|i| {
        let level = (i as f64 / 255.0) as f32;
        let curve = level.powf(gamma);
        (f64::from(curve) * 65535.0 + 0.5) as u16
    });
    Ramp([colour; 3])
}

/// The brightness slider's 16 values with its default ends (`fGammaMin:Display` 1.4, step 0, the
/// darkest; `fGammaMax:Display` 0.6, step 15, the brightest), as `007d19c0` makes them:
/// `min + (max - min) * step / 15`, the step's fraction a float. For tests (the game's x87 steps
/// may round a last bit otherwise; the ramp is what the platform passes on).
pub fn menu_gammas() -> [f32; 16] {
    let (min, max) = (1.4f32, 0.6f32);
    std::array::from_fn(|step| {
        let ratio = (step as f64 / 15.0) as f32;
        ((f64::from(max) - f64::from(min)) * f64::from(ratio) + f64::from(min)) as f32
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A device on this machine's first API with an adapter (none: the GPU test is skipped).
    fn device() -> Option<(wgpu::Device, wgpu::Queue)> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            ..wgpu::InstanceDescriptor::new_without_display_handle()
        });
        let adapter =
            crate::device::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                force_fallback_adapter: false,
                compatible_surface: None,
                apply_limit_buckets: false,
            }))
            .ok()?;
        crate::device::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default())).ok()
    }

    /// The test picture: 256 pixels in a row, pixel `x` with red `x`, green `255 - x`, blue
    /// `(x + 85) % 256` and alpha `x` (each colour takes every level once, in a different order).
    fn picture() -> Vec<[u8; 4]> {
        (0..=255u8)
            .map(|x| [x, 255 - x, x.wrapping_add(85), x])
            .collect()
    }

    /// [`picture`] copied through `ramp` by [`SHADER`]'s pipeline into a B8G8R8A8 frame of its
    /// size on `device`, read back as red, green, blue, alpha.
    fn through((device, queue): &(wgpu::Device, wgpu::Queue), ramp: &Ramp) -> Vec<[u8; 4]> {
        let size = wgpu::Extent3d {
            width: 256,
            height: 1,
            depth_or_array_layers: 1,
        };
        let format = wgpu::TextureFormat::Bgra8Unorm;
        let texture = |usage| {
            device.create_texture(&wgpu::TextureDescriptor {
                label: None,
                size,
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format,
                usage,
                view_formats: &[],
            })
        };
        let source = texture(wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST);
        let bgra: Vec<u8> = picture()
            .iter()
            .flat_map(|&[r, g, b, a]| [b, g, r, a])
            .collect();
        queue.write_texture(
            source.as_image_copy(),
            &bgra,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(256 * 4),
                rows_per_image: Some(1),
            },
            size,
        );
        let target =
            texture(wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC);
        let layout = layout(device);
        let pipeline = pipeline(device, &layout, format);
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor::default());
        let rect = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: 16,
            usage: wgpu::BufferUsages::UNIFORM,
            mapped_at_creation: true,
        });
        if let Ok(mut m) = rect.slice(..).get_mapped_range_mut() {
            let uv: Vec<u8> = [0f32, 0.0, 1.0, 1.0]
                .iter()
                .flat_map(|v| v.to_le_bytes())
                .collect();
            m.copy_from_slice(&uv);
        }
        rect.unmap();
        let ramp = buffer(device, ramp);
        let view = source.create_view(&Default::default());
        let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: rect.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: ramp.as_entire_binding(),
                },
            ],
        });
        let out = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: 256 * 4,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut encoder = device.create_command_encoder(&Default::default());
        {
            let target_view = target.create_view(&Default::default());
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &target_view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
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
        encoder.copy_texture_to_buffer(
            target.as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &out,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(256 * 4),
                    rows_per_image: Some(1),
                },
            },
            size,
        );
        queue.submit([encoder.finish()]);
        out.slice(..).map_async(wgpu::MapMode::Read, |_| {});
        let _ = device.poll(wgpu::PollType::wait_indefinitely());
        let data = out
            .slice(..)
            .get_mapped_range()
            .expect("the frame read back")
            .to_vec();
        data.chunks_exact(4)
            .map(|p| [p[2], p[1], p[0], p[3]])
            .collect()
    }

    /// The bytes of a ramp whose red, green and blue entries are `f(colour, i)`.
    fn bytes(f: impl Fn(usize, usize) -> u16) -> Vec<u8> {
        (0..3)
            .flat_map(|c| (0..ENTRIES).map(move |i| (c, i)))
            .flat_map(|(c, i)| f(c, i).to_le_bytes())
            .collect()
    }

    /// `SetGammaRamp`'s bytes are red's 256 entries, then green's, then blue's, little-endian;
    /// another length is not a ramp.
    #[test]
    fn the_ramps_bytes_are_red_then_green_then_blue() {
        let made = bytes(|c, i| (c * 1000 + i) as u16);
        let ramp = Ramp::from_bytes(&made).expect("a ramp");
        assert_eq!(ramp.to_bytes(), made, "and back");
        assert_eq!(ramp.0[0][0], 0);
        assert_eq!(ramp.0[0][255], 255);
        assert_eq!(ramp.0[1][7], 1007);
        assert_eq!(ramp.0[2][255], 2255);
        assert_eq!(Ramp::from_bytes(&[0; 0x5FE]), None);
        assert_eq!(Ramp::from_bytes(&[0; 0x602]), None);
        let words = ramp.uniform_bytes();
        assert_eq!(words.len(), 256 * 16);
        let word = |n: usize| u32::from_le_bytes(words[n * 4..n * 4 + 4].try_into().unwrap());
        // Level 7: red, green, blue, 0.
        assert_eq!([word(28), word(29), word(30), word(31)], [7, 1007, 2007, 0]);
    }

    /// Which ramps change nothing: Direct3D 9's own (`i * 257`) and the game's at `fGamma` 1.0
    /// do; every value the brightness slider offers changes the picture (1.0 lies between two of
    /// its steps); `i * 256` darkens the brightest level by one, and one changed entry is enough.
    #[test]
    fn the_games_ramp_at_gamma_one_changes_nothing_and_every_slider_step_does() {
        let direct3d = Ramp::from_bytes(&bytes(|_, i| (i * 257) as u16)).unwrap();
        assert!(direct3d.is_identity());
        assert_eq!(
            games_ramp(1.0),
            direct3d,
            "the game's ramp at 1.0 is Direct3D 9's"
        );
        for (step, gamma) in menu_gammas().into_iter().enumerate() {
            assert!(!games_ramp(gamma).is_identity(), "step {step}: {gamma}");
        }
        assert!(!Ramp::from_bytes(&bytes(|_, i| (i * 256) as u16))
            .unwrap()
            .is_identity());
        assert_eq!(level(255 * 256), 254);
        let mut one = direct3d.clone();
        one.0[2][100] = 101 * 257;
        assert!(!one.is_identity());
    }

    /// The brightness read back from a ramp is the game's `fGamma`: each slider step's own to a
    /// thousandth, 1.0 for Direct3D 9's identity; none where the middle entry is 0 or full.
    #[test]
    fn a_ramps_brightness_is_the_games_fgamma() {
        for (step, gamma) in menu_gammas().into_iter().enumerate() {
            let back = games_ramp(gamma).gamma().expect("a curve of the game's");
            assert!(
                (back - gamma).abs() < 0.001,
                "step {step}: {back} for {gamma}"
            );
        }
        let identity = Ramp::from_bytes(&bytes(|_, i| (i * 257) as u16)).unwrap();
        let one = identity.gamma().expect("a curve");
        assert!((one - 1.0).abs() < 0.001, "{one}");
        for middle in [0, u16::MAX] {
            let mut flat = identity.clone();
            flat.0[0][128] = middle;
            assert_eq!(flat.gamma(), None, "{middle}");
        }
    }

    /// The brightness lines are bounded. 100 ramps of different values one frame apart (60 a
    /// second, as `SetGamma` sent by a mod every frame), then two seconds of presents: three lines,
    /// the first value at once, one a second later while they keep changing, and the value they
    /// settled at a second after the last. 100 ramps of one value, or of values the same to two
    /// places: one line. A value back at the last one printed before its line was due: nothing more.
    #[test]
    fn a_ramp_every_frame_prints_a_line_a_second_at_most() {
        let frame = Duration::from_micros(16_667);
        let run = |gammas: &[f32]| {
            let mut said = Said::default();
            let mut printed = Vec::new();
            let mut t = Instant::now();
            for &gamma in gammas {
                printed.extend(said.ramp(change_line(&games_ramp(gamma)), t));
                printed.extend(said.present(t));
                t += frame;
            }
            for _ in 0..120 {
                printed.extend(said.present(t));
                t += frame;
            }
            printed
        };
        let changing: Vec<f32> = (0..100).map(|n| 0.6 + n as f32 * 0.008).collect();
        let printed = run(&changing);
        assert_eq!(printed.len(), 3, "{printed:?}");
        assert_eq!(printed[0], "brightness 0.60 applied in the window");
        assert_eq!(printed[2], change_line(&games_ramp(changing[99])));
        assert_eq!(run(&[1.4; 100]).len(), 1);
        let close: Vec<f32> = (0..100).map(|n| 1.401 + n as f32 * 0.00003).collect();
        assert_eq!(run(&close), vec!["brightness 1.40 applied in the window"]);
        // Back at the printed value before the next line was due: the held one is dropped.
        let mut said = Said::default();
        let t = Instant::now();
        let a = change_line(&games_ramp(1.4));
        assert_eq!(said.ramp(a.clone(), t), Some(a.clone()));
        assert_eq!(said.ramp(change_line(&games_ramp(0.6)), t + frame), None);
        assert_eq!(said.ramp(a, t + frame * 2), None);
        assert_eq!(said.present(t + Duration::from_secs(5)), None);
    }

    /// The brightness in plain words: a ramp's log line (a step's value to two places, the
    /// identity as off, a ramp not of the game's curve as such), and the F12 report's place line
    /// from what the window shows, each one line.
    #[test]
    fn the_brightness_is_said_in_plain_words() {
        let g = menu_gammas();
        assert_eq!(
            change_line(&games_ramp(g[15])),
            "brightness 0.60 applied in the window"
        );
        assert_eq!(
            change_line(&games_ramp(g[0])),
            "brightness 1.40 applied in the window"
        );
        let identity = Ramp::from_bytes(&bytes(|_, i| (i * 257) as u16)).unwrap();
        assert_eq!(
            change_line(&identity),
            "brightness 1.00 (the default): off, the window shows the picture as drawn"
        );
        let mut own = identity.clone();
        own.0[0][128] = u16::MAX;
        assert_eq!(
            change_line(&own),
            "brightness applied in the window (a ramp not of the game's curve)"
        );
        // What the window shows, for the report (the only test that sets it).
        set_shown(None);
        assert_eq!(shown(), None);
        assert_eq!(
            place_line(),
            "brightness in the window: none (the picture as drawn)\n"
        );
        set_shown(Some(&games_ramp(g[15])));
        assert!(shown().is_some_and(|s| (s - 0.6).abs() < 0.001));
        assert_eq!(
            place_line(),
            "brightness in the window: 0.60 (fGamma; picture.bmp is without it)\n"
        );
        set_shown(Some(&own));
        assert!(shown().is_some_and(f32::is_nan));
        assert_eq!(
            place_line(),
            "brightness in the window: a ramp not of the game's curve (picture.bmp is without it)\n"
        );
        set_shown(None);
        for line in [FIRST_LINE.to_string(), change_line(&own), place_line()] {
            assert_eq!(line.trim_end().lines().count(), 1, "{line}");
        }
    }

    /// The slider's ends and middle as `007d19c0` makes them from its default ends: the darkest
    /// step 1.4, the brightest 0.6, every step brighter than the one before.
    #[test]
    fn the_sliders_steps_run_from_the_darkest_to_the_brightest() {
        let g = menu_gammas();
        assert_eq!(g[0], 1.4);
        assert_eq!(g[15], 0.6);
        assert!(g.windows(2).all(|w| w[1] < w[0]), "{g:?}");
        // The middle of mid grey: darker at the first step, brighter at the last.
        assert!(level(games_ramp(g[0]).0[0][128]) < 128);
        assert!(level(games_ramp(g[15]).0[0][128]) > 128);
    }

    /// The pass on the GPU, on every system with an adapter: Direct3D 9's identity ramp gives
    /// every pixel back unchanged (alpha too); a ramp with a different curve per colour (red
    /// inverted, green a step at 128, blue a fixed level) maps each colour by its own; and the
    /// ramp the game sets for each of the slider's 16 steps maps every level to its entry's
    /// [`level`] exactly (level 12 at the darkest step: entry 908, 3.53 levels, shown as 4).
    #[test]
    fn the_pass_maps_every_level_through_its_colours_entry() {
        let Some(gpu) = device() else {
            eprintln!("no GPU adapter: the brightness pass test is skipped");
            return;
        };
        let identity = Ramp::from_bytes(&bytes(|_, i| (i * 257) as u16)).unwrap();
        assert_eq!(
            through(&gpu, &identity),
            picture(),
            "Direct3D 9's identity ramp: the picture unchanged"
        );
        let curves = Ramp::from_bytes(&bytes(|c, i| match c {
            0 => (65535 - i * 257) as u16,
            1 if i < 128 => 0,
            1 => 65535,
            _ => 100 * 257,
        }))
        .unwrap();
        let out = through(&gpu, &curves);
        for (x, (seen, [r, g, _, a])) in out.iter().zip(picture()).enumerate() {
            let step = if g < 128 { 0 } else { 255 };
            assert_eq!(*seen, [255 - r, step, 100, a], "pixel {x}");
        }
        for (step, gamma) in menu_gammas().into_iter().enumerate() {
            let ramp = games_ramp(gamma);
            let out = through(&gpu, &ramp);
            for (x, (seen, want)) in out.iter().zip(picture()).enumerate() {
                for c in 0..3 {
                    let entry = ramp.0[c][usize::from(want[c])];
                    assert_eq!(
                        seen[c],
                        level(entry),
                        "step {step} ({gamma}), pixel {x}, colour {c}: level {} entry {entry}",
                        want[c]
                    );
                }
                assert_eq!(seen[3], want[3], "step {step}, pixel {x}: alpha kept");
            }
        }
    }
}
