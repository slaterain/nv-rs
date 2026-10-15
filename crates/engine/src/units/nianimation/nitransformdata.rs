//! `NiAnimation/nitransformdata.cpp` (Xbox PDB source unit), subsystem `NiAnimation`: its functions in
//! FalloutNV.exe 1.4.0.525, translated (docs/ENGINE_CRATE.md).

#[allow(unused_imports)]
use crate::prelude::*;

/// `NiTransformData::ms_RTTI` (the class's `NiRTTI` record).
const NI_TRANSFORM_DATA_RTTI: u32 = 0x011f_3fbc;

// Translated from 00a4e0a0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `NiTransformData::GetRTTI` (Xbox PDB): the address of the class's `NiRTTI` record.
pub fn ni_transform_data_get_rtti(_e: &mut Engine) -> Ptr {
    Ptr::new(NI_TRANSFORM_DATA_RTTI)
}

/// This unit's translated functions, by exe address.
pub fn funcs() -> Vec<(u32, AbiFn)> {
    vec![entry!(0x00a4e0a0, ni_transform_data_get_rtti() -> Ptr)]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ni_transform_data_get_rtti_returns_the_class_rtti() {
        let mut e = Engine::new();
        assert_eq!(e.call(0x00a4e0a0, &args![]).u32(), NI_TRANSFORM_DATA_RTTI);
    }
}
