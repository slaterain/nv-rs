//! `NiParticle/niparticlesystem.cpp` (Xbox PDB source unit), subsystem `NiParticle`: its functions in
//! FalloutNV.exe 1.4.0.525, translated (docs/ENGINE_CRATE.md).

#[allow(unused_imports)]
use crate::prelude::*;

/// `NiParticleSystem::ms_RTTI` (the class's `NiRTTI` record).
const NI_PARTICLE_SYSTEM_RTTI: u32 = 0x0120_24e0;

// Translated from 00c1b5b0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `NiParticleSystem::GetRTTI` (Xbox PDB): the address of the class's `NiRTTI` record.
pub fn ni_particle_system_get_rtti(_e: &mut Engine) -> Ptr {
    Ptr::new(NI_PARTICLE_SYSTEM_RTTI)
}

/// This unit's translated functions, by exe address.
pub fn funcs() -> Vec<(u32, AbiFn)> {
    vec![entry!(0x00c1b5b0, ni_particle_system_get_rtti() -> Ptr)]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ni_particle_system_get_rtti_returns_the_class_rtti() {
        let mut e = Engine::new();
        assert_eq!(e.call(0x00c1b5b0, &args![]).u32(), NI_PARTICLE_SYSTEM_RTTI);
    }
}
