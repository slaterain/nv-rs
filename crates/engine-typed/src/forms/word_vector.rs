//! The debug `std::vector<unsigned long>` members the linker placed before the debug-heap string
//! (`00449fd0` to `0044a2c0`) and its word allocator's after it (`0044abd0` to `0044ac20`): the
//! constructor and destructor, `clear`, `begin`, `erase(first, last)`, `_Buy` and `max_size`, and
//! the allocator's `allocate`, `max_size` and one-word copy.
//!
//! The vector: the checked-iterator base at +0 (its constructor `0065d3a0`), the allocator at +8,
//! the first element at +0xc, the end of the elements at +0x10 and the end of the storage at +0x14.
//! An iterator is two words (the element and the vector).
//!
//! **Native.** The allocator's empty constructor (`006815c0`, which answers its own `ECX`) and the
//! two `max_size` bodies (the constant `0x3fffffff`) are done in place, and the unit's members call
//! each other as typed calls. What stays a call by address: the checked iterators' constructors and
//! compare (`0065e000`, `0065ded0`, `0065f550`, `0065dfd0`), the elements' move and destruction
//! (`0044d600`, `0065f2b0`), the base's constructor and destructor and `_Tidy` (`0065d3a0`,
//! `0065cfb0`, `0065f240`), the length error (`0065f640`, which throws) and the array allocation
//! (`0044d670`). The constructor's and destructor's unwinding frames are left out as the
//! translations leave them.
//!
//! No floats.

use super::FAMILY;
use engine_core::{args, typed_call, Engine};

/// The allocator (+8), the first element (+0xc), the end of the elements (+0x10) and of the storage
/// (+0x14).
const ALLOCATOR: u32 = 0x8;
const FIRST: u32 = 0xc;
const LAST: u32 = 0x10;
const END: u32 = 0x14;
/// The word allocator's `max_size()` (`0044abf0`).
const MAX_WORDS: u32 = 0x3fff_ffff;
/// The checked iterator's constructor (`thiscall (iterator, element, vector)`), the iterator from
/// a position's two words (`thiscall (vector, &result, word, word)`), `end()` (`thiscall (vector,
/// &result)`) and the compare (`thiscall (iterator, &other)`, answers a byte: whether they differ).
const ITERATOR_CONSTRUCT: u32 = 0x0065_e000;
const ITERATOR_AT: u32 = 0x0065_ded0;
const ITERATOR_END: u32 = 0x0065_f550;
const ITERATOR_DIFFERS: u32 = 0x0065_dfd0;
/// The elements' move (cdecl `(from, to, destination)`, answers the new end) and destruction
/// (`thiscall (vector, from, to)`).
const MOVE_ELEMENTS: u32 = 0x0044_d600;
const DESTROY_ELEMENTS: u32 = 0x0065_f2b0;
/// The base's constructor (`thiscall`, the allocator by value) and destructor, and `_Tidy`.
const BASE_CONSTRUCT: u32 = 0x0065_d3a0;
const BASE_DESTRUCT: u32 = 0x0065_cfb0;
const TIDY: u32 = 0x0065_f240;
/// `_Xlen` of the vector (throws `length_error`).
const LENGTH_ERROR: u32 = 0x0065_f640;
/// The array allocation of `count` words (cdecl; two words pushed, the count read).
const ALLOCATE_WORDS: u32 = 0x0044_d670;

fn word_at(e: &Engine, this: u32, offset: u32) -> u32 {
    e.mem.u32(this.wrapping_add(offset))
}

fn set_word_at(e: &mut Engine, this: u32, offset: u32, value: u32) {
    e.mem.set_u32(this.wrapping_add(offset), value);
}

// Typed from 00449fd0
/// The constructor: the base's constructor with the allocator (the empty allocator constructor
/// answers its own slot, which holds `this`, so that is the word passed), then `_Buy(0)`. Answers
/// `this`.
pub fn word_vector_construct(e: &mut Engine, this: u32) -> u32 {
    e.call(BASE_CONSTRUCT, &args![this, this]);
    typed_call!(FAMILY, 0x0044_a240, word_vector_buy(e, this, 0));
    this
}

// Typed from 0044a040
/// The destructor: `_Tidy`, then the base's destructor.
pub fn word_vector_destruct(e: &mut Engine, this: u32) {
    e.call(TIDY, &args![this]);
    e.call(BASE_DESTRUCT, &args![this]);
}

// Typed from 0044a0a0
/// `clear()`: `erase(begin(), end())`, answering what the erase answers (the address of its
/// result, a local).
pub fn word_vector_clear(e: &mut Engine, this: u32) -> u32 {
    e.with_stack(0x18, |e, frame| {
        let frame = frame.addr();
        let (end_result, begin_result, erased) = (frame, frame + 8, frame + 0x10);
        let end = e.call(ITERATOR_END, &args![this, end_result]).u32();
        let (end_base, end_element) = (e.mem.u32(end), e.mem.u32(end.wrapping_add(4)));
        let begin = typed_call!(
            FAMILY,
            0x0044_a170,
            word_vector_begin(e, this, begin_result)
        );
        let (begin_base, begin_element) = (e.mem.u32(begin), e.mem.u32(begin.wrapping_add(4)));
        typed_call!(
            FAMILY,
            0x0044_a1a0,
            word_vector_erase(
                e,
                this,
                erased,
                begin_base,
                begin_element,
                end_base,
                end_element
            )
        )
    })
}

// Typed from 0044a170
/// `begin()`: the iterator at `result` built from the first element and the vector. Answers
/// `result`.
pub fn word_vector_begin(e: &mut Engine, this: u32, result: u32) -> u32 {
    let first = word_at(e, this, FIRST);
    e.call(ITERATOR_CONSTRUCT, &args![result, first, this]);
    result
}

// Typed from 0044a1a0
/// `erase(first, last)` (each iterator by value: its base word and its element): iterators made of
/// both; when they differ, the elements after the range moved down, the left-over tail destroyed and
/// the end moved back. `result` gets the first iterator; answers `result`.
pub fn word_vector_erase(
    e: &mut Engine,
    this: u32,
    result: u32,
    first_base: u32,
    first_element: u32,
    last_base: u32,
    last_element: u32,
) -> u32 {
    let (base, element) = e.with_stack(0x10, |e, frame| {
        let frame = frame.addr();
        let (last, first) = (frame, frame + 8);
        e.call(ITERATOR_AT, &args![this, first, first_base, first_element]);
        e.call(ITERATOR_AT, &args![this, last, last_base, last_element]);
        if e.call(ITERATOR_DIFFERS, &args![first, last]).u8() != 0 {
            let end = word_at(e, this, LAST);
            let from = e.mem.u32(last + 4);
            let to = e.mem.u32(first + 4);
            let new_end = e.call(MOVE_ELEMENTS, &args![from, end, to]).u32();
            let end = word_at(e, this, LAST);
            e.call(DESTROY_ELEMENTS, &args![this, new_end, end]);
            set_word_at(e, this, LAST, new_end);
        }
        (e.mem.u32(first), e.mem.u32(first + 4))
    });
    e.mem.set_u32(result, base);
    e.mem.set_u32(result.wrapping_add(4), element);
    result
}

// Typed from 0044a240
/// `_Buy(count)`: the three pointers cleared; false for 0; past `max_size()` the length error (which
/// throws; the exe answers true after it); else storage for `count` words from the allocator, the
/// end of the elements at its start and the end of the storage `count` words on. True.
pub fn word_vector_buy(e: &mut Engine, this: u32, count: u32) -> bool {
    set_word_at(e, this, FIRST, 0);
    set_word_at(e, this, LAST, 0);
    set_word_at(e, this, END, 0);
    if count == 0 {
        return false;
    }
    if MAX_WORDS < count {
        e.call(LENGTH_ERROR, &[]);
        return true;
    }
    let storage = typed_call!(
        FAMILY,
        0x0044_abd0,
        word_allocator_allocate(e, this.wrapping_add(ALLOCATOR), count)
    );
    set_word_at(e, this, FIRST, storage);
    set_word_at(e, this, LAST, storage);
    set_word_at(e, this, END, storage.wrapping_add(count.wrapping_mul(4)));
    true
}

// Typed from 0044a2c0
/// `max_size()`: the allocator's, `0x3fffffff`.
pub fn word_vector_max_size(_e: &mut Engine, _this: u32) -> u32 {
    MAX_WORDS
}

// Typed from 0044abd0
/// The word allocator's `allocate(count)` (`this` unread): the array allocation of `count` words
/// (with a second word 0), answering its block.
pub fn word_allocator_allocate(e: &mut Engine, _this: u32, count: u32) -> u32 {
    e.call(ALLOCATE_WORDS, &args![count, 0u32]).u32()
}

// Typed from 0044abf0
/// The word allocator's `max_size()`: `0x3fffffff` (the library's `count > 0 ? count : 1` of the
/// constant).
pub fn word_allocator_max_size(_e: &mut Engine, _this: u32) -> u32 {
    MAX_WORDS
}

// Typed from 0044ac20
/// A one-word object's copy: the word at `source` stored at `this`.
pub fn copy_one_word(e: &mut Engine, this: u32, source: u32) {
    let value = e.mem.u32(source);
    e.mem.set_u32(this, value);
}
