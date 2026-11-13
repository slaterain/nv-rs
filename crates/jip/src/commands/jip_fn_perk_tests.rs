//! Tests of the perk flags, at the raw offsets of the perk loader (`005EADA0`: trait +0x38,
//! playable +0x3B, hidden +0x3C).

use super::*;
use crate::commands::test_support::*;

const PERK: u32 = FORMS + 0x1000;

#[test]
fn the_flags() {
    let mut e = engine();
    form(&mut e, PERK, 0x56, 0x0000_9e4c);
    set_ref(&mut e, 1, PERK);
    let get = |e: &mut Engine, f: i32| call(e, 0x27be, &[r(1), n(f)]).1;
    let set = |e: &mut Engine, f: i32, v: i32| {
        call(e, 0x27bf, &[r(1), n(f), n(v)]);
    };
    for (f, off) in [(0, 0x38), (1, 0x3b), (2, 0x3c)] {
        assert_eq!(get(&mut e, f), 0.0, "flag {f} clear");
        e.mem.set_u8(PERK + off, 2);
        assert_eq!(get(&mut e, f), 1.0, "flag {f}: any non-zero byte");
        set(&mut e, f, 0);
        assert_eq!(e.mem.u8(PERK + off), 0);
        set(&mut e, f, -7);
        assert_eq!(e.mem.u8(PERK + off), 1, "flag {f}: = (value != 0)");
    }
    e.mem.set_u8(PERK + 0x39, 5);
    e.mem.set_u8(PERK + 0x3a, 5);
    assert_eq!(get(&mut e, 3), 0.0, "flag 3: nothing");
    let before = e.mem.bytes(PERK, 0x50);
    set(&mut e, 3, 1);
    assert_eq!(e.mem.bytes(PERK, 0x50), before, "flag 3: nothing");
}
