//! Native float maths for typed code that runs on main and on the AI threads (ADR-0014's float rule):
//! the running thread's x87 precision decides what a step of the exe's code rounds to, so a function
//! is written once over [`Real`] and run as `f32` on main (24 bits: every operation rounds to a float)
//! or as `f64` on the AI threads (53 bits: every operation rounds to a double, and only the exe's float
//! stores round to a float). Each is the exe's arithmetic on its thread, without the x87 helpers.
//!
//! This is the one shared place for that rule: typed code of any family that needs the per-thread
//! precision choice uses these, so there is a single module for it. Keep it small.

use std::ops::{Add, Div, Mul, Neg, Sub};

/// Whether the running game thread computes at a float's precision: its x87 control word's precision
/// bits are 00 (main, once Direct3D set them).
#[inline]
pub(crate) fn float_precision() -> bool {
    engine_core::x87::control_word() & 0x0300 == 0
}

/// A float store of a double (`FSTP dword`): rounded to the nearest float.
#[allow(clippy::cast_possible_truncation)]
#[inline]
pub(crate) fn to_float(value: f64) -> f32 {
    value as f32
}

/// The type one function computes in on the running thread ([`float_precision`]): `f32` or `f64`.
pub(crate) trait Real:
    Copy
    + PartialOrd
    + Add<Output = Self>
    + Sub<Output = Self>
    + Mul<Output = Self>
    + Div<Output = Self>
    + Neg<Output = Self>
{
    const ZERO: Self;
    const ONE: Self;
    /// `FLD dword` of a float's bits (or one as a memory operand).
    fn load(bits: u32) -> Self;
    /// `FSTP dword`: rounded to a float, as its bits.
    fn store(self) -> u32;
    /// `FSQRT` at this precision.
    fn sqrt(self) -> Self;
    /// The value as a double, for a callee that takes one.
    fn wide(self) -> f64;
    /// A callee's double result at this precision (rounded to a float on main).
    fn narrow(value: f64) -> Self;
}

impl Real for f32 {
    const ZERO: f32 = 0.0;
    const ONE: f32 = 1.0;
    #[inline]
    fn load(bits: u32) -> f32 {
        f32::from_bits(bits)
    }
    #[inline]
    fn store(self) -> u32 {
        self.to_bits()
    }
    #[inline]
    fn sqrt(self) -> f32 {
        f32::sqrt(self)
    }
    #[inline]
    fn wide(self) -> f64 {
        f64::from(self)
    }
    #[inline]
    fn narrow(value: f64) -> f32 {
        to_float(value)
    }
}

impl Real for f64 {
    const ZERO: f64 = 0.0;
    const ONE: f64 = 1.0;
    #[inline]
    fn load(bits: u32) -> f64 {
        f64::from(f32::from_bits(bits))
    }
    #[inline]
    fn store(self) -> u32 {
        to_float(self).to_bits()
    }
    #[inline]
    fn sqrt(self) -> f64 {
        f64::sqrt(self)
    }
    #[inline]
    fn wide(self) -> f64 {
        self
    }
    #[inline]
    fn narrow(value: f64) -> f64 {
        value
    }
}
