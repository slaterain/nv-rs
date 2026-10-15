//! `NiMain/nigeometrydata.cpp` (Xbox PDB source unit), subsystem `NiMain`: its functions in
//! FalloutNV.exe 1.4.0.525, translated (docs/ENGINE_CRATE.md).

#[allow(unused_imports)]
use crate::prelude::*;

/// `NiGeometryData::ms_RTTI` (the class's `NiRTTI` record).
const NI_GEOMETRY_DATA_RTTI: u32 = 0x011f_4884;

// Translated from 00a67bf0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `NiGeometryData::GetRTTI` (Xbox PDB): the address of the class's `NiRTTI` record.
pub fn ni_geometry_data_get_rtti(_e: &mut Engine) -> Ptr {
    Ptr::new(NI_GEOMETRY_DATA_RTTI)
}

/// This unit's translated functions, by exe address.
pub fn funcs() -> Vec<(u32, AbiFn)> {
    vec![entry!(0x00a67bf0, ni_geometry_data_get_rtti() -> Ptr)]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ni_geometry_data_get_rtti_returns_the_class_rtti() {
        let mut e = Engine::new();
        assert_eq!(e.call(0x00a67bf0, &args![]).u32(), NI_GEOMETRY_DATA_RTTI);
    }
}
