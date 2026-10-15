//! `NiMain/niparticlemeshesdata.cpp` (Xbox PDB source unit), subsystem `NiMain`: its functions in
//! FalloutNV.exe 1.4.0.525, translated (docs/ENGINE_CRATE.md).

#[allow(unused_imports)]
use crate::prelude::*;

/// `NiParticleMeshesData::ms_RTTI` (the class's `NiRTTI` record).
const NI_PARTICLE_MESHES_DATA_RTTI: u32 = 0x011f_5f0c;

// Translated from 00a98b00 (decompiled, FalloutNV.exe 1.4.0.525)
/// `NiParticleMeshesData::GetRTTI` (Xbox PDB): the address of the class's `NiRTTI` record.
pub fn ni_particle_meshes_data_get_rtti(_e: &mut Engine) -> Ptr {
    Ptr::new(NI_PARTICLE_MESHES_DATA_RTTI)
}

/// This unit's translated functions, by exe address.
pub fn funcs() -> Vec<(u32, AbiFn)> {
    vec![entry!(0x00a98b00, ni_particle_meshes_data_get_rtti() -> Ptr)]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ni_particle_meshes_data_get_rtti_returns_the_class_rtti() {
        let mut e = Engine::new();
        assert_eq!(
            e.call(0x00a98b00, &args![]).u32(),
            NI_PARTICLE_MESHES_DATA_RTTI
        );
    }
}
