//! The path builder's small accessors: the solution's incomplete flag, the request's heading
//! fields, the virtual node's flag bit, the request's lenient byte, and the ray-cast path's
//! constructor, destructor and ray-cast request destructor.

use engine_core::x87::x87_quiet_bits;
use engine_core::Engine;

/// Callees.
const EDGE_ARRAY_DESTRUCT: u32 = 0x006c_f230;
const LOCATION_DESTRUCT: u32 = 0x004f_f7e0;
const LOCATION_ARRAY_CTOR: u32 = 0x006c_f180;
const LOCATION_ARRAY_DESTRUCT: u32 = 0x006a_bfc0;

// Typed from 006c9fa0
/// Stores `value` in the solution's `bIncompletePath` byte (+0x40).
pub fn set_incomplete(e: &mut Engine, this: u32, value: u8) {
    e.mem.set_u8(this + 0x40, value);
}

// Typed from 006ca4c0
/// The request's `bFirstTangentUsesHeading` byte (+0xA0).
pub fn first_tangent_uses_heading(e: &mut Engine, this: u32) -> bool {
    e.mem.u8(this + 0xa0) != 0
}

// Typed from 006ca4e0
/// The request's `fInitialPathHeading` float (+0x90), as `FLD` loads it (a signalling NaN
/// quieted).
pub fn initial_heading(e: &mut Engine, this: u32) -> f32 {
    f32::from_bits(x87_quiet_bits(e.mem.u32(this + 0x90)))
}

// Typed from 006ca830
/// Whether bit 0 of the virtual node's `iFlags` (+0) is set.
pub fn node_has_flag_1(e: &mut Engine, this: u32) -> bool {
    e.mem.u32(this) & 1 != 0
}

// Typed from 006caa20
/// The request's byte at +0xA1.
pub fn request_byte_a1(e: &mut Engine, this: u32) -> bool {
    e.mem.u8(this + 0xa1) != 0
}

// Typed from 006cb410
/// The `RayCastPathRequest` destructor: `Edges` (+0x58), `GoalLocation` (+0x30), `StartLocation`
/// (+0x08), in that order.
pub fn ray_cast_request_drop(e: &mut Engine, this: u32) {
    e.call(EDGE_ARRAY_DESTRUCT, &[this + 0x58]);
    e.call(LOCATION_DESTRUCT, &[this + 0x30]);
    e.call(LOCATION_DESTRUCT, &[this + 0x08]);
}

// Typed from 006cb480
/// Constructs the `BSSimpleArray<PathingLocation>` at +8 (`006cf180`); returns `this`.
pub fn ray_cast_path_new(e: &mut Engine, this: u32) -> u32 {
    e.call(LOCATION_ARRAY_CTOR, &[this + 8]);
    this
}

// Typed from 006cb4a0
/// Destroys the location array at +8 (`006abfc0`).
pub fn ray_cast_path_drop(e: &mut Engine, this: u32) {
    e.call(LOCATION_ARRAY_DESTRUCT, &[this + 8]);
}
