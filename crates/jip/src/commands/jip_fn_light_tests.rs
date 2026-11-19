//! Tests of the light commands, at the raw offsets the light loader (`0050CAF0`) gives: radius
//! +0xA0, colour +0xA4..+0xA6, flags +0xA8, falloff +0xAC, field of view +0xB0, fade +0xB4; the
//! setters' mark on JIP's listed point lights (`resetTraits` +0x9E, their form +0xE8).

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

/// Two point lights in JIP's list: one made from the light form, one from another form.
fn listed_lights(e: &mut Engine) -> (u32, u32) {
    let mine = FORMS + 0x8000;
    let other = FORMS + 0x8400;
    e.mem.set_u32(mine + 0xe8, LIGHT);
    e.mem.set_u32(other + 0xe8, FORMS + 0x2000);
    let list = e.ext_free::<crate::game_patches::gl::lights::ActivePointLights>();
    list.0.lock().unwrap().extend([mine, other]);
    (mine, other)
}

fn marked(e: &Engine, light: u32) -> u8 {
    e.mem.u8(light + 0x9e)
}

fn clear_marks(e: &mut Engine, lights: (u32, u32)) {
    e.mem.set_u8(lights.0 + 0x9e, 0);
    e.mem.set_u8(lights.1 + 0x9e, 0);
}

/// Each trait written as JIP writes it (`(int)value` for 0 to 3, the low byte for the colour), and
/// the lights made from the form marked for 0-3 and 6, not for 4 and 5 (JIP returns before its
/// `MarkLightModified`) nor for an unknown trait; another form's light never.
#[test]
fn set_light_trait_numeric_writes_and_marks() {
    let mut e = setup();
    let lights = listed_lights(&mut e);
    let set = |e: &mut Engine, t: i32, v: f64| {
        call(e, 0x2643, &[r(1), n(t), z(v)]);
    };
    let cases: [(i32, f64, bool); 11] = [
        (0, 512.9, true),
        (0, -1.0, true),
        (0, -7.9, true),
        (1, 300.0, true),
        (2, 20.7, true),
        (3, 255.0, true),
        (3, -1.5, true),
        (4, 1.25, false),
        (5, 75.5, false),
        (6, 0.5, true),
        (7, 9.0, false),
    ];
    for (t, v, marks) in cases {
        clear_marks(&mut e, lights);
        set(&mut e, t, v);
        assert_eq!(marked(&e, lights.0), u8::from(marks), "trait {t}");
        assert_eq!(marked(&e, lights.1), 0, "another form's light, trait {t}");
        let got = match t {
            0 => f64::from(e.mem.u32(LIGHT + 0xa0) as i32),
            1..=3 => f64::from(e.mem.u8(LIGHT + 0xa3 + t as u32)),
            4 => f64::from(e.mem.f32(LIGHT + 0xac)),
            5 => f64::from(e.mem.f32(LIGHT + 0xb0)),
            6 => f64::from(e.mem.f32(LIGHT + 0xb4)),
            _ => continue,
        };
        let want = match t {
            0 => v.trunc(),
            1..=3 => f64::from(v.trunc() as i32 as u8),
            _ => v,
        };
        assert_eq!(got, want, "trait {t} value {v}");
    }
    // An unknown trait wrote nothing: the fields hold the last values.
    assert_eq!(e.mem.u32(LIGHT + 0xa0) as i32, -7);
    assert_eq!(e.mem.f32(LIGHT + 0xb4), 0.5);
    // Not a light: nothing written, nothing marked.
    clear_marks(&mut e, lights);
    e.mem.set_u8(LIGHT + 4, 0x1f);
    set(&mut e, 1, 7.0);
    assert_eq!(e.mem.u8(LIGHT + 0xa4), 44);
    assert_eq!(marked(&e, lights.0), 0);
}

/// A flag bit 0 to 31 set or cleared and the lights made from the form marked; bit 32: nothing.
#[test]
fn set_light_flag_sets_clears_and_marks() {
    let mut e = setup();
    let lights = listed_lights(&mut e);
    e.mem.set_u32(LIGHT + 0xa8, 0x0000_0100);
    call(&mut e, 0x2645, &[r(1), n(3), n(1)]);
    assert_eq!(e.mem.u32(LIGHT + 0xa8), 0x0000_0108);
    assert_eq!((marked(&e, lights.0), marked(&e, lights.1)), (1, 0));
    clear_marks(&mut e, lights);
    call(&mut e, 0x2645, &[r(1), n(31), n(-2)]);
    assert_eq!(
        e.mem.u32(LIGHT + 0xa8),
        0x8000_0108,
        "any number but 0 sets"
    );
    assert_eq!(marked(&e, lights.0), 1);
    call(&mut e, 0x2645, &[r(1), n(8), n(0)]);
    assert_eq!(e.mem.u32(LIGHT + 0xa8), 0x8000_0008);
    call(&mut e, 0x2645, &[r(1), n(0), n(0)]);
    assert_eq!(
        e.mem.u32(LIGHT + 0xa8),
        0x8000_0008,
        "a clear bit stays clear"
    );
    clear_marks(&mut e, lights);
    call(&mut e, 0x2645, &[r(1), n(32), n(1)]);
    assert_eq!(e.mem.u32(LIGHT + 0xa8), 0x8000_0008);
    assert_eq!(marked(&e, lights.0), 0, "bit 32: nothing");
    call(&mut e, 0x2645, &[r(1), n(-1), n(1)]);
    assert_eq!(
        e.mem.u32(LIGHT + 0xa8),
        0x8000_0008,
        "a negative bit is past 31"
    );
    assert_eq!(marked(&e, lights.0), 0);
    // Not a light: nothing written, nothing marked.
    e.mem.set_u8(LIGHT + 4, 0x1f);
    call(&mut e, 0x2645, &[r(1), n(1), n(1)]);
    assert_eq!(e.mem.u32(LIGHT + 0xa8), 0x8000_0008);
    assert_eq!(marked(&e, lights.0), 0);
}

/// Every listed light made from the form is marked, wherever it is in the list (another form's
/// light, the form's, a third form's, the form's again).
#[test]
fn the_mark_reaches_every_light_of_the_form() {
    let mut e = setup();
    let lights: Vec<u32> = (0..4).map(|i| FORMS + 0x8000 + 0x200 * i).collect();
    let forms = [FORMS + 0x2000, LIGHT, FORMS + 0x2400, LIGHT];
    for (&l, f) in lights.iter().zip(forms) {
        e.mem.set_u32(l + 0xe8, f);
    }
    e.ext_free::<crate::game_patches::gl::lights::ActivePointLights>()
        .0
        .lock()
        .unwrap()
        .extend(lights.iter().copied());
    let marks = |e: &Engine| -> Vec<u8> { lights.iter().map(|&l| marked(e, l)).collect() };
    call(&mut e, 0x2645, &[r(1), n(2), n(1)]);
    assert_eq!(marks(&e), vec![0, 1, 0, 1], "SetLightFlag");
    for &l in &lights {
        e.mem.set_u8(l + 0x9e, 0);
    }
    call(&mut e, 0x2643, &[r(1), n(0), z(100.0)]);
    assert_eq!(marks(&e), vec![0, 1, 0, 1], "SetLightTraitNumeric");
}

/// The getters answer what the setters wrote at once.
#[test]
fn a_set_value_reads_back() {
    let mut e = setup();
    call(&mut e, 0x2643, &[r(1), n(0), z(640.0)]);
    call(&mut e, 0x2643, &[r(1), n(6), z(0.75)]);
    call(&mut e, 0x2645, &[r(1), n(5), n(1)]);
    assert_eq!(call(&mut e, 0x2642, &[r(1), n(0)]).1, 640.0);
    assert_eq!(call(&mut e, 0x2642, &[r(1), n(6)]).1, 0.75);
    assert_eq!(call(&mut e, 0x2644, &[r(1), n(5)]).1, 1.0);
}
