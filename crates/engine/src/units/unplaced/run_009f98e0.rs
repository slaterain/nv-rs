//! `(unplaced)/run_009f98e0` (Xbox PDB source unit), subsystem `(unplaced)`: its functions in
//! FalloutNV.exe 1.4.0.525, translated (docs/ENGINE_CRATE.md).

#[allow(unused_imports)]
use crate::prelude::*;

/// `009f9c80`: the jump target of `009f9d50`.
const JUMP_TARGET_009F9D50: u32 = 0x009f_9c80;

// Translated from 009f9d50 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed in the engine map: a 5-byte jump to `009f9c80`, passing on `this` and its one argument; returns the target result word. Return type (u32): the target `009f9c80` returns EAX at both returns (0, or `0x8007000e`); its meaning is unconfirmed.
pub fn fn_009f9d50(e: &mut Engine, this: Ptr, arg: Ptr) -> u32 {
    e.call(JUMP_TARGET_009F9D50, &args![this, arg]).u32()
}

/// This unit's translated functions, by exe address.
pub fn funcs() -> Vec<(u32, AbiFn)> {
    vec![entry!(0x009f_9d50, fn_009f9d50(Ptr, Ptr) -> u32)]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn thunk_009f9d50_forwards_its_words_and_returns_the_target_result() {
        const THUNK: u32 = 0x009f_9d50;
        const TARGET: u32 = 0x009f_9c80;
        let mut e = Engine::new();
        e.register(TARGET, |_, _| Ret {
            eax: 0x8007_000e,
            ..Ret::default()
        });
        e.call_log = Some(vec![]);
        assert_eq!(
            e.call(THUNK, &args![0x1000u32, 0x2000u32]).u32(),
            0x8007_000e
        );
        let log = e.call_log.take().unwrap();
        let hits: Vec<_> = log.iter().filter(|call| call.0 == TARGET).collect();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].1, args![0x1000u32, 0x2000u32]);
    }
}
