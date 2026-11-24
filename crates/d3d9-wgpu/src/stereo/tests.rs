use super::*;

#[test]
fn mode_names() {
    assert_eq!(Mode::parse("anaglyph"), Ok(Some(Mode::Anaglyph)));
    assert_eq!(Mode::parse(" CrossEyed "), Ok(Some(Mode::CrossEyed)));
    for off in ["", "0", "off"] {
        assert_eq!(Mode::parse(off), Ok(None));
    }
    assert!(Mode::parse("crosseyed|anaglyph").is_err());
}

#[test]
fn shader_validates_without_an_adapter() {
    let module = naga::front::wgsl::parse_str(SHADER)
        .unwrap_or_else(|e| panic!("{}", e.emit_to_string(SHADER)));
    naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::empty(),
    )
    .validate(&module)
    .unwrap();
}

#[test]
fn stereo_uses_world_depth_and_composes_the_expected_eyes() {
    use crate::device::block_on;
    use wgpu::{TextureFormat as Format, TextureUsages as Usage};

    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::PRIMARY,
        ..wgpu::InstanceDescriptor::new_without_display_handle()
    });
    let Ok(adapter) = block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default()))
    else {
        eprintln!("no GPU adapter: skipping stereo rendering test");
        return;
    };
    let (device, queue) =
        block_on(adapter.request_device(&wgpu::DeviceDescriptor::default())).unwrap();
    let size = wgpu::Extent3d {
        width: 256,
        height: 16,
        depth_or_array_layers: 1,
    };
    let texture = |format, usage, sample_count| {
        device.create_texture(&wgpu::TextureDescriptor {
            label: Some("stereo test"),
            size,
            mip_level_count: 1,
            sample_count,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage,
            view_formats: &[],
        })
    };
    let colour = texture(
        Format::Rgba8Unorm,
        Usage::TEXTURE_BINDING | Usage::COPY_DST,
        1,
    );
    let colour_view = colour.create_view(&Default::default());
    let output = texture(
        Format::Rgba8Unorm,
        Usage::RENDER_ATTACHMENT | Usage::COPY_SRC,
        1,
    );
    let output_view = output.create_view(&Default::default());
    let row = size.width * 4;
    let layout = wgpu::TexelCopyBufferLayout {
        offset: 0,
        bytes_per_row: Some(row),
        rows_per_image: None,
    };
    let gradient: Vec<u8> = (0..size.height)
        .flat_map(|_| (0..=255).flat_map(|v| [v, v, v, 255]))
        .collect();
    queue.write_texture(colour.as_image_copy(), &gradient, layout, size);
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: u64::from(size.height * row),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let clear = |encoder: &mut wgpu::CommandEncoder, view: &wgpu::TextureView, depth| {
        drop(encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(depth),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            ..Default::default()
        }));
    };
    let params = Params {
        near: 10.0,
        far: 1000.0,
        left: -0.75,
        right: 0.75,
        bottom: -0.5,
        top: 0.5,
        sun: [0.0; 3],
        shadow: 0.0,
        strength: 0.0,
    };
    let mut stereo = Gpu::new(&device, Mode::Anaglyph);
    let mut render = |mode, samples, depth, current, bright: bool| {
        stereo.mode = mode;
        let ds = texture(
            Format::Depth32Float,
            Usage::RENDER_ATTACHMENT | Usage::TEXTURE_BINDING,
            samples,
        );
        let view = ds.create_view(&Default::default());
        let mut encoder = device.create_command_encoder(&Default::default());
        let dims = (size.width, size.height);
        clear(&mut encoder, &view, depth);
        stereo.capture(&device, &mut encoder, 0, (&view, samples, dims), params);
        // The game clears depth to draw the first-person arms. The eye views still use the world.
        clear(&mut encoder, &view, 1.0);
        let ramp = bright.then(|| crate::ramp::buffer(&device, &crate::ramp::games_ramp(0.6)));
        if !stereo.compose(
            &device,
            &mut encoder,
            current,
            (&colour_view, dims),
            (&output_view, Format::Rgba8Unorm),
            ramp.as_ref(),
        ) {
            return None;
        }
        encoder.copy_texture_to_buffer(
            output.as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &readback,
                layout,
            },
            size,
        );
        queue.submit([encoder.finish()]);
        let slice = readback.slice(..);
        slice.map_async(wgpu::MapMode::Read, |r| r.unwrap());
        device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
        let pixels = slice.get_mapped_range().unwrap().to_vec();
        readback.unmap();
        Some(pixels)
    };
    let pixel = |p: &[u8], x: u32, y: u32| -> [u8; 4] {
        let i = (y * row + x * 4) as usize;
        p[i..i + 4].try_into().unwrap()
    };
    for samples in [1, 4] {
        if !adapter
            .get_texture_format_features(Format::Depth32Float)
            .flags
            .sample_count_supported(samples)
        {
            continue;
        }
        let near = render(Mode::Anaglyph, samples, 0.0, 0, false).unwrap();
        let [r, g, b, _] = pixel(&near, 128, 8);
        assert!(
            r.abs_diff(125) <= 1 && g.abs_diff(131) <= 1 && g == b,
            "{r} {g} {b}"
        );
        let far = render(Mode::Anaglyph, samples, 1.0, 0, false).unwrap();
        let [r, g, b, _] = pixel(&far, 128, 8);
        assert!(r > 128 && g < 128 && g == b, "{r} {g} {b}");
    }
    let crossed = render(Mode::CrossEyed, 1, 0.0, 0, false).unwrap();
    assert!(pixel(&crossed, 64, 8)[0] > pixel(&crossed, 192, 8)[0] + 4);
    assert_eq!(pixel(&crossed, 64, 1), [0, 0, 0, 255]);
    assert_eq!(pixel(&crossed, 192, 14), [0, 0, 0, 255]);
    let flat = render(Mode::CrossEyed, 1, 0.0, 1, false).unwrap();
    assert_eq!(pixel(&flat, 64, 8), pixel(&flat, 192, 8));
    assert!(render(Mode::Anaglyph, 1, 0.0, 1, false).is_none());
    let bright = render(Mode::Anaglyph, 1, 0.0, 0, true).unwrap();
    assert!(pixel(&bright, 128, 8)[0] > 150);
}
