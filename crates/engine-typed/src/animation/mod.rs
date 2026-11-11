//! The animation family (the update half's fifth group, stage 1): a skeleton's bones moved from their
//! animation each frame. `BSBipNode::UpdateAllButBip` walks a bip node's controllers; a
//! `NiMultiTargetTransformController` updates each bone it drives (`BSTransCon::UpdateAllButTarget`)
//! from its interpolator's transform (`NiBlendTransformInterpolator::Update`, through the interpolator's
//! slot, with its single-value store), and the transform's translation, rotation (from its quaternion,
//! `NiQuaternion::ToRotation`) and scale go into the bone. The player's update reaches it
//! through `Actor::UpdateAnimationMovement` (twice a frame), the actors' through their animation updates.
//!
//! - [`bip`]: the bip node's walk, the transform controller's targets and the small tests and
//!   setters they make;
//! - [`blend`]: the blend transform interpolator's update and its single-value store.
//!
//! What stays by address: `BSTransCon::UpdateAllButTarget` and `NiBlendTransformInterpolator::Update`
//! themselves where they are called (typed here, but kNVSE hooks both, so a built-in may replace them:
//! `overridable.tsv`), the blend of several interpolators (`BlendValues`, `00a40c10`) and the
//! weights' normalisation, the single interpolator's own `Update` (slot +0x8c, the key lookups), the
//! quaternion's normalisation, the transform's getters and the bone's setters, the tread
//! controller's update.
//!
//! Stage 1: the objects stay in guest memory; each function stays registered at its exe address.
//! **Off by default** (`NV_TYPED=animation` turns it on) until a batch gate (AGENT-RULES.md, "Typed
//! code merges OFF first"). **Floats.** The actors' animation runs on the AI threads as well as main,
//! so the row is [`Threads::Both`] and the x87 code keeps the exact helpers ([`engine_core::x87`]),
//! each load, operation and store the exe's, in its order, as the translations have them.

use engine_core::ptr::Ptr;
use engine_core::typed::{Family, FamilyId, Threads};
use engine_core::{entry, AbiFn};

pub mod bip;
pub mod blend;

/// This family's bit: what each `typed_call!` of it names.
pub const FAMILY: FamilyId = FamilyId::new(24);

/// `NV_TYPED=animation`. Proved by its differential tests (each planted mutant caught); off until a
/// batch gate runs the recorded street run with it on against off.
pub static ANIMATION: Family = Family {
    name: "animation",
    id: FAMILY,
    past_gate: false,
    threads: Threads::Both,
    stage2: false,
    funcs,
};

/// The family's functions, by exe address.
#[rustfmt::skip]
fn funcs() -> Vec<(u32, AbiFn)> {
    vec![
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
    ]
}
