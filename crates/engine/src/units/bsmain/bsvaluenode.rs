//! `BSMain/bsvaluenode.obj` (Xbox PDB source unit), subsystem `BSMain`: its functions in
//! FalloutNV.exe 1.4.0.525, translated (docs/ENGINE_CRATE.md).

#[allow(unused_imports)]
use crate::prelude::*;

/// `00a5e9d0` (named `NiNode::LinkObject` in the engine map): the base body the wrapper
/// forwards to; `thiscall`, `this` plus one stack word (`RET 4`).
const LINK_OBJECT_TARGET_00A5E9D0: u32 = 0x00a5_e9d0;

// Translated from 00c43900 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed in the engine map: the `BSValueNode` vtable word 010c19ac, right after
/// `BSValueNode::LoadBinary` (010c19a8). Forwards `this` and the stream to `00a5e9d0`
/// (`NiNode::LinkObject`). Returns nothing: Xbox PDB
/// `?LinkObject@NiObject@@UAAXAAVNiStream@@@Z` is void; `stream` is the `NiStream` reference
/// as an address. Parameter count from the `RET 4` of both bodies.
pub fn fn_00c43900(e: &mut Engine, this: Ptr, stream: Ptr) {
    e.call(LINK_OBJECT_TARGET_00A5E9D0, &args![this, stream]);
}

/// This unit's translated functions, by exe address.
pub fn funcs() -> Vec<(u32, AbiFn)> {
    vec![entry!(0x00c4_3900, fn_00c43900(Ptr, Ptr))]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn forwarder_fn_00c43900_forwards_this_and_the_argument() {
        let mut e = Engine::new();
        e.register(LINK_OBJECT_TARGET_00A5E9D0, |_, _| Ret::default());
        e.call_log = Some(vec![]);
        e.call(0x00c4_3900, &args![0x1000u32, 0x2000u32]);
        let log = e.call_log.take().unwrap();
        let hits: Vec<_> = log
            .iter()
            .filter(|call| call.0 == LINK_OBJECT_TARGET_00A5E9D0)
            .collect();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].1, vec![0x1000u32, 0x2000]);
    }
}
