//! The lock-free maps' scalar deleting destructors the linker placed in the run after the debug-heap
//! string (`0044d8c0` to `0044dce0`: the maps of the queued references, idles, replacement KF
//! lists, helmets and textures, of the models, KF models and loaded files, by the destructor bodies
//! they run), the queued task pointer's release (`0044dd60`) and the map iterator's flag test
//! (`0044de20`).
//!
//! **Native.** Each deleting destructor runs its map's destructor body as the lock-free family's
//! typed call (`0044d8f0`, `0044d9b0`, `0044da70`, `0044db30`, `0044c1e0`, `0044c3f0`, `0044dc50`,
//! `0044cae0`), then `operator delete` (`00401030`) by address when bit 0 of the flags is set. The
//! release's interlocked decrement is dispatch's typed `004019a0`; the object's own scalar deleting
//! destructor stays a virtual call (slot 0: the task classes override it).
//!
//! No floats.

use crate::{dispatch, lockfree};
use engine_core::ptr::Ptr;
use engine_core::{args, typed_call, Engine};

/// `operator delete` (cdecl).
const OPERATOR_DELETE: u32 = 0x0040_1030;
/// A counted task's count (+8) and its scalar deleting destructor (slot 0).
const COUNT: u32 = 8;
const SLOT_DELETING_DESTRUCTOR: u32 = 0;
/// The iterator's flags byte (+0xc).
const FLAGS: u32 = 0xc;

/// The deleting destructors' end: the block freed when bit 0 of `flags` is set; `this`.
fn delete_if_flagged(e: &mut Engine, this: u32, flags: u32) -> u32 {
    if flags & 1 != 0 {
        e.call(OPERATOR_DELETE, &args![this]);
    }
    this
}

// Typed from 0044d8c0
/// `LockFreeMap<TESObjectREFR*, NiPointer<QueuedReference>>`'s scalar deleting destructor: its
/// destructor (`0044d8f0`), then the block freed on flag bit 0. Answers `this`.
pub fn queued_reference_map_delete(e: &mut Engine, this: u32, flags: u32) -> u32 {
    typed_call!(
        lockfree::FAMILY,
        0x0044_d8f0,
        lockfree::map::destroy_queued_reference_map(e, Ptr::new(this))
    );
    delete_if_flagged(e, this, flags)
}

// Typed from 0044d980
/// `LockFreeMap<AnimIdle*, NiPointer<QueuedAnimIdle>>`'s scalar deleting destructor (its
/// destructor `0044d9b0`).
pub fn anim_idle_map_delete(e: &mut Engine, this: u32, flags: u32) -> u32 {
    typed_call!(
        lockfree::FAMILY,
        0x0044_d9b0,
        lockfree::map::destroy_anim_idle_map(e, Ptr::new(this))
    );
    delete_if_flagged(e, this, flags)
}

// Typed from 0044da40
/// `LockFreeMap<Animation*, NiPointer<QueuedReplacementKFList>>`'s scalar deleting destructor (its
/// destructor `0044da70`).
pub fn replacement_kf_list_map_delete(e: &mut Engine, this: u32, flags: u32) -> u32 {
    typed_call!(
        lockfree::FAMILY,
        0x0044_da70,
        lockfree::map::destroy_replacement_kf_list_map(e, Ptr::new(this))
    );
    delete_if_flagged(e, this, flags)
}

// Typed from 0044db00
/// `LockFreeMap<TESObjectREFR*, NiPointer<QueuedHelmet>>`'s scalar deleting destructor (its
/// destructor `0044db30`).
pub fn helmet_map_delete(e: &mut Engine, this: u32, flags: u32) -> u32 {
    typed_call!(
        lockfree::FAMILY,
        0x0044_db30,
        lockfree::map::destroy_helmet_map(e, Ptr::new(this))
    );
    delete_if_flagged(e, this, flags)
}

// Typed from 0044dbc0
/// `LockFreeMap<const char*, Model*>`'s scalar deleting destructor (its destructor `0044c1e0`).
pub fn model_map_delete(e: &mut Engine, this: u32, flags: u32) -> u32 {
    typed_call!(
        lockfree::FAMILY,
        0x0044_c1e0,
        lockfree::map::destroy_model_map(e, Ptr::new(this))
    );
    delete_if_flagged(e, this, flags)
}

// Typed from 0044dbf0
/// `LockFreeMap<const char*, KFModel*>`'s scalar deleting destructor (its destructor `0044c3f0`).
pub fn kf_model_map_delete(e: &mut Engine, this: u32, flags: u32) -> u32 {
    typed_call!(
        lockfree::FAMILY,
        0x0044_c3f0,
        lockfree::map::destroy(e, Ptr::new(this))
    );
    delete_if_flagged(e, this, flags)
}

// Typed from 0044dc20
/// `LockFreeMap<BSFileEntry*, NiPointer<QueuedTexture>>`'s scalar deleting destructor (its
/// destructor `0044dc50`).
pub fn texture_map_delete(e: &mut Engine, this: u32, flags: u32) -> u32 {
    typed_call!(
        lockfree::FAMILY,
        0x0044_dc50,
        lockfree::map::destroy_texture_map(e, Ptr::new(this))
    );
    delete_if_flagged(e, this, flags)
}

// Typed from 0044dce0
/// `LockFreeMap<const char*, LoadedFile*>`'s scalar deleting destructor (its destructor
/// `0044cae0`).
pub fn loaded_file_map_delete(e: &mut Engine, this: u32, flags: u32) -> u32 {
    typed_call!(
        lockfree::FAMILY,
        0x0044_cae0,
        lockfree::map::destroy_loaded_file_map(e, Ptr::new(this))
    );
    delete_if_flagged(e, this, flags)
}

// Typed from 0044dd60
/// A queued task pointer's release: the task's count (+8) one less (dispatch's typed interlocked
/// decrement); at 0 a task that is not null runs its scalar deleting destructor (slot 0) with 1.
pub fn release_task(e: &mut Engine, this: u32) {
    let count = typed_call!(
        dispatch::FAMILY,
        0x0040_19a0,
        dispatch::refs::decrement(e, Ptr::new(this.wrapping_add(COUNT)))
    );
    if count == 0 && this != 0 {
        e.vcall(this, SLOT_DELETING_DESTRUCTOR, &args![1u32]);
    }
}

// Typed from 0044de20
/// The map iterator's flag test: whether bit 0 of the byte at +0xc is set.
pub fn iterator_flag_set(e: &mut Engine, this: u32) -> bool {
    e.mem.u8(this.wrapping_add(FLAGS)) & 1 != 0
}
