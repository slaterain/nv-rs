//! Expectations worked out from JIP's asm text (`internal/Ni_types.cpp`) by the independent
//! interpreter `measure/jip-cmds/h3-sse.pl` (its printed lanes, as bits), or by hand where a
//! comment says so; never from the Rust under test.

use super::*;

fn bits(m: &[f32]) -> Vec<u32> {
    m.iter().map(|v| v.to_bits()).collect()
}

/// The interpreter's chosen sine and cosine lanes (not real sines: only the lanes' maths count).
const S: [f32; 3] = [0.25, -0.5, 0.75];
const C: [f32; 3] = [0.875, 0.625, -0.375];
const A: [f32; 9] = [1.5, -2.25, 0.125, 3.0, 0.5, -1.75, 2.0, 0.375, -0.625];
const B: [f32; 9] = [-0.5, 1.25, 2.5, 0.75, -3.0, 0.25, 1.125, -0.875, 4.0];
/// h3-sse.pl `mul`: `A * B`.
const AB: [u32; 9] = [
    0xc013_0000,
    0x4108_4000,
    0x406c_0000,
    0xc046_0000,
    0x4072_0000,
    0x3f20_0000,
    0xbfb6_0000,
    0x3ff6_0000,
    0x4026_0000,
];

#[test]
fn from_euler_ypr_lanes_as_jip() {
    // h3-sse.pl `ypr`.
    assert_eq!(
        bits(&from_euler_ypr_sc(S, C)),
        [
            0xbea8_0000,
            0x3f28_0000,
            0xbd90_0000,
            0xbed8_0000,
            0xbea8_0000,
            0xbede_0000,
            0xbee0_0000,
            0xbe80_0000,
            0x3f0c_0000
        ]
    );
}

#[test]
fn from_euler_ypr_takes_the_angles_sines_and_cosines() {
    let pry = [0.3, -1.1, 2.0];
    let (s, c) = sin_cos(pry);
    assert_eq!(bits(&from_euler_ypr(pry)), bits(&from_euler_ypr_sc(s, c)));
}

/// `FromEulerPRYInv`'s lanes as read from its asm (by hand): `[c1*c2, (s0*s1)*c2 + -(c0*s2),
/// (c0*s1)*c2 + s0*s2, c1*s2, (s0*s1)*s2 + c0*c2, (c0*s1)*s2 + -(s0*c2), -s1, c1*s0, c1*c0]`.
fn pry_inv_by_hand(s: [f32; 3], c: [f32; 3]) -> [f32; 9] {
    let n = |v: f32| -v;
    [
        c[1] * c[2],
        (s[0] * s[1]) * c[2] + n(c[0] * s[2]),
        (c[0] * s[1]) * c[2] + s[0] * s[2],
        c[1] * s[2],
        (s[0] * s[1]) * s[2] + c[0] * c[2],
        (c[0] * s[1]) * s[2] + n(s[0] * c[2]),
        n(s[1]),
        c[1] * s[0],
        c[1] * c[0],
    ]
}

#[test]
fn from_euler_pry_inv_lanes_as_jip() {
    // The hand reading agrees with the interpreter (h3-sse.pl `pryinv`) ...
    assert_eq!(
        bits(&pry_inv_by_hand(S, C)),
        [
            0xbe70_0000,
            0xbf1c_0000,
            0x3eb4_0000,
            0x3ef0_0000,
            0xbed8_0000,
            0xbe70_0000,
            0x3f00_0000,
            0x3e20_0000,
            0x3f0c_0000
        ]
    );
    // ... and the port's matrix of real angles is that reading of their sines and cosines.
    for pry in [[0.3, -1.1, 2.0], [-2.9, 0.7, -0.05], [0.0, 0.0, 0.0]] {
        let (s, c) = sin_cos(pry);
        assert_eq!(bits(&from_euler_pry_inv(pry)), bits(&pry_inv_by_hand(s, c)));
    }
}

#[test]
fn multiply_matrices_lanes_as_jip() {
    assert_eq!(bits(&multiply_matrices(&A, &B)), AB);
}

#[test]
fn rotate_origin_as_jip() {
    // h3-sse.pl `rotorigin`: this = (A, (10.5, -4.25, 3.75)), origin = (B, (2.5, 1.25, -0.75)).
    let (rot, pos) = rotate_origin(&A, [10.5, -4.25, 3.75], &B, [2.5, 1.25, -0.75]);
    assert_eq!(bits(&rot), AB);
    assert_eq!(bits(&pos), [0x4038_0000, 0x41c7_0000, 0x41f8_8000]);
}

#[test]
fn rotate_origin_last_row_takes_the_origin_x_past_the_matrix() {
    // By hand: the last row's fourth float is origin x; an infinite x makes inf * 0 = NaN there.
    let (_, pos) = rotate_origin(&A, [1.0, 2.0, 3.0], &B, [f32::INFINITY, 0.0, 0.0]);
    assert!(pos[2].is_nan());
    assert!(!pos[1].is_nan());
}

#[test]
fn get_rotated_pos_as_jip() {
    // h3-sse.pl `rotpos`: the matrix of S and C, pos (10.5, -4.25, 3.75), origin (2.5, 1.25, -0.75).
    let m = from_euler_ypr_sc(S, C);
    let p = get_rotated_pos(&m, [10.5, -4.25, 3.75], [2.5, 1.25, -0.75]);
    assert_eq!(bits(&p), [0xc081_a000, 0xc011_6000, 0xbed4_0000]);
}

/// The interpreter's `rsqrtps`: 1/sqrt rounded to f32.
fn exact_rsqrt(v: f32) -> f32 {
    (1.0 / f64::from(v).sqrt()) as f32
}

#[test]
fn from_2_points_general_path_flips_y() {
    // h3-sse.pl `2pts` (2,5,1)-(1,3,3): x and y not 0, so the y lane flips.
    assert_eq!(
        bits(&from_2_points_with(
            [2.0, 5.0, 1.0],
            [1.0, 3.0, 3.0],
            exact_rsqrt
        )),
        [
            0xbf64_f92e,
            0xbeaa_aaab,
            0x3e98_a61f,
            0x3ee4_f92e,
            0xbf2a_aaab,
            0x3f18_a61f,
            0,
            0x3f2a_aaab,
            0x3f3e_cfa7
        ]
    );
}

#[test]
fn from_2_points_y_zero_keeps_y() {
    // h3-sse.pl `2pts` (10,4,7)-(13,4,3): y 0 and x not: no flip (m5 = -(sp * +0) = +0).
    assert_eq!(
        bits(&from_2_points_with(
            [10.0, 4.0, 7.0],
            [13.0, 4.0, 3.0],
            exact_rsqrt
        )),
        [
            0,
            0x3f19_999a,
            0x3f4c_cccd,
            0xbf80_0000,
            0,
            0,
            0,
            0xbf4c_cccd,
            0x3f19_999a
        ]
    );
}

#[test]
fn from_2_points_straight_up_or_down() {
    // h3-sse.pl `2pts` (5,-1,2)-(5,-1,-6): x and y 0 (the zZero branch), no square roots.
    assert_eq!(
        bits(&from_2_points([5.0, -1.0, 2.0], [5.0, -1.0, -6.0])),
        [
            0x3f80_0000,
            0x8000_0000,
            0x8000_0000,
            0,
            0,
            0x3f80_0000,
            0,
            0xbf80_0000,
            0
        ]
    );
    // By hand, the other way: -z positive, so sin pitch +1 and m2 = 0 * 1 = +0, m5 = -(1*1).
    assert_eq!(
        bits(&from_2_points([5.0, -1.0, -6.0], [5.0, -1.0, 2.0])),
        [
            0x3f80_0000,
            0x8000_0000,
            0,
            0,
            0,
            0xbf80_0000,
            0,
            0x3f80_0000,
            0
        ]
    );
}

#[test]
fn from_2_points_same_point() {
    // h3-sse.pl `2pts` (1,2,3) twice: the allZero branch.
    assert_eq!(
        bits(&from_2_points([1.0, 2.0, 3.0], [1.0, 2.0, 3.0])),
        [
            0x3f80_0000,
            0x8000_0000,
            0,
            0,
            0x3f80_0000,
            0x8000_0000,
            0,
            0,
            0x3f80_0000
        ]
    );
}

#[test]
fn from_2_points_uses_the_given_estimate() {
    // The processor's estimate and the exact root differ in low bits: the estimate is what runs.
    let est = from_2_points([2.0, 5.0, 1.0], [1.0, 3.0, 3.0]);
    let with = from_2_points_with(
        [2.0, 5.0, 1.0],
        [1.0, 3.0, 3.0],
        engine_core::x87::rsqrt_estimate,
    );
    assert_eq!(bits(&est), bits(&with));
}

#[test]
fn to_euler_pry_fourth_lane() {
    // By hand from ATan2's paths (utility.cpp): |m1| on the general path, else 0.
    let mut m = [0.5, -0.25, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0];
    assert_eq!(to_euler_pry_lane3(&m), 0.25);
    m[1] = 0.0;
    assert_eq!(to_euler_pry_lane3(&m).to_bits(), 0);
    m[1] = -0.25;
    m[0] = 0.0;
    assert_eq!(to_euler_pry_lane3(&m).to_bits(), 0);
    m[0] = f32::NAN;
    assert_eq!(to_euler_pry_lane3(&m).to_bits(), 0);
}
