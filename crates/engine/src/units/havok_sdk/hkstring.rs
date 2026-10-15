//! `Havok SDK/hkstring.cpp` (Xbox PDB source unit), subsystem `Havok SDK`: its functions in
//! FalloutNV.exe 1.4.0.525, translated (docs/ENGINE_CRATE.md).
//!
//! `hkString::memCpy`, `memMove` and `memSet` are each a single `JMP` to the C runtime
//! (`memcpy` `00ec44d0`, `memmove` `00ec7230`, `memset` `00ec61c0`), so they forward their
//! arguments and return the runtime's result.

#[allow(unused_imports)]
use crate::prelude::*;

/// The C runtime's `memcpy`, `memmove` and `memset`.
const CRT_MEMCPY: u32 = 0x00ec_44d0;
const CRT_MEMMOVE: u32 = 0x00ec_7230;
const CRT_MEMSET: u32 = 0x00ec_61c0;

// Translated from 00cb10d0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `hkString::strLen` (Xbox PDB, cdecl): the number of bytes before the terminating zero.
pub fn hk_string_str_len(e: &mut Engine, string: Ptr) -> i32 {
    let mut length = 0u32;
    while e.mem.u8(string.addr().wrapping_add(length)) != 0 {
        length = length.wrapping_add(1);
    }
    length as i32
}

// Translated from 00cb1260 (decompiled, FalloutNV.exe 1.4.0.525)
/// `hkString::memCpy` (Xbox PDB, cdecl): a jump to the runtime's `memcpy`
/// `(destination, source, size)`; returns its result.
pub fn hk_string_mem_cpy(e: &mut Engine, destination: Ptr, source: Ptr, size: u32) -> Ptr {
    e.call(CRT_MEMCPY, &args![destination, source, size]).ptr()
}

// Translated from 00cb1270 (decompiled, FalloutNV.exe 1.4.0.525)
/// `hkString::memMove` (Xbox PDB, cdecl): a jump to the runtime's `memmove`
/// `(destination, source, size)`; returns its result.
pub fn hk_string_mem_move(e: &mut Engine, destination: Ptr, source: Ptr, size: u32) -> Ptr {
    e.call(CRT_MEMMOVE, &args![destination, source, size]).ptr()
}

// Translated from 00cb1280 (decompiled, FalloutNV.exe 1.4.0.525)
/// `hkString::memSet` (Xbox PDB, cdecl): a jump to the runtime's `memset`
/// `(destination, value, size)`; returns its result.
pub fn hk_string_mem_set(e: &mut Engine, destination: Ptr, value: i32, size: u32) -> Ptr {
    e.call(CRT_MEMSET, &args![destination, value, size]).ptr()
}

/// This unit's translated functions, by exe address.
pub fn funcs() -> Vec<(u32, AbiFn)> {
    vec![
        entry!(0x00cb_10d0, hk_string_str_len(Ptr) -> i32),
        entry!(0x00cb_1260, hk_string_mem_cpy(Ptr, Ptr, u32) -> Ptr),
        entry!(0x00cb_1270, hk_string_mem_move(Ptr, Ptr, u32) -> Ptr),
        entry!(0x00cb_1280, hk_string_mem_set(Ptr, i32, u32) -> Ptr),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn str_len_counts_bytes_before_the_zero() {
        let mut e = Engine::new();
        let text = e.mem.alloc(16);
        e.mem.set_cstr(text, b"hello");
        assert_eq!(e.call(0x00cb_10d0, &args![text]).i32(), 5);
        e.mem.set_u8(text, 0);
        assert_eq!(e.call(0x00cb_10d0, &args![text]).i32(), 0);
    }

    /// Each wrapper passes its three words on unchanged and returns the runtime's result.
    fn forwards(wrapper: u32, runtime: u32) {
        let mut e = Engine::new();
        e.register(runtime, |_, a| Ret {
            eax: a[0] ^ a[1] ^ a[2],
            ..Ret::default()
        });
        e.call_log = Some(vec![]);
        let got = e.call(wrapper, &args![0x1000u32, 0x20u32, 0x300u32]).u32();
        assert_eq!(got, 0x1000 ^ 0x20 ^ 0x300);
        let log = e.call_log.take().unwrap();
        let inner: Vec<_> = log.iter().filter(|c| c.0 == runtime).collect();
        assert_eq!(inner.len(), 1);
        assert_eq!(inner[0].1, vec![0x1000, 0x20, 0x300]);
    }

    #[test]
    fn mem_functions_forward_to_the_runtime() {
        forwards(0x00cb_1260, CRT_MEMCPY);
        forwards(0x00cb_1270, CRT_MEMMOVE);
        forwards(0x00cb_1280, CRT_MEMSET);
    }
}
