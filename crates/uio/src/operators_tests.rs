//! UIO's operators, each against the values its `DoTileOperator*` gives (worked from its
//! instructions: SSE's rules for NaNs, zeros and conversions), and its jump table's seam.

use super::*;
use std::cell::RefCell;

const QNAN: u32 = 0x7fc0_0000;
const QNAN_NEG: u32 = 0xffc0_0000;
const SNAN: u32 = 0x7f80_0001;
const NEG_ZERO: u32 = 0x8000_0000;
const MIN_INT: f32 = -2_147_483_648.0;

fn no_random(_: u32) -> u32 {
    panic!("no draw expected")
}

/// The operation on floats: the new float, or `None` for "unchanged".
fn op(kind: i32, value: f32, operand: f32) -> Option<f32> {
    arithmetic(
        kind,
        value.to_bits(),
        operand.to_bits(),
        false,
        &mut no_random,
    )
    .map(f32::from_bits)
}

/// The same on bits, with or without a string.
fn bits(kind: i32, value: u32, operand: u32, text: bool) -> Option<u32> {
    arithmetic(kind, value, operand, text, &mut no_random)
}

const ADD: i32 = 0x7d1;
const SUB: i32 = 0x7d2;
const MUL: i32 = 0x7d3;
const DIV: i32 = 0x7d4;
const MIN: i32 = 0x7d5;
const MAX: i32 = 0x7d6;
const MOD: i32 = 0x7d7;
const FLOOR: i32 = 0x7d8;
const CEIL: i32 = 0x7d9;
const ABS: i32 = 0x7da;
const ROUND: i32 = 0x7db;
const GT: i32 = 0x7dc;
const EQ: i32 = 0x7de;
const LT: i32 = 0x7e0;
const AND: i32 = 0x7e2;
const OR: i32 = 0x7e3;
const NOT: i32 = 0x7e4;
const ONLYIF: i32 = 0x7e5;
const ONLYIFNOT: i32 = 0x7e6;
const RECIPR: i32 = 0x7eb;
const LAND: i32 = 0x7ec;
const LOR: i32 = 0x7ed;
const SHL: i32 = 0x7ee;
const SHR: i32 = 0x7ef;
const BT: i32 = 0x7f0;
const SQRT: i32 = 0x7f1;
const POW: i32 = 0x7f2;
const SIN: i32 = 0x7f3;
const COS: i32 = 0x7f4;
const TAN: i32 = 0x7f5;
const ASIN: i32 = 0x7f6;
const ACOS: i32 = 0x7f7;
const ATAN: i32 = 0x7f8;
const LOG: i32 = 0x7f9;

#[test]
fn the_table_and_the_names_are_uios() {
    assert_eq!(COPY + COUNT as i32 - 1, RAND);
    assert_eq!(NAMES.len(), 20);
    // The new operators are kAction_neg .. kAction_rand, one after the other, after the aliases.
    for (i, (_, id)) in NAMES[3..].iter().enumerate() {
        assert_eq!(*id, NEG + i as i32);
    }
    assert_eq!(NAMES[..3], [("ge", GTE), ("ne", NEQ), ("le", LTE)]);
    assert_eq!((GTE, NEQ, LTE), (0x7dd, 0x7df, 0x7e1));
}

#[test]
fn conversions_round_to_even_and_give_the_indefinite_integer() {
    assert_eq!(cvt_round(2.5), 2);
    assert_eq!(cvt_round(3.5), 4);
    assert_eq!(cvt_round(-2.5), -2);
    assert_eq!(cvt_round(0.5), 0);
    assert_eq!(cvt_round(-0.5), 0);
    assert_eq!(cvt_round(1.5), 2);
    assert_eq!(cvt_round(-1.5), -2);
    assert_eq!(cvt_round(2.4999998), 2);
    assert_eq!(cvt_round(8_388_609.0), 8_388_609);
    assert_eq!(cvt_round(-0.6), -1);
    assert_eq!(cvt_trunc(-2.9), -2);
    assert_eq!(cvt_trunc(2.9), 2);
    for x in [
        f32::NAN,
        f32::INFINITY,
        f32::NEG_INFINITY,
        2_147_483_648.0,
        -3e9,
    ] {
        assert_eq!(cvt_round(x), i32::MIN, "{x}");
        assert_eq!(cvt_trunc(x), i32::MIN, "{x}");
    }
    assert_eq!(cvt_round(-2_147_483_648.0), i32::MIN);
    assert_eq!(cvt_round(2_147_483_520.0), 2_147_483_520);
}

#[test]
fn add_sub_mul_div_are_single_precision_with_sses_nans() {
    assert_eq!(op(ADD, 10.0, 5.0), Some(15.0));
    assert_eq!(op(ADD, 0.1, 0.2), Some(0.1f32 + 0.2f32));
    assert_eq!(op(SUB, 10.0, 4.0), Some(6.0));
    assert_eq!(op(MUL, 10.0, 3.0), Some(30.0));
    assert_eq!(op(MUL, 3e38, 10.0), Some(f32::INFINITY));
    assert_eq!(op(DIV, 10.0, 4.0), Some(2.5));
    // The value's NaN wins (quieted), then the operand's; an invalid pair gives the default NaN.
    assert_eq!(bits(ADD, SNAN, 1.0f32.to_bits(), false), Some(0x7fc0_0001));
    assert_eq!(bits(SUB, 1.0f32.to_bits(), QNAN, false), Some(QNAN));
    assert_eq!(
        bits(SUB, f32::INFINITY.to_bits(), f32::INFINITY.to_bits(), false),
        Some(QNAN_NEG)
    );
    assert_eq!(bits(MUL, 0, f32::INFINITY.to_bits(), false), Some(QNAN_NEG));
    // Division: by +0 nothing; by -0 (other bits) an infinity.
    assert_eq!(op(DIV, 10.0, 0.0), None);
    assert_eq!(op(DIV, 10.0, -0.0), Some(f32::NEG_INFINITY));
    assert_eq!(bits(DIV, 0, NEG_ZERO, false), Some(QNAN_NEG));
}

#[test]
fn min_and_max_answer_the_operand_on_a_nan_or_two_zeros() {
    assert_eq!(op(MIN, 10.0, 4.0), Some(4.0));
    assert_eq!(op(MIN, 3.0, 4.0), Some(3.0));
    assert_eq!(op(MAX, 10.0, 40.0), Some(40.0));
    assert_eq!(op(MAX, 50.0, 40.0), Some(50.0));
    assert_eq!(
        bits(MIN, QNAN, 4.0f32.to_bits(), false),
        Some(4.0f32.to_bits())
    );
    assert_eq!(bits(MAX, 4.0f32.to_bits(), SNAN, false), Some(SNAN));
    assert_eq!(bits(MIN, 0, NEG_ZERO, false), Some(NEG_ZERO));
    assert_eq!(bits(MAX, NEG_ZERO, 0, false), Some(0));
}

#[test]
fn mod_works_on_the_rounded_integers() {
    assert_eq!(op(MOD, 13.0, 5.0), Some(3.0));
    assert_eq!(op(MOD, 13.4, 5.6), Some(1.0));
    assert_eq!(op(MOD, -7.0, 3.0), Some(-1.0));
    assert_eq!(op(MOD, 7.0, -3.0), Some(1.0));
    assert_eq!(op(MOD, 2.5, 3.0), Some(2.0));
    // A divisor rounding to 0: nothing.
    assert_eq!(op(MOD, 13.0, 0.4), None);
    assert_eq!(op(MOD, 13.0, -0.5), None);
    // A NaN value is the indefinite integer.
    assert_eq!(op(MOD, f32::NAN, 5.0), Some(-3.0));
    // 0x80000000 / -1 faults in UIO's idiv; here 0.
    assert_eq!(op(MOD, MIN_INT, -1.0), Some(0.0));
}

#[test]
fn floor_ceil_abs_take_the_sum_and_floor_ceil_go_through_an_integer() {
    assert_eq!(op(FLOOR, -3.5, 1.0), Some(-3.0));
    assert_eq!(op(FLOOR, 1.25, 1.0), Some(2.0));
    assert_eq!(op(FLOOR, -0.5, 0.0), Some(-1.0));
    assert_eq!(op(CEIL, 1.25, 1.0), Some(3.0));
    assert_eq!(op(CEIL, -2.5, 0.0), Some(-2.0));
    // FLOOR's MXCSR 0x3FA0 rounds down, never to nearest: 2.7 gives 2 (to nearest: 3).
    assert_eq!(op(FLOOR, 2.0, 0.7), Some(2.0));
    // Out of the integer range or NaN: the indefinite integer, as a float.
    assert_eq!(op(FLOOR, 3e9, 0.0), Some(MIN_INT));
    assert_eq!(op(CEIL, f32::NAN, 0.0), Some(MIN_INT));
    assert_eq!(op(ABS, -3.5, 1.0), Some(2.5));
    assert_eq!(op(ABS, -5.0, 0.0), Some(5.0));
    assert_eq!(bits(ABS, NEG_ZERO, 0, false), Some(0));
    assert_eq!(bits(ABS, QNAN_NEG, 0, false), Some(QNAN));
}

#[test]
fn round_is_to_a_multiple_of_a_positive_operand_with_uios_hundredth() {
    assert_eq!(op(ROUND, 7.3, 2.0), Some(8.0));
    assert_eq!(op(ROUND, 6.9, 2.0), Some(6.0));
    assert_eq!(op(ROUND, 5.0, 2.0), Some(6.0));
    assert_eq!(op(ROUND, -5.0, 2.0), Some(-4.0));
    // value / operand + 0.01 rounded to nearest: 4.97 / 2 + 0.01 = 2.495 -> 2.
    assert_eq!(op(ROUND, 4.97, 2.0), Some(4.0));
    // A tie: 4.98f32 / 2 + 0.01f32 is exactly 2.5 in single precision, and `cvtps2dq` takes it
    // to the even 2 (half away from zero would give 3, so 6).
    assert_eq!(sse::add(sse::div(4.98, 2.0), HUNDREDTH), 2.5);
    assert_eq!(op(ROUND, 4.98, 2.0), Some(4.0));
    // A value of +0, an operand not above +0: nothing.
    assert_eq!(op(ROUND, 0.0, 2.0), None);
    assert_eq!(op(ROUND, 7.0, 0.0), None);
    assert_eq!(op(ROUND, 7.0, -2.0), None);
    assert_eq!(bits(ROUND, 7.0f32.to_bits(), QNAN_NEG, false), None);
    // A positive NaN operand runs (the indefinite integer times NaN: the NaN).
    assert_eq!(bits(ROUND, 7.0f32.to_bits(), QNAN, false), Some(QNAN));
}

#[test]
fn comparisons_follow_comiss_and_its_flags() {
    let all = |v: f32, o: f32| -> Vec<f32> { (GT..=LTE).map(|k| op(k, v, o).unwrap()).collect() };
    // gt gte eq neq lt lte
    assert_eq!(all(5.0, 3.0), [1.0, 1.0, 0.0, 1.0, 0.0, 0.0]);
    assert_eq!(all(3.0, 3.0), [0.0, 1.0, 1.0, 0.0, 0.0, 1.0]);
    assert_eq!(all(2.0, 3.0), [0.0, 0.0, 0.0, 1.0, 1.0, 1.0]);
    assert_eq!(all(0.0, -0.0), [0.0, 1.0, 1.0, 0.0, 0.0, 1.0]);
    // Unordered sets ZF and CF: eq, lt and lte answer 1.
    assert_eq!(all(f32::NAN, 3.0), [0.0, 0.0, 1.0, 0.0, 1.0, 1.0]);
    assert_eq!(all(3.0, f32::NAN), [0.0, 0.0, 1.0, 0.0, 1.0, 1.0]);
    assert_eq!(op(EQ, 1.0, 1.0), Some(1.0));
    assert_eq!(op(LT, 1.0, 2.0), Some(1.0));
}

#[test]
fn logic_operators_test_bits_and_the_string() {
    let one = Some(1.0f32.to_bits());
    let f = |x: f32| x.to_bits();
    // and: a +0 value stays; -0 is "true"; a string makes 1.
    assert_eq!(bits(AND, f(2.0), f(3.0), false), one);
    assert_eq!(bits(AND, 0, f(3.0), false), None);
    assert_eq!(bits(AND, f(2.0), 0, false), Some(0));
    assert_eq!(bits(AND, NEG_ZERO, NEG_ZERO, false), one);
    assert_eq!(bits(AND, f(2.0), 0, true), one);
    // or: with a string only the value counts.
    assert_eq!(bits(OR, 0, 0, false), Some(0));
    assert_eq!(bits(OR, 0, f(2.0), false), one);
    assert_eq!(bits(OR, f(3.0), 0, false), one);
    assert_eq!(bits(OR, 0, f(2.0), true), Some(0));
    assert_eq!(bits(OR, NEG_ZERO, 0, false), one);
    // `or ecx, [ebp-0x38]`: the operand's bits, so a -0 operand is "true".
    assert_eq!(bits(OR, 0, NEG_ZERO, false), one);
    // not: 0 with a string; -0 is not +0.
    assert_eq!(bits(NOT, f(9.0), 0, false), one);
    assert_eq!(bits(NOT, f(9.0), f(5.0), false), Some(0));
    assert_eq!(bits(NOT, f(9.0), 0, true), Some(0));
    assert_eq!(bits(NOT, f(9.0), NEG_ZERO, false), Some(0));
    // onlyif / onlyifnot.
    assert_eq!(bits(ONLYIF, f(9.0), 0, false), Some(0));
    assert_eq!(bits(ONLYIF, f(9.0), f(1.0), false), None);
    assert_eq!(bits(ONLYIF, f(9.0), 0, true), None);
    assert_eq!(bits(ONLYIF, f(9.0), NEG_ZERO, false), None);
    assert_eq!(bits(ONLYIFNOT, f(9.0), f(1.0), false), Some(0));
    assert_eq!(bits(ONLYIFNOT, f(9.0), 0, false), None);
    assert_eq!(bits(ONLYIFNOT, f(9.0), 0, true), Some(0));
    // `mov ecx,[ebp-0x38]; or ecx,[ebp-0x34]; jz`: bits, so a -0 operand zeroes the value.
    assert_eq!(bits(ONLYIFNOT, f(9.0), NEG_ZERO, false), Some(0));
}

#[test]
fn copy_ref_the_parentheses_and_ids_outside_the_table_change_nothing_here() {
    for kind in [COPY, 0x7e7, 0x7e8, 0x7e9, 0x7fb, 0x7cf, 0, -1, i32::MAX] {
        assert_eq!(op(kind, 9.0, 1.0), None, "{kind:x}");
    }
}

#[test]
fn neg_and_recipr_take_the_operand() {
    assert_eq!(op(NEG, 9.0, 5.0), Some(-5.0));
    assert_eq!(op(NEG, 9.0, -5.0), Some(5.0));
    assert_eq!(bits(NEG, 1, 0, false), Some(0));
    assert_eq!(bits(NEG, 1, NEG_ZERO, false), Some(0));
    assert_eq!(bits(NEG, 1, QNAN, false), Some(QNAN_NEG));
    assert_eq!(op(NEG, 9.0, f32::INFINITY), Some(f32::NEG_INFINITY));
    // recipr: +0 gives 0; else what the `rcpss` instruction gives. UIO executes `rcpss`, whose
    // exact bits differ between processor makers (Intel's and AMD's estimate tables), so no
    // table of one machine's values can pin it. Instead: the answer is the instruction itself,
    // run here directly; it is an estimate, not the correctly rounded 1 / x (for 1/3 and the
    // others below at least one differs on any x86_64); and it is within the instruction's
    // documented relative error, 1.5 * 2^-12.
    assert_eq!(bits(RECIPR, 7, 0, false), Some(0));
    #[cfg(target_arch = "x86_64")]
    let instruction = |x: f32| {
        use std::arch::x86_64::{_mm_cvtss_f32, _mm_rcp_ss, _mm_set_ss};
        // SAFETY: SSE is part of the x86_64 baseline.
        #[allow(unused_unsafe)]
        unsafe {
            _mm_cvtss_f32(_mm_rcp_ss(_mm_set_ss(x)))
        }
    };
    let inputs = [4.0f32, -3.0, 0.1, 1e30, 7.5, 3.0, 7.0, 11.0];
    for x in inputs {
        let r = op(RECIPR, 9.0, x).unwrap();
        assert!(
            ((r - 1.0 / x) / (1.0 / x)).abs() <= 1.5 / 4096.0,
            "{x}: {r}"
        );
        #[cfg(target_arch = "x86_64")]
        assert_eq!(r.to_bits(), instruction(x).to_bits(), "{x}");
    }
    #[cfg(target_arch = "x86_64")]
    assert!(
        inputs
            .iter()
            .any(|&x| op(RECIPR, 9.0, x).unwrap() != 1.0 / x),
        "an estimate, not the quotient"
    );
    assert_eq!(op(RECIPR, 9.0, -0.0), Some(f32::NEG_INFINITY));
    assert_eq!(op(RECIPR, 9.0, f32::INFINITY), Some(0.0));
    assert!(op(RECIPR, 9.0, f32::NAN).unwrap().is_nan());
}

#[test]
fn integer_operators_round_both_and_shifts_count_from_the_quadword() {
    assert_eq!(op(LAND, 12.0, 10.0), Some(8.0));
    assert_eq!(op(LAND, 12.6, 10.0), Some(8.0));
    assert_eq!(op(LAND, -1.0, 255.0), Some(255.0));
    assert_eq!(op(LAND, f32::NAN, -1.0), Some(MIN_INT));
    assert_eq!(op(LOR, 12.0, 3.0), Some(15.0));
    assert_eq!(op(LOR, 2.5, 1.0), Some(3.0));
    assert_eq!(op(SHL, 1.0, 4.0), Some(16.0));
    assert_eq!(op(SHL, 3.0, 0.0), Some(3.0));
    assert_eq!(op(SHL, 1.0, 31.0), Some(MIN_INT));
    assert_eq!(op(SHL, 1.0, 32.0), Some(0.0));
    assert_eq!(op(SHL, 1.0, -1.0), Some(0.0));
    assert_eq!(op(SHR, -16.0, 2.0), Some(-4.0));
    assert_eq!(op(SHR, 16.0, 2.0), Some(4.0));
    assert_eq!(op(SHR, -16.0, 40.0), Some(-1.0));
    assert_eq!(op(SHR, 16.0, 40.0), Some(0.0));
    assert_eq!(op(SHR, -16.0, -1.0), Some(-1.0));
    // Both operands through `cvtps2dq` (to nearest), not truncated: 2.7 is 3, a count of 1.6 is 2.
    assert_eq!(op(LOR, 2.7, 0.0), Some(3.0));
    assert_eq!(op(SHL, 1.0, 1.6), Some(4.0));
    // `psrad`'s count is the whole low quadword, not masked to 5 bits: 33 shifts everything out
    // (33 & 31 would be 1).
    assert_eq!(op(SHR, 65536.0, 33.0), Some(0.0));
    assert_eq!(op(SHR, -65536.0, 33.0), Some(-1.0));
    // bt: truncated, the bit number masked to 0..31.
    assert_eq!(op(BT, 5.0, 0.0), Some(1.0));
    assert_eq!(op(BT, 5.0, 1.0), Some(0.0));
    assert_eq!(op(BT, 5.0, 2.0), Some(1.0));
    assert_eq!(op(BT, 5.0, 32.0), Some(1.0));
    assert_eq!(op(BT, 5.9, 1.9), Some(0.0));
    assert_eq!(op(BT, -1.0, 31.0), Some(1.0));
    assert_eq!(op(BT, f32::NAN, 31.0), Some(1.0));
    assert_eq!(op(BT, 4.0, -30.0), Some(1.0));
}

#[test]
fn sqrt_and_log_skip_operands_with_the_sign_or_not_above_zero() {
    assert_eq!(op(SQRT, 9.0, 16.0), Some(4.0));
    assert_eq!(op(SQRT, 9.0, 0.0), Some(0.0));
    assert_eq!(op(SQRT, 9.0, -4.0), None);
    assert_eq!(op(SQRT, 9.0, -0.0), None);
    assert_eq!(op(SQRT, 9.0, f32::INFINITY), Some(f32::INFINITY));
    assert_eq!(bits(SQRT, 1, SNAN, false), Some(0x7fc0_0001));
    assert_eq!(bits(SQRT, 1, QNAN_NEG, false), None);
    assert_eq!(op(LOG, 9.0, 1.0), Some(0.0));
    // The float nearest e is a little under e: its log rounds to the float under 1.
    assert_eq!(op(LOG, 9.0, std::f32::consts::E), Some(0.999_999_94));
    assert_eq!(op(LOG, 9.0, 100.0), Some((100.0f64).ln() as f32));
    assert_eq!(op(LOG, 9.0, 0.0), None);
    assert_eq!(op(LOG, 9.0, -1.0), None);
    assert_eq!(op(LOG, 9.0, f32::INFINITY), Some(f32::INFINITY));
    // `test ecx,ecx; jle` on the bits: a positive NaN runs (its log is a NaN), a negative one not.
    assert!(f32::from_bits(bits(LOG, 1, QNAN, false).unwrap()).is_nan());
    assert_eq!(bits(LOG, 1, QNAN_NEG, false), None);
}

#[test]
fn pow_and_the_trigonometry_compute_in_double_and_in_degrees() {
    assert_eq!(op(POW, 2.0, 10.0), Some(1024.0));
    assert_eq!(op(POW, 9.0, 0.5), Some(3.0));
    assert_eq!(op(POW, 0.0, 0.0), Some(1.0));
    assert_eq!(op(POW, 10.0, 39.0), Some(f32::INFINITY));
    assert!(op(POW, -8.0, 1.0 / 3.0).unwrap().is_nan());
    // The value is not read by sin .. atan.
    assert_eq!(op(SIN, 1234.0, 90.0), Some(1.0));
    assert_eq!(op(SIN, 0.0, 30.0), Some(0.5));
    assert_eq!(op(SIN, 0.0, -90.0), Some(-1.0));
    assert_eq!(op(COS, 0.0, 60.0), Some(0.5));
    assert_eq!(op(COS, 0.0, 0.0), Some(1.0));
    // cos 90 in double is 6.1e-17, not 0.
    assert_eq!(op(COS, 0.0, 90.0), Some((90.0f64 * DEGREE).cos() as f32));
    assert_ne!(op(COS, 0.0, 90.0), Some(0.0));
    assert_eq!(op(TAN, 0.0, 45.0), Some(1.0));
    assert_eq!(op(ASIN, 0.0, 1.0), Some(90.0));
    assert_eq!(op(ASIN, 0.0, -0.5), Some(-30.0));
    assert!(op(ASIN, 0.0, 2.0).unwrap().is_nan());
    assert_eq!(op(ACOS, 0.0, 1.0), Some(0.0));
    assert_eq!(op(ACOS, 0.0, -1.0), Some(180.0));
    assert_eq!(op(ATAN, 0.0, 1.0), Some(45.0));
    assert_eq!(op(ATAN, 0.0, f32::INFINITY), Some(90.0));
    assert!(op(SIN, 0.0, f32::INFINITY).unwrap().is_nan());
}

#[test]
fn rand_draws_below_the_truncated_operand_or_answers_zero() {
    let draw = |operand: f32, answer: u32| {
        let mut asked = vec![];
        let result = arithmetic(RAND, 9.0f32.to_bits(), operand.to_bits(), false, &mut |n| {
            asked.push(n);
            answer
        });
        (result.map(f32::from_bits), asked)
    };
    assert_eq!(draw(6.0, 4), (Some(4.0), vec![6]));
    assert_eq!(draw(6.9, 5), (Some(5.0), vec![6]));
    assert_eq!(draw(1.0, 0), (Some(0.0), vec![1]));
    // Not above 0 once truncated (0.9, negatives, NaN): the value becomes 0, no draw.
    for operand in [0.9, 0.0, -3.0, f32::NAN, -0.0] {
        assert_eq!(draw(operand, 7), (Some(0.0), vec![]), "{operand}");
    }
}

// The jump table's seam on an engine: the game's heap, strcmp and BSRandom stood in by doubles.

thread_local! {
    static FREED: RefCell<Vec<u32>> = const { RefCell::new(Vec::new()) };
    static DRAWS: RefCell<Vec<(u32, u32)>> = const { RefCell::new(Vec::new()) };
}

fn ret(v: u32) -> Ret {
    Ret {
        eax: v,
        ..Ret::default()
    }
}

fn engine() -> Engine {
    FREED.with(|f| f.borrow_mut().clear());
    DRAWS.with(|d| d.borrow_mut().clear());
    let mut e = Engine::new();
    // The game's heap hands out used blocks: filled with 0xCD here, so a copy must write its own
    // terminator (UIO's memcpy of StrLen + 1 bytes).
    e.register(HEAP_ALLOCATE, |e, a| {
        let size = a[1].max(1);
        let at = e.mem.alloc(size);
        e.mem.write(at, &vec![0xcd; size as usize]);
        ret(at)
    });
    e.register(HEAP_FREE, |_, a| {
        FREED.with(|f| f.borrow_mut().push(a[1]));
        ret(0)
    });
    e.register(STRCMP, |e, a| {
        let (x, y) = (
            nvse::table::guest_text(e, a[0]),
            nvse::table::guest_text(e, a[1]),
        );
        ret(x.cmp(&y) as i32 as u32)
    });
    e.register(RANDOM_UNSIGNED_INT, |_, a| {
        DRAWS.with(|d| d.borrow_mut().push((a[0], a[1])));
        ret(a[1] - 1)
    });
    e
}

fn text(e: &mut Engine, s: &str) -> u32 {
    let at = e.mem.alloc(s.len() as u32 + 1);
    e.mem.write(at, &[s.as_bytes(), b"\0"].concat());
    at
}

/// A `TileValue` with `num` and `str`.
fn value(e: &mut Engine, num: f32, string: u32) -> u32 {
    let v = e.mem.alloc(0x14);
    e.mem.set_f32(v + 8, num);
    e.mem.set_u32(v + 0xc, string);
    v
}

#[test]
fn the_seam_runs_the_operation_and_answers_the_changed_flag() {
    let mut e = engine();
    let v = value(&mut e, 10.0, 0);
    // add: the float written, the flag passed through.
    assert_eq!(
        operate(&mut e, &[v, ADD as u32, 5.0f32.to_bits(), 0, 1]),
        Some(1)
    );
    assert_eq!(e.mem.f32(v + 8), 15.0);
    assert_eq!(
        operate(&mut e, &[v, ADD as u32, 5.0f32.to_bits(), 0, 0]),
        Some(0)
    );
    assert_eq!(e.mem.f32(v + 8), 20.0);
    // Outside the table and the parentheses: nothing, the flag kept.
    for kind in [0x7e8u32, 0x7fb, 0x7cf] {
        assert_eq!(operate(&mut e, &[v, kind, 5.0f32.to_bits(), 0, 1]), Some(1));
        assert_eq!(e.mem.f32(v + 8), 20.0);
    }
    // rand: BSRandom::UnsignedInt on the static generator.
    assert_eq!(
        operate(&mut e, &[v, RAND as u32, 6.0f32.to_bits(), 0, 0]),
        Some(0)
    );
    assert_eq!(e.mem.f32(v + 8), 5.0);
    assert_eq!(DRAWS.with(|d| d.borrow().clone()), vec![(0x011c_4180, 6)]);
}

#[test]
fn copy_takes_the_operand_or_copies_a_new_text() {
    let mut e = engine();
    // No string: the operand's bits, the flag untouched.
    let v = value(&mut e, 1.0, 0);
    assert_eq!(operate(&mut e, &[v, COPY as u32, QNAN_NEG, 0, 1]), Some(1));
    assert_eq!(e.mem.u32(v + 8), QNAN_NEG);
    assert_eq!(
        operate(&mut e, &[v, COPY as u32, 7.0f32.to_bits(), 0, 0]),
        Some(0)
    );
    assert_eq!(e.mem.f32(v + 8), 7.0);
    // A string into a value without one: a copy on the heap, changed; the float kept.
    let hello = text(&mut e, "hello");
    assert_eq!(operate(&mut e, &[v, COPY as u32, 0, hello, 0]), Some(1));
    let copy = e.mem.u32(v + 0xc);
    assert_ne!(copy, hello);
    assert_eq!(nvse::table::guest_text(&e, copy), b"hello");
    assert_eq!(e.mem.f32(v + 8), 7.0);
    // The same text again: not changed, nothing freed or copied.
    let again = text(&mut e, "hello");
    assert_eq!(operate(&mut e, &[v, COPY as u32, 0, again, 1]), Some(0));
    assert_eq!(e.mem.u32(v + 0xc), copy);
    assert!(FREED.with(|f| f.borrow().is_empty()));
    // Another text: the old one freed, a new copy, changed.
    let world = text(&mut e, "world");
    assert_eq!(operate(&mut e, &[v, COPY as u32, 0, world, 0]), Some(1));
    let second = e.mem.u32(v + 0xc);
    assert_eq!(nvse::table::guest_text(&e, second), b"world");
    assert_eq!(FREED.with(|f| f.borrow().clone()), vec![copy]);
    // An empty text is a text.
    let empty = text(&mut e, "");
    assert_eq!(operate(&mut e, &[v, COPY as u32, 0, empty, 0]), Some(1));
    assert_eq!(e.mem.u8(e.mem.u32(v + 0xc)), 0);
}

/// A string operand of 2 MiB (a menu file's text may be 64 MiB): copied on the game's heap,
/// never onto its stack, so the game goes on.
#[test]
fn copy_of_a_two_megabyte_text_never_stops_the_game() {
    let mut e = engine();
    let v = value(&mut e, 1.0, 0);
    let long = "w".repeat(2 << 20);
    let at = text(&mut e, &long);
    assert_eq!(operate(&mut e, &[v, COPY as u32, 0, at, 0]), Some(1));
    let copy = e.mem.u32(v + 0xc);
    assert_eq!(nvse::table::guest_text(&e, copy).len(), 2 << 20);
    let again = text(&mut e, &long);
    assert_eq!(operate(&mut e, &[v, COPY as u32, 0, again, 1]), Some(0));
    // The logic operators (`and` 0x7E2, `or` 0x7E3) only look at whether there is a string.
    for kind in [0x7e2u32, 0x7e3] {
        assert!(operate(&mut e, &[v, kind, 1.0f32.to_bits(), at, 0]).is_some());
    }
}
