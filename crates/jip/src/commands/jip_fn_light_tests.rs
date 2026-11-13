//! Tests of the light getters, at the raw offsets the light loader (`0050CAF0`) gives: radius
//! +0xA0, colour +0xA4..+0xA6, flags +0xA8, falloff +0xAC, field of view +0xB0, fade +0xB4.

use super::*;
use crate::commands::test_support::*;

const LIGHT: u32 = FORMS + 0x1000;

fn setup() -> Engine {
    let mut e = engine();
    form(&mut e, LIGHT, 0x1e, 0x0000_0119);
    set_ref(&mut e, 1, LIGHT);
    // `kParamType_BoundObject`: xNVSE's reader asks the form's `IsBoundObject` virtual.
    let vtbl = FORMS + 0x6000;
    e.mem.set_u32(LIGHT, vtbl);
    e.mem
        .set_u32(vtbl + nvse::vtable::IS_BOUND_OBJECT, 0x0050_1000);
    e.register(0x0050_1000, |_, _| Ret {
        eax: 1,
        ..Ret::default()
    });
    e
}

#[test]
fn the_numbers_and_flags() {
    let mut e = setup();
    e.mem.set_u32(LIGHT + 0xa0, 0xffff_ffff);
    e.mem.set_u8(LIGHT + 0xa4, 10);
    e.mem.set_u8(LIGHT + 0xa5, 20);
    e.mem.set_u8(LIGHT + 0xa6, 30);
    e.mem.set_f32(LIGHT + 0xac, 1.5);
    e.mem.set_f32(LIGHT + 0xb0, 90.0);
    e.mem.set_f32(LIGHT + 0xb4, 0.25);
    e.mem.set_u32(LIGHT + 0xa8, 0x8000_0002);
    let get = |e: &mut Engine, t: i32| call(e, 0x2642, &[r(1), n(t)]).1;
    let want = [-1.0, 10.0, 20.0, 30.0, 1.5, 90.0, 0.25];
    for (t, v) in want.iter().enumerate() {
        assert_eq!(get(&mut e, t as i32), *v, "trait {t}");
    }
    assert_eq!(get(&mut e, 7), 0.0);
    let flag = |e: &mut Engine, f: i32| call(e, 0x2644, &[r(1), n(f)]).1;
    assert_eq!((flag(&mut e, 1), flag(&mut e, 31)), (1.0, 1.0));
    assert_eq!((flag(&mut e, 0), flag(&mut e, 32)), (0.0, 0.0));
    e.mem.set_u8(LIGHT + 4, 0x1f);
    assert_eq!(get(&mut e, 1), 0.0, "not a light");
}

/// The two setters wait for JIP's point light hooks: no handler.
#[test]
fn the_setters_are_not_built() {
    let e = setup();
    for opcode in [0x2643, 0x2645] {
        assert_eq!(
            crate::test_support::execute_of(&e, opcode),
            0,
            "{opcode:04x}"
        );
    }
}
