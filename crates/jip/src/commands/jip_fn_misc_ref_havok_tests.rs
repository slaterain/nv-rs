//! Tests of the Havok reference commands on small scenes built in guest memory: nodes with the
//! classes' real vtable slots, collision objects, world objects and Havok objects at the offsets
//! the exe uses, the game's pick functions as recording stand-ins. Every expectation is taken
//! from JIP's command bodies and helpers (functions_jip/jip_fn_misc_ref.h, internal/havok.cpp,
//! internal/netimmerse.cpp, internal/jip_core.cpp, nvse/GameObjects.cpp).

use super::*;
use crate::commands::ref_id;
use crate::commands::test_support::*;
use crate::test_support::LIST;
use nvse::vars::Data;

const AREA: u32 = 0x0500_0000;
const AREA_SIZE: u32 = 0x10_0000;
const NI_NODE: u32 = 0x0109_b5ac;
const NI_TRI_SHAPE: u32 = 0x0109_d454;
const RETURN_NULL: u32 = 0x00ac_bb70;
const REF_VTBL: u32 = AREA + 0xf_0000;
const ACTOR_VTBL: u32 = AREA + 0xf_0400;
const PROJ_VTBL: u32 = AREA + 0xf_0800;
const PROJ_TYPE_FN: u32 = 0x00de_0304;
const PROJ_PROX_FN: u32 = 0x00de_0318;
const PROJ_ANSWERS: u32 = AREA + 0xf_0c00;
const LOCALS: u32 = LIST + 0x100;
const BAD: u32 = 0x7ff0_0000;

fn setup() -> Engine {
    let mut e = engine();
    e.map(AREA, AREA_SIZE);
    e.map(0x0109_b000, 0x4000);
    e.map(0x010a_8000, 0x1000);
    e.mem.set_u32(NI_NODE + 0xc, 0x0068_15c0);
    e.mem.set_u32(h::BS_FADE_NODE_VTBL + 0xc, 0x0068_15c0);
    e.mem.set_u32(NI_TRI_SHAPE + 0xc, RETURN_NULL);
    e.mem.set_u32(REF_VTBL + 0x100, 0x0047_c850);
    e.mem.set_u32(ACTOR_VTBL + 0x100, h::RETURN_TRUE);
    e.mem.set_u32(PROJ_VTBL + 0x224, h::RETURN_TRUE);
    e.mem.set_u32(PROJ_VTBL + 0x304, PROJ_TYPE_FN);
    e.mem.set_u32(PROJ_VTBL + 0x318, PROJ_PROX_FN);
    e.register(PROJ_TYPE_FN, |e, _| Ret {
        eax: e.mem.u32(PROJ_ANSWERS),
        ..Ret::default()
    });
    e.register(PROJ_PROX_FN, |e, _| Ret {
        eax: e.mem.u32(PROJ_ANSWERS + 4),
        ..Ret::default()
    });
    e
}

fn alloc(e: &mut Engine, size: u32) -> u32 {
    let top = AREA + AREA_SIZE - 4;
    let mut at = e.mem.u32(top);
    if at == 0 {
        at = AREA + 0x1000;
    }
    e.mem.set_u32(top, at + size.next_multiple_of(16));
    for i in (0..size).step_by(4) {
        e.mem.set_u32(at + i, 0);
    }
    at
}

fn text(e: &mut Engine, t: &[u8]) -> u32 {
    let at = alloc(e, t.len() as u32 + 1);
    e.mem.write(at, t);
    e.mem.set_u8(at + t.len() as u32, 0);
    at
}

/// A block of class `vtbl` named `name`, its children `slots` (0 an empty slot).
fn block(e: &mut Engine, vtbl: u32, name: &[u8], slots: &[u32]) -> u32 {
    let at = alloc(e, 0x100);
    e.mem.set_u32(at, vtbl);
    let n = text(e, name);
    e.mem.set_u32(at + 8, n);
    let data = alloc(e, 4 * slots.len() as u32 + 4);
    for (i, &c) in slots.iter().enumerate() {
        e.mem.set_u32(data + 4 * i as u32, c);
        if c != 0 {
            e.mem.set_u32(c + 0x18, at);
        }
    }
    e.mem.set_u32(at + 0xa0, data);
    e.mem.set_u16(at + 0xa6, slots.len() as u16);
    at
}

fn node(e: &mut Engine, name: &[u8], slots: &[u32]) -> u32 {
    block(e, NI_NODE, name, slots)
}

/// A Havok object of broadphase type `kind` (1 a rigid body, 2 a phantom), 0x240 bytes.
fn havok(e: &mut Engine, kind: u8) -> u32 {
    let at = alloc(e, 0x240);
    e.mem.set_u8(at + 0x28, kind);
    at
}

/// A rigid body with motion type `motion`.
fn body(e: &mut Engine, motion: u8) -> u32 {
    let b = havok(e, 1);
    e.mem.set_u8(b + 0xe8, motion);
    b
}

/// `block`'s collision object (+0x1C) with a world object (+0x10) holding `object` (+8).
fn attach(e: &mut Engine, block: u32, object: u32) {
    let world = alloc(e, 0x20);
    e.mem.set_u32(world + 8, object);
    let collision = alloc(e, 0x20);
    e.mem.set_u32(collision + 0x10, world);
    e.mem.set_u32(block + 0x1c, collision);
}

/// A base form of `kind` with id `id`.
fn base(e: &mut Engine, kind: u8, id: u32) -> u32 {
    let at = alloc(e, 0x40);
    e.mem.set_u8(at + 4, kind);
    e.mem.set_u32(at + 0xc, id);
    at
}

/// A reference (vtable `vtbl`) with id `id`, base `base`, loaded root `root` (0: no 3D).
fn reference(e: &mut Engine, vtbl: u32, id: u32, base: u32, root: u32) -> u32 {
    let at = alloc(e, 0x1000);
    e.mem.set_u32(at, vtbl);
    e.mem.set_u8(at + 4, 0x3a);
    e.mem.set_u32(at + 0xc, id);
    e.mem.set_u32(at + 0x20, base);
    if root != 0 {
        let loaded = alloc(e, 0x20);
        e.mem.set_u32(loaded + 0x14, root);
        e.mem.set_u32(at + 0x64, loaded);
    }
    at
}

/// A reference in the scene: a `BSFadeNode` root holding the reference (+0xCC), with a child
/// node holding a rigid body whose `NiAVObject` property points at that child.
struct Placed {
    refr: u32,
    body: u32,
}

fn placed(e: &mut Engine, id: u32, base_form: u32) -> Placed {
    let part = node(e, b"Part", &[]);
    let root = block(e, h::BS_FADE_NODE_VTBL, b"Root", &[part]);
    let refr = reference(e, REF_VTBL, id, base_form, root);
    e.mem.set_u32(root + 0xcc, refr);
    let b = body(e, 1);
    attach(e, part, b);
    property(e, b, h::KEY_AV_OBJECT, part);
    Placed { refr, body: b }
}

/// The Havok object's properties: one entry `key` -> `value`.
fn property(e: &mut Engine, object: u32, key: u32, value: u32) {
    let data = alloc(e, 0x20);
    e.mem.set_u32(data, 0x0bad_0000);
    e.mem.set_u32(data + 0x10, key);
    e.mem.set_u32(data + 0x18, value);
    e.mem.set_u32(object + 0x78, data);
    e.mem.set_u32(object + 0x7c, 2);
}

fn set4(e: &mut Engine, at: u32, v: [f32; 4]) {
    for (i, x) in v.iter().enumerate() {
        e.mem.set_f32(at + 4 * i as u32, *x);
    }
}

fn run(e: &mut Engine, opcode: u32, this: u32, args: &[Vec<u8>]) -> f64 {
    e.mem.set_f64(RESULT, 0.0);
    call_on(e, opcode, this, args).1
}

fn axis(a: u8) -> Vec<u8> {
    vec![a]
}

fn v(id: u16) -> Vec<u8> {
    let mut b = vec![b'f'];
    b.extend_from_slice(&id.to_le_bytes());
    b
}

fn locals(e: &mut Engine, n: u32) {
    for i in 0..n {
        let local = LOCALS + 0x100 + i * 0x10;
        e.mem.set_u32(local, i + 1);
        e.mem.set_f64(local + 8, -1.0);
        let node = LOCALS + i * 8;
        e.mem.set_u32(node, local);
        e.mem
            .set_u32(node + 4, if i + 1 < n { node + 8 } else { 0 });
    }
    e.mem.set_u32(LIST + 0xc, LOCALS);
}

fn vars(e: &Engine) -> [f64; 3] {
    [0, 1, 2].map(|i| e.mem.f64(LOCALS + 0x100 + i * 0x10 + 8))
}

/// A plain reference whose root has a rigid body of motion `motion` with the given velocities.
fn moving(e: &mut Engine, motion: u8, lin: [f32; 4], ang: [f32; 4]) -> (u32, u32) {
    let root = node(e, b"Root", &[]);
    let refr = reference(e, REF_VTBL, 0x0100_0001, 0, root);
    let b = body(e, motion);
    attach(e, root, b);
    set4(e, b + 0x1b0, lin);
    set4(e, b + 0x1c0, ang);
    (refr, b)
}

// ---- mass ----

#[test]
fn mass_sums_rigid_bodies_depth_first_with_the_estimate() {
    let mut e = setup();
    let deep = node(&mut e, b"Deep", &[]);
    let child = node(&mut e, b"Child", &[deep]);
    let shape = block(&mut e, NI_TRI_SHAPE, b"Shape", &[]);
    let root = node(&mut e, b"Root", &[shape, 0, child]);
    let refr = reference(&mut e, REF_VTBL, 0x0100_0001, 0, root);
    let (b1, b2, b3) = (body(&mut e, 1), body(&mut e, 1), havok(&mut e, 2));
    let b4 = body(&mut e, 1);
    attach(&mut e, root, b1);
    attach(&mut e, deep, b2);
    attach(&mut e, child, b3);
    attach(&mut e, shape, b4);
    e.mem.set_f32(b1 + 0x1ac, 0.25);
    e.mem.set_f32(b2 + 0x1ac, 0.1);
    e.mem.set_f32(b3 + 0x1ac, 0.5); // a phantom: not a rigid body
    e.mem.set_f32(b4 + 0x1ac, 0.5); // on a block that is not a node: not walked
    let want = 0.0f32 + h::rcp_estimate(0.25) + h::rcp_estimate(0.1);
    assert_eq!(run(&mut e, 0x2843, refr, &[]), f64::from(want));
    // An inverse mass of 0 is skipped.
    e.mem.set_f32(b2 + 0x1ac, 0.0);
    assert_eq!(
        run(&mut e, 0x2843, refr, &[]),
        f64::from(h::rcp_estimate(0.25))
    );
}

#[test]
fn mass_answers_nothing_for_actors_and_references_without_3d() {
    let mut e = setup();
    let root = node(&mut e, b"Root", &[]);
    let b = body(&mut e, 1);
    attach(&mut e, root, b);
    e.mem.set_f32(b + 0x1ac, 0.5);
    let actor = reference(&mut e, ACTOR_VTBL, 0x0100_0002, 0, root);
    assert_eq!(run(&mut e, 0x2843, actor, &[]), 0.0);
    let bare = reference(&mut e, REF_VTBL, 0x0100_0003, 0, 0);
    assert_eq!(run(&mut e, 0x2843, bare, &[]), 0.0);
    // A damaged collision object reads as none.
    e.mem.set_u32(root + 0x1c, BAD);
    let refr = reference(&mut e, REF_VTBL, 0x0100_0004, 0, root);
    assert_eq!(run(&mut e, 0x2843, refr, &[]), 0.0);
}

// ---- velocities ----

#[test]
fn object_velocity_axis_and_length_of_four_lanes() {
    let mut e = setup();
    let (refr, _) = moving(&mut e, 2, [3.0, 4.0, 12.0, 2.0], [0.0; 4]);
    assert_eq!(run(&mut e, 0x285a, refr, &[axis(b'Y')]), 4.0);
    assert_eq!(run(&mut e, 0x285a, refr, &[axis(b'Z')]), 12.0);
    let s = (3.0f32 * 3.0 + 4.0 * 4.0) + (12.0 * 12.0 + 2.0 * 2.0);
    let want = super::super::ln_fn_math::newton_sqrt(s);
    assert_eq!(run(&mut e, 0x285a, refr, &[]), f64::from(want));
}

#[test]
fn rcp_estimate_is_the_processors_estimate_not_the_quotient() {
    // Processors' estimates differ (this one gives 0.1 exactly 10): each within the documented
    // bound, and 3.0's not the rounded quotient (a 12-bit table estimate; checked on this one).
    for x in [0.1f32, 3.0, 7.0, 0.3] {
        let est = h::rcp_estimate(x);
        let rel = ((f64::from(est) - 1.0 / f64::from(x)) * f64::from(x)).abs();
        assert!(rel <= 1.5 * 2f64.powi(-12), "{x}: {rel}");
    }
    assert_ne!(h::rcp_estimate(3.0), 1.0 / 3.0f32);
}

#[test]
fn object_velocity_length_is_the_newton_step_not_the_exact_root() {
    let mut e = setup();
    let (refr, _) = moving(&mut e, 2, [3.0, 4.0, 12.0, 0.0], [0.0; 4]);
    let got = run(&mut e, 0x285a, refr, &[]);
    assert_ne!(got, f64::from(169f32.sqrt()));
    assert!((got - 13.0).abs() <= 13.0 * 2f64.powi(-20), "{got}");
}

#[test]
fn velocities_need_a_mobile_rigid_body() {
    let mut e = setup();
    // Motion type 4 (keyframed) and 5 (fixed) lack bit 1.
    for motion in [1u8, 4, 5] {
        let (refr, _) = moving(&mut e, motion, [3.0, 4.0, 12.0, 0.0], [1.0; 4]);
        assert_eq!(run(&mut e, 0x285a, refr, &[axis(b'X')]), 0.0, "{motion}");
        assert_eq!(run(&mut e, 0x285b, refr, &[s(b"")]), 0.0, "{motion}");
    }
    for motion in [3u8, 6, 7] {
        let (refr, _) = moving(&mut e, motion, [3.0, 4.0, 12.0, 0.0], [0.0; 4]);
        assert_eq!(run(&mut e, 0x285a, refr, &[axis(b'X')]), 3.0, "{motion}");
    }
    // A phantom is not a rigid body.
    let (refr, b) = moving(&mut e, 2, [3.0, 0.0, 0.0, 0.0], [0.0; 4]);
    e.mem.set_u8(b + 0x28, 2);
    assert_eq!(run(&mut e, 0x285a, refr, &[axis(b'X')]), 0.0);
}

#[test]
fn angular_velocity_dot_with_a_column_or_exact_length() {
    let mut e = setup();
    let (refr, b) = moving(&mut e, 2, [0.0; 4], [1.0, 2.0, 2.0, 99.0]);
    set4(&mut e, b + 0xf0, [1.0, 0.0, 0.0, 0.0]);
    set4(&mut e, b + 0x100, [0.5, 0.25, 2.0, 7.0]);
    set4(&mut e, b + 0x110, [0.0, 0.0, -1.0, 0.0]);
    // The fourth lane is cleared: 99 does not count.
    assert_eq!(run(&mut e, 0x285b, refr, &[s(b""), axis(b'X')]), 1.0);
    let y = (1.0f32 * 0.5 + 2.0 * 0.25) + (2.0 * 2.0 + 0.0 * 7.0);
    assert_eq!(
        run(&mut e, 0x285b, refr, &[s(b""), axis(b'Y')]),
        f64::from(y)
    );
    assert_eq!(run(&mut e, 0x285b, refr, &[s(b""), axis(b'Z')]), -2.0);
    assert_eq!(run(&mut e, 0x285b, refr, &[s(b"")]), 3.0);
}

#[test]
fn angular_velocity_of_a_named_block() {
    let mut e = setup();
    let arm = node(&mut e, b"Arm", &[]);
    let root = node(&mut e, b"Root", &[arm]);
    let refr = reference(&mut e, REF_VTBL, 0x0100_0001, 0, root);
    let b = body(&mut e, 2);
    attach(&mut e, arm, b);
    set4(&mut e, b + 0x1c0, [0.0, 3.0, 4.0, 0.0]);
    assert_eq!(run(&mut e, 0x285b, refr, &[s(b"Arm")]), 5.0);
    assert_eq!(run(&mut e, 0x285b, refr, &[s(b"Leg")]), 0.0);
    // The root has no body.
    assert_eq!(run(&mut e, 0x285b, refr, &[s(b"")]), 0.0);
}

#[test]
fn linear_velocity_alt_raw_or_turned() {
    let mut e = setup();
    locals(&mut e, 3);
    let (refr, b) = moving(&mut e, 2, [1.0, 2.0, 3.0, 5.0], [0.0; 4]);
    set4(&mut e, b + 0xf0, [0.0, 1.0, 0.0, 0.0]);
    set4(&mut e, b + 0x100, [1.0, 0.0, 0.0, 0.0]);
    set4(&mut e, b + 0x110, [0.0, 0.0, 2.0, 0.0]);
    let args = |l: i32| vec![s(b""), v(1), v(2), v(3), n(l)];
    assert_eq!(run(&mut e, 0x290f, refr, &args(0)), 1.0);
    assert_eq!(vars(&e), [1.0, 2.0, 3.0]);
    assert_eq!(run(&mut e, 0x290f, refr, &args(1)), 1.0);
    assert_eq!(vars(&e), [2.0, 1.0, 6.0]);
    // Immobile: nothing written.
    locals(&mut e, 3);
    e.mem.set_u8(b + 0xe8, 4);
    assert_eq!(run(&mut e, 0x290f, refr, &args(0)), 0.0);
    assert_eq!(vars(&e), [-1.0; 3]);
}

#[test]
fn angular_velocity_alt_turned_unless_global() {
    let mut e = setup();
    locals(&mut e, 3);
    let (refr, b) = moving(&mut e, 2, [0.0; 4], [1.0, 2.0, 3.0, 5.0]);
    set4(&mut e, b + 0xf0, [0.0, 1.0, 0.0, 0.0]);
    set4(&mut e, b + 0x100, [1.0, 0.0, 0.0, 0.0]);
    set4(&mut e, b + 0x110, [0.0, 0.0, 2.0, 0.0]);
    let args = |g: i32| vec![s(b""), v(1), v(2), v(3), n(g)];
    assert_eq!(run(&mut e, 0x2910, refr, &args(0)), 1.0);
    assert_eq!(vars(&e), [2.0, 1.0, 6.0]);
    assert_eq!(run(&mut e, 0x2910, refr, &args(1)), 1.0);
    assert_eq!(vars(&e), [1.0, 2.0, 3.0]);
}

#[test]
fn rotate_sums_pairs_and_keeps_a_nan_column_word() {
    let cols = [
        [1.0, 2.0, 3.0, f32::INFINITY],
        [0.0; 4],
        [1.0, 1.0, 1.0, 0.0],
    ];
    let r = rotate(cols, [1.0, 1.0, 1.0, 1.0]);
    assert!(r[0].is_nan());
    assert_eq!([r[1], r[2]], [0.0, 3.0]);
}

// ---- collision properties and layers ----

#[test]
fn collision_properties_zero_to_eight() {
    let mut e = setup();
    let (refr, b) = moving(&mut e, 4, [0.0; 4], [0.0; 4]);
    let fields = [
        (0, 0x90, 0.5f32),
        (1, 0x94, 0.25),
        (2, 0x194, 0.125),
        (3, 0x198, 2.0),
        (4, 0x1a0, 3.0),
        (5, 0x1a4, 4.0),
        (6, 0x1a8, 5.0),
    ];
    for &(_, at, val) in &fields {
        e.mem.set_f32(b + at, val);
    }
    e.mem.set_f32(b + 0x1ac, 0.3);
    for &(p, _, val) in &fields {
        assert_eq!(
            run(&mut e, 0x2913, refr, &[s(b""), n(p)]),
            f64::from(val),
            "{p}"
        );
    }
    // The mass in doubles; an immobile body still answers.
    assert_eq!(
        run(&mut e, 0x2913, refr, &[s(b""), n(7)]),
        1.0 / f64::from(0.3f32)
    );
    assert_eq!(run(&mut e, 0x2913, refr, &[s(b""), n(8)]), 4.0);
    assert_eq!(run(&mut e, 0x2913, refr, &[s(b""), n(9)]), 0.0);
    e.mem.set_f32(b + 0x1ac, 0.0);
    assert_eq!(run(&mut e, 0x2913, refr, &[s(b""), n(7)]), 0.0);
    e.mem.set_f32(b + 0x1ac, -2.0);
    assert_eq!(run(&mut e, 0x2913, refr, &[s(b""), n(7)]), 0.0);
    // Not a rigid body.
    e.mem.set_u8(b + 0x28, 2);
    assert_eq!(run(&mut e, 0x2913, refr, &[s(b""), n(0)]), 0.0);
}

#[test]
fn collision_layer_of_a_named_node() {
    let mut e = setup();
    let shape = block(&mut e, NI_TRI_SHAPE, b"Shape", &[]);
    let arm = node(&mut e, b"Arm", &[]);
    let root = node(&mut e, b"Root", &[shape, arm]);
    let refr = reference(&mut e, REF_VTBL, 0x0100_0001, 0, root);
    for (blk, layer) in [(root, 0x21u8), (arm, 0x0b), (shape, 0x05)] {
        let o = havok(&mut e, 2);
        e.mem.set_u32(o + 0x2c, 0xabcd_0000 | u32::from(layer));
        attach(&mut e, blk, o);
    }
    assert_eq!(run(&mut e, 0x2915, refr, &[s(b"")]), 33.0);
    assert_eq!(run(&mut e, 0x2915, refr, &[s(b"Arm")]), 11.0);
    // A block that is not a node finds nothing.
    assert_eq!(run(&mut e, 0x2915, refr, &[s(b"Shape")]), 0.0);
}

// ---- phantoms, mobility, grabbing ----

#[test]
fn has_phantom_cases() {
    let mut e = setup();
    let leaf = node(&mut e, b"Leaf", &[]);
    let mid = node(&mut e, b"Mid", &[leaf]);
    let root = node(&mut e, b"Root", &[mid]);
    let refr = reference(&mut e, REF_VTBL, 0x0100_0001, 0, root);
    assert_eq!(run(&mut e, 0x27bb, refr, &[]), 0.0);
    let ph = havok(&mut e, 2);
    attach(&mut e, leaf, ph);
    assert_eq!(run(&mut e, 0x27bb, refr, &[]), 1.0);
    e.mem.set_u8(ph + 0x28, 1);
    assert_eq!(run(&mut e, 0x27bb, refr, &[]), 0.0);
    // The trigger phantom counts only as a bhkSimpleShapePhantom.
    let trig = alloc(&mut e, 0x20);
    let loaded = e.mem.u32(refr + 0x64);
    e.mem.set_u32(loaded + 0x18, trig);
    e.mem.set_u32(trig, 0x0103_0000);
    assert_eq!(run(&mut e, 0x27bb, refr, &[]), 0.0);
    e.mem.set_u32(trig, h::BHK_SIMPLE_SHAPE_PHANTOM_VTBL);
    assert_eq!(run(&mut e, 0x27bb, refr, &[]), 1.0);
    // An actor with 3D has one; without 3D nothing.
    let a_root = node(&mut e, b"A", &[]);
    let actor = reference(&mut e, ACTOR_VTBL, 0x0100_0002, 0, a_root);
    assert_eq!(run(&mut e, 0x27bb, actor, &[]), 1.0);
    let ghost = reference(&mut e, ACTOR_VTBL, 0x0100_0003, 0, 0);
    assert_eq!(run(&mut e, 0x27bb, ghost, &[]), 0.0);
}

#[test]
fn is_mobile_cases() {
    let mut e = setup();
    let actor = reference(&mut e, ACTOR_VTBL, 0x0100_0002, 0, 0);
    assert_eq!(run(&mut e, 0x2876, actor, &[]), 1.0);
    let proj = reference(&mut e, PROJ_VTBL, 0x0100_0003, 0, 0);
    assert_eq!(run(&mut e, 0x2876, proj, &[]), 1.0);
    let child = node(&mut e, b"Child", &[]);
    let root = node(&mut e, b"Root", &[child]);
    let refr = reference(&mut e, REF_VTBL, 0x0100_0001, 0, root);
    let fixed = body(&mut e, 5);
    attach(&mut e, root, fixed);
    assert_eq!(run(&mut e, 0x2876, refr, &[]), 0.0);
    // A movable child (bit 1 of the motion type, read whatever the object).
    let o = havok(&mut e, 2);
    e.mem.set_u8(o + 0xe8, 6);
    attach(&mut e, child, o);
    assert_eq!(run(&mut e, 0x2876, refr, &[]), 1.0);
    let bare = reference(&mut e, REF_VTBL, 0x0100_0004, 0, 0);
    assert_eq!(run(&mut e, 0x2876, bare, &[]), 0.0);
}

#[test]
fn is_grabbable_actors_projectiles_and_objects() {
    let mut e = setup();
    let actor = reference(&mut e, ACTOR_VTBL, 0x0100_0002, 0, 0);
    e.map(0x011e_0000, 0x1000);
    e.mem.set_u32(0x011e_0b20, 0);
    assert_eq!(run(&mut e, 0x2877, actor, &[]), 0.0);
    for (state, want) in [(1u32, 1.0), (2, 1.0), (3, 0.0), (0, 0.0)] {
        e.mem.set_u32(actor + 0x108, state);
        assert_eq!(run(&mut e, 0x2877, actor, &[]), want, "{state}");
    }
    e.mem.set_u32(0x011e_0b20, 1);
    assert_eq!(run(&mut e, 0x2877, actor, &[]), 1.0);
    let proj = reference(&mut e, PROJ_VTBL, 0x0100_0003, 0, 0);
    e.mem.set_u32(PROJ_ANSWERS, 3);
    e.mem.set_u32(PROJ_ANSWERS + 4, 1);
    e.mem.set_u32(proj + 0xc8, 0x200);
    assert_eq!(run(&mut e, 0x2877, proj, &[]), 1.0);
    e.mem.set_u32(proj + 0xc8, 0x1ff);
    assert_eq!(run(&mut e, 0x2877, proj, &[]), 0.0);
    e.mem.set_u32(proj + 0xc8, 0x200);
    e.mem.set_u32(PROJ_ANSWERS, 2);
    assert_eq!(run(&mut e, 0x2877, proj, &[]), 0.0);
    e.mem.set_u32(PROJ_ANSWERS, 3);
    e.mem.set_u32(PROJ_ANSWERS + 4, 0x100);
    assert_eq!(run(&mut e, 0x2877, proj, &[]), 0.0);
    let (refr, b) = moving(&mut e, 2, [0.0; 4], [0.0; 4]);
    assert_eq!(run(&mut e, 0x2877, refr, &[]), 1.0);
    e.mem.set_u8(b + 0xe8, 4);
    assert_eq!(run(&mut e, 0x2877, refr, &[]), 0.0);
}

// ---- contacts ----

/// A plain reference whose root body has master and slave constraints with `others`' bodies.
fn touching(e: &mut Engine, others: &[u32], slaves: &[u32]) -> u32 {
    let root = node(e, b"Root", &[]);
    let refr = reference(e, REF_VTBL, 0x0100_0010, 0, root);
    let b = body(e, 1);
    attach(e, root, b);
    let masters = alloc(e, 0x30 * others.len() as u32 + 0x30);
    for (i, &o) in others.iter().enumerate() {
        e.mem.set_u32(masters + 0x30 * i as u32 + 8, o);
    }
    e.mem.set_u32(b + 0xac, masters);
    e.mem.set_u16(b + 0xb0, others.len() as u16);
    if !slaves.is_empty() {
        let data = alloc(e, 4 * slaves.len() as u32);
        for (i, &o) in slaves.iter().enumerate() {
            let inst = alloc(e, 0x38);
            e.mem.set_u32(inst + 0x14, o);
            e.mem.set_u32(data + 4 * i as u32, inst);
        }
        e.mem.set_u32(b + 0xb4, data);
        e.mem.set_u32(b + 0xb8, slaves.len() as u32);
    }
    refr
}

#[test]
fn contact_refs_of_a_plain_reference() {
    let mut e = setup();
    let (ba, bb) = (
        base(&mut e, 0x1f, 0x0200_0001),
        base(&mut e, 0x28, 0x0200_0002),
    );
    let a = placed(&mut e, 0x0100_0021, ba);
    let b = placed(&mut e, 0x0100_0022, bb);
    // A body whose collision-object property leads to b's part node.
    let viac = body(&mut e, 1);
    let part_b = e.mem.u32(e.mem.u32(e.mem.u32(b.refr + 0x64) + 0x14) + 0xa0);
    let part_b = e.mem.u32(part_b);
    let coll = alloc(&mut e, 0x20);
    e.mem.set_u32(coll + 8, part_b);
    property(&mut e, viac, h::KEY_COLLISION_OBJECT, coll);
    let lone = body(&mut e, 1);
    let refr = touching(&mut e, &[0, viac, lone], &[a.body]);
    let id = run(&mut e, 0x27b7, refr, &[]);
    let mut want = [a.refr, b.refr];
    want.sort();
    assert_eq!(
        array_data(&e, id),
        want.iter()
            .map(|&r| Data::Form(ref_id(&e, r)))
            .collect::<Vec<_>>()
    );
    set_ref(&mut e, 1, a.refr);
    assert_eq!(run(&mut e, 0x27b6, refr, &[r(1)]), 1.0);
    set_ref(&mut e, 1, ba);
    assert_eq!(run(&mut e, 0x27b6, refr, &[r(1)]), 0.0);
    assert_eq!(run(&mut e, 0x27b8, refr, &[r(1)]), 1.0);
    assert_eq!(run(&mut e, 0x27b9, refr, &[n(0x28)]), 1.0);
    assert_eq!(run(&mut e, 0x27b9, refr, &[n(0x29)]), 0.0);
    // The slave list is walked only when its data is not null (its size alone does not count).
    let root = e.mem.u32(e.mem.u32(refr + 0x64) + 0x14);
    let own = e.mem.u32(e.mem.u32(e.mem.u32(root + 0x1c) + 0x10) + 8);
    e.mem.set_u32(own + 0xb4, 0);
    let id = run(&mut e, 0x27b7, refr, &[]);
    assert_eq!(array_data(&e, id), vec![Data::Form(ref_id(&e, b.refr))]);
    set_ref(&mut e, 1, a.refr);
    assert_eq!(run(&mut e, 0x27b6, refr, &[r(1)]), 0.0);
    // With no master entries either: nothing, no array.
    e.mem.set_u16(own + 0xb0, 0);
    assert_eq!(run(&mut e, 0x27b7, refr, &[]), 0.0);
}

#[test]
fn contacts_through_a_form_list_and_the_eval() {
    let mut e = setup();
    let bb = base(&mut e, 0x28, 0x0200_0002);
    let b = placed(&mut e, 0x0100_0022, bb);
    let refr = touching(&mut e, &[b.body], &[]);
    let list = base(&mut e, 0x55, 0x0200_0050);
    let n1 = alloc(&mut e, 8);
    let other = base(&mut e, 0x1f, 0x0200_0003);
    e.mem.set_u32(list + 0x18, other);
    e.mem.set_u32(list + 0x1c, n1);
    e.mem.set_u32(n1, b.refr);
    set_ref(&mut e, 1, list);
    assert_eq!(run(&mut e, 0x27b6, refr, &[r(1)]), 1.0);
    e.mem.set_u32(n1, other);
    assert_eq!(run(&mut e, 0x27b6, refr, &[r(1)]), 0.0);
    e.mem.set_u32(n1, bb);
    assert_eq!(run(&mut e, 0x27b8, refr, &[r(1)]), 1.0);
    assert_eq!(eval(&mut e, 0x27b9, refr, 0x28), 1.0);
    assert_eq!(eval(&mut e, 0x27b9, refr, 0x27), -5.0);
}

fn eval(e: &mut Engine, opcode: u32, this: u32, arg1: u32) -> f64 {
    let record = nvse::table::record_of(e, opcode);
    let at = e.mem.u32(record + 0x20);
    e.mem.set_f64(RESULT, -5.0);
    e.call(at, &[this, arg1, 0, RESULT]);
    e.mem.f64(RESULT)
}

#[test]
fn contacts_of_a_trigger_phantom_and_a_phantom_node() {
    let mut e = setup();
    let bb = base(&mut e, 0x28, 0x0200_0002);
    let b = placed(&mut e, 0x0100_0022, bb);
    let c = placed(&mut e, 0x0100_0023, bb);
    let root = node(&mut e, b"Root", &[]);
    let refr = reference(&mut e, REF_VTBL, 0x0100_0010, 0, root);
    // The trigger phantom's Havok object lists cdBodies at +0x10 of the world objects.
    let hk = alloc(&mut e, 0x170);
    let cds = alloc(&mut e, 8);
    e.mem.set_u32(cds, b.body + 0x10);
    e.mem.set_u32(hk + 0x160, cds);
    e.mem.set_u32(hk + 0x164, 1);
    let trig = alloc(&mut e, 0x20);
    e.mem.set_u32(trig, h::BHK_SIMPLE_SHAPE_PHANTOM_VTBL);
    e.mem.set_u32(trig + 8, hk);
    let loaded = e.mem.u32(refr + 0x64);
    e.mem.set_u32(loaded + 0x18, trig);
    // The root's phantom: an hkpSimpleShapePhantom listing c.
    let ph = havok(&mut e, 2);
    e.mem.set_u32(ph, h::HKP_SIMPLE_SHAPE_PHANTOM_VTBL);
    let cds2 = alloc(&mut e, 8);
    e.mem.set_u32(cds2, c.body + 0x10);
    e.mem.set_u32(ph + 0x160, cds2);
    e.mem.set_u32(ph + 0x164, 1);
    attach(&mut e, root, ph);
    let id = run(&mut e, 0x27b7, refr, &[]);
    let mut want = [b.refr, c.refr];
    want.sort();
    let got = array_data(&e, id);
    assert_eq!(
        got,
        want.iter()
            .map(|&r| Data::Form(ref_id(&e, r)))
            .collect::<Vec<_>>()
    );
    // A phantom of another class lists nothing of its own.
    e.mem.set_u32(ph, 0x010c_0000);
    let id = run(&mut e, 0x27b7, refr, &[]);
    assert_eq!(array_data(&e, id), vec![Data::Form(ref_id(&e, b.refr))]);
}

/// An actor with a middle-high process (level `level`) whose controller has `under` under its
/// feet (support count `count`) and touches `bodies`.
fn actor_with(e: &mut Engine, level: u8, count: u32, under: u32, bodies: &[u32]) -> u32 {
    let actor = reference(e, ACTOR_VTBL, 0x0100_0030, 0, 0);
    let process = alloc(e, 0x200);
    e.mem.set_u8(process + 0x28, level);
    let ctrl = alloc(e, 0x700);
    e.mem.set_u32(process + 0x138, ctrl);
    e.mem.set_u32(actor + 0x68, process);
    e.mem.set_u32(ctrl + 0x608, count);
    e.mem.set_u32(ctrl + 0x60c, under);
    let data = alloc(e, 4 * bodies.len() as u32 + 4);
    for (i, &o) in bodies.iter().enumerate() {
        e.mem.set_u32(data + 4 * i as u32, o);
    }
    e.mem.set_u32(ctrl + 0x3b4, data);
    e.mem.set_u32(ctrl + 0x3b8, bodies.len() as u32);
    actor
}

#[test]
fn actor_contacts_under_feet_first_then_inserted() {
    let mut e = setup();
    let bb = base(&mut e, 0x28, 0x0200_0002);
    let (p1, p2, p3) = (
        placed(&mut e, 0x0100_0041, bb),
        placed(&mut e, 0x0100_0042, bb),
        placed(&mut e, 0x0100_0043, bb),
    );
    // The body under the feet (p2, support count 0x100: low byte 0) appended first, the rest
    // inserted uniquely by address: p3 after it, p1 before, the repeats not again.
    let actor = actor_with(
        &mut e,
        1,
        0x100,
        p2.body,
        &[p3.body, p1.body, p2.body, p3.body],
    );
    let id = run(&mut e, 0x27b7, actor, &[]);
    assert!(p1.refr < p2.refr && p2.refr < p3.refr);
    assert_eq!(
        array_data(&e, id),
        [p1.refr, p2.refr, p3.refr]
            .map(|r| Data::Form(ref_id(&e, r)))
            .to_vec()
    );
    // A support count whose low byte is not 0 leaves the body under the feet out.
    let actor = actor_with(&mut e, 0, 0x01, p3.body, &[p1.body]);
    let id = run(&mut e, 0x27b7, actor, &[]);
    assert_eq!(array_data(&e, id), vec![Data::Form(ref_id(&e, p1.refr))]);
    set_ref(&mut e, 1, p3.refr);
    assert_eq!(run(&mut e, 0x27b6, actor, &[r(1)]), 0.0);
    // A low process (level 2) has no controller: nothing.
    let low = actor_with(&mut e, 2, 0, p3.body, &[p1.body]);
    assert_eq!(run(&mut e, 0x27b7, low, &[]), 0.0);
    assert_eq!(run(&mut e, 0x27b9, low, &[n(0x28)]), 0.0);
    let ctrl_actor = actor_with(&mut e, 1, 0, p3.body, &[]);
    assert_eq!(run(&mut e, 0x27b6, ctrl_actor, &[r(1)]), 1.0);
}

#[test]
fn parent_ref_walks_up_to_a_fade_node_with_a_reference() {
    let mut e = setup();
    let leaf = node(&mut e, b"Leaf", &[]);
    let fade_empty = block(&mut e, h::BS_FADE_NODE_VTBL, b"F1", &[leaf]);
    let top = block(&mut e, h::BS_FADE_NODE_VTBL, b"F2", &[fade_empty]);
    e.mem.set_u32(top + 0xcc, 0x0500_9990);
    assert_eq!(
        h::block_parent_ref(&e, leaf, &mut h::Walk::new()),
        0x0500_9990
    );
    e.mem.set_u32(fade_empty + 0xcc, 0x0500_9980);
    assert_eq!(
        h::block_parent_ref(&e, leaf, &mut h::Walk::new()),
        0x0500_9980
    );
    // A collision-object property with a null object answers none.
    let o = havok(&mut e, 1);
    property(&mut e, o, h::KEY_COLLISION_OBJECT, 0);
    assert_eq!(h::object_parent_ref(&e, o, &mut h::Walk::new()), 0);
    // A damaged properties array answers none.
    e.mem.set_u32(o + 0x78, BAD);
    assert_eq!(h::object_parent_ref(&e, o, &mut h::Walk::new()), 0);
    // Parents running in a loop end at the bound.
    e.mem.set_u32(top + 0x18, leaf);
    e.mem.set_u32(top + 0xcc, 0);
    e.mem.set_u32(fade_empty + 0xcc, 0);
    assert_eq!(h::block_parent_ref(&e, leaf, &mut h::Walk::new()), 0);
}

#[test]
fn walks_end_on_a_child_loop() {
    let mut e = setup();
    let a = node(&mut e, b"A", &[]);
    let root = node(&mut e, b"Root", &[a]);
    // a's child is root: a cycle.
    let data = alloc(&mut e, 4);
    e.mem.set_u32(data, root);
    e.mem.set_u32(a + 0xa0, data);
    e.mem.set_u16(a + 0xa6, 1);
    let refr = reference(&mut e, REF_VTBL, 0x0100_0001, 0, root);
    assert_eq!(run(&mut e, 0x2876, refr, &[]), 0.0);
    assert_eq!(run(&mut e, 0x27bb, refr, &[]), 0.0);
    assert_eq!(run(&mut e, 0x2843, refr, &[]), 0.0);
    assert_eq!(run(&mut e, 0x285b, refr, &[s(b"Missing")]), 0.0);
    assert_eq!(run(&mut e, 0x27b7, refr, &[]), 0.0);
}

/// A node whose 0xFFFF children slots all hold the node itself.
fn self_loop(e: &mut Engine) -> u32 {
    let n = node(e, b"Loop", &[]);
    let data = alloc(e, 4 * 0xffff);
    for i in 0..0xffffu32 {
        e.mem.set_u32(data + 4 * i, n);
    }
    e.mem.set_u32(n + 0xa0, data);
    e.mem.set_u16(n + 0xa6, 0xffff);
    n
}

#[test]
fn walk_budget_charges_each_child_slot_of_a_self_loop() {
    let mut e = setup();
    let root = self_loop(&mut e);
    let refr = reference(&mut e, REF_VTBL, 0x0100_0001, 0, root);
    // Every command returns, finding nothing.
    let cases: [(u32, Vec<Vec<u8>>); 8] = [
        (0x2876, vec![]),
        (0x2877, vec![]),
        (0x27bb, vec![]),
        (0x2843, vec![]),
        (0x285b, vec![s(b"Missing")]),
        (0x27b7, vec![]),
        (0x2913, vec![s(b"Missing"), n(0)]),
        (0x2915, vec![s(b"Missing")]),
    ];
    for (op, args) in cases {
        assert_eq!(run(&mut e, op, refr, &args), 0.0, "{op:x}");
    }
    set_ref(&mut e, 1, refr);
    assert_eq!(run(&mut e, 0x27b6, refr, &[r(1)]), 0.0);
    assert_eq!(run(&mut e, 0x27b9, refr, &[n(0x3a)]), 0.0);
    // The budget is charged per slot examined: with 10 units left, 10 slots are read.
    let mut walk = h::Walk::new();
    for _ in 0..h::MAX_WORK - 10 {
        assert!(walk.charge());
    }
    assert_eq!(h::children(&e, root, &mut walk).len(), 10);
    assert!(walk.spent());
    assert!(h::children(&e, root, &mut walk).is_empty());
    assert_eq!(h::children(&e, root, &mut h::Walk::new()).len(), 0xffff);
}

#[test]
fn contact_list_stops_at_the_item_cap_with_masters_of_0xffff() {
    const MASTERS_AREA: u32 = 0x0600_0000;
    let mut e = setup();
    let bb = base(&mut e, 0x28, 0x0200_0002);
    let b = placed(&mut e, 0x0100_0022, bb);
    e.map(MASTERS_AREA, 0x30_0000);
    for i in 0..0xffffu32 {
        e.mem.set_u32(MASTERS_AREA + 0x30 * i + 8, b.body);
    }
    let masters_body = |e: &mut Engine| {
        let x = body(e, 1);
        e.mem.set_u32(x + 0xac, MASTERS_AREA);
        e.mem.set_u16(x + 0xb0, 0xffff);
        x
    };
    let child = node(&mut e, b"Child", &[]);
    let root = node(&mut e, b"Root", &[child]);
    let (b1, b2) = (masters_body(&mut e), masters_body(&mut e));
    attach(&mut e, root, b1);
    attach(&mut e, child, b2);
    let refr = reference(&mut e, REF_VTBL, 0x0100_0010, 0, root);
    // 2 * 0xFFFF bodies listed, the list stops at the cap.
    let got = h::ref_contacts(&e, refr, &mut h::Walk::new());
    assert_eq!(got.len(), h::MAX_ITEMS as usize);
    assert!(got.iter().all(|&x| x == b.body));
    // Every command returns, each body's reference found.
    let id = run(&mut e, 0x27b7, refr, &[]);
    assert_eq!(array_data(&e, id), vec![Data::Form(ref_id(&e, b.refr))]);
    set_ref(&mut e, 1, b.refr);
    assert_eq!(run(&mut e, 0x27b6, refr, &[r(1)]), 1.0);
    set_ref(&mut e, 1, bb);
    assert_eq!(run(&mut e, 0x27b8, refr, &[r(1)]), 1.0);
    assert_eq!(run(&mut e, 0x27b9, refr, &[n(0x28)]), 1.0);
    assert_eq!(eval(&mut e, 0x27b9, refr, 0x28), 1.0);
    // Each master entry read is charged: with 5 units left, 5 bodies.
    assert_eq!(h::ref_contacts(&e, refr, &mut nearly_spent(5)).len(), 5);
}

/// A walk with only `left` units of its budget left.
fn nearly_spent(left: u32) -> h::Walk {
    let mut walk = h::Walk::new();
    for _ in 0..h::MAX_WORK - left {
        walk.charge();
    }
    walk
}

#[test]
fn walk_budget_charges_array_items_properties_and_parents() {
    let mut e = setup();
    let bb = base(&mut e, 0x28, 0x0200_0002);
    let b = placed(&mut e, 0x0100_0022, bb);
    // An actor's touched objects: one unit per item.
    let actor = actor_with(&mut e, 1, 1, 0, &[b.body, b.body, b.body]);
    let got = h::contacts(&e, actor, &mut nearly_spent(2)).map(|c| c.bodies);
    assert_eq!(got, Some(vec![b.body, b.body]));
    // The parent lookup: two property entries, then the part and the root (the fade node with
    // the reference): four units; three are not enough.
    assert_eq!(
        h::object_parent_ref(&e, b.body, &mut nearly_spent(4)),
        b.refr
    );
    assert_eq!(h::object_parent_ref(&e, b.body, &mut nearly_spent(3)), 0);
}

// ---- ray casts ----

const PICK_LOG: u32 = AREA + 0xe_0000;
const PICK_ANSWER: u32 = AREA + 0xe_0100;
const PICK_FRACTION: u32 = AREA + 0xe_0104;

/// The pick records its record (0xB0 bytes) and arguments, writes the fraction, answers the
/// object at [`PICK_ANSWER`].
fn pick_stand_in(e: &mut Engine) {
    e.register(h::PICK, |e, a| {
        let (tes, rec, by_cell) = (a[0], a[1], a[2]);
        e.mem.set_u32(PICK_LOG, tes);
        e.mem.set_u32(PICK_LOG + 4, by_cell & 0xff);
        e.mem.set_u32(PICK_LOG + 8, e.mem.u32(PICK_LOG + 8) + 1);
        for i in (0..0xb0).step_by(4) {
            let w = e.mem.u32(rec + i);
            e.mem.set_u32(PICK_LOG + 0x10 + i, w);
        }
        let f = e.mem.u32(PICK_FRACTION);
        e.mem.set_u32(rec + 0x40, f);
        Ret {
            eax: e.mem.u32(PICK_ANSWER),
            ..Ret::default()
        }
    });
}

/// The player with a controller whose phantom's filter info is `filter`, and the TES.
fn world(e: &mut Engine, filter: u32) -> u32 {
    let tes = alloc(e, 0xc4);
    e.mem.set_u32(h::TES, tes);
    let player = reference(e, ACTOR_VTBL, 0x14, 0, 0);
    e.mem.set_u8(player + 0x64a, 1);
    let process = alloc(e, 0x200);
    let ctrl = alloc(e, 0x700);
    let shape = alloc(e, 0x20);
    let hk = havok(e, 2);
    e.mem.set_u32(hk + 0x2c, filter);
    e.mem.set_u32(shape + 8, hk);
    e.mem.set_u32(ctrl + 0x594, shape);
    e.mem.set_u32(process + 0x138, ctrl);
    e.mem.set_u32(player + 0x68, process);
    e.mem.set_u32(h::PLAYER, player);
    pick_stand_in(e);
    tes
}

/// A plain reference whose root's world rotation and translation (and scale) are set.
fn looker(e: &mut Engine) -> u32 {
    let root = node(e, b"Root", &[]);
    for (i, v) in [1.0f32, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0]
        .iter()
        .enumerate()
    {
        e.mem.set_f32(root + 0x68 + 4 * i as u32, *v);
    }
    set4(e, root + 0x8c, [70.0, -140.0, 7.0, 1.0]);
    reference(e, REF_VTBL, 0x0100_0060, 0, root)
}

fn logged_f32(e: &Engine, off: u32) -> f32 {
    e.mem.f32(PICK_LOG + 0x10 + off)
}

#[test]
fn ray_cast_ref_builds_jips_record_and_answers_the_hit_ref() {
    let mut e = setup();
    let tes = world(&mut e, 0x1234_5678);
    let refr = looker(&mut e);
    let target = placed(&mut e, 0x0100_0070, 0);
    let part = e.mem.u32(
        e.mem
            .u32(e.mem.u32(e.mem.u32(target.refr + 0x64) + 0x14) + 0xa0),
    );
    e.mem.set_u32(PICK_ANSWER, part);
    e.mem.set_f32(PICK_FRACTION, 0.5);
    run(&mut e, 0x28e2, refr, &[n(-3)]);
    assert_eq!(form_result(&e), 0x0100_0070);
    assert_eq!(e.mem.u32(PICK_LOG), tes);
    assert_eq!(e.mem.u32(PICK_LOG + 4), 1);
    let k = h::to_havok();
    assert_eq!(
        [0, 4, 8, 12].map(|o| logged_f32(&e, o)),
        [70.0 * k, -140.0 * k, 7.0 * k, 0.0]
    );
    // Column 1 (2, 5, 8) times 50000 times kUnitConv, plus the start.
    assert_eq!(
        [0x10, 0x14, 0x18, 0x1c].map(|o| logged_f32(&e, o)),
        [
            (2.0 * 50000.0) * k + 70.0 * k,
            (5.0 * 50000.0) * k + -140.0 * k,
            (8.0 * 50000.0) * k + 7.0 * k,
            0.0
        ]
    );
    // A negative layer is 6; the filter keeps its high word.
    assert_eq!(e.mem.u32(PICK_LOG + 0x10 + 0x24), 0x1234_0006);
    assert_eq!(logged_f32(&e, 0x40), 1.0);
    assert_eq!(e.mem.u32(PICK_LOG + 0x10 + 0x44), u32::MAX);
    assert_eq!(e.mem.u32(PICK_LOG + 0x10 + 0x50), u32::MAX);
    run(&mut e, 0x28e2, refr, &[n(0x85)]);
    assert_eq!(e.mem.u32(PICK_LOG + 0x10 + 0x24), 0x1234_0005);
    // No hit: no result.
    e.mem.set_u32(PICK_ANSWER, 0);
    run(&mut e, 0x28e2, refr, &[]);
    assert_eq!(form_result(&e), 0);
    assert_eq!(e.mem.u32(PICK_LOG + 0x10 + 0x24), 0x1234_0006);
}

#[test]
fn ray_cast_without_the_players_controller_casts_nothing() {
    let mut e = setup();
    world(&mut e, 0x10);
    let refr = looker(&mut e);
    let player = e.mem.u32(h::PLAYER);
    e.mem.set_u32(player + 0x68, 0);
    run(&mut e, 0x28e2, refr, &[]);
    assert_eq!(e.mem.u32(PICK_LOG + 8), 0);
    // Nor for a reference without 3D.
    let process = alloc(&mut e, 0x200);
    e.mem.set_u32(player + 0x68, process);
    let bare = reference(&mut e, REF_VTBL, 0x0100_0061, 0, 0);
    run(&mut e, 0x28e2, bare, &[]);
    assert_eq!(e.mem.u32(PICK_LOG + 8), 0);
}

/// Three globals (type 6) as script references 1-3, their values -1.
fn globals(e: &mut Engine) -> [u32; 3] {
    let g = [0, 1, 2].map(|i| {
        let at = FORMS + 0x5000 + 0x40 * i;
        form(e, at, 0x06, 0x0300_0001 + i);
        e.mem.set_f32(at + 0x24, -1.0);
        at
    });
    for (i, &at) in g.iter().enumerate() {
        set_ref(e, i as u16 + 1, at);
    }
    g
}

fn gvals(e: &Engine, g: [u32; 3]) -> [f32; 3] {
    g.map(|at| e.mem.f32(at + 0x24))
}

#[test]
fn ray_cast_pos_hit_interior_grid_and_seven_arguments() {
    let mut e = setup();
    let tes = world(&mut e, 0);
    let refr = looker(&mut e);
    let g = globals(&mut e);
    e.mem.set_u32(PICK_ANSWER, 0x0500_7000);
    e.mem.set_f32(PICK_FRACTION, 0.25);
    let args = vec![r(1), r(2), r(3), z(9.0), n(6), s(b""), z(1000.0)];
    assert_eq!(run(&mut e, 0x285e, refr, &args[..6]), 1.0);
    let k = h::to_havok();
    let from = [70.0 * k, -140.0 * k, 7.0 * k];
    let to = [
        (2.0 * 50000.0) * k + from[0],
        (5.0 * 50000.0) * k + from[1],
        (8.0 * 50000.0) * k + from[2],
    ];
    let want: [f32; 3] =
        std::array::from_fn(|i| (from[i] + (to[i] - from[i]) * 0.25) * h::from_havok());
    assert_eq!(gvals(&e, g), want);
    // No hit in an exterior without a grid cell there: nothing; seven arguments write anyway.
    e.mem.set_u32(PICK_ANSWER, 0);
    e.mem.set_f32(PICK_FRACTION, 0.0);
    let grid = alloc(&mut e, 0x28);
    let cells = alloc(&mut e, 4 * 25);
    e.mem.set_u32(grid + 4, 0);
    e.mem.set_u32(grid + 8, 0);
    e.mem.set_u32(grid + 0xc, 5);
    e.mem.set_u32(grid + 0x10, cells);
    e.mem.set_u32(tes + 8, grid);
    for gl in g {
        e.mem.set_f32(gl + 0x24, -1.0);
    }
    assert_eq!(run(&mut e, 0x285e, refr, &args[..6]), 0.0);
    assert_eq!(gvals(&e, g), [-1.0; 3]);
    assert_eq!(run(&mut e, 0x285e, refr, &args), 1.0);
    // The range argument: 1000.
    let to_x = (2.0 * 1000.0) * k + from[0];
    assert_eq!(logged_f32(&e, 0x10), to_x);
    // The start (70, -140) lies in cell (0, -1): x' = 2, y' = 1, index 1 + 2 + (2 << 2) = 11.
    e.mem.set_u32(cells + 4 * 11, 0x0500_8000);
    assert_eq!(run(&mut e, 0x285e, refr, &args[..6]), 1.0);
    e.mem.set_u32(cells + 4 * 11, 0);
    e.mem.set_u32(tes + 0x34, 0x0500_8100);
    assert_eq!(run(&mut e, 0x285e, refr, &args[..6]), 1.0);
}

#[test]
fn hit_coords_grid_index_follows_jips_shift() {
    let mut e = setup();
    let tes = alloc(&mut e, 0xc4);
    e.mem.set_u32(h::TES, tes);
    let grid = alloc(&mut e, 0x28);
    let cells = alloc(&mut e, 4 * 81);
    e.mem.set_u32(grid + 4, 10);
    e.mem.set_u32(grid + 8, (-3i32) as u32);
    e.mem.set_u32(grid + 0xc, 7);
    e.mem.set_u32(grid + 0x10, cells);
    e.mem.set_u32(tes + 8, grid);
    let at = |x: f32, y: f32| h::Cast {
        from: [x / h::from_havok(), y / h::from_havok(), 0.0, 0.0],
        to: [0.0; 4],
        fraction: 0.0,
        object: 0,
    };
    // Cell (11, -4): x' = 11 - 10 + 3 = 4, y' = -4 + 3 + 3 = 2; index 2 + 4 + (4 << 3) = 38.
    e.mem.set_u32(cells + 4 * 38, 1);
    let c = at(11.0 * 4096.0 + 5.0, -4.0 * 4096.0 + 5.0);
    let (coords, found) = h::hit_coords(&e, &c);
    assert!(found, "{coords:?}");
    e.mem.set_u32(cells + 4 * 38, 0);
    assert!(!h::hit_coords(&e, &c).1);
    // Outside the grid (x' = 7).
    assert!(!h::hit_coords(&e, &at(14.0 * 4096.0, 0.0)).1);
}

// ---- the crosshair ----

const VIEW_LOG: u32 = AREA + 0xe_0400;

#[test]
fn crosshair_ref_ex_interface_ref_or_the_pick() {
    let mut e = setup();
    let manager = alloc(&mut e, 0x200);
    e.map(0x011d_8000, 0x1000);
    e.mem.set_u32(0x011d_8a80, manager);
    let target = reference(&mut e, REF_VTBL, 0x0100_0080, 0, 0);
    e.mem.set_u32(manager + 0xfc, target);
    run(&mut e, 0x2875, 0, &[]);
    assert_eq!(form_result(&e), 0x0100_0080);
    e.mem.set_u32(manager + 0xfc, 0);
    let caster = alloc(&mut e, 0x10);
    e.mem.set_u32(manager + 0x13c, caster);
    let root = node(&mut e, b"Root", &[]);
    for (i, v) in [1.0f32, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0]
        .iter()
        .enumerate()
    {
        e.mem.set_f32(root + 0x68 + 4 * i as u32, *v);
    }
    let player = reference(&mut e, ACTOR_VTBL, 0x14, 0, root);
    e.mem.set_u32(h::PLAYER, player);
    set4(&mut e, player + 0xdd4, [10.0, 20.0, 30.0, 40.0]);
    set4(&mut e, player + 0xd58, [1.0, 2.0, 3.0, 4.0]);
    e.mem.set_u32(VIEW_LOG + 0x30, target);
    e.register(0x0063_1d60, |e, a| {
        e.mem.set_u32(VIEW_LOG, a[0]);
        for i in 0..4 {
            let w = e.mem.u32(a[1] + 4 * i);
            e.mem.set_u32(VIEW_LOG + 4 + 4 * i, w);
        }
        for i in 0..3 {
            let w = e.mem.u32(a[2] + 4 * i);
            e.mem.set_u32(VIEW_LOG + 0x14 + 4 * i, w);
        }
        e.mem.set_u32(VIEW_LOG + 0x20, a[3]);
        e.mem.set_u32(VIEW_LOG + 0x24, a[4].wrapping_sub(a[1]));
        e.mem.set_u32(VIEW_LOG + 0x28, a[5].wrapping_sub(a[1]));
        Ret {
            eax: e.mem.u32(VIEW_LOG + 0x30),
            ..Ret::default()
        }
    });
    let f = |e: &Engine, o: u32| e.mem.f32(VIEW_LOG + o);
    // First person: the camera position alone.
    run(&mut e, 0x2875, 0, &[]);
    assert_eq!(form_result(&e), 0x0100_0080);
    assert_eq!(e.mem.u32(VIEW_LOG), caster);
    assert_eq!([4, 8, 12, 16].map(|o| f(&e, o)), [10.0, 20.0, 30.0, 40.0]);
    assert_eq!([0x14, 0x18, 0x1c].map(|o| f(&e, o)), [2.0, 5.0, 8.0]);
    assert_eq!(e.mem.u32(VIEW_LOG + 0x20), 0x4640_0000);
    assert_eq!(e.mem.u32(VIEW_LOG + 0x24), 0xc);
    assert_eq!(e.mem.u32(VIEW_LOG + 0x28), 0xc);
    // Third person: plus the shoulder offset in all four lanes.
    e.mem.set_u8(player + 0x64b, 1);
    run(&mut e, 0x2875, 0, &[]);
    assert_eq!([4, 8, 12, 16].map(|o| f(&e, o)), [11.0, 22.0, 33.0, 44.0]);
    // Without the view caster nothing is picked.
    e.mem.set_u32(manager + 0x13c, 0);
    e.mem.set_u32(VIEW_LOG, 0);
    run(&mut e, 0x2875, 0, &[]);
    assert_eq!(form_result(&e), 0);
    assert_eq!(e.mem.u32(VIEW_LOG), 0);
}
