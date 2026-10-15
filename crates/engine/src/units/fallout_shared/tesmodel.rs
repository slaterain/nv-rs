//! `fallout shared/tesmodel.cpp` (Xbox PDB source unit), subsystem `fallout shared`: its functions in
//! FalloutNV.exe 1.4.0.525, translated (docs/ENGINE_CRATE.md).

#[allow(unused_imports)]
use crate::prelude::*;

/// The function `fn_004890e0` calls on the member (`00489620`).
const CALLEE_00489620: u32 = 0x0048_9620;

// Translated from 004890e0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Named `TESModel::ClearDataComponent` (Xbox PDB name in the engine map). Thin wrapper: calls `00489620` with `ECX` = `this + 0xc` and passes on its `bool` result (`AL`).
pub fn fn_004890e0(e: &mut Engine, this: Ptr) -> bool {
    e.call(CALLEE_00489620, &args![this.byte_add(0xc)]).bool()
}

/// This unit's translated functions, by exe address.
pub fn funcs() -> Vec<(u32, AbiFn)> {
    vec![entry!(0x0048_90e0, fn_004890e0(Ptr) -> bool)]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn l68d_004890e0_passes_this_plus_c_to_the_callee() {
        let mut e = Engine::new();
        e.register(CALLEE_00489620, |_, a| Ret {
            eax: a[0] & 1,
            ..Ret::default()
        });
        e.call_log = Some(vec![]);
        assert_eq!(e.call(0x0048_90e0, &args![0x4100u32]).bool(), false);
        assert_eq!(e.call(0x0048_90e0, &args![0x4100u32 + 1]).bool(), true);
        let log = e.call_log.take().unwrap();
        let inner: Vec<_> = log.iter().filter(|c| c.0 == CALLEE_00489620).collect();
        assert_eq!(inner.len(), 2);
        assert_eq!(inner[0].1, vec![0x4100 + 0xc]);
    }
}
