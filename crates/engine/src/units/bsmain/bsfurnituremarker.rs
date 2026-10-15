//! `BSMain/bsfurnituremarker.obj` (Xbox PDB source unit), subsystem `BSMain`: its functions in
//! FalloutNV.exe 1.4.0.525, translated (docs/ENGINE_CRATE.md).

#[allow(unused_imports)]
use crate::prelude::*;

/// `00a7b3b0` (named `NiExtraData::LinkObject` in the engine map): the base body the
/// wrapper forwards to; `thiscall`, `this` plus one stack word (`RET 4`).
const LINK_OBJECT_TARGET_00A7B3B0: u32 = 0x00a7_b3b0;

// Translated from 00c54130 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed in the engine map: the `BSFurnitureMarker` vtable word 010c2ae4, right after
/// `BSFurnitureMarker::LoadBinary` (010c2ae0). Forwards `this` and the stream to `00a7b3b0`
/// (`NiExtraData::LinkObject`). Returns nothing: Xbox PDB
/// `?LinkObject@NiObject@@UAAXAAVNiStream@@@Z` is void; `stream` is the `NiStream` reference
/// as an address. Parameter count from the `RET 4` of both bodies.
pub fn fn_00c54130(e: &mut Engine, this: Ptr, stream: Ptr) {
    e.call(LINK_OBJECT_TARGET_00A7B3B0, &args![this, stream]);
}

/// `00a7b290` (named `NiExtraData::RegisterStreamables` in the engine map): the base body
/// the wrapper forwards to; `thiscall`, `this` plus one stack word (`RET 4`).
const REGISTER_STREAMABLES_TARGET_00A7B290: u32 = 0x00a7_b290;

// Translated from 00c54150 (decompiled, FalloutNV.exe 1.4.0.525)
/// `BSFurnitureMarker::RegisterStreamables` (engine map vtable name, vtable word 010c2ae8):
/// forwards `this` and the stream to `00a7b290` (`NiExtraData::RegisterStreamables`). Returns
/// the `bool` in `AL`: Xbox PDB `?RegisterStreamables@NiObject@@UAA_NAAVNiStream@@@Z` returns
/// `_N`; `stream` is the `NiStream` reference as an address. Parameter count from the `RET 4`
/// of both bodies.
pub fn bs_furniture_marker_register_streamables(e: &mut Engine, this: Ptr, stream: Ptr) -> bool {
    e.call(REGISTER_STREAMABLES_TARGET_00A7B290, &args![this, stream])
        .bool()
}

/// This unit's translated functions, by exe address.
pub fn funcs() -> Vec<(u32, AbiFn)> {
    vec![
        entry!(0x00c5_4130, fn_00c54130(Ptr, Ptr)),
        entry!(0x00c5_4150, bs_furniture_marker_register_streamables(Ptr, Ptr) -> bool),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn forwarder_fn_00c54130_forwards_this_and_the_argument() {
        let mut e = Engine::new();
        e.register(LINK_OBJECT_TARGET_00A7B3B0, |_, _| Ret::default());
        e.call_log = Some(vec![]);
        e.call(0x00c5_4130, &args![0x1000u32, 0x2000u32]);
        let log = e.call_log.take().unwrap();
        let hits: Vec<_> = log
            .iter()
            .filter(|call| call.0 == LINK_OBJECT_TARGET_00A7B3B0)
            .collect();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].1, vec![0x1000u32, 0x2000]);
    }

    #[test]
    fn forwarder_bs_furniture_marker_register_streamables_forwards_this_and_the_argument_and_returns_the_byte(
    ) {
        let mut e = Engine::new();
        // The double sets bit 8 too: only the low byte (AL) is the result.
        e.register(REGISTER_STREAMABLES_TARGET_00A7B290, |_, a| {
            (0x100 | (a[1] & 1)).into_ret()
        });
        for (argument, expected) in [(0x2001u32, true), (0x2000u32, false)] {
            e.call_log = Some(vec![]);
            let result = e.call(0x00c5_4150, &args![0x1000u32, argument]);
            let log = e.call_log.take().unwrap();
            let hits: Vec<_> = log
                .iter()
                .filter(|call| call.0 == REGISTER_STREAMABLES_TARGET_00A7B290)
                .collect();
            assert_eq!(hits.len(), 1);
            assert_eq!(hits[0].1, vec![0x1000u32, argument]);
            assert_eq!(result.bool(), expected);
        }
    }
}
