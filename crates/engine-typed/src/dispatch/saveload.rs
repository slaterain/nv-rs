//! `BGSSaveLoadGame::CleanupExpiredData` (`0084ab20`), which every save runs first (autosaves at
//! doors too): it drops the stored state of references in exterior cells left long ago. The exe keeps
//! the expired cells in a temporary `NiTMap<__int64,int>` of 37 buckets whose hash is the key's low
//! word, the worldspace's id, so every expired cell of one worldspace sits in one chain and each of
//! the changes map's references walks the whole chain (two virtual calls a step). Expired cells keep
//! their entry, so the walk grows over a playthrough: about 0.8 s at 200 expired cells and 40,000
//! changed references, 5.6 s at 1,500 (debug build, `crates/engine/tests/save_maps.rs`). Engine
//! Optimizations fixes the hash and gives it 4,999 buckets (lStewieAl's Tweaks, `InitSaveMapHook`,
//! `NiTMap_int64_int__Insert`/`__Lookup`).
//!
//! Here the queue is built, filled, flushed and freed as the exe does it (the same calls, heap blocks
//! and memory), and a Rust hash map beside it, filled with the same keys and values, answers the
//! finds (`0084b8b0`): the second pass's walk of the chain is the cost, and it is gone. The answer
//! and the "found" slot are the exe's, so the changes map the function leaves, and the save, are the
//! same byte for byte. The inserts still walk their chain as the exe does (a smaller cost: once per
//! expired cell).

use engine_core::ptr::Ptr;
use engine_core::typed::leaf;
use engine_core::{args, typed_call, Engine};
use std::collections::HashMap;

/// `BGSSaveLoadGame`'s fields the clean-up reads.
const CHANGES_MAP: u32 = 0x00;
const REFERENCES_MAP: u32 = 0x10;
const HISTORY: u32 = 0x1c;

/// The data handler (`011c3f2c`) and the calendar (`011de7b8`).
const DATA_HANDLER: u32 = 0x011c_3f2c;
const CALENDAR: u32 = 0x011d_e7b8;

const SET_THREAD_FLAG: u32 = 0x0046_23f0;
const CALENDAR_TIME_STAMP: u32 = 0x0086_7e30;
const MAP_FIRST_POSITION: u32 = 0x004b_9ba0;
const MAP_NEXT: u32 = 0x006b_7f20;
const FLAGS_TEST: u32 = 0x0042_80f0;
const FLAGS_EQUAL: u32 = 0x0082_2510;
const FLAGS_CLEAR: u32 = 0x0056_2210;
const STORED_BUFFER_TYPE: u32 = 0x0085_3640;
const INITIAL_DATA_KIND: u32 = 0x0084_e730;
const UNLOADED_FORM_BUFFER_ADVANCED: u32 = 0x0086_6390;
const UNLOADED_FORM_BUFFER_SIZE: u32 = 0x0086_63e0;
const UNLOADED_FORM_BUFFER_CLEAR: u32 = 0x0066_65a0;
const UNLOADED_FORM_BUFFER_INITIALIZE: u32 = 0x0086_65d0;
const LOAD_INITIAL_DATA_STRUCT: u32 = 0x0084_e930;
const INITIAL_DATA_TYPE_SIZE: u32 = 0x0084_e800;
const LOAD_VARIABLE_SIZED_VALUE: u32 = 0x0084_6080;
const LOOKUP_FORM: u32 = 0x0048_39c0;
const FORM_GET_TYPE: u32 = 0x0040_1170;
const CELL_DETACH_TIME: u32 = 0x0054_6af0;
const CELL_FLAG_ZERO_TEST: u32 = 0x0042_5fd0;
const CELL_WORLD_SPACE: u32 = 0x0054_ddd0;
const CELL_DATA_X: u32 = 0x0054_4c30;
const CELL_DATA_Y: u32 = 0x0054_4c60;
const EXPIRY_LIMIT: u32 = 0x0052_6100;
const PACK_GRID_COORDS: u32 = 0x0058_7410;
const ROUND_TO_INT: u32 = 0x0040_6d90;
const IS_DYNAMIC_FORM_ID: u32 = 0x0046_9860;
const REFERENCE_PARENT_CELL: u32 = 0x008d_6f30;
const REFERENCE_WORLD_SPACE: u32 = 0x0057_5d70;
const REFERENCE_MOVE_TO_LOCATION: u32 = 0x0056_1ef0;
const CHANGES_MAP_REMOVE: u32 = 0x0084_5a20;
const MAP_REMOVE_AT: u32 = 0x0040_5430;
const REFERENCES_MAP_REMOVE: u32 = 0x0085_2a10;
const REFERENCES_MAP_REMOVE_EXTERIOR: u32 = 0x0085_2cd0;
const OPERATOR_DELETE: u32 = 0x0040_1030;
/// The expiry queue (`NiTMap<__int64,int>`): base constructor, own vtable, insert, flush, destructor.
const EXPIRY_QUEUE_BASE_CONSTRUCT: u32 = 0x0084_b750;
const EXPIRY_QUEUE_VTABLE: u32 = 0x0107_f494;
const EXPIRY_QUEUE_FLUSH: u32 = 0x0043_8af0;
const EXPIRY_QUEUE_DESTRUCT: u32 = 0x0084_b9c0;
const SAVE_GAME_WARNING: u32 = 0x0084_5cc0;
const HISTORY_ADD_NOTE: u32 = 0x0084_dff0;
/// `"SAVELOAD: CleanupExpiredData: no form or buffer for %08X (flags %08X)"`-style warning.
const CLEANUP_MISSING_BUFFER_WARNING: u32 = 0x0107_f410;
/// The history note with the three counts.
const CLEANUP_NOTE: u32 = 0x0107_f398;
/// A form's virtual slot "is it a reference" and a reference's place.
const FORM_SLOT_IS_REFERENCE: u32 = 0xf0;
const REFERENCE_SLOT_PLACE: u32 = 0x1f4;

/// The frame slots the exe's passes use (the translation's offsets into its block).
const POSITION: u32 = 0x00;
const KEY: u32 = 0x04;
const ENTRY: u32 = 0x08;
const FOUND: u32 = 0x0c;
const VALUE_SIZE: u32 = 0x10;
const INITIAL_DATA: u32 = 0x20;
const SECOND_INITIAL_DATA: u32 = 0x50;

fn word(e: &mut Engine, address: u32) -> u32 {
    typed_call!(leaf::FAMILY, 0x0055_9450, leaf::word_at_0(e, address))
}

fn form_id(e: &mut Engine, form: u32) -> u32 {
    typed_call!(leaf::FAMILY, 0x0084_e3a0, leaf::word_at_c(e, form))
}

/// `00849280`: the initial data structure's constructor (the identity cast on +0x14 and +0x20).
fn initial_data(e: &mut Engine, at: u32) {
    typed_call!(leaf::FAMILY, 0x0068_15c0, leaf::same_word(e, at + 0x14));
    typed_call!(leaf::FAMILY, 0x0068_15c0, leaf::same_word(e, at + 0x20));
}

/// `00849220`: the stored buffer's form type.
fn stored_type(e: &mut Engine, buffer_slot: u32) -> u32 {
    let inner = e.mem.u32(buffer_slot);
    e.call(STORED_BUFFER_TYPE, &args![inner]).u32()
}

/// The expiry queue's 64-bit key as two words: the area, then (with grid coordinates) the packed
/// coordinates.
fn expiry_key(e: &mut Engine, area: u32, x: u32, y: u32) -> (u32, u32) {
    if x != 0x7fff_ffff {
        let packed = e
            .call(PACK_GRID_COORDS, &args![x & 0xffff, y & 0xffff])
            .u32();
        (area, packed)
    } else {
        (area, 0)
    }
}

fn grid_of(e: &mut Engine, value: u32) -> u32 {
    let rounded = e.call(ROUND_TO_INT, &args![f32::from_bits(value)]).i32();
    u32::from_ne_bytes((rounded >> 12).to_ne_bytes())
}

// Typed from 0084ab20
/// `BGSSaveLoadGame::CleanupExpiredData` (`0084ab20`) with the expiry queue native.
pub fn cleanup_expired_data(e: &mut Engine, this: Ptr) {
    let saved_flag = e.call(SET_THREAD_FLAG, &args![this, 0u32]).u8();
    // The exe's queue had 0x10 bytes of frame, then its passes 0xa0.
    e.with_stack(0x10, |e, queue| {
        // `0084b4e0`: the queue's base constructor (37 buckets), then its own vtable.
        e.call(EXPIRY_QUEUE_BASE_CONSTRUCT, &args![queue, 0x25u32]);
        e.mem.set_u32(queue.addr(), EXPIRY_QUEUE_VTABLE);
        e.with_stack(0xa0, |e, frame| {
            body(e, this.addr(), queue.addr(), frame.addr());
        });
        e.call(EXPIRY_QUEUE_FLUSH, &args![queue]);
        e.call(SET_THREAD_FLAG, &args![this, u32::from(saved_flag)]);
        e.call(EXPIRY_QUEUE_DESTRUCT, &args![queue]);
    });
}

fn body(e: &mut Engine, this: u32, queue: u32, frame: u32) {
    // The queue's keys and values beside it, for the finds.
    let mut expired_cells: HashMap<(u32, u32), u32> = HashMap::new();
    let data_handler = e.mem.u32(DATA_HANDLER);
    let position = frame + POSITION;
    let key_slot = frame + KEY;
    let entry_slot = frame + ENTRY;
    let init = frame + INITIAL_DATA;
    let now = e.call(CALENDAR_TIME_STAMP, &args![CALENDAR]).u32();
    let mut expired = 0u32;

    // First pass: which cells have expired.
    let changes = e.mem.u32(this + CHANGES_MAP);
    let first = e.call(MAP_FIRST_POSITION, &args![changes]).u32();
    e.mem.set_u32(position, first);
    while e.mem.u32(position) != 0 {
        e.mem.set_u32(key_slot, 0);
        e.mem.set_u32(entry_slot, 0);
        let changes = e.mem.u32(this + CHANGES_MAP);
        e.call(MAP_NEXT, &args![changes, position, key_slot, entry_slot]);
        let key = e.mem.u32(key_slot);
        let entry = e.mem.u32(entry_slot);
        let mut area = key;
        if !e.call(FLAGS_TEST, &args![entry, 0x4000_0000u32]).bool() {
            continue;
        }
        let mut detach_time = 0u32;
        let mut x = 0x7fff_ffffu32;
        let mut y = 0x7fff_ffffu32;
        if word(e, entry + 4) != 0 {
            if stored_type(e, entry + 4) == 0x39 {
                let flags = e.mem.u32(entry);
                let kind = e.call(INITIAL_DATA_KIND, &args![key, 0x39u32, flags]).u32();
                if kind != 0 {
                    let advanced = e
                        .call(UNLOADED_FORM_BUFFER_ADVANCED, &args![entry + 4])
                        .u32();
                    e.call(LOAD_INITIAL_DATA_STRUCT, &args![advanced, kind, init]);
                    detach_time = e.mem.u32(init + 0xc);
                    if kind != 3 {
                        area = e.mem.u32(init);
                        x = e.mem.u32(init + 4);
                        y = e.mem.u32(init + 8);
                    }
                }
            }
        } else {
            let cell = e.call(LOOKUP_FORM, &args![key]).u32();
            if cell != 0 && e.call(FORM_GET_TYPE, &args![cell]).u32() == 0x39 {
                detach_time = e.call(CELL_DETACH_TIME, &args![cell]).u32();
                if !e.call(CELL_FLAG_ZERO_TEST, &args![cell]).bool() {
                    let world_space = e.call(CELL_WORLD_SPACE, &args![cell]).u32();
                    area = form_id(e, world_space);
                    x = e.call(CELL_DATA_X, &args![cell]).u32();
                    y = e.call(CELL_DATA_Y, &args![cell]).u32();
                }
            }
        }
        if detach_time != 0 && now.wrapping_sub(detach_time) > e.call(EXPIRY_LIMIT, &args![]).u32()
        {
            let (low, high) = expiry_key(e, area, x, y);
            typed_call!(
                super::FAMILY,
                0x0084_b7c0,
                queue_insert(e, Ptr::new(queue), low, high, detach_time)
            );
            expired_cells.insert((low, high), detach_time);
            expired += 1;
        }
    }
    if expired == 0 {
        return;
    }

    // Second pass: strip the stored state of references in those cells.
    let mut references_removed = 0u32;
    let mut all_removed = 0u32;
    let mut partial_removed = 0u32;
    let changes = e.mem.u32(this + CHANGES_MAP);
    let first = e.call(MAP_FIRST_POSITION, &args![changes]).u32();
    e.mem.set_u32(position, first);
    while e.mem.u32(position) != 0 {
        e.mem.set_u32(key_slot, 0);
        e.mem.set_u32(entry_slot, 0);
        let changes = e.mem.u32(this + CHANGES_MAP);
        e.call(MAP_NEXT, &args![changes, position, key_slot, entry_slot]);
        let key = e.mem.u32(key_slot);
        let entry = e.mem.u32(entry_slot);
        if e.call(IS_DYNAMIC_FORM_ID, &args![data_handler, key]).bool() {
            continue;
        }
        let mut area = 0u32;
        let mut x = 0u32;
        let mut y = 0u32;
        let mut reference = 0u32;
        let form_type;
        if word(e, entry + 4) != 0 {
            form_type = stored_type(e, entry + 4);
        } else {
            let form = e.call(LOOKUP_FORM, &args![key]).u32();
            if form != 0 {
                form_type = e.call(FORM_GET_TYPE, &args![form]).u32();
                if e.vcall(form, FORM_SLOT_IS_REFERENCE, &args![]).bool() {
                    reference = form;
                }
            } else {
                form_type = 0;
                let flags = word(e, entry);
                e.call(
                    SAVE_GAME_WARNING,
                    &args![CLEANUP_MISSING_BUFFER_WARNING, key, flags],
                );
            }
        }
        if form_type == 0x3a {
            if word(e, entry + 4) != 0 {
                let flags = e.mem.u32(entry);
                let kind = e
                    .call(INITIAL_DATA_KIND, &args![key, form_type, flags])
                    .u32();
                if kind != 0 {
                    initial_data(e, init);
                    let advanced = e
                        .call(UNLOADED_FORM_BUFFER_ADVANCED, &args![entry + 4])
                        .u32();
                    e.call(LOAD_INITIAL_DATA_STRUCT, &args![advanced, kind, init]);
                    area = e.mem.u32(init);
                    let float_x = e.mem.u32(init + 0x14);
                    let float_y = e.mem.u32(init + 0x18);
                    x = grid_of(e, float_x);
                    y = grid_of(e, float_y);
                }
            } else if reference != 0 {
                let parent = e.call(REFERENCE_PARENT_CELL, &args![reference]).u32();
                let world_space = e.call(REFERENCE_WORLD_SPACE, &args![reference]).u32();
                if parent != 0 && e.call(CELL_FLAG_ZERO_TEST, &args![parent]).bool() {
                    area = form_id(e, parent);
                } else if world_space != 0 {
                    area = form_id(e, world_space);
                    let place = e.vcall(reference, REFERENCE_SLOT_PLACE, &args![]).u32();
                    let float_x = e.mem.u32(place);
                    let float_y = e.mem.u32(place + 4);
                    x = grid_of(e, float_x);
                    y = grid_of(e, float_y);
                }
            }
        }
        if area == 0 {
            continue;
        }
        let key_words = expiry_key(e, area, x, y);
        // `0084b8b0` (the find): the exe zeroes its "found" slot and stores the value there.
        let found = frame + FOUND;
        e.mem.set_u32(found, 0);
        let in_queue = match expired_cells.get(&key_words) {
            Some(value) => {
                e.mem.set_u32(found, *value);
                true
            }
            None => false,
        };
        if in_queue
            && e.call(FLAGS_TEST, &args![entry, 4u32]).bool()
            && !e.call(FLAGS_TEST, &args![entry, 2u32]).bool()
        {
            references_removed += 1;
            let flags = e.mem.u32(entry);
            let kind = e
                .call(INITIAL_DATA_KIND, &args![key, form_type, flags])
                .u32();
            let second_init = frame + SECOND_INITIAL_DATA;
            initial_data(e, second_init);
            let references_map = e.mem.u32(this + REFERENCES_MAP);
            if word(e, entry + 4) != 0 && kind == 6 {
                let advanced = e
                    .call(UNLOADED_FORM_BUFFER_ADVANCED, &args![entry + 4])
                    .u32();
                e.call(
                    LOAD_INITIAL_DATA_STRUCT,
                    &args![advanced, kind, second_init],
                );
                e.call(MAP_REMOVE_AT, &args![references_map, key]);
                let cell_id = e.mem.u32(second_init);
                let cell = e.call(LOOKUP_FORM, &args![cell_id]).u32();
                if cell != 0 && e.call(FORM_GET_TYPE, &args![cell]).u32() == 0x41 {
                    e.call(
                        REFERENCES_MAP_REMOVE_EXTERIOR,
                        &args![references_map, cell_id, key, second_init + 0x14],
                    );
                } else {
                    e.call(
                        REFERENCES_MAP_REMOVE,
                        &args![references_map + 0x10, cell_id, key],
                    );
                }
            }
            if e.call(FLAGS_EQUAL, &args![entry, 4u32]).bool() {
                let changes = e.mem.u32(this + CHANGES_MAP);
                e.call(CHANGES_MAP_REMOVE, &args![changes, key]);
                all_removed += 1;
            } else {
                if word(e, entry + 4) != 0 && kind != 0 {
                    let advanced = e
                        .call(UNLOADED_FORM_BUFFER_ADVANCED, &args![entry + 4])
                        .u32();
                    let header_size = e.call(INITIAL_DATA_TYPE_SIZE, &args![kind]).u32();
                    let after_header = advanced.wrapping_add(header_size);
                    let size_slot = frame + VALUE_SIZE;
                    e.mem.set_u32(size_slot, 0);
                    let length_size = e
                        .call(LOAD_VARIABLE_SIZED_VALUE, &args![after_header, size_slot])
                        .u32();
                    let value_size = e.mem.u32(size_slot);
                    let data = length_size
                        .wrapping_add(value_size)
                        .wrapping_add(after_header);
                    let old_buffer = word(e, entry + 4);
                    // `0084b450`: the stored block's flag byte at +1.
                    let block = e.mem.u32(entry + 4);
                    let flag = e.mem.u8(block + 1);
                    let total = e.call(UNLOADED_FORM_BUFFER_SIZE, &args![entry + 4]).u32();
                    let remaining = total
                        .wrapping_sub(header_size)
                        .wrapping_sub(length_size)
                        .wrapping_sub(value_size);
                    e.call(UNLOADED_FORM_BUFFER_CLEAR, &args![entry + 4]);
                    e.call(
                        UNLOADED_FORM_BUFFER_INITIALIZE,
                        &args![entry + 4, data, remaining, form_type, flag],
                    );
                    e.call(OPERATOR_DELETE, &args![old_buffer]);
                    partial_removed += 1;
                }
                e.call(FLAGS_CLEAR, &args![entry, 0xcu32]);
            }
            if reference != 0 {
                e.call(REFERENCE_MOVE_TO_LOCATION, &args![reference, 0u32]);
            }
        }
    }
    if references_removed != 0 {
        let history = e.mem.u32(this + HISTORY);
        e.call(
            HISTORY_ADD_NOTE,
            &args![
                history,
                CLEANUP_NOTE,
                references_removed,
                all_removed,
                partial_removed
            ],
        );
    }
}

// The expiry queue's own functions (`NiTMap<__int64,int>`): its hash, its key compare and its
// insert, so the clean-up's inserts walk their chain as Rust code (no virtual call a step). A queue
// whose vtable holds other functions there is asked through them, as the exe asks.

/// The queue's item: the next item at +0, the 64-bit key at +8 and +0xc, the value at +0x10.
const ITEM_KEY_LOW: u32 = 0x08;
const ITEM_KEY_HIGH: u32 = 0x0c;
const ITEM_VALUE: u32 = 0x10;
/// The map's bucket count, bucket array and item count.
const MAP_SIZE: u32 = 0x04;
const MAP_TABLE: u32 = 0x08;
const MAP_COUNT: u32 = 0x0c;
/// The queue's vtable slots: hash, key compare, set value, new item.
const SLOT_HASH: u32 = 0x04;
const SLOT_EQUAL: u32 = 0x08;
const SLOT_SET_VALUE: u32 = 0x0c;
const SLOT_NEW_ITEM: u32 = 0x14;
const QUEUE_HASH: u32 = 0x0084_b930;
const QUEUE_KEYS_EQUAL: u32 = 0x0084_b950;

// Typed from 0084b930
/// `NiTMapBase<...,__int64,int>::KeyToHashIndex` (Xbox PDB): the key's low word modulo the bucket
/// count (the high word is not read).
pub fn queue_hash(e: &mut Engine, this: Ptr, key_low: u32, _key_high: u32) -> u32 {
    key_low % e.mem.u32(this.addr() + MAP_SIZE)
}

// Typed from 0084b950
/// `NiTMapBase<...,__int64,int>::IsKeysEqual` (Xbox PDB): both words of the two keys are equal.
pub fn queue_keys_equal(
    _e: &mut Engine,
    _this: Ptr,
    low_a: u32,
    high_a: u32,
    low_b: u32,
    high_b: u32,
) -> bool {
    low_a == low_b && high_a == high_b
}

// Typed from 0084b7c0
/// `NiTMapBase<...,__int64,int>::SetAt` shape: an item with an equal key in the key's bucket gets
/// the new value; otherwise a new item (slot +0x14) is filled (slot +0xc), put at the head of the
/// bucket and counted.
pub fn queue_insert(e: &mut Engine, this: Ptr, key_low: u32, key_high: u32, value: u32) {
    let me = this.addr();
    let index = if crate::slot_target(e, me, SLOT_HASH) == QUEUE_HASH {
        typed_call!(
            super::FAMILY,
            0x0084_b930,
            queue_hash(e, this, key_low, key_high)
        )
    } else {
        e.vcall(me, SLOT_HASH, &args![key_low, key_high]).u32()
    };
    let direct_equal = crate::slot_target(e, me, SLOT_EQUAL) == QUEUE_KEYS_EQUAL;
    let table = e.mem.u32(me + MAP_TABLE);
    let mut item = e.mem.u32(table + index * 4);
    while item != 0 {
        let item_low = e.mem.u32(item + ITEM_KEY_LOW);
        let item_high = e.mem.u32(item + ITEM_KEY_HIGH);
        let equal = if direct_equal {
            typed_call!(
                super::FAMILY,
                0x0084_b950,
                queue_keys_equal(e, this, key_low, key_high, item_low, item_high)
            )
        } else {
            e.vcall(
                me,
                SLOT_EQUAL,
                &args![key_low, key_high, item_low, item_high],
            )
            .bool()
        };
        if equal {
            e.mem.set_u32(item + ITEM_VALUE, value);
            return;
        }
        item = e.mem.u32(item);
    }
    let item = e.vcall(me, SLOT_NEW_ITEM, &args![]).u32();
    e.vcall(me, SLOT_SET_VALUE, &args![item, key_low, key_high, value]);
    // The exe reads the bucket array again after the two calls, for the head (`0084b872`) and
    // again for the store (`0084b883`): a slot may have moved it.
    let table = e.mem.u32(me + MAP_TABLE);
    let head = e.mem.u32(table + index * 4);
    e.mem.set_u32(item, head);
    let table = e.mem.u32(me + MAP_TABLE);
    e.mem.set_u32(table + index * 4, item);
    let count = e.mem.u32(me + MAP_COUNT);
    e.mem.set_u32(me + MAP_COUNT, count.wrapping_add(1));
}
