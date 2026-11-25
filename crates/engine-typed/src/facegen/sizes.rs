//! The byte sizes of a face model's loaded morph data (EGM, `0065c110`) and texture data (EGT,
//! `0065c490`), which the face model map adds up over every model it holds when it checks its
//! memory budget. Each walks the data's two record vectors through small getters, three calls a
//! record; here the walk reads the vectors directly.
//!
//! The getters (thiscall, no state): a vector's count, `(end - begin) / size` as a signed division
//! (`CDQ; IDIV`) of its `+0x10` and `+0xc` words; its element at an index, with that count checked
//! (an index at or past it calls `_invalid_parameter_noinfo`, which the walks never reach: each
//! asks for an index below the count it has just read) and `begin + index * size`; and a vector's
//! byte size, `end - begin`. They read nothing else and write nothing, and a walk writes nothing,
//! so the words the exe reads again at each step are read once here.

use engine_core::{Engine, Ptr};

/// A vector's `begin` and `end` words (`+0xc`, `+0x10`).
#[inline(always)]
fn ends(e: &Engine, vector: u32) -> (u32, u32) {
    (
        e.mem.u32(vector.wrapping_add(0xc)),
        e.mem.u32(vector.wrapping_add(0x10)),
    )
}

/// A vector's count of `size`-byte elements: `(end - begin) / size`, signed (`CDQ; IDIV`).
#[inline(always)]
fn count(e: &Engine, vector: u32, size: i32) -> u32 {
    let (begin, end) = ends(e, vector);
    let bytes = i32::from_ne_bytes(end.wrapping_sub(begin).to_ne_bytes());
    u32::from_ne_bytes((bytes / size).to_ne_bytes())
}

/// The walk both sizes are: the holder at `this + holder` (0 without one), its data at `+8` (0
/// without), then 0x34, and for each of the data's two vectors (`+4`, `+0x1c`) of `record`-byte
/// records, `per_record` a record and `inner` of each record.
#[inline(always)]
fn walk(
    e: &Engine,
    this: Ptr,
    holder: u32,
    record: u32,
    per_record: u32,
    inner: impl Fn(&Engine, u32) -> u32,
) -> u32 {
    let holder = e.mem.u32(this.addr().wrapping_add(holder));
    if holder == 0 {
        return 0;
    }
    let data = e.mem.u32(holder.wrapping_add(8));
    if data == 0 {
        return 0;
    }
    let mut total = 0x34u32;
    for list in 0..2u32 {
        let vector = data.wrapping_add(list * 0x18).wrapping_add(4);
        let n = count(e, vector, i32::from_ne_bytes(record.to_ne_bytes()));
        total = n.wrapping_mul(per_record).wrapping_add(total);
        let mut at = e.mem.u32(vector.wrapping_add(0xc));
        for _ in 0..n {
            total = inner(e, at).wrapping_add(total);
            at = at.wrapping_add(record);
        }
    }
    total
}

// Typed from 0065c110
/// The bytes of a model's loaded EGM morph data (`thiscall`; the model data at `this + 8`, its
/// morph data at `+8`): 0 without either; else 0x34, and for each of its two morph vectors
/// (0x1c-byte morphs) 0x18 a morph and 6 for each 6-byte vertex offset in the morph's vector
/// (`+4`).
pub fn egm_size(e: &mut Engine, this: Ptr) -> u32 {
    walk(e, this, 8, 0x1c, 0x18, |e, morph| {
        count(e, morph.wrapping_add(4), 6).wrapping_mul(6)
    })
}

// Typed from 0065c490
/// The bytes of a model's loaded EGT texture data (`thiscall`; the texture data at `this + 0xc`,
/// its EGT data at `+8`): 0 without either; else 0x34, and for each of its two vectors of 0x58-byte
/// records 0x58 a record and three times the byte size of the record's vector at `+0x10`.
pub fn egt_size(e: &mut Engine, this: Ptr) -> u32 {
    walk(e, this, 0xc, 0x58, 0x58, |e, record| {
        let (begin, end) = ends(e, record.wrapping_add(0x10));
        end.wrapping_sub(begin).wrapping_mul(3)
    })
}
