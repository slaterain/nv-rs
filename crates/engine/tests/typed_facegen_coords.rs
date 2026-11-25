//! The differential test of the face family's coordinate sum (`engine_typed::facegen::coords`,
//! `00652af0`) against its translation, on real face coordinate blocks: each block's four
//! matrices made by their constructor (`00449610`) and sized by the game's resize (`0060b340`),
//! every helper the sum reaches the game's own translation (the copy, the assignment, the clear,
//! the storage's resize, `memcpy`, the limit's length and scale) but the square root, a double:
//! its CRT error path for a NaN or a negative length reads CRT words not mapped here.
//!
//! The translation asks the float vector's size (`00662820`), capacity (`0060b860`) and one-float
//! fill (`0060b8a0`) for each element; the typed sum reads and stores those words itself, so those
//! three are left out of the call comparison (`ignore_calls`). Every other call, every byte of
//! memory and the heap compare; `_invalid_parameter_noinfo` is a counter of the test's own (the
//! CRT's handler words are not mapped here). The cases take the shapes a face's blocks have (50 by
//! 1 and 30 by 1, as `InitFaceGenCoord` sizes them, and their transposes), matrices with one
//! dimension 0, small odd ones, a second block shorter than the first (each missing element a
//! checked element's `_invalid_parameter_noinfo`), a destination that is one of the sources, the
//! copy of row 1, a single source, limits that scale, do not, or are NaN, and values that are
//! quiet and signalling NaNs.

use engine as _;
use engine_core::prelude::*;
use engine_core::typed::diff::{Case, Diff, Input};
use engine_core::typed::leaf;
use engine_typed::facegen::{self, coords};

/// `FR2MatrixVTC<float>`'s constructor (`thiscall`) and resize (`thiscall`: rows, columns).
const MATRIX_CONSTRUCT: u32 = 0x0044_9610;
const MATRIX_RESIZE: u32 = 0x0060_b340;

/// The per-element callees the typed sum does not call (see the module's words).
const IGNORED: [u32; 3] = [0x0066_2820, 0x0060_b860, 0x0060_b8a0];

/// The first block's four matrices' dimensions, by shape.
const SHAPES: [[(u32, u32); 4]; 5] = [
    [(50, 1), (30, 1), (50, 1), (0, 0)],
    [(1, 50), (1, 30), (1, 50), (0, 0)],
    [(1, 3), (2, 2), (0, 4), (1, 1)],
    [(3, 2), (1, 0), (2, 1), (1, 4)],
    [(1, 1), (1, 1), (1, 1), (1, 1)],
];

/// Float words a face's coefficients take, with the awkward ones among them.
fn value(i: u32, k: u32, pattern: u32) -> u32 {
    let n = i
        .wrapping_mul(2_654_435_761)
        .wrapping_add(k * 97 + pattern * 13);
    match n % 23 {
        0 => 0x8000_0000,                  // -0.0
        1 => 0,                            // 0.0
        2 => 0x0000_0001,                  // the smallest subnormal
        3 => 0x7f7f_ffff,                  // the largest float
        4 if pattern == 1 => 0x7fc0_0000,  // a quiet NaN
        5 if pattern == 1 => 0xff80_0000,  // negative infinity
        10 if pattern == 1 => 0xffc0_0042, // a quiet NaN of another payload
        // A signalling NaN: the second block's word wherever the first block's is the quiet NaN
        // (its salt moves the category on by 12), so a sum meets both NaNs, the signalling one
        // quieted first: the larger payload wins.
        16 if pattern == 1 => 0x7f80_0001,
        _ => ((n % 2001) as f32 / 97.0 - 10.0).to_bits(),
    }
}

/// A block of four matrices of `shape`, each `extra` elements longer (negative: shorter, as far as
/// its count allows), filled with `pattern`'s values.
fn block(e: &mut Engine, shape: &[(u32, u32); 4], extra: i32, salt: u32, pattern: u32) -> u32 {
    let block = e.mem.alloc(0x80);
    for (k, &(rows, columns)) in (0u32..).zip(shape.iter()) {
        let matrix = block + 0x20 * k;
        e.call(MATRIX_CONSTRUCT, &args![matrix]);
        let count = (rows * columns) as i32 + extra;
        if rows != 0 && columns != 0 && count > 0 {
            // Sized by the resize, then the dimensions put back to the first block's: a longer or
            // shorter vector under the same matrix.
            e.call(MATRIX_RESIZE, &args![matrix, 1u32, count as u32]);
            e.mem.set_u32(matrix + 0x18, rows);
            e.mem.set_u32(matrix + 0x1c, columns);
            let data = e.mem.u32(matrix + 0xc);
            for i in 0..count as u32 {
                e.mem.set_u32(data + 4 * i, value(i, k + salt, pattern));
            }
        } else if rows != 0 && columns != 0 {
            // None left: storage of one element, the vector empty (a checked element past its end
            // reads that storage, as a face block always has some).
            e.call(MATRIX_RESIZE, &args![matrix, 1u32, 1u32]);
            e.mem.set_u32(matrix + 0x18, rows);
            e.mem.set_u32(matrix + 0x1c, columns);
            let data = e.mem.u32(matrix + 0xc);
            e.mem.set_u32(matrix + 0x10, data);
        } else {
            // A dimension 0: no storage, the shape's dimensions as they are (1 by 0 is not 0 by 0).
            e.mem.set_u32(matrix + 0x18, rows);
            e.mem.set_u32(matrix + 0x1c, columns);
        }
    }
    block
}

#[test]
fn facegen_coords_sum() {
    Diff::new(entry!(
        0x0065_2af0,
        coords::sum_coordinates(u32, u32, u32, u8, f32)
    ))
    .family(&facegen::FACEGEN)
    .family(&leaf::LEAVES)
    .ignore_calls(&IGNORED)
    .inputs(&[
        Input::Of(&[0, 1, 2, 3, 4]),
        // The second block's length against the first's: the same, 3 or 20 longer, one shorter.
        Input::Of(&[0, 1, 2, 3]),
        // The destination: a new block, a used one (longer, so its storage is reused), the first
        // block, the second; then no first, no second, no destination.
        Input::Of(&[0, 1, 2, 3, 4, 5, 6]),
        Input::Flag,
        // The limit: none, scaling, too large to scale, a NaN, negative.
        Input::Of(&[0, 0x4000_0000, 0x4f00_0000, 0x7fc0_0000, 0xbf80_0000]),
        Input::Of(&[0, 1]),
    ])
    .setup(|e, case: &Case| {
        // The globals the helpers read: the limit's 0.0 and the SSE2 flag; the invalid-parameter
        // report a counter of its own (the CRT's handler words are not mapped here).
        e.map(0x0101_2000, 0x1000);
        e.mem.set_f64(0x0101_2060, 0.0);
        e.map(0x0127_0000, 0x1000);
        e.mem.set_u32(0x0127_0a6c, 1);
        e.map(0x0300_0000, 0x1000);
        // The square root (`0040_19b0`, called by address on both sides) as a double: its CRT error
        // path for a NaN or a negative length reads CRT words this test does not map.
        e.register(0x0040_19b0, |_, a| Ret {
            st0: f64::from(f32::from_bits(a[0])).sqrt(),
            ..Ret::default()
        });
        e.register(0x00ec_7c56, |e, _| {
            let n = e.mem.u32(0x0300_0000);
            e.mem.set_u32(0x0300_0000, n + 1);
            Ret::default()
        });
        let shape = SHAPES[case.word(0) as usize];
        let pattern = case.word(5);
        let first = block(e, &shape, 0, 0, pattern);
        let extra = match case.word(1) {
            0 => 0,
            1 => 3,
            2 => -1,
            _ => 20,
        };
        let second = block(e, &shape, extra, 7, pattern);
        let destination = match case.word(2) {
            0 => {
                let empty = [(0, 0); 4];
                block(e, &empty, 0, 0, pattern)
            }
            1 => block(e, &[(2, 40), (1, 31), (3, 3), (1, 2)], 0, 3, pattern),
            2 => first,
            3 => second,
            _ => block(e, &[(0, 0); 4], 0, 0, pattern),
        };
        let (first, second, destination) = match case.word(2) {
            4 => (0, second, destination),
            5 => (first, 0, destination),
            6 => (first, second, 0),
            _ => (first, second, destination),
        };
        vec![first, second, destination, case.word(3), case.word(4)]
    })
    .max_cases(400)
    .max_calls(200_000)
    .run()
    .assert_same();
}
