//! An object's base pass and its extra light passes drawn as one draw (`NV_WGPU_FUSE_LIGHTS`, off by
//! default; [`sm::fused`]). With the held draws on ([`Device::may_hold`]), a held opaque draw is a
//! base; a later draw of the same vertices that adds light (one, one) or multiplies the colour
//! (destination colour, zero), tests depth EQUAL or LESSEQUAL without writing it, writes colour
//! only (no alpha), has no alpha test, no stencil and no occlusion count, joins its base instead of
//! being held after the opaque draws. When the held draws are drawn, a base with extra passes is
//! drawn once with a fused pipeline; where one cannot be made, the base and its passes are drawn
//! one by one right after each other (the light passes of other objects that lay between them in
//! the game's order touch other pixels: depth EQUAL on their own surfaces).
//!
//! A see-through or other blended draw that is not such a pass closes every base held before it:
//! light added after a see-through surface was blended over the base must stay added after it.

use super::*;
use crate::sm::fused::{self, Blend, Moved};

/// One pass of a fused group as its own draw had it.
pub(super) struct Pass {
    vs: Arc<Shader>,
    ps: Arc<Shader>,
    inputs: Vec<wgsl::Input>,
    /// Its vertex elements by stream: each one's format, offset and the register its vertex shader
    /// reads it through ([`VertexLink`]).
    attributes: [Vec<wgpu::VertexAttribute>; 8],
    clip: wgsl::Clip,
    /// Each sampler of the pair: its binding (the vertex shader's from 32), dimension, filtering,
    /// view and sampler.
    textures: Vec<((u32, u8, bool), wgpu::TextureView, wgpu::Sampler)>,
    /// The pass's registers' places ([`Device::blocks`]) and its digest.
    blocks: [u32; 4],
    digest: Option<u64>,
}

/// A held base and the light passes that joined it.
pub(super) struct Group {
    base: Pass,
    key: PipeKey,
    topology: wgpu::PrimitiveTopology,
    /// The bases held before the last blended draw that was not a light pass are closed.
    epoch: u64,
    pub(super) extras: Vec<(Pass, Blend, HeldKey)>,
}

/// The fused groups' state of a device.
#[derive(Default)]
pub(super) struct State {
    pub(super) on: bool,
    pub(super) epoch: u64,
    /// The fused pipelines made, by their passes (None: one that could not be made).
    pub(super) pipelines: Map<Vec<u64>, Option<Made>>,
    /// Groups drawn fused, and passes they held (tests).
    pub(super) drawn: (u64, u64),
    /// Groups with light passes drawn pass by pass (no fused pipeline could be made for them).
    pub(super) spilled: u64,
}

/// Where a fused stage's sampler goes: its binding, dimension and filtering, the pass and the
/// pass's sampler.
type Placed = Vec<((u32, u8, bool), usize, usize)>;

/// A fused pipeline made ([`Device::fuse_make`]): with its two sampler group layouts and where the
/// generator put each pass's samplers in the vertex and the pixel stage.
#[derive(Clone)]
pub(super) struct Made {
    pipeline: wgpu::RenderPipeline,
    layouts: [wgpu::BindGroupLayout; 2],
    vertex: Vec<Moved>,
    pixel: Vec<Moved>,
}

/// The fused stage's samplers from where the generator `moved` them: each one's new binding (the
/// vertex shader's from 32), dimension and filtering, its pass and that pass's sampler. None where
/// a pass has no such sampler or a binding runs past the stage's 16.
fn place(passes: &[&Pass], moved: &[Moved], vertex: bool) -> Option<Placed> {
    moved
        .iter()
        .map(|m| {
            let from = if vertex { 32 + m.from } else { m.from };
            let k = passes[m.pass]
                .textures
                .iter()
                .position(|((b, _, _), _, _)| *b == from)?;
            let (_, dim, filterable) = passes[m.pass].textures[k].0;
            let to = if vertex { 32 + m.to } else { m.to };
            (to < if vertex { 64 } else { 32 }).then_some(((to, dim, filterable), m.pass, k))
        })
        .collect()
}

/// A fused group ready to draw: its pipeline and its two sampler groups.
pub(super) struct Ready {
    pub(super) pipeline: wgpu::RenderPipeline,
    pub(super) groups: [wgpu::BindGroup; 2],
}

impl Device {
    /// Whether the draw about to be given is a light pass that may join a held base (module
    /// documentation), and how its colour goes into the target.
    pub(super) fn fuse_blend(&self) -> Option<Blend> {
        let r = &self.st.rs;
        if !self.fuse.on
            || r[rs::ALPHABLENDENABLE] == 0
            || r[rs::ZWRITEENABLE] != 0
            || !matches!(r[rs::ZFUNC], 3 | 4)
            || r[rs::COLORWRITEENABLE] & 0xF != 0x7
            || r[rs::ALPHATESTENABLE] != 0
            || r[rs::STENCILENABLE] != 0
            || r[rs::SEPARATEALPHABLENDENABLE] != 0
            || r[rs::BLENDOP] != 1
            || self.counting.is_some()
        {
            return None;
        }
        match (r[rs::SRCBLEND], r[rs::DESTBLEND]) {
            (2, 2) => Some(Blend::Add),
            (9, 1) | (1, 3) => Some(Blend::Multiply),
            _ => None,
        }
    }

    /// A blended draw that is not a light pass: the bases held before it may take no more passes.
    pub(super) fn fuse_close(&mut self) {
        self.fuse.epoch += 1;
    }

    /// The pass of the draw about to be given (its shaders, link and textures).
    fn fuse_pass(
        &mut self,
        vs: &Arc<Shader>,
        ps: &Arc<Shader>,
        link: &VertexLink,
        samplers: &[((u32, u8, bool), usize)],
        blocks: [u32; 4],
        clip: wgsl::Clip,
    ) -> Pass {
        let mut textures = Vec::new();
        for &((binding, dim, filterable), slot) in samplers {
            let view = match self.st.textures[slot].as_ref() {
                Some(t) if (dim == 1) == t.cube => t.view.clone(),
                _ => self.dummy[dim as usize].clone(),
            };
            let sk = SamplerKey::new(&self.st.samplers[slot], filterable);
            textures.push(((binding, dim, filterable), view, self.sampler(sk)));
        }
        Pass {
            vs: vs.clone(),
            ps: ps.clone(),
            inputs: link.inputs.clone(),
            attributes: link.attributes.clone(),
            clip,
            textures,
            blocks,
            digest: None,
        }
    }

    /// A newly held base (the group `g`, one draw so far): it may take light passes.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn fuse_base(
        &mut self,
        g: usize,
        vs: &Arc<Shader>,
        ps: &Arc<Shader>,
        link: &VertexLink,
        samplers: &[((u32, u8, bool), usize)],
        blocks: [u32; 4],
        key: &PipeKey,
        topology: wgpu::PrimitiveTopology,
    ) {
        if !self.fuse.on || self.held.get(g).map_or(true, |h| h.members.len() != 1) || key.sun {
            return;
        }
        let base = self.fuse_pass(vs, ps, link, samplers, blocks, key.clip);
        let group = Group {
            base,
            key: key.clone(),
            topology,
            epoch: self.fuse.epoch,
            extras: Vec::new(),
        };
        self.held[g].fuse = Some(Box::new(group));
    }

    /// The light pass about to be given joins the held base of its vertices, if there is one that
    /// may take it; answers its group.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn fuse_join(
        &mut self,
        blend: Blend,
        key: &HeldKey,
        vs: &Arc<Shader>,
        ps: &Arc<Shader>,
        link: &VertexLink,
        samplers: &[((u32, u8, bool), usize)],
        blocks: [u32; 4],
        clip: wgsl::Clip,
    ) -> Option<usize> {
        let &g = self.held_geometry.get(&key.geometry.surface())?;
        let epoch = self.fuse.epoch;
        let h = self.held.get(g)?;
        let ok = h.members.len() == 1
            && h.key.geometry == key.geometry
            && h.fuse.as_ref().is_some_and(|f| f.epoch == epoch)
            && key.viewport == h.key.viewport
            && key.scissor == h.key.scissor;
        if !ok {
            return None;
        }
        let pass = self.fuse_pass(vs, ps, link, samplers, blocks, clip);
        let group = self.held[g].fuse.as_mut()?;
        group.extras.push((pass, blend, key.clone()));
        self.held_draws += 1;
        Some(g)
    }

    /// The digest of the light pass that just joined group `g`.
    pub(super) fn fuse_digest(&mut self, g: usize, digest: Option<u64>) {
        if let Some(p) = self
            .held
            .get_mut(g)
            .and_then(|h| h.fuse.as_mut())
            .and_then(|f| f.extras.last_mut())
        {
            p.0.digest = digest;
        }
    }

    /// The fused pipeline and sampler groups of a held group with light passes; None to draw its
    /// passes one by one. The pipeline (and where the generator put each pass's samplers) is made
    /// once for each combination of passes and kept ([`State::pipelines`]); only the sampler groups
    /// are made for each group.
    pub(super) fn fuse_ready(&mut self, h: &Held) -> Option<Ready> {
        let f = h.fuse.as_ref().filter(|f| !f.extras.is_empty())?;
        let passes: Vec<&Pass> = std::iter::once(&f.base)
            .chain(f.extras.iter().map(|(p, _, _)| p))
            .collect();
        let blends: Vec<Blend> = f.extras.iter().map(|(_, b, _)| *b).collect();
        let clamp =
            f.key.colours[0].is_some_and(|(format, _)| !format!("{format:?}").contains("Float"));
        // The vertex elements the passes read, each once (by stream, offset and format), at the
        // fused stage's input locations; each pass's registers fed from the elements its own
        // declaration gives them.
        let mut elements: Vec<(usize, u64, wgpu::VertexFormat)> = Vec::new();
        let mut fed: Vec<Vec<(u32, u32)>> = Vec::new();
        for p in &passes {
            let mut regs = Vec::new();
            for (stream, list) in p.attributes.iter().enumerate() {
                for a in list {
                    let e = (stream, a.offset, a.format);
                    let at = match elements.iter().position(|x| *x == e) {
                        Some(at) => at,
                        None => {
                            elements.push(e);
                            elements.len() - 1
                        }
                    };
                    regs.push((a.shader_location, at as u32));
                }
            }
            fed.push(regs);
        }
        // Everything the pipeline is made from: the base's pipeline key, each pass's shaders,
        // inputs, clip code and blend, and the elements.
        let cache_key: Vec<u64> = {
            use std::hash::{Hash, Hasher};
            let mut hasher = std::collections::hash_map::DefaultHasher::new();
            (clamp, &f.key, &elements, &fed).hash(&mut hasher);
            let mut key = vec![hasher.finish()];
            for (pass, b) in passes
                .iter()
                .zip(std::iter::once(None).chain(blends.iter().map(Some)))
            {
                let mut hasher = std::collections::hash_map::DefaultHasher::new();
                (&pass.inputs, pass.clip).hash(&mut hasher);
                key.extend([
                    pass.vs.id,
                    pass.ps.id,
                    b.map_or(0, |b| *b as u64 + 1),
                    hasher.finish(),
                ]);
            }
            key
        };
        let made = match self.fuse.pipelines.get(&cache_key) {
            Some(m) => m.clone(),
            None => {
                let made = self.fuse_make(f, &passes, &blends, clamp, &elements, &fed);
                self.fuse.pipelines.insert(cache_key, made.clone());
                made
            }
        };
        let made = made?;
        let vertex_samplers = place(&passes, &made.vertex, true)?;
        let pixel_samplers = place(&passes, &made.pixel, false)?;
        let group = |device: &wgpu::Device,
                     layout: &wgpu::BindGroupLayout,
                     set: &[((u32, u8, bool), usize, usize)]| {
            let mut entries = Vec::new();
            for ((to, _, _), pass, k) in set {
                let (_, view, sampler) = &passes[*pass].textures[*k];
                entries.push(wgpu::BindGroupEntry {
                    binding: to % 32,
                    resource: wgpu::BindingResource::TextureView(view),
                });
                entries.push(wgpu::BindGroupEntry {
                    binding: to % 32 + 1,
                    resource: wgpu::BindingResource::Sampler(sampler),
                });
            }
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("fused lamp passes"),
                layout,
                entries: &entries,
            })
        };
        let groups = [
            group(&self.device, &made.layouts[0], &pixel_samplers),
            group(&self.device, &made.layouts[1], &vertex_samplers),
        ];
        Some(Ready {
            pipeline: made.pipeline,
            groups,
        })
    }

    /// Generates the fused modules of `passes` and makes their pipeline ([`Device::fuse_pipeline`]);
    /// None where the generator refuses them or the pipeline cannot be made.
    /// `NV_WGPU_FUSE_DUMP=DIR` (a diagnostic): each one's modules written there, with each pass's
    /// own.
    fn fuse_make(
        &mut self,
        f: &Group,
        passes: &[&Pass],
        blends: &[Blend],
        clamp: bool,
        elements: &[(usize, u64, wgpu::VertexFormat)],
        fed: &[Vec<(u32, u32)>],
    ) -> Option<Made> {
        let vertex_links: Vec<wgsl::Link> = passes
            .iter()
            .map(|p| wgsl::Link {
                inputs: p.inputs.clone(),
                centroid: wgsl::centroid_everywhere(p.ps.varyings.len()),
                varyings: p.ps.varyings.clone(),
                clip: p.clip,
                count: false,
                fixed: false,
                storage: true,
            })
            .collect();
        let pixel_links: Vec<wgsl::Link> = vertex_links
            .iter()
            .map(|l| wgsl::Link {
                inputs: Vec::new(),
                ..l.clone()
            })
            .collect();
        let vs: Vec<(&sm::Program, &wgsl::Link)> = passes
            .iter()
            .zip(&vertex_links)
            .map(|(p, l)| (&p.vs.program, l))
            .collect();
        let ps: Vec<(&sm::Program, &wgsl::Link)> = passes
            .iter()
            .zip(&pixel_links)
            .map(|(p, l)| (&p.ps.program, l))
            .collect();
        let v = fused::fused_vertex(&vs, fed).ok()?;
        let p = fused::fused_pixel(&ps, blends, clamp).ok()?;
        if let Some(dir) = std::env::var_os("NV_WGPU_FUSE_DUMP") {
            let dir = std::path::PathBuf::from(dir);
            let n = self.fuse.pipelines.len();
            let _ = std::fs::create_dir_all(&dir);
            let _ = std::fs::write(dir.join(format!("{n}-vertex.wgsl")), &v.source);
            let _ = std::fs::write(dir.join(format!("{n}-pixel.wgsl")), &p.source);
            for (k, ((vp, vl), (pp, pl))) in vs.iter().zip(&ps).enumerate() {
                let one = |p: &sm::Program, l: &wgsl::Link| {
                    wgsl::convert(p, l).unwrap_or_else(|e| format!("// {e:?}"))
                };
                let _ = std::fs::write(dir.join(format!("{n}-pass{k}-vertex.wgsl")), one(vp, vl));
                let _ = std::fs::write(dir.join(format!("{n}-pass{k}-pixel.wgsl")), one(pp, pl));
            }
            let textures: Vec<Vec<(u32, u8, bool)>> = passes
                .iter()
                .map(|p| p.textures.iter().map(|t| t.0).collect())
                .collect();
            let _ = std::fs::write(
                dir.join(format!("{n}-info.txt")),
                format!(
                    "blends {blends:?}\nclamp {clamp}\nelements {elements:?}\nfed {fed:?}\n\
                     vertex bindings {:?}\npixel bindings {:?}\ntextures {textures:?}\n",
                    v.bindings, p.bindings
                ),
            );
        }
        // The fused stages' samplers, the vertex shader's first as the device orders a pair's.
        let list: Vec<(u32, u8, bool)> = place(passes, &v.bindings, true)?
            .iter()
            .chain(&place(passes, &p.bindings, false)?)
            .map(|(k, _, _)| *k)
            .collect();
        if elements.len() > self.device.limits().max_vertex_attributes as usize {
            return None;
        }
        let mut attributes: [Vec<wgpu::VertexAttribute>; 8] = Default::default();
        for (at, &(stream, offset, format)) in elements.iter().enumerate() {
            attributes[stream].push(wgpu::VertexAttribute {
                format,
                offset,
                shader_location: at as u32,
            });
        }
        let (pipeline, layouts) =
            self.fuse_pipeline(f, &v.source, &p.source, &list, &attributes)?;
        Some(Made {
            pipeline,
            layouts,
            vertex: v.bindings,
            pixel: p.bindings,
        })
    }

    /// Makes a fused pipeline: the base's states with the fused modules, samplers and vertex
    /// elements ([`Device::fuse_ready`]).
    fn fuse_pipeline(
        &mut self,
        f: &Group,
        vertex: &str,
        pixel: &str,
        list: &[(u32, u8, bool)],
        attributes: &[Vec<wgpu::VertexAttribute>; 8],
    ) -> Option<(wgpu::RenderPipeline, [wgpu::BindGroupLayout; 2])> {
        let vm = create_module(&self.device, vertex.to_string()).ok()?;
        let pm = create_module(&self.device, pixel.to_string()).ok()?;
        // Under keys of their own (the top bit set), so the device's job maker finds them.
        let id = FUSED_ID | (self.fuse.pipelines.len() as u64);
        let vkey: VKey = (id, Vec::new(), Vec::new(), wgsl::Clip::Off, false, true);
        let pkey: PKey = (id, Vec::new(), false, wgsl::Clip::Off, false, true);
        self.vs_modules.insert(vkey.clone(), Some(vm));
        self.ps_modules.insert(pkey.clone(), Some(pm));
        let job = self.pipeline_job(
            &f.key, &f.base.ps, &vkey, &pkey, attributes, f.topology, list,
        );
        self.vs_modules.remove(&vkey);
        self.ps_modules.remove(&pkey);
        let job = job?;
        let made = build_pipeline(
            &self.device,
            self.pipeline_cache.as_ref().map(|c| &c.cache),
            &job,
        )
        .ok()?;
        Some((made, job.group_layouts))
    }
}

/// The top bit of a fused pipeline's module keys: no shader's id has it.
const FUSED_ID: u64 = 1 << 63;

/// The table entries of a fused group in pass order (the base's, then each pass's), and the digests
/// in the same order.
pub(super) fn entries(f: &Group) -> (Vec<[u32; 4]>, Vec<Option<u64>>) {
    let mut blocks = vec![f.base.blocks];
    let mut digests = vec![f.base.digest];
    for (p, _, _) in &f.extras {
        blocks.push(p.blocks);
        digests.push(p.digest);
    }
    (blocks, digests)
}

/// A fused group's light passes as held draws of their own, for drawing one by one after the base.
pub(super) fn one_by_one(f: &Group) -> Vec<Held> {
    f.extras
        .iter()
        .map(|(p, _, key)| Held {
            key: key.clone(),
            members: vec![Member {
                blocks: p.blocks,
                digest: p.digest,
                state: None,
                streams: Vec::new(),
                sun: false,
            }],
            fuse: None,
        })
        .collect()
}
