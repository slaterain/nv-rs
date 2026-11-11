//! The lighting shaders' per-light set-up (`00b70820`): what the shadow light shader's light loop
//! ([`super::geometry::lights`]) runs for each of a geometry's lights, about 600 times a frame in the
//! crowded street. It leaves the light's colour, its position or direction and its kind in the shaders'
//! scratch rows and, for a light that is not a point light, the ambient colour.
//!
//! **Floats.** Only the main thread runs it (the thread census), so its maths is plain `f32`, each
//! operation the exe's in its order ([`crate::native`]): the exe's own rounding at the main thread's
//! 24-bit precision. A float the exe copies through the x87 stack is copied as it is.

use crate::native::{add_d, mul_d, narrow};
use engine_core::prelude::*;

/// The zero colour (4 words, `011a9bd0`).
const ZERO_COLOUR: u32 = 0x011a_9bd0;
/// The rendering object (`011f91e0`; the first word its geometry, whose +0xa8 is the shader property:
/// used when its word at +0x1c is 8 to 12).
const RENDERING_PASS: u32 = 0x011f_91e0;
/// What the set-up reads: three bytes (the HDR switch `011f941e`, `011f91de`, `011f9427`), the
/// brightness threshold (`011f947c`), the extra scale (`011f9190`) and the 16-bit pass type
/// (`011f91e4`).
const HDR: u32 = 0x011f_941e;
const SKIP_AMBIENT: u32 = 0x011f_91de;
const NO_EXTRA_SCALE: u32 = 0x011f_9427;
const THRESHOLD: u32 = 0x011f_947c;
const EXTRA_SCALE: u32 = 0x011f_9190;
const PASS_TYPE: u32 = 0x011f_91e4;
/// The `double`s: two luminance weights (`010784d0` on green, `01026998` on red and added once more
/// as it is), the brightness offset (`0101ffa0`) and 1.0 (`01012070`).
const GREEN_WEIGHT: u32 = 0x0107_84d0;
const RED_WEIGHT: u32 = 0x0102_6998;
const BRIGHTNESS_OFFSET: u32 = 0x0101_ffa0;
const ONE: u32 = 0x0101_2070;
/// What it fills: the ambient colour (`011fa0c0`, 4 words), the per-light colours (`011fa0d0`, 16
/// bytes each), the per-light directions (`011fa1f0`, 16 bytes each), the per-light kinds (`011fa070`,
/// a word each) and the last light's flag byte (`011f91a6`).
const AMBIENT_COLOUR: u32 = 0x011f_a0c0;
const LIGHT_COLOURS: u32 = 0x011f_a0d0;
const LIGHT_DIRECTIONS: u32 = 0x011f_a1f0;
const LIGHT_KINDS: u32 = 0x011f_a070;
const LAST_LIGHT_FLAG: u32 = 0x011f_91a6;
/// The neutral colour a point light gets below full `scale` (3 words, `011f4998`).
const DIMMED_COLOUR: u32 = 0x011f_4998;

#[inline(always)]
fn float_word(e: &Engine, address: u32) -> f32 {
    e.mem.f32(address)
}

/// The rendering geometry's shader property (`[[011f91e0]] + 0xa8`) when its word at +0x1c is 8 to 12
/// (signed), else null.
fn shader_object(e: &Engine) -> u32 {
    let geometry = e.mem.u32(e.global::<u32>(RENDERING_PASS));
    let object = e.mem.u32(geometry.wrapping_add(0xa8));
    if object == 0 {
        return 0;
    }
    let kind = e.mem.u32(object.wrapping_add(0x1c)) as i32;
    if (8..=12).contains(&kind) {
        object
    } else {
        0
    }
}

/// The brightness step: with the threshold above zero (else `None`), the luminance `red * w2 + green *
/// w1 + blue + w2` (stored as a float) at or below zero makes all three the threshold; otherwise each
/// component plus the offset is scaled by threshold / luminance (1.0 when the luminance is above the
/// threshold), each step in the exe's order.
#[allow(clippy::neg_cmp_op_on_partial_ord)] // NaN-aware as the exe: not above, including unordered.
fn brighten(e: &Engine, colour: [f32; 3], threshold: f32) -> Option<[f32; 3]> {
    if !(threshold > 0.0) {
        return None;
    }
    let [x, y, z] = colour;
    let green = mul_d(y, e.global::<f64>(GREEN_WEIGHT));
    let red_weight = e.global::<f64>(RED_WEIGHT);
    let red = mul_d(x, red_weight);
    let sum = (red + green) + z;
    let luminance = add_d(sum, red_weight);
    if luminance <= 0.0 {
        return Some([threshold; 3]);
    }
    let ratio = if luminance > threshold {
        1.0
    } else {
        threshold / luminance
    };
    let offset = e.global::<f64>(BRIGHTNESS_OFFSET);
    let red = add_d(x, offset) * ratio;
    let green = add_d(y, offset) * ratio;
    let blue = ratio * add_d(z, offset);
    Some([red, green, blue])
}

/// The light's strength (the float at +0xc4 of its data), made 1.0 when above 1.0 unless the HDR
/// switch is set.
fn capped_strength(e: &Engine, data: u32) -> f32 {
    let strength = float_word(e, data.wrapping_add(0xc4));
    if e.global::<u8>(HDR) == 0 && f64::from(strength) > e.global::<f64>(ONE) {
        1.0
    } else {
        strength
    }
}

/// A point scaled by a float (`00439180`, the pathing family's).
fn scale_point(e: &mut Engine, point: u32, scale: u32) {
    let _: Ptr<_> = typed_call!(
        crate::pathing::FAMILY,
        0x0043_9180,
        crate::pathing::point::scale_by(e, Ptr::new(point), f32::from_bits(scale))
    );
}

// Typed from 00b70820
/// Sets lighting-shader light `index` (below 8; unnamed in the engine map; `cdecl (index, light,
/// scale)`). Without a light the light's colour row gets the zero colour. With one (its data at +0xf8,
/// its flags at +0xf4 and +0xf5): a point light (flag +0xf4) with data sets row `index + 0xb` of the
/// scratch rows (`004e20c0`) to its position (+0x8c, w 1.0); another light with data sets row `index +
/// 0x13` to its unitized direction (+0xf0, w the shader property's vtable +0x10c or 1.0) and, unless the
/// byte at `011f91de` is set while the flag +0xf5 is clear, the ambient colour `011fa0c0` to the data's
/// +0xc8 colour times its strength (`0040ebd0` against 1.0 unless the HDR switch), brightened and times
/// `scale` (w 1.0). Then the light's colour (+0xd4 times the capped strength, times `scale`, times the
/// light's +0xd0; a point light below full `scale` takes `011f4998`, another light with the HDR switch
/// and not `011f9427` is scaled by `011f9190`; in pass types 0x154 to 0x177 the shader property's vtable
/// +0x10c scales it too) goes to its row at `011fa0d0` with the light's +0xd4 as w; a point light's row
/// at `011fa1f0` gets (its radius +0xe0, 0, 0, 1). The flag byte `011f91a6` takes the light's +0xf4 and
/// the kind word at `011fa070` 2 for a point light, else 1.
pub fn light_setup(e: &mut Engine, index: u32, light: Ptr, scale: f32) {
    if index >= 8 {
        return;
    }
    let light = light.addr();
    let row = LIGHT_COLOURS.wrapping_add(index << 4);
    if light == 0 {
        for i in 0..4u32 {
            let value = e.global::<u32>(ZERO_COLOUR + 4 * i);
            e.set_global(row + 4 * i, value);
        }
        return;
    }
    let ambient_allowed = e.mem.u8(light.wrapping_add(0xf5));
    let data = e.mem.u32(light.wrapping_add(0xf8));
    let is_point = e.mem.u8(light.wrapping_add(0xf4));
    let object = shader_object(e);
    if is_point != 0 {
        if data != 0 {
            let position: [u32; 3] =
                std::array::from_fn(|i| e.mem.u32(data.wrapping_add(0x8c + 4 * idx(i))));
            typed_call!(
                leaf::FAMILY,
                0x004e_20c0,
                leaf::store_colour(
                    e,
                    index + 0xb,
                    position[0],
                    position[1],
                    position[2],
                    1.0f32
                )
            );
        }
    } else {
        if data != 0 {
            let direction = e.with_stack(0xc, |e, point| {
                let point = point.addr();
                for i in 0..3u32 {
                    let value = e.mem.u32(data.wrapping_add(0xf0 + 4 * i));
                    e.mem.set_u32(point + 4 * i, value);
                }
                typed_call!(
                    crate::dispatch::FAMILY,
                    0x004a_0c10,
                    crate::dispatch::math::normalize(e, Ptr::new(point))
                );
                [e.mem.u32(point), e.mem.u32(point + 4), e.mem.u32(point + 8)]
            });
            let w = if object != 0 {
                narrow(e.vcall(object, 0x10c, &args![]).f64())
            } else {
                1.0
            };
            typed_call!(
                leaf::FAMILY,
                0x004e_20c0,
                leaf::store_colour(e, index + 0x13, direction[0], direction[1], direction[2], w)
            );
        }
        let skipped = e.global::<u8>(SKIP_AMBIENT) != 0 && ambient_allowed == 0;
        if !skipped {
            let mut strength = float_word(e, data.wrapping_add(0xc4));
            if e.global::<u8>(HDR) == 0 {
                // The smaller of 1 and the strength (`0040ebd0`, `cdecl(a, b)`, answering in ST0).
                strength = typed_call!(
                    crate::draw_f::FAMILY,
                    0x0040_ebd0,
                    crate::draw_f::helpers::float_min(e, 1.0, strength)
                );
            }
            let base: [f32; 3] =
                std::array::from_fn(|i| float_word(e, data.wrapping_add(0xc8 + 4 * idx(i))));
            let mut colour = [
                base[0] * strength,
                base[1] * strength,
                strength * float_word(e, data.wrapping_add(0xd0)),
            ];
            let threshold = e.global::<f32>(THRESHOLD);
            if let Some(bright) = brighten(e, colour, threshold) {
                colour = bright;
            }
            for (i, component) in colour.iter().enumerate() {
                e.set_global(AMBIENT_COLOUR + 4 * idx(i), *component * scale);
            }
            e.set_global(AMBIENT_COLOUR + 0xc, 1.0f32.to_bits());
        }
    }
    let strength = capped_strength(e, data);
    let weight = float_word(e, light.wrapping_add(0xd0));
    let base: [f32; 3] =
        std::array::from_fn(|i| float_word(e, data.wrapping_add(0xd4 + 4 * idx(i))));
    let colour = [
        base[0] * strength,
        base[1] * strength,
        strength * float_word(e, data.wrapping_add(0xdc)),
    ];
    let scaled = colour.map(|component| component * scale);
    e.with_stack(0xc, |e, point| {
        let point = point.addr();
        let products = [scaled[0] * weight, weight * scaled[1], weight * scaled[2]];
        for (i, value) in products.iter().enumerate() {
            e.mem.set_f32(point + 4 * idx(i), *value);
        }
        if is_point != 0 {
            if scale < 1.0 {
                for i in 0..3u32 {
                    let value = e.global::<u32>(DIMMED_COLOUR + 4 * i);
                    e.mem.set_u32(point + 4 * i, value);
                }
            }
        } else if e.global::<u8>(HDR) != 0 && e.global::<u8>(NO_EXTRA_SCALE) == 0 {
            let extra = e.global::<u32>(EXTRA_SCALE);
            scale_point(e, point, extra);
        }
        let pass = e.global::<u16>(PASS_TYPE).wrapping_sub(0x154);
        let alpha = e.mem.u32(light.wrapping_add(0xd4));
        if pass <= 0x23 && object != 0 {
            let factor = narrow(e.vcall(object, 0x10c, &args![]).f64());
            scale_point(e, point, factor.to_bits());
        }
        for i in 0..3u32 {
            let value = e.mem.u32(point + 4 * i);
            e.set_global(row + 4 * i, value);
        }
        e.set_global(row + 0xc, alpha);
    });
    if is_point != 0 {
        let direction = LIGHT_DIRECTIONS.wrapping_add(index << 4);
        let first = e.mem.u32(data.wrapping_add(0xe0));
        // The exe also reads +0xe4 and +0xe8 and overwrites them with 0.0.
        let _ = (
            e.mem.u32(data.wrapping_add(0xe4)),
            e.mem.u32(data.wrapping_add(0xe8)),
        );
        e.set_global(direction, first);
        e.set_global(direction + 4, 0u32);
        e.set_global(direction + 8, 0u32);
        e.set_global(direction + 0xc, 1.0f32.to_bits());
    }
    e.set_global(LAST_LIGHT_FLAG, is_point);
    e.set_global(LIGHT_KINDS + 4 * index, u32::from(is_point != 0) + 1);
}

/// A loop's small index as a word.
#[allow(clippy::cast_possible_truncation)]
fn idx(i: usize) -> u32 {
    i as u32
}
