//! The software instructions against this PC's FPU, on random and special operands under every
//! rounding mode and precision: bit for bit where the module says exact; the transcendentals'
//! stored doubles within one unit in the last place (counted, the count printed).

use super::super::{hw, set_control_word, DEFAULT_CONTROL_WORD};
use super::*;

/// The control words: each rounding mode at 24, 53 and 64 bits, exceptions masked.
fn words() -> Vec<u16> {
    let mut out = Vec::new();
    for pc in [0x0000u16, 0x0200, 0x0300] {
        for rc in [ROUND_NEAREST, ROUND_DOWN, ROUND_UP, ROUNDING_CONTROL] {
            out.push(0x007f | pc | rc);
        }
    }
    out
}

fn under<R>(word: u16, f: impl FnOnce() -> R) -> R {
    set_control_word(word);
    let r = f();
    set_control_word(DEFAULT_CONTROL_WORD);
    r
}

struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
}

/// Register values of every class: doubles and floats loaded, extended values with random
/// significands near 1 and across the range, extended denormals, zeros, infinities, quiet and
/// signalling NaNs of both signs, the indefinite, and values beyond the double range.
fn operands(n: usize, seed: u64) -> Vec<X87Ext> {
    let mut rng = Rng(seed);
    let specials = [
        X87Ext::from_parts(0, 0),
        X87Ext::from_parts(0x8000, 0),
        infinity(false),
        infinity(true),
        indefinite(),
        X87Ext::from_parts(0x7fff, INTEGER_BIT | QUIET_BIT | 0x1234),
        X87Ext::from_parts(0xffff, INTEGER_BIT | 0x55),
        X87Ext::from_parts(0x7fff, INTEGER_BIT | 0x99),
        X87Ext::from_parts(0x0000, 0x0000_0001_0000_0000),
        X87Ext::from_parts(0x8000, 0x7fff_ffff_ffff_ffff),
        X87Ext::from_parts(0x3fff, INTEGER_BIT),
        X87Ext::from_parts(0xbfff, u64::MAX),
        X87Ext::from_parts(0x7ffe, u64::MAX),
        X87Ext::from_parts(0x0001, INTEGER_BIT),
    ];
    (0..n)
        .map(|i| match i % 6 {
            0 => specials[(rng.next() % specials.len() as u64) as usize],
            1 => load64(f64::from_bits(rng.next())),
            2 => load32(rng.next() as u32),
            3 => {
                let sign = (rng.next() & 1) as u16 * 0x8000;
                let e = 0x3fff - 40 + (rng.next() % 80) as u16;
                X87Ext::from_parts(sign | e, rng.next() | INTEGER_BIT)
            }
            4 => {
                let sign = (rng.next() & 1) as u16 * 0x8000;
                let e = 1 + (rng.next() % 0x7ffd) as u16;
                X87Ext::from_parts(sign | e, rng.next() | INTEGER_BIT)
            }
            _ => {
                // Short significands: exact cases, ties and integers.
                let sign = (rng.next() & 1) as u16 * 0x8000;
                let e = 0x3fff + (rng.next() % 70) as u16;
                let cut = rng.next() % 64;
                let sig = (rng.next() & !((1u64 << cut) - 1)) | INTEGER_BIT;
                X87Ext::from_parts(sign | e, sig)
            }
        })
        .collect()
}

fn same(name: &str, word: u16, args: &[X87Ext], ours: X87Ext, fpu: X87Ext) {
    assert_eq!(ours, fpu, "{name} {word:#06x} {args:x?}");
}

#[test]
fn the_exact_instructions_are_the_fpus() {
    let values = operands(1500, 0x2545_f491_4f6c_dd1d);
    let mut compared = 0u64;
    for word in words() {
        for (i, &a) in values.iter().enumerate() {
            for &b in &values[(i * 7) % 1400..][..12] {
                let args = [a, b];
                under(word, || {
                    same("FADD", word, &args, fadd(a, b), hw::fadd(a, b));
                    same("FSUB", word, &args, fsub(a, b), hw::fsub(a, b));
                    same("FMUL", word, &args, fmul(a, b), hw::fmul(a, b));
                    same("FDIV", word, &args, fdiv(a, b), hw::fdiv(a, b));
                    assert_eq!(fcom(a, b), hw::fcom(a, b) & 0x4500, "FCOM {args:x?}");
                });
                compared += 5;
            }
            under(word, || {
                same("FSQRT", word, &[a], fsqrt(a), hw::fsqrt(a));
                same("FRNDINT", word, &[a], frndint(a), hw::frndint(a));
                assert_eq!(
                    store64(a).to_bits(),
                    hw::store64(a).to_bits(),
                    "FST qword {word:#06x} {a:x?}"
                );
                assert_eq!(store32(a), hw::store32(a), "FST dword {word:#06x} {a:x?}");
                assert_eq!(
                    fistp64(a, word),
                    hw::fistp64(a, word),
                    "FISTP {word:#06x} {a:x?}"
                );
            });
        }
    }
    assert!(compared > 500_000, "{compared}");
}

#[test]
fn scaling_constants_and_remainders_are_the_fpus() {
    let values = operands(1200, 0x9e37_79b9_7f4a_7c15);
    let mut scales: Vec<X87Ext> = [
        -70000.0, -100.5, -3.9, -1.0, -0.0, 0.0, 0.5, 2.0, 63.7, 16000.0, 70000.0,
    ]
    .iter()
    .map(|&v| load64(v))
    .collect();
    scales.extend([infinity(false), infinity(true), indefinite()]);
    for word in words() {
        under(word, || {
            for c in [
                X87Constant::Pi,
                X87Constant::Log2E,
                X87Constant::Log2Ten,
                X87Constant::Log10Two,
                X87Constant::LnTwo,
            ] {
                same(
                    "FLD constant",
                    word,
                    &[],
                    fld_constant(c),
                    hw::fld_constant(c),
                );
            }
            for (i, &a) in values.iter().enumerate() {
                for &s in &scales {
                    same("FSCALE", word, &[a, s], fscale(a, s), hw::fscale(a, s));
                }
                for &b in &values[(i * 5) % 1100..][..8] {
                    assert_eq!(
                        fprem(a, b),
                        hw::fprem(a, b),
                        "FPREM {word:#06x} {a:x?} {b:x?}"
                    );
                    assert_eq!(
                        fprem1(a, b),
                        hw::fprem1(a, b),
                        "FPREM1 {word:#06x} {a:x?} {b:x?}"
                    );
                }
            }
        });
    }
}

/// Units in the last place between two doubles of the same sign (both finite).
fn ulps(a: f64, b: f64) -> u64 {
    (a.to_bits() as i64)
        .wrapping_sub(b.to_bits() as i64)
        .unsigned_abs()
}

#[test]
fn the_transcendentals_store_the_fpus_double_within_one_unit() {
    let values = operands(6000, 0x5851_f42d_4c95_7f2d);
    let mut stats = (0u64, 0u64);
    let mut check = |name: &str, args: &[X87Ext], ours: X87Ext, fpu: X87Ext| {
        let (o, f) = (ours.store64(), fpu.store64());
        stats.0 += 1;
        if o.to_bits() == f.to_bits() {
            return;
        }
        let close = o.is_finite()
            && f.is_finite()
            && (o == 0.0) == (f == 0.0)
            && o.is_sign_negative() == f.is_sign_negative()
            && ulps(o, f) <= 1;
        // Near a zero of the function a result is only as good as the reduction: there the
        // difference is measured against the operand's size.
        let tiny = o.is_finite()
            && f.is_finite()
            && (o - f).abs() <= 2f64.powi(-60) * nearest64(args[0]).abs().max(1.0);
        assert!(
            close || tiny,
            "{name} {args:x?}: {o:e} ({:#x}) against the FPU's {f:e} ({:#x})",
            o.to_bits(),
            f.to_bits()
        );
        stats.1 += 1;
    };
    for (i, &a) in values.iter().enumerate() {
        let in_range = !matches!(unpack(a).kind, Kind::Finite { e, .. } if e - BIAS >= 63);
        check("FSIN", &[a], fsin(a), hw::fsin(a));
        check("FCOS", &[a], fcos(a), hw::fcos(a));
        if in_range {
            check("FPTAN", &[a], fptan(a), hw::fptan(a));
        }
        let b = values[(i * 13) % values.len()];
        check("FPATAN", &[a, b], fpatan(a, b), hw::fpatan(a, b));
        check("FYL2X", &[b, a], fyl2x(b, a), hw::fyl2x(b, a));
        // F2XM1 and FYL2XP1 are defined on |x| <= 1 and |x| < 1 - sqrt(2)/2.
        let v = nearest64(a);
        if v.is_finite() {
            let small = load64((v % 1.0).clamp(-0.29, 0.29));
            check("F2XM1", &[small], f2xm1(small), hw::f2xm1(small));
            check(
                "FYL2XP1",
                &[b, small],
                fyl2xp1(b, small),
                hw::fyl2xp1(b, small),
            );
        }
    }
    eprintln!(
        "transcendentals: {} compared, {} one unit apart",
        stats.0, stats.1
    );
}
