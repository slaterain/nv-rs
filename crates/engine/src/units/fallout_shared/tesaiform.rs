//! `fallout shared/tesaiform.cpp` (Xbox PDB source unit), subsystem `fallout shared`: its functions in
//! FalloutNV.exe 1.4.0.525, translated (docs/ENGINE_CRATE.md).

#[allow(unused_imports)]
use crate::prelude::*;

/// The function `fn_0047ede0` calls on the member (`0067e400`).
const CALLEE_0067E400: u32 = 0x0067_e400;

// Translated from 0047ede0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Named `TESAIForm::ClearDataComponent` (Xbox PDB name in the engine map). Thin wrapper: calls `0067e400` with `ECX` = `this + 0x18` and passes on nothing (the callee returns no value).
pub fn fn_0047ede0(e: &mut Engine, this: Ptr) {
    e.call(CALLEE_0067E400, &args![this.byte_add(0x18)]);
}

/// This unit's translated functions, by exe address.
pub fn funcs() -> Vec<(u32, AbiFn)> {
    vec![entry!(0x0047_ede0, fn_0047ede0(Ptr))]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn l68d_0047ede0_passes_this_plus_18_to_the_callee() {
        let mut e = Engine::new();
        e.register(CALLEE_0067E400, |_, _| Ret::default());
        e.call_log = Some(vec![]);
        e.call(0x0047_ede0, &args![0x4100u32]);
        let log = e.call_log.take().unwrap();
        let inner: Vec<_> = log.iter().filter(|c| c.0 == CALLEE_0067E400).collect();
        assert_eq!(inner.len(), 1);
        assert_eq!(inner[0].1, vec![0x4100 + 0x18]);
    }
}
