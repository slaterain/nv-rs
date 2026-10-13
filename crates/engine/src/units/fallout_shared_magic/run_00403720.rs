//! `fallout shared/magic/run_00403720` (Xbox PDB source unit), subsystem `fallout shared/magic`: its functions in
//! FalloutNV.exe 1.4.0.525, translated (docs/ENGINE_CRATE.md).

#[allow(unused_imports)]
use crate::prelude::*;
use crate::types::BSStringT;

/// The string object's initializer (`00403970`): zeroes the word at `+0` and
/// then calls `004039c0` and `00403a00` with 0.
const INITIALIZE: u32 = 0x0040_3970;
/// The string object's text setter (`004037f0`, `RET 8`): two arguments, the
/// destructor passes (0, 0).
const SET_TEXT: u32 = 0x0040_37f0;

/// `0044a670` (`strlen` wrapper, cdecl: text).
const STRLEN: u32 = 0x0044_a670;
/// `00401000` (the allocator, cdecl: byte count).
const ALLOCATE: u32 = 0x0040_1000;
/// `00401030` (the deallocator, cdecl: pointer).
const DEALLOCATE: u32 = 0x0040_1030;
/// `00401460` (`memcpy`, cdecl: destination, source, byte count).
const MEMCPY: u32 = 0x0040_1460;

// Translated from 004037f0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `BSStringT<char>` text assignment (unnamed in the engine map): copies the C string `text` (null: an empty string)
/// into the string, reserving at least `min_capacity` characters. The new
/// capacity is the larger of the text length and `min_capacity`; a bigger
/// buffer than the string's capacity is allocated (`00401000`) and the old
/// one released (`00401030`), a capacity of 0 frees the buffer. The length
/// field is set to the text length (saturating at 16 bits, `004039c0`).
/// Returns whether the text length is not zero. `RET 8`.
pub fn fn_004037f0(e: &mut Engine, this: Ptr<BSStringT>, text: Ptr, min_capacity: u32) -> bool {
    let length = if text.addr() == 0 {
        0
    } else {
        e.call(STRLEN, &args![text]).u32()
    };
    let wanted = fn_00403940(e, length, min_capacity);
    let current = fn_004039a0(e, this);
    if wanted > current {
        let old = e.get(this, BSStringT::pString);
        let buffer = e.call(ALLOCATE, &args![wanted.wrapping_add(1)]).u32();
        e.set(this, BSStringT::pString, buffer);
        let buffer = e.get(this, BSStringT::pString);
        if text.addr() != 0 {
            e.call(MEMCPY, &args![buffer, text, length.wrapping_add(1)]);
        } else {
            e.mem.set_u8(buffer, 0);
        }
        if old != 0 {
            e.call(DEALLOCATE, &args![old]);
        }
        fn_00403a00(e, this, wanted);
    } else if wanted == 0 {
        let old = e.get(this, BSStringT::pString);
        if old != 0 {
            e.call(DEALLOCATE, &args![old]);
        }
        e.set(this, BSStringT::pString, 0);
        fn_00403a00(e, this, 0);
    } else if text.addr() != 0 {
        let buffer = e.get(this, BSStringT::pString);
        e.call(MEMCPY, &args![buffer, text, length.wrapping_add(1)]);
    } else {
        let buffer = e.get(this, BSStringT::pString);
        e.mem.set_u8(buffer, 0);
    }
    fn_004039c0(e, this, length);
    length != 0
}

// Translated from 00403940 (decompiled, FalloutNV.exe 1.4.0.525)
/// The larger of two unsigned values, `max(first, second)`: `second` when
/// `first <= second`. cdecl.
pub fn fn_00403940(_e: &mut Engine, first: u32, second: u32) -> u32 {
    if first > second {
        first
    } else {
        second
    }
}

// Translated from 004039a0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Reads the string's capacity (`sMaxLen`, Xbox PDB) as an unsigned 16-bit
/// value. `__fastcall` on `this` (ECX).
pub fn fn_004039a0(e: &mut Engine, this: Ptr<BSStringT>) -> u32 {
    e.get(this, BSStringT::sMaxLen) as u32
}

// Translated from 004037b0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Constructor of the string object (see `00406f60`, `Calendar::GetDateString`
/// in `00867970`): initializes it (`00403970`) and returns `this`.
pub fn fn_004037b0(e: &mut Engine, this: Ptr) -> Ptr {
    e.call(INITIALIZE, &args![this]);
    this
}

// Translated from 004037d0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Destructor of the string object: clears its text (`004037f0` with (0, 0)).
/// The decompiler's library name for this address is a false match.
pub fn fn_004037d0(e: &mut Engine, this: Ptr) {
    e.call(SET_TEXT, &args![this, 0u32, 0u32]);
}

// Translated from 00403970 (decompiled, FalloutNV.exe 1.4.0.525)
/// Initializes the string object (`BSStringT<char>`, Xbox PDB): no text
/// (`pString` 0) and a length and capacity of 0.
pub fn fn_00403970(e: &mut Engine, this: Ptr<BSStringT>) {
    e.set(this, BSStringT::pString, 0);
    fn_004039c0(e, this, 0);
    fn_00403a00(e, this, 0);
}

// Translated from 004039c0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Stores `value` as the string's length (`sLen`), limited to 16 bits: a
/// value of 0x10000 or more is stored as 0xFFFF.
pub fn fn_004039c0(e: &mut Engine, this: Ptr<BSStringT>, value: u32) {
    e.set(this, BSStringT::sLen, value.min(0xffff) as u16);
}

// Translated from 00403a00 (decompiled, FalloutNV.exe 1.4.0.525)
/// Stores `value` as the string's capacity (`sMaxLen`), limited to 16 bits: a
/// value of 0x10000 or more is stored as 0xFFFF.
pub fn fn_00403a00(e: &mut Engine, this: Ptr<BSStringT>, value: u32) {
    e.set(this, BSStringT::sMaxLen, value.min(0xffff) as u16);
}

/// This unit's translated functions, by exe address.
pub fn funcs() -> Vec<(u32, AbiFn)> {
    vec![
        entry!(0x004037f0, fn_004037f0(Ptr<BSStringT>, Ptr, u32) -> bool),
        entry!(0x00403940, fn_00403940(u32, u32) -> u32),
        entry!(0x004039a0, fn_004039a0(Ptr<BSStringT>) -> u32),
        entry!(0x004037b0, fn_004037b0(Ptr) -> Ptr),
        entry!(0x004037d0, fn_004037d0(Ptr)),
        entry!(0x00403970, fn_00403970(Ptr<BSStringT>)),
        entry!(0x004039c0, fn_004039c0(Ptr<BSStringT>, u32)),
        entry!(0x00403a00, fn_00403a00(Ptr<BSStringT>, u32)),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Doubles for the allocator, `memcpy`, `strlen` and the deallocator; the
    /// deallocator records the pointers it releases in the global at 0x6000.
    fn string_doubles(e: &mut Engine) {
        e.register(STRLEN, |e, a| {
            let mut n = 0;
            while e.mem.u8(a[0] + n) != 0 {
                n += 1;
            }
            n.into_ret()
        });
        e.register(ALLOCATE, |e, a| e.mem.alloc(a[0]).into_ret());
        e.register(MEMCPY, |e, a| {
            for i in 0..a[2] {
                let b = e.mem.u8(a[1] + i);
                e.mem.set_u8(a[0] + i, b);
            }
            a[0].into_ret()
        });
        e.register(DEALLOCATE, |_, _| Ret::default());
    }

    fn c_string(e: &mut Engine, text: &str) -> u32 {
        let p = e.mem.alloc(text.len() as u32 + 1);
        for (i, b) in text.bytes().enumerate() {
            e.mem.set_u8(p + i as u32, b);
        }
        e.mem.set_u8(p + text.len() as u32, 0);
        p
    }

    #[test]
    fn max_and_capacity_helpers() {
        let mut e = Engine::new();
        assert_eq!(e.call(0x0040_3940, &args![3u32, 9u32]).u32(), 9);
        assert_eq!(e.call(0x0040_3940, &args![9u32, 3u32]).u32(), 9);
        assert_eq!(e.call(0x0040_3940, &args![5u32, 5u32]).u32(), 5);
        assert_eq!(
            e.call(0x0040_3940, &args![0x8000_0000u32, 1u32]).u32(),
            0x8000_0000
        );
        let string: Ptr<BSStringT> = e.new_object();
        e.set(string, BSStringT::sMaxLen, 0xfffe);
        assert_eq!(e.call(0x0040_39a0, &args![string]).u32(), 0xfffe);
    }

    #[test]
    fn setter_allocates_when_the_text_outgrows_the_capacity() {
        let mut e = Engine::new();
        string_doubles(&mut e);
        let string: Ptr<BSStringT> = e.new_object();
        e.set(string, BSStringT::pString, 0x5000);
        e.set(string, BSStringT::sMaxLen, 2);
        let text = c_string(&mut e, "hello");
        e.call_log = Some(vec![]);
        let r = e.call(0x0040_37f0, &args![string, text, 0u32]);
        assert!(r.bool());
        let log = e.call_log.take().unwrap();
        let buffer = e.get(string, BSStringT::pString);
        assert_eq!(log[2], (ALLOCATE, vec![6]));
        assert_eq!(log[3], (MEMCPY, vec![buffer, text, 6]));
        assert_eq!(log[4], (DEALLOCATE, vec![0x5000]));
        assert_eq!(e.mem.bytes(buffer, 6), b"hello\0".to_vec());
        assert_eq!(e.get(string, BSStringT::sMaxLen), 5);
        assert_eq!(e.get(string, BSStringT::sLen), 5);
    }

    #[test]
    fn setter_reserves_the_requested_capacity_and_keeps_a_null_old_buffer() {
        let mut e = Engine::new();
        string_doubles(&mut e);
        let string: Ptr<BSStringT> = e.new_object();
        let text = c_string(&mut e, "ab");
        e.call_log = Some(vec![]);
        assert!(e.call(0x0040_37f0, &args![string, text, 9u32]).bool());
        let log = e.call_log.take().unwrap();
        assert_eq!(log[2], (ALLOCATE, vec![10]));
        assert!(!log.iter().any(|c| c.0 == DEALLOCATE));
        assert_eq!(e.get(string, BSStringT::sMaxLen), 9);
        assert_eq!(e.get(string, BSStringT::sLen), 2);
        // A null text with a capacity: an empty string in a new buffer.
        let other: Ptr<BSStringT> = e.new_object();
        assert!(!e.call(0x0040_37f0, &args![other, 0u32, 4u32]).bool());
        let buffer = e.get(other, BSStringT::pString);
        assert_eq!(e.mem.u8(buffer), 0);
        assert_eq!(e.get(other, BSStringT::sMaxLen), 4);
        assert_eq!(e.get(other, BSStringT::sLen), 0);
    }

    #[test]
    fn setter_copies_into_the_existing_buffer_when_it_fits() {
        let mut e = Engine::new();
        string_doubles(&mut e);
        let string: Ptr<BSStringT> = e.new_object();
        let buffer = e.mem.alloc(16);
        e.mem.set_u8(buffer, b'z');
        e.set(string, BSStringT::pString, buffer);
        e.set(string, BSStringT::sMaxLen, 8);
        let text = c_string(&mut e, "abc");
        e.call_log = Some(vec![]);
        assert!(e.call(0x0040_37f0, &args![string, text, 0u32]).bool());
        let log = e.call_log.take().unwrap();
        assert!(!log.iter().any(|c| c.0 == ALLOCATE || c.0 == DEALLOCATE));
        assert_eq!(log[2], (MEMCPY, vec![buffer, text, 4]));
        assert_eq!(e.get(string, BSStringT::pString), buffer);
        assert_eq!(e.get(string, BSStringT::sMaxLen), 8, "capacity unchanged");
        assert_eq!(e.get(string, BSStringT::sLen), 3);
        // Null text with room: the first byte is cleared.
        assert!(!e.call(0x0040_37f0, &args![string, 0u32, 4u32]).bool());
        assert_eq!(e.mem.u8(buffer), 0);
        assert_eq!(e.get(string, BSStringT::sLen), 0);
    }

    #[test]
    fn setter_with_nothing_and_no_reservation_frees_the_buffer() {
        let mut e = Engine::new();
        string_doubles(&mut e);
        let string: Ptr<BSStringT> = e.new_object();
        e.set(string, BSStringT::pString, 0x5000);
        e.set(string, BSStringT::sLen, 3);
        e.set(string, BSStringT::sMaxLen, 3);
        e.call_log = Some(vec![]);
        assert!(!e.call(0x0040_37f0, &args![string, 0u32, 0u32]).bool());
        let log = e.call_log.take().unwrap();
        assert_eq!(log[1], (DEALLOCATE, vec![0x5000]));
        assert_eq!(e.get(string, BSStringT::pString), 0);
        assert_eq!(e.get(string, BSStringT::sMaxLen), 0);
        assert_eq!(e.get(string, BSStringT::sLen), 0);
        // An empty C string behaves the same, with no buffer to free.
        let empty = c_string(&mut e, "");
        e.call_log = Some(vec![]);
        assert!(!e.call(0x0040_37f0, &args![string, empty, 0u32]).bool());
        assert!(!e.call_log.take().unwrap().iter().any(|c| c.0 == DEALLOCATE));
    }

    #[test]
    fn setter_saturates_the_length_at_16_bits() {
        let mut e = Engine::new();
        string_doubles(&mut e);
        e.register(STRLEN, |_, _| 0x1_0005u32.into_ret());
        e.register(ALLOCATE, |e, _| e.mem.alloc(16).into_ret());
        e.register(MEMCPY, |_, a| a[0].into_ret());
        let string: Ptr<BSStringT> = e.new_object();
        assert!(e.call(0x0040_37f0, &args![string, 0x1234u32, 0u32]).bool());
        assert_eq!(e.get(string, BSStringT::sLen), 0xffff);
        assert_eq!(e.get(string, BSStringT::sMaxLen), 0xffff);
    }

    #[test]
    fn constructor_initializes_and_returns_this() {
        let mut e = Engine::new();
        e.register(INITIALIZE, |_, _| Ret::default());
        e.call_log = Some(vec![]);
        let back = e.call(0x0040_37b0, &args![0x1000u32]).u32();
        assert_eq!(back, 0x1000);
        let log = e.call_log.take().unwrap();
        assert_eq!(log[1], (INITIALIZE, vec![0x1000]));
    }

    #[test]
    fn initializer_clears_the_text_and_both_counts() {
        let mut e = Engine::new();
        let string: Ptr<BSStringT> = e.new_object();
        e.set(string, BSStringT::pString, 0x1234);
        e.set(string, BSStringT::sLen, 7);
        e.set(string, BSStringT::sMaxLen, 9);
        e.call(0x0040_3970, &args![string]);
        assert_eq!(e.get(string, BSStringT::pString), 0);
        assert_eq!(e.get(string, BSStringT::sLen), 0);
        assert_eq!(e.get(string, BSStringT::sMaxLen), 0);
    }

    #[test]
    fn length_and_capacity_setters_saturate_at_16_bits() {
        let mut e = Engine::new();
        let string: Ptr<BSStringT> = e.new_object();
        for (value, stored) in [
            (0u32, 0u16),
            (0x1234, 0x1234),
            (0xffff, 0xffff),
            (0x1_0000, 0xffff),
            (u32::MAX, 0xffff),
        ] {
            e.call(0x0040_39c0, &args![string, value]);
            e.call(0x0040_3a00, &args![string, value]);
            assert_eq!(e.get(string, BSStringT::sLen), stored);
            assert_eq!(e.get(string, BSStringT::sMaxLen), stored);
        }
    }

    #[test]
    fn destructor_sets_empty_text() {
        let mut e = Engine::new();
        e.register(SET_TEXT, |_, _| Ret::default());
        e.call_log = Some(vec![]);
        e.call(0x0040_37d0, &args![0x1000u32]);
        let log = e.call_log.take().unwrap();
        assert_eq!(log[1], (SET_TEXT, vec![0x1000, 0, 0]));
    }
}
