//! `fallout shared/havok/havokstream.cpp` (Xbox PDB source unit), subsystem `fallout shared/havok`: its functions in
//! FalloutNV.exe 1.4.0.525, translated (docs/ENGINE_CRATE.md).

#[allow(unused_imports)]
use crate::prelude::*;

/// `00cb1fb0` (unnamed in the engine map): the base body the wrapper forwards to;
/// `thiscall`, `this` plus one stack word (`RET 4`).
const COUNT_TARGET_00CB1FB0: u32 = 0x00cb_1fb0;

// Translated from 006264b0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed in the engine map (only caller `006255f0`): forwards `this` and one integer to
/// `00cb1fb0` (Havok SDK `hkmap.cpp` in the engine map), which compares `00cb18f0(count)` with
/// the field at `this+8` and returns `EAX` = 1 when it is larger, else calls `00cb19b0` and
/// returns `EAX` = 0 (`MOV EAX,1` / `XOR EAX,EAX`, a full-register result). The wrapper
/// returns it unchanged.
pub fn fn_006264b0(e: &mut Engine, this: Ptr, count: u32) -> u32 {
    e.call(COUNT_TARGET_00CB1FB0, &args![this, count]).u32()
}

/// This unit's translated functions, by exe address.
pub fn funcs() -> Vec<(u32, AbiFn)> {
    vec![entry!(0x0062_64b0, fn_006264b0(Ptr, u32) -> u32)]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn forwarder_fn_006264b0_forwards_this_and_the_argument() {
        let mut e = Engine::new();
        e.register(COUNT_TARGET_00CB1FB0, |_, a| {
            (0x4000_0000 | a[1]).into_ret()
        });
        e.call_log = Some(vec![]);
        let result = e.call(0x0062_64b0, &args![0x1000u32, 0x2000u32]);
        let log = e.call_log.take().unwrap();
        let hits: Vec<_> = log
            .iter()
            .filter(|call| call.0 == COUNT_TARGET_00CB1FB0)
            .collect();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].1, vec![0x1000u32, 0x2000]);
        assert_eq!(result.u32(), 0x4000_2000);
    }
}
