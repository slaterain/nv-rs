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
