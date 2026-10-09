//! `fallout shared/run_0044d8c0` (Xbox PDB source unit), subsystem `fallout shared`: its functions in
//! FalloutNV.exe 1.4.0.525, translated (docs/ENGINE_CRATE.md).

#[allow(unused_imports)]
use crate::prelude::*;

// Translated from 0044ddc0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Returns the word at `+8` of `this` (a plain field getter; `0040fae0` follows it from an element to the next one).
pub fn fn_0044ddc0(e: &mut Engine, this: Ptr) -> u32 {
    e.mem.u32(this.addr() + 8)
}

/// This unit's translated functions, by exe address.
pub fn funcs() -> Vec<(u32, AbiFn)> {
    vec![entry!(0x0044ddc0, fn_0044ddc0(Ptr) -> u32)]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_field_at_offset_8() {
        let mut e = Engine::new();
        let object: Ptr = Ptr::new(e.mem.alloc(0x50));
        e.mem.set_u32(object.addr() + 8, 0x1234_5678);
        assert_eq!(e.call(0x0044_ddc0, &args![object]).u32(), 0x1234_5678);
    }
}
