//! `Havok SDK/run_00dff0e0` (Xbox PDB source unit), subsystem `Havok SDK`: its functions in
//! FalloutNV.exe 1.4.0.525, translated (docs/ENGINE_CRATE.md).

#[allow(unused_imports)]
use crate::prelude::*;

// Translated from 00dff440 (decompiled, FalloutNV.exe 1.4.0.525)
/// Thunk: moves `this` back by `0x8` (`SUB ECX,0x8`) and jumps to `00dff470`, which (`RET 4`) has the shape of a scalar deleting destructor: it reads bit 0 of the one stack word. The caller gets its `EAX`; the class is unconfirmed.
pub fn fn_00dff440(e: &mut Engine, this: Ptr, flags: u32) -> u32 {
    e.call(0x00df_f470, &args![this.addr().wrapping_sub(0x8), flags])
        .u32()
}

// Translated from 00dff450 (decompiled, FalloutNV.exe 1.4.0.525)
/// Thunk: moves `this` back by `0xc` (`SUB ECX,0xc`) and jumps to `00dff470`, which (`RET 4`) has the shape of a scalar deleting destructor: it reads bit 0 of the one stack word. The caller gets its `EAX`; the class is unconfirmed.
pub fn fn_00dff450(e: &mut Engine, this: Ptr, flags: u32) -> u32 {
    e.call(0x00df_f470, &args![this.addr().wrapping_sub(0xc), flags])
        .u32()
}

// Translated from 00dff1a0 (decompiled, FalloutNV.exe 1.4.0.525)
/// No name in the engine map. Builds a temporary object in a 0x40-byte stack buffer by calling `00e45010` on it (flag word 0; the callee pops it) and returns the first word of that buffer, which is what the code reads back after the call. The class is unconfirmed.
pub fn fn_00dff1a0(e: &mut Engine) -> u32 {
    e.with_stack(0x40, |e, buffer| {
        e.call(0x00e4_5010, &args![buffer, 0u32]);
        e.mem.u32(buffer.addr())
    })
}

// Translated from 00dff310 (decompiled, FalloutNV.exe 1.4.0.525)
/// No name in the engine map. Builds a temporary object in a 0x70-byte stack buffer by calling `00e45120` on it (flag word 0; the callee pops it) and returns the first word of that buffer, which is what the code reads back after the call. The class is unconfirmed.
pub fn fn_00dff310(e: &mut Engine) -> u32 {
    e.with_stack(0x70, |e, buffer| {
        e.call(0x00e4_5120, &args![buffer, 0u32]);
        e.mem.u32(buffer.addr())
    })
}

// Translated from 00dff4f0 (decompiled, FalloutNV.exe 1.4.0.525)
/// No name in the engine map. Builds a temporary object in a 0x60-byte stack buffer by calling `00e46300` on it (flag word 0; the callee pops it) and returns the first word of that buffer, which is what the code reads back after the call. The class is unconfirmed.
pub fn fn_00dff4f0(e: &mut Engine) -> u32 {
    e.with_stack(0x60, |e, buffer| {
        e.call(0x00e4_6300, &args![buffer, 0u32]);
        e.mem.u32(buffer.addr())
    })
}

// Translated from 00dff550 (decompiled, FalloutNV.exe 1.4.0.525)
/// No name in the engine map. Builds a temporary object in a 0x20-byte stack buffer by calling `00e466a0` on it (flag word 0; the callee pops it) and returns the first word of that buffer, which is what the code reads back after the call. The class is unconfirmed.
pub fn fn_00dff550(e: &mut Engine) -> u32 {
    e.with_stack(0x20, |e, buffer| {
        e.call(0x00e4_66a0, &args![buffer, 0u32]);
        e.mem.u32(buffer.addr())
    })
}

/// This unit's translated functions, by exe address.
pub fn funcs() -> Vec<(u32, AbiFn)> {
    vec![
        entry!(0x00df_f1a0, fn_00dff1a0() -> u32),
        entry!(0x00df_f310, fn_00dff310() -> u32),
        entry!(0x00df_f4f0, fn_00dff4f0() -> u32),
        entry!(0x00df_f550, fn_00dff550() -> u32),
        entry!(0x00df_f440, fn_00dff440(Ptr, u32) -> u32),
        entry!(0x00df_f450, fn_00dff450(Ptr, u32) -> u32),
    ]
}

#[cfg(test)]
mod a07_thunk_tests {
    use crate::prelude::*;

    /// Calls `thunk` with `this = 0x2000` and one flags word, with a double
    /// over `target` that returns `0xdeadbeef`; checks that the double ran
    /// once, with `this` moved back by `back` and the flags passed on, and
    /// returns the thunk's `EAX`.
    fn check(thunk: u32, target: u32, back: u32) -> u32 {
        let mut e = Engine::new();
        e.register(target, |_, _| Ret {
            eax: 0xdead_beef,
            ..Ret::default()
        });
        e.call_log = Some(vec![]);
        let result = e.call(thunk, &[0x2000u32, 0x1]);
        let log = e.call_log.take().unwrap();
        let calls: Vec<_> = log.iter().filter(|(a, _)| *a == target).collect();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].1, vec![0x2000 - back, 0x1]);
        result.u32()
    }

    #[test]
    fn thunk_00dff440() {
        assert_eq!(check(0x00df_f440, 0x00df_f470, 0x8), 0xdead_beef);
    }

    #[test]
    fn thunk_00dff450() {
        assert_eq!(check(0x00df_f450, 0x00df_f470, 0xc), 0xdead_beef);
    }
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
        fn_00dff1a0_returns_first_word_of_built_object,
        0x00df_f1a0,
        0x00e4_5010
    );
    first_word_test!(
        fn_00dff310_returns_first_word_of_built_object,
        0x00df_f310,
        0x00e4_5120
    );
    first_word_test!(
        fn_00dff4f0_returns_first_word_of_built_object,
        0x00df_f4f0,
        0x00e4_6300
    );
    first_word_test!(
        fn_00dff550_returns_first_word_of_built_object,
        0x00df_f550,
        0x00e4_66a0
    );
}
