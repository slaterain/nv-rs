//! The differential tests of the face image's colour vector assignment and range erase
//! (`engine_typed::facegen::colours`, `0064dcf0` and `0064dc50`) against their translations, on
//! real vectors: each made by the image's vector constructor (`0065c750`, its proxy included) and
//! given storage by the vector's own `_Buy` (`0065d0d0`), every helper they reach the game's own
//! translation (the iterators, the erase's and the insert's library helpers, `operator new` and
//! `operator delete`), but `_invalid_parameter_noinfo` and the length error (`0065f640`),
//! counters of the test's own (the CRT's handler words are not mapped here; the length error
//! throws in the game).
//!
//! The translation reaches the library's helpers that only answer a word (the category tags
//! `0065e750` and `0065fd70`, the unchecked pointer `0065fe40`, the placement `operator new`
//! `006e6da0`, `max_size` `0064e2d0`), the empty element destructor (`0065e9a0`) and the fill
//! (`0064e4b0`, of an empty range here); the typed assignment does their work in place, so those
//! are left out of the call comparison (`ignore_calls`). Every other call, every byte of memory and
//! the heap compare. The cases: a new vector (no storage), storage empty, part full, full, and
//! larger than needed; a vector whose proxy word is 0 (the iterators' compare calls
//! `_invalid_parameter_noinfo`); counts of none, within the storage, exactly it, one past it, past
//! half again, more than two writes' worth (the typed side writes 4,096 copies at a time; a face's
//! 65,536 would cost the translation that many calls a case), and past `max_size`; and a value that
//! is a colour of its own or one of the vector's own elements (copied before they are erased and
//! freed). The erase's
//! own cases: everything (the image's clear), the middle, the tail, the head, nothing, and
//! iterators outside the elements (their checks call `_invalid_parameter_noinfo`).

use engine as _;
use engine_core::prelude::*;
use engine_core::typed::diff::{Case, Diff, Input};
use engine_core::typed::leaf;
use engine_typed::facegen::{self, colours};

/// The colour vector's constructor (`thiscall`) and its `_Buy` (`thiscall`: the element count).
const VECTOR_CONSTRUCT: u32 = 0x0065_c750;
const VECTOR_BUY: u32 = 0x0065_d0d0;

/// The callees the typed assignment does not call (see the module's words).
const IGNORED: [u32; 7] = [
    0x0065_e750,
    0x0065_fd70,
    0x0065_fe40,
    0x006e_6da0,
    0x0064_e2d0,
    0x0065_e9a0,
    0x0064_e4b0,
];

/// The counters of `_invalid_parameter_noinfo` and of the length error.
const COUNTERS: u32 = 0x0300_0000;

/// The vector states: (elements, storage for, proxy word kept).
const STATES: [(u32, u32, bool); 8] = [
    (0, 0, true),
    (0, 4, true),
    (3, 4, true),
    (4, 4, true),
    (2, 8, true),
    (6, 6, true),
    (0, 0, false),
    (3, 4, false),
];

/// One more on the counter at `at`.
fn counted(e: &mut Engine, at: u32) -> Ret {
    let count = e.mem.u32(at);
    e.mem.set_u32(at, count + 1);
    Ret::default()
}

/// A colour's words: floats, a NaN, a negative zero and a subnormal among them (copied as words).
fn word(i: u32, k: u32) -> u32 {
    match (i + k) % 5 {
        0 => 0x7fc0_1234,
        1 => 0x8000_0000,
        2 => 0x0000_0001,
        3 => (i as f32 * 0.25 - 1.0).to_bits(),
        _ => 0xbf80_0000 ^ i,
    }
}

/// A colour vector of `size` elements in storage for `capacity`, its proxy word cleared unless
/// `proxy`.
fn vector(e: &mut Engine, size: u32, capacity: u32, proxy: bool) -> u32 {
    let this = e.mem.alloc(0x18);
    e.call(VECTOR_CONSTRUCT, &args![this]);
    if capacity != 0 {
        e.call(VECTOR_BUY, &args![this, capacity]);
    }
    let first = e.mem.u32(this + 0xc);
    for i in 0..size {
        for k in 0..4 {
            e.mem.set_u32(first + 16 * i + 4 * k, word(i, k));
        }
    }
    e.mem.set_u32(this + 0x10, first + 16 * size);
    if !proxy {
        e.mem.set_u32(this, 0);
    }
    this
}

#[test]
fn facegen_colours_assign() {
    Diff::new(entry!(0x0064_dcf0, colours::assign(Ptr, u32, u32)))
        .family(&facegen::FACEGEN)
        .family(&leaf::LEAVES)
        .ignore_calls(&IGNORED)
        // The translation calls its erase plainly; the typed assignment through `typed_call!`.
        .translation_calls_plainly(&[0x0064_dc50])
        .inputs(&[
            Input::Of(&[0, 1, 2, 3, 4, 5, 6, 7]),
            // The count: none, within, exactly, one past, past half again, more than two writes of
            // copies, past `max_size`.
            Input::Of(&[3, 0, 1, 4, 5, 9, 0x2345, 0x1000_0000]),
            // The value: a colour of its own, the vector's first element, its last.
            Input::Of(&[0, 1, 2]),
        ])
        .setup(|e, case: &Case| {
            e.map(COUNTERS, 0x1000);
            e.register(0x00ec_7c56, |e, _| counted(e, COUNTERS));
            e.register(0x0065_f640, |e, _| counted(e, COUNTERS + 4));
            let (size, capacity, proxy) = STATES[case.word(0) as usize];
            let this = vector(e, size, capacity, proxy);
            let first = e.mem.u32(this + 0xc);
            let value = match case.word(2) {
                1 if size != 0 => first,
                2 if size != 0 => first + 16 * (size - 1),
                _ => {
                    let value = e.mem.alloc(16);
                    for k in 0..4 {
                        e.mem.set_u32(value + 4 * k, word(7, k));
                    }
                    value
                }
            };
            vec![this, case.word(1), value]
        })
        .max_cases(200)
        .run()
        .assert_same();
}

/// The callees the typed erase does not call: the copy's tags and unchecked pointers, and the
/// element destructor.
const ERASE_IGNORED: [u32; 4] = [0x0065_e750, 0x0065_fd70, 0x0065_fe40, 0x0065_e9a0];

/// The erased range of a vector of `size` elements at `first` with its last at `last`, by kind:
/// everything, the middle (1 to 3), none at the start, the tail after the first, the head, from
/// one element before the start to the end (`_invalid_parameter_noinfo`), none past the end (each
/// iterator's check calls it). A vector too short for a kind takes everything or nothing.
fn range(kind: u32, size: u32, first: u32, last: u32) -> (u32, u32) {
    let at = |i: u32| first.wrapping_add(16 * i);
    match kind {
        0 => (first, last),
        1 if size >= 3 => (at(1), at(3)),
        1 => (first, last),
        3 if size >= 1 => (at(1), last),
        4 if size >= 1 => (first, at(1)),
        5 => (first.wrapping_sub(16), last),
        6 => (last.wrapping_add(16), last.wrapping_add(16)),
        _ => (first, first),
    }
}

#[test]
fn facegen_colours_erase() {
    Diff::new(entry!(
        0x0064_dc50,
        colours::erase(Ptr, u32, u32, u32, u32, u32) -> u32
    ))
    .family(&facegen::FACEGEN)
    .family(&leaf::LEAVES)
    .ignore_calls(&ERASE_IGNORED)
    .inputs(&[
        Input::Of(&[2, 0, 1, 3, 4, 5, 6, 7]),
        Input::Of(&[0, 1, 2, 3, 4, 5, 6]),
        // The iterators' owner words: the vector's proxy as `begin()` gives it, or another word
        // (the erase makes both iterators again from the vector, so it reads neither).
        Input::Flag,
    ])
    .setup(|e, case: &Case| {
        e.map(COUNTERS, 0x1000);
        e.register(0x00ec_7c56, |e, _| counted(e, COUNTERS));
        let (size, capacity, proxy) = STATES[case.word(0) as usize];
        let this = vector(e, size, capacity, proxy);
        let (first, last) = (e.mem.u32(this + 0xc), e.mem.u32(this + 0x10));
        let (from, to) = range(case.word(1), size, first, last);
        let owner = if case.word(2) != 0 {
            e.mem.u32(this)
        } else {
            0x0700_1234
        };
        let result = e.mem.alloc(8);
        vec![this, result, owner, from, owner, to]
    })
    .run()
    .assert_same();
}
