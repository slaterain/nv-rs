//! `NiAnimation/nibooltimelineinterpolator.cpp` (Xbox PDB source unit), subsystem `NiAnimation`: its functions in
//! FalloutNV.exe 1.4.0.525, translated (docs/ENGINE_CRATE.md).

#[allow(unused_imports)]
use crate::prelude::*;

/// `NiBoolTimelineInterpolator::ms_RTTI` (the class's `NiRTTI` record).
const NI_BOOL_TIMELINE_INTERPOLATOR_RTTI: u32 = 0x011f_4044;

// Translated from 00a52d70 (decompiled, FalloutNV.exe 1.4.0.525)
/// `NiBoolTimelineInterpolator::GetRTTI` (Xbox PDB): the address of the class's `NiRTTI` record.
pub fn ni_bool_timeline_interpolator_get_rtti(_e: &mut Engine) -> Ptr {
    Ptr::new(NI_BOOL_TIMELINE_INTERPOLATOR_RTTI)
}

/// This unit's translated functions, by exe address.
pub fn funcs() -> Vec<(u32, AbiFn)> {
    vec![entry!(0x00a52d70, ni_bool_timeline_interpolator_get_rtti() -> Ptr)]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ni_bool_timeline_interpolator_get_rtti_returns_the_class_rtti() {
        let mut e = Engine::new();
        assert_eq!(
            e.call(0x00a52d70, &args![]).u32(),
            NI_BOOL_TIMELINE_INTERPOLATOR_RTTI
        );
    }
}
