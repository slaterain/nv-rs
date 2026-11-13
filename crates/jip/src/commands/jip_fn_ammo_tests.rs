//! Tests of the ammo commands. Fields are written and read at their raw offsets (JIP's header,
//! confirmed by the ammo loader `00502CD0`: speed +0xA8, clip rounds +0x84, projectiles per shot
//! +0xB0, projectile +0xB4, casing +0xBC, casing chance +0xC0), not through the layouts. Casts as
//! MSVC compiles JIP's source: `(int)val` toward zero, `INT_MIN` past the range or for NaN.

use super::*;
use crate::commands::test_support::*;

const AMMO: u32 = FORMS + 0x1000;
const PROJ: u32 = FORMS + 0x2000;
const MISC: u32 = FORMS + 0x2100;
const CLIMATE: u32 = FORMS + 0x2200;

fn setup() -> Engine {
    let mut e = engine();
    form(&mut e, AMMO, 0x29, 0x0000_a330);
    e.mem.set_u32(AMMO, 0x0102_6064);
    form(&mut e, PROJ, 0x33, 0x0000_0333);
    form(&mut e, MISC, 0x1f, 0x0000_01f1);
    form(&mut e, CLIMATE, 0x36, 0x0000_0036);
    for (i, f) in [AMMO, PROJ, MISC, CLIMATE].iter().enumerate() {
        set_ref(&mut e, i as u16 + 1, *f);
    }
    e
}

#[test]
fn the_numeric_traits_at_their_offsets() {
    let mut e = setup();
    e.mem.set_f32(AMMO + 0xa8, 1500.5);
    e.mem.set_u32(AMMO + 0xb0, 0xffff_ffff);
    e.mem.set_f32(AMMO + 0xc0, 0.25);
    e.mem.set_u8(AMMO + 0x84, 30);
    let get = |e: &mut Engine, t: i32| call(e, 0x260b, &[r(1), n(t)]).1;
    assert_eq!(get(&mut e, 0), 1500.5);
    assert_eq!(get(&mut e, 1), -1.0, "read as an int");
    assert_eq!(get(&mut e, 2), 0.25);
    assert_eq!(get(&mut e, 3), 30.0);
    assert_eq!(get(&mut e, 4), 0.0);
    // The type is the vtable's: an ammo form type with another vtable is no ammo.
    e.mem.set_u32(AMMO, 0x0102_6068);
    assert_eq!(get(&mut e, 0), 0.0);
}

#[test]
fn the_numeric_traits_set_with_jips_casts() {
    let mut e = setup();
    let set = |e: &mut Engine, t: i32, v: f64| {
        call(e, 0x260c, &[r(1), n(t), z(v)]);
    };
    set(&mut e, 0, 12.5);
    assert_eq!(e.mem.f32(AMMO + 0xa8), 12.5);
    // projPerShot = (int)val.
    set(&mut e, 1, -2.7);
    assert_eq!(e.mem.u32(AMMO + 0xb0), (-2i32) as u32);
    set(&mut e, 1, 3e9);
    assert_eq!(
        e.mem.u32(AMMO + 0xb0),
        0x8000_0000,
        "past the range: INT_MIN"
    );
    set(&mut e, 2, 0.75);
    assert_eq!(e.mem.f32(AMMO + 0xc0), 0.75);
    // clipRounds (a byte) = val: (int) then the low byte.
    set(&mut e, 3, 300.5);
    assert_eq!(e.mem.u8(AMMO + 0x84), 44, "300 & 0xFF");
    set(&mut e, 3, -1.5);
    assert_eq!(e.mem.u8(AMMO + 0x84), 0xff, "-1's low byte");
    set(&mut e, 3, f64::NAN);
    assert_eq!(e.mem.u8(AMMO + 0x84), 0, "NaN: INT_MIN's low byte");
    let before = e.mem.bytes(AMMO, 0xdc);
    set(&mut e, 4, 1.0);
    assert_eq!(e.mem.bytes(AMMO, 0xdc), before, "trait 4: nothing");
}

#[test]
fn the_projectile() {
    let mut e = setup();
    call(&mut e, 0x260d, &[r(1)]);
    assert_eq!(form_result(&e), 0);
    call(&mut e, 0x260e, &[r(1), r(2)]);
    assert_eq!(e.mem.u32(AMMO + 0xb4), PROJ);
    call(&mut e, 0x260d, &[r(1)]);
    assert_eq!(form_result(&e), 0x0333);
    call(&mut e, 0x260e, &[r(1), r(3)]);
    assert_eq!(e.mem.u32(AMMO + 0xb4), PROJ, "a misc item is no projectile");
    call(&mut e, 0x260e, &[r(1)]);
    assert_eq!(e.mem.u32(AMMO + 0xb4), 0);
}

#[test]
fn the_casing() {
    let mut e = setup();
    call(&mut e, 0x2939, &[r(1), r(3)]);
    assert_eq!(e.mem.u32(AMMO + 0xbc), MISC);
    call(&mut e, 0x2939, &[r(1), r(4)]);
    assert_eq!(
        e.mem.u32(AMMO + 0xbc),
        MISC,
        "a climate is no inventory item"
    );
    call(&mut e, 0x2939, &[r(1)]);
    assert_eq!(e.mem.u32(AMMO + 0xbc), 0);
    call(&mut e, 0x2939, &[r(4), r(3)]);
    assert_eq!(e.mem.u32(CLIMATE + 0xbc), 0, "not an ammo");
}
