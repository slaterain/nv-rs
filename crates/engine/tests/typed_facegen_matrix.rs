//! The differential test of the face family's matrix product (`engine_typed::facegen::matrix`,
//! `00658360`) and the face shape statistic on it (`00652230`) against their translations, on real
//! `FR2MatrixVTC<float>` matrices: made by their constructor (`00449610`) and sized by the game's
//! resize (`0060b340`); the product's own matrix, its copy into the result and its destructor are
//! the game's translations on both sides.
//!
//! The translation reaches every element through `operator[]` (`0060b370`, with the checked first
//! element `006578b0`, the vector's size `00662820` and the row view `004b0680`) and the row's
//! element (`00629ab0`); the typed product computes each address itself, so those five are left
//! out of the call comparison (`ignore_calls`). Every other call (`fr2Assert` for mismatched sizes,
//! the constructor, the copy, the destructor, the allocations), every byte of memory beside the
//! stack and the heap compare, under both control words. A matrix whose vector is empty fails the
//! checked element (`_invalid_parameter_noinfo`, a counter of the test's own); the Diff leaves
//! that call out with the getters, so those cases run on two engines made alike and compare the
//! counts, the answer, the heap and memory ([`both_ways`]).
//!
//! The shapes are the statistic's (an entry of 1 by 50 or 1 by 30 times a block's 50 by 1 or
//! 30 by 1) and small ones of every kind: inner sizes that differ (`fr2Assert`, then the product of
//! what is there), a dimension 0, vectors longer or shorter than their dimensions (the exe reads
//! past a short one's end). Values: signed zeros, subnormals, the largest float, infinities,
//! quiet and signalling NaNs, ordinary ones.

use engine as _;
use engine_core::prelude::*;
use engine_core::typed::diff::{
    memory_differences_beside_stack, memory_digest_beside_stack, Case, Diff, Input,
};
use engine_core::typed::leaf;
use engine_typed::facegen::{self, matrix};

/// `FR2MatrixVTC<float>`'s constructor (`thiscall`) and resize (`thiscall`: rows, columns).
const MATRIX_CONSTRUCT: u32 = 0x0044_9610;
const MATRIX_RESIZE: u32 = 0x0060_b340;
/// The per-element callees the typed product does not call (see the module's words).
const IGNORED: [u32; 5] = [
    0x0060_b370, // operator[]
    0x0062_9ab0, // the row's element
    0x0065_78b0, // the checked element
    0x0066_2820, // the vector's size
    0x004b_0680, // the row view
];
/// The test's counters: `_invalid_parameter_noinfo` and `fr2Assert`.
const INVALID: u32 = 0x0300_0000;
const ASSERTS: u32 = 0x0300_0004;
/// The face generation manager's word.
const MANAGER: u32 = 0x011d_59e8;

/// A float for element `i` of matrix `k`: the awkward ones among ordinary values.
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
        16 if pattern == 1 => 0x7f80_0001, // a signalling NaN
        _ => ((n % 2001) as f32 / 97.0 - 10.0).to_bits(),
    }
}

/// A matrix of `rows` by `columns` at `at` (0x20 bytes) whose vector has `extra` elements more
/// (negative: fewer, down to none) than the dimensions say, filled with `pattern`'s values. With
/// none left under dimensions that have elements, the vector is empty over storage of one element
/// (a failing checked element then reads that storage, as the exe goes on to).
fn matrix_at(e: &mut Engine, at: u32, rows: u32, columns: u32, extra: i32, k: u32, pattern: u32) {
    e.call(MATRIX_CONSTRUCT, &args![at]);
    let count = (rows * columns) as i32 + extra;
    if count > 0 {
        e.call(MATRIX_RESIZE, &args![at, 1u32, count as u32]);
        let data = e.mem.u32(at + 0xc);
        for i in 0..count as u32 {
            e.mem.set_u32(data + 4 * i, value(i, k, pattern));
        }
    } else if rows * columns != 0 {
        e.call(MATRIX_RESIZE, &args![at, 1u32, 1u32]);
        let data = e.mem.u32(at + 0xc);
        e.mem.set_u32(data, value(0, k, pattern));
        e.mem.set_u32(at + 0x10, data);
    }
    e.mem.set_u32(at + 0x18, rows);
    e.mem.set_u32(at + 0x1c, columns);
}

/// The same in a block of its own.
fn new_matrix(e: &mut Engine, rows: u32, columns: u32, extra: i32, k: u32, pattern: u32) -> u32 {
    let at = e.mem.alloc(0x20);
    matrix_at(e, at, rows, columns, extra, k, pattern);
    at
}

/// The globals and counters both tests need: `fr2Assert` and `_invalid_parameter_noinfo` as
/// counters of the test's own (the CRT's handler words and the assert's log are not mapped here).
fn counters(e: &mut Engine) {
    e.map(0x0300_0000, 0x1000);
    e.register(0x00ec_7c56, |e, _| {
        let n = e.mem.u32(INVALID);
        e.mem.set_u32(INVALID, n + 1);
        Ret::default()
    });
    e.register(0x00aa_fb90, |e, _| {
        let n = e.mem.u32(ASSERTS);
        e.mem.set_u32(ASSERTS, n + 1);
        Ret::default()
    });
}

/// The two matrices' shapes: (a's rows, a's columns, b's rows, b's columns).
const SHAPES: [(u32, u32, u32, u32); 9] = [
    (1, 50, 50, 1),
    (1, 30, 30, 1),
    (2, 3, 3, 2),
    (3, 1, 1, 4),
    (1, 1, 1, 1),
    (2, 3, 2, 2), // inner sizes differ: the assert, then 3 steps reading past b
    (0, 3, 3, 2), // no rows: an empty product
    (2, 0, 0, 2), // no inner size: zeros
    (2, 2, 2, 0), // no columns
];

#[test]
fn facegen_matrix_product() {
    Diff::new(entry!(
        0x0065_8360,
        matrix::product(Ptr<_>, Ptr<_>, Ptr<_>) -> Ptr<_>
    ))
    .family(&facegen::FACEGEN)
    .family(&leaf::LEAVES)
    .ignore_calls(&IGNORED)
    .inputs(&[
        Input::Of(&[0, 1, 2, 3, 4, 5, 6, 7, 8]),
        // Each vector against its dimensions: as long, 3 longer, one shorter (not below one).
        Input::Of(&[0, 1, 2]),
        Input::Of(&[0, 1, 2]),
        // The result: a block of its own, or a's or b's own words (the product copied over them).
        Input::Of(&[0, 1, 2]),
        Input::Of(&[0, 1]),
    ])
    .setup(|e, case: &Case| {
        counters(e);
        let (ar, ac, br, bc) = SHAPES[case.word(0) as usize];
        let pattern = case.word(4);
        let extra = |w: u32, n: u32| match w {
            0 => 0,
            1 => 3,
            _ => -i32::from(n > 1),
        };
        let a = new_matrix(e, ar, ac, extra(case.word(1), ar * ac), 0, pattern);
        let b = new_matrix(e, br, bc, extra(case.word(2), br * bc), 7, pattern);
        let result = match case.word(3) {
            0 => e.mem.alloc(0x20),
            1 => a,
            _ => b,
        };
        vec![a, result, b]
    })
    .max_cases(520)
    .max_calls(200_000)
    .run()
    .assert_same();
}

/// The statistic's shapes: (an entry's rows and columns, the block matrix's rows and columns).
const STATISTIC_SHAPES: [(u32, u32, u32, u32); 4] = [
    (1, 50, 50, 1),
    (1, 30, 30, 1),
    (2, 3, 3, 2), // a 2 by 2 product: its element (0, 0) answered
    (1, 3, 2, 1), // inner sizes differ: the assert
];

/// The manager (a block of its own at the manager's word) with `count` entries of `shape` in each
/// of its four statistics vectors (`+0xb8 + row * 0x30 + column * 0x18`, 0x3c-byte entries).
fn manager(e: &mut Engine, shape: (u32, u32), count: u32, pattern: u32) {
    e.map(0x011d_5000, 0x1000);
    let manager = e.mem.alloc(0x200);
    e.mem.set_u32(MANAGER, manager);
    for k in 0..4u32 {
        let vector = manager + 0xb8 + (k / 2) * 0x30 + (k % 2) * 0x18;
        let entries = e.mem.alloc(0x3c * count.max(1));
        for i in 0..count {
            matrix_at(
                e,
                entries + 0x3c * i,
                shape.0,
                shape.1,
                0,
                20 + 4 * k + i,
                pattern,
            );
        }
        e.mem.set_u32(vector + 0xc, entries);
        e.mem.set_u32(vector + 0x10, entries + 0x3c * count);
    }
}

/// A coordinate block: its four matrices (`+ row * 0x40 + column * 0x20`) of `shape`.
fn block(e: &mut Engine, shape: (u32, u32), pattern: u32) -> u32 {
    let block = e.mem.alloc(0x80);
    for k in 0..4u32 {
        matrix_at(e, block + 0x20 * k, shape.0, shape.1, 0, 40 + k, pattern);
    }
    block
}

#[test]
fn facegen_matrix_shape_statistic() {
    Diff::new(entry!(
        0x0065_2230,
        matrix::shape_statistic(u32, u32, u32, u32) -> f32
    ))
    .family(&facegen::FACEGEN)
    .family(&leaf::LEAVES)
    .ignore_calls(&IGNORED)
    .translation_calls_plainly(&[0x0065_8360])
    .inputs(&[
        Input::Of(&[0, 1, 2, 3]),
        // No block, or one.
        Input::Flag,
        Input::Of(&[0, 1]),
        Input::Of(&[0, 1]),
        // The index: the first, the last of three, one past, far past (an unsigned compare).
        Input::Of(&[0, 2, 3, 0xffff_ffff]),
        // Three entries, or none.
        Input::Of(&[3, 0]),
        Input::Of(&[0, 1]),
    ])
    .setup(|e, case: &Case| {
        counters(e);
        let (er, ec, br, bc) = STATISTIC_SHAPES[case.word(0) as usize];
        let pattern = case.word(6);
        manager(e, (er, ec), case.word(5), pattern);
        let block = if case.word(1) == 1 {
            block(e, (br, bc), pattern)
        } else {
            0
        };
        vec![block, case.word(2), case.word(3), case.word(4)]
    })
    .max_cases(520)
    .max_calls(200_000)
    .run()
    .assert_same();
}

/// Runs the translation at `typed.0` and, on a second engine made alike, the typed version, on the
/// words `setup` answers, and compares what the Diff cannot when a checked element fails (the
/// translation's invalid-parameter calls lie under the getters the Diff leaves out): the answer,
/// how many invalid-parameter and assert calls each made (at least one invalid), the heap and
/// every byte of memory beside the stack. Under both control words.
fn both_ways(typed: (u32, AbiFn), setup: impl Fn(&mut Engine) -> Vec<u32>) {
    for word in [0x027fu16, 0x007f] {
        let mut sides = Vec::new();
        for typed_side in [false, true] {
            let mut e = Engine::new_untyped_for_test();
            engine_core::x87::set_control_word(word);
            let words = setup(&mut e);
            counters(&mut e);
            // The stack for locals made before the call, as the Diff makes it.
            e.with_stack(4, |_, _| ());
            if typed_side {
                e.register(typed.0, typed.1);
            }
            let answer = e.call(typed.0, &words);
            sides.push((e, answer.eax, answer.st0.to_bits()));
        }
        let (a, b) = (&sides[0].0, &sides[1].0);
        assert_eq!(
            (sides[0].1, sides[0].2),
            (sides[1].1, sides[1].2),
            "the answer, control word {word:04x}"
        );
        assert!(
            a.mem.u32(INVALID) > 0,
            "a failing case that does not tests nothing"
        );
        assert_eq!(
            (a.mem.u32(INVALID), a.mem.u32(ASSERTS)),
            (b.mem.u32(INVALID), b.mem.u32(ASSERTS)),
            "invalid-parameter and assert calls"
        );
        assert_eq!(
            (a.mem.heap_top(), a.mem.heap_available()),
            (b.mem.heap_top(), b.mem.heap_available()),
            "the heap"
        );
        let differences = memory_differences_beside_stack(a, b, 8);
        assert!(
            differences.is_empty(),
            "memory differs at {differences:08x?}, control word {word:04x}"
        );
        assert_eq!(memory_digest_beside_stack(a), memory_digest_beside_stack(b));
    }
    engine_core::x87::set_control_word(engine_core::x87::DEFAULT_CONTROL_WORD);
}

/// Matrices whose vectors are empty under dimensions that say otherwise: each element read of
/// one is a failing checked element (it then reads the storage's word 0, a null vector's 0).
#[test]
fn facegen_matrix_failing_checks() {
    let product = || {
        entry!(
            0x0065_8360,
            matrix::product(Ptr<_>, Ptr<_>, Ptr<_>) -> Ptr<_>
        )
    };
    // a's vector empty under 1 by 3; b's under 3 by 2; both.
    for empty in [1u32, 2, 3] {
        both_ways(product(), move |e| {
            let a = new_matrix(e, 1, 3, if empty & 1 != 0 { -3 } else { 0 }, 0, 1);
            let b = new_matrix(e, 3, 2, if empty & 2 != 0 { -6 } else { 0 }, 7, 1);
            vec![a, e.mem.alloc(0x20), b]
        });
    }
    // The statistic over a block whose matrix's vector is empty under 50 by 1.
    both_ways(
        entry!(
            0x0065_2230,
            matrix::shape_statistic(u32, u32, u32, u32) -> f32
        ),
        |e| {
            manager(e, (1, 50), 3, 0);
            let block = block(e, (50, 1), 0);
            let at = block + 0x20;
            matrix_at(e, at, 50, 1, -50, 3, 0);
            vec![block, 0, 1, 2]
        },
    );
}
