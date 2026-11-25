//! The ray-triangle test (`00eadb90`) every `NiPick` of a triangle runs: the crosshair's pick in the
//! HUD's update (`0070bc20`, once a frame), VATS and the decals, through the typed picks of the
//! geometry family (`crate::geometry::pick`, a Rust call here) and the translated picks and
//! `NiOBBLeaf`'s test (by address). With a crowd in front of the player the crosshair's ray passes the
//! bounds of many actors' meshes and the test runs once for each of their triangles: in the counted
//! fight of 2026-11-25 (play build nov25c, twelve fighters placed in front of the player) about 12,500
//! times a frame on main while they stood there, and the frame's tail after the threads' join took
//! 5 to 6 ms against 3 ms once they had spread out.
//!
//! **Floats.** The picks run on main; the test is written once over [`Real`] at the running thread's
//! precision ([`float_precision`]), so it is the exe's arithmetic on any thread: on main (24 bits)
//! each operation rounds to a float, as `f32` maths does; at 53 bits each rounds to a double and only
//! the exe's float stores (`FSTP dword`) round to a float. Every sum is the exe's grouping of its
//! products, and the one value the exe compares unstored (`u + v` against the determinant) stays at
//! the running precision. Main's `f32` differs from the x87 only where an unstored step leaves the
//! float's range (past 3.4e38 or under 1.2e-38), which no game coordinate reaches.
//!
//! The back faces' cross product stays the exe's function (`004b3800`, by address), read from a frame
//! laid out as the exe's; the dot products, the scaled direction (`004a3760`) and the sum (`00439e90`)
//! the exe calls are done in place.

use crate::native::{float_precision, Real};
use engine_core::ptr::Ptr;
use engine_core::x87::x87_quiet_bits;
use engine_core::{args, Engine};

/// `1e-5` (`010718c0`) and `-1e-5` (`010d5368`): the determinant's limits.
const EPSILON: u32 = 0x0107_18c0;
const MINUS_EPSILON: u32 = 0x010d_5368;
/// `NiPoint3::operator%` (`004b3800`, `thiscall(this, out, other)`): the cross product.
const POINT_CROSS: u32 = 0x004b_3800;
/// The exe's frame (0x30 bytes) where the back faces' cross product reads its points: the edges (+0,
/// +0xc), the ray origin less the first corner (+0x18), the product (+0x24).
const FRAME: u32 = 0x30;
const FRAME_EDGE1: u32 = 0x0;
const FRAME_EDGE2: u32 = 0xc;
const FRAME_DIFFERENCE: u32 = 0x18;
const FRAME_CROSS: u32 = 0x24;

/// The test's words: the ray, the corners and the outs.
#[derive(Clone, Copy)]
struct Words {
    origin: u32,
    direction: u32,
    v0: u32,
    v1: u32,
    v2: u32,
    cull: u32,
    point: u32,
    t: u32,
    u: u32,
    v: u32,
}

// Typed from 00eadb90
/// The ray-triangle test (`cdecl`, ten words: the ray's origin and direction, the corners `v0`, `v1`,
/// `v2`, the cull word, then the outs for the point, `t`, `u` and `v`). With `e1 = v1 - v0`,
/// `e2 = v2 - v0`, `p = d x e2` and `det = e1 . p` (each stored as a float): for `det >= 1e-5`,
/// `u = (o - v0) . p` (to its out) must lie in `[0, det]`, `q = (o - v0) x e1`, `v = d . q` (to its out)
/// must be at least 0 with `u + v <= det`, and `t = e2 . q` (to its out) at least 0. For
/// `det <= -1e-5`, unless the cull word's low byte is set, the same tests mirrored (`u` in `[det, 0]`,
/// `v <= 0`, `u + v >= det`, `t <= 0`), with `q` from the exe's cross product. On success `u`, `v`
/// and `t` are scaled by `1 / det` and the point is `o + d t`; answers true. A NaN takes the branch
/// the exe's flag tests give it.
#[allow(clippy::too_many_arguments)]
pub fn ray_hits_triangle(
    e: &mut Engine,
    origin: Ptr,
    direction: Ptr,
    v0: Ptr,
    v1: Ptr,
    v2: Ptr,
    cull: u32,
    point: Ptr,
    t: Ptr,
    u: Ptr,
    v: Ptr,
) -> bool {
    let w = Words {
        origin: origin.addr(),
        direction: direction.addr(),
        v0: v0.addr(),
        v1: v1.addr(),
        v2: v2.addr(),
        cull,
        point: point.addr(),
        t: t.addr(),
        u: u.addr(),
        v: v.addr(),
    };
    if float_precision() {
        hits::<f32>(e, w)
    } else {
        hits::<f64>(e, w)
    }
}

/// The float at `at`, loaded.
#[inline(always)]
fn ld<R: Real>(e: &Engine, at: u32) -> R {
    R::load(e.mem.u32(at))
}

/// `a - b` per axis of two points in memory, each stored.
#[inline(always)]
fn difference<R: Real>(e: &Engine, a: u32, b: u32) -> [u32; 3] {
    [0u32, 4, 8].map(|i| (ld::<R>(e, a + i) - ld::<R>(e, b + i)).store())
}

/// Three stored floats, loaded.
#[inline(always)]
fn loaded<R: Real>(bits: [u32; 3]) -> [R; 3] {
    bits.map(R::load)
}

/// The test at precision `R`.
// `!(a >= b)`: a NaN fails the test, as the exe's flag test fails it.
#[allow(clippy::neg_cmp_op_on_partial_ord)]
fn hits<R: Real>(e: &mut Engine, w: Words) -> bool {
    let (o, d, a) = (w.origin, w.direction, w.v0);
    let edge1 = difference::<R>(e, w.v1, a);
    let edge2 = difference::<R>(e, w.v2, a);
    let [e1x, e1y, e1z] = loaded::<R>(edge1);
    let [e2x, e2y, e2z] = loaded::<R>(edge2);
    // p = d x e2, each stored.
    let p = [
        (ld::<R>(e, d + 4) * e2z - ld::<R>(e, d + 8) * e2y).store(),
        (ld::<R>(e, d + 8) * e2x - e2z * ld::<R>(e, d)).store(),
        (e2y * ld::<R>(e, d) - e2x * ld::<R>(e, d + 4)).store(),
    ];
    let [px, py, pz] = loaded::<R>(p);
    let det = R::load((px * e1x + py * e1y + pz * e1z).store());
    if det >= ld::<R>(e, EPSILON) {
        let [tx, ty, tz] = loaded::<R>(difference::<R>(e, o, a));
        let u_bits = (pz * tz + (px * tx + py * ty)).store();
        e.mem.set_u32(w.u, u_bits);
        let u = R::load(u_bits);
        if R::ZERO > u || det < u {
            return false;
        }
        // q = (o - v0) x e1, each stored.
        let [qx, qy, qz] = loaded::<R>([
            (ty * e1z - tz * e1y).store(),
            (tz * e1x - tx * e1z).store(),
            (e1y * tx - e1x * ty).store(),
        ]);
        let v_bits = (ld::<R>(e, d + 4) * qy + qx * ld::<R>(e, d) + ld::<R>(e, d + 8) * qz).store();
        e.mem.set_u32(w.v, v_bits);
        let v = R::load(v_bits);
        if R::ZERO > v {
            return false;
        }
        if det < v + ld::<R>(e, w.u) {
            return false;
        }
        // t = e2 . q (`004b6190`: its sum stored).
        let t_bits = (e2x * qx + e2y * qy + e2z * qz).store();
        e.mem.set_u32(w.t, t_bits);
        if R::ZERO > R::load(t_bits) {
            return false;
        }
    } else {
        if !(ld::<R>(e, MINUS_EPSILON) >= det) || w.cull & 0xff != 0 {
            return false;
        }
        let difference = difference::<R>(e, o, a);
        let [tx, ty, tz] = loaded::<R>(difference);
        let u_bits = (pz * tz + (py * ty + px * tx)).store();
        e.mem.set_u32(w.u, u_bits);
        let u = R::load(u_bits);
        if R::ZERO < u || u < det {
            return false;
        }
        let [cx, cy, cz] = loaded::<R>(cross(e, edge1, edge2, difference));
        // v = d . q and t = e2 . q (`004b6190`: each sum stored).
        let v_bits = (ld::<R>(e, d) * cx + ld::<R>(e, d + 4) * cy + ld::<R>(e, d + 8) * cz).store();
        e.mem.set_u32(w.v, v_bits);
        let v = R::load(v_bits);
        if R::ZERO < v {
            return false;
        }
        if det > v + ld::<R>(e, w.u) {
            return false;
        }
        let t_bits = (e2x * cx + e2y * cy + e2z * cz).store();
        e.mem.set_u32(w.t, t_bits);
        if R::ZERO < R::load(t_bits) {
            return false;
        }
    }
    let inverse = R::load((R::ONE / det).store());
    let u = (ld::<R>(e, w.u) * inverse).store();
    e.mem.set_u32(w.u, u);
    let v = (ld::<R>(e, w.v) * inverse).store();
    e.mem.set_u32(w.v, v);
    let t = x87_quiet_bits((inverse * ld::<R>(e, w.t)).store());
    e.mem.set_u32(w.t, t);
    // The point `o + d t`: the scaled direction (`004a3760`) and the sum (`00439e90`), one operation
    // on two floats each, so plain `f32` on every thread; the whole sum before the out is written.
    let scale = f32::from_bits(t);
    let [sx, sy, sz] = [0u32, 4, 8].map(|i| scale * f32::from_bits(e.mem.u32(d + i)));
    let [ox, oy, oz] = [0u32, 4, 8].map(|i| f32::from_bits(e.mem.u32(o + i)));
    for (i, value) in (0u32..).zip([ox + sx, oy + sy, oz + sz]) {
        e.mem.set_u32(w.point + 4 * i, value.to_bits());
    }
    true
}

/// `(o - v0) x e1` through the exe's cross product (`004b3800`), from a frame laid out as the exe's:
/// the edges, the difference, and the product read back.
fn cross(e: &mut Engine, edge1: [u32; 3], edge2: [u32; 3], difference: [u32; 3]) -> [u32; 3] {
    e.with_stack(FRAME, |e, frame| {
        let f = frame.addr();
        for (at, words) in [
            (FRAME_EDGE1, edge1),
            (FRAME_EDGE2, edge2),
            (FRAME_DIFFERENCE, difference),
        ] {
            for (i, word) in (0u32..).zip(words) {
                e.mem.set_u32(f + at + 4 * i, word);
            }
        }
        e.call(
            POINT_CROSS,
            &args![f + FRAME_DIFFERENCE, f + FRAME_CROSS, f + FRAME_EDGE1],
        );
        [0u32, 4, 8].map(|i| e.mem.u32(f + FRAME_CROSS + i))
    })
}
