//! The navmesh's record helpers: the child-cell base's parent-cell getter, the info's mesh
//! holder assignment, the vertex/triangle/edge-info appends, and the extra edge info removal
//! with its triangle repairs.

use super::layout::NavMesh;
use super::{navmesh, FAMILY};
use engine_core::ptr::Ptr;
use engine_core::typed::leaf;
use engine_core::{args, typed_call, Engine};

/// Callees.
const GET_PARENT_CELL: u32 = 0x0059_bb30;
const ARRAY_ADD_12: u32 = 0x0069_1ab0;
const ARRAY_ADD_16: u32 = 0x0069_1b60;
const EXTRA_INFO_ADD: u32 = 0x0069_1c10;
const ARRAY_REMOVE_12: u32 = 0x0069_1c90;
const HAS_EXTRA_INFO: u32 = 0x0069_0e10;
const CLEAR_EXTRA: u32 = 0x0069_0790;
const SET_EXTRA: u32 = 0x0069_0e30;

/// The `TESChildCell` base's offset inside the `NavMesh`.
const CHILD_CELL_BASE: u32 = 0x18;
/// The info's `pNavMesh` offset.
const INFO_NAV_MESH: u32 = 0x54;

// Typed from 0068ef50
/// `TESChildCell::GetSaveParentCell` (the `TESChildCell` base's vtable slot 0): `this` is the
/// base at `NavMesh + 0x18`; returns the navmesh's parent cell (`0059bb30` on the `NavMesh`).
pub fn get_save_parent_cell(e: &mut Engine, this: Ptr) -> u32 {
    let mesh = Ptr::<NavMesh>::new(this.addr().wrapping_sub(CHILD_CELL_BASE));
    e.call(GET_PARENT_CELL, &args![mesh]).u32()
}

// Typed from 00690860
/// `NavMeshInfo` method (`thiscall(info, result)`): assigns the info's `pNavMesh` (+0x54) to the
/// `NiPointer<NavMesh>` at `result` ([`navmesh::hold`]); returns whether `pNavMesh` is non-null.
pub fn info_get_mesh(e: &mut Engine, this: Ptr, result: Ptr) -> bool {
    let mesh = e.mem.u32(this.addr() + INFO_NAV_MESH);
    typed_call!(FAMILY, 0x0069_0890, navmesh::hold(e, result, mesh));
    mesh != 0
}

// Typed from 00690fe0
/// Appends a copy of the 12-byte vertex at `vertex` to the mesh's vertex array (`00691ab0`);
/// returns the new vertex's index.
pub fn append_vertex(e: &mut Engine, this: Ptr<NavMesh>, vertex: Ptr) -> u32 {
    e.call(ARRAY_ADD_12, &args![this.at(NavMesh::vertices), vertex])
        .u32()
}

// Typed from 00691000
/// Appends a copy of the 16-byte triangle at `triangle` to the mesh's triangle array
/// (`00691b60`); returns the new triangle's index.
pub fn append_triangle(e: &mut Engine, this: Ptr<NavMesh>, triangle: Ptr) -> u32 {
    e.call(ARRAY_ADD_16, &args![this.at(NavMesh::triangles), triangle])
        .u32()
}

// Typed from 00691020
/// Appends the `EdgeExtraInfo` at `info` to the mesh's `ExtraEdgeInfo` array (`00691c10`);
/// returns the new index.
pub fn append_edge_info(e: &mut Engine, this: Ptr<NavMesh>, info: Ptr) -> u32 {
    e.call(EXTRA_INFO_ADD, &args![this.at(NavMesh::edge_infos), info])
        .u32()
}

// Typed from 00690e90
/// `NavMesh::DeleteExtraEdgeInfo` (`thiscall(mesh, index)`): removes `ExtraEdgeInfo[index]`
/// (`00691c90`, the array moves its last element into the gap) and repairs the triangles: an edge
/// that used `index` loses its extra info flag (`00690790`), and an edge that used the moved last
/// element now points at `index` (`00690e30`).
pub fn delete_extra_edge_info(e: &mut Engine, this: Ptr<NavMesh>, index: u16) {
    let infos = this.at(NavMesh::edge_infos);
    e.call(ARRAY_REMOVE_12, &args![infos, index, 1u32]);
    let triangles = this.at(NavMesh::triangles);
    let count: u32 = typed_call!(leaf::FAMILY, 0x0044_ddc0, leaf::word_at_8(e, triangles));
    let mut t = 0u32;
    while t < count {
        let tri: Ptr<_> = typed_call!(FAMILY, 0x0055_8dd0, navmesh::triangle_at(e, triangles, t));
        if e.call(HAS_EXTRA_INFO, &args![tri]).bool() {
            let mut edge = 0u32;
            while edge < 3 {
                let tri: Ptr<_> =
                    typed_call!(FAMILY, 0x0055_8dd0, navmesh::triangle_at(e, triangles, t));
                let has: bool = typed_call!(FAMILY, 0x0068_f1d0, navmesh::edge_flag(e, tri, edge));
                if has {
                    let tri: Ptr<_> =
                        typed_call!(FAMILY, 0x0055_8dd0, navmesh::triangle_at(e, triangles, t));
                    let slot: u16 =
                        typed_call!(FAMILY, 0x0068_f2c0, navmesh::link(e, tri, edge as i32));
                    if slot == index {
                        let tri: Ptr<_> =
                            typed_call!(FAMILY, 0x0055_8dd0, navmesh::triangle_at(e, triangles, t));
                        e.call(CLEAR_EXTRA, &args![tri, edge]);
                    } else {
                        let _: Ptr<_> =
                            typed_call!(FAMILY, 0x0055_8dd0, navmesh::triangle_at(e, triangles, t));
                        let size: u32 =
                            typed_call!(leaf::FAMILY, 0x0044_ddc0, leaf::word_at_8(e, infos));
                        if slot as u32 == size {
                            let tri: Ptr<_> = typed_call!(
                                FAMILY,
                                0x0055_8dd0,
                                navmesh::triangle_at(e, triangles, t)
                            );
                            e.call(SET_EXTRA, &args![tri, edge, index]);
                        }
                    }
                }
                edge += 1;
            }
        }
        t += 1;
    }
}
