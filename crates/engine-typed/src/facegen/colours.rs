//! The face image's colour vector (`BSFaceGenImage`'s `std::vector` of 16-byte colours): its
//! `assign(count, value)` (`0064dcf0`), which the face's tint (`0065b410`, through the resize
//! `0064db50`) runs on a new image, a texel count of copies of (0, 0, 0, 0): 65,536 for a face's
//! 256 by 256 tint; and its range `erase` (`0064dc50`), which the image's destructor (`0064ce20`,
//! through its clear `0064db70`) runs on all of them. Both come every click on a preset or a hair
//! in the race menu (a 2026-11-24 profile of the menu: about 67,000 elements a click each way, 2 to
//! 3 ms together). The translation constructs each copy and destroys each element through the
//! library's helpers, one call by address an element (the placement `operator new` of
//! `_Construct`, the allocator's `destroy`); here the copies are written in one pass, and the
//! destructor, which does nothing, is not called.
//!
//! What stays by address: the checked iterators the exe makes (`end()` `0065f550`, `begin()`
//! `0044a170`, the copy `0065ded0`) and compares (`0065dfd0`), with their
//! `_invalid_parameter_noinfo` checks, in the translation's frames; `operator new` and
//! `operator delete`; the length error. What is done in place: the library's helpers that only
//! answer a word (the pointer and iterator category tags `0065e750` and `0065fd70`, the unchecked
//! pointer `0065fe40`, the placement `operator new` `006e6da0`, the allocator's `max_size`
//! `0064e2d0`), the element copies and fills, `std::fill` (`0064e4b0`) of the empty range the
//! in-place insert passes it (it writes nothing), and the element destructor (`0065e9a0`), which
//! calls only the empty `0040fbe0`: nothing to do for an element that goes.
//!
//! No float maths: the colours are copied as words.

use engine_core::ptr::Ptr;
use engine_core::{args, typed_call, Engine};

/// The vector's first, last and end-of-storage pointers.
const FIRST: u32 = 0xc;
const LAST: u32 = 0x10;
const END: u32 = 0x14;

/// The checked iterators: `end()` (`0065f550`) and `begin()` (`0044a170`) (`thiscall`, one word:
/// the iterator they fill; they answer its address), the iterator made again from another's two
/// words (`0065ded0`, `thiscall`: the result and the words) and `operator!=` (`0065dfd0`,
/// `thiscall`: the other; answers a byte).
const VECTOR_END: u32 = 0x0065_f550;
const VECTOR_BEGIN: u32 = 0x0044_a170;
const MAKE_ITERATOR: u32 = 0x0065_ded0;
const ITERATORS_DIFFER: u32 = 0x0065_dfd0;
/// `operator new` (cdecl: the size) and `operator delete` (cdecl: the block).
const OPERATOR_NEW: u32 = 0x0040_1000;
const OPERATOR_DELETE: u32 = 0x0040_1030;
/// The vector's length error (`0065f640`, no words; it throws in the game).
const LENGTH_ERROR: u32 = 0x0065_f640;
/// The translated insert (`0064deb0`, `thiscall`: the iterator's two words, the count, the value's
/// address), for a vector the erase did not leave empty (it always does).
const INSERT: u32 = 0x0064_deb0;
/// The allocator's `max_size` (`0064e2d0`), a constant.
const MAX_SIZE: u32 = 0x0fff_ffff;
/// The most copies one write puts down (64 KB).
const COPIES_A_WRITE: u32 = 0x1000;

// Typed from 0064dcf0
/// `assign(count, value)` of the colour vector (`thiscall`, two words: the count and the value's
/// address). The value is copied first, so one inside the vector stays as it was; the whole range
/// is erased (both iterators from `begin()` and `end()`), and `count` copies go in at `begin()`:
/// into the storage there is when it holds them, else into new storage, the old one freed.
pub fn assign(e: &mut Engine, this: Ptr, count: u32, value: u32) {
    let vector = this.addr();
    e.with_stack(0x40, |e, frame| {
        // The translation's frame: the value's copy at +0, then the iterators: the end at +0x10,
        // the begin at +0x18, the erase's result at +0x20, the insert's place at +0x28.
        let frame = frame.addr();
        let copy = frame;
        let colour = e.mem.bytes(value, 16);
        e.mem.write(copy, &colour);
        let at = e.call(VECTOR_END, &args![vector, frame + 0x10]).u32();
        let end = (e.mem.u32(at), e.mem.u32(at.wrapping_add(4)));
        let at = e.call(VECTOR_BEGIN, &args![vector, frame + 0x18]).u32();
        let begin = (e.mem.u32(at), e.mem.u32(at.wrapping_add(4)));
        let erased = frame + 0x20;
        typed_call!(
            super::FAMILY,
            0x0064_dc50,
            erase(e, this, erased, begin.0, begin.1, end.0, end.1)
        );
        let at = e.call(VECTOR_BEGIN, &args![vector, frame + 0x28]).u32();
        let (owner, place) = (e.mem.u32(at), e.mem.u32(at.wrapping_add(4)));
        let first = e.mem.u32(vector.wrapping_add(FIRST));
        if place == first && first == e.mem.u32(vector.wrapping_add(LAST)) {
            insert_into_empty(e, vector, count, copy);
        } else {
            e.call(INSERT, &args![vector, owner, place, count, copy]);
        }
    });
}

/// The element count between two pointers (`SAR 4` of the byte distance).
fn elements(first: u32, last: u32) -> u32 {
    ((last.wrapping_sub(first) as i32) >> 4) as u32
}

// Typed from 0064dc50
/// The range `erase(first, last)` of the colour vector (`thiscall`, five words: the result
/// iterator's address and the two iterators' two words each), in the translation's frame of 0x10
/// bytes: both iterators made again (`0065ded0`), and when they differ (`0065dfd0`), the elements
/// from `last` up to the vector's last copied down to `first` (`std::copy`, `0064e380`) and the
/// last pointer moved to the end of the copies; the elements past it have the empty destructor.
/// The result is the first iterator; answers its address.
pub fn erase(
    e: &mut Engine,
    this: Ptr,
    result: u32,
    first_owner: u32,
    first: u32,
    last_owner: u32,
    last: u32,
) -> u32 {
    let vector = this.addr();
    e.with_stack(0x10, |e, frame| {
        let from = frame.addr();
        let to = from + 8;
        e.call(MAKE_ITERATOR, &args![vector, from, first_owner, first]);
        e.call(MAKE_ITERATOR, &args![vector, to, last_owner, last]);
        if e.call(ITERATORS_DIFFER, &args![from, to]).bool() {
            let old_last = e.mem.u32(vector.wrapping_add(LAST));
            let (source, dest) = (e.mem.u32(to + 4), e.mem.u32(from + 4));
            let new_last = copy_down(e, source, old_last, dest);
            e.mem.set_u32(vector.wrapping_add(LAST), new_last);
        }
        let owner = e.mem.u32(from);
        e.mem.set_u32(result, owner);
        let pointer = e.mem.u32(from + 4);
        e.mem.set_u32(result.wrapping_add(4), pointer);
    });
    result
}

/// `std::copy` of the 16-byte elements from `first` while not at `last` to `dest` on (`0064e380`'s
/// loop); answers `dest` plus the whole elements from `first` to `last`, as `0064e5a0` does.
fn copy_down(e: &mut Engine, first: u32, last: u32, dest: u32) -> u32 {
    let end = (elements(first, last) << 4).wrapping_add(dest);
    let (mut source, mut target) = (first, dest);
    while source != last {
        let element = e.mem.bytes(source, 16);
        e.mem.write(target, &element);
        target = target.wrapping_add(0x10);
        source = source.wrapping_add(0x10);
    }
    end
}

/// `_Insert_n` (`0064df50`, through `0064deb0`) of `count` copies of the colour at `value` into
/// the empty vector the erase leaves, at its start. Nothing for no copies; the length error
/// (`0065f640`) past `max_size`. Within the capacity, the copies are constructed from the last
/// pointer on and it moves past them. Else new storage of `capacity + capacity / 2` elements
/// (no half when it would pass `max_size`), at least `count`, takes the copies; the old storage,
/// when there is one, is freed (its elements have the empty destructor) and the three pointers
/// are set. With no elements before, the new storage holds at most `max_size` elements, so its
/// size fits in 32 bits and `_Allocate`'s `std::bad_alloc` cannot come.
fn insert_into_empty(e: &mut Engine, vector: u32, count: u32, value: u32) {
    let field = |offset: u32| vector.wrapping_add(offset);
    let first = e.mem.u32(field(FIRST));
    let capacity = if first == 0 {
        0
    } else {
        elements(first, e.mem.u32(field(END)))
    };
    if count == 0 {
        return;
    }
    if MAX_SIZE < count {
        e.call(LENGTH_ERROR, &[]);
        return;
    }
    if capacity < count {
        let mut new_capacity = if MAX_SIZE.wrapping_sub(capacity >> 1) < capacity {
            0
        } else {
            (capacity >> 1).wrapping_add(capacity)
        };
        if new_capacity < count {
            new_capacity = count;
        }
        let storage = e.call(OPERATOR_NEW, &args![new_capacity << 4]).u32();
        construct_copies(e, storage, count, value, false);
        let first = e.mem.u32(field(FIRST));
        let total = elements(first, e.mem.u32(field(LAST))).wrapping_add(count);
        let first = e.mem.u32(field(FIRST));
        if first != 0 {
            e.call(OPERATOR_DELETE, &args![first]);
        }
        e.mem
            .set_u32(field(END), (new_capacity << 4).wrapping_add(storage));
        e.mem
            .set_u32(field(LAST), (total << 4).wrapping_add(storage));
        e.mem.set_u32(field(FIRST), storage);
        return;
    }
    // In place the exe copies the value into a local of its own first, and the copies read that.
    let last = e.mem.u32(field(LAST));
    construct_copies(e, last, count, value, true);
    let last = e.mem.u32(field(LAST));
    e.mem.set_u32(field(LAST), (count << 4).wrapping_add(last));
}

/// Whether `[a, a + a_len)` and `[b, b + b_len)` share a byte (64-bit ends: no wrapping).
fn overlaps(a: u64, a_len: u64, b: u64, b_len: u64) -> bool {
    a_len != 0 && b_len != 0 && a < b + b_len && b < a + a_len
}

/// `count` copies of the 16-byte value at `value` constructed from `first` on (`_Ufill`,
/// `0064e340`): each through the placement `operator new`, which answers its place, and copied
/// there unless that is 0. `read_once`: the value was copied into the exe's own local, which no
/// copy reaches; else the exe's loop reads it at `value` for each copy. The copies go down in
/// writes of up to [`COPIES_A_WRITE`] when none is at 0, none passes 4 GB and none covers a value
/// that is read again; else one by one as the exe's loop goes. Answers whether they went in writes.
fn construct_copies(e: &mut Engine, first: u32, count: u32, value: u32, read_once: bool) -> bool {
    let bytes = u64::from(count) * 16;
    let wraps = u64::from(first) + bytes > 1 << 32;
    let covers_value =
        !read_once && (wraps || overlaps(u64::from(first), bytes, u64::from(value), 16));
    let colour = e.mem.bytes(value, 16);
    if first != 0 && !wraps && !covers_value {
        let chunk = colour.repeat(count.min(COPIES_A_WRITE) as usize);
        let (mut at, mut left) = (first, count);
        while left != 0 {
            let copies = left.min(COPIES_A_WRITE);
            e.mem.write(at, &chunk[..copies as usize * 16]);
            at = at.wrapping_add(copies << 4);
            left -= copies;
        }
        return true;
    }
    let mut at = first;
    for _ in 0..count {
        if at != 0 {
            if read_once {
                e.mem.write(at, &colour);
            } else {
                let now = e.mem.bytes(value, 16);
                e.mem.write(at, &now);
            }
        }
        at = at.wrapping_add(0x10);
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The test colour's four words.
    const WORDS: [u32; 4] = [0x3f80_0000, 0x7fc0_1234, 0x8000_0000, 1];

    /// The test colour at `at`.
    fn put_colour(e: &mut Engine, at: u32) {
        for (k, word) in (0u32..).zip(WORDS) {
            e.mem.set_u32(at + 4 * k, word);
        }
    }

    /// The words from `first` on, `count` copies' worth.
    fn copies(e: &Engine, first: u32, count: u32) -> Vec<u32> {
        (0..4 * count)
            .map(|k| e.mem.u32(first.wrapping_add(4 * k)))
            .collect()
    }

    /// The copies go down in writes unless one would be at 0, pass 4 GB or cover a value read
    /// again: each of those alone, with the copies the exe's loop makes either way.
    #[test]
    fn the_copies_go_down_in_writes_unless_a_guard_says_not() {
        // The plain cases: more than one write's worth, fewer, one; read once or again.
        for (count, read_once) in [(COPIES_A_WRITE * 3 + 5, false), (7, true), (1, false)] {
            let mut e = Engine::new();
            let value = e.mem.alloc(16);
            put_colour(&mut e, value);
            let first = e.mem.alloc(16 * count);
            assert!(construct_copies(&mut e, first, count, value, read_once));
            assert_eq!(copies(&e, first, count), WORDS.repeat(count as usize));
        }
        // The value inside the first copy's second half: read again, the first copy writes the
        // colour's first half over it, and each later copy is that (one by one); read once, every
        // copy is the colour (in writes).
        for read_once in [false, true] {
            let mut e = Engine::new();
            let first = e.mem.alloc(16 * 4);
            let value = first + 8;
            put_colour(&mut e, value);
            assert_eq!(
                construct_copies(&mut e, first, 4, value, read_once),
                read_once
            );
            let later = if read_once {
                WORDS
            } else {
                [WORDS[2], WORDS[3], WORDS[2], WORDS[3]]
            };
            let mut expected = WORDS.to_vec();
            for _ in 1..4 {
                expected.extend(later);
            }
            assert_eq!(copies(&e, first, 4), expected, "read once: {read_once}");
        }
        // A copy at 0 is left out, the others made one by one.
        let mut e = Engine::new();
        let value = e.mem.alloc(16);
        put_colour(&mut e, value);
        e.map(0, 0x1000);
        assert!(!construct_copies(&mut e, 0, 3, value, true));
        assert_eq!(copies(&e, 0, 1), vec![0; 4]);
        assert_eq!(copies(&e, 0x10, 2), WORDS.repeat(2));
        // Copies past 4 GB wrap to 0, which is left out, one by one.
        let mut e = Engine::new();
        let value = e.mem.alloc(16);
        put_colour(&mut e, value);
        e.map(0xffff_f000, 0x1000);
        e.map(0, 0x1000);
        assert!(!construct_copies(&mut e, 0xffff_fff0, 3, value, true));
        assert_eq!(copies(&e, 0xffff_fff0, 1), WORDS.to_vec());
        assert_eq!(copies(&e, 0, 1), vec![0; 4]);
        assert_eq!(copies(&e, 0x10, 1), WORDS.to_vec());
    }
}
