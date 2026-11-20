//! macOS: the game's device drawing into a window's Metal layer from a thread that is not the main
//! one, through this crate's own path (`surface_for`, the device's window, its configure and its
//! present), on GitHub's Mac (nobody runs the game on a Mac yet; this is the only Mac the project
//! has).
//!
//! The port on macOS (#694): the window layer makes the window and its `CAMetalLayer` on the main
//! thread (AppKit's views are the main thread's only); the game's device makes its surface on that
//! layer from the game's or the render thread, configures it and presents its frames there. This
//! test does the same, in two cases, each on a SPAWNED thread with the crate's backend
//! (`WgpuBackend::new(Output::Window)`, `CreateDevice` on the registered window, a `Clear`, a
//! `Present`), checking that the device was made and its present ran:
//! 1. a bare `CAMetalLayer` (no view, no window): exactly one frame shown;
//! 2. a real window: this program's real main thread runs winit's loop, makes an `NSWindow`, and
//!    the layer from its view as the window layer does (`raw_window_metal::Layer::from_ns_view`).
//!    wgpu's Metal surface then also reads that window's occlusion state on the render thread
//!    each frame, as it will in the game. CI's Mac has no screen session, so its window is
//!    hidden from view and wgpu gives no frame: the port's own outcomes for that,
//!    [`d3d9_wgpu::NoFrame::Occluded`] or `Resting` with no frame shown, pass as well as one frame
//!    shown (the line says which); anything else fails.
//!
//! No test harness (`harness = false`): case 2 needs the process's real main thread. A machine
//! with no Metal adapter (enumerated without any surface) skips, saying so in a line that starts
//! "SKIPPED"; on CI (`GITHUB_ACTIONS` set) that is a failure, as is a window that cannot be made.
//! With a Metal adapter, any failure to make the device or show the frame fails.

#[cfg(not(target_vendor = "apple"))]
fn main() {}

#[cfg(target_vendor = "apple")]
fn main() {
    mac::main()
}

#[cfg(target_vendor = "apple")]
mod mac {
    use d3d9_stream::{Backend, Cmd, Data};
    use d3d9_wgpu::{GameWindow, NoFrame, Output, WgpuBackend};
    use raw_window_handle::{
        DisplayHandle, HandleError, HasDisplayHandle, HasWindowHandle, WindowHandle,
    };
    use std::ptr::NonNull;
    use std::sync::Arc;

    const DEVICE: u32 = 0x1000;
    const BACK: u32 = 0x1100;
    /// `D3DFMT_A8R8G8B8`.
    const A8R8G8B8: u32 = 21;
    const WIDTH: u32 = 64;
    const HEIGHT: u32 = 48;

    fn words(w: &[u32]) -> Vec<u8> {
        w.iter().flat_map(|w| w.to_le_bytes()).collect()
    }

    fn on_ci() -> bool {
        std::env::var_os("GITHUB_ACTIONS").is_some()
    }

    /// Ends the run: a failure (status 1) with `why`.
    fn fail(why: &str) -> ! {
        eprintln!("metal_surface: FAILED: {why}");
        std::process::exit(1)
    }

    /// A window as the drawing side sees it: its Metal layer, and (case 2) winit's window for the
    /// handles; the bare layer of case 1 has no handles, and the surface never asks for them.
    struct TestWindow {
        layer: raw_window_metal::Layer,
        window: Option<winit::window::Window>,
    }

    impl HasWindowHandle for TestWindow {
        fn window_handle(&self) -> Result<WindowHandle<'_>, HandleError> {
            match &self.window {
                Some(w) => w.window_handle(),
                None => Err(HandleError::Unavailable),
            }
        }
    }

    impl HasDisplayHandle for TestWindow {
        fn display_handle(&self) -> Result<DisplayHandle<'_>, HandleError> {
            match &self.window {
                Some(w) => w.display_handle(),
                None => Err(HandleError::Unavailable),
            }
        }
    }

    impl GameWindow for TestWindow {
        fn client_size(&self) -> Option<(u32, u32)> {
            Some((WIDTH, HEIGHT))
        }
        fn minimised(&self) -> bool {
            false
        }
        fn refresh_millihertz(&self) -> Option<u32> {
            None
        }
        fn metal_layer(&self) -> Option<NonNull<std::ffi::c_void>> {
            Some(self.layer.as_ptr())
        }
    }

    /// wgpu's native futures are ready when polled after their work (as device.rs block_on).
    fn block_on<F: std::future::Future>(f: F) -> F::Output {
        struct Noop;
        impl std::task::Wake for Noop {
            fn wake(self: Arc<Self>) {}
        }
        let waker = std::task::Waker::from(Arc::new(Noop));
        let mut cx = std::task::Context::from_waker(&waker);
        let mut f = std::pin::pin!(f);
        loop {
            if let std::task::Poll::Ready(v) = f.as_mut().poll(&mut cx) {
                return v;
            }
            std::thread::yield_now();
        }
    }

    /// Whether wgpu has a Metal graphics card here, asked with no surface involved.
    fn metal_card() -> Option<String> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::METAL,
            ..wgpu::InstanceDescriptor::new_without_display_handle()
        });
        let adapters = block_on(instance.enumerate_adapters(wgpu::Backends::METAL));
        adapters
            .iter()
            .map(|a| a.get_info())
            .find(|i| i.device_type != wgpu::DeviceType::Cpu)
            .map(|i| i.name)
    }

    /// What one present did: the frames the window showed, and why its frames are not shown (the
    /// first reason of the stretch: the frames after an Occluded one rest; None when one was shown).
    type Shown = (u64, Option<NoFrame>);

    /// On a spawned thread, as the game's render thread: the backend's device on the registered
    /// window `id`, its back buffer cleared and presented once.
    fn draw_off_the_main_thread(id: isize) -> std::thread::JoinHandle<Result<Shown, String>> {
        std::thread::Builder::new()
            .name("render".into())
            .spawn(move || {
                let mut b = WgpuBackend::new(Output::Window);
                let present = words(&[WIDTH, HEIGHT, A8R8G8B8, 1, 0, 0, 1, 0, 1, 0, 0, 0, 0, 0]);
                b.execute(Cmd::CreateDevice {
                    device: DEVICE,
                    focus: id,
                    window: id,
                    behavior: 0x54,
                    present: Data::Bytes(&present),
                    back_buffers: Data::Bytes(&words(&[BACK])),
                    depth_stencil: 0,
                });
                if b.device().is_none() {
                    return Err("the device was not made on the window's Metal layer \
                                (the lines above say why)"
                        .into());
                }
                b.execute(Cmd::SetRenderTarget {
                    device: DEVICE,
                    index: 0,
                    surface: BACK,
                });
                b.execute(Cmd::Clear {
                    device: DEVICE,
                    count: 0,
                    rects: None,
                    flags: 1,
                    colour: 0xff20_6040,
                    z: 1f32.to_bits(),
                    stencil: 0,
                    model: 0,
                });
                b.execute(Cmd::Present {
                    device: DEVICE,
                    source: None,
                    dest: None,
                    window: 0,
                    dirty: false,
                    model: 0,
                    frame: 1,
                });
                match b.device() {
                    Some(d) => Ok((d.frames_shown(), d.frame_not_shown())),
                    None => Err("the device was gone after its present".into()),
                }
            })
            .expect("the render thread")
    }

    /// The result of one case: one frame shown; or, with `hidden_ok` (a real window, which CI's
    /// Mac keeps hidden from view), no frame shown because the window is hidden from view (the
    /// port's own outcomes for a hidden window: `NoFrame::Occluded`, or `NoFrame::Resting`, the
    /// window left alone after a frame it did not give). Anything else is a failure.
    fn judge(case: &str, hidden_ok: bool, done: std::thread::Result<Result<Shown, String>>) {
        match done {
            Ok(Ok((1, None))) => eprintln!("metal_surface: {case}: device made, 1 frame shown: ok"),
            Ok(Ok((0, Some(why @ (NoFrame::Occluded | NoFrame::Resting))))) if hidden_ok => {
                eprintln!(
                    "metal_surface: {case}: device made, present ran, no frame shown because the \
                     window is hidden from view ({why:?}): ok"
                )
            }
            Ok(Ok((n, why))) => fail(&format!(
                "{case}: {n} frames shown (not shown because: {why:?}); 1 expected{}",
                if hidden_ok {
                    ", or none because the window is hidden from view (Occluded or Resting)"
                } else {
                    ""
                }
            )),
            Ok(Err(e)) => fail(&format!("{case}: {e}")),
            Err(_) => fail(&format!("{case}: the render thread panicked")),
        }
    }

    pub fn main() {
        let Some(card) = metal_card() else {
            if on_ci() {
                fail("no Metal graphics card on this CI runner (enumerated without a surface)");
            }
            eprintln!(
                "metal_surface: SKIPPED (not a pass): no Metal graphics card on this machine"
            );
            return;
        };
        eprintln!("metal_surface: Metal card: {card}");
        // Case 1: a bare layer.
        let bare = objc2_quartz_core::CAMetalLayer::new();
        // SAFETY: a CAMetalLayer the test holds for the whole run (`bare` is never dropped
        // before the process ends).
        let layer = unsafe { raw_window_metal::Layer::from_ca_layer(NonNull::from(&*bare).cast()) };
        d3d9_wgpu::register_window(
            1,
            Arc::new(TestWindow {
                layer,
                window: None,
            }),
        );
        judge("bare layer", false, draw_off_the_main_thread(1).join());
        // Case 2: a real window on this, the real main thread.
        window::run();
    }

    mod window {
        use super::*;
        use winit::application::ApplicationHandler;
        use winit::event::WindowEvent;
        use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
        use winit::window::WindowId;

        #[derive(Default)]
        struct App {
            render: Option<std::thread::JoinHandle<Result<Shown, String>>>,
            done: bool,
        }

        impl ApplicationHandler for App {
            fn resumed(&mut self, event_loop: &ActiveEventLoop) {
                if self.render.is_some() || self.done {
                    return;
                }
                let attributes = winit::window::Window::default_attributes()
                    .with_title("nv-rs metal_surface test")
                    .with_inner_size(winit::dpi::LogicalSize::new(WIDTH, HEIGHT));
                let window = match event_loop.create_window(attributes) {
                    Ok(w) => w,
                    Err(e) => fail(&format!("window: winit could not make an NSWindow ({e})")),
                };
                // On the main thread, as the window layer makes it (winit_window.rs).
                let layer = match window.window_handle().map(|h| h.as_raw()) {
                    Ok(raw_window_handle::RawWindowHandle::AppKit(h)) => {
                        // SAFETY: winit's AppKit handle names the window's live NSView, and this
                        // runs on the main thread (winit's loop).
                        unsafe { raw_window_metal::Layer::from_ns_view(h.ns_view) }
                    }
                    other => fail(&format!("window: no AppKit view ({other:?})")),
                };
                d3d9_wgpu::register_window(
                    2,
                    Arc::new(TestWindow {
                        layer,
                        window: Some(window),
                    }),
                );
                self.render = Some(draw_off_the_main_thread(2));
                event_loop.set_control_flow(ControlFlow::Poll);
            }

            fn window_event(&mut self, _: &ActiveEventLoop, _: WindowId, _: WindowEvent) {}

            fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
                // The main thread keeps answering AppKit (winit's window calls from the render
                // thread run here) until the render thread is done.
                if self.render.as_ref().is_some_and(|r| r.is_finished()) {
                    let render = self.render.take().expect("the render thread");
                    judge("real window", true, render.join());
                    self.done = true;
                    event_loop.exit();
                }
            }
        }

        pub fn run() {
            let event_loop = match EventLoop::new() {
                Ok(l) => l,
                Err(e) => fail(&format!("window: winit's loop could not start ({e})")),
            };
            let mut app = App::default();
            if let Err(e) = event_loop.run_app(&mut app) {
                fail(&format!("window: winit's loop ended with {e}"));
            }
            if !app.done {
                fail("window: winit's loop ended before the render thread was done");
            }
        }
    }
}
