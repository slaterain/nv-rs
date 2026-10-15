//! `NiAnimation/nipathinterpolator.cpp` (Xbox PDB source unit), subsystem `NiAnimation`: its functions in
//! FalloutNV.exe 1.4.0.525, translated (docs/ENGINE_CRATE.md).

#[allow(unused_imports)]
use crate::prelude::*;

/// `NiPosData::ms_RTTI` (the class's `NiRTTI` record).
const NI_POS_DATA_RTTI: u32 = 0x011f_3f68;

/// `NiPathInterpolator::ms_RTTI` (the class's `NiRTTI` record).
const NI_PATH_INTERPOLATOR_RTTI: u32 = 0x011f_3e48;

// Translated from 00a3dab0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `NiPosData::GetRTTI` (Xbox PDB): the address of the class's `NiRTTI` record.
pub fn ni_pos_data_get_rtti(_e: &mut Engine) -> Ptr {
    Ptr::new(NI_POS_DATA_RTTI)
}

// Translated from 00a3dde0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `NiPathInterpolator::GetRTTI` (Xbox PDB): the address of the class's `NiRTTI` record.
pub fn ni_path_interpolator_get_rtti(_e: &mut Engine) -> Ptr {
    Ptr::new(NI_PATH_INTERPOLATOR_RTTI)
}

/// This unit's translated functions, by exe address.
pub fn funcs() -> Vec<(u32, AbiFn)> {
    vec![
        entry!(0x00a3dab0, ni_pos_data_get_rtti() -> Ptr),
        entry!(0x00a3dde0, ni_path_interpolator_get_rtti() -> Ptr),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ni_pos_data_get_rtti_returns_the_class_rtti() {
        let mut e = Engine::new();
        assert_eq!(e.call(0x00a3dab0, &args![]).u32(), NI_POS_DATA_RTTI);
    }

    #[test]
    fn ni_path_interpolator_get_rtti_returns_the_class_rtti() {
        let mut e = Engine::new();
        assert_eq!(
            e.call(0x00a3dde0, &args![]).u32(),
            NI_PATH_INTERPOLATOR_RTTI
        );
    }
}
