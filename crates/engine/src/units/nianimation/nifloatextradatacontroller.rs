//! `NiAnimation/nifloatextradatacontroller.cpp` (Xbox PDB source unit), subsystem `NiAnimation`: its functions in
//! FalloutNV.exe 1.4.0.525, translated (docs/ENGINE_CRATE.md).

#[allow(unused_imports)]
use crate::prelude::*;

/// `NiFloatExtraDataController::ms_RTTI` (the class's `NiRTTI` record).
const NI_FLOAT_EXTRA_DATA_CONTROLLER_RTTI: u32 = 0x011f_3e6c;

// Translated from 00a41230 (decompiled, FalloutNV.exe 1.4.0.525)
/// `NiFloatExtraDataController::GetRTTI` (Xbox PDB): the address of the class's `NiRTTI` record.
pub fn ni_float_extra_data_controller_get_rtti(_e: &mut Engine) -> Ptr {
    Ptr::new(NI_FLOAT_EXTRA_DATA_CONTROLLER_RTTI)
}

/// This unit's translated functions, by exe address.
pub fn funcs() -> Vec<(u32, AbiFn)> {
    vec![entry!(0x00a41230, ni_float_extra_data_controller_get_rtti() -> Ptr)]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ni_float_extra_data_controller_get_rtti_returns_the_class_rtti() {
        let mut e = Engine::new();
        assert_eq!(
            e.call(0x00a41230, &args![]).u32(),
            NI_FLOAT_EXTRA_DATA_CONTROLLER_RTTI
        );
    }
}
