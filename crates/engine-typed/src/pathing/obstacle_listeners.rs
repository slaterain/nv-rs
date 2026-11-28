//! The navmesh obstacle manager's update queues and Havok listeners: pushing rigid bodies on the
//! add/remove update lists, and adding/removing the entity and activation listeners on a rigid
//! body's Havok entity.

use engine_core::ptr::Ptr;
use engine_core::{args, typed_call, Engine};

use super::FAMILY;

/// Callees.
const QUEUE_PUSH: u32 = 0x006c_5ff0;
const RIGID_BODY_ENTITY: u32 = 0x004a_e750;
const ENTITY_ADD_ENTITY_LISTENER: u32 = 0x00c9_c5b0;
const ENTITY_ADD_ACTIVATION_LISTENER: u32 = 0x00c9_c700;
const ENTITY_REMOVE_ENTITY_LISTENER: u32 = 0x00c9_c670;
const ENTITY_REMOVE_ACTIVATION_LISTENER: u32 = 0x00c9_c0f0;

/// The manager's `ObstaclesToAddToUpdateList` offset.
const QUEUE_ADD: u32 = 0x60;
/// The manager's `ObstaclesToRemoveFromUpdateList` offset.
const QUEUE_REMOVE: u32 = 0xa0;
/// The manager's `pListener` offset.
const LISTENER: u32 = 0x120;
/// The manager's `pRemoveListener` offset.
const REMOVE_LISTENER: u32 = 0x124;

/// One queue operation: the value in a stack slot, then `operation(list, slot)`.
fn list_operation(e: &mut Engine, operation: u32, list: u32, value: u32) {
    e.with_stack(4, |e, slot| {
        e.mem.set_u32(slot.addr(), value);
        e.call(operation, &args![list, slot.addr()]);
    })
}

// Typed from 006c1c70
/// Pushes a rigid body on `ObstaclesToAddToUpdateList` (+0x60).
pub fn queue_obstacle_add(e: &mut Engine, this: Ptr, rigid_body: u32) {
    list_operation(e, QUEUE_PUSH, this.addr() + QUEUE_ADD, rigid_body);
}

// Typed from 006c1c90
/// Pushes a rigid body on `ObstaclesToRemoveFromUpdateList` (+0xa0).
pub fn queue_obstacle_remove(e: &mut Engine, this: Ptr, rigid_body: u32) {
    list_operation(e, QUEUE_PUSH, this.addr() + QUEUE_REMOVE, rigid_body);
}

// Typed from 006c1da0
/// Adds an entity listener to a rigid body's Havok entity (`004ae750` of the rigid body): when
/// there is one, `00c9c5b0` on it.
pub fn add_entity_listener(e: &mut Engine, this: Ptr, listener: u32) {
    let entity = e.call(RIGID_BODY_ENTITY, &args![this]).u32();
    if entity != 0 {
        e.call(ENTITY_ADD_ENTITY_LISTENER, &args![entity, listener]);
    }
}

// Typed from 006c1dd0
/// Adds an activation listener to a rigid body's Havok entity (`004ae750` of the rigid body):
/// when there is one, `00c9c700` on it.
pub fn add_activation_listener(e: &mut Engine, this: Ptr, listener: u32) {
    let entity = e.call(RIGID_BODY_ENTITY, &args![this]).u32();
    if entity != 0 {
        e.call(ENTITY_ADD_ACTIVATION_LISTENER, &args![entity, listener]);
    }
}

// Typed from 006c1e40
/// Removes an entity listener from a rigid body's Havok entity (`004ae750` of the rigid body):
/// when there is one, `00c9c670` on it.
pub fn remove_entity_listener(e: &mut Engine, this: Ptr, listener: u32) {
    let entity = e.call(RIGID_BODY_ENTITY, &args![this]).u32();
    if entity != 0 {
        e.call(ENTITY_REMOVE_ENTITY_LISTENER, &args![entity, listener]);
    }
}

// Typed from 006c1e70
/// Removes an activation listener from a rigid body's Havok entity (`004ae750` of the rigid
/// body): when there is one, `00c9c0f0` on it.
pub fn remove_activation_listener(e: &mut Engine, this: Ptr, listener: u32) {
    let entity = e.call(RIGID_BODY_ENTITY, &args![this]).u32();
    if entity != 0 {
        e.call(ENTITY_REMOVE_ACTIVATION_LISTENER, &args![entity, listener]);
    }
}

// Typed from 006c1d60
/// Registers the manager's two listeners on a rigid body's Havok entity: the activation listener
/// (`pListener`, +0x120) through [`add_activation_listener`], then the entity listener
/// (`pRemoveListener`, +0x124) through [`add_entity_listener`].
pub fn register_listeners(e: &mut Engine, this: Ptr, rigid_body: u32) {
    let listener = e.mem.u32(this.addr() + LISTENER);
    typed_call!(
        FAMILY,
        0x006c_1dd0,
        add_activation_listener(e, Ptr::new(rigid_body), listener)
    );
    let remove_listener = e.mem.u32(this.addr() + REMOVE_LISTENER);
    typed_call!(
        FAMILY,
        0x006c_1da0,
        add_entity_listener(e, Ptr::new(rigid_body), remove_listener)
    );
}

// Typed from 006c1e00
/// Removes the manager's two Havok listeners from a rigid body: the activation listener
/// (`pListener`, +0x120) through [`remove_activation_listener`], then the entity listener
/// (`pRemoveListener`, +0x124) through [`remove_entity_listener`].
pub fn unregister_listeners(e: &mut Engine, this: Ptr, rigid_body: u32) {
    let listener = e.mem.u32(this.addr() + LISTENER);
    typed_call!(
        FAMILY,
        0x006c_1e70,
        remove_activation_listener(e, Ptr::new(rigid_body), listener)
    );
    let remove_listener = e.mem.u32(this.addr() + REMOVE_LISTENER);
    typed_call!(
        FAMILY,
        0x006c_1e40,
        remove_entity_listener(e, Ptr::new(rigid_body), remove_listener)
    );
}
