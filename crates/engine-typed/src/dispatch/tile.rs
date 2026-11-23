//! The menus' tile values and their reaction map (Stewie's Engine Optimizations, item 7: "opening
//! and closing menus"). A menu that opens reads its file and builds its tiles: every trait a tile
//! gets or sets goes through `Tile::GetValue` / `GetOrCreateValue` (a binary search of the tile's
//! sorted trait array with the compare `00a01160`), every `<copy>`-style action records a reaction
//! (`Tile::AddReaction`, the reaction map `011f3358`), and every name goes through the menu string
//! map's hash (`00a0d1b0`, one `tolower` call a character). In the translation each of those steps
//! is a call by address (the search's compares, the hash's characters, the map's slots); here they
//! are Rust calls, with the same answers, calls into the rest of the game and memory.
//!
//! Their self time in a profiled start-up of 2026-11-10 (frames 0 to 122, the main menu's load;
//! `measure/stewie-eo`, private): the string hash 31.5 ms, `bsearch` 23.5, the map's `SetAt` 23.5,
//! the sorted find 6.3, `AddReaction` 6.3, the compare 4.8.
//!
//! Typed as the exe does it, not as Tweaks does: Tweaks' `AddReaction` skips a reaction already
//! anywhere in the list (the exe checks only the last node) and its compare is unsigned (the exe's
//! is signed); both would change what the game does in a case the exe's code allows.

use super::refs;
use crate::crt;
use engine_core::ptr::Ptr;
use engine_core::{args, typed_call, Engine};

/// The tiles' critical section.
const TILE_CRITICAL_SECTION: u32 = 0x011f_3330;
/// The memory manager singleton's allocate and free.
const MEMORY_MANAGER: u32 = 0x011f_6238;
const MEMORY_ALLOCATE: u32 = 0x00aa_3e40;
const MEMORY_DEALLOCATE: u32 = 0x00aa_4060;
/// `atexit`, for the search keys' destructors.
const ATEXIT: u32 = 0x00ec_658f;
/// `BSSimpleArray<T *>::Insert(index, &item)`.
const ARRAY_INSERT: u32 = 0x00a0_cd50;

/// The trait compare (`00a01160`), the sorted find and insert's usual compare.
const COMPARE_VALUES: u32 = 0x00a0_1160;
/// The search keys of `GetValue` and `GetOrCreateValue`: function-local statics, each with its
/// guard word and the destructor `atexit` is given.
const GET_VALUE_KEY: u32 = 0x011f_3374;
const GET_VALUE_KEY_GUARD: u32 = 0x011f_3388;
const GET_VALUE_KEY_DESTROY: u32 = 0x00fd_a290;
const GET_OR_CREATE_KEY: u32 = 0x011f_338c;
const GET_OR_CREATE_KEY_GUARD: u32 = 0x011f_33a0;
const GET_OR_CREATE_KEY_DESTROY: u32 = 0x00fd_a2a0;
/// An unset trait id (`eNone`).
const TRAIT_NONE: u32 = 0x8000_0000;

/// The reaction map (`NiTPointerMap<Tile::Value *, Tile::Reaction *>`) and its remove.
const REACTION_MAP: u32 = 0x011f_3358;
const MAP_REMOVE: u32 = 0x00a1_e240;
/// A pointer map's vtable slots: hash, key compare, node fill, new node.
const MAP_SLOT_HASH: u32 = 0x4;
const MAP_SLOT_KEYS_EQUAL: u32 = 0x8;
const MAP_SLOT_INIT_NODE: u32 = 0xc;
const MAP_SLOT_NEW_NODE: u32 = 0x14;
/// The integer maps' hash (`key % buckets`) and key compare (`==`) the reaction map's vtable holds.
const INT_MAP_HASH: u32 = 0x00a0_d260;
const INT_MAP_KEYS_EQUAL: u32 = 0x00ae_8dc0;

/// A tile's trait array (`BSSimpleArray<Tile::Value *, 8>` at +0x10): its buffer and size.
const TILE_TRAITS: u32 = 0x10;
const ARRAY_BUFFER: u32 = 0x04;
const ARRAY_SIZE: u32 = 0x08;
/// A trait value's fields: id, owning tile, float, string, actions.
const VALUE_ID: u32 = 0x00;
const VALUE_PARENT: u32 = 0x04;
const VALUE_FLOAT: u32 = 0x08;
const VALUE_STRING: u32 = 0x0c;
const VALUE_ACTIONS: u32 = 0x10;
/// An action's referenced trait (vtable slot +4) and its next action (+8).
const ACTION_SLOT_REFERENCED: u32 = 0x4;
const ACTION_NEXT: u32 = 0x08;

fn enter(e: &mut Engine) {
    engine_core::sync_direct::enter_critical_section(e, TILE_CRITICAL_SECTION);
}

fn leave(e: &mut Engine) {
    engine_core::sync_direct::leave_critical_section(e, TILE_CRITICAL_SECTION);
}

fn allocate(e: &mut Engine, size: u32) -> u32 {
    e.call(MEMORY_ALLOCATE, &args![MEMORY_MANAGER, size]).u32()
}

fn deallocate(e: &mut Engine, block: u32) {
    e.call(MEMORY_DEALLOCATE, &args![MEMORY_MANAGER, block]);
}

/// The trait id at `value` (signed, as the compare reads it).
fn id_of(e: &Engine, value: u32) -> i32 {
    e.mem.i32(value.wrapping_add(VALUE_ID))
}

/// -1, 0 or 1 by the signed trait ids of the values the two slots point to.
fn order_of_ids(e: &Engine, first: u32, second: u32) -> i32 {
    let a = id_of(e, e.mem.u32(first));
    let b = id_of(e, e.mem.u32(second));
    match a.cmp(&b) {
        std::cmp::Ordering::Less => -1,
        std::cmp::Ordering::Greater => 1,
        std::cmp::Ordering::Equal => 0,
    }
}

// Typed from 00a01160
/// The compare `GetValue`, `GetOrCreateValue` and the sorted array functions use: both arguments
/// point to a pointer to a trait value; -1, 0 or 1 by the signed trait ids.
pub fn compare_values(e: &mut Engine, first: Ptr, second: Ptr) -> i32 {
    order_of_ids(e, first.addr(), second.addr())
}

/// The compare at `compare` called on (`key`, `element`): the trait compare a Rust call while the
/// family is direct (by address otherwise, so an override or wrap there runs), any other by address.
fn compare_at(e: &mut Engine, compare: u32, key: u32, element: u32) -> i32 {
    if compare == COMPARE_VALUES {
        typed_call!(
            super::FAMILY,
            0x00a0_1160,
            compare_values(e, Ptr::new(key), Ptr::new(element))
        )
    } else {
        e.call(compare, &args![key, element]).i32()
    }
}

// Typed from 00a0cb90
/// `BSSimpleArray<Tile::Value *, 8>::SortedFind(&key, compare)` (Xbox PDB): the C runtime's
/// `bsearch` of the key in the buffer (4-byte elements, the typed `bsearch` below while the family
/// is direct); the element's index or -1.
pub fn sorted_find(e: &mut Engine, this: Ptr, key: u32, compare: u32) -> i32 {
    let array = this.addr();
    let buffer = e.mem.u32(array.wrapping_add(ARRAY_BUFFER));
    let size = e.mem.u32(array.wrapping_add(ARRAY_SIZE));
    let found = typed_call!(
        super::FAMILY,
        0x00ec_716d,
        bsearch(e, key, buffer, size, 4, compare)
    );
    if found == 0 {
        -1
    } else {
        // The exe reads the buffer again after the search (`00a0cbcd`): a compare can move it.
        let buffer = e.mem.u32(array.wrapping_add(ARRAY_BUFFER));
        i32::from_ne_bytes((found.wrapping_sub(buffer) >> 2).to_ne_bytes())
    }
}

/// The C runtime's report of an invalid parameter (`00ec7c30`, five zero words) and `EINVAL`.
const INVALID_PARAMETER: u32 = 0x00ec_7c30;
const EINVAL: u32 = 0x16;

// Typed from 00ec716d
/// `_bsearch` (the C runtime's, cdecl: key, base, element count, element size, compare): the
/// matching element's address or 0. A null base with elements, a size of 0 or no compare sets
/// `errno` to `EINVAL`, reports an invalid parameter and answers 0. Otherwise, while the low element
/// is at or below the high one (the last element's address taken first, wrapping): with no half
/// left a count of 0 answers 0 and a count of 1 compares the low element; else the middle is `half`
/// elements up for an odd count and `half - 1` for an even one, 0 answers it, a negative answer
/// keeps the part below it (`half` or `half - 1` elements) and a positive one the part above
/// (`half`). The compares are `compare_at`'s: the trait compare a Rust call while the family is
/// direct.
pub fn bsearch(e: &mut Engine, key: u32, base: u32, num: u32, width: u32, compare: u32) -> u32 {
    let mut lo = base;
    let mut num = num;
    let mut hi = num.wrapping_sub(1).wrapping_mul(width).wrapping_add(lo);
    if (lo == 0 && num != 0) || width == 0 || compare == 0 {
        let slot = typed_call!(crt::FAMILY, 0x00ec_85e3, crt::math::errno(e));
        e.mem.set_u32(slot.addr(), EINVAL);
        e.call(INVALID_PARAMETER, &args![0u32, 0u32, 0u32, 0u32, 0u32]);
        return 0;
    }
    while lo <= hi {
        let half = num >> 1;
        if half == 0 {
            if num == 0 {
                return 0;
            }
            return if compare_at(e, compare, key, lo) != 0 {
                0
            } else {
                lo
            };
        }
        let odd = num & 1 != 0;
        let step = if odd { half } else { half - 1 };
        let mid = step.wrapping_mul(width).wrapping_add(lo);
        let result = compare_at(e, compare, key, mid);
        if result == 0 {
            return mid;
        }
        if result < 0 {
            hi = mid.wrapping_sub(width);
            num = step;
        } else {
            lo = mid.wrapping_add(width);
            num = half;
        }
    }
    0
}

// Typed from 00a0caf0
/// `BSSimpleArray<Tile::Value *, 8>::SortedInsert(&value, compare)`: a binary search for the place
/// of `*value` (`compare(&value, &element)` answering -1, 0 or 1; a match ends the search at its
/// index), then the insert there (`00a0cd50`). Any other answer would make the game loop forever;
/// here it stops with a panic, as the translation does.
pub fn sorted_insert(e: &mut Engine, this: Ptr, value: u32, compare: u32) {
    let array = this.addr();
    let mut low: i32 = 0;
    let size = e.mem.u32(array.wrapping_add(ARRAY_SIZE));
    let mut high = i32::from_ne_bytes(size.to_ne_bytes()).wrapping_sub(1);
    while low <= high {
        let middle = ((high - low) >> 1) + low;
        let buffer = e.mem.u32(array.wrapping_add(ARRAY_BUFFER));
        let element = buffer.wrapping_add_signed(middle.wrapping_mul(4));
        match compare_at(e, compare, value, element) {
            -1 => high = middle - 1,
            0 => {
                low = middle;
                high = -1;
            }
            1 => low = middle + 1,
            other => panic!("comparison answered {other}: the game loops forever"),
        }
    }
    e.call(
        ARRAY_INSERT,
        &args![this, u32::from_ne_bytes(low.to_ne_bytes()), value],
    );
}

/// The search keys are function-local statics: on the first call the key is set to the unset state
/// (`eNone`, null tile, 0.0, no string, no actions) and its destructor given to `atexit`.
fn init_search_key(e: &mut Engine, key: u32, guard: u32, destroy: u32) {
    let guard_word = e.mem.u32(guard);
    if guard_word & 1 == 0 {
        e.mem.set_u32(guard, guard_word | 1);
        e.mem.set_u32(key + VALUE_ID, TRAIT_NONE);
        e.mem.set_u32(key + VALUE_PARENT, 0);
        e.mem.set_f32(key + VALUE_FLOAT, 0.0);
        e.mem.set_u32(key + VALUE_STRING, 0);
        e.mem.set_u32(key + VALUE_ACTIONS, 0);
        e.call(ATEXIT, &args![destroy]);
    }
}

/// The sorted find of the search key `key` in the tile's traits (the find takes the address of a
/// variable holding the key's address): the index or -1.
fn find_value_index(e: &mut Engine, tile: u32, key: u32) -> i32 {
    e.with_stack(4, |e, slot| {
        e.mem.set_u32(slot.addr(), key);
        typed_call!(
            super::FAMILY,
            0x00a0_cb90,
            sorted_find(e, Ptr::new(tile + TILE_TRAITS), slot.addr(), COMPARE_VALUES)
        )
    })
}

/// The trait at `index` of the tile's array.
fn value_at(e: &Engine, tile: u32, index: i32) -> u32 {
    let buffer = e.mem.u32(tile + TILE_TRAITS + ARRAY_BUFFER);
    e.mem.u32(buffer.wrapping_add_signed(index.wrapping_mul(4)))
}

// Typed from 00a00e90
/// `Tile::GetValueQ` (Xbox PDB): the trait value with id `trait_id` or null; a linear search of the
/// sorted trait array under the critical section.
pub fn get_value_q(e: &mut Engine, this: Ptr, trait_id: i32) -> Ptr {
    let tile = this.addr();
    let mut result = 0;
    let mut index = 0u32;
    enter(e);
    let size_at = tile + TILE_TRAITS + ARRAY_SIZE;
    let id_at = |e: &Engine, index: u32| {
        let buffer = e.mem.u32(tile + TILE_TRAITS + ARRAY_BUFFER);
        id_of(e, e.mem.u32(buffer.wrapping_add(index.wrapping_mul(4))))
    };
    while index < e.mem.u32(size_at) && id_at(e, index) < trait_id {
        index += 1;
    }
    if index < e.mem.u32(size_at) && id_at(e, index) == trait_id {
        let buffer = e.mem.u32(tile + TILE_TRAITS + ARRAY_BUFFER);
        result = e.mem.u32(buffer.wrapping_add(index.wrapping_mul(4)));
    }
    leave(e);
    Ptr::new(result)
}

// Typed from 00a00f30
/// `Tile::GetValue` (Xbox PDB): the trait value with id `trait_id`, or null, by the sorted find of
/// the shared search key, under the critical section.
pub fn get_value(e: &mut Engine, this: Ptr, trait_id: i32) -> Ptr {
    let tile = this.addr();
    let mut result = 0;
    enter(e);
    init_search_key(e, GET_VALUE_KEY, GET_VALUE_KEY_GUARD, GET_VALUE_KEY_DESTROY);
    e.mem.set_i32(GET_VALUE_KEY + VALUE_ID, trait_id);
    let index = find_value_index(e, tile, GET_VALUE_KEY);
    if index != -1 {
        result = value_at(e, tile, index);
    }
    leave(e);
    Ptr::new(result)
}

// Typed from 00a01000
/// `Tile::GetOrCreateValue` (Xbox PDB): the trait value with id `trait_id`; if the tile has none a
/// new one (0x14 bytes, unset, then given the id and this tile) is allocated and put in its sorted
/// place.
pub fn get_or_create_value(e: &mut Engine, this: Ptr, trait_id: i32) -> Ptr {
    let tile = this.addr();
    enter(e);
    init_search_key(
        e,
        GET_OR_CREATE_KEY,
        GET_OR_CREATE_KEY_GUARD,
        GET_OR_CREATE_KEY_DESTROY,
    );
    e.mem.set_i32(GET_OR_CREATE_KEY + VALUE_ID, trait_id);
    let index = find_value_index(e, tile, GET_OR_CREATE_KEY);
    let result = if index != -1 {
        value_at(e, tile, index)
    } else {
        let value = allocate(e, 0x14);
        e.mem.set_u32(value + VALUE_ID, TRAIT_NONE);
        e.mem.set_u32(value + VALUE_PARENT, 0);
        e.mem.set_f32(value + VALUE_FLOAT, 0.0);
        e.mem.set_u32(value + VALUE_STRING, 0);
        e.mem.set_u32(value + VALUE_ACTIONS, 0);
        e.mem.set_i32(value + VALUE_ID, trait_id);
        e.mem.set_u32(value + VALUE_PARENT, tile);
        // The insert takes the address of a variable holding the value.
        e.with_stack(4, |e, slot| {
            e.mem.set_u32(slot.addr(), value);
            typed_call!(
                super::FAMILY,
                0x00a0_caf0,
                sorted_insert(e, Ptr::new(tile + TILE_TRAITS), slot.addr(), COMPARE_VALUES)
            );
            e.mem.u32(slot.addr())
        })
    };
    leave(e);
    Ptr::new(result)
}

// Typed from 00a0d1b0
/// `Tile::MenuStringMap::KeyToHashIndex` (Xbox PDB): `hash = hash * 33 + tolower(c)` over the
/// characters (each a signed byte, the lowered one taken back as a signed byte), modulo the bucket
/// count at +4.
pub fn string_hash(e: &mut Engine, this: Ptr, key: Ptr) -> u32 {
    let mut hash = 0u32;
    let mut at = key.addr();
    loop {
        let character = e.mem.u8(at);
        if character == 0 {
            break;
        }
        let widened = u32::from_ne_bytes(i32::from(i8::from_ne_bytes([character])).to_ne_bytes());
        let lowered = typed_call!(crt::FAMILY, 0x00ec_67aa, crt::string::tolower(e, widened));
        let lowered = i32::from(i8::from_ne_bytes([crt::low_byte(lowered)]));
        hash = hash
            .wrapping_mul(33)
            .wrapping_add(u32::from_ne_bytes(lowered.to_ne_bytes()));
        at = at.wrapping_add(1);
    }
    hash.checked_rem(e.mem.u32(this.addr() + 4))
        .expect("hash map without buckets (division by zero in the game)")
}

// Typed from 00a0d180
/// `Tile::MenuStringMap::IsKeysEqual` (Xbox PDB): the two keys are equal ignoring case (`_stricmp`
/// answers 0).
pub fn string_keys_equal(e: &mut Engine, _this: Ptr, a: Ptr, b: Ptr) -> bool {
    typed_call!(
        crt::FAMILY,
        0x00ec_68e4,
        crt::string::stricmp(e, a.addr(), b.addr())
    ) == 0
}

// Typed from 00a0d260
/// An integer map's hash: the key modulo the bucket count at +4 (a map without buckets divides by
/// zero in the game: here it panics, as the translation does).
pub fn int_hash(e: &mut Engine, this: Ptr, key: u32) -> u32 {
    let buckets = e.mem.u32(this.addr() + 4);
    key.checked_rem(buckets)
        .expect("hash map without buckets (division by zero in the game)")
}

// Typed from 00ae8dc0
/// An integer map's key compare: the two keys are equal.
pub fn int_keys_equal(_e: &mut Engine, _this: u32, key: u32, other: u32) -> bool {
    key == other
}

// Typed from 00a0c6c0
/// A pointer map's `SetAt(key, value)` (the reaction map's and the integer maps'): the key's bucket
/// (slot +4), then its chain searched with the key compare (slot +8); a match gets the new value,
/// else a node is made (slot +0x14), filled (slot +0xc), linked at the head of its bucket, and the
/// count at +0xc grows. The integer maps' own hash and compare are Rust calls, other slots asked
/// as the exe asks.
pub fn map_set_at(e: &mut Engine, this: Ptr, key: u32, value: u32) {
    let map = this.addr();
    let index = if crate::slot_target(e, map, MAP_SLOT_HASH) == INT_MAP_HASH {
        typed_call!(super::FAMILY, 0x00a0_d260, int_hash(e, this, key))
    } else {
        e.vcall(map, MAP_SLOT_HASH, &args![key]).u32()
    };
    let direct_equal = crate::slot_target(e, map, MAP_SLOT_KEYS_EQUAL) == INT_MAP_KEYS_EQUAL;
    let mut node = e
        .mem
        .u32(e.mem.u32(map + 8).wrapping_add(index.wrapping_mul(4)));
    while node != 0 {
        let node_key = e.mem.u32(node + 4);
        let equal = if direct_equal {
            typed_call!(
                super::FAMILY,
                0x00ae_8dc0,
                int_keys_equal(e, map, key, node_key)
            )
        } else {
            e.vcall(map, MAP_SLOT_KEYS_EQUAL, &args![key, node_key])
                .bool()
        };
        if equal {
            e.mem.set_u32(node + 8, value);
            return;
        }
        node = e.mem.u32(node);
    }
    let node = e.vcall(map, MAP_SLOT_NEW_NODE, &[]).u32();
    e.vcall(map, MAP_SLOT_INIT_NODE, &args![node, key, value]);
    let slot = e.mem.u32(map + 8).wrapping_add(index.wrapping_mul(4));
    let old_head = e.mem.u32(slot);
    e.mem.set_u32(node, old_head);
    e.mem.set_u32(slot, node);
    let count = e.mem.u32(map + 0xc);
    e.mem.set_u32(map + 0xc, count.wrapping_add(1));
}

/// The reaction map's find (`00a1e380`, the word map's lookup): the list of `value` into `out`.
fn reactions_of(e: &mut Engine, value: u32, out: u32) -> bool {
    typed_call!(
        super::FAMILY,
        0x00a1_e380,
        refs::map_lookup(e, Ptr::new(REACTION_MAP), value, Ptr::new(out))
    ) != 0
}

fn set_reactions(e: &mut Engine, value: u32, list: u32) {
    typed_call!(
        super::FAMILY,
        0x00a0_c6c0,
        map_set_at(e, Ptr::new(REACTION_MAP), value, list)
    );
}

// Typed from 00a0a130
/// `Tile::AddReaction` (Xbox PDB, `cdecl`): records in the reaction map that `owner` depends on
/// `value`. A reaction node (owner, next) is made; if `value` has no list yet the node becomes its
/// list, otherwise it goes behind the list's last node, unless that last node already names `owner`
/// (then the node is freed).
pub fn add_reaction(e: &mut Engine, value: Ptr, owner: Ptr) {
    let node = allocate(e, 8);
    e.mem.set_u32(node, 0);
    e.mem.set_u32(node + 4, 0);
    e.mem.set_u32(node, owner.addr());
    e.with_stack(4, |e, list| {
        e.mem.set_u32(list.addr(), 0);
        if !reactions_of(e, value.addr(), list.addr()) {
            set_reactions(e, value.addr(), node);
            return;
        }
        let mut last = e.mem.u32(list.addr());
        let mut names_owner = e.mem.u32(last) == owner.addr();
        while e.mem.u32(last + 4) != 0 {
            last = e.mem.u32(last + 4);
            names_owner = e.mem.u32(last) == owner.addr();
        }
        if names_owner {
            deallocate(e, node);
        } else {
            e.mem.set_u32(last + 4, node);
        }
    });
}

/// `Tile::Value::SetFloat(float, bool)`, `SetString(string, bool)` and `CalculateValue(bool)` (Xbox
/// PDB): overridable (`crates/engine-core/data/overridable.tsv`), so always called by address.
const VALUE_SET_FLOAT: u32 = 0x00a0_a270;
const VALUE_SET_STRING: u32 = 0x00a0_a300;
const VALUE_CALCULATE: u32 = 0x00a0_9410;
/// The traits the position sums read: `x` (`0xfa1`), `y` (`0xfa2`) and `locus` (`0xfa8`).
const TRAIT_X: i32 = 0xfa1;
const TRAIT_Y: i32 = 0xfa2;
const TRAIT_LOCUS: i32 = 0xfa8;
/// A tile's parent (`Tile::pParent`, +0x28).
const TILE_PARENT: u32 = 0x28;

// Typed from 00a044f0
/// `Tile::Lock` (Xbox PDB): `EnterCriticalSection` on the tiles' critical section.
pub fn lock(e: &mut Engine) {
    enter(e);
}

// Typed from 00a04500
/// `Tile::Unlock` (Xbox PDB): `LeaveCriticalSection` on the tiles' critical section.
pub fn unlock(e: &mut Engine) {
    leave(e);
}

/// The trait value with id `trait_id` of the tile (`GetValue`, a Rust call while the family is
/// direct).
fn value_of(e: &mut Engine, this: Ptr, trait_id: i32) -> u32 {
    typed_call!(super::FAMILY, 0x00a0_0f30, get_value(e, this, trait_id)).addr()
}

/// The trait value with id `trait_id` of the tile, made if it has none (`GetOrCreateValue`).
fn value_made(e: &mut Engine, this: Ptr, trait_id: i32) -> u32 {
    typed_call!(
        super::FAMILY,
        0x00a0_1000,
        get_or_create_value(e, this, trait_id)
    )
    .addr()
}

// Typed from 00a011b0
/// `Tile::GetFloat` (no PDB name; `thiscall(trait)`, `RET 4`, the answer in `ST0`): the float of the
/// tile's trait `trait_id`, 0.0 when it has none. The float is copied as it is.
pub fn get_float(e: &mut Engine, this: Ptr, trait_id: i32) -> f32 {
    match value_of(e, this, trait_id) {
        0 => 0.0,
        value => e.mem.f32(value + VALUE_FLOAT),
    }
}

// Typed from 00a011f0
/// `Tile::GetString` (Xbox PDB): the string of the tile's trait `trait_id`, null when it has none.
pub fn get_string(e: &mut Engine, this: Ptr, trait_id: i32) -> Ptr {
    match value_of(e, this, trait_id) {
        0 => Ptr::NULL,
        value => Ptr::new(e.mem.u32(value + VALUE_STRING)),
    }
}

// Typed from 00a01230
/// `Tile::IsTrue` (Xbox PDB): the tile has trait `trait_id` and its float is not 0.0 (`FCOMP` with
/// the double 0.0 and `TEST AH,44`: a NaN counts as true).
pub fn is_true(e: &mut Engine, this: Ptr, trait_id: i32) -> bool {
    match value_of(e, this, trait_id) {
        0 => false,
        value => e.mem.f32(value + VALUE_FLOAT) != 0.0,
    }
}

// Typed from 00a012d0
/// `Tile::SetFloat` (no PDB name; `thiscall(trait, value, flag)`, `RET 0xc`): the tile's trait
/// `trait_id` (made when it has none) gets `value` through `Tile::Value::SetFloat(value, flag)`
/// (by address), under `Tile::Lock`.
pub fn set_float(e: &mut Engine, this: Ptr, trait_id: i32, value: f32, flag: u8) {
    let trait_value = value_made(e, this, trait_id);
    if trait_value != 0 {
        typed_call!(super::FAMILY, 0x00a0_44f0, lock(e));
        e.call(VALUE_SET_FLOAT, &args![trait_value, value, u32::from(flag)]);
        typed_call!(super::FAMILY, 0x00a0_4500, unlock(e));
    }
}

// Typed from 00a01350
/// `Tile::SetString` (no PDB name; `thiscall(trait, string, flag)`, `RET 0xc`): as
/// [`set_float`], through `Tile::Value::SetString(string, flag)` (by address).
pub fn set_string(e: &mut Engine, this: Ptr, trait_id: i32, string: Ptr, flag: u8) {
    let trait_value = value_made(e, this, trait_id);
    if trait_value != 0 {
        typed_call!(super::FAMILY, 0x00a0_44f0, lock(e));
        e.call(
            VALUE_SET_STRING,
            &args![trait_value, string, u32::from(flag)],
        );
        typed_call!(super::FAMILY, 0x00a0_4500, unlock(e));
    }
}

/// An integer as the float `FILD; FSTP float` makes of it: rounded to nearest, ties to even, as
/// `as f32` rounds.
#[allow(clippy::cast_precision_loss)]
fn int_to_float(value: i32) -> f32 {
    value as f32
}

// Typed from 00700320
/// `Tile::SetInt` (no PDB name; `thiscall(trait, value)`, `RET 8`): the tile's trait `trait_id` set
/// to the integer `value` as a float ([`set_float`] with flag 1).
pub fn set_int(e: &mut Engine, this: Ptr, trait_id: i32, value: i32) {
    let value = int_to_float(value);
    typed_call!(
        super::FAMILY,
        0x00a0_12d0,
        set_float(e, this, trait_id, value, 1)
    );
}

// Typed from 00a01290
/// `Tile::Poke` (Xbox PDB; `thiscall(trait, value)`, `RET 8`): trait `trait_id` set to `value` and
/// at once back to 0.0, so the reactions to the change run (both with flag 1).
pub fn poke(e: &mut Engine, this: Ptr, trait_id: i32, value: f32) {
    typed_call!(
        super::FAMILY,
        0x00a0_12d0,
        set_float(e, this, trait_id, value, 1)
    );
    typed_call!(
        super::FAMILY,
        0x00a0_12d0,
        set_float(e, this, trait_id, 0.0, 1)
    );
}

/// Trait `trait_id` of the tile and of each ancestor whose `locus` is not 0.0 (a NaN counts), each
/// added in turn to a float running total (`FADD float; FSTP float`: one rounding, as `f32` adds).
// `term + sum`, not `sum += term`: the operand order picks which NaN's payload is kept.
#[allow(clippy::assign_op_pattern)]
fn sum_over_ancestors(e: &mut Engine, this: Ptr, trait_id: i32) -> f32 {
    let mut sum = typed_call!(super::FAMILY, 0x00a0_11b0, get_float(e, this, trait_id));
    let mut ancestor = e.mem.u32(this.addr() + TILE_PARENT);
    while ancestor != 0 {
        let locus = typed_call!(
            super::FAMILY,
            0x00a0_11b0,
            get_float(e, Ptr::new(ancestor), TRAIT_LOCUS)
        );
        if locus != 0.0 {
            let term = typed_call!(
                super::FAMILY,
                0x00a0_11b0,
                get_float(e, Ptr::new(ancestor), trait_id)
            );
            // The translation's operand order (`term + sum`): with both NaN the first one's
            // payload is kept, so the answer is the same bits.
            sum = term + sum;
        }
        ancestor = e.mem.u32(ancestor + TILE_PARENT);
    }
    sum
}

// Typed from 00a013d0
/// `Tile::GetAbsoluteX` (no PDB name; `thiscall`, the answer in `ST0`): the tile's `x` plus the `x`
/// of each ancestor whose `locus` is not 0.0.
pub fn absolute_x(e: &mut Engine, this: Ptr) -> f32 {
    sum_over_ancestors(e, this, TRAIT_X)
}

// Typed from 00a01440
/// `Tile::GetAbsoluteY`: as [`absolute_x`], with the `y` trait.
pub fn absolute_y(e: &mut Engine, this: Ptr) -> f32 {
    sum_over_ancestors(e, this, TRAIT_Y)
}

// Typed from 00a0a220
/// `Tile::ValueChangeEvent` (Xbox PDB, `cdecl`): each value with a reaction on `value` (its list in
/// the reaction map, in order) is recalculated with `CalculateValue(false)` (by address).
pub fn value_change_event(e: &mut Engine, value: Ptr) {
    e.with_stack(4, |e, list| {
        let at = list.addr();
        e.mem.set_u32(at, 0);
        if !reactions_of(e, value.addr(), at) {
            return;
        }
        loop {
            let reaction = e.mem.u32(at);
            if reaction == 0 {
                break;
            }
            let dependent = e.mem.u32(reaction);
            e.call(VALUE_CALCULATE, &args![dependent, 0u32]);
            let next = e.mem.u32(e.mem.u32(at) + 4);
            e.mem.set_u32(at, next);
        }
    });
}

// Typed from 00a09200
/// `Tile::Value::ClearActions` (Xbox PDB): deletes the trait's actions one by one. An action that
/// reads another trait (slot +4 answers it; asked each time, as the exe asks, and read in place when
/// the slot holds one of the action classes' own: [`action_ref_value`]) is first taken out of that
/// trait's reaction list: this trait's reactions leave the list, and the map's entry is erased when
/// none are left or set when the head changed.
pub fn clear_actions(e: &mut Engine, this: Ptr) {
    let me = this.addr();
    let list_head = me + VALUE_ACTIONS;
    loop {
        let action = e.mem.u32(list_head);
        if action == 0 {
            break;
        }
        if action_ref_value(e, action) != 0 {
            e.with_stack(4, |e, reactions| {
                let at = reactions.addr();
                e.mem.set_u32(at, 0);
                let referenced = action_ref_value(e, action);
                if !reactions_of(e, referenced, at) {
                    return;
                }
                let first_before = e.mem.u32(at);
                let mut link = at;
                loop {
                    let reaction = e.mem.u32(link);
                    if reaction == 0 {
                        break;
                    }
                    if e.mem.u32(reaction) == me {
                        let next = e.mem.u32(reaction + 4);
                        e.mem.set_u32(link, next);
                        deallocate(e, reaction);
                    } else {
                        link = reaction + 4;
                    }
                }
                let first_after = e.mem.u32(at);
                if first_after == 0 {
                    let referenced = action_ref_value(e, action);
                    e.call(MAP_REMOVE, &args![REACTION_MAP, referenced]);
                } else if first_after != first_before {
                    let referenced = action_ref_value(e, action);
                    set_reactions(e, referenced, first_after);
                }
            });
        }
        let next = e.mem.u32(action + ACTION_NEXT);
        e.mem.set_u32(list_head, next);
        deallocate(e, action);
    }
}

// Typed from 00a09330
/// `Tile::Value::~Value` (Xbox PDB): the actions cleared ([`clear_actions`]); then, when the value
/// has a reaction list in the reaction map, each reaction's owner has every action of its that reads
/// this value blanked (its operand word +0xC zeroed), the reaction nodes are freed and the map's
/// entry erased (by address). The string is freed, the float and the owning tile zeroed (the value
/// itself is not freed). An action's `QRefValue` is read in place when its vtable slot holds one of
/// the action classes' own ([`action_ref_value`]). Every value of every tile goes through here when a
/// menu closes (`Tile::Release`, by address).
pub fn value_destructor(e: &mut Engine, this: Ptr) {
    let me = this.addr();
    typed_call!(super::FAMILY, 0x00a0_9200, clear_actions(e, this));
    e.with_stack(4, |e, reactions| {
        let at = reactions.addr();
        e.mem.set_u32(at, 0);
        if !reactions_of(e, me, at) {
            return;
        }
        let mut reaction = e.mem.u32(at);
        while reaction != 0 {
            let owner = e.mem.u32(reaction);
            let mut action = e.mem.u32(owner + VALUE_ACTIONS);
            while action != 0 {
                if action_ref_value(e, action) == me {
                    e.mem.set_u32(action + ACTION_OPERAND, 0);
                }
                action = e.mem.u32(action + ACTION_NEXT);
            }
            let node = reaction;
            reaction = e.mem.u32(reaction + 4);
            deallocate(e, node);
        }
        e.call(MAP_REMOVE, &args![REACTION_MAP, me]);
    });
    let text = e.mem.u32(me + VALUE_STRING);
    if text != 0 {
        deallocate(e, text);
    }
    e.mem.set_f32(me + VALUE_FLOAT, 0.0);
    e.mem.set_u32(me + VALUE_PARENT, 0);
}

/// The C runtime's `_stricmp` (`00ec68e4`) and `strcmp` (`00ec6da0`) through the crt family, the
/// menus root (`00706cd0`), the interface manager's globals tile (`00706cf0`: its word at +0xA0, `globalsTile` in JIP LN's layout), the path's split (`00a018d0`: the word before the
/// parentheses as a text-table code, the name inside them copied out), `Tile::GetMenu`
/// (`00a03c90`) and `Tile::GetMenuTile` (`00a03d40`).
const MENUS_ROOT: u32 = 0x0070_6cd0;
const GLOBALS_TILE: u32 = 0x0070_6cf0;
const SPLIT_PATH: u32 = 0x00a0_18d0;
const GET_MENU: u32 = 0x00a0_3c90;
const GET_MENU_TILE: u32 = 0x00a0_3d40;
/// The split's buffer for the name (`char[0x104]` in the exe's frame).
const NAME_BUFFER: u32 = 0x104;
/// The words of an action link's path: `parent`, `me`, `sibling`, `child`, the menus root,
/// the globals tile, the menu's and the grandparent.
const LINK_PARENT: i32 = 0x1389;
const LINK_SELF: i32 = 0x138a;
const LINK_SIBLING: i32 = 0x138c;
const LINK_CHILD: i32 = 0x138d;
const LINK_MENUS_ROOT: i32 = 0x138e;
const LINK_GLOBALS: i32 = 0x138f;
const LINK_MENU: i32 = 0x1390;
const LINK_GRANDPARENT: i32 = 0x1391;
/// A tile's children list (head at +4, count at +0xC; a node is next, previous, tile) and its name
/// (`+0x20`, a C string or 0).
const TILE_CHILDREN: u32 = 0x04;
const TILE_CHILD_COUNT: u32 = 0x0c;
const TILE_NAME: u32 = 0x20;

/// `_stricmp(a, b) == 0`.
fn same_name_any_case(e: &mut Engine, a: u32, b: u32) -> bool {
    typed_call!(crt::FAMILY, 0x00ec_68e4, crt::string::stricmp(e, a, b)) == 0
}

// Typed from 00a08f20
/// The first tile named `name` (`_stricmp`) in the tree under and including `tile`, depth first
/// (the tile, then each child's tree in the children list's order); 0 when there is none. Every
/// action link a menu file names by a tile's name (`<copy src="MCM_Options" .../>`) searches the
/// menu's whole tree this way while the menu is built: in the translation a call by address per
/// tile visited, here a loop of reads and the compare.
pub fn find_named_in_tree(e: &mut Engine, tile: Ptr, name: Ptr) -> Ptr {
    let t = tile.addr();
    if t == 0 {
        return Ptr::NULL;
    }
    let own = e.mem.u32(t + TILE_NAME);
    if own != 0 && same_name_any_case(e, own, name.addr()) {
        return tile;
    }
    let mut node = e.mem.u32(t + TILE_CHILDREN);
    while node != 0 {
        let child = e.mem.u32(node + 8);
        node = e.mem.u32(node);
        // The exe's own recursion (a plain call, as in the translation).
        let found = find_named_in_tree(e, Ptr::new(child), name);
        if !found.is_null() {
            return found;
        }
    }
    Ptr::NULL
}

// Typed from 00a03da0
/// `Tile::GetChildByName` (Xbox PDB): under `Tile::Lock`, the first tile named `name` (`strcmp`,
/// with case) among the tile's descendants, depth first (each child compared before its own
/// children are searched, each level under its own lock); 0 when there is none or `name` is 0.
pub fn child_by_name(e: &mut Engine, this: Ptr, name: Ptr) -> Ptr {
    typed_call!(super::FAMILY, 0x00a0_44f0, lock(e));
    let mut result = Ptr::NULL;
    let mut node = e.mem.u32(this.addr() + TILE_CHILDREN);
    while node != 0 && !name.is_null() {
        let child = e.mem.u32(node + 8);
        node = e.mem.u32(node);
        let child_name = e.mem.u32(child + TILE_NAME);
        if child_name != 0
            && typed_call!(
                crt::FAMILY,
                0x00ec_6da0,
                crt::string::strcmp(e, child_name, name.addr())
            ) == 0
        {
            result = Ptr::new(child);
            break;
        }
        // The exe's own recursion (a plain call, as in the translation).
        let found = child_by_name(e, Ptr::new(child), name);
        if !found.is_null() {
            result = found;
            break;
        }
    }
    typed_call!(super::FAMILY, 0x00a0_4500, unlock(e));
    result
}

// Typed from 00a08b20
/// The tile an action link's path names (no PDB name; `cdecl(tile, path)`): the path split into a
/// word and a name in parentheses (`00a018d0`, by address); by the word, the parent, the tile
/// itself, the next sibling (the first child after the last) or the first sibling named so (any
/// case), the first child or the descendant named so ([`child_by_name`]), the menus root,
/// the globals tile, the menu's tile or the grandparent; any other word (a tile's own name) the
/// depth-first search of the menu's tree from the tile's menu tile, or from the menus root for no
/// tile ([`find_named_in_tree`]). The tile is read without a null check where the exe reads it so.
pub fn find_by_path(e: &mut Engine, tile: Ptr, path: Ptr) -> Ptr {
    let t = tile.addr();
    e.with_stack(NAME_BUFFER, |e, buffer| {
        let b = buffer.addr();
        e.mem.set_u8(b, 0);
        let code = e.call(SPLIT_PATH, &args![path, b]).i32();
        let named = e.mem.i8(b) != 0;
        match code {
            LINK_PARENT => Ptr::new(e.mem.u32(t + TILE_PARENT)),
            LINK_SELF => tile,
            LINK_SIBLING => {
                let parent = e.mem.u32(t + TILE_PARENT);
                let siblings = e.mem.u32(parent + TILE_CHILD_COUNT);
                if !named && siblings != 0 {
                    let mut node = e.mem.u32(parent + TILE_CHILDREN);
                    while node != 0 {
                        let element = e.mem.u32(node + 8);
                        node = e.mem.u32(node);
                        if element == t {
                            break;
                        }
                    }
                    if node != 0 {
                        Ptr::new(e.mem.u32(node + 8))
                    } else {
                        Ptr::new(e.mem.u32(e.mem.u32(parent + TILE_CHILDREN) + 8))
                    }
                } else if named && siblings != 0 {
                    let mut node = e.mem.u32(parent + TILE_CHILDREN);
                    while node != 0 {
                        let sibling = e.mem.u32(node + 8);
                        node = e.mem.u32(node);
                        let sibling_name = e.mem.u32(sibling + TILE_NAME);
                        if sibling_name != 0 && same_name_any_case(e, sibling_name, b) {
                            return Ptr::new(sibling);
                        }
                    }
                    Ptr::NULL
                } else {
                    Ptr::NULL
                }
            }
            LINK_CHILD => {
                if !named && e.mem.u32(t + TILE_CHILD_COUNT) != 0 {
                    return Ptr::new(e.mem.u32(e.mem.u32(t + TILE_CHILDREN) + 8));
                }
                if t != 0 && named && e.mem.u32(t + TILE_CHILD_COUNT) != 0 {
                    typed_call!(
                        super::FAMILY,
                        0x00a0_3da0,
                        child_by_name(e, tile, Ptr::new(b))
                    )
                } else {
                    Ptr::NULL
                }
            }
            LINK_MENUS_ROOT => e.call(MENUS_ROOT, &args![]).ptr(),
            LINK_GLOBALS => e.call(GLOBALS_TILE, &args![]).ptr(),
            LINK_MENU => {
                let menu = e.call(GET_MENU, &args![tile]).u32();
                Ptr::new(e.mem.u32(menu.wrapping_add(4)))
            }
            LINK_GRANDPARENT => {
                let parent = e.mem.u32(t + TILE_PARENT);
                if parent == 0 {
                    Ptr::NULL
                } else {
                    Ptr::new(e.mem.u32(parent + TILE_PARENT))
                }
            }
            _ => {
                let start = if t == 0 {
                    e.call(MENUS_ROOT, &args![]).ptr()
                } else {
                    e.call(GET_MENU_TILE, &args![tile]).ptr()
                };
                typed_call!(
                    super::FAMILY,
                    0x00a0_8f20,
                    find_named_in_tree(e, start, path)
                )
            }
        }
    })
}

/// What `Tile::Value::CalculateValue` reads and calls besides the tile values: the tile's flags
/// (`+0x30`) and their bits, the thread block's current-line word (`+0x2b4`), an action's type
/// (`+4`) and its operand slots (vtable `+0` `QFloat`, `+4` `QRefValue`), the stack of floats the
/// parentheses use (its vtable, constructor, push and destructor), the user-trait map (`011f3308`),
/// `Tile::GetUnderscoreValue`, UI Organizer's operator site, the C runtime's `floor`, `ceil`,
/// `fabs`, `_ftol`, `strcpy_s` and `sprintf_s`, and `Tile::FinalPostParse`.
const TILE_FLAGS: u32 = 0x30;
const FLAG_TILE_LOADING: u32 = 0x10000;
const FLAG_MENU_DELETING: u32 = 0x4000;
const FLAG_RELEASED: u32 = 0x2000;
const TRAIT_CLASS: i32 = 0xfa4;
const TRAIT_STRING: i32 = 0xfc4;
const TLS_CURRENT_LINE: u32 = 0x2b4;
const ACTION_TYPE: u32 = 0x4;
const ACTION_OPERAND: u32 = 0xc;
const ACTION_SLOT_FLOAT: u32 = 0x0;
const VTABLE_FLOAT_STACK: u32 = 0x0106_cc64;
const FLOAT_STACK_CONSTRUCT: u32 = 0x006b_3eb0;
const FLOAT_STACK_PUSH: u32 = 0x006d_c320;
const FLOAT_STACK_DESTROY: u32 = 0x006d_b720;
const USER_TRAIT_MAP: u32 = 0x011f_3308;
const FIRST_USER_TEXT_ID: i32 = 10000;
const GET_UNDERSCORE_VALUE: u32 = 0x00a0_a0b0;
const UIO_OPERATOR_SITE: u32 = 0x00a0_968c;
const FLOOR: u32 = 0x00ec_6940;
const CEIL: u32 = 0x00ec_9e10;
const FABS: u32 = 0x00ec_6cde;
const FTOL: u32 = 0x00ec_62c0;
const STRCPY_S: u32 = 0x00ec_65a6;
const SPRINTF_S: u32 = 0x0040_6d00;
const FORMAT_INTEGER: u32 = 0x0102_0764;
const FINAL_POST_PARSE: u32 = 0x00a0_37e0;
/// The doubles 0.0 and 0.5 in `.rdata`.
const ZERO_DOUBLE: u32 = 0x0101_2060;
const HALF_DOUBLE: u32 = 0x0101_1588;
/// The action classes' operand functions: `Tile::FloatAction::QFloat` (its float at `+0xc`),
/// `Tile::RefValueAction::QFloat` (the float of the value at `+0xc`, 0.0 without one), the
/// reference action's `QRefValue` (the value at `+0xc`) and the plain action's (0).
const FLOAT_ACTION_Q_FLOAT: u32 = 0x00a0_c000;
const REF_ACTION_Q_FLOAT: u32 = 0x00a0_c020;
const REF_ACTION_Q_REF_VALUE: u32 = 0x00a0_c060;
const NO_REF_VALUE: u32 = 0x00ad_87c0;
/// The value actions (`VALUE_ACTION`) and the parentheses.
const VA_COPY: i32 = 2000;
const VA_ADD: i32 = 2001;
const VA_SUB: i32 = 2002;
const VA_MULT: i32 = 2003;
const VA_DIV: i32 = 2004;
const VA_MIN: i32 = 2005;
const VA_MAX: i32 = 2006;
const VA_MOD: i32 = 2007;
const VA_FLOOR: i32 = 2008;
const VA_CEIL: i32 = 2009;
const VA_ABS: i32 = 2010;
const VA_ROUND: i32 = 2011;
const VA_GT: i32 = 2012;
const VA_GTE: i32 = 2013;
const VA_EQ: i32 = 2014;
const VA_NEQ: i32 = 2015;
const VA_LT: i32 = 2016;
const VA_LTE: i32 = 2017;
const VA_AND: i32 = 2018;
const VA_OR: i32 = 2019;
const VA_NOT: i32 = 2020;
const VA_ONLYIF: i32 = 2021;
const VA_ONLYIFNOT: i32 = 2022;
const ACTION_LEFT_PAREN: i32 = 0x7e8;
const ACTION_RIGHT_PAREN: i32 = 0x7e9;

/// An action's `QFloat` (vtable slot 0): read in place when the slot holds one of the two action
/// classes' own, else called through the vtable. A float read in place comes back as the call
/// answers it in `ST0`: loaded into the x87 register, so a signalling NaN comes back quiet (its
/// quiet bit set, sign and payload kept).
fn action_float(e: &mut Engine, action: u32) -> f32 {
    let as_answered = |float: f32| {
        if float.is_nan() {
            f32::from_bits(float.to_bits() | 0x0040_0000)
        } else {
            float
        }
    };
    match crate::slot_target(e, action, ACTION_SLOT_FLOAT) {
        FLOAT_ACTION_Q_FLOAT => as_answered(e.mem.f32(action + ACTION_OPERAND)),
        REF_ACTION_Q_FLOAT => match e.mem.u32(action + ACTION_OPERAND) {
            0 => 0.0,
            value => as_answered(e.mem.f32(value + VALUE_FLOAT)),
        },
        _ => e.vcall(action, ACTION_SLOT_FLOAT, &[]).f32(),
    }
}

/// An action's `QRefValue` (vtable slot 4): read in place when the slot holds one of the two
/// action classes' own, else called through the vtable.
fn action_ref_value(e: &mut Engine, action: u32) -> u32 {
    match crate::slot_target(e, action, ACTION_SLOT_REFERENCED) {
        REF_ACTION_Q_REF_VALUE => e.mem.u32(action + ACTION_OPERAND),
        NO_REF_VALUE => 0,
        _ => e.vcall(action, ACTION_SLOT_REFERENCED, &[]).u32(),
    }
}

/// `FISTP` of a float in the default rounding mode: to nearest, ties to even; out of range or not a
/// number gives `0x80000000`.
fn float_to_int_rounded(value: f32) -> i32 {
    let value = f64::from(value);
    let below = value.floor();
    let rounded = if value - below > 0.5 || (value - below == 0.5 && below % 2.0 != 0.0) {
        below + 1.0
    } else {
        below
    };
    // Already an integer: the store toward zero keeps it, and gives the integer indefinite for a
    // NaN or a value out of range.
    engine_core::x87::fistp_chop(rounded)
}

/// `_ftol` (by address) of a float.
fn float_to_int(e: &mut Engine, value: f32) -> i32 {
    e.call(FTOL, &args![f64::from(value)]).i32()
}

/// `!(a < b)`: true when `a >= b` and when the two are unordered.
#[allow(clippy::neg_cmp_op_on_partial_ord)]
fn not_less(a: f64, b: f64) -> bool {
    !(a < b)
}

// Typed from 00a09410
/// `Tile::Value::CalculateValue` (Xbox PDB; `thiscall(this, flag)`): the value's actions run in
/// order from its float (a reference action's operand is the other value's float, and its string;
/// a user trait whose number in the user-trait map is -1 is looked up by `GetUnderscoreValue` for
/// the current value rounded), the parentheses through a stack of floats; then the value counts as
/// changed when it has no string and its float moved, an `eString` trait's string is rewritten as
/// the rounded integer when the float moved, and when it changed (or `flag`) the values that read
/// it are recalculated (`ValueChangeEvent`) and the tile, unless released, is asked about the trait
/// (vtable `+0x18`, `PostParse`), an answer of 0 leading to `Tile::FinalPostParse` (by address).
/// Nothing for a tile still loading (but its class) or one its menu is deleting; the tile is read
/// without a null check first, as the exe reads it.
///
/// Every value a menu's build sets or links is calculated here, at least twice (when it is set and
/// at its tile's end), and again whenever a value it reads changes: in the translation each action
/// asked its operand through two or three calls through its vtable.
pub fn calculate_value(e: &mut Engine, this: Ptr, flag: u8) {
    let me = this.addr();
    let tile = e.mem.u32(me + VALUE_PARENT);
    let trait_id = e.mem.i32(me + VALUE_ID);
    let tile_flags = e.mem.u32(tile.wrapping_add(TILE_FLAGS));
    if !(tile_flags & FLAG_TILE_LOADING == 0 || trait_id == TRAIT_CLASS) {
        return;
    }
    if tile == 0 || e.mem.u32(tile + TILE_FLAGS) & FLAG_MENU_DELETING != 0 {
        return;
    }
    let line_word = e.tls() + TLS_CURRENT_LINE;
    let saved_line = e.mem.u32(line_word);
    e.mem.set_u32(line_word, 0xd);
    let mut action = e.mem.u32(me + VALUE_ACTIONS);
    let start = e.mem.f32(me + VALUE_FLOAT);
    let mut changed = false;
    e.with_stack(0x10, |e, stack| {
        let s = stack.addr();
        e.mem.set_u32(s, VTABLE_FLOAT_STACK);
        e.call(FLOAT_STACK_CONSTRUCT, &args![s, 0u32, 0u32]);
        while action != 0 {
            run_action(e, me, action, s, &mut changed);
            action = e.mem.u32(action + ACTION_NEXT);
        }
        if e.mem.u32(me + VALUE_STRING) == 0 {
            changed = e.mem.f32(me + VALUE_FLOAT) != start;
        }
        let float = e.mem.f32(me + VALUE_FLOAT);
        if float != start && trait_id == TRAIT_STRING {
            let number = float_to_int_rounded(float);
            let mut digits = u32::from(number == 0);
            let mut rest = number;
            while rest != 0 {
                digits += 1;
                rest /= 10;
            }
            if number < 0 {
                digits += 1;
            }
            let text = e.mem.u32(me + VALUE_STRING);
            if text == 0
                || typed_call!(crt::FAMILY, 0x00ec_6130, crt::string::strlen(e, text)) != digits
            {
                let old = e.mem.u32(me + VALUE_STRING);
                deallocate(e, old);
                let new_text = allocate(e, digits + 1);
                e.mem.set_u32(me + VALUE_STRING, new_text);
            }
            let text = e.mem.u32(me + VALUE_STRING);
            e.call(SPRINTF_S, &args![text, digits + 1, FORMAT_INTEGER, number]);
            changed = true;
        }
        if changed || flag != 0 {
            typed_call!(super::FAMILY, 0x00a0_a220, value_change_event(e, this));
            let tile = e.mem.u32(me + VALUE_PARENT);
            if tile != 0 && e.mem.u32(tile + TILE_FLAGS) & FLAG_RELEASED == 0 {
                let value = e.mem.f32(me + VALUE_FLOAT);
                let text = e.mem.u32(me + VALUE_STRING);
                let handled = e.vcall(tile, 0x18, &args![trait_id, value, text]);
                if handled.u32() == 0 {
                    e.call(FINAL_POST_PARSE, &args![tile, trait_id, value, text]);
                }
            }
        }
        e.call(FLOAT_STACK_DESTROY, &args![s]);
    });
    e.mem.set_u32(line_word, saved_line);
}

/// One action of [`calculate_value`] on the value at `me`, in the exe's order of reads and calls.
// Each operation rounds its double result to the float the exe stores (`FSTP float`), as the
// translation does.
#[allow(clippy::cast_possible_truncation)]
fn run_action(e: &mut Engine, me: u32, action: u32, stack: u32, changed: &mut bool) {
    let mut operand = action_float(e, action);
    let mut text = 0u32;
    if action_ref_value(e, action) != 0 {
        let referenced = action_ref_value(e, action);
        let mut found_number: i32 = 0;
        text = e.mem.u32(referenced + VALUE_STRING);
        let referenced_trait = e.mem.i32(referenced);
        if referenced_trait >= FIRST_USER_TEXT_ID {
            e.with_stack(4, |e, out| {
                // The exe zeroes its local first: the lookup leaves it alone when the trait is
                // not in the map.
                e.mem.set_u32(out.addr(), 0);
                typed_call!(
                    super::FAMILY,
                    0x00a1_e380,
                    refs::map_lookup(
                        e,
                        Ptr::new(USER_TRAIT_MAP),
                        referenced_trait as u32,
                        Ptr::new(out.addr())
                    )
                );
                found_number = e.mem.i32(out.addr());
            });
        }
        if found_number == -1 {
            let rounded = float_to_int_rounded(e.mem.f32(me + VALUE_FLOAT));
            let owner = e.mem.u32(referenced + VALUE_PARENT);
            let found = e
                .call(
                    GET_UNDERSCORE_VALUE,
                    &args![owner, referenced_trait, rounded],
                )
                .u32();
            if found != 0 {
                operand = e.mem.f32(found + VALUE_FLOAT);
                text = e.mem.u32(found + VALUE_STRING);
            }
        }
    }
    let mut kind = e.mem.i32(action + ACTION_TYPE);
    if kind == ACTION_LEFT_PAREN {
        e.call(FLOAT_STACK_PUSH, &args![stack, me + VALUE_FLOAT]);
        e.mem.set_f32(me + VALUE_FLOAT, 0.0);
    } else if kind == ACTION_RIGHT_PAREN {
        let size = e.mem.u32(stack + 8);
        let top = e.mem.f32(
            e.mem
                .u32(stack + 4)
                .wrapping_add(size.wrapping_mul(4))
                .wrapping_sub(4),
        );
        let encoded = action_float(e, action);
        kind = float_to_int_rounded(encoded);
        operand = e.mem.f32(me + VALUE_FLOAT);
        e.mem.set_f32(me + VALUE_FLOAT, top);
        if size != 0 {
            e.mem.set_u32(stack + 8, size - 1);
        }
    }
    // UI Organizer's jump table (its patch at 00a0968c) runs the operation in place of the switch
    // below; its answer is the "string changed" flag afterwards.
    if e.uio() {
        let at = [
            me,
            kind as u32,
            operand.to_bits(),
            text,
            u32::from(*changed),
        ];
        if let Some(flag) = e.site(UIO_OPERATOR_SITE, &at) {
            *changed = flag != 0;
            return;
        }
    }
    let current = e.mem.f32(me + VALUE_FLOAT);
    let (current64, operand64) = (f64::from(current), f64::from(operand));
    let zero = e.mem.f64(ZERO_DOUBLE);
    let flag = |condition: bool| if condition { 1.0f32 } else { 0.0f32 };
    let result: Option<f32> = match kind {
        VA_COPY => {
            if text == 0 {
                Some(operand)
            } else {
                let length = typed_call!(crt::FAMILY, 0x00ec_6130, crt::string::strlen(e, text));
                let old = e.mem.u32(me + VALUE_STRING);
                *changed = old == 0
                    || typed_call!(crt::FAMILY, 0x00ec_6da0, crt::string::strcmp(e, old, text))
                        != 0;
                if old != 0 {
                    deallocate(e, old);
                    e.mem.set_u32(me + VALUE_STRING, 0);
                }
                let copy = allocate(e, length + 1);
                e.mem.set_u32(me + VALUE_STRING, copy);
                e.call(STRCPY_S, &args![copy, length + 1, text]);
                None
            }
        }
        VA_ADD => Some((current64 + operand64) as f32),
        VA_SUB => Some((current64 - operand64) as f32),
        VA_MULT => Some((current64 * operand64) as f32),
        VA_DIV => {
            if operand64 != zero {
                Some((current64 / operand64) as f32)
            } else {
                None
            }
        }
        VA_MIN => Some(if current < operand { current } else { operand }),
        VA_MAX => Some(if operand < current { current } else { operand }),
        VA_MOD => {
            if operand64 != zero {
                let dividend = float_to_int_rounded(current);
                let divisor = float_to_int_rounded(operand);
                Some((dividend % divisor) as f32)
            } else {
                None
            }
        }
        VA_FLOOR => {
            let sum = (operand64 + current64) as f32;
            Some(e.call(FLOOR, &args![f64::from(sum)]).f64() as f32)
        }
        VA_CEIL => {
            let sum = (operand64 + current64) as f32;
            Some(e.call(CEIL, &args![f64::from(sum)]).f64() as f32)
        }
        VA_ABS => {
            let sum = (operand64 + current64) as f32;
            Some(e.call(FABS, &args![f64::from(sum)]).f64() as f32)
        }
        VA_ROUND => {
            let quotient = (current64 / operand64) as f32;
            let truncated = float_to_int(e, quotient);
            let round_up = i32::from(not_less(
                f64::from(quotient) - f64::from(truncated),
                e.mem.f64(HALF_DOUBLE),
            ));
            let multiple = float_to_int(e, quotient).wrapping_add(round_up);
            Some((operand64 * f64::from(multiple)) as f32)
        }
        VA_GT => Some(flag(operand < current)),
        VA_GTE => Some(flag(operand <= current)),
        VA_EQ => Some(flag(operand == current)),
        VA_NEQ => Some(flag(operand != current)),
        VA_LT => Some(flag(current < operand)),
        VA_LTE => Some(flag(current <= operand)),
        VA_AND => Some(flag(if text == 0 {
            current64 != zero && operand64 != zero
        } else {
            current64 != zero
        })),
        VA_OR => Some(flag(if text == 0 {
            current64 != zero || operand64 != zero
        } else {
            current64 != zero || text != 0
        })),
        VA_NOT => Some(if text == 0 {
            flag(operand64 == zero)
        } else {
            0.0
        }),
        VA_ONLYIF => {
            if operand64 == zero && text == 0 {
                Some(0.0)
            } else {
                None
            }
        }
        VA_ONLYIFNOT => {
            if operand64 != zero || text != 0 {
                Some(0.0)
            } else {
                None
            }
        }
        _ => None,
    };
    if let Some(value) = result {
        e.mem.set_f32(me + VALUE_FLOAT, value);
    }
}
