//! The `NavMeshTriangle` accessors: its flag tests, the edge-info flag set/clear with the
//! neighbour slot, the vertex setter, and the mesh's vertex/triangle array append wrappers.

use super::layout::{NavMesh, NavMeshTriangle};
use super::FAMILY;
use engine_core::ptr::Ptr;
use engine_core::{args, typed_call, Engine};

/// Any cover flag (mask `0x0FBE0000`) of `TriangleFlags`.
const COVER_FLAGS: u32 = 0x0fbe_0000;
/// The edge-info flags (bits 0 to 2) of `TriangleFlags`.
const EDGE_INFO_FLAGS: u32 = 0x7;
/// Flag bit `0x400` of `TriangleFlags`.
const FLAG_0X400: u32 = 0x400;
/// No neighbour in a `Triangles[edge]` slot.
const NO_NEIGHBOUR: u16 = 0xffff;

/// Callees.
const ARRAY_ADD_12: u32 = 0x0069_1ab0;

/// The `Triangles[edge]` slot of a triangle (+6, two bytes per edge).
#[inline(always)]
fn neighbour_slot(tri: Ptr<NavMeshTriangle>, edge: u32) -> u32 {
    tri.addr()
        .wrapping_add(6)
        .wrapping_add(edge.wrapping_mul(2))
}

// Typed from 00690770
/// Whether any cover flag (mask `0x0FBE0000`) of `TriangleFlags` is set.
pub fn has_cover_flags(e: &mut Engine, this: Ptr<NavMeshTriangle>) -> bool {
    e.get(this, NavMeshTriangle::flags) & COVER_FLAGS != 0
}

// Typed from 00690e10
/// Whether any edge has extra info (flag bits 0 to 2).
pub fn has_edge_info(e: &mut Engine, this: Ptr<NavMeshTriangle>) -> bool {
    e.get(this, NavMeshTriangle::flags) & EDGE_INFO_FLAGS != 0
}

// Typed from 00690e70
/// Whether flag bit `0x400` is set.
pub fn flag_0x400(e: &mut Engine, this: Ptr<NavMeshTriangle>) -> bool {
    e.get(this, NavMeshTriangle::flags) & FLAG_0X400 != 0
}

// Typed from 00690df0
/// Stores vertex index `value` at `Vertices[index]`.
pub fn set_vertex(e: &mut Engine, this: Ptr<NavMeshTriangle>, index: u32, value: u16) {
    e.mem
        .set_u16(this.addr().wrapping_add(index.wrapping_mul(2)), value);
}

// Typed from 00690790
/// Clears edge info flag `edge` and sets the neighbour slot `Triangles[edge]` to `0xFFFF` (no
/// neighbour).
pub fn clear_edge_info(e: &mut Engine, this: Ptr<NavMeshTriangle>, edge: u32) {
    let flags = e.get(this, NavMeshTriangle::flags);
    e.set(this, NavMeshTriangle::flags, !(1u32 << (edge & 31)) & flags);
    e.mem.set_u16(neighbour_slot(this, edge), NO_NEIGHBOUR);
}

// Typed from 006907d0
/// Clears edge info flag `edge` (as `00690790`), then stores `value` in the neighbour slot
/// `Triangles[edge]`.
pub fn clear_edge_info_to(e: &mut Engine, this: Ptr<NavMeshTriangle>, edge: u32, value: u16) {
    typed_call!(FAMILY, 0x0069_0790, clear_edge_info(e, this, edge));
    e.mem.set_u16(neighbour_slot(this, edge), value);
}

// Typed from 00690e30
/// Sets edge info flag `edge` and stores `value` (an `ExtraEdgeInfo` index) in the neighbour
/// slot `Triangles[edge]`.
pub fn set_edge_info(e: &mut Engine, this: Ptr<NavMeshTriangle>, edge: u32, value: u16) {
    let flags = e.get(this, NavMeshTriangle::flags);
    e.set(this, NavMeshTriangle::flags, (1u32 << (edge & 31)) | flags);
    e.mem.set_u16(neighbour_slot(this, edge), value);
}

// Typed from 00690fe0
/// Appends a copy of the 12-byte vertex at `vertex` to the mesh's vertex array; returns the
/// array's result (the new vertex's index).
pub fn add_vertex(e: &mut Engine, this: Ptr<NavMesh>, vertex: Ptr) -> u32 {
    let vertices = this.at(NavMesh::vertices);
    e.call(ARRAY_ADD_12, &args![vertices, vertex]).u32()
}
