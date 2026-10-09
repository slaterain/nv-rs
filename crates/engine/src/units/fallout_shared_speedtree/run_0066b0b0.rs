//! `fallout shared/speedtree/run_0066b0b0` (Xbox PDB source unit), subsystem `fallout shared/speedtree`: its functions in
//! FalloutNV.exe 1.4.0.525, translated (docs/ENGINE_CRATE.md).

#[allow(unused_imports)]
use crate::prelude::*;

/// Drops a reference to a counted object (`00401970`: decrements the count at
/// `+4`, deletes through vtable slot `+4` at zero).
const RELEASE_REFERENCE: u32 = 0x0040_1970;
/// Adds a reference to a counted object (`0040f6e0`: increments the count at
/// `+4`).
const ADD_REFERENCE: u32 = 0x0040_f6e0;

// Translated from 0066b0d0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Smart-pointer assignment from a raw pointer (`__thiscall`, `RET 4`): when
/// `pointer` differs from the held one, releases the held object (if any),
/// stores `pointer` at `+0` and adds a reference to it (if not null).
/// Returns `this`.
pub fn fn_0066b0d0(e: &mut Engine, this: Ptr, pointer: Ptr) -> Ptr {
    let held = e.mem.u32(this.addr());
    if held != pointer.addr() {
        if held != 0 {
            e.call(RELEASE_REFERENCE, &args![held]);
        }
        e.mem.set_u32(this.addr(), pointer.addr());
        if !pointer.is_null() {
            e.call(ADD_REFERENCE, &args![pointer]);
        }
    }
    this
}

/// This unit's translated functions, by exe address.
pub fn funcs() -> Vec<(u32, AbiFn)> {
    vec![entry!(0x0066b0d0, fn_0066b0d0(Ptr, Ptr) -> Ptr)]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assign(held: u32, pointer: u32) -> (u32, Vec<(u32, Vec<u32>)>) {
        let mut e = Engine::new();
        e.register(RELEASE_REFERENCE, |_, _| Ret::default());
        e.register(ADD_REFERENCE, |_, _| Ret::default());
        let holder: Ptr = Ptr::new(e.mem.alloc(4));
        e.mem.set_u32(holder.addr(), held);
        e.call_log = Some(vec![]);
        let back = e.call(0x0066_b0d0, &args![holder, pointer]);
        assert_eq!(back.ptr::<()>(), holder);
        let log = e.call_log.take().unwrap();
        (e.mem.u32(holder.addr()), log[1..].to_vec())
    }

    #[test]
    fn replaces_the_held_object() {
        let (now, calls) = assign(0x4000, 0x5000);
        assert_eq!(now, 0x5000);
        assert_eq!(
            calls,
            vec![
                (RELEASE_REFERENCE, vec![0x4000]),
                (ADD_REFERENCE, vec![0x5000])
            ]
        );
    }

    #[test]
    fn assigning_the_same_pointer_does_nothing() {
        let (now, calls) = assign(0x4000, 0x4000);
        assert_eq!(now, 0x4000);
        assert!(calls.is_empty());
    }

    #[test]
    fn an_empty_holder_only_adds_and_null_only_releases() {
        let (now, calls) = assign(0, 0x5000);
        assert_eq!((now, calls), (0x5000, vec![(ADD_REFERENCE, vec![0x5000])]));
        let (now, calls) = assign(0x4000, 0);
        assert_eq!((now, calls), (0, vec![(RELEASE_REFERENCE, vec![0x4000])]));
    }
}
