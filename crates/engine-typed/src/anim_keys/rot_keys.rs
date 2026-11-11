//! The rotation keys (the Xbox PDB's `NiRotKey` source unit, and the unit at `00a289b0`):
//! `NiRotKey::GenInterp` with the key search the `GenInterp` functions share (native maths: the
//! times as `f64` locals, the ratio rounded where the exe stores it), and the preparation of a
//! rotation key array (the exact x87 helpers).

use super::quaternion;
use super::{narrow, Keys, NiQuaternion, FAMILY};
use engine_core::ptr::Ptr;
use engine_core::x87::{x87_add, x87_load32, x87_m32, x87_mul, x87_store32};
use engine_core::{args, typed_call, Engine};

/// The rotation keys' interpolation functions, one word per key type (`011f3cc0`; each `cdecl`, (ratio
/// or time, key, next key, &result)).
const INTERPOLATIONS: u32 = 0x011f_3cc0;
/// The key type `GenInterp` hands the time and the whole key array (4).
const WHOLE_ARRAY_TYPE: u32 = 4;
/// `-FLT_MAX` as the `double` the time is compared with (`010241b0`).
const LOWEST_TIME: u32 = 0x0102_41b0;
/// `float` 0.0 (`01011d78`), the sign test of the dot product, and -1.0 (`01012054`), the lower clamp
/// of `W`.
const ZERO_FLOAT: u32 = 0x0101_1d78;
const MINUS_ONE_FLOAT: u32 = 0x0101_2054;

/// Where the key search stopped: the key before the time and the one after it, with their times as
/// the search last loaded them.
struct Search {
    index: u32,
    next_index: u32,
    previous_time: f64,
    /// 0 when the search loaded no later key (the value left on the x87 stack).
    next_time: f64,
}

/// The key search the `GenInterp` functions share (the same code inlined in each). It starts at the
/// last index, back at key 0 when that key's time is after `time`, then walks forward (four keys per
/// round while at least four remain, counted signed, then one by one up to `count - 1`, unsigned) to
/// the first key whose time is not before `time` (`FCOM`: a NaN on either side counts as before).
fn search(e: &Engine, time: f64, keys: Keys, count: u32, last_index: u32) -> Search {
    let mut index = last_index;
    let mut previous_time = keys.time(e, index);
    if previous_time > time {
        previous_time = keys.time(e, 0);
        index = 0;
    }
    let mut next_index = index.wrapping_add(1);
    let mut next_time = 0.0;
    let remaining = count
        .wrapping_sub(1)
        .wrapping_sub(next_index)
        .wrapping_add(1) as i32;
    if remaining >= 4 {
        loop {
            for step in 0..4u32 {
                let k = keys.time(e, next_index.wrapping_add(step));
                next_time = k;
                if k >= time {
                    return Search {
                        index: index.wrapping_add(step),
                        next_index: next_index.wrapping_add(step),
                        previous_time,
                        next_time,
                    };
                }
                previous_time = k;
            }
            index = index.wrapping_add(4);
            next_index = next_index.wrapping_add(4);
            if next_index > count.wrapping_sub(4) {
                break;
            }
        }
    }
    while next_index <= count.wrapping_sub(1) {
        let k = keys.time(e, next_index);
        next_time = k;
        if k >= time {
            break;
        }
        next_index = next_index.wrapping_add(1);
        previous_time = k;
        index = index.wrapping_add(1);
    }
    Search {
        index,
        next_index,
        previous_time,
        next_time,
    }
}

/// The ratio `(time - previous) / (next - previous)` the interpolation function is given, as the
/// float stored for it (native: `f64` locals, rounded once at the store).
fn ratio(time: f64, search: &Search) -> u32 {
    let elapsed = time - search.previous_time;
    let span = search.next_time - search.previous_time;
    narrow(elapsed / span).to_bits()
}

/// Copies the four words at `from` to `result`.
fn copy_quaternion(e: &mut Engine, result: Ptr<NiQuaternion>, from: u32) {
    for word in 0..4u32 {
        let value = e.mem.u32(from.wrapping_add(4 * word));
        e.mem.set_u32(result.addr().wrapping_add(4 * word), value);
    }
}

// Typed from 00a28740
/// `NiRotKey::GenInterp` (Xbox PDB, `cdecl`, the quaternion returned through `result`): for key type
/// 4, always the type's function (`011f3cc0`) with the time itself, the key array and a null next
/// key, into a stack quaternion copied to `result` (the last index untouched). Otherwise, with one key
/// or at time `-FLT_MAX`, the first key's quaternion (+4); else the key search and the type's function
/// between the keys `index` and `next_index` with the ratio, the last index becoming `index` (a search
/// running off the end still interpolates, towards the key past the array). Answers `result`.
/// The exe keeps the key times it loads, and the ratio, in its own `count` argument's slot once it
/// has read the count; this keeps them in locals, as the translation does, and leaves the argument
/// words as they came (`cdecl`: the caller drops them).
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
    let time_value = f64::from(time);
    let keys = Keys::new(keys.addr(), u32::from(key_size));
    let function = |e: &Engine| {
        e.mem
            .u32(INTERPOLATIONS.wrapping_add(key_type.wrapping_mul(4)))
    };
    let lowest = count == 1 || time_value == e.global::<f64>(LOWEST_TIME);
    if key_type == WHOLE_ARRAY_TYPE {
        let function = function(e);
        let time_bits = time.to_bits();
        e.with_stack(16, |e, rotation| {
            e.call(function, &args![time_bits, keys.at(0), 0u32, rotation]);
            copy_quaternion(e, result, rotation.addr());
        });
        return result;
    }
    if lowest {
        copy_quaternion(e, result, keys.at(0).wrapping_add(4));
        return result;
    }
    let last = e.mem.u32(last_index.addr());
    let found = search(e, time_value, keys, count, last);
    let ratio = ratio(time_value, &found);
    let (key, next_key) = (keys.at(found.index), keys.at(found.next_index));
    let function = function(e);
    e.with_stack(16, |e, rotation| {
        e.call(function, &args![ratio, key, next_key, rotation]);
        e.mem.set_u32(last_index.addr(), found.index);
        copy_quaternion(e, result, rotation.addr());
    });
    result
}

/// Clamps the `float` at `at` as the exe's clamp blocks do: the value (through `FLD`/`FSTP`/`FLD` of
/// a local) below `low` stores `low`, otherwise above 1 stores 1; NaN stores nothing.
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
/// lie in one hemisphere; then every key's `W` is clamped to `[-1, 1]`, four keys at a time and then
/// the rest. As in the exe, the first loop runs `count - 1` times without a guard for `count == 0`.
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
