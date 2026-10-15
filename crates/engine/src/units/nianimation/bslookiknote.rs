//! `NiAnimation/bslookiknote.cpp` (Xbox PDB source unit), subsystem `NiAnimation`: its functions in
//! FalloutNV.exe 1.4.0.525, translated (docs/ENGINE_CRATE.md).

#[allow(unused_imports)]
use crate::prelude::*;

/// `BSAnimNote::ms_RTTI` (the class's `NiRTTI` record).
const BS_ANIM_NOTE_RTTI: u32 = 0x011f_3ef8;

/// `BSLookIKNote::ms_RTTI` (the class's `NiRTTI` record).
const BS_LOOK_IK_NOTE_RTTI: u32 = 0x011f_3ee8;

// Translated from 00a43f70 (decompiled, FalloutNV.exe 1.4.0.525)
/// `BSAnimNote::GetRTTI` (Xbox PDB): the address of the class's `NiRTTI` record.
pub fn bs_anim_note_get_rtti(_e: &mut Engine) -> Ptr {
    Ptr::new(BS_ANIM_NOTE_RTTI)
}

// Translated from 00a43f80 (decompiled, FalloutNV.exe 1.4.0.525)
/// `BSLookIKNote::GetRTTI` (Xbox PDB): the address of the class's `NiRTTI` record.
pub fn bs_look_ik_note_get_rtti(_e: &mut Engine) -> Ptr {
    Ptr::new(BS_LOOK_IK_NOTE_RTTI)
}

/// This unit's translated functions, by exe address.
pub fn funcs() -> Vec<(u32, AbiFn)> {
    vec![
        entry!(0x00a43f70, bs_anim_note_get_rtti() -> Ptr),
        entry!(0x00a43f80, bs_look_ik_note_get_rtti() -> Ptr),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bs_anim_note_get_rtti_returns_the_class_rtti() {
        let mut e = Engine::new();
        assert_eq!(e.call(0x00a43f70, &args![]).u32(), BS_ANIM_NOTE_RTTI);
    }

    #[test]
    fn bs_look_ik_note_get_rtti_returns_the_class_rtti() {
        let mut e = Engine::new();
        assert_eq!(e.call(0x00a43f80, &args![]).u32(), BS_LOOK_IK_NOTE_RTTI);
    }
}
