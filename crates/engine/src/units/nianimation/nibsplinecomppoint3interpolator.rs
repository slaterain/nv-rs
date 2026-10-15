//! `NiAnimation/nibsplinecomppoint3interpolator.cpp` (Xbox PDB source unit), subsystem `NiAnimation`: its functions in
//! FalloutNV.exe 1.4.0.525, translated (docs/ENGINE_CRATE.md).

#[allow(unused_imports)]
use crate::prelude::*;

/// `NiBSplineCompPoint3Interpolator::ms_RTTI` (the class's `NiRTTI` record).
const NI_B_SPLINE_COMP_POINT3_INTERPOLATOR_RTTI: u32 = 0x011f_4014;

// Translated from 00a51270 (decompiled, FalloutNV.exe 1.4.0.525)
/// `NiBSplineCompPoint3Interpolator::GetRTTI` (Xbox PDB): the address of the class's `NiRTTI` record.
pub fn ni_b_spline_comp_point3_interpolator_get_rtti(_e: &mut Engine) -> Ptr {
    Ptr::new(NI_B_SPLINE_COMP_POINT3_INTERPOLATOR_RTTI)
}

/// This unit's translated functions, by exe address.
pub fn funcs() -> Vec<(u32, AbiFn)> {
    vec![entry!(0x00a51270, ni_b_spline_comp_point3_interpolator_get_rtti() -> Ptr)]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ni_b_spline_comp_point3_interpolator_get_rtti_returns_the_class_rtti() {
        let mut e = Engine::new();
        assert_eq!(
            e.call(0x00a51270, &args![]).u32(),
            NI_B_SPLINE_COMP_POINT3_INTERPOLATOR_RTTI
        );
    }
}
