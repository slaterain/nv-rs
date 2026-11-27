//! The extra data list in the old save format (the save-load stream, `TESSaveLoadGame`): the size a
//! list will take (`00422c40`), `ExtraDataList::SaveGame` (`004235e0`) and `LoadGame` (`00424960`).
//! The stream's one-instruction leaves are done in place as the AI families do them: the save-load
//! object's position (`00825c00`, +0x14), version byte (`008df040`, +0x80), whether it uses save-game
//! blocks (`00862110`, versions 0x1f to 0x59) and the record being loaded (`004fd3c0`, +0x84); the
//! debug switch's byte (`00408d60` answers the setting's value at +4); a created form's id
//! (`00469860`, an id from 0xff000000 on), a form's id (`0084e3a0`), a package's type (`0041ca90`),
//! a list node's item and next, a reference's parent cell (`008d6f30`, +0x40) and an empty world
//! location's constructor (`006d5320`, which only returns). The stream's reads and writes, the
//! numeric ids and the other units' savers and loaders are called by address.

use super::{
    access, followers, give_lock, head_of, kind_of, links, new_object, next_of, packages, setters,
    take_lock, word_at, zero_bytes, EXTRA_PAYLOAD, FAMILY,
};
use crate::crt::{low_byte, low_half};
use crate::{cell_refs, forms, multibound};
use engine_core::typed::leaf;
use engine_core::x87::x87_quiet_bits;
use engine_core::{args, typed_call, Engine, Ptr};

/// The save-load object (`TESSaveLoadGame`), a global, and its position (+0x14), version byte
/// (+0x80) and record being loaded (+0x84).
const SAVE_LOAD_GAME: u32 = 0x011d_e45c;
const POSITION: u32 = 0x14;
const VERSION: u32 = 0x80;
const LOADING_RECORD: u32 = 0x84;
/// The stream's write and read of bytes, and its numeric ids (`thiscall(save, at, 4)`).
const STREAM_WRITE: u32 = 0x0085_79b0;
const STREAM_READ: u32 = 0x0085_79e0;
const SAVE_NUMERIC_ID: u32 = 0x0085_7a10;
const LOAD_NUMERIC_ID: u32 = 0x0085_7aa0;
/// The debug switches setting, whose value byte (+4) turns the size messages on.
const DEBUG_SWITCH: u32 = 0x011d_e4e8 + 4;
/// The save block's tag (`KOLB` as the stream holds it).
const BLOCK_TAG: u32 = 0x424c_4f4b;
/// `__RTDynamicCast(object, 0, from, to, 0)` and the type descriptors it is given.
const RT_DYNAMIC_CAST: u32 = 0x00ec_43fb;
const RTTI_BS_EXTRA_DATA: u32 = 0x0118_3b2c;
const RTTI_TES_FORM: u32 = 0x0118_3028;
const RTTI_TES_OBJECT_REFR: u32 = 0x0118_41cc;
const RTTI_ACTOR: u32 = 0x0118_46d4;
const RTTI_TES_WORLD_SPACE: u32 = 0x0118_3fd0;
const RTTI_TES_OBJECT_CELL: u32 = 0x0118_3fb4;
const RTTI_TES_PACKAGE: u32 = 0x0118_46a0;
const RTTI_TES_KEY: u32 = 0x0118_41b4;
const RTTI_EXTRA_PERSISTENT_CELL: u32 = 0x0118_4594;
const RTTI_EXTRA_PACKAGE_START_LOCATION: u32 = 0x0118_448c;
const RTTI_EXTRA_PACKAGE: u32 = 0x0118_44f8;
const RTTI_EXTRA_RUN_ONCE_PACKS: u32 = 0x0118_44b4;
const RTTI_EXTRA_FOLLOWER: u32 = 0x0118_4470;
const RTTI_EXTRA_TELEPORT: u32 = 0x0118_4430;
const RTTI_EXTRA_ITEM_DROPPER: u32 = 0x0118_4574;
const RTTI_EXTRA_LAST_FINISHED_SEQUENCE: u32 = 0x0118_4408;
const RTTI_EXTRA_HEAD_TRACK_TARGET: u32 = 0x0118_4550;
const RTTI_EXTRA_INFO_GENERAL_TOPIC: u32 = 0x0118_43e4;
const RTTI_EXTRA_NO_RUMORS: u32 = 0x0118_4530;
/// `ModelLoader`'s message box (`overridable.tsv`: by address) and the `Error` messages.
const LOG_MESSAGE: u32 = 0x005b_5e40;
const SOURCE_FILE_NAME: u32 = 0x0101_52d0;
const SIZE_FORMAT: u32 = 0x0101_2cb0;
const SIZE_SHORT_FORMAT: u32 = 0x0101_2c78;
const SAVE_FORMAT: u32 = 0x0101_53a0;
const SAVE_SHORT_FORMAT: u32 = 0x0101_536c;
const BLOCK_TOO_LARGE: u32 = 0x0101_5318;
const NO_START_LOCATION: u32 = 0x0101_53f0;
const BAD_TAG_FORMAT: u32 = 0x0101_5718;
const BAD_TAG_SHORT_FORMAT: u32 = 0x0101_56a8;
const UNKNOWN_TYPE_FORMAT: u32 = 0x0101_5610;
const OVERREAD_FORMAT: u32 = 0x0101_5588;
const UNDERREAD_FORMAT: u32 = 0x0101_5500;
const OVERREAD_SHORT_FORMAT: u32 = 0x0101_54a0;
const UNDERREAD_SHORT_FORMAT: u32 = 0x0101_5440;
/// A form's name slot.
const NAME_SLOT: u32 = 0x130;
/// Other units: the list count, the teleport data's and the info general topic's sizes, savers,
/// loaders and constructors, the cell's and world space's calls, the reference's.
const LIST_COUNT: u32 = 0x005a_e380;
const TELEPORT_SAVE_SIZE: u32 = 0x0043_a8f0;
const TELEPORT_SAVE: u32 = 0x0043_a930;
const TELEPORT_LOAD: u32 = 0x0043_a9a0;
const TELEPORT_NEW: u32 = 0x0043_a160;
const TOPIC_SAVE_SIZE: u32 = 0x0083_f280;
const TOPIC_SAVE: u32 = 0x0083_f3e0;
const TOPIC_NEW: u32 = 0x0083_da50;
const TOPIC_LOAD: u32 = 0x0083_f680;
const TOPIC_CHECK: u32 = 0x0096_11e0;
const TOPIC_DELETE: u32 = 0x0042_5ff0;
const SET_INFO_GENERAL_TOPIC: u32 = 0x0042_dee0;
const CELL_WORLD_SPACE: u32 = 0x0054_ddd0;
const CELL_REMOVE_REFERENCE: u32 = 0x0054_ca90;
const CELL_ADD_REFERENCE: u32 = 0x0054_8230;
const CELL_IS_INTERIOR: u32 = 0x0042_5fd0;
const WORLD_PERSISTENT_CELL: u32 = 0x005f_36f0;
const REFERENCE_PERSISTS: u32 = 0x0056_53d0;
const REFERENCE_SET_SCALE: u32 = 0x0056_7490;
const LOAD_PACKAGE: u32 = 0x0085_a240;
/// The sequence names of save versions 0x15 and 0x16 (0x24 bytes each, 0xf5 of them).
const SEQUENCE_NAMES: u32 = 0x0119_77d8;
/// The local the last finished sequence's name is read into.
const SEQUENCE_TEXT: u32 = 0x104;

/// The save-load object.
#[inline]
fn save_load(e: &Engine) -> u32 {
    e.mem.u32(SAVE_LOAD_GAME)
}

/// The stream's position, in place (`00825c00`).
#[inline]
fn position(e: &Engine) -> u32 {
    word_at(e, save_load(e), POSITION)
}

/// The save's version byte, in place (`008df040`).
#[inline]
fn version(e: &Engine) -> u8 {
    e.mem.u8(save_load(e).wrapping_add(VERSION))
}

/// Whether the save uses save-game blocks, in place (`TESSaveLoadGame::UseSaveGameBlocks`,
/// `00862110`: versions 0x1f to 0x59).
#[inline]
fn uses_blocks(e: &Engine) -> bool {
    (0x1f..0x5a).contains(&version(e))
}

/// Whether the debug switch is set, in place (`00408d60` answers the value byte's address).
#[inline]
fn debug_switch(e: &Engine) -> bool {
    e.mem.u8(DEBUG_SWITCH) != 0
}

/// A form's id, in place (`0084e3a0`).
#[inline]
fn id_of(e: &Engine, form: u32) -> u32 {
    word_at(e, form, 0x0c)
}

/// Whether a form id is a created form's, in place (`00469860`, called on the data handler).
#[inline]
fn is_created(id: u32) -> bool {
    id >= 0xff00_0000
}

/// A list node's item and next, in place (`006815c0`, `00726070`), and whether the node holds
/// nothing (`008256d0`: both words zero).
#[inline]
fn node_empty(e: &Engine, node: u32) -> bool {
    word_at(e, node, 0) == 0 && word_at(e, node, 4) == 0
}

/// `size` bytes at `at` written to the stream.
fn write(e: &mut Engine, at: u32, size: u32) {
    let save = save_load(e);
    e.call(STREAM_WRITE, &args![save, at, size]);
}

/// The low `size` bytes of `value` written from a local.
fn write_value(e: &mut Engine, value: u32, size: u32) {
    e.with_stack(4, |e, local| {
        e.mem.set_u32(local.addr(), value);
        write(e, local.addr(), size);
    });
}

/// A form id written as a numeric id, from a local.
fn write_id(e: &mut Engine, id: u32) {
    e.with_stack(4, |e, local| {
        e.mem.set_u32(local.addr(), id);
        let save = save_load(e);
        e.call(SAVE_NUMERIC_ID, &args![save, local, 4u32]);
    });
}

/// `size` bytes of the stream read to `at`.
fn read(e: &mut Engine, at: u32, size: u32) {
    let save = save_load(e);
    e.call(STREAM_READ, &args![save, at, size]);
}

/// `size` bytes of the stream read into a local and answered.
fn read_value(e: &mut Engine, size: u32) -> u32 {
    e.with_stack(4, |e, local| {
        read(e, local.addr(), size);
        e.mem.u32(local.addr())
    })
}

/// A numeric id read into a local and answered.
fn read_id(e: &mut Engine) -> u32 {
    e.with_stack(4, |e, local| {
        let save = save_load(e);
        e.call(LOAD_NUMERIC_ID, &args![save, local, 4u32]);
        e.mem.u32(local.addr())
    })
}

/// An extra data cast to its class (`__RTDynamicCast` from `BSExtraData`).
fn cast_extra(e: &mut Engine, extra: u32, target: u32) -> u32 {
    e.call(
        RT_DYNAMIC_CAST,
        &args![extra, 0u32, RTTI_BS_EXTRA_DATA, target, 0u32],
    )
    .u32()
}

/// A form cast from `TESForm` (null for null, as the cast answers).
fn cast_form(e: &mut Engine, form: u32, target: u32) -> u32 {
    e.call(
        RT_DYNAMIC_CAST,
        &args![form, 0u32, RTTI_TES_FORM, target, 0u32],
    )
    .u32()
}

/// The form of an id (`004839c0`, the forms family's).
fn form_by_id(e: &mut Engine, id: u32) -> u32 {
    typed_call!(
        forms::FAMILY,
        0x0048_39c0,
        forms::tesform::form_by_id(e, id)
    )
}

/// The form of an id cast to `target`.
fn form_as(e: &mut Engine, id: u32, target: u32) -> u32 {
    let form = form_by_id(e, id);
    cast_form(e, form, target)
}

/// A size message of the save functions (`Error`, `0040fbe0`, the leaf family's): with the record
/// being saved (`004fd3e0`, the multibound family's) its id, name and flags, else the short format.
fn log_size(e: &mut Engine, written: u32, format: u32, short_format: u32, line: u32) {
    let save = save_load(e);
    let record = typed_call!(
        multibound::FAMILY,
        0x004f_d3e0,
        multibound::getters::world_space(e, Ptr::new(save))
    );
    if record != 0 {
        let id = e.mem.u32(record);
        let form = form_by_id(e, id);
        let name = e.vcall(form, NAME_SLOT, &args![]).u32();
        let flags = e.mem.u32(record.wrapping_add(5));
        typed_call!(leaf::FAMILY, 0x0040_fbe0, leaf::nothing(e); words format, written, id, name, flags, line, SOURCE_FILE_NAME);
    } else {
        typed_call!(leaf::FAMILY, 0x0040_fbe0, leaf::nothing(e); words short_format, written, line, SOURCE_FILE_NAME);
    }
}

/// A message of the loader (`ModelLoader`'s box, by address): with the record being loaded (+0x84)
/// the file, the line, its id, name, version byte and flags after `leading`; else the file, the
/// line and the save's version.
fn log_load(e: &mut Engine, format: u32, short_format: u32, leading: Option<u32>, line: u32) {
    let record = word_at(e, save_load(e), LOADING_RECORD);
    let mut words = vec![if record != 0 { format } else { short_format }];
    words.extend(leading);
    if record != 0 {
        let id = e.mem.u32(record);
        let form = form_by_id(e, id);
        let flags = e.mem.u32(record.wrapping_add(5));
        let record_version = u32::from(e.mem.u8(record.wrapping_add(9)));
        let name = e.vcall(form, NAME_SLOT, &args![]).u32();
        words.extend([SOURCE_FILE_NAME, line, id, name, record_version, flags]);
    } else {
        words.extend([SOURCE_FILE_NAME, line, u32::from(version(e))]);
    }
    e.call(LOG_MESSAGE, &words);
}

/// The nodes of a simple list from `node` on, as the savers walk them: the item of each node that
/// holds one, stopping at a null or empty node.
fn each_item(e: &mut Engine, mut node: u32, mut f: impl FnMut(&mut Engine, u32)) {
    while node != 0 && !node_empty(e, node) {
        let item = word_at(e, node, 0);
        if item != 0 {
            f(e, item);
        }
        node = word_at(e, node, 4);
    }
}

// Typed from 00422c40
/// The size `ExtraDataList::SaveGame` ([`save_game`]) writes for the change flags `flags` and the
/// reference `reference` (the exe's message calls it `GetSaveSize`): under the extra data lock, 2
/// for the count (6 more with save-game blocks), and for each extra data of a saved type its type
/// byte and payload, a 16-bit wrapping sum. A ghost (0x1f) adds its type byte only. The other
/// arms of the Xbox build are masked with 0 here and never run. With the debug switch set, the size
/// is logged (line 0x29b5).
pub fn save_size(e: &mut Engine, this: u32, flags: u32, reference: u32) -> u16 {
    take_lock(e, 0);
    let mut size: u16 = if uses_blocks(e) { 8 } else { 2 };
    let mut extra = head_of(e, this);
    while extra != 0 {
        let before = size;
        let add: u16 = match kind_of(e, extra) {
            // The persistent cell, for an actor reference (slot 0x100).
            0x0c => {
                if reference != 0 && e.vcall(reference, 0x100, &args![]).bool() {
                    4
                } else {
                    0
                }
            }
            // The package start location: a form id, a position and a rotation.
            0x18 => {
                cast_extra(e, extra, RTTI_EXTRA_PACKAGE_START_LOCATION);
                0x14
            }
            // The package: 14, and from version 0x40 for a created package its type byte and data
            // (slot 0x14c of the package).
            0x19 => {
                let package = cast_extra(e, extra, RTTI_EXTRA_PACKAGE);
                size = size.wrapping_add(0x0e);
                let mut own: u16 = 0;
                if version(e) >= 0x40 {
                    let form = word_at(e, package, EXTRA_PAYLOAD);
                    if is_created(id_of(e, form)) {
                        size = size.wrapping_add(1);
                        own = e.vcall(form, 0x14c, &args![]).u16();
                    }
                }
                own
            }
            // The run once packages: a count and an id and a byte each.
            0x1b => {
                let packages = cast_extra(e, extra, RTTI_EXTRA_RUN_ONCE_PACKS);
                size = size.wrapping_add(2);
                let list = word_at(e, packages, EXTRA_PAYLOAD);
                let count = e.call(LIST_COUNT, &args![list]).u32();
                low_half(count.wrapping_mul(5))
            }
            // The followers: an id each and a count.
            0x1d => {
                let followers = cast_extra(e, extra, RTTI_EXTRA_FOLLOWER);
                let list = word_at(e, followers, EXTRA_PAYLOAD);
                let count = e.call(LIST_COUNT, &args![list]).u32();
                low_half(count.wrapping_mul(4)).wrapping_add(2)
            }
            // The lock, with change flag 0x1000: its level, key id and flags.
            0x2a if flags & 0x1000 != 0 => 6,
            // The teleport, with change flag 0x20000: its data's size.
            0x2b if flags & 0x2_0000 != 0 => {
                let teleport = cast_extra(e, extra, RTTI_EXTRA_TELEPORT);
                let data = word_at(e, teleport, EXTRA_PAYLOAD);
                e.call(TELEPORT_SAVE_SIZE, &args![data]).u16()
            }
            // The item dropper and the head track target: an id.
            0x39 | 0x46 => 4,
            // The last finished sequence, with change flag 0x10000000: a length byte and the name.
            0x41 if flags & 0x1000_0000 != 0 => {
                let sequence = cast_extra(e, extra, RTTI_EXTRA_LAST_FINISHED_SEQUENCE);
                size = size.wrapping_add(1);
                let name = word_at(e, sequence, EXTRA_PAYLOAD);
                low_half(typed_call!(
                    forms::FAMILY,
                    0x0044_a670,
                    forms::debug_string::text_length(e, name)
                ))
            }
            // The info general topic, when it holds one: its size.
            0x4d => {
                let topic = cast_extra(e, extra, RTTI_EXTRA_INFO_GENERAL_TOPIC);
                let data = word_at(e, topic, EXTRA_PAYLOAD);
                if data != 0 {
                    e.call(TOPIC_SAVE_SIZE, &args![data]).u16()
                } else {
                    0
                }
            }
            // No rumors: a byte.
            0x4e => 1,
            _ => 0,
        };
        size = size.wrapping_add(add);
        // The type byte: for an arm that added, and for a ghost (the type read again).
        if before != size || kind_of(e, extra) == 0x1f {
            size = size.wrapping_add(1);
        }
        extra = next_of(e, extra);
    }
    if debug_switch(e) {
        log_size(e, u32::from(size), SIZE_FORMAT, SIZE_SHORT_FORMAT, 0x29b5);
    }
    give_lock(e);
    size
}

// Typed from 004235e0
/// `ExtraDataList::SaveGame` (Xbox PDB): the list written to the save-load stream for the change
/// flags `flags` and the reference `reference`. With save-game blocks the tag and a 16-bit block
/// size first (filled in at the end, with a message when it does not fit); then, under the extra
/// data lock, a 16-bit count (filled in at the end) and for each extra data of a saved type its type
/// byte and payload, counted when the stream moved. The arms are [`save_size`]'s. With the debug
/// switch set, the bytes written are logged (line 0x2c1f).
pub fn save_game(e: &mut Engine, this: u32, flags: u32, reference: u32) {
    let mut block_size_at = 0u32;
    // The start is taken again when the debug switch is set, as the exe does.
    let start = position(e);
    let start = if debug_switch(e) { position(e) } else { start };
    if uses_blocks(e) {
        write_value(e, BLOCK_TAG, 4);
        block_size_at = position(e);
        write_value(e, 0, 2);
    }
    take_lock(e, 0);
    let mut count: u16 = 0;
    let count_at = position(e);
    write_value(e, 0, 2);
    let mut extra = head_of(e, this);
    while extra != 0 {
        let kind = kind_of(e, extra);
        let type_byte = u32::from(kind);
        let before = position(e);
        match kind {
            // The persistent cell, for an actor reference: the id of its cell's world space.
            0x0c => {
                if reference != 0 && e.vcall(reference, 0x100, &args![]).bool() {
                    write_value(e, type_byte, 1);
                    let cell = cast_extra(e, extra, RTTI_EXTRA_PERSISTENT_CELL);
                    let data = word_at(e, cell, EXTRA_PAYLOAD);
                    let id = if data != 0 {
                        let world = e.call(CELL_WORLD_SPACE, &args![data]).u32();
                        id_of(e, world)
                    } else {
                        0
                    };
                    write_id(e, id);
                }
            }
            // The package start location: the location's id (a message, with the reference's
            // name, when there is none), 12 bytes of position and the rotation.
            0x18 => {
                write_value(e, type_byte, 1);
                let location = cast_extra(e, extra, RTTI_EXTRA_PACKAGE_START_LOCATION);
                let form = word_at(e, location, EXTRA_PAYLOAD);
                let id = if form == 0 {
                    let name = e.vcall(reference, NAME_SLOT, &args![]).u32();
                    let reference_id = id_of(e, reference);
                    e.call(LOG_MESSAGE, &args![NO_START_LOCATION, reference_id, name]);
                    0
                } else {
                    id_of(e, form)
                };
                write_id(e, id);
                write(e, location.wrapping_add(0x10), 0x0c);
                write(e, location.wrapping_add(0x1c), 4);
            }
            // The package: its id, the word at +0x10, the target's id, two bytes, and from version
            // 0x40 for a created package its type byte and data (slot 0x150).
            0x19 => {
                write_value(e, type_byte, 1);
                let package = cast_extra(e, extra, RTTI_EXTRA_PACKAGE);
                let form = word_at(e, package, EXTRA_PAYLOAD);
                let id = id_of(e, form);
                let target = word_at(e, package, 0x14);
                let target_id = if target != 0 { id_of(e, target) } else { 0 };
                write_id(e, id);
                write(e, package.wrapping_add(0x10), 4);
                write_id(e, target_id);
                write(e, package.wrapping_add(0x18), 1);
                write(e, package.wrapping_add(0x19), 1);
                if version(e) >= 0x40 && is_created(id) {
                    // The package's type, in place (`0041ca90`'s signed byte at +0x20).
                    let package_type = u32::from(e.mem.u8(form.wrapping_add(0x20)));
                    write_value(e, package_type, 1);
                    e.vcall(form, 0x150, &args![]);
                }
            }
            // The run once packages: the count (filled in after) and each package's id and byte.
            0x1b => {
                write_value(e, type_byte, 1);
                let items_at = position(e);
                write_value(e, 0, 2);
                let packages = cast_extra(e, extra, RTTI_EXTRA_RUN_ONCE_PACKS);
                let mut items: u16 = 0;
                let first = word_at(e, packages, EXTRA_PAYLOAD);
                each_item(e, first, |e, item| {
                    let package = word_at(e, item, 0);
                    let id = id_of(e, package);
                    write_id(e, id);
                    write(e, item.wrapping_add(4), 1);
                    items = items.wrapping_add(1);
                });
                e.mem.set_u16(items_at, items);
            }
            // The followers: the count (filled in after) and each follower's id.
            0x1d => {
                write_value(e, type_byte, 1);
                let followers = cast_extra(e, extra, RTTI_EXTRA_FOLLOWER);
                let items_at = position(e);
                write_value(e, 0, 2);
                let mut items: u16 = 0;
                let first = word_at(e, followers, EXTRA_PAYLOAD);
                each_item(e, first, |e, follower| {
                    let id = id_of(e, follower);
                    write_id(e, id);
                    items = items.wrapping_add(1);
                });
                e.mem.set_u16(items_at, items);
            }
            // The ghost: the type byte alone.
            0x1f => write_value(e, type_byte, 1),
            // The lock, with change flag 0x1000: its level, key id and flags byte.
            0x2a if flags & 0x1000 != 0 => {
                write_value(e, type_byte, 1);
                let lock = word_at(e, extra, EXTRA_PAYLOAD);
                let key = word_at(e, lock, 4);
                let key_id = if key != 0 { id_of(e, key) } else { 0 };
                write(e, lock, 1);
                write_id(e, key_id);
                write(e, lock.wrapping_add(8), 1);
            }
            // The teleport, with change flag 0x20000: `DoorTeleportData::SaveGame`.
            0x2b if flags & 0x2_0000 != 0 => {
                write_value(e, type_byte, 1);
                let teleport = cast_extra(e, extra, RTTI_EXTRA_TELEPORT);
                let data = word_at(e, teleport, EXTRA_PAYLOAD);
                e.call(TELEPORT_SAVE, &args![data]);
            }
            // The item dropper and the head track target: the form's id (0 for none).
            0x39 | 0x46 => {
                write_value(e, type_byte, 1);
                let class = if kind == 0x39 {
                    RTTI_EXTRA_ITEM_DROPPER
                } else {
                    RTTI_EXTRA_HEAD_TRACK_TARGET
                };
                let holder = cast_extra(e, extra, class);
                let form = word_at(e, holder, EXTRA_PAYLOAD);
                let id = if form != 0 { id_of(e, form) } else { 0 };
                write_id(e, id);
            }
            // The last finished sequence, with change flag 0x10000000: a length byte and the name.
            0x41 if flags & 0x1000_0000 != 0 => {
                write_value(e, type_byte, 1);
                let sequence = cast_extra(e, extra, RTTI_EXTRA_LAST_FINISHED_SEQUENCE);
                let name = word_at(e, sequence, EXTRA_PAYLOAD);
                let length = typed_call!(
                    forms::FAMILY,
                    0x0044_a670,
                    forms::debug_string::text_length(e, name)
                ) & 0xff;
                write_value(e, length, 1);
                write(e, name, length);
            }
            // The info general topic, when it holds one: its data.
            0x4d => {
                let topic = cast_extra(e, extra, RTTI_EXTRA_INFO_GENERAL_TOPIC);
                let data = word_at(e, topic, EXTRA_PAYLOAD);
                if data != 0 {
                    write_value(e, type_byte, 1);
                    e.call(TOPIC_SAVE, &args![data]);
                }
            }
            // No rumors: its byte.
            0x4e => {
                write_value(e, type_byte, 1);
                let rumors = cast_extra(e, extra, RTTI_EXTRA_NO_RUMORS);
                write(e, rumors.wrapping_add(EXTRA_PAYLOAD), 1);
            }
            _ => {}
        }
        if before != position(e) {
            count = count.wrapping_add(1);
        }
        extra = next_of(e, extra);
    }
    e.mem.set_u16(count_at, count);
    if debug_switch(e) {
        let written = position(e).wrapping_sub(start);
        log_size(e, written, SAVE_FORMAT, SAVE_SHORT_FORMAT, 0x2c1f);
    }
    if uses_blocks(e) {
        let end = position(e);
        if end > block_size_at.wrapping_add(0xffff) {
            e.call(
                LOG_MESSAGE,
                &args![BLOCK_TOO_LARGE, SOURCE_FILE_NAME, 0x2c1fu32],
            );
        }
        e.mem
            .set_u16(block_size_at, low_half(end.wrapping_sub(block_size_at)));
    }
    give_lock(e);
}

// Typed from 00424960
/// `ExtraDataList::LoadGame` (Xbox PDB): the list [`save_game`] wrote read back from the stream for
/// the change flags `flags` and the reference `reference` (the third word unread). With save-game
/// blocks the tag (a message when it is not there) and the block size first; then the count and
/// each extra data's type byte and payload, set through the list's setters. The types 0x2e, 0x45,
/// 0x90 and 0x91 read nothing, nor do the types the Xbox build saved under masks that are 0 here; a
/// type outside the table is reported. At the end the position is checked against the block.
pub fn load_game(e: &mut Engine, this: u32, flags: u32, _unused: u32, reference: u32) {
    let mut block_size = 0u32;
    let mut block_start = 0u32;
    if uses_blocks(e) {
        if read_value(e, 4) != BLOCK_TAG {
            log_load(e, BAD_TAG_FORMAT, BAD_TAG_SHORT_FORMAT, None, 0x2c2c);
        }
        block_start = position(e);
        block_size = read_value(e, 2) & 0xffff;
    }
    // The reference as an actor; the answer is not used.
    e.call(
        RT_DYNAMIC_CAST,
        &args![reference, 0u32, RTTI_TES_OBJECT_REFR, RTTI_ACTOR, 0u32],
    );
    let count = read_value(e, 2) & 0xffff;
    for _ in 0..count {
        let kind = low_byte(read_value(e, 1));
        match kind {
            0x0c => load_persistent_cell(e, this, reference),
            // The package start location: a world location (the location as a form, 12 bytes of
            // position, the rotation); set when the form is a world space or a cell.
            0x18 => e.with_stack(0x14, |e, record| {
                // `WORLD_LOCATION`'s constructor (`006d5320`) only answers itself: in place.
                let record = record.addr();
                let id = read_id(e);
                read(e, record.wrapping_add(4), 0x0c);
                read(e, record.wrapping_add(0x10), 4);
                let form = form_by_id(e, id);
                e.mem.set_u32(record, form);
                let world = cast_form(e, form, RTTI_TES_WORLD_SPACE);
                let cell = cast_form(e, form, RTTI_TES_OBJECT_CELL);
                if world != 0 || cell != 0 {
                    let rotation =
                        f32::from_bits(x87_quiet_bits(e.mem.u32(record.wrapping_add(0x10))));
                    typed_call!(
                        FAMILY,
                        0x0041_ad00,
                        setters::set_package_start_location(
                            e,
                            this,
                            world,
                            cell,
                            record.wrapping_add(4),
                            rotation
                        )
                    );
                }
            }),
            // The package: its id, a word, the target's id, two bytes; from version 0x40 a created
            // package is loaded with its type byte (slot 0x154), else the id's package is used.
            0x19 => {
                let id = read_id(e);
                let word = read_value(e, 4);
                let target_id = read_id(e);
                let first = low_byte(read_value(e, 1));
                let second = low_byte(read_value(e, 1));
                let mut package = 0;
                if version(e) >= 0x40 && is_created(id) {
                    let package_type = read_value(e, 1);
                    let save = save_load(e);
                    package = e.call(LOAD_PACKAGE, &args![save, id, package_type]).u32();
                    e.vcall(package, 0x154, &args![]);
                }
                if package == 0 {
                    package = form_as(e, id, RTTI_TES_PACKAGE);
                }
                if package != 0 {
                    typed_call!(
                        FAMILY,
                        0x0041_c930,
                        packages::set_package(
                            e,
                            this,
                            package,
                            word as i32,
                            target_id,
                            first,
                            second,
                            0
                        )
                    );
                }
            }
            // The run once packages: each package's id and byte.
            0x1b => {
                let n = read_value(e, 2) & 0xffff;
                for _ in 0..n {
                    let id = read_id(e);
                    let byte = low_byte(read_value(e, 1));
                    let package = form_as(e, id, RTTI_TES_PACKAGE);
                    if package != 0 {
                        typed_call!(
                            FAMILY,
                            0x0041_d700,
                            packages::add_run_once_package(e, this, package, byte)
                        );
                    }
                }
            }
            // The followers: the ids go in as they are.
            0x1d => {
                let n = read_value(e, 2) & 0xffff;
                for _ in 0..n {
                    let id = read_id(e);
                    typed_call!(FAMILY, 0x0042_2480, followers::add_follower(e, this, id));
                }
            }
            0x1f => {
                typed_call!(FAMILY, 0x0041_a960, setters::set_ghost(e, this, true));
            }
            // The lock, with change flag 0x1000: a new lock record, and from version 0x15 its level,
            // key and flags (an unused id before version 0x1a).
            0x2a if flags & 0x1000 != 0 => {
                let lock = new_object(e, 0x14, |e, block| {
                    typed_call!(
                        cell_refs::FAMILY,
                        0x0041_1b00,
                        cell_refs::extra_list::lock_data_new(e, block)
                    )
                });
                if version(e) >= 0x15 {
                    read(e, lock, 1);
                    let key_id = read_id(e);
                    if version(e) < 0x1a {
                        read_id(e);
                    }
                    read(e, lock.wrapping_add(8), 1);
                    if key_id != 0 {
                        let key = form_as(e, key_id, RTTI_TES_KEY);
                        e.mem.set_u32(lock.wrapping_add(4), key);
                    }
                }
                typed_call!(FAMILY, 0x0041_9050, access::set_lock_ptr(e, this, lock));
            }
            // The teleport, with change flag 0x20000: new data loaded by its class.
            0x2b if flags & 0x2_0000 != 0 => {
                let data = new_object(e, 0x20, |e, block| {
                    e.call(TELEPORT_NEW, &args![block]).u32()
                });
                e.call(TELEPORT_LOAD, &args![data]);
                typed_call!(FAMILY, 0x0041_9120, access::set_teleport(e, this, data));
            }
            // The scale: a float saved before version 0x43 under change flag 0x10.
            0x30 => {
                if version(e) < 0x43 && flags & 0x10 != 0 {
                    let scale = f32::from_bits(x87_quiet_bits(read_value(e, 4)));
                    if reference == 0 {
                        typed_call!(FAMILY, 0x0041_9fb0, access::set_scale(e, this, scale));
                    } else {
                        e.call(REFERENCE_SET_SCALE, &args![reference, scale]);
                    }
                }
            }
            // The item dropper: a dropped item's id.
            0x39 => {
                let id = read_id(e);
                if id != 0 {
                    typed_call!(FAMILY, 0x0041_de40, links::add_dropped_item(e, this, id));
                }
            }
            // The dropped item list: a count byte and the ids.
            0x3a => {
                let n = read_value(e, 1);
                for _ in 0..n {
                    let id = read_id(e);
                    if id != 0 {
                        typed_call!(
                            FAMILY,
                            0x0041_e000,
                            links::add_to_dropped_items(e, this, id)
                        );
                    }
                }
            }
            // The last finished sequence, with change flag 0x10000000: a table index in versions
            // 0x15 and 0x16, a length-prefixed name in the others.
            0x41 if flags & 0x1000_0000 != 0 => load_sequence(e, this),
            // The head track target: the id, through the item dropper's setter (`00419dc0`).
            0x46 => {
                let id = read_id(e);
                if id != 0 {
                    typed_call!(FAMILY, 0x0041_9dc0, access::set_item_dropper(e, this, id));
                }
            }
            // The info general topic: new topic data loaded for the reference, set when it checks
            // out, else deleted.
            0x4d => {
                let topic = new_object(e, 0x2c, |e, block| e.call(TOPIC_NEW, &args![block]).u32());
                e.call(TOPIC_LOAD, &args![topic, reference]);
                if e.call(TOPIC_CHECK, &args![topic]).u32() != 0 {
                    e.call(SET_INFO_GENERAL_TOPIC, &args![this, topic]);
                } else if topic != 0 {
                    e.call(TOPIC_DELETE, &args![topic, 1u32]);
                }
            }
            // No rumors: its byte.
            0x4e => {
                let flag = low_byte(read_value(e, 1));
                typed_call!(FAMILY, 0x0042_1600, followers::set_no_rumors(e, this, flag));
            }
            // No payload, and the types saved under masks that are 0 here (their flags-guarded
            // arms above too).
            0x2e
            | 0x45
            | 0x90
            | 0x91
            | 0x0d
            | 0x12
            | 0x16
            | 0x17
            | 0x1a
            | 0x1c
            | 0x21..=0x28
            | 0x2a
            | 0x2b
            | 0x2c
            | 0x2f
            | 0x32
            | 0x33
            | 0x3e
            | 0x3f
            | 0x41
            | 0x49
            | 0x4a
            | 0x8d => {}
            _ => {
                e.call(LOG_MESSAGE, &args![UNKNOWN_TYPE_FORMAT, u32::from(kind)]);
            }
        }
    }
    if uses_blocks(e) {
        let at = position(e);
        let expected = block_size.wrapping_add(block_start);
        if at > expected {
            let over = at.wrapping_sub(expected);
            log_load(
                e,
                OVERREAD_FORMAT,
                OVERREAD_SHORT_FORMAT,
                Some(over),
                0x2ee9,
            );
        } else if at < expected {
            let under = expected.wrapping_sub(at);
            log_load(
                e,
                UNDERREAD_FORMAT,
                UNDERREAD_SHORT_FORMAT,
                Some(under),
                0x2ee9,
            );
        }
    }
}

/// [`load_game`]'s persistent cell (type 0x0c), for an actor reference: the reference leaves the
/// cell the list names (set to none); then for a reference that persists, the id's world space's
/// persistent cell is set and the reference added to it, and taken out of the interior cell it is
/// in.
fn load_persistent_cell(e: &mut Engine, this: u32, reference: u32) {
    if reference == 0 || !e.vcall(reference, 0x100, &args![]).bool() {
        return;
    }
    let cell = typed_call!(FAMILY, 0x0041_d460, packages::persistent_cell(e, this));
    if cell != 0 {
        typed_call!(
            FAMILY,
            0x0041_d390,
            packages::set_persistent_cell(e, this, 0)
        );
        e.call(CELL_REMOVE_REFERENCE, &args![cell, reference]);
    }
    let id = read_id(e);
    if !e.call(REFERENCE_PERSISTS, &args![reference]).bool() {
        return;
    }
    let world = form_as(e, id, RTTI_TES_WORLD_SPACE);
    if world == 0 {
        return;
    }
    let persistent = e.call(WORLD_PERSISTENT_CELL, &args![world]).u32();
    typed_call!(
        FAMILY,
        0x0041_d390,
        packages::set_persistent_cell(e, this, persistent)
    );
    let persistent = e.call(WORLD_PERSISTENT_CELL, &args![world]).u32();
    e.call(CELL_ADD_REFERENCE, &args![persistent, reference, 0u32]);
    // The reference's parent cell, in place (`008d6f30`): read again after the interior test.
    let parent = word_at(e, reference, 0x40);
    if parent != 0 && e.call(CELL_IS_INTERIOR, &args![parent]).bool() {
        let parent = word_at(e, reference, 0x40);
        e.call(CELL_REMOVE_REFERENCE, &args![parent, reference]);
    }
}

/// [`load_game`]'s last finished sequence (type 0x41): in versions 0x15 and 0x16 an index into the
/// sequence names (none from 0xf5 on, signed); in the others a length byte and the name, read into
/// a cleared local of 0x104 bytes.
fn load_sequence(e: &mut Engine, this: u32) {
    let v = version(e);
    if (0x15..0x17).contains(&v) {
        let index = read_value(e, 4);
        if (index as i32) < 0xf5 {
            let name = e
                .mem
                .u32(SEQUENCE_NAMES.wrapping_add(index.wrapping_mul(0x24)));
            typed_call!(
                FAMILY,
                0x0042_2850,
                followers::set_last_finished_sequence(e, this, name)
            );
        }
    }
    let v = version(e);
    if !(0x15..0x17).contains(&v) {
        let length = read_value(e, 1);
        e.with_stack(SEQUENCE_TEXT, |e, text| {
            let text = text.addr();
            zero_bytes(e, text, SEQUENCE_TEXT);
            read(e, text, length);
            typed_call!(
                FAMILY,
                0x0042_2850,
                followers::set_last_finished_sequence(e, this, text)
            );
        });
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn a_created_id_starts_at_ff000000() {
        assert!(!super::is_created(0xfeff_ffff));
        assert!(super::is_created(0xff00_0000));
    }
}
