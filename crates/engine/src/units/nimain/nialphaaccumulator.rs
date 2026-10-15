//! `NiMain/nialphaaccumulator.cpp` (Xbox PDB source unit), subsystem `NiMain`: its functions in
//! FalloutNV.exe 1.4.0.525, translated (docs/ENGINE_CRATE.md).

#[allow(unused_imports)]
use crate::prelude::*;

/// `NiAlphaAccumulator::ms_RTTI` (the class's `NiRTTI` record).
const NI_ALPHA_ACCUMULATOR_RTTI: u32 = 0x011f_5f74;

// Translated from 00a9b530 (decompiled, FalloutNV.exe 1.4.0.525)
/// `NiAlphaAccumulator::GetRTTI` (Xbox PDB): the address of the class's `NiRTTI` record.
pub fn ni_alpha_accumulator_get_rtti(_e: &mut Engine) -> Ptr {
    Ptr::new(NI_ALPHA_ACCUMULATOR_RTTI)
}

/// This unit's translated functions, by exe address.
pub fn funcs() -> Vec<(u32, AbiFn)> {
    vec![entry!(0x00a9b530, ni_alpha_accumulator_get_rtti() -> Ptr)]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ni_alpha_accumulator_get_rtti_returns_the_class_rtti() {
        let mut e = Engine::new();
        assert_eq!(
            e.call(0x00a9b530, &args![]).u32(),
            NI_ALPHA_ACCUMULATOR_RTTI
        );
    }
}
