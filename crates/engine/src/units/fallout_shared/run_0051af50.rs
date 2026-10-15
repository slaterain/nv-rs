//! `fallout shared/run_0051af50` (Xbox PDB source unit), subsystem `fallout shared`: its functions in
//! FalloutNV.exe 1.4.0.525, translated (docs/ENGINE_CRATE.md).

#[allow(unused_imports)]
use crate::prelude::*;

// Translated from 0051b140 (decompiled, FalloutNV.exe 1.4.0.525)
// Translation incomplete: 0051b140 - the exception-unwinding frame (FS:[0]) and the security-cookie bookkeeping are not translated.
/// Constructor of the form `CreateFormOfType` (`00465110`) makes for form type 0x20 (0x58 bytes); `ECX` is the object.
/// The engine map gives it no name (a `range-sub` function), so the class is unconfirmed.
/// It runs the base and component constructors by address, stores the vtable pointers and sets the fields to their start values.
/// The exception-unwinding frame is not translated.
pub fn fn_0051b140(e: &mut Engine, this: Ptr) -> Ptr {
    let base = this.addr();
    e.call(0x0050_e820, &args![this]);
    e.call(0x0048_a3d0, &args![base + 0x30]);
    e.mem.set_u32(base, 0x0102_ba2c);
    e.mem.set_u32(base + 0x30, 0x0102_ba04);
    e.call(0x004f_15a0, &args![this, 0x20]);
    e.mem.set_u8(base + 0x50, 0xff);
    e.mem.set_u32(base + 0x54, 0x0);
    this
}

// Translated from 0051b130 (decompiled, FalloutNV.exe 1.4.0.525)
/// Thunk: moves `this` back by 0x3c (`SUB ECX,0x3c`) and jumps to `0051a810`. `0051a810` (`RET 4`) has the shape of a deleting destructor: it tests bit 0 of the flags word and returns the object pointer in `EAX`. The return type is `u32` from that `EAX`; the parameter count is from the `RET 4`. The stack word is passed on unchanged.
pub fn fn_0051b130(e: &mut Engine, this: Ptr, flags: u32) -> u32 {
    e.call(0x0051_a810, &args![this.addr().wrapping_sub(0x3c), flags])
        .u32()
}

/// This unit's translated functions, by exe address.
pub fn funcs() -> Vec<(u32, AbiFn)> {
    vec![
        entry!(0x0051_b140, fn_0051b140(Ptr) -> Ptr),
        entry!(0x0051_b130, fn_0051b130(Ptr, u32) -> u32),
    ]
}

#[cfg(test)]
mod tests {
    use crate::units::crt::ctor_test::{self, A};

    #[test]
    fn fn_0051b140_builds_the_object() {
        ctor_test::run(&ctor_test::Case {
            addr: 0x0051_b140,
            size: 0x58,
            globals: &[],
            returns: &[],
            calls: &[
                (0x0050_e820, &[A::T(0x0)]),
                (0x0048_a3d0, &[A::T(0x30)]),
                (0x004f_15a0, &[A::T(0x0), A::V(0x20)]),
            ],
            stores: &[
                (0x0, 4, 0x0102_ba2c),
                (0x30, 4, 0x0102_ba04),
                (0x50, 1, 0xff),
                (0x54, 4, 0x0),
            ],
        });
    }
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
    fn le81_thunk_0051b130() {
        assert_eq!(
            check(0x0051_b130, 0x0051_a810, 0x3c, &[0x1]).u32(),
            0xdead_beef
        );
    }
}
