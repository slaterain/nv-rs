//! `fallout/misc/run_00845230` (Xbox PDB source unit), subsystem `fallout/misc`: its functions in
//! FalloutNV.exe 1.4.0.525, translated (docs/ENGINE_CRATE.md).

#[allow(unused_imports)]
use crate::prelude::*;

/// `00845b30(this, word)` (unit `bgssaveloadchangesmap`): the base
/// constructor `00845550` runs with `0x13af`.
const BASE_CONSTRUCTOR: u32 = 0x0084_5b30;
/// The vtable `00845550` stores.
const VTABLE: u32 = 0x0107_ed68;

// Translated from 00845550 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed in the engine map: runs `00845b30(this, 0x13af)` and stores the
/// vtable `0107ed68` over the one it set. Returns `this`.
pub fn fn_00845550(e: &mut Engine, this: Ptr) -> Ptr {
    e.call(BASE_CONSTRUCTOR, &args![this, 0x13afu32]);
    e.mem.set_u32(this.addr(), VTABLE);
    this
}

/// This unit's translated functions, by exe address.
pub fn funcs() -> Vec<(u32, AbiFn)> {
    vec![entry!(0x00845550, fn_00845550(Ptr) -> Ptr)]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constructs_the_base_then_stores_its_vtable() {
        let mut e = Engine::new();
        e.register(BASE_CONSTRUCTOR, |e, a| {
            e.mem.set_u32(a[0], 0x1234);
            Ret::default()
        });
        let this = e.mem.alloc(16);
        e.call_log = Some(vec![]);
        let r = e.call(0x00845550, &args![this]);
        assert_eq!(r.u32(), this);
        let log = e.call_log.take().unwrap();
        assert_eq!(log[1], (BASE_CONSTRUCTOR, vec![this, 0x13af]));
        assert_eq!(e.mem.u32(this), VTABLE, "the vtable replaces the base's");
    }
}
