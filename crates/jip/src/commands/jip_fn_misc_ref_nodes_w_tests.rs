//! Tests of the scene-graph write commands on trees built by `nif_nodes_tests.rs`, every
//! expectation from JIP's command bodies (functions_jip/jip_fn_misc_ref.h, internal/
//! netimmerse.cpp, internal/Ni_types.cpp). The game functions the commands call are stand-ins
//! registered at their exe addresses, reached through the real vtable slots (written below as the
//! exe has them), and record the words they receive.

use super::super::nif_nodes::tests::*;
use super::multiply_matrices_inv;
use crate::commands::jip_fn_misc_ref::from_euler_pry;
use crate::commands::ni_matrix::{from_2_points, from_euler_pry_inv, multiply_matrices};
use crate::commands::nif_nodes::to_euler_pry_inv;
use crate::commands::test_support::{self as ts, call_on, calls_to, n, s, z, CALLS};
use engine_core::prelude::*;

/// `UpdateDownwardPass` (vtable +0xA4): `NiNode` `0109B5AC` holds `00A5DD70`, `NiTriShape`
/// `0109D454` `00A59F90`, `BSFadeNode` `010A8F90` `00B4DED0` (a jump to `00A5DD70`); each `RET 8`.
const NODE_UPDATE: u32 = 0x00a5_dd70;
const GEOMETRY_UPDATE: u32 = 0x00a5_9f90;
const FADE_UPDATE: u32 = 0x00b4_ded0;
/// `RemoveObject` (vtable +0xE8) of every node class: `00A5D8F0` (`RET 4`).
const REMOVE_OBJECT: u32 = 0x00a5_d8f0;
/// The game's fixed-string maker `00A5B690` (`cdecl`, one argument).
const ADD_STRING: u32 = 0x00a5_b690;
/// `NiControllerSequence::Activate` `00A34F20` (`RET 0x18`), `Deactivate` (vtable `01096584`
/// +0x8C: `00A35030`, `RET 8`).
const ACTIVATE: u32 = 0x00a3_4f20;
const DEACTIVATE: u32 = 0x00a3_5030;
/// `NiTextureTransformController` `01098184` +0x94: `00A46490` (`RET 4`).
const TTC_UPDATE: u32 = 0x00a4_6490;
/// Class vtables (RTTI names read in the exe).
const MANAGER_VTBL: u32 = 0x0109_619c;
const SEQUENCE_VTBL: u32 = 0x0109_6584;
const TTC_VTBL: u32 = 0x0109_8184;
const FLOAT_INTERP_VTBL: u32 = 0x0109_6f34;
const FLOAT_DATA_VTBL: u32 = 0x0102_dabc;
/// `Actor`'s vtable +0x100 (`IsActor`): `008D0360`.
const IS_ACTOR_TRUE: u32 = 0x008d_0360;
/// What `ADD_STRING`'s stand-in answers (a fixed string the test made), and the texts it got.
const STRING_ANSWER: u32 = AREA + 0xe_0000;
/// What `ACTIVATE`'s stand-in answers (its low byte).
const ACTIVATE_ANSWER: u32 = AREA + 0xe_0010;

fn setup() -> Engine {
    let mut e = scene_engine();
    ts::extra_stand_ins(&mut e);
    e.map(0x0109_6000, 0x3000);
    e.map(0x0102_d000, 0x1000);
    // `BSFadeNode`'s slots +0xA4 and +0xE8 lie on the page after the one the scene maps.
    e.map(0x010a_9000, 0x1000);
    e.mem.set_u32(NI_NODE + 0xa4, NODE_UPDATE);
    e.mem.set_u32(NI_TRI_SHAPE + 0xa4, GEOMETRY_UPDATE);
    e.mem.set_u32(BS_FADE_NODE + 0xa4, FADE_UPDATE);
    e.mem.set_u32(NI_NODE + 0xe8, REMOVE_OBJECT);
    e.mem.set_u32(BS_FADE_NODE + 0xe8, REMOVE_OBJECT);
    e.mem.set_u32(SEQUENCE_VTBL + 0x8c, DEACTIVATE);
    e.mem.set_u32(TTC_VTBL + 0x94, TTC_UPDATE);
    e.mem.set_u32(ACTOR_VTBL + 0x100, IS_ACTOR_TRUE);
    e.mem.set_u32(ACTIVATE_ANSWER, 1);
    // Updates and the controller update: the words, then the update data's three words.
    fn with_data(e: &Engine, f: u32, w: &[u32]) -> Ret {
        let mut words = w.to_vec();
        words.extend((0..3).map(|i| e.mem.u32(w[1] + 4 * i)));
        CALLS.with(|c| c.borrow_mut().push((f, words)));
        Ret::default()
    }
    fn plain(f: u32, w: &[u32]) -> Ret {
        CALLS.with(|c| c.borrow_mut().push((f, w.to_vec())));
        Ret::default()
    }
    e.register(NODE_UPDATE, |e, w| with_data(e, NODE_UPDATE, w));
    e.register(GEOMETRY_UPDATE, |e, w| with_data(e, GEOMETRY_UPDATE, w));
    e.register(FADE_UPDATE, |e, w| with_data(e, FADE_UPDATE, w));
    e.register(TTC_UPDATE, |e, w| with_data(e, TTC_UPDATE, w));
    e.register(REMOVE_OBJECT, |_, w| plain(REMOVE_OBJECT, w));
    e.register(DEACTIVATE, |_, w| plain(DEACTIVATE, w));
    e.register(ADD_STRING, |e, w| {
        // Records the text's bytes (up to 0x40) as words after the argument.
        let mut words = w.to_vec();
        let text: Vec<u8> = (0..0x40)
            .map(|i| e.mem.u8(w[0] + i))
            .take_while(|&b| b != 0)
            .collect();
        words.extend(text.iter().map(|&b| u32::from(b)));
        CALLS.with(|c| c.borrow_mut().push((ADD_STRING, words)));
        let s = e.mem.u32(STRING_ANSWER);
        if s != 0 {
            e.mem.set_u32(s - 8, e.mem.u32(s - 8) + 1);
        }
        Ret {
            eax: s,
            ..Ret::default()
        }
    });
    e.register(ACTIVATE, |e, w| {
        CALLS.with(|c| c.borrow_mut().push((ACTIVATE, w.to_vec())));
        Ret {
            eax: e.mem.u32(ACTIVATE_ANSWER),
            ..Ret::default()
        }
    });
    e
}

fn run(e: &mut Engine, opcode: u32, this: u32, args: &[Vec<u8>]) -> f64 {
    call_on(e, opcode, this, args).1
}

fn set3(e: &mut Engine, at: u32, v: [f32; 3]) {
    for (i, x) in v.into_iter().enumerate() {
        e.mem.set_f32(at + 4 * i as u32, x);
    }
}

fn get3(e: &Engine, at: u32) -> [f32; 3] {
    std::array::from_fn(|i| e.mem.f32(at + 4 * i as u32))
}

fn set9(e: &mut Engine, at: u32, m: [f32; 9]) {
    for (i, x) in m.into_iter().enumerate() {
        e.mem.set_f32(at + 4 * i as u32, x);
    }
}

fn get9(e: &Engine, at: u32) -> [f32; 9] {
    std::array::from_fn(|i| e.mem.f32(at + 4 * i as u32))
}

fn bits9(m: [f32; 9]) -> [u32; 9] {
    m.map(f32::to_bits)
}

/// A zeroed block of 0x100 bytes of the scene's memory (an object without a class).
fn raw(e: &mut Engine) -> u32 {
    object(e, 0, None)
}

/// A fixed string with its header (count +-8, length +-4), the count `count`.
fn counted(e: &mut Engine, text: &[u8], count: u32) -> u32 {
    let at = raw(e) + 0x10;
    e.mem.set_u32(at, count);
    e.mem.set_u32(at + 4, text.len() as u32);
    e.mem.write(at + 8, text);
    e.mem.set_u8(at + 8 + text.len() as u32, 0);
    at + 8
}

/// `Root { Body (node) { Head (shape) }, Arm (shape) }` under a plain reference.
struct Scene {
    refr: u32,
    root: u32,
    body: u32,
    head: u32,
    arm: u32,
}

fn scene(e: &mut Engine, actor: bool) -> Scene {
    let head = shape(e, b"Head");
    let body = node(e, b"Body", &[head]);
    let arm = shape(e, b"Arm");
    let root = node(e, b"Root", &[body, arm]);
    let refr = reference(e, 0x1234, actor, root);
    Scene {
        refr,
        root,
        body,
        head,
        arm,
    }
}

/// A collision object (flags +0xC) on `block` (+0x1C); its address.
fn collision(e: &mut Engine, block: u32) -> u32 {
    let c = raw(e);
    e.mem.set_u32(block + COLLISION_OBJECT, c);
    c
}

/// The calls made to the update functions, their words (this, data, flags, the data's words).
fn updates() -> Vec<(u32, Vec<u32>)> {
    ts::calls()
        .into_iter()
        .filter(|c| [NODE_UPDATE, GEOMETRY_UPDATE, FADE_UPDATE].contains(&c.0))
        .collect()
}

const PI_D_180: f32 = 0.017_453_292;

// ---- MultiplyMatricesInv ----

/// JIP's asm on inputs that round, run by lane H3's interpreter (`measure/jip-cmds/f2-sse.pl`,
/// `h3-sse.pl` loaded): A = 0.1 times, B = a third of lane H3's matrices. Also the asm's order by
/// hand, and a regrouped sum that the inputs tell apart.
#[test]
fn multiply_matrices_inv_bits() {
    let a = [
        0x3e19_999a,
        0xbe66_6666,
        0x3c4c_cccd,
        0x3e99_999a,
        0x3d4c_cccd,
        0xbe33_3333,
        0x3e4c_cccd,
        0x3d19_999a,
        0xbd80_0000,
    ]
    .map(f32::from_bits);
    let b = [
        0xbe2a_aaab,
        0x3ed5_5555,
        0x3f55_5555,
        0x3e80_0000,
        0xbf80_0000,
        0x3daa_aaab,
        0x3ec0_0000,
        0xbe95_5555,
        0x3faa_aaab,
    ]
    .map(f32::from_bits);
    let want = [
        0x3e00_0000,
        0x3d83_3333,
        0xbd8d_ddde,
        0xbe97_7778,
        0xbe1e_6666,
        0x3e4b_3333,
        0x3ed5_5556,
        0xbe08_8888,
        0xbdb3_3334,
    ];
    let got = multiply_matrices_inv(&a, &b);
    assert_eq!(bits9(got), want);
    // By hand: row i, column j = (b[i]*a[j] + b[3+i]*a[3+j]) + b[6+i]*a[6+j].
    let r10 = (b[1] * a[0] + b[4] * a[3]) + b[7] * a[6];
    let r22 = (b[2] * a[2] + b[5] * a[5]) + b[8] * a[8];
    assert_eq!((r10.to_bits(), r22.to_bits()), (want[3], want[8]));
    // The other grouping gives other bits for some element: the order is pinned.
    let regrouped: [f32; 9] = std::array::from_fn(|k| {
        let (i, j) = (k / 3, k % 3);
        b[i] * a[j] + (b[3 + i] * a[3 + j] + b[6 + i] * a[6 + j])
    });
    assert_ne!(bits9(regrouped), want);
    // And the transposed product (a's rows by b's columns) is another matrix.
    assert_ne!(bits9(multiply_matrices(&a, &b)), want);
}

// ---- SetNifBlockTranslation 0x284C ----

/// Transform 0 sets the local translation; then a node of a plain reference gets its collision
/// reset (bit 0x40 of the collision object's flags, below too) and its own update (`00A5DD70`:
/// the node, 12 zero bytes, 0).
#[test]
fn translation_set_resets_and_updates() {
    let mut e = setup();
    let sc = scene(&mut e, false);
    let col = collision(&mut e, sc.body);
    let below = collision(&mut e, sc.head);
    e.mem.set_u8(col + 0xc, 0x01);
    run(
        &mut e,
        0x284c,
        sc.refr,
        &[s(b"Body"), z(1.5), z(-2.0), z(0.1)],
    );
    assert_eq!(
        get3(&e, sc.body + LOCAL_TRANSLATE).map(f32::to_bits),
        [1.5f32, -2.0, 0.1].map(f32::to_bits)
    );
    assert_eq!(e.mem.u8(col + 0xc), 0x41);
    assert_eq!(e.mem.u8(below + 0xc), 0, "the head is no node: not walked");
    let u = updates();
    assert_eq!(u.len(), 1);
    assert_eq!((u[0].0, u[0].1.len(), u[0].1[0]), (NODE_UPDATE, 6, sc.body));
    assert_eq!(
        u[0].1[2..],
        [0, 0, 0, 0],
        "flags 0, then the data's 12 zero bytes"
    );
}

/// Transform 1 adds (local + t); any other adds t turned by the world rotation (`MultiplyVector`,
/// the word after the matrix the world translation's x): bits from lane H3's interpreter
/// (`h3-sse.pl mulvec`: A, after 9.5, (0.3, -1.7, 2.9) -> 40946667 c0a0cccd bfeccccd).
#[test]
fn translation_add_and_turned_add() {
    let mut e = setup();
    let sc = scene(&mut e, false);
    set3(&mut e, sc.arm + LOCAL_TRANSLATE, [1.25, -0.5, 0.1]);
    run(
        &mut e,
        0x284c,
        sc.refr,
        &[s(b"Arm"), z(0.3), z(0.7), z(-0.2), n(0), n(1)],
    );
    assert_eq!(
        get3(&e, sc.arm + LOCAL_TRANSLATE),
        [1.25f32 + 0.3f32, -0.5f32 + 0.7f32, 0.1f32 + -0.2f32]
    );
    // A geometry block: no collision reset, its own update (00A59F90).
    assert_eq!(updates().len(), 1);
    assert_eq!(updates()[0].0, GEOMETRY_UPDATE);
    assert_eq!(updates()[0].1[0], sc.arm);
    let a = [1.5f32, -2.25, 0.125, 3.0, 0.5, -1.75, 2.0, 0.375, -0.625];
    set9(&mut e, sc.arm + WORLD_ROTATE, a);
    e.mem.set_f32(sc.arm + WORLD_TRANSLATE, 9.5);
    set3(&mut e, sc.arm + LOCAL_TRANSLATE, [1.25, -0.5, 0.1]);
    for transform in [2, 7] {
        set3(&mut e, sc.arm + LOCAL_TRANSLATE, [1.25, -0.5, 0.1]);
        run(
            &mut e,
            0x284c,
            sc.refr,
            &[s(b"Arm"), z(0.3), z(-1.7), z(2.9), n(0), n(transform)],
        );
        let v = [0x4094_6667, 0xc0a0_cccd, 0xbfec_cccd].map(f32::from_bits);
        assert_eq!(
            get3(&e, sc.arm + LOCAL_TRANSLATE),
            [1.25f32 + v[0], -0.5f32 + v[1], 0.1f32 + v[2]],
            "transform {transform}"
        );
    }
    // A NaN after the matrix makes the last row's fourth product NaN (`haddps` sums it in); the
    // other rows' fourth floats are the next rows' first.
    e.mem.set_u32(sc.arm + WORLD_TRANSLATE, 0x7fc0_0000);
    run(
        &mut e,
        0x284c,
        sc.refr,
        &[s(b"Arm"), z(0.3), z(-1.7), z(2.9), n(0), n(2)],
    );
    let got = get3(&e, sc.arm + LOCAL_TRANSLATE);
    let v = [0x4094_6667, 0xc0a0_cccd].map(f32::from_bits);
    assert_eq!(
        (got[0], got[1]),
        ((1.25f32 + v[0]) + v[0], (-0.5f32 + v[1]) + v[1])
    );
    assert!(got[2].is_nan());
}

/// An empty name, a missing block or no 3D: nothing written, nothing called. An actor's node:
/// no collision reset, the update still. The player: argument 4 odd the third person root.
#[test]
fn translation_guards_actor_and_player() {
    let mut e = setup();
    let sc = scene(&mut e, false);
    set3(&mut e, sc.root + LOCAL_TRANSLATE, [7.0, 7.0, 7.0]);
    run(&mut e, 0x284c, sc.refr, &[s(b""), z(1.0), z(2.0), z(3.0)]);
    run(
        &mut e,
        0x284c,
        sc.refr,
        &[s(b"Nope"), z(1.0), z(2.0), z(3.0)],
    );
    assert_eq!(get3(&e, sc.root + LOCAL_TRANSLATE), [7.0; 3]);
    assert!(updates().is_empty());
    let bare = reference(&mut e, 0x99, false, 0);
    run(&mut e, 0x284c, bare, &[s(b"Root"), z(1.0), z(2.0), z(3.0)]);
    assert!(updates().is_empty());

    let ac = scene(&mut e, true);
    let col = collision(&mut e, ac.body);
    run(
        &mut e,
        0x284c,
        ac.refr,
        &[s(b"Body"), z(1.0), z(2.0), z(3.0)],
    );
    assert_eq!(e.mem.u8(col + 0xc), 0, "an actor's collision is not reset");
    assert_eq!(get3(&e, ac.body + LOCAL_TRANSLATE), [1.0, 2.0, 3.0]);
    assert_eq!(updates().len(), 1);

    let third = node(&mut e, b"Spine", &[]);
    let first = node(&mut e, b"Spine", &[]);
    let p = player(&mut e, third, first, false);
    run(
        &mut e,
        0x284c,
        p,
        &[s(b"Spine"), z(4.0), z(5.0), z(6.0), n(1)],
    );
    assert_eq!(get3(&e, third + LOCAL_TRANSLATE), [4.0, 5.0, 6.0]);
    assert_eq!(get3(&e, first + LOCAL_TRANSLATE), [0.0; 3]);
    run(
        &mut e,
        0x284c,
        p,
        &[s(b"Spine"), z(8.0), z(9.0), z(1.0), n(2)],
    );
    assert_eq!(get3(&e, first + LOCAL_TRANSLATE), [8.0, 9.0, 1.0]);
}

// ---- SetNifBlockRotation 0x284E ----

/// The angles in radians as JIP's `rot * GET_PS(8)` lanes make them (one float product each).
fn rad(v: [f32; 3]) -> [f32; 3] {
    v.map(|x| x * PI_D_180)
}

/// Transforms 0 (and 6, any other) `FromEulerPRY`, 2 and 3 `FromEulerPRYInv`, 1 the local times
/// `FromEulerPRYInv` (`Rotate`); a node of a plain reference reset and updated.
#[test]
fn rotation_modes_0_to_3() {
    let mut e = setup();
    let sc = scene(&mut e, false);
    let col = collision(&mut e, sc.body);
    let deg = [30.0f32, -45.5, 100.25];
    for (mode, want) in [
        (0, from_euler_pry(rad(deg))),
        (6, from_euler_pry(rad(deg))),
        (2, from_euler_pry_inv(rad(deg))),
        (3, from_euler_pry_inv(rad(deg))),
    ] {
        set9(&mut e, sc.body + LOCAL_ROTATE, [0.0; 9]);
        run(
            &mut e,
            0x284e,
            sc.refr,
            &[s(b"Body"), z(30.0), z(-45.5), z(100.25), n(mode)],
        );
        assert_eq!(
            bits9(get9(&e, sc.body + LOCAL_ROTATE)),
            bits9(want),
            "mode {mode}"
        );
    }
    let local = [0.1f32, 0.2, -0.3, 0.4, 0.5, 0.6, -0.7, 0.8, 0.9];
    set9(&mut e, sc.body + LOCAL_ROTATE, local);
    run(
        &mut e,
        0x284e,
        sc.refr,
        &[s(b"Body"), z(30.0), z(-45.5), z(100.25), n(1)],
    );
    let want = multiply_matrices(&local, &from_euler_pry_inv(rad(deg)));
    assert_eq!(bits9(get9(&e, sc.body + LOCAL_ROTATE)), bits9(want));
    assert_ne!(
        bits9(want),
        bits9(multiply_matrices(&from_euler_pry_inv(rad(deg)), &local)),
        "the order shows"
    );
    assert_eq!(e.mem.u8(col + 0xc), 0x40);
    assert_eq!(updates().len(), 5);
    assert!(updates()
        .iter()
        .all(|u| u.0 == NODE_UPDATE && u.1[0] == sc.body));
}

/// Transform 4: `FromEulerPRYInv(angles + world.ToEulerPRYInv())` then `MultiplyMatricesInv` by
/// the parent's world rotation; 5: `From2Points(world translation, the floats as a point)`, the
/// same. A root (no parent): nothing written or called (JIP stops the game there).
#[test]
fn rotation_modes_4_and_5() {
    let mut e = setup();
    let sc = scene(&mut e, false);
    let world = [0.36f32, 0.48, -0.8, -0.8, 0.6, 0.0, 0.48, 0.64, 0.6];
    let parent_world = [0.6f32, 0.0, -0.8, 0.0, 1.0, 0.0, 0.8, 0.0, 0.6];
    set9(&mut e, sc.head + WORLD_ROTATE, world);
    set9(&mut e, sc.body + WORLD_ROTATE, parent_world);
    run(
        &mut e,
        0x284e,
        sc.refr,
        &[s(b"Head"), z(10.0), z(20.0), z(-30.0), n(4)],
    );
    let now = to_euler_pry_inv(&world);
    let r = rad([10.0, 20.0, -30.0]);
    let m = from_euler_pry_inv([r[0] + now[0], r[1] + now[1], r[2] + now[2]]);
    let want = multiply_matrices_inv(&m, &parent_world);
    assert_eq!(bits9(get9(&e, sc.head + LOCAL_ROTATE)), bits9(want));
    assert_ne!(bits9(want), bits9(multiply_matrices_inv(&parent_world, &m)));

    set3(&mut e, sc.head + WORLD_TRANSLATE, [2.0, 5.0, 1.0]);
    run(
        &mut e,
        0x284e,
        sc.refr,
        &[s(b"Head"), z(1.0), z(3.0), z(3.0), n(5)],
    );
    let m = from_2_points([2.0, 5.0, 1.0], [1.0, 3.0, 3.0]);
    let want = multiply_matrices_inv(&m, &parent_world);
    assert_eq!(bits9(get9(&e, sc.head + LOCAL_ROTATE)), bits9(want));
    assert_eq!(updates().len(), 2);
    assert!(updates().iter().all(|u| u.0 == GEOMETRY_UPDATE));

    set9(&mut e, sc.root + LOCAL_ROTATE, [1.0; 9]);
    for mode in [4, 5] {
        run(
            &mut e,
            0x284e,
            sc.refr,
            &[s(b"Root"), z(1.0), z(3.0), z(3.0), n(mode)],
        );
    }
    assert_eq!(get9(&e, sc.root + LOCAL_ROTATE), [1.0; 9]);
    assert_eq!(updates().len(), 2);
}

/// The fifth argument picks the player's root (here the first person node), the fourth is the
/// transform; an actor's node is not reset; an empty name does nothing.
#[test]
fn rotation_player_actor_empty() {
    let mut e = setup();
    let third = node(&mut e, b"Spine", &[]);
    let first = node(&mut e, b"Spine", &[]);
    let p = player(&mut e, third, first, true);
    let col = collision(&mut e, first);
    run(
        &mut e,
        0x284e,
        p,
        &[s(b"Spine"), z(0.0), z(90.0), z(0.0), n(2), n(2)],
    );
    assert_eq!(
        bits9(get9(&e, first + LOCAL_ROTATE)),
        bits9(from_euler_pry_inv(rad([0.0, 90.0, 0.0])))
    );
    assert_eq!(get9(&e, third + LOCAL_ROTATE), [0.0; 9]);
    assert_eq!(e.mem.u8(col + 0xc), 0, "the player is an actor");
    assert_eq!(updates().len(), 1);
    run(&mut e, 0x284e, p, &[s(b""), z(0.0), z(90.0), z(0.0)]);
    assert_eq!(updates().len(), 1);
}

// ---- SetNifBlockScale 0x2850 ----

/// The local scale written; a node of a plain reference reset; the update.
#[test]
fn scale_written() {
    let mut e = setup();
    let sc = scene(&mut e, false);
    let col = collision(&mut e, sc.root);
    run(&mut e, 0x2850, sc.refr, &[s(b"Root"), z(0.3)]);
    assert_eq!(e.mem.f32(sc.root + LOCAL_SCALE).to_bits(), 0.3f32.to_bits());
    assert_eq!(e.mem.u8(col + 0xc), 0x40);
    assert_eq!(updates().len(), 1);
    assert_eq!(updates()[0].1[..3], [sc.root, updates()[0].1[1], 0]);
    assert_eq!(updates()[0].1[3..], [0, 0, 0]);
    run(&mut e, 0x2850, sc.refr, &[s(b""), z(2.0)]);
    run(&mut e, 0x2850, sc.refr, &[s(b"Nope"), z(2.0)]);
    assert_eq!(updates().len(), 1);
    // A fade node: its own slot (00B4DED0).
    let fade = object(&mut e, BS_FADE_NODE, Some(b"Fade"));
    set_children(&mut e, sc.root, &[sc.body, sc.arm, fade]);
    run(&mut e, 0x2850, sc.refr, &[s(b"Fade"), z(2.0)]);
    assert_eq!(updates()[1].0, FADE_UPDATE);
}

// ---- SetNifBlockFlag 0x2852 ----

/// Bit set or cleared (0-26; 27 rejected); the empty name is the root; no update call.
#[test]
fn flag_set_and_cleared() {
    let mut e = setup();
    let sc = scene(&mut e, false);
    e.mem.set_u32(sc.head + FLAGS, 0x8000_0001);
    run(&mut e, 0x2852, sc.refr, &[s(b"Head"), n(26), n(1)]);
    assert_eq!(e.mem.u32(sc.head + FLAGS), 0x8400_0001);
    run(&mut e, 0x2852, sc.refr, &[s(b"Head"), n(0), n(0)]);
    assert_eq!(e.mem.u32(sc.head + FLAGS), 0x8400_0000);
    run(&mut e, 0x2852, sc.refr, &[s(b"Head"), n(27), n(1)]);
    run(&mut e, 0x2852, sc.refr, &[s(b"Head"), n(-1), n(1)]);
    assert_eq!(e.mem.u32(sc.head + FLAGS), 0x8400_0000);
    run(&mut e, 0x2852, sc.refr, &[s(b""), n(3), n(5)]);
    assert_eq!(e.mem.u32(sc.root + FLAGS), 8);
    assert!(updates().is_empty());
}

// ---- SetMaterialPropertyValue 0x278D ----

/// A material (`NiMaterialProperty`) on the geometry block (+0xA4); its address.
fn material(e: &mut Engine, block: u32) -> u32 {
    let m = raw(e);
    e.mem.set_u32(block + 0xa4, m);
    m
}

/// Each trait to its field (specular +0x1C.., emissive +0x28.., glossiness +0x38, alpha +0x3C,
/// emit multiplier +0x40); trait 9, a value below 0 or a NaN rejected, -0 taken; a node or a
/// missing block or an empty name nothing.
#[test]
fn material_traits() {
    let mut e = setup();
    let sc = scene(&mut e, false);
    let m = material(&mut e, sc.head);
    let at = [0x1c, 0x20, 0x24, 0x28, 0x2c, 0x30, 0x38, 0x3c, 0x40];
    for (t, &off) in at.iter().enumerate() {
        run(
            &mut e,
            0x278d,
            sc.refr,
            &[s(b"Head"), n(t as i32), z(0.1 + t as f64)],
        );
        assert_eq!(
            e.mem.f32(m + off).to_bits(),
            ((0.1 + t as f64) as f32).to_bits(),
            "trait {t}"
        );
    }
    let before: Vec<u32> = (0..0x20).map(|i| e.mem.u32(m + 4 * i)).collect();
    run(&mut e, 0x278d, sc.refr, &[s(b"Head"), n(9), z(5.0)]);
    run(&mut e, 0x278d, sc.refr, &[s(b"Head"), n(0), z(-0.5)]);
    run(&mut e, 0x278d, sc.refr, &[s(b"Head"), n(0), z(f64::NAN)]);
    run(&mut e, 0x278d, sc.refr, &[s(b""), n(0), z(5.0)]);
    run(&mut e, 0x278d, sc.refr, &[s(b"Body"), n(0), z(5.0)]);
    let after: Vec<u32> = (0..0x20).map(|i| e.mem.u32(m + 4 * i)).collect();
    assert_eq!(before, after);
    run(&mut e, 0x278d, sc.refr, &[s(b"Head"), n(7), z(-0.0)]);
    assert_eq!(e.mem.u32(m + 0x3c), 0x8000_0000);
}

/// `*`: every geometry block below the root (depth first, nodes walked into), none of a node;
/// the player in first person: the first person node's blocks.
#[test]
fn material_star_recurses() {
    let mut e = setup();
    let sc = scene(&mut e, false);
    let deep = shape(&mut e, b"Deep");
    let mid = node(&mut e, b"Mid", &[0, deep]);
    set_children(&mut e, sc.body, &[sc.head, mid]);
    let (mh, md, ma) = (
        material(&mut e, sc.head),
        material(&mut e, deep),
        material(&mut e, sc.arm),
    );
    let node_mat = raw(&mut e);
    e.mem.set_u32(sc.body + 0xa4, node_mat);
    run(&mut e, 0x278d, sc.refr, &[s(b"*"), n(6), z(2.5)]);
    for m in [mh, md, ma] {
        assert_eq!(e.mem.f32(m + 0x38), 2.5);
    }
    assert_eq!(
        e.mem.u32(node_mat + 0x38),
        0,
        "a node's +0xA4 is not a material"
    );
    // "**" is a name, not the star.
    run(&mut e, 0x278d, sc.refr, &[s(b"**"), n(6), z(9.0)]);
    assert_eq!(e.mem.f32(mh + 0x38), 2.5);

    let shot = shape(&mut e, b"Gun");
    let first = node(&mut e, b"1st", &[shot]);
    let third = node(&mut e, b"3rd", &[]);
    let p = player(&mut e, third, first, false);
    let mg = material(&mut e, shot);
    run(&mut e, 0x278d, p, &[s(b"*"), n(8), z(4.0)]);
    assert_eq!(e.mem.f32(mg + 0x40), 4.0);
}

// ---- Animation sequences ----

/// A controller manager on `node` holding `slots` (0 an empty slot) as its sequences.
fn manager(e: &mut Engine, node: u32, slots: &[u32]) -> u32 {
    let m = object(e, MANAGER_VTBL, None);
    let data = raw(e);
    for (i, &s) in slots.iter().enumerate() {
        e.mem.set_u32(data + 4 * i as u32, s);
        if s != 0 {
            e.mem.set_u32(s + 0x40, m);
        }
    }
    e.mem.set_u32(m + 0x38, data);
    e.mem.set_u16(m + 0x3e, slots.len() as u16);
    e.mem.set_u32(node + 0xc, m);
    m
}

/// A sequence named `name` (none: a null name) with frequency `f`.
fn sequence(e: &mut Engine, name: Option<&[u8]>, f: f32) -> u32 {
    let s = object(e, SEQUENCE_VTBL, name);
    e.mem.set_f32(s + 0x28, f);
    s
}

/// The frequency of the named sequence, the name compared ignoring case; unnamed and empty
/// slots passed over; -1 for none, a dying actor, a controller of another class, no 3D.
#[test]
fn anim_sequence_frequency_read() {
    let mut e = setup();
    let sc = scene(&mut e, false);
    let unnamed = sequence(&mut e, None, 7.0);
    let walk = sequence(&mut e, Some(b"Walk"), 1.5);
    let open = sequence(&mut e, Some(b"Open"), 0.75);
    let m = manager(&mut e, sc.root, &[unnamed, 0, walk, open]);
    assert_eq!(run(&mut e, 0x285f, sc.refr, &[s(b"oPEN")]), 0.75);
    assert_eq!(run(&mut e, 0x285f, sc.refr, &[s(b"Walk")]), 1.5);
    assert_eq!(run(&mut e, 0x285f, sc.refr, &[s(b"Run")]), -1.0);
    assert_eq!(run(&mut e, 0x285f, sc.refr, &[s(b"")]), -1.0);
    e.mem.set_u32(m, MANAGER_VTBL + 4);
    assert_eq!(run(&mut e, 0x285f, sc.refr, &[s(b"Walk")]), -1.0);
    e.mem.set_u32(m, MANAGER_VTBL);
    let bare = reference(&mut e, 0x77, false, 0);
    assert_eq!(run(&mut e, 0x285f, bare, &[s(b"Walk")]), -1.0);
    // An actor: dying (life state 1) none; dead (2) still read.
    let ac = scene(&mut e, true);
    manager(&mut e, ac.root, &[walk]);
    e.mem.set_u32(ac.refr + 0x108, 1);
    assert_eq!(run(&mut e, 0x285f, ac.refr, &[s(b"Walk")]), -1.0);
    e.mem.set_u32(ac.refr + 0x108, 2);
    assert_eq!(run(&mut e, 0x285f, ac.refr, &[s(b"Walk")]), 1.5);
}

/// One sequence's frequency, or with `*` every one's (empty slots passed over); a dying actor's
/// none.
#[test]
fn anim_sequence_frequency_set() {
    let mut e = setup();
    let sc = scene(&mut e, false);
    let walk = sequence(&mut e, Some(b"Walk"), 1.5);
    let open = sequence(&mut e, Some(b"Open"), 0.75);
    manager(&mut e, sc.root, &[walk, 0, open]);
    run(&mut e, 0x2860, sc.refr, &[s(b"WALK"), z(0.3)]);
    assert_eq!(e.mem.f32(walk + 0x28).to_bits(), 0.3f32.to_bits());
    assert_eq!(e.mem.f32(open + 0x28), 0.75);
    run(&mut e, 0x2860, sc.refr, &[s(b"*"), z(2.0)]);
    assert_eq!((e.mem.f32(walk + 0x28), e.mem.f32(open + 0x28)), (2.0, 2.0));
    let ac = scene(&mut e, true);
    let idle = sequence(&mut e, Some(b"Idle"), 1.0);
    manager(&mut e, ac.root, &[idle]);
    e.mem.set_u32(ac.refr + 0x108, 1);
    run(&mut e, 0x2860, ac.refr, &[s(b"*"), z(5.0)]);
    assert_eq!(e.mem.f32(idle + 0x28), 1.0);
}

/// Every other sequence not named `Idle...` (case counts: the four bytes) deactivated, the
/// played one too, in order; then `Activate(0x64, 1, 1.0, 0, 0, 0)`; on success the offset
/// `-FLT_MAX`, the manager's bit 8, `MarkModified(0x10000000)`, result 1.
#[test]
fn play_anim_sequence_plays() {
    let mut e = setup();
    let sc = scene(&mut e, false);
    let idle = sequence(&mut e, Some(b"IdleLoop"), 1.0);
    let low = sequence(&mut e, Some(b"idle"), 1.0);
    let open = sequence(&mut e, Some(b"Open"), 1.0);
    let unnamed = sequence(&mut e, None, 1.0);
    let m = manager(&mut e, sc.body, &[idle, low, 0, unnamed, open]);
    e.mem.set_u8(m + 8, 0x41);
    let r = run(&mut e, 0x2904, sc.refr, &[s(b"OPEN"), s(b"Body")]);
    assert_eq!(r, 1.0);
    assert_eq!(
        calls_to(DEACTIVATE),
        vec![vec![low, 0, 0], vec![open, 0, 0]]
    );
    assert_eq!(
        calls_to(ACTIVATE),
        vec![vec![open, 0x64, 1, 0x3f80_0000, 0, 0, 0]]
    );
    assert_eq!(e.mem.u32(open + 0x48), 0xff7f_ffff);
    assert_eq!(e.mem.u8(m + 8), 0x49);
    assert_eq!(
        calls_to(ts::ADD_CHANGE_FN),
        vec![vec![ts::SAVE_LOAD, sc.refr, 0x1000_0000, 0]]
    );
    let order: Vec<u32> = ts::calls()
        .into_iter()
        .filter(|c| [DEACTIVATE, ACTIVATE, ts::ADD_CHANGE_FN].contains(&c.0))
        .map(|c| c.0)
        .collect();
    assert_eq!(order, [DEACTIVATE, DEACTIVATE, ACTIVATE, ts::ADD_CHANGE_FN]);
}

/// `Activate` answering false: no offset, no bit, no mark, result 0. No such sequence, an empty
/// node name with no manager on the root, a dying actor: nothing called.
#[test]
fn play_anim_sequence_fails() {
    let mut e = setup();
    let sc = scene(&mut e, false);
    let open = sequence(&mut e, Some(b"Open"), 1.0);
    let m = manager(&mut e, sc.root, &[open]);
    e.mem.set_u32(ACTIVATE_ANSWER, 0x100);
    assert_eq!(run(&mut e, 0x2904, sc.refr, &[s(b"Open")]), 0.0);
    assert_eq!(calls_to(ACTIVATE).len(), 1, "the empty node name: the root");
    assert_eq!(e.mem.u32(open + 0x48), 0);
    assert_eq!(e.mem.u8(m + 8), 0);
    assert!(calls_to(ts::ADD_CHANGE_FN).is_empty());
    e.mem.set_u32(ACTIVATE_ANSWER, 1);
    assert_eq!(run(&mut e, 0x2904, sc.refr, &[s(b"Close")]), 0.0);
    assert_eq!(run(&mut e, 0x2904, sc.refr, &[s(b"Open"), s(b"Head")]), 0.0);
    let ac = scene(&mut e, true);
    let s2 = sequence(&mut e, Some(b"Open"), 1.0);
    manager(&mut e, ac.root, &[s2]);
    e.mem.set_u32(ac.refr + 0x108, 1);
    assert_eq!(run(&mut e, 0x2904, ac.refr, &[s(b"Open")]), 0.0);
    assert_eq!(calls_to(ACTIVATE).len(), 1);
    assert_eq!(calls_to(DEACTIVATE).len(), 1, "only the first call's");
}

// ---- SetTextureTransformKey 0x28FB ----

/// A texturing property on `block` with the controllers `ctrls` chained (+0xC, then +0x30).
fn texturing(e: &mut Engine, block: u32, ctrls: &[u32]) -> u32 {
    let t = raw(e);
    e.mem.set_u32(block + 0xb0, t);
    let mut next = 0;
    for &c in ctrls.iter().rev() {
        e.mem.set_u32(c + 0x30, next);
        next = c;
    }
    e.mem.set_u32(t + 0xc, next);
    t
}

/// A texture transform controller with a float interpolator and float data of `n` keys.
fn ttc(e: &mut Engine, n: u32) -> (u32, u32, u32, u32) {
    let c = object(e, TTC_VTBL, None);
    let i = object(e, FLOAT_INTERP_VTBL, None);
    let d = object(e, FLOAT_DATA_VTBL, None);
    let keys = raw(e);
    e.mem.set_u32(c + 0x34, i);
    e.mem.set_u32(i + 0x10, d);
    e.mem.set_u32(d + 8, n);
    e.mem.set_u32(d + 0xc, keys);
    (c, i, d, keys)
}

/// Controller 1 (the second along the list), key 2: the value at keys + 8*2 + 4; then the
/// controller's `Update` (this, 12 zero bytes).
#[test]
fn texture_transform_key_written() {
    let mut e = setup();
    let sc = scene(&mut e, false);
    let (c0, ..) = ttc(&mut e, 4);
    let (c1, _, _, keys) = ttc(&mut e, 4);
    texturing(&mut e, sc.head, &[c0, c1]);
    run(&mut e, 0x28fb, sc.refr, &[s(b"Head"), n(1), n(2), z(0.3)]);
    assert_eq!(e.mem.f32(keys + 0x14).to_bits(), 0.3f32.to_bits());
    assert_eq!(e.mem.u32(keys + 0x10), 0, "the key's time is not touched");
    let up = calls_to(TTC_UPDATE);
    assert_eq!(up.len(), 1);
    assert_eq!(
        (up[0][0], up[0].len(), &up[0][2..]),
        (c1, 5, &[0, 0, 0][..])
    );
}

/// Key 4 of 4, a controller past the list's end, another controller class, interpolator class or
/// data class, a node, no texturing property: nothing written or called. A list that loops with
/// a huge index ends.
#[test]
fn texture_transform_key_rejected() {
    let mut e = setup();
    let sc = scene(&mut e, false);
    let (c, i, d, keys) = ttc(&mut e, 4);
    texturing(&mut e, sc.head, &[c]);
    run(&mut e, 0x28fb, sc.refr, &[s(b"Head"), n(0), n(4), z(1.0)]);
    run(&mut e, 0x28fb, sc.refr, &[s(b"Head"), n(1), n(0), z(1.0)]);
    run(&mut e, 0x28fb, sc.refr, &[s(b"Body"), n(0), n(0), z(1.0)]);
    run(&mut e, 0x28fb, sc.refr, &[s(b"Arm"), n(0), n(0), z(1.0)]);
    for (obj, vt) in [(c, TTC_VTBL), (i, FLOAT_INTERP_VTBL), (d, FLOAT_DATA_VTBL)] {
        e.mem.set_u32(obj, vt + 4);
        run(&mut e, 0x28fb, sc.refr, &[s(b"Head"), n(0), n(0), z(1.0)]);
        e.mem.set_u32(obj, vt);
    }
    assert!((0..8).all(|k| e.mem.u32(keys + 4 * k) == 0));
    assert!(calls_to(TTC_UPDATE).is_empty());
    // A loop: c's next is c; index 0x7FFFFFFF is walked at most the bound and finds nothing.
    e.mem.set_u32(c + 0x30, c);
    run(
        &mut e,
        0x28fb,
        sc.refr,
        &[s(b"Head"), n(0x7fff_ffff), n(0), z(1.0)],
    );
    assert!(calls_to(TTC_UPDATE).is_empty());
    // 65,536 steps along the loop are still taken (the bound is not below that).
    run(
        &mut e,
        0x28fb,
        sc.refr,
        &[s(b"Head"), n(0x1_0000), n(1), z(1.0)],
    );
    assert_eq!(calls_to(TTC_UPDATE).len(), 1);
    run(
        &mut e,
        0x28fb,
        sc.refr,
        &[s(b"Head"), n(0x1_0001), n(1), z(2.0)],
    );
    assert_eq!(calls_to(TTC_UPDATE).len(), 1);
}

// ---- RenameNifBlock 0x2900 ----

/// The new name's string from the game's table (one word, the text), stored; the old name's
/// count down; result 1. The same string again: its new count given back. The empty name is the
/// root.
#[test]
fn rename_nif_block() {
    let mut e = setup();
    let sc = scene(&mut e, false);
    let old = counted(&mut e, b"Arm", 3);
    e.mem.set_u32(sc.arm + NAME, old);
    let new = counted(&mut e, b"Wing", 1);
    e.mem.set_u32(STRING_ANSWER, new);
    assert_eq!(run(&mut e, 0x2900, sc.refr, &[s(b"Arm"), s(b"Wing")]), 1.0);
    assert_eq!(e.mem.u32(sc.arm + NAME), new);
    assert_eq!((e.mem.u32(old - 8), e.mem.u32(new - 8)), (2, 2));
    let calls = calls_to(ADD_STRING);
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0][1..], b"Wing".map(u32::from));
    // Again to the same name: counted up by the table, then down again.
    assert_eq!(run(&mut e, 0x2900, sc.refr, &[s(b"Wing"), s(b"Wing")]), 1.0);
    assert_eq!(e.mem.u32(new - 8), 2);
    // A block without a name: nothing to count down. The root by the empty name.
    e.mem.set_u32(sc.root + NAME, 0);
    assert_eq!(run(&mut e, 0x2900, sc.refr, &[s(b""), s(b"Wing")]), 1.0);
    assert_eq!((e.mem.u32(sc.root + NAME), e.mem.u32(new - 8)), (new, 3));
    // No such block: no call, result 0.
    assert_eq!(run(&mut e, 0x2900, sc.refr, &[s(b"Nope"), s(b"X")]), 0.0);
    assert_eq!(calls_to(ADD_STRING).len(), 3);
}

// ---- RemoveNifBlock 0x2901 ----

/// The parent's `RemoveObject(block)` (two words), result 1; the root (no parent): nothing,
/// result 0.
#[test]
fn remove_nif_block() {
    let mut e = setup();
    let sc = scene(&mut e, false);
    assert_eq!(run(&mut e, 0x2901, sc.refr, &[s(b"Head")]), 1.0);
    assert_eq!(calls_to(REMOVE_OBJECT), vec![vec![sc.body, sc.head]]);
    assert_eq!(run(&mut e, 0x2901, sc.refr, &[s(b"")]), 0.0);
    assert_eq!(run(&mut e, 0x2901, sc.refr, &[s(b"Nope")]), 0.0);
    // A parent that is not a node: nothing (JIP would call through it).
    let cam = object(&mut e, NI_CAMERA, Some(b"Cam"));
    e.mem.set_u32(sc.arm + PARENT, cam);
    assert_eq!(run(&mut e, 0x2901, sc.refr, &[s(b"Arm")]), 0.0);
    assert_eq!(calls_to(REMOVE_OBJECT).len(), 1);
    let _ = sc.root;
}

/// The `*` walk's depth bound at its edge (as `nif_nodes.rs`' walks): a geometry block 511 levels
/// below the root is reached, one 512 below is not.
#[test]
fn material_star_depth_edge() {
    for (level, reached) in [(511u32, true), (512, false)] {
        let mut e = setup();
        let leaf = shape(&mut e, b"Leaf");
        let m = material(&mut e, leaf);
        // The leaf is at `level`: level - 1 nodes above it under the root.
        let mut top = leaf;
        for _ in 1..level {
            top = node(&mut e, b"N", &[top]);
        }
        let root = node(&mut e, b"Root", &[top]);
        let refr = reference(&mut e, 0x1234, false, root);
        run(&mut e, 0x278d, refr, &[s(b"*"), n(7), z(0.5)]);
        assert_eq!(e.mem.f32(m + 0x3c) == 0.5, reached, "level {level}");
    }
}
