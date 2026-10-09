//! `fallout shared/pathfinding/pathfind.cpp` (Xbox PDB source unit), subsystem `fallout shared/pathfinding`: its functions in
//! FalloutNV.exe 1.4.0.525, translated (docs/ENGINE_CRATE.md).
//!
//! The unit solves a `PathingRequest` for the pathing task: `fn_006d0900`
//! looks at the request's type (virtual slot `GetType`, +0x10) and hands it
//! to the solver of that type (0 base request, 3 flee, 4 close point, 5 line
//! of sight, 6 hide, 7 optimal location, 8 covered move). Each solver fills a
//! `PathingSolution` and returns whether it found a path. The rest of the
//! file are small accessors and constructors the solvers use.
//!
//! How the translations read the game code:
//! - The game keeps many objects on its stack (searches, node arrays, scope
//!   guards) and passes their addresses to its functions. Each solver here
//!   takes one zeroed block for its frame ([`with_frame`]) and addresses the
//!   locals by their `EBP`-relative offset exactly as the code does, so a
//!   comment `// ebp-0x20ec` names the local and the test can find it.
//! - The compiler's exception-unwinding frame (`__CxxFrameHandler` state
//!   writes and the `FS:[0]` chain) is not translated; every exit path
//!   destroys the live locals in the order the game does.
//! - Callee arguments are listed in the uniform form (`this` first, then the
//!   stack words from the top of the stack downwards, i.e. the last `PUSH`
//!   first). Floats the callee leaves in `ST0` come back through `.f32()` or,
//!   where the game compares the unrounded `ST0`, `.f64()`.
//! - Offsets that are not in a declared layout are named in comments with
//!   the Xbox PDB field they correspond to.

#[allow(unused_imports)]
use crate::prelude::*;
use crate::types::NiPoint3;

// ---------------------------------------------------------------------------
// Layouts (PC offsets; names from the Xbox PDB where it has the class)
// ---------------------------------------------------------------------------

layout! {
    /// `PathingLocation` (Xbox PDB), 0x28 bytes: vtable, position, navmesh
    /// info, cell, worldspace, triangle and flags.
    pub struct PathingLocation: 0x28 {
        /// `Location` (Xbox PDB): the position.
        0x04 Location: Inline<NiPoint3>,
        /// `pCell` (Xbox PDB): `TESObjectCELL*`.
        0x18 pCell: Ptr,
        /// `pWorldSpace` (Xbox PDB): `TESWorldSpace*`.
        0x1C pWorldSpace: Ptr,
        /// `usTriangle` (Xbox PDB).
        0x24 usTriangle: u16,
        /// `uiFlags` (Xbox PDB): bit 1 (`2`) is set by `fn_006d2c00` and
        /// tested by `006a9520`.
        0x26 uiFlags: u8,
    }

    /// `PathingRequest` (Xbox PDB), 0xB0 bytes; only the fields this unit
    /// touches.
    pub struct PathingRequest: 0xB0 {
        /// `Origin` (Xbox PDB).
        0x0C Origin: Inline<PathingLocation>,
        /// `Destination` (Xbox PDB).
        0x34 Destination: Inline<PathingLocation>,
        /// `fActorRadius` (Xbox PDB).
        0x68 fActorRadius: f32,
        /// `fInitialPathHeading` (Xbox PDB).
        0x90 fInitialPathHeading: f32,
        /// `spAvoidNodeArray` (Xbox PDB): `NiPointer<PathingAvoidNodeArray>`.
        0x94 spAvoidNodeArray: Ptr,
        /// `bFirstTangentUsesHeading` (Xbox PDB).
        0xA0 bFirstTangentUsesHeading: bool,
    }

    /// `PathingRequestCoveredMove` (Xbox PDB), 0x10C bytes: a
    /// `PathingRequest` plus the cover data; only the fields this unit
    /// touches.
    pub struct PathingRequestCoveredMove: 0x10C {
        /// `CoverLocation` (Xbox PDB).
        0xC8 CoverLocation: Inline<NiPoint3>,
        /// `pCombatCoverLocation` (Xbox PDB): `CombatCoverLocation*`.
        0x104 pCombatCoverLocation: Ptr,
        /// `pCombatCoverSearchInfo` (Xbox PDB): `CombatCoverSearchInfo*`.
        0x108 pCombatCoverSearchInfo: Ptr,
    }

    /// `PathingSolution` (Xbox PDB); the PC build has only the node data,
    /// laid out right after the `NiRefObject` base (the Xbox build's debug
    /// data is missing), so the offsets differ from the PDB's.
    pub struct PathingSolution: 0x44 {
        /// `VirtualPathingNodes` (Xbox PDB): a `BSSimpleArray` of 0x30-byte
        /// `VirtualPathingNode`s.
        0x08 VirtualPathingNodes: Inline<crate::types::BSSimpleArray>,
        /// `iFirstLoadedVirtualNodeIndex` (Xbox PDB).
        0x18 iFirstLoadedVirtualNodeIndex: i32,
        /// `iLastLoadedVirtualNodeIndex` (Xbox PDB).
        0x1C iLastLoadedVirtualNodeIndex: i32,
        /// `CurrentPathingNodes` (Xbox PDB): a `BSSimpleArray` of
        /// `PathingNode`s.
        0x20 CurrentPathingNodes: Inline<crate::types::BSSimpleArray>,
        /// `bIncompletePath` (Xbox PDB).
        0x40 bIncompletePath: bool,
    }

    /// `CombatCoverLocation` (Xbox PDB), 0x78 bytes.
    pub struct CombatCoverLocation: 0x78 {
        /// `bDirtyFlag` (Xbox PDB).
        0x00 bDirtyFlag: bool,
        /// `bReserved` (Xbox PDB).
        0x01 bReserved: bool,
        /// `cCoverHeight` (Xbox PDB).
        0x02 cCoverHeight: u8,
        /// `iLocation` (Xbox PDB).
        0x04 iLocation: i32,
        /// `CoverSearchTargetLocation` (Xbox PDB).
        0x08 CoverSearchTargetLocation: Inline<NiPoint3>,
        /// `CoverLocation` (Xbox PDB): a `PathingCoverLocation`.
        0x14 CoverLocation: Inline<PathingCoverLocation>,
        /// `iCoverLocationKey` (Xbox PDB).
        0x68 iCoverLocationKey: i32,
        /// `iCoverLocationLastMovedTimeStamp` (Xbox PDB).
        0x6C iCoverLocationLastMovedTimeStamp: i32,
        /// `ReservationInfo` (Xbox PDB): a `CoverReservationInfo`.
        0x70 ReservationInfo: Inline<CoverReservationInfo>,
    }

    /// `PathingCoverLocation` (Xbox PDB), 0x54 bytes: a `PathingLocation`
    /// followed by three points.
    pub struct PathingCoverLocation: 0x54 {}

    /// `CoverReservationInfo` (Xbox PDB), 6 bytes of flags.
    pub struct CoverReservationInfo: 0x06 {}

    /// `CombatCoverSearchInfo` (Xbox PDB), 0x3C bytes.
    pub struct CombatCoverSearchInfo: 0x3C {
        /// `TargetLocation` (Xbox PDB).
        0x00 TargetLocation: Inline<NiPoint3>,
        /// `Center` (Xbox PDB).
        0x0C Center: Inline<NiPoint3>,
        /// `fRadius` (Xbox PDB).
        0x18 fRadius: f32,
        /// `fTimeStamp` (Xbox PDB): a `CombatTimeStamp` (one float).
        0x1C fTimeStamp: f32,
        /// `iPreviousCoverLocationKey` (Xbox PDB).
        0x30 iPreviousCoverLocationKey: i32,
        /// `ePreviousCoverState` (Xbox PDB).
        0x34 ePreviousCoverState: i32,
        /// `bUsedLastSeenLocation` (Xbox PDB).
        0x38 bUsedLastSeenLocation: bool,
    }
}

// ---------------------------------------------------------------------------
// Frames and callees
// ---------------------------------------------------------------------------

/// The stack frame of a translated function: a zeroed block standing in for
/// the game's stack, addressed by the `EBP`-relative offsets the code uses.
#[derive(Clone, Copy)]
struct Frame(u32);

impl Frame {
    /// The address of the local at `EBP + offset` (the offsets are negative
    /// for locals).
    fn at(self, offset: i32) -> u32 {
        self.0.wrapping_add(offset as u32)
    }
}

/// Runs `body` with a zeroed frame of the size the function reserves
/// (`locals`, from its `SUB ESP` or `__alloca_probe` size) plus slack.
fn with_frame<R>(e: &mut Engine, locals: u32, body: impl FnOnce(&mut Engine, Frame) -> R) -> R {
    e.with_stack(locals + 0x80, |e, block| {
        body(e, Frame(block.addr() + locals + 0x40))
    })
}

/// The scope timer every solver opens (`ECX` = the timer, stack: kind, 1,
/// source file name, line); closed by [`TIMER_SCOPE_DESTROY`].
const TIMER_SCOPE_CONSTRUCT: u32 = 0x0040_4eb0;
const TIMER_SCOPE_DESTROY: u32 = 0x0040_4ee0;
/// The timer kind every solver passes.
const TIMER_SCOPE_KIND: u32 = 0x2d;
/// `D:\_Fallout3\Platforms\Common\Code\Fallout Shared\Pathfinding\Pathfind.cpp`.
const PATHFIND_SOURCE_FILE_NAME: u32 = 0x0106_cb70;

/// Opens the scope timer at `timer` for source line `line`.
fn scope_enter(e: &mut Engine, timer: u32, line: u32) {
    e.call(
        TIMER_SCOPE_CONSTRUCT,
        &args![
            timer,
            TIMER_SCOPE_KIND,
            1u32,
            PATHFIND_SOURCE_FILE_NAME,
            line
        ],
    );
}

/// Closes the scope timer at `timer`.
fn scope_leave(e: &mut Engine, timer: u32) {
    e.call(TIMER_SCOPE_DESTROY, &args![timer]);
}

// General callees (cdecl unless noted; `ECX` is the first word otherwise).

/// `cdecl (size)`: the allocator, returns the block.
const MEMORY_ALLOCATE: u32 = 0x0040_1000;
/// `cdecl (block)`: frees a block from [`MEMORY_ALLOCATE`].
const MEMORY_FREE: u32 = 0x0040_1030;
/// `stdcall (array, element size, count, constructor)`: the compiler's
/// `vector constructor iterator`, runs a constructor over an array.
const VECTOR_CONSTRUCTOR_ITERATOR: u32 = 0x0040_1050;
/// `ECX` = setting object: the address of its `float` value.
const SETTING_FLOAT_ADDRESS: u32 = 0x0040_3e20;
/// `cdecl (a, b)`, `float` in `ST0`: `b`, or `a` when `b < a`.
const FLOAT_MAX: u32 = 0x0040_4010;
/// `cdecl (value)`, `float` in `ST0`: the absolute value.
const FLOAT_ABSOLUTE_VALUE: u32 = 0x0040_8840;
/// `ECX` = point, stack (x, y, z): stores the three floats.
const POINT3_CONSTRUCT: u32 = 0x0041_6870;
/// `ECX` = point, stack (scale): multiplies the point in place.
const POINT3_SCALE_IN_PLACE: u32 = 0x0043_9180;
/// `ECX` = point, stack (result, other): `*result = this + other`, returns
/// `result`.
const POINT3_ADD: u32 = 0x0043_9e90;
/// `ECX` = point, stack (result, other): `*result = this - other`, returns
/// `result`.
const POINT3_SUBTRACT: u32 = 0x0043_9ef0;
/// `ECX` = point, stack (result, scalar): `*result = this * scalar`, returns
/// `result`.
const POINT3_TIMES_SCALAR: u32 = 0x0045_bb20;
/// `ECX` = point: normalizes it in place (zero when shorter than 1e-6).
const POINT3_NORMALIZE: u32 = 0x004a_0c10;
/// `cdecl (result, scalar, vector)`: `*result = *vector * scalar`, returns
/// `result`.
const POINT3_SCALED_COPY: u32 = 0x004a_3760;
/// `ECX` = point: the squared length, in `ST0`.
const POINT3_LENGTH_SQUARED: u32 = 0x004a_7290;
/// `ECX` = point: `NiPoint3::UnitizeGetLength` (Xbox PDB), the length in
/// `ST0`.
const POINT3_UNITIZE_GET_LENGTH: u32 = 0x0045_7910;
/// `ECX` = point, stack (other): adds `other` in place.
const POINT3_ADD_ASSIGN: u32 = 0x0063_c8a0;
/// `ECX` = point: the `NiPoint3` constructor (does nothing), returns `ECX`.
const POINT3_DEFAULT_CONSTRUCTOR: u32 = 0x0068_15c0;
/// `cdecl (angle)`, `float` in `ST0`: one of the two trigonometric functions
/// of an angle in radians; `004e44b0` and `004e44d0` apply the same one.
const ANGLE_FUNCTION_004E44B0: u32 = 0x004e_44b0;
const ANGLE_FUNCTION_004E44D0: u32 = 0x004e_44d0;
/// `cdecl (angle)`, `float` in `ST0`: the other one; `004e4470`, `004e4490`
/// and `005b9e80` apply it.
const ANGLE_FUNCTION_004E4470: u32 = 0x004e_4470;
const ANGLE_FUNCTION_004E4490: u32 = 0x004e_4490;
const ANGLE_FUNCTION_005B9E80: u32 = 0x005b_9e80;
/// `cdecl (min, max)`, `float` in `ST0`: a random number in the range.
const RANDOM_FLOAT: u32 = 0x0047_6b70;
/// No arguments, `float` in `ST0`: a random angle.
const RANDOM_ANGLE: u32 = 0x004a_4240;
/// No arguments: the global `BSRandom` object.
const RANDOM_OBJECT: u32 = 0x0047_6c00;
/// `ECX` = the `BSRandom`: a random unsigned integer.
const RANDOM_UNSIGNED: u32 = 0x0047_6be0;

// Navmesh holders and navmeshes.

/// `ECX` = the holder (a `NiPointer` to a navmesh): constructs it empty.
const NAV_HOLDER_CONSTRUCT: u32 = 0x0042_fb00;
/// `ECX` = the holder: releases the navmesh it holds.
const NAV_HOLDER_RELEASE: u32 = 0x0042_fa40;
/// `ECX` = the holder: the navmesh pointer it holds (same body as
/// [`NI_POINTER_GET_FROM_FIELD`]).
const NAV_HOLDER_GET: u32 = 0x0045_8b50;
/// `ECX` = a `NiPointer` slot: the pointer it holds.
const NI_POINTER_GET_FROM_FIELD: u32 = 0x0055_9450;
/// `ECX` = navmesh, stack (out, triangle): `NavMesh::GetCenter` (Xbox PDB).
const NAVMESH_GET_CENTER: u32 = 0x0055_8220;
/// `ECX` = navmesh: its `NavMeshInfo` (the word at +0x104).
const NAVMESH_GET_INFO: u32 = 0x0068_efb0;
/// `ECX` = navmesh, stack (vertex index): the address of the vertex.
const NAVMESH_GET_VERTEX: u32 = 0x0068_f0a0;
/// `ECX` = navmesh, stack (triangle index): the address of the triangle
/// record.
const NAVMESH_GET_TRIANGLE: u32 = 0x0055_82f0;
/// `ECX` = triangle record, stack (corner): the vertex index (`u16`) of
/// the corner.
const TRIANGLE_GET_VERTEX_INDEX: u32 = 0x0055_82d0;
/// `cdecl (out, a, b, c)`: a random point in the triangle `a b c`, returns
/// `out`.
const TRIANGLE_RANDOM_POINT: u32 = 0x006b_bad0;

// Requests and locations.

/// `ECX` = request: the address of its `Origin` location (+0x0C).
const REQUEST_ORIGIN: u32 = 0x0048_d150;
/// `ECX` = request: the address of its `Destination` location (+0x34).
const REQUEST_DESTINATION: u32 = 0x006a_9540;
/// `ECX` = request: the `float` at +0x68 (`fActorRadius`), in `ST0`.
const REQUEST_ACTOR_RADIUS: u32 = 0x0069_ef80;
/// `ECX` = request: the pointer held by `spAvoidNodeArray` (+0x94).
const REQUEST_AVOID_NODE_ARRAY: u32 = 0x0069_e390;
/// `ECX` = request: the byte at +0xA0 (`bFirstTangentUsesHeading`).
const REQUEST_FIRST_TANGENT_USES_HEADING: u32 = 0x006c_a4c0;
/// `ECX` = request: the `float` at +0x90 (`fInitialPathHeading`), in `ST0`.
const REQUEST_INITIAL_PATH_HEADING: u32 = 0x006c_a4e0;
/// `ECX` = optimal-location request: the `float` at +0xBC
/// (`fAbsoluteMaxDistance`), in `ST0`.
const REQUEST_ABSOLUTE_MAXIMUM_DISTANCE: u32 = 0x0099_e040;
/// `ECX` = optimal-location request: the `float` at +0xB8
/// (`fAbsoluteMinDistance`), in `ST0`.
const REQUEST_ABSOLUTE_MINIMUM_DISTANCE: u32 = 0x0048_ee50;
/// `ECX` = request, stack (value): stores the `float` at +0x74
/// (`fTargetRadius`).
const REQUEST_SET_TARGET_RADIUS: u32 = 0x006d_3b00;
/// `ECX` = request, stack (value): stores the `float` at +0xB0.
const REQUEST_SET_MINIMUM_DISTANCE: u32 = 0x0050_7610;
/// `ECX` = request, stack (value): stores the `float` at +0xB4.
const REQUEST_SET_MAXIMUM_DISTANCE: u32 = 0x006e_5ee0;
/// `ECX` = request, stack (flag): stores the byte at +0xE1 (`bFinished` of
/// the covered-move request).
const REQUEST_SET_FINISHED: u32 = 0x0061_3fb0;
/// `ECX` = request (to construct), stack (source request):
/// `PathingRequest::PathingRequest_ov2` (Xbox PDB).
const REQUEST_COPY_CONSTRUCT: u32 = 0x006e_26a0;
/// `ECX` = request copy: its destructor.
const REQUEST_COPY_DESTRUCT: u32 = 0x006e_2620;
/// `ECX` = `PathingRequestSafeStraightLine`: constructor (Xbox PDB).
const SAFE_STRAIGHT_LINE_CONSTRUCT: u32 = 0x006e_5fa0;
/// `ECX` = `PathingRequestSafeStraightLine`: destructor.
const SAFE_STRAIGHT_LINE_DESTRUCT: u32 = 0x006d_3b20;
/// `cdecl (request, out, flag)`: the point search `fn_006d3780` runs on its
/// request, returns whether it found a point.
const SAFE_STRAIGHT_LINE_FIND: u32 = 0x006d_3b40;
/// `ECX` = covered-move request, stack (source request):
/// `PathingRequestCoveredMove::PathingRequestCoveredMove_ov2` (Xbox PDB).
const COVERED_MOVE_CONSTRUCT_COPY: u32 = 0x006e_4d20;
/// `ECX` = covered-move request: `PathingRequestCoveredMove::
/// ~PathingRequestCoveredMove` (Xbox PDB).
const COVERED_MOVE_DESTRUCT: u32 = 0x006e_4bf0;
/// `ECX` = covered-move request: the address of its `CoverLocation`
/// (this + 0xC8).
const COVERED_MOVE_COVER_POINT: u32 = 0x0063_77e0;
/// `cdecl (request, distance, flag)`: whether a covered move for the
/// request's destination is possible.
const COVERED_MOVE_CHECK: u32 = 0x0099_d480;

/// `ECX` = location: `PathingLocation::GetCell` (Xbox PDB).
const LOCATION_GET_CELL: u32 = 0x006d_d4f0;
/// `ECX` = anything with a word at +0x1C: returns it. Its Xbox PDB name is
/// `PathingLocation::GetWorldspace`; the linker folded the other getters of
/// that word onto it.
const LOCATION_GET_WORLDSPACE: u32 = 0x0044_1110;
/// `ECX` = location, stack (out): copies the position (+4) to `out`,
/// returns `out`.
const LOCATION_COPY_POSITION: u32 = 0x005c_3420;
/// `ECX` = location: `PathingLocation::ResolveToClosestNavmeshAndTriangle`
/// (Xbox PDB), returns whether it found one.
const LOCATION_RESOLVE_CLOSEST: u32 = 0x006d_dc00;
/// `ECX` = location, stack (holder, triangle `u16`): `PathingLocation::
/// GetNavMeshAndTriangle` (Xbox PDB).
const LOCATION_GET_NAVMESH_AND_TRIANGLE: u32 = 0x006d_d640;
/// `ECX` = location, stack (flag): `PathingLocation::ResolveNavMeshInfo`
/// (Xbox PDB), returns whether the location has navmesh info.
const LOCATION_RESOLVE_NAVMESH_INFO: u32 = 0x006d_d6f0;
/// `ECX` = location: whether bit 1 of the flags byte (+0x26) is set.
const LOCATION_HAS_FLAG_2: u32 = 0x006a_9520;
/// `ECX` = location, stack (other): `PathingLocation::operator=` (Xbox PDB).
const LOCATION_ASSIGN: u32 = 0x006d_cce0;
/// `ECX` = location, stack (other): the copy constructor
/// (`PathingLocation::PathingLocation_ov2`, Xbox PDB).
const LOCATION_COPY_CONSTRUCT: u32 = 0x006d_cc40;
/// `ECX` = location, stack (reference): `PathingLocation::
/// PathingLocation_ov3` (Xbox PDB), returns `ECX`.
const LOCATION_CONSTRUCT_AT_REFERENCE: u32 = 0x006d_cd70;
/// `ECX` = location, stack (position, cell, worldspace):
/// `PathingLocation::PathingLocation_ov7` (Xbox PDB).
const LOCATION_CONSTRUCT_FROM_PARTS: u32 = 0x006d_cee0;
/// `ECX` = location, stack (position, navmesh info, triangle `u16`):
/// constructs a location on a navmesh triangle.
const LOCATION_CONSTRUCT_FROM_MESH_POINT: u32 = 0x006d_d010;
/// `ECX` = location, stack (position, navmesh info): constructs a location
/// at a navmesh's point.
const LOCATION_CONSTRUCT_FROM_POINT_AND_INFO: u32 = 0x006d_cf20;
/// `ECX` = location, stack (position, other location): constructs a
/// location at a point, taking the rest from the other location.
const LOCATION_CONSTRUCT_FROM_POINT_AND_LOCATION: u32 = 0x006d_d0d0;
/// `ECX` = location: `PathingLocation::PathingLocation` (Xbox PDB).
const LOCATION_CONSTRUCT: u32 = 0x006d_cbb0;
/// `ECX` = location: its destructor.
const LOCATION_DESTRUCT: u32 = 0x004f_f7e0;
/// `ECX` = cell, stack (position, out height): `TESObjectCELL::
/// GetLandHeight` (Xbox PDB); returns whether the cell has land.
const CELL_GET_LAND_HEIGHT: u32 = 0x0055_47c0;
/// `ECX` = reference: `TESObjectREFR::GetScale` (Xbox PDB), in `ST0`.
const REFERENCE_GET_SCALE: u32 = 0x0056_7400;

// Solutions, nodes and arrays.

/// `ECX` = array, stack (): the element count of a `BSSimpleArray` (+8).
const ARRAY_COUNT: u32 = 0x0044_ddc0;
/// `ECX` = array: whether its element count (+8) is zero.
const ARRAY_IS_EMPTY: u32 = 0x0076_b610;
/// `ECX` = `BSSimpleArray`: reserves a slot, size + 1, returns the old
/// size.
const ARRAY_ADD_SLOT: u32 = 0x0097_8bc0;
/// `ECX` = array, stack (slot, count): constructs `count` 12-byte elements
/// from `slot` on.
const ARRAY_CONSTRUCT_ELEMENTS: u32 = 0x0049_f8f0;
/// `ECX` = array of 0x14-byte search nodes, stack (index): the address of
/// the element.
const NODE_ARRAY_ELEMENT: u32 = 0x006b_3160;
/// `ECX` = array, stack (0x20, 0, 0): constructs the first kind of scrap
/// node array.
const NODE_ARRAY_A_CONSTRUCT: u32 = 0x006a_d1b0;
/// `ECX` = array: destructor of the first kind.
const NODE_ARRAY_A_DESTRUCT: u32 = 0x006a_bc50;
/// `ECX` = array, stack (0x20, 0, 0): constructs the second kind.
const NODE_ARRAY_B_CONSTRUCT: u32 = 0x006a_d250;
/// `ECX` = array: destructor of the second kind.
const NODE_ARRAY_B_DESTRUCT: u32 = 0x006a_bde0;
/// `ECX` = array, stack (0x20, 0, 0): constructs the third kind.
const NODE_ARRAY_C_CONSTRUCT: u32 = 0x006d_b290;
/// `ECX` = array: destructor of the third kind.
const NODE_ARRAY_C_DESTRUCT: u32 = 0x006d_b350;
/// `ECX` = virtual node array: reserves a slot, returns its index.
const VIRTUAL_NODE_ARRAY_ADD_SLOT: u32 = 0x006c_ef10;
/// `ECX` = virtual node array, stack (index): the address of the 0x30-byte
/// element.
const VIRTUAL_NODE_ARRAY_ELEMENT: u32 = 0x006e_8f40;
/// `cdecl (size, block)`: placement `new`, returns the block.
const PLACEMENT_NEW: u32 = 0x006e_6da0;
/// `ECX` = virtual node, stack (location): constructs it from a location.
const VIRTUAL_NODE_CONSTRUCT: u32 = 0x006f_4d00;
/// `ECX` = virtual node, stack (location): `VirtualPathingNode::
/// SetLocation` (Xbox PDB).
const VIRTUAL_NODE_SET_LOCATION: u32 = 0x006f_5060;
/// `ECX` = solution, stack (position, navmesh info): appends a virtual node
/// for the point, returns its index.
const SOLUTION_ADD_VIRTUAL_NODE: u32 = 0x006c_99c0;
/// `ECX` = solution, stack (flag): stores `bIncompletePath` (+0x40).
const SOLUTION_SET_INCOMPLETE: u32 = 0x006c_9fa0;
/// `ECX` = solution: the number of current pathing nodes (the count of the
/// array at +0x20).
const SOLUTION_NODE_COUNT: u32 = 0x008b_6800;
/// `ECX` = solution: the number of virtual nodes (the count of the array
/// at +8).
const SOLUTION_VIRTUAL_NODE_COUNT: u32 = 0x005a_4320;
/// `ECX` = solution, stack (index): the address of the current pathing
/// node `index`.
const SOLUTION_NODE_AT: u32 = 0x006e_7970;
/// `ECX` = solution, stack (index): the address of the virtual node
/// `index`.
const SOLUTION_VIRTUAL_NODE_AT: u32 = 0x006e_78b0;
/// `ECX` = solution: clears it (virtual node indices to -1, incomplete flag
/// to 0, arrays emptied).
const SOLUTION_CLEAR: u32 = 0x006e_77b0;
/// `ECX` = solution (to construct).
const SOLUTION_CONSTRUCT: u32 = 0x006e_7650;
/// `ECX` = solution, stack (other solution): copies the indices, the
/// incomplete flag and the node arrays to the other solution.
const SOLUTION_COPY_TO: u32 = 0x006e_8ab0;
/// `ECX` = solution: its destructor.
const SOLUTION_DESTRUCT: u32 = 0x006e_7720;
/// `ECX` = pathing node: the address of its location (+4).
const NODE_LOCATION: u32 = 0x0071_7e50;
/// `ECX` = pathing node, stack (position): stores the position at +0x2C.
const NODE_SET_POSITION: u32 = 0x006b_0570;
/// `ECX` = search node (copied onto the stack), stack (out): constructs the
/// point (words 2 to 4 of the node) in `out`, returns `out`.
const NODE_TO_POSITION: u32 = 0x006a_6eb0;
/// `ECX` = avoid-node array, stack (position, out): the sum of the pushes
/// the avoid nodes give at the position, in `out`; a weight in `ST0`.
const AVOID_ARRAY_COMPUTE_PUSH: u32 = 0x006d_c9f0;
/// `cdecl (out, heading, location a, location b, radius)`: builds the
/// tangent point of the path's first node.
const BUILD_FIRST_TANGENT: u32 = 0x006d_99a0;
/// `cdecl (worldspace, cell, point, out)`: `Pathing::
/// FindClosestPointOnNavmesh` (engine map), returns whether it found one.
const FIND_CLOSEST_POINT_ON_NAVMESH: u32 = 0x006d_6f80;
/// `cdecl (location, radius, out)`: finds a point for the location (the
/// found point's position is at +0xC of `out`), returns whether it found
/// one.
const FIND_PATH_POINT: u32 = 0x006d_4570;
/// `ECX` = path point: constructor.
const PATH_POINT_CONSTRUCT: u32 = 0x006a_0480;

// Solvers' objects.

/// `ECX` = solver, stack (request, solution): constructs the base solver
/// (0x34 bytes).
const BASE_SOLVER_CONSTRUCT: u32 = 0x006c_8d30;
/// `ECX` = solver: runs it, returns whether it found a path.
const BASE_SOLVER_RUN: u32 = 0x006c_8e10;
/// `ECX` = navmesh list: constructor.
const NAVMESH_LIST_CONSTRUCT: u32 = 0x0042_f800;
/// `ECX` = navmesh list: destructor.
const NAVMESH_LIST_DESTRUCT: u32 = 0x0042_f830;
/// `ECX` = the base solver's member at +0x14: destructor.
const BASE_SOLVER_MEMBER_DESTRUCT: u32 = 0x005e_0540;
/// `cdecl (navmesh list)`: fills the list.
const NAVMESH_LIST_FILL: u32 = 0x006d_91d0;
/// `ECX` = `NavMeshSearchFlee`: constructor.
const FLEE_SEARCH_CONSTRUCT: u32 = 0x006a_9200;
/// `ECX` = flee search: destructor.
const FLEE_SEARCH_DESTRUCT: u32 = 0x006a_92c0;
/// `ECX` = flee search, stack (request, start, path nodes, goal nodes):
/// `NavMeshSearchFlee::BuildNodePath` (Xbox PDB).
const FLEE_SEARCH_BUILD_NODE_PATH: u32 = 0x006a_92e0;
/// `ECX` = path builder: constructor.
const PATH_BUILDER_CONSTRUCT: u32 = 0x006a_d4c0;
/// `ECX` = path builder: destructor.
const PATH_BUILDER_DESTRUCT: u32 = 0x006a_d690;
/// `ECX` = path builder, stack (request, path nodes, goal nodes, start
/// position, end position, solution, 0, 0): builds the solution, returns
/// whether it succeeded.
const PATH_BUILDER_BUILD: u32 = 0x006a_d770;
/// `ECX` = `NavMeshSearchClosePoint`: constructor.
const CLOSE_POINT_SEARCH_CONSTRUCT: u32 = 0x006a_9cb0;
/// `ECX` = close-point search: destructor.
const CLOSE_POINT_SEARCH_DESTRUCT: u32 = 0x006a_9d60;
/// `ECX` = close-point search, stack (request, start, path nodes, goal
/// nodes): builds the node path.
const CLOSE_POINT_SEARCH_BUILD_NODE_PATH: u32 = 0x006a_9d80;
/// `ECX` = `PathSmoother`: constructor.
const PATH_SMOOTHER_CONSTRUCT: u32 = 0x006e_d100;
/// `ECX` = path smoother, stack (request, path nodes, goal nodes, start
/// position, end position, solution, 0, 1): smooths the node path into the
/// solution.
const PATH_SMOOTHER_SMOOTH: u32 = 0x006e_d530;
/// `ECX` = path smoother: `PathSmoother::~PathSmoother` (Xbox PDB).
const PATH_SMOOTHER_DESTRUCT: u32 = 0x006e_d220;
/// `ECX` = `NavMeshSearchLOS`: constructor.
const LOS_SEARCH_CONSTRUCT: u32 = 0x006a_a620;
/// `ECX` = LOS search: destructor.
const LOS_SEARCH_DESTRUCT: u32 = 0x006a_a710;
/// `ECX` = LOS search, stack (request, start, path nodes, goal nodes):
/// `NavMeshSearchLOS::BuildNodePath` (Xbox PDB).
const LOS_SEARCH_BUILD_NODE_PATH: u32 = 0x006a_a780;
/// `ECX` = LOS search, stack (node, path nodes, goal nodes): fills the node
/// arrays from a node the search recorded, returns whether it found a
/// path.
const LOS_SEARCH_PATH_FROM_NODE: u32 = 0x006a_8290;
/// `ECX` = LOS map, stack (holder, triangle `u16` pointer): finds the
/// triangle the map points at, returns whether it found one.
const LOS_MAP_FIND_TRIANGLE: u32 = 0x006e_1770;
/// `ECX` = `NavMeshSearchHide`: constructor.
const HIDE_SEARCH_CONSTRUCT: u32 = 0x006a_c0e0;
/// `ECX` = hide search: destructor.
const HIDE_SEARCH_DESTRUCT: u32 = 0x006a_c1b0;
/// `ECX` = hide search, stack (request, origin start, destination start,
/// path nodes, goal nodes): builds the node path.
const HIDE_SEARCH_BUILD_NODE_PATH: u32 = 0x006a_c340;
/// `ECX` = `NavMeshSearchMaxCost`: constructor.
const MAX_COST_SEARCH_CONSTRUCT: u32 = 0x006a_b410;
/// `ECX` = max-cost search: destructor.
const MAX_COST_SEARCH_DESTRUCT: u32 = 0x006a_b480;
/// `ECX` = max-cost search, stack (request, start, goal location, path
/// nodes, goal nodes): `NavMeshSearchMaxCost::BuildNodePath` (Xbox PDB).
const MAX_COST_SEARCH_BUILD_NODE_PATH: u32 = 0x006a_b4a0;
/// `ECX` = max-cost search: the pointer at +0x2048 (the best node so far).
const MAX_COST_SEARCH_BEST_NODE: u32 = 0x006a_bb90;
/// `ECX` = the search `fn_006d34d0` runs: constructor.
const RANDOM_POINT_SEARCH_CONSTRUCT: u32 = 0x006a_8a80;
/// `ECX` = the same search: destructor.
const RANDOM_POINT_SEARCH_DESTRUCT: u32 = 0x006a_8b40;
/// `ECX` = the same search, stack (request, start, path nodes, goal nodes):
/// builds the node path.
const RANDOM_POINT_SEARCH_BUILD_NODE_PATH: u32 = 0x006a_8b60;
/// `ECX` = time stamp, stack (value): stores the value (`CombatTimeStamp`
/// constructor).
const TIME_STAMP_CONSTRUCT: u32 = 0x0043_5de0;
/// `ECX` = array: constructs the `BSSimpleArray<PathingCoverLocation>`.
const COVER_LOCATION_ARRAY_CONSTRUCT: u32 = 0x006d_ae60;

// Data.

/// The vtable `fn_006d3200` stores in a `PathingCoverLocation`.
const PATHING_COVER_LOCATION_VTABLE: u32 = 0x0106_cbc0;
/// The zero vector the game copies into fresh `NiPoint3`s.
const ZERO_VECTOR: u32 = 0x011f_426c;
/// The three angle settings `Pathing::Init` converts (setting objects).
const SETTING_ANGLE_A: u32 = 0x011f_2ba8;
const SETTING_ANGLE_B: u32 = 0x011f_2af8;
const SETTING_ANGLE_C: u32 = 0x011f_2b04;
/// The three converted angles `Pathing::Init` stores (`float`s).
const CONVERTED_ANGLE_A: u32 = 0x011a_5cf4;
const CONVERTED_ANGLE_B: u32 = 0x011a_5cf8;
const CONVERTED_ANGLE_C: u32 = 0x011a_5cfc;
/// `double` `pi / 180`.
const DEGREES_TO_RADIANS: u32 = 0x0102_3128;
/// `double` scale of a random unsigned number to an angle range.
const RANDOM_ANGLE_SCALE: u32 = 0x0101_e5a0;
/// `double` `pi` (as `float`), subtracted from the scaled random number.
const PI_AS_FLOAT: u32 = 0x0101_ff40;
/// `double` `2.0`.
const TWO: u32 = 0x0101_1590;
/// `double` `0.5`.
const HALF: u32 = 0x0101_1588;
/// `double` `0.9`.
const NINE_TENTHS: u32 = 0x0106_b9e8;
/// `float` `32.0`: the least reference size of the safe-straight-line
/// request.
const MINIMUM_RADIUS: u32 = 0x0101_e340;
/// `float` `512.0`: the distance `fn_006d2c80` asks the covered-move check
/// about.
const COVERED_MOVE_DISTANCE: u32 = 0x0102_31a0;
/// `float` `FLT_MAX`: `fn_006d32e0` stores its negation as the time stamp.
const FLOAT_MAXIMUM: u32 = 0x0106_cb30;
/// Global unit vectors `fn_006d12a0` scales to get a point on a circle.
const CIRCLE_AXIS_A: u32 = 0x011a_9478;
const CIRCLE_AXIS_B: u32 = 0x011a_946c;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Copies `count` words from `source` to `destination`.
fn copy_words(e: &mut Engine, destination: u32, source: u32, count: u32) {
    for i in 0..count {
        let word = e.mem.u32(source + 4 * i);
        e.mem.set_u32(destination + 4 * i, word);
    }
}

/// Copies the five-word search node at `source` (a path node: navmesh
/// pointer, triangle in the low half of the second word, and three more
/// words) to `destination`.
fn copy_search_node(e: &mut Engine, destination: u32, source: u32) {
    copy_words(e, destination, source, 5);
}

/// The last element of a node array (`array[count - 1]`).
fn last_node(e: &mut Engine, array: u32) -> u32 {
    let count = e.call(ARRAY_COUNT, &args![array]).u32();
    e.call(NODE_ARRAY_ELEMENT, &args![array, count.wrapping_sub(1)])
        .u32()
}

// ---------------------------------------------------------------------------
// Functions
// ---------------------------------------------------------------------------

// Translated from 0049f210 (decompiled, FalloutNV.exe 1.4.0.525)
/// `BSSimpleArray<ParentSpaceNode,1024>::Add` (Xbox PDB): reserves a new
/// slot (`00978bc0` bumps the size and returns the old size), constructs the
/// 12-byte element in it (`0049f8f0`), copies `item` over it and returns the
/// index. The buffer is read again after the construction because it can
/// have been reallocated.
pub fn bs_simple_array_parent_space_node_add(
    e: &mut Engine,
    this: Ptr<crate::types::BSSimpleArray>,
    item: Ptr,
) -> i32 {
    let index = e.call(ARRAY_ADD_SLOT, &args![this]).u32();
    let buffer = e.get(this, crate::types::BSSimpleArray::pBuffer);
    let slot = index.wrapping_mul(12).wrapping_add(buffer);
    e.call(ARRAY_CONSTRUCT_ELEMENTS, &args![this, slot, 1u32]);
    let buffer = e.get(this, crate::types::BSSimpleArray::pBuffer);
    let destination = buffer.wrapping_add(index.wrapping_mul(12));
    copy_words(e, destination, item.addr(), 3);
    index as i32
}

// Translated from 006d0870 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Pathing::Init` (Xbox PDB): converts three angle settings from degrees to
/// radians, applies the angle function (`005b9e80`) to each and stores the
/// results in three globals. Always returns true.
pub fn pathing_init(e: &mut Engine) -> bool {
    let degrees_to_radians = e.global::<f64>(DEGREES_TO_RADIANS);
    for (setting, destination) in [
        (SETTING_ANGLE_A, CONVERTED_ANGLE_A),
        (SETTING_ANGLE_B, CONVERTED_ANGLE_B),
        (SETTING_ANGLE_C, CONVERTED_ANGLE_C),
    ] {
        let value_address = e.call(SETTING_FLOAT_ADDRESS, &args![setting]).u32();
        let degrees = e.mem.f32(value_address);
        let radians = (degrees as f64 * degrees_to_radians) as f32;
        let result = e.call(ANGLE_FUNCTION_005B9E80, &args![radians]).f32();
        e.set_global(destination, result);
    }
    true
}

// Translated from 006d0900 (decompiled, FalloutNV.exe 1.4.0.525)
/// Solves a `PathingRequest` into a `PathingSolution`: dispatches on the
/// request's `GetType` (virtual slot +0x10) to the solver of that type
/// (0 base, 3 flee, 4 close point, 5 line of sight, 6 hide, 7 optimal
/// location, 8 covered move: named after the search and request classes of
/// the Xbox PDB each solver uses; types 1, 2 and anything above 8 do
/// nothing and fail). Runs inside a scope timer for source line 0x6a.
pub fn fn_006d0900(
    e: &mut Engine,
    request: Ptr<PathingRequest>,
    solution: Ptr<PathingSolution>,
) -> bool {
    e.with_stack(0x10, |e, timer| {
        let timer = timer.addr();
        scope_enter(e, timer, 0x6a);
        let kind = e.vcall(request.addr(), 0x10, &args![]).u32();
        let found = match kind {
            0 => fn_006d0b10(e, request, solution),
            3 => fn_006d0be0(e, request, solution),
            4 => fn_006d1c60(e, request, solution),
            5 => fn_006d1660(e, request, solution),
            6 => fn_006d1f80(e, request, solution),
            7 => fn_006d23d0(e, request, solution),
            8 => fn_006d2c80(e, request, solution),
            _ => false,
        };
        scope_leave(e, timer);
        found
    })
}

// Translated from 006d0b10 (decompiled, FalloutNV.exe 1.4.0.525)
/// Solver for a type-0 request: builds the 0x34-byte base solver on the
/// stack for the request and the solution (`006c8d30`), runs it (`006c8e10`)
/// and destroys it (`fn_006d0b80`). Returns what the run returned.
pub fn fn_006d0b10(
    e: &mut Engine,
    request: Ptr<PathingRequest>,
    solution: Ptr<PathingSolution>,
) -> bool {
    with_frame(e, 0x38, |e, f| {
        let solver = f.at(-0x40);
        e.call(BASE_SOLVER_CONSTRUCT, &args![solver, request, solution]);
        let found = e.call(BASE_SOLVER_RUN, &args![solver]).bool();
        fn_006d0b80(e, Ptr::new(solver));
        found
    })
}

// Translated from 006d0b80 (decompiled, FalloutNV.exe 1.4.0.525)
/// Destructor of the base solver: destroys its members at +0x24 and +0x14,
/// in that order.
pub fn fn_006d0b80(e: &mut Engine, this: Ptr) {
    e.call(NAVMESH_LIST_DESTRUCT, &args![this.addr() + 0x24]);
    e.call(BASE_SOLVER_MEMBER_DESTRUCT, &args![this.addr() + 0x14]);
}

// Translated from 006d0be0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Solver for a type-3 request (flee): resolves the origin to a navmesh and
/// triangle (`fn_006d12a0` is the fallback whenever that fails: it picks a
/// point to run to without a search), runs `NavMeshSearchFlee::
/// BuildNodePath`, takes the center of the last triangle as the goal (when
/// `006d4570` finds a point for it and the goal is closer to that point than
/// the request's radius, the goal is pushed out to exactly that radius from
/// it) and builds the solution with the path builder.
///
/// When the destination was not resolved (flag 2 of its flags) the
/// destination is resolved and its navmesh fetched first. When the final
/// path has more than two nodes and the request asks for it
/// (`bFirstTangentUsesHeading`), the first tangent of the first node is set
/// from the request's heading.
pub fn fn_006d0be0(
    e: &mut Engine,
    request: Ptr<PathingRequest>,
    solution: Ptr<PathingSolution>,
) -> bool {
    with_frame(e, 0x4310, |e, f| {
        let timer = f.at(-0x20ec);
        let origin_holder = f.at(-0x20a0);
        let origin_triangle = f.at(-0x20f4);
        scope_enter(e, timer, 0xb3);
        let origin = e.call(REQUEST_ORIGIN, &args![request]).u32();
        if !e.call(LOCATION_RESOLVE_CLOSEST, &args![origin]).bool() {
            let found = fn_006d12a0(e, request, solution);
            scope_leave(e, timer);
            return found;
        }
        e.call(NAV_HOLDER_CONSTRUCT, &args![origin_holder]);
        e.mem.set_u16(origin_triangle, 0xffff);
        let origin = e.call(REQUEST_ORIGIN, &args![request]).u32();
        if !e
            .call(
                LOCATION_GET_NAVMESH_AND_TRIANGLE,
                &args![origin, origin_holder, origin_triangle],
            )
            .bool()
        {
            let found = fn_006d12a0(e, request, solution);
            e.call(NAV_HOLDER_RELEASE, &args![origin_holder]);
            scope_leave(e, timer);
            return found;
        }

        // The destination, unless it already carries navmesh info.
        let destination = e.call(REQUEST_DESTINATION, &args![request]).u32();
        if !e.call(LOCATION_HAS_FLAG_2, &args![destination]).bool() {
            let destination = e.call(REQUEST_DESTINATION, &args![request]).u32();
            if !e
                .call(LOCATION_RESOLVE_NAVMESH_INFO, &args![destination, 0u32])
                .bool()
            {
                let found = fn_006d12a0(e, request, solution);
                e.call(NAV_HOLDER_RELEASE, &args![origin_holder]);
                scope_leave(e, timer);
                return found;
            }
            let destination_holder = f.at(-0x4280);
            let destination_triangle = f.at(-0x4284);
            e.call(NAV_HOLDER_CONSTRUCT, &args![destination_holder]);
            e.mem.set_u16(destination_triangle, 0xffff);
            let destination = e.call(REQUEST_DESTINATION, &args![request]).u32();
            if !e
                .call(
                    LOCATION_GET_NAVMESH_AND_TRIANGLE,
                    &args![destination, destination_holder, destination_triangle],
                )
                .bool()
            {
                let found = fn_006d12a0(e, request, solution);
                e.call(NAV_HOLDER_RELEASE, &args![destination_holder]);
                e.call(NAV_HOLDER_RELEASE, &args![origin_holder]);
                scope_leave(e, timer);
                return found;
            }
            e.call(NAV_HOLDER_RELEASE, &args![destination_holder]);
        }

        // The start: the origin's navmesh and triangle.
        let start = f.at(-0x20e8);
        let navmesh = e.call(NAV_HOLDER_GET, &args![origin_holder]).u32();
        e.mem.set_u32(start, navmesh);
        let triangle = e.mem.u16(origin_triangle);
        e.mem.set_u16(start + 4, triangle);
        let start_position = f.at(-0x20ac);
        let origin = e.call(REQUEST_ORIGIN, &args![request]).u32();
        e.call(LOCATION_COPY_POSITION, &args![origin, start_position]);

        let search = f.at(-0x209c);
        e.call(FLEE_SEARCH_CONSTRUCT, &args![search]);
        let path_nodes = f.at(-0x2130);
        e.call(
            NODE_ARRAY_A_CONSTRUCT,
            &args![path_nodes, 0x20u32, 0u32, 0u32],
        );
        let goal_nodes = f.at(-0x20d4);
        e.call(
            NODE_ARRAY_B_CONSTRUCT,
            &args![goal_nodes, 0x20u32, 0u32, 0u32],
        );
        let built = e
            .call(
                FLEE_SEARCH_BUILD_NODE_PATH,
                &args![search, request, start, path_nodes, goal_nodes],
            )
            .bool();
        if !built {
            e.call(NODE_ARRAY_B_DESTRUCT, &args![goal_nodes]);
            e.call(NODE_ARRAY_A_DESTRUCT, &args![path_nodes]);
            e.call(FLEE_SEARCH_DESTRUCT, &args![search]);
            e.call(NAV_HOLDER_RELEASE, &args![origin_holder]);
            scope_leave(e, timer);
            return false;
        }

        // The goal: the center of the last triangle of the path.
        let last = f.at(-0x20c0);
        let element = last_node(e, path_nodes);
        copy_search_node(e, last, element);
        let goal = f.at(-0x427c);
        let last_navmesh = e.mem.u32(last);
        let last_triangle = e.mem.u16(last + 4) as u32;
        e.call(
            NAVMESH_GET_CENTER,
            &args![last_navmesh, goal, last_triangle],
        );
        let info = e.call(NAVMESH_GET_INFO, &args![last_navmesh]).u32();
        let goal_location = f.at(-0x211c);
        e.call(
            LOCATION_CONSTRUCT_FROM_POINT_AND_INFO,
            &args![goal_location, goal, info],
        );
        let found_point = f.at(-0x2148);
        e.call(PATH_POINT_CONSTRUCT, &args![found_point]);
        let radius = e.call(REQUEST_ACTOR_RADIUS, &args![request]).f32();
        if e.call(FIND_PATH_POINT, &args![goal_location, radius, found_point])
            .bool()
        {
            // Pull the goal to the request's radius from the found point.
            let offset = f.at(-0x4294);
            let found_position = found_point + 0xc;
            e.call(POINT3_SUBTRACT, &args![goal, offset, found_position]);
            let length = e.call(POINT3_UNITIZE_GET_LENGTH, &args![offset]).f32();
            let radius = e.call(REQUEST_ACTOR_RADIUS, &args![request]).f32();
            if length < radius {
                let radius = e.call(REQUEST_ACTOR_RADIUS, &args![request]).f32();
                let scaled_result = f.at(-0x430c);
                let scaled = e
                    .call(POINT3_TIMES_SCALAR, &args![offset, scaled_result, radius])
                    .u32();
                let sum_result = f.at(-0x4318);
                let sum = e
                    .call(POINT3_ADD, &args![found_position, sum_result, scaled])
                    .u32();
                copy_words(e, goal, sum, 3);
            }
        }

        let start_navmesh = e.mem.u32(start);
        let info = e.call(NAVMESH_GET_INFO, &args![start_navmesh]).u32();
        e.call(
            SOLUTION_ADD_VIRTUAL_NODE,
            &args![solution, start_position, info],
        );
        let info = e.call(NAVMESH_GET_INFO, &args![last_navmesh]).u32();
        e.call(SOLUTION_ADD_VIRTUAL_NODE, &args![solution, goal, info]);
        e.set(solution, PathingSolution::iFirstLoadedVirtualNodeIndex, 0);
        e.set(solution, PathingSolution::iLastLoadedVirtualNodeIndex, 1);

        let builder = f.at(-0x425c);
        e.call(PATH_BUILDER_CONSTRUCT, &args![builder]);
        let extra_nodes = f.at(-0x4270);
        e.call(
            NODE_ARRAY_C_CONSTRUCT,
            &args![extra_nodes, 0x20u32, 0u32, 0u32],
        );
        let found = e
            .call(
                PATH_BUILDER_BUILD,
                &args![
                    builder,
                    request,
                    path_nodes,
                    goal_nodes,
                    start_position,
                    goal,
                    solution,
                    0u32,
                    0u32
                ],
            )
            .bool();
        if !found {
            e.call(SOLUTION_SET_INCOMPLETE, &args![solution, 1u32]);
        } else if e
            .call(REQUEST_FIRST_TANGENT_USES_HEADING, &args![request])
            .bool()
            && e.call(SOLUTION_NODE_COUNT, &args![solution]).u32() > 2
        {
            let first_node = e.call(SOLUTION_NODE_AT, &args![solution, 0u32]).u32();
            let second_node = e.call(SOLUTION_NODE_AT, &args![solution, 1u32]).u32();
            if first_node != 0 && second_node != 0 {
                let first_location = e.call(NODE_LOCATION, &args![first_node]).u32();
                let first_copy = f.at(-0x42f8);
                e.call(LOCATION_COPY_CONSTRUCT, &args![first_copy, first_location]);
                let second_location = e.call(NODE_LOCATION, &args![second_node]).u32();
                let second_copy = f.at(-0x42c4);
                e.call(
                    LOCATION_COPY_CONSTRUCT,
                    &args![second_copy, second_location],
                );
                // The radius is read for the tangent builder's last word.
                let radius = e.call(REQUEST_ACTOR_RADIUS, &args![request]).f32();
                let heading = e.call(REQUEST_INITIAL_PATH_HEADING, &args![request]).f32();
                let tangent = f.at(-0x42d0);
                e.call(
                    BUILD_FIRST_TANGENT,
                    &args![tangent, heading, first_copy, second_copy, radius],
                );
                e.call(NODE_SET_POSITION, &args![first_node, tangent]);
                e.call(LOCATION_DESTRUCT, &args![second_copy]);
                e.call(LOCATION_DESTRUCT, &args![first_copy]);
            }
        }

        e.call(NODE_ARRAY_C_DESTRUCT, &args![extra_nodes]);
        e.call(PATH_BUILDER_DESTRUCT, &args![builder]);
        e.call(LOCATION_DESTRUCT, &args![goal_location]);
        e.call(NODE_ARRAY_B_DESTRUCT, &args![goal_nodes]);
        e.call(NODE_ARRAY_A_DESTRUCT, &args![path_nodes]);
        e.call(FLEE_SEARCH_DESTRUCT, &args![search]);
        e.call(NAV_HOLDER_RELEASE, &args![origin_holder]);
        scope_leave(e, timer);
        found
    })
}

// Translated from 006d12a0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Flee without a search: picks a point `fMinFleeDistance` (the float at
/// +0xB0 of the request) away from the origin and adds the origin and that
/// point as the solution's two virtual nodes. Always returns true.
///
/// When the request has a non-empty avoid-node array the point lies in the
/// direction away from the avoid nodes (`origin - push`, normalized); else
/// it is a random point on the circle around the origin spanned by the two
/// global axes. If the origin has a cell, the point's height is replaced by
/// the land height there. Both points are appended with `fn_006d1520` and the
/// solution's loaded node range is set to 0..1.
pub fn fn_006d12a0(
    e: &mut Engine,
    request: Ptr<PathingRequest>,
    solution: Ptr<PathingSolution>,
) -> bool {
    with_frame(e, 0x94, |e, f| {
        let timer = f.at(-0x3c);
        scope_enter(e, timer, 0x12f);
        let position = f.at(-0x2c);
        let origin = e.call(REQUEST_ORIGIN, &args![request]).u32();
        e.call(LOCATION_COPY_POSITION, &args![origin, position]);
        let origin = e.call(REQUEST_ORIGIN, &args![request]).u32();
        let cell = e.call(LOCATION_GET_CELL, &args![origin]).u32();
        let origin = e.call(REQUEST_ORIGIN, &args![request]).u32();
        let worldspace = e.call(LOCATION_GET_WORLDSPACE, &args![origin]).u32();
        let push = f.at(-0x38);
        e.call(POINT3_DEFAULT_CONSTRUCTOR, &args![push]);
        let target = f.at(-0x1c);
        e.call(POINT3_DEFAULT_CONSTRUCTOR, &args![target]);

        let avoid = e.call(REQUEST_AVOID_NODE_ARRAY, &args![request]).u32();
        let has_avoid_nodes = avoid != 0 && {
            let avoid = e.call(REQUEST_AVOID_NODE_ARRAY, &args![request]).u32();
            !e.call(ARRAY_IS_EMPTY, &args![avoid]).bool()
        };
        if has_avoid_nodes {
            // Away from the avoid nodes: normalized (origin - push) times the
            // minimum distance, from the origin.
            let avoid = e.call(REQUEST_AVOID_NODE_ARRAY, &args![request]).u32();
            e.call(AVOID_ARRAY_COMPUTE_PUSH, &args![avoid, position, push]);
            let away = f.at(-0x54);
            let result = e.call(POINT3_SUBTRACT, &args![position, away, push]).u32();
            copy_words(e, target, result, 3);
            e.call(POINT3_NORMALIZE, &args![target]);
            let distance = fn_006d2c20(e, request.cast());
            e.call(POINT3_SCALE_IN_PLACE, &args![target, distance]);
            e.call(POINT3_ADD_ASSIGN, &args![target, position]);
        } else {
            copy_words(e, push, position, 3);
            let angle = fn_006d1600(e);
            let first_scale = e.call(ANGLE_FUNCTION_004E44B0, &args![angle]).f64();
            let distance = fn_006d2c20(e, request.cast()) as f64;
            let first_scale = (distance * first_scale) as f32;
            let first = f.at(-0x78);
            let first_axis = e
                .call(
                    POINT3_SCALED_COPY,
                    &args![first, first_scale, CIRCLE_AXIS_A],
                )
                .u32();
            let second_scale = e.call(ANGLE_FUNCTION_004E4470, &args![angle]).f64();
            let distance = fn_006d2c20(e, request.cast()) as f64;
            let second_scale = (distance * second_scale) as f32;
            let second = f.at(-0x60);
            let second_axis = e
                .call(
                    POINT3_SCALED_COPY,
                    &args![second, second_scale, CIRCLE_AXIS_B],
                )
                .u32();
            let partial = f.at(-0x6c);
            let partial_result = e.call(POINT3_ADD, &args![push, partial, second_axis]).u32();
            let total = f.at(-0x84);
            let result = e
                .call(POINT3_ADD, &args![partial_result, total, first_axis])
                .u32();
            copy_words(e, target, result, 3);
        }

        if cell != 0 {
            let height = f.at(-0x48);
            if e.call(CELL_GET_LAND_HEIGHT, &args![cell, target, height])
                .bool()
            {
                let height = e.mem.f32(height);
                e.mem.set_f32(target + 8, height);
            }
        }
        fn_006d1520(e, solution, Ptr::new(position), cell, worldspace);
        fn_006d1520(e, solution, Ptr::new(target), cell, worldspace);
        e.set(solution, PathingSolution::iFirstLoadedVirtualNodeIndex, 0);
        e.set(solution, PathingSolution::iLastLoadedVirtualNodeIndex, 1);
        scope_leave(e, timer);
        true
    })
}

// Translated from 006d1520 (decompiled, FalloutNV.exe 1.4.0.525)
/// Appends a virtual node for a point to the solution: builds a
/// `PathingLocation` from the position, cell and worldspace
/// (`PathingLocation_ov7`), resolves its navmesh info, reserves a slot in
/// the solution's virtual node array (+8), constructs the 0x30-byte node
/// there from the location and returns the slot's index.
pub fn fn_006d1520(
    e: &mut Engine,
    this: Ptr<PathingSolution>,
    position: Ptr,
    cell: u32,
    worldspace: u32,
) -> u32 {
    with_frame(e, 0x44, |e, f| {
        let location = f.at(-0x3c);
        e.call(
            LOCATION_CONSTRUCT_FROM_PARTS,
            &args![location, position, cell, worldspace],
        );
        e.call(LOCATION_RESOLVE_NAVMESH_INFO, &args![location, 0u32]);
        let array = this.addr() + 8;
        let index = e.call(VIRTUAL_NODE_ARRAY_ADD_SLOT, &args![array]).u32();
        let element = e
            .call(VIRTUAL_NODE_ARRAY_ELEMENT, &args![array, index])
            .u32();
        let block = e.call(PLACEMENT_NEW, &args![0x30u32, element]).u32();
        if block != 0 {
            e.call(VIRTUAL_NODE_CONSTRUCT, &args![block, location]);
        }
        e.call(LOCATION_DESTRUCT, &args![location]);
        index
    })
}

// Translated from 006d1600 (decompiled, FalloutNV.exe 1.4.0.525)
/// A random angle: `fn_006d1620` on the global random generator.
pub fn fn_006d1600(e: &mut Engine) -> f32 {
    let random = e.call(RANDOM_OBJECT, &args![]).u32();
    fn_006d1620(e, Ptr::new(random))
}

// Translated from 006d1620 (decompiled, FalloutNV.exe 1.4.0.525)
/// A random angle in `[-pi, pi)`: a random unsigned number times the scale
/// at `0101e5a0`, minus `pi` (`0101ff40`), as a `float`.
pub fn fn_006d1620(e: &mut Engine, this: Ptr) -> f32 {
    let random = e.call(RANDOM_UNSIGNED, &args![this]).u32();
    let scale = e.global::<f64>(RANDOM_ANGLE_SCALE);
    let pi = e.global::<f64>(PI_AS_FLOAT);
    (random as f64 * scale - pi) as f32
}

// Translated from 006d1660 (decompiled, FalloutNV.exe 1.4.0.525)
/// Solver for a type-5 request (line of sight): resolves the origin to a
/// navmesh and triangle (failing if it cannot), runs `NavMeshSearchLOS::
/// BuildNodePath`, and when that finds no path tries the search's three
/// fallback nodes in turn (the first ignored goal, the last valid node and
/// the closest node, each through `006a8290`), falling back to a failed
/// result when none gives a path.
///
/// On success the center of the path's last triangle becomes a copy of the
/// request's destination and the copy is solved again through
/// `fn_006d0900`. When that does not succeed but a fallback node was found,
/// the last virtual node of the solution is replaced by the point on the
/// LOS map's triangle.
pub fn fn_006d1660(
    e: &mut Engine,
    request: Ptr<PathingRequest>,
    solution: Ptr<PathingSolution>,
) -> bool {
    with_frame(e, 0x2244, |e, f| {
        let timer = f.at(-0x20f0);
        let holder = f.at(-0x20b0);
        let triangle = f.at(-0x20f8);
        scope_enter(e, timer, 0x17a);
        let origin = e.call(REQUEST_ORIGIN, &args![request]).u32();
        if !e.call(LOCATION_RESOLVE_CLOSEST, &args![origin]).bool() {
            scope_leave(e, timer);
            return false;
        }
        e.call(NAV_HOLDER_CONSTRUCT, &args![holder]);
        e.mem.set_u16(triangle, 0xffff);
        let origin = e.call(REQUEST_ORIGIN, &args![request]).u32();
        if !e
            .call(
                LOCATION_GET_NAVMESH_AND_TRIANGLE,
                &args![origin, holder, triangle],
            )
            .bool()
        {
            e.call(NAV_HOLDER_RELEASE, &args![holder]);
            scope_leave(e, timer);
            return false;
        }
        let start = f.at(-0x20ec);
        let navmesh = e.call(NAV_HOLDER_GET, &args![holder]).u32();
        e.mem.set_u32(start, navmesh);
        let start_triangle = e.mem.u16(triangle);
        e.mem.set_u16(start + 4, start_triangle);
        let position = f.at(-0x20c0);
        let origin = e.call(REQUEST_ORIGIN, &args![request]).u32();
        e.call(LOCATION_COPY_POSITION, &args![origin, position]);
        let found_first = f.at(-0x20b1);
        let found_other = f.at(-0x20d5);
        e.mem.set_u8(found_first, 0);
        e.mem.set_u8(found_other, 0);
        let search = f.at(-0x20ac);
        e.call(LOS_SEARCH_CONSTRUCT, &args![search]);
        let path_nodes = f.at(-0x210c);
        e.call(
            NODE_ARRAY_A_CONSTRUCT,
            &args![path_nodes, 0x20u32, 0u32, 0u32],
        );
        let goal_nodes = f.at(-0x20d4);
        e.call(
            NODE_ARRAY_B_CONSTRUCT,
            &args![goal_nodes, 0x20u32, 0u32, 0u32],
        );
        let succeeded = f.at(-0x20f1);
        e.mem.set_u8(succeeded, 1);
        let built = e
            .call(
                LOS_SEARCH_BUILD_NODE_PATH,
                &args![search, request, start, path_nodes, goal_nodes],
            )
            .bool();
        if !built {
            // The search's recorded nodes, one after the other.
            let node = fn_006d1be0(e, Ptr::new(search));
            if node != 0 {
                let node = fn_006d1be0(e, Ptr::new(search));
                let found = e
                    .call(
                        LOS_SEARCH_PATH_FROM_NODE,
                        &args![search, node, path_nodes, goal_nodes],
                    )
                    .u8();
                e.mem.set_u8(found_first, found);
            }
            if e.mem.u8(found_first) == 0 {
                let node = fn_006d1c00(e, Ptr::new(search));
                if node != 0 {
                    let node = fn_006d1c00(e, Ptr::new(search));
                    let found = e
                        .call(
                            LOS_SEARCH_PATH_FROM_NODE,
                            &args![search, node, path_nodes, goal_nodes],
                        )
                        .u8();
                    e.mem.set_u8(found_other, found);
                }
            }
            if e.mem.u8(found_first) == 0 && e.mem.u8(found_other) == 0 {
                let node = fn_006d1c20(e, Ptr::new(search));
                if node != 0 {
                    let node = fn_006d1c20(e, Ptr::new(search));
                    let found = e
                        .call(
                            LOS_SEARCH_PATH_FROM_NODE,
                            &args![search, node, path_nodes, goal_nodes],
                        )
                        .u8();
                    e.mem.set_u8(found_other, found);
                }
            }
            if e.mem.u8(found_first) == 0 && e.mem.u8(found_other) == 0 {
                e.mem.set_u8(succeeded, 0);
                e.mem.set_u8(found_other, 1);
            }
        }

        if e.mem.u8(succeeded) != 0 {
            // Solve again from the path's last triangle.
            let last = f.at(-0x2120);
            let element = last_node(e, path_nodes);
            copy_search_node(e, last, element);
            let center = f.at(-0x2208);
            let last_navmesh = e.mem.u32(last);
            let last_triangle = e.mem.u16(last + 4) as u32;
            e.call(
                NAVMESH_GET_CENTER,
                &args![last_navmesh, center, last_triangle],
            );
            e.call(SOLUTION_CLEAR, &args![solution]);
            let copy = f.at(-0x21fc);
            e.call(REQUEST_COPY_CONSTRUCT, &args![copy, request]);
            let info = e.call(NAVMESH_GET_INFO, &args![last_navmesh]).u32();
            let location = f.at(-0x2148);
            e.call(
                LOCATION_CONSTRUCT_FROM_MESH_POINT,
                &args![location, center, info, last_triangle],
            );
            fn_006d1bc0(e, Ptr::new(copy), Ptr::new(location));
            let solved = fn_006d0900(e, Ptr::new(copy), solution);
            e.mem.set_u8(succeeded, solved as u8);
            e.call(LOCATION_DESTRUCT, &args![location]);
            e.call(REQUEST_COPY_DESTRUCT, &args![copy]);
        }

        if e.mem.u8(succeeded) == 0 && (e.mem.u8(found_first) != 0 || e.mem.u8(found_other) != 0) {
            // Move the last virtual node to the LOS map's triangle.
            let map_holder = f.at(-0x2210);
            e.call(NAV_HOLDER_CONSTRUCT, &args![map_holder]);
            let map_triangle = f.at(-0x220c);
            e.mem.set_u16(map_triangle, 0);
            let map = fn_006d1c40(e, Ptr::new(search));
            if e.call(LOS_MAP_FIND_TRIANGLE, &args![map, map_holder, map_triangle])
                .bool()
            {
                // The word at +0x1C of the solution (the last loaded
                // virtual node index; the body is shared with
                // `PathingLocation::GetWorldspace`).
                let last_index = e.call(LOCATION_GET_WORLDSPACE, &args![solution]).u32();
                let use_index = last_index;
                let usable = last_index != u32::MAX
                    && e.call(SOLUTION_VIRTUAL_NODE_COUNT, &args![solution]).u32() != 0;
                if !usable {
                    let start_navmesh = e.mem.u32(start);
                    let info = e.call(NAVMESH_GET_INFO, &args![start_navmesh]).u32();
                    e.call(SOLUTION_ADD_VIRTUAL_NODE, &args![solution, position, info]);
                    e.set(solution, PathingSolution::iLastLoadedVirtualNodeIndex, 0);
                }
                let node = e
                    .call(SOLUTION_VIRTUAL_NODE_AT, &args![solution, use_index])
                    .u32();
                if node != 0 {
                    let map_triangle_index = e.mem.u16(map_triangle) as u32;
                    let map_navmesh = e.call(NI_POINTER_GET_FROM_FIELD, &args![map_holder]).u32();
                    let triangle_center = f.at(-0x2224);
                    e.call(
                        NAVMESH_GET_CENTER,
                        &args![map_navmesh, triangle_center, map_triangle_index],
                    );
                    let map_navmesh = e.call(NI_POINTER_GET_FROM_FIELD, &args![map_holder]).u32();
                    let info = e.call(NAVMESH_GET_INFO, &args![map_navmesh]).u32();
                    let new_location = f.at(-0x224c);
                    e.call(
                        LOCATION_CONSTRUCT_FROM_MESH_POINT,
                        &args![new_location, triangle_center, info, map_triangle_index],
                    );
                    e.call(VIRTUAL_NODE_SET_LOCATION, &args![node, new_location]);
                    e.call(LOCATION_DESTRUCT, &args![new_location]);
                }
            }
            e.call(NAV_HOLDER_RELEASE, &args![map_holder]);
        }

        let result = e.mem.u8(succeeded) != 0;
        e.call(NODE_ARRAY_B_DESTRUCT, &args![goal_nodes]);
        e.call(NODE_ARRAY_A_DESTRUCT, &args![path_nodes]);
        e.call(LOS_SEARCH_DESTRUCT, &args![search]);
        e.call(NAV_HOLDER_RELEASE, &args![holder]);
        scope_leave(e, timer);
        result
    })
}

// Translated from 006d1bc0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Assigns `location` to the request's `Destination` (+0x34).
pub fn fn_006d1bc0(e: &mut Engine, this: Ptr, location: Ptr) {
    e.call(LOCATION_ASSIGN, &args![this.addr() + 0x34, location]);
}

// Translated from 006d1be0 (decompiled, FalloutNV.exe 1.4.0.525)
/// The word at +0x2090 of the LOS search (`NavMeshSearchLOS`). By position
/// it is `pFirstIgnoredGoal` (+0x2094 in the Xbox PDB, which is four bytes
/// larger before this field).
pub fn fn_006d1be0(e: &mut Engine, this: Ptr) -> u32 {
    e.mem.u32(this.addr() + 0x2090)
}

// Translated from 006d1c00 (decompiled, FalloutNV.exe 1.4.0.525)
/// The word at +0x2094 of the LOS search (`pLastValidNode`, +0x2098 in the
/// Xbox PDB).
pub fn fn_006d1c00(e: &mut Engine, this: Ptr) -> u32 {
    e.mem.u32(this.addr() + 0x2094)
}

// Translated from 006d1c20 (decompiled, FalloutNV.exe 1.4.0.525)
/// The word at +0x2098 of the LOS search (`pClosestNode`, +0x209c in the
/// Xbox PDB).
pub fn fn_006d1c20(e: &mut Engine, this: Ptr) -> u32 {
    e.mem.u32(this.addr() + 0x2098)
}

// Translated from 006d1c40 (decompiled, FalloutNV.exe 1.4.0.525)
/// The pointer held by the `NiPointer` at +0x2070 of the LOS search
/// (`spLOSMap`, +0x2074 in the Xbox PDB).
pub fn fn_006d1c40(e: &mut Engine, this: Ptr) -> u32 {
    e.call(NI_POINTER_GET_FROM_FIELD, &args![this.addr() + 0x2070])
        .u32()
}

// Translated from 006d1c60 (decompiled, FalloutNV.exe 1.4.0.525)
/// Solver for a type-4 request (close point): requires the origin to have
/// navmesh info and a navmesh and triangle, runs the close-point search's
/// `BuildNodePath`, adds the origin and the center of the last triangle as
/// the solution's first two virtual nodes and smooths the node path into
/// the solution with a `PathSmoother`. Afterwards the loaded node range is
/// 0..(virtual node count - 1).
pub fn fn_006d1c60(
    e: &mut Engine,
    request: Ptr<PathingRequest>,
    solution: Ptr<PathingSolution>,
) -> bool {
    with_frame(e, 0x221c, |e, f| {
        let timer = f.at(-0x5c);
        scope_enter(e, timer, 0x1f5);
        let origin = e.call(REQUEST_ORIGIN, &args![request]).u32();
        if !e
            .call(LOCATION_RESOLVE_NAVMESH_INFO, &args![origin, 0u32])
            .bool()
        {
            scope_leave(e, timer);
            return false;
        }
        let holder = f.at(-0x10);
        let triangle = f.at(-0x60);
        e.call(NAV_HOLDER_CONSTRUCT, &args![holder]);
        e.mem.set_u16(triangle, 0xffff);
        let origin = e.call(REQUEST_ORIGIN, &args![request]).u32();
        if !e
            .call(
                LOCATION_GET_NAVMESH_AND_TRIANGLE,
                &args![origin, holder, triangle],
            )
            .bool()
        {
            e.call(NAV_HOLDER_RELEASE, &args![holder]);
            scope_leave(e, timer);
            return false;
        }
        let start = f.at(-0x58);
        let navmesh = e.call(NAV_HOLDER_GET, &args![holder]).u32();
        e.mem.set_u32(start, navmesh);
        let start_triangle = e.mem.u16(triangle);
        e.mem.set_u16(start + 4, start_triangle);
        let position = f.at(-0x1c);
        let origin = e.call(REQUEST_ORIGIN, &args![request]).u32();
        e.call(LOCATION_COPY_POSITION, &args![origin, position]);
        let search = f.at(-0x2204);
        e.call(CLOSE_POINT_SEARCH_CONSTRUCT, &args![search]);
        let path_nodes = f.at(-0x74);
        e.call(
            NODE_ARRAY_A_CONSTRUCT,
            &args![path_nodes, 0x20u32, 0u32, 0u32],
        );
        let goal_nodes = f.at(-0x44);
        e.call(
            NODE_ARRAY_B_CONSTRUCT,
            &args![goal_nodes, 0x20u32, 0u32, 0u32],
        );
        let built = e
            .call(
                CLOSE_POINT_SEARCH_BUILD_NODE_PATH,
                &args![search, request, start, path_nodes, goal_nodes],
            )
            .bool();
        if !built {
            e.call(NODE_ARRAY_B_DESTRUCT, &args![goal_nodes]);
            e.call(NODE_ARRAY_A_DESTRUCT, &args![path_nodes]);
            e.call(CLOSE_POINT_SEARCH_DESTRUCT, &args![search]);
            e.call(NAV_HOLDER_RELEASE, &args![holder]);
            scope_leave(e, timer);
            return false;
        }
        let last = f.at(-0x30);
        let element = last_node(e, path_nodes);
        copy_search_node(e, last, element);
        let center = f.at(-0x2224);
        let last_navmesh = e.mem.u32(last);
        let last_triangle = e.mem.u16(last + 4) as u32;
        e.call(
            NAVMESH_GET_CENTER,
            &args![last_navmesh, center, last_triangle],
        );
        let info = e.call(NAVMESH_GET_INFO, &args![navmesh]).u32();
        e.call(SOLUTION_ADD_VIRTUAL_NODE, &args![solution, position, info]);
        let info = e.call(NAVMESH_GET_INFO, &args![last_navmesh]).u32();
        e.call(SOLUTION_ADD_VIRTUAL_NODE, &args![solution, center, info]);
        e.set(solution, PathingSolution::iFirstLoadedVirtualNodeIndex, 0);
        e.set(solution, PathingSolution::iLastLoadedVirtualNodeIndex, 1);
        let smoother = f.at(-0x12c);
        e.call(PATH_SMOOTHER_CONSTRUCT, &args![smoother]);
        let extra_nodes = f.at(-0x2218);
        e.call(
            NODE_ARRAY_C_CONSTRUCT,
            &args![extra_nodes, 0x20u32, 0u32, 0u32],
        );
        e.call(
            PATH_SMOOTHER_SMOOTH,
            &args![
                smoother, request, path_nodes, goal_nodes, position, center, solution, 0u32, 1u32
            ],
        );
        e.set(solution, PathingSolution::iFirstLoadedVirtualNodeIndex, 0);
        let count = e.call(SOLUTION_VIRTUAL_NODE_COUNT, &args![solution]).u32();
        e.set(
            solution,
            PathingSolution::iLastLoadedVirtualNodeIndex,
            count.wrapping_sub(1) as i32,
        );
        e.call(NODE_ARRAY_C_DESTRUCT, &args![extra_nodes]);
        e.call(PATH_SMOOTHER_DESTRUCT, &args![smoother]);
        e.call(NODE_ARRAY_B_DESTRUCT, &args![goal_nodes]);
        e.call(NODE_ARRAY_A_DESTRUCT, &args![path_nodes]);
        e.call(CLOSE_POINT_SEARCH_DESTRUCT, &args![search]);
        e.call(NAV_HOLDER_RELEASE, &args![holder]);
        scope_leave(e, timer);
        true
    })
}

// Translated from 006d1f80 (decompiled, FalloutNV.exe 1.4.0.525)
/// Solver for a type-6 request (hide): like the close-point solver but the
/// search takes two starting navmeshes, the origin's and the destination's
/// (the destination must resolve to a navmesh and triangle too), and builds
/// the node path with the hide search's `BuildNodePath`.
pub fn fn_006d1f80(
    e: &mut Engine,
    request: Ptr<PathingRequest>,
    solution: Ptr<PathingSolution>,
) -> bool {
    with_frame(e, 0x2270, |e, f| {
        let timer = f.at(-0x2168);
        scope_enter(e, timer, 0x23f);
        let origin = e.call(REQUEST_ORIGIN, &args![request]).u32();
        if !e.call(LOCATION_RESOLVE_CLOSEST, &args![origin]).bool() {
            scope_leave(e, timer);
            return false;
        }
        let origin_holder = f.at(-0x5c);
        let origin_triangle = f.at(-0x2198);
        e.call(NAV_HOLDER_CONSTRUCT, &args![origin_holder]);
        e.mem.set_u16(origin_triangle, 0xffff);
        let origin = e.call(REQUEST_ORIGIN, &args![request]).u32();
        if !e
            .call(
                LOCATION_GET_NAVMESH_AND_TRIANGLE,
                &args![origin, origin_holder, origin_triangle],
            )
            .bool()
        {
            e.call(NAV_HOLDER_RELEASE, &args![origin_holder]);
            scope_leave(e, timer);
            return false;
        }
        let origin_start = f.at(-0x58);
        let origin_navmesh = e.call(NAV_HOLDER_GET, &args![origin_holder]).u32();
        e.mem.set_u32(origin_start, origin_navmesh);
        let triangle = e.mem.u16(origin_triangle);
        e.mem.set_u16(origin_start + 4, triangle);
        let position = f.at(-0x18);
        let origin = e.call(REQUEST_ORIGIN, &args![request]).u32();
        e.call(LOCATION_COPY_POSITION, &args![origin, position]);

        let destination = e.call(REQUEST_DESTINATION, &args![request]).u32();
        if !e.call(LOCATION_RESOLVE_CLOSEST, &args![destination]).bool() {
            e.call(NAV_HOLDER_RELEASE, &args![origin_holder]);
            scope_leave(e, timer);
            return false;
        }
        let destination_holder = f.at(-0x2194);
        let destination_triangle = f.at(-0x30);
        e.call(NAV_HOLDER_CONSTRUCT, &args![destination_holder]);
        e.mem.set_u16(destination_triangle, 0xffff);
        let destination = e.call(REQUEST_DESTINATION, &args![request]).u32();
        if !e
            .call(
                LOCATION_GET_NAVMESH_AND_TRIANGLE,
                &args![destination, destination_holder, destination_triangle],
            )
            .bool()
        {
            e.call(NAV_HOLDER_RELEASE, &args![destination_holder]);
            e.call(NAV_HOLDER_RELEASE, &args![origin_holder]);
            scope_leave(e, timer);
            return false;
        }
        let destination_start = f.at(-0x2190);
        let navmesh = e.call(NAV_HOLDER_GET, &args![destination_holder]).u32();
        e.mem.set_u32(destination_start, navmesh);
        let triangle = e.mem.u16(destination_triangle);
        e.mem.set_u16(destination_start + 4, triangle);

        let search = f.at(-0x2164);
        e.call(HIDE_SEARCH_CONSTRUCT, &args![search]);
        let path_nodes = f.at(-0x217c);
        e.call(
            NODE_ARRAY_A_CONSTRUCT,
            &args![path_nodes, 0x20u32, 0u32, 0u32],
        );
        let goal_nodes = f.at(-0x44);
        e.call(
            NODE_ARRAY_B_CONSTRUCT,
            &args![goal_nodes, 0x20u32, 0u32, 0u32],
        );
        let built = e
            .call(
                HIDE_SEARCH_BUILD_NODE_PATH,
                &args![
                    search,
                    request,
                    origin_start,
                    destination_start,
                    path_nodes,
                    goal_nodes
                ],
            )
            .bool();
        if !built {
            e.call(NODE_ARRAY_B_DESTRUCT, &args![goal_nodes]);
            e.call(NODE_ARRAY_A_DESTRUCT, &args![path_nodes]);
            e.call(HIDE_SEARCH_DESTRUCT, &args![search]);
            e.call(NAV_HOLDER_RELEASE, &args![destination_holder]);
            e.call(NAV_HOLDER_RELEASE, &args![origin_holder]);
            scope_leave(e, timer);
            return false;
        }
        let last = f.at(-0x2c);
        let element = last_node(e, path_nodes);
        copy_search_node(e, last, element);
        let center = f.at(-0x2274);
        let last_navmesh = e.mem.u32(last);
        let last_triangle = e.mem.u16(last + 4) as u32;
        e.call(
            NAVMESH_GET_CENTER,
            &args![last_navmesh, center, last_triangle],
        );
        let info = e.call(NAVMESH_GET_INFO, &args![origin_navmesh]).u32();
        e.call(SOLUTION_ADD_VIRTUAL_NODE, &args![solution, position, info]);
        let info = e.call(NAVMESH_GET_INFO, &args![last_navmesh]).u32();
        e.call(SOLUTION_ADD_VIRTUAL_NODE, &args![solution, center, info]);
        e.set(solution, PathingSolution::iFirstLoadedVirtualNodeIndex, 0);
        e.set(solution, PathingSolution::iLastLoadedVirtualNodeIndex, 1);
        let smoother = f.at(-0x2254);
        e.call(PATH_SMOOTHER_CONSTRUCT, &args![smoother]);
        let extra_nodes = f.at(-0x2268);
        e.call(
            NODE_ARRAY_C_CONSTRUCT,
            &args![extra_nodes, 0x20u32, 0u32, 0u32],
        );
        e.call(
            PATH_SMOOTHER_SMOOTH,
            &args![
                smoother, request, path_nodes, goal_nodes, position, center, solution, 0u32, 1u32
            ],
        );
        e.set(solution, PathingSolution::iFirstLoadedVirtualNodeIndex, 0);
        let count = e.call(SOLUTION_VIRTUAL_NODE_COUNT, &args![solution]).u32();
        e.set(
            solution,
            PathingSolution::iLastLoadedVirtualNodeIndex,
            count.wrapping_sub(1) as i32,
        );
        e.call(NODE_ARRAY_C_DESTRUCT, &args![extra_nodes]);
        e.call(PATH_SMOOTHER_DESTRUCT, &args![smoother]);
        e.call(NODE_ARRAY_B_DESTRUCT, &args![goal_nodes]);
        e.call(NODE_ARRAY_A_DESTRUCT, &args![path_nodes]);
        e.call(HIDE_SEARCH_DESTRUCT, &args![search]);
        e.call(NAV_HOLDER_RELEASE, &args![destination_holder]);
        e.call(NAV_HOLDER_RELEASE, &args![origin_holder]);
        scope_leave(e, timer);
        true
    })
}

// Translated from 006d23d0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Solver for a type-7 request (optimal location): aims at the point on
/// the line from the destination to the origin that lies halfway between
/// the request's two distances (`fn_006d2c20`, `fn_006d2c40`) from the
/// destination, finds the closest navmesh point to it (`006d6f80`), fails
/// when that point is farther than `|halfway - distance|` away, searches
/// with `NavMeshSearchMaxCost`, and builds the solution with the path
/// builder from the origin to the chosen point.
///
/// When the search finds no path it falls back to the search's best node,
/// or, when no node array was filled, either to the base solver
/// (`fn_006d0b10`, when the direct distance exceeds the request's bound
/// from `0099e040`) or to a failed result. The chosen point must lie
/// between the squared distances of the request's two bounds.
pub fn fn_006d23d0(
    e: &mut Engine,
    request: Ptr<PathingRequest>,
    solution: Ptr<PathingSolution>,
) -> bool {
    with_frame(e, 0x4348, |e, f| {
        let timer = f.at(-0x2194);
        scope_enter(e, timer, 0x29c);
        let origin = e.call(REQUEST_ORIGIN, &args![request]).u32();
        if !e.call(LOCATION_RESOLVE_CLOSEST, &args![origin]).bool() {
            scope_leave(e, timer);
            return false;
        }
        let holder = f.at(-0x20);
        let triangle = f.at(-0x21a8);
        e.call(NAV_HOLDER_CONSTRUCT, &args![holder]);
        e.mem.set_u16(triangle, 0xffff);
        let origin = e.call(REQUEST_ORIGIN, &args![request]).u32();
        if !e
            .call(
                LOCATION_GET_NAVMESH_AND_TRIANGLE,
                &args![origin, holder, triangle],
            )
            .bool()
        {
            e.call(NAV_HOLDER_RELEASE, &args![holder]);
            scope_leave(e, timer);
            return false;
        }

        // The point halfway between the two distances, on the line from the
        // destination to the origin.
        let minimum = fn_006d2c20(e, request.cast());
        let maximum = fn_006d2c40(e, request.cast());
        let halfway = ((minimum as f64 + maximum as f64) / e.global::<f64>(TWO)) as f32;
        let origin_position = f.at(-0x2190);
        let origin = e.call(REQUEST_ORIGIN, &args![request]).u32();
        e.call(LOCATION_COPY_POSITION, &args![origin, origin_position]);
        let destination_position = f.at(-0x216c);
        let destination = e.call(REQUEST_DESTINATION, &args![request]).u32();
        e.call(
            LOCATION_COPY_POSITION,
            &args![destination, destination_position],
        );
        let direction = f.at(-0x21b4);
        e.call(
            POINT3_SUBTRACT,
            &args![destination_position, direction, origin_position],
        );
        let length = e.call(POINT3_UNITIZE_GET_LENGTH, &args![direction]).f32();
        let scaled_result = f.at(-0x431c);
        let scaled = e
            .call(
                POINT3_TIMES_SCALAR,
                &args![direction, scaled_result, halfway],
            )
            .u32();
        let aim = f.at(-0x21a4);
        e.call(POINT3_SUBTRACT, &args![destination_position, aim, scaled]);
        let slack = e
            .call(
                FLOAT_ABSOLUTE_VALUE,
                &args![((halfway as f64) - (length as f64)) as f32],
            )
            .f32();
        let slack_slot = f.at(-0x2160);
        e.mem.set_f32(slack_slot, slack);

        let closest = f.at(-0x48);
        e.call(POINT3_DEFAULT_CONSTRUCTOR, &args![closest]);
        let origin = e.call(REQUEST_ORIGIN, &args![request]).u32();
        let cell = e.call(LOCATION_GET_CELL, &args![origin]).u32();
        let origin = e.call(REQUEST_ORIGIN, &args![request]).u32();
        let worldspace = e.call(LOCATION_GET_WORLDSPACE, &args![origin]).u32();
        if !e
            .call(
                FIND_CLOSEST_POINT_ON_NAVMESH,
                &args![worldspace, cell, aim, closest],
            )
            .bool()
        {
            e.call(NAV_HOLDER_RELEASE, &args![holder]);
            scope_leave(e, timer);
            return false;
        }
        let gap = f.at(-0x3c);
        e.call(POINT3_SUBTRACT, &args![closest, gap, aim]);
        let gap_squared = e.call(POINT3_LENGTH_SQUARED, &args![gap]).f64();
        let slack = e.mem.f32(slack_slot) as f64;
        // Fails when the closest point is farther than the slack (an unordered
        // comparison continues, as the x87 test does).
        if slack * slack < gap_squared {
            e.call(NAV_HOLDER_RELEASE, &args![holder]);
            scope_leave(e, timer);
            return false;
        }

        // The goal location and the max-cost search from the origin.
        let goal_location = f.at(-0x215c);
        let origin = e.call(REQUEST_ORIGIN, &args![request]).u32();
        e.call(
            LOCATION_CONSTRUCT_FROM_POINT_AND_LOCATION,
            &args![goal_location, closest, origin],
        );
        let start = f.at(-0x2180);
        let navmesh = e.call(NAV_HOLDER_GET, &args![holder]).u32();
        e.mem.set_u32(start, navmesh);
        let start_triangle = e.mem.u16(triangle);
        e.mem.set_u16(start + 4, start_triangle);
        let center = f.at(-0x18);
        e.call(
            NAVMESH_GET_CENTER,
            &args![navmesh, center, start_triangle as u32],
        );
        let center_copy = f.at(-0x2178);
        copy_words(e, center_copy, center, 3);
        let origin_position_copy = f.at(-0x30);
        let origin = e.call(REQUEST_ORIGIN, &args![request]).u32();
        e.call(LOCATION_COPY_POSITION, &args![origin, origin_position_copy]);
        if !e
            .call(LOCATION_RESOLVE_NAVMESH_INFO, &args![goal_location, 0u32])
            .bool()
        {
            fn_006d2c00(e, Ptr::new(goal_location));
        }
        let search = f.at(-0x2134);
        e.call(MAX_COST_SEARCH_CONSTRUCT, &args![search]);
        let path_nodes = f.at(-0x21d8);
        e.call(
            NODE_ARRAY_A_CONSTRUCT,
            &args![path_nodes, 0x20u32, 0u32, 0u32],
        );
        let goal_nodes = f.at(-0x80);
        e.call(
            NODE_ARRAY_B_CONSTRUCT,
            &args![goal_nodes, 0x20u32, 0u32, 0u32],
        );
        fn_006d2c60(e, Ptr::new(search), slack_times_two(slack));
        let cover_holder = f.at(-0x21c4);
        e.call(NAVMESH_LIST_CONSTRUCT, &args![cover_holder]);
        e.call(NAVMESH_LIST_FILL, &args![cover_holder]);
        let chosen = f.at(-0x430c);
        e.call(POINT3_DEFAULT_CONSTRUCTOR, &args![chosen]);
        let found_path = e
            .call(
                MAX_COST_SEARCH_BUILD_NODE_PATH,
                &args![
                    search,
                    request,
                    start,
                    goal_location,
                    path_nodes,
                    goal_nodes
                ],
            )
            .bool();
        let end_node = f.at(-0x64);
        let mut fallback_result = None;
        if found_path {
            let element = last_node(e, path_nodes);
            copy_search_node(e, end_node, element);
            if !e.call(LOCATION_HAS_FLAG_2, &args![goal_location]).bool() {
                copy_words(e, chosen, closest, 3);
            } else {
                let point_slot = f.at(-0x432c);
                let point = e.call(NODE_TO_POSITION, &args![end_node, point_slot]).u32();
                copy_words(e, chosen, point, 3);
            }
        } else if !e.call(ARRAY_IS_EMPTY, &args![path_nodes]).bool() {
            // No complete path: use the search's best node.
            let best = e.call(MAX_COST_SEARCH_BEST_NODE, &args![search]).u32();
            copy_search_node(e, end_node, best + 8);
            let point_slot = f.at(-0x4338);
            let point = e.call(NODE_TO_POSITION, &args![end_node, point_slot]).u32();
            copy_words(e, chosen, point, 3);
        } else {
            // No nodes at all: hand over to the base solver when the
            // direct distance exceeds the request's bound.
            let bound = e
                .call(REQUEST_ABSOLUTE_MAXIMUM_DISTANCE, &args![request])
                .f64();
            fallback_result = Some(if (length as f64) > bound {
                fn_006d0b10(e, request, solution)
            } else {
                false
            });
        }
        if let Some(result) = fallback_result {
            release_optimal_locals(
                e,
                cover_holder,
                goal_nodes,
                path_nodes,
                search,
                goal_location,
                holder,
            );
            scope_leave(e, timer);
            return result;
        }

        // The chosen point must lie between the request's two bounds
        // (squared); an unordered comparison passes both tests.
        let near_bound = e
            .call(REQUEST_ABSOLUTE_MAXIMUM_DISTANCE, &args![request])
            .f32();
        let far_bound = e
            .call(REQUEST_ABSOLUTE_MINIMUM_DISTANCE, &args![request])
            .f32();
        let chosen_offset = f.at(-0x4348);
        let offset = e
            .call(
                POINT3_SUBTRACT,
                &args![chosen, chosen_offset, destination_position],
            )
            .u32();
        copy_words(e, direction, offset, 3);
        let offset_squared = e.call(POINT3_LENGTH_SQUARED, &args![direction]).f32() as f64;
        let far_squared = far_bound as f64 * far_bound as f64;
        let near_squared = near_bound as f64 * near_bound as f64;
        let too_close = offset_squared < far_squared;
        let too_far = offset_squared > near_squared;
        if too_close || too_far {
            release_optimal_locals(
                e,
                cover_holder,
                goal_nodes,
                path_nodes,
                search,
                goal_location,
                holder,
            );
            scope_leave(e, timer);
            return false;
        }

        let info = e.call(NAVMESH_GET_INFO, &args![navmesh]).u32();
        e.call(
            SOLUTION_ADD_VIRTUAL_NODE,
            &args![solution, origin_position_copy, info],
        );
        let end_navmesh = e.mem.u32(end_node);
        let info = e.call(NAVMESH_GET_INFO, &args![end_navmesh]).u32();
        e.call(SOLUTION_ADD_VIRTUAL_NODE, &args![solution, chosen, info]);
        e.set(solution, PathingSolution::iFirstLoadedVirtualNodeIndex, 0);
        e.set(solution, PathingSolution::iLastLoadedVirtualNodeIndex, 1);
        let builder = f.at(-0x42ec);
        e.call(PATH_BUILDER_CONSTRUCT, &args![builder]);
        let extra_nodes = f.at(-0x4300);
        e.call(
            NODE_ARRAY_C_CONSTRUCT,
            &args![extra_nodes, 0x20u32, 0u32, 0u32],
        );
        let built = e
            .call(
                PATH_BUILDER_BUILD,
                &args![
                    builder,
                    request,
                    path_nodes,
                    goal_nodes,
                    origin_position_copy,
                    chosen,
                    solution,
                    0u32,
                    0u32
                ],
            )
            .bool();
        e.call(NODE_ARRAY_C_DESTRUCT, &args![extra_nodes]);
        e.call(PATH_BUILDER_DESTRUCT, &args![builder]);
        release_optimal_locals(
            e,
            cover_holder,
            goal_nodes,
            path_nodes,
            search,
            goal_location,
            holder,
        );
        scope_leave(e, timer);
        built
    })
}

/// Twice the slack, stored as a `float` (`FADD ST0,ST0`).
fn slack_times_two(slack: f64) -> f32 {
    (slack + slack) as f32
}

/// The common exit of `fn_006d23d0`: destroys the locals it still holds, in
/// the game's order.
#[allow(clippy::too_many_arguments)]
fn release_optimal_locals(
    e: &mut Engine,
    cover_holder: u32,
    goal_nodes: u32,
    path_nodes: u32,
    search: u32,
    goal_location: u32,
    holder: u32,
) {
    e.call(NAVMESH_LIST_DESTRUCT, &args![cover_holder]);
    e.call(NODE_ARRAY_B_DESTRUCT, &args![goal_nodes]);
    e.call(NODE_ARRAY_A_DESTRUCT, &args![path_nodes]);
    e.call(MAX_COST_SEARCH_DESTRUCT, &args![search]);
    e.call(LOCATION_DESTRUCT, &args![goal_location]);
    e.call(NAV_HOLDER_RELEASE, &args![holder]);
}

// Translated from 006d2c00 (decompiled, FalloutNV.exe 1.4.0.525)
/// Sets bit 1 (`2`) of the location's flags (+0x26).
pub fn fn_006d2c00(e: &mut Engine, this: Ptr<PathingLocation>) {
    let flags = e.get(this, PathingLocation::uiFlags);
    e.set(this, PathingLocation::uiFlags, flags | 2);
}

// Translated from 006d2c20 (decompiled, FalloutNV.exe 1.4.0.525)
/// The `float` at +0xB0: `fMinFleeDistance` of a `PathingRequestFlee` and
/// `fMinDistance` of the close-point family (Xbox PDB), returned in `ST0`.
pub fn fn_006d2c20(e: &mut Engine, this: Ptr) -> f32 {
    e.mem.f32(this.addr() + 0xb0)
}

// Translated from 006d2c40 (decompiled, FalloutNV.exe 1.4.0.525)
/// The `float` at +0xB4 (`fMaxDistance` of the close-point family, Xbox
/// PDB), returned in `ST0`.
pub fn fn_006d2c40(e: &mut Engine, this: Ptr) -> f32 {
    e.mem.f32(this.addr() + 0xb4)
}

// Translated from 006d2c60 (decompiled, FalloutNV.exe 1.4.0.525)
/// Stores the `float` at +0x20A4 of the max-cost search (its cost limit).
pub fn fn_006d2c60(e: &mut Engine, this: Ptr, value: f32) {
    e.mem.set_f32(this.addr() + 0x20a4, value);
}

// Translated from 006d2c80 (decompiled, FalloutNV.exe 1.4.0.525)
/// Solver for a type-8 request (covered move): first solves the request as
/// a plain request (`fn_006d0b10`), then tries each later node of the
/// solution as the destination of a covered-move copy of the request
/// (`PathingRequestCoveredMove` with a fresh `CombatCoverLocation` and
/// `CombatCoverSearchInfo` shared with the request) until one passes
/// `0099d480` and also solves; that copy's cover location and destination
/// then replace the request's and the new solution is copied over. Without
/// a hit the request's cover location pointer is cleared (and the cover
/// location object deleted). The request is marked finished (+0xE1 = 1)
/// either way.
pub fn fn_006d2c80(
    e: &mut Engine,
    request: Ptr<PathingRequest>,
    solution: Ptr<PathingSolution>,
) -> bool {
    with_frame(e, 0x240, |e, f| {
        let timer = f.at(-0x1c);
        scope_enter(e, timer, 0x322);
        if !fn_006d0b10(e, request, solution) {
            scope_leave(e, timer);
            return false;
        }
        let distance = e.global::<f32>(COVERED_MOVE_DISTANCE);
        let origin = e.call(REQUEST_ORIGIN, &args![request]).u32();
        let previous = f.at(-0x15c);
        e.call(LOCATION_COPY_CONSTRUCT, &args![previous, origin]);

        let cover_block = e.call(MEMORY_ALLOCATE, &args![0x78u32]).u32();
        let cover_location = if cover_block != 0 {
            fn_006d3150(e, Ptr::new(cover_block)).addr()
        } else {
            0
        };
        let search_block = e.call(MEMORY_ALLOCATE, &args![0x3cu32]).u32();
        let search_info = if search_block != 0 {
            fn_006d32e0(e, Ptr::new(search_block)).addr()
        } else {
            0
        };
        let copy = f.at(-0x134);
        e.call(COVERED_MOVE_CONSTRUCT_COPY, &args![copy, request]);
        fn_006d3100(e, Ptr::new(copy), cover_location);
        fn_006d30e0(e, Ptr::new(copy), search_info);
        fn_006d3100(e, request.cast(), cover_location);
        fn_006d30e0(e, request.cast(), search_info);

        let mut index = 1u32;
        while e.call(SOLUTION_NODE_COUNT, &args![solution]).u32() > index {
            let node = e.call(SOLUTION_NODE_AT, &args![solution, index]).u32();
            let node_location = e.call(NODE_LOCATION, &args![node]).u32();
            let next = f.at(-0x188);
            e.call(LOCATION_COPY_CONSTRUCT, &args![next, node_location]);
            fn_006d1bc0(e, Ptr::new(copy), Ptr::new(next));
            if e.call(COVERED_MOVE_CHECK, &args![copy, distance, 1u32])
                .bool()
            {
                let cover_point = e.call(COVERED_MOVE_COVER_POINT, &args![copy]).u32();
                let cover_target = f.at(-0x1fc);
                e.call(
                    LOCATION_CONSTRUCT_FROM_POINT_AND_LOCATION,
                    &args![cover_target, cover_point, next],
                );
                fn_006d1bc0(e, Ptr::new(copy), Ptr::new(cover_target));
                let result = f.at(-0x1d4);
                e.call(SOLUTION_CONSTRUCT, &args![result]);
                let solved = fn_006d0b10(e, Ptr::new(copy), Ptr::new(result));
                if solved {
                    let cover_point = e.call(COVERED_MOVE_COVER_POINT, &args![copy]).u32();
                    fn_006d30b0(e, request.cast(), Ptr::new(cover_point));
                    fn_006d1bc0(e, request.cast(), Ptr::new(cover_target));
                    fn_006d3120(e, Ptr::new(copy));
                    e.call(SOLUTION_COPY_TO, &args![result, solution]);
                    e.call(REQUEST_SET_FINISHED, &args![request, 1u32]);
                    e.call(SOLUTION_DESTRUCT, &args![result]);
                    e.call(LOCATION_DESTRUCT, &args![cover_target]);
                    e.call(LOCATION_DESTRUCT, &args![next]);
                    e.call(COVERED_MOVE_DESTRUCT, &args![copy]);
                    e.call(LOCATION_DESTRUCT, &args![previous]);
                    scope_leave(e, timer);
                    return true;
                }
                e.call(SOLUTION_DESTRUCT, &args![result]);
                e.call(LOCATION_DESTRUCT, &args![cover_target]);
            }
            index += 1;
            e.call(LOCATION_ASSIGN, &args![previous, next]);
            e.call(LOCATION_DESTRUCT, &args![next]);
        }

        fn_006d3120(e, Ptr::new(copy));
        if cover_location != 0 {
            fn_006d3370(e, Ptr::new(cover_location), 1);
        }
        fn_006d3100(e, request.cast(), 0);
        e.call(REQUEST_SET_FINISHED, &args![request, 1u32]);
        e.call(COVERED_MOVE_DESTRUCT, &args![copy]);
        e.call(LOCATION_DESTRUCT, &args![previous]);
        scope_leave(e, timer);
        true
    })
}

// Translated from 006d30b0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Copies the point at `point` to the covered-move request's
/// `CoverLocation` (+0xC8).
pub fn fn_006d30b0(e: &mut Engine, this: Ptr, point: Ptr) {
    copy_words(e, this.addr() + 0xc8, point.addr(), 3);
}

// Translated from 006d30e0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Stores the covered-move request's `pCombatCoverSearchInfo` (+0x108).
pub fn fn_006d30e0(e: &mut Engine, this: Ptr<PathingRequestCoveredMove>, info: u32) {
    e.set(
        this,
        PathingRequestCoveredMove::pCombatCoverSearchInfo,
        Ptr::new(info),
    );
}

// Translated from 006d3100 (decompiled, FalloutNV.exe 1.4.0.525)
/// Stores the covered-move request's `pCombatCoverLocation` (+0x104).
pub fn fn_006d3100(e: &mut Engine, this: Ptr<PathingRequestCoveredMove>, location: u32) {
    e.set(
        this,
        PathingRequestCoveredMove::pCombatCoverLocation,
        Ptr::new(location),
    );
}

// Translated from 006d3120 (decompiled, FalloutNV.exe 1.4.0.525)
/// Clears the covered-move request's `pCombatCoverLocation` and
/// `pCombatCoverSearchInfo`.
pub fn fn_006d3120(e: &mut Engine, this: Ptr<PathingRequestCoveredMove>) {
    e.set(
        this,
        PathingRequestCoveredMove::pCombatCoverLocation,
        Ptr::NULL,
    );
    e.set(
        this,
        PathingRequestCoveredMove::pCombatCoverSearchInfo,
        Ptr::NULL,
    );
}

// Translated from 006d3150 (decompiled, FalloutNV.exe 1.4.0.525)
/// Constructor of the 0x78-byte object `fn_006d2c80` stores in
/// `pCombatCoverLocation` (the Xbox PDB's `CombatCoverLocation`): clears its
/// flags and location, copies the global zero vector into
/// `CoverSearchTargetLocation`, constructs the embedded
/// `PathingCoverLocation` (+0x14), clears the key and sets the time stamp to
/// -1 and constructs the `CoverReservationInfo` (+0x70). Returns `this`.
pub fn fn_006d3150(e: &mut Engine, this: Ptr<CombatCoverLocation>) -> Ptr<CombatCoverLocation> {
    e.set(this, CombatCoverLocation::bDirtyFlag, false);
    e.set(this, CombatCoverLocation::bReserved, false);
    e.set(this, CombatCoverLocation::cCoverHeight, 0);
    e.set(this, CombatCoverLocation::iLocation, 0);
    copy_words(e, this.addr() + 8, ZERO_VECTOR, 3);
    fn_006d3200(e, Ptr::new(this.addr() + 0x14));
    e.set(this, CombatCoverLocation::iCoverLocationKey, 0);
    e.set(
        this,
        CombatCoverLocation::iCoverLocationLastMovedTimeStamp,
        -1,
    );
    fn_006d3280(e, Ptr::new(this.addr() + 0x70));
    this
}

// Translated from 006d3200 (decompiled, FalloutNV.exe 1.4.0.525)
/// Constructor of the 0x54-byte `PathingCoverLocation`: the
/// `PathingLocation` constructor, the derived vtable (`0106cbc0`) and three
/// default-constructed points at +0x28, +0x34 and +0x40. Returns `this`.
pub fn fn_006d3200(e: &mut Engine, this: Ptr<PathingCoverLocation>) -> Ptr<PathingCoverLocation> {
    e.call(LOCATION_CONSTRUCT, &args![this]);
    e.mem.set_u32(this.addr(), PATHING_COVER_LOCATION_VTABLE);
    for offset in [0x28u32, 0x34, 0x40] {
        e.call(POINT3_DEFAULT_CONSTRUCTOR, &args![this.addr() + offset]);
    }
    this
}

// Translated from 006d3280 (decompiled, FalloutNV.exe 1.4.0.525)
/// Constructor of the six-byte `CoverReservationInfo`: clears its six
/// bytes. Returns `this`.
pub fn fn_006d3280(e: &mut Engine, this: Ptr<CoverReservationInfo>) -> Ptr<CoverReservationInfo> {
    for offset in 0..6 {
        e.mem.set_u8(this.addr() + offset, 0);
    }
    this
}

// Translated from 006d32c0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Destructor of `PathingCoverLocation`: destroys the `PathingLocation`
/// base.
pub fn fn_006d32c0(e: &mut Engine, this: Ptr<PathingCoverLocation>) {
    e.call(LOCATION_DESTRUCT, &args![this]);
}

// Translated from 006d32e0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Constructor of the 0x3C-byte object `fn_006d2c80` stores in
/// `pCombatCoverSearchInfo` (the Xbox PDB's `CombatCoverSearchInfo`): zero
/// vector in `TargetLocation` and `Center`, radius 0, the time stamp set to
/// -`FLT_MAX` (`00435de0`), the cover location array constructed
/// (`006dae60`), the key, state and flag cleared. Returns `this`.
pub fn fn_006d32e0(e: &mut Engine, this: Ptr<CombatCoverSearchInfo>) -> Ptr<CombatCoverSearchInfo> {
    copy_words(e, this.addr(), ZERO_VECTOR, 3);
    copy_words(e, this.addr() + 0xc, ZERO_VECTOR, 3);
    e.set(this, CombatCoverSearchInfo::fRadius, 0.0);
    let maximum = e.global::<f32>(FLOAT_MAXIMUM);
    e.call(TIME_STAMP_CONSTRUCT, &args![this.addr() + 0x1c, -maximum]);
    e.call(COVER_LOCATION_ARRAY_CONSTRUCT, &args![this.addr() + 0x20]);
    e.set(this, CombatCoverSearchInfo::iPreviousCoverLocationKey, 0);
    e.set(this, CombatCoverSearchInfo::ePreviousCoverState, 0);
    e.set(this, CombatCoverSearchInfo::bUsedLastSeenLocation, false);
    this
}

// Translated from 006d3370 (decompiled, FalloutNV.exe 1.4.0.525)
/// Scalar deleting destructor of `CombatCoverLocation`: runs
/// `fn_006d33a0` and frees the block when bit 0 of `flags` is set. Returns
/// `this`.
pub fn fn_006d3370(
    e: &mut Engine,
    this: Ptr<CombatCoverLocation>,
    flags: u32,
) -> Ptr<CombatCoverLocation> {
    fn_006d33a0(e, this);
    if flags & 1 != 0 {
        e.call(MEMORY_FREE, &args![this]);
    }
    this
}

// Translated from 006d33a0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Destructor of `CombatCoverLocation`: destroys the embedded
/// `PathingCoverLocation` (+0x14).
pub fn fn_006d33a0(e: &mut Engine, this: Ptr<CombatCoverLocation>) {
    fn_006d32c0(e, Ptr::new(this.addr() + 0x14));
}

// Translated from 006d33c0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Picks a random point near the request's origin and stores it in `out`.
/// First tries `fn_006d34d0` (a random point on the last triangle of a node
/// path from the origin); if that gives a point there is nothing more to
/// do. Otherwise `out = origin position + (x, y, 0)` with `(x, y)` a point
/// on a circle of random radius between `fn_006d2c20` and `fn_006d2c40` at a
/// random angle, and `out.z` replaced by the origin cell's land height when
/// the cell has one.
pub fn fn_006d33c0(e: &mut Engine, request: Ptr<PathingRequest>, out: Ptr) {
    if fn_006d34d0(e, request, out) {
        return;
    }
    let angle = e.call(RANDOM_ANGLE, &args![]).f32();
    let maximum = fn_006d2c40(e, request.cast());
    let minimum = fn_006d2c20(e, request.cast());
    let radius = e.call(RANDOM_FLOAT, &args![minimum, maximum]).f32();
    let y_scale = e.call(ANGLE_FUNCTION_004E44D0, &args![angle]).f64();
    let y = (y_scale * radius as f64) as f32;
    let x_scale = e.call(ANGLE_FUNCTION_004E4490, &args![angle]).f64();
    let x = (x_scale * radius as f64) as f32;
    with_frame(e, 0x3c, |e, f| {
        let offset = f.at(-0x10);
        e.call(POINT3_CONSTRUCT, &args![offset, x, y, 0.0f32]);
        let origin = e.call(REQUEST_ORIGIN, &args![request]).u32();
        let position = f.at(-0x28);
        let position = e
            .call(LOCATION_COPY_POSITION, &args![origin, position])
            .u32();
        let sum_result = f.at(-0x34);
        let sum = e
            .call(POINT3_ADD, &args![position, sum_result, offset])
            .u32();
        copy_words(e, out.addr(), sum, 3);
        let origin = e.call(REQUEST_ORIGIN, &args![request]).u32();
        let cell = e.call(LOCATION_GET_CELL, &args![origin]).u32();
        if cell == 0 {
            return;
        }
        let height = f.at(-0x18);
        if e.call(CELL_GET_LAND_HEIGHT, &args![cell, out, height])
            .bool()
        {
            let height = e.mem.f32(height);
            e.mem.set_f32(out.addr() + 8, height);
        }
    });
}

// Translated from 006d34d0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Finds a path from the request's origin with the random-point search
/// (`006a8b60`) and stores in `out` a random point (`006bbad0`) on the
/// triangle at the end of the path. Fails when the origin cannot be
/// resolved, has no navmesh and triangle, or the search finds no path.
pub fn fn_006d34d0(e: &mut Engine, request: Ptr<PathingRequest>, out: Ptr) -> bool {
    with_frame(e, 0x2134, |e, f| {
        let origin = e.call(REQUEST_ORIGIN, &args![request]).u32();
        if !e.call(LOCATION_RESOLVE_CLOSEST, &args![origin]).bool() {
            return false;
        }
        let holder = f.at(-0x10);
        let triangle = f.at(-0x74);
        e.call(NAV_HOLDER_CONSTRUCT, &args![holder]);
        e.mem.set_u16(triangle, 0xffff);
        let origin = e.call(REQUEST_ORIGIN, &args![request]).u32();
        if !e
            .call(
                LOCATION_GET_NAVMESH_AND_TRIANGLE,
                &args![origin, holder, triangle],
            )
            .bool()
        {
            e.call(NAV_HOLDER_RELEASE, &args![holder]);
            return false;
        }
        let start = f.at(-0x4c);
        let navmesh = e.call(NAV_HOLDER_GET, &args![holder]).u32();
        e.mem.set_u32(start, navmesh);
        let start_triangle = e.mem.u16(triangle);
        e.mem.set_u16(start + 4, start_triangle);
        let search = f.at(-0x212c);
        e.call(RANDOM_POINT_SEARCH_CONSTRUCT, &args![search]);
        let path_nodes = f.at(-0x8c);
        e.call(
            NODE_ARRAY_A_CONSTRUCT,
            &args![path_nodes, 0x20u32, 0u32, 0u32],
        );
        let goal_nodes = f.at(-0x38);
        e.call(
            NODE_ARRAY_B_CONSTRUCT,
            &args![goal_nodes, 0x20u32, 0u32, 0u32],
        );
        let built = e
            .call(
                RANDOM_POINT_SEARCH_BUILD_NODE_PATH,
                &args![search, request, start, path_nodes, goal_nodes],
            )
            .bool();
        if !built {
            e.call(NODE_ARRAY_B_DESTRUCT, &args![goal_nodes]);
            e.call(NODE_ARRAY_A_DESTRUCT, &args![path_nodes]);
            e.call(RANDOM_POINT_SEARCH_DESTRUCT, &args![search]);
            e.call(NAV_HOLDER_RELEASE, &args![holder]);
            return false;
        }
        let last = f.at(-0x24);
        let element = last_node(e, path_nodes);
        copy_search_node(e, last, element);
        // Three corner points, constructed by the vector constructor
        // iterator, then read from the triangle's vertices.
        let corners = f.at(-0x70);
        e.call(
            VECTOR_CONSTRUCTOR_ITERATOR,
            &args![corners, 0xcu32, 3u32, POINT3_DEFAULT_CONSTRUCTOR],
        );
        let last_navmesh = e.mem.u32(last);
        let last_triangle = e.mem.u16(last + 4) as u32;
        let record = e
            .call(NAVMESH_GET_TRIANGLE, &args![last_navmesh, last_triangle])
            .u32();
        for corner in 0..3u32 {
            let vertex_index = e
                .call(TRIANGLE_GET_VERTEX_INDEX, &args![record, corner])
                .u16() as u32;
            let vertex = e
                .call(NAVMESH_GET_VERTEX, &args![last_navmesh, vertex_index])
                .u32();
            copy_words(e, corners + 12 * corner, vertex, 3);
        }
        let point_result = f.at(-0x213c);
        let point = e
            .call(
                TRIANGLE_RANDOM_POINT,
                &args![point_result, corners, corners + 12, corners + 24],
            )
            .u32();
        copy_words(e, out.addr(), point, 3);
        e.call(NODE_ARRAY_B_DESTRUCT, &args![goal_nodes]);
        e.call(NODE_ARRAY_A_DESTRUCT, &args![path_nodes]);
        e.call(RANDOM_POINT_SEARCH_DESTRUCT, &args![search]);
        e.call(NAV_HOLDER_RELEASE, &args![holder]);
        true
    })
}

// Translated from 006d3780 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Pathing::FindSafeStraightLinePathPoint` (Xbox PDB): builds a
/// `PathingRequestSafeStraightLine` from reference `from` to reference
/// `to` and asks `006d3b40` for a safe point, stored in `out`. Each
/// reference's size is the larger of its two bound extents in x and y (the
/// vtable slots +0x1dc and +0x1d8 write the maximum and minimum corners)
/// times its scale, at least 32, halved; these are the origin's
/// (`fActorRadius`) and the destination's (`fTargetRadius`, +0x74) radii.
/// The request's `fMinDistance` is `0.9 * distance + origin radius` and its
/// `fMaxDistance` `distance + origin radius`.
pub fn pathing_find_safe_straight_line_path_point(
    e: &mut Engine,
    from: Ptr,
    to: Ptr,
    distance: f32,
    out: Ptr,
) -> bool {
    with_frame(e, 0x170, |e, f| {
        let request = f.at(-0xd4);
        e.call(SAFE_STRAIGHT_LINE_CONSTRUCT, &args![request]);
        let origin_location = f.at(-0x120);
        let result = e
            .call(
                LOCATION_CONSTRUCT_AT_REFERENCE,
                &args![origin_location, from],
            )
            .u32();
        fn_006d3ac0(e, Ptr::new(request), Ptr::new(result));
        e.call(LOCATION_DESTRUCT, &args![origin_location]);
        let destination_location = f.at(-0x148);
        let result = e
            .call(
                LOCATION_CONSTRUCT_AT_REFERENCE,
                &args![destination_location, to],
            )
            .u32();
        fn_006d1bc0(e, Ptr::new(request), Ptr::new(result));
        e.call(LOCATION_DESTRUCT, &args![destination_location]);

        // The origin's extent, from the first reference's own bounds.
        let maximum_corner = f.at(-0xf0);
        let minimum_corner = f.at(-0xe0);
        e.vcall(from.addr(), 0x1dc, &args![maximum_corner]);
        e.vcall(from.addr(), 0x1d8, &args![minimum_corner]);
        let origin_radius = reference_radius(e, from.addr(), f);
        fn_006d3ae0(e, Ptr::new(request), origin_radius);

        // The destination's extent.
        let bounds_a = f.at(-0x154);
        let corner = e.vcall(to.addr(), 0x1dc, &args![bounds_a]).u32();
        copy_words(e, maximum_corner, corner, 3);
        let bounds_b = f.at(-0x160);
        let corner = e.vcall(to.addr(), 0x1d8, &args![bounds_b]).u32();
        copy_words(e, minimum_corner, corner, 3);
        let destination_radius = reference_radius(e, to.addr(), f);
        e.call(
            REQUEST_SET_TARGET_RADIUS,
            &args![request, destination_radius],
        );

        let minimum =
            (distance as f64 * e.global::<f64>(NINE_TENTHS) + origin_radius as f64) as f32;
        e.call(REQUEST_SET_MINIMUM_DISTANCE, &args![request, minimum]);
        let maximum = (distance as f64 + origin_radius as f64) as f32;
        e.call(REQUEST_SET_MAXIMUM_DISTANCE, &args![request, maximum]);
        let found = e
            .call(SAFE_STRAIGHT_LINE_FIND, &args![request, out, 0u32])
            .bool();
        e.call(SAFE_STRAIGHT_LINE_DESTRUCT, &args![request]);
        found
    })
}

/// Half the reference's footprint: the larger of the x and y differences
/// of the corners at `-0xf0` and `-0xe0` (maximum minus minimum), times the
/// reference's scale (`TESObjectREFR::GetScale`), at least 32, times 0.5.
fn reference_radius(e: &mut Engine, reference: u32, f: Frame) -> f32 {
    let maximum_x = e.mem.f32(f.at(-0xf0)) as f64;
    let minimum_x = e.mem.f32(f.at(-0xe0)) as f64;
    let maximum_y = e.mem.f32(f.at(-0xec)) as f64;
    let minimum_y = e.mem.f32(f.at(-0xdc)) as f64;
    let width = (maximum_x - minimum_x) as f32;
    let depth = (maximum_y - minimum_y) as f32;
    let larger = e.call(FLOAT_MAX, &args![width, depth]).f32();
    let scale = e.call(REFERENCE_GET_SCALE, &args![reference]).f64();
    let size = (scale * larger as f64) as f32;
    let least = e.global::<f32>(MINIMUM_RADIUS);
    let size = e.call(FLOAT_MAX, &args![size, least]).f32();
    (size as f64 * e.global::<f64>(HALF)) as f32
}

// Translated from 006d3ac0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Assigns `location` to the request's `Origin` (+0x0C).
pub fn fn_006d3ac0(e: &mut Engine, this: Ptr, location: Ptr) {
    e.call(LOCATION_ASSIGN, &args![this.addr() + 0xc, location]);
}

// Translated from 006d3ae0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Stores the request's `fActorRadius` (+0x68).
pub fn fn_006d3ae0(e: &mut Engine, this: Ptr<PathingRequest>, radius: f32) {
    e.set(this, PathingRequest::fActorRadius, radius);
}

/// This unit's translated functions, by exe address.
pub fn funcs() -> Vec<(u32, AbiFn)> {
    vec![
        entry!(
            0x0049f210,
            bs_simple_array_parent_space_node_add(Ptr<crate::types::BSSimpleArray>, Ptr) -> i32
        ),
        entry!(0x006d0870, pathing_init() -> bool),
        entry!(
            0x006d0900,
            fn_006d0900(Ptr<PathingRequest>, Ptr<PathingSolution>) -> bool
        ),
        entry!(
            0x006d0b10,
            fn_006d0b10(Ptr<PathingRequest>, Ptr<PathingSolution>) -> bool
        ),
        entry!(0x006d0b80, fn_006d0b80(Ptr)),
        entry!(
            0x006d0be0,
            fn_006d0be0(Ptr<PathingRequest>, Ptr<PathingSolution>) -> bool
        ),
        entry!(
            0x006d12a0,
            fn_006d12a0(Ptr<PathingRequest>, Ptr<PathingSolution>) -> bool
        ),
        entry!(
            0x006d1520,
            fn_006d1520(Ptr<PathingSolution>, Ptr, u32, u32) -> u32
        ),
        entry!(0x006d1600, fn_006d1600() -> f32),
        entry!(0x006d1620, fn_006d1620(Ptr) -> f32),
        entry!(
            0x006d1660,
            fn_006d1660(Ptr<PathingRequest>, Ptr<PathingSolution>) -> bool
        ),
        entry!(0x006d1bc0, fn_006d1bc0(Ptr, Ptr)),
        entry!(0x006d1be0, fn_006d1be0(Ptr) -> u32),
        entry!(0x006d1c00, fn_006d1c00(Ptr) -> u32),
        entry!(0x006d1c20, fn_006d1c20(Ptr) -> u32),
        entry!(0x006d1c40, fn_006d1c40(Ptr) -> u32),
        entry!(
            0x006d1c60,
            fn_006d1c60(Ptr<PathingRequest>, Ptr<PathingSolution>) -> bool
        ),
        entry!(
            0x006d1f80,
            fn_006d1f80(Ptr<PathingRequest>, Ptr<PathingSolution>) -> bool
        ),
        entry!(
            0x006d23d0,
            fn_006d23d0(Ptr<PathingRequest>, Ptr<PathingSolution>) -> bool
        ),
        entry!(0x006d2c00, fn_006d2c00(Ptr<PathingLocation>)),
        entry!(0x006d2c20, fn_006d2c20(Ptr) -> f32),
        entry!(0x006d2c40, fn_006d2c40(Ptr) -> f32),
        entry!(0x006d2c60, fn_006d2c60(Ptr, f32)),
        entry!(
            0x006d2c80,
            fn_006d2c80(Ptr<PathingRequest>, Ptr<PathingSolution>) -> bool
        ),
        entry!(0x006d30b0, fn_006d30b0(Ptr, Ptr)),
        entry!(0x006d30e0, fn_006d30e0(Ptr<PathingRequestCoveredMove>, u32)),
        entry!(0x006d3100, fn_006d3100(Ptr<PathingRequestCoveredMove>, u32)),
        entry!(0x006d3120, fn_006d3120(Ptr<PathingRequestCoveredMove>)),
        entry!(
            0x006d3150,
            fn_006d3150(Ptr<CombatCoverLocation>) -> Ptr<CombatCoverLocation>
        ),
        entry!(
            0x006d3200,
            fn_006d3200(Ptr<PathingCoverLocation>) -> Ptr<PathingCoverLocation>
        ),
        entry!(
            0x006d3280,
            fn_006d3280(Ptr<CoverReservationInfo>) -> Ptr<CoverReservationInfo>
        ),
        entry!(0x006d32c0, fn_006d32c0(Ptr<PathingCoverLocation>)),
        entry!(
            0x006d32e0,
            fn_006d32e0(Ptr<CombatCoverSearchInfo>) -> Ptr<CombatCoverSearchInfo>
        ),
        entry!(
            0x006d3370,
            fn_006d3370(Ptr<CombatCoverLocation>, u32) -> Ptr<CombatCoverLocation>
        ),
        entry!(0x006d33a0, fn_006d33a0(Ptr<CombatCoverLocation>)),
        entry!(0x006d33c0, fn_006d33c0(Ptr<PathingRequest>, Ptr)),
        entry!(0x006d34d0, fn_006d34d0(Ptr<PathingRequest>, Ptr) -> bool),
        entry!(0x006d3780, pathing_find_safe_straight_line_path_point(Ptr, Ptr, f32, Ptr) -> bool),
        entry!(0x006d3ac0, fn_006d3ac0(Ptr, Ptr)),
        entry!(0x006d3ae0, fn_006d3ae0(Ptr<PathingRequest>, f32)),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::rc::Rc;

    /// Every callee outside this file: each gets a double that returns zero
    /// unless a test installs a better one.
    const CALLEES: &[u32] = &[
        TIMER_SCOPE_CONSTRUCT,
        TIMER_SCOPE_DESTROY,
        MEMORY_ALLOCATE,
        MEMORY_FREE,
        VECTOR_CONSTRUCTOR_ITERATOR,
        SETTING_FLOAT_ADDRESS,
        FLOAT_MAX,
        FLOAT_ABSOLUTE_VALUE,
        POINT3_CONSTRUCT,
        POINT3_SCALE_IN_PLACE,
        POINT3_ADD,
        POINT3_SUBTRACT,
        POINT3_TIMES_SCALAR,
        POINT3_NORMALIZE,
        POINT3_SCALED_COPY,
        POINT3_LENGTH_SQUARED,
        POINT3_UNITIZE_GET_LENGTH,
        POINT3_ADD_ASSIGN,
        POINT3_DEFAULT_CONSTRUCTOR,
        ANGLE_FUNCTION_004E44B0,
        ANGLE_FUNCTION_004E44D0,
        ANGLE_FUNCTION_004E4470,
        ANGLE_FUNCTION_004E4490,
        ANGLE_FUNCTION_005B9E80,
        RANDOM_FLOAT,
        RANDOM_ANGLE,
        RANDOM_OBJECT,
        RANDOM_UNSIGNED,
        NAV_HOLDER_CONSTRUCT,
        NAV_HOLDER_RELEASE,
        NAV_HOLDER_GET,
        NI_POINTER_GET_FROM_FIELD,
        NAVMESH_GET_CENTER,
        NAVMESH_GET_INFO,
        NAVMESH_GET_VERTEX,
        NAVMESH_GET_TRIANGLE,
        TRIANGLE_GET_VERTEX_INDEX,
        TRIANGLE_RANDOM_POINT,
        REQUEST_ORIGIN,
        REQUEST_DESTINATION,
        REQUEST_ACTOR_RADIUS,
        REQUEST_AVOID_NODE_ARRAY,
        REQUEST_FIRST_TANGENT_USES_HEADING,
        REQUEST_INITIAL_PATH_HEADING,
        REQUEST_ABSOLUTE_MAXIMUM_DISTANCE,
        REQUEST_ABSOLUTE_MINIMUM_DISTANCE,
        REQUEST_SET_TARGET_RADIUS,
        REQUEST_SET_MINIMUM_DISTANCE,
        REQUEST_SET_MAXIMUM_DISTANCE,
        REQUEST_SET_FINISHED,
        REQUEST_COPY_CONSTRUCT,
        REQUEST_COPY_DESTRUCT,
        SAFE_STRAIGHT_LINE_CONSTRUCT,
        SAFE_STRAIGHT_LINE_DESTRUCT,
        SAFE_STRAIGHT_LINE_FIND,
        COVERED_MOVE_CONSTRUCT_COPY,
        COVERED_MOVE_DESTRUCT,
        COVERED_MOVE_COVER_POINT,
        COVERED_MOVE_CHECK,
        LOCATION_GET_CELL,
        LOCATION_GET_WORLDSPACE,
        LOCATION_COPY_POSITION,
        LOCATION_RESOLVE_CLOSEST,
        LOCATION_GET_NAVMESH_AND_TRIANGLE,
        LOCATION_RESOLVE_NAVMESH_INFO,
        LOCATION_HAS_FLAG_2,
        LOCATION_ASSIGN,
        LOCATION_COPY_CONSTRUCT,
        LOCATION_CONSTRUCT_AT_REFERENCE,
        LOCATION_CONSTRUCT_FROM_PARTS,
        LOCATION_CONSTRUCT_FROM_MESH_POINT,
        LOCATION_CONSTRUCT_FROM_POINT_AND_INFO,
        LOCATION_CONSTRUCT_FROM_POINT_AND_LOCATION,
        LOCATION_CONSTRUCT,
        LOCATION_DESTRUCT,
        CELL_GET_LAND_HEIGHT,
        REFERENCE_GET_SCALE,
        ARRAY_COUNT,
        ARRAY_IS_EMPTY,
        ARRAY_ADD_SLOT,
        ARRAY_CONSTRUCT_ELEMENTS,
        NODE_ARRAY_ELEMENT,
        NODE_ARRAY_A_CONSTRUCT,
        NODE_ARRAY_A_DESTRUCT,
        NODE_ARRAY_B_CONSTRUCT,
        NODE_ARRAY_B_DESTRUCT,
        NODE_ARRAY_C_CONSTRUCT,
        NODE_ARRAY_C_DESTRUCT,
        VIRTUAL_NODE_ARRAY_ADD_SLOT,
        VIRTUAL_NODE_ARRAY_ELEMENT,
        PLACEMENT_NEW,
        VIRTUAL_NODE_CONSTRUCT,
        VIRTUAL_NODE_SET_LOCATION,
        SOLUTION_ADD_VIRTUAL_NODE,
        SOLUTION_SET_INCOMPLETE,
        SOLUTION_NODE_COUNT,
        SOLUTION_VIRTUAL_NODE_COUNT,
        SOLUTION_NODE_AT,
        SOLUTION_VIRTUAL_NODE_AT,
        SOLUTION_CLEAR,
        SOLUTION_CONSTRUCT,
        SOLUTION_COPY_TO,
        SOLUTION_DESTRUCT,
        NODE_LOCATION,
        NODE_SET_POSITION,
        NODE_TO_POSITION,
        AVOID_ARRAY_COMPUTE_PUSH,
        BUILD_FIRST_TANGENT,
        FIND_CLOSEST_POINT_ON_NAVMESH,
        FIND_PATH_POINT,
        PATH_POINT_CONSTRUCT,
        BASE_SOLVER_CONSTRUCT,
        BASE_SOLVER_RUN,
        NAVMESH_LIST_CONSTRUCT,
        NAVMESH_LIST_DESTRUCT,
        BASE_SOLVER_MEMBER_DESTRUCT,
        NAVMESH_LIST_FILL,
        FLEE_SEARCH_CONSTRUCT,
        FLEE_SEARCH_DESTRUCT,
        FLEE_SEARCH_BUILD_NODE_PATH,
        PATH_BUILDER_CONSTRUCT,
        PATH_BUILDER_DESTRUCT,
        PATH_BUILDER_BUILD,
        CLOSE_POINT_SEARCH_CONSTRUCT,
        CLOSE_POINT_SEARCH_DESTRUCT,
        CLOSE_POINT_SEARCH_BUILD_NODE_PATH,
        PATH_SMOOTHER_CONSTRUCT,
        PATH_SMOOTHER_SMOOTH,
        PATH_SMOOTHER_DESTRUCT,
        LOS_SEARCH_CONSTRUCT,
        LOS_SEARCH_DESTRUCT,
        LOS_SEARCH_BUILD_NODE_PATH,
        LOS_SEARCH_PATH_FROM_NODE,
        LOS_MAP_FIND_TRIANGLE,
        HIDE_SEARCH_CONSTRUCT,
        HIDE_SEARCH_DESTRUCT,
        HIDE_SEARCH_BUILD_NODE_PATH,
        MAX_COST_SEARCH_CONSTRUCT,
        MAX_COST_SEARCH_DESTRUCT,
        MAX_COST_SEARCH_BUILD_NODE_PATH,
        MAX_COST_SEARCH_BEST_NODE,
        RANDOM_POINT_SEARCH_CONSTRUCT,
        RANDOM_POINT_SEARCH_DESTRUCT,
        RANDOM_POINT_SEARCH_BUILD_NODE_PATH,
        TIME_STAMP_CONSTRUCT,
        COVER_LOCATION_ARRAY_CONSTRUCT,
    ];

    /// The pages the translations read constants and globals from.
    const DATA_PAGES: &[u32] = &[
        0x0101_1000,
        0x0101_e000,
        0x0101_f000,
        0x0102_3000,
        0x0106_b000,
        0x0106_c000,
        0x011a_5000,
        0x011a_9000,
        0x011f_2000,
        0x011f_4000,
    ];

    /// Points recorded by a double.
    type Points = Rc<RefCell<Vec<[f32; 3]>>>;
    /// Floats recorded by a double.
    type Limits = Rc<RefCell<Vec<f32>>>;
    /// An engine with a request, a solution and a point recorder.
    type RecordedFixture = (Engine, Ptr<PathingRequest>, Ptr<PathingSolution>, Points);

    const NAVMESH: u32 = 0x00c0_0100;
    const LAST_NAVMESH: u32 = 0x00c0_0200;
    const ROUTE_TYPE_FUNCTION: u32 = 0x00a0_1000;
    const REQUEST_VTABLE: u32 = 0x00a0_0000;

    fn ret(value: u32) -> Ret {
        Ret {
            eax: value,
            ..Ret::default()
        }
    }

    fn ret_float(value: f32) -> Ret {
        Ret {
            st0: value as f64,
            ..Ret::default()
        }
    }

    fn float_arg(word: u32) -> f32 {
        f32::from_bits(word)
    }

    /// An engine where every callee outside the file is a no-op that
    /// returns zero, and the data pages exist.
    fn engine() -> Engine {
        let mut e = Engine::new();
        for address in CALLEES {
            e.register(*address, |_, _| Ret::default());
        }
        for page in DATA_PAGES {
            e.map(*page, 0x1000);
        }
        e.set_global(TWO, 2.0f64);
        e.set_global(HALF, 0.5f64);
        e.set_global(NINE_TENTHS, 0.9f64);
        e.set_global(MINIMUM_RADIUS, 32.0f32);
        e.set_global(COVERED_MOVE_DISTANCE, 512.0f32);
        e.set_global(FLOAT_MAXIMUM, f32::MAX);
        e.set_global(
            RANDOM_ANGLE_SCALE,
            std::f64::consts::PI * 2.0 / 4294967296.0,
        );
        e.set_global(PI_AS_FLOAT, std::f32::consts::PI as f64);
        e.set_global(DEGREES_TO_RADIANS, std::f64::consts::PI / 180.0);
        e
    }

    fn set_point(e: &mut Engine, address: u32, point: [f32; 3]) {
        for (i, value) in point.iter().enumerate() {
            e.mem.set_f32(address + 4 * i as u32, *value);
        }
    }

    fn point(e: &Engine, address: u32) -> [f32; 3] {
        [
            e.mem.f32(address),
            e.mem.f32(address + 4),
            e.mem.f32(address + 8),
        ]
    }

    fn assert_close(actual: [f32; 3], expected: [f32; 3]) {
        for i in 0..3 {
            assert!(
                (actual[i] - expected[i]).abs() < 1e-4,
                "{actual:?} != {expected:?}"
            );
        }
    }

    /// A request big enough for every subclass, and a solution.
    fn objects(e: &mut Engine) -> (Ptr<PathingRequest>, Ptr<PathingSolution>) {
        let request = Ptr::new(e.mem.alloc(0x120));
        let solution = Ptr::new(e.mem.alloc(0x60));
        (request, solution)
    }

    /// Gives the request a vtable whose `GetType` returns `kind`.
    fn set_request_type(e: &mut Engine, request: Ptr<PathingRequest>, kind: u32) {
        let mut slots = vec![0u32; 5];
        slots[4] = ROUTE_TYPE_FUNCTION;
        e.put_vtable(REQUEST_VTABLE, &slots);
        e.register_double(ROUTE_TYPE_FUNCTION, move |_, _| ret(kind));
        e.mem.set_u32(request.addr(), REQUEST_VTABLE);
    }

    fn log_of(e: &mut Engine) -> Vec<(u32, Vec<u32>)> {
        e.call_log.take().unwrap()
    }

    fn calls_to(log: &[(u32, Vec<u32>)], address: u32) -> Vec<Vec<u32>> {
        log.iter()
            .filter(|(callee, _)| *callee == address)
            .map(|(_, args)| args.clone())
            .collect()
    }

    fn position_of(log: &[(u32, Vec<u32>)], address: u32) -> Option<usize> {
        log.iter().position(|(callee, _)| *callee == address)
    }

    /// The source lines of the scope timers opened, in order.
    fn timer_lines(log: &[(u32, Vec<u32>)]) -> Vec<u32> {
        calls_to(log, TIMER_SCOPE_CONSTRUCT)
            .iter()
            .map(|args| args[4])
            .collect()
    }

    /// Every scope timer that was opened was closed.
    fn assert_timers_balanced(log: &[(u32, Vec<u32>)]) {
        assert_eq!(
            calls_to(log, TIMER_SCOPE_CONSTRUCT).len(),
            calls_to(log, TIMER_SCOPE_DESTROY).len()
        );
    }

    /// Records the three floats at argument `which` on every call of
    /// `address`; the double returns the index of the call.
    fn record_points(e: &mut Engine, address: u32, which: usize) -> Points {
        let points = Rc::new(RefCell::new(Vec::new()));
        let sink = points.clone();
        e.register_double(address, move |e, a| {
            let mut points = sink.borrow_mut();
            points.push(point(e, a[which]));
            ret(points.len() as u32 - 1)
        });
        points
    }

    /// Arithmetic doubles for the point helpers.
    fn vector_math(e: &mut Engine) {
        e.register(POINT3_SUBTRACT, |e, a| {
            let (x, y) = (point(e, a[0]), point(e, a[2]));
            set_point(e, a[1], [x[0] - y[0], x[1] - y[1], x[2] - y[2]]);
            ret(a[1])
        });
        e.register(POINT3_ADD, |e, a| {
            let (x, y) = (point(e, a[0]), point(e, a[2]));
            set_point(e, a[1], [x[0] + y[0], x[1] + y[1], x[2] + y[2]]);
            ret(a[1])
        });
        e.register(POINT3_TIMES_SCALAR, |e, a| {
            let (x, s) = (point(e, a[0]), float_arg(a[2]));
            set_point(e, a[1], [x[0] * s, x[1] * s, x[2] * s]);
            ret(a[1])
        });
        e.register(POINT3_SCALED_COPY, |e, a| {
            let (x, s) = (point(e, a[2]), float_arg(a[1]));
            set_point(e, a[0], [x[0] * s, x[1] * s, x[2] * s]);
            ret(a[0])
        });
        e.register(POINT3_SCALE_IN_PLACE, |e, a| {
            let (x, s) = (point(e, a[0]), float_arg(a[1]));
            set_point(e, a[0], [x[0] * s, x[1] * s, x[2] * s]);
            ret(a[0])
        });
        e.register(POINT3_ADD_ASSIGN, |e, a| {
            let (x, y) = (point(e, a[0]), point(e, a[1]));
            set_point(e, a[0], [x[0] + y[0], x[1] + y[1], x[2] + y[2]]);
            ret(a[0])
        });
        e.register(POINT3_NORMALIZE, |e, a| {
            let x = point(e, a[0]);
            let length = (x[0] * x[0] + x[1] * x[1] + x[2] * x[2]).sqrt();
            if length > 1e-6 {
                set_point(e, a[0], [x[0] / length, x[1] / length, x[2] / length]);
            }
            Ret::default()
        });
        e.register(POINT3_LENGTH_SQUARED, |e, a| {
            let x = point(e, a[0]);
            ret_float(x[0] * x[0] + x[1] * x[1] + x[2] * x[2])
        });
        e.register(POINT3_UNITIZE_GET_LENGTH, |e, a| {
            let x = point(e, a[0]);
            let length = (x[0] * x[0] + x[1] * x[1] + x[2] * x[2]).sqrt();
            if length > 1e-6 {
                set_point(e, a[0], [x[0] / length, x[1] / length, x[2] / length]);
            }
            ret_float(length)
        });
        e.register(FLOAT_MAX, |_, a| {
            let (x, y) = (float_arg(a[0]), float_arg(a[1]));
            ret_float(if y < x { x } else { y })
        });
        e.register(FLOAT_ABSOLUTE_VALUE, |_, a| {
            ret_float(float_arg(a[0]).abs())
        });
    }

    /// A resolvable origin (1, 2, 3) and destination (11, 2, 3) on a
    /// navmesh, with the search node arrays ending in one node. Returns the
    /// address of that node.
    fn geometry(e: &mut Engine, request: Ptr<PathingRequest>) -> u32 {
        e.register(REQUEST_ORIGIN, |_, a| ret(a[0] + 0xc));
        e.register(REQUEST_DESTINATION, |_, a| ret(a[0] + 0x34));
        e.register(LOCATION_RESOLVE_CLOSEST, |_, _| ret(1));
        e.register(LOCATION_RESOLVE_NAVMESH_INFO, |_, _| ret(1));
        e.register(LOCATION_HAS_FLAG_2, |_, _| ret(1));
        e.register(LOCATION_GET_NAVMESH_AND_TRIANGLE, |e, a| {
            e.mem.set_u16(a[2], 9);
            ret(1)
        });
        e.register(NAV_HOLDER_GET, |_, _| ret(NAVMESH));
        e.register(NAVMESH_GET_INFO, |_, a| ret(a[0] + 0x1000));
        let origin = request.addr() + 0xc;
        e.register_double(LOCATION_COPY_POSITION, move |e, a| {
            let position = if a[0] == origin {
                [1.0, 2.0, 3.0]
            } else {
                [11.0, 2.0, 3.0]
            };
            set_point(e, a[1], position);
            ret(a[1])
        });
        e.register(NAVMESH_GET_CENTER, |e, a| {
            set_point(e, a[1], [10.0, 20.0, 30.0]);
            Ret::default()
        });
        e.register(ARRAY_COUNT, |_, _| ret(2));
        let node = e.mem.alloc(0x14);
        e.mem.set_u32(node, LAST_NAVMESH);
        e.mem.set_u16(node + 4, 7);
        set_point(e, node + 8, [5.0, 6.0, 7.0]);
        e.register_double(NODE_ARRAY_ELEMENT, move |_, a| {
            assert_eq!(a[1], 1, "the last of two nodes");
            ret(node)
        });
        e.register(SOLUTION_VIRTUAL_NODE_COUNT, |_, _| ret(5));
        node
    }

    // -- 0049f210 ----------------------------------------------------------

    #[test]
    fn parent_space_node_add_constructs_then_copies_the_item() {
        let mut e = engine();
        let array: Ptr<crate::types::BSSimpleArray> = e.new_object();
        let first_buffer = e.mem.alloc(0x40);
        let second_buffer = e.mem.alloc(0x40);
        e.set(array, crate::types::BSSimpleArray::pBuffer, first_buffer);
        e.register(ARRAY_ADD_SLOT, |_, _| ret(2));
        // The construction reallocates the buffer: the copy must go to the
        // new one.
        let array_address = array.addr();
        e.register_double(ARRAY_CONSTRUCT_ELEMENTS, move |e, _| {
            e.mem.set_u32(array_address + 4, second_buffer);
            Ret::default()
        });
        let item = e.mem.alloc(12);
        for i in 0..3 {
            e.mem.set_u32(item + 4 * i, 0x100 + i);
        }
        e.call_log = Some(vec![]);
        let index = e
            .call(0x0049_f210, &args![array, Ptr::<()>::new(item)])
            .i32();
        assert_eq!(index, 2);
        let log = log_of(&mut e);
        assert_eq!(
            calls_to(&log, ARRAY_CONSTRUCT_ELEMENTS),
            vec![vec![array.addr(), first_buffer + 24, 1]]
        );
        assert_eq!(e.mem.u32(second_buffer + 24), 0x100);
        assert_eq!(e.mem.u32(second_buffer + 28), 0x101);
        assert_eq!(e.mem.u32(second_buffer + 32), 0x102);
        assert_eq!(e.mem.u32(first_buffer + 24), 0);
    }

    // -- 006d0870 ----------------------------------------------------------

    #[test]
    fn init_converts_the_three_angle_settings() {
        let mut e = engine();
        e.set_global(DEGREES_TO_RADIANS, std::f64::consts::PI / 180.0);
        for (setting, degrees) in [
            (SETTING_ANGLE_A, 90.0f32),
            (SETTING_ANGLE_B, 180.0),
            (SETTING_ANGLE_C, 45.0),
        ] {
            e.set_global(setting, degrees);
        }
        // The settings object is its own value address.
        e.register(SETTING_FLOAT_ADDRESS, |_, a| ret(a[0]));
        // The angle function doubles its argument, so the stores can be
        // told apart from the conversion.
        e.register(ANGLE_FUNCTION_005B9E80, |_, a| {
            ret_float(float_arg(a[0]) * 2.0)
        });
        assert!(e.call(0x006d_0870, &args![]).bool());
        for (destination, degrees) in [
            (CONVERTED_ANGLE_A, 90.0f32),
            (CONVERTED_ANGLE_B, 180.0),
            (CONVERTED_ANGLE_C, 45.0),
        ] {
            let radians = (degrees as f64 * (std::f64::consts::PI / 180.0)) as f32;
            assert_eq!(e.global::<f32>(destination), radians * 2.0);
        }
    }

    // -- 006d0900 ----------------------------------------------------------

    #[test]
    fn dispatch_sends_each_type_to_its_solver() {
        // The line number the solver passes to its scope timer says which
        // solver ran.
        for (kind, line) in [
            (3u32, 0xb3u32),
            (4, 0x1f5),
            (5, 0x17a),
            (6, 0x23f),
            (7, 0x29c),
            (8, 0x322),
        ] {
            let mut e = engine();
            vector_math(&mut e);
            let (request, solution) = objects(&mut e);
            set_request_type(&mut e, request, kind);
            e.call_log = Some(vec![]);
            e.call(0x006d_0900, &args![request, solution]);
            let log = log_of(&mut e);
            assert_eq!(timer_lines(&log)[..2], [0x6a, line], "type {kind}");
            assert_timers_balanced(&log);
        }
    }

    #[test]
    fn dispatch_type_zero_runs_the_base_solver() {
        let mut e = engine();
        let (request, solution) = objects(&mut e);
        set_request_type(&mut e, request, 0);
        e.register(BASE_SOLVER_RUN, |_, _| ret(1));
        e.call_log = Some(vec![]);
        assert!(e.call(0x006d_0900, &args![request, solution]).bool());
        let log = log_of(&mut e);
        assert_eq!(timer_lines(&log), vec![0x6a]);
        assert_eq!(calls_to(&log, BASE_SOLVER_CONSTRUCT).len(), 1);
        assert_timers_balanced(&log);
    }

    #[test]
    fn dispatch_fails_for_the_unused_types() {
        for kind in [1u32, 2, 9, 0x100] {
            let mut e = engine();
            let (request, solution) = objects(&mut e);
            set_request_type(&mut e, request, kind);
            e.register(BASE_SOLVER_RUN, |_, _| ret(1));
            e.call_log = Some(vec![]);
            assert!(!e.call(0x006d_0900, &args![request, solution]).bool());
            let log = log_of(&mut e);
            assert_eq!(timer_lines(&log), vec![0x6a], "type {kind}");
            assert!(calls_to(&log, BASE_SOLVER_CONSTRUCT).is_empty());
        }
    }

    // -- 006d0b10, 006d0b80 -------------------------------------------------

    #[test]
    fn base_solver_runs_and_destroys_the_solver() {
        for found in [true, false] {
            let mut e = engine();
            let (request, solution) = objects(&mut e);
            let run_result = found as u32;
            e.register_double(BASE_SOLVER_RUN, move |_, _| ret(run_result));
            e.call_log = Some(vec![]);
            let result = e.call(0x006d_0b10, &args![request, solution]).bool();
            assert_eq!(result, found);
            let log = log_of(&mut e);
            let construct = &calls_to(&log, BASE_SOLVER_CONSTRUCT)[0];
            let solver = construct[0];
            assert_eq!(construct[1..], [request.addr(), solution.addr()]);
            assert_eq!(calls_to(&log, BASE_SOLVER_RUN), vec![vec![solver]]);
            // Destroyed after the run: the member at +0x24, then +0x14.
            let run = position_of(&log, BASE_SOLVER_RUN).unwrap();
            assert_eq!(log[run + 1], (NAVMESH_LIST_DESTRUCT, vec![solver + 0x24]));
            assert_eq!(
                log[run + 2],
                (BASE_SOLVER_MEMBER_DESTRUCT, vec![solver + 0x14])
            );
        }
    }

    #[test]
    fn base_solver_destructor_destroys_both_members() {
        let mut e = engine();
        let solver = e.mem.alloc(0x40);
        e.call_log = Some(vec![]);
        e.call(0x006d_0b80, &args![Ptr::<()>::new(solver)]);
        let log = log_of(&mut e);
        assert_eq!(
            log[1..],
            [
                (NAVMESH_LIST_DESTRUCT, vec![solver + 0x24]),
                (BASE_SOLVER_MEMBER_DESTRUCT, vec![solver + 0x14]),
            ]
        );
    }

    // -- 006d0be0 ----------------------------------------------------------

    /// A flee request on a fully resolvable navmesh whose search and path
    /// builder succeed, with the solution's virtual node points recorded.
    fn flee_setup() -> RecordedFixture {
        let mut e = engine();
        let (request, solution) = objects(&mut e);
        geometry(&mut e, request);
        vector_math(&mut e);
        e.register(FLEE_SEARCH_BUILD_NODE_PATH, |_, _| ret(1));
        e.register(PATH_BUILDER_BUILD, |_, _| ret(1));
        let added = record_points(&mut e, SOLUTION_ADD_VIRTUAL_NODE, 1);
        (e, request, solution, added)
    }

    #[test]
    fn flee_builds_a_path_to_the_center_of_the_last_triangle() {
        let (mut e, request, solution, added) = flee_setup();
        e.call_log = Some(vec![]);
        assert!(e.call(0x006d_0be0, &args![request, solution]).bool());
        let log = log_of(&mut e);
        assert_eq!(timer_lines(&log), vec![0xb3]);
        assert_timers_balanced(&log);
        // The start is the origin, the goal the last triangle's center.
        assert_eq!(*added.borrow(), vec![[1.0, 2.0, 3.0], [10.0, 20.0, 30.0]]);
        let adds = calls_to(&log, SOLUTION_ADD_VIRTUAL_NODE);
        assert_eq!(adds[0][2], NAVMESH + 0x1000);
        assert_eq!(adds[1][2], LAST_NAVMESH + 0x1000);
        assert_eq!(calls_to(&log, NAVMESH_GET_CENTER)[0][0], LAST_NAVMESH);
        assert_eq!(calls_to(&log, NAVMESH_GET_CENTER)[0][2], 7);
        assert_eq!(
            e.get(solution, PathingSolution::iFirstLoadedVirtualNodeIndex),
            0
        );
        assert_eq!(
            e.get(solution, PathingSolution::iLastLoadedVirtualNodeIndex),
            1
        );
        let build = &calls_to(&log, PATH_BUILDER_BUILD)[0];
        assert_eq!(build.len(), 9);
        assert_eq!(build[1], request.addr());
        assert_eq!(build[6], solution.addr());
        assert_eq!(build[7..], [0, 0]);
        assert!(calls_to(&log, SOLUTION_SET_INCOMPLETE).is_empty());
        // Everything built was destroyed again.
        assert_eq!(
            calls_to(&log, NODE_ARRAY_A_CONSTRUCT).len(),
            calls_to(&log, NODE_ARRAY_A_DESTRUCT).len()
        );
        assert_eq!(
            calls_to(&log, NAV_HOLDER_CONSTRUCT).len(),
            calls_to(&log, NAV_HOLDER_RELEASE).len()
        );
    }

    #[test]
    fn flee_marks_the_solution_incomplete_when_the_path_builder_fails() {
        let (mut e, request, solution, _) = flee_setup();
        e.register(PATH_BUILDER_BUILD, |_, _| ret(0));
        e.call_log = Some(vec![]);
        assert!(!e.call(0x006d_0be0, &args![request, solution]).bool());
        let log = log_of(&mut e);
        assert_eq!(
            calls_to(&log, SOLUTION_SET_INCOMPLETE),
            vec![vec![solution.addr(), 1]]
        );
    }

    #[test]
    fn flee_without_a_search_result_fails_and_cleans_up() {
        let (mut e, request, solution, added) = flee_setup();
        e.register(FLEE_SEARCH_BUILD_NODE_PATH, |_, _| ret(0));
        e.call_log = Some(vec![]);
        assert!(!e.call(0x006d_0be0, &args![request, solution]).bool());
        let log = log_of(&mut e);
        assert!(added.borrow().is_empty());
        assert!(calls_to(&log, PATH_BUILDER_BUILD).is_empty());
        assert_eq!(calls_to(&log, FLEE_SEARCH_DESTRUCT).len(), 1);
        assert_timers_balanced(&log);
    }

    #[test]
    fn flee_without_a_resolvable_origin_flees_without_a_search() {
        let (mut e, request, solution, _) = flee_setup();
        e.register(LOCATION_RESOLVE_CLOSEST, |_, _| ret(0));
        e.call_log = Some(vec![]);
        assert!(e.call(0x006d_0be0, &args![request, solution]).bool());
        let log = log_of(&mut e);
        assert_eq!(timer_lines(&log), vec![0xb3, 0x12f]);
        assert!(calls_to(&log, FLEE_SEARCH_BUILD_NODE_PATH).is_empty());
        assert_timers_balanced(&log);
    }

    #[test]
    fn flee_resolves_an_unresolved_destination_first() {
        let (mut e, request, solution, _) = flee_setup();
        e.register(LOCATION_HAS_FLAG_2, |_, _| ret(0));
        e.call_log = Some(vec![]);
        assert!(e.call(0x006d_0be0, &args![request, solution]).bool());
        let log = log_of(&mut e);
        let resolves = calls_to(&log, LOCATION_RESOLVE_NAVMESH_INFO);
        assert_eq!(resolves[0], vec![request.addr() + 0x34, 0]);
        // The destination's navmesh is fetched too.
        let fetches = calls_to(&log, LOCATION_GET_NAVMESH_AND_TRIANGLE);
        assert_eq!(fetches.len(), 2);
        assert_eq!(fetches[1][0], request.addr() + 0x34);

        // A destination that cannot be resolved flees without a search.
        let (mut e, request, solution, _) = flee_setup();
        e.register(LOCATION_HAS_FLAG_2, |_, _| ret(0));
        e.register(LOCATION_RESOLVE_NAVMESH_INFO, |_, _| ret(0));
        e.call_log = Some(vec![]);
        e.call(0x006d_0be0, &args![request, solution]);
        let log = log_of(&mut e);
        assert_eq!(timer_lines(&log), vec![0xb3, 0x12f]);
    }

    #[test]
    fn flee_pulls_the_goal_to_the_requests_radius_from_the_found_point() {
        let (mut e, request, solution, added) = flee_setup();
        // `006d4570` finds a point; the goal is 1 away from it (less than
        // the radius 2), so the goal becomes found point + unit * 2.
        e.register(FIND_PATH_POINT, |_, _| ret(1));
        e.register(REQUEST_ACTOR_RADIUS, |_, _| ret_float(2.0));
        e.register(POINT3_UNITIZE_GET_LENGTH, |_, _| ret_float(1.0));
        let pulled = e.mem.alloc(12);
        set_point(&mut e, pulled, [7.0, 8.0, 9.0]);
        e.register(POINT3_TIMES_SCALAR, |_, a| ret(a[1]));
        e.register_double(POINT3_ADD, move |_, _| ret(pulled));
        e.call_log = Some(vec![]);
        assert!(e.call(0x006d_0be0, &args![request, solution]).bool());
        let log = log_of(&mut e);
        assert_eq!(calls_to(&log, POINT3_TIMES_SCALAR)[0][2], 2.0f32.to_bits());
        assert_eq!(added.borrow()[1], [7.0, 8.0, 9.0]);
    }

    #[test]
    fn flee_leaves_the_goal_alone_when_it_is_beyond_the_radius() {
        let (mut e, request, solution, added) = flee_setup();
        e.register(FIND_PATH_POINT, |_, _| ret(1));
        e.register(REQUEST_ACTOR_RADIUS, |_, _| ret_float(2.0));
        e.register(POINT3_UNITIZE_GET_LENGTH, |_, _| ret_float(3.0));
        e.call_log = Some(vec![]);
        assert!(e.call(0x006d_0be0, &args![request, solution]).bool());
        let log = log_of(&mut e);
        assert!(calls_to(&log, POINT3_TIMES_SCALAR).is_empty());
        assert_eq!(added.borrow()[1], [10.0, 20.0, 30.0]);
    }

    #[test]
    fn flee_sets_the_first_tangent_from_the_heading() {
        let (mut e, request, solution, _) = flee_setup();
        e.register(REQUEST_FIRST_TANGENT_USES_HEADING, |_, _| ret(1));
        e.register(SOLUTION_NODE_COUNT, |_, _| ret(3));
        e.register(SOLUTION_NODE_AT, |_, a| ret(0x00d0_0000 + 0x100 * a[1]));
        e.register(NODE_LOCATION, |_, a| ret(a[0] + 4));
        e.register(REQUEST_INITIAL_PATH_HEADING, |_, _| ret_float(0.5));
        e.register(REQUEST_ACTOR_RADIUS, |_, _| ret_float(2.0));
        e.call_log = Some(vec![]);
        assert!(e.call(0x006d_0be0, &args![request, solution]).bool());
        let log = log_of(&mut e);
        let tangent = &calls_to(&log, BUILD_FIRST_TANGENT)[0];
        assert_eq!(tangent[1], 0.5f32.to_bits());
        assert_eq!(tangent[4], 2.0f32.to_bits());
        let copies = calls_to(&log, LOCATION_COPY_CONSTRUCT);
        assert_eq!(copies[0][1], 0x00d0_0004);
        assert_eq!(copies[1][1], 0x00d0_0104);
        assert_eq!(copies[0][0], tangent[2]);
        assert_eq!(copies[1][0], tangent[3]);
        assert_eq!(
            calls_to(&log, NODE_SET_POSITION),
            vec![vec![0x00d0_0000, tangent[0]]]
        );
        assert_eq!(calls_to(&log, LOCATION_DESTRUCT).len(), 3);

        // Two path nodes are not enough for a tangent.
        let (mut e, request, solution, _) = flee_setup();
        e.register(REQUEST_FIRST_TANGENT_USES_HEADING, |_, _| ret(1));
        e.register(SOLUTION_NODE_COUNT, |_, _| ret(2));
        e.call_log = Some(vec![]);
        e.call(0x006d_0be0, &args![request, solution]);
        assert!(calls_to(&log_of(&mut e), BUILD_FIRST_TANGENT).is_empty());
    }

    // -- 006d12a0 ----------------------------------------------------------

    /// A request with a minimum flee distance of 4, a resolvable origin
    /// and the circle axes in memory; the locations built for the two
    /// virtual nodes are recorded.
    fn flee_point_setup() -> RecordedFixture {
        let mut e = engine();
        let (request, solution) = objects(&mut e);
        geometry(&mut e, request);
        vector_math(&mut e);
        e.mem.set_f32(request.addr() + 0xb0, 4.0);
        set_point(&mut e, CIRCLE_AXIS_A, [0.0, 1.0, 0.0]);
        set_point(&mut e, CIRCLE_AXIS_B, [1.0, 0.0, 0.0]);
        let built = record_points(&mut e, LOCATION_CONSTRUCT_FROM_PARTS, 1);
        (e, request, solution, built)
    }

    #[test]
    fn flee_point_runs_away_from_the_avoid_nodes() {
        let (mut e, request, solution, built) = flee_point_setup();
        e.register(REQUEST_AVOID_NODE_ARRAY, |_, _| ret(0x00d1_0000));
        e.register(ARRAY_IS_EMPTY, |_, _| ret(0));
        // The nodes push (1, 0, 0): away is the direction (0, 2, 3).
        e.register(AVOID_ARRAY_COMPUTE_PUSH, |e, a| {
            set_point(e, a[2], [1.0, 0.0, 0.0]);
            ret_float(1.0)
        });
        e.call_log = Some(vec![]);
        assert!(e.call(0x006d_12a0, &args![request, solution]).bool());
        let log = log_of(&mut e);
        assert_eq!(timer_lines(&log), vec![0x12f]);
        assert_timers_balanced(&log);
        let built = built.borrow();
        assert_eq!(built.len(), 2);
        assert_close(built[0], [1.0, 2.0, 3.0]);
        let length = 13.0f32.sqrt();
        assert_close(
            built[1],
            [1.0, 2.0 + 4.0 * 2.0 / length, 3.0 + 4.0 * 3.0 / length],
        );
        assert_eq!(
            e.get(solution, PathingSolution::iFirstLoadedVirtualNodeIndex),
            0
        );
        assert_eq!(
            e.get(solution, PathingSolution::iLastLoadedVirtualNodeIndex),
            1
        );
    }

    #[test]
    fn flee_point_without_avoid_nodes_takes_a_random_point_on_the_circle() {
        let (mut e, request, solution, built) = flee_point_setup();
        // An empty avoid array counts as none.
        e.register(REQUEST_AVOID_NODE_ARRAY, |_, _| ret(0x00d1_0000));
        e.register(ARRAY_IS_EMPTY, |_, _| ret(1));
        // The random angle is 3/4 of a turn minus pi: pi / 2.
        e.set_global(
            RANDOM_ANGLE_SCALE,
            std::f64::consts::PI * 2.0 / 4294967296.0,
        );
        e.set_global(PI_AS_FLOAT, std::f32::consts::PI as f64);
        e.register(RANDOM_OBJECT, |_, _| ret(0x00d2_0000));
        e.register_double(RANDOM_UNSIGNED, |_, a| {
            assert_eq!(a[0], 0x00d2_0000);
            ret(0xc000_0000)
        });
        e.register(ANGLE_FUNCTION_004E44B0, |_, a| {
            ret_float(float_arg(a[0]).sin())
        });
        e.register(ANGLE_FUNCTION_004E4470, |_, a| {
            ret_float(float_arg(a[0]).cos())
        });
        // The origin has a cell with land at height 44.
        e.register(LOCATION_GET_CELL, |_, _| ret(0x00e0_0000));
        e.register(LOCATION_GET_WORLDSPACE, |_, _| ret(0x00e1_0000));
        e.register(CELL_GET_LAND_HEIGHT, |e, a| {
            e.mem.set_f32(a[2], 44.0);
            ret(1)
        });
        e.call_log = Some(vec![]);
        assert!(e.call(0x006d_12a0, &args![request, solution]).bool());
        let log = log_of(&mut e);
        let built = built.borrow();
        // Distance 4 along (0, 1, 0) (the sine of pi / 2 is 1); the cosine
        // of pi / 2 is about 0.
        assert_close(built[1], [1.0, 2.0 + 4.0, 44.0]);
        assert_close(built[0], [1.0, 2.0, 3.0]);
        let parts = calls_to(&log, LOCATION_CONSTRUCT_FROM_PARTS);
        assert_eq!(parts[0][2..], [0x00e0_0000, 0x00e1_0000]);
        assert_eq!(parts[1][2..], [0x00e0_0000, 0x00e1_0000]);
        // The land height is looked up for the new point only.
        assert_eq!(calls_to(&log, CELL_GET_LAND_HEIGHT).len(), 1);
    }

    #[test]
    fn flee_point_keeps_the_height_when_the_cell_has_no_land() {
        let (mut e, request, solution, built) = flee_point_setup();
        e.register(LOCATION_GET_CELL, |_, _| ret(0x00e0_0000));
        e.register(CELL_GET_LAND_HEIGHT, |_, _| ret(0));
        e.call(0x006d_12a0, &args![request, solution]);
        assert_eq!(built.borrow()[1][2], 3.0);
    }

    // -- 006d1520 ----------------------------------------------------------

    #[test]
    fn add_virtual_node_builds_the_node_from_the_resolved_location() {
        let mut e = engine();
        let solution: Ptr<PathingSolution> = e.new_object();
        let position = e.mem.alloc(12);
        e.register(VIRTUAL_NODE_ARRAY_ADD_SLOT, |_, _| ret(3));
        e.register_double(VIRTUAL_NODE_ARRAY_ELEMENT, |_, a| {
            assert_eq!(a[1], 3);
            ret(0x00d3_0000)
        });
        e.register_double(PLACEMENT_NEW, |_, a| {
            assert_eq!(a[..2], [0x30, 0x00d3_0000]);
            ret(a[1])
        });
        e.call_log = Some(vec![]);
        let index = e
            .call(
                0x006d_1520,
                &args![solution, Ptr::<()>::new(position), 0x11u32, 0x22u32],
            )
            .u32();
        assert_eq!(index, 3);
        let log = log_of(&mut e);
        let parts = &calls_to(&log, LOCATION_CONSTRUCT_FROM_PARTS)[0];
        let location = parts[0];
        assert_eq!(parts[1..], [position, 0x11, 0x22]);
        assert_eq!(
            calls_to(&log, LOCATION_RESOLVE_NAVMESH_INFO),
            vec![vec![location, 0]]
        );
        // The array is the solution's, at +8.
        assert_eq!(
            calls_to(&log, VIRTUAL_NODE_ARRAY_ADD_SLOT),
            vec![vec![solution.addr() + 8]]
        );
        assert_eq!(
            calls_to(&log, VIRTUAL_NODE_CONSTRUCT),
            vec![vec![0x00d3_0000, location]]
        );
        assert_eq!(calls_to(&log, LOCATION_DESTRUCT), vec![vec![location]]);

        // No block: no node is constructed.
        e.register(PLACEMENT_NEW, |_, _| ret(0));
        e.call_log = Some(vec![]);
        e.call(
            0x006d_1520,
            &args![solution, Ptr::<()>::new(position), 0u32, 0u32],
        );
        assert!(calls_to(&log_of(&mut e), VIRTUAL_NODE_CONSTRUCT).is_empty());
    }

    // -- 006d1600, 006d1620 --------------------------------------------------

    #[test]
    fn random_angle_maps_a_random_number_to_minus_pi_to_pi() {
        let mut e = engine();
        e.set_global(
            RANDOM_ANGLE_SCALE,
            std::f64::consts::PI * 2.0 / 4294967296.0,
        );
        e.set_global(PI_AS_FLOAT, std::f32::consts::PI as f64);
        e.register(RANDOM_OBJECT, |_, _| ret(0x00d2_0000));
        e.register_double(RANDOM_UNSIGNED, |_, a| {
            assert_eq!(a[0], 0x00d2_0000);
            ret(0x4000_0000)
        });
        let angle = e.call(0x006d_1600, &args![]).f32();
        assert!((angle + std::f32::consts::FRAC_PI_2).abs() < 1e-6);
        e.register(RANDOM_UNSIGNED, |_, _| ret(0));
        let angle = e.call(0x006d_1620, &args![Ptr::<()>::new(1)]).f32();
        assert_eq!(angle, -(std::f32::consts::PI));
        e.register(RANDOM_UNSIGNED, |_, _| ret(u32::MAX));
        let angle = e.call(0x006d_1620, &args![Ptr::<()>::new(1)]).f32();
        assert!(angle > 3.0 && angle <= std::f32::consts::PI + 1e-6);
    }

    // -- 006d1660 ----------------------------------------------------------

    /// A line-of-sight request on a resolvable navmesh whose search finds a
    /// path; the copy of the request the solver builds is typed 0 (base)
    /// and the base solver succeeds.
    fn los_setup() -> (Engine, Ptr<PathingRequest>, Ptr<PathingSolution>) {
        let mut e = engine();
        let (request, solution) = objects(&mut e);
        geometry(&mut e, request);
        e.register(LOS_SEARCH_BUILD_NODE_PATH, |_, _| ret(1));
        e.register(BASE_SOLVER_RUN, |_, _| ret(1));
        e.register(LOCATION_GET_WORLDSPACE, |e, a| ret(e.mem.u32(a[0] + 0x1c)));
        let mut slots = vec![0u32; 5];
        slots[4] = ROUTE_TYPE_FUNCTION;
        e.put_vtable(REQUEST_VTABLE, &slots);
        e.register(ROUTE_TYPE_FUNCTION, |_, _| ret(0));
        e.register(REQUEST_COPY_CONSTRUCT, |e, a| {
            e.mem.set_u32(a[0], REQUEST_VTABLE);
            Ret::default()
        });
        (e, request, solution)
    }

    #[test]
    fn los_solves_again_from_the_last_triangle_of_the_path() {
        let (mut e, request, solution) = los_setup();
        e.call_log = Some(vec![]);
        assert!(e.call(0x006d_1660, &args![request, solution]).bool());
        let log = log_of(&mut e);
        assert_eq!(timer_lines(&log), vec![0x17a, 0x6a]);
        assert_timers_balanced(&log);
        // The destination of the copy is a location on the last triangle.
        let copy = calls_to(&log, REQUEST_COPY_CONSTRUCT)[0].clone();
        assert_eq!(copy[1], request.addr());
        let location = &calls_to(&log, LOCATION_CONSTRUCT_FROM_MESH_POINT)[0];
        assert_eq!(location[2], LAST_NAVMESH + 0x1000);
        assert_eq!(location[3], 7);
        assert_eq!(
            calls_to(&log, LOCATION_ASSIGN),
            vec![vec![copy[0] + 0x34, location[0]]]
        );
        assert_eq!(calls_to(&log, SOLUTION_CLEAR), vec![vec![solution.addr()]]);
        assert_eq!(calls_to(&log, REQUEST_COPY_DESTRUCT), vec![vec![copy[0]]]);
        assert_eq!(calls_to(&log, LOS_SEARCH_DESTRUCT).len(), 1);
    }

    #[test]
    fn los_fails_when_the_origin_cannot_be_resolved() {
        let (mut e, request, solution) = los_setup();
        e.register(LOCATION_RESOLVE_CLOSEST, |_, _| ret(0));
        e.call_log = Some(vec![]);
        assert!(!e.call(0x006d_1660, &args![request, solution]).bool());
        let log = log_of(&mut e);
        assert_eq!(timer_lines(&log), vec![0x17a]);
        assert!(calls_to(&log, NAV_HOLDER_CONSTRUCT).is_empty());

        let (mut e, request, solution) = los_setup();
        e.register(LOCATION_GET_NAVMESH_AND_TRIANGLE, |_, _| ret(0));
        e.call_log = Some(vec![]);
        assert!(!e.call(0x006d_1660, &args![request, solution]).bool());
        let log = log_of(&mut e);
        assert!(calls_to(&log, LOS_SEARCH_CONSTRUCT).is_empty());
        assert_eq!(calls_to(&log, NAV_HOLDER_RELEASE).len(), 1);
        assert_timers_balanced(&log);
    }

    #[test]
    fn los_tries_the_searchs_recorded_nodes_in_order() {
        // The first search fails; the recorded nodes at +0x2090, +0x2094 and
        // +0x2098 are tried in turn until one gives a path.
        for (winner, expected_calls) in [(0usize, 1usize), (1, 2), (2, 3)] {
            let (mut e, request, solution) = los_setup();
            e.register(LOS_SEARCH_BUILD_NODE_PATH, |_, _| ret(0));
            e.register(LOS_SEARCH_CONSTRUCT, |e, a| {
                e.mem.set_u32(a[0] + 0x2090, 0x1111);
                e.mem.set_u32(a[0] + 0x2094, 0x2222);
                e.mem.set_u32(a[0] + 0x2098, 0x3333);
                Ret::default()
            });
            let calls = Rc::new(RefCell::new(Vec::new()));
            let sink = calls.clone();
            e.register_double(LOS_SEARCH_PATH_FROM_NODE, move |_, a| {
                sink.borrow_mut().push(a[1]);
                ret((sink.borrow().len() == winner + 1) as u32)
            });
            e.call_log = Some(vec![]);
            assert!(e.call(0x006d_1660, &args![request, solution]).bool());
            assert_eq!(
                calls.borrow()[..],
                [0x1111, 0x2222, 0x3333][..expected_calls]
            );
        }
    }

    #[test]
    fn los_moves_the_last_virtual_node_to_the_map_triangle_when_all_fail() {
        let (mut e, request, solution) = los_setup();
        e.register(LOS_SEARCH_BUILD_NODE_PATH, |_, _| ret(0));
        // No recorded node gives a path; the LOS map points at triangle 6.
        e.register(NI_POINTER_GET_FROM_FIELD, |_, a| ret(a[0] + 0x500));
        e.register(LOS_MAP_FIND_TRIANGLE, |e, a| {
            e.mem.set_u16(a[2], 6);
            ret(1)
        });
        e.register(SOLUTION_VIRTUAL_NODE_AT, |_, _| ret(0x00d4_0000));
        e.set(solution, PathingSolution::iLastLoadedVirtualNodeIndex, -1);
        e.call_log = Some(vec![]);
        assert!(!e.call(0x006d_1660, &args![request, solution]).bool());
        let log = log_of(&mut e);
        // No loaded node: the start is added and the index reset to 0, but
        // the node looked up is the one for the old index (-1).
        assert_eq!(calls_to(&log, SOLUTION_ADD_VIRTUAL_NODE).len(), 1);
        assert_eq!(
            e.get(solution, PathingSolution::iLastLoadedVirtualNodeIndex),
            0
        );
        assert_eq!(
            calls_to(&log, SOLUTION_VIRTUAL_NODE_AT),
            vec![vec![solution.addr(), u32::MAX]]
        );
        let center = &calls_to(&log, NAVMESH_GET_CENTER)[0];
        assert_eq!(center[2], 6);
        let mesh_point = &calls_to(&log, LOCATION_CONSTRUCT_FROM_MESH_POINT)[0];
        assert_eq!(mesh_point[3], 6);
        assert_eq!(
            calls_to(&log, VIRTUAL_NODE_SET_LOCATION),
            vec![vec![0x00d4_0000, mesh_point[0]]]
        );
        assert_timers_balanced(&log);
    }

    #[test]
    fn los_keeps_the_loaded_node_index_when_there_is_one() {
        let (mut e, request, solution) = los_setup();
        e.register(LOS_SEARCH_BUILD_NODE_PATH, |_, _| ret(0));
        e.register(NI_POINTER_GET_FROM_FIELD, |_, a| ret(a[0] + 0x500));
        e.register(LOS_MAP_FIND_TRIANGLE, |_, _| ret(1));
        e.register(SOLUTION_VIRTUAL_NODE_AT, |_, _| ret(0));
        e.set(solution, PathingSolution::iLastLoadedVirtualNodeIndex, 4);
        e.call_log = Some(vec![]);
        e.call(0x006d_1660, &args![request, solution]);
        let log = log_of(&mut e);
        assert!(calls_to(&log, SOLUTION_ADD_VIRTUAL_NODE).is_empty());
        assert_eq!(
            calls_to(&log, SOLUTION_VIRTUAL_NODE_AT),
            vec![vec![solution.addr(), 4]]
        );
        // A missing node means nothing is moved.
        assert!(calls_to(&log, VIRTUAL_NODE_SET_LOCATION).is_empty());
    }

    // -- small accessors ---------------------------------------------------

    #[test]
    fn destination_and_origin_setters_assign_the_locations() {
        let mut e = engine();
        let request = e.mem.alloc(0x120);
        let location = e.mem.alloc(0x28);
        e.call_log = Some(vec![]);
        e.call(
            0x006d_1bc0,
            &args![Ptr::<()>::new(request), Ptr::<()>::new(location)],
        );
        e.call(
            0x006d_3ac0,
            &args![Ptr::<()>::new(request), Ptr::<()>::new(location)],
        );
        let log = log_of(&mut e);
        assert_eq!(
            calls_to(&log, LOCATION_ASSIGN),
            vec![
                vec![request + 0x34, location],
                vec![request + 0xc, location]
            ]
        );
    }

    #[test]
    fn los_search_getters_read_their_fields() {
        let mut e = engine();
        let search = e.mem.alloc(0x2100);
        e.mem.set_u32(search + 0x2090, 0xa1);
        e.mem.set_u32(search + 0x2094, 0xa2);
        e.mem.set_u32(search + 0x2098, 0xa3);
        e.register(NI_POINTER_GET_FROM_FIELD, |_, a| ret(a[0]));
        let search = Ptr::<()>::new(search);
        assert_eq!(e.call(0x006d_1be0, &args![search]).u32(), 0xa1);
        assert_eq!(e.call(0x006d_1c00, &args![search]).u32(), 0xa2);
        assert_eq!(e.call(0x006d_1c20, &args![search]).u32(), 0xa3);
        // The LOS map is the pointer held by the `NiPointer` at +0x2070.
        assert_eq!(
            e.call(0x006d_1c40, &args![search]).u32(),
            search.addr() + 0x2070
        );
    }

    // -- 006d1c60 ----------------------------------------------------------

    /// A close-point request whose search succeeds.
    fn close_point_setup() -> RecordedFixture {
        let mut e = engine();
        let (request, solution) = objects(&mut e);
        geometry(&mut e, request);
        e.register(CLOSE_POINT_SEARCH_BUILD_NODE_PATH, |_, _| ret(1));
        let added = record_points(&mut e, SOLUTION_ADD_VIRTUAL_NODE, 1);
        (e, request, solution, added)
    }

    #[test]
    fn close_point_smooths_the_path_into_the_solution() {
        let (mut e, request, solution, added) = close_point_setup();
        e.call_log = Some(vec![]);
        assert!(e.call(0x006d_1c60, &args![request, solution]).bool());
        let log = log_of(&mut e);
        assert_eq!(timer_lines(&log), vec![0x1f5]);
        assert_timers_balanced(&log);
        assert_eq!(*added.borrow(), vec![[1.0, 2.0, 3.0], [10.0, 20.0, 30.0]]);
        let smooth = &calls_to(&log, PATH_SMOOTHER_SMOOTH)[0];
        assert_eq!(smooth.len(), 9);
        assert_eq!(smooth[1], request.addr());
        assert_eq!(smooth[6], solution.addr());
        assert_eq!(smooth[7..], [0, 1]);
        // The loaded range ends at the last virtual node.
        assert_eq!(
            e.get(solution, PathingSolution::iFirstLoadedVirtualNodeIndex),
            0
        );
        assert_eq!(
            e.get(solution, PathingSolution::iLastLoadedVirtualNodeIndex),
            4
        );
        // Destroyed again: smoother, node arrays, search, holder.
        assert_eq!(
            calls_to(&log, PATH_SMOOTHER_DESTRUCT),
            vec![vec![smooth[0]]]
        );
        assert_eq!(calls_to(&log, CLOSE_POINT_SEARCH_DESTRUCT).len(), 1);
        assert_eq!(calls_to(&log, NAV_HOLDER_RELEASE).len(), 1);
    }

    #[test]
    fn close_point_fails_without_navmesh_info_or_path() {
        let (mut e, request, solution, _) = close_point_setup();
        e.register(LOCATION_RESOLVE_NAVMESH_INFO, |_, _| ret(0));
        e.call_log = Some(vec![]);
        assert!(!e.call(0x006d_1c60, &args![request, solution]).bool());
        assert!(calls_to(&log_of(&mut e), NAV_HOLDER_CONSTRUCT).is_empty());

        let (mut e, request, solution, added) = close_point_setup();
        e.register(CLOSE_POINT_SEARCH_BUILD_NODE_PATH, |_, _| ret(0));
        e.call_log = Some(vec![]);
        assert!(!e.call(0x006d_1c60, &args![request, solution]).bool());
        let log = log_of(&mut e);
        assert!(added.borrow().is_empty());
        assert_eq!(calls_to(&log, CLOSE_POINT_SEARCH_DESTRUCT).len(), 1);
        assert_timers_balanced(&log);

        let (mut e, request, solution, _) = close_point_setup();
        e.register(LOCATION_GET_NAVMESH_AND_TRIANGLE, |_, _| ret(0));
        e.call_log = Some(vec![]);
        assert!(!e.call(0x006d_1c60, &args![request, solution]).bool());
        let log = log_of(&mut e);
        assert_eq!(calls_to(&log, NAV_HOLDER_RELEASE).len(), 1);
        assert!(calls_to(&log, CLOSE_POINT_SEARCH_CONSTRUCT).is_empty());
    }

    // -- 006d1f80 ----------------------------------------------------------

    #[test]
    fn hide_searches_with_both_start_navmeshes() {
        let mut e = engine();
        let (request, solution) = objects(&mut e);
        geometry(&mut e, request);
        e.register(HIDE_SEARCH_BUILD_NODE_PATH, |_, _| ret(1));
        let added = record_points(&mut e, SOLUTION_ADD_VIRTUAL_NODE, 1);
        e.call_log = Some(vec![]);
        assert!(e.call(0x006d_1f80, &args![request, solution]).bool());
        let log = log_of(&mut e);
        assert_eq!(timer_lines(&log), vec![0x23f]);
        assert_timers_balanced(&log);
        assert_eq!(*added.borrow(), vec![[1.0, 2.0, 3.0], [10.0, 20.0, 30.0]]);
        // Both locations are resolved; the search gets both starts.
        let navs = calls_to(&log, LOCATION_GET_NAVMESH_AND_TRIANGLE);
        assert_eq!(navs[0][0], request.addr() + 0xc);
        assert_eq!(navs[1][0], request.addr() + 0x34);
        let build = &calls_to(&log, HIDE_SEARCH_BUILD_NODE_PATH)[0];
        assert_eq!(build.len(), 6);
        assert_eq!(build[1], request.addr());
        assert_ne!(build[2], build[3]);
        assert_eq!(e.mem.u32(build[2]), NAVMESH);
        let smooth = &calls_to(&log, PATH_SMOOTHER_SMOOTH)[0];
        assert_eq!(smooth[7..], [0, 1]);
        assert_eq!(
            e.get(solution, PathingSolution::iLastLoadedVirtualNodeIndex),
            4
        );
        assert_eq!(
            calls_to(&log, NAV_HOLDER_CONSTRUCT).len(),
            calls_to(&log, NAV_HOLDER_RELEASE).len()
        );
    }

    #[test]
    fn hide_fails_when_a_location_or_the_search_fails() {
        // The origin cannot be resolved.
        let mut e = engine();
        let (request, solution) = objects(&mut e);
        geometry(&mut e, request);
        e.register(HIDE_SEARCH_BUILD_NODE_PATH, |_, _| ret(1));
        e.register(LOCATION_RESOLVE_CLOSEST, |_, _| ret(0));
        e.call_log = Some(vec![]);
        assert!(!e.call(0x006d_1f80, &args![request, solution]).bool());
        assert!(calls_to(&log_of(&mut e), NAV_HOLDER_CONSTRUCT).is_empty());

        // The destination cannot be resolved (the origin can).
        let mut e = engine();
        let (request, solution) = objects(&mut e);
        geometry(&mut e, request);
        let destination = request.addr() + 0x34;
        e.register_double(LOCATION_RESOLVE_CLOSEST, move |_, a| {
            ret((a[0] != destination) as u32)
        });
        e.call_log = Some(vec![]);
        assert!(!e.call(0x006d_1f80, &args![request, solution]).bool());
        let log = log_of(&mut e);
        assert_eq!(calls_to(&log, NAV_HOLDER_RELEASE).len(), 1);
        assert_timers_balanced(&log);

        // The destination has no navmesh triangle.
        let mut e = engine();
        let (request, solution) = objects(&mut e);
        geometry(&mut e, request);
        let destination = request.addr() + 0x34;
        e.register_double(LOCATION_GET_NAVMESH_AND_TRIANGLE, move |_, a| {
            ret((a[0] != destination) as u32)
        });
        e.call_log = Some(vec![]);
        assert!(!e.call(0x006d_1f80, &args![request, solution]).bool());
        assert_eq!(calls_to(&log_of(&mut e), NAV_HOLDER_RELEASE).len(), 2);

        // The search finds nothing.
        let mut e = engine();
        let (request, solution) = objects(&mut e);
        geometry(&mut e, request);
        e.call_log = Some(vec![]);
        assert!(!e.call(0x006d_1f80, &args![request, solution]).bool());
        let log = log_of(&mut e);
        assert_eq!(calls_to(&log, HIDE_SEARCH_DESTRUCT).len(), 1);
        assert!(calls_to(&log, SOLUTION_ADD_VIRTUAL_NODE).is_empty());
    }

    // -- 006d23d0 ----------------------------------------------------------

    /// An optimal-location request: distances 4 and 8 (halfway 6) and
    /// absolute bounds 5 and 7, origin (1, 2, 3), destination (11, 2, 3),
    /// a closest navmesh point equal to the aim point (5, 2, 3), a max-cost
    /// search that finds a path ending at (5, 2, 3). The cost limit the
    /// search is given is recorded.
    fn optimal_setup() -> (Engine, Ptr<PathingRequest>, Ptr<PathingSolution>, Limits) {
        let mut e = engine();
        let (request, solution) = objects(&mut e);
        geometry(&mut e, request);
        vector_math(&mut e);
        e.mem.set_f32(request.addr() + 0xb0, 4.0);
        e.mem.set_f32(request.addr() + 0xb4, 8.0);
        e.register(REQUEST_ABSOLUTE_MINIMUM_DISTANCE, |_, _| ret_float(5.0));
        e.register(REQUEST_ABSOLUTE_MAXIMUM_DISTANCE, |_, _| ret_float(7.0));
        e.register(FIND_CLOSEST_POINT_ON_NAVMESH, |e, a| {
            let aim = point(e, a[2]);
            set_point(e, a[3], aim);
            ret(1)
        });
        let limits = Rc::new(RefCell::new(Vec::new()));
        let sink = limits.clone();
        e.register_double(MAX_COST_SEARCH_BUILD_NODE_PATH, move |e, a| {
            sink.borrow_mut().push(e.mem.f32(a[0] + 0x20a4));
            ret(1)
        });
        let end = e.mem.alloc(12);
        set_point(&mut e, end, [5.0, 2.0, 3.0]);
        e.register_double(NODE_TO_POSITION, move |_, _| ret(end));
        e.register(PATH_BUILDER_BUILD, |_, _| ret(1));
        (e, request, solution, limits)
    }

    #[test]
    fn optimal_location_builds_the_path_to_the_chosen_point() {
        let (mut e, request, solution, limits) = optimal_setup();
        let added = record_points(&mut e, SOLUTION_ADD_VIRTUAL_NODE, 1);
        e.call_log = Some(vec![]);
        assert!(e.call(0x006d_23d0, &args![request, solution]).bool());
        let log = log_of(&mut e);
        assert_eq!(timer_lines(&log), vec![0x29c]);
        assert_timers_balanced(&log);
        // The cost limit is twice the slack |6 - 10| = 4.
        assert_eq!(*limits.borrow(), vec![8.0]);
        assert_eq!(*added.borrow(), vec![[1.0, 2.0, 3.0], [5.0, 2.0, 3.0]]);
        let build = &calls_to(&log, PATH_BUILDER_BUILD)[0];
        assert_eq!(build[1], request.addr());
        assert_eq!(build[6], solution.addr());
        assert_eq!(
            e.get(solution, PathingSolution::iFirstLoadedVirtualNodeIndex),
            0
        );
        assert_eq!(
            e.get(solution, PathingSolution::iLastLoadedVirtualNodeIndex),
            1
        );
        // Everything is destroyed again.
        assert_eq!(calls_to(&log, MAX_COST_SEARCH_DESTRUCT).len(), 1);
        assert_eq!(calls_to(&log, NAVMESH_LIST_DESTRUCT).len(), 1);
        assert_eq!(calls_to(&log, NODE_ARRAY_C_DESTRUCT).len(), 1);
        assert_eq!(calls_to(&log, PATH_BUILDER_DESTRUCT).len(), 1);
        assert_eq!(calls_to(&log, LOCATION_DESTRUCT).len(), 1);
        assert_eq!(calls_to(&log, NAV_HOLDER_RELEASE).len(), 1);
    }

    #[test]
    fn optimal_location_aims_between_the_two_distances() {
        let (mut e, request, solution, _) = optimal_setup();
        e.call_log = Some(vec![]);
        e.call(0x006d_23d0, &args![request, solution]);
        let log = log_of(&mut e);
        // Closest-point query: worldspace, cell, aim point, result.
        let query = &calls_to(&log, FIND_CLOSEST_POINT_ON_NAVMESH)[0];
        assert_eq!(query.len(), 4);
        // The goal location is built from the closest point and the origin.
        let goal = &calls_to(&log, LOCATION_CONSTRUCT_FROM_POINT_AND_LOCATION)[0];
        assert_eq!(goal[1], query[3]);
        assert_eq!(goal[2], request.addr() + 0xc);
    }

    #[test]
    fn optimal_location_fails_when_the_closest_point_is_too_far() {
        let (mut e, request, solution, limits) = optimal_setup();
        // The closest point is 10 from the aim point; the slack is 4.
        e.register(FIND_CLOSEST_POINT_ON_NAVMESH, |e, a| {
            let aim = point(e, a[2]);
            set_point(e, a[3], [aim[0] + 10.0, aim[1], aim[2]]);
            ret(1)
        });
        e.call_log = Some(vec![]);
        assert!(!e.call(0x006d_23d0, &args![request, solution]).bool());
        let log = log_of(&mut e);
        assert!(limits.borrow().is_empty());
        assert_eq!(calls_to(&log, NAV_HOLDER_RELEASE).len(), 1);
        assert_timers_balanced(&log);

        // No closest point at all.
        let (mut e, request, solution, _) = optimal_setup();
        e.register(FIND_CLOSEST_POINT_ON_NAVMESH, |_, _| ret(0));
        assert!(!e.call(0x006d_23d0, &args![request, solution]).bool());
    }

    #[test]
    fn optimal_location_fails_when_the_origin_is_unresolved() {
        let (mut e, request, solution, _) = optimal_setup();
        e.register(LOCATION_RESOLVE_CLOSEST, |_, _| ret(0));
        e.call_log = Some(vec![]);
        assert!(!e.call(0x006d_23d0, &args![request, solution]).bool());
        assert!(calls_to(&log_of(&mut e), NAV_HOLDER_CONSTRUCT).is_empty());

        let (mut e, request, solution, _) = optimal_setup();
        e.register(LOCATION_GET_NAVMESH_AND_TRIANGLE, |_, _| ret(0));
        e.call_log = Some(vec![]);
        assert!(!e.call(0x006d_23d0, &args![request, solution]).bool());
        assert_eq!(calls_to(&log_of(&mut e), NAV_HOLDER_RELEASE).len(), 1);
    }

    #[test]
    fn optimal_location_without_a_path_uses_the_best_node_or_the_base_solver() {
        // No path, but the node array holds nodes: the best node of the
        // search (copied from +8 of it) gives the point.
        let (mut e, request, solution, _) = optimal_setup();
        e.register(MAX_COST_SEARCH_BUILD_NODE_PATH, |_, _| ret(0));
        e.register(ARRAY_IS_EMPTY, |_, _| ret(0));
        let best = e.mem.alloc(0x20);
        for i in 0..5 {
            e.mem.set_u32(best + 8 + 4 * i, 0xa0 + i);
        }
        e.register_double(MAX_COST_SEARCH_BEST_NODE, move |_, _| ret(best));
        let seen = Rc::new(RefCell::new(Vec::new()));
        let sink = seen.clone();
        let end = e.mem.alloc(12);
        set_point(&mut e, end, [5.0, 2.0, 3.0]);
        e.register_double(NODE_TO_POSITION, move |e, a| {
            for i in 0..5 {
                sink.borrow_mut().push(e.mem.u32(a[0] + 4 * i));
            }
            ret(end)
        });
        assert!(e.call(0x006d_23d0, &args![request, solution]).bool());
        assert_eq!(*seen.borrow(), vec![0xa0, 0xa1, 0xa2, 0xa3, 0xa4]);

        // No path and no nodes: the base solver runs when the direct
        // distance (10) exceeds the absolute maximum, else it fails.
        for (absolute_maximum, expect_base) in [(7.0f32, true), (10.0, false), (20.0, false)] {
            let (mut e, request, solution, _) = optimal_setup();
            e.register(MAX_COST_SEARCH_BUILD_NODE_PATH, |_, _| ret(0));
            e.register(ARRAY_IS_EMPTY, |_, _| ret(1));
            e.register_double(REQUEST_ABSOLUTE_MAXIMUM_DISTANCE, move |_, _| {
                ret_float(absolute_maximum)
            });
            e.register(BASE_SOLVER_RUN, |_, _| ret(1));
            e.call_log = Some(vec![]);
            let result = e.call(0x006d_23d0, &args![request, solution]).bool();
            let log = log_of(&mut e);
            assert_eq!(result, expect_base, "bound {absolute_maximum}");
            assert_eq!(
                !calls_to(&log, BASE_SOLVER_CONSTRUCT).is_empty(),
                expect_base
            );
            assert!(calls_to(&log, PATH_BUILDER_BUILD).is_empty());
            assert_eq!(calls_to(&log, MAX_COST_SEARCH_DESTRUCT).len(), 1);
            assert_timers_balanced(&log);
        }
    }

    #[test]
    fn optimal_location_requires_the_point_between_the_absolute_bounds() {
        // The chosen point is 6 from the destination: squared 36.
        for (minimum, maximum, expected) in [
            (5.0f32, 7.0f32, true),
            (6.5, 7.0, false),
            (5.0, 5.5, false),
            (6.0, 6.0, true),
        ] {
            let (mut e, request, solution, _) = optimal_setup();
            e.register_double(REQUEST_ABSOLUTE_MINIMUM_DISTANCE, move |_, _| {
                ret_float(minimum)
            });
            e.register_double(REQUEST_ABSOLUTE_MAXIMUM_DISTANCE, move |_, _| {
                ret_float(maximum)
            });
            e.call_log = Some(vec![]);
            let result = e.call(0x006d_23d0, &args![request, solution]).bool();
            let log = log_of(&mut e);
            assert_eq!(result, expected, "bounds {minimum} {maximum}");
            assert_eq!(!calls_to(&log, PATH_BUILDER_BUILD).is_empty(), expected);
            assert_timers_balanced(&log);
            assert_eq!(calls_to(&log, LOCATION_DESTRUCT).len(), 1);
        }
    }

    #[test]
    fn optimal_location_marks_an_unresolved_goal_and_uses_the_node_position() {
        // With navmesh info the goal location has no flag and the chosen
        // point is the closest point; without it the flag is set and the
        // point comes from the last node of the path.
        let (mut e, request, solution, _) = optimal_setup();
        e.register(LOCATION_HAS_FLAG_2, |e, a| {
            ret(((e.mem.u8(a[0] + 0x26) & 2) != 0) as u32)
        });
        e.register(LOCATION_RESOLVE_NAVMESH_INFO, |_, _| ret(1));
        let added = record_points(&mut e, SOLUTION_ADD_VIRTUAL_NODE, 1);
        e.call_log = Some(vec![]);
        assert!(e.call(0x006d_23d0, &args![request, solution]).bool());
        let log = log_of(&mut e);
        assert!(calls_to(&log, NODE_TO_POSITION).is_empty());
        assert_eq!(added.borrow()[1], [5.0, 2.0, 3.0]);

        let (mut e, request, solution, _) = optimal_setup();
        e.register(LOCATION_HAS_FLAG_2, |e, a| {
            ret(((e.mem.u8(a[0] + 0x26) & 2) != 0) as u32)
        });
        e.register(LOCATION_RESOLVE_NAVMESH_INFO, |_, _| ret(0));
        e.call_log = Some(vec![]);
        assert!(e.call(0x006d_23d0, &args![request, solution]).bool());
        assert_eq!(calls_to(&log_of(&mut e), NODE_TO_POSITION).len(), 1);
    }

    // -- 006d2c00 .. 006d2c60 -----------------------------------------------

    #[test]
    fn location_flag_setter_sets_bit_one() {
        let mut e = engine();
        let location: Ptr<PathingLocation> = e.new_object();
        e.set(location, PathingLocation::uiFlags, 0x11);
        e.call(0x006d_2c00, &args![location]);
        assert_eq!(e.get(location, PathingLocation::uiFlags), 0x13);
        e.call(0x006d_2c00, &args![location]);
        assert_eq!(e.get(location, PathingLocation::uiFlags), 0x13);
    }

    #[test]
    fn distance_accessors_read_and_write_their_floats() {
        let mut e = engine();
        let object = e.mem.alloc(0x2100);
        e.mem.set_f32(object + 0xb0, 1.5);
        e.mem.set_f32(object + 0xb4, 2.5);
        let object = Ptr::<()>::new(object);
        assert_eq!(e.call(0x006d_2c20, &args![object]).f32(), 1.5);
        assert_eq!(e.call(0x006d_2c40, &args![object]).f32(), 2.5);
        e.call(0x006d_2c60, &args![object, 7.25f32]);
        assert_eq!(e.mem.f32(object.addr() + 0x20a4), 7.25);
    }

    // -- 006d2c80 ----------------------------------------------------------

    /// A covered-move request whose base solve succeeds, with a solution of
    /// three path nodes; the cover check passes for the candidates in
    /// `passing` (a counter picks them in order) and writes (7, 8, 9) as
    /// the cover point of the request it checks.
    fn covered_move_setup(
        results: Vec<u32>,
    ) -> (Engine, Ptr<PathingRequest>, Ptr<PathingSolution>) {
        let mut e = engine();
        let (request, solution) = objects(&mut e);
        e.register(REQUEST_ORIGIN, |_, a| ret(a[0] + 0xc));
        e.register(MEMORY_ALLOCATE, |e, a| ret(e.mem.alloc(a[0])));
        e.register(MEMORY_FREE, |e, a| {
            e.mem.free(a[0]);
            Ret::default()
        });
        e.register(BASE_SOLVER_RUN, |_, _| ret(1));
        e.register(SOLUTION_NODE_COUNT, |_, _| ret(3));
        e.register(SOLUTION_NODE_AT, |_, a| ret(0x00d0_0000 + 0x100 * a[1]));
        e.register(NODE_LOCATION, |_, a| ret(a[0] + 4));
        e.register(COVERED_MOVE_COVER_POINT, |_, a| ret(a[0] + 0xc8));
        let mut results = results.into_iter();
        e.register_double(COVERED_MOVE_CHECK, move |e, a| {
            set_point(e, a[0] + 0xc8, [7.0, 8.0, 9.0]);
            ret(results.next().unwrap_or(0))
        });
        (e, request, solution)
    }

    #[test]
    fn covered_move_adopts_the_first_destination_that_passes() {
        let (mut e, request, solution) = covered_move_setup(vec![1]);
        e.set_global(COVERED_MOVE_DISTANCE, 512.0f32);
        // The copy's cover pointers are cleared before it is destroyed.
        let cleared = Rc::new(RefCell::new(Vec::new()));
        let sink = cleared.clone();
        e.register_double(COVERED_MOVE_DESTRUCT, move |e, a| {
            sink.borrow_mut()
                .push((e.mem.u32(a[0] + 0x104), e.mem.u32(a[0] + 0x108)));
            Ret::default()
        });
        e.call_log = Some(vec![]);
        assert!(e.call(0x006d_2c80, &args![request, solution]).bool());
        let log = log_of(&mut e);
        assert_eq!(timer_lines(&log), vec![0x322]);
        assert_timers_balanced(&log);
        // The cover point of the copy became the request's.
        assert_eq!(point(&e, request.addr() + 0xc8), [7.0, 8.0, 9.0]);
        // The candidate was the second node (index 1); the check gets the
        // distance and the flag 1.
        let check = &calls_to(&log, COVERED_MOVE_CHECK)[0];
        assert_eq!(check[1..], [512.0f32.to_bits(), 1]);
        assert_eq!(calls_to(&log, SOLUTION_NODE_AT)[0][1], 1);
        // The request keeps the cover objects, the copy does not.
        assert_eq!(*cleared.borrow(), vec![(0, 0)]);
        let cover = e.mem.u32(request.addr() + 0x104);
        let info = e.mem.u32(request.addr() + 0x108);
        assert!(cover != 0 && info != 0);
        let allocations = calls_to(&log, MEMORY_ALLOCATE);
        assert_eq!(allocations, vec![vec![0x78], vec![0x3c]]);
        assert!(calls_to(&log, MEMORY_FREE).is_empty());
        // The new solution is copied over and the request is finished.
        let copied = &calls_to(&log, SOLUTION_COPY_TO)[0];
        assert_eq!(copied[1], solution.addr());
        assert_eq!(
            calls_to(&log, REQUEST_SET_FINISHED),
            vec![vec![request.addr(), 1]]
        );
        // The request's destination is the cover target location.
        let assigned = calls_to(&log, LOCATION_ASSIGN);
        assert_eq!(assigned.last().unwrap()[0], request.addr() + 0x34);
        assert_eq!(calls_to(&log, SOLUTION_DESTRUCT).len(), 1);
        assert_eq!(calls_to(&log, COVERED_MOVE_DESTRUCT).len(), 1);
    }

    #[test]
    fn covered_move_without_a_hit_drops_the_cover_location() {
        let (mut e, request, solution) = covered_move_setup(vec![0, 0]);
        e.call_log = Some(vec![]);
        assert!(e.call(0x006d_2c80, &args![request, solution]).bool());
        let log = log_of(&mut e);
        // Candidates 1 and 2 were checked.
        assert_eq!(calls_to(&log, COVERED_MOVE_CHECK).len(), 2);
        assert!(calls_to(&log, SOLUTION_COPY_TO).is_empty());
        // The cover location object is deleted and the request's pointer
        // cleared; the search info stays.
        let allocations = calls_to(&log, MEMORY_ALLOCATE);
        assert_eq!(allocations.len(), 2);
        assert_eq!(e.mem.u32(request.addr() + 0x104), 0);
        assert_ne!(e.mem.u32(request.addr() + 0x108), 0);
        assert_eq!(calls_to(&log, MEMORY_FREE).len(), 1);
        assert_eq!(calls_to(&log, LOCATION_DESTRUCT).len(), 4);
        assert_eq!(
            calls_to(&log, REQUEST_SET_FINISHED),
            vec![vec![request.addr(), 1]]
        );
        assert_timers_balanced(&log);
        // Per candidate the copy takes it as destination and the running
        // previous location follows it.
        assert_eq!(calls_to(&log, LOCATION_ASSIGN).len(), 4);
    }

    #[test]
    fn covered_move_tries_on_when_the_copy_cannot_be_solved() {
        let (mut e, request, solution) = covered_move_setup(vec![1, 1]);
        // Base solve of the request, the copy's first try fails, the
        // second try succeeds.
        let runs = Rc::new(RefCell::new(0u32));
        let counter = runs.clone();
        e.register_double(BASE_SOLVER_RUN, move |_, _| {
            *counter.borrow_mut() += 1;
            ret((*counter.borrow() != 2) as u32)
        });
        e.call_log = Some(vec![]);
        assert!(e.call(0x006d_2c80, &args![request, solution]).bool());
        let log = log_of(&mut e);
        assert_eq!(*runs.borrow(), 3);
        assert_eq!(calls_to(&log, SOLUTION_COPY_TO).len(), 1);
        assert_eq!(calls_to(&log, SOLUTION_DESTRUCT).len(), 2);
        assert_eq!(calls_to(&log, SOLUTION_NODE_AT).len(), 2);
    }

    #[test]
    fn covered_move_fails_when_the_plain_solve_fails() {
        let (mut e, request, solution) = covered_move_setup(vec![1]);
        e.register(BASE_SOLVER_RUN, |_, _| ret(0));
        e.call_log = Some(vec![]);
        assert!(!e.call(0x006d_2c80, &args![request, solution]).bool());
        let log = log_of(&mut e);
        assert!(calls_to(&log, MEMORY_ALLOCATE).is_empty());
        assert_timers_balanced(&log);
    }

    // -- covered-move request setters --------------------------------------

    #[test]
    fn covered_move_setters_store_and_clear_the_cover_pointers() {
        let mut e = engine();
        let request = Ptr::<()>::new(e.mem.alloc(0x120));
        let point_source = e.mem.alloc(12);
        set_point(&mut e, point_source, [1.5, 2.5, 3.5]);
        e.call(0x006d_30b0, &args![request, Ptr::<()>::new(point_source)]);
        assert_eq!(point(&e, request.addr() + 0xc8), [1.5, 2.5, 3.5]);
        e.call(0x006d_30e0, &args![request, 0xa1u32]);
        e.call(0x006d_3100, &args![request, 0xa2u32]);
        assert_eq!(e.mem.u32(request.addr() + 0x108), 0xa1);
        assert_eq!(e.mem.u32(request.addr() + 0x104), 0xa2);
        e.call(0x006d_3120, &args![request]);
        assert_eq!(e.mem.u32(request.addr() + 0x108), 0);
        assert_eq!(e.mem.u32(request.addr() + 0x104), 0);
    }

    // -- cover objects -----------------------------------------------------

    /// Fills `size` bytes at `address` with 0xaa.
    fn dirty(e: &mut Engine, address: u32, size: u32) {
        for i in 0..size {
            e.mem.set_u8(address + i, 0xaa);
        }
    }

    #[test]
    fn cover_location_constructor_initializes_every_field() {
        let mut e = engine();
        set_point(&mut e, ZERO_VECTOR, [0.0, 0.0, 0.0]);
        let object = e.mem.alloc(0x78);
        dirty(&mut e, object, 0x78);
        set_point(&mut e, ZERO_VECTOR, [1.0, 2.0, 3.0]);
        e.call_log = Some(vec![]);
        let back = e.call(0x006d_3150, &args![Ptr::<()>::new(object)]).u32();
        assert_eq!(back, object);
        let log = log_of(&mut e);
        assert_eq!(e.mem.u8(object), 0);
        assert_eq!(e.mem.u8(object + 1), 0);
        assert_eq!(e.mem.u8(object + 2), 0);
        assert_eq!(e.mem.u32(object + 4), 0);
        assert_eq!(point(&e, object + 8), [1.0, 2.0, 3.0]);
        assert_eq!(e.mem.u32(object + 0x68), 0);
        assert_eq!(e.mem.u32(object + 0x6c), u32::MAX);
        for i in 0..6 {
            assert_eq!(e.mem.u8(object + 0x70 + i), 0);
        }
        // The embedded `PathingCoverLocation` is constructed at +0x14.
        assert_eq!(
            calls_to(&log, LOCATION_CONSTRUCT),
            vec![vec![object + 0x14]]
        );
        assert_eq!(e.mem.u32(object + 0x14), PATHING_COVER_LOCATION_VTABLE);
    }

    #[test]
    fn pathing_cover_location_constructor_builds_the_base_and_three_points() {
        let mut e = engine();
        let object = e.mem.alloc(0x54);
        e.call_log = Some(vec![]);
        let back = e.call(0x006d_3200, &args![Ptr::<()>::new(object)]).u32();
        assert_eq!(back, object);
        let log = log_of(&mut e);
        assert_eq!(calls_to(&log, LOCATION_CONSTRUCT), vec![vec![object]]);
        assert_eq!(e.mem.u32(object), PATHING_COVER_LOCATION_VTABLE);
        assert_eq!(
            calls_to(&log, POINT3_DEFAULT_CONSTRUCTOR),
            vec![
                vec![object + 0x28],
                vec![object + 0x34],
                vec![object + 0x40]
            ]
        );
    }

    #[test]
    fn reservation_info_constructor_clears_its_six_bytes() {
        let mut e = engine();
        let object = e.mem.alloc(8);
        dirty(&mut e, object, 8);
        let back = e.call(0x006d_3280, &args![Ptr::<()>::new(object)]).u32();
        assert_eq!(back, object);
        for i in 0..6 {
            assert_eq!(e.mem.u8(object + i), 0);
        }
        assert_eq!(e.mem.u8(object + 6), 0xaa);
    }

    #[test]
    fn pathing_cover_location_destructor_destroys_the_location_base() {
        let mut e = engine();
        e.call_log = Some(vec![]);
        e.call(0x006d_32c0, &args![Ptr::<()>::new(0x1234)]);
        let log = log_of(&mut e);
        assert_eq!(calls_to(&log, LOCATION_DESTRUCT), vec![vec![0x1234]]);
    }

    #[test]
    fn cover_search_info_constructor_initializes_every_field() {
        let mut e = engine();
        let object = e.mem.alloc(0x3c);
        dirty(&mut e, object, 0x3c);
        set_point(&mut e, ZERO_VECTOR, [4.0, 5.0, 6.0]);
        e.set_global(FLOAT_MAXIMUM, f32::MAX);
        e.call_log = Some(vec![]);
        let back = e.call(0x006d_32e0, &args![Ptr::<()>::new(object)]).u32();
        assert_eq!(back, object);
        let log = log_of(&mut e);
        assert_eq!(point(&e, object), [4.0, 5.0, 6.0]);
        assert_eq!(point(&e, object + 0xc), [4.0, 5.0, 6.0]);
        assert_eq!(e.mem.f32(object + 0x18), 0.0);
        assert_eq!(
            calls_to(&log, TIME_STAMP_CONSTRUCT),
            vec![vec![object + 0x1c, (-f32::MAX).to_bits()]]
        );
        assert_eq!(
            calls_to(&log, COVER_LOCATION_ARRAY_CONSTRUCT),
            vec![vec![object + 0x20]]
        );
        assert_eq!(e.mem.u32(object + 0x30), 0);
        assert_eq!(e.mem.u32(object + 0x34), 0);
        assert_eq!(e.mem.u8(object + 0x38), 0);
    }

    #[test]
    fn cover_location_deleting_destructor_frees_on_request() {
        let mut e = engine();
        e.call_log = Some(vec![]);
        let back = e
            .call(0x006d_3370, &args![Ptr::<()>::new(0x5000), 1u32])
            .u32();
        assert_eq!(back, 0x5000);
        let log = log_of(&mut e);
        assert_eq!(calls_to(&log, LOCATION_DESTRUCT), vec![vec![0x5014]]);
        assert_eq!(calls_to(&log, MEMORY_FREE), vec![vec![0x5000]]);
        // Bit 0 clear: destroyed but not freed.
        e.call_log = Some(vec![]);
        e.call(0x006d_3370, &args![Ptr::<()>::new(0x5000), 2u32]);
        let log = log_of(&mut e);
        assert_eq!(calls_to(&log, LOCATION_DESTRUCT).len(), 1);
        assert!(calls_to(&log, MEMORY_FREE).is_empty());
    }

    #[test]
    fn cover_location_destructor_destroys_the_embedded_cover_location() {
        let mut e = engine();
        e.call_log = Some(vec![]);
        e.call(0x006d_33a0, &args![Ptr::<()>::new(0x6000)]);
        let log = log_of(&mut e);
        assert_eq!(calls_to(&log, LOCATION_DESTRUCT), vec![vec![0x6014]]);
    }

    // -- 006d34d0 ----------------------------------------------------------

    /// A request on a resolvable navmesh whose random-point search finds a
    /// path ending on triangle 7; the triangle's corners are (0, 0, 0),
    /// (4, 0, 0) and (0, 4, 0) and the random point in it is (1, 1, 0).
    fn random_point_setup() -> (Engine, Ptr<PathingRequest>, Points) {
        let mut e = engine();
        let (request, _) = objects(&mut e);
        geometry(&mut e, request);
        e.register(RANDOM_POINT_SEARCH_BUILD_NODE_PATH, |_, _| ret(1));
        e.register(NAVMESH_GET_TRIANGLE, |_, a| {
            assert_eq!(a[..2], [LAST_NAVMESH, 7]);
            ret(0x00d5_0000)
        });
        e.register_double(TRIANGLE_GET_VERTEX_INDEX, |_, a| {
            assert_eq!(a[0], 0x00d5_0000);
            ret(0x20 + a[1])
        });
        let vertices = e.mem.alloc(36);
        set_point(&mut e, vertices, [0.0, 0.0, 0.0]);
        set_point(&mut e, vertices + 12, [4.0, 0.0, 0.0]);
        set_point(&mut e, vertices + 24, [0.0, 4.0, 0.0]);
        e.register_double(NAVMESH_GET_VERTEX, move |_, a| {
            assert_eq!(a[0], LAST_NAVMESH);
            ret(vertices + 12 * (a[1] - 0x20))
        });
        let corners = Rc::new(RefCell::new(Vec::new()));
        let sink = corners.clone();
        let random = e.mem.alloc(12);
        set_point(&mut e, random, [1.0, 1.0, 0.0]);
        e.register_double(TRIANGLE_RANDOM_POINT, move |e, a| {
            for corner in &a[1..4] {
                sink.borrow_mut().push(point(e, *corner));
            }
            ret(random)
        });
        (e, request, corners)
    }

    #[test]
    fn random_point_is_taken_on_the_last_triangle_of_the_path() {
        let (mut e, request, corners) = random_point_setup();
        let out = e.mem.alloc(12);
        e.call_log = Some(vec![]);
        assert!(e
            .call(0x006d_34d0, &args![request, Ptr::<()>::new(out)])
            .bool());
        let log = log_of(&mut e);
        assert_eq!(
            *corners.borrow(),
            vec![[0.0, 0.0, 0.0], [4.0, 0.0, 0.0], [0.0, 4.0, 0.0]]
        );
        assert_eq!(point(&e, out), [1.0, 1.0, 0.0]);
        // The three corners were default-constructed first.
        let iterator = &calls_to(&log, VECTOR_CONSTRUCTOR_ITERATOR)[0];
        assert_eq!(iterator[1..], [12, 3, POINT3_DEFAULT_CONSTRUCTOR]);
        assert_eq!(calls_to(&log, RANDOM_POINT_SEARCH_DESTRUCT).len(), 1);
        assert_eq!(calls_to(&log, NAV_HOLDER_RELEASE).len(), 1);
    }

    #[test]
    fn random_point_fails_without_a_path_or_a_navmesh() {
        let (mut e, request, _) = random_point_setup();
        e.register(RANDOM_POINT_SEARCH_BUILD_NODE_PATH, |_, _| ret(0));
        let out = e.mem.alloc(12);
        e.call_log = Some(vec![]);
        assert!(!e
            .call(0x006d_34d0, &args![request, Ptr::<()>::new(out)])
            .bool());
        let log = log_of(&mut e);
        assert_eq!(calls_to(&log, RANDOM_POINT_SEARCH_DESTRUCT).len(), 1);
        assert_eq!(calls_to(&log, NAV_HOLDER_RELEASE).len(), 1);
        assert!(calls_to(&log, TRIANGLE_RANDOM_POINT).is_empty());

        let (mut e, request, _) = random_point_setup();
        e.register(LOCATION_GET_NAVMESH_AND_TRIANGLE, |_, _| ret(0));
        assert!(!e
            .call(0x006d_34d0, &args![request, Ptr::<()>::new(out)])
            .bool());

        let (mut e, request, _) = random_point_setup();
        e.register(LOCATION_RESOLVE_CLOSEST, |_, _| ret(0));
        e.call_log = Some(vec![]);
        assert!(!e
            .call(0x006d_34d0, &args![request, Ptr::<()>::new(out)])
            .bool());
        assert!(calls_to(&log_of(&mut e), NAV_HOLDER_CONSTRUCT).is_empty());
    }

    // -- 006d33c0 ----------------------------------------------------------

    #[test]
    fn random_destination_prefers_a_point_on_the_navmesh() {
        let (mut e, request, _) = random_point_setup();
        let out = e.mem.alloc(12);
        e.call_log = Some(vec![]);
        e.call(0x006d_33c0, &args![request, Ptr::<()>::new(out)]);
        let log = log_of(&mut e);
        assert_eq!(point(&e, out), [1.0, 1.0, 0.0]);
        assert!(calls_to(&log, RANDOM_ANGLE).is_empty());
    }

    #[test]
    fn random_destination_falls_back_to_a_point_on_a_circle() {
        let mut e = engine();
        let (request, _) = objects(&mut e);
        geometry(&mut e, request);
        vector_math(&mut e);
        // The navmesh search is not possible.
        e.register(LOCATION_RESOLVE_CLOSEST, |_, _| ret(0));
        e.mem.set_f32(request.addr() + 0xb0, 3.0);
        e.mem.set_f32(request.addr() + 0xb4, 9.0);
        e.register(RANDOM_ANGLE, |_, _| ret_float(std::f32::consts::FRAC_PI_2));
        // The radius is picked between the two distances.
        e.register_double(RANDOM_FLOAT, |_, a| {
            assert_eq!(a[..2], [3.0f32.to_bits(), 9.0f32.to_bits()]);
            ret_float(6.0)
        });
        e.register(ANGLE_FUNCTION_004E44D0, |_, a| {
            ret_float(float_arg(a[0]).cos())
        });
        e.register(ANGLE_FUNCTION_004E4490, |_, a| {
            ret_float(float_arg(a[0]).sin())
        });
        e.register(POINT3_CONSTRUCT, |e, a| {
            for i in 1..4 {
                e.mem.set_u32(a[0] + 4 * (i - 1), a[i as usize]);
            }
            ret(a[0])
        });
        let out = e.mem.alloc(12);
        // The origin has land at height 50.
        e.register(LOCATION_GET_CELL, |_, _| ret(0x00e0_0000));
        e.register(CELL_GET_LAND_HEIGHT, |e, a| {
            e.mem.set_f32(a[2], 50.0);
            ret(1)
        });
        e.call(0x006d_33c0, &args![request, Ptr::<()>::new(out)]);
        // (x, y) = (sin, cos) of the angle times the radius, from the
        // origin (1, 2); the height is the land height.
        assert_close(point(&e, out), [7.0, 2.0, 50.0]);

        // A cell without land keeps the origin's height.
        e.register(CELL_GET_LAND_HEIGHT, |_, _| ret(0));
        e.call(0x006d_33c0, &args![request, Ptr::<()>::new(out)]);
        assert_close(point(&e, out), [7.0, 2.0, 3.0]);

        // No cell: nothing to look up.
        e.register(LOCATION_GET_CELL, |_, _| ret(0));
        e.call_log = Some(vec![]);
        e.call(0x006d_33c0, &args![request, Ptr::<()>::new(out)]);
        assert!(calls_to(&log_of(&mut e), CELL_GET_LAND_HEIGHT).is_empty());
    }

    // -- 006d3780 ----------------------------------------------------------

    /// A reference whose vtable writes the corners (slot +0x1dc: the
    /// maximum, +0x1d8: the minimum) into its out parameter and returns it.
    fn reference_with_bounds(
        e: &mut Engine,
        vtable: u32,
        maximum: [f32; 3],
        minimum: [f32; 3],
    ) -> u32 {
        let reference = e.mem.alloc(0x10);
        let mut slots = vec![0u32; 0x1dc / 4 + 1];
        slots[0x1dc / 4] = vtable + 0x1000;
        slots[0x1d8 / 4] = vtable + 0x2000;
        e.put_vtable(vtable, &slots);
        e.register_double(vtable + 0x1000, move |e, a| {
            set_point(e, a[1], maximum);
            ret(a[1])
        });
        e.register_double(vtable + 0x2000, move |e, a| {
            set_point(e, a[1], minimum);
            ret(a[1])
        });
        e.mem.set_u32(reference, vtable);
        reference
    }

    #[test]
    fn safe_straight_line_sizes_the_request_from_both_references() {
        let mut e = engine();
        vector_math(&mut e);
        e.set_global(MINIMUM_RADIUS, 32.0f32);
        e.set_global(HALF, 0.5f64);
        e.set_global(NINE_TENTHS, 0.9f64);
        // The first reference is 40 wide at scale 1, the second 40 at
        // scale 2.
        let from = reference_with_bounds(&mut e, 0x00a2_0000, [50.0, 40.0, 0.0], [10.0, 0.0, 0.0]);
        let to =
            reference_with_bounds(&mut e, 0x00a3_0000, [100.0, 100.0, 0.0], [60.0, 100.0, 0.0]);
        e.register_double(REFERENCE_GET_SCALE, move |_, a| {
            ret_float(if a[0] == to { 2.0 } else { 1.0 })
        });
        // The locations built for the references are their own addresses.
        e.register(LOCATION_CONSTRUCT_AT_REFERENCE, |_, a| ret(a[0]));
        let request_address = Rc::new(RefCell::new(0u32));
        let remember = request_address.clone();
        e.register_double(SAFE_STRAIGHT_LINE_CONSTRUCT, move |_, a| {
            *remember.borrow_mut() = a[0];
            ret(a[0])
        });
        let seen = Rc::new(RefCell::new(Vec::new()));
        let sink = seen.clone();
        e.register_double(SAFE_STRAIGHT_LINE_FIND, move |e, a| {
            // The origin radius is already stored in the request.
            sink.borrow_mut().push(e.mem.f32(a[0] + 0x68));
            ret(1)
        });
        let out = e.mem.alloc(12);
        e.call_log = Some(vec![]);
        let found = e
            .call(
                0x006d_3780,
                &args![
                    Ptr::<()>::new(from),
                    Ptr::<()>::new(to),
                    100.0f32,
                    Ptr::<()>::new(out)
                ],
            )
            .bool();
        assert!(found);
        let log = log_of(&mut e);
        let request = *request_address.borrow();
        assert_eq!(*seen.borrow(), vec![20.0]);
        // Origin and destination are assigned from the locations built at the
        // two references.
        let built = calls_to(&log, LOCATION_CONSTRUCT_AT_REFERENCE);
        assert_eq!(built[0][1], from);
        assert_eq!(built[1][1], to);
        assert_eq!(
            calls_to(&log, LOCATION_ASSIGN),
            vec![
                vec![request + 0xc, built[0][0]],
                vec![request + 0x34, built[1][0]]
            ]
        );
        // fTargetRadius 40, minimum 100 * 0.9 + 20, maximum 100 + 20.
        assert_eq!(
            calls_to(&log, REQUEST_SET_TARGET_RADIUS),
            vec![vec![request, 40.0f32.to_bits()]]
        );
        assert_eq!(
            calls_to(&log, REQUEST_SET_MINIMUM_DISTANCE),
            vec![vec![request, 110.0f32.to_bits()]]
        );
        assert_eq!(
            calls_to(&log, REQUEST_SET_MAXIMUM_DISTANCE),
            vec![vec![request, 120.0f32.to_bits()]]
        );
        assert_eq!(
            calls_to(&log, SAFE_STRAIGHT_LINE_FIND),
            vec![vec![request, out, 0]]
        );
        assert_eq!(
            calls_to(&log, SAFE_STRAIGHT_LINE_DESTRUCT),
            vec![vec![request]]
        );
    }

    #[test]
    fn safe_straight_line_enforces_the_least_radius_and_passes_failure_on() {
        let mut e = engine();
        vector_math(&mut e);
        e.set_global(MINIMUM_RADIUS, 32.0f32);
        e.set_global(HALF, 0.5f64);
        e.set_global(NINE_TENTHS, 0.9f64);
        // Tiny references: the size is raised to 32, the radius is 16.
        let from = reference_with_bounds(&mut e, 0x00a2_0000, [1.0, 1.0, 0.0], [0.0, 0.0, 0.0]);
        let to = reference_with_bounds(&mut e, 0x00a3_0000, [1.0, 1.0, 0.0], [0.0, 0.0, 0.0]);
        e.register(REFERENCE_GET_SCALE, |_, _| ret_float(1.0));
        e.register(LOCATION_CONSTRUCT_AT_REFERENCE, |_, a| ret(a[0]));
        e.call_log = Some(vec![]);
        let found = e
            .call(
                0x006d_3780,
                &args![
                    Ptr::<()>::new(from),
                    Ptr::<()>::new(to),
                    10.0f32,
                    Ptr::<()>::new(0x1000)
                ],
            )
            .bool();
        // The find double returns zero.
        assert!(!found);
        let log = log_of(&mut e);
        assert_eq!(
            calls_to(&log, REQUEST_SET_TARGET_RADIUS)[0][1],
            16.0f32.to_bits()
        );
        assert_eq!(
            calls_to(&log, REQUEST_SET_MINIMUM_DISTANCE)[0][1],
            25.0f32.to_bits()
        );
    }

    // -- 006d3ae0 ----------------------------------------------------------

    #[test]
    fn actor_radius_setter_stores_the_float() {
        let mut e = engine();
        let (request, _) = objects(&mut e);
        e.call(0x006d_3ae0, &args![request, 12.5f32]);
        assert_eq!(e.get(request, PathingRequest::fActorRadius), 12.5);
    }

    #[test]
    fn funcs_cover_the_forty_functions() {
        assert_eq!(funcs().len(), 40);
    }
}
