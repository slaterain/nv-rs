//! The screen the backend shows while it does a long step of its own before play (the first
//! start's pipelines, [`crate::prepare`]): the nv-rs logo, the word "Loading" and one line that
//! names the step and its progress, centred on black, scaled to the window. Whoever does the long
//! step gives it a [`Progress`]; the same numbers go to the log, for a launcher to read.
//!
//! The logo is the 128 by 128 image in the port's icon (`viewer/nv-rs.ico`: an uncompressed
//! 32-bit picture, so no image decoder is needed), drawn at a whole multiple of its size (no
//! blur). The text is drawn by Windows' GDI into a bitmap at the size it is shown (elsewhere the
//! screen shows the logo and the bar only). Only a backend that owns the game's window draws it:
//! offscreen backends (the shadow of the real one, replays, tests) never do.

/// What a long step says about itself.
#[derive(Debug, Clone, PartialEq)]
pub struct Progress {
    /// The big line ("Loading").
    pub title: String,
    /// The step and its count ("Preparing shaders 212 of 1,480").
    pub status: String,
    /// How much is done, 0 to 1.
    pub done: f32,
}

/// The 128 by 128 logo from the icon: rows top first, B, G, R, A bytes.
fn logo() -> Option<(u32, u32, Vec<u8>)> {
    const ICO: &[u8] = include_bytes!("../../../viewer/nv-rs.ico");
    let count = u16::from_le_bytes([*ICO.get(4)?, *ICO.get(5)?]) as usize;
    for i in 0..count {
        let e = ICO.get(6 + 16 * i..6 + 16 * i + 16)?;
        let size = u32::from_le_bytes(e[8..12].try_into().ok()?) as usize;
        let at = u32::from_le_bytes(e[12..16].try_into().ok()?) as usize;
        let img = ICO.get(at..at + size)?;
        // A BITMAPINFOHEADER image of 128 by 128 (its height counts the mask too), 32 bits.
        let word = |o: usize| u32::from_le_bytes(img[o..o + 4].try_into().expect("4 bytes"));
        if img.len() < 40 || word(0) != 40 || word(4) != 128 || word(8) != 256 {
            continue;
        }
        if u16::from_le_bytes([img[14], img[15]]) != 32 {
            continue;
        }
        let pixels = img.get(40..40 + 128 * 128 * 4)?;
        // Bottom row first in the file.
        let mut out = Vec::with_capacity(pixels.len());
        for row in (0..128).rev() {
            out.extend_from_slice(&pixels[row * 512..row * 512 + 512]);
        }
        return Some((128, 128, out));
    }
    None
}

/// `text` drawn white on transparent at `height` pixels, B, G, R, A rows top first (Windows'
/// GDI); None elsewhere or when GDI fails.
#[cfg(windows)]
fn text_image(text: &str, height: u32) -> Option<(u32, u32, Vec<u8>)> {
    use std::ffi::c_void;
    #[repr(C)]
    struct BitmapInfoHeader {
        size: u32,
        width: i32,
        height: i32,
        planes: u16,
        bit_count: u16,
        compression: u32,
        size_image: u32,
        x_ppm: i32,
        y_ppm: i32,
        clr_used: u32,
        clr_important: u32,
    }
    #[repr(C)]
    struct Size {
        cx: i32,
        cy: i32,
    }
    #[link(name = "gdi32")]
    extern "system" {
        fn CreateCompatibleDC(dc: isize) -> isize;
        fn DeleteDC(dc: isize) -> i32;
        fn CreateDIBSection(
            dc: isize,
            info: *const BitmapInfoHeader,
            usage: u32,
            bits: *mut *mut c_void,
            section: isize,
            offset: u32,
        ) -> isize;
        fn SelectObject(dc: isize, object: isize) -> isize;
        fn DeleteObject(object: isize) -> i32;
        fn CreateFontW(
            height: i32,
            width: i32,
            escapement: i32,
            orientation: i32,
            weight: i32,
            italic: u32,
            underline: u32,
            strike: u32,
            charset: u32,
            precision: u32,
            clip: u32,
            quality: u32,
            pitch: u32,
            face: *const u16,
        ) -> isize;
        fn SetTextColor(dc: isize, colour: u32) -> u32;
        fn SetBkMode(dc: isize, mode: i32) -> i32;
        fn TextOutW(dc: isize, x: i32, y: i32, text: *const u16, len: i32) -> i32;
        fn GetTextExtentPoint32W(dc: isize, text: *const u16, len: i32, size: *mut Size) -> i32;
    }
    let wide: Vec<u16> = text.encode_utf16().collect();
    let face: Vec<u16> = "Segoe UI\0".encode_utf16().collect();
    // SAFETY: plain GDI calls on objects made here and deleted before returning; the DIB's bits
    // are read while the bitmap is alive, within its size.
    unsafe {
        let dc = CreateCompatibleDC(0);
        if dc == 0 {
            return None;
        }
        // FW_NORMAL, DEFAULT_CHARSET, ANTIALIASED_QUALITY (grey levels: the alpha below).
        let font = CreateFontW(
            -(height as i32),
            0,
            0,
            0,
            400,
            0,
            0,
            0,
            1,
            0,
            0,
            4,
            0,
            face.as_ptr(),
        );
        let old_font = SelectObject(dc, font);
        let mut size = Size { cx: 0, cy: 0 };
        GetTextExtentPoint32W(dc, wide.as_ptr(), wide.len() as i32, &mut size);
        let (w, h) = (size.cx.max(1) as u32, size.cy.max(1) as u32);
        let info = BitmapInfoHeader {
            size: 40,
            width: w as i32,
            height: -(h as i32),
            planes: 1,
            bit_count: 32,
            compression: 0,
            size_image: 0,
            x_ppm: 0,
            y_ppm: 0,
            clr_used: 0,
            clr_important: 0,
        };
        let mut bits: *mut c_void = std::ptr::null_mut();
        let bitmap = CreateDIBSection(dc, &info, 0, &mut bits, 0, 0);
        let mut out = None;
        if bitmap != 0 && !bits.is_null() {
            let old_bitmap = SelectObject(dc, bitmap);
            SetBkMode(dc, 1);
            SetTextColor(dc, 0x00FF_FFFF);
            TextOutW(dc, 0, 0, wide.as_ptr(), wide.len() as i32);
            let px = std::slice::from_raw_parts(bits as *const u8, (w * h * 4) as usize);
            // GDI writes the colour, not an alpha: the grey level is the coverage.
            let mut v = Vec::with_capacity(px.len());
            for p in px.chunks_exact(4) {
                let a = p[0].max(p[1]).max(p[2]);
                v.extend_from_slice(&[255, 255, 255, a]);
            }
            out = Some((w, h, v));
            SelectObject(dc, old_bitmap);
            DeleteObject(bitmap);
        }
        SelectObject(dc, old_font);
        DeleteObject(font);
        DeleteDC(dc);
        out
    }
}

#[cfg(not(windows))]
fn text_image(_text: &str, _height: u32) -> Option<(u32, u32, Vec<u8>)> {
    None
}

const SHADER: &str = "
struct Q { rect: vec4<f32>, colour: vec4<f32> }
@group(0) @binding(0) var t: texture_2d<f32>;
@group(0) @binding(1) var s: sampler;
@group(0) @binding(2) var<uniform> q: Q;
struct V { @builtin(position) p: vec4<f32>, @location(0) uv: vec2<f32> }
@vertex fn vs(@builtin(vertex_index) i: u32) -> V {
    let c = vec2<f32>(f32(i & 1u), f32((i >> 1u) & 1u));
    var v: V;
    v.p = vec4<f32>(q.rect.xy + c * q.rect.zw, 0.0, 1.0);
    v.uv = vec2<f32>(c.x, 1.0 - c.y);
    return v;
}
@fragment fn fs(v: V) -> @location(0) vec4<f32> {
    return textureSample(t, s, v.uv) * q.colour;
}
";

/// The drawing of the screen on one device and window format.
pub(crate) struct Screen {
    pipeline: wgpu::RenderPipeline,
    /// The window format the pipeline is made for.
    pub(crate) format: wgpu::TextureFormat,
    layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    logo: Option<(u32, u32, wgpu::TextureView)>,
    white: wgpu::TextureView,
    /// The last text drawn and its texture, by line.
    text: Vec<(String, u32, (u32, u32, wgpu::TextureView))>,
}

fn texture(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    (w, h, px): (u32, u32, &[u8]),
) -> wgpu::TextureView {
    let size = wgpu::Extent3d {
        width: w,
        height: h,
        depth_or_array_layers: 1,
    };
    let t = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("loading screen"),
        size,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Bgra8Unorm,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    queue.write_texture(
        t.as_image_copy(),
        px,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(w * 4),
            rows_per_image: Some(h),
        },
        size,
    );
    t.create_view(&Default::default())
}

impl Screen {
    pub(crate) fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        format: wgpu::TextureFormat,
    ) -> Screen {
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("loading screen"),
            source: wgpu::ShaderSource::Wgsl(SHADER.into()),
        });
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("loading screen"),
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
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
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
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("loading screen"),
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
        // Nearest: the logo is drawn at a whole multiple of its size, the text at its own.
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor::default());
        let logo = logo().map(|(w, h, px)| (w, h, texture(device, queue, (w, h, &px))));
        let white = texture(device, queue, (1, 1, &[255, 255, 255, 255]));
        Screen {
            pipeline,
            format,
            layout,
            sampler,
            logo,
            white,
            text: Vec::new(),
        }
    }

    /// The text line `i` as a texture at `height` pixels, made again only when it changed.
    fn line(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        i: usize,
        text: &str,
        height: u32,
    ) -> Option<(u32, u32, wgpu::TextureView)> {
        if let Some((t, h, v)) = self.text.get(i) {
            if t == text && *h == height {
                return Some(v.clone());
            }
        }
        let (w, h, px) = text_image(text, height)?;
        let v = (w, h, texture(device, queue, (w, h, &px)));
        let e = (text.to_string(), height, v.clone());
        if i < self.text.len() {
            self.text[i] = e;
        } else {
            self.text.push(e);
        }
        Some(v)
    }

    /// Draws `p` over all of `target` (`width` by `height` pixels), cleared to black.
    pub(crate) fn draw(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        target: &wgpu::TextureView,
        (width, height): (u32, u32),
        p: &Progress,
    ) {
        let (fw, fh) = (width.max(1) as f32, height.max(1) as f32);
        // The logo at the largest whole multiple of its size that fits a third of the height.
        let scale = ((height / 3) / 128).max(1);
        let title_px = (height / 18).max(12);
        let status_px = (height / 40).max(10);
        let title = self.line(device, queue, 0, &p.title, title_px);
        let status = self.line(device, queue, 1, &p.status, status_px);
        // Quads in pixels (x, y, w, h), laid out top to bottom, centred.
        let mut quads: Vec<(wgpu::TextureView, [f32; 4], [f32; 4])> = Vec::new();
        let logo_h = self.logo.as_ref().map_or(0, |(_, h, _)| h * scale);
        let bar_w = (width / 3).max(64);
        let bar_h = (height / 200).max(3);
        let gap = height / 40;
        let total = logo_h
            + gap
            + title.as_ref().map_or(0, |t| t.1)
            + gap
            + status.as_ref().map_or(0, |t| t.1)
            + gap
            + bar_h;
        let mut y = height.saturating_sub(total) / 2;
        let white = [1.0, 1.0, 1.0, 1.0];
        if let Some((w, h, v)) = &self.logo {
            let (w, h) = (w * scale, h * scale);
            quads.push((
                v.clone(),
                [
                    (width.saturating_sub(w) / 2) as f32,
                    y as f32,
                    w as f32,
                    h as f32,
                ],
                white,
            ));
            y += h + gap;
        }
        for t in [title, status].into_iter().flatten() {
            let (w, h, v) = t;
            quads.push((
                v,
                [
                    (width.saturating_sub(w) / 2) as f32,
                    y as f32,
                    w as f32,
                    h as f32,
                ],
                white,
            ));
            y += h + gap;
        }
        let x = (width.saturating_sub(bar_w) / 2) as f32;
        quads.push((
            self.white.clone(),
            [x, y as f32, bar_w as f32, bar_h as f32],
            [0.25, 0.25, 0.25, 1.0],
        ));
        quads.push((
            self.white.clone(),
            [
                x,
                y as f32,
                bar_w as f32 * p.done.clamp(0.0, 1.0),
                bar_h as f32,
            ],
            [0.9, 0.75, 0.35, 1.0],
        ));
        let mut groups = Vec::new();
        for (view, [x, y, w, h], colour) in &quads {
            // Pixels to clip space, rounded to whole pixels.
            let rect = [
                x.round() / fw * 2.0 - 1.0,
                1.0 - (y.round() + h) / fh * 2.0,
                w / fw * 2.0,
                h / fh * 2.0,
            ];
            let words: Vec<u8> = rect
                .iter()
                .chain(colour.iter())
                .flat_map(|v| v.to_le_bytes())
                .collect();
            let ub = device.create_buffer(&wgpu::BufferDescriptor {
                label: None,
                size: 32,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            queue.write_buffer(&ub, 0, &words);
            groups.push(device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: None,
                layout: &self.layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&self.sampler),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: ub.as_entire_binding(),
                    },
                ],
            }));
        }
        let mut encoder = device.create_command_encoder(&Default::default());
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("loading screen"),
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
            pass.set_pipeline(&self.pipeline);
            for g in &groups {
                pass.set_bind_group(0, g, &[]);
                pass.draw(0..4, 0..1);
            }
        }
        queue.submit([encoder.finish()]);
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_icon_holds_the_128_pixel_logo() {
        let (w, h, px) = super::logo().expect("the 128 by 128 image");
        assert_eq!((w, h, px.len()), (128, 128, 128 * 128 * 4));
        // Not all transparent: the picture is there.
        assert!(px.chunks_exact(4).filter(|p| p[3] > 0).count() > 1000);
    }

    #[cfg(windows)]
    #[test]
    fn gdi_draws_the_status_line() {
        let (w, h, px) = super::text_image("Preparing shaders 212 of 1,480", 24).expect("GDI");
        assert!(w > 100 && h >= 20, "{w}x{h}");
        assert!(px.chunks_exact(4).any(|p| p[3] == 255));
    }
}
