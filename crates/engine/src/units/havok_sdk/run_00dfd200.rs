//! `Havok SDK/run_00dfd200` (Xbox PDB source unit), subsystem `Havok SDK`: its functions in
//! FalloutNV.exe 1.4.0.525, translated (docs/ENGINE_CRATE.md).

#[allow(unused_imports)]
use crate::prelude::*;

// Translated from 00dfd200 (decompiled, FalloutNV.exe 1.4.0.525)
/// No name in the engine map. Builds a temporary object in a 0x70-byte stack buffer by calling `00cf56f0` on it (flag word 0; the callee pops it) and returns the first word of that buffer, which is what the code reads back after the call. The class is unconfirmed.
pub fn fn_00dfd200(e: &mut Engine) -> u32 {
    e.with_stack(0x70, |e, buffer| {
        e.call(0x00cf_56f0, &args![buffer, 0u32]);
        e.mem.u32(buffer.addr())
    })
}

// Translated from 00dfd280 (decompiled, FalloutNV.exe 1.4.0.525)
/// No name in the engine map. Builds a temporary object in a 0x70-byte stack buffer by calling `00d59830` on it (flag word 0; the callee pops it) and returns the first word of that buffer, which is what the code reads back after the call. The class is unconfirmed.
pub fn fn_00dfd280(e: &mut Engine) -> u32 {
    e.with_stack(0x70, |e, buffer| {
        e.call(0x00d5_9830, &args![buffer, 0u32]);
        e.mem.u32(buffer.addr())
    })
}

// Translated from 00dfd320 (decompiled, FalloutNV.exe 1.4.0.525)
/// No name in the engine map. Builds a temporary object in a 0xf0-byte stack buffer by calling `00e3dbd0` on it (flag word 0; the callee pops it) and returns the first word of that buffer, which is what the code reads back after the call. The class is unconfirmed.
pub fn fn_00dfd320(e: &mut Engine) -> u32 {
    e.with_stack(0xf0, |e, buffer| {
        e.call(0x00e3_dbd0, &args![buffer, 0u32]);
        e.mem.u32(buffer.addr())
    })
}

// Translated from 00dfd630 (decompiled, FalloutNV.exe 1.4.0.525)
/// No name in the engine map. Builds a temporary object in a 0x70-byte stack buffer by calling `00e3e380` on it (flag word 0; the callee pops it) and returns the first word of that buffer, which is what the code reads back after the call. The class is unconfirmed.
pub fn fn_00dfd630(e: &mut Engine) -> u32 {
    e.with_stack(0x70, |e, buffer| {
        e.call(0x00e3_e380, &args![buffer, 0u32]);
        e.mem.u32(buffer.addr())
    })
}

/// This unit's translated functions, by exe address.
pub fn funcs() -> Vec<(u32, AbiFn)> {
    vec![
        entry!(0x00df_d200, fn_00dfd200() -> u32),
        entry!(0x00df_d280, fn_00dfd280() -> u32),
        entry!(0x00df_d320, fn_00dfd320() -> u32),
        entry!(0x00df_d630, fn_00dfd630() -> u32),
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
        fn_00dfd200_returns_first_word_of_built_object,
        0x00df_d200,
        0x00cf_56f0
    );
    first_word_test!(
        fn_00dfd280_returns_first_word_of_built_object,
        0x00df_d280,
        0x00d5_9830
    );
    first_word_test!(
        fn_00dfd320_returns_first_word_of_built_object,
        0x00df_d320,
        0x00e3_dbd0
    );
    first_word_test!(
        fn_00dfd630_returns_first_word_of_built_object,
        0x00df_d630,
        0x00e3_e380
    );
}
