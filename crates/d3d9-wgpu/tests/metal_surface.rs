//! macOS: the drawing surface on a `CAMetalLayer`, used from a thread that is not the main one.
//!
//! On macOS the window layer makes the window's Metal layer on the main thread (AppKit's view is
//! the main thread's only), and the game's device makes its surface on that layer from the game's
//! or the render thread (`surface_for`: `create_surface_unsafe(CoreAnimationLayer)`), then
//! configures it and presents its frames there. Nobody runs the game on a Mac yet, so this test
//! runs that same sequence on GitHub's Mac: a layer made here (a bare one: no view or window is
//! needed for it), then on a SPAWNED thread the surface, an adapter that can show it, a device,
//! the configuration the game's window uses (`window.rs`), and one frame cleared and presented.
//! A machine without a Metal device skips with a line saying so.

#![cfg(target_vendor = "apple")]

use objc2_quartz_core::CAMetalLayer;

/// wgpu's native futures are ready when polled after their work (as `device.rs block_on`).
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

/// What the spawned thread did: a frame presented, or no Metal device (skipped).
#[derive(Debug, PartialEq)]
enum Done {
    Presented,
    NoDevice,
}

/// The surface's whole off-main-thread sequence on the layer at `layer`.
fn off_the_main_thread(layer: usize) -> Result<Done, String> {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::METAL,
        ..wgpu::InstanceDescriptor::new_without_display_handle()
    });
    // SAFETY: `layer` is a live CAMetalLayer the test's main thread holds until this thread ends;
    // wgpu's Metal surface retains it for its own life.
    let surface = unsafe {
        instance.create_surface_unsafe(wgpu::SurfaceTargetUnsafe::CoreAnimationLayer(
            layer as *mut std::ffi::c_void,
        ))
    }
    .map_err(|e| format!("create_surface on the Metal layer: {e}"))?;
    let Ok(adapter) = block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::HighPerformance,
        force_fallback_adapter: false,
        compatible_surface: Some(&surface),
        apply_limit_buckets: false,
    })) else {
        return Ok(Done::NoDevice);
    };
    let (device, queue) = block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))
        .map_err(|e| format!("request_device: {e}"))?;
    // The game's window's configuration (window.rs `Window::new`).
    let config = wgpu::SurfaceConfiguration {
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        format: wgpu::TextureFormat::Bgra8Unorm,
        width: 64,
        height: 48,
        present_mode: wgpu::PresentMode::Fifo,
        color_space: wgpu::SurfaceColorSpace::Auto,
        desired_maximum_frame_latency: 2,
        alpha_mode: wgpu::CompositeAlphaMode::Auto,
        view_formats: vec![],
    };
    surface.configure(&device, &config);
    let frame = match surface.get_current_texture() {
        wgpu::CurrentSurfaceTexture::Success(t) | wgpu::CurrentSurfaceTexture::Suboptimal(t) => t,
        other => return Err(format!("no frame from the surface: {other:?}")),
    };
    let view = frame
        .texture
        .create_view(&wgpu::TextureViewDescriptor::default());
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    drop(encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: None,
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view: &view,
            depth_slice: None,
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                store: wgpu::StoreOp::Store,
            },
        })],
        ..Default::default()
    }));
    queue.submit([encoder.finish()]);
    queue.present(frame);
    Ok(Done::Presented)
}

/// The surface is made, configured and presents a frame on a thread other than the main one.
#[test]
fn a_surface_on_a_metal_layer_presents_from_another_thread() {
    let layer = CAMetalLayer::new();
    let at = &*layer as *const CAMetalLayer as usize;
    let done = std::thread::Builder::new()
        .name("render".into())
        .spawn(move || off_the_main_thread(at))
        .unwrap()
        .join()
        .expect("the spawned thread panicked");
    match done {
        Ok(Done::Presented) => {}
        Ok(Done::NoDevice) => eprintln!("no Metal device on this machine: skipped"),
        Err(e) => panic!("{e}"),
    }
    drop(layer);
}
