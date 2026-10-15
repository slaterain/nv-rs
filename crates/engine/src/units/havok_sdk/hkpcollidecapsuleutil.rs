//! `Havok SDK/hkpcollidecapsuleutil.cpp` (Xbox PDB source unit), subsystem `Havok SDK`: its
//! functions in FalloutNV.exe 1.4.0.525, translated (docs/ENGINE_CRATE.md).
//!
//! The vectors are SSE (`f32` lanes, three-component sums added as `z + (y + x)`, `RSQRTSS`
//! refined by one Newton step); the scalar solving runs on the x87 stack, approximated here in
//! `f64` with a rounding to `f32` wherever the code stores a `float`.

#[allow(unused_imports)]
use crate::prelude::*;
use crate::units::fallout_shared::tesobjectrefr::rsqrt_estimate;

/// The CRT square root (`00ec6040`, argument in ST0).
const X87_SQRT: u32 = 0x00ec_6040;

/// `FLT_EPSILON`, 2^-23 (the float at `0104da4c` and the double at `010c9300`).
const EPSILON: f32 = 1.192_092_9e-7;
/// The double 2^-20 at `010cb420`.
const TWO_POW_MINUS_20: f64 = 9.536_743_164_062_5e-7;
/// The double at `01018a90`: 0.2 rounded to a `float`.
const FIFTH: f64 = 0.200_000_002_980_232_24;

type Vec4 = [f32; 4];

fn load(e: &Engine, at: u32) -> Vec4 {
    [
        e.mem.f32(at),
        e.mem.f32(at + 4),
        e.mem.f32(at + 8),
        e.mem.f32(at + 12),
    ]
}

fn store(e: &mut Engine, at: u32, value: Vec4) {
    for (lane, component) in value.into_iter().enumerate() {
        e.mem.set_f32(at + 4 * lane as u32, component);
    }
}

fn sub(a: Vec4, b: Vec4) -> Vec4 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2], a[3] - b[3]]
}

/// The sum of the three lane products added as `z + (y + x)`.
fn dot3(a: Vec4, b: Vec4) -> f32 {
    let p = [a[0] * b[0], a[1] * b[1], a[2] * b[2]];
    p[2] + (p[1] + p[0])
}

/// `scale * direction + base` in every lane (`MULPS` then `ADDPS`).
fn scaled_add(scale: f32, direction: Vec4, base: Vec4) -> Vec4 {
    [
        scale * direction[0] + base[0],
        scale * direction[1] + base[1],
        scale * direction[2] + base[2],
        scale * direction[3] + base[3],
    ]
}

/// XORPS with the sign mask: every lane negated.
fn negate(v: Vec4) -> Vec4 {
    v.map(|lane| f32::from_bits(lane.to_bits() ^ 0x8000_0000))
}

/// The vector scaled to unit length with `RSQRTSS` and one Newton step,
/// `(0.5 * r) * (3 - (sum * r) * r)`; a zero vector gives zeros.
fn normalize_estimate(v: Vec4) -> Vec4 {
    let sum = dot3(v, v);
    let estimate = rsqrt_estimate(sum);
    let factor = if sum == 0.0 {
        0.0
    } else {
        (0.5 * estimate) * (3.0 - (sum * estimate) * estimate)
    };
    v.map(|lane| factor * lane)
}

// Translated from 00cd58b0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `hkCollideCapsuleUtilManifoldCapsVsCaps` (Xbox PDB), cdecl
/// `(capsule_a, radius_a, capsule_b, radius_b, out)`. A capsule is two 16-byte points, its
/// axis. `out` receives up to three contacts of 0x20 bytes (position at +0, normal at +0x10 and
/// the signed gap in the normal's fourth lane); the first holds the closest approach of the two
/// axes and the other two the ends of the overlap when the axes are almost parallel. The
/// result is a bit set of the clamped axis parameters: bit 0 `s` = 1, bit 1 `s` = 0 (A's
/// parameter), bit 2 `t` = 1, bit 3 `t` = 0 (B's).
///
/// The gap already in the first contact (`out + 0x1c`, left by the caller) is raised to
/// `FLT_EPSILON`; when the axes are at least that far apart beyond the radii (gap above it)
/// the call only stores the difference scaled by `1 / distance` as the normal and the new gap,
/// leaving the contact position alone.
#[allow(clippy::neg_cmp_op_on_partial_ord)]
pub fn hk_collide_capsule_util_manifold_caps_vs_caps(
    e: &mut Engine,
    capsule_a: Ptr,
    radius_a: f32,
    capsule_b: Ptr,
    radius_b: f32,
    out: Ptr,
) -> u32 {
    let a_start = load(e, capsule_a.addr());
    let a_end = load(e, capsule_a.addr() + 0x10);
    let b_start = load(e, capsule_b.addr());
    let b_end = load(e, capsule_b.addr() + 0x10);
    let out = out.addr();

    let mut v = sub(b_end, b_start);
    let mut u = sub(a_end, a_start);
    let mut uv = x87_quiet(dot3(v, u));
    // Axis A is walked so that the two directions do not point apart.
    let (start, end) = if uv < 0.0 {
        uv = x87_quiet(-uv);
        u = negate(u);
        (a_end, a_start)
    } else {
        (a_start, a_end)
    };
    let r = sub(b_start, start);
    let ur = dot3(r, u);
    let vr = dot3(r, v);
    let uu = dot3(u, u);
    let vv = dot3(v, v);
    let (uv_d, ur_d, vr_d, uu_d, vv_d) = (
        f64::from(uv),
        f64::from(ur),
        f64::from(vr),
        f64::from(uu),
        f64::from(vv),
    );
    let uuvv = vv_d * uu_d;
    let determinant = (uuvv - uv_d * uv_d) as f32;
    let den = x87_quiet(determinant.abs());
    let den_d = f64::from(den);
    let num = (vv_d * ur_d - uv_d * vr_d) as f32;
    let num_d = f64::from(num);

    let mut status: u32;
    let mut s: f32;
    if num_d * den_d >= den_d * den_d {
        s = 1.0;
        status = 1;
    } else if num <= 0.0 {
        s = 0.0;
        status = 2;
    } else {
        let magnitude = x87_quiet((uuvv as f32).abs());
        let threshold = ((uv_d * uv_d + f64::from(magnitude)) * TWO_POW_MINUS_20) as f32;
        if threshold < den {
            s = (num_d / den_d) as f32;
            status = 0;
        } else {
            s = 1.0;
            status = 1;
        }
    }

    // The parameter on B: its own clamps replace the status bits when it leaves 0..1.
    let w = (uv_d * f64::from(s) - vr_d) as f32;
    let t: f32;
    let mut clamped_t = true;
    if w >= vv {
        status = 4;
        t = 1.0;
    } else if w <= 0.0 {
        status = 8;
        t = 0.0;
    } else {
        t = (f64::from(w) / vv_d) as f32;
        clamped_t = false;
    }
    if clamped_t {
        let projected = (uv_d * f64::from(t) + ur_d) as f32;
        if projected <= 0.0 {
            status |= 2;
            s = 0.0;
        } else if uu <= projected {
            status |= 1;
            s = 1.0;
        } else {
            s = (f64::from(projected) / uu_d) as f32;
        }
    }

    let radius_sum = (f64::from(radius_a) + f64::from(radius_b)) as f32;
    let on_a = scaled_add(s, u, start);
    let on_b = scaled_add(t, v, b_start);
    let difference = sub(on_a, on_b);
    let previous_gap = x87_quiet(e.mem.f32(out + 0x1c));
    let min_gap = if previous_gap < EPSILON {
        EPSILON
    } else {
        previous_gap
    };
    let distance = x87_quiet(dot3(difference, difference).sqrt());
    store(e, out + 0x10, difference);
    let inverse = (1.0 / f64::from(distance)) as f32;
    let gap = (f64::from(distance) - f64::from(radius_sum)) as f32;

    if !(min_gap > gap) {
        // Far enough apart: the normal is the scaled difference and the gap is stored.
        let scaled = difference.map(|lane| inverse * lane);
        store(e, out + 0x10, scaled);
        e.mem.set_f32(out + 0x1c, gap);
        return status;
    }
    let mut distance = distance;
    if EPSILON > distance {
        // The axes touch: the normal is any perpendicular of the direction of A.
        distance = 0.0;
        let magnitude = [u[0].abs(), u[1].abs(), u[2].abs()];
        // `pair` is the axis index swapped with the smallest-magnitude one (0 or 1), `other`
        // the component the perpendicular is built from (2 unless it is the smallest).
        let (pair, other, smallest) = if magnitude[0] > magnitude[1] {
            (0usize, 1usize, magnitude[1])
        } else {
            (1, 0, magnitude[0])
        };
        let third = if magnitude[2] < smallest { other } else { 2 };
        let mut normal = [0.0f32; 4];
        normal[pair] = x87_quiet(u[third]);
        normal[third] = x87_quiet(-u[pair]);
        store(e, out + 0x10, normalize_estimate(normal));
    } else {
        let scaled = difference.map(|lane| inverse * lane);
        store(e, out + 0x10, scaled);
    }
    let depth = x87_quiet((f64::from(radius_b) - f64::from(distance)) as f32);
    let normal = load(e, out + 0x10);
    e.mem.set_f32(out + 0x1c, gap);
    store(e, out, scaled_add(depth, normal, on_a));
    if FIFTH * uu_d * vv_d < den_d {
        return status;
    }

    // The overlap of almost parallel axes: a contact at each end of the shorter stretch.
    let inverse_product = (1.0 / uuvv) as f32;
    let f3 = (f64::from(inverse_product) * vv_d) as f32;
    let f2 = (f64::from(inverse_product) * uu_d) as f32;
    let mut nvr = x87_quiet(-vr);
    let mut ur_loop = x87_quiet(ur);
    let mut s_point = start;
    let mut b_point = b_start;
    let mut slot = out + 0x20;
    for iteration in 0..2 {
        let mask = if iteration == 0 { 10 } else { 5 };
        if status & mask == 0 {
            'contact: {
                let mut s_side = false;
                if nvr > 0.0 {
                    if vv < nvr {
                        break 'contact;
                    }
                    if ur_loop > 0.0 {
                        if uu < ur_loop {
                            break 'contact;
                        }
                        let (ur_d, nvr_d) = (f64::from(ur_loop), f64::from(nvr));
                        s_side = ur_d * ur_d * f64::from(f3) < nvr_d * nvr_d * f64::from(f2);
                    }
                    if !s_side {
                        let step = (f64::from(nvr) * f64::from(f2)) as f32;
                        b_point = scaled_add(step, v, b_point);
                    }
                } else if ur_loop > 0.0 {
                    if uu < ur_loop {
                        break 'contact;
                    }
                    s_side = true;
                }
                if s_side {
                    let step = (f64::from(ur_loop) * f64::from(f3)) as f32;
                    s_point = scaled_add(step, u, s_point);
                }

                let offset = sub(s_point, b_point);
                let squared = dot3(offset, offset);
                let slot_gap = x87_quiet(e.mem.f32(slot + 0x1c));
                let limit = (f64::from(slot_gap) + f64::from(radius_sum)) as f32;
                let limit_squared = (f64::from(limit) * f64::from(limit)) as f32;
                if !(limit_squared > squared) {
                    break 'contact;
                }
                let root = e.call(X87_SQRT, &args![f64::from(squared)]).f32();
                let normal = if EPSILON < squared {
                    normalize_estimate(offset)
                } else {
                    load(e, out + 0x10)
                };
                store(e, slot + 0x10, normal);
                let depth = (f64::from(radius_b) - f64::from(root)) as f32;
                let contact_gap = (f64::from(root) - f64::from(radius_sum)) as f32;
                e.mem.set_f32(slot + 0x1c, contact_gap);
                store(e, slot, scaled_add(depth, normal, s_point));
            }
        }
        if iteration == 1 {
            break;
        }
        // The second pass walks both axes from their far ends.
        let (uv_d, uu_d, vv_d) = (f64::from(uv), f64::from(uu), f64::from(vv));
        let next_nvr = ((vv_d - f64::from(nvr)) - uv_d) as f32;
        let next_ur = ((uu_d - f64::from(ur_loop)) - uv_d) as f32;
        s_point = end;
        b_point = b_end;
        u = negate(u);
        v = negate(v);
        slot += 0x20;
        nvr = next_nvr;
        ur_loop = next_ur;
    }
    status
}

/// This unit's translated functions, by exe address.
pub fn funcs() -> Vec<(u32, AbiFn)> {
    vec![entry!(
        0x00cd58b0,
        hk_collide_capsule_util_manifold_caps_vs_caps(Ptr, f32, Ptr, f32, Ptr) -> u32
    )]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vector(e: &mut Engine, v: [f32; 3]) -> u32 {
        let at = e.mem.alloc(0x10);
        for (i, c) in v.into_iter().enumerate() {
            e.mem.set_f32(at + 4 * i as u32, c);
        }
        at
    }

    fn capsule(e: &mut Engine, from: [f32; 3], to: [f32; 3]) -> Ptr {
        let at = e.mem.alloc(0x20);
        let first = vector(e, from);
        let second = vector(e, to);
        for lane in 0..4 {
            let low = e.mem.u32(first + 4 * lane);
            let high = e.mem.u32(second + 4 * lane);
            e.mem.set_u32(at + 4 * lane, low);
            e.mem.set_u32(at + 0x10 + 4 * lane, high);
        }
        Ptr::new(at)
    }

    /// An output block of three contacts whose gaps start at `gap`.
    fn contacts(e: &mut Engine, gap: f32) -> Ptr {
        let at = e.mem.alloc(0x60);
        for slot in 0..3 {
            e.mem.set_f32(at + 0x20 * slot + 0x1c, gap);
        }
        Ptr::new(at)
    }

    fn read(e: &Engine, at: u32) -> [f32; 3] {
        [e.mem.f32(at), e.mem.f32(at + 4), e.mem.f32(at + 8)]
    }

    fn near(actual: [f32; 3], expected: [f32; 3]) {
        for (a, x) in actual.iter().zip(expected) {
            assert!((a - x).abs() < 2e-3, "{actual:?} vs {expected:?}");
        }
    }

    #[test]
    fn crossing_axes_touch_in_one_contact() {
        let mut e = Engine::new();
        let a = capsule(&mut e, [0.0, 0.0, 0.0], [2.0, 0.0, 0.0]);
        let b = capsule(&mut e, [1.0, 0.8, -1.0], [1.0, 0.8, 1.0]);
        let out = contacts(&mut e, 0.1);
        let status = e.call(0x00cd58b0, &args![a, 0.5f32, b, 0.5f32, out]).u32();
        assert_eq!(status, 0);
        let out = out.addr();
        // The axes are 0.8 apart (gap -0.2 against the radii 0.5 + 0.5).
        near(read(&e, out + 0x10), [0.0, -1.0, 0.0]);
        assert!((e.mem.f32(out + 0x1c) + 0.2).abs() < 1e-5);
        near(read(&e, out), [1.0, 0.3, 0.0]);
    }

    #[test]
    fn a_wide_gap_stores_only_the_scaled_difference() {
        let mut e = Engine::new();
        let a = capsule(&mut e, [0.0, 0.0, 0.0], [2.0, 0.0, 0.0]);
        let b = capsule(&mut e, [1.0, 0.8, -1.0], [1.0, 0.8, 1.0]);
        let out = contacts(&mut e, 0.5);
        let status = e.call(0x00cd58b0, &args![a, 0.1f32, b, 0.1f32, out]).u32();
        assert_eq!(status, 0);
        let out = out.addr();
        // Gap 0.6 is at least the stored 0.5: the contact point is left untouched.
        near(read(&e, out + 0x10), [0.0, -1.0, 0.0]);
        assert!((e.mem.f32(out + 0x1c) - 0.6).abs() < 1e-5);
        assert_eq!(read(&e, out), [0.0, 0.0, 0.0]);
    }

    #[test]
    fn touching_axes_get_a_perpendicular_normal() {
        let mut e = Engine::new();
        let a = capsule(&mut e, [0.0, 0.0, 0.0], [2.0, 0.0, 0.0]);
        let b = capsule(&mut e, [1.0, 0.0, -1.0], [1.0, 0.0, 1.0]);
        let out = contacts(&mut e, 0.1);
        let status = e.call(0x00cd58b0, &args![a, 0.5f32, b, 0.5f32, out]).u32();
        assert_eq!(status, 0);
        let out = out.addr();
        near(read(&e, out + 0x10), [0.0, 0.0, -1.0]);
        near(read(&e, out), [1.0, 0.0, -0.5]);
        assert!((e.mem.f32(out + 0x1c) + 1.0).abs() < 1e-5);
    }

    #[test]
    fn almost_parallel_axes_add_a_second_contact() {
        let mut e = Engine::new();
        let a = capsule(&mut e, [0.0, 0.0, 0.0], [2.0, 0.0, 0.0]);
        let b = capsule(&mut e, [0.5, 0.8, 0.0], [2.5, 0.8, 0.25]);
        let out = contacts(&mut e, 1.0e9);
        let status = e.call(0x00cd58b0, &args![a, 0.5f32, b, 0.5f32, out]).u32();
        // Parameter t of B clamps at 0.
        assert_eq!(status, 8);
        let out = out.addr();
        near(read(&e, out), [0.5, 0.3, 0.0]);
        // The far end of A is paired with the point of B at the end of the overlap.
        let second = out + 0x40;
        near(read(&e, second), [1.991, 0.313, 0.0722]);
        assert!(e.mem.f32(second + 0x14) < -0.9);
    }
}
