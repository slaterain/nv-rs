//! `NiBlendInterpolator::ComputeNormalizedWeights` and its case of exactly two interpolators: each
//! blended interpolator's share of a bone's transform, worked out again whenever a sequence's
//! weight or ease changed (flag 4). Native maths at the running thread's precision ([`Real`]), each
//! float the exe stores rounded to a float, in the exe's order of operations.
//!
//! Fields as the code reads them (Xbox PDB): the flags (+0xc: 2 only the highest weight, 4 the
//! weights out of date), the array size (+0xd), the interpolator count (+0xe), the single index
//! (+0xf), the high and next-high priorities (+0x10, +0x11, signed bytes), the item array (+0x14),
//! the weight threshold (+0x1c), the high and next-high sums of weights (+0x24, +0x28) and the high
//! ease spinner (+0x2c). An item (0x18 bytes): its interpolator (+0), weight (+4), normalized
//! weight (+8), priority (+0xc) and ease spinner (+0x10).

use super::FAMILY;
use crate::native::{float_precision, Real};
use core::cmp::Ordering;
use engine_core::ptr::Ptr;
use engine_core::{typed_call, Engine};

const FLAGS: u32 = 0xc;
const ONLY_USE_HIGHEST_WEIGHT: u8 = 2;
const COMPUTE_NORMALIZED_WEIGHTS: u8 = 4;
const ARRAY_SIZE: u32 = 0xd;
const INTERP_COUNT: u32 = 0xe;
const SINGLE_INDEX: u32 = 0xf;
const HIGH_PRIORITY: u32 = 0x10;
const NEXT_HIGH_PRIORITY: u32 = 0x11;
const INTERP_ARRAY: u32 = 0x14;
const WEIGHT_THRESHOLD: u32 = 0x1c;
const HIGH_SUM_OF_WEIGHTS: u32 = 0x24;
const NEXT_HIGH_SUM_OF_WEIGHTS: u32 = 0x28;
const HIGH_EASE_SPINNER: u32 = 0x2c;
const ITEM_SIZE: u32 = 0x18;
const ITEM_WEIGHT: u32 = 0x4;
const ITEM_NORMALIZED_WEIGHT: u32 = 0x8;
const ITEM_PRIORITY: u32 = 0xc;
const ITEM_EASE_SPINNER: u32 = 0x10;
/// 1.0, the weight of the one interpolator that counts.
const ONE: u32 = 0x3f80_0000;
/// The unset high sum (`-FLT_MAX`, the `double` at `010241b0`).
const UNSET: f32 = f32::MIN;
/// The largest weight's starting value (`01012054`, -1.0).
const MINUS_ONE: f32 = -1.0;

/// The blend interpolator's fields and items, read and written in guest memory.
struct Blend {
    this: u32,
}

impl Blend {
    fn byte(&self, e: &Engine, offset: u32) -> u8 {
        e.mem.u8(self.this.wrapping_add(offset))
    }

    fn at(&self, offset: u32) -> u32 {
        self.this.wrapping_add(offset)
    }

    /// Item `index`'s address (the array re-read, as the exe does each time).
    fn item(&self, e: &Engine, index: u32) -> u32 {
        e.mem
            .u32(self.at(INTERP_ARRAY))
            .wrapping_add(index.wrapping_mul(ITEM_SIZE))
    }
}

/// A float field as `R`.
#[inline(always)]
fn ld<R: Real>(e: &Engine, at: u32) -> R {
    R::load(e.mem.u32(at))
}

/// A float rounded as a store and a load round it.
#[inline(always)]
fn round<R: Real>(value: R) -> R {
    R::load(value.store())
}

/// The general case at precision `R` (more than two interpolators, or none).
fn compute<R: Real>(e: &mut Engine, b: &Blend) {
    let zero = R::ZERO;
    let size = u32::from(b.byte(e, ARRAY_SIZE));
    if f32::from_bits(e.mem.u32(b.at(HIGH_SUM_OF_WEIGHTS))) == UNSET {
        let mut high_sum = zero;
        let mut next_sum = zero;
        let mut high_spinner = 0u32;
        let high = b.byte(e, HIGH_PRIORITY);
        let next_high = b.byte(e, NEXT_HIGH_PRIORITY);
        for i in 0..size {
            let entry = b.item(e, i);
            if e.mem.u32(entry) == 0 {
                continue;
            }
            let ease_bits = e.mem.u32(entry.wrapping_add(ITEM_EASE_SPINNER));
            let weighted = round(ld::<R>(e, entry.wrapping_add(ITEM_WEIGHT)) * R::load(ease_bits));
            let priority = e.mem.u8(entry.wrapping_add(ITEM_PRIORITY));
            if priority == high {
                high_sum = round(high_sum + weighted);
                if R::load(high_spinner) < R::load(ease_bits) {
                    high_spinner = ease_bits;
                }
            } else if priority == next_high {
                next_sum = round(weighted + next_sum);
            }
        }
        e.mem.set_u32(b.at(HIGH_SUM_OF_WEIGHTS), high_sum.store());
        e.mem
            .set_u32(b.at(NEXT_HIGH_SUM_OF_WEIGHTS), next_sum.store());
        e.mem.set_u32(b.at(HIGH_EASE_SPINNER), high_spinner);
    }
    let high_spinner = ld::<R>(e, b.at(HIGH_EASE_SPINNER));
    let rest = round(R::ONE - high_spinner);
    let next_part = ld::<R>(e, b.at(NEXT_HIGH_SUM_OF_WEIGHTS)) * rest;
    let high_part = ld::<R>(e, b.at(HIGH_SUM_OF_WEIGHTS)) * high_spinner;
    let total = round(next_part + high_part);
    let reciprocal = round(if total > zero { R::ONE / total } else { zero });
    let high = b.byte(e, HIGH_PRIORITY);
    let next_high = b.byte(e, NEXT_HIGH_PRIORITY);
    for i in 0..size {
        let entry = b.item(e, i);
        if e.mem.u32(entry) == 0 {
            continue;
        }
        let priority = e.mem.u8(entry.wrapping_add(ITEM_PRIORITY));
        let weight = ld::<R>(e, entry.wrapping_add(ITEM_WEIGHT));
        let ease = ld::<R>(e, entry.wrapping_add(ITEM_EASE_SPINNER));
        let normalized = if priority == high {
            (weight * high_spinner * ease * reciprocal).store()
        } else if priority == next_high {
            (weight * rest * ease * reciprocal).store()
        } else {
            0
        };
        e.mem
            .set_u32(entry.wrapping_add(ITEM_NORMALIZED_WEIGHT), normalized);
    }
    let threshold = ld::<R>(e, b.at(WEIGHT_THRESHOLD));
    if zero < threshold {
        let mut sum = zero;
        for i in 0..size {
            let entry = b.item(e, i);
            let at = entry.wrapping_add(ITEM_NORMALIZED_WEIGHT);
            if e.mem.u32(entry) == 0 || ld::<R>(e, at) == zero {
                continue;
            }
            if threshold > ld::<R>(e, at) {
                e.mem.set_u32(at, 0);
            }
            sum = round(ld::<R>(e, at) + sum);
        }
        if sum != R::ONE {
            let factor = if sum > zero {
                round(R::ONE / sum)
            } else {
                zero
            };
            for i in 0..size {
                let at = b.item(e, i).wrapping_add(ITEM_NORMALIZED_WEIGHT);
                if ld::<R>(e, at) != zero {
                    e.mem.set_u32(at, (ld::<R>(e, at) * factor).store());
                }
            }
        }
    }
    if b.byte(e, FLAGS) & ONLY_USE_HIGHEST_WEIGHT == 0 {
        return;
    }
    let mut highest = MINUS_ONE;
    let mut index = 0xffu32;
    for i in 0..size {
        let at = b.item(e, i).wrapping_add(ITEM_NORMALIZED_WEIGHT);
        let weight = f32::from_bits(e.mem.u32(at));
        if highest < weight {
            highest = weight;
            index = i;
        }
        e.mem.set_u32(at, 0);
    }
    let chosen = b.item(e, index);
    e.mem
        .set_u32(chosen.wrapping_add(ITEM_NORMALIZED_WEIGHT), ONE);
}

// Typed from 00a37260
/// `NiBlendInterpolator::ComputeNormalizedWeights` (Xbox PDB; `this` in `ECX`): only when the
/// weights are out of date (flag 4, then cleared). One interpolator: its normalized weight 1. Two:
/// [`compute_for_two`]. Otherwise, when the high sum is unset (`-FLT_MAX`), the sums of `weight *
/// ease spinner` of the highest- and next-highest-priority items and the largest ease spinner among
/// the highest are worked out again (from 0); each used item's normalized weight is then `weight *
/// spinner * ease * r` (highest priority), `weight * (1 - spinner) * ease * r` (next highest) or 0,
/// with `r` the reciprocal of `high * spinner + next * (1 - spinner)` (0 when that is not
/// positive). With a positive threshold, weights below it are zeroed and every weight scaled to sum
/// to 1 (by 0 when the sum is not positive; untouched when it is exactly 1). With "only the highest
/// weight" (flag 2) the first largest weight becomes 1 and every other 0 (the exe writes item 0xff
/// for an empty array or when no weight is above -1).
pub fn compute_normalized_weights(e: &mut Engine, this: Ptr) {
    let b = Blend { this: this.addr() };
    let flags = b.byte(e, FLAGS);
    if flags & COMPUTE_NORMALIZED_WEIGHTS == 0 {
        return;
    }
    e.mem
        .set_u8(b.at(FLAGS), flags & !COMPUTE_NORMALIZED_WEIGHTS);
    match b.byte(e, INTERP_COUNT) {
        1 => {
            let single = b.item(e, u32::from(b.byte(e, SINGLE_INDEX)));
            e.mem
                .set_u32(single.wrapping_add(ITEM_NORMALIZED_WEIGHT), ONE);
        }
        2 => typed_call!(FAMILY, 0x00a3_6bd0, compute_for_two(e, this)),
        _ if float_precision() => compute::<f32>(e, &b),
        _ => compute::<f64>(e, &b),
    }
}

/// [`compute_for_two`] at precision `R`.
fn for_two<R: Real>(e: &mut Engine, b: &Blend) {
    let size = u32::from(b.byte(e, ARRAY_SIZE));
    let mut first = 0u32;
    let mut second = 0u32;
    for i in 0..size {
        let candidate = b.item(e, i);
        if e.mem.u32(candidate) != 0 {
            if first != 0 {
                second = candidate;
                break;
            }
            first = candidate;
        }
    }
    let f = |field: u32| first.wrapping_add(field);
    let s = |field: u32| second.wrapping_add(field);
    let wa = round(ld::<R>(e, f(ITEM_WEIGHT)) * ld::<R>(e, f(ITEM_EASE_SPINNER)));
    let wb = round(ld::<R>(e, s(ITEM_WEIGHT)) * ld::<R>(e, s(ITEM_EASE_SPINNER)));
    let set = |e: &mut Engine, a: u32, b: u32| {
        e.mem.set_u32(f(ITEM_NORMALIZED_WEIGHT), a);
        e.mem.set_u32(s(ITEM_NORMALIZED_WEIGHT), b);
    };
    if wa == R::ZERO && wb == R::ZERO {
        set(e, 0, 0);
        return;
    }
    let priority_a = e.mem.u8(f(ITEM_PRIORITY)) as i8;
    let priority_b = e.mem.u8(s(ITEM_PRIORITY)) as i8;
    let (a, b2) = if priority_a > priority_b {
        if f32::from_bits(e.mem.u32(f(ITEM_EASE_SPINNER))) == 1.0 {
            set(e, ONE, 0);
            return;
        }
        let ease = ld::<R>(e, f(ITEM_EASE_SPINNER));
        let rest = round(R::ONE - ease);
        let weighted_a = wa * ease;
        let weighted_b = wb * rest;
        let inverse = round(R::ONE / (weighted_b + weighted_a));
        (
            (weighted_a * inverse).store(),
            (weighted_b * inverse).store(),
        )
    } else if priority_a < priority_b {
        if f32::from_bits(e.mem.u32(s(ITEM_EASE_SPINNER))) == 1.0 {
            set(e, 0, ONE);
            return;
        }
        let ease = ld::<R>(e, s(ITEM_EASE_SPINNER));
        let rest = round(R::ONE - ease);
        let weighted_a = wa * rest;
        let inverse = round(R::ONE / (ease * wb + weighted_a));
        let a = (weighted_a * inverse).store();
        (a, (inverse * (wb * ease)).store())
    } else {
        let inverse = round(R::ONE / (wb + wa));
        ((wa * inverse).store(), (wb * inverse).store())
    };
    set(e, a, b2);
    if b.byte(e, FLAGS) & ONLY_USE_HIGHEST_WEIGHT != 0 {
        if R::load(b2) <= R::load(a) {
            set(e, ONE, 0);
        } else {
            set(e, 0, ONE);
        }
        return;
    }
    let threshold = ld::<R>(e, b.at(WEIGHT_THRESHOLD));
    if threshold.partial_cmp(&R::ZERO) != Some(Ordering::Greater) {
        return;
    }
    let mut zeroed = false;
    if threshold > R::load(a) {
        e.mem.set_u32(f(ITEM_NORMALIZED_WEIGHT), 0);
        zeroed = true;
    }
    if threshold > ld::<R>(e, s(ITEM_NORMALIZED_WEIGHT)) {
        // The exe stores the two values left on its stack, 0 then 1.0, both into B's weight.
        e.mem.set_u32(s(ITEM_NORMALIZED_WEIGHT), ONE);
        return;
    }
    if zeroed {
        e.mem.set_u32(s(ITEM_NORMALIZED_WEIGHT), ONE);
    }
}

// Typed from 00a36bd0
/// `ComputeNormalizedWeights`' case of exactly two interpolators (no Xbox PDB name; `this` in
/// `ECX`): the first two used items A and B (the exe reads through null when there are fewer) and
/// their weights times ease spinners `wa`, `wb` (rounded to floats). Both 0: both normalized
/// weights 0. Otherwise by priority (signed): A's higher with A's spinner 1 gives (1, 0), else
/// `wa * ea` and `wb * (1 - ea)` normalized; B's higher the same mirrored ((0, 1) for B's spinner 1,
/// else `wa * (1 - eb)` and `eb * wb`); equal priorities `wa` and `wb` normalized (each through the
/// reciprocal of the sum, rounded to a float). Then with "only the highest weight" (flag 2) the
/// larger gets 1 and the other 0 (A on a tie, B when unordered); otherwise with a positive
/// threshold A below it is zeroed, and B gets 1 when either was below it.
pub fn compute_for_two(e: &mut Engine, this: Ptr) {
    let b = Blend { this: this.addr() };
    if float_precision() {
        for_two::<f32>(e, &b);
    } else {
        for_two::<f64>(e, &b);
    }
}
