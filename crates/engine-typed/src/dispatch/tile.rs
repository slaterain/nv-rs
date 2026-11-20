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

/// The tiles' critical section and the import thunks that enter and leave it.
const TILE_CRITICAL_SECTION: u32 = 0x011f_3330;
const ENTER_CRITICAL_SECTION: u32 = 0x00fd_f05c;
const LEAVE_CRITICAL_SECTION: u32 = 0x00fd_f100;
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
    e.call(ENTER_CRITICAL_SECTION, &args![TILE_CRITICAL_SECTION]);
}

fn leave(e: &mut Engine) {
    e.call(LEAVE_CRITICAL_SECTION, &args![TILE_CRITICAL_SECTION]);
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

// Typed from 00a09200
/// `Tile::Value::ClearActions` (Xbox PDB): deletes the trait's actions one by one. An action that
/// reads another trait (slot +4 answers it; asked each time, as the exe asks) is first taken out of
/// that trait's reaction list: this trait's reactions leave the list, and the map's entry is erased
/// when none are left or set when the head changed.
pub fn clear_actions(e: &mut Engine, this: Ptr) {
    let me = this.addr();
    let list_head = me + VALUE_ACTIONS;
    loop {
        let action = e.mem.u32(list_head);
        if action == 0 {
            break;
        }
        if e.vcall(action, ACTION_SLOT_REFERENCED, &[]).u32() != 0 {
            e.with_stack(4, |e, reactions| {
                let at = reactions.addr();
                e.mem.set_u32(at, 0);
                let referenced = e.vcall(action, ACTION_SLOT_REFERENCED, &[]).u32();
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
                    let referenced = e.vcall(action, ACTION_SLOT_REFERENCED, &[]).u32();
                    e.call(MAP_REMOVE, &args![REACTION_MAP, referenced]);
                } else if first_after != first_before {
                    let referenced = e.vcall(action, ACTION_SLOT_REFERENCED, &[]).u32();
                    set_reactions(e, referenced, first_after);
                }
            });
        }
        let next = e.mem.u32(action + ACTION_NEXT);
        e.mem.set_u32(list_head, next);
        deallocate(e, action);
    }
}
