//! The downward update pass of the classes beside `NiNode`: a plain object (a geometry, a light),
//! the fade node's slot, the particle system, and the update's entry; and the slot dispatch the
//! typed passes use for a child, so a child of a common class is a Rust call.

use super::FAMILY;
use crate::culling::layout::{NiAVObject, NiGeometry, NiNode};
use crate::{culling, dispatch, slot_target};
use engine_core::ptr::Ptr;
use engine_core::{args, typed_call, Engine};

/// The update data's flags: run the controllers (+4), skip the particle systems (+5), and the
/// byte a skipped particle system sets (+6).
const DATA_CONTROLLERS: u32 = 4;
const DATA_SKIP_PARTICLES: u32 = 5;
const DATA_PARTICLES_SKIPPED: u32 = 6;
/// Vtable slots: `UpdateDownwardPass` (+0xa4), `UpdateWorldData` (+0xb8), `UpdateWorldBound`
/// (+0xbc); and the slot the update's entry calls on the object at +0x18 (+0xfc).
const SLOT_UPDATE_DOWNWARD: u32 = 0xa4;
const SLOT_UPDATE_WORLD_DATA: u32 = 0xb8;
const SLOT_UPDATE_WORLD_BOUND: u32 = 0xbc;
const SLOT_AFTER_UPDATE: u32 = 0xfc;

/// `UpdateWorldData` (vtable +0xb8) of `object` with the update data: `NiAVObject`'s (`00a68c60`)
/// directly when the slot holds it.
fn world_data(e: &mut Engine, object: Ptr<NiAVObject>, data: u32) {
    if slot_target(e, object.addr(), SLOT_UPDATE_WORLD_DATA) == 0x00a6_8c60 {
        typed_call!(
            culling::FAMILY,
            0x00a6_8c60,
            culling::update::update_world_data(e, object, data)
        );
    } else {
        e.vcall(object.addr(), SLOT_UPDATE_WORLD_DATA, &args![data]);
    }
}

/// `UpdateWorldBound` (vtable +0xbc) of `object`: `NiGeometry`'s (`00a80710`) directly when the
/// slot holds it.
fn world_bound(e: &mut Engine, object: Ptr<NiAVObject>) {
    if slot_target(e, object.addr(), SLOT_UPDATE_WORLD_BOUND) == 0x00a8_0710 {
        typed_call!(
            culling::FAMILY,
            0x00a8_0710,
            culling::update::geometry_update_world_bound(e, object.cast::<NiGeometry>())
        );
    } else {
        e.vcall(object.addr(), SLOT_UPDATE_WORLD_BOUND, &args![]);
    }
}

/// `UpdateDownwardPass` (vtable +0xa4) of `object` with the update data and the flags: the typed
/// pass of the classes whose slot holds one (`NiNode` 00a5dd70, the fade node 00b4ded0, a plain
/// object 00a59f90, the particle system 00c1ad90) as a Rust call, any other through the vtable.
pub fn downward(e: &mut Engine, object: u32, data: u32, flags: u32) {
    match slot_target(e, object, SLOT_UPDATE_DOWNWARD) {
        0x00a5_dd70 => typed_call!(
            dispatch::FAMILY,
            0x00a5_dd70,
            dispatch::scene::update_downward(e, Ptr::new(object), data, flags)
        ),
        0x00a5_9f90 => typed_call!(
            FAMILY,
            0x00a5_9f90,
            object_update_downward(e, Ptr::new(object), Ptr::new(data), flags)
        ),
        0x00b4_ded0 => typed_call!(
            FAMILY,
            0x00b4_ded0,
            fade_update_downward(e, Ptr::new(object), data, flags)
        ),
        0x00c1_ad90 => typed_call!(
            FAMILY,
            0x00c1_ad90,
            particles_update_downward(e, Ptr::new(object), Ptr::new(data), flags)
        ),
        _ => {
            e.vcall(object, SLOT_UPDATE_DOWNWARD, &args![data, flags]);
        }
    }
}

// Typed from 00a59f90
/// `NiAVObject::UpdateDownwardPass` (`thiscall(this, data, flags)`): the object's controllers when
/// the data's byte +4 is set (`004eff50`, properties too), then its `UpdateWorldData` (+0xb8) with
/// the data and its `UpdateWorldBound` (+0xbc). The flags word is not read.
pub fn object_update_downward(e: &mut Engine, this: Ptr<NiAVObject>, data: Ptr, _flags: u32) {
    if e.mem.u8(data.addr() + DATA_CONTROLLERS) != 0 {
        typed_call!(
            dispatch::FAMILY,
            0x004e_ff50,
            dispatch::scene::update_object_controllers(e, this, data, 1u8)
        );
    }
    world_data(e, this, data.addr());
    world_bound(e, this);
}

// Typed from 00b4ded0
/// The fade node's `UpdateDownwardPass` slot (`thiscall(this, data, flags)`): `NiNode`'s pass
/// (`00a5dd70`) with the same two words.
pub fn fade_update_downward(e: &mut Engine, this: Ptr<NiNode>, data: u32, flags: u32) {
    typed_call!(
        dispatch::FAMILY,
        0x00a5_dd70,
        dispatch::scene::update_downward(e, this, data, flags)
    );
}

// Typed from 00c1ad90
/// `NiParticleSystem::UpdateDownwardPass` (`thiscall(this, data, flags)`): with the data's byte +5
/// set only its byte +6 is set; otherwise the system's `UpdateWorldData` (+0xb8) with the data, then
/// `NiAVObject`'s pass (`00a59f90`) with both words.
pub fn particles_update_downward(e: &mut Engine, this: Ptr<NiAVObject>, data: Ptr, flags: u32) {
    let d = data.addr();
    if e.mem.u8(d + DATA_SKIP_PARTICLES) != 0 {
        e.mem.set_u8(d + DATA_PARTICLES_SKIPPED, 1);
        return;
    }
    e.vcall(this.addr(), SLOT_UPDATE_WORLD_DATA, &args![d]);
    typed_call!(
        FAMILY,
        0x00a5_9f90,
        object_update_downward(e, this, data, flags)
    );
}

// Typed from 00a59c60
/// The update's entry (`thiscall(this, data)`): `UpdateDownwardPass` (+0xa4) of `this` with the data
/// and 0, then, with an object at +0x18, that object's slot +0xfc.
pub fn update(e: &mut Engine, this: Ptr, data: u32) {
    downward(e, this.addr(), data, 0);
    let after = e.mem.u32(this.addr() + 0x18);
    if after != 0 {
        e.vcall(after, SLOT_AFTER_UPDATE, &args![]);
    }
}
