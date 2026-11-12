//! Tests of the weather commands. The weathers are built at `TESWeather`'s offsets as JIP's
//! header and `TESWeather::Load` (`00580990`) place them (image space modifiers +0x1C, cloud
//! textures +0x34 in 0xC steps, cloud speeds +0x64, cloud colours +0x68, model +0xC8, `DATA`
//! +0xE0, fog +0xF0, colours +0x108); the sky at the engine's `Sky` offsets. Expected answers are
//! worked out from JIP's C++.

use super::*;
use crate::commands::test_support::*;

const WEATHER: u32 = FORMS + 0x1000;
const CLIMATE: u32 = FORMS + 0x2000;
const IMOD: u32 = FORMS + 0x2100;
const SKY_OBJECT: u32 = FORMS + 0x8000;
const CLOUDS_OBJECT: u32 = FORMS + 0x9000;

fn setup() -> Engine {
    let mut e = engine();
    form(&mut e, WEATHER, 0x35, 0x0000_beef);
    form(&mut e, CLIMATE, 0x36, 0x0000_c1c1);
    form(&mut e, IMOD, 0x54, 0x0000_1a0d);
    set_ref(&mut e, 1, WEATHER);
    set_ref(&mut e, 2, CLIMATE);
    set_ref(&mut e, 3, IMOD);
    e.mem.set_u32(0x011d_ea20, SKY_OBJECT);
    CALLS.with(|c| c.borrow_mut().clear());
    e
}

#[test]
fn the_image_space_modifiers() {
    let mut e = setup();
    e.mem.set_u32(WEATHER + 0x1c + 4 * 2, IMOD);
    call(&mut e, 0x2653, &[r(1), n(2)]);
    assert_eq!(form_result(&e), 0x1a0d);
    call(&mut e, 0x2653, &[r(1), n(3)]);
    assert_eq!(form_result(&e), 0, "none set");
    e.mem.set_u32(WEATHER + 0x1c + 4 * 6, IMOD);
    call(&mut e, 0x2653, &[r(1), n(6)]);
    assert_eq!(form_result(&e), 0, "time 6 is past the range");
    // A climate is not a weather: the reader refuses it.
    call(&mut e, 0x2653, &[r(2), n(2)]);
    assert_eq!(form_result(&e), 0);
    // Set and clear.
    call(&mut e, 0x2654, &[r(1), n(5), r(3)]);
    assert_eq!(e.mem.u32(WEATHER + 0x1c + 4 * 5), IMOD);
    call(&mut e, 0x2654, &[r(1), n(5)]);
    assert_eq!(e.mem.u32(WEATHER + 0x1c + 4 * 5), 0);
    e.mem.set_u32(WEATHER + 0x1c + 4 * 6, 0x1234);
    call(&mut e, 0x2654, &[r(1), n(6), r(3)]);
    assert_eq!(e.mem.u32(WEATHER + 0x1c + 4 * 6), 0x1234, "time 6: nothing");
}

#[test]
fn the_cloud_textures() {
    let mut e = setup();
    string(&mut e, WEATHER + 0x34 + 0xc * 3 + 4, Some(b"sky\\clouds3.dds"));
    let (_, res) = call(&mut e, 0x2655, &[r(1), n(3)]);
    assert_eq!(string_result(&e, res), b"sky\\clouds3.dds");
    let (_, res) = call(&mut e, 0x2655, &[r(1), n(4)]);
    assert_eq!(string_result(&e, res), b"", "layer 4 is past the range");
    let (_, res) = call(&mut e, 0x2655, &[r(1), n(0)]);
    assert_eq!(string_result(&e, res), b"", "no buffer");
    call(&mut e, 0x2656, &[r(1), n(0), s(b"sky\\new.dds")]);
    assert_eq!(text_at(&e, WEATHER + 0x34 + 4), b"sky\\new.dds");
    call(&mut e, 0x2656, &[r(1), n(4), s(b"x")]);
    assert_eq!(e.mem.u32(WEATHER + 0x34 + 0xc * 4 + 4), 0, "layer 4: nothing");
}

#[test]
fn the_precipitation_model() {
    let mut e = setup();
    model(&mut e, WEATHER + 0xc8, Some(b"rain.nif"));
    let (_, res) = call(&mut e, 0x2657, &[r(1)]);
    assert_eq!(string_result(&e, res), b"rain.nif");
    call(&mut e, 0x2658, &[r(1), s(b"snow.nif")]);
    let set = calls()
        .into_iter()
        .find(|c| c.0 == 0x0048_9100)
        .expect("SetModelPath");
    assert_eq!(set.1[0], WEATHER + 0xc8);
    assert_eq!(text_at(&e, WEATHER + 0xcc), b"snow.nif");
    let (_, res) = call(&mut e, 0x2657, &[r(2)]);
    assert_eq!(string_result(&e, res), b"", "not a weather");
}

#[test]
fn the_numeric_traits() {
    let mut e = setup();
    e.mem.set_u8(WEATHER + 0x65, 51);
    e.mem.set_u8(WEATHER + 0xe0, 255);
    e.mem.set_u8(WEATHER + 0xe3, 30);
    e.mem.set_u8(WEATHER + 0xe4, 0x80);
    e.mem.set_u8(WEATHER + 0xea, 0x33);
    e.mem.set_u8(WEATHER + 0xeb, 4);
    e.mem.set_u32(WEATHER + 0xec, 0x00ff_8040);
    e.mem.set_f32(WEATHER + 0xf0, 1000.5);
    e.mem.set_f32(WEATHER + 0x104, -7.25);
    let get = |e: &mut Engine, t: i32| call(e, 0x2659, &[r(1), n(t)]).1;
    assert_eq!(get(&mut e, 1), 51.0 * (1.0 / 2550.0));
    assert_eq!(get(&mut e, 0), 0.0);
    assert_eq!(get(&mut e, 4), 255.0 * (1.0 / 255.0));
    assert_eq!(get(&mut e, 5), 30.0 * 0.001);
    assert_eq!(get(&mut e, 6), 128.0 * (1.0 / 255.0), "+0xE4");
    assert_eq!(get(&mut e, 12), 51.0 * (1.0 / 255.0), "+0xEA");
    assert_eq!(get(&mut e, 13), 4.0);
    // 0x40 red, 0x80 green, 0xFF blue: 064 128 255.
    assert_eq!(get(&mut e, 14), 64_128_255.0);
    assert_eq!(get(&mut e, 15), 1000.5);
    assert_eq!(get(&mut e, 20), -7.25);
    assert_eq!(get(&mut e, 21), 0.0, "past the range");
    assert_eq!(call(&mut e, 0x2659, &[r(2), n(13)]).1, 0.0, "not a weather");
}

#[test]
fn the_numeric_traits_set() {
    let mut e = setup();
    let set = |e: &mut Engine, t: i32, v: f64| {
        call(e, 0x265a, &[r(1), n(t), z(v)]);
    };
    set(&mut e, 2, 0.04);
    assert_eq!(e.mem.u8(WEATHER + 0x66), 102);
    set(&mut e, 3, 0.2);
    assert_eq!(e.mem.u8(WEATHER + 0x67), 255, "above 0.1");
    // -0.01 * 2550 = -25.5: -25 as an integer, its low byte 0xE7.
    set(&mut e, 0, -0.01);
    assert_eq!(e.mem.u8(WEATHER + 0x64), 0xe7);
    set(&mut e, 4, 0.5);
    assert_eq!(e.mem.u8(WEATHER + 0xe0), 127);
    set(&mut e, 5, 0.1);
    assert_eq!(e.mem.u8(WEATHER + 0xe3), 100);
    set(&mut e, 5, 0.3);
    assert_eq!(e.mem.u8(WEATHER + 0xe3), 255, "above 0.25");
    set(&mut e, 8, 0.5);
    assert_eq!(e.mem.u8(WEATHER + 0xe6), 127, "trait 8: +0xE6");
    set(&mut e, 12, 2.0);
    assert_eq!(e.mem.u8(WEATHER + 0xea), 255);
    set(&mut e, 13, 8.9);
    assert_eq!(e.mem.u8(WEATHER + 0xeb), 8);
    set(&mut e, 13, 6.0);
    assert_eq!(e.mem.u8(WEATHER + 0xeb), 8, "6 is no power of two");
    set(&mut e, 13, 0.0);
    assert_eq!(e.mem.u8(WEATHER + 0xeb), 0);
    set(&mut e, 14, 64_128_255.0);
    assert_eq!(e.mem.u32(WEATHER + 0xec), 0x00ff_8040);
    set(&mut e, 14, 255_255_256.0);
    assert_eq!(e.mem.u32(WEATHER + 0xec), 0x00ff_8040, "past 255255255");
    set(&mut e, 14, -1.0);
    assert_eq!(e.mem.u32(WEATHER + 0xec), 0x00ff_8040, "-1 is 0xFFFFFFFF");
    set(&mut e, 17, 1234.5);
    assert_eq!(e.mem.f32(WEATHER + 0xf8), 1234.5);
    let before = e.mem.bytes(WEATHER, 0x120);
    set(&mut e, 21, 1.0);
    assert_eq!(e.mem.bytes(WEATHER, 0x120), before, "trait 21: nothing");
}

#[test]
fn the_colours() {
    let mut e = setup();
    // colors[3][4] at +0x108 + (3 * 6 + 4) * 4; cloudColor[2][1] at +0x68 + (2 * 6 + 1) * 4.
    e.mem.set_u32(WEATHER + 0x108 + 22 * 4, 0x0003_0201);
    e.mem.set_u32(WEATHER + 0x68 + 13 * 4, 0x0006_0504);
    let get = |e: &mut Engine, args: &[Vec<u8>]| call(e, 0x265b, args).1;
    assert_eq!(get(&mut e, &[r(1), n(3), n(4)]), 1_002_003.0);
    assert_eq!(get(&mut e, &[r(1), n(2), n(1), n(2)]), 4_005_006.0);
    assert_eq!(get(&mut e, &[r(1), n(2), n(1), n(4)]), 0.0, "layer 4");
    assert_eq!(get(&mut e, &[r(1), n(10), n(1)]), 0.0, "kind 10");
    assert_eq!(get(&mut e, &[r(1), n(3), n(6)]), 0.0, "time 6");
    // Set.
    call(&mut e, 0x265c, &[r(1), n(9), n(5), n(255_000_001)]);
    assert_eq!(e.mem.u32(WEATHER + 0x108 + 59 * 4), 0x0001_00ff);
    call(&mut e, 0x265c, &[r(1), n(2), n(0), n(1_002_003), n(3)]);
    assert_eq!(e.mem.u32(WEATHER + 0x68 + 18 * 4), 0x0003_0201);
    call(&mut e, 0x265c, &[r(1), n(1), n(0), n(255_255_256)]);
    assert_eq!(e.mem.u32(WEATHER + 0x108 + 6 * 4), 0, "past 255255255");
    call(&mut e, 0x265c, &[r(1), n(2), n(0), n(1), n(4)]);
    assert_eq!(e.mem.u32(WEATHER + 0x68 + 24 * 4), 0, "layer 4");
}

#[test]
fn the_sky_now() {
    let mut e = setup();
    call(&mut e, 0x26cf, &[]);
    assert_eq!(form_result(&e), 0);
    e.mem.set_u32(SKY_OBJECT + 0x10, WEATHER);
    call(&mut e, 0x26cf, &[]);
    assert_eq!(form_result(&e), 0xbeef);
    e.mem.set_f32(SKY_OBJECT + 0xd0, 0.5);
    let (_, res) = call(&mut e, 0x2855, &[]);
    assert_eq!(res, 0.5 * -57.295_779_513_082_32);
    call(&mut e, 0x2856, &[z(90.0)]);
    assert_eq!(
        e.mem.f32(SKY_OBJECT + 0xd0),
        90.0f32 * -0.017_453_292_38f32
    );
}

#[test]
fn lightning_only_in_rain() {
    let mut e = setup();
    let flash = |e: &mut Engine| {
        e.mem.set_f32(SKY_OBJECT + 0x100, 0.0);
        call(e, 0x2859, &[]);
        e.mem.f32(SKY_OBJECT + 0x100)
    };
    assert_eq!(flash(&mut e), 0.0, "no weather");
    e.mem.set_u32(SKY_OBJECT + 0x10, WEATHER);
    e.mem.set_u8(WEATHER + 0xeb, 4);
    e.mem.set_f32(SKY_OBJECT + 0xf4, 1.0);
    assert_eq!(flash(&mut e), 1.0);
    e.mem.set_f32(SKY_OBJECT + 0xf4, 0.5);
    assert_eq!(flash(&mut e), 0.0, "the rain not in full yet");
    e.mem.set_f32(SKY_OBJECT + 0xf4, f32::NAN);
    assert_eq!(flash(&mut e), 1.0, "NaN compares unordered: raining");
    // The weather fading out counts when the current one is not rain.
    let other = FORMS + 0x3000;
    form(&mut e, other, 0x35, 1);
    e.mem.set_u8(other + 0xeb, 2);
    e.mem.set_u32(SKY_OBJECT + 0x10, other);
    e.mem.set_u32(SKY_OBJECT + 0x14, WEATHER);
    e.mem.set_f32(SKY_OBJECT + 0xf4, 2.0);
    assert_eq!(flash(&mut e), 1.0);
    e.mem.set_u32(SKY_OBJECT + 0x14, 0);
    assert_eq!(flash(&mut e), 0.0);
}

#[test]
fn the_clouds() {
    let mut e = setup();
    for i in 0..5 {
        e.mem.set_u32(0x011f_f8b4 + 4 * i, 0x3f80_0000);
    }
    call(&mut e, 0x287e, &[]);
    for i in 0..4 {
        assert_eq!(e.mem.u32(0x011f_f8b4 + 4 * i), 0);
    }
    assert_eq!(e.mem.u32(0x011f_f8c4), 0x3f80_0000, "16 bytes only");
    // No clouds: nothing called.
    call(&mut e, 0x28cd, &[]);
    assert!(calls().is_empty());
    e.mem.set_u32(SKY_OBJECT + 0x2c, CLOUDS_OBJECT);
    e.mem.set_u32(CLOUDS_OBJECT, CLOUDS_VTBL);
    call(&mut e, 0x28cd, &[]);
    assert_eq!(e.mem.u8(CLOUDS_OBJECT + 0x5a), 1);
    assert_eq!(
        calls(),
        vec![(0x0063_4110, vec![CLOUDS_OBJECT, SKY_OBJECT, 0])]
    );
}
