//! The part 5 file's small templates (`0042f4c0` to `0042ffd0`): the one-word counted pointer of
//! the navmesh array (`NavMeshPtr`: its constructors, destructor, assignment and compare) and that
//! array (`BSSimpleArray<NavMeshPtr,1024>`: its constructors, destructors, clear, add, removal at
//! an index, the buffer's move and its elements' construction, release and move), the shared
//! array's set size (`0042f5f0`), `NiPointer<ActorCause>`'s constructor, destructor and assignment,
//! the `NiTMap<TESObjectREFR *, bool>` and its base's constructors and destructors, the bound
//! object array's (`BSSimpleArray<TESBoundObject *,1024>`) constructor and destructors, an unsigned
//! `min` and the memory manager's reallocation.
//!
//! The array is the shared `BSSimpleArray`: its vtable (slot 1 allocates a buffer, slot 3
//! reallocates one), the buffer (+4), the size (+8) and the capacity (+0xc). The counted pointer
//! holds an object whose reference count is at +0x1c (+4 of the `NiRefObject` there): taken and
//! released through the dispatch family's `0040f6e0` and `00401970`. The pathing family's array
//! free, bound object array body and shrink test (`006a8500`, `006b3eb0`, `006f3170`) are typed
//! calls. The file's one-instruction leaves (placement `new`, the counted pointer's null
//! constructor, the compare of the words two pointers hold, a 4-byte `memmove`, the memory
//! manager's address) are done in place; its other functions call one another as typed calls. The
//! array's allocate and reallocate stay calls through the vtable (each array class has its own);
//! `operator delete`, the actor cause's reference calls, the map's empty-all, its table's
//! allocation and free, the elements' constructions and destructions of the shared array, the
//! array's add-a-slot and the capacity it shrinks to, and the memory manager's reallocation (an
//! overridable function) are called by address.

use super::{dispatch, free_when, zero_bytes, FAMILY};
use crate::pathing;
use engine_core::ptr::Ptr;
use engine_core::{args, typed_call, Engine};

/// The array's words: buffer, size, capacity; its vtable's allocate and reallocate slots.
const BUFFER: u32 = 4;
const SIZE: u32 = 8;
const CAPACITY: u32 = 0xc;
const SLOT_ALLOCATE: u32 = 4;
const SLOT_REALLOCATE: u32 = 0xc;
/// Where the counted pointer's object keeps its `NiRefObject` (the count at its +4).
const REFERENCE_AT: u32 = 0x1c;

/// The vtables the constructors and destructors set.
const VTABLE_NAVMESH_ARRAY: u32 = 0x0101_59d4;
const VTABLE_REFR_MAP: u32 = 0x0101_59e8;
const VTABLE_BOUND_OBJECT_ARRAY: u32 = 0x0101_5a08;
const VTABLE_REFR_MAP_BASE: u32 = 0x0101_5a1c;

/// The memory manager (`00401020` answers its address) and its reallocation (`thiscall(manager,
/// block, size)`, overridable).
const MEMORY_MANAGER: u32 = 0x011f_6238;
const MEMORY_MANAGER_REALLOCATE: u32 = 0x00aa_4150;
/// The actor cause's reference taken (`0044bbc0`, `thiscall(object)`) and released (`0066e2b0`).
const ACTOR_CAUSE_ADD_REFERENCE: u32 = 0x0044_bbc0;
const ACTOR_CAUSE_RELEASE: u32 = 0x0066_e2b0;
/// The shared array's members (`thiscall(array, ...)`): its elements constructed (`(address,
/// count)`), destroyed (`(address, count)`), a slot made for one more (answers its index), whether
/// it is sparse enough to shrink (a byte) and the capacity it shrinks to.
const ELEMENTS_CONSTRUCT: u32 = 0x0043_0000;
const ELEMENTS_DESTROY: u32 = 0x0072_ba80;
const ADD_SLOT: u32 = 0x0076_1540;
const SHRUNK_CAPACITY: u32 = 0x0086_9600;
/// The map's empty-all (`thiscall(map)`), its table's allocation (`cdecl(bytes)`) and free
/// (`cdecl(block)`).
const MAP_REMOVE_ALL: u32 = 0x0043_8af0;
const TABLE_ALLOCATE: u32 = 0x00aa_1070;
const TABLE_FREE: u32 = 0x00aa_10f0;

// ---- What the file shares --------------------------------------------------------------------

/// The array's word at `offset`.
#[inline]
fn word(e: &Engine, this: u32, offset: u32) -> u32 {
    e.mem.u32(this.wrapping_add(offset))
}

/// The array's word at `offset` set.
#[inline]
fn set_word(e: &mut Engine, this: u32, offset: u32, value: u32) {
    e.mem.set_u32(this.wrapping_add(offset), value);
}

/// Element `index` of the buffer at `buffer` (pointer-sized elements).
#[inline]
fn slot_at(buffer: u32, index: u32) -> u32 {
    buffer.wrapping_add(index.wrapping_mul(4))
}

/// A buffer of `capacity` elements from the array's vtable (slot 1).
#[inline]
fn new_buffer(e: &mut Engine, this: u32, capacity: u32) -> u32 {
    e.vcall(this, SLOT_ALLOCATE, &args![capacity]).u32()
}

/// The array's buffer freed (`006a8500`, the pathing family's: the vtable's free slot, then a null
/// buffer).
#[inline]
fn free_buffer(e: &mut Engine, this: u32) {
    typed_call!(
        pathing::FAMILY,
        0x006a_8500,
        pathing::search::free_data(e, Ptr::new(this))
    );
}

/// A reference taken on the counted pointer's object (`0040f6e0` on its +0x1c).
#[inline]
fn take_reference(e: &mut Engine, object: u32) {
    typed_call!(
        dispatch::FAMILY,
        0x0040_f6e0,
        dispatch::refs::add_reference(e, Ptr::new(object.wrapping_add(REFERENCE_AT)))
    );
}

/// A reference released from the counted pointer's object (`00401970` on its +0x1c).
#[inline]
fn give_reference(e: &mut Engine, object: u32) {
    typed_call!(
        dispatch::FAMILY,
        0x0040_1970,
        dispatch::refs::release(e, Ptr::new(object.wrapping_add(REFERENCE_AT)))
    );
}

// ---- The functions --------------------------------------------------------------------------

// Typed from 0042f4c0
/// An array element assigned from the counted pointer at `source` ([`navmesh_ptr_assign`]).
pub fn element_assign(e: &mut Engine, this: u32, source: u32) {
    typed_call!(FAMILY, 0x0042_fdc0, navmesh_ptr_assign(e, this, source));
}

// Typed from 0042f4e0
/// `NiTMap<TESObjectREFR *, bool>`'s constructor: its base's ([`refr_map_base_init`]) with
/// `hash_size`, then its vtable. Answers `this`.
pub fn refr_map_new(e: &mut Engine, this: u32, hash_size: u32) -> u32 {
    typed_call!(FAMILY, 0x0042_fe10, refr_map_base_init(e, this, hash_size));
    e.mem.set_u32(this, VTABLE_REFR_MAP);
    this
}

// Typed from 0042f510
/// `BSSimpleArray<NavMeshPtr,1024>`'s scalar deleting destructor: [`navmesh_array_drop`], then
/// `operator delete` for bit 0 of `flags`. Answers `this`.
pub fn navmesh_array_delete(e: &mut Engine, this: u32, flags: u32) -> u32 {
    typed_call!(FAMILY, 0x0042_f830, navmesh_array_drop(e, this));
    free_when(e, this, flags)
}

// Typed from 0042f540
/// `NiTMap<TESObjectREFR *, bool>`'s scalar deleting destructor: [`refr_map_drop`], then `operator
/// delete` for bit 0 of `flags`. Answers `this`.
pub fn refr_map_delete(e: &mut Engine, this: u32, flags: u32) -> u32 {
    typed_call!(FAMILY, 0x0042_fe80, refr_map_drop(e, this));
    free_when(e, this, flags)
}

// Typed from 0042f570
/// `BSSimpleArray<TESBoundObject *,1024>`'s constructor: its vtable, then the array's body with no
/// capacity and no size (`006b3eb0`, the pathing family's). Answers `this`.
pub fn bound_object_array_new(e: &mut Engine, this: u32) -> u32 {
    e.mem.set_u32(this, VTABLE_BOUND_OBJECT_ARRAY);
    typed_call!(
        pathing::FAMILY,
        0x006b_3eb0,
        pathing::search::init_vertices(e, Ptr::new(this), 0, 0)
    );
    this
}

// Typed from 0042f5a0
/// The smaller of two unsigned words (`cdecl`).
pub fn min_word(_e: &mut Engine, first: u32, second: u32) -> u32 {
    first.min(second)
}

// Typed from 0042f5d0
/// The memory manager's reallocation of `block` to `size` bytes (`cdecl`; the manager's address in
/// place of `00401020`, its `00aa4150` by address). Answers the block.
pub fn reallocate(e: &mut Engine, block: u32, size: u32) -> u32 {
    e.call(
        MEMORY_MANAGER_REALLOCATE,
        &args![MEMORY_MANAGER, block, size],
    )
    .u32()
}

// Typed from 0042f5f0
/// `BSSimpleArray::SetSize` (`thiscall(array, size, shrink)`): 0 clears the array (`008454f0`, the
/// dispatch family's) with `shrink`. Above the capacity: a first buffer from the vtable, or the
/// buffer moved ([`reallocate_buffer`]), the capacity set, then the new elements constructed
/// (`00430000`). Below the size: the elements past it destroyed (`0072ba80`) and, with `shrink` and
/// the new size no more than a quarter of the capacity, the buffer moved to the new size. Otherwise
/// the new elements constructed. The size set last.
pub fn set_size(e: &mut Engine, this: u32, new_size: u32, shrink: u8) {
    if new_size == 0 {
        typed_call!(
            dispatch::FAMILY,
            0x0084_54f0,
            dispatch::refs::array_clear(e, Ptr::new(this), shrink)
        );
        return;
    }
    let capacity = word(e, this, CAPACITY);
    if new_size > capacity {
        if capacity == 0 {
            let buffer = new_buffer(e, this, new_size);
            set_word(e, this, BUFFER, buffer);
        } else {
            let size = word(e, this, SIZE);
            typed_call!(
                FAMILY,
                0x0042_fc00,
                reallocate_buffer(e, this, new_size, size)
            );
        }
        set_word(e, this, CAPACITY, new_size);
        construct_past(e, this, new_size);
    } else {
        let size = word(e, this, SIZE);
        if new_size < size {
            let buffer = word(e, this, BUFFER);
            e.call(
                ELEMENTS_DESTROY,
                &args![this, slot_at(buffer, new_size), size.wrapping_sub(new_size)],
            );
            set_word(e, this, SIZE, new_size);
            if shrink != 0 && new_size <= word(e, this, CAPACITY) >> 2 {
                typed_call!(
                    FAMILY,
                    0x0042_fc00,
                    reallocate_buffer(e, this, new_size, new_size)
                );
                set_word(e, this, CAPACITY, new_size);
            }
        } else {
            construct_past(e, this, new_size);
        }
    }
}

/// [`set_size`]'s growth: the elements from the size to `new_size` constructed (`00430000`), then
/// the size set.
#[inline]
fn construct_past(e: &mut Engine, this: u32, new_size: u32) {
    let size = word(e, this, SIZE);
    let buffer = word(e, this, BUFFER);
    e.call(
        ELEMENTS_CONSTRUCT,
        &args![this, slot_at(buffer, size), new_size.wrapping_sub(size)],
    );
    set_word(e, this, SIZE, new_size);
}

// Typed from 0042f730
/// `NiPointer<ActorCause>`'s constructor: the object stored and, when not null, a reference taken
/// (`0044bbc0`). Answers `this`.
pub fn actor_cause_handle_new(e: &mut Engine, this: u32, object: u32) -> u32 {
    e.mem.set_u32(this, object);
    if object != 0 {
        e.call(ACTOR_CAUSE_ADD_REFERENCE, &args![object]);
    }
    this
}

// Typed from 0042f760
/// `NiPointer<ActorCause>`'s destructor: the object held released (`0066e2b0`), when not null.
pub fn actor_cause_handle_drop(e: &mut Engine, this: u32) {
    let object = e.mem.u32(this);
    if object != 0 {
        e.call(ACTOR_CAUSE_RELEASE, &args![object]);
    }
}

// Typed from 0042f780
/// `NiPointer<ActorCause>::operator=` (Xbox PDB): another object than the one held releases the old
/// one (`0066e2b0`, when not null), is stored and has a reference taken (`0044bbc0`, when not
/// null). Answers `this`.
pub fn actor_cause_assign(e: &mut Engine, this: u32, object: u32) -> u32 {
    let held = e.mem.u32(this);
    if held != object {
        if held != 0 {
            e.call(ACTOR_CAUSE_RELEASE, &args![held]);
        }
        e.mem.set_u32(this, object);
        if object != 0 {
            e.call(ACTOR_CAUSE_ADD_REFERENCE, &args![object]);
        }
    }
    this
}

// Typed from 0042f7d0
/// `BSSimpleArray<NavMeshPtr,1024>`'s constructor with a capacity: its vtable, then
/// [`navmesh_array_init`] with `capacity` as capacity and size. Answers `this`.
pub fn navmesh_array_with(e: &mut Engine, this: u32, capacity: u32) -> u32 {
    e.mem.set_u32(this, VTABLE_NAVMESH_ARRAY);
    typed_call!(
        FAMILY,
        0x0042_fcb0,
        navmesh_array_init(e, this, capacity, capacity)
    );
    this
}

// Typed from 0042f800
/// `BSSimpleArray<NavMeshPtr,1024>`'s default constructor: its vtable, then [`navmesh_array_init`]
/// with no capacity and no size. Answers `this`.
pub fn navmesh_array_new(e: &mut Engine, this: u32) -> u32 {
    e.mem.set_u32(this, VTABLE_NAVMESH_ARRAY);
    typed_call!(FAMILY, 0x0042_fcb0, navmesh_array_init(e, this, 0, 0));
    this
}

// Typed from 0042f830
/// `BSSimpleArray<NavMeshPtr,1024>`'s destructor: its vtable, then [`navmesh_array_clear`] freeing
/// the buffer.
pub fn navmesh_array_drop(e: &mut Engine, this: u32) {
    e.mem.set_u32(this, VTABLE_NAVMESH_ARRAY);
    typed_call!(FAMILY, 0x0042_f9c0, navmesh_array_clear(e, this, 1));
}

// Typed from 0042f850
/// The navmesh array's add (`thiscall(array, &pointer)`): a slot made (`00761540`, its index), the
/// element there constructed null ([`navmesh_ptrs_new`]) and assigned from the counted pointer at
/// `source` ([`element_assign`]). Answers the index.
pub fn navmesh_array_add(e: &mut Engine, this: u32, source: u32) -> u32 {
    let index = e.call(ADD_SLOT, &args![this]).u32();
    let slot = slot_at(word(e, this, BUFFER), index);
    typed_call!(FAMILY, 0x0042_fa60, navmesh_ptrs_new(e, this, slot, 1));
    let slot = slot_at(word(e, this, BUFFER), index);
    typed_call!(FAMILY, 0x0042_f4c0, element_assign(e, slot, source));
    index
}

// Typed from 0042f8a0
/// The navmesh array's removal at `index` (`thiscall(array, index, shrink)`). With `shrink` and the
/// array sparse enough (`006f3170`, the pathing family's): a new buffer of the shrunk capacity
/// (`00869600`, from the vtable), the elements before `index` moved to it, the element released,
/// the rest moved after `index` (with the size less one as the count, as the exe does), the old
/// buffer freed and the new one and its capacity set. Otherwise the element released in place and
/// the rest moved down one. The size one less.
pub fn navmesh_array_remove_at(e: &mut Engine, this: u32, index: u32, shrink: u8) {
    if shrink != 0
        && typed_call!(
            pathing::FAMILY,
            0x006f_3170,
            pathing::search::smoother_array_should_shrink(e, Ptr::new(this))
        )
    {
        let capacity = e.call(SHRUNK_CAPACITY, &args![this]).u32();
        let fresh = new_buffer(e, this, capacity);
        let buffer = word(e, this, BUFFER);
        typed_call!(
            FAMILY,
            0x0042_fb60,
            move_words(e, this, fresh, buffer, index)
        );
        let buffer = word(e, this, BUFFER);
        typed_call!(
            FAMILY,
            0x0042_fb20,
            navmesh_ptrs_drop(e, this, slot_at(buffer, index), 1)
        );
        let count = word(e, this, SIZE).wrapping_sub(1);
        let buffer = word(e, this, BUFFER);
        typed_call!(
            FAMILY,
            0x0042_fb60,
            move_words(
                e,
                this,
                slot_at(fresh, index),
                slot_at(buffer, index).wrapping_add(4),
                count
            )
        );
        free_buffer(e, this);
        set_word(e, this, BUFFER, fresh);
        set_word(e, this, CAPACITY, capacity);
    } else {
        let buffer = word(e, this, BUFFER);
        typed_call!(
            FAMILY,
            0x0042_fb20,
            navmesh_ptrs_drop(e, this, slot_at(buffer, index), 1)
        );
        let count = word(e, this, SIZE).wrapping_sub(index).wrapping_sub(1);
        let at = slot_at(word(e, this, BUFFER), index);
        typed_call!(
            FAMILY,
            0x0042_fb60,
            move_words(e, this, at, at.wrapping_add(4), count)
        );
    }
    let size = word(e, this, SIZE);
    set_word(e, this, SIZE, size.wrapping_sub(1));
}

// Typed from 0042f9c0
/// The navmesh array's clear (`thiscall(array, free)`): with a buffer, its elements released
/// ([`navmesh_ptrs_drop`]); with `free` the buffer freed and the buffer and capacity cleared; then
/// the size 0. An array without a buffer is left as it is.
pub fn navmesh_array_clear(e: &mut Engine, this: u32, free: u8) {
    let buffer = word(e, this, BUFFER);
    if buffer == 0 {
        return;
    }
    let size = word(e, this, SIZE);
    typed_call!(
        FAMILY,
        0x0042_fb20,
        navmesh_ptrs_drop(e, this, buffer, size)
    );
    if free != 0 {
        free_buffer(e, this);
        set_word(e, this, BUFFER, 0);
        set_word(e, this, CAPACITY, 0);
    }
    set_word(e, this, SIZE, 0);
}

// Typed from 0042fa20
/// The counted pointer's copy constructor ([`navmesh_ptr_copy`] from the pointer at `source`).
/// Answers `this`.
pub fn navmesh_ptr_copy_new(e: &mut Engine, this: u32, source: u32) -> u32 {
    typed_call!(FAMILY, 0x0042_fd40, navmesh_ptr_copy(e, this, source));
    this
}

// Typed from 0042fa40
/// The counted pointer's destructor ([`navmesh_ptr_release`]).
pub fn navmesh_ptr_drop(e: &mut Engine, this: u32) {
    typed_call!(FAMILY, 0x0042_fda0, navmesh_ptr_release(e, this));
}

// Typed from 0042fa60
/// `count` counted pointers constructed null from `address` on (placement `new`, `006e6da0`, and
/// the null constructor [`navmesh_ptr_null`], both in place; a slot at address 0 is skipped, as the
/// exe's null test does). The array is not read.
pub fn navmesh_ptrs_new(e: &mut Engine, _this: u32, address: u32, count: u32) {
    for index in 0..count {
        let slot = slot_at(address, index);
        if slot != 0 {
            e.mem.set_u32(slot, 0);
        }
    }
}

// Typed from 0042fb00
/// The counted pointer's default constructor: null (`0042fd70` with no object, in place). Answers
/// `this`.
pub fn navmesh_ptr_null(e: &mut Engine, this: u32) -> u32 {
    e.mem.set_u32(this, 0);
    this
}

// Typed from 0042fb20
/// `count` counted pointers released from `address` on (each its scalar deleting destructor with 0,
/// [`navmesh_ptr_delete`]). The array is not read.
pub fn navmesh_ptrs_drop(e: &mut Engine, _this: u32, address: u32, count: u32) {
    for index in 0..count {
        let slot = slot_at(address, index);
        typed_call!(FAMILY, 0x0042_ffa0, navmesh_ptr_delete(e, slot, 0));
    }
}

// Typed from 0042fb60
/// `count` words moved from `source` to `destination`, each a 4-byte `memmove` (`00ec7230`, in
/// place): upwards when the destination is below the source, downwards (the index counted down as a
/// signed word, as the exe does) when it is above, none when they are the same. The array is not
/// read.
pub fn move_words(e: &mut Engine, _this: u32, destination: u32, source: u32, count: u32) {
    if count == 0 {
        return;
    }
    let move_one = |e: &mut Engine, index: u32| {
        let w = e.mem.u32(slot_at(source, index));
        e.mem.set_u32(slot_at(destination, index), w);
    };
    if destination < source {
        for index in 0..count {
            move_one(e, index);
        }
    } else if destination > source {
        let mut index = count.wrapping_sub(1) as i32;
        while index >= 0 {
            move_one(e, index as u32);
            index -= 1;
        }
    }
}

// Typed from 0042fc00
/// The array's buffer moved to `capacity` elements (`thiscall(array, capacity, count)`). No buffer:
/// one from the vtable, and the capacity set. `count` equal to the capacity: the vtable's
/// reallocation of the buffer. Otherwise a new buffer from the vtable, `count` words moved to it,
/// the old one freed and the new one set.
pub fn reallocate_buffer(e: &mut Engine, this: u32, capacity: u32, count: u32) {
    let buffer = word(e, this, BUFFER);
    if buffer == 0 {
        let fresh = new_buffer(e, this, capacity);
        set_word(e, this, BUFFER, fresh);
        set_word(e, this, CAPACITY, capacity);
    } else if count == word(e, this, CAPACITY) {
        let fresh = e
            .vcall(this, SLOT_REALLOCATE, &args![buffer, capacity])
            .u32();
        set_word(e, this, BUFFER, fresh);
    } else {
        let fresh = new_buffer(e, this, capacity);
        let old = word(e, this, BUFFER);
        typed_call!(FAMILY, 0x0042_fb60, move_words(e, this, fresh, old, count));
        free_buffer(e, this);
        set_word(e, this, BUFFER, fresh);
    }
}

// Typed from 0042fcb0
/// The navmesh array's body (`thiscall(array, capacity, size)`): its three words zero; the capacity
/// raised to the size; with a capacity a buffer from the vtable and the capacity set; with a size
/// that many null elements ([`navmesh_ptrs_new`]) and the size set.
pub fn navmesh_array_init(e: &mut Engine, this: u32, capacity: u32, size: u32) {
    set_word(e, this, BUFFER, 0);
    set_word(e, this, SIZE, 0);
    set_word(e, this, CAPACITY, 0);
    let capacity = capacity.max(size);
    if capacity != 0 {
        let buffer = new_buffer(e, this, capacity);
        set_word(e, this, BUFFER, buffer);
        set_word(e, this, CAPACITY, capacity);
    }
    if size != 0 {
        let buffer = word(e, this, BUFFER);
        typed_call!(FAMILY, 0x0042_fa60, navmesh_ptrs_new(e, this, buffer, size));
        set_word(e, this, SIZE, size);
    }
}

// Typed from 0042fd40
/// The counted pointer's copy: the object held at `source` stored and, when not null, a reference
/// taken. Answers `this`.
pub fn navmesh_ptr_copy(e: &mut Engine, this: u32, source: u32) -> u32 {
    let object = e.mem.u32(source);
    navmesh_ptr_set(e, this, object)
}

// Typed from 0042fd70
/// The counted pointer's constructor from an object: stored and, when not null, a reference taken.
/// Answers `this`.
pub fn navmesh_ptr_set(e: &mut Engine, this: u32, object: u32) -> u32 {
    e.mem.set_u32(this, object);
    if object != 0 {
        take_reference(e, object);
    }
    this
}

// Typed from 0042fda0
/// The counted pointer's destructor body: the object's reference released, when not null.
pub fn navmesh_ptr_release(e: &mut Engine, this: u32) {
    let object = e.mem.u32(this);
    if object != 0 {
        give_reference(e, object);
    }
}

// Typed from 0042fdc0
/// The counted pointer's assignment from the one at `source`: another object than the one held
/// releases the old one (when not null), is stored and has a reference taken (when not null). The
/// source is read again after the release, as the exe reads it (`0042fdeb`): the release can delete
/// the object the source lives in. Answers `this`.
pub fn navmesh_ptr_assign(e: &mut Engine, this: u32, source: u32) -> u32 {
    let held = e.mem.u32(this);
    if held != e.mem.u32(source) {
        if held != 0 {
            give_reference(e, held);
        }
        let object = e.mem.u32(source);
        e.mem.set_u32(this, object);
        if object != 0 {
            take_reference(e, object);
        }
    }
    this
}

// Typed from 0042fe10
/// `NiTMapBase<..., TESObjectREFR *, bool>`'s constructor body: its vtable, the hash size (+4), no
/// items (+0xc), a table of that many words (`00aa1070`, +8) cleared (`memset`, the C runtime
/// family's). Answers `this`.
pub fn refr_map_base_init(e: &mut Engine, this: u32, hash_size: u32) -> u32 {
    e.mem.set_u32(this, VTABLE_REFR_MAP_BASE);
    set_word(e, this, 4, hash_size);
    set_word(e, this, 0xc, 0);
    let table = e
        .call(TABLE_ALLOCATE, &args![hash_size.wrapping_shl(2)])
        .u32();
    set_word(e, this, 8, table);
    let bytes = word(e, this, 4).wrapping_shl(2);
    zero_bytes(e, table, bytes);
    this
}

// Typed from 0042fe80
/// `NiTMap<TESObjectREFR *, bool>`'s destructor body: its vtable, the map emptied (`00438af0`),
/// then its base's ([`refr_map_base_drop`]).
pub fn refr_map_drop(e: &mut Engine, this: u32) {
    e.mem.set_u32(this, VTABLE_REFR_MAP);
    e.call(MAP_REMOVE_ALL, &args![this]);
    typed_call!(FAMILY, 0x0042_fee0, refr_map_base_drop(e, this));
}

// Typed from 0042fee0
/// `NiTMapBase<..., TESObjectREFR *, bool>`'s destructor body: its vtable, the map emptied
/// (`00438af0`), the table (+8) freed (`00aa10f0`).
pub fn refr_map_base_drop(e: &mut Engine, this: u32) {
    e.mem.set_u32(this, VTABLE_REFR_MAP_BASE);
    e.call(MAP_REMOVE_ALL, &args![this]);
    let table = word(e, this, 8);
    e.call(TABLE_FREE, &args![table]);
}

// Typed from 0042ff10
/// [`navmesh_ptr_equal`] as a `cdecl` function of the two pointers.
pub fn navmesh_ptr_equal_of(e: &mut Engine, first: u32, second: u32) -> u32 {
    typed_call!(FAMILY, 0x0042_ff30, navmesh_ptr_equal(e, first, second))
}

// Typed from 0042ff30
/// Whether two counted pointers hold the same object (`004b0460`, in place): 1 or 0.
pub fn navmesh_ptr_equal(e: &mut Engine, this: u32, other: u32) -> u32 {
    u32::from(e.mem.u32(this) == e.mem.u32(other))
}

// Typed from 0042ff50
/// `BSSimpleArray<TESBoundObject *,1024>`'s scalar deleting destructor:
/// [`bound_object_array_drop`], then `operator delete` for bit 0 of `flags`. Answers `this`.
pub fn bound_object_array_delete(e: &mut Engine, this: u32, flags: u32) -> u32 {
    typed_call!(FAMILY, 0x0042_ff80, bound_object_array_drop(e, this));
    free_when(e, this, flags)
}

// Typed from 0042ff80
/// `BSSimpleArray<TESBoundObject *,1024>`'s destructor body: its vtable, then the array cleared
/// with its buffer freed (`008454f0`, the dispatch family's).
pub fn bound_object_array_drop(e: &mut Engine, this: u32) {
    e.mem.set_u32(this, VTABLE_BOUND_OBJECT_ARRAY);
    typed_call!(
        dispatch::FAMILY,
        0x0084_54f0,
        dispatch::refs::array_clear(e, Ptr::new(this), 1)
    );
}

// Typed from 0042ffa0
/// The counted pointer's scalar deleting destructor: [`navmesh_ptr_drop`], then `operator delete`
/// for bit 0 of `flags`. Answers `this`.
pub fn navmesh_ptr_delete(e: &mut Engine, this: u32, flags: u32) -> u32 {
    typed_call!(FAMILY, 0x0042_fa40, navmesh_ptr_drop(e, this));
    free_when(e, this, flags)
}

// Typed from 0042ffd0
/// `NiTMapBase<..., TESObjectREFR *, bool>`'s scalar deleting destructor: [`refr_map_base_drop`],
/// then `operator delete` for bit 0 of `flags`. Answers `this`.
pub fn refr_map_base_delete(e: &mut Engine, this: u32, flags: u32) -> u32 {
    typed_call!(FAMILY, 0x0042_fee0, refr_map_base_drop(e, this));
    free_when(e, this, flags)
}
