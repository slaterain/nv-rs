//! `fallout shared/facegen/bsfacegenmanager.cpp` (Xbox PDB source unit), subsystem `fallout shared/facegen`: its functions in
//! FalloutNV.exe 1.4.0.525, translated (docs/ENGINE_CRATE.md).

#[allow(unused_imports)]
use crate::prelude::*;

/// `tolower` (LIBCMT, `00ec67aa`).
const TOLOWER: u32 = 0x00ec_67aa;

// Translated from 00600b20 (PC disassembly, FalloutNV.exe 1.4.0.525)
/// `BSTCaseInsensitiveStringMap<..>::KeyToHashIndex` (Xbox PDB; `thiscall`,
/// `RET 4`): the bucket of a key. Each byte up to the terminator, lowered
/// (`tolower` on the sign-extended byte), is folded in by [`fn_00659210`];
/// the hash modulo the map's bucket count (+4, unsigned).
pub fn key_to_hash_index(e: &mut Engine, this: Ptr, key: u32) -> u32 {
    let mut hash = 0u32;
    let mut p = key;
    loop {
        let c = e.mem.u8(p) as i8 as i32;
        if c == 0 {
            break;
        }
        let lowered = e.call(TOLOWER, &args![c as u32]).u32() as u8;
        hash = fn_00659210(e, this, lowered, hash);
        p = p.wrapping_add(1);
    }
    hash % e.mem.u32(this.addr() + 4)
}

// Translated from 00659210 (PC disassembly, FalloutNV.exe 1.4.0.525)
/// One step of the string hash (`thiscall`, `RET 8`; `this` unused):
/// `hash * 33 + c`, the byte sign-extended (`MOVSX`).
pub fn fn_00659210(_e: &mut Engine, _this: Ptr, c: u8, hash: u32) -> u32 {
    (hash << 5)
        .wrapping_add(hash)
        .wrapping_add(c as i8 as i32 as u32)
}

/// The `__stricmp` wrapper (`00404dc0`, `cdecl`).
const STRICMP_WRAPPER: u32 = 0x0040_4dc0;

// Translated from 00486b50 (PC disassembly, FalloutNV.exe 1.4.0.525)
/// `BSTCaseInsensitiveStringMap<..>::IsKeysEqual` (Xbox PDB; `thiscall`,
/// `RET 8`; `this` unused): whether the keys are equal ignoring case
/// (`00404dc0` answers 0).
pub fn is_keys_equal(e: &mut Engine, _this: Ptr, a: u32, b: u32) -> bool {
    e.call(STRICMP_WRAPPER, &args![a, b]).u32() == 0
}

/// `006532c0`: thiscall (`this`, one word), returns a bool in AL.
const CHECK_006532C0: u32 = 0x0065_32c0;

// Translated from 00653290 (decompiled, FalloutNV.exe 1.4.0.525)
/// No map name (the map places it in `bsfacegenmanager.cpp`). Thiscall
/// (`RET 4`): `this` when `006532c0(this, argument)` is true, otherwise null.
pub fn fn_00653290(e: &mut Engine, this: Ptr, argument: u32) -> Ptr {
    if e.call(CHECK_006532C0, &args![this, argument]).bool() {
        this
    } else {
        Ptr::NULL
    }
}

/// The function `fn_00657fd0` calls on the member (`00658100`).
const CALLEE_00658100: u32 = 0x0065_8100;

/// The function `fn_00658040` calls on the member (`00658130`).
const CALLEE_00658130: u32 = 0x0065_8130;

// Translated from 00657fd0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Thin wrapper: calls `00658100` with `ECX` = `this + 0x8` and passes on its result.
pub fn fn_00657fd0(e: &mut Engine, this: Ptr) -> u32 {
    e.call(CALLEE_00658100, &args![this.byte_add(0x8)]).u32()
}

// Translated from 00658040 (decompiled, FalloutNV.exe 1.4.0.525)
/// Thin wrapper: calls `00658130` with `ECX` = `this + 0x8` and passes on its result.
pub fn fn_00658040(e: &mut Engine, this: Ptr) -> u32 {
    e.call(CALLEE_00658130, &args![this.byte_add(0x8)]).u32()
}

/// This unit's translated functions, by exe address.
pub fn funcs() -> Vec<(u32, AbiFn)> {
    vec![
        entry!(0x00653290, fn_00653290(Ptr, u32) -> Ptr),
        entry!(0x00486b50, is_keys_equal(Ptr, u32, u32) -> bool),
        entry!(0x00600b20, key_to_hash_index(Ptr, u32) -> u32),
        entry!(0x00659210, fn_00659210(Ptr, u8, u32) -> u32),
        entry!(0x00486b50, is_keys_equal(Ptr, u32, u32) -> bool),
        entry!(0x00600b20, key_to_hash_index(Ptr, u32) -> u32),
        entry!(0x00659210, fn_00659210(Ptr, u8, u32) -> u32),
        entry!(0x0065_7fd0, fn_00657fd0(Ptr) -> u32),
        entry!(0x0065_8040, fn_00658040(Ptr) -> u32),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_are_equal_when_the_compare_answers_zero() {
        let mut e = Engine::new();
        e.register(STRICMP_WRAPPER, |_, a| u32::from(a[0] != a[1]).into_ret());
        assert_eq!(e.call(0x00486b50, &[0, 5, 5]).u32() & 0xff, 1);
        assert_eq!(e.call(0x00486b50, &[0, 5, 6]).u32() & 0xff, 0);
    }

    #[test]
    fn hashes_the_lowered_key_modulo_the_bucket_count() {
        let mut e = Engine::new();
        let map = e.mem.alloc(8);
        e.mem.set_u32(map + 4, 37);
        let key = e.mem.alloc(8);
        e.mem.write(key, b"fAb\0");
        let mut h = 0u32;
        for c in b"fab" {
            h = h.wrapping_mul(33).wrapping_add(u32::from(*c));
        }
        assert_eq!(e.call(0x00600b20, &[map, key]).u32(), h % 37);
    }

    #[test]
    fn a_high_byte_is_sign_extended() {
        let mut e = Engine::new();
        assert_eq!(
            e.call(0x00659210, &[0, 0xe9, 1]).u32(),
            33u32.wrapping_sub(0x17)
        );
    }

    #[test]
    fn fn_00653290_returns_this_only_when_the_check_passes() {
        let mut e = Engine::new();
        e.register(CHECK_006532C0, |_, a| Ret {
            eax: a[1],
            ..Ret::default()
        });
        let this = Ptr::<()>::new(0x5000);
        assert_eq!(e.call(0x0065_3290, &args![this, 1u32]).ptr::<()>(), this);
        assert_eq!(
            e.call(0x0065_3290, &args![this, 0u32]).ptr::<()>(),
            Ptr::<()>::NULL
        );
    }

    #[test]
    fn keys_are_equal_when_the_compare_answers_zero() {
        let mut e = Engine::new();
        e.register(STRICMP_WRAPPER, |_, a| u32::from(a[0] != a[1]).into_ret());
        assert_eq!(e.call(0x00486b50, &[0, 5, 5]).u32() & 0xff, 1);
        assert_eq!(e.call(0x00486b50, &[0, 5, 6]).u32() & 0xff, 0);
    }

    #[test]
    fn hashes_the_lowered_key_modulo_the_bucket_count() {
        let mut e = Engine::new();
        let map = e.mem.alloc(8);
        e.mem.set_u32(map + 4, 37);
        let key = e.mem.alloc(8);
        e.mem.write(key, b"fAb\0");
        let mut h = 0u32;
        for c in b"fab" {
            h = h.wrapping_mul(33).wrapping_add(u32::from(*c));
        }
        assert_eq!(e.call(0x00600b20, &[map, key]).u32(), h % 37);
    }

    #[test]
    fn a_high_byte_is_sign_extended() {
        let mut e = Engine::new();
        assert_eq!(
            e.call(0x00659210, &[0, 0xe9, 1]).u32(),
            33u32.wrapping_sub(0x17)
        );
    }

    #[test]
    fn l68d_00657fd0_passes_this_plus_8_to_the_callee() {
        let mut e = Engine::new();
        e.register(CALLEE_00658100, |_, a| Ret {
            eax: a[0] ^ 0x5555,
            ..Ret::default()
        });
        e.call_log = Some(vec![]);
        assert_eq!(e.call(0x0065_7fd0, &args![0x4100u32]).u32(), 0x145d);
        let log = e.call_log.take().unwrap();
        let inner: Vec<_> = log.iter().filter(|c| c.0 == CALLEE_00658100).collect();
        assert_eq!(inner.len(), 1);
        assert_eq!(inner[0].1, vec![0x4100 + 0x8]);
    }

    #[test]
    fn l68d_00658040_passes_this_plus_8_to_the_callee() {
        let mut e = Engine::new();
        e.register(CALLEE_00658130, |_, a| Ret {
            eax: a[0] ^ 0x5555,
            ..Ret::default()
        });
        e.call_log = Some(vec![]);
        assert_eq!(e.call(0x0065_8040, &args![0x4200u32]).u32(), 0x175d);
        let log = e.call_log.take().unwrap();
        let inner: Vec<_> = log.iter().filter(|c| c.0 == CALLEE_00658130).collect();
        assert_eq!(inner.len(), 1);
        assert_eq!(inner[0].1, vec![0x4200 + 0x8]);
    }
}
