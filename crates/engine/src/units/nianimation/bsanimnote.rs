//! `NiAnimation/bsanimnote.cpp` (Xbox PDB source unit), subsystem `NiAnimation`: its functions in
//! FalloutNV.exe 1.4.0.525, translated (docs/ENGINE_CRATE.md).

#[allow(unused_imports)]
use crate::prelude::*;

/// `BSAnimNotes::ms_RTTI` (the class's `NiRTTI` record).
const BS_ANIM_NOTES_RTTI: u32 = 0x011f_3f00;

// Translated from 00a44920 (decompiled, FalloutNV.exe 1.4.0.525)
/// `BSAnimNotes::GetRTTI` (Xbox PDB): the address of the class's `NiRTTI` record.
pub fn bs_anim_notes_get_rtti(_e: &mut Engine) -> Ptr {
    Ptr::new(BS_ANIM_NOTES_RTTI)
}

/// This unit's translated functions, by exe address.
pub fn funcs() -> Vec<(u32, AbiFn)> {
    vec![entry!(0x00a44920, bs_anim_notes_get_rtti() -> Ptr)]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bs_anim_notes_get_rtti_returns_the_class_rtti() {
        let mut e = Engine::new();
        assert_eq!(e.call(0x00a44920, &args![]).u32(), BS_ANIM_NOTES_RTTI);
    }
}
