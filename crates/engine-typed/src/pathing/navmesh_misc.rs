//! Smaller `navmesh_util` helpers: the search grid's bounds copy and forwarding, the obstacle
//! record's copy assignments, the triangle-to-obstacle map lookup, the embedded edge arrays'
//! constructor, and the small edge-record pack and key compare.

use super::layout::{NavMesh, NavMeshGrid};
use super::{search, FAMILY};
use engine_core::ptr::Ptr;
use engine_core::{args, typed_call, Engine};

/// Callees.
const GRID_FORWARDED: u32 = 0x0069_d4b0;
const COPY_20: u32 = 0x0069_6ca0;
const MAP_LOOKUP: u32 = 0x0069_afb0;

// Typed from 00696a10
/// `NavMeshGrid` method (`thiscall(grid, min, max)`): copies the bounds minimum to the three
/// floats at `min` and the bounds maximum to those at `max`.
pub fn grid_bounds_copy(e: &mut Engine, this: Ptr<NavMeshGrid>, min: Ptr, max: Ptr) {
    for word in 0..3 {
        let value = e.mem.u32(this.addr() + 0x0c + 4 * word);
        e.mem.set_u32(min.addr() + 4 * word, value);
    }
    for word in 0..3 {
        let value = e.mem.u32(this.addr() + 0x18 + 4 * word);
        e.mem.set_u32(max.addr() + 4 * word, value);
    }
}

// Typed from 00696c00
/// Forwards three words to `0069d4b0` on the mesh's search grid (`thiscall(mesh, a, b, c)`).
pub fn grid_forward(e: &mut Engine, this: Ptr<NavMesh>, first: u32, second: u32, third: u32) {
    e.call(
        GRID_FORWARDED,
        &args![this.at(NavMesh::grid), first, second, third],
    );
}

// Typed from 00696c30
/// Copy assignment of the 0x54-byte structure (`thiscall(this, other)`): [`copy_44`] (the first
/// 0x44 bytes), then the word at +0x50. Returns `this`.
pub fn copy_assign_54(e: &mut Engine, this: Ptr, other: Ptr) -> Ptr {
    let _: Ptr = typed_call!(FAMILY, 0x0069_6c60, copy_44(e, this, other));
    let value = e.mem.u32(other.addr() + 0x50);
    e.mem.set_u32(this.addr() + 0x50, value);
    this
}

// Typed from 00696c60
/// Copy assignment of the 0x44-byte structure (`thiscall(this, other)`): `00696ca0` (the first
/// 0x20 bytes), then the eight words at +0x20 and the word at +0x40. Returns `this`.
pub fn copy_44(e: &mut Engine, this: Ptr, other: Ptr) -> Ptr {
    e.call(COPY_20, &args![this, other]);
    for word in 0..8 {
        let value = e.mem.u32(other.addr() + 0x20 + 4 * word);
        e.mem.set_u32(this.addr() + 0x20 + 4 * word, value);
    }
    let value = e.mem.u32(other.addr() + 0x40);
    e.mem.set_u32(this.addr() + 0x40, value);
    this
}

// Typed from 006997b0
/// Looks a triangle (16-bit key) up in the navmesh's triangle-to-obstacle map (the word at
/// `this + 0xe0`), assigning the mapped `NiPointer` to `out` (`0069afb0`), and answers whether it
/// was found.
pub fn obstacle_map_lookup(e: &mut Engine, this: Ptr, key: u16, out: Ptr) -> bool {
    let map = e.mem.u32(this.addr() + 0xe0);
    e.call(MAP_LOOKUP, &args![map, key, out]).bool()
}

// Typed from 0069a730
/// Constructs the embedded edge-list members at `this + 8` and `this + 0x18`
/// ([`search::edge_array_new`]); returns `this`.
pub fn embedded_lists_new(e: &mut Engine, this: Ptr) -> Ptr {
    typed_call!(
        FAMILY,
        0x0069_b2c0,
        search::edge_array_new(e, Ptr::new(this.addr() + 8))
    );
    typed_call!(
        FAMILY,
        0x0069_b2c0,
        search::edge_array_new(e, Ptr::new(this.addr() + 0x18))
    );
    this
}

// Typed from 0069a700
/// Packs the low 14 bits of the word at `record + 4` and the low two bits of the word at
/// `record + 8` into the 16-bit edge descriptor at `out`.
pub fn pack_edge_descriptor(e: &mut Engine, out: Ptr, record: Ptr) {
    let low = e.mem.u16(record.addr() + 4) & 0x3fff;
    let high = (e.mem.u16(record.addr() + 8) & 3) << 14;
    e.mem.set_u16(out.addr(), low | high);
}

// Typed from 0069ae60
/// Compares the unsigned 16-bit keys at `first` and `second`: 1 when the first is greater, -1
/// when the second is greater, 0 when they are equal.
pub fn compare_keys(e: &mut Engine, first: Ptr, second: Ptr) -> u32 {
    match e.mem.u16(first.addr()).cmp(&e.mem.u16(second.addr())) {
        std::cmp::Ordering::Greater => 1,
        std::cmp::Ordering::Less => u32::MAX,
        std::cmp::Ordering::Equal => 0,
    }
}
