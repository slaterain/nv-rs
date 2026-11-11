//! `NiQuaternion`'s functions the rotation keys reach (the Xbox PDB's `NiQuaternion` source unit,
//! and the unit at `00a6db50`). The constructor and the negation move floats as the exe's
//! `FLD`/`FSTP` pairs do (`x87_load32`, `x87_store32`); `Slerp` and its parameter correction are
//! native maths (the family's float rule: `f64` locals, each operation in the exe's order, rounded
//! where the exe stores a float).

use super::{narrow, rounded, wide, NiQuaternion, FAMILY};
use engine_core::ptr::{Field, Ptr};
use engine_core::x87::{x87_load32, x87_quiet_bits, x87_store32};
use engine_core::{args, typed_call, Engine};

/// `NiQuaternion::FastNormalize` (Xbox PDB), `00a4ff00` (`thiscall`): stays by address.
const FAST_NORMALIZE: u32 = 0x00a4_ff00;
/// `float` 0.5 (`01016248`): at or below it `Slerp` corrects its parameter, above it the mirrored one.
const HALF_FLOAT: u32 = 0x0101_6248;
/// The parameter correction's `double` factors: of the cosine (`0109c9d8`, about 0.8228) and of the
/// squared term (`0109c9d0`, about 0.5855); and 3.0 (`01021928`).
const COSINE_FACTOR: u32 = 0x0109_c9d8;
const SQUARE_FACTOR: u32 = 0x0109_c9d0;
const THREE: u32 = 0x0102_1928;

/// The four components in memory order.
const COMPONENTS: [Field<NiQuaternion, u32>; 4] = [
    NiQuaternion::m_fW,
    NiQuaternion::m_fX,
    NiQuaternion::m_fY,
    NiQuaternion::m_fZ,
];

// Typed from 00a6db50
/// The constructor from four floats (`thiscall`, no Xbox PDB name; the rotation keys' constructor
/// `00a2bdb0` makes their quaternions with it): each float through `FLD`/`FSTP` into +0, +4, +8, +0xc
/// (a signalling NaN is stored quiet). Returns `this`.
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

// Typed from 00a6dc30
/// The parameter correction `Slerp` makes (`cdecl`, no Xbox PDB name): with `f = 1 - cos * 0.8228`
/// and `g = f * f * 0.5855` (each rounded to a float), answers `t * (g + (1 + g * t * (2t - 3)))`
/// rounded to a float, in `ST0`. Native: the exe's operations in its order on `f64` locals, rounded
/// at its three stores and at the end. The exe keeps `f`, `f * f`, `g` and the result in its own
/// `cos` argument's slot (`[esp+8]`, stored and loaded back as floats); this keeps them in locals,
/// as the translation does, and leaves the argument words as they came (`cdecl`: the caller drops
/// them).
pub fn counter_warp(e: &mut Engine, t: f32, cos: f32) -> f64 {
    let f = rounded(1.0 - f64::from(cos) * e.global::<f64>(COSINE_FACTOR));
    let g = rounded(rounded(f * f) * e.global::<f64>(SQUARE_FACTOR));
    let t = f64::from(t);
    let a = g * t * (t + t - e.global::<f64>(THREE));
    rounded(t * (g + (1.0 + a)))
}

// Typed from 00a6e330
/// `NiQuaternion::Slerp` (Xbox PDB, `thiscall` on the result): with `cos = p . q` (summed `W`, `X`,
/// `Y`, `Z`, rounded to a float), the parameter is corrected by [`counter_warp`] (`t <= 0.5`: on `t`;
/// otherwise, a NaN `t` included, `1 - warp(1 - t)`, `1 - t` rounded to a float), rounded to a
/// float `k`; each component becomes `p + k (q - p)`, read and stored one after the other, and the
/// result is normalized (`FastNormalize`, by address). Native maths on `f64` locals.
pub fn slerp(
    e: &mut Engine,
    this: Ptr<NiQuaternion>,
    t: f32,
    p: Ptr<NiQuaternion>,
    q: Ptr<NiQuaternion>,
) {
    let [w, x, y, z] = COMPONENTS.map(|field| wide(e.get(p, field)) * wide(e.get(q, field)));
    let cos = narrow(((w + x) + y) + z);
    let half = wide(e.global::<u32>(HALF_FLOAT));
    // FCOM; TEST AH,0x41; JP: below or equal corrects `t`; above or unordered the mirrored parameter.
    let corrected = if f64::from(t) <= half {
        typed_call!(FAMILY, 0x00a6_dc30, counter_warp(e, t, cos))
    } else {
        let mirrored = narrow(1.0 - f64::from(t));
        1.0 - typed_call!(FAMILY, 0x00a6_dc30, counter_warp(e, mirrored, cos))
    };
    let k = rounded(corrected);
    for field in COMPONENTS {
        let from = wide(e.get(p, field));
        let d = wide(e.get(q, field)) - from;
        e.set(this, field, narrow(from + d * k).to_bits());
    }
    e.call(FAST_NORMALIZE, &args![this]);
}
