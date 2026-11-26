//! The debug-heap `std::basic_string<char>` members the linker placed in `TESObject`'s unit (the
//! Xbox PDB's `tesobject` source unit, `0044a4c0` to `0044a7f0`: Visual Studio 2008's
//! `basic_string<char, char_traits<char>, _DebugHeapAllocator<char>>`), each as the exe does it.
//!
//! The string: the allocator proxy at +0, the storage at +4 (a 16-byte buffer, or the heap pointer
//! once the reserve is 16 or more), the length (`_Mysize`) at +0x14 and the reserve (`_Myres`) at
//! +0x18.
//!
//! **Native.** The one-line library members the exe calls are done in place, each the body it holds
//! (checked in the exe): `_Myptr` (`0044a970`: the heap pointer or the buffer), the length getter
//! (`00825c00`), `_Inside` (`0044ab90`), `_Eos` (`0044a880`: the length, then the terminator through
//! `char_traits::assign`, `0044a8c0`) and the allocator's empty constructors (`006815c0`,
//! `0065de30`: they answer `this` and store nothing, so the two proxy constructors here do nothing
//! but answer `this`). A word the exe reads twice with no call between is read once. The `strlen`
//! wrapper is the C runtime family's typed `strlen`, and this unit's members call each other as
//! typed calls. What stays a call by address: `_Grow` (`0044a8d0`), the traits' `copy_s` and
//! `move_s` (`0044ccd0`, `0044d580`), the allocator's deallocate (`0064df10`) and `_Xran`
//! (`00ec30e8`, which throws).
//!
//! No floats.

use super::FAMILY;
use crate::crt;
use engine_core::{args, typed_call, Engine};

/// The storage (+4), the length (+0x14) and the reserve (+0x18).
const BUFFER: u32 = 4;
const SIZE: u32 = 0x14;
const RESERVE: u32 = 0x18;
/// A reserve of this or more means the characters are on the heap.
const HEAP_RESERVE: u32 = 0x10;
/// The reserve of the inline buffer.
const INLINE_RESERVE: u32 = 0xf;
/// `std::string::npos` (`010173f0`, a read-only constant).
const NPOS: u32 = u32::MAX;
/// `_Xran`: throws `out_of_range`.
const OUT_OF_RANGE: u32 = 0x00ec_30e8;
/// `_Grow(newsize, trim)`.
const GROW: u32 = 0x0044_a8d0;
/// `_Traits_helper::copy_s(dest, destsize, src, count)` and `move_s` (cdecl; `move_s` reads the
/// `ECX` it is entered with, the string, as its first word).
const COPY_CHARS: u32 = 0x0044_ccd0;
const MOVE_CHARS: u32 = 0x0044_d580;
/// The allocator's `Deallocate(ptr, size)`, with the string as `ECX`.
const FREE: u32 = 0x0064_df10;

fn length(e: &Engine, this: u32) -> u32 {
    e.mem.u32(this.wrapping_add(SIZE))
}

fn reserve(e: &Engine, this: u32) -> u32 {
    e.mem.u32(this.wrapping_add(RESERVE))
}

/// `_Myptr` (`0044a970`): the heap pointer once the reserve is 16 or more, else the buffer at +4.
fn characters(e: &Engine, this: u32) -> u32 {
    if reserve(e, this) >= HEAP_RESERVE {
        e.mem.u32(this.wrapping_add(BUFFER))
    } else {
        this.wrapping_add(BUFFER)
    }
}

/// `_Eos(newsize)` (`0044a880`): the length stored, then `'\0'` at the characters plus it.
fn set_length(e: &mut Engine, this: u32, new_length: u32) {
    e.mem.set_u32(this.wrapping_add(SIZE), new_length);
    let end = characters(e, this).wrapping_add(new_length);
    e.mem.set_u8(end, 0);
}

/// `_Inside(text)` (`0044ab90`): whether `text` points into the characters; false for null.
fn holds(e: &Engine, this: u32, text: u32) -> bool {
    if text == 0 {
        return false;
    }
    let start = characters(e, this);
    text >= start && start.wrapping_add(length(e, this)) > text
}

/// `_Grow(count, false)`: whether there is room for `count` characters (and `count` is not 0).
fn grow(e: &mut Engine, this: u32, count: u32) -> bool {
    e.call(GROW, &args![this, count, 0u32]).bool()
}

/// The length of `string`, after `_Xran` when it is shorter than `position` (the exe reads it again
/// after that call).
fn length_from(e: &mut Engine, string: u32, position: u32) -> u32 {
    let size = length(e, string);
    if size >= position {
        return size;
    }
    e.call(OUT_OF_RANGE, &[]);
    length(e, string)
}

// Typed from 0044a4c0
/// `basic_string(const char* text)`: the allocator argument and the proxy (empty constructors),
/// `_Tidy(false, 0)`, then `assign(text)`. Answers `this`.
pub fn construct_from_text(e: &mut Engine, this: u32, text: u32) -> u32 {
    typed_call!(FAMILY, 0x0044_a730, tidy(e, this, false, 0));
    typed_call!(FAMILY, 0x0044_a640, assign_text(e, this, text));
    this
}

// Typed from 0044a500
/// The copy constructor: the proxy (an empty constructor), `_Tidy(false, 0)`, then the whole of
/// `right` assigned. Answers `this`.
pub fn copy_construct(e: &mut Engine, this: u32, right: u32) -> u32 {
    typed_call!(FAMILY, 0x0044_a730, tidy(e, this, false, 0));
    typed_call!(FAMILY, 0x0044_a580, assign_range(e, this, right, 0, NPOS));
    this
}

// Typed from 0044a540
/// The destructor's body: `_Tidy(true, 0)`.
pub fn destruct(e: &mut Engine, this: u32) {
    typed_call!(FAMILY, 0x0044_a730, tidy(e, this, true, 0));
}

// Typed from 0044a560
/// `c_str()`: `_Myptr()`.
pub fn c_str(e: &mut Engine, this: u32) -> u32 {
    characters(e, this)
}

// Typed from 0044a580
/// `assign(right, offset, count)`: `_Xran` when `offset` is past `right`'s end, `count` cut to what
/// is left; for `right == this` the string erased around the range, otherwise grown and the range
/// copied in. Answers `this`.
pub fn assign_range(e: &mut Engine, this: u32, right: u32, offset: u32, count: u32) -> u32 {
    let left = length_from(e, right, offset).wrapping_sub(offset);
    let count = count.min(left);
    if this == right {
        typed_call!(
            FAMILY,
            0x0044_a690,
            erase(e, this, offset.wrapping_add(count), NPOS)
        );
        typed_call!(FAMILY, 0x0044_a690, erase(e, this, 0, offset));
    } else if grow(e, this, count) {
        let source = characters(e, right).wrapping_add(offset);
        let room = reserve(e, this);
        let destination = characters(e, this);
        e.call(COPY_CHARS, &args![destination, room, source, count]);
        set_length(e, this, count);
    }
    this
}

// Typed from 0044a640
/// `assign(const char* text)`: `assign(text, strlen(text))`, answering what that answers.
pub fn assign_text(e: &mut Engine, this: u32, text: u32) -> u32 {
    let count = typed_call!(FAMILY, 0x0044_a670, text_length(e, text));
    typed_call!(FAMILY, 0x0044_a7f0, assign_chars(e, this, text, count))
}

// Typed from 0044a670
/// The `strlen` wrapper (cdecl): the C runtime's `strlen`.
pub fn text_length(e: &mut Engine, text: u32) -> u32 {
    typed_call!(crt::FAMILY, 0x00ec_6130, crt::string::strlen(e, text))
}

// Typed from 0044a690
/// `erase(position, count)`: `_Xran` when `position` is past the end, `count` cut to what is left;
/// the tail moved down (`move_s`) and the new length set. Answers `this`.
pub fn erase(e: &mut Engine, this: u32, position: u32, count: u32) -> u32 {
    let after = length_from(e, this, position).wrapping_sub(position);
    let count = count.min(after);
    if count != 0 {
        let tail = after.wrapping_sub(count);
        let start = characters(e, this);
        let source = start.wrapping_add(position).wrapping_add(count);
        let room = reserve(e, this).wrapping_sub(position);
        let destination = start.wrapping_add(position);
        e.call(MOVE_CHARS, &args![this, destination, room, source, tail]);
        let new_length = length(e, this).wrapping_sub(count);
        set_length(e, this, new_length);
    }
    this
}

// Typed from 0044a730
/// `_Tidy(built, new_length)`: when `built` and the characters are on the heap, the first
/// `new_length` copied back into the buffer and the heap block freed; then the reserve 0xf and the
/// length `new_length`.
pub fn tidy(e: &mut Engine, this: u32, built: bool, new_length: u32) {
    if built && reserve(e, this) >= HEAP_RESERVE {
        let heap = e.mem.u32(this.wrapping_add(BUFFER));
        if new_length > 0 {
            e.call(
                COPY_CHARS,
                &args![this.wrapping_add(BUFFER), HEAP_RESERVE, heap, new_length],
            );
        }
        let block = reserve(e, this).wrapping_add(1);
        e.call(FREE, &args![this, heap, block]);
    }
    e.mem.set_u32(this.wrapping_add(RESERVE), INLINE_RESERVE);
    set_length(e, this, new_length);
}

// Typed from 0044a7b0
/// The proxy's constructor taking the allocator by value: the proxy's constructor (`0065de30`) is
/// an empty body. Answers `this`.
pub fn construct_proxy(_e: &mut Engine, this: u32, _allocator: u32) -> u32 {
    this
}

// Typed from 0044a7d0
/// The proxy's constructor taking the allocator by reference: as [`construct_proxy`].
pub fn construct_proxy_from(_e: &mut Engine, this: u32, _allocator: u32) -> u32 {
    this
}

// Typed from 0044a7f0
/// `assign(text, count)`: when `text` points into the string, that range of the string assigned
/// (`assign(*this, offset, count)`, answering what it answers); otherwise grown and the characters
/// copied in. Answers `this`.
pub fn assign_chars(e: &mut Engine, this: u32, text: u32, count: u32) -> u32 {
    if holds(e, this, text) {
        let offset = text.wrapping_sub(characters(e, this));
        return typed_call!(
            FAMILY,
            0x0044_a580,
            assign_range(e, this, this, offset, count)
        );
    }
    if grow(e, this, count) {
        let room = reserve(e, this);
        let destination = characters(e, this);
        e.call(COPY_CHARS, &args![destination, room, text, count]);
        set_length(e, this, count);
    }
    this
}
