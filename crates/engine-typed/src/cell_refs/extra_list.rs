//! `BaseExtraList` and `ExtraDataList` (Xbox PDB), the list of extra data every reference, cell and
//! inventory entry carries, from the exe's extra data list unit (`0040f680` to `00411ea0`):
//! the list's constructors and destructors, its operations under the extra data lock (`AddExtra`,
//! both `RemoveExtra`s, `RemoveAll`, `RemoveAllDefault`, `ItemsInList`, `GetPrevExtraData`), the
//! type bitmap, the thread's cache of the last lookup, and the small extra data classes whose
//! constructors and destructors the compiler placed in the unit. Every caller reaches them by
//! address (the lookup `00410220` is the dispatch family's).
//!
//! **The list.** A vtable, the chain's head (+4) and a bitmap of the types the chain holds (21
//! bytes from +8: bit `type & 7` of byte `type >> 3`). Each extra data is a `BSExtraData`: a vtable
//! (slot 0 the scalar deleting destructor), the type byte (+4) and the next (+8). The thread's cache
//! (the TLS block) holds the list last looked up (+8), the dirty count it is valid for (+0xc) and
//! one entry a type from +0x10 (0x93 types); the global dirty count (`011c38e4`) goes up whenever a
//! list loses an extra data, so every thread drops what it cached.
//!
//! With them the three small bodies every extra data's code calls, which the compiler placed just
//! before the unit: `BSExtraData`'s constructor (`0040ec80`: the base vtable `01014210`, the type, a
//! null next), its destructor (`0040ecb0`: the base vtable) and the store of the next (`00403550`).
//!
//! **Native.** The exe's order of work and its calls. The type byte is the dispatch family's
//! getter (`004f1540`), the next the leaf family's word at +8 (`0044ddc0`), each as its typed call.
//! The extra data lock (`011c3920`) is taken and given back with the dispatch family's `0040fbf0`
//! and `0040fba0`, the bitmap and the cache are cleared with the C runtime family's `memset`
//! wrapper (`00403d30`). An extra data is deleted through its slot 0; `operator delete`
//! (`00401030`), the four reference lists' destructors and the empty `BSSimpleList`'s constructor
//! (`0096a2d0`) are called by address. Where the translation calls a function of this unit as a
//! Rust function (the exe's direct calls inside the unit), the typed one runs the same body as a
//! Rust function too.
//!
//! **JIP LN NVSE.** JIP's ExtraJIP framework makes its keyed data's type (0x27) go to the head of
//! the chain while JIP is installed (`SafeWrite8(0x40F9D7, 0)`): the site at `0040f9d7` answers in
//! place of the table, as in the translation. Nothing else of JIP's (or of the other built-in
//! extenders) replaces these functions or has a site in them; JIP calls `AddExtra`, `RemoveExtra`
//! and the lock by address, which reaches them here.
//!
//! **Which thread.** Every thread that touches a list (main, the loaders, the AI threads), under
//! the extra data lock as the exe takes it. Integers only, but for the north rotation's 0.0 (a
//! constant word): no float rule applies.

use super::FAMILY;
use crate::dispatch::layout::LightLock;
use crate::{crt, dispatch};
use engine_core::ptr::Ptr;
use engine_core::typed::leaf;
use engine_core::{args, typed_call, Engine};

/// The extra data lock (`BaseExtraList::ExtraCritSection`, Xbox PDB) and the names the operations
/// give it.
const EXTRA_LOCK: u32 = 0x011c_3920;
const NAME_REMOVE_ALL: u32 = 0x0101_4304;
const NAME_REMOVE_ALL_DEFAULT: u32 = 0x0101_4320;
const NAME_ITEMS_IN_LIST: u32 = 0x0101_4344;
const NAME_ADD_EXTRA: u32 = 0x0101_4364;
const NAME_REMOVE_EXTRA: u32 = 0x0101_4380;
const NAME_GET_PREV_EXTRA_DATA: u32 = 0x0101_43c0;
/// `BaseExtraList::iDirty` (Xbox PDB).
const DIRTY: u32 = 0x011c_38e4;
/// The list: its head (+4) and its type bitmap (+8, 21 bytes).
const LIST_HEAD: u32 = 4;
const LIST_FLAGS: u32 = 8;
const FLAGS_LEN: u32 = 0x15;
/// An extra data: its type (+4), its next (+8), its payload (+0xc).
const EXTRA_TYPE: u32 = 4;
const EXTRA_NEXT: u32 = 8;
const EXTRA_PAYLOAD: u32 = 0xc;
/// The thread's cache in the TLS block: the last list (+8), its dirty count (+0xc), the entries
/// (+0x10, 0x93 of them).
const CACHE_LIST: u32 = 8;
const CACHE_DIRTY: u32 = 0xc;
const CACHE_ENTRIES: u32 = 0x10;
const CACHED_TYPES: u32 = 0x93;
/// The vtables the constructors and destructors set.
const VTABLE_BS_EXTRA_DATA: u32 = 0x0101_4210;
const VTABLE_EXTRA_NORTH_ROTATION: u32 = 0x0101_42a0;
const VTABLE_EXTRA_DETACH_TIME: u32 = 0x0101_42ac;
const VTABLE_BASE_EXTRA_LIST: u32 = 0x0101_4300;
const VTABLE_EXTRA_DATA_LIST: u32 = 0x0101_43e8;
const VTABLE_EXTRA_REFLECTED_REFS: u32 = 0x0101_4428;
const VTABLE_EXTRA_REFLECTOR_REFS: u32 = 0x0101_4434;
const VTABLE_EXTRA_WATER_LIGHT_REFS: u32 = 0x0101_4440;
const VTABLE_EXTRA_LIT_WATER_REFS: u32 = 0x0101_444c;
const VTABLE_EXTRA_TYPE_92: u32 = 0x0101_4458;
/// `operator delete` (`cdecl(block)`) and the reference lists' destructors (`thiscall(extra)`).
const OPERATOR_DELETE: u32 = 0x0040_1030;
const REFLECTED_REFS_DESTROY: u32 = 0x0043_4560;
const REFLECTOR_REFS_DESTROY: u32 = 0x0043_4af0;
const WATER_LIGHT_REFS_DESTROY: u32 = 0x0043_4950;
const LIT_WATER_REFS_DESTROY: u32 = 0x0043_4dc0;
/// The constructor of an empty `BSSimpleList` (two null words, `thiscall(list)`).
const SIMPLE_LIST_NEW: u32 = 0x0096_a2d0;
/// JIP's site in the table of the types added at the head (`0x40F9D7`), for its keyed data's type.
const JIP_HEAD_TYPE_SITE: u32 = 0x0040_f9d7;
const JIP_KEYED_DATA_TYPE: u32 = 0x27;

// ---- What the operations share --------------------------------------------------------------

/// The extra data lock taken (`0040fbf0`) under `name`.
#[inline]
fn take_lock(e: &mut Engine, name: u32) {
    typed_call!(
        dispatch::FAMILY,
        0x0040_fbf0,
        dispatch::refs::lock(e, Ptr::<LightLock>::new(EXTRA_LOCK), name)
    );
}

/// The extra data lock given back (`0040fba0`).
#[inline]
fn give_lock(e: &mut Engine) {
    typed_call!(
        dispatch::FAMILY,
        0x0040_fba0,
        dispatch::refs::unlock(e, Ptr::<LightLock>::new(EXTRA_LOCK))
    );
}

/// An extra data's type (`004f1540`).
#[inline]
fn type_of(e: &mut Engine, extra: u32) -> u8 {
    typed_call!(
        dispatch::FAMILY,
        0x004f_1540,
        dispatch::refs::extra_kind(e, Ptr::new(extra))
    )
}

/// An extra data's next (`0044ddc0`).
#[inline]
fn next_of(e: &mut Engine, extra: u32) -> u32 {
    typed_call!(leaf::FAMILY, 0x0044_ddc0, leaf::word_at_8(e, extra))
}

/// An extra data's next set (`00403550`).
#[inline]
fn link(e: &mut Engine, extra: u32, next: u32) {
    typed_call!(FAMILY, 0x0040_3550, set_next(e, extra, next));
}

/// `memset` of `n` bytes at `at` to 0 (`00403d30`).
#[inline]
fn clear(e: &mut Engine, at: u32, n: u32) {
    typed_call!(
        crt::FAMILY,
        0x0040_3d30,
        crt::mem::memset_wrapper(e, Ptr::new(at), 0, n)
    );
}

/// The thread's cache (the TLS block) and the address of its entry for `index`.
#[inline]
fn cache_entry(cache: u32, index: u32) -> u32 {
    cache
        .wrapping_add(CACHE_ENTRIES)
        .wrapping_add(index.wrapping_mul(4))
}

/// The cache's entries cleared.
#[inline]
fn clear_entries(e: &mut Engine, cache: u32) {
    clear(e, cache_entry(cache, 0), CACHED_TYPES * 4);
}

/// The global dirty count and this thread's both one up.
#[inline]
fn bump_dirty(e: &mut Engine) {
    let dirty = e.mem.u32(DIRTY).wrapping_add(1);
    e.mem.set_u32(DIRTY, dirty);
    let cache = e.tls();
    let own = e.mem.u32(cache.wrapping_add(CACHE_DIRTY)).wrapping_add(1);
    e.mem.set_u32(cache.wrapping_add(CACHE_DIRTY), own);
}

/// An extra data deleted through its slot 0 (the scalar deleting destructor with 1); none for null.
#[inline]
fn delete_extra_data(e: &mut Engine, extra: u32) {
    if extra != 0 {
        e.vcall(extra, 0, &args![1u32]);
    }
}

/// The scalar deleting destructors' end: `operator delete` when bit 0 of `flags` is set.
#[inline]
fn free_when(e: &mut Engine, this: u32, flags: u32) -> u32 {
    if flags & 1 != 0 {
        e.call(OPERATOR_DELETE, &args![this]);
    }
    this
}

/// `BSExtraData`'s constructor (`0040ec80`) on `this` with the type.
#[inline]
fn extra_init(e: &mut Engine, this: u32, kind: u8) {
    typed_call!(FAMILY, 0x0040_ec80, extra_data_new(e, this, kind));
}

/// The bit of `kind` in the list's bitmap: its byte's address and its mask, none past the bitmap.
#[inline]
fn flag_bit(list: u32, kind: u8) -> Option<(u32, u8)> {
    let byte = u32::from(kind >> 3);
    (byte < FLAGS_LEN).then(|| (list.wrapping_add(LIST_FLAGS + byte), 1u8 << (kind & 7)))
}

// The bodies the translation runs as Rust functions of the unit, shared by the typed functions.

/// `0040f860`'s body.
fn forget_all_cached(e: &mut Engine, list: u32) {
    bump_dirty(e);
    let cache = e.tls();
    if list == e.mem.u32(cache.wrapping_add(CACHE_LIST)) {
        clear_entries(e, cache);
        e.mem.set_u32(cache.wrapping_add(CACHE_LIST), 0);
    }
}

/// `0040f7d0`'s body.
fn forget_cached(e: &mut Engine, list: u32, kind: i32) {
    bump_dirty(e);
    let cache = e.tls();
    if list == e.mem.u32(cache.wrapping_add(CACHE_LIST)) && (0..CACHED_TYPES as i32).contains(&kind)
    {
        e.mem.set_u32(cache_entry(cache, kind as u32), 0);
    }
}

/// `0040f900`'s body.
fn drop_stale(e: &mut Engine) {
    let cache = e.tls();
    let dirty = e.mem.u32(DIRTY);
    if e.mem.u32(cache.wrapping_add(CACHE_DIRTY)) != dirty {
        e.mem.set_u32(cache.wrapping_add(CACHE_LIST), 0);
        clear_entries(e, cache);
        e.mem.set_u32(cache.wrapping_add(CACHE_DIRTY), dirty);
    }
}

/// `0040fee0`'s body.
fn set_flag(e: &mut Engine, list: u32, kind: u8, set: bool) {
    if let Some((at, bit)) = flag_bit(list, kind) {
        let flags = e.mem.u8(at);
        e.mem
            .set_u8(at, if set { flags | bit } else { !bit & flags });
    }
}

/// `0040fe80`'s body.
fn has_flag(e: &Engine, list: u32, kind: u8) -> bool {
    flag_bit(list, kind).is_some_and(|(at, bit)| e.mem.u8(at) & bit != 0)
}

/// `0040f980`'s body.
fn goes_to_head(e: &mut Engine, kind: u32) -> bool {
    if e.jip_patches() & engine_core::extend::jip_patches::INSTALLED != 0
        && kind == JIP_KEYED_DATA_TYPE
    {
        if let Some(head) = e.site(JIP_HEAD_TYPE_SITE, &[kind]) {
            return head != 0;
        }
    }
    matches!(kind, 0x0c | 0x0d | 0x0e | 0x15 | 0x2b)
}

/// `004102d0`'s body.
fn previous_of_kind(e: &mut Engine, list: u32, kind: u8) -> u32 {
    if !has_flag(e, list, kind) {
        return 0;
    }
    take_lock(e, NAME_GET_PREV_EXTRA_DATA);
    let mut current = e.mem.u32(list.wrapping_add(LIST_HEAD));
    let mut previous = 0;
    while current != 0 && type_of(e, current) != kind {
        previous = current;
        current = next_of(e, current);
    }
    give_lock(e);
    previous
}

/// `0040fae0`'s body.
fn remove_everything(e: &mut Engine, list: u32, destroy: bool) {
    take_lock(e, NAME_REMOVE_ALL);
    forget_all_cached(e, list);
    if destroy {
        let mut current = e.mem.u32(list.wrapping_add(LIST_HEAD));
        while current != 0 {
            let doomed = current;
            current = next_of(e, current);
            e.mem.set_u32(list.wrapping_add(LIST_HEAD), current);
            delete_extra_data(e, doomed);
        }
    } else {
        e.mem.set_u32(list.wrapping_add(LIST_HEAD), 0);
    }
    clear(e, list.wrapping_add(LIST_FLAGS), FLAGS_LEN);
    give_lock(e);
}

/// `0040f740`'s body.
fn base_list_init(e: &mut Engine, this: u32) {
    e.mem.set_u32(this, VTABLE_BASE_EXTRA_LIST);
    e.mem.set_u32(this.wrapping_add(LIST_HEAD), 0);
    clear(e, this.wrapping_add(LIST_FLAGS), FLAGS_LEN);
}

/// `0040f7b0`'s body.
fn base_list_destroy(e: &mut Engine, this: u32) {
    e.mem.set_u32(this, VTABLE_BASE_EXTRA_LIST);
    remove_everything(e, this, true);
}

/// `004103b0`'s body.
fn data_list_destroy(e: &mut Engine, this: u32) {
    e.mem.set_u32(this, VTABLE_EXTRA_DATA_LIST);
    base_list_destroy(e, this);
}

/// The reference lists' constructor: `BSExtraData`'s with the type, the vtable, an empty
/// `BSSimpleList` at +0xc (`0096a2d0`, by address).
fn ref_list_init(e: &mut Engine, this: u32, kind: u8, vtable: u32) -> u32 {
    extra_init(e, this, kind);
    e.mem.set_u32(this, vtable);
    e.call(SIMPLE_LIST_NEW, &args![this.wrapping_add(EXTRA_PAYLOAD)]);
    this
}

// ---- The functions --------------------------------------------------------------------------

// Typed from 0040ec80
/// `BSExtraData`'s constructor (`thiscall(this, type)`): the base vtable, the type, a null next.
/// Answers `this`.
pub fn extra_data_new(e: &mut Engine, this: u32, kind: u8) -> u32 {
    e.mem.set_u32(this, VTABLE_BS_EXTRA_DATA);
    e.mem.set_u8(this.wrapping_add(EXTRA_TYPE), kind);
    e.mem.set_u32(this.wrapping_add(EXTRA_NEXT), 0);
    this
}

// Typed from 0040ecb0
/// `BSExtraData`'s destructor: the base vtable back, nothing else.
pub fn extra_data_drop(e: &mut Engine, this: u32) {
    e.mem.set_u32(this, VTABLE_BS_EXTRA_DATA);
}

// Typed from 00403550
/// An extra data's next (+8) set (`thiscall(this, next)`).
pub fn set_next(e: &mut Engine, this: u32, next: u32) {
    e.mem.set_u32(this.wrapping_add(EXTRA_NEXT), next);
}

// Typed from 0040f680
/// `ExtraNorthRotation`'s constructor (`thiscall`): a `BSExtraData` of type 0x43, its vtable, the
/// rotation (+0xc) 0.0. Answers `this`.
pub fn north_rotation_new(e: &mut Engine, this: u32) -> u32 {
    extra_init(e, this, 0x43);
    e.mem.set_u32(this, VTABLE_EXTRA_NORTH_ROTATION);
    e.mem.set_u32(this.wrapping_add(EXTRA_PAYLOAD), 0);
    this
}

// Typed from 0040f6b0
/// The type 0xb extra data's constructor (`ExtraDetachTime`'s shape, Xbox PDB): its vtable and a
/// word of 0 at +0xc. Answers `this`.
pub fn detach_time_new(e: &mut Engine, this: u32) -> u32 {
    extra_init(e, this, 0xb);
    e.mem.set_u32(this, VTABLE_EXTRA_DETACH_TIME);
    e.mem.set_u32(this.wrapping_add(EXTRA_PAYLOAD), 0);
    this
}

// Typed from 0040f700
/// `BSExtraData::Compare` (Xbox PDB, `thiscall(this, other)`): whether the two differ, for the base
/// class whether their types differ; a null `other` always differs.
pub fn extra_compare(e: &mut Engine, this: u32, other: u32) -> bool {
    if other == 0 {
        return true;
    }
    type_of(e, this) != type_of(e, other)
}

// Typed from 0040f740
/// `BaseExtraList`'s constructor: the vtable, no head, the bitmap cleared. Answers `this`.
pub fn base_list_new(e: &mut Engine, this: u32) -> u32 {
    base_list_init(e, this);
    this
}

// Typed from 0040f780
/// `BaseExtraList`'s scalar deleting destructor (`thiscall(this, flags)`). Answers `this`.
pub fn base_list_delete(e: &mut Engine, this: u32, flags: u32) -> u32 {
    base_list_destroy(e, this);
    free_when(e, this, flags)
}

// Typed from 0040f7b0
/// `BaseExtraList`'s destructor: the vtable, then every extra data removed and deleted.
pub fn base_list_drop(e: &mut Engine, this: u32) {
    base_list_destroy(e, this);
}

// Typed from 0040f7d0
/// `BaseExtraList::ClearLastExtra` (Xbox PDB, `cdecl(list, type)`): the dirty counts up and, when
/// `list` is the one this thread's cache holds, its entry for the type (0 to 0x92) forgotten.
pub fn clear_last_extra(e: &mut Engine, list: u32, kind: i32) {
    forget_cached(e, list, kind);
}

// Typed from 0040f860
/// `BaseExtraList::ClearLastExtraAll` (Xbox PDB, `cdecl(list)`): the dirty counts up and, when
/// `list` is the one this thread's cache holds, the cache emptied and the list forgotten.
pub fn clear_last_extra_all(e: &mut Engine, list: u32) {
    forget_all_cached(e, list);
}

// Typed from 0040f900
/// The thread's cache dropped when the global dirty count has moved since it was filled: no list,
/// no entries, the count taken.
pub fn drop_stale_cache(e: &mut Engine) {
    drop_stale(e);
}

// Typed from 0040f980
/// Whether `AddExtra` puts an extra data of this type at the head of the chain (`cdecl(type)`):
/// types 0xc, 0xd, 0xe, 0x15 and 0x2b (the exe's table); with JIP installed, its site answers
/// for its keyed data's type 0x27.
pub fn added_at_head(e: &mut Engine, kind: u32) -> bool {
    goes_to_head(e, kind)
}

// Typed from 0040f9e0
/// The extra data of `kind` this thread's cache holds for `list` (`cdecl(list, type)`), or null: a
/// stale cache is dropped first, a type out of 0 to 0x92 answers null.
pub fn cached_extra(e: &mut Engine, list: u32, kind: i32) -> u32 {
    let cache = e.tls();
    if list != e.mem.u32(cache.wrapping_add(CACHE_LIST)) {
        return 0;
    }
    drop_stale(e);
    if (0..CACHED_TYPES as i32).contains(&kind) {
        e.mem.u32(cache_entry(cache, kind as u32))
    } else {
        0
    }
}

// Typed from 0040fa40
/// `extra` cached for `list` under its type (`cdecl(list, extra)`; no range check, as the exe), the
/// cache switched to `list` and emptied when it held another; nothing for a null list or extra.
pub fn cache_extra(e: &mut Engine, list: u32, extra: u32) {
    if list == 0 || extra == 0 {
        return;
    }
    let cache = e.tls();
    if list != e.mem.u32(cache.wrapping_add(CACHE_LIST)) {
        clear_entries(e, cache);
        e.mem.set_u32(cache.wrapping_add(CACHE_LIST), list);
    }
    let kind = type_of(e, extra);
    e.mem.set_u32(cache_entry(cache, u32::from(kind)), extra);
}

// Typed from 0040fae0
/// `BaseExtraList::RemoveAll` (Xbox PDB, `thiscall(list, destroy)`): under the lock, the caches
/// forgotten, then every extra data unlinked from the head and deleted (`destroy`) or the chain
/// just dropped, and the bitmap cleared.
pub fn remove_all(e: &mut Engine, this: u32, destroy: bool) {
    remove_everything(e, this, destroy);
}

// Typed from 0040fcb0
/// `BaseExtraList::RemoveAllDefault` (Xbox PDB, `thiscall(list, destroy)`): under the lock, the
/// caches forgotten, then every extra data but the kept types (0x2e, 0x38, 0x4f, 0x52, 0x54, 0x58,
/// 0x65, 0x7b, 0x84) unlinked, deleted when `destroy`, and its bit cleared.
pub fn remove_all_default(e: &mut Engine, this: u32, destroy: bool) {
    take_lock(e, NAME_REMOVE_ALL_DEFAULT);
    forget_all_cached(e, this);
    let mut kept = 0;
    let mut current = e.mem.u32(this.wrapping_add(LIST_HEAD));
    while current != 0 {
        let kind = type_of(e, current);
        let remove = !matches!(
            kind,
            0x2e | 0x38 | 0x4f | 0x52 | 0x54 | 0x58 | 0x65 | 0x7b | 0x84
        );
        let next = e.mem.u32(current.wrapping_add(EXTRA_NEXT));
        if remove {
            if kept == 0 {
                e.mem.set_u32(this.wrapping_add(LIST_HEAD), next);
            } else {
                link(e, kept, next);
            }
            if destroy {
                delete_extra_data(e, current);
            }
            set_flag(e, this, kind, false);
        } else {
            kept = current;
        }
        current = next;
    }
    give_lock(e);
}

// Typed from 0040fe20
/// `BaseExtraList::ItemsInList` (Xbox PDB): the chain's length, counted under the lock.
pub fn items_in_list(e: &mut Engine, this: u32) -> i32 {
    take_lock(e, NAME_ITEMS_IN_LIST);
    let mut count = 0i32;
    let mut current = e.mem.u32(this.wrapping_add(LIST_HEAD));
    while current != 0 {
        count = count.wrapping_add(1);
        current = next_of(e, current);
    }
    give_lock(e);
    count
}

// Typed from 0040fe80
/// `BaseExtraList::HasExtra` (Xbox PDB, `thiscall(list, type)`): the type's bit in the bitmap
/// (false past it).
pub fn has_extra_type(e: &mut Engine, this: u32, kind: u8) -> bool {
    has_flag(e, this, kind)
}

// Typed from 0040fee0
/// The type's bit in the bitmap set or cleared (`thiscall(list, type, set)`); a type past the
/// bitmap is left.
pub fn set_extra_flag(e: &mut Engine, this: u32, kind: u8, set: bool) {
    set_flag(e, this, kind, set);
}

// Typed from 0040ff60
/// `BaseExtraList::AddExtra` (Xbox PDB, `thiscall(list, extra)`): under the lock, `extra` linked
/// at the head (the types [`added_at_head`] names; the old head its next) or after the chain's
/// last, and its type's bit set. Answers `extra`.
pub fn add_extra(e: &mut Engine, this: u32, extra: u32) -> u32 {
    take_lock(e, NAME_ADD_EXTRA);
    let kind = type_of(e, extra);
    let head = e.mem.u32(this.wrapping_add(LIST_HEAD));
    if goes_to_head(e, u32::from(kind)) {
        if head != 0 {
            link(e, extra, head);
        }
        e.mem.set_u32(this.wrapping_add(LIST_HEAD), extra);
    } else if head == 0 {
        e.mem.set_u32(this.wrapping_add(LIST_HEAD), extra);
    } else {
        let mut last = head;
        while next_of(e, last) != 0 {
            last = next_of(e, last);
        }
        link(e, last, extra);
    }
    let kind = type_of(e, extra);
    set_flag(e, this, kind, true);
    give_lock(e);
    extra
}

// Typed from 00410020
/// `BaseExtraList::RemoveExtra` (Xbox PDB, `thiscall(list, extra, destroy)`): nothing for a null
/// `extra`. Under the lock, `extra` unlinked after the extra data before the first of its type
/// ([`get_prev_extra_data`]; the head when there is none), the cache's entry forgotten, `extra`
/// deleted (`destroy`) or detached, its bit cleared and the dirty counts up.
pub fn remove_extra(e: &mut Engine, this: u32, extra: u32, destroy: bool) {
    if extra == 0 {
        return;
    }
    take_lock(e, NAME_REMOVE_EXTRA);
    let kind = type_of(e, extra);
    let previous = previous_of_kind(e, this, kind);
    let next = next_of(e, extra);
    if previous == 0 {
        e.mem.set_u32(this.wrapping_add(LIST_HEAD), next);
    } else {
        link(e, previous, next);
    }
    forget_cached(e, this, i32::from(kind));
    if destroy {
        delete_extra_data(e, extra);
    } else {
        link(e, extra, 0);
    }
    set_flag(e, this, kind, false);
    bump_dirty(e);
    give_lock(e);
}

// Typed from 00410140
/// `BaseExtraList::RemoveExtra` (Xbox PDB, the overload of a type, `thiscall(list, type)`): under
/// the lock, the first extra data of the type unlinked, its cache entry forgotten and deleted; the
/// type's bit cleared whether or not there was one.
pub fn remove_extra_of_type(e: &mut Engine, this: u32, kind: u8) {
    take_lock(e, NAME_REMOVE_EXTRA);
    let mut previous = 0;
    let mut current = typed_call!(leaf::FAMILY, 0x0072_6070, leaf::word_at_4(e, this));
    while current != 0 && type_of(e, current) != kind {
        previous = current;
        current = next_of(e, current);
    }
    if current != 0 {
        let next = next_of(e, current);
        if previous == 0 {
            e.mem.set_u32(this.wrapping_add(LIST_HEAD), next);
        } else {
            link(e, previous, next);
        }
        forget_cached(e, this, i32::from(kind));
        delete_extra_data(e, current);
    }
    set_flag(e, this, kind, false);
    give_lock(e);
}

// Typed from 004102d0
/// `BaseExtraList::GetPrevExtraData` (Xbox PDB, `thiscall(list, type)`): the extra data right
/// before the first of the type, under the lock; null when that one is the head, when the bitmap
/// says there is none, or when the walk runs off the chain.
pub fn get_prev_extra_data(e: &mut Engine, this: u32, kind: u8) -> u32 {
    previous_of_kind(e, this, kind)
}

// Typed from 00410360
/// `ExtraDataList::ExtraDataList` (Xbox PDB): `BaseExtraList`'s constructor, then its own vtable.
/// Answers `this`.
pub fn data_list_new(e: &mut Engine, this: u32) -> u32 {
    base_list_init(e, this);
    e.mem.set_u32(this, VTABLE_EXTRA_DATA_LIST);
    this
}

// Typed from 00410380
/// `ExtraDataList`'s scalar deleting destructor (`thiscall(this, flags)`). Answers `this`.
pub fn data_list_delete(e: &mut Engine, this: u32, flags: u32) -> u32 {
    data_list_destroy(e, this);
    free_when(e, this, flags)
}

// Typed from 004103b0
/// `ExtraDataList`'s destructor: its vtable, then `BaseExtraList`'s.
pub fn data_list_drop(e: &mut Engine, this: u32) {
    data_list_destroy(e, this);
}

// Typed from 00411b00
/// The 0x14 bytes behind `ExtraLock` set up: the bytes at +0 and +8 and the words at +4, +0x10
/// and +0xc zero, in the exe's order. Answers `this`.
pub fn lock_data_new(e: &mut Engine, this: u32) -> u32 {
    e.mem.set_u8(this, 0);
    e.mem.set_u32(this.wrapping_add(4), 0);
    e.mem.set_u8(this.wrapping_add(8), 0);
    e.mem.set_u32(this.wrapping_add(0x10), 0);
    e.mem.set_u32(this.wrapping_add(0xc), 0);
    this
}

// Typed from 00411b40
/// `ExtraReflectedRefs`'s constructor (type 0x65). Answers `this`.
pub fn reflected_refs_new(e: &mut Engine, this: u32) -> u32 {
    ref_list_init(e, this, 0x65, VTABLE_EXTRA_REFLECTED_REFS)
}

// Typed from 00411bb0
/// `ExtraReflectedRefs`'s scalar deleting destructor (its destructor `00434560` by address).
pub fn reflected_refs_delete(e: &mut Engine, this: u32, flags: u32) -> u32 {
    e.call(REFLECTED_REFS_DESTROY, &args![this]);
    free_when(e, this, flags)
}

// Typed from 00411be0
/// `ExtraReflectorRefs`'s constructor (type 0x66). Answers `this`.
pub fn reflector_refs_new(e: &mut Engine, this: u32) -> u32 {
    ref_list_init(e, this, 0x66, VTABLE_EXTRA_REFLECTOR_REFS)
}

// Typed from 00411c50
/// `ExtraReflectorRefs`'s scalar deleting destructor (its destructor `00434af0` by address).
pub fn reflector_refs_delete(e: &mut Engine, this: u32, flags: u32) -> u32 {
    e.call(REFLECTOR_REFS_DESTROY, &args![this]);
    free_when(e, this, flags)
}

// Typed from 00411c80
/// `ExtraWaterLightRefs`'s constructor (type 0x84). Answers `this`.
pub fn water_light_refs_new(e: &mut Engine, this: u32) -> u32 {
    ref_list_init(e, this, 0x84, VTABLE_EXTRA_WATER_LIGHT_REFS)
}

// Typed from 00411cf0
/// `ExtraWaterLightRefs`'s scalar deleting destructor (its destructor `00434950` by address).
pub fn water_light_refs_delete(e: &mut Engine, this: u32, flags: u32) -> u32 {
    e.call(WATER_LIGHT_REFS_DESTROY, &args![this]);
    free_when(e, this, flags)
}

// Typed from 00411d20
/// `ExtraLitWaterRefs`'s constructor (type 0x85). Answers `this`.
pub fn lit_water_refs_new(e: &mut Engine, this: u32) -> u32 {
    ref_list_init(e, this, 0x85, VTABLE_EXTRA_LIT_WATER_REFS)
}

// Typed from 00411d90
/// `ExtraLitWaterRefs`'s scalar deleting destructor (its destructor `00434dc0` by address).
pub fn lit_water_refs_delete(e: &mut Engine, this: u32, flags: u32) -> u32 {
    e.call(LIT_WATER_REFS_DESTROY, &args![this]);
    free_when(e, this, flags)
}

// Typed from 00411dc0
/// Four words of 0 (the payload `AddExtraCopy` copies for type 0x76). Answers `this`.
pub fn four_words_new(e: &mut Engine, this: u32) -> u32 {
    for i in 0..4u32 {
        e.mem.set_u32(this.wrapping_add(i * 4), 0);
    }
    this
}

// Typed from 00411e00
/// The word 0x16 (the structure `AddExtraCopy` builds for type 0x72). Answers `this`.
pub fn word_16_new(e: &mut Engine, this: u32) -> u32 {
    e.mem.set_u32(this, 0x16);
    this
}

// Typed from 00411e20
/// Whether any bit of `mask` is set in the byte at +0xc of the extra data (`ExtraWeaponModFlags`'
/// `cWeaponModsActive`, Xbox PDB).
pub fn weapon_mod_flags_any(e: &mut Engine, this: u32, mask: u8) -> bool {
    e.mem.u8(this.wrapping_add(EXTRA_PAYLOAD)) & mask != 0
}

// Typed from 00411e40
/// The type 0x92 extra data's constructor: its vtable and a word of 0 at +0xc. Answers `this`.
pub fn type_92_new(e: &mut Engine, this: u32) -> u32 {
    extra_init(e, this, 0x92);
    e.mem.set_u32(this, VTABLE_EXTRA_TYPE_92);
    e.mem.set_u32(this.wrapping_add(EXTRA_PAYLOAD), 0);
    this
}

// Typed from 00411e70
/// The type 0x92 extra data's scalar deleting destructor (its destructor `00411ea0` by address, as
/// the exe and the translation call it).
pub fn type_92_delete(e: &mut Engine, this: u32, flags: u32) -> u32 {
    typed_call!(FAMILY, 0x0041_1ea0, type_92_drop(e, this));
    free_when(e, this, flags)
}

// Typed from 00411ea0
/// The type 0x92 extra data's destructor: its vtable, then `BSExtraData`'s (`0040ecb0`: the base
/// vtable).
pub fn type_92_drop(e: &mut Engine, this: u32) {
    e.mem.set_u32(this, VTABLE_EXTRA_TYPE_92);
    typed_call!(FAMILY, 0x0040_ecb0, extra_data_drop(e, this));
}
