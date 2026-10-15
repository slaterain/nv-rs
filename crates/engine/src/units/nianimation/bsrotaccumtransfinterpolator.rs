//! `NiAnimation/bsrotaccumtransfinterpolator.cpp` (Xbox PDB source unit), subsystem `NiAnimation`: its functions in
//! FalloutNV.exe 1.4.0.525, translated (docs/ENGINE_CRATE.md).

#[allow(unused_imports)]
use crate::prelude::*;

/// `BSRotAccumTransfInterpolator::ms_RTTI` (the class's `NiRTTI` record).
const BS_ROT_ACCUM_TRANSF_INTERPOLATOR_RTTI: u32 = 0x011f_3ee0;

// Translated from 00a43ab0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `BSRotAccumTransfInterpolator::GetRTTI` (Xbox PDB): the address of the class's `NiRTTI` record.
pub fn bs_rot_accum_transf_interpolator_get_rtti(_e: &mut Engine) -> Ptr {
    Ptr::new(BS_ROT_ACCUM_TRANSF_INTERPOLATOR_RTTI)
}

/// This unit's translated functions, by exe address.
pub fn funcs() -> Vec<(u32, AbiFn)> {
    vec![entry!(0x00a43ab0, bs_rot_accum_transf_interpolator_get_rtti() -> Ptr)]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bs_rot_accum_transf_interpolator_get_rtti_returns_the_class_rtti() {
        let mut e = Engine::new();
        assert_eq!(
            e.call(0x00a43ab0, &args![]).u32(),
            BS_ROT_ACCUM_TRANSF_INTERPOLATOR_RTTI
        );
    }
}
