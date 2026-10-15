//! `NiMain/ni2dbuffer.cpp` (Xbox PDB source unit), subsystem `NiMain`: its functions in
//! FalloutNV.exe 1.4.0.525, translated (docs/ENGINE_CRATE.md).

#[allow(unused_imports)]
use crate::prelude::*;

/// `Ni2DBuffer::ms_RTTI` (the class's `NiRTTI` record).
const NI2_D_BUFFER_RTTI: u32 = 0x011f_5d00;

// Translated from 00a8f1b0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Ni2DBuffer::GetRTTI` (Xbox PDB): the address of the class's `NiRTTI` record.
pub fn ni2_d_buffer_get_rtti(_e: &mut Engine) -> Ptr {
    Ptr::new(NI2_D_BUFFER_RTTI)
}

/// This unit's translated functions, by exe address.
pub fn funcs() -> Vec<(u32, AbiFn)> {
    vec![entry!(0x00a8f1b0, ni2_d_buffer_get_rtti() -> Ptr)]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ni2_d_buffer_get_rtti_returns_the_class_rtti() {
        let mut e = Engine::new();
        assert_eq!(e.call(0x00a8f1b0, &args![]).u32(), NI2_D_BUFFER_RTTI);
    }
}
