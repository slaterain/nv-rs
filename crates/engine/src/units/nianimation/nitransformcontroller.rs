//! `NiAnimation/nitransformcontroller.cpp` (Xbox PDB source unit), subsystem `NiAnimation`: its functions in
//! FalloutNV.exe 1.4.0.525, translated (docs/ENGINE_CRATE.md).

#[allow(unused_imports)]
use crate::prelude::*;

/// `NiBlendAccumTransformInterpolator::ms_RTTI` (the class's `NiRTTI` record).
const NI_BLEND_ACCUM_TRANSFORM_INTERPOLATOR_RTTI: u32 = 0x011f_371c;

/// `NiTransformController::ms_RTTI` (the class's `NiRTTI` record).
const NI_TRANSFORM_CONTROLLER_RTTI: u32 = 0x011f_36ec;

// Translated from 00a36480 (decompiled, FalloutNV.exe 1.4.0.525)
/// `NiBlendAccumTransformInterpolator::GetRTTI` (Xbox PDB): the address of the class's `NiRTTI` record.
pub fn ni_blend_accum_transform_interpolator_get_rtti(_e: &mut Engine) -> Ptr {
    Ptr::new(NI_BLEND_ACCUM_TRANSFORM_INTERPOLATOR_RTTI)
}

// Translated from 00a36580 (decompiled, FalloutNV.exe 1.4.0.525)
/// `NiTransformController::GetRTTI` (Xbox PDB): the address of the class's `NiRTTI` record.
pub fn ni_transform_controller_get_rtti(_e: &mut Engine) -> Ptr {
    Ptr::new(NI_TRANSFORM_CONTROLLER_RTTI)
}

/// This unit's translated functions, by exe address.
pub fn funcs() -> Vec<(u32, AbiFn)> {
    vec![
        entry!(0x00a36480, ni_blend_accum_transform_interpolator_get_rtti() -> Ptr),
        entry!(0x00a36580, ni_transform_controller_get_rtti() -> Ptr),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ni_blend_accum_transform_interpolator_get_rtti_returns_the_class_rtti() {
        let mut e = Engine::new();
        assert_eq!(
            e.call(0x00a36480, &args![]).u32(),
            NI_BLEND_ACCUM_TRANSFORM_INTERPOLATOR_RTTI
        );
    }

    #[test]
    fn ni_transform_controller_get_rtti_returns_the_class_rtti() {
        let mut e = Engine::new();
        assert_eq!(
            e.call(0x00a36580, &args![]).u32(),
            NI_TRANSFORM_CONTROLLER_RTTI
        );
    }
}
