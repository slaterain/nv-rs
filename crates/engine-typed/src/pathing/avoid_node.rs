//! The avoid nodes (Xbox PDB names): the extra cost a point, or a segment with a radius, adds to
//! a point, and the sum of them over an array.
//!
//! Native float maths (ADR-0014's float rule, [`super`]'s floats): the radius squared and the
//! array's running sum are each one operation on two floats, stored, so they are plain `f32`;
//! the segment's squared length, projection and difference (a sum of products and a quotient,
//! compared before it is stored) run in [`Real`] at the thread's precision. The squared lengths
//! are computed in place rather than called: the exe reads the length's `ST0` where it is not
//! stored (the projection's total), and a call would give the stored `float`.

use super::layout::PathingAvoidNode;
use super::point;
use super::{double_global, float, float_global, get, real, FAMILY};
use crate::culling::layout::NiPoint3;
use crate::native::{float_precision, Real};
use engine_core::ptr::Ptr;
use engine_core::typed::leaf;
use engine_core::{args, typed_call, Engine};

/// `FLT_MAX` (`float`), at `0106ccd0`.
const FLOAT_MAX: u32 = 0x0106_ccd0;
/// `0.0` (`double`), at `01012060`.
const ZERO: u32 = 0x0101_2060;
/// `006a1400`: the array's element `index` (`thiscall`, one word).
const ARRAY_AT: u32 = 0x006a_1400;

/// The node's cost at `point` in `R` ([`cost_for_point`]).
fn cost_in<R: Real>(e: &mut Engine, this: Ptr, point: Ptr) -> f32 {
    let node = this.cast::<PathingAvoidNode>();
    let start = node.at(PathingAvoidNode::start);
    let end = node.at(PathingAvoidNode::end);
    let point = point.cast::<NiPoint3>();
    // `radius * radius`, one operation on two floats, stored to a float: plain `f32`.
    let radius = e.get(node, PathingAvoidNode::radius);
    let radius_squared = radius * radius;
    let mut distance = float_global(e, FLOAT_MAX);
    let kind = e.get(node, PathingAvoidNode::kind);
    e.with_stack(0x40, |e, frame| {
        let frame = frame.addr();
        let offset = Ptr::<NiPoint3>::new(frame + 0x2c);
        let segment = Ptr::<NiPoint3>::new(frame + 0xc);
        let from_start = Ptr::<NiPoint3>::new(frame + 0x18);
        match kind {
            0 | 2 => {
                typed_call!(
                    FAMILY,
                    0x0043_9ef0,
                    point::subtract(e, start, offset, point)
                );
                distance = float(point::squares_xyz::<R>(e, offset));
            }
            1 => {
                typed_call!(FAMILY, 0x0043_9ef0, point::subtract(e, end, segment, start));
                typed_call!(
                    FAMILY,
                    0x0043_9ef0,
                    point::subtract(e, point, from_start, start)
                );
                let segment_squared = float(point::squares_xyz::<R>(e, segment));
                let along = typed_call!(
                    leaf::FAMILY,
                    0x004b_6190,
                    leaf::point3_dot(e, from_start, segment)
                );
                // `TEST AH,5; JNP`, then `JP`: below zero, or past the end, is `FLT_MAX`
                // (a NaN is neither).
                let before = f64::from(along) < double_global(e, ZERO);
                let past = real::<R>(segment_squared) < real::<R>(along);
                if before || past {
                    distance = float_global(e, FLOAT_MAX);
                } else {
                    let total = point::squares_xyz::<R>(e, from_start);
                    let along = real::<R>(along);
                    let projected = (along * along) / real::<R>(segment_squared);
                    distance = float(total - projected);
                }
            }
            _ => {}
        }
    });
    // `TEST AH,0x41; JNZ`: the cost only when `r2 > d` (ordered, so a NaN is not).
    let (r2, d) = (real::<R>(radius_squared), real::<R>(distance));
    if r2 > d {
        let cost = get::<R, _>(e, node, PathingAvoidNode::cost);
        float((R::ONE - d / r2) * cost)
    } else {
        0.0
    }
}

// Typed from 006dc780
/// No map name: the avoid node's cost at `point`: with the radius (`+0x18`) squared as `r2` and the
/// node's squared distance `d` to the point (type `+0x20` 0 or 2: from the node's position at `+0`;
/// type 1: from the segment `+0` to `+0xc`, `FLT_MAX` when the point projects before its start or
/// past its end; other types: `FLT_MAX`), the cost is `(1 - d / r2) * cost` (`+0x1c`) when
/// `d < r2`, else 0.
pub fn cost_for_point(e: &mut Engine, this: Ptr, point: Ptr) -> f32 {
    if float_precision() {
        cost_in::<f32>(e, this, point)
    } else {
        cost_in::<f64>(e, this, point)
    }
}

// Typed from 006dc990
/// `PathingAvoidNodeArray::GetCostForPoint` (engine map): the sum of every node's cost at `point`
/// ([`cost_for_point`]), each sum stored to a `float`: plain `f32`.
pub fn array_cost_for_point(e: &mut Engine, this: Ptr, point: Ptr) -> f32 {
    let mut sum = 0.0f32;
    let count = typed_call!(leaf::FAMILY, 0x0044_ddc0, leaf::word_at_8(e, this));
    let mut index = 0u32;
    while index < count {
        let node = e.call(ARRAY_AT, &args![this, index]).ptr();
        let cost = typed_call!(FAMILY, 0x006d_c780, cost_for_point(e, node, point));
        sum += cost;
        index = index.wrapping_add(1);
    }
    sum
}
