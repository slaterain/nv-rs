//! The bound of a run of points (`NiBound::ComputeFromData`): what the geometry data's setters and the
//! renderer's release of packed arrays compute over the positions, and what other callers (the
//! interface's tiles and text, the morph bound, the skinned bound) compute over points of their own.

use super::layout::Bound;
use engine_core::ptr::Ptr;
use engine_core::x87::{
    x87_add, x87_load32, x87_load64, x87_m32, x87_mul, x87_quiet_bits, x87_store32, x87_sub,
};
use engine_core::{args, Engine};

/// `_CIsqrt` (`00ec6040`): the value in ST0, its root in ST0 (a leading `f64` word pair).
const SQUARE_ROOT: u32 = 0x00ec_6040;
/// The three floats an empty bound's centre is copied from (`011f426c`).
const ZERO_POINT: u32 = 0x011f_426c;
/// The `double` 0.5 (`01011588`), loaded with `FLD qword`.
const HALF: u32 = 0x0101_1588;

/// `FLD a; FLD b; FLD c` and the exe's shared sequence after them: `(a*a + b*b) + c*c`, each step
/// with the x87's rounding and choice of NaN.
fn sum_of_squares(a: f64, b: f64, c: f64) -> f64 {
    let bb = x87_mul(b, b);
    let aa = x87_mul(a, a);
    let first_two = x87_add(aa, bb);
    x87_add(first_two, x87_mul(c, c))
}

// Typed from 00a7ee30
/// `NiBound::ComputeFromData` (Xbox PDB; `thiscall` on the bound, `RET 8`: the point count and the
/// address of that many packed three-float points). No points (a count of 0 or below): the centre is
/// the point at `011f426c` (copied as words) and the radius 0. Otherwise the centre is the middle of
/// the points' box: each axis's minimum and maximum from the first point on (a later coordinate
/// replaces the minimum when it is below it and the maximum when above, ordered compares), `max +
/// min` stored as a float and multiplied by 0.5; the radius is the square root (`00ec6040`) of the
/// largest squared distance from the centre to a point (each difference and the sum stored as floats,
/// compared with the largest so far, which starts at 0). The exe walks the points four at a time and
/// then the rest; each point takes the same steps in the same order.
pub fn compute_from_data(e: &mut Engine, this: Ptr, count: i32, points: Ptr) {
    let bound = this.cast::<Bound>();
    let base = bound.addr();
    if count <= 0 {
        for axis in 0..3 {
            let word = e.mem.u32(ZERO_POINT + 4 * axis);
            e.mem.set_u32(base + 4 * axis, word);
        }
        e.set(bound, Bound::m_fRadius, x87_store32(0.0));
        return;
    }
    let count = count.unsigned_abs();
    let at = points.addr();
    let coordinate = |e: &Engine, point: u32, axis: u32| {
        e.mem
            .u32(at.wrapping_add(12u32.wrapping_mul(point)) + 4 * axis)
    };
    let mut low = [0u32, 1, 2].map(|axis| coordinate(e, 0, axis));
    let mut high = low;
    for point in 1..count {
        for (axis, least) in (0u32..).zip(low.iter_mut()) {
            let value = coordinate(e, point, axis);
            // FLD min; FLD p; FCOMPP; TEST AH,5; JP: replaced when p < min.
            if x87_load32(value) < x87_load32(*least) {
                *least = x87_quiet_bits(value);
            }
        }
        for (axis, most) in (0u32..).zip(high.iter_mut()) {
            let value = coordinate(e, point, axis);
            // FLD max; FLD p; FCOMPP; TEST AH,0x41; JNZ: replaced when p > max.
            if x87_load32(value) > x87_load32(*most) {
                *most = x87_quiet_bits(value);
            }
        }
    }
    let sums =
        [0usize, 1, 2].map(|axis| x87_store32(x87_add(x87_load32(high[axis]), x87_m32(low[axis]))));
    let half = x87_load64(e.mem.u64(HALF));
    // FLD sx; FLD half; FMUL ST1,ST0; FXCH; FSTP: x = sx * half.
    let x = x87_store32(x87_mul(x87_load32(sums[0]), half));
    e.mem.set_u32(base, x);
    // FLD sy; FMUL ST0,ST1; FSTP: y = sy * half.
    let y = x87_store32(x87_mul(x87_load32(sums[1]), half));
    e.mem.set_u32(base + 4, y);
    // FMUL dword [sz]; FSTP: z = half * sz.
    let z = x87_store32(x87_mul(half, x87_m32(sums[2])));
    e.mem.set_u32(base + 8, z);
    let mut best = x87_store32(0.0);
    for point in 0..count {
        let difference = [0u32, 1, 2].map(|axis| {
            let centre = e.mem.u32(base + 4 * axis);
            x87_store32(x87_sub(
                x87_load32(coordinate(e, point, axis)),
                x87_m32(centre),
            ))
        });
        // FLD dx; FLD dy; FLD dz: (dx*dx + dy*dy) + dz*dz.
        let squared = x87_store32(sum_of_squares(
            x87_load32(difference[0]),
            x87_load32(difference[1]),
            x87_load32(difference[2]),
        ));
        // FLD d2; FLD best; FCOMP; TEST AH,5; JP: kept when best < d2.
        if x87_load32(best) < x87_load32(squared) {
            best = x87_store32(x87_load32(squared));
        }
    }
    let root = e.call(SQUARE_ROOT, &args![x87_load32(best)]).f64();
    let radius = x87_store32(x87_load32(x87_store32(root)));
    e.set(bound, Bound::m_fRadius, radius);
}
