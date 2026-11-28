//! The float keys (the Xbox PDB's `NiFloatKey` source unit): `NiFloatKey::GenInterp` and the key
//! types' interpolation functions it reaches through the table at `011f3c90` (linear, Bezier, TCB,
//! step, and the empty function of a key without interpolation), with the cubic Hermite value and
//! the linear blend they share. Native maths at the running thread's precision ([`Real`]): each
//! operation rounded as the exe's x87 code rounds it there, each float the exe stores rounded to a
//! float.
//!
//! `GenInterp` does the interpolation of a known key type in place, with no call: a table word
//! holding one of the functions here. Any other word (none in the game) is called by address as the
//! exe does.

use super::search::{ratio, round, search, LOWEST_TIME};
use super::Keys;
use crate::native::{float_precision, Real};
use engine_core::ptr::Ptr;
use engine_core::{args, Engine};

/// The float key types' interpolation functions, one word per key type (`cdecl`, (ratio, key, next
/// key, &result)).
const INTERPOLATIONS: u32 = 0x011f_3c90;
/// The functions the table holds (`00a26d70`'s, `00a298f0`'s and `00a2bde0`'s registrations): the
/// empty function (key type 0), linear (1), Bezier (2), TCB (3) and step (5).
const NO_INTERPOLATION: u32 = 0x00a2_9680;
const LINEAR: u32 = 0x00a2_a560;
const BEZIER: u32 = 0x00a2_6fe0;
const TCB: u32 = 0x00a2_c610;
const STEP: u32 = 0x00a2_d200;

/// A float word as `R`.
#[inline(always)]
fn ld<R: Real>(e: &Engine, at: u32) -> R {
    R::load(e.mem.u32(at))
}

/// `00a42050`'s blend `t * b + (1 - t) * a`, rounded to a float.
fn lerp_value<R: Real>(t: R, a: R, b: R) -> u32 {
    let from_a = (R::ONE - t) * a;
    let from_b = t * b;
    (from_b + from_a).store()
}

/// `00a42070`'s cubic Hermite value at `t` (its two coefficients stored as floats, as the exe keeps
/// them in an argument's stack word), rounded to a float.
fn hermite_value<R: Real>(t: R, start: R, start_tangent: R, end: R, end_tangent: R) -> u32 {
    let two = R::ONE + R::ONE;
    let three = two + R::ONE;
    let difference = end - start;
    let cubic = round((start_tangent + end_tangent) - difference * two);
    let quadratic = round(difference * three - (end_tangent + two * start_tangent));
    let inner = (cubic * t + quadratic) * t;
    ((start_tangent + inner) * t + start).store()
}

/// The value the step keys pick: the first key's while 1.0 is above `t` (a NaN takes the second).
fn step_value(e: &Engine, t: f32, key: u32, next: u32) -> u32 {
    let chosen = if 1.0 > t { key } else { next };
    e.mem.u32(chosen.wrapping_add(4))
}

/// What the key type's function `function` writes for the ratio `t` between `key` and `next`, done
/// in place; `None` for a function this does not know.
fn interpolate<R: Real>(e: &Engine, function: u32, t: u32, key: u32, next: u32) -> Option<u32> {
    let r = R::load(t);
    let at = |base: u32, offset: u32| ld::<R>(e, base.wrapping_add(offset));
    Some(match function {
        // The empty function leaves the result word as it is: the ratio, which it held.
        NO_INTERPOLATION => t,
        LINEAR => lerp_value(r, at(key, 4), at(next, 4)),
        BEZIER => hermite_value(r, at(key, 4), at(key, 0xc), at(next, 4), at(next, 8)),
        TCB => hermite_value(r, at(key, 4), at(key, 0x18), at(next, 4), at(next, 0x14)),
        STEP => step_value(e, f32::from_bits(t), key, next),
        _ => return None,
    })
}

/// `GenInterp`'s value as a float word, updating `last_index` (see [`gen_interp`]).
pub(crate) fn value<R: Real>(
    e: &mut Engine,
    time: f32,
    keys: u32,
    key_type: u32,
    count: u32,
    last_index: &mut u32,
    key_size: u32,
) -> u32 {
    let first_value = keys.wrapping_add(4);
    if count == 1 || time == LOWEST_TIME {
        return e.mem.u32(first_value);
    }
    let keys = Keys::new(keys, key_size);
    let time = f64::from(time);
    let found = search(e, time, keys, count, *last_index);
    *last_index = found.index;
    let key = keys.at(found.index);
    if found.next_index >= count {
        return e.mem.u32(key.wrapping_add(4));
    }
    let t = ratio::<R>(time, &found);
    let next = keys.at(found.next_index);
    let function = e
        .mem
        .u32(INTERPOLATIONS.wrapping_add(key_type.wrapping_mul(4)));
    if let Some(value) = interpolate::<R>(e, function, t, key, next) {
        return value;
    }
    e.with_stack(4, |e, out| {
        e.mem.set_u32(out.addr(), t);
        e.call(function, &args![t, key, next, out]);
        e.mem.u32(out.addr())
    })
}

// Typed from 00a26b40
/// `NiFloatKey::GenInterp` (Xbox PDB, `cdecl`, the value in ST0): with one key, or at time
/// `-FLT_MAX`, the first key's value (+4). Otherwise the key search; when it ran off the end
/// (`next_index >= count`) the value of the key at `index`, else the key type's interpolation (the
/// table at `011f3c90`) between the keys `index` and `next_index` with the ratio. Either way the
/// last index becomes `index`.
pub fn gen_interp(
    e: &mut Engine,
    time: f32,
    keys: Ptr,
    key_type: u32,
    count: u32,
    last_index: Ptr,
    key_size: u8,
) -> f64 {
    let first = count == 1 || time == LOWEST_TIME;
    let mut last = if first {
        0
    } else {
        e.mem.u32(last_index.addr())
    };
    let size = u32::from(key_size);
    let bits = if float_precision() {
        value::<f32>(e, time, keys.addr(), key_type, count, &mut last, size)
    } else {
        value::<f64>(e, time, keys.addr(), key_type, count, &mut last, size)
    };
    if !first {
        e.mem.set_u32(last_index.addr(), last);
    }
    f64::from(f32::from_bits(bits))
}

// Typed from 00a42050
/// The linear blend of `a` and `b` by `t` (`cdecl`, three floats, the value in ST0): `t * b + (1 -
/// t) * a`, rounded to a float.
pub fn lerp(_e: &mut Engine, t: f32, a: f32, b: f32) -> f64 {
    let bits = if float_precision() {
        lerp_value(t, a, b)
    } else {
        lerp_value(f64::from(t), f64::from(a), f64::from(b))
    };
    f64::from(f32::from_bits(bits))
}

// Typed from 00a42070
/// The cubic Hermite value at `t` of the segment from `start` (tangent `start_tangent`) to `end`
/// (tangent `end_tangent`) (`cdecl`, five floats, the value in ST0): `start + t (start_tangent + t
/// (c2 + t c3))` with `c3 = (start_tangent + end_tangent) - 2 (end - start)` and `c2 = 3 (end -
/// start) - (end_tangent + 2 start_tangent)`, each coefficient stored as a float, rounded to a
/// float.
pub fn hermite(
    _e: &mut Engine,
    t: f32,
    start: f32,
    start_tangent: f32,
    end: f32,
    end_tangent: f32,
) -> f64 {
    let bits = if float_precision() {
        hermite_value(t, start, start_tangent, end, end_tangent)
    } else {
        let w = f64::from;
        hermite_value(w(t), w(start), w(start_tangent), w(end), w(end_tangent))
    };
    f64::from(f32::from_bits(bits))
}

/// One of the table's functions as the exe registers it: [`interpolate`] with `function`, the value
/// stored at `result`.
fn interpolation(e: &mut Engine, function: u32, t: f32, key: Ptr, next: Ptr, result: Ptr) {
    let (t, key, next) = (t.to_bits(), key.addr(), next.addr());
    let value = if float_precision() {
        interpolate::<f32>(e, function, t, key, next)
    } else {
        interpolate::<f64>(e, function, t, key, next)
    };
    if let Some(value) = value {
        e.mem.set_u32(result.addr(), value);
    }
}

// Typed from 00a2a560
/// The linear float keys' interpolation (`cdecl`, (ratio, key, next key, &result)): the blend of
/// the two keys' values (+4) by the ratio ([`lerp`]) at `result`.
pub fn linear(e: &mut Engine, t: f32, key: Ptr, next: Ptr, result: Ptr) {
    interpolation(e, LINEAR, t, key, next, result);
}

// Typed from 00a26fe0
/// The Bezier float keys' interpolation (`cdecl`, (ratio, key, next key, &result)): [`hermite`] of
/// the key's value (+4) and outgoing tangent (+0xc) and the next key's value and incoming tangent
/// (+8).
pub fn bezier(e: &mut Engine, t: f32, key: Ptr, next: Ptr, result: Ptr) {
    interpolation(e, BEZIER, t, key, next, result);
}

// Typed from 00a2c610
/// The TCB float keys' interpolation (`cdecl`, (ratio, key, next key, &result)): [`hermite`] of the
/// key's value (+4) and outgoing tangent (+0x18) and the next key's value and incoming tangent
/// (+0x14).
pub fn tcb(e: &mut Engine, t: f32, key: Ptr, next: Ptr, result: Ptr) {
    interpolation(e, TCB, t, key, next, result);
}

// Typed from 00a2d200
/// The step float keys' interpolation (`cdecl`, (ratio, key, next key, &result)): the key's value
/// (+4) while 1.0 is above the ratio, else the next key's.
pub fn step(e: &mut Engine, t: f32, key: Ptr, next: Ptr, result: Ptr) {
    interpolation(e, STEP, t, key, next, result);
}
