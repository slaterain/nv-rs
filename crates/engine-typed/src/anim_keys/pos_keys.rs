//! The position keys (the Xbox PDB's `NiPosKey` source unit): `NiPosKey::GenInterp` and the key
//! types' interpolation functions it reaches through the table at `011f3ca8` (none: the zero point;
//! linear; Bezier and TCB, the cubic of the key's precomputed coefficients). Native maths at the
//! running thread's precision ([`Real`]), each float the exe stores rounded to a float (these
//! functions store every step).
//!
//! `GenInterp` does the interpolation of a known key type in place, with no call: a table word
//! holding one of the functions here. Any other word is called by address as the exe does.

use super::search::{ratio, round, search, LOWEST_TIME};
use super::Keys;
use crate::native::{float_precision, Real};
use engine_core::ptr::Ptr;
use engine_core::{args, Engine};

/// The position key types' interpolation functions, one word per key type (`cdecl`, (ratio, key,
/// next key, &point)).
const INTERPOLATIONS: u32 = 0x011f_3ca8;
/// The functions the table holds (`00a276a0`'s and `00a298f0`'s registrations): none (key type 0),
/// linear (1), Bezier (2) and TCB (3).
const NO_INTERPOLATION: u32 = 0x00a2_7470;
const LINEAR: u32 = 0x00a2_a7f0;
const BEZIER: u32 = 0x00a2_7c80;
const TCB: u32 = 0x00a2_af70;
/// The zero point the empty interpolation copies (`011f426c`, three floats).
const ZERO_POINT: u32 = 0x011f_426c;

/// A float word as `R`.
#[inline(always)]
fn ld<R: Real>(e: &Engine, at: u32) -> R {
    R::load(e.mem.u32(at))
}

/// The linear interpolation `next * t + key * (1 - t)` of the two keys' points (+4), each product
/// and sum stored as a float, in the exe's order of operands.
fn linear_point<R: Real>(e: &Engine, t: R, key: u32, next: u32) -> [u32; 3] {
    let rest = round(R::ONE - t);
    let k = |i: u32| ld::<R>(e, key.wrapping_add(4 + 4 * i));
    let n = |i: u32| ld::<R>(e, next.wrapping_add(4 + 4 * i));
    let a = [round(k(0) * rest), round(k(1) * rest), round(rest * k(2))];
    let b = [round(n(0) * t), round(n(1) * t), round(t * n(2))];
    [
        (b[0] + a[0]).store(),
        (a[1] + b[1]).store(),
        (b[2] + a[2]).store(),
    ]
}

/// The cubic `point + t (tangent + t (a + t b))` of one key, with its point at +4 and its tangent
/// and coefficients at `tangent`, `a` and `b`, every step stored as a float (the Bezier and TCB
/// keys).
fn cubic_point<R: Real>(e: &Engine, t: R, key: u32, tangent: u32, a: u32, b: u32) -> [u32; 3] {
    let at = |offset: u32, i: u32| ld::<R>(e, key.wrapping_add(offset + 4 * i));
    let lane = |i: u32| {
        let bt = round(at(b, i) * t);
        let sum = round(at(a, i) + bt);
        let scaled = round(sum * t);
        let inner = round(at(tangent, i) + scaled);
        let outer = round(inner * t);
        (at(4, i) + outer).store()
    };
    [lane(0), lane(1), lane(2)]
}

/// What the key type's function `function` writes for the ratio `t` between `key` and `next`, done
/// in place; `None` for a function this does not know.
fn interpolate<R: Real>(
    e: &Engine,
    function: u32,
    t: u32,
    key: u32,
    next: u32,
) -> Option<[u32; 3]> {
    let r = R::load(t);
    Some(match function {
        NO_INTERPOLATION => [0u32, 4, 8].map(|o| e.mem.u32(ZERO_POINT + o)),
        LINEAR => linear_point(e, r, key, next),
        BEZIER => cubic_point(e, r, key, 0x1c, 0x28, 0x34),
        TCB => cubic_point(e, r, key, 0x28, 0x34, 0x40),
        _ => return None,
    })
}

/// `GenInterp`'s point, updating `last_index` (see [`gen_interp`]).
pub(crate) fn point<R: Real>(
    e: &mut Engine,
    time: f32,
    keys: u32,
    key_type: u32,
    count: u32,
    last_index: &mut u32,
    key_size: u32,
) -> [u32; 3] {
    if count == 1 || time == LOWEST_TIME {
        return [4u32, 8, 0xc].map(|o| e.mem.u32(keys.wrapping_add(o)));
    }
    let keys = Keys::new(keys, key_size);
    let time = f64::from(time);
    let found = search(e, time, keys, count, *last_index);
    let t = ratio::<R>(time, &found);
    let (key, next) = (keys.at(found.index), keys.at(found.next_index));
    *last_index = found.index;
    let function = e
        .mem
        .u32(INTERPOLATIONS.wrapping_add(key_type.wrapping_mul(4)));
    if let Some(point) = interpolate::<R>(e, function, t, key, next) {
        return point;
    }
    e.with_stack(12, |e, local| {
        e.call(function, &args![t, key, next, local]);
        [0u32, 4, 8].map(|o| e.mem.u32(local.addr() + o))
    })
}

// Typed from 00a27490
/// `NiPosKey::GenInterp` (Xbox PDB, `cdecl`, the point returned through `result`): with one key, or
/// at time `-FLT_MAX`, the first key's point (+4). Otherwise the key search and the key type's
/// interpolation (the table at `011f3ca8`) between the keys `index` and `next_index` with the
/// ratio; the last index becomes `index`. A search running off the end still interpolates (towards
/// the key at `next_index`, past the array). Answers `result`.
#[allow(clippy::too_many_arguments)]
pub fn gen_interp(
    e: &mut Engine,
    result: Ptr,
    time: f32,
    keys: Ptr,
    key_type: u32,
    count: u32,
    last_index: Ptr,
    key_size: u8,
) -> Ptr {
    let first = count == 1 || time == LOWEST_TIME;
    let mut last = if first {
        0
    } else {
        e.mem.u32(last_index.addr())
    };
    let size = u32::from(key_size);
    let point = if float_precision() {
        point::<f32>(e, time, keys.addr(), key_type, count, &mut last, size)
    } else {
        point::<f64>(e, time, keys.addr(), key_type, count, &mut last, size)
    };
    if !first {
        e.mem.set_u32(last_index.addr(), last);
    }
    for (i, word) in (0u32..).zip(point) {
        e.mem.set_u32(result.addr().wrapping_add(4 * i), word);
    }
    result
}

/// One of the table's functions as the exe registers it: [`interpolate`] with `function`, the point
/// stored at `result`.
fn interpolation(e: &mut Engine, function: u32, t: f32, key: Ptr, next: Ptr, result: Ptr) {
    let (t, key, next) = (t.to_bits(), key.addr(), next.addr());
    let point = if float_precision() {
        interpolate::<f32>(e, function, t, key, next)
    } else {
        interpolate::<f64>(e, function, t, key, next)
    };
    for (i, word) in (0u32..).zip(point.into_iter().flatten()) {
        e.mem.set_u32(result.addr().wrapping_add(4 * i), word);
    }
}

// Typed from 00a27470
/// The position keys' function for no interpolation (`cdecl`, (ratio, key, next key, &point)): the
/// zero point (`011f426c`) at `point`; the other words are not read.
pub fn none(e: &mut Engine, t: f32, key: Ptr, next: Ptr, result: Ptr) {
    interpolation(e, NO_INTERPOLATION, t, key, next, result);
}

// Typed from 00a2a7f0
/// The linear position keys' interpolation (`cdecl`, (ratio, key, next key, &point)): `next * t +
/// key * (1 - t)` of the two keys' points (+4), each product and sum stored as a float.
pub fn linear(e: &mut Engine, t: f32, key: Ptr, next: Ptr, result: Ptr) {
    interpolation(e, LINEAR, t, key, next, result);
}

// Typed from 00a27c80
/// The Bezier position keys' interpolation (`cdecl`, (ratio, key, next key (not read), &point)):
/// per lane `point + t (tangent + t (a + t b))` with the key's point (+4), outgoing tangent (+0x1c)
/// and coefficients a (+0x28) and b (+0x34), each step stored as a float.
pub fn bezier(e: &mut Engine, t: f32, key: Ptr, next: Ptr, result: Ptr) {
    interpolation(e, BEZIER, t, key, next, result);
}

// Typed from 00a2af70
/// The TCB position keys' interpolation (`cdecl`, (ratio, key, next key (not read), &point)): the
/// same cubic with the key's outgoing tangent at +0x28 and coefficients at +0x34 and +0x40.
pub fn tcb(e: &mut Engine, t: f32, key: Ptr, next: Ptr, result: Ptr) {
    interpolation(e, TCB, t, key, next, result);
}
