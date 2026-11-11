// This file is an altered version of parts of DXVK v3.1.1 (https://github.com/doitsujin/dxvk):
// translated into Rust from src/dxvk/dxvk_graphics.cpp, src/dxvk/dxvk_shader.cpp and
// src/dxvk/dxvk_context.cpp, and changed to make wgpu-hal's render pipelines, for nv-rs
// (vendor/README.md). DXVK's notice follows, as its licence requires.
//
//                   Copyright (c) 2017 Philip Rebohle
//                   Copyright (c) 2019 Joshua Ashton
//                   Copyright (c) 2019 Robin Kertels
//                   Copyright (c) 2023 Jeffrey Ellison
//
//                           zlib/libpng license
//
// This software is provided 'as-is', without any express or implied
// warranty. In no event will the authors be held liable for any damages
// arising from the use of this software.
//
// Permission is granted to anyone to use this software for any purpose,
// including commercial applications, and to alter it and redistribute it
// freely, subject to the following restrictions:
//
// – The origin of this software must not be misrepresented; you must not
//   claim that you wrote the original software. If you use this software
//   in a product, an acknowledgment in the product documentation would be
//   appreciated but is not required.
//
// – Altered source versions must be plainly marked as such, and must not
//   be misrepresented as being the original software.
//
// – This notice may not be removed or altered from any source distribution.

//! Render pipelines linked from pipeline libraries (`VK_EXT_graphics_pipeline_library`), the way
//! DXVK makes them, so that a new combination of shaders and state costs a link instead of a
//! compile.
//!
//! A pipeline is linked, without link-time optimisation, from four libraries:
//! - vertex input: the vertex buffer layouts, the topology and primitive restart (no shader);
//! - pre-rasterisation: the vertex shader, with cull mode, front face and depth bias dynamic;
//! - fragment shader: the fragment shader, with the depth and stencil tests dynamic;
//! - fragment output: the target formats, blending, write masks and multisampling (no shader).
//!
//! Each library is made once per key and kept: the shaders' until their module is destroyed, the
//! two kinds without a shader for the device's life. Linking four libraries that exist costs about
//! as much as recording a command. The states made dynamic are set on the command buffer when the
//! pipeline is bound ([`DynamicState`]), so pipelines that differ only in them are one linked
//! pipeline. The shader libraries are made with layouts of only the sets their stage can see
//! (`VK_PIPELINE_LAYOUT_CREATE_INDEPENDENT_SETS_BIT_EXT`), so a shader's library does not depend on
//! the shader it is paired with when the bind groups are separate per stage.
//!
//! Render pipelines this cannot make (a shader module wgpu-hal keeps as naga IR, mesh shaders,
//! multiview, a layout with a gap or with a group no draw stage sees) are made whole, as published
//! wgpu-hal makes them, with the same layout. A pipeline layout's sets are bound for draws with
//! its independent-sets variant ([`super::PipelineLayout::raw_for`]): layouts with and without the
//! flag are not compatible, and compute pipelines cannot use the flag.

// Our code in a vendored crate: no CI step lints it (`vendor/` is out of the workspace), and as a
// path dependency its warnings are not capped, so a warning here fails every build.
#![deny(warnings)]

use alloc::{boxed::Box, string::String, sync::Arc, sync::Weak, vec::Vec};
use core::{
    fmt::{self, Write as _},
    sync::atomic::{AtomicU64, Ordering},
};
use std::{sync::OnceLock, time::Instant};

use arrayvec::ArrayVec;
use ash::vk;
use naga::FastHashMap;
use parking_lot::Mutex;

use super::conv;

// ---------------------------------------------------------------------------
// Pipeline layouts
// ---------------------------------------------------------------------------

/// What a pipeline layout keeps for linked pipelines ([`LayoutParts::new`]).
pub(super) struct LayoutParts {
    /// The layout's sets with `VK_PIPELINE_LAYOUT_CREATE_INDEPENDENT_SETS_BIT_EXT`: render
    /// pipelines are made with it, and descriptor sets for draws are bound with it.
    independent: vk::PipelineLayout,
    vertex: Arc<StageLayout>,
    fragment: Arc<StageLayout>,
    device: ash::Device,
}

impl Drop for LayoutParts {
    fn drop(&mut self) {
        unsafe { self.device.destroy_pipeline_layout(self.independent, None) };
    }
}

/// A pipeline layout as one stage's library sees it: the sets with a binding visible to that
/// stage, the others left out (`VK_NULL_HANDLE`, allowed with independent sets). The libraries
/// made with it keep it as long as they live.
struct StageLayout {
    raw: vk::PipelineLayout,
    key: LayoutKey,
    device: ash::Device,
}

impl Drop for StageLayout {
    fn drop(&mut self) {
        unsafe { self.device.destroy_pipeline_layout(self.raw, None) };
    }
}

/// A stage layout by its content: each set's entries (`None` left out) and the size of the
/// immediates. Two layouts with equal keys are identically defined.
#[derive(Clone, PartialEq, Eq, Hash)]
struct LayoutKey {
    sets: Arc<[Option<Arc<[wgt::BindGroupLayoutEntry]>>]>,
    immediate_size: u32,
}

impl LayoutParts {
    /// The parts of a new pipeline layout, or `None` where its render pipelines are made whole:
    /// a device without libraries, a gap in the groups, or a group no draw stage can see (the
    /// union of the libraries' layouts would then differ from the layout descriptor sets are
    /// bound with).
    pub(super) fn new(
        device: &super::DeviceShared,
        desc: &crate::PipelineLayoutDescriptor<super::BindGroupLayout>,
        immediates: Option<vk::PushConstantRange>,
    ) -> Result<Option<Self>, crate::DeviceError> {
        if !device.private_caps.extras.pipeline_library {
            return Ok(None);
        }
        let draw_stages = wgt::ShaderStages::VERTEX | wgt::ShaderStages::FRAGMENT;
        let mut groups = Vec::with_capacity(desc.bind_group_layouts.len());
        for bgl in desc.bind_group_layouts {
            match bgl {
                Some(bgl)
                    if bgl
                        .entries
                        .iter()
                        .any(|e| e.visibility.intersects(draw_stages)) =>
                {
                    groups.push(*bgl)
                }
                _ => return Ok(None),
            }
        }
        let entries: Vec<Arc<[wgt::BindGroupLayoutEntry]>> =
            groups.iter().map(|g| Arc::from(&g.entries[..])).collect();
        let create = |sets: &[vk::DescriptorSetLayout]| {
            let info = vk::PipelineLayoutCreateInfo::default()
                .flags(vk::PipelineLayoutCreateFlags::INDEPENDENT_SETS_EXT)
                .set_layouts(sets)
                .push_constant_ranges(immediates.as_slice());
            unsafe { device.raw.create_pipeline_layout(&info, None) }
                .map_err(super::map_host_device_oom_err)
        };
        let stage = |visible: wgt::ShaderStages| -> Result<Arc<StageLayout>, crate::DeviceError> {
            let mut sets = Vec::with_capacity(groups.len());
            let mut key = Vec::with_capacity(groups.len());
            for (group, entries) in groups.iter().zip(&entries) {
                if group
                    .entries
                    .iter()
                    .any(|e| e.visibility.intersects(visible))
                {
                    sets.push(group.raw);
                    key.push(Some(Arc::clone(entries)));
                } else {
                    sets.push(vk::DescriptorSetLayout::null());
                    key.push(None);
                }
            }
            Ok(Arc::new(StageLayout {
                raw: create(&sets)?,
                key: LayoutKey {
                    sets: key.into(),
                    immediate_size: desc.immediate_size,
                },
                device: device.raw.clone(),
            }))
        };
        let vertex = stage(wgt::ShaderStages::VERTEX)?;
        let fragment = stage(wgt::ShaderStages::FRAGMENT)?;
        let all: Vec<vk::DescriptorSetLayout> = groups.iter().map(|g| g.raw).collect();
        Ok(Some(Self {
            independent: create(&all)?,
            vertex,
            fragment,
            device: device.raw.clone(),
        }))
    }
}

impl super::PipelineLayout {
    /// The layout pipelines at `bind_point` are made and bound with: for draws the
    /// independent-sets variant where the layout has one, otherwise the published layout.
    pub(super) fn raw_for(&self, bind_point: vk::PipelineBindPoint) -> vk::PipelineLayout {
        match self.libraries {
            Some(ref parts) if bind_point == vk::PipelineBindPoint::GRAPHICS => parts.independent,
            _ => self.raw,
        }
    }
}

// ---------------------------------------------------------------------------
// Dynamic state
// ---------------------------------------------------------------------------

/// One face's stencil state as `vkCmdSetStencilOp`, `vkCmdSetStencilCompareMask` and
/// `vkCmdSetStencilWriteMask` set it.
#[derive(Clone, Copy, Debug, PartialEq)]
struct StencilFace {
    fail: vk::StencilOp,
    pass: vk::StencilOp,
    depth_fail: vk::StencilOp,
    compare: vk::CompareOp,
    compare_mask: u32,
    write_mask: u32,
}

impl StencilFace {
    const OFF: Self = Self {
        fail: vk::StencilOp::KEEP,
        pass: vk::StencilOp::KEEP,
        depth_fail: vk::StencilOp::KEEP,
        compare: vk::CompareOp::NEVER,
        compare_mask: 0,
        write_mask: 0,
    };

    fn new(face: &wgt::StencilFaceState, compare_mask: u32, write_mask: u32) -> Self {
        let op = conv::map_stencil_face(face, compare_mask, write_mask);
        Self {
            fail: op.fail_op,
            pass: op.pass_op,
            depth_fail: op.depth_fail_op,
            compare: op.compare_op,
            compare_mask: op.compare_mask,
            write_mask: op.write_mask,
        }
    }
}

/// The states a linked pipeline leaves dynamic, with the values the published path bakes into
/// a whole pipeline for the same descriptor (DXVK's `DxvkGraphicsPipelineDynamicState` for the
/// cases wgpu has; the stencil reference, blend constants, viewport and scissor are dynamic in
/// every wgpu-hal pipeline already).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct DynamicState {
    cull_mode: vk::CullModeFlags,
    front_face: vk::FrontFace,
    depth_bias_enable: bool,
    /// Constant factor, clamp, slope factor.
    depth_bias: [f32; 3],
    depth_test: bool,
    depth_write: bool,
    depth_compare: vk::CompareOp,
    stencil_test: bool,
    stencil_front: StencilFace,
    stencil_back: StencilFace,
}

impl DynamicState {
    fn new(
        primitive: &wgt::PrimitiveState,
        depth_stencil: Option<&wgt::DepthStencilState>,
    ) -> Self {
        let mut state = Self {
            cull_mode: primitive
                .cull_mode
                .map_or(vk::CullModeFlags::NONE, conv::map_cull_face),
            front_face: conv::map_front_face(primitive.front_face),
            depth_bias_enable: false,
            depth_bias: [0.0; 3],
            depth_test: false,
            depth_write: false,
            depth_compare: vk::CompareOp::NEVER,
            stencil_test: false,
            stencil_front: StencilFace::OFF,
            stencil_back: StencilFace::OFF,
        };
        if let Some(ds) = depth_stencil {
            if ds.is_depth_enabled() {
                state.depth_test = true;
                state.depth_write = ds.depth_write_enabled.unwrap_or_default();
                state.depth_compare = conv::map_comparison(ds.depth_compare.unwrap_or_default());
            }
            if ds.stencil.is_enabled() {
                let s = &ds.stencil;
                state.stencil_test = true;
                state.stencil_front = StencilFace::new(&s.front, s.read_mask, s.write_mask);
                state.stencil_back = StencilFace::new(&s.back, s.read_mask, s.write_mask);
            }
            if ds.bias.is_enabled() {
                state.depth_bias_enable = true;
                state.depth_bias = [ds.bias.constant as f32, ds.bias.clamp, ds.bias.slope_scale];
            }
        }
        state
    }
}

impl super::CommandEncoder {
    /// After a render pipeline is bound: its dynamic states set where they differ from what this
    /// command buffer has (DXVK's `DxvkContext::updateDynamicState`). A whole pipeline sets every
    /// state itself, so after one nothing is assumed.
    pub(super) unsafe fn bind_dynamic_state(&mut self, pipeline: &super::RenderPipeline) {
        let Some(new) = pipeline.dynamic else {
            self.dynamic_state = None;
            return;
        };
        let old = self.dynamic_state.replace(new);
        let changed =
            |part: fn(&DynamicState) -> [u32; 4]| old.is_none_or(|o| part(&o) != part(&new));
        let (raw, cb) = (&self.device.raw, self.active);
        unsafe {
            if changed(|s| [s.cull_mode.as_raw(), s.front_face.as_raw() as u32, 0, 0]) {
                raw.cmd_set_cull_mode(cb, new.cull_mode);
                raw.cmd_set_front_face(cb, new.front_face);
            }
            if changed(|s| {
                let [c, cl, sl] = s.depth_bias;
                [
                    u32::from(s.depth_bias_enable),
                    c.to_bits(),
                    cl.to_bits(),
                    sl.to_bits(),
                ]
            }) {
                raw.cmd_set_depth_bias_enable(cb, new.depth_bias_enable);
                let [constant, clamp, slope] = new.depth_bias;
                raw.cmd_set_depth_bias(cb, constant, clamp, slope);
            }
            if changed(|s| {
                [
                    u32::from(s.depth_test),
                    u32::from(s.depth_write),
                    s.depth_compare.as_raw() as u32,
                    0,
                ]
            }) {
                raw.cmd_set_depth_test_enable(cb, new.depth_test);
                raw.cmd_set_depth_write_enable(cb, new.depth_write);
                raw.cmd_set_depth_compare_op(cb, new.depth_compare);
            }
            if old.is_none_or(|o| {
                (o.stencil_test, o.stencil_front, o.stencil_back)
                    != (new.stencil_test, new.stencil_front, new.stencil_back)
            }) {
                raw.cmd_set_stencil_test_enable(cb, new.stencil_test);
                for (face, s) in [
                    (vk::StencilFaceFlags::FRONT, new.stencil_front),
                    (vk::StencilFaceFlags::BACK, new.stencil_back),
                ] {
                    raw.cmd_set_stencil_op(cb, face, s.fail, s.pass, s.depth_fail, s.compare);
                    raw.cmd_set_stencil_compare_mask(cb, face, s.compare_mask);
                    raw.cmd_set_stencil_write_mask(cb, face, s.write_mask);
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Libraries
// ---------------------------------------------------------------------------

/// One pipeline library; destroyed with its last user.
pub(super) struct Library {
    raw: vk::Pipeline,
    /// The layout a shader library was made with, kept as long as it.
    _layout: Option<Arc<StageLayout>>,
    device: ash::Device,
}

impl Drop for Library {
    fn drop(&mut self) {
        unsafe { self.device.destroy_pipeline(self.raw, None) };
    }
}

/// A pipeline linked from four libraries; shared by the wgpu-hal pipelines that differ only in
/// dynamic state, destroyed with the last of them.
pub(super) struct Linked {
    raw: vk::Pipeline,
    _libraries: [Arc<Library>; 4],
    device: ash::Device,
}

impl Drop for Linked {
    fn drop(&mut self) {
        unsafe { self.device.destroy_pipeline(self.raw, None) };
    }
}

/// The vertex input library's state: bindings, attributes, topology, primitive restart.
#[derive(Clone, PartialEq, Eq, Hash)]
struct VertexInputKey {
    bindings: Vec<(u32, u32, vk::VertexInputRate)>,
    /// Location, binding, format, offset.
    attributes: Vec<(u32, u32, vk::Format, u32)>,
    topology: vk::PrimitiveTopology,
    primitive_restart: bool,
}

/// A shader module as library keys name it: wgpu-hal keeps one either as a `VkShaderModule` or as naga
/// IR to write SPIR-V from per pipeline (`Workarounds::SEPARATE_ENTRY_POINTS`, on every device, and
/// modules with overrides), which gets an id when it is made ([`next_module_id`]).
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub(super) enum ModuleKey {
    /// No shader: a pipeline without a fragment stage.
    None,
    Raw(vk::ShaderModule),
    Ir(u64),
}

/// A new naga-IR module's id ([`ModuleKey::Ir`]): never the same twice in a run.
pub(super) fn next_module_id() -> u64 {
    static NEXT: AtomicU64 = AtomicU64::new(1);
    NEXT.fetch_add(1, Ordering::Relaxed)
}

impl ModuleKey {
    fn of(stage: Option<&crate::ProgrammableStage<'_, super::ShaderModule>>) -> Self {
        match stage.map(|s| s.module) {
            None => Self::None,
            Some(super::ShaderModule::Raw(raw)) => Self::Raw(*raw),
            Some(super::ShaderModule::Intermediate { id, .. }) => Self::Ir(*id),
        }
    }
}

/// A stage's pipeline-overridable constants as a key: by name, sorted, their values' bits.
fn constants_key(
    stage: Option<&crate::ProgrammableStage<'_, super::ShaderModule>>,
) -> Box<[(Box<str>, u64)]> {
    let mut constants: Vec<(Box<str>, u64)> = stage
        .map(|s| {
            s.constants
                .iter()
                .map(|(k, v)| (k.as_str().into(), v.to_bits()))
                .collect()
        })
        .unwrap_or_default();
    constants.sort();
    constants.into()
}

/// A shader library's state: the module, its entry point, the stage's layout, and the static
/// rasterisation state of a pre-rasterisation library.
#[derive(Clone, PartialEq, Eq, Hash)]
struct StageKey {
    module: ModuleKey,
    entry_point: Box<str>,
    /// The pipeline-overridable constants, by name, sorted (their values' bits).
    constants: Box<[(Box<str>, u64)]>,
    layout: LayoutKey,
    polygon_mode: vk::PolygonMode,
    unclipped_depth: bool,
    conservative: bool,
}

/// One colour target of the fragment output library.
#[derive(Clone, PartialEq, Eq, Hash)]
struct ColorKey {
    format: vk::Format,
    write_mask: u32,
    /// Colour op, source and destination factor, then the same for alpha.
    blend: Option<(
        vk::BlendOp,
        vk::BlendFactor,
        vk::BlendFactor,
        vk::BlendOp,
        vk::BlendFactor,
        vk::BlendFactor,
    )>,
}

/// The fragment output library's state.
#[derive(Clone, PartialEq, Eq, Hash)]
struct FragmentOutputKey {
    colors: ArrayVec<Option<ColorKey>, { crate::MAX_COLOR_ATTACHMENTS }>,
    depth_format: vk::Format,
    stencil_format: vk::Format,
    samples: u32,
    sample_mask: u64,
    alpha_to_coverage: bool,
}

/// A library being made or made (`None`: the driver refused it), so that threads asking for one
/// at once wait for one compile (DXVK keeps a lock per library).
type Slot = Arc<OnceLock<Option<Arc<Library>>>>;

/// The device's libraries and linked pipelines, and what making them cost.
#[derive(Default)]
pub(super) struct Cache {
    maps: Mutex<Maps>,
    stats: Stats,
}

#[derive(Default)]
struct Maps {
    vertex_input: FastHashMap<VertexInputKey, Slot>,
    pre_rasterization: FastHashMap<StageKey, Slot>,
    fragment_shader: FastHashMap<StageKey, Slot>,
    fragment_output: FastHashMap<FragmentOutputKey, Slot>,
    /// By the four libraries' addresses (alive while the linked pipeline is).
    linked: FastHashMap<[usize; 4], Weak<Linked>>,
}

/// Counts and times, in nanoseconds ([`super::Device::pipeline_library_report`]).
#[derive(Default)]
struct Stats {
    libraries: [AtomicU64; 4],
    library_ns: [AtomicU64; 4],
    library_max_ns: [AtomicU64; 4],
    /// Pipelines whose four libraries existed: linked only, or found linked.
    link_only: AtomicU64,
    link_only_ns: AtomicU64,
    link_only_max_ns: AtomicU64,
    /// Pipelines that made a library first.
    with_libraries: AtomicU64,
    with_libraries_ns: AtomicU64,
    shared: AtomicU64,
    whole: AtomicU64,
}

const VERTEX_INPUT: usize = 0;
const PRE_RASTERIZATION: usize = 1;
const FRAGMENT_SHADER: usize = 2;
const FRAGMENT_OUTPUT: usize = 3;

fn add_time(total: &AtomicU64, max: &AtomicU64, ns: u64) {
    total.fetch_add(ns, Ordering::Relaxed);
    max.fetch_max(ns, Ordering::Relaxed);
}

impl Cache {
    /// Forgets the shader libraries of a module that is being destroyed (pipelines linked from
    /// them keep them until they go).
    pub(super) fn forget_module(&self, module: ModuleKey) {
        let mut maps = self.maps.lock();
        maps.pre_rasterization.retain(|k, _| k.module != module);
        maps.fragment_shader.retain(|k, _| k.module != module);
    }

    /// Drops everything kept, before the device is destroyed.
    pub(super) fn clear(&self) {
        *self.maps.lock() = Maps::default();
    }

    /// One line on what was made so far.
    pub(super) fn report(&self) -> String {
        let s = &self.stats;
        let ms = |ns: &AtomicU64| ns.load(Ordering::Relaxed) as f64 / 1e6;
        let n = |c: &AtomicU64| c.load(Ordering::Relaxed);
        let names = ["vertex input", "vertex shader", "pixel shader", "output"];
        let mut libraries = String::new();
        for (((name, count), total), longest) in names
            .iter()
            .zip(&s.libraries)
            .zip(&s.library_ns)
            .zip(&s.library_max_ns)
        {
            let _ = write!(
                libraries,
                "{}{} {name} ({:.1} ms, longest {:.2})",
                if libraries.is_empty() { "" } else { ", " },
                n(count),
                ms(total),
                ms(longest),
            );
        }
        format!(
            "pipeline libraries: {libraries}; pipelines linked from libraries that existed: {} \
             ({:.1} ms, longest {:.3} ms), {} of them shared; after making a library: {} \
             ({:.1} ms); made whole: {}",
            n(&s.link_only),
            ms(&s.link_only_ns),
            ms(&s.link_only_max_ns),
            n(&s.shared),
            n(&s.with_libraries),
            ms(&s.with_libraries_ns),
            n(&s.whole),
        )
    }

    /// The library under `key` in `map`, made by `make` if new (on this thread; others asking
    /// for it wait). `made` is set when it was not there yet (made here or waited for).
    fn library<K: Clone + Eq + core::hash::Hash>(
        &self,
        kind: usize,
        map: fn(&mut Maps) -> &mut FastHashMap<K, Slot>,
        key: &K,
        made: &mut bool,
        make: impl FnOnce() -> Option<Library>,
    ) -> Option<Arc<Library>> {
        let slot = {
            let mut maps = self.maps.lock();
            Arc::clone(map(&mut maps).entry(key.clone()).or_default())
        };
        *made |= slot.get().is_none();
        slot.get_or_init(|| {
            let start = Instant::now();
            let library = make().map(Arc::new);
            let ns = start.elapsed().as_nanos() as u64;
            self.stats.libraries[kind].fetch_add(1, Ordering::Relaxed);
            add_time(
                &self.stats.library_ns[kind],
                &self.stats.library_max_ns[kind],
                ns,
            );
            library
        })
        .clone()
    }
}

/// The parts of a render pipeline descriptor the libraries are made from.
type Desc<'a> = crate::RenderPipelineDescriptor<
    'a,
    super::PipelineLayout,
    super::ShaderModule,
    super::PipelineCache,
>;

/// A render pipeline linked from libraries, or `None` where it is made whole (module docs).
pub(super) fn create(device: &super::Device, desc: &Desc<'_>) -> Option<super::RenderPipeline> {
    let shared = &device.shared;
    let parts = desc.layout.libraries.as_ref()?;
    let crate::VertexProcessor::Standard {
        vertex_buffers,
        vertex_stage,
    } = &desc.vertex_processor
    else {
        return None;
    };
    if desc.multiview_mask.is_some() {
        return None;
    }
    let fragment_stage = desc.fragment_stage.as_ref();
    let start = Instant::now();
    let cache_raw = desc.cache.map_or(vk::PipelineCache::null(), |c| c.raw);
    let cache = &shared.pipeline_libraries;
    let mut made = false;

    let vi_key = vertex_input_key(vertex_buffers, &desc.primitive);
    let vi = cache.library(
        VERTEX_INPUT,
        |m| &mut m.vertex_input,
        &vi_key,
        &mut made,
        || make_vertex_input(shared, cache_raw, &vi_key),
    )?;

    let vs_key = StageKey {
        module: ModuleKey::of(Some(vertex_stage)),
        entry_point: vertex_stage.entry_point.into(),
        constants: constants_key(Some(vertex_stage)),
        layout: parts.vertex.key.clone(),
        polygon_mode: conv::map_polygon_mode(desc.primitive.polygon_mode),
        unclipped_depth: desc.primitive.unclipped_depth,
        conservative: desc.primitive.conservative,
    };
    let vs = cache.library(
        PRE_RASTERIZATION,
        |m| &mut m.pre_rasterization,
        &vs_key,
        &mut made,
        || {
            make_pre_rasterization(
                device,
                cache_raw,
                &vs_key,
                vertex_stage,
                desc,
                &parts.vertex,
            )
        },
    )?;

    let fs_key = StageKey {
        module: ModuleKey::of(fragment_stage),
        entry_point: fragment_stage.map_or("", |s| s.entry_point).into(),
        constants: constants_key(fragment_stage),
        layout: parts.fragment.key.clone(),
        polygon_mode: vk::PolygonMode::FILL,
        unclipped_depth: false,
        conservative: false,
    };
    let fs = cache.library(
        FRAGMENT_SHADER,
        |m| &mut m.fragment_shader,
        &fs_key,
        &mut made,
        || make_fragment_shader(device, cache_raw, fragment_stage, desc, &parts.fragment),
    )?;

    let fo_key = fragment_output_key(shared, desc);
    let fo = cache.library(
        FRAGMENT_OUTPUT,
        |m| &mut m.fragment_output,
        &fo_key,
        &mut made,
        || make_fragment_output(shared, cache_raw, &fo_key),
    )?;

    let libraries = [vi, vs, fs, fo];
    let ids = libraries.each_ref().map(|l| Arc::as_ptr(l) as usize);
    let found = cache.maps.lock().linked.get(&ids).and_then(Weak::upgrade);
    let linked = match found {
        Some(linked) => {
            cache.stats.shared.fetch_add(1, Ordering::Relaxed);
            linked
        }
        None => {
            let linked = Arc::new(link(shared, cache_raw, parts.independent, libraries)?);
            let mut maps = cache.maps.lock();
            if maps.linked.len() >= 1024 {
                maps.linked.retain(|_, l| l.strong_count() > 0);
            }
            maps.linked.insert(ids, Arc::downgrade(&linked));
            linked
        }
    };

    let ns = start.elapsed().as_nanos() as u64;
    let s = &cache.stats;
    if made {
        s.with_libraries.fetch_add(1, Ordering::Relaxed);
        s.with_libraries_ns.fetch_add(ns, Ordering::Relaxed);
    } else {
        s.link_only.fetch_add(1, Ordering::Relaxed);
        add_time(&s.link_only_ns, &s.link_only_max_ns, ns);
    }
    device.counters.render_pipelines.add(1);
    Some(super::RenderPipeline {
        raw: linked.raw,
        is_multiview: false,
        dynamic: Some(DynamicState::new(
            &desc.primitive,
            desc.depth_stencil.as_ref(),
        )),
        linked: Some(linked),
    })
}

/// Counts a render pipeline made whole on a device with libraries.
pub(super) fn count_whole(device: &super::DeviceShared) {
    if device.private_caps.extras.pipeline_library {
        device
            .pipeline_libraries
            .stats
            .whole
            .fetch_add(1, Ordering::Relaxed);
    }
}

fn vertex_input_key(
    vertex_buffers: &[Option<crate::VertexBufferLayout<'_>>],
    primitive: &wgt::PrimitiveState,
) -> VertexInputKey {
    // As the published `create_render_pipeline` reads them.
    let mut bindings = Vec::with_capacity(vertex_buffers.len());
    let mut attributes = Vec::new();
    for (i, vb) in vertex_buffers.iter().enumerate() {
        let Some(vb) = vb else {
            continue;
        };
        let rate = match vb.step_mode {
            wgt::VertexStepMode::Vertex => vk::VertexInputRate::VERTEX,
            wgt::VertexStepMode::Instance => vk::VertexInputRate::INSTANCE,
        };
        bindings.push((i as u32, vb.array_stride as u32, rate));
        for at in vb.attributes {
            attributes.push((
                at.shader_location,
                i as u32,
                conv::map_vertex_format(at.format),
                at.offset as u32,
            ));
        }
    }
    VertexInputKey {
        bindings,
        attributes,
        topology: conv::map_topology(primitive.topology),
        primitive_restart: primitive.strip_index_format.is_some(),
    }
}

fn fragment_output_key(device: &super::DeviceShared, desc: &Desc<'_>) -> FragmentOutputKey {
    let caps = &device.private_caps;
    let colors = desc
        .color_targets
        .iter()
        .map(|target| {
            target.as_ref().map(|t| ColorKey {
                format: caps.map_texture_format(t.format),
                write_mask: t.write_mask.bits(),
                blend: t.blend.as_ref().map(|b| {
                    let (color_op, color_src, color_dst) = conv::map_blend_component(&b.color);
                    let (alpha_op, alpha_src, alpha_dst) = conv::map_blend_component(&b.alpha);
                    (
                        color_op, color_src, color_dst, alpha_op, alpha_src, alpha_dst,
                    )
                }),
            })
        })
        .collect();
    let (mut depth_format, mut stencil_format) = (vk::Format::UNDEFINED, vk::Format::UNDEFINED);
    if let Some(ref ds) = desc.depth_stencil {
        let format = caps.map_texture_format(ds.format);
        if ds.format.has_depth_aspect() {
            depth_format = format;
        }
        if ds.format.has_stencil_aspect() {
            stencil_format = format;
        }
    }
    FragmentOutputKey {
        colors,
        depth_format,
        stencil_format,
        samples: desc.multisample.count,
        sample_mask: desc.multisample.mask,
        alpha_to_coverage: desc.multisample.alpha_to_coverage_enabled,
    }
}

/// `vkCreateGraphicsPipelines` for one pipeline; `None` (and a log line) if the driver refuses.
fn create_pipeline(
    device: &super::DeviceShared,
    cache: vk::PipelineCache,
    info: &vk::GraphicsPipelineCreateInfo<'_>,
    what: &str,
) -> Option<vk::Pipeline> {
    match unsafe {
        device
            .raw
            .create_graphics_pipelines(cache, core::slice::from_ref(info), None)
    } {
        Ok(mut raw) => raw.pop(),
        Err((_, e)) => {
            log::warn!("Vulkan pipeline library: the {what} was refused ({e:?})");
            None
        }
    }
}

fn library(
    raw: vk::Pipeline,
    layout: Option<&Arc<StageLayout>>,
    device: &super::DeviceShared,
) -> Library {
    Library {
        raw,
        _layout: layout.cloned(),
        device: device.raw.clone(),
    }
}

/// DXVK's `DxvkGraphicsPipelineVertexInputLibrary`, without dynamic strides (wgpu-hal binds
/// vertex buffers without them).
fn make_vertex_input(
    device: &super::DeviceShared,
    cache: vk::PipelineCache,
    key: &VertexInputKey,
) -> Option<Library> {
    let bindings: Vec<vk::VertexInputBindingDescription> = key
        .bindings
        .iter()
        .map(
            |&(binding, stride, input_rate)| vk::VertexInputBindingDescription {
                binding,
                stride,
                input_rate,
            },
        )
        .collect();
    let attributes: Vec<vk::VertexInputAttributeDescription> = key
        .attributes
        .iter()
        .map(
            |&(location, binding, format, offset)| vk::VertexInputAttributeDescription {
                location,
                binding,
                format,
                offset,
            },
        )
        .collect();
    let vertex_input = vk::PipelineVertexInputStateCreateInfo::default()
        .vertex_binding_descriptions(&bindings)
        .vertex_attribute_descriptions(&attributes);
    let input_assembly = vk::PipelineInputAssemblyStateCreateInfo::default()
        .topology(key.topology)
        .primitive_restart_enable(key.primitive_restart);
    let mut library_info = vk::GraphicsPipelineLibraryCreateInfoEXT::default()
        .flags(vk::GraphicsPipelineLibraryFlagsEXT::VERTEX_INPUT_INTERFACE);
    let info = vk::GraphicsPipelineCreateInfo::default()
        .flags(vk::PipelineCreateFlags::LIBRARY_KHR)
        .vertex_input_state(&vertex_input)
        .input_assembly_state(&input_assembly)
        .base_pipeline_index(-1)
        .push_next(&mut library_info);
    let raw = create_pipeline(device, cache, &info, "vertex input library")?;
    Some(library(raw, None, device))
}

/// The module `compile_stage` wrote for a naga-IR stage, once the library is made from it (a library
/// keeps nothing of the module it was made from).
fn destroy_temporary(device: &super::Device, compiled: &super::device::CompiledStage) {
    if let Some(module) = compiled.temp_raw_module {
        unsafe { device.shared.raw.destroy_shader_module(module, None) };
    }
}

/// DXVK's `DxvkShaderPipelineLibrary::compileVertexShaderPipeline`: viewport and scissor (one
/// each), cull mode, front face and depth bias dynamic; the polygon mode, depth clamp and
/// conservative rasterisation as the pipeline asks (a DXVK pipeline that needs them is made
/// whole; here they are part of the library's key).
fn make_pre_rasterization(
    device: &super::Device,
    cache: vk::PipelineCache,
    key: &StageKey,
    stage: &crate::ProgrammableStage<'_, super::ShaderModule>,
    desc: &Desc<'_>,
    layout: &Arc<StageLayout>,
) -> Option<Library> {
    // The stage as wgpu-hal makes it for a whole pipeline (SPIR-V from naga IR for this layout,
    // or its own module); a stage it cannot make is left to that path, which reports why.
    let compiled = device
        .compile_stage(stage, naga::ShaderStage::Vertex, &desc.layout.binding_map)
        .ok()?;
    let stages = [compiled.create_info];
    let dynamic_states = [
        vk::DynamicState::VIEWPORT,
        vk::DynamicState::SCISSOR,
        vk::DynamicState::DEPTH_BIAS,
        vk::DynamicState::DEPTH_BIAS_ENABLE,
        vk::DynamicState::CULL_MODE,
        vk::DynamicState::FRONT_FACE,
    ];
    let dynamic = vk::PipelineDynamicStateCreateInfo::default().dynamic_states(&dynamic_states);
    let viewport = vk::PipelineViewportStateCreateInfo::default()
        .viewport_count(1)
        .scissor_count(1);
    let mut conservative = vk::PipelineRasterizationConservativeStateCreateInfoEXT::default()
        .conservative_rasterization_mode(vk::ConservativeRasterizationModeEXT::OVERESTIMATE);
    let mut rasterization = vk::PipelineRasterizationStateCreateInfo::default()
        .polygon_mode(key.polygon_mode)
        .depth_clamp_enable(key.unclipped_depth)
        .line_width(1.0);
    if key.conservative {
        rasterization = rasterization.push_next(&mut conservative);
    }
    let mut rendering = vk::PipelineRenderingCreateInfo::default();
    let mut library_info = vk::GraphicsPipelineLibraryCreateInfoEXT::default()
        .flags(vk::GraphicsPipelineLibraryFlagsEXT::PRE_RASTERIZATION_SHADERS);
    let info = vk::GraphicsPipelineCreateInfo::default()
        .flags(vk::PipelineCreateFlags::LIBRARY_KHR)
        .stages(&stages)
        .viewport_state(&viewport)
        .rasterization_state(&rasterization)
        .dynamic_state(&dynamic)
        .layout(layout.raw)
        .base_pipeline_index(-1)
        .push_next(&mut library_info)
        .push_next(&mut rendering);
    let raw = create_pipeline(&device.shared, cache, &info, "vertex shader library");
    destroy_temporary(device, &compiled);
    Some(library(raw?, Some(layout), &device.shared))
}

/// DXVK's `DxvkShaderPipelineLibrary::compileFragmentShaderPipeline`: the depth and stencil tests
/// dynamic (depth bounds stay off, as wgpu never tests them); no multisample state, as no wgpu
/// pipeline here asks for sample shading. A pipeline without a fragment shader gets a library
/// without one.
fn make_fragment_shader(
    device: &super::Device,
    cache: vk::PipelineCache,
    stage: Option<&crate::ProgrammableStage<'_, super::ShaderModule>>,
    desc: &Desc<'_>,
    layout: &Arc<StageLayout>,
) -> Option<Library> {
    let compiled = match stage {
        Some(stage) => Some(
            device
                .compile_stage(stage, naga::ShaderStage::Fragment, &desc.layout.binding_map)
                .ok()?,
        ),
        None => None,
    };
    let stages: ArrayVec<vk::PipelineShaderStageCreateInfo<'_>, 1> =
        compiled.iter().map(|c| c.create_info).collect();
    let dynamic_states = [
        vk::DynamicState::DEPTH_TEST_ENABLE,
        vk::DynamicState::DEPTH_WRITE_ENABLE,
        vk::DynamicState::DEPTH_COMPARE_OP,
        vk::DynamicState::STENCIL_COMPARE_MASK,
        vk::DynamicState::STENCIL_WRITE_MASK,
        vk::DynamicState::STENCIL_REFERENCE,
        vk::DynamicState::STENCIL_TEST_ENABLE,
        vk::DynamicState::STENCIL_OP,
    ];
    let dynamic = vk::PipelineDynamicStateCreateInfo::default().dynamic_states(&dynamic_states);
    let depth_stencil = vk::PipelineDepthStencilStateCreateInfo::default();
    let mut rendering = vk::PipelineRenderingCreateInfo::default();
    let mut library_info = vk::GraphicsPipelineLibraryCreateInfoEXT::default()
        .flags(vk::GraphicsPipelineLibraryFlagsEXT::FRAGMENT_SHADER);
    let info = vk::GraphicsPipelineCreateInfo::default()
        .flags(vk::PipelineCreateFlags::LIBRARY_KHR)
        .stages(&stages)
        .depth_stencil_state(&depth_stencil)
        .dynamic_state(&dynamic)
        .layout(layout.raw)
        .base_pipeline_index(-1)
        .push_next(&mut library_info)
        .push_next(&mut rendering);
    let raw = create_pipeline(&device.shared, cache, &info, "pixel shader library");
    if let Some(ref compiled) = compiled {
        destroy_temporary(device, compiled);
    }
    Some(library(raw?, Some(layout), &device.shared))
}

/// DXVK's `DxvkGraphicsPipelineFragmentOutputLibrary`: the targets' formats, blending and write
/// masks, and the multisample state as the published path sets them; blend constants dynamic.
fn make_fragment_output(
    device: &super::DeviceShared,
    cache: vk::PipelineCache,
    key: &FragmentOutputKey,
) -> Option<Library> {
    let mut formats = ArrayVec::<vk::Format, { crate::MAX_COLOR_ATTACHMENTS }>::new();
    let mut attachments =
        ArrayVec::<vk::PipelineColorBlendAttachmentState, { crate::MAX_COLOR_ATTACHMENTS }>::new();
    for color in &key.colors {
        let Some(color) = color else {
            formats.push(vk::Format::UNDEFINED);
            attachments.push(vk::PipelineColorBlendAttachmentState::default());
            continue;
        };
        formats.push(color.format);
        let mut attachment = vk::PipelineColorBlendAttachmentState::default()
            .color_write_mask(vk::ColorComponentFlags::from_raw(color.write_mask));
        if let Some((color_op, color_src, color_dst, alpha_op, alpha_src, alpha_dst)) = color.blend
        {
            attachment = attachment
                .blend_enable(true)
                .color_blend_op(color_op)
                .src_color_blend_factor(color_src)
                .dst_color_blend_factor(color_dst)
                .alpha_blend_op(alpha_op)
                .src_alpha_blend_factor(alpha_src)
                .dst_alpha_blend_factor(alpha_dst);
        }
        attachments.push(attachment);
    }
    let color_blend = vk::PipelineColorBlendStateCreateInfo::default().attachments(&attachments);
    let sample_mask = [key.sample_mask as u32, (key.sample_mask >> 32) as u32];
    let multisample = vk::PipelineMultisampleStateCreateInfo::default()
        .rasterization_samples(vk::SampleCountFlags::from_raw(key.samples))
        .alpha_to_coverage_enable(key.alpha_to_coverage)
        .sample_mask(&sample_mask);
    let dynamic_states = [vk::DynamicState::BLEND_CONSTANTS];
    let dynamic = vk::PipelineDynamicStateCreateInfo::default().dynamic_states(&dynamic_states);
    let mut rendering = vk::PipelineRenderingCreateInfo::default()
        .color_attachment_formats(&formats)
        .depth_attachment_format(key.depth_format)
        .stencil_attachment_format(key.stencil_format);
    let mut library_info = vk::GraphicsPipelineLibraryCreateInfoEXT::default()
        .flags(vk::GraphicsPipelineLibraryFlagsEXT::FRAGMENT_OUTPUT_INTERFACE);
    let info = vk::GraphicsPipelineCreateInfo::default()
        .flags(vk::PipelineCreateFlags::LIBRARY_KHR)
        .color_blend_state(&color_blend)
        .multisample_state(&multisample)
        .dynamic_state(&dynamic)
        .base_pipeline_index(-1)
        .push_next(&mut library_info)
        .push_next(&mut rendering);
    let raw = create_pipeline(device, cache, &info, "output library")?;
    Some(library(raw, None, device))
}

/// DXVK's `DxvkGraphicsPipeline::createBasePipeline`: the four libraries linked without
/// link-time optimisation, with the layout's independent-sets variant (the union of the
/// libraries' layouts).
fn link(
    device: &super::DeviceShared,
    cache: vk::PipelineCache,
    layout: vk::PipelineLayout,
    libraries: [Arc<Library>; 4],
) -> Option<Linked> {
    let raws = libraries.each_ref().map(|l| l.raw);
    let mut library_info = vk::PipelineLibraryCreateInfoKHR::default().libraries(&raws);
    let info = vk::GraphicsPipelineCreateInfo::default()
        .layout(layout)
        .base_pipeline_index(-1)
        .push_next(&mut library_info);
    let raw = create_pipeline(device, cache, &info, "linked pipeline")?;
    Some(Linked {
        raw,
        _libraries: libraries,
        device: device.raw.clone(),
    })
}

impl fmt::Debug for LayoutParts {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LayoutParts")
            .field("independent", &self.independent)
            .field("vertex", &self.vertex.raw)
            .field("fragment", &self.fragment.raw)
            .finish()
    }
}

impl fmt::Debug for Linked {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Linked").field("raw", &self.raw).finish()
    }
}

impl super::Device {
    /// What pipeline libraries were made so far and what linking cost, in one line; `None` on a
    /// device that makes its render pipelines whole.
    pub fn pipeline_library_report(&self) -> Option<String> {
        let caps = &self.shared.private_caps.extras;
        caps.pipeline_library.then(|| {
            let fast = if caps.fast_linking {
                ""
            } else {
                " (the driver says it links slowly)"
            };
            format!("{}{fast}", self.shared.pipeline_libraries.report())
        })
    }
}
