//! `SetMoonTexture`, called by its opcode, with the moon's constructor as a stand-in that asks
//! the seam as the translation does. Expectations from JIP's `Cmd_SetMoonTexture_Execute`,
//! `InitMoonHook` and `Sky::RefreshMoon`.

use super::*;
use crate::commands::test_support::*;

const SKY_AT: u32 = FORMS + 0x1000;
const OLD_MOON: u32 = FORMS + 0x1400;
const MOON_VT: u32 = FORMS + 0x1800;
const SKY_NODE_AT: u32 = FORMS + 0x1c00;
const DESTROY_FN: u32 = 0x0dea_9000;
const REFRESH_FN: u32 = 0x0dea_9004;

fn record(addr: u32, w: &[u32]) {
    CALLS.with(|c| c.borrow_mut().push((addr, w.to_vec())));
}

/// The moon constructor's stand-in: records its words, gives the moon its vtable, and asks the
/// seam (JIP installed); without an answer it names the eight textures `exe0`..`exe7`.
fn constructor(e: &mut Engine, w: &[u32]) -> Ret {
    record(MOON_CONSTRUCTOR, w);
    let moon = w[0];
    e.mem.set_u32(moon, MOON_VT);
    // The eight strings constructed empty, as the exe's vector constructor does.
    for i in 0..16u32 {
        e.mem.set_u32(moon + 0x18 + 4 * i, 0);
    }
    if e.site(MOON_NAMES.addr(), &[moon]).is_none() {
        for i in 0..8u32 {
            super::super::string_set(e, moon + 0x18 + 8 * i, format!("exe{i}").as_bytes());
        }
    }
    Ret {
        eax: moon,
        ..Ret::default()
    }
}

fn world() -> Engine {
    let mut e = engine();
    for page in [0x011c_c000, 0x011d_e000] {
        if !e.mem.is_mapped(page) {
            e.map(page, 0x1000);
        }
    }
    e.mem.set_u32(SKY, SKY_AT);
    e.mem.set_u32(SKY_AT + SKY_NODE, SKY_NODE_AT);
    e.mem.set_u32(SKY_AT + SKY_MOON, OLD_MOON);
    e.mem.set_u32(OLD_MOON, MOON_VT);
    e.mem.set_u32(MOON_VT, DESTROY_FN);
    e.mem.set_u32(MOON_VT + 0x10, REFRESH_FN);
    e.register(DESTROY_FN, |_, w| {
        record(DESTROY_FN, w);
        Ret::default()
    });
    e.register(REFRESH_FN, |_, w| {
        record(REFRESH_FN, w);
        Ret::default()
    });
    e.register(MOON_CONSTRUCTOR, constructor);
    for (at, v) in [
        (F_MASSER_ANGLE_FADE_START, 1.0f32.to_bits()),
        (F_MASSER_ANGLE_FADE_END, 2.0f32.to_bits()),
        (F_MASSER_ANGLE_SHADOW_EARLY_FADE, 3.0f32.to_bits()),
        (F_MASSER_SPEED, 4.0f32.to_bits()),
        (F_MASSER_Z_OFFSET, 5.0f32.to_bits()),
        (I_MASSER_SIZE, 6),
    ] {
        e.mem.set_u32(at, v);
    }
    e
}

/// The moon's names, 0 for a phase with no text buffer.
fn names(e: &Engine, moon: u32) -> Vec<Vec<u8>> {
    (0..8u32)
        .map(|i| {
            let data = e.mem.u32(moon + 0x18 + 8 * i);
            if data == 0 {
                b"-".to_vec()
            } else {
                e.mem.cstr(data).to_vec()
            }
        })
        .collect()
}

#[test]
fn a_phase_replaced_and_the_moon_rebuilt() {
    let mut e = world();
    let (_, result) = call(&mut e, 0x27a4, &[n(2), s(b"textures\\sky\\mine.dds")]);
    assert_eq!(result, 1.0);
    let moon = e.mem.u32(SKY_AT + SKY_MOON);
    assert_ne!(moon, OLD_MOON);
    let calls: Vec<_> = calls()
        .into_iter()
        .filter(|c| [DESTROY_FN, MOON_CONSTRUCTOR, REFRESH_FN].contains(&c.0))
        .collect();
    assert_eq!(calls[0], (DESTROY_FN, vec![OLD_MOON, 1]));
    assert_eq!(
        calls[1],
        (
            MOON_CONSTRUCTOR,
            vec![
                moon,
                MOON_PREFIX,
                1.0f32.to_bits(),
                2.0f32.to_bits(),
                3.0f32.to_bits(),
                4.0f32.to_bits(),
                5.0f32.to_bits(),
                6
            ]
        )
    );
    assert_eq!(calls[2], (REFRESH_FN, vec![moon, SKY_NODE_AT, MOON_PREFIX]));
    let mut want: Vec<Vec<u8>> = DEFAULTS
        .iter()
        .map(|d| d.map_or(b"-".to_vec(), <[u8]>::to_vec))
        .collect();
    want[2] = b"textures\\sky\\mine.dds".to_vec();
    assert_eq!(names(&e, moon), want);
    // A second call keeps the first one's phase; outside the command the seam answers nothing.
    call(&mut e, 0x27a4, &[n(4), s(b"new.dds")]);
    let moon = e.mem.u32(SKY_AT + SKY_MOON);
    want[4] = b"new.dds".to_vec();
    assert_eq!(names(&e, moon), want);
    assert_eq!(e.site(MOON_NAMES.addr(), &[moon]), None);
}

/// A phase past 7 or no sky: nothing.
#[test]
fn nothing_without_a_sky_or_past_seven() {
    let mut e = world();
    e.mem.set_f64(RESULT, 0.0);
    assert_eq!(call(&mut e, 0x27a4, &[n(8), s(b"x.dds")]).1, 0.0);
    e.mem.set_u32(SKY, 0);
    assert_eq!(call(&mut e, 0x27a4, &[n(1), s(b"x.dds")]).1, 0.0);
    assert!(calls().is_empty());
}
