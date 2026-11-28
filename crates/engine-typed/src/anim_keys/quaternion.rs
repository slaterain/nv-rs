//! `NiQuaternion`'s functions the rotation keys reach (the Xbox PDB's `NiQuaternion` source unit,
//! and the unit at `00a6db50`). The constructor and the negation move floats as the exe's
//! `FLD`/`FSTP` pairs do (`x87_load32`, `x87_store32`). `Slerp`, its parameter correction and
//! `FastNormalize` run for every animated bone every frame: native maths at the running thread's
//! precision ([`Real`]: `f32` on main, `f64` on the AI threads), each operation rounded as the
//! exe's x87 code rounds it there and each float the exe stores rounded to a float, so the result
//! is the exe's on both. `Slerp` does the correction and the normalisation in place, with no call.

use super::search::round;
use super::NiQuaternion;
use crate::native::{float_precision, Real};
use engine_core::ptr::{Field, Ptr};
use engine_core::x87::{x87_load32, x87_quiet_bits, x87_store32};
use engine_core::Engine;

/// `float` 0.5 (`01016248`): at or below it `Slerp` corrects its parameter, above it the mirrored
/// one.
const HALF: f32 = 0.5;
/// The parameter correction's `double` factors, each a float's value: of the cosine (`0109c9d8`,
/// about 0.8228) and of the squared term (`0109c9d0`, about 0.5855); and 3.0 (`01021928`).
const COSINE_FACTOR: u64 = 0x3fea_545a_2000_0000;
const SQUARE_FACTOR: u64 = 0x3fe2_bc5a_2000_0000;
/// `FastNormalize`'s constants: the point its linear guess of `1 / sqrt` is centred on (`01099680`,
/// a `double` holding a float's value), its slope and base (`011f3fec`, `011f3ff0`: floats the
/// static initialiser works out, read where they are), the squared length up to which it takes a
/// Newton step (`01099690`, a `double` holding a float's value) and up to which a second
/// (`01099688`, a float).
const NORMALIZE_CENTER: u64 = 0x3fee_b0ab_2000_0000;
const NORMALIZE_SLOPE: u32 = 0x011f_3fec;
const NORMALIZE_BASE: u32 = 0x011f_3ff0;
const NORMALIZE_SECOND_LIMIT: u64 = 0x3fed_496a_a000_0000;
const NORMALIZE_THIRD_LIMIT: u32 = 0x3f26_f151;

/// A `double` constant that holds a float's value, as `R` (exact).
#[inline(always)]
fn constant<R: Real>(bits: u64) -> R {
    R::narrow(f64::from_bits(bits))
}

/// The four components in memory order.
const COMPONENTS: [Field<NiQuaternion, u32>; 4] = [
    NiQuaternion::m_fW,
    NiQuaternion::m_fX,
    NiQuaternion::m_fY,
    NiQuaternion::m_fZ,
];

// Typed from 00a6db50
/// The constructor from four floats (`thiscall`, no Xbox PDB name; the rotation keys' constructor
/// `00a2bdb0` makes their quaternions with it): each float through `FLD`/`FSTP` into +0, +4, +8,
/// +0xc (a signalling NaN is stored quiet). Returns `this`.
pub fn from_floats(
    e: &mut Engine,
    this: Ptr<NiQuaternion>,
    first: f32,
    second: f32,
    third: f32,
    fourth: f32,
) -> Ptr<NiQuaternion> {
    for (field, value) in COMPONENTS.into_iter().zip([first, second, third, fourth]) {
        e.set(this, field, x87_quiet_bits(value.to_bits()));
    }
    this
}

// Typed from 00a6dbd0
/// `NiQuaternion::operator-` (Xbox PDB, unary), returned by value (`thiscall`, the result's address
/// pushed): each component through `FLD`/`FCHS`/`FSTP` into `result`, one after the other. Returns
/// `result`.
pub fn negate(
    e: &mut Engine,
    this: Ptr<NiQuaternion>,
    result: Ptr<NiQuaternion>,
) -> Ptr<NiQuaternion> {
    for field in COMPONENTS {
        let v = x87_load32(e.get(this, field));
        e.set(result, field, x87_store32(-v));
    }
    result
}

/// [`counter_warp`] at precision `R`, the result as the float the exe stores.
fn counter_warp_value<R: Real>(t: R, cos: R) -> R {
    let f = round(R::ONE - cos * constant::<R>(COSINE_FACTOR));
    let g = round(round(f * f) * constant::<R>(SQUARE_FACTOR));
    let three = R::ONE + R::ONE + R::ONE;
    let a = g * t * ((t + t) - three);
    round(t * (g + (R::ONE + a)))
}

// Typed from 00a6dc30
/// The parameter correction `Slerp` makes (`cdecl`, no Xbox PDB name): with `f = 1 - cos * 0.8228`
/// and `g = f * f * 0.5855` (each rounded to a float), answers `t * (g + (1 + g * t * (2t - 3)))`
/// rounded to a float, in `ST0`, each operation at the running thread's precision. The exe keeps
/// `f`, `f * f`, `g` and the result in its own `cos` argument's slot (`[esp+8]`, stored and loaded
/// back as floats); this keeps them in locals, as the translation does, and leaves the argument
/// words as they came (`cdecl`: the caller drops them).
pub fn counter_warp(_e: &mut Engine, t: f32, cos: f32) -> f64 {
    if float_precision() {
        f64::from(counter_warp_value(t, cos))
    } else {
        counter_warp_value(f64::from(t), f64::from(cos))
    }
}

/// One Newton step of `FastNormalize`'s estimate (each stored value a float).
fn newton_step<R: Real>(estimate: R, length: R, center: R, slope: R, base: R) -> R {
    let squared = round(estimate * estimate * length);
    let factor = round((squared - center) * slope + base);
    round(estimate * factor)
}

/// [`fast_normalize`] of the quaternion `[w, x, y, z]` at precision `R`, as float words.
pub(crate) fn fast_normalized<R: Real>(e: &Engine, q: [u32; 4]) -> [u32; 4] {
    let [w, x, y, z] = q.map(R::load);
    let length = round(((y * y + x * x) + z * z) + w * w);
    let center = constant::<R>(NORMALIZE_CENTER);
    let slope = R::load(e.mem.u32(NORMALIZE_SLOPE));
    let base = R::load(e.mem.u32(NORMALIZE_BASE));
    let mut estimate = round((length - center) * slope + base);
    // FCOM with the double: at most the limit (ordered) takes a step; FCOMP of the float limit with
    // the length: the limit at least the length takes another.
    if length <= constant::<R>(NORMALIZE_SECOND_LIMIT) {
        estimate = newton_step(estimate, length, center, slope, base);
        if R::load(NORMALIZE_THIRD_LIMIT) >= length {
            estimate = newton_step(estimate, length, center, slope, base);
        }
    }
    [
        (estimate * w).store(),
        (x * estimate).store(),
        (estimate * y).store(),
        (z * estimate).store(),
    ]
}

/// The quaternion's four words at `at`.
pub(crate) fn words(e: &Engine, at: u32) -> [u32; 4] {
    [0u32, 4, 8, 0xc].map(|o| e.mem.u32(at.wrapping_add(o)))
}

/// Writes the four words at `at`.
pub(crate) fn set_words(e: &mut Engine, at: u32, q: [u32; 4]) {
    for (i, word) in (0u32..).zip(q) {
        e.mem.set_u32(at.wrapping_add(4 * i), word);
    }
}

// Typed from 00a4ff00
/// `NiQuaternion::FastNormalize` (engine map; `this` in `ECX`, plain `RET`): scales the quaternion
/// by an estimate of `1 / sqrt(length^2)`: a linear guess around `01099680` (slope and base from
/// `011f3fec`, `011f3ff0`), one Newton step when the squared length is at most `01099690` and
/// another when it is at most `01099688` (a NaN length takes none); each stored step a float, each
/// operation at the running thread's precision.
pub fn fast_normalize(e: &mut Engine, this: Ptr) {
    let q = words(e, this.addr());
    let normalized = if float_precision() {
        fast_normalized::<f32>(e, q)
    } else {
        fast_normalized::<f64>(e, q)
    };
    set_words(e, this.addr(), normalized);
}

/// [`slerp`] of the quaternions `p` and `q` (their words) at precision `R`: the normalized result's
/// words. The exe reads each component of `p` and `q` just before it stores the result's, so a
/// result over `p` or `q` reads only words it has not yet written: reading them first is the same.
pub(crate) fn slerp_value<R: Real>(e: &Engine, t: f32, p: [u32; 4], q: [u32; 4]) -> [u32; 4] {
    let (p, q) = (p.map(R::load), q.map(R::load));
    let cos = round(((p[0] * q[0] + p[1] * q[1]) + p[2] * q[2]) + p[3] * q[3]);
    let t_value = R::load(t.to_bits());
    // FCOM; TEST AH,0x41; JP: below or equal corrects `t`; above or unordered the mirrored
    // parameter.
    let corrected = if t <= HALF {
        counter_warp_value(t_value, cos)
    } else {
        let mirrored = round(R::ONE - t_value);
        R::ONE - counter_warp_value(mirrored, cos)
    };
    let k = round(corrected);
    let blended = [0usize, 1, 2, 3].map(|i| (p[i] + (q[i] - p[i]) * k).store());
    fast_normalized::<R>(e, blended)
}

/// [`slerp`] of the quaternions at `p` and `q` into `this`, at the running thread's precision.
pub(crate) fn slerp_at(e: &mut Engine, this: u32, t: f32, p: u32, q: u32) {
    let (p, q) = (words(e, p), words(e, q));
    let result = if float_precision() {
        slerp_value::<f32>(e, t, p, q)
    } else {
        slerp_value::<f64>(e, t, p, q)
    };
    set_words(e, this, result);
}

/// [`squad`] of the quaternions `p`, `a`, `b`, `q` (their words) at precision `R`: the result's
/// words.
pub(crate) fn squad_value<R: Real>(e: &Engine, t: f32, quads: [[u32; 4]; 4]) -> [u32; 4] {
    let [p, a, b, q] = quads;
    let second = slerp_value::<R>(e, t, p, q);
    let first = slerp_value::<R>(e, t, a, b);
    let t_value = R::load(t.to_bits());
    let u = ((R::ONE - t_value) * (t_value + t_value)).store();
    slerp_value::<R>(e, f32::from_bits(u), second, first)
}

/// [`squad`] of the quaternions at `p`, `a`, `b`, `q` at the running thread's precision.
pub(crate) fn squad_of(e: &Engine, t: f32, p: u32, a: u32, b: u32, q: u32) -> [u32; 4] {
    let quads = [p, a, b, q].map(|at| words(e, at));
    if float_precision() {
        squad_value::<f32>(e, t, quads)
    } else {
        squad_value::<f64>(e, t, quads)
    }
}

// Typed from 00a6e430
/// The spherical quadrangle interpolation (`thiscall(this, t, p, a, b, q)`, no Xbox PDB name):
/// `Slerp` at `t` of `p` and `q` and of `a` and `b` into two locals, then `this` the `Slerp` of
/// those at `2t (1 - t)` (rounded to a float), each `Slerp` done in place.
pub fn squad(
    e: &mut Engine,
    this: Ptr<NiQuaternion>,
    t: f32,
    p: Ptr<NiQuaternion>,
    a: Ptr<NiQuaternion>,
    b: Ptr<NiQuaternion>,
    q: Ptr<NiQuaternion>,
) {
    let result = squad_of(e, t, p.addr(), a.addr(), b.addr(), q.addr());
    set_words(e, this.addr(), result);
}

// Typed from 00a6e330
/// `NiQuaternion::Slerp` (Xbox PDB, `thiscall` on the result): with `cos = p . q` (summed `W`, `X`,
/// `Y`, `Z`, rounded to a float), the parameter is corrected by [`counter_warp`] (`t <= 0.5`: on
/// `t`; otherwise, a NaN `t` included, `1 - warp(1 - t)`, `1 - t` rounded to a float), rounded to a
/// float `k`; each component becomes `p + k (q - p)`, read and stored one after the other, and the
/// result is normalized ([`fast_normalize`]). The correction and the normalisation are done in
/// place.
pub fn slerp(
    e: &mut Engine,
    this: Ptr<NiQuaternion>,
    t: f32,
    p: Ptr<NiQuaternion>,
    q: Ptr<NiQuaternion>,
) {
    slerp_at(e, this.addr(), t, p.addr(), q.addr());
}
