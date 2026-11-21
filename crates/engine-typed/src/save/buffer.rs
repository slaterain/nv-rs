//! The save buffer every form's save writes its fields into (`BGSSaveGameBuffer`, Xbox PDB: the
//! vtable, the block at `+4`, its size at `+8`, the write position at `+0xc` and the separators
//! written at `+0x10`), and the save file it goes to (`BGSSaveLoadFile::Write`). A field is a
//! positioned write (`00865ce0`): the block grown by 0x10000 bytes while the write would pass its
//! end (`008660b0`), the bytes copied in unless the save is only being sized (the save/load
//! manager's byte at `+0xc`, `00424940`), and a `|` after them. Each form's save makes tens of
//! these; a quicksave about a million.

use super::FAMILY;
use crate::crt;
use engine_core::ptr::Ptr;
use engine_core::{args, typed_call, Engine};

/// The save/load manager (`011de134`); its byte at `+0xc` says the save is only being sized.
const MANAGER: u32 = 0x011d_e134;
/// The game's `operator new` and `operator delete` (`00401000`, `00401030`, cdecl).
const ALLOCATE: u32 = 0x0040_1000;
const FREE: u32 = 0x0040_1030;
/// The bounded move `insert_space` makes room with (`008661e0`, cdecl: destination, capacity,
/// source, size).
const MOVE_BYTES: u32 = 0x0086_61e0;
/// `SaveGameWarning` (`00845cc0`, cdecl) and `"SAVELOAD: Variable-sized value %08X is too large to
/// be stored."` (`0107ee00`).
const SAVE_GAME_WARNING: u32 = 0x0084_5cc0;
const VALUE_TOO_LARGE_FORMAT: u32 = 0x0107_ee00;
/// `SaveGameDataToFileFunction` (`00845d00`, cdecl: file, source, size).
const SAVE_DATA_TO_FILE: u32 = 0x0084_5d00;
/// The real file's write (`00473180`, `thiscall`: source, size).
const FILE_WRITE: u32 = 0x0047_3180;

/// `BGSSaveGameBuffer`'s fields.
const BUFFER: u32 = 4;
const SIZE: u32 = 8;
const POSITION: u32 = 0xc;
const SEPARATORS: u32 = 0x10;
/// `BGSSaveLoadFile`'s fields: the real file, the simulated size and position.
const FILE: u32 = 0x104;
const SIMULATED_SIZE: u32 = 0x108;
const SIMULATED_POSITION: u32 = 0x10c;

// Typed from 00424940
/// The byte at `+0xc` (`thiscall`): for the save/load manager, whether the save is only being sized.
pub fn byte_c(e: &mut Engine, this: Ptr) -> u8 {
    e.mem.u8(this.addr().wrapping_add(0xc))
}

/// Whether the save is only being sized (`00424940` on the manager, read again at every call).
#[inline(always)]
fn sizing(e: &mut Engine) -> bool {
    let manager = e.mem.u32(MANAGER);
    typed_call!(FAMILY, 0x0042_4940, byte_c(e, Ptr::new(manager))) != 0
}

// Typed from 008660b0
/// The buffer grown by 0x10000 bytes: the size first; unless sizing, a new block of the new size
/// (read again), the old block's bytes copied in when there was one, the old block freed (also a
/// null one).
pub fn grow(e: &mut Engine, this: Ptr) {
    let t = this.addr();
    let old_buffer = e.mem.u32(t.wrapping_add(BUFFER));
    let old_size = e.mem.u32(t.wrapping_add(SIZE));
    e.mem
        .set_u32(t.wrapping_add(SIZE), old_size.wrapping_add(0x10000));
    if !sizing(e) {
        let size = e.mem.u32(t.wrapping_add(SIZE));
        let buffer = e.call(ALLOCATE, &args![size]).u32();
        e.mem.set_u32(t.wrapping_add(BUFFER), buffer);
        if old_buffer != 0 {
            let destination = e.mem.u32(t.wrapping_add(BUFFER));
            typed_call!(
                crt::FAMILY,
                0x0040_1460,
                crt::mem::memcpy_wrapper(e, Ptr::new(destination), Ptr::new(old_buffer), old_size)
            );
        }
        e.call(FREE, &args![old_buffer]);
    }
}

// Typed from 00865ce0
/// A positioned write (`thiscall(buffer, source, size, position, no_separator)`): the end is the
/// position, the size and one for the separator unless `no_separator`; the buffer grown while the
/// end passes its size (unsigned); unless sizing, the bytes copied in at the position; then the
/// separator (unless sizing) and, when the write was at the write position, a separator counted;
/// the write position moved to the end when it was before it. Fields read again across calls.
pub fn write_at(
    e: &mut Engine,
    this: Ptr,
    source: Ptr,
    size: u32,
    position: u32,
    no_separator: u8,
) {
    let t = this.addr();
    let end = position
        .wrapping_add(size)
        .wrapping_add(u32::from(no_separator == 0));
    while end > e.mem.u32(t.wrapping_add(SIZE)) {
        typed_call!(FAMILY, 0x0086_60b0, grow(e, this));
    }
    if !sizing(e) {
        let destination = e.mem.u32(t.wrapping_add(BUFFER)).wrapping_add(position);
        typed_call!(
            crt::FAMILY,
            0x0040_1460,
            crt::mem::memcpy_wrapper(e, Ptr::new(destination), source, size)
        );
    }
    if no_separator == 0 {
        if !sizing(e) {
            let buffer = e.mem.u32(t.wrapping_add(BUFFER));
            e.mem
                .set_u8(buffer.wrapping_add(position).wrapping_add(size), b'|');
        }
        if position == e.mem.u32(t.wrapping_add(POSITION)) {
            let separators = e.mem.u32(t.wrapping_add(SEPARATORS));
            e.mem
                .set_u32(t.wrapping_add(SEPARATORS), separators.wrapping_add(1));
        }
    }
    if e.mem.u32(t.wrapping_add(POSITION)) < end {
        e.mem.set_u32(t.wrapping_add(POSITION), end);
    }
}

// Typed from 00865570
/// The append (`thiscall(buffer, source, size, no_separator)`): a positioned write at the write
/// position.
pub fn append(e: &mut Engine, this: Ptr, source: Ptr, size: u32, no_separator: u8) {
    let position = e.mem.u32(this.addr().wrapping_add(POSITION));
    typed_call!(
        FAMILY,
        0x0086_5ce0,
        write_at(e, this, source, size, position, no_separator)
    );
}

// Typed from 00865e50
/// `BGSSaveGameBuffer::SaveData` (`RET 0xc`, the third word unused): appended with a separator.
pub fn save_bytes(e: &mut Engine, this: Ptr, source: Ptr, size: u32, _unused: u32) {
    typed_call!(FAMILY, 0x0086_5570, append(e, this, source, size, 0));
}

/// `value` in the low `size` bytes of a 4-byte local, appended (flag 0).
#[inline(always)]
fn append_local(e: &mut Engine, this: Ptr, value: u32, size: u32) {
    e.with_stack(4, |e, local| {
        match size {
            1 => e.mem.set_u8(local.addr(), value.to_le_bytes()[0]),
            2 => {
                let [low, high, _, _] = value.to_le_bytes();
                e.mem.set_u16(local.addr(), u16::from_le_bytes([low, high]));
            }
            _ => e.mem.set_u32(local.addr(), value),
        }
        typed_call!(FAMILY, 0x0086_5570, append(e, this, local, size, 0));
    });
}

// Typed from 00865f60
/// `BGSSaveGameBuffer::SaveVariableSizedValue` (Xbox PDB): `value` tagged in its two low bits, one
/// byte (`value << 2`) below 0x40, two (`| 1`) below 0x4000, four (`| 2`) below 0x40000000; larger
/// values only give the warning.
pub fn save_variable_sized_value(e: &mut Engine, this: Ptr, value: u32) {
    if value < 0x40 {
        append_local(e, this, value << 2, 1);
    } else if value < 0x4000 {
        append_local(e, this, (value << 2) | 1, 2);
    } else if value < 0x4000_0000 {
        append_local(e, this, (value << 2) | 2, 4);
    } else {
        e.call(SAVE_GAME_WARNING, &args![VALUE_TOO_LARGE_FORMAT, value]);
    }
}

// Typed from 00865e70
/// `BGSSaveGameBuffer::SaveString` (Xbox PDB; `RET 8`, the second word unused): the length
/// (`strlen`, 0 for no text) as 16 bits, then that many bytes when it is not 0.
pub fn save_string(e: &mut Engine, this: Ptr, text: Ptr, _unused: u32) {
    let length = if text.is_null() {
        0
    } else {
        typed_call!(
            crt::FAMILY,
            0x00ec_6130,
            crt::string::strlen(e, text.addr())
        )
    };
    let [low, high, _, _] = length.to_le_bytes();
    let count = u16::from_le_bytes([low, high]);
    append_local(e, this, u32::from(count), 2);
    if count != 0 {
        typed_call!(
            FAMILY,
            0x0086_5570,
            append(e, this, text, u32::from(count), 0)
        );
    }
}

// Typed from 00865f20
/// `BGSSaveGameBuffer::StartVariableSizedValue` (Xbox PDB): `#` and its separator written at the
/// write position; answers the position before.
pub fn start_variable_value(e: &mut Engine, this: Ptr) -> u32 {
    let start = e.mem.u32(this.addr().wrapping_add(POSITION));
    e.with_stack(4, |e, marker| {
        e.mem.set_u8(marker.addr(), b'#');
        let position = e.mem.u32(this.addr().wrapping_add(POSITION));
        typed_call!(
            FAMILY,
            0x0086_5ce0,
            write_at(e, this, marker, 1, position, 0)
        );
    });
    start
}

// Typed from 00866140
/// Room for `extra` bytes at `position` (signed compares, as the exe's): nothing when the new end
/// is not above 0; the buffer grown while the end passes its size; unless sizing, the bytes from
/// the position to the write position moved up (`008661e0`); the write position becomes the end.
pub fn insert_space(e: &mut Engine, this: Ptr, position: u32, extra: u32) {
    let t = this.addr();
    let end = e.mem.u32(t.wrapping_add(POSITION)).wrapping_add(extra);
    // The exe's compares here are signed.
    #[allow(clippy::cast_possible_wrap)]
    let signed = |w: u32| w as i32;
    if signed(end) <= 0 {
        return;
    }
    while signed(end) > signed(e.mem.u32(t.wrapping_add(SIZE))) {
        typed_call!(FAMILY, 0x0086_60b0, grow(e, this));
    }
    if !sizing(e) {
        let buffer = e.mem.u32(t.wrapping_add(BUFFER));
        let source = buffer.wrapping_add(position);
        let destination = source.wrapping_add(extra);
        let capacity = e
            .mem
            .u32(t.wrapping_add(SIZE))
            .wrapping_sub(position.wrapping_add(extra));
        let size = e.mem.u32(t.wrapping_add(POSITION)).wrapping_sub(position);
        e.call(MOVE_BYTES, &args![destination, capacity, source, size]);
    }
    e.mem.set_u32(t.wrapping_add(POSITION), end);
}

// Typed from 00865ff0
/// `BGSSaveGameBuffer::SaveVariableSizedValue_ov2` (Xbox PDB): the placeholder at `start` patched
/// with `value` tagged as above (room made first for two or four bytes), each with its separator;
/// a value too large only gives the warning.
pub fn finish_variable_value(e: &mut Engine, this: Ptr, value: u32, start: u32) {
    e.with_stack(4, |e, encoded| {
        if value < 0x40 {
            e.mem.set_u8(encoded.addr(), (value << 2).to_le_bytes()[0]);
            typed_call!(FAMILY, 0x0086_5ce0, write_at(e, this, encoded, 1, start, 0));
        } else if value < 0x4000 {
            typed_call!(FAMILY, 0x0086_6140, insert_space(e, this, start, 1));
            let [low, high, _, _] = ((value << 2) | 1).to_le_bytes();
            e.mem
                .set_u16(encoded.addr(), u16::from_le_bytes([low, high]));
            typed_call!(FAMILY, 0x0086_5ce0, write_at(e, this, encoded, 2, start, 0));
        } else if value < 0x4000_0000 {
            typed_call!(FAMILY, 0x0086_6140, insert_space(e, this, start, 3));
            e.mem.set_u32(encoded.addr(), (value << 2) | 2);
            typed_call!(FAMILY, 0x0086_5ce0, write_at(e, this, encoded, 4, start, 0));
        } else {
            e.call(SAVE_GAME_WARNING, &args![VALUE_TOO_LARGE_FORMAT, value]);
        }
    });
}

// Typed from 00865c80
/// `BGSSaveGameBuffer::Save_ov2` (Xbox PDB): `size` bytes of the buffer to the file
/// (`00846330`); while sizing with no source and only a size that is not 0, else only with a
/// block and a size that is not 0.
pub fn save_part(e: &mut Engine, this: Ptr, file: Ptr, size: u32) {
    if sizing(e) {
        if size != 0 {
            typed_call!(FAMILY, 0x0084_6330, file_write(e, file, 0, size));
        }
    } else {
        let buffer = e.mem.u32(this.addr().wrapping_add(BUFFER));
        if buffer != 0 && size != 0 {
            typed_call!(FAMILY, 0x0084_6330, file_write(e, file, buffer, size));
        }
    }
}

// Typed from 00865c40
/// `BGSSaveGameBuffer::Save` (Xbox PDB): the write position's four bytes to the file
/// (`SaveGameDataToFileFunction`), then that many bytes of the buffer.
pub fn save(e: &mut Engine, this: Ptr, file: Ptr) {
    e.call(
        SAVE_DATA_TO_FILE,
        &args![file, this.addr().wrapping_add(POSITION), 4u32],
    );
    let size = e.mem.u32(this.addr().wrapping_add(POSITION));
    typed_call!(FAMILY, 0x0086_5c80, save_part(e, this, file, size));
}

// Typed from 00846330
/// `BGSSaveLoadFile::Write` (Xbox PDB): while sizing, the simulated size grown to cover the write
/// and the simulated position moved on (unsigned, wrapping), answering `size`; else the real
/// file's write (`00473180`).
pub fn file_write(e: &mut Engine, this: Ptr, buffer: u32, size: u32) -> u32 {
    let t = this.addr();
    if sizing(e) {
        let file_size = e.mem.u32(t.wrapping_add(SIMULATED_SIZE));
        let remaining = file_size.wrapping_sub(e.mem.u32(t.wrapping_add(SIMULATED_POSITION)));
        if remaining < size {
            let file_size = e.mem.u32(t.wrapping_add(SIMULATED_SIZE));
            e.mem.set_u32(
                t.wrapping_add(SIMULATED_SIZE),
                size.wrapping_sub(remaining).wrapping_add(file_size),
            );
        }
        let position = e
            .mem
            .u32(t.wrapping_add(SIMULATED_POSITION))
            .wrapping_add(size);
        e.mem.set_u32(t.wrapping_add(SIMULATED_POSITION), position);
        size
    } else {
        let file = e.mem.u32(t.wrapping_add(FILE));
        e.call(FILE_WRITE, &args![file, buffer, size]).u32()
    }
}
