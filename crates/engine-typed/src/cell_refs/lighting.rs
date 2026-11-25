//! The shadow scene node's queue of the objects whose lighting is to be redone, as a cell's load
//! fills it: `ShadowSceneNode::AddObject` (`00b5eeb0`, Xbox PDB) and
//! `ShadowSceneNode::UpdateObjectLighting` (`00b5d9f0`, Xbox PDB), with the add at the tail of a
//! list of counted pointers (`00a5a310`) that puts the object in the queue. `TES`'s load of a
//! reference into the scene ([`super::scene`]) calls both for every node it puts in the scene
//! (`AddObject`, then the update but for a type 0x1e base form). The exe's other callers (a cell's
//! node attach `PerformCellNodeAttach`, an actor's 3D moved `UpdateActor3DPosition`, the animation
//! scene graph updates, `FinalizeGeometry`, `Update3dModel` through `AddObject`, about a dozen in
//! all) call them by address, so they reach them through their registrations; `00a5a310` is the
//! exe's add for every list of counted pointers (an object's properties among them). The queue is
//! drained once a frame before the draw (`00b5fd60`, the lights family's).
//!
//! **Native.** The exe's order of work and its calls:
//!
//! - the queue's lock (`011f9ec0`, the game's recursive spin lock) is taken with `0040fbf0` (the
//!   dispatch family's) and given back in place, as the exe inlines it: the count at `011f9ec4`
//!   down and, at zero, the owner word cleared (the dispatch family's `0040fba0` body without its
//!   empty call, shared with it: a release store when the memory is shared between real threads);
//! - the object is held by an `NiPointer` temporary in a stack word (its count at +4 up,
//!   `InterlockedIncrement`) while the list's add takes a count of its own, and released after
//!   (`InterlockedDecrement`, the object deleted through its slot 4 at zero);
//! - the list's add takes a node from the list node pool (`0043a010`, the draw F family's) and
//!   gives it the object (the node's old element released and the new one counted, unless they
//!   are the same), then links it after the tail and counts it.
//!
//! The interlocked words go through the engine's direct calls of the imports
//! ([`engine_core::sync_direct`]), as the other families' reference counts do.
//!
//! **JIP LN NVSE.** Nothing of JIP's (or of the other built-in extenders) replaces these functions
//! or hooks a site in them; with JIP on (the default preset) its attach calls `TES`'s load by
//! address, so the same code runs here with JIP on and off.
//!
//! **Which thread.** Main for the cell's load, the model updates and the animation scene graph
//! updates (`UpdateNonRenderSafeAITasks` calls the parallel one on main); a caller on another
//! thread takes the game's own lock as the exe does. Integers only: no float rule applies.

use super::FAMILY;
use crate::dispatch::layout::LightLock;
use crate::{dispatch, draw_f};
use engine_core::ptr::Ptr;
use engine_core::sync_direct::{interlocked_decrement, interlocked_increment};
use engine_core::{args, typed_call, Engine};

/// `NiAVObject::m_uFlags` (+0x30): bit 0, the object culled (`AddObject` leaves it out).
const AV_FLAGS: u32 = 0x30;
const AV_FLAG_CULLED: u8 = 1;
/// The node's queues: `lObjectQueueUpdate` (+0x108) and `lObjectQueueUpdateActiveOnly` (+0x114)
/// (Xbox PDB), each a list of counted pointers.
const OBJECT_QUEUE_UPDATE: u32 = 0x108;
const OBJECT_QUEUE_UPDATE_ACTIVE_ONLY: u32 = 0x114;
/// The queues' lock (its owner word; its count at `011f9ec4`).
const OBJECT_QUEUE_LOCK: u32 = 0x011f_9ec0;
/// A counted object's count (+4) and its deleting slot (4).
const REF_COUNT: u32 = 4;
const SLOT_DELETE: u32 = 4;
/// A list of counted pointers: its head (+0), tail (+4) and count (+8, the allocator's word the
/// node pool is called on). A node: the next node (+0), the previous (+4), the element (+8).
const LIST_HEAD: u32 = 0;
const LIST_TAIL: u32 = 4;
const LIST_COUNT: u32 = 8;
const NODE_NEXT: u32 = 0;
const NODE_PREVIOUS: u32 = 4;
const NODE_ELEMENT: u32 = 8;

/// The object's count up (an `NiPointer` taking it).
#[inline]
fn count_up(e: &mut Engine, object: u32) {
    interlocked_increment(e, object.wrapping_add(REF_COUNT));
}

/// The object's count down (an `NiPointer` letting it go), the object deleted (slot 4) at zero.
#[inline]
fn count_down(e: &mut Engine, object: u32) {
    if interlocked_decrement(e, object.wrapping_add(REF_COUNT)) == 0 {
        e.vcall(object, SLOT_DELETE, &args![]);
    }
}

/// The queues' lock given back as the exe inlines it: the count down and, at zero, no owner (the
/// dispatch family's [`dispatch::refs::unlock_inline`], `0040fba0`'s body: a release store of the
/// owner word when the memory is shared between real threads, so the list's links made under the
/// lock are seen by the next thread that takes it).
#[inline]
fn leave_queue_lock(e: &mut Engine) {
    dispatch::refs::unlock_inline(e, Ptr::<LightLock>::new(OBJECT_QUEUE_LOCK));
}

// Typed from 00b5eeb0
/// `ShadowSceneNode::AddObject` (Xbox PDB), `thiscall(node, object)`, `ret 4`: an object that is
/// not null and not culled (bit 0 of the byte at +0x30) goes to `UpdateObjectLighting` with 0.
pub fn add_object(e: &mut Engine, this: u32, object: u32) {
    if object == 0 || e.mem.u8(object.wrapping_add(AV_FLAGS)) & AV_FLAG_CULLED != 0 {
        return;
    }
    typed_call!(
        FAMILY,
        0x00b5_d9f0,
        update_object_lighting(e, this, object, 0)
    );
}

// Typed from 00b5d9f0
/// `ShadowSceneNode::UpdateObjectLighting` (Xbox PDB), `thiscall(node, object, active only)`, `ret
/// 8`: nothing for a null object. Under the queues' lock the object, held by an `NiPointer`
/// temporary, is added at the tail of `lObjectQueueUpdate`, or of `lObjectQueueUpdateActiveOnly`
/// when the byte `active_only` is set; then the temporary lets it go and the lock is given back.
pub fn update_object_lighting(e: &mut Engine, this: u32, object: u32, active_only: u8) {
    if object == 0 {
        return;
    }
    typed_call!(
        dispatch::FAMILY,
        0x0040_fbf0,
        dispatch::refs::lock(e, Ptr::<LightLock>::new(OBJECT_QUEUE_LOCK), 0u32)
    );
    let queue = if active_only == 0 {
        OBJECT_QUEUE_UPDATE
    } else {
        OBJECT_QUEUE_UPDATE_ACTIVE_ONLY
    };
    let list = this.wrapping_add(queue);
    e.with_stack(4, |e, held| {
        e.mem.set_u32(held.addr(), object);
        count_up(e, object);
        typed_call!(FAMILY, 0x00a5_a310, list_add_tail(e, list, held.addr()));
        count_down(e, object);
    });
    leave_queue_lock(e);
}

// Typed from 00a5a310
/// The add at the tail of a list of counted pointers (`thiscall(list, pointer)`, `ret 4`; the
/// pointer is an `NiPointer`'s address): a node from the list node pool (`0043a010` on the list's
/// count word) is given the object the pointer holds (its old element let go and the new one
/// counted, both left alone when they are the same; the pointer read again for the store) and
/// linked after the tail (the head too for an empty list); the list's count grows.
pub fn list_add_tail(e: &mut Engine, list: u32, pointer: u32) {
    let count_word = list.wrapping_add(LIST_COUNT);
    let node = typed_call!(
        draw_f::FAMILY,
        0x0043_a010,
        draw_f::helpers::allocate_node(e, Ptr::new(count_word))
    )
    .addr();
    let old = e.mem.u32(node.wrapping_add(NODE_ELEMENT));
    if old != e.mem.u32(pointer) {
        if old != 0 {
            count_down(e, old);
        }
        let new = e.mem.u32(pointer);
        e.mem.set_u32(node.wrapping_add(NODE_ELEMENT), new);
        if new != 0 {
            count_up(e, new);
        }
    }
    e.mem.set_u32(node.wrapping_add(NODE_NEXT), 0);
    let tail = e.mem.u32(list.wrapping_add(LIST_TAIL));
    e.mem.set_u32(node.wrapping_add(NODE_PREVIOUS), tail);
    let tail = e.mem.u32(list.wrapping_add(LIST_TAIL));
    if tail != 0 {
        e.mem.set_u32(tail.wrapping_add(NODE_NEXT), node);
    }
    let count = e.mem.u32(count_word).wrapping_add(1);
    e.mem.set_u32(count_word, count);
    if tail == 0 {
        e.mem.set_u32(list.wrapping_add(LIST_HEAD), node);
    }
    e.mem.set_u32(list.wrapping_add(LIST_TAIL), node);
}
