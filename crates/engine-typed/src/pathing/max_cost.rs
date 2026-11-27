//! The max-cost navmesh search (`NavMeshSearchMaxCost`): its constructor body, the destructors of
//! its node arrays and of itself, and its goal test and best-node getter.

use super::{search, FAMILY};
use engine_core::ptr::Ptr;
use engine_core::{args, typed_call, Engine};

/// The vtable `NavMeshSearchMaxCost` stores at `+0` (RTTI `.?AVNavMeshSearchMaxCost@@`).
const MAX_COST_VTABLE: u32 = 0x0106_b87c;
/// The vtable of `BSSimpleArray<NavMeshSearchNode,1024>` (RTTI
/// `.?AV?$BSSimpleArray@UNavMeshSearchNode@@$0EAA@@@`).
const SIMPLE_ARRAY_VTABLE: u32 = 0x0106_b8a0;
/// The vtable of `BSScrapArray<NavMeshSearchNode,1024>` (RTTI
/// `.?AV?$BSScrapArray@UNavMeshSearchNode@@$0EAA@@@`).
const SCRAP_ARRAY_VTABLE: u32 = 0x0106_b88c;

/// Callees.
const BASE_CTOR: u32 = 0x006a_6b00;
const ARRAY_RELEASE: u32 = 0x0084_54f0;
const OPERATOR_DELETE: u32 = 0x0040_1030;

/// `bHasGoalNode` of `NavMeshSearchMaxCost` (+0x20A8, past the `AStarSearch` base).
const HAS_GOAL_NODE: u32 = 0x20a8;
/// `BestNodeSoFar` (+0x2048).
const BEST_NODE_SO_FAR: u32 = 0x2048;

// Typed from 006ab480
/// `NavMeshSearchMaxCost`'s constructor body (by the vtable it stores): the class's vtable at `+0`
/// of `this`, then the `NavMeshSearch` base constructor (`006a6b00`).
pub fn ctor_body(e: &mut Engine, this: Ptr) {
    e.mem.set_u32(this.addr(), MAX_COST_VTABLE);
    e.call(BASE_CTOR, &args![this]);
}

// Typed from 006abc30
/// `BSSimpleArray<NavMeshSearchNode,1024>`'s destructor body (by the vtable it stores): the class's
/// vtable at `+0`, then the array's release (`008454f0(this, 1)`).
pub fn simple_array_dtor(e: &mut Engine, this: Ptr) {
    e.mem.set_u32(this.addr(), SIMPLE_ARRAY_VTABLE);
    e.call(ARRAY_RELEASE, &args![this, 1u32]);
}

// Typed from 006abc50
/// `BSScrapArray<NavMeshSearchNode,1024>`'s destructor body (by the vtable it stores): the scrap
/// array's vtable at `+0`, the array's release (`008454f0(this, 1)`), then the base `BSSimpleArray`
/// destructor body (`006abc30`).
pub fn scrap_array_dtor(e: &mut Engine, this: Ptr) {
    e.mem.set_u32(this.addr(), SCRAP_ARRAY_VTABLE);
    e.call(ARRAY_RELEASE, &args![this, 1u32]);
    typed_call!(FAMILY, 0x006a_bc30, simple_array_dtor(e, this));
}

// Typed from 006ab450
/// `NavMeshSearchMaxCost::scalar deleting destructor` (Xbox PDB): the destructor body (`006ab480`),
/// then `operator delete` when bit 0 of `flags` is set. Answers `this`.
pub fn scalar_deleting_dtor(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    typed_call!(FAMILY, 0x006a_b480, ctor_body(e, this));
    if flags & 1 != 0 {
        e.call(OPERATOR_DELETE, &args![this]);
    }
    this
}

// Typed from 006abb60
/// `NavMeshSearchMaxCost::IsGoalNode` (Xbox PDB): false without a goal node (`bHasGoalNode`
/// +0x20A8), else `NavMeshSearch::IsGoalNode` (`006a79c0`).
pub fn is_goal_node(e: &mut Engine, this: Ptr, node: Ptr) -> bool {
    if e.mem.u8(this.addr() + HAS_GOAL_NODE) == 0 {
        return false;
    }
    let goal: bool = typed_call!(FAMILY, 0x006a_79c0, search::is_goal(e, this.cast(), node));
    goal
}

// Typed from 006abb90
/// `BestNodeSoFar` (+0x2048).
pub fn best_node_so_far(e: &mut Engine, this: Ptr) -> u32 {
    e.mem.u32(this.addr() + BEST_NODE_SO_FAR)
}
