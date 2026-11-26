//! The shadow scene node's (`ShadowSceneNode`, Xbox PDB) light lists and queues and the small
//! containers under them: its lights looked up by `NiLight`
//! (`GetLight`) and counted, the lights added (`AddLight`, a node's lights `AddAllLightsFromNode`
//! and `FindAndCullLight`), made dynamic and followed as they move; the queues of lights to update
//! and to remove (`RemoveLight`), the drop of every shadow map, the removal of an object's
//! geometries from their lights (`RemoveObject`); the light's point, shadow map and shadow scene
//! setters; the list move after an item; and the `BSSimpleArray`-shaped arrays' slot reservation,
//! search, removals and clear.
//!
//! Done in place: the `NiPointer` counts (`InterlockedIncrement` / `InterlockedDecrement` as direct
//! import calls), the light queues' lock given back (the dispatch family's `0040fba0` body, as the
//! exe inlines it), the objects' `IsNode` and `IsTriBasedGeom` slots and the lights' RTTI slot when
//! they hold the exe's constant bodies ([`slot_word`]), and the point comparison of a moved light
//! (`NiPoint3::operator!=` `00439090` and its `operator==` `004390c0`: three float compares), and
//! the arrays' element moves (`00cb9930`: a 4-byte `memmove` per element).
//! Typed calls: the queues' lock (`0040fbf0`, the dispatch family's), the counted list's add at the
//! tail (`00a5a310`, the cell references family's), the object update (`00a59c60`, the scene
//! family's), the cast (`00653290`, the dispatch family's), the shadow drop of the set-up family
//! (`00b5c3c0`), the light's constructor, `SetLight`, shadow flag and geometry removal (the lights
//! family's), and these functions' calls of each other. What stays by address: the allocation, the
//! `NiLight` constructor, the light update (`00b5d300`) and its cull (`00b9ef30`), the renderer's
//! shadow map release and end, the property's light walk, the arrays' allocator slots, and an array
//! search's compare function.
//!
//! Also here: the counted list's `RemoveAll` (`00e74d40`), which the node's lists, the particle
//! systems' modifier lists and every other `NiPointer` list of the exe share: each item's pointer
//! let go and the item handed back to the item pool under the pool's lock (the dispatch family's
//! `0040fbf0`, given back in place); and the arrays' growth (`00cb9ce0`), which the physics code's
//! `BSSimpleArray`s share too.
//!
//! **JIP LN NVSE.** Its H03 patch writes the size of the `NiLight` a light made dynamic gets
//! (`00b5cd00`'s push at `00b5cd29`): the seam `00b5cd2a` is asked here as in the translation,
//! behind the same test of JIP's patches.
//!
//! **Floats.** None worked out: the sort comparison of two lights' luminances and the moved
//! light's point comparison only compare floats (exact under either control word, a NaN unordered
//! as in the exe), and the radius a node's point lights get is stored as the exe's `FLD`/`FSTP`
//! stores it (a signalling NaN made quiet).

use super::FAMILY;
use crate::dispatch::layout::LightLock;
use crate::lights::walk::slot_word;
use crate::lights::{self, scene_light, ShadowSceneLight};
use crate::{cell_refs, dispatch, scene, setup};
use engine_core::ptr::Ptr;
use engine_core::sync_direct::{interlocked_decrement, interlocked_increment};
use engine_core::types::NiTPointerList;
use engine_core::x87::x87_quiet_bits;
use engine_core::{args, typed_call, Engine};

/// The node's lists (each an `NiTPointerList`: head, tail, count): `lLightList` (+0xb4),
/// `lShadowLightList` (+0xc0), `lLightQueueAdd` (+0xe4), `lLightQueueRemove` (+0xf0) and
/// `lLightQueueUpdate` (+0xfc) (Xbox PDB names); `kShadowLightActiveFence` and
/// `kShadowLightFadingInFence` (+0xd0, +0xd4), `pBSPortalGraph` (+0x1e0) and
/// `bAllowLightRemoveQueues` (+0x1fc).
pub(super) const LIGHT_LIST: u32 = 0xb4;
pub(super) const SHADOW_LIGHT_LIST: u32 = 0xc0;
pub(super) const LIGHT_QUEUE_ADD: u32 = 0xe4;
pub(super) const LIGHT_QUEUE_REMOVE: u32 = 0xf0;
pub(super) const LIGHT_QUEUE_UPDATE: u32 = 0xfc;
pub(super) const ACTIVE_FENCE: u32 = 0xd0;
pub(super) const FADING_FENCE: u32 = 0xd4;
pub(super) const PORTAL_GRAPH: u32 = 0x1e0;
const ALLOW_REMOVE_QUEUES: u32 = 0x1fc;
/// A list item: the next item (+0), the previous (+4), the element (+8).
pub(super) const ITEM_NEXT: u32 = 0;
pub(super) const ITEM_PREVIOUS: u32 = 4;
pub(super) const ITEM_ELEMENT: u32 = 8;
/// The light queues' lock (its owner word; its count at `011f9ea4`).
const QUEUE_LOCK: u32 = 0x011f_9ea0;
/// A shadow scene light's portal graph (+0x240, the first word of the image's last four).
pub(super) const LIGHT_PORTAL_GRAPH: u32 = 0x240;
/// `NiAVObject` members (PC): `m_uFlags` (+0x30, bit 0 culled), the world translation (+0x8c);
/// `NiNode`'s children (+0xa0, their used size +0xa6) and the node's child index word (+0xb0).
pub(super) const AV_FLAGS: u32 = 0x30;
pub(super) const WORLD_TRANSLATE: u32 = 0x8c;
const CHILDREN_BASE: u32 = 0xa0;
const CHILDREN_SIZE: u32 = 0xa6;
const CHILD_INDEX: u32 = 0xb0;
/// A geometry's property words: the lighting property at +0xa8, or through the word at +0x9c
/// (its +0xc); a property's type at +0x1c.
const GEOMETRY_PROPERTY_BASE: u32 = 0x9c;
const GEOMETRY_PROPERTY: u32 = 0xa8;
const PROPERTY_TYPE: u32 = 0x1c;
/// The slots: the deleting destructor of a counted object (+4, no stack words) and of the sun light
/// (+0, a flag), the RTTI (+8), `IsNode` (+0xc), `IsTriBasedGeom` (+0x1c); an array's allocate
/// (+4, an element count) and free (+8, the buffer).
pub(super) const SLOT_DELETE: u32 = 4;
const SLOT_RTTI: u32 = 8;
const SLOT_IS_NODE: u32 = 0xc;
const SLOT_IS_TRI_BASED_GEOM: u32 = 0x1c;
const SLOT_ARRAY_ALLOCATE: u32 = 4;
const SLOT_ARRAY_FREE: u32 = 8;
const SLOT_ARRAY_REALLOCATE: u32 = 0xc;
/// An array (`BSSimpleArray`-shaped): vtable, buffer (+4), count (+8), capacity (+0xc).
const ARRAY_BUFFER: u32 = 4;
const ARRAY_COUNT: u32 = 8;
const ARRAY_CAPACITY: u32 = 0xc;
/// The RTTI records: the radius-free light (`011f4a68`), `NiPointLight` (`011f4a98`), `NiLight`
/// (`011f4a28`) and `ShadowSceneNode` (`011f9e80`).
const RADIUS_FREE_LIGHT_RTTI: u32 = 0x011f_4a68;
const POINT_LIGHT_RTTI: u32 = 0x011f_4a98;
const LIGHT_RTTI: u32 = 0x011f_4a28;
const SHADOW_SCENE_NODE_RTTI: u32 = 0x011f_9e80;
/// The renderer object (`011f91a8`), its shadow map release (`00b6d4c0`) and its end of the maps'
/// drop (`00b6d130`).
pub(super) const RENDERER: u32 = 0x011f_91a8;
const RENDERER_RELEASE_MAP: u32 = 0x00b6_d4c0;
const RENDERER_MAPS_DROPPED: u32 = 0x00b6_d130;
/// By address: the allocation (`cdecl`, a size), `NiLight`'s constructor (`00a7d670`), the light
/// update (`00b5d300`) and cull (`00b9ef30`), the lighting property's light walk (`00b70490`, its
/// iterator a word the caller holds).
pub(super) const NEW_OBJECT: u32 = 0x00aa_13e0;
const NI_LIGHT_CONSTRUCT: u32 = 0x00a7_d670;
pub(super) const LIGHT_UPDATE: u32 = 0x00b5_d300;
pub(super) const LIGHT_CULL: u32 = 0x00b9_ef30;
const PROPERTY_NEXT_LIGHT: u32 = 0x00b7_0490;
/// The size of the `NiLight` `00b5cd00` makes (its `PUSH 0xFC`; JIP's H03 answers its own size at
/// the seam `00b5cd2a` inside that push).
const NI_LIGHT_SIZE: u32 = 0xfc;
/// `0x250` bytes: a `ShadowSceneLight`.
pub(super) const SHADOW_SCENE_LIGHT_SIZE: u32 = 0x250;

/// An item's next item.
#[inline]
pub(super) fn next_of(e: &Engine, item: u32) -> u32 {
    e.mem.u32(item.wrapping_add(ITEM_NEXT))
}

/// An item's element.
#[inline]
pub(super) fn element_of(e: &Engine, item: u32) -> u32 {
    e.mem.u32(item.wrapping_add(ITEM_ELEMENT))
}

/// The head item of the node's list at `list`.
#[inline]
pub(super) fn head_of(e: &Engine, this: u32, list: u32) -> u32 {
    e.mem.u32(this.wrapping_add(list))
}

/// A light's `NiLight` (`spLight`).
#[inline]
pub(super) fn ni_light_of(e: &Engine, light: u32) -> u32 {
    e.mem.u32(light.wrapping_add(ShadowSceneLight::spLight.off))
}

/// The object's count up (an `NiPointer` taking it).
#[inline]
pub(super) fn hold(e: &mut Engine, object: u32) {
    interlocked_increment(e, object.wrapping_add(4));
}

/// The object's count down (an `NiPointer` letting it go), the object deleted (slot 4) at zero.
#[inline]
pub(super) fn release(e: &mut Engine, object: u32) {
    if interlocked_decrement(e, object.wrapping_add(4)) == 0 {
        e.vcall(object, SLOT_DELETE, &args![]);
    }
}

/// The light queues' lock taken (`0040fbf0` with 0).
#[inline]
fn lock_queues(e: &mut Engine) {
    typed_call!(
        dispatch::FAMILY,
        0x0040_fbf0,
        dispatch::refs::lock(e, Ptr::<LightLock>::new(QUEUE_LOCK), 0u32)
    );
}

/// The light queues' lock given back as the exe inlines it: the count down and, at zero, no owner.
#[inline]
fn unlock_queues(e: &mut Engine) {
    dispatch::refs::unlock_inline(e, Ptr::<LightLock>::new(QUEUE_LOCK));
}

/// The inlined `AddTail(NiPointer(object))` of the list at `list`: the object in a stack word,
/// held (unless `checked` and null) while the list's add takes a count of its own, then let go.
pub(super) fn add_tail_held(e: &mut Engine, list: u32, object: u32, checked: bool) {
    e.with_stack(4, |e, slot| {
        e.mem.set_u32(slot.addr(), object);
        let held = !checked || object != 0;
        if held {
            hold(e, object);
        }
        typed_call!(
            cell_refs::FAMILY,
            0x00a5_a310,
            cell_refs::lighting::list_add_tail(e, list, slot.addr())
        );
        if held {
            release(e, object);
        }
    });
}

/// The inlined `NiPointer` assignment of `object` to the word at `slot`: nothing for the same
/// object; the old one let go, the new one stored and held.
pub(super) fn assign(e: &mut Engine, slot: u32, object: u32) {
    let old = e.mem.u32(slot);
    if old == object {
        return;
    }
    if old != 0 {
        release(e, old);
    }
    e.mem.set_u32(slot, object);
    if object != 0 {
        hold(e, object);
    }
}

/// The inlined kind-of test: the object's RTTI ([`slot_word`]), then the chain of base records
/// (each `NiRTTI`'s +4) until `rtti` or null.
fn is_kind_of(e: &mut Engine, object: u32, rtti: u32) -> bool {
    let mut record = slot_word(e, object, SLOT_RTTI);
    while record != 0 {
        if record == rtti {
            return true;
        }
        record = e.mem.u32(record.wrapping_add(4));
    }
    false
}

/// The first light of `lLightList` whose `spLight` is `ni_light` (null elements skipped), or 0:
/// `GetLight`'s walk, which `00b5c450`, `00b5f080` and `RemoveLight` inline.
fn light_of(e: &Engine, this: u32, ni_light: u32) -> u32 {
    let mut item = head_of(e, this, LIGHT_LIST);
    while item != 0 {
        let light = element_of(e, item);
        item = next_of(e, item);
        if light != 0 && ni_light_of(e, light) == ni_light {
            return light;
        }
    }
    0
}

/// The arrays' element move (`00cb9930`, `thiscall(array, to, from, count)`, `ret 0xc`, the array
/// unread) in place: `count` words from `from` to `to`, each a 4-byte `memmove` in the exe, upwards
/// when `to` is below `from`, downwards from the last when above (no move when the count less one
/// is negative as a signed word), none when they are the same or the count is 0.
fn move_words(e: &mut Engine, to: u32, from: u32, count: u32) {
    if count == 0 || to == from {
        return;
    }
    let copy = |e: &mut Engine, i: u32| {
        let word = e.mem.u32(from.wrapping_add(i.wrapping_mul(4)));
        e.mem.set_u32(to.wrapping_add(i.wrapping_mul(4)), word);
    };
    if to < from {
        for i in 0..count {
            copy(e, i);
        }
    } else if (count.wrapping_sub(1) as i32) >= 0 {
        for i in (0..count).rev() {
            copy(e, i);
        }
    }
}

// Typed from 00cb9ce0
/// The arrays' growth (`thiscall(array, capacity, count)`, `ret 8`; `BSSimpleArray`-shaped, its
/// allocator slots asked): without a buffer, one from the allocate slot (+4) and the capacity set;
/// for a count equal to the capacity, the reallocate slot (+0xc: the buffer, the capacity) answers
/// the buffer; otherwise a new buffer from the allocate slot takes the `count` elements (the move
/// `00cb9930` in place, [`move_words`]) and the old one goes to the free slot (+8). The capacity is
/// the caller's to set in the last two.
pub fn grow(e: &mut Engine, this: Ptr, capacity: u32, count: u32) {
    let array = this.addr();
    let buffer = e.mem.u32(array.wrapping_add(ARRAY_BUFFER));
    if buffer == 0 {
        let fresh = e.vcall(array, SLOT_ARRAY_ALLOCATE, &args![capacity]).u32();
        e.mem.set_u32(array.wrapping_add(ARRAY_CAPACITY), capacity);
        e.mem.set_u32(array.wrapping_add(ARRAY_BUFFER), fresh);
        return;
    }
    if count == e.mem.u32(array.wrapping_add(ARRAY_CAPACITY)) {
        let kept = e
            .vcall(array, SLOT_ARRAY_REALLOCATE, &args![buffer, capacity])
            .u32();
        e.mem.set_u32(array.wrapping_add(ARRAY_BUFFER), kept);
        return;
    }
    let fresh = e.vcall(array, SLOT_ARRAY_ALLOCATE, &args![capacity]).u32();
    // The buffer after the allocate slot; the move is in place, so the free slot gets it too.
    let buffer = e.mem.u32(array.wrapping_add(ARRAY_BUFFER));
    move_words(e, fresh, buffer, count);
    e.vcall(array, SLOT_ARRAY_FREE, &args![buffer]);
    e.mem.set_u32(array.wrapping_add(ARRAY_BUFFER), fresh);
}

// Typed from 00b5c360
/// The next slot of an array reserved (`fastcall(array)`), its index answered. A full array grows
/// first: with no capacity, a four-element buffer from the allocate slot (+4) and the count,
/// buffer and capacity 4 set; otherwise the capacity doubles (0x400 more once above 0x400) through
/// the growth `00cb9ce0` (new capacity, count; [`grow`]).
pub fn reserve_slot(e: &mut Engine, this: Ptr) -> u32 {
    let array = this.addr();
    let count = e.mem.u32(array.wrapping_add(ARRAY_COUNT));
    let capacity = e.mem.u32(array.wrapping_add(ARRAY_CAPACITY));
    if count == capacity {
        if capacity == 0 {
            let buffer = e.vcall(array, SLOT_ARRAY_ALLOCATE, &args![4u32]).u32();
            let index = e.mem.u32(array.wrapping_add(ARRAY_COUNT));
            e.mem
                .set_u32(array.wrapping_add(ARRAY_COUNT), index.wrapping_add(1));
            e.mem.set_u32(array.wrapping_add(ARRAY_BUFFER), buffer);
            e.mem.set_u32(array.wrapping_add(ARRAY_CAPACITY), 4);
            return index;
        }
        let grown = if capacity > 0x400 {
            capacity.wrapping_add(0x400)
        } else {
            capacity.wrapping_mul(2)
        };
        typed_call!(FAMILY, 0x00cb_9ce0, grow(e, this, grown, count));
        e.mem.set_u32(array.wrapping_add(ARRAY_CAPACITY), grown);
    }
    let index = e.mem.u32(array.wrapping_add(ARRAY_COUNT));
    e.mem
        .set_u32(array.wrapping_add(ARRAY_COUNT), index.wrapping_add(1));
    index
}

// Typed from 00b5a930
/// An array's search (`thiscall(array, value, start, compare)`, `ret 0xc`): from `start` while
/// below the count (read at each step), the first index whose `cdecl` compare (the element's
/// address, the value) answers a non-zero byte, the compare skipped while the buffer is null;
/// else `0xffffffff`.
pub fn find(e: &mut Engine, this: Ptr, value: u32, start: u32, compare: u32) -> u32 {
    let array = this.addr();
    let mut index = start;
    while index < e.mem.u32(array.wrapping_add(ARRAY_COUNT)) {
        let buffer = e.mem.u32(array.wrapping_add(ARRAY_BUFFER));
        if buffer != 0 {
            let element = buffer.wrapping_add(index.wrapping_mul(4));
            if e.call(compare, &args![element, value]).u8() != 0 {
                return index;
            }
        }
        index = index.wrapping_add(1);
    }
    u32::MAX
}

// Typed from 00b5ade0
/// An array's element `index` removed (`thiscall(array, index, shrink)`, `ret 8`). Without the
/// shrink byte, or while the count is above a quarter of the capacity (capacity up to 0x400) or
/// above the capacity less 0x800 (larger; wrapping, so a capacity of 0x401 to 0x7ff shrinks),
/// the later elements move down one ([`move_words`]) and the count drops. Otherwise a smaller
/// buffer (half the capacity, or 0x400 less above 0x800) comes from the allocate slot, the
/// elements before the index and then `count - 1` from after it move into it, the old buffer goes
/// to the free slot, and the count, buffer and capacity are set.
pub fn remove_at(e: &mut Engine, this: Ptr, index: u32, shrink: u8) {
    let array = this.addr();
    let capacity = e.mem.u32(array.wrapping_add(ARRAY_CAPACITY));
    let count = e.mem.u32(array.wrapping_add(ARRAY_COUNT));
    let shrinks = shrink != 0
        && if capacity > 0x400 {
            count <= capacity.wrapping_sub(0x800)
        } else {
            count <= capacity >> 2
        };
    if !shrinks {
        let at = e
            .mem
            .u32(array.wrapping_add(ARRAY_BUFFER))
            .wrapping_add(index.wrapping_mul(4));
        let after = count.wrapping_sub(index).wrapping_sub(1);
        move_words(e, at, at.wrapping_add(4), after);
        // The move is done in place: the count read above is the one the exe decrements.
        e.mem
            .set_u32(array.wrapping_add(ARRAY_COUNT), count.wrapping_sub(1));
        return;
    }
    let smaller = if capacity > 0x800 {
        capacity.wrapping_sub(0x400)
    } else {
        capacity >> 1
    };
    let fresh = e.vcall(array, SLOT_ARRAY_ALLOCATE, &args![smaller]).u32();
    // The buffer and the count as the exe reads them after the allocate slot; the moves are done
    // in place, so nothing changes them until the free slot.
    let buffer = e.mem.u32(array.wrapping_add(ARRAY_BUFFER));
    let count = e.mem.u32(array.wrapping_add(ARRAY_COUNT));
    move_words(e, fresh, buffer, index);
    let gap = index.wrapping_mul(4);
    move_words(
        e,
        fresh.wrapping_add(gap),
        buffer.wrapping_add(gap).wrapping_add(4),
        count.wrapping_sub(1),
    );
    e.vcall(array, SLOT_ARRAY_FREE, &args![buffer]);
    let left = e.mem.u32(array.wrapping_add(ARRAY_COUNT)).wrapping_sub(1);
    e.mem.set_u32(array.wrapping_add(ARRAY_COUNT), left);
    e.mem.set_u32(array.wrapping_add(ARRAY_BUFFER), fresh);
    e.mem.set_u32(array.wrapping_add(ARRAY_CAPACITY), smaller);
}

// Typed from 00b5aea0
/// `count` elements at `index` removed from an array (`thiscall(array, index, count)`, `ret 8`):
/// the last `min(count, elements after them)` (unsigned) move into the gap ([`move_words`]), then the
/// count drops by `count`.
pub fn remove_run(e: &mut Engine, this: Ptr, index: u32, count: u32) {
    let array = this.addr();
    let size = e.mem.u32(array.wrapping_add(ARRAY_COUNT));
    let moved = size.wrapping_sub(index).wrapping_sub(count).min(count);
    let buffer = e.mem.u32(array.wrapping_add(ARRAY_BUFFER));
    move_words(
        e,
        buffer.wrapping_add(index.wrapping_mul(4)),
        buffer.wrapping_add(size.wrapping_sub(moved).wrapping_mul(4)),
        moved,
    );
    let left = e
        .mem
        .u32(array.wrapping_add(ARRAY_COUNT))
        .wrapping_sub(count);
    e.mem.set_u32(array.wrapping_add(ARRAY_COUNT), left);
}

// Typed from 00b5af80
/// An array emptied (`thiscall(array, free)`, `ret 4`): with a buffer, the count cleared, and with
/// the free byte the buffer first given to the free slot and the buffer and capacity cleared.
pub fn clear(e: &mut Engine, this: Ptr, free: u8) {
    let array = this.addr();
    let buffer = e.mem.u32(array.wrapping_add(ARRAY_BUFFER));
    if buffer == 0 {
        return;
    }
    if free != 0 {
        e.vcall(array, SLOT_ARRAY_FREE, &args![buffer]);
        e.mem.set_u32(array.wrapping_add(ARRAY_BUFFER), 0);
        e.mem.set_u32(array.wrapping_add(ARRAY_CAPACITY), 0);
    }
    e.mem.set_u32(array.wrapping_add(ARRAY_COUNT), 0);
}

// Typed from 00b5a8d0
/// `item` of a list moved to just after `after` (`thiscall(list, item, after)`, `ret 8`; nothing
/// for the same item). The head and tail pass on from `item` when they hold it, the tail becomes
/// `item` when it is `after`; `item` is unlinked from its neighbours and linked in after `after`.
/// Each word is read where the exe reads it (the links may alias).
pub fn move_after(e: &mut Engine, this: Ptr<NiTPointerList>, item: u32, after: u32) {
    if item == after {
        return;
    }
    let list = this.addr();
    let tail = list.wrapping_add(4);
    if e.mem.u32(list) == item {
        let next = next_of(e, item);
        e.mem.set_u32(list, next);
    }
    if e.mem.u32(tail) == item {
        let previous = e.mem.u32(item.wrapping_add(ITEM_PREVIOUS));
        e.mem.set_u32(tail, previous);
    }
    if e.mem.u32(tail) == after {
        e.mem.set_u32(tail, item);
    }
    let next = next_of(e, item);
    if next != 0 {
        let previous = e.mem.u32(item.wrapping_add(ITEM_PREVIOUS));
        e.mem.set_u32(next.wrapping_add(ITEM_PREVIOUS), previous);
    }
    let previous = e.mem.u32(item.wrapping_add(ITEM_PREVIOUS));
    if previous != 0 {
        let next = next_of(e, item);
        e.mem.set_u32(previous.wrapping_add(ITEM_NEXT), next);
    }
    let following = next_of(e, after);
    e.mem.set_u32(item.wrapping_add(ITEM_NEXT), following);
    e.mem.set_u32(item.wrapping_add(ITEM_PREVIOUS), after);
    if following != 0 {
        e.mem.set_u32(following.wrapping_add(ITEM_PREVIOUS), item);
    }
    e.mem.set_u32(after.wrapping_add(ITEM_NEXT), item);
}

// Typed from 00b5a9d0
/// A light's `bPointPosition` set from a point by value (`thiscall(light, x, y, z)`, `ret 0xc`).
pub fn set_point_position(e: &mut Engine, this: Ptr, x: u32, y: u32, z: u32) {
    let at = this
        .addr()
        .wrapping_add(ShadowSceneLight::bPointPosition.off);
    e.mem.set_u32(at, x);
    e.mem.set_u32(at.wrapping_add(4), y);
    e.mem.set_u32(at.wrapping_add(8), z);
}

// Typed from 00b5aee0
/// The `NiPointer` assignment of a light's `spDynamicShadowMap` (`thiscall(light, map)`, `ret 4`).
pub fn set_shadow_map(e: &mut Engine, this: Ptr, map: u32) {
    let slot = this
        .addr()
        .wrapping_add(ShadowSceneLight::spDynamicShadowMap.off);
    assign(e, slot, map);
}

// Typed from 00b5af30
/// The `NiPointer` assignment of a light's `spShadowScene` (`thiscall(light, object)`, `ret 4`).
pub fn set_shadow_scene(e: &mut Engine, this: Ptr, object: u32) {
    let slot = this
        .addr()
        .wrapping_add(ShadowSceneLight::spShadowScene.off);
    assign(e, slot, object);
}

// Typed from 00b5aa70
/// The sort comparison of two lights by the float at +0xc (`cdecl`, two pointers to the lights'
/// pointers): -1 when the second's is smaller, 1 when it is greater, 0 when they are equal or
/// unordered (the exe's `FCOM` then `FCOMPP` of the two loaded floats).
pub fn compare_luminance(e: &mut Engine, first: Ptr, second: Ptr) -> i32 {
    let a = e.mem.f32(e.mem.u32(first.addr()).wrapping_add(0xc));
    let b = e.mem.f32(e.mem.u32(second.addr()).wrapping_add(0xc));
    if b < a {
        -1
    } else if b > a {
        1
    } else {
        0
    }
}

// Typed from 00b5aa40
/// A node's child at its index word (`fastcall(node)`): the child when the index (+0xb0) is not
/// negative and below the children's used size (signed), else 0.
pub fn indexed_child(e: &mut Engine, this: Ptr) -> u32 {
    let node = this.addr();
    let index = e.mem.i32(node.wrapping_add(CHILD_INDEX));
    if index < 0 || index >= i32::from(e.mem.u16(node.wrapping_add(CHILDREN_SIZE))) {
        return 0;
    }
    let children = e.mem.u32(node.wrapping_add(CHILDREN_BASE));
    e.mem
        .u32(children.wrapping_add(index.unsigned_abs().wrapping_mul(4)))
}

// Typed from 00b5e590
/// `ShadowSceneNode::GetRTTI` (its vtable's slot +8): the class's `NiRTTI` record.
pub fn get_rtti(_e: &mut Engine) -> Ptr {
    Ptr::new(SHADOW_SCENE_NODE_RTTI)
}

// Typed from 00b5ac60
/// The number of lights of `lLightList` whose `usFrustumCull` is not 0xff (`fastcall(node)`; null
/// elements not counted).
pub fn count_frustum_lights(e: &mut Engine, this: Ptr) -> u32 {
    let mut item = head_of(e, this.addr(), LIGHT_LIST);
    let mut count = 0u32;
    while item != 0 {
        let light = element_of(e, item);
        item = next_of(e, item);
        if light != 0
            && e.mem
                .u16(light.wrapping_add(ShadowSceneLight::usFrustumCull.off))
                != 0xff
        {
            count = count.wrapping_add(1);
        }
    }
    count
}

// Typed from 00b5b4a0
/// `ShadowSceneNode::GetLight` (Xbox PDB, `thiscall(node, NiLight)`, `ret 4`): the light of
/// `lLightList` whose `spLight` is that `NiLight`, or 0.
pub fn get_light(e: &mut Engine, this: Ptr, ni_light: u32) -> u32 {
    light_of(e, this.addr(), ni_light)
}

// Typed from 00b5c450
/// The shadow flag of the light with an `NiLight` (`thiscall(node, NiLight, cast)`, `ret 8`):
/// `GetLight`'s walk inlined, then the light's `SetCastShadow` (`00b9dcb0`) with the word's low
/// byte (the exe pushes the whole word, of which the callee reads that byte: by address only the
/// byte goes on); nothing when there is none.
pub fn set_light_cast_shadow(e: &mut Engine, this: Ptr, ni_light: u32, cast: u32) {
    let light = light_of(e, this.addr(), ni_light);
    if light != 0 {
        typed_call!(
            lights::FAMILY,
            0x00b9_dcb0,
            scene_light::set_cast_shadow(e, Ptr::new(light), cast.to_le_bytes()[0])
        );
    }
}

// Typed from 00b5d270
/// A light queued for an update (`thiscall(node, light)`, `ret 4`): appended to
/// `lLightQueueUpdate` through an `NiPointer` temporary; nothing for none.
pub fn queue_update(e: &mut Engine, this: Ptr, light: u32) {
    if light != 0 {
        add_tail_held(
            e,
            this.addr().wrapping_add(LIGHT_QUEUE_UPDATE),
            light,
            false,
        );
    }
}

// Typed from 00b5f080
/// The light with an `NiLight` queued for an update (`thiscall(node, NiLight)`, `ret 4`):
/// `GetLight`'s walk inlined, then a jump to `00b5d270`; nothing when there is none.
pub fn queue_update_of(e: &mut Engine, this: Ptr, ni_light: u32) {
    let light = light_of(e, this.addr(), ni_light);
    if light != 0 {
        typed_call!(FAMILY, 0x00b5_d270, queue_update(e, this, light));
    }
}

// Typed from 00b5cff0
/// A light queued for removal (`thiscall(node, pointer)`, `ret 4`: an `NiPointer`'s address). When
/// it holds a light and `bAllowLightRemoveQueues` is set: under the light queues' lock the pointer
/// is appended to `lLightQueueRemove`.
pub fn queue_removal(e: &mut Engine, this: Ptr, pointer: Ptr) {
    let node = this.addr();
    if e.mem.u32(pointer.addr()) == 0 || e.mem.u8(node.wrapping_add(ALLOW_REMOVE_QUEUES)) == 0 {
        return;
    }
    lock_queues(e);
    let list = node.wrapping_add(LIGHT_QUEUE_REMOVE);
    typed_call!(
        cell_refs::FAMILY,
        0x00a5_a310,
        cell_refs::lighting::list_add_tail(e, list, pointer.addr())
    );
    unlock_queues(e);
}

/// `00b5edf0`'s and `RemoveLight`'s queueing of a found light for removal: `00b5cff0` through an
/// `NiPointer` temporary.
fn queue_removal_held(e: &mut Engine, this: Ptr, light: u32) {
    e.with_stack(4, |e, slot| {
        e.mem.set_u32(slot.addr(), light);
        hold(e, light);
        typed_call!(FAMILY, 0x00b5_cff0, queue_removal(e, this, slot));
        release(e, light);
    });
}

// Typed from 00b5edf0
/// The shadow light of an object queued for removal (`thiscall(node, object)`, `ret 4`): the first
/// light of `lShadowLightList` whose `spShadowScene` is the object goes to `00b5cff0`; nothing for
/// no object or no such light.
pub fn queue_removal_of_scene(e: &mut Engine, this: Ptr, object: u32) {
    if object == 0 {
        return;
    }
    let mut item = head_of(e, this.addr(), SHADOW_LIGHT_LIST);
    while item != 0 {
        let light = element_of(e, item);
        item = next_of(e, item);
        if light != 0
            && e.mem
                .u32(light.wrapping_add(ShadowSceneLight::spShadowScene.off))
                == object
        {
            queue_removal_held(e, this, light);
            return;
        }
    }
}

// Typed from 00b5eed0
/// `ShadowSceneNode::RemoveLight` (Xbox PDB, `thiscall(node, NiLight)`, `ret 4`): with
/// `bAllowLightRemoveQueues` set, the light of `lLightList` with that `NiLight` goes to
/// `00b5cff0`; nothing for none.
pub fn remove_light(e: &mut Engine, this: Ptr, ni_light: u32) {
    if ni_light == 0 || e.mem.u8(this.addr().wrapping_add(ALLOW_REMOVE_QUEUES)) == 0 {
        return;
    }
    let light = light_of(e, this.addr(), ni_light);
    if light != 0 {
        queue_removal_held(e, this, light);
    }
}

// Typed from 00b5d180
/// Every light of `lLightList` queued for removal (`thiscall(node)`): each held by an `NiPointer`
/// temporary and, while `bAllowLightRemoveQueues` is set, appended to `lLightQueueRemove` under the
/// light queues' lock. Then the shadow maps are dropped (`00b5c3c0`), `lShadowLightList` emptied,
/// both fences cleared and the maps dropped again.
pub fn queue_all_removals(e: &mut Engine, this: Ptr) {
    let node = this.addr();
    let mut item = head_of(e, node, LIGHT_LIST);
    while item != 0 {
        let light = element_of(e, item);
        item = next_of(e, item);
        if light == 0 {
            continue;
        }
        e.with_stack(4, |e, slot| {
            e.mem.set_u32(slot.addr(), light);
            hold(e, light);
            if e.mem.u8(node.wrapping_add(ALLOW_REMOVE_QUEUES)) != 0 {
                lock_queues(e);
                let list = node.wrapping_add(LIGHT_QUEUE_REMOVE);
                typed_call!(
                    cell_refs::FAMILY,
                    0x00a5_a310,
                    cell_refs::lighting::list_add_tail(e, list, slot.addr())
                );
                unlock_queues(e);
            }
            release(e, light);
        });
    }
    typed_call!(
        setup::FAMILY,
        0x00b5_c3c0,
        setup::lists::drop_shadow_maps(e, this)
    );
    typed_call!(
        FAMILY,
        0x00e7_4d40,
        remove_all(e, Ptr::new(node.wrapping_add(SHADOW_LIGHT_LIST)))
    );
    e.mem.set_u32(node.wrapping_add(ACTIVE_FENCE), 0);
    e.mem.set_u32(node.wrapping_add(FADING_FENCE), 0);
    typed_call!(
        setup::FAMILY,
        0x00b5_c3c0,
        setup::lists::drop_shadow_maps(e, this)
    );
}

// Typed from 00b5ddb0
/// The node's lights queued for updates (`thiscall(node, each, keep)`, `ret 8`): with the first
/// byte, each light of `lLightList` goes to `00b5d270`, then unless the second byte is set every
/// light is queued for removal (`00b5d180`). Nothing without the first byte.
pub fn queue_all_updates(e: &mut Engine, this: Ptr, each: u8, keep: u8) {
    if each == 0 {
        return;
    }
    let mut item = head_of(e, this.addr(), LIGHT_LIST);
    while item != 0 {
        let light = element_of(e, item);
        item = next_of(e, item);
        if light != 0 {
            typed_call!(FAMILY, 0x00b5_d270, queue_update(e, this, light));
        }
    }
    if keep == 0 {
        typed_call!(FAMILY, 0x00b5_d180, queue_all_removals(e, this));
    }
}

// Typed from 00b5c940
/// `ShadowSceneNode::AddLight` (Xbox PDB, `thiscall(node, NiLight, dynamic)`, `ret 8`): only for a
/// light whose RTTI is exactly the radius-free light's record or `NiPointLight`'s (the slot asked
/// again for the second); else 0. Under the light queues' lock a new `ShadowSceneLight` (null when
/// the allocation fails) is set to the light, gets the byte as `bDynamicLight`, is appended to
/// `lLightQueueAdd` through an `NiPointer` temporary and takes the node's portal graph. Answers the
/// new light.
pub fn add_light(e: &mut Engine, this: Ptr, light: u32, dynamic: u8) -> u32 {
    if light == 0 {
        return 0;
    }
    if slot_word(e, light, SLOT_RTTI) != RADIUS_FREE_LIGHT_RTTI
        && slot_word(e, light, SLOT_RTTI) != POINT_LIGHT_RTTI
    {
        return 0;
    }
    lock_queues(e);
    let memory = e.call(NEW_OBJECT, &args![SHADOW_SCENE_LIGHT_SIZE]).u32();
    let shadow_light = if memory != 0 {
        typed_call!(
            lights::FAMILY,
            0x00b9_fda0,
            scene_light::construct(e, Ptr::new(memory))
        )
        .addr()
    } else {
        0
    };
    typed_call!(
        lights::FAMILY,
        0x00b9_dda0,
        scene_light::set_light(e, Ptr::new(shadow_light), Ptr::new(light))
    );
    e.mem.set_u8(
        shadow_light.wrapping_add(ShadowSceneLight::bDynamicLight.off),
        dynamic,
    );
    let node = this.addr();
    add_tail_held(e, node.wrapping_add(LIGHT_QUEUE_ADD), shadow_light, false);
    let graph = e.mem.u32(node.wrapping_add(PORTAL_GRAPH));
    e.mem
        .set_u32(shadow_light.wrapping_add(LIGHT_PORTAL_GRAPH), graph);
    unlock_queues(e);
    shadow_light
}

// Typed from 00b5ca70
/// `ShadowSceneNode::AddAllLightsFromNode_Recurse` (Xbox PDB, `thiscall(node, parent, radius)`,
/// `ret 8`): each child of the parent (the used size read at each step): a node (its `IsNode`) is
/// walked the same way, its lights counted; a light (kind of `NiLight`) is added (`AddLight`, its
/// dynamic byte 1) and updated (`00a59c60`, a zero float and five zero bytes), and when it casts
/// to `NiPointLight` its word +0xa8 goes up and its three floats at +0xe0 become the radius and two
/// zeros. Answers the number of lights.
pub fn add_all_lights_from_node(e: &mut Engine, this: Ptr, parent: u32, radius: f32) -> u32 {
    // The radius as the exe's `FLD`/`FSTP` passes and stores it.
    let radius = x87_quiet_bits(radius.to_bits());
    let mut found = 0u32;
    let mut index = 0u32;
    while index < u32::from(e.mem.u16(parent.wrapping_add(CHILDREN_SIZE))) {
        let children = e.mem.u32(parent.wrapping_add(CHILDREN_BASE));
        let child = e.mem.u32(children.wrapping_add(index.wrapping_mul(4)));
        index = index.wrapping_add(1);
        if child == 0 {
            continue;
        }
        let node = slot_word(e, child, SLOT_IS_NODE);
        if node != 0 {
            let lights = typed_call!(
                FAMILY,
                0x00b5_ca70,
                add_all_lights_from_node(e, this, node, f32::from_bits(radius))
            );
            found = found.wrapping_add(lights);
            continue;
        }
        if !is_kind_of(e, child, LIGHT_RTTI) {
            continue;
        }
        typed_call!(FAMILY, 0x00b5_c940, add_light(e, this, child, 1));
        e.with_stack(0xc, |e, data| {
            e.mem.set_u32(data.addr(), 0);
            for byte in 4..9u32 {
                e.mem.set_u8(data.addr().wrapping_add(byte), 0);
            }
            typed_call!(
                scene::FAMILY,
                0x00a5_9c60,
                scene::walk::update(e, Ptr::new(child), data.addr())
            );
        });
        let point = typed_call!(
            dispatch::FAMILY,
            0x0065_3290,
            dispatch::scene::cast(e, Ptr::new(child), POINT_LIGHT_RTTI)
        )
        .addr();
        if point != 0 {
            let count = e.mem.u32(point.wrapping_add(0xa8)).wrapping_add(1);
            e.mem.set_u32(point.wrapping_add(0xa8), count);
            e.mem.set_u32(point.wrapping_add(0xe0), radius);
            e.mem.set_u32(point.wrapping_add(0xe4), 0);
            e.mem.set_u32(point.wrapping_add(0xe8), 0);
        }
        found = found.wrapping_add(1);
    }
    found
}

// Typed from 00b5f0c0
/// `ShadowSceneNode::FindAndCullLightRecurse` (engine map, `thiscall(node, parent, remove)`, `ret
/// 8`): each child below the parent's used size (read once; the children's buffer read at each
/// step): a light (kind of `NiLight`) is removed (`RemoveLight`) with the word's low byte set, or
/// added (`AddLight` with 1); any other child that is a node (`IsNode`) is walked the same way
/// with the whole word.
pub fn find_and_cull_light_recurse(e: &mut Engine, this: Ptr, parent: u32, remove: u32) {
    let size = u32::from(e.mem.u16(parent.wrapping_add(CHILDREN_SIZE)));
    for index in 0..size {
        let children = e.mem.u32(parent.wrapping_add(CHILDREN_BASE));
        let child = e.mem.u32(children.wrapping_add(index.wrapping_mul(4)));
        if child == 0 {
            continue;
        }
        if is_kind_of(e, child, LIGHT_RTTI) {
            if remove & 0xff != 0 {
                typed_call!(FAMILY, 0x00b5_eed0, remove_light(e, this, child));
            } else {
                typed_call!(FAMILY, 0x00b5_c940, add_light(e, this, child, 1));
            }
            continue;
        }
        if slot_word(e, child, SLOT_IS_NODE) != 0 {
            typed_call!(
                FAMILY,
                0x00b5_f0c0,
                find_and_cull_light_recurse(e, this, child, remove)
            );
        }
    }
}

// Typed from 00b5f150
/// `ShadowSceneNode::FindAndCullLight` (engine map, `thiscall(node, parent, remove)`, `ret 8`): a
/// jump to `FindAndCullLightRecurse` for a parent; nothing for none.
pub fn find_and_cull_light(e: &mut Engine, this: Ptr, parent: u32, remove: u32) {
    if parent != 0 {
        typed_call!(
            FAMILY,
            0x00b5_f0c0,
            find_and_cull_light_recurse(e, this, parent, remove)
        );
    }
}

// Typed from 00b5cd00
/// A shadow light made dynamic (`thiscall(node, light)`, `ret 4`): `bDynamicLight` set; a new
/// `NiLight` (`00a7d670`; 0xfc bytes, or the size JIP's H03 answers at its seam; null when the
/// allocation fails) with bit 0 of its flags cleared is set to it (`SetLight`), its shadow flag set
/// (`00b9dcb0` with 1), `bCubeShadowMap` cleared, and it is appended to `lShadowLightList` through
/// an `NiPointer` temporary. Answers the light.
pub fn make_dynamic(e: &mut Engine, this: Ptr, light: u32) -> u32 {
    e.mem
        .set_u8(light.wrapping_add(ShadowSceneLight::bDynamicLight.off), 1);
    let size = if e.jip_patches() & engine_core::extend::jip_patches::INSTALLED != 0 {
        e.site(0x00b5_cd2a, &[]).unwrap_or(NI_LIGHT_SIZE)
    } else {
        NI_LIGHT_SIZE
    };
    let memory = e.call(NEW_OBJECT, &args![size]).u32();
    let ni_light = if memory != 0 {
        e.call(NI_LIGHT_CONSTRUCT, &args![memory]).u32()
    } else {
        0
    };
    let flags = e.mem.u32(ni_light.wrapping_add(AV_FLAGS));
    e.mem.set_u32(ni_light.wrapping_add(AV_FLAGS), flags & !1);
    typed_call!(
        lights::FAMILY,
        0x00b9_dda0,
        scene_light::set_light(e, Ptr::new(light), Ptr::new(ni_light))
    );
    typed_call!(
        lights::FAMILY,
        0x00b9_dcb0,
        scene_light::set_cast_shadow(e, Ptr::new(light), 1)
    );
    e.mem
        .set_u8(light.wrapping_add(ShadowSceneLight::bCubeShadowMap.off), 0);
    add_tail_held(e, this.addr().wrapping_add(SHADOW_LIGHT_LIST), light, false);
    light
}

// Typed from 00b5d930
/// A moved dynamic point light followed (`thiscall(node, light)`, `ret 4`): only for a dynamic point
/// light without `bCastShadow`. With its `NiLight` culled (bit 0) the light's cull `00b9ef30` runs;
/// otherwise, when `bPointPosition` differs from the `NiLight`'s world position (`NiPoint3`'s `!=`,
/// in place: a component unequal or unordered) and `usFrustumCull` is not 0xff, the position is
/// copied (`00b5a9d0`) and the light updated (`00b5d300`).
pub fn follow_moved_light(e: &mut Engine, this: Ptr, light: u32) {
    if e.mem
        .u8(light.wrapping_add(ShadowSceneLight::bPointLight.off))
        == 0
        || e.mem
            .u8(light.wrapping_add(ShadowSceneLight::bDynamicLight.off))
            == 0
        || e.mem
            .u8(light.wrapping_add(ShadowSceneLight::bCastShadow.off))
            != 0
    {
        return;
    }
    let ni_light = ni_light_of(e, light);
    if e.mem.u32(ni_light.wrapping_add(AV_FLAGS)) & 1 != 0 {
        e.call(LIGHT_CULL, &args![light]);
        return;
    }
    let point = light.wrapping_add(ShadowSceneLight::bPointPosition.off);
    let world = ni_light.wrapping_add(WORLD_TRANSLATE);
    let [x, y, z] = [0u32, 4, 8].map(|i| e.mem.u32(world.wrapping_add(i)));
    let same = [0u32, 4, 8]
        .iter()
        .all(|&i| e.mem.f32(point.wrapping_add(i)) == e.mem.f32(world.wrapping_add(i)));
    if same
        || e.mem
            .u16(light.wrapping_add(ShadowSceneLight::usFrustumCull.off))
            == 0xff
    {
        return;
    }
    typed_call!(
        FAMILY,
        0x00b5_a9d0,
        set_point_position(e, Ptr::new(light), x, y, z)
    );
    e.call(LIGHT_UPDATE, &args![this, light]);
}

// Typed from 00b5b1f0
/// Every shadow map dropped (`fastcall(node)`): for each light of `lShadowLightList`, its dynamic
/// shadow map goes to the renderer's release (`00b6d4c0`) and, read again, is let go and cleared;
/// then a jump to the renderer's `00b6d130`.
pub fn drop_all_shadow_maps(e: &mut Engine, this: Ptr) {
    let map_at = ShadowSceneLight::spDynamicShadowMap.off;
    let mut item = head_of(e, this.addr(), SHADOW_LIGHT_LIST);
    while item != 0 {
        let light = element_of(e, item);
        item = next_of(e, item);
        if light == 0 {
            continue;
        }
        let map = e.mem.u32(light.wrapping_add(map_at));
        let renderer = e.mem.u32(RENDERER);
        e.call(RENDERER_RELEASE_MAP, &args![renderer, map]);
        let map = e.mem.u32(light.wrapping_add(map_at));
        if map != 0 {
            release(e, map);
            e.mem.set_u32(light.wrapping_add(map_at), 0);
        }
    }
    let renderer = e.mem.u32(RENDERER);
    e.call(RENDERER_MAPS_DROPPED, &args![renderer]);
}

// Typed from 00b5b0e0
/// `ShadowSceneNode::RemoveGeometryFromPropertyLights` (Xbox PDB, `cdecl(geometry)`): the
/// geometry's lighting property (the word through +0x9c's +0xc, else +0xa8; of type 1 to 12) has
/// each of its lights (`00b70490`, its iterator the function's own argument word) remove the
/// geometry (`RemoveTriGeom` with 0). Nothing for no geometry or property.
pub fn remove_geometry_from_property_lights(e: &mut Engine, geometry: u32) {
    if geometry == 0 {
        return;
    }
    let base = geometry.wrapping_add(GEOMETRY_PROPERTY_BASE);
    let mut property = if base != 0 {
        e.mem.u32(base.wrapping_add(0xc))
    } else {
        0
    };
    if property == 0 {
        property = e.mem.u32(geometry.wrapping_add(GEOMETRY_PROPERTY));
    }
    if property == 0
        || e.mem
            .u32(property.wrapping_add(PROPERTY_TYPE))
            .wrapping_sub(1)
            > 0xb
    {
        return;
    }
    e.with_stack(4, |e, iterator| {
        e.mem.set_u32(iterator.addr(), geometry);
        loop {
            let light = e
                .call(PROPERTY_NEXT_LIGHT, &args![property, iterator])
                .u32();
            if light == 0 {
                break;
            }
            typed_call!(
                lights::FAMILY,
                0x00ba_01a0,
                scene_light::remove_tri_geom(e, Ptr::new(light), geometry, 0)
            );
        }
    });
}

// Typed from 00b5b150
/// `ShadowSceneNode::RemoveObjectRecurse` (Xbox PDB, `thiscall(node, parent)`, `ret 4`): each child
/// of the parent (the used size read at each step) that is a tri-based geometry is removed from its
/// property's lights (`00b5b0e0`); one that is a node (its `IsNode`) is walked the same way.
pub fn remove_object_recurse(e: &mut Engine, this: Ptr, parent: u32) {
    let mut index = 0u32;
    while index < u32::from(e.mem.u16(parent.wrapping_add(CHILDREN_SIZE))) {
        let children = e.mem.u32(parent.wrapping_add(CHILDREN_BASE));
        let child = e.mem.u32(children.wrapping_add(index.wrapping_mul(4)));
        index = index.wrapping_add(1);
        if child == 0 {
            continue;
        }
        if slot_word(e, child, SLOT_IS_TRI_BASED_GEOM) != 0 {
            typed_call!(
                FAMILY,
                0x00b5_b0e0,
                remove_geometry_from_property_lights(e, child)
            );
            continue;
        }
        let node = slot_word(e, child, SLOT_IS_NODE);
        if node != 0 {
            typed_call!(FAMILY, 0x00b5_b150, remove_object_recurse(e, this, node));
        }
    }
}

// Typed from 00b5b1c0
/// `ShadowSceneNode::RemoveObject` (Xbox PDB, `thiscall(node, object)`, `ret 4`): an object that is
/// a node (its `IsNode`) goes to `RemoveObjectRecurse`.
pub fn remove_object(e: &mut Engine, this: Ptr, object: u32) {
    if object == 0 {
        return;
    }
    let node = slot_word(e, object, SLOT_IS_NODE);
    if node != 0 {
        typed_call!(FAMILY, 0x00b5_b150, remove_object_recurse(e, this, node));
    }
}

// Typed from 00b5c6e0
/// An object appended to the `NiPointer` list at +0x30 unless it is in it already (`thiscall(graph,
/// object)`, `ret 4`; the portal graph's rooms, from `00b5f170`): the object held while the list is
/// searched and let go after; a missing one appended through an `NiPointer` temporary.
pub fn add_unique(e: &mut Engine, this: Ptr, object: u32) {
    if object != 0 {
        hold(e, object);
    }
    let list = this.addr().wrapping_add(0x30);
    let mut item = e.mem.u32(list);
    let mut missing = true;
    while item != 0 {
        if element_of(e, item) == object {
            missing = false;
            break;
        }
        item = next_of(e, item);
    }
    if object != 0 {
        release(e, object);
    }
    if missing {
        add_tail_held(e, list, object, true);
    }
}

/// The pool the list items go back to (`NiTPointerAllocator`'s): its lock (owner `011c5f80`,
/// count `011c5f84`) and its free item head (`011c5f58`).
const ITEM_POOL_LOCK: u32 = 0x011c_5f80;
const ITEM_POOL_COUNT: u32 = 0x011c_5f84;
const ITEM_POOL_FREE: u32 = 0x011c_5f58;

// Typed from 00e74d40
/// `NiTPointerListBase<NiTPointerAllocator<unsigned int>, NiPointer<..>>::RemoveAll` (engine map,
/// `thiscall(list)`): for each item from the head (its next read first): its `NiPointer` let go
/// and cleared when set; under the item pool's lock (`0040fbf0` with 0) the item's previous word
/// cleared and the item pushed on the pool's free list, the lock given back as the exe inlines it
/// (the count down, the free head stored, at zero no owner). Then the count, head and tail
/// cleared.
pub fn remove_all(e: &mut Engine, this: Ptr<NiTPointerList>) {
    let list = this.addr();
    let mut item = e.mem.u32(list);
    while item != 0 {
        let element = element_of(e, item);
        let next = next_of(e, item);
        if element != 0 {
            release(e, element);
            e.mem.set_u32(item.wrapping_add(ITEM_ELEMENT), 0);
        }
        typed_call!(
            dispatch::FAMILY,
            0x0040_fbf0,
            dispatch::refs::lock(e, Ptr::<LightLock>::new(ITEM_POOL_LOCK), 0u32)
        );
        e.mem.set_u32(item.wrapping_add(ITEM_PREVIOUS), 0);
        let free = e.mem.u32(ITEM_POOL_FREE);
        e.mem.set_u32(item.wrapping_add(ITEM_NEXT), free);
        let count = e.mem.u32(ITEM_POOL_COUNT).wrapping_sub(1);
        e.mem.set_u32(ITEM_POOL_COUNT, count);
        e.mem.set_u32(ITEM_POOL_FREE, item);
        if count == 0 {
            // The exe's plain store of the owner gives the lock back: a release store, so the
            // free list's words are seen before the lock is free.
            e.mem.set_u32_release(ITEM_POOL_LOCK, 0);
        }
        item = next;
    }
    e.mem.set_u32(list.wrapping_add(8), 0);
    e.mem.set_u32(list, 0);
    e.mem.set_u32(list.wrapping_add(4), 0);
}
