//! Tests of the maths commands: JIP's routines against the standard library within a few units in
//! the last place (they are polynomial approximations), every early answer of the asm exactly,
//! and the distances on references at the raw offsets (position +0x30, parent cell +0x40, the
//! cell's worldspace +0xC0, `ExtraPersistentCell` type 0x0C with its cell at +0xC).

use super::*;
use crate::commands::test_support::*;

const REF_A: u32 = FORMS + 0x1000;
const REF_B: u32 = FORMS + 0x2000;
const CELL_1: u32 = FORMS + 0x3000;
const CELL_2: u32 = FORMS + 0x3100;
const WS: u32 = FORMS + 0x3200;
const XP: u32 = FORMS + 0x4000;

/// The difference in units in the last place of two floats of one sign.
fn ulps(a: f32, b: f32) -> u32 {
    (a.to_bits() as i64 - b.to_bits() as i64).unsigned_abs() as u32
}

fn close(a: f32, b: f64, abs: f64) -> bool {
    (f64::from(a) - b).abs() <= abs
}

#[test]
fn cosine_and_sine_follow_the_standard_ones() {
    for i in -2000..=2000 {
        let x = i as f32 * 0.01;
        assert!(close(cos(x), f64::from(x).cos(), 2e-6), "cos {x}: {}", cos(x));
        assert!(close(sin(x), f64::from(x).sin(), 2e-6), "sin {x}: {}", sin(x));
        assert!(close(cos_lane(x), f64::from(x).cos(), 2e-6), "lane {x}");
    }
}

/// `Cos`'s early answer: exactly 1 for a reduced angle of 0 or NaN; `Cos_V3` has none (the
/// polynomial's last constant at 0, NaN through).
#[test]
fn cosine_at_zero_and_nan() {
    assert_eq!(cos(0.0).to_bits(), 1.0f32.to_bits());
    assert_eq!(cos(-0.0).to_bits(), 1.0f32.to_bits());
    assert_eq!(cos(f32::NAN).to_bits(), 1.0f32.to_bits());
    assert_eq!(cos_lane(0.0).to_bits(), 0x3f7f_ffff);
    assert!(cos_lane(f32::NAN).is_nan());
    // Quadrant 2 (between pi and 3pi/2): the sign flipped.
    assert!(cos(3.5) < 0.0 && cos(-3.5) < 0.0);
    // Past 3 quadrants: whole turns taken off, the same answer as one turn less within 2 ulps.
    assert!(ulps(cos(1.0 + 2.0 * FLT_PI_X2), cos(1.0)) < 64);
}

#[test]
fn tangent_and_its_cut_off() {
    for i in -150..=150 {
        let x = i as f32 * 0.01;
        assert!(
            close(tan(x), f64::from(x).tan(), 2e-5 * (1.0 + f64::from(x).tan().powi(2))),
            "tan {x}: {}",
            tan(x)
        );
    }
    // A cosine below FLT_EPSILON in size: 0.
    assert_eq!(tan(FLT_PI_D2), 0.0);
    assert_eq!(tan(f32::NAN), 0.0);
}

/// JIP's inverse routines are coarser than the forward ones: up to about 8e-6 from the true
/// value (ASin's cubic and its 128 corrections; ATan2's polynomial, whose last constant is
/// 0x3F7FFFB7, a little under 1: atan2(-1, -1) is -2.356198 against -2.3561945).
#[test]
fn inverse_functions_follow_the_standard_ones() {
    for i in -999..=999 {
        let x = i as f32 * 0.001;
        assert!(close(asin(x), f64::from(x).asin(), 1e-5), "asin {x}: {}", asin(x));
        assert!(close(acos(x), f64::from(x).acos(), 1e-5), "acos {x}: {}", acos(x));
        assert!(close(atan(x * 9.0), f64::from(x * 9.0).atan(), 1e-5), "atan {x}");
    }
    for (y, x) in [(1.0f32, 2.0f32), (-1.0, 2.0), (1.0, -2.0), (-3.0, -0.5), (5.0, 0.25)] {
        assert!(close(atan2(y, x), f64::from(y).atan2(f64::from(x)), 1e-5), "atan2 {y} {x}");
    }
}

/// The early answers of `ASin`, `ACos` and `ATan2`, bit for bit.
#[test]
fn inverse_functions_edges() {
    let pid2 = 0x3fc9_0fdb;
    assert_eq!(asin(0.0).to_bits(), 0);
    assert_eq!(asin(-0.0).to_bits(), 0x8000_0000);
    assert!(asin(f32::NAN).is_nan());
    assert_eq!(asin(1.0).to_bits(), pid2);
    assert_eq!(asin(-7.0).to_bits(), pid2 | 0x8000_0000);
    assert_eq!(acos(0.0).to_bits(), pid2);
    assert_eq!(acos(f32::NAN).to_bits(), pid2);
    assert_eq!(acos(1.0).to_bits(), 0);
    assert_eq!(acos(-1.5).to_bits(), 0x4049_0fdb);
    // y of 0: y itself for x of 0 or more, else pi (also for a NaN x).
    assert_eq!(atan2(-0.0, 3.0).to_bits(), 0x8000_0000);
    assert_eq!(atan2(0.0, 0.0).to_bits(), 0);
    assert_eq!(atan2(-0.0, -1.0).to_bits(), 0x4049_0fdb);
    assert_eq!(atan2(0.0, f32::NAN).to_bits(), 0x4049_0fdb);
    assert!(atan2(f32::NAN, 1.0).is_nan());
    // x of 0 or NaN: pi/2 with y's sign.
    assert_eq!(atan2(-2.0, 0.0).to_bits(), pid2 | 0x8000_0000);
    assert_eq!(atan2(2.0, f32::NAN).to_bits(), pid2);
    // The table index is trunc(x * 128): the last entry for x just below 1.
    assert_eq!(asin_index(0.999_999_9), 127);
    assert_eq!(asin_index(0.5), 64);
    assert_eq!(asin_index(0.007_8), 0);
}

#[test]
fn distances_in_one_cell() {
    let a = [1.0, 2.0, 3.0];
    let b = [4.0, 6.0, 15.0];
    // 3-4-5 in the plane; 3, 4, 12 gives 13 in space. The Newton step lands within an ulp.
    assert!(ulps(point2_distance(a, b), 5.0) <= 1);
    assert!(ulps(point3_distance(a, b), 13.0) <= 1);
    assert_eq!(point2_distance(a, a).to_bits(), 0);
    assert!(point3_distance([f32::NAN, 0.0, 0.0], b).is_nan());
}

fn setup() -> Engine {
    let mut e = engine();
    extra_stand_ins(&mut e);
    for (r, pos) in [(REF_A, [1.0f32, 2.0, 3.0]), (REF_B, [4.0, 6.0, 15.0])] {
        for (i, v) in pos.iter().enumerate() {
            e.mem.set_f32(r + 0x30 + 4 * i as u32, *v);
        }
        // The scale after the position: not part of the distance (`PS_XYZ0Mask`).
        e.mem.set_f32(r + 0x3c, 1000.0 * pos[0]);
    }
    e.mem.set_u32(REF_A + 0x40, CELL_1);
    e.mem.set_u32(REF_B + 0x40, CELL_1);
    form(&mut e, REF_A, 0x3a, 0x0001_0001);
    form(&mut e, REF_B, 0x3a, 0x0001_0002);
    set_ref(&mut e, 1, REF_B);
    e
}

fn eval(e: &mut Engine, opcode: u32, this: u32, arg1: u32) -> f64 {
    let record = nvse::table::record_of(e, opcode);
    let at = e.mem.u32(record + 0x20);
    assert_ne!(at, 0, "{opcode:04x}: an eval");
    e.mem.set_f64(RESULT, 0.5);
    e.call(at, &[this, arg1, 0, RESULT]);
    e.mem.f64(RESULT)
}

#[test]
fn distance_commands_and_conditions() {
    let mut e = setup();
    let d2 = f64::from(point2_distance([1.0, 2.0, 3.0], [4.0, 6.0, 15.0]));
    let d3 = f64::from(point3_distance([1.0, 2.0, 3.0], [4.0, 6.0, 15.0]));
    assert_eq!(call_on(&mut e, 0x22e0, REF_A, &[r(1)]).1, d2);
    assert_eq!(call_on(&mut e, 0x22e1, REF_A, &[r(1)]).1, d3);
    assert_eq!(eval(&mut e, 0x22e0, REF_A, REF_B), d2);
    assert_eq!(eval(&mut e, 0x22e1, REF_A, REF_B), d3);
    // Different cells, no worldspace: FLT_MAX from all four.
    e.mem.set_u32(REF_B + 0x40, CELL_2);
    let max = f64::from(f32::MAX);
    assert_eq!(call_on(&mut e, 0x22e0, REF_A, &[r(1)]).1, max);
    assert_eq!(call_on(&mut e, 0x22e1, REF_A, &[r(1)]).1, max);
    assert_eq!(eval(&mut e, 0x22e0, REF_A, REF_B), max);
    assert_eq!(eval(&mut e, 0x22e1, REF_A, REF_B), max);
    // Both cells in one worldspace: the distance again.
    e.mem.set_u32(CELL_1 + 0xc0, WS);
    e.mem.set_u32(CELL_2 + 0xc0, WS);
    assert_eq!(eval(&mut e, 0x22e1, REF_A, REF_B), d3);
    // Another worldspace: FLT_MAX.
    e.mem.set_u32(CELL_2 + 0xc0, WS + 4);
    assert_eq!(eval(&mut e, 0x22e0, REF_A, REF_B), max);
}

#[test]
fn a_reference_with_no_parent_cell_uses_its_persistent_cell() {
    let mut e = setup();
    e.mem.set_u32(REF_B + 0x40, 0);
    let max = f64::from(f32::MAX);
    // No extra: not in the same cell (both orders).
    assert_eq!(eval(&mut e, 0x22e1, REF_A, REF_B), max);
    assert_eq!(eval(&mut e, 0x22e1, REF_B, REF_A), max);
    put_extra(&mut e, REF_B + 0x44, XP, 0x0c, CELL_1);
    let d3 = f64::from(point3_distance([1.0, 2.0, 3.0], [4.0, 6.0, 15.0]));
    assert_eq!(eval(&mut e, 0x22e1, REF_A, REF_B), d3);
    assert_eq!(eval(&mut e, 0x22e1, REF_B, REF_A), d3);
    // Its persistent cell elsewhere: FLT_MAX.
    e.mem.set_u32(XP + 0xc, CELL_2);
    assert_eq!(eval(&mut e, 0x22e0, REF_B, REF_A), max);
}

#[test]
fn square_root_command() {
    let mut e = engine();
    assert_eq!(call(&mut e, 0x22f0, &[z(6.25)]).1, 2.5);
    assert_eq!(call(&mut e, 0x22f0, &[z(-4.0)]).1, 0.0, "negative: 0");
    let r = call(&mut e, 0x22f0, &[z(-0.0)]).1;
    assert_eq!(r.to_bits(), (-0.0f64).to_bits(), "-0 passes maxsd");
    assert!(call(&mut e, 0x22f0, &[z(f64::NAN)]).1.is_nan());
    // Double precision throughout.
    assert_eq!(call(&mut e, 0x22f0, &[z(2.0)]).1, 2f64.sqrt());
}

/// Each trigonometric command: its float argument (degrees for the forward ones), JIP's routine,
/// the float answer widened (times `Dbl180dPI` for the inverse ones).
#[test]
fn trigonometric_commands() {
    let mut e = engine();
    let one = |e: &mut Engine, op: u32, v: f64| call(e, op, &[z(v)]).1;
    assert_eq!(one(&mut e, 0x22fd, 30.0), f64::from(sin(30.0 * FLT_PI_D_180)));
    assert!((one(&mut e, 0x22fd, 30.0) - 0.5).abs() < 1e-6);
    assert_eq!(one(&mut e, 0x22fe, 60.0), f64::from(cos(60.0 * FLT_PI_D_180)));
    assert_eq!(one(&mut e, 0x22fe, 0.0), 1.0);
    assert_eq!(one(&mut e, 0x2893, 45.0), f64::from(tan(45.0 * FLT_PI_D_180)));
    assert_eq!(one(&mut e, 0x2893, 90.0), 0.0, "the cosine cut off");
    assert_eq!(one(&mut e, 0x230f, 0.5), f64::from(asin(0.5)) * DBL_180_D_PI);
    assert!((one(&mut e, 0x230f, 0.5) - 30.0).abs() < 1e-4);
    assert_eq!(one(&mut e, 0x2310, -1.0), f64::from(FLT_PI) * DBL_180_D_PI);
    assert_eq!(one(&mut e, 0x230b, 1.0), f64::from(atan(1.0)) * DBL_180_D_PI);
    // The double argument read as a float: 0.1 rounds first.
    assert_eq!(one(&mut e, 0x230f, 0.1), f64::from(asin(0.1f32)) * DBL_180_D_PI);
    let r = call(&mut e, 0x2894, &[z(-1.0), z(-1.0)]).1;
    assert_eq!(r, f64::from(atan2(-1.0, -1.0)) * DBL_180_D_PI);
    assert!((r + 135.0).abs() < 1e-3);
    // y first, then x.
    let r = call(&mut e, 0x2894, &[z(1.0), z(0.0)]).1;
    assert_eq!(r, f64::from(FLT_PI_D2) * DBL_180_D_PI);
}

