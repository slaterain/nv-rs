//! `NiAnimation/bsmultitargettreadtransfcontroller.cpp` (Xbox PDB source unit), subsystem `NiAnimation`: its functions in
//! FalloutNV.exe 1.4.0.525, translated (docs/ENGINE_CRATE.md).

#[allow(unused_imports)]
use crate::prelude::*;

/// `BSMultiTargetTreadTransfController::ms_RTTI` (the class's `NiRTTI` record).
const BS_MULTI_TARGET_TREAD_TRANSF_CONTROLLER_RTTI: u32 = 0x011f_36e4;

// Translated from 00a35950 (decompiled, FalloutNV.exe 1.4.0.525)
/// `BSMultiTargetTreadTransfController::GetRTTI` (Xbox PDB): the address of the class's `NiRTTI` record.
pub fn bs_multi_target_tread_transf_controller_get_rtti(_e: &mut Engine) -> Ptr {
    Ptr::new(BS_MULTI_TARGET_TREAD_TRANSF_CONTROLLER_RTTI)
}

/// This unit's translated functions, by exe address.
pub fn funcs() -> Vec<(u32, AbiFn)> {
    vec![entry!(0x00a35950, bs_multi_target_tread_transf_controller_get_rtti() -> Ptr)]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bs_multi_target_tread_transf_controller_get_rtti_returns_the_class_rtti() {
        let mut e = Engine::new();
        assert_eq!(
            e.call(0x00a35950, &args![]).u32(),
            BS_MULTI_TARGET_TREAD_TRANSF_CONTROLLER_RTTI
        );
    }
}
