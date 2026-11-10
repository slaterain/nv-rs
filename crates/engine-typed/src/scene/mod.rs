//! The scene family (the update half's first group, stage 1): the downward update pass over the
//! scene graph's node classes the typed `NiNode` pass (`dispatch::scene::update_downward`) does not
//! cover itself, so that the walk stays native across a geometry, a fade node and a particle system,
//! and the skinned geometry's bound with the transform inverse it needs. The pass runs once a frame
//! over every attached scene (about 1,700 objects a frame in the crowded Goodsprings street: 1,267
//! nodes and 402 geometries, by the profile of 2026-11-10).
//!
//! Stage 1: the objects stay in guest memory where the game made them. Each function stays
//! registered at its exe address, so the vtables reach the typed version unchanged; a call into a
//! family is a `typed_call!`, and a virtual call is direct only when the slot holds the typed
//! function the game's classes put there ([`crate::slot_target`]).
//!
//! **Floats.** The pass runs on the main thread and, for the actors' animated scenes, on the AI
//! threads, so the family's row is [`Threads::Both`] and its x87 code keeps the exact helpers
//! ([`engine_core::x87`]), each load, operation and store the exe's, in its order.

use engine_core::ptr::Ptr;
use engine_core::typed::{Family, FamilyId, Threads};
use engine_core::{entry, AbiFn};

use crate::culling::layout::{NiAVObject, NiNode};

pub mod skin;
pub mod walk;

/// This family's bit: what each `typed_call!` of it names.
pub const FAMILY: FamilyId = FamilyId::new(19);

/// `NV_TYPED=scene`. Proved by its differential tests (each planted mutant caught) and the
/// byte-identical recorded street run in both presets (on against `NV_TYPED=scene:off`), so on
/// unless `NV_TYPED` says otherwise.
pub static SCENE: Family = Family {
    name: "scene",
    id: FAMILY,
    past_gate: true,
    threads: Threads::Both,
    funcs,
};

/// The family's functions, by exe address.
#[rustfmt::skip]
fn funcs() -> Vec<(u32, AbiFn)> {
    vec![
        entry!(0x00a5_9f90, walk::object_update_downward(Ptr<NiAVObject>, Ptr, u32)),
        entry!(0x00b4_ded0, walk::fade_update_downward(Ptr<NiNode>, u32, u32)),
        entry!(0x00c1_ad90, walk::particles_update_downward(Ptr<NiAVObject>, Ptr, u32)),
        entry!(0x00a5_9c60, walk::update(Ptr, u32)),
        entry!(0x00a8_6530, skin::skin_bound(Ptr, Ptr)),
        entry!(0x008b_42e0, skin::transform_inverse(Ptr, Ptr)),
    ]
}
