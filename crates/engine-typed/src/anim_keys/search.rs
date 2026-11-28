//! The key search the `GenInterp` functions share (the same code inlined in each of `NiFloatKey`,
//! `NiPosKey` and `NiRotKey::GenInterp`), and the ratio between the two keys it finds, at the
//! running thread's precision.

use super::Keys;
use crate::native::Real;
use engine_core::Engine;

/// `-FLT_MAX`, the time at which a `GenInterp` answers the first key (the exe compares with the
/// `double` at `010241b0`, which holds this float exactly).
pub(crate) const LOWEST_TIME: f32 = f32::MIN;

/// Where the key search stopped: the key before the time and the one after it, with their times as
/// the search last loaded them.
pub(crate) struct Search {
    pub(crate) index: u32,
    pub(crate) next_index: u32,
    pub(crate) previous_time: f64,
    /// 0 when the search loaded no later key (the value left on the x87 stack).
    pub(crate) next_time: f64,
}

/// The key search. It starts at the last index, back at key 0 when that key's time is after `time`,
/// then walks forward (four keys per round while at least four remain, counted signed, then one by
/// one up to `count - 1`, unsigned) to the first key whose time is not before `time` (`FCOM`: a NaN
/// on either side counts as before). Only loads and compares: the same on every thread.
pub(crate) fn search(e: &Engine, time: f64, keys: Keys, count: u32, last_index: u32) -> Search {
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

/// The ratio `(time - previous) / (next - previous)` the interpolation function is handed, as the
/// float the exe stores for it, each operation at the running thread's precision ([`Real`]). Every
/// time here is a float, so the narrowing to `R` is exact.
pub(crate) fn ratio<R: Real>(time: f64, search: &Search) -> u32 {
    let previous = R::narrow(search.previous_time);
    let elapsed = R::narrow(time) - previous;
    let span = R::narrow(search.next_time) - previous;
    (elapsed / span).store()
}

/// A value stored as a float and loaded back (`FSTP dword` then `FLD dword`).
#[inline(always)]
pub(crate) fn round<R: Real>(value: R) -> R {
    R::load(value.store())
}
