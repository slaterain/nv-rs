//! `NiSystem/run_00aa2ff0` (Xbox PDB source unit), subsystem `NiSystem`: its functions in
//! FalloutNV.exe 1.4.0.525, translated (docs/ENGINE_CRATE.md).

#[allow(unused_imports)]
use crate::prelude::*;

/// `00aa2710`: the jump target of `00aa3040`.
const JUMP_TARGET_00AA3040: u32 = 0x00aa_2710;

// Translated from 00aa3040 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed in the engine map: a 5-byte jump to `00aa2710`, passing on `this`; returns the target result word. Return type (u32): the target `00aa2710` returns EAX at every return (-1, or the result of an imported call); its meaning is unconfirmed.
pub fn fn_00aa3040(e: &mut Engine, this: Ptr) -> u32 {
    e.call(JUMP_TARGET_00AA3040, &args![this]).u32()
}

/// This unit's translated functions, by exe address.
pub fn funcs() -> Vec<(u32, AbiFn)> {
    vec![entry!(0x00aa_3040, fn_00aa3040(Ptr) -> u32)]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn thunk_00aa3040_forwards_this_and_returns_the_target_result() {
        const THUNK: u32 = 0x00aa_3040;
        const TARGET: u32 = 0x00aa_2710;
        let mut e = Engine::new();
        e.register(TARGET, |_, _| Ret {
            eax: 0xffff_ffff,
            ..Ret::default()
        });
        e.call_log = Some(vec![]);
        assert_eq!(e.call(THUNK, &args![0x1000u32]).u32(), 0xffff_ffff);
        let log = e.call_log.take().unwrap();
        let hits: Vec<_> = log.iter().filter(|call| call.0 == TARGET).collect();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].1, args![0x1000u32]);
    }
}
