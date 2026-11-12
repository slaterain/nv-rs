//! The six culling planes of a camera's frustum (`NiFrustumPlanes::Set`): the planes the walk tests
//! every bound against, made each time a culling process or compound frustum takes a camera.

use super::layout::{NiFrustum, NiFrustumPlanes, NiMatrix3, NiPlane, NiPoint3, NiTransform};
use crate::anim_keys::narrow;
use engine_core::ptr::{Field, Ptr};
use engine_core::{args, Engine};

/// `_CIsqrt` (`00ec6040`): the value in `ST0`, its root in `ST0`.
const SQUARE_ROOT: u32 = 0x00ec_6040;
/// The `double` 1.0 at `01012070`, which `FADD` adds.
const ONE_DOUBLE: u32 = 0x0101_2070;

/// Three floats the exe keeps in stack slots.
type Floats = [f32; 3];

/// A column of the rotation (each entry through a float slot in the exe, copied as it is here).
#[inline(always)]
fn column(e: &Engine, matrix: Ptr<NiMatrix3>, index: u32) -> Floats {
    [0, 1, 2].map(|row| e.get(matrix, NiMatrix3::entry(row, index)))
}

/// `column * factor`, each product stored.
#[inline(always)]
fn scaled(column: Floats, factor: f32) -> Floats {
    column.map(|c| c * factor)
}

/// `first + second` per component, stored.
#[inline(always)]
fn sum(first: Floats, second: Floats) -> Floats {
    [0, 1, 2].map(|i| first[i] + second[i])
}

/// The column negated (`FCHS` of the loaded value, stored).
#[inline(always)]
fn negated(column: Floats) -> Floats {
    column.map(|c| -c)
}

/// `1 / sqrt(x * x + 1)`, each step stored to a float: `x * x`, plus the `double` 1.0 (in `f64`,
/// rounded once), the root (`00ec6040`), then `FLD1; FDIVRP`.
#[inline(always)]
fn inverse_length(e: &mut Engine, x: f32) -> f32 {
    let one = e.global::<f64>(ONE_DOUBLE);
    let square = x * x;
    let square = narrow(f64::from(square) + one);
    let root = narrow(e.call(SQUARE_ROOT, &args![f64::from(square)]).f64());
    1.0 / root
}

/// What `NiPlane::NiPlane(normal, point)` (`00a69990`) makes, done here: the normal's words, and the
/// constant `(ny*py + nx*px) + nz*pz` (in `f64`, rounded where the exe stores it), stored into plane
/// `index`.
#[inline(always)]
fn set_plane(
    e: &mut Engine,
    this: Ptr<NiFrustumPlanes>,
    index: u32,
    normal: Floats,
    point: Floats,
) {
    let plane = this.cast::<NiPlane>().index(index, 0x10);
    let at = plane.at(NiPlane::m_kNormal);
    let components: [Field<NiPoint3, f32>; 3] = [NiPoint3::x, NiPoint3::y, NiPoint3::z];
    for (component, value) in components.iter().zip(normal) {
        e.set(at, *component, value);
    }
    let product = |i: usize| f64::from(normal[i]) * f64::from(point[i]);
    let constant = narrow((product(1) + product(0)) + product(2));
    e.set(plane, NiPlane::m_fConstant, constant);
}

// Typed from 00a74e10
/// `NiFrustumPlanes::Set` (Xbox PDB, `thiscall(planes, frustum, world transform)`): the six planes of
/// the frustum seen through the transform, with `c0`, `c1`, `c2` the rotation's columns (each entry
/// through a float slot) and `t` the translation (its words, unquieted). Near `(c0, t + near c0)`;
/// far `(-c0, far c0 + t)`. Orthographic: left `(c2, left c2 + t)`, right `(-c2, right c2 + t)`, top
/// `(-c1, top c1 + t)`, bottom `(c1, bottom c1 + t)`. Perspective, with `i = 1 / sqrt(x*x + 1)` for
/// each edge `x`, every plane through `t`: left `(-left i) c0 + i c2`, right `(right i) c0 + (-i)
/// c2`, top `(i top) c0 + (-i) c1`, bottom `(-bottom i) c0 + i c1`. Every product and sum is stored to
/// a float where the exe stores it (a single operation in `f32`, the planes' constants and the edge
/// lengths' `double` step in `f64`); the near point sums `t` first, the others the product first, as
/// the exe does.
pub fn frustum_planes_set(
    e: &mut Engine,
    this: Ptr<NiFrustumPlanes>,
    frustum: Ptr<NiFrustum>,
    transform: Ptr<NiTransform>,
) {
    let matrix = transform.at(NiTransform::m_Rotate);
    let (c0, c1, c2) = (
        column(e, matrix, 0),
        column(e, matrix, 1),
        column(e, matrix, 2),
    );
    let at = transform.at(NiTransform::m_Translate);
    let t: Floats = [
        e.get(at, NiPoint3::x),
        e.get(at, NiPoint3::y),
        e.get(at, NiPoint3::z),
    ];
    let stored = |e: &Engine, f: Field<NiFrustum, f32>| e.get(frustum, f);

    let near = scaled(c0, stored(e, NiFrustum::m_fNear));
    set_plane(e, this, 0, c0, sum(t, near));
    let far = scaled(c0, stored(e, NiFrustum::m_fFar));
    set_plane(e, this, 1, negated(c0), sum(far, t));

    if e.get(frustum, NiFrustum::m_bOrtho) != 0 {
        let left = scaled(c2, stored(e, NiFrustum::m_fLeft));
        set_plane(e, this, 2, c2, sum(left, t));
        let right = scaled(c2, stored(e, NiFrustum::m_fRight));
        set_plane(e, this, 3, negated(c2), sum(right, t));
        let top = scaled(c1, stored(e, NiFrustum::m_fTop));
        set_plane(e, this, 4, negated(c1), sum(top, t));
        let bottom = scaled(c1, stored(e, NiFrustum::m_fBottom));
        set_plane(e, this, 5, c1, sum(bottom, t));
        return;
    }

    // Left: u = -left * i.
    let i = inverse_length(e, e.get(frustum, NiFrustum::m_fLeft));
    let u = -e.get(frustum, NiFrustum::m_fLeft) * i;
    let normal = sum(scaled(c0, u), scaled(c2, i));
    set_plane(e, this, 2, normal, t);

    // Right: k = right * i, m = -i.
    let i = inverse_length(e, e.get(frustum, NiFrustum::m_fRight));
    let k = e.get(frustum, NiFrustum::m_fRight) * i;
    let m = -i;
    let normal = sum(scaled(c0, k), scaled(c2, m));
    set_plane(e, this, 3, normal, t);

    // Top: k = i * top (the float a memory operand), m = -i.
    let i = inverse_length(e, e.get(frustum, NiFrustum::m_fTop));
    let k = i * e.get(frustum, NiFrustum::m_fTop);
    let m = -i;
    let normal = sum(scaled(c0, k), scaled(c1, m));
    set_plane(e, this, 4, normal, t);

    // Bottom: k = -bottom * i.
    let i = inverse_length(e, e.get(frustum, NiFrustum::m_fBottom));
    let k = -e.get(frustum, NiFrustum::m_fBottom) * i;
    let normal = sum(scaled(c0, k), scaled(c1, i));
    set_plane(e, this, 5, normal, t);
}
