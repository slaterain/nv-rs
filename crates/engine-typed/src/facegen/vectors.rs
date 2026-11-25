//! The face data's `std::vector` of 6-byte elements (a word and a 16-bit value; the vector held at
//! +4 of each 0x1c-byte record of the face data, after its float): its `_Tidy` (`0065f430`), which
//! the vector's destructor (`0065ee70`) and the records' resize and insert (`00ab0d50`, `00ab1800`)
//! run. The translation destroys each element through the allocator's `destroy` (`0065e9a0`, by
//! address), one call an element: about 2,500 a frame on the street while heads are made. That
//! destroy calls only the empty `0040fbe0`, so here nothing is called for an element that goes.
//!
//! What stays by address: the allocator's `deallocate` (`0064df10`, `operator delete` on the
//! storage), with the exe's words. What is done in place: the iterator category tag (`0065e750`),
//! which only answers a byte that the range destructor (`0065ff00`) does not read, and that range
//! destructor's loop of empty destroys.
//!
//! No float maths.

use engine_core::ptr::Ptr;
use engine_core::{args, Engine};

/// The vector's allocator, its first, last and end-of-storage pointers.
const ALLOCATOR: u32 = 8;
const FIRST: u32 = 0xc;
const LAST: u32 = 0x10;
const END: u32 = 0x14;

/// The allocator's `deallocate` (`0064df10`, `thiscall` on the allocator: the storage and its
/// element count, which it does not read).
const DEALLOCATE: u32 = 0x0064_df10;

// Typed from 0065f430
/// `_Tidy()` of the 6-byte element vector (`thiscall`): when it has storage, its elements are
/// destroyed (each destroy is empty: no call) and the storage goes to the allocator's
/// `deallocate` with its element count (end minus first, signed, over 6); then the first, last and
/// end pointers are cleared. The exe's destroy loop runs while the element pointer differs from the
/// last, so a range that is not whole elements would never end it; the vector only ever holds whole
/// elements.
pub fn tidy(e: &mut Engine, this: Ptr) {
    let vector = this.addr();
    let field = |offset: u32| vector.wrapping_add(offset);
    let first = e.mem.u32(field(FIRST));
    if first != 0 {
        let bytes = e.mem.u32(field(END)).wrapping_sub(first) as i32;
        let count = bytes.wrapping_div(6) as u32;
        e.call(DEALLOCATE, &args![field(ALLOCATOR), first, count]);
    }
    e.mem.set_u32(field(FIRST), 0);
    e.mem.set_u32(field(LAST), 0);
    e.mem.set_u32(field(END), 0);
}
