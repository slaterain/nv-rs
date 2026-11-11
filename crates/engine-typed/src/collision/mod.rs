//! The collision family (the update half's third group, stage 1): the Havok collision objects' part
//! of the scene graph's world-data update. A node with a collision object hands its world-data update
//! to it (`NiAVObject::UpdateWorldData`, the culling family, calls it directly when the object's slot
//! holds the blend collision object's); a blend collision object (the actors' ragdoll bones) then moves
//! its node to its Havok body or the body to the node, blending the two when it is part keyframed; a
//! collision object moves its Havok body to its node (`NitohkTransform`) with the quaternion of the
//! node's rotation and the body's velocity it reads. About 350 objects a frame in the crowded
//! Goodsprings street (the profile of 2026-11-10), most from the queued node updates main runs after
//! the frame is shown.
//!
//! - [`blend`]: `bhkBlendCollisionObject::UpdateWorldData` and `hktoNiTransform` with its blend;
//! - [`body`]: `bhkCollisionObject::NitohkTransform`, `NiQuaternion::FromRotation` and the body's
//!   linear velocity.
//!
//! What stays by address: `bhkCollisionObject::hktoNiTransform` (`00c65a80`, replaced by the fixes
//! preset's smooth physics drawing, overridable.tsv), the Havok calls (motion type, hard keyframe,
//! velocities' set-up, the blend's quaternion and transform helpers), the Havok world.
//!
//! Stage 1: the objects stay in guest memory. **Floats.** These run inside the scene graph's update, so
//! wherever the scene family's pass runs (the main thread, and the AI threads for the actors' animated
//! scenes): [`Threads::Both`]. The x87 parts keep the exact helpers; the SSE parts (Havok's vectors)
//! compute in `f32` lane by lane in the exe's operand order ([`engine_core::sse`]).

use engine_core::ptr::Ptr;
use engine_core::typed::{Family, FamilyId, Threads};
use engine_core::{entry, AbiFn};

pub mod blend;
pub mod body;

/// This family's bit: what each `typed_call!` of it names.
pub const FAMILY: FamilyId = FamilyId::new(21);

/// `NV_TYPED=collision`. Proved by its differential tests (each planted mutant caught) and the
/// byte-identical recorded street run in both presets (on against `NV_TYPED=collision:off`), so on
/// unless `NV_TYPED` says otherwise.
pub static COLLISION: Family = Family {
    name: "collision",
    id: FAMILY,
    past_gate: true,
    threads: Threads::Both,
    stage2: false,
    funcs,
};

/// The family's functions, by exe address.
#[rustfmt::skip]
fn funcs() -> Vec<(u32, AbiFn)> {
    vec![
        entry!(0x00c8_17d0, blend::update_world_data(Ptr, u32)),
        entry!(0x00c8_1e30, blend::hk_to_ni(Ptr)),
        entry!(0x00c6_5b00, body::ni_to_hk(Ptr)),
        entry!(0x00a6_df40, body::quaternion_from_rotation(Ptr, Ptr)),
        entry!(0x0056_0d80, body::linear_velocity(Ptr) -> Ptr),
        entry!(0x0056_0dc0, body::linear_velocity_of(Ptr) -> Ptr),
    ]
}
