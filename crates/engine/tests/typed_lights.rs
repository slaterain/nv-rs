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
