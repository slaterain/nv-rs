//! Sets A and C of the world draw's accessors: the writes that are the drawing's own, one section per
//! condition of the pipeline probe (`measure/smooth-200/PROBE.md`): the pass lists it empties after
//! drawing them, which the next frame's registration fills again (condition 1); the blend, clip and
//! depth state it leaves behind (condition 2); the blocks it frees (condition 4); the one reference
//! count it takes (condition 7). Nothing outside this file touches what a section names. Each accessor
//! does what the exe does there, in its order; the pass lists' words go through the drawing thread's view
//! ([`super::detach`]), which holds them while the cut's draw runs with `NV_DRAW_DETACH=1` (a check).

use crate::shader::layout::{BSBatchRenderer, BSShaderAccumulator, PassGroup, PassHolder};
use crate::shader::{self, batch, state};
use engine_core::ptr::Ptr;
use engine_core::x87::{x87_quiet_bits, x87_store32};
use engine_core::{args, typed_call, Engine};

// ---- Condition 1: the pass lists the next frame's registration fills again. ----
//
// The draw empties each list it drew (its nodes to the list's spare chain or the global pool) and
// clears the active type lists and counts; the next frame's `RegisterObject` fills the same lists
// from those nodes. A drawing thread needs the filled lists handed over and fresh ones for main: step 1
// (`super::detach`) takes their words out of guest memory at the cut, and everything the draw does to them
// after drawing is here, through it.

/// `BSBatchRenderer::kActivePassIndexList` cleared (`00b64ec0`, by address).
const ACTIVE_LIST_CLEAR: u32 = 0x00b6_4ec0;
/// `NiTPointerListBase::RemoveAll` (`00b71300`, by address).
const POINTER_LIST_REMOVE_ALL: u32 = 0x00b7_1300;

/// Clears a batch renderer's active pass type list (condition 1).
#[inline(always)]
pub(crate) fn clear_active_list(e: &mut Engine, renderer: u32) {
    let list = renderer.wrapping_add(BSBatchRenderer::kActivePassIndexList.off);
    super::detach::suspended_for(
        e,
        |_| vec![list, list.wrapping_add(4)],
        |e| e.call(ACTIVE_LIST_CLEAR, &args![list]),
    );
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
    super::detach::set_half(e, holder.wrapping_add(PassHolder::usCount.off), 0);
}

/// Empties an alpha group list (`RemoveAll`) and clears the renderer's current grouped alpha
/// holder of that group (+0x90 array) (condition 1).
#[inline(always)]
pub(crate) fn empty_alpha_list(e: &mut Engine, renderer: Ptr<BSBatchRenderer>, group: u32) {
    let list = super::packet::alpha_list(renderer, group);
    let words = |_: &Engine| vec![list, list.wrapping_add(4), list.wrapping_add(8)];
    super::detach::suspended_for(e, words, |e| e.call(POINTER_LIST_REMOVE_ALL, &args![list]));
    let current = renderer
        .addr()
        .wrapping_add(group.wrapping_mul(4))
        .wrapping_add(0x90);
    super::detach::set_word(e, current, 0u32);
}

/// The pass type range a batch renderer takes from its active list (+0x20, +0x24) (condition 1:
/// the draw's cursor into the lists).
#[inline(always)]
pub(crate) fn set_pass_range(e: &mut Engine, renderer: u32, low: u32, high: u32) {
    let renderer = Ptr::<BSBatchRenderer>::new(renderer);
    e.set(renderer, BSBatchRenderer::uiFirstPass, low);
    e.set(renderer, BSBatchRenderer::uiLastPass, high);
}

/// The pass type range draw's cursors, the accumulator's arrays by renderer (+0x17c pass types, +0x180
/// buckets, +0x184 first-pass bytes, +0x188 active bytes) (condition 1: the draw's cursor into the
/// lists): the address of renderer `index`'s entry, each array read afresh as the exe reads it.
#[inline(always)]
pub(crate) fn cursor_pass_at(e: &Engine, acc: Ptr<BSShaderAccumulator>, index: u32) -> u32 {
    e.get(acc, BSShaderAccumulator::puicurrentPassA)
        .wrapping_add(index.wrapping_mul(4))
}
#[inline(always)]
pub(crate) fn cursor_bucket_at(e: &Engine, acc: Ptr<BSShaderAccumulator>, index: u32) -> u32 {
    e.get(acc, BSShaderAccumulator::picurrentBucketA)
        .wrapping_add(index.wrapping_mul(4))
}
#[inline(always)]
pub(crate) fn cursor_first_at(e: &Engine, acc: Ptr<BSShaderAccumulator>, index: u32) -> u32 {
    e.get(acc, BSShaderAccumulator::pbcurrentFirstPassA)
        .wrapping_add(index)
}
#[inline(always)]
fn cursor_active_at(e: &Engine, acc: Ptr<BSShaderAccumulator>, index: u32) -> u32 {
    e.get(acc, BSShaderAccumulator::pbcurrentActiveA)
        .wrapping_add(index)
}

/// Renderer `index`'s cursor: its pass type, its bucket, its first-pass byte, its active byte.
#[inline(always)]
pub(crate) fn cursor_pass(e: &Engine, acc: Ptr<BSShaderAccumulator>, index: u32) -> u32 {
    e.global::<u32>(cursor_pass_at(e, acc, index))
}
#[inline(always)]
pub(crate) fn cursor_bucket(e: &Engine, acc: Ptr<BSShaderAccumulator>, index: u32) -> i32 {
    e.global::<i32>(cursor_bucket_at(e, acc, index))
}
#[inline(always)]
pub(crate) fn cursor_first(e: &Engine, acc: Ptr<BSShaderAccumulator>, index: u32) -> bool {
    e.global::<u8>(cursor_first_at(e, acc, index)) != 0
}
#[inline(always)]
pub(crate) fn cursor_active(e: &Engine, acc: Ptr<BSShaderAccumulator>, index: u32) -> bool {
    e.global::<u8>(cursor_active_at(e, acc, index)) != 0
}

/// Renderer `index`'s cursor pass type cleared before its first list is made ready (condition 1).
#[inline(always)]
pub(crate) fn clear_cursor_pass(e: &mut Engine, acc: Ptr<BSShaderAccumulator>, index: u32) {
    let at = cursor_pass_at(e, acc, index);
    e.set_global(at, 0u32);
}

/// Renderer `index`'s active byte (condition 1).
#[inline(always)]
pub(crate) fn set_cursor_active(
    e: &mut Engine,
    acc: Ptr<BSShaderAccumulator>,
    index: u32,
    active: bool,
) {
    let at = cursor_active_at(e, acc, index);
    e.set_global(at, u8::from(active));
}

/// Takes the first pass off a holder's list (+0xc: head +0x10, tail +0x14, spare chain +0x18, count
/// +0x1c) into its spare chain, the holder's count (+2) down first, as `RenderNextAlpha` does after
/// drawing it (condition 1).
pub(crate) fn take_first_pass(e: &mut Engine, holder: u32) {
    if super::detach::word(e, holder.wrapping_add(0x10)) == 0 {
        return;
    }
    let count = super::detach::half(e, holder.wrapping_add(2)).wrapping_add(0xffff);
    super::detach::set_half(e, holder.wrapping_add(2), count);
    let node = super::detach::word(e, holder.wrapping_add(0x10));
    if node == 0 {
        return;
    }
    if node == super::detach::word(e, holder.wrapping_add(0x14)) {
        super::detach::set_word(e, holder.wrapping_add(0x14), 0u32);
    }
    let mut next = 0;
    if e.global::<u32>(node) != 0 {
        next = e.global::<u32>(node);
        e.set_global(next.wrapping_add(4), 0u32);
    }
    let spare = e.global::<u32>(holder.wrapping_add(0x18));
    if spare != 0 {
        let head = super::detach::word(e, holder.wrapping_add(0x10));
        e.set_global(spare.wrapping_add(4), head);
    }
    let head = super::detach::word(e, holder.wrapping_add(0x10));
    let spare = e.global::<u32>(holder.wrapping_add(0x18));
    e.set_global(head, spare);
    let head = super::detach::word(e, holder.wrapping_add(0x10));
    let items = super::detach::word(e, holder.wrapping_add(0x1c)).wrapping_sub(1);
    super::detach::set_word(e, holder.wrapping_add(0x1c), items);
    e.set_global(holder.wrapping_add(0x18), head);
    super::detach::set_word(e, holder.wrapping_add(0x10), next);
}

/// Takes the first node off an alpha group list (head, tail, count), gives it back to the node pool
/// ([`give_node`]) and counts the list down (condition 1).
pub(crate) fn take_sorted_node(e: &mut Engine, list: u32) {
    let node = super::detach::word(e, list);
    let next = e.global::<u32>(node);
    super::detach::set_word(e, list, next);
    if next != 0 {
        e.set_global(next.wrapping_add(4), 0u32);
    } else {
        super::detach::set_word(e, list.wrapping_add(4), 0u32);
    }
    give_node(e, node);
    let count = super::detach::word(e, list.wrapping_add(8)).wrapping_sub(1);
    super::detach::set_word(e, list.wrapping_add(8), count);
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
    e.call(
        SET_CLIP_PLANES,
        &args![on, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32],
    );
}

/// The word at `011a9608` set to `value`, answering what it held (condition 2: a shader-state
/// global the draw of group 0 reads).
#[inline(always)]
pub(crate) fn swap_group_zero_flag(e: &mut Engine, value: u32) -> u32 {
    let saved = e.global::<u32>(GROUP_ZERO_FLAG);
    e.set_global(GROUP_ZERO_FLAG, value);
    saved
}

/// The depth bias set-up (`00b98320`, `cdecl(bias, lock)`, by address), the decal depth bias
/// (`01200458`) and whether it is used (`011ad80f`), and the two lock counters the decal state gives
/// back by hand (`011ff9dc`, `011ff9e4`).
const SET_DEPTH_BIAS: u32 = 0x00b9_8320;
const DECAL_DEPTH_BIAS: u32 = 0x0120_0458;
const DECAL_DEPTH_BIAS_ENABLED: u32 = 0x011a_d80f;
const LOCK_COUNT_1: u32 = 0x011f_f9dc;
const LOCK_COUNT_3: u32 = 0x011f_f9e4;

/// The decal state on: z-writes off and the decal depth bias (0 when not used; the float goes through
/// x87, so a signalling NaN is quieted), both locked (condition 2).
pub(crate) fn decal_state_on(e: &mut Engine) {
    let mut bias = x87_quiet_bits(e.global::<u32>(DECAL_DEPTH_BIAS));
    if e.global::<u8>(DECAL_DEPTH_BIAS_ENABLED) == 0 {
        bias = x87_store32(0.0);
    }
    typed_call!(
        shader::FAMILY,
        0x00b9_7e30,
        state::set_z_write_enable(e, 0u8, 1u32)
    );
    e.call(SET_DEPTH_BIAS, &args![bias, 1u32]);
}

/// The decal state off: the first lock given back by hand, z-writes on, the third lock given back,
/// no depth bias (condition 2).
pub(crate) fn decal_state_off(e: &mut Engine) {
    let count = e.global::<u32>(LOCK_COUNT_1).wrapping_sub(1);
    e.set_global(LOCK_COUNT_1, count);
    typed_call!(
        shader::FAMILY,
        0x00b9_7e30,
        state::set_z_write_enable(e, 1u8, 0u32)
    );
    let count = e.global::<u32>(LOCK_COUNT_3).wrapping_sub(1);
    e.set_global(LOCK_COUNT_3, count);
    e.call(SET_DEPTH_BIAS, &args![x87_store32(0.0), 0u32]);
}

// Typed from 00bbb840
/// The lighting shaders' fog-coloured ambient, no words: the fog colour (`011a9bd0`) into the ambient
/// (`01201b30`), the ambient's enable flag (`012010c4`, +8) and the bytes `01200bf4` and `01200bf5` set
/// (condition 2: shader-state globals).
pub fn fog_ambient(e: &mut Engine) {
    for index in 0..4u32 {
        let word = e.global::<u32>(FOG_COLOUR.wrapping_add(4 * index));
        e.set_global(AMBIENT.wrapping_add(4 * index), word);
    }
    let enable = e.global::<u32>(AMBIENT_ENABLE);
    e.set_global(enable.wrapping_add(8), 1u8);
    e.set_global(AMBIENT_SET, 1u8);
    e.set_global(AMBIENT_FOG, 1u8);
}
const FOG_COLOUR: u32 = 0x011a_9bd0;
const AMBIENT: u32 = 0x0120_1b30;
const AMBIENT_ENABLE: u32 = 0x0120_10c4;
const AMBIENT_SET: u32 = 0x0120_0bf4;
const AMBIENT_FOG: u32 = 0x0120_0bf5;

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

/// A list node back to the global node pool: its item cleared, the pool's lock taken (`0040fbf0`)
/// and given back once the node is on the free list (the shader family's `pool_give`) (condition 4).
pub(crate) fn give_node(e: &mut Engine, node: u32) {
    e.set_global(node.wrapping_add(8), 0u32);
    e.call(LOCK_ENTER, &args![batch::POOL_LOCK, 0u32]);
    batch::pool_give(e, node);
}
const LOCK_ENTER: u32 = 0x0040_fbf0;

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
    let tree = e.call(CREATE_RENDER_PASS_TREE, &args![accumulator]).u32();
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
            .call(
                TEXTURE_BORROW,
                &args![manager, renderer, 5u32, 0u32, 0u32, 0u32],
            )
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

// ---- The geometry paths (group D): what the drawn geometry's draw and packing write. ----
//
// Condition 3: the changed flags of the data the draw packed or drew (`NiGeometryData` +0xe) are read
// by the next frame's other draws and set by the update; the draw settles them. With them go the
// counts and buffers the draws write into the data's buffer data. Condition 2: the device's world
// transform, streams and indices, and the renderer's copies of the drawn bound and camera vectors,
// which the next draw on main reads. Condition 5: the blocks the drawing allocates (the vertex lock's
// staging block, a skin's bone-matrix buffer, a declaration's element array). The vertex lock's
// critical section (held from the lock to the unlock) and the reference counts given back (a
// declaration or an index buffer released) are here too.

/// `IDirect3DDevice9::SetTransform` (slot 44), `SetStreamSource` (slot 100) and `SetIndices` (slot
/// 104); `D3DTS_WORLD`.
const DEVICE_SET_TRANSFORM: u32 = 0xb0;
const DEVICE_SET_STREAM_SOURCE: u32 = 0x190;
const DEVICE_SET_INDICES: u32 = 0x1a0;
const TRANSFORM_WORLD: u32 = 0x100;
/// `IUnknown::Release` (slot 2).
const SLOT_RELEASE: u32 = 8;
/// The staging block's allocate (`00aa1070`, `cdecl(size)`) and free (`00aa10f0`, `cdecl(block)`), the
/// aligned allocate (`00aa10b0`, `cdecl(size, alignment)`) and free (`00aa1110`, `cdecl(block)`), the
/// memory manager's allocate (`00aa3e40`, `thiscall(manager, size)`).
const ALLOCATE: u32 = 0x00aa_1070;
const FREE: u32 = 0x00aa_10f0;
const ALLOCATE_ALIGNED: u32 = 0x00aa_10b0;
const FREE_ALIGNED: u32 = 0x00aa_1110;
const MEMORY_ALLOCATE: u32 = 0x00aa_3e40;
/// `EnterCriticalSection` and `LeaveCriticalSection` (the imports' slots).
const ENTER_CRITICAL_SECTION: u32 = 0x00fd_f05c;
const LEAVE_CRITICAL_SECTION: u32 = 0x00fd_f100;
/// The vertex manager's critical section (+0x80), staging block (+0x40) and its capacity (+0x44).
const STAGING_SECTION: u32 = 0x80;
const STAGING_BLOCK: u32 = 0x40;
const STAGING_CAPACITY: u32 = 0x44;

/// The data's changed flags (+0xe) keep only their top four bits once it is drawn (condition 3).
#[inline(always)]
pub(crate) fn settle_changed_flags(e: &mut Engine, data: u32) {
    let flags = e.global::<u16>(data.wrapping_add(0xe));
    e.set_global(data.wrapping_add(0xe), flags & 0xf000);
}

/// The points draw's counts in the buffer data: four vertices (+0x18, +0x1c) and two triangles (+0x40,
/// +0x44) a point of the active and the whole count, no strips (+0x50, +0x4c), one primitive group
/// (+0x48) (condition 3).
#[inline(always)]
pub(crate) fn points_buffer_counts(
    e: &mut Engine,
    buffer: u32,
    vertices: u32,
    all: u32,
    count: u32,
) {
    e.set_global(buffer.wrapping_add(0x18), vertices);
    e.set_global(buffer.wrapping_add(0x1c), all.wrapping_mul(4));
    e.set_global(buffer.wrapping_add(0x40), count.wrapping_mul(2));
    e.set_global(buffer.wrapping_add(0x44), all.wrapping_mul(2));
    e.set_global(buffer.wrapping_add(0x50), 0u32);
    e.set_global(buffer.wrapping_add(0x4c), 0u32);
    e.set_global(buffer.wrapping_add(0x48), 1u32);
}

/// A new index buffer for the points kept in the buffer data (+0x34), with its index count (+0x2c)
/// and size (+0x30) (condition 3).
#[inline(always)]
pub(crate) fn points_index_buffer(
    e: &mut Engine,
    buffer: u32,
    index_buffer: u32,
    count: u32,
    size: u32,
) {
    e.set_global(buffer.wrapping_add(0x34), index_buffer);
    e.set_global(buffer.wrapping_add(0x2c), count);
    e.set_global(buffer.wrapping_add(0x30), size);
}

/// A remade chip into the data's chip array (+0x28) when the stream is within its count (+0x20)
/// (condition 3).
#[inline(always)]
pub(crate) fn store_chip(e: &mut Engine, data: u32, stream: u32, chip: u32) {
    if stream < e.global::<u32>(data.wrapping_add(0x20)) {
        let chips = e.global::<u32>(data.wrapping_add(0x28));
        e.set_global(chips.wrapping_add(stream.wrapping_mul(4)), chip);
    }
}

/// The drawn bound's copy in the renderer (centre +0x664, radius +0x670) (condition 2).
#[inline(always)]
pub(crate) fn drawn_bound(e: &mut Engine, renderer: u32, centre: [u32; 3], radius: u32) {
    for (i, word) in (0u32..).zip(centre) {
        e.set_global(renderer.wrapping_add(0x664 + 4 * i), word);
    }
    e.set_global(renderer.wrapping_add(0x670), radius);
}

/// A camera vector in model space (three words from `from`) into the renderer at `to` (condition 2).
#[inline(always)]
pub(crate) fn camera_vector(e: &mut Engine, renderer: u32, to: u32, from: u32) {
    for i in 0..3u32 {
        let word = e.global::<u32>(from.wrapping_add(4 * i));
        e.set_global(renderer.wrapping_add(to).wrapping_add(4 * i), word);
    }
}

/// The device's world transform (`SetTransform(D3DTS_WORLD, world)`) (condition 2).
#[inline(always)]
pub(crate) fn device_world_transform(e: &mut Engine, renderer: u32, world: u32) {
    let device = super::packet::renderer_device(e, renderer);
    e.vcall(device, DEVICE_SET_TRANSFORM, &args![TRANSFORM_WORLD, world]);
}

/// Stream 0 of the device (`SetStreamSource(0, buffer, 0, stride)`) (condition 2).
#[inline(always)]
pub(crate) fn device_stream(e: &mut Engine, renderer: u32, buffer: u32, stride: u32) {
    let device = super::packet::renderer_device(e, renderer);
    e.vcall(
        device,
        DEVICE_SET_STREAM_SOURCE,
        &args![0u32, buffer, 0u32, stride],
    );
}

/// The device's indices (`SetIndices`) (condition 2).
#[inline(always)]
pub(crate) fn device_indices(e: &mut Engine, renderer: u32, indices: u32) {
    let device = super::packet::renderer_device(e, renderer);
    e.vcall(device, DEVICE_SET_INDICES, &args![indices]);
}

/// A Direct3D object's count given back (`Release`) (condition 7).
#[inline(always)]
pub(crate) fn release(e: &mut Engine, object: u32) {
    e.vcall(object, SLOT_RELEASE, &args![]);
}

/// The vertex manager's critical section entered for a lock (held until [`leave_staging`]).
#[inline(always)]
pub(crate) fn enter_staging(e: &mut Engine, manager: u32) {
    e.call(
        ENTER_CRITICAL_SECTION,
        &args![manager.wrapping_add(STAGING_SECTION)],
    );
}

/// The vertex manager's critical section left at an unlock.
#[inline(always)]
pub(crate) fn leave_staging(e: &mut Engine, manager: u32) {
    e.call(
        LEAVE_CRITICAL_SECTION,
        &args![manager.wrapping_add(STAGING_SECTION)],
    );
}

/// The vertex manager's staging block made anew for `size` bytes: the old one freed, a new one taken,
/// both kept with the capacity (condition 5: about one frame in 70 grows it).
#[inline(always)]
pub(crate) fn grow_staging(e: &mut Engine, manager: u32, size: u32) {
    let old = e.global::<u32>(manager.wrapping_add(STAGING_BLOCK));
    e.call(FREE, &args![old]);
    let block = e.call(ALLOCATE, &args![size]).u32();
    e.set_global(manager.wrapping_add(STAGING_BLOCK), block);
    e.set_global(manager.wrapping_add(STAGING_CAPACITY), size);
}

/// A skin's bone-matrix buffer made anew for `size` bytes, 16-byte aligned: the old one freed, the new
/// one kept at +0x28 with its size at +0x24 (condition 5). Answers the new buffer.
#[inline(always)]
pub(crate) fn renew_bone_buffer(e: &mut Engine, skin: u32, old: u32, size: u32) -> u32 {
    e.call(FREE_ALIGNED, &args![old]);
    let buffer = e.call(ALLOCATE_ALIGNED, &args![size, 0x10u32]).u32();
    e.set_global(skin.wrapping_add(0x28), buffer);
    e.set_global(skin.wrapping_add(0x24), size);
    buffer
}

/// A block from the memory manager (condition 5).
#[inline(always)]
pub(crate) fn allocate_block(e: &mut Engine, size: u32) -> u32 {
    e.call(MEMORY_ALLOCATE, &args![MEMORY_MANAGER, size]).u32()
}

/// A block back to the memory manager (condition 4).
#[inline(always)]
pub(crate) fn free_block(e: &mut Engine, block: u32) {
    e.call(MEMORY_FREE, &args![MEMORY_MANAGER, block]);
}

// ---- The pass begin and end (group B): the shader state a pass leaves behind. ----
//
// Condition 2: what the pass begin sets that the next draw on main reads: the current shader pass
// (`0126f74c`), the textures its stages take, the constant entries switched on for the pass type, the
// fog and toggle constants, the device's vertex shader constants. Condition 7: the shader's counted
// pointers the pass begin sets (its +0x28 and the two constant maps, +0x30 and +0x34).

/// `IDirect3DDevice9::SetVertexShaderConstantF` (slot 94).
const DEVICE_SET_VERTEX_CONSTANTS: u32 = 0x178;
/// `00a5f860` (a counted pointer set to an object, `thiscall(slot, object)`) and `00c663d0` (a counted
/// pointer set from another, `thiscall(slot, source)`), by address.
const SET_COUNTED: u32 = 0x00a5_f860;
const COPY_COUNTED: u32 = 0x00c6_63d0;

/// The current shader pass set (condition 2).
#[inline(always)]
pub(crate) fn set_current_pass(e: &mut Engine, pass: u32) {
    e.set_global(super::packet::CURRENT_PASS, pass);
}

/// A pass stage's texture (+8) set (condition 2).
#[inline(always)]
pub(crate) fn set_stage_texture(e: &mut Engine, stage: u32, texture: u32) {
    e.set_global(stage.wrapping_add(8), texture);
}

/// A constant switch object's word +0x20 set (condition 2).
#[inline(always)]
pub(crate) fn set_switch_word(e: &mut Engine, switch: u32, value: u32) {
    e.set_global(switch.wrapping_add(0x20), value);
}

/// A constant entry switched on or off (its byte +8) for the pass (condition 2).
#[inline(always)]
pub(crate) fn set_entry_enabled(e: &mut Engine, entry: u32, on: bool) {
    e.set_global(entry.wrapping_add(8), u8::from(on));
}

/// A word of the shader constants the pass begin fills (the fog constants `011fa280`, their colour's
/// copies `011fd9b8`, the land toggles `011fa2d0`) (condition 2).
#[inline(always)]
pub(crate) fn set_constant_word(e: &mut Engine, at: u32, word: u32) {
    e.set_global(at, word);
}

/// `count` vertex shader constant registers from `register` on (`data`) on `device`, of the kind the
/// device's slot `slot` sets (floats 0x178, integers 0x180, booleans 0x188) (condition 2). Answers the
/// device's result.
#[inline(always)]
pub(crate) fn device_vertex_constants_of(
    e: &mut Engine,
    device: u32,
    slot: u32,
    register: u32,
    data: u32,
    count: u32,
) -> u32 {
    e.vcall(device, slot, &args![register, data, count]).u32()
}

/// `count` float vertex shader constant registers from `register` on (`data`) on `device` (condition 2).
#[inline(always)]
pub(crate) fn device_vertex_constants(
    e: &mut Engine,
    device: u32,
    register: u32,
    data: u32,
    count: u32,
) {
    device_vertex_constants_of(
        e,
        device,
        DEVICE_SET_VERTEX_CONSTANTS,
        register,
        data,
        count,
    );
}

/// The counted pointer at `slot` set to the word at `source`, the exe's own code (condition 7):
/// nothing when they are equal; else the old object's count goes down (deleted at 0, its slot 4), the
/// new one is stored (read again from `source` when `reread`) and, when not null, counted.
pub(crate) fn assign_counted(e: &mut Engine, slot: u32, source: u32, reread: bool) {
    let old = e.global::<u32>(slot);
    let new = e.global::<u32>(source);
    if old == new {
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
    let new = if reread { e.global::<u32>(source) } else { new };
    e.set_global(slot, new);
    if new != 0 {
        e.call(INTERLOCKED_INCREMENT, &args![new.wrapping_add(4)]);
    }
}

/// The counted pointer at `slot` set to `object` through `00a5f860` (condition 7).
#[inline(always)]
pub(crate) fn set_counted(e: &mut Engine, slot: u32, object: u32) {
    e.call(SET_COUNTED, &args![slot, object]);
}

/// The counted pointer at `slot` set from the one at `source` through `00c663d0` (condition 7).
#[inline(always)]
pub(crate) fn copy_counted(e: &mut Engine, slot: u32, source: u32) {
    e.call(COPY_COUNTED, &args![slot, source]);
}

/// A pass stage's texture (+8) as the draw left it, read back.
#[inline(always)]
pub(crate) fn stage_texture(e: &Engine, stage: u32) -> u32 {
    e.global::<u32>(stage.wrapping_add(8))
}

/// Whether a constant entry is switched on (its byte +8), read back.
#[inline(always)]
pub(crate) fn entry_is_enabled(e: &Engine, entry: u32) -> bool {
    e.global::<u8>(entry.wrapping_add(8)) != 0
}

/// The shader's counted constant map at `offset` (+0x30 pixel, +0x34 vertex), read back.
#[inline(always)]
pub(crate) fn constant_map(e: &Engine, shader: u32, offset: u32) -> u32 {
    e.global::<u32>(shader.wrapping_add(offset))
}

/// `size` bytes of constants from `from` copied into an entry's own buffer at `to` (`_memcpy`, the CRT
/// family's) (condition 2).
#[inline(always)]
pub(crate) fn copy_constants(e: &mut Engine, to: u32, from: u32, size: u32) {
    let _: u32 = typed_call!(
        crate::crt::FAMILY,
        0x00ec_44d0,
        crate::crt::mem::memcpy(e, to, from, size)
    );
}
