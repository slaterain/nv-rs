//! Bound and plane tests: which side of a plane a bound is on, a point's distance from a plane, and
//! the merge of two bounds.

use super::layout::{NiBound, NiPlane, NiPoint3};
use super::{copy_float, FAMILY};
use crate::anim_keys::narrow;
use engine_core::ptr::Ptr;
use engine_core::typed::leaf;
use engine_core::{args, typed_call, Engine};
use std::cmp::Ordering;

/// `_CIsqrt` (`00ec6040`): the value in `ST0`, its root in `ST0`; called with the value as a leading
/// `f64` word pair.
const SQUARE_ROOT: u32 = 0x00ec_6040;
/// The float `Merge` compares the distance of the two centres with (`011ab3c0`, 1e-6).
const MERGE_THRESHOLD: u32 = 0x011a_b3c0;
/// The `double` 0.5 at `01011588`, which the radius is multiplied by.
const HALF: u32 = 0x0101_1588;

// Typed from 004b6160
/// `NiPlane::Distance` (`thiscall(plane, point)`): the dot product of the normal with the point
/// (`004b6190`, a float), minus the constant, stored to a float.
pub fn plane_distance(e: &mut Engine, this: Ptr<NiPlane>, point: Ptr<NiPoint3>) -> f32 {
    let dot = typed_call!(leaf::FAMILY, 0x004b_6190, leaf::point3_dot(e, this, point));
    dot - e.get(this, NiPlane::m_fConstant)
}

// Typed from 004b6100
/// `NiBound::WhichSide` (Xbox PDB, `thiscall(bound, plane)`): the centre's distance from the plane
/// (`004b6160`) against the radius: 2 when the distance is at most minus the radius, else 1 when it
/// is at least the radius, else 0, also when either is a NaN (each `FCOMPP` with `TEST AH,0x41; JP` is
/// taken on greater and on unordered). The radius is read for each test, as the exe loads it twice.
pub fn which_side(e: &mut Engine, this: Ptr<NiBound>, plane: Ptr<NiPlane>) -> u32 {
    let distance = typed_call!(
        FAMILY,
        0x004b_6160,
        plane_distance(e, plane, this.at(NiBound::m_kCenter))
    );
    if distance <= -e.get(this, NiBound::m_fRadius) {
        return 2;
    }
    if e.get(this, NiBound::m_fRadius) <= distance {
        1
    } else {
        0
    }
}

// Typed from 00a7f3f0
/// `NiBound::Merge` (Xbox PDB, `thiscall(this, other)`): makes `this` the bound of both. With
/// `d = this.center - other.center` (each axis stored to a float), `dist2 = (dx*dx + dy*dy) + dz*dz`
/// and `dr = other.radius - this.radius` (both stored), `dr2 = dr*dr` (stored):
///
/// - `0 <= dr` and `dist2 <= dr2`: `other` holds `this`; its centre and radius are copied;
/// - `dr < 0` or a NaN, and `dist2 <= dr2` or unordered: `this` holds `other`, nothing changes;
/// - otherwise `dist = sqrt(dist2)` (`00ec6040`, stored to a float); when 1e-6 is below it the centre
///   moves to `other.center + t * d`, `t = (dist - dr) / (dist + dist)`; the radius becomes
///   `(dist + other.radius + this.radius) * 0.5`.
///
/// Native maths (the family's "Floats"): a single operation on two floats stored at once in `f32`;
/// `dist2`, `t` and the radius, which the exe keeps on the stack across several operations, in `f64`
/// rounded where it stores them.
pub fn merge(e: &mut Engine, this: Ptr<NiBound>, other: Ptr<NiBound>) {
    let (center, other_center) = (this.at(NiBound::m_kCenter), other.at(NiBound::m_kCenter));
    let dx = e.get(center, NiPoint3::x) - e.get(other_center, NiPoint3::x);
    let dy = e.get(center, NiPoint3::y) - e.get(other_center, NiPoint3::y);
    let dz = e.get(center, NiPoint3::z) - e.get(other_center, NiPoint3::z);
    let (x, y, z) = (f64::from(dx), f64::from(dy), f64::from(dz));
    let dist2 = narrow((x * x + y * y) + z * z);
    let dr = e.get(other, NiBound::m_fRadius) - e.get(this, NiBound::m_fRadius);
    let dr2 = dr * dr;
    // FLDZ; FCOMPP (0 against dr); TEST AH,0x41; JP: taken when dr < 0 or unordered.
    if 0.0 <= dr {
        // FCOM (dist2 against dr2); TEST AH,0x41; JP: taken when greater or unordered.
        if dist2 <= dr2 {
            copy_float(e, other_center, NiPoint3::x, center, NiPoint3::x);
            copy_float(e, other_center, NiPoint3::y, center, NiPoint3::y);
            copy_float(e, other_center, NiPoint3::z, center, NiPoint3::z);
            copy_float(e, other, NiBound::m_fRadius, this, NiBound::m_fRadius);
            return;
        }
    } else if dist2.partial_cmp(&dr2) != Some(Ordering::Greater) {
        // TEST AH,0x41; JNZ: less, equal or unordered.
        return;
    }
    let root = e.call(SQUARE_ROOT, &args![f64::from(dist2)]).f64();
    let dist = narrow(root);
    // FLD threshold; FCOMP (against dist); TEST AH,5; JP: the centre moves when threshold < dist.
    if e.global::<f32>(MERGE_THRESHOLD) < dist {
        let d = f64::from(dist);
        let t = narrow((d - f64::from(dr)) / (d + d));
        let (sx, sy, sz) = (dx * t, dy * t, t * dz);
        let nx = e.get(other_center, NiPoint3::x) + sx;
        let ny = sy + e.get(other_center, NiPoint3::y);
        let nz = e.get(other_center, NiPoint3::z) + sz;
        e.set(center, NiPoint3::x, nx);
        e.set(center, NiPoint3::y, ny);
        e.set(center, NiPoint3::z, nz);
    }
    let sum = (f64::from(dist) + f64::from(e.get(other, NiBound::m_fRadius)))
        + f64::from(e.get(this, NiBound::m_fRadius));
    e.set(
        this,
        NiBound::m_fRadius,
        narrow(sum * e.global::<f64>(HALF)),
    );
}
