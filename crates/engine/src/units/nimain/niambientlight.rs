//! `NiMain/niambientlight.cpp` (Xbox PDB source unit), subsystem `NiMain`: its functions in
//! FalloutNV.exe 1.4.0.525, translated (docs/ENGINE_CRATE.md).

#[allow(unused_imports)]
use crate::prelude::*;

/// `NiAmbientLight::ms_RTTI` (the class's `NiRTTI` record).
const NI_AMBIENT_LIGHT_RTTI: u32 = 0x011f_5f6c;

// Translated from 00a9b390 (decompiled, FalloutNV.exe 1.4.0.525)
/// `NiAmbientLight::GetRTTI` (Xbox PDB): the address of the class's `NiRTTI` record.
pub fn ni_ambient_light_get_rtti(_e: &mut Engine) -> Ptr {
    Ptr::new(NI_AMBIENT_LIGHT_RTTI)
}

/// This unit's translated functions, by exe address.
pub fn funcs() -> Vec<(u32, AbiFn)> {
    vec![entry!(0x00a9b390, ni_ambient_light_get_rtti() -> Ptr)]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ni_ambient_light_get_rtti_returns_the_class_rtti() {
        let mut e = Engine::new();
        assert_eq!(e.call(0x00a9b390, &args![]).u32(), NI_AMBIENT_LIGHT_RTTI);
    }
}
