//! Tests of the read-only scene-graph commands on trees built by `nif_nodes_tests.rs`, every
//! expectation from JIP's command bodies (functions_jip/jip_fn_misc_ref.h).

use super::super::nif_nodes::tests::*;
use super::super::nif_nodes::{
    BS_BOUND_VTBL, COLLISION_OBJECT, FLAGS, GET_SCALE, LOCAL_ROTATE, LOCAL_SCALE, LOCAL_TRANSLATE,
    WORLD_ROTATE, WORLD_TRANSLATE,
};
use crate::commands::test_support::*;
use crate::test_support::LIST;
use engine_core::prelude::*;
use nvse::vars::Data;

/// The scale `GetScale` answers (a float word), the calls to it counted at the next word.
const SCALE_AT: u32 = AREA + 0xe_0000;
const SCALE_CALLS: u32 = SCALE_AT + 4;
/// A bound base form (type 0x1F), its vtable with `IsBoundObject` (+0xE8) answering the byte at
/// [`BOUND_ANSWER`].
const BASE_FORM: u32 = AREA + 0xe_0100;
const BASE_VTBL: u32 = AREA + 0xe_0200;
const IS_BOUND_FN: u32 = 0x00de_00e8;
const BOUND_ANSWER: u32 = AREA + 0xe_0400;
const LOCALS: u32 = LIST + 0x100;

fn setup() -> Engine {
    let mut e = scene_engine();
    e.mem.set_f32(SCALE_AT, 1.0);
    e.register(GET_SCALE, |e, _| {
        let n = e.mem.u32(SCALE_CALLS);
        e.mem.set_u32(SCALE_CALLS, n + 1);
        Ret {
            st0: f64::from(e.mem.f32(SCALE_AT)),
            ..Ret::default()
        }
    });
    e.mem.set_u32(BASE_FORM, BASE_VTBL);
    e.mem.set_u8(BASE_FORM + 4, 0x1f);
    e.mem.set_u32(BASE_FORM + 0xc, 0x0000_0201);
    e.mem.set_u32(BASE_VTBL + 0xe8, IS_BOUND_FN);
    e.mem.set_u32(BOUND_ANSWER, 1);
    e.register(IS_BOUND_FN, |e, _| Ret {
        eax: e.mem.u32(BOUND_ANSWER),
        ..Ret::default()
    });
    e
}

fn run(e: &mut Engine, opcode: u32, this: u32, args: &[Vec<u8>]) -> f64 {
    call_on(e, opcode, this, args).1
}

fn axis(a: u8) -> Vec<u8> {
    vec![a]
}

/// A script variable argument (`f` and its id).
fn v(id: u16) -> Vec<u8> {
    let mut b = vec![b'f'];
    b.extend_from_slice(&id.to_le_bytes());
    b
}

/// Script variables `1..=n` in the event list, each 0x10 bytes (id, data at +8), set to -1.
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
    [1, 2, 3].map(|i| e.mem.f64(LOCALS + 0x100 + (i - 1) * 0x10 + 8))
}

fn set3(e: &mut Engine, at: u32, v: [f32; 3]) {
    for (i, x) in v.into_iter().enumerate() {
        e.mem.set_f32(at + 4 * i as u32, x);
    }
}

fn set_matrix(e: &mut Engine, at: u32, m: [f32; 9]) {
    for (i, x) in m.into_iter().enumerate() {
        e.mem.set_f32(at + 4 * i as u32, x);
    }
}

fn numbers(e: &Engine, id: f64) -> Vec<f64> {
    array_data(e, id)
        .into_iter()
        .map(|d| match d {
            Data::Number(n) => n,
            other => panic!("not a number: {other:?}"),
        })
        .collect()
}

fn texts(e: &Engine, id: f64) -> Vec<Vec<u8>> {
    array_data(e, id)
        .into_iter()
        .map(|d| match d {
            Data::Str(s) => s,
            other => panic!("not a text: {other:?}"),
        })
        .collect()
}

fn t(s: &str) -> Vec<u8> {
    s.as_bytes().to_vec()
}

/// A reference whose loaded root is `Root { Body { Head (shape) }, Arm (shape) }`; the blocks.
struct Scene {
    refr: u32,
    root: u32,
    body: u32,
    head: u32,
    arm: u32,
}

fn scene(e: &mut Engine) -> Scene {
    let head = shape(e, b"Head");
    let body = node(e, b"Body", &[head]);
    let arm = shape(e, b"Arm");
    let root = node(e, b"Root", &[body, arm]);
    let refr = reference(e, 0x1234, false, root);
    e.mem.set_u32(refr + 0x20, BASE_FORM);
    Scene {
        refr,
        root,
        body,
        head,
        arm,
    }
}

// ---- GetObjectDimensions 0x262C ----

/// With a bounding box: `(centre + dimensions) * scale` on the axis; without one the base
/// form's `|max - min| * scale`; a given bound object's bounds at scale 1 (no scale call).
#[test]
fn object_dimensions() {
    let mut e = setup();
    let s = scene(&mut e);
    let b = bound(&mut e, BS_BOUND_VTBL, [1.0, 2.0, 3.0], [10.0, 20.0, 30.0]);
    set_extra(&mut e, s.root, &[b]);
    e.mem.set_f32(SCALE_AT, 2.0);
    assert_eq!(run(&mut e, 0x262c, s.refr, &[axis(b'X')]), 22.0);
    assert_eq!(run(&mut e, 0x262c, s.refr, &[axis(b'Y')]), 44.0);
    assert_eq!(run(&mut e, 0x262c, s.refr, &[axis(b'Z')]), 66.0);
    // Float maths: (0.1 + 0.2) * 3 in floats.
    set3(&mut e, b + 0xc, [0.1, 0.0, 0.0]);
    set3(&mut e, b + 0x18, [0.2, 0.0, 0.0]);
    e.mem.set_f32(SCALE_AT, 3.0);
    let want = f64::from((0.1f32 + 0.2f32) * 3.0f32);
    assert_eq!(run(&mut e, 0x262c, s.refr, &[axis(b'X')]), want);
    // No bounding box: the base form's bounds (min -5 2 0, max 7 -6 9), made positive.
    set_extra(&mut e, s.root, &[]);
    for (i, v) in [-5i16, 2, 0, 7, -6, 9].into_iter().enumerate() {
        e.mem.set_u16(BASE_FORM + 0x24 + 2 * i as u32, v as u16);
    }
    e.mem.set_f32(SCALE_AT, 1.5);
    assert_eq!(run(&mut e, 0x262c, s.refr, &[axis(b'X')]), 18.0);
    assert_eq!(
        run(&mut e, 0x262c, s.refr, &[axis(b'Y')]),
        12.0,
        "|(-6 - 2) * 1.5|"
    );
    e.mem.set_f32(SCALE_AT, -1.0);
    assert_eq!(
        run(&mut e, 0x262c, s.refr, &[axis(b'Z')]),
        9.0,
        "abs after the scale"
    );
    // A given bound object: scale 1, no GetScale.
    set_ref(&mut e, 1, BASE_FORM);
    e.mem.set_u32(SCALE_CALLS, 0);
    assert_eq!(run(&mut e, 0x262c, s.refr, &[axis(b'Y'), r(1)]), 8.0);
    assert_eq!(
        run(&mut e, 0x262c, 0, &[axis(b'X'), r(1)]),
        12.0,
        "no reference needed"
    );
    assert_eq!(e.mem.u32(SCALE_CALLS), 0);
    // No reference and no object: nothing.
    assert_eq!(run(&mut e, 0x262c, 0, &[axis(b'X')]), 0.0);
    // A reference without a base form and without bounds: nothing.
    e.mem.set_u32(s.refr + 0x20, 0);
    assert_eq!(run(&mut e, 0x262c, s.refr, &[axis(b'X')]), 0.0);
    // An axis byte that is not X, Y or Z: nothing.
    e.mem.set_u32(s.refr + 0x20, BASE_FORM);
    assert_eq!(run(&mut e, 0x262c, s.refr, &[axis(b'W')]), 0.0);
}

/// An actor with a high process takes its process' bound, even when the node has one.
#[test]
fn object_dimensions_actor_process() {
    let mut e = setup();
    let s = scene(&mut e);
    let node_bound = bound(&mut e, BS_BOUND_VTBL, [0.0; 3], [1.0; 3]);
    set_extra(&mut e, s.root, &[node_bound]);
    let actor = reference(&mut e, 0x2000, true, s.root);
    e.mem.set_u32(actor + 0x20, BASE_FORM);
    let process = AREA + 0xe_1000;
    let pb = bound(&mut e, BS_BOUND_VTBL, [4.0; 3], [5.0; 3]);
    e.mem.set_u32(process + 0x224, pb);
    e.mem.set_u32(process + 0x28, 1);
    e.mem.set_u32(actor + 0x68, process);
    assert_eq!(run(&mut e, 0x262c, actor, &[axis(b'X')]), 9.0);
    e.mem.set_u32(process + 0x28, 3);
    assert_eq!(
        run(&mut e, 0x262c, actor, &[axis(b'X')]),
        1.0,
        "low process: the node's"
    );
}

// ---- GetCenterPos 0x276F ----

/// Position plus the bounding box's centre times the scale; without a box the middle of a bound
/// base form's bounds; neither: the position.
#[test]
fn center_pos() {
    let mut e = setup();
    let s = scene(&mut e);
    set3(&mut e, s.refr + 0x30, [100.0, 200.0, 300.0]);
    let b = bound(&mut e, BS_BOUND_VTBL, [1.0, -2.0, 3.5], [9.0; 3]);
    set_extra(&mut e, s.root, &[b]);
    e.mem.set_f32(SCALE_AT, 2.0);
    assert_eq!(run(&mut e, 0x276f, s.refr, &[axis(b'X')]), 102.0);
    assert_eq!(run(&mut e, 0x276f, s.refr, &[axis(b'Y')]), 196.0);
    assert_eq!(run(&mut e, 0x276f, s.refr, &[axis(b'Z')]), 307.0);
    // The base form: (min + max) * 0.5 (bounds -5 2 0 / 8 -6 9).
    set_extra(&mut e, s.root, &[]);
    for (i, v) in [-5i16, 2, 0, 8, -6, 9].into_iter().enumerate() {
        e.mem.set_u16(BASE_FORM + 0x24 + 2 * i as u32, v as u16);
    }
    assert_eq!(run(&mut e, 0x276f, s.refr, &[axis(b'X')]), 103.0);
    assert_eq!(run(&mut e, 0x276f, s.refr, &[axis(b'Y')]), 196.0);
    assert_eq!(run(&mut e, 0x276f, s.refr, &[axis(b'Z')]), 309.0);
    // Float maths: 0.1 + 1.5 * 0.3 in floats.
    set3(&mut e, s.refr + 0x30, [0.1, 0.0, 0.0]);
    e.mem.set_f32(SCALE_AT, 0.3);
    for (i, v) in [1i16, 0, 0, 2, 0, 0].into_iter().enumerate() {
        e.mem.set_u16(BASE_FORM + 0x24 + 2 * i as u32, v as u16);
    }
    let want = f64::from(0.1f32 + 1.5f32 * 0.3f32);
    assert_eq!(run(&mut e, 0x276f, s.refr, &[axis(b'X')]), want);
    // A base form that is not a bound object: centre 0.
    e.mem.set_u32(BOUND_ANSWER, 0);
    set3(&mut e, s.refr + 0x30, [5.0, 0.0, 0.0]);
    assert_eq!(run(&mut e, 0x276f, s.refr, &[axis(b'X')]), 5.0);
    e.mem.set_u32(BOUND_ANSWER, 1);
    // No 3D at all and no base form: the position.
    set_root(&mut e, s.refr, 0);
    e.mem.set_u32(s.refr + 0x20, 0);
    assert_eq!(run(&mut e, 0x276f, s.refr, &[axis(b'X')]), 5.0);
    assert_eq!(
        run(&mut e, 0x276f, s.refr, &[axis(b'Q')]),
        0.0,
        "not an axis"
    );
}

// ---- GetMaterialPropertyValue 0x278C ----

/// Traits 0-8 of a geometry block's material; nothing past 8, for a node, or without a material.
#[test]
fn material_property_value() {
    let mut e = setup();
    let s = scene(&mut e);
    let mat = AREA + 0xe_2000;
    let vals = [0.1f32, 0.2, 0.3, 0.4, 0.5, 0.6, 7.5, 0.25, 1.75];
    for (i, at) in [0x1c, 0x20, 0x24, 0x28, 0x2c, 0x30, 0x38, 0x3c, 0x40]
        .into_iter()
        .enumerate()
    {
        e.mem.set_f32(mat + at, vals[i]);
    }
    e.mem.set_f32(mat + 0x34, 99.0);
    e.mem.set_u32(s.head + 0xa4, mat);
    for (id, want) in vals.iter().enumerate() {
        let got = run(&mut e, 0x278c, s.refr, &[s_(b"Head"), n(id as i32)]);
        assert_eq!(got, f64::from(*want), "trait {id}");
    }
    assert_eq!(run(&mut e, 0x278c, s.refr, &[s_(b"Head"), n(9)]), 0.0);
    assert_eq!(run(&mut e, 0x278c, s.refr, &[s_(b"Head"), n(-1)]), 0.0);
    e.mem.set_u32(s.body + 0xa4, mat);
    assert_eq!(
        run(&mut e, 0x278c, s.refr, &[s_(b"Body"), n(6)]),
        0.0,
        "a node"
    );
    assert_eq!(
        run(&mut e, 0x278c, s.refr, &[s_(b"Arm"), n(6)]),
        0.0,
        "no material"
    );
    assert_eq!(run(&mut e, 0x278c, s.refr, &[s_(b"Nope"), n(6)]), 0.0);
    // The player's first person model when the camera is first person.
    let fp_head = shape(&mut e, b"Head");
    e.mem.set_u32(fp_head + 0xa4, mat + 4);
    let first = node(&mut e, b"1st", &[fp_head]);
    let p = player(&mut e, s.root, first, false);
    assert_eq!(
        run(&mut e, 0x278c, p, &[s_(b"Head"), n(0)]),
        f64::from(0.2f32)
    );
}

fn s_(text: &[u8]) -> Vec<u8> {
    s(text)
}

// ---- GetNifBlockTranslation 0x284B / Alt 0x290D ----

#[test]
fn block_translation() {
    let mut e = setup();
    let sc = scene(&mut e);
    set3(&mut e, sc.head + LOCAL_TRANSLATE, [1.0, 2.5, -3.0]);
    set3(&mut e, sc.head + WORLD_TRANSLATE, [10.0, 20.0, 1e10]);
    let id = run(&mut e, 0x284b, sc.refr, &[s(b"Head")]);
    assert_eq!(numbers(&e, id), vec![1.0, 2.5, -3.0]);
    let id = run(&mut e, 0x284b, sc.refr, &[s(b"Head"), n(5)]);
    assert_eq!(numbers(&e, id), vec![10.0, 20.0, f64::from(1e10f32)]);
    assert_eq!(run(&mut e, 0x284b, sc.refr, &[s(b"Missing")]), 0.0);
    // An empty name: the root.
    set3(&mut e, sc.root + LOCAL_TRANSLATE, [7.0, 8.0, 9.0]);
    let id = run(&mut e, 0x284b, sc.refr, &[s(b"")]);
    assert_eq!(numbers(&e, id), vec![7.0, 8.0, 9.0]);
    set_root(&mut e, sc.refr, 0);
    assert_eq!(run(&mut e, 0x284b, sc.refr, &[s(b"Head")]), 0.0, "no 3D");
}

#[test]
fn block_translation_pc_node() {
    let mut e = setup();
    let sc = scene(&mut e);
    let fp_head = shape(&mut e, b"Head");
    set3(&mut e, fp_head + LOCAL_TRANSLATE, [4.0, 4.0, 4.0]);
    set3(&mut e, sc.head + LOCAL_TRANSLATE, [3.0, 3.0, 3.0]);
    let first = node(&mut e, b"1st", &[fp_head]);
    let p = player(&mut e, sc.root, first, true);
    let id = run(&mut e, 0x284b, p, &[s(b"Head"), n(0), n(2)]);
    assert_eq!(numbers(&e, id), vec![4.0; 3], "2: first person");
    let id = run(&mut e, 0x284b, p, &[s(b"Head"), n(0), n(0)]);
    assert_eq!(numbers(&e, id), vec![3.0; 3], "0: the camera's");
}

#[test]
fn block_translation_alt() {
    let mut e = setup();
    let sc = scene(&mut e);
    locals(&mut e, 3);
    set3(&mut e, sc.head + LOCAL_TRANSLATE, [1.0, 2.0, 0.1]);
    set3(&mut e, sc.head + WORLD_TRANSLATE, [5.0, 6.0, 7.0]);
    let args = |name: &[u8], world: i32| vec![s(name), v(1), v(2), v(3), n(world), n(0)];
    assert_eq!(run(&mut e, 0x290d, sc.refr, &args(b"Head", 0)), 1.0);
    assert_eq!(vars(&e), [1.0, 2.0, f64::from(0.1f32)]);
    assert_eq!(run(&mut e, 0x290d, sc.refr, &args(b"Head", 1)), 1.0);
    assert_eq!(vars(&e), [5.0, 6.0, 7.0]);
    locals(&mut e, 3);
    assert_eq!(run(&mut e, 0x290d, sc.refr, &args(b"None", 1)), 0.0);
    assert_eq!(vars(&e), [-1.0; 3], "not found: variables untouched");
    // Variables x, y, z written in that order (one variable for all three: z last).
    assert_eq!(
        run(&mut e, 0x290d, sc.refr, &[s(b"Head"), v(2), v(2), v(2)]),
        1.0
    );
    assert_eq!(vars(&e), [-1.0, 0.1f32.into(), -1.0]);
    // A missing variable: the read fails.
    locals(&mut e, 2);
    assert_eq!(
        run(&mut e, 0x290d, sc.refr, &[s(b"Head"), v(1), v(2), v(3)]),
        0.0
    );
}

// ---- GetNifBlockRotation 0x284D / Alt 0x290E ----

/// Mode bit 1 world else local; bit 2 `ToEulerPRYInv` else `ToEulerPRY`; each angle times
/// `Flt180dPI` in floats.
#[test]
fn block_rotation() {
    use crate::commands::ln_fn_math::{asin, atan2};
    let mut e = setup();
    let sc = scene(&mut e);
    let local = [0.5, 0.25, 0.125, -0.75, 0.5, 0.0625, 0.3, -0.2, 0.9];
    let world = [0.0, 1.0, 0.0, -1.0, 0.0, 0.0, 0.0, 0.0, 1.0];
    set_matrix(&mut e, sc.head + LOCAL_ROTATE, local);
    set_matrix(&mut e, sc.head + WORLD_ROTATE, world);
    let k = 57.295_78f32;
    let deg = |a: [f32; 3]| a.map(|x| f64::from(x * k)).to_vec();
    let pry = |m: [f32; 9]| [atan2(m[5], m[8]), asin(-m[2]), atan2(m[1], m[0])];
    let inv = |m: [f32; 9]| [atan2(m[7], m[8]), asin(-m[6]), atan2(m[3], m[0])];
    for (mode, want) in [
        (0, deg(pry(local))),
        (1, deg(pry(world))),
        (2, deg(inv(local))),
        (3, deg(inv(world))),
        (4, deg(pry(local))),
    ] {
        let id = run(&mut e, 0x284d, sc.refr, &[s(b"Head"), n(mode)]);
        assert_eq!(numbers(&e, id), want, "mode {mode}");
    }
    let id = run(&mut e, 0x284d, sc.refr, &[s(b"Head"), n(1)]);
    assert_eq!(
        numbers(&e, id)[2],
        f64::from(std::f32::consts::FRAC_PI_2 * k)
    );
    assert_eq!(run(&mut e, 0x284d, sc.refr, &[s(b"Nope")]), 0.0);
    // Alt: the same angles into the variables; result 1.
    locals(&mut e, 3);
    let got = run(
        &mut e,
        0x290e,
        sc.refr,
        &[s(b"Head"), v(1), v(2), v(3), n(2), n(0)],
    );
    assert_eq!(got, 1.0);
    assert_eq!(vars(&e).to_vec(), deg(inv(local)));
    locals(&mut e, 3);
    assert_eq!(
        run(&mut e, 0x290e, sc.refr, &[s(b"Nope"), v(1), v(2), v(3)]),
        0.0
    );
    assert_eq!(vars(&e), [-1.0; 3]);
    assert_eq!(
        run(
            &mut e,
            0x290e,
            sc.refr,
            &[s(b"Head"), v(1), v(2), v(3), n(1)]
        ),
        1.0
    );
    assert_eq!(vars(&e).to_vec(), deg(pry(world)));
}

// ---- GetNifBlockScale 0x284F, GetNifBlockFlag 0x2851 ----

#[test]
fn block_scale_and_flag() {
    let mut e = setup();
    let sc = scene(&mut e);
    e.mem.set_f32(sc.arm + LOCAL_SCALE, 1.25);
    e.mem.set_f32(sc.arm + LOCAL_SCALE + 0x34, 9.0);
    assert_eq!(run(&mut e, 0x284f, sc.refr, &[s(b"Arm")]), 1.25, "local");
    assert_eq!(run(&mut e, 0x284f, sc.refr, &[s(b"Hand")]), 0.0);
    e.mem.set_u32(sc.arm + FLAGS, 0x8000_0001);
    for (bit, want) in [(0, 1.0), (1, 0.0), (31, 1.0), (32, 0.0)] {
        assert_eq!(
            run(&mut e, 0x2851, sc.refr, &[s(b"Arm"), n(bit)]),
            want,
            "bit {bit}"
        );
    }
    assert_eq!(run(&mut e, 0x2851, sc.refr, &[s(b"Hand"), n(0)]), 0.0);
}

// ---- GetNifBlockParentNodes 0x2871 ----

/// The parents' names nearest first, the root's last; an empty array for the root; none for a
/// missing block or an empty name; the player's root by the argument and the camera.
#[test]
fn parent_nodes() {
    let mut e = setup();
    let sc = scene(&mut e);
    let id = run(&mut e, 0x2871, sc.refr, &[s(b"Head")]);
    assert_eq!(texts(&e, id), vec![t("Body"), t("Root")]);
    let id = run(&mut e, 0x2871, sc.refr, &[s(b"Root")]);
    assert_ne!(id, 0.0);
    assert!(texts(&e, id).is_empty(), "the root: an empty array");
    assert_eq!(
        run(&mut e, 0x2871, sc.refr, &[s(b"")]),
        0.0,
        "GetBlock: no name"
    );
    assert_eq!(run(&mut e, 0x2871, sc.refr, &[s(b"Leg")]), 0.0);
    // A parent without a name: an empty text.
    e.mem.set_u32(sc.body + 8, 0);
    let id = run(&mut e, 0x2871, sc.refr, &[s(b"Head")]);
    assert_eq!(texts(&e, id), vec![t(""), t("Root")]);
    // A broken parent chain ends the list.
    e.mem.set_u32(sc.body + 0x18, 0);
    let id = run(&mut e, 0x2871, sc.refr, &[s(b"Head")]);
    assert_eq!(texts(&e, id), vec![t("")]);
    // A parent chain that loops (never reaching the root) is bounded.
    e.mem.set_u32(sc.body + 0x18, sc.head);
    let id = run(&mut e, 0x2871, sc.refr, &[s(b"Head")]);
    assert_eq!(
        texts(&e, id).len(),
        super::super::nif_nodes::MAX_VISITS as usize
    );
    // The player: first person unless the argument is odd or the camera third person.
    let fp_head = shape(&mut e, b"Head");
    let fp = node(&mut e, b"FP", &[fp_head]);
    let third = node(&mut e, b"TP", &[sc.arm]);
    let p = player(&mut e, third, fp, false);
    let id = run(&mut e, 0x2871, p, &[s(b"Head")]);
    assert_eq!(texts(&e, id), vec![t("FP")]);
    assert_eq!(
        run(&mut e, 0x2871, p, &[s(b"Head"), n(1)]),
        0.0,
        "third person root"
    );
    let id = run(&mut e, 0x2871, p, &[s(b"Arm"), n(3)]);
    assert_eq!(texts(&e, id), vec![t("TP")]);
    e.mem.set_u8(p + 0x64a, 1);
    let id = run(&mut e, 0x2871, p, &[s(b"Arm")]);
    assert_eq!(texts(&e, id), vec![t("TP")]);
    set_root(&mut e, p, 0);
    e.mem.set_u8(p + 0x64a, 0);
    let id = run(&mut e, 0x2871, p, &[s(b"Head")]);
    assert_eq!(
        texts(&e, id),
        vec![t("FP")],
        "first person without loaded data"
    );
}

// ---- GetChildBlocks 0x28F3 ----

/// Every block below the node in walk order (the node's own name left out); with the third
/// argument only its children; none for a geometry block or a node without children.
#[test]
fn child_blocks() {
    let mut e = setup();
    let sc = scene(&mut e);
    let id = run(&mut e, 0x28f3, sc.refr, &[]);
    assert_eq!(texts(&e, id), vec![t("Body"), t("Head"), t("Arm")]);
    let id = run(&mut e, 0x28f3, sc.refr, &[s(b""), n(0), n(1)]);
    assert_eq!(texts(&e, id), vec![t("Body"), t("Arm")]);
    let id = run(&mut e, 0x28f3, sc.refr, &[s(b"Body")]);
    assert_eq!(texts(&e, id), vec![t("Head")]);
    assert_eq!(
        run(&mut e, 0x28f3, sc.refr, &[s(b"Head")]),
        0.0,
        "not a node"
    );
    let empty = node(&mut e, b"Empty", &[0]);
    set_children(&mut e, sc.arm, &[]);
    set_children(&mut e, sc.root, &[sc.body, sc.arm, empty]);
    assert_eq!(
        run(&mut e, 0x28f3, sc.refr, &[s(b"Empty")]),
        0.0,
        "none below"
    );
    assert_eq!(
        run(&mut e, 0x28f3, sc.refr, &[s(b"Empty"), n(0), n(1)]),
        0.0
    );
    // Empty slots are skipped; unnamed blocks give empty texts.
    let unnamed = object(&mut e, 0x0109_d454, None);
    set_children(&mut e, empty, &[0, unnamed]);
    let id = run(&mut e, 0x28f3, sc.refr, &[s(b"Empty")]);
    assert_eq!(texts(&e, id), vec![t("")]);
    // A cycle is bounded.
    set_children(&mut e, empty, &[sc.root]);
    let id = run(&mut e, 0x28f3, sc.refr, &[s(b"Empty")]);
    assert!(texts(&e, id).len() <= super::super::nif_nodes::MAX_VISITS as usize + 1);
    set_root(&mut e, sc.refr, 0);
    assert_eq!(run(&mut e, 0x28f3, sc.refr, &[]), 0.0, "no 3D");
}

// ---- GetCollisionNodes 0x28F2 ----

/// The loaded root's nodes (itself included) whose collision object has a world object, in walk
/// order; geometry blocks are not looked at.
#[test]
fn collision_nodes() {
    let mut e = setup();
    let sc = scene(&mut e);
    assert_eq!(run(&mut e, 0x28f2, sc.refr, &[]), 0.0, "none");
    let with_world = AREA + 0xe_3000;
    let without = AREA + 0xe_3100;
    e.mem.set_u32(with_world + 0x10, AREA + 0xe_3200);
    e.mem.set_u32(without + 0x10, 0);
    e.mem.set_u32(sc.body + COLLISION_OBJECT, with_world);
    e.mem.set_u32(sc.root + COLLISION_OBJECT, with_world);
    e.mem.set_u32(sc.head + COLLISION_OBJECT, with_world);
    let id = run(&mut e, 0x28f2, sc.refr, &[]);
    assert_eq!(texts(&e, id), vec![t("Root"), t("Body")]);
    e.mem.set_u32(sc.root + COLLISION_OBJECT, without);
    let id = run(&mut e, 0x28f2, sc.refr, &[]);
    assert_eq!(texts(&e, id), vec![t("Body")]);
    // The loaded root even for the player in first person.
    let fp = node(&mut e, b"FP", &[]);
    e.mem.set_u32(fp + COLLISION_OBJECT, with_world);
    let p = player(&mut e, sc.root, fp, false);
    let id = run(&mut e, 0x28f2, p, &[]);
    assert_eq!(texts(&e, id), vec![t("Body")]);
    set_root(&mut e, sc.refr, 0);
    assert_eq!(run(&mut e, 0x28f2, sc.refr, &[]), 0.0);
}

/// A name argument of 0x40 bytes or more runs past JIP's buffer: with the list guard row it is
/// read whole (and finds a block of that long name).
#[test]
fn long_name_with_guard() {
    let mut e = setup();
    let long = vec![b'L'; 0x41];
    let block = shape(&mut e, &long);
    let root = node(&mut e, b"Root", &[block]);
    let r = reference(&mut e, 0x1234, false, root);
    e.mem.set_f32(block + LOCAL_SCALE, 3.0);
    nvse::commands::array::set_fix(&mut e, nvse::commands::array::LIST_GUARD, true);
    assert_eq!(run(&mut e, 0x284f, r, &[s(&long)]), 3.0);
}

// ---- GetBlockTextureSet 0x28F4 ----

/// A game string at `at` holding `text` (none: no buffer).
fn game_string(e: &mut Engine, at: u32, text: Option<&[u8]>) {
    match text {
        None => e.mem.set_u32(at, 0),
        Some(t) => {
            let p = fixed(e, t);
            e.mem.set_u32(at, p);
            e.mem.set_u16(at + 4, t.len() as u16);
            e.mem.set_u16(at + 6, t.len() as u16 + 1);
        }
    }
}

/// A lit shader (type 8): its texture set's six paths, `data\textures\` (any case) taken off
/// the front; an unlit one (0x15): its source texture's path then five empty texts; others,
/// a missing set, a texture of another class or without a path: no array.
#[test]
fn block_texture_set() {
    let mut e = setup();
    let sc = scene(&mut e);
    let shader = AREA + 0xe_4000;
    let set = AREA + 0xe_4100;
    e.mem.set_u32(sc.head + 0xa8, shader);
    e.mem.set_u32(shader + 0x1c, 8);
    e.mem.set_u32(shader + 0xa4, set);
    let paths: [Option<&[u8]>; 6] = [
        Some(br"Data\Textures\a\b.dds"),
        Some(br"textures\c.dds"),
        None,
        Some(b""),
        Some(br"DATA\TEXTURES\"),
        Some(br"data\texturesX\d.dds"),
    ];
    for (i, p) in paths.into_iter().enumerate() {
        game_string(&mut e, set + 8 + 8 * i as u32, p);
    }
    let id = run(&mut e, 0x28f4, sc.refr, &[s(b"Head")]);
    assert_eq!(
        texts(&e, id),
        vec![
            t(r"a\b.dds"),
            t(r"textures\c.dds"),
            t(""),
            t(""),
            t(""),
            t(r"data\texturesX\d.dds")
        ]
    );
    e.mem.set_u32(shader + 0xa4, 0);
    assert_eq!(
        run(&mut e, 0x28f4, sc.refr, &[s(b"Head")]),
        0.0,
        "no texture set"
    );
    // Unlit: the source texture's path.
    let tex = AREA + 0xe_4200;
    e.mem.set_u32(shader + 0x1c, 0x15);
    e.mem.set_u32(shader + 0x60, tex);
    e.mem.set_u32(tex, 0x0109_b9ec);
    let path = fixed(&mut e, br"data\textures\sky.dds");
    e.mem.set_u32(tex + 0x30, path);
    let id = run(&mut e, 0x28f4, sc.refr, &[s(b"Head")]);
    assert_eq!(
        texts(&e, id),
        vec![t("sky.dds"), t(""), t(""), t(""), t(""), t("")]
    );
    e.mem.set_u32(tex + 0x30, 0);
    assert_eq!(run(&mut e, 0x28f4, sc.refr, &[s(b"Head")]), 0.0, "no path");
    e.mem.set_u32(tex + 0x30, path);
    e.mem.set_u32(tex, 0x0109_b9f0);
    assert_eq!(
        run(&mut e, 0x28f4, sc.refr, &[s(b"Head")]),
        0.0,
        "not a source texture"
    );
    e.mem.set_u32(tex, 0x0109_b9ec);
    e.mem.set_u32(shader + 0x60, 0);
    assert_eq!(
        run(&mut e, 0x28f4, sc.refr, &[s(b"Head")]),
        0.0,
        "no texture"
    );
    e.mem.set_u32(shader + 0x60, tex);
    // Another shader type, no shader, a node.
    e.mem.set_u32(shader + 0x1c, 9);
    assert_eq!(run(&mut e, 0x28f4, sc.refr, &[s(b"Head")]), 0.0);
    e.mem.set_u32(shader + 0x1c, 0x15);
    e.mem.set_u32(sc.head + 0xa8, 0);
    assert_eq!(run(&mut e, 0x28f4, sc.refr, &[s(b"Head")]), 0.0);
    e.mem.set_u32(sc.body + 0xa8, shader);
    assert_eq!(run(&mut e, 0x28f4, sc.refr, &[s(b"Body")]), 0.0, "a node");
    e.mem.set_u32(sc.head + 0xa8, shader);
    let id = run(&mut e, 0x28f4, sc.refr, &[s(b"Head")]);
    assert_eq!(texts(&e, id)[0], t("sky.dds"));
}
