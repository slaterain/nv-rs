//! The launcher's drawing: the terminal's picture ([`crate::canvas::Canvas`]) as one texture,
//! scaled to the window (or to a capture): the 4:3 screen with its shape kept and black around it
//! on a picture of the window's own shape ([`framing`]); a whole multiple of its size is scaled by
//! repeating pixels, any other size smoothly.
//!
//! The launcher has to start where the game's own renderer does not: it is where the player
//! changes the renderer. So it asks for the plainest device there is (no features, the lowest
//! limits), one API at a time, each in an instance of its own so a broken driver of the next one
//! is never even loaded while an earlier one works ([`tries`]): on Windows Direct3D 12's cards,
//! then Direct3D 12's software adapter (WARP, always there), and only then Vulkan's cards and its
//! software adapter; elsewhere the platform's own API (Vulkan, Metal), its cards, then its
//! software adapter. Among one API's cards the one built into the processor first, then a
//! separate one ([`order`]); a card whose device or window surface cannot be set up, or whose
//! device is lost while that is done, is passed over for the next ([`first_working`]). A device
//! lost later, while the launcher runs, is chosen again the same way (`app.rs`). Its whole
//! picture is drawn on the processor, so even the software adapter draws it at once.

use std::sync::atomic::{AtomicBool, Ordering};
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
    /// The two samplers: by repeating pixels, and smoothly.
    samplers: [wgpu::Sampler; 2],
    /// The picture's texture (made again when the picture's size changes: [`Gpu::upload`]) and
    /// its two bind groups, by repeating pixels and smoothly.
    picture: wgpu::Texture,
    nearest: wgpu::BindGroup,
    smooth: wgpu::BindGroup,
    /// Set when the driver loses the device ([`counts_as_lost`]): wgpu files some swap-chain
    /// failures that way (inside `configure`, outside any error scope), and a driver can lose it
    /// at any later time too.
    lost: Arc<AtomicBool>,
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

/// The order of the tries: on Windows Direct3D 12's card, then Direct3D 12's software adapter
/// (WARP, always there), and only then Vulkan's card and software adapter, so a Vulkan driver is
/// never loaded into the launcher while Direct3D 12 draws; elsewhere the platform's own API's
/// card, then its software adapter.
pub fn tries() -> Vec<(wgpu::Backends, bool)> {
    launcher_apis()
        .into_iter()
        .flat_map(|api| [(api, false), (api, true)])
        .collect()
}

/// The launcher's own drawing, ready: its API's instance (kept as long as the device), the
/// device, and the window's surface configured for it.
pub struct Drawing {
    pub instance: wgpu::Instance,
    pub gpu: Gpu,
    pub surface: Option<(wgpu::Surface<'static>, wgpu::SurfaceConfiguration)>,
}

/// The first entry that works: for each of `tries`, `open` lists that API's adapters with their
/// kinds (and what must live with them: the instance, the window's surface), and the adapters
/// [`order`] takes are handed to `make` one at a time until one works; one that fails is passed
/// over for the next, down to the last software adapter. `Err` carries every failure, in order.
pub fn first_working<C, A, T>(
    tries: &[(wgpu::Backends, bool)],
    mut open: impl FnMut(wgpu::Backends) -> Result<(C, Vec<(A, wgpu::DeviceType)>), String>,
    mut make: impl FnMut(&C, A) -> Result<T, String>,
) -> Result<(C, T), Vec<String>> {
    let mut why = Vec::new();
    for &(backends, software) in tries {
        let (ctx, listed) = match open(backends) {
            Ok(v) => v,
            Err(e) => {
                why.push(e);
                continue;
            }
        };
        let kinds: Vec<wgpu::DeviceType> = listed.iter().map(|(_, k)| *k).collect();
        let mut adapters: Vec<Option<A>> = listed.into_iter().map(|(a, _)| Some(a)).collect();
        for i in order(&kinds, software) {
            let Some(a) = adapters[i].take() else {
                continue;
            };
            match make(&ctx, a) {
                Ok(t) => return Ok((ctx, t)),
                Err(e) => why.push(e),
            }
        }
    }
    Err(why)
}

/// `surface` configured for `gpu`'s device inside error scopes (validation, internal, out of
/// memory, popped in reverse), so a driver that makes the device but refuses the swap chain is an
/// error to pass over, not a panic in wgpu's default handler.
fn configure(
    surface: &wgpu::Surface<'_>,
    gpu: &Gpu,
    size: (u32, u32),
) -> Result<wgpu::SurfaceConfiguration, String> {
    let mut config = surface
        .get_default_config(&gpu.adapter, size.0.max(1), size.1.max(1))
        .ok_or("the window's surface does not work with it")?;
    // The game's menus blend in the stored values: a format without the sRGB step keeps them.
    let caps = surface.get_capabilities(&gpu.adapter);
    if let Some(f) = caps.formats.iter().find(|f| !f.is_srgb()) {
        config.format = *f;
    }
    let validation = gpu.device.push_error_scope(wgpu::ErrorFilter::Validation);
    let internal = gpu.device.push_error_scope(wgpu::ErrorFilter::Internal);
    let memory = gpu.device.push_error_scope(wgpu::ErrorFilter::OutOfMemory);
    surface.configure(&gpu.device, &config);
    let errors = [
        block_on(memory.pop()),
        block_on(internal.pop()),
        block_on(validation.pop()),
    ];
    surface_result(
        errors.into_iter().flatten().next().map(|e| e.to_string()),
        gpu.is_lost(),
    )
    .map(|()| config)
}

/// What a surface set-up came to: an error a scope caught, or the device lost while it was done
/// (wgpu files a swap chain the driver refuses in some of its steps as a lost device, which no
/// scope sees: the card is passed over all the same, or its first frame would fail).
fn surface_result(caught: Option<String>, lost: bool) -> Result<(), String> {
    match caught {
        Some(e) => Err(format!("the window's surface could not be set up: {e}")),
        None if lost => Err("the window's surface could not be set up: the device was lost".into()),
        None => Ok(()),
    }
}

/// Whether a device-lost report means the device is gone: a driver's loss (`Unknown`), not the
/// launcher's own `destroy`.
pub fn counts_as_lost(reason: wgpu::DeviceLostReason) -> bool {
    reason == wgpu::DeviceLostReason::Unknown
}

/// The first adapter of [`tries`] (and of [`order`] in each) that makes a device and, with
/// `window` (its size `size`), a surface configuration ([`first_working`]).
pub fn choose(
    window: Option<Arc<winit::window::Window>>,
    size: (u32, u32),
) -> Result<Drawing, String> {
    let open = |backends: wgpu::Backends| {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends,
            ..wgpu::InstanceDescriptor::new_without_display_handle()
        });
        let surface = match &window {
            Some(w) => Some(
                instance
                    .create_surface(w.clone())
                    .map_err(|e| format!("{backends:?}: {e}"))?,
            ),
            None => None,
        };
        let listed: Vec<(wgpu::Adapter, wgpu::DeviceType)> =
            block_on(instance.enumerate_adapters(backends))
                .into_iter()
                .filter(|a| surface.as_ref().is_none_or(|s| a.is_surface_supported(s)))
                .map(|a| {
                    let kind = a.get_info().device_type;
                    (a, kind)
                })
                .collect();
        Ok(((instance, surface), listed))
    };
    let make = |(_, surface): &(wgpu::Instance, Option<wgpu::Surface<'static>>),
                adapter: wgpu::Adapter| {
        let name = adapter.get_info().name;
        let gpu = Gpu::new(adapter, (crate::screen::WIDTH, crate::screen::HEIGHT))
            .map_err(|e| format!("{name}: {e}"))?;
        let config = match surface {
            Some(s) => Some(configure(s, &gpu, size).map_err(|e| format!("{name}: {e}"))?),
            None => None,
        };
        Ok((gpu, config))
    };
    match first_working(&tries(), open, make) {
        Ok(((instance, surface), (gpu, config))) => Ok(Drawing {
            instance,
            gpu,
            surface: surface.zip(config),
        }),
        Err(why) => Err(format!(
            "no graphics device draws it, not even the software one{}",
            if why.is_empty() {
                String::new()
            } else {
                format!(" ({})", why.join("; "))
            }
        )),
    }
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
        // For the device's whole life an error outside an error scope (a later surface set-up on
        // a resize) is said in the log, never wgpu's default panic: the launcher is where the
        // player gets past a broken driver. A lost device is not such an error: see below.
        device.on_uncaptured_error(Arc::new(|e: wgpu::Error| {
            eprintln!("the launcher's drawing: {e}");
        }));
        // A lost device is marked (and said in the log), so the launcher chooses again instead of
        // drawing on it: inside `configure` (wgpu calls this at once there) and at any later time.
        let lost = Arc::new(AtomicBool::new(false));
        {
            let lost = lost.clone();
            device.set_device_lost_callback(move |reason, message| {
                if counts_as_lost(reason) {
                    lost.store(true, Ordering::SeqCst);
                    eprintln!("the launcher's drawing: its device was lost: {message}");
                }
            });
        }
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
        let sampler = |filter: wgpu::FilterMode| {
            device.create_sampler(&wgpu::SamplerDescriptor {
                label: Some("terminal"),
                mag_filter: filter,
                min_filter: filter,
                ..Default::default()
            })
        };
        let samplers = [
            sampler(wgpu::FilterMode::Nearest),
            sampler(wgpu::FilterMode::Linear),
        ];
        let (picture, [nearest, smooth]) = picture_texture(&device, &layout, &samplers, size);
        Ok(Gpu {
            adapter,
            device,
            queue,
            layout,
            module,
            pipeline: None,
            samplers,
            picture,
            nearest,
            smooth,
            lost,
        })
    }

    /// The largest picture the device takes, on either side.
    pub fn most_side(&self) -> u32 {
        self.device.limits().max_texture_dimension_2d
    }

    /// Whether the driver has lost the device ([`counts_as_lost`]).
    pub fn is_lost(&self) -> bool {
        self.lost.load(Ordering::SeqCst)
    }

    /// The adapter's name and API, for the log.
    pub fn describe(&self) -> String {
        let info = self.adapter.get_info();
        format!("{} ({:?})", info.name, info.backend)
    }

    /// The picture's pixels (RGBA rows, `size` the picture's): its texture made again first when
    /// the size is not the last one's (the window's shape changed).
    pub fn upload(&mut self, rgba: &[u8], size: (u32, u32)) {
        if (self.picture.width(), self.picture.height()) != size {
            let (picture, [nearest, smooth]) =
                picture_texture(&self.device, &self.layout, &self.samplers, size);
            (self.picture, self.nearest, self.smooth) = (picture, nearest, smooth);
        }
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

    /// The picture (the last one uploaded, of `framing`'s size) drawn into `target` (`format`)
    /// where `framing` puts it: black around it, its shape kept.
    pub fn draw(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        format: wgpu::TextureFormat,
        framing: &Framing,
    ) {
        let (x, y, w, h) = framing.rect;
        let picture = framing.picture;
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

    /// The picture drawn off screen at `size` where `framing` (made for that size) puts it, and
    /// read back: RGBA rows.
    pub fn capture(&mut self, framing: &Framing, size: (u32, u32)) -> Result<Vec<u8>, String> {
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
        self.draw(&mut encoder, &view, format, framing);
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

/// The picture's texture at `size` and its bind groups with each of `samplers`.
fn picture_texture(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    samplers: &[wgpu::Sampler; 2],
    size: (u32, u32),
) -> (wgpu::Texture, [wgpu::BindGroup; 2]) {
    let picture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("terminal picture"),
        size: wgpu::Extent3d {
            width: size.0.max(1),
            height: size.1.max(1),
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
    let groups = samplers.each_ref().map(|sampler| {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("terminal"),
            layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(sampler),
                },
            ],
        })
    });
    (picture, groups)
}

/// The scale a picture of `picture`'s size is drawn at in `target`: the largest that fits, or
/// the largest whole one when that is at least nine tenths as large (its pixels stay sharp).
fn scale(picture: (u32, u32), target: (u32, u32)) -> f32 {
    let s = (target.0 as f32 / picture.0 as f32).min(target.1 as f32 / picture.1 as f32);
    let whole = s.floor();
    if whole >= 1.0 && whole >= s * 0.9 {
        whole
    } else {
        s
    }
}

/// Where a picture of `picture`'s size goes in `target`, centred, at [`scale`]'s size.
#[cfg(test)]
pub fn fit(picture: (u32, u32), target: (u32, u32)) -> (u32, u32, u32, u32) {
    let s = scale(picture, target);
    let (w, h) = (
        ((picture.0 as f32 * s) as u32).min(target.0),
        ((picture.1 as f32 * s) as u32).min(target.1),
    );
    ((target.0 - w) / 2, (target.1 - h) / 2, w, h)
}

/// How the terminal's 4:3 screen (`screen`, its size in its own units) stands in a window of
/// `window` pixels: at the scale `fit` gives it, centred, with black bars beside it (a wide
/// window) or above and below it (a tall one). The picture drawn is the window's own shape at
/// that scale, the screen in its middle, so what belongs to the window (its buttons) goes in the
/// window's corner; on neither side larger than `most` (the device's largest texture).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Framing {
    /// The picture's size, in the screen's units: the screen's own at the least.
    pub picture: (u32, u32),
    /// Where the screen's top-left corner is on the picture.
    pub screen_at: (i32, i32),
    /// Where the picture goes in the window: x, y, width and height, in pixels.
    pub rect: (u32, u32, u32, u32),
}

/// [`Framing`] for `screen` in `window`, the picture at most `most` on a side.
pub fn framing(screen: (u32, u32), window: (u32, u32), most: u32) -> Framing {
    let s = scale(screen, window);
    let side = |window: u32, own: u32| {
        if s > 0.0 {
            ((window as f32 / s) as u32).clamp(own, most.max(own))
        } else {
            own
        }
    };
    let picture = (side(window.0, screen.0), side(window.1, screen.1));
    let (w, h) = (
        ((picture.0 as f32 * s) as u32).min(window.0),
        ((picture.1 as f32 * s) as u32).min(window.1),
    );
    Framing {
        picture,
        screen_at: (
            ((picture.0 - screen.0) / 2) as i32,
            ((picture.1 - screen.1) / 2) as i32,
        ),
        rect: ((window.0 - w) / 2, (window.1 - h) / 2, w, h),
    }
}

impl Framing {
    /// The window position `pos` (pixels) in the picture's units; None in a window too small to
    /// show it.
    pub fn in_picture(&self, pos: (f64, f64)) -> Option<(f64, f64)> {
        let (x, y, w, h) = self.rect;
        if w == 0 || h == 0 {
            return None;
        }
        Some((
            (pos.0 - x as f64) * self.picture.0 as f64 / w as f64,
            (pos.1 - y as f64) * self.picture.1 as f64 / h as f64,
        ))
    }

    /// The window position `pos` in the screen's own units ([`Framing::in_picture`], less where
    /// the screen stands on the picture).
    pub fn in_screen(&self, pos: (f64, f64)) -> Option<(f64, f64)> {
        let (px, py) = self.in_picture(pos)?;
        Some((px - self.screen_at.0 as f64, py - self.screen_at.1 as f64))
    }

    /// The window rectangle (x, y, width, height, in pixels) of the picture's area `a` (in its
    /// units: the window's buttons).
    #[cfg(test)]
    pub fn area_in_window(&self, a: (i32, i32, u32, u32)) -> (f64, f64, f64, f64) {
        let (x, y, w, h) = self.rect;
        let (sx, sy) = (
            w as f64 / self.picture.0 as f64,
            h as f64 / self.picture.1 as f64,
        );
        (
            x as f64 + a.0 as f64 * sx,
            y as f64 + a.1 as f64 * sy,
            a.2 as f64 * sx,
            a.3 as f64 * sy,
        )
    }

    /// The window rectangle of the screen's own area `a` (in its units: its rows).
    #[cfg(test)]
    pub fn screen_area_in_window(&self, a: (i32, i32, u32, u32)) -> (f64, f64, f64, f64) {
        self.area_in_window((a.0 + self.screen_at.0, a.1 + self.screen_at.1, a.2, a.3))
    }
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

    /// On Windows Direct3D 12's card, then its software adapter (WARP), and only then Vulkan's, so
    /// Vulkan is never loaded into the launcher unless both Direct3D 12 entries fail; elsewhere the
    /// platform's own card, then its software adapter.
    #[test]
    fn direct3d_12_and_warp_before_any_vulkan() {
        use wgpu::Backends as B;
        if cfg!(windows) {
            assert_eq!(
                tries(),
                [
                    (B::DX12, false),
                    (B::DX12, true),
                    (B::VULKAN, false),
                    (B::VULKAN, true)
                ]
            );
        } else {
            assert_eq!(tries(), [(B::PRIMARY, false), (B::PRIMARY, true)]);
        }
        assert_eq!(api_name(wgpu::Backend::Dx12), "Direct3D 12");
        assert_eq!(api_name(wgpu::Backend::Vulkan), "Vulkan");
        assert_eq!(api_name(wgpu::Backend::Metal), "Metal");
    }

    /// The first entry that works is taken: an API that cannot be opened, a card that cannot make
    /// its device or its surface, are passed over (each said in the failures), down to the last
    /// software adapter; with nothing working, every failure in order.
    #[test]
    fn a_failing_card_passes_on_to_the_next_down_to_software() {
        use wgpu::Backends as B;
        let order = [
            (B::DX12, false),
            (B::DX12, true),
            (B::VULKAN, false),
            (B::VULKAN, true),
        ];
        type Listed = Result<((), Vec<(&'static str, wgpu::DeviceType)>), String>;
        let lists = |b: B| -> Listed {
            match b {
                B::DX12 => Ok(((), vec![("dx-card", DiscreteGpu), ("warp", Cpu)])),
                B::VULKAN => Ok(((), vec![("vk-card", DiscreteGpu), ("vk-cpu", Cpu)])),
                _ => Err("no such API".into()),
            }
        };
        let works = |good: &'static [&'static str]| {
            move |_: &(), a: &'static str| {
                if good.contains(&a) {
                    Ok(a)
                } else {
                    Err(format!("{a} failed"))
                }
            }
        };
        // Direct3D 12's card refuses its surface: WARP draws, Vulkan is never tried.
        let mut tried = Vec::new();
        let r = first_working(&order, lists, |c: &(), a: &'static str| {
            tried.push(a);
            works(&["warp", "vk-card"])(c, a)
        });
        assert_eq!(r.map(|(_, t)| t), Ok("warp"));
        assert_eq!(tried, ["dx-card", "warp"]);
        // Only the last software adapter works.
        let r = first_working(&order, lists, works(&["vk-cpu"]));
        assert_eq!(r.map(|(_, t)| t), Ok("vk-cpu"));
        // An API that cannot be opened is passed over.
        let r = first_working(
            &[(B::METAL, false), (B::VULKAN, false)],
            lists,
            works(&["vk-card"]),
        );
        assert_eq!(r.map(|(_, t)| t), Ok("vk-card"));
        // Two cards in one API: the built-in one fails, the separate one is taken before any
        // software adapter.
        let two = |_: B| -> Listed {
            Ok((
                (),
                vec![
                    ("built-in", IntegratedGpu),
                    ("separate", DiscreteGpu),
                    ("warp", Cpu),
                ],
            ))
        };
        let mut tried = Vec::new();
        let r = first_working(&order, two, |c: &(), a: &'static str| {
            tried.push(a);
            works(&["separate", "warp"])(c, a)
        });
        assert_eq!(r.map(|(_, t)| t), Ok("separate"));
        assert_eq!(tried, ["built-in", "separate"]);
        // Nothing works: every failure, in order.
        let r = first_working(&order, lists, works(&[]));
        assert_eq!(
            r.map(|(_, t)| t),
            Err(vec![
                "dx-card failed".to_string(),
                "warp failed".into(),
                "vk-card failed".into(),
                "vk-cpu failed".into()
            ])
        );
    }

    /// A surface set-up that a scope caught, or during which the device was lost, passes the card
    /// over; only a clean one keeps it. A driver's loss counts, the launcher's own `destroy` not.
    #[test]
    fn a_device_lost_while_the_surface_is_set_up_passes_the_card_over() {
        assert_eq!(surface_result(None, false), Ok(()));
        assert!(surface_result(None, true)
            .unwrap_err()
            .contains("the device was lost"));
        assert!(surface_result(Some("refused".into()), false)
            .unwrap_err()
            .contains("refused"));
        assert!(counts_as_lost(wgpu::DeviceLostReason::Unknown));
        assert!(!counts_as_lost(wgpu::DeviceLostReason::Destroyed));
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

    /// In a 4:3 window the picture is the screen itself, as before; in a wider or taller one it
    /// is the window's shape, the screen whole in its middle at the same scale as `fit`'s, so
    /// the picture fills the window (to a pixel) and the screen goes where `fit` puts it. The
    /// device's largest texture caps the picture, centred then.
    #[test]
    fn the_picture_takes_the_windows_shape_around_the_screen() {
        let screen = (960, 720);
        let f = framing(screen, (960, 720), 8192);
        assert_eq!(f.picture, screen);
        assert_eq!((f.screen_at, f.rect), ((0, 0), (0, 0, 960, 720)));
        assert_eq!(framing(screen, (1920, 1440), 8192).picture, screen);
        for (window, picture, at) in [
            ((3440, 1440), (1720, 720), (380, 0)),
            ((5120, 1440), (2560, 720), (800, 0)),
            ((2560, 1080), (1706, 720), (373, 0)),
            ((1280, 720), (1280, 720), (160, 0)),
            ((1080, 1920), (960, 1706), (0, 493)),
            ((2000, 1440), (1000, 720), (20, 0)),
        ] {
            let f = framing(screen, window, 8192);
            assert_eq!((f.picture, f.screen_at), (picture, at), "{window:?}");
            let (x, y, w, h) = f.rect;
            assert!(x <= 1 && y <= 1, "{window:?} {:?}", f.rect);
            assert!(
                window.0 - w <= 2 && window.1 - h <= 2,
                "{window:?} {:?}",
                f.rect
            );
            // The screen where `fit` puts it, to a pixel.
            let (fx, fy, fw, fh) = fit(screen, window);
            let (sx, sy, sw, sh) = f.screen_area_in_window((0, 0, screen.0, screen.1));
            assert!((sx - fx as f64).abs() <= 1.0 && (sy - fy as f64).abs() <= 1.0);
            assert!((sw - fw as f64).abs() <= 1.0 && (sh - fh as f64).abs() <= 1.0);
            // Window positions come back to the screen's units.
            let (px, py) = f.in_screen((sx + sw / 2.0, sy + sh / 2.0)).unwrap();
            assert!((px - 480.0).abs() < 1.0 && (py - 360.0).abs() < 1.0);
        }
        let capped = framing(screen, (5120, 1440), 2048);
        assert_eq!((capped.picture, capped.screen_at), ((2048, 720), (544, 0)));
        assert_eq!(capped.rect, (512, 0, 4096, 1440));
        assert_eq!(framing(screen, (0, 0), 8192).rect, (0, 0, 0, 0));
        assert_eq!(framing(screen, (0, 0), 8192).in_picture((1.0, 1.0)), None);
    }
}
