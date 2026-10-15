//! `fallout/magic/run_008229d0` (Xbox PDB source unit), subsystem `fallout/magic`: its functions in
//! FalloutNV.exe 1.4.0.525, translated (docs/ENGINE_CRATE.md).

#[allow(unused_imports)]
use crate::prelude::*;

// Translated from 00822a00 (decompiled, FalloutNV.exe 1.4.0.525)
/// A forwarder: it hands `this` (`ECX`) to `0081de70` (Ghidra name
/// `MagicProjectile::~MagicProjectile`), a destructor with an exception frame, and returns
/// nothing. Parameters: one word, `this`; the disassembly of the function ends in a plain
/// `RET` and passes only `ECX` on. `0081de70`'s disassembly ends with its exception-frame
/// unlink and returns no value (Ghidra types it void).
pub fn fn_00822a00(e: &mut Engine, this: Ptr) {
    e.call(0x0081_de70, &args![this]);
}

/// This unit's translated functions, by exe address.
pub fn funcs() -> Vec<(u32, AbiFn)> {
    vec![entry!(0x0082_2a00, fn_00822a00(Ptr))]
}

#[cfg(test)]
mod thunk_tests {
    use super::*;

    fn calls_to(e: &Engine, callee: u32) -> Vec<Vec<u32>> {
        e.call_log
            .as_ref()
            .unwrap()
            .iter()
            .filter(|(address, _)| *address == callee)
            .map(|(_, words)| words.clone())
            .collect()
    }

    #[test]
    fn fwd_a03_00822a00_passes_this_to_the_callee() {
        let mut e = Engine::new();
        e.register(0x0081_de70, |_, _| Ret::default());
        e.call_log = Some(vec![]);
        e.call(0x0082_2a00, &args![0x0000_5000u32]);
        assert_eq!(calls_to(&e, 0x0081_de70), vec![vec![0x5000]]);
    }
}
