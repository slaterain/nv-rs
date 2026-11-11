//! The geometry packing the draws run before a skinned or a dynamic geometry is drawn: a skin
//! partition's buffer packed ([`pack_skinned`], what the stream set-up runs for each drawn partition,
//! about 100 a frame in the crowded street), a shader declaration's pack of one stream
//! ([`pack_stream`]) and its build of the Direct3D declaration ([`build_declaration`], which the
//! stream set-up asks about 1,300 times a frame, almost always answering the one already built), a
//! stream's chip made again ([`remake_chip`], [`create_chip`]) and a vertex buffer's lock into the
//! renderer's staging block and its unlock ([`lock_vertices`], [`unlock_vertices`]).
//!
//! What these write that is the draw's own goes through [`super::own`]: the blocks they allocate
//! (condition 5 of the pipeline probe: the staging block, the declaration's element array, dynamic
//! chips), the critical section a lock holds until its unlock, and the reference counts they take or
//! give back.
//!
//! The translation of `00e76770` also notes what it packed for the `NV_HEAD_CHECK` diagnostic (not exe
//! behaviour); with this family on it takes no notes from here: `NV_TYPED=draw:off` for them.

use super::{own, shapes, FAMILY};
use crate::shader::{self, state};
use engine_core::ptr::Ptr;
use engine_core::{args, typed_call, Engine};

/// `IDirect3DVertexBuffer9::Lock` (slot 11), `Unlock` (slot 12) and `GetDesc` (slot 13).
const VERTEX_BUFFER_LOCK: u32 = 0x2c;
const VERTEX_BUFFER_UNLOCK: u32 = 0x30;
const VERTEX_BUFFER_GET_DESC: u32 = 0x34;
/// A geometry group's `CreateChip` (slot 0x18; `NiDynamicGeometryGroup`'s is [`create_chip`]) and
/// `ReleaseChip` (slot 0x1c).
const SLOT_CREATE_CHIP: u32 = 0x18;
const SLOT_RELEASE_CHIP: u32 = 0x1c;

/// A word's low byte, what a byte store or a byte flag keeps.
#[inline(always)]
#[allow(clippy::cast_possible_truncation)]
fn low8(word: u32) -> u8 {
    word as u8
}

// Typed from 00e8bfa0
/// `thiscall(manager, buffer data, stream)` (the manager is not read): stream `stream` of the buffer
/// data made again through its geometry group (+8): `ReleaseChip`, then `CreateChip` ([`create_chip`]
/// directly when the slot holds it); a null chip answers false, else it goes to the data's chip array
/// (+0x28, when the stream is within the count +0x20) and the answer is true.
pub fn remake_chip(e: &mut Engine, _this: Ptr, data: Ptr, stream: u32) -> bool {
    let d = data.addr();
    let group = e.mem.u32(d.wrapping_add(8));
    e.vcall(group, SLOT_RELEASE_CHIP, &args![data, stream]);
    let chip = if crate::slot_target(e, group, SLOT_CREATE_CHIP) == 0x00e9_5550 {
        typed_call!(
            FAMILY,
            0x00e9_5550,
            create_chip(e, Ptr::new(group), d, stream)
        )
    } else {
        e.vcall(group, SLOT_CREATE_CHIP, &args![data, stream]).u32()
    };
    if chip == 0 {
        return false;
    }
    own::store_chip(e, d, stream, chip);
    true
}

// Typed from 00e8bff0
/// `thiscall(manager, vertex buffer, offset, size, flags)`: `size` bytes at `offset` of the buffer
/// locked with `flags` (`Lock` into a local cleared first); a failed lock answers null. Otherwise the
/// manager's critical section (+0x80) is entered and held until the unlock, the locked address and
/// size kept (+0x48, +0x4c), the staging block (+0x40) made anew when its capacity (+0x44) is smaller
/// than `size`, and unless `flags` has a bit of 0x3000 (discard, no overwrite) the locked bytes copied
/// into it. Answers the staging block, which the caller fills.
pub fn lock_vertices(
    e: &mut Engine,
    this: Ptr,
    buffer: u32,
    offset: u32,
    size: u32,
    flags: u32,
) -> u32 {
    let base = this.addr();
    let (result, locked) = e.with_stack(4, |e, out| {
        e.mem.set_u32(out.addr(), 0);
        let result = e
            .vcall(buffer, VERTEX_BUFFER_LOCK, &args![offset, size, out, flags])
            .i32();
        (result, e.mem.u32(out.addr()))
    });
    if result < 0 {
        return 0;
    }
    own::enter_staging(e, base);
    e.mem.set_u32(base.wrapping_add(0x48), locked);
    e.mem.set_u32(base.wrapping_add(0x4c), size);
    if e.mem.u32(base.wrapping_add(0x44)) < size {
        own::grow_staging(e, base, size);
    }
    if flags & 0x3000 == 0 {
        let block = e.mem.u32(base.wrapping_add(0x40));
        let _: u32 = typed_call!(
            crate::crt::FAMILY,
            0x00ec_44d0,
            crate::crt::mem::memcpy(e, block, locked, size)
        );
    }
    e.mem.u32(base.wrapping_add(0x40))
}

// Typed from 00e8c0d0
/// `thiscall(manager, vertex buffer)`: the staging block (+0x40) copied back to the locked address
/// (+0x48, size +0x4c), the locked address and size cleared, the critical section the lock entered
/// left (+0x80), the buffer unlocked; answers whether the unlock succeeded.
pub fn unlock_vertices(e: &mut Engine, this: Ptr, buffer: u32) -> bool {
    let base = this.addr();
    let size = e.mem.u32(base.wrapping_add(0x4c));
    let block = e.mem.u32(base.wrapping_add(0x40));
    let locked = e.mem.u32(base.wrapping_add(0x48));
    let _: u32 = typed_call!(
        crate::crt::FAMILY,
        0x00ec_44d0,
        crate::crt::mem::memcpy(e, locked, block, size)
    );
    e.mem.set_u32(base.wrapping_add(0x48), 0);
    e.mem.set_u32(base.wrapping_add(0x4c), 0);
    own::leave_staging(e, base);
    e.vcall(buffer, VERTEX_BUFFER_UNLOCK, &[]).i32() >= 0
}

/// What `CreateChip` calls: the arrays' `SetSize` (`00a90180`, the size) and `SetAt` (`00ba3aa0`:
/// the index and the address of the element), the maps' `GetAt` (`00a53c30`: the key and the
/// address of the value; answers a byte) and `SetAt` (`00b57ea0`: key and value), the buffer
/// record maker (`00e95230`, `cdecl(size, device, third, fourth)`), the record's lock (`00e987c0`,
/// `thiscall(record, size, the addresses of the offset and of a word for the chip's +0x10, a flag,
/// the data's +0xc with its low byte replaced by its +0x14)`) and the chip maker (`00537d60`,
/// `cdecl`, five words).
const ARRAY_SET_SIZE: u32 = 0x00a9_0180;
const ARRAY_SET_AT: u32 = 0x00ba_3aa0;
const MAP_GET_AT: u32 = 0x00a5_3c30;
const MAP_SET_AT: u32 = 0x00b5_7ea0;
const NEW_RECORD: u32 = 0x00e9_5230;
const RECORD_LOCK: u32 = 0x00e9_87c0;
const NEW_CHIP: u32 = 0x0053_7d60;

// Typed from 00e95550
/// `NiDynamicGeometryGroup::CreateChip` (engine map), `thiscall(group, buffer data, stream)`: the
/// stream's stride is the data's +0x24 entry (0 past the count +0x20), the size the data's +0x1c times
/// it. With more than one stream the two arrays (+0x2c, +0x3c) grow to the stream count when their
/// 16-bit +0x34 is below it, the cursor +0x4c wraps to 0 at that size, the buffer record at the cursor
/// (a new one when the slot is empty) is locked with flag 1 and the chip at the cursor (a new one when
/// empty) gets the locked buffer, the offset and the size; the chip's +4 takes the record and +0x10
/// the lock's second word, the data's +0x38 is cleared and the cursor moves on. With one stream the key
/// is the data's +0xc (when 0, the stride with the top bit set): the record comes from the map at +0xc
/// (made and stored when absent or null), the lock runs with flag 0, the chip comes from the map at
/// +0x1c (made and stored when absent); the data's +0x38 becomes the offset over the stride (a stride
/// of 0 faults, as in the exe). Answers the chip.
pub fn create_chip(e: &mut Engine, this: Ptr, buffer_data: u32, index: u32) -> u32 {
    let base = this.addr();
    let streams = e.mem.u32(buffer_data.wrapping_add(0x20));
    let key_word = e.mem.u32(buffer_data.wrapping_add(0xc));
    let stride = if index < streams {
        let strides = e.mem.u32(buffer_data.wrapping_add(0x24));
        e.mem.u32(strides.wrapping_add(index.wrapping_mul(4)))
    } else {
        0
    };
    let size = e
        .mem
        .u32(buffer_data.wrapping_add(0x1c))
        .wrapping_mul(stride);
    // The locals the callees write through: the record (+0), the chip (+4), the lock's second word
    // (+8) and the offset (+0xc).
    e.with_stack(0x10, |e, locals| {
        let locals = locals.addr();
        let record_slot = locals;
        let chip_slot = locals + 4;
        let second_slot = locals + 8;
        let offset_slot = locals + 0xc;
        let flag_word =
            (key_word & 0xffff_ff00) | u32::from(e.mem.u8(buffer_data.wrapping_add(0x14)));
        if streams > 1 {
            if u32::from(e.mem.u16(base.wrapping_add(0x34))) < streams {
                e.call(ARRAY_SET_SIZE, &args![base.wrapping_add(0x2c), streams]);
                e.call(ARRAY_SET_SIZE, &args![base.wrapping_add(0x3c), streams]);
            }
            if e.mem.u32(base.wrapping_add(0x4c)) >= u32::from(e.mem.u16(base.wrapping_add(0x34))) {
                e.mem.set_u32(base.wrapping_add(0x4c), 0);
            }
            let cursor = e.mem.u32(base.wrapping_add(0x4c));
            let records = e.mem.u32(base.wrapping_add(0x30));
            let record = e.mem.u32(records.wrapping_add(cursor.wrapping_mul(4)));
            e.mem.set_u32(record_slot, record);
            if record == 0 {
                let device = e.mem.u32(base.wrapping_add(8));
                let made = e
                    .call(NEW_RECORD, &args![0x800u32, device, 0u32, 0u32])
                    .u32();
                let cursor = e.mem.u32(base.wrapping_add(0x4c));
                e.mem.set_u32(record_slot, made);
                e.call(
                    ARRAY_SET_AT,
                    &args![base.wrapping_add(0x2c), cursor, record_slot],
                );
            }
            let record = e.mem.u32(record_slot);
            let locked = e
                .call(
                    RECORD_LOCK,
                    &args![record, size, offset_slot, second_slot, 1u32, flag_word],
                )
                .u32();
            let cursor = e.mem.u32(base.wrapping_add(0x4c));
            let chips = e.mem.u32(base.wrapping_add(0x40));
            let mut chip = e.mem.u32(chips.wrapping_add(cursor.wrapping_mul(4)));
            if chip == 0 {
                let offset = e.mem.u32(offset_slot);
                chip = e
                    .call(NEW_CHIP, &args![0u32, locked, offset, size, 0u32])
                    .u32();
                let cursor = e.mem.u32(base.wrapping_add(0x4c));
                e.mem.set_u32(chip_slot, chip);
                e.call(
                    ARRAY_SET_AT,
                    &args![base.wrapping_add(0x3c), cursor, chip_slot],
                );
            } else {
                e.mem.set_u32(chip.wrapping_add(8), locked);
                let offset = e.mem.u32(offset_slot);
                e.mem.set_u32(chip.wrapping_add(0xc), offset);
                e.mem.set_u32(chip.wrapping_add(0x14), size);
            }
            let record = e.mem.u32(record_slot);
            e.mem.set_u32(chip.wrapping_add(4), record);
            let second = e.mem.u32(second_slot);
            e.mem.set_u32(chip.wrapping_add(0x10), second);
            e.mem.set_u32(buffer_data.wrapping_add(0x38), 0);
            let cursor = e.mem.u32(base.wrapping_add(0x4c)).wrapping_add(1);
            e.mem.set_u32(base.wrapping_add(0x4c), cursor);
            return chip;
        }
        let key = if key_word != 0 {
            key_word
        } else {
            stride | 0x8000_0000
        };
        let found = e
            .call(MAP_GET_AT, &args![base.wrapping_add(0xc), key, record_slot])
            .bool();
        if !found || e.mem.u32(record_slot) == 0 {
            let device = e.mem.u32(base.wrapping_add(8));
            let made = e
                .call(NEW_RECORD, &args![0x800u32, device, key_word, 0u32])
                .u32();
            e.mem.set_u32(record_slot, made);
            e.call(MAP_SET_AT, &args![base.wrapping_add(0xc), key, made]);
        }
        let record = e.mem.u32(record_slot);
        let locked = e
            .call(
                RECORD_LOCK,
                &args![record, size, offset_slot, second_slot, 0u32, flag_word],
            )
            .u32();
        e.call(MAP_GET_AT, &args![base.wrapping_add(0x1c), key, chip_slot]);
        let mut chip = e.mem.u32(chip_slot);
        if chip == 0 {
            let offset = e.mem.u32(offset_slot);
            chip = e
                .call(NEW_CHIP, &args![0u32, locked, offset, size, 0u32])
                .u32();
            e.call(MAP_SET_AT, &args![base.wrapping_add(0x1c), key, chip]);
        } else {
            e.mem.set_u32(chip.wrapping_add(8), locked);
            let offset = e.mem.u32(offset_slot);
            e.mem.set_u32(chip.wrapping_add(0xc), offset);
            e.mem.set_u32(chip.wrapping_add(0x14), size);
        }
        let record = e.mem.u32(record_slot);
        e.mem.set_u32(chip.wrapping_add(4), record);
        let second = e.mem.u32(second_slot);
        e.mem.set_u32(chip.wrapping_add(0x10), second);
        // `DIV`: a stride of 0 faults in the game as it stops here.
        let offset = e.mem.u32(offset_slot);
        e.mem
            .set_u32(buffer_data.wrapping_add(0x38), offset / stride);
        chip
    })
}

/// The memory manager (`011f6238`) and the element adder (`00e76660`, `thiscall(declaration, entry,
/// stream)`, answering the bytes the entry covers).
const ADD_ELEMENT: u32 = 0x00e7_6660;
/// `IDirect3DDevice9::CreateVertexDeclaration` (slot 86).
const DEVICE_CREATE_VERTEX_DECLARATION: u32 = 0x158;

/// Whether a declaration entry is in use: its +8 is not -1 and its type +0xc not 0x11.
#[inline(always)]
fn entry_used(e: &Engine, entry: u32) -> bool {
    e.mem.u32(entry.wrapping_add(8)) != 0xffff_ffff && e.mem.u32(entry.wrapping_add(0xc)) != 0x11
}

// Typed from 00e76a80
/// A shader declaration's build (its slot 0xb4), `thiscall(declaration)`: without the modified byte
/// (+0x28) the built declaration (+0x30). Otherwise the built one is released and cleared; each stream
/// (16 bytes at +0x24; the address tested for null) whose entries (+8, 0x1c bytes each, count +0x1c)
/// are in use is marked and counted, plus one for the end when there are any. The element array (+0x2c,
/// capacity +0x14) is freed when too small; with nothing in use the capacity is cleared and 0
/// answered. Otherwise a new array (8 bytes each, all ones on overflow) is taken when there is none,
/// the element count +0x18 cleared, and each marked stream's used entries get their offset (+4, the
/// running total) and are added (`00e76660`); the stream's +0xc gets the total. The end element
/// (`0xff`, 0, type 0x11, 0, 0, 0) is written, and the device (+0x10) makes the declaration into +0x30:
/// a failure answers 0, otherwise the modified byte is cleared and the declaration answered.
pub fn build_declaration(e: &mut Engine, this: Ptr) -> u32 {
    let base = this.addr();
    let built = e.mem.u32(base.wrapping_add(0x30));
    if e.mem.u8(base.wrapping_add(0x28)) == 0 {
        return built;
    }
    if built != 0 {
        own::release(e, built);
        e.mem.set_u32(base.wrapping_add(0x30), 0);
    }
    let mut used = 0u32;
    let mut index = 0u32;
    let mut offset = 0u32;
    if e.mem.u32(base.wrapping_add(0x20)) != 0 {
        while index < e.mem.u32(base.wrapping_add(0x20)) {
            let stream = e.mem.u32(base.wrapping_add(0x24)).wrapping_add(offset);
            if stream != 0 {
                e.mem.set_u8(stream, 0);
                let mut at = e.mem.u32(stream.wrapping_add(8));
                let mut entry = 0u32;
                while entry < e.mem.u32(base.wrapping_add(0x1c)) {
                    if entry_used(e, at) {
                        used += 1;
                        e.mem.set_u8(stream, 1);
                    }
                    entry += 1;
                    at = at.wrapping_add(0x1c);
                }
            }
            index += 1;
            offset = offset.wrapping_add(0x10);
        }
        if used != 0 {
            used += 1;
        }
    }
    if e.mem.u32(base.wrapping_add(0x14)) < used {
        let elements = e.mem.u32(base.wrapping_add(0x2c));
        own::free_block(e, elements);
        e.mem.set_u32(base.wrapping_add(0x2c), 0);
    }
    if used == 0 {
        e.mem.set_u32(base.wrapping_add(0x14), 0);
        return 0;
    }
    if e.mem.u32(base.wrapping_add(0x2c)) == 0 {
        e.mem.set_u32(base.wrapping_add(0x14), used);
        let bytes = u32::try_from(u64::from(used) * 8).unwrap_or(u32::MAX);
        let elements = own::allocate_block(e, bytes);
        e.mem.set_u32(base.wrapping_add(0x2c), elements);
    }
    e.mem.set_u32(base.wrapping_add(0x18), 0);
    let mut stream_index = 0u32;
    let mut offset = 0u32;
    while stream_index < e.mem.u32(base.wrapping_add(0x20)) {
        let stream = e.mem.u32(base.wrapping_add(0x24)).wrapping_add(offset);
        if e.mem.u8(stream) != 0 {
            let mut total = 0u32;
            let mut entry = 0u32;
            let mut entry_offset = 0u32;
            while entry < e.mem.u32(base.wrapping_add(0x1c)) {
                let at = e.mem.u32(stream.wrapping_add(8)).wrapping_add(entry_offset);
                if entry_used(e, at) {
                    e.mem.set_u32(at.wrapping_add(4), total);
                    let covered = e.call(ADD_ELEMENT, &args![base, at, stream_index]).u32();
                    total = total.wrapping_add(covered);
                }
                entry += 1;
                entry_offset = entry_offset.wrapping_add(0x1c);
            }
            e.mem.set_u32(stream.wrapping_add(0xc), total);
        }
        offset = offset.wrapping_add(0x10);
        stream_index += 1;
    }
    let end = |e: &Engine| {
        let count = e.mem.u32(base.wrapping_add(0x18));
        let elements = e.mem.u32(base.wrapping_add(0x2c));
        elements.wrapping_add(count.wrapping_mul(8))
    };
    let at = end(e);
    e.mem.set_u16(at, 0xff);
    let at = end(e);
    e.mem.set_u16(at.wrapping_add(2), 0);
    let at = end(e);
    e.mem.set_u8(at.wrapping_add(4), 0x11);
    for byte in 5..8u32 {
        let at = end(e);
        e.mem.set_u8(at.wrapping_add(byte), 0);
    }
    let device = e.mem.u32(base.wrapping_add(0x10));
    let elements = e.mem.u32(base.wrapping_add(0x2c));
    let result = e
        .vcall(
            device,
            DEVICE_CREATE_VERTEX_DECLARATION,
            &args![elements, base.wrapping_add(0x30)],
        )
        .i32();
    if result < 0 {
        return 0;
    }
    e.mem.set_u8(base.wrapping_add(0x28), 0);
    e.mem.u32(base.wrapping_add(0x30))
}

/// What a stream's pack calls: the buffer data's update (`00e8f040`, `thiscall`, one word), the
/// stream's chip maker (`00e69b40`, `thiscall(buffer data, stream)`), the entry packer (`00e7a200`,
/// `thiscall`, nine words: the geometry family's element pack, a direct call while that family is on)
/// and the data's own pack of the derived vectors (`00a855f0`, `thiscall`, eight words).
const BUFFER_DATA_UPDATE: u32 = 0x00e8_f040;
const NEW_STREAM_CHIP: u32 = 0x00e6_9b40;
const PACK_DERIVED: u32 = 0x00a8_55f0;

/// What an entry of the declaration does in a stream's pack, by its usage: packed always, packed with
/// a bit of the flags, or a derived vector (packed with its bit unless the geometry data packs those
/// itself, which then gets its address in a local slot).
enum Usage {
    Pack,
    Bit(u8),
    Derived(u8, u32),
}

/// The usage's case in the code's byte table (`00e76a68`) and jump table (`00e76a4c`).
#[inline(always)]
fn usage_case(usage: u32) -> Usage {
    match usage {
        0 => Usage::Derived(1, 0x18),
        3 => Usage::Derived(2, 0x14),
        4 => Usage::Bit(4),
        5..=12 => Usage::Bit(8),
        13 => Usage::Derived(2, 0xc),
        14 => Usage::Derived(2, 0x10),
        _ => Usage::Pack,
    }
}

// Typed from 00e76770
/// A shader declaration's pack of one stream (its slot 0xac), `thiscall(declaration, nine words: the
/// geometry data, a second data word (the data that packs the derived vectors itself), the skin
/// partition, the flags, the chip, the stream, a locked address, two words for the packers)`. The
/// vertex count is the partition's 16-bit +0x1c (without one, the data's vertex count); with none the
/// chip is answered. The declaration's slot 0xb0 must answer set (it gives a word for `00e8f040` and
/// another), else 0. The buffer data (the partition's +0x28, else the data's +0x34) is updated, takes
/// the stream's stride and, with the renderer's byte +0x5d4, the declaration's slot 0xa8 byte at
/// +0x14. With a locked address that chip is used as is; otherwise the chip is kept when its vertex
/// buffer is vertex data (`GetDesc`, format 100) big enough for the count times the declaration's slot
/// 0xa0 for the stream, else a new one is made ([`remake_chip`] must answer set, else 0; `00e69b40`;
/// the flags become 0xfff), and either way locked ([`lock_vertices`]). Each entry of the stream is
/// packed (`00e7a200`) at the locked address plus its offset, a derived vector (usages 0, 3, 13, 14)
/// only with its flag bit and, with the second data word and no partition, kept for `00a855f0` instead,
/// which then packs them; usages 4 and 5 to 12 need the flag bits 4 and 8. Without a locked address
/// the chip's vertex buffer is unlocked ([`unlock_vertices`]; 0 when it answers false). Answers the
/// chip.
#[allow(clippy::too_many_arguments)]
pub fn pack_stream(
    e: &mut Engine,
    this: Ptr,
    data: u32,
    derived: u32,
    partition: u32,
    flags: u32,
    chip: u32,
    stream: u32,
    locked: u32,
    eighth: u32,
    ninth: u32,
) -> u32 {
    let base = this.addr();
    let count = if partition != 0 {
        u32::from(e.mem.u16(partition.wrapping_add(0x1c)))
    } else {
        u32::from(shapes::vertex_count(e, data))
    };
    if count == 0 {
        return chip;
    }
    // The locals passed by address: the two words of slot 0xb0 (+0, +4) and the vertex buffer
    // description (+8: format, +0x18: size).
    e.with_stack(0x20, |e, locals| {
        let locals = locals.addr();
        let first = locals;
        let second = locals + 4;
        let description = locals + 8;
        if !e.vcall(base, 0xb0, &args![first, second]).bool() {
            return 0;
        }
        let buffer_data = if partition != 0 {
            e.mem.u32(partition.wrapping_add(0x28))
        } else {
            e.mem.u32(data.wrapping_add(0x34))
        };
        let word = e.mem.u32(first);
        e.call(BUFFER_DATA_UPDATE, &args![buffer_data, word]);
        if stream < e.mem.u32(buffer_data.wrapping_add(0x20)) {
            let streams = e.mem.u32(base.wrapping_add(0x24));
            let stride = e.mem.u32(
                stream
                    .wrapping_shl(4)
                    .wrapping_add(streams)
                    .wrapping_add(0xc),
            );
            let strides = e.mem.u32(buffer_data.wrapping_add(0x24));
            e.mem
                .set_u32(strides.wrapping_add(stream.wrapping_mul(4)), stride);
        }
        let renderer = e.mem.u32(base.wrapping_add(8));
        if e.mem.u8(renderer.wrapping_add(0x5d4)) != 0 {
            let byte = low8(e.vcall(base, 0xa8, &args![]).u32());
            e.mem.set_u8(buffer_data.wrapping_add(0x14), byte);
        }
        let mut flags = flags;
        let mut result = chip;
        let mut address = locked;
        let unlock = locked == 0;
        if unlock {
            let mut keep = false;
            if chip != 0 {
                let buffer = e.mem.u32(chip.wrapping_add(8));
                if buffer != 0 {
                    e.vcall(buffer, VERTEX_BUFFER_GET_DESC, &args![description]);
                    if e.mem.u32(description) == 0x64 {
                        let per_vertex = e.vcall(base, 0xa0, &args![stream]).u32();
                        if e.mem.u32(description + 0x10) >= per_vertex.wrapping_mul(count) {
                            keep = true;
                        }
                    }
                }
            }
            if !keep {
                let manager = e.mem.u32(base.wrapping_add(0xc));
                if !typed_call!(
                    FAMILY,
                    0x00e8_bfa0,
                    remake_chip(e, Ptr::new(manager), Ptr::new(buffer_data), stream)
                ) {
                    return 0;
                }
                result = e.call(NEW_STREAM_CHIP, &args![buffer_data, stream]).u32();
                flags = 0xfff;
            }
            let manager = e.mem.u32(base.wrapping_add(0xc));
            let buffer = e.mem.u32(result.wrapping_add(8));
            let offset = e.mem.u32(result.wrapping_add(0xc));
            let size = e.mem.u32(result.wrapping_add(0x14));
            let other = e.mem.u32(result.wrapping_add(0x10));
            address = typed_call!(
                FAMILY,
                0x00e8_bff0,
                lock_vertices(e, Ptr::new(manager), buffer, offset, size, other)
            );
        }
        let packs_itself = derived != 0 && partition == 0;
        let stream_entry = e
            .mem
            .u32(base.wrapping_add(0x24))
            .wrapping_add(stream.wrapping_shl(4));
        // The derived vectors' addresses, by their local slot (+0xc, +0x10, +0x14, +0x18).
        let mut kept = [0u32; 4];
        if stream_entry != 0 {
            let mut index = 0u32;
            let mut offset = 0u32;
            while index < e.mem.u32(base.wrapping_add(0x1c)) {
                let entries = e.mem.u32(stream_entry.wrapping_add(8));
                let at = entries.wrapping_add(offset);
                let target = e.mem.u32(at.wrapping_add(4)).wrapping_add(address);
                let pack = match usage_case(e.mem.u32(at.wrapping_add(8))) {
                    Usage::Pack => true,
                    Usage::Bit(bit) => low8(flags) & bit != 0,
                    Usage::Derived(bit, slot) => {
                        if low8(flags) & bit == 0 {
                            false
                        } else if packs_itself {
                            kept[((slot - 0xc) / 4) as usize] = target;
                            false
                        } else {
                            true
                        }
                    }
                };
                if pack {
                    let step = e.mem.u32(stream_entry.wrapping_add(0xc));
                    let _: u32 = typed_call!(
                        crate::geometry::FAMILY,
                        0x00e7_a200,
                        crate::geometry::pack::pack_entry(
                            e,
                            Ptr::new(base),
                            count,
                            data,
                            derived,
                            partition,
                            at,
                            step,
                            target,
                            eighth,
                            ninth
                        )
                    );
                }
                index += 1;
                offset = offset.wrapping_add(0x1c);
            }
        }
        if packs_itself {
            let step = e.mem.u32(stream_entry.wrapping_add(0xc));
            let vertices = e.mem.u32(data.wrapping_add(0x20));
            let normals = e.mem.u32(data.wrapping_add(0x24));
            let vertex_count = u32::from(e.mem.u16(data.wrapping_add(8)));
            e.call(
                PACK_DERIVED,
                &args![
                    derived,
                    vertices,
                    normals,
                    vertex_count,
                    kept[3],
                    kept[2],
                    kept[1],
                    kept[0],
                    step
                ],
            );
        }
        if unlock {
            let manager = e.mem.u32(base.wrapping_add(0xc));
            let buffer = e.mem.u32(result.wrapping_add(8));
            if !typed_call!(
                FAMILY,
                0x00e8_c0d0,
                unlock_vertices(e, Ptr::new(manager), buffer)
            ) {
                return 0;
            }
        }
        result
    })
}

/// The buffer data's stream count set (`00e8ef10`, `thiscall`, the count), `RemoveIB` (`00e8eee0`),
/// the vertex packer (`00e8c820`, on the renderer's +0x8bc, six words) and the index buffer maker
/// (`00e8f200`, on the renderer's +0x8c0, seven words).
const SET_STREAM_COUNT: u32 = 0x00e8_ef10;
const REMOVE_IB: u32 = 0x00e8_eee0;
const VERTEX_PACK: u32 = 0x00e8_c820;
const INDEX_PACK: u32 = 0x00e8_f200;
/// The cap on the texture-set count (`011bf558`) and whether the device is Direct3D 9Ex (`0126f0d0`).
const TEXTURE_SET_LIMIT: u32 = 0x011b_f558;
const DIRECT3D_IS_EX: u32 = 0x0126_f0d0;

// Typed from 00e6d780
/// The renderer's pack of a skin partition's buffer (`thiscall(renderer, buffer data, geometry data, a
/// word passed on, the partition, the packer or 0, force)`): false at once for no buffer data or data;
/// true at once when not forced, packed (`00e8eeb0`) and the data's +0xe at 0x4000 in its bits 0x7000.
/// The buffer data's +4 gets the stream flags (the texture-set bit capped by `011bf558` in the top
/// byte; 0x400000 with colours, 0x800000 with normals, 0x200000 with normals and a bit of 0xf000 in
/// +0xc). The streams (the packer's +0x20, else 1) are made again when not packed, or set again when
/// packed with another count (both repack everything); the partition's counts go to the buffer data.
/// With a bit of the mask other than 0x10 each stream is packed by the packer's slot 0xac
/// ([`pack_stream`] directly when the slot holds it) or, failing that with one stream, `00e8c820`; a
/// failure answers false. With 0x30 in the mask the index buffer is asked of `00e8f200` (a null answer
/// is false) and a new one replaces the old. Answers true.
#[allow(clippy::too_many_arguments)]
pub fn pack_skinned(
    e: &mut Engine,
    this: Ptr,
    buffer_data: Ptr,
    geometry: Ptr,
    passed: u32,
    source: Ptr,
    packer: Ptr,
    force: u8,
) -> bool {
    let b = buffer_data.addr();
    let g = geometry.addr();
    if b == 0 || g == 0 {
        return false;
    }
    let has_buffer: bool = typed_call!(
        shader::FAMILY,
        0x00e8_eeb0,
        state::has_streams(e, Ptr::new(b))
    );
    if force == 0 && has_buffer && e.mem.u16(g.wrapping_add(0xe)) & 0x7000 == 0x4000 {
        return true;
    }
    let has_28 = e.mem.u32(g.wrapping_add(0x28)) != 0;
    let has_24 = e.mem.u32(g.wrapping_add(0x24)) != 0;
    let has_f000 = has_24 && e.mem.u16(g.wrapping_add(0xc)) & 0xf000 != 0;
    let mut low = u32::from(e.mem.u16(g.wrapping_add(0xc))) & 1;
    let limit: u32 = e.global(TEXTURE_SET_LIMIT);
    if low > limit {
        low = limit;
    }
    let mut bits = if has_28 { 0x40_0000 } else { 0 };
    if has_24 {
        bits |= 0x80_0000;
    }
    if has_f000 {
        bits |= 0x20_0000;
    }
    e.mem.set_u32(b.wrapping_add(4), (low << 24) | bits);
    let s = source.addr();
    let word_14 = e.mem.u32(s.wrapping_add(0x14));
    let half_1c = u32::from(e.mem.u16(s.wrapping_add(0x1c)));
    let half_1e = u32::from(e.mem.u16(s.wrapping_add(0x1e)));
    let half_22 = e.mem.u16(s.wrapping_add(0x22));
    let word_18 = e.mem.u32(s.wrapping_add(0x18));
    let (index_count, strips) = if half_22 == 0 {
        (half_1e.wrapping_mul(3), 1u32)
    } else {
        (
            half_1e.wrapping_add(u32::from(half_22) * 2),
            u32::from(half_22),
        )
    };
    let p = packer.addr();
    let count = if p != 0 {
        e.mem.u32(p.wrapping_add(0x20))
    } else {
        1
    };
    let mut mask: u32 = if force != 0 {
        0xfff
    } else {
        u32::from(e.mem.u16(g.wrapping_add(0xe)) & 0xfff)
    };
    if has_buffer {
        if count != e.mem.u32(b.wrapping_add(0x20)) {
            e.call(SET_STREAM_COUNT, &args![b, count]);
            mask = 0xfff;
        }
    } else {
        let streams = e.mem.u32(b.wrapping_add(8));
        let mut stream = 0u32;
        while stream < count {
            e.vcall(streams, 0x1c, &args![b, stream]);
            stream += 1;
        }
        e.call(SET_STREAM_COUNT, &args![b, count]);
        e.mem.set_u32(b.wrapping_add(0x38), 0);
        mask = 0xfff;
    }
    e.mem.set_u32(b.wrapping_add(0x18), half_1c);
    e.mem.set_u32(b.wrapping_add(0x1c), half_1c);
    e.mem.set_u32(b.wrapping_add(0x4c), word_18);
    e.mem.set_u32(b.wrapping_add(0x50), word_14);
    e.mem.set_u32(b.wrapping_add(0x40), half_1e);
    e.mem.set_u32(b.wrapping_add(0x44), half_1e);
    e.mem.set_u32(b.wrapping_add(0x48), strips);
    if mask & 0xffff_ffef != 0 {
        let mut stream = 0u32;
        while stream < count {
            let target = if stream < e.mem.u32(b.wrapping_add(0x20)) {
                let list = e.mem.u32(b.wrapping_add(0x28));
                e.mem.u32(list.wrapping_add(stream * 4))
            } else {
                0
            };
            let packed = p != 0 && {
                let chip = if crate::slot_target(e, p, 0xac) == 0x00e7_6770 {
                    typed_call!(
                        FAMILY,
                        0x00e7_6770,
                        pack_stream(
                            e, packer, g, passed, s, mask, target, stream, 0u32, 0u32, 0u32
                        )
                    )
                } else {
                    e.vcall(
                        p,
                        0xac,
                        &args![g, passed, s, mask, target, stream, 0u32, 0u32, 0u32],
                    )
                    .u32()
                };
                chip != 0
            };
            if !packed {
                if count != 1 {
                    return false;
                }
                let vertex_packer = e.mem.u32(this.addr().wrapping_add(0x8bc));
                if e.call(
                    VERTEX_PACK,
                    &args![vertex_packer, g, passed, s, mask, target, 0u32],
                )
                .u32()
                    == 0
                {
                    return false;
                }
            }
            stream += 1;
        }
    }
    if mask & 0x30 != 0 {
        let flag_10: u32 = if e.mem.u8(b.wrapping_add(0x14)) != 0 {
            0x10
        } else {
            0
        };
        let old = e.mem.u32(b.wrapping_add(0x34));
        let not_ex = u32::from(e.global::<u8>(DIRECT3D_IS_EX) == 0);
        let index_packer = e.mem.u32(this.addr().wrapping_add(0x8c0));
        let start = e.mem.u32(b.wrapping_add(0x30));
        let (result, slot) = e.with_stack(4, |e, slot| {
            e.mem.set_u32(slot.addr(), start);
            let result = e
                .call(
                    INDEX_PACK,
                    &args![
                        index_packer,
                        word_14,
                        index_count,
                        index_count,
                        old,
                        slot,
                        not_ex,
                        flag_10
                    ],
                )
                .u32();
            (result, e.mem.u32(slot.addr()))
        });
        if result == 0 {
            return false;
        }
        if old != result {
            e.call(REMOVE_IB, &args![b]);
            e.mem.set_u32(b.wrapping_add(0x34), result);
            e.mem.set_u32(b.wrapping_add(0x2c), index_count);
            e.mem.set_u32(b.wrapping_add(0x30), slot);
        }
    }
    true
}
