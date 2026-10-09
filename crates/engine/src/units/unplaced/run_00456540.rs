//! `(unplaced)/run_00456540` (Xbox PDB source unit), subsystem `(unplaced)`: its functions in
//! FalloutNV.exe 1.4.0.525, translated (docs/ENGINE_CRATE.md).

#[allow(unused_imports)]
use crate::prelude::*;

/// Tests the given mask against the flag word at `+0x30` of `this`
/// (`this->flags & mask`, as 0 or 1; `RET 4`).
const TEST_FLAGS: u32 = 0x0045_6630;

// Translated from 00456610 (decompiled, FalloutNV.exe 1.4.0.525)
/// Tests flag bit 0: `00456630` with the mask 1. The result stays in `EAX`
/// (0 or 1) although the decompiler shows no return value.
pub fn fn_00456610(e: &mut Engine, this: Ptr) -> bool {
    e.call(TEST_FLAGS, &args![this, 1u32]).bool()
}

/// This unit's translated functions, by exe address.
pub fn funcs() -> Vec<(u32, AbiFn)> {
    vec![entry!(0x00456610, fn_00456610(Ptr) -> bool)]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn asks_the_flag_test_for_mask_one() {
        let mut e = Engine::new();
        e.register(TEST_FLAGS, |_, a| Ret {
            eax: a[1],
            ..Ret::default()
        });
        e.call_log = Some(vec![]);
        assert!(e.call(0x0045_6610, &args![0x1234u32]).bool());
        let log = e.call_log.take().unwrap();
        assert_eq!(log[1], (TEST_FLAGS, vec![0x1234, 1]));

        e.register(TEST_FLAGS, |_, _| Ret::default());
        assert!(!e.call(0x0045_6610, &args![0x1234u32]).bool());
    }
}
