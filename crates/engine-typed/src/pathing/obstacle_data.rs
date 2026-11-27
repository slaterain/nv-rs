//! The `ObstacleData` record: its constructor, destructor and scalar deleting destructor.

use engine_core::ptr::Ptr;
use engine_core::typed::leaf;
use engine_core::{args, typed_call, Engine};

use super::FAMILY;

/// Callees and data.
const REFERENCE_COUNTED_CTOR: u32 = 0x0049_68b0;
const REFERENCE_COUNTED_DTOR: u32 = 0x0049_6910;
const NI_POINTER_INIT: u32 = 0x0063_3c90;
const NI_POINTER_RELEASE: u32 = 0x0045_cec0;
const OBSTACLE_NAVMESH_ARRAY_CTOR: u32 = 0x0069_aee0;
const OBSTACLE_NAVMESH_ARRAY_DTOR: u32 = 0x0069_af10;
const OPERATOR_DELETE_SIZED: u32 = 0x00aa_1460;
const OBSTACLE_DATA_VTABLE: u32 = 0x0106_c490;
/// The record's size (`operator delete` frees `0x8c` bytes).
const OBSTACLE_DATA_SIZE: u32 = 0x8c;

/// The `ObstacleData` field offsets.
const P_RIGID_BODY: u32 = 0x0c;
const CENTER: u32 = 0x10;
const ORIENTATION: u32 = 0x1c;
const BOX_MIN: u32 = 0x40;
const BOX_MAX: u32 = 0x4c;
const AABB_MIN: u32 = 0x58;
const AABB_MAX: u32 = 0x64;
const B_ACTIVE: u32 = 0x74;
const NAVMESHES: u32 = 0x78;
const SP_3D_NODE: u32 = 0x88;

// Typed from 006c23c0
/// The `ObstacleData` constructor: the reference-counted base (`004968b0`), the vtable, null
/// `pRigidBody`, the six float triplets zeroed (`006815c0`), `bActive` 1, the `Navmeshes` array
/// constructed (`0069aee0`) and null `sp3DNode`. Returns `this`.
pub fn obstacle_data_new(e: &mut Engine, this: Ptr) -> Ptr {
    let base = this.addr();
    e.call(REFERENCE_COUNTED_CTOR, &args![this]);
    e.mem.set_u32(base, OBSTACLE_DATA_VTABLE);
    e.call(NI_POINTER_INIT, &args![base + P_RIGID_BODY, 0u32]);
    for field in [CENTER, ORIENTATION, BOX_MIN, BOX_MAX, AABB_MIN, AABB_MAX] {
        typed_call!(leaf::FAMILY, 0x0068_15c0, leaf::same_word(e, base + field));
    }
    e.mem.set_u8(base + B_ACTIVE, 1);
    e.call(OBSTACLE_NAVMESH_ARRAY_CTOR, &args![base + NAVMESHES]);
    e.call(NI_POINTER_INIT, &args![base + SP_3D_NODE, 0u32]);
    this
}

// Typed from 006c24d0
/// The `ObstacleData` destructor: releases `sp3DNode`, destroys the `Navmeshes` array
/// (`0069af10`), releases `pRigidBody` and runs the reference-counted base destructor
/// (`00496910`). The vtable is left as it is.
pub fn obstacle_data_drop(e: &mut Engine, this: Ptr) {
    let base = this.addr();
    e.call(NI_POINTER_RELEASE, &args![base + SP_3D_NODE]);
    e.call(OBSTACLE_NAVMESH_ARRAY_DTOR, &args![base + NAVMESHES]);
    e.call(NI_POINTER_RELEASE, &args![base + P_RIGID_BODY]);
    e.call(REFERENCE_COUNTED_DTOR, &args![this]);
}

// Typed from 006c2490
/// The `ObstacleData` scalar deleting destructor: the destructor ([`obstacle_data_drop`]), then
/// the sized `operator delete` (`0x8c` bytes) when bit 0 of `flags` is set. Returns `this`.
pub fn obstacle_data_scalar_delete(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    typed_call!(FAMILY, 0x006c_24d0, obstacle_data_drop(e, this));
    if flags & 1 != 0 {
        e.call(OPERATOR_DELETE_SIZED, &args![this, OBSTACLE_DATA_SIZE]);
    }
    this
}
