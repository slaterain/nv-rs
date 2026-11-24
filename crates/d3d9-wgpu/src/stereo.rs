//! Optional screen-space stereo at wgpu's Present. Save disparity from the completed world
//! (FalloutNV.exe 1.4.0.525: world pass `00873200` in `00870bd0`/`00870a00`), before the game's
//! first-person depth clear. Reproject the finished colour image so the game's own image-space
//! effects and brightness still apply. This does not move the camera or render hidden geometry.

use crate::scene_light::Params;
use std::sync::OnceLock;

/// Private stream marker carrying the perspective camera at `world_drawn` (not at depth clears).
pub const STATE: u32 = u32::from_le_bytes(*b"NV3D");

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Anaglyph,
    CrossEyed,
}

impl Mode {
    fn parse(value: &str) -> Result<Option<Self>, ()> {
        let value = value.trim();
        if value.eq_ignore_ascii_case("anaglyph") {
            Ok(Some(Self::Anaglyph))
        } else if value.eq_ignore_ascii_case("crosseyed") {
            Ok(Some(Self::CrossEyed))
        } else if value.is_empty() || value.eq_ignore_ascii_case("off") || value == "0" {
            Ok(None)
        } else {
            Err(())
        }
    }

    pub fn requested() -> Option<Self> {
        static MODE: OnceLock<Option<Mode>> = OnceLock::new();
        *MODE.get_or_init(|| {
            let value = std::env::var_os("NV_3D")?;
            Self::parse(&value.to_string_lossy()).unwrap_or_else(|()| {
                eprintln!("NV_3D={value:?}: use anaglyph or crosseyed; stereo is off");
                None
            })
        })
    }

    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::Anaglyph => "anaglyph (red left, cyan right)",
            Self::CrossEyed => "crosseyed (right view on the left)",
        }
    }
}

const SHADER: &str = include_str!("stereo.wgsl");
const DISPARITY: wgpu::TextureFormat = wgpu::TextureFormat::R16Float;

struct Snapshot {
    frame: u64,
    size: (u32, u32),
    view: wgpu::TextureView,
}

pub(crate) struct Gpu {
    mode: Mode,
    module: wgpu::ShaderModule,
    depth: [Option<wgpu::RenderPipeline>; 2],
    outputs: Vec<(wgpu::TextureFormat, &'static str, wgpu::RenderPipeline)>,
    sampler: wgpu::Sampler,
    identity: wgpu::Buffer,
    snapshot: Option<Snapshot>,
}

impl Gpu {
    pub(crate) fn new(device: &wgpu::Device, mode: Mode) -> Self {
        Self {
            mode,
            module: device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("NV_3D"),
                source: wgpu::ShaderSource::Wgsl(SHADER.into()),
            }),
            depth: [None, None],
            outputs: Vec::new(),
            sampler: device.create_sampler(&wgpu::SamplerDescriptor {
                mag_filter: wgpu::FilterMode::Linear,
                min_filter: wgpu::FilterMode::Linear,
                ..Default::default()
            }),
            identity: crate::ramp::buffer(device, &crate::ramp::games_ramp(1.0)),
            snapshot: None,
        }
    }

    pub(crate) fn reset(&mut self) {
        self.snapshot = None;
    }

    pub(crate) fn captured(&self, frame: u64) -> bool {
        self.snapshot.as_ref().is_some_and(|s| s.frame == frame)
    }

    pub(crate) fn capture(
        &mut self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        frame: u64,
        (depth, samples, size): (&wgpu::TextureView, u32, (u32, u32)),
        params: Params,
    ) {
        if self.snapshot.as_ref().map_or(true, |s| s.size != size) {
            let texture = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("NV_3D disparity"),
                size: wgpu::Extent3d {
                    width: size.0,
                    height: size.1,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: DISPARITY,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                    | wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            });
            self.snapshot = Some(Snapshot {
                frame,
                size,
                view: texture.create_view(&Default::default()),
            });
        }
        let snapshot = self.snapshot.as_mut().expect("made above");
        snapshot.frame = frame;
        let msaa = samples > 1;
        let entry = if msaa { "save_msaa" } else { "save" };
        let pipe = self.depth[usize::from(msaa)]
            .get_or_insert_with(|| pipeline(device, &self.module, DISPARITY, entry));
        // Immutable per capture so a later frame's camera can't race a queued draw.
        use wgpu::util::DeviceExt;
        let mut bytes = [0; 16];
        for (out, f) in bytes.chunks_exact_mut(4).zip([
            params.near.recip(),
            params.far.recip(),
            (params.right - params.left).recip(),
            0.0,
        ]) {
            out.copy_from_slice(&f.to_le_bytes());
        }
        let camera = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("NV_3D camera"),
            contents: &bytes,
            usage: wgpu::BufferUsages::UNIFORM,
        });
        draw(
            device,
            encoder,
            pipe,
            &snapshot.view,
            &[
                binding(u32::from(msaa), wgpu::BindingResource::TextureView(depth)),
                binding(2, camera.as_entire_binding()),
            ],
        );
    }

    /// Returns false when Present should take its unchanged path (anaglyph on a 2D frame).
    pub(crate) fn compose(
        &mut self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        frame: u64,
        (colour, size): (&wgpu::TextureView, (u32, u32)),
        (target, format): (&wgpu::TextureView, wgpu::TextureFormat),
        ramp: Option<&wgpu::Buffer>,
    ) -> bool {
        let snapshot = self
            .snapshot
            .as_ref()
            .filter(|s| s.frame == frame && s.size == size);
        let entry = match (self.mode, snapshot.is_some()) {
            (Mode::Anaglyph, false) => return false,
            (Mode::Anaglyph, true) => "anaglyph",
            (Mode::CrossEyed, false) => "crossed_flat",
            (Mode::CrossEyed, true) => "crossed",
        };
        let at = self
            .outputs
            .iter()
            .position(|p| (p.0, p.1) == (format, entry))
            .unwrap_or_else(|| {
                self.outputs
                    .push((format, entry, pipeline(device, &self.module, format, entry)));
                self.outputs.len() - 1
            });
        let pipe = &self.outputs[at].2;
        let entries = [
            binding(4, wgpu::BindingResource::TextureView(colour)),
            binding(5, wgpu::BindingResource::Sampler(&self.sampler)),
            binding(6, ramp.unwrap_or(&self.identity).as_entire_binding()),
            binding(
                3,
                wgpu::BindingResource::TextureView(snapshot.map_or(colour, |s| &s.view)),
            ),
        ];
        draw(
            device,
            encoder,
            pipe,
            target,
            &entries[..3 + usize::from(snapshot.is_some())],
        );
        true
    }
}

fn binding(binding: u32, resource: wgpu::BindingResource<'_>) -> wgpu::BindGroupEntry<'_> {
    wgpu::BindGroupEntry { binding, resource }
}

fn pipeline(
    device: &wgpu::Device,
    module: &wgpu::ShaderModule,
    format: wgpu::TextureFormat,
    entry: &str,
) -> wgpu::RenderPipeline {
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some(entry),
        layout: None,
        vertex: wgpu::VertexState {
            module,
            entry_point: Some("vs"),
            compilation_options: Default::default(),
            buffers: &[],
        },
        primitive: Default::default(),
        depth_stencil: None,
        multisample: Default::default(),
        fragment: Some(wgpu::FragmentState {
            module,
            entry_point: Some(entry),
            compilation_options: Default::default(),
            targets: &[Some(format.into())],
        }),
        multiview_mask: None,
        cache: None,
    })
}

fn draw(
    device: &wgpu::Device,
    encoder: &mut wgpu::CommandEncoder,
    pipe: &wgpu::RenderPipeline,
    target: &wgpu::TextureView,
    entries: &[wgpu::BindGroupEntry<'_>],
) {
    let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("NV_3D"),
        layout: &pipe.get_bind_group_layout(0),
        entries,
    });
    let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("NV_3D"),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view: target,
            depth_slice: None,
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                store: wgpu::StoreOp::Store,
            },
        })],
        depth_stencil_attachment: None,
        timestamp_writes: None,
        occlusion_query_set: None,
        multiview_mask: None,
    });
    pass.set_pipeline(pipe);
    pass.set_bind_group(0, &group, &[]);
    pass.draw(0..3, 0..1);
}

#[cfg(test)]
mod tests;
