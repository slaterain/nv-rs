//! Tests of the water commands, at the raw offsets of the water loader (`0057EF80`: noise texture
//! +0x64, its path +0x68) and JIP's header (sound +0x7C, drink effect +0x178).

use super::*;
use crate::commands::test_support::*;

const WATER: u32 = FORMS + 0x1000;
const SPELL: u32 = FORMS + 0x2000;
const SOUND: u32 = FORMS + 0x2100;

fn setup() -> Engine {
    let mut e = engine();
    form(&mut e, WATER, 0x4e, 0x0000_4a7e);
    form(&mut e, SPELL, 0x14, 0x0000_0514);
    form(&mut e, SOUND, 0x0d, 0x0000_050d);
    for (i, f) in [WATER, SPELL, SOUND].iter().enumerate() {
        set_ref(&mut e, i as u16 + 1, *f);
    }
    e
}

#[test]
fn the_noise_texture() {
    let mut e = setup();
    string(&mut e, WATER + 0x68, Some(b"water\\noise.dds"));
    let (_, res) = call(&mut e, 0x27c3, &[r(1)]);
    assert_eq!(string_result(&e, res), b"water\\noise.dds");
    call(&mut e, 0x27c4, &[r(1), s(b"n.dds")]);
    assert_eq!(text_at(&e, WATER + 0x68), b"n.dds");
    let (_, res) = call(&mut e, 0x27c3, &[r(2)]);
    assert_eq!(string_result(&e, res), b"", "not a water form");
}

#[test]
fn the_effect_and_the_sound() {
    let mut e = setup();
    call(&mut e, 0x27c5, &[r(1), r(2)]);
    assert_eq!(e.mem.u32(WATER + 0x178), SPELL);
    call(&mut e, 0x27c5, &[r(1), r(3)]);
    assert_eq!(e.mem.u32(WATER + 0x178), SPELL, "a sound is no spell");
    call(&mut e, 0x27c5, &[r(1)]);
    assert_eq!(e.mem.u32(WATER + 0x178), 0);
    call(&mut e, 0x27c6, &[r(1)]);
    assert_eq!(form_result(&e), 0);
    call(&mut e, 0x27c7, &[r(1), r(3)]);
    assert_eq!(e.mem.u32(WATER + 0x7c), SOUND);
    call(&mut e, 0x27c6, &[r(1)]);
    assert_eq!(form_result(&e), 0x050d);
    call(&mut e, 0x27c7, &[r(1), r(2)]);
    assert_eq!(e.mem.u32(WATER + 0x7c), SOUND, "a spell is no sound");
    call(&mut e, 0x27c7, &[r(1)]);
    assert_eq!(e.mem.u32(WATER + 0x7c), 0);
}
