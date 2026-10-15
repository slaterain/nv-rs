//! `BSMain/bsxflags.obj` (Xbox PDB source unit), subsystem `BSMain`: its functions in
//! FalloutNV.exe 1.4.0.525, translated (docs/ENGINE_CRATE.md).

#[allow(unused_imports)]
use crate::prelude::*;

/// `NiIntegerExtraData::ms_RTTI` (the class's `NiRTTI` record).
const NI_INTEGER_EXTRA_DATA_RTTI: u32 = 0x011f_5f2c;

// Translated from 00a99c60 (decompiled, FalloutNV.exe 1.4.0.525)
/// `NiIntegerExtraData::GetRTTI` (Xbox PDB): the address of the class's `NiRTTI` record.
pub fn ni_integer_extra_data_get_rtti(_e: &mut Engine) -> Ptr {
    Ptr::new(NI_INTEGER_EXTRA_DATA_RTTI)
}

/// This unit's translated functions, by exe address.
pub fn funcs() -> Vec<(u32, AbiFn)> {
    vec![entry!(0x00a99c60, ni_integer_extra_data_get_rtti() -> Ptr)]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ni_integer_extra_data_get_rtti_returns_the_class_rtti() {
        let mut e = Engine::new();
        assert_eq!(
            e.call(0x00a99c60, &args![]).u32(),
            NI_INTEGER_EXTRA_DATA_RTTI
        );
    }
}
