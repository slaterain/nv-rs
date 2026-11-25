//! The navmesh edges family (`NV_TYPED=navmesh_edges`): the rebuild of a navmesh's boundary-edge pairs
//! and of its per-triangle preferred vertices, `00699b60`, which the game runs for every navmesh a cell
//! brings in (`00693e60` after `NavMesh::Load`, and the two navmesh repairs, `0069a9ef` and its sibling).
//!
//! The exe's function has four passes:
//!
//! 1. Every boundary edge (no neighbour, or an edge to another mesh) of every triangle goes into a
//!    temporary map keyed by its two vertices: a record per vertex with its ordinary and its portal
//!    edges.
//! 2. A vertex with exactly two such edges, the corner they make tested, gets the packed pair of its
//!    edges in the mesh's pair map (`+0x88`). The records are then deleted.
//! 3. For every triangle, the pair map is walked whole: the vertex nearest the triangle's
//!    circumcenter in x and y, within 180 units in z, is the triangle's preferred vertex (`+0x98`).
//! 4. The temporary map is destroyed.
//!
//! Pass 3 is the cost: triangles times pairs, four calls by address for each pair (the map's next item
//! `00694d30`, the vertex's point `0068f0a0`, the difference `00439ef0`, its squared length in x and y
//! `00595c80`). A large exterior navmesh makes hundreds of thousands of them while a cell loads, which
//! the lead's call count saw as about 11,000 a frame on the street.
//!
//! **Native.** Passes 1, 2 and 4 keep the translation's order of work and its calls, with the calls into
//! typed families (pathing, the leaves, dispatch) made `typed_call!`s: they are per boundary edge and few.
//! The 12-byte array entries (`006a1440`, a read) are read in place.
//! Pass 3 reads the pair map once, in the order the exe's walk visits it (`004b9ba0`, then `00694d30`
//! as it steps from item to item and from bucket to bucket), with each pair's vertex point, and then
//! does each triangle's search over that list in plain Rust: no call by address and no guest memory per
//! pair. Nothing in pass 3 writes the pair map or the vertices (its only writes are the preferred-vertex
//! array, a different block, and its own locals), so every triangle's walk sees what the first one saw.
//! The calls pass 3 still makes are the exe's per triangle (its corners, the planar length `00589850`,
//! the append `00691820`) and the absolute value `00408840` for each pair that comes nearer, as the
//! exe calls it.
//!
//! **Floats**, ADR-0014's rule as the pathing family has it ([`crate::pathing`]): a difference of two
//! floats is plain `f32` (one operation: the same float at either precision); the squared length in x
//! and y is `f32` at each step on the main thread (its 24-bit control word rounds every step to a
//! float) and `f64` rounded once on the other threads, as `00595c80` computes there. The circumcenter
//! keeps the translation's doubles (`00697290` is translated, not typed), and the comparisons are the
//! exe's (`FCOM`, a NaN never nearer). Integers and the order of every guest write are exact.

use crate::culling::layout::NiPoint3;
use crate::dispatch;
use crate::native::{float_precision, to_float};
use crate::pathing::layout::{MeshEdge, NavMesh, Point2, SimpleArray};
use crate::pathing::{self, double_global, float_global, navmesh, point, pov, search};
use crate::slot_target;
use engine_core::ptr::Ptr;
use engine_core::typed::{leaf, Family, FamilyId, Threads};
use engine_core::{args, entry, typed_call, AbiFn, Engine};

/// This family's bit: what each `typed_call!` of it names. Id 33: the next free after every family
/// registered in main (0-2, 5-13, 15-28, 30 in production; 14, 29 and 31 held by the lead) and 32,
/// which the archives family takes.
pub const FAMILY: FamilyId = FamilyId::new(33);

/// `NV_TYPED=navmesh_edges`. A native conversion proved by its differential tests (the maintainer's
/// rule of 2026-11-11: native code merges on): on unless `NV_TYPED` says otherwise
/// (`NV_TYPED=off` or `navmesh_edges:off`). Navmeshes load on main and, with background loading, on
/// the loader threads.
pub static NAVMESH_EDGES: Family = Family {
    name: "navmesh_edges",
    id: FAMILY,
    past_gate: true,
    threads: Threads::Both,
    stage2: false,
    funcs,
};

/// The mesh's map of a vertex to its packed edge pair (`NiTMap<unsigned short, ..>`, the
/// point-of-view map's class: vtable, bucket count, bucket table, item count; each item the next item,
/// the 16-bit key at +4 and the value at +6).
const EDGE_PAIRS: u32 = 0x88;
/// The mesh's array of each triangle's preferred vertex (`BSSimpleArray<unsigned short>`).
const PREFERRED: u32 = 0x98;

/// The temporary vertex map's vtable and bucket count (`0069ad40` with `0x25`), and its base's vtable
/// (`0069b730`, the destructor's second half).
const VERTEX_MAP_VTABLE: u32 = 0x0106_b184;
const VERTEX_MAP_BASE_VTABLE: u32 = 0x0106_b1f4;
const VERTEX_MAP_BUCKETS: u32 = 0x25;
/// A vertex record (`POVVertexData`, Xbox PDB, 0x28 bytes): its ordinary edges and its portal edges,
/// each a `BSSimpleArray` of 12-byte edge handles.
const RECORD_SIZE: u32 = 0x28;
const RECORD_EDGES: u32 = 8;
const RECORD_PORTAL_EDGES: u32 = 0x18;

/// The exe's constants: the float 128 widening a triangle's reach, the doubles 0, 0.1 and -0.1 (the
/// corner test, each a float widened) and 180 (the height limit).
const REACH: u32 = 0x0101_e704;
const ZERO: u32 = 0x0101_2060;
const POSITIVE_LIMIT: u32 = 0x0101_ffa0;
const NEGATIVE_LIMIT: u32 = 0x0106_b170;
const HEIGHT_LIMIT: u32 = 0x0104_ed58;

/// The hash a `NiTMap<unsigned short, ..>` holds in its vtable slot 4: the key modulo the bucket count.
const HASH_16: u32 = 0x0049_c4c0;
/// No vertex, no triangle.
const NONE: u16 = 0xffff;

// The translated callees, by address.
/// `NiTMapBase::RemoveAll` (empties a map, its items freed through its vtable).
const MAP_REMOVE_ALL: u32 = 0x0043_8af0;
/// The vertex map's base constructor (`this`, bucket count): allocates and clears the bucket table.
const VERTEX_MAP_BASE_NEW: u32 = 0x0069_b360;
/// Frees a map's bucket table.
const BUCKETS_FREE: u32 = 0x00aa_10f0;
/// `operator new` and `operator delete`.
const OPERATOR_NEW: u32 = 0x0040_1000;
const OPERATOR_DELETE: u32 = 0x0040_1030;
/// A record's destructor body (its two edge arrays).
const RECORD_DESTRUCT: u32 = 0x0069_a7d0;
/// The next free slot of a `BSSimpleArray` (grows it), and the construction of 12-byte edge handles in
/// place (`this`, address, count).
const ARRAY_NEXT_SLOT: u32 = 0x0097_8bc0;
const EDGE_HANDLES_NEW: u32 = 0x0069_b9d0;
/// `NiPoint3::Cross` (`this`, out, other).
const POINT3_CROSS: u32 = 0x004b_3800;
/// `NiTMapBase::SetAt` of the pair map (`this`, key, value).
const PAIR_MAP_SET: u32 = 0x0069_4c50;

/// The family's functions, by exe address.
fn funcs() -> Vec<(u32, AbiFn)> {
    vec![entry!(0x0069_9b60, rebuild_edge_pairs(Ptr<NavMesh>))]
}

// ---------------------------------------------------------------------------
// Small helpers
// ---------------------------------------------------------------------------

/// A word of guest memory.
#[inline(always)]
fn word(e: &Engine, at: u32) -> u32 {
    e.mem.u32(at)
}

/// The three floats at `at`.
#[inline(always)]
fn read_point(e: &Engine, at: u32) -> [f32; 3] {
    [
        f32::from_bits(word(e, at)),
        f32::from_bits(word(e, at.wrapping_add(4))),
        f32::from_bits(word(e, at.wrapping_add(8))),
    ]
}

/// Stores three floats at `at`, x first.
#[inline(always)]
fn write_point(e: &mut Engine, at: u32, p: [f32; 3]) {
    e.mem.set_u32(at, p[0].to_bits());
    e.mem.set_u32(at.wrapping_add(4), p[1].to_bits());
    e.mem.set_u32(at.wrapping_add(8), p[2].to_bits());
}

/// The low 16 bits of a word: a triangle index as the exe passes it (`MOVZX` of its loop counter).
#[allow(clippy::cast_possible_truncation)]
#[inline(always)]
fn low_16(v: u32) -> u16 {
    v as u16
}

/// `a - b` for each axis: `NiPoint3::operator-` (`00439ef0`), one float operation each.
#[inline(always)]
fn difference(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

/// Entry `index` of an array of 12-byte entries, unchecked: `006a1440`'s read, done in place.
#[inline(always)]
fn entry_12(e: &Engine, array: Ptr<SimpleArray>, index: u32) -> u32 {
    index
        .wrapping_mul(0xc)
        .wrapping_add(e.get(array, SimpleArray::data))
}

/// The address of vertex `index`'s point (`0068f0a0` through `006a1440`): entry `index` of the mesh's
/// vertex array.
#[inline(always)]
fn vertex_point(e: &Engine, mesh: Ptr<NavMesh>, index: u16) -> u32 {
    entry_12(e, mesh.at(NavMesh::vertices), u32::from(index))
}

/// The first item of a `NiTMap` (`004b9ba0`): the head of its first non-empty bucket, or 0.
fn first_item(e: &Engine, map: u32) -> u32 {
    let size = word(e, map.wrapping_add(4));
    let table = word(e, map.wrapping_add(8));
    (0..size)
        .map(|bucket| word(e, table.wrapping_add(bucket.wrapping_mul(4))))
        .find(|&head| head != 0)
        .unwrap_or(0)
}

/// The item after `item` of the pair map, as `00694d30` moves its position: the next in its bucket,
/// else the head of the first non-empty bucket after the key's (the map's hash, vtable slot 4), else 0.
/// The pair map's class holds [`HASH_16`] there, done here; any other hash is called.
fn next_pair(e: &mut Engine, map: u32, item: u32, key: u16) -> u32 {
    let next = word(e, item);
    if next != 0 {
        return next;
    }
    let mut bucket = if slot_target(e, map, 4) == HASH_16 {
        u32::from(key) % word(e, map.wrapping_add(4))
    } else {
        e.vcall(map, 4, &args![u32::from(key)]).u32()
    };
    loop {
        bucket = bucket.wrapping_add(1);
        if bucket >= word(e, map.wrapping_add(4)) {
            return 0;
        }
        let head = word(
            e,
            word(e, map.wrapping_add(8)).wrapping_add(bucket.wrapping_mul(4)),
        );
        if head != 0 {
            return head;
        }
    }
}

/// The pair map's keys in the order the exe's walk visits them, each with its vertex's point: the
/// candidates of every triangle's search. Read item by item, each item's point after its key, as the
/// exe's first walk reads them.
fn pairs_in_walk_order(e: &mut Engine, mesh: Ptr<NavMesh>) -> Vec<(u16, [f32; 3])> {
    let map = mesh.addr().wrapping_add(EDGE_PAIRS);
    let mut pairs = Vec::new();
    let mut item = first_item(e, map);
    while item != 0 {
        let key = e.mem.u16(item.wrapping_add(4));
        let at = vertex_point(e, mesh, key);
        pairs.push((key, read_point(e, at)));
        item = next_pair(e, map, item, key);
    }
    pairs
}

/// The squared length in x and y of `(dx, dy)` (`00595c80`), at the thread's precision: each step a
/// float on the main thread, a double rounded once to a float elsewhere.
#[inline(always)]
fn squared_xy(main: bool, dx: f32, dy: f32) -> f32 {
    if main {
        dx * dx + dy * dy
    } else {
        let (x, y) = (f64::from(dx), f64::from(dy));
        to_float(x * x + y * y)
    }
}

/// The condition `FCOMP` leaves for `TEST AH, mask`: C0 (`st0 < operand`), C2 (unordered) and C3
/// (equal) as `0x01`, `0x04` and `0x40`.
fn compare_flags(st0: f64, operand: f64) -> u8 {
    if st0.is_nan() || operand.is_nan() {
        0x45
    } else if st0 < operand {
        0x01
    } else if st0 == operand {
        0x40
    } else {
        0
    }
}

/// `TEST AH, mask` then `JP`: whether the masked flags have even parity.
fn parity_even(flags: u8, mask: u8) -> bool {
    (flags & mask).count_ones() % 2 == 0
}

// ---------------------------------------------------------------------------
// Passes 1 and 2: the temporary vertex map
// ---------------------------------------------------------------------------

/// `0069b4d0`: the value of the vertex map's item for the key (mesh, vertex), stored at `out`; false
/// when there is none. The hash and the compare are the map's vtable slots 4 and 8.
fn vertex_map_find(e: &mut Engine, map: Ptr, mesh: u32, vertex: u32, out: Ptr) -> bool {
    let bucket = e.vcall(map.addr(), 4, &args![mesh, vertex]).u32();
    let mut item = word(
        e,
        word(e, map.addr() + 8).wrapping_add(bucket.wrapping_mul(4)),
    );
    while item != 0 {
        let (low, high) = (word(e, item + 4), word(e, item + 8));
        if e.vcall(map.addr(), 8, &args![mesh, vertex, low, high])
            .bool()
        {
            let value = word(e, item + 0xc);
            e.mem.set_u32(out.addr(), value);
            return true;
        }
        item = word(e, item);
    }
    false
}

/// `0069b3d0`: sets the vertex map's value for the key, adding an item (slots `0x14`, then `0xc`) at
/// the head of its bucket when there is none.
fn vertex_map_set(e: &mut Engine, map: Ptr, mesh: u32, vertex: u32, value: u32) {
    let bucket = e.vcall(map.addr(), 4, &args![mesh, vertex]).u32();
    let slot = word(e, map.addr() + 8).wrapping_add(bucket.wrapping_mul(4));
    let mut item = word(e, slot);
    while item != 0 {
        let (low, high) = (word(e, item + 4), word(e, item + 8));
        if e.vcall(map.addr(), 8, &args![mesh, vertex, low, high])
            .bool()
        {
            e.mem.set_u32(item + 0xc, value);
            return;
        }
        item = word(e, item);
    }
    let item = e.vcall(map.addr(), 0x14, &[]).u32();
    e.vcall(map.addr(), 0xc, &args![item, mesh, vertex, value]);
    let head = word(e, slot);
    e.mem.set_u32(item, head);
    e.mem.set_u32(slot, item);
    let count = word(e, map.addr() + 0xc);
    e.mem.set_u32(map.addr() + 0xc, count.wrapping_add(1));
}

/// `0069b560`: the vertex map's item at `*position` (its key copied to `key`, its value to `value`), and
/// `*position` moved to the next item (the next in its bucket, else the first of a later bucket: the
/// key's bucket from the map's hash, slot 4).
fn vertex_map_next(e: &mut Engine, map: Ptr, position: Ptr, key: Ptr, value: Ptr) {
    let item = word(e, position.addr());
    typed_call!(
        pathing::FAMILY,
        0x0069_a690,
        pov::triangle_ref_copy(e, key.cast::<MeshEdge>(), Ptr::new(item.wrapping_add(4)))
    );
    let held = word(e, item.wrapping_add(0xc));
    e.mem.set_u32(value.addr(), held);
    let next = word(e, item);
    if next != 0 {
        e.mem.set_u32(position.addr(), next);
        return;
    }
    let (low, high) = (word(e, key.addr()), word(e, key.addr() + 4));
    let start = e
        .vcall(map.addr(), 4, &args![low, high])
        .u32()
        .wrapping_add(1);
    let buckets = word(e, map.addr() + 4);
    let table = word(e, map.addr() + 8);
    for bucket in start..buckets {
        let head = word(e, table.wrapping_add(bucket.wrapping_mul(4)));
        if head != 0 {
            e.mem.set_u32(position.addr(), head);
            return;
        }
    }
    e.mem.set_u32(position.addr(), 0);
}

/// `0069b310`: appends the 12-byte edge handle at `handle` to the array at `array`.
fn append_edge(e: &mut Engine, array: Ptr, handle: Ptr) {
    let index = e.call(ARRAY_NEXT_SLOT, &args![array]).u32();
    let element = word(e, array.addr() + 4).wrapping_add(index.wrapping_mul(12));
    e.call(EDGE_HANDLES_NEW, &args![array, element, 1u32]);
    typed_call!(
        pathing::FAMILY,
        0x006b_c870,
        pov::mesh_edge_copy(e, Ptr::new(element), handle.cast::<MeshEdge>())
    );
}

/// Adds the edge handle to the record of `vertex` (whose 6-byte handle is at `key`), making the record
/// (`operator new`, `0069a730`'s two arrays, the key copied in, the map set) when the vertex has none.
fn add_vertex_edge(
    e: &mut Engine,
    map: Ptr,
    mesh: Ptr<NavMesh>,
    vertex: u16,
    key: Ptr,
    edge: Ptr,
    portal: bool,
) {
    e.with_stack(4, |e, found| {
        let mut record = 0u32;
        if vertex_map_find(e, map, mesh.addr(), u32::from(vertex), found) {
            record = word(e, found.addr());
        } else {
            let block = e.call(OPERATOR_NEW, &args![RECORD_SIZE]).u32();
            if block != 0 {
                for offset in [RECORD_EDGES, RECORD_PORTAL_EDGES] {
                    typed_call!(
                        pathing::FAMILY,
                        0x0069_b2c0,
                        search::edge_array_new(e, Ptr::new(block.wrapping_add(offset)))
                    );
                }
                record = block;
            }
            // The exe copies the key into the record with no null test (00699cea).
            typed_call!(
                pathing::FAMILY,
                0x0069_a690,
                pov::triangle_ref_copy(e, Ptr::new(record), key.cast::<MeshEdge>())
            );
            vertex_map_set(e, map, mesh.addr(), u32::from(vertex), record);
        }
        let offset = if portal {
            RECORD_PORTAL_EDGES
        } else {
            RECORD_EDGES
        };
        append_edge(e, Ptr::new(record.wrapping_add(offset)), edge);
    });
}

/// Pass 1: every boundary edge of every triangle into the records of its two vertices.
fn gather_boundary_edges(e: &mut Engine, this: Ptr<NavMesh>, map: Ptr, triangles: u32) {
    for index in 0..triangles {
        let t = typed_call!(
            pathing::FAMILY,
            0x0055_82f0,
            navmesh::triangle(e, this, low_16(index))
        );
        for edge in 0..3u32 {
            let link = typed_call!(
                pathing::FAMILY,
                0x0068_f2c0,
                navmesh::link(e, t, edge as i32)
            );
            let portal = if link == NONE {
                false
            } else if typed_call!(pathing::FAMILY, 0x0068_f1d0, navmesh::edge_flag(e, t, edge)) {
                true
            } else {
                continue;
            };
            e.with_stack(24, |e, handles| {
                let edge_handle = handles;
                let vertex_handle = handles.byte_add(12);
                // `0069a6c0`: the mesh, the triangle and the edge.
                e.mem.set_u32(edge_handle.addr(), this.addr());
                e.mem.set_u16(edge_handle.addr() + 4, low_16(index));
                e.mem.set_u32(edge_handle.addr() + 8, edge);
                for corner in [edge, (edge + 1) % 3] {
                    let vertex =
                        typed_call!(pathing::FAMILY, 0x0055_82d0, navmesh::vertex(e, t, corner));
                    // `0069a660`: the mesh and the vertex.
                    e.mem.set_u32(vertex_handle.addr(), this.addr());
                    e.mem.set_u16(vertex_handle.addr() + 4, vertex);
                    add_vertex_edge(e, map, this, vertex, vertex_handle, edge_handle, portal);
                }
            });
        }
    }
}

/// `0069a700`: the low 14 bits of the triangle (+4) and the low two of the edge (+8) of an edge
/// handle, packed into one word at `out`.
fn pack_edge(e: &mut Engine, out: Ptr, handle: Ptr) {
    let low = e.mem.u16(handle.addr() + 4) & 0x3fff;
    let high = (e.mem.u16(handle.addr() + 8) & 3) << 14;
    e.mem.set_u16(out.addr(), low | high);
}

/// Pass 2: for each vertex record with exactly two edges (at least one of them ordinary), the corner
/// test, and the packed pair into the mesh's pair map when it passes or one edge is a portal.
fn pair_corners(e: &mut Engine, this: Ptr<NavMesh>, map: Ptr) {
    let edge_pairs = this.addr().wrapping_add(EDGE_PAIRS);
    let mut position = first_item(e, map.addr());
    while position != 0 {
        e.with_stack(0x90, |e, scratch| {
            let position_slot = scratch;
            let key = scratch.byte_add(4);
            let record_slot = scratch.byte_add(12);
            let first = scratch.byte_add(16);
            let second = scratch.byte_add(28);
            let first_vertices = scratch.byte_add(40);
            let second_vertices = scratch.byte_add(48);
            let first_vector = scratch.byte_add(56);
            let second_vector = scratch.byte_add(68);
            let cross = scratch.byte_add(80);
            let pair = scratch.byte_add(92);
            e.mem.set_u32(position_slot.addr(), position);
            vertex_map_next(e, map, position_slot, key, record_slot);
            position = word(e, position_slot.addr());
            let record = word(e, record_slot.addr());
            let edges = Ptr::<SimpleArray>::new(record.wrapping_add(RECORD_EDGES));
            let portal_edges = Ptr::<SimpleArray>::new(record.wrapping_add(RECORD_PORTAL_EDGES));
            let ordinary = typed_call!(leaf::FAMILY, 0x0044_ddc0, leaf::word_at_8(e, edges));
            let portals = typed_call!(leaf::FAMILY, 0x0044_ddc0, leaf::word_at_8(e, portal_edges));
            if ordinary.wrapping_add(portals) != 2 || ordinary == 0 {
                return;
            }
            let first_element = entry_12(e, edges, 0);
            typed_call!(
                pathing::FAMILY,
                0x006b_c870,
                pov::mesh_edge_copy(e, first.cast(), Ptr::new(first_element))
            );
            let second_element = if ordinary == 2 {
                entry_12(e, edges, 1)
            } else {
                entry_12(e, portal_edges, 0)
            };
            typed_call!(
                pathing::FAMILY,
                0x006b_c870,
                pov::mesh_edge_copy(e, second.cast(), Ptr::new(second_element))
            );
            for (handle, out) in [(first, first_vertices), (second, second_vertices)] {
                let triangle = e.mem.u16(handle.addr() + 4);
                let edge = word(e, handle.addr() + 8) as i32;
                typed_call!(
                    pathing::FAMILY,
                    0x0068_f040,
                    navmesh::edge_vertices(e, this, out, triangle, edge)
                );
            }
            let first_start = word(e, first_vertices.addr());
            let first_end = word(e, first_vertices.addr() + 4);
            let second_start = word(e, second_vertices.addr());
            let second_end = word(e, second_vertices.addr() + 4);
            let (common, outer_first, outer_second, packed_first, packed_second) =
                if first_start == second_end {
                    (first_start, first_end, second_start, first, second)
                } else if first_end == second_start {
                    (first_end, second_end, first_start, second, first)
                } else {
                    return;
                };
            let corner = read_point(e, common);
            let a = difference(read_point(e, outer_first), corner);
            let b = difference(read_point(e, outer_second), corner);
            write_point(e, first_vector.addr(), a);
            write_point(e, second_vector.addr(), b);
            e.mem.set_u32(first_vector.addr() + 8, 0f32.to_bits());
            e.mem.set_u32(second_vector.addr() + 8, 0f32.to_bits());
            typed_call!(
                dispatch::FAMILY,
                0x004a_0c10,
                dispatch::math::normalize(e, first_vector.cast::<NiPoint3>())
            );
            typed_call!(
                dispatch::FAMILY,
                0x004a_0c10,
                dispatch::math::normalize(e, second_vector.cast::<NiPoint3>())
            );
            e.call(POINT3_CROSS, &args![first_vector, cross, second_vector]);
            let dot = typed_call!(
                leaf::FAMILY,
                0x004b_6190,
                leaf::point3_dot(e, first_vector, second_vector)
            );
            let cross_z = f32::from_bits(word(e, cross.addr() + 8));
            let accept = if compare_flags(f64::from(dot), double_global(e, ZERO)) & 0x41 == 0 {
                // 0069a2f2-0069a35e may swap the two local vectors here; neither is read after.
                !parity_even(
                    compare_flags(f64::from(cross_z), double_global(e, POSITIVE_LIMIT)),
                    0x05,
                )
            } else {
                !parity_even(
                    compare_flags(f64::from(cross_z), double_global(e, NEGATIVE_LIMIT)),
                    0x05,
                )
            };
            if accept || ordinary == 1 {
                pack_edge(e, pair, packed_first);
                pack_edge(e, pair.byte_add(2), packed_second);
                let vertex = u32::from(e.mem.u16(key.addr() + 4));
                let packed = word(e, pair.addr());
                e.call(PAIR_MAP_SET, &args![edge_pairs, vertex, packed]);
            }
        });
    }
}

/// Pass 2's end: every record deleted (`0069a7a0` with 1: its destructor body, then `operator
/// delete`).
fn delete_records(e: &mut Engine, map: Ptr) {
    let mut position = first_item(e, map.addr());
    while position != 0 {
        e.with_stack(16, |e, scratch| {
            let position_slot = scratch;
            let key = scratch.byte_add(4);
            let record_slot = scratch.byte_add(12);
            e.mem.set_u32(position_slot.addr(), position);
            vertex_map_next(e, map, position_slot, key, record_slot);
            position = word(e, position_slot.addr());
            let record = word(e, record_slot.addr());
            if record != 0 {
                e.call(RECORD_DESTRUCT, &args![record]);
                e.call(OPERATOR_DELETE, &args![record]);
            }
        });
    }
}

// ---------------------------------------------------------------------------
// Pass 3: each triangle's preferred vertex
// ---------------------------------------------------------------------------

/// The circumcenter of triangle `index` in x and y with the highest z of its corners (`00697290`, the
/// translation's doubles): each corner through the triangle (`005582f0`) and its vertex (`005582d0`).
fn circumcenter(e: &mut Engine, this: Ptr<NavMesh>, index: u16) -> [f32; 3] {
    let mut corners = [[0f32; 3]; 3];
    for (k, corner) in (0u32..).zip(corners.iter_mut()) {
        let t = typed_call!(
            pathing::FAMILY,
            0x0055_82f0,
            navmesh::triangle(e, this, index)
        );
        let vertex = typed_call!(pathing::FAMILY, 0x0055_82d0, navmesh::vertex(e, t, k));
        *corner = read_point(e, vertex_point(e, this, vertex));
    }
    let [v0, v1, v2] = corners.map(|c| c.map(f64::from));
    let (ax, ay) = (to_float(v1[0] - v0[0]), to_float(v1[1] - v0[1]));
    let (bx, by) = (to_float(v2[0] - v0[0]), to_float(v2[1] - v0[1]));
    let [ax, ay, bx, by] = [ax, ay, bx, by].map(f64::from);
    let a_squared = f64::from(to_float(ax * ax + ay * ay));
    let b_squared = f64::from(to_float(bx * bx + by * by));
    let determinant = f64::from(to_float(ax * by - bx * ay));
    let reciprocal_half = f64::from(to_float(double_global(e, DOUBLE_HALF) / determinant));
    let z = if v1[2] <= v0[2] && v2[2] <= v0[2] {
        v0[2]
    } else if v2[2] <= v1[2] {
        v1[2]
    } else {
        v2[2]
    };
    let y = to_float((ax * b_squared - bx * a_squared) * reciprocal_half + v0[1]);
    let x = to_float(v0[0] - (ay * b_squared - by * a_squared) * reciprocal_half);
    [x, y, to_float(z)]
}

/// The double 0.5 the circumcenter's reciprocal divides.
const DOUBLE_HALF: u32 = 0x0101_1588;

/// Pass 3: for each triangle, the pair vertex nearest its circumcenter in x and y whose height is
/// within 180, appended to the preferred-vertex array (`0xffff` when none is).
fn choose_preferred(e: &mut Engine, this: Ptr<NavMesh>, triangles: u32) {
    let preferred = Ptr::<SimpleArray>::new(this.addr().wrapping_add(PREFERRED));
    let reach = float_global(e, REACH);
    let mut pairs: Option<Vec<(u16, [f32; 3])>> = None;
    for index in 0..triangles {
        let index = low_16(index);
        // The translation's 80 bytes of locals for one triangle (the exe keeps them in its one
        // 0x1d0-byte frame): the planar difference at +24 and the chosen vertex at +52 are the two
        // locals the calls see, placed as the translation places them.
        e.with_stack(80, |e, scratch| {
            let local_difference = scratch.byte_add(24);
            let local_key = scratch.byte_add(52);
            let center = circumcenter(e, this, index);
            let t = typed_call!(
                pathing::FAMILY,
                0x0055_82f0,
                navmesh::triangle(e, this, index)
            );
            let first = typed_call!(pathing::FAMILY, 0x0055_82d0, navmesh::vertex(e, t, 0));
            let planar = difference(read_point(e, vertex_point(e, this, first)), center);
            write_point(e, local_difference.addr(), planar);
            let length = typed_call!(
                pathing::FAMILY,
                0x0058_9850,
                point::length_2d(e, local_difference.cast::<Point2>())
            );
            let widened = length + reach;
            let squared = widened * widened;
            let mut nearest = squared * squared;
            if pairs.is_none() {
                pairs = Some(pairs_in_walk_order(e, this));
            }
            // Read once a triangle: the word holds through the loop (`fabs` restores it).
            let main = float_precision();
            let height_limit = double_global(e, HEIGHT_LIMIT);
            let mut best = NONE;
            // Both tests are the exe's `FCOM`s: a NaN is never nearer and never low enough.
            for &(key, p) in pairs.as_deref().unwrap_or_default() {
                let distance = squared_xy(main, p[0] - center[0], p[1] - center[1]);
                if distance < nearest {
                    let height = typed_call!(
                        pathing::FAMILY,
                        0x0040_8840,
                        point::abs(e, p[2] - center[2])
                    );
                    if f64::from(height) < height_limit {
                        nearest = distance;
                        best = key;
                    }
                }
            }
            e.mem.set_u16(local_key.addr(), best);
            typed_call!(
                pathing::FAMILY,
                0x0069_1820,
                search::push_index(e, preferred, local_key)
            );
        });
    }
}

// ---------------------------------------------------------------------------
// The function
// ---------------------------------------------------------------------------

// Typed from 00699b60
/// Rebuilds the mesh's boundary-edge pair map (`+0x88`) and its preferred-vertex array (`+0x98`)
/// (`thiscall(mesh)`): both emptied, then the four passes of the [module](self).
pub fn rebuild_edge_pairs(e: &mut Engine, this: Ptr<NavMesh>) {
    let edge_pairs = this.addr().wrapping_add(EDGE_PAIRS);
    let preferred = this.addr().wrapping_add(PREFERRED);
    e.call(MAP_REMOVE_ALL, &args![edge_pairs]);
    typed_call!(
        dispatch::FAMILY,
        0x0084_54f0,
        dispatch::refs::array_clear(e, Ptr::new(preferred), 1)
    );
    e.with_stack(0x20, |e, map| {
        // `0069ad40`: the base with 0x25 buckets, then the map's vtable.
        e.call(VERTEX_MAP_BASE_NEW, &args![map, VERTEX_MAP_BUCKETS]);
        e.mem.set_u32(map.addr(), VERTEX_MAP_VTABLE);
        let triangles = typed_call!(
            pathing::FAMILY,
            0x0055_8200,
            navmesh::triangle_count(e, this)
        );
        gather_boundary_edges(e, this, map, triangles);
        pair_corners(e, this, map);
        delete_records(e, map);
        choose_preferred(e, this, triangles);
        // `0069b6d0`: the map emptied, then its base (`0069b730`) emptied and its buckets freed.
        e.mem.set_u32(map.addr(), VERTEX_MAP_VTABLE);
        e.call(MAP_REMOVE_ALL, &args![map]);
        e.mem.set_u32(map.addr(), VERTEX_MAP_BASE_VTABLE);
        e.call(MAP_REMOVE_ALL, &args![map]);
        let buckets = word(e, map.addr() + 8);
        e.call(BUCKETS_FREE, &args![buckets]);
    });
}
