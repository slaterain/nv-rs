//! `functions_ln/ln_fn_math.h` (JIP LN NVSE): a square root, the distance between two references
//! (2D and 3D, also as conditions) and the trigonometric functions in degrees. All 10 of its
//! commands.
//!
//! JIP computes these with its own SSE routines (`internal/utility.cpp`: `Cos`, `Cos_V3`,
//! `GetSinCos`, `Tan`, `ASin`, `ACos`, `ATan2`; `internal/Ni_types.cpp`: `Point2Distance`,
//! `Point3Distance`): polynomials in single precision with JIP's constants, translated here
//! operation for operation in `f32`, so each answer is JIP's to the bit. The distances use the
//! processor's reciprocal square root estimate (`rsqrtss`) and one Newton step, as JIP does:
//! the port runs the same instruction, so the answer is the one JIP gives on the same processor
//! (Intel and AMD answer `rsqrtss` differently in the last bits).

use super::extra::{self, kind};
use super::{Handlers, DBL_180_D_PI, FLT_PI_D_180};
use engine_core::prelude::*;
use nvse::args::Arg;
use nvse::commands::Read;
use nvse::{handled, CommandArgs, EvalArgs};

#[rustfmt::skip]
pub static COMMANDS: Handlers = &[
    (0x22f0, fsqrt),
    (0x22fd, f_sin),
    (0x22fe, f_cos),
    (0x2893, f_tan),
    (0x230f, f_asin),
    (0x2310, f_acos),
    (0x230b, f_atan),
    (0x2894, f_atan2),
];

#[rustfmt::skip]
pub static CONDITIONS: super::Conditions = &[
    (0x22e0, get_distance_2d, get_distance_2d_eval),
    (0x22e1, get_distance_3d, get_distance_3d_eval),
];

/// `FLT_MAX`, the answer for two references in different cells and worldspaces.
const FLT_MAX: f32 = f32::MAX;
/// A reference's position (`TESObjectREFR::position`, +0x30: x, y, z; the exe's `GetPos`
/// reads it there) and its parent cell (+0x40).
const POSITION: u32 = 0x30;
const PARENT_CELL: u32 = 0x40;
/// `TESObjectCELL::worldSpace` (+0xC0).
const CELL_WORLD_SPACE: u32 = 0xc0;
/// `ExtraPersistentCell::persistentCell` (+0xC).
const PERSISTENT_CELL: u32 = 0x0c;

/// JIP's float constants (`internal/utility.h`, `kPackedValues`), by their bits.
const FLT_PI_D2: f32 = f32::from_bits(0x3fc9_0fdb);
const FLT_PI: f32 = f32::from_bits(0x4049_0fdb);
const FLT_PI_X2: f32 = f32::from_bits(0x40c9_0fdb);
const FLT_EPSILON: f32 = f32::EPSILON;
const SIGN: u32 = 0x8000_0000;

fn flip(v: f32, sign: u32) -> f32 {
    f32::from_bits(v.to_bits() ^ sign)
}

// ---- JIP's maths (`internal/utility.cpp`, `internal/Ni_types.cpp`) ----

/// `kFlt2dPI` and `kCosConsts` of `Cos` and `Cos_V3`.
const FLT_2_D_PI: f32 = f32::from_bits(0x3f22_f983);
const COS_CONSTS: [u32; 5] = [0x37c2_3ab1, 0x3ab5_9551, 0x3d2a_a76f, 0x3eff_ffe0, 0x3f7f_ffff];

/// The polynomial both `Cos` routines end with, on the reduced angle `x`, its sign flipped by
/// `sign`.
fn cos_poly(x: f32, sign: u32) -> f32 {
    let c = COS_CONSTS.map(f32::from_bits);
    let x2 = x * x;
    let mut p = x2 * c[0];
    p -= c[1];
    p *= x2;
    p += c[2];
    p *= x2;
    p -= c[3];
    flip(x2 * p + c[4], sign)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/utility.cpp: Cos (asm).
/// `Cos`: the absolute angle's quadrant `q` (`cvttss2si` of the angle times 2/pi, the integer
/// indefinite for NaN and past the range); above 3 the angle less whole turns (`q >> 2` times
/// 2pi) and `q & 3`. A reduced angle of 0 or less, or NaN, answers exactly 1. Quadrant 1 and 2:
/// less pi, the sign flipped; quadrant 3 (parity even): less 2pi; then the polynomial.
pub fn cos(angle: f32) -> f32 {
    let mut x = angle.abs();
    let mut q = super::cvttss2si(x * FLT_2_D_PI);
    if q > 3 {
        let turns = (q as u32) >> 2;
        q &= 3;
        x -= (turns as i32) as f32 * FLT_PI_X2;
    }
    if x.partial_cmp(&0.0) != Some(std::cmp::Ordering::Greater) {
        return 1.0;
    }
    // `test al, al`: 0 keeps the angle; then `jp` on the low byte's parity: 1 and 2 (odd) take
    // pi and flip the sign, 3 (even) takes 2pi. Past 3 the quadrant was masked, and the
    // indefinite's low byte is 0.
    let (x, sign) = match q & 0xff {
        0 => (x, 0),
        1 | 2 => (x - FLT_PI, SIGN),
        _ => (x - FLT_PI_X2, 0),
    };
    cos_poly(x, sign)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/utility.cpp: Cos_V3 (asm), one lane.
/// `Cos_V3` in one lane (its lanes do not mix: a reduction other lanes start subtracts 0 here):
/// as [`cos`] without its early answer of 1, so an angle of 0 gives the polynomial's
/// `0x3F7FFFFF` and NaN stays NaN; the quadrant past 3 masked by 3 and the turns taken off; a
/// quadrant of 1 or 2 less pi with the sign flipped, 3 less 2pi, any other (0, the indefinite)
/// nothing.
pub fn cos_lane(angle: f32) -> f32 {
    let mut x = angle.abs();
    let mut q = super::cvttss2si(x * FLT_2_D_PI);
    if q > 3 {
        let turns = (q as u32) >> 2;
        q &= 3;
        x -= (turns as i32) as f32 * FLT_PI_X2;
    }
    let (offset, sign) = match q {
        3 => (FLT_PI_X2, 0),
        1 | 2 => (FLT_PI, SIGN),
        _ => (0.0, 0),
    };
    cos_poly(x - offset, sign)
}

/// `Sin(angle)` (JIP LN NVSE, `internal/utility.h`): `Cos(FltPId2 - angle)`.
pub fn sin(angle: f32) -> f32 {
    cos(FLT_PI_D2 - angle)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/utility.cpp: Tan (asm), GetSinCos.
/// `Tan`: the sine and cosine from `Cos_V3` (`GetSinCos`: lanes pi/2 - angle and angle); 0
/// when the cosine's size is below `FLT_EPSILON` or NaN, else their quotient.
pub fn tan(angle: f32) -> f32 {
    let s = cos_lane(FLT_PI_D2 - angle);
    let c = cos_lane(angle);
    if c.abs() >= FLT_EPSILON {
        s / c
    } else {
        0.0
    }
}

/// `kASinConsts`: the polynomial (4), 2^-23, 1, pi, the `0x03800000` exponent step (times 128),
/// then the 128 corrections by `trunc(x * 128)`.
#[rustfmt::skip]
const ASIN_CONSTS: [u32; 136] = [
    0xBC996E30, 0x3D981627, 0x3E593484, 0x3FC90FDB, 0x34000000, 0x3F800000, 0x40490FDB, 0x03800000,
    0x37202A00, 0x37E24000, 0x38333600, 0x386E4C00, 0x38913200, 0x38A84000, 0x38BC5200, 0x38CDAC00,
    0x38DC5000, 0x38E8C000, 0x38F2C800, 0x38FAA800, 0x39005600, 0x39029000, 0x3903C800, 0x39046400,
    0x39042C00, 0x39036800, 0x39022800, 0x39003400, 0x38FB7000, 0x38F60800, 0x38EFD800, 0x38E8B800,
    0x38E15800, 0x38D96000, 0x38D0C000, 0x38C83000, 0x38BF2000, 0x38B59800, 0x38AC5800, 0x38A2E000,
    0x38998000, 0x388FD000, 0x38866000, 0x387AA000, 0x38678000, 0x3855E000, 0x3844A000, 0x38338000,
    0x38234000, 0x3812E000, 0x38042000, 0x37EB4000, 0x37CF4000, 0x37B64000, 0x379D4000, 0x37874000,
    0x37658000, 0x373F8000, 0x371A8000, 0x36F90000, 0x36C30000, 0x36900000, 0x364E0000, 0x360E0000,
    0x35B40000, 0x35500000, 0x34D00000, 0x33C00000, 0x34000000, 0x34800000, 0x35400000, 0x35A80000,
    0x36040000, 0x36400000, 0x367C0000, 0x36A60000, 0x36D60000, 0x37040000, 0x371D0000, 0x37390000,
    0x37560000, 0x37750000, 0x378B0000, 0x379C0000, 0x37AC8000, 0x37BE8000, 0x37D08000, 0x37E30000,
    0x37F58000, 0x3803C000, 0x380D0000, 0x3815C000, 0x381EC000, 0x38274000, 0x382F8000, 0x38380000,
    0x383F8000, 0x38470000, 0x384E8000, 0x3854C000, 0x385B0000, 0x38604000, 0x38658000, 0x3869C000,
    0x386D4000, 0x38708000, 0x3872C000, 0x38744000, 0x38750000, 0x38750000, 0x38740000, 0x38724000,
    0x38700000, 0x386C0000, 0x3867C000, 0x3862C000, 0x385C8000, 0x38558000, 0x384D8000, 0x38450000,
    0x383B0000, 0x38318000, 0x38260000, 0x381A0000, 0x380D8000, 0x38010000, 0x37E70000, 0x37CB0000,
    0x37AF0000, 0x37930000, 0x376C0000, 0x37340000, 0x37020000, 0x36A00000, 0x36180000, 0x35000000,
];

fn asin_const(i: usize) -> f32 {
    f32::from_bits(ASIN_CONSTS[i])
}

/// The correction index of `|x|` below 1: `cvttss2si` of `x` with 7 added to its exponent
/// (`paddd` of `0x03800000`).
fn asin_index(x: f32) -> usize {
    let scaled = f32::from_bits(x.to_bits().wrapping_add(ASIN_CONSTS[7]));
    super::cvttss2si(scaled) as usize
}

/// `x * (x * (x * c0 + c1) - c2)`: the first three steps both routines share.
fn asin_head(x: f32) -> f32 {
    let mut p = asin_const(0) * x;
    p += asin_const(1);
    p *= x;
    p -= asin_const(2);
    p * x
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/utility.cpp: ASin (asm).
/// `ASin`: 0 (either sign) and NaN answer themselves; a size of 1 or more pi/2 with the sign;
/// else pi/2 - sqrt(1 - x) * polynomial + the correction, the sign put back.
pub fn asin(value: f32) -> f32 {
    if value == 0.0 || value.is_nan() {
        return value;
    }
    let sign = value.to_bits() & SIGN;
    let x = value.abs();
    if x >= 1.0 {
        return flip(FLT_PI_D2, sign);
    }
    let idx = asin_index(x);
    let p = asin_head(x) + FLT_PI_D2;
    let r = FLT_PI_D2 - (1.0 - x).sqrt() * p;
    flip(r + asin_const(8 + idx), sign)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/utility.cpp: ACos (asm).
/// `ACos`: 0 (either sign) and NaN answer pi/2; a size of 1 or more 0, or pi for a negative
/// value; else sqrt(1 - x) * polynomial - the correction, and for a negative value pi less that.
pub fn acos(value: f32) -> f32 {
    if value == 0.0 || value.is_nan() {
        return FLT_PI_D2;
    }
    let sign = value.to_bits() & SIGN;
    let x = value.abs();
    if x >= 1.0 {
        return if sign != 0 { FLT_PI } else { 0.0 };
    }
    let idx = asin_index(x);
    let p = asin_head(x) + FLT_PI_D2;
    let r = (1.0 - x).sqrt() * p - asin_const(8 + idx);
    if sign != 0 {
        flip(r, SIGN) + FLT_PI
    } else {
        r
    }
}

/// `kATanConsts` of `ATan2`.
const ATAN_CONSTS: [u32; 6] = [0xbc5c_dd30, 0x3d6b_6d55, 0x3df8_4c31, 0x3e48_54c9, 0x3eaa_7e45, 0x3f7f_ffb7];

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/utility.cpp: ATan2 (asm).
/// `ATan2(y, x)`: y of 0 or NaN answers y when x is 0 or more, else pi; x of 0 or NaN answers
/// pi/2 with y's sign. Else the polynomial on min/max of the sizes, pi/2 less it when |y| is
/// larger, pi less that when x is negative, y's sign put on.
pub fn atan2(y: f32, x: f32) -> f32 {
    if y == 0.0 || y.is_nan() {
        return if x >= 0.0 { y } else { FLT_PI };
    }
    if x == 0.0 || x.is_nan() {
        return f32::from_bits((y.to_bits() & SIGN) | FLT_PI_D2.to_bits());
    }
    let c = ATAN_CONSTS.map(f32::from_bits);
    let (ay, ax) = (y.abs(), x.abs());
    // `maxss xmm3(|y|), |x|` and `minss xmm4(|x|), |y|`.
    let hi = if ay > ax { ay } else { ax };
    let lo = if ax < ay { ax } else { ay };
    let t = lo / hi;
    let s = t * t;
    let mut p = s * c[0];
    p += c[1];
    p *= s;
    p -= c[2];
    p *= s;
    p += c[3];
    p *= s;
    p -= c[4];
    p *= s;
    p += c[5];
    let mut r = t * p;
    if ay > ax {
        r = flip(r, SIGN) + FLT_PI_D2;
    }
    if x < 0.0 {
        r = flip(r, SIGN) + FLT_PI;
    }
    f32::from_bits((y.to_bits() & SIGN) ^ r.to_bits())
}

/// `ATan(x)` (JIP LN NVSE, `internal/utility.h`): `ATan2(x, 1)`.
pub fn atan(x: f32) -> f32 {
    atan2(x, 1.0)
}

/// `rsqrtss`: the processor's estimate of 1/sqrt, as JIP's code gets it.
fn rsqrt_estimate(v: f32) -> f32 {
    #[cfg(target_arch = "x86_64")]
    {
        use std::arch::x86_64::{_mm_cvtss_f32, _mm_rsqrt_ss, _mm_set_ss};
        // SAFETY: SSE is part of every x86_64 processor.
        unsafe { _mm_cvtss_f32(_mm_rsqrt_ss(_mm_set_ss(v))) }
    }
    #[cfg(target_arch = "x86")]
    {
        use std::arch::x86::{_mm_cvtss_f32, _mm_rsqrt_ss, _mm_set_ss};
        // SAFETY: the port's 32-bit builds require SSE2.
        unsafe { _mm_cvtss_f32(_mm_rsqrt_ss(_mm_set_ss(v))) }
    }
    #[cfg(not(any(target_arch = "x86_64", target_arch = "x86")))]
    {
        1.0 / v.sqrt()
    }
}

/// The square root both distance routines take of a sum of squares `s`: 0 (and NaN) answer
/// themselves; else `s * ((3 - s * r * r) * r * 0.5)` with `r` the estimate.
fn newton_sqrt(s: f32) -> f32 {
    if s == 0.0 || s.is_nan() {
        return s;
    }
    let r = rsqrt_estimate(s);
    let t = s * r * r;
    let u = (3.0 - t) * r * 0.5;
    s * u
}

fn position(e: &Engine, refr: u32) -> [f32; 3] {
    [0, 4, 8].map(|o| e.mem.f32(refr + POSITION + o))
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/Ni_types.cpp: Point2Distance (asm).
/// The distance in x and y: `dx*dx + dy*dy` (`haddps`), then [`newton_sqrt`].
pub fn point2_distance(a: [f32; 3], b: [f32; 3]) -> f32 {
    let (dx, dy) = (a[0] - b[0], a[1] - b[1]);
    newton_sqrt(dx * dx + dy * dy)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/Ni_types.cpp: Point3Distance (asm).
/// The distance in x, y and z: `(dx*dx + dy*dy) + dz*dz` (two `haddps`, the fourth lane masked
/// to 0), then [`newton_sqrt`].
pub fn point3_distance(a: [f32; 3], b: [f32; 3]) -> f32 {
    let (dx, dy, dz) = (a[0] - b[0], a[1] - b[1], a[2] - b[2]);
    newton_sqrt((dx * dx + dy * dy) + dz * dz)
}

/// A reference's cell for the test below: its parent cell, else its `ExtraPersistentCell`'s
/// cell; `None` when it has neither.
fn cell_or_persistent(e: &mut Engine, refr: u32) -> Option<u32> {
    let cell = e.mem.u32(refr + PARENT_CELL);
    if cell != 0 {
        return Some(cell);
    }
    match extra::refr_extra(e, refr, kind::PERSISTENT_CELL) {
        0 => None,
        x => Some(e.mem.u32(x + PERSISTENT_CELL)),
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), nvse/GameObjects.cpp:
// TESObjectREFR::GetInSameCellOrWorld (asm).
/// Whether the two references are in the same cell, or their cells in the same (non-null)
/// worldspace. A reference with no cell of either kind: false (the first's looked at first).
/// JIP reads the target without a null check (a condition's reference is always given).
pub fn in_same_cell_or_world(e: &mut Engine, refr: u32, target: u32) -> bool {
    let Some(c1) = cell_or_persistent(e, refr) else {
        return false;
    };
    let Some(c2) = cell_or_persistent(e, target) else {
        return false;
    };
    if c1 == c2 {
        return true;
    }
    let ws = e.mem.u32(c1 + CELL_WORLD_SPACE);
    ws != 0 && ws == e.mem.u32(c2 + CELL_WORLD_SPACE)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), nvse/GameObjects.cpp: TESObjectREFR::GetDistance.
/// The 3D distance when in the same cell or worldspace, else `FLT_MAX`.
pub fn distance_3d(e: &mut Engine, refr: u32, target: u32) -> f32 {
    if in_same_cell_or_world(e, refr, target) {
        point3_distance(position(e, refr), position(e, target))
    } else {
        FLT_MAX
    }
}

fn distance_2d(e: &mut Engine, refr: u32, target: u32) -> f32 {
    if in_same_cell_or_world(e, refr, target) {
        point2_distance(position(e, refr), position(e, target))
    } else {
        FLT_MAX
    }
}

// ---- the commands ----

fn float_arg(read: &Read, i: usize) -> Option<f32> {
    read.ok.then(|| read.float(i, 0.0))
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_math.h: Cmd_fsqrt_Execute.
/// `sqrtsd` of `maxsd(0, value)`: a negative value is 0, -0 and NaN pass (the compare is false).
fn fsqrt(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if read.ok {
        let value = match read.args.first() {
            Some(Arg::Double(v)) => *v,
            _ => 0.0,
        };
        let m = if 0.0 > value { 0.0 } else { value };
        a.set_result(e, m.sqrt());
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_math.h:
// Cmd_GetDistance2D_Execute.
/// The 2D distance to the reference; `FLT_MAX` in different cells and worldspaces, or when the
/// argument is not read.
fn get_distance_2d(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let d = if read.ok {
        distance_2d(e, a.this_obj, read.form(0))
    } else {
        FLT_MAX
    };
    a.set_result(e, f64::from(d));
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_math.h: Cmd_GetDistance2D_Eval.
fn get_distance_2d_eval(e: &mut Engine, w: &[u32]) -> Ret {
    let a = EvalArgs::new(w);
    let d = distance_2d(e, a.this_obj, a.arg1);
    e.mem.set_f64(a.result, f64::from(d));
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_math.h:
// Cmd_GetDistance3D_Execute.
fn get_distance_3d(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let d = if read.ok {
        distance_3d(e, a.this_obj, read.form(0))
    } else {
        FLT_MAX
    };
    a.set_result(e, f64::from(d));
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_math.h: Cmd_GetDistance3D_Eval.
fn get_distance_3d_eval(e: &mut Engine, w: &[u32]) -> Ret {
    let a = EvalArgs::new(w);
    let d = distance_3d(e, a.this_obj, a.arg1);
    e.mem.set_f64(a.result, f64::from(d));
    handled(true)
}

/// A one-float command: `f(value)` as the result, nothing when the argument is not read.
fn float_command(e: &mut Engine, w: &[u32], f: impl Fn(f32) -> f64) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if let Some(v) = float_arg(&read, 0) {
        a.set_result(e, f(v));
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_math.h: Cmd_fSin_Execute.
/// `Sin(value * FltPId180)`, the product a float.
fn f_sin(e: &mut Engine, w: &[u32]) -> Ret {
    float_command(e, w, |v| f64::from(sin(v * FLT_PI_D_180)))
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_math.h: Cmd_fCos_Execute.
fn f_cos(e: &mut Engine, w: &[u32]) -> Ret {
    float_command(e, w, |v| f64::from(cos(v * FLT_PI_D_180)))
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_math.h: Cmd_fTan_Execute.
fn f_tan(e: &mut Engine, w: &[u32]) -> Ret {
    float_command(e, w, |v| f64::from(tan(v * FLT_PI_D_180)))
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_math.h: Cmd_fAsin_Execute.
/// `ASin(value) * Dbl180dPI`, the float answer widened before the product.
fn f_asin(e: &mut Engine, w: &[u32]) -> Ret {
    float_command(e, w, |v| f64::from(asin(v)) * DBL_180_D_PI)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_math.h: Cmd_fAcos_Execute.
fn f_acos(e: &mut Engine, w: &[u32]) -> Ret {
    float_command(e, w, |v| f64::from(acos(v)) * DBL_180_D_PI)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_math.h: Cmd_fAtan_Execute.
fn f_atan(e: &mut Engine, w: &[u32]) -> Ret {
    float_command(e, w, |v| f64::from(atan(v)) * DBL_180_D_PI)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_math.h: Cmd_fAtan2_Execute.
fn f_atan2(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if read.ok {
        let (y, x) = (read.float(0, 0.0), read.float(1, 0.0));
        a.set_result(e, f64::from(atan2(y, x)) * DBL_180_D_PI);
    }
    handled(true)
}

#[cfg(test)]
#[path = "ln_fn_math_tests.rs"]
mod tests;
