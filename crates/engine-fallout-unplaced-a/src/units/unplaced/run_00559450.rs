//! `(unplaced)/run_00559450` (Xbox PDB source unit), subsystem `(unplaced)`: its functions in
//! FalloutNV.exe 1.4.0.525, translated (docs/ENGINE_CRATE.md).

#[allow(unused_imports)]
use crate::prelude::*;

// Translated from 00559450 (decompiled, FalloutNV.exe 1.4.0.525)
/// Returns the word at `+0` of `this`: the first element of a list-like container (`0049c680` and `008c7a30` start their walks from it).
/// A null `this` answers 0 with JIP LN NVSE's game patches in (its null check here, G01:
/// [`engine_core::typed::leaf::word_at_0_of_null`]), else the game reads address 0.
pub fn fn_00559450(e: &mut Engine, this: Ptr) -> u32 {
    if this.addr() == 0 {
        return engine_core::typed::leaf::word_at_0_of_null(e);
    }
    e.mem.u32(this.addr())
}

/// This unit's translated functions, by exe address.
pub fn funcs() -> Vec<(u32, AbiFn)> {
    vec![entry!(0x00559450, fn_00559450(Ptr) -> u32)]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_field_at_offset_zero() {
        let mut e = Engine::new();
        let object: Ptr = Ptr::new(e.mem.alloc(0x50));
        e.mem.set_u32(object.addr(), 0x1234_5678);
        assert_eq!(e.call(0x0055_9450, &args![object]).u32(), 0x1234_5678);
    }

    /// A null `this`: the game reads address 0; with JIP LN NVSE's game patches in (G01), 0.
    #[test]
    fn a_null_this_answers_0_with_jips_fixes() {
        use engine_core::extend::jip_patches::{FIXES, INSTALLED};
        let mut e = Engine::new();
        e.map(0, 0x1000);
        e.mem.set_u32(0, 0x77);
        assert_eq!(e.call(0x0055_9450, &args![0u32]).u32(), 0x77);
        e.set_jip_patches(INSTALLED | FIXES);
        assert_eq!(e.call(0x0055_9450, &args![0u32]).u32(), 0);
        let object: Ptr = Ptr::new(e.mem.alloc(8));
        e.mem.set_u32(object.addr(), 9);
        assert_eq!(e.call(0x0055_9450, &args![object]).u32(), 9);
    }
}
