//! `NiAnimation/nicontrollermanager.cpp` (Xbox PDB source unit), subsystem `NiAnimation`: its functions in
//! FalloutNV.exe 1.4.0.525, translated (docs/ENGINE_CRATE.md).

#[allow(unused_imports)]
use crate::prelude::*;

/// `NiAVObjectPalette::ms_RTTI` (the class's `NiRTTI` record).
const NI_AV_OBJECT_PALETTE_RTTI: u32 = 0x011f_5b58;

/// `NiDefaultAVObjectPalette::ms_RTTI` (the class's `NiRTTI` record).
const NI_DEFAULT_AV_OBJECT_PALETTE_RTTI: u32 = 0x011f_4a04;

// Translated from 00a2e1e0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `NiAVObjectPalette::GetRTTI` (Xbox PDB): the address of the class's `NiRTTI` record.
pub fn ni_av_object_palette_get_rtti(_e: &mut Engine) -> Ptr {
    Ptr::new(NI_AV_OBJECT_PALETTE_RTTI)
}

// Translated from 00a2ede0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `NiDefaultAVObjectPalette::GetRTTI` (Xbox PDB): the address of the class's `NiRTTI` record.
pub fn ni_default_av_object_palette_get_rtti(_e: &mut Engine) -> Ptr {
    Ptr::new(NI_DEFAULT_AV_OBJECT_PALETTE_RTTI)
}

/// This unit's translated functions, by exe address.
pub fn funcs() -> Vec<(u32, AbiFn)> {
    vec![
        entry!(0x00a2e1e0, ni_av_object_palette_get_rtti() -> Ptr),
        entry!(0x00a2ede0, ni_default_av_object_palette_get_rtti() -> Ptr),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ni_av_object_palette_get_rtti_returns_the_class_rtti() {
        let mut e = Engine::new();
        assert_eq!(
            e.call(0x00a2e1e0, &args![]).u32(),
            NI_AV_OBJECT_PALETTE_RTTI
        );
    }

    #[test]
    fn ni_default_av_object_palette_get_rtti_returns_the_class_rtti() {
        let mut e = Engine::new();
        assert_eq!(
            e.call(0x00a2ede0, &args![]).u32(),
            NI_DEFAULT_AV_OBJECT_PALETTE_RTTI
        );
    }
}
