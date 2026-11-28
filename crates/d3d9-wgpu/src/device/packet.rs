//! The executor's draw packet: stage 0 of drawing without Direct3D 9 (`measure\dx9-removal`,
//! DESIGN.md B.6 and F, PR 0.1).
//!
//! A draw reaches wgpu in two steps. The side that holds the game's state (until the world draw
//! records natively, the Direct3D 9 command stream's: [`Device::draw`] with
//! [`Device::draw_packet`]) turns the state's values into a [`DrawPacket`]: the pipeline block
//! resolved ([`DrawRecord`]: the two shaders, the vertex layout, the samplers and the pipeline key
//! with the draw's own fields), the draw's arguments, and what the pass and the holding of draws
//! need, already in wgpu's terms (viewport, scissor rectangle, stencil reference, blend factor, the
//! colour channels written, whether the depth and blend states let it be held). The executor
//! ([`Device::execute_draw`]) draws a packet: the pipeline from its key through the cache (made and
//! listed on first use), the bind groups through theirs, the register blocks placed in the ring, the
//! vertex and index buffers, the draw, its digest. It reads none of the game's render-state values
//! itself.
//!
//! What the executor reads besides the packet is the block the packet was made against, which from
//! stage 1 the game's packets fill instead of the Direct3D 9 commands: the textures, vertex and
//! index buffers and targets bound (resolved resources), the register files and the few states
//! their blocks' headers carry ([`Device::write_blocks`], [`Device::ffp_constants`]), the sampler
//! states of the stage table ([`Device::sampler_key`]). One reader stays on the Direct3D 9 state
//! until the new lighting is fed from packets (stage 1, PR 1.4): the sun shadows' caster record
//! ([`Device::sun_record`]), which infers a caster from the draw's state as it did.
//!
//! Nothing here changes what a draw gives wgpu: the same calls in the same order (the replay
//! digests across the two builds are alike draw for draw; the frame tests' pictures are the same).

#[cfg(doc)]
use super::Device;
use super::DrawRecord;

/// One draw as the executor draws it ([`Device::execute_draw`]).
pub(super) struct DrawPacket {
    /// The pipeline block resolved, with the key's fields of the draw's own (strides, topology,
    /// index size, filtering) set; the occlusion count's and the sun shadows' are the executor's.
    pub(super) block: DrawRecord,
    /// The primitive type (the game's number, data: [`Device::topology`] made `topology` of it).
    pub(super) kind: u32,
    pub(super) topology: wgpu::PrimitiveTopology,
    pub(super) indexed: bool,
    /// The base vertex, the first index (or vertex) and how many (vertices, not primitives).
    pub(super) base_vertex: i32,
    pub(super) start: u32,
    pub(super) n: u32,
    /// The pass's settings: the viewport (x, y, width, height in whole pixels inside the target,
    /// the depth range clamped to 0..1; `f32` bits), the scissor rectangle (x, y, width, height:
    /// the target's whole area without the scissor test), the stencil reference and the blend
    /// factor (the game's word: [`super::blend_colour`]).
    pub(super) viewport: [u32; 6],
    pub(super) scissor: (u32, u32, u32, u32),
    pub(super) stencil: u32,
    pub(super) blend: u32,
    /// The vertex shaders' half-pixel move for this viewport ([`Device::write_blocks`]).
    pub(super) centre: [f32; 2],
    /// The colour channels the draw writes ([`Device::colour_channels`]).
    pub(super) channels: Option<u32>,
    /// Its depth and blend states allow holding it with the opaque draws ([`Device::may_hold`]),
    /// or after them ([`Device::may_hold_after`]): the states' part of those rules.
    pub(super) opaque: bool,
    pub(super) blended: bool,
    /// A user clip plane is on, for the pipeline list (only worked out while listing).
    pub(super) planes: bool,
    /// The blending and alpha test words for the sun shadows' dump (`NV_SUN_SHADOWS_DUMP`):
    /// alpha blend enable, source and destination blend, alpha test enable.
    pub(super) dump: [u32; 4],
}
