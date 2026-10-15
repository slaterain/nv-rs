//! `fallout shared/run_005128e0` (Xbox PDB source unit), subsystem `fallout shared`: its functions in
//! FalloutNV.exe 1.4.0.525, translated (docs/ENGINE_CRATE.md).

#[allow(unused_imports)]
use crate::prelude::*;

// Translated from 005129c0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Thunk: moves `this` back by 0x58 (`SUB ECX,0x58`) and jumps to `00616640`. `00616640` (`RET 0x8`) takes two stack words and leaves no deliberate value in `EAX` (its decompiled form is `void`), so the thunk returns nothing; the parameter count is from the `RET`. The stack words are passed on unchanged.
pub fn fn_005129c0(e: &mut Engine, this: Ptr, first: u32, second: u32) {
    e.call(
        0x0061_6640,
        &args![this.addr().wrapping_sub(0x58), first, second],
    );
}

// Translated from 005129d0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Thunk: moves `this` back by 0x58 (`SUB ECX,0x58`) and jumps to `0059adb0`. `0059adb0` (`RET 0x4`) takes one stack word and leaves no deliberate value in `EAX` (its decompiled form is `void`), so the thunk returns nothing; the parameter count is from the `RET`. The stack word is passed on unchanged.
pub fn fn_005129d0(e: &mut Engine, this: Ptr, argument: u32) {
    e.call(
        0x0059_adb0,
        &args![this.addr().wrapping_sub(0x58), argument],
    );
}

// Translated from 005129e0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Thunk: moves `this` back by 0x58 (`SUB ECX,0x58`) and jumps to `005a7770`. `005a7770` (`RET 0x4`) returns a 16-bit value in `AX` (it ends `MOV AX,word ptr [...]`), so the thunk returns `u16`; the parameter count is from the `RET`. The stack word is passed on unchanged.
pub fn fn_005129e0(e: &mut Engine, this: Ptr, argument: u32) -> u16 {
    e.call(
        0x005a_7770,
        &args![this.addr().wrapping_sub(0x58), argument],
    )
    .u16()
}

/// This unit's translated functions, by exe address.
pub fn funcs() -> Vec<(u32, AbiFn)> {
    vec![
        entry!(0x0051_29c0, fn_005129c0(Ptr, u32, u32)),
        entry!(0x0051_29d0, fn_005129d0(Ptr, u32)),
        entry!(0x0051_29e0, fn_005129e0(Ptr, u32) -> u16),
    ]
}

#[cfg(test)]
mod le81_thunk_tests {
    use crate::prelude::*;

    /// Calls `thunk` with `this = 0x2000` and `extra` stack words, with a
    /// double over `target` that returns `0xdeadbeef`; checks that the
    /// double ran once, with `this` moved back by `back` and `extra` passed on.
    fn check(thunk: u32, target: u32, back: u32, extra: &[u32]) -> Ret {
        let mut e = Engine::new();
        e.register(target, |_, _| Ret {
            eax: 0xdead_beef,
            ..Ret::default()
        });
        e.call_log = Some(vec![]);
        let mut words = vec![0x2000u32];
        words.extend_from_slice(extra);
        let result = e.call(thunk, &words);
        let log = e.call_log.take().unwrap();
        let mut expected = vec![0x2000 - back];
        expected.extend_from_slice(extra);
        let calls: Vec<_> = log.iter().filter(|(a, _)| *a == target).collect();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].1, expected);
        result
    }

    #[test]
    fn le81_thunk_005129c0() {
        check(0x0051_29c0, 0x0061_6640, 0x58, &[0x77, 0x88]);
    }

    #[test]
    fn le81_thunk_005129d0() {
        check(0x0051_29d0, 0x0059_adb0, 0x58, &[0x77]);
    }

    #[test]
    fn le81_thunk_005129e0() {
        assert_eq!(check(0x0051_29e0, 0x005a_7770, 0x58, &[0x77]).u16(), 0xbeef);
    }
}
