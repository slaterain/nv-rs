//! The scene family (the update half's first group, stage 1): the downward update pass over the
//! scene graph's node classes the typed `NiNode` pass (`dispatch::scene::update_downward`) does not
//! cover itself, so that the walk stays native across a geometry, a fade node and a particle system,
//! and the skinned geometry's bound with the transform inverse it needs. The pass runs once a frame
//! over every attached scene (about 1,700 objects a frame in the crowded Goodsprings street: 1,267
//! nodes and 402 geometries, by the profile of 2026-11-10). With more than one thread the actors'
//! scene graphs reach it from the queued node updates ([`queue`]), which main runs after the frame
//! is shown.
//!
//! Stage 1: the objects stay in guest memory where the game made them. Each function stays
//! registered at its exe address, so the vtables reach the typed version unchanged; a call into a
//! family is a `typed_call!`, and a virtual call is direct only when the slot holds the typed
//! function the game's classes put there ([`crate::slot_target`]).
//!
//! **Floats.** The pass runs on the main thread and, for the actors' animated scenes, on the AI
//! threads, so the family's row is [`Threads::Both`]. Its x87 code is native maths (ADR-0014's float
//! rule) with no emulation left: the skinned bound's per-axis sums and the transform inverse's
//! reciprocal are each one operation of floats stored at once, so plain `f32` gives the exe's float
//! on either thread's word; the bound's radius, which the exe moves through the x87, is copied with
//! a signalling NaN quieted as the x87 does (one bit).

use engine_core::ptr::Ptr;
use engine_core::typed::{Family, FamilyId, Threads};
use engine_core::{entry, AbiFn};

use crate::culling::layout::{NiAVObject, NiNode};

pub mod queue;
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
    stage2: false,
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
        // The queued node updates main runs after the frame is shown.
        entry!(0x0055_2570, queue::run_queued()),
        entry!(0x0055_26b0, queue::waiting(Ptr) -> bool),
        entry!(0x0055_26d0, queue::set_waiting(Ptr, bool)),
        entry!(0x0055_26f0, queue::update_geometries(Ptr)),
    ]
}
