//! `ShadowSceneLight::ComputeWeightedShadowLight` (`00b9d150`): where a shadow caster's light goes
//! and what colour it has, from the lights around the caster. The shadow preparation (`00871290`)
//! calls it for each shadow light it recomputes, every frame, with no lighting property: the lights
//! are then the shadow scene node's whole light list, which the exe asks for by index
//! (`00b5ac20`, a walk from the list's head each time), twice over, so a scene with `n` lights costs
//! about `n * n` list steps per shadow light. Here the walk keeps its place in the list
//! ([`NodeLights`]) and answers the same light for each index.
//!
//! **Floats.** The shadow preparation runs on the main thread. Every operation keeps the exe's x87 steps through the exact helpers
//! (each stored where the exe stores it), as the translation does; the square root of a stored float,
//! stored again as a float, is `f32::sqrt` (one correctly rounded root, as `_CIsqrt` then the store).

use crate::{dispatch, draw, scene};
use engine_core::ptr::Ptr;
use engine_core::x87::{
    x87_add, x87_div, x87_load32, x87_load64, x87_m32, x87_mul, x87_quiet_bits, x87_store32,
    x87_sub,
};
use engine_core::{args, typed_call, Engine};

use super::list::NodeLights;
use super::ShadowSceneLight;

/// The scene node pointer (`011f91c8`) and its sun light (`pSunLight`, +0xe0).
const SCENE_NODE: u32 = 0x011f_91c8;
const SUN_LIGHT: u32 = 0xe0;
/// `NiAVObject` members: the flags (+0x30), the world translation (+0x8c), the scale (+0x98).
const FLAGS: u32 = 0x30;
const WORLD_TRANSLATE: u32 = 0x8c;
const WORLD_SCALE: u32 = 0x98;
/// The extra data (`00a5bdd0` with the name at `012031e4`): its offset (+0xc) and extents (+0x18).
const GET_EXTRA_DATA: u32 = 0x00a5_bdd0;
const EXTRA_DATA_NAME: u32 = 0x0120_31e4;
/// The zero point (`011f426c`) and the colour sum's start (`011f4998`).
const ZERO_POINT: u32 = 0x011f_426c;
const COLOUR_START: u32 = 0x011f_4998;
/// The weights of the counted lights (`float[40]`, written twice; the second array is read).
const WEIGHTS_A: u32 = 0x011f_ff40;
const WEIGHTS_B: u32 = 0x011f_fea0;
/// The exe's constants (see the translation): 0.0, the radius share and divisor, the
/// lowest heights, the radius cap and extent cap, the offset and specular scales, the colour share,
/// the lowest greys and 1.0.
const ZERO: u32 = 0x0101_2060;
const RADIUS_SHARE: u32 = 0x010b_7e98;
const RADIUS_DIVISOR: u32 = 0x0107_31a8;
const LOWEST_HEIGHT: u32 = 0x0101_de30;
const LOWEST_HEIGHT_FLOAT: u32 = 0x0101_6264;
const RADIUS_CAP: u32 = 0x0102_31d8;
const EXTENT_CAP: u32 = 0x0103_2adc;
const OFFSET_SCALE: u32 = 0x010b_7e90;
const SPECULAR_SCALE: u32 = 0x0102_40b0;
const COLOUR_SHARE: u32 = 0x0102_90c0;
const LOWEST_GREY: u32 = 0x0102_17d8;
const LOWEST_GREY_FLOAT: u32 = 0x0103_0ff0;
const ONE: u32 = 0x0101_2070;
/// The light's `spShadowScene` (+0x128) and `spLight` (+0xf8), its `bCastShadow` (+0xec).
const LIGHT_SHADOW_SCENE: u32 = 0x128;
const LIGHT_SP_LIGHT: u32 = 0xf8;
const LIGHT_CAST_SHADOW: u32 = 0xec;

fn read_words(e: &Engine, address: u32) -> [u32; 3] {
    [0, 4, 8].map(|offset| e.mem.u32(address.wrapping_add(offset)))
}

fn write_words(e: &mut Engine, address: u32, words: [u32; 3]) {
    for (i, word) in (0u32..).zip(words) {
        e.mem.set_u32(address.wrapping_add(4 * i), word);
    }
}

fn add32(a: u32, b: u32) -> u32 {
    x87_store32(x87_add(x87_load32(a), x87_m32(b)))
}

fn sub32(a: u32, b: u32) -> u32 {
    x87_store32(x87_sub(x87_load32(a), x87_m32(b)))
}

/// The three words of a point times a loaded factor, each stored as a float.
fn scaled_words(words: [u32; 3], factor: f64) -> [u32; 3] {
    [
        x87_store32(x87_mul(x87_load32(words[0]), factor)),
        x87_store32(x87_mul(x87_load32(words[1]), factor)),
        x87_store32(x87_mul(factor, x87_m32(words[2]))),
    ]
}

/// `a` when `b < a`, else `b` (a NaN included).
fn keep_larger(a: f64, b: f64) -> f64 {
    if b < a {
        a
    } else {
        b
    }
}

/// `(y * y + x * x) + z * z` in the exe's order.
fn squared_length(x: f64, y: f64, z: f64) -> f64 {
    x87_add(x87_add(x87_mul(y, y), x87_mul(x, x)), x87_mul(z, z))
}

/// The square root of a stored float, stored as a float.
fn sqrt_stored(squared: u32) -> u32 {
    f32::from_bits(squared).sqrt().to_bits()
}

fn normalize(e: &mut Engine, point: u32) {
    typed_call!(
        dispatch::FAMILY,
        0x004a_0c10,
        dispatch::math::normalize(e, Ptr::new(point))
    );
}

/// The first light of the walk: the property's first active non-shadow light, or the node's light 0.
fn first_light(e: &mut Engine, cursor: &mut NodeLights, property: u32, iterator: u32) -> u32 {
    if property != 0 {
        typed_call!(
            draw::FAMILY,
            0x00b7_0600,
            draw::lights::first_active_non_shadow_light(e, Ptr::new(property), Ptr::new(iterator))
        )
    } else {
        let node = e.global::<u32>(SCENE_NODE);
        cursor.light_at(e, node, 0)
    }
}

/// The next light of the walk (`index` used without a property).
fn next_light(
    e: &mut Engine,
    cursor: &mut NodeLights,
    property: u32,
    iterator: u32,
    index: u32,
) -> u32 {
    if property != 0 {
        typed_call!(
            draw::FAMILY,
            0x00b7_0700,
            draw::lights::next_active_non_shadow_light(e, Ptr::new(property), Ptr::new(iterator))
        )
    } else {
        let node = e.global::<u32>(SCENE_NODE);
        cursor.light_at(e, node, index)
    }
}

// Typed from 00b9d150
/// `thiscall(light, property, three words)`: see the translation's account. The position is the
/// shadow scene's, moved by its scale times the extra data's offset; the radius the largest of the
/// scaled extents (or of the zero point without extra data). Up to 40 usable lights (not casting
/// shadows, `NiLight` not culled, dimmer above 0) get a weight in [0, 1]; without weight the sun
/// alone gives the direction and colour, otherwise each counted light blends with the sun by its
/// share. The `NiLight` is placed along the direction, updated, and gets its grey diffuse and its
/// specular, its revision up by 2.
pub fn compute_weighted(
    e: &mut Engine,
    this: Ptr<ShadowSceneLight>,
    property: u32,
    _x: u32,
    _y: u32,
    _z: u32,
) {
    let this = this.addr();
    e.with_stack(0x30, |e, frame| {
        // Locals: the light iterator (+0x00), the sun direction (+0x04), a light's direction
        // (+0x10), the direction (+0x1c).
        let iterator = frame.addr();
        let sun_direction = iterator + 0x04;
        let light_direction = iterator + 0x10;
        let direction = iterator + 0x1c;
        let mut cursor = NodeLights::new();
        let scene = e.mem.u32(this.wrapping_add(LIGHT_SHADOW_SCENE));
        let name = e.global::<u32>(EXTRA_DATA_NAME);
        let extra = e.call(GET_EXTRA_DATA, &args![scene, name]).u32();
        let scene = e.mem.u32(this.wrapping_add(LIGHT_SHADOW_SCENE));
        let mut position = read_words(e, scene.wrapping_add(WORLD_TRANSLATE));
        let extents = if extra != 0 {
            let scale = x87_load32(x87_quiet_bits(e.mem.u32(scene.wrapping_add(WORLD_SCALE))));
            let offset = scaled_words(read_words(e, extra.wrapping_add(0xc)), scale);
            position = [
                add32(offset[0], position[0]),
                add32(position[1], offset[1]),
                add32(position[2], offset[2]),
            ];
            scaled_words(read_words(e, extra.wrapping_add(0x18)), scale)
        } else {
            read_words(e, ZERO_POINT)
        };
        let larger = keep_larger(x87_load32(extents[0]), x87_load32(extents[1]));
        let larger = x87_load32(x87_store32(larger));
        let radius = x87_store32(keep_larger(larger, x87_load32(extents[2])));

        // The weights of the usable lights.
        let mut sum = x87_store32(0.0);
        let mut count = 0u32;
        let mut index = 0u32;
        let mut light = first_light(e, &mut cursor, property, iterator);
        while light != 0 && count < 0x28 {
            let usable = e.mem.u8(light.wrapping_add(LIGHT_CAST_SHADOW)) == 0 && {
                let ni_light = e.mem.u32(light.wrapping_add(LIGHT_SP_LIGHT));
                e.mem.u8(ni_light.wrapping_add(FLAGS)) & 1 == 0
                    && 0.0 < x87_m32(e.mem.u32(ni_light.wrapping_add(0xc4)))
            };
            if usable {
                let ni_light = e.mem.u32(light.wrapping_add(LIGHT_SP_LIGHT));
                let mut delta = [0u32; 3];
                for (i, d) in (0u32..).zip(delta.iter_mut()) {
                    let p = e.mem.u32(ni_light.wrapping_add(WORLD_TRANSLATE + 4 * i));
                    *d = sub32(p, position[i as usize]);
                }
                let squared = x87_store32(squared_length(
                    x87_load32(delta[1]),
                    x87_load32(delta[0]),
                    x87_load32(delta[2]),
                ));
                let distance = sqrt_stored(squared);
                let range = e.mem.u32(ni_light.wrapping_add(0xe0));
                let ratio = x87_div(
                    x87_sub(x87_load32(distance), x87_m32(radius)),
                    x87_m32(range),
                );
                let mut weight = x87_store32(x87_sub(1.0, ratio));
                let zero: f64 = e.global(ZERO);
                if x87_load32(weight) < zero {
                    weight = x87_store32(0.0);
                }
                let w = x87_load32(weight);
                let weight = x87_store32(if 1.0 < w { 1.0 } else { w });
                e.mem.set_u32(WEIGHTS_A + 4 * count, weight);
                e.mem.set_u32(WEIGHTS_B + 4 * count, weight);
                count += 1;
                sum = x87_store32(x87_add(x87_load32(weight), x87_m32(sum)));
            }
            index += 1;
            light = next_light(e, &mut cursor, property, iterator, index & 0xffff);
        }

        let node = e.global::<u32>(SCENE_NODE);
        write_words(e, direction, read_words(e, ZERO_POINT));
        let mut colour = read_words(e, COLOUR_START);
        let sun = e.mem.u32(
            e.mem
                .u32(node.wrapping_add(SUN_LIGHT))
                .wrapping_add(LIGHT_SP_LIGHT),
        );
        let sun_words = read_words(e, sun.wrapping_add(0xf0));
        let share: f64 = e.global(RADIUS_SHARE);
        let reach = x87_mul(x87_load32(radius), share);
        write_words(
            e,
            sun_direction,
            [
                x87_store32(-x87_load32(sun_words[0])),
                x87_store32(-x87_load32(sun_words[1])),
                x87_store32(x87_mul(-x87_load32(sun_words[2]), reach)),
            ],
        );
        normalize(e, sun_direction);
        if x87_m32(sum) == 0.0 {
            // No weight: the sun alone.
            write_words(e, direction, read_words(e, sun_direction));
            colour = read_words(e, sun.wrapping_add(0xd4));
            normalize(e, direction);
            let divisor: f64 = e.global(RADIUS_DIVISOR);
            let lowest = x87_store32(x87_div(x87_load32(radius), divisor));
            if x87_load32(e.mem.u32(direction + 8)) < x87_load32(lowest) {
                e.mem.set_u32(direction + 8, x87_quiet_bits(lowest));
            }
        } else {
            let mut cursor = NodeLights::new();
            let mut light = if property != 0 {
                first_light(e, &mut cursor, property, iterator)
            } else {
                cursor.light_at(e, node, 0)
            };
            let mut node_index = 0u32;
            let mut walked = 0u32;
            while walked < count {
                if light != 0 && e.mem.u8(light.wrapping_add(LIGHT_CAST_SHADOW)) == 0 {
                    let ni_light = e.mem.u32(light.wrapping_add(LIGHT_SP_LIGHT));
                    if e.mem.u8(ni_light.wrapping_add(FLAGS)) & 1 == 0 {
                        blend_light(
                            e,
                            ni_light,
                            WEIGHTS_B + 4 * walked,
                            sum,
                            &position,
                            [sun_direction, light_direction, direction],
                            sun,
                            &mut colour,
                        );
                    }
                }
                light = if property != 0 {
                    next_light(e, &mut cursor, property, iterator, 0)
                } else {
                    node_index += 1;
                    next_light(e, &mut cursor, 0, iterator, node_index)
                };
                walked += 1;
            }
            normalize(e, direction);
            let lowest: f64 = e.global(LOWEST_HEIGHT);
            if x87_load32(e.mem.u32(direction + 8)) < lowest {
                let height = x87_quiet_bits(e.global::<u32>(LOWEST_HEIGHT_FLOAT));
                e.mem.set_u32(direction + 8, height);
            }
        }
        normalize(e, direction);

        // The shadow light.
        let cap: f64 = e.global(RADIUS_CAP);
        let r = x87_load32(radius);
        let reach = x87_store32(if cap < r {
            x87_load32(e.global::<u32>(EXTENT_CAP))
        } else {
            r
        });
        let ni_light = e.mem.u32(this.wrapping_add(LIGHT_SP_LIGHT));
        let offset = scaled_words(read_words(e, direction), x87_load32(reach));
        let scale: f64 = e.global(OFFSET_SCALE);
        let offset = scaled_words(offset, scale);
        write_words(
            e,
            ni_light.wrapping_add(0x58),
            [
                add32(offset[0], position[0]),
                add32(offset[1], position[1]),
                add32(offset[2], position[2]),
            ],
        );
        let specular_scale: f64 = e.global(SPECULAR_SCALE);
        let specular = x87_store32(x87_mul(x87_load32(reach), specular_scale));
        let ni_light = e.mem.u32(this.wrapping_add(LIGHT_SP_LIGHT));
        e.with_stack(0xc, |e, local| {
            e.mem.set_u32(local.addr(), x87_store32(0.0));
            for byte in 4..9u32 {
                e.mem.set_u8(local.addr() + byte, 0);
            }
            typed_call!(
                scene::FAMILY,
                0x00a5_9c60,
                scene::walk::update(e, Ptr::new(ni_light), local.addr())
            );
        });
        let colour_share: f64 = e.global(COLOUR_SHARE);
        let grey = x87_add(
            x87_add(
                x87_mul(x87_load32(colour[1]), colour_share),
                x87_mul(x87_load32(colour[0]), colour_share),
            ),
            x87_mul(colour_share, x87_m32(colour[2])),
        );
        let grey = x87_load32(x87_store32(grey));
        let lowest: f64 = e.global(LOWEST_GREY);
        let grey = if grey < lowest {
            x87_load32(e.global::<u32>(LOWEST_GREY_FLOAT))
        } else {
            grey
        };
        let grey = x87_load32(x87_store32(grey));
        let one: f64 = e.global(ONE);
        let grey = x87_store32(if one < grey { 1.0 } else { grey });
        let ni_light = e.mem.u32(this.wrapping_add(LIGHT_SP_LIGHT));
        write_words(e, ni_light.wrapping_add(0xd4), [grey; 3]);
        let revision = e.mem.u32(ni_light.wrapping_add(0xa8));
        e.mem
            .set_u32(ni_light.wrapping_add(0xa8), revision.wrapping_add(1));
        let ni_light = e.mem.u32(this.wrapping_add(LIGHT_SP_LIGHT));
        let revision = e.mem.u32(ni_light.wrapping_add(0xa8));
        e.mem
            .set_u32(ni_light.wrapping_add(0xa8), revision.wrapping_add(1));
        let zero = x87_store32(0.0);
        write_words(
            e,
            ni_light.wrapping_add(0xe0),
            [x87_quiet_bits(specular), zero, zero],
        );
    });
}

/// One counted light of the second walk: with a positive share `w = weight / sum` it adds
/// `w * (b * light direction + (1 - b) * sun direction)` to the direction and the same blend of the
/// colours to `colour` (`b` its weight).
#[allow(clippy::too_many_arguments)]
fn blend_light(
    e: &mut Engine,
    ni_light: u32,
    slot: u32,
    sum: u32,
    position: &[u32; 3],
    [sun_direction, light_direction, direction]: [u32; 3],
    sun: u32,
    colour: &mut [u32; 3],
) {
    let w = x87_store32(x87_div(x87_load32(e.mem.u32(slot)), x87_m32(sum)));
    let w = x87_load32(w);
    if w.partial_cmp(&0.0) != Some(core::cmp::Ordering::Greater) {
        return;
    }
    let b = x87_load32(e.mem.u32(slot));
    let rest = x87_sub(1.0, b);
    let mut delta = [0u32; 3];
    for (i, d) in (0u32..).zip(delta.iter_mut()) {
        let p = e.mem.u32(ni_light.wrapping_add(WORLD_TRANSLATE + 4 * i));
        *d = sub32(p, position[i as usize]);
    }
    write_words(e, light_direction, delta);
    normalize(e, light_direction);
    // `FST qword` then `FSTP dword` of 1 - b.
    let rest = x87_load32(x87_store32(x87_load64(rest.to_bits())));
    let sun_part = read_words(e, sun_direction);
    let sun_part = [
        x87_store32(x87_mul(rest, x87_m32(sun_part[0]))),
        x87_store32(x87_mul(x87_load32(sun_part[1]), rest)),
        x87_store32(x87_mul(rest, x87_m32(sun_part[2]))),
    ];
    let b = x87_load32(x87_quiet_bits(e.mem.u32(slot)));
    let light_part = scaled_words(read_words(e, light_direction), b);
    let mut part = [0u32; 3];
    for (i, p) in part.iter_mut().enumerate() {
        *p = add32(light_part[i], sun_part[i]);
    }
    // `FLD; FMUL ST1` for all three (no memory operand).
    let part = part.map(|p| x87_store32(x87_mul(x87_load32(p), w)));
    let sum_of = read_words(e, direction);
    write_words(
        e,
        direction,
        [
            add32(part[0], sum_of[0]),
            add32(sum_of[1], part[1]),
            add32(part[2], sum_of[2]),
        ],
    );
    let sun_colour = scaled_words(read_words(e, sun.wrapping_add(0xd4)), rest);
    let b = x87_load32(x87_quiet_bits(e.mem.u32(slot)));
    let light_colour = scaled_words(read_words(e, ni_light.wrapping_add(0xd4)), b);
    let mut mixed = [0u32; 3];
    for (i, m) in mixed.iter_mut().enumerate() {
        *m = add32(light_colour[i], sun_colour[i]);
    }
    let mixed = scaled_words(mixed, w);
    for (i, c) in colour.iter_mut().enumerate() {
        *c = add32(mixed[i], *c);
    }
}
