//! `fallout shared/tesmodeltextureswap.cpp` (Xbox PDB source unit), subsystem `fallout shared`: its functions in
//! FalloutNV.exe 1.4.0.525, translated (docs/ENGINE_CRATE.md).

#[allow(unused_imports)]
use crate::prelude::*;

// Translated from 0048a470 (decompiled, FalloutNV.exe 1.4.0.525)
/// A forwarder: it hands `this` (`ECX`) to `00489040` (Ghidra name
/// `TESModel::InitializeDataComponent`), which tests `00559450` and clears the byte at
/// `+0x14`. The Xbox PDB name `TESModelTextureSwap::InitializeDataComponent` is the class's
/// vslot +0x000 (pdbdump `type ... TESModelTextureSwap`; the engine map row is `vt
/// 822daa78`), and returns nothing. Parameters: one word, `this`; the disassembly of the
/// function ends in a plain `RET` and passes only `ECX` on. `00489040`'s disassembly ends on
/// the byte store `MOV byte ptr [EAX+0x14],0x0` with no result computed (Ghidra types it
/// void).
pub fn tes_model_texture_swap_initialize_data_component(e: &mut Engine, this: Ptr) {
    e.call(0x0048_9040, &args![this]);
}

/// This unit's translated functions, by exe address.
pub fn funcs() -> Vec<(u32, AbiFn)> {
    vec![entry!(
        0x0048_a470,
        tes_model_texture_swap_initialize_data_component(Ptr)
    )]
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
    fn fwd_a03_0048a470_passes_this_to_the_callee() {
        let mut e = Engine::new();
        e.register(0x0048_9040, |_, _| Ret::default());
        e.call_log = Some(vec![]);
        e.call(0x0048_a470, &args![0x0000_5000u32]);
        assert_eq!(calls_to(&e, 0x0048_9040), vec![vec![0x5000]]);
    }
}
