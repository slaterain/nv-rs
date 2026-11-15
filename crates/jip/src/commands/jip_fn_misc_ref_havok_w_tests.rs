//! Tests of the Havok write commands and the ray cast's material (jip_fn_misc_ref_havok_w.rs), on
//! the scenes of the read commands' tests (this module sits under them to share them). The game
//! functions JIP calls are recording stand-ins; every expectation is taken from JIP's command
//! bodies and helpers (functions_jip/jip_fn_misc_ref.h, internal/havok.cpp,
//! internal/netimmerse.cpp, internal/jip_core.cpp, internal/utility.cpp).

use super::*;
use crate::commands::jip_fn_misc_ref_havok_w as wm;
use crate::commands::test_support::{calls_to, CALLS};

const WORLD_VTBL: u32 = AREA + 0xf_1000;
const CREATURE_VTBL: u32 = AREA + 0xf_1400;
const SET_MOTION_FN: u32 = 0x00de_10e4;
const UPDATE_FILTER_FN: u32 = 0x00de_10c4;
const CENTER_FN: u32 = 0x00de_10f0;
const IS_CHARACTER_TRUE: u32 = 0x00de_1218;
const IS_CHARACTER_FALSE: u32 = 0x00de_1219;
const CENTER: u32 = AREA + 0xe_8000;
const LAND_ANSWER: u32 = AREA + 0xe_8100;

fn log(addr: u32, words: &[u32]) {
    CALLS.with(|c| c.borrow_mut().push((addr, words.to_vec())));
}

fn answer(v: u32) -> Ret {
    Ret {
        eax: v,
        ..Ret::default()
    }
}

/// The read tests' scene set up, with the game functions JIP's writes call as recorders: the
/// activation, `SetMotionType` and the filter update on [`WORLD_VTBL`] objects, the centre of
/// mass (the vector at [`CENTER`] copied out, `out` answered), the land lookup (answering the
/// word at [`LAND_ANSWER`]) and `IsCharacter` (true on the actor vtable, false on
/// [`CREATURE_VTBL`]).
fn wsetup() -> Engine {
    let mut e = setup();
    CALLS.with(|c| c.borrow_mut().clear());
    e.register(wm::ENTITY_ACTIVATE, |_, a| {
        log(wm::ENTITY_ACTIVATE, &a[..1]);
        answer(0)
    });
    e.mem.set_u32(WORLD_VTBL + 0xe4, SET_MOTION_FN);
    e.mem.set_u32(WORLD_VTBL + 0xc4, UPDATE_FILTER_FN);
    e.mem.set_u32(WORLD_VTBL + 0xf0, CENTER_FN);
    e.register(SET_MOTION_FN, |_, a| {
        log(SET_MOTION_FN, &a[..2]);
        answer(1)
    });
    e.register(UPDATE_FILTER_FN, |_, a| {
        log(UPDATE_FILTER_FN, &a[..1]);
        answer(0)
    });
    e.register(CENTER_FN, |e, a| {
        log(CENTER_FN, &a[..2]);
        for i in 0..4 {
            let v = e.mem.u32(CENTER + 4 * i);
            e.mem.set_u32(a[1] + 4 * i, v);
        }
        answer(a[1])
    });
    e.register(wm::LAND_AT, |e, a| {
        let p = [0, 4, 8, 12].map(|o| e.mem.u32(a[1] + o));
        log(wm::LAND_AT, &[a[0], p[0], p[1], p[2], p[3]]);
        answer(e.mem.u32(LAND_ANSWER))
    });
    e.mem.set_u32(ACTOR_VTBL + 0x218, IS_CHARACTER_TRUE);
    e.mem.set_u32(CREATURE_VTBL + 0x100, h::RETURN_TRUE);
    e.mem.set_u32(CREATURE_VTBL + 0x218, IS_CHARACTER_FALSE);
    e.register(IS_CHARACTER_TRUE, |_, _| answer(1));
    e.register(IS_CHARACTER_FALSE, |_, _| answer(0x100));
    e
}

/// The block's world object (made a [`WORLD_VTBL`] object) and the Havok object's owner link
/// (+0xC) to it.
fn link(e: &mut Engine, block: u32) -> u32 {
    let world = e.mem.u32(e.mem.u32(block + 0x1c) + 0x10);
    e.mem.set_u32(world, WORLD_VTBL);
    let object = e.mem.u32(world + 8);
    e.mem.set_u32(object + 0xc, world);
    world
}

fn get4(e: &Engine, at: u32) -> [f32; 4] {
    [0, 4, 8, 12].map(|o| e.mem.f32(at + o))
}

/// A plain reference whose root holds a rigid body of motion `motion` (linked to a world object)
/// with rotation columns c0 = (1, 0, 0, inf), c1 = (0, 0, 1, 0), c2 = (0, -1, 0, 0).
fn rotated(e: &mut Engine, motion: u8) -> (u32, u32, u32) {
    let (refr, b) = moving(e, motion, [10.0, 20.0, 30.0, 40.0], [1.0, 2.0, 3.0, 4.0]);
    set4(e, b + 0xf0, [1.0, 0.0, 0.0, f32::INFINITY]);
    set4(e, b + 0x100, [0.0, 0.0, 1.0, 0.0]);
    set4(e, b + 0x110, [0.0, -1.0, 0.0, 0.0]);
    let root = e.mem.u32(e.mem.u32(refr + 0x64) + 0x14);
    let world = link(e, root);
    (refr, b, world)
}

fn floats(v: [f64; 3]) -> Vec<Vec<u8>> {
    v.iter().map(|&x| z(x)).collect()
}

fn vel_args(name: &[u8], v: [f64; 3], flags: Option<i32>) -> Vec<Vec<u8>> {
    let mut a = vec![s(name)];
    a.extend(floats(v));
    a.extend(flags.map(n));
    a
}

// ---- velocities ----

#[test]
fn linear_velocity_global_local_added_and_woken() {
    let mut e = wsetup();
    let (refr, b, _) = rotated(&mut e, 2);
    // Without flags (JIP passes flags ^ 1: global): stored as given, the fourth lane 0.
    assert_eq!(
        run(&mut e, 0x28d0, refr, &vel_args(b"", [1.0, 2.0, 3.0], None)),
        1.0
    );
    assert_eq!(get4(&e, b + 0x1b0), [1.0, 2.0, 3.0, 0.0]);
    assert_eq!(calls_to(wm::ENTITY_ACTIVATE), vec![vec![b]]);
    // Flag 1 (local): (c0*x + c1*y) + c2*z lane by lane: (1, -3, 2); the fourth lane
    // (inf*0 + 0*0) + 0*0 is NaN.
    run(
        &mut e,
        0x28d0,
        refr,
        &vel_args(b"", [1.0, 2.0, 3.0], Some(1)),
    );
    let v = get4(&e, b + 0x1b0);
    assert_eq!(&v[..3], &[1.0, -3.0, 2.0]);
    assert!(v[3].is_nan());
    // Flag 2 (add, global): the stored four lanes added.
    set4(&mut e, b + 0x1b0, [10.0, 20.0, 30.0, 40.0]);
    run(
        &mut e,
        0x28d0,
        refr,
        &vel_args(b"", [1.0, 2.0, 3.0], Some(2)),
    );
    assert_eq!(get4(&e, b + 0x1b0), [11.0, 22.0, 33.0, 40.0]);
    // Flag 0x10 picks the angular velocity, as JIP's SetVelocity reads it.
    run(
        &mut e,
        0x28d0,
        refr,
        &vel_args(b"", [5.0, 6.0, 7.0], Some(0x10)),
    );
    assert_eq!(get4(&e, b + 0x1c0), [5.0, 6.0, 7.0, 0.0]);
    assert_eq!(calls_to(wm::ENTITY_ACTIVATE).len(), 4);
    // Flags above the low byte: nothing (JIP stores elsewhere), no result.
    assert_eq!(
        run(
            &mut e,
            0x28d0,
            refr,
            &vel_args(b"", [9.0, 9.0, 9.0], Some(0x100))
        ),
        0.0
    );
    assert_eq!(get4(&e, b + 0x1b0), [11.0, 22.0, 33.0, 40.0]);
    assert_eq!(calls_to(wm::ENTITY_ACTIVATE).len(), 4);
}

#[test]
fn velocity_writes_need_a_mobile_rigid_body() {
    let mut e = wsetup();
    for motion in [1u8, 4, 5] {
        let (refr, b, _) = rotated(&mut e, motion);
        assert_eq!(
            run(&mut e, 0x28d0, refr, &vel_args(b"", [1.0, 1.0, 1.0], None)),
            0.0
        );
        assert_eq!(
            run(&mut e, 0x2911, refr, &vel_args(b"", [1.0, 1.0, 1.0], None)),
            0.0
        );
        run(&mut e, 0x285c, refr, &[s(b""), axis(b'X'), z(2.0)]);
        assert_eq!(get4(&e, b + 0x1b0), [10.0, 20.0, 30.0, 40.0], "{motion}");
        assert_eq!(get4(&e, b + 0x1c0), [1.0, 2.0, 3.0, 4.0], "{motion}");
    }
    assert!(calls_to(wm::ENTITY_ACTIVATE).is_empty());
    // A named block that is missing: nothing.
    let (refr, _, _) = rotated(&mut e, 2);
    assert_eq!(
        run(
            &mut e,
            0x28d0,
            refr,
            &vel_args(b"Missing", [1.0, 1.0, 1.0], None)
        ),
        0.0
    );
    assert!(calls_to(wm::ENTITY_ACTIVATE).is_empty());
}

#[test]
fn angular_velocity_ex_local_unless_global() {
    let mut e = wsetup();
    let (refr, b, _) = rotated(&mut e, 2);
    // Without flags: turned (JIP passes flags | 0x10, bit 0 clear).
    assert_eq!(
        run(&mut e, 0x2911, refr, &vel_args(b"", [1.0, 2.0, 3.0], None)),
        1.0
    );
    let v = get4(&e, b + 0x1c0);
    assert_eq!(&v[..3], &[1.0, -3.0, 2.0]);
    assert!(v[3].is_nan());
    assert_eq!(get4(&e, b + 0x1b0), [10.0, 20.0, 30.0, 40.0]);
    // Flag 1: as given; flag 3: added.
    run(
        &mut e,
        0x2911,
        refr,
        &vel_args(b"", [1.0, 2.0, 3.0], Some(1)),
    );
    assert_eq!(get4(&e, b + 0x1c0), [1.0, 2.0, 3.0, 0.0]);
    run(
        &mut e,
        0x2911,
        refr,
        &vel_args(b"", [1.0, 2.0, 3.0], Some(3)),
    );
    assert_eq!(get4(&e, b + 0x1c0), [2.0, 4.0, 6.0, 0.0]);
    assert_eq!(calls_to(wm::ENTITY_ACTIVATE), vec![vec![b]; 3]);
}

#[test]
fn angular_velocity_is_the_axis_column_times_the_value() {
    let mut e = wsetup();
    let (refr, b, _) = rotated(&mut e, 2);
    set4(&mut e, b + 0x100, [0.5, 0.25, 2.0, f32::INFINITY]);
    run(&mut e, 0x285c, refr, &[s(b""), axis(b'Y'), z(4.0)]);
    // (v, v, v, 0) * column Y; the fourth lane 0 * inf is NaN.
    let v = get4(&e, b + 0x1c0);
    assert_eq!(&v[..3], &[2.0, 1.0, 8.0]);
    assert!(v[3].is_nan());
    assert_eq!(calls_to(wm::ENTITY_ACTIVATE), vec![vec![b]]);
    // Column X is (1, 0, 0, inf).
    run(&mut e, 0x285c, refr, &[s(b""), axis(b'X'), z(3.0)]);
    let v = get4(&e, b + 0x1c0);
    assert_eq!(&v[..3], &[3.0, 0.0, 0.0]);
    assert!(v[3].is_nan());
}

#[test]
fn angular_velocity_axis_other_than_x_y_z_does_nothing() {
    let mut e = wsetup();
    let (refr, b, _) = rotated(&mut e, 2);
    run(&mut e, 0x285c, refr, &[s(b""), axis(b'W'), z(4.0)]);
    run(&mut e, 0x285c, refr, &[s(b""), axis(b'['), z(4.0)]);
    assert_eq!(get4(&e, b + 0x1c0), [1.0, 2.0, 3.0, 4.0]);
    assert!(calls_to(wm::ENTITY_ACTIVATE).is_empty());
}

#[test]
fn multiply_inv_keeps_the_masked_lane_products() {
    let cols = [
        [2.0, 3.0, 4.0, 1.0],
        [5.0, 6.0, 7.0, 1.0],
        [8.0, 9.0, 10.0, f32::NAN],
    ];
    let v = wm::multiply_inv(cols, [1.0, 10.0, 100.0, 99.0]);
    // (c0*x + c1*y) + c2*z, by hand.
    assert_eq!(&v[..3], &[852.0, 963.0, 1074.0]);
    assert!(v[3].is_nan());
}

// ---- pushes ----

#[test]
fn normalize_is_the_newton_step_and_zero_below_epsilon() {
    // Each lane within 2^-20 of the exact quotient (the estimate's one Newton step), and not
    // always the exact quotient: some of these lanes differ in the last bits.
    let cases = [
        [3.0f32, 4.0, 0.0, 0.0],
        [1.0, 1.0, 1.0, 0.0],
        [1.0, 2.0, 2.0, 0.0],
        [2.0, 3.0, 6.0, 0.0],
        [5.0, 12.0, 0.0, 0.0],
        [1.0, 1.0, 0.0, 0.0],
        [7.0, 1.0, 3.0, 2.0],
    ];
    let mut differs = false;
    for c in cases {
        let len = f64::from((c[0] * c[0] + c[1] * c[1]) + (c[2] * c[2] + c[3] * c[3])).sqrt();
        let v = wm::normalize_v4(c);
        for i in 0..4 {
            let exact = f64::from(c[i]) / len;
            assert!(
                (f64::from(v[i]) - exact).abs() <= 2f64.powi(-20),
                "{c:?} {v:?}"
            );
            differs |= v[i] != exact as f32;
        }
    }
    assert!(differs, "the Newton step, not the exact square root");
    // Squares summed below FLT_EPSILON, or NaN: the zero vector.
    assert_eq!(wm::normalize_v4([1e-4, 0.0, 0.0, 0.0]), [0.0; 4]);
    assert_eq!(wm::normalize_v4([f32::NAN, 1.0, 0.0, 0.0]), [0.0; 4]);
    // The fourth lane counts and is scaled too.
    let q = wm::normalize_v4([0.0, 0.0, 0.0, 2.0]);
    assert!((q[3] - 1.0).abs() <= 2f32.powi(-20));
}

/// A plain reference whose root holds a mobile body (centre of mass from [`CENTER`]) and a
/// child node holding a keyframed body; both linked to world objects.
fn pushable(e: &mut Engine) -> (u32, u32, u32, u32) {
    let child = node(e, b"Child", &[]);
    let root = node(e, b"Root", &[child]);
    let refr = reference(e, REF_VTBL, 0x0100_0080, 0, root);
    let (mobile, fixed) = (body(e, 2), body(e, 4));
    attach(e, root, mobile);
    attach(e, child, fixed);
    let world = link(e, root);
    link(e, child);
    set4(e, mobile + 0x1b0, [1.0, 1.0, 1.0, 1.0]);
    for (i, v) in [100.0f32, 200.0, 300.0].iter().enumerate() {
        e.mem.set_f32(refr + 0x30 + 4 * i as u32, *v);
    }
    (refr, mobile, fixed, world)
}

#[test]
fn push_object_adds_the_direction_from_the_point_times_the_strength() {
    let mut e = wsetup();
    let (refr, mobile, fixed, world) = pushable(&mut e);
    // The centre of mass in Havok units: times PS_HKUnitCnvrt it is the point plus (3, 4, 0).
    let k = h::from_havok();
    set4(&mut e, CENTER, [103.0 / k, 204.0 / k, 300.0 / k, 7.0]);
    run(&mut e, 0x2844, refr, &[z(0.0), z(0.0), z(0.0), z(10.0)]);
    assert_eq!(calls_to(CENTER_FN).len(), 1);
    assert_eq!(calls_to(CENTER_FN)[0][0], world);
    let out = calls_to(CENTER_FN)[0][1];
    assert_eq!(out % 16, 0, "JIP's aligned buffer");
    // direction (k*c - point) normalized, times 10 * kUnitConv, added (fourth lane + 0).
    let d = [
        103.0 / k * k - 100.0,
        204.0 / k * k - 200.0,
        300.0 / k * k - 300.0,
    ];
    let len = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt();
    let s = 10.0 * h::to_havok();
    let v = get4(&e, mobile + 0x1b0);
    for i in 0..3 {
        let want = d[i] / len * s + 1.0;
        assert!((v[i] - want).abs() <= 2f32.powi(-18), "{i}: {v:?} {want}");
    }
    assert_eq!(v[3], 1.0);
    // Only the mobile body is pushed and woken.
    assert_eq!(calls_to(wm::ENTITY_ACTIVATE), vec![vec![mobile]]);
    assert_eq!(get4(&e, fixed + 0x1b0), [0.0; 4]);
}

#[test]
fn push_object_from_another_reference_and_not_for_actors() {
    let mut e = wsetup();
    let (refr, mobile, _, _) = pushable(&mut e);
    let k = h::from_havok();
    // The point is the other reference's position plus the offset: (0, 0, 0) + (1, 0, 0).
    let other = reference(&mut e, REF_VTBL, 0x0100_0081, 0, 0);
    set4(&mut e, CENTER, [1.0 / k, 5.0 / k, 0.0, 0.0]);
    set_ref(&mut e, 1, other);
    run(
        &mut e,
        0x2844,
        refr,
        &[z(1.0), z(0.0), z(0.0), z(7.0), r(1)],
    );
    let v = get4(&e, mobile + 0x1b0);
    // Direction (1/k*k - 1, 5/k*k, 0): about (0, 1, 0).
    assert!(
        (v[1] - (1.0 + 7.0 * h::to_havok())).abs() <= 2f32.powi(-16),
        "{v:?}"
    );
    assert!((v[0] - 1.0).abs() <= 2f32.powi(-16), "{v:?}");
    // An actor: nothing at all.
    CALLS.with(|c| c.borrow_mut().clear());
    let root = e.mem.u32(e.mem.u32(refr + 0x64) + 0x14);
    let actor = reference(&mut e, ACTOR_VTBL, 0x0100_0082, 0, root);
    run(&mut e, 0x2844, actor, &[z(0.0), z(0.0), z(0.0), z(10.0)]);
    assert!(calls_to(CENTER_FN).is_empty());
    assert!(calls_to(wm::ENTITY_ACTIVATE).is_empty());
    // Without 3D: nothing.
    let bare = reference(&mut e, REF_VTBL, 0x0100_0083, 0, 0);
    run(&mut e, 0x2844, bare, &[z(0.0), z(0.0), z(0.0), z(10.0)]);
    assert!(calls_to(CENTER_FN).is_empty());
}

#[test]
fn push_object_at_the_centre_itself_adds_nothing_but_still_wakes() {
    let mut e = wsetup();
    let (refr, mobile, _, _) = pushable(&mut e);
    let k = h::from_havok();
    set4(&mut e, CENTER, [100.0 / k, 200.0 / k, 300.0 / k, 0.0]);
    run(&mut e, 0x2844, refr, &[z(0.0), z(0.0), z(0.0), z(10.0)]);
    // A direction below FLT_EPSILON normalizes to 0: the velocity stays (plus 0).
    let v = get4(&e, mobile + 0x1b0);
    for x in v {
        assert!((x - 1.0).abs() <= 2f32.powi(-10), "{v:?}");
    }
    assert_eq!(calls_to(wm::ENTITY_ACTIVATE), vec![vec![mobile]]);
}

// ---- collision properties ----

#[test]
fn collision_properties_zero_to_seven_are_stores() {
    let mut e = wsetup();
    let (refr, b, _) = rotated(&mut e, 1);
    let offsets = [0x90, 0x94, 0x194, 0x198, 0x1a0, 0x1a4, 0x1a8];
    for (prop, off) in offsets.iter().enumerate() {
        run(
            &mut e,
            0x2914,
            refr,
            &[s(b""), n(prop as i32), z(1.5 + prop as f64)],
        );
        assert_eq!(e.mem.f32(b + off), 1.5 + prop as f32, "{prop}");
    }
    // The mass is stored as its inverse, 0 when not above 0.
    run(&mut e, 0x2914, refr, &[s(b""), n(7), z(4.0)]);
    assert_eq!(e.mem.f32(b + 0x1ac), 0.25);
    run(&mut e, 0x2914, refr, &[s(b""), n(7), z(-2.0)]);
    assert_eq!(e.mem.u32(b + 0x1ac), 0);
    // Property 9: nothing.
    e.mem.set_f32(b + 0x90, 7.0);
    run(&mut e, 0x2914, refr, &[s(b""), n(9), z(3.0)]);
    assert_eq!(e.mem.f32(b + 0x90), 7.0);
    assert!(calls_to(SET_MOTION_FN).is_empty());
}

#[test]
fn collision_property_eight_calls_set_motion_type_when_it_differs() {
    let mut e = wsetup();
    let (refr, b, world) = rotated(&mut e, 1);
    // Truncated: 4.9 is 4 (keyframed), passed to the owner's SetMotionType.
    run(&mut e, 0x2914, refr, &[s(b""), n(8), z(4.9)]);
    assert_eq!(calls_to(SET_MOTION_FN), vec![vec![world, 4]]);
    // The same type: no call; a type outside 1 to 7: not passed on.
    run(&mut e, 0x2914, refr, &[s(b""), n(8), z(1.0)]);
    run(&mut e, 0x2914, refr, &[s(b""), n(8), z(0.0)]);
    run(&mut e, 0x2914, refr, &[s(b""), n(8), z(8.0)]);
    run(&mut e, 0x2914, refr, &[s(b""), n(8), z(-1.0)]);
    assert_eq!(calls_to(SET_MOTION_FN).len(), 1);
    // No owner linked: nothing (JIP calls through null).
    e.mem.set_u32(b + 0xc, 0);
    run(&mut e, 0x2914, refr, &[s(b""), n(8), z(5.0)]);
    assert_eq!(calls_to(SET_MOTION_FN).len(), 1);
}

#[test]
fn collision_property_star_walks_every_rigid_body() {
    let mut e = wsetup();
    let deep = node(&mut e, b"Deep", &[]);
    let ph_node = node(&mut e, b"Ph", &[]);
    let root = node(&mut e, b"Root", &[ph_node, deep]);
    let refr = reference(&mut e, REF_VTBL, 0x0100_0090, 0, root);
    let (b1, b2, ph) = (body(&mut e, 1), body(&mut e, 1), havok(&mut e, 2));
    attach(&mut e, root, b1);
    attach(&mut e, deep, b2);
    attach(&mut e, ph_node, ph);
    run(&mut e, 0x2914, refr, &[s(b"*"), n(0), z(0.75)]);
    assert_eq!(e.mem.f32(b1 + 0x90), 0.75);
    assert_eq!(e.mem.f32(b2 + 0x90), 0.75);
    assert_eq!(e.mem.f32(ph + 0x90), 0.0, "a phantom is skipped");
    // A named block: only its body.
    run(&mut e, 0x2914, refr, &[s(b"Deep"), n(1), z(0.5)]);
    assert_eq!(e.mem.f32(b2 + 0x94), 0.5);
    assert_eq!(e.mem.f32(b1 + 0x94), 0.0);
    // With property 8 the walk asks each body's owner.
    let w1 = link(&mut e, root);
    let w2 = link(&mut e, deep);
    run(&mut e, 0x2914, refr, &[s(b"*"), n(8), z(2.0)]);
    assert_eq!(calls_to(SET_MOTION_FN), vec![vec![w1, 2], vec![w2, 2]]);
}

// ---- layers and the no-collision flag ----

#[test]
fn collision_layer_of_a_named_node_stored_then_the_filter_updated() {
    let mut e = wsetup();
    let child = node(&mut e, b"Child", &[]);
    let shape = block(&mut e, NI_TRI_SHAPE, b"Shape", &[]);
    let root = node(&mut e, b"Root", &[child, shape]);
    let refr = reference(&mut e, REF_VTBL, 0x0100_00a0, 0, root);
    let (b1, b2, b3) = (body(&mut e, 1), havok(&mut e, 2), body(&mut e, 1));
    attach(&mut e, root, b1);
    attach(&mut e, child, b2);
    attach(&mut e, shape, b3);
    let (w1, w2) = (link(&mut e, root), link(&mut e, child));
    link(&mut e, shape);
    e.mem.set_u32(b2 + 0x2c, 0x1234_5605);
    run(&mut e, 0x2916, refr, &[s(b"Child"), n(9)]);
    assert_eq!(e.mem.u32(b2 + 0x2c), 0x1234_5609);
    assert_eq!(calls_to(UPDATE_FILTER_FN), vec![vec![w2]]);
    // The same layer: no store, no update.
    run(&mut e, 0x2916, refr, &[s(b"Child"), n(9)]);
    assert_eq!(calls_to(UPDATE_FILTER_FN).len(), 1);
    // Layer 0 or 43 and up: nothing; an empty name or a block that is not a node: nothing.
    let cases: [(&[u8], i32); 4] = [(b"Child", 0), (b"Child", 43), (b"", 4), (b"Shape", 4)];
    for (name, layer) in cases {
        run(&mut e, 0x2916, refr, &[s(name), n(layer)]);
    }
    assert_eq!(calls_to(UPDATE_FILTER_FN).len(), 1);
    assert_eq!(e.mem.u8(b3 + 0x2c), 0);
    // '*': the root and every node under it (the shape is not a node).
    run(&mut e, 0x2916, refr, &[s(b"*"), n(42)]);
    assert_eq!(e.mem.u8(b1 + 0x2c), 42);
    assert_eq!(e.mem.u8(b2 + 0x2c), 42);
    assert_eq!(e.mem.u8(b3 + 0x2c), 0);
    assert_eq!(
        calls_to(UPDATE_FILTER_FN),
        vec![vec![w2], vec![w1], vec![w2]]
    );
}

#[test]
fn toggle_node_collision_sets_the_flag_with_no_filter_update() {
    let mut e = wsetup();
    let child = node(&mut e, b"Child", &[]);
    let root = node(&mut e, b"Root", &[child]);
    let refr = reference(&mut e, REF_VTBL, 0x0100_00b0, 0, root);
    let (b1, b2) = (body(&mut e, 1), body(&mut e, 1));
    attach(&mut e, root, b1);
    attach(&mut e, child, b2);
    link(&mut e, child);
    e.mem.set_u8(b2 + 0x2d, 0x81);
    run(&mut e, 0x2928, refr, &[s(b"Child"), n(0)]);
    assert_eq!(e.mem.u8(b2 + 0x2d), 0xc1);
    run(&mut e, 0x2928, refr, &[s(b"Child"), n(5)]);
    assert_eq!(e.mem.u8(b2 + 0x2d), 0x81);
    assert!(calls_to(UPDATE_FILTER_FN).is_empty());
    // An empty name: the root (GetNode's seen root).
    run(&mut e, 0x2928, refr, &[s(b""), n(0)]);
    assert_eq!(e.mem.u8(b1 + 0x2d), 0x40);
    // No world object: nothing.
    e.mem.set_u32(e.mem.u32(child + 0x1c) + 0x10, 0);
    run(&mut e, 0x2928, refr, &[s(b"Child"), n(0)]);
    assert_eq!(e.mem.u8(b2 + 0x2d), 0x81);
}

// ---- the ray cast's material ----

#[test]
fn ray_cast_material_of_nothing_land_and_a_static() {
    let mut e = wsetup();
    let tes = world(&mut e, 0x0000_0000);
    let refr = looker(&mut e);
    // Nothing hit: -1.
    assert_eq!(run(&mut e, 0x28e3, refr, &[]), -1.0);
    // A block without a reference (the land): the hit point to the land lookup, its +0x1C byte
    // converted (2: dirt, 1).
    let land_block = node(&mut e, b"Land", &[]);
    e.mem.set_u32(PICK_ANSWER, land_block);
    e.mem.set_f32(PICK_FRACTION, 0.25);
    let texture = alloc(&mut e, 0x40);
    e.mem.set_u8(texture + 0x1c, 2);
    e.mem.set_u32(LAND_ANSWER, texture);
    assert_eq!(run(&mut e, 0x28e3, refr, &[n(3)]), 1.0);
    let call = &calls_to(wm::LAND_AT)[0];
    assert_eq!(call[0], tes);
    let k = h::to_havok();
    let from = [70.0 * k, -140.0 * k, 7.0 * k];
    let to = [
        (2.0 * 50000.0) * k + from[0],
        (5.0 * 50000.0) * k + from[1],
        (8.0 * 50000.0) * k + from[2],
    ];
    for i in 0..3 {
        let want = (from[i] + (to[i] - from[i]) * 0.25) * h::from_havok();
        assert_eq!(f32::from_bits(call[1 + i]), want, "{i}");
    }
    assert_eq!(call[4], 0);
    // Material bytes past 31, or nothing found: -1.
    e.mem.set_u8(texture + 0x1c, 32);
    assert_eq!(run(&mut e, 0x28e3, refr, &[]), -1.0);
    e.mem.set_u32(LAND_ANSWER, 0);
    assert_eq!(run(&mut e, 0x28e3, refr, &[]), -1.0);
    // A static: the hit block's shape's bhkShape material, converted (9: wood, 5).
    let target = placed(&mut e, 0x0100_00c0, 0);
    let part = e.mem.u32(
        e.mem
            .u32(e.mem.u32(e.mem.u32(target.refr + 0x64) + 0x14) + 0xa0),
    );
    e.mem.set_u32(PICK_ANSWER, part);
    let shape = alloc(&mut e, 0x20);
    let bhk = alloc(&mut e, 0x20);
    e.mem.set_u32(target.body + 0x10, shape);
    e.mem.set_u32(shape + 8, bhk);
    e.mem.set_u32(bhk + 0x10, 9);
    assert_eq!(run(&mut e, 0x28e3, refr, &[]), 5.0);
    e.mem.set_u32(bhk + 0x10, 0x20);
    assert_eq!(run(&mut e, 0x28e3, refr, &[]), -1.0);
    e.mem.set_u32(shape + 8, 0);
    assert_eq!(run(&mut e, 0x28e3, refr, &[]), -1.0);
    e.mem.set_u32(target.body + 0x10, 0);
    assert_eq!(run(&mut e, 0x28e3, refr, &[]), -1.0);
    assert_eq!(calls_to(wm::LAND_AT).len(), 3);
}

#[test]
fn ray_cast_material_of_an_actor_is_its_bases_blood_material() {
    let mut e = wsetup();
    world(&mut e, 0);
    let refr = looker(&mut e);
    let npc = alloc(&mut e, 0x200);
    e.mem.set_u8(npc + 4, 0x2a);
    e.mem.set_u32(npc + 0x1e4, 0x1234);
    let target = placed(&mut e, 0x0100_00d0, npc);
    e.mem.set_u32(target.refr, ACTOR_VTBL);
    let part = e.mem.u32(
        e.mem
            .u32(e.mem.u32(e.mem.u32(target.refr + 0x64) + 0x14) + 0xa0),
    );
    e.mem.set_u32(PICK_ANSWER, part);
    // A character: TESNPC +0x1E4, the whole word as it is (not converted).
    assert_eq!(run(&mut e, 0x28e3, refr, &[]), f64::from(0x1234));
    // A creature (IsCharacter's low byte 0): TESCreature +0x148.
    e.mem.set_u32(target.refr, CREATURE_VTBL);
    e.mem.set_u32(npc + 0x148, 0xffff_fff0);
    assert_eq!(run(&mut e, 0x28e3, refr, &[]), -16.0);
    // No base form: -1.
    e.mem.set_u32(target.refr + 0x20, 0);
    assert_eq!(run(&mut e, 0x28e3, refr, &[]), -1.0);
}

#[test]
fn ray_cast_material_without_a_node_or_the_players_controller_is_minus_one() {
    let mut e = wsetup();
    let refr = looker(&mut e);
    // No world (no player controller): -1, no pick.
    assert_eq!(run(&mut e, 0x28e3, refr, &[]), -1.0);
    world(&mut e, 0);
    let target = placed(&mut e, 0x0100_00e0, 0);
    e.mem.set_u32(PICK_ANSWER, target.body);
    assert_eq!(run(&mut e, 0x28e3, refr, &[n(6), s(b"Missing")]), -1.0);
    assert_eq!(e.mem.u32(PICK_LOG + 8), 0, "no pick for a missing node");
}
