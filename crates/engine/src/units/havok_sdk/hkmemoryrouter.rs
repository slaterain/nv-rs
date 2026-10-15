//! `Havok SDK/hkmemoryrouter.cpp` (Xbox PDB source unit), subsystem `Havok SDK`: its functions in
//! FalloutNV.exe 1.4.0.525, translated (docs/ENGINE_CRATE.md).

#[allow(unused_imports)]
use crate::prelude::*;

/// The thread-local-storage index (a word in `.data`).
const TLS_INDEX_01268108: u32 = 0x0126_8108;
/// The import slot of `TlsGetValue` (`kernel32`).
const TLS_GET_VALUE: u32 = 0x00fd_f044;

// Translated from 00c85750 (decompiled, FalloutNV.exe 1.4.0.525)
/// No map name (the map places it in `hkmemoryrouter.cpp`): `TlsGetValue`
/// of the index stored at `01268108`; the other Havok code uses the result
/// as the calling thread's memory router. Returns that value.
pub fn fn_00c85750(e: &mut Engine) -> u32 {
    let index = e.global::<u32>(TLS_INDEX_01268108);
    e.call(TLS_GET_VALUE, &args![index]).u32()
}

/// This unit's translated functions, by exe address.
pub fn funcs() -> Vec<(u32, AbiFn)> {
    vec![entry!(0x00c85750, fn_00c85750() -> u32)]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fn_00c85750_returns_the_tls_value_of_the_stored_index() {
        let mut e = Engine::new();
        e.map(0x0126_8000, 0x1000);
        e.set_global(TLS_INDEX_01268108, 7u32);
        e.register(TLS_GET_VALUE, |_, a| Ret {
            eax: 0x5000 + a[0],
            ..Ret::default()
        });
        assert_eq!(e.call(0x00c8_5750, &args![]).u32(), 0x5007);
    }
}
