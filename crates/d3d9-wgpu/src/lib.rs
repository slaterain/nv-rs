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
//! - `ffp`: the fixed-function pipeline for draws without shaders (FVF layouts, pre-transformed
//!   vertices, the texture stages), as shader bytecode the converter takes.
//! - `device`: resources, passes, pipelines, draws and `Present`.
//! - `window`: the game's window: the swap chain at the window's size, minimised, sized or lost.
//! - `shot`: `NV_SHOT` captures numbered like the forwarding layer's.
//! - `scene_light`: the new lighting (port-added, off by default): ambient occlusion on the finished
//!   scene.
//! - `sun_shadow`: sun shadows (port-added, off by default): cascaded shadow maps from the game's
//!   own opaque draws, shown by a full-screen step.
//! - `bright`: the HDR watch (a diagnostic, read only): the game's float targets checked for
//!   pixels that are not finite, lone bright dots and jumps of the luminance targets.

mod ahead;
mod answers;
// Used off Windows only; its tests run everywhere.
#[cfg_attr(windows, allow(dead_code))]
mod bitmap_font;
pub mod bright;
mod cache;
pub mod device;
mod ffp;
#[cfg(test)]
mod fingerprint;
pub mod format;
pub mod frame_rate;
mod fxhash;
pub mod gpu_times;
pub mod loading;
mod prepare;
pub mod scene_light;
pub mod shot;
pub mod sm;
pub mod state;
pub mod sun_shadow;
mod window;

/// Writes every live wgpu device's pipeline cache and pipeline list now, when they changed: for
/// the end of the run (the host's exit hooks; the process ends with `std::process::exit`).
pub fn save_pipeline_caches() {
    cache::save_all_now();
    device::save_lists_now();
}
pub use answers::{Answer, QueryAnswers};
use d3d9_stream::{Backend, Cmd, Data, Handle, Reply};
pub use device::{hardware_adapter, Output};
pub use window::window_changed;
pub use window::NoFrame;
#[cfg(not(windows))]
pub use window::{register_window, GameWindow};

/// The Direct3D 9 calls carried out on wgpu.
pub struct WgpuBackend {
    output: Output,
    device: Option<device::Device>,
    /// The handle of the game's device (one is made).
    game_device: Handle,
    /// Where the driver's compiled pipelines are kept between runs ([`WgpuBackend::with_cache_dir`]).
    cache_dir: Option<std::path::PathBuf>,
    /// Where occlusion counts are posted for the game's threads ([`WgpuBackend::with_answers`]).
    answers: Option<std::sync::Arc<QueryAnswers>>,
}

impl WgpuBackend {
    /// A backend whose frames go to the game's window, or nowhere but captures
    /// ([`Output::Offscreen`], the shadow of the forwarding to the real `d3d9.dll`).
    pub fn new(output: Output) -> Self {
        WgpuBackend {
            output,
            device: None,
            game_device: 0,
            cache_dir: None,
            answers: None,
        }
    }

    /// Keeps the driver's compiled pipelines in `dir` between runs, where the backend has a
    /// pipeline cache (Vulkan): read when the device is made, written after new pipelines
    /// (`cache.rs`). The host chooses the folder; without one nothing is read or written.
    pub fn with_cache_dir(mut self, dir: std::path::PathBuf) -> Self {
        self.cache_dir = Some(dir);
        self
    }

    /// Posts each occlusion count to `answers` once the GPU has it, for the game's threads to
    /// read without waiting for the render thread ([`QueryAnswers`]).
    pub fn with_answers(mut self, answers: std::sync::Arc<QueryAnswers>) -> Self {
        self.answer_to(answers);
        self
    }

    /// [`WgpuBackend::with_answers`] on a backend that may have made its device already: the
    /// queries made from now on are posted.
    pub fn answer_to(&mut self, answers: std::sync::Arc<QueryAnswers>) {
        if let Some(d) = self.device.as_mut() {
            d.answers = Some(std::sync::Arc::clone(&answers));
        }
        self.answers = Some(answers);
    }

    /// The wgpu device, once the game made its own.
    pub fn device(&mut self) -> Option<&mut device::Device> {
        self.device.as_mut()
    }
}

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
            // A panic while the device is made (in wgpu or a driver's answer it did not expect)
            // answers as a refused device does: the platform then draws with Direct3D 9.
            let made = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                device::Device::new(
                    self.output,
                    if *window != 0 { *window } else { *focus },
                    &pp,
                    &handles(back_buffers),
                    *depth_stencil,
                    self.cache_dir.as_deref(),
                )
            }))
            .unwrap_or_else(|_| Err("making the device panicked".into()));
            return match made {
                Ok(mut d) => {
                    d.answers = self.answers.clone();
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
        // A batch that has taken the most one submit carries is sent before the next command.
        d.bound_batch();
        // What the last draw resolved is used again only while nothing it came from changed
        // ([`device::Device::state_changed`]): the sets below count their own changes (most set
        // what is set already), the draws and the uploads change none of it, and every other
        // command (targets, clears, queries, creations, releases, presents...) counts as a change.
        if !matches!(
            cmd,
            Cmd::DrawIndexedPrimitive { .. }
                | Cmd::DrawPrimitive { .. }
                | Cmd::DrawPrimitiveUp { .. }
                | Cmd::SetConstants { .. }
                | Cmd::SetStreamSource { .. }
                | Cmd::SetIndices { .. }
                | Cmd::SetTexture { .. }
                | Cmd::SetSamplerState { .. }
                | Cmd::SetRenderState { .. }
                | Cmd::SetTextureStageState { .. }
                | Cmd::SetShader { .. }
                | Cmd::SetVertexDeclaration { .. }
                | Cmd::SetFvf { .. }
                | Cmd::SetViewport { .. }
                | Cmd::SetScissorRect { .. }
                | Cmd::UploadBuffer { .. }
                | Cmd::UploadSurface { .. }
                | Cmd::QueryGetData { .. }
        ) {
            d.state_changed();
        }
        // Held draws ([`device::Device::emit_held`]) are given to the pass before any command
        // that is not a draw or a set of what later draws use: each held draw has its own state
        // already, and nothing else (a clear, a copy, an upload, a query, a target, a release)
        // may come before a draw the game gave before it.
        if !matches!(
            cmd,
            Cmd::DrawIndexedPrimitive { .. }
                | Cmd::DrawPrimitive { .. }
                | Cmd::SetConstants { .. }
                | Cmd::SetStreamSource { .. }
                | Cmd::SetIndices { .. }
                | Cmd::SetTexture { .. }
                | Cmd::SetSamplerState { .. }
                | Cmd::SetRenderState { .. }
                | Cmd::SetTextureStageState { .. }
                | Cmd::SetShader { .. }
                | Cmd::SetVertexDeclaration { .. }
                | Cmd::SetFvf { .. }
                | Cmd::SetViewport { .. }
                | Cmd::SetScissorRect { .. }
                | Cmd::SetClipPlane { .. }
        ) {
            d.emit_held();
        }
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
            Cmd::SetFvf { fvf, .. } => d.set_fvf(fvf),
            Cmd::SetTextureStageState {
                stage, kind, value, ..
            } => d.set_stage_state(stage, kind, value),
            Cmd::SetShader { pixel, shader, .. } => d.set_shader(pixel, shader),
            Cmd::SetConstants {
                kind,
                start,
                count,
                data,
                ..
            } => d.set_constants(kind, start, count, &data),
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
            // The new lighting's frame parameters, recorded just before the game's depth-only
            // clear of its scene ([`scene_light`]); no Direct3D 9 transform has this number.
            Cmd::SetTransform { state, matrix, .. } if state == scene_light::STATE => {
                d.scene_light(&matrix.to_words());
            }
            // The sun shadows' frame, recorded just after the new lighting's ([`sun_shadow`]).
            Cmd::SetTransform { state, matrix, .. } if state == sun_shadow::STATE => {
                d.sun_shadows(&matrix.to_words());
            }
            // The sun shadows' camera and frame at the scene's clear, for this frame's lit draws.
            Cmd::SetTransform { state, matrix, .. } if state == sun_shadow::VIEW_STATE => {
                d.sun_begin_view(&matrix.to_words());
            }
            // The world the camera is in, before them: a new one forgets the casters kept.
            Cmd::SetTransform { state, matrix, .. } if state == sun_shadow::WORLD_STATE => {
                d.sun_world(&matrix.to_words());
            }
            Cmd::SetTransform { state, matrix, .. } if state == sun_shadow::BEGIN_STATE => {
                d.sun_lookup(&matrix.to_words());
            }
            // The fixed-function vertex pipeline's state (transforms, materials, lights): no draw
            // of the game uses it (the fixed-function draws, `ffp`, are pre-transformed), and a
            // draw of untransformed vertices without a vertex shader is skipped and printed by
            // the device. (`SetGammaRamp` changes nothing in a window, which is how the game runs.)
            Cmd::SetTransform { .. }
            | Cmd::SetGammaRamp { .. }
            | Cmd::SetMaterial { .. }
            | Cmd::SetLight { .. }
            | Cmd::LightEnable { .. }
            | Cmd::SetSoftwareVertexProcessing { .. }
            | Cmd::SetLod { .. } => {}
        }
        Reply::None
    }

    /// The awaited occlusion counts polled and posted ([`QueryAnswers`]); more turns while any is
    /// still on the GPU.
    fn idle(&mut self) -> bool {
        self.device.as_mut().is_some_and(device::Device::idle)
    }
}

/// A recording (`NV_D3D9_RECORD`'s file, [`d3d9_stream::wire`]) replayed into an offscreen
/// backend with the draw records on or off ([`device::Device::set_draw_records`]): each draw's
/// digest of what it gave wgpu ([`device::Device::record_draw_digests`]) with the number of the
/// Present it came before, for the draws before Presents `from` to `to` (all commands are
/// carried out). The proof that the records change nothing: the same list with them off and on
/// (`examples/replay_digests.rs`, and the frame tests on their own recording). None without a
/// GPU adapter.
pub fn replay_draw_digests(
    bytes: &[u8],
    records: bool,
    from: u64,
    to: u64,
    cache: Option<std::path::PathBuf>,
) -> Result<Option<Vec<(u64, u64)>>, d3d9_stream::wire::WireError> {
    replay_draw_digests_with(bytes, records, false, from, to, cache)
}

/// [`replay_draw_digests`] with the registers read from storage or not
/// ([`device::Device::set_storage_regs`]). None also where `storage` is asked and the adapter
/// cannot.
pub fn replay_draw_digests_with(
    bytes: &[u8],
    records: bool,
    storage: bool,
    from: u64,
    to: u64,
    cache: Option<std::path::PathBuf>,
) -> Result<Option<Vec<(u64, u64)>>, d3d9_stream::wire::WireError> {
    Ok(replay_digests(bytes, records, storage, None, from, to, cache)?.map(|(drawn, _)| drawn))
}

/// A recording replayed with the registers from storage and the held draws merged
/// ([`device::Device::set_merge_draws`]; sorted with `sort`, blended draws held after the opaque
/// ones with `across`), records on: each draw's digest as the game gave the draws and as the pass
/// was given them ([`device::Device::take_emitted_digests`]),
/// each with its Present. The proof that merging draws every draw once with what it gave wgpu:
/// the first list equals the unmerged replay's, and the second holds the same digests within each
/// Present. None without a GPU adapter or where it cannot read the registers from storage.
#[allow(clippy::type_complexity)]
pub fn replay_merged_digests(
    bytes: &[u8],
    sort: bool,
    across: bool,
    from: u64,
    to: u64,
    cache: Option<std::path::PathBuf>,
) -> Result<Option<(Vec<(u64, u64)>, Vec<(u64, u64)>)>, d3d9_stream::wire::WireError> {
    replay_digests(bytes, true, true, Some((sort, across)), from, to, cache)
}

/// [`replay_draw_digests_with`] and [`replay_merged_digests`]: `merge` the sorting and holding
/// across blended draws when the held draws are merged.
#[allow(clippy::type_complexity)]
fn replay_digests(
    bytes: &[u8],
    records: bool,
    storage: bool,
    merge: Option<(bool, bool)>,
    from: u64,
    to: u64,
    cache: Option<std::path::PathBuf>,
) -> Result<Option<(Vec<(u64, u64)>, Vec<(u64, u64)>)>, d3d9_stream::wire::WireError> {
    let mut b = WgpuBackend::new(Output::Offscreen);
    if let Some(dir) = cache {
        b = b.with_cache_dir(dir);
    }
    let mut set = false;
    let mut present = 0u64;
    let mut out = Vec::new();
    let mut emitted = Vec::new();
    for c in d3d9_stream::wire::commands(bytes)? {
        let c = c?;
        let p = matches!(c, Cmd::Present { .. });
        b.execute(c);
        match b.device() {
            Some(d) => {
                if !set {
                    if d.set_storage_regs(storage) != storage {
                        return Ok(None);
                    }
                    d.set_draw_records(records);
                    if let Some((sort, across)) = merge {
                        d.set_merge_draws(true, sort, across);
                    }
                    d.record_draw_digests(true);
                    set = true;
                }
                let digests = d.take_draw_digests();
                if present >= from && present < to {
                    out.extend(digests.into_iter().map(|x| (x, present)));
                }
                let given = d.take_emitted_digests();
                if present >= from && present < to {
                    emitted.extend(given.into_iter().map(|x| (x, present)));
                }
            }
            None if set => return Ok(None),
            None => {}
        }
        if p {
            present += 1;
        }
    }
    Ok(set.then_some((out, emitted)))
}
