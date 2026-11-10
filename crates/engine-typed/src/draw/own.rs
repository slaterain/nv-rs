//! Sets A and C of the world draw's accessors: the writes that are the drawing's own, one section per
//! condition of the pipeline probe (`measure/smooth-200/PROBE.md`): the pass lists it empties after
//! drawing them, which the next frame's registration fills again (condition 1); the blend, clip and
//! depth state it leaves behind (condition 2); the blocks it frees (condition 4); the one reference
//! count it takes (condition 7). Nothing outside this file touches what a section names. Each accessor
//! does what the exe does there, in its order: nothing is copied, deferred or moved yet.

use crate::shader::layout::{BSBatchRenderer, PassGroup, PassHolder};
use crate::shader::{self, batch, state};
use engine_core::ptr::Ptr;
use engine_core::{args, typed_call, Engine};

// ---- Condition 1: the pass lists the next frame's registration fills again. ----
//
// The draw empties each list it drew (its nodes to the list's spare chain or the global pool) and
// clears the active type lists and counts; the next frame's `RegisterObject` fills the same lists
// from those nodes. A drawing thread needs the filled lists handed over and fresh ones for main
// (or two batch renderers taking turns): everything the draw does to them after drawing is here.

/// `BSBatchRenderer::kActivePassIndexList` cleared (`00b64ec0`, by address).
const ACTIVE_LIST_CLEAR: u32 = 0x00b6_4ec0;
/// `NiTPointerListBase::RemoveAll` (`00b71300`, by address).
const POINTER_LIST_REMOVE_ALL: u32 = 0x00b7_1300;

/// Clears a batch renderer's active pass type list (condition 1).
#[inline(always)]
pub(crate) fn clear_active_list(e: &mut Engine, renderer: u32) {
    let list = renderer.wrapping_add(BSBatchRenderer::kActivePassIndexList.off);
    e.call(ACTIVE_LIST_CLEAR, &args![list]);
}

/// Empties a pass holder (`00b9a490`, the shader family's): its list and its batch renderer's
/// groups (condition 1).
#[inline(always)]
pub(crate) fn reset_holder(e: &mut Engine, holder: u32) {
    typed_call!(
        shader::FAMILY,
        0x00b9_a490,
        batch::clear_holder(e, Ptr::new(holder))
    );
}

/// Empties the five lists of a pass type's group (`00b99a50`, the shader family's) (condition 1).
#[inline(always)]
pub(crate) fn clear_pass_group(e: &mut Engine, group: Ptr<PassGroup>) {
    typed_call!(
        shader::FAMILY,
        0x00b9_9a50,
        batch::clear_group(e, group, 0u32)
    );
}

/// Clears a holder's count of passes (16 bits at +2) (condition 1).
#[inline(always)]
pub(crate) fn clear_holder_count(e: &mut Engine, holder: u32) {
    e.set(Ptr::<PassHolder>::new(holder), PassHolder::usCount, 0);
}

/// Empties an alpha group list (`RemoveAll`) and clears the renderer's current grouped alpha
/// holder of that group (+0x90 array) (condition 1).
#[inline(always)]
pub(crate) fn empty_alpha_list(e: &mut Engine, renderer: Ptr<BSBatchRenderer>, group: u32) {
    let list = super::packet::alpha_list(renderer, group);
    e.call(POINTER_LIST_REMOVE_ALL, &args![list]);
    let current = renderer
        .addr()
        .wrapping_add(group.wrapping_mul(4))
        .wrapping_add(0x90);
    e.set_global(current, 0u32);
}

/// The pass type range a batch renderer takes from its active list (+0x20, +0x24) (condition 1:
/// the draw's cursor into the lists).
#[inline(always)]
pub(crate) fn set_pass_range(e: &mut Engine, renderer: u32, low: u32, high: u32) {
    let renderer = Ptr::<BSBatchRenderer>::new(renderer);
    e.set(renderer, BSBatchRenderer::uiFirstPass, low);
    e.set(renderer, BSBatchRenderer::uiLastPass, high);
}

// ---- Condition 2: blend, clip and depth state the draw leaves behind. ----
//
// The render state cache and the shader-state globals are written by the draw and read by the next
// draw on main. Each call here goes to the shader family's setters (their caches) or the exe's.

/// `BSShaderManager`'s clip planes on or off (`00b98280`, seven words, by address).
const SET_CLIP_PLANES: u32 = 0x00b9_8280;
/// The word the first half sets to 1 around render group 0 (`011a9608`).
const GROUP_ZERO_FLAG: u32 = 0x011a_9608;

/// Alpha blending off (`00b97fa0(0, 0)`, the shader family's) (condition 2).
#[inline(always)]
pub(crate) fn alpha_blend_off(e: &mut Engine) {
    typed_call!(
        shader::FAMILY,
        0x00b9_7fa0,
        state::set_alpha_blend_enable(e, 0u8, 0u32)
    );
}

/// The batch renderer's current pass ended (`00b988e0`, the shader family's): the begun shader's
/// end and its states restored (condition 2).
#[inline(always)]
pub(crate) fn end_pass(e: &mut Engine) {
    typed_call!(shader::FAMILY, 0x00b9_88e0, batch::end_pass(e));
}

/// The clip planes on (`1`) or off (`0`) (condition 2).
#[inline(always)]
pub(crate) fn clip_planes(e: &mut Engine, on: u32) {
    e.call(SET_CLIP_PLANES, &args![on, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32]);
}

/// The word at `011a9608` set to `value`, answering what it held (condition 2: a shader-state
/// global the draw of group 0 reads).
#[inline(always)]
pub(crate) fn swap_group_zero_flag(e: &mut Engine, value: u32) -> u32 {
    let saved = e.global::<u32>(GROUP_ZERO_FLAG);
    e.set_global(GROUP_ZERO_FLAG, value);
    saved
}

// ---- Condition 4: the blocks the draw frees. ----
//
// The draw frees the alpha groups it has drawn (each with its own pass lists) and gives list nodes
// back to the global node pool. A drawing thread must hand these back to main (or free them later).

/// `BSBatchRenderer::GeometryGroup`'s destructor (`00b9a410`) and the memory manager's free
/// (`00aa4060`), both by address.
const GEOMETRY_GROUP_DESTRUCTOR: u32 = 0x00b9_a410;
const MEMORY_MANAGER: u32 = 0x011f_6238;
const MEMORY_FREE: u32 = 0x00aa_4060;

/// Destroys and frees a drawn alpha group (condition 4).
#[inline(always)]
pub(crate) fn free_alpha_group(e: &mut Engine, group: u32) {
    e.call(GEOMETRY_GROUP_DESTRUCTOR, &args![group]);
    e.call(MEMORY_FREE, &args![MEMORY_MANAGER, group]);
}

// ---- Condition 7: the one reference count. ----
//
// The draw holds the accumulator it draws in a counted global (`011f95f0`, [`hold_accumulator`]);
// main releases it when the next draw takes another. With a drawing thread the count must stay
// atomic (it is, through the interlocked imports) and the release must wait for the draw.

/// The counted global that holds the accumulator being drawn (`011f95f0`).
pub(crate) const HELD_ACCUMULATOR: u32 = 0x011f_95f0;
/// The interlocked imports (`00fdf1c0` increment, `00fdf1c4` decrement).
const INTERLOCKED_INCREMENT: u32 = 0x00fd_f1c0;
const INTERLOCKED_DECREMENT: u32 = 0x00fd_f1c4;
/// An `NiRefObject`'s delete slot (4).
const SLOT_DELETE: u32 = 4;

// Typed from 00b54b10
/// `cdecl(object)`: `object` into the counted global `011f95f0` (condition 7): when it is another
/// one, the old one's count goes down (deleted at 0, its slot 4), the new one is stored and, when
/// not null, its count goes up.
pub fn hold_accumulator(e: &mut Engine, object: u32) {
    let old = e.global::<u32>(HELD_ACCUMULATOR);
    if old == object {
        return;
    }
    if old != 0 {
        let left = e
            .call(INTERLOCKED_DECREMENT, &args![old.wrapping_add(4)])
            .u32();
        if left == 0 {
            e.vcall(old, SLOT_DELETE, &[]);
        }
    }
    e.set_global(HELD_ACCUMULATOR, object);
    if object != 0 {
        e.call(INTERLOCKED_INCREMENT, &args![object.wrapping_add(4)]);
    }
}

// ---- Render targets and the render pass tree (main's renderer state; no probe condition). ----

/// The render pass tree's owner (`011fa008`), its assignment (`0066b0d0`, `thiscall(owner, tree)`)
/// and `BSShaderAccumulator::CreateRenderPassTree` (`00b63420`, `cdecl(accumulator)`).
const PASS_TREE_OWNER: u32 = 0x011f_a008;
const PASS_TREE_ASSIGN: u32 = 0x0066_b0d0;
const CREATE_RENDER_PASS_TREE: u32 = 0x00b6_3420;
/// The water reflection's copy (`011f943c`), the texture manager (`011f91a8`), the image-space
/// manager (`011f91ac`).
const REFLECTION_COPY: u32 = 0x011f_943c;
const TEXTURE_MANAGER: u32 = 0x011f_91a8;
const IMAGE_SPACE_MANAGER: u32 = 0x011f_91ac;
/// The renderer's clear (`00b6b790`), the texture manager's borrow (`00b6e110`, five words) and
/// return (`00b6da10`), the image-space effect render (`00b97550`, six words), a rendered texture's
/// target (`00b6b260`) and the render target set (`00b6b8d0`).
const RENDERER_CLEAR: u32 = 0x00b6_b790;
const TEXTURE_BORROW: u32 = 0x00b6_e110;
const TEXTURE_RETURN: u32 = 0x00b6_da10;
const IMAGE_SPACE_RENDER: u32 = 0x00b9_7550;
const RENDER_TARGET_OF: u32 = 0x00b6_b260;
const SET_RENDER_TARGET: u32 = 0x00b6_b8d0;

/// Makes the render pass tree when asked (`011f9fc1`): made for the accumulator, given to its owner,
/// the request cleared.
#[inline(always)]
pub(crate) fn make_pass_tree(e: &mut Engine, accumulator: u32) {
    let tree = e
        .call(CREATE_RENDER_PASS_TREE, &args![accumulator])
        .u32();
    e.call(PASS_TREE_ASSIGN, &args![PASS_TREE_OWNER, tree]);
    e.set_global(super::packet::MAKE_PASS_TREE, 0u8);
}

/// Copies the water reflection's source into a borrowed target (borrowed once, kept at `011f943c`)
/// through image-space effect 0x22 and makes the source's target the render target.
#[inline(always)]
pub(crate) fn copy_reflection(e: &mut Engine) {
    e.call(RENDERER_CLEAR, &[]);
    let mut copy = e.global::<u32>(REFLECTION_COPY);
    if copy == 0 {
        let manager = e.global::<u32>(TEXTURE_MANAGER);
        let renderer = e.global::<u32>(super::packet::RENDERER);
        copy = e
            .call(TEXTURE_BORROW, &args![manager, renderer, 5u32, 0u32, 0u32, 0u32])
            .u32();
        e.set_global(REFLECTION_COPY, copy);
    }
    let source = e.global::<u32>(super::packet::REFLECTION_SOURCE);
    let renderer = e.global::<u32>(super::packet::RENDERER);
    let image_space = e.global::<u32>(IMAGE_SPACE_MANAGER);
    e.call(
        IMAGE_SPACE_RENDER,
        &args![image_space, 0x22u32, renderer, source, copy, 0u32, 1u32],
    );
    let source = e.global::<u32>(super::packet::REFLECTION_SOURCE);
    let target = e.call(RENDER_TARGET_OF, &args![source]).u32();
    e.call(SET_RENDER_TARGET, &args![0u32, target]);
}

/// Gives the borrowed reflection copy back and forgets it.
#[inline(always)]
pub(crate) fn return_reflection(e: &mut Engine) {
    let copy = e.global::<u32>(REFLECTION_COPY);
    let manager = e.global::<u32>(TEXTURE_MANAGER);
    e.call(TEXTURE_RETURN, &args![manager, copy]);
    e.set_global(REFLECTION_COPY, 0u32);
}
