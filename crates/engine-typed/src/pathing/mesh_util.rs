//! The navmesh utilities: the triangle's packed-flag getters, the unsigned max helper, the
//! obstacle-undo removal, and the embedded-list destructor.

use super::layout::NavMeshTriangle;
use engine_core::ptr::Ptr;
use engine_core::{args, Engine};

/// The packed edge bits of `TriangleFlags` (mask `0x0FFF0000`).
const PACKED_EDGE_BITS: u32 = 0x0fff_0000;
/// The three-bit type field of the word at `+0xC` (bits 13 to 15).
const TYPE_FIELD: u32 = 0xe000;

/// Callees.
const UNDO_REMOVE: u32 = 0x0069_b120;
const LIST_DROP: u32 = 0x0069_b2f0;

/// The obstacle undo array (+0xD0) and the embedded lists (+0x18, +8).
const UNDO_ARRAY: u32 = 0xd0;
const LIST_A: u32 = 0x18;
const LIST_B: u32 = 8;

// Typed from 006957d0
/// Whether any of the packed edge bits (flags `0x0FFF0000`) is set.
pub fn has_packed_edge_bits(e: &mut Engine, this: Ptr<NavMeshTriangle>) -> bool {
    e.get(this, NavMeshTriangle::flags) & PACKED_EDGE_BITS != 0
}

// Typed from 00698030
/// The four packed bits of edge `edge` (bits `16 + 6 * edge` upward of the flags).
pub fn packed_edge_bits(e: &mut Engine, this: Ptr<NavMeshTriangle>, edge: u16) -> u32 {
    let shift = u32::from(edge).wrapping_mul(6).wrapping_add(0x10);
    (e.get(this, NavMeshTriangle::flags) >> (shift & 31)) & 0xf
}

// Typed from 00699760
/// The greater of an unsigned word and an unsigned 32-bit value (cdecl, two words).
pub fn max_word(_e: &mut Engine, value: u32, candidate: u16) -> u32 {
    value.max(u32::from(candidate))
}

// Typed from 00699790
/// The three-bit type encoded in bits 13 through 15 of the word at `this + 0xC`.
pub fn triangle_type(e: &mut Engine, this: Ptr) -> u32 {
    (e.mem.u32(this.addr() + 0x0c) & TYPE_FIELD) >> 13
}

// Typed from 00699590
/// Removes one item from the obstacle undo array (+0xD0) at `index`.
pub fn undo_remove(e: &mut Engine, this: Ptr, index: u32) {
    e.call(UNDO_REMOVE, &args![this.addr() + UNDO_ARRAY, index, 1u32]);
}

// Typed from 0069a7d0
/// Destroys the embedded list members at `this + 0x18` and `this + 8`.
pub fn drop_embedded_lists(e: &mut Engine, this: Ptr) {
    e.call(LIST_DROP, &args![this.addr() + LIST_A]);
    e.call(LIST_DROP, &args![this.addr() + LIST_B]);
}
