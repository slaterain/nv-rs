//! Native float maths for the typed families (ADR-0014's float rule; AGENT-RULES, "SPEED FIRST"): what
//! replaces the exact x87 helpers ([`engine_core::x87`]) where a family's float code goes native.
//!
//! **Code only the main thread runs: plain `f32`.** The exe runs its x87 code there at 24-bit
//! precision (the main thread's control word once Direct3D has made its device), which rounds every
//! `FADD`, `FSUB`, `FMUL` and `FDIV` of two floats to a float's 24 bits, as `f32` arithmetic does. The
//! same operations in the exe's order therefore give the exe's float, bit for bit, while every value
//! stays inside the float range: the x87 keeps a wider exponent, so an intermediate past `f32::MAX`,
//! or below the smallest normal float, is where the two part. An operation with a `double` operand (a
//! constant of the exe, or a value a callee leaves in ST0) is done in `f64` and rounded to a float once
//! ([`mul_d`], [`div_d`], [`add_d`], [`sub_d`]). The x87 rounds the exact result once; the `f64`
//! rounds it to 53 bits first, which changes the float only where that lands exactly halfway between
//! two floats.
//!
//! **Code that also runs on other threads: `f64` locals ([`wide`]), each operation in the exe's order,
//! rounded where the exe stores a float ([`narrow`]).** The game's other threads keep 53-bit
//! precision, so there this is the exe's arithmetic bit for bit; on the main thread the last bits may
//! differ.
//!
//! Either way there is no control word to read and no x87 NaN choice to make: a NaN's payload may
//! differ from the exe's, and an `FLD`/`FSTP` pair is a plain copy (a signalling NaN keeps its
//! payload). Float to integer still goes through [`engine_core::x87::fistp_chop`] and its family,
//! never `as`.

/// A `float` word as an `f64` local, in code that runs on any thread (every float is exact in one).
#[inline(always)]
pub fn wide(bits: u32) -> f64 {
    f64::from(f32::from_bits(bits))
}

/// An `f64` rounded to the `float` a store of it makes (to nearest, ties to even).
#[inline(always)]
#[allow(clippy::cast_possible_truncation)]
pub fn narrow(value: f64) -> f32 {
    value as f32
}

/// `a * b` for a float and a `double` (`FMUL qword`, or ST0 times a float), rounded to a float once.
#[inline(always)]
pub fn mul_d(a: f32, b: f64) -> f32 {
    narrow(f64::from(a) * b)
}

/// `a / b` for a float and a `double` (`FDIV qword`), rounded to a float once.
#[inline(always)]
pub fn div_d(a: f32, b: f64) -> f32 {
    narrow(f64::from(a) / b)
}

/// `a + b` for a float and a `double` (`FADD qword`), rounded to a float once.
#[inline(always)]
pub fn add_d(a: f32, b: f64) -> f32 {
    narrow(f64::from(a) + b)
}

/// `a - b` for a float and a `double` (`FSUB qword`), rounded to a float once.
#[inline(always)]
pub fn sub_d(a: f32, b: f64) -> f32 {
    narrow(f64::from(a) - b)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The double-operand helpers round once from the exact result where the `f64` holds it, and
    /// `narrow` rounds to nearest, ties to even.
    #[test]
    fn helpers_round_once() {
        // 1 + 2^-24 is halfway between 1 and the next float: ties to even gives 1.
        assert_eq!(narrow(1.0 + 2f64.powi(-24)), 1.0);
        assert_eq!(narrow(1.0 + 3.0 * 2f64.powi(-24)), 1.0 + 2f32.powi(-22));
        assert_eq!(mul_d(3.0, 0.5), 1.5);
        assert_eq!(div_d(1.0, 3.0), 1.0 / 3.0);
        assert_eq!(add_d(1.0, 2f64.powi(-30)), 1.0);
        assert_eq!(sub_d(1.0, 0.25), 0.75);
        assert_eq!(wide(0x3f80_0000), 1.0);
        assert!(wide(0x7fc0_0000).is_nan());
    }
}
