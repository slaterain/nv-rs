//! Tests of the package flags, at the raw offset of the package loader (`00673A40`: +0x1C).

use super::*;
use crate::commands::test_support::*;

const PACK: u32 = FORMS + 0x1000;

#[test]
fn the_flags() {
    let mut e = engine();
    form(&mut e, PACK, 0x49, 0x0000_9acc);
    set_ref(&mut e, 1, PACK);
    let get = |e: &mut Engine, f: i32| call(e, 0x2624, &[r(1), n(f)]).1;
    let set = |e: &mut Engine, f: i32, v: i32| {
        call(e, 0x2625, &[r(1), n(f), n(v)]);
    };
    e.mem.set_u32(PACK + 0x1c, 0x8000_0001);
    assert_eq!(get(&mut e, 0), 1.0);
    assert_eq!(get(&mut e, 31), 1.0);
    assert_eq!(get(&mut e, 1), 0.0);
    assert_eq!(get(&mut e, 32), 0.0);
    set(&mut e, 5, 2);
    assert_eq!(e.mem.u32(PACK + 0x1c), 0x8000_0021);
    set(&mut e, 31, 0);
    assert_eq!(e.mem.u32(PACK + 0x1c), 0x21);
    set(&mut e, 32, 0);
    assert_eq!(e.mem.u32(PACK + 0x1c), 0x21, "flag 32: nothing");
}
