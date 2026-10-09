//! `(unplaced)/run_0045cb90` (Xbox PDB source unit), subsystem `(unplaced)`: its functions in
//! FalloutNV.exe 1.4.0.525, translated (docs/ENGINE_CRATE.md).

#[allow(unused_imports)]
use crate::prelude::*;

/// Drops one reference of a counted object (`NiRefObject`: the count is at
/// `+4`, the object deletes itself through vtable slot `+4` at zero).
const RELEASE_REFERENCE: u32 = 0x0040_1970;

// Translated from 0045cd60 (decompiled, FalloutNV.exe 1.4.0.525)
/// Returns the word at `+0x28` of `this` (a plain field getter).
pub fn fn_0045cd60(e: &mut Engine, this: Ptr) -> u32 {
    e.mem.u32(this.addr() + 0x28)
}

// Translated from 0045cec0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Smart-pointer destructor: if the pointer held at `+0` of `this` is not
/// null, drops its reference. (The decompiler's library name for this address
/// is a false match.)
pub fn fn_0045cec0(e: &mut Engine, this: Ptr) {
    let held = e.mem.u32(this.addr());
    if held != 0 {
        e.call(RELEASE_REFERENCE, &args![held]);
    }
}

/// This unit's translated functions, by exe address.
pub fn funcs() -> Vec<(u32, AbiFn)> {
    vec![
        entry!(0x0045cd60, fn_0045cd60(Ptr) -> u32),
        entry!(0x0045cec0, fn_0045cec0(Ptr)),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn getter_reads_the_word_at_0x28() {
        let mut e = Engine::new();
        let object: Ptr = Ptr::new(e.mem.alloc(0x30));
        e.mem.set_u32(object.addr() + 0x28, 0x1234_5678);
        assert_eq!(e.call(0x0045_cd60, &args![object]).u32(), 0x1234_5678);
    }

    #[test]
    fn destructor_releases_a_held_object_only() {
        let mut e = Engine::new();
        e.register(RELEASE_REFERENCE, |_, _| Ret::default());
        let holder: Ptr = Ptr::new(e.mem.alloc(8));
        e.call_log = Some(vec![]);
        e.call(0x0045_cec0, &args![holder]);
        assert_eq!(
            e.call_log.take().unwrap().len(),
            1,
            "only the top-level call"
        );

        e.mem.set_u32(holder.addr(), 0x5000);
        e.call_log = Some(vec![]);
        e.call(0x0045_cec0, &args![holder]);
        let log = e.call_log.take().unwrap();
        assert_eq!(log[1], (RELEASE_REFERENCE, vec![0x5000]));
    }
}
