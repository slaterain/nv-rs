//! The POV search's small members: the edge-reference word getters, the wrapping-add key
//! helper, the 0x24-byte element address and its forwarder, the first-triangle step, the
//! scrap-heap init, the byte-count helper, and two `BSSimpleArray` destructor bodies.

use super::layout::NavMeshTriangle;
use super::{navmesh, FAMILY};
use engine_core::ptr::Ptr;
use engine_core::{args, typed_call, Engine};

/// The vtable of `BSSimpleArray<AStarNode<NavMeshPOVSearchNode, NavMeshPOVSearchLink>,1024>`
/// (`0106b44c`).
const NODE_ARRAY_VTABLE: u32 = 0x0106_b44c;
/// The vtable of `BSSimpleArray<FastNavMeshVertHandle,1024>` (`0106b460`).
const VERT_HANDLE_ARRAY_VTABLE: u32 = 0x0106_b460;

/// Callees.
const ARRAY_RELEASE: u32 = 0x0084_54f0;
const SCRAP_INIT: u32 = 0x00aa_5410;
const BYTES: u32 = 0x0042_f5d0;

/// A `BSSimpleArray`'s buffer (+4); a 0x24-byte element's stride.
const BUFFER: u32 = 4;
const ELEMENT_24: u32 = 0x24;

// Typed from 0069ef40
/// The low 14 bits of the 16-bit word at `this`.
pub fn low_14(e: &mut Engine, this: Ptr) -> u32 {
    u32::from(e.mem.u16(this.addr())) & 0x3fff
}

// Typed from 0069ef60
/// The top two bits of the 16-bit word at `this` (0 to 3).
pub fn top_2(e: &mut Engine, this: Ptr) -> u32 {
    (u32::from(e.mem.u16(this.addr())) & 0xc000) >> 0xe
}

// Typed from 0069f900
/// `a + b` (wrapping); `this` unread.
pub fn wrapping_add(_e: &mut Engine, _this: Ptr, a: u32, b: u32) -> u32 {
    a.wrapping_add(b)
}

// Typed from 006a1490
/// The address of element `index` of a `BSSimpleArray` of 0x24-byte elements (buffer at +4).
pub fn element_24(e: &mut Engine, this: Ptr, index: u32) -> u32 {
    index
        .wrapping_mul(ELEMENT_24)
        .wrapping_add(e.mem.u32(this.addr() + BUFFER))
}

// Typed from 006a1400
/// Forwards to `006a1490`.
pub fn element_24_forward(e: &mut Engine, this: Ptr, index: u32) -> u32 {
    let r: u32 = typed_call!(FAMILY, 0x006a_1490, element_24(e, this, index));
    r
}

// Typed from 006a1420
/// `00558dd0(this, 0)`, its result returned.
pub fn first_triangle(e: &mut Engine, this: Ptr) -> u32 {
    let t: Ptr<NavMeshTriangle> =
        typed_call!(FAMILY, 0x0055_8dd0, navmesh::triangle_at(e, this.cast(), 0));
    t.addr()
}

// Typed from 006a1f70
/// `00aa5410(this, 0xD000)`; answers `this`.
pub fn scrap_init(e: &mut Engine, this: Ptr) -> Ptr {
    e.call(SCRAP_INIT, &args![this, 0xd000u32]);
    this
}

// Typed from 006a28b0
/// `0042f5d0(first, count * 8)`; `this` unread.
pub fn byte_span(e: &mut Engine, _this: Ptr, first: u32, count: u32) -> u32 {
    e.call(BYTES, &args![first, count << 3]).u32()
}

// Typed from 006a2580
/// The node array's destructor body (by the vtable it sets): vtable `0106b44c`, then the release
/// (`008454f0(this, 1)`).
pub fn node_array_dtor(e: &mut Engine, this: Ptr) {
    e.mem.set_u32(this.addr(), NODE_ARRAY_VTABLE);
    e.call(ARRAY_RELEASE, &args![this, 1u32]);
}

// Typed from 006a2900
/// The vert-handle array's destructor body (by the vtable it sets): vtable `0106b460`, then the
/// release (`008454f0(this, 1)`).
pub fn vert_handle_array_dtor(e: &mut Engine, this: Ptr) {
    e.mem.set_u32(this.addr(), VERT_HANDLE_ARRAY_VTABLE);
    e.call(ARRAY_RELEASE, &args![this, 1u32]);
}
