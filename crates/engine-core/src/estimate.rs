//! The processor's quick estimates `RCPSS` (of `1 / x`) and `RSQRTSS` (of `1 / sqrt(x)`), in one
//! shared place.
//!
//! The exe uses them (Havok's normalisers and broad phase, D3DX's maths, JIP's and UIO's code),
//! and their results are not fixed by Intel's manual: only the error bound is (relative error at
//! most 1.5 * 2^-12). Each processor maker's table gives its own last bits (AMD's differ from
//! Intel's), so the original game itself gives different bits on different PCs. Every copy of the
//! port's that answers for them uses this module off x86: engine-core `x87::rsqrt_estimate`
//! (Havok's normaliser), engine-havok's broad phase, AABB tree, ray casts, sphere shape,
//! quaternion and ragdoll helpers, engine-platform's D3DX maths, JIP's and UIO's.
//!
//! On x86 ([`rcp`], [`rsqrt`]) the instruction itself runs: the player's processor answers, as it
//! does for FalloutNV.exe on that PC. Elsewhere (Apple silicon, ARM Linux) there is no such
//! instruction, and the answer comes from a model ([`rcp_model`], [`rsqrt_model`]) of one x86
//! processor's, bit for bit on every input: so such a machine agrees with that PC.
//!
//! The processor modelled is the one named in `estimate/tables.rs`'s header (an AMD Zen 5: the
//! maintainer's PC, where the port is played and judged). The tables are one generated file: an
//! Intel processor's replace them with one run of `scripts/estimate-measure` on an Intel PC, with
//! no code change. Any maker's answers are within the instruction's documented error, so the
//! choice is no difference a player can see or feel.
//!
//! The model: on the processor measured, the result depends only on the input's top 12 mantissa
//! bits and its exponent (`RSQRTSS`: also the exponent's parity picks one of two halves of its
//! table); the exponent only moves the result's exponent. Proven on all 2^32 inputs by
//! `scripts/estimate-measure` (the tool that also wrote `estimate/tables.rs`), and the unit
//! tests compare the model with the instruction on every exponent, sign and table entry wherever
//! the processor is the one the tables were measured on ([`model_is_this_processor`]). Intel's
//! `RSQRTSS`, as published by its reverse-engineering, indexes fewer bits (the exponent's parity
//! and 10 mantissa bits), which tables of this shape also hold: another maker's tables are a new
//! `tables.rs`, not new code.
//!
//! The special inputs, as the measured processor answers them (with the game's MXCSR, 0x1F80): zero and denormal inputs give the infinity of their sign; an infinity
//! gives the zero of its sign (`RSQRTSS` of `-inf`: the default NaN); a NaN comes back quieted;
//! `RSQRTSS` of a negative number gives the default NaN `0xffc00000`; a result below the normal
//! range is the zero of its sign.

mod tables;

/// The default NaN SSE answers for an invalid operation.
const DEFAULT_NAN: u32 = 0xffc0_0000;
const SIGN: u32 = 0x8000_0000;
const EXPONENT: u32 = 0x7f80_0000;
const MANTISSA: u32 = 0x007f_ffff;
const QUIET: u32 = 0x0040_0000;

/// `RCPSS`: the estimate of `1 / x`. The processor's own on x86; [`rcp_model`] elsewhere.
#[inline]
pub fn rcp(x: f32) -> f32 {
    #[cfg(target_arch = "x86_64")]
    {
        use std::arch::x86_64::{_mm_cvtss_f32, _mm_rcp_ss, _mm_set_ss};
        // SAFETY: SSE is part of the x86_64 baseline.
        unsafe { _mm_cvtss_f32(_mm_rcp_ss(_mm_set_ss(x))) }
    }
    #[cfg(target_arch = "x86")]
    {
        use std::arch::x86::{_mm_cvtss_f32, _mm_rcp_ss, _mm_set_ss};
        // SAFETY: the port's 32-bit builds require SSE2.
        unsafe { _mm_cvtss_f32(_mm_rcp_ss(_mm_set_ss(x))) }
    }
    #[cfg(not(any(target_arch = "x86_64", target_arch = "x86")))]
    {
        f32::from_bits(rcp_model(x.to_bits()))
    }
}

/// `RSQRTSS`: the estimate of `1 / sqrt(x)`. The processor's own on x86; [`rsqrt_model`]
/// elsewhere.
#[inline]
pub fn rsqrt(x: f32) -> f32 {
    #[cfg(target_arch = "x86_64")]
    {
        use std::arch::x86_64::{_mm_cvtss_f32, _mm_rsqrt_ss, _mm_set_ss};
        // SAFETY: SSE is part of the x86_64 baseline.
        unsafe { _mm_cvtss_f32(_mm_rsqrt_ss(_mm_set_ss(x))) }
    }
    #[cfg(target_arch = "x86")]
    {
        use std::arch::x86::{_mm_cvtss_f32, _mm_rsqrt_ss, _mm_set_ss};
        // SAFETY: the port's 32-bit builds require SSE2.
        unsafe { _mm_cvtss_f32(_mm_rsqrt_ss(_mm_set_ss(x))) }
    }
    #[cfg(not(any(target_arch = "x86_64", target_arch = "x86")))]
    {
        f32::from_bits(rsqrt_model(x.to_bits()))
    }
}

/// A table entry as a float's bits, for an input whose exponent field is `reference` (the one
/// the table was measured at), moved to the input's exponent field `exponent` by `shift`: the
/// result's exponent field is the entry's plus `shift`. Below the normal range: zero.
#[inline]
fn from_entry(entry: u16, sign: u32, shift: i32) -> u32 {
    let entry = u32::from(entry);
    let exponent = (entry >> 12) as i32 + 120 + shift;
    if exponent < 1 {
        return sign;
    }
    // The table's results lie well inside the range: an input's exponent field of at least 1
    // keeps the result's at most 254 (checked by the tests at the extremes).
    sign | ((exponent as u32) << 23) | ((entry & 0xfff) << 11)
}

/// [`rcp`]'s model: the measured processor's `RCPSS` of the float with bits `x`.
pub fn rcp_model(x: u32) -> u32 {
    let sign = x & SIGN;
    let exponent = x & EXPONENT;
    if exponent == EXPONENT {
        return if x & MANTISSA != 0 { x | QUIET } else { sign };
    }
    if exponent == 0 {
        // Zero and the denormals (read as zero).
        return sign | EXPONENT;
    }
    let field = (exponent >> 23) as i32;
    let entry = tables::RCP[((x & MANTISSA) >> 11) as usize];
    from_entry(entry, sign, 127 - field)
}

/// [`rsqrt`]'s model: the measured processor's `RSQRTSS` of the float with bits `x`.
pub fn rsqrt_model(x: u32) -> u32 {
    let sign = x & SIGN;
    let exponent = x & EXPONENT;
    if exponent == EXPONENT {
        return if x & MANTISSA != 0 {
            x | QUIET
        } else if sign != 0 {
            DEFAULT_NAN
        } else {
            0
        };
    }
    if exponent == 0 {
        // Zero and the denormals (read as zero), negative ones too.
        return sign | EXPONENT;
    }
    if sign != 0 {
        return DEFAULT_NAN;
    }
    let field = (exponent >> 23) as i32;
    // Odd exponent fields (127: 1.m) use the first half, even ones (128: 2 * 1.m) the second.
    let odd = field & 1 == 1;
    let (half, reference) = if odd { (0, 127) } else { (4096, 128) };
    let entry = tables::RSQRT[half + ((x & MANTISSA) >> 11) as usize];
    from_entry(entry, 0, (reference - field) / 2)
}

/// Whether this processor's `RCPSS` and `RSQRTSS` are the ones the tables were measured on (a
/// few inputs that differ between makers): only then can the tests compare the model with the
/// instruction bit for bit.
#[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
pub fn model_is_this_processor() -> bool {
    [
        0x4000_0000u32,
        0x4040_0000,
        0x3f33_3333,
        0x4640_e6b6,
        0x3f80_0000,
        0x3fff_f000,
    ]
    .iter()
    .all(|&x| {
        rcp(f32::from_bits(x)).to_bits() == rcp_model(x)
            && rsqrt(f32::from_bits(x)).to_bits() == rsqrt_model(x)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Inputs of every exponent field and sign, every table entry, with the low 11 mantissa bits
    /// none, all, and a mixed pattern (the measuring tool proved on all 2^32 inputs that those
    /// bits never change a result).
    fn every_kind_of_input() -> impl Iterator<Item = u32> {
        (0..2u32).flat_map(|sign| {
            (0..256u32).flat_map(move |field| {
                (0..4096u32).flat_map(move |index| {
                    [0u32, 0x7ff, 0x5a3]
                        .map(move |low| (sign << 31) | (field << 23) | (index << 11) | low)
                })
            })
        })
    }

    /// The model is the instruction, bit for bit, on this processor when it is the measured one
    /// (elsewhere the test says so and checks nothing: another maker's tables differ).
    #[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
    #[test]
    fn the_model_is_the_instruction_on_the_measured_processor() {
        if !model_is_this_processor() {
            eprintln!("not the processor the estimate tables were measured on: not compared");
            return;
        }
        let mut checked = 0u64;
        for x in every_kind_of_input() {
            let f = f32::from_bits(x);
            assert_eq!(rcp_model(x), rcp(f).to_bits(), "RCPSS {x:08x}");
            assert_eq!(rsqrt_model(x), rsqrt(f).to_bits(), "RSQRTSS {x:08x}");
            checked += 1;
        }
        assert_eq!(checked, 2 * 256 * 4096 * 3);
    }

    /// A digest of the model's answers to every kind of input (FNV-1a over both results' bits),
    /// checked on every system: a fault in the tables or the rules is caught without the
    /// measured processor (the comparison with the instruction runs only on it).
    #[test]
    fn the_models_answers_have_their_digest() {
        let mut digest: u64 = 0xcbf2_9ce4_8422_2325;
        for x in every_kind_of_input() {
            for word in [rcp_model(x), rsqrt_model(x)] {
                for byte in word.to_le_bytes() {
                    digest ^= u64::from(byte);
                    digest = digest.wrapping_mul(0x0000_0100_0000_01b3);
                }
            }
        }
        assert_eq!(digest, DIGEST, "the model's digest: {digest:#018x}");
    }

    /// [`the_models_answers_have_their_digest`]'s value for the committed tables.
    const DIGEST: u64 = 0xe816_cdbe_850e_37a0;

    /// Values measured on the reference processor, checked on every system (Apple silicon's CI
    /// runs this one through the model).
    #[test]
    fn the_model_gives_the_measured_values() {
        for (x, rcp_bits, rsqrt_bits) in [
            (0x4000_0000u32, 0x3eff_f000u32, 0x3f35_0000u32),
            (0x4040_0000, 0x3eaa_a800, 0x3f13_c800),
            (0x3f33_3333, 0x3fb6_e000, 0x3f99_0000),
            (0x4640_e6b6, 0x38a9_e800, 0x3c13_7800),
            (0x0080_0000, 0x7e7f_f000, 0x5eff_f800),
            (0x7f7f_ffff, 0x0000_0000, 0x1f80_0000),
            (0x7f00_0000, 0x0000_0000, 0x1fb5_0000),
            (0xbf80_0000, 0xbf7f_f000, DEFAULT_NAN),
        ] {
            assert_eq!(rcp_model(x), rcp_bits, "RCPSS {x:08x}");
            assert_eq!(rsqrt_model(x), rsqrt_bits, "RSQRTSS {x:08x}");
        }
        // The special inputs.
        for (x, rcp_bits, rsqrt_bits) in [
            (0x0000_0000u32, 0x7f80_0000u32, 0x7f80_0000u32),
            (0x8000_0000, 0xff80_0000, 0xff80_0000),
            (0x0000_0001, 0x7f80_0000, 0x7f80_0000),
            (0x007f_ffff, 0x7f80_0000, 0x7f80_0000),
            (0x7f80_0000, 0x0000_0000, 0x0000_0000),
            (0xff80_0000, 0x8000_0000, DEFAULT_NAN),
            (0x7fc0_0000, 0x7fc0_0000, 0x7fc0_0000),
            (0x7f80_0001, 0x7fc0_0001, 0x7fc0_0001),
            (0xffc0_0000, 0xffc0_0000, 0xffc0_0000),
            (0x7fa0_0000, 0x7fe0_0000, 0x7fe0_0000),
        ] {
            assert_eq!(rcp_model(x), rcp_bits, "RCPSS {x:08x}");
            assert_eq!(rsqrt_model(x), rsqrt_bits, "RSQRTSS {x:08x}");
        }
    }

    /// Within Intel's documented bound everywhere (relative error at most 1.5 * 2^-12), on
    /// every normal input whose result is normal: the tables are estimates of the right values.
    #[test]
    fn the_model_is_within_the_documented_error() {
        let bound = 1.5 * (-12f64).exp2();
        for x in every_kind_of_input() {
            let v = f64::from(f32::from_bits(x));
            if !f32::from_bits(x).is_normal() || x & SIGN != 0 {
                continue;
            }
            let r = f64::from(f32::from_bits(rcp_model(x)));
            if r != 0.0 {
                assert!((r * v - 1.0).abs() <= bound, "RCPSS {x:08x}");
            }
            let s = f64::from(f32::from_bits(rsqrt_model(x)));
            assert!((s * v.sqrt() - 1.0).abs() <= bound, "RSQRTSS {x:08x}");
        }
    }
}
