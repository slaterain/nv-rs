//! The differential test of every typed function of the animation keys family
//! (`engine_typed::anim_keys`) against its translation: here because the translations live in the part
//! crates, and this crate registers them all. Quaternions and key arrays are filled from the case's
//! floats; the exe's constants these functions read are written at their addresses.
//!
//! What stands in: the key types' interpolation function (the table at `011f3cc0`; it writes a
//! quaternion of its first three words and 1.0, so what it was handed shows in the result) and
//! `NiQuaternion::FastNormalize` (`00a4ff00`; it flips `W`'s sign, so the call and what it saw show).
//! Everything else runs translated.

use engine as _;
use engine_core::prelude::*;
use engine_core::typed::diff::{Case, Diff, Input};
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

/// `count` rotation keys of `size` bytes (room for 12): key `k`'s time `k + 0.25`, its quaternion
/// from the case's eight floats at `first` (in turn, so neighbours differ), every third key's
/// negated (so neighbours fall in both hemispheres), and the rest of a key's bytes a pattern of its
/// own (so a copy of them shows).
fn rot_keys_of(e: &mut Engine, case: &Case, first: usize, size: u32) -> u32 {
    let keys = e.mem.alloc(size * 12);
    for k in 0..12u32 {
        let key = keys + k * size;
        e.mem.set_u32(key, (k as f32 + 0.25).to_bits());
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

#[test]
fn anim_keys_counter_warp() {
    // The parameter; the cosine.
    Diff::new(entry!(
        0x00a6_dc30,
        quaternion::counter_warp(f32, f32) -> f64
    ))
    .common()
    .inputs(&[Input::F32, Input::F32])
    .setup(|e, case| {
        constants(e);
        vec![case.f32(0).to_bits(), case.f32(1).to_bits()]
    })
    .run()
    .assert_same();
}

#[test]
fn anim_keys_slerp() {
    // The parameter; the result apart, over p or over q; p's and q's floats.
    Diff::new(entry!(
        0x00a6_e330,
        quaternion::slerp(Ptr<_>, f32, Ptr<_>, Ptr<_>)
    ))
    .common()
    .inputs(&inputs(&[&[Input::F32, Input::Of(&[0, 1, 2])], &FLOATS8]))
    .setup(|e, case| {
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
    })
    .run()
    .assert_same();
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
            let keys = rot_keys_of(e, case, 2, case.word(1) & 0xff);
            vec![keys, case.word(0), case.word(1)]
        })
        .run()
        .assert_same();
}

#[test]
fn anim_keys_gen_interp() {
    // The time; at -FLT_MAX instead or not; the key type (1, or 4: the whole array); the count; the
    // last index; the key size; the keys' floats.
    Diff::new(entry!(
        0x00a2_8740,
        rot_keys::gen_interp(Ptr<_>, f32, Ptr, u32, u32, Ptr, u8) -> Ptr<_>
    ))
    .common()
    .inputs(&inputs(&[
        &[
            Input::F32,
            Input::Of(&[0, 1]),
            Input::Of(&[1, 4]),
            Input::Of(&[6, 1, 2, 5, 9]),
            Input::Of(&[0, 1, 3, 8]),
            Input::Of(&[0x14, 0x24]),
        ],
        &FLOATS8,
    ]))
    .setup(|e, case| {
        constants(e);
        stand_ins(e);
        let size = case.word(5);
        let keys = rot_keys_of(e, case, 6, size);
        let last = e.mem.alloc(4);
        e.mem.set_u32(last, case.word(4));
        let result = e.mem.alloc(0x10);
        let time = if case.word(1) == 1 {
            f32::MIN
        } else {
            case.f32(0)
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
    })
    .run()
    .assert_same();
}
