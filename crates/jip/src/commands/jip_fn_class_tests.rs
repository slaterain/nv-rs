//! Tests of the class commands, at the raw offsets the class loader (`005F6930`) and JIP's header
//! give: attributes +0x3C, tag skills +0x44, flags +0x54, services +0x58.

use super::*;
use crate::commands::test_support::*;

const CLASS: u32 = FORMS + 0x1000;

fn setup() -> Engine {
    let mut e = engine();
    form(&mut e, CLASS, 0x07, 0x0000_c1a5);
    set_ref(&mut e, 1, CLASS);
    e
}

#[test]
fn tag_skills_and_attributes() {
    let mut e = setup();
    e.mem.set_u32(CLASS + 0x44 + 4 * 2, 0xffff_ffff);
    e.mem.set_u32(CLASS + 0x44 + 4 * 3, 45);
    e.mem.set_u8(CLASS + 0x3c + 6, 9);
    let get = |e: &mut Engine, t: i32| call(e, 0x263e, &[r(1), n(t)]).1;
    assert_eq!(get(&mut e, 2), -1.0);
    assert_eq!(get(&mut e, 3), 45.0);
    assert_eq!(get(&mut e, 10), 9.0, "luck: attribute 6");
    e.mem.set_u8(CLASS + 0x3c + 7, 99);
    assert_eq!(get(&mut e, 11), 0.0, "past 10");
    let set = |e: &mut Engine, t: i32, v: i32| {
        call(e, 0x263f, &[r(1), n(t), n(v)]);
    };
    for (v, kept) in [(32, true), (33, false), (31, false), (46, false), (-1, true), (44, true)] {
        e.mem.set_u32(CLASS + 0x44, 7);
        set(&mut e, 0, v);
        let want = if kept { v as u32 } else { 7 };
        assert_eq!(e.mem.u32(CLASS + 0x44), want, "tag skill {v}");
    }
    set(&mut e, 4, 10);
    assert_eq!(e.mem.u8(CLASS + 0x3c), 10);
    set(&mut e, 4, 11);
    assert_eq!(e.mem.u8(CLASS + 0x3c), 10);
    set(&mut e, 5, 0);
    assert_eq!(e.mem.u8(CLASS + 0x3d), 0, "0 refused (was 0)");
    set(&mut e, 5, 1);
    assert_eq!(e.mem.u8(CLASS + 0x3d), 1);
    set(&mut e, 11, 5);
    assert_eq!(e.mem.u8(CLASS + 0x3c + 7), 99, "trait 11: nothing");
}

#[test]
fn flags_and_services() {
    let mut e = setup();
    e.mem.set_u32(CLASS + 0x54, 2);
    e.mem.set_u32(CLASS + 0x58, 1 << 17);
    let get = |e: &mut Engine, f: i32| call(e, 0x2640, &[r(1), n(f)]).1;
    assert_eq!((get(&mut e, 0), get(&mut e, 1)), (0.0, 1.0));
    assert_eq!(get(&mut e, 19), 1.0, "service bit 17");
    assert_eq!(get(&mut e, 2), 0.0);
    e.mem.set_u32(CLASS + 0x58, u32::MAX);
    assert_eq!(get(&mut e, 20), 0.0, "past 19");
    let set = |e: &mut Engine, f: i32, v: i32| {
        call(e, 0x2641, &[r(1), n(f), n(v)]);
    };
    set(&mut e, 0, 1);
    assert_eq!(e.mem.u32(CLASS + 0x54), 3);
    set(&mut e, 1, 0);
    assert_eq!(e.mem.u32(CLASS + 0x54), 1);
    set(&mut e, 2, 0);
    assert_eq!(e.mem.u32(CLASS + 0x58), u32::MAX - 1);
    set(&mut e, 20, 0);
    assert_eq!(e.mem.u32(CLASS + 0x58), u32::MAX - 1, "flag 20: nothing");
}
