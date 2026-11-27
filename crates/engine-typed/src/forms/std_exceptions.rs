//! The C++ library's `std::logic_error`, `std::length_error` and `std::bad_alloc` members the linker
//! placed beside the debug-heap string (`0044a2e0` to `0044a450`, `0044a9a0` and `0044dd10`; the
//! classes by the vtables they store, whose RTTI names them): the constructors from a message, the
//! copy constructors, `what()`, the destructor bodies and the scalar deleting destructors.
//!
//! The object: `std::exception`'s vtable and its two words at +0, +4 and +8, then (for the two
//! `logic_error` classes) the message, a debug-heap string ([`super::debug_string`]), at +0xc.
//!
//! **Native.** The message string's members (the copy constructor, `c_str`, the destructor) and
//! the classes' own members are typed calls. What stays a call by address: `std::exception`'s
//! constructor, copy constructor and destructor (`00ec78e0`, `00ec7944`, `00ec79a1`, the C
//! runtime's) and `operator delete` (`00401030`). The constructors' unwinding frames (which destroy
//! the base when the message copy throws) are left out as the translations leave them: the copy
//! answers or stops the game.
//!
//! No floats.

use super::{debug_string, FAMILY};
use engine_core::{args, typed_call, Engine};

/// The vtables: `std::logic_error` (`010173e8`), `std::length_error` (`010173dc`) and
/// `std::bad_alloc` (`010176f8`), by their RTTI.
const LOGIC_ERROR_VTABLE: u32 = 0x0101_73e8;
const LENGTH_ERROR_VTABLE: u32 = 0x0101_73dc;
const BAD_ALLOC_VTABLE: u32 = 0x0101_76f8;
/// The message string.
const MESSAGE: u32 = 0xc;
/// `std::exception`'s constructor (`thiscall`, no stack word), copy constructor (`thiscall`, the
/// source) and destructor (`thiscall`).
const EXCEPTION_CONSTRUCT: u32 = 0x00ec_78e0;
const EXCEPTION_COPY: u32 = 0x00ec_7944;
const EXCEPTION_DESTRUCT: u32 = 0x00ec_79a1;
/// `operator delete` (cdecl).
const OPERATOR_DELETE: u32 = 0x0040_1030;

/// The scalar deleting destructors' end: the block freed when bit 0 of `flags` is set; `this`.
fn delete_if_flagged(e: &mut Engine, this: u32, flags: u32) -> u32 {
    if flags & 1 != 0 {
        e.call(OPERATOR_DELETE, &args![this]);
    }
    this
}

// Typed from 0044a2e0
/// `std::length_error(const string& message)`: `logic_error`'s constructor with the message, then
/// the vtable. Answers `this`.
pub fn length_error_construct(e: &mut Engine, this: u32, message: u32) -> u32 {
    typed_call!(FAMILY, 0x0044_a310, logic_error_construct(e, this, message));
    e.mem.set_u32(this, LENGTH_ERROR_VTABLE);
    this
}

// Typed from 0044a310
/// `std::logic_error(const string& message)`: `std::exception`'s constructor, the vtable, then the
/// message copied into the string at +0xc. Answers `this`.
pub fn logic_error_construct(e: &mut Engine, this: u32, message: u32) -> u32 {
    e.call(EXCEPTION_CONSTRUCT, &args![this]);
    e.mem.set_u32(this, LOGIC_ERROR_VTABLE);
    typed_call!(
        FAMILY,
        0x0044_a500,
        debug_string::copy_construct(e, this.wrapping_add(MESSAGE), message)
    );
    this
}

// Typed from 0044a380
/// `logic_error::what()`: the message's characters (`c_str` of the string at +0xc).
pub fn logic_error_what(e: &mut Engine, this: u32) -> u32 {
    typed_call!(
        FAMILY,
        0x0044_a560,
        debug_string::c_str(e, this.wrapping_add(MESSAGE))
    )
}

// Typed from 0044a3a0
/// `logic_error`'s scalar deleting destructor: the destructor's body, then the block freed when bit
/// 0 of `flags` is set. Answers `this`.
pub fn logic_error_deleting_destructor(e: &mut Engine, this: u32, flags: u32) -> u32 {
    typed_call!(FAMILY, 0x0044_a3d0, logic_error_destruct(e, this));
    delete_if_flagged(e, this, flags)
}

// Typed from 0044a3d0
/// `logic_error`'s destructor body: the vtable, the message's destructor, then `std::exception`'s.
pub fn logic_error_destruct(e: &mut Engine, this: u32) {
    e.mem.set_u32(this, LOGIC_ERROR_VTABLE);
    typed_call!(
        FAMILY,
        0x0044_a540,
        debug_string::destruct(e, this.wrapping_add(MESSAGE))
    );
    e.call(EXCEPTION_DESTRUCT, &args![this]);
}

// Typed from 0044a400
/// `length_error`'s destructor body: its vtable, then `logic_error`'s body.
pub fn length_error_destruct(e: &mut Engine, this: u32) {
    e.mem.set_u32(this, LENGTH_ERROR_VTABLE);
    typed_call!(FAMILY, 0x0044_a3d0, logic_error_destruct(e, this));
}

// Typed from 0044a420
/// `length_error`'s scalar deleting destructor: its destructor's body, then the block freed when bit
/// 0 of `flags` is set. Answers `this`.
pub fn length_error_deleting_destructor(e: &mut Engine, this: u32, flags: u32) -> u32 {
    typed_call!(FAMILY, 0x0044_a400, length_error_destruct(e, this));
    delete_if_flagged(e, this, flags)
}

// Typed from 0044a450
/// `logic_error`'s copy constructor: `std::exception`'s copy, the vtable, then the other's message
/// copied into this one's. Answers `this`.
pub fn logic_error_copy(e: &mut Engine, this: u32, other: u32) -> u32 {
    e.call(EXCEPTION_COPY, &args![this, other]);
    e.mem.set_u32(this, LOGIC_ERROR_VTABLE);
    typed_call!(
        FAMILY,
        0x0044_a500,
        debug_string::copy_construct(e, this.wrapping_add(MESSAGE), other.wrapping_add(MESSAGE))
    );
    this
}

// Typed from 0044a9a0
/// `length_error`'s copy constructor: `logic_error`'s copy, then its vtable. Answers `this`.
pub fn length_error_copy(e: &mut Engine, this: u32, other: u32) -> u32 {
    typed_call!(FAMILY, 0x0044_a450, logic_error_copy(e, this, other));
    e.mem.set_u32(this, LENGTH_ERROR_VTABLE);
    this
}

// Typed from 0044dd10
/// `bad_alloc`'s copy constructor: `std::exception`'s copy, then the vtable. Answers `this`.
pub fn bad_alloc_copy(e: &mut Engine, this: u32, other: u32) -> u32 {
    e.call(EXCEPTION_COPY, &args![this, other]);
    e.mem.set_u32(this, BAD_ALLOC_VTABLE);
    this
}
