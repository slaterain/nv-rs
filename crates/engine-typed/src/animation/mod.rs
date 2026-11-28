//! The animation family (the update half's fifth group): a skeleton's bones moved from their
//! animation each frame. `BSBipNode::UpdateAllButBip` walks a bip node's controllers; a
//! `NiMultiTargetTransformController` updates each bone it drives
//! (`BSTransCon::UpdateAllButTarget`) from its interpolator's transform:
//! `NiBlendTransformInterpolator::Update`, which takes one interpolator's value or blends several
//! by their normalized weights (`BlendValues`, `ComputeNormalizedWeights`), each a
//! `NiTransformInterpolator` whose `Update` interpolates the bone's translation, rotation and scale
//! from its keys (the animation keys family, in place). The transform's translation, rotation (from
//! its quaternion, `NiQuaternion::ToRotation`) and scale go into the bone. The player's update
//! reaches it through `Actor::UpdateAnimationMovement` (twice a frame), the actors' through their
//! animation updates (on the AI threads, and on main for the player and when the game runs them
//! there); the bones' world update after the frame is shown is `0096c970`'s.
//!
//! - [`bip`]: the bip node's walk, the transform controller's targets and the small tests and
//!   setters they make;
//! - [`blend`]: the blend transform interpolator's update, its single-value store, the blend of
//!   several and `NiQuaternion::Normalize`;
//! - [`weights`]: `NiBlendInterpolator::ComputeNormalizedWeights` and its case of two;
//! - [`interp`]: `NiTransformInterpolator::Update` with its data's key getters, and
//!   `NiFloatInterpolator::Update`;
//! - [`queued`]: the queued scene graph updates.
//!
//! What stays by address: `BSTransCon::UpdateAllButTarget`, `NiBlendTransformInterpolator::Update`,
//! `NiBlendInterpolator::ComputeNormalizedWeights` and `NiTransformInterpolator::Update` where they
//! are called (typed here, but kNVSE hooks them, so a built-in may replace them:
//! `overridable.tsv`), an interpolator's own `Update` (slot +0x8c), the transform's getters and the
//! bone's setters, the tread controller's update.
//!
//! Stage 1: the objects stay in guest memory; each function stays registered at its exe address.
//! **On by default** since 2026-11-11 (`NV_TYPED=animation:off` turns it off): on its differential
//! tests, planted mutants and review, with no comparison run for native code (AGENT-RULES.md).
//! **Floats.** The actors' animation runs on the AI threads as well as main, so the row is
//! [`Threads::Both`] and the float maths is native at the running thread's precision: plain `f32`
//! on main (24 bits), `f64` locals rounded at each float store on the AI threads (53 bits)
//! (`crate::native::Real`); compares and copies plain (no x87 helper; the NaN quieting an x87 load
//! does is gone, which only a signalling NaN's payload could show). The interpolators, the blend
//! and the weights are the exe's arithmetic on both threads; the bip node's tests compare floats
//! within an ulp (`Diff::float_ulps`).

use engine_core::ptr::Ptr;
use engine_core::typed::{Family, FamilyId, Threads};
use engine_core::{entry, AbiFn};

pub mod bip;
pub mod blend;
pub mod interp;
pub mod queued;
pub mod weights;

/// This family's bit: what each `typed_call!` of it names.
pub const FAMILY: FamilyId = FamilyId::new(24);

/// `NV_TYPED=animation:off` turns it off. On by default, proved by its differential tests (each
/// planted mutant caught) and review.
pub static ANIMATION: Family = Family {
    name: "animation",
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
        entry!(0x0049_3930, queued::update_queued_scenegraph(Ptr) -> bool),
        entry!(0x0049_3bd0, queued::update_scene_graph_no_controller(Ptr)),
        entry!(0x004f_0040, bip::update_all_but_bip(Ptr, f32)),
        entry!(0x004f_0380, bip::update_all_but_target(Ptr, f32, u8)),
        entry!(0x004f_05a0, bip::controller_active(Ptr) -> u32),
        entry!(0x004f_0360, bip::flags_share(Ptr, u32) -> u32),
        entry!(0x004f_0140, bip::flag_2(Ptr) -> bool),
        entry!(0x004f_02e0, bip::rotation_valid(Ptr) -> bool),
        entry!(0x004f_0320, bip::scale_valid(Ptr) -> bool),
        entry!(0x004f_0160, bip::set_rotation(Ptr, Ptr)),
        entry!(0x004f_0180, bip::to_rotation(Ptr, Ptr)),
        entry!(0x00a4_1110, blend::update_transform(Ptr, f32, Ptr, Ptr) -> bool),
        entry!(0x00a4_0b30, blend::store_single_value(Ptr, f32, Ptr, Ptr) -> bool),
        entry!(0x00a4_0c10, blend::blend_values(Ptr, f32, Ptr, Ptr) -> bool),
        entry!(0x00a3_8560, blend::normalize(Ptr)),
        entry!(0x00a3_7260, weights::compute_normalized_weights(Ptr)),
        entry!(0x00a3_6bd0, weights::compute_for_two(Ptr)),
        entry!(0x00a3_fdb0, interp::transform_update(Ptr, f32, u32, Ptr) -> bool),
        entry!(0x00a3_f990, interp::translate_keys(Ptr, Ptr, Ptr, Ptr) -> u32),
        entry!(0x0058_cb00, interp::rotate_keys(Ptr, Ptr, Ptr, Ptr) -> u32),
        entry!(0x00a3_f9e0, interp::scale_keys(Ptr, Ptr, Ptr, Ptr) -> u32),
        entry!(0x00a3_c6d0, interp::float_update(Ptr, f32, u32, Ptr) -> bool),
    ]
}
