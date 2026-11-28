//! The navmesh obstacle task's tasklet data: its vtable install, the destructor bodies and
//! deleting destructors of the tasklet data, the task base and `ObstacleTaskData`, the
//! `Operations` array's constructor and destructors, and the manager's performance-timer byte
//! address.

use super::FAMILY;
use engine_core::ptr::Ptr;
use engine_core::{args, typed_call, Engine};

/// The tasklet data base class's vtable (`0106c814`).
const TASKLET_VTABLE: u32 = 0x0106_c814;
/// `ObstacleTaskData`'s vtable (`0106c7ec`, the one `006c77a0` installs).
const TASK_VTABLE: u32 = 0x0106_c7ec;
/// The `Operations` array's vtable (`0106c8d4`).
const OPERATIONS_VTABLE: u32 = 0x0106_c8d4;

/// Callees.
const TASK_BASE_DESTRUCTOR: u32 = 0x00b0_0ec0;
const PORTAL_SWAPS_DESTRUCTOR: u32 = 0x0069_94b0;
const HOLDER_DESTRUCTOR: u32 = 0x0042_fa40;
const OPERATOR_DELETE: u32 = 0x0040_1030;
const ARRAY_INIT: u32 = 0x006c_8c70;
const ARRAY_DESTROY_ALL: u32 = 0x006c_8a30;

/// `ObstacleTaskData`'s members the destructor destroys: `PortalSwaps` (+0x34), `Operations`
/// (+0x24), `spNewNavMesh` (+0x20), `spSrcMesh` (+0x1C).
const PORTAL_SWAPS: u32 = 0x34;
const OPERATIONS: u32 = 0x24;
const SP_NEW_NAV_MESH: u32 = 0x20;
const SP_SRC_MESH: u32 = 0x1c;
/// The manager's performance-timer byte (+0x1A1).
const TIMER_BYTE: u32 = 0x1a1;

// Typed from 006c85c0
/// The address of a byte of the `NavMeshObstacleManager` (its performance timer): `this +
/// 0x1A1`.
pub fn timer_byte(_e: &mut Engine, this: u32) -> u32 {
    this.wrapping_add(TIMER_BYTE)
}

// Typed from 006c78f0
/// Installs the tasklet data base class's vtable (`0106c814`) at `+0` of `this`.
pub fn tasklet_vtable(e: &mut Engine, this: Ptr) {
    e.mem.set_u32(this.addr(), TASKLET_VTABLE);
}

// Typed from 006c7910
/// `BSTaskletData::_scalar_deleting_destructor_` (Xbox PDB): the destructor body (`006c78f0`),
/// then `operator delete` when bit 0 of `flags` is set. Answers `this`.
pub fn tasklet_data_scalar_deleting_dtor(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    typed_call!(FAMILY, 0x006c_78f0, tasklet_vtable(e, this));
    if flags & 1 != 0 {
        e.call(OPERATOR_DELETE, &args![this]);
    }
    this
}

// Typed from 006c7940
/// The task base class's deleting destructor: the base destructor body (`00b00ec0`), then
/// `operator delete` when bit 0 of `flags` is set. Answers `this`.
pub fn task_base_deleting_dtor(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    e.call(TASK_BASE_DESTRUCTOR, &args![this]);
    if flags & 1 != 0 {
        e.call(OPERATOR_DELETE, &args![this]);
    }
    this
}

// Typed from 006c79a0
/// The `ObstacleTaskData` destructor body (it installs the vtable `0106c7ec` that `006c77a0`
/// installs): destroys `PortalSwaps` (+0x34), `Operations` (+0x24, `006c87f0`), `spNewNavMesh`
/// (+0x20) and `spSrcMesh` (+0x1C), then the task base body (`00b00ec0`).
pub fn obstacle_task_data_dtor(e: &mut Engine, this: Ptr) {
    e.mem.set_u32(this.addr(), TASK_VTABLE);
    e.call(PORTAL_SWAPS_DESTRUCTOR, &args![this.addr() + PORTAL_SWAPS]);
    typed_call!(
        FAMILY,
        0x006c_87f0,
        operations_array_dtor(e, this.addr() + OPERATIONS)
    );
    e.call(HOLDER_DESTRUCTOR, &args![this.addr() + SP_NEW_NAV_MESH]);
    e.call(HOLDER_DESTRUCTOR, &args![this.addr() + SP_SRC_MESH]);
    e.call(TASK_BASE_DESTRUCTOR, &args![this]);
}

// Typed from 006c7970
/// `ObstacleTaskData::_scalar_deleting_destructor_` (Xbox PDB): the destructor body (`006c79a0`),
/// then `operator delete` when bit 0 of `flags` is set. Answers `this`.
pub fn obstacle_task_data_scalar_deleting_dtor(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    typed_call!(FAMILY, 0x006c_79a0, obstacle_task_data_dtor(e, this));
    if flags & 1 != 0 {
        e.call(OPERATOR_DELETE, &args![this]);
    }
    this
}

// Typed from 006c87c0
/// The `Operations` array's constructor: installs the vtable `0106c8d4` and runs `006c8c70(this,
/// 0, 0)` (empty, nothing reserved). Answers `this`.
pub fn operations_array_ctor(e: &mut Engine, this: Ptr) -> Ptr {
    e.mem.set_u32(this.addr(), OPERATIONS_VTABLE);
    e.call(ARRAY_INIT, &args![this, 0u32, 0u32]);
    this
}

// Typed from 006c87f0
/// The `Operations` array's destructor body: installs the vtable `0106c8d4` and runs
/// `006c8a30(this, true)` (destroy the elements and free the buffer).
pub fn operations_array_dtor(e: &mut Engine, this: u32) {
    e.mem.set_u32(this, OPERATIONS_VTABLE);
    e.call(ARRAY_DESTROY_ALL, &args![this, true]);
}

// Typed from 006c89b0
/// `BSSimpleArray<ObstacleTaskNavMeshOperation,1024>::_scalar_deleting_destructor_` (Xbox PDB):
/// the destructor body (`006c87f0`), then `operator delete` when bit 0 of `flags` is set.
/// Answers `this`.
pub fn operations_array_scalar_deleting_dtor(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    typed_call!(FAMILY, 0x006c_87f0, operations_array_dtor(e, this.addr()));
    if flags & 1 != 0 {
        e.call(OPERATOR_DELETE, &args![this]);
    }
    this
}
