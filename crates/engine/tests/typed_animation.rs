//! The differential test of every typed function of the animation family (`engine_typed::animation`)
//! against its translation. The world: a bip node with its controller list (each controller an object
//! whose vtable's stand-in slots answer its class's `NiRTTI` and log its `Update`); multi-target
//! transform controllers with their flags, bones (each with its flags at +0x30) and inline
//! interpolators: stand-ins whose transform `Update` (slot +0x8c) writes the case's transform, or blend
//! transform interpolators (the real `Update` in the slot) with their count, flags, single interpolator
//! and single time, and the items the blend of several reads (two stand-in interpolators, the second
//! with its w negated, and an empty item); the exe's constants.
//!
//! What stands in: the update data's and the transform's constructors, the transform's translation
//! test and its rotation and scale getters, the bone's translation and scale setters, the matrix's
//! column set and the square root (by address on both sides). Everything else runs translated, the
//! blend of several interpolators, the weights' normalisation and the quaternion's included.
//!
//! The interpolators, the blend and the weights are native at the running thread's precision, so
//! each comparison runs twice ([`on_both`]): under the other threads' control word with the case's
//! floats as they come, under the main thread's with them in play ([`fold`]: magnitudes far from
//! overflow and underflow, where the exe keeps the x87 exponent range and `f32` does not).
//!
//! The translations call the family's small functions with plain Rust calls (as fast as before), and
//! the typed versions call them as typed functions: each test lists them as callees the translation
//! calls plainly, so what they do still compares, not the call itself.

use engine as _;
use engine_core::prelude::*;
use engine_core::typed::diff::{Case, Diff, Input};
use engine_core::typed::{MAIN_THREAD, OTHER_THREADS};
use engine_typed::animation::{self, bip, blend};
use engine_typed::{dispatch, pathing};
use std::cell::Cell;

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

/// What the translations call with a plain Rust call: the flag 2 test, the rotation and scale
/// tests, the bone's rotation and `ToRotation` (`bsbipnode.rs`) and the single-value store
/// (`niblendtransforminterpolator.rs`).
const PLAIN: [u32; 6] = [
    0x004f_0140,
    0x004f_02e0,
    0x004f_0320,
    0x004f_0160,
    0x004f_0180,
    0x00a4_0b30,
];

/// `_CIsqrt` (typed in the C runtime family, which these tests leave out: it goes by address to the
/// stand-in on both sides) and `NiQuaternion::operator-`, which the blend does in place.
const SQRT: u32 = 0x00ec_6040;
const QUATERNION_NEGATE: u32 = 0x00a6_dbd0;

/// The answer a test sets: the translation's test.
const TAG_TRANSLATE_VALID: u32 = 0x01;

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

thread_local! {
    /// Whether the comparison running on this thread takes the case's floats in play ([`fold`]).
    static PLAY: Cell<bool> = const { Cell::new(false) };
}

/// A float in play: its sign and mantissa kept, its exponent folded into 2^-8 .. 1 (rotations
/// stay near unit quaternions, whose products never overflow); zeros,
/// infinities and NaNs as they come. (The unset marker `-FLT_MAX` folds too: the tests write it
/// where it is a marker, and its paths compare only, the same under both words.)
fn fold(bits: u32) -> u32 {
    let exponent = (bits >> 23) & 0xff;
    if exponent == 0xff || bits & 0x7fff_ffff == 0 {
        return bits;
    }
    (bits & 0x807f_ffff) | ((119 + exponent % 8) << 23)
}

/// The case's float `i`, in play when the running comparison says so.
fn bits(case: &Case, i: usize) -> u32 {
    let b = case.f32(i).to_bits();
    if PLAY.with(Cell::get) {
        fold(b)
    } else {
        b
    }
}

/// Runs the comparison `diff` builds under the other threads' control word with the case's floats as
/// they come, then under the main thread's with them in play.
fn on_both(diff: impl Fn() -> Diff) {
    for (word, play) in [(OTHER_THREADS, false), (MAIN_THREAD, true)] {
        PLAY.with(|p| p.set(play));
        let report = diff().control_words(&[word]).run();
        PLAY.with(|p| p.set(false));
        report.assert_same();
    }
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
        0x0101_2000,
        0x0101_5000,
        0x0102_3000,
        0x0102_4000,
        0x0103_0000,
        0x0109_6000,
        0x011a_8000,
        0x011f_3000,
        0x011f_4000,
    ] {
        e.map(page, 0x1000);
    }
    // 2.0 (a double), the invalid transform part, FLT_MAX, -FLT_MAX (a double) and
    // `NiInterpolator::INVALID_FLOAT`, as in the exe.
    e.mem.set_u64(0x0101_1590, 2.0f64.to_bits());
    e.mem.set_u32(0x0101_2054, MINUS_ONE);
    e.mem.set_u32(0x0101_5f5c, LOWEST);
    e.mem.set_u32(0x0102_3900, 0x7f7f_ffff);
    e.mem.set_u64(0x0102_41b0, f64::from(f32::MIN).to_bits());
    e.mem.set_u32(0x0109_696c, LOWEST);
    // The blend of several: 0.0, the double 0.0001, the sums' starting translation and rotation
    // (zero), and a fresh transform's translation, rotation and scale (the invalid marker).
    e.mem.set_u32(0x0101_1d78, 0);
    e.mem.set_u64(0x0103_0f28, 0.0001f64.to_bits());
    words(e, 0x011f_426c, &[0; 3]);
    words(e, 0x011f_49ec, &[0; 4]);
    words(e, 0x011a_8400, &[LOWEST; 3]);
    words(e, 0x011f_3704, &[LOWEST; 4]);
    e.mem.set_u32(0x0109_6b8c, LOWEST);
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
    // The square root (its double argument in two words, the result in ST0), by address on both
    // sides ([`Common::common`]).
    e.register(SQRT, |e, a| {
        log(e, 0x6040, a[1]);
        let x = f64::from_bits(u64::from(a[0]) | u64::from(a[1]) << 32);
        ret_f(x.sqrt())
    });
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

/// The items the blend of several reads at a blend transform interpolator `at` (+0xd the array
/// size, +0x14 the array, 0x18 bytes each: the interpolator, its weight at +8, its update time at
/// +0x14): a stand-in interpolator giving the case's transform with the weight 0.5, an empty item, and
/// one giving the transform with its w negated (so the sum's dot product can turn negative) with the
/// weight `second`; each update time 0.5.
fn blend_items(e: &mut Engine, case: &Case, at: u32, second: u32, f: usize) {
    let transform = transform_words(case, f);
    let mut negated = transform;
    negated[3] ^= 0x8000_0000;
    let first = object(e, 0x10, 1, &transform);
    let other = object(e, 0x10, 1, &negated);
    let items = e.mem.alloc(3 * 0x18);
    for (i, (interpolator, weight)) in (0u32..).zip([(first, HALF), (0, HALF), (other, second)]) {
        let item = items + 0x18 * i;
        words(e, item, &[interpolator, 0, weight]);
        e.mem.set_u32(item + 0x14, HALF);
    }
    e.mem.set_u8(at + 0xd, 3);
    e.mem.set_u32(at + 0x14, items);
}

/// The interpolator at `at` (0x30 bytes inline) by `kind`: 0, a stand-in giving the case's transform;
/// 1, one giving nothing (its answer 0x100, whose low byte is 0); 2, 3 and 4, a blend transform
/// interpolator (the real `Update` in its slot) with one interpolator (a stand-in giving the
/// transform), none, or two ([`blend_items`]).
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
    blend_items(e, case, at, HALF, f);
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
/// the case's transform), the single time (+0x20) and the items of the blend of several
/// ([`blend_items`], the second weighing `second`); the last time (+8) a marker.
fn blend_interpolator(
    e: &mut Engine,
    case: &Case,
    [count, flags, gave, single_time, second]: [u32; 5],
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
    blend_items(e, case, b, second, f);
    b
}

/// A transform (0x20 bytes) with the case's first float at `at` and ones elsewhere.
fn transform_with(e: &mut Engine, case: &Case, at: u32) -> u32 {
    let t = block(e, 0x20, &[ONE; 8]);
    e.mem.set_u32(t + at, bits(case, 0));
    t
}

/// What every test adds to its comparison: the families, the plain callees (all but the function
/// under test), floats within an ulp (the native float maths, ADR-0014), the call budget, the float
/// sweep.
trait Common {
    fn common(self, function: u32) -> Diff;
}

impl Common for Diff {
    fn common(self, function: u32) -> Diff {
        let plain: Vec<u32> = PLAIN.into_iter().filter(|c| *c != function).collect();
        self.family(&animation::ANIMATION)
            .family(&dispatch::DISPATCH)
            .family(&pathing::PATHING)
            .family(&leaf::LEAVES)
            .translation_calls_plainly(&plain)
            .goes_by_address(&[SQRT])
            .ignore_calls(&[QUATERNION_NEGATE])
            .allow_nan_payloads()
            .float_ulps(1)
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
    on_both(|| {
        Diff::new(entry!(0x004f_0040, bip::update_all_but_bip(Ptr<_>, f32)))
            .common(0x004f_0040)
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
    });
}

#[test]
fn animation_update_all_but_target() {
    // The controller's flags (8: active); the bone count; inverse; the bones (none, flag 2); the
    // interpolators' kinds; the translation's test; the time; the transform.
    on_both(|| {
        Diff::new(entry!(
            0x004f_0380,
            bip::update_all_but_target(Ptr<_>, f32, u8)
        ))
        .common(0x004f_0380)
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
    });
}

#[test]
fn animation_controller_active() {
    // The word at +8: only its low 16 bits are the flags.
    on_both(|| {
        Diff::new(entry!(0x004f_05a0, bip::controller_active(Ptr<_>) -> u32))
            .common(0x004f_05a0)
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
    });
}

#[test]
fn animation_flags_share() {
    // The word at +8 and the mask: only the low 16 bits of each count.
    on_both(|| {
        Diff::new(entry!(0x004f_0360, bip::flags_share(Ptr<_>, u32) -> u32))
            .common(0x004f_0360)
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
    });
}

#[test]
fn animation_flag_2() {
    on_both(|| {
        Diff::new(entry!(0x004f_0140, bip::flag_2(Ptr<_>) -> bool))
            .common(0x004f_0140)
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
    });
}

#[test]
fn animation_rotation_valid() {
    // The rotation's x (+0x10), each operand class: the invalid marker, NaNs, infinities.
    on_both(|| {
        Diff::new(entry!(0x004f_02e0, bip::rotation_valid(Ptr<_>) -> bool))
            .common(0x004f_02e0)
            .inputs(&[Input::F32])
            .setup(|e, case| {
                stand_ins(e);
                vec![transform_with(e, case, 0x10)]
            })
    });
}

#[test]
fn animation_scale_valid() {
    // The scale (+0x1c), each operand class.
    on_both(|| {
        Diff::new(entry!(0x004f_0320, bip::scale_valid(Ptr<_>) -> bool))
            .common(0x004f_0320)
            .inputs(&[Input::F32])
            .setup(|e, case| {
                stand_ins(e);
                vec![transform_with(e, case, 0x1c)]
            })
    });
}

#[test]
fn animation_set_rotation() {
    // The quaternion: w from the operand classes, the rest swept.
    on_both(|| {
        Diff::new(entry!(0x004f_0160, bip::set_rotation(Ptr<_>, Ptr<_>)))
            .common(0x004f_0160)
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
    });
}

#[test]
fn animation_to_rotation() {
    // The quaternion (w, x, y, z), each from the operand classes; floats.
    on_both(|| {
        Diff::new(entry!(0x004f_0180, bip::to_rotation(Ptr<_>, Ptr<_>)))
            .common(0x004f_0180)
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
    });
}

#[test]
fn animation_blend_update() {
    // The count (one: the single store; more: the blend of several); the blend's second item's weight
    // (none or negative: passed over); the flags; the single interpolator's answer; the single time
    // (the invalid marker, NaN); the time; the transform.
    on_both(|| {
        Diff::new(entry!(
            0x00a4_1110,
            blend::update_transform(Ptr<_>, f32, Ptr<_>, Ptr<_>) -> bool
        ))
        .common(0x00a4_1110)
        .inputs(&inputs(&[
            &[
                Input::Of(&[1, 0, 2, 0x80, 0xff]),
                Input::Of(&[HALF, 0, MINUS_ONE]),
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
            let w = [
                case.word(0),
                case.word(2),
                case.word(3),
                case.word(4),
                case.word(1),
            ];
            let b = blend_interpolator(e, case, w, 6);
            let object = block(e, 0x40, &[0x7777_7777; 16]);
            let transform = block(e, 0x20, &[0x6666_6666; 8]);
            vec![b, bits(case, 5), object, transform]
        })
    });
}

#[test]
fn animation_store_single_value() {
    // The flags; the single interpolator's answer; the single time (the invalid marker, NaNs); the
    // time; the transform.
    on_both(|| {
        Diff::new(entry!(
            0x00a4_0b30,
            blend::store_single_value(Ptr<_>, f32, Ptr<_>, Ptr<_>) -> bool
        ))
        .common(0x00a4_0b30)
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
            let w = [1, case.word(0), case.word(1), case.word(2), HALF];
            let b = blend_interpolator(e, case, w, 4);
            let object = block(e, 0x40, &[0x7777_7777; 16]);
            let transform = block(e, 0x20, &[0x6666_6666; 8]);
            vec![b, bits(case, 3), object, transform]
        })
    });
}

// ---- The interpolators, the blend of several, the weights and the normalisation ----

/// What the translation of `NiTransformInterpolator::Update` calls by address and the typed version
/// does in place: the data's three key getters and the three `GenInterp`s (and so, below them, the
/// key types' functions, `Slerp`, the quadrangle interpolation and `FastNormalize`).
const TRANSFORM_IN_PLACE: [u32; 6] = [
    0x00a3_f990,
    0x0058_cb00,
    0x00a3_f9e0,
    0x00a2_7490,
    0x00a2_8740,
    0x00a2_6b40,
];
/// The key sizes per key type, for position, rotation and float keys (what each type's function
/// reads, as in `typed_anim_keys.rs`).
const POINT_SIZES: [u32; 6] = [0x10, 0x10, 0x40, 0x4c, 0x10, 0x10];
const ROTATION_SIZES: [u32; 6] = [0x14, 0x14, 0x24, 0x40, 0x14, 0x14];
const FLOAT_SIZES: [u32; 6] = [8, 8, 0x10, 0x1c, 8, 8];
/// Times against keys at `0.25 + k (1 + k / 8)`: the interpolators' last time (1.0), before the
/// first key, on it, on key 2, between keys, past the last (key 11 is at 26.375), NaN, -FLT_MAX.
const TIMES: &[u32] = &[
    ONE,
    0x0000_0000,
    0x3e80_0000,
    0x4030_0000,
    0x4110_0000,
    0x41f0_0000,
    NAN,
    LOWEST,
];
/// Eight floats swept along by the float sweep, plain in the ordinary cases.
const SWEPT8: [Input; 8] = [
    Input::F32Swept(0x3f9d_70a4),
    Input::F32Swept(0xbf40_0000),
    Input::F32Swept(THIRD),
    Input::F32Swept(0x4049_0fdb),
    Input::F32Swept(0xbe4c_cccd),
    Input::F32Swept(0x3f35_04f3),
    Input::F32Swept(0x40a0_0000),
    Input::F32Swept(0xbfb3_3333),
];

/// The keys' and quaternions' constants (-1.0, 0.5, 3.0, the parameter correction's factors,
/// `FastNormalize`'s centre, limits, slope and base, the float interpolators' invalid value) and the
/// key types' tables holding the exe's functions, over [`stand_ins`].
fn key_world(e: &mut Engine) {
    stand_ins(e);
    for page in [
        0x0101_6000,
        0x0102_1000,
        0x0109_9000,
        0x0109_a000,
        0x0109_c000,
    ] {
        e.map(page, 0x1000);
    }
    e.mem.set_u32(0x0101_6248, HALF);
    e.mem.set_u64(0x0102_1928, 3.0f64.to_bits());
    e.mem.set_u64(0x0109_c9d8, 0x3fea_545a_2000_0000);
    e.mem.set_u64(0x0109_c9d0, 0x3fe2_bc5a_2000_0000);
    let center = f64::from_bits(0x3fee_b0ab_2000_0000);
    e.mem.set_u64(0x0109_9680, center.to_bits());
    e.mem.set_u64(0x0109_9690, 0x3fed_496a_a000_0000);
    e.mem.set_u32(0x0109_9688, 0x3f26_f151);
    e.mem
        .set_u32(0x011f_3fec, ((-0.5 * center.powf(-1.5)) as f32).to_bits());
    e.mem
        .set_u32(0x011f_3ff0, ((1.0 / center.sqrt()) as f32).to_bits());
    e.mem.set_u32(0x0109_a918, LOWEST);
    let rows: [(u32, [u32; 6]); 3] = [
        (
            0x011f_3c90,
            [
                0x00a2_9680,
                0x00a2_a560,
                0x00a2_6fe0,
                0x00a2_c610,
                0,
                0x00a2_d200,
            ],
        ),
        (
            0x011f_3ca8,
            [0x00a2_7470, 0x00a2_a7f0, 0x00a2_7c80, 0x00a2_af70, 0, 0],
        ),
        (
            0x011f_3cc0,
            [
                0x00a2_86e0,
                0x00a2_ab90,
                0x00a2_8d90,
                0x00a2_bc60,
                0,
                0x00a2_cf20,
            ],
        ),
    ];
    for (row, w) in rows {
        words(e, row, &w);
    }
}

/// 14 keys of `size` bytes: key `k`'s time `0.25 + k (1 + k / 8)`, every word after it one of the
/// case's eight floats from `f` in turn.
fn spread_keys(e: &mut Engine, case: &Case, f: usize, size: u32) -> u32 {
    let keys = e.mem.alloc(size * 14);
    let mut n = 0usize;
    for k in 0..14u32 {
        let key = keys + k * size;
        let time = 0.25 + k as f32 * (1.0 + k as f32 / 8.0);
        e.mem.set_u32(key, time.to_bits());
        for offset in (4..size).step_by(4) {
            e.mem.set_u32(key + offset, bits(case, f + n % 8));
            n += 3;
        }
    }
    keys
}

/// What every new test adds: the family, the square root by address, the negation in place, NaN
/// payloads, the call budget, the float sweep. Floats compare bit for bit.
trait Fresh {
    fn fresh(self) -> Diff;
}

impl Fresh for Diff {
    fn fresh(self) -> Diff {
        self.family(&animation::ANIMATION)
            .goes_by_address(&[SQRT])
            .ignore_calls(&[QUATERNION_NEGATE])
            .allow_nan_payloads()
            .max_calls(MAX_CALLS)
            .float_sweep(FLOAT_SWEEP)
    }
}

/// An `NiTransformData`'s key lists (type, count) for translation, rotation and scale: linear,
/// Bezier, TCB, the empty and step types, lists without keys and with one key.
const LAYOUTS: [[(u32, u32); 3]; 7] = [
    [(1, 12), (1, 12), (1, 12)],
    [(2, 5), (2, 5), (2, 5)],
    [(3, 12), (3, 12), (3, 12)],
    [(0, 2), (5, 2), (5, 2)],
    [(1, 12), (0, 0), (0, 0)],
    [(1, 1), (1, 1), (1, 1)],
    [(2, 0), (3, 12), (0, 2)],
];

#[test]
fn animation_transform_update() {
    // The time; a pose or not; data or none; the key lists' layout; the last indices; the transform
    // value's parts set or all unset; the keys' floats.
    on_both(|| {
        Diff::new(entry!(
            0x00a3_fdb0,
            animation::interp::transform_update(Ptr<_>, f32, u32, Ptr<_>) -> bool
        ))
        .fresh()
        .ignore_calls(&TRANSFORM_IN_PLACE)
        .inputs(&inputs(&[
            &[
                Input::Of(TIMES),
                Input::Of(&[0, 1]),
                Input::Of(&[1, 0]),
                Input::Of(&[0, 1, 2, 3, 4, 5, 6]),
                Input::Of(&[0, 5, 11]),
                Input::Of(&[0, 1]),
            ],
            &SWEPT8,
        ]))
        .max_cases(1400)
        .setup(|e, case| {
            key_world(e);
            let layout = LAYOUTS[case.word(3) as usize];
            let data = if case.word(2) == 1 {
                let d = e.mem.alloc(0x30);
                let lists: [(u32, u32, u32, u32, &[u32; 6]); 3] = [
                    (0xa, 0x14, 0x1d, 0x24, &POINT_SIZES),
                    (0x8, 0x10, 0x1c, 0x20, &ROTATION_SIZES),
                    (0xc, 0x18, 0x1e, 0x28, &FLOAT_SIZES),
                ];
                for ((count, kind, size, keys, sizes), (key_type, n)) in
                    lists.into_iter().zip(layout)
                {
                    let key_size = sizes[key_type as usize];
                    e.mem.set_u16(d + count, n as u16);
                    e.mem.set_u32(d + kind, key_type);
                    e.mem.set_u8(d + size, key_size as u8);
                    let array = spread_keys(e, case, 6, key_size);
                    e.mem.set_u32(d + keys, array);
                }
                d
            } else {
                0
            };
            let t = e.mem.alloc(0x48);
            e.mem.set_u32(t + 8, ONE);
            let value: [u32; 8] = if case.word(5) == 1 {
                [LOWEST; 8]
            } else {
                core::array::from_fn(|i| bits(case, 6 + i))
            };
            words(e, t + 0xc, &value);
            e.mem.set_u32(t + 0x2c, data);
            for offset in [0x30, 0x32, 0x34] {
                e.mem.set_u16(t + offset, case.word(4) as u16);
            }
            e.mem.set_u8(t + 0x44, case.word(1) as u8);
            let out = block(e, 0x20, &[0x6666_6666; 8]);
            vec![t, case.word(0), 0x1234, out]
        })
    });
}

#[test]
fn animation_key_getters() {
    // Data or none: each getter's outputs and answer.
    for f in [
        entry!(0x00a3_f990, animation::interp::translate_keys(Ptr, Ptr, Ptr, Ptr) -> u32),
        entry!(0x0058_cb00, animation::interp::rotate_keys(Ptr, Ptr, Ptr, Ptr) -> u32),
        entry!(0x00a3_f9e0, animation::interp::scale_keys(Ptr, Ptr, Ptr, Ptr) -> u32),
    ] {
        on_both(|| {
            Diff::new(f)
                .fresh()
                .ignore_calls(&[0x0055_9450])
                .inputs(&[Input::Of(&[1, 0])])
                .setup(|e, case| {
                    stand_ins(e);
                    let d = block(e, 0x30, &[0; 12]);
                    for i in 0..12u32 {
                        e.mem.set_u32(d + 4 * i, 0x8100_0001 + 0x0102_0304 * i);
                    }
                    let t = block(e, 0x30, &[0; 12]);
                    e.mem
                        .set_u32(t + 0x2c, if case.word(0) == 1 { d } else { 0 });
                    let outs = block(e, 0x10, &[0x5555_5555; 4]);
                    vec![t, outs, outs + 4, outs + 8]
                })
        });
    }
}

#[test]
fn animation_float_update() {
    // The time; data or none; the key type; the count; the last index; the value a float or the
    // invalid one; the keys' floats.
    on_both(|| {
        Diff::new(entry!(
            0x00a3_c6d0,
            animation::interp::float_update(Ptr<_>, f32, u32, Ptr<_>) -> bool
        ))
        .fresh()
        .ignore_calls(&[0x00a2_6b40])
        .inputs(&inputs(&[
            &[
                Input::Of(TIMES),
                Input::Of(&[1, 0]),
                Input::Of(&[1, 2, 3, 5, 0]),
                Input::Of(&[12, 1, 0]),
                Input::Of(&[0, 4, 11]),
                Input::Of(&[0, 1]),
            ],
            &SWEPT8,
        ]))
        .max_cases(1500)
        .setup(|e, case| {
            key_world(e);
            let key_type = case.word(2);
            let size = FLOAT_SIZES[key_type as usize];
            let data = if case.word(1) == 1 {
                let keys = spread_keys(e, case, 6, size);
                block(e, 0x18, &[0, 0, case.word(3), keys, key_type, size])
            } else {
                0
            };
            let value = if case.word(5) == 1 {
                LOWEST
            } else {
                bits(case, 7)
            };
            let t = block(e, 0x18, &[0, 0, ONE, value, data, case.word(4)]);
            let out = block(e, 4, &[0x6666_6666]);
            vec![t, case.word(0), 0x1234, out]
        })
    });
}

/// A blend transform interpolator (0x30 bytes) of four items for the blend of several: item `i`'s
/// four bits of `pattern` (from bit `4i`): 1 no interpolator, 2 a weight of 0 (or -0.5 for odd
/// items), 4 its interpolator gives nothing, 8 its update time the invalid one. Its interpolator gives
/// the case's transform (its rotation negated for odd items, so a dot product turns negative) with
/// the three bits of `parts` from bit `3i` setting the translation, the rotation's `X` and the scale
/// to the unset marker; its weight one of 0.5, 1/3, 0.25 and 1.
fn blend_of_several(e: &mut Engine, case: &Case, [pattern, parts, flags, size]: [u32; 4]) -> u32 {
    let b = block(e, 0x30, &[0x5555_5555; 12]);
    e.mem.set_u8(b + 0xc, flags as u8);
    e.mem.set_u8(b + 0xd, size as u8);
    let items = e.mem.alloc(4 * 0x18);
    for i in 0..4u32 {
        let item = items + 0x18 * i;
        let bits4 = (pattern >> (4 * i)) & 0xf;
        let unset = (parts >> (3 * i)) & 7;
        let mut t = transform_words(case, 5);
        if i % 2 == 1 {
            for w in &mut t[3..7] {
                *w ^= 0x8000_0000;
            }
        }
        if unset & 1 != 0 {
            t[0] = LOWEST;
        }
        if unset & 2 != 0 {
            t[4] = LOWEST;
        }
        if unset & 4 != 0 {
            t[7] = LOWEST;
        }
        let interpolator = if bits4 & 1 != 0 {
            0
        } else {
            object(e, 0x10, if bits4 & 4 != 0 { 0x100 } else { 1 }, &t)
        };
        let weight = if bits4 & 2 != 0 {
            [0, 0xbf00_0000][i as usize % 2]
        } else {
            [HALF, THIRD, 0x3e80_0000, ONE][i as usize]
        };
        let time = if bits4 & 8 != 0 { LOWEST } else { HALF };
        words(e, item, &[interpolator, 0x7777_7777, weight, 0, 0, time]);
    }
    e.mem.set_u32(b + 0x14, items);
    b
}

#[test]
fn animation_blend_values() {
    // Each item's pattern (none, no weight, nothing given, the invalid time); the parts unset; the
    // flags (manager controlled); the array size; the time; the transforms' floats.
    on_both(|| {
        Diff::new(entry!(
            0x00a4_0c10,
            blend::blend_values(Ptr<_>, f32, Ptr<_>, Ptr<_>) -> bool
        ))
        .fresh()
        .inputs(&inputs(&[
            &[
                Input::Of(&[
                    0x0000, 0x0001, 0x0024, 0x8421, 0x0f00, 0x2222, 0x4444, 0x1248, 0xffff,
                ]),
                Input::Of(&[0, 0o111, 0o222, 0o444, 0o7777, 0o1234]),
                Input::Of(&[0, 1]),
                Input::Of(&[4, 0, 2]),
                Input::Of(&[HALF, LOWEST, NAN]),
            ],
            &TRANSFORM,
        ]))
        .max_cases(1000)
        .setup(|e, case| {
            stand_ins(e);
            let w = [case.word(0), case.word(1), case.word(2), case.word(3)];
            let b = blend_of_several(e, case, w);
            let object = block(e, 0x40, &[0x7777_7777; 16]);
            let transform = block(e, 0x20, &[0x6666_6666; 8]);
            vec![b, case.word(4), object, transform]
        })
    });
}

/// A blend interpolator (0x30 bytes) for the weights: the flags, the interpolator count, the single
/// index 1, the priorities 3 (high) and 1 (next high), the threshold, the high sum (unset or 0.5, with
/// the next-high sum and the spinner from the case's floats); 256 items (the exe may write item
/// 0xff), the first four with an interpolator unless `nulls` has their bit, the case's weights and
/// ease spinners, the priority bytes of `priorities`, a marker normalized weight.
fn weights_blend(
    e: &mut Engine,
    case: &Case,
    [flags, count, nulls, priorities, high, threshold]: [u32; 6],
) -> u32 {
    let b = block(e, 0x30, &[0x5555_5555; 12]);
    e.mem.set_u8(b + 0xc, flags as u8);
    e.mem.set_u8(b + 0xd, 4);
    e.mem.set_u8(b + 0xe, count as u8);
    e.mem.set_u8(b + 0xf, 1);
    e.mem.set_u8(b + 0x10, 3);
    e.mem.set_u8(b + 0x11, 1);
    e.mem.set_u32(b + 0x1c, threshold);
    e.mem.set_u32(b + 0x24, high);
    e.mem.set_u32(b + 0x28, bits(case, WEIGHT_FLOATS + 6));
    e.mem.set_u32(b + 0x2c, bits(case, WEIGHT_FLOATS + 7));
    let items = e.mem.alloc(256 * 0x18);
    for i in 0..4u32 {
        let item = items + 0x18 * i;
        let interpolator = if (nulls >> i) & 1 != 0 {
            0
        } else {
            0x00ff_4000
        };
        let weight = bits(case, WEIGHT_FLOATS + i as usize);
        let ease = if i == 3 {
            ONE
        } else {
            bits(case, WEIGHT_FLOATS + 4 + i as usize % 2)
        };
        words(
            e,
            item,
            &[
                interpolator,
                weight,
                0x5a5a_5a5a,
                (priorities >> (8 * i)) & 0xff,
                ease,
                0,
            ],
        );
    }
    e.mem.set_u32(b + 0x14, items);
    b
}

/// Where the weights' floats start in their tests' cases (after six chosen words).
const WEIGHT_FLOATS: usize = 6;

/// The weights' floats: four weights and two ease spinners from the operand classes, the next-high
/// sum and spinner swept.
const WEIGHTS: [Input; 8] = [
    Input::F32,
    Input::F32Swept(HALF),
    Input::F32Swept(THIRD),
    Input::F32Swept(0x3e80_0000),
    Input::F32,
    Input::F32Swept(0x3f40_0000),
    Input::F32Swept(ONE),
    Input::F32Swept(0x3f00_0000),
];

#[test]
fn animation_compute_normalized_weights() {
    // The flags (4: out of date, 2: only the highest); the count (1 and 2 their own cases); the
    // items without an interpolator; the priorities; the high sum unset or not; the threshold.
    on_both(|| {
        Diff::new(entry!(
            0x00a3_7260,
            animation::weights::compute_normalized_weights(Ptr<_>)
        ))
        .fresh()
        .inputs(&inputs(&[
            &[
                Input::Of(&[4, 6, 0, 5]),
                Input::Of(&[3, 1, 2, 0, 4]),
                Input::Of(&[0, 0b0010, 0b1001, 0b1111]),
                Input::Of(&[0x0301_0003, 0x0102_0301, 0x0505_0505, 0xff01_00ff]),
                Input::Of(&[LOWEST, HALF]),
                Input::Of(&[0, 0x3dcc_cccd, 0x3e99_999a]),
            ],
            &WEIGHTS,
        ]))
        .max_cases(2000)
        .setup(|e, case| {
            stand_ins(e);
            let mut w: [u32; 6] = core::array::from_fn(|i| case.word(i));
            if w[1] == 2 {
                // Two used items at least (the exe reads through null with fewer).
                w[2] &= 0b0011;
            }
            vec![weights_blend(e, case, w)]
        })
    });
}

#[test]
fn animation_compute_for_two() {
    // The first item missing or not; the priorities (A below, above, equal, -1 below 1); an ease spinner of 1
    // (A's, B's) or the case's; both weights zero or not; the flags (2: only the highest); the
    // threshold.
    on_both(|| {
        Diff::new(entry!(
            0x00a3_6bd0,
            animation::weights::compute_for_two(Ptr<_>)
        ))
        .fresh()
        .inputs(&inputs(&[
            &[
                Input::Of(&[0, 1]),
                Input::Of(&[0, 1, 2, 3]),
                Input::Of(&[0, 1, 2]),
                Input::Of(&[0, 1]),
                Input::Of(&[0, 2]),
                Input::Of(&[0, 0x3e99_999a, 0x3f19_999a]),
            ],
            &WEIGHTS,
        ]))
        .setup(|e, case| {
            stand_ins(e);
            let nulls = case.word(0);
            let flags = case.word(4);
            let w = [flags, 2, nulls, 0, LOWEST, case.word(5)];
            let b = weights_blend(e, case, w);
            let items = e.mem.u32(b + 0x14);
            let (a, second) = if nulls == 1 {
                (items + 0x18, items + 0x30)
            } else {
                (items, items + 0x18)
            };
            let (pa, pb) = [(1u8, 3u8), (3, 1), (3, 3), (0xff, 1)][case.word(1) as usize];
            e.mem.set_u8(a + 0xc, pa);
            e.mem.set_u8(second + 0xc, pb);
            match case.word(2) {
                1 => e.mem.set_u32(a + 0x10, ONE),
                2 => e.mem.set_u32(second + 0x10, ONE),
                _ => {}
            }
            if case.word(3) == 1 {
                e.mem.set_u32(a + 4, 0);
                e.mem.set_u32(second + 4, 0x8000_0000);
            }
            vec![b]
        })
    });
}

#[test]
fn animation_normalize() {
    // The quaternion's four floats.
    on_both(|| {
        Diff::new(entry!(0x00a3_8560, blend::normalize(Ptr<_>)))
            .fresh()
            .inputs(&[Input::F32, Input::F32, Input::F32, Input::F32])
            .setup(|e, case| {
                stand_ins(e);
                // At most one NaN: x87 and SSE pick between two NaNs' payloads differently, and the
                // root's double argument carries the payload.
                let q = e.mem.alloc(0x10);
                let mut nan = false;
                for i in 0..4u32 {
                    let mut b = bits(case, i as usize);
                    if f32::from_bits(b).is_nan() {
                        if nan {
                            b = HALF;
                        }
                        nan = true;
                    }
                    e.mem.set_u32(q + 4 * i, b);
                }
                vec![q]
            })
    });
}
