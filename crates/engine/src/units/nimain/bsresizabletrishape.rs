//! `NiMain/bsresizabletrishape.cpp` (Xbox PDB source unit), subsystem `NiMain`: its functions in
//! FalloutNV.exe 1.4.0.525, translated (docs/ENGINE_CRATE.md).

#[allow(unused_imports)]
use crate::prelude::*;

/// `BSResizableTriShape::ms_RTTI` (the class's `NiRTTI` record).
const BS_RESIZABLE_TRI_SHAPE_RTTI: u32 = 0x011f_5b34;

// Translated from 00a86d20 (decompiled, FalloutNV.exe 1.4.0.525)
/// `BSResizableTriShape::GetRTTI` (Xbox PDB): the address of the class's `NiRTTI` record.
pub fn bs_resizable_tri_shape_get_rtti(_e: &mut Engine) -> Ptr {
    Ptr::new(BS_RESIZABLE_TRI_SHAPE_RTTI)
}

/// This unit's translated functions, by exe address.
pub fn funcs() -> Vec<(u32, AbiFn)> {
    vec![entry!(0x00a86d20, bs_resizable_tri_shape_get_rtti() -> Ptr)]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bs_resizable_tri_shape_get_rtti_returns_the_class_rtti() {
        let mut e = Engine::new();
        assert_eq!(
            e.call(0x00a86d20, &args![]).u32(),
            BS_RESIZABLE_TRI_SHAPE_RTTI
        );
    }
}
