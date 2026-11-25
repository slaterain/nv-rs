//! The face generation matrices' product (`FR2MatrixVTC<float>`'s multiply, `00658360`) and the
//! face shape statistic built on it (`00652230`): what the race menu's slider refresh (`007b16d0`,
//! through `RaceSexMenu`'s face shape values) and the face controls (`FanControls`) compute a
//! face's slider positions with, several hundred products a click on a preset or a hair (a
//! 2026-11-24 profile of the menu put about 22,000 row and element getter calls a click under the
//! statistic).
//!
//! The translation reaches each element through `operator[]` (`0060b370`: the column count, the
//! checked first element `006578b0` with the vector's size `00662820`, the row view `004b0680`)
//! and the row's element (`00629ab0`), four calls by address for every element read or written,
//! three elements a step. Here each element's address is computed in place from the same words,
//! read in the same order and at the same points between the float reads and writes, with the
//! checked element's `_invalid_parameter_noinfo` call where it comes (a matrix with no elements).
//! The row view, which `operator[]` writes into the caller's frame and the element reads back,
//! stays in host locals. Everything else is called by address as the exe calls it: the product
//! matrix's constructor (`00659000`), the copy into the result (`00658f30`), the destructor
//! (`00449680`), `fr2Assert` for mismatched sizes; the statistic's manager (`BSFaceGenManager::
//! GetInstance`, `00651b30`, when there is none yet), its entry count and checked entry
//! (`00657a30`, `00657a50`, in place: they only read the vector's words, the entry's check calls
//! `_invalid_parameter_noinfo` as it does).
//!
//! **Floats.** Products are made on main (the race menu) and on loading threads (an NPC's head),
//! so the family's [`Threads::Both`] holds: each step keeps the exe's `FLD`, `FMUL`, `FADD`,
//! `FSTP` with the exact x87 helpers at the running thread's precision, the sum stored to the
//! element after every step as the exe stores it.
//!
//! [`Threads::Both`]: engine_core::typed::Threads::Both

use engine_core::ptr::Ptr;
use engine_core::x87::{x87_add, x87_load32, x87_m32, x87_mul, x87_quiet_bits, x87_store32};
use engine_core::{args, typed_call, Engine};

/// A matrix's words: its `std::vector<float>`'s first and last, its dimensions.
const FIRST: u32 = 0xc;
const LAST: u32 = 0x10;
const ROWS: u32 = 0x18;
const COLUMNS: u32 = 0x1c;
/// The product matrix's constructor from a row and a column count (`thiscall`, two words), the
/// copy constructor (`thiscall`, the source) and the destructor (`thiscall`).
const MATRIX_CONSTRUCT: u32 = 0x0065_9000;
const MATRIX_COPY: u32 = 0x0065_8f30;
const MATRIX_DESTRUCT: u32 = 0x0044_9680;
/// `fr2Assert` (cdecl: the file name, the line) with the matrix header's name and its line 0x113.
const FR2_ASSERT: u32 = 0x00aa_fb90;
const MATRIX_FILE: u32 = 0x0106_5830;
/// `_invalid_parameter_noinfo`: a checked getter's call past the end.
const INVALID_PARAMETER: u32 = 0x00ec_7c56;
/// `BSFaceGenManager::GetInstance` and the manager it answers (`011d59e8`).
const GET_INSTANCE: u32 = 0x0065_1b30;
const MANAGER: u32 = 0x011d_59e8;

/// The address of element (`row`, `column`) of the matrix at `matrix`, as `operator[]` (`0060b370`)
/// and the row's element (`00629ab0`) give it: the vector's first element checked (`006578b0`:
/// `_invalid_parameter_noinfo` when the vector's size, its bytes over 4 shifted signed, is 0 as an
/// unsigned word), then `first + 4 * (row * columns) + 4 * column`, wrapping. The words are read
/// before and after the check as the exe reads them.
fn element(e: &mut Engine, matrix: u32, row: u32, column: u32) -> u32 {
    let begin = e.mem.u32(matrix.wrapping_add(FIRST));
    let end = e.mem.u32(matrix.wrapping_add(LAST));
    if (end.wrapping_sub(begin) as i32) >> 2 == 0 {
        e.call(INVALID_PARAMETER, &args![]);
    }
    let data = e.mem.u32(matrix.wrapping_add(FIRST));
    let columns = e.mem.u32(matrix.wrapping_add(COLUMNS));
    data.wrapping_add(row.wrapping_mul(columns).wrapping_mul(4))
        .wrapping_add(column.wrapping_mul(4))
}

// Typed from 00658360
/// The matrix product of `this` and `other` into `result` (`thiscall`, two words). Mismatched
/// inner sizes report `fr2Assert`. A stack matrix of this one's rows by the other's columns gets
/// each element set to 0.0 and then the sum of the products, each step stored; it is copied into
/// `result` and destroyed. The loops' bounds are read again at every test, as the exe's are.
/// Answers `result`.
pub fn product(e: &mut Engine, this: Ptr, result: Ptr, other: Ptr) -> Ptr {
    let a = this.addr();
    let b = other.addr();
    if e.mem.u32(a.wrapping_add(COLUMNS)) != e.mem.u32(b.wrapping_add(ROWS)) {
        e.call(FR2_ASSERT, &args![MATRIX_FILE, 0x113u32]);
    }
    e.with_stack(0x20, |e, product| {
        let product = product.addr();
        let rows = e.mem.u32(a.wrapping_add(ROWS));
        let columns = e.mem.u32(b.wrapping_add(COLUMNS));
        e.call(MATRIX_CONSTRUCT, &args![product, rows, columns]);
        let mut row = 0u32;
        while row < e.mem.u32(a.wrapping_add(ROWS)) {
            let mut column = 0u32;
            while column < e.mem.u32(b.wrapping_add(COLUMNS)) {
                let cell = element(e, product, row, column);
                e.mem.set_u32(cell, 0.0f32.to_bits());
                let mut inner = 0u32;
                while inner < e.mem.u32(a.wrapping_add(COLUMNS)) {
                    let cell = element(e, product, row, column);
                    let left = element(e, a, row, inner);
                    let right = element(e, b, inner, column);
                    let sum = x87_add(
                        x87_mul(x87_load32(e.mem.u32(left)), x87_m32(e.mem.u32(right))),
                        x87_m32(e.mem.u32(cell)),
                    );
                    e.mem.set_u32(cell, x87_store32(sum));
                    inner += 1;
                }
                column += 1;
            }
            row += 1;
        }
        e.call(MATRIX_COPY, &args![result, product]);
        e.call(MATRIX_DESTRUCT, &args![product]);
    });
    result
}

/// The manager (`BSFaceGenManager::GetInstance`, which the translation runs plainly): the word at
/// `011d59e8`, or the call that makes it when there is none yet.
fn manager(e: &mut Engine) -> u32 {
    let manager = e.mem.u32(MANAGER);
    if manager != 0 {
        manager
    } else {
        e.call(GET_INSTANCE, &args![]).u32()
    }
}

/// The manager's statistics vector for `row`, `column` (0x3c-byte entries).
fn statistics(e: &mut Engine, row: u32, column: u32) -> u32 {
    manager(e)
        .wrapping_add(row.wrapping_mul(0x30))
        .wrapping_add(0xb8)
        .wrapping_add(column.wrapping_mul(0x18))
}

/// The entry count of a statistics vector (`00657a30`): its bytes over 0x3c, signed.
fn entry_count(e: &Engine, vector: u32) -> u32 {
    let end = e.mem.u32(vector.wrapping_add(LAST)) as i32;
    let begin = e.mem.u32(vector.wrapping_add(FIRST)) as i32;
    end.wrapping_sub(begin).wrapping_div(0x3c) as u32
}

// Typed from 00652230
/// The face shape statistic `index` of the coordinate block `block` for `row`, `column` (cdecl,
/// four words; the result in `ST0`): 0.0 for a null block or an index past the manager's entries;
/// otherwise entry `index` (checked: `_invalid_parameter_noinfo` past the count, as `00657a50`
/// does) times the block's matrix ([`product`] into a stack matrix), whose element (0, 0) is
/// answered, the stack matrix then destroyed.
pub fn shape_statistic(e: &mut Engine, block: u32, row: u32, column: u32, index: u32) -> f32 {
    let mut result = 0.0f32.to_bits();
    if block != 0 {
        let entries = statistics(e, row, column);
        if index < entry_count(e, entries) {
            let matrix = block
                .wrapping_add(row.wrapping_mul(0x40))
                .wrapping_add(column.wrapping_mul(0x20));
            let entries = statistics(e, row, column);
            if index >= entry_count(e, entries) {
                e.call(INVALID_PARAMETER, &args![]);
            }
            let entry = index
                .wrapping_mul(0x3c)
                .wrapping_add(e.mem.u32(entries.wrapping_add(FIRST)));
            e.with_stack(0x20, |e, slot| {
                let made = typed_call!(
                    super::FAMILY,
                    0x0065_8360,
                    product(e, Ptr::new(entry), slot, Ptr::new(matrix))
                );
                let at = element(e, made.addr(), 0, 0);
                result = x87_quiet_bits(e.mem.u32(at));
                e.call(MATRIX_DESTRUCT, &args![made]);
            });
        }
    }
    f32::from_bits(result)
}
