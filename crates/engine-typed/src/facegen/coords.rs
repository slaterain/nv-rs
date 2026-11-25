//! The sum of two face coordinate blocks (`00652af0`): what the race's face data and the face
//! menu's slider refresh build a face's coordinates with. The race menu's slider refresh
//! (`007b16d0`) runs it once a slider, about 150 times a click on a preset or a hair (a 2026-11-24
//! profile of the menu), and each element of each matrix went through four or five calls by
//! address in the translation (the vector's size, its capacity, the uninitialised fill). Here
//! the elements of a matrix are summed and appended in one loop with plain loads and stores when
//! nothing can differ from the exe's steps; else element by element as the exe goes.
//!
//! What the translation reaches with plain Rust calls (`BSFaceGenManager::CopyFaceGenCoord`
//! `006529a0`, `FR2MatrixVTC<float>::operator=` `006581e0`, the vector's checked element `006578b0`
//! and its append `006578e0`) runs here in place, with the same calls by address; the vector's
//! size (`00662820`), capacity (`0060b860`) and one-float fill (`0060b8a0`), which only read the
//! vector's words or store the float, are plain loads and stores.
//!
//! **Floats.** The block sums are reached on main and on loading threads (a head made for an NPC),
//! so the family's [`Threads::Both`] holds: each sum keeps the translation's x87 steps (one
//! float loaded, the other added, the result stored) at the running thread's precision.
//!
//! [`Threads::Both`]: engine_core::typed::Threads::Both

// The float tests are NaN-aware as the exe's compares are (a NaN takes the `else` side).
#![allow(clippy::neg_cmp_op_on_partial_ord)]

use engine_core::x87::{x87_add, x87_div, x87_load32, x87_m32, x87_quiet_bits, x87_store32};
use engine_core::{args, Engine};

/// A matrix's dimensions (`thiscall`: the words at `+0x18` and `+0x1c`) and its resize
/// (`thiscall`: rows, columns).
const MATRIX_ROWS: u32 = 0x0096_11e0;
const MATRIX_COLUMNS: u32 = 0x0044_1110;
const MATRIX_RESIZE: u32 = 0x0060_b340;
/// `FR2MatrixVTC<float>::operator=` (`thiscall`: the source), which the copy calls by address.
const MATRIX_ASSIGN: u32 = 0x0065_81e0;
/// The storage's resize in the assignment (`thiscall`: the count) and `memcpy` (cdecl).
const STORAGE_RESIZE: u32 = 0x0060_b3f0;
const MEMCPY: u32 = 0x0040_1460;
/// The float vector's clear (`thiscall`).
const VECTOR_CLEAR: u32 = 0x0044_a0a0;
/// The append past the capacity: the end iterator (`thiscall`: its result) and the insert
/// (`thiscall`: result, the iterator's two words, the value's address).
const VECTOR_END: u32 = 0x0065_f550;
const VECTOR_INSERT: u32 = 0x0065_7c50;
/// `_invalid_parameter_noinfo`: the checked element's call past the end.
const INVALID_PARAMETER: u32 = 0x00ec_7c56;
/// The limit's steps: the matrix's length (`thiscall`, `ST0`), the square root (cdecl: a float,
/// `ST0`), the matrix's scale (`thiscall`: a float); the `double` 0.0 the limit is compared with.
const MATRIX_LENGTH: u32 = 0x0065_88a0;
const SQUARE_ROOT: u32 = 0x0040_19b0;
const MATRIX_SCALE: u32 = 0x0065_8650;
const ZERO: u32 = 0x0101_2060;

/// A matrix's words: its dimensions, and its `std::vector<float>`'s first, last and end of storage.
const ROWS: u32 = 0x18;
const COLUMNS: u32 = 0x1c;
const FIRST: u32 = 0xc;
const LAST: u32 = 0x10;
const END: u32 = 0x14;

// Typed from 00652af0
/// Combines two coordinate blocks into `destination` (cdecl, five words: the blocks, the
/// destination, a byte and a `float` limit). Nothing without a destination or without both
/// sources; with one source it is copied (`BSFaceGenManager::CopyFaceGenCoord` with 0, 0).
/// Otherwise each of the 2 x 2 matrices takes the first source's size (or 0 by 0 when that has
/// none); row 1 with the byte set takes the first source's values, the others the element-wise
/// sums of both, each matrix then limited.
pub fn sum_coordinates(
    e: &mut Engine,
    first: u32,
    second: u32,
    destination: u32,
    copy_row_1: u8,
    limit: f32,
) {
    if destination == 0 || (first == 0 && second == 0) {
        return;
    }
    if first == 0 {
        copy_coordinates(e, second, destination, 0, 0);
        return;
    }
    if second == 0 {
        copy_coordinates(e, first, destination, 0, 0);
        return;
    }
    let limit = limit.to_bits();
    for row in 0..2u32 {
        for column in 0..2u32 {
            let offset = row * 0x40 + column * 0x20;
            let from = first.wrapping_add(offset);
            let other = second.wrapping_add(offset);
            let to = destination.wrapping_add(offset);
            let rows = e.call(MATRIX_ROWS, &args![from]).u32();
            let columns = e.call(MATRIX_COLUMNS, &args![from]).u32();
            if rows == 0 || columns == 0 {
                e.call(MATRIX_RESIZE, &args![to, 0u32, 0u32]);
                continue;
            }
            e.call(MATRIX_RESIZE, &args![to, rows, columns]);
            if row == 1 && copy_row_1 != 0 {
                assign(e, to, from);
                limit_matrix(e, to, rows, limit);
            } else {
                let count = rows.wrapping_mul(columns);
                e.call(VECTOR_CLEAR, &args![to]);
                if !sum_in_place(e, from, other, to, count) {
                    sum_by_element(e, from, other, to, count);
                }
                limit_matrix(e, to, rows, limit);
            }
        }
    }
}

/// `BSFaceGenManager::CopyFaceGenCoord` (`006529a0`, which the translation calls plainly): each
/// matrix of `source` with both dimensions takes them in `destination`, and its values when
/// `first` is set and the row is 0, `second` is set and the row is 1, or the two bytes are equal;
/// any other matrix becomes 0 by 0.
fn copy_coordinates(e: &mut Engine, source: u32, destination: u32, first: u8, second: u8) {
    if source == 0 || destination == 0 {
        return;
    }
    for row in 0..2u32 {
        for column in 0..2u32 {
            let from = source + row * 0x40 + column * 0x20;
            let to = destination + row * 0x40 + column * 0x20;
            if e.call(MATRIX_ROWS, &args![from]).u32() != 0
                && e.call(MATRIX_COLUMNS, &args![from]).u32() != 0
            {
                let columns = e.call(MATRIX_COLUMNS, &args![from]).u32();
                let rows = e.call(MATRIX_ROWS, &args![from]).u32();
                e.call(MATRIX_RESIZE, &args![to, rows, columns]);
                if (first != 0 && row == 0) || (second != 0 && row == 1) || first == second {
                    e.call(MATRIX_ASSIGN, &args![to, from]);
                }
            } else {
                e.call(MATRIX_RESIZE, &args![to, 0u32, 0u32]);
            }
        }
    }
}

/// `FR2MatrixVTC<float>::operator=` (`006581e0`, called plainly here by the translation): unless
/// it is itself, `this` takes the source's dimensions, sizes its storage (`0060b3f0` with their
/// product) and copies the values (`memcpy` of the product times 4 bytes, from element 0 of each).
fn assign(e: &mut Engine, this: u32, source: u32) {
    if this == source {
        return;
    }
    let rows = e.mem.u32(source.wrapping_add(ROWS));
    e.mem.set_u32(this.wrapping_add(ROWS), rows);
    let columns = e.mem.u32(source.wrapping_add(COLUMNS));
    e.mem.set_u32(this.wrapping_add(COLUMNS), columns);
    let count = e
        .mem
        .u32(this.wrapping_add(ROWS))
        .wrapping_mul(e.mem.u32(this.wrapping_add(COLUMNS)));
    e.call(STORAGE_RESIZE, &args![this, count]);
    let bytes = e
        .mem
        .u32(this.wrapping_add(ROWS))
        .wrapping_mul(e.mem.u32(this.wrapping_add(COLUMNS)))
        << 2;
    let from = element(e, source, 0);
    let to = element(e, this, 0);
    e.call(MEMCPY, &args![to, from, bytes]);
}

/// A float vector's element count as `00662820` computes it (signed, `SAR` by 2) and its
/// capacity as `0060b860` does (0 without storage), both as the unsigned words their callers
/// compare.
fn size_and_capacity(e: &Engine, vector: u32) -> (u32, u32) {
    let first = e.mem.u32(vector.wrapping_add(FIRST));
    let last = e.mem.u32(vector.wrapping_add(LAST));
    let end = e.mem.u32(vector.wrapping_add(END));
    let size = ((last.wrapping_sub(first) as i32) >> 2) as u32;
    let capacity = if first == 0 {
        0
    } else {
        ((end.wrapping_sub(first) as i32) >> 2) as u32
    };
    (size, capacity)
}

/// The vector's checked element (`006578b0`): `_invalid_parameter_noinfo` when `index` is not
/// below the size, then the address of float `index`.
fn element(e: &mut Engine, vector: u32, index: u32) -> u32 {
    let (size, _) = size_and_capacity(e, vector);
    if index >= size {
        e.call(INVALID_PARAMETER, &args![]);
    }
    e.mem
        .u32(vector.wrapping_add(FIRST))
        .wrapping_add(index.wrapping_mul(4))
}

/// The vector's append (`006578e0`) of the float at `value`: within the capacity stored at the end
/// (`0060b8a0`'s one-float fill) and the end moved past it, else inserted at the end iterator
/// (`0065f550`, `00657c50`) in the translation's frame.
fn append(e: &mut Engine, vector: u32, value: u32) {
    let (size, capacity) = size_and_capacity(e, vector);
    if size < capacity {
        let end = e.mem.u32(vector.wrapping_add(LAST));
        let float = e.mem.u32(value);
        e.mem.set_u32(end, float);
        e.mem
            .set_u32(vector.wrapping_add(LAST), end.wrapping_add(4));
        return;
    }
    e.with_stack(0x10, |e, slots| {
        let end_slot = slots.addr();
        let result = end_slot + 8;
        let end = e.call(VECTOR_END, &args![vector, end_slot]).u32();
        let first = e.mem.u32(end);
        let second = e.mem.u32(end.wrapping_add(4));
        e.call(VECTOR_INSERT, &args![vector, result, first, second, value]);
    });
}

/// One element's sum as the translation makes it: the first source's float loaded, the second's
/// added, the result stored as a float.
#[inline]
fn summed(a: u32, b: u32) -> u32 {
    x87_store32(x87_add(
        x87_load32(x87_quiet_bits(a)),
        x87_m32(x87_quiet_bits(b)),
    ))
}

/// Whether `[a, a + a_len)` and `[b, b + b_len)` share a byte (64-bit ends: no wrapping).
fn overlaps(a: u64, a_len: u64, b: u64, b_len: u64) -> bool {
    a_len != 0 && b_len != 0 && a < b + b_len && b < a + a_len
}

/// The element loop in one pass: `count` sums of the two sources' floats appended to `to`.
/// Answers false, having read and written nothing, unless every checked element is within its
/// source (no `_invalid_parameter_noinfo`), every append is within `to`'s capacity (none
/// inserts), the floats it writes overlap nothing the loop reads (both sources' floats, the
/// three vectors' words) and nothing passes 4 GB.
fn sum_in_place(e: &mut Engine, from: u32, other: u32, to: u32, count: u32) -> bool {
    if count == 0 {
        return false;
    }
    let (from_size, _) = size_and_capacity(e, from);
    let (other_size, _) = size_and_capacity(e, other);
    let (to_size, to_capacity) = size_and_capacity(e, to);
    // Each append tests `size < capacity` with the size one more each time.
    if count > from_size
        || count > other_size
        || u64::from(to_size) + u64::from(count) > u64::from(to_capacity)
    {
        return false;
    }
    let bytes = 4 * u64::from(count);
    let from_data = e.mem.u32(from.wrapping_add(FIRST));
    let other_data = e.mem.u32(other.wrapping_add(FIRST));
    let written = e.mem.u32(to.wrapping_add(LAST));
    let limit = 1u64 << 32;
    if [from_data, other_data, written]
        .iter()
        .any(|&at| u64::from(at) + bytes > limit)
    {
        return false;
    }
    let read = [
        (u64::from(from_data), bytes),
        (u64::from(other_data), bytes),
        (u64::from(from.wrapping_add(FIRST)), 8),
        (u64::from(other.wrapping_add(FIRST)), 8),
        (u64::from(to.wrapping_add(FIRST)), 12),
    ];
    if read
        .iter()
        .any(|&(at, len)| overlaps(u64::from(written), bytes, at, len))
    {
        return false;
    }
    let n = count as usize;
    let mut a = vec![0u8; 4 * n];
    let mut b = vec![0u8; 4 * n];
    e.mem.read(from_data, &mut a);
    e.mem.read(other_data, &mut b);
    let mut out = vec![0u8; 4 * n];
    for ((sum, x), y) in out
        .chunks_exact_mut(4)
        .zip(a.chunks_exact(4))
        .zip(b.chunks_exact(4))
    {
        let x = u32::from_le_bytes([x[0], x[1], x[2], x[3]]);
        let y = u32::from_le_bytes([y[0], y[1], y[2], y[3]]);
        sum.copy_from_slice(&summed(x, y).to_le_bytes());
    }
    e.mem.write(written, &out);
    e.mem.set_u32(
        to.wrapping_add(LAST),
        written.wrapping_add(count.wrapping_mul(4)),
    );
    true
}

/// The element loop as the translation goes: each source's checked element, the sum, and its
/// append from a stack word.
fn sum_by_element(e: &mut Engine, from: u32, other: u32, to: u32, count: u32) {
    for index in 0..count {
        let a = element(e, from, index);
        let a = e.mem.u32(a);
        let b = element(e, other, index);
        let b = e.mem.u32(b);
        let sum = summed(a, b);
        e.with_stack(4, |e, slot| {
            e.mem.set_u32(slot.addr(), sum);
            append(e, to, slot.addr());
        });
    }
}

/// The coordinate scaling of `00652af0`: when `limit` is above 0.0 and below the square root of
/// the matrix's length over its rows, the matrix is scaled by their quotient.
fn limit_matrix(e: &mut Engine, matrix: u32, rows: u32, limit: u32) {
    let zero: f64 = e.global(ZERO);
    if !(x87_load32(limit) > zero) {
        return;
    }
    let length = e.call(MATRIX_LENGTH, &args![matrix]).f64();
    let quotient = x87_store32(x87_div(length, f64::from(rows)));
    let root = e.call(SQUARE_ROOT, &args![quotient]).f64();
    let root = x87_store32(root);
    if x87_load32(limit) < x87_load32(root) {
        let scale = x87_store32(x87_div(x87_load32(limit), x87_m32(root)));
        e.call(MATRIX_SCALE, &args![matrix, scale]);
    }
}
