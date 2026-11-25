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
//! - [`head`]: making a head on a loading thread under the face generation lock: the EGM morphs moved
//!   into a new head part's vertices (`0065a970`), the face tint summed from the EGT morphs
//!   (`ApplyCoordinateTexturingToMesh`, `0065b410`) and packed into pixel bytes
//!   (`BSFaceGenImage::GetCompressedImage`, `0064ceb0`).
//! - [`coords`]: the sum of two face coordinate blocks (`00652af0`), which the race menu runs once
//!   a slider at every click.
//! - [`apply`]: the head's morph pass (`00663050`) with its appliers, the morph data's apply and tables,
//!   the animation data's weight getters, and the exact skip of a reshape that would change nothing.
//!
//! What stays by address (a call a morph, not a vertex): the geometry's data and vertex count
//! getters, the locked array's start (`00a67410`), the morph extra data's getter, the data's
//! `MarkAsChanged`, the settings and the nearly-equal test; every virtual call whose slot does not
//! hold this class's own function. None of these functions is an extender's hook
//! (`data/overridable.tsv`), and each stays registered at its exe address, so a call by address
//! still reaches it.
//!
//! Stage 1: the objects stay in guest memory.
//! **Floats.** Heads are morphed on main after the frame is shown; the base copy and the keyframe
//! functions are also reached while a head is made, which may be a loading thread, so the row is
//! [`Threads::Both`] and the tests run under both threads' control words. The float maths is native (ADR-0014's float rule): a result that is
//! one operation on two floats, stored (every vertex step), is plain `f32`, the same float at
//! either precision; the keyframes' two chains run in [`crate::native::Real`] at the running
//! thread's precision and are stored where the exe stores them. The copy has no float maths.

use engine_core::ptr::Ptr;
use engine_core::typed::{Family, FamilyId, Threads};
use engine_core::{entry, AbiFn};

pub mod apply;
pub mod coords;
pub mod head;
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
        entry!(0x0066_3050, apply::morph_pass(Ptr, u8)),
        entry!(0x0066_2bc0, apply::apply_expressions(Ptr, u32)),
        entry!(0x0066_2e90, apply::apply_phonemes(Ptr, u32)),
        entry!(0x0066_2f70, apply::apply_custom(Ptr, u32)),
        entry!(0x0066_2ca0, apply::apply_modifiers(Ptr, u32)),
        entry!(0x0066_2030, apply::apply_one(Ptr, u32, i32, u32, u32, f32)),
        entry!(0x0066_20d0, apply::expression_morph(Ptr, i32) -> u32),
        entry!(0x0066_2110, apply::modifier_morph(Ptr, i32) -> u32),
        entry!(0x0066_2150, apply::phoneme_morph(Ptr, i32) -> u32),
        entry!(0x0066_2190, apply::custom_morph(Ptr, i32) -> u32),
        entry!(0x0064_99e0, apply::expression_value(Ptr, i32) -> f32),
        entry!(0x0064_9aa0, apply::phoneme_value(Ptr, i32) -> f32),
        entry!(0x0064_9b10, apply::custom_value(Ptr, i32) -> f32),
        entry!(0x005d_d4a0, apply::modifier_value(Ptr, i32) -> f32),
        entry!(0x0065_3d70, apply::split_pair(f32, f32, u32, u32)),
        entry!(0x0065_a970, head::apply_coordinates(Ptr, u32, u32, u8) -> bool),
        entry!(0x0065_b410, head::apply_coordinate_texturing(Ptr, u32, u32, u32) -> bool),
        entry!(0x0064_ceb0, head::get_compressed_image(Ptr, f32, f32, f32, u32) -> u32),
        entry!(0x0065_2af0, coords::sum_coordinates(u32, u32, u32, u8, f32)),
    ]
}
