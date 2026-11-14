//! Set B of the world draw's accessors: what the draw reads that the frame around it changes (the
//! camera, the accumulator's settings, its batch renderers and their pass lists, the bounds). A draw
//! packet would be filled from these ([`super`] has the plan). Each reads what the exe reads, where it
//! reads it.

use super::capture;
use crate::shader::elem_addr;
use crate::shader::layout::{BSBatchRenderer, BSShaderAccumulator, PassHolder};
use engine_core::ptr::Ptr;
use engine_core::typed::leaf;
use engine_core::x87::{x87_load32, x87_quiet_bits};
use engine_core::{args, typed_call, Engine};

/// The renderer as the scene holds it (`011f4748`): the draw starts only with one.
pub(crate) const SCENE_RENDERER: u32 = 0x011f_4748;
/// The renderer singleton (`011f9508`).
pub(crate) const RENDERER: u32 = 0x011f_9508;
/// `NiAVObject`'s empty bound (`011f4288`), what a geometry without a bound is measured by.
const NULL_BOUND: u32 = 0x011f_4288;
/// The byte that makes the pass type range draw set the fog-coloured ambient first (`011f94a6`).
const LIGHTING_FOG_AMBIENT: u32 = 0x011f_94a6;
/// The byte that asks for the render pass tree to be made (`011f9fc1`).
pub(crate) const MAKE_PASS_TREE: u32 = 0x011f_9fc1;
/// The water reflection's source target (`011f9438`).
pub(crate) const REFLECTION_SOURCE: u32 = 0x011f_9438;

/// The renderer as the scene holds it (`011f4748`).
#[inline(always)]
pub(crate) fn scene_renderer(e: &Engine) -> u32 {
    e.global::<u32>(SCENE_RENDERER)
}

/// The render modes' draws (`011f9f40`, `cdecl(accumulator)`): mode `mode`'s entry.
const RENDER_MODE_DRAWS: u32 = 0x011f_9f40;
#[inline(always)]
pub(crate) fn render_mode_draw(e: &Engine, mode: u32) -> u32 {
    e.global::<u32>(RENDER_MODE_DRAWS.wrapping_add(mode.wrapping_mul(4)))
}

/// The accumulator's camera (`m_pkCamera`, +8): nothing is drawn without one. The draw's own state (step 2,
/// piece 3: written as the draw starts and cleared as it ends).
#[inline(always)]
pub(crate) fn camera(e: &Engine, acc: Ptr<BSShaderAccumulator>) -> u32 {
    capture::own_word(e, acc.addr(), BSShaderAccumulator::m_pkCamera.off)
}

/// The render mode (`eRenderMode`).
#[inline(always)]
pub(crate) fn render_mode(e: &Engine, acc: Ptr<BSShaderAccumulator>) -> u32 {
    capture::word(e, acc.addr(), BSShaderAccumulator::eRenderMode.off)
}

/// How many batch renderers the accumulator has (one per drawing thread of the exe).
#[inline(always)]
pub(crate) fn renderer_count(e: &Engine, acc: Ptr<BSShaderAccumulator>) -> u32 {
    capture::word(e, acc.addr(), BSShaderAccumulator::uiBatchRendererCount.off)
}

/// Batch renderer `index`.
#[inline(always)]
pub(crate) fn renderer(
    e: &Engine,
    acc: Ptr<BSShaderAccumulator>,
    index: u32,
) -> Ptr<BSBatchRenderer> {
    Ptr::new(capture::word(
        e,
        acc.addr(),
        BSShaderAccumulator::pBatchRenderer
            .off
            .wrapping_add(index.wrapping_mul(4)),
    ))
}

/// Whether the alpha distances take the radius off (+0x31), as the byte the exe passes.
#[inline(always)]
pub(crate) fn subtract_radius(e: &Engine, acc: Ptr<BSShaderAccumulator>) -> u8 {
    capture::byte(e, acc.addr(), BSShaderAccumulator::ucSubtractRadius.off)
}

/// Whether the second half copies the water reflection first (+0x38; main clears it after the draw: the
/// packet's).
#[inline(always)]
pub(crate) fn copy_reflection(e: &Engine, acc: Ptr<BSShaderAccumulator>) -> bool {
    capture::byte(e, acc.addr(), BSShaderAccumulator::ucCopyReflection.off) != 0
}

/// Whether the first half splits its pass types at 0x237 with clip planes off for the rest
/// (+0x164).
#[inline(always)]
pub(crate) fn split_clip(e: &Engine, acc: Ptr<BSShaderAccumulator>) -> bool {
    capture::byte(e, acc.addr(), BSShaderAccumulator::ucSkipType13.off) != 0
}

/// Whether shadow pass types are drawn through the group-clearing draw (+0x164 or +0x165).
#[inline(always)]
pub(crate) fn shadow_groups(e: &Engine, acc: Ptr<BSShaderAccumulator>) -> bool {
    capture::byte(e, acc.addr(), BSShaderAccumulator::ucSkipType13.off) != 0
        || capture::byte(e, acc.addr(), BSShaderAccumulator::ucShadowGroups.off) != 0
}

/// Whether the second half draws alpha group 1 (+0x167); the first half draws the other.
#[inline(always)]
pub(crate) fn late_alpha_group(e: &Engine, acc: Ptr<BSShaderAccumulator>) -> bool {
    capture::byte(e, acc.addr(), BSShaderAccumulator::ucAlphaGroupLate.off) != 0
}

/// Whether decals are drawn (`bRenderDecals`).
#[inline(always)]
pub(crate) fn render_decals(e: &Engine, acc: Ptr<BSShaderAccumulator>) -> bool {
    capture::byte(e, acc.addr(), BSShaderAccumulator::bRenderDecals.off) != 0
}

/// The accumulator's +0x164 alone (`ucSkipType13`): what `RenderBatches` reads first, from its own getter call, to
/// tell a shadow pass that only has its group cleared.
#[inline(always)]
pub(crate) fn skip_type_13(e: &Engine, acc: Ptr<BSShaderAccumulator>) -> bool {
    capture::byte(e, acc.addr(), BSShaderAccumulator::ucSkipType13.off) != 0
}

/// The accumulator's +0x165 alone (`ucShadowGroups`): what `RenderBatches` reads, from a second getter call, when
/// +0x164 is clear.
#[inline(always)]
pub(crate) fn shadow_groups_set(e: &Engine, acc: Ptr<BSShaderAccumulator>) -> bool {
    capture::byte(e, acc.addr(), BSShaderAccumulator::ucShadowGroups.off) != 0
}

/// The fog-coloured ambient flag of the lighting shaders (`011f94a6`).
#[inline(always)]
pub(crate) fn fog_ambient(e: &Engine) -> bool {
    e.global::<u8>(LIGHTING_FOG_AMBIENT) != 0
}

/// The pass holder of register group `group` (+0x3c array), as a word.
#[inline(always)]
pub(crate) fn group_holder(e: &Engine, renderer: Ptr<BSBatchRenderer>, group: u32) -> u32 {
    e.global::<u32>(elem_addr(renderer, BSBatchRenderer::pGroups, group, 4))
}

/// The alpha holder of alpha group `group` (+0x70 array), as a word.
#[inline(always)]
pub(crate) fn alpha_holder(e: &Engine, renderer: Ptr<BSBatchRenderer>, group: u32) -> u32 {
    e.global::<u32>(
        renderer
            .addr()
            .wrapping_add(group.wrapping_mul(4))
            .wrapping_add(0x70),
    )
}

/// The address of alpha group `group`'s list of sorted groups (+0x78, an `NiTPointerList` of
/// three words a group: head, tail, count).
#[inline(always)]
pub(crate) fn alpha_list(renderer: Ptr<BSBatchRenderer>, group: u32) -> u32 {
    renderer
        .addr()
        .wrapping_add(group.wrapping_mul(0xc))
        .wrapping_add(0x78)
}

/// How many sorted groups an alpha group list holds (its +8).
#[inline(always)]
pub(crate) fn sorted_count(e: &Engine, list: u32) -> u32 {
    super::detach::word(e, list.wrapping_add(8))
}

/// The first sorted group of an alpha group list (its head's item).
#[inline(always)]
pub(crate) fn first_sorted_group(e: &Engine, list: u32) -> u32 {
    let head = super::detach::word(e, list);
    e.global::<u32>(head.wrapping_add(8))
}

/// A sorted group's distance (+0x20), as its word.
#[inline(always)]
pub(crate) fn sorted_distance(e: &Engine, sorted: u32) -> u32 {
    e.global::<u32>(sorted.wrapping_add(0x20))
}

/// A holder's first list node (+0x10), 0 for an empty list.
#[inline(always)]
pub(crate) fn holder_head(e: &Engine, holder: u32) -> u32 {
    super::detach::word(e, holder.wrapping_add(0x10))
}

/// A list's first node (its +0).
#[inline(always)]
pub(crate) fn list_head(e: &Engine, list: u32) -> u32 {
    super::detach::word(e, list)
}

/// A list node's next node (its +0).
#[inline(always)]
pub(crate) fn node_next(e: &Engine, node: u32) -> u32 {
    e.global::<u32>(node)
}

/// A list node's pass (+8).
#[inline(always)]
pub(crate) fn node_pass(e: &Engine, node: u32) -> u32 {
    e.global::<u32>(node.wrapping_add(8))
}

/// A pass's geometry (+0).
#[inline(always)]
pub(crate) fn pass_geometry(e: &Engine, pass: u32) -> u32 {
    e.global::<u32>(pass)
}

/// A pass's type (16 bits at +4).
#[inline(always)]
pub(crate) fn pass_type(e: &Engine, pass: u32) -> u32 {
    u32::from(e.global::<u16>(pass.wrapping_add(4)))
}

/// A geometry's shader property (+0xa8).
#[inline(always)]
pub(crate) fn shader_property(e: &Engine, geometry: u32) -> u32 {
    e.global::<u32>(geometry.wrapping_add(0xa8))
}

/// A shader property's kind (+0x1c) and its flags (+0x20).
#[inline(always)]
pub(crate) fn property_kind(e: &Engine, property: u32) -> u32 {
    e.global::<u32>(property.wrapping_add(0x1c))
}
#[inline(always)]
pub(crate) fn property_flags(e: &Engine, property: u32) -> u32 {
    e.global::<u32>(property.wrapping_add(0x20))
}

/// The shader mode word (`011f91b8`; 9 skips two property kinds in the alpha pass): own state (`own::select_shader_mode`).
const SHADER_MODE: u32 = 0x011f_91b8;
#[inline(always)]
pub(crate) fn shader_mode(e: &Engine) -> u32 {
    capture::own_at::<u32>(e, SHADER_MODE)
}

/// Whether the render pass tree is asked for (`011f9fc1`).
#[inline(always)]
pub(crate) fn pass_tree_asked(e: &Engine) -> bool {
    e.global::<u8>(MAKE_PASS_TREE) != 0
}

/// The water reflection's source target (`011f9438`), 0 for none.
#[inline(always)]
pub(crate) fn reflection_source(e: &Engine) -> u32 {
    e.global::<u32>(REFLECTION_SOURCE)
}

/// The renderer singleton (`011f9508`).
#[inline(always)]
pub(crate) fn renderer_singleton(e: &Engine) -> u32 {
    e.global::<u32>(RENDERER)
}

/// Whether a holder keeps its passes in its own list (bit 0 of its flags).
#[inline(always)]
pub(crate) fn keeps_own(e: &Engine, holder: u32) -> bool {
    super::detach::byte(e, holder) & 1 != 0
}

/// A holder's batch renderer (+8), as a word.
#[inline(always)]
pub(crate) fn holder_renderer(e: &Engine, holder: u32) -> u32 {
    e.get(Ptr::<PassHolder>::new(holder), PassHolder::pRenderer)
        .addr()
}

/// A holder's count of passes (16 bits at +2).
#[inline(always)]
pub(crate) fn holder_count(e: &Engine, holder: u32) -> u16 {
    super::detach::half(e, holder.wrapping_add(PassHolder::usCount.off))
}

/// A geometry's world bound (+0x20), or the empty bound.
#[inline(always)]
pub(crate) fn bound_of(e: &Engine, geometry: u32) -> u32 {
    let bound = e.global::<u32>(geometry.wrapping_add(0x20));
    if bound == 0 {
        NULL_BOUND
    } else {
        bound
    }
}

/// Word `index` of a bound (the centre's x, y, z, then the radius; the packet's).
#[inline(always)]
pub(crate) fn bound_word(e: &Engine, bound: u32, index: u32) -> u32 {
    capture::word(e, bound, index.wrapping_mul(4))
}

/// The camera's view direction (+0x68, +0x74, +0x80), each through an `FLD` and `FSTP` into a
/// local, so quieted, and loaded again.
#[inline(always)]
pub(crate) fn camera_direction(e: &Engine, camera: u32) -> [f64; 3] {
    [0x68u32, 0x74, 0x80].map(|offset| x87_load32(x87_quiet_bits(capture::word(e, camera, offset))))
}

// ---- The packet's own inputs, written as the draw starts and ends. ----

/// The camera the draw last started with (`011f917c`).
const LAST_CAMERA: u32 = 0x011f_917c;

/// Notes the camera the draw starts with (`011f917c`).
#[inline(always)]
pub(crate) fn note_camera(e: &mut Engine, camera: u32) {
    e.set_global(LAST_CAMERA, camera);
}

/// Clears the scene renderer's word +0xc as the second half ends.
#[inline(always)]
pub(crate) fn clear_scene_word(e: &mut Engine) {
    let scene = e.global::<u32>(SCENE_RENDERER);
    e.set_global(scene.wrapping_add(0xc), 0u32);
}

// Typed from 00d47a40
/// `thiscall(accumulator, camera)`, the accumulator's slot 0x8c: the camera into `m_pkCamera` (+8).
pub fn set_camera(e: &mut Engine, this: Ptr<BSShaderAccumulator>, camera: u32) {
    capture::set_word(e, this.addr(), BSShaderAccumulator::m_pkCamera.off, camera);
}

// Typed from 00aa0fe0
/// `NiAccumulator::FinishAccumulating` (Xbox PDB), `thiscall(accumulator)`: the camera cleared.
pub fn finish_accumulating(e: &mut Engine, this: Ptr<BSShaderAccumulator>) {
    capture::set_word(e, this.addr(), BSShaderAccumulator::m_pkCamera.off, 0);
}

/// The camera direction's words as the exe keeps them in locals (each through `FLD`/`FSTP`, so quieted).
#[inline(always)]
pub(crate) fn camera_direction_words(e: &Engine, camera: u32) -> [u32; 3] {
    [0x68u32, 0x74, 0x80].map(|offset| x87_quiet_bits(capture::word(e, camera, offset)))
}

// ---- The camera handed to the scene renderer as the draw starts. ----

/// `NiMatrix3::GetCol` (`00439f50`, `thiscall(matrix, column, out)`), by address.
const MATRIX_COLUMN: u32 = 0x0043_9f50;
/// The render object's check for a named extra (`004a0e10`, `thiscall(object, name, 1)`), the name
/// "SetCameraData" (`010231b8`) and the render object's slot that takes the camera data (0x18c).
const HAS_EXTRA: u32 = 0x004a_0e10;
const SET_CAMERA_DATA_NAME: u32 = 0x0102_31b8;
const SLOT_SET_CAMERA_DATA: u32 = 0x18c;

/// Column `column` of the camera's world rotation (+0x68) into `out` through a local, as the exe's
/// helpers do; answers `out`.
#[inline(always)]
fn rotation_column(e: &mut Engine, camera: u32, column: u32, out: u32) -> u32 {
    e.with_stack(0x0c, |e, local| {
        typed_call!(leaf::FAMILY, 0x0068_15c0, leaf::same_word(e, local.addr()));
        e.call(
            MATRIX_COLUMN,
            &args![camera.wrapping_add(0x68), column, local],
        );
        for word in 0..3u32 {
            let value = e.global::<u32>(local.addr().wrapping_add(4 * word));
            e.set_global(out.wrapping_add(4 * word), value);
        }
    });
    out
}

// Typed from 004a0d10
/// `thiscall(camera)`: the address of its view frustum (`m_kViewFrustum`, Xbox PDB, +0x100).
pub fn camera_frustum(_e: &mut Engine, camera: u32) -> u32 {
    camera.wrapping_add(0x100)
}

// Typed from 0045bbe0
/// `thiscall(camera)`: the address of its port (+0xdc).
pub fn camera_port(_e: &mut Engine, camera: u32) -> u32 {
    camera.wrapping_add(0xdc)
}

// Typed from 0045bb80
/// `thiscall(object)`: the address of its world translate (+0x8c).
pub fn world_translate(_e: &mut Engine, object: u32) -> u32 {
    object.wrapping_add(0x8c)
}

// Typed from 0045bba0
/// `thiscall(object, out)`: column 0 of its world rotation into `out`; answers `out`.
pub fn rotation_column_0(e: &mut Engine, object: u32, out: u32) -> u32 {
    rotation_column(e, object, 0, out)
}

// Typed from 004e9c10
/// `thiscall(object, out)`: column 1 of its world rotation into `out`; answers `out`.
pub fn rotation_column_1(e: &mut Engine, object: u32, out: u32) -> u32 {
    rotation_column(e, object, 1, out)
}

// Typed from 004e9c50
/// `thiscall(object, out)`: column 2 of its world rotation into `out`; answers `out`.
pub fn rotation_column_2(e: &mut Engine, object: u32, out: u32) -> u32 {
    rotation_column(e, object, 2, out)
}

// Typed from 004e9bb0
/// `thiscall(renderer, camera)`: the camera to the scene renderer: its frustum, its port, the three
/// columns of its rotation (2, 1, 0, into a local) and its translate go to [`pass_camera_data`]. While the value
/// packet holds the camera, the addresses point into its image of the camera ([`capture::image`]).
pub fn hand_camera(e: &mut Engine, renderer: u32, camera: u32) {
    let (from, to) = super::cut::CAMERA;
    capture::image(e, camera, from, to, |e, camera| {
        hand_camera_from(e, renderer, camera);
    });
}

/// [`hand_camera`]'s body, with the camera's address (or its image's).
fn hand_camera_from(e: &mut Engine, renderer: u32, camera: u32) {
    e.with_stack(0x24, |e, columns| {
        let columns = columns.addr();
        let frustum = typed_call!(super::FAMILY, 0x004a_0d10, camera_frustum(e, camera));
        let port = typed_call!(super::FAMILY, 0x0045_bbe0, camera_port(e, camera));
        let two = typed_call!(
            super::FAMILY,
            0x004e_9c50,
            rotation_column_2(e, camera, columns)
        );
        let one = typed_call!(
            super::FAMILY,
            0x004e_9c10,
            rotation_column_1(e, camera, columns.wrapping_add(0xc))
        );
        let zero = typed_call!(
            super::FAMILY,
            0x0045_bba0,
            rotation_column_0(e, camera, columns.wrapping_add(0x18))
        );
        let translate = typed_call!(super::FAMILY, 0x0045_bb80, world_translate(e, camera));
        typed_call!(
            super::FAMILY,
            0x004e_9c90,
            pass_camera_data(e, renderer, translate, zero, one, two, port, frustum)
        );
    });
}

// Typed from 004e9c90
/// `thiscall(renderer, translate, column 0, column 1, column 2, port, frustum)`: when the render
/// object has the "SetCameraData" extra, its slot 0x18c with the six words.
#[allow(clippy::too_many_arguments)]
pub fn pass_camera_data(
    e: &mut Engine,
    renderer: u32,
    translate: u32,
    zero: u32,
    one: u32,
    two: u32,
    port: u32,
    frustum: u32,
) {
    if !e
        .call(HAS_EXTRA, &args![renderer, SET_CAMERA_DATA_NAME, 1u32])
        .bool()
    {
        return;
    }
    e.vcall(
        renderer,
        SLOT_SET_CAMERA_DATA,
        &args![translate, zero, one, two, port, frustum],
    );
}

// ---- The geometry paths' reads: the renderer, the drawn geometry and its data. ----

/// The renderer's draw mode (+0x200): 1 and 2 draw immediately.
#[inline(always)]
pub(crate) fn renderer_draw_mode(e: &Engine, renderer: u32) -> u32 {
    e.global::<u32>(renderer.wrapping_add(0x200))
}

/// The renderer's immediate byte (+0x208).
#[inline(always)]
pub(crate) fn renderer_immediate(e: &Engine, renderer: u32) -> u8 {
    e.global::<u8>(renderer.wrapping_add(0x208))
}

/// Whether the renderer skips drawing (its byte +0x6fc).
#[inline(always)]
pub(crate) fn renderer_skips(e: &Engine, renderer: u32) -> bool {
    e.global::<u8>(renderer.wrapping_add(0x6fc)) != 0
}

/// The renderer's Direct3D device (+0x288), read at each use as the exe reads it.
#[inline(always)]
pub(crate) fn renderer_device(e: &Engine, renderer: u32) -> u32 {
    e.global::<u32>(renderer.wrapping_add(0x288))
}

/// The renderer's current property state (+0xc), which `NiGeometry::RenderImmediate` points at the
/// drawn geometry's.
#[inline(always)]
pub(crate) fn current_property_state(e: &Engine, renderer: u32) -> u32 {
    e.global::<u32>(renderer.wrapping_add(0xc))
}

/// A geometry's model data (+0xb8).
#[inline(always)]
pub(crate) fn model_data(e: &Engine, geometry: u32) -> u32 {
    e.global::<u32>(geometry.wrapping_add(0xb8))
}

/// A geometry's skin instance (+0xbc).
#[inline(always)]
pub(crate) fn skin_instance(e: &Engine, geometry: u32) -> u32 {
    e.global::<u32>(geometry.wrapping_add(0xbc))
}

/// A geometry's world transform (`m_kWorld`, +0x68, 13 words; the packet's) copied word by word to `to`.
#[inline(always)]
pub(crate) fn copy_world_transform(e: &mut Engine, geometry: u32, to: u32) {
    let words = capture::words::<13>(e, geometry, 0x68);
    for (i, word) in (0u32..).zip(words) {
        e.set_global(to.wrapping_add(4 * i), word);
    }
}

/// A geometry's world bound (+0x20), or the empty bound, as [`bound_of`].
#[inline(always)]
pub(crate) fn bound_or_empty(e: &Engine, geometry: u32) -> u32 {
    bound_of(e, geometry)
}

/// A data's vertex count (`m_usVertices`, +8).
#[inline(always)]
pub(crate) fn data_vertices(e: &Engine, data: u32) -> u16 {
    e.global::<u16>(data.wrapping_add(8))
}

/// A particle data's active count (+0x50; the packet's).
#[inline(always)]
pub(crate) fn active_particles(e: &Engine, data: u32) -> u16 {
    capture::half(e, data, 0x50)
}

/// A triangle data's triangle count (`m_usTriangles`, +0x40).
#[inline(always)]
pub(crate) fn data_triangles(e: &Engine, data: u32) -> u16 {
    e.global::<u16>(data.wrapping_add(0x40))
}

/// A dynamic strip data's active triangle count (+0x52).
#[inline(always)]
pub(crate) fn active_triangles(e: &Engine, data: u32) -> u16 {
    e.global::<u16>(data.wrapping_add(0x52))
}

/// A strip data's index array (`m_pusStripLists`, +0x4c).
#[inline(always)]
pub(crate) fn strip_lists_of(e: &Engine, data: u32) -> u32 {
    e.global::<u32>(data.wrapping_add(0x4c))
}

/// A particle data's index array (+0x44), 0 for the plain run of points.
#[inline(always)]
pub(crate) fn point_indices(e: &Engine, data: u32) -> u32 {
    e.global::<u32>(data.wrapping_add(0x44))
}

/// The arrays the particles' quads are built from, as the points draw takes them.
pub(crate) struct ParticleArrays {
    /// The positions (`NiPoint3`, +0x20).
    pub positions: u32,
    /// The colours (`NiColorA`, +0x28), or 0.
    pub colors: u32,
    /// The sizes (+0x4c) and their scales (+0x54).
    pub sizes: u32,
    pub scales: u32,
    /// The rotation angles (+0x5c), or 0.
    pub angles: u32,
    /// The extra bytes (+0x64), or 0.
    pub bytes: u32,
}

/// A particle data's arrays ([`ParticleArrays`]).
#[inline(always)]
pub(crate) fn particle_arrays(e: &Engine, data: u32) -> ParticleArrays {
    let word = |offset: u32| e.global::<u32>(data.wrapping_add(offset));
    ParticleArrays {
        sizes: word(0x4c),
        colors: word(0x28),
        scales: word(0x54),
        angles: word(0x5c),
        positions: word(0x20),
        bytes: word(0x64),
    }
}

/// Whether a draw object (the segmented shape) takes its model data whole, as one record (its byte
/// +0xcd).
#[inline(always)]
pub(crate) fn draws_whole(e: &Engine, draw: u32) -> bool {
    e.global::<u8>(draw.wrapping_add(0xcd)) != 0
}

/// A skin instance's root (+0x10), whose world transform (+0x68) the bones are measured from.
#[inline(always)]
pub(crate) fn skin_root(e: &Engine, skin: u32) -> u32 {
    e.global::<u32>(skin.wrapping_add(0x10))
}

/// A skin instance's bone list (+0x14).
#[inline(always)]
pub(crate) fn skin_bones(e: &Engine, skin: u32) -> u32 {
    e.global::<u32>(skin.wrapping_add(0x14))
}

/// Bone `bone` of a bone list: its node, whose world transform (+0x68) the animation wrote.
#[inline(always)]
pub(crate) fn bone_node(e: &Engine, list: u32, bone: u32) -> u32 {
    e.global::<u32>(list.wrapping_add(bone.wrapping_mul(4)))
}

/// An object's world scale (`NiAVObject` +0x98, the packet's), as its word: a bone node's in the bone fill, the
/// rendered geometry's in the pass set-ups.
#[inline(always)]
pub(crate) fn bone_scale(e: &Engine, node: u32) -> u32 {
    capture::word(e, node, 0x98)
}

// ---- The pass begin and end (group B): the current pass, the pass being rendered. ----
//
// The current shader pass (`0126f74c`) is what the pass begin picks (own.rs writes it); the pass
// being rendered (`011f91e0`, its type `011f91e4`) and the render-state cache's stored values are the
// batch renderer's, changed as the frame draws. Not routed: the shader package's tables and the
// passes' own contents (stages, state lists, masks, programs), filled at load and only read here.

/// The current shader pass (`0126f74c`).
pub(crate) const CURRENT_PASS: u32 = 0x0126_f74c;
/// The pass being rendered (`011f91e0`) and its type (`011f91e4`).
const RENDERING_PASS: u32 = 0x011f_91e0;
const RENDERING_PASS_TYPE: u32 = 0x011f_91e4;

/// The current shader pass.
#[inline(always)]
pub(crate) fn current_pass(e: &Engine) -> u32 {
    capture::own_word(e, CURRENT_PASS, 0)
}

/// The current pass's constants holder (+0x5c).
#[inline(always)]
pub(crate) fn pass_constants_holder(e: &Engine) -> u32 {
    e.global::<u32>(current_pass(e).wrapping_add(0x5c))
}

/// The device the current pass's constants go to (its constants holder's +0x20).
#[inline(always)]
pub(crate) fn pass_device(e: &Engine) -> u32 {
    e.global::<u32>(pass_constants_holder(e).wrapping_add(0x20))
}

/// The pass being rendered.
#[inline(always)]
pub(crate) fn rendering_pass(e: &Engine) -> u32 {
    capture::own_word(e, RENDERING_PASS, 0)
}

/// The pass being rendered's geometry (its first word).
#[inline(always)]
pub(crate) fn rendering_geometry(e: &Engine) -> u32 {
    e.global::<u32>(rendering_pass(e))
}

/// The type of the pass being rendered.
#[inline(always)]
pub(crate) fn rendering_pass_type(e: &Engine) -> u32 {
    capture::own_word(e, RENDERING_PASS_TYPE, 0)
}

/// The renderer's render-state cache (+0x8b8).
#[inline(always)]
pub(crate) fn render_state_cache(e: &Engine, renderer: u32) -> u32 {
    e.global::<u32>(renderer.wrapping_add(0x8b8))
}

/// The render-state cache's stored value of `state` (+0x124 + 8 · state).
#[inline(always)]
pub(crate) fn stored_render_state(e: &Engine, cache: u32, state: u32) -> u32 {
    capture::own_word(e, cache, state.wrapping_mul(8).wrapping_add(0x124))
}

/// The shadow scene nodes (`011f91c8`, a word an index).
const SHADOW_SCENE_NODES: u32 = 0x011f_91c8;

/// Shadow scene node `index`'s fog property (+0x134).
#[inline(always)]
pub(crate) fn scene_fog_property(e: &Engine, index: u32) -> u32 {
    let node = e.global::<u32>(SHADOW_SCENE_NODES.wrapping_add(index.wrapping_mul(4)));
    e.global::<u32>(node.wrapping_add(0x134))
}

/// A geometry's property state block's property `index` (a word each: 0 the alpha property, 3 the shader
/// property, 4 the second property the cull mode reads).
#[inline(always)]
pub(crate) fn state_property(e: &Engine, block: u32, index: u32) -> u32 {
    e.global::<u32>(block.wrapping_add(index.wrapping_mul(4)))
}

/// An `NiProperty`'s 16-bit flags (+0x18).
#[inline(always)]
pub(crate) fn ni_property_flags(e: &Engine, property: u32) -> u16 {
    e.global::<u16>(property.wrapping_add(0x18))
}

/// An alpha property's reference (+0x1a).
#[inline(always)]
pub(crate) fn alpha_reference(e: &Engine, property: u32) -> u8 {
    e.global::<u8>(property.wrapping_add(0x1a))
}

/// The pass being rendered's byte +9 (its light count, the packet's).
#[inline(always)]
pub(crate) fn rendering_pass_byte9(e: &Engine, pass: u32) -> u8 {
    capture::byte(e, pass, 9)
}

/// The fog property index byte (`011f91c4`), the scene node whose fog property the passes use.
const FOG_INDEX: u32 = 0x011f_91c4;

#[inline(always)]
pub(crate) fn fog_index(e: &Engine) -> u8 {
    capture::byte(e, FOG_INDEX, 0)
}

/// A fog property's word at `offset` (+0x20, +0x24, +0x28 its colour, +0x2c the near distance, +0x30
/// the far one, +0x60 its power; the packet's).
#[inline(always)]
pub(crate) fn fog_field(e: &Engine, fog: u32, offset: u32) -> u32 {
    capture::word(e, fog, offset)
}

/// The fog colour (`011a9bd0`, four floats): word `index`.
const FOG_COLOUR: u32 = 0x011a_9bd0;

#[inline(always)]
pub(crate) fn fog_colour_word(e: &Engine, index: u32) -> u32 {
    e.global::<u32>(FOG_COLOUR.wrapping_add(index.wrapping_mul(4)))
}

/// A shader property's table of land bytes (+0xc4), which the land toggles index.
#[inline(always)]
pub(crate) fn land_table(e: &Engine, property: u32) -> u32 {
    e.mem.u32(property.wrapping_add(0xc4))
}

/// A lighting property's second flags word (+0x24) and its distance from the camera (+0x34).
#[inline(always)]
pub(crate) fn property_flags_24(e: &Engine, property: u32) -> u32 {
    capture::word(e, property, 0x24)
}
#[inline(always)]
pub(crate) fn property_distance(e: &Engine, property: u32) -> u32 {
    capture::word(e, property, 0x34)
}

/// A shader property's word at `offset` in the range the cut copies (+0x24 to +0x38: its second flags word, alpha,
/// fade and distance), the packet's.
#[inline(always)]
pub(crate) fn property_word(e: &Engine, property: u32, offset: u32) -> u32 {
    capture::word(e, property, offset)
}

// ---- The per-light set-up (`00b70820`) and the passes' lights: what the cut copies for each pass light. ----
//
// A pass's light count (+9) and array (+0xc) and the array's entries; a pass light (`ShadowSceneLight`): its weight
// +0xd0, its +0xd4, its flags +0xf4/+0xf5 and its light +0xf8; that light (`NiLight`): its world block (+0x68 to
// +0x9c, the position +0x8c), its values +0xc4 to +0xec (strength, colours, radius) and, for a light that is not a
// point light, its direction +0xf0 (`cut::take_lights`).

/// A pass's light array (+0xc), the packet's.
#[inline(always)]
pub(crate) fn rendering_pass_lights(e: &Engine, pass: u32) -> u32 {
    capture::word(e, pass, 0xc)
}

/// Entry `index` of a pass's light array, the packet's.
#[inline(always)]
pub(crate) fn pass_light_at(e: &Engine, lights: u32, index: u32) -> u32 {
    capture::word(e, lights, index.wrapping_mul(4))
}

/// A pass light's word at `offset` (+0xd0, +0xd4, +0xf8), the packet's.
#[inline(always)]
pub(crate) fn pass_light_word(e: &Engine, light: u32, offset: u32) -> u32 {
    capture::word(e, light, offset)
}

/// A pass light's flag byte at `offset` (+0xf4 a point light, +0xf5 ambient allowed), the packet's.
#[inline(always)]
pub(crate) fn pass_light_byte(e: &Engine, light: u32, offset: u32) -> u8 {
    capture::byte(e, light, offset)
}

/// A pass light's light's word at `offset` (its position +0x8c, values +0xc4..+0xec, direction +0xf0), the
/// packet's.
#[inline(always)]
pub(crate) fn light_value_word(e: &Engine, data: u32, offset: u32) -> u32 {
    capture::word(e, data, offset)
}

/// The shaders' time values (`011f940c`, four floats) and their start time (`011f94ac`), the packet's.
const GAME_TIMES: u32 = 0x011f_940c;
const START_TIME: u32 = 0x011f_94ac;

/// Word `index` of the shaders' time values, the packet's.
#[inline(always)]
pub(crate) fn time_value(e: &Engine, index: u32) -> u32 {
    capture::word(e, GAME_TIMES, index.wrapping_mul(4))
}

/// The shaders' start time, the packet's.
#[inline(always)]
pub(crate) fn start_time(e: &Engine) -> u32 {
    capture::word(e, START_TIME, 0)
}

// ---- The lighting property's light list (`draw::lights`) and the scene's camera light: what the cut copies for each
// drawn geometry's property (+0x60, its nodes, their lights) and for the first scene node's light (+0xe0). ----

/// A lighting property's light list (+0x60, its first node), the packet's.
#[inline(always)]
pub(crate) fn property_light_list(e: &Engine, property: u32) -> u32 {
    capture::word(e, property, 0x60)
}

/// A light list node's word at `offset` (+0 the next node, +8 its light), the packet's.
#[inline(always)]
pub(crate) fn light_node_word(e: &Engine, node: u32, offset: u32) -> u32 {
    capture::word(e, node, offset)
}

/// A pass light's 16-bit half at `offset` (+0x110, its index: 0xff when it has none), the packet's.
#[inline(always)]
pub(crate) fn pass_light_half(e: &Engine, light: u32, offset: u32) -> u16 {
    capture::half(e, light, offset)
}

/// A pass light's light's byte at `offset` (+0x30, its flags: bit 0 app-culled), the packet's.
#[inline(always)]
pub(crate) fn light_value_byte(e: &Engine, data: u32, offset: u32) -> u8 {
    capture::byte(e, data, offset)
}

/// A scene node's light (+0xe0), the packet's.
#[inline(always)]
pub(crate) fn scene_node_light(e: &Engine, node: u32) -> u32 {
    capture::word(e, node, 0xe0)
}

// ---- Group C: the reads of the shader set-ups the world draw reached as translations. ----
//
// Each reads guest memory today; the drawing thread's piece 2 backs them with the frame's captured
// values (`measure/typed-draw/GROUP-C.md`). Read in place, not routed: a shader's renderer (+0x18, set
// when the shader is made).

/// A world transform's word at `offset` (+0 to +0x20 the rotation's entries, +0x24 to +0x2c the
/// translation, +0x30 the scale), as a set-up reads the geometry's transform it is handed.
#[inline(always)]
pub(crate) fn transform_word(e: &Engine, transform: u32, offset: u32) -> u32 {
    e.global::<u32>(transform.wrapping_add(offset))
}

/// The light of an accumulator's shadow scene node (+0x194, then the node's +0xe0), the packet's (the cut copies both
/// for the drawn accumulator and the shader accumulator).
#[inline(always)]
pub(crate) fn accumulator_lights(e: &Engine, accumulator: u32) -> u32 {
    let node = accumulator_shadow_node(e, accumulator);
    scene_node_light(e, node)
}

/// A shadow scene light's light (+0xf8), none when 0, the packet's.
#[inline(always)]
pub(crate) fn first_light(e: &Engine, lights: u32) -> u32 {
    pass_light_word(e, lights, 0xf8)
}

/// An accumulator's shadow scene node (+0x194), the packet's.
#[inline(always)]
pub(crate) fn accumulator_shadow_node(e: &Engine, accumulator: u32) -> u32 {
    capture::word(e, accumulator, 0x194)
}

/// The byte that turns the point lights on (`011f9ef0`).
#[inline(always)]
pub(crate) fn point_lights_on(e: &Engine) -> u8 {
    e.global::<u8>(0x011f_9ef0)
}

/// The pass being rendered's first scene light (the first word of its light list, +0xc; the packet's, value
/// packet B).
#[inline(always)]
pub(crate) fn rendering_first_light(e: &Engine) -> u32 {
    rendering_light(e, rendering_pass(e), 0)
}

/// A scene light's word at `offset` (+0x10c its shadow map, a rendered texture).
#[inline(always)]
pub(crate) fn scene_light_word(e: &Engine, light: u32, offset: u32) -> u32 {
    e.global::<u32>(light.wrapping_add(offset))
}

/// A geometry data's renderer buffers (`m_pkBuffData`, +0x34).
#[inline(always)]
pub(crate) fn buffer_data(e: &Engine, data: u32) -> u32 {
    e.global::<u32>(data.wrapping_add(0x34))
}

/// A tall grass property's instance data (+0x7c; the packet's, value packet B: the cut copies it).
#[inline(always)]
pub(crate) fn grass_instances(e: &Engine, property: u32) -> u32 {
    capture::word(e, property, 0x7c)
}

/// An instance data's geometry (its first word).
#[inline(always)]
pub(crate) fn grass_geometry(e: &Engine, data: u32) -> u32 {
    e.global::<u32>(data)
}

/// An instance data's count of instances (`u16` +0x10) and how many are active (`u16` +0x12; the packet's).
#[inline(always)]
pub(crate) fn grass_instance_count(e: &Engine, data: u32) -> u16 {
    capture::half(e, data, 0x10)
}
#[inline(always)]
pub(crate) fn grass_active_instances(e: &Engine, data: u32) -> u16 {
    capture::half(e, data, 0x12)
}

/// An instance data's array of 0x10-byte instances (+0x14; the packet's).
#[inline(always)]
pub(crate) fn grass_instance_array(e: &Engine, data: u32) -> u32 {
    capture::word(e, data, 0x14)
}

/// An instance data's triangles of one instance (the `u16` +0x34 of its +8).
#[inline(always)]
pub(crate) fn grass_instance_triangles(e: &Engine, data: u32) -> u16 {
    let shape = e.global::<u32>(data.wrapping_add(8));
    e.global::<u16>(shape.wrapping_add(0x34))
}

/// An instance data's float that scales the wind phase (+0x34), as the word.
#[inline(always)]
pub(crate) fn grass_wind_speed(e: &Engine, data: u32) -> u32 {
    e.global::<u32>(data.wrapping_add(0x34))
}

/// The shader manager's frame values the set-ups read: full bright (`011f91a7`), the game time the
/// shaders animate with (`011f940c`, a float's word), the ambient colour (`011f9ee0`, four words),
/// HDR on (`011f941e`) and its value (`011f9188`), the shader level (`011f91c0`), shadows skipped
/// (`011f9427`), the shadow textures (`011f95e4`), the shader flags' low byte (`011f91d8`), the
/// shadow fade (`011fd890`) and the shadow's four words (`011fd874`, `011fd878`, `011adce0`,
/// `011fd880`).
#[inline(always)]
pub(crate) fn full_bright(e: &Engine) -> u8 {
    e.global::<u8>(0x011f_91a7)
}
#[inline(always)]
pub(crate) fn game_time(e: &Engine) -> u32 {
    time_value(e, 0)
}
#[inline(always)]
pub(crate) fn ambient_word(e: &Engine, index: u32) -> u32 {
    e.global::<u32>(0x011f_9ee0u32.wrapping_add(index.wrapping_mul(4)))
}
#[inline(always)]
pub(crate) fn hdr_on(e: &Engine) -> u8 {
    e.global::<u8>(0x011f_941e)
}
#[inline(always)]
pub(crate) fn hdr_value(e: &Engine) -> u32 {
    e.global::<u32>(0x011f_9188)
}
#[inline(always)]
pub(crate) fn shader_level(e: &Engine) -> i32 {
    e.global::<i32>(0x011f_91c0)
}
#[inline(always)]
pub(crate) fn shadows_skipped(e: &Engine) -> u8 {
    e.global::<u8>(0x011f_9427)
}
#[inline(always)]
pub(crate) fn shadow_textures(e: &Engine) -> u32 {
    e.global::<u32>(0x011f_95e4)
}
#[inline(always)]
pub(crate) fn shader_flags_byte(e: &Engine) -> u8 {
    e.global::<u8>(0x011f_91d8)
}
#[inline(always)]
pub(crate) fn shadow_fade(e: &Engine) -> u32 {
    capture::own_at::<u32>(e, 0x011f_d890)
}
const SHADOW_WORDS: [u32; 4] = [0x011f_d874, 0x011f_d878, 0x011a_dce0, 0x011f_d880];
#[inline(always)]
pub(crate) fn shadow_word(e: &Engine, index: u32) -> u32 {
    capture::own_at::<u32>(e, SHADOW_WORDS[index as usize])
}

/// A geometry data's sub-textures: their count (a byte, +0x6c) and their 0x10-byte entries (+0x68; the packet's).
#[inline(always)]
pub(crate) fn subtexture_count(e: &Engine, data: u32) -> u8 {
    capture::byte(e, data, 0x6c)
}
#[inline(always)]
pub(crate) fn subtextures(e: &Engine, data: u32) -> u32 {
    capture::word(e, data, 0x68)
}

/// A shader property's alpha (+0x28, a float's word; the packet's, value packet B).
#[inline(always)]
pub(crate) fn property_alpha(e: &Engine, property: u32) -> u32 {
    property_word(e, property, 0x28)
}

/// A shader property's falloff word `index` (+0x70 to +0x7c).
#[inline(always)]
pub(crate) fn property_falloff_word(e: &Engine, property: u32, index: u32) -> u32 {
    e.global::<u32>(property.wrapping_add(0x70 + 4 * index))
}

/// A material's colour pointer (+0x34; without one its own colour is at +0x28) and its scale (+0x40, a
/// float's word).
#[inline(always)]
pub(crate) fn material_colour(e: &Engine, material: u32) -> u32 {
    e.global::<u32>(material.wrapping_add(0x34))
}
#[inline(always)]
pub(crate) fn material_scale(e: &Engine, material: u32) -> u32 {
    e.global::<u32>(material.wrapping_add(0x40))
}

/// A colour's word `index` (red, green, blue).
#[inline(always)]
pub(crate) fn colour_word(e: &Engine, colour: u32, index: u32) -> u32 {
    e.global::<u32>(colour.wrapping_add(4 * index))
}

/// The neutral colour (`011f4998`) a material colour is compared with, and the one that then replaces
/// it (`011a9b7c`): word `index` of each.
#[inline(always)]
pub(crate) fn neutral_colour_word(e: &Engine, index: u32) -> u32 {
    e.global::<u32>(0x011f_4998 + 4 * index)
}
#[inline(always)]
pub(crate) fn replacement_colour_word(e: &Engine, index: u32) -> u32 {
    e.global::<u32>(0x011a_9b7c + 4 * index)
}

/// The sun's vector (`011fa2a0`, word `index`): what the no-lighting light fog dots the fog plane with,
/// the sky's eye position less the origin.
#[inline(always)]
pub(crate) fn sun_word(e: &Engine, index: u32) -> u32 {
    capture::own_at::<u32>(e, 0x011f_a2a0 + 4 * index)
}
/// The fog plane (`011f9604`, word `index`), its offset (`011f9610`), and the fog's far distance
/// (`011f9618`).
#[inline(always)]
pub(crate) fn fog_plane_word(e: &Engine, index: u32) -> u32 {
    e.global::<u32>(0x011f_9604 + 4 * index)
}
#[inline(always)]
pub(crate) fn fog_plane_offset(e: &Engine) -> u32 {
    e.global::<u32>(0x011f_9610)
}
#[inline(always)]
pub(crate) fn fog_far(e: &Engine) -> u32 {
    e.global::<u32>(0x011f_9618)
}

/// A texturing property's first map (the first word of its maps, +0x20), a map's texture transform
/// (+0xc), and a texture transform's changed byte (+0x1c) and matrix word `index` (+0x20, nine).
#[inline(always)]
pub(crate) fn first_texture_map(e: &Engine, texturing: u32) -> u32 {
    let maps = e.global::<u32>(texturing.wrapping_add(0x20));
    e.global::<u32>(maps)
}
#[inline(always)]
pub(crate) fn map_transform(e: &Engine, map: u32) -> u32 {
    e.global::<u32>(map.wrapping_add(0xc))
}
#[inline(always)]
pub(crate) fn texture_transform_changed(e: &Engine, transform: u32) -> u8 {
    e.global::<u8>(transform.wrapping_add(0x1c))
}
#[inline(always)]
pub(crate) fn texture_transform_word(e: &Engine, transform: u32, index: u32) -> u32 {
    e.global::<u32>(transform.wrapping_add(0x20 + 4 * index))
}

/// The origin the draw takes positions from (`011f474c`, word `index`): a skinned geometry's identity
/// world transform takes it as its translation, the sky's eye position is the sun less it.
#[inline(always)]
pub(crate) fn origin_word(e: &Engine, index: u32) -> u32 {
    e.global::<u32>(0x011f_474c + 4 * index)
}

/// A sky property's first texture (+0x70) and second (+0x80).
#[inline(always)]
pub(crate) fn sky_first_texture(e: &Engine, property: u32) -> u32 {
    e.global::<u32>(property.wrapping_add(0x70))
}
#[inline(always)]
pub(crate) fn sky_second_texture(e: &Engine, property: u32) -> u32 {
    e.global::<u32>(property.wrapping_add(0x80))
}

/// A sky property's kind (+0x8c), value (+0x84, a float's word), blend index (`u16` +0x88) and colour
/// word `index` (+0x60, four).
#[inline(always)]
pub(crate) fn sky_kind(e: &Engine, property: u32) -> u32 {
    e.global::<u32>(property.wrapping_add(0x8c))
}
#[inline(always)]
pub(crate) fn sky_value(e: &Engine, property: u32) -> u32 {
    e.global::<u32>(property.wrapping_add(0x84))
}
#[inline(always)]
pub(crate) fn sky_blend_index(e: &Engine, property: u32) -> u16 {
    e.global::<u16>(property.wrapping_add(0x88))
}
#[inline(always)]
pub(crate) fn sky_colour_word(e: &Engine, property: u32, index: u32) -> u32 {
    e.global::<u32>(property.wrapping_add(0x60 + 4 * index))
}

/// The sky's frame values: its height (`011ff8a8`), blend value `index` (`011ff8b4`), sky colour word
/// `index` (`011ff8e0`, twelve) and HDR value (`011ad87c`).
#[inline(always)]
pub(crate) fn sky_height(e: &Engine) -> u32 {
    capture::own_at::<u32>(e, 0x011f_f8a8)
}
#[inline(always)]
pub(crate) fn sky_blend_value(e: &Engine, index: u32) -> u32 {
    capture::own_at::<u32>(e, 0x011f_f8b4u32.wrapping_add(index.wrapping_mul(4)))
}
#[inline(always)]
pub(crate) fn sky_colours_word(e: &Engine, index: u32) -> u32 {
    capture::own_at::<u32>(e, 0x011f_f8e0 + 4 * index)
}
#[inline(always)]
pub(crate) fn sky_hdr_value(e: &Engine) -> u32 {
    e.global::<u32>(0x011a_d87c)
}

/// Whether an accumulator skips type 13 (`ucSkipType13`, +0x164), as the sky set-up reads the held
/// one: the sky then moved to the accumulator's camera.
#[inline(always)]
pub(crate) fn skips_type13(e: &Engine, acc: u32) -> u8 {
    // The accumulator's settings are the packet's (`cut::ACC_SETTINGS`), as `skip_type_13` reads them.
    capture::byte(e, acc, BSShaderAccumulator::ucSkipType13.off)
}

/// The scene camera (`011f95d8`).
#[inline(always)]
pub(crate) fn scene_camera(e: &Engine) -> u32 {
    e.global::<u32>(0x011f_95d8)
}

/// An object's world translation word `index` (+0x8c, `m_kWorld`'s translation).
#[inline(always)]
pub(crate) fn world_translation_word(e: &Engine, object: u32, index: u32) -> u32 {
    e.global::<u32>(object.wrapping_add(0x8c + 4 * index))
}

/// A water property's word at `offset`: its colours (+0x88, twelve), its values (+0x6c, +0x70, +0x74,
/// +0x78, +0xb8 to +0xc4, +0xd0, +0xd4 the lights' factor, +0xd8 to +0xe4, +0x100, +0x104, +0x118, +0x120,
/// +0x124), its stencil reference (+0x84), its light count (+0x130) and textures (+0x138, +0x13c the
/// reflection, +0x144, +0x148).
#[inline(always)]
pub(crate) fn water_word(e: &Engine, property: u32, offset: u32) -> u32 {
    e.global::<u32>(property.wrapping_add(offset))
}

/// A water property's byte at `offset`: its switches (+0x60, +0x63, +0x80, +0x81) and shade (+0x82).
#[inline(always)]
pub(crate) fn water_byte(e: &Engine, property: u32, offset: u32) -> u8 {
    e.global::<u8>(property.wrapping_add(offset))
}

/// A water property's light list (+0x128, the first node), its light count (+0x130), a node's light (+8) and the next
/// node (+0): the packet's (the cut copies the list, its nodes and lights).
#[inline(always)]
pub(crate) fn water_lights(e: &Engine, property: u32) -> u32 {
    capture::word(e, property, 0x128)
}
#[inline(always)]
pub(crate) fn water_light_count(e: &Engine, property: u32) -> u32 {
    capture::word(e, property, 0x130)
}
#[inline(always)]
pub(crate) fn light_node_light(e: &Engine, node: u32) -> u32 {
    capture::word(e, node, 8)
}
#[inline(always)]
pub(crate) fn light_node_next(e: &Engine, node: u32) -> u32 {
    capture::word(e, node, 0)
}

/// The default texture (`011f9510`), the water's second rendered texture (`011f943c`), the water
/// system's value (`011ff108`) and eight words (`011ff398`, word `index`).
#[inline(always)]
pub(crate) fn default_texture(e: &Engine) -> u32 {
    e.global::<u32>(0x011f_9510)
}
#[inline(always)]
pub(crate) fn water_rendered_texture(e: &Engine) -> u32 {
    e.global::<u32>(0x011f_943c)
}
#[inline(always)]
pub(crate) fn water_value(e: &Engine) -> u32 {
    capture::own_at::<u32>(e, 0x011f_f108)
}
#[inline(always)]
pub(crate) fn water_extra_word(e: &Engine, index: u32) -> u32 {
    capture::own_at::<u32>(e, 0x011f_f398 + 4 * index)
}

/// The pass being rendered's scene light `index` (its lights, +0xc, a word each). The array's word and its words as the cut copies them
/// (value packet B).
#[inline(always)]
pub(crate) fn rendering_light(e: &Engine, pass: u32, index: u32) -> u32 {
    let lights = capture::word(e, pass, 0xc);
    capture::word(e, lights, index.wrapping_mul(4))
}

/// A leaf property's light value (+0x6c, a float's word).
#[inline(always)]
pub(crate) fn leaf_light_value(e: &Engine, property: u32) -> u32 {
    e.global::<u32>(property.wrapping_add(0x6c))
}

/// A leaf data's word at `offset` (+0xc, +0x10, +0x14, +0x18, +0x1c).
#[inline(always)]
pub(crate) fn leaf_data_word(e: &Engine, data: u32, offset: u32) -> u32 {
    e.global::<u32>(data.wrapping_add(offset))
}

/// The leaf scales (`01200608`, `0120060c`: word `index`).
#[inline(always)]
pub(crate) fn leaf_scale_word(e: &Engine, index: u32) -> u32 {
    capture::own_at::<u32>(e, 0x0120_0608 + 4 * index)
}

/// The HDR scale of the image space (`011f91ac`): its effect 1's (the second of its list, +8) data
/// (+0x6c) word +0xc, a float's word.
#[inline(always)]
pub(crate) fn hdr_effect_scale(e: &Engine) -> u32 {
    let manager = e.global::<u32>(0x011f_91ac);
    let effects = e.global::<u32>(manager.wrapping_add(8));
    let effect = e.global::<u32>(effects.wrapping_add(4));
    let data = e.global::<u32>(effect.wrapping_add(0x6c));
    e.global::<u32>(data.wrapping_add(0xc))
}

/// A lighting property's word at `offset`: its alpha (+0x30), light value (+0x6c), emittance (+0x84 to
/// +0x90), glow (+0xe0), refraction count (+0xe4) and parallax (+0xe8, +0xec); the alpha the packet's ([`property_word`],
/// value packet B).
#[inline(always)]
pub(crate) fn lighting_word(e: &Engine, property: u32, offset: u32) -> u32 {
    // `offset` is a constant at every call: the test folds away.
    let (from, to) = super::cut::PROPERTY;
    if offset >= from && offset < to {
        property_word(e, property, offset)
    } else {
        e.global::<u32>(property.wrapping_add(offset)) // own: main's object read in place (set at load)
    }
}

/// Whether a lighting property has lights (the first word of its light list, +0xb4).
#[inline(always)]
pub(crate) fn lighting_has_lights(e: &Engine, property: u32) -> bool {
    let list = e.global::<u32>(property.wrapping_add(0xb4));
    e.global::<u32>(list) != 0
}

/// A material's word at `offset` (+0x38, the light data's third word).
#[inline(always)]
pub(crate) fn material_word(e: &Engine, material: u32, offset: u32) -> u32 {
    e.global::<u32>(material.wrapping_add(offset))
}

/// The material constants' defaults (`011a9be0`, word `index`).
#[inline(always)]
pub(crate) fn material_default_word(e: &Engine, index: u32) -> u32 {
    e.global::<u32>(0x011a_9be0 + 4 * index)
}

/// The scene's shadow node 0 (`011f91c8`), its camera position (+0x1e4, word `index`) and its sun's
/// colour (its light +0xe0, that light's light +0xf8, the light's colour +0xd4, word `index`: the cut copies the
/// four, so they are the packet's; the node itself, the scene's, is set at load and read in place).
#[inline(always)]
pub(crate) fn scene_shadow_node(e: &Engine) -> u32 {
    e.global::<u32>(0x011f_91c8)
}
#[inline(always)]
pub(crate) fn scene_camera_position(e: &Engine, index: u32) -> u32 {
    capture::word(e, scene_shadow_node(e), 0x1e4 + 4 * index)
}
#[inline(always)]
pub(crate) fn sun_light_colour_word(e: &Engine, index: u32) -> u32 {
    let shadow = scene_node_light(e, scene_shadow_node(e));
    let light = pass_light_word(e, shadow, 0xf8);
    light_value_word(e, light, 0xd4 + 4 * index)
}

/// A pass light's byte that says its clip-plane cache is made (+0x208): the draw's own state (the cut owns the cache
/// per pass light).
#[inline(always)]
pub(crate) fn clip_planes_made(e: &Engine, light: u32) -> u8 {
    let cache = light.wrapping_add(crate::shader::geometry::CLIP_PLANE_CACHE);
    capture::own_at::<u8>(
        e,
        cache.wrapping_add(crate::shader::geometry::CLIP_PLANES_MADE),
    ) // own: a pass light's clip-plane cache, owned at the cut
}

/// A renderer's view and light matrices' word at `offset` (+0x940, +0x980, +0x9c0, sixteen words each).
#[inline(always)]
pub(crate) fn renderer_view_word(e: &Engine, renderer: u32, offset: u32) -> u32 {
    e.global::<u32>(renderer.wrapping_add(offset))
}

/// The light colour's scale (`011f9184`) and the shadow's eight words (`011fd874`, word `index`).
#[inline(always)]
pub(crate) fn light_scale(e: &Engine) -> u32 {
    e.global::<u32>(0x011f_9184)
}
#[inline(always)]
pub(crate) fn shadow_extra_word(e: &Engine, index: u32) -> u32 {
    capture::own_at::<u32>(e, 0x011f_d874 + 4 * index)
}

/// An accumulator's ambient colour (+0x154, word `index`), as shader mode 0xf reads the held one.
#[inline(always)]
pub(crate) fn accumulator_ambient_word(e: &Engine, acc: u32, index: u32) -> u32 {
    e.global::<u32>(acc.wrapping_add(0x154 + 4 * index))
}

/// The fog's near distance (`011f9614`).
#[inline(always)]
pub(crate) fn fog_near(e: &Engine) -> u32 {
    e.global::<u32>(0x011f_9614)
}

/// The scene renderer's Direct3D device (the renderer `011f4748`, its +0x288), read at each use as the
/// exe reads it.
#[inline(always)]
pub(crate) fn scene_renderer_device(e: &Engine) -> u32 {
    renderer_device(e, scene_renderer(e))
}

/// The held accumulator (`011f95f0`, what the exe's getter `00b4f5d0` answers).
#[inline(always)]
pub(crate) fn held_accumulator(e: &Engine) -> u32 {
    capture::own_at::<u32>(e, 0x011f_95f0)
}

/// The shader manager's accumulator (`011f95ec`, what `BSShaderManager::GetAccumulator` `00b4f5c0`
/// answers).
#[inline(always)]
pub(crate) fn shader_accumulator(e: &Engine) -> u32 {
    capture::own_at::<u32>(e, 0x011f_95ec)
}

// ---- Value packet B: the set-ups that hand helpers what the cut copies, and the Group C bulk copies. ----
//
// A set-up that passes a drawn geometry's world transform or bound to helpers by pointer hands them the packet's
// guest-stack image of it ([`with_world`], [`with_drawn`]); a word of the transform is read through [`world_word`];
// the Group C bulk copies read the packet's image of their block. The pass lights' and the time values' accessors
// are the shared ones above (the per-light set-up's section).

/// The cut's camera's world translation word `index` (+0x8c), the packet's (the cut copies the camera from +0x68).
#[inline(always)]
pub(crate) fn camera_translation_word(e: &Engine, camera: u32, index: u32) -> u32 {
    capture::word(e, camera, 0x8c + 4 * index)
}

/// Hands `f` the address of `object`'s world transform (`m_kWorld`, +0x68, 13 words: a drawn geometry's, which the
/// cut copies), for the set-ups that pass the transform to helpers by pointer: while the packet is open, the
/// transform of a guest-stack image of the packet's copy; else the object's own (`object + 0x68`). Off, one relaxed
/// load and a branch (`capture::image`).
#[inline(always)]
pub(crate) fn with_world<R>(
    e: &mut Engine,
    object: u32,
    f: impl FnOnce(&mut Engine, u32) -> R,
) -> R {
    let (from, to) = super::cut::WORLD;
    capture::image(e, object, from, to, |e, base| f(e, base.wrapping_add(from)))
}

/// A drawn geometry's world transform word at `offset` (+0x68 to +0x98: the rotation, the translation +0x8c, the
/// scale +0x98), as the cut copies it.
#[inline(always)]
pub(crate) fn world_word(e: &Engine, object: u32, offset: u32) -> u32 {
    debug_assert!((0x68..0x9c).contains(&offset));
    capture::word(e, object, offset)
}

// The Group C set-ups' bulk copies (value packet B): the tall grass's instance array, the no-lighting sub-textures
// and the SpeedTree leaves' 0x300 bytes, which the cut copies (the drawing agent's part 2). Each copy reads the
// packet's guest-stack image of the block while the packet is open, the block itself otherwise.

/// The tall grass's `count` 0x10-byte instances (the instance data's array, +0x14) copied to `to` (`_memcpy`).
#[inline(always)]
pub(crate) fn copy_grass_instances(e: &mut Engine, to: u32, data: u32, count: u16) {
    let source = grass_instance_array(e, data);
    let size = u32::from(count) << 4;
    capture::image(e, source, 0, size, |e, image| {
        super::own::copy_constants(e, to, image, size);
    });
}

/// Hands `f` a geometry data's `count` 0x10-byte sub-textures (+0x68) and their size: the packet's image while
/// it is open, the array itself otherwise (the caller copies them into its own constants).
#[inline(always)]
pub(crate) fn with_subtextures(
    e: &mut Engine,
    data: u32,
    count: u8,
    f: impl FnOnce(&mut Engine, u32, u32),
) {
    let source = subtextures(e, data);
    let size = u32::from(count) << 4;
    capture::image(e, source, 0, size, |e, image| f(e, image, size));
}

/// The SpeedTree leaf property's slots 0xfc (`00bb3c80`: its leaf data +0x88) and 0x100 (`00bb3c90`: that data's
/// leaves +8, null without data), read here without the calls where the slots hold the exe's.
const SLOT_LEAF_DATA: u32 = 0xfc;
const LEAF_DATA: u32 = 0x00bb_3c80;
const SLOT_LEAVES: u32 = 0x100;
const LEAVES: u32 = 0x00bb_3c90;

/// A leaf property's leaf data (its slot 0xfc): where the slot holds the exe's (`00bb3c80`) its +0x88, the packet's;
/// another target through the vtable.
#[inline(always)]
pub(crate) fn leaf_data(e: &mut Engine, property: u32) -> u32 {
    if crate::slot_target(e, property, SLOT_LEAF_DATA) == LEAF_DATA {
        capture::word(e, property, 0x88)
    } else {
        e.vcall(property, SLOT_LEAF_DATA, &[]).u32()
    }
}

/// A leaf property's leaves (its slot 0x100: where the slot holds the exe's (`00bb3c90`) its leaf data's +8, the
/// packet's, null without data; another target through the vtable), 0x300 bytes copied to `to` (`_memcpy`).
#[inline(always)]
pub(crate) fn copy_leaves(e: &mut Engine, to: u32, property: u32) {
    let source = if crate::slot_target(e, property, SLOT_LEAVES) == LEAVES {
        let data = capture::word(e, property, 0x88);
        if data == 0 {
            0
        } else {
            capture::word(e, data, 8)
        }
    } else {
        e.vcall(property, SLOT_LEAVES, &[]).u32()
    };
    capture::image(e, source, 0, 0x300, |e, image| {
        super::own::copy_constants(e, to, image, 0x300);
    });
}

/// [`with_world`] with the geometry's world bound as well ([`bound_of`]: its +0x20, or the empty bound): `f` gets
/// the transform's address and the bound's, each the packet's guest-stack image while the packet is open, for the
/// set-ups that hand both to helpers by pointer.
#[inline(always)]
pub(crate) fn with_drawn<R>(
    e: &mut Engine,
    geometry: u32,
    f: impl FnOnce(&mut Engine, u32, u32) -> R,
) -> R {
    let bound = bound_of(e, geometry);
    with_world(e, geometry, |e, transform| {
        let (from, to) = super::cut::BOUND;
        capture::image(e, bound, from, to, |e, image| f(e, transform, image))
    })
}

/// The word at `offset` of a block a set-up was handed by [`with_world`] or [`with_drawn`] (the packet's image while
/// the packet is open, the object's own otherwise): a plain read, since the image is the copy.
#[inline(always)]
pub(crate) fn handed_word(e: &Engine, at: u32, offset: u32) -> u32 {
    e.global::<u32>(at.wrapping_add(offset))
}

#[cfg(test)]
mod value_packet_b_tests {
    use super::*;

    /// The words at `at`, `n` of them.
    fn words_at(e: &Engine, at: u32, n: u32) -> Vec<u32> {
        (0..n).map(|k| e.mem.u32(at + 4 * k)).collect()
    }

    /// `with_world` and `with_drawn` hand the object's own transform and bound with no packet open, and with one
    /// open the images of the packet's copies: the words the cut took, whatever guest memory holds since.
    #[test]
    fn the_images_handed_hold_the_packets_copies() {
        let mut e = Engine::new();
        let geometry = e.mem.alloc(0xc4);
        let bound = e.mem.alloc(0x10);
        for k in 0..13u32 {
            e.mem.set_u32(geometry + 0x68 + 4 * k, 0x1000 + k);
        }
        for k in 0..4u32 {
            e.mem.set_u32(bound + 4 * k, 0x2000 + k);
        }
        e.mem.set_u32(geometry + 0x20, bound);
        let handed = with_drawn(&mut e, geometry, |_, t, b| (t, b));
        assert_eq!(handed, (geometry + 0x68, bound));
        assert_eq!(with_world(&mut e, geometry, |_, t| t), geometry + 0x68);
        assert!(capture::begin(&e));
        capture::take(&e, geometry, 0x68, 0x9c);
        capture::take(&e, bound, 0, 0x10);
        for k in 0..13u32 {
            e.mem.set_u32(geometry + 0x68 + 4 * k, 0xdead_0000 + k);
        }
        for k in 0..4u32 {
            e.mem.set_u32(bound + 4 * k, 0xbeef_0000 + k);
        }
        let (transform, bound_words, at) = with_drawn(&mut e, geometry, |e, t, b| {
            (words_at(e, t, 13), words_at(e, b, 4), t)
        });
        assert_eq!(transform, (0..13).map(|k| 0x1000 + k).collect::<Vec<u32>>());
        assert_eq!(
            bound_words,
            (0..4).map(|k| 0x2000 + k).collect::<Vec<u32>>()
        );
        assert_ne!(at, geometry + 0x68);
        let transform = with_world(&mut e, geometry, |e, t| words_at(e, t, 13));
        assert_eq!(transform, (0..13).map(|k| 0x1000 + k).collect::<Vec<u32>>());
        let report = capture::end(&mut e);
        assert_eq!(report.misses, 0);
    }
}
