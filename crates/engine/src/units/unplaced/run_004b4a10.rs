//! `(unplaced)/run_004b4a10` (Xbox PDB source unit), subsystem `(unplaced)`: its functions in
//! FalloutNV.exe 1.4.0.525, translated (docs/ENGINE_CRATE.md).

#[allow(unused_imports)]
use crate::prelude::*;

/// `004b4ad0`: thiscall with two stack words (the body keeps the stack
/// 16-byte aligned itself and returns nothing).
const IMPLEMENTATION: u32 = 0x004b_4ad0;

// Translated from 004b4ab0 (decompiled, FalloutNV.exe 1.4.0.525)
/// No map name. Thiscall wrapper (`RET 8`): forwards `this` and both stack
/// words, in order, to `004b4ad0`.
pub fn fn_004b4ab0(e: &mut Engine, this: Ptr, first: u32, second: u32) {
    e.call(IMPLEMENTATION, &args![this, first, second]);
}

/// This unit's translated functions, by exe address.
pub fn funcs() -> Vec<(u32, AbiFn)> {
    vec![entry!(0x004b4ab0, fn_004b4ab0(Ptr, u32, u32))]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn forwards_this_and_both_words() {
        let mut e = Engine::new();
        e.register(IMPLEMENTATION, |_, _| Ret::default());
        e.call_log = Some(vec![]);
        e.call(0x004b_4ab0, &args![Ptr::<()>::new(0x100), 7u32, 9u32]);
        let log = e.call_log.take().unwrap();
        let forwarded = log
            .iter()
            .find(|(addr, _)| *addr == IMPLEMENTATION)
            .unwrap();
        assert_eq!(forwarded.1, vec![0x100, 7, 9]);
    }
}
