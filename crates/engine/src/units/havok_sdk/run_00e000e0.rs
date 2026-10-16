//! `Havok SDK/run_00e000e0` (Xbox PDB source unit), subsystem `Havok SDK`: its functions in
//! FalloutNV.exe 1.4.0.525, translated (docs/ENGINE_CRATE.md).

#[allow(unused_imports)]
use crate::prelude::*;

// Translated from 00e000e0 (decompiled, FalloutNV.exe 1.4.0.525)
/// No name in the engine map. Builds a temporary object in a 0x60-byte stack buffer by calling `00d50c00` on it (flag word 0; the callee pops it) and returns the first word of that buffer, which is what the code reads back after the call. The class is unconfirmed.
pub fn fn_00e000e0(e: &mut Engine) -> u32 {
    e.with_stack(0x60, |e, buffer| {
        e.call(0x00d5_0c00, &args![buffer, 0u32]);
        e.mem.u32(buffer.addr())
    })
}

// Translated from 00e003b0 (decompiled, FalloutNV.exe 1.4.0.525)
/// No name in the engine map. Builds a temporary object in a 0x170-byte stack buffer by calling `00e003d0` on it (flag word 0; the callee pops it) and returns the first word of that buffer, which is what the code reads back after the call. The class is unconfirmed.
pub fn fn_00e003b0(e: &mut Engine) -> u32 {
    e.with_stack(0x170, |e, buffer| {
        e.call(0x00e0_03d0, &args![buffer, 0u32]);
        e.mem.u32(buffer.addr())
    })
}

// Translated from 00e004d0 (decompiled, FalloutNV.exe 1.4.0.525)
/// No name in the engine map. Builds a temporary object in a 0xe0-byte stack buffer by calling `00e004f0` on it (flag word 0; the callee pops it) and returns the first word of that buffer, which is what the code reads back after the call. The class is unconfirmed.
pub fn fn_00e004d0(e: &mut Engine) -> u32 {
    e.with_stack(0xe0, |e, buffer| {
        e.call(0x00e0_04f0, &args![buffer, 0u32]);
        e.mem.u32(buffer.addr())
    })
}

// Translated from 00e005f0 (decompiled, FalloutNV.exe 1.4.0.525)
/// No name in the engine map. Builds a temporary object in a 0x40-byte stack buffer by calling `00e00610` on it (flag word 0; the callee pops it) and returns the first word of that buffer, which is what the code reads back after the call. The class is unconfirmed.
pub fn fn_00e005f0(e: &mut Engine) -> u32 {
    e.with_stack(0x40, |e, buffer| {
        e.call(0x00e0_0610, &args![buffer, 0u32]);
        e.mem.u32(buffer.addr())
    })
}

// Translated from 00e006b0 (decompiled, FalloutNV.exe 1.4.0.525)
/// No name in the engine map. Builds a temporary object in a 0x30-byte stack buffer by calling `00e006d0` on it (flag word 0; the callee pops it) and returns the first word of that buffer, which is what the code reads back after the call. The class is unconfirmed.
pub fn fn_00e006b0(e: &mut Engine) -> u32 {
    e.with_stack(0x30, |e, buffer| {
        e.call(0x00e0_06d0, &args![buffer, 0u32]);
        e.mem.u32(buffer.addr())
    })
}

/// This unit's translated functions, by exe address.
pub fn funcs() -> Vec<(u32, AbiFn)> {
    vec![
        entry!(0x00e0_00e0, fn_00e000e0() -> u32),
        entry!(0x00e0_03b0, fn_00e003b0() -> u32),
        entry!(0x00e0_04d0, fn_00e004d0() -> u32),
        entry!(0x00e0_05f0, fn_00e005f0() -> u32),
        entry!(0x00e0_06b0, fn_00e006b0() -> u32),
    ]
}

#[cfg(test)]
mod a07_first_word_tests {
    use crate::prelude::*;

    const BUILT_WORD: u32 = 0x0abc_def0;

    /// A double over `callee` writes `BUILT_WORD` at the buffer it is given; `function`
    /// must call it once with that buffer and a zero flag word, and return the word.
    fn check(function: u32, callee: u32) {
        let mut e = Engine::new();
        e.register(callee, |e, a| {
            e.mem.set_u32(a[0], BUILT_WORD);
            Ret::default()
        });
        e.call_log = Some(vec![]);
        let result = e.call(function, &args![]).u32();
        let log = e.call_log.take().unwrap();
        let calls: Vec<_> = log.iter().filter(|(a, _)| *a == callee).collect();
        assert_eq!(calls.len(), 1);
        assert_ne!(calls[0].1[0], 0);
        assert_eq!(calls[0].1[1], 0);
        assert_eq!(result, BUILT_WORD);
    }

    macro_rules! first_word_test {
        ($name:ident, $function:expr, $callee:expr) => {
            #[test]
            fn $name() {
                check($function, $callee);
            }
        };
    }

    first_word_test!(
        fn_00e000e0_returns_first_word_of_built_object,
        0x00e0_00e0,
        0x00d5_0c00
    );
    first_word_test!(
        fn_00e003b0_returns_first_word_of_built_object,
        0x00e0_03b0,
        0x00e0_03d0
    );
    first_word_test!(
        fn_00e004d0_returns_first_word_of_built_object,
        0x00e0_04d0,
        0x00e0_04f0
    );
    first_word_test!(
        fn_00e005f0_returns_first_word_of_built_object,
        0x00e0_05f0,
        0x00e0_0610
    );
    first_word_test!(
        fn_00e006b0_returns_first_word_of_built_object,
        0x00e0_06b0,
        0x00e0_06d0
    );
}
