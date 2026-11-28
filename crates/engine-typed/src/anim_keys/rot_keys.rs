//! The rotation keys (the Xbox PDB's `NiRotKey` source unit, and the unit at `00a289b0`):
//! `NiRotKey::GenInterp`, the key types' interpolation functions it reaches through the table at
//! `011f3cc0` (none: the identity; linear: `Slerp`; Bezier and TCB: the spherical quadrangle
//! interpolation; step), and the preparation of a rotation key array (the exact x87 helpers).
//!
//! `GenInterp` runs for every animated bone every frame: the key search ([`super::search`]), the
//! ratio at the running thread's precision, and the interpolation of a known key type in place,
//! with no call (`Slerp`, the quadrangle interpolation and the normalisation native,
//! [`quaternion`]). A table word this does not know and the Euler keys (type 4, whose angles take
//! `FSINCOS`) stay calls by address.

use super::quaternion;
use super::search::{ratio, search, LOWEST_TIME};
use super::{Keys, NiQuaternion, FAMILY};
use crate::native::float_precision;
use engine_core::ptr::Ptr;
use engine_core::x87::{x87_add, x87_load32, x87_m32, x87_mul, x87_store32};
use engine_core::{args, typed_call, Engine};

/// The rotation keys' interpolation functions, one word per key type (`011f3cc0`; each `cdecl`,
/// (ratio or time, key, next key, &result)).
const INTERPOLATIONS: u32 = 0x011f_3cc0;
/// The key type `GenInterp` hands the time and the whole key array (4).
const WHOLE_ARRAY_TYPE: u32 = 4;
/// The functions the table holds (`00a289b0`'s, `00a298f0`'s and `00a2bde0`'s registrations): the
/// identity (key type 0), linear (1), Bezier (2), TCB (3) and step (5).
const NO_INTERPOLATION: u32 = 0x00a2_86e0;
const LINEAR: u32 = 0x00a2_ab90;
const BEZIER: u32 = 0x00a2_8d90;
const TCB: u32 = 0x00a2_bc60;
const STEP: u32 = 0x00a2_cf20;
/// 1.0, the identity's `W`.
const ONE: u32 = 0x3f80_0000;
/// `float` 0.0 (`01011d78`), the sign test of the dot product, and -1.0 (`01012054`), the lower
/// clamp of `W`.
const ZERO_FLOAT: u32 = 0x0101_1d78;
const MINUS_ONE_FLOAT: u32 = 0x0101_2054;

/// What the key type's function `function` writes for the ratio `t` between `key` and `next`, done
/// in place; `None` for a function this does not know.
fn interpolate(e: &mut Engine, function: u32, t: u32, key: u32, next: u32) -> Option<[u32; 4]> {
    let at = |base: u32, offset: u32| base.wrapping_add(offset);
    Some(match function {
        NO_INTERPOLATION => [ONE, 0, 0, 0],
        LINEAR => {
            let p = quaternion::words(e, at(key, 4));
            let q = quaternion::words(e, at(next, 4));
            let time = f32::from_bits(t);
            if float_precision() {
                quaternion::slerp_value::<f32>(e, time, p, q)
            } else {
                quaternion::slerp_value::<f64>(e, time, p, q)
            }
        }
        BEZIER => quaternion::squad_of(
            e,
            f32::from_bits(t),
            at(key, 4),
            at(key, 0x14),
            at(next, 0x14),
            at(next, 4),
        ),
        TCB => quaternion::squad_of(
            e,
            f32::from_bits(t),
            at(key, 4),
            at(key, 0x20),
            at(next, 0x30),
            at(next, 4),
        ),
        STEP => {
            let chosen = if 1.0 > f32::from_bits(t) { key } else { next };
            quaternion::words(e, at(chosen, 4))
        }
        _ => return None,
    })
}

/// The key type's function `function` for `(t, key, next)`: in place when known, else called by
/// address into a stack quaternion.
fn interpolated(e: &mut Engine, function: u32, t: u32, key: u32, next: u32) -> [u32; 4] {
    if let Some(q) = interpolate(e, function, t, key, next) {
        return q;
    }
    e.with_stack(0x10, |e, local| {
        e.call(function, &args![t, key, next, local]);
        quaternion::words(e, local.addr())
    })
}

// Typed from 00a28740
/// `NiRotKey::GenInterp` (Xbox PDB, `cdecl`, the quaternion returned through `result`): for key
/// type 4, always the type's function (`011f3cc0`) with the time itself, the key array and a null
/// next key, into a stack quaternion copied to `result` (the last index untouched). Otherwise, with
/// one key or at time `-FLT_MAX`, the first key's quaternion (+4); else the key search and the
/// type's function between the keys `index` and `next_index` with the ratio, the last index
/// becoming `index` (a search running off the end still interpolates, towards the key past the
/// array). Answers `result`. The exe keeps the key times it loads, and the ratio, in its own
/// `count` argument's slot once it has read the count; this keeps them in locals and leaves the
/// argument words as they came (`cdecl`: the caller drops them).
#[allow(clippy::too_many_arguments)]
pub fn gen_interp(
    e: &mut Engine,
    result: Ptr<NiQuaternion>,
    time: f32,
    keys: Ptr,
    key_type: u32,
    count: u32,
    last_index: Ptr,
    key_size: u8,
) -> Ptr<NiQuaternion> {
    let searched = key_type != WHOLE_ARRAY_TYPE && count != 1 && time != LOWEST_TIME;
    let mut last = if searched {
        e.mem.u32(last_index.addr())
    } else {
        0
    };
    let rotation = rotation(e, time, keys.addr(), key_type, count, &mut last, key_size);
    if searched {
        e.mem.set_u32(last_index.addr(), last);
    }
    quaternion::set_words(e, result.addr(), rotation);
    result
}

/// [`gen_interp`]'s quaternion, updating `last_index` as it does (callers that keep the index in a
/// field of their own hand it in and store it back).
pub(crate) fn rotation(
    e: &mut Engine,
    time: f32,
    keys: u32,
    key_type: u32,
    count: u32,
    last_index: &mut u32,
    key_size: u8,
) -> [u32; 4] {
    let keys = Keys::new(keys, u32::from(key_size));
    let function = e
        .mem
        .u32(INTERPOLATIONS.wrapping_add(key_type.wrapping_mul(4)));
    if key_type == WHOLE_ARRAY_TYPE {
        return interpolated(e, function, time.to_bits(), keys.at(0), 0);
    }
    if count == 1 || time == LOWEST_TIME {
        return quaternion::words(e, keys.at(0).wrapping_add(4));
    }
    let time = f64::from(time);
    let found = search(e, time, keys, count, *last_index);
    let t = if float_precision() {
        ratio::<f32>(time, &found)
    } else {
        ratio::<f64>(time, &found)
    };
    *last_index = found.index;
    interpolated(
        e,
        function,
        t,
        keys.at(found.index),
        keys.at(found.next_index),
    )
}

/// One of the table's functions as the exe registers it, the quaternion stored at `result`.
fn interpolation(e: &mut Engine, function: u32, t: f32, key: Ptr, next: Ptr, result: Ptr) {
    if let Some(q) = interpolate(e, function, t.to_bits(), key.addr(), next.addr()) {
        quaternion::set_words(e, result.addr(), q);
    }
}

// Typed from 00a286e0
/// The rotation keys' function for no interpolation (`cdecl`, (ratio, key, next key, &result)): the
/// identity quaternion (1, 0, 0, 0) at `result` (the exe builds it on its stack with `00a6db50`);
/// the other words are not read.
pub fn identity(e: &mut Engine, t: f32, key: Ptr, next: Ptr, result: Ptr) {
    interpolation(e, NO_INTERPOLATION, t, key, next, result);
}

// Typed from 00a2ab90
/// The linear rotation keys' interpolation (`cdecl`, (ratio, key, next key, &result)): `Slerp` into
/// `result` of the two keys' quaternions (+4) at the ratio, done in place. Answers `Slerp`'s `EAX`,
/// 0.
pub fn linear(e: &mut Engine, t: f32, key: Ptr, next: Ptr, result: Ptr) -> u32 {
    quaternion::slerp_at(e, result.addr(), t, key.addr() + 4, next.addr() + 4);
    0
}

// Typed from 00a28d90
/// The Bezier rotation keys' interpolation (`cdecl`, (ratio, key, next key, &result)): the
/// quadrangle interpolation ([`quaternion::squad`], in place) at `result` of the key's quaternion
/// (+4) and intermediate (+0x14) and the next key's intermediate and quaternion.
pub fn bezier(e: &mut Engine, t: f32, key: Ptr, next: Ptr, result: Ptr) {
    interpolation(e, BEZIER, t, key, next, result);
}

// Typed from 00a2bc60
/// The TCB rotation keys' interpolation (`cdecl`, (ratio, key, next key, &result)): the quadrangle
/// interpolation ([`quaternion::squad`], in place) at `result` of the key's quaternion (+4) and
/// outgoing tangent quaternion (+0x20) and the next key's incoming tangent quaternion (+0x30) and
/// quaternion (+4). Answers the quadrangle interpolation's `EAX`, 0.
pub fn tcb(e: &mut Engine, t: f32, key: Ptr, next: Ptr, result: Ptr) -> u32 {
    interpolation(e, TCB, t, key, next, result);
    0
}

// Typed from 00a2cf20
/// The step rotation keys' interpolation (`cdecl`, (ratio, key, next key, &result)): the key's
/// quaternion (+4) while 1.0 is above the ratio, else the next key's, at `result`.
pub fn step(e: &mut Engine, t: f32, key: Ptr, next: Ptr, result: Ptr) {
    interpolation(e, STEP, t, key, next, result);
}

/// Clamps the `float` at `at` as the exe's clamp blocks do: the value (through `FLD`/`FSTP`/`FLD`
/// of a local) below `low` stores `low`, otherwise above 1 stores 1; NaN stores nothing.
fn clamp_w(e: &mut Engine, at: u32, low: f64) {
    let v = x87_load32(x87_store32(x87_load32(e.mem.u32(at))));
    if v < low {
        e.mem.set_u32(at, x87_store32(low));
    } else if v > 1.0 {
        e.mem.set_u32(at, x87_store32(1.0));
    }
}

// Typed from 00a28ab0
/// The preparation of `count` rotation keys of `size` bytes (`cdecl`, no Xbox PDB name; each key's
/// quaternion at +4, `W` first): each key's quaternion whose dot product with the previous key's is
/// negative is negated ([`quaternion::negate`] into a stack quaternion, copied back), so neighbours
/// lie in one hemisphere; then every key's `W` is clamped to `[-1, 1]`, four keys at a time and
/// then the rest. As in the exe, the first loop runs `count - 1` times without a guard for `count
/// == 0`.
pub fn prepare(e: &mut Engine, keys: Ptr, count: u32, size: u8) {
    let keys = Keys::new(keys.addr(), u32::from(size));
    let zero = x87_m32(e.global::<u32>(ZERO_FLOAT));
    let mut remaining = count.wrapping_sub(1);
    let mut k = 1u32;
    while remaining != 0 {
        // The products in the exe's order: X, W, Y, Z (this key's operand loaded, the previous
        // key's the memory operand, but for W).
        let ld = |e: &Engine, key: u32, offset: u32| x87_load32(keys.word(e, key, offset));
        let m = |e: &Engine, key: u32, offset: u32| x87_m32(keys.word(e, key, offset));
        let previous = k.wrapping_sub(1);
        let x = x87_mul(ld(e, k, 8), m(e, previous, 8));
        let w = x87_mul(ld(e, previous, 4), m(e, k, 4));
        let sum = x87_add(x, w);
        let y = x87_mul(ld(e, k, 0xc), m(e, previous, 0xc));
        let sum = x87_add(sum, y);
        let z = x87_mul(ld(e, k, 0x10), m(e, previous, 0x10));
        let dot = x87_load32(x87_store32(x87_add(sum, z)));
        // FCOMP 0.0; TEST AH,0x5; JP: only "below" negates.
        if dot < zero {
            let at = keys.at(k).wrapping_add(4);
            e.with_stack(0x10, |e, scratch| {
                let this: Ptr<NiQuaternion> = Ptr::new(at);
                let result: Ptr<NiQuaternion> = scratch.cast();
                let negated = typed_call!(FAMILY, 0x00a6_dbd0, quaternion::negate(e, this, result));
                for i in 0..4u32 {
                    let word = e.mem.u32(negated.addr().wrapping_add(4 * i));
                    e.mem.set_u32(at.wrapping_add(4 * i), word);
                }
            });
        }
        k = k.wrapping_add(1);
        remaining = remaining.wrapping_sub(1);
    }
    let low = x87_load32(e.global::<u32>(MINUS_ONE_FLOAT));
    let w = |k: u32| keys.at(k).wrapping_add(4);
    let mut done = 0u32;
    if count as i32 >= 4 {
        let mut groups = (count.wrapping_sub(4) >> 2).wrapping_add(1);
        done = groups.wrapping_mul(4);
        let mut first = 0u32;
        while groups != 0 {
            for step in 0..4u32 {
                clamp_w(e, w(first.wrapping_add(step)), low);
            }
            first = first.wrapping_add(4);
            groups = groups.wrapping_sub(1);
        }
    }
    let mut k = done;
    while k < count {
        clamp_w(e, w(k), low);
        k += 1;
    }
}
