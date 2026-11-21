//! The shadow scene node's dominant light direction at a point (`00b5bca0`), which the world draw
//! asks twice a frame (through `00b803b0`) for the camera's lighting. When the sun is dim (night,
//! interiors) it weighs up to 100 lights of the node's whole light list; the exe fetches each with
//! an inlined copy of the light-at-an-index walk (`00b5ac20`'s loop at `00b5c03a`..`00b5c065`),
//! from the list's head every time, so a list of `n` lights takes about `n * n / 2` list steps per
//! call. Here the walk keeps its place ([`NodeLights`]): the same lights in the same order.
//!
//! **Floats.** Main thread only (the world draw). Every operation keeps the exe's x87 steps through
//! the exact helpers, each stored where the exe stores it, as the translation does; the square root
//! of a stored float, stored again as a float, is `f32::sqrt` (one correctly rounded root, as
//! `_CIsqrt` then the store).

use crate::dispatch;
use engine_core::ptr::Ptr;
use engine_core::x87::{
    x87_add, x87_div, x87_load32, x87_load64, x87_m32, x87_mul, x87_quiet_bits, x87_store32,
    x87_sub,
};
use engine_core::{typed_call, Engine};

use super::list::NodeLights;

/// The node's `pSunLight` (+0xe0); a `ShadowSceneLight`'s `spLight` (+0xf8); an `NiAVObject`'s
/// flags (+0x30) and world translation (+0x8c); an `NiLight`'s dimmer (+0xc4), colour (+0xd4),
/// direction (+0xf0) and radius (+0xe0).
const SUN_LIGHT: u32 = 0xe0;
const LIGHT_SP_LIGHT: u32 = 0xf8;
const FLAGS: u32 = 0x30;
const WORLD_TRANSLATE: u32 = 0x8c;
const DIMMER: u32 = 0xc4;
const COLOUR: u32 = 0xd4;
const DIRECTION: u32 = 0xf0;
const RADIUS: u32 = 0xe0;
/// The exe's data: the zero point (`011f426c`), 3.0 (`01021928`), the sun's luminance limit 0.4
/// (`010228a0`), 1000.0 (`01017b70`), 0.0 (`01012060`), all `double`s; the weight limit (`float`
/// 0.01, `01013ea4`); a function-local static's guard word (`011f9e78`) and its three arrays,
/// weights twice (`011f9838`, `011f96a8`, 100 `float`s each) and offsets (`011f99c8`, 100 points).
const ZERO_POINT: u32 = 0x011f_426c;
const LIGHT_SUM_DIVISOR: u32 = 0x0102_1928;
const SUN_LUMINANCE_LIMIT: u32 = 0x0102_28a0;
const DIRECTION_SCALE: u32 = 0x0101_7b70;
const ZERO: u32 = 0x0101_2060;
const WEIGHT_LIMIT: u32 = 0x0101_3ea4;
const STATIC_GUARD: u32 = 0x011f_9e78;
const WEIGHTS_A: u32 = 0x011f_9838;
const WEIGHTS_B: u32 = 0x011f_96a8;
const OFFSETS: u32 = 0x011f_99c8;
/// The accepted lights' offsets fill at most 0x4b0 bytes (100 points).
const OFFSETS_BYTES: u32 = 0x4b0;

fn read_words(e: &Engine, address: u32) -> [u32; 3] {
    [0, 4, 8].map(|offset| e.mem.u32(address.wrapping_add(offset)))
}

fn normalize(e: &mut Engine, point: u32) {
    typed_call!(
        dispatch::FAMILY,
        0x004a_0c10,
        dispatch::math::normalize(e, Ptr::new(point))
    );
}

/// `a` when `b < a`, else `b` (a NaN included).
fn keep_larger(a: f64, b: f64) -> f64 {
    if b < a {
        a
    } else {
        b
    }
}

/// An `NiLight`'s luminance: its dimmer times each component of its colour, summed `(g + r) + b`
/// and divided by `[01021928]`, as a float.
fn light_luminance(e: &Engine, ni_light: u32) -> u32 {
    let dimmer = x87_load32(e.mem.u32(ni_light.wrapping_add(DIMMER)));
    let part = |i: u32| {
        x87_store32(x87_mul(
            dimmer,
            x87_m32(e.mem.u32(ni_light.wrapping_add(COLOUR + 4 * i))),
        ))
    };
    let (r, g, b) = (part(0), part(1), part(2));
    let divisor: f64 = e.global(LIGHT_SUM_DIVISOR);
    let sum = x87_add(x87_add(x87_load32(g), x87_m32(r)), x87_m32(b));
    x87_store32(x87_div(sum, divisor))
}

/// The light's negated direction normalized in a local and scaled by `[01017b70]`.
fn scaled_reverse_direction(e: &mut Engine, ni_light: u32) -> [u32; 3] {
    e.with_stack(0xc, |e, local| {
        for word in 0..3u32 {
            let value = x87_load32(e.mem.u32(ni_light.wrapping_add(DIRECTION + 4 * word)));
            e.mem.set_u32(local.addr() + 4 * word, x87_store32(-value));
        }
        normalize(e, local.addr());
        let scale = x87_load64(e.global::<u64>(DIRECTION_SCALE));
        let x = x87_store32(x87_mul(x87_load32(e.mem.u32(local.addr())), scale));
        let y = x87_store32(x87_mul(x87_load32(e.mem.u32(local.addr() + 4)), scale));
        let z = x87_store32(x87_mul(scale, x87_m32(e.mem.u32(local.addr() + 8))));
        [x, y, z]
    })
}

/// One light of the weighing: its offset from the point, the weight `falloff * dot * luminance`
/// kept twice at `index` with the offset; answers the weight's bits.
fn weigh(e: &mut Engine, ni_light: u32, point: u32, direction: u32, bias: u32, index: u32) -> u32 {
    let mut delta = [0u32; 3];
    for (word, d) in (0u32..).zip(delta.iter_mut()) {
        let a = x87_load32(e.mem.u32(ni_light.wrapping_add(WORLD_TRANSLATE + 4 * word)));
        let b = e.mem.u32(point.wrapping_add(4 * word));
        *d = x87_store32(x87_sub(a, x87_m32(b)));
    }
    let unit = e.with_stack(0xc, |e, local| {
        for (word, d) in (0u32..).zip(delta) {
            e.mem.set_u32(local.addr() + 4 * word, d);
        }
        normalize(e, local.addr());
        read_words(e, local.addr())
    });
    let [dx, dy, dz] = read_words(e, direction);
    let y_term = x87_mul(x87_load32(dy), x87_m32(unit[1]));
    let x_term = x87_mul(x87_load32(dx), x87_m32(unit[0]));
    let z_term = x87_mul(x87_load32(dz), x87_m32(unit[2]));
    let mut dot = x87_store32(x87_add(x87_add(y_term, x_term), z_term));
    // `FLDZ; FCOM [dot]; TEST AH,0x41; JNZ`: 0 replaces a negative dot.
    if 0.0 > x87_m32(dot) {
        dot = x87_store32(0.0);
    }
    let luminance = light_luminance(e, ni_light);
    let (ex, ey, ez) = (
        x87_load32(delta[0]),
        x87_load32(delta[1]),
        x87_load32(delta[2]),
    );
    let squared = x87_store32(x87_add(
        x87_add(x87_mul(ey, ey), x87_mul(ex, ex)),
        x87_mul(ez, ez),
    ));
    let distance = f32::from_bits(squared).sqrt().to_bits();
    let q = x87_div(
        x87_sub(x87_load32(distance), x87_m32(bias)),
        x87_m32(e.mem.u32(ni_light.wrapping_add(RADIUS))),
    );
    let mut falloff = x87_store32(x87_sub(1.0, q));
    let zero: f64 = e.global(ZERO);
    if x87_load32(falloff) < zero {
        falloff = x87_store32(0.0);
    }
    if x87_load32(falloff) > 1.0 {
        falloff = x87_store32(1.0);
    }
    let weight = x87_mul(
        x87_mul(x87_load32(falloff), x87_m32(dot)),
        x87_m32(luminance),
    );
    let weight = x87_quiet_bits(x87_store32(weight));
    e.set_global(WEIGHTS_A + 4 * index, weight);
    e.set_global(WEIGHTS_B + 4 * index, weight);
    let slot = OFFSETS + 0xc * index;
    for (word, d) in (0u32..).zip(delta) {
        e.set_global(slot + 4 * word, d);
    }
    weight
}

// Typed from 00b5bca0
/// `thiscall(node, point, direction, out)`, `RET 0xc`: the dominant light direction at the point.
/// When the sun's luminance is above `[010228a0]`, the offset is the sun's reverse direction scaled
/// by `[01017b70]`. Otherwise up to 100 lights of `lLightList` whose `NiLight` is not culled are
/// weighed (see [`weigh`]; the walk goes on past them only to stop at the count or a null light),
/// and with a counted weight above `[01013ea4]` each offset with a positive share of the sum is
/// added in by that share; with none, the sun's direction as above. The output is the point plus
/// the offset; the count (the sun's 1, or the weighed lights') is answered as 0 or 1.
pub fn dominant_light_direction(
    e: &mut Engine,
    this: Ptr,
    point: Ptr,
    direction: Ptr,
    out: Ptr,
) -> u32 {
    let node = this.addr();
    let point = point.addr();
    let sun = e.mem.u32(node.wrapping_add(SUN_LIGHT));
    let sun_light = e.mem.u32(sun.wrapping_add(LIGHT_SP_LIGHT));
    let luminance = light_luminance(e, sun_light);
    let base = read_words(e, ZERO_POINT);
    let mut result = base;
    let mut count = 0u32;
    let limit: f64 = e.global(SUN_LUMINANCE_LIMIT);
    let mut use_sun = false;
    if x87_load32(luminance) > limit {
        result = scaled_reverse_direction(e, sun_light);
        count = 1;
    } else {
        let guard: u32 = e.global(STATIC_GUARD);
        if guard & 1 == 0 {
            e.set_global(STATIC_GUARD, guard | 1);
        }
        let first_max = x87_store32(keep_larger(x87_load32(base[0]), x87_load32(base[1])));
        let bias = x87_store32(keep_larger(x87_load32(first_max), x87_load32(base[2])));
        let mut sum = x87_store32(0.0);
        let mut positive = 0i32;
        let mut index = 0u32;
        let mut list_index = 0u32;
        let mut lights = NodeLights::new();
        let mut light = lights.light_at(e, node, 0);
        if light == 0 {
            use_sun = true;
        } else {
            while 0xc * index < OFFSETS_BYTES {
                let ni_light = e.mem.u32(light.wrapping_add(LIGHT_SP_LIGHT));
                if e.mem.u32(ni_light.wrapping_add(FLAGS)) & 1 == 0 {
                    let weight = weigh(e, ni_light, point, direction.addr(), bias, index);
                    let weight_limit = e.global::<u32>(WEIGHT_LIMIT);
                    if x87_load32(weight_limit) < x87_load32(weight) {
                        positive += 1;
                    }
                    sum = x87_store32(x87_add(x87_load32(weight), x87_m32(sum)));
                    index += 1;
                }
                // The next light: the 16-bit index, stopping at the count or a null light.
                list_index = list_index.wrapping_add(1);
                light = lights.light_at(e, node, list_index & 0xffff);
                if light == 0 {
                    break;
                }
            }
            if positive <= 0 {
                use_sun = true;
            } else {
                let total = x87_load32(sum);
                for k in 0..index {
                    let share = x87_store32(x87_div(
                        x87_load32(e.global::<u32>(WEIGHTS_B + 4 * k)),
                        total,
                    ));
                    let s = x87_load32(share);
                    if s.partial_cmp(&0.0) != Some(core::cmp::Ordering::Greater) {
                        continue;
                    }
                    let slot = OFFSETS + 0xc * k;
                    let tx = x87_store32(x87_mul(x87_load32(e.global::<u32>(slot)), s));
                    let ty = x87_store32(x87_mul(x87_load32(e.global::<u32>(slot + 4)), s));
                    let tz = x87_store32(x87_mul(s, x87_m32(e.global::<u32>(slot + 8))));
                    result[0] = x87_store32(x87_add(x87_load32(tx), x87_m32(result[0])));
                    result[1] = x87_store32(x87_add(x87_load32(result[1]), x87_m32(ty)));
                    result[2] = x87_store32(x87_add(x87_load32(result[2]), x87_m32(tz)));
                }
                count = index;
            }
        }
    }
    if use_sun {
        let sun = e.mem.u32(node.wrapping_add(SUN_LIGHT));
        let sun_light = e.mem.u32(sun.wrapping_add(LIGHT_SP_LIGHT));
        result = scaled_reverse_direction(e, sun_light);
        count = 1;
    }
    for (word, r) in (0u32..).zip(result) {
        let p = x87_load32(e.mem.u32(point.wrapping_add(4 * word)));
        let value = x87_store32(x87_add(p, x87_m32(r)));
        e.mem.set_u32(out.addr().wrapping_add(4 * word), value);
    }
    u32::from(count != 0)
}
