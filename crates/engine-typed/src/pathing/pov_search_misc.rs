//! Small `navmeshpovsearch` helpers: the packed edge-reference unpackers, the word adder, the
//! scrap heap init, the block copy, the first-element getter, and the solution node append.

use super::{navmesh, FAMILY};
use engine_core::ptr::Ptr;
use engine_core::{args, typed_call, Engine};

/// Callees.
const SCRAP_INIT: u32 = 0x00aa_5410;
const BYTES_COPY: u32 = 0x0042_f5d0;
const ARRAY_INDEX: u32 = 0x006a_1920;
const ARRAY_AT: u32 = 0x006e_8f80;
const PLACEMENT_NEW: u32 = 0x006e_6da0;
const NODE_CTOR: u32 = 0x006e_2280;

// Typed from 0069ef40
/// The low 14 bits of the 16-bit word at `this`.
pub fn low_14_bits(e: &mut Engine, this: Ptr) -> u32 {
    u32::from(e.mem.u16(this.addr())) & 0x3fff
}

// Typed from 0069ef60
/// The top two bits of the 16-bit word at `this` (0 to 3).
pub fn top_2_bits(e: &mut Engine, this: Ptr) -> u32 {
    (u32::from(e.mem.u16(this.addr())) & 0xc000) >> 0xe
}

// Typed from 0069f900
/// `a + b` (wrapping); `this` unread (`RET 8`).
pub fn add_words(_e: &mut Engine, _this: Ptr, a: u32, b: u32) -> u32 {
    a.wrapping_add(b)
}

// Typed from 006a1420
/// `00558dd0(this, 0)`, its result returned ([`navmesh::triangle_at`]).
pub fn first_element(e: &mut Engine, this: Ptr) -> u32 {
    let p: Ptr<_> = typed_call!(FAMILY, 0x0055_8dd0, navmesh::triangle_at(e, this.cast(), 0));
    p.addr()
}

// Typed from 006a1f70
/// `00aa5410(this, 0xD000)`. Returns `this`.
pub fn scrap_init(e: &mut Engine, this: Ptr) -> Ptr {
    e.call(SCRAP_INIT, &args![this, 0xd000u32]);
    this
}

// Typed from 006a28b0
/// `0042f5d0(first, count * 8)`, its result returned; `this` unread (`RET 8`).
pub fn block_copy(e: &mut Engine, _this: Ptr, first: u32, count: u32) -> u32 {
    e.call(BYTES_COPY, &args![first, count << 3]).u32()
}

// Typed from 0069f920
/// Appends a 0x3C-byte node to the array at `this + 0x20`: reserves an index (`006a1920`), takes
/// its element (`006e8f80`), placement-`new`s it (`006e6da0(0x3C, place)`) and, when that gives a
/// place, constructs it with `first` and `second` (`006e2280`). Returns the index.
pub fn append_node(e: &mut Engine, this: Ptr, first: u32, second: u32) -> u32 {
    let array = this.addr() + 0x20;
    let index = e.call(ARRAY_INDEX, &args![array]).u32();
    let place = e.call(ARRAY_AT, &args![array, index]).u32();
    let node = e.call(PLACEMENT_NEW, &args![0x3cu32, place]).u32();
    if node != 0 {
        e.call(NODE_CTOR, &args![node, first, second]);
    }
    index
}
