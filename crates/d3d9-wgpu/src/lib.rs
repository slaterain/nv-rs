//! Direct3D 9 on wgpu (ADR-0007): the part of Direct3D 9 the game uses, as a backend of the
//! recorded call stream (`d3d9-stream`), and the converter of the game's shader bytecode.
//!
//! The game's own renderer is translated from the exe and calls Direct3D 9 through the device's
//! interface; the platform layer's model answers every call and records the ones that change what
//! is drawn (`measure/render-thread/DESIGN.md`, "The seam"). This crate carries those out on wgpu
//! (Vulkan, Direct3D 12, Metal): [`WgpuBackend`] is a [`d3d9_stream::Backend`]. It reads no game
//! memory and keeps no answer the game reads, apart from the sync commands' replies.
//!
//! - `sm`: shader bytecode read, listed and converted to WGSL (`sm::wgsl`).
//! - `format`: `D3DFORMAT` to wgpu formats.
//! - `state`: render and sampler states to wgpu states.
//! - `device`: resources, passes, pipelines, draws and `Present`.
//! - `shot`: `NV_SHOT` captures numbered like the forwarding layer's.

pub mod device;
pub mod format;
mod fxhash;
pub mod shot;
pub mod sm;
pub mod state;

use d3d9_stream::{Backend, Cmd, Data, Handle, Reply};
pub use device::{hardware_adapter, Output};

/// The Direct3D 9 calls carried out on wgpu.
pub struct WgpuBackend {
    output: Output,
    device: Option<device::Device>,
    /// The handle of the game's device (one is made).
    game_device: Handle,
}

impl WgpuBackend {
    /// A backend whose frames go to the game's window, or nowhere but captures
    /// ([`Output::Offscreen`], the shadow of the forwarding to the real `d3d9.dll`).
    pub fn new(output: Output) -> Self {
        WgpuBackend {
            output,
            device: None,
            game_device: 0,
        }
    }

    /// The wgpu device, once the game made its own.
    pub fn device(&mut self) -> Option<&mut device::Device> {
        self.device.as_mut()
    }
}

/// Moves and sizes `window` over its monitor (in the monitor's real pixels).
#[cfg(windows)]
fn cover_monitor(window: isize) {
    #[repr(C)]
    struct MonitorInfo {
        size: u32,
        monitor: [i32; 4],
        work: [i32; 4],
        flags: u32,
    }
    #[link(name = "user32")]
    extern "system" {
        fn MonitorFromWindow(window: isize, flags: u32) -> isize;
        fn GetMonitorInfoW(monitor: isize, info: *mut MonitorInfo) -> i32;
        fn SetThreadDpiAwarenessContext(context: isize) -> isize;
        fn SetWindowPos(window: isize, after: isize, x: i32, y: i32, w: i32, h: i32, f: u32)
            -> i32;
    }
    /// `DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2`, `MONITOR_DEFAULTTONEAREST`,
    /// `SWP_NOZORDER | SWP_NOACTIVATE | SWP_ASYNCWINDOWPOS`: the window belongs to another thread
    /// (engine-platform's window thread, which only pumps messages, but the request is posted
    /// rather than sent all the same, so this thread can never wait on it).
    const PER_MONITOR_V2: isize = -4;
    const NEAREST: u32 = 2;
    const FLAGS: u32 = 0x4 | 0x10 | 0x4000;
    // SAFETY: plain calls on a window the window thread owns; `info` is this frame's, its size set.
    unsafe {
        let before = SetThreadDpiAwarenessContext(PER_MONITOR_V2);
        let mut info = MonitorInfo {
            size: std::mem::size_of::<MonitorInfo>() as u32,
            monitor: [0; 4],
            work: [0; 4],
            flags: 0,
        };
        if GetMonitorInfoW(MonitorFromWindow(window, NEAREST), &mut info) != 0 {
            let [l, t, r, b] = info.monitor;
            SetWindowPos(window, 0, l, t, r - l, b - t, FLAGS);
        }
        if before != 0 {
            SetThreadDpiAwarenessContext(before);
        }
    }
}

#[cfg(not(windows))]
fn cover_monitor(_window: isize) {}

fn handles(d: &Data<'_>) -> Vec<Handle> {
    d.to_words().into_owned()
}

fn rect_words(d: &Data<'_>) -> Vec<i32> {
    d.to_words().iter().map(|&w| w as i32).collect()
}

/// A rectangle a command carries (left, top, right, bottom).
fn rect(d: &Option<Data<'_>>) -> Option<[i32; 4]> {
    let w = rect_words(d.as_ref()?);
    (w.len() == 4).then(|| [w[0], w[1], w[2], w[3]])
}

impl Backend for WgpuBackend {
    fn execute(&mut self, cmd: Cmd<Data<'_>>) -> Reply {
        if let Cmd::CreateDevice {
            device,
            focus,
            window,
            present,
            back_buffers,
            depth_stencil,
            ..
        } = &cmd
        {
            let pp = device::PresentParameters::from_words(&present.to_words());
            let hwnd = if *window != 0 { *window } else { *focus };
            // `Windowed` 0 (`bFull Screen=1`): wgpu has no exclusive mode switch, so the frame goes
            // to the window made to cover its monitor instead, as a borderless window does.
            if present.to_words().get(8) == Some(&0) && hwnd != 0 && self.output == Output::Window {
                eprintln!("NV_D3D9 wgpu: full screen asked: drawn in a borderless window over the monitor (wgpu has no exclusive full screen)");
                cover_monitor(hwnd);
            }
            return match device::Device::new(
                self.output,
                hwnd,
                &pp,
                &handles(back_buffers),
                *depth_stencil,
            ) {
                Ok(d) => {
                    self.device = Some(d);
                    self.game_device = *device;
                    // No D3DX side channel: the texture loader cannot use a wgpu device.
                    Reply::Device {
                        created: true,
                        native: None,
                    }
                }
                Err(e) => {
                    eprintln!("NV_D3D9 wgpu: CreateDevice: {e}");
                    Reply::Device {
                        created: false,
                        native: None,
                    }
                }
            };
        }
        let Some(d) = self.device.as_mut() else {
            return match cmd {
                Cmd::ReadSurface { .. } => Reply::Bytes(None),
                Cmd::QueryGetData { .. } => Reply::Query(None),
                Cmd::GetRenderTargetData { .. } => Reply::Rows(None),
                _ => Reply::None,
            };
        };
        match cmd {
            Cmd::CreateDevice { .. } => unreachable!("handled above"),
            Cmd::Reset {
                present,
                back_buffers,
                depth_stencil,
                ..
            } => {
                let pp = device::PresentParameters::from_words(&present.to_words());
                d.reset(&pp, &handles(&back_buffers), depth_stencil);
            }
            Cmd::Present { frame, .. } => d.present(frame),
            Cmd::BeginScene { .. } | Cmd::EndScene { .. } => {}
            Cmd::Clear {
                rects,
                flags,
                colour,
                z,
                stencil,
                ..
            } => {
                let rects = rects.as_ref().map(rect_words);
                d.clear(rects.as_deref(), flags, colour, f32::from_bits(z), stencil);
            }
            Cmd::SetRenderTarget { index, surface, .. } => d.set_render_target(index, surface),
            Cmd::SetDepthStencilSurface { surface, .. } => d.set_depth_stencil(surface),
            Cmd::SetViewport { viewport, .. } => d.set_viewport(viewport),
            Cmd::SetScissorRect { rect, .. } => d.set_scissor(rect),
            Cmd::SetRenderState { state, value, .. } => d.set_render_state(state, value),
            Cmd::SetSamplerState {
                sampler,
                kind,
                value,
                ..
            } => d.set_sampler_state(sampler, kind, value),
            Cmd::SetTexture {
                sampler, texture, ..
            } => d.set_texture(sampler, texture),
            Cmd::SetVertexDeclaration { declaration, .. } => d.set_declaration(declaration),
            Cmd::SetShader { pixel, shader, .. } => d.set_shader(pixel, shader),
            Cmd::SetConstants {
                kind,
                start,
                count,
                data,
                ..
            } => d.set_constants(kind, start, count, &data.to_words()),
            Cmd::SetStreamSource {
                stream,
                buffer,
                offset,
                stride,
                ..
            } => d.set_stream(stream, buffer, offset, stride),
            Cmd::SetIndices { buffer, .. } => d.set_indices(buffer),
            Cmd::DrawPrimitive {
                kind, start, count, ..
            } => d.draw(kind, false, 0, start, count, None),
            Cmd::DrawIndexedPrimitive {
                kind,
                base_vertex,
                start_index,
                count,
                ..
            } => d.draw(kind, true, base_vertex, start_index, count, None),
            Cmd::DrawPrimitiveUp {
                kind,
                count,
                stride,
                vertices,
                ..
            } => {
                let bytes = vertices.bytes();
                d.draw(kind, false, 0, 0, count, Some((&bytes[..], stride)));
            }
            Cmd::UpdateTexture {
                source,
                destination,
                ..
            } => d.update_texture(source, destination),
            Cmd::StretchRect {
                source,
                source_rect,
                destination,
                destination_rect,
                filter,
                ..
            } => d.stretch_rect(
                source,
                rect(&source_rect),
                destination,
                rect(&destination_rect),
                filter,
            ),
            Cmd::CreateTexture {
                texture,
                levels,
                width,
                height,
                usage,
                format,
                pool,
                ..
            } => d.create_texture(
                texture,
                &handles(&levels),
                width,
                height,
                usage,
                format,
                false,
                pool,
            ),
            Cmd::CreateCubeTexture {
                texture,
                surfaces,
                edge,
                usage,
                format,
                pool,
                ..
            } => d.create_texture(
                texture,
                &handles(&surfaces),
                edge,
                edge,
                usage,
                format,
                true,
                pool,
            ),
            Cmd::CreateSurface {
                surface,
                depth,
                width,
                height,
                format,
                multisample,
                ..
            } => d.create_surface(surface, depth, width, height, format, multisample),
            Cmd::CreateOffscreenSurface {
                surface,
                width,
                height,
                format,
                pool,
                ..
            } => d.create_offscreen(surface, width, height, format, pool),
            Cmd::CreateBuffer {
                buffer,
                index,
                length,
                format,
                ..
            } => d.create_buffer(buffer, index, length, format),
            Cmd::CreateShader { shader, tokens, .. } => d.create_shader(shader, &tokens.to_words()),
            Cmd::CreateDeclaration {
                declaration,
                elements,
                ..
            } => d.create_declaration(declaration, &elements.bytes()),
            Cmd::CreateQuery { query, kind, .. } => d.create_query(query, kind),
            Cmd::Destroy { object } => d.destroy(object),
            Cmd::UploadSurface { surface, data, .. } => d.upload_surface(surface, &data.bytes()),
            Cmd::UploadBuffer {
                buffer,
                offset,
                data,
                ..
            } => d.upload_buffer(buffer, offset, &data),
            Cmd::ReadSurface { surface, size } => {
                return Reply::Bytes(d.read_surface(surface, size))
            }
            Cmd::GetRenderTargetData {
                source,
                destination,
                pitch,
                ..
            } => return Reply::Rows(d.get_render_target_data(source, destination, pitch)),
            Cmd::QueryGetData { query, flags } => {
                return Reply::Query(d.query_get_data(query, flags))
            }
            Cmd::QueryIssue { query, flags } => d.query_issue(query, flags),
            Cmd::SetClipPlane { index, plane, .. } => d.set_clip_plane(index, plane),
            Cmd::ColorFill { .. } => d.not_provided("ColorFill is not provided (skipped)"),
            // Fixed-function state and the rest: the game's shaders do not read them; a draw
            // without shaders is skipped and printed by the device. (`SetGammaRamp` changes
            // nothing in a window, which is how the game runs.)
            Cmd::SetTransform { .. }
            | Cmd::SetGammaRamp { .. }
            | Cmd::SetTextureStageState { .. }
            | Cmd::SetFvf { .. }
            | Cmd::SetMaterial { .. }
            | Cmd::SetLight { .. }
            | Cmd::LightEnable { .. }
            | Cmd::SetSoftwareVertexProcessing { .. }
            | Cmd::SetLod { .. } => {}
        }
        Reply::None
    }
}
