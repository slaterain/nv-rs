//! The hide, LOS and flee navmesh searches' small members: the hide search's constructor body
//! and deleting destructor, the LOS search's destructor body and deleting destructor, and the
//! flee request's `bAllowIncompletePath` byte getter.

use super::FAMILY;
use engine_core::ptr::Ptr;
use engine_core::{args, typed_call, Engine};

/// The vtable of `NavMeshSearchHide` (`0106b7dc`, RTTI `.?AVNavMeshSearchHide@@`).
const HIDE_VTABLE: u32 = 0x0106_b7dc;
/// The vtable of `NavMeshSearchLOS` (`0106b830`).
const LOS_VTABLE: u32 = 0x0106_b830;

/// Callees.
const BASE_CTOR: u32 = 0x006a_6b00;
const OPERATOR_DELETE: u32 = 0x0040_1030;
const RELEASE_HELD: u32 = 0x0045_cec0;
const ASTAR_DTOR: u32 = 0x006a_7bc0;

/// `spLOSMap` of `NavMeshSearchLOS` (+0x2070).
const LOS_MAP: u32 = 0x2070;
/// A `PathingRequest`'s `bAllowIncompletePath` byte (+0x9B).
const ALLOW_INCOMPLETE_PATH: u32 = 0x9b;

// Typed from 006a9d60
/// `NavMeshSearchHide`'s constructor body (by the vtable it stores): the class's vtable
/// `0106b7dc` at `+0` of `this`, then the `NavMeshSearch` base constructor (`006a6b00`).
pub fn hide_ctor_body(e: &mut Engine, this: Ptr) {
    e.mem.set_u32(this.addr(), HIDE_VTABLE);
    e.call(BASE_CTOR, &args![this]);
}

// Typed from 006a9d30
/// `NavMeshSearchHide::scalar deleting destructor` (Xbox PDB): the destructor body (`006a9d60`),
/// then `operator delete` when bit 0 of `flags` is set. Answers `this`.
pub fn hide_scalar_deleting_dtor(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    typed_call!(FAMILY, 0x006a_9d60, hide_ctor_body(e, this));
    if flags & 1 != 0 {
        e.call(OPERATOR_DELETE, &args![this]);
    }
    this
}

// Typed from 006aa710
/// The `NavMeshSearchLOS` destructor body (by the vtable it stores): its vtable `0106b830`,
/// releases `spLOSMap` (+0x2070, `0045cec0`), then the `AStarSearch` base destructor (`006a7bc0`).
pub fn los_dtor_body(e: &mut Engine, this: Ptr) {
    e.mem.set_u32(this.addr(), LOS_VTABLE);
    e.call(RELEASE_HELD, &args![this.addr() + LOS_MAP]);
    e.call(ASTAR_DTOR, &args![this]);
}

// Typed from 006aa6e0
/// `NavMeshSearchLOS::scalar deleting destructor` (Xbox PDB): the destructor body (`006aa710`),
/// then `operator delete` when bit 0 of `flags` is set. Answers `this`.
pub fn los_scalar_deleting_dtor(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    typed_call!(FAMILY, 0x006a_a710, los_dtor_body(e, this));
    if flags & 1 != 0 {
        e.call(OPERATOR_DELETE, &args![this]);
    }
    this
}

// Typed from 006a9560
/// A `PathingRequest`'s `bAllowIncompletePath` byte (+0x9B).
pub fn allow_incomplete_path(e: &mut Engine, request: Ptr) -> u8 {
    e.mem.u8(request.addr() + ALLOW_INCOMPLETE_PATH)
}
