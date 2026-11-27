//! The path smoother's POV search arrays: the constructors, destructor bodies and buffer
//! allocations of its `BSSimpleArray`s, and the reference-counted pointer holder's constructor
//! and destructor.

use super::{search, FAMILY};
use engine_core::ptr::Ptr;
use engine_core::{args, typed_call, Engine};

/// The vtable of `BSSimpleArray<PathSmootherPOV*,1024>` (`0106ba1c`).
const POV_ARRAY_VTABLE: u32 = 0x0106_ba1c;
/// The vtable of `BSSimpleArray<PathSmootherPOVNode,1024>` (`0106ba30`).
const NODE_ARRAY_VTABLE: u32 = 0x0106_ba30;
/// The vtable of `BSSimpleArray<PathSmootherPOVLink,1024>` (`0106ba44`).
const LINK_ARRAY_VTABLE: u32 = 0x0106_ba44;

/// Callees.
const ARRAY_CLEAR: u32 = 0x0084_54f0;
const ALLOCATE: u32 = 0x0040_1000;
const LINK_ARRAY_INIT: u32 = 0x006a_2130;
const INTERLOCKED_INCREMENT: u32 = 0x0040_f6e0;
const INTERLOCKED_DECREMENT: u32 = 0x0040_1970;

/// A reference count, at `+0x10` of the held object.
const COUNT: u32 = 0x10;

// Typed from 006b3570
/// The `BSSimpleArray<PathSmootherPOV*,1024>` constructor (by the vtable it sets): its vtable
/// `0106ba1c`, then `006b3eb0(this, 0, 0)`. Answers `this`.
pub fn pov_array_ctor(e: &mut Engine, this: Ptr) -> Ptr {
    e.mem.set_u32(this.addr(), POV_ARRAY_VTABLE);
    typed_call!(
        FAMILY,
        0x006b_3eb0,
        search::init_vertices(e, this.cast(), 0, 0)
    );
    this
}

// Typed from 006b35a0
/// The `BSSimpleArray<PathSmootherPOV*,1024>` destructor body (by the vtable it sets): its
/// vtable `0106ba1c`, then the array's release (`008454f0(this, 1)`).
pub fn pov_array_dtor(e: &mut Engine, this: Ptr) {
    e.mem.set_u32(this.addr(), POV_ARRAY_VTABLE);
    e.call(ARRAY_CLEAR, &args![this, 1u32]);
}

// Typed from 006b35c0
/// The `BSSimpleArray<PathSmootherPOVNode,1024>` constructor (by the vtable it sets): its
/// vtable `0106ba30`, then `006b3f40(this, 0, 0)`. Answers `this`.
pub fn node_array_ctor(e: &mut Engine, this: Ptr) -> Ptr {
    e.mem.set_u32(this.addr(), NODE_ARRAY_VTABLE);
    typed_call!(
        FAMILY,
        0x006b_3f40,
        search::init_nodes(e, this.cast(), 0, 0)
    );
    this
}

// Typed from 006b35f0
/// The `BSSimpleArray<PathSmootherPOVNode,1024>` destructor body: its vtable `0106ba30`, then
/// the release (`008454f0(this, 1)`).
pub fn node_array_dtor(e: &mut Engine, this: Ptr) {
    e.mem.set_u32(this.addr(), NODE_ARRAY_VTABLE);
    e.call(ARRAY_CLEAR, &args![this, 1u32]);
}

// Typed from 006b3610
/// The `BSSimpleArray<PathSmootherPOVLink,1024>` constructor (by the vtable it sets): its
/// vtable `0106ba44`, then `006a2130(this, 0, 0)`. Answers `this`.
pub fn link_array_ctor(e: &mut Engine, this: Ptr) -> Ptr {
    e.mem.set_u32(this.addr(), LINK_ARRAY_VTABLE);
    e.call(LINK_ARRAY_INIT, &args![this, 0u32, 0u32]);
    this
}

// Typed from 006b3640
/// The `BSSimpleArray<PathSmootherPOVLink,1024>` destructor body: its vtable `0106ba44`, then
/// the release (`008454f0(this, 1)`).
pub fn link_array_dtor(e: &mut Engine, this: Ptr) {
    e.mem.set_u32(this.addr(), LINK_ARRAY_VTABLE);
    e.call(ARRAY_CLEAR, &args![this, 1u32]);
}

// Typed from 006b36b0
/// The link array's buffer allocation: `00401000(count << 6)` (0x40-byte links).
pub fn link_buffer_allocate(e: &mut Engine, _this: Ptr, count: u32) -> u32 {
    e.call(ALLOCATE, &args![count << 6]).u32()
}

// Typed from 006b3780
/// The `PathEdgeVert` array's buffer allocation: `00401000(count << 5)` (0x20-byte elements).
pub fn edge_vert_buffer_allocate(e: &mut Engine, _this: Ptr, count: u32) -> u32 {
    e.call(ALLOCATE, &args![count << 5]).u32()
}

// Typed from 006b37b0
/// A holder of a reference-counted pointer: stores `object` at `+0` and, when it is not null,
/// increments its count at `+0x10` (`0040f6e0`). Answers `this`.
pub fn holder_new(e: &mut Engine, this: Ptr, object: u32) -> Ptr {
    e.mem.set_u32(this.addr(), object);
    if e.mem.u32(this.addr()) != 0 {
        let held = e.mem.u32(this.addr());
        e.call(INTERLOCKED_INCREMENT, &args![held + COUNT]);
    }
    this
}

// Typed from 006b37e0
/// That holder's destructor: when the pointer is not null, `00401970` on its count at `+0x10`.
pub fn holder_drop(e: &mut Engine, this: Ptr) {
    let held = e.mem.u32(this.addr());
    if held != 0 {
        let held = e.mem.u32(this.addr());
        e.call(INTERLOCKED_DECREMENT, &args![held + COUNT]);
    }
}
