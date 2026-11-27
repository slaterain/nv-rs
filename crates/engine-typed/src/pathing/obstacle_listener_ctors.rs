//! The obstacle listener constructors and destructors: the deactivation and removal listeners of
//! the navmesh obstacle manager.

use engine_core::ptr::Ptr;
use engine_core::{args, typed_call, Engine};

use super::FAMILY;

/// Callees and data.
const ACTIVATION_LISTENER_BASE_CTOR: u32 = 0x006c_0100;
const ENTITY_LISTENER_BASE_CTOR: u32 = 0x0061_f580;
const ENTITY_LISTENER_BASE_DTOR: u32 = 0x0055_4650;
const OPERATOR_DELETE: u32 = 0x0040_1030;
const DEACTIVATION_LISTENER_VTABLE: u32 = 0x0106_c3b4;
const REMOVAL_LISTENER_VTABLE: u32 = 0x0106_c3d4;

// Typed from 006c00b0
/// Constructor of `bhkObstacleDeactivationListener`: the activation-listener base constructor
/// (`006c0100`), then the class's vtable. Returns `this`.
pub fn deactivation_listener_new(e: &mut Engine, this: Ptr) -> Ptr {
    e.call(ACTIVATION_LISTENER_BASE_CTOR, &args![this]);
    e.mem.set_u32(this.addr(), DEACTIVATION_LISTENER_VTABLE);
    this
}

// Typed from 006c0210
/// Constructor of `bhkObstacleRemovalListener`: the entity-listener base constructor
/// (`0061f580`), then the class's vtable. Returns `this`.
pub fn removal_listener_new(e: &mut Engine, this: Ptr) -> Ptr {
    e.call(ENTITY_LISTENER_BASE_CTOR, &args![this]);
    e.mem.set_u32(this.addr(), REMOVAL_LISTENER_VTABLE);
    this
}

// Typed from 006c0260
/// Destructor of `bhkObstacleRemovalListener`: the class's vtable, then the entity-listener base
/// destructor (`00554650`).
pub fn removal_listener_drop(e: &mut Engine, this: Ptr) {
    e.mem.set_u32(this.addr(), REMOVAL_LISTENER_VTABLE);
    e.call(ENTITY_LISTENER_BASE_DTOR, &args![this]);
}

// Typed from 006c0230
/// Scalar deleting destructor of `bhkObstacleRemovalListener`: the destructor
/// ([`removal_listener_drop`]), then `operator delete` when bit 0 of `flags` is set. Returns
/// `this`.
pub fn removal_listener_scalar_delete(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    typed_call!(FAMILY, 0x006c_0260, removal_listener_drop(e, this));
    if flags & 1 != 0 {
        e.call(OPERATOR_DELETE, &args![this]);
    }
    this
}
