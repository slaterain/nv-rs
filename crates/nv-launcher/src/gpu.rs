//! The launcher's drawing: the terminal's picture ([`crate::canvas::Canvas`]) as one texture,
//! scaled to the window (or to a capture) with its 4:3 shape kept and black around it; a whole
//! multiple of its size is scaled by repeating pixels, any other size smoothly.
//!
//! The launcher has to start where the game's own renderer does not: it is where the player
//! changes the renderer. So it asks for the plainest device there is (no features, the lowest
//! limits), one API at a time, each in an instance of its own so a broken driver of the next one
//! is never even loaded while an earlier one works ([`launcher_apis`], [`pick`]): on Windows
//! Direct3D 12 first, then Vulkan; elsewhere the platform's own (Vulkan, Metal). A graphics card
//! first (the one built into the processor before a separate one), and only when no API has one,
//! the system's software adapter (Windows' WARP is always there). Its whole picture is drawn on
//! the processor, so even the software adapter draws it at once.

use std::sync::Arc;

const SHADER: &str = "
@group(0) @binding(0) var t: texture_2d<f32>;
@group(0) @binding(1) var s: sampler;
struct V { @builtin(position) pos: vec4<f32>, @location(0) uv: vec2<f32> };
@vertex fn vs(@builtin(vertex_index) i: u32) -> V {
    let x = f32((i << 1u) & 2u);
    let y = f32(i & 2u);
    var v: V;
    v.pos = vec4<f32>(x * 2.0 - 1.0, 1.0 - y * 2.0, 0.0, 1.0);
    v.uv = vec2<f32>(x, y);
    return v;
}
@fragment fn fs(v: V) -> @location(0) vec4<f32> {
    return vec4<f32>(textureSample(t, s, v.uv).rgb, 1.0);
}
";

pub struct Gpu {
    pub adapter: wgpu::Adapter,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    layout: wgpu::BindGroupLayout,
    module: wgpu::ShaderModule,
    pipeline: Option<(wgpu::TextureFormat, wgpu::RenderPipeline)>,
    picture: wgpu::Texture,
    nearest: wgpu::BindGroup,
    smooth: wgpu::BindGroup,
}

/// wgpu's futures are ready once polled after their work: no executor needed.
fn block_on<F: std::future::Future>(f: F) -> F::Output {
    let mut cx = std::task::Context::from_waker(std::task::Waker::noop());
    let mut f = std::pin::pin!(f);
    loop {
        if let std::task::Poll::Ready(v) = f.as_mut().poll(&mut cx) {
            return v;
        }
        std::thread::yield_now();
    }
}

/// The APIs the launcher draws with on this system, in the order it tries them, one instance each.
pub fn launcher_apis() -> Vec<wgpu::Backends> {
    if cfg!(windows) {
        vec![wgpu::Backends::DX12, wgpu::Backends::VULKAN]
    } else {
        vec![wgpu::Backends::PRIMARY]
    }
}

/// The adapters to try among one API's `kinds`, in order: with `software` false the graphics
/// cards, the plainest first (built into the processor, then separate, virtual, any other); with
/// `software` true only the system's software adapter.
pub fn order(kinds: &[wgpu::DeviceType], software: bool) -> Vec<usize> {
    let rank = |t: wgpu::DeviceType| match t {
        wgpu::DeviceType::IntegratedGpu => 0,
        wgpu::DeviceType::DiscreteGpu => 1,
        wgpu::DeviceType::VirtualGpu => 2,
        wgpu::DeviceType::Other => 3,
        wgpu::DeviceType::Cpu => 4,
    };
    let mut out: Vec<usize> = (0..kinds.len())
        .filter(|&i| (kinds[i] == wgpu::DeviceType::Cpu) == software)
        .collect();
    out.sort_by_key(|&i| rank(kinds[i]));
    out
}

/// The first adapter [`order`] tries (the tests' shorthand).
#[cfg(test)]
fn pick(kinds: &[wgpu::DeviceType], software: bool) -> Option<usize> {
    order(kinds, software).first().copied()
}

/// The order of the tries: every API's graphics card first, then every API's software adapter.
pub fn tries() -> Vec<(wgpu::Backends, bool)> {
    [false, true]
        .into_iter()
        .flat_map(|software| launcher_apis().into_iter().map(move |api| (api, software)))
        .collect()
}

/// The launcher's own drawing, ready: its API's instance (kept as long as the device), the
/// device, and the window's surface configured for it.
pub struct Drawing {
    pub instance: wgpu::Instance,
    pub gpu: Gpu,
    pub surface: Option<(wgpu::Surface<'static>, wgpu::SurfaceConfiguration)>,
}

/// The first adapter of [`tries`] (and of [`order`] in each) that makes a device and, with
/// `window` (its size `size`), a surface configuration: an adapter that is listed but cannot
/// make either is passed over for the next one, down to the software adapter.
pub fn choose(
    window: Option<Arc<winit::window::Window>>,
    size: (u32, u32),
) -> Result<Drawing, String> {
    let mut why = Vec::new();
    for (backends, software) in tries() {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends,
            ..wgpu::InstanceDescriptor::new_without_display_handle()
        });
        let surface = match &window {
            Some(w) => match instance.create_surface(w.clone()) {
                Ok(s) => Some(s),
                Err(e) => {
                    why.push(format!("{backends:?}: {e}"));
                    continue;
                }
            },
            None => None,
        };
        let mut adapters: Vec<Option<wgpu::Adapter>> =
            block_on(instance.enumerate_adapters(backends))
                .into_iter()
                .filter(|a| surface.as_ref().is_none_or(|s| a.is_surface_supported(s)))
                .map(Some)
                .collect();
        let kinds: Vec<wgpu::DeviceType> = adapters
            .iter()
            .flatten()
            .map(|a| a.get_info().device_type)
            .collect();
        for i in order(&kinds, software) {
            let Some(adapter) = adapters[i].take() else {
                continue;
            };
            let name = adapter.get_info().name;
            let gpu = match Gpu::new(adapter, (crate::screen::WIDTH, crate::screen::HEIGHT)) {
                Ok(g) => g,
                Err(e) => {
                    why.push(format!("{name}: {e}"));
                    continue;
                }
            };
            let Some(s) = &surface else {
                return Ok(Drawing {
                    instance,
                    gpu,
                    surface: None,
                });
            };
            let Some(mut config) = s.get_default_config(&gpu.adapter, size.0.max(1), size.1.max(1))
            else {
                why.push(format!(
                    "{name}: the window's surface does not work with it"
                ));
                continue;
            };
            // The game's menus blend in the stored values: a format without the sRGB step keeps
            // them.
            let caps = s.get_capabilities(&gpu.adapter);
            if let Some(f) = caps.formats.iter().find(|f| !f.is_srgb()) {
                config.format = *f;
            }
            s.configure(&gpu.device, &config);
            return Ok(Drawing {
                instance,
                gpu,
                surface: surface.map(|s| (s, config)),
            });
        }
    }
    Err(format!(
        "no graphics device draws it, not even the software one{}",
        if why.is_empty() {
            String::new()
        } else {
            format!(" ({})", why.join("; "))
        }
    ))
}

impl Gpu {
    /// A device on `adapter` for the picture `size` (width, height).
    pub fn new(adapter: wgpu::Adapter, size: (u32, u32)) -> Result<Gpu, String> {
        let (device, queue) = block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("nv-launcher"),
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits::downlevel_defaults().using_resolution(adapter.limits()),
            memory_hints: wgpu::MemoryHints::MemoryUsage,
            experimental_features: Default::default(),
            trace: Default::default(),
        }))
        .map_err(|e| format!("the graphics device could not be made: {e}"))?;
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("terminal"),
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
            ],
        });
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("terminal"),
            source: wgpu::ShaderSource::Wgsl(SHADER.into()),
        });
        let picture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("terminal picture"),
            size: wgpu::Extent3d {
                width: size.0,
                height: size.1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let view = picture.create_view(&Default::default());
        let group = |filter: wgpu::FilterMode| {
            let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
                label: Some("terminal"),
                mag_filter: filter,
                min_filter: filter,
                ..Default::default()
            });
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("terminal"),
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
                ],
            })
        };
        let nearest = group(wgpu::FilterMode::Nearest);
        let smooth = group(wgpu::FilterMode::Linear);
        Ok(Gpu {
            adapter,
            device,
            queue,
            layout,
            module,
            pipeline: None,
            picture,
            nearest,
            smooth,
        })
    }

    /// The adapter's name and API, for the log.
    pub fn describe(&self) -> String {
        let info = self.adapter.get_info();
        format!("{} ({:?})", info.name, info.backend)
    }

    /// The picture's pixels (RGBA rows, the texture's size).
    pub fn upload(&self, rgba: &[u8], size: (u32, u32)) {
        self.queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &self.picture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            rgba,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(size.0 * 4),
                rows_per_image: Some(size.1),
            },
            wgpu::Extent3d {
                width: size.0,
                height: size.1,
                depth_or_array_layers: 1,
            },
        );
    }

    fn pipeline(&mut self, format: wgpu::TextureFormat) -> &wgpu::RenderPipeline {
        if self.pipeline.as_ref().map(|(f, _)| *f) != Some(format) {
            let pl = self
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: None,
                    bind_group_layouts: &[Some(&self.layout)],
                    immediate_size: 0,
                });
            let pipeline = self
                .device
                .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                    label: Some("terminal"),
                    layout: Some(&pl),
                    vertex: wgpu::VertexState {
                        module: &self.module,
                        entry_point: Some("vs"),
                        compilation_options: Default::default(),
                        buffers: &[],
                    },
                    primitive: Default::default(),
                    depth_stencil: None,
                    multisample: Default::default(),
                    fragment: Some(wgpu::FragmentState {
                        module: &self.module,
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
                });
            self.pipeline = Some((format, pipeline));
        }
        &self.pipeline.as_ref().expect("just made").1
    }

    /// The picture (`picture` its size) drawn into `target` (`target_size`, `format`): black
    /// around it, its shape kept.
    pub fn draw(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        format: wgpu::TextureFormat,
        target_size: (u32, u32),
        picture: (u32, u32),
    ) {
        let (x, y, w, h) = fit(picture, target_size);
        let whole = w % picture.0 == 0 && h % picture.1 == 0;
        let group = if whole {
            self.nearest.clone()
        } else {
            self.smooth.clone()
        };
        let pipeline = self.pipeline(format).clone();
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("terminal"),
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
        if w == 0 || h == 0 {
            return;
        }
        pass.set_viewport(x as f32, y as f32, w as f32, h as f32, 0.0, 1.0);
        pass.set_pipeline(&pipeline);
        pass.set_bind_group(0, &group, &[]);
        pass.draw(0..3, 0..1);
    }

    /// The picture drawn at `size` off screen and read back: RGBA rows.
    pub fn capture(&mut self, picture: (u32, u32), size: (u32, u32)) -> Result<Vec<u8>, String> {
        let format = wgpu::TextureFormat::Rgba8Unorm;
        let target = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("capture"),
            size: wgpu::Extent3d {
                width: size.0,
                height: size.1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let view = target.create_view(&Default::default());
        let row = (size.0 * 4).div_ceil(256) * 256;
        let buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("capture"),
            size: (row * size.1) as u64,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut encoder = self.device.create_command_encoder(&Default::default());
        self.draw(&mut encoder, &view, format, size, picture);
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &target,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(row),
                    rows_per_image: Some(size.1),
                },
            },
            wgpu::Extent3d {
                width: size.0,
                height: size.1,
                depth_or_array_layers: 1,
            },
        );
        self.queue.submit([encoder.finish()]);
        let slice = buffer.slice(..);
        let done = Arc::new(std::sync::Mutex::new(None));
        {
            let done = done.clone();
            slice.map_async(wgpu::MapMode::Read, move |r| {
                *done.lock().expect("not poisoned") = Some(r);
            });
        }
        let _ = self.device.poll(wgpu::PollType::wait_indefinitely());
        match done.lock().expect("not poisoned").take() {
            Some(Ok(())) => {}
            other => return Err(format!("the capture could not be read back: {other:?}")),
        }
        let mapped = slice
            .get_mapped_range()
            .map_err(|e| format!("the capture could not be read back: {e:?}"))?;
        let mut out = Vec::with_capacity((size.0 * size.1 * 4) as usize);
        for y in 0..size.1 {
            let at = (y * row) as usize;
            out.extend_from_slice(&mapped[at..at + (size.0 * 4) as usize]);
        }
        Ok(out)
    }
}

/// The graphics card the GAME will draw with on Vulkan (`sRenderer=vulkan`) and the API it will
/// land on, by the game's own rule (`d3d9_wgpu::device::card_facts`, which the start's
/// `request_adapter` with high performance follows): the APIs in the game's order (Vulkan, then
/// Direct3D 12 on Windows; the platform's own elsewhere), on the first that has a card a separate
/// card before one built into the processor, then a virtual one, then any other; never the
/// software adapter. The launcher's own drawing keeps its plainest device ([`Gpu::new`]).
pub fn game_card() -> Option<(String, wgpu::Backend)> {
    let tries = if cfg!(windows) {
        vec![wgpu::Backends::VULKAN, wgpu::Backends::DX12]
    } else {
        vec![wgpu::Backends::PRIMARY]
    };
    let rank = |t: wgpu::DeviceType| match t {
        wgpu::DeviceType::DiscreteGpu => 0,
        wgpu::DeviceType::IntegratedGpu => 1,
        wgpu::DeviceType::VirtualGpu => 2,
        wgpu::DeviceType::Other => 3,
        wgpu::DeviceType::Cpu => 4,
    };
    tries.into_iter().find_map(|backends| {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends,
            ..wgpu::InstanceDescriptor::new_without_display_handle()
        });
        block_on(instance.enumerate_adapters(backends))
            .into_iter()
            .map(|a| a.get_info())
            .filter(|i| i.device_type != wgpu::DeviceType::Cpu)
            .min_by_key(|i| rank(i.device_type))
            .map(|i| (i.name, i.backend))
    })
}

/// Where a picture of `picture`'s size goes in `target`, centred: the largest size of its shape
/// that fits, or the largest whole multiple of it when that is at least nine tenths as large (its
/// pixels stay sharp).
pub fn fit(picture: (u32, u32), target: (u32, u32)) -> (u32, u32, u32, u32) {
    let s = (target.0 as f32 / picture.0 as f32).min(target.1 as f32 / picture.1 as f32);
    let whole = s.floor();
    let s = if whole >= 1.0 && whole >= s * 0.9 {
        whole
    } else {
        s
    };
    let (w, h) = (
        ((picture.0 as f32 * s) as u32).min(target.0),
        ((picture.1 as f32 * s) as u32).min(target.1),
    );
    ((target.0 - w) / 2, (target.1 - h) / 2, w, h)
}

/// The API's name as the start screen says it.
pub fn api_name(backend: wgpu::Backend) -> &'static str {
    match backend {
        wgpu::Backend::Dx12 => "Direct3D 12",
        wgpu::Backend::Metal => "Metal",
        wgpu::Backend::Gl => "OpenGL",
        _ => "Vulkan",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wgpu::DeviceType::{Cpu, DiscreteGpu, IntegratedGpu, Other, VirtualGpu};

    /// A card first, the one built into the processor before a separate one; the software adapter
    /// only on the software try, and never a card then.
    #[test]
    fn the_plainest_card_first_and_software_last() {
        assert_eq!(pick(&[DiscreteGpu, IntegratedGpu, Cpu], false), Some(1));
        assert_eq!(pick(&[Cpu, DiscreteGpu], false), Some(1));
        assert_eq!(pick(&[Other, VirtualGpu], false), Some(1));
        assert_eq!(pick(&[Cpu], false), None);
        assert_eq!(pick(&[DiscreteGpu, Cpu], true), Some(1));
        assert_eq!(pick(&[DiscreteGpu], true), None);
        assert_eq!(pick(&[], false), None);
        // Every card is tried in that order (one that cannot make a device or a surface is
        // passed over for the next), the software adapter on its own try.
        assert_eq!(
            order(&[DiscreteGpu, Cpu, IntegratedGpu, Other, VirtualGpu], false),
            [2, 0, 4, 3]
        );
        assert_eq!(order(&[DiscreteGpu, Cpu], true), [1]);
    }

    /// Every API's card is tried before any software adapter; on Windows Direct3D 12 before
    /// Vulkan, so a broken Vulkan driver is never loaded while Direct3D 12 has a card.
    #[test]
    fn every_card_before_any_software_adapter() {
        let t = tries();
        let apis = launcher_apis();
        assert_eq!(t.len(), apis.len() * 2);
        assert!(t[..apis.len()].iter().all(|&(_, s)| !s));
        assert!(t[apis.len()..].iter().all(|&(_, s)| s));
        if cfg!(windows) {
            assert_eq!(apis, [wgpu::Backends::DX12, wgpu::Backends::VULKAN]);
        } else {
            assert_eq!(apis, [wgpu::Backends::PRIMARY]);
        }
        assert_eq!(api_name(wgpu::Backend::Dx12), "Direct3D 12");
        assert_eq!(api_name(wgpu::Backend::Vulkan), "Vulkan");
        assert_eq!(api_name(wgpu::Backend::Metal), "Metal");
    }

    /// The picture keeps its shape, centred; a size close to a whole multiple takes the multiple.
    #[test]
    fn the_picture_keeps_its_shape() {
        assert_eq!(fit((960, 720), (960, 720)), (0, 0, 960, 720));
        assert_eq!(fit((960, 720), (1920, 1440)), (0, 0, 1920, 1440));
        assert_eq!(fit((960, 720), (2000, 1440)), (40, 0, 1920, 1440));
        let (_, y, w, h) = fit((960, 720), (800, 800));
        assert!(
            (799..=800).contains(&w) && (599..=600).contains(&h),
            "{w}x{h}"
        );
        assert!((100..=101).contains(&y));
    }
}
