//! `(unplaced)/run_00401000` (Xbox PDB source unit), subsystem `(unplaced)`: its functions in
//! FalloutNV.exe 1.4.0.525, translated (docs/ENGINE_CRATE.md).

#[allow(unused_imports)]
use crate::prelude::*;

// Translated from 00401050 (decompiled, FalloutNV.exe 1.4.0.525)
/// The compiler's "vector constructor iterator" (`__stdcall`, four words):
/// calls the `__thiscall` constructor at `constructor` once for each of
/// `count` elements of `size` bytes, starting at `array`. A zero or negative
/// `count` constructs nothing. The constructor's result is ignored.
pub fn fn_00401050(e: &mut Engine, array: u32, size: u32, count: i32, constructor: u32) {
    let mut element = array;
    let mut remaining = count;
    loop {
        remaining = remaining.wrapping_sub(1);
        if remaining < 0 {
            break;
        }
        e.call(constructor, &args![element]);
        element = element.wrapping_add(size);
    }
}

// Translated from 00401170 (decompiled, FalloutNV.exe 1.4.0.525)
/// Returns the byte at `+4` of `this` (a plain field getter, zero-extended
/// into `EAX`).
pub fn fn_00401170(e: &mut Engine, this: Ptr) -> u8 {
    e.mem.u8(this.addr() + 4)
}

/// This unit's translated functions, by exe address.
pub fn funcs() -> Vec<(u32, AbiFn)> {
    vec![
        entry!(0x00401050, fn_00401050(u32, u32, i32, u32)),
        entry!(0x00401170, fn_00401170(Ptr) -> u8),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    const CONSTRUCTOR: u32 = 0x0060_0000;

    fn constructed(count: i32) -> Vec<u32> {
        let mut e = Engine::new();
        let seen = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
        let log = seen.clone();
        e.register_double(CONSTRUCTOR, move |_, a| {
            log.borrow_mut().push(a[0]);
            Ret::default()
        });
        e.call(0x0040_1050, &args![0x1000u32, 0x24u32, count, CONSTRUCTOR]);
        let result = seen.borrow().clone();
        result
    }

    #[test]
    fn constructs_each_element_in_turn() {
        assert_eq!(constructed(3), vec![0x1000, 0x1024, 0x1048]);
    }

    #[test]
    fn constructs_nothing_for_zero_or_negative_counts() {
        assert!(constructed(0).is_empty());
        assert!(constructed(-2).is_empty());
    }

    #[test]
    fn reads_the_byte_at_4() {
        let mut e = Engine::new();
        let object: Ptr = Ptr::new(e.mem.alloc(0x10));
        e.mem.set_u8(object.addr() + 4, 0x5a);
        // Neighbouring bytes are not part of the result.
        e.mem.set_u8(object.addr() + 5, 0xff);
        assert_eq!(e.call(0x0040_1170, &args![object]).u32(), 0x5a);
    }
}
