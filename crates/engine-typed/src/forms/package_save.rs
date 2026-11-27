//! `TESPackage`'s save game functions (Xbox PDB; the unit `fallout shared/package`, `00678fd0` to
//! `00679b10`): the save size, the old save and load (`SaveGame`, `LoadGame` without a buffer),
//! the location's and target's step after a load, the form flags' load and clear hooks, the
//! buffered `SaveGame`, `LoadGame` and `InitLoadGame`, and the flags' test of the buffer for a
//! package from a file.
//!
//! **Native.** The one-line bodies the exe calls are done in place, each the body it holds
//! (checked in the exe): the save/load object's "use save game blocks" test (`00862110`: its
//! version byte at +0x80 from 0x1f to 0x59), its stream position (`00825c00`, +0x14), the form
//! being loaded or saved (`004fd3c0` +0x84, `004fd3e0` +0x88), its version byte (`008df040`), the
//! save size logging flag (`00408d60` on its fixed object: the byte at +4), the created-id test
//! (`00469860`), the form's id (`0084e3a0`), `PackageLocation`'s and `PackageTarget`'s
//! constructors (`0067f030`, `0067ff70`) and the form's empty flag clear (`004534f0`). `TESForm`'s
//! old save and load of a buffer, its `LoadGame` and its lookup by id are typed calls; the
//! save buffer's save kind and flags reads (`00428110`, `0042ce30`) and their test (`004280f0`) are
//! done in place. What stays a call by address: the allocator, the save/load object's
//! raw save and load, the save buffer's write and read, the location's and target's save, load,
//! save size and after-load functions, the logger (which YUP and xNVSE may replace); the data
//! object's save, load and init-load go through its slots and the form's name through its slot.
//! No floats.

use super::{tesform, FAMILY};
use engine_core::typed::leaf;
use engine_core::{args, typed_call, Engine};

/// The package's procedure type (saved as a word), flags (the 12 bytes saved from +0x1c), data
/// object, location and target; the form's id.
const PROCEDURE: u32 = 0x18;
const SAVED_BYTES: u32 = 0x1c;
const PACK_FLAGS: u32 = 0x1c;
const DATA: u32 = 0x28;
const LOCATION: u32 = 0x2c;
const TARGET: u32 = 0x30;
const FORM_ID: u32 = 0xc;
const CREATED_IDS: u32 = 0xff00_0000;
/// The save/load object (a global), its version byte, stream position and current forms.
const SAVE_LOAD_GAME: u32 = 0x011d_e45c;
const VERSION: u32 = 0x80;
const POSITION: u32 = 0x14;
const LOADING_FORM: u32 = 0x84;
const SAVING_FORM: u32 = 0x88;
/// The save size logging flag's object (its byte at +4).
const SAVE_SIZE_LOG: u32 = 0x011d_e4e8;
/// The raw save and load (`this` = the save/load object: address, size), the logger.
const SAVE_BYTES: u32 = 0x0085_79b0;
const LOAD_BYTES: u32 = 0x0085_79e0;
const LOG: u32 = 0x005b_5e40;
/// The save buffer's write (address, size, 0) and read (address, size).
const BUFFER_SAVE: u32 = 0x0086_5e50;
const BUFFER_LOAD: u32 = 0x0086_4980;
/// `operator new` and the location's and target's sizes.
const OPERATOR_NEW: u32 = 0x0040_1000;
const LOCATION_SIZE: u32 = 0xc;
const TARGET_SIZE: u32 = 0x10;
/// `PackageLocation`'s and `PackageTarget`'s save size, old save, old load, after-load step,
/// buffered save, load and init-load.
const LOCATION_SAVE_SIZE: u32 = 0x0067_f990;
const LOCATION_SAVE_OLD: u32 = 0x0067_fa00;
const LOCATION_LOAD_OLD: u32 = 0x0067_faa0;
const LOCATION_AFTER_LOAD: u32 = 0x0067_fb20;
const LOCATION_SAVE: u32 = 0x0067_fb60;
const LOCATION_LOAD: u32 = 0x0067_fbe0;
const LOCATION_INIT_LOAD: u32 = 0x0067_fc50;
const TARGET_SAVE_SIZE: u32 = 0x0068_0520;
const TARGET_SAVE_OLD: u32 = 0x0068_0590;
const TARGET_LOAD_OLD: u32 = 0x0068_0640;
const TARGET_AFTER_LOAD: u32 = 0x0068_06e0;
const TARGET_SAVE: u32 = 0x0068_0720;
const TARGET_LOAD: u32 = 0x0068_07b0;
const TARGET_INIT_LOAD: u32 = 0x0068_0830;
/// The data object's slots: `SaveGame`, `LoadGame`, `InitLoadGame` (each the buffer); the form's
/// name slot.
const SLOT_DATA_SAVE: u32 = 0x14;
const SLOT_DATA_LOAD: u32 = 0x18;
const SLOT_DATA_INIT_LOAD: u32 = 0x1c;
const SLOT_FORM_NAME: u32 = 0x130;
/// The `BLOK` tag of a save game block.
const BLOCK_TAG: u32 = 0x424c_4f4b;
/// The form flags the package keeps in its own flags, and those flags' bits.
const FORM_FLAG_HIGH: u32 = 0x8000_0000;
const FORM_FLAG_NEXT: u32 = 0x4000_0000;
const PACK_FLAG_FOR_HIGH: u32 = 0x8000;
const PACK_FLAG_FOR_NEXT: u32 = 0x1_0000;
/// The package unit's file name (the messages' `%s`).
const PACKAGE_SOURCE_FILE: u32 = 0x0106_85dc;
/// A save buffer's save kind and flags word.
const BUFFER_SAVE_KIND: u32 = 0x17;
const BUFFER_FLAGS: u32 = 0x2c;

fn at(base: u32, offset: u32) -> u32 {
    base.wrapping_add(offset)
}

fn save_load_game(e: &Engine) -> u32 {
    e.mem.u32(SAVE_LOAD_GAME)
}

/// `UseSaveGameBlocks` (`00862110`): the global save/load object's version byte from 0x1f to 0x59.
fn uses_blocks(e: &Engine) -> bool {
    (0x1f..0x5a).contains(&e.mem.u8(at(save_load_game(e), VERSION)))
}

fn position(e: &Engine, save: u32) -> u32 {
    e.mem.u32(at(save, POSITION))
}

fn size_logging(e: &Engine) -> bool {
    e.mem.u8(at(SAVE_SIZE_LOG, 4)) != 0
}

/// A package with a created id (`00469860`).
fn has_created_id(e: &Engine, this: u32) -> bool {
    e.mem.u32(at(this, FORM_ID)) >= CREATED_IDS
}

/// A new location (`0067f030` in place: the byte 3 and two zero words), or 0.
fn new_location(e: &mut Engine) -> u32 {
    let block = e.call(OPERATOR_NEW, &args![LOCATION_SIZE]).u32();
    if block != 0 {
        e.mem.set_u8(block, 3);
        e.mem.set_u32(at(block, 4), 0);
        e.mem.set_u32(at(block, 8), 0);
    }
    block
}

/// A new target (`0067ff70` in place: the byte 2, two zero words and 0.0), or 0.
fn new_target(e: &mut Engine) -> u32 {
    let block = e.call(OPERATOR_NEW, &args![TARGET_SIZE]).u32();
    if block != 0 {
        e.mem.set_u8(block, 2);
        for offset in [4, 8, 0xc] {
            e.mem.set_u32(at(block, offset), 0);
        }
    }
    block
}

/// The form a save/load record names (`current`: its first word is the form id).
fn form_of_record(e: &mut Engine, current: u32) -> u32 {
    let id = e.mem.u32(current);
    typed_call!(FAMILY, 0x0048_39c0, tesform::form_by_id(e, id))
}

/// The size report (`Error`, empty in the release exe) with the form being saved, or without.
fn report_size(e: &mut Engine, saved: u32, current: u32, line: u32, formats: (u32, u32)) {
    if current != 0 {
        let form = form_of_record(e, current);
        let id = e.mem.u32(current);
        let name = e.vcall(form, SLOT_FORM_NAME, &args![]).u32();
        let flags = e.mem.u32(at(current, 5));
        typed_call!(leaf::FAMILY, 0x0040_fbe0, leaf::nothing(e); words formats.0, saved, id, name, flags, line, PACKAGE_SOURCE_FILE);
    } else {
        typed_call!(leaf::FAMILY, 0x0040_fbe0, leaf::nothing(e); words formats.1, saved, line, PACKAGE_SOURCE_FILE);
    }
}

// Typed from 00678fd0
/// The package's save size (16 bits): 6 with save game blocks, 13 for the form data, the location's
/// and target's save sizes, 4 for the procedure word; reported when the size logging flag is set.
pub fn save_size(e: &mut Engine, this: u32) -> u16 {
    let save = save_load_game(e);
    let mut size: u16 = if uses_blocks(e) { 6 } else { 0 };
    size = size.wrapping_add(0xd);
    let location = e.mem.u32(at(this, LOCATION));
    if location != 0 {
        size = size.wrapping_add(e.call(LOCATION_SAVE_SIZE, &args![location]).u16());
    }
    let target = e.mem.u32(at(this, TARGET));
    if target != 0 {
        size = size.wrapping_add(e.call(TARGET_SAVE_SIZE, &args![target]).u16());
    }
    size = size.wrapping_add(4);
    if size_logging(e) {
        let current = e.mem.u32(at(save, SAVING_FORM));
        report_size(
            e,
            u32::from(size),
            current,
            0x1d51,
            (0x0101_2cb0, 0x0101_2c78),
        );
    }
    size
}

// Typed from 00679120
/// The package's old save: with save game blocks the `BLOK` tag and a 16-bit length patched at the
/// end, the 12 bytes at +0x1c, the parts byte (1 location, 2 target) through `TESForm`'s old save,
/// the location's and target's old saves and the procedure word; the bytes reported with the
/// logging flag, a block over 0xffff bytes logged.
pub fn save_old(e: &mut Engine, this: u32) {
    let save = save_load_game(e);
    let start = position(e, save);
    e.with_stack(0x10, |e, locals| {
        let locals = locals.addr();
        let (tag, length, parts) = (locals, at(locals, 4), at(locals, 8));
        let mut block = 0u32;
        e.mem.set_u16(length, 0);
        if uses_blocks(e) {
            e.mem.set_u32(tag, BLOCK_TAG);
            e.call(SAVE_BYTES, &args![save, tag, 4u32]);
            block = position(e, save);
            e.call(SAVE_BYTES, &args![save, length, 2u32]);
        }
        e.call(SAVE_BYTES, &args![save, at(this, SAVED_BYTES), 0xcu32]);
        let location = e.mem.u32(at(this, LOCATION));
        let target = e.mem.u32(at(this, TARGET));
        e.mem
            .set_u8(parts, u8::from(location != 0) | u8::from(target != 0) << 1);
        typed_call!(
            FAMILY,
            0x0048_4ce0,
            tesform::save_game_data_old(e, this, parts, 1)
        );
        let location = e.mem.u32(at(this, LOCATION));
        if location != 0 {
            e.call(LOCATION_SAVE_OLD, &args![location]);
        }
        let target = e.mem.u32(at(this, TARGET));
        if target != 0 {
            e.call(TARGET_SAVE_OLD, &args![target]);
        }
        e.call(SAVE_BYTES, &args![save, at(this, PROCEDURE), 4u32]);
        if size_logging(e) {
            let end = position(e, save);
            let current = e.mem.u32(at(save, SAVING_FORM));
            report_size(
                e,
                end.wrapping_sub(start),
                current,
                0x1d6e,
                (0x0101_53a0, 0x0101_536c),
            );
        }
        if uses_blocks(e) {
            let end = position(e, save);
            if end > block.wrapping_add(0xffff) {
                e.call(LOG, &args![0x0101_5318u32, PACKAGE_SOURCE_FILE, 0x1d6eu32]);
            }
            // The low half of the length (a block over 0xffff bytes was logged above).
            let length = end.wrapping_sub(block).to_le_bytes();
            e.mem
                .set_u16(block, u16::from_le_bytes([length[0], length[1]]));
        }
    });
}

/// A `LoadGame` message of the logger: with the form being loaded its id, name, version and flags
/// from the record; without, the save/load object's version.
fn log_load(
    e: &mut Engine,
    (save, current, form): (u32, u32, u32),
    (with_form, without): (u32, u32),
    amount: Option<u32>,
    line: u32,
) {
    let mut words: Vec<u32> = Vec::with_capacity(8);
    if current != 0 {
        let id = e.mem.u32(current);
        let name = e.vcall(form, SLOT_FORM_NAME, &args![]).u32();
        let version = u32::from(e.mem.u8(at(current, 9)));
        let flags = e.mem.u32(at(current, 5));
        words.push(with_form);
        words.extend(amount);
        words.extend([PACKAGE_SOURCE_FILE, line, id, name, version, flags]);
    } else {
        let version = u32::from(e.mem.u8(at(save, VERSION)));
        words.push(without);
        words.extend(amount);
        words.extend([PACKAGE_SOURCE_FILE, line, version]);
    }
    e.call(LOG, &words);
}

/// The form being loaded, as the logger's messages name it: (the record, its form).
fn loading_form(e: &mut Engine, save: u32) -> (u32, u32) {
    let current = e.mem.u32(at(save, LOADING_FORM));
    let form = if current != 0 {
        form_of_record(e, current)
    } else {
        0
    };
    (current, form)
}

// Typed from 00679340
/// `TESPackage::LoadGame` (Xbox PDB), the old form: what [`save_old`] wrote. A wrong `BLOK` tag is
/// logged; the block's length is checked against the bytes read (overrun and underrun logged).
pub fn load_old(e: &mut Engine, this: u32) {
    let save = save_load_game(e);
    e.with_stack(0x10, |e, locals| {
        let locals = locals.addr();
        let (tag, length, parts) = (locals, at(locals, 4), at(locals, 8));
        let mut block = 0u32;
        e.mem.set_u16(length, 0);
        if uses_blocks(e) {
            e.call(LOAD_BYTES, &args![save, tag, 4u32]);
            if e.mem.u32(tag) != BLOCK_TAG {
                let (current, form) = loading_form(e, save);
                log_load(
                    e,
                    (save, current, form),
                    (0x0101_5718, 0x0101_56a8),
                    None,
                    0x1d74,
                );
            }
            block = position(e, save);
            e.call(LOAD_BYTES, &args![save, length, 2u32]);
        }
        e.call(LOAD_BYTES, &args![save, at(this, SAVED_BYTES), 0xcu32]);
        typed_call!(
            FAMILY,
            0x0048_4d00,
            tesform::load_game_data_old(e, this, parts, 1)
        );
        let flags = e.mem.u8(parts);
        if flags & 1 != 0 {
            let made = new_location(e);
            e.mem.set_u32(at(this, LOCATION), made);
            e.call(LOCATION_LOAD_OLD, &args![made]);
        }
        if flags & 2 != 0 {
            let made = new_target(e);
            e.mem.set_u32(at(this, TARGET), made);
            e.call(TARGET_LOAD_OLD, &args![made]);
        }
        e.call(LOAD_BYTES, &args![save, at(this, PROCEDURE), 4u32]);
        if uses_blocks(e) {
            let read = position(e, save);
            let (current, form) = loading_form(e, save);
            let expected = u32::from(e.mem.u16(length)).wrapping_add(block);
            let context = (save, current, form);
            if read > expected {
                let amount = Some(read.wrapping_sub(expected));
                log_load(e, context, (0x0101_5588, 0x0101_54a0), amount, 0x1d8b);
            } else if read < expected {
                let amount = Some(expected.wrapping_sub(read));
                log_load(e, context, (0x0101_5500, 0x0101_5440), amount, 0x1d8b);
            }
        }
    });
}

// Typed from 006796c0
/// The location's and target's step after a load, for those the package has.
pub fn after_load(e: &mut Engine, this: u32) {
    let location = e.mem.u32(at(this, LOCATION));
    if location != 0 {
        e.call(LOCATION_AFTER_LOAD, &args![location]);
    }
    let target = e.mem.u32(at(this, TARGET));
    if target != 0 {
        e.call(TARGET_AFTER_LOAD, &args![target]);
    }
}

fn set_pack_bits(e: &mut Engine, this: u32, bits: u32, on: bool) {
    let flags = e.mem.u32(at(this, PACK_FLAGS));
    let flags = if on { flags | bits } else { flags & !bits };
    e.mem.set_u32(at(this, PACK_FLAGS), flags);
}

// Typed from 00679700
/// `TESForm::LoadGame(flags, extra)`, then the form flag 0x8000_0000 sets the flag 0x8000 and the
/// form flag 0x4000_0000 the flag 0x10000.
pub fn load_form_flags(e: &mut Engine, this: u32, flags: u32, extra: u32) {
    typed_call!(
        FAMILY,
        0x0048_4c50,
        tesform::load_game(e, this, flags, extra)
    );
    if flags & FORM_FLAG_HIGH != 0 {
        set_pack_bits(e, this, PACK_FLAG_FOR_HIGH, true);
    }
    if flags & FORM_FLAG_NEXT != 0 {
        set_pack_bits(e, this, PACK_FLAG_FOR_NEXT, true);
    }
}

// Typed from 00679760
/// The form's flag clear (`004534f0`, empty: in place, nothing), then the form flag 0x8000_0000
/// clears the flag 0x8000 and the form flag 0x4000_0000 the flag 0x10000.
pub fn clear_form_flags(e: &mut Engine, this: u32, flags: u32) {
    if flags & FORM_FLAG_HIGH != 0 {
        set_pack_bits(e, this, PACK_FLAG_FOR_HIGH, false);
    }
    if flags & FORM_FLAG_NEXT != 0 {
        set_pack_bits(e, this, PACK_FLAG_FOR_NEXT, false);
    }
}

// Typed from 006797c0
/// `TESPackage::SaveGame` (Xbox PDB): for a package with a created id, the 12 bytes at +0x1c, the
/// parts byte (1 location, 2 target, 4 data object), each part's buffered save and the procedure
/// word, into `buffer`.
pub fn save_game(e: &mut Engine, this: u32, buffer: u32) {
    if !has_created_id(e, this) {
        return;
    }
    e.call(
        BUFFER_SAVE,
        &args![buffer, at(this, SAVED_BYTES), 0xcu32, 0u32],
    );
    let location = e.mem.u32(at(this, LOCATION));
    let target = e.mem.u32(at(this, TARGET));
    let data = e.mem.u32(at(this, DATA));
    let parts = u8::from(location != 0) | u8::from(target != 0) << 1 | u8::from(data != 0) << 2;
    e.with_stack(4, |e, slot| {
        e.mem.set_u8(slot.addr(), parts);
        e.call(BUFFER_SAVE, &args![buffer, slot, 1u32, 0u32]);
    });
    let location = e.mem.u32(at(this, LOCATION));
    if location != 0 {
        e.call(LOCATION_SAVE, &args![location, buffer]);
    }
    let target = e.mem.u32(at(this, TARGET));
    if target != 0 {
        e.call(TARGET_SAVE, &args![target, buffer]);
    }
    let data = e.mem.u32(at(this, DATA));
    if data != 0 {
        e.vcall(data, SLOT_DATA_SAVE, &args![buffer]);
    }
    e.call(BUFFER_SAVE, &args![buffer, at(this, PROCEDURE), 4u32, 0u32]);
}

/// The buffer's flag test for a package from a file (`00428110` or `0042ce30`, then `004280f0`, in
/// place): its save kind (the unaligned word at +0x17) or, for the init-load, its flags word (+0x2c)
/// stored in a local, tested against `mask`.
fn buffer_flag(e: &mut Engine, buffer: u32, mask: u32, init: bool) -> bool {
    let word = e.mem.u32(at(
        buffer,
        if init { BUFFER_FLAGS } else { BUFFER_SAVE_KIND },
    ));
    e.with_stack(4, |e, slot| {
        e.mem.set_u32(slot.addr(), word);
        word & mask != 0
    })
}

// Typed from 006798c0
/// `TESPackage::LoadGame` (Xbox PDB, the buffered one): for a package with a created id, what
/// [`save_game`] wrote (a new location and target when flagged, the data object's load through its
/// slot); for another the buffer's two flag tests set the flags 0x8000 and 0x10000.
pub fn load_game(e: &mut Engine, this: u32, buffer: u32) {
    if !has_created_id(e, this) {
        for (mask, bit) in [
            (FORM_FLAG_HIGH, PACK_FLAG_FOR_HIGH),
            (FORM_FLAG_NEXT, PACK_FLAG_FOR_NEXT),
        ] {
            if buffer_flag(e, buffer, mask, false) {
                set_pack_bits(e, this, bit, true);
            }
        }
        return;
    }
    e.call(BUFFER_LOAD, &args![buffer, at(this, SAVED_BYTES), 0xcu32]);
    let parts = e.with_stack(4, |e, slot| {
        e.mem.set_u8(slot.addr(), 0);
        e.call(BUFFER_LOAD, &args![buffer, slot, 1u32]);
        e.mem.u8(slot.addr())
    });
    if parts & 1 != 0 {
        let made = new_location(e);
        e.mem.set_u32(at(this, LOCATION), made);
        e.call(LOCATION_LOAD, &args![made, buffer]);
    }
    if parts & 2 != 0 {
        let made = new_target(e);
        e.mem.set_u32(at(this, TARGET), made);
        e.call(TARGET_LOAD, &args![made, buffer]);
    }
    if parts & 4 != 0 {
        let data = e.mem.u32(at(this, DATA));
        e.vcall(data, SLOT_DATA_LOAD, &args![buffer]);
    }
    e.call(BUFFER_LOAD, &args![buffer, at(this, PROCEDURE), 4u32]);
}

// Typed from 00679a90
/// `TESPackage::InitLoadGame` (Xbox PDB): for a package with a created id, the location's,
/// target's and data object's init-load (the data object's through its slot) for those it has.
pub fn init_load_game(e: &mut Engine, this: u32, buffer: u32) {
    if !has_created_id(e, this) {
        return;
    }
    let location = e.mem.u32(at(this, LOCATION));
    if location != 0 {
        e.call(LOCATION_INIT_LOAD, &args![location, buffer]);
    }
    let target = e.mem.u32(at(this, TARGET));
    if target != 0 {
        e.call(TARGET_INIT_LOAD, &args![target, buffer]);
    }
    let data = e.mem.u32(at(this, DATA));
    if data != 0 {
        e.vcall(data, SLOT_DATA_INIT_LOAD, &args![buffer]);
    }
}

// Typed from 00679b10
/// For a package from a file (no created id): the flag 0x8000 cleared when the buffer's init-load
/// flag test holds for 0x8000_0000, the flag 0x10000 for 0x4000_0000.
pub fn clear_loaded_flags(e: &mut Engine, this: u32, buffer: u32) {
    if has_created_id(e, this) {
        return;
    }
    for (mask, bit) in [
        (FORM_FLAG_HIGH, PACK_FLAG_FOR_HIGH),
        (FORM_FLAG_NEXT, PACK_FLAG_FOR_NEXT),
    ] {
        if buffer_flag(e, buffer, mask, true) {
            set_pack_bits(e, this, bit, false);
        }
    }
}
