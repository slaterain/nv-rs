//! The cost a point or segment avoidance node adds to a pathfinding query.

use super::layout::PathingAvoidNode;
use super::point::{length_squared, subtract};
use super::{double_global, float_global, FAMILY};
use crate::culling::layout::NiPoint3;
use crate::native::{float_precision, round_to_24_bits, to_float};
use engine_core::ptr::Ptr;
use engine_core::typed::leaf;
use engine_core::{args, typed_call, Engine};

/// `FLT_MAX` (`float`), at `0106ccd0`.
const FLOAT_MAX: u32 = 0x0106_ccd0;
/// `0.0` (`double`), at `01012060`.
const ZERO: u32 = 0x0101_2060;
/// `006a1400`: the array's element `index` (`thiscall`, one word).
const ARRAY_AT: u32 = 0x006a_1400;

// Typed from 006dc780
/// No map name: the avoid node's cost at `point`: with the radius (`+0x18`) squared as `r2` and the
/// node's squared distance `d` to the point (type `+0x20` 0 or 2: from the node's position at `+0`;
/// type 1: from the segment `+0` to `+0xc`, `FLT_MAX` when the point projects before its start or
/// past its end; other types: `FLT_MAX`), the cost is `(1 - d / r2) * cost` (`+0x1c`) when
/// `d < r2`, else 0.
pub fn cost_for_point(e: &mut Engine, this: Ptr, point: Ptr) -> f32 {
    let node = this.cast();
    let point = point.cast();
    if float_precision() {
        cost_at(e, node, point, round_to_24_bits)
    } else {
        cost_at(e, node, point, std::convert::identity)
    }
}

// The squared projection can exceed f32 range before division brings it back.
// Keep the register exponent through the chain, rounding each step to the thread precision.
fn cost_at(
    e: &mut Engine,
    node: Ptr<PathingAvoidNode>,
    point: Ptr<NiPoint3>,
    step: impl Fn(f64) -> f64,
) -> f32 {
    let radius = f64::from(e.get(node, PathingAvoidNode::radius));
    let radius_squared = to_float(step(radius * radius));
    let max_distance = float_global(e, FLOAT_MAX);
    let start = node.at(PathingAvoidNode::start);
    let distance = e.with_stack(0x40, |e, frame| match e.get(node, PathingAvoidNode::kind) {
        0 | 2 => {
            let offset = frame.byte_add(0x2c).cast();
            typed_call!(FAMILY, 0x0043_9ef0, subtract(e, start, offset, point));
            typed_call!(FAMILY, 0x004a_7290, length_squared(e, offset))
        }
        1 => {
            let segment = frame.byte_add(0xc).cast();
            let offset = frame.byte_add(0x18).cast();
            let end = node.at(PathingAvoidNode::end);
            typed_call!(FAMILY, 0x0043_9ef0, subtract(e, end, segment, start));
            typed_call!(FAMILY, 0x0043_9ef0, subtract(e, point, offset, start));
            let squared = typed_call!(FAMILY, 0x004a_7290, length_squared(e, segment));
            let along = typed_call!(
                leaf::FAMILY,
                0x004b_6190,
                leaf::point3_dot(e, offset, segment)
            );
            if f64::from(along) < double_global(e, ZERO) || squared < along {
                return float_global(e, FLOAT_MAX);
            }
            let total = typed_call!(FAMILY, 0x004a_7290, length_squared(e, offset));
            let along = f64::from(along);
            let projected = step(step(along * along) / f64::from(squared));
            to_float(step(f64::from(total) - projected))
        }
        _ => max_distance,
    });
    if radius_squared > distance {
        let ratio = step(f64::from(distance) / f64::from(radius_squared));
        to_float(step(
            step(1.0 - ratio) * f64::from(e.get(node, PathingAvoidNode::cost)),
        ))
    } else {
        0.0
    }
}

// Typed from 006dc990
/// `PathingAvoidNodeArray::GetCostForPoint` (engine map): the sum of every node's cost at `point`
/// ([`cost_for_point`]), each sum stored to a `float`: plain `f32`.
#[allow(clippy::assign_op_pattern)] // FADD loads the new cost first, which matters for NaN payloads.
pub fn array_cost_for_point(e: &mut Engine, array: Ptr, point: Ptr) -> f32 {
    let count = typed_call!(leaf::FAMILY, 0x0044_ddc0, leaf::word_at_8(e, array));
    let mut sum = 0.0;
    for index in 0..count {
        let node = e.call(ARRAY_AT, &args![array, index]).ptr();
        let cost = typed_call!(FAMILY, 0x006d_c780, cost_for_point(e, node, point));
        sum = cost + sum;
    }
    sum
}
