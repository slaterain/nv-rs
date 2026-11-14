//! Tests of `PushActorNoRagdoll`: JIP's sine and cosine, the push and its force. Layouts at
//! literal offsets: an actor's vtable `+0` (its `+0x100` answering true: an actor), process
//! `+0x68`, position `+0x30`, life state `+0x108`, value owner `+0xA4`; a process's level
//! `+0x28`, knockdown word `+0x140` and character controller `+0x138`; a controller's push
//! vector `+0x500`, time `+0x524` and JIP's push bit (`+0x417` bit 7).

use super::*;
use crate::test_support::{engine, form_arg, line, script_ref, RESULT};

const AREA: u32 = 0x0500_0000;
const ACTOR: u32 = AREA + 0x1000;
const OTHER: u32 = AREA + 0x1400;
const PROCESS: u32 = AREA + 0x2000;
const CONTROLLER: u32 = AREA + 0x3000;
const ACTOR_VT: u32 = AREA + 0x900;

fn world() -> Engine {
    let mut e = engine();
    e.map(AREA, 0x1_0000);
    e.map(0x011c_e000, 0x2000);
    e.mem.set_u32(ACTOR_VT + 0x100, 0x008d_0360);
    for actor in [ACTOR, OTHER] {
        e.mem.set_u32(actor, ACTOR_VT);
        e.mem.set_u8(actor + 4, 0x3b);
    }
    e.mem.set_u32(ACTOR + 0x68, PROCESS);
    e.mem.set_u32(PROCESS + 0x138, CONTROLLER);
    e.mem.set_f32(SETTING_SCALE, 0.5);
    e.mem.set_f32(SETTING_BASE, 1.0);
    e.mem.set_f32(SETTING_OFFSET, 2.0);
    e.mem.set_f32(SETTING_MULT, 3.0);
    e.register(ACTOR_VALUE, |_, w| {
        assert_eq!(w, [ACTOR + 0xa4, 0xa]);
        Ret {
            st0: 6.0,
            ..Ret::default()
        }
    });
    crate::install_as(&mut e, true);
    script_ref(&mut e, 1, OTHER);
    script_ref(&mut e, 2, ACTOR);
    e
}

fn float_arg(v: f32) -> Vec<u8> {
    let mut b = vec![b'z'];
    b.extend_from_slice(&f64::from(v).to_le_bytes());
    b
}

fn int(v: i32) -> Vec<u8> {
    crate::test_support::int_arg(v)
}

fn push(e: &mut Engine, args: &[Vec<u8>]) {
    let mut words = line(e, crate::world_hooks::PUSH_ACTOR_NO_RAGDOLL, args);
    words[2] = ACTOR;
    let at = crate::test_support::execute_of(e, crate::world_hooks::PUSH_ACTOR_NO_RAGDOLL);
    assert_eq!(e.call(at, &words).eax & 0xff, 1);
    assert_eq!(e.mem.f64(RESULT), 0.0);
}

fn vector(e: &Engine) -> [f32; 4] {
    std::array::from_fn(|i| e.mem.f32(CONTROLLER + 0x500 + 4 * i as u32))
}

/// JIP's sine and cosine stay within a few single-precision steps of the true values, in every
/// quarter and past a whole turn (the reduction by turns).
#[test]
fn sin_cos_follow_the_true_values() {
    for degrees in [
        0.0f32, 30.0, 90.0, 135.0, 200.0, 290.0, 359.0, 400.0, -45.0, 1000.0,
    ] {
        let r = degrees.to_radians();
        let [s, c, z, _] = sin_cos(r);
        assert!((s - r.sin()).abs() < 2e-6, "sin {degrees}: {s}");
        assert!((c - r.cos()).abs() < 2e-6, "cos {degrees}: {c}");
        assert_eq!(z.to_bits(), COS_CONSTS[4], "cos 0 is the last constant");
    }
    // A NaN stays one.
    assert!(sin_cos(f32::NAN)[1].is_nan());
}

/// The force adjusted (`(6 * 10 * 0.5 + 1) * (f * 3 + copysign(2, f))`), the push along the
/// angle, its time, JIP's bit and the hook switched on.
#[test]
fn a_push_by_angle() {
    let mut e = world();
    push(&mut e, &[float_arg(1.0), float_arg(90.0)]);
    assert_eq!(e.hook_users(Hook::ApplyActorVelocity), 1);
    let force = (6.0f32 * 10.0 * 0.5 + 1.0) * (1.0f32 * 3.0 + 2.0);
    let time = force * f32::from_bits(PUSH_TIME);
    let speed = force * f32::from_bits(PUSH_SCALE) * time;
    let dir = sin_cos(90.0f32 * f32::from_bits(PI_D180));
    assert_eq!(vector(&e), [dir[0] * speed, dir[1] * speed, 0.0, 0.0]);
    assert_eq!(e.mem.f32(CONTROLLER + 0x524), time);
    assert_eq!(e.mem.u8(CONTROLLER + 0x417), 0x80);
    // A negative force: the offset with its sign, the time from its size.
    push(&mut e, &[float_arg(-1.0), float_arg(0.0)]);
    let force = 31.0f32 * (-3.0 - 2.0);
    assert_eq!(
        e.mem.f32(CONTROLLER + 0x524),
        -force * f32::from_bits(PUSH_TIME)
    );
    // Not adjusted: the force as given.
    push(
        &mut e,
        &[float_arg(2.0), float_arg(0.0), form_arg(2), int(0)],
    );
    assert_eq!(
        e.mem.f32(CONTROLLER + 0x524),
        2.0 * f32::from_bits(PUSH_TIME)
    );
    // No force: the push cleared.
    e.mem.set_u8(CONTROLLER + 0x417, 0x81);
    push(
        &mut e,
        &[float_arg(0.0), float_arg(0.0), form_arg(2), int(0)],
    );
    assert_eq!(vector(&e), [0.0; 4]);
    assert_eq!(e.mem.u32(CONTROLLER + 0x524), 0);
    assert_eq!(e.mem.u8(CONTROLLER + 0x417), 0x81, "the bit kept");
}

/// Away from another reference (its own self is no reference): the unit vector by `rsqrtss`;
/// none when it stands within epsilon.
#[test]
fn a_push_away_from_a_reference() {
    let mut e = world();
    e.mem.set_f32(ACTOR + 0x30, 3.0);
    e.mem.set_f32(ACTOR + 0x34, 4.0);
    push(
        &mut e,
        &[float_arg(1.0), float_arg(0.0), form_arg(1), int(0)],
    );
    let r = rsqrt(25.0);
    let time = f32::from_bits(PUSH_TIME);
    let speed = f32::from_bits(PUSH_SCALE) * time;
    assert_eq!(vector(&e), [3.0 * r * speed, 4.0 * r * speed, 0.0, 0.0]);
    assert!((r - 0.2).abs() < 1e-3, "an approximation of 1/5");
    // Standing on it: nothing changes.
    e.mem.set_f32(OTHER + 0x30, 3.0);
    e.mem.set_f32(OTHER + 0x34, 4.0);
    e.mem.set_u32(CONTROLLER + 0x524, 0);
    push(
        &mut e,
        &[float_arg(1.0), float_arg(0.0), form_arg(1), int(0)],
    );
    assert_eq!(e.mem.u32(CONTROLLER + 0x524), 0);
    // The actor itself: by the angle instead.
    push(
        &mut e,
        &[float_arg(1.0), float_arg(0.0), form_arg(2), int(0)],
    );
    assert_ne!(e.mem.u32(CONTROLLER + 0x524), 0);
}

/// An actor that cannot be pushed is left alone (the hook is switched on all the same).
#[test]
fn actors_that_cannot_be_pushed() {
    for (at, value) in [
        (PROCESS + 0x28, 1u32),
        (PROCESS + 0x140, 1),
        (PROCESS + 0x138, 0),
        (ACTOR + 0x108, 1),
        (ACTOR + 0x108, 2),
        (ACTOR + 0x68, 0),
    ] {
        let mut e = world();
        e.mem.set_u32(at, value);
        push(
            &mut e,
            &[float_arg(1.0), float_arg(0.0), form_arg(2), int(0)],
        );
        assert_eq!(e.mem.u32(CONTROLLER + 0x524), 0, "{at:08x} = {value}");
        assert_eq!(e.hook_users(Hook::ApplyActorVelocity), 1);
    }
    // Life state 3 (unconscious) can.
    let mut e = world();
    e.mem.set_u32(ACTOR + 0x108, 3);
    push(
        &mut e,
        &[float_arg(1.0), float_arg(0.0), form_arg(2), int(0)],
    );
    assert_ne!(e.mem.u32(CONTROLLER + 0x524), 0);
}
