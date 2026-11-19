//! The extra data lookup (`BaseExtraList::GetExtraData`, `00410220`) and the getters over it that the
//! process lists' clean-up of a reference that goes away (`0096f600`, `00976680`) asks for every actor
//! in the process lists: its followers, head-track target, package data and package target. The exe
//! (a build without inlining) reaches each through two to four calls: the getter, the lookup, the
//! type bitmap's test, the thread's cache. Here each is a Rust function, a getter calls the lookup as a
//! Rust call, and the bitmap and the cache are read in place.
//!
//! The lookup, as the exe does it: an extra data list holds a chain of extra data and a bitmap of the
//! types the chain holds (21 bytes from +8). A type whose bit is clear answers null at once. Otherwise
//! the thread's cache (the TLS block: the list last looked up at +8, the dirty count it is valid for at
//! +0xc, one entry a type from +0x10, 0x93 types) answers when it holds this list (dropped first when
//! the global dirty count `011c38e4` has moved); else the chain is walked under the extra data lock
//! (`011c3920`), and the extra data found is put in the cache (which switches to this list, emptied,
//! when it held another).

use super::layout::LightLock;
use super::{refs, FAMILY};
use crate::crt;
use engine_core::ptr::Ptr;
use engine_core::typed::leaf;
use engine_core::{typed_call, Engine};

/// `BaseExtraList::iDirty` (Xbox PDB): goes up whenever a list loses an extra data.
const DIRTY: u32 = 0x011c_38e4;
/// `BaseExtraList::ExtraCritSection` (Xbox PDB): the lock every list operation takes.
const EXTRA_LOCK: u32 = 0x011c_3920;
/// The name the lookup gives the lock (`"BaseExtraList::GetExtraData"`).
const LOCK_NAME: u32 = 0x0101_43a0;
/// The bytes of the type bitmap; a type past them has no bit and is never found.
const FLAGS_LEN: u32 = 0x15;
/// The thread cache's last list, its dirty count and its first entry, in the TLS block.
const CACHE_LIST: u32 = 0x08;
const CACHE_DIRTY: u32 = 0x0c;
const CACHE_ENTRIES: u32 = 0x10;
/// The cache's entries: the types below this one are cached.
const CACHED_TYPES: u32 = 0x93;

/// The extra data types the getters ask for: followers (`0x1d`), the head-track target (`0x46`), the
/// package data (`0x70`) and the package (`0x19`).
const FOLLOWERS: u8 = 0x1d;
const HEAD_TRACK_TARGET: u8 = 0x46;
const PACKAGE_DATA: u8 = 0x70;
const PACKAGE: u8 = 0x19;

/// `BaseExtraList::HasExtra` (`0040fe80`): the type's bit in the bitmap.
fn has_extra(e: &mut Engine, list: u32, extra_type: u8) -> bool {
    let byte = u32::from(extra_type >> 3);
    byte < FLAGS_LEN && e.mem.u8(list.wrapping_add(8 + byte)) & (1 << (extra_type & 7)) != 0
}

/// Empties the cache's entries (`memset` through the game's wrapper `00403d30`).
fn clear_entries(e: &mut Engine, cache: u32) {
    typed_call!(
        crt::FAMILY,
        0x0040_3d30,
        crt::mem::memset_wrapper(
            e,
            Ptr::new(cache.wrapping_add(CACHE_ENTRIES)),
            0,
            CACHED_TYPES * 4
        )
    );
}

/// `0040f9e0` with `0040f900`: the extra data of `extra_type` the thread's cache holds for `list`, or
/// 0. When the cache holds this list and the dirty count has moved, the cache is dropped first (its
/// list, its entries, then the count taken), and the answer is then 0.
fn cached(e: &mut Engine, list: u32, extra_type: u8) -> u32 {
    let cache = e.tls();
    if e.mem.u32(cache.wrapping_add(CACHE_LIST)) != list {
        return 0;
    }
    let dirty = e.mem.u32(DIRTY);
    if e.mem.u32(cache.wrapping_add(CACHE_DIRTY)) != dirty {
        e.mem.set_u32(cache.wrapping_add(CACHE_LIST), 0);
        clear_entries(e, cache);
        e.mem.set_u32(cache.wrapping_add(CACHE_DIRTY), dirty);
    }
    if u32::from(extra_type) < CACHED_TYPES {
        e.mem
            .u32(cache.wrapping_add(CACHE_ENTRIES + 4 * u32::from(extra_type)))
    } else {
        0
    }
}

/// `0040fa40`: caches `extra` for `list` under its own type byte (no range check, as the exe), the
/// cache switched to `list` and emptied when it held another. Nothing for a null list or extra data.
fn remember(e: &mut Engine, list: u32, extra: u32) {
    if list == 0 || extra == 0 {
        return;
    }
    let cache = e.tls();
    if e.mem.u32(cache.wrapping_add(CACHE_LIST)) != list {
        clear_entries(e, cache);
        e.mem.set_u32(cache.wrapping_add(CACHE_LIST), list);
    }
    let kind = typed_call!(FAMILY, 0x004f_1540, refs::extra_kind(e, Ptr::new(extra)));
    e.mem.set_u32(
        cache.wrapping_add(CACHE_ENTRIES + 4 * u32::from(kind)),
        extra,
    );
}

/// The lookup.s body.
fn lookup(e: &mut Engine, list: u32, extra_type: u8) -> u32 {
    if !has_extra(e, list, extra_type) {
        return 0;
    }
    let found = cached(e, list, extra_type);
    if found != 0 {
        return found;
    }
    let lock = Ptr::<LightLock>::new(EXTRA_LOCK);
    typed_call!(FAMILY, 0x0040_fbf0, refs::lock(e, lock, LOCK_NAME));
    let mut found = 0;
    let mut current = e.mem.u32(list.wrapping_add(4));
    while current != 0 {
        if typed_call!(FAMILY, 0x004f_1540, refs::extra_kind(e, Ptr::new(current))) == extra_type {
            remember(e, list, current);
            found = current;
            break;
        }
        current = typed_call!(leaf::FAMILY, 0x0044_ddc0, leaf::word_at_8(e, current));
    }
    typed_call!(FAMILY, 0x0040_fba0, refs::unlock(e, lock));
    found
}

/// The lookup as the getters make it: a call to `00410220`.
fn find(e: &mut Engine, list: u32, extra_type: u8) -> u32 {
    typed_call!(
        FAMILY,
        0x0041_0220,
        get_extra_data(e, Ptr::new(list), extra_type)
    )
    .addr()
}

/// The word at `offset` of the first extra data of `extra_type`, or 0.
fn word_of(e: &mut Engine, list: u32, extra_type: u8, offset: u32) -> u32 {
    match find(e, list, extra_type) {
        0 => 0,
        extra => e.mem.u32(extra.wrapping_add(offset)),
    }
}

// Typed from 00410220
/// `BaseExtraList::GetExtraData` (Xbox PDB, `thiscall(list, type)`): the first extra data of the type
/// in the list's chain, or null.
pub fn get_extra_data(e: &mut Engine, this: Ptr, extra_type: u8) -> Ptr {
    Ptr::new(lookup(e, this.addr(), extra_type))
}

// Typed from 00422700
/// The list's followers extra data (type `0x1d`), or null.
pub fn followers(e: &mut Engine, this: Ptr) -> Ptr {
    Ptr::new(find(e, this.addr(), FOLLOWERS))
}

// Typed from 00419ea0
/// `ExtraDataList::GetHeadTrackTargetExtra` (Xbox PDB): the word at +0xc of the type `0x46` extra
/// data, or 0.
pub fn head_track_target(e: &mut Engine, this: Ptr) -> u32 {
    word_of(e, this.addr(), HEAD_TRACK_TARGET, 0xc)
}

// Typed from 004212b0
/// `ExtraDataList::GetActorPackageData` (Xbox PDB): the word at +0xc of the type `0x70` extra data, or
/// 0.
pub fn actor_package_data(e: &mut Engine, this: Ptr) -> u32 {
    word_of(e, this.addr(), PACKAGE_DATA, 0xc)
}

// Typed from 0041cb70
/// `ExtraDataList::GetPackageExtraTarget` (Xbox PDB): the target (+0x14) of the type `0x19` extra
/// data, or 0.
pub fn package_target(e: &mut Engine, this: Ptr) -> u32 {
    word_of(e, this.addr(), PACKAGE, 0x14)
}
