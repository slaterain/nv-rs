//! The differential test of the face model sizes (`engine_typed::facegen::sizes`: the EGM morph
//! data's `0065c110` and the EGT texture data's `0065c490`) against their translations, under both
//! control words. The models are built in guest memory: the holder, its data and the data's two
//! record vectors, whose records hold the inner vector's two words. The translations reach each
//! record through the vectors' getters (count, element, byte size), which the typed walks read
//! for themselves, so those are left out of the call comparison (`ignore_calls`); the answer and
//! all memory compare.
//!
//! The shapes: no holder, no data, empty vectors, the counts of a face's model (50 and 30 EGM
//! morphs, 50 and 30 EGT records) and small ones, vectors whose byte length is not a multiple of
//! the record (the signed division drops the rest), and inner vectors whose two words differ by
//! any amount: not a multiple of 6, zero, negative (the division truncates toward zero) and huge
//! (the sum wraps).

use engine as _;
use engine_core::prelude::*;
use engine_core::typed::diff::{Case, Diff, Input};
use engine_core::typed::leaf;
use engine_typed::facegen::{self, sizes};

/// The getters the translations call and the typed walks read past: EGM (`0065c110`): the morph
/// vector's count, its element and the vertex vector's count; EGT (`0065c490`): the record
/// vector's count, its element and the pixel vector's byte size.
const EGM_GETTERS: [u32; 3] = [0x0065_7c00, 0x0065_7c20, 0x0065_7bc0];
const EGT_GETTERS: [u32; 3] = [0x0065_cea0, 0x0065_cec0, 0x0065_cc50];

/// The record counts of the two vectors and the bytes past the last whole record, by shape;
/// shapes 0 and 1 are the model without a holder and the holder without data.
const SHAPES: [(u32, u32, u32); 8] = [
    (0, 0, 0),
    (0, 0, 0),
    (0, 0, 0),
    (1, 0, 0),
    (0, 3, 0),
    (2, 5, 1),
    (50, 30, 0),
    (7, 11, 0x1b),
];

/// An inner vector's `end - begin` for record `i` of a case's `seed`: the awkward ones among
/// ordinary lengths.
fn inner_length(i: u32, seed: u32) -> u32 {
    let n = i.wrapping_mul(2_654_435_761).wrapping_add(seed * 97);
    match n % 11 {
        0 => 0,
        1 => 5,
        2 => 0xffff_fff9, // -7
        3 => 0x8000_0000, // the most negative
        4 => 0x7fff_ffff, // the largest
        5 => 0xffff_fffa, // -6
        _ => (n >> 8) % 0x3_0000,
    }
}

/// A model of `shape` for the size at `holder_at` (8: EGM, 0xc: EGT) with `record`-byte records
/// whose inner vector is at `inner_at`; the model's address.
fn model(e: &mut Engine, case: &Case, holder_at: u32, record: u32, inner_at: u32) -> u32 {
    let shape = case.word(0) % 8;
    let seed = case.word(0) / 8;
    let this = e.mem.alloc(0x10);
    if shape == 0 {
        return this;
    }
    let holder = e.mem.alloc(0x10);
    e.mem.set_u32(this + holder_at, holder);
    if shape == 1 {
        return this;
    }
    let data = e.mem.alloc(0x34);
    e.mem.set_u32(holder + 8, data);
    let (first, second, rest) = SHAPES[shape as usize];
    for (list, n) in [first, second].into_iter().enumerate() {
        let vector = data + 4 + list as u32 * 0x18;
        let bytes = n * record + rest;
        let begin = e.mem.alloc(bytes.max(4));
        e.mem.set_u32(vector + 0xc, begin);
        e.mem.set_u32(vector + 0x10, begin + bytes);
        for i in 0..n {
            let at = begin + i * record + inner_at;
            let start = seed.wrapping_mul(0x0101_0101).wrapping_add(i * 0x40);
            e.mem.set_u32(at + 0xc, start);
            e.mem.set_u32(
                at + 0x10,
                start.wrapping_add(inner_length(i + list as u32 * 64, seed)),
            );
        }
    }
    this
}

/// The cases: shape `w % 8` with seed `w / 8`, every pair, the face's shape first.
const CASES: [u32; 48] = {
    let mut all = [0u32; 48];
    let mut i = 0;
    while i < 48 {
        all[i] = (i as u32 + 6) % 48;
        i += 1;
    }
    all
};

#[test]
fn facegen_egm_size() {
    Diff::new(entry!(0x0065_c110, sizes::egm_size(Ptr) -> u32))
        .family(&facegen::FACEGEN)
        .family(&leaf::LEAVES)
        .ignore_calls(&EGM_GETTERS)
        .inputs(&[Input::Of(&CASES)])
        .setup(|e, case| vec![model(e, case, 8, 0x1c, 4)])
        .run()
        .assert_same();
}

#[test]
fn facegen_egt_size() {
    Diff::new(entry!(0x0065_c490, sizes::egt_size(Ptr) -> u32))
        .family(&facegen::FACEGEN)
        .family(&leaf::LEAVES)
        .ignore_calls(&EGT_GETTERS)
        .inputs(&[Input::Of(&CASES)])
        .setup(|e, case| vec![model(e, case, 0xc, 0x58, 0x10)])
        .run()
        .assert_same();
}
