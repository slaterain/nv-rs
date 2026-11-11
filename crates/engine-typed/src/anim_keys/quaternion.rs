//! `NiQuaternion`'s functions the rotation keys reach (the Xbox PDB's `NiQuaternion` source unit,
//! and the unit at `00a6db50`): x87 code on four floats, each `FADD`, `FSUB` and `FMUL` through the
//! `x87_*` helpers in the exe's operand order (destination first); `FLD dword` is `x87_load32`, a
//! `float` memory operand `x87_m32`, `FSTP dword` `x87_store32`.

use super::{NiQuaternion, FAMILY};
use engine_core::ptr::{Field, Ptr};
use engine_core::x87::{
    x87_add, x87_load32, x87_m32, x87_mul, x87_quiet_bits, x87_store32, x87_sub,
};
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

/// `FSTP dword tmp; FLD dword tmp`: the value rounded to a float and loaded back.
fn spill(v: f64) -> f64 {
    x87_load32(x87_store32(v))
}

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
/// rounded to a float, in `ST0`.
pub fn counter_warp(e: &mut Engine, t: f32, cos: f32) -> f64 {
    let scaled = x87_mul(x87_load32(cos.to_bits()), e.global::<f64>(COSINE_FACTOR));
    let f = spill(x87_sub(1.0, scaled));
    let f2 = spill(x87_mul(f, f));
    let g = spill(x87_mul(f2, e.global::<f64>(SQUARE_FACTOR)));
    let t = x87_load32(t.to_bits());
    let gt = x87_mul(g, t);
    let twice = x87_add(t, t);
    let a = x87_mul(gt, x87_sub(twice, e.global::<f64>(THREE)));
    let one_plus = x87_add(1.0, a);
    let sum = x87_add(g, one_plus);
    spill(x87_mul(t, sum))
}

// Typed from 00a6e330
/// `NiQuaternion::Slerp` (Xbox PDB, `thiscall` on the result): with `cos = p . q`, the parameter is
/// corrected by [`counter_warp`] (`t <= 0.5`: on `t`; otherwise, a NaN `t` included, `1 - warp(1 -
/// t)`), each component becomes `p + k (q - p)`, and the result is normalized (`FastNormalize`, by
/// address).
pub fn slerp(
    e: &mut Engine,
    this: Ptr<NiQuaternion>,
    t: f32,
    p: Ptr<NiQuaternion>,
    q: Ptr<NiQuaternion>,
) {
    let products =
        COMPONENTS.map(|field| x87_mul(x87_load32(e.get(p, field)), x87_m32(e.get(q, field))));
    let [d0, d1, d2, d3] = products;
    let dot = x87_store32(x87_add(x87_add(x87_add(d0, d1), d2), d3));
    let half = x87_load32(e.global::<u32>(HALF_FLOAT));
    let tv = x87_load32(t.to_bits());
    let cos = f32::from_bits(x87_quiet_bits(dot));
    // FCOM; TEST AH,0x41; JP: below or equal corrects `t`; above or unordered the mirrored parameter.
    let corrected = if tv <= half {
        let t = f32::from_bits(x87_store32(tv));
        typed_call!(FAMILY, 0x00a6_dc30, counter_warp(e, t, cos))
    } else {
        let mirrored = f32::from_bits(x87_store32(spill(x87_sub(1.0, tv))));
        let warped = typed_call!(FAMILY, 0x00a6_dc30, counter_warp(e, mirrored, cos));
        x87_sub(1.0, warped)
    };
    let k = spill(corrected);
    for (i, field) in COMPONENTS.into_iter().enumerate() {
        let from = spill(x87_load32(e.get(p, field)));
        let d = x87_sub(x87_load32(e.get(q, field)), from);
        // The exe's operand orders: `p + d k` for W, `d k + p` for X and Y, `k d + p` for Z.
        let r = match i {
            0 => x87_add(from, x87_mul(d, k)),
            3 => x87_add(x87_mul(k, d), from),
            _ => x87_add(x87_mul(d, k), from),
        };
        e.set(this, field, x87_store32(r));
    }
    e.call(FAST_NORMALIZE, &args![this]);
}
