//! `fallout shared/run_00488f10` (Xbox PDB source unit), subsystem `fallout shared`: its functions in
//! FalloutNV.exe 1.4.0.525, translated (docs/ENGINE_CRATE.md).

#[allow(unused_imports)]
use crate::prelude::*;

// Translated from 00488ff0 (decompiled, FalloutNV.exe 1.4.0.525)
/// A forwarder: it hands `this` (`ECX`) to `00489620`, which frees the block held at `+4`
/// (through `00401030`) when it is non-null, zeroes `+4`, clears the byte at `+0` and
/// answers whether a block was freed, and returns its result (a `bool`). Parameters: one
/// word, `this`; the disassembly of the function ends in a plain `RET` and passes only `ECX`
/// on. The return type (`bool`, AL) is from `00489620`'s disassembly, which ends `MOV
/// AL,[EBP-1]` (the flag it sets to 1 only on the freeing path); the wrapper leaves AL as it
/// came back.
pub fn fn_00488ff0(e: &mut Engine, this: Ptr) -> bool {
    e.call(0x0048_9620, &args![this]).bool()
}

/// This unit's translated functions, by exe address.
pub fn funcs() -> Vec<(u32, AbiFn)> {
    vec![entry!(0x0048_8ff0, fn_00488ff0(Ptr) -> bool)]
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
    fn fwd_a03_00488ff0_returns_the_callees_al() {
        let mut e = Engine::new();
        e.register(0x0048_9620, |_, _| Ret {
            eax: 0x1234_5601,
            ..Ret::default()
        });
        e.call_log = Some(vec![]);
        assert!(e.call(0x0048_8ff0, &args![0x0000_5000u32]).bool());
        assert_eq!(calls_to(&e, 0x0048_9620), vec![vec![0x5000]]);
        // Only AL counts: a non-zero upper part with AL clear is false.
        e.register(0x0048_9620, |_, _| Ret {
            eax: 0x1234_5600,
            ..Ret::default()
        });
        assert!(!e.call(0x0048_8ff0, &args![0x0000_5000u32]).bool());
    }
}
