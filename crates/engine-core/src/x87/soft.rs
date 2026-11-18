//! The x87 instructions in software, for a host without an x87 (Apple silicon, ARM Linux), and on
//! x86-64 with the `soft-x87` feature (so that this PC can test it against its own FPU).
//!
//! What is exact, bit for bit with the FPU (integer arithmetic on the 64-bit significand, the
//! extended exponent range, the control word's precision and rounding): loads, stores to double
//! and float, `FADD`, `FSUB`, `FMUL`, `FDIV`, `FSQRT` (rounded once to the precision control and
//! rounding mode), `FCOM`, `FRNDINT`, `FISTP`, `FSCALE`, the loaded constants, and `FPREM` and
//! `FPREM1` (the remainder, the quotient's low bits and C2), with the x87's choice of NaN and its
//! negative "indefinite" for an invalid operation.
//!
//! What is not (the last bits only): `FSIN`, `FCOS`, `FSINCOS`, `FPTAN`, `FPATAN`, `F2XM1`,
//! `FYL2X` and `FYL2XP1` compute with the host's `f64` functions, so their 64-bit results are a
//! 53-bit value, within about one unit in the last place of a double of the FPU's. Their
//! special cases are the FPU's: the reduction of `FSIN`/`FCOS`/`FPTAN` arguments uses the FPU's
//! own 66-bit pi (so `sin(pi)` is the well-known x87 answer), an operand at or above 2^63 is left
//! alone (C2) as the FPU leaves it, and invalid operands give the indefinite.

use super::{
    control_word, X87Constant, X87Ext, PRECISION_CONTROL, ROUNDING_CONTROL, ROUND_DOWN,
    ROUND_NEAREST, ROUND_UP, X87_STATUS_C0, X87_STATUS_C2, X87_STATUS_C3,
};

const BIAS: i32 = 16383;
const INTEGER_BIT: u64 = 1 << 63;
const QUIET_BIT: u64 = 1 << 62;
/// Status word bit C1 (the lowest quotient bit of `FPREM`).
const STATUS_C1: u16 = 0x0200;
/// The status word's stack top after the two loads of the remainder helpers (`FNINIT`, two
/// `FLD`s: TOP = 6).
const TOP_AFTER_TWO_LOADS: u16 = 6 << 11;
/// Status word bit IE (invalid operation) and DE (denormal operand).
const STATUS_IE: u16 = 0x0001;
const STATUS_DE: u16 = 0x0002;

/// A register's value taken apart.
#[derive(Clone, Copy, Debug)]
enum Kind {
    Zero,
    /// `sig * 2^(e - BIAS - 63)` with bit 63 of `sig` set (a denormal is normalised here, so
    /// `e` may be below 1).
    Finite {
        e: i32,
        sig: u64,
    },
    Inf,
    Nan,
    /// An encoding the FPU does not support (an unnormal, a pseudo-NaN or pseudo-infinity): an
    /// operand that is one is an invalid operation.
    Unsupported,
}

#[derive(Clone, Copy, Debug)]
struct Val {
    neg: bool,
    kind: Kind,
    /// The operand was a denormal (DE).
    denormal: bool,
}

fn unpack(x: X87Ext) -> Val {
    let se = x.sign_exponent();
    let neg = se & 0x8000 != 0;
    let exponent = i32::from(se & 0x7fff);
    let sig = x.significand();
    let mut denormal = false;
    let kind = match exponent {
        0x7fff if sig & INTEGER_BIT == 0 => Kind::Unsupported,
        0x7fff if sig << 1 == 0 => Kind::Inf,
        0x7fff => Kind::Nan,
        0 if sig == 0 => Kind::Zero,
        0 => {
            denormal = true;
            let lz = sig.leading_zeros() as i32;
            Kind::Finite {
                e: 1 - lz,
                sig: sig << lz,
            }
        }
        _ if sig & INTEGER_BIT == 0 => Kind::Unsupported,
        _ => Kind::Finite { e: exponent, sig },
    };
    Val {
        neg,
        kind,
        denormal,
    }
}

fn sign_word(neg: bool) -> u16 {
    if neg {
        0x8000
    } else {
        0
    }
}

fn zero(neg: bool) -> X87Ext {
    X87Ext::from_parts(sign_word(neg), 0)
}

fn infinity(neg: bool) -> X87Ext {
    X87Ext::from_parts(sign_word(neg) | 0x7fff, INTEGER_BIT)
}

/// The real indefinite: the NaN the FPU makes for an invalid operation (negative, quiet, no
/// payload).
pub(super) fn indefinite() -> X87Ext {
    X87Ext::from_parts(0xffff, INTEGER_BIT | QUIET_BIT)
}

fn quieted(x: X87Ext) -> X87Ext {
    X87Ext::from_parts(x.sign_exponent(), x.significand() | QUIET_BIT)
}

/// The NaN of an operation on `a` and `b` when either is a NaN (Intel SDM vol. 1, table 4-7, x87
/// column): one NaN, that one quieted; two, a quiet one over a signalling one, otherwise the larger
/// significand, and on a tie the positive one. The indefinite when an operand is unsupported.
/// `None` when neither is a NaN or unsupported.
fn nan_of_two(a: X87Ext, ua: Val, b: X87Ext, ub: Val) -> Option<X87Ext> {
    if matches!(ua.kind, Kind::Unsupported) || matches!(ub.kind, Kind::Unsupported) {
        return Some(indefinite());
    }
    match (matches!(ua.kind, Kind::Nan), matches!(ub.kind, Kind::Nan)) {
        (false, false) => None,
        (true, false) => Some(quieted(a)),
        (false, true) => Some(quieted(b)),
        (true, true) => {
            let (qa, qb) = (a.significand() & QUIET_BIT, b.significand() & QUIET_BIT);
            let chosen = if qa != qb {
                if qa != 0 {
                    a
                } else {
                    b
                }
            } else {
                let (sa, sb) = (a.significand() | QUIET_BIT, b.significand() | QUIET_BIT);
                match sa.cmp(&sb) {
                    std::cmp::Ordering::Greater => a,
                    std::cmp::Ordering::Less => b,
                    std::cmp::Ordering::Equal if ua.neg => b,
                    std::cmp::Ordering::Equal => a,
                }
            };
            Some(quieted(chosen))
        }
    }
}

/// The NaN of an operation on one operand: a NaN quieted, the indefinite for an unsupported one.
fn nan_of_one(x: X87Ext, u: Val) -> Option<X87Ext> {
    match u.kind {
        Kind::Nan => Some(quieted(x)),
        Kind::Unsupported => Some(indefinite()),
        _ => None,
    }
}

/// The significand bits the precision control keeps.
fn precision(word: u16) -> u32 {
    match word & PRECISION_CONTROL {
        0x0000 => 24,
        0x0200 => 53,
        _ => 64,
    }
}

/// Whether a value of sign `neg`, whose kept bits end in `odd` and whose dropped bits are `rest`
/// (half a unit of the kept part: `half`), rounds away from zero under the rounding mode `rc`.
fn rounds_up(rc: u16, neg: bool, odd: bool, rest: u128, half: u128) -> bool {
    match rc {
        ROUND_NEAREST => rest > half || (rest == half && odd),
        ROUND_DOWN => neg && rest != 0,
        ROUND_UP => !neg && rest != 0,
        _ => false,
    }
}

/// `v >> s` with every bit shifted out folded into bit 0 (sticky).
fn shift_right_sticky(v: u128, s: u32) -> u128 {
    if s == 0 {
        v
    } else if s >= 128 {
        u128::from(v != 0)
    } else {
        (v >> s) | u128::from(v & ((1u128 << s) - 1) != 0)
    }
}

/// `q * 2^exp0` (`q` not zero) as `(e, sig)` for [`round_pack`]: `sig` with bit 127 set.
fn normalize(q: u128, exp0: i32) -> (i32, u128) {
    let top = 127 - q.leading_zeros();
    (exp0 + top as i32 + BIAS, q << (127 - top))
}

/// The largest finite value at precision `p`, or the infinity, as an overflow rounds.
fn overflow(neg: bool, p: u32, rc: u16) -> X87Ext {
    let to_infinity = match rc {
        ROUND_NEAREST => true,
        ROUND_DOWN => neg,
        ROUND_UP => !neg,
        _ => false,
    };
    if to_infinity {
        infinity(neg)
    } else {
        X87Ext::from_parts(sign_word(neg) | 0x7ffe, u64::MAX << (64 - p))
    }
}

/// The value `sig * 2^(e - BIAS - 127)` (bit 127 of `sig` set; bits below the exact value folded
/// into bit 0) rounded once to `p` significand bits with the rounding mode `rc`, in the extended
/// exponent range (below it, a denormal: the bits are shifted out before the rounding).
fn round_pack(neg: bool, mut e: i32, mut sig: u128, p: u32, rc: u16) -> X87Ext {
    if e < 1 {
        sig = shift_right_sticky(sig, (1 - e).min(200) as u32);
        e = 1;
    }
    let drop = 128 - p;
    let mut kept = sig >> drop;
    let rest = sig & ((1u128 << drop) - 1);
    if rounds_up(rc, neg, kept & 1 != 0, rest, 1u128 << (drop - 1)) {
        kept += 1;
        if kept >> p != 0 {
            kept >>= 1;
            e += 1;
        }
    }
    if e >= 0x7fff {
        return overflow(neg, p, rc);
    }
    let sig64 = (kept << (64 - p)) as u64;
    let field = if sig64 & INTEGER_BIT == 0 { 0 } else { e };
    X87Ext::from_parts(sign_word(neg) | field as u16, sig64)
}

fn rounding() -> u16 {
    control_word() & ROUNDING_CONTROL
}

// ---- Loads and stores ------------------------------------------------------------------------

pub fn load64(value: f64) -> X87Ext {
    let bits = value.to_bits();
    let sign = ((bits >> 63) as u16) << 15;
    let exponent = ((bits >> 52) & 0x7ff) as i32;
    let fraction = bits & 0x000f_ffff_ffff_ffff;
    match exponent {
        0 if fraction == 0 => X87Ext::from_parts(sign, 0),
        0 => {
            // A double denormal is a normal extended value.
            let shift = fraction.leading_zeros();
            let e = -1022 - (shift as i32 - 11) + BIAS;
            X87Ext::from_parts(sign | e as u16, fraction << shift)
        }
        0x7ff if fraction == 0 => X87Ext::from_parts(sign | 0x7fff, INTEGER_BIT),
        // A NaN, quieted (bit 62 of the significand), payload kept.
        0x7ff => X87Ext::from_parts(sign | 0x7fff, INTEGER_BIT | QUIET_BIT | (fraction << 11)),
        _ => X87Ext::from_parts(
            sign | (exponent - 1023 + BIAS) as u16,
            INTEGER_BIT | (fraction << 11),
        ),
    }
}

pub fn load32(bits: u32) -> X87Ext {
    // Widened exactly with the payload kept, then quieted by `load64`.
    load64(super::x87_m32(bits))
}

/// `x` rounded to a binary format of `frac` fraction bits and `width` exponent bits with the
/// rounding mode `rc`, as its bits (`FSTP qword`/`FSTP dword`).
fn store_bits(x: X87Ext, frac: u32, width: u32, rc: u16) -> u64 {
    let u = unpack(x);
    let exp_max = (1u64 << width) - 1;
    let bias = (1i32 << (width - 1)) - 1;
    let sign = u64::from(u.neg) << (frac + width);
    let infinite = exp_max << frac;
    match u.kind {
        Kind::Zero => sign,
        Kind::Inf => sign | infinite,
        Kind::Nan => sign | infinite | (1 << (frac - 1)) | ((x.significand() << 1) >> (64 - frac)),
        Kind::Unsupported => (1 << (frac + width)) | infinite | (1 << (frac - 1)),
        Kind::Finite { e, sig } => {
            let biased = e - BIAS + bias;
            if biased >= exp_max as i32 {
                return sign | overflow_bits(u.neg, frac, exp_max, rc);
            }
            let drop = if biased >= 1 {
                63 - frac
            } else {
                (63 - frac as i32 + 1 - biased).min(127) as u32
            };
            let wide = u128::from(sig);
            let mut kept = (wide >> drop) as u64;
            let rest = wide & ((1u128 << drop) - 1);
            if rounds_up(rc, u.neg, kept & 1 != 0, rest, 1u128 << (drop - 1)) {
                kept += 1;
            }
            // A normal value's kept bits carry the integer bit, which adds one to the exponent
            // field: a carry out of the significand moves on into the exponent by itself.
            let bits = if biased >= 1 {
                ((biased as u64 - 1) << frac) + kept
            } else {
                kept
            };
            if bits >> frac >= exp_max {
                sign | overflow_bits(u.neg, frac, exp_max, rc)
            } else {
                sign | bits
            }
        }
    }
}

/// [`store_bits`]'s overflow: the infinity or the largest finite value, as the rounding mode says.
fn overflow_bits(neg: bool, frac: u32, exp_max: u64, rc: u16) -> u64 {
    let to_infinity = match rc {
        ROUND_NEAREST => true,
        ROUND_DOWN => neg,
        ROUND_UP => !neg,
        _ => false,
    };
    if to_infinity {
        exp_max << frac
    } else {
        ((exp_max - 1) << frac) | ((1 << frac) - 1)
    }
}

pub fn store64(x: X87Ext) -> f64 {
    f64::from_bits(store_bits(x, 52, 11, rounding()))
}

pub fn store32(x: X87Ext) -> u32 {
    store_bits(x, 23, 8, rounding()) as u32
}

/// `x` as the nearest double, whatever the control word says (the transcendentals' input).
fn nearest64(x: X87Ext) -> f64 {
    f64::from_bits(store_bits(x, 52, 11, ROUND_NEAREST))
}

// ---- Arithmetic ------------------------------------------------------------------------------

/// `a + b` (or `a - b`: `negate_b`) rounded to `p` bits with `rc`.
fn add(a: X87Ext, b: X87Ext, negate_b: bool, p: u32, rc: u16) -> X87Ext {
    let (ua, mut ub) = (unpack(a), unpack(b));
    if let Some(nan) = nan_of_two(a, ua, b, ub) {
        return nan;
    }
    ub.neg ^= negate_b;
    match (ua.kind, ub.kind) {
        (Kind::Inf, Kind::Inf) if ua.neg != ub.neg => indefinite(),
        (Kind::Inf, _) => infinity(ua.neg),
        (_, Kind::Inf) => infinity(ub.neg),
        (Kind::Zero, Kind::Zero) => zero(if ua.neg == ub.neg {
            ua.neg
        } else {
            rc == ROUND_DOWN
        }),
        (Kind::Zero, Kind::Finite { e, sig }) => {
            round_pack(ub.neg, e, u128::from(sig) << 64, p, rc)
        }
        (Kind::Finite { e, sig }, Kind::Zero) => {
            round_pack(ua.neg, e, u128::from(sig) << 64, p, rc)
        }
        (Kind::Finite { e: ea, sig: sa }, Kind::Finite { e: eb, sig: sb }) => {
            let ((big_neg, big_e, big_sig), (small_neg, small_e, small_sig)) =
                if (ea, sa) >= (eb, sb) {
                    ((ua.neg, ea, sa), (ub.neg, eb, sb))
                } else {
                    ((ub.neg, eb, sb), (ua.neg, ea, sa))
                };
            let x = u128::from(big_sig) << 63;
            let y = shift_right_sticky(u128::from(small_sig) << 63, (big_e - small_e) as u32);
            let sum = if big_neg == small_neg { x + y } else { x - y };
            if sum == 0 {
                return zero(rc == ROUND_DOWN);
            }
            let (e, sig) = normalize(sum, big_e - BIAS - 126);
            round_pack(big_neg, e, sig, p, rc)
        }
        _ => indefinite(),
    }
}

pub fn fadd(a: X87Ext, b: X87Ext) -> X87Ext {
    let word = control_word();
    add(a, b, false, precision(word), word & ROUNDING_CONTROL)
}

pub fn fsub(a: X87Ext, b: X87Ext) -> X87Ext {
    let word = control_word();
    add(a, b, true, precision(word), word & ROUNDING_CONTROL)
}

/// `a * b` rounded to `p` bits with `rc` (`FMUL` with the control word's; the transcendentals'
/// last multiplication at 64 bits).
fn multiply(a: X87Ext, b: X87Ext, p: u32, rc: u16) -> X87Ext {
    let (ua, ub) = (unpack(a), unpack(b));
    if let Some(nan) = nan_of_two(a, ua, b, ub) {
        return nan;
    }
    let neg = ua.neg != ub.neg;
    match (ua.kind, ub.kind) {
        (Kind::Inf, Kind::Zero) | (Kind::Zero, Kind::Inf) => indefinite(),
        (Kind::Inf, _) | (_, Kind::Inf) => infinity(neg),
        (Kind::Zero, _) | (_, Kind::Zero) => zero(neg),
        (Kind::Finite { e: ea, sig: sa }, Kind::Finite { e: eb, sig: sb }) => {
            let (e, sig) = normalize(
                u128::from(sa) * u128::from(sb),
                (ea - BIAS - 63) + (eb - BIAS - 63),
            );
            round_pack(neg, e, sig, p, rc)
        }
        _ => indefinite(),
    }
}

pub fn fmul(a: X87Ext, b: X87Ext) -> X87Ext {
    let word = control_word();
    multiply(a, b, precision(word), word & ROUNDING_CONTROL)
}

pub fn fdiv(a: X87Ext, b: X87Ext) -> X87Ext {
    let word = control_word();
    divide(a, b, precision(word), word & ROUNDING_CONTROL)
}

/// `a / b` rounded to `p` bits with `rc`.
fn divide(a: X87Ext, b: X87Ext, p: u32, rc: u16) -> X87Ext {
    let (ua, ub) = (unpack(a), unpack(b));
    if let Some(nan) = nan_of_two(a, ua, b, ub) {
        return nan;
    }
    let neg = ua.neg != ub.neg;
    match (ua.kind, ub.kind) {
        (Kind::Inf, Kind::Inf) | (Kind::Zero, Kind::Zero) => indefinite(),
        (Kind::Inf, _) | (_, Kind::Zero) => infinity(neg),
        (_, Kind::Inf) | (Kind::Zero, _) => zero(neg),
        (Kind::Finite { e: ea, sig: sa }, Kind::Finite { e: eb, sig: sb }) => {
            // Two more quotient bits after the first 64 or 65, and whatever is left in bit 0.
            let divisor = u128::from(sb);
            let numerator = u128::from(sa) << 64;
            let (q1, r1) = (numerator / divisor, numerator % divisor);
            let (q2, r2) = ((r1 << 2) / divisor, (r1 << 2) % divisor);
            let q = (q1 << 2) | q2 | u128::from(r2 != 0);
            let (e, sig) = normalize(q, ea - eb - 66);
            round_pack(neg, e, sig, p, rc)
        }
        _ => indefinite(),
    }
}

/// The integer square root of `n` (the largest `r` with `r * r <= n`).
fn isqrt(mut n: u128) -> u128 {
    let mut root = 0u128;
    let mut bit = 1u128 << 126;
    while bit > n {
        bit >>= 2;
    }
    while bit != 0 {
        if n >= root + bit {
            n -= root + bit;
            root = (root >> 1) + bit;
        } else {
            root >>= 1;
        }
        bit >>= 2;
    }
    root
}

pub fn fsqrt(a: X87Ext) -> X87Ext {
    let u = unpack(a);
    if let Some(nan) = nan_of_one(a, u) {
        return nan;
    }
    match u.kind {
        Kind::Zero => a,
        _ if u.neg => indefinite(),
        Kind::Inf => a,
        Kind::Finite { e, sig } => {
            // `sig * 2^exponent` with an even exponent, as N * 2^(exponent - 62).
            let mut exponent = e - BIAS - 63;
            let mut m = u128::from(sig);
            if exponent & 1 != 0 {
                m <<= 1;
                exponent -= 1;
            }
            let n = m << 62;
            let r0 = isqrt(n);
            let rem = n - r0 * r0;
            // Two more root bits: the largest d in 0..=3 with (4 r0 + d)^2 <= 16 n.
            let d = (0..4u128)
                .rev()
                .find(|&d| 8 * r0 * d + d * d <= 16 * rem)
                .unwrap_or(0);
            let rem2 = 16 * rem - 8 * r0 * d - d * d;
            let q = ((4 * r0 + d) << 1) | u128::from(rem2 != 0);
            let word = control_word();
            let (e, sig) = normalize(q, (exponent - 62) / 2 - 3);
            round_pack(false, e, sig, precision(word), word & ROUNDING_CONTROL)
        }
        _ => indefinite(),
    }
}

/// Orders two finite or infinite values (not NaNs): -1, 0 or 1.
fn order(ua: Val, ub: Val) -> i32 {
    // The magnitude as a key: zero, finite by (exponent, significand), infinity.
    let key = |u: Val| match u.kind {
        Kind::Zero => (0, 0i64, 0u64),
        Kind::Finite { e, sig } => (1, i64::from(e), sig),
        _ => (2, 0, 0),
    };
    let (ka, kb) = (key(ua), key(ub));
    if ka.0 == 0 && kb.0 == 0 {
        return 0;
    }
    let magnitude = ka.cmp(&kb) as i32;
    match (ua.neg, ub.neg) {
        (false, false) => magnitude,
        (true, true) => -magnitude,
        (false, true) => 1,
        (true, false) => -1,
    }
}

pub fn fcom(a: X87Ext, b: X87Ext) -> u16 {
    let (ua, ub) = (unpack(a), unpack(b));
    let unordered = |u: Val| matches!(u.kind, Kind::Nan | Kind::Unsupported);
    if unordered(ua) || unordered(ub) {
        return X87_STATUS_C0 | X87_STATUS_C2 | X87_STATUS_C3;
    }
    match order(ua, ub) {
        1 => 0,
        -1 => X87_STATUS_C0,
        _ => X87_STATUS_C3,
    }
}

// ---- Constants, rounding to integers, scaling, remainders ----------------------------------

pub fn fld_constant(constant: X87Constant) -> X87Ext {
    // The FPU's constants to 64 bits, rounded toward zero, and whether the bits after them round
    // to nearest upward (all five are irrational, so rounding up always adds one).
    let (sign_exponent, truncated, nearest_up) = match constant {
        X87Constant::Pi => (0x4000, 0xc90f_daa2_2168_c234u64, true),
        X87Constant::Log2E => (0x3fff, 0xb8aa_3b29_5c17_f0bb, true),
        X87Constant::Log2Ten => (0x4000, 0xd49a_784b_cd1b_8afe, false),
        X87Constant::Log10Two => (0x3ffd, 0x9a20_9a84_fbcf_f798, true),
        X87Constant::LnTwo => (0x3ffe, 0xb172_17f7_d1cf_79ab, true),
    };
    let up = match rounding() {
        ROUND_NEAREST => nearest_up,
        ROUND_UP => true,
        _ => false,
    };
    X87Ext::from_parts(sign_exponent, truncated + u64::from(up))
}

/// `|x|` rounded to an integer with `rc` (`x = sig * 2^(e - BIAS - 63)`, below 2^64), as an
/// integer.
fn integer_magnitude(neg: bool, e: i32, sig: u64, rc: u16) -> u128 {
    let below = BIAS + 63 - e;
    if below <= 0 {
        return u128::from(sig) << (-below).min(64) as u32;
    }
    let s = below.min(127) as u32;
    let wide = u128::from(sig);
    let kept = wide >> s;
    let rest = wide & ((1u128 << s) - 1);
    kept + u128::from(rounds_up(rc, neg, kept & 1 != 0, rest, 1u128 << (s - 1)))
}

pub fn frndint(x: X87Ext) -> X87Ext {
    let u = unpack(x);
    if let Some(nan) = nan_of_one(x, u) {
        return nan;
    }
    match u.kind {
        Kind::Finite { e, sig } if e < BIAS + 63 => {
            let n = integer_magnitude(u.neg, e, sig, rounding());
            if n == 0 {
                return zero(u.neg);
            }
            let (e, sig) = normalize(n, 0);
            round_pack(u.neg, e, sig, 64, ROUND_NEAREST)
        }
        _ => x,
    }
}

pub fn fistp64(x: X87Ext, word: u16) -> i64 {
    let u = unpack(x);
    match u.kind {
        Kind::Zero => 0,
        Kind::Finite { e, sig } if e - BIAS <= 63 => {
            let n = integer_magnitude(u.neg, e, sig, word & ROUNDING_CONTROL);
            let limit = 1u128 << 63;
            if u.neg {
                if n > limit {
                    i64::MIN
                } else {
                    (n as i128).wrapping_neg() as i64
                }
            } else if n >= limit {
                i64::MIN
            } else {
                n as i64
            }
        }
        _ => i64::MIN,
    }
}

pub fn fscale(x: X87Ext, scale: X87Ext) -> X87Ext {
    let (ux, us) = (unpack(x), unpack(scale));
    if let Some(nan) = nan_of_two(x, ux, scale, us) {
        return nan;
    }
    match (ux.kind, us.kind) {
        (Kind::Zero, Kind::Inf) if !us.neg => indefinite(),
        (Kind::Inf, Kind::Inf) if us.neg => indefinite(),
        (Kind::Finite { .. }, Kind::Inf) => {
            if us.neg {
                zero(ux.neg)
            } else {
                infinity(ux.neg)
            }
        }
        (Kind::Finite { e, sig }, Kind::Finite { e: es, sig: ss }) => {
            // trunc(scale), large ones clamped (anything past 2^17 overflows or underflows).
            let n = if es - BIAS >= 20 {
                1 << 20
            } else {
                integer_magnitude(false, es, ss, ROUNDING_CONTROL) as i32
            };
            let n = if us.neg { -n } else { n };
            round_pack(ux.neg, e + n, u128::from(sig) << 64, 64, rounding())
        }
        _ => x,
    }
}

/// `FPREM` (`nearest` false: the quotient truncated) and `FPREM1` (`nearest`: rounded to nearest,
/// ties to even), with the status word `FNSTSW` reads after `FNINIT` and the two loads.
fn remainder(x: X87Ext, y: X87Ext, nearest: bool) -> (X87Ext, u16) {
    let (ux, uy) = (unpack(x), unpack(y));
    let mut status = TOP_AFTER_TWO_LOADS;
    if let Some(nan) = nan_of_two(x, ux, y, uy) {
        let signalling = |u: Val, v: X87Ext| {
            matches!(u.kind, Kind::Nan) && v.significand() & QUIET_BIT == 0
                || matches!(u.kind, Kind::Unsupported)
        };
        if signalling(ux, x) || signalling(uy, y) {
            status |= STATUS_IE;
        }
        return (nan, status);
    }
    if matches!(ux.kind, Kind::Inf) || matches!(uy.kind, Kind::Zero) {
        return (indefinite(), status | STATUS_IE);
    }
    if ux.denormal || uy.denormal {
        status |= STATUS_DE;
    }
    let (ex, sx, ey, sy) = match (ux.kind, uy.kind) {
        (Kind::Zero, _) | (_, Kind::Inf) => return (x, status),
        (Kind::Finite { e: ex, sig: sx }, Kind::Finite { e: ey, sig: sy }) => (ex, sx, ey, sy),
        _ => return (indefinite(), status | STATUS_IE),
    };
    let d = ex - ey;
    if d >= 64 {
        // Incomplete: the exponent difference brought down by between 32 and 63 (this
        // implementation: to 32 below what remains, as the FPUs measured do).
        let n = 32 + (d - 64) % 32;
        let n = n.clamp(32, 63);
        let numerator = u128::from(sx) << n;
        let r = numerator % u128::from(sy);
        let result = if r == 0 {
            zero(ux.neg)
        } else {
            let (e, sig) = normalize(r, ex - n - BIAS - 63);
            round_pack(ux.neg, e, sig, 64, ROUND_NEAREST)
        };
        return (result, status | X87_STATUS_C2);
    }
    // The remainder in units of 2^(min(ex, ey) - BIAS - 63): x = sx << max(d, 0) of them.
    let (numerator, divisor, unit_e) = if d >= 0 {
        (u128::from(sx) << d, u128::from(sy), ey)
    } else if nearest && d == -1 {
        (u128::from(sx), u128::from(sy) << 1, ex)
    } else {
        return (x, status);
    };
    let mut q = numerator / divisor;
    let mut r = numerator % divisor;
    let mut neg = ux.neg;
    if nearest && (2 * r > divisor || (2 * r == divisor && q & 1 != 0)) {
        r = divisor - r;
        q += 1;
        neg = !neg;
    }
    let result = if r == 0 {
        zero(ux.neg)
    } else {
        let (e, sig) = normalize(r, unit_e - BIAS - 63);
        round_pack(neg, e, sig, 64, ROUND_NEAREST)
    };
    let mut cc = 0;
    if q & 4 != 0 {
        cc |= X87_STATUS_C0;
    }
    if q & 2 != 0 {
        cc |= X87_STATUS_C3;
    }
    if q & 1 != 0 {
        cc |= STATUS_C1;
    }
    (result, status | cc)
}

pub fn fprem(x: X87Ext, y: X87Ext) -> (X87Ext, u16) {
    remainder(x, y, false)
}

pub fn fprem1(x: X87Ext, y: X87Ext) -> (X87Ext, u16) {
    remainder(x, y, true)
}

// ---- Transcendentals (the last bits are the host library's) ---------------------------------

/// `|x|` (finite, below 2^63) reduced by the FPU's pi/2: `(r, k)` with `|x| = k * pi/2 + r`,
/// `|r| <= pi/4`.
fn reduce(e: i32, sig: u64) -> (f64, u64) {
    let value = |e: i32, sig: u64| (sig as f64) * 2f64.powi(e - BIAS - 63);
    // pi/4 = 0.785..., below it nothing is reduced (2^-1 * 1.57...).
    if e < BIAS - 1 || (e == BIAS - 1 && sig < 0xc90f_daa2_2168_c234) {
        return (value(e, sig), 0);
    }
    let modulus = pi66_bits();
    // |x| / (pi/2) = sig * 2^(e - BIAS - 63 + 67) / M, by long division bit by bit.
    let shift = (e - BIAS - 63 + 67) as u32;
    let total = 64 + shift;
    let mut rem: u128 = 0;
    let mut q: u64 = 0;
    for i in (0..total).rev() {
        let bit = if i >= shift {
            (sig >> (i - shift)) & 1
        } else {
            0
        };
        rem = (rem << 1) | u128::from(bit);
        q <<= 1;
        if rem >= modulus {
            rem -= modulus;
            q |= 1;
        }
    }
    let scale = 2f64.powi(-67);
    if 2 * rem > modulus {
        (-((modulus - rem) as f64) * scale, q.wrapping_add(1))
    } else {
        ((rem as f64) * scale, q)
    }
}

/// The FPU's pi for argument reduction (66 significant bits, Intel SDM vol. 1, 8.3.10) as an
/// integer M with pi = M * 2^-66, so pi/2 = M * 2^-67.
fn pi66_bits() -> u128 {
    0xc_90fd_aa22_168c_234c_u128
}

/// Which instruction [`circular`] computes.
#[derive(Clone, Copy)]
enum Circular {
    Sin,
    Cos,
    Tan,
}

/// `FSIN`, `FCOS` and `FPTAN`'s result for a finite `|x|` below 2^63 (`u` its parts).
fn circular(x: X87Ext, u: Val, e: i32, sig: u64, which: Circular) -> X87Ext {
    // Below 2^-40 the sine and tangent are the operand and the cosine 1, at 64 bits (and no
    // double could carry an operand below the double range).
    if e < BIAS - 40 {
        return match which {
            Circular::Cos => load64(1.0),
            _ => x,
        };
    }
    let (r, k) = reduce(e, sig);
    let quadrant = k & 3;
    let magnitude = match which {
        Circular::Sin => match quadrant {
            0 => r.sin(),
            1 => r.cos(),
            2 => -r.sin(),
            _ => -r.cos(),
        },
        Circular::Cos => match quadrant {
            0 => r.cos(),
            1 => -r.sin(),
            2 => -r.cos(),
            _ => r.sin(),
        },
        Circular::Tan => {
            if quadrant & 1 == 0 {
                r.tan()
            } else {
                -1.0 / r.tan()
            }
        }
    };
    // Sine and tangent are odd, cosine even.
    let result = if u.neg && !matches!(which, Circular::Cos) {
        -magnitude
    } else {
        magnitude
    };
    load64(result)
}

/// The part `FSIN`, `FCOS` and `FPTAN` share: the special operands, then [`circular`].
fn trig(x: X87Ext, which: Circular) -> Result<X87Ext, ()> {
    let u = unpack(x);
    if let Some(nan) = nan_of_one(x, u) {
        return Ok(nan);
    }
    match u.kind {
        Kind::Inf => Ok(indefinite()),
        Kind::Zero => Ok(match which {
            Circular::Cos => load64(1.0),
            _ => x,
        }),
        // Out of range: C2, the operand left as it is (nothing pushed for FPTAN).
        Kind::Finite { e, .. } if e - BIAS >= 63 => Err(()),
        Kind::Finite { e, sig } => Ok(circular(x, u, e, sig, which)),
        _ => Ok(indefinite()),
    }
}

pub fn fsin(x: X87Ext) -> X87Ext {
    trig(x, Circular::Sin).unwrap_or(x)
}

pub fn fcos(x: X87Ext) -> X87Ext {
    trig(x, Circular::Cos).unwrap_or(x)
}

pub fn fsincos(x: X87Ext) -> (X87Ext, X87Ext) {
    // Out of range (finite, |x| >= 2^63): the stores the FPU block makes (the hardware path's
    // `settle_unpushed`): the cosine's place the operand, the sine's the indefinite.
    match (trig(x, Circular::Sin), trig(x, Circular::Cos)) {
        (Ok(sine), Ok(cosine)) => (sine, cosine),
        _ => (indefinite(), x),
    }
}

pub fn fsincos32(bits: u32) -> (u32, u32) {
    let (sine, cosine) = fsincos(load32(bits));
    (store32(sine), store32(cosine))
}

pub fn fptan(x: X87Ext) -> X87Ext {
    match trig(x, Circular::Tan) {
        Ok(tangent) => tangent,
        Err(()) => {
            panic!("FPTAN of {x:?}: operand out of range (|x| >= 2^63), nothing pushed")
        }
    }
}

pub fn fpatan(y: X87Ext, x: X87Ext) -> X87Ext {
    let (uy, ux) = (unpack(y), unpack(x));
    if let Some(nan) = nan_of_two(y, uy, x, ux) {
        return nan;
    }
    match (uy.kind, ux.kind) {
        (Kind::Finite { e: ey, .. }, Kind::Finite { e: ex, .. }) => {
            if ey - ex < -70 && !ux.neg {
                // atan(y/x) is y/x to far below the last bit.
                return divide(y, x, 64, ROUND_NEAREST);
            }
            if ey - ex < -70 {
                // Just short of +-pi: pi to 64 bits.
                let pi = X87Ext::from_parts(0x4000 | sign_word(uy.neg), 0xc90f_daa2_2168_c235);
                return pi;
            }
            // Both brought near 1 by the same power of two: the quotient and its quadrant
            // stay, and values beyond the double range keep their ratio.
            let top = ey.max(ex);
            let near_one = |v: X87Ext, u: Val, e: i32| match u.kind {
                Kind::Finite { sig, .. } => {
                    let shifted = e - top + BIAS;
                    if shifted < 1 {
                        0.0
                    } else {
                        nearest64(X87Ext::from_parts(sign_word(u.neg) | shifted as u16, sig))
                    }
                }
                _ => nearest64(v),
            };
            load64(near_one(y, uy, ey).atan2(near_one(x, ux, ex)))
        }
        // A zero or an infinity with a finite value: only the finite value's sign counts.
        _ => {
            let v = |w: X87Ext, u: Val| match u.kind {
                Kind::Finite { .. } => {
                    if u.neg {
                        -1.0
                    } else {
                        1.0
                    }
                }
                _ => nearest64(w),
            };
            load64(v(y, uy).atan2(v(x, ux)))
        }
    }
}

pub fn f2xm1(x: X87Ext) -> X87Ext {
    let u = unpack(x);
    if let Some(nan) = nan_of_one(x, u) {
        return nan;
    }
    if let Kind::Zero = u.kind {
        return x;
    }
    let v = nearest64(x);
    let (e, sig) = match u.kind {
        // Far below 1: x ln 2 to below the last bit.
        Kind::Finite { e, .. } if e < BIAS - 70 => {
            return multiply(x, ext_of(LN_2), 64, ROUND_NEAREST)
        }
        Kind::Finite { e, sig } if v.fract() != 0.0 => (e, sig),
        // The integers (-1 and 1, the ends of the range, and those out of range) and the
        // infinities: exact, as the double function gives them.
        _ => return load64(v.exp2() - 1.0),
    };
    // t = x ln 2 and e^t - 1 by its series, both to about 100 bits (|t| < 0.7: 30 terms
    // at most), so the host library's last bits play no part.
    let mut xd = dd_of(sig, e - BIAS - 63);
    if u.neg {
        xd = dd_neg(xd);
    }
    let t = dd_mul(xd, LN_2);
    let mut term = t;
    let mut sum = t;
    for k in 2..40 {
        term = dd_div(dd_mul(term, t), Dd(f64::from(k), 0.0));
        sum = dd_add(sum, term);
        if term.0.abs() <= 1e-34 * sum.0.abs() {
            break;
        }
    }
    ext_of(sum)
}

// ---- Double-double arithmetic for the logarithms --------------------------------------------
//
// `FYL2X` and `FYL2XP1` feed `pow`, `log` and `exp` of the CRT, which multiply the logarithm by
// up to a thousand or so before `F2XM1`/`FSCALE`: a logarithm good to a double's 53 bits would
// cost hundreds of units in the last place there. So it is computed to about 100 bits (the sum of
// two doubles) and rounded to the FPU's 64.

/// `hi + lo`, `|lo|` at most half a unit in the last place of `hi`.
#[derive(Clone, Copy)]
struct Dd(f64, f64);

fn two_sum(a: f64, b: f64) -> Dd {
    let s = a + b;
    let bb = s - a;
    Dd(s, (a - (s - bb)) + (b - bb))
}

fn dd_add(a: Dd, b: Dd) -> Dd {
    let s = two_sum(a.0, b.0);
    two_sum(s.0, s.1 + a.1 + b.1)
}

fn dd_neg(a: Dd) -> Dd {
    Dd(-a.0, -a.1)
}

fn dd_mul(a: Dd, b: Dd) -> Dd {
    let p = a.0 * b.0;
    let e = a.0.mul_add(b.0, -p);
    two_sum(p, e + (a.0 * b.1 + a.1 * b.0))
}

fn dd_div(a: Dd, b: Dd) -> Dd {
    let q1 = a.0 / b.0;
    let r = dd_add(a, dd_neg(dd_mul(b, Dd(q1, 0.0))));
    let q2 = r.0 / b.0;
    let r = dd_add(r, dd_neg(dd_mul(b, Dd(q2, 0.0))));
    let q3 = r.0 / b.0;
    dd_add(two_sum(q1, q2), Dd(q3, 0.0))
}

/// ln(2) as a double-double.
const LN_2: Dd = Dd(std::f64::consts::LN_2, 2.3190468138462996e-17);

/// log2(e) as a double-double.
const LOG2_E: Dd = Dd(std::f64::consts::LOG2_E, 2.0355273740931033e-17);

/// `2 atanh(s) = ln((1 + s) / (1 - s))` for `|s| <= 1/3`, by its series.
fn twice_atanh(s: Dd) -> Dd {
    let s2 = dd_mul(s, s);
    let mut term = s;
    let mut sum = s;
    for k in 1..60 {
        term = dd_mul(term, s2);
        let t = dd_div(term, Dd(f64::from(2 * k + 1), 0.0));
        sum = dd_add(sum, t);
        if t.0.abs() <= 1e-34 * sum.0.abs() || t.0 == 0.0 {
            break;
        }
    }
    dd_add(sum, sum)
}

/// The value `sig * 2^shift` (`sig` a 64-bit integer) as a double-double, exactly (`shift` in
/// the double range).
fn dd_of(sig: u64, shift: i32) -> Dd {
    let scale = 2f64.powi(shift);
    Dd(
        ((sig >> 11) << 11) as f64 * scale,
        (sig & 0x7ff) as f64 * scale,
    )
}

/// A double-double rounded to the nearest 64-bit register value.
fn ext_of(d: Dd) -> X87Ext {
    add(load64(d.0), load64(d.1), false, 64, ROUND_NEAREST)
}

/// `log2(x)` of a positive finite `x = sig * 2^(e - BIAS - 63)` as a register value: the
/// exponent added to the significand's logarithm (taken in [sqrt(1/2), sqrt(2)), where the
/// series is short).
fn log2_of(e: i32, sig: u64) -> X87Ext {
    let mut k = e - BIAS;
    let mut m = dd_of(sig, -63);
    if sig > 0xb504_f333_f9de_6484 {
        m = Dd(m.0 / 2.0, m.1 / 2.0);
        k += 1;
    }
    let s = dd_div(dd_add(m, Dd(-1.0, 0.0)), dd_add(m, Dd(1.0, 0.0)));
    let log2m = dd_mul(twice_atanh(s), LOG2_E);
    ext_of(dd_add(Dd(f64::from(k), 0.0), log2m))
}

pub fn fyl2x(y: X87Ext, x: X87Ext) -> X87Ext {
    let (uy, ux) = (unpack(y), unpack(x));
    if let Some(nan) = nan_of_two(y, uy, x, ux) {
        return nan;
    }
    if ux.neg && !matches!(ux.kind, Kind::Zero) {
        return indefinite();
    }
    let l = match ux.kind {
        Kind::Zero => infinity(true),
        Kind::Finite { e, sig } => log2_of(e, sig),
        _ => infinity(false),
    };
    // y * log2(x): 0 * infinity (x 0 or infinity with y 0, or x 1 with y infinite) is invalid,
    // as `multiply` has it.
    multiply(y, l, 64, ROUND_NEAREST)
}

pub fn fyl2xp1(y: X87Ext, x: X87Ext) -> X87Ext {
    let (uy, ux) = (unpack(y), unpack(x));
    if let Some(nan) = nan_of_two(y, uy, x, ux) {
        return nan;
    }
    let l = match ux.kind {
        Kind::Zero => x,
        // Far below 1: log2(1 + x) is x log2(e) to below the last bit.
        Kind::Finite { e, .. } if e < BIAS - 70 => multiply(x, ext_of(LOG2_E), 64, ROUND_NEAREST),
        Kind::Finite { e, sig } if e < BIAS => {
            // |x| < 1: ln(1 + x) = 2 atanh(x / (2 + x)).
            let mut xd = dd_of(sig, e - BIAS - 63);
            if ux.neg {
                xd = dd_neg(xd);
            }
            let s = dd_div(xd, dd_add(Dd(2.0, 0.0), xd));
            ext_of(dd_mul(twice_atanh(s), LOG2_E))
        }
        // Outside the instruction's range (|x| >= 1): 1 + x then the logarithm.
        _ => {
            let one_plus = add(load64(1.0), x, false, 64, ROUND_NEAREST);
            let u = unpack(one_plus);
            match u.kind {
                Kind::Finite { e, sig } if !u.neg => log2_of(e, sig),
                Kind::Zero => infinity(true),
                Kind::Inf if !u.neg => infinity(false),
                _ => return indefinite(),
            }
        }
    };
    multiply(y, l, 64, ROUND_NEAREST)
}

#[cfg(all(test, target_arch = "x86_64"))]
mod against_the_fpu;
