//! The differential test of every typed function of the animation keys family
//! (`engine_typed::anim_keys`) against its translation: here because the translations live in the part
//! crates, and this crate registers them all. Quaternions and key arrays are filled from the case's
//! floats; the exe's constants these functions read are written at their addresses.
//!
//! What stands in: a key type's interpolation function the family does not know (a stand-in past
//! `.text` in a table slot; it writes what it was handed, so the call shows). The tables otherwise hold
//! the exe's functions, and everything runs translated on one side and native on the other.
//!
//! The maths is native at the running thread's precision (`f32` on main, `f64` on the AI threads), so
//! every comparison is bit for bit, NaN payloads aside, under both control words: under the other
//! threads' (53-bit precision) on every input; under the main thread's (24-bit precision, where the
//! exe keeps the x87 exponent range and `f32` does not) on the values these functions get in play
//! ([`in_play`]: a magnitude far from overflow and underflow).

use engine as _;
use engine_core::prelude::*;
use engine_core::typed::diff::{Case, Diff, Input, Value};
use engine_core::typed::{MAIN_THREAD, OTHER_THREADS};
use engine_typed::anim_keys::{self, quaternion, rot_keys};

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
/// mirror), 2.0 and 3.0 (doubles: the Hermite value's), the correction's two factors (doubles),
/// `-FLT_MAX` as a double (the lowest time), `FastNormalize`'s constants.
fn constants(e: &mut Engine) {
    for page in [
        0x0101_1000,
        0x0101_2000,
        0x0101_6000,
        0x0102_1000,
        0x0102_4000,
        0x0109_9000,
        0x0109_c000,
        0x011f_3000,
        0x011f_4000,
    ] {
        e.map(page, 0x1000);
    }
    // `FastNormalize`'s centre and limits (doubles holding floats, and a float), and the slope and base
    // the static initialiser works out (here those of 1 / sqrt around the centre).
    let center = f64::from_bits(0x3fee_b0ab_2000_0000);
    e.mem.set_u64(0x0109_9680, center.to_bits());
    e.mem.set_u64(0x0109_9690, 0x3fed_496a_a000_0000);
    e.mem.set_u32(0x0109_9688, 0x3f26_f151);
    e.mem
        .set_u32(0x011f_3fec, ((-0.5 * center.powf(-1.5)) as f32).to_bits());
    e.mem
        .set_u32(0x011f_3ff0, ((1.0 / center.sqrt()) as f32).to_bits());
    e.mem.set_u32(0x0101_1d78, 0.0f32.to_bits());
    e.mem.set_u32(0x0101_2054, (-1.0f32).to_bits());
    e.mem.set_u32(0x0101_6248, 0.5f32.to_bits());
    e.mem.set_u64(0x0101_1590, 2.0f64.to_bits());
    e.mem.set_u64(0x0102_1928, 3.0f64.to_bits());
    e.mem.set_u64(0x0102_41b0, f64::from(f32::MIN).to_bits());
    e.mem.set_u64(0x0109_c9d8, 0x3fea_545a_2000_0000);
    e.mem.set_u64(0x0109_c9d0, 0x3fe2_bc5a_2000_0000);
}

/// The stand-in interpolation function, in the rotation table for key types 1 and 4.
fn stand_ins(e: &mut Engine) {
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
    // The parameter; the cosine. Under the other threads' control word on every input, under the
    // main thread's in play; bit for bit on both.
    for (word, play) in [(OTHER_THREADS, false), (MAIN_THREAD, true)] {
        Diff::new(entry!(
            0x00a6_dc30,
            quaternion::counter_warp(f32, f32) -> f64
        ))
        .common()
        .control_words(&[word])
        .allow_nan_payloads()
        .inputs(&[Input::F32, Input::F32])
        .setup(move |e, case| counter_warp_words(e, case, play))
        .run()
        .assert_same();
    }
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
    // Under the other threads' control word on every input, under the main thread's in play; bit
    // for bit on both. The typed version does the parameter correction and `FastNormalize` in place.
    for (word, play) in [(OTHER_THREADS, false), (MAIN_THREAD, true)] {
        Diff::new(entry!(
            0x00a6_e330,
            quaternion::slerp(Ptr<_>, f32, Ptr<_>, Ptr<_>)
        ))
        .common()
        .ignore_calls(&[0x00a6_dc30, 0x00a4_ff00])
        .control_words(&[word])
        .allow_nan_payloads()
        .inputs(&slerp_inputs())
        .setup(move |e, case| slerp_words(e, case, play))
        .run()
        .assert_same();
    }
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
    // Every edge of the key search, bit for bit under both control words (the keys' floats are only
    // copied, the ratio is the stand-in's to hand on).
    Diff::new(entry!(
        0x00a2_8740,
        rot_keys::gen_interp(Ptr<_>, f32, Ptr, u32, u32, Ptr, u8) -> Ptr<_>
    ))
    .common()
    .allow_nan_payloads()
    .inputs(&gen_interp_inputs(false))
    .setup(|e, case| gen_interp_words(e, case, false))
    .run()
    .assert_same();
}

#[test]
fn anim_keys_gen_interp_ratio() {
    // The ratio, at swept times with the keys spread apart, bit for bit under both control words:
    // on the main thread the exe rounds the time's and the span's differences to 24 bits before it
    // divides, as the native `f32` arithmetic does.
    Diff::new(entry!(
        0x00a2_8740,
        rot_keys::gen_interp(Ptr<_>, f32, Ptr, u32, u32, Ptr, u8) -> Ptr<_>
    ))
    .common()
    .allow_nan_payloads()
    .inputs(&gen_interp_inputs(true))
    .setup(|e, case| gen_interp_words(e, case, true))
    .run()
    .assert_same();
}

// ---- The float and position keys, the key types' functions, Squad and FastNormalize ----

/// The stand-ins a table slot holds where the family knows no function: one writes a word of what it
/// was handed into the float result, one three words into the point.
const FLOAT_STAND_IN: u32 = 0x00ff_3100;
const POINT_STAND_IN: u32 = 0x00ff_3200;
/// The tables of the key types' functions: float, position, rotation.
const FLOAT_TABLE: u32 = 0x011f_3c90;
const POINT_TABLE: u32 = 0x011f_3ca8;
const ROTATION_TABLE: u32 = 0x011f_3cc0;
/// What the family does in place where a translation calls by address: the float keys' functions
/// (with the empty one, the blend and the Hermite value they call), the position keys', the rotation
/// keys' (with the constructor, `Slerp`, its correction, `FastNormalize` and the quadrangle
/// interpolation).
const FLOAT_IN_PLACE: [u32; 7] = [
    0x00a2_9680,
    0x00a2_a560,
    0x00a4_2050,
    0x00a2_6fe0,
    0x00a4_2070,
    0x00a2_c610,
    0x00a2_d200,
];
const POINT_IN_PLACE: [u32; 4] = [0x00a2_7470, 0x00a2_a7f0, 0x00a2_7c80, 0x00a2_af70];
const ROTATION_IN_PLACE: [u32; 10] = [
    0x00a2_86e0,
    0x00a6_db50,
    0x00a2_ab90,
    0x00a6_e330,
    0x00a6_dc30,
    0x00a4_ff00,
    0x00a2_8d90,
    0x00a2_bc60,
    0x00a6_e430,
    0x00a2_cf20,
];
/// What the rotation keys' Bezier and TCB functions do in place (the linear one the last three).
const SQUAD_IN_PLACE: [u32; 4] = [0x00a6_e430, 0x00a6_e330, 0x00a6_dc30, 0x00a4_ff00];
/// The key sizes per key type: what each type's function reads (float: value +4, Bezier tangents +8
/// and +0xc, TCB tangents +0x14 and +0x18; position: point +4, Bezier tangent and coefficients +0x1c,
/// +0x28, +0x34, TCB +0x28, +0x34, +0x40; rotation: quaternion +4, Bezier intermediate +0x14, TCB
/// tangents +0x20 and +0x30).
const FLOAT_SIZES: [u32; 6] = [8, 8, 0x10, 0x1c, 8, 8];
const POINT_SIZES: [u32; 6] = [0x10, 0x10, 0x40, 0x4c, 0x10, 0x10];
const ROTATION_SIZES: [u32; 6] = [0x14, 0x14, 0x24, 0x40, 0x14, 0x14];
/// Times against keys at `0.25 + k (1 + k / 8)` (0.25, 1.375, 2.75, ... 26.375 for key 11): between
/// keys, before the first, on the first, on key 2, on the last, past the end, NaN, -FLT_MAX, negative.
const SPREAD_TIMES: &[u32] = &[
    0x3f80_0000, // 1.0
    0x0000_0000, // 0.0
    0x3e80_0000, // 0.25, key 0
    0x4030_0000, // 2.75, key 2
    0x40a0_0000, // 5.0
    0x4110_0000, // 9.0
    0x41d3_0000, // 26.375, key 11
    0x41f0_0000, // 30.0
    0x7fc0_0000, // NaN
    0xff7f_ffff, // -FLT_MAX
    0xbf80_0000, // -1.0
];
/// Ratios a key type's function is handed: inside, at the ends, past them, NaN.
const RATIOS: &[u32] = &[
    0x3f00_0000, // 0.5
    0x0000_0000, // 0
    0x3f80_0000, // 1.0
    0x3eaa_aaab, // 1/3
    0x3f7f_ffff, // just below 1
    0x3fc0_0000, // 1.5
    0xbe80_0000, // -0.25
    0x7fc0_0000, // NaN
];
/// Eight floats each swept along by the float sweep, plain in the ordinary cases (distinct values
/// with low bits, so every product rounds).
const SWEPT8: [Input; 8] = [
    Input::F32Swept(0x3f9d_70a4),
    Input::F32Swept(0xbf40_0000),
    Input::F32Swept(0x3eaa_aaab),
    Input::F32Swept(0x4049_0fdb),
    Input::F32Swept(0xbe4c_cccd),
    Input::F32Swept(0x3f35_04f3),
    Input::F32Swept(0x40a0_0000),
    Input::F32Swept(0xbfb3_3333),
];

/// The case's float `i`, [`in_play`] or as it came.
fn float_of(case: &Case, i: usize, play: bool) -> u32 {
    case_of(case, play).f32(i).to_bits()
}

/// The exe's functions in the three tables, the stand-ins in the float and position rows' slot 4 (and
/// the position row's 5), the rotation row's 4 (the Euler keys) the rotation stand-in; the zero point
/// the empty position interpolation copies (distinct words, so the copy shows).
fn tables(e: &mut Engine) {
    constants(e);
    stand_ins(e);
    e.register(FLOAT_STAND_IN, |e, a| {
        e.mem.set_u32(a[3], a[0] ^ a[1]);
        Ret::default()
    });
    e.register(POINT_STAND_IN, |e, a| {
        for (i, w) in [a[0], a[1], a[2]].into_iter().enumerate() {
            e.mem.set_u32(a[3] + 4 * i as u32, w);
        }
        Ret::default()
    });
    let rows: [(u32, [u32; 6]); 3] = [
        (
            FLOAT_TABLE,
            [
                0x00a2_9680,
                0x00a2_a560,
                0x00a2_6fe0,
                0x00a2_c610,
                FLOAT_STAND_IN,
                0x00a2_d200,
            ],
        ),
        (
            POINT_TABLE,
            [
                0x00a2_7470,
                0x00a2_a7f0,
                0x00a2_7c80,
                0x00a2_af70,
                POINT_STAND_IN,
                POINT_STAND_IN,
            ],
        ),
        (
            ROTATION_TABLE,
            [
                0x00a2_86e0,
                0x00a2_ab90,
                0x00a2_8d90,
                0x00a2_bc60,
                INTERPOLATE,
                0x00a2_cf20,
            ],
        ),
    ];
    for (row, words) in rows {
        for (i, w) in (0u32..).zip(words) {
            e.mem.set_u32(row + 4 * i, w);
        }
    }
    for (i, w) in (0u32..).zip([0x3f00_0000u32, 0xbf80_0000, 0x4000_0000]) {
        e.mem.set_u32(0x011f_426c + 4 * i, w);
    }
}

/// 14 keys of `size` bytes (room past the 12 a search may reach): key `k`'s time `0.25 + k (1 + k /
/// 8)`, every word after it one of the case's eight floats from `first` in turn ([`in_play`] or as they
/// came).
fn spread_keys(e: &mut Engine, case: &Case, first: usize, size: u32, play: bool) -> u32 {
    let keys = e.mem.alloc(size * 14);
    let mut n = 0usize;
    for k in 0..14u32 {
        let key = keys + k * size;
        let time = 0.25 + k as f32 * (1.0 + k as f32 / 8.0);
        e.mem.set_u32(key, time.to_bits());
        for offset in (4..size).step_by(4) {
            e.mem
                .set_u32(key + offset, float_of(case, first + n % 8, play));
            n += 3;
        }
    }
    keys
}

/// A `GenInterp`'s inputs: the time, the key type (from `types`), the count, the last index, the
/// eight floats.
fn gen_inputs(types: &'static [u32]) -> Vec<Input> {
    inputs(&[
        &[
            Input::Of(SPREAD_TIMES),
            Input::Of(types),
            Input::Of(&[12, 1, 2, 5]),
            Input::Of(&[0, 4, 11]),
        ],
        &SWEPT8,
    ])
}

/// A `GenInterp`'s words after the result (if any): time, keys, type, count, last index, key size.
fn gen_words(e: &mut Engine, case: &Case, sizes: &[u32; 6], play: bool) -> Vec<u32> {
    tables(e);
    let key_type = case.word(1);
    let size = sizes[key_type as usize];
    let keys = spread_keys(e, case, 4, size, play);
    let last = e.mem.alloc(4);
    e.mem.set_u32(last, case.word(3));
    vec![case.word(0), keys, key_type, case.word(2), last, size]
}

#[test]
fn anim_keys_float_gen_interp() {
    // Every edge of the search with each key type the table has (1 linear, 2 Bezier, 3 TCB, 5 step,
    // 0 none, 4 a function the family does not know), bit for bit: under the other threads' control
    // word on every input, under the main thread's in play.
    for (word, play) in [(OTHER_THREADS, false), (MAIN_THREAD, true)] {
        Diff::new(entry!(
            0x00a2_6b40,
            anim_keys::float_keys::gen_interp(f32, Ptr, u32, u32, Ptr, u8) -> f64
        ))
        .common()
        .ignore_calls(&FLOAT_IN_PLACE)
        .control_words(&[word])
        .allow_nan_payloads()
        .max_cases(800)
        .inputs(&gen_inputs(&[1, 2, 3, 5, 0, 4]))
        .setup(move |e, case| gen_words(e, case, &FLOAT_SIZES, play))
        .run()
        .assert_same();
    }
}

#[test]
fn anim_keys_pos_gen_interp() {
    // As the float keys: 1 linear, 2 Bezier, 3 TCB, 0 none, 4 a function the family does not know.
    for (word, play) in [(OTHER_THREADS, false), (MAIN_THREAD, true)] {
        Diff::new(entry!(
            0x00a2_7490,
            anim_keys::pos_keys::gen_interp(Ptr, f32, Ptr, u32, u32, Ptr, u8) -> Ptr
        ))
        .common()
        .ignore_calls(&POINT_IN_PLACE)
        .control_words(&[word])
        .allow_nan_payloads()
        .max_cases(800)
        .inputs(&gen_inputs(&[1, 2, 3, 0, 4]))
        .setup(move |e, case| {
            let mut words = gen_words(e, case, &POINT_SIZES, play);
            words.insert(0, e.mem.alloc(0xc));
            words
        })
        .run()
        .assert_same();
    }
}

#[test]
fn anim_keys_rot_gen_interp_in_place() {
    // The rotation keys with the exe's functions in the table (1 linear, 2 Bezier, 3 TCB, 5 step, 0
    // the identity; 4 the Euler keys, the stand-in), which the typed version does in place.
    for (word, play) in [(OTHER_THREADS, false), (MAIN_THREAD, true)] {
        Diff::new(entry!(
            0x00a2_8740,
            rot_keys::gen_interp(Ptr<_>, f32, Ptr, u32, u32, Ptr, u8) -> Ptr<_>
        ))
        .common()
        .ignore_calls(&ROTATION_IN_PLACE)
        .control_words(&[word])
        .allow_nan_payloads()
        .max_cases(800)
        .inputs(&gen_inputs(&[1, 2, 3, 5, 0, 4]))
        .setup(move |e, case| {
            let mut words = gen_words(e, case, &ROTATION_SIZES, play);
            words.insert(0, e.mem.alloc(0x10));
            words
        })
        .run()
        .assert_same();
    }
}

/// A key type's function's inputs: the ratio; which of the result's places (fresh, or over the second
/// key's first words); the eight floats.
fn function_inputs() -> Vec<Input> {
    inputs(&[&[Input::Of(RATIOS), Input::Of(&[0, 1])], &SWEPT8])
}

/// A key type's function's words: the ratio, two keys of `size` bytes from the case's floats
/// ([`in_play`] for the main thread), the result (fresh, or the second key's +4).
fn function_words(e: &mut Engine, case: &Case, size: u32, play: bool) -> Vec<u32> {
    tables(e);
    let keys = spread_keys(e, case, 2, size, play);
    let result = if case.word(1) == 1 {
        keys + size + 4
    } else {
        e.mem.alloc(0x10)
    };
    vec![case.word(0), keys, keys + size, result]
}

/// Runs one key type's function's comparison under both control words.
fn key_function(f: (u32, AbiFn), size: u32, in_place: &'static [u32]) {
    for (word, play) in [(OTHER_THREADS, false), (MAIN_THREAD, true)] {
        Diff::new(f)
            .common()
            .ignore_calls(in_place)
            .control_words(&[word])
            .allow_nan_payloads()
            .inputs(&function_inputs())
            .setup(move |e, case| function_words(e, case, size, play))
            .run()
            .assert_same();
    }
}

#[test]
fn anim_keys_float_functions() {
    use anim_keys::float_keys;
    key_function(
        entry!(0x00a2_a560, float_keys::linear(f32, Ptr, Ptr, Ptr)),
        8,
        &[0x00a4_2050],
    );
    key_function(
        entry!(0x00a2_6fe0, float_keys::bezier(f32, Ptr, Ptr, Ptr)),
        0x10,
        &[0x00a4_2070],
    );
    key_function(
        entry!(0x00a2_c610, float_keys::tcb(f32, Ptr, Ptr, Ptr)),
        0x1c,
        &[0x00a4_2070],
    );
    key_function(
        entry!(0x00a2_d200, float_keys::step(f32, Ptr, Ptr, Ptr)),
        8,
        &[],
    );
}

#[test]
fn anim_keys_pos_functions() {
    use anim_keys::pos_keys;
    key_function(
        entry!(0x00a2_7470, pos_keys::none(f32, Ptr, Ptr, Ptr)),
        0x10,
        &[],
    );
    key_function(
        entry!(0x00a2_a7f0, pos_keys::linear(f32, Ptr, Ptr, Ptr)),
        0x10,
        &[],
    );
    key_function(
        entry!(0x00a2_7c80, pos_keys::bezier(f32, Ptr, Ptr, Ptr)),
        0x40,
        &[],
    );
    key_function(
        entry!(0x00a2_af70, pos_keys::tcb(f32, Ptr, Ptr, Ptr)),
        0x4c,
        &[],
    );
}

#[test]
fn anim_keys_rot_functions() {
    key_function(
        entry!(0x00a2_86e0, rot_keys::identity(f32, Ptr, Ptr, Ptr)),
        0x14,
        &[0x00a6_db50],
    );
    key_function(
        entry!(0x00a2_ab90, rot_keys::linear(f32, Ptr, Ptr, Ptr) -> u32),
        0x14,
        &SQUAD_IN_PLACE[1..],
    );
    key_function(
        entry!(0x00a2_8d90, rot_keys::bezier(f32, Ptr, Ptr, Ptr)),
        0x24,
        &SQUAD_IN_PLACE,
    );
    key_function(
        entry!(0x00a2_bc60, rot_keys::tcb(f32, Ptr, Ptr, Ptr) -> u32),
        0x40,
        &SQUAD_IN_PLACE,
    );
    key_function(
        entry!(0x00a2_cf20, rot_keys::step(f32, Ptr, Ptr, Ptr)),
        0x14,
        &[],
    );
}

#[test]
fn anim_keys_lerp_and_hermite() {
    // The floats as they come under the other threads' word, in play under the main thread's.
    for (word, play) in [(OTHER_THREADS, false), (MAIN_THREAD, true)] {
        Diff::new(entry!(
            0x00a4_2050,
            anim_keys::float_keys::lerp(f32, f32, f32) -> f64
        ))
        .common()
        .control_words(&[word])
        .allow_nan_payloads()
        .inputs(&[Input::F32, Input::F32, Input::F32])
        .setup(move |e, case| {
            constants(e);
            (0..3).map(|i| float_of(case, i, play)).collect()
        })
        .run()
        .assert_same();
        Diff::new(entry!(
            0x00a4_2070,
            anim_keys::float_keys::hermite(f32, f32, f32, f32, f32) -> f64
        ))
        .common()
        .control_words(&[word])
        .allow_nan_payloads()
        .inputs(&[Input::F32; 5])
        .setup(move |e, case| {
            constants(e);
            (0..5).map(|i| float_of(case, i, play)).collect()
        })
        .run()
        .assert_same();
    }
}

#[test]
fn anim_keys_fast_normalize() {
    // The four floats, each varied and all swept: a unit quaternion's length takes two Newton steps,
    // a longer one one, past the second limit none.
    for (word, play) in [(OTHER_THREADS, false), (MAIN_THREAD, true)] {
        Diff::new(entry!(0x00a4_ff00, quaternion::fast_normalize(Ptr)))
            .common()
            .control_words(&[word])
            .allow_nan_payloads()
            .inputs(&[Input::F32, Input::F32, Input::F32, Input::F32])
            .setup(move |e, case| {
                constants(e);
                let q = e.mem.alloc(0x10);
                for i in 0..4u32 {
                    e.mem.set_u32(q + 4 * i, float_of(case, i as usize, play));
                }
                vec![q]
            })
            .run()
            .assert_same();
    }
}

#[test]
fn anim_keys_squad() {
    // The parameter; the result fresh or over p; the four quaternions' sixteen floats from the case's
    // eight (in turn).
    for (word, play) in [(OTHER_THREADS, false), (MAIN_THREAD, true)] {
        Diff::new(entry!(
            0x00a6_e430,
            quaternion::squad(Ptr<_>, f32, Ptr<_>, Ptr<_>, Ptr<_>, Ptr<_>)
        ))
        .common()
        .ignore_calls(&SQUAD_IN_PLACE[1..])
        .control_words(&[word])
        .allow_nan_payloads()
        .inputs(&inputs(&[&[Input::F32, Input::Of(&[0, 1])], &SWEPT8]))
        .setup(move |e, case| {
            constants(e);
            let quads: Vec<u32> = (0..4usize)
                .map(|k| {
                    let q = e.mem.alloc(0x10);
                    for j in 0..4usize {
                        let float = float_of(case, 2 + (k * 5 + j) % 8, play);
                        e.mem.set_u32(q + 4 * j as u32, float);
                    }
                    q
                })
                .collect();
            let result = if case.word(1) == 1 {
                quads[0]
            } else {
                e.mem.alloc(0x10)
            };
            let t = float_of(case, 0, play);
            vec![result, t, quads[0], quads[1], quads[2], quads[3]]
        })
        .run()
        .assert_same();
    }
}
