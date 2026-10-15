//! `NiMain/nibacktofrontaccumulator.cpp` (Xbox PDB source unit), subsystem `NiMain`: its functions in
//! FalloutNV.exe 1.4.0.525, translated (docs/ENGINE_CRATE.md).

#[allow(unused_imports)]
use crate::prelude::*;

/// `NiBackToFrontAccumulator::ms_RTTI` (the class's `NiRTTI` record).
const NI_BACK_TO_FRONT_ACCUMULATOR_RTTI: u32 = 0x011f_6058;

// Translated from 00aa0e50 (decompiled, FalloutNV.exe 1.4.0.525)
/// `NiBackToFrontAccumulator::GetRTTI` (Xbox PDB): the address of the class's `NiRTTI` record.
pub fn ni_back_to_front_accumulator_get_rtti(_e: &mut Engine) -> Ptr {
    Ptr::new(NI_BACK_TO_FRONT_ACCUMULATOR_RTTI)
}

/// This unit's translated functions, by exe address.
pub fn funcs() -> Vec<(u32, AbiFn)> {
    vec![entry!(0x00aa0e50, ni_back_to_front_accumulator_get_rtti() -> Ptr)]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ni_back_to_front_accumulator_get_rtti_returns_the_class_rtti() {
        let mut e = Engine::new();
        assert_eq!(
            e.call(0x00aa0e50, &args![]).u32(),
            NI_BACK_TO_FRONT_ACCUMULATOR_RTTI
        );
    }
}
