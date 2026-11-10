//! Set B of the world draw's accessors: what the draw reads that the frame around it changes (the
//! camera, the accumulator's settings, its batch renderers and their pass lists, the bounds). A draw
//! packet would be filled from these ([`super`] has the plan). Each reads what the exe reads, where it
//! reads it.

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

/// The accumulator's camera (`m_pkCamera`, +8): nothing is drawn without one.
#[inline(always)]
pub(crate) fn camera(e: &Engine, acc: Ptr<BSShaderAccumulator>) -> u32 {
    e.get(acc, BSShaderAccumulator::m_pkCamera)
}

/// The render mode (`eRenderMode`).
#[inline(always)]
pub(crate) fn render_mode(e: &Engine, acc: Ptr<BSShaderAccumulator>) -> u32 {
    e.get(acc, BSShaderAccumulator::eRenderMode)
}

/// How many batch renderers the accumulator has (one per drawing thread of the exe).
#[inline(always)]
pub(crate) fn renderer_count(e: &Engine, acc: Ptr<BSShaderAccumulator>) -> u32 {
    e.get(acc, BSShaderAccumulator::uiBatchRendererCount)
}

/// Batch renderer `index`.
#[inline(always)]
pub(crate) fn renderer(
    e: &Engine,
    acc: Ptr<BSShaderAccumulator>,
    index: u32,
) -> Ptr<BSBatchRenderer> {
    Ptr::new(e.global::<u32>(elem_addr(
        acc,
        BSShaderAccumulator::pBatchRenderer,
        index,
        4,
    )))
}

/// Whether the alpha distances take the radius off (+0x31), as the byte the exe passes.
#[inline(always)]
pub(crate) fn subtract_radius(e: &Engine, acc: Ptr<BSShaderAccumulator>) -> u8 {
    e.get(acc, BSShaderAccumulator::ucSubtractRadius)
}

/// Whether the second half copies the water reflection first (+0x38).
#[inline(always)]
pub(crate) fn copy_reflection(e: &Engine, acc: Ptr<BSShaderAccumulator>) -> bool {
    e.get(acc, BSShaderAccumulator::ucCopyReflection) != 0
}

/// Whether the first half splits its pass types at 0x237 with clip planes off for the rest
/// (+0x164).
#[inline(always)]
pub(crate) fn split_clip(e: &Engine, acc: Ptr<BSShaderAccumulator>) -> bool {
    e.get(acc, BSShaderAccumulator::ucSkipType13) != 0
}

/// Whether shadow pass types are drawn through the group-clearing draw (+0x164 or +0x165).
#[inline(always)]
pub(crate) fn shadow_groups(e: &Engine, acc: Ptr<BSShaderAccumulator>) -> bool {
    e.get(acc, BSShaderAccumulator::ucSkipType13) != 0
        || e.get(acc, BSShaderAccumulator::ucShadowGroups) != 0
}

/// Whether the second half draws alpha group 1 (+0x167); the first half draws the other.
#[inline(always)]
pub(crate) fn late_alpha_group(e: &Engine, acc: Ptr<BSShaderAccumulator>) -> bool {
    e.get(acc, BSShaderAccumulator::ucAlphaGroupLate) != 0
}

/// Whether decals are drawn (`bRenderDecals`).
#[inline(always)]
pub(crate) fn render_decals(e: &Engine, acc: Ptr<BSShaderAccumulator>) -> bool {
    e.get(acc, BSShaderAccumulator::bRenderDecals) != 0
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
    e.global::<u32>(list.wrapping_add(8))
}

/// The first sorted group of an alpha group list (its head's item).
#[inline(always)]
pub(crate) fn first_sorted_group(e: &Engine, list: u32) -> u32 {
    let head = e.global::<u32>(list);
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
    e.global::<u32>(holder.wrapping_add(0x10))
}

/// A list's first node (its +0).
#[inline(always)]
pub(crate) fn list_head(e: &Engine, list: u32) -> u32 {
    e.global::<u32>(list)
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

/// The shader mode word (`011f91b8`; 9 skips two property kinds in the alpha pass).
const SHADER_MODE: u32 = 0x011f_91b8;
#[inline(always)]
pub(crate) fn shader_mode(e: &Engine) -> u32 {
    e.global::<u32>(SHADER_MODE)
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
    e.global::<u8>(holder) & 1 != 0
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
    e.get(Ptr::<PassHolder>::new(holder), PassHolder::usCount)
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

/// Word `index` of a bound (the centre's x, y, z, then the radius).
#[inline(always)]
pub(crate) fn bound_word(e: &Engine, bound: u32, index: u32) -> u32 {
    e.global::<u32>(bound.wrapping_add(index.wrapping_mul(4)))
}

/// The camera's view direction (+0x68, +0x74, +0x80), each through an `FLD` and `FSTP` into a
/// local, so quieted, and loaded again.
#[inline(always)]
pub(crate) fn camera_direction(e: &Engine, camera: u32) -> [f64; 3] {
    [0x68u32, 0x74, 0x80]
        .map(|offset| x87_load32(x87_quiet_bits(e.global::<u32>(camera.wrapping_add(offset)))))
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
    e.set(this, BSShaderAccumulator::m_pkCamera, camera);
}

// Typed from 00aa0fe0
/// `NiAccumulator::FinishAccumulating` (Xbox PDB), `thiscall(accumulator)`: the camera cleared.
pub fn finish_accumulating(e: &mut Engine, this: Ptr<BSShaderAccumulator>) {
    e.set(this, BSShaderAccumulator::m_pkCamera, 0);
}

/// The camera direction's words as the exe keeps them in locals (each through `FLD`/`FSTP`, so quieted).
#[inline(always)]
pub(crate) fn camera_direction_words(e: &Engine, camera: u32) -> [u32; 3] {
    [0x68u32, 0x74, 0x80].map(|offset| x87_quiet_bits(e.global::<u32>(camera.wrapping_add(offset))))
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
/// columns of its rotation (2, 1, 0, into a local) and its translate go to [`pass_camera_data`].
pub fn hand_camera(e: &mut Engine, renderer: u32, camera: u32) {
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
