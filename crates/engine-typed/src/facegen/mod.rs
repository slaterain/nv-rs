//! The face family (stage 1): the vertex work of a head's face morphing, which every head close to
//! the camera redoes each frame (`BSFaceGenNiNode::UpdateMorphing` after the frame is shown, the
//! translated `00663050`): the base offsets copied into the face geometry's locked vertices, then each
//! morph whose weight is above zero added on top. In the translations every vertex costs several
//! calls by address (the strided cursor's read, write and step, the point's scale and add); here each
//! is a loop over the locked vertex array with direct loads and stores.
//!
//! - [`keyframe`]: `BSFaceGenKeyframeMultiple`'s weight array: its getter and setter, the copy, compare,
//!   strength, availability, the move toward a target and the merge (`0064f5b0`..`0064f900`, `0064eea0`).
//! - [`morph`]: `BSFaceGenMorphDifferential::ApplyMorph` (`00660860`),
//!   `BSFaceGenMorphStatistical::ApplyMorph` (`00660a90`) and the base copy (`00653340`).
//!
//! What stays by address: the geometry's data and its vertex count getters, the locked array's
//! start (`00a67410`), the model extra data's getter, the data's `MarkAsChanged`.
//!
//! Stage 1: the objects stay in guest memory; each function stays registered at its exe address.
//! **Floats.** Heads are morphed on main and, while a head is made, on the loader threads, so the
//! row is [`Threads::Both`] and the float maths is native over [`crate::native::Real`] at the running
//! thread's precision (each product and sum stored as a float where the exe stores it); the copy has
//! no float maths.

use engine_core::ptr::Ptr;
use engine_core::typed::{Family, FamilyId, Threads};
use engine_core::{entry, AbiFn};

pub mod keyframe;
pub mod morph;

/// This family's bit: what each `typed_call!` of it names.
pub const FAMILY: FamilyId = FamilyId::new(26);

/// `NV_TYPED=facegen`. Native code, so on by its differential tests and planted mutants (the
/// native-code rule of 2026-11-11) unless `NV_TYPED` says otherwise.
pub static FACEGEN: Family = Family {
    name: "facegen",
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
        entry!(0x0066_0860, morph::differential_apply(Ptr, u32, u32, f32)),
        entry!(0x0066_0a90, morph::statistical_apply(Ptr, u32, u32, f32)),
        entry!(0x0065_3340, morph::copy_base(u32, u32)),
        entry!(0x005d_d4e0, keyframe::quantity(Ptr, u32) -> f32),
        entry!(0x0064_a4d0, keyframe::set_quantity(Ptr, u32, f32) -> bool),
        entry!(0x0044_edb0, keyframe::word_at_10(Ptr) -> u32),
        entry!(0x0064_fb90, keyframe::is_valid_quantity(Ptr, u32) -> bool),
        entry!(0x0064_f5b0, keyframe::copy_from(Ptr, u32)),
        entry!(0x0064_f690, keyframe::compare(Ptr, u32) -> bool),
        entry!(0x0064_f760, keyframe::get_strength(Ptr) -> f64),
        entry!(0x0064_f880, keyframe::is_target_available(Ptr) -> bool),
        entry!(0x0064_f900, keyframe::update_to_target(Ptr, f32, u32) -> bool),
        entry!(0x0064_eea0, keyframe::merge_morph_data_from(Ptr, u32, f32, u8, u8) -> bool),
    ]
}
