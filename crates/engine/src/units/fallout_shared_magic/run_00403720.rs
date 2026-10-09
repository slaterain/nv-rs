//! `fallout shared/magic/run_00403720` (Xbox PDB source unit), subsystem `fallout shared/magic`: its functions in
//! FalloutNV.exe 1.4.0.525, translated (docs/ENGINE_CRATE.md).

#[allow(unused_imports)]
use crate::prelude::*;

/// The string object's initializer (`00403970`): zeroes the word at `+0` and
/// then calls `004039c0` and `00403a00` with 0.
const INITIALIZE: u32 = 0x0040_3970;
/// The string object's text setter (`004037f0`, `RET 8`): two arguments, the
/// destructor passes (0, 0).
const SET_TEXT: u32 = 0x0040_37f0;

// Translated from 004037b0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Constructor of the string object (see `00406f60`, `Calendar::GetDateString`
/// in `00867970`): initializes it (`00403970`) and returns `this`.
pub fn fn_004037b0(e: &mut Engine, this: Ptr) -> Ptr {
    e.call(INITIALIZE, &args![this]);
    this
}

// Translated from 004037d0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Destructor of the string object: clears its text (`004037f0` with (0, 0)).
/// The decompiler's library name for this address is a false match.
pub fn fn_004037d0(e: &mut Engine, this: Ptr) {
    e.call(SET_TEXT, &args![this, 0u32, 0u32]);
}

/// This unit's translated functions, by exe address.
pub fn funcs() -> Vec<(u32, AbiFn)> {
    vec![
        entry!(0x004037b0, fn_004037b0(Ptr) -> Ptr),
        entry!(0x004037d0, fn_004037d0(Ptr)),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constructor_initializes_and_returns_this() {
        let mut e = Engine::new();
        e.register(INITIALIZE, |_, _| Ret::default());
        e.call_log = Some(vec![]);
        let back = e.call(0x0040_37b0, &args![0x1000u32]).u32();
        assert_eq!(back, 0x1000);
        let log = e.call_log.take().unwrap();
        assert_eq!(log[1], (INITIALIZE, vec![0x1000]));
    }

    #[test]
    fn destructor_sets_empty_text() {
        let mut e = Engine::new();
        e.register(SET_TEXT, |_, _| Ret::default());
        e.call_log = Some(vec![]);
        e.call(0x0040_37d0, &args![0x1000u32]);
        let log = e.call_log.take().unwrap();
        assert_eq!(log[1], (SET_TEXT, vec![0x1000, 0, 0]));
    }
}
