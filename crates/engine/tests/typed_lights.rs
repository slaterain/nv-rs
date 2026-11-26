//! The differential test of every typed function of the lights family (`engine_typed::lights`)
//! against its translation. The world: a shadow scene light (its frustum planes and their mask, its
//! `NiLight` with a position, radius and RTTI, its shadow scene, its geometry list with a fence), the
//! scene node's lighting offset, and an object tree of nodes and geometries whose vtables answer
//! `IsNode`, `IsTriBasedGeom` and the RTTI as each object is set up, with bounds and flags.
//!
//! What stands in: the geometry and lighting property tests, the property's light add, the list's
//! add and move, `_atexit` and the CRT square root (which the typed tests leave out: native maths).

use engine as _;
use engine_core::prelude::*;
use engine_core::typed::diff::{Case, Diff, Input};
use engine_typed::lights::{self, walk};

#[path = "typed_lights/scene_node.rs"]
mod scene_node;

const VT_FN: u32 = 0x00ff_2000;
const VT_SLOTS: u32 = 0x200;
const LOG: u32 = 0x0300_0000;
const MAX_CALLS: usize = 20_000;

const ONE: u32 = 0x3f80_0000;
const TWO: u32 = 0x4000_0000;
const HALF: u32 = 0x3f00_0000;
const THIRD: u32 = 0x3eaa_aaab;
const MINUS_ONE: u32 = 0xbf80_0000;

/// The null bound, the scene node pointer, the radius-free RTTI, the add's static and its guard.
const NULL_BOUND: u32 = 0x011f_4288;
const SCENE_NODE: u32 = 0x011f_91c8;
const RADIUS_FREE_RTTI: u32 = 0x011f_4a68;
const OTHER_RTTI: u32 = 0x011f_4a00;
const ADD_GUARD: u32 = 0x011f_ffe4;

fn ret(eax: u32) -> Ret {
    Ret {
        eax,
        ..Ret::default()
    }
}

fn ret_f(st0: f64) -> Ret {
    Ret {
        st0,
        ..Ret::default()
    }
}

fn f64_arg(a: &[u32], i: usize) -> f64 {
    f64::from_bits(u64::from(a[i]) | u64::from(a[i + 1]) << 32)
}

fn log(e: &mut Engine, tag: u32, word: u32) {
    let n = e.mem.u32(LOG);
    if n >= 0x7c0 {
        return;
    }
    e.mem.set_u32(LOG + 4 + 8 * n, tag);
    e.mem.set_u32(LOG + 8 + 8 * n, word);
    e.mem.set_u32(LOG, n + 1);
}

fn words(e: &mut Engine, at: u32, w: &[u32]) {
    for (i, v) in (0u32..).zip(w) {
        e.mem.set_u32(at + 4 * i, *v);
    }
}

fn block(e: &mut Engine, size: u32, w: &[u32]) -> u32 {
    let a = e.mem.alloc(size.max(4 * w.len() as u32));
    words(e, a, w);
    a
}

fn bits(case: &Case, i: usize) -> u32 {
    case.f32(i).to_bits()
}

/// A stand-in that logs its words under `TAG` and answers the word at `LOG + 0x3f00 + 4 * (TAG &
/// 0x3f)`.
fn logged<const TAG: u32>(e: &mut Engine, a: &[u32]) -> Ret {
    log(e, TAG, a.first().copied().unwrap_or(0));
    for w in a.iter().skip(1) {
        log(e, TAG | 0x1_0000, *w);
    }
    ret(e.mem.u32(LOG + 0x3f00 + 4 * (TAG & 0x3f)))
}

fn set_logged_answer(e: &mut Engine, tag: u32, answer: u32) {
    e.mem.set_u32(LOG + 0x3f00 + 4 * (tag & 0x3f), answer);
}

/// The slot stand-ins: `IsNode` (+0xc) answers the vtable's word +VT_SLOTS, `IsTriBasedGeom` (+0x1c)
/// its word +VT_SLOTS+4, the RTTI (+8) its word +VT_SLOTS+8; each logs `this`.
fn slot_answer<const SLOT: u32, const K: u32>(e: &mut Engine, a: &[u32]) -> Ret {
    let this = a.first().copied().unwrap_or(0);
    log(e, 0xff00 | SLOT, this);
    let vt = e.mem.u32(this);
    ret(e.mem.u32(vt + VT_SLOTS + 4 * K))
}

/// An object of `size` bytes with its own vtable: `IsNode` answers `node`, `IsTriBasedGeom` `tri`,
/// the RTTI `rtti`.
fn object(e: &mut Engine, size: u32, node: u32, tri: u32, rtti: u32) -> u32 {
    let vt = e.mem.alloc(VT_SLOTS + 0x40);
    for slot in (0..VT_SLOTS).step_by(4) {
        e.mem.set_u32(vt + slot, VT_FN + slot);
    }
    let o = e.mem.alloc(size);
    e.mem.set_u32(o, vt);
    words(e, vt + VT_SLOTS, &[node, tri, rtti]);
    o
}

/// The exe's data and the stand-ins.
fn stand_ins(e: &mut Engine) {
    e.map(LOG, 0x4000);
    for page in [0x011f_4000, 0x011f_9000, 0x011f_f000] {
        e.map(page, 0x1000);
    }
    words(e, NULL_BOUND, &[0, 0, 0, 0]);
    // The scene node and its lighting offset.
    let node = block(e, 0x200, &[]);
    words(e, node + 0x1e4, &[HALF, MINUS_ONE, 0]);
    e.mem.set_u32(SCENE_NODE, node);
    e.register(0x00ec_6040, |_, a| ret_f(f64_arg(a, 0).sqrt()));
    e.register(0x00ec_658f, logged::<0x01>);
    e.register(0x00b4_f580, logged::<0x02>);
    e.register(0x004a_2020, logged::<0x03>);
    e.register(0x00b7_1560, logged::<0x04>);
    e.register(0x00c1_b4a0, logged::<0x05>);
    e.register(0x00b7_02e0, logged::<0x06>);
    e.register(VT_FN + 8, slot_answer::<8, 2>);
    e.register(VT_FN + 0xc, slot_answer::<0xc, 0>);
    e.register(VT_FN + 0x1c, slot_answer::<0x1c, 1>);
    set_logged_answer(e, 0x02, 1);
}

trait Common {
    fn common(self) -> Diff;
}

impl Common for Diff {
    fn common(self) -> Diff {
        self.family(&lights::LIGHTS)
            .family(&leaf::LEAVES)
            // The typed radius test takes its square root natively, not through the CRT's `_CIsqrt`.
            .ignore_calls(&[0x00ec_6040])
            .max_calls(MAX_CALLS)
            .float_sweep(64)
    }
}

/// An `NiLight` at the case's floats (position at `f`, `f + 1`, 2.0) with radius `radius` and the
/// RTTI `rtti`.
fn ni_light(e: &mut Engine, case: &Case, f: usize, radius: u32, rtti: u32) -> u32 {
    let l = object(e, 0x100, 0, 0, rtti);
    words(e, l + 0x8c, &[bits(case, f), bits(case, f + 1), TWO]);
    e.mem.set_u32(l + 0xe0, radius);
    l
}

/// A shadow scene light: its planes (plane `i`'s normal and distance from `planes`), their mask, its
/// `NiLight`, shadow scene and cast flag, and its geometry list (`items`, the fence at `fence` of
/// them or none).
fn light(
    e: &mut Engine,
    planes: &[[u32; 4]; 6],
    mask: u32,
    ni: u32,
    shadow_scene: u32,
    cast: u8,
) -> u32 {
    let l = block(e, 0x250, &[]);
    for (i, p) in (0u32..).zip(planes) {
        words(e, l + 0x144 + 0x10 * i, p);
    }
    e.mem.set_u32(l + 0x1a4, mask);
    e.mem.set_u32(l + 0xf8, ni);
    e.mem.set_u32(l + 0x128, shadow_scene);
    e.mem.set_u8(l + 0xec, cast);
    l
}

/// The light's list `lGeomList` holding `items` (geometries) in order; the fence at item `fence`
/// (none past the end).
fn geometry_list(e: &mut Engine, l: u32, items: &[u32], fence: usize) {
    let mut next = 0;
    let mut nodes = Vec::new();
    for &g in items.iter().rev() {
        next = block(e, 0xc, &[next, 0, g]);
        nodes.push(next);
    }
    nodes.reverse();
    e.mem.set_u32(l + 0xe0, next);
    e.mem
        .set_u32(l + 0x138, nodes.get(fence).copied().unwrap_or(0));
}

/// Six planes from the case's floats at `f`..: normals (`f`, `f + 1`, 1.0) etc., distances spread.
fn planes(case: &Case, f: usize) -> [[u32; 4]; 6] {
    [
        [bits(case, f), bits(case, f + 1), ONE, HALF],
        [MINUS_ONE, 0, 0, MINUS_ONE],
        [0, ONE, 0, TWO],
        [0, 0, MINUS_ONE, bits(case, f + 2)],
        [HALF, HALF, HALF, 0],
        [THIRD, MINUS_ONE, THIRD, 0xc040_0000],
    ]
}

const F3: [Input; 3] = [Input::F32, Input::F32Swept(ONE), Input::F32Swept(MINUS_ONE)];

fn inputs(parts: &[&[Input]]) -> Vec<Input> {
    parts.iter().flat_map(|p| p.iter().copied()).collect()
}

#[test]
fn lights_bound_within_frustum() {
    // The plane mask (none, all, some); the bound's radius (zero, small, large, NaN); floats for the
    // first plane and the bound's centre.
    Diff::new(entry!(
        0x00b9_bbb0,
        walk::bound_within_frustum(Ptr<_>, Ptr<_>) -> u8
    ))
    .common()
    .inputs(&inputs(&[
        &[
            Input::Of(&[0, 0x3f, 0x15, 0x2a]),
            Input::Of(&[0, HALF, 0x4120_0000, 0x7fc0_0000]),
        ],
        &F3,
        &F3,
    ]))
    .max_cases(256)
    .setup(|e, case| {
        stand_ins(e);
        let p = planes(case, 2);
        let l = light(e, &p, case.word(0), 0, 0, 1);
        let bound = block(
            e,
            0x10,
            &[bits(case, 5), bits(case, 6), bits(case, 7), case.word(1)],
        );
        vec![l, bound]
    })
    .run()
    .assert_same();
}

#[test]
fn lights_bound_within_radius() {
    // The RTTI (radius-free or not); the `NiLight` given (one other than the light's own, at another
    // place with another radius) or null (the light's own); the scale; the radius; the bound's radius
    // (a half, or 4.0, where the scale moves the edge by 2); floats for the light's position and the
    // bound.
    Diff::new(entry!(
        0x00b9_dae0,
        walk::bound_within_radius(Ptr<_>, Ptr<_>, u32, f32) -> u8
    ))
    .common()
    .inputs(&inputs(&[
        &[
            Input::Flag,
            Input::Flag,
            Input::Of(&[ONE, HALF]),
            Input::Of(&[0x4120_0000, ONE, 0x40a0_0000, 0x7fc0_0000]),
            Input::Of(&[HALF, 0x4080_0000]),
        ],
        &F3,
        &F3,
    ]))
    .max_cases(256)
    .setup(|e, case| {
        stand_ins(e);
        let rtti = if case.word(0) != 0 {
            RADIUS_FREE_RTTI
        } else {
            OTHER_RTTI
        };
        let own = ni_light(e, case, 5, 0x4040_0000, OTHER_RTTI);
        let ni = ni_light(e, case, 5, case.word(3), rtti);
        words(e, ni + 0x8c, &[bits(case, 6), MINUS_ONE, ONE]);
        let p = planes(case, 5);
        let given = if case.word(1) != 0 { ni } else { 0 };
        let l = light(e, &p, 0, if given != 0 { own } else { ni }, 0, 0);
        let bound = block(
            e,
            0x10,
            &[bits(case, 7), bits(case, 8), bits(case, 5), case.word(4)],
        );
        vec![l, bound, given, case.word(2)]
    })
    .run()
    .assert_same();
}

/// A tri-based geometry: its +0xc0 object (or none), its lighting property of `kind` (0 none) and
/// a bound (or none).
fn geometry(e: &mut Engine, with_object: bool, kind: u32, with_bound: bool) -> u32 {
    let g = object(e, 0xd0, 0, 0, OTHER_RTTI);
    e.mem.set_u32(g + 0x18, 0);
    if with_object {
        let o = block(e, 0x20, &[]);
        e.mem.set_u32(g + 0xc0, o);
    }
    if kind != 0 {
        let p = block(e, 0x40, &[]);
        e.mem.set_u32(p + 0x1c, kind);
        e.mem.set_u32(g + 0xa8, p);
    }
    if with_bound {
        let b = block(e, 0x10, &[ONE, TWO, HALF, ONE]);
        e.mem.set_u32(g + 0x20, b);
    }
    let vt = e.mem.u32(g);
    e.mem.set_u32(vt + VT_SLOTS + 4, g);
    g
}

#[test]
fn lights_add_tri_geom() {
    // The guard set or not; a +0xc0 object or none and the geometry test's answer; the property's
    // kind (none, 0 is none, 1, 4, 12, 13); where the geometry is in the list (absent, before the
    // fence, at it, after it) and the fence (none, item 1); the property test's answer; a bound.
    Diff::new(entry!(0x00b9_f480, walk::add_tri_geom(Ptr<_>, u32)))
        .common()
        .inputs(&[
            Input::Flag,
            Input::Of(&[0, 1, 2]),
            Input::Of(&[0, 1, 4, 12, 13]),
            Input::Of(&[0, 1, 2, 3]),
            Input::Of(&[1, 9]),
            Input::Flag,
            Input::Flag,
        ])
        .max_cases(256)
        .setup(|e, case| {
            stand_ins(e);
            e.mem.set_u32(ADD_GUARD, case.word(0));
            set_logged_answer(e, 0x02, u32::from(case.word(1) != 2));
            set_logged_answer(e, 0x03, case.word(5));
            let g = geometry(e, case.word(1) != 0, case.word(2), case.word(6) != 0);
            let others: Vec<u32> = (0..3).map(|_| geometry(e, true, 1, true)).collect();
            let mut items = others.clone();
            match case.word(3) {
                1 => items.insert(0, g),
                2 => items.insert(1, g),
                3 => items.push(g),
                _ => {}
            }
            let p = [[0; 4]; 6];
            let l = light(e, &p, 0, 0, 0, 0);
            geometry_list(e, l, &items, case.word(4) as usize);
            vec![l, g]
        })
        .run()
        .assert_same();
}

/// An object tree from `k`: a root node with five children: a geometry, a culled geometry (flag 1),
/// a null child, a node with two geometries (one without a bound, one with flag 0x800 and no bound), and a
/// geometry with a zero radius; the bounds' centres from the case's floats at `f`.
fn tree(e: &mut Engine, case: &Case, k: u32, f: usize) -> u32 {
    let bound =
        |e: &mut Engine, x: u32, r: u32| block(e, 0x10, &[x, bits(case, f), bits(case, f + 1), r]);
    let leaf = |e: &mut Engine, flags: u32, b: u32| {
        let g = geometry(e, true, 1 + (k % 2) * 3, false);
        e.mem.set_u32(g + 0x30, flags);
        e.mem.set_u32(g + 0x20, b);
        g
    };
    let node = |e: &mut Engine, children: &[u32], flags: u32, b: u32| {
        let n = object(e, 0xb0, 0, 0, OTHER_RTTI);
        let vt = e.mem.u32(n);
        e.mem.set_u32(vt + VT_SLOTS, n);
        let base = block(e, 4 * children.len() as u32, children);
        e.mem.set_u32(n + 0xa0, base);
        e.mem.set_u16(n + 0xa6, children.len() as u16);
        e.mem.set_u32(n + 0x30, flags);
        e.mem.set_u32(n + 0x20, b);
        n
    };
    let b1 = bound(e, ONE, HALF);
    let g1 = leaf(e, 0, b1);
    let b2 = bound(e, TWO, HALF);
    let g2 = leaf(e, 1, b2);
    let g3 = leaf(e, 0, 0);
    // Flag 0x800 with no bound: the walks take it without a bound test.
    let g4 = leaf(e, 0x800, 0);
    let b5 = bound(e, HALF, ONE);
    let inner = node(e, &[g3, g4], (k & 4) << 9, b5);
    let b6 = bound(e, 0x4120_0000, 0);
    let g6 = leaf(e, 0, b6);
    let b7 = bound(e, 0, 0x4120_0000);
    node(e, &[g1, g2, 0, inner, g6], 0, b7)
}

/// The world of a walk case from `w` = [the tree's pattern, the light casts shadows, the shadow
/// scene is the tree, the tree's flags (0, 1, 0x800000), the plane mask, the `NiLight`'s radius].
fn walk_world(e: &mut Engine, case: &Case, w: [u32; 6], f: usize) -> (u32, u32) {
    let [k, cast, own, flags, mask, radius] = w;
    stand_ins(e);
    let root = tree(e, case, k, f);
    e.mem.set_u32(root + 0x30, flags);
    let ni = ni_light(e, case, f, radius, OTHER_RTTI);
    let p = planes(case, f);
    let shadow_scene = if own != 0 { root } else { 0 };
    let l = light(e, &p, mask, ni, shadow_scene, cast as u8);
    let others: Vec<u32> = (0..2).map(|_| geometry(e, true, 1, true)).collect();
    geometry_list(e, l, &others, 1);
    (l, root)
}

fn walk_inputs() -> Vec<Input> {
    inputs(&[
        &[
            Input::Of(&[0, 1, 4, 5]),
            Input::Flag,
            Input::Flag,
            Input::Of(&[0, 1, 0x0080_0000]),
            Input::Of(&[0, 0x3f, 0x09]),
            Input::Of(&[0x4120_0000, ONE, 0x4248_0000]),
        ],
        &F3,
    ])
}

#[test]
fn lights_add_object() {
    Diff::new(entry!(0x00ba_0110, walk::add_object(Ptr<_>, u32)))
        .common()
        .inputs(&walk_inputs())
        .max_cases(256)
        .setup(|e, case| {
            let w = core::array::from_fn(|i| case.word(i));
            let (l, root) = walk_world(e, case, w, 6);
            vec![l, root]
        })
        .run()
        .assert_same();
}

#[test]
fn lights_parse_frustum() {
    Diff::new(entry!(0x00b9_f5b0, walk::parse_frustum(Ptr<_>, u32)))
        .common()
        .inputs(&walk_inputs())
        .max_cases(256)
        .setup(|e, case| {
            let w = core::array::from_fn(|i| case.word(i));
            let (l, root) = walk_world(e, case, w, 6);
            vec![l, root]
        })
        .run()
        .assert_same();
}

#[test]
fn lights_parse_radius() {
    Diff::new(entry!(0x00b9_f6a0, walk::parse_radius(Ptr<_>, u32)))
        .common()
        .inputs(&walk_inputs())
        .max_cases(256)
        .setup(|e, case| {
            let w = core::array::from_fn(|i| case.word(i));
            let (l, root) = walk_world(e, case, w, 6);
            vec![l, root]
        })
        .run()
        .assert_same();
}

/// The exe's constant bodies the walks answer in place: the object itself (nodes' `IsNode`, the
/// geometries' `IsTriBasedGeom`), 0, and the point, directional and ambient lights' `GetRTTI`.
const EXE_BODIES: [u32; 6] = [
    0x0068_15c0,
    0x00e6_8810,
    0x00ac_bb70,
    0x00a7_d790,
    0x00a7_5c70,
    0x00a9_b390,
];

/// The tree from `object` down with the exe's bodies in its objects' slots where their stand-ins'
/// answers are what those bodies answer: `IsNode` the object itself (`006815c0`) or 0, and
/// `IsTriBasedGeom` the object itself (`00e68810`) or 0 (`00acbb70`); any other answer keeps its
/// stand-in.
fn exe_bodies(e: &mut Engine, object: u32) {
    if object == 0 {
        return;
    }
    let vt = e.mem.u32(object);
    for (slot, k, itself) in [(0xc, 0, 0x0068_15c0), (0x1c, 1, 0x00e6_8810)] {
        match e.mem.u32(vt + VT_SLOTS + 4 * k) {
            0 => e.mem.set_u32(vt + slot, 0x00ac_bb70),
            a if a == object => e.mem.set_u32(vt + slot, itself),
            _ => {}
        }
    }
    if e.mem.u32(vt + VT_SLOTS) == object {
        let base = e.mem.u32(object + 0xa0);
        for i in 0..u32::from(e.mem.u16(object + 0xa6)) {
            let child = e.mem.u32(base + 4 * i);
            exe_bodies(e, child);
        }
    }
}

/// The walk world with the exe's bodies in the tree's slots, and the light's `NiLight` answering its
/// RTTI with the point light's (`k` of 0 or 1), the directional light's (4) or the ambient light's
/// (5) `GetRTTI`.
fn exe_body_world(e: &mut Engine, case: &Case) -> Vec<u32> {
    let w: [u32; 6] = core::array::from_fn(|i| case.word(i));
    let (l, root) = walk_world(e, case, w, 6);
    exe_bodies(e, root);
    let ni = e.mem.u32(l + 0xf8);
    let rtti = match w[0] {
        4 => 0x00a7_5c70,
        5 => 0x00a9_b390,
        _ => 0x00a7_d790,
    };
    let vt = e.mem.u32(ni);
    e.mem.set_u32(vt + 8, rtti);
    vec![l, root]
}

#[test]
fn lights_add_object_exe_bodies() {
    Diff::new(entry!(0x00ba_0110, walk::add_object(Ptr<_>, u32)))
        .common()
        .ignore_calls(&EXE_BODIES)
        .inputs(&walk_inputs())
        .max_cases(256)
        .setup(exe_body_world)
        .run()
        .assert_same();
}

#[test]
fn lights_parse_frustum_exe_bodies() {
    Diff::new(entry!(0x00b9_f5b0, walk::parse_frustum(Ptr<_>, u32)))
        .common()
        .ignore_calls(&EXE_BODIES)
        .inputs(&walk_inputs())
        .max_cases(256)
        .setup(exe_body_world)
        .run()
        .assert_same();
}

#[test]
fn lights_parse_radius_exe_bodies() {
    Diff::new(entry!(0x00b9_f6a0, walk::parse_radius(Ptr<_>, u32)))
        .common()
        .ignore_calls(&EXE_BODIES)
        .inputs(&walk_inputs())
        .max_cases(256)
        .setup(exe_body_world)
        .run()
        .assert_same();
}

#[test]
fn lights_bound_rounding() {
    // Fixed inputs where the distances computed at the main thread's precision, each step stored as
    // a float, and computed in double precision part at the test's boundary, so the tests computed
    // in the wrong precision fail. The radius: the light at (0x40e7f741, 0xbf658507, 0x40b24c7a),
    // the bound at (0xc000ddde, 0x4045b340, 0x3f7efb10) with radius 0x3f2d8a3c, the offset (0.5, -1,
    // 0): the gap is exactly the light's radius 0x413335fe as floats (out of range), below it in
    // double precision.
    Diff::new(entry!(
        0x00b9_dae0,
        walk::bound_within_radius(Ptr<_>, Ptr<_>, u32, f32) -> u8
    ))
    .common()
    .float_sweep(0)
    .inputs(&[Input::Of(&[0])])
    .setup(|e, _| {
        stand_ins(e);
        let ni = object(e, 0x100, 0, 0, OTHER_RTTI);
        words(e, ni + 0x8c, &[0x40e7_f741, 0xbf65_8507, 0x40b2_4c7a]);
        e.mem.set_u32(ni + 0xe0, 0x4133_35fe);
        let p = [[0; 4]; 6];
        let l = light(e, &p, 0, ni, 0, 0);
        let bound = block(
            e,
            0x10,
            &[0xc000_ddde, 0x4045_b340, 0x3f7e_fb10, 0x3f2d_8a3c],
        );
        vec![l, bound, 0, ONE]
    })
    .run()
    .assert_same();
    // The frustum: plane 0 (0x3e85bb48, 0x3f5bc6c8, 0xbe607317, d 0x3f8795ec), the bound at
    // (0x40de380f, 0x408659b8, 0xc082c0fd) with radius 0x40a8255b: the distance is exactly the
    // radius as floats (the plane is dropped from the mask), below it in double precision.
    Diff::new(entry!(
        0x00b9_bbb0,
        walk::bound_within_frustum(Ptr<_>, Ptr<_>) -> u8
    ))
    .common()
    .float_sweep(0)
    .inputs(&[Input::Of(&[0])])
    .setup(|e, _| {
        stand_ins(e);
        let mut p = [[0; 4]; 6];
        p[0] = [0x3e85_bb48, 0x3f5b_c6c8, 0xbe60_7317, 0x3f87_95ec];
        let l = light(e, &p, 1, 0, 0, 1);
        let bound = block(
            e,
            0x10,
            &[0x40de_380f, 0x4086_59b8, 0xc082_c0fd, 0x40a8_255b],
        );
        vec![l, bound]
    })
    .run()
    .assert_same();
}

#[test]
fn lights_bound_radius_scale() {
    // The light 3.0 from the bound's centre (its position (2.5, 1, 0) moved by the offset (0.5, -1,
    // 0)), the bound's radius 4.0 scaled by 0.5 (2.0): the gap 1.0 is past the light's radius 0.5,
    // while the unscaled radius would bring the bound in.
    Diff::new(entry!(
        0x00b9_dae0,
        walk::bound_within_radius(Ptr<_>, Ptr<_>, u32, f32) -> u8
    ))
    .common()
    .float_sweep(0)
    .inputs(&[Input::Of(&[HALF, ONE])])
    .setup(|e, case| {
        stand_ins(e);
        let ni = object(e, 0x100, 0, 0, OTHER_RTTI);
        words(e, ni + 0x8c, &[0x4020_0000, ONE, 0]);
        e.mem.set_u32(ni + 0xe0, HALF);
        let p = [[0; 4]; 6];
        let l = light(e, &p, 0, ni, 0, 0);
        let bound = block(e, 0x10, &[0, 0, 0, 0x4080_0000]);
        vec![l, bound, 0, case.word(0)]
    })
    .run()
    .assert_same();
}

#[test]
fn lights_add_object_target() {
    // What `AddObject` is handed: the tree's root, or one of its geometries (the direct add at the
    // end) with a lighting property that passes (type 1) or not (type 13); the root's bound: its own,
    // none, a zero radius, a NaN radius (which still parses: `FLDZ; FCOMP`); the light casts shadows
    // or not.
    Diff::new(entry!(0x00ba_0110, walk::add_object(Ptr<_>, u32)))
        .common()
        .inputs(&inputs(&[
            &[Input::Of(&[0, 1, 2]), Input::Of(&[0, 1, 2, 3]), Input::Flag],
            &F3,
        ]))
        .max_cases(256)
        .setup(|e, case| {
            let (l, root) = walk_world(e, case, [1, case.word(2), 0, 0, 0x3f, 0x4248_0000], 3);
            match case.word(1) {
                1 => e.mem.set_u32(root + 0x20, 0),
                2 => {
                    let b = block(e, 0x10, &[0, 0, 0, 0]);
                    e.mem.set_u32(root + 0x20, b);
                }
                3 => {
                    let b = block(e, 0x10, &[0, 0, 0, 0x7fc0_0000]);
                    e.mem.set_u32(root + 0x20, b);
                }
                _ => {}
            }
            let object = match case.word(0) {
                0 => root,
                k => {
                    let g = geometry(e, true, if k == 1 { 1 } else { 13 }, true);
                    e.mem.set_u32(g + 0x30, 0);
                    g
                }
            };
            vec![l, object]
        })
        .run()
        .assert_same();
}

#[test]
fn lights_frustum_mask_inherited() {
    // Planes 0 (x > 0) and 1 (y > 0) active. The root at (5, 0, 0), radius 1, is wholly inside plane
    // 0, which its walk drops; its first child at (-5, 5, 0) is outside plane 0 but inside plane 1
    // (the exe adds it, with plane 0 dropped, and drops plane 1 for its own walk); its second child
    // at (0, -5, 0) is outside plane 1, which only the first child was wholly inside: with the mask
    // restored after the first child the exe leaves it out.
    Diff::new(entry!(0x00b9_f5b0, walk::parse_frustum(Ptr<_>, u32)))
        .common()
        .float_sweep(0)
        .inputs(&[Input::Of(&[0])])
        .setup(|e, _| {
            stand_ins(e);
            let five = 0x40a0_0000;
            let minus_five = 0xc0a0_0000;
            let leaf = |e: &mut Engine, x: u32, y: u32| {
                let g = geometry(e, true, 1, false);
                let b = block(e, 0x10, &[x, y, 0, ONE]);
                e.mem.set_u32(g + 0x20, b);
                g
            };
            let a = leaf(e, minus_five, five);
            let b = leaf(e, 0, minus_five);
            let root = object(e, 0xb0, 0, 0, OTHER_RTTI);
            let vt = e.mem.u32(root);
            e.mem.set_u32(vt + VT_SLOTS, root);
            let children = block(e, 8, &[a, b]);
            e.mem.set_u32(root + 0xa0, children);
            e.mem.set_u16(root + 0xa6, 2);
            let rb = block(e, 0x10, &[five, 0, 0, ONE]);
            e.mem.set_u32(root + 0x20, rb);
            let mut p = [[0; 4]; 6];
            p[0] = [ONE, 0, 0, 0];
            p[1] = [0, ONE, 0, 0];
            let l = light(e, &p, 3, 0, 0, 1);
            vec![l, root]
        })
        .run()
        .assert_same();
}

/// The light lists' slow loops made linear (`ResortLights` `00b70390`, `ComputeWeightedShadowLight`
/// `00b9d150`, the dominant light direction `00b5bca0`), against their translations, under both
/// control words. The world: the shadow scene node at `011f91c8` with its lighting offset, its sun
/// light and its light list (real `NiTListItem`s, in several shapes: items allocated out of order,
/// a count below the items' number, a null light inside), shadow scene lights with their `NiLight`s
/// (position, radius, dimmer, colour, flags), a lighting property's light list, a bound, and the
/// exe's constants.
///
/// What stands in: the extra data look-up, the point normalisation (halves the point, and logs it),
/// the object update, the property's active non-shadow light walk (answers the property's lights in
/// order through the iterator word) and `_CIsqrt`; `GetLuminanceToBound` and the node's light at an
/// index are left out of the call compare (the typed code does their work in place).
mod lists {
    use super::{bits, f64_arg, log, ret, ret_f, words, HALF, LOG, MINUS_ONE, ONE, TWO};
    use engine_core::prelude::*;
    use engine_core::typed::diff::{Case, Diff, Input};
    use engine_core::typed::{MAIN_THREAD, OTHER_THREADS};
    use engine_typed::lights::{self, dominant, resort, weighted};

    const TEN: u32 = 0x4120_0000;
    const QUARTER: u32 = 0x3e80_0000;
    const NAN: u32 = 0x7fc0_0000;
    const BOTH: [u16; 2] = [OTHER_THREADS, MAIN_THREAD];

    const SCENE_NODE: u32 = 0x011f_91c8;
    const EXTRA_DATA_NAME: u32 = 0x0120_31e4;

    fn logged<const TAG: u32>(e: &mut Engine, a: &[u32]) -> Ret {
        log(e, TAG, a.first().copied().unwrap_or(0));
        for w in a.iter().skip(1) {
            log(e, TAG | 0x1_0000, *w);
        }
        ret(e.mem.u32(LOG + 0x3f00 + 4 * (TAG & 0x3f)))
    }

    fn set_logged(e: &mut Engine, tag: u32, value: u32) {
        e.mem.set_u32(LOG + 0x3f00 + 4 * (tag & 0x3f), value);
    }

    /// The normalisation stand-in: halves the point's three floats.
    fn halve(e: &mut Engine, a: &[u32]) -> Ret {
        log(e, 0x42, a[0]);
        for i in 0..3 {
            let v = e.mem.f32(a[0] + 4 * i) * 0.5;
            e.mem.set_f32(a[0] + 4 * i, v);
        }
        ret(0)
    }

    /// The exe's data pages and constants, the scene node with its lighting offset (`offset`).
    fn world(e: &mut Engine, offset: [u32; 3]) -> u32 {
        e.map(LOG, 0x4000);
        for page in [
            0x0101_2000,
            0x0101_3000,
            0x0101_6000,
            0x0101_7000,
            0x0101_d000,
            0x0102_1000,
            0x0102_2000,
            0x0102_3000,
            0x0102_4000,
            0x0102_9000,
            0x0103_0000,
            0x0103_2000,
            0x0107_3000,
            0x010b_7000,
            0x011f_4000,
            0x011f_9000,
            0x011f_f000,
            0x0120_3000,
        ] {
            e.map(page, 0x1000);
        }
        for (at, value) in [
            (0x0101_2060u32, 0.0f64),
            (0x0101_2070, 1.0),
            (0x010b_7e98, 0.0625),
            (0x0107_31a8, 75.0),
            (0x0101_7b70, 1000.0),
            (0x0101_de30, 0.75),
            (0x0102_31d8, 256.0),
            (0x010b_7e90, f64::from(1.4f32)),
            (0x0102_40b0, 8.0),
            (0x0102_90c0, f64::from(0.4f32)),
            (0x0102_17d8, f64::from(0.8f32)),
            (0x0102_1928, 3.0),
            (0x0102_28a0, 0.4),
        ] {
            e.mem.set_u64(at, value.to_bits());
        }
        e.mem.set_u32(0x0101_3ea4, 0x3c23_d70a);
        e.mem.set_u32(0x0101_6264, 0x3f40_0000);
        e.mem.set_u32(0x0103_0ff0, 0x3f4c_cccd);
        e.mem.set_u32(0x0103_2adc, 0x4380_0000);
        e.mem.set_u32(EXTRA_DATA_NAME, 0x99);
        words(e, 0x011f_4998, &[HALF, HALF, HALF]);
        words(e, 0x011f_426c, &[0, HALF, 0]);
        let node = e.mem.alloc(0x200);
        words(e, node + 0x1e4, &offset);
        e.mem.set_u32(SCENE_NODE, node);
        e.register(0x00ec_6040, |_, a| ret_f(f64_arg(a, 0).sqrt()));
        e.register(0x004a_0c10, halve);
        node
    }

    /// An `NiLight` at `position` with radius `radius`, dimmer `dimmer`, culled or not, its colour.
    fn ni_light(e: &mut Engine, position: [u32; 3], radius: u32, dimmer: u32, culled: bool) -> u32 {
        let l = e.mem.alloc(0x100);
        words(e, l + 0x8c, &position);
        e.mem.set_u32(l + 0xe0, radius);
        e.mem.set_u32(l + 0xc4, dimmer);
        e.mem.set_u32(l + 0x30, u32::from(culled));
        words(e, l + 0xd4, &[ONE, HALF, QUARTER]);
        l
    }

    /// A shadow scene light with its `NiLight`, casting shadows or not.
    fn shadow_light(e: &mut Engine, ni: u32, cast: bool) -> u32 {
        let l = e.mem.alloc(0x250);
        e.mem.set_u32(l + 0xf8, ni);
        e.mem.set_u8(l + 0xec, u8::from(cast));
        l
    }

    /// A list at `list` (head, tail, count) of `elements` as `NiTListItem`s, in the shape `shape`:
    /// 0 plain, 1 its items allocated last first (the list's order is not the memory's, as after
    /// adds and removals mid-list), 2 its count one below the items' number, 3 the third element
    /// null.
    fn list(e: &mut Engine, list: u32, elements: &[u32], shape: u32) {
        let mut items: Vec<u32> = elements.iter().map(|_| e.mem.alloc(0xc)).collect();
        if shape == 1 {
            items.reverse();
        }
        for (i, (&item, &element)) in items.iter().zip(elements).enumerate() {
            let next = items.get(i + 1).copied().unwrap_or(0);
            let previous = if i == 0 { 0 } else { items[i - 1] };
            let element = if shape == 3 && i == 2 { 0 } else { element };
            words(e, item, &[next, previous, element]);
        }
        let count = items.len() as u32 - u32::from(shape == 2 && !items.is_empty());
        words(
            e,
            list,
            &[
                items.first().copied().unwrap_or(0),
                items.last().copied().unwrap_or(0),
                count,
            ],
        );
    }

    /// The positions the lights take in turn.
    const POSITIONS: [[u32; 3]; 7] = [
        [ONE, TWO, HALF],
        [TEN, 0, MINUS_ONE],
        [0, 0, 0],
        [MINUS_ONE, TEN, TWO],
        [HALF, HALF, HALF],
        [0x41a0_0000, ONE, 0],
        [TWO, MINUS_ONE, TEN],
    ];
    /// The radii the lights take in turn (a zero and a NaN among them).
    const RADII: [u32; 5] = [TEN, TWO, 0x4248_0000, 0, NAN];

    #[test]
    fn lights_resort_lights() {
        // The number of lights; the pattern of positions and radii (equal luminances included,
        // which must not move); the list's shape (no null light: the exe reads through it); floats
        // for the bound's centre and the lighting offset.
        Diff::new(entry!(0x00b7_0390, resort::resort_lights(Ptr<_>, u32)))
            .family(&lights::LIGHTS)
            .family(&leaf::LEAVES)
            .ignore_calls(&[0x00b9_dbe0, 0x00ec_6040])
            .control_words(&BOTH)
            // The luminance is `GetLuminanceToBound`'s native body: x87 and SSE may give a NaN
            // luminance another sign or payload, which the sort's comparisons treat alike.
            .allow_nan_payloads()
            .inputs(&[
                Input::Of(&[0, 1, 2, 5, 9]),
                Input::Of(&[0, 1, 2, 3, 4, 5, 6]),
                Input::Of(&[0, 1]),
                Input::F32,
                Input::F32Swept(ONE),
                Input::F32Swept(HALF),
            ])
            .max_cases(400)
            .float_sweep(64)
            .setup(|e, case: &Case| {
                world(e, [bits(case, 5), 0, HALF]);
                let k = case.word(1) as usize;
                let lights: Vec<u32> = (0..case.word(0) as usize)
                    .map(|i| {
                        // Lights in threes share a place and a radius, so ties come up.
                        let p = POSITIONS[(i / 3 * 2 + k) % 7];
                        let ni = ni_light(e, p, RADII[(i / 3 + k) % 5], ONE, false);
                        shadow_light(e, ni, false)
                    })
                    .collect();
                let property = e.mem.alloc(0x80);
                e.mem.set_u32(property + 0x38, 7);
                list(e, property + 0x60, &lights, case.word(2));
                let bound = e.mem.alloc(0x10);
                words(e, bound, &[bits(case, 3), bits(case, 4), ONE, TWO]);
                vec![property, bound]
            })
            .run()
            .assert_same();
    }

    /// The property's light walk stand-ins: the property's +0x60 holds the address of a zero-ended
    /// array of lights, and the iterator word the index of the next one.
    fn first_property_light(e: &mut Engine, a: &[u32]) -> Ret {
        log(e, 0x40, a[0]);
        e.mem.set_u32(a[1], 1);
        ret(e.mem.u32(e.mem.u32(a[0] + 0x60)))
    }

    fn next_property_light(e: &mut Engine, a: &[u32]) -> Ret {
        log(e, 0x41, a[0]);
        let i = e.mem.u32(a[1]);
        e.mem.set_u32(a[1], i + 1);
        ret(e.mem.u32(e.mem.u32(a[0] + 0x60) + 4 * i))
    }

    /// The lights of a case: `n` shadow scene lights in pattern `k` (casting shadows, culled,
    /// dimmed out, far, near).
    fn pattern_lights(e: &mut Engine, n: usize, k: usize) -> Vec<u32> {
        (0..n)
            .map(|i| {
                let pattern = (i + k) % 7;
                let p = POSITIONS[(i * 3 + k) % 7];
                let dimmer = if pattern == 3 { 0 } else { ONE };
                let ni = ni_light(e, p, RADII[(i + k) % 4], dimmer, pattern == 2);
                shadow_light(e, ni, pattern == 1)
            })
            .collect()
    }

    #[test]
    fn lights_compute_weighted() {
        // With a property or the scene node's list; the number of lights (past the 40 counted
        // ones too) and their pattern; the node list's shape; the extra data (none, or one with an
        // offset and extents); floats for the sun's z and the shadow scene's position and scale.
        Diff::new(entry!(
            0x00b9_d150,
            weighted::compute_weighted(Ptr<_>, u32, u32, u32, u32)
        ))
        .family(&lights::LIGHTS)
        .family(&leaf::LEAVES)
        .ignore_calls(&[0x00b5_ac20, 0x00ec_6040])
        .goes_by_address(&[0x00b7_0600, 0x00b7_0700, 0x004a_0c10, 0x00a5_9c60])
        .control_words(&BOTH)
        .inputs(&[
            Input::Flag,
            Input::Of(&[3, 0, 1, 2, 8, 45]),
            Input::Of(&[0, 1, 2, 3, 4, 5, 6]),
            Input::Of(&[0, 1, 2, 3]),
            Input::Flag,
            Input::F32,
            Input::F32Swept(ONE),
        ])
        .max_cases(400)
        .float_sweep(64)
        .max_calls(50_000)
        .setup(|e, case: &Case| {
            let node = world(e, [HALF, 0, MINUS_ONE]);
            e.register(0x00a5_bdd0, logged::<0x01>);
            e.register(0x00a5_9c60, logged::<0x02>);
            e.register(0x00b7_0600, first_property_light);
            e.register(0x00b7_0700, next_property_light);
            let lights = pattern_lights(e, case.word(1) as usize, case.word(2) as usize);
            list(e, node + 0xb4, &lights, case.word(3));
            // The sun: a shadow scene light whose `NiLight` has a direction at +0xf0.
            let sun_ni = ni_light(e, [0, 0, 0], ONE, ONE, false);
            words(e, sun_ni + 0xf0, &[HALF, MINUS_ONE, bits(case, 5)]);
            let sun = shadow_light(e, sun_ni, false);
            e.mem.set_u32(node + 0xe0, sun);
            // The property (or none) with its zero-ended array of the same lights.
            let property = if case.word(0) == 1 {
                let p = e.mem.alloc(0x80);
                let array = e.mem.alloc(4 * (lights.len() as u32 + 1));
                words(e, array, &lights);
                e.mem.set_u32(p + 0x60, array);
                p
            } else {
                0
            };
            // The shadow scene with its position and scale, the extra data or none.
            let scene = e.mem.alloc(0x100);
            words(e, scene + 0x8c, &[ONE, bits(case, 6), TWO]);
            e.mem.set_u32(scene + 0x98, bits(case, 6));
            if case.word(4) == 1 {
                let extra = e.mem.alloc(0x30);
                words(e, extra + 0xc, &[HALF, 0, ONE]);
                words(e, extra + 0x18, &[TWO, TEN, bits(case, 5)]);
                set_logged(e, 0x01, extra);
            }
            let this_ni = ni_light(e, [0, 0, 0], ONE, ONE, false);
            let this = shadow_light(e, this_ni, true);
            e.mem.set_u32(this + 0x128, scene);
            vec![this, property, 1, 2, 3]
        })
        .run()
        .assert_same();
    }

    #[test]
    fn lights_dominant_light_direction() {
        // The sun bright (its direction answers) or dim (the lights are weighed); the number of
        // lights (past the 100 weighed ones too) and their pattern; the list's shape; floats for
        // the point and the direction.
        Diff::new(entry!(
            0x00b5_bca0,
            dominant::dominant_light_direction(Ptr<_>, Ptr<_>, Ptr<_>, Ptr<_>) -> u32
        ))
        .family(&lights::LIGHTS)
        .family(&leaf::LEAVES)
        .ignore_calls(&[0x00b5_ac20, 0x00ec_6040])
        .goes_by_address(&[0x004a_0c10])
        .control_words(&BOTH)
        .inputs(&[
            Input::Flag,
            Input::Of(&[3, 0, 1, 2, 9, 104]),
            Input::Of(&[0, 1, 2, 3, 4, 5, 6]),
            Input::Of(&[0, 1, 2, 3]),
            Input::F32,
            Input::F32Swept(MINUS_ONE),
        ])
        .max_cases(400)
        .float_sweep(64)
        .max_calls(50_000)
        .setup(|e, case: &Case| {
            let node = world(e, [HALF, 0, MINUS_ONE]);
            let lights = pattern_lights(e, case.word(1) as usize, case.word(2) as usize);
            list(e, node + 0xb4, &lights, case.word(3));
            // The sun: its luminance (dimmer times (1 + 0.5 + 0.25) / 3) above 0.4 or below it.
            let dimmer = if case.word(0) == 1 { ONE } else { QUARTER };
            let sun_ni = ni_light(e, [0, 0, 0], ONE, dimmer, false);
            words(e, sun_ni + 0xf0, &[HALF, MINUS_ONE, bits(case, 5)]);
            let sun = shadow_light(e, sun_ni, false);
            e.mem.set_u32(node + 0xe0, sun);
            let point = e.mem.alloc(0x10);
            words(e, point, &[bits(case, 4), ONE, HALF]);
            let direction = e.mem.alloc(0x10);
            words(e, direction, &[HALF, bits(case, 5), MINUS_ONE]);
            let out = e.mem.alloc(0x10);
            vec![node, point, direction, out]
        })
        .run()
        .assert_same();
    }
}

/// The scene node's queue drain and the object half under it (`00b5fd60`, `00b5dac0`, `00b5c490`,
/// `00b5b260`, `00b5c5c0`, `00b5b410`), against their translations. The world: the walk tests' object
/// trees and geometries (their properties, bounds, flags and vtables), shadow scene lights with their
/// `NiLight`s (some radius-free, some view-culled), the node's light list and queues, a parent of each
/// kind (a room, the other kind, none) with its light array.
///
/// What stands in, beyond the walk tests' doubles: the locks, the light half's callees, the removal,
/// the renderer's map release, the portal graph's update, the lists' remove-all, the reference counts
/// (they count), the property's fence and fenced removal, the accumulator, the multibound cast and
/// its child, and the scrap heap (a fresh block per allocation). The CRT `qsort`, the comparator and
/// `GetLuminanceToBound` are the real translations.
mod queue {
    use super::{
        bits, block, geometry, light, logged, ni_light, object, set_logged_answer, stand_ins, tree,
        words, HALF, LOG, ONE, OTHER_RTTI, RADIUS_FREE_RTTI, TWO, VT_FN, VT_SLOTS,
    };
    use engine_core::prelude::*;
    use engine_core::typed::diff::{memory_differences_beside_stack, Case, Diff, Input, Value};
    use engine_typed::lights::{self, queue};

    const ROOM_RTTI: u32 = 0x0120_2828;
    const OTHERS_RTTI: u32 = 0x0120_30b8;

    fn count_up(e: &mut Engine, a: &[u32]) -> Ret {
        let n = e.mem.u32(a[0]).wrapping_add(1);
        e.mem.set_u32(a[0], n);
        super::ret(n)
    }

    fn count_down(e: &mut Engine, a: &[u32]) -> Ret {
        let n = e.mem.u32(a[0]).wrapping_sub(1);
        e.mem.set_u32(a[0], n);
        super::ret(n)
    }

    fn scrap_block(e: &mut Engine, a: &[u32]) -> Ret {
        let b = e.mem.alloc(a[1].max(4));
        super::ret(b)
    }

    /// The doubles of the drain's callees, on top of the walk tests' ones.
    fn queue_stand_ins(e: &mut Engine) {
        stand_ins(e);
        e.map(0x0102_3000, 0x1000);
        e.map(0x010a_2000, 0x1000);
        e.map(0x011c_5000, 0x1000);
        e.map(0x011f_6000, 0x1000);
        e.mem.set_f32(0x0102_31d0, 150.0);
        e.mem.set_u32(0x010a_2720, 4);
        for (address, f) in [
            (0x0040_fbf0u32, logged::<0x07> as AbiFn),
            (0x00b5_cd00, logged::<0x08>),
            (0x00b5_eca0, logged::<0x09>),
            (0x00b5_d930, logged::<0x0a>),
            (0x00b5_d300, logged::<0x0b>),
            (0x00b5_ef90, logged::<0x0c>),
            (0x00b6_d4c0, logged::<0x0d>),
            (0x00c5_b8d0, logged::<0x0e>),
            (0x00e7_4d40, logged::<0x0f>),
            (0x00b7_18b0, logged::<0x10>),
            (0x00b7_1600, logged::<0x11>),
            (0x00b4_f5c0, logged::<0x12>),
            (0x0065_3290, logged::<0x13>),
            (0x00b5_aa40, logged::<0x14>),
            (0x00aa_42e0, logged::<0x15>),
            (0x00aa_5610, logged::<0x16>),
        ] {
            e.register(address, f);
        }
        e.register(VT_FN + 4, logged::<0x17>);
        e.register(0x00fd_f1c0, count_up);
        e.register(0x00fd_f1c4, count_down);
        e.register(0x00aa_54a0, scrap_block);
        set_logged_answer(e, 0x15, 0x0e77_0000);
    }

    /// `n` shadow scene lights in pattern `k`: their `NiLight`s at places from the case's floats at
    /// `f`, radii 10, 0.5, 50, a radius-free one now and then (radius 0.01); every fourth view-culled, every
    /// third with equal luminance to the one before (the same place).
    fn lights_of(e: &mut Engine, case: &Case, n: usize, k: usize, f: usize) -> Vec<u32> {
        let radii = [0x4120_0000u32, HALF, 0x4248_0000];
        let planes = [[0; 4]; 6];
        (0..n)
            .map(|i| {
                let rtti = if (i + k) % 5 == 4 {
                    RADIUS_FREE_RTTI
                } else {
                    OTHER_RTTI
                };
                // A radius-free light gets a radius that reaches nothing, so only its kind lets it in.
                let radius = if rtti == RADIUS_FREE_RTTI {
                    0x3c23_d70a
                } else {
                    radii[(i / 3 + k) % 3]
                };
                let ni = ni_light(e, case, f, radius, rtti);
                words(
                    e,
                    ni + 0x8c,
                    &[bits(case, f), [0, ONE, TWO][i / 3 % 3], TWO],
                );
                if rtti == RADIUS_FREE_RTTI {
                    // ... and stands 1000 units off.
                    e.mem.set_u32(ni + 0x8c, 0x447a_0000);
                }
                let l = light(e, &planes, 0, ni, 0, 0);
                if (i + k) % 4 == 3 {
                    e.mem.set_u16(l + 0x110, 0xff);
                }
                l
            })
            .collect()
    }

    /// A list at `list` (head, tail, count) of `elements` as `NiTListItem`s.
    fn list(e: &mut Engine, list: u32, elements: &[u32]) {
        let items: Vec<u32> = elements.iter().map(|_| e.mem.alloc(0xc)).collect();
        for (i, (&item, &element)) in items.iter().zip(elements).enumerate() {
            let next = items.get(i + 1).copied().unwrap_or(0);
            let previous = if i == 0 { 0 } else { items[i - 1] };
            words(e, item, &[next, previous, element]);
        }
        words(
            e,
            list,
            &[
                items.first().copied().unwrap_or(0),
                items.last().copied().unwrap_or(0),
                items.len() as u32,
            ],
        );
    }

    /// The scene node with its light list (`lights`) and the scene node pointer.
    fn scene_node(e: &mut Engine, lights: &[u32]) -> u32 {
        let node = e.mem.u32(0x011f_91c8);
        list(e, node + 0xb4, lights);
        node
    }

    /// The parent of kind `kind` (0 none, 1 a room, 2 the other kind, 3 some other object) holding
    /// `lights` in its array.
    fn parent(e: &mut Engine, kind: u32, lights: &[u32]) -> u32 {
        let rtti = match kind {
            0 => return 0,
            1 => ROOM_RTTI,
            2 => OTHERS_RTTI,
            _ => OTHER_RTTI,
        };
        let p = object(e, 0x100, 0, 0, rtti);
        let array = block(e, 4 * lights.len() as u32 + 4, lights);
        let (at, count) = if kind == 1 {
            (0xe0, 0xe4)
        } else {
            (0xb4, 0xb8)
        };
        e.mem.set_u32(p + at, array);
        e.mem.set_u32(p + count, lights.len() as u32);
        p
    }

    /// The queued object of a case: a geometry (its bound from the case's floats) or a tree.
    fn queued_object(e: &mut Engine, case: &Case, tree_or_geometry: u32, f: usize) -> u32 {
        if tree_or_geometry == 0 {
            let g = geometry(e, true, 1, false);
            let b = block(e, 0x10, &[bits(case, f), bits(case, f + 1), ONE, TWO]);
            e.mem.set_u32(g + 0x20, b);
            e.mem.set_u32(g + 0x30, 0);
            g
        } else {
            tree(e, case, tree_or_geometry, f)
        }
    }

    trait QueueCommon {
        fn queue_common(self) -> Diff;
    }

    impl QueueCommon for Diff {
        fn queue_common(self) -> Diff {
            self.family(&lights::LIGHTS)
                .family(&leaf::LEAVES)
                .ignore_calls(&[0x00ec_6040])
                .max_calls(50_000)
                .float_sweep(64)
        }
    }

    #[test]
    fn lights_update_start() {
        // The object (a geometry, or a tree pattern); its parent's kind; the skip and collect
        // flags; the number of lights and their pattern; the multibound cast's answer; floats for
        // the lights' and the object's places.
        Diff::new(entry!(0x00b5_dac0, queue::update_start(u32, u32, u32, u32) -> u8))
            .queue_common()
            .inputs(&START_INPUTS)
            .max_cases(400)
            .setup(|e, case| start_world(e, case).0)
            .run()
            .assert_same();
    }

    const START_INPUTS: [Input; 9] = [
        Input::Of(&[0, 1, 4, 5]),
        Input::Of(&[0, 1, 2, 3]),
        Input::Flag,
        Input::Flag,
        Input::Of(&[3, 0, 1, 7]),
        Input::Of(&[0, 1, 2]),
        Input::Flag,
        Input::F32,
        Input::F32Swept(ONE),
    ];

    /// The world of an object-half case: answers the call's words, the lights, the object and its
    /// parent.
    fn start_world(e: &mut Engine, case: &Case) -> (Vec<u32>, Vec<u32>, u32, u32) {
        queue_stand_ins(e);
        let lights = lights_of(e, case, case.word(4) as usize, case.word(5) as usize, 7);
        let node = scene_node(e, &lights);
        let object = queued_object(e, case, case.word(0), 7);
        let p = parent(e, case.word(1), &lights);
        e.mem.set_u32(object + 0x18, p);
        // The multibound cast: none, or the node itself with its first child indexed.
        if case.word(6) == 1 && case.word(0) != 0 {
            set_logged_answer(e, 0x13, object);
            let first = e.mem.u32(e.mem.u32(object + 0xa0));
            set_logged_answer(e, 0x14, first);
        }
        (
            vec![object, node, case.word(2), case.word(3)],
            lights,
            object,
            p,
        )
    }

    /// The object half with the exe's bodies in every slot the queue answers in place: the tree's
    /// `IsNode` and `IsTriBasedGeom` (`006815c0`, `00e68810`, `00acbb70`), the lamps' `GetRTTI`
    /// (the point light's `00a7d790` for the ordinary lamps, the directional light's `00a75c70` for
    /// the radius-free ones: the same answers as their stand-ins' records) and an other parent's
    /// (the ambient light's `00a9b390`: neither a room nor the other kind, as its stand-in). The
    /// exe's bodies are left out of the calls, so a site gone back to the vtable shows as a call
    /// the translation lacks; with the light table on too (its kind read).
    #[test]
    fn lights_update_start_exe_bodies() {
        for table in [false, true] {
            queue::force_light_table(Some(table));
            let report =
                Diff::new(entry!(0x00b5_dac0, queue::update_start(u32, u32, u32, u32) -> u8))
                    .queue_common()
                    .ignore_calls(&super::EXE_BODIES)
                    .inputs(&START_INPUTS)
                    .max_cases(400)
                    .setup(|e, case| {
                        let (call, lights, object, parent) = start_world(e, case);
                        super::exe_bodies(e, object);
                        for l in lights {
                            let ni = e.mem.u32(l + 0xf8);
                            let vt = e.mem.u32(ni);
                            let free = e.mem.u32(vt + VT_SLOTS + 8) == RADIUS_FREE_RTTI;
                            let body = if free { 0x00a7_5c70 } else { 0x00a7_d790 };
                            e.mem.set_u32(vt + 8, body);
                        }
                        if parent != 0 {
                            let vt = e.mem.u32(parent);
                            if e.mem.u32(vt + VT_SLOTS + 8) == OTHER_RTTI {
                                e.mem.set_u32(vt + 8, 0x00a9_b390);
                            }
                        }
                        call
                    })
                    .run();
            queue::force_light_table(None);
            report.assert_same();
        }
    }

    /// The inputs of the per-geometry update and its walk: a null node (the accumulator's, or the
    /// pointer's); the property's kind (as made, none, -1, 13); the candidates' kind; the number of
    /// lights; the tree pattern; floats.
    const GEOMETRY_INPUTS: [Input; 7] = [
        Input::Flag,
        Input::Of(&[1, 0, 0xffff_ffff, 13]),
        Input::Of(&[0, 1, 2]),
        Input::Of(&[3, 0, 5]),
        Input::Of(&[0, 1, 4, 5]),
        Input::F32,
        Input::F32Swept(ONE),
    ];

    /// The world of a per-geometry case: the geometry (`tree_pattern` 0) or a tree; answers the
    /// call's words.
    fn geometry_case(e: &mut Engine, case: &Case, tree_pattern: u32) -> Vec<u32> {
        queue_stand_ins(e);
        let lights = lights_of(e, case, case.word(3) as usize, 1, 5);
        let node = scene_node(e, &lights);
        let accumulator = block(e, 0x200, &[]);
        e.mem.set_u32(
            accumulator + 0x194,
            if case.word(0) == 1 { 0 } else { node },
        );
        set_logged_answer(e, 0x12, accumulator);
        let object = queued_object(e, case, tree_pattern, 5);
        if tree_pattern == 0 {
            match case.word(1) {
                0 => e.mem.set_u32(object + 0xa8, 0),
                1 => {}
                kind => {
                    let p = e.mem.u32(object + 0xa8);
                    e.mem.set_u32(p + 0x1c, kind);
                }
            }
        }
        let rooms = if case.word(2) == 1 {
            parent(e, 1, &lights)
        } else {
            0
        };
        let others = if case.word(2) == 2 {
            parent(e, 2, &lights)
        } else {
            0
        };
        let given = if case.word(0) == 1 { 0 } else { node };
        vec![object, given, 0, rooms, others]
    }

    #[test]
    fn lights_update_geometry() {
        Diff::new(entry!(0x00b5_b260, queue::update_geometry(u32, u32, u32, u32, u32) -> u8))
            .queue_common()
            .inputs(&GEOMETRY_INPUTS)
            .max_cases(300)
            .setup(|e, case| geometry_case(e, case, 0))
            .run()
            .assert_same();
    }

    #[test]
    fn lights_update_tree() {
        Diff::new(entry!(0x00b5_c490, queue::update_tree(u32, u32, u32, u32, u32) -> u8))
            .queue_common()
            .inputs(&GEOMETRY_INPUTS)
            .max_cases(300)
            .setup(|e, case| geometry_case(e, case, case.word(4)))
            .run()
            .assert_same();
    }

    /// The world of a lamp-array case: the zero-ended array of 0, 1, 3 or 6 lamps and the geometry
    /// (`tree_pattern` 0) or a tree; answers the call's words.
    fn list_case(e: &mut Engine, case: &Case, tree_pattern: u32) -> Vec<u32> {
        queue_stand_ins(e);
        let lights = lights_of(e, case, case.word(1) as usize, 0, 2);
        let mut zero_ended = lights.clone();
        zero_ended.push(0);
        let array = block(e, 4 * zero_ended.len() as u32, &zero_ended);
        let object = queued_object(e, case, tree_pattern, 2);
        vec![object, 0, array, 0]
    }

    const LIST_INPUTS: [Input; 4] = [
        Input::Of(&[0, 1, 4, 5]),
        Input::Of(&[3, 0, 1, 6]),
        Input::F32,
        Input::F32Swept(ONE),
    ];

    #[test]
    fn lights_update_geometry_list() {
        Diff::new(entry!(0x00b5_b410, queue::update_geometry_list(u32, u32, u32, u32) -> u8))
            .queue_common()
            .inputs(&LIST_INPUTS)
            .max_cases(100)
            .setup(|e, case| list_case(e, case, 0))
            .run()
            .assert_same();
    }

    #[test]
    fn lights_update_tree_list() {
        Diff::new(entry!(0x00b5_c5c0, queue::update_tree_list(u32, u32, u32, u32) -> u8))
            .queue_common()
            .inputs(&LIST_INPUTS)
            .max_cases(100)
            .setup(|e, case| list_case(e, case, case.word(0)))
            .run()
            .assert_same();
    }

    /// The light table (`NV_LIGHT_TABLE`) on and off give the same drain: the same memory (the
    /// flags, the luminances, the sorted arrays) and the same calls of the doubles with their words
    /// (every lamp add, list add and move, fence and removal), the kind calls left out; with it on,
    /// fewer kind calls are made.
    #[test]
    fn lights_table_gives_the_same_lists() {
        let mut fewer = false;
        for k in 0..12u32 {
            let case = Case(vec![
                Value::Word(k % 3),
                Value::Word((k / 3) % 3),
                Value::Word(k % 3),
                Value::Word([0, 1, 3][(k % 3) as usize]),
                Value::Word(k % 2),
                Value::F32([0, ONE, HALF, 0xc0a0_0000][(k % 4) as usize]),
                Value::F32(ONE),
            ]);
            let run = |table: bool| {
                let mut e = Engine::new();
                queue::force_light_table(Some(table));
                let node = drain_world(&mut e, &case);
                e.call(0x00b5_fd60, &[node]);
                queue::force_light_table(None);
                e
            };
            let off = run(false);
            let on = run(true);
            let differences: Vec<u32> = memory_differences_beside_stack(&off, &on, 64)
                .into_iter()
                .filter(|a| !(LOG..LOG + 0x4000).contains(a))
                .collect();
            assert!(
                differences.is_empty(),
                "case {k}: memory differs at {differences:08x?}"
            );
            // The doubles' log, the kind calls left out: every add (the property's lamp add, the
            // lamp's list add and move), fence and removal, with its words, in order.
            let calls = |e: &Engine| {
                let n = e.mem.u32(LOG);
                assert!(n < 0x7c0, "case {k}: the log is full");
                (0..n)
                    .map(|i| (e.mem.u32(LOG + 4 + 8 * i), e.mem.u32(LOG + 8 + 8 * i)))
                    .filter(|(tag, _)| *tag != 0xff08)
                    .collect::<Vec<_>>()
            };
            assert_eq!(calls(&off), calls(&on), "case {k}: the adds differ");
            let kind_calls = |e: &Engine| {
                let n = e.mem.u32(LOG);
                (0..n)
                    .filter(|i| e.mem.u32(LOG + 4 + 8 * i) == 0xff08)
                    .count()
            };
            assert!(kind_calls(&on) <= kind_calls(&off), "case {k}");
            fewer |= kind_calls(&on) < kind_calls(&off);
        }
        assert!(fewer, "the table never saved a kind call: it was not used");
    }

    /// The drain's world of a case: queued lights to add (shadow casters or not), to update
    /// (dynamic or not), to remove (a shadow caster with a map or not); queued objects in both
    /// queues (geometries and trees, near or far); a portal graph or none; floats for the places.
    /// Answers the node.
    fn drain_world(e: &mut Engine, case: &Case) -> u32 {
        queue_stand_ins(e);
        let lights = lights_of(e, case, 6, 1, 5);
        let node = scene_node(e, &lights);
        // Lights queued to add and to update, flags by position.
        let queued: Vec<u32> = (0..case.word(0) as usize + 1)
            .map(|i| {
                let l = lights[i];
                e.mem.set_u8(l + 0xec, u8::from(i == 1));
                e.mem.set_u8(l + 0xfc, u8::from(i != 1));
                l
            })
            .collect();
        list(e, node + 0xe4, &queued[..case.word(0) as usize]);
        list(
            e,
            node + 0xfc,
            &queued[..case.word(1).min(queued.len() as u32) as usize],
        );
        // Lights queued for removal, each with a reference count; the first a shadow
        // caster holding a map.
        let removed: Vec<u32> = (0..case.word(2) as usize)
            .map(|i| {
                let l = object(e, 0x250, 0, 0, OTHER_RTTI);
                e.mem.set_u32(l + 4, 1);
                if i == 0 {
                    e.mem.set_u8(l + 0xec, 1);
                    let map = object(e, 0x20, 0, 0, OTHER_RTTI);
                    e.mem.set_u32(map + 4, 1);
                    e.mem.set_u32(l + 0x10c, map);
                }
                l
            })
            .collect();
        list(e, node + 0xf0, &removed);
        e.mem.set_u32(0x011f_91a8, 0x0e55_0000);
        // Queued objects: geometries and trees, one far (radius 200).
        let objects: Vec<u32> = (0..case.word(3) as usize)
            .map(|i| {
                let o = queued_object(e, case, [0, 1, 5][i % 3], 5);
                if i == 2 {
                    let b = e.mem.u32(o + 0x20);
                    e.mem.set_f32(b + 0xc, 200.0);
                }
                o
            })
            .collect();
        let split = objects.len() / 2;
        list(e, node + 0x114, &objects[..split]);
        list(e, node + 0x108, &objects[split..]);
        if case.word(4) == 1 {
            e.mem.set_u32(node + 0x1e0, 0x0e56_0000);
        }
        words(e, 0x011f_9ea0, &[3, 1]);
        words(e, 0x011f_9ec0, &[4, 2]);
        node
    }

    #[test]
    fn lights_queue_drain() {
        Diff::new(entry!(0x00b5_fd60, queue::drain(Ptr<_>)))
            .queue_common()
            .inputs(&[
                Input::Of(&[0, 1, 2]),
                Input::Of(&[0, 1, 2]),
                Input::Of(&[0, 1, 2]),
                Input::Of(&[0, 1, 3]),
                Input::Flag,
                Input::F32,
                Input::F32Swept(ONE),
            ])
            .max_cases(300)
            .setup(|e, case| vec![drain_world(e, case)])
            .run()
            .assert_same();
    }
}

/// The fixes row "dead lights in a light list" (`engine_typed::lights::dead`): a lighting property
/// whose list still holds a light destroyed while the property was fenced (the light's destructor
/// `00b9f0c0`, translated, ran with the property's fence word set, so its `RemoveLight` did
/// nothing) is walked by each guarded walker without a stop: `ResortLights` and the sweep take the
/// entry out (its node unlinked as `RemoveLight`'s list remove does, the list marked changed, the
/// passes' key cleared); the active-light getters and counts skip it; the pass staging stages no
/// light. With the row off nothing is taken out and `ResortLights` reads through the light, as the
/// exe does, and stops.
mod dead_lights {
    use engine_core::prelude::*;
    use engine_typed::draw::lights as getters;
    use engine_typed::lights::{dead, resort};
    use engine_typed::shader::lighting;

    /// `ShadowSceneLight`'s vtable, `NiRefObject`'s (what the destructor leaves), the objects' count
    /// the destructor lowers, the fence marker.
    const LIGHT_VTABLE: u32 = 0x010b_7eb8;
    const REF_OBJECT_VTABLE: u32 = 0x0101_dce4;
    const OBJECT_COUNT: u32 = 0x011f_4400;
    const FENCE_MARKER: u32 = 0x011f_a310;
    /// The stand-ins: `InterlockedDecrement`, the list's remove-all and remove of an element, and
    /// the release hook.
    const INTERLOCKED_DECREMENT: u32 = 0x00fd_f1c4;
    const LIST_REMOVE_ALL: u32 = 0x00e7_4d40;
    const LIST_REMOVE: u32 = 0x00e7_db70;
    const RELEASE_HOOK: u32 = 0x00b6_d4c0;
    /// A vtable whose slot 4 (the deleting destructor) does nothing.
    const QUIET_VTABLE: u32 = 0x0090_0000;
    const QUIET_DELETE: u32 = 0x0090_1000;

    fn decrement(e: &mut Engine, a: &[u32]) -> Ret {
        let count = e.mem.u32(a[0]).wrapping_sub(1);
        e.mem.set_u32(a[0], count);
        Ret {
            eax: count,
            ..Ret::default()
        }
    }

    /// `NiTPointerList`'s remove-all: the list (head, tail, count) emptied.
    fn remove_all(e: &mut Engine, a: &[u32]) -> Ret {
        for word in 0..3u32 {
            e.mem.set_u32(a[0] + 4 * word, 0);
        }
        Ret::default()
    }

    /// The list's remove of an element (`thiscall(list, slot)`): the first node holding the slot's
    /// word unlinked (head, tail and count kept right), as the exe's does before freeing it.
    fn remove(e: &mut Engine, a: &[u32]) -> Ret {
        let (list, element) = (a[0], e.mem.u32(a[1]));
        let mut node = e.mem.u32(list);
        while node != 0 {
            let (next, previous) = (e.mem.u32(node), e.mem.u32(node + 4));
            if e.mem.u32(node + 8) == element {
                if previous == 0 {
                    e.mem.set_u32(list, next);
                } else {
                    e.mem.set_u32(previous, next);
                }
                if next == 0 {
                    e.mem.set_u32(list + 4, previous);
                } else {
                    e.mem.set_u32(next + 4, previous);
                }
                let count = e.mem.u32(list + 8) - 1;
                e.mem.set_u32(list + 8, count);
                break;
            }
            node = next;
        }
        Ret::default()
    }

    /// A list (head, tail, count) at `list` holding `elements` in order.
    fn list(e: &mut Engine, list: u32, elements: &[u32]) {
        let nodes: Vec<u32> = elements.iter().map(|_| e.mem.alloc(0xc)).collect();
        for (i, (&node, &element)) in nodes.iter().zip(elements).enumerate() {
            e.mem.set_u32(node, nodes.get(i + 1).copied().unwrap_or(0));
            e.mem
                .set_u32(node + 4, if i == 0 { 0 } else { nodes[i - 1] });
            e.mem.set_u32(node + 8, element);
        }
        e.mem.set_u32(list, nodes.first().copied().unwrap_or(0));
        e.mem.set_u32(list + 4, nodes.last().copied().unwrap_or(0));
        e.mem.set_u32(list + 8, nodes.len() as u32);
    }

    fn elements(e: &Engine, list: u32) -> Vec<u32> {
        let mut out = Vec::new();
        let mut node = e.mem.u32(list);
        while node != 0 {
            out.push(e.mem.u32(node + 8));
            node = e.mem.u32(node);
        }
        out
    }

    /// The world: a property (a lighting property of type 1, fenced) listing the light to be
    /// destroyed and a live one, a geometry carrying the property in the first light's geometry
    /// list; that light's destructor run (its `RemoveLight` on the fenced property does nothing),
    /// then the fence cleared, as `RemoveFencedLights` leaves a light that was added again. Answers
    /// the engine, the property, the dead light and the live one.
    fn world(guard: bool) -> (Engine, u32, u32, u32) {
        let mut e = Engine::new();
        e.register(INTERLOCKED_DECREMENT, decrement);
        e.register(LIST_REMOVE_ALL, remove_all);
        e.register(LIST_REMOVE, remove);
        e.register(RELEASE_HOOK, |_, _| Ret::default());
        e.register(QUIET_DELETE, |_, _| Ret::default());
        e.map(OBJECT_COUNT, 4);
        e.set_global(OBJECT_COUNT, 10u32);
        e.put_vtable(QUIET_VTABLE, &[0, QUIET_DELETE]);
        // The scene node the luminance reads its lighting offset from, the staging's rows and its
        // zero colour.
        e.map(0x011f_9000, 0x2000);
        e.map(0x011a_9000, 0x1000);
        let node = e.mem.alloc(0x200);
        e.set_global(0x011f_91c8, node);
        let ni_light = |e: &mut Engine| {
            let ni = e.mem.alloc(0x100);
            e.mem.set_u32(ni, QUIET_VTABLE);
            e.mem.set_u32(ni + 4, 2);
            ni
        };
        let shadow_light = |e: &mut Engine, ni: u32| {
            let light = e.mem.alloc(0x250);
            e.mem.set_u32(light, LIGHT_VTABLE);
            e.mem.set_u32(light + 4, 1);
            e.mem.set_u32(light + 0xf8, ni);
            light
        };
        let live_ni = ni_light(&mut e);
        let live = shadow_light(&mut e, live_ni);
        let dying_ni = ni_light(&mut e);
        let dying = shadow_light(&mut e, dying_ni);
        let property = e.mem.alloc(0x80);
        e.mem.set_u32(property + 0x1c, 1);
        list(&mut e, property + 0x60, &[dying, live]);
        let geometry = e.mem.alloc(0xc0);
        e.mem.set_u32(geometry + 0xa8, property);
        list(&mut e, dying + 0xe0, &[geometry]);
        // Fenced: the destructor's RemoveLight on this property does nothing.
        e.mem.set_u32(property + 0x78, 1);
        e.call(0x00b9_f0c0, &[dying]);
        e.mem.set_u32(property + 0x78, 0);
        assert_eq!(
            elements(&e, property + 0x60),
            vec![dying, live],
            "the property kept the destroyed light"
        );
        assert_eq!(e.mem.u32(dying), REF_OBJECT_VTABLE);
        assert_eq!(e.mem.u32(dying + 0xf8), 0);
        dead::set_guard(&e, guard);
        (e, property, dying, live)
    }

    #[test]
    fn a_dead_light_is_taken_out_by_the_sort_and_passed_over_by_the_walks() {
        // The sort: the dead entry taken out, the list marked changed and the passes' key cleared.
        let (mut e, property, _, live) = world(true);
        e.mem.set_u32(property + 0x38, 7);
        let bound = e.mem.alloc(0x10);
        resort::resort_lights(&mut e, Ptr::new(property), bound);
        assert_eq!(elements(&e, property + 0x60), vec![live]);
        assert_eq!(e.mem.u32(property + 0x68), 1);
        assert_eq!(e.mem.u8(property + 0x74), 1);
        assert_eq!(e.mem.u32(property + 0x38), 0);
        // The sweep itself (as GetRenderPasses runs it): nothing more to take out.
        assert!(!dead::sweep(&mut e, property, 0));
        dead::set_guard(&e, false);

        // The walks over a list still holding it: the getters and counts pass over it, the
        // staging stages no light.
        let (mut e, property, dying, live) = world(true);
        let iterator = e.mem.alloc(4);
        assert_eq!(
            getters::first_active_light(&mut e, Ptr::new(property), Ptr::new(iterator)),
            live,
            "the first active light is the live one"
        );
        assert_eq!(
            getters::next_active_light(&mut e, Ptr::new(property), Ptr::new(iterator)),
            0
        );
        assert_eq!(
            getters::first_active_non_shadow_light(&mut e, Ptr::new(property), Ptr::new(iterator)),
            live
        );
        assert_eq!(getters::active_light_count(&mut e, Ptr::new(property)), 1);
        assert_eq!(
            getters::active_non_shadow_light_count(&mut e, Ptr::new(property)),
            1
        );
        e.mem.set_u32(0x011f_a0d0, 0x1234_5678);
        lighting::light_setup(&mut e, 0, Ptr::new(dying), 1.0);
        assert_eq!(
            e.mem.u32(0x011f_a0d0),
            e.mem.u32(0x011a_9bd0),
            "the dead light's colour row is the zero colour"
        );
        assert_eq!(elements(&e, property + 0x60), vec![dying, live]);
        dead::set_guard(&e, false);
    }

    #[test]
    fn with_the_row_off_the_sort_reads_through_the_dead_light() {
        let (mut e, property, dying, live) = world(false);
        assert!(!dead::sweep(&mut e, property, 0), "nothing taken out");
        let bound = e.mem.alloc(0x10);
        let stopped = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            resort::resort_lights(&mut e, Ptr::new(property), bound);
        }));
        assert!(stopped.is_err(), "the exe's read through spLight 0 stops");
        assert_eq!(elements(&e, property + 0x60), vec![dying, live]);
    }

    /// The fence marker is never taken for a dead light, nor a null entry (the walkers stop there).
    #[test]
    fn the_fence_marker_is_not_a_dead_light() {
        let (e, _, dying, live) = world(true);
        assert!(!dead::is_dead(&e, FENCE_MARKER));
        assert!(dead::is_dead(&e, dying));
        assert!(!dead::is_dead(&e, live));
        assert!(
            !dead::is_dead(&e, 0),
            "a null entry is left to the walkers, as the exe's"
        );
        dead::set_guard(&e, false);
    }

    /// The translated `GetRenderPasses` of the decal property (vtable `010b9910`, `00bb4740`) and
    /// of the `010bcb60` class (`00c058f0`), called by address as the accumulator calls them, with
    /// the property's state at its key and its list not marked changed (so the exe keeps the
    /// passes, whose lights hold the dead one) and a light destroyed while the property was
    /// fenced: with the row's hooks the dead entry is taken out first, the lights resorted and the
    /// passes rebuilt; with the row off the passes are kept and nothing is touched, as the exe.
    #[test]
    fn decal_and_other_lighting_passes_take_out_a_dead_light_and_rebuild() {
        const HAS_FLAG: u32 = 0x004a_2020;
        const RESORT_LIGHTS: u32 = 0x00b7_0390;
        const CHECK_CREATE_RENDER_PASSES: u32 = 0x00ba_95e0;
        /// Two words the stand-ins mark: resorted, rebuilt.
        const MARKS: u32 = 0x0091_0000;
        for (at, vtable) in [(0x00bb_4740u32, 0x010b_9910u32), (0x00c0_58f0, 0x010b_cb60)] {
            for guard in [false, true] {
                let (mut e, property, dying, live) = world(guard);
                if guard {
                    engine::dead_lights::install(&mut e);
                }
                e.map(MARKS, 8);
                e.register(HAS_FLAG, |_, _| Ret::default());
                e.register(RESORT_LIGHTS, |e, _| {
                    e.set_global(MARKS, 1u32);
                    Ret::default()
                });
                e.register(CHECK_CREATE_RENDER_PASSES, |e, _| {
                    e.set_global(MARKS + 4, 1u32);
                    Ret::default()
                });
                e.mem.set_u32(property, vtable);
                // No fades or flags; the alpha 0, so a rebuild ends right after its pass list is
                // made; the state at the key of pass kind 0x10 with render mode 3.
                for offset in [0x20u32, 0x24, 0x28] {
                    e.mem.set_u32(property + offset, 0);
                }
                let passes = e.mem.alloc(0x20);
                e.mem.set_u32(property + 0x3c, passes);
                e.mem.set_u32(property + 0x38, (0x10 << 8) | 3);
                e.mem.set_u8(property + 0x74, 0);
                let geometry = e.mem.alloc(0xc0);
                e.call(at, &[property, geometry, 3, 0, 0x10, 0, 1]);
                let (resorted, rebuilt) =
                    (e.global::<u32>(MARKS) == 1, e.global::<u32>(MARKS + 4) == 1);
                if guard {
                    assert_eq!(
                        elements(&e, property + 0x60),
                        vec![live],
                        "{at:08x}: the dead light taken out"
                    );
                    assert!(resorted && rebuilt, "{at:08x}: resorted and rebuilt");
                    assert_eq!(e.mem.u8(property + 0x74), 0, "{at:08x}: rebuilt, unmarked");
                } else {
                    assert_eq!(
                        elements(&e, property + 0x60),
                        vec![dying, live],
                        "{at:08x}: row off: kept"
                    );
                    assert!(
                        !resorted && !rebuilt,
                        "{at:08x}: row off: the passes kept, as the exe"
                    );
                }
                dead::set_guard(&e, false);
            }
        }
    }
}

/// The light's own functions (`lights::scene_light`): each world builds the light, its `NiLight`
/// (with the exe's `GetRTTI` bodies in its RTTI slot, or a stand-in answering a record chained to
/// one), its lists and the globals the function reads, from the case's words so that each branch
/// is taken in some case; the luminances run under both control words with swept floats.
mod scene_light {
    use super::{
        bits, block, light, log, logged, object, ret, set_logged_answer, stand_ins, words,
        EXE_BODIES, HALF, LOG, MINUS_ONE, ONE, OTHER_RTTI, RADIUS_FREE_RTTI, TWO, VT_FN, VT_SLOTS,
    };
    use engine_core::prelude::*;
    use engine_core::typed::diff::{Case, Diff, Input};
    use engine_core::typed::{MAIN_THREAD, OTHER_THREADS};
    use engine_typed::lights::{self, scene_light};

    const BOTH: [u16; 2] = [OTHER_THREADS, MAIN_THREAD];
    const NAN: u32 = 0x7fc0_0000;
    const TEN: u32 = 0x4120_0000;
    const POINT_RTTI: u32 = 0x011f_4a98;
    const AMBIENT_RTTI: u32 = 0x011f_5f6c;
    /// The exe's `GetRTTI` bodies of the point, directional and ambient lights.
    const POINT_BODY: u32 = 0x00a7_d790;
    const DIRECTIONAL_BODY: u32 = 0x00a7_5c70;
    const AMBIENT_BODY: u32 = 0x00a9_b390;

    fn count_up(e: &mut Engine, a: &[u32]) -> Ret {
        let n = e.mem.u32(a[0]).wrapping_add(1);
        e.mem.set_u32(a[0], n);
        ret(n)
    }

    fn count_down(e: &mut Engine, a: &[u32]) -> Ret {
        let n = e.mem.u32(a[0]).wrapping_sub(1);
        e.mem.set_u32(a[0], n);
        ret(n)
    }

    /// A stand-in that logs its words under `TAG` and writes its answer word into the `NiPointer`
    /// its second word points at (a list removal's out pointer).
    fn out_writer<const TAG: u32>(e: &mut Engine, a: &[u32]) -> Ret {
        for w in a {
            log(e, TAG, *w);
        }
        let answer = e.mem.u32(LOG + 0x3f00 + 4 * (TAG & 0x3f));
        e.mem.set_u32(a[1], answer);
        ret(0)
    }

    /// The renderer's shadow map release: logs and, when `LOG + 0x3e00` names a word, clears it
    /// (the light's map, which the typed code must read again).
    fn release_map(e: &mut Engine, a: &[u32]) -> Ret {
        for w in a {
            log(e, 0x20, *w);
        }
        let at = e.mem.u32(LOG + 0x3e00);
        if at != 0 {
            e.mem.set_u32(at, 0);
        }
        ret(0)
    }

    /// The walk tests' stand-ins and the data these functions read: the imports' counts, the
    /// deleting slot, the list item free, the exe's 0.0 and 1.0 doubles, `Unitize`'s minimum and
    /// the spotlight's field of view.
    fn world(e: &mut Engine) {
        stand_ins(e);
        for page in [
            0x0101_2000,
            0x0101_7000,
            0x0102_4000,
            0x011a_d000,
            0x011f_5000,
        ] {
            e.map(page, 0x1000);
        }
        words(e, 0x0101_2060, &[0, 0, 0, 0, 0, 0x3ff0_0000]);
        words(e, 0x0101_7cf8, &[0xa000_0000, 0x3eb0_c6f7]);
        e.mem.set_u32(0x0102_49d8, 0x42b4_0000);
        e.register(0x00fd_f1c0, count_up);
        e.register(0x00fd_f1c4, count_down);
        e.register(VT_FN + 4, logged::<0x17>);
        e.register(0x00e6_e830, logged::<0x18>);
    }

    /// A counted object (its count at +4) of `size` bytes with its own vtable.
    fn counted(e: &mut Engine, size: u32, count: u32) -> u32 {
        let o = object(e, size, 0, 0, OTHER_RTTI);
        e.mem.set_u32(o + 4, count);
        o
    }

    /// An `NiLight` of `kind`: 0 none; 1, 2, 3 the point, ambient and directional lights' exe
    /// `GetRTTI`; 4, 5 a stand-in answering a record whose base is the point or the ambient light's
    /// record; 6 a stand-in answering an unrelated record. Its count 1, its place from the case's
    /// floats at `f`, its radius `radius`.
    fn ni_light_of(e: &mut Engine, case: &Case, kind: u32, f: usize, radius: u32) -> u32 {
        if kind == 0 {
            return 0;
        }
        let record = match kind {
            4 => block(e, 8, &[0, POINT_RTTI]),
            5 => block(e, 8, &[0, AMBIENT_RTTI]),
            _ => OTHER_RTTI,
        };
        let l = object(e, 0x100, 0, 0, record);
        let vt = e.mem.u32(l);
        match kind {
            1 => e.mem.set_u32(vt + 8, POINT_BODY),
            2 => e.mem.set_u32(vt + 8, AMBIENT_BODY),
            3 => e.mem.set_u32(vt + 8, DIRECTIONAL_BODY),
            _ => {}
        }
        e.mem.set_u32(l + 4, 1);
        words(e, l + 0x8c, &[bits(case, f), bits(case, f + 1), TWO]);
        e.mem.set_u32(l + 0xe0, radius);
        l
    }

    /// A shadow scene light with no planes, its `NiLight` `ni` and its `bCastShadow`.
    fn plain_light(e: &mut Engine, ni: u32, cast: u8) -> u32 {
        light(e, &[[0; 4]; 6], 0, ni, 0, cast)
    }

    trait Own {
        fn own(self) -> Diff;
    }

    impl Own for Diff {
        fn own(self) -> Diff {
            self.family(&lights::LIGHTS)
                .family(&leaf::LEAVES)
                .ignore_calls(&EXE_BODIES)
                .max_calls(20_000)
        }
    }

    #[test]
    fn lights_set_light() {
        // The new light's kind; the old one (none, the same, another deleted by its release,
        // another kept); bDynamicLight; floats for the new light's place.
        Diff::new(entry!(0x00b9_dda0, scene_light::set_light(Ptr<_>, Ptr<_>)))
            .own()
            .inputs(&[
                Input::Of(&[0, 1, 2, 3, 4, 5, 6]),
                Input::Of(&[0, 1, 2, 3]),
                Input::Flag,
                Input::F32,
                Input::F32Swept(ONE),
            ])
            .max_cases(400)
            .setup(|e, case| {
                world(e);
                let new = ni_light_of(e, case, case.word(0), 3, TEN);
                let old = match case.word(1) {
                    0 => 0,
                    1 => new,
                    2 => counted(e, 0x40, 1),
                    _ => counted(e, 0x40, 3),
                };
                let l = plain_light(e, old, 0);
                e.mem.set_u8(l + 0xfc, case.word(2) as u8);
                words(e, l + 0xd0, &[HALF]);
                words(e, l + 0x100, &[7, 8, 9]);
                words(e, l + 0xf4, &[0x0101_0101]);
                vec![l, new]
            })
            .run()
            .assert_same();
    }

    #[test]
    fn lights_construct() {
        // The block's old contents (a pattern or zeros); the field of view (90.0, a signalling NaN,
        // which the exe's load and store quiet, minus infinity); the zero point's words.
        Diff::new(entry!(
            0x00b9_fda0,
            scene_light::construct(Ptr<_>) -> Ptr<_>
        ))
        .own()
        .ignore_calls(&[0x00a6_9940, 0x00ec_61c0])
        .inputs(&[
            Input::Flag,
            Input::Of(&[0x42b4_0000, 0x7f80_0001, 0xff80_0000]),
            Input::F32,
            Input::F32Swept(MINUS_ONE),
        ])
        .max_cases(100)
        .setup(|e, case| {
            world(e);
            let l = e.mem.alloc(0x260);
            if case.word(0) != 0 {
                for i in 0..0x98 {
                    e.mem.set_u32(l + 4 * i, 0x5a5a_0000 | i);
                }
            }
            e.mem.set_u32(0x0102_49d8, case.word(1));
            words(e, 0x011f_426c, &[bits(case, 2), bits(case, 3), HALF]);
            vec![l]
        })
        .run()
        .assert_same();
    }

    /// A list at `list` of `n` items whose elements follow `pattern` (bit `i` clear: item `i`'s
    /// element null; set: an object with count 1).
    fn pointer_list(e: &mut Engine, list: u32, n: u32, pattern: u32) {
        let items: Vec<u32> = (0..n).map(|_| e.mem.alloc(0xc)).collect();
        for (i, &item) in items.iter().enumerate() {
            let next = items.get(i + 1).copied().unwrap_or(0);
            let previous = if i == 0 { 0 } else { items[i - 1] };
            let element = if pattern >> i & 1 != 0 {
                counted(e, 0x40, 1)
            } else {
                0
            };
            words(e, item, &[next, previous, element]);
        }
        let (head, tail) = (items.first().copied(), items.last().copied());
        words(e, list, &[head.unwrap_or(0), tail.unwrap_or(0), n]);
    }

    fn pop_world(e: &mut Engine, case: &Case) -> Vec<u32> {
        world(e);
        let list = block(e, 0xc, &[]);
        pointer_list(e, list, case.word(0), case.word(1));
        let out = block(e, 4, &[0xdead_beef]);
        vec![list, out]
    }

    #[test]
    fn lights_pop_head() {
        Diff::new(entry!(
            0x00b9_e7c0,
            scene_light::pop_head(Ptr<_>, Ptr<_>) -> Ptr<_>
        ))
        .own()
        .inputs(&[Input::Of(&[1, 2, 3]), Input::Of(&[0, 1, 2, 5, 7])])
        .setup(pop_world)
        .run()
        .assert_same();
    }

    #[test]
    fn lights_pop_tail() {
        Diff::new(entry!(
            0x00b9_e850,
            scene_light::pop_tail(Ptr<_>, Ptr<_>) -> Ptr<_>
        ))
        .own()
        .inputs(&[Input::Of(&[1, 2, 3]), Input::Of(&[0, 1, 2, 4, 7])])
        .setup(pop_world)
        .run()
        .assert_same();
    }

    #[test]
    fn lights_set_held_1a0() {
        // The held object (none, the given one, another deleted by its release, another kept); the
        // given object (none, one).
        Diff::new(entry!(0x00b9_cfd0, scene_light::set_held_1a0(Ptr<_>, u32)))
            .own()
            .inputs(&[Input::Of(&[0, 1, 2, 3]), Input::Flag])
            .setup(|e, case| {
                world(e);
                let given = if case.word(1) != 0 {
                    counted(e, 0x40, 2)
                } else {
                    0
                };
                let held = match case.word(0) {
                    0 => 0,
                    1 => given,
                    2 => counted(e, 0x40, 1),
                    _ => counted(e, 0x40, 3),
                };
                let this = block(e, 0x200, &[]);
                e.mem.set_u32(this + 0x1a0, held);
                vec![this, given]
            })
            .run()
            .assert_same();
    }

    /// Radii: ordinary, zero, negative, NaN, huge, small.
    const RADII: &[u32] = &[TEN, 0, MINUS_ONE, NAN, 0x7f00_0000, HALF];

    #[test]
    fn lights_luminance_to_bound() {
        // The `NiLight`'s radius; the bound's radius; floats for the `NiLight`, the bound's centre
        // and the scene node's lighting offset.
        Diff::new(entry!(
            0x00b9_dbe0,
            scene_light::luminance_to_bound(Ptr<_>, Ptr<_>) -> f32
        ))
        .own()
        .ignore_calls(&[0x00ec_6040])
        .control_words(&BOTH)
        // x87 and SSE pick different NaNs of two (the float rule accepts it).
        .allow_nan_payloads()
        .inputs(&[
            Input::Of(RADII),
            Input::Of(&[HALF, 0, TEN, NAN]),
            Input::F32,
            Input::F32Swept(ONE),
            Input::F32Swept(TWO),
            Input::F32Swept(HALF),
            Input::F32Swept(MINUS_ONE),
        ])
        .max_cases(300)
        .float_sweep(96)
        .setup(|e, case| {
            world(e);
            let ni = ni_light_of(e, case, 6, 2, case.word(0));
            let l = plain_light(e, ni, 0);
            let node = e.mem.u32(0x011f_91c8);
            words(e, node + 0x1e4, &[bits(case, 5), bits(case, 6), HALF]);
            let bound = block(e, 0x10, &[bits(case, 4), bits(case, 3), ONE, case.word(1)]);
            vec![l, bound]
        })
        .run()
        .assert_same();
    }

    /// The colour components: ordinary, equal, zero, NaN.
    const COLOURS: &[u32] = &[ONE, HALF, TEN, 0, NAN];
    /// The colour of pattern 5: minus zero, minus zero, zero, where the largest component's
    /// "less than" (not "at most") decides which zero comes out.
    const ZEROS: [u32; 3] = [0x8000_0000, 0x8000_0000, 0];

    /// A light for the luminances, from the case's words at `w`: its `NiLight`'s colour (two words
    /// pick the pattern; 5 is [`ZEROS`]), dimmer and radius, its `bCastShadow` and `bPointLight`;
    /// floats for the `NiLight`'s place at `f`. Answers the light and its `NiLight`.
    fn luminance_light(e: &mut Engine, case: &Case, w: usize, f: usize) -> (u32, u32) {
        let ni = ni_light_of(e, case, 6, f, case.word(w + 3));
        let colour = |i: usize| {
            if case.word(w) == 5 {
                return ZEROS[i];
            }
            COLOURS[(case.word(w) as usize + i * case.word(w + 1) as usize) % COLOURS.len()]
        };
        words(e, ni + 0xd4, &[colour(0), colour(1), colour(2)]);
        e.mem.set_u32(ni + 0xc4, case.word(w + 2));
        let l = plain_light(e, ni, case.word(w + 4) as u8);
        e.mem.set_u8(l + 0xf4, case.word(w + 5) as u8);
        words(e, l + 0xc, &[0x4040_0000]);
        (l, ni)
    }

    /// The luminances' words: the colour pattern (two words), the dimmer, the radius,
    /// bCastShadow, bPointLight.
    const LUMINANCE_INPUTS: [Input; 6] = [
        Input::Of(&[0, 1, 2, 3, 4, 5]),
        Input::Of(&[0, 1, 2]),
        Input::Of(&[ONE, HALF, 0, NAN]),
        Input::Of(RADII),
        Input::Of(&[0, 1, 2]),
        Input::Flag,
    ];

    #[test]
    fn lights_luminance_at() {
        // The `NiLight` left out (the light's own or none); the luminance words; floats for the
        // `NiLight`'s place and the point.
        let mut inputs = vec![Input::Flag];
        inputs.extend(LUMINANCE_INPUTS);
        inputs.extend([
            Input::F32,
            Input::F32Swept(ONE),
            Input::F32Swept(TWO),
            Input::F32Swept(HALF),
            Input::F32Swept(MINUS_ONE),
        ]);
        Diff::new(entry!(
            0x00b9_bc50,
            scene_light::luminance_at(Ptr<_>, u32, u32, u32, u32) -> f32
        ))
        .own()
        .ignore_calls(&[0x00ec_6040])
        .control_words(&BOTH)
        // x87 and SSE pick different NaNs of two (the float rule accepts it).
        .allow_nan_payloads()
        .inputs(&inputs)
        .max_cases(600)
        .float_sweep(128)
        .setup(|e, case| {
            world(e);
            let (l, ni) = luminance_light(e, case, 1, 7);
            let excluded = if case.word(0) != 0 { ni } else { 0 };
            vec![l, bits(case, 9), bits(case, 10), bits(case, 11), excluded]
        })
        .run()
        .assert_same();
    }

    #[test]
    fn lights_facing_luminance() {
        // The `NiLight` left out; culled or not; the luminance words; the direction point `b` (the
        // point itself: a zero direction; a far point; from the case's floats; the light's own
        // place, so that both unit vectors are the same and their dot rounds to just above 1.0; the
        // point moved by 0.5 along x); floats for the `NiLight`'s place and the point `a`;
        // `Unitize`'s minimum (the exe's, or 0.5: a direction of exactly that length is zeroed).
        let mut inputs = vec![Input::Flag, Input::Flag];
        inputs.extend(LUMINANCE_INPUTS);
        inputs.extend([
            Input::Of(&[0, 1, 2, 3, 4]),
            Input::F32,
            Input::F32Swept(ONE),
            Input::F32Swept(TWO),
            Input::F32Swept(HALF),
            Input::F32Swept(MINUS_ONE),
            Input::F32Swept(TEN),
            Input::Of(&[0, 1]),
        ]);
        Diff::new(entry!(
            0x00b9_bdf0,
            scene_light::facing_luminance(Ptr<_>, u32, u32, u32, u32, u32, u32, u32) -> f32
        ))
        .own()
        .ignore_calls(&[
            0x00ec_6040,
            0x0045_7990,
            0x004a_0c10,
            0x0040_4010,
            0x0040_ebd0,
        ])
        .control_words(&BOTH)
        .allow_nan_payloads()
        .inputs(&inputs)
        .max_cases(800)
        .float_sweep(160)
        .setup(|e, case| {
            world(e);
            // The C runtime's `sqrt` under the point length's root (its error path calls imports).
            e.register(0x00ec_6054, |_, a| {
                super::ret_f(super::f64_arg(a, 0).sqrt())
            });
            let (l, ni) = luminance_light(e, case, 2, 9);
            let excluded = if case.word(0) != 0 { ni } else { 0 };
            e.mem.set_u32(ni + 0x30, case.word(1));
            let a = [bits(case, 11), bits(case, 12), bits(case, 13)];
            let b = match case.word(8) {
                0 => a,
                1 => [0x447a_0000, MINUS_ONE, bits(case, 14)],
                2 => [bits(case, 14), bits(case, 10), bits(case, 9)],
                3 => [bits(case, 9), bits(case, 10), TWO],
                _ => [(case.f32(11) + 0.5).to_bits(), a[1], a[2]],
            };
            if case.word(15) != 0 {
                words(e, 0x0101_7cf8, &[0, 0x3fe0_0000]);
            }
            vec![l, a[0], a[1], a[2], b[0], b[1], b[2], excluded]
        })
        .run()
        .assert_same();
    }

    #[test]
    fn lights_set_cast_shadow() {
        // The light's flag before and the one given; the render level (signed); the render flags'
        // bit 0x10; the map (none, deleted by its release, kept, cleared by the renderer's
        // release).
        Diff::new(entry!(
            0x00b9_dcb0,
            scene_light::set_cast_shadow(Ptr<_>, u8)
        ))
        .own()
        .inputs(&[
            Input::Flag,
            Input::Flag,
            Input::Of(&[3, 2, 0xffff_ffff, 7]),
            Input::Flag,
            Input::Of(&[0, 1, 2, 3]),
        ])
        .setup(|e, case| {
            world(e);
            e.register(0x00b6_d4c0, release_map);
            let l = plain_light(e, 0, case.word(0) as u8);
            e.mem.set_u32(0x011f_91a8, 0x0abc_0000);
            e.mem.set_u32(0x011f_91c0, case.word(2));
            e.mem
                .set_u8(0x011f_91d8, if case.word(3) != 0 { 0x13 } else { 0xef });
            let map = match case.word(4) {
                0 => 0,
                1 => counted(e, 0x40, 1),
                _ => counted(e, 0x40, 2),
            };
            e.mem.set_u32(l + 0x10c, map);
            if case.word(4) == 3 {
                e.mem.set_u32(LOG + 0x3e00, l + 0x10c);
            }
            let tls = e.tls();
            e.mem.set_u32(tls + 0x2b4, 0x1234);
            vec![l, case.word(1)]
        })
        .run()
        .assert_same();
    }

    /// A geometry (count 1) with a lighting property of type `kind` (0 none) at +0xa8 whose +0x38
    /// is set.
    fn lit_geometry(e: &mut Engine, kind: u32) -> u32 {
        let g = counted(e, 0x100, 1);
        if kind != 0 {
            let p = block(e, 0x40, &[]);
            words(e, p + 0x1c, &[kind]);
            words(e, p + 0x38, &[0x55]);
            e.mem.set_u32(g + 0xa8, p);
        }
        g
    }

    /// The property types: none, the bounds of 1 to 12, outside them.
    const KINDS: [u32; 6] = [0, 1, 12, 13, 0xffff_ffff, 4];

    #[test]
    fn lights_clear_fence() {
        // The fence (none, the head, the second item); the list's geometries (by the pattern of
        // types, a null element among them).
        Diff::new(entry!(0x00b9_e770, scene_light::clear_fence(Ptr<_>)))
            .own()
            .inputs(&[Input::Of(&[0, 1, 2]), Input::Of(&[0, 1, 2, 3, 4, 5])])
            .setup(|e, case| {
                world(e);
                let l = plain_light(e, 0, 0);
                let geometries: Vec<u32> = (0..6usize)
                    .map(|i| {
                        if i == 3 {
                            0
                        } else {
                            lit_geometry(e, KINDS[(i + case.word(1) as usize) % 6])
                        }
                    })
                    .collect();
                super::geometry_list(e, l, &geometries, 9);
                let head = e.mem.u32(l + 0xe0);
                let fence = match case.word(0) {
                    0 => 0,
                    1 => head,
                    _ => e.mem.u32(head),
                };
                e.mem.set_u32(l + 0x138, fence);
                vec![l]
            })
            .run()
            .assert_same();
    }

    #[test]
    fn lights_remove_tri_geom() {
        // The geometry's property type; an item given or not; the geometry's count (0: its release
        // deletes it); what the removal hands back (none, an object deleted by its release, one
        // kept).
        Diff::new(entry!(
            0x00ba_01a0,
            scene_light::remove_tri_geom(Ptr<_>, u32, u32)
        ))
        .own()
        .inputs(&[
            Input::Of(&KINDS),
            Input::Flag,
            Input::Of(&[0, 1]),
            Input::Of(&[0, 1, 2]),
        ])
        .setup(|e, case| {
            world(e);
            e.register(0x00b7_17a0, logged::<0x19>);
            e.register(0x00b9_efa0, out_writer::<0x1a>);
            e.register(0x00b9_fcc0, out_writer::<0x1b>);
            let l = plain_light(e, 0, 0);
            let g = lit_geometry(e, case.word(0));
            e.mem.set_u32(g + 4, case.word(2));
            let item = if case.word(1) != 0 {
                block(e, 0xc, &[0, 0, g])
            } else {
                0
            };
            let removed = match case.word(3) {
                0 => 0,
                1 => counted(e, 0x40, 1),
                _ => counted(e, 0x40, 4),
            };
            set_logged_answer(e, 0x1a, removed);
            set_logged_answer(e, 0x1b, removed);
            vec![l, g, item]
        })
        .run()
        .assert_same();
    }

    #[test]
    fn lights_cull_geometries() {
        // The `NiLight` culled or not; the callback set or not; the render flags' bit 0; the busy
        // test's answer; the types' pattern (+0x68 of 1, 5, 6, 0x11, 0, -1, 2, and none at +0xc0);
        // the geometries' +0xbc words.
        Diff::new(entry!(
            0x00b9_d020,
            scene_light::cull_geometries(Ptr<_>, u32) -> bool
        ))
        .own()
        .family(&engine_typed::culling::CULLING)
        .inputs(&[
            Input::Flag,
            Input::Flag,
            Input::Flag,
            Input::Flag,
            Input::Of(&[0, 1, 2, 3, 4, 5, 6, 7]),
            Input::Flag,
        ])
        .max_cases(300)
        .setup(|e, case| {
            const TYPES: [u32; 7] = [1, 5, 6, 0x11, 0, 0xffff_ffff, 2];
            world(e);
            let ni = object(e, 0x100, 0, 0, OTHER_RTTI);
            e.mem.set_u32(ni + 0x30, case.word(0));
            let l = plain_light(e, ni, 0);
            e.register(0x00c0_0000, logged::<0x1c>);
            let callback = if case.word(1) != 0 { 0x00c0_0000 } else { 0 };
            e.mem.set_u32(0x011f_91ec, callback);
            e.mem
                .set_u8(0x011a_d82c, if case.word(2) != 0 { 0x81 } else { 0x7e });
            let renderer = block(e, 0x10, &[0, 0, 0x0bad_0000]);
            e.mem.set_u32(0x011f_4748, renderer);
            e.register(0x00aa_0ff0, logged::<0x1d>);
            set_logged_answer(e, 0x1d, case.word(3));
            e.register(0x00b7_a870, logged::<0x1e>);
            e.register(0x00bb_d880, logged::<0x1f>);
            e.register(VT_FN + 0x24, logged::<0x21>);
            e.register(VT_FN + 0x44, logged::<0x22>);
            let mut geometries: Vec<u32> = (0..7usize)
                .map(|i| {
                    let g = block(e, 0x100, &[]);
                    // The cull's flags (+0x30): the application culls every third.
                    e.mem.set_u32(g + 0x30, u32::from(i % 3 == 2));
                    e.mem.set_u32(g + 0xbc, (i as u32 ^ case.word(5)) & 1);
                    let k = (i + case.word(4) as usize) % 8;
                    if k < 7 {
                        let p = block(e, 0x80, &[]);
                        e.mem.set_u32(p + 0x68, TYPES[k]);
                        let first = object(e, 0x20, 0, 0, OTHER_RTTI);
                        let second = object(e, 0x20, 0, 0, OTHER_RTTI);
                        e.mem.set_u32(p + 0x34, first);
                        e.mem.set_u32(p + 0x30, second);
                        e.mem.set_u32(g + 0xc0, p);
                    }
                    g
                })
                .collect();
            geometries.insert(2, 0);
            super::geometry_list(e, l, &geometries, 99);
            let culler = object(e, 0x40, 0, 0, OTHER_RTTI);
            vec![l, culler]
        })
        .run()
        .assert_same();
    }

    #[test]
    fn lights_rtti_bodies_answer_their_records() {
        // The exe's `GetRTTI` bodies the light's kind tests answer in place answer the same records
        // through their translations.
        let mut e = Engine::new();
        world(&mut e);
        for (body, record) in [
            (POINT_BODY, POINT_RTTI),
            (AMBIENT_BODY, AMBIENT_RTTI),
            (DIRECTIONAL_BODY, RADIUS_FREE_RTTI),
        ] {
            let o = object(&mut e, 0x40, 0, 0, 0);
            let vt = e.mem.u32(o);
            e.mem.set_u32(vt + 8, body);
            assert_eq!(e.vcall(o, 8, &[]).u32(), record, "{body:08x}");
            assert_eq!(e.mem.u32(vt + VT_SLOTS + 8), 0);
        }
    }

    /// The light's frustum functions (`lights::scene_frustum`). Cameras carry a plane-set word
    /// (+0x40) that the culler's `SetFrustum` stand-in turns into one of the case's plane sets
    /// (everything inside, nothing inside, every point on a plane, a box, a half-space that edges
    /// cross, one whose crossings another plane rejects, the case's floats); their rotation,
    /// position and frustum come from the case's floats, so that the volume test takes each of
    /// its ways out. Floats run under both control words.
    mod frustum {
        use super::super::{ret_f, THIRD};
        use super::*;
        use engine_typed::lights::scene_frustum;

        /// The plane sets (0x80 bytes each: six planes, then the mask) and the unit cross's
        /// answers (three points) and its call count.
        const PLANE_SETS: u32 = 0x0300_8000;
        const CROSSES: u32 = 0x0300_9000;
        /// The cube faces' plane sets, four bits a face.
        const FACE_KINDS: u32 = CROSSES + 0x80;
        const KIND: u32 = 0x40;
        const QUARTER: u32 = 0x3e80_0000;
        const NEG_ZERO: u32 = 0x8000_0000;

        /// `NiCullingProcess::SetFrustum`'s stand-in: logs the culler and the frustum, copies the
        /// plane set its camera's +0x40 names to the culler's +0x2c.
        fn set_frustum(e: &mut Engine, a: &[u32]) -> Ret {
            log(e, 0x30, a[0]);
            log(e, 0x30, a[1]);
            let camera = e.mem.u32(a[0] + 0xc);
            let set = PLANE_SETS + 0x80 * (e.mem.u32(camera + KIND) & 7);
            for i in 0..0x19 {
                let w = e.mem.u32(set + 4 * i);
                e.mem.set_u32(a[0] + 0x2c + 4 * i, w);
            }
            ret(0)
        }

        /// `BSCubeMapCamera::SetupTransformation`'s stand-in: logs, and gives the local camera
        /// the face's plane set, a rotation and a far corner spread from the face.
        fn cube_setup(e: &mut Engine, a: &[u32]) -> Ret {
            log(e, 0x31, a[0]);
            log(e, 0x31, a[1]);
            let face = a[1] & 0xffff;
            let kinds = e.mem.u32(FACE_KINDS);
            e.mem.set_u32(a[0] + KIND, (kinds >> (4 * face)) & 7);
            let s = if face & 1 != 0 { MINUS_ONE } else { ONE };
            words(e, a[0] + 0x68, &[s, 0, 0, 0, ONE, 0, 0, 0, s]);
            words(
                e,
                a[0] + 0xdc,
                &[MINUS_ONE, ONE, ONE, MINUS_ONE, ONE, TEN, 0],
            );
            // The world translation `NiAVObject::Update` would make of the local one.
            for i in 0..3 {
                let w = e.mem.u32(a[0] + 0x58 + 4 * i);
                e.mem.set_u32(a[0] + 0x8c + 4 * i, w);
            }
            ret(0)
        }

        /// `NiPoint3::UnitCross`'s stand-in: logs its words, writes the next of the case's three
        /// answers to `out` and answers `out`.
        fn unit_cross(e: &mut Engine, a: &[u32]) -> Ret {
            for w in a {
                log(e, 0x32, *w);
            }
            let n = e.mem.u32(CROSSES + 0x40);
            e.mem.set_u32(CROSSES + 0x40, n + 1);
            for i in 0..3 {
                let w = e.mem.u32(CROSSES + 0xc * (n % 3) + 4 * i);
                e.mem.set_u32(a[1] + 4 * i, w);
            }
            ret(a[1])
        }

        /// A stand-in that logs its words under `TAG` and answers its first word.
        fn this_back<const TAG: u32>(e: &mut Engine, a: &[u32]) -> Ret {
            for w in a {
                log(e, TAG, *w);
            }
            ret(a.first().copied().unwrap_or(0))
        }

        /// The renderer's shadow map setter's stand-in: logs and puts the map in the light.
        fn set_map(e: &mut Engine, a: &[u32]) -> Ret {
            log(e, 0x33, a[0]);
            log(e, 0x33, a[1]);
            e.mem.set_u32(a[0] + 0x10c, a[1]);
            ret(0)
        }

        /// `NiPointer` assignment's stand-in: logs and stores.
        fn assign(e: &mut Engine, a: &[u32]) -> Ret {
            log(e, 0x34, a[0]);
            log(e, 0x34, a[1]);
            e.mem.set_u32(a[0], a[1]);
            ret(a[0])
        }

        /// The point maths' stand-ins (their families' own tests cover them): `Unitize` logs,
        /// `operator*` writes the point times the scale to `out`, `Length` answers the root of the
        /// squared length, `operator+=` adds, all in `f32`.
        fn normalize(e: &mut Engine, a: &[u32]) -> Ret {
            log(e, 0x29, a[0]);
            ret(0)
        }

        fn times(e: &mut Engine, a: &[u32]) -> Ret {
            for w in a {
                log(e, 0x2a, *w);
            }
            for i in 0..3 {
                let v = e.mem.f32(a[0] + 4 * i) * f32::from_bits(a[2]);
                e.mem.set_f32(a[1] + 4 * i, v);
            }
            ret(a[1])
        }

        fn length(e: &mut Engine, a: &[u32]) -> Ret {
            log(e, 0x2b, a[0]);
            let [x, y, z] = [0, 4, 8].map(|i| e.mem.f32(a[0] + i));
            ret_f(f64::from((x * x + y * y + z * z).sqrt()))
        }

        fn add_to(e: &mut Engine, a: &[u32]) -> Ret {
            log(e, 0x2c, a[0]);
            log(e, 0x2c, a[1]);
            for i in 0..3 {
                let v = e.mem.f32(a[0] + 4 * i) + e.mem.f32(a[1] + 4 * i);
                e.mem.set_f32(a[0] + 4 * i, v);
            }
            ret(a[0])
        }

        /// The data these functions read and their callees' stand-ins.
        fn fworld(e: &mut Engine) {
            world(e);
            for page in [
                0x0101_6000,
                0x0101_8000,
                0x0102_0000,
                0x0103_1000,
                0x0103_2000,
                0x0106_b000,
                0x0106_c000,
                0x0108_2000,
                0x010b_7000,
                0x011a_9000,
                0x0120_0000,
                0x0120_3000,
                PLANE_SETS,
                CROSSES,
            ] {
                e.map(page, 0x1000);
            }
            let double = |e: &mut Engine, at: u32, v: f64| {
                let b = v.to_bits();
                words(e, at, &[b as u32, (b >> 32) as u32]);
            };
            double(e, 0x0108_2cb0, 0.05);
            double(e, 0x0103_1148, 0.01);
            double(e, 0x010b_7ea0, -0.01);
            double(e, 0x0101_6ff0, 1.5);
            double(e, 0x0106_c948, 150.0);
            double(e, 0x0102_0758, 10.0);
            double(e, 0x0106_b178, 400.0);
            double(e, 0x0101_8a90, 0.2);
            e.mem.set_u32(0x0103_2adc, 0x4380_0000);
            words(e, 0x011a_946c, &[0, 0, ONE]);
            words(e, 0x011a_9bd0, &[HALF, QUARTER, 0, ONE]);
            e.mem.set_u32(0x0101_62c0, 0x7f80_0001);
            e.mem.set_u32(0x0120_31e4, 0x0abc_0010);
            e.register(0x00a6_9400, this_back::<0x35>);
            e.register(0x00a6_94a0, set_frustum);
            e.register(0x00bd_8290, this_back::<0x36>);
            e.register(0x00bd_7200, cube_setup);
            e.register(0x00bd_74a0, logged::<0x37>);
            e.register(0x00a5_9c60, logged::<0x38>);
            e.register(0x00b4_f5c0, logged::<0x39>);
            e.register(0x00a5_9d30, logged::<0x3a>);
            e.register(0x00b6_d4c0, release_map);
            e.register(0x00b6_d3f0, logged::<0x3b>);
            e.register(0x00b5_aee0, set_map);
            e.register(0x00ba_30f0, logged::<0x3c>);
            e.register(0x00ba_3390, logged::<0x3d>);
            e.register(0x0053_d1a0, unit_cross);
            e.register(0x00a5_bdd0, logged::<0x3e>);
            e.register(0x005b_f7b0, logged::<0x21>);
            e.register(0x00b5_40f0, logged::<0x22>);
            e.register(0x0066_b0d0, assign);
            e.register(0x00aa_13e0, logged::<0x23>);
            e.register(0x00a7_12f0, this_back::<0x24>);
            e.register(0x00a7_01b0, logged::<0x25>);
            e.register(0x00a6_faf0, logged::<0x26>);
            e.register(0x0071_2e60, logged::<0x27>);
            e.register(0x00a7_0080, logged::<0x28>);
            e.register(0x004a_0c10, normalize);
            e.register(0x0045_bb20, times);
            e.register(0x0045_7990, length);
            e.register(0x0063_c8a0, add_to);
            e.register(0x00a7_4e10, logged::<0x2d>);
        }

        /// The plane sets from the case's floats at `f`: everything inside (0), nothing (1), on a
        /// plane (2), a box of half size `f` (3), the half-space x > 0 with the rest open (4), x >
        /// 0 with y below -1 shut (5), the case's floats (6, 7).
        fn plane_sets(e: &mut Engine, case: &Case, f: usize) {
            let open = [0, 0, 0, MINUS_ONE];
            let s = bits(case, f) ^ NEG_ZERO;
            let sets: [[[u32; 4]; 6]; 8] = [
                [open; 6],
                [[0, 0, 0, ONE]; 6],
                [[0, 0, 0, 0]; 6],
                [
                    [ONE, 0, 0, s],
                    [MINUS_ONE, 0, 0, s],
                    [0, ONE, 0, s],
                    [0, MINUS_ONE, 0, s],
                    [0, 0, ONE, s],
                    [0, 0, MINUS_ONE, s],
                ],
                [[ONE, 0, 0, 0], open, open, open, open, open],
                [
                    [ONE, 0, 0, 0],
                    [0, MINUS_ONE, 0, ONE],
                    open,
                    open,
                    open,
                    open,
                ],
                super::super::planes(case, f + 1),
                [
                    [bits(case, f + 2), ONE, 0, bits(case, f + 3)],
                    [0, bits(case, f + 1), ONE, HALF],
                    open,
                    [MINUS_ONE, HALF, bits(case, f + 2), 0],
                    open,
                    [THIRD, THIRD, THIRD, MINUS_ONE],
                ],
            ];
            for (k, set) in (0u32..).zip(sets) {
                for (i, p) in (0u32..).zip(set) {
                    words(e, PLANE_SETS + 0x80 * k + 0x10 * i, &p);
                }
                e.mem.set_u32(PLANE_SETS + 0x80 * k + 0x60, 0x3f);
            }
        }

        /// A camera (0x200 bytes) with plane set `kind`, at (`x`, `y`, 0.5) with the rotation
        /// `turn` (0 identity, 1 a half turn about z, 2 from the case's floats at `f`), the far
        /// distance `far` and the frustum's sides `side` and `up`.
        #[allow(clippy::too_many_arguments)]
        fn camera(
            e: &mut Engine,
            case: &Case,
            kind: u32,
            turn: u32,
            f: usize,
            at: [u32; 2],
            far: u32,
            side: u32,
        ) -> u32 {
            let c = block(e, 0x200, &[]);
            e.mem.set_u32(c + KIND, kind);
            let rotation = match turn {
                0 => [ONE, 0, 0, 0, ONE, 0, 0, 0, ONE],
                1 => [MINUS_ONE, 0, 0, 0, MINUS_ONE, 0, 0, 0, ONE],
                _ => [
                    bits(case, f),
                    HALF,
                    0,
                    bits(case, f + 1),
                    ONE,
                    THIRD,
                    0,
                    MINUS_ONE,
                    bits(case, f + 2),
                ],
            };
            words(e, c + 0x68, &rotation);
            words(e, c + 0x8c, &[at[0], at[1], HALF]);
            words(
                e,
                c + 0xdc,
                &[MINUS_ONE, side, HALF, MINUS_ONE, ONE, far, 0],
            );
            c
        }

        trait Frustum {
            fn frustum(self) -> Diff;
        }

        impl Frustum for Diff {
            fn frustum(self) -> Diff {
                self.own()
                    .ignore_calls(&[0x00a6_9940, 0x00a6_93e0])
                    .control_words(&BOTH)
                    .allow_nan_payloads()
            }
        }

        /// The volume test's inputs: the two cameras' plane sets and rotations; floats for the
        /// rotation, the box, the planes, the cameras' places, the far distance and the side.
        fn volume_inputs() -> Vec<Input> {
            vec![
                Input::Of(&[0, 1, 2, 3, 4, 5, 6, 7]),
                Input::Of(&[3, 4, 5, 0, 1, 2, 6, 7]),
                Input::Of(&[0, 1, 2]),
                Input::Of(&[0, 2, 1]),
                Input::F32Swept(HALF),
                Input::F32Swept(TWO),
                Input::F32Swept(THIRD),
                Input::F32Swept(MINUS_ONE),
                Input::F32Swept(MINUS_ONE),
                Input::F32Swept(TWO),
                Input::F32Swept(0xc0a0_0000),
                Input::F32Swept(QUARTER),
                Input::F32,
                Input::F32Swept(HALF),
            ]
        }

        /// The two cameras of [`volume_inputs`].
        fn volume_cameras(e: &mut Engine, case: &Case) -> (u32, u32) {
            plane_sets(e, case, 5);
            let far = bits(case, 12);
            let first = camera(
                e,
                case,
                case.word(0),
                case.word(2),
                4,
                [bits(case, 9), bits(case, 11)],
                TEN,
                bits(case, 13),
            );
            let second = camera(
                e,
                case,
                case.word(1),
                case.word(3),
                5,
                [bits(case, 10), QUARTER],
                far,
                ONE,
            );
            (first, second)
        }

        #[test]
        fn lights_volumes_meet() {
            Diff::new(entry!(
                0x00b9_c020,
                scene_frustum::volumes_meet(Ptr<_>, Ptr<_>, Ptr<_>) -> bool
            ))
            .frustum()
            .goes_by_address(&[0x00a6_94a0])
            .inputs(&volume_inputs())
            .max_cases(900)
            .float_sweep(200)
            .setup(|e, case| {
                fworld(e);
                let (first, second) = volume_cameras(e, case);
                vec![0x0abc_0000, first, second]
            })
            .run()
            .assert_same();
        }

        #[test]
        fn lights_cube_face_meets() {
            // The position (floats), the source camera's plane set, the faces' plane sets (a
            // pattern of six), the face.
            Diff::new(entry!(
                0x00b9_de90,
                scene_frustum::cube_face_meets(Ptr<_>, u32, u32, u32, u32, Ptr<_>, u16) -> bool
            ))
            .frustum()
            .goes_by_address(&[0x00a6_94a0, 0x00a5_9c60])
            .inputs(&[
                Input::F32Swept(MINUS_ONE),
                Input::F32Swept(HALF),
                Input::F32,
                Input::Of(&[0, 3, 4, 1, 5, 7]),
                Input::Of(&[0x0054_3210, 0x0011_1111, 0x0076_5432, 0]),
                Input::Of(&[0, 1, 2, 3, 4, 5, 0x1_0002]),
                Input::F32Swept(TWO),
                Input::F32Swept(THIRD),
                Input::F32Swept(QUARTER),
                Input::F32Swept(MINUS_ONE),
            ])
            .max_cases(500)
            .float_sweep(120)
            .setup(|e, case| {
                fworld(e);
                plane_sets(e, case, 6);
                e.mem.set_u32(FACE_KINDS, case.word(4));
                let source = block(e, 0x40, &[]);
                let c = camera(e, case, case.word(3), 0, 6, [bits(case, 0), 0], TEN, ONE);
                e.mem.set_u32(source + 0xc, c);
                let tls = e.tls();
                e.mem.set_u32(tls + 0x2b4, 0x1234);
                vec![
                    0x0abc_0000,
                    bits(case, 0),
                    bits(case, 1),
                    bits(case, 2),
                    0x5555,
                    source,
                    case.word(5),
                ]
            })
            .run()
            .assert_same();
        }

        /// A shadow scene light for `TestFrustumCull` from the case's words at `w`: the shadow
        /// camera (none, or one with plane set `w + 1`), `bCastShadow`, `bCubeShadowMap`, the old
        /// cull word, dimmer and shadow dimmer, the target, the elapsed time, the shadow map
        /// (none, kept, cleared by the renderer's release), the shadow scene (none, with property
        /// 2 or without), a geometry list of lit geometries.
        fn cull_light(e: &mut Engine, case: &Case, w: usize, ni: u32) -> u32 {
            let l = plain_light(e, ni, case.word(w + 2) as u8);
            if case.word(w) != 0 {
                let c = camera(e, case, case.word(w + 1), 0, 0, [0, HALF], TEN, ONE);
                e.mem.set_u32(l + 0x140, c);
            }
            e.mem.set_u8(l + 0xed, case.word(w + 3) as u8);
            e.mem.set_u16(l + 0x110, case.word(w + 4) as u16);
            words(
                e,
                l + 0xd0,
                &[case.word(w + 5), case.word(w + 6), case.word(w + 7)],
            );
            words(e, l + 0xdc, &[QUARTER]);
            let map = match case.word(w + 8) {
                0 => 0,
                _ => counted(e, 0x40, 1),
            };
            e.mem.set_u32(l + 0x10c, map);
            if case.word(w + 8) == 2 {
                e.mem.set_u32(LOG + 0x3e00, l + 0x10c);
            }
            if case.word(w + 9) != 0 {
                let scene = block(e, 0x100, &[]);
                e.mem.set_u32(l + 0x128, scene);
                if case.word(w + 9) == 1 {
                    let property = block(e, 0x40, &[]);
                    e.mem.set_u32(property + 0x3c, HALF);
                    set_logged_answer(e, 0x3a, property);
                }
            }
            let geometries: Vec<u32> = KINDS.iter().map(|&k| lit_geometry(e, k)).collect();
            super::super::geometry_list(e, l, &geometries, 9);
            l
        }

        #[test]
        fn lights_test_frustum_cull() {
            // The culler's planes (the case's floats and a box), their mask, the `NiLight`'s
            // radius; the light (see `cull_light`); the render level and flags; the shader
            // accumulator's object (none, scale 2); the fade distances and the fade time.
            let mut inputs = vec![
                Input::Of(&[0, 1]),
                Input::Of(&[0x3f, 0x15, 0x2a, 0]),
                Input::Of(&[TEN, HALF, 0, NAN, 0x4248_0000]),
                // cull_light from word 3.
                Input::Of(&[0, 1]),
                Input::Of(&[0, 1, 3, 4]),
                Input::Of(&[1, 0]),
                Input::Of(&[1, 0]),
                Input::Of(&[0, 0xff, 3]),
                Input::Of(&[ONE, 0, HALF, NAN]),
                Input::Of(&[0, ONE, HALF]),
                Input::Of(&[ONE, 0]),
                Input::Of(&[0, 1, 2]),
                Input::Of(&[1, 0, 2]),
                // The globals.
                Input::Of(&[3, 0, 0xffff_ffff]),
                Input::Flag,
                Input::Flag,
                Input::Of(&[0, 1, 2, 3]),
            ];
            inputs.extend([
                Input::F32Swept(HALF),
                Input::F32Swept(MINUS_ONE),
                Input::F32Swept(TWO),
                Input::F32Swept(0x4120_0000),
                Input::F32Swept(0x4248_0000),
                Input::F32Swept(QUARTER),
                Input::F32Swept(ONE),
                Input::F32Swept(MINUS_ONE),
            ]);
            Diff::new(entry!(
                0x00b9_e970,
                scene_frustum::test_frustum_cull(Ptr<_>, u32)
            ))
            .frustum()
            .ignore_calls(&[0x00ec_6040])
            .goes_by_address(&[0x00a6_94a0, 0x00a5_9c60, 0x00b4_f5c0, 0x00a5_9d30])
            .inputs(&inputs)
            .max_cases(1500)
            .float_sweep(200)
            .setup(|e, case| {
                fworld(e);
                plane_sets(e, case, 17);
                e.mem.set_u32(FACE_KINDS, 0x0034_5012);
                let f = |i: usize| bits(case, 17 + i);
                let ni = ni_light_of(e, case, 6, 18, case.word(2));
                words(e, ni + 0x8c, &[f(0), f(1), HALF]);
                let culler = block(e, 0x90, &[]);
                let planes = if case.word(0) == 0 {
                    super::super::planes(case, 17)
                } else {
                    let s = f(2) ^ NEG_ZERO;
                    [
                        [ONE, 0, 0, s],
                        [MINUS_ONE, 0, 0, s],
                        [0, ONE, 0, s],
                        [0, MINUS_ONE, 0, s],
                        [0, 0, ONE, s],
                        [0, 0, MINUS_ONE, s],
                    ]
                };
                for (i, p) in (0u32..).zip(planes) {
                    words(e, culler + 0x2c + 0x10 * i, &p);
                }
                e.mem.set_u32(culler + 0x8c, case.word(1));
                let cc = camera(e, case, 3, 0, 18, [f(5), f(6)], TEN, ONE);
                e.mem.set_u32(culler + 0xc, cc);
                let l = cull_light(e, case, 3, ni);
                e.mem.set_u32(0x011f_91c0, case.word(13));
                e.mem
                    .set_u8(0x011f_91d8, if case.word(14) != 0 { 0x10 } else { 0xef });
                let accumulator = block(e, 0x40, &[]);
                if case.word(15) != 0 {
                    let object = block(e, 0x200, &[]);
                    e.mem.set_u32(object + 0x110, TWO);
                    e.mem.set_u32(accumulator + 8, object);
                }
                set_logged_answer(e, 0x39, accumulator);
                let (near, far) = match case.word(16) {
                    0 => (f(3), f(4)),
                    1 => (f(3), 0),
                    2 => (0x4248_0000, f(4)),
                    _ => (NAN, f(7)),
                };
                words(e, 0x011f_9444, &[near, far, f(3), far]);
                e.mem.set_u32(0x011f_9410, f(5));
                e.mem.set_u32(0x011a_d834, HALF);
                let tls = e.tls();
                e.mem.set_u32(tls + 0x2b4, 0x1234);
                vec![l, culler]
            })
            .run()
            .assert_same();
        }

        #[test]
        fn lights_setup_frustum_depth_map() {
            // bCastShadow, bShadowSpotlight, the render level and flags; the light's map (none,
            // kept); the cull word; the shadow dimmer (above, just above, just below 0.05, NaN);
            // the accumulator (kept, none); the renderer's free map (one, none); the scene's bound;
            // the extents' extra data; the shadow camera (a new one, kept); the script test; the
            // scene's scale; the unit cross small first; the depth pass; floats.
            Diff::new(entry!(
                0x00b9_dfc0,
                scene_frustum::setup_frustum_depth_map(Ptr<_>, u32, u32)
            ))
            .frustum()
            .ignore_calls(&[0x00a7_1b70, 0x0040_4010, 0x0040_ebd0])
            .goes_by_address(&[
                0x00a5_9c60,
                0x004a_0c10,
                0x0045_bb20,
                0x0045_7990,
                0x0063_c8a0,
                0x00a7_4e10,
                // The map setter (the scene node family's): its stand-in stores the map.
                0x00b5_aee0,
            ])
            .inputs(&[
                Input::Flag,
                Input::Flag,
                Input::Of(&[3, 0]),
                Input::Flag,
                Input::Of(&[1, 0]),
                Input::Of(&[0, 0xff]),
                Input::Of(&[ONE, 0x3d4c_cccd, 0x3d4c_cccc, NAN]),
                Input::Of(&[1, 0]),
                Input::Flag,
                Input::Of(&[1, 0]),
                Input::Of(&[1, 0]),
                Input::Of(&[0, 1]),
                Input::Of(&[1, 0]),
                Input::Of(&[0, 1]),
                Input::Of(&[0, 1]),
                Input::Of(&[0, 1]),
                Input::F32Swept(HALF),
                Input::F32Swept(MINUS_ONE),
                Input::F32Swept(TWO),
                Input::F32Swept(0x4120_0000),
                Input::F32Swept(QUARTER),
                Input::F32,
                Input::F32Swept(THIRD),
            ])
            .max_cases(1200)
            .float_sweep(200)
            .setup(|e, case| {
                fworld(e);
                let f = |i: usize| bits(case, 16 + i);
                let ni = ni_light_of(e, case, 6, 16, f(3));
                e.mem.set_u32(ni + 0x30, 0x10);
                words(e, ni + 0x68, &[ONE, 0, 0, 0, ONE, 0, 0, 0, ONE]);
                let l = plain_light(e, ni, case.word(0) as u8);
                e.mem.set_u8(l + 0x118, case.word(1) as u8);
                e.mem.set_u32(0x011f_91c0, case.word(2));
                e.mem
                    .set_u8(0x011f_91d8, if case.word(3) != 0 { 0x10 } else { 0 });
                if case.word(4) != 0 {
                    let map = counted(e, 0x40, 2);
                    e.mem.set_u32(l + 0x10c, map);
                }
                let free_map = if case.word(8) != 0 {
                    counted(e, 0x40, 1)
                } else {
                    0
                };
                set_logged_answer(e, 0x3b, free_map);
                e.mem.set_u16(l + 0x110, case.word(5) as u16);
                e.mem.set_u32(l + 0xd4, case.word(6));
                let accumulator = counted(e, 0x200, 2);
                if case.word(7) == 1 {
                    e.mem.set_u32(l + 0x20c, accumulator);
                }
                set_logged_answer(e, 0x22, accumulator);
                let scene = block(e, 0x100, &[]);
                words(e, scene + 0x8c, &[f(5), f(4), f(0)]);
                e.mem.set_u32(
                    scene + 0x98,
                    if case.word(13) != 0 { TWO } else { 0x7f80_0001 },
                );
                if case.word(9) != 0 {
                    let bound = block(e, 0x10, &[0, 0, 0, f(6)]);
                    e.mem.set_u32(scene + 0x20, bound);
                }
                e.mem.set_u32(l + 0x128, scene);
                if case.word(10) != 0 {
                    let extra = block(e, 0x40, &[]);
                    words(e, extra + 0xc, &[f(1), HALF, f(2)]);
                    words(e, extra + 0x18, &[f(2), f(0), f(4)]);
                    set_logged_answer(e, 0x3e, extra);
                }
                // Without extra data the camera's set-up would read it: the script test says no.
                let script = if case.word(10) != 0 { case.word(12) } else { 0 };
                set_logged_answer(e, 0x21, script);
                let new_camera = block(e, 0x200, &[]);
                words(
                    e,
                    new_camera + 0x68,
                    &[ONE, 0, 0, 0, ONE, 0, 0, 0, ONE, f(0), f(1), HALF],
                );
                words(
                    e,
                    new_camera + 0xdc,
                    &[MINUS_ONE, ONE, ONE, MINUS_ONE, ONE, f(3), 1],
                );
                if case.word(11) == 0 {
                    set_logged_answer(e, 0x23, new_camera);
                } else {
                    e.mem.set_u32(l + 0x140, new_camera);
                }
                // The unit cross's answers: small first (the fallback up taken) or not.
                let small = if case.word(14) != 0 {
                    [0, 0x3c00_0000, 0]
                } else {
                    [f(0), ONE, 0]
                };
                words(e, CROSSES, &small);
                words(e, CROSSES + 0xc, &[0, ONE, 0, HALF, 0, ONE]);
                words(e, 0x011a_9484, &[f(6), f(1), MINUS_ONE]);
                e.mem.set_u32(0x011f_91b0, case.word(15));
                vec![l, 0x7777, case.word(13) + 2]
            })
            .run()
            .assert_same();
        }

        #[test]
        fn lights_render_frustum_depth_map() {
            // The holder's old accumulator (none, the same, another); the accumulator's +0x1a0
            // (none, one); the depth pass flag; bShowDebugTexture; floats for the light's place and
            // the camera's rotation.
            Diff::new(entry!(
                0x00b9_f780,
                scene_frustum::render_frustum_depth_map(Ptr<_>, u32, u32)
            ))
            .frustum()
            .goes_by_address(&[0x00b6_c0d0, 0x004e_9bb0, 0x00ee_6de0])
            .inputs(&[
                Input::Of(&[0, 1, 2]),
                Input::Flag,
                Input::Flag,
                Input::Flag,
                Input::Of(&[0, 1, 0x7f]),
                Input::F32Swept(HALF),
                Input::F32Swept(MINUS_ONE),
                Input::F32,
                Input::F32Swept(THIRD),
                Input::F32Swept(TWO),
            ])
            .max_cases(300)
            .float_sweep(160)
            .setup(|e, case| {
                fworld(e);
                for (addr, f) in [
                    (
                        0x00ba_3130u32,
                        logged::<0x40> as fn(&mut Engine, &[u32]) -> Ret,
                    ),
                    (0x00b6_b260, logged::<0x41>),
                    (0x00b6_b8d0, logged::<0x42>),
                    (0x00b6_c0d0, logged::<0x43>),
                    (0x004e_9bb0, logged::<0x44>),
                    (0x00ee_6de0, logged::<0x45>),
                    (0x00b6_b790, logged::<0x46>),
                    (0x00ba_37a0, logged::<0x47>),
                    (0x004e_ba20, logged::<0x48>),
                    (0x00ba_3cb0, logged::<0x49>),
                    (0x00b9_7550, logged::<0x4a>),
                    (0x004e_ba90, logged::<0x4b>),
                    (0x00ba_3a00, logged::<0x4c>),
                    (0x004b_c320, logged::<0x4d>),
                    (0x00b5_5560, logged::<0x4e>),
                    (0x00bd_22c0, logged::<0x4f>),
                    (VT_FN + 0xb4, logged::<0x50>),
                    (VT_FN + 0xac, logged::<0x51>),
                ] {
                    e.register(addr, f);
                }
                let renderer = object(e, 0x40, 0, 0, OTHER_RTTI);
                e.mem.set_u32(0x011f_4748, renderer);
                let state = block(e, 0xa00, &[]);
                for i in 0..0x20 {
                    e.mem
                        .set_u32(state + 0x980 + 4 * i, 0x3f00_0000 + 0x1_0000 * i);
                }
                e.mem.set_u32(0x011f_9508, state);
                let inner = block(e, 0x60, &[]);
                e.mem.set_u32(inner + 0x44, 0x0abc_0044);
                let manager = block(e, 0x20, &[0, 0, inner]);
                e.mem.set_u32(0x011f_91ac, manager);
                e.mem.set_u32(0x011f_91a8, 0x0abc_0000);
                set_logged_answer(e, 0x3b, 0x0abc_0300);
                set_logged_answer(e, 0x41, 0x0abc_0100);
                let ni = ni_light_of(e, case, 6, 5, TEN);
                words(e, ni + 0x8c, &[bits(case, 5), bits(case, 6), bits(case, 7)]);
                let l = plain_light(e, ni, 1);
                let camera = block(e, 0x200, &[]);
                words(
                    e,
                    camera + 0x68,
                    &[
                        bits(case, 8),
                        HALF,
                        0,
                        bits(case, 9),
                        ONE,
                        THIRD,
                        NEG_ZERO,
                        MINUS_ONE,
                        bits(case, 7),
                    ],
                );
                e.mem.set_u32(l + 0x140, camera);
                let accumulator = counted(e, 0x200, 3);
                e.mem.set_u32(l + 0x20c, accumulator);
                if case.word(1) != 0 {
                    let held = counted(e, 0x40, 1);
                    e.mem.set_u32(accumulator + 0x1a0, held);
                }
                let map = counted(e, 0x40, 1);
                e.mem.set_u32(l + 0x10c, map);
                let holder = block(e, 0x100, &[]);
                let old = match case.word(0) {
                    0 => 0,
                    1 => accumulator,
                    _ => counted(e, 0x40, 1),
                };
                e.mem.set_u32(holder + 0xc4, old);
                e.mem.set_u32(0x011f_91b0, case.word(2));
                e.mem.set_u8(l + 0x124, case.word(3) as u8);
                e.mem.set_u8(l + 0x208, 1);
                vec![l, holder, case.word(4)]
            })
            .run()
            .assert_same();
        }
    }
}
