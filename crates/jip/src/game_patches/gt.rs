//! Group GT's game patch (step 3.9: the fixed strings): JIP LN NVSE's fixed-string pool (G43,
//! `InitGamePatches`, `internal/patches_game.h` (JIP LN NVSE): `SafeWrite8(0xA5B630, 0xC3)`,
//! `WriteRelJump(0xA5B690, GetNiFixedString)`, `WriteRelCall(0x878203, FreeUnusedNiFixedStrings)`), a
//! fix: in with the game-patch row `NV_JIP_GAME_PATCHES` once JIP answers "installed".
//!
//! The game keeps one copy of every name string its models, nodes, controllers and settings share
//! (`NiFixedString`: a pointer to the characters, the reference count at `-8`, the length at `-4`)
//! in a table of 0x201 buckets that every lookup walks one entry at a time under a lock
//! (`00a5b690`, `00a5b300`). JIP replaces it with its own pool, a hash map of 0x800 buckets keyed by
//! JIP's case-sensitive text hash (`StrHashCS`):
//! - the game's table is never made (`00a5b630` returns at once; its word `0x11F42C0` stays 0, so
//!   the table's destruction at the exit, `00a5b660`, finds nothing to do);
//! - "make this string" (`00a5b690`, cdecl, the text) answers 0 for no text; else the pool's string
//!   of the text's hash with its count raised by one, or a new string: a block of
//!   `(length + 0x19) & !0xF` bytes, count 1, the length, the text and its terminator at `+8`;
//! - the clean-ups (`00878200`'s call of `00a5b460`, its only caller: cell clean-ups, fast travel,
//!   `TestAllCells`) free every string of the pool whose count is 0, where the game freed its
//!   table's.
//!
//! JIP's map compares hashes only (`HashedKey<const SInt8*>::Equal`), so two texts with the same
//! 32-bit hash get one string, the first one made: kept as JIP does it.
//!
//! Faster than the game's table: a hash lookup in place of a walk of a bucket of hundreds of
//! strings under a lock, at every model, node and controller name a load makes. The strings stay
//! in guest memory (they are guest words in the game's objects).
//!
//! Threads: models load on the loader threads, the main thread and the AI threads make strings
//! too; the pool is one map behind one lock (JIP's `PrimitiveCS`), made before the threads start,
//! shared by every thread's engine. A count is raised with an interlocked add, as JIP's `lock inc`.

use super::Area;
use engine_core::extend::{Owner, WrapSlot};
use engine_core::prelude::*;
use std::collections::HashMap;
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex, PoisonError};

/// G43's wraps: the table's making, "make this string", the clean-up of unused strings.
pub const MAKE_TABLE: WrapSlot = WrapSlot::new(Owner::Jip, 0x00a5_b630);
pub const MAKE_STRING: WrapSlot = WrapSlot::new(Owner::Jip, 0x00a5_b690);
pub const FREE_UNUSED: WrapSlot = WrapSlot::new(Owner::Jip, 0x00a5_b460);

/// The group's rows and installers.
pub const AREA: Area = Area {
    name: "GT",
    fix_sites: &[],
    fix_wraps: &[MAKE_TABLE.addr(), MAKE_STRING.addr(), FREE_UNUSED.addr()],
    install_fixes,
    jip_sites: &[],
    jip_wraps: &[],
    install_jip: super::nothing,
};

/// JIP's pool (`s_NiFixedStringsMap`): the text's hash to the string (the characters' address).
#[derive(Default)]
pub struct Pool(Arc<Mutex<HashMap<u32, u32>>>);

impl Pool {
    fn map(e: &mut Engine) -> Arc<Mutex<HashMap<u32, u32>>> {
        e.ext_free::<Pool>().0.clone()
    }
}

/// A string's block before its characters: the count (`-8`) and the length (`-4`).
const HEADER: u32 = 8;

/// The block JIP's `MemoryPool::Alloc` is asked for: the length plus the header, the terminator
/// and up to 16 bytes, to a multiple of 16 (`lea ecx, [eax+0x19]; and cl, 0xF0`).
pub fn block_size(length: u32) -> u32 {
    length.wrapping_add(0x19) & !0xf
}

fn install_fixes(e: &mut Engine) {
    // Made before the threads start, so every thread's engine shares the one pool.
    e.ext_free::<Pool>();
    e.wrap(MAKE_TABLE, make_table);
    e.wrap(MAKE_STRING, make_string);
    e.wrap(FREE_UNUSED, free_unused);
}

/// `00a5b630` made `ret`: the game's table is not made.
fn make_table(_: &mut Engine, _: &[u32]) -> Ret {
    Ret::default()
}

fn answer(eax: u32) -> Ret {
    Ret {
        eax,
        ..Ret::default()
    }
}

/// The count word of a string (`-8`), raised by one as `lock inc` does.
fn raise_count(e: &mut Engine, string: u32) {
    let at = string.wrapping_sub(HEADER);
    match e.mem.atomic_u32(at) {
        Some(word) => {
            word.fetch_add(1, Ordering::SeqCst);
        }
        None => {
            let n = e.mem.u32(at);
            e.mem.set_u32(at, n.wrapping_add(1));
        }
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_game.h: GetNiFixedString
// (InsertNiFixedString; containers.h UnorderedMap::InsertKey with HashedKey<const SInt8*>).
/// `00a5b690` replaced (cdecl, the text): 0 for no text; the pool's string of the text's hash,
/// its count raised; or a new string made and put in the pool, count 1.
fn make_string(e: &mut Engine, a: &[u32]) -> Ret {
    let text = a[0];
    if text == 0 {
        return answer(0);
    }
    let chars = e.mem.cstr(text);
    let hash = crate::containers::str_hash_cs(&chars);
    let pool = Pool::map(e);
    let mut map = pool.lock().unwrap_or_else(PoisonError::into_inner);
    if let Some(&string) = map.get(&hash) {
        raise_count(e, string);
        return answer(string);
    }
    let length = chars.len() as u32;
    let block = e.mem.alloc(block_size(length));
    e.mem.set_u32(block, 1);
    e.mem.set_u32(block + 4, length);
    let string = block + HEADER;
    e.mem.write(string, &chars);
    e.mem.set_u8(string + length, 0);
    map.insert(hash, string);
    answer(string)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_game.h: FreeUnusedNiFixedStrings.
/// The clean-up's call (`0x878203`, of `00a5b460`, its only caller): every string of the pool
/// whose count is 0 is taken out and its block freed; the others stay.
fn free_unused(e: &mut Engine, _: &[u32]) -> Ret {
    let pool = Pool::map(e);
    let mut map = pool.lock().unwrap_or_else(PoisonError::into_inner);
    let mut unused = Vec::new();
    map.retain(|_, string| {
        let keep = e.mem.u32(string.wrapping_sub(HEADER)) != 0;
        if !keep {
            unused.push(*string);
        }
        keep
    });
    for string in unused {
        e.mem.free(string - HEADER);
    }
    Ret::default()
}

#[cfg(test)]
#[path = "gt_tests.rs"]
mod tests;
