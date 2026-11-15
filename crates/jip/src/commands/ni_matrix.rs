//! JIP LN NVSE's matrix maths for turning references (`internal/Ni_types.cpp`) that the earlier
//! lanes did not need: `NiMatrix33::FromEulerPRYInv`, `FromEulerYPR`, `From2Points`,
//! `MultiplyMatrices`, `NiTransform::RotateOrigin`, `NiVector3::GetRotatedPos`, and the fourth
//! lane `NiMatrix33::ToEulerPRY` leaves in its result (which `RotateAroundPoint` stores).
//! Lane E's `FromEulerPRY`, `MultiplyVector` and `GetSinCos_V3` (`jip_fn_misc_ref`) and lane
//! F's `ToEulerPRY`/`ToEulerPRYInv` (`nif_nodes`) are reused, not repeated.
//!
//! Each is JIP's asm lane by lane: every product and sum is one `f32` operation in the order its
//! `mulps`/`addps`/`subps` lanes make it, and each sign flip an `xorps` of the sign bit (so a
//! subtraction of a flipped value stays a subtraction, as the asm's `subps` is). The tests'
//! expectations were worked out from JIP's asm text by an independent interpreter
//! (`measure/jip-cmds/h3-sse.pl`), not from this file.

use super::jip_fn_misc_ref::{from_euler_pry, multiply_vector, sin_cos};

fn neg(v: f32) -> f32 {
    f32::from_bits(v.to_bits() ^ 0x8000_0000)
}

fn transpose(m: [f32; 9]) -> [f32; 9] {
    [m[0], m[3], m[6], m[1], m[4], m[7], m[2], m[5], m[8]]
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/Ni_types.cpp:
// NiMatrix33::FromEulerPRYInv (asm).
/// The inverse (transposed) rotation of pitch, roll and yaw: its lanes compute the very products
/// and sums of `FromEulerPRY` (`c1*c2`, `(s0*s1)*c2 + -(c0*s2)`, ...) and store them transposed,
/// so it is lane E's [`from_euler_pry`] transposed, bit for bit.
pub(crate) fn from_euler_pry_inv(pry: [f32; 3]) -> [f32; 9] {
    transpose(from_euler_pry(pry))
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/Ni_types.cpp: NiMatrix33::FromEulerYPR
// (asm).
/// The rotation of yaw, pitch and roll by the angles' sines `s` and cosines `c` (lanes 0, 1, 2):
/// rows `[(s1*s0)*s2 - -(c1*c2), c0*s2, (c1*s0)*s2 - s1*c2]`,
/// `[(s1*s0)*c2 - c1*s2, c0*c2, (c1*s0)*c2 - -(s1*s2)]`, `[c0*s1, -s0, c0*c1]`.
pub(crate) fn from_euler_ypr_sc(s: [f32; 3], c: [f32; 3]) -> [f32; 9] {
    let [s0, s1, s2] = s;
    let [c0, c1, c2] = c;
    [
        (s1 * s0) * s2 - neg(c1 * c2),
        c0 * s2,
        (c1 * s0) * s2 - s1 * c2,
        (s1 * s0) * c2 - c1 * s2,
        c0 * c2,
        (c1 * s0) * c2 - neg(s1 * s2),
        c0 * s1,
        neg(s0),
        c0 * c1,
    ]
}

/// [`from_euler_ypr_sc`] of the angles (`GetSinCos_V3`, the fourth lane masked off).
pub(crate) fn from_euler_ypr(pry: [f32; 3]) -> [f32; 9] {
    let (s, c) = sin_cos(pry);
    from_euler_ypr_sc(s, c)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/Ni_types.cpp:
// NiMatrix33::MultiplyMatrices (asm).
/// `a * b` (rows): each row of the result `(a[i][0]*b row 0 + a[i][1]*b row 1) + a[i][2]*b row 2`,
/// lane by lane as the asm's `mulps`/`addps`.
pub(crate) fn multiply_matrices(a: &[f32; 9], b: &[f32; 9]) -> [f32; 9] {
    std::array::from_fn(|k| {
        let (i, j) = (k / 3, k % 3);
        (a[3 * i] * b[j] + a[3 * i + 1] * b[3 + j]) + a[3 * i + 2] * b[6 + j]
    })
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/Ni_types.cpp: NiMatrix33::From2Points
// (asm), with `rsqrtps` as `rsqrt`.
/// The rotation that points from `pt2` to `pt1`, its two reciprocal square roots by `rsqrt`.
pub(crate) fn from_2_points_with(
    pt1: [f32; 3],
    pt2: [f32; 3],
    rsqrt: impl Fn(f32) -> f32,
) -> [f32; 9] {
    let d = [pt1[0] - pt2[0], pt1[1] - pt2[1], pt1[2] - pt2[2]];
    let sq = [d[0] * d[0], d[1] * d[1], d[2] * d[2]];
    // `haddps` twice: (x² + y²) + (z² + 0); then (x² + y²) alone.
    let len2 = (sq[0] + sq[1]) + (sq[2] + 0.0);
    // `comiss ... jz`: equal, or unordered (a NaN), takes the branch.
    let zero = |v: f32| v == 0.0 || v.is_nan();
    // [sin pitch, cos pitch, the two yaw terms].
    let [sp, cp, a, b] = if zero(len2) {
        // `allZero`: `pshufd PS_V3_One, 0x33` = [0, 1, 0, 1].
        [0.0, 1.0, 0.0, 1.0]
    } else {
        let minus_z = neg(d[2]);
        let h2 = sq[0] + sq[1];
        if zero(h2) {
            // `zZero`: [±1 with the sign of -z, 0, 0, 1].
            let sign = minus_z.to_bits() & 0x8000_0000;
            [f32::from_bits(sign | 1.0f32.to_bits()), 0.0, 0.0, 1.0]
        } else {
            // The inverse lengths by `rsqrtps` and one Newton step: ((3 - (v*r)*r)*r)*0.5.
            let inv = |v: f32| {
                let r = rsqrt(v);
                ((3.0 - (v * r) * r) * r) * 0.5
            };
            let (inv_len, inv_h) = (inv(len2), inv(h2));
            let h = h2 * inv_h;
            // The last lane flips unless x is not 0 and y is 0 (`cmpeqps`, mask bits 0 and 1 = 2).
            let x_zero = d[0] == 0.0;
            let y_zero = d[1] == 0.0;
            let y = if !x_zero && y_zero { d[1] } else { neg(d[1]) };
            [minus_z * inv_len, h * inv_len, d[0] * inv_h, y * inv_h]
        }
    };
    [b, neg(a * cp), a * sp, a, cp * b, neg(sp * b), 0.0, sp, cp]
}

/// [`from_2_points_with`] with the processor's own estimate, as JIP runs it.
pub(crate) fn from_2_points(pt1: [f32; 3], pt2: [f32; 3]) -> [f32; 9] {
    from_2_points_with(pt1, pt2, engine_core::x87::rsqrt_estimate)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/Ni_types.cpp: NiMatrix33::ToEulerPRY
// (asm), its result's fourth lane.
/// What `ToEulerPRY` leaves in its fourth lane: the second lane of its first `ATan2(m1, m0)`
/// (`movq xmm6` keeps it, `unpcklpd` moves it up): `|m1|` on `ATan2`'s general path (both `m1`
/// and `m0` not 0 and not NaN), else 0 (its early returns clear the upper lanes).
pub(crate) fn to_euler_pry_lane3(m: &[f32; 9]) -> f32 {
    let ordered_nonzero = |v: f32| v != 0.0 && !v.is_nan();
    if ordered_nonzero(m[1]) && ordered_nonzero(m[0]) {
        m[1].abs()
    } else {
        0.0
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/Ni_types.cpp: NiTransform::RotateOrigin
// (asm).
/// A transform (`rotate`, `translate`) turned about `origin`'s point by `origin`'s rotation:
/// the translation `origin_rot * (translate - origin_pos) + origin_pos` (the product's last row's
/// fourth float is the word after `origin`'s matrix: its translation's x), then
/// `rotate * origin_rot`.
pub(crate) fn rotate_origin(
    rotate: &[f32; 9],
    translate: [f32; 3],
    origin_rot: &[f32; 9],
    origin_pos: [f32; 3],
) -> ([f32; 9], [f32; 3]) {
    let v = [
        translate[0] - origin_pos[0],
        translate[1] - origin_pos[1],
        translate[2] - origin_pos[2],
    ];
    let r = multiply_vector(origin_rot, origin_pos[0], v);
    let pos = [
        r[0] + origin_pos[0],
        r[1] + origin_pos[1],
        r[2] + origin_pos[2],
    ];
    (multiply_matrices(rotate, origin_rot), pos)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/Ni_types.cpp: NiVector3::GetRotatedPos
// (asm).
/// `pos` turned about `origin` by the yaw, pitch and roll matrix `m`: `m * (pos - origin) +
/// origin`. The product's last row's fourth float is the word after JIP's matrix on its stack (a
/// stack address: finite, so its product with the cleared lane is +0).
pub(crate) fn get_rotated_pos(m: &[f32; 9], pos: [f32; 3], origin: [f32; 3]) -> [f32; 3] {
    let v = [pos[0] - origin[0], pos[1] - origin[1], pos[2] - origin[2]];
    let r = multiply_vector(m, 0.0, v);
    [r[0] + origin[0], r[1] + origin[1], r[2] + origin[2]]
}

#[cfg(test)]
#[path = "ni_matrix_tests.rs"]
mod tests;
