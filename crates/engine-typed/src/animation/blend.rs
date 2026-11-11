//! `NiBlendTransformInterpolator`: a bone's transform from one interpolator, or several blended by
//! their weights. Its fields as the code reads them (`NiBlendInterpolator`, Xbox PDB): the last time
//! (+8), the flags (+0xc, bit 0: manager controlled), the interpolator count (+0xe), the single
//! interpolator (+0x18) and the single time (+0x20).

use super::FAMILY;
use engine_core::ptr::Ptr;
use engine_core::{args, typed_call, Engine};

/// The fields.
const LAST_TIME: u32 = 0x8;
const FLAGS: u32 = 0xc;
const INTERP_COUNT: u32 = 0xe;
const SINGLE_INTERPOLATOR: u32 = 0x18;
const SINGLE_TIME: u32 = 0x20;
/// `NiQuatTransform` (Xbox PDB): the rotation (+0xc, four floats) and the scale (+0x1c).
const ROTATE: u32 = 0xc;
const SCALE: u32 = 0x1c;
/// `NiInterpolator::INVALID_FLOAT` (`0109696c`), the float written into invalid parts
/// (`01015f5c`) and the `double` `-FLT_MAX` (`010241b0`) the rotation is compared with.
const INVALID_FLOAT: u32 = 0x0109_696c;
const INVALID_TRANSFORM_PART: u32 = 0x0101_5f5c;
const LOWEST: u32 = 0x0102_41b0;
/// `NiQuaternion::Normalize` (`00a38560`), `NiBlendInterpolator::ComputeNormalizedWeights`
/// (`00a37260`) and `NiBlendTransformInterpolator::BlendValues` (`00a40c10`, `thiscall(this, time,
/// object, transform)`, a byte), all by address.
const QUATERNION_NORMALIZE: u32 = 0x00a3_8560;
const COMPUTE_NORMALIZED_WEIGHTS: u32 = 0x00a3_7260;
const BLEND_VALUES: u32 = 0x00a4_0c10;
/// An interpolator's transform `Update` (slot +0x8c, `(time, object, transform)`, a byte).
const SLOT_TRANSFORM_UPDATE: u32 = 0x8c;

// Typed from 00a41110
/// `NiBlendTransformInterpolator::Update` (`thiscall(this, time, object, transform)`): with one
/// interpolator [`store_single_value`], with more `ComputeNormalizedWeights` then `BlendValues` (by
/// address), with none nothing (false); the last time becomes `time`. Answers what the store or
/// blend answered.
pub fn update_transform(e: &mut Engine, this: Ptr, time: f32, object: Ptr, transform: Ptr) -> bool {
    let time_bits = time.to_bits();
    let count = e.mem.u8(this.addr().wrapping_add(INTERP_COUNT));
    let answer = if count == 1 {
        typed_call!(
            FAMILY,
            0x00a4_0b30,
            store_single_value(e, this, f32::from_bits(time_bits), object, transform)
        )
    } else if count > 0 {
        e.call(COMPUTE_NORMALIZED_WEIGHTS, &args![this]);
        e.call(BLEND_VALUES, &args![this, time_bits, object, transform])
            .u8()
            != 0
    } else {
        false
    };
    e.mem
        .set_u32(this.addr().wrapping_add(LAST_TIME), time_bits);
    answer
}

/// The invalid marker in a transform's translation, rotation (its float at +0x10) and scale.
fn invalidate(e: &mut Engine, transform: u32) {
    let invalid = e.mem.u32(INVALID_TRANSFORM_PART);
    e.mem.set_u32(transform, invalid);
    e.mem.set_u32(transform.wrapping_add(0x10), invalid);
    e.mem.set_u32(transform.wrapping_add(SCALE), invalid);
}

// Typed from 00a40b30
/// `NiBlendTransformInterpolator::StoreSingleValue` (`thiscall(this, time, object, transform)`): the
/// single interpolator's transform at `time` (the single time when manager controlled). At the
/// invalid time, or when its `Update` (slot +0x8c) gives nothing, the transform is marked invalid and
/// the answer is false; otherwise its rotation is normalized (`NiQuaternion::Normalize` on a copy,
/// unless the rotation's float at +0x10 is the invalid marker) and the answer is true.
pub fn store_single_value(
    e: &mut Engine,
    this: Ptr,
    time: f32,
    object: Ptr,
    transform: Ptr,
) -> bool {
    let mut time_bits = time.to_bits();
    if e.mem.u8(this.addr().wrapping_add(FLAGS)) & 1 != 0 {
        time_bits = e.mem.u32(this.addr().wrapping_add(SINGLE_TIME));
    }
    if f32::from_bits(e.mem.u32(INVALID_FLOAT)) == f32::from_bits(time_bits) {
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
    let lowest = f64::from_bits(e.global::<u64>(LOWEST));
    let rotation = transform.addr().wrapping_add(ROTATE);
    if f64::from(f32::from_bits(e.mem.u32(rotation.wrapping_add(4)))) != lowest {
        e.with_stack(16, |e, copy| {
            for word in 0..4u32 {
                let value = e.mem.u32(rotation.wrapping_add(4 * word));
                e.mem.set_u32(copy.addr().wrapping_add(4 * word), value);
            }
            e.call(QUATERNION_NORMALIZE, &args![copy]);
            for word in 0..4u32 {
                let value = e.mem.u32(copy.addr().wrapping_add(4 * word));
                e.mem.set_u32(rotation.wrapping_add(4 * word), value);
            }
        });
    }
    true
}
