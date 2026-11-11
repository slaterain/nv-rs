//! The differential test of every typed function of the animation keys family
//! (`engine_typed::anim_keys`) against its translation: here because the translations live in the part
//! crates, and this crate registers them all. Quaternions and key arrays are filled from the case's
//! floats; the exe's constants these functions read are written at their addresses.
//!
//! What stands in: the key types' interpolation function (the table at `011f3cc0`; it writes a
//! quaternion of its first three words and 1.0, so what it was handed shows in the result) and
//! `NiQuaternion::FastNormalize` (`00a4ff00`; it flips `W`'s sign, so the call and what it saw show).
//! Everything else runs translated.
//!
//! `Slerp`, its parameter correction and `GenInterp` are native maths (`f64` locals rounded where the
//! exe stores a float), so each is compared twice: under the other threads' control word (53-bit
//! precision) bit for bit, NaN payloads aside, on every input; under the main thread's (24-bit
//! precision: the exe rounds every operation to 24 bits) within a stated tolerance, on the values
//! these functions get in play ([`in_play`]; outside them a result that cancels can land
//! arbitrarily many ulps away: the correction at `t` 1 + 2^-23 with a cosine of 2^31, which no unit
//! quaternion has). With `--nocapture` the main thread's comparisons print what they let through.

use engine as _;
use engine_core::prelude::*;
use engine_core::typed::diff::{Case, Diff, Input, Value};
use engine_core::typed::{MAIN_THREAD, OTHER_THREADS};
use engine_typed::anim_keys::{self, quaternion, rot_keys};

/// `NiQuaternion::FastNormalize`, which `Slerp` calls by address.
const FAST_NORMALIZE: u32 = 0x00a4_ff00;
/// The interpolation function's stand-in (past `.text`) and the table of the key types' functions.
const INTERPOLATE: u32 = 0x00ff_3000;
const INTERPOLATIONS: u32 = 0x011f_3cc0;
const FLOAT_SWEEP: usize = 64;

/// Eight floats, each swept.
const FLOATS8: [Input; 8] = [Input::F32; 8];

fn inputs(parts: &[&[Input]]) -> Vec<Input> {
    parts.iter().flat_map(|p| p.iter().copied()).collect()
}

/// The exe's constants: 0.0 and -1.0 (the rotation keys' sign test and clamp), 0.5 (`Slerp`'s
/// mirror), 3.0, the correction's two factors (doubles), `-FLT_MAX` as a double (the lowest time).
fn constants(e: &mut Engine) {
    for page in [
        0x0101_1000,
        0x0101_2000,
        0x0101_6000,
        0x0102_1000,
        0x0102_4000,
        0x0109_c000,
        0x011f_3000,
    ] {
        e.map(page, 0x1000);
    }
    e.mem.set_u32(0x0101_1d78, 0.0f32.to_bits());
    e.mem.set_u32(0x0101_2054, (-1.0f32).to_bits());
    e.mem.set_u32(0x0101_6248, 0.5f32.to_bits());
    e.mem.set_u64(0x0102_1928, 3.0f64.to_bits());
    e.mem.set_u64(0x0102_41b0, f64::from(f32::MIN).to_bits());
    e.mem.set_u64(0x0109_c9d8, 0x3fea_545a_2000_0000);
    e.mem.set_u64(0x0109_c9d0, 0x3fe2_bc5a_2000_0000);
}

/// The stand-ins, and the interpolation function in the table for key types 1 and 4.
fn stand_ins(e: &mut Engine) {
    e.register(FAST_NORMALIZE, |e, a| {
        let w = e.mem.u32(a[0]);
        e.mem.set_u32(a[0], w ^ 0x8000_0000);
        Ret::default()
    });
    e.register(INTERPOLATE, |e, a| {
        for (i, w) in [a[0], a[1], a[2], 1.0f32.to_bits()].into_iter().enumerate() {
            e.mem.set_u32(a[3] + 4 * i as u32, w);
        }
        Ret::default()
    });
    for key_type in [1u32, 4] {
        e.mem.set_u32(INTERPOLATIONS + 4 * key_type, INTERPOLATE);
    }
}

/// A quaternion of the case's floats `first..first + 4`.
fn quaternion_of(e: &mut Engine, case: &Case, first: usize) -> u32 {
    let q = e.mem.alloc(0x10);
    for i in 0..4 {
        e.mem
            .set_u32(q + 4 * i as u32, case.f32(first + i).to_bits());
    }
    q
}

/// `count` rotation keys of `size` bytes (room for 12): key `k`'s time `0.25 + k (1 + k spread)`
/// (`k + 0.25` with no spread; with a spread the spans differ and are no powers of two, so the
/// ratio's division rounds), its quaternion from the case's eight floats at `first` (in turn, so
/// neighbours differ), every third key's negated (so neighbours fall in both hemispheres), and the
/// rest of a key's bytes a pattern of its own (so a copy of them shows).
fn rot_keys_of(e: &mut Engine, case: &Case, first: usize, size: u32, spread: f32) -> u32 {
    let keys = e.mem.alloc(size * 12);
    for k in 0..12u32 {
        let key = keys + k * size;
        let time = 0.25 + k as f32 * (1.0 + k as f32 * spread);
        e.mem.set_u32(key, time.to_bits());
        for j in 0..4u32 {
            let float = case.f32(first + ((k * 4 + j) % 8) as usize).to_bits();
            let sign = if k % 3 == 2 { 0x8000_0000 } else { 0 };
            e.mem.set_u32(key + 4 + 4 * j, float ^ sign);
        }
        for offset in (0x14..size).step_by(4) {
            e.mem.set_u32(key + offset, 0x5a00_0000 | (k << 8) | offset);
        }
    }
    keys
}

/// What every test adds to its comparison: the family and the float sweep.
trait Common {
    fn common(self) -> Diff;
}

impl Common for Diff {
    fn common(self) -> Diff {
        self.family(&anim_keys::ANIM_KEYS).float_sweep(FLOAT_SWEEP)
    }
}

/// The case's floats as these functions get them in play: a quaternion's components, a parameter
/// and a cosine from 2^-8 to 1 in magnitude (the float's sign and mantissa kept, its exponent
/// folded into that range); zeros, infinities and NaNs as they come.
fn in_play(case: &Case) -> Case {
    let fold = |bits: u32| {
        let exponent = (bits >> 23) & 0xff;
        if exponent == 0xff || bits & 0x7fff_ffff == 0 {
            return bits;
        }
        (bits & 0x807f_ffff) | ((119 + exponent % 8) << 23)
    };
    Case(
        case.0
            .iter()
            .map(|v| match *v {
                Value::F32(b) => Value::F32(fold(b)),
                other => other,
            })
            .collect(),
    )
}

/// The case as a setup reads it: its floats [`in_play`] or as they came.
fn case_of(case: &Case, play: bool) -> Case {
    if play {
        in_play(case)
    } else {
        case.clone()
    }
}

// ---- The quaternion ----

#[test]
fn anim_keys_from_floats() {
    // The four floats.
    Diff::new(entry!(
        0x00a6_db50,
        quaternion::from_floats(Ptr<_>, f32, f32, f32, f32) -> Ptr<_>
    ))
    .common()
    .inputs(&[Input::F32, Input::F32, Input::F32, Input::F32])
    .setup(|e, case| {
        let q = e.mem.alloc(0x10);
        let mut words = vec![q];
        words.extend((0..4).map(|i| case.f32(i).to_bits()));
        words
    })
    .run()
    .assert_same();
}

#[test]
fn anim_keys_negate() {
    // The quaternion's floats; the result apart from it or over it.
    Diff::new(entry!(
        0x00a6_dbd0,
        quaternion::negate(Ptr<_>, Ptr<_>) -> Ptr<_>
    ))
    .common()
    .inputs(&inputs(&[&FLOATS8[..4], &[Input::Of(&[0, 1])]]))
    .setup(|e, case| {
        let q = quaternion_of(e, case, 0);
        let result = if case.word(4) == 1 {
            q
        } else {
            e.mem.alloc(0x10)
        };
        vec![q, result]
    })
    .run()
    .assert_same();
}

/// The correction's words: the parameter and the cosine, as they came or [`in_play`].
fn counter_warp_words(e: &mut Engine, case: &Case, play: bool) -> Vec<u32> {
    let case = case_of(case, play);
    constants(e);
    vec![case.f32(0).to_bits(), case.f32(1).to_bits()]
}

#[test]
fn anim_keys_counter_warp() {
    // The parameter; the cosine. Under the other threads' control word bit for bit, every input.
    Diff::new(entry!(
        0x00a6_dc30,
        quaternion::counter_warp(f32, f32) -> f64
    ))
    .common()
    .control_words(&[OTHER_THREADS])
    .allow_nan_payloads()
    .inputs(&[Input::F32, Input::F32])
    .setup(|e, case| counter_warp_words(e, case, false))
    .run()
    .assert_same();
    // Under the main thread's, in play, within 4 ulps: the exe rounds `1 - cos * 0.8228` to 24 bits
    // before the store rounds it again, and each step after it.
    let report = Diff::new(entry!(
        0x00a6_dc30,
        quaternion::counter_warp(f32, f32) -> f64
    ))
    .common()
    .control_words(&[MAIN_THREAD])
    .float_ulps(4)
    .inputs(&[Input::F32, Input::F32])
    .setup(|e, case| counter_warp_words(e, case, true))
    .run();
    println!("counter_warp on the main thread: {report}");
    report.assert_same();
}

/// `Slerp`'s inputs: the parameter; the result apart, over p or over q; p's and q's floats.
fn slerp_inputs() -> Vec<Input> {
    inputs(&[&[Input::F32, Input::Of(&[0, 1, 2])], &FLOATS8])
}

/// `Slerp`'s words, its floats as they came or [`in_play`].
fn slerp_words(e: &mut Engine, case: &Case, play: bool) -> Vec<u32> {
    let case = &case_of(case, play);
    constants(e);
    stand_ins(e);
    let p = quaternion_of(e, case, 2);
    let q = quaternion_of(e, case, 6);
    let result = match case.word(1) {
        1 => p,
        2 => q,
        _ => e.mem.alloc(0x10),
    };
    vec![result, case.f32(0).to_bits(), p, q]
}

#[test]
fn anim_keys_slerp() {
    // Under the other threads' control word bit for bit, every input. The translation calls the
    // parameter correction as a plain Rust call.
    Diff::new(entry!(
        0x00a6_e330,
        quaternion::slerp(Ptr<_>, f32, Ptr<_>, Ptr<_>)
    ))
    .common()
    .translation_calls_plainly(&[0x00a6_dc30])
    .control_words(&[OTHER_THREADS])
    .allow_nan_payloads()
    .inputs(&slerp_inputs())
    .setup(|e, case| slerp_words(e, case, false))
    .run()
    .assert_same();
    // Under the main thread's, in play, within 16 ulps, or within 2^-23 (one ulp of 1.0) where a
    // component cancels to near zero: `p + k (q - p)` for a component crossing zero is -2^-26
    // natively where the exe's 24-bit arithmetic gives -2^-25, 2^23 ulps apart and 1.5e-8 next to
    // the unit quaternion around it.
    let report = Diff::new(entry!(
        0x00a6_e330,
        quaternion::slerp(Ptr<_>, f32, Ptr<_>, Ptr<_>)
    ))
    .common()
    .translation_calls_plainly(&[0x00a6_dc30])
    .control_words(&[MAIN_THREAD])
    .float_ulps(16)
    .float_abs(2f32.powi(-23))
    .inputs(&slerp_inputs())
    .setup(|e, case| slerp_words(e, case, true))
    .run();
    println!("slerp on the main thread: {report}");
    report.assert_same();
}

// ---- The rotation keys ----

#[test]
fn anim_keys_prepare() {
    // The count; the key size (only its low byte counts); the keys' floats.
    Diff::new(entry!(0x00a2_8ab0, rot_keys::prepare(Ptr, u32, u8)))
        .common()
        .inputs(&inputs(&[
            &[
                Input::Of(&[5, 1, 2, 3, 4, 8, 9]),
                Input::Of(&[0x14, 0x24, 0x0001_0014]),
            ],
            &FLOATS8,
        ]))
        .setup(|e, case| {
            constants(e);
            let keys = rot_keys_of(e, case, 2, case.word(1) & 0xff, 0.0);
            vec![keys, case.word(0), case.word(1)]
        })
        .run()
        .assert_same();
}

/// The times `GenInterp` is tried at, as float bits, around the keys (key `k` at `k + 0.25`): between
/// two keys (6.0 and 7.0: from the last index 3 the unrolled search finds them at its third and
/// fourth step), on a key (3.25, 4.25: the restart's and the search's compares at equality), before
/// the second key and before the first (1.0, -1.0), past key 8 (8.5, 20.0: with 9 keys the search
/// runs off the end) and NaN.
const KEY_TIMES: &[u32] = &[
    0x40c0_0000, // 6.0
    0x40e0_0000, // 7.0
    0x4050_0000, // 3.25
    0x4088_0000, // 4.25
    0x3f80_0000, // 1.0
    0x4108_0000, // 8.5
    0x41a0_0000, // 20.0
    0xbf80_0000, // -1.0
    0x7fc0_0000, // NaN
];

/// `GenInterp`'s inputs: the time ([`KEY_TIMES`], or `swept`, a swept float); at -FLT_MAX instead or
/// not; the key type (1, or 4: the whole array); the count (9 first: the last index 3 then finds
/// 6.0 in the unrolled search; 2 with the last index 3 loads no later key, so the ratio's next time
/// is the 0.0 the exe leaves); the last index (3, a cached one, first; 0 restarts nothing, 8 past
/// 6.0 restarts at key 0); the key size; the keys' floats.
fn gen_interp_inputs(swept: bool) -> Vec<Input> {
    let time = if swept {
        Input::F32
    } else {
        Input::Of(KEY_TIMES)
    };
    inputs(&[
        &[
            time,
            Input::Of(&[0, 1]),
            Input::Of(&[1, 4]),
            Input::Of(&[9, 1, 2, 5, 6]),
            Input::Of(&[3, 0, 1, 5, 8]),
            Input::Of(&[0x14, 0x24]),
        ],
        &FLOATS8,
    ])
}

/// `GenInterp`'s words: with the keys at `k + 0.25`, or (`swept`) spread apart, so that the ratio's
/// division rounds.
fn gen_interp_words(e: &mut Engine, case: &Case, swept: bool) -> Vec<u32> {
    constants(e);
    stand_ins(e);
    let size = case.word(5);
    let spread = if swept { 0.125 } else { 0.0 };
    let keys = rot_keys_of(e, case, 6, size, spread);
    let last = e.mem.alloc(4);
    e.mem.set_u32(last, case.word(4));
    let result = e.mem.alloc(0x10);
    let time = if case.word(1) == 1 {
        f32::MIN
    } else if swept {
        case.f32(0)
    } else {
        f32::from_bits(case.word(0))
    };
    vec![
        result,
        time.to_bits(),
        keys,
        case.word(2),
        case.word(3),
        last,
        size,
    ]
}

#[test]
fn anim_keys_gen_interp() {
    // Every edge of the key search. Under the other threads' control word bit for bit.
    Diff::new(entry!(
        0x00a2_8740,
        rot_keys::gen_interp(Ptr<_>, f32, Ptr, u32, u32, Ptr, u8) -> Ptr<_>
    ))
    .common()
    .control_words(&[OTHER_THREADS])
    .allow_nan_payloads()
    .inputs(&gen_interp_inputs(false))
    .setup(|e, case| gen_interp_words(e, case, false))
    .run()
    .assert_same();
    // Under the main thread's within 1 ulp, every input, as below.
    let report = Diff::new(entry!(
        0x00a2_8740,
        rot_keys::gen_interp(Ptr<_>, f32, Ptr, u32, u32, Ptr, u8) -> Ptr<_>
    ))
    .common()
    .control_words(&[MAIN_THREAD])
    .float_ulps(1)
    .inputs(&gen_interp_inputs(false))
    .setup(|e, case| gen_interp_words(e, case, false))
    .run();
    println!("gen_interp on the main thread: {report}");
    report.assert_same();
}

#[test]
fn anim_keys_gen_interp_ratio() {
    // The ratio, at swept times with the keys spread apart. Under the other threads' control word
    // bit for bit.
    Diff::new(entry!(
        0x00a2_8740,
        rot_keys::gen_interp(Ptr<_>, f32, Ptr, u32, u32, Ptr, u8) -> Ptr<_>
    ))
    .common()
    .control_words(&[OTHER_THREADS])
    .allow_nan_payloads()
    .inputs(&gen_interp_inputs(true))
    .setup(|e, case| gen_interp_words(e, case, true))
    .run()
    .assert_same();
    // Under the main thread's within 1 ulp, every input: the exe rounds the time's and the span's
    // differences to 24 bits before it divides.
    let report = Diff::new(entry!(
        0x00a2_8740,
        rot_keys::gen_interp(Ptr<_>, f32, Ptr, u32, u32, Ptr, u8) -> Ptr<_>
    ))
    .common()
    .control_words(&[MAIN_THREAD])
    .float_ulps(1)
    .inputs(&gen_interp_inputs(true))
    .setup(|e, case| gen_interp_words(e, case, true))
    .run();
    println!("gen_interp_ratio on the main thread: {report}");
    report.assert_same();
}
