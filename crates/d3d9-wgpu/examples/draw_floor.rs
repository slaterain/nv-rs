//! A measuring tool: `draw_floor DRAWS PIPES PIPE_SWITCHES TEXSETS
//! TEX_SWITCHES BUFFERS BUFFER_SWITCHES [FRAMES]`. The CPU floor of a native wgpu draw path for a frame shaped like
//! the street's world draw: DRAWS small indexed draws a frame, each with its 256 bytes of constants in ONE uniform
//! buffer written once a frame (a dynamic offset per draw), PIPES prebuilt pipelines switched PIPE_SWITCHES times,
//! TEXSETS prebuilt texture bind groups switched TEX_SWITCHES times, BUFFERS vertex/index buffers switched
//! BUFFER_SWITCHES times. Nothing is made per draw. Two ways a frame: (1) recorded into a render pass each frame,
//! (2) one render bundle recorded once and executed each frame. Times the CPU from the encoder to the submit's
//! return (the GPU's wait after it is left out), median of FRAMES (default 400) after 50 warm-up frames.
use std::time::Instant;

const SHADER: &str = r#"
struct U { m: mat4x4<f32>, c: vec4<f32>, }
@group(0) @binding(0) var<uniform> u: U;
@group(1) @binding(0) var t: texture_2d<f32>;
@group(1) @binding(1) var s: sampler;
struct O { @builtin(position) p: vec4<f32>, @location(0) uv: vec2<f32>, }
@vertex fn vs(@location(0) p: vec3<f32>) -> O { var o: O; o.p = u.m * vec4<f32>(p, 1.0); o.uv = p.xy; return o; }
@fragment fn fs(i: O) -> @location(0) vec4<f32> { return textureSample(t, s, i.uv) * u.c; }
"#;

fn block_on<F: std::future::Future>(f: F) -> F::Output {
    struct Noop;
    impl std::task::Wake for Noop {
        fn wake(self: std::sync::Arc<Self>) {}
    }
    let waker = std::task::Waker::from(std::sync::Arc::new(Noop));
    let mut cx = std::task::Context::from_waker(&waker);
    let mut f = std::pin::pin!(f);
    loop {
        if let std::task::Poll::Ready(v) = f.as_mut().poll(&mut cx) {
            return v;
        }
        std::thread::yield_now();
    }
}

fn main() {
    let a: Vec<usize> = std::env::args()
        .skip(1)
        .map(|s| s.parse().expect("a number"))
        .collect();
    let [draws, pipes, pipe_sw, texsets, tex_sw, buffers, buf_sw] = a[..7] else {
        panic!("seven numbers")
    };
    let frames = a.get(7).copied().unwrap_or(400);
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::VULKAN,
        ..wgpu::InstanceDescriptor::new_without_display_handle()
    });
    let adapter = block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::HighPerformance,
        force_fallback_adapter: false,
        compatible_surface: None,
        apply_limit_buckets: false,
    }))
    .expect("a Vulkan adapter");
    eprintln!("adapter: {}", adapter.get_info().name);
    let (device, queue) = block_on(adapter.request_device(&wgpu::DeviceDescriptor {
        label: Some("draw floor"),
        required_features: wgpu::Features::empty(),
        required_limits: adapter.limits(),
        memory_hints: wgpu::MemoryHints::Performance,
        experimental_features: Default::default(),
        trace: Default::default(),
    }))
    .expect("a device");
    let format = wgpu::TextureFormat::Rgba8Unorm;
    let depth_format = wgpu::TextureFormat::Depth32Float;
    let target = device.create_texture(&wgpu::TextureDescriptor {
        label: None,
        size: wgpu::Extent3d {
            width: 256,
            height: 256,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    let depth = device.create_texture(&wgpu::TextureDescriptor {
        label: None,
        size: wgpu::Extent3d {
            width: 256,
            height: 256,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: depth_format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    let (tview, dview) = (
        target.create_view(&Default::default()),
        depth.create_view(&Default::default()),
    );
    use wgpu::ShaderStages as S;
    let ulayout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: None,
        entries: &[wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: S::VERTEX_FRAGMENT,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: true,
                min_binding_size: std::num::NonZeroU64::new(80),
            },
            count: None,
        }],
    });
    let tlayout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: None,
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
        ],
    });
    let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: None,
        source: wgpu::ShaderSource::Wgsl(SHADER.into()),
    });
    let pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: None,
        bind_group_layouts: &[Some(&ulayout), Some(&tlayout)],
        immediate_size: 0,
    });
    let compares = [
        wgpu::CompareFunction::LessEqual,
        wgpu::CompareFunction::Less,
        wgpu::CompareFunction::Always,
        wgpu::CompareFunction::GreaterEqual,
    ];
    let pipelines: Vec<wgpu::RenderPipeline> = (0..pipes)
        .map(|i| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: None,
                layout: Some(&pl),
                vertex: wgpu::VertexState {
                    module: &module,
                    entry_point: Some("vs"),
                    compilation_options: Default::default(),
                    buffers: &[Some(wgpu::VertexBufferLayout {
                        array_stride: 12,
                        step_mode: wgpu::VertexStepMode::Vertex,
                        attributes: &[wgpu::VertexAttribute {
                            format: wgpu::VertexFormat::Float32x3,
                            offset: 0,
                            shader_location: 0,
                        }],
                    })],
                },
                primitive: wgpu::PrimitiveState {
                    cull_mode: [None, Some(wgpu::Face::Back), Some(wgpu::Face::Front)][i % 3],
                    ..Default::default()
                },
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: depth_format,
                    depth_write_enabled: Some(i % 2 == 0),
                    depth_compare: Some(compares[(i / 6) % 4]),
                    stencil: Default::default(),
                    bias: wgpu::DepthBiasState {
                        constant: (i / 24) as i32,
                        ..Default::default()
                    },
                }),
                multisample: Default::default(),
                fragment: Some(wgpu::FragmentState {
                    module: &module,
                    entry_point: Some("fs"),
                    compilation_options: Default::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format,
                        blend: if i % 2 == 1 {
                            Some(wgpu::BlendState::ALPHA_BLENDING)
                        } else {
                            None
                        },
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                multiview_mask: None,
                cache: None,
            })
        })
        .collect();
    let sampler = device.create_sampler(&wgpu::SamplerDescriptor::default());
    let groups: Vec<wgpu::BindGroup> = (0..texsets)
        .map(|_| {
            let t = device.create_texture(&wgpu::TextureDescriptor {
                label: None,
                size: wgpu::Extent3d {
                    width: 4,
                    height: 4,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format,
                usage: wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            });
            let v = t.create_view(&Default::default());
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: None,
                layout: &tlayout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&v),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&sampler),
                    },
                ],
            })
        })
        .collect();
    let mk = |usage, size| {
        device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size,
            usage,
            mapped_at_creation: false,
        })
    };
    let vbs: Vec<wgpu::Buffer> = (0..buffers)
        .map(|_| mk(wgpu::BufferUsages::VERTEX, 48))
        .collect();
    let ibs: Vec<wgpu::Buffer> = (0..buffers)
        .map(|_| mk(wgpu::BufferUsages::INDEX, 12))
        .collect();
    let ubuf = mk(
        wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        (draws * 256) as u64,
    );
    let ugroup = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        layout: &ulayout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                buffer: &ubuf,
                offset: 0,
                size: std::num::NonZeroU64::new(80),
            }),
        }],
    });
    let consts = vec![0u8; draws * 256];
    // Which pipeline, texture set and buffer each draw takes: each switched evenly over the frame.
    let at = |i: usize, sw: usize, n: usize| {
        if sw == 0 {
            0
        } else {
            (i * sw / draws) % n.max(1)
        }
    };
    let plan: Vec<(usize, usize, usize)> = (0..draws)
        .map(|i| {
            (
                at(i, pipe_sw, pipes),
                at(i, tex_sw, texsets),
                at(i, buf_sw, buffers),
            )
        })
        .collect();
    // The frame's draws into a pass or a bundle; only what changes is set.
    macro_rules! record {
        ($p:expr) => {{
            let p = &mut $p;
            let mut last = (usize::MAX, usize::MAX, usize::MAX);
            for (i, &(pi, ti, bi)) in plan.iter().enumerate() {
                if pi != last.0 {
                    p.set_pipeline(&pipelines[pi]);
                }
                if ti != last.1 {
                    p.set_bind_group(1, &groups[ti], &[]);
                }
                p.set_bind_group(0, &ugroup, &[(i * 256) as u32]);
                if bi != last.2 {
                    p.set_vertex_buffer(0, vbs[bi].slice(..));
                    p.set_index_buffer(ibs[bi].slice(..), wgpu::IndexFormat::Uint16);
                }
                p.draw_indexed(0..6, 0, 0..1);
                last = (pi, ti, bi);
            }
        }};
    }
    let bundle = {
        let mut e = device.create_render_bundle_encoder(&wgpu::RenderBundleEncoderDescriptor {
            label: None,
            color_formats: &[Some(format)],
            depth_stencil: Some(wgpu::RenderBundleDepthStencil {
                format: depth_format,
                depth_read_only: false,
                stencil_read_only: true,
            }),
            sample_count: 1,
            multiview: None,
        });
        record!(e);
        e.finish(&wgpu::RenderBundleDescriptor { label: None })
    };
    for mode in ["pass", "bundle"] {
        let mut times = Vec::new();
        for f in 0..frames + 50 {
            let s = Instant::now();
            queue.write_buffer(&ubuf, 0, &consts);
            let mut enc = device.create_command_encoder(&Default::default());
            {
                let mut pass = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: None,
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &tview,
                        depth_slice: None,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                        view: &dview,
                        depth_ops: Some(wgpu::Operations {
                            load: wgpu::LoadOp::Clear(1.0),
                            store: wgpu::StoreOp::Store,
                        }),
                        stencil_ops: None,
                    }),
                    timestamp_writes: None,
                    occlusion_query_set: None,
                    multiview_mask: None,
                });
                if mode == "pass" {
                    record!(pass);
                } else {
                    pass.execute_bundles(std::iter::once(&bundle));
                }
            }
            queue.submit(std::iter::once(enc.finish()));
            let t = s.elapsed().as_secs_f64() * 1e3;
            let _ = device.poll(wgpu::PollType::wait_indefinitely());
            if f >= 50 {
                times.push(t);
            }
        }
        times.sort_by(|x, y| x.partial_cmp(y).unwrap());
        println!(
            "{mode:6}: {draws} draws, {pipes} pipelines ({pipe_sw} switches), {texsets} texture sets ({tex_sw}), {buffers} buffers ({buf_sw}): CPU {:.3} ms a frame median, {:.3} at 90%, {:.2} us a draw",
            times[times.len() / 2],
            times[times.len() * 9 / 10],
            times[times.len() / 2] * 1e3 / draws as f64
        );
    }
}
