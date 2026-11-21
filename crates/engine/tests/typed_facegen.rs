//! The differential test of the face family (`engine_typed::facegen`) against the translations.
//!
//! The morphs: a face geometry (its data at +0xb8 with the vertex count at +8), its locked vertex
//! array (a stand-in for the lock's start, `00a67410`, answering the case's array and stride), the
//! morphs (differential: an offset per vertex; statistical: indexed vertices and basis rows) and
//! the base offsets of the morph extra data (a stand-in for `NiObjectNET::GetExtraData`). The
//! arrays go up to 300 vertices with values a face has, and the awkward ones: signed zeros, the smallest subnormal, the largest float, a NaN. The
//! weights try zero, one, between, above one, below zero, a NaN and a tiny one. The translations
//! make several calls by address a vertex (the strided cursor's read, write and step, the point's
//! scale, add and difference); the typed loops make none, so those callees are left out of the
//! call comparison (`ignore_calls`); every vertex byte, every other call and the heap compare.
//!
//! The keyframes: `BSFaceGenKeyframeMultiple`s on a vtable of the class's own functions (the
//! translations, `GetType` and `IsMultiple` doubles as the slots' folded bodies read), or on one
//! whose `Compare`, `GetStrength` and `IsValidQuantity` are other functions (doubles), so both the
//! direct calls and the virtual calls are compared; weight arrays of the counts a face has (16
//! phonemes, 15 expressions, 14 modifiers) with the weights a face takes and the awkward ones.
//!
//! Floats: the vertex steps are one operation on two floats each, plain `f32`; the keyframes' two
//! chains run at the thread's precision (both control words run). Floats compare bit for bit,
//! except NaN payloads (`allow_nan_payloads`: SSE and x87 pick a NaN's payload differently, which
//! the game cannot see).

use engine as _;
use engine_core::prelude::*;
use engine_core::typed::diff::{Case, Diff, Input};
use engine_typed::facegen::{self, keyframe, morph};

/// The case's words the stand-ins read: the locked array and its stride, the extra data.
const TEST: u32 = 0x0300_0000;
const LOCKED_ARRAY: u32 = TEST;
const LOCKED_STRIDE: u32 = TEST + 4;
const EXTRA: u32 = TEST + 8;
const MARKS: u32 = TEST + 0xc;
/// The keyframes' vtables: the class's own, and one with other functions in three slots.
const OWN_VTABLE: u32 = TEST + 0x100;
const OTHER_VTABLE: u32 = TEST + 0x200;
const SINGLE_VTABLE: u32 = TEST + 0x300;
/// Doubles: `GetType` (the word at +4) and `IsMultiple` (true; false on the single vtable), and the
/// other functions of the other vtable.
const GET_TYPE: u32 = TEST + 0x800;
const IS_MULTIPLE: u32 = TEST + 0x804;
const IS_SINGLE: u32 = TEST + 0x808;
const OTHER_COMPARE: u32 = TEST + 0x80c;
const OTHER_STRENGTH: u32 = TEST + 0x810;
const OTHER_VALID: u32 = TEST + 0x814;

/// The per-vertex callees of the translations the typed loops do not make.
const PER_VERTEX: [u32; 11] = [
    0x0053_4550, // the cursor's constructor
    0x0065_6fd0, // the cursor's read
    0x0065_33f0, // the cursor's write
    0x0065_7be0, // the cursor's step
    0x0053_4570, // the indexed read
    0x0065_b390, // the indexed write
    0x0045_34f0, // its bounds check (does nothing)
    0x0045_bb20, // the point's scale
    0x0063_c8a0, // the point's add
    0x0043_9ef0, // the points' difference
    0x0045_6650, // the face's vertex count, asked again each step of the base copy
];

fn ret(eax: u32) -> Ret {
    Ret {
        eax,
        ..Ret::default()
    }
}

/// The exe's constants these read: 0.0, 1.0 and `FLT_MAX` (doubles), `FLT_MAX` and 0.001 (floats).
fn constants(e: &mut Engine) {
    e.map(0x0101_2000, 0x1000);
    e.mem.set_f64(0x0101_2060, 0.0);
    e.mem.set_f64(0x0101_2070, 1.0);
    e.map(0x0101_6000, 0x2000);
    e.mem.set_f32(0x0101_6970, f32::MAX);
    e.mem.set_f32(0x0101_7d00, 0.001);
    e.map(0x0102_3000, 0x1000);
    e.mem.set_f64(0x0102_31b0, f64::from(f32::MAX));
}

fn stand_ins(e: &mut Engine) {
    e.map(TEST, 0x1000);
    constants(e);
    e.map(0x011d_5000, 0x1000);
    e.mem.set_u32(0x011d_5b8c, 0x0700_0000);
    // The lock's start: the case's array and stride, byte 0.
    e.register(0x00a6_7410, |e, a| {
        let out = a[1];
        let array = e.mem.u32(LOCKED_ARRAY);
        let stride = e.mem.u32(LOCKED_STRIDE);
        e.mem.set_u32(out, array);
        e.mem.set_u32(out + 4, stride);
        e.mem.set_u8(out + 8, 0);
        ret(0)
    });
    // `GetExtraData` with the morph data's name: the case's extra data (or none).
    e.register(0x00a5_bdd0, |e, a| {
        assert_eq!(a[1], 0x0700_0000);
        ret(e.mem.u32(EXTRA))
    });
    // `MarkAsChanged`: counted.
    e.register(0x00a6_7090, |e, a| {
        assert_eq!(a[1], 1);
        let n = e.mem.u32(MARKS);
        e.mem.set_u32(MARKS, n + 1);
        ret(0)
    });
}

/// Float words a face's coordinates and offsets take, with the awkward ones among them.
fn value(i: u32, k: u32, pattern: u32) -> u32 {
    let n = i
        .wrapping_mul(2_654_435_761)
        .wrapping_add(k * 97 + pattern * 13);
    match n % 23 {
        0 => 0x8000_0000,                 // -0.0
        1 => 0,                           // 0.0
        2 => 0x0000_0001,                 // the smallest subnormal
        3 => 0x7f7f_ffff,                 // the largest float
        4 if pattern == 2 => 0x7fc0_0000, // a NaN
        _ => ((n % 2001) as f32 / 97.0 - 10.0).to_bits(),
    }
}

/// A block of `count` 12-byte points from the pattern.
fn points(e: &mut Engine, count: u32, pattern: u32, salt: u32) -> u32 {
    let a = e.mem.alloc(12 * count.max(1));
    for i in 0..count {
        for k in 0..3 {
            e.mem
                .set_u32(a + 12 * i + 4 * k, value(i + salt, k, pattern));
        }
    }
    a
}

/// The face geometry with `vertices` vertices, its locked array of that many at `stride`, its data
/// word (+0x20 of the data, what `0049ec60` reads) set when `with_data`; returns the geometry.
fn face(e: &mut Engine, vertices: u32, stride: u32, pattern: u32, with_data: bool) -> u32 {
    let data = e.mem.alloc(0x40);
    e.mem.set_u16(data + 8, vertices as u16);
    if with_data {
        let form = e.mem.alloc(0x40);
        e.mem.set_u32(data + 0x20, form);
    }
    let geometry = e.mem.alloc(0xc0);
    e.mem.set_u32(geometry + 0xb8, data);
    let array = e.mem.alloc(stride * vertices.max(1) + 16);
    for i in 0..vertices {
        for k in 0..(stride / 4) {
            e.mem
                .set_u32(array + stride * i + 4 * k, value(i, k + 5, pattern));
        }
    }
    e.mem.set_u32(LOCKED_ARRAY, array);
    e.mem.set_u32(LOCKED_STRIDE, stride);
    geometry
}

const WEIGHTS: [u32; 7] = [
    0,           // 0.0: not applied
    0x3e80_0000, // 0.25
    0x3f80_0000, // 1.0
    0x3fc0_0000, // 1.5: not applied
    0xbf00_0000, // -0.5: not applied
    0x7fc0_0000, // a NaN: applied, as in the exe
    0x0d80_0000, // tiny
];

/// The sizes the cases try: (vertices, morph count): none, fewer offsets than vertices, more, and
/// a larger block (a head has about 1,400; the loops do the same per vertex, and the translation's
/// side logs every call, so a smaller block keeps the run short).
const SIZES: [(u32, u32); 4] = [(0, 0), (40, 7), (25, 100), (300, 300)];

trait Common {
    fn common(self) -> Self;
    fn keyframes(self) -> Self;
}

impl Common for Diff {
    fn common(self) -> Diff {
        self.family(&facegen::FACEGEN)
            .family(&leaf::LEAVES)
            .ignore_calls(&PER_VERTEX)
            // The translations construct their stack point (`006815c0`, a leaf that only answers its
            // `this`); the typed loops keep the point in registers.
            .ignore_calls(&[0x0068_15c0])
            .local_at(0x00a6_7410, 1, 0xc)
            .allow_nan_payloads()
            .max_cases(160)
            .max_calls(200_000)
    }

    fn keyframes(self) -> Diff {
        self.family(&facegen::FACEGEN)
            .family(&leaf::LEAVES)
            .allow_nan_payloads()
            .max_calls(20_000)
    }
}

#[test]
fn facegen_differential_apply_morph() {
    Diff::new(entry!(
        0x0066_0860,
        morph::differential_apply(Ptr<_>, u32, u32, f32)
    ))
    .common()
    .inputs(&[
        Input::Of(&[0, 1, 2, 3]),
        Input::Of(&WEIGHTS),
        Input::Of(&[12, 16, 32]),
        Input::Of(&[0, 1, 2]),
        Input::Flag,
    ])
    .setup(|e, case: &Case| {
        stand_ins(e);
        let (vertices, count) = SIZES[case.word(0) as usize];
        let stride = case.word(2);
        let pattern = case.word(3);
        let geometry = face(e, vertices, stride, pattern, true);
        let morph = e.mem.alloc(0x10);
        let offsets = points(e, count, pattern, 7);
        e.mem.set_u32(morph + 4, offsets);
        e.mem.set_u32(morph + 8, count);
        let object = if case.word(4) == 0 { geometry } else { 0 };
        vec![morph, object, 0x5555, case.word(1)]
    })
    .run()
    .assert_same();
}

#[test]
fn facegen_statistical_apply_morph() {
    Diff::new(entry!(
        0x0066_0a90,
        morph::statistical_apply(Ptr<_>, u32, u32, f32)
    ))
    .common()
    .inputs(&[
        Input::Of(&[0, 1, 2, 3]),
        Input::Of(&WEIGHTS),
        Input::Of(&[12, 32]),
        Input::Of(&[0, 1, 2]),
        Input::Of(&[0, 1, 2]),
    ])
    .setup(|e, case: &Case| {
        stand_ins(e);
        let (vertices, count) = SIZES[case.word(0) as usize];
        let count = count.min(vertices);
        let stride = case.word(2);
        let pattern = case.word(3);
        let geometry = face(e, vertices, stride, pattern, true);
        // The basis: a row per vertex and then the morph's own rows.
        let rows = vertices + count + 4;
        let basis = points(e, rows, pattern, 3);
        let base = e.mem.alloc(0x10);
        let morph = e.mem.alloc(0x10);
        let indices = e.mem.alloc(4 * count.max(1));
        for i in 0..count {
            // Indices out of order and repeated, inside the vertices.
            e.mem
                .set_u32(indices + 4 * i, (i * 7 + 3) % vertices.max(1));
        }
        e.mem.set_u32(morph + 4, indices);
        e.mem.set_u32(morph + 8, count);
        e.mem.set_u32(morph + 0xc, vertices);
        // No object; no basis; both.
        let (object, with_basis) = match case.word(4) {
            0 => (geometry, true),
            1 => (0, true),
            _ => (geometry, false),
        };
        e.mem
            .set_u32(base + 0xc, if with_basis { basis } else { 0 });
        vec![morph, object, base, case.word(1)]
    })
    .run()
    .assert_same();
}

#[test]
fn facegen_copy_base() {
    Diff::new(entry!(0x0065_3340, morph::copy_base(u32, u32)))
        .common()
        .inputs(&[
            Input::Of(&[0, 1, 2, 3]),
            Input::Of(&[12, 32]),
            Input::Of(&[0, 1, 2]),
            // No extra data; no cursor array; the geometry without its data word; all there; no
            // geometry.
            Input::Of(&[0, 1, 2, 3, 4]),
        ])
        .setup(|e, case: &Case| {
            stand_ins(e);
            let (vertices, _) = SIZES[case.word(0) as usize];
            let stride = case.word(1);
            let pattern = case.word(2);
            let shape = case.word(3);
            let geometry = face(e, vertices, stride, pattern, shape != 2);
            let extra = e.mem.alloc(0x10);
            let offsets = points(e, vertices, pattern, 11);
            e.mem.set_u32(extra + 0xc, offsets);
            e.mem.set_u32(EXTRA, if shape == 0 { 0 } else { extra });
            let cursor = e.mem.alloc(0xc);
            let array = e.mem.u32(LOCKED_ARRAY);
            e.mem.set_u32(cursor, if shape == 1 { 0 } else { array });
            e.mem.set_u32(cursor + 4, stride);
            let geometry = if shape == 4 { 0 } else { geometry };
            vec![geometry, cursor]
        })
        .run()
        .assert_same();
}

// The keyframes.

/// Weights a face takes, and the awkward ones.
const KEY_WEIGHTS: [f32; 12] = [
    0.0,
    0.25,
    0.5,
    1.0,
    -0.5,
    1.5,
    f32::NAN,
    f32::MAX,
    0.0005,
    -0.0,
    1.0e-30,
    0.999_999_9,
];

/// The weight counts a face has (phonemes, expressions, modifiers), and none.
const COUNTS: [u32; 4] = [16, 15, 14, 0];

/// The vtables and their doubles (every keyframe test's setup).
fn keyframe_stand_ins(e: &mut Engine) {
    e.map(TEST, 0x1000);
    constants(e);
    let own: [(u32, u32); 9] = [
        (0x4, GET_TYPE),
        (0x8, 0x0064_eb00),
        (0xc, 0x0048_8d50),
        (0x10, 0x0064_eb30),
        (0x2c, 0x0064_f690),
        (0x30, 0x0064_f760),
        (0x44, IS_MULTIPLE),
        (0x4c, 0x0064_fb90),
        (0x50, 0x0064_fc00),
    ];
    for (slot, target) in own {
        e.mem.set_u32(OWN_VTABLE + slot, target);
        let other = match slot {
            0x2c => OTHER_COMPARE,
            0x30 => OTHER_STRENGTH,
            0x4c => OTHER_VALID,
            _ => target,
        };
        e.mem.set_u32(OTHER_VTABLE + slot, other);
        e.mem.set_u32(
            SINGLE_VTABLE + slot,
            if slot == 0x44 { IS_SINGLE } else { target },
        );
    }
    e.register(GET_TYPE, |e, a| ret(e.mem.u32(a[0] + 4)));
    e.register(IS_MULTIPLE, |_, _| ret(1));
    e.register(IS_SINGLE, |_, _| ret(0));
    // The other functions: `Compare` says "differs" unless the first weights are equal by their
    // bits; `GetStrength` is the first weight; `IsValidQuantity` takes the even indices in range.
    e.register(OTHER_COMPARE, |e, a| {
        let (this, other) = (a[0], a[1]);
        let first = |e: &Engine, k: u32| e.mem.u32(e.mem.u32(k + 0xc));
        let same = e.mem.u32(this + 0x10) != 0
            && e.mem.u32(other + 0x10) != 0
            && first(e, this) == first(e, other);
        ret(u32::from(!same))
    });
    e.register(OTHER_STRENGTH, |e, a| {
        let this = a[0];
        let st0 = if e.mem.u32(this + 0x10) == 0 {
            0.0
        } else {
            f64::from(e.mem.f32(e.mem.u32(this + 0xc)))
        };
        Ret {
            st0,
            ..Ret::default()
        }
    });
    e.register(OTHER_VALID, |e, a| {
        ret(u32::from(a[1] < e.mem.u32(a[0] + 0x10) && a[1] % 2 == 0))
    });
    // The settings `UpdateToTarget` reads (their value at +4).
    e.map(0x011d_5000, 0x1000);
}

/// A multiple keyframe on `vtable` with `count` weights from the pattern (`salt` moves it), of
/// type `kind`, time `time`.
fn keyframe_of(e: &mut Engine, vtable: u32, kind: u32, count: u32, pattern: u32, salt: u32) -> u32 {
    let k = e.mem.alloc(0x14);
    e.mem.set_u32(k, vtable);
    e.mem.set_u32(k + 4, kind);
    e.mem.set_f32(k + 8, 0.5 + pattern as f32);
    let array = e.mem.alloc(4 * count.max(1));
    for i in 0..count {
        let n = (i * 7 + salt * 5 + pattern * 3) as usize;
        e.mem
            .set_f32(array + 4 * i, KEY_WEIGHTS[n % KEY_WEIGHTS.len()]);
    }
    e.mem.set_u32(k + 0xc, array);
    e.mem.set_u32(k + 0x10, count);
    k
}

/// The second keyframe of a pair: like `first` (its weights, partly moved by the pattern), or of
/// another count, another type, a single keyframe, or null.
fn second_of(e: &mut Engine, first: u32, shape: u32, pattern: u32) -> u32 {
    let vtable = e.mem.u32(first);
    let kind = e.mem.u32(first + 4);
    let count = e.mem.u32(first + 0x10);
    match shape {
        // The same weights.
        0 => {
            let k = keyframe_of(e, vtable, kind, count, 0, 0);
            for i in 0..count {
                let w = e.mem.u32(e.mem.u32(first + 0xc) + 4 * i);
                e.mem.set_u32(e.mem.u32(k + 0xc) + 4 * i, w);
            }
            k
        }
        // Other weights.
        1 => keyframe_of(e, vtable, kind, count, pattern + 1, 3),
        2 => keyframe_of(e, vtable, kind, count + 1, pattern, 3),
        3 => keyframe_of(e, vtable, kind + 1, count, pattern, 3),
        4 => keyframe_of(e, SINGLE_VTABLE, kind, count, pattern, 3),
        5 => first,
        _ => 0,
    }
}

/// Each weight of `keyframe` times `by` (the sweep's full-mantissa floats reach the chains'
/// operands this way); `1.0` leaves them.
fn scale_weights(e: &mut Engine, keyframe: u32, by: f32) {
    let array = e.mem.u32(keyframe + 0xc);
    for i in 0..e.mem.u32(keyframe + 0x10) {
        let w = e.mem.f32(array + 4 * i) * by;
        e.mem.set_f32(array + 4 * i, w);
    }
}

/// Each NaN or infinite weight of `keyframe` (when there is one) set to 0.3.
fn finite_only(e: &mut Engine, keyframe: u32) {
    if keyframe == 0 {
        return;
    }
    let array = e.mem.u32(keyframe + 0xc);
    for i in 0..e.mem.u32(keyframe + 0x10) {
        if !e.mem.f32(array + 4 * i).is_finite() {
            e.mem.set_f32(array + 4 * i, 0.3);
        }
    }
}

const VTABLES: [u32; 2] = [OWN_VTABLE, OTHER_VTABLE];

#[test]
fn facegen_keyframe_quantity() {
    Diff::new(entry!(0x005d_d4e0, keyframe::quantity(Ptr<_>, u32) -> f32))
        .keyframes()
        .inputs(&[
            Input::Of(&[0, 1, 2, 3]),
            Input::Of(&[0, 1, 2]),
            Input::Of(&[0, 1, 13, 14, 15, 16, 0xffff_ffff]),
        ])
        .setup(|e, case: &Case| {
            keyframe_stand_ins(e);
            let k = keyframe_of(
                e,
                OWN_VTABLE,
                0,
                COUNTS[case.word(0) as usize],
                case.word(1),
                0,
            );
            vec![k, case.word(2)]
        })
        .run()
        .assert_same();
}

#[test]
fn facegen_keyframe_set_quantity() {
    let values: Vec<u32> = KEY_WEIGHTS.iter().map(|v| v.to_bits()).collect();
    let values: &'static [u32] = values.leak();
    Diff::new(entry!(
        0x0064_a4d0,
        keyframe::set_quantity(Ptr<_>, u32, f32) -> bool
    ))
    .keyframes()
    .inputs(&[
        Input::Of(&[0, 1, 3]),
        Input::Of(&[0, 1, 2]),
        Input::Of(&[0, 5, 13, 14, 15, 16, 0xffff_ffff]),
        Input::Of(values),
    ])
    .setup(|e, case: &Case| {
        keyframe_stand_ins(e);
        let k = keyframe_of(
            e,
            OWN_VTABLE,
            0,
            COUNTS[case.word(0) as usize],
            case.word(1),
            0,
        );
        vec![k, case.word(2), case.word(3)]
    })
    .run()
    .assert_same();
}

#[test]
fn facegen_keyframe_count() {
    Diff::new(entry!(0x0044_edb0, keyframe::word_at_10(Ptr<_>) -> u32))
        .keyframes()
        .inputs(&[Input::Of(&[0, 1, 2, 3])])
        .setup(|e, case: &Case| {
            keyframe_stand_ins(e);
            let k = keyframe_of(e, OWN_VTABLE, 0, COUNTS[case.word(0) as usize], 0, 0);
            vec![k]
        })
        .run()
        .assert_same();
}

#[test]
fn facegen_keyframe_is_valid_quantity() {
    Diff::new(entry!(
        0x0064_fb90,
        keyframe::is_valid_quantity(Ptr<_>, u32) -> bool
    ))
    .keyframes()
    .inputs(&[
        Input::Of(&[0, 1, 3]),
        Input::Of(&[0, 1, 2, 3, 4, 5]),
        Input::Of(&[
            0,
            1,
            2,
            3,
            4,
            5,
            6,
            7,
            8,
            9,
            10,
            11,
            14,
            15,
            16,
            0xffff_ffff,
        ]),
    ])
    .setup(|e, case: &Case| {
        keyframe_stand_ins(e);
        let k = keyframe_of(
            e,
            OWN_VTABLE,
            0,
            COUNTS[case.word(0) as usize],
            case.word(1),
            0,
        );
        vec![k, case.word(2)]
    })
    .run()
    .assert_same();
}

#[test]
fn facegen_keyframe_copy_from() {
    Diff::new(entry!(0x0064_f5b0, keyframe::copy_from(Ptr<_>, u32)))
        .keyframes()
        .inputs(&[
            Input::Of(&[0, 1, 2, 3]),
            Input::Of(&[0, 1, 2]),
            Input::Of(&[0, 1, 2, 3, 4, 5, 6]),
        ])
        .setup(|e, case: &Case| {
            keyframe_stand_ins(e);
            let first = keyframe_of(
                e,
                OWN_VTABLE,
                1,
                COUNTS[case.word(0) as usize],
                case.word(1),
                0,
            );
            let second = second_of(e, first, case.word(2), case.word(1));
            vec![first, second]
        })
        .run()
        .assert_same();
}

#[test]
fn facegen_keyframe_compare() {
    Diff::new(entry!(0x0064_f690, keyframe::compare(Ptr<_>, u32) -> bool))
        .keyframes()
        .inputs(&[
            Input::Of(&[0, 1, 2, 3]),
            Input::Of(&[0, 1, 2]),
            Input::Of(&[0, 1, 2, 3, 4, 5, 6]),
        ])
        .setup(|e, case: &Case| {
            keyframe_stand_ins(e);
            let first = keyframe_of(
                e,
                OWN_VTABLE,
                2,
                COUNTS[case.word(0) as usize],
                case.word(1),
                0,
            );
            let second = second_of(e, first, case.word(2), case.word(1));
            vec![first, second]
        })
        .run()
        .assert_same();
}

#[test]
fn facegen_keyframe_get_strength() {
    Diff::new(entry!(0x0064_f760, keyframe::get_strength(Ptr<_>) -> f64))
        .keyframes()
        .inputs(&[
            Input::Of(&[0, 1, 2, 3]),
            Input::Of(&[0, 1, 2, 3, 4, 5]),
            Input::Of(&[0, 1]),
        ])
        .setup(|e, case: &Case| {
            keyframe_stand_ins(e);
            let vtable = VTABLES[case.word(2) as usize];
            let k = keyframe_of(e, vtable, 0, COUNTS[case.word(0) as usize], case.word(1), 1);
            vec![k]
        })
        .run()
        .assert_same();
}

#[test]
fn facegen_keyframe_is_target_available() {
    Diff::new(entry!(
        0x0064_f880,
        keyframe::is_target_available(Ptr<_>) -> bool
    ))
    .keyframes()
    .inputs(&[
        Input::Of(&[0, 1, 2, 3]),
        Input::Of(&[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11]),
        // All the weights one of the awkward values.
        Input::Flag,
    ])
    .setup(|e, case: &Case| {
        keyframe_stand_ins(e);
        let count = COUNTS[case.word(0) as usize];
        let k = keyframe_of(e, OWN_VTABLE, 0, count, case.word(1), 2);
        if case.word(2) != 0 {
            let w = KEY_WEIGHTS[case.word(1) as usize];
            for i in 0..count {
                e.mem.set_f32(e.mem.u32(k + 0xc) + 4 * i, w);
            }
        }
        vec![k]
    })
    .run()
    .assert_same();
}

#[test]
fn facegen_keyframe_update_to_target() {
    Diff::new(entry!(
        0x0064_f900,
        keyframe::update_to_target(Ptr<_>, f32, u32) -> bool
    ))
    .keyframes()
    // `fabs` (`00408860`) of the difference is done in place.
    .ignore_calls(&[0x0040_8860])
    .inputs(&[
        Input::Of(&[0, 1, 2]),
        Input::Of(&[0, 1, 2]),
        Input::F32,
        Input::Of(&[0, 1, 2, 3, 4, 6]),
        Input::Of(&[0, 1]),
        // The settings (offset, scale): positive; zero; negative; large; the swept pair.
        Input::Of(&[0, 1, 2, 3, 4]),
        Input::F32Swept(0.2f32.to_bits()),
        Input::F32Swept(1.3f32.to_bits()),
        // Each weight of the first keyframe scaled by this (full mantissas in the sweep).
        Input::F32Swept(1.0f32.to_bits()),
    ])
    .float_sweep(256)
    .setup(|e, case: &Case| {
        keyframe_stand_ins(e);
        let vtable = VTABLES[case.word(4) as usize];
        let first = keyframe_of(e, vtable, 3, COUNTS[case.word(0) as usize], case.word(1), 0);
        let second = second_of(e, first, case.word(3), case.word(1));
        let (offset, scale) = match case.word(5) {
            0 => (0.2f32, 1.3f32),
            1 => (0.0, 0.0),
            2 => (-1.0, -2.0),
            3 => (7.0, 3.0e30),
            _ => (case.f32(6), case.f32(7)),
        };
        scale_weights(e, first, case.f32(8));
        // Finite weights only: the exe's `fabs` of a NaN difference goes on into the C runtime's
        // error path (`errno`), which these engines lack and which the native version leaves out.
        finite_only(e, first);
        finite_only(e, second);
        e.mem.set_f32(0x011d_57c8, offset);
        e.mem.set_f32(0x011d_5780, scale);
        vec![first, case.f32(2).to_bits(), second]
    })
    .run()
    .assert_same();
}

#[test]
fn facegen_keyframe_merge_morph_data_from() {
    Diff::new(entry!(
        0x0064_eea0,
        keyframe::merge_morph_data_from(Ptr<_>, u32, f32, u8, u8) -> bool
    ))
    .keyframes()
    .inputs(&[
        Input::Of(&[0, 1, 2]),
        Input::Of(&[0, 1, 2]),
        Input::Of(&[0, 1, 2, 3, 4, 5, 6]),
        Input::F32,
        Input::Flag,
        Input::Flag,
        // Each weight of both keyframes scaled by this (full mantissas in the sweep).
        Input::F32Swept(1.0f32.to_bits()),
    ])
    .float_sweep(256)
    .setup(|e, case: &Case| {
        keyframe_stand_ins(e);
        let first = keyframe_of(
            e,
            OWN_VTABLE,
            4,
            COUNTS[case.word(0) as usize],
            case.word(1),
            0,
        );
        let second = second_of(e, first, case.word(2), case.word(1));
        scale_weights(e, first, case.f32(6));
        if second != 0 && second != first {
            scale_weights(e, second, case.f32(6));
        }
        vec![
            first,
            second,
            case.f32(3).to_bits(),
            case.word(4),
            case.word(5),
        ]
    })
    .run()
    .assert_same();
}
