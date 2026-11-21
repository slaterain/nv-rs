//! `BSFaceGenKeyframeMultiple` (0x14 bytes: vtable, `eType` +4, `fTime` +8, `pQuantities` +0xc,
//! `iNumQuantities` +0x10): a face's array of morph weights, which `BSFaceGenAnimationData::Update`
//! moves toward its targets, copies and merges for every head every frame. The translations make a
//! call by address for every weight they read or write (the folded getter `005dd4e0`, the setter
//! `0064a4d0`, the count `0044edb0`) and a virtual call for every validity test; here the weights
//! are read and written in place, and the class's own virtual functions are called directly when the
//! slot holds them.

use super::FAMILY;
use engine_core::ptr::Ptr;
use engine_core::x87::{x87_add, x87_load32, x87_m32, x87_mul, x87_quiet_bits, x87_store32, x87_sub};
use engine_core::{args, typed_call, Engine};

/// The fields.
const TIME: u32 = 0x8;
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
/// `0.0`, `1.0` and `FLT_MAX` (doubles), `FLT_MAX` (float), the merge's tolerance 0.001 (float).
const ZERO: u32 = 0x0101_2060;
const ONE: u32 = 0x0101_2070;
const MAXIMUM: u32 = 0x0102_31b0;
const MAXIMUM_FLOAT: u32 = 0x0101_6970;
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
    if index < e.mem.u32(t + COUNT) {
        let data = e.mem.u32(t + QUANTITIES);
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
    if index >= e.mem.u32(t + COUNT) {
        return false;
    }
    let slot = e.mem.u32(t + QUANTITIES).wrapping_add(index.wrapping_mul(4));
    let old = x87_load32(e.mem.u32(slot));
    if x87_load32(value.to_bits()) == old {
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

fn get(e: &mut Engine, keyframe: u32, index: u32) -> f64 {
    f64::from(typed_call!(
        FAMILY,
        0x005d_d4e0,
        quantity(e, Ptr::new(keyframe), index)
    ))
}

fn set(e: &mut Engine, keyframe: u32, index: u32, value: u32) -> u8 {
    u8::from(typed_call!(
        FAMILY,
        0x0064_a4d0,
        set_quantity(e, Ptr::new(keyframe), index, f32::from_bits(value))
    ))
}

fn count(e: &mut Engine, keyframe: u32) -> u32 {
    typed_call!(FAMILY, 0x0044_edb0, word_at_10(e, Ptr::new(keyframe)))
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
    if index >= e.mem.u32(t + COUNT) {
        return false;
    }
    let zero: f64 = e.global(ZERO);
    let one: f64 = e.global(ONE);
    let array = e.mem.u32(t + QUANTITIES);
    let value = x87_load32(e.mem.u32(array.wrapping_add(index.wrapping_mul(4))));
    if value < zero || value.is_nan() {
        return false;
    }
    value <= one
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
    let time = e.vcall(source, SLOT_GET_TIME, &[]).f64();
    e.vcall(base, SLOT_SET_TIME, &args![x87_store32(time)]);
    let n = count(e, source);
    e.vcall(base, SLOT_RESET_QUANTITY_ARRAY, &args![n, 1u32]);
    let mut i = 0u32;
    while i < e.mem.u32(base + COUNT) {
        let value = x87_store32(get(e, source, i));
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
        if !(from == current) {
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
    let mut strength = x87_store32(0.0);
    let mut i = 0u32;
    while i < e.mem.u32(base + COUNT) {
        if valid(e, base, i) {
            let array = e.mem.u32(base + QUANTITIES);
            let value = e.mem.u32(array.wrapping_add(i.wrapping_mul(4)));
            if x87_load32(strength) < x87_load32(value) {
                strength = x87_quiet_bits(value);
            }
        }
        i += 1;
    }
    x87_load32(strength)
}

// Typed from 0064f880
/// `IsTargetAvailable` (`thiscall(this)`): whether any weight is in `(0, 1]` (a NaN never is).
pub fn is_target_available(e: &mut Engine, this: Ptr) -> bool {
    let base = this.addr();
    let zero: f64 = e.global(ZERO);
    let one: f64 = e.global(ONE);
    let mut i = 0u32;
    while i < e.mem.u32(base + COUNT) {
        let array = e.mem.u32(base + QUANTITIES);
        let value = x87_load32(e.mem.u32(array.wrapping_add(i.wrapping_mul(4))));
        i += 1;
        if value < zero || value > one || !(value > zero) {
            continue;
        }
        return true;
    }
    false
}

/// A float setting's value when above zero, else 0 (`0064fb10`, `0064fb50`), in `ST0`.
fn positive_setting(e: &mut Engine, setting: u32) -> f64 {
    let address = e.call(SETTING_VALUE, &args![setting]).u32();
    let zero: f64 = e.global(ZERO);
    let value = if x87_load32(e.mem.u32(address)) > zero {
        let address = e.call(SETTING_VALUE, &args![setting]).u32();
        x87_quiet_bits(e.mem.u32(address))
    } else {
        x87_store32(0.0)
    };
    x87_load32(value)
}

// Typed from 0064f900
/// `UpdateToTarget` (`thiscall(this, time, target)`, `RET 8`): for a positive time (or a NaN) and a
/// multiple target of the same type and count that differs, each valid weight moves toward the
/// target's by at most `step = (strength * scale + offset) * time`, reaching it when nearer; whether
/// it ran.
pub fn update_to_target(e: &mut Engine, this: Ptr, time: f32, target: u32) -> bool {
    let base = this.addr();
    let source = as_multiple(e, target);
    let zero: f64 = e.global(ZERO);
    if x87_load32(time.to_bits()) <= zero {
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
    let strength = if crate::slot_target(e, target, SLOT_GET_STRENGTH) == OWN_GET_STRENGTH {
        typed_call!(FAMILY, 0x0064_f760, get_strength(e, Ptr::new(target)))
    } else {
        e.vcall(target, SLOT_GET_STRENGTH, &[]).f64()
    };
    let step = x87_store32(x87_mul(
        x87_add(x87_mul(strength, scale), offset),
        x87_m32(time.to_bits()),
    ));
    let mut i = 0u32;
    while i < count(e, base) {
        if !valid(e, base, i) {
            i += 1;
            continue;
        }
        let goal = get(e, source, i);
        let current = get(e, base, i);
        let difference = x87_store32(x87_sub(goal, current));
        // `00408860`: `fabs` of the float as a double, stored as a float (exact).
        let distance = f64::from(f32::from_bits(difference).abs());
        let d = x87_load32(difference);
        if x87_load32(step) > distance {
            let value = x87_store32(get(e, source, i));
            set(e, base, i, value);
        } else if d < zero {
            let value = x87_store32(x87_sub(get(e, base, i), x87_m32(step)));
            set(e, base, i, x87_quiet_bits(value));
        } else if d > zero {
            let value = x87_store32(x87_add(get(e, base, i), x87_m32(step)));
            set(e, base, i, x87_quiet_bits(value));
        }
        i += 1;
    }
    true
}

// Typed from 0064eea0
/// `MergeMorphDataFrom` (`thiscall(this, other, weight, skip_unset, keep_unset)`, `RET 0x10`):
/// blends the weights of `other` (a multiple keyframe of the same type and count, not this one) into
/// this one with `weight` clamped to 0..1: `other * w + this * (1 - w)`, or `other`'s when the weight
/// is at least 1 or this one is `FLT_MAX`; without `skip_unset`, `other`'s weights within 0.001 of 0
/// are left alone; an `other` weight of `FLT_MAX` sets `FLT_MAX` only with `keep_unset`. Whether any
/// weight changed.
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
    let zero: f64 = e.global(ZERO);
    let one: f64 = e.global(ONE);
    let w = x87_load32(weight.to_bits());
    let clamped = if w < zero {
        x87_store32(0.0)
    } else if !(w > one) {
        x87_quiet_bits(weight.to_bits())
    } else {
        x87_store32(1.0)
    };
    let maximum: f64 = e.global(MAXIMUM);
    let mut i = 0u32;
    while i < e.mem.u32(base + COUNT) {
        if skip_unset == 0 {
            let value = x87_store32(get(e, source, i));
            let tolerance = x87_quiet_bits(e.global::<u32>(TOLERANCE));
            if e.call(NEARLY_EQUAL, &args![value, x87_store32(0.0), tolerance])
                .bool()
            {
                i += 1;
                continue;
            }
        }
        let from = get(e, source, i);
        if from < maximum {
            let mut copy = !(w < one);
            if !copy {
                let current = get(e, base, i);
                copy = !(current < maximum);
            }
            let value = if copy {
                x87_store32(get(e, source, i))
            } else {
                let current = get(e, base, i);
                let rest = x87_sub(1.0, x87_load32(clamped));
                let kept = x87_mul(current, rest);
                let from = get(e, source, i);
                let blended = x87_add(x87_mul(from, x87_m32(clamped)), kept);
                x87_quiet_bits(x87_store32(blended))
            };
            changed |= set(e, base, i, value);
        } else if keep_unset != 0 {
            let value = x87_quiet_bits(e.global::<u32>(MAXIMUM_FLOAT));
            changed |= set(e, base, i, value);
        }
        i += 1;
    }
    changed != 0
}
