//! The differential test of every typed function of the animation family (`engine_typed::animation`)
//! against its translation. The world: a bip node with its controller list (each controller an object
//! whose vtable's stand-in slots answer its class's `NiRTTI` and log its `Update`); multi-target
//! transform controllers with their flags, bones (each with its flags at +0x30) and inline
//! interpolators: stand-ins whose transform `Update` (slot +0x8c) writes the case's transform, or blend
//! transform interpolators (the real `Update` in the slot) with their count, flags, single interpolator
//! and single time; the exe's constants.
//!
//! What stands in: the update data's and the transform's constructors, the transform's translation
//! test and its rotation and scale getters, the bone's translation and scale setters, the matrix's
//! column set, the quaternion's normalisation, the weights' normalisation and the blend of several
//! interpolators. Everything else runs translated.

use engine as _;
use engine_core::prelude::*;
use engine_core::typed::diff::{Case, Diff, Input};
use engine_typed::animation::{self, bip, blend};
use engine_typed::{dispatch, pathing};

const VT_FN: u32 = 0x00ff_2000;
const VT_SLOTS: u32 = 0x100;
const LOG: u32 = 0x0300_0000;
const MAX_CALLS: usize = 5_000;
const FLOAT_SWEEP: usize = 32;

const ONE: u32 = 0x3f80_0000;
const TWO: u32 = 0x4000_0000;
const HALF: u32 = 0x3f00_0000;
const THIRD: u32 = 0x3eaa_aaab;
const MINUS_ONE: u32 = 0xbf80_0000;
/// `-FLT_MAX`, the invalid marker; a quiet and a signalling NaN.
const LOWEST: u32 = 0xff7f_ffff;
const NAN: u32 = 0x7fc0_0000;
const SNAN: u32 = 0x7f80_0001;

/// The classes the bip node's walk tells apart (`NiMultiTargetTransformController`,
/// `BSMultiTargetTreadTransfController`) and one it passes over.
const MULTI_TARGET_RTTI: u32 = 0x011f_36bc;
const TREAD_RTTI: u32 = 0x011f_36e4;
const OTHER_RTTI: u32 = 0x0120_0000;
/// `NiBlendTransformInterpolator::Update`, the real function a test puts in slot +0x8c.
const BLEND_UPDATE: u32 = 0x00a4_1110;

/// The answers a test sets: the translation's test, the blend of several interpolators.
const TAG_TRANSLATE_VALID: u32 = 0x01;
const TAG_BLEND: u32 = 0x04;

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

fn log(e: &mut Engine, tag: u32, word: u32) {
    let n = e.mem.u32(LOG);
    if n >= 0x7f0 {
        return;
    }
    e.mem.set_u32(LOG + 4 + 8 * n, tag);
    e.mem.set_u32(LOG + 8 + 8 * n, word);
    e.mem.set_u32(LOG, n + 1);
}

fn bits(case: &Case, i: usize) -> u32 {
    case.f32(i).to_bits()
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

/// A stand-in that logs its words under `TAG` and answers the word at `LOG + 0x3f00 + 4 * (TAG &
/// 0x3f)`, so a test can set what it answers.
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

/// Any slot but the transform `Update`: logs `this` and its words, answers the vtable's answer word (a
/// controller's `GetRTTI` answers its class).
fn any_slot(e: &mut Engine, a: &[u32]) -> Ret {
    let this = a.first().copied().unwrap_or(0);
    log(e, 0xffff, this);
    for w in a.iter().skip(1) {
        log(e, 0xfffe, *w);
    }
    let vt = e.mem.u32(this);
    ret(e.mem.u32(vt + VT_SLOTS))
}

/// An interpolator's transform `Update` (slot +0x8c, `thiscall(this, time, object, transform)`): logs
/// its words, writes the vtable's eight transform words into the transform and answers the vtable's
/// answer word (its low byte is what the callers read).
fn transform_slot(e: &mut Engine, a: &[u32]) -> Ret {
    log(e, 0x8c, a[0]);
    for w in &a[1..] {
        log(e, 0x1_008c, *w);
    }
    let vt = e.mem.u32(a[0]);
    for k in 0..8 {
        let w = e.mem.u32(vt + VT_SLOTS + 4 + 4 * k);
        e.mem.set_u32(a[3] + 4 * k, w);
    }
    ret(e.mem.u32(vt + VT_SLOTS))
}

/// A vtable of stand-in slots whose answer word (after the slots) is `answer`, with the words
/// `transform` after it (what [`transform_slot`] writes).
fn vtable(e: &mut Engine, answer: u32, transform: &[u32]) -> u32 {
    let vt = e.mem.alloc(VT_SLOTS + 0x40);
    for slot in (0..VT_SLOTS).step_by(4) {
        e.mem.set_u32(vt + slot, VT_FN + slot);
    }
    e.mem.set_u32(vt + VT_SLOTS, answer);
    words(e, vt + VT_SLOTS + 4, transform);
    vt
}

/// An object of `size` bytes with its own [`vtable`].
fn object(e: &mut Engine, size: u32, answer: u32, transform: &[u32]) -> u32 {
    let vt = vtable(e, answer, transform);
    let o = e.mem.alloc(size);
    e.mem.set_u32(o, vt);
    o
}

/// The exe's data and the stand-ins.
fn stand_ins(e: &mut Engine) {
    e.map(LOG, 0x4000);
    for page in [
        0x0101_1000,
        0x0101_5000,
        0x0102_3000,
        0x0102_4000,
        0x0109_6000,
    ] {
        e.map(page, 0x1000);
    }
    // 2.0 (a double), the invalid transform part, FLT_MAX, -FLT_MAX (a double) and
    // `NiInterpolator::INVALID_FLOAT`, as in the exe.
    e.mem.set_u64(0x0101_1590, 2.0f64.to_bits());
    e.mem.set_u32(0x0101_5f5c, LOWEST);
    e.mem.set_u32(0x0102_3900, 0x7f7f_ffff);
    e.mem.set_u64(0x0102_41b0, f64::from(f32::MIN).to_bits());
    e.mem.set_u32(0x0109_696c, LOWEST);
    // The update data's constructor (`thiscall(data, time, a, b)`): the three words.
    e.register(0x0043_d410, |e, a| {
        log(e, 0xd410, a[0]);
        words(e, a[0], &a[1..4]);
        ret(a[0])
    });
    // The transform's constructor: a marker in its eight words.
    e.register(0x0049_4260, |e, a| {
        log(e, 0x4260, a[0]);
        for k in 0..8 {
            e.mem.set_u32(a[0] + 4 * k, 0x1111_1111 * (k + 1));
        }
        ret(a[0])
    });
    // The translation's test answers what the test set; the rotation getter answers the rotation
    // (+0xc), the scale getter the scale (+0x1c) in ST0.
    e.register(0x0049_42c0, logged::<TAG_TRANSLATE_VALID>);
    e.register(0x0048_d150, |e, a| {
        log(e, 0xd150, a[0]);
        ret(a[0] + 0xc)
    });
    e.register(0x0049_5460, |e, a| {
        log(e, 0x5460, a[0]);
        ret_f(f64::from(f32::from_bits(e.mem.u32(a[0] + 0x1c))))
    });
    // The bone's translation and scale setters.
    e.register(0x0044_0460, logged::<0x02>);
    e.register(0x0044_0490, logged::<0x03>);
    // A matrix column (`thiscall(matrix, column, x, y, z)`), rows of three floats.
    e.register(0x004b_4660, |e, a| {
        log(e, 0x4660, a[1]);
        if a[1] < 3 {
            for k in 0..3u32 {
                e.mem.set_u32(a[0] + 0xc * k + 4 * a[1], a[2 + k as usize]);
            }
        }
        ret(0)
    });
    // The quaternion's normalisation: its four words reversed.
    e.register(0x00a3_8560, |e, a| {
        log(e, 0x8560, a[0]);
        let q: Vec<u32> = (0..4).map(|k| e.mem.u32(a[0] + 4 * k)).collect();
        for (k, w) in (0u32..).zip(q.iter().rev()) {
            e.mem.set_u32(a[0] + 4 * k, *w);
        }
        ret(a[0])
    });
    // The weights' normalisation; the blend of several answers what the test set (1 unless set).
    e.register(0x00a3_7260, logged::<0x05>);
    e.register(0x00a4_0c10, logged::<TAG_BLEND>);
    set_logged_answer(e, TAG_BLEND, 1);
    for s in (0..VT_SLOTS).step_by(4) {
        e.register(VT_FN + s, any_slot);
    }
    e.register(VT_FN + 0x8c, transform_slot);
}

/// The transform an interpolator gives, from the case's floats at `f`: the translation (the float
/// `f + 5`, 1, 2), the rotation (w `f + 2`, x `f`, y `f + 3`, z `f + 4`; x is the float the
/// rotation's test reads) and the scale (`f + 1`).
fn transform_words(case: &Case, f: usize) -> [u32; 8] {
    let b = |i| bits(case, f + i);
    [b(5), ONE, TWO, b(2), b(0), b(3), b(4), b(1)]
}

/// The case's transform floats: the rotation's x and the scale from the operand classes (the invalid
/// marker among them), the rest swept.
const TRANSFORM: [Input; 6] = [
    Input::F32,
    Input::F32,
    Input::F32Swept(HALF),
    Input::F32Swept(THIRD),
    Input::F32Swept(MINUS_ONE),
    Input::F32Swept(ONE),
];

fn inputs(parts: &[&[Input]]) -> Vec<Input> {
    parts.iter().flat_map(|p| p.iter().copied()).collect()
}

/// The interpolator at `at` (0x30 bytes inline) by `kind`: 0, a stand-in giving the case's transform;
/// 1, one giving nothing (its answer 0x100, whose low byte is 0); 2, 3 and 4, a blend transform
/// interpolator (the real `Update` in its slot) with one interpolator (a stand-in giving the
/// transform), none, or two (the blend's stand-in).
fn interpolator(e: &mut Engine, case: &Case, at: u32, kind: u32, f: usize) {
    let transform = transform_words(case, f);
    if kind < 2 {
        let vt = vtable(e, if kind == 0 { 1 } else { 0x100 }, &transform);
        e.mem.set_u32(at, vt);
        return;
    }
    let vt = vtable(e, 0, &[]);
    e.mem.set_u32(vt + 0x8c, BLEND_UPDATE);
    e.mem.set_u32(at, vt);
    e.mem.set_u32(at + 8, 0x5555_5555);
    e.mem.set_u8(at + 0xe, [1, 0, 2][(kind - 2) as usize]);
    let single = object(e, 0x10, 1, &transform);
    e.mem.set_u32(at + 0x18, single);
    e.mem.set_u32(at + 0x20, HALF);
}

/// A multi-target transform controller (0x48 bytes, its class from `GetRTTI`) with the word `flags`
/// at +8 and `count` bones (+0x38, the count +0x3c) with inline interpolators (+0x34, 0x30 bytes
/// each). Bone `i`'s two bits of `pattern` (from bit `2i`): 1, no bone; 2, its flag 2 set. Its
/// interpolator is [`interpolator`]'s kind `(kind + i) % 5`.
fn controller(
    e: &mut Engine,
    case: &Case,
    [flags, count, pattern, kind]: [u32; 4],
    f: usize,
) -> u32 {
    let c = object(e, 0x48, MULTI_TARGET_RTTI, &[]);
    e.mem.set_u32(c + 8, flags);
    let targets = block(e, 0x10, &[0; 4]);
    let interpolators = e.mem.alloc(4 * 0x30);
    for i in 0..count {
        let bone_bits = (pattern >> (2 * i)) & 3;
        if bone_bits & 1 == 0 {
            let flag = if bone_bits & 2 != 0 { 2 } else { 0x0100_0001 };
            let bone = block(e, 0x70, &[0x3333_3333; 28]);
            e.mem.set_u32(bone + 0x30, flag);
            e.mem.set_u32(targets + 4 * i, bone);
        }
        interpolator(e, case, interpolators + 0x30 * i, (kind + i) % 5, f);
    }
    e.mem.set_u32(c + 0x34, interpolators);
    e.mem.set_u32(c + 0x38, targets);
    e.mem.set_u16(c + 0x3c, count as u16);
    c
}

/// A blend transform interpolator (0x30 bytes) with `count` interpolators (+0xe), the flags byte
/// (+0xc, bit 0: manager controlled), a single interpolator (+0x18: a stand-in answering `gave` with
/// the case's transform) and the single time (+0x20); the last time (+8) a marker.
fn blend_interpolator(
    e: &mut Engine,
    case: &Case,
    [count, flags, gave, single_time]: [u32; 4],
    f: usize,
) -> u32 {
    let b = object(e, 0x30, 0, &[]);
    let vt = e.mem.u32(b);
    e.mem.set_u32(vt + 0x8c, BLEND_UPDATE);
    e.mem.set_u32(b + 8, 0x5555_5555);
    e.mem.set_u8(b + 0xc, flags as u8);
    e.mem.set_u8(b + 0xe, count as u8);
    let single = object(e, 0x10, gave, &transform_words(case, f));
    e.mem.set_u32(b + 0x18, single);
    e.mem.set_u32(b + 0x20, single_time);
    b
}

/// A transform (0x20 bytes) with the case's first float at `at` and ones elsewhere.
fn transform_with(e: &mut Engine, case: &Case, at: u32) -> u32 {
    let t = block(e, 0x20, &[ONE; 8]);
    e.mem.set_u32(t + at, bits(case, 0));
    t
}

trait Common {
    fn common(self) -> Diff;
}

impl Common for Diff {
    fn common(self) -> Diff {
        self.family(&animation::ANIMATION)
            .family(&dispatch::DISPATCH)
            .family(&pathing::PATHING)
            .family(&leaf::LEAVES)
            .max_calls(MAX_CALLS)
            .float_sweep(FLOAT_SWEEP)
    }
}

/// The bip node's controller lists, by the case's word: none; a multi-target transform controller; a
/// tread transform controller; another; and two orders of several.
const CHAINS: [&[u32]; 6] = [
    &[],
    &[MULTI_TARGET_RTTI],
    &[TREAD_RTTI],
    &[OTHER_RTTI],
    &[MULTI_TARGET_RTTI, TREAD_RTTI, OTHER_RTTI, MULTI_TARGET_RTTI],
    &[OTHER_RTTI, TREAD_RTTI, TREAD_RTTI],
];

#[test]
fn animation_update_all_but_bip() {
    // The controller list; the time; the transform. Each multi-target controller is active with three
    // bones, the first and last with flag 2.
    Diff::new(entry!(0x004f_0040, bip::update_all_but_bip(Ptr<_>, f32)))
        .common()
        .inputs(&inputs(&[
            &[Input::Of(&[4, 0, 1, 2, 3, 5]), Input::F32],
            &TRANSFORM,
        ]))
        .max_cases(256)
        .setup(|e, case| {
            stand_ins(e);
            set_logged_answer(e, TAG_TRANSLATE_VALID, 1);
            let node = block(e, 0x40, &[0; 16]);
            let mut next = 0;
            for (n, class) in CHAINS[case.word(0) as usize].iter().enumerate().rev() {
                let c = if *class == MULTI_TARGET_RTTI {
                    controller(e, case, [8, 3, 0b10_00_10, n as u32], 2)
                } else {
                    object(e, 0x48, *class, &[])
                };
                e.mem.set_u32(c + 0x30, next);
                next = c;
            }
            e.mem.set_u32(node + 0xc, next);
            vec![node, bits(case, 1)]
        })
        .run()
        .assert_same();
}

#[test]
fn animation_update_all_but_target() {
    // The controller's flags (8: active); the bone count; inverse; the bones (none, flag 2); the
    // interpolators' kinds; the translation's test; the time; the transform.
    Diff::new(entry!(
        0x004f_0380,
        bip::update_all_but_target(Ptr<_>, f32, u8)
    ))
    .common()
    .inputs(&inputs(&[
        &[
            Input::Of(&[8, 0, 0x0108, 0xfff7]),
            Input::Of(&[3, 0, 1, 2]),
            Input::Flag,
            Input::Of(&[
                0b10_10_10, 0, 0b00_10_00, 0b10_00_10, 0b01_10_10, 0b10_10_01, 0b01_01_01,
            ]),
            Input::Of(&[0, 1, 2, 3, 4]),
            Input::Flag,
            Input::F32,
        ],
        &TRANSFORM,
    ]))
    .max_cases(512)
    .setup(|e, case| {
        stand_ins(e);
        set_logged_answer(e, TAG_TRANSLATE_VALID, case.word(5));
        let w = [case.word(0), case.word(1), case.word(3), case.word(4)];
        let c = controller(e, case, w, 7);
        vec![c, bits(case, 6), case.word(2)]
    })
    .run()
    .assert_same();
}

#[test]
fn animation_controller_active() {
    // The word at +8: only its low 16 bits are the flags.
    Diff::new(entry!(0x004f_05a0, bip::controller_active(Ptr<_>) -> u32))
        .common()
        .inputs(&[Input::Of(&[
            8,
            0,
            0x0108,
            0xfff7,
            0x10,
            0x0008_0000,
            0xffff_ffff,
        ])])
        .setup(|e, case| {
            stand_ins(e);
            vec![block(e, 0x10, &[0, 0, case.word(0), 0])]
        })
        .run()
        .assert_same();
}

#[test]
fn animation_flags_share() {
    // The word at +8 and the mask: only the low 16 bits of each count.
    Diff::new(entry!(0x004f_0360, bip::flags_share(Ptr<_>, u32) -> u32))
        .common()
        .inputs(&[
            Input::Of(&[8, 0, 1, 0x100, 0x8000, 0xffff, 0x0001_0000, 0xffff_0000]),
            Input::Of(&[
                8,
                0,
                1,
                0x100,
                0x8000,
                0x0001_0000,
                0xffff_0100,
                0xffff_ffff,
            ]),
        ])
        .setup(|e, case| {
            stand_ins(e);
            vec![block(e, 0x10, &[0, 0, case.word(0), 0]), case.word(1)]
        })
        .run()
        .assert_same();
}

#[test]
fn animation_flag_2() {
    Diff::new(entry!(0x004f_0140, bip::flag_2(Ptr<_>) -> bool))
        .common()
        .inputs(&[Input::Of(&[
            2,
            0,
            1,
            3,
            0xffff_fffd,
            0xffff_ffff,
            0x0002_0000,
        ])])
        .setup(|e, case| {
            stand_ins(e);
            let o = block(e, 0x40, &[0; 16]);
            e.mem.set_u32(o + 0x30, case.word(0));
            vec![o]
        })
        .run()
        .assert_same();
}

#[test]
fn animation_rotation_valid() {
    // The rotation's x (+0x10), each operand class: the invalid marker, NaNs, infinities.
    Diff::new(entry!(0x004f_02e0, bip::rotation_valid(Ptr<_>) -> bool))
        .common()
        .inputs(&[Input::F32])
        .setup(|e, case| {
            stand_ins(e);
            vec![transform_with(e, case, 0x10)]
        })
        .run()
        .assert_same();
}

#[test]
fn animation_scale_valid() {
    // The scale (+0x1c), each operand class.
    Diff::new(entry!(0x004f_0320, bip::scale_valid(Ptr<_>) -> bool))
        .common()
        .inputs(&[Input::F32])
        .setup(|e, case| {
            stand_ins(e);
            vec![transform_with(e, case, 0x1c)]
        })
        .run()
        .assert_same();
}

#[test]
fn animation_set_rotation() {
    // The quaternion: w from the operand classes, the rest swept.
    Diff::new(entry!(0x004f_0160, bip::set_rotation(Ptr<_>, Ptr<_>)))
        .common()
        .inputs(&[
            Input::F32,
            Input::F32Swept(HALF),
            Input::F32Swept(THIRD),
            Input::F32Swept(MINUS_ONE),
        ])
        .setup(|e, case| {
            stand_ins(e);
            let bone = block(e, 0x70, &[0x3333_3333; 28]);
            let q = block(
                e,
                0x10,
                &[bits(case, 0), bits(case, 1), bits(case, 2), bits(case, 3)],
            );
            vec![bone, q]
        })
        .run()
        .assert_same();
}

#[test]
fn animation_to_rotation() {
    // The quaternion (w, x, y, z), each from the operand classes; floats.
    Diff::new(entry!(0x004f_0180, bip::to_rotation(Ptr<_>, Ptr<_>)))
        .common()
        .inputs(&[Input::F32, Input::F32, Input::F32, Input::F32])
        .max_cases(256)
        .float_sweep(64)
        .setup(|e, case| {
            stand_ins(e);
            let q = block(
                e,
                0x10,
                &[bits(case, 0), bits(case, 1), bits(case, 2), bits(case, 3)],
            );
            let m = block(e, 0x24, &[0x4444_4444; 9]);
            vec![q, m]
        })
        .run()
        .assert_same();
}

#[test]
fn animation_blend_update() {
    // The count (one: the single store; more: the blend); the blend's answer (0x100: a byte); the
    // flags; the single interpolator's answer; the single time (the invalid marker, NaN); the time;
    // the transform.
    Diff::new(entry!(
        0x00a4_1110,
        blend::update(Ptr<_>, f32, Ptr<_>, Ptr<_>) -> bool
    ))
    .common()
    .inputs(&inputs(&[
        &[
            Input::Of(&[1, 0, 2, 0x80, 0xff]),
            Input::Of(&[1, 0, 0x100]),
            Input::Of(&[0, 1, 0xfe]),
            Input::Of(&[1, 0, 0x100]),
            Input::Of(&[HALF, LOWEST, NAN]),
            Input::F32,
        ],
        &TRANSFORM,
    ]))
    .max_cases(512)
    .setup(|e, case| {
        stand_ins(e);
        set_logged_answer(e, TAG_BLEND, case.word(1));
        let w = [case.word(0), case.word(2), case.word(3), case.word(4)];
        let b = blend_interpolator(e, case, w, 6);
        let object = block(e, 0x40, &[0x7777_7777; 16]);
        let transform = block(e, 0x20, &[0x6666_6666; 8]);
        vec![b, bits(case, 5), object, transform]
    })
    .run()
    .assert_same();
}

#[test]
fn animation_store_single_value() {
    // The flags; the single interpolator's answer; the single time (the invalid marker, NaNs); the
    // time; the transform.
    Diff::new(entry!(
        0x00a4_0b30,
        blend::store_single_value(Ptr<_>, f32, Ptr<_>, Ptr<_>) -> bool
    ))
    .common()
    .inputs(&inputs(&[
        &[
            Input::Of(&[0, 1, 0xfe]),
            Input::Of(&[1, 0, 0x100]),
            Input::Of(&[HALF, LOWEST, NAN, SNAN]),
            Input::F32,
        ],
        &TRANSFORM,
    ]))
    .max_cases(512)
    .setup(|e, case| {
        stand_ins(e);
        let w = [1, case.word(0), case.word(1), case.word(2)];
        let b = blend_interpolator(e, case, w, 4);
        let object = block(e, 0x40, &[0x7777_7777; 16]);
        let transform = block(e, 0x20, &[0x6666_6666; 8]);
        vec![b, bits(case, 3), object, transform]
    })
    .run()
    .assert_same();
}
