//! `NiAnimation/nirollcontroller.cpp` (Xbox PDB source unit), subsystem `NiAnimation`: its functions in
//! FalloutNV.exe 1.4.0.525, translated (docs/ENGINE_CRATE.md).

#[allow(unused_imports)]
use crate::prelude::*;

/// `NiFloatController::ms_RTTI` (the class's `NiRTTI` record).
const NI_FLOAT_CONTROLLER_RTTI: u32 = 0x011f_4230;

/// `NiRollController::ms_RTTI` (the class's `NiRTTI` record).
const NI_ROLL_CONTROLLER_RTTI: u32 = 0x011f_3f58;

// Translated from 00a485a0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `NiFloatController::GetRTTI` (Xbox PDB): the address of the class's `NiRTTI` record.
pub fn ni_float_controller_get_rtti(_e: &mut Engine) -> Ptr {
    Ptr::new(NI_FLOAT_CONTROLLER_RTTI)
}

// Translated from 00a485b0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `NiRollController::GetRTTI` (Xbox PDB): the address of the class's `NiRTTI` record.
pub fn ni_roll_controller_get_rtti(_e: &mut Engine) -> Ptr {
    Ptr::new(NI_ROLL_CONTROLLER_RTTI)
}

/// This unit's translated functions, by exe address.
pub fn funcs() -> Vec<(u32, AbiFn)> {
    vec![
        entry!(0x00a485a0, ni_float_controller_get_rtti() -> Ptr),
        entry!(0x00a485b0, ni_roll_controller_get_rtti() -> Ptr),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ni_float_controller_get_rtti_returns_the_class_rtti() {
        let mut e = Engine::new();
        assert_eq!(e.call(0x00a485a0, &args![]).u32(), NI_FLOAT_CONTROLLER_RTTI);
    }

    #[test]
    fn ni_roll_controller_get_rtti_returns_the_class_rtti() {
        let mut e = Engine::new();
        assert_eq!(e.call(0x00a485b0, &args![]).u32(), NI_ROLL_CONTROLLER_RTTI);
    }
}
