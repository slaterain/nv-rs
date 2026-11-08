//! The arithmetic D3DX's texture code does, as it does it.
//!
//! The 32-bit `d3dx9_38.dll` computes on the x87 unit, in the precision the thread has. The
//! game (and the capture of the reference answers) made its Direct3D 9 device without
//! `D3DCREATE_FPU_PRESERVE`, which sets single precision (measured with `d3dx-measure
//! control-word`: 0x27f before `CreateDevice`, 0x7f after; the DLL's own block encoders called
//! at 0x7f reproduce the reference blocks, at 0x27f or 0x37f they do not). In single precision
//! every x87 operation rounds to a 24-bit significand: each is the `f32` operation. Every
//! constant the DLL loads (as a double) is a value an `f32` holds exactly.
//!
//! The DLL's encoders set the rounding control to "toward zero" (they OR 0xC00 into the control
//! word) around their loops, for their integer conversions; every add, subtract, multiply and
//! divide in those loops then truncates too. That arithmetic is [`Z`]'s. Everything else rounds
//! to nearest (`f32`).
//!
//! The order of operations is the DLL's: a sum `a + (b + c)` is not `(a + b) + c` in floating
//! point.

use std::ops::{Add, Div, Mul, Neg, Sub};

/// `FISTP` of a 32-bit integer with the rounding control set to truncate: towards zero, and
/// the "integer indefinite" `0x80000000` for a NaN or a value out of range.
#[inline(always)]
pub fn fistp(x: f32) -> i32 {
    if x.is_nan() || x >= 2_147_483_648.0 || x < -2_147_483_648.0 {
        i32::MIN
    } else {
        x as i32
    }
}

/// `s`, the nearest `f32` to an exact result, made the result rounded towards zero: the one
/// before it when the exact result is nearer zero (`below`; never for an exact zero, an
/// infinity or a NaN, whose errors are zero or NaN). Round-to-nearest is within half a unit, so
/// truncation is `s` or its neighbour towards zero, one step down in magnitude: its bits minus 1.
#[inline(always)]
fn chop(s: f32, below: bool) -> f32 {
    f32::from_bits(s.to_bits().wrapping_sub(u32::from(below)))
}

/// `a + b` rounded towards zero: the nearest sum and its exact error (TwoSum, exact in `f32`);
/// an error pointing towards zero moves the sum one step that way.
#[inline]
pub fn add_z(a: f32, b: f32) -> f32 {
    let s = a + b;
    let bb = s - a;
    let e = (a - (s - bb)) + (b - bb);
    chop(s, e != 0.0 && (e < 0.0) != (s < 0.0))
}

/// `a * b` rounded towards zero (the product of two `f32`s is exact in `f64`).
#[inline]
pub fn mul_z(a: f32, b: f32) -> f32 {
    let p = f64::from(a) * f64::from(b);
    let s = p as f32;
    chop(s, f64::from(s).abs() > p.abs())
}

/// `a / b` rounded towards zero: the nearest quotient `q` is too far out when `|q b| > |a|`
/// (`q b` is exact in `f64`).
#[inline]
pub fn div_z(a: f32, b: f32) -> f32 {
    let q = a / b;
    chop(q, (f64::from(q) * f64::from(b)).abs() > f64::from(a).abs())
}

/// The processor's own rounding towards zero (x86-64: SSE with MXCSR's rounding control set to
/// truncate by [`Rz`]). Each operation is one instruction in its own `asm!` block: the compiler
/// can neither fold it with its round-to-nearest assumptions nor move it out of the region the
/// guard sets, since it keeps such blocks in order with the guard's. The functions that use them
/// are not inlined into code that rounds to nearest, and use no other floating point arithmetic.
#[cfg(all(target_arch = "x86_64", not(feature = "software-truncation")))]
mod hw {
    use std::arch::asm;

    const RC_TRUNCATE: u32 = 0x6000;

    /// MXCSR's rounding control set to truncate until dropped.
    pub struct Rz(u32);

    impl Rz {
        #[inline(always)]
        pub fn new() -> Rz {
            let mut old = 0u32;
            // SAFETY: STMXCSR/LDMXCSR of a local; only the rounding control bits change.
            unsafe {
                asm!("stmxcsr [{p}]", p = in(reg) &mut old, options(nostack));
                let new = old | RC_TRUNCATE;
                asm!("ldmxcsr [{p}]", p = in(reg) &new, options(nostack, readonly));
            }
            Rz(old)
        }
    }

    impl Drop for Rz {
        #[inline(always)]
        fn drop(&mut self) {
            // SAFETY: restores the MXCSR read in `new`.
            unsafe { asm!("ldmxcsr [{p}]", p = in(reg) &self.0, options(nostack, readonly)) }
        }
    }

    /// Whether the rounding control truncates now (debug builds check every operation).
    #[cfg(debug_assertions)]
    fn truncating() -> bool {
        let mut m = 0u32;
        // SAFETY: STMXCSR of a local.
        unsafe { asm!("stmxcsr [{p}]", p = in(reg) &mut m, options(nostack)) };
        m & RC_TRUNCATE == RC_TRUNCATE
    }

    macro_rules! op {
        ($name:ident, $ins:literal) => {
            #[inline(always)]
            pub fn $name(a: f32, b: f32) -> f32 {
                #[cfg(debug_assertions)]
                debug_assert!(truncating(), "a truncating operation outside an Rz region");
                let mut x = a;
                // SAFETY: one SSE arithmetic instruction on registers.
                unsafe { asm!(concat!($ins, " {x}, {b}"), x = inout(xmm_reg) x, b = in(xmm_reg) b, options(nomem, nostack)) };
                x
            }
        };
    }
    op!(add, "addss");
    op!(sub, "subss");
    op!(mul, "mulss");
    op!(div, "divss");
}

/// Truncating arithmetic in force for the guard's life: on x86-64 the processor's rounding
/// control (restored when dropped); elsewhere nothing, [`Z`] truncating in software. Every
/// [`Z`] operation runs inside one.
#[cfg(all(target_arch = "x86_64", not(feature = "software-truncation")))]
pub use hw::Rz;

/// Truncating arithmetic in force for the guard's life (software here: nothing to set).
#[cfg(any(not(target_arch = "x86_64"), feature = "software-truncation"))]
pub struct Rz;

#[cfg(any(not(target_arch = "x86_64"), feature = "software-truncation"))]
impl Rz {
    #[inline(always)]
    pub fn new() -> Rz {
        Rz
    }
}

impl Default for Rz {
    fn default() -> Rz {
        Rz::new()
    }
}

/// A value computed while the DLL's rounding control truncates: its operators round towards
/// zero. Used only inside an [`Rz`] region.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Z(pub f32);

#[cfg(all(target_arch = "x86_64", not(feature = "software-truncation")))]
use hw::{add as zadd, div as zdiv, mul as zmul, sub as zsub};

#[cfg(any(not(target_arch = "x86_64"), feature = "software-truncation"))]
#[inline(always)]
fn zadd(a: f32, b: f32) -> f32 {
    add_z(a, b)
}
#[cfg(any(not(target_arch = "x86_64"), feature = "software-truncation"))]
#[inline(always)]
fn zsub(a: f32, b: f32) -> f32 {
    add_z(a, -b)
}
#[cfg(any(not(target_arch = "x86_64"), feature = "software-truncation"))]
#[inline(always)]
fn zmul(a: f32, b: f32) -> f32 {
    mul_z(a, b)
}
#[cfg(any(not(target_arch = "x86_64"), feature = "software-truncation"))]
#[inline(always)]
fn zdiv(a: f32, b: f32) -> f32 {
    div_z(a, b)
}

impl Add for Z {
    type Output = Z;
    #[inline(always)]
    fn add(self, o: Z) -> Z {
        Z(zadd(self.0, o.0))
    }
}

impl Sub for Z {
    type Output = Z;
    #[inline(always)]
    fn sub(self, o: Z) -> Z {
        Z(zsub(self.0, o.0))
    }
}

impl Mul for Z {
    type Output = Z;
    #[inline(always)]
    fn mul(self, o: Z) -> Z {
        Z(zmul(self.0, o.0))
    }
}

impl Div for Z {
    type Output = Z;
    #[inline(always)]
    fn div(self, o: Z) -> Z {
        Z(zdiv(self.0, o.0))
    }
}

impl Neg for Z {
    type Output = Z;
    #[inline(always)]
    fn neg(self) -> Z {
        Z(-self.0)
    }
}

/// `1/31`, `1/63`, `1/255`, `1/15`, `1/7`, `1/5`, `1/3`, `2/3` as the DLL holds them (each the
/// `f32` nearest the fraction).
pub const INV31: f32 = 1.0 / 31.0;
pub const INV63: f32 = 1.0 / 63.0;
pub const INV255: f32 = 1.0 / 255.0;
pub const INV15: f32 = 1.0 / 15.0;
pub const INV7: f32 = 1.0 / 7.0;
pub const INV5: f32 = 1.0 / 5.0;
pub const THIRD: f32 = 1.0 / 3.0;
pub const TWO_THIRDS: f32 = 2.0 / 3.0;

/// The error diffusion weights (Floyd-Steinberg) the DLL's encoders use.
pub const W7: f32 = 7.0 / 16.0;
pub const W3: f32 = 3.0 / 16.0;
pub const W5: f32 = 5.0 / 16.0;
pub const W1: f32 = 1.0 / 16.0;

#[cfg(test)]
mod tests {
    use super::*;

    /// The constants are the DLL's (its doubles at 0x407c10, 0x407c00, 0x4014f8, 0x407e58,
    /// 0x407ee0, 0x408128, 0x407e70, 0x405bd0), read from its 32-bit `d3dx9_38.dll`.
    #[test]
    fn constants_are_the_dlls() {
        let dll = [
            (INV31, 0x3fa0_8421_0000_0000u64),
            (INV63, 0x3f90_4104_2000_0000),
            (INV255, 0x3f70_1010_2000_0000),
            (INV15, 0x3fb1_1111_2000_0000),
            (INV7, 0x3fc2_4924_a000_0000),
            (INV5, 0x3fc9_9999_a000_0000),
            (THIRD, 0x3fd5_5555_6000_0000),
            (TWO_THIRDS, 0x3fe5_5555_6000_0000),
        ];
        for (ours, theirs) in dll {
            assert_eq!(f64::from(ours).to_bits(), theirs);
        }
    }

    #[test]
    fn fistp_truncates_and_gives_the_indefinite_value() {
        assert_eq!(fistp(2.9), 2);
        assert_eq!(fistp(-2.9), -2);
        assert_eq!(fistp(f32::NAN), i32::MIN);
        assert_eq!(fistp(3e9), i32::MIN);
    }

    /// The processor's truncation (x86-64) and the software one agree, on values around the
    /// awkward places (halfway cases, cancellation, tiny against large, powers of two).
    #[test]
    fn processor_and_software_truncation_agree() {
        let mut seed = 0x9e37_79b9_7f4a_7c15u64;
        let mut next = || {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            seed
        };
        let pick = |r: u64| -> f32 {
            match r % 6 {
                0 => f32::from_bits(r as u32 >> 2 | 0x3c00_0000),
                1 => (r % 1000) as f32 / 255.0,
                2 => 2f32.powi((r % 40) as i32 - 20),
                3 => -f32::from_bits((r >> 8) as u32 & 0x3fff_ffff),
                4 => f32::from_bits(0x3f80_0000 + (r % 5) as u32 - 2),
                _ => (r % 64) as f32 / 63.0 - 0.5,
            }
        };
        let pairs: Vec<(f32, f32)> = (0..200_000).map(|_| (pick(next()), pick(next()))).collect();
        // The software answers, rounding to nearest underneath.
        let soft: Vec<[u32; 4]> = pairs
            .iter()
            .map(|&(a, b)| {
                let q = if b != 0.0 { div_z(a, b).to_bits() } else { 0 };
                [
                    add_z(a, b).to_bits(),
                    add_z(a, -b).to_bits(),
                    mul_z(a, b).to_bits(),
                    q,
                ]
            })
            .collect();
        let hard: Vec<[u32; 4]> = {
            let _rz = Rz::new();
            pairs
                .iter()
                .map(|&(a, b)| {
                    let q = if b != 0.0 {
                        (Z(a) / Z(b)).0.to_bits()
                    } else {
                        0
                    };
                    [
                        (Z(a) + Z(b)).0.to_bits(),
                        (Z(a) - Z(b)).0.to_bits(),
                        (Z(a) * Z(b)).0.to_bits(),
                        q,
                    ]
                })
                .collect()
        };
        for (i, (s, h)) in soft.iter().zip(&hard).enumerate() {
            assert_eq!(s, h, "{:?}", pairs[i]);
        }
    }

    /// Rounding towards zero, against exact rational arithmetic on the f64 of each pair.
    #[test]
    fn chopped_operations_truncate() {
        let one_plus = f32::from_bits(0x3f80_0001);
        // 1 + 2^-24 is halfway: nearest rounds to even (1), towards zero gives 1.
        assert_eq!(add_z(1.0, 2f32.powi(-24)), 1.0);
        // 1 + 1.5 * 2^-23 is above 1 + 2^-23: nearest gives 1 + 2^-22, towards zero 1 + 2^-23.
        assert_eq!(add_z(1.0, 1.5 * 2f32.powi(-23)), one_plus);
        // 1 - tiny is below 1: towards zero gives the f32 below 1.
        assert_eq!(add_z(1.0, -1e-30), f32::from_bits(0x3f7f_ffff));
        assert_eq!(add_z(-1.0, 1e-30), -f32::from_bits(0x3f7f_ffff));
        // 1/3 truncated is below the nearest 1/3 (which rounds up).
        assert_eq!(div_z(1.0, 3.0), f32::from_bits(0x3eaa_aaaa));
        assert_eq!(div_z(2.0, 3.0), f32::from_bits(0x3f2a_aaaa));
        assert_eq!(div_z(1.0, 4.0), 0.25);
        assert_eq!(mul_z(one_plus, one_plus), f32::from_bits(0x3f80_0002));
        // (1 - 2^-24)^2 = 1 - 2^-23 + 2^-48: towards zero 1 - 2^-23.
        assert_eq!(
            mul_z(f32::from_bits(0x3f7f_ffff), f32::from_bits(0x3f7f_ffff)),
            f32::from_bits(0x3f7f_fffe)
        );
        // 3 * (1/3 rounded up) is above 1: towards zero gives 1, nearest too; 3 * 0x3eaaaaab
        // = 1 + 2^-24 exactly, below the next f32.
        assert_eq!(mul_z(3.0, f32::from_bits(0x3eaa_aaab)), 1.0);
    }
}
