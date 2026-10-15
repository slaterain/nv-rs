//! `(unplaced)/run_00ca9580` (Xbox PDB source unit), subsystem `(unplaced)`: its functions in
//! FalloutNV.exe 1.4.0.525, translated (docs/ENGINE_CRATE.md).

#[allow(unused_imports)]
use crate::prelude::*;

// Translated from 00ca9600 (decompiled, FalloutNV.exe 1.4.0.525)
/// No map name. `__fastcall` getter: the word at `this + 0x10`.
pub fn fn_00ca9600(e: &mut Engine, this: Ptr) -> u32 {
    e.mem.u32(this.addr() + 0x10)
}

/// This unit's translated functions, by exe address.
pub fn funcs() -> Vec<(u32, AbiFn)> {
    vec![entry!(0x00ca9600, fn_00ca9600(Ptr) -> u32)]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fn_00ca9600_reads_the_word_at_0x10() {
        let mut e = Engine::new();
        let this = e.mem.alloc(0x20);
        e.mem.set_u32(this + 0x10, 0xabcd_1234);
        assert_eq!(
            e.call(0x00ca_9600, &args![Ptr::<()>::new(this)]).u32(),
            0xabcd_1234
        );
    }
}
