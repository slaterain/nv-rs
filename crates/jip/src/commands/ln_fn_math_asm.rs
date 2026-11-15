//! An independent check of `ln_fn_math`'s routines: JIP's asm (`internal/utility.cpp`: `Cos`,
//! `Cos_V3`, `GetSinCos`, `Tan`, `ASin`, `ACos`, `ATan2`; `internal/Ni_types.cpp`:
//! `Point2Distance`, `Point3Distance`) run instruction by instruction on emulated XMM registers
//! (four 32-bit lanes each, the flags of `comiss`), with JIP's constants written out again here
//! from its source. Each answer is compared with the port's routine bit for bit, and a few are
//! pinned to recorded bit patterns.
//
//! JIP's float literals are written as its source writes them, and the lanes are indexed as the asm
//! names them: two lints are allowed for that.
#![allow(clippy::excessive_precision, clippy::needless_range_loop)]

use super::*;
use crate::commands::test_support::{allowed_rsqrt_estimates, newton_step, processor_rsqrt};

type X = [u32; 4];

fn f(b: u32) -> f32 {
    f32::from_bits(b)
}
fn b(v: f32) -> u32 {
    v.to_bits()
}
/// `movss xmm, m32`: the low lane loaded, the others cleared.
fn load(v: u32) -> X {
    [v, 0, 0, 0]
}
fn addss(d: &mut X, s: u32) {
    d[0] = b(f(d[0]) + f(s));
}
fn subss(d: &mut X, s: u32) {
    d[0] = b(f(d[0]) - f(s));
}
fn mulss(d: &mut X, s: u32) {
    d[0] = b(f(d[0]) * f(s));
}
fn divss(d: &mut X, s: u32) {
    d[0] = b(f(d[0]) / f(s));
}
/// `comiss a, b`: (ZF, CF); unordered sets both (and PF).
fn comiss(a: u32, s: u32) -> (bool, bool) {
    let (x, y) = (f(a), f(s));
    if x.is_nan() || y.is_nan() {
        (true, true)
    } else {
        (x == y, x < y)
    }
}
/// `cvttss2si`: toward zero, the integer indefinite for NaN and past the range.
fn cvttss2si(v: u32) -> u32 {
    let x = f64::from(f(v));
    if x.is_nan() || x >= 2_147_483_648.0 || x <= -2_147_483_649.0 {
        0x8000_0000
    } else {
        x as i32 as u32
    }
}
/// `maxss d, s` and `minss d, s`: `d > s ? d : s`, `d < s ? d : s`.
fn maxss(d: &mut X, s: u32) {
    d[0] = if f(d[0]) > f(s) { d[0] } else { s };
}
fn minss(d: &mut X, s: u32) {
    d[0] = if f(d[0]) < f(s) { d[0] } else { s };
}

/// JIP's packed constants (`kPackedValues`, `internal/utility.cpp`): the masks and the floats.
const ABS_MASK0: X = [0x7fff_ffff, 0, 0, 0];
const FLIP_SIGN_MASK0: X = [0x8000_0000, 0, 0, 0];
const PS_ABS_MASK: X = [0x7fff_ffff; 4];
/// `FltPId2`, `FltPI`, `FltPIx2` as JIP's literals (1.570796371F, 3.141592741F, 6.283185482F),
/// `FLT_EPSILON`, 1.0, 3.0, 0.5.
fn pid2() -> u32 {
    b(1.570_796_371_f32)
}
fn pi() -> u32 {
    b(3.141_592_741_f32)
}
fn pix2() -> u32 {
    b(6.283_185_482_f32)
}
const FLT_EPSILON_BITS: u32 = 0x3400_0000;
const ONE: u32 = 0x3f80_0000;
/// `kFlt2dPI` and `kCosConsts` of `Cos` (the scalar routine's own copies).
const K_FLT_2D_PI: u32 = 0x3F22F983;
const K_COS: [u32; 5] = [0x37C23AB1, 0x3AB59551, 0x3D2AA76F, 0x3EFFFFE0, 0x3F7FFFFF];
/// `kATanConsts`.
const K_ATAN: [u32; 6] = [
    0xBC5CDD30, 0x3D6B6D55, 0x3DF84C31, 0x3E4854C9, 0x3EAA7E45, 0x3F7FFFB7,
];
/// `kASinConsts`, as JIP's source writes it.
#[rustfmt::skip]
const K_ASIN: [u32; 136] = [
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

fn and(d: &mut X, m: X) {
    for i in 0..4 {
        d[i] &= m[i];
    }
}
fn xor(d: &mut X, m: X) {
    for i in 0..4 {
        d[i] ^= m[i];
    }
}

/// `Cos` (scalar), line by line.
fn emu_cos(angle: f32) -> f32 {
    let mut x0: X = [b(angle), 0, 0, 0];
    and(&mut x0, ABS_MASK0); // andps xmm0, PS_AbsMask0
    let mut x2 = load(pix2()); // movss xmm2, PS_V3_PIx2
    let mut x1 = load(K_FLT_2D_PI); // movss xmm1, kFlt2dPI
    mulss(&mut x1, x0[0]); // mulss xmm1, xmm0
    let mut eax = cvttss2si(x1[0]); // cvttss2si eax, xmm1
    if (eax as i32) > 3 {
        // cmp eax, 3; jle perdOK
        let mut edx = eax;
        eax &= 3;
        edx >>= 2;
        x1 = [b(edx as i32 as f32), x1[1], x1[2], x1[3]]; // cvtsi2ss xmm1, edx
        mulss(&mut x1, x2[0]);
        subss(&mut x0, x1[0]);
    }
    let mut x3: X = [0; 4]; // xorps xmm3, xmm3
    let (zf, cf) = comiss(x0[0], x3[0]);
    if zf || cf {
        return f(ONE); // jbe retn1
    }
    let al = eax as u8;
    if al != 0 {
        // test al, al; jz doCalc
        if al.count_ones() % 2 != 0 {
            // jp fourthQ not taken (odd parity)
            x2 = load(pi());
            x3 = FLIP_SIGN_MASK0;
        }
        subss(&mut x0, x2[0]); // fourthQ: subss xmm0, xmm2
    }
    let s = x0[0];
    mulss(&mut x0, s); // mulss xmm0, xmm0
    let mut x1 = x0; // movq xmm1, xmm0
    mulss(&mut x1, K_COS[0]);
    subss(&mut x1, K_COS[1]);
    mulss(&mut x1, x0[0]);
    addss(&mut x1, K_COS[2]);
    mulss(&mut x1, x0[0]);
    subss(&mut x1, K_COS[3]);
    mulss(&mut x0, x1[0]);
    addss(&mut x0, K_COS[4]);
    xor(&mut x0, x3);
    f(x0[0])
}

/// `Cos_V3`, all four lanes (`pcmpgtd`, `movmskps`, `psrld`, `cvtdq2ps` as written).
fn emu_cos_v3(angles: [f32; 4]) -> [f32; 4] {
    let mut x0: X = angles.map(b);
    and(&mut x0, PS_ABS_MASK);
    let mut x1 = x0;
    let mut x2: X = [pix2(), pix2(), pix2(), 0]; // EMIT_PS_3
    for i in 0..4 {
        x1[i] = b(f(x1[i]) * f(if i < 3 { K_FLT_2D_PI } else { 0 })); // mulps kPS2dPI
        x1[i] = cvttss2si(x1[i]); // cvttps2dq
    }
    let mut x3 = x1;
    let quad: X = [3, 3, 3, 0];
    for i in 0..4 {
        x3[i] = if (x3[i] as i32) > (quad[i] as i32) {
            u32::MAX
        } else {
            0
        };
    }
    if x3.iter().any(|&m| m != 0) {
        let mut t = x1;
        and(&mut x1, quad);
        for (i, v) in t.iter_mut().enumerate() {
            *v = b(((*v >> 2) as i32 as f32) * f(x2[i])); // psrld; cvtdq2ps; mulps xmm3, xmm2
        }
        for i in 0..4 {
            x0[i] = b(f(x0[i]) - f(t[i]));
        }
    }
    let x3 = x1;
    let two: X = [2, 2, 2, 0];
    let mut m: X = [0; 4];
    for i in 0..4 {
        m[i] = if (x1[i] as i32) > (two[i] as i32) {
            u32::MAX
        } else {
            0
        }; // pcmpgtd
    }
    and(&mut x2, m); // andps xmm2, xmm1
    let mut n: X = [0; 4];
    for i in 0..4 {
        n[i] = !m[i] & x3[i]; // andnps xmm1, xmm3
        n[i] = if (n[i] as i32) > 0 { u32::MAX } else { 0 }; // pcmpgtd xmm1, 0
    }
    let sign: X = n.map(|v| v << 31); // pslld xmm3, 0x1F
    let mut off: X = [0; 4];
    for i in 0..4 {
        off[i] = (n[i] & if i < 3 { pi() } else { 0 }) | x2[i];
        x0[i] = b(f(x0[i]) - f(off[i]));
    }
    let mut out = [0f32; 4];
    for i in 0..4 {
        let c = |k: usize| if i < 3 { f(K_COS[k]) } else { 0.0 };
        let x = f(x0[i]) * f(x0[i]);
        let mut p = c(0) * x;
        p -= c(1);
        p *= x;
        p += c(2);
        p *= x;
        p -= c(3);
        let r = x * p + c(4);
        out[i] = f(b(r) ^ sign[i]);
    }
    out
}

/// `Tan` through `GetSinCos` (lanes pi/2 - angle and angle).
fn emu_tan(angle: f32) -> f32 {
    let mut x0 = load(pid2());
    subss(&mut x0, b(angle));
    let lanes = emu_cos_v3([f(x0[0]), angle, 0.0, 0.0]);
    let (s, c) = (lanes[0], lanes[1]);
    let mut x2: X = [b(c), 0, 0, 0];
    and(&mut x2, ABS_MASK0);
    let (_, cf) = comiss(x2[0], FLT_EPSILON_BITS);
    if cf {
        return 0.0;
    }
    let mut r = load(b(s));
    divss(&mut r, b(c));
    f(r[0])
}

/// `ASin`, line by line.
fn emu_asin(x: f32) -> f32 {
    let mut x0: X = [b(x), 0, 0, 0];
    let (zf, _) = comiss(x0[0], 0);
    if zf {
        return f(x0[0]);
    }
    let mut x4 = x0;
    and(&mut x4, FLIP_SIGN_MASK0);
    xor(&mut x0, x4);
    let mut x3 = load(K_ASIN[5]);
    let (_, cf) = comiss(x0[0], x3[0]);
    if !cf {
        let mut r = load(K_ASIN[3]);
        xor(&mut r, x4);
        return f(r[0]);
    }
    let x1i = K_ASIN[7].wrapping_add(x0[0]); // paddd
    let mut x2: X = [K_ASIN[0], K_ASIN[1], K_ASIN[2], K_ASIN[3]];
    let eax = cvttss2si(x1i) as usize;
    let mut x1: X = [x2[0], x2[1], 0, 0]; // movq
    mulss(&mut x1, x0[0]);
    x2 = [x2[1], x2[2], x2[3], 0]; // psrldq 4
    addss(&mut x1, x2[0]);
    mulss(&mut x1, x0[0]);
    x2 = [x2[1], x2[2], x2[3], 0];
    subss(&mut x1, x2[0]);
    mulss(&mut x1, x0[0]);
    subss(&mut x3, x0[0]);
    let mut x0 = [x2[1], x2[1], x2[3], x2[3]]; // movshdup
    addss(&mut x1, x0[0]);
    let mut x2 = [b(f(x3[0]).sqrt()), x2[1], x2[2], x2[3]]; // sqrtss
    mulss(&mut x2, x1[0]);
    subss(&mut x0, x2[0]);
    addss(&mut x0, K_ASIN[eax + 8]);
    xor(&mut x0, x4);
    f(x0[0])
}

/// `ACos`, line by line.
fn emu_acos(x: f32) -> f32 {
    let mut x0: X = [b(x), 0, 0, 0];
    let (zf, _) = comiss(x0[0], 0);
    if zf {
        return f(K_ASIN[3]);
    }
    let mut x4 = x0;
    and(&mut x4, FLIP_SIGN_MASK0);
    xor(&mut x0, x4);
    let mut x3 = load(K_ASIN[5]);
    let (_, cf) = comiss(x0[0], x3[0]);
    if !cf {
        let m = ((x4[0] as i32) >> 31) as u32; // psrad xmm4, 0x1F
        return f(K_ASIN[6] & m);
    }
    let x1i = K_ASIN[7].wrapping_add(x0[0]);
    let mut x2: X = [K_ASIN[0], K_ASIN[1], K_ASIN[2], K_ASIN[3]];
    let eax = cvttss2si(x1i) as usize;
    let mut x1: X = [x2[0], x2[1], 0, 0];
    mulss(&mut x1, x0[0]);
    x2 = [x2[1], x2[2], x2[3], 0];
    addss(&mut x1, x2[0]);
    mulss(&mut x1, x0[0]);
    x2 = [x2[1], x2[2], x2[3], 0];
    subss(&mut x1, x2[0]);
    mulss(&mut x1, x0[0]);
    x2 = [x2[1], x2[2], x2[3], 0];
    addss(&mut x1, x2[0]);
    subss(&mut x3, x0[0]);
    let mut x0 = load(b(f(x3[0]).sqrt()));
    mulss(&mut x0, x1[0]);
    subss(&mut x0, K_ASIN[eax + 8]);
    if x4[0] >> 31 != 0 {
        // movmskps eax, xmm4; test al, al
        xor(&mut x0, x4);
        addss(&mut x0, K_ASIN[6]);
    }
    f(x0[0])
}

/// `ATan2`, line by line.
fn emu_atan2(y: f32, xv: f32) -> f32 {
    let mut x0: X = [b(y), 0, 0, 0];
    let x1: X = [b(xv), 0, 0, 0];
    let (zf, _) = comiss(x0[0], 0);
    if zf {
        let (_, cf) = comiss(x1[0], 0);
        return if !cf { f(x0[0]) } else { f(pi()) };
    }
    let (zf, _) = comiss(x1[0], 0);
    if zf {
        let mut r = x0;
        and(&mut r, FLIP_SIGN_MASK0);
        r[0] |= pid2();
        return f(r[0]);
    }
    let mut x2: X = [x0[0], x0[1], x1[0], x1[1]]; // movq; unpcklpd
    and(&mut x2, PS_ABS_MASK);
    let mut x3 = x2;
    let mut x4: X = [x2[2], x2[3], x2[3], x2[3]]; // pshufd 0xFE
    maxss(&mut x3, x4[0]);
    minss(&mut x4, x2[0]);
    divss(&mut x4, x3[0]);
    let mut x3 = x4;
    mulss(&mut x3, x4[0]);
    let mut x5 = x3;
    mulss(&mut x5, K_ATAN[0]);
    addss(&mut x5, K_ATAN[1]);
    mulss(&mut x5, x3[0]);
    subss(&mut x5, K_ATAN[2]);
    mulss(&mut x5, x3[0]);
    addss(&mut x5, K_ATAN[3]);
    mulss(&mut x5, x3[0]);
    subss(&mut x5, K_ATAN[4]);
    mulss(&mut x5, x3[0]);
    addss(&mut x5, K_ATAN[5]);
    mulss(&mut x4, x5[0]);
    let flip = FLIP_SIGN_MASK0;
    let (zf, cf) = comiss(x2[0], x2[2]);
    if !(zf || cf) {
        xor(&mut x4, flip);
        addss(&mut x4, pid2());
    }
    let (_, cf) = comiss(x1[0], 0);
    if cf {
        xor(&mut x4, flip);
        addss(&mut x4, pi());
    }
    and(&mut x0, flip);
    xor(&mut x0, x4);
    f(x0[0])
}

/// `rsqrtss` and JIP's Newton step, as `Point2Distance`/`Point3Distance` end.
fn emu_sqrt(s: f32) -> f32 {
    let mut x0 = load(b(s));
    let (zf, _) = comiss(x0[0], 0);
    if zf {
        return f(x0[0]);
    }
    #[cfg(target_arch = "x86_64")]
    let r = {
        use std::arch::x86_64::{_mm_cvtss_f32, _mm_rsqrt_ss, _mm_set_ss};
        // SAFETY: SSE is part of every x86_64 processor.
        unsafe { _mm_cvtss_f32(_mm_rsqrt_ss(_mm_set_ss(s))) }
    };
    #[cfg(not(target_arch = "x86_64"))]
    let r = 1.0 / s.sqrt();
    let mut x1 = x0;
    mulss(&mut x1, b(r));
    mulss(&mut x1, b(r));
    let mut x3 = load(b(3.0));
    subss(&mut x3, x1[0]);
    mulss(&mut x3, b(r));
    mulss(&mut x3, b(0.5));
    mulss(&mut x0, x3[0]);
    f(x0[0])
}

fn same(a: f32, e: f32, what: &str) {
    assert_eq!(a.to_bits(), e.to_bits(), "{what}: port {a:e}, asm {e:e}");
}

#[test]
fn the_tables_are_jips() {
    assert_eq!(ASIN_CONSTS, K_ASIN);
    assert_eq!(COS_CONSTS, K_COS);
    assert_eq!(ATAN_CONSTS, K_ATAN);
    assert_eq!(FLT_2_D_PI.to_bits(), K_FLT_2D_PI);
    assert_eq!(
        (FLT_PI_D2.to_bits(), FLT_PI.to_bits(), FLT_PI_X2.to_bits()),
        (pid2(), pi(), pix2())
    );
    // JIP's FltPId180 (0.01745329238F) and Dbl180dPI (57.29577951308232).
    assert_eq!(super::FLT_PI_D_180.to_bits(), 0x3c8e_fa35);
    assert_eq!(
        super::FLT_PI_D_180.to_bits(),
        0.017_453_292_38_f32.to_bits()
    );
    assert_eq!(
        super::DBL_180_D_PI.to_bits(),
        57.295_779_513_082_32_f64.to_bits()
    );
}

#[test]
fn cosine_and_sine_bit_for_bit() {
    // Quadrants 0, 1, 2, 3, past 3 (whole turns off), negative angles, the early answer.
    for a in [
        0.3f32,
        1.9,
        3.5,
        5.2,
        12.0,
        100.7,
        -2.2,
        -7.9,
        0.0,
        1e-30,
        6.283_185_5,
        1e9,
    ] {
        same(cos(a), emu_cos(a), &format!("cos {a}"));
        same(sin(a), emu_cos(pid2_f() - a), &format!("sin {a}"));
    }
    for a in [0.3f32, 1.9, 3.5, 5.2, 12.0, -2.2, 0.0, f32::NAN] {
        let lane = emu_cos_v3([a, 0.0, 0.0, 0.0])[0];
        same(cos_lane(a), lane, &format!("lane {a}"));
    }
    // Three lanes at once: one lane's wrap reduces the others, an indefinite lane included.
    for lanes in [
        [f(0x4f49_0fdc), 7.0, 0.0],
        [5.0, 13.0, -1e10],
        [1e10, f32::NAN, 3.0],
        [f32::NAN, 7.0, -2.2],
        [0.3, 1.9, 3.5],
        [1e9, -7.9, 12.0],
        [f32::INFINITY, 100.7, 0.0],
    ] {
        let want = emu_cos_v3([lanes[0], lanes[1], lanes[2], 0.0]);
        let got = cos_v3(lanes);
        for i in 0..3 {
            same(got[i], want[i], &format!("lane {i} of {lanes:?}"));
        }
    }
}

fn pid2_f() -> f32 {
    f(pid2())
}

#[test]
fn tangent_bit_for_bit() {
    for a in [
        0.3f32,
        1.0,
        1.9,
        2.8,
        4.0,
        5.5,
        8.0,
        -1.2,
        -3.3,
        0.0,
        1.570_796_4,
    ] {
        same(tan(a), emu_tan(a), &format!("tan {a}"));
    }
}

#[test]
fn inverse_bit_for_bit() {
    // Table indices 0, 1, 64, 100, 127, both signs, the edges.
    for v in [
        0.003f32,
        0.01,
        0.5,
        0.78,
        0.999,
        0.999_999_9,
        0.0,
        -0.0,
        1.0,
        2.0,
    ] {
        for x in [v, -v] {
            same(asin(x), emu_asin(x), &format!("asin {x}"));
            same(acos(x), emu_acos(x), &format!("acos {x}"));
        }
    }
    // Four quadrants, |y| larger and smaller, the zero cases.
    for (y, x) in [
        (1.0f32, 3.0f32),
        (3.0, 1.0),
        (1.0, -3.0),
        (3.0, -1.0),
        (-1.0, -3.0),
        (-3.0, -1.0),
        (-1.0, 3.0),
        (-3.0, 1.0),
        (2.0, 2.0),
        (0.0, -1.0),
        (-0.0, 1.0),
        (5.0, 0.0),
    ] {
        same(atan2(y, x), emu_atan2(y, x), &format!("atan2 {y} {x}"));
    }
}

/// Answers recorded from the emulation (2026-11-14), pinned on the port's routines: a change to both
/// shows here too.
#[test]
fn recorded_bits() {
    let pins = [
        (cos(1.9), 0xbea5_8634),
        (cos(5.2), 0x3eef_e160),
        (cos(12.0), 0x3f58_06ce),
        (tan(1.0), 0x3fc7_5923),
        (asin(0.5), 0x3f06_0a99),
        (asin(-0.999), 0xbfc3_5654),
        (acos(0.78), 0x3f2d_16e7),
        (acos(-0.01), 0x3fca_57a9),
        (atan2(3.0, -1.0), 0x3ff2_3f00),
        (atan2(-1.0, 3.0), 0xbea4_bc94),
    ];
    for (i, (v, bits)) in pins.iter().enumerate() {
        assert_eq!(v.to_bits(), *bits, "pin {i}");
    }
}

#[test]
fn distances_bit_for_bit() {
    let sums = [2.0f32, 3.0, 1_000_001.0, 1e-3, 0.0, 7.0, 169.0];
    for s in sums {
        same(newton_sqrt(s), emu_sqrt(s), &format!("sqrt {s}"));
        if s != 0.0 {
            // The asm's arithmetic is the step written out on this processor's estimate.
            same(
                newton_step(s, processor_rsqrt(s)),
                emu_sqrt(s),
                &format!("step {s}"),
            );
        }
    }
    // The estimate and one step differ from sqrt on every processor: for 169 no estimate the
    // specification allows lands the step on 13 (whether the other sums land on their rounded
    // roots depends on the processor's table).
    let allowed = allowed_rsqrt_estimates(169.0);
    assert!(allowed.contains(&processor_rsqrt(169.0)));
    assert!(allowed.iter().all(|&r| newton_step(169.0, r) != 13.0));
    assert_ne!(newton_sqrt(169.0), 169f32.sqrt());
    let a = [1.5f32, -2.25, 3.0];
    let c = [-4.0f32, 7.5, 0.125];
    let (dx, dy, dz) = (a[0] - c[0], a[1] - c[1], a[2] - c[2]);
    same(point2_distance(a, c), emu_sqrt(dx * dx + dy * dy), "point2");
    same(
        point3_distance(a, c),
        emu_sqrt((dx * dx + dy * dy) + dz * dz),
        "point3",
    );
}
