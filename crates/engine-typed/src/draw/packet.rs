//! Set B of the world draw's accessors: what the draw reads that the frame around it changes (the
//! camera, the accumulator's settings, its batch renderers and their pass lists, the bounds). A draw
//! packet would be filled from these ([`super`] has the plan). Each reads what the exe reads, where it
//! reads it.

use crate::shader::elem_addr;
use crate::shader::layout::{BSBatchRenderer, BSShaderAccumulator, PassHolder};
use engine_core::ptr::Ptr;
use engine_core::x87::{x87_load32, x87_quiet_bits};
use engine_core::Engine;

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
    e.global::<u32>(renderer.addr().wrapping_add(group.wrapping_mul(4)).wrapping_add(0x70))
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

/// The camera's view direction (+0x68, +0x74, +0x80), each through an `FLD` and `FSTP` into a
/// local, so quieted, and loaded again.
#[inline(always)]
pub(crate) fn camera_direction(e: &Engine, camera: u32) -> [f64; 3] {
    [0x68u32, 0x74, 0x80]
        .map(|offset| x87_load32(x87_quiet_bits(e.global::<u32>(camera.wrapping_add(offset)))))
}
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
