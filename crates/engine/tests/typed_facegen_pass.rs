//! The differential test of a head's morph pass (`engine_typed::facegen::apply`) against the
//! translations: `BSFaceGenNiNode`'s pass (`00663050`), its four appliers, the morph data's apply
//! (`00662030`) and table getters, the animation data's weight getters and the pair split.
//!
//! A node (its vtable's +0x100 the real `GetAnimationData`, the animation data's pointer at +0xac)
//! over an animation data whose weight getters are the real ones (+0x94, +0xa4, +0xac; the
//! modifier array at +0xa8) and whose +0xd4 and +0xb4 are doubles; its child geometries with the
//! morph data's tables of differential, statistical and other morphs (an "other" `ApplyMorph`
//! double, so the virtual call is compared too). Doubles answer what the pass asks of the scene:
//! children, the geometry's data lock, settings, positions, the menu mode. The vertex loops are
//! compared in `typed_facegen.rs`; here their per-vertex callees are left out the same way.
//!
//! Floats: the distance tests' sums run at the thread's precision (both control words run); NaN
//! payloads may differ (`allow_nan_payloads`).

use engine as _;
use engine_core::prelude::*;
use engine_core::typed::diff::{Case, Diff, Input};
use engine_typed::facegen::{self, apply};

const TEST: u32 = 0x0300_0000;
const LOCKED_ARRAY: u32 = TEST;
const LOCKED_STRIDE: u32 = TEST + 4;
const EXTRA: u32 = TEST + 8;
const MARKS: u32 = TEST + 0xc;
/// What the doubles answer: the children (count, then up to four), the far and near settings,
/// the quartering test, the menu mode, the animation data's +0xd4, the distance's position.
const CHILDREN: u32 = TEST + 0x20;
const FAR: u32 = TEST + 0x40;
const NEAR: u32 = TEST + 0x44;
const QUARTERED: u32 = TEST + 0x48;
const MENU_MODE: u32 = TEST + 0x4c;
const ANIMATED: u32 = TEST + 0x50;
const LOCKS: u32 = TEST + 0x54;
const RESETS: u32 = TEST + 0x58;
const OTHER_APPLIES: u32 = TEST + 0x5c;
const PARENT_ANSWER: u32 = TEST + 0x60;
/// Base copies made (each asks the morph extra data once): how the skip test tells a reshape made.
const BASE_COPIES: u32 = TEST + 0x64;
const NODE_POSITION: u32 = TEST + 0x70;
const CAMERA_POSITION: u32 = TEST + 0x80;
/// Vtables: the node's, the animation data's, the child's, the morphs' (three kinds), the
/// parent's.
const NODE_VTABLE: u32 = TEST + 0x400;
const DATA_VTABLE: u32 = TEST + 0x600;
const CHILD_VTABLE: u32 = TEST + 0x800;
const DIFFERENTIAL_VTABLE: u32 = TEST + 0x900;
const STATISTICAL_VTABLE: u32 = TEST + 0x920;
const OTHER_VTABLE: u32 = TEST + 0x940;
const PARENT_VTABLE: u32 = TEST + 0x960;
/// The doubles' addresses.
const D_ANIMATED: u32 = TEST + 0xa00;
const D_RESET: u32 = TEST + 0xa04;
const D_GEOMETRY_OF: u32 = TEST + 0xa08;
const D_OTHER_APPLY: u32 = TEST + 0xa0c;
const D_PARENT_ANSWER: u32 = TEST + 0xa10;

const PER_VERTEX: [u32; 11] = [
    0x0053_4550,
    0x0065_6fd0,
    0x0065_33f0,
    0x0065_7be0,
    0x0053_4570,
    0x0065_b390,
    0x0045_34f0,
    0x0045_bb20,
    0x0063_c8a0,
    0x0043_9ef0,
    0x0045_6650,
];

fn ret(eax: u32) -> Ret {
    Ret {
        eax,
        ..Ret::default()
    }
}

fn st0(value: f64) -> Ret {
    Ret {
        st0: value,
        ..Ret::default()
    }
}

fn count(e: &mut Engine, at: u32) {
    let n = e.mem.u32(at);
    e.mem.set_u32(at, n + 1);
}

/// The constants, the scene's doubles and the vtables.
fn stand_ins(e: &mut Engine) {
    e.map(TEST, 0x1000);
    e.map(0x0101_2000, 0x1000);
    e.mem.set_f64(0x0101_2060, 0.0);
    e.mem.set_f64(0x0101_2070, 1.0);
    e.map(0x0101_7000, 0x1000);
    e.mem.set_f32(0x0101_7b78, 10.0);
    e.map(0x0101_a000, 0x1000);
    e.mem.set_f64(0x0101_a6b0, -1.0);
    e.map(0x0102_9000, 0x1000);
    e.mem.set_f64(0x0102_90b0, 0.25);
    e.map(0x011d_5000, 0x1000);
    e.mem.set_u32(0x011d_5b8c, 0x0700_0000);
    // The camera holder: an object whose position the double answers.
    e.mem.set_u32(0x011d_59ec, CAMERA_POSITION);
    e.register(0x00a6_7410, |e, a| {
        let (array, stride) = (e.mem.u32(LOCKED_ARRAY), e.mem.u32(LOCKED_STRIDE));
        e.mem.set_u32(a[1], array);
        e.mem.set_u32(a[1] + 4, stride);
        e.mem.set_u8(a[1] + 8, 0);
        ret(0)
    });
    e.register(0x00a5_bdd0, |e, _| ret(e.mem.u32(EXTRA)));
    e.register(0x00a6_7090, |e, _| {
        count(e, MARKS);
        ret(0)
    });
    // The scene: children, their geometry, the data lock (refused for the third child), unlock.
    e.register(0x0043_b480, |e, _| ret(e.mem.u32(CHILDREN)));
    e.register(0x0043_b4a0, |e, a| ret(e.mem.u32(CHILDREN + 4 + 4 * a[1])));
    e.register(D_GEOMETRY_OF, |e, a| ret(e.mem.u32(a[0] + 4)));
    e.register(0x0065_3230, |e, a| ret(e.mem.u32(a[0] + 0x10)));
    e.register(0x00a6_7360, |e, a| {
        count(e, LOCKS);
        ret(u32::from(e.mem.u32(a[0] + 0x30) == 0))
    });
    e.register(0x00a6_73d0, |_, _| ret(0));
    // Settings (far +0x5b28, near +0x5b34), the quartering test, menu mode.
    e.register(0x0045_0410, |e, a| {
        let at = if a[0] == 0x011d_5b28 { FAR } else { NEAR };
        st0(f64::from(e.mem.f32(at)))
    });
    e.register(0x0052_5420, |e, _| ret(e.mem.u32(QUARTERED)));
    e.register(0x0070_2360, |e, _| ret(e.mem.u32(MENU_MODE)));
    // Positions: the camera holder's and the node's.
    e.register(0x0045_bb80, |_, a| {
        ret(if a[0] == CAMERA_POSITION {
            CAMERA_POSITION
        } else {
            NODE_POSITION
        })
    });
    // The parent and its slot +0x10.
    e.register(0x0096_11e0, |_, _| ret(PARENT_VTABLE - 0x100));
    e.register(D_PARENT_ANSWER, |e, _| ret(e.mem.u32(PARENT_ANSWER)));
    // The animation data's +0xd4 and +0xb4.
    e.register(D_ANIMATED, |e, _| ret(e.mem.u32(ANIMATED)));
    e.register(D_RESET, |e, _| {
        count(e, RESETS);
        ret(0)
    });
    e.register(D_OTHER_APPLY, |e, _| {
        count(e, OTHER_APPLIES);
        ret(0)
    });
    e.mem.set_u32(NODE_VTABLE + 0x100, 0x0066_29f0);
    for (slot, target) in [
        (0x94, 0x0064_99e0),
        (0xa4, 0x0064_9aa0),
        (0xac, 0x0064_9b10),
        (0xd4, D_ANIMATED),
        (0xb4, D_RESET),
    ] {
        e.mem.set_u32(DATA_VTABLE + slot, target);
    }
    e.mem.set_u32(CHILD_VTABLE + 0x1c, D_GEOMETRY_OF);
    e.mem.set_u32(DIFFERENTIAL_VTABLE + 4, 0x0066_0860);
    e.mem.set_u32(STATISTICAL_VTABLE + 4, 0x0066_0a90);
    e.mem.set_u32(OTHER_VTABLE + 4, D_OTHER_APPLY);
    // The parent object sits 0x100 below its vtable word.
    e.mem.set_u32(PARENT_VTABLE - 0x100, PARENT_VTABLE);
    e.mem.set_u32(PARENT_VTABLE + 0x10, D_PARENT_ANSWER);
}

/// Weights from the pattern: in and out of `(0, 1]`, a NaN, zeros.
const WEIGHTS: [f32; 8] = [0.0, 0.25, 1.0, 1.5, -0.25, f32::NAN, 0.75, -0.0];

/// A weight keyframe of `count` weights from the pattern at `at` (0x14 bytes in place).
fn weights_at(e: &mut Engine, at: u32, count: u32, pattern: u32) {
    let array = e.mem.alloc(4 * count.max(1));
    for i in 0..count {
        let n = (i * 3 + pattern * 5) as usize;
        e.mem.set_f32(array + 4 * i, WEIGHTS[n % WEIGHTS.len()]);
    }
    e.mem.set_u32(at + 0xc, array);
    e.mem.set_u32(at + 0x10, count);
}

/// The animation data with its four weight arrays from the pattern.
fn animation_data(e: &mut Engine, pattern: u32) -> u32 {
    let data = e.mem.alloc(0x200);
    e.mem.set_u32(data, DATA_VTABLE);
    weights_at(e, data + 0x74, 15, pattern);
    weights_at(e, data + 0xa8, 17, pattern + 1);
    weights_at(e, data + 0xdc, 16, pattern + 2);
    weights_at(e, data + 0x110, 1, pattern + 3);
    data
}

/// A morph of `kind` (0 differential, 1 statistical, 2 other) over `vertices` vertices.
fn a_morph(e: &mut Engine, kind: u32, vertices: u32, salt: u32) -> u32 {
    let m = e.mem.alloc(0x10);
    let vtable = [DIFFERENTIAL_VTABLE, STATISTICAL_VTABLE, OTHER_VTABLE][kind as usize % 3];
    e.mem.set_u32(m, vtable);
    let n = vertices.max(1);
    let offsets = e.mem.alloc(12 * n);
    for i in 0..3 * n {
        e.mem
            .set_f32(offsets + 4 * i, ((i * 7 + salt) % 13) as f32 / 8.0 - 0.75);
    }
    if kind % 3 == 1 {
        let indices = e.mem.alloc(4 * n);
        for i in 0..n {
            e.mem.set_u32(indices + 4 * i, (i * 5 + salt) % n);
        }
        e.mem.set_u32(m + 4, indices);
        e.mem.set_u32(m + 0xc, 0);
    } else {
        e.mem.set_u32(m + 4, offsets);
    }
    e.mem.set_u32(m + 8, vertices);
    m
}

/// Morph data with its four tables (15, 17, 16, 1 slots) of morphs of mixed kinds; some slots
/// empty, one table missing when `holes`.
fn morph_data(e: &mut Engine, vertices: u32, pattern: u32, holes: bool) -> u32 {
    let data = e.mem.alloc(0x20);
    for (offset, slots) in [(0x8u32, 15u32), (0xc, 17), (0x10, 16), (0x14, 1)] {
        if holes && offset == 0xc {
            continue;
        }
        let table = e.mem.alloc(4 * slots);
        for s in 0..slots {
            let k = s + pattern + offset;
            let m = if k % 5 == 4 {
                0
            } else {
                a_morph(e, k % 3, vertices, k)
            };
            e.mem.set_u32(table + 4 * s, m);
        }
        e.mem.set_u32(data + offset, table);
    }
    data
}

/// The basis the statistical morphs read (the extra data's +0xc).
fn base_of(e: &mut Engine, vertices: u32) -> u32 {
    let base = e.mem.alloc(0x10);
    let n = 2 * vertices.max(1) + 2;
    let basis = e.mem.alloc(12 * n);
    for i in 0..3 * n {
        e.mem
            .set_f32(basis + 4 * i, ((i * 11) % 17) as f32 / 4.0 - 2.0);
    }
    e.mem.set_u32(base + 0xc, basis);
    base
}

/// A face geometry of `vertices` vertices with its locked array (stride 12), its data's lock
/// refused when `refuse`; returns the geometry.
fn geometry(e: &mut Engine, vertices: u32, pattern: u32, refuse: bool) -> u32 {
    let data = e.mem.alloc(0x40);
    e.mem.set_u16(data + 8, vertices as u16);
    let form = e.mem.alloc(0x40);
    e.mem.set_u32(data + 0x20, form);
    e.mem.set_u32(data + 0x30, u32::from(refuse));
    let g = e.mem.alloc(0xc0);
    e.mem.set_u32(g + 0xb8, data);
    // Four vertices of slack: a statistical morph may write one past the count.
    let array = e.mem.alloc(12 * (vertices + 4));
    for i in 0..3 * vertices {
        e.mem
            .set_f32(array + 4 * i, ((i + pattern) % 9) as f32 / 3.0 - 1.0);
    }
    e.mem.set_u32(LOCKED_ARRAY, array);
    e.mem.set_u32(LOCKED_STRIDE, 12);
    let extra = e.mem.alloc(0x10);
    let offsets = e.mem.alloc(12 * (vertices + 4));
    e.mem.set_u32(extra + 0xc, offsets);
    e.mem.set_u32(EXTRA, extra);
    g
}

/// A context `{geometry, cursor, morph data, base, vertices, prepared}` on the heap.
fn context(e: &mut Engine, vertices: u32, pattern: u32, prepared: bool) -> u32 {
    let g = geometry(e, vertices, pattern, false);
    let c = e.mem.alloc(0x24);
    e.mem.set_u32(c, g);
    let array = e.mem.u32(LOCKED_ARRAY);
    e.mem.set_u32(c + 4, array);
    e.mem.set_u32(c + 8, 12);
    let data = morph_data(e, vertices, pattern, pattern == 2);
    e.mem.set_u32(c + 0x10, data);
    let base = base_of(e, vertices);
    e.mem.set_u32(c + 0x14, base);
    e.mem.set_u32(c + 0x18, vertices);
    e.mem.set_u8(c + 0x1c, u8::from(prepared));
    c
}

/// A node over the animation data.
fn node(e: &mut Engine, pattern: u32) -> u32 {
    let n = e.mem.alloc(0xf0);
    e.mem.set_u32(n, NODE_VTABLE);
    let data = animation_data(e, pattern);
    e.mem.set_u32(n + 0xac, data);
    n
}

trait Common {
    fn common(self) -> Self;
}

impl Common for Diff {
    fn common(self) -> Diff {
        self.family(&facegen::FACEGEN)
            .family(&engine_typed::multibound::MULTIBOUND)
            .family(&leaf::LEAVES)
            .ignore_calls(&PER_VERTEX)
            // The translations construct their stack points (`006815c0`, a leaf answering its `this`).
            .ignore_calls(&[0x0068_15c0])
            // The geometry data's vertex count (`004a8ae0`, its word at +8): the typed morph reads it
            // in place (#1101).
            .ignore_calls(&[0x004a_8ae0])
            // The pair split's two outputs are locals it only writes: what they held before (stack residue) differs.
            .local_at(0x0065_3d70, 2, 0)
            .local_at(0x0065_3d70, 3, 0)
            .local_at(0x00a6_7410, 1, 0xc)
            .allow_nan_payloads()
            .max_cases(160)
            .max_calls(200_000)
    }
}

#[test]
fn facegen_morph_data_apply_one() {
    Diff::new(entry!(
        0x0066_2030,
        apply::apply_one(Ptr<_>, u32, i32, u32, u32, f32)
    ))
    .common()
    .inputs(&[
        Input::Of(&[0, 1, 2, 3, 4]),
        Input::Of(&[0xffff_ffff, 0, 1, 2, 4, 9, 14, 15, 16, 17]),
        Input::Of(&[0, 1, 2]),
        Input::Of(&[0x3e80_0000, 0x3f80_0000]),
    ])
    .setup(|e, case: &Case| {
        stand_ins(e);
        let c = context(e, 30, case.word(2), false);
        let data = e.mem.u32(c + 0x10);
        let g = e.mem.u32(c);
        let base = e.mem.u32(c + 0x14);
        vec![data, case.word(0), case.word(1), g, base, case.word(3)]
    })
    .run()
    .assert_same();
}

/// Table indices: below, in and past each table, and a huge one.
const TABLE_INDICES: [u32; 8] = [0xffff_ffff, 0, 1, 14, 15, 16, 17, 0x7fff_ffff];

fn table_setup(e: &mut Engine, case: &Case) -> Vec<u32> {
    stand_ins(e);
    let data = morph_data(e, 4, case.word(1), case.word(1) == 2);
    vec![data, case.word(0)]
}

#[test]
fn facegen_morph_data_expression_table() {
    Diff::new(entry!(
        0x0066_20d0,
        apply::expression_morph(Ptr<_>, i32) -> u32
    ))
    .common()
    .inputs(&[Input::Of(&TABLE_INDICES), Input::Of(&[0, 2])])
    .setup(table_setup)
    .run()
    .assert_same();
}

#[test]
fn facegen_morph_data_modifier_table() {
    Diff::new(entry!(
        0x0066_2110,
        apply::modifier_morph(Ptr<_>, i32) -> u32
    ))
    .common()
    .inputs(&[Input::Of(&TABLE_INDICES), Input::Of(&[0, 2])])
    .setup(table_setup)
    .run()
    .assert_same();
}

#[test]
fn facegen_morph_data_phoneme_table() {
    Diff::new(entry!(
        0x0066_2150,
        apply::phoneme_morph(Ptr<_>, i32) -> u32
    ))
    .common()
    .inputs(&[Input::Of(&TABLE_INDICES), Input::Of(&[0, 2])])
    .setup(table_setup)
    .run()
    .assert_same();
}

#[test]
fn facegen_morph_data_custom_table() {
    Diff::new(entry!(0x0066_2190, apply::custom_morph(Ptr<_>, i32) -> u32))
        .common()
        .inputs(&[Input::Of(&TABLE_INDICES), Input::Of(&[0, 2])])
        .setup(table_setup)
        .run()
        .assert_same();
}

/// Weight indices: below, in and past each array, and the sign bit.
const WEIGHT_INDICES: [u32; 9] = [0xffff_ffff, 0, 1, 8, 14, 15, 16, 17, 0x8000_0000];

fn weights_setup(e: &mut Engine, case: &Case) -> Vec<u32> {
    stand_ins(e);
    vec![animation_data(e, case.word(1)), case.word(0)]
}

#[test]
fn facegen_animation_expression_weight() {
    Diff::new(entry!(
        0x0064_99e0,
        apply::expression_value(Ptr<_>, i32) -> f32
    ))
    .common()
    .inputs(&[Input::Of(&WEIGHT_INDICES), Input::Of(&[0, 1, 2])])
    .setup(weights_setup)
    .run()
    .assert_same();
}

#[test]
fn facegen_animation_phoneme_weight() {
    Diff::new(entry!(
        0x0064_9aa0,
        apply::phoneme_value(Ptr<_>, i32) -> f32
    ))
    .common()
    .inputs(&[Input::Of(&WEIGHT_INDICES), Input::Of(&[0, 1, 2])])
    .setup(weights_setup)
    .run()
    .assert_same();
}

#[test]
fn facegen_animation_custom_weight() {
    Diff::new(entry!(0x0064_9b10, apply::custom_value(Ptr<_>, i32) -> f32))
        .common()
        .inputs(&[Input::Of(&WEIGHT_INDICES), Input::Of(&[0, 1, 2])])
        .setup(weights_setup)
        .run()
        .assert_same();
}

#[test]
fn facegen_animation_modifier_weight() {
    Diff::new(entry!(
        0x005d_d4a0,
        apply::modifier_value(Ptr<_>, i32) -> f32
    ))
    .common()
    .inputs(&[Input::Of(&WEIGHT_INDICES), Input::Of(&[0, 1, 2])])
    .setup(weights_setup)
    .run()
    .assert_same();
}
#[test]
fn facegen_split_pair() {
    Diff::new(entry!(0x0065_3d70, apply::split_pair(f32, f32, u32, u32)))
        .common()
        .inputs(&[Input::F32, Input::F32])
        .float_sweep(256)
        .setup(|e, case: &Case| {
            stand_ins(e);
            let out = e.mem.alloc(8);
            vec![case.f32(0).to_bits(), case.f32(1).to_bits(), out, out + 4]
        })
        .run()
        .assert_same();
}

/// The appliers' cases: the weights' pattern, the morph tables' pattern, prepared or not.
const APPLIER_INPUTS: [Input; 3] = [
    Input::Of(&[0, 1, 2, 3, 4, 5]),
    Input::Of(&[0, 1, 2]),
    Input::Flag,
];

fn applier_setup(e: &mut Engine, case: &Case) -> Vec<u32> {
    stand_ins(e);
    let n = node(e, case.word(0));
    let c = context(e, 24, case.word(1), case.word(2) != 0);
    vec![n, c]
}

#[test]
fn facegen_apply_expressions() {
    Diff::new(entry!(0x0066_2bc0, apply::apply_expressions(Ptr<_>, u32)))
        .common()
        .inputs(&APPLIER_INPUTS)
        .setup(applier_setup)
        .run()
        .assert_same();
}

#[test]
fn facegen_apply_phonemes() {
    Diff::new(entry!(0x0066_2e90, apply::apply_phonemes(Ptr<_>, u32)))
        .common()
        .inputs(&APPLIER_INPUTS)
        .setup(applier_setup)
        .run()
        .assert_same();
}

#[test]
fn facegen_apply_custom() {
    Diff::new(entry!(0x0066_2f70, apply::apply_custom(Ptr<_>, u32)))
        .common()
        .inputs(&APPLIER_INPUTS)
        .setup(applier_setup)
        .run()
        .assert_same();
}

#[test]
fn facegen_apply_modifiers() {
    Diff::new(entry!(0x0066_2ca0, apply::apply_modifiers(Ptr<_>, u32)))
        .common()
        .inputs(&APPLIER_INPUTS)
        .setup(applier_setup)
        .run()
        .assert_same();
}
#[test]
fn facegen_morph_pass() {
    Diff::new(entry!(0x0066_3050, apply::morph_pass(Ptr<_>, u8)))
        .common()
        .inputs(&[
            // The distance: near, between the settings, far.
            Input::Of(&[0, 1, 2, 3]),
            // Children: none; one; a null one, one whose lock is refused and one; one without
            // morph data.
            Input::Of(&[0, 1, 2, 3]),
            Input::Of(&[0, 1, 2]),
            Input::Flag,              // force
            Input::Of(&[0, 1, 2, 3]), // flags: base, disable, force updates, dialogue
            Input::Of(&[0, 1, 2, 3]), // quartered, menu mode, animated, parent answer
        ])
        .setup(|e, case: &Case| {
            stand_ins(e);
            let n = node(e, case.word(2));
            let flags = case.word(4);
            e.mem.set_u8(n + 0xd4, u8::from(flags == 0));
            e.mem.set_u8(n + 0xe3, u8::from(flags == 1));
            e.mem.set_u8(n + 0xe4, u8::from(flags == 2));
            e.mem.set_u8(n + 0xe2, u8::from(flags == 3));
            let world = case.word(5);
            e.mem.set_u32(QUARTERED, u32::from(world == 0));
            e.mem.set_u32(MENU_MODE, u32::from(world == 1));
            e.mem.set_u32(ANIMATED, u32::from(world >= 2));
            e.mem.set_u32(PARENT_ANSWER, u32::from(world == 3));
            e.mem.set_f32(FAR, 300.0);
            e.mem.set_f32(NEAR, 1200.0);
            let distance = [50.0f32, 700.0, 1215.0, 5000.0][case.word(0) as usize];
            for k in 0..3 {
                e.mem.set_f32(CAMERA_POSITION + 4 * k, 0.0);
                e.mem
                    .set_f32(NODE_POSITION + 4 * k, if k == 0 { distance } else { 0.0 });
            }
            let child = |e: &mut Engine, refuse: bool, with_morphs: bool| -> u32 {
                let g = geometry(e, 20, case.word(2), refuse);
                let data = if with_morphs {
                    morph_data(e, 20, case.word(2), false)
                } else {
                    0
                };
                e.mem.set_u32(g + 0x10, data);
                let c = e.mem.alloc(0x10);
                e.mem.set_u32(c, CHILD_VTABLE);
                e.mem.set_u32(c + 4, g);
                c
            };
            let kids: Vec<u32> = match case.word(1) {
                0 => vec![],
                1 => vec![child(e, false, true)],
                2 => vec![0, child(e, true, true), child(e, false, true)],
                _ => vec![child(e, false, false)],
            };
            e.mem.set_u32(CHILDREN, kids.len() as u32);
            for (i, k) in kids.iter().enumerate() {
                e.mem.set_u32(CHILDREN + 4 + 4 * i as u32, *k);
            }
            vec![n, case.word(3)]
        })
        .run()
        .assert_same();
}

// The exact skip: frames of one head on an engine with the translations only and on one with the
// face family on, the same inputs each frame; after every frame the vertex arrays hold the same
// words on both, and the typed engine leaves out the reshape (no `MarkAsChanged`) exactly on the
// frames where nothing that feeds it changed.

/// One head with two morphed children (and their arrays); returns the node, the animation data and
/// the two geometries.
fn head(e: &mut Engine) -> (u32, u32, [u32; 2]) {
    stand_ins(e);
    let n = node(e, 1);
    let data = e.mem.u32(n + 0xac);
    e.mem.set_f32(FAR, 300.0);
    e.mem.set_f32(NEAR, 1200.0);
    for k in 0..3 {
        e.mem.set_f32(CAMERA_POSITION + 4 * k, 0.0);
        e.mem.set_f32(NODE_POSITION + 4 * k, 0.0);
    }
    let mut geometries = [0; 2];
    let mut children = Vec::new();
    for (i, g) in geometries.iter_mut().enumerate() {
        *g = geometry(e, 20 + 4 * i as u32, i as u32, false);
        let g_data = e.mem.u32(*g + 0xb8);
        let array = e.mem.u32(LOCKED_ARRAY);
        e.mem.set_u32(g_data + 0x38, array);
        let base = e.mem.u32(EXTRA);
        let offsets = e.mem.u32(base + 0xc);
        for w in 0..3 * (20 + 4 * i as u32) {
            e.mem.set_f32(offsets + 4 * w, (w % 7) as f32 / 16.0);
        }
        // Each child's own extra data: the double answers the one at +0x20 of the geometry.
        e.mem.set_u32(*g + 0x20, base);
        let morphs = morph_data(e, 20 + 4 * i as u32, i as u32, false);
        own_morphs_only(e, morphs);
        e.mem.set_u32(*g + 0x10, morphs);
        let c = e.mem.alloc(0x10);
        e.mem.set_u32(c, CHILD_VTABLE);
        e.mem.set_u32(c + 4, *g);
        children.push(c);
    }
    // Each geometry's own locked array and extra data (the doubles read them per geometry).
    e.register_double(0x00a5_bdd0, |e, a| {
        count(e, BASE_COPIES);
        ret(e.mem.u32(a[0] + 0x20))
    });
    e.register_double(0x00a6_7410, |e, a| {
        // `a[0]` is the geometry's data; the array sits at +0x38 of it.
        e.mem.set_u32(a[1], e.mem.u32(a[0] + 0x38));
        e.mem.set_u32(a[1] + 4, 12);
        e.mem.set_u8(a[1] + 8, 0);
        ret(0)
    });
    e.mem.set_u32(CHILDREN, 2);
    for (i, c) in children.iter().enumerate() {
        e.mem.set_u32(CHILDREN + 4 + 4 * i as u32, *c);
    }
    (n, data, geometries)
}

/// Every morph of `data` whose class is not one of the two own ones made a differential one (the
/// skip keeps only reshapes whose effects are known).
fn own_morphs_only(e: &mut Engine, data: u32) {
    for (offset, slots) in [(0x8u32, 15u32), (0xc, 17), (0x10, 16), (0x14, 1)] {
        let table = e.mem.u32(data + offset);
        if table == 0 {
            continue;
        }
        for s in 0..slots {
            let m = e.mem.u32(table + 4 * s);
            if m != 0 && e.mem.u32(m) == OTHER_VTABLE {
                e.mem.set_u32(m, DIFFERENTIAL_VTABLE);
            }
        }
    }
}

/// The array of a geometry (kept at +0x38 of its data by [`head`]).
fn array_of(e: &Engine, geometry: u32) -> u32 {
    e.mem.u32(e.mem.u32(geometry + 0xb8) + 0x38)
}

/// Each array's positions.
fn shapes(e: &Engine, geometries: &[u32; 2]) -> Vec<Vec<u32>> {
    geometries
        .iter()
        .enumerate()
        .map(|(i, g)| {
            let a = array_of(e, *g);
            (0..3 * (20 + 4 * i as u32))
                .map(|w| e.mem.u32(a + 4 * w))
                .collect()
        })
        .collect()
}

/// What a frame changes before the pass.
#[derive(Clone, Copy, Debug)]
enum Change {
    Nothing,
    Weight(u32, f32),
    OtherWriter,
    Far,
    Near,
    BaseOffset,
    ForceBase,
    /// Both heads freed and rebuilt at the same addresses with other data (other base offsets,
    /// the arrays back to an unmorphed mesh).
    Rebuilt,
    /// Child 0's morphs become a class whose effects are not known; and back.
    UnknownClass,
    KnownClass,
    /// Child 1's first phoneme morph becomes a statistical one writing one vertex past the count.
    PastCount,
    /// Morph rows rewritten in place (pointers unchanged): every differential morph's first offset
    /// and every statistical morph's first index, in both children. The skip assumes this does not
    /// happen (the exe builds the rows once, `00660ba0`); the frame documents what it does if it does.
    MorphRows,
}

/// What the typed engine must do in a frame, against the translations' reshapes.
#[derive(Clone, Copy, Debug)]
enum Expect {
    /// Left out (no reshape).
    Skip,
    /// Reshaped as the exe (both children).
    Full,
    /// One child reshaped, the other left out.
    One,
    /// Left out though the morph rows changed in place: the vertices are the last frame's, not the
    /// exe's (the documented limit of the skip), until something else triggers a reshape.
    Stale,
}

fn frame(e: &mut Engine, n: u32, data: u32, geometries: &[u32; 2], change: Change) {
    match change {
        Change::Nothing => {}
        Change::Weight(at, w) => {
            // Phonemes (+0xdc), expressions (+0x74), modifiers (+0xa8) by `at`.
            let keyframe = [0xdc, 0x74, 0xa8][(at % 3) as usize];
            let array = e.mem.u32(data + keyframe + 0xc);
            e.mem.set_f32(array + 4 * (at % 13), w);
        }
        Change::OtherWriter => {
            let a = array_of(e, geometries[0]);
            e.mem.set_f32(a + 8, 123.0);
        }
        Change::Far => e.mem.set_f32(NODE_POSITION, 700.0),
        Change::Near => e.mem.set_f32(NODE_POSITION, 0.0),
        Change::BaseOffset => {
            let base = e.mem.u32(geometries[1] + 0x20);
            let offsets = e.mem.u32(base + 0xc);
            e.mem.set_f32(offsets + 4, -0.5);
        }
        Change::ForceBase => e.mem.set_u8(n + 0xd4, 1),
        Change::Rebuilt => {
            for (i, g) in geometries.iter().enumerate() {
                let vertices = 20 + 4 * i as u32;
                let a = array_of(e, *g);
                let base = e.mem.u32(*g + 0x20);
                let offsets = e.mem.u32(base + 0xc);
                for w in 0..3 * vertices {
                    e.mem.set_f32(a + 4 * w, (w % 5) as f32 - 2.0);
                    e.mem.set_f32(offsets + 4 * w, (w % 11) as f32 / 32.0);
                }
            }
        }
        Change::UnknownClass | Change::KnownClass => {
            let (from, to) = if matches!(change, Change::UnknownClass) {
                (DIFFERENTIAL_VTABLE, OTHER_VTABLE)
            } else {
                (OTHER_VTABLE, DIFFERENTIAL_VTABLE)
            };
            let morphs = e.mem.u32(geometries[0] + 0x10);
            for (offset, slots) in [(0x8u32, 15u32), (0xc, 17), (0x10, 16), (0x14, 1)] {
                let table = e.mem.u32(morphs + offset);
                for s in 0..slots {
                    let m = e.mem.u32(table + 4 * s);
                    if m != 0 && e.mem.u32(m) == from {
                        e.mem.set_u32(m, to);
                    }
                }
            }
        }
        Change::MorphRows => {
            for (i, g) in geometries.iter().enumerate() {
                let vertices = 20 + 4 * i as u32;
                let morphs = e.mem.u32(*g + 0x10);
                for (offset, slots) in [(0x8u32, 15u32), (0xc, 17), (0x10, 16), (0x14, 1)] {
                    let table = e.mem.u32(morphs + offset);
                    for s in 0..slots {
                        let m = e.mem.u32(table + 4 * s);
                        if m == 0 {
                            continue;
                        }
                        let rows = e.mem.u32(m + 4);
                        if e.mem.u32(m) == STATISTICAL_VTABLE {
                            let index = e.mem.u32(rows);
                            e.mem.set_u32(rows, (index + 1) % vertices);
                        } else {
                            let delta = e.mem.f32(rows);
                            e.mem.set_f32(rows, delta + 0.125);
                        }
                    }
                }
            }
        }
        Change::PastCount => {
            let vertices = 24;
            let m = e.mem.alloc(0x10);
            let indices = e.mem.alloc(8);
            e.mem.set_u32(indices, 3);
            e.mem.set_u32(indices + 4, vertices);
            e.mem.set_u32(m, STATISTICAL_VTABLE);
            e.mem.set_u32(m + 4, indices);
            e.mem.set_u32(m + 8, 2);
            e.mem.set_u32(m + 0xc, 0);
            let morphs = e.mem.u32(geometries[1] + 0x10);
            let phonemes = e.mem.u32(morphs + 0x10);
            for s in 0..16 {
                e.mem.set_u32(phonemes + 4 * s, m);
            }
        }
    }
    e.call(0x0066_3050, &args![n, 0u32]);
}

#[test]
fn facegen_skip_gives_the_same_vertices() {
    use Change::*;
    use Expect::*;
    let mut frames = vec![
        (Nothing, Full),
        (Nothing, Skip),
        (Weight(0, 0.5), Full),
        (Nothing, Skip),
        (Weight(1, 0.75), Full),
        (Weight(1, 0.75), Skip),
        (OtherWriter, One),
        (Nothing, Skip),
        (Far, Full),
        (Nothing, Skip),
        (Near, Full),
        (BaseOffset, One),
        (Nothing, Skip),
        // A forced base copy is the copy the first morph makes: the same writes.
        (ForceBase, Skip),
        (Nothing, Skip),
        (Weight(2, 0.25), Full),
        (Nothing, Skip),
        // A weight going A, B, A.
        (Weight(0, 0.75), Full),
        (Weight(0, 0.5), Full),
        (Nothing, Skip),
        (Rebuilt, Full),
        (Nothing, Skip),
        // Morph rows changed in place: not seen (documented); the next weight change catches up.
        (MorphRows, Stale),
        (Weight(0, 0.6), Full),
        (Nothing, Skip),
        // An unknown class is never kept (child 0 reshapes every frame, child 1 is left out);
        // back to the known one, it is kept again.
        (UnknownClass, One),
        (Nothing, One),
        (Nothing, One),
        (KnownClass, One),
        (Nothing, Skip),
    ];
    // A talking face: three misses in a row and the arrays rest (the exe's pass, no compare),
    // then are kept again.
    for k in 0..4u8 {
        frames.push((Weight(0, 0.1 + 0.1 * f32::from(k)), Full));
    }
    frames.extend(std::iter::repeat_n((Nothing, Full), 2));
    let mut translated = Engine::new_untyped_for_test();
    let mut typed = Engine::new();
    let (tn, td, tg) = head(&mut translated);
    let (yn, yd, yg) = head(&mut typed);
    let mut run = |frames: &[(Change, Option<Expect>)], from: usize| {
        for (j, (change, expect)) in frames.iter().enumerate() {
            let i = from + j;
            let (tm, ym) = (translated.mem.u32(MARKS), typed.mem.u32(BASE_COPIES));
            let typed_marks = typed.mem.u32(MARKS);
            frame(&mut translated, tn, td, &tg, *change);
            frame(&mut typed, yn, yd, &yg, *change);
            // Base copies made: each child's pass asks the extra data once (`00653430`), each copy once more.
            let reshaped = typed.mem.u32(BASE_COPIES) - ym - 2;
            assert_eq!(
                typed.mem.u32(MARKS) - typed_marks,
                translated.mem.u32(MARKS) - tm,
                "frame {i} ({change:?}): MarkAsChanged as the exe makes it, skipped or not"
            );
            if matches!(expect, Some(Stale)) {
                assert_ne!(
                    shapes(&translated, &tg),
                    shapes(&typed, &yg),
                    "frame {i} ({change:?}): expected the documented stale vertices"
                );
                assert_eq!(reshaped, 0, "frame {i} ({change:?}): expected no reshape");
                continue;
            }
            assert_eq!(
                shapes(&translated, &tg),
                shapes(&typed, &yg),
                "frame {i} ({change:?}): the vertices differ"
            );
            let translation_reshaped = translated.mem.u32(MARKS) - tm;
            assert!(
                translation_reshaped > 0,
                "frame {i}: the head was not morphed"
            );
            let expected = match expect {
                None => continue,
                Some(Skip) => 0,
                Some(Full) => translation_reshaped,
                Some(One) => translation_reshaped / 2,
                Some(Stale) => unreachable!(),
            };
            assert_eq!(
                reshaped, expected,
                "frame {i} ({change:?}): expected {expect:?}"
            );
        }
    };
    let listed: Vec<(Change, Option<Expect>)> =
        frames.iter().map(|(c, x)| (*c, Some(*x))).collect();
    run(&listed, 0);
    // Through the rest (the vertices still compared every frame), then kept again: one frame
    // to keep, then left out.
    let rest: Vec<(Change, Option<Expect>)> = std::iter::repeat_n((Nothing, None), 40).collect();
    run(&rest, listed.len());
    let after = [(Nothing, Some(Skip))];
    run(&after, listed.len() + 40);
    // A statistical vertex past the count: child 1 is never kept.
    let past = [
        (PastCount, Some(One)),
        (Nothing, Some(One)),
        (Nothing, Some(One)),
    ];
    run(&past, listed.len() + 41);
}
