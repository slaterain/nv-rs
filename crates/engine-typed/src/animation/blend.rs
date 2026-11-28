//! `NiBlendTransformInterpolator`: a bone's transform from one interpolator, or several blended by
//! their weights, and `NiQuaternion::Normalize`, which the single value's rotation takes. Its
//! fields as the code reads them (`NiBlendInterpolator`, Xbox PDB): the last time (+8), the flags
//! (+0xc, bit 0: manager controlled), the array size (+0xd), the interpolator count (+0xe), the
//! item array (+0x14), the single interpolator (+0x18) and the single time (+0x20); an item (0x18
//! bytes): its interpolator (+0), normalized weight (+8) and update time (+0x14).
//!
//! The blend of several (`BlendValues`) and the normalisation are native maths at the running
//! thread's precision ([`Real`]), each float the exe stores rounded to a float. The square roots
//! are the C runtime's (`_CIsqrt`, a typed call); each interpolator's own `Update` (slot +0x8c)
//! stays a virtual call, since a built-in may replace `NiTransformInterpolator::Update`, and so
//! does `ComputeNormalizedWeights` where `Update` calls it.

use super::FAMILY;
use crate::crt;
use crate::native::{float_precision, Real};
use engine_core::ptr::Ptr;
use engine_core::{args, typed_call, Engine};

/// The fields.
const LAST_TIME: u32 = 0x8;
const FLAGS: u32 = 0xc;
const MANAGER_CONTROLLED: u8 = 1;
const ARRAY_SIZE: u32 = 0xd;
const INTERP_COUNT: u32 = 0xe;
const INTERP_ARRAY: u32 = 0x14;
const SINGLE_INTERPOLATOR: u32 = 0x18;
const SINGLE_TIME: u32 = 0x20;
const ITEM_SIZE: u32 = 0x18;
const ITEM_NORMALIZED_WEIGHT: u32 = 0x8;
const ITEM_UPDATE_TIME: u32 = 0x14;
/// `NiQuatTransform` (Xbox PDB): the translation (+0), the rotation (+0xc, four floats, `X` at
/// +0x10) and the scale (+0x1c).
const ROTATE: u32 = 0xc;
const ROTATE_X: u32 = 0x10;
const SCALE: u32 = 0x1c;
/// `NiInterpolator::INVALID_FLOAT` (`0109696c`, `-FLT_MAX`), the float written into invalid parts
/// (`01015f5c`, `-FLT_MAX`) and the `double` `-FLT_MAX` (`010241b0`) the parts are compared with.
const INVALID_FLOAT: f32 = f32::MIN;
const INVALID_TRANSFORM_PART: u32 = 0xff7f_ffff;
const UNSET: f32 = f32::MIN;
/// The smallest scale weight a blended scale is divided by (the `double` 0.0001 at `01030f28`).
const SMALLEST_SCALE_WEIGHT: u64 = 0x3f1a_36e2_eb1c_432d;
/// Where the blend starts its sums (`011f426c` the zero point, `011f49ec` the identity quaternion)
/// and the fresh transform each interpolator is handed (`011a8400` the invalid point, `011f3704`
/// the invalid quaternion, `01096b8c` the invalid scale): read where they are.
const START_TRANSLATE: u32 = 0x011f_426c;
const START_ROTATE: u32 = 0x011f_49ec;
const FRESH_TRANSLATE: u32 = 0x011a_8400;
const FRESH_ROTATE: u32 = 0x011f_3704;
const INVALID_SCALE: u32 = 0x0109_6b8c;
/// `NiBlendInterpolator::ComputeNormalizedWeights` (`00a37260`): a built-in may replace it, so
/// `Update` calls it by address.
const COMPUTE_NORMALIZED_WEIGHTS: u32 = 0x00a3_7260;
/// An interpolator's transform `Update` (slot +0x8c, `(time, object, transform)`, a byte).
const SLOT_TRANSFORM_UPDATE: u32 = 0x8c;

/// A float word as `R`.
#[inline(always)]
fn ld<R: Real>(e: &Engine, at: u32) -> R {
    R::load(e.mem.u32(at))
}

/// A float rounded as a store and a load round it.
#[inline(always)]
fn round<R: Real>(value: R) -> R {
    R::load(value.store())
}

/// The C runtime's root of a float (`_CIsqrt`, typed), at precision `R`, rounded to a float.
fn root<R: Real>(e: &mut Engine, value: R) -> R {
    let wide = value.wide();
    let answer = typed_call!(crt::FAMILY, 0x00ec_6040, crt::math::ci_sqrt(e, wide));
    round(R::narrow(answer))
}

// Typed from 00a41110
/// `NiBlendTransformInterpolator::Update` (`thiscall(this, time, object, transform)`): with one
/// interpolator [`store_single_value`], with more `ComputeNormalizedWeights` (by address) then
/// [`blend_values`], with none nothing (false); the last time becomes `time`. Answers what the
/// store or blend answered.
pub fn update_transform(e: &mut Engine, this: Ptr, time: f32, object: Ptr, transform: Ptr) -> bool {
    let count = e.mem.u8(this.addr().wrapping_add(INTERP_COUNT));
    let answer = if count == 1 {
        typed_call!(
            FAMILY,
            0x00a4_0b30,
            store_single_value(e, this, time, object, transform)
        )
    } else if count > 0 {
        e.call(COMPUTE_NORMALIZED_WEIGHTS, &args![this]);
        typed_call!(
            FAMILY,
            0x00a4_0c10,
            blend_values(e, this, time, object, transform)
        )
    } else {
        false
    };
    e.mem
        .set_u32(this.addr().wrapping_add(LAST_TIME), time.to_bits());
    answer
}

/// The invalid marker in a transform's translation, rotation (its float at +0x10) and scale.
fn invalidate(e: &mut Engine, transform: u32) {
    for offset in [0, ROTATE_X, SCALE] {
        e.mem
            .set_u32(transform.wrapping_add(offset), INVALID_TRANSFORM_PART);
    }
}

// Typed from 00a40b30
/// `NiBlendTransformInterpolator::StoreSingleValue` (`thiscall(this, time, object, transform)`):
/// the single interpolator's transform at `time` (the single time when manager controlled). At the
/// invalid time, or when its `Update` (slot +0x8c) gives nothing, the transform is marked invalid
/// and the answer is false; otherwise its rotation is normalized ([`normalize`] on a copy, unless
/// the rotation's `X` is `-FLT_MAX`) and the answer is true.
pub fn store_single_value(
    e: &mut Engine,
    this: Ptr,
    time: f32,
    object: Ptr,
    transform: Ptr,
) -> bool {
    let mut time_bits = time.to_bits();
    if e.mem.u8(this.addr().wrapping_add(FLAGS)) & MANAGER_CONTROLLED != 0 {
        time_bits = e.mem.u32(this.addr().wrapping_add(SINGLE_TIME));
    }
    if f32::from_bits(time_bits) == INVALID_FLOAT {
        invalidate(e, transform.addr());
        return false;
    }
    let single = e.mem.u32(this.addr().wrapping_add(SINGLE_INTERPOLATOR));
    let gave = e
        .vcall(
            single,
            SLOT_TRANSFORM_UPDATE,
            &args![time_bits, object, transform],
        )
        .u8()
        != 0;
    if !gave {
        invalidate(e, transform.addr());
        return false;
    }
    let rotation = transform.addr().wrapping_add(ROTATE);
    if f32::from_bits(e.mem.u32(rotation.wrapping_add(4))) != UNSET {
        e.with_stack(16, |e, copy| {
            for word in 0..4u32 {
                let value = e.mem.u32(rotation.wrapping_add(4 * word));
                e.mem.set_u32(copy.addr().wrapping_add(4 * word), value);
            }
            typed_call!(FAMILY, 0x00a3_8560, normalize(e, copy));
            for word in 0..4u32 {
                let value = e.mem.u32(copy.addr().wrapping_add(4 * word));
                e.mem.set_u32(rotation.wrapping_add(4 * word), value);
            }
        });
    }
    true
}

/// [`normalize`] at precision `R`.
fn normalized<R: Real>(e: &mut Engine, q: [u32; 4]) -> [u32; 4] {
    let [w, x, y, z] = q.map(R::load);
    let sum = round(((x * x + w * w) + y * y) + z * z);
    let length = root(e, sum);
    let inverse = round(R::ONE / length);
    [
        (w * inverse).store(),
        (inverse * x).store(),
        (y * inverse).store(),
        (inverse * z).store(),
    ]
}

// Typed from 00a38560
/// `NiQuaternion::Normalize` (Xbox PDB; `this` in `ECX`): divides the four floats at `this` (`W` at
/// +0, `X`, `Y`, `Z` after it) by the square root (`_CIsqrt`) of their sum of squares, summed as
/// `((x x + w w) + y y) + z z`; the sum, the root and its reciprocal stored as floats.
pub fn normalize(e: &mut Engine, this: Ptr) {
    let at = this.addr();
    let q = [0u32, 4, 8, 0xc].map(|o| e.mem.u32(at.wrapping_add(o)));
    let result = if float_precision() {
        normalized::<f32>(e, q)
    } else {
        normalized::<f64>(e, q)
    };
    for (i, word) in (0u32..).zip(result) {
        e.mem.set_u32(at.wrapping_add(4 * i), word);
    }
}

/// The blend's running sums.
struct Sums<R> {
    translate_weight: R,
    scale_weight: R,
    scale: R,
    translate: [R; 3],
    rotate: [R; 4],
    translated: bool,
    rotated: bool,
    scaled: bool,
    first_rotation: bool,
}

/// One interpolator's transform `t` (eight words) added to the sums with the weight at `weight_at`
/// (read where the exe reads it, after the interpolator's `Update`).
fn add_part<R: Real>(e: &Engine, s: &mut Sums<R>, t: [u32; 8], weight_at: u32) {
    let weight = |e: &Engine| ld::<R>(e, weight_at);
    let part = t.map(R::load);
    if f32::from_bits(t[0]) == UNSET {
        s.translate_weight = round(s.translate_weight - weight(e));
    } else {
        let w = weight(e);
        s.translated = true;
        let p = [round(part[0] * w), round(part[1] * w), round(w * part[2])];
        s.translate[0] = round(p[0] + s.translate[0]);
        s.translate[1] = round(s.translate[1] + p[1]);
        s.translate[2] = round(s.translate[2] + p[2]);
    }
    if f32::from_bits(t[4]) != UNSET {
        let mut q = [part[3], part[4], part[5], part[6]];
        if s.first_rotation {
            s.first_rotation = false;
        } else {
            let r = &s.rotate;
            let dot = q[1] * r[1] + q[0] * r[0];
            let dot = round(dot + q[2] * r[2] + q[3] * r[3]);
            if dot < R::ZERO {
                q = q.map(|v| -v);
            }
        }
        let w = weight(e);
        s.rotated = true;
        let p = [round(q[0] * w), round(q[1] * w), round(q[2] * w)];
        s.rotate[0] = round(p[0] + s.rotate[0]);
        s.rotate[1] = round(s.rotate[1] + p[1]);
        s.rotate[2] = round(s.rotate[2] + p[2]);
        let p3 = round(w * q[3]);
        s.rotate[3] = round(p3 + s.rotate[3]);
    }
    if f32::from_bits(t[7]) == UNSET {
        s.scale_weight = round(s.scale_weight - weight(e));
    } else {
        s.scaled = true;
        s.scale = round(part[7] * weight(e) + s.scale);
    }
}

/// [`blend_values`] at precision `R`.
fn blend<R: Real>(e: &mut Engine, this: u32, time: f32, object: u32, out: u32) -> bool {
    let words = |e: &Engine, at: u32, n: u32| -> Vec<u32> {
        (0..n).map(|i| e.mem.u32(at.wrapping_add(4 * i))).collect()
    };
    let start_t = words(e, START_TRANSLATE, 3);
    let start_r = words(e, START_ROTATE, 4);
    let mut s = Sums {
        translate_weight: R::ONE,
        scale_weight: R::ONE,
        scale: R::ZERO,
        translate: [0, 1, 2].map(|i| R::load(start_t[i])),
        rotate: [0, 1, 2, 3].map(|i| R::load(start_r[i])),
        translated: false,
        rotated: false,
        scaled: false,
        first_rotation: true,
    };
    let mut index = 0u32;
    while index < u32::from(e.mem.u8(this.wrapping_add(ARRAY_SIZE))) {
        let item = e
            .mem
            .u32(this.wrapping_add(INTERP_ARRAY))
            .wrapping_add(index * ITEM_SIZE);
        index += 1;
        let interpolator = e.mem.u32(item);
        if interpolator == 0 {
            continue;
        }
        let weight_at = item.wrapping_add(ITEM_NORMALIZED_WEIGHT);
        let weight = f32::from_bits(e.mem.u32(weight_at));
        if weight.partial_cmp(&0.0) != Some(core::cmp::Ordering::Greater) {
            continue;
        }
        let both_off = |e: &Engine, s: &mut Sums<R>| {
            let w = ld::<R>(e, weight_at);
            s.translate_weight = round(s.translate_weight - w);
            s.scale_weight = round(s.scale_weight - w);
        };
        let mut item_time = time.to_bits();
        if e.mem.u8(this.wrapping_add(FLAGS)) & MANAGER_CONTROLLED != 0 {
            item_time = e.mem.u32(item.wrapping_add(ITEM_UPDATE_TIME));
        }
        if f32::from_bits(item_time) == INVALID_FLOAT {
            both_off(e, &mut s);
            continue;
        }
        let fresh_t = words(e, FRESH_TRANSLATE, 3);
        let fresh_r = words(e, FRESH_ROTATE, 4);
        let fresh_s = e.mem.u32(INVALID_SCALE);
        let (gave, part) = e.with_stack(0x20, |e, local| {
            let local = local.addr();
            for (i, word) in (0u32..).zip(fresh_t.iter().chain(&fresh_r)) {
                e.mem.set_u32(local + 4 * i, *word);
            }
            e.mem.set_u32(local + SCALE, fresh_s);
            let gave = e
                .vcall(
                    interpolator,
                    SLOT_TRANSFORM_UPDATE,
                    &args![item_time, object, local],
                )
                .bool();
            let mut part = [0u32; 8];
            for (i, word) in (0u32..).zip(part.iter_mut()) {
                *word = e.mem.u32(local + 4 * i);
            }
            (gave, part)
        });
        if !gave {
            both_off(e, &mut s);
            continue;
        }
        add_part(e, &mut s, part, weight_at);
    }
    invalidate(e, out);
    if s.translated || s.rotated || s.scaled {
        let total = s.translate_weight;
        if total == R::ZERO {
            s.translated = false;
        }
        if s.scale_weight.wide() < f64::from_bits(SMALLEST_SCALE_WEIGHT) {
            s.scaled = false;
        }
        if s.translated {
            let reciprocal = round(R::ONE / total);
            let t = &s.translate;
            e.mem.set_u32(out, (reciprocal * t[0]).store());
            e.mem.set_u32(out + 4, (t[1] * reciprocal).store());
            e.mem.set_u32(out + 8, (reciprocal * t[2]).store());
        }
        if s.rotated {
            let q = s.rotate;
            let sum = round(((q[1] * q[1] + q[0] * q[0]) + q[2] * q[2]) + q[3] * q[3]);
            let length = root(e, sum);
            let reciprocal = round(R::ONE / length);
            e.mem.set_u32(out + ROTATE, (reciprocal * q[0]).store());
            e.mem.set_u32(out + ROTATE + 4, (q[1] * reciprocal).store());
            e.mem.set_u32(out + ROTATE + 8, (q[2] * reciprocal).store());
            e.mem
                .set_u32(out + ROTATE + 12, (reciprocal * q[3]).store());
        }
        if s.scaled {
            e.mem
                .set_u32(out + SCALE, (s.scale / s.scale_weight).store());
        }
    }
    let unset = |offset: u32| f32::from_bits(e.mem.u32(out.wrapping_add(offset))) == UNSET;
    !unset(SCALE) || !unset(ROTATE_X) || !unset(0)
}

// Typed from 00a40c10
/// `NiBlendTransformInterpolator::BlendValues` (Xbox PDB; `thiscall(this, time, object,
/// transform)`): blends the transforms of the items with a positive normalized weight. Each item's
/// interpolator gives a transform at `time` (its update time when manager controlled) through
/// `Update` (slot +0x8c) into a fresh local transform; at the invalid time or without a value its
/// weight is taken off both the translation and the scale weight totals (from 1). Each valid part
/// is summed weighted: the translation (else its weight comes off the translation total), the
/// rotation (each after the first negated when its dot product with the running sum is negative),
/// the scale (else its weight comes off the scale total). The result starts invalid; a summed
/// translation is divided by its total (unless that is 0), the rotation sum normalized (`1 / sqrt`
/// of its squared length, `_CIsqrt`), the scale sum divided by its total (unless that is below
/// 0.0001). Answers whether any part of the result is valid.
pub fn blend_values(e: &mut Engine, this: Ptr, time: f32, object: Ptr, out: Ptr) -> bool {
    let (this, object, out) = (this.addr(), object.addr(), out.addr());
    if float_precision() {
        blend::<f32>(e, this, time, object, out)
    } else {
        blend::<f64>(e, this, time, object, out)
    }
}
