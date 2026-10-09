//! `fallout shared/extradatalist.cpp` (Xbox PDB source unit), subsystem `fallout shared`: its functions in
//! FalloutNV.exe 1.4.0.525, translated (docs/ENGINE_CRATE.md).
//!
//! This unit holds `BaseExtraList` / `ExtraDataList` (the list of extra data
//! hung on a reference, a cell or an inventory entry), the small
//! `BSExtraData` subclasses whose constructors the compiler placed here, and
//! the very large `ExtraDataList::AddExtraCopy`, `ExtraDataList::Load` and
//! their neighbours. 624 functions in all: translated so far, in address
//! order, `0040f680` to `00411e70` (the first 40 the queue listed).
//!
//! A later session continues at `00411ea0`. What it needs is up here:
//!
//! - the layouts and the constants (the lock, the dirty counter, the thread
//!   cache, the callees every function uses) below;
//! - `00411ea0` (the `BSExtraData` subclass destructor `fn_00411e70` calls),
//!   `00411ec0` `ExtraDataList::CopyList` and `00411f20`
//!   `ExtraDataList::IsCopyableExtra` (called by address by `AddExtraCopy`);
//! - the type byte `cEtype` of an extra data is the index into the 21-byte
//!   bitmap `iFlags` of the list (`fn_0040fee0` sets and clears a bit,
//!   `base_extra_list_has_extra` tests it) and into the per-thread cache
//!   (`fn_0040f9e0`, `fn_0040fa40`).
//!
//! The compiler's exception-unwinding frames (the `FS:[0]` chains of the
//! constructors that build a `BSSimpleList`, of `AddExtraCopy`) are not
//! translated.

#[allow(unused_imports)]
use crate::prelude::*;
use crate::types::BSSimpleList;

// ---------------------------------------------------------------------------
// Layouts

layout! {
    /// `BSExtraData` (Xbox PDB), 0x0C bytes: vtable at +0 (slot 0 the scalar
    /// deleting destructor, slot 1 `Compare`), then the type and the next
    /// extra data of the list.
    pub struct BSExtraData: 0x0C {
        /// `cEtype` (Xbox PDB): the extra data type (`0x43` is
        /// `ExtraNorthRotation`, `0x65` `ExtraReflectedRefs`, ...).
        0x04 cEtype: u8,
        /// `pNext` (Xbox PDB): the next extra data of the list.
        0x08 pNext: Ptr,
    }

    /// `BaseExtraList` (Xbox PDB), 0x20 bytes: vtable at +0, the head of the
    /// chain of `BSExtraData`, and a bitmap of the types the chain holds.
    pub struct BaseExtraList: 0x20 {
        /// `pHead` (Xbox PDB): the first `BSExtraData`.
        0x04 pHead: Ptr,
        /// First byte of `iFlags` (Xbox PDB, `u8[21]`): bit `type & 7` of byte
        /// `type >> 3` says the chain holds an extra data of that type.
        0x08 iFlags: u8,
    }

    /// `ExtraDataList` (Xbox PDB): a `BaseExtraList` with its own vtable and
    /// no fields of its own.
    pub struct ExtraDataList: 0x20 {
        /// `pHead` (Xbox PDB), as in `BaseExtraList`.
        0x04 pHead: Ptr,
    }

    /// The statics of `BaseExtraList` that are `__declspec(thread)` in the PC
    /// build, as laid out in the TLS block (`e.tls()`): the last list looked
    /// up, the dirty counter it was valid for, and a cache of the extra data
    /// found in it by type. The Xbox PDB names these `pLastExtraList`,
    /// `iThreadDirty` and `ppLastExtraData`.
    pub struct BaseExtraListThreadCache: 0x25C {
        /// `pLastExtraList` (Xbox PDB).
        0x08 pLastExtraList: Ptr,
        /// `iThreadDirty` (Xbox PDB): the value of the global `iDirty` the
        /// cache was last cleared at.
        0x0C iThreadDirty: u32,
        /// `ppLastExtraData` (Xbox PDB): first of `0x93` pointers, one per
        /// extra data type, 0 for not cached.
        0x10 ppLastExtraData: Ptr,
    }

    /// `ExtraNorthRotation` (Xbox PDB), type `0x43`, 0x10 bytes.
    pub struct ExtraNorthRotation: 0x10 {
        /// `fNorthRot` (Xbox PDB).
        0x0C fNorthRot: f32,
    }

    /// `ExtraDetachTime` (Xbox PDB), 0x10 bytes: the 0x0B-type extra data
    /// `fn_0040f6b0` constructs has this shape (one 32-bit value at +0x0C).
    pub struct ExtraDetachTime: 0x10 {
        /// `iTime` (Xbox PDB).
        0x0C iTime: u32,
    }

    /// `ExtraReflectedRefs` (type `0x65`), `ExtraReflectorRefs` (`0x66`),
    /// `ExtraWaterLightRefs` (`0x84`) and `ExtraLitWaterRefs` (`0x85`)
    /// (Xbox PDB), 0x14 bytes each: a `BSExtraData` and the head node of a
    /// `BSSimpleList` of references.
    pub struct ExtraRefList: 0x14 {
        /// `RefList` (Xbox PDB): the inline head node (item, next).
        0x0C RefList: Inline<BSSimpleList>,
    }
}

// ---------------------------------------------------------------------------
// Constants

/// `BaseExtraList::ExtraCritSection` (Xbox PDB), the `BSSpinLock` that guards
/// every list operation.
const EXTRA_CRIT_SECTION: u32 = 0x011c_3920;
/// `BSSpinLock::Lock(const char *name)` on [`EXTRA_CRIT_SECTION`].
const LOCK: u32 = 0x0040_fbf0;
/// `BSSpinLock::Unlock()`.
const UNLOCK: u32 = 0x0040_fba0;
/// `BaseExtraList::iDirty` (Xbox PDB): bumped whenever a list loses an
/// extra data, so every thread's cache is dropped.
const DIRTY: u32 = 0x011c_38e4;
/// `BSExtraData::BSExtraData(type)`: sets the base vtable, the type, and a
/// null next. The subclass constructors call it, then set their own vtable.
const BS_EXTRA_DATA_INIT: u32 = 0x0040_ec80;
/// `cEtype` getter (`MOV AL,[ECX+4]`; the engine map files it under
/// `tesregiondata.cpp`, identical code folded).
const GET_TYPE: u32 = 0x004f_1540;
/// `pNext` getter (`MOV EAX,[ECX+8]`).
const GET_NEXT: u32 = 0x0044_ddc0;
/// `pNext` setter (`MOV [ECX+8],arg`).
const SET_NEXT: u32 = 0x0040_3550;
/// `MOV EAX,[ECX+4]`: `BSSimpleList`'s `m_pkNext`, and the `pHead` of a
/// `BaseExtraList` where `RemoveExtra_ov2` starts its walk.
const SIMPLE_LIST_NEXT: u32 = 0x0072_6070;
/// `MOV EAX,ECX`: a `BSSimpleList` node's address, which is the address of its
/// item slot (the item is read through the returned pointer).
const SIMPLE_LIST_ITEM: u32 = 0x0068_15c0;
/// `MOV EAX,[ECX]`: reads the word a handle points at.
const READ_WORD: u32 = 0x0055_9450;
/// `memset(dst, value, size)`.
const MEMSET: u32 = 0x0040_3d30;
/// `memcpy(dst, src, size)` (the game's wrapper).
const MEMCPY: u32 = 0x0040_1460;
/// `operator new(size)`.
const OPERATOR_NEW: u32 = 0x0040_1000;
/// `operator delete(block)` (`platform`).
const OPERATOR_DELETE: u32 = 0x0040_1030;
/// `__RTDynamicCast(object, vfDelta, srcType, targetType, isReference)`.
const RT_DYNAMIC_CAST: u32 = 0x00ec_43fb;

/// Size of the bitmap `iFlags`, in bytes.
const FLAGS_LEN: u32 = 0x15;
/// Entries of `ppLastExtraData`; also the first type that is not cached.
const CACHE_ENTRIES: u32 = 0x93;
/// Bytes of `ppLastExtraData` (`CACHE_ENTRIES * 4`).
const CACHE_BYTES: u32 = 0x24c;

/// Lock names the functions pass to [`LOCK`] (strings in the exe).
const NAME_REMOVE_ALL: u32 = 0x0101_4304;
const NAME_REMOVE_ALL_DEFAULT: u32 = 0x0101_4320;
const NAME_ITEMS_IN_LIST: u32 = 0x0101_4344;
const NAME_ADD_EXTRA: u32 = 0x0101_4364;
/// `"BaseExtraList::RemoveExtra()"`, used by both overloads.
const NAME_REMOVE_EXTRA: u32 = 0x0101_4380;
const NAME_GET_EXTRA_DATA: u32 = 0x0101_43a0;
const NAME_GET_PREV_EXTRA_DATA: u32 = 0x0101_43c0;
/// `"AI: No Copy function available for Extra Data type %i."`.
const NO_COPY_MESSAGE: u32 = 0x0101_43ec;
/// Reports a message through the game's log (`005b5e40`, cdecl, printf-like).
const LOG_MESSAGE: u32 = 0x005b_5e40;

/// Vtables set by the constructors of this unit.
const VTABLE_EXTRA_NORTH_ROTATION: u32 = 0x0101_42a0;
const VTABLE_EXTRA_DETACH_TIME: u32 = 0x0101_42ac;
const VTABLE_BASE_EXTRA_LIST: u32 = 0x0101_4300;
const VTABLE_EXTRA_DATA_LIST: u32 = 0x0101_43e8;
const VTABLE_EXTRA_REFLECTED_REFS: u32 = 0x0101_4428;
const VTABLE_EXTRA_REFLECTOR_REFS: u32 = 0x0101_4434;
const VTABLE_EXTRA_WATER_LIGHT_REFS: u32 = 0x0101_4440;
const VTABLE_EXTRA_LIT_WATER_REFS: u32 = 0x0101_444c;
const VTABLE_EXTRA_TYPE_92: u32 = 0x0101_4458;

// ---------------------------------------------------------------------------
// Helpers

fn lock(e: &mut Engine, name: u32) {
    e.call(LOCK, &args![EXTRA_CRIT_SECTION, name]);
}

fn unlock(e: &mut Engine) {
    e.call(UNLOCK, &args![EXTRA_CRIT_SECTION]);
}

fn get_type(e: &mut Engine, extra: Ptr<BSExtraData>) -> u8 {
    e.call(GET_TYPE, &args![extra]).u8()
}

fn get_next(e: &mut Engine, extra: Ptr<BSExtraData>) -> Ptr<BSExtraData> {
    e.call(GET_NEXT, &args![extra]).ptr()
}

fn set_next(e: &mut Engine, extra: Ptr<BSExtraData>, next: Ptr<BSExtraData>) {
    e.call(SET_NEXT, &args![extra, next]);
}

/// The thread's `BaseExtraList` statics (the TLS block).
fn thread_cache(e: &mut Engine) -> Ptr<BaseExtraListThreadCache> {
    Ptr::new(e.tls())
}

/// Address of cache entry `index` (`ppLastExtraData[index]`).
fn cache_entry(cache: Ptr<BaseExtraListThreadCache>, index: u32) -> u32 {
    cache.addr() + 0x10 + index.wrapping_mul(4)
}

/// `memset` of the whole `ppLastExtraData` array to null.
fn clear_cache_entries(e: &mut Engine, cache: Ptr<BaseExtraListThreadCache>) {
    e.call(MEMSET, &args![cache_entry(cache, 0), 0u32, CACHE_BYTES]);
}

/// `iDirty` and this thread's `iThreadDirty` both go up by one, so the other
/// threads (and this one) drop what they cached.
fn bump_dirty_counters(e: &mut Engine) {
    let dirty = e.global::<u32>(DIRTY).wrapping_add(1);
    e.set_global(DIRTY, dirty);
    let cache = thread_cache(e);
    let thread_dirty = e
        .get(cache, BaseExtraListThreadCache::iThreadDirty)
        .wrapping_add(1);
    e.set(cache, BaseExtraListThreadCache::iThreadDirty, thread_dirty);
}

/// `delete extra`: the scalar deleting destructor, slot 0 of the vtable.
fn delete_extra(e: &mut Engine, extra: Ptr<BSExtraData>) {
    if !extra.is_null() {
        e.vcall(extra.addr(), 0, &args![1u32]);
    }
}

/// The destructors of the extra data subclasses all end the same way:
/// the real destructor, then `operator delete` when bit 0 of `flags` is set.
fn finish_scalar_deleting_destructor(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    if flags & 1 != 0 {
        e.call(OPERATOR_DELETE, &args![this]);
    }
    this
}

// Translated from 0040f680 (decompiled, FalloutNV.exe 1.4.0.525)
/// Constructor of `ExtraNorthRotation` (its only caller is
/// `SetNorthRotation`): a `BSExtraData` of type `0x43` with a north
/// rotation of 0.0. Returns `this`.
pub fn fn_0040f680(e: &mut Engine, this: Ptr<ExtraNorthRotation>) -> Ptr<ExtraNorthRotation> {
    e.call(BS_EXTRA_DATA_INIT, &args![this, 0x43u32]);
    e.mem.set_u32(this.addr(), VTABLE_EXTRA_NORTH_ROTATION);
    e.set(this, ExtraNorthRotation::fNorthRot, 0.0);
    this
}

// Translated from 0040f6b0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Constructor of the type `0x0B` extra data, a `BSExtraData` with one
/// 32-bit value (zero) at +0x0C; the Xbox PDB class of that shape is
/// `ExtraDetachTime` (`iTime`). Returns `this`.
pub fn fn_0040f6b0(e: &mut Engine, this: Ptr<ExtraDetachTime>) -> Ptr<ExtraDetachTime> {
    e.call(BS_EXTRA_DATA_INIT, &args![this, 0x0Bu32]);
    e.mem.set_u32(this.addr(), VTABLE_EXTRA_DETACH_TIME);
    e.set(this, ExtraDetachTime::iTime, 0);
    this
}

// Translated from 0040f700 (decompiled, FalloutNV.exe 1.4.0.525)
/// `BSExtraData::Compare` (Xbox PDB): true when the two differ, which for
/// the base class means their types differ; a null `other` always differs.
pub fn bs_extra_data_compare(
    e: &mut Engine,
    this: Ptr<BSExtraData>,
    other: Ptr<BSExtraData>,
) -> bool {
    if other.is_null() {
        return true;
    }
    let this_type = get_type(e, this);
    let other_type = get_type(e, other);
    this_type != other_type
}

// Translated from 0040f740 (decompiled, FalloutNV.exe 1.4.0.525)
/// `BaseExtraList::BaseExtraList` (inlined into the unit): sets the vtable,
/// clears the head and the 21-byte type bitmap. Returns `this`.
pub fn fn_0040f740(e: &mut Engine, this: Ptr<BaseExtraList>) -> Ptr<BaseExtraList> {
    e.mem.set_u32(this.addr(), VTABLE_BASE_EXTRA_LIST);
    e.set(this, BaseExtraList::pHead, Ptr::NULL);
    let flags = this.byte_add(8);
    e.call(MEMSET, &args![flags, 0u32, FLAGS_LEN]);
    this
}

// Translated from 0040f780 (decompiled, FalloutNV.exe 1.4.0.525)
/// `BaseExtraList::_scalar_deleting_destructor_`: runs the destructor and
/// frees the object when bit 0 of `flags` is set. Returns `this`.
pub fn fn_0040f780(e: &mut Engine, this: Ptr<BaseExtraList>, flags: u32) -> Ptr<BaseExtraList> {
    fn_0040f7b0(e, this);
    finish_scalar_deleting_destructor(e, this.cast(), flags).cast()
}

// Translated from 0040f7b0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `BaseExtraList::~BaseExtraList` (Xbox PDB name of the destructor): sets
/// the vtable and destroys every extra data of the list.
pub fn fn_0040f7b0(e: &mut Engine, this: Ptr<BaseExtraList>) {
    e.mem.set_u32(this.addr(), VTABLE_BASE_EXTRA_LIST);
    base_extra_list_remove_all(e, this, true);
}

// Translated from 0040f7d0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `BaseExtraList::ClearLastExtra` (Xbox PDB): bumps the dirty counters and,
/// if `list` is the list this thread last looked at, forgets the cached
/// extra data of `extra_type`.
pub fn base_extra_list_clear_last_extra(e: &mut Engine, list: Ptr<BaseExtraList>, extra_type: i32) {
    bump_dirty_counters(e);
    let cache = thread_cache(e);
    if list
        == e.get(cache, BaseExtraListThreadCache::pLastExtraList)
            .cast()
        && (0..CACHE_ENTRIES as i32).contains(&extra_type)
    {
        e.mem.set_u32(cache_entry(cache, extra_type as u32), 0);
    }
}

// Translated from 0040f860 (decompiled, FalloutNV.exe 1.4.0.525)
/// `BaseExtraList::ClearLastExtraAll` (Xbox PDB): bumps the dirty counters
/// and, if `list` is the list this thread last looked at, empties the whole
/// cache and forgets the list.
pub fn base_extra_list_clear_last_extra_all(e: &mut Engine, list: Ptr<BaseExtraList>) {
    bump_dirty_counters(e);
    let cache = thread_cache(e);
    if list
        == e.get(cache, BaseExtraListThreadCache::pLastExtraList)
            .cast()
    {
        clear_cache_entries(e, cache);
        e.set(cache, BaseExtraListThreadCache::pLastExtraList, Ptr::NULL);
    }
}

// Translated from 0040f900 (decompiled, FalloutNV.exe 1.4.0.525)
/// Drops this thread's cache when the global dirty counter has moved since
/// the cache was filled: forgets the list, clears the entries, and takes the
/// current counter.
pub fn fn_0040f900(e: &mut Engine) {
    let cache = thread_cache(e);
    let dirty = e.global::<u32>(DIRTY);
    if e.get(cache, BaseExtraListThreadCache::iThreadDirty) != dirty {
        e.set(cache, BaseExtraListThreadCache::pLastExtraList, Ptr::NULL);
        clear_cache_entries(e, cache);
        e.set(cache, BaseExtraListThreadCache::iThreadDirty, dirty);
    }
}

// Translated from 0040f980 (decompiled, FalloutNV.exe 1.4.0.525)
/// True for the extra data types that `AddExtra` puts at the head of the
/// chain instead of the end: `0x0C`, `0x0D`, `0x0E`, `0x15` and `0x2B`
/// (a jump table in the exe).
pub fn fn_0040f980(_e: &mut Engine, extra_type: u32) -> bool {
    matches!(extra_type, 0x0c | 0x0d | 0x0e | 0x15 | 0x2b)
}

// Translated from 0040f9e0 (decompiled, FalloutNV.exe 1.4.0.525)
/// The extra data of `extra_type` this thread has cached for `list`, or null
/// (not the list cached, type out of range, or nothing cached yet). Drops a
/// stale cache first.
pub fn fn_0040f9e0(e: &mut Engine, list: Ptr<BaseExtraList>, extra_type: i32) -> Ptr<BSExtraData> {
    let mut found = Ptr::NULL;
    let cache = thread_cache(e);
    if list
        == e.get(cache, BaseExtraListThreadCache::pLastExtraList)
            .cast()
    {
        fn_0040f900(e);
        if (0..CACHE_ENTRIES as i32).contains(&extra_type) {
            found = Ptr::new(e.mem.u32(cache_entry(cache, extra_type as u32)));
        }
    }
    found
}

// Translated from 0040fa40 (decompiled, FalloutNV.exe 1.4.0.525)
/// Caches `extra` as the extra data of its type for `list` in this thread's
/// cache, switching the cache to `list` (and emptying it) when it held
/// another list. Nothing happens for a null list or extra data. The index is
/// the type byte as it is, with no range check.
pub fn fn_0040fa40(e: &mut Engine, list: Ptr<BaseExtraList>, extra: Ptr<BSExtraData>) {
    if list.is_null() || extra.is_null() {
        return;
    }
    let cache = thread_cache(e);
    if list
        != e.get(cache, BaseExtraListThreadCache::pLastExtraList)
            .cast()
    {
        clear_cache_entries(e, cache);
        e.set(cache, BaseExtraListThreadCache::pLastExtraList, list.cast());
    }
    let extra_type = get_type(e, extra);
    e.mem
        .set_u32(cache_entry(cache, extra_type as u32), extra.addr());
}

// Translated from 0040fae0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `BaseExtraList::RemoveAll` (Xbox PDB): empties the list. With `destroy`
/// every extra data is unlinked and deleted; without it the chain is just
/// forgotten. The type bitmap is cleared either way.
pub fn base_extra_list_remove_all(e: &mut Engine, this: Ptr<BaseExtraList>, destroy: bool) {
    lock(e, NAME_REMOVE_ALL);
    base_extra_list_clear_last_extra_all(e, this);
    if destroy {
        let mut current: Ptr<BSExtraData> = e.get(this, BaseExtraList::pHead).cast();
        while !current.is_null() {
            let doomed = current;
            current = get_next(e, current);
            e.set(this, BaseExtraList::pHead, current.cast());
            if destroy {
                delete_extra(e, doomed);
            }
        }
    } else {
        e.set(this, BaseExtraList::pHead, Ptr::NULL);
    }
    let flags = this.byte_add(8);
    e.call(MEMSET, &args![flags, 0u32, FLAGS_LEN]);
    unlock(e);
}

// Translated from 0040fcb0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `BaseExtraList::RemoveAllDefault` (Xbox PDB): removes every extra data
/// except the persistent kinds (types `0x2E`, `0x38`, `0x4F`, `0x52`, `0x54`,
/// `0x58`, `0x65`, `0x7B`, `0x84`, from a jump table in the exe), deleting
/// them when `destroy` is set, and clears their bits in the type bitmap.
pub fn base_extra_list_remove_all_default(e: &mut Engine, this: Ptr<BaseExtraList>, destroy: bool) {
    lock(e, NAME_REMOVE_ALL_DEFAULT);
    base_extra_list_clear_last_extra_all(e, this);
    let mut kept: Ptr<BSExtraData> = Ptr::NULL;
    let mut current: Ptr<BSExtraData> = e.get(this, BaseExtraList::pHead).cast();
    while !current.is_null() {
        let extra_type = get_type(e, current);
        let remove = !matches!(
            extra_type,
            0x2e | 0x38 | 0x4f | 0x52 | 0x54 | 0x58 | 0x65 | 0x7b | 0x84
        );
        let next = e.mem.u32(current.addr() + 8);
        if remove {
            if kept.is_null() {
                e.set(this, BaseExtraList::pHead, Ptr::new(next));
            } else {
                set_next(e, kept, Ptr::new(next));
            }
            if destroy {
                delete_extra(e, current);
            }
            fn_0040fee0(e, this, extra_type, false);
        } else {
            kept = current;
        }
        current = Ptr::new(next);
    }
    unlock(e);
}

// Translated from 0040fe20 (decompiled, FalloutNV.exe 1.4.0.525)
/// `BaseExtraList::ItemsInList` (Xbox PDB): the number of extra data in the
/// chain.
pub fn base_extra_list_items_in_list(e: &mut Engine, this: Ptr<BaseExtraList>) -> i32 {
    lock(e, NAME_ITEMS_IN_LIST);
    let mut count = 0;
    let mut current: Ptr<BSExtraData> = e.get(this, BaseExtraList::pHead).cast();
    while !current.is_null() {
        count += 1;
        current = get_next(e, current);
    }
    unlock(e);
    count
}

// Translated from 0040fe80 (decompiled, FalloutNV.exe 1.4.0.525)
/// `BaseExtraList::HasExtra` (Xbox PDB): whether the type bitmap has the bit
/// of `extra_type` (false for a type whose byte is past the 21-byte bitmap).
pub fn base_extra_list_has_extra(e: &mut Engine, this: Ptr<BaseExtraList>, extra_type: u8) -> bool {
    let byte_index = (extra_type >> 3) as u32;
    if byte_index >= FLAGS_LEN {
        return false;
    }
    let flags = e.mem.u8(this.addr() + 8 + byte_index);
    flags & (1 << (extra_type & 7)) != 0
}

// Translated from 0040fee0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Sets (`set`) or clears the bit of `extra_type` in the type bitmap; a type
/// whose byte is past the 21-byte bitmap is ignored.
pub fn fn_0040fee0(e: &mut Engine, this: Ptr<BaseExtraList>, extra_type: u8, set: bool) {
    let byte_index = (extra_type >> 3) as u32;
    if byte_index >= FLAGS_LEN {
        return;
    }
    let address = this.addr() + 8 + byte_index;
    let bit = 1u8 << (extra_type & 7);
    let flags = e.mem.u8(address);
    e.mem
        .set_u8(address, if set { flags | bit } else { !bit & flags });
}

// Translated from 0040ff60 (decompiled, FalloutNV.exe 1.4.0.525)
/// `BaseExtraList::AddExtra` (Xbox PDB): links `extra` into the chain and
/// sets its type bit. The types of `fn_0040f980` go to the head (the old head
/// becomes the new extra data's next), every other type to the end. Returns
/// `extra`.
pub fn base_extra_list_add_extra(
    e: &mut Engine,
    this: Ptr<BaseExtraList>,
    extra: Ptr<BSExtraData>,
) -> Ptr<BSExtraData> {
    lock(e, NAME_ADD_EXTRA);
    let extra_type = get_type(e, extra);
    let head: Ptr<BSExtraData> = e.get(this, BaseExtraList::pHead).cast();
    if fn_0040f980(e, extra_type as u32) {
        if !head.is_null() {
            set_next(e, extra, head);
        }
        e.set(this, BaseExtraList::pHead, extra.cast());
    } else if head.is_null() {
        e.set(this, BaseExtraList::pHead, extra.cast());
    } else {
        let mut last = head;
        loop {
            let next = get_next(e, last);
            if next.is_null() {
                break;
            }
            last = get_next(e, last);
        }
        set_next(e, last, extra);
    }
    let extra_type = get_type(e, extra);
    fn_0040fee0(e, this, extra_type, true);
    unlock(e);
    extra
}

// Translated from 00410020 (decompiled, FalloutNV.exe 1.4.0.525)
/// `BaseExtraList::RemoveExtra` (Xbox PDB), the overload that takes the extra
/// data: unlinks `extra` from the chain, drops it from the caches, then
/// deletes it (`destroy`) or only detaches it (next set to null), clears its
/// type bit and bumps the dirty counters. A null `extra` does nothing.
pub fn base_extra_list_remove_extra(
    e: &mut Engine,
    this: Ptr<BaseExtraList>,
    extra: Ptr<BSExtraData>,
    destroy: bool,
) {
    if extra.is_null() {
        return;
    }
    lock(e, NAME_REMOVE_EXTRA);
    let extra_type = get_type(e, extra);
    let previous = base_extra_list_get_prev_extra_data(e, this, extra_type);
    if previous.is_null() {
        let next = get_next(e, extra);
        e.set(this, BaseExtraList::pHead, next.cast());
    } else {
        let next = get_next(e, extra);
        set_next(e, previous, next);
    }
    base_extra_list_clear_last_extra(e, this, extra_type as i32);
    if destroy {
        delete_extra(e, extra);
    } else {
        set_next(e, extra, Ptr::NULL);
    }
    fn_0040fee0(e, this, extra_type, false);
    bump_dirty_counters(e);
    unlock(e);
}

// Translated from 00410140 (decompiled, FalloutNV.exe 1.4.0.525)
/// `BaseExtraList::RemoveExtra` (Xbox PDB, `_ov2`), the overload that takes a
/// type: finds the first extra data of that type, unlinks and deletes it, and
/// clears the type bit whether or not one was found.
pub fn base_extra_list_remove_extra_ov2(e: &mut Engine, this: Ptr<BaseExtraList>, extra_type: u8) {
    lock(e, NAME_REMOVE_EXTRA);
    let mut previous: Ptr<BSExtraData> = Ptr::NULL;
    let mut current: Ptr<BSExtraData> = e.call(SIMPLE_LIST_NEXT, &args![this]).ptr();
    while !current.is_null() && get_type(e, current) != extra_type {
        previous = current;
        current = get_next(e, current);
    }
    if !current.is_null() {
        if previous.is_null() {
            let next = get_next(e, current);
            e.set(this, BaseExtraList::pHead, next.cast());
        } else {
            let next = get_next(e, current);
            set_next(e, previous, next);
        }
        base_extra_list_clear_last_extra(e, this, extra_type as i32);
        delete_extra(e, current);
    }
    fn_0040fee0(e, this, extra_type, false);
    unlock(e);
}

// Translated from 00410220 (decompiled, FalloutNV.exe 1.4.0.525)
/// `BaseExtraList::GetExtraData` (Xbox PDB): the first extra data of
/// `extra_type`, or null. Answers from the type bitmap and this thread's
/// cache when it can; otherwise walks the chain under the lock and caches
/// what it finds.
pub fn base_extra_list_get_extra_data(
    e: &mut Engine,
    this: Ptr<BaseExtraList>,
    extra_type: u8,
) -> Ptr<BSExtraData> {
    if !base_extra_list_has_extra(e, this, extra_type) {
        return Ptr::NULL;
    }
    let mut found = fn_0040f9e0(e, this, extra_type as i32);
    if found.is_null() {
        lock(e, NAME_GET_EXTRA_DATA);
        let mut current: Ptr<BSExtraData> = e.get(this, BaseExtraList::pHead).cast();
        while !current.is_null() {
            if get_type(e, current) == extra_type {
                fn_0040fa40(e, this, current);
                found = current;
                break;
            }
            current = get_next(e, current);
        }
        unlock(e);
    }
    found
}

// Translated from 004102d0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `BaseExtraList::GetPrevExtraData` (Xbox PDB): the extra data that comes
/// right before the first one of `extra_type`; null when that one is the head,
/// when the type bitmap says there is none, or when the walk runs off the end
/// (then it is the last extra data).
pub fn base_extra_list_get_prev_extra_data(
    e: &mut Engine,
    this: Ptr<BaseExtraList>,
    extra_type: u8,
) -> Ptr<BSExtraData> {
    if !base_extra_list_has_extra(e, this, extra_type) {
        return Ptr::NULL;
    }
    lock(e, NAME_GET_PREV_EXTRA_DATA);
    let mut current: Ptr<BSExtraData> = e.get(this, BaseExtraList::pHead).cast();
    let mut previous: Ptr<BSExtraData> = Ptr::NULL;
    while !current.is_null() && get_type(e, current) != extra_type {
        previous = current;
        current = get_next(e, current);
    }
    unlock(e);
    previous
}

// Translated from 00410360 (decompiled, FalloutNV.exe 1.4.0.525)
/// `ExtraDataList::ExtraDataList` (Xbox PDB): the `BaseExtraList`
/// constructor, then the `ExtraDataList` vtable. Returns `this`.
pub fn extra_data_list_extra_data_list(
    e: &mut Engine,
    this: Ptr<ExtraDataList>,
) -> Ptr<ExtraDataList> {
    fn_0040f740(e, this.cast());
    e.mem.set_u32(this.addr(), VTABLE_EXTRA_DATA_LIST);
    this
}

// Translated from 00410380 (decompiled, FalloutNV.exe 1.4.0.525)
/// `ExtraDataList::_scalar_deleting_destructor_`: runs the destructor and
/// frees the object when bit 0 of `flags` is set. Returns `this`.
pub fn fn_00410380(e: &mut Engine, this: Ptr<ExtraDataList>, flags: u32) -> Ptr<ExtraDataList> {
    fn_004103b0(e, this);
    finish_scalar_deleting_destructor(e, this.cast(), flags).cast()
}

// Translated from 004103b0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `ExtraDataList::~ExtraDataList`: sets the `ExtraDataList` vtable, then
/// runs the `BaseExtraList` destructor.
pub fn fn_004103b0(e: &mut Engine, this: Ptr<ExtraDataList>) {
    e.mem.set_u32(this.addr(), VTABLE_EXTRA_DATA_LIST);
    fn_0040f7b0(e, this.cast());
}

// ---------------------------------------------------------------------------
// AddExtraCopy

/// Copies whose setter takes the 32-bit word at `extra + 0x0C`:
/// `list.Setter(word)`. (type, setter address)
const COPY_WORD: &[(u8, u32)] = &[
    (0x03, 0x0041_e160),
    (0x07, 0x0041_bd10),
    (0x08, 0x0041_c190),
    (0x14, 0x0041_d5b0),
    (0x1a, 0x0041_cc90),
    (0x1c, 0x0041_c7f0),
    (0x1e, 0x0042_1540),
    (0x20, 0x0041_8550),
    (0x21, 0x0041_9700),
    (0x22, 0x0041_97d0),
    (0x23, 0x0041_98a0),
    (0x3b, 0x0041_e250),
    (0x3c, 0x0042_1430),
    (0x3f, 0x0041_9d10),
    (0x44, 0x0042_1b70),
    (0x46, 0x0041_9dc0),
    (0x51, 0x0041_e440),
    (0x59, 0x0041_c290),
    (0x63, 0x0042_1e40),
    (0x67, 0x0042_1d50),
    (0x74, 0x0042_1c60),
    (0x81, 0x0041_c090),
];

/// Copies whose setter takes the `float` at `extra + 0x0C`.
const COPY_FLOAT: &[(u8, u32)] = &[
    (0x25, 0x0041_9970),
    (0x27, 0x0041_9bb0),
    (0x28, 0x0041_9c60),
    (0x30, 0x0041_9fb0),
    (0x5c, 0x0042_2220),
    (0x5d, 0x0042_2350),
    (0x7a, 0x0041_b580),
];

/// Copies whose setter takes the byte at `extra + 0x0C`.
const COPY_BYTE: &[(u8, u32)] = &[
    (0x0e, 0x0041_b3d0),
    (0x26, 0x0041_9a20),
    (0x31, 0x0041_ac30),
    (0x4a, 0x0042_dde0),
];

/// Copies whose setter takes the word that `READ_WORD(extra + 0x0C)` reads.
const COPY_READ_WORD: &[(u8, u32)] = &[
    (0x02, 0x0041_b7e0),
    (0x61, 0x0042_2050),
    (0x71, 0x0042_2150),
    (0x79, 0x0042_0f00),
];

/// Copies whose setter takes the address of the payload, `extra + 0x0C`.
const COPY_PAYLOAD_ADDRESS: &[(u8, u32)] = &[(0x13, 0x0041_d950), (0x68, 0x0041_8310)];

/// Copies that make a new payload object: (type, size, constructor,
/// copy-from-source `object.Copy(source word)`, list setter).
const COPY_NEW_OBJECT: &[(u8, u32, u32, u32, u32)] = &[
    (0x2b, 0x20, 0x0043_a160, 0x0043_a810, 0x0041_9120),
    (0x2c, 0x14, 0x0043_8bb0, 0x0043_8df0, 0x0041_9250),
    (0x6f, 0x14, 0x0067_c650, 0x0067_c6f0, 0x0041_fc10),
    (0x8c, 0x15c, 0x0058_efd0, 0x0058_f4c0, 0x0041_c390),
    (0x90, 0x34, 0x0058_9450, 0x0058_9770, 0x0041_9380),
    (0x91, 0x08, 0x0068_0890, 0x0045_34f0, 0x0041_94b0),
];

/// `new` and construct: allocates `size` bytes and runs `construct` on the
/// block, or gives null when the allocation failed.
fn new_object(e: &mut Engine, size: u32, construct: impl FnOnce(&mut Engine, u32) -> u32) -> u32 {
    let block = e.call(OPERATOR_NEW, &args![size]).u32();
    if block == 0 {
        0
    } else {
        construct(e, block)
    }
}

/// The extra data of `extra_type` in `list`, created with `new_object` and
/// added when the list has none.
fn get_or_add_extra(
    e: &mut Engine,
    list: Ptr<ExtraDataList>,
    extra_type: u8,
    size: u32,
    construct: impl FnOnce(&mut Engine, u32) -> u32,
) -> Ptr<BSExtraData> {
    let mut existing = base_extra_list_get_extra_data(e, list.cast(), extra_type);
    if existing.is_null() {
        existing = Ptr::new(new_object(e, size, construct));
        base_extra_list_add_extra(e, list.cast(), existing);
    }
    existing
}

/// The word of the source extra data's payload at `extra + offset`.
fn payload(e: &Engine, extra: Ptr<BSExtraData>, offset: u32) -> u32 {
    e.mem.u32(extra.addr() + offset)
}

/// Copies the 32-bit words `[from, to)` (byte offsets) of `source` to the same
/// offsets of `target`.
fn copy_words(e: &mut Engine, source: u32, target: u32, from: u32, to: u32) {
    for offset in (from..to).step_by(4) {
        let word = e.mem.u32(source + offset);
        e.mem.set_u32(target + offset, word);
    }
}

// Translated from 004103d0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `ExtraDataList::AddExtraCopy` (Xbox PDB): puts a copy of `extra` into this
/// list, using the setter of the list that matches the extra data's type
/// (`this.SetX(payload of extra)`), or for the types that keep a list or a
/// structure, adding to the existing extra data of that type (creating it
/// first) or building a copy of the structure. Does nothing for a null
/// `extra` or one `IsCopyableExtra` (`00411f20`) refuses. A type with no
/// copy function is reported to the log.
///
/// The types are dispatched by a jump table in the exe (82 types, `0x02` to
/// `0x92`); the setters of the same shape are the `COPY_*` tables above, the
/// rest are the arms of the `match`. `payload(extra, 0x0C)` is the first word
/// after the `BSExtraData` base.
pub fn extra_data_list_add_extra_copy(
    e: &mut Engine,
    this: Ptr<ExtraDataList>,
    extra: Ptr<BSExtraData>,
) {
    if extra.is_null() || !e.call(0x0041_1f20, &args![this, extra]).bool() {
        return;
    }
    let extra_type = get_type(e, extra);
    let list = this.addr();

    if let Some(&(_, setter)) = COPY_WORD.iter().find(|entry| entry.0 == extra_type) {
        let word = payload(e, extra, 0x0c);
        e.call(setter, &args![list, word]);
        return;
    }
    if let Some(&(_, setter)) = COPY_FLOAT.iter().find(|entry| entry.0 == extra_type) {
        let value = f32::from_bits(payload(e, extra, 0x0c));
        e.call(setter, &args![list, value]);
        return;
    }
    if let Some(&(_, setter)) = COPY_BYTE.iter().find(|entry| entry.0 == extra_type) {
        let byte = e.mem.u8(extra.addr() + 0x0c);
        e.call(setter, &args![list, byte]);
        return;
    }
    if let Some(&(_, setter)) = COPY_READ_WORD.iter().find(|entry| entry.0 == extra_type) {
        let word = e.call(READ_WORD, &args![extra.addr() + 0x0c]).u32();
        e.call(setter, &args![list, word]);
        return;
    }
    if let Some(&(_, setter)) = COPY_PAYLOAD_ADDRESS
        .iter()
        .find(|entry| entry.0 == extra_type)
    {
        e.call(setter, &args![list, extra.addr() + 0x0c]);
        return;
    }
    if let Some(&(_, size, construct, copy, setter)) =
        COPY_NEW_OBJECT.iter().find(|entry| entry.0 == extra_type)
    {
        let source = payload(e, extra, 0x0c);
        let object = new_object(e, size, |e, block| e.call(construct, &args![block]).u32());
        e.call(copy, &args![object, source]);
        e.call(setter, &args![list, object]);
        return;
    }

    match extra_type {
        0x0a => {
            // SetCanopyShadowMask(word, handle, &result) hands back the
            // structure it made through `result`; the two words at +0x14 and
            // +0x18 of the source are then copied into its first two words.
            let word = payload(e, extra, 0x0c);
            let handle = e.call(READ_WORD, &args![extra.addr() + 0x10]).u32();
            let result = e.mem.alloc(4);
            e.call(0x0041_c490, &args![list, word, handle, result]);
            let created = e.mem.u32(result);
            let second = payload(e, extra, 0x18);
            e.mem.set_u32(created + 4, second);
            let first = payload(e, extra, 0x14);
            e.mem.set_u32(created, first);
            e.mem.free(result);
        }
        0x0d => {
            let (first, second) = (payload(e, extra, 0x0c), payload(e, extra, 0x10));
            e.call(0x0041_9ed0, &args![list, first]);
            e.call(0x0041_9f80, &args![list, second]);
        }
        0x16 => {
            e.call(0x0041_aa20, &args![list, 1u32, 0u32]);
        }
        0x17 => {
            e.call(0x0041_aa20, &args![list, 1u32, 1u32]);
        }
        0x18 => {
            let word = payload(e, extra, 0x0c);
            let first_cast = e
                .call(
                    RT_DYNAMIC_CAST,
                    &args![word, 0u32, 0x0118_3028u32, 0x0118_3fd0u32, 0u32],
                )
                .u32();
            let second_cast = e
                .call(
                    RT_DYNAMIC_CAST,
                    &args![word, 0u32, 0x0118_3028u32, 0x0118_3fb4u32, 0u32],
                )
                .u32();
            let last = f32::from_bits(payload(e, extra, 0x1c));
            e.call(
                0x0041_ad00,
                &args![list, first_cast, second_cast, extra.addr() + 0x10, last],
            );
        }
        0x19 => {
            let words = [0x0cu32, 0x10, 0x14].map(|offset| payload(e, extra, offset));
            let bytes = [0x18u32, 0x19, 0x1a].map(|offset| e.mem.u8(extra.addr() + offset));
            e.call(
                0x0041_c930,
                &args![list, words[0], words[1], words[2], bytes[0], bytes[1], bytes[2]],
            );
        }
        0x1b => {
            let mut node = payload(e, extra, 0x0c);
            while node != 0 {
                let item = e.call(SIMPLE_LIST_ITEM, &args![node]).u32();
                let entry = e.mem.u32(item);
                if entry != 0 {
                    let (word, flag) = (e.mem.u32(entry), e.mem.u8(entry + 4));
                    e.call(0x0041_d700, &args![list, word, flag]);
                }
                node = e.call(SIMPLE_LIST_NEXT, &args![node]).u32();
            }
        }
        0x1d => {
            let mut node = payload(e, extra, 0x0c);
            while node != 0 {
                let item = e.call(SIMPLE_LIST_ITEM, &args![node]).u32();
                if e.mem.u32(item) == 0 {
                    break;
                }
                let item = e.call(SIMPLE_LIST_ITEM, &args![node]).u32();
                let follower = e.mem.u32(item);
                e.call(0x0042_2480, &args![list, follower]);
                node = e.call(SIMPLE_LIST_NEXT, &args![node]).u32();
            }
        }
        0x2a => {
            // ExtraLock: a copy of the 0x14-byte lock structure.
            let source = payload(e, extra, 0x0c);
            let object = new_object(e, 0x14, |e, block| fn_00411b00(e, Ptr::new(block)).addr());
            e.call(MEMCPY, &args![object, source, 0x14u32]);
            e.call(0x0041_9050, &args![list, object]);
        }
        0x2f => {
            let word = payload(e, extra, 0x0c);
            let flag = e.mem.u8(extra.addr() + 0x10);
            e.call(0x0041_d280, &args![list, word]);
            e.call(0x0041_d330, &args![list, flag]);
        }
        0x37 => {
            let word = payload(e, extra, 0x0c);
            let flag = e.mem.u8(extra.addr() + 0x10);
            e.call(0x0041_da40, &args![list, word]);
            e.call(0x0041_dc70, &args![list, flag]);
        }
        0x24 => {
            let short = e.mem.u16(extra.addr() + 0x0c);
            e.call(0x0041_9ad0, &args![list, short]);
        }
        0x3e => {
            e.call(0x0041_ab70, &args![list, 1u32]);
        }
        0x48 => {
            let value = e.mem.f32(extra.addr() + 0x0c);
            e.call(0x0042_2750, &args![list, 1u32, value]);
        }
        0x4c => {
            let existing = get_or_add_extra(e, this, 0x4c, 0x30, |e, block| {
                e.call(0x0043_5fa0, &args![block, 0u32]).u32()
            });
            // The payload from +0x0C to +0x2F (nine words) is copied whole.
            copy_words(e, extra.addr(), existing.addr(), 0x0c, 0x30);
        }
        0x53 => {
            let existing = get_or_add_extra(e, this, 0x53, 0x20, |e, block| {
                e.call(0x0043_38b0, &args![block]).u32()
            });
            e.call(0x0043_3b70, &args![existing, extra]);
        }
        0x55 => {}
        0x57 => {
            let existing = get_or_add_extra(e, this, 0x57, 0x14, |e, block| {
                e.call(0x0043_3ca0, &args![block]).u32()
            });
            e.call(0x0043_42b0, &args![existing, extra]);
        }
        0x5a => {
            e.call(0x0042_e2c0, &args![list, extra]);
        }
        0x5e => {
            e.call(0x0042_e760, &args![list]);
            let target = e.call(0x0042_e800, &args![list]).u32();
            let mut node = payload(e, extra, 0x0c);
            while node != 0 {
                let item = e.call(SIMPLE_LIST_ITEM, &args![node]).u32();
                if e.mem.u32(item) == 0 {
                    break;
                }
                let item = e.call(SIMPLE_LIST_ITEM, &args![node]).u32();
                let entry = e.mem.u32(item);
                if entry != 0 {
                    // The entry is a word and a signed byte.
                    let (word, small) = (e.mem.u32(entry), e.mem.i8(entry + 4) as i32);
                    e.call(0x0043_6f80, &args![target, word, small]);
                }
                node = e.call(SIMPLE_LIST_NEXT, &args![node]).u32();
            }
        }
        0x62 => {
            let source = payload(e, extra, 0x0c);
            let mut object = 0;
            if source != 0 {
                object = new_object(e, 0xc, |e, block| e.call(0x0043_8f50, &args![block]).u32());
                e.call(0x0043_9120, &args![object, source]);
            }
            e.call(0x0042_1f30, &args![list, object]);
        }
        0x65 => {
            let existing = get_or_add_extra(e, this, 0x65, 0x14, |e, block| {
                fn_00411b40(e, Ptr::new(block)).addr()
            });
            e.call(0x0043_46f0, &args![existing, extra]);
        }
        0x66 => {
            let existing = get_or_add_extra(e, this, 0x66, 0x14, |e, block| {
                fn_00411be0(e, Ptr::new(block)).addr()
            });
            e.call(0x0043_4c80, &args![existing, extra]);
        }
        0x6b => {
            let word = payload(e, extra, 0x0c);
            let converted = e.call(0x004a_4d40, &args![word]).u32();
            e.call(0x0041_fa60, &args![list, converted]);
        }
        0x6e => {
            let (first, second) = (payload(e, extra, 0x0c), payload(e, extra, 0x10));
            e.call(0x0042_eb60, &args![list, first, second]);
        }
        0x72 => {
            let word = payload(e, extra, 0x0c);
            let object = new_object(e, 4, |e, block| fn_00411e00(e, Ptr::new(block)).addr());
            let read = e.call(READ_WORD, &args![word]).u32();
            e.call(0x0053_7e90, &args![object, read]);
            e.call(0x0042_0fd0, &args![list, object]);
        }
        0x73 => {
            let mut node = payload(e, extra, 0x0c);
            while node != 0 {
                let item = e.call(SIMPLE_LIST_ITEM, &args![node]).u32();
                if e.mem.u32(item) == 0 {
                    break;
                }
                let item = e.call(SIMPLE_LIST_ITEM, &args![node]).u32();
                let entry = e.mem.u32(item);
                e.call(0x0042_ef20, &args![list, entry]);
                node = e.call(SIMPLE_LIST_NEXT, &args![node]).u32();
            }
        }
        0x76 => {
            let source = payload(e, extra, 0x0c);
            let object = new_object(e, 0x10, |e, block| fn_00411dc0(e, Ptr::new(block)).addr());
            copy_words(e, source, object, 0, 0x10);
            e.call(0x0041_fec0, &args![list, object]);
        }
        0x77 => {
            let source = payload(e, extra, 0x0c);
            for index in 0..2u32 {
                let value = e.mem.u32(source + 4 * index);
                e.call(0x0042_0a60, &args![list, index, value]);
            }
        }
        0x78 => {
            // The original builds a temporary (0x0045cec0 destroys it) when
            // `extra` is null, which the check at the top rules out.
            let temporary = e.mem.alloc(4);
            let address = if extra.is_null() {
                e.call(0x0063_3c90, &args![temporary, 0u32]).u32()
            } else {
                extra.addr() + 0x0c
            };
            let word = e.call(READ_WORD, &args![address]).u32();
            if extra.is_null() {
                e.call(0x0045_cec0, &args![temporary]);
            }
            e.mem.free(temporary);
            e.call(0x0042_0e00, &args![list, word]);
        }
        0x7b => {
            let room_data = payload(e, extra, 0x0c);
            let mut node = room_data + 8;
            while node != 0 {
                let item = e.call(SIMPLE_LIST_ITEM, &args![node]).u32();
                if e.mem.u32(item) == 0 {
                    break;
                }
                let item = e.call(SIMPLE_LIST_ITEM, &args![node]).u32();
                let entry = e.mem.u32(item);
                e.call(0x0042_0c10, &args![list, entry]);
                node = e.call(SIMPLE_LIST_NEXT, &args![node]).u32();
            }
            let flag = e.mem.u8(room_data + 0x10) != 0;
            e.call(0x0042_0870, &args![list, flag]);
        }
        0x80 => {
            e.call(0x0042_f200, &args![list, 1u32]);
        }
        0x84 => {
            let existing = get_or_add_extra(e, this, 0x84, 0x14, |e, block| {
                fn_00411c80(e, Ptr::new(block)).addr()
            });
            e.call(0x0043_4a70, &args![existing, extra]);
        }
        0x85 => {
            let existing = get_or_add_extra(e, this, 0x85, 0x14, |e, block| {
                fn_00411d20(e, Ptr::new(block)).addr()
            });
            e.call(0x0043_4ee0, &args![existing, extra]);
        }
        0x8b => {
            // The get-or-create is a call (0042f420) that adds the new extra
            // data to the list itself.
            let target = e.call(0x0042_f420, &args![list]).u32();
            copy_words(e, extra.addr(), target, 0x10, 0x1c);
            let word = payload(e, extra, 0x1c);
            e.mem.set_u32(target + 0x1c, word);
            let first = payload(e, extra, 0x0c);
            e.mem.set_u32(target + 0x0c, first);
            e.call(0x0047_0470, &args![target + 0x20]);
            let mut node = extra.addr() + 0x20;
            while node != 0 {
                let item = e.call(SIMPLE_LIST_ITEM, &args![node]).u32();
                if e.mem.u32(item) != 0 {
                    let item = e.call(SIMPLE_LIST_ITEM, &args![node]).u32();
                    e.call(0x0090_5820, &args![target + 0x20, item]);
                }
                node = e.call(SIMPLE_LIST_NEXT, &args![node]).u32();
            }
        }
        0x8d => {
            // Each weapon mod slot that is active in the source is set here.
            for mask in [1u8, 2, 4] {
                if fn_00411e20(e, extra, mask) {
                    e.call(0x0042_e380, &args![list, mask as u32]);
                }
            }
        }
        0x92 => {
            let existing = get_or_add_extra(e, this, 0x92, 0x14, |e, block| {
                fn_00411e40(e, Ptr::new(block)).addr()
            });
            e.vcall(existing.addr(), 0x0c, &args![extra]);
        }
        _ => {
            e.call(LOG_MESSAGE, &args![NO_COPY_MESSAGE, extra_type as u32]);
        }
    }
}

// Translated from 00411b00 (decompiled, FalloutNV.exe 1.4.0.525)
/// Initializes the 0x14-byte structure behind `ExtraLock` (its callers are
/// `DoorLock`, `SetFromDoorRef`, `AddLock`): byte +0x00, word +0x04, byte
/// +0x08, words +0x0C and +0x10, all zero. Returns `this`.
pub fn fn_00411b00(e: &mut Engine, this: Ptr) -> Ptr {
    e.mem.set_u8(this.addr(), 0);
    e.mem.set_u32(this.addr() + 4, 0);
    e.mem.set_u8(this.addr() + 8, 0);
    e.mem.set_u32(this.addr() + 0x10, 0);
    e.mem.set_u32(this.addr() + 0x0c, 0);
    this
}

/// The constructors of `ExtraReflectedRefs` & co: the base constructor with
/// the type, the vtable, and an empty `BSSimpleList` (item and next null,
/// `0096a2d0`) at +0x0C.
fn construct_ref_list_extra(
    e: &mut Engine,
    this: Ptr<ExtraRefList>,
    extra_type: u32,
    vtable: u32,
) -> Ptr<ExtraRefList> {
    e.call(BS_EXTRA_DATA_INIT, &args![this, extra_type]);
    e.mem.set_u32(this.addr(), vtable);
    e.call(0x0096_a2d0, &args![this.byte_add(0x0c)]);
    this
}

// Translated from 00411b40 (decompiled, FalloutNV.exe 1.4.0.525)
/// Constructor of `ExtraReflectedRefs` (type `0x65`; the exception-unwinding
/// frame is not translated). Returns `this`.
pub fn fn_00411b40(e: &mut Engine, this: Ptr<ExtraRefList>) -> Ptr<ExtraRefList> {
    construct_ref_list_extra(e, this, 0x65, VTABLE_EXTRA_REFLECTED_REFS)
}

// Translated from 00411bb0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `ExtraReflectedRefs::_scalar_deleting_destructor_` (Xbox PDB): runs the
/// destructor (`00434560`) and frees the object when bit 0 of `flags` is set.
/// Returns `this`.
pub fn extra_reflected_refs_scalar_deleting_destructor(
    e: &mut Engine,
    this: Ptr<ExtraRefList>,
    flags: u32,
) -> Ptr<ExtraRefList> {
    e.call(0x0043_4560, &args![this]);
    finish_scalar_deleting_destructor(e, this.cast(), flags).cast()
}

// Translated from 00411be0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Constructor of `ExtraReflectorRefs` (type `0x66`). Returns `this`.
pub fn fn_00411be0(e: &mut Engine, this: Ptr<ExtraRefList>) -> Ptr<ExtraRefList> {
    construct_ref_list_extra(e, this, 0x66, VTABLE_EXTRA_REFLECTOR_REFS)
}

// Translated from 00411c50 (decompiled, FalloutNV.exe 1.4.0.525)
/// `ExtraReflectorRefs::_scalar_deleting_destructor_` (Xbox PDB): runs the
/// destructor (`00434af0`) and frees the object when bit 0 of `flags` is set.
/// Returns `this`.
pub fn extra_reflector_refs_scalar_deleting_destructor(
    e: &mut Engine,
    this: Ptr<ExtraRefList>,
    flags: u32,
) -> Ptr<ExtraRefList> {
    e.call(0x0043_4af0, &args![this]);
    finish_scalar_deleting_destructor(e, this.cast(), flags).cast()
}

// Translated from 00411c80 (decompiled, FalloutNV.exe 1.4.0.525)
/// Constructor of `ExtraWaterLightRefs` (type `0x84`). Returns `this`.
pub fn fn_00411c80(e: &mut Engine, this: Ptr<ExtraRefList>) -> Ptr<ExtraRefList> {
    construct_ref_list_extra(e, this, 0x84, VTABLE_EXTRA_WATER_LIGHT_REFS)
}

// Translated from 00411cf0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `ExtraWaterLightRefs::_scalar_deleting_destructor_` (Xbox PDB): runs the
/// destructor (`00434950`) and frees the object when bit 0 of `flags` is set.
/// Returns `this`.
pub fn extra_water_light_refs_scalar_deleting_destructor(
    e: &mut Engine,
    this: Ptr<ExtraRefList>,
    flags: u32,
) -> Ptr<ExtraRefList> {
    e.call(0x0043_4950, &args![this]);
    finish_scalar_deleting_destructor(e, this.cast(), flags).cast()
}

// Translated from 00411d20 (decompiled, FalloutNV.exe 1.4.0.525)
/// Constructor of `ExtraLitWaterRefs` (type `0x85`). Returns `this`.
pub fn fn_00411d20(e: &mut Engine, this: Ptr<ExtraRefList>) -> Ptr<ExtraRefList> {
    construct_ref_list_extra(e, this, 0x85, VTABLE_EXTRA_LIT_WATER_REFS)
}

// Translated from 00411d90 (decompiled, FalloutNV.exe 1.4.0.525)
/// `ExtraLitWaterRefs::_scalar_deleting_destructor_` (Xbox PDB): runs the
/// destructor (`00434dc0`) and frees the object when bit 0 of `flags` is set.
/// Returns `this`.
pub fn extra_lit_water_refs_scalar_deleting_destructor(
    e: &mut Engine,
    this: Ptr<ExtraRefList>,
    flags: u32,
) -> Ptr<ExtraRefList> {
    e.call(0x0043_4dc0, &args![this]);
    finish_scalar_deleting_destructor(e, this.cast(), flags).cast()
}

// Translated from 00411dc0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Initializes a 16-byte structure of four words (the payload `AddExtraCopy`
/// copies for type `0x76`) to zero. Returns `this`.
pub fn fn_00411dc0(e: &mut Engine, this: Ptr) -> Ptr {
    for index in 0..4 {
        e.mem.set_u32(this.addr() + index * 4, 0);
    }
    this
}

// Translated from 00411e00 (decompiled, FalloutNV.exe 1.4.0.525)
/// Initializes the 4-byte structure `AddExtraCopy` builds for type `0x72`
/// to the value `0x16`. Returns `this`.
pub fn fn_00411e00(e: &mut Engine, this: Ptr) -> Ptr {
    e.mem.set_u32(this.addr(), 0x16);
    this
}

// Translated from 00411e20 (decompiled, FalloutNV.exe 1.4.0.525)
/// Whether any bit of `mask` is set in the byte at +0x0C of the extra data:
/// the weapon mod slots of `ExtraWeaponModFlags` (`cWeaponModsActive`, Xbox
/// PDB; its other caller is `GetWeaponModSlotActive`).
pub fn fn_00411e20(e: &mut Engine, this: Ptr<BSExtraData>, mask: u8) -> bool {
    e.mem.u8(this.addr() + 0x0c) & mask != 0
}

// Translated from 00411e40 (decompiled, FalloutNV.exe 1.4.0.525)
/// Constructor of the type `0x92` extra data (`0x14` bytes, `AddExtraCopy`
/// copies it with its virtual slot `+0x0C`): a `BSExtraData` whose word at
/// +0x0C is zero. Returns `this`.
pub fn fn_00411e40(e: &mut Engine, this: Ptr) -> Ptr {
    e.call(BS_EXTRA_DATA_INIT, &args![this, 0x92u32]);
    e.mem.set_u32(this.addr(), VTABLE_EXTRA_TYPE_92);
    e.mem.set_u32(this.addr() + 0x0c, 0);
    this
}

// Translated from 00411e70 (decompiled, FalloutNV.exe 1.4.0.525)
/// Scalar deleting destructor of the type `0x92` extra data: runs its
/// destructor (`00411ea0`, the next function of this unit) and frees the
/// object when bit 0 of `flags` is set. Returns `this`.
pub fn fn_00411e70(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    e.call(0x0041_1ea0, &args![this]);
    finish_scalar_deleting_destructor(e, this, flags)
}

/// This unit's translated functions, by exe address.
pub fn funcs() -> Vec<(u32, AbiFn)> {
    vec![
        entry!(
            0x0040f680,
            fn_0040f680(Ptr<ExtraNorthRotation>) -> Ptr<ExtraNorthRotation>
        ),
        entry!(
            0x0040f6b0,
            fn_0040f6b0(Ptr<ExtraDetachTime>) -> Ptr<ExtraDetachTime>
        ),
        entry!(
            0x0040f700,
            bs_extra_data_compare(Ptr<BSExtraData>, Ptr<BSExtraData>) -> bool
        ),
        entry!(
            0x0040f740,
            fn_0040f740(Ptr<BaseExtraList>) -> Ptr<BaseExtraList>
        ),
        entry!(
            0x0040f780,
            fn_0040f780(Ptr<BaseExtraList>, u32) -> Ptr<BaseExtraList>
        ),
        entry!(0x0040f7b0, fn_0040f7b0(Ptr<BaseExtraList>)),
        entry!(
            0x0040f7d0,
            base_extra_list_clear_last_extra(Ptr<BaseExtraList>, i32)
        ),
        entry!(
            0x0040f860,
            base_extra_list_clear_last_extra_all(Ptr<BaseExtraList>)
        ),
        entry!(0x0040f900, fn_0040f900()),
        entry!(0x0040f980, fn_0040f980(u32) -> bool),
        entry!(
            0x0040f9e0,
            fn_0040f9e0(Ptr<BaseExtraList>, i32) -> Ptr<BSExtraData>
        ),
        entry!(
            0x0040fa40,
            fn_0040fa40(Ptr<BaseExtraList>, Ptr<BSExtraData>)
        ),
        entry!(
            0x0040fae0,
            base_extra_list_remove_all(Ptr<BaseExtraList>, bool)
        ),
        entry!(
            0x0040fcb0,
            base_extra_list_remove_all_default(Ptr<BaseExtraList>, bool)
        ),
        entry!(
            0x0040fe20,
            base_extra_list_items_in_list(Ptr<BaseExtraList>) -> i32
        ),
        entry!(
            0x0040fe80,
            base_extra_list_has_extra(Ptr<BaseExtraList>, u8) -> bool
        ),
        entry!(0x0040fee0, fn_0040fee0(Ptr<BaseExtraList>, u8, bool)),
        entry!(
            0x0040ff60,
            base_extra_list_add_extra(Ptr<BaseExtraList>, Ptr<BSExtraData>) -> Ptr<BSExtraData>
        ),
        entry!(
            0x00410020,
            base_extra_list_remove_extra(Ptr<BaseExtraList>, Ptr<BSExtraData>, bool)
        ),
        entry!(
            0x00410140,
            base_extra_list_remove_extra_ov2(Ptr<BaseExtraList>, u8)
        ),
        entry!(
            0x00410220,
            base_extra_list_get_extra_data(Ptr<BaseExtraList>, u8) -> Ptr<BSExtraData>
        ),
        entry!(
            0x004102d0,
            base_extra_list_get_prev_extra_data(Ptr<BaseExtraList>, u8) -> Ptr<BSExtraData>
        ),
        entry!(
            0x00410360,
            extra_data_list_extra_data_list(Ptr<ExtraDataList>) -> Ptr<ExtraDataList>
        ),
        entry!(
            0x00410380,
            fn_00410380(Ptr<ExtraDataList>, u32) -> Ptr<ExtraDataList>
        ),
        entry!(0x004103b0, fn_004103b0(Ptr<ExtraDataList>)),
        entry!(
            0x004103d0,
            extra_data_list_add_extra_copy(Ptr<ExtraDataList>, Ptr<BSExtraData>)
        ),
        entry!(0x00411b00, fn_00411b00(Ptr) -> Ptr),
        entry!(
            0x00411b40,
            fn_00411b40(Ptr<ExtraRefList>) -> Ptr<ExtraRefList>
        ),
        entry!(
            0x00411bb0,
            extra_reflected_refs_scalar_deleting_destructor(
                Ptr<ExtraRefList>,
                u32,
            ) -> Ptr<ExtraRefList>
        ),
        entry!(
            0x00411be0,
            fn_00411be0(Ptr<ExtraRefList>) -> Ptr<ExtraRefList>
        ),
        entry!(
            0x00411c50,
            extra_reflector_refs_scalar_deleting_destructor(
                Ptr<ExtraRefList>,
                u32,
            ) -> Ptr<ExtraRefList>
        ),
        entry!(
            0x00411c80,
            fn_00411c80(Ptr<ExtraRefList>) -> Ptr<ExtraRefList>
        ),
        entry!(
            0x00411cf0,
            extra_water_light_refs_scalar_deleting_destructor(
                Ptr<ExtraRefList>,
                u32,
            ) -> Ptr<ExtraRefList>
        ),
        entry!(
            0x00411d20,
            fn_00411d20(Ptr<ExtraRefList>) -> Ptr<ExtraRefList>
        ),
        entry!(
            0x00411d90,
            extra_lit_water_refs_scalar_deleting_destructor(
                Ptr<ExtraRefList>,
                u32,
            ) -> Ptr<ExtraRefList>
        ),
        entry!(0x00411dc0, fn_00411dc0(Ptr) -> Ptr),
        entry!(0x00411e00, fn_00411e00(Ptr) -> Ptr),
        entry!(0x00411e20, fn_00411e20(Ptr<BSExtraData>, u8) -> bool),
        entry!(0x00411e40, fn_00411e40(Ptr) -> Ptr),
        entry!(0x00411e70, fn_00411e70(Ptr, u32) -> Ptr),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    type Log = Vec<(u32, Vec<u32>)>;

    /// Test vtable of the extra data (slot 0 the scalar deleting destructor,
    /// slot 3 the virtual copy `AddExtraCopy` calls for type `0x92`).
    const VTABLE: u32 = 0x0200_0000;
    const DESTRUCTOR: u32 = 0x0200_1000;
    const VIRTUAL_COPY: u32 = 0x0200_1004;

    fn returns(value: u32) -> Ret {
        Ret {
            eax: value,
            ..Ret::default()
        }
    }

    fn stub(e: &mut Engine, address: u32) {
        e.register(address, |_, _| Ret::default());
    }

    /// An engine with working doubles for the small callees every function of
    /// the unit uses: the extra data accessors, `memset`/`memcpy`, the lock
    /// (logged only), `operator new` and `delete`, the `BSExtraData`
    /// constructor, and the list node accessors.
    fn extra_engine() -> Engine {
        let mut e = Engine::new();
        e.map(0x011c_3000, 0x1000);
        e.put_vtable(VTABLE, &[DESTRUCTOR, 0, 0, VIRTUAL_COPY]);
        e.register(DESTRUCTOR, |_, _| Ret::default());
        e.register(VIRTUAL_COPY, |_, _| Ret::default());
        e.register(GET_TYPE, |e, a| returns(e.mem.u8(a[0] + 4) as u32));
        e.register(GET_NEXT, |e, a| returns(e.mem.u32(a[0] + 8)));
        e.register(SET_NEXT, |e, a| {
            e.mem.set_u32(a[0] + 8, a[1]);
            Ret::default()
        });
        e.register(SIMPLE_LIST_NEXT, |e, a| returns(e.mem.u32(a[0] + 4)));
        e.register(SIMPLE_LIST_ITEM, |_, a| returns(a[0]));
        e.register(READ_WORD, |e, a| returns(e.mem.u32(a[0])));
        e.register(MEMSET, |e, a| {
            for offset in 0..a[2] {
                e.mem.set_u8(a[0] + offset, a[1] as u8);
            }
            Ret::default()
        });
        e.register(MEMCPY, |e, a| {
            for offset in 0..a[2] {
                let byte = e.mem.u8(a[1] + offset);
                e.mem.set_u8(a[0] + offset, byte);
            }
            Ret::default()
        });
        e.register(BS_EXTRA_DATA_INIT, |e, a| {
            e.mem.set_u8(a[0] + 4, a[1] as u8);
            e.mem.set_u32(a[0] + 8, 0);
            returns(a[0])
        });
        e.register(OPERATOR_NEW, |e, a| returns(e.mem.alloc(a[0])));
        stub(&mut e, LOCK);
        stub(&mut e, UNLOCK);
        stub(&mut e, OPERATOR_DELETE);
        stub(&mut e, 0x0096_a2d0);
        e
    }

    /// An extra data of type `extra_type` with room for any payload.
    fn extra_of_type(e: &mut Engine, extra_type: u8) -> Ptr<BSExtraData> {
        let extra: Ptr<BSExtraData> = Ptr::new(e.mem.alloc(0x40));
        e.mem.set_u32(extra.addr(), VTABLE);
        e.set(extra, BSExtraData::cEtype, extra_type);
        extra
    }

    /// A list holding extra data of the given types, in order, with the type
    /// bitmap filled in.
    fn list_with(e: &mut Engine, types: &[u8]) -> (Ptr<BaseExtraList>, Vec<Ptr<BSExtraData>>) {
        let list: Ptr<BaseExtraList> = e.new_object();
        let extras: Vec<Ptr<BSExtraData>> = types.iter().map(|&ty| extra_of_type(e, ty)).collect();
        for (index, extra) in extras.iter().enumerate() {
            if let Some(next) = extras.get(index + 1) {
                e.set(*extra, BSExtraData::pNext, next.cast());
            }
            fn_0040fee0(e, list, types[index], true);
        }
        if let Some(first) = extras.first() {
            e.set(list, BaseExtraList::pHead, first.cast());
        }
        (list, extras)
    }

    fn chain_types(e: &Engine, list: Ptr<BaseExtraList>) -> Vec<u8> {
        let mut types = vec![];
        let mut current: Ptr<BSExtraData> = e.get(list, BaseExtraList::pHead).cast();
        while !current.is_null() {
            types.push(e.get(current, BSExtraData::cEtype));
            current = e.get(current, BSExtraData::pNext).cast();
        }
        types
    }

    fn flag_bytes(e: &Engine, list: Ptr<BaseExtraList>) -> Vec<u8> {
        e.mem.bytes(list.addr() + 8, FLAGS_LEN)
    }

    fn calls_to(log: &Log, address: u32) -> Vec<Vec<u32>> {
        log.iter()
            .filter(|(callee, _)| *callee == address)
            .map(|(_, args)| args.clone())
            .collect()
    }

    /// The objects the log shows deleted (the destructor called with 1).
    fn deleted(log: &Log) -> Vec<u32> {
        calls_to(log, DESTRUCTOR).iter().map(|a| a[0]).collect()
    }

    fn assert_locked_and_released(log: &Log) {
        let locks = calls_to(log, LOCK);
        assert!(!locks.is_empty());
        assert_eq!(locks.len(), calls_to(log, UNLOCK).len());
        assert!(locks.iter().all(|a| a[0] == EXTRA_CRIT_SECTION));
    }

    fn cache_of(e: &mut Engine) -> Ptr<BaseExtraListThreadCache> {
        thread_cache(e)
    }

    #[test]
    fn north_rotation_constructor_sets_type_vtable_and_zero() {
        let mut e = extra_engine();
        let extra: Ptr<ExtraNorthRotation> = e.new_object();
        e.set(extra, ExtraNorthRotation::fNorthRot, 3.5);
        let result = e.call(0x0040_f680, &args![extra]).ptr::<()>();
        assert_eq!(result.addr(), extra.addr());
        assert_eq!(e.mem.u8(extra.addr() + 4), 0x43);
        assert_eq!(e.mem.u32(extra.addr()), 0x0101_42a0);
        assert_eq!(e.get(extra, ExtraNorthRotation::fNorthRot), 0.0);
    }

    #[test]
    fn detach_time_constructor_sets_type_vtable_and_zero() {
        let mut e = extra_engine();
        let extra: Ptr<ExtraDetachTime> = e.new_object();
        e.set(extra, ExtraDetachTime::iTime, 99);
        e.call(0x0040_f6b0, &args![extra]);
        assert_eq!(e.mem.u8(extra.addr() + 4), 0x0b);
        assert_eq!(e.mem.u32(extra.addr()), 0x0101_42ac);
        assert_eq!(e.get(extra, ExtraDetachTime::iTime), 0);
    }

    #[test]
    fn compare_is_true_for_a_null_other_or_a_different_type() {
        let mut e = extra_engine();
        let first = extra_of_type(&mut e, 0x10);
        let same = extra_of_type(&mut e, 0x10);
        let other = extra_of_type(&mut e, 0x11);
        assert!(e.call(0x0040_f700, &args![first, Ptr::<()>::NULL]).bool());
        assert!(!e.call(0x0040_f700, &args![first, same]).bool());
        assert!(e.call(0x0040_f700, &args![first, other]).bool());
    }

    #[test]
    fn base_extra_list_constructor_clears_head_and_bitmap() {
        let mut e = extra_engine();
        let list: Ptr<BaseExtraList> = e.new_object();
        e.mem.write(list.addr() + 4, &[0xff; 0x1c]);
        e.call(0x0040_f740, &args![list]);
        assert_eq!(e.mem.u32(list.addr()), 0x0101_4300);
        assert_eq!(e.get(list, BaseExtraList::pHead), Ptr::NULL);
        assert_eq!(flag_bytes(&e, list), vec![0u8; 0x15]);
        // Bytes after the bitmap are not the constructor's.
        assert_eq!(e.mem.u8(list.addr() + 0x1d), 0xff);
    }

    #[test]
    fn base_extra_list_scalar_deleting_destructor_frees_only_on_bit_zero() {
        let mut e = extra_engine();
        let (list, extras) = list_with(&mut e, &[0x10, 0x11]);
        e.call_log = Some(vec![]);
        let result = e.call(0x0040_f780, &args![list, 0u32]).ptr::<()>();
        let log = e.call_log.take().unwrap();
        assert_eq!(result.addr(), list.addr());
        assert!(calls_to(&log, OPERATOR_DELETE).is_empty());
        assert_eq!(deleted(&log), vec![extras[0].addr(), extras[1].addr()]);
        assert_eq!(e.mem.u32(list.addr()), 0x0101_4300);

        e.call_log = Some(vec![]);
        e.call(0x0040_f780, &args![list, 1u32]);
        let log = e.call_log.take().unwrap();
        assert_eq!(calls_to(&log, OPERATOR_DELETE), vec![vec![list.addr()]]);
    }

    #[test]
    fn base_extra_list_destructor_sets_vtable_and_destroys_the_chain() {
        let mut e = extra_engine();
        let (list, extras) = list_with(&mut e, &[0x20]);
        e.call_log = Some(vec![]);
        e.call(0x0040_f7b0, &args![list]);
        let log = e.call_log.take().unwrap();
        assert_eq!(e.mem.u32(list.addr()), 0x0101_4300);
        assert_eq!(deleted(&log), vec![extras[0].addr()]);
        assert_eq!(e.get(list, BaseExtraList::pHead), Ptr::NULL);
    }

    #[test]
    fn clear_last_extra_forgets_only_the_cached_entry_of_the_cached_list() {
        let mut e = extra_engine();
        let list: Ptr<BaseExtraList> = e.new_object();
        let other: Ptr<BaseExtraList> = e.new_object();
        let cache = cache_of(&mut e);
        e.set(cache, BaseExtraListThreadCache::pLastExtraList, list.cast());
        e.mem.set_u32(cache_entry(cache, 5), 0x1234);
        e.mem.set_u32(cache_entry(cache, 6), 0x5678);

        e.call(0x0040_f7d0, &args![other, 5i32]);
        assert_eq!(e.mem.u32(cache_entry(cache, 5)), 0x1234);
        e.call(0x0040_f7d0, &args![list, 5i32]);
        assert_eq!(e.mem.u32(cache_entry(cache, 5)), 0);
        assert_eq!(e.mem.u32(cache_entry(cache, 6)), 0x5678);
        // Out of range: nothing cleared, no fault.
        e.call(0x0040_f7d0, &args![list, -1i32]);
        e.call(0x0040_f7d0, &args![list, 0x93i32]);
        assert_eq!(e.mem.u32(cache_entry(cache, 6)), 0x5678);
        // Four calls: both counters moved by four.
        assert_eq!(e.global::<u32>(DIRTY), 4);
        assert_eq!(e.get(cache, BaseExtraListThreadCache::iThreadDirty), 4);
    }

    #[test]
    fn clear_last_extra_all_empties_the_cache_of_the_cached_list() {
        let mut e = extra_engine();
        let list: Ptr<BaseExtraList> = e.new_object();
        let other: Ptr<BaseExtraList> = e.new_object();
        let cache = cache_of(&mut e);
        e.set(cache, BaseExtraListThreadCache::pLastExtraList, list.cast());
        e.mem.set_u32(cache_entry(cache, 0), 1);
        e.mem.set_u32(cache_entry(cache, 0x92), 2);

        e.call(0x0040_f860, &args![other]);
        assert_eq!(e.mem.u32(cache_entry(cache, 0)), 1);
        e.call(0x0040_f860, &args![list]);
        assert_eq!(e.mem.u32(cache_entry(cache, 0)), 0);
        assert_eq!(e.mem.u32(cache_entry(cache, 0x92)), 0);
        assert_eq!(
            e.get(cache, BaseExtraListThreadCache::pLastExtraList),
            Ptr::NULL
        );
        assert_eq!(e.global::<u32>(DIRTY), 2);
        assert_eq!(e.get(cache, BaseExtraListThreadCache::iThreadDirty), 2);
    }

    #[test]
    fn stale_thread_cache_is_dropped_and_a_fresh_one_kept() {
        let mut e = extra_engine();
        let list: Ptr<BaseExtraList> = e.new_object();
        let cache = cache_of(&mut e);
        e.set(cache, BaseExtraListThreadCache::pLastExtraList, list.cast());
        e.mem.set_u32(cache_entry(cache, 3), 0x77);
        e.set(cache, BaseExtraListThreadCache::iThreadDirty, 5);
        e.set_global(DIRTY, 5u32);
        e.call(0x0040_f900, &args![]);
        assert_eq!(e.mem.u32(cache_entry(cache, 3)), 0x77);
        assert_eq!(
            e.get(cache, BaseExtraListThreadCache::pLastExtraList),
            list.cast()
        );

        e.set_global(DIRTY, 9u32);
        e.call(0x0040_f900, &args![]);
        assert_eq!(e.mem.u32(cache_entry(cache, 3)), 0);
        assert_eq!(
            e.get(cache, BaseExtraListThreadCache::pLastExtraList),
            Ptr::NULL
        );
        assert_eq!(e.get(cache, BaseExtraListThreadCache::iThreadDirty), 9);
    }

    #[test]
    fn head_inserted_types_are_exactly_the_jump_table_entries() {
        let mut e = extra_engine();
        let yes: Vec<u32> = (0..0x100u32)
            .filter(|&t| e.call(0x0040_f980, &args![t]).bool())
            .collect();
        assert_eq!(yes, vec![0x0c, 0x0d, 0x0e, 0x15, 0x2b]);
    }

    #[test]
    fn cached_lookup_needs_the_cached_list_a_valid_type_and_a_fresh_cache() {
        let mut e = extra_engine();
        let list: Ptr<BaseExtraList> = e.new_object();
        let other: Ptr<BaseExtraList> = e.new_object();
        let cache = cache_of(&mut e);
        e.set(cache, BaseExtraListThreadCache::pLastExtraList, list.cast());
        e.mem.set_u32(cache_entry(cache, 0x10), 0xabcd);
        assert_eq!(e.call(0x0040_f9e0, &args![list, 0x10i32]).u32(), 0xabcd);
        assert_eq!(e.call(0x0040_f9e0, &args![other, 0x10i32]).u32(), 0);
        assert_eq!(e.call(0x0040_f9e0, &args![list, 0x93i32]).u32(), 0);
        assert_eq!(e.call(0x0040_f9e0, &args![list, -1i32]).u32(), 0);
        // A global dirty counter that moved on drops the cache first.
        e.set_global(DIRTY, 1u32);
        assert_eq!(e.call(0x0040_f9e0, &args![list, 0x10i32]).u32(), 0);
    }

    #[test]
    fn caching_an_extra_switches_the_cached_list() {
        let mut e = extra_engine();
        let list: Ptr<BaseExtraList> = e.new_object();
        let other: Ptr<BaseExtraList> = e.new_object();
        let first = extra_of_type(&mut e, 0x20);
        let second = extra_of_type(&mut e, 0x21);
        let cache = cache_of(&mut e);

        e.call(0x0040_fa40, &args![Ptr::<()>::NULL, first]);
        e.call(0x0040_fa40, &args![list, Ptr::<()>::NULL]);
        assert_eq!(e.mem.u32(cache_entry(cache, 0x20)), 0);

        e.call(0x0040_fa40, &args![list, first]);
        assert_eq!(e.mem.u32(cache_entry(cache, 0x20)), first.addr());
        e.call(0x0040_fa40, &args![list, second]);
        assert_eq!(e.mem.u32(cache_entry(cache, 0x20)), first.addr());
        assert_eq!(e.mem.u32(cache_entry(cache, 0x21)), second.addr());
        // Another list: the cache starts over.
        e.call(0x0040_fa40, &args![other, second]);
        assert_eq!(e.mem.u32(cache_entry(cache, 0x20)), 0);
        assert_eq!(e.mem.u32(cache_entry(cache, 0x21)), second.addr());
        assert_eq!(
            e.get(cache, BaseExtraListThreadCache::pLastExtraList),
            other.cast()
        );
    }

    #[test]
    fn remove_all_destroys_the_chain_or_only_forgets_it() {
        let mut e = extra_engine();
        let (list, extras) = list_with(&mut e, &[0x10, 0x20, 0x30]);
        e.call_log = Some(vec![]);
        e.call(0x0040_fae0, &args![list, true]);
        let log = e.call_log.take().unwrap();
        assert_eq!(
            deleted(&log),
            extras.iter().map(|x| x.addr()).collect::<Vec<_>>()
        );
        assert_eq!(e.get(list, BaseExtraList::pHead), Ptr::NULL);
        assert_eq!(flag_bytes(&e, list), vec![0u8; 0x15]);
        assert_locked_and_released(&log);
        assert_eq!(calls_to(&log, LOCK)[0][1], NAME_REMOVE_ALL);

        let (list, _) = list_with(&mut e, &[0x10, 0x20]);
        e.call_log = Some(vec![]);
        e.call(0x0040_fae0, &args![list, false]);
        let log = e.call_log.take().unwrap();
        assert!(deleted(&log).is_empty());
        assert_eq!(e.get(list, BaseExtraList::pHead), Ptr::NULL);
        assert_eq!(flag_bytes(&e, list), vec![0u8; 0x15]);
    }

    #[test]
    fn remove_all_default_keeps_the_persistent_types() {
        let mut e = extra_engine();
        // 0x2e and 0x65 stay; 0x10, 0x20 and 0x30 go, wherever they are.
        let (list, extras) = list_with(&mut e, &[0x10, 0x2e, 0x20, 0x65, 0x30]);
        e.call_log = Some(vec![]);
        e.call(0x0040_fcb0, &args![list, true]);
        let log = e.call_log.take().unwrap();
        assert_eq!(chain_types(&e, list), vec![0x2e, 0x65]);
        assert_eq!(
            deleted(&log),
            vec![extras[0].addr(), extras[2].addr(), extras[4].addr()]
        );
        assert!(!base_extra_list_has_extra(&mut e, list, 0x10));
        assert!(base_extra_list_has_extra(&mut e, list, 0x2e));
        assert!(base_extra_list_has_extra(&mut e, list, 0x65));
        assert_locked_and_released(&log);
        assert_eq!(calls_to(&log, LOCK)[0][1], NAME_REMOVE_ALL_DEFAULT);

        // Without destroy the removed ones are unlinked but not deleted.
        let (list, _) = list_with(&mut e, &[0x10, 0x84]);
        e.call_log = Some(vec![]);
        e.call(0x0040_fcb0, &args![list, false]);
        let log = e.call_log.take().unwrap();
        assert!(deleted(&log).is_empty());
        assert_eq!(chain_types(&e, list), vec![0x84]);
    }

    #[test]
    fn items_in_list_counts_the_chain_under_the_lock() {
        let mut e = extra_engine();
        let (empty, _) = list_with(&mut e, &[]);
        let (list, _) = list_with(&mut e, &[1, 2, 3]);
        e.call_log = Some(vec![]);
        assert_eq!(e.call(0x0040_fe20, &args![empty]).i32(), 0);
        assert_eq!(e.call(0x0040_fe20, &args![list]).i32(), 3);
        let log = e.call_log.take().unwrap();
        assert_locked_and_released(&log);
        assert_eq!(calls_to(&log, LOCK)[0][1], NAME_ITEMS_IN_LIST);
    }

    #[test]
    fn has_extra_tests_the_bit_and_ignores_types_past_the_bitmap() {
        let mut e = extra_engine();
        let list: Ptr<BaseExtraList> = e.new_object();
        e.mem.set_u8(list.addr() + 8 + 2, 0b0000_0100);
        e.mem.set_u8(list.addr() + 8 + 20, 0b1000_0000);
        assert!(e.call(0x0040_fe80, &args![list, 0x12u8]).bool());
        assert!(!e.call(0x0040_fe80, &args![list, 0x11u8]).bool());
        assert!(!e.call(0x0040_fe80, &args![list, 0x13u8]).bool());
        // Last bit of the last byte: type 0xA7. Type 0xA8 is past the end.
        assert!(e.call(0x0040_fe80, &args![list, 0xa7u8]).bool());
        e.mem.set_u8(list.addr() + 8 + 21, 0xff);
        assert!(!e.call(0x0040_fe80, &args![list, 0xa8u8]).bool());
    }

    #[test]
    fn flag_setter_sets_clears_and_ignores_types_past_the_bitmap() {
        let mut e = extra_engine();
        let list: Ptr<BaseExtraList> = e.new_object();
        e.call(0x0040_fee0, &args![list, 0x2bu8, true]);
        assert_eq!(e.mem.u8(list.addr() + 8 + 5), 0b0000_1000);
        e.call(0x0040_fee0, &args![list, 0x2cu8, true]);
        assert_eq!(e.mem.u8(list.addr() + 8 + 5), 0b0001_1000);
        e.call(0x0040_fee0, &args![list, 0x2bu8, false]);
        assert_eq!(e.mem.u8(list.addr() + 8 + 5), 0b0001_0000);
        e.call(0x0040_fee0, &args![list, 0xffu8, true]);
        assert_eq!(flag_bytes(&e, list)[5], 0b0001_0000);
        assert_eq!(e.mem.u8(list.addr() + 8 + 0x1f), 0);
    }

    #[test]
    fn add_extra_appends_ordinary_types_and_sets_their_bit() {
        let mut e = extra_engine();
        let (list, _) = list_with(&mut e, &[]);
        let first = extra_of_type(&mut e, 0x10);
        let second = extra_of_type(&mut e, 0x11);
        let third = extra_of_type(&mut e, 0x12);
        e.call_log = Some(vec![]);
        let result = e.call(0x0040_ff60, &args![list, first]).ptr::<()>();
        assert_eq!(result.addr(), first.addr());
        e.call(0x0040_ff60, &args![list, second]);
        e.call(0x0040_ff60, &args![list, third]);
        let log = e.call_log.take().unwrap();
        assert_eq!(chain_types(&e, list), vec![0x10, 0x11, 0x12]);
        assert_eq!(e.get(list, BaseExtraList::pHead), first.cast());
        assert!(base_extra_list_has_extra(&mut e, list, 0x11));
        assert_locked_and_released(&log);
        assert_eq!(calls_to(&log, LOCK)[0][1], NAME_ADD_EXTRA);
    }

    #[test]
    fn add_extra_puts_the_head_types_in_front() {
        let mut e = extra_engine();
        let (list, _) = list_with(&mut e, &[0x10, 0x11]);
        let front = extra_of_type(&mut e, 0x2b);
        e.call(0x0040_ff60, &args![list, front]);
        assert_eq!(chain_types(&e, list), vec![0x2b, 0x10, 0x11]);
        assert!(base_extra_list_has_extra(&mut e, list, 0x2b));
        // Into an empty list too.
        let (empty, _) = list_with(&mut e, &[]);
        let alone = extra_of_type(&mut e, 0x0e);
        e.call(0x0040_ff60, &args![empty, alone]);
        assert_eq!(chain_types(&e, empty), vec![0x0e]);
    }

    #[test]
    fn remove_extra_unlinks_deletes_and_bumps_the_counters() {
        let mut e = extra_engine();
        let (list, extras) = list_with(&mut e, &[0x10, 0x11, 0x12]);
        let cache = cache_of(&mut e);
        e.set(cache, BaseExtraListThreadCache::pLastExtraList, list.cast());
        e.mem.set_u32(cache_entry(cache, 0x11), extras[1].addr());
        e.call_log = Some(vec![]);
        e.call(0x0041_0020, &args![list, extras[1], true]);
        let log = e.call_log.take().unwrap();
        assert_eq!(chain_types(&e, list), vec![0x10, 0x12]);
        assert_eq!(deleted(&log), vec![extras[1].addr()]);
        assert!(!base_extra_list_has_extra(&mut e, list, 0x11));
        assert_eq!(e.mem.u32(cache_entry(cache, 0x11)), 0);
        // ClearLastExtra and the final bump: two each.
        assert_eq!(e.global::<u32>(DIRTY), 2);
        assert_eq!(e.get(cache, BaseExtraListThreadCache::iThreadDirty), 2);
        assert_locked_and_released(&log);
        assert_eq!(calls_to(&log, LOCK)[0][1], NAME_REMOVE_EXTRA);
    }

    #[test]
    fn remove_extra_of_the_head_without_destroy_detaches_it() {
        let mut e = extra_engine();
        let (list, extras) = list_with(&mut e, &[0x10, 0x11]);
        e.call_log = Some(vec![]);
        e.call(0x0041_0020, &args![list, extras[0], false]);
        let log = e.call_log.take().unwrap();
        assert!(deleted(&log).is_empty());
        assert_eq!(chain_types(&e, list), vec![0x11]);
        assert_eq!(e.get(extras[0], BSExtraData::pNext), Ptr::NULL);
        assert!(!base_extra_list_has_extra(&mut e, list, 0x10));
    }

    #[test]
    fn remove_extra_of_null_does_nothing() {
        let mut e = extra_engine();
        let (list, _) = list_with(&mut e, &[0x10]);
        e.call_log = Some(vec![]);
        e.call(0x0041_0020, &args![list, Ptr::<()>::NULL, true]);
        let log = e.call_log.take().unwrap();
        assert_eq!(log.len(), 1);
        assert_eq!(chain_types(&e, list), vec![0x10]);
    }

    #[test]
    fn remove_extra_by_type_deletes_the_first_match_and_always_clears_the_bit() {
        let mut e = extra_engine();
        let (list, extras) = list_with(&mut e, &[0x10, 0x11, 0x12]);
        e.call_log = Some(vec![]);
        e.call(0x0041_0140, &args![list, 0x11u8]);
        let log = e.call_log.take().unwrap();
        assert_eq!(chain_types(&e, list), vec![0x10, 0x12]);
        assert_eq!(deleted(&log), vec![extras[1].addr()]);
        assert!(!base_extra_list_has_extra(&mut e, list, 0x11));
        assert_locked_and_released(&log);
        assert_eq!(calls_to(&log, LOCK)[0][1], NAME_REMOVE_EXTRA);

        // The head.
        e.call(0x0041_0140, &args![list, 0x10u8]);
        assert_eq!(chain_types(&e, list), vec![0x12]);

        // A type the chain does not hold: nothing deleted, bit cleared anyway.
        fn_0040fee0(&mut e, list, 0x40, true);
        e.call_log = Some(vec![]);
        e.call(0x0041_0140, &args![list, 0x40u8]);
        let log = e.call_log.take().unwrap();
        assert!(deleted(&log).is_empty());
        assert_eq!(chain_types(&e, list), vec![0x12]);
        assert!(!base_extra_list_has_extra(&mut e, list, 0x40));
    }

    #[test]
    fn get_extra_data_answers_from_the_bitmap_the_cache_or_a_walk() {
        let mut e = extra_engine();
        let (list, extras) = list_with(&mut e, &[0x10, 0x11, 0x12]);
        let cache = cache_of(&mut e);

        // Not in the bitmap: no lock, no walk.
        e.call_log = Some(vec![]);
        assert_eq!(e.call(0x0041_0220, &args![list, 0x40u8]).u32(), 0);
        assert!(e.call_log.take().unwrap().iter().all(|(a, _)| *a != LOCK));

        // In the bitmap, not cached: walks under the lock and caches it.
        e.call_log = Some(vec![]);
        assert_eq!(
            e.call(0x0041_0220, &args![list, 0x11u8]).u32(),
            extras[1].addr()
        );
        let log = e.call_log.take().unwrap();
        assert_locked_and_released(&log);
        assert_eq!(calls_to(&log, LOCK)[0][1], NAME_GET_EXTRA_DATA);
        assert_eq!(e.mem.u32(cache_entry(cache, 0x11)), extras[1].addr());
        assert_eq!(
            e.get(cache, BaseExtraListThreadCache::pLastExtraList),
            list.cast()
        );

        // Cached now: answered without the lock.
        e.call_log = Some(vec![]);
        assert_eq!(
            e.call(0x0041_0220, &args![list, 0x11u8]).u32(),
            extras[1].addr()
        );
        assert!(e.call_log.take().unwrap().iter().all(|(a, _)| *a != LOCK));

        // In the bitmap but not in the chain (a stale bit): null.
        fn_0040fee0(&mut e, list, 0x50, true);
        assert_eq!(e.call(0x0041_0220, &args![list, 0x50u8]).u32(), 0);
    }

    #[test]
    fn get_prev_extra_data_is_the_one_before_or_null() {
        let mut e = extra_engine();
        let (list, extras) = list_with(&mut e, &[0x10, 0x11, 0x12]);
        e.call_log = Some(vec![]);
        assert_eq!(
            e.call(0x0041_02d0, &args![list, 0x12u8]).u32(),
            extras[1].addr()
        );
        let log = e.call_log.take().unwrap();
        assert_locked_and_released(&log);
        assert_eq!(calls_to(&log, LOCK)[0][1], NAME_GET_PREV_EXTRA_DATA);
        // The head has no previous; an absent type returns before the lock.
        assert_eq!(e.call(0x0041_02d0, &args![list, 0x10u8]).u32(), 0);
        e.call_log = Some(vec![]);
        assert_eq!(e.call(0x0041_02d0, &args![list, 0x60u8]).u32(), 0);
        assert_eq!(e.call_log.take().unwrap().len(), 1);
    }

    #[test]
    fn extra_data_list_constructor_and_destructors() {
        let mut e = extra_engine();
        let list: Ptr<ExtraDataList> = e.new_object();
        e.mem.write(list.addr() + 4, &[0xff; 0x19]);
        let result = e.call(0x0041_0360, &args![list]).ptr::<()>();
        assert_eq!(result.addr(), list.addr());
        assert_eq!(e.mem.u32(list.addr()), 0x0101_43e8);
        assert_eq!(e.get(list, ExtraDataList::pHead), Ptr::NULL);
        assert_eq!(e.mem.bytes(list.addr() + 8, 0x15), vec![0u8; 0x15]);

        let extra = extra_of_type(&mut e, 0x10);
        base_extra_list_add_extra(&mut e, list.cast(), extra);
        e.call_log = Some(vec![]);
        e.call(0x0041_03b0, &args![list]);
        let log = e.call_log.take().unwrap();
        assert_eq!(deleted(&log), vec![extra.addr()]);
        // The base destructor leaves its own vtable.
        assert_eq!(e.mem.u32(list.addr()), 0x0101_4300);

        let extra = extra_of_type(&mut e, 0x10);
        base_extra_list_add_extra(&mut e, list.cast(), extra);
        e.call_log = Some(vec![]);
        e.call(0x0041_0380, &args![list, 1u32]);
        let log = e.call_log.take().unwrap();
        assert_eq!(deleted(&log), vec![extra.addr()]);
        assert_eq!(calls_to(&log, OPERATOR_DELETE), vec![vec![list.addr()]]);
    }

    // --- AddExtraCopy ---------------------------------------------------

    const IS_COPYABLE: u32 = 0x0041_1f20;

    /// An engine where every setter in the copy tables is a stub, and the
    /// copyability check answers yes.
    fn copy_engine() -> Engine {
        let mut e = extra_engine();
        e.register(IS_COPYABLE, |_, _| returns(1));
        for table in [
            COPY_WORD,
            COPY_FLOAT,
            COPY_BYTE,
            COPY_READ_WORD,
            COPY_PAYLOAD_ADDRESS,
        ] {
            for &(_, setter) in table {
                stub(&mut e, setter);
            }
        }
        for &(_, _, construct, copy, setter) in COPY_NEW_OBJECT {
            e.register(construct, |_, a| returns(a[0]));
            stub(&mut e, copy);
            stub(&mut e, setter);
        }
        e
    }

    fn copy_list(e: &mut Engine) -> Ptr<ExtraDataList> {
        let list: Ptr<ExtraDataList> = e.new_object();
        list
    }

    fn run_copy(e: &mut Engine, list: Ptr<ExtraDataList>, extra: Ptr<BSExtraData>) -> Log {
        e.call_log = Some(vec![]);
        e.call(0x0041_03d0, &args![list, extra]);
        let mut log = e.call_log.take().unwrap();
        // The first entry is the call under test itself.
        log.remove(0);
        log
    }

    /// The calls of a copy except the ones every copy makes (copyability and
    /// type).
    fn copy_calls(log: &Log) -> Log {
        log.iter()
            .filter(|(callee, _)| *callee != IS_COPYABLE && *callee != GET_TYPE)
            .cloned()
            .collect()
    }

    #[test]
    fn copy_does_nothing_for_null_or_uncopyable_extra_data() {
        let mut e = copy_engine();
        let list = copy_list(&mut e);
        let log = run_copy(&mut e, list, Ptr::NULL);
        assert!(log.is_empty());

        e.register(IS_COPYABLE, |_, _| returns(0));
        let extra = extra_of_type(&mut e, 0x03);
        let log = run_copy(&mut e, list, extra);
        assert_eq!(log, vec![(IS_COPYABLE, vec![list.addr(), extra.addr()])]);
    }

    #[test]
    fn copy_word_types_pass_the_payload_word_to_their_setter() {
        let mut e = copy_engine();
        let list = copy_list(&mut e);
        for &(extra_type, setter) in COPY_WORD {
            let extra = extra_of_type(&mut e, extra_type);
            e.mem.set_u32(extra.addr() + 0x0c, 0x1234_5678);
            let log = run_copy(&mut e, list, extra);
            assert_eq!(
                copy_calls(&log),
                vec![(setter, vec![list.addr(), 0x1234_5678])],
                "type {extra_type:#x}"
            );
        }
    }

    #[test]
    fn copy_float_byte_and_short_types_pass_the_payload_by_size() {
        let mut e = copy_engine();
        let list = copy_list(&mut e);
        for &(extra_type, setter) in COPY_FLOAT {
            let extra = extra_of_type(&mut e, extra_type);
            e.mem.set_f32(extra.addr() + 0x0c, 2.5);
            let log = run_copy(&mut e, list, extra);
            assert_eq!(
                copy_calls(&log),
                vec![(setter, vec![list.addr(), 2.5f32.to_bits()])],
                "type {extra_type:#x}"
            );
        }
        for &(extra_type, setter) in COPY_BYTE {
            let extra = extra_of_type(&mut e, extra_type);
            e.mem.set_u32(extra.addr() + 0x0c, 0xaabb_ccdd);
            let log = run_copy(&mut e, list, extra);
            assert_eq!(
                copy_calls(&log),
                vec![(setter, vec![list.addr(), 0xdd])],
                "type {extra_type:#x}"
            );
        }
        // Type 0x24 is a 16-bit payload.
        stub(&mut e, 0x0041_9ad0);
        let extra = extra_of_type(&mut e, 0x24);
        e.mem.set_u32(extra.addr() + 0x0c, 0xaabb_ccdd);
        let log = run_copy(&mut e, list, extra);
        assert_eq!(
            copy_calls(&log),
            vec![(0x0041_9ad0, vec![list.addr(), 0xccdd])]
        );
        // Type 0x48 passes 1 and then the float.
        stub(&mut e, 0x0042_2750);
        let extra = extra_of_type(&mut e, 0x48);
        e.mem.set_f32(extra.addr() + 0x0c, -4.0);
        let log = run_copy(&mut e, list, extra);
        assert_eq!(
            copy_calls(&log),
            vec![(0x0042_2750, vec![list.addr(), 1, (-4.0f32).to_bits()])]
        );
    }

    #[test]
    fn copy_handle_and_address_types() {
        let mut e = copy_engine();
        let list = copy_list(&mut e);
        // The setter gets the word the payload address points at.
        for &(extra_type, setter) in COPY_READ_WORD {
            let extra = extra_of_type(&mut e, extra_type);
            e.mem.set_u32(extra.addr() + 0x0c, 0x4242);
            let log = run_copy(&mut e, list, extra);
            assert_eq!(
                copy_calls(&log),
                vec![
                    (READ_WORD, vec![extra.addr() + 0x0c]),
                    (setter, vec![list.addr(), 0x4242])
                ],
                "type {extra_type:#x}"
            );
        }
        for &(extra_type, setter) in COPY_PAYLOAD_ADDRESS {
            let extra = extra_of_type(&mut e, extra_type);
            let log = run_copy(&mut e, list, extra);
            assert_eq!(
                copy_calls(&log),
                vec![(setter, vec![list.addr(), extra.addr() + 0x0c])],
                "type {extra_type:#x}"
            );
        }
    }

    #[test]
    fn copy_new_object_types_build_copy_and_set() {
        let mut e = copy_engine();
        let list = copy_list(&mut e);
        for &(extra_type, size, construct, copy, setter) in COPY_NEW_OBJECT {
            let extra = extra_of_type(&mut e, extra_type);
            e.mem.set_u32(extra.addr() + 0x0c, 0x9999);
            let log = run_copy(&mut e, list, extra);
            let calls = copy_calls(&log);
            assert_eq!(calls.len(), 4, "type {extra_type:#x}");
            assert_eq!(calls[0], (OPERATOR_NEW, vec![size]));
            let object = calls[1].1[0];
            assert_eq!(calls[1].0, construct);
            assert_eq!(calls[2], (copy, vec![object, 0x9999]));
            assert_eq!(calls[3], (setter, vec![list.addr(), object]));
        }
    }

    #[test]
    fn copy_of_a_failed_allocation_goes_on_with_null() {
        let mut e = copy_engine();
        let list = copy_list(&mut e);
        e.register(OPERATOR_NEW, |_, _| Ret::default());
        let extra = extra_of_type(&mut e, 0x2b);
        e.mem.set_u32(extra.addr() + 0x0c, 7);
        let log = run_copy(&mut e, list, extra);
        // No construction; the copy and the setter still get the null.
        assert_eq!(
            copy_calls(&log),
            vec![
                (OPERATOR_NEW, vec![0x20]),
                (0x0043_a810, vec![0, 7]),
                (0x0041_9120, vec![list.addr(), 0]),
            ]
        );
    }

    #[test]
    fn copy_of_types_without_payload() {
        let mut e = copy_engine();
        let list = copy_list(&mut e);
        for (extra_type, callee, expected) in [
            (0x16u8, 0x0041_aa20u32, vec![1u32, 0]),
            (0x17, 0x0041_aa20, vec![1, 1]),
            (0x3e, 0x0041_ab70, vec![1]),
            (0x80, 0x0042_f200, vec![1]),
        ] {
            stub(&mut e, callee);
            let extra = extra_of_type(&mut e, extra_type);
            let log = run_copy(&mut e, list, extra);
            let mut args = vec![list.addr()];
            args.extend(expected);
            assert_eq!(copy_calls(&log), vec![(callee, args)]);
        }
        // Type 0x55 has nothing to copy.
        let extra = extra_of_type(&mut e, 0x55);
        let log = run_copy(&mut e, list, extra);
        assert!(copy_calls(&log).is_empty());
        // Type 0x5a hands over the whole extra data.
        stub(&mut e, 0x0042_e2c0);
        let extra = extra_of_type(&mut e, 0x5a);
        let log = run_copy(&mut e, list, extra);
        assert_eq!(
            copy_calls(&log),
            vec![(0x0042_e2c0, vec![list.addr(), extra.addr()])]
        );
    }

    #[test]
    fn copy_of_two_part_payloads() {
        let mut e = copy_engine();
        let list = copy_list(&mut e);
        for callee in [
            0x0041_9ed0u32,
            0x0041_9f80,
            0x0041_d280,
            0x0041_d330,
            0x0041_da40,
            0x0041_dc70,
            0x0042_eb60,
        ] {
            stub(&mut e, callee);
        }
        let extra = extra_of_type(&mut e, 0x0d);
        e.mem.set_u32(extra.addr() + 0x0c, 11);
        e.mem.set_u32(extra.addr() + 0x10, 12);
        let log = run_copy(&mut e, list, extra);
        assert_eq!(
            copy_calls(&log),
            vec![
                (0x0041_9ed0, vec![list.addr(), 11]),
                (0x0041_9f80, vec![list.addr(), 12])
            ]
        );
        for (extra_type, word_setter, byte_setter) in [
            (0x2fu8, 0x0041_d280u32, 0x0041_d330u32),
            (0x37, 0x0041_da40, 0x0041_dc70),
        ] {
            let extra = extra_of_type(&mut e, extra_type);
            e.mem.set_u32(extra.addr() + 0x0c, 21);
            e.mem.set_u32(extra.addr() + 0x10, 0x1ff);
            let log = run_copy(&mut e, list, extra);
            assert_eq!(
                copy_calls(&log),
                vec![
                    (word_setter, vec![list.addr(), 21]),
                    (byte_setter, vec![list.addr(), 0xff])
                ]
            );
        }
        let extra = extra_of_type(&mut e, 0x6e);
        e.mem.set_u32(extra.addr() + 0x0c, 31);
        e.mem.set_u32(extra.addr() + 0x10, 32);
        let log = run_copy(&mut e, list, extra);
        assert_eq!(
            copy_calls(&log),
            vec![(0x0042_eb60, vec![list.addr(), 31, 32])]
        );
    }

    #[test]
    fn copy_of_canopy_shadow_mask_fills_the_structure_the_setter_made() {
        let mut e = copy_engine();
        let list = copy_list(&mut e);
        let created = e.mem.alloc(8);
        e.register_double(0x0041_c490, move |e, a| {
            // The out parameter receives the structure.
            e.mem.set_u32(a[3], created);
            Ret::default()
        });
        let extra = extra_of_type(&mut e, 0x0a);
        e.mem.set_u32(extra.addr() + 0x0c, 0x61);
        e.mem.set_u32(extra.addr() + 0x10, 0x62);
        e.mem.set_u32(extra.addr() + 0x14, 0x63);
        e.mem.set_u32(extra.addr() + 0x18, 0x64);
        let log = run_copy(&mut e, list, extra);
        let calls = calls_to(&log, 0x0041_c490);
        assert_eq!(calls.len(), 1);
        // Handle read from +0x10's address.
        assert_eq!(&calls[0][..3], &[list.addr(), 0x61, 0x62]);
        assert_eq!(e.mem.u32(created), 0x63);
        assert_eq!(e.mem.u32(created + 4), 0x64);
    }

    #[test]
    fn copy_of_package_start_location_casts_twice() {
        let mut e = copy_engine();
        let list = copy_list(&mut e);
        // The cast double answers with the target type it was given.
        e.register(RT_DYNAMIC_CAST, |_, a| returns(a[3]));
        stub(&mut e, 0x0041_ad00);
        let extra = extra_of_type(&mut e, 0x18);
        e.mem.set_u32(extra.addr() + 0x0c, 0x7000);
        e.mem.set_f32(extra.addr() + 0x1c, 1.5);
        let log = run_copy(&mut e, list, extra);
        let casts = calls_to(&log, RT_DYNAMIC_CAST);
        assert_eq!(casts[0], vec![0x7000, 0, 0x0118_3028, 0x0118_3fd0, 0]);
        assert_eq!(casts[1], vec![0x7000, 0, 0x0118_3028, 0x0118_3fb4, 0]);
        assert_eq!(
            calls_to(&log, 0x0041_ad00),
            vec![vec![
                list.addr(),
                0x0118_3fd0,
                0x0118_3fb4,
                extra.addr() + 0x10,
                1.5f32.to_bits()
            ]]
        );
    }

    #[test]
    fn copy_of_package_data_passes_three_words_and_three_bytes() {
        let mut e = copy_engine();
        let list = copy_list(&mut e);
        stub(&mut e, 0x0041_c930);
        let extra = extra_of_type(&mut e, 0x19);
        for (offset, word) in [(0x0c, 1), (0x10, 2), (0x14, 3)] {
            e.mem.set_u32(extra.addr() + offset, word);
        }
        e.mem.write(extra.addr() + 0x18, &[4, 5, 6]);
        let log = run_copy(&mut e, list, extra);
        assert_eq!(
            calls_to(&log, 0x0041_c930),
            vec![vec![list.addr(), 1, 2, 3, 4, 5, 6]]
        );
    }

    /// A `BSSimpleList` of `items`: nodes of (item, next), returns the head.
    fn simple_list(e: &mut Engine, items: &[u32]) -> u32 {
        let nodes: Vec<u32> = items.iter().map(|_| e.mem.alloc(8)).collect();
        for (index, node) in nodes.iter().enumerate() {
            e.mem.set_u32(*node, items[index]);
            e.mem
                .set_u32(*node + 4, nodes.get(index + 1).copied().unwrap_or(0));
        }
        nodes.first().copied().unwrap_or(0)
    }

    #[test]
    fn copy_of_list_types_walks_the_items() {
        let mut e = copy_engine();
        let list = copy_list(&mut e);
        // 0x1d: items until the first null.
        stub(&mut e, 0x0042_2480);
        let extra = extra_of_type(&mut e, 0x1d);
        let head = simple_list(&mut e, &[0x11, 0x22, 0, 0x44]);
        e.mem.set_u32(extra.addr() + 0x0c, head);
        let log = run_copy(&mut e, list, extra);
        assert_eq!(
            calls_to(&log, 0x0042_2480),
            vec![vec![list.addr(), 0x11], vec![list.addr(), 0x22]]
        );
        // 0x73: the same walk over the list at +0x0C.
        stub(&mut e, 0x0042_ef20);
        let extra = extra_of_type(&mut e, 0x73);
        let head = simple_list(&mut e, &[5, 6]);
        e.mem.set_u32(extra.addr() + 0x0c, head);
        let log = run_copy(&mut e, list, extra);
        assert_eq!(
            calls_to(&log, 0x0042_ef20),
            vec![vec![list.addr(), 5], vec![list.addr(), 6]]
        );
        // An empty list: no calls.
        e.mem.set_u32(extra.addr() + 0x0c, 0);
        let log = run_copy(&mut e, list, extra);
        assert!(calls_to(&log, 0x0042_ef20).is_empty());
    }

    #[test]
    fn copy_of_pair_list_skips_null_entries_and_does_not_stop() {
        let mut e = copy_engine();
        let list = copy_list(&mut e);
        stub(&mut e, 0x0041_d700);
        let pair_one = e.mem.alloc(8);
        e.mem.set_u32(pair_one, 100);
        e.mem.set_u8(pair_one + 4, 1);
        let pair_two = e.mem.alloc(8);
        e.mem.set_u32(pair_two, 200);
        e.mem.set_u8(pair_two + 4, 2);
        let extra = extra_of_type(&mut e, 0x1b);
        let head = simple_list(&mut e, &[pair_one, 0, pair_two]);
        e.mem.set_u32(extra.addr() + 0x0c, head);
        let log = run_copy(&mut e, list, extra);
        assert_eq!(
            calls_to(&log, 0x0041_d700),
            vec![vec![list.addr(), 100, 1], vec![list.addr(), 200, 2]]
        );
    }

    #[test]
    fn copy_of_type_5e_passes_the_signed_byte() {
        let mut e = copy_engine();
        let list = copy_list(&mut e);
        stub(&mut e, 0x0042_e760);
        e.register(0x0042_e800, |_, _| returns(0x5000));
        stub(&mut e, 0x0043_6f80);
        let entry = e.mem.alloc(8);
        e.mem.set_u32(entry, 300);
        e.mem.set_u8(entry + 4, 0xfe);
        let extra = extra_of_type(&mut e, 0x5e);
        let head = simple_list(&mut e, &[entry, 0]);
        e.mem.set_u32(extra.addr() + 0x0c, head);
        let log = run_copy(&mut e, list, extra);
        assert_eq!(
            calls_to(&log, 0x0043_6f80),
            vec![vec![0x5000, 300, (-2i32) as u32]]
        );
        assert_eq!(calls_to(&log, 0x0042_e760), vec![vec![list.addr()]]);
    }

    #[test]
    fn copy_of_room_data_walks_the_inline_list_then_sets_the_master_flag() {
        let mut e = copy_engine();
        let list = copy_list(&mut e);
        stub(&mut e, 0x0042_0c10);
        stub(&mut e, 0x0042_0870);
        // The room data: a list head node inline at +8, the flag at +0x10.
        let room = e.mem.alloc(0x20);
        let second = e.mem.alloc(8);
        e.mem.set_u32(second, 0x88);
        e.mem.set_u32(room + 8, 0x77);
        e.mem.set_u32(room + 12, second);
        e.mem.set_u8(room + 0x10, 1);
        let extra = extra_of_type(&mut e, 0x7b);
        e.mem.set_u32(extra.addr() + 0x0c, room);
        let log = run_copy(&mut e, list, extra);
        assert_eq!(
            calls_to(&log, 0x0042_0c10),
            vec![vec![list.addr(), 0x77], vec![list.addr(), 0x88]]
        );
        assert_eq!(calls_to(&log, 0x0042_0870), vec![vec![list.addr(), 1]]);
    }

    #[test]
    fn copy_of_lock_builds_and_copies_the_structure() {
        let mut e = copy_engine();
        let list = copy_list(&mut e);
        stub(&mut e, 0x0041_9050);
        let source = e.mem.alloc(0x14);
        for index in 0..5 {
            e.mem.set_u32(source + 4 * index, 0x10 + index);
        }
        let extra = extra_of_type(&mut e, 0x2a);
        e.mem.set_u32(extra.addr() + 0x0c, source);
        let log = run_copy(&mut e, list, extra);
        let set = calls_to(&log, 0x0041_9050);
        assert_eq!(set.len(), 1);
        let object = set[0][1];
        assert_eq!(set[0][0], list.addr());
        assert_eq!(calls_to(&log, MEMCPY), vec![vec![object, source, 0x14]]);
        assert_eq!(e.mem.bytes(object, 0x14), e.mem.bytes(source, 0x14));
    }

    #[test]
    fn copy_of_a_four_word_structure_and_a_two_word_table() {
        let mut e = copy_engine();
        let list = copy_list(&mut e);
        stub(&mut e, 0x0041_fec0);
        let source = e.mem.alloc(16);
        for index in 0..4 {
            e.mem.set_u32(source + 4 * index, 0xa0 + index);
        }
        let extra = extra_of_type(&mut e, 0x76);
        e.mem.set_u32(extra.addr() + 0x0c, source);
        let log = run_copy(&mut e, list, extra);
        let set = calls_to(&log, 0x0041_fec0);
        assert_eq!(set.len(), 1);
        for index in 0..4 {
            assert_eq!(e.mem.u32(set[0][1] + 4 * index), 0xa0 + index);
        }
        // The structure was constructed (zeroed) by the unit's own function.
        assert_eq!(calls_to(&log, OPERATOR_NEW), vec![vec![0x10]]);

        stub(&mut e, 0x0042_0a60);
        let table = e.mem.alloc(8);
        e.mem.set_u32(table, 41);
        e.mem.set_u32(table + 4, 42);
        let extra = extra_of_type(&mut e, 0x77);
        e.mem.set_u32(extra.addr() + 0x0c, table);
        let log = run_copy(&mut e, list, extra);
        assert_eq!(
            calls_to(&log, 0x0042_0a60),
            vec![vec![list.addr(), 0, 41], vec![list.addr(), 1, 42]]
        );
    }

    #[test]
    fn copy_of_type_62_and_72_and_6b() {
        let mut e = copy_engine();
        let list = copy_list(&mut e);
        // 0x62: a null source gives a null object and no copy.
        for callee in [0x0043_8f50u32, 0x0043_9120, 0x0042_1f30] {
            stub(&mut e, callee);
        }
        e.register(0x0043_8f50, |_, a| returns(a[0]));
        let extra = extra_of_type(&mut e, 0x62);
        let log = run_copy(&mut e, list, extra);
        assert_eq!(calls_to(&log, 0x0042_1f30), vec![vec![list.addr(), 0]]);
        assert!(calls_to(&log, 0x0043_9120).is_empty());
        e.mem.set_u32(extra.addr() + 0x0c, 0x333);
        let log = run_copy(&mut e, list, extra);
        let copy = calls_to(&log, 0x0043_9120);
        assert_eq!(copy.len(), 1);
        assert_eq!(copy[0][1], 0x333);
        assert_eq!(
            calls_to(&log, 0x0042_1f30),
            vec![vec![list.addr(), copy[0][0]]]
        );

        // 0x72: a 4-byte object initialised to 0x16, then given the word the
        // payload handle reads.
        for callee in [0x0053_7e90u32, 0x0042_0fd0] {
            stub(&mut e, callee);
        }
        let handle = e.mem.alloc(4);
        e.mem.set_u32(handle, 0x999);
        let extra = extra_of_type(&mut e, 0x72);
        e.mem.set_u32(extra.addr() + 0x0c, handle);
        let log = run_copy(&mut e, list, extra);
        let call = calls_to(&log, 0x0053_7e90);
        assert_eq!(call[0][1], 0x999);
        assert_eq!(e.mem.u32(call[0][0]), 0x16);
        assert_eq!(
            calls_to(&log, 0x0042_0fd0),
            vec![vec![list.addr(), call[0][0]]]
        );

        // 0x6b: cdecl conversion of the payload word, then the setter.
        e.register(0x004a_4d40, |_, a| returns(a[0] + 1));
        stub(&mut e, 0x0041_fa60);
        let extra = extra_of_type(&mut e, 0x6b);
        e.mem.set_u32(extra.addr() + 0x0c, 50);
        let log = run_copy(&mut e, list, extra);
        assert_eq!(calls_to(&log, 0x0041_fa60), vec![vec![list.addr(), 51]]);
    }

    #[test]
    fn copy_of_portal_reads_the_payload_handle() {
        let mut e = copy_engine();
        let list = copy_list(&mut e);
        stub(&mut e, 0x0042_0e00);
        let extra = extra_of_type(&mut e, 0x78);
        e.mem.set_u32(extra.addr() + 0x0c, 0x1357);
        let log = run_copy(&mut e, list, extra);
        assert_eq!(calls_to(&log, READ_WORD), vec![vec![extra.addr() + 0x0c]]);
        assert_eq!(calls_to(&log, 0x0042_0e00), vec![vec![list.addr(), 0x1357]]);
    }

    #[test]
    fn copy_of_ref_list_types_adds_the_extra_once_and_copies_into_it() {
        for (extra_type, copy) in [
            (0x65u8, 0x0043_46f0u32),
            (0x66, 0x0043_4c80),
            (0x84, 0x0043_4a70),
            (0x85, 0x0043_4ee0),
        ] {
            let mut e = copy_engine();
            let list = copy_list(&mut e);
            stub(&mut e, copy);
            let source = extra_of_type(&mut e, extra_type);
            let log = run_copy(&mut e, list, source);
            // A new one, constructed by this unit, added to the list.
            assert_eq!(calls_to(&log, OPERATOR_NEW), vec![vec![0x14]]);
            let made = chain_types(&e, list.cast());
            assert_eq!(made, vec![extra_type]);
            let created: Ptr<BSExtraData> = e.get(list, ExtraDataList::pHead).cast();
            assert!(base_extra_list_has_extra(&mut e, list.cast(), extra_type));
            assert_eq!(
                calls_to(&log, copy),
                vec![vec![created.addr(), source.addr()]]
            );

            // The second copy finds it and allocates nothing.
            let log = run_copy(&mut e, list, source);
            assert!(calls_to(&log, OPERATOR_NEW).is_empty());
            assert_eq!(
                calls_to(&log, copy),
                vec![vec![created.addr(), source.addr()]]
            );
            assert_eq!(chain_types(&e, list.cast()), vec![extra_type]);
        }
    }

    #[test]
    fn copy_of_type_4c_creates_then_overwrites_nine_words() {
        let mut e = copy_engine();
        let list = copy_list(&mut e);
        e.register(0x0043_5fa0, |e, a| {
            e.mem.set_u8(a[0] + 4, 0x4c);
            e.mem.set_u32(a[0] + 8, 0);
            returns(a[0])
        });
        let source = extra_of_type(&mut e, 0x4c);
        for index in 3..12u32 {
            e.mem.set_u32(source.addr() + 4 * index, 0x500 + index);
        }
        e.mem.set_u32(source.addr() + 0x30, 0xdead);
        let log = run_copy(&mut e, list, source);
        assert_eq!(calls_to(&log, OPERATOR_NEW), vec![vec![0x30]]);
        let created: Ptr<BSExtraData> = e.get(list, ExtraDataList::pHead).cast();
        assert_eq!(calls_to(&log, 0x0043_5fa0), vec![vec![created.addr(), 0]]);
        for index in 3..12u32 {
            assert_eq!(e.mem.u32(created.addr() + 4 * index), 0x500 + index);
        }
    }

    #[test]
    fn copy_of_type_53_and_57_use_their_own_copy_calls() {
        let mut e = copy_engine();
        let list = copy_list(&mut e);
        for (extra_type, size, construct, copy) in [
            (0x53u8, 0x20u32, 0x0043_38b0u32, 0x0043_3b70u32),
            (0x57, 0x14, 0x0043_3ca0, 0x0043_42b0),
        ] {
            e.register_double(construct, move |e, a| {
                e.mem.set_u8(a[0] + 4, extra_type);
                e.mem.set_u32(a[0] + 8, 0);
                returns(a[0])
            });
            stub(&mut e, copy);
            let source = extra_of_type(&mut e, extra_type);
            let log = run_copy(&mut e, list, source);
            assert_eq!(calls_to(&log, OPERATOR_NEW), vec![vec![size]]);
            let created = calls_to(&log, construct)[0][0];
            assert_eq!(calls_to(&log, copy), vec![vec![created, source.addr()]]);
        }
        assert_eq!(chain_types(&e, list.cast()), vec![0x53, 0x57]);
    }

    #[test]
    fn copy_of_type_92_copies_through_the_virtual_slot() {
        let mut e = copy_engine();
        let list = copy_list(&mut e);
        e.put_vtable(VTABLE_EXTRA_TYPE_92, &[DESTRUCTOR, 0, 0, VIRTUAL_COPY]);
        let source = extra_of_type(&mut e, 0x92);
        let log = run_copy(&mut e, list, source);
        let created: Ptr<BSExtraData> = e.get(list, ExtraDataList::pHead).cast();
        assert_eq!(e.get(created, BSExtraData::cEtype), 0x92);
        assert_eq!(
            calls_to(&log, VIRTUAL_COPY),
            vec![vec![created.addr(), source.addr()]]
        );
        // Again: the existing one is used.
        let log = run_copy(&mut e, list, source);
        assert!(calls_to(&log, OPERATOR_NEW).is_empty());
        assert_eq!(
            calls_to(&log, VIRTUAL_COPY),
            vec![vec![created.addr(), source.addr()]]
        );
    }

    #[test]
    fn copy_of_swim_breadcrumbs_copies_fields_and_walks_the_list() {
        let mut e = copy_engine();
        let list = copy_list(&mut e);
        let target = e.mem.alloc(0x40);
        e.register_double(0x0042_f420, move |_, _| returns(target));
        stub(&mut e, 0x0047_0470);
        stub(&mut e, 0x0090_5820);
        let source = extra_of_type(&mut e, 0x8b);
        for index in 3..8u32 {
            e.mem.set_u32(source.addr() + 4 * index, 0x600 + index);
        }
        // The inline list head at +0x20: an item, then a null item, then one.
        e.mem.set_u32(source.addr() + 0x20, 0x31);
        let second = e.mem.alloc(8);
        let third = e.mem.alloc(8);
        e.mem.set_u32(source.addr() + 0x24, second);
        e.mem.set_u32(second, 0);
        e.mem.set_u32(second + 4, third);
        e.mem.set_u32(third, 0x33);
        let log = run_copy(&mut e, list, source);
        for index in 3..8u32 {
            assert_eq!(e.mem.u32(target + 4 * index), 0x600 + index);
        }
        assert_eq!(calls_to(&log, 0x0047_0470), vec![vec![target + 0x20]]);
        assert_eq!(
            calls_to(&log, 0x0090_5820),
            vec![
                vec![target + 0x20, source.addr() + 0x20],
                vec![target + 0x20, third]
            ]
        );
    }

    #[test]
    fn copy_of_weapon_mod_flags_sets_each_active_slot() {
        let mut e = copy_engine();
        let list = copy_list(&mut e);
        stub(&mut e, 0x0042_e380);
        let extra = extra_of_type(&mut e, 0x8d);
        e.mem.set_u8(extra.addr() + 0x0c, 0b101);
        let log = run_copy(&mut e, list, extra);
        assert_eq!(
            calls_to(&log, 0x0042_e380),
            vec![vec![list.addr(), 1], vec![list.addr(), 4]]
        );
    }

    #[test]
    fn copy_of_an_unknown_type_logs_a_message() {
        let mut e = copy_engine();
        let list = copy_list(&mut e);
        stub(&mut e, LOG_MESSAGE);
        let extra = extra_of_type(&mut e, 0x04);
        let log = run_copy(&mut e, list, extra);
        assert_eq!(
            copy_calls(&log),
            vec![(LOG_MESSAGE, vec![NO_COPY_MESSAGE, 4])]
        );
    }

    // --- small structures and constructors --------------------------------

    #[test]
    fn lock_structure_is_zeroed() {
        let mut e = extra_engine();
        let block = Ptr::new(e.mem.alloc(0x14));
        e.mem.write(block.addr(), &[0xff; 0x14]);
        let result = e.call(0x0041_1b00, &args![block]).ptr::<()>();
        assert_eq!(result, block);
        assert_eq!(e.mem.u8(block.addr()), 0);
        assert_eq!(e.mem.u32(block.addr() + 4), 0);
        assert_eq!(e.mem.u8(block.addr() + 8), 0);
        assert_eq!(e.mem.u32(block.addr() + 0x0c), 0);
        assert_eq!(e.mem.u32(block.addr() + 0x10), 0);
        // Bytes between the fields are not touched.
        assert_eq!(e.mem.u8(block.addr() + 1), 0xff);
        assert_eq!(e.mem.u8(block.addr() + 9), 0xff);
    }

    #[test]
    fn ref_list_extra_constructors_set_type_vtable_and_an_empty_list() {
        let mut e = extra_engine();
        e.register(0x0096_a2d0, |e, a| {
            e.mem.set_u32(a[0], 0);
            e.mem.set_u32(a[0] + 4, 0);
            returns(a[0])
        });
        for (constructor, extra_type, vtable) in [
            (0x0041_1b40u32, 0x65u8, 0x0101_4428u32),
            (0x0041_1be0, 0x66, 0x0101_4434),
            (0x0041_1c80, 0x84, 0x0101_4440),
            (0x0041_1d20, 0x85, 0x0101_444c),
        ] {
            let extra: Ptr<ExtraRefList> = e.new_object();
            e.mem.write(extra.addr() + 0x0c, &[0xff; 8]);
            e.call_log = Some(vec![]);
            let result = e.call(constructor, &args![extra]).ptr::<()>();
            let log = e.call_log.take().unwrap();
            assert_eq!(result.addr(), extra.addr());
            assert_eq!(e.mem.u8(extra.addr() + 4), extra_type);
            assert_eq!(e.mem.u32(extra.addr()), vtable);
            assert_eq!(e.mem.u32(extra.addr() + 0x0c), 0);
            assert_eq!(e.mem.u32(extra.addr() + 0x10), 0);
            assert_eq!(calls_to(&log, 0x0096_a2d0), vec![vec![extra.addr() + 0x0c]]);
        }
    }

    #[test]
    fn ref_list_extra_destructors_run_theirs_and_free_on_bit_zero() {
        let mut e = extra_engine();
        for (function, destructor) in [
            (0x0041_1bb0u32, 0x0043_4560u32),
            (0x0041_1c50, 0x0043_4af0),
            (0x0041_1cf0, 0x0043_4950),
            (0x0041_1d90, 0x0043_4dc0),
        ] {
            stub(&mut e, destructor);
            let extra: Ptr<ExtraRefList> = e.new_object();
            e.call_log = Some(vec![]);
            let result = e.call(function, &args![extra, 0u32]).ptr::<()>();
            let log = e.call_log.take().unwrap();
            assert_eq!(result.addr(), extra.addr());
            assert_eq!(calls_to(&log, destructor), vec![vec![extra.addr()]]);
            assert!(calls_to(&log, OPERATOR_DELETE).is_empty());

            e.call_log = Some(vec![]);
            e.call(function, &args![extra, 3u32]);
            let log = e.call_log.take().unwrap();
            assert_eq!(calls_to(&log, OPERATOR_DELETE), vec![vec![extra.addr()]]);
        }
    }

    #[test]
    fn four_word_structure_is_zeroed_and_the_value_structure_is_0x16() {
        let mut e = extra_engine();
        let block = Ptr::new(e.mem.alloc(0x14));
        e.mem.write(block.addr(), &[0xff; 0x14]);
        e.call(0x0041_1dc0, &args![block]);
        assert_eq!(e.mem.bytes(block.addr(), 16), vec![0u8; 16]);
        assert_eq!(e.mem.u32(block.addr() + 16), 0xffff_ffff);

        let result = e.call(0x0041_1e00, &args![block]).ptr::<()>();
        assert_eq!(result, block);
        assert_eq!(e.mem.u32(block.addr()), 0x16);
    }

    #[test]
    fn weapon_mod_flag_test_masks_the_byte_at_0c() {
        let mut e = extra_engine();
        let extra = extra_of_type(&mut e, 0x8d);
        e.mem.set_u32(extra.addr() + 0x0c, 0xffff_ff02);
        assert!(e.call(0x0041_1e20, &args![extra, 2u8]).bool());
        assert!(!e.call(0x0041_1e20, &args![extra, 1u8]).bool());
        assert!(e.call(0x0041_1e20, &args![extra, 3u8]).bool());
        assert!(!e.call(0x0041_1e20, &args![extra, 0xfdu8]).bool());
    }

    #[test]
    fn type_92_constructor_and_scalar_deleting_destructor() {
        let mut e = extra_engine();
        let extra = Ptr::new(e.mem.alloc(0x14));
        e.mem.write(extra.addr() + 0x0c, &[0xff; 4]);
        let result = e.call(0x0041_1e40, &args![extra]).ptr::<()>();
        assert_eq!(result, extra);
        assert_eq!(e.mem.u8(extra.addr() + 4), 0x92);
        assert_eq!(e.mem.u32(extra.addr()), 0x0101_4458);
        assert_eq!(e.mem.u32(extra.addr() + 0x0c), 0);

        stub(&mut e, 0x0041_1ea0);
        e.call_log = Some(vec![]);
        let result = e.call(0x0041_1e70, &args![extra, 0u32]).ptr::<()>();
        let log = e.call_log.take().unwrap();
        assert_eq!(result, extra);
        assert_eq!(calls_to(&log, 0x0041_1ea0), vec![vec![extra.addr()]]);
        assert!(calls_to(&log, OPERATOR_DELETE).is_empty());

        e.call_log = Some(vec![]);
        e.call(0x0041_1e70, &args![extra, 1u32]);
        let log = e.call_log.take().unwrap();
        assert_eq!(calls_to(&log, OPERATOR_DELETE), vec![vec![extra.addr()]]);
    }
}
