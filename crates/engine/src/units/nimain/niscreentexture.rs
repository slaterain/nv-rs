//! `NiMain/niscreentexture.cpp` (Xbox PDB source unit), subsystem `NiMain`: its functions in
//! FalloutNV.exe 1.4.0.525, translated (docs/ENGINE_CRATE.md).

#[allow(unused_imports)]
use crate::prelude::*;

/// `NiScreenTexture::ms_RTTI` (the class's `NiRTTI` record).
const NI_SCREEN_TEXTURE_RTTI: u32 = 0x011f_5ee0;

// Translated from 00a955f0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `NiScreenTexture::GetRTTI` (Xbox PDB): the address of the class's `NiRTTI` record.
pub fn ni_screen_texture_get_rtti(_e: &mut Engine) -> Ptr {
    Ptr::new(NI_SCREEN_TEXTURE_RTTI)
}

/// This unit's translated functions, by exe address.
pub fn funcs() -> Vec<(u32, AbiFn)> {
    vec![entry!(0x00a955f0, ni_screen_texture_get_rtti() -> Ptr)]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ni_screen_texture_get_rtti_returns_the_class_rtti() {
        let mut e = Engine::new();
        assert_eq!(e.call(0x00a955f0, &args![]).u32(), NI_SCREEN_TEXTURE_RTTI);
    }
}
