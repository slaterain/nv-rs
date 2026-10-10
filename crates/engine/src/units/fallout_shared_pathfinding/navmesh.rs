//! `fallout shared/pathfinding/navmesh.cpp` (Xbox PDB source unit), subsystem `fallout shared/pathfinding`: its functions in
//! FalloutNV.exe 1.4.0.525, translated (docs/ENGINE_CRATE.md).
//!
//! A `NavMesh` is a form (`TESForm`, 0x18 bytes on PC) with two more bases,
//! `TESChildCell` (vtable at +0x18) and `NiRefObject` (vtable at +0x1C,
//! reference count at +0x20), followed by the mesh data: arrays of vertices
//! (12 bytes), triangles (16 bytes) and extra edge infos (12 bytes), the door
//! portals, closed doors and cover list, a point-of-view map, the search grid
//! and the obstacle bookkeeping.
//!
//! Triangles ([`NavMeshTriangle`]) hold three vertex indexes, three neighbour
//! triangle indexes (`0xFFFF` = no neighbour) and a flag word. Bit `n` of
//! the flags says that edge `n` has extra info: its neighbour slot then does
//! not hold a triangle index but an index into the mesh's `ExtraEdgeInfo`
//! array ([`EdgeExtraInfo`]: the portal's `NavMeshInfo` and triangle).
//!
//! Translated so far (address order): everything up to and including
//! `00691000`; the next session continues at `00691020`.
//!
//! The `NavMeshTriangle` and `NavMeshInfo` methods in this unit are unnamed
//! in the engine map; they are named here by what their bodies do. Methods
//! of `NavMeshInfo` (another unit's class, whose fields are read at their
//! offsets, see the `INFO_` constants) take the `NavMeshInfo` as `this`.
//!
//! Left out everywhere: the compiler's exception-unwinding frames.

#[allow(unused_imports)]
use crate::prelude::*;
use crate::types::{BSSimpleArray, NiTPointerMap};

// ---------------------------------------------------------------------------
// Layouts and constants
// ---------------------------------------------------------------------------

layout! {
    /// `NavMesh` (Xbox PDB), 0x108 bytes on PC (0x118 on the Xbox: the PC
    /// `TESForm` is 0x10 smaller, so the fields after it sit 0x10 lower).
    pub struct NavMesh: 0x108 {
        /// `pParentCell` (Xbox PDB): `TESObjectCELL*`.
        0x24 pParentCell: Ptr,
        /// `Vertices` (Xbox PDB): `BSSimpleArray<NavMeshVertex>`, 12-byte
        /// elements (three floats).
        0x28 Vertices: Inline<BSSimpleArray>,
        /// `Triangles` (Xbox PDB): `BSSimpleArray<NavMeshTriangle>`.
        0x38 Triangles: Inline<BSSimpleArray>,
        /// `ExtraEdgeInfo` (Xbox PDB): `BSSimpleArray<EdgeExtraInfo>`.
        0x48 ExtraEdgeInfo: Inline<BSSimpleArray>,
        /// `DoorPortals` (Xbox PDB): `BSSimpleArray<NavMeshTriangleDoorPortal>`.
        0x58 DoorPortals: Inline<BSSimpleArray>,
        /// `ClosedDoors` (Xbox PDB): `BSSimpleArray<NavMeshClosedDoorInfo>`.
        0x68 ClosedDoors: Inline<BSSimpleArray>,
        /// `CoverArray` (Xbox PDB): `BSSimpleArray<unsigned short>` of
        /// triangle indexes.
        0x78 CoverArray: Inline<BSSimpleArray>,
        /// `POVs` (Xbox PDB): `NiTMap<unsigned short, NavMeshPOVData>`.
        0x88 POVs: Inline<NiTPointerMap>,
        /// `ClosestPOVs` (Xbox PDB): `BSSimpleArray<unsigned short>`.
        0x98 ClosestPOVs: Inline<BSSimpleArray>,
        /// `Obstacles` (Xbox PDB): `BSSimpleArray<NiPointer<ObstacleUndoData>>`.
        0xD0 Obstacles: Inline<BSSimpleArray>,
        /// `pTriangleToObstacleMap` (Xbox PDB): `NiTMap*`, 0x10 bytes,
        /// created with 0x25 buckets.
        0xE0 pTriangleToObstacleMap: Ptr,
        /// `ObstaclePOVs` (Xbox PDB): `BSSimpleArray<unsigned short>`.
        0xE4 ObstaclePOVs: Inline<BSSimpleArray>,
        /// `StaticAvoidNodes` (Xbox PDB): `BSSimpleArray<NavMeshStaticAvoidNode>`.
        0xF4 StaticAvoidNodes: Inline<BSSimpleArray>,
        /// `pNavMeshInfo` (Xbox PDB): `NavMeshInfo*`.
        0x104 pNavMeshInfo: Ptr,
    }

    /// `NavMeshTriangle` (Xbox PDB), 0x10 bytes.
    pub struct NavMeshTriangle: 0x10 {
        /// `Vertices[0]` (Xbox PDB): first of three `u16` vertex indexes.
        0x00 Vertices: u16,
        /// `Triangles[0]` (Xbox PDB): first of three `u16` neighbour
        /// triangle indexes (or extra-info indexes, see the module docs).
        0x06 Triangles: u16,
        /// `TriangleFlags` (Xbox PDB).
        0x0C TriangleFlags: u32,
    }

    /// `EdgeExtraInfo` (Xbox PDB), 0x0C bytes.
    pub struct EdgeExtraInfo: 0x0C {
        /// `eType` (Xbox PDB).
        0x00 eType: u32,
        /// `Portal.pOtherMesh` (Xbox PDB): `NavMeshInfo*`.
        0x04 pOtherMesh: Ptr,
        /// `Portal.sTriangle` (Xbox PDB): triangle index in the other mesh.
        0x08 sTriangle: u16,
    }
}

/// `NavMesh` vtable, `TESChildCell` vtable (slot 0 is `0068ef50`) and
/// `NiRefObject` vtable, stored by the constructor and destructor.
const NAV_MESH_VTABLE: u32 = 0x0106_a0b4;
const CHILD_CELL_VTABLE: u32 = 0x0106_a0a8;
const REF_OBJECT_VTABLE: u32 = 0x0106_a09c;
/// Offsets of the `TESChildCell` and `NiRefObject` bases inside a `NavMesh`.
const CHILD_CELL_BASE: u32 = 0x18;
const REF_OBJECT_BASE: u32 = 0x1c;
/// `TESForm::cFormType` stored by the constructor.
const NAV_MESH_FORM_TYPE: u32 = 0x43;
/// Bucket count of the triangle-to-obstacle map and the POV map.
const OBSTACLE_MAP_BUCKETS: u32 = 0x25;
/// Size of a `NavMesh`, for the allocator's free call.
const NAV_MESH_SIZE: u32 = 0x108;
/// Offset of the `MeshGrid` (`NavMeshGrid`, 0x28 bytes) inside a `NavMesh`.
const MESH_GRID_OFFSET: u32 = 0xa8;

// `NavMeshInfo` (Xbox PDB; a class of `navmeshinfo.cpp`): offsets of the
// fields these functions read.
/// `ParentSpaceID` (`u32`).
const INFO_PARENT_SPACE_ID: u32 = 0x04;
/// `uiFlags`; bit 0x10 is tested by `0068f320`.
const INFO_FLAGS: u32 = 0x08;
/// `pParentSpace` (`TESForm*`).
const INFO_PARENT_SPACE: u32 = 0x1c;
/// `pNavMesh` (`NavMesh*`).
const INFO_NAV_MESH: u32 = 0x54;

/// Triangle flag bits 0 to 2: edge extra info.
const EDGE_INFO_FLAGS: u32 = 0x7;
/// Flag bits `00690770` tests (the cover flags).
const COVER_FLAGS: u32 = 0x0fbe_0000;
/// Flag bit `00690e70` tests.
const TRIANGLE_FLAG_0X400: u32 = 0x400;
/// "No neighbour" in a triangle's neighbour slot.
const NO_NEIGHBOUR: u16 = 0xffff;

// Values the mesh check reads from the exe's data.
/// Largest vertex coordinate magnitude (`double`, 1.00000000376832e14).
const MAX_COORDINATE: u32 = 0x0106_ab98;
/// The `double` 0.0 the triangle normal's z is compared with.
const ZERO: u32 = 0x0101_2060;
/// The `double` 4096.0 (width of an exterior cell) added to a cell origin.
const CELL_WIDTH: u32 = 0x0101_7a10;

/// Type descriptors `00690800` casts between (`.?AVTESForm@@`,
/// `.?AVTESWorldSpace@@`).
const TYPE_TES_FORM: u32 = 0x0118_3028;
const TYPE_TES_WORLD_SPACE: u32 = 0x0118_3fd0;
/// The `TES` instance pointer (`TES::GetNavMeshInfoMap` is called on it).
const TES_INSTANCE: u32 = 0x011d_ea10;

// Functions of other units this one calls.
/// `operator new`.
const OPERATOR_NEW: u32 = 0x0040_1000;
/// Allocator free (cdecl: block, size).
const FREE_BLOCK: u32 = 0x00aa_1460;
/// `_finite` (cdecl, one `double`).
const FINITE: u32 = 0x00ec_7595;
/// `__RTDynamicCast`.
const RT_DYNAMIC_CAST: u32 = 0x00ec_43fb;
/// `TESForm::TESForm` and `TESForm::~TESForm`.
const TES_FORM_CONSTRUCTOR: u32 = 0x0048_3370;
const TES_FORM_DESTRUCTOR: u32 = 0x0048_3630;
/// Constructor of the `TESChildCell` base; constructor and destructor of the
/// `NiRefObject` base.
const CHILD_CELL_CONSTRUCTOR: u32 = 0x0053_3240;
const REF_OBJECT_CONSTRUCTOR: u32 = 0x0049_68b0;
const REF_OBJECT_DESTRUCTOR: u32 = 0x0049_6910;
/// Stores its argument as the form type byte (`TESForm::cFormType`, +0x04).
const SET_FORM_TYPE: u32 = 0x004f_15a0;
/// `BSSimpleArray` size getter (`iSize`, +8) and clear (argument 1: free the
/// buffer).
const ARRAY_SIZE: u32 = 0x0044_ddc0;
const ARRAY_CLEAR: u32 = 0x0084_54f0;
// Constructors and destructors of the mesh's members (all in this unit,
// not yet translated).
const ARRAY_CONSTRUCTOR_VERTICES: u32 = 0x0069_1a60;
const ARRAY_DESTRUCTOR_VERTICES: u32 = 0x0069_1a90;
const ARRAY_CONSTRUCTOR_TRIANGLES: u32 = 0x0069_1b10;
const ARRAY_DESTRUCTOR_TRIANGLES: u32 = 0x0069_1b40;
const ARRAY_CONSTRUCTOR_EXTRA_INFO: u32 = 0x0069_1bc0;
const ARRAY_DESTRUCTOR_EXTRA_INFO: u32 = 0x0069_1bf0;
const ARRAY_CONSTRUCTOR_DOOR_PORTALS: u32 = 0x0069_1d40;
const ARRAY_DESTRUCTOR_DOOR_PORTALS: u32 = 0x0069_1d70;
const ARRAY_CONSTRUCTOR_CLOSED_DOORS: u32 = 0x0069_1d90;
const ARRAY_DESTRUCTOR_CLOSED_DOORS: u32 = 0x0069_1dc0;
/// Constructor and destructor shared by the `u16` arrays (cover, closest
/// POVs, obstacle POVs).
const ARRAY_CONSTRUCTOR_U16: u32 = 0x0069_17d0;
const ARRAY_DESTRUCTOR_U16: u32 = 0x0069_1800;
/// Constructor (argument: bucket count) and destructor of the POV map.
const MAP_CONSTRUCTOR: u32 = 0x0069_1720;
const MAP_DESTRUCTOR: u32 = 0x0069_1ea0;
/// Constructor and clear (`0069d2d0`) of the search grid.
const GRID_CONSTRUCTOR: u32 = 0x0069_d280;
const GRID_CLEAR: u32 = 0x0069_d2d0;
/// Constructor, clear (argument 1: free the buffer) and destructor of the
/// obstacle array.
const OBSTACLES_CONSTRUCTOR: u32 = 0x0069_1f60;
const OBSTACLES_CLEAR: u32 = 0x006c_6200;
const OBSTACLES_DESTRUCTOR: u32 = 0x0069_1f90;
/// Constructor and destructor of the static avoid node array.
const AVOID_NODES_CONSTRUCTOR: u32 = 0x0069_1fb0;
const AVOID_NODES_DESTRUCTOR: u32 = 0x0069_1fe0;
/// Constructor of the triangle-to-obstacle map object (argument: buckets).
const OBSTACLE_MAP_CONSTRUCTOR: u32 = 0x0069_16f0;
/// Stores its argument in `NavMeshInfo::pNavMesh` (+0x54).
const INFO_SET_NAV_MESH: u32 = 0x005a_8040;
/// `BSSimpleArray` element address for 12-byte elements (`buffer + index *
/// 12`): used for the vertex and the extra-info arrays.
const ELEMENT_12: u32 = 0x006a_1440;
/// `BSSimpleArray` element address for 16-byte elements (the triangles).
const TRIANGLE_ELEMENT: u32 = 0x0055_8dd0;
/// `NavMesh::GetTriangle(index)`: `Triangles[index]` (through `00558dd0`).
const NAV_MESH_GET_TRIANGLE: u32 = 0x0055_82f0;
/// `NavMeshTriangle::GetVertex(i)`: the `u16` vertex index `i`.
const TRIANGLE_VERTEX: u32 = 0x0055_82d0;
/// `NavMesh::GetTriangleCount` (`Triangles.iSize`, through `0044ddc0`).
const NAV_MESH_TRIANGLE_COUNT: u32 = 0x0055_8200;
/// `TESObjectCELL` virtual slot 0x90: `GetFormDetailedString(BSString*)`.
const GET_FORM_DETAILED_STRING_SLOT: u32 = 0x90;
/// Returns a `NavMesh`'s `pParentCell` (+0x24).
const GET_PARENT_CELL: u32 = 0x0059_bb30;
/// `TESObjectCELL::GetWorldSpace`, the cell's interior test, and
/// `GetDataX` / `GetDataY` (exterior cell coordinates).
const CELL_GET_WORLD_SPACE: u32 = 0x0054_ddd0;
const CELL_IS_INTERIOR: u32 = 0x0042_5fd0;
const CELL_GET_X: u32 = 0x0054_4c30;
const CELL_GET_Y: u32 = 0x0054_4c60;
/// `TESWorldSpace::GetCellFromCellCoord(x, y)`.
const WORLD_SPACE_GET_CELL: u32 = 0x0058_75a0;
/// Cell's navmesh array getter (`pNavMeshes`, +0x64 on PC).
const CELL_GET_NAV_MESHES: u32 = 0x0070_ec90;
/// `NavMeshArray::GetNavMeshByIndex(out NiPointer, index)` (returns `out`).
const NAV_MESH_ARRAY_GET: u32 = 0x0046_4f60;
/// Size of the cell's navmesh array.
const NAV_MESH_ARRAY_SIZE: u32 = 0x0062_0b80;
// `NiPointer` helpers: constructor from a raw pointer (`this`, pointer),
// the same as a cdecl function (`out`, pointer; returns `out`), default
// constructor, assignment (`this`, other), destructor, and the read of the
// pointer held (the word at +0, which also reads a `BSString`'s characters
// or the first word of any object).
const POINTER_CONSTRUCT_FROM: u32 = 0x0046_4ff0;
const POINTER_CONSTRUCT_FROM_CDECL: u32 = 0x0046_4fc0;
const POINTER_CONSTRUCT: u32 = 0x0042_fb00;
const POINTER_ASSIGN: u32 = 0x0042_f4c0;
const POINTER_DESTRUCT: u32 = 0x0042_fa40;
const POINTER_GET: u32 = 0x0055_9450;
/// The held pointer of an `NiPointer<NavMesh>` (calls `POINTER_GET`).
const POINTER_DEREFERENCE: u32 = 0x0045_8b50;
/// Debug-print stub the mesh check reports through (cdecl: format string,
/// then its arguments; returns 0 and prints nothing in this build).
const DEBUG_PRINT: u32 = 0x005b_5e40;
/// Debug stub called for every adjacent-cell navmesh (cdecl, five words:
/// the mesh, the neighbour mesh, a direction flag and two coordinates); it
/// always returns true.
const NEIGHBOUR_CHECK: u32 = 0x005d_4a40;
/// `BSString` constructor and destructor (the cell text).
const STRING_CONSTRUCT: u32 = 0x0040_37b0;
const STRING_DESTRUCT: u32 = 0x0040_37d0;
/// Returns `TESForm::iFormID` (+0x0C).
const GET_FORM_ID: u32 = 0x0084_e3a0;
/// `fabs` for a `float` argument (result in ST0).
const FLOAT_ABS: u32 = 0x0040_8840;
/// Triangle normal into an out `NiPoint3` (`this`, out, triangle index).
const TRIANGLE_NORMAL: u32 = 0x0069_71e0;
/// Flips a triangle (`this`, triangle index).
const FLIP_TRIANGLE: u32 = 0x0069_5660;
/// `BSSimpleArray<unsigned short>` element address (`buffer + index * 2`).
const U16_ELEMENT: u32 = 0x005e_eda0;
/// Removes `count` elements from `index` of a 12-byte element array (the
/// array moves its last elements into the gap).
const ARRAY_REMOVE_12: u32 = 0x0069_1c90;
/// Appends a copy of a 12-byte element to its array, returns the index.
const ARRAY_ADD_12: u32 = 0x0069_1ab0;
/// Appends a copy of a 16-byte element to its array, returns the index.
const ARRAY_ADD_16: u32 = 0x0069_1b60;
/// `DoorPortals` element getter.
const DOOR_PORTAL_ELEMENT: u32 = 0x0084_4160;
/// `TES::GetNavMeshInfoMap` and `NavMeshInfoMap::DeleteNavMeshInfo(id)`.
const GET_NAV_MESH_INFO_MAP: u32 = 0x0045_af00;
const DELETE_NAV_MESH_INFO: u32 = 0x006b_6a00;
/// Looks a form up by its id (cdecl).
const LOOKUP_FORM: u32 = 0x0048_39c0;

// Format strings of the problems `CheckNavMesh` reports, in the order the
// function checks them. All start with `"PATHFINDING: Navmesh %08x Cell %s,
// "` and are printed with the navmesh's form id, the cell's text and the
// numbers listed.
/// `"has bad vertex X coordinate, Please regenerate"` and the Y, Z forms.
const FORMAT_BAD_X: u32 = 0x0106_ab40;
const FORMAT_BAD_Y: u32 = 0x0106_aae8;
const FORMAT_BAD_Z: u32 = 0x0106_aa90;
/// `"has a large number of triangles, %d, please check and optimize"`.
const FORMAT_MANY_TRIANGLES: u32 = 0x0106_aa28;
/// `"Triangle %d, edge %d has bad Triangle index, Clearing the connection"`.
const FORMAT_BAD_TRIANGLE_INDEX: u32 = 0x0106_a9c0;
/// `"Triangle %d, has a downfacing normal, flipping the triangle"`.
const FORMAT_DOWNFACING: u32 = 0x0106_a960;
/// `"Triangle %d, edge %d has bad extra info index, Clearing extra info flag"`.
const FORMAT_BAD_EXTRA_INFO_INDEX: u32 = 0x0106_a8f0;
/// `"Triangle %d, edge %d has bad Portal (Navmesh %08x does not have a
/// triangle index %d), Clearing the portal"`.
const FORMAT_BAD_PORTAL_TRIANGLE: u32 = 0x0106_a860;
/// `"Triangle %d, edge %d has a Portal to a Navmesh (%08x) in a different
/// Worldspace, Clearing the portal"`.
const FORMAT_PORTAL_OTHER_WORLD_SPACE: u32 = 0x0106_a7d8;
/// `"Triangle %d is degenerate, Vertices 0 and 1 both use vertex index %d"`
/// and the (0, 2) and (1, 2) forms.
const FORMAT_DEGENERATE_01: u32 = 0x0106_a770;
const FORMAT_DEGENERATE_02: u32 = 0x0106_a708;
const FORMAT_DEGENERATE_12: u32 = 0x0106_a6a0;
/// `"Triangle %d Edges 0 and 1 both point to the same triangle %d"` and the
/// (0, 2) and (1, 2) forms.
const FORMAT_SAME_NEIGHBOUR_01: u32 = 0x0106_a640;
const FORMAT_SAME_NEIGHBOUR_02: u32 = 0x0106_a5e0;
const FORMAT_SAME_NEIGHBOUR_12: u32 = 0x0106_a580;
/// `"Triangle %d and %d have opposite normals but are linked"`.
const FORMAT_OPPOSITE_NORMALS: u32 = 0x0106_a520;
/// `"Triangle %d Edge %d has a portal data to Triangle %d, but Triangle %d
/// doesn't have any Portal info for its matching edge %d"`.
const FORMAT_PORTAL_WITHOUT_MATCH: u32 = 0x0106_a480;
/// `"Triangle %d Edge %d Should have a link to Triangle %d, but doesn't"`.
const FORMAT_MISSING_LINK: u32 = 0x0106_a418;
/// `"Triangle %d Edge %d Should have Portal information, but doesn't"`.
const FORMAT_MISSING_PORTAL: u32 = 0x0106_a3b0;
/// `"Triangle %d has a link to Triangle %d, but Triangle %d doesn't have the
/// matching link"`.
const FORMAT_LINK_NOT_RETURNED: u32 = 0x0106_a330;
/// `"Triangles %d and %d are linked, but their vertices do not match"`.
const FORMAT_VERTICES_MISMATCH: u32 = 0x0106_a2c8;
/// `"Cover Array (index %d) referes to triangle %d, which does not exist."`
/// and the `"... which does not have cover."` form.
const FORMAT_COVER_MISSING_TRIANGLE: u32 = 0x0106_a260;
const FORMAT_COVER_WITHOUT_COVER: u32 = 0x0106_a1f0;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Address of `Triangles[edge]` (the neighbour slot) of a triangle.
fn neighbour_slot(tri: Ptr<NavMeshTriangle>, edge: i32) -> u32 {
    tri.addr()
        .wrapping_add(6)
        .wrapping_add((edge as u32).wrapping_mul(2))
}

/// `NavMesh::GetTriangle` (`005582f0`).
fn mesh_triangle(e: &mut Engine, mesh: Ptr<NavMesh>, index: u16) -> Ptr<NavMeshTriangle> {
    e.call(NAV_MESH_GET_TRIANGLE, &args![mesh, index]).ptr()
}

/// `NavMeshTriangle::GetVertex` (`005582d0`).
fn triangle_vertex(e: &mut Engine, tri: Ptr<NavMeshTriangle>, index: i32) -> u16 {
    e.call(TRIANGLE_VERTEX, &args![tri, index]).u16()
}

/// `BSSimpleArray::iSize` (`0044ddc0`).
fn array_size<T>(e: &mut Engine, array: Ptr<T>) -> u32 {
    e.call(ARRAY_SIZE, &args![array]).u32()
}

/// `Triangles[index]` through the triangle array's element getter
/// (`00558dd0`), as `CheckNavMesh` and `DeleteExtraEdgeInfo` call it.
fn triangle_element(e: &mut Engine, mesh: Ptr<NavMesh>, index: u32) -> Ptr<NavMeshTriangle> {
    let triangles = mesh.at(NavMesh::Triangles);
    e.call(TRIANGLE_ELEMENT, &args![triangles, index]).ptr()
}

// Translated from 0049c4c0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `NiTMapBase<DFALL<NiTMapItem<unsigned short, NiPointer<ObstacleData>>>,
/// unsigned short, NiPointer<ObstacleData>>::KeyToHashIndex` (Xbox PDB): the
/// bucket of a 16-bit key, `key % m_uiHashSize`.
pub fn ni_t_map_base_key_to_hash_index(e: &mut Engine, this: Ptr<NiTPointerMap>, key: u16) -> u32 {
    let hash_size = e.get(this, NiTPointerMap::m_uiHashSize);
    key as u32 % hash_size
}

// Translated from 0068eb80 (decompiled, FalloutNV.exe 1.4.0.525)
/// `NavMesh::NavMesh` (Xbox PDB): constructs the form and its two bases,
/// stores the three vtables, constructs every array and map member, sets the
/// form type to `0x43`, clears the parent cell and info pointers, and
/// allocates the triangle-to-obstacle map (16 bytes, 0x25 buckets; null when
/// the allocation fails). Returns `this`.
pub fn nav_mesh_nav_mesh(e: &mut Engine, this: Ptr<NavMesh>) -> Ptr<NavMesh> {
    e.call(TES_FORM_CONSTRUCTOR, &args![this]);
    e.call(
        CHILD_CELL_CONSTRUCTOR,
        &args![this.byte_add(CHILD_CELL_BASE)],
    );
    e.call(
        REF_OBJECT_CONSTRUCTOR,
        &args![this.byte_add(REF_OBJECT_BASE)],
    );
    e.mem.set_u32(this.addr(), NAV_MESH_VTABLE);
    e.mem
        .set_u32(this.addr() + CHILD_CELL_BASE, CHILD_CELL_VTABLE);
    e.mem
        .set_u32(this.addr() + REF_OBJECT_BASE, REF_OBJECT_VTABLE);
    e.call(
        ARRAY_CONSTRUCTOR_VERTICES,
        &args![this.at(NavMesh::Vertices)],
    );
    e.call(
        ARRAY_CONSTRUCTOR_TRIANGLES,
        &args![this.at(NavMesh::Triangles)],
    );
    e.call(
        ARRAY_CONSTRUCTOR_EXTRA_INFO,
        &args![this.at(NavMesh::ExtraEdgeInfo)],
    );
    e.call(
        ARRAY_CONSTRUCTOR_DOOR_PORTALS,
        &args![this.at(NavMesh::DoorPortals)],
    );
    e.call(
        ARRAY_CONSTRUCTOR_CLOSED_DOORS,
        &args![this.at(NavMesh::ClosedDoors)],
    );
    e.call(ARRAY_CONSTRUCTOR_U16, &args![this.at(NavMesh::CoverArray)]);
    e.call(
        MAP_CONSTRUCTOR,
        &args![this.at(NavMesh::POVs), OBSTACLE_MAP_BUCKETS],
    );
    e.call(ARRAY_CONSTRUCTOR_U16, &args![this.at(NavMesh::ClosestPOVs)]);
    e.call(GRID_CONSTRUCTOR, &args![this.byte_add(MESH_GRID_OFFSET)]);
    e.call(OBSTACLES_CONSTRUCTOR, &args![this.at(NavMesh::Obstacles)]);
    e.call(
        ARRAY_CONSTRUCTOR_U16,
        &args![this.at(NavMesh::ObstaclePOVs)],
    );
    e.call(
        AVOID_NODES_CONSTRUCTOR,
        &args![this.at(NavMesh::StaticAvoidNodes)],
    );
    e.call(SET_FORM_TYPE, &args![this, NAV_MESH_FORM_TYPE]);
    e.set(this, NavMesh::pParentCell, Ptr::NULL);
    e.set(this, NavMesh::pNavMeshInfo, Ptr::NULL);
    let block = e.call(OPERATOR_NEW, &args![0x10u32]).ptr::<()>();
    let map = if block.is_null() {
        Ptr::NULL
    } else {
        e.call(
            OBSTACLE_MAP_CONSTRUCTOR,
            &args![block, OBSTACLE_MAP_BUCKETS],
        )
        .ptr::<()>()
    };
    e.set(this, NavMesh::pTriangleToObstacleMap, map);
    this
}

// Translated from 0068ed30 (decompiled, FalloutNV.exe 1.4.0.525)
/// The `NavMesh` scalar deleting destructor: runs the destructor `0068ed70`,
/// then frees the 0x108-byte object when bit 0 of `flags` is set. Returns
/// `this`.
pub fn fn_0068ed30(e: &mut Engine, this: Ptr<NavMesh>, flags: u32) -> Ptr<NavMesh> {
    fn_0068ed70(e, this);
    if flags & 1 != 0 {
        e.call(FREE_BLOCK, &args![this, NAV_MESH_SIZE]);
    }
    this
}

// Translated from 0068ed70 (decompiled, FalloutNV.exe 1.4.0.525)
/// The `NavMesh` destructor body: restores the three vtables, empties the
/// vertex, triangle, extra-info, door-portal and cover arrays and the grid,
/// detaches itself from its `NavMeshInfo` (`pNavMesh` = null), deletes the
/// triangle-to-obstacle map through its virtual destructor, then destroys
/// the members in reverse order and finally the `NiRefObject` base and the
/// form.
pub fn fn_0068ed70(e: &mut Engine, this: Ptr<NavMesh>) {
    e.mem.set_u32(this.addr(), NAV_MESH_VTABLE);
    e.mem
        .set_u32(this.addr() + CHILD_CELL_BASE, CHILD_CELL_VTABLE);
    e.mem
        .set_u32(this.addr() + REF_OBJECT_BASE, REF_OBJECT_VTABLE);
    for array in [
        this.at(NavMesh::Vertices),
        this.at(NavMesh::Triangles),
        this.at(NavMesh::ExtraEdgeInfo),
        this.at(NavMesh::DoorPortals),
        this.at(NavMesh::CoverArray),
    ] {
        e.call(ARRAY_CLEAR, &args![array, 1u32]);
    }
    e.call(GRID_CLEAR, &args![this.byte_add(MESH_GRID_OFFSET)]);
    let info = e.get(this, NavMesh::pNavMeshInfo);
    if !info.is_null() {
        e.call(INFO_SET_NAV_MESH, &args![info, 0u32]);
    }
    let map = e.get(this, NavMesh::pTriangleToObstacleMap);
    if !map.is_null() {
        // Scalar deleting destructor (slot 0) of the map, with delete.
        e.vcall(map.addr(), 0, &args![1u32]);
    }
    e.call(
        AVOID_NODES_DESTRUCTOR,
        &args![this.at(NavMesh::StaticAvoidNodes)],
    );
    e.call(ARRAY_DESTRUCTOR_U16, &args![this.at(NavMesh::ObstaclePOVs)]);
    e.call(OBSTACLES_DESTRUCTOR, &args![this.at(NavMesh::Obstacles)]);
    e.call(ARRAY_DESTRUCTOR_U16, &args![this.at(NavMesh::ClosestPOVs)]);
    e.call(MAP_DESTRUCTOR, &args![this.at(NavMesh::POVs)]);
    e.call(ARRAY_DESTRUCTOR_U16, &args![this.at(NavMesh::CoverArray)]);
    e.call(
        ARRAY_DESTRUCTOR_CLOSED_DOORS,
        &args![this.at(NavMesh::ClosedDoors)],
    );
    e.call(
        ARRAY_DESTRUCTOR_DOOR_PORTALS,
        &args![this.at(NavMesh::DoorPortals)],
    );
    e.call(
        ARRAY_DESTRUCTOR_EXTRA_INFO,
        &args![this.at(NavMesh::ExtraEdgeInfo)],
    );
    e.call(
        ARRAY_DESTRUCTOR_TRIANGLES,
        &args![this.at(NavMesh::Triangles)],
    );
    e.call(
        ARRAY_DESTRUCTOR_VERTICES,
        &args![this.at(NavMesh::Vertices)],
    );
    e.call(
        REF_OBJECT_DESTRUCTOR,
        &args![this.byte_add(REF_OBJECT_BASE)],
    );
    e.call(TES_FORM_DESTRUCTOR, &args![this]);
}

// Translated from 0068ef50 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESChildCell::GetSaveParentCell` (Xbox PDB name of slot 0 of the
/// `TESChildCell` base, whose vtable `0106a0a8` points here): `this` is the
/// base at `NavMesh + 0x18`; returns the navmesh's parent cell (`0059bb30` on
/// the `NavMesh`).
pub fn fn_0068ef50(e: &mut Engine, this: Ptr) -> u32 {
    let mesh = Ptr::<NavMesh>::new(this.addr().wrapping_sub(CHILD_CELL_BASE));
    e.call(GET_PARENT_CELL, &args![mesh]).u32()
}

// Translated from 0068ef70 (decompiled, FalloutNV.exe 1.4.0.525)
/// `NavMesh::GetParentSpace` (Xbox PDB): the worldspace of the parent cell
/// if it has one, else the parent cell itself; null without a parent cell.
pub fn nav_mesh_get_parent_space(e: &mut Engine, this: Ptr<NavMesh>) -> Ptr {
    let cell = e.get(this, NavMesh::pParentCell);
    if cell.is_null() {
        return Ptr::NULL;
    }
    let world_space = e.call(CELL_GET_WORLD_SPACE, &args![cell]).u32();
    if world_space != 0 {
        e.call(CELL_GET_WORLD_SPACE, &args![cell]).ptr()
    } else {
        cell
    }
}

// Translated from 0068efb0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Returns the navmesh's `NavMeshInfo` (`pNavMeshInfo`, +0x104).
pub fn fn_0068efb0(e: &mut Engine, this: Ptr<NavMesh>) -> Ptr {
    e.get(this, NavMesh::pNavMeshInfo)
}

// Translated from 0068efd0 (decompiled, FalloutNV.exe 1.4.0.525)
/// The two vertex indexes of edge `edge` of triangle `triangle`: stores
/// them as two consecutive `u16` (vertex `edge`, then vertex `(edge + 1) % 3`)
/// in the 4 bytes at `result`, and returns `result`.
pub fn fn_0068efd0(
    e: &mut Engine,
    this: Ptr<NavMesh>,
    result: Ptr,
    triangle: u16,
    edge: i32,
) -> Ptr {
    let tri = mesh_triangle(e, this, triangle);
    let first = triangle_vertex(e, tri, edge);
    let next = (edge + 1) % 3;
    let tri = mesh_triangle(e, this, triangle);
    let second = triangle_vertex(e, tri, next);
    e.mem
        .set_u32(result.addr(), first as u32 | (second as u32) << 16);
    result
}

// Translated from 0068f040 (decompiled, FalloutNV.exe 1.4.0.525)
/// `NavMesh::GetEdgeVertices` (Xbox PDB): stores at `result` the addresses
/// of the two vertices that bound edge `edge` of triangle `triangle`, and
/// returns `result`.
pub fn nav_mesh_get_edge_vertices(
    e: &mut Engine,
    this: Ptr<NavMesh>,
    result: Ptr,
    triangle: u16,
    edge: i32,
) -> Ptr {
    e.with_stack(4, |e, pair| {
        fn_0068efd0(e, this, pair, triangle, edge);
        let first = e.mem.u16(pair.addr());
        let second = e.mem.u16(pair.addr() + 2);
        let first_vertex = fn_0068f0a0(e, this, first);
        let second_vertex = fn_0068f0a0(e, this, second);
        e.mem.set_u32(result.addr(), first_vertex.addr());
        e.mem.set_u32(result.addr() + 4, second_vertex.addr());
    });
    result
}

// Translated from 0068f0a0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Address of vertex `index` of the mesh (`Vertices[index]`).
pub fn fn_0068f0a0(e: &mut Engine, this: Ptr<NavMesh>, index: u16) -> Ptr {
    let vertices = this.at(NavMesh::Vertices);
    e.call(ELEMENT_12, &args![vertices, index]).ptr()
}

// Translated from 0068f0c0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Copies the coordinates (three floats each) of the two vertices bounding
/// edge `edge` of triangle `triangle` to `first` and `second`. Always
/// returns true.
pub fn fn_0068f0c0(
    e: &mut Engine,
    this: Ptr<NavMesh>,
    triangle: u16,
    edge: i32,
    first: Ptr,
    second: Ptr,
) -> bool {
    let tri = mesh_triangle(e, this, triangle);
    let first_index = triangle_vertex(e, tri, edge);
    let next = (edge + 1) % 3;
    let tri = mesh_triangle(e, this, triangle);
    let second_index = triangle_vertex(e, tri, next);
    for (index, out) in [(first_index, first), (second_index, second)] {
        let vertex = fn_0068f0a0(e, this, index);
        for word in 0..3 {
            let value = e.mem.u32(vertex.addr() + 4 * word);
            e.mem.set_u32(out.addr() + 4 * word, value);
        }
    }
    true
}

// Translated from 0068f160 (decompiled, FalloutNV.exe 1.4.0.525)
/// Stores the addresses of the three vertices of triangle `triangle` at
/// `first`, `second` and `third`.
pub fn fn_0068f160(
    e: &mut Engine,
    this: Ptr<NavMesh>,
    triangle: u16,
    first: Ptr,
    second: Ptr,
    third: Ptr,
) {
    let tri = mesh_triangle(e, this, triangle);
    for (corner, out) in [(0, first), (1, second), (2, third)] {
        let index = triangle_vertex(e, tri, corner);
        let vertex = fn_0068f0a0(e, this, index);
        e.mem.set_u32(out.addr(), vertex.addr());
    }
}

// Translated from 0068f1d0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `NavMeshTriangle` method: whether bit `edge` of `TriangleFlags` is set,
/// that is whether the edge has extra info (only the low five bits of
/// `edge` count, as for an x86 shift).
pub fn fn_0068f1d0(e: &mut Engine, this: Ptr<NavMeshTriangle>, edge: u32) -> bool {
    let flags = e.get(this, NavMeshTriangle::TriangleFlags);
    (1u32 << (edge & 31)) & flags != 0
}

// Translated from 0068f200 (decompiled, FalloutNV.exe 1.4.0.525)
/// `NavMeshTriangle` method: whether neighbour slot `Triangles[edge]` holds
/// something other than `0xFFFF`.
pub fn fn_0068f200(e: &mut Engine, this: Ptr<NavMeshTriangle>, edge: i32) -> bool {
    e.mem.u16(neighbour_slot(this, edge)) != NO_NEIGHBOUR
}

// Translated from 0068f230 (decompiled, FalloutNV.exe 1.4.0.525)
/// `NavMesh::GetEdgeExtraInfo` (Xbox PDB): the `EdgeExtraInfo` of edge
/// `edge` of triangle `triangle`, or null if the edge has no extra info
/// (flag clear, or neighbour slot `0xFFFF`).
pub fn nav_mesh_get_edge_extra_info(
    e: &mut Engine,
    this: Ptr<NavMesh>,
    triangle: u16,
    edge: i32,
) -> Ptr<EdgeExtraInfo> {
    let triangles = this.at(NavMesh::Triangles);
    let tri: Ptr<NavMeshTriangle> = e.call(TRIANGLE_ELEMENT, &args![triangles, triangle]).ptr();
    if !fn_0068f1d0(e, tri, edge as u32) {
        return Ptr::NULL;
    }
    let tri: Ptr<NavMeshTriangle> = e.call(TRIANGLE_ELEMENT, &args![triangles, triangle]).ptr();
    if fn_0068f2c0(e, tri, edge) == NO_NEIGHBOUR {
        return Ptr::NULL;
    }
    let tri: Ptr<NavMeshTriangle> = e.call(TRIANGLE_ELEMENT, &args![triangles, triangle]).ptr();
    let slot = fn_0068f2c0(e, tri, edge);
    let infos = this.at(NavMesh::ExtraEdgeInfo);
    e.call(ELEMENT_12, &args![infos, slot]).ptr()
}

// Translated from 0068f2c0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `NavMeshTriangle` method: the neighbour slot `Triangles[edge]`.
pub fn fn_0068f2c0(e: &mut Engine, this: Ptr<NavMeshTriangle>, edge: i32) -> u16 {
    e.mem.u16(neighbour_slot(this, edge))
}

// Translated from 0068f2e0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `NavMeshInfo` method: stores the info's navmesh pointer (`pNavMesh`) at
/// `result`, or null when the info is flagged (`0068f320`); returns whether
/// the stored pointer is non-null.
pub fn fn_0068f2e0(e: &mut Engine, this: Ptr, result: Ptr) -> bool {
    if fn_0068f320(e, this) {
        e.mem.set_u32(result.addr(), 0);
        false
    } else {
        let mesh = e.mem.u32(this.addr() + INFO_NAV_MESH);
        e.mem.set_u32(result.addr(), mesh);
        e.mem.u32(this.addr() + INFO_NAV_MESH) != 0
    }
}

// Translated from 0068f320 (decompiled, FalloutNV.exe 1.4.0.525)
/// `NavMeshInfo` method: whether bit 0x10 of `uiFlags` is set.
pub fn fn_0068f320(e: &mut Engine, this: Ptr) -> bool {
    e.mem.u32(this.addr() + INFO_FLAGS) & 0x10 != 0
}

// Translated from 0068f340 (decompiled, FalloutNV.exe 1.4.0.525)
/// `NavMesh::GetMatchingEdge` (Xbox PDB): finds the edge on the other side
/// of the edge named by `edge_ref` (a record with the triangle index as
/// `u16` at +4 and the edge number at +8), and stores the other mesh's
/// `NavMeshInfo` at `result + 0`, the other triangle at `result + 4` (`u16`)
/// and the other edge at `result + 8`. Returns whether a match exists.
pub fn nav_mesh_get_matching_edge(
    e: &mut Engine,
    this: Ptr<NavMesh>,
    edge_ref: Ptr,
    result: Ptr,
) -> bool {
    e.with_stack(12, |e, locals| {
        // The matched mesh (+0), the matched triangle (+4), the matched edge
        // (+8, starts at -1).
        e.mem.set_u32(locals.addr() + 8, 0xffff_ffff);
        let triangle = e.mem.u16(edge_ref.addr() + 4);
        let edge = e.mem.i32(edge_ref.addr() + 8);
        let found = nav_mesh_get_matching_edge_ov3(
            e,
            this,
            triangle,
            edge,
            locals,
            locals.byte_add(4),
            locals.byte_add(8),
        );
        if found {
            let mesh = e.mem.u32(locals.addr());
            let info = fn_0068efb0(e, Ptr::new(mesh));
            e.mem.set_u32(result.addr(), info.addr());
            let other_triangle = e.mem.u16(locals.addr() + 4);
            e.mem.set_u16(result.addr() + 4, other_triangle);
            let other_edge = e.mem.u32(locals.addr() + 8);
            e.mem.set_u32(result.addr() + 8, other_edge);
        }
        found
    })
}

// Translated from 0068f3b0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `NavMesh::GetMatchingEdge` overload 2 (Xbox PDB): as overload 3, but the
/// matched mesh is delivered into the `NiPointer<NavMesh>` at `mesh_out`
/// (through a temporary smart pointer), the triangle into `triangle_out`
/// (`u16`) and the edge into `edge_out`.
pub fn nav_mesh_get_matching_edge_ov2(
    e: &mut Engine,
    this: Ptr<NavMesh>,
    triangle: u16,
    edge: i32,
    mesh_out: Ptr,
    triangle_out: Ptr,
    edge_out: Ptr,
) -> bool {
    e.with_stack(4, |e, found_mesh| {
        let found = nav_mesh_get_matching_edge_ov3(
            e,
            this,
            triangle,
            edge,
            found_mesh,
            triangle_out,
            edge_out,
        );
        if found {
            let mesh = e.mem.u32(found_mesh.addr());
            e.with_stack(8, |e, temp| {
                let temp = e
                    .call(POINTER_CONSTRUCT_FROM_CDECL, &args![temp, mesh])
                    .u32();
                e.call(POINTER_ASSIGN, &args![mesh_out, temp]);
                e.call(POINTER_DESTRUCT, &args![temp]);
            });
        }
        found
    })
}

// Translated from 0068f460 (decompiled, FalloutNV.exe 1.4.0.525)
/// `NavMesh::GetMatchingEdge` overload 3 (Xbox PDB): finds the edge that
/// mirrors edge `edge` of triangle `triangle`. Without extra info on the
/// edge the neighbour is in this mesh and `*mesh_out` = `this`; with extra
/// info the portal's other mesh is used, and the edge must point back with a
/// portal naming this mesh's info and `triangle`. Stores the mesh, the
/// neighbour triangle (`u16`) and the neighbour's edge, and returns whether
/// a mirror edge exists.
pub fn nav_mesh_get_matching_edge_ov3(
    e: &mut Engine,
    this: Ptr<NavMesh>,
    triangle: u16,
    edge: i32,
    mesh_out: Ptr,
    triangle_out: Ptr,
    edge_out: Ptr,
) -> bool {
    let count = e.call(NAV_MESH_TRIANGLE_COUNT, &args![this]).u32();
    if triangle as u32 >= count {
        return false;
    }
    let tri = mesh_triangle(e, this, triangle);
    if !fn_0068f200(e, tri, edge) {
        return false;
    }
    if !fn_0068f1d0(e, tri, edge as u32) {
        e.mem.set_u32(mesh_out.addr(), this.addr());
        let neighbour = fn_0068f2c0(e, tri, edge);
        e.mem.set_u16(triangle_out.addr(), neighbour);
        let neighbour_index = e.mem.u16(triangle_out.addr());
        let neighbour_tri = mesh_triangle(e, this, neighbour_index);
        let mut other_edge = 0i32;
        while other_edge < 3 {
            if !fn_0068f1d0(e, neighbour_tri, other_edge as u32)
                && fn_0068f2c0(e, neighbour_tri, other_edge) == triangle
            {
                break;
            }
            other_edge += 1;
        }
        if other_edge >= 3 {
            return false;
        }
        e.mem.set_i32(edge_out.addr(), other_edge);
        return true;
    }
    let info = nav_mesh_get_edge_extra_info(e, this, triangle, edge);
    let portal_info = e.get(info, EdgeExtraInfo::pOtherMesh);
    if portal_info.is_null() {
        return false;
    }
    e.with_stack(4, |e, other_mesh| {
        if !fn_0068f2e0(e, portal_info, other_mesh) {
            return false;
        }
        let other = Ptr::<NavMesh>::new(e.mem.u32(other_mesh.addr()));
        let other_triangle = e.get(info, EdgeExtraInfo::sTriangle);
        let other_count = e.call(NAV_MESH_TRIANGLE_COUNT, &args![other]).u32();
        if other_triangle as u32 >= other_count {
            return false;
        }
        let other_tri = mesh_triangle(e, other, other_triangle);
        let mut other_edge = 0i32;
        while other_edge < 3 {
            if fn_0068f1d0(e, other_tri, other_edge as u32) {
                let back = nav_mesh_get_edge_extra_info(e, other, other_triangle, other_edge);
                if !back.is_null() {
                    let own_info = fn_0068efb0(e, this);
                    if e.get(back, EdgeExtraInfo::pOtherMesh) == own_info
                        && e.get(back, EdgeExtraInfo::sTriangle) == triangle
                    {
                        break;
                    }
                }
            }
            other_edge += 1;
        }
        if other_edge == 3 {
            return false;
        }
        e.mem.set_u32(mesh_out.addr(), other.addr());
        e.mem.set_u16(triangle_out.addr(), other_triangle);
        e.mem.set_i32(edge_out.addr(), other_edge);
        true
    })
}

// Translated from 0068f670 (decompiled, FalloutNV.exe 1.4.0.525)
/// `NavMesh::GetMatchingEdge` overload 4 (Xbox PDB): overload 3 on the edge
/// named by `edge_ref` (triangle `u16` at +4, edge at +8), writing the mesh,
/// triangle and edge to the first three words of `result`. Returns overload
/// 3's result.
pub fn nav_mesh_get_matching_edge_ov4(
    e: &mut Engine,
    this: Ptr<NavMesh>,
    edge_ref: Ptr,
    result: Ptr,
) -> bool {
    let triangle = e.mem.u16(edge_ref.addr() + 4);
    let edge = e.mem.i32(edge_ref.addr() + 8);
    nav_mesh_get_matching_edge_ov3(
        e,
        this,
        triangle,
        edge,
        result,
        result.byte_add(4),
        result.byte_add(8),
    )
}

// Translated from 0068f6b0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `DoorPortals[index]` through the array's element getter (`00844160`);
/// returns its result.
pub fn fn_0068f6b0(e: &mut Engine, this: Ptr<NavMesh>, index: u16) -> u32 {
    let portals = this.at(NavMesh::DoorPortals);
    e.call(DOOR_PORTAL_ELEMENT, &args![portals, index]).u32()
}

/// One report of `CheckNavMesh`: the format, then the navmesh's form id, the
/// cell text's characters and the numbers `extra`, to the debug print.
fn report(e: &mut Engine, this: Ptr<NavMesh>, text: Ptr, format: u32, extra: &[u32]) {
    let characters = e.call(POINTER_GET, &args![text]).u32();
    let form_id = e.call(GET_FORM_ID, &args![this]).u32();
    let mut words = vec![format, form_id, characters];
    words.extend_from_slice(extra);
    e.call(DEBUG_PRINT, &words);
}

/// Whether one coordinate (`offset` 0, 4 or 8) of vertex `index` is finite
/// and not larger than the limit in magnitude.
fn vertex_coordinate_ok(
    e: &mut Engine,
    vertices: Ptr<BSSimpleArray>,
    index: u32,
    offset: u32,
) -> bool {
    let vertex = e.call(ELEMENT_12, &args![vertices, index]).u32();
    let value = e.mem.f32(vertex + offset);
    if e.call(FINITE, &args![value as f64]).i32() == 0 {
        return false;
    }
    let vertex = e.call(ELEMENT_12, &args![vertices, index]).u32();
    let value = e.mem.f32(vertex + offset);
    let magnitude = e.call(FLOAT_ABS, &args![value]).f64();
    let limit: f64 = e.global(MAX_COORDINATE);
    !matches!(
        magnitude.partial_cmp(&limit),
        Some(std::cmp::Ordering::Greater)
    )
}

// Translated from 0068f6d0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `NavMesh::CheckNavMesh` (Xbox PDB): validates the mesh, repairs what it
/// can and reports each problem through the debug print (format, form id,
/// cell text, numbers). Checks, in order: vertex coordinates (finite, below
/// 1e14; stops at the first bad one), a large triangle count (2000 or
/// more), triangle neighbour indexes (bad ones are cleared), downfacing
/// triangles (flipped), extra-info indexes and portals (bad ones are
/// deleted; a portal must reach a triangle that exists and a mesh in the same
/// worldspace), degenerate triangles, two edges pointing to the same
/// neighbour, opposite normals and missing portal data between neighbouring
/// triangles, links the neighbour does not return and shared vertices
/// that do not match, and the cover list. It ends with the cross-cell portal
/// check `006908f0` (its result is not used). Returns true if no problem was
/// found. The word argument is not read.
pub fn nav_mesh_check_nav_mesh(e: &mut Engine, this: Ptr<NavMesh>, _unused_1: u32) -> bool {
    let mut problems = false;
    let text = Ptr::<()>::new(e.mem.alloc(8));
    e.call(STRING_CONSTRUCT, &args![text]);
    let cell = e.call(GET_PARENT_CELL, &args![this]).u32();
    e.vcall(cell, GET_FORM_DETAILED_STRING_SLOT, &args![text]);

    let vertices = this.at(NavMesh::Vertices);
    let triangles = this.at(NavMesh::Triangles);

    // Vertex coordinates; the first bad one ends this check.
    let mut index = 0u32;
    'vertices: while index < array_size(e, vertices) {
        for (offset, format) in [(0u32, FORMAT_BAD_X), (4, FORMAT_BAD_Y), (8, FORMAT_BAD_Z)] {
            if !vertex_coordinate_ok(e, vertices, index, offset) {
                report(e, this, text, format, &[]);
                problems = true;
                break 'vertices;
            }
        }
        index += 1;
    }

    if array_size(e, triangles) >= 2000 {
        let count = array_size(e, triangles);
        report(e, this, text, FORMAT_MANY_TRIANGLES, &[count]);
    }

    // Neighbour indexes out of range, and downfacing triangles.
    let mut t = 0u32;
    while t < array_size(e, triangles) {
        let tri = triangle_element(e, this, t);
        for edge in 0..3i32 {
            if !fn_0068f1d0(e, tri, edge as u32) && fn_0068f200(e, tri, edge) {
                let neighbour = fn_0068f2c0(e, tri, edge);
                if neighbour as u32 >= array_size(e, triangles) {
                    report(e, this, text, FORMAT_BAD_TRIANGLE_INDEX, &[t, edge as u32]);
                    fn_006907d0(e, tri, edge as u32, 0xffff);
                    problems = true;
                }
            }
        }
        e.with_stack(0x10, |e, normal| {
            e.call(TRIANGLE_NORMAL, &args![this, normal, t as u16]);
            let z = e.mem.f32(normal.addr() + 8);
            let zero: f64 = e.global(ZERO);
            if matches!(
                (z as f64).partial_cmp(&zero),
                Some(std::cmp::Ordering::Less)
            ) {
                report(e, this, text, FORMAT_DOWNFACING, &[t]);
                e.call(FLIP_TRIANGLE, &args![this, t as u16]);
                problems = true;
            }
        });
        t += 1;
    }

    // Extra edge info: indexes and portals.
    let mut t = 0u32;
    while t < array_size(e, triangles) {
        let tri = mesh_triangle(e, this, t as u16);
        for edge in 0..3i32 {
            if !fn_0068f1d0(e, tri, edge as u32) {
                continue;
            }
            let slot = fn_0068f2c0(e, tri, edge);
            let infos = this.at(NavMesh::ExtraEdgeInfo);
            if slot as u32 >= array_size(e, infos) {
                report(
                    e,
                    this,
                    text,
                    FORMAT_BAD_EXTRA_INFO_INDEX,
                    &[t, edge as u32],
                );
                fn_00690790(e, tri, edge as u32);
                problems = true;
                continue;
            }
            let info = nav_mesh_get_edge_extra_info(e, this, t as u16, edge);
            if e.get(info, EdgeExtraInfo::pOtherMesh).is_null() {
                let slot = fn_0068f2c0(e, tri, edge);
                nav_mesh_delete_extra_edge_info(e, this, slot);
                problems = true;
                continue;
            }
            if e.get(info, EdgeExtraInfo::sTriangle) == NO_NEIGHBOUR {
                let slot = fn_0068f2c0(e, tri, edge);
                nav_mesh_delete_extra_edge_info(e, this, slot);
                problems = true;
                continue;
            }
            let portal_info = e.get(info, EdgeExtraInfo::pOtherMesh);
            let other_triangle = e.get(info, EdgeExtraInfo::sTriangle);
            let holder = Ptr::<()>::new(e.mem.alloc(8));
            e.call(POINTER_CONSTRUCT, &args![holder]);
            let mut cleared = false;
            if fn_00690860(e, portal_info, holder) {
                let other = e.call(POINTER_GET, &args![holder]).u32();
                let other_count = e.call(NAV_MESH_TRIANGLE_COUNT, &args![other]).u32();
                if other_triangle as u32 >= other_count {
                    let other = e.call(POINTER_GET, &args![holder]).u32();
                    let other_id = e.call(GET_FORM_ID, &args![other]).u32();
                    report(
                        e,
                        this,
                        text,
                        FORMAT_BAD_PORTAL_TRIANGLE,
                        &[t, edge as u32, other_id, other_triangle as u32],
                    );
                    let slot = fn_0068f2c0(e, tri, edge);
                    nav_mesh_delete_extra_edge_info(e, this, slot);
                    problems = true;
                    cleared = true;
                }
            }
            if !cleared {
                let cell = e.call(GET_PARENT_CELL, &args![this]).u32();
                let own_space = e.call(CELL_GET_WORLD_SPACE, &args![cell]).u32();
                let portal_space = fn_00690800(e, portal_info).addr();
                if own_space != portal_space {
                    let portal_id = e.call(POINTER_GET, &args![portal_info]).u32();
                    report(
                        e,
                        this,
                        text,
                        FORMAT_PORTAL_OTHER_WORLD_SPACE,
                        &[t, edge as u32, portal_id],
                    );
                    let slot = fn_0068f2c0(e, tri, edge);
                    nav_mesh_delete_extra_edge_info(e, this, slot);
                    problems = true;
                }
            }
            e.call(POINTER_DESTRUCT, &args![holder]);
            e.mem.free(holder.addr());
        }
        t += 1;
    }

    // Degenerate triangles: two corners share a vertex.
    let mut t = 0u32;
    while t < array_size(e, triangles) {
        let tri = mesh_triangle(e, this, t as u16);
        for (a, b, format) in [
            (0, 1, FORMAT_DEGENERATE_01),
            (0, 2, FORMAT_DEGENERATE_02),
            (1, 2, FORMAT_DEGENERATE_12),
        ] {
            if triangle_vertex(e, tri, a) == triangle_vertex(e, tri, b) {
                let shared = triangle_vertex(e, tri, a);
                report(e, this, text, format, &[t, shared as u32]);
                problems = true;
            }
        }
        t += 1;
    }

    // Two edges leading to the same neighbour.
    let mut t = 0u32;
    while t < array_size(e, triangles) {
        let tri = mesh_triangle(e, this, t as u16);
        for (a, b, format) in [
            (0, 1, FORMAT_SAME_NEIGHBOUR_01),
            (0, 2, FORMAT_SAME_NEIGHBOUR_02),
            (1, 2, FORMAT_SAME_NEIGHBOUR_12),
        ] {
            if fn_0068f2c0(e, tri, a) == fn_0068f2c0(e, tri, b)
                && fn_0068f2c0(e, tri, a) != NO_NEIGHBOUR
                && fn_0068f1d0(e, tri, a as u32) == fn_0068f1d0(e, tri, b as u32)
            {
                let shared = fn_0068f2c0(e, tri, a);
                report(e, this, text, format, &[t, shared as u32]);
                problems = true;
            }
        }
        t += 1;
    }

    // Triangles sharing an edge: orientation and portal data.
    let mut t = 0u32;
    while t < array_size(e, triangles) {
        let tri = mesh_triangle(e, this, t as u16);
        for edge in 0..3i32 {
            let next_edge = (edge + 1) % 3;
            let edge_start = triangle_vertex(e, tri, edge);
            let edge_end = triangle_vertex(e, tri, next_edge);
            let mut other = 0u32;
            while other < array_size(e, triangles) {
                if other != t {
                    let other_tri = mesh_triangle(e, this, other as u16);
                    for corner in 0..3i32 {
                        if triangle_vertex(e, other_tri, corner) != edge_end {
                            continue;
                        }
                        let corner_next = (corner + 1) % 3;
                        for probe in 0..3i32 {
                            if probe == corner || triangle_vertex(e, other_tri, probe) != edge_start
                            {
                                continue;
                            }
                            if probe != corner_next {
                                report(e, this, text, FORMAT_OPPOSITE_NORMALS, &[t, other]);
                                problems = true;
                            }
                            if !fn_0068f1d0(e, tri, edge as u32) {
                                if fn_0068f2c0(e, tri, edge) != NO_NEIGHBOUR {
                                    if fn_0068f1d0(e, other_tri, corner as u32) {
                                        report(
                                            e,
                                            this,
                                            text,
                                            FORMAT_PORTAL_WITHOUT_MATCH,
                                            &[t, edge as u32, other, other, corner as u32],
                                        );
                                        problems = true;
                                    }
                                    if fn_0068f2c0(e, tri, edge) as u32 != other {
                                        report(
                                            e,
                                            this,
                                            text,
                                            FORMAT_MISSING_LINK,
                                            &[t, edge as u32, other],
                                        );
                                        problems = true;
                                    }
                                    if fn_0068f2c0(e, other_tri, corner) as u32 != t {
                                        report(
                                            e,
                                            this,
                                            text,
                                            FORMAT_MISSING_LINK,
                                            &[other, corner as u32, t],
                                        );
                                        problems = true;
                                    }
                                }
                            } else if !fn_0068f1d0(e, other_tri, corner as u32) {
                                report(
                                    e,
                                    this,
                                    text,
                                    FORMAT_MISSING_PORTAL,
                                    &[other, corner as u32],
                                );
                                problems = true;
                            }
                        }
                    }
                }
                other += 1;
            }
        }
        t += 1;
    }

    // Every link must be returned by the neighbour, with matching vertices.
    let mut t = 0u32;
    while t < array_size(e, triangles) {
        let tri = mesh_triangle(e, this, t as u16);
        for edge in 0..3i32 {
            if fn_0068f1d0(e, tri, edge as u32) || fn_0068f2c0(e, tri, edge) == NO_NEIGHBOUR {
                continue;
            }
            let neighbour = fn_0068f2c0(e, tri, edge);
            let neighbour_tri = mesh_triangle(e, this, neighbour);
            let mut found = false;
            let mut back = 0i32;
            while back < 3 {
                if !fn_0068f1d0(e, neighbour_tri, back as u32)
                    && fn_0068f2c0(e, neighbour_tri, back) != NO_NEIGHBOUR
                    && fn_0068f2c0(e, neighbour_tri, back) as u32 == t
                {
                    found = true;
                    break;
                }
                back += 1;
            }
            if !found {
                let first = fn_0068f2c0(e, tri, edge);
                let second = fn_0068f2c0(e, tri, edge);
                report(
                    e,
                    this,
                    text,
                    FORMAT_LINK_NOT_RETURNED,
                    &[t, second as u32, first as u32],
                );
                problems = true;
            }
            if back < 3 {
                let next_edge = (edge + 1) % 3;
                let back_next = (back + 1) % 3;
                let matching = triangle_vertex(e, tri, edge)
                    == triangle_vertex(e, neighbour_tri, back_next)
                    && triangle_vertex(e, tri, next_edge)
                        == triangle_vertex(e, neighbour_tri, back);
                if !matching {
                    let linked = fn_0068f2c0(e, tri, edge);
                    report(e, this, text, FORMAT_VERTICES_MISMATCH, &[t, linked as u32]);
                    problems = true;
                }
            }
        }
        t += 1;
    }

    // The cover list names existing triangles that have cover.
    let cover = this.at(NavMesh::CoverArray);
    let mut cover_index = 0u32;
    while cover_index < array_size(e, cover) {
        let slot = e.call(U16_ELEMENT, &args![cover, cover_index]).u32();
        let cover_triangle = e.mem.u16(slot);
        let tri = mesh_triangle(e, this, cover_triangle);
        if tri.is_null() {
            report(
                e,
                this,
                text,
                FORMAT_COVER_MISSING_TRIANGLE,
                &[cover_index, cover_triangle as u32],
            );
            problems = true;
        } else if !fn_00690770(e, tri) {
            report(
                e,
                this,
                text,
                FORMAT_COVER_WITHOUT_COVER,
                &[cover_index, cover_triangle as u32],
            );
            problems = true;
        }
        cover_index += 1;
    }

    fn_006908f0(e, this);
    let ok = !problems;
    e.call(STRING_DESTRUCT, &args![text]);
    e.mem.free(text.addr());
    ok
}

// Translated from 00690770 (decompiled, FalloutNV.exe 1.4.0.525)
/// `NavMeshTriangle` method: whether any cover flag (mask `0x0FBE0000`) of
/// `TriangleFlags` is set.
pub fn fn_00690770(e: &mut Engine, this: Ptr<NavMeshTriangle>) -> bool {
    e.get(this, NavMeshTriangle::TriangleFlags) & COVER_FLAGS != 0
}

// Translated from 00690790 (decompiled, FalloutNV.exe 1.4.0.525)
/// `NavMeshTriangle` method: clears edge info flag `edge` and sets the
/// neighbour slot `Triangles[edge]` to `0xFFFF` (no neighbour).
pub fn fn_00690790(e: &mut Engine, this: Ptr<NavMeshTriangle>, edge: u32) {
    let flags = e.get(this, NavMeshTriangle::TriangleFlags);
    e.set(
        this,
        NavMeshTriangle::TriangleFlags,
        !(1u32 << (edge & 31)) & flags,
    );
    e.mem
        .set_u16(neighbour_slot(this, edge as i32), NO_NEIGHBOUR);
}

// Translated from 006907d0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `NavMeshTriangle` method: clears edge info flag `edge` (as `00690790`),
/// then stores `value` in the neighbour slot `Triangles[edge]`.
pub fn fn_006907d0(e: &mut Engine, this: Ptr<NavMeshTriangle>, edge: u32, value: u16) {
    fn_00690790(e, this, edge);
    e.mem.set_u16(neighbour_slot(this, edge as i32), value);
}

// Translated from 00690800 (decompiled, FalloutNV.exe 1.4.0.525)
/// `NavMeshInfo` method: the info's parent space (`00690830`) cast with
/// `__RTDynamicCast` from `TESForm` to `TESWorldSpace`; null if it is not a
/// worldspace.
pub fn fn_00690800(e: &mut Engine, this: Ptr) -> Ptr {
    let space = fn_00690830(e, this);
    e.call(
        RT_DYNAMIC_CAST,
        &args![space, 0i32, TYPE_TES_FORM, TYPE_TES_WORLD_SPACE, 0i32],
    )
    .ptr()
}

// Translated from 00690830 (decompiled, FalloutNV.exe 1.4.0.525)
/// `NavMeshInfo` method: `pParentSpace` if set, else the form found by
/// `ParentSpaceID` (`004839c0`).
pub fn fn_00690830(e: &mut Engine, this: Ptr) -> u32 {
    let space = e.mem.u32(this.addr() + INFO_PARENT_SPACE);
    if space != 0 {
        space
    } else {
        let id = e.mem.u32(this.addr() + INFO_PARENT_SPACE_ID);
        e.call(LOOKUP_FORM, &args![id]).u32()
    }
}

// Translated from 00690860 (decompiled, FalloutNV.exe 1.4.0.525)
/// `NavMeshInfo` method: assigns the info's `pNavMesh` to the
/// `NiPointer<NavMesh>` at `result` (`00690890`); returns whether `pNavMesh`
/// is non-null.
pub fn fn_00690860(e: &mut Engine, this: Ptr, result: Ptr) -> bool {
    let mesh = e.mem.u32(this.addr() + INFO_NAV_MESH);
    fn_00690890(e, result, Ptr::new(mesh));
    e.mem.u32(this.addr() + INFO_NAV_MESH) != 0
}

// Translated from 00690890 (decompiled, FalloutNV.exe 1.4.0.525)
/// Assigns the raw navmesh pointer `mesh` to the `NiPointer` at `result`,
/// through a temporary smart pointer (cdecl; the temporary's unwinding frame
/// is not translated).
pub fn fn_00690890(e: &mut Engine, result: Ptr, mesh: Ptr) {
    e.with_stack(8, |e, temp| {
        e.call(POINTER_CONSTRUCT_FROM, &args![temp, mesh]);
        e.call(POINTER_ASSIGN, &args![result, temp]);
        e.call(POINTER_DESTRUCT, &args![temp]);
    });
}

// Translated from 006908f0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Checks the mesh against the four neighbouring exterior cells: for every
/// navmesh the cell to the west, east, south and north holds, asks
/// `005d4a40` (a debug stub that always accepts) with the mesh, the
/// neighbour, a direction flag (1 for west/east, 0 for south/north) and the
/// border coordinate (the west or south border as `float`, the east or north
/// border 4096 further); false if the check rejects any. Interior cells
/// always pass. Cells without a navmesh array are skipped.
pub fn fn_006908f0(e: &mut Engine, this: Ptr<NavMesh>) -> bool {
    let cell = e.call(GET_PARENT_CELL, &args![this]).u32();
    if e.call(CELL_IS_INTERIOR, &args![cell]).bool() {
        return true;
    }
    let mut ok = true;
    let cell = e.call(GET_PARENT_CELL, &args![this]).u32();
    let x = e.call(CELL_GET_X, &args![cell]).i32();
    let cell = e.call(GET_PARENT_CELL, &args![this]).u32();
    let y = e.call(CELL_GET_Y, &args![cell]).i32();
    let cell_width: f64 = e.global(CELL_WIDTH);

    // (x offset, y offset, direction flag, far border).
    let neighbours: [(i32, i32, u32, bool); 4] = [
        (-1, 0, 1, false),
        (1, 0, 1, true),
        (0, -1, 0, false),
        (0, 1, 0, true),
    ];
    for (dx, dy, horizontal, far) in neighbours {
        let origin = if horizontal == 1 { x } else { y };
        let mut border = (origin << 12) as f32;
        if far {
            border = (((origin << 12) as f64) + cell_width) as f32;
        }
        let (x_coordinate, y_coordinate) = if horizontal == 1 {
            (border, 0.0f32)
        } else {
            (0.0f32, border)
        };
        let own_cell = e.call(GET_PARENT_CELL, &args![this]).u32();
        let world_space = e.call(CELL_GET_WORLD_SPACE, &args![own_cell]).u32();
        let neighbour_cell = e
            .call(WORLD_SPACE_GET_CELL, &args![world_space, x + dx, y + dy])
            .u32();
        if neighbour_cell == 0 {
            continue;
        }
        if e.call(CELL_GET_NAV_MESHES, &args![neighbour_cell]).u32() == 0 {
            continue;
        }
        let mut n = 0u32;
        loop {
            let array = e.call(CELL_GET_NAV_MESHES, &args![neighbour_cell]).u32();
            let count = e.call(NAV_MESH_ARRAY_SIZE, &args![array]).u32();
            if n >= count {
                break;
            }
            let array = e.call(CELL_GET_NAV_MESHES, &args![neighbour_cell]).u32();
            let rejected = e.with_stack(8, |e, holder| {
                let held = e.call(NAV_MESH_ARRAY_GET, &args![array, holder, n]).u32();
                let neighbour_mesh = e.call(POINTER_DEREFERENCE, &args![held]).u32();
                let accepted = e
                    .call(
                        NEIGHBOUR_CHECK,
                        &args![this, neighbour_mesh, horizontal, x_coordinate, y_coordinate],
                    )
                    .bool();
                e.call(POINTER_DESTRUCT, &args![held]);
                !accepted
            });
            if rejected {
                ok = false;
            }
            n += 1;
        }
    }
    ok
}

// Translated from 00690d50 (decompiled, FalloutNV.exe 1.4.0.525)
/// `NavMesh::ClearNavMesh` (Xbox PDB): empties the vertex, triangle,
/// extra-info, cover, door-portal and obstacle arrays and the grid, and, if
/// the mesh has a `NavMeshInfo`, deletes that info from the `TES` navmesh
/// info map by the mesh's form id. The word argument is not read.
pub fn nav_mesh_clear_nav_mesh(e: &mut Engine, this: Ptr<NavMesh>, _unused_1: u32) {
    for array in [
        this.at(NavMesh::Vertices),
        this.at(NavMesh::Triangles),
        this.at(NavMesh::ExtraEdgeInfo),
    ] {
        e.call(ARRAY_CLEAR, &args![array, 1u32]);
    }
    e.call(GRID_CLEAR, &args![this.byte_add(MESH_GRID_OFFSET)]);
    e.call(ARRAY_CLEAR, &args![this.at(NavMesh::CoverArray), 1u32]);
    e.call(OBSTACLES_CLEAR, &args![this.at(NavMesh::Obstacles), 1u32]);
    e.call(ARRAY_CLEAR, &args![this.at(NavMesh::DoorPortals), 1u32]);
    if !e.get(this, NavMesh::pNavMeshInfo).is_null() {
        let form_id = e.call(GET_FORM_ID, &args![this]).u32();
        let tes: u32 = e.global(TES_INSTANCE);
        let map = e.call(GET_NAV_MESH_INFO_MAP, &args![tes]).u32();
        e.call(DELETE_NAV_MESH_INFO, &args![map, form_id]);
    }
}

// Translated from 00690df0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `NavMeshTriangle` method: stores vertex index `value` at
/// `Vertices[index]`.
pub fn fn_00690df0(e: &mut Engine, this: Ptr<NavMeshTriangle>, index: u32, value: u16) {
    e.mem
        .set_u16(this.addr().wrapping_add(index.wrapping_mul(2)), value);
}

// Translated from 00690e10 (decompiled, FalloutNV.exe 1.4.0.525)
/// `NavMeshTriangle` method: whether any edge has extra info (flags 0 to 2).
pub fn fn_00690e10(e: &mut Engine, this: Ptr<NavMeshTriangle>) -> bool {
    e.get(this, NavMeshTriangle::TriangleFlags) & EDGE_INFO_FLAGS != 0
}

// Translated from 00690e30 (decompiled, FalloutNV.exe 1.4.0.525)
/// `NavMeshTriangle` method: sets edge info flag `edge` and stores `value`
/// (an `ExtraEdgeInfo` index) in the neighbour slot `Triangles[edge]`.
pub fn fn_00690e30(e: &mut Engine, this: Ptr<NavMeshTriangle>, edge: u32, value: u16) {
    let flags = e.get(this, NavMeshTriangle::TriangleFlags);
    e.set(
        this,
        NavMeshTriangle::TriangleFlags,
        (1u32 << (edge & 31)) | flags,
    );
    e.mem.set_u16(neighbour_slot(this, edge as i32), value);
}

// Translated from 00690e70 (decompiled, FalloutNV.exe 1.4.0.525)
/// `NavMeshTriangle` method: whether flag bit `0x400` is set.
pub fn fn_00690e70(e: &mut Engine, this: Ptr<NavMeshTriangle>) -> bool {
    e.get(this, NavMeshTriangle::TriangleFlags) & TRIANGLE_FLAG_0X400 != 0
}

// Translated from 00690e90 (decompiled, FalloutNV.exe 1.4.0.525)
/// `NavMesh::DeleteExtraEdgeInfo` (Xbox PDB): removes `ExtraEdgeInfo[index]`
/// (the array moves its last element into the gap) and repairs the
/// triangles: an edge that used `index` loses its extra info flag, and an
/// edge that used the moved last element now points at `index`.
pub fn nav_mesh_delete_extra_edge_info(e: &mut Engine, this: Ptr<NavMesh>, index: u16) {
    let infos = this.at(NavMesh::ExtraEdgeInfo);
    e.call(ARRAY_REMOVE_12, &args![infos, index, 1u32]);
    let triangles = this.at(NavMesh::Triangles);
    let count = array_size(e, triangles);
    let mut t = 0u32;
    while t < count {
        let tri = triangle_element(e, this, t);
        if fn_00690e10(e, tri) {
            let mut edge = 0u32;
            while edge < 3 {
                let tri = triangle_element(e, this, t);
                if fn_0068f1d0(e, tri, edge) {
                    let tri = triangle_element(e, this, t);
                    if fn_0068f2c0(e, tri, edge as i32) == index {
                        let tri = triangle_element(e, this, t);
                        fn_00690790(e, tri, edge);
                    } else {
                        let tri = triangle_element(e, this, t);
                        let slot = fn_0068f2c0(e, tri, edge as i32);
                        let size = array_size(e, infos);
                        if slot as u32 == size {
                            let tri = triangle_element(e, this, t);
                            fn_00690e30(e, tri, edge, index);
                        }
                    }
                }
                edge += 1;
            }
        }
        t += 1;
    }
}

// Translated from 00690fe0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Appends a copy of the 12-byte vertex at `vertex` to the mesh's vertex
/// array; returns the array's result (the new vertex's index).
pub fn fn_00690fe0(e: &mut Engine, this: Ptr<NavMesh>, vertex: Ptr) -> u32 {
    let vertices = this.at(NavMesh::Vertices);
    e.call(ARRAY_ADD_12, &args![vertices, vertex]).u32()
}

// Translated from 00691000 (decompiled, FalloutNV.exe 1.4.0.525)
/// Appends a copy of the 16-byte triangle at `triangle` to the mesh's
/// triangle array; returns the array's result (the new triangle's index).
pub fn fn_00691000(e: &mut Engine, this: Ptr<NavMesh>, triangle: Ptr) -> u32 {
    let triangles = this.at(NavMesh::Triangles);
    e.call(ARRAY_ADD_16, &args![triangles, triangle]).u32()
}

/// This unit's translated functions, by exe address.
pub fn funcs() -> Vec<(u32, AbiFn)> {
    vec![
        entry!(
            0x0049c4c0,
            ni_t_map_base_key_to_hash_index(Ptr<NiTPointerMap>, u16) -> u32
        ),
        entry!(0x0068eb80, nav_mesh_nav_mesh(Ptr<NavMesh>) -> Ptr<NavMesh>),
        entry!(0x0068ed30, fn_0068ed30(Ptr<NavMesh>, u32) -> Ptr<NavMesh>),
        entry!(0x0068ed70, fn_0068ed70(Ptr<NavMesh>)),
        entry!(0x0068ef50, fn_0068ef50(Ptr) -> u32),
        entry!(0x0068ef70, nav_mesh_get_parent_space(Ptr<NavMesh>) -> Ptr),
        entry!(0x0068efb0, fn_0068efb0(Ptr<NavMesh>) -> Ptr),
        entry!(0x0068efd0, fn_0068efd0(Ptr<NavMesh>, Ptr, u16, i32) -> Ptr),
        entry!(
            0x0068f040,
            nav_mesh_get_edge_vertices(Ptr<NavMesh>, Ptr, u16, i32) -> Ptr
        ),
        entry!(0x0068f0a0, fn_0068f0a0(Ptr<NavMesh>, u16) -> Ptr),
        entry!(
            0x0068f0c0,
            fn_0068f0c0(Ptr<NavMesh>, u16, i32, Ptr, Ptr) -> bool
        ),
        entry!(0x0068f160, fn_0068f160(Ptr<NavMesh>, u16, Ptr, Ptr, Ptr)),
        entry!(0x0068f1d0, fn_0068f1d0(Ptr<NavMeshTriangle>, u32) -> bool),
        entry!(0x0068f200, fn_0068f200(Ptr<NavMeshTriangle>, i32) -> bool),
        entry!(
            0x0068f230,
            nav_mesh_get_edge_extra_info(Ptr<NavMesh>, u16, i32) -> Ptr<EdgeExtraInfo>
        ),
        entry!(0x0068f2c0, fn_0068f2c0(Ptr<NavMeshTriangle>, i32) -> u16),
        entry!(0x0068f2e0, fn_0068f2e0(Ptr, Ptr) -> bool),
        entry!(0x0068f320, fn_0068f320(Ptr) -> bool),
        entry!(
            0x0068f340,
            nav_mesh_get_matching_edge(Ptr<NavMesh>, Ptr, Ptr) -> bool
        ),
        entry!(
            0x0068f3b0,
            nav_mesh_get_matching_edge_ov2(Ptr<NavMesh>, u16, i32, Ptr, Ptr, Ptr) -> bool
        ),
        entry!(
            0x0068f460,
            nav_mesh_get_matching_edge_ov3(Ptr<NavMesh>, u16, i32, Ptr, Ptr, Ptr) -> bool
        ),
        entry!(
            0x0068f670,
            nav_mesh_get_matching_edge_ov4(Ptr<NavMesh>, Ptr, Ptr) -> bool
        ),
        entry!(0x0068f6b0, fn_0068f6b0(Ptr<NavMesh>, u16) -> u32),
        entry!(
            0x0068f6d0,
            nav_mesh_check_nav_mesh(Ptr<NavMesh>, u32) -> bool
        ),
        entry!(0x00690770, fn_00690770(Ptr<NavMeshTriangle>) -> bool),
        entry!(0x00690790, fn_00690790(Ptr<NavMeshTriangle>, u32)),
        entry!(0x006907d0, fn_006907d0(Ptr<NavMeshTriangle>, u32, u16)),
        entry!(0x00690800, fn_00690800(Ptr) -> Ptr),
        entry!(0x00690830, fn_00690830(Ptr) -> u32),
        entry!(0x00690860, fn_00690860(Ptr, Ptr) -> bool),
        entry!(0x00690890, fn_00690890(Ptr, Ptr)),
        entry!(0x006908f0, fn_006908f0(Ptr<NavMesh>) -> bool),
        entry!(0x00690d50, nav_mesh_clear_nav_mesh(Ptr<NavMesh>, u32)),
        entry!(0x00690df0, fn_00690df0(Ptr<NavMeshTriangle>, u32, u16)),
        entry!(0x00690e10, fn_00690e10(Ptr<NavMeshTriangle>) -> bool),
        entry!(0x00690e30, fn_00690e30(Ptr<NavMeshTriangle>, u32, u16)),
        entry!(0x00690e70, fn_00690e70(Ptr<NavMeshTriangle>) -> bool),
        entry!(
            0x00690e90,
            nav_mesh_delete_extra_edge_info(Ptr<NavMesh>, u16)
        ),
        entry!(0x00690fe0, fn_00690fe0(Ptr<NavMesh>, Ptr) -> u32),
        entry!(0x00691000, fn_00691000(Ptr<NavMesh>, Ptr) -> u32),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    type CallLog = Vec<(u32, Vec<u32>)>;

    fn result(value: u32) -> Ret {
        Ret {
            eax: value,
            ..Ret::default()
        }
    }

    /// Registers `do nothing, return 0` doubles.
    fn noops(e: &mut Engine, addresses: &[u32]) {
        for address in addresses {
            e.register(*address, |_, _| Ret::default());
        }
    }

    /// The arguments of every call to `address` in the log.
    fn calls_to(log: &CallLog, address: u32) -> Vec<Vec<u32>> {
        log.iter()
            .filter(|(a, _)| *a == address)
            .map(|(_, args)| args.clone())
            .collect()
    }

    /// An engine whose array and triangle getters read the real memory
    /// layout of a `BSSimpleArray` (buffer +4, size +8), the way the game's
    /// own getters do.
    fn engine() -> Engine {
        let mut e = Engine::new();
        e.register(ARRAY_SIZE, |e, a| result(e.mem.u32(a[0] + 8)));
        e.register(ELEMENT_12, |e, a| result(e.mem.u32(a[0] + 4) + a[1] * 12));
        e.register(TRIANGLE_ELEMENT, |e, a| {
            result(e.mem.u32(a[0] + 4) + a[1] * 16)
        });
        e.register(NAV_MESH_GET_TRIANGLE, |e, a| {
            result(e.mem.u32(a[0] + 0x38 + 4) + (a[1] & 0xffff) * 16)
        });
        e.register(TRIANGLE_VERTEX, |e, a| {
            result(e.mem.u16(a[0] + 2 * a[1]) as u32)
        });
        e.register(NAV_MESH_TRIANGLE_COUNT, |e, a| {
            result(e.mem.u32(a[0] + 0x38 + 8))
        });
        e
    }

    fn put_array(e: &mut Engine, array: Ptr<BSSimpleArray>, data: &[u8], count: u32) {
        let buffer = e.mem.alloc(data.len().max(8) as u32);
        e.mem.write(buffer, data);
        e.mem.set_u32(array.addr() + 4, buffer);
        e.mem.set_u32(array.addr() + 8, count);
    }

    /// A triangle record: vertex indexes, neighbour slots, flags.
    fn triangle_bytes(vertices: [u16; 3], neighbours: [u16; 3], flags: u32) -> Vec<u8> {
        let mut bytes = Vec::new();
        for word in vertices.iter().chain(neighbours.iter()) {
            bytes.extend_from_slice(&word.to_le_bytes());
        }
        bytes.extend_from_slice(&flags.to_le_bytes());
        bytes
    }

    fn vertex_bytes(vertices: &[[f32; 3]]) -> Vec<u8> {
        vertices
            .iter()
            .flat_map(|v| v.iter().flat_map(|c| c.to_le_bytes()))
            .collect()
    }

    type TriangleSpec = ([u16; 3], [u16; 3], u32);

    /// A mesh with the given vertices and `(vertices, neighbours, flags)`
    /// triangles.
    fn mesh_with(
        e: &mut Engine,
        vertices: &[[f32; 3]],
        triangles: &[TriangleSpec],
    ) -> Ptr<NavMesh> {
        let mesh: Ptr<NavMesh> = e.new_object();
        put_array(
            e,
            mesh.at(NavMesh::Vertices),
            &vertex_bytes(vertices),
            vertices.len() as u32,
        );
        let bytes: Vec<u8> = triangles
            .iter()
            .flat_map(|(v, n, f)| triangle_bytes(*v, *n, *f))
            .collect();
        put_array(
            e,
            mesh.at(NavMesh::Triangles),
            &bytes,
            triangles.len() as u32,
        );
        mesh
    }

    fn triangle_ptr(e: &Engine, mesh: Ptr<NavMesh>, index: u32) -> Ptr<NavMeshTriangle> {
        let base = e.mem.u32(mesh.addr() + 0x38 + 4);
        Ptr::new(base + index * 16)
    }

    /// Two triangles `(0, 1, 2)` and `(1, 3, 2)` sharing the edge 1-2, the
    /// first one's edge 1 is the second one's edge 2.
    fn two_triangles(e: &mut Engine) -> Ptr<NavMesh> {
        mesh_with(
            e,
            &[
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
                [1.0, 1.0, 0.0],
            ],
            &[
                ([0, 1, 2], [0xffff, 1, 0xffff], 0),
                ([1, 3, 2], [0xffff, 0xffff, 0], 0),
            ],
        )
    }

    #[test]
    fn key_to_hash_index_is_the_key_modulo_the_bucket_count() {
        let mut e = Engine::new();
        let map: Ptr<NiTPointerMap> = e.new_object();
        e.set(map, NiTPointerMap::m_uiHashSize, 37);
        assert_eq!(e.call(0x0049_c4c0, &args![map, 100u16]).u32(), 100 % 37);
        assert_eq!(
            e.call(0x0049_c4c0, &args![map, 0xffffu16]).u32(),
            0xffff % 37
        );
    }

    /// Doubles for everything the constructor and destructor call.
    fn member_doubles(e: &mut Engine) {
        noops(
            e,
            &[
                TES_FORM_CONSTRUCTOR,
                TES_FORM_DESTRUCTOR,
                CHILD_CELL_CONSTRUCTOR,
                REF_OBJECT_CONSTRUCTOR,
                REF_OBJECT_DESTRUCTOR,
                ARRAY_CONSTRUCTOR_VERTICES,
                ARRAY_DESTRUCTOR_VERTICES,
                ARRAY_CONSTRUCTOR_TRIANGLES,
                ARRAY_DESTRUCTOR_TRIANGLES,
                ARRAY_CONSTRUCTOR_EXTRA_INFO,
                ARRAY_DESTRUCTOR_EXTRA_INFO,
                ARRAY_CONSTRUCTOR_DOOR_PORTALS,
                ARRAY_DESTRUCTOR_DOOR_PORTALS,
                ARRAY_CONSTRUCTOR_CLOSED_DOORS,
                ARRAY_DESTRUCTOR_CLOSED_DOORS,
                ARRAY_CONSTRUCTOR_U16,
                ARRAY_DESTRUCTOR_U16,
                MAP_CONSTRUCTOR,
                MAP_DESTRUCTOR,
                GRID_CONSTRUCTOR,
                GRID_CLEAR,
                OBSTACLES_CONSTRUCTOR,
                OBSTACLES_DESTRUCTOR,
                AVOID_NODES_CONSTRUCTOR,
                AVOID_NODES_DESTRUCTOR,
                SET_FORM_TYPE,
                ARRAY_CLEAR,
                INFO_SET_NAV_MESH,
                FREE_BLOCK,
            ],
        );
    }

    #[test]
    fn constructor_builds_the_bases_and_members() {
        let mut e = Engine::new();
        member_doubles(&mut e);
        e.register(OPERATOR_NEW, |e, a| result(e.mem.alloc(a[0])));
        e.register(OBSTACLE_MAP_CONSTRUCTOR, |_, a| result(a[0]));
        let mesh: Ptr<NavMesh> = e.new_object();
        e.set(mesh, NavMesh::pParentCell, Ptr::new(0x77));
        e.set(mesh, NavMesh::pNavMeshInfo, Ptr::new(0x88));
        e.call_log = Some(vec![]);
        let back = e.call(0x0068_eb80, &args![mesh]).ptr::<NavMesh>();
        assert_eq!(back, mesh);
        let log = e.call_log.take().unwrap();
        assert_eq!(e.mem.u32(mesh.addr()), NAV_MESH_VTABLE);
        assert_eq!(e.mem.u32(mesh.addr() + 0x18), CHILD_CELL_VTABLE);
        assert_eq!(e.mem.u32(mesh.addr() + 0x1c), REF_OBJECT_VTABLE);
        assert!(e.get(mesh, NavMesh::pParentCell).is_null());
        assert!(e.get(mesh, NavMesh::pNavMeshInfo).is_null());
        // The form type 0x43 is stored; the obstacle map is the 16-byte block.
        assert_eq!(calls_to(&log, SET_FORM_TYPE), vec![vec![mesh.addr(), 0x43]]);
        let map = e.get(mesh, NavMesh::pTriangleToObstacleMap);
        assert!(!map.is_null());
        assert_eq!(calls_to(&log, OPERATOR_NEW), vec![vec![0x10]]);
        assert_eq!(
            calls_to(&log, OBSTACLE_MAP_CONSTRUCTOR),
            vec![vec![map.addr(), 0x25]]
        );
        // Bases first (form, child cell, ref object), then the vertex array.
        let order: Vec<u32> = log.iter().map(|(a, _)| *a).collect();
        assert_eq!(
            &order[..5],
            &[
                0x0068_eb80,
                TES_FORM_CONSTRUCTOR,
                CHILD_CELL_CONSTRUCTOR,
                REF_OBJECT_CONSTRUCTOR,
                ARRAY_CONSTRUCTOR_VERTICES
            ]
        );
        assert_eq!(
            calls_to(&log, MAP_CONSTRUCTOR),
            vec![vec![mesh.addr() + 0x88, 0x25]]
        );
    }

    #[test]
    fn constructor_leaves_the_map_null_when_allocation_fails() {
        let mut e = Engine::new();
        member_doubles(&mut e);
        e.register(OPERATOR_NEW, |_, _| result(0));
        let mesh: Ptr<NavMesh> = e.new_object();
        e.set(mesh, NavMesh::pTriangleToObstacleMap, Ptr::new(0x99));
        e.call_log = Some(vec![]);
        e.call(0x0068_eb80, &args![mesh]);
        let log = e.call_log.take().unwrap();
        assert!(e.get(mesh, NavMesh::pTriangleToObstacleMap).is_null());
        assert!(calls_to(&log, OBSTACLE_MAP_CONSTRUCTOR).is_empty());
    }

    #[test]
    fn scalar_deleting_destructor_frees_only_when_asked() {
        let mut e = Engine::new();
        member_doubles(&mut e);
        let mesh: Ptr<NavMesh> = e.new_object();
        e.call_log = Some(vec![]);
        assert_eq!(
            e.call(0x0068_ed30, &args![mesh, 1u32]).ptr::<NavMesh>(),
            mesh
        );
        let log = e.call_log.take().unwrap();
        assert_eq!(calls_to(&log, FREE_BLOCK), vec![vec![mesh.addr(), 0x108]]);
        assert_eq!(calls_to(&log, TES_FORM_DESTRUCTOR).len(), 1);
        e.call_log = Some(vec![]);
        e.call(0x0068_ed30, &args![mesh, 0u32]);
        let log = e.call_log.take().unwrap();
        assert!(calls_to(&log, FREE_BLOCK).is_empty());
        assert_eq!(calls_to(&log, TES_FORM_DESTRUCTOR).len(), 1);
    }

    #[test]
    fn destructor_detaches_from_the_info_and_deletes_the_map() {
        let mut e = Engine::new();
        member_doubles(&mut e);
        let mesh: Ptr<NavMesh> = e.new_object();
        let info = Ptr::<()>::new(e.mem.alloc(0x60));
        e.set(mesh, NavMesh::pNavMeshInfo, info);
        // A map object whose virtual destructor (slot 0) is a double.
        e.put_vtable(0x0e10_0000, &[0x00f0_0100]);
        e.register(0x00f0_0100, |_, _| Ret::default());
        let map = e.mem.alloc(16);
        e.mem.set_u32(map, 0x0e10_0000);
        e.set(mesh, NavMesh::pTriangleToObstacleMap, Ptr::new(map));
        e.call_log = Some(vec![]);
        e.call(0x0068_ed70, &args![mesh]);
        let log = e.call_log.take().unwrap();
        assert_eq!(e.mem.u32(mesh.addr()), NAV_MESH_VTABLE);
        assert_eq!(
            calls_to(&log, INFO_SET_NAV_MESH),
            vec![vec![info.addr(), 0]]
        );
        assert_eq!(calls_to(&log, 0x00f0_0100), vec![vec![map, 1]]);
        // The five arrays emptied first, members destroyed last to first,
        // the form last.
        assert_eq!(calls_to(&log, ARRAY_CLEAR).len(), 5);
        let order: Vec<u32> = log.iter().map(|(a, _)| *a).collect();
        let position = |address: u32| order.iter().position(|a| *a == address).unwrap();
        assert!(position(AVOID_NODES_DESTRUCTOR) < position(OBSTACLES_DESTRUCTOR));
        assert!(position(ARRAY_DESTRUCTOR_TRIANGLES) < position(ARRAY_DESTRUCTOR_VERTICES));
        assert_eq!(*order.last().unwrap(), TES_FORM_DESTRUCTOR);
    }

    #[test]
    fn destructor_without_info_or_map_skips_them() {
        let mut e = Engine::new();
        member_doubles(&mut e);
        let mesh: Ptr<NavMesh> = e.new_object();
        e.call_log = Some(vec![]);
        e.call(0x0068_ed70, &args![mesh]);
        let log = e.call_log.take().unwrap();
        assert!(calls_to(&log, INFO_SET_NAV_MESH).is_empty());
    }

    #[test]
    fn child_cell_thunk_passes_the_navmesh() {
        let mut e = Engine::new();
        e.register(GET_PARENT_CELL, |_, a| result(a[0] + 1));
        // `this` is the TESChildCell base at +0x18.
        assert_eq!(e.call(0x0068_ef50, &args![0x1018u32]).u32(), 0x1001);
    }

    #[test]
    fn parent_space_prefers_the_worldspace() {
        let mut e = Engine::new();
        e.register(CELL_GET_WORLD_SPACE, |e, a| result(e.mem.u32(a[0] + 0x40)));
        let mesh: Ptr<NavMesh> = e.new_object();
        assert!(e.call(0x0068_ef70, &args![mesh]).ptr::<()>().is_null());
        let cell = e.mem.alloc(0x50);
        e.set(mesh, NavMesh::pParentCell, Ptr::new(cell));
        assert_eq!(e.call(0x0068_ef70, &args![mesh]).u32(), cell);
        e.mem.set_u32(cell + 0x40, 0xabc0);
        assert_eq!(e.call(0x0068_ef70, &args![mesh]).u32(), 0xabc0);
    }

    #[test]
    fn info_getter_returns_the_navmesh_info() {
        let mut e = Engine::new();
        let mesh: Ptr<NavMesh> = e.new_object();
        e.set(mesh, NavMesh::pNavMeshInfo, Ptr::new(0x1234));
        assert_eq!(e.call(0x0068_efb0, &args![mesh]).u32(), 0x1234);
    }

    #[test]
    fn edge_vertex_indexes_wrap_around_the_triangle() {
        let mut e = engine();
        let mesh = mesh_with(&mut e, &[], &[([10, 20, 30], [0xffff; 3], 0)]);
        let out = Ptr::<()>::new(e.mem.alloc(8));
        assert_eq!(
            e.call(0x0068_efd0, &args![mesh, out, 0u16, 2i32])
                .ptr::<()>(),
            out
        );
        // Edge 2 runs from vertex 30 back to vertex 10.
        assert_eq!(e.mem.u16(out.addr()), 30);
        assert_eq!(e.mem.u16(out.addr() + 2), 10);
        e.call(0x0068_efd0, &args![mesh, out, 0u16, 0i32]);
        assert_eq!((e.mem.u16(out.addr()), e.mem.u16(out.addr() + 2)), (10, 20));
    }

    #[test]
    fn edge_vertices_are_the_addresses_of_the_vertices() {
        let mut e = engine();
        let mesh = two_triangles(&mut e);
        let out = Ptr::<()>::new(e.mem.alloc(8));
        let back = e
            .call(0x0068_f040, &args![mesh, out, 1u16, 1i32])
            .ptr::<()>();
        assert_eq!(back, out);
        let base = e.mem.u32(mesh.addr() + 0x28 + 4);
        // Triangle 1 is (1, 3, 2): edge 1 runs from vertex 3 to vertex 2.
        assert_eq!(e.mem.u32(out.addr()), base + 3 * 12);
        assert_eq!(e.mem.u32(out.addr() + 4), base + 2 * 12);
    }

    #[test]
    fn vertex_getter_indexes_the_vertex_array() {
        let mut e = engine();
        let mesh = two_triangles(&mut e);
        let base = e.mem.u32(mesh.addr() + 0x28 + 4);
        assert_eq!(e.call(0x0068_f0a0, &args![mesh, 2u16]).u32(), base + 24);
    }

    #[test]
    fn edge_coordinates_are_copied() {
        let mut e = engine();
        let mesh = two_triangles(&mut e);
        let first = Ptr::<()>::new(e.mem.alloc(12));
        let second = Ptr::<()>::new(e.mem.alloc(12));
        // Triangle 0, edge 1: vertex 1 (1, 0, 0) to vertex 2 (0, 1, 0).
        assert!(e
            .call(0x0068_f0c0, &args![mesh, 0u16, 1i32, first, second])
            .bool());
        assert_eq!(
            [e.mem.f32(first.addr()), e.mem.f32(first.addr() + 4)],
            [1.0, 0.0]
        );
        assert_eq!(
            [e.mem.f32(second.addr()), e.mem.f32(second.addr() + 4)],
            [0.0, 1.0]
        );
    }

    #[test]
    fn triangle_vertex_addresses_are_stored() {
        let mut e = engine();
        let mesh = two_triangles(&mut e);
        let outs: Vec<Ptr> = (0..3).map(|_| Ptr::new(e.mem.alloc(4))).collect();
        e.call(0x0068_f160, &args![mesh, 1u16, outs[0], outs[1], outs[2]]);
        let base = e.mem.u32(mesh.addr() + 0x28 + 4);
        let stored: Vec<u32> = outs.iter().map(|p| e.mem.u32(p.addr())).collect();
        assert_eq!(stored, vec![base + 12, base + 36, base + 24]);
    }

    #[test]
    fn triangle_flag_tests() {
        let mut e = Engine::new();
        let tri: Ptr<NavMeshTriangle> = e.new_object();
        e.set(tri, NavMeshTriangle::TriangleFlags, 0b101);
        assert!(e.call(0x0068_f1d0, &args![tri, 0u32]).bool());
        assert!(!e.call(0x0068_f1d0, &args![tri, 1u32]).bool());
        assert!(e.call(0x0068_f1d0, &args![tri, 2u32]).bool());
        // Only the low five bits of the edge number count.
        assert!(e.call(0x0068_f1d0, &args![tri, 32u32]).bool());
    }

    #[test]
    fn neighbour_tests_and_getter() {
        let mut e = Engine::new();
        let tri: Ptr<NavMeshTriangle> = e.new_object();
        e.mem.write(tri.addr() + 6, &[7, 0, 0xff, 0xff, 9, 0]);
        assert!(e.call(0x0068_f200, &args![tri, 0i32]).bool());
        assert!(!e.call(0x0068_f200, &args![tri, 1i32]).bool());
        assert_eq!(e.call(0x0068_f2c0, &args![tri, 2i32]).u16(), 9);
        assert_eq!(e.call(0x0068_f2c0, &args![tri, 1i32]).u16(), 0xffff);
    }

    #[test]
    fn edge_extra_info_needs_the_flag_and_a_neighbour_slot() {
        let mut e = engine();
        let mesh = mesh_with(
            &mut e,
            &[],
            &[
                ([0, 1, 2], [1, 0xffff, 0xffff], 0b001),
                ([0, 1, 2], [0xffff, 0xffff, 0xffff], 0b010),
                ([0, 1, 2], [0xffff, 0xffff, 0], 0),
            ],
        );
        put_array(&mut e, mesh.at(NavMesh::ExtraEdgeInfo), &[0u8; 36], 3);
        let infos = e.mem.u32(mesh.addr() + 0x48 + 4);
        // Flag set, slot 1: the second info record.
        assert_eq!(
            e.call(0x0068_f230, &args![mesh, 0u16, 0i32]).u32(),
            infos + 12
        );
        // Flag set but slot 0xFFFF.
        assert_eq!(e.call(0x0068_f230, &args![mesh, 1u16, 1i32]).u32(), 0);
        // Flag clear.
        assert_eq!(e.call(0x0068_f230, &args![mesh, 2u16, 2i32]).u32(), 0);
    }

    #[test]
    fn info_navmesh_getter_honours_the_flag() {
        let mut e = Engine::new();
        let info = Ptr::<()>::new(e.mem.alloc(0x60));
        let out = Ptr::<()>::new(e.mem.alloc(4));
        e.mem.set_u32(out.addr(), 0xdead);
        e.mem.set_u32(info.addr() + 0x54, 0x4321);
        assert!(e.call(0x0068_f2e0, &args![info, out]).bool());
        assert_eq!(e.mem.u32(out.addr()), 0x4321);
        // A null mesh: stored, but false.
        e.mem.set_u32(info.addr() + 0x54, 0);
        assert!(!e.call(0x0068_f2e0, &args![info, out]).bool());
        // Flagged: null and false even with a mesh.
        e.mem.set_u32(info.addr() + 0x54, 0x4321);
        e.mem.set_u32(info.addr() + 8, 0x10);
        assert!(!e.call(0x0068_f2e0, &args![info, out]).bool());
        assert_eq!(e.mem.u32(out.addr()), 0);
    }

    #[test]
    fn info_flag_bit_0x10() {
        let mut e = Engine::new();
        let info = Ptr::<()>::new(e.mem.alloc(0x60));
        assert!(!e.call(0x0068_f320, &args![info]).bool());
        e.mem.set_u32(info.addr() + 8, 0x10);
        assert!(e.call(0x0068_f320, &args![info]).bool());
        e.mem.set_u32(info.addr() + 8, 0xffff_ffef);
        assert!(!e.call(0x0068_f320, &args![info]).bool());
    }

    fn matching_edge_engine() -> (Engine, Ptr<NavMesh>) {
        let mut e = engine();
        let mesh = two_triangles(&mut e);
        let info = e.mem.alloc(0x60);
        e.set(mesh, NavMesh::pNavMeshInfo, Ptr::new(info));
        (e, mesh)
    }

    #[test]
    fn matching_edge_inside_one_mesh() {
        let (mut e, mesh) = matching_edge_engine();
        let mesh_out = e.mem.alloc(4);
        let triangle_out = e.mem.alloc(4);
        let edge_out = e.mem.alloc(4);
        // Triangle 0 edge 1 mirrors triangle 1 edge 2.
        assert!(e
            .call(
                0x0068_f460,
                &args![mesh, 0u16, 1i32, mesh_out, triangle_out, edge_out]
            )
            .bool());
        assert_eq!(e.mem.u32(mesh_out), mesh.addr());
        assert_eq!(e.mem.u16(triangle_out), 1);
        assert_eq!(e.mem.i32(edge_out), 2);
        // And back.
        assert!(e
            .call(
                0x0068_f460,
                &args![mesh, 1u16, 2i32, mesh_out, triangle_out, edge_out]
            )
            .bool());
        assert_eq!((e.mem.u16(triangle_out), e.mem.i32(edge_out)), (0, 1));
    }

    #[test]
    fn matching_edge_fails_without_a_neighbour_or_return_link() {
        let (mut e, mesh) = matching_edge_engine();
        let outs: Vec<u32> = (0..3).map(|_| e.mem.alloc(4)).collect();
        let call = |e: &mut Engine, triangle: u16, edge: i32| {
            e.call(
                0x0068_f460,
                &args![mesh, triangle, edge, outs[0], outs[1], outs[2]],
            )
            .bool()
        };
        // Triangle index past the end, an edge without a neighbour.
        assert!(!call(&mut e, 2, 0));
        assert!(!call(&mut e, 0, 0));
        // The neighbour does not link back.
        let neighbour = triangle_ptr(&e, mesh, 1);
        e.mem.set_u16(neighbour.addr() + 6 + 4, 0xffff);
        assert!(!call(&mut e, 0, 1));
    }

    type PortalPair = (Ptr<NavMesh>, Ptr<NavMesh>, Ptr, Ptr);

    /// Mesh A (triangle 0, edge 0) has a portal to mesh B (triangle 0,
    /// edge 1) which has one back.
    fn portal_pair(e: &mut Engine) -> PortalPair {
        let a = mesh_with(e, &[], &[([0, 1, 2], [0, 0xffff, 0xffff], 0b001)]);
        let b = mesh_with(e, &[], &[([0, 1, 2], [0xffff, 0, 0xffff], 0b010)]);
        let info_a = Ptr::<()>::new(e.mem.alloc(0x60));
        let info_b = Ptr::<()>::new(e.mem.alloc(0x60));
        e.set(a, NavMesh::pNavMeshInfo, info_a);
        e.set(b, NavMesh::pNavMeshInfo, info_b);
        e.mem.set_u32(info_a.addr() + 0x54, a.addr());
        e.mem.set_u32(info_b.addr() + 0x54, b.addr());
        // A's info record: portal to B's info, triangle 0. B's back to A.
        let mut record_a = vec![0u8; 12];
        record_a[4..8].copy_from_slice(&info_b.addr().to_le_bytes());
        put_array(e, a.at(NavMesh::ExtraEdgeInfo), &record_a, 1);
        let mut record_b = vec![0u8; 12];
        record_b[4..8].copy_from_slice(&info_a.addr().to_le_bytes());
        put_array(e, b.at(NavMesh::ExtraEdgeInfo), &record_b, 1);
        (a, b, info_a, info_b)
    }

    #[test]
    fn matching_edge_through_a_portal() {
        let mut e = engine();
        let (a, b, _, _) = portal_pair(&mut e);
        let outs: Vec<u32> = (0..3).map(|_| e.mem.alloc(4)).collect();
        assert!(e
            .call(
                0x0068_f460,
                &args![a, 0u16, 0i32, outs[0], outs[1], outs[2]]
            )
            .bool());
        assert_eq!(e.mem.u32(outs[0]), b.addr());
        assert_eq!(e.mem.u16(outs[1]), 0);
        assert_eq!(e.mem.i32(outs[2]), 1);
    }

    #[test]
    fn matching_edge_through_a_portal_needs_a_back_portal() {
        let mut e = engine();
        let (a, b, _, info_b) = portal_pair(&mut e);
        let outs: Vec<u32> = (0..3).map(|_| e.mem.alloc(4)).collect();
        // B's portal points to the wrong mesh.
        let record = e.mem.u32(b.addr() + 0x48 + 4);
        e.mem.set_u32(record + 4, info_b.addr());
        assert!(!e
            .call(
                0x0068_f460,
                &args![a, 0u16, 0i32, outs[0], outs[1], outs[2]]
            )
            .bool());
        // A flagged (unloaded) other info ends the search early.
        let record = e.mem.u32(b.addr() + 0x48 + 4);
        let info_a = e.get(a, NavMesh::pNavMeshInfo);
        e.mem.set_u32(record + 4, info_a.addr());
        e.mem.set_u32(info_b.addr() + 8, 0x10);
        assert!(!e
            .call(
                0x0068_f460,
                &args![a, 0u16, 0i32, outs[0], outs[1], outs[2]]
            )
            .bool());
    }

    #[test]
    fn matching_edge_with_a_null_portal_info_fails() {
        let mut e = engine();
        let (a, _, _, _) = portal_pair(&mut e);
        let record = e.mem.u32(a.addr() + 0x48 + 4);
        e.mem.set_u32(record + 4, 0);
        let outs: Vec<u32> = (0..3).map(|_| e.mem.alloc(4)).collect();
        assert!(!e
            .call(
                0x0068_f460,
                &args![a, 0u16, 0i32, outs[0], outs[1], outs[2]]
            )
            .bool());
    }

    #[test]
    fn matching_edge_record_form_stores_the_info() {
        let (mut e, mesh) = matching_edge_engine();
        let edge_ref = e.mem.alloc(12);
        e.mem.set_u16(edge_ref + 4, 0);
        e.mem.set_i32(edge_ref + 8, 1);
        let result_record = e.mem.alloc(12);
        assert!(e
            .call(0x0068_f340, &args![mesh, edge_ref, result_record])
            .bool());
        let info = e.get(mesh, NavMesh::pNavMeshInfo);
        assert_eq!(e.mem.u32(result_record), info.addr());
        assert_eq!(e.mem.u16(result_record + 4), 1);
        assert_eq!(e.mem.i32(result_record + 8), 2);
        // No match: the record is left alone.
        e.mem.set_i32(edge_ref + 8, 0);
        e.mem.set_u32(result_record, 0x55);
        assert!(!e
            .call(0x0068_f340, &args![mesh, edge_ref, result_record])
            .bool());
        assert_eq!(e.mem.u32(result_record), 0x55);
    }

    #[test]
    fn matching_edge_overload_2_hands_the_mesh_to_a_smart_pointer() {
        let (mut e, mesh) = matching_edge_engine();
        e.register(POINTER_CONSTRUCT_FROM_CDECL, |e, a| {
            e.mem.set_u32(a[0], a[1]);
            result(a[0])
        });
        e.register(POINTER_ASSIGN, |e, a| {
            let value = e.mem.u32(a[1]);
            e.mem.set_u32(a[0], value);
            result(a[0])
        });
        e.register(POINTER_DESTRUCT, |_, _| Ret::default());
        let pointer = e.mem.alloc(4);
        let triangle_out = e.mem.alloc(4);
        let edge_out = e.mem.alloc(4);
        assert!(e
            .call(
                0x0068_f3b0,
                &args![mesh, 0u16, 1i32, pointer, triangle_out, edge_out]
            )
            .bool());
        assert_eq!(e.mem.u32(pointer), mesh.addr());
        assert_eq!((e.mem.u16(triangle_out), e.mem.i32(edge_out)), (1, 2));
        // No match: the pointer is untouched.
        e.mem.set_u32(pointer, 0);
        assert!(!e
            .call(
                0x0068_f3b0,
                &args![mesh, 0u16, 0i32, pointer, triangle_out, edge_out]
            )
            .bool());
        assert_eq!(e.mem.u32(pointer), 0);
    }

    #[test]
    fn matching_edge_overload_4_writes_three_words() {
        let (mut e, mesh) = matching_edge_engine();
        let edge_ref = e.mem.alloc(12);
        e.mem.set_u16(edge_ref + 4, 1);
        e.mem.set_i32(edge_ref + 8, 2);
        let out = e.mem.alloc(12);
        assert!(e.call(0x0068_f670, &args![mesh, edge_ref, out]).bool());
        assert_eq!(e.mem.u32(out), mesh.addr());
        assert_eq!((e.mem.u16(out + 4), e.mem.i32(out + 8)), (0, 1));
    }

    #[test]
    fn door_portal_getter_forwards_the_array() {
        let mut e = Engine::new();
        e.register(DOOR_PORTAL_ELEMENT, |_, a| result(a[0] * 1000 + a[1]));
        let mesh: Ptr<NavMesh> = Ptr::new(0x2000);
        assert_eq!(
            e.call(0x0068_f6b0, &args![mesh, 3u16]).u32(),
            (0x2000 + 0x58) * 1000 + 3
        );
    }

    // ---- CheckNavMesh ------------------------------------------------------

    /// Characters the cell text double stores, and the navmesh's form id.
    const CELL_TEXT: u32 = 0xcafe_0001;
    const FORM_ID: u32 = 0x0001_2345;
    const WORLD_SPACE: u32 = 0x0e20_0000;

    /// Doubles for everything `CheckNavMesh` and the functions it runs call.
    fn check_engine() -> Engine {
        let mut e = engine();
        e.register(CELL_IS_INTERIOR, |_, _| result(1));
        noops(
            &mut e,
            &[
                STRING_CONSTRUCT,
                STRING_DESTRUCT,
                DEBUG_PRINT,
                FLIP_TRIANGLE,
                POINTER_CONSTRUCT,
                POINTER_DESTRUCT,
            ],
        );
        e.register(GET_PARENT_CELL, |e, a| result(e.mem.u32(a[0] + 0x24)));
        e.register(POINTER_GET, |e, a| result(e.mem.u32(a[0])));
        e.register(GET_FORM_ID, |e, a| result(e.mem.u32(a[0] + 0xc)));
        e.register(FINITE, |_, a| {
            let value = f64::from_bits(a[0] as u64 | (a[1] as u64) << 32);
            result(value.is_finite() as u32)
        });
        e.register(FLOAT_ABS, |_, a| Ret {
            st0: f32::from_bits(a[0]).abs() as f64,
            ..Ret::default()
        });
        e.register(U16_ELEMENT, |e, a| result(e.mem.u32(a[0] + 4) + a[1] * 2));
        e.register(CELL_GET_WORLD_SPACE, |_, _| result(WORLD_SPACE));
        e.register(RT_DYNAMIC_CAST, |_, a| result(a[0]));
        e.register(POINTER_CONSTRUCT_FROM, |e, a| {
            e.mem.set_u32(a[0], a[1]);
            Ret::default()
        });
        e.register(POINTER_ASSIGN, |e, a| {
            let value = e.mem.u32(a[1]);
            e.mem.set_u32(a[0], value);
            Ret::default()
        });
        e.register(ARRAY_REMOVE_12, |e, a| {
            let size = e.mem.u32(a[0] + 8);
            e.mem.set_u32(a[0] + 8, size - a[2]);
            Ret::default()
        });
        e.map(0x0106_a000, 0x1000);
        e.map(0x0101_2000, 0x1000);
        e.set_global(MAX_COORDINATE, 1.00000000376832e14f64);
        e.set_global(ZERO, 0.0f64);
        // A cell whose detailed-string virtual stores the cell text.
        e.put_vtable(0x0e30_0000, &[0x00f0_0200; 37]);
        e.register(0x00f0_0200, |e, a| {
            e.mem.set_u32(a[1], CELL_TEXT);
            Ret::default()
        });
        e
    }

    /// Gives the mesh its cell and form id, and makes the normals point up
    /// unless `down_triangle` names one.
    fn checked(e: &mut Engine, mesh: Ptr<NavMesh>, down_triangle: Option<u32>) {
        let cell = e.mem.alloc(0x80);
        e.mem.set_u32(cell, 0x0e30_0000);
        e.set(mesh, NavMesh::pParentCell, Ptr::new(cell));
        e.mem.set_u32(mesh.addr() + 0xc, FORM_ID);
        e.register_double(TRIANGLE_NORMAL, move |e, a| {
            let z = if Some(a[2]) == down_triangle {
                -1.0
            } else {
                1.0
            };
            e.mem.set_f32(a[1] + 8, z);
            Ret::default()
        });
    }

    /// Runs the check and returns its result and the debug prints (format,
    /// then the numbers after the form id and cell text).
    fn run_check(e: &mut Engine, mesh: Ptr<NavMesh>) -> (bool, Vec<Vec<u32>>) {
        e.call_log = Some(vec![]);
        let ok = e.call(0x0068_f6d0, &args![mesh, 0u32]).bool();
        let log = e.call_log.take().unwrap();
        let prints = calls_to(&log, DEBUG_PRINT)
            .into_iter()
            .map(|words| {
                assert_eq!(words[1], FORM_ID, "form id");
                assert_eq!(words[2], CELL_TEXT, "cell text");
                let mut report = vec![words[0]];
                report.extend_from_slice(&words[3..]);
                report
            })
            .collect();
        (ok, prints)
    }

    #[test]
    fn check_accepts_a_consistent_mesh() {
        let mut e = check_engine();
        let mesh = two_triangles(&mut e);
        checked(&mut e, mesh, None);
        let (ok, prints) = run_check(&mut e, mesh);
        assert!(ok);
        assert!(prints.is_empty(), "{prints:?}");
    }

    #[test]
    fn check_reports_the_first_bad_vertex_coordinate() {
        let mut e = check_engine();
        let mesh = mesh_with(
            &mut e,
            &[[0.0, 0.0, 0.0], [f32::NAN, 0.0, 0.0], [0.0, 3.0e14, 0.0]],
            &[],
        );
        checked(&mut e, mesh, None);
        let (ok, prints) = run_check(&mut e, mesh);
        assert!(!ok);
        // The NaN stops the vertex scan; the huge y coordinate is not seen.
        assert_eq!(prints, vec![vec![FORMAT_BAD_X]]);
        let mesh = mesh_with(&mut e, &[[0.0, 3.0e14, 0.0]], &[]);
        checked(&mut e, mesh, None);
        assert_eq!(run_check(&mut e, mesh).1, vec![vec![FORMAT_BAD_Y]]);
        let mesh = mesh_with(&mut e, &[[0.0, 0.0, -3.0e14]], &[]);
        checked(&mut e, mesh, None);
        assert_eq!(run_check(&mut e, mesh).1, vec![vec![FORMAT_BAD_Z]]);
    }

    #[test]
    fn check_warns_about_many_triangles() {
        let mut e = check_engine();
        let mesh = mesh_with(&mut e, &[], &[]);
        // The mesh reports 2000 triangles on the second and third size query
        // (the check and the warning); every other query sees an empty array,
        // so the O(n^2) loops stay short. A warning is not a problem.
        let mut queries = 0;
        e.register_double(ARRAY_SIZE, move |_, _| {
            queries += 1;
            result(if queries == 2 || queries == 3 {
                2000
            } else {
                0
            })
        });
        checked(&mut e, mesh, None);
        let (ok, prints) = run_check(&mut e, mesh);
        assert!(ok);
        assert_eq!(prints, vec![vec![FORMAT_MANY_TRIANGLES, 2000]]);
    }

    #[test]
    fn check_clears_a_neighbour_index_past_the_end() {
        let mut e = check_engine();
        let mesh = mesh_with(
            &mut e,
            &[[0.0; 3], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
            &[([0, 1, 2], [0xffff, 7, 0xffff], 0)],
        );
        checked(&mut e, mesh, None);
        let (ok, prints) = run_check(&mut e, mesh);
        assert!(!ok);
        assert_eq!(prints, vec![vec![FORMAT_BAD_TRIANGLE_INDEX, 0, 1]]);
        let tri = triangle_ptr(&e, mesh, 0);
        assert_eq!(e.mem.u16(tri.addr() + 8), 0xffff);
    }

    #[test]
    fn check_flips_a_downfacing_triangle() {
        let mut e = check_engine();
        let mesh = two_triangles(&mut e);
        checked(&mut e, mesh, Some(1));
        e.call_log = Some(vec![]);
        let ok = e.call(0x0068_f6d0, &args![mesh, 0u32]).bool();
        let log = e.call_log.take().unwrap();
        assert!(!ok);
        assert_eq!(
            calls_to(&log, DEBUG_PRINT)
                .iter()
                .map(|w| (w[0], w[3]))
                .collect::<Vec<_>>(),
            vec![(FORMAT_DOWNFACING, 1)]
        );
        assert_eq!(calls_to(&log, FLIP_TRIANGLE), vec![vec![mesh.addr(), 1]]);
    }

    #[test]
    fn check_reports_degenerate_triangles() {
        let mut e = check_engine();
        let mesh = mesh_with(
            &mut e,
            &[[0.0; 3], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
            &[
                ([0, 0, 2], [0xffff; 3], 0),
                ([0, 1, 1], [0xffff; 3], 0),
                ([2, 1, 2], [0xffff; 3], 0),
            ],
        );
        checked(&mut e, mesh, None);
        let (ok, prints) = run_check(&mut e, mesh);
        assert!(!ok);
        assert!(prints.contains(&vec![FORMAT_DEGENERATE_01, 0, 0]));
        assert!(prints.contains(&vec![FORMAT_DEGENERATE_12, 1, 1]));
        assert!(prints.contains(&vec![FORMAT_DEGENERATE_02, 2, 2]));
    }

    #[test]
    fn check_reports_two_edges_to_the_same_neighbour() {
        let mut e = check_engine();
        let mesh = two_triangles(&mut e);
        checked(&mut e, mesh, None);
        // Triangle 0: edges 0 and 1 both lead to triangle 1.
        let tri = triangle_ptr(&e, mesh, 0);
        e.mem.set_u16(tri.addr() + 6, 1);
        let (ok, prints) = run_check(&mut e, mesh);
        assert!(!ok);
        assert!(prints.contains(&vec![FORMAT_SAME_NEIGHBOUR_01, 0, 1]));
    }

    #[test]
    fn check_reports_a_link_the_neighbour_does_not_return() {
        let mut e = check_engine();
        let mesh = two_triangles(&mut e);
        checked(&mut e, mesh, None);
        let neighbour = triangle_ptr(&e, mesh, 1);
        e.mem.set_u16(neighbour.addr() + 6 + 4, 0xffff);
        let (ok, prints) = run_check(&mut e, mesh);
        assert!(!ok);
        assert!(prints.contains(&vec![FORMAT_LINK_NOT_RETURNED, 0, 1, 1]));
        assert!(prints.contains(&vec![FORMAT_MISSING_LINK, 1, 2, 0]));
    }

    #[test]
    fn check_reports_linked_triangles_with_opposite_normals() {
        let mut e = check_engine();
        // Triangle 1 runs the shared edge in the same direction as triangle 0.
        let mesh = mesh_with(
            &mut e,
            &[[0.0; 3], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [1.0, 1.0, 0.0]],
            &[
                ([0, 1, 2], [0xffff, 1, 0xffff], 0),
                ([1, 2, 3], [0, 0xffff, 0xffff], 0),
            ],
        );
        checked(&mut e, mesh, None);
        let (ok, prints) = run_check(&mut e, mesh);
        assert!(!ok);
        assert!(prints.contains(&vec![FORMAT_OPPOSITE_NORMALS, 0, 1]));
    }

    #[test]
    fn check_reports_mismatched_shared_vertices() {
        let mut e = check_engine();
        let mesh = two_triangles(&mut e);
        checked(&mut e, mesh, None);
        // Triangle 1 keeps the link but moves its vertex 0 away from the edge.
        let neighbour = triangle_ptr(&e, mesh, 1);
        e.mem.set_u16(neighbour.addr(), 3);
        let (ok, prints) = run_check(&mut e, mesh);
        assert!(!ok);
        assert!(prints.contains(&vec![FORMAT_VERTICES_MISMATCH, 0, 1]));
    }

    #[test]
    fn check_reports_missing_portal_data_on_the_other_side() {
        let mut e = check_engine();
        let mesh = two_triangles(&mut e);
        checked(&mut e, mesh, None);
        // Triangle 0 claims extra info on the shared edge (valid record),
        // triangle 1 has none.
        let record = portal_record(&mut e, 0, WORLD_SPACE, None);
        put_array(&mut e, mesh.at(NavMesh::ExtraEdgeInfo), &record, 1);
        let tri = triangle_ptr(&e, mesh, 0);
        e.mem.set_u32(tri.addr() + 0xc, 0b010);
        e.mem.set_u16(tri.addr() + 8, 0);
        let (ok, prints) = run_check(&mut e, mesh);
        assert!(!ok);
        assert!(prints.contains(&vec![FORMAT_MISSING_PORTAL, 1, 2]));
    }

    #[test]
    fn check_reports_portal_data_the_other_side_lacks_a_match_for() {
        let mut e = check_engine();
        let mesh = two_triangles(&mut e);
        checked(&mut e, mesh, None);
        // Triangle 1 has extra info on its shared edge, triangle 0 does not.
        let record = portal_record(&mut e, 0, WORLD_SPACE, None);
        put_array(&mut e, mesh.at(NavMesh::ExtraEdgeInfo), &record, 1);
        let neighbour = triangle_ptr(&e, mesh, 1);
        e.mem.set_u32(neighbour.addr() + 0xc, 0b100);
        e.mem.set_u16(neighbour.addr() + 6 + 4, 0);
        let (ok, prints) = run_check(&mut e, mesh);
        assert!(!ok);
        assert!(prints.contains(&vec![FORMAT_PORTAL_WITHOUT_MATCH, 0, 1, 1, 1, 2]));
    }

    #[test]
    fn check_clears_a_bad_extra_info_index() {
        let mut e = check_engine();
        let mesh = mesh_with(
            &mut e,
            &[[0.0; 3], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
            &[([0, 1, 2], [4, 0xffff, 0xffff], 0b001)],
        );
        checked(&mut e, mesh, None);
        let (ok, prints) = run_check(&mut e, mesh);
        assert!(!ok);
        assert_eq!(prints, vec![vec![FORMAT_BAD_EXTRA_INFO_INDEX, 0, 0]]);
        let tri = triangle_ptr(&e, mesh, 0);
        assert_eq!(e.mem.u32(tri.addr() + 0xc), 0);
        assert_eq!(e.mem.u16(tri.addr() + 6), 0xffff);
    }

    /// An extra-info record with a portal to triangle `portal_triangle` of
    /// another mesh whose info lives in `world_space` (or to `other_info`).
    fn portal_record(
        e: &mut Engine,
        portal_triangle: u16,
        world_space: u32,
        other_info: Option<u32>,
    ) -> Vec<u8> {
        let other = mesh_with(e, &[], &[([0, 1, 2], [0xffff; 3], 0)]);
        e.mem.set_u32(other.addr() + 0xc, 0x0bad_0001);
        let info = e.mem.alloc(0x60);
        e.mem.set_u32(info, 0xbeef);
        e.mem.set_u32(info + 0x1c, world_space);
        e.mem.set_u32(info + 0x54, other.addr());
        let mut record = vec![0u8; 12];
        record[4..8].copy_from_slice(&other_info.unwrap_or(info).to_le_bytes());
        record[8..10].copy_from_slice(&portal_triangle.to_le_bytes());
        record
    }

    /// A mesh with one triangle whose edge 0 has extra info (record 0, see
    /// [`portal_record`]).
    fn portal_check(
        e: &mut Engine,
        portal_triangle: u16,
        world_space: u32,
        other_info: Option<u32>,
    ) -> Ptr<NavMesh> {
        let record = portal_record(e, portal_triangle, world_space, other_info);
        let mesh = mesh_with(
            e,
            &[[0.0; 3], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
            &[([0, 1, 2], [0, 0xffff, 0xffff], 0b001)],
        );
        put_array(e, mesh.at(NavMesh::ExtraEdgeInfo), &record, 1);
        checked(e, mesh, None);
        mesh
    }

    #[test]
    fn check_accepts_a_valid_portal() {
        let mut e = check_engine();
        let mesh = portal_check(&mut e, 0, WORLD_SPACE, None);
        let (ok, prints) = run_check(&mut e, mesh);
        assert!(ok, "{prints:?}");
        assert!(prints.is_empty());
    }

    #[test]
    fn check_deletes_a_portal_to_a_missing_triangle() {
        let mut e = check_engine();
        let mesh = portal_check(&mut e, 5, WORLD_SPACE, None);
        e.call_log = Some(vec![]);
        let ok = e.call(0x0068_f6d0, &args![mesh, 0u32]).bool();
        let log = e.call_log.take().unwrap();
        assert!(!ok);
        let prints = calls_to(&log, DEBUG_PRINT);
        assert_eq!(prints.len(), 1);
        // Triangle 0, edge 0, the other mesh's form id, the bad index 5.
        assert_eq!(prints[0][0], FORMAT_BAD_PORTAL_TRIANGLE);
        assert_eq!(&prints[0][3..], &[0, 0, 0x0bad_0001, 5]);
        assert_eq!(calls_to(&log, ARRAY_REMOVE_12).len(), 1);
    }

    #[test]
    fn check_deletes_a_portal_into_another_worldspace() {
        let mut e = check_engine();
        let mesh = portal_check(&mut e, 0, 0x0e20_1000, None);
        e.call_log = Some(vec![]);
        let ok = e.call(0x0068_f6d0, &args![mesh, 0u32]).bool();
        let log = e.call_log.take().unwrap();
        assert!(!ok);
        let prints = calls_to(&log, DEBUG_PRINT);
        assert_eq!(prints.len(), 1);
        assert_eq!(prints[0][0], FORMAT_PORTAL_OTHER_WORLD_SPACE);
        assert_eq!(&prints[0][3..], &[0, 0, 0xbeef]);
        assert_eq!(calls_to(&log, ARRAY_REMOVE_12).len(), 1);
    }

    #[test]
    fn check_deletes_a_portal_without_a_target_or_triangle() {
        let mut e = check_engine();
        // Portal info pointer is null.
        let mesh = portal_check(&mut e, 0, WORLD_SPACE, Some(0));
        e.call_log = Some(vec![]);
        assert!(!e.call(0x0068_f6d0, &args![mesh, 0u32]).bool());
        let log = e.call_log.take().unwrap();
        assert_eq!(calls_to(&log, ARRAY_REMOVE_12).len(), 1);
        assert!(calls_to(&log, DEBUG_PRINT).is_empty());
        // Portal triangle is 0xFFFF.
        let mesh = portal_check(&mut e, 0xffff, WORLD_SPACE, None);
        e.call_log = Some(vec![]);
        assert!(!e.call(0x0068_f6d0, &args![mesh, 0u32]).bool());
        let log = e.call_log.take().unwrap();
        assert_eq!(calls_to(&log, ARRAY_REMOVE_12).len(), 1);
    }

    #[test]
    fn check_reports_cover_entries() {
        let mut e = check_engine();
        let mesh = two_triangles(&mut e);
        checked(&mut e, mesh, None);
        // Cover entry 0: triangle 0 without cover flags; entry 1: triangle 1
        // with a cover flag.
        put_array(&mut e, mesh.at(NavMesh::CoverArray), &[0, 0, 1, 0], 2);
        let covered = triangle_ptr(&e, mesh, 1);
        e.mem.set_u32(covered.addr() + 0xc, 0x0002_0000);
        let (ok, prints) = run_check(&mut e, mesh);
        assert!(!ok);
        assert_eq!(prints, vec![vec![FORMAT_COVER_WITHOUT_COVER, 0, 0]]);
    }

    #[test]
    fn check_reports_a_cover_entry_without_a_triangle() {
        let mut e = check_engine();
        let mesh = mesh_with(&mut e, &[], &[]);
        checked(&mut e, mesh, None);
        // No triangle buffer: triangle 0 resolves to null.
        e.mem.set_u32(mesh.addr() + 0x38 + 4, 0);
        put_array(&mut e, mesh.at(NavMesh::CoverArray), &[0, 0], 1);
        let (ok, prints) = run_check(&mut e, mesh);
        assert!(!ok);
        assert_eq!(prints, vec![vec![FORMAT_COVER_MISSING_TRIANGLE, 0, 0]]);
    }

    // ---- triangle and info methods -----------------------------------------

    #[test]
    fn cover_flags_mask() {
        let mut e = Engine::new();
        let tri: Ptr<NavMeshTriangle> = e.new_object();
        assert!(!e.call(0x0069_0770, &args![tri]).bool());
        e.set(tri, NavMeshTriangle::TriangleFlags, 0x0002_0000);
        assert!(e.call(0x0069_0770, &args![tri]).bool());
        // Bits outside the mask (0x0FBE0000) do not count.
        e.set(tri, NavMeshTriangle::TriangleFlags, 0x0041_ffff);
        assert!(!e.call(0x0069_0770, &args![tri]).bool());
    }

    #[test]
    fn clearing_edge_info_resets_flag_and_neighbour() {
        let mut e = Engine::new();
        let tri: Ptr<NavMeshTriangle> = e.new_object();
        e.set(tri, NavMeshTriangle::TriangleFlags, 0b111);
        e.mem.write(tri.addr() + 6, &[1, 0, 2, 0, 3, 0]);
        e.call(0x0069_0790, &args![tri, 1u32]);
        assert_eq!(e.get(tri, NavMeshTriangle::TriangleFlags), 0b101);
        assert_eq!(e.mem.u16(tri.addr() + 8), 0xffff);
        assert_eq!(e.mem.u16(tri.addr() + 6), 1);
        assert_eq!(e.mem.u16(tri.addr() + 10), 3);
    }

    #[test]
    fn setting_a_plain_neighbour_clears_the_flag_first() {
        let mut e = Engine::new();
        let tri: Ptr<NavMeshTriangle> = e.new_object();
        e.set(tri, NavMeshTriangle::TriangleFlags, 0b010);
        e.call(0x0069_07d0, &args![tri, 1u32, 9u16]);
        assert_eq!(e.get(tri, NavMeshTriangle::TriangleFlags), 0);
        assert_eq!(e.mem.u16(tri.addr() + 8), 9);
    }

    #[test]
    fn parent_space_is_cast_to_a_worldspace() {
        let mut e = Engine::new();
        e.register(LOOKUP_FORM, |_, a| result(a[0] + 0x100));
        e.register_double(RT_DYNAMIC_CAST, |_, a| result(a[0] + 1));
        let info = Ptr::<()>::new(e.mem.alloc(0x60));
        e.mem.set_u32(info.addr() + 0x1c, 0x5000);
        e.call_log = Some(vec![]);
        assert_eq!(e.call(0x0069_0800, &args![info]).u32(), 0x5001);
        let log = e.call_log.take().unwrap();
        assert_eq!(
            calls_to(&log, RT_DYNAMIC_CAST),
            vec![vec![0x5000, 0, TYPE_TES_FORM, TYPE_TES_WORLD_SPACE, 0]]
        );
    }

    #[test]
    fn parent_space_falls_back_to_the_form_lookup() {
        let mut e = Engine::new();
        e.register(LOOKUP_FORM, |_, a| result(a[0] + 0x100));
        let info = Ptr::<()>::new(e.mem.alloc(0x60));
        assert_eq!(e.call(0x0069_0830, &args![info]).u32(), 0x100);
        e.mem.set_u32(info.addr() + 4, 0x3c);
        assert_eq!(e.call(0x0069_0830, &args![info]).u32(), 0x13c);
        e.mem.set_u32(info.addr() + 0x1c, 0x7777);
        assert_eq!(e.call(0x0069_0830, &args![info]).u32(), 0x7777);
    }

    #[test]
    fn info_mesh_assignment_reports_a_loaded_mesh() {
        let mut e = Engine::new();
        e.register(POINTER_CONSTRUCT_FROM, |e, a| {
            e.mem.set_u32(a[0], a[1]);
            Ret::default()
        });
        e.register(POINTER_ASSIGN, |e, a| {
            let value = e.mem.u32(a[1]);
            e.mem.set_u32(a[0], value);
            Ret::default()
        });
        e.register(POINTER_DESTRUCT, |_, _| Ret::default());
        let info = Ptr::<()>::new(e.mem.alloc(0x60));
        let out = Ptr::<()>::new(e.mem.alloc(4));
        e.mem.set_u32(info.addr() + 0x54, 0x6000);
        assert!(e.call(0x0069_0860, &args![info, out]).bool());
        assert_eq!(e.mem.u32(out.addr()), 0x6000);
        e.mem.set_u32(info.addr() + 0x54, 0);
        assert!(!e.call(0x0069_0860, &args![info, out]).bool());
        assert_eq!(e.mem.u32(out.addr()), 0);
    }

    #[test]
    fn smart_pointer_assignment_uses_a_temporary() {
        let mut e = Engine::new();
        noops(
            &mut e,
            &[POINTER_CONSTRUCT_FROM, POINTER_ASSIGN, POINTER_DESTRUCT],
        );
        e.call_log = Some(vec![]);
        e.call(0x0069_0890, &args![0x100u32, 0x200u32]);
        let log = e.call_log.take().unwrap();
        let construct = calls_to(&log, POINTER_CONSTRUCT_FROM);
        let assign = calls_to(&log, POINTER_ASSIGN);
        let destruct = calls_to(&log, POINTER_DESTRUCT);
        let temp = construct[0][0];
        assert_eq!(construct[0][1], 0x200);
        assert_eq!(assign, vec![vec![0x100, temp]]);
        assert_eq!(destruct, vec![vec![temp]]);
    }

    /// Doubles for the neighbour-cell walk of `006908f0`.
    fn neighbour_engine() -> (Engine, Ptr<NavMesh>) {
        let mut e = engine();
        e.map(0x0101_7000, 0x1000);
        e.register(GET_PARENT_CELL, |e, a| result(e.mem.u32(a[0] + 0x24)));
        e.register(CELL_IS_INTERIOR, |e, a| {
            result((e.mem.u8(a[0] + 0x24) & 1) as u32)
        });
        e.register(CELL_GET_X, |e, a| result(e.mem.u32(a[0] + 0x48)));
        e.register(CELL_GET_Y, |e, a| result(e.mem.u32(a[0] + 0x4c)));
        e.register(CELL_GET_WORLD_SPACE, |_, _| result(0x7000));
        e.register(CELL_GET_NAV_MESHES, |e, a| result(e.mem.u32(a[0] + 0x64)));
        e.register(NAV_MESH_ARRAY_SIZE, |e, a| result(e.mem.u32(a[0] + 8)));
        e.register(NAV_MESH_ARRAY_GET, |e, a| {
            let element = e.mem.u32(e.mem.u32(a[0] + 4) + 4 * a[2]);
            e.mem.set_u32(a[1], element);
            result(a[1])
        });
        e.register(POINTER_DEREFERENCE, |e, a| result(e.mem.u32(a[0])));
        noops(&mut e, &[POINTER_DESTRUCT]);
        let mesh: Ptr<NavMesh> = e.new_object();
        let cell = e.mem.alloc(0x80);
        e.mem.set_u32(cell + 0x48, 10);
        e.mem.set_u32(cell + 0x4c, 20);
        e.set(mesh, NavMesh::pParentCell, Ptr::new(cell));
        (e, mesh)
    }

    /// A cell holding the given navmeshes as its navmesh array.
    fn cell_with_meshes(e: &mut Engine, meshes: &[u32]) -> u32 {
        let cell = e.mem.alloc(0x80);
        let array = e.mem.alloc(0x10);
        let buffer = e.mem.alloc(8 + 4 * meshes.len() as u32);
        for (i, mesh) in meshes.iter().enumerate() {
            e.mem.set_u32(buffer + 4 * i as u32, *mesh);
        }
        e.mem.set_u32(array + 4, buffer);
        e.mem.set_u32(array + 8, meshes.len() as u32);
        e.mem.set_u32(cell + 0x64, array);
        cell
    }

    #[test]
    fn interior_cells_pass_the_neighbour_check() {
        let (mut e, mesh) = neighbour_engine();
        let cell = e.get(mesh, NavMesh::pParentCell);
        e.mem.set_u8(cell.addr() + 0x24, 1);
        e.call_log = Some(vec![]);
        assert!(e.call(0x0069_08f0, &args![mesh]).bool());
        let log = e.call_log.take().unwrap();
        assert!(calls_to(&log, WORLD_SPACE_GET_CELL).is_empty());
    }

    #[test]
    fn exterior_cells_are_checked_against_their_neighbours() {
        let (mut e, mesh) = neighbour_engine();
        e.set_global(CELL_WIDTH, 4096.0f64);
        let east = cell_with_meshes(&mut e, &[0x111, 0x222]);
        let north = cell_with_meshes(&mut e, &[0x333]);
        // West of the cell there is no cell; south has a cell without an
        // array.
        let south = e.mem.alloc(0x80);
        e.register_double(WORLD_SPACE_GET_CELL, move |_, a| {
            result(match (a[1] as i32, a[2] as i32) {
                (11, 20) => east,
                (10, 21) => north,
                (10, 19) => south,
                _ => 0,
            })
        });
        e.register_double(NEIGHBOUR_CHECK, |_, _| result(1));
        e.call_log = Some(vec![]);
        assert!(e.call(0x0069_08f0, &args![mesh]).bool());
        let log = e.call_log.take().unwrap();
        let checks = calls_to(&log, NEIGHBOUR_CHECK);
        let far_x = (10.0f32 * 4096.0 + 4096.0).to_bits();
        let far_y = (20.0f32 * 4096.0 + 4096.0).to_bits();
        assert_eq!(
            checks,
            vec![
                vec![mesh.addr(), 0x111, 1, far_x, 0],
                vec![mesh.addr(), 0x222, 1, far_x, 0],
                vec![mesh.addr(), 0x333, 0, 0, far_y],
            ]
        );
        // The west neighbour is asked for first, as cell (x - 1, y).
        let lookups = calls_to(&log, WORLD_SPACE_GET_CELL);
        assert_eq!(lookups[0], vec![0x7000, 9u32, 20]);
    }

    #[test]
    fn a_rejecting_neighbour_check_fails_the_mesh_check() {
        let (mut e, mesh) = neighbour_engine();
        e.set_global(CELL_WIDTH, 4096.0f64);
        let west = cell_with_meshes(&mut e, &[0x444]);
        e.register_double(WORLD_SPACE_GET_CELL, move |_, a| {
            result(if (a[1] as i32, a[2] as i32) == (9, 20) {
                west
            } else {
                0
            })
        });
        e.register_double(NEIGHBOUR_CHECK, |_, _| result(0));
        e.call_log = Some(vec![]);
        assert!(!e.call(0x0069_08f0, &args![mesh]).bool());
        let log = e.call_log.take().unwrap();
        let near_x = (10.0f32 * 4096.0).to_bits();
        assert_eq!(
            calls_to(&log, NEIGHBOUR_CHECK),
            vec![vec![mesh.addr(), 0x444, 1, near_x, 0]]
        );
    }

    #[test]
    fn clearing_the_mesh_empties_everything_and_deletes_the_info() {
        let mut e = Engine::new();
        noops(
            &mut e,
            &[
                ARRAY_CLEAR,
                GRID_CLEAR,
                OBSTACLES_CLEAR,
                DELETE_NAV_MESH_INFO,
            ],
        );
        e.register(GET_FORM_ID, |e, a| result(e.mem.u32(a[0] + 0xc)));
        e.register(GET_NAV_MESH_INFO_MAP, |_, a| result(a[0] + 1));
        e.map(0x011d_e000, 0x1000);
        e.set_global(TES_INSTANCE, 0x5000u32);
        let mesh: Ptr<NavMesh> = e.new_object();
        e.mem.set_u32(mesh.addr() + 0xc, 0xabcd);
        e.call_log = Some(vec![]);
        e.call(0x0069_0d50, &args![mesh, 0u32]);
        let log = e.call_log.take().unwrap();
        // No info yet: nothing is deleted.
        assert!(calls_to(&log, DELETE_NAV_MESH_INFO).is_empty());
        assert_eq!(calls_to(&log, ARRAY_CLEAR).len(), 5);
        assert_eq!(calls_to(&log, OBSTACLES_CLEAR).len(), 1);
        assert_eq!(calls_to(&log, GRID_CLEAR), vec![vec![mesh.addr() + 0xa8]]);
        e.set(mesh, NavMesh::pNavMeshInfo, Ptr::new(0x4444));
        e.call_log = Some(vec![]);
        e.call(0x0069_0d50, &args![mesh, 0u32]);
        let log = e.call_log.take().unwrap();
        assert_eq!(
            calls_to(&log, DELETE_NAV_MESH_INFO),
            vec![vec![0x5001, 0xabcd]]
        );
    }

    #[test]
    fn triangle_vertex_setter() {
        let mut e = Engine::new();
        let tri: Ptr<NavMeshTriangle> = e.new_object();
        e.call(0x0069_0df0, &args![tri, 2u32, 0x1234u16]);
        assert_eq!(e.mem.u16(tri.addr() + 4), 0x1234);
        assert_eq!(e.mem.u16(tri.addr()), 0);
    }

    #[test]
    fn edge_info_presence_flags() {
        let mut e = Engine::new();
        let tri: Ptr<NavMeshTriangle> = e.new_object();
        assert!(!e.call(0x0069_0e10, &args![tri]).bool());
        assert!(!e.call(0x0069_0e70, &args![tri]).bool());
        e.set(tri, NavMeshTriangle::TriangleFlags, 0x4);
        assert!(e.call(0x0069_0e10, &args![tri]).bool());
        assert!(!e.call(0x0069_0e70, &args![tri]).bool());
        e.set(tri, NavMeshTriangle::TriangleFlags, 0x408);
        assert!(!e.call(0x0069_0e10, &args![tri]).bool());
        assert!(e.call(0x0069_0e70, &args![tri]).bool());
    }

    #[test]
    fn setting_edge_info_sets_flag_and_slot() {
        let mut e = Engine::new();
        let tri: Ptr<NavMeshTriangle> = e.new_object();
        e.set(tri, NavMeshTriangle::TriangleFlags, 0b001);
        e.call(0x0069_0e30, &args![tri, 2u32, 6u16]);
        assert_eq!(e.get(tri, NavMeshTriangle::TriangleFlags), 0b101);
        assert_eq!(e.mem.u16(tri.addr() + 10), 6);
    }

    #[test]
    fn deleting_extra_edge_info_repairs_the_triangles() {
        let mut e = engine();
        e.register(ARRAY_REMOVE_12, |e, a| {
            let size = e.mem.u32(a[0] + 8);
            e.mem.set_u32(a[0] + 8, size - a[2]);
            Ret::default()
        });
        // Three infos; info 0 is deleted and info 2 (the last) moves to 0.
        // Triangle 0: edge 0 uses info 0, edge 1 uses info 2, edge 2 uses a
        // plain neighbour. Triangle 1 has no extra info flags at all.
        let mesh = mesh_with(
            &mut e,
            &[],
            &[([0, 1, 2], [0, 2, 1], 0b011), ([0, 1, 2], [5, 5, 5], 0)],
        );
        put_array(&mut e, mesh.at(NavMesh::ExtraEdgeInfo), &[0u8; 36], 3);
        e.call_log = Some(vec![]);
        e.call(0x0069_0e90, &args![mesh, 0u16]);
        let log = e.call_log.take().unwrap();
        assert_eq!(
            calls_to(&log, ARRAY_REMOVE_12),
            vec![vec![mesh.addr() + 0x48, 0, 1]]
        );
        let tri = triangle_ptr(&e, mesh, 0);
        // Edge 0 lost its flag and neighbour; edge 1 now points at info 0.
        assert_eq!(e.mem.u32(tri.addr() + 0xc), 0b010);
        assert_eq!(e.mem.u16(tri.addr() + 6), 0xffff);
        assert_eq!(e.mem.u16(tri.addr() + 8), 0);
        assert_eq!(e.mem.u16(tri.addr() + 10), 1);
        let other = triangle_ptr(&e, mesh, 1);
        assert_eq!(e.mem.u16(other.addr() + 6), 5);
    }

    #[test]
    fn deleting_extra_edge_info_leaves_other_infos_alone() {
        let mut e = engine();
        e.register(ARRAY_REMOVE_12, |e, a| {
            let size = e.mem.u32(a[0] + 8);
            e.mem.set_u32(a[0] + 8, size - a[2]);
            Ret::default()
        });
        let mesh = mesh_with(&mut e, &[], &[([0, 1, 2], [1, 0xffff, 0xffff], 0b001)]);
        put_array(&mut e, mesh.at(NavMesh::ExtraEdgeInfo), &[0u8; 36], 3);
        e.call(0x0069_0e90, &args![mesh, 0u16]);
        let tri = triangle_ptr(&e, mesh, 0);
        assert_eq!(e.mem.u32(tri.addr() + 0xc), 0b001);
        assert_eq!(e.mem.u16(tri.addr() + 6), 1);
    }

    #[test]
    fn appending_vertices_and_triangles_forwards_to_the_arrays() {
        let mut e = Engine::new();
        e.register(ARRAY_ADD_12, |_, a| result(a[0] * 10 + a[1]));
        e.register(ARRAY_ADD_16, |_, a| result(a[0] * 100 + a[1]));
        let mesh: Ptr<NavMesh> = Ptr::new(0x1000);
        assert_eq!(
            e.call(0x0069_0fe0, &args![mesh, 7u32]).u32(),
            (0x1000 + 0x28) * 10 + 7
        );
        assert_eq!(
            e.call(0x0069_1000, &args![mesh, 9u32]).u32(),
            (0x1000 + 0x38) * 100 + 9
        );
    }
}
