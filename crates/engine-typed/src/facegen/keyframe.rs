//! `BSFaceGenKeyframeMultiple` (0x14 bytes: vtable, `eType` +4, `fTime` +8, `pQuantities` +0xc,
//! `iNumQuantities` +0x10): a face's array of morph weights, which `BSFaceGenAnimationData::Update`
//! moves toward its targets, copies and merges for every head every frame. The translations make a
//! call by address for every weight they read or write (the folded getter `005dd4e0`, the setter
//! `0064a4d0`, the count `0044edb0`) and a virtual call for every validity test; here those are
//! direct (`typed_call!`), and the class's own virtual functions are called directly when the slot
//! holds them (any other target is called as the exe calls it).
//!
//! Native floats (ADR-0014's float rule): the compares are exact in any width; a result that is one
//! operation on two floats, stored, is plain `f32`; the two chains (`UpdateToTarget`'s step and the
//! merge's blend) run in [`Real`] at the running thread's precision and are stored once, where the
//! exe stores them.

use super::FAMILY;
use crate::native::{float_precision, to_float, Real};
use engine_core::ptr::Ptr;
use engine_core::x87::x87_quiet_bits;
use engine_core::{args, typed_call, Engine};

/// The fields.
const QUANTITIES: u32 = 0xc;
const COUNT: u32 = 0x10;
/// The virtual slots: `GetType` (+0x4), `SetType` (+0x8), `GetTime` (+0xc), `SetTime` (+0x10),
/// `Compare` (+0x2c), `GetStrength` (+0x30), `IsMultiple` (+0x44), `IsValidQuantity` (+0x4c),
/// `ResetQuantityArray` (+0x50); and this class's own functions in them.
const SLOT_GET_TYPE: u32 = 0x4;
const SLOT_SET_TYPE: u32 = 0x8;
const SLOT_GET_TIME: u32 = 0xc;
const SLOT_SET_TIME: u32 = 0x10;
const SLOT_COMPARE: u32 = 0x2c;
const SLOT_GET_STRENGTH: u32 = 0x30;
const SLOT_IS_MULTIPLE: u32 = 0x44;
const SLOT_IS_VALID_QUANTITY: u32 = 0x4c;
const SLOT_RESET_QUANTITY_ARRAY: u32 = 0x50;
const OWN_COMPARE: u32 = 0x0064_f690;
const OWN_GET_STRENGTH: u32 = 0x0064_f760;
const OWN_IS_VALID_QUANTITY: u32 = 0x0064_fb90;
/// `FLT_MAX` (the exe compares with it as a double, `010231b0`, and stores the float `01016970`;
/// both are `f32::MAX`), the merge's tolerance 0.001 (float, `01017d00`). The exe's 0.0 and 1.0
/// doubles (`01012060`, `01012070`) are the literals.
const TOLERANCE: u32 = 0x0101_7d00;
/// The nearly-equal test (`0040eb10`, cdecl, three floats), by address.
const NEARLY_EQUAL: u32 = 0x0040_eb10;
/// The settings `UpdateToTarget`'s step reads (`00403e20` gives a float setting's address).
const SETTING_VALUE: u32 = 0x0040_3e20;
const SETTING_OFFSET: u32 = 0x011d_57c4;
const SETTING_SCALE: u32 = 0x011d_577c;

// Typed from 005dd4e0
/// Element `index` of the float array (`thiscall(this, index)`: pointer at +0xc, count at +0x10), or
/// 0.0 when the index is out of range (unsigned). A folded body: every caller of the address gets it.
pub fn quantity(e: &mut Engine, this: Ptr, index: u32) -> f32 {
    let t = this.addr();
    if index < e.mem.u32(t.wrapping_add(COUNT)) {
        let data = e.mem.u32(t.wrapping_add(QUANTITIES));
        e.mem.f32(data.wrapping_add(index.wrapping_mul(4)))
    } else {
        0.0
    }
}

// Typed from 0064a4d0
/// Sets weight `index` to `value` (`thiscall(this, index, value)`): false when the index is out of
/// range or the value already equal (a NaN never is), true after storing it (quieted, as `FSTP`).
pub fn set_quantity(e: &mut Engine, this: Ptr, index: u32, value: f32) -> bool {
    let t = this.addr();
    if index >= e.mem.u32(t.wrapping_add(COUNT)) {
        return false;
    }
    let slot = e
        .mem
        .u32(t.wrapping_add(QUANTITIES))
        .wrapping_add(index.wrapping_mul(4));
    if value == e.mem.f32(slot) {
        return false;
    }
    e.mem.set_u32(slot, x87_quiet_bits(value.to_bits()));
    true
}

// Typed from 0044edb0
/// The word at +0x10 (`thiscall`): a keyframe's weight count. A folded body: every caller of the
/// address gets it.
pub fn word_at_10(e: &mut Engine, this: Ptr) -> u32 {
    e.mem.u32(this.addr().wrapping_add(COUNT))
}

#[inline(always)]
fn get(e: &mut Engine, keyframe: u32, index: u32) -> f32 {
    typed_call!(FAMILY, 0x005d_d4e0, quantity(e, Ptr::new(keyframe), index))
}

#[inline(always)]
fn set(e: &mut Engine, keyframe: u32, index: u32, value: f32) -> u8 {
    u8::from(typed_call!(
        FAMILY,
        0x0064_a4d0,
        set_quantity(e, Ptr::new(keyframe), index, value)
    ))
}

#[inline(always)]
fn count(e: &mut Engine, keyframe: u32) -> u32 {
    typed_call!(FAMILY, 0x0044_edb0, word_at_10(e, Ptr::new(keyframe)))
}

/// Weight `index` read in place.
#[inline(always)]
fn weight_at(e: &Engine, keyframe: u32, index: u32) -> f32 {
    let array = e.mem.u32(keyframe.wrapping_add(QUANTITIES));
    e.mem.f32(array.wrapping_add(index.wrapping_mul(4)))
}

/// `IsValidQuantity` (+0x4c): this class's directly when the slot holds it.
fn valid(e: &mut Engine, keyframe: u32, index: u32) -> bool {
    if crate::slot_target(e, keyframe, SLOT_IS_VALID_QUANTITY) == OWN_IS_VALID_QUANTITY {
        typed_call!(
            FAMILY,
            0x0064_fb90,
            is_valid_quantity(e, Ptr::new(keyframe), index)
        )
    } else {
        e.vcall(keyframe, SLOT_IS_VALID_QUANTITY, &args![index])
            .bool()
    }
}

/// `other` when it is a multiple keyframe (+0x44), else null.
fn as_multiple(e: &mut Engine, other: u32) -> u32 {
    if other != 0 && e.vcall(other, SLOT_IS_MULTIPLE, &[]).bool() {
        other
    } else {
        0
    }
}

/// Whether `other` has this keyframe's type and weight count.
fn same_shape(e: &mut Engine, this: u32, other: u32) -> bool {
    let first = e.vcall(this, SLOT_GET_TYPE, &[]).u32();
    let second = e.vcall(other, SLOT_GET_TYPE, &[]).u32();
    first == second && count(e, this) == count(e, other)
}

// Typed from 0064fb90
/// `IsValidQuantity` (`thiscall(this, index)`, `RET 4`): the index in range and its weight in
/// `[0, 1]` (a NaN is not).
pub fn is_valid_quantity(e: &mut Engine, this: Ptr, index: u32) -> bool {
    let t = this.addr();
    if index >= e.mem.u32(t.wrapping_add(COUNT)) {
        return false;
    }
    let value = weight_at(e, t, index);
    (0.0..=1.0).contains(&value)
}

// Typed from 0064f5b0
/// `CopyFrom` (`thiscall(this, other)`, `RET 4`): from a multiple keyframe, its type and time, a
/// weight array of its count (`ResetQuantityArray(count, 1)`) and each weight.
pub fn copy_from(e: &mut Engine, this: Ptr, other: u32) {
    let base = this.addr();
    let source = as_multiple(e, other);
    if source == 0 {
        return;
    }
    let kind = e.vcall(source, SLOT_GET_TYPE, &[]).u32();
    e.vcall(base, SLOT_SET_TYPE, &args![kind]);
    let time = to_float(e.vcall(source, SLOT_GET_TIME, &[]).f64());
    e.vcall(base, SLOT_SET_TIME, &args![time]);
    let n = count(e, source);
    e.vcall(base, SLOT_RESET_QUANTITY_ARRAY, &args![n, 1u32]);
    let mut i = 0u32;
    while i < e.mem.u32(base.wrapping_add(COUNT)) {
        let value = get(e, source, i);
        set(e, base, i, value);
        i += 1;
    }
}

// Typed from 0064f690
/// `Compare` (`thiscall(this, other)`, `RET 4`): true when `other` is not a multiple keyframe of
/// the same type and count or any weight differs (a NaN differs); false when all are equal.
pub fn compare(e: &mut Engine, this: Ptr, other: u32) -> bool {
    let base = this.addr();
    let source = as_multiple(e, other);
    if source == 0 || !same_shape(e, source, base) {
        return true;
    }
    let mut i = 0u32;
    while i < count(e, base) {
        let from = get(e, source, i);
        let current = get(e, base, i);
        if from != current {
            return true;
        }
        i += 1;
    }
    false
}

// Typed from 0064f760
/// `GetStrength` (`thiscall(this)`): the largest valid weight, at least 0 (`ST0`).
pub fn get_strength(e: &mut Engine, this: Ptr) -> f64 {
    let base = this.addr();
    let mut strength = 0.0f32;
    let mut i = 0u32;
    while i < e.mem.u32(base.wrapping_add(COUNT)) {
        if valid(e, base, i) {
            let value = weight_at(e, base, i);
            if strength < value {
                strength = value;
            }
        }
        i += 1;
    }
    f64::from(strength)
}

// Typed from 0064f880
/// `IsTargetAvailable` (`thiscall(this)`): whether any weight is in `(0, 1]` (a NaN never is).
pub fn is_target_available(e: &mut Engine, this: Ptr) -> bool {
    let base = this.addr();
    let mut i = 0u32;
    while i < e.mem.u32(base.wrapping_add(COUNT)) {
        let value = weight_at(e, base, i);
        i += 1;
        if value > 0.0 && value <= 1.0 {
            return true;
        }
    }
    false
}

/// A float setting's value when above zero, else 0 (`0064fb10`, `0064fb50`; the setting's address
/// asked again for the value, as there).
fn positive_setting(e: &mut Engine, setting: u32) -> f32 {
    let address = e.call(SETTING_VALUE, &args![setting]).u32();
    if e.mem.f32(address) > 0.0 {
        let address = e.call(SETTING_VALUE, &args![setting]).u32();
        e.mem.f32(address)
    } else {
        0.0
    }
}

/// `(strength * scale + offset) * time` in `R`, stored as a float.
#[inline(always)]
fn step_in<R: Real>(strength: f32, scale: f32, offset: f32, time: f32) -> f32 {
    let r = |v: f32| R::load(v.to_bits());
    f32::from_bits(((r(strength) * r(scale) + r(offset)) * r(time)).store())
}

// Typed from 0064f900
/// `UpdateToTarget` (`thiscall(this, time, target)`, `RET 8`): for a positive time (or a NaN) and a
/// multiple target of the same type and count that differs, each valid weight moves toward the
/// target's by at most `step = (strength * scale + offset) * time`, reaching it when nearer; whether
/// it ran.
#[allow(clippy::neg_cmp_op_on_partial_ord)] // NaN-aware as the exe.
pub fn update_to_target(e: &mut Engine, this: Ptr, time: f32, target: u32) -> bool {
    let base = this.addr();
    let source = as_multiple(e, target);
    if time <= 0.0 {
        return false;
    }
    if source == 0 || !same_shape(e, source, base) {
        return false;
    }
    let differs = if crate::slot_target(e, base, SLOT_COMPARE) == OWN_COMPARE {
        typed_call!(FAMILY, 0x0064_f690, compare(e, this, source))
    } else {
        e.vcall(base, SLOT_COMPARE, &args![source]).bool()
    };
    if !differs {
        return false;
    }
    // `0064fb10` and `0064fb50` (their bodies in place).
    let offset = positive_setting(e, SETTING_OFFSET);
    let scale = positive_setting(e, SETTING_SCALE);
    // A strength is a float loaded (`ST0`), so this narrowing is exact.
    let strength = to_float(
        if crate::slot_target(e, target, SLOT_GET_STRENGTH) == OWN_GET_STRENGTH {
            typed_call!(FAMILY, 0x0064_f760, get_strength(e, Ptr::new(target)))
        } else {
            e.vcall(target, SLOT_GET_STRENGTH, &[]).f64()
        },
    );
    let step = if float_precision() {
        step_in::<f32>(strength, scale, offset, time)
    } else {
        step_in::<f64>(strength, scale, offset, time)
    };
    let mut i = 0u32;
    while i < count(e, base) {
        if !valid(e, base, i) {
            i += 1;
            continue;
        }
        let goal = get(e, source, i);
        let current = get(e, base, i);
        let difference = goal - current;
        // `00408860`: `fabs` of the float (exact). For a NaN difference the exe's `fabs` also sets
        // the C runtime's `errno`, which nothing on this path reads; that is left out.
        if step > difference.abs() {
            let value = get(e, source, i);
            set(e, base, i, value);
        } else if difference < 0.0 {
            let value = get(e, base, i) - step;
            set(e, base, i, value);
        } else if difference > 0.0 {
            let value = get(e, base, i) + step;
            set(e, base, i, value);
        }
        i += 1;
    }
    true
}

/// `from * w + current * (1 - w)` in `R`, stored as a float.
#[inline(always)]
fn blend_in<R: Real>(from: f32, current: f32, w: f32) -> f32 {
    let r = |v: f32| R::load(v.to_bits());
    let kept = r(current) * (R::ONE - r(w));
    f32::from_bits((r(from) * r(w) + kept).store())
}

// Typed from 0064eea0
/// `MergeMorphDataFrom` (`thiscall(this, other, weight, skip_unset, keep_unset)`, `RET 0x10`):
/// blends the weights of `other` (a multiple keyframe of the same type and count, not this one) into
/// this one with `weight` clamped to 0..1: `other * w + this * (1 - w)`, or `other`'s when the weight
/// is at least 1 (or a NaN) or this one is `FLT_MAX`; without `skip_unset`, `other`'s weights within
/// 0.001 of 0 are left alone; an `other` weight of `FLT_MAX` sets `FLT_MAX` only with `keep_unset`.
/// Whether any weight changed.
#[allow(clippy::neg_cmp_op_on_partial_ord)] // NaN-aware as the exe.
pub fn merge_morph_data_from(
    e: &mut Engine,
    this: Ptr,
    other: u32,
    weight: f32,
    skip_unset: u8,
    keep_unset: u8,
) -> bool {
    let base = this.addr();
    let source = as_multiple(e, other);
    let mut changed = 0u8;
    if source == 0 || source == base || !same_shape(e, base, source) {
        return false;
    }
    let clamped = if weight < 0.0 {
        0.0
    } else if !(weight > 1.0) {
        weight
    } else {
        1.0
    };
    let single = float_precision();
    let mut i = 0u32;
    while i < e.mem.u32(base.wrapping_add(COUNT)) {
        if skip_unset == 0 {
            let value = get(e, source, i);
            let tolerance = e.global::<u32>(TOLERANCE);
            if e.call(NEARLY_EQUAL, &args![value, 0.0f32, tolerance])
                .bool()
            {
                i += 1;
                continue;
            }
        }
        let from = get(e, source, i);
        if from < f32::MAX {
            let copy = !(weight < 1.0) || !(get(e, base, i) < f32::MAX);
            let value = if copy {
                get(e, source, i)
            } else {
                let current = get(e, base, i);
                let from = get(e, source, i);
                if single {
                    blend_in::<f32>(from, current, clamped)
                } else {
                    blend_in::<f64>(from, current, clamped)
                }
            };
            changed |= set(e, base, i, value);
        } else if keep_unset != 0 {
            changed |= set(e, base, i, f32::MAX);
        }
        i += 1;
    }
    changed != 0
}
