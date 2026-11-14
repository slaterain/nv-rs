//! The lighting 3.0 shader's set-ups (`Lighting30Shader`, vtable `010ba0f8`; Group C of the drawing
//! thread's plan, `measure/typed-draw/GROUP-C.md`): what the world draw reached as translations. Live
//! reads go through [`super::packet`], the constants and states they leave through [`super::own`]. Main
//! thread only (the drawing thread replays main's draw with main's control word): `f32` maths. Read in
//! place, not routed: the shader's own words, its constant entries (the pointers at `01201080` to
//! `012010e8`, made at load), the passes' contents and the exe's constants.

use super::capture;
use super::{group_c, own, packet};
use crate::anim_keys::narrow;
use crate::shader::{self, accumulator, div_d, mul_d};
use engine_core::ptr::Ptr;
use engine_core::{args, typed_call, Engine};

/// Bits of 1.0.
const ONE: u32 = 0x3f80_0000;

/// Whether the constant entry whose pointer is at `slot` is switched on (its byte +8; the draw's own
/// switches, read back).
fn entry_on(e: &Engine, slot: u32) -> bool {
    let entry = capture::own_at::<u32>(e, slot);
    own::entry_is_enabled(e, entry)
}

// Typed from 00bbe650
/// The lighting 3.0 shader's slot 0x78, `thiscall(shader, state)`: but in the pass types 0x1e0, 0x1e1,
/// 0xe and 0xf, its own slots 0x13c and 0x140 with the pass type and the state's shader property, then
/// its slot 0x10c.
pub fn setup_textures(e: &mut Engine, this: Ptr, state: Ptr) {
    let base = this.addr();
    let property = packet::state_property(e, state.addr(), 3);
    let pass_type = packet::rendering_pass_type(e);
    if matches!(pass_type, 0x1e0 | 0x1e1 | 0xe | 0xf) {
        return;
    }
    e.vcall(base, 0x13c, &args![pass_type, property]);
    e.vcall(base, 0x140, &args![pass_type, property]);
    e.vcall(base, 0x10c, &[]);
}

/// The emittance constant (`01201b50`, four words) and its entry (`012010cc`).
const EMITTANCE: u32 = 0x0120_1b50;
const EMITTANCE_ENTRY: u32 = 0x0120_10cc;

// Typed from 00bbbda0
/// The emittance, `thiscall(shader, property)` (the shader not read): with its entry on, the property's
/// emittance (+0x84 to +0x90).
pub fn emittance(e: &mut Engine, _this: Ptr, property: Ptr) {
    if !entry_on(e, EMITTANCE_ENTRY) {
        return;
    }
    for k in 0..4u32 {
        let word = packet::lighting_word(e, property.addr(), 0x84 + 4 * k);
        own::set_constant_word(e, EMITTANCE + 4 * k, word);
    }
}

/// The glow constants (`01201b90`, two floats), their entry (`012010d8`) and the exe's float for a
/// property with bit 0x10 of +0x24 (`010177e8`).
const GLOW: u32 = 0x0120_1b90;
const GLOW_ENTRY: u32 = 0x0120_10d8;
const GLOW_SCALE: u32 = 0x0101_77e8;

// Typed from 00bbbdf0
/// The glow, `thiscall(shader, property)` (the shader not read): with its entry on, the property's glow
/// (+0xe0) and the exe's scale with the property's bit 0x10 of +0x24, else 1.0.
pub fn glow(e: &mut Engine, _this: Ptr, property: Ptr) {
    if !entry_on(e, GLOW_ENTRY) {
        return;
    }
    let property = property.addr();
    let value = packet::lighting_word(e, property, 0xe0);
    own::set_constant_word(e, GLOW, value);
    let scale = if packet::property_flags_24(e, property) & 0x10 != 0 {
        e.global::<u32>(GLOW_SCALE)
    } else {
        ONE
    };
    own::set_constant_word(e, GLOW + 4, scale);
}

/// The alpha constants (`01201be8`, two floats) and their entry (`012010e0`).
const ALPHA: u32 = 0x0120_1be8;
const ALPHA_ENTRY: u32 = 0x0120_10e0;

// Typed from 00bbbe30
/// The alpha, `stdcall(property)`: with its entry on, 1.0 or 0.0 as the property's slot 0xf4 answers
/// for (5, 0), then the property's alpha (+0x30).
pub fn alpha(e: &mut Engine, property: Ptr) {
    if !entry_on(e, ALPHA_ENTRY) {
        return;
    }
    let property = property.addr();
    let answer = e.vcall(property, 0xf4, &args![5u32, 0u32]).u32();
    own::set_constant_word(e, ALPHA + 4, if answer != 0 { ONE } else { 0 });
    let value = packet::lighting_word(e, property, 0x30);
    own::set_constant_word(e, ALPHA, value);
}

/// The eye position constant (`01201360`, three floats) and its entry (`012010b0`).
const EYE: u32 = 0x0120_1360;
const EYE_ENTRY: u32 = 0x0120_10b0;

// Typed from 00bbbcf0
/// The eye position, `thiscall(shader, position)` (the shader not read): with its entry on, the position
/// less the origin, less the scene camera's position (each difference a float).
pub fn eye_position(e: &mut Engine, _this: Ptr, position: Ptr) {
    if !entry_on(e, EYE_ENTRY) {
        return;
    }
    for k in 0..3u32 {
        // The bound as handed (the packet's image of it while the packet is open: `packet::with_drawn`).
        let value = f32::from_bits(packet::handed_word(e, position.addr(), 4 * k));
        let first = value - f32::from_bits(packet::origin_word(e, k));
        let second = first - f32::from_bits(packet::scene_camera_position(e, k));
        own::set_constant_word(e, EYE + 4 * k, second.to_bits());
    }
}

/// The world matrix constant (`012012f0`, sixteen floats, then the eight shadow words at `01201330`)
/// and its entry (`012010b4`).
const WORLD: u32 = 0x0120_12f0;
const WORLD_ENTRY: u32 = 0x0120_10b4;

// Typed from 00bbba00
/// The world matrix, `thiscall(shader, flag, transform)` (the shader not read): with its entry on,
/// without the flag the transform's rotation rows with its translation plus the scene camera's position
/// in the fourth column and (0, 0, 0, its scale) as the last row; with the flag the identity with the
/// origin plus the camera's position. Then the eight shadow words.
pub fn world_matrix(e: &mut Engine, _this: Ptr, flag: u32, transform: Ptr) {
    if !entry_on(e, WORLD_ENTRY) {
        return;
    }
    let position = [0u32, 1, 2].map(|k| f32::from_bits(packet::scene_camera_position(e, k)));
    let mut matrix = [0u32; 16];
    let base = transform.addr();
    if flag == 0 {
        for row in 0..3u32 {
            for column in 0..3u32 {
                let word = packet::transform_word(e, base, 0xc * row + 4 * column);
                matrix[(4 * row + column) as usize] = word;
            }
            let translation = f32::from_bits(packet::transform_word(e, base, 0x24 + 4 * row));
            matrix[(4 * row + 3) as usize] = (translation + position[row as usize]).to_bits();
        }
        matrix[15] = packet::transform_word(e, base, 0x30);
    } else {
        for row in 0..3u32 {
            matrix[(5 * row) as usize] = ONE;
            let origin = f32::from_bits(packet::origin_word(e, row));
            matrix[(4 * row + 3) as usize] = (origin + position[row as usize]).to_bits();
        }
        matrix[15] = ONE;
    }
    for (k, word) in (0u32..).zip(matrix) {
        own::set_constant_word(e, WORLD + 4 * k, word);
    }
    for k in 0..8u32 {
        let word = packet::shadow_extra_word(e, k);
        own::set_constant_word(e, WORLD + 0x40 + 4 * k, word);
    }
}

/// The light position constants (`01201370` with 1.0, `01201b40` with the object's +0x30) and their
/// entries (`012010a4`, `012010c8`).
const LIGHT_POSITION: u32 = 0x0120_1370;
const LIGHT_POSITION_2: u32 = 0x0120_1b40;
const LIGHT_POSITION_ENTRY: u32 = 0x0120_10a4;
const LIGHT_POSITION_ENTRY_2: u32 = 0x0120_10c8;

// Typed from 00bbb890
/// The light position, `thiscall(shader, matrix by value, object)` (the shader not read): with either
/// entry on, in the exe's frame (0x64 bytes: the sum at +0, its copy +0x10, the matrix +0x20, the
/// object's word +0x60) the sun's vector plus the scene camera's position (each sum a float) is
/// transformed by the matrix as a point (`D3DXVec3TransformCoord`) into `01201370` with 1.0 and into
/// `01201b40` with the object's +0x30.
pub fn light_position(e: &mut Engine, _this: Ptr, matrix: [u32; 16], object: Ptr) {
    if !entry_on(e, LIGHT_POSITION_ENTRY) && !entry_on(e, LIGHT_POSITION_ENTRY_2) {
        return;
    }
    e.with_stack(0x64, |e, frame| {
        let frame = frame.addr();
        for (k, word) in (0u32..).zip(matrix) {
            e.set_global(frame + 0x20 + 4 * k, word);
        }
        e.set_global(frame + 0x60, object.addr());
        let sun = [0u32, 1, 2].map(|k| f32::from_bits(packet::sun_word(e, k)));
        let position = [0u32, 1, 2].map(|k| f32::from_bits(packet::scene_camera_position(e, k)));
        for (at, (s, p)) in (0u32..).step_by(4).zip(sun.into_iter().zip(position)) {
            let sum = (s + p).to_bits();
            e.set_global(frame + at, sum);
            e.set_global(frame + 0x10 + at, sum);
        }
        super::thunks::call_vec3_transform_coord(e, frame, frame + 0x10, frame + 0x20);
        let out = [0u32, 1, 2].map(|k| e.global::<u32>(frame + 4 * k));
        for (k, word) in (0u32..).zip(out) {
            own::set_constant_word(e, LIGHT_POSITION + 4 * k, word);
        }
        own::set_constant_word(e, LIGHT_POSITION + 0xc, ONE);
        for (k, word) in (0u32..).zip(out) {
            own::set_constant_word(e, LIGHT_POSITION_2 + 4 * k, word);
        }
        let object = e.global::<u32>(frame + 0x60);
        let value = packet::transform_word(e, object, 0x30);
        own::set_constant_word(e, LIGHT_POSITION_2 + 0xc, value);
    });
}

/// The fog constants (`01201bc0`: the fog plane and its distance, then the far, range and two
/// switches), their entry (`012010e4`) and the exe's -1.0 (`01012054`).
const FOG: u32 = 0x0120_1bc0;
const FOG_ENTRY: u32 = 0x0120_10e4;
const MINUS_ONE: u32 = 0x0101_2054;

// Typed from 00bbbe80
/// The fog, `thiscall(shader)` (the shader not read): with its entry on, the fog plane, its dot
/// product with the sun's vector plus the scene camera's position (each sum a float; the products
/// summed in the exe's order) less the fog's offset; then, when the far and near distances are both 0.0
/// (a NaN is not), 0.0, 0.0, the exe's -1.0 and 0.0, else the far distance, far less near, 1.0, 1.0.
pub fn fog(e: &mut Engine, _this: Ptr) {
    if !entry_on(e, FOG_ENTRY) {
        return;
    }
    let sun = [0u32, 1, 2].map(|k| f32::from_bits(packet::sun_word(e, k)));
    let position = [0u32, 1, 2].map(|k| f32::from_bits(packet::scene_camera_position(e, k)));
    let plane = [0u32, 1, 2].map(|k| packet::fog_plane_word(e, k));
    let offset = f32::from_bits(packet::fog_plane_offset(e));
    let sums = [0usize, 1, 2].map(|k| position[k] + sun[k]);
    for (k, word) in (0u32..).zip(plane) {
        own::set_constant_word(e, FOG + 4 * k, word);
    }
    let p = plane.map(f32::from_bits);
    let dot = (p[0] * sums[0] + p[1] * sums[1]) + p[2] * sums[2];
    own::set_constant_word(e, FOG + 0xc, (dot - offset).to_bits());
    let far = packet::fog_far(e);
    let near = packet::fog_near(e);
    let (far_value, near_value) = (f32::from_bits(far), f32::from_bits(near));
    let words = if far_value == 0.0 && near_value == 0.0 {
        [0, 0, e.global::<u32>(MINUS_ONE), 0]
    } else {
        [far, (far_value - near_value).to_bits(), ONE, ONE]
    };
    for (k, word) in (0u32..).zip(words) {
        own::set_constant_word(e, FOG + 0x10 + 4 * k, word);
    }
}

/// The material constants (`01201b60`: the alpha, the fade, the two field switches; set first from
/// the defaults `011a9be0`), the alpha test's entry (`012010d0`), the byte the ambient fog takes
/// (`01200bf5`) and the exe's 255.0 (`double`, `0101e568`).
const MATERIAL: u32 = 0x0120_1b60;
const ALPHA_TEST_ENTRY: u32 = 0x0120_10d0;
const AMBIENT_FOG: u32 = 0x0120_0bf5;
const BYTE_SCALE: u32 = 0x0101_e568;
/// What the light-scaled alpha calls by address: the property's alpha test (`004a2020`, `thiscall`),
/// `BSShaderLightingProperty::GetFirstActiveNonShadowLight` (`00b70600`), a vector's length (`00b719e0`,
/// `thiscall`, in `ST0`), the transform `00439ef0`, a point times a scalar `00633b40`, the length `00457990` (`thiscall`, in
/// `ST0`) and the clamps `0068b050` (a `double`, a float) and `00b719c0` (two `double`s), each in `ST0`.
const ALPHA_TEST: u32 = 0x004a_2020;
const FIRST_ACTIVE_LIGHT: u32 = 0x00b7_0600;
const VECTOR_LENGTH: u32 = 0x00b7_19e0;
const TRANSFORM_POINT: u32 = 0x0043_9ef0;
const POINT_TIMES_SCALAR: u32 = 0x0063_3b40;
const LENGTH: u32 = 0x0045_7990;
const CLAMP_LOW: u32 = 0x0068_b050;
const CLAMP_HIGH: u32 = 0x00b7_19c0;

/// The alpha scaled by the light (the material constants' alpha in the pass types 0x1bd to 0x1c3 outside
/// mode 10): the property's alpha times its slot 0x110's factor (a float), or 1.0 when not below 1.0;
/// with the property's bit 0x8000 of +0x24 and its alpha test (`004a2020` with 0x11) failing, in a
/// 0x1c-byte frame the larger of the scene light's colour's length and, with a first active light, the
/// length of the light's colour scaled by its fade (the distance to the light over its radius clamped to
/// 0 and 1, less from 1), times the alpha. A step with a callee's `double` (`ST0`) in `f64`, rounded once.
fn light_alpha(e: &mut Engine, fallback: u32, property: u32) {
    // The property's distance fade (slot 0x110: the exe's 00b66c40 typed, reading the packet).
    let factor = narrow(super::group_c::property_distance_fade(e, property));
    let alpha = f32::from_bits(packet::property_alpha(e, property)) * factor;
    let value = if alpha < 1.0 { alpha.to_bits() } else { ONE };
    own::set_constant_word(e, MATERIAL, value);
    if packet::property_flags_24(e, property) & 0x8000 == 0 {
        return;
    }
    #[allow(clippy::cast_possible_truncation)]
    let failed = e.call(ALPHA_TEST, &args![property, 0x11u32]).u32() as u8 != 0;
    if failed {
        return;
    }
    e.with_stack(0x1c, |e, frame| {
        let frame = frame.addr();
        let vector = frame;
        let other = frame + 0xc;
        let out = frame + 0x18;
        let mut best = 0f32;
        e.set_global(out, 0u32);
        let light = e.call(FIRST_ACTIVE_LIGHT, &args![property, out]).u32();
        for k in 0..3u32 {
            let word = packet::sun_light_colour_word(e, k);
            e.set_global(vector + 4 * k, word);
        }
        let length = narrow(e.call(VECTOR_LENGTH, &args![vector]).f64());
        if light != 0 {
            // The property list's light (the cut copies it, its light and their values): its light (+0xf8), that
            // light's position (+0x8c) and colour (+0xd4) handed by pointer, strength +0xc4 and radius +0xe0, all
            // the packet's.
            let data = packet::pass_light_word(e, light, 0xf8);
            capture::image(e, data, 0x8c, 0x98, |e, base| {
                e.call(TRANSFORM_POINT, &args![base + 0x8c, vector, fallback]);
            });
            let radius = packet::light_value_word(e, data, 0xc4);
            capture::image(e, data, 0xd4, 0xe0, |e, base| {
                e.call(POINT_TIMES_SCALAR, &args![base + 0xd4, other, radius]);
            });
            let distance = e.call(LENGTH, &args![vector]).f64();
            let ratio = narrow(
                distance / f64::from(f32::from_bits(packet::light_value_word(e, data, 0xe0))),
            );
            let low = e.call(CLAMP_LOW, &args![0.0f64, ratio.to_bits()]).f64();
            let clamped = narrow(e.call(CLAMP_HIGH, &args![1.0f64, low]).f64());
            let rest = 1.0 - clamped;
            let other_length = e.call(VECTOR_LENGTH, &args![other]).f64();
            best = narrow(other_length * f64::from(rest));
        }
        let chosen = if length < best { best } else { length };
        let alpha = f32::from_bits(own::constant_word(e, MATERIAL));
        own::set_constant_word(e, MATERIAL, (chosen * alpha).to_bits());
    });
}

// Typed from 00bbe9c0
/// The material constants, `thiscall(shader, fallback, property, state, pass type)` (the shader not
/// read): reset from the defaults; in shader mode 8 the alpha 0.0 with the property's bit 0x20000 of
/// +0x24, else 1.0, and the fade 0.0 when the state's alpha property has its flag 0 or 9, else 1.0, and
/// done. Otherwise the alpha: scaled by the light in the pass types 0x1bd to 0x1c3 outside mode 10
/// ([`light_alpha`]), else the property's alpha when below 1.0, else 1.0; the fade the alpha property's
/// reference over 255 with its flag 9, else 0.0; the switches 1.0 for its flags' field 0 or 2; the ambient
/// fog's byte whether the alpha is below 1.0, the alpha test's entry on unless that byte was and stays
/// clear.
pub fn material(
    e: &mut Engine,
    _this: Ptr,
    fallback: u32,
    property: Ptr,
    state: Ptr,
    pass_type: u32,
) {
    let property = property.addr();
    let block = state.addr();
    for k in 0..4u32 {
        let word = packet::material_default_word(e, k);
        own::set_constant_word(e, MATERIAL + 4 * k, word);
    }
    let mode = packet::shader_mode(e);
    if mode == 8 {
        let alpha = if packet::property_flags_24(e, property) & 0x2_0000 != 0 {
            0
        } else {
            ONE
        };
        own::set_constant_word(e, MATERIAL, alpha);
        let flags = packet::state_property(e, block, 0);
        let off = flags != 0 && {
            let bits = packet::ni_property_flags(e, flags);
            bits & 1 != 0 || (bits >> 9) & 1 != 0
        };
        own::set_constant_word(e, MATERIAL + 4, if off { 0 } else { ONE });
        return;
    }
    if mode != 10 && pass_type.wrapping_sub(0x1bd) <= 6 {
        light_alpha(e, fallback, property);
    } else {
        let alpha = f32::from_bits(packet::property_alpha(e, property));
        let value = if 1.0 > alpha { alpha.to_bits() } else { ONE };
        own::set_constant_word(e, MATERIAL, value);
    }
    let flags = packet::state_property(e, block, 0);
    let bits = packet::ni_property_flags(e, flags);
    let fade = if (bits >> 9) & 1 != 0 {
        let reference = f32::from(packet::alpha_reference(e, flags));
        div_d(reference, e.global(BYTE_SCALE)).to_bits()
    } else {
        0
    };
    own::set_constant_word(e, MATERIAL + 4, fade);
    own::set_constant_word(e, MATERIAL + 8, 0);
    own::set_constant_word(e, MATERIAL + 0xc, 0);
    match (u32::from(packet::ni_property_flags(e, flags)) >> 5) & 0xf {
        0 => own::set_constant_word(e, MATERIAL + 8, ONE),
        2 => own::set_constant_word(e, MATERIAL + 0xc, ONE),
        _ => {}
    }
    let old = own::constant_byte(e, AMBIENT_FOG);
    let alpha = f32::from_bits(own::constant_word(e, MATERIAL));
    let below = u8::from(1.0 > alpha);
    own::set_constant_byte(e, AMBIENT_FOG, below);
    let entry = capture::own_at::<u32>(e, ALPHA_TEST_ENTRY);
    own::set_entry_enabled(e, entry, !(old == 0 && below == 0));
}

/// The ambient constant (`01201b30`, four floats), its entry (`012010c4`) and the byte it sets
/// (`01200bf4`); the diffuse constant (`01201b70`, four floats).
const AMBIENT: u32 = 0x0120_1b30;
const AMBIENT_ENTRY: u32 = 0x0120_10c4;
const AMBIENT_SET: u32 = 0x0120_0bf4;
const DIFFUSE: u32 = 0x0120_1b70;

// Typed from 00bbecd0
/// The colour, `thiscall(shader, material, property)` (the shader not read): the material's colour (its
/// colour pointer +0x34, or its own +0x28); nothing more when it equals the neutral colour and the
/// property has no lights (its light list +0xb4 empty). With HDR on it is scaled by the image space's
/// HDR scale times the material's (a float), each component a float. Without lights it is added to the
/// ambient constant (each sum a float, held to 1.0 without HDR; the fourth word kept), whose entry and
/// byte are set; with lights it is the diffuse constant (held to 1.0 without HDR) with 0.0.
pub fn colour(e: &mut Engine, _this: Ptr, material: Ptr, property: Ptr) {
    let material = material.addr();
    let mut source = packet::material_colour(e, material);
    if source == 0 {
        source = material.wrapping_add(0x28);
    }
    let mut colour = [0u32, 1, 2].map(|k| f32::from_bits(packet::colour_word(e, source, k)));
    let neutral = [0u32, 1, 2].map(|k| f32::from_bits(packet::neutral_colour_word(e, k)));
    let lights = packet::lighting_has_lights(e, property.addr());
    if neutral == colour && !lights {
        return;
    }
    let scaled = packet::hdr_on(e) != 0;
    if scaled {
        let factor = f32::from_bits(packet::hdr_effect_scale(e))
            * f32::from_bits(packet::material_scale(e, material));
        colour = colour.map(|c| c * factor);
    }
    let held = |value: f32| if 1.0 > value { value } else { 1.0 };
    if !lights {
        let ambient = [0u32, 1, 2, 3].map(|k| own::constant_word(e, AMBIENT + 4 * k));
        let mut sums = [0usize, 1, 2].map(|k| colour[k] + f32::from_bits(ambient[k]));
        if !scaled {
            sums = sums.map(held);
        }
        for (k, value) in (0u32..).zip(sums) {
            own::set_constant_word(e, AMBIENT + 4 * k, value.to_bits());
        }
        own::set_constant_word(e, AMBIENT + 0xc, ambient[3]);
        let entry = capture::own_at::<u32>(e, AMBIENT_ENTRY);
        own::set_entry_enabled(e, entry, true);
        own::set_constant_byte(e, AMBIENT_SET, 1);
        return;
    }
    if !scaled {
        colour = colour.map(held);
    }
    for (k, value) in (0u32..).zip(colour) {
        own::set_constant_word(e, DIFFUSE + 4 * k, value.to_bits());
    }
    own::set_constant_word(e, DIFFUSE + 0xc, 0);
}

/// The view constants (`01201170`, sixteen floats) and their entry (`01201098`); the projection
/// constants (`012012b0`) and their entry (`012010b8`); `GetD3DFromNi` (`00b799a0`, by address).
const VIEW: u32 = 0x0120_1170;
const VIEW_ENTRY: u32 = 0x0120_1098;
const PROJECTION: u32 = 0x0120_12b0;
const PROJECTION_ENTRY: u32 = 0x0120_10b8;
const D3D_FROM_NI: u32 = 0x00b7_99a0;

/// The identity matrix's words.
const IDENTITY: [u32; 16] = [ONE, 0, 0, 0, 0, ONE, 0, 0, 0, 0, ONE, 0, 0, 0, 0, ONE];

// Typed from 00bbe6b0
/// The view matrix, `thiscall(shader, rendered pass, pass type, flag, transform)`, in a 0xd0-byte frame
/// laid out as the exe's (the matrix +0x10, a local +0x50, the result +0x90). With its entry on: without
/// a light list, with the pass's light-selected byte (+9) and its first light's byte +0xed, or in the pass
/// types 0x1bd to 0x1c3 and 0x2d8 to 0x2ef, the renderer's matrix (the shader's +0x18, +0x940) transposed
/// (`D3DXMatrixTranspose`) into the view constants; otherwise, with the selected byte, the identity (or the
/// transform's D3D form without the flag, `GetD3DFromNi`) with the origin added to its translation, times
/// the first light's matrix (+0x10; the pass types 0x1c4 to 0x1c7) or the second's (`D3DXMatrixMultiply`),
/// transposed and copied (`_memcpy`). With the projection's entry on and the flag, the identity copied to
/// the projection constants.
pub fn view_matrix(
    e: &mut Engine,
    this: Ptr,
    pass: Ptr,
    pass_type: u32,
    flag: u32,
    transform: Ptr,
) {
    e.with_stack(0xd0, |e, frame| {
        let frame = frame.addr();
        let matrix = frame + 0x10;
        let local = frame + 0x50;
        let result = frame + 0x90;
        if entry_on(e, VIEW_ENTRY) {
            let pass = pass.addr();
            // The pass's lights, the first one's byte +0xed and the chosen one's matrix (+0x10) through the
            // packet (value packet B).
            let lights = packet::rendering_pass_lights(e, pass);
            let selected = packet::rendering_pass_byte9(e, pass);
            let copy = lights == 0
                || (selected != 0
                    && packet::pass_light_byte(e, packet::pass_light_at(e, lights, 0), 0xed) != 0)
                || (0x1bd..=0x1c3).contains(&pass_type)
                || (0x2d8..=0x2ef).contains(&pass_type);
            if copy {
                let renderer = e.global::<u32>(this.addr().wrapping_add(0x18));
                for k in 0..16u32 {
                    let word = packet::renderer_view_word(e, renderer, 0x940 + 4 * k);
                    e.set_global(matrix + 4 * k, word);
                }
                super::thunks::call_matrix_transpose(e, matrix, matrix);
                for k in 0..16u32 {
                    let word = e.global::<u32>(matrix + 4 * k);
                    own::set_constant_word(e, VIEW + 4 * k, word);
                }
            } else if selected != 0 {
                for (k, word) in (0u32..).zip(IDENTITY) {
                    e.set_global(local + 4 * k, word);
                    e.set_global(matrix + 4 * k, word);
                }
                if flag == 0 {
                    e.call(D3D_FROM_NI, &args![local, transform.addr()]);
                }
                for k in 0..3u32 {
                    let origin = f32::from_bits(packet::origin_word(e, k));
                    let word = f32::from_bits(e.global::<u32>(local + 0x30 + 4 * k));
                    e.set_global(local + 0x30 + 4 * k, (origin + word).to_bits());
                }
                let light = if pass_type.wrapping_sub(0x1c4) <= 3 {
                    packet::pass_light_at(e, lights, 0)
                } else {
                    packet::pass_light_at(e, lights, 1)
                };
                super::capture::image(e, light, 0x10, 0x50, |e, image| {
                    super::thunks::call_matrix_multiply(e, result, local, image.wrapping_add(0x10));
                });
                for k in 0..16u32 {
                    let word = e.global::<u32>(result + 4 * k);
                    e.set_global(matrix + 4 * k, word);
                }
                super::thunks::call_matrix_transpose(e, matrix, matrix);
                own::copy_constants(e, VIEW, matrix, 0x40);
            }
        }
        if entry_on(e, PROJECTION_ENTRY) && flag != 0 {
            for (k, word) in (0u32..).zip(IDENTITY) {
                e.set_global(matrix + 4 * k, word);
            }
            own::copy_constants(e, PROJECTION, matrix, 0x40);
        }
    });
}

// Typed from 00bbc050
/// The lights, `thiscall(shader, property, rendered pass, pass type, material, matrix, flag,
/// transform)` (the shader not read): the property's light value (+0x6c) taken; in shader mode 0xf the
/// ambient constant the held accumulator's four words (+0x154), else the lights' set-up
/// ([`light_set_up`]).
#[allow(clippy::too_many_arguments)]
pub fn lights(
    e: &mut Engine,
    _this: Ptr,
    property: Ptr,
    pass: Ptr,
    pass_type: u32,
    material: Ptr,
    matrix: u32,
    flag: u32,
    transform: Ptr,
) {
    let value = packet::lighting_word(e, property.addr(), 0x6c);
    if packet::shader_mode(e) == 0xf {
        let held = packet::held_accumulator(e);
        for k in 0..4u32 {
            let word = packet::accumulator_ambient_word(e, held, k);
            own::set_constant_word(e, AMBIENT + 4 * k, word);
        }
        return;
    }
    light_set_up(
        e,
        property.addr(),
        value,
        pass.addr(),
        pass_type,
        material.addr(),
        matrix,
        flag,
        transform.addr(),
    );
}

/// The light setters (`00b70de0` for the first light, `00b71130` for the others; `cdecl(index, light,
/// value)`, this family's), the light colour (`011fa0d0`, four floats), the light data (`01201c70`: the
/// directional and point counts, the material's word, the fourth; then the 0x20-byte entries from
/// `01201c90`) and its entry (`012010e8`, its count word +0x20), the exe's limit on the light count
/// (`011ae60c`) and 0.5 (`01016248`), `GetNumberofActiveLights` (`00b70790`, the draw family's).
const LIGHT_COLOUR_ROW: u32 = 0x011f_a0d0;
const LIGHT_DATA: u32 = 0x0120_1c70;
const LIGHT_ENTRIES: u32 = 0x0120_1c90;
const LIGHT_DATA_ENTRY: u32 = 0x0120_10e8;
const LIGHT_LIMIT: u32 = 0x011a_e60c;
const HALF: u32 = 0x0101_6248;

/// The setter of the other lights (`00b71130`), `value` a float's bits.
fn other_light(e: &mut Engine, index: u32, light: u32, value: u32) {
    typed_call!(
        super::FAMILY,
        0x00b7_1130,
        super::group_c::other_light(e, index, Ptr::new(light), f32::from_bits(value))
    );
}

/// The lights' set-up after the mode test (the exe's `00bbc0a4`, entered by `00bbc050`'s jump only):
/// the pass's lights to the setters; the light colour scaled by the light scale (HDR and the property's
/// bit 0x400 of +0x20) or the fog colour under full bright; the light data's header by the pass type; then
/// the directional entries' negated directions through the matrix as normals, normalised twice, and the
/// point entries' positions plus the scene camera's through it as points, their fourth word over the
/// transform's scale without the flag (or the position times it in the pass types 0x1c4 and 0x1c5). A
/// 0x54-byte frame as the exe's (the input +0, the unit +0xc, the output +0x24).
#[allow(clippy::too_many_arguments)]
fn light_set_up(
    e: &mut Engine,
    property: u32,
    value: u32,
    pass: u32,
    pass_type: u32,
    material: u32,
    matrix: u32,
    flag: u32,
    transform: u32,
) {
    let kind = packet::pass_type(e, pass);
    let count = u32::from(packet::rendering_pass_byte9(e, pass));
    if (0x1c4..=0x1cb).contains(&kind) {
        let light = packet::rendering_light(e, pass, 0);
        other_light(e, 0, light, value);
    } else {
        if count > 0 {
            let light = packet::rendering_light(e, pass, 0);
            typed_call!(
                super::FAMILY,
                0x00b7_0de0,
                super::group_c::direction_light(e, 0, Ptr::new(light), f32::from_bits(value))
            );
        }
        let mut index = 1u32;
        while index < count {
            let light = packet::rendering_light(e, pass, index);
            other_light(e, index, light, value);
            index += 1;
        }
    }
    if packet::hdr_on(e) != 0 && packet::property_flags(e, property) & 0x400 != 0 {
        let colour = [0u32, 1, 2, 3]
            .map(|k| f32::from_bits(own::constant_word(e, LIGHT_COLOUR_ROW + 4 * k)));
        let factor = f32::from_bits(packet::light_scale(e));
        let scaled = [
            colour[0] * factor,
            colour[1] * factor,
            colour[2] * factor,
            factor * colour[3],
        ];
        for (k, c) in (0u32..).zip(scaled) {
            own::set_constant_word(e, LIGHT_COLOUR_ROW + 4 * k, c.to_bits());
        }
    }
    if packet::full_bright(e) != 0 {
        for k in 0..4u32 {
            let word = packet::fog_colour_word(e, k);
            own::set_constant_word(e, LIGHT_COLOUR_ROW + 4 * k, word);
        }
    }
    let entry = capture::own_at::<u32>(e, LIGHT_DATA_ENTRY);
    if pass_type < 0x1bd {
        // The lights less one, its low half (the exe's 16-bit register): the count held to the
        // exe's limit (signed).
        let limited = if count != 0 {
            let limit = e.global::<i32>(LIGHT_LIMIT);
            let lights = i32::try_from(count).unwrap_or(i32::MAX);
            let low = lights.min(limit).wrapping_sub(1).to_le_bytes();
            u16::from_le_bytes([low[0], low[1]])
        } else {
            0
        };
        own::set_constant_word(e, LIGHT_DATA, ONE);
        own::set_constant_word(e, LIGHT_DATA + 4, f32::from(limited).to_bits());
        let extra = packet::material_word(e, material, 0x38);
        own::set_constant_word(e, LIGHT_DATA + 8, extra);
        let fourth = if packet::shadows_skipped(e) != 0 {
            0
        } else {
            ONE
        };
        own::set_constant_word(e, LIGHT_DATA + 0xc, fourth);
        own::set_entry_word(e, entry, 0x20, u32::from(limited) * 2 + 3);
    } else if pass_type == 0x1c3 {
        let active: u32 = typed_call!(
            super::FAMILY,
            0x00b7_0790,
            super::lights::active_light_count(e, Ptr::new(property))
        );
        // The exe tests the count's low 16 bits.
        let active = active & 0xffff;
        if active == 0 {
            if packet::shader_mode(e) == 0xa {
                let half = e.global::<u32>(HALF);
                for k in 0..3u32 {
                    own::set_constant_word(e, LIGHT_DATA + 0x10 + 4 * k, half);
                }
                own::set_constant_word(e, LIGHT_DATA + 0x1c, ONE);
            } else {
                for k in 0..4u32 {
                    let word = own::constant_word(e, AMBIENT + 4 * k);
                    own::set_constant_word(e, LIGHT_DATA + 0x10 + 4 * k, word);
                }
            }
        }
        let entry = capture::own_at::<u32>(e, LIGHT_DATA_ENTRY);
        own::set_entry_word(e, entry, 0x20, 2);
    } else {
        own::set_entry_word(e, entry, 0x20, 3);
        own::set_constant_word(e, LIGHT_DATA, 0);
        let second = if pass_type.wrapping_sub(0x1c4) <= 3 {
            ONE
        } else {
            0
        };
        own::set_constant_word(e, LIGHT_DATA + 4, second);
    }
    let directional = ftol(e, own::constant_word(e, LIGHT_DATA));
    let points = ftol(e, own::constant_word(e, LIGHT_DATA + 4));
    let mut done = 0u32;
    e.with_stack(0x54, |e, frame| {
        let frame = frame.addr();
        let input = frame;
        let output = frame + 0x24;
        let unit = frame + 0xc;
        #[allow(clippy::cast_possible_wrap)]
        let some_directional = (directional as i32) > 0;
        if !(0x1c4..=0x1c7).contains(&pass_type) && some_directional {
            done = directional;
            for index in 0..directional {
                let entry = LIGHT_ENTRIES + 0x20 * index; // own: an address, used through the accessors
                for k in 0..3u32 {
                    let value = f32::from_bits(own::constant_word(e, entry + 4 * k));
                    e.set_global(input + 4 * k, (-value).to_bits());
                }
                super::thunks::call_vec3_transform_normal(e, output, input, matrix);
                super::thunks::call_vec3_normalize(e, input, output);
                for k in 0..3u32 {
                    let word = e.global::<u32>(input + 4 * k);
                    e.set_global(unit + 4 * k, word);
                }
                typed_call!(
                    crate::dispatch::FAMILY,
                    0x004a_0c10,
                    crate::dispatch::math::normalize(e, Ptr::new(unit))
                );
                for k in 0..3u32 {
                    let word = e.global::<u32>(unit + 4 * k);
                    own::set_constant_word(e, entry + 4 * k, word);
                }
            }
        }
        #[allow(clippy::cast_possible_wrap)]
        let some_points = (points as i32) > 0;
        if some_points {
            for index in 0..points {
                let entry = LIGHT_ENTRIES + 0x20 * (done + index); // own: an address, used through the accessors
                let words = [0u32, 1, 2, 3].map(|k| own::constant_word(e, entry + 4 * k));
                let position =
                    [0u32, 1, 2].map(|k| f32::from_bits(packet::scene_camera_position(e, k)));
                for (k, (word, p)) in (0u32..).zip(words.into_iter().zip(position)) {
                    let sum = f32::from_bits(word) + p;
                    e.set_global(unit + 4 * k, sum.to_bits());
                }
                super::thunks::call_vec3_transform_coord(e, output, unit, matrix);
                let mut out = [0u32, 1, 2].map(|k| f32::from_bits(e.global::<u32>(output + 4 * k)));
                let mut fourth = f32::from_bits(words[3]);
                if flag == 0 {
                    let scale = f32::from_bits(packet::transform_word(e, transform, 0x30));
                    if pass_type == 0x1c4 || pass_type == 0x1c5 {
                        out = [out[0] * scale, out[1] * scale, scale * out[2]];
                    } else {
                        fourth /= scale;
                    }
                }
                for (k, value) in (0u32..).zip(out) {
                    own::set_constant_word(e, entry + 4 * k, value.to_bits());
                }
                own::set_constant_word(e, entry + 0xc, fourth.to_bits());
            }
        }
    });
}

/// `_ftol2` of a float's word (the CRT family's: the value truncated toward zero), its low word.
#[allow(clippy::cast_possible_truncation)]
fn ftol(e: &mut Engine, word: u32) -> u32 {
    let value: u64 = typed_call!(
        crate::crt::FAMILY,
        0x00ec_62c0,
        crate::crt::math::ftol2_sse(e, f64::from(f32::from_bits(word)))
    );
    value as u32
}

/// What the geometry set-up calls by address: the model matrix (`00c4c2d0`, the dispatch family's), the
/// special set-ups of the pass types 0xe and 0xf (`00bbdda0`) and 0x1e0 and 0x1e1 (`00bbdcd0`,
/// `00bbde20`); the camera's position (`00b71b40`) and the light's plane store (`00b71ad0`) are this family's.
const SET_UP_DDA0: u32 = 0x00bb_dda0;
const SET_UP_DCD0: u32 = 0x00bb_dcd0;
const SET_UP_DE20: u32 = 0x00bb_de20;
/// The refraction constant (`01201350`) and its entry (`012010ac`); the parallax constants (`01201ba0`,
/// two floats) and their entry (`012010dc`); the exe's 3600.0 and the cycle (`double`s, `01012640`,
/// `0104eea0`).
const REFRACTION: u32 = 0x0120_1350;
const REFRACTION_ENTRY: u32 = 0x0120_10ac;
const PARALLAX: u32 = 0x0120_1ba0;
const PARALLAX_ENTRY: u32 = 0x0120_10dc;
const SECONDS_PER_HOUR: u32 = 0x0101_2640;
const CYCLE: u32 = 0x0104_eea0;

/// A shadow-casting light's clip planes (the geometry set-up's pass types 0x1c4 to 0x1c7), in a 0x200-byte
/// frame laid out as the exe's: the renderer's light matrix (+0x980) with the camera position plus the
/// origin folded into its last row (minus its dot product with each of the first three columns,
/// `NiPoint3::Dot`, the leaf family's), times its matrix +0x9c0, inverted and transposed; the light's six
/// planes (+0x144, the distance negated) normalised and transformed by it, set on the device (its slot
/// 0xdc) and stored on the light (`00b71ad0`).
fn shadow_clip_planes(e: &mut Engine, shader: u32, light: u32) {
    e.with_stack(0x200, |e, frame| {
        let frame = frame.addr();
        let position = frame + 0x50;
        let column = frame + 0x34;
        let point = frame + 0x24;
        let light_matrix = frame + 0x60;
        let product = frame + 0x140;
        let other = frame + 0x180;
        let inverse = frame + 0x1c0;
        let planes = frame + 0xe0;
        let renderer = e.global::<u32>(shader.wrapping_add(0x18));
        for k in 0..16u32 {
            let word = packet::renderer_view_word(e, renderer, 0x980 + 4 * k);
            e.set_global(light_matrix + 4 * k, word);
        }
        let camera = packet::scene_shadow_node(e);
        typed_call!(
            super::FAMILY,
            0x00b7_1b40,
            super::lights::camera_position(e, Ptr::new(camera), Ptr::new(position))
        );
        let sums = [0u32, 1, 2].map(|k| {
            let value = f32::from_bits(e.global::<u32>(position + 4 * k));
            (value + f32::from_bits(packet::origin_word(e, k))).to_bits()
        });
        for index in 0..3u32 {
            for (row, word) in (0u32..).zip([0u32, 4, 8]) {
                let value = e.global::<u32>(light_matrix + 4 * (index + word));
                let (first, second) = if index == 0 {
                    (column, point)
                } else {
                    (point, column)
                };
                e.set_global(first + 4 * row, value);
                e.set_global(second + 4 * row, sums[row as usize]);
            }
            let (target, argument) = if index == 0 {
                (point, column)
            } else {
                (column, point)
            };
            let dot: f32 = typed_call!(
                engine_core::typed::leaf::FAMILY,
                0x004b_6190,
                engine_core::typed::leaf::point3_dot(e, target, argument)
            );
            e.set_global(light_matrix + 0x30 + 4 * index, (-dot).to_bits());
        }
        for k in 0..16u32 {
            let word = packet::renderer_view_word(e, renderer, 0x9c0 + 4 * k);
            e.set_global(other + 4 * k, word);
        }
        super::thunks::call_matrix_multiply(e, product, light_matrix, other);
        super::thunks::call_matrix_inverse(e, inverse, 0u32, product);
        super::thunks::call_matrix_transpose(e, product, inverse);
        for index in 0..6u32 {
            let source = 0x144 + 0x10 * index;
            for k in 0..3u32 {
                // The pass light's planes through the packet (value packet B).
                let word = packet::pass_light_word(e, light, source + 4 * k);
                e.set_global(point + 4 * k, word);
            }
            let distance = f32::from_bits(packet::pass_light_word(e, light, source + 0xc));
            e.set_global(point + 0xc, (-distance).to_bits());
            super::thunks::call_plane_normalize(e, point, point);
            super::thunks::call_plane_transform(e, column, point, product);
            for k in 0..4u32 {
                let word = e.global::<u32>(column + 4 * k);
                e.set_global(planes + 0x10 * index + 4 * k, word);
            }
        }
        for index in 0..6u32 {
            let device = packet::scene_renderer_device(e);
            own::device_clip_plane(e, device, index, planes + 0x10 * index);
        }
        typed_call!(
            super::FAMILY,
            0x00b7_1ad0,
            super::lights::store_clip_planes(e, Ptr::new(light), planes)
        );
    });
}

/// The light's stored planes set on the device (when the light or the shader accumulator refuses the
/// computed ones): the six planes at +0x1a8.
fn stored_clip_planes(e: &mut Engine, light: u32) {
    // The cache is the draw's own state (owned at the cut per pass light): the device is handed an image of it.
    let cache = light.wrapping_add(crate::shader::geometry::CLIP_PLANE_CACHE);
    capture::own_image(
        e,
        cache,
        crate::shader::geometry::CLIP_PLANES_SIZE,
        |e, planes| {
            for index in 0..6u32 {
                let device = packet::scene_renderer_device(e);
                own::device_clip_plane(e, device, index, planes.wrapping_add(0x10 * index));
            }
        },
    );
}

// Typed from 00bbef90
/// The lighting 3.0 shader's slot 0x7c (its geometry set-up), `thiscall(shader, state)`. With the
/// rendered pass and its geometry: in the pass types 0x1d3 to 0x1d6 with the alpha property's flag 0, z
/// writes off (locked); in a 0x40-byte frame the model matrix (`00c4c2d0`, the dispatch family's) and the
/// renderer's world transform (`00e6fbb0`); for the pass types 0x1e0 and 0x1e1 the special set-ups
/// (`00bbdcd0`, `00bbde20`) and the light position, done; otherwise (`00bbdda0` first for 0xe and 0xf)
/// the lights, the view and world matrices, the refraction (the game time in hours times the cycle times
/// the property's count +0xe4) with its entry on, the eye position, the light position, the emittance,
/// the material, the colour, the glow, the parallax (+0xe8, +0xec) with its entry on, the alpha and the
/// fog; for the pass types 0x1c4 to 0x1c7 the first light's clip planes (computed, or its stored ones
/// when it or the shader accumulator (`011f95ec`) refuses them: none when the accumulator does). Then the pass's
/// constant maps (its +0x34 and +0x48, each when the shader has the map +0x30 / +0x34 and the pass its
/// program +0x44 / +0x5c), and the shader's +0x1c object's slot 0x24 with the state's fifth property when
/// its flag 0 is set.
pub fn setup_constants(e: &mut Engine, this: Ptr, state: Ptr) {
    let geometry = packet::rendering_geometry(e);
    // The geometry's world transform as the value packet holds it (value packet B): every helper handed the
    // transform reads the packet's image of it while the packet is open, the geometry itself otherwise.
    packet::with_drawn(e, geometry, |e, transform, bound| {
        constants_with(e, this, state, geometry, transform, bound);
    });
}

/// [`setup_constants`] with the transform and the bound (`transform`: the geometry's +0x68, `bound`: its bound; or the
/// packet's images of them).
fn constants_with(
    e: &mut Engine,
    this: Ptr,
    state: Ptr,
    geometry: u32,
    transform: u32,
    bound: u32,
) {
    let base = this.addr();
    let block = state.addr();
    let pass = packet::rendering_pass(e);
    let pass_type = packet::rendering_pass_type(e);
    let buffers = packet::buffer_data(e, packet::model_data(e, geometry));
    let skin = packet::skin_instance(e, geometry);
    let property = packet::state_property(e, block, 3);
    let material_property = packet::state_property(e, block, 2);
    if pass_type.wrapping_sub(0x1d3) <= 3 {
        let flags = packet::state_property(e, block, 0);
        if packet::ni_property_flags(e, flags) & 1 != 0 {
            typed_call!(
                shader::FAMILY,
                0x00b9_7e30,
                shader::state::set_z_write_enable(e, 0u8, 1u32)
            );
        }
    }
    e.with_stack(0x40, |e, frame| {
        let matrix = frame.addr();
        typed_call!(
            crate::dispatch::FAMILY,
            0x00c4_c2d0,
            crate::dispatch::math::inverse_4x4(e, Ptr::new(transform), skin, Ptr::new(matrix))
        );
        let renderer = e.global::<u32>(base.wrapping_add(0x18));
        typed_call!(
            shader::FAMILY,
            0x00e6_fbb0,
            accumulator::set_world_transform(e, Ptr::new(renderer), Ptr::new(transform), 0u8)
        );
        let matrix_words = |e: &Engine| -> [u32; 16] {
            let mut words = [0u32; 16];
            for (k, word) in (0u32..).zip(words.iter_mut()) {
                *word = e.global::<u32>(matrix + 4 * k);
            }
            words
        };
        if pass_type == 0x1e0 || pass_type == 0x1e1 {
            let current = packet::current_pass(e);
            e.call(SET_UP_DCD0, &args![base, property, current]);
            let current = packet::current_pass(e);
            e.call(SET_UP_DE20, &args![base, geometry, property, current]);
            let words = matrix_words(e);
            typed_call!(
                super::FAMILY,
                0x00bb_b890,
                light_position(e, this, words, Ptr::new(transform))
            );
            return;
        }
        if pass_type == 0xe || pass_type == 0xf {
            let current = packet::current_pass(e);
            e.call(SET_UP_DDA0, &args![base, property, current]);
        }
        typed_call!(
            super::FAMILY,
            0x00bb_c050,
            lights(
                e,
                this,
                Ptr::new(property),
                Ptr::new(pass),
                pass_type,
                Ptr::new(material_property),
                matrix,
                skin,
                Ptr::new(transform)
            )
        );
        typed_call!(
            super::FAMILY,
            0x00bb_e6b0,
            view_matrix(
                e,
                this,
                Ptr::new(pass),
                pass_type,
                skin,
                Ptr::new(transform)
            )
        );
        typed_call!(
            super::FAMILY,
            0x00bb_ba00,
            world_matrix(e, this, skin, Ptr::new(transform))
        );
        if entry_on(e, REFRACTION_ENTRY) {
            let seconds = f32::from_bits(packet::game_time(e));
            let hours = div_d(seconds, e.global(SECONDS_PER_HOUR));
            let angle = mul_d(hours, e.global(CYCLE));
            #[allow(clippy::cast_possible_wrap)]
            let count = packet::lighting_word(e, property, 0xe4) as i32;
            let value = narrow(f64::from(angle) * f64::from(count));
            own::set_constant_word(e, REFRACTION, value.to_bits());
        }
        typed_call!(
            super::FAMILY,
            0x00bb_bcf0,
            eye_position(e, this, Ptr::new(bound))
        );
        let words = matrix_words(e);
        typed_call!(
            super::FAMILY,
            0x00bb_b890,
            light_position(e, this, words, Ptr::new(transform))
        );
        typed_call!(
            super::FAMILY,
            0x00bb_bda0,
            emittance(e, this, Ptr::new(property))
        );
        typed_call!(
            super::FAMILY,
            0x00bb_e9c0,
            material(e, this, bound, Ptr::new(property), state, pass_type)
        );
        typed_call!(
            super::FAMILY,
            0x00bb_ecd0,
            colour(e, this, Ptr::new(material_property), Ptr::new(property))
        );
        typed_call!(
            super::FAMILY,
            0x00bb_bdf0,
            glow(e, this, Ptr::new(property))
        );
        if entry_on(e, PARALLAX_ENTRY) {
            for k in 0..2u32 {
                let word = packet::lighting_word(e, property, 0xe8 + 4 * k);
                own::set_constant_word(e, PARALLAX + 4 * k, word);
            }
        }
        typed_call!(super::FAMILY, 0x00bb_be30, alpha(e, Ptr::new(property)));
        typed_call!(super::FAMILY, 0x00bb_be80, fog(e, this));
        if pass_type.wrapping_sub(0x1c4) <= 3 {
            let light = packet::rendering_light(e, packet::rendering_pass(e), 0);
            let refused = packet::clip_planes_made(e, light) != 0 || {
                let held = packet::shader_accumulator(e);
                packet::skips_type13(e, held) != 0 || {
                    let held = packet::shader_accumulator(e);
                    packet::shadow_groups(e, Ptr::new(held))
                }
            };
            if !refused {
                shadow_clip_planes(e, base, light);
            } else {
                let held = packet::shader_accumulator(e);
                if packet::skips_type13(e, held) == 0 {
                    let held = packet::shader_accumulator(e);
                    if !packet::shadow_groups(e, Ptr::new(held)) {
                        stored_clip_planes(e, light);
                    }
                }
            }
        }
    });
    let words = [geometry, skin, 0, buffers, block, 0, transform, bound, 0, 1];
    for (map_offset, holder_offset, program_offset) in
        [(0x30u32, 0x34u32, 0x44u32), (0x34, 0x48, 0x5c)]
    {
        let current = packet::current_pass(e);
        if own::constant_map(e, base, map_offset) == 0 {
            continue;
        }
        let program = e.global::<u32>(current.wrapping_add(program_offset));
        if program == 0 {
            continue;
        }
        let map = e.global::<u32>(current.wrapping_add(holder_offset));
        group_c::set_constants_of(e, map, program, words);
    }
    let other = packet::state_property(e, block, 4);
    if other != 0 && packet::ni_property_flags(e, other) & 1 != 0 {
        let target = e.global::<u32>(base.wrapping_add(0x1c));
        e.vcall(target, 0x24, &args![other]);
    }
}

/// The shader's switch tables (filled at load, read in place): its entries (the pointers at
/// `01201080`, seventeen, and `012010c4`, ten) and the pass indices' masks (`01200a28`, `01200868`); the
/// flag bytes the switches copy (`01200854`, seventeen, and `01200848`, ten); the fog constants
/// (`012013b0`, the colour `012013c0`) and their entry (`012010bc`); the eye constant (`012013d0`) and its
/// entry (`012010c0`); the exe's far distance for a fog without distances (`010ad6e0`).
const ENTRIES_A: u32 = 0x0120_1080;
const ENTRIES_B: u32 = 0x0120_10c4;
const MASKS_A: u32 = 0x0120_0a28;
const MASKS_B: u32 = 0x0120_0868;
const FLAGS_A: u32 = 0x0120_0854;
const FLAGS_B: u32 = 0x0120_0848;
const SWITCH_FOG: u32 = 0x0120_13b0;
const SWITCH_FOG_COLOUR: u32 = 0x0120_13c0;
const SWITCH_FOG_ENTRY: u32 = 0x0120_10bc;
const SWITCH_EYE: u32 = 0x0120_13d0;
const SWITCH_EYE_ENTRY: u32 = 0x0120_10c0;
const FOG_FAR_DEFAULT: u32 = 0x010a_d6e0;

/// The mask index of a pass type, as the exe's compares and jump tables give it (`00bbe5f4`,
/// `00bbe60c`, the bytes `00bbe634` into `00bbe628`).
fn pass_index(pass_type: u32) -> u32 {
    match pass_type {
        10..=15 => pass_type - 9,
        0x1d3..=0x1da => pass_type - 0x17a,
        0x2da | 0x2e0 | 0x2e6 | 0x2ec => 0x68,
        0x2db | 0x2e1 | 0x2e7 | 0x2ed => 0x69,
        _ => pass_type.wrapping_sub(0x178),
    }
}

// Typed from 00bbe120
/// The lighting 3.0 shader's slot 0xfc, `thiscall(shader, pass type)`: nothing for 0x1e0 and 0x1e1.
/// Otherwise the pass type's mask index: its first mask's bits 1 to 17 switch the first seventeen entries
/// (when there) and set the flag bytes `01200854`, its second mask's bits 1 to 10 the ten others and the
/// bytes `01200848`; the shader's +0x34 and +0x30 objects' slot 0x24; with the fog entry on and a fog
/// property, the fog constants (the exe's far distance, 0.0, the power, 0.0 and the fog colour when the
/// far and near distances are both 0.0, else far, far less near, the power, 0.0 and its colour with 1.0);
/// with the eye entry on, the sun less the origin with 1.0.
pub fn setup_switches(e: &mut Engine, this: Ptr, pass_type: u32) {
    if pass_type == 0x1e0 || pass_type == 0x1e1 {
        return;
    }
    let index = pass_index(pass_type);
    let mask_a = MASKS_A.wrapping_add(index.wrapping_mul(4)); // own: an address, read through own_at
    let mut bit = 1u32;
    for i in 0..0x11u32 {
        let entry = capture::own_at::<u32>(e, ENTRIES_A + 4 * i);
        if entry != 0 {
            let mask = capture::own_at::<u32>(e, mask_a);
            own::set_entry_enabled(e, entry, mask & bit.rotate_left(1) != 0);
        }
        bit = bit.rotate_left(1);
        let mask = capture::own_at::<u32>(e, mask_a);
        own::set_constant_byte(e, FLAGS_A + i, u8::from(mask & bit != 0));
    }
    let mask_b = MASKS_B.wrapping_add(index.wrapping_mul(4)); // own: an address, read through own_at
    for j in 1..=10u32 {
        let entry = capture::own_at::<u32>(e, ENTRIES_B + 4 * (j - 1));
        if entry != 0 {
            let mask = capture::own_at::<u32>(e, mask_b);
            own::set_entry_enabled(e, entry, (mask >> j) & 1 != 0);
        }
        let mask = capture::own_at::<u32>(e, mask_b);
        own::set_constant_byte(e, FLAGS_B + (j - 1), u8::from((mask >> j) & 1 != 0));
    }
    let base = this.addr();
    let first = e.global::<u32>(base.wrapping_add(0x34));
    e.vcall(first, 0x24, &[]);
    let second = e.global::<u32>(base.wrapping_add(0x30));
    e.vcall(second, 0x24, &[]);
    if entry_on(e, SWITCH_FOG_ENTRY) {
        let index = u32::from(packet::fog_index(e));
        let fog: u32 = typed_call!(
            super::FAMILY,
            0x00b5_5520,
            super::passes::fog_property(e, index)
        );
        if fog != 0 {
            let start = packet::fog_field(e, fog, 0x2c);
            let colour = [0x20u32, 0x24, 0x28].map(|offset| packet::fog_field(e, fog, offset));
            let end = packet::fog_field(e, fog, 0x30);
            let power = packet::fog_field(e, fog, 0x60);
            let (end_value, start_value) = (f32::from_bits(end), f32::from_bits(start));
            if end_value == 0.0 && start_value == 0.0 {
                let far = e.global::<u32>(FOG_FAR_DEFAULT);
                for (k, word) in (0u32..).zip([far, 0, power, 0]) {
                    own::set_constant_word(e, SWITCH_FOG + 4 * k, word);
                }
                for k in 0..4u32 {
                    let word = packet::fog_colour_word(e, k);
                    own::set_constant_word(e, SWITCH_FOG_COLOUR + 4 * k, word);
                }
            } else {
                let range = (end_value - start_value).to_bits();
                for (k, word) in (0u32..).zip([end, range, power, 0]) {
                    own::set_constant_word(e, SWITCH_FOG + 4 * k, word);
                }
                for (k, word) in (0u32..).zip([colour[0], colour[1], colour[2], ONE]) {
                    own::set_constant_word(e, SWITCH_FOG_COLOUR + 4 * k, word);
                }
            }
        }
    }
    if entry_on(e, SWITCH_EYE_ENTRY) {
        for k in 0..3u32 {
            let sun = f32::from_bits(packet::sun_word(e, k));
            let value = sun - f32::from_bits(packet::origin_word(e, k));
            own::set_constant_word(e, SWITCH_EYE + 4 * k, value.to_bits());
        }
        own::set_constant_word(e, SWITCH_EYE + 0xc, ONE);
    }
}

/// The shadow light shader's stage set-up `00b7c1f0` (`stdcall(property, stage, index, flag)`, by
/// address; its neighbours `00b7c0a0`, `00b7c0e0` and the base `00b7d200` are the shader family's).
const STAGE_TEXTURE_F: u32 = 0x00b7_c1f0;

/// One of the fixed stage set-ups of [`setup_stage_textures`]: `00b7c0a0(property, stage, texture)`,
/// `00b7c0e0(property, stage, word)` or `00b7c1f0(property, stage, index, flag)`.
#[derive(Clone, Copy)]
enum Stage {
    Texture(u32, u32),
    Second(u32, u32),
    Indexed(u32, u32, u32),
}

/// The exe's jump table for the pass types 0xc5 to 0x142 (the bytes at `00bd03a0` into the four targets
/// at `00bd0390`): 0, 1 and 2 pick a list, 3 the base.
const LIST_BYTES: [u8; 126] = [
    0, 3, 3, 0, 3, 3, 3, 0, 3, 3, 0, 3, 3, 3, 0, 3, 3, 0, 3, 3, 3, 3, 1, 3, 3, 1, 3, 3, 3, 3, 3, 1,
    3, 3, 1, 3, 3, 3, 3, 3, 1, 3, 3, 1, 3, 3, 3, 3, 3, 1, 3, 3, 1, 3, 3, 3, 3, 3, 3, 0, 3, 3, 3, 3,
    3, 3, 0, 3, 3, 3, 3, 3, 0, 3, 3, 3, 3, 3, 0, 3, 3, 3, 3, 0, 3, 3, 3, 3, 3, 0, 3, 3, 3, 3, 0, 3,
    3, 3, 3, 3, 0, 3, 3, 3, 3, 3, 3, 3, 3, 3, 2, 3, 3, 2, 3, 3, 3, 3, 3, 3, 3, 3, 2, 3, 3, 2,
];

/// The three lists (`00bd032c`, `00bd030c`, `00bd0356`).
const LISTS: [&[Stage]; 3] = [
    &[
        Stage::Texture(0, 0),
        Stage::Second(1, 0),
        Stage::Texture(2, 1),
        Stage::Second(3, 1),
        Stage::Indexed(4, 0, 1),
    ],
    &[
        Stage::Texture(0, 0),
        Stage::Second(1, 0),
        Stage::Indexed(3, 0, 1),
    ],
    &[Stage::Second(0, 0), Stage::Indexed(4, 0, 1)],
];

// Typed from 00bd02e0
/// Slot 0x78 (`SetupGeometryTextures`) of the vtable `010bb980`, `thiscall(shader, state)`: by the pass
/// type, in 0xc5 to 0x142 through the exe's jump table, one of three fixed lists of stage set-ups with the
/// state's shader property, then the shader's slot 0x108; every other pass type the base set-up
/// (`00b7d200`, the shader family's).
pub fn setup_stage_textures(e: &mut Engine, this: Ptr, state: Ptr) {
    let base = this.addr();
    let pass_type = packet::rendering_pass_type(e);
    let property = packet::state_property(e, state.addr(), 3);
    let case = pass_type.wrapping_sub(0xc5);
    let list = LIST_BYTES
        .get(case as usize)
        .and_then(|byte| LISTS.get(usize::from(*byte)));
    let Some(list) = list else {
        typed_call!(
            shader::FAMILY,
            0x00b7_d200,
            shader::textures::setup_geometry_textures(e, this, state)
        );
        return;
    };
    for stage in list.iter() {
        match *stage {
            Stage::Texture(stage, texture) => typed_call!(
                shader::FAMILY,
                0x00b7_c0a0,
                shader::textures::stage_texture(e, Ptr::new(property), stage, texture)
            ),
            Stage::Second(stage, word) => typed_call!(
                shader::FAMILY,
                0x00b7_c0e0,
                shader::textures::stage_second_texture(e, Ptr::new(property), stage, word)
            ),
            Stage::Indexed(stage, index, flag) => {
                e.call(STAGE_TEXTURE_F, &args![property, stage, index, flag]);
            }
        }
    }
    e.vcall(base, 0x108, &[]);
}

// Typed from 00bbe020
/// Lighting 3.0's pass choice (slot 0xf0), `thiscall(shader, pass type)` (not a Ghidra function: typed from its
/// bytes): the pass [`pass_for_type`] (`00c0abd0`) answers for the type becomes the current pass; the fog-coloured ambient
/// ([`own::fog_ambient`], which the exe has inlined here); then for the types 0x1e0 and 0x1e1 the counted pointers
/// +0x30 and +0x34 take the shader's maps +0x6c and +0x70 (the exe's inlined assignment, the new one read again
/// after the old one's count went down), for every other type `00c663d0` copies +0x74 into +0x30 and +0x78 into
/// +0x34.
pub fn select_pass(e: &mut Engine, this: Ptr, pass_type: u32) {
    let base = this.addr();
    let pass: u32 = typed_call!(super::FAMILY, 0x00c0_abd0, pass_for_type(e, this, pass_type));
    own::set_current_pass(e, pass);
    own::fog_ambient_inline(e);
    if pass_type == 0x1e0 || pass_type == 0x1e1 {
        own::assign_counted(e, base.wrapping_add(0x30), base.wrapping_add(0x6c), true);
        own::assign_counted(e, base.wrapping_add(0x34), base.wrapping_add(0x70), true);
    } else {
        own::copy_counted(e, base.wrapping_add(0x30), base.wrapping_add(0x74));
        own::copy_counted(e, base.wrapping_add(0x34), base.wrapping_add(0x78));
    }
}

// Typed from 00bbc5c0
/// Lighting 3.0's blending (slot 0x80), `thiscall(shader, state)` (not a Ghidra function): nothing in the pass
/// types 0x1bd to 0x1c3, 0x1cc to 0x1dc, 0x1e0, 0x1e1 and 0x2d8 to 0x2ef; otherwise the shared blending of the state
/// (`00be1ff0`, the shader family's: the exe's tail jump). Answers what the exe leaves in `EAX`: the pass type when it does
/// nothing, else 0 (the batch renderer reads no answer).
pub fn blending(e: &mut Engine, this: Ptr, state: Ptr) -> u32 {
    let pass_type = packet::rendering_pass_type(e);
    let none = (0x1cc..=0x1dc).contains(&pass_type)
        || (0x1bd..=0x1c3).contains(&pass_type)
        || (0x1e0..=0x1e1).contains(&pass_type)
        || (0x2d8..=0x2ef).contains(&pass_type);
    if none {
        return pass_type;
    }
    typed_call!(
        shader::FAMILY,
        0x00be_1ff0,
        accumulator::set_blend_of(e, this.addr(), state.addr())
    );
    0
}

/// The depth-write lock counter (`011ff9dc`, the lock counters' second word) that lighting 3.0's geometry end
/// lowers by hand.
const Z_WRITE_COUNTER: u32 = 0x011f_f9dc;

// Typed from 00bbeef0
/// Lighting 3.0's geometry end (slot 0x8c), `thiscall(shader, state)` (the shader not read; not a Ghidra function):
/// in the pass types 0x1e0 and 0x1e1 the depth function put back (`00b97e80(3, 0)`); the stencil test off
/// (`00b98070(0, 0)`) when the state's property 4 has bit 0 of its flags; in the types 0x1d3 to 0x1d6, when the
/// state's alpha property (no null check, as the exe) has bit 0 of its flags, the depth-write lock counter lowered
/// and depth writes on (`SetZWriteEnable(1, 0)`).
pub fn geometry_done(e: &mut Engine, _this: Ptr, state: Ptr) {
    let pass_type = packet::rendering_pass_type(e);
    if pass_type == 0x1e0 || pass_type == 0x1e1 {
        typed_call!(
            shader::FAMILY,
            0x00b9_7e80,
            crate::shader::state::set_z_function(e, 3, 0)
        );
    }
    let block = state.addr();
    let other = packet::state_property(e, block, 4);
    if other != 0 && packet::ni_property_flags(e, other) & 1 != 0 {
        typed_call!(
            shader::FAMILY,
            0x00b9_8070,
            crate::shader::state::set_stencil_enable(e, 0u8, 0)
        );
    }
    if pass_type.wrapping_sub(0x1d3) <= 3 {
        let alpha = packet::state_property(e, block, 0);
        if packet::ni_property_flags(e, alpha) & 1 != 0 {
            let count = capture::own_at::<u32>(e, Z_WRITE_COUNTER).wrapping_sub(1);
            capture::set_own_at(e, Z_WRITE_COUNTER, count);
            typed_call!(
                shader::FAMILY,
                0x00b9_7e30,
                crate::shader::state::set_z_write_enable(e, 1u8, 0)
            );
        }
    }
}

/// The bone constants' switch object lighting 3.0 uploads under (`01201090`: with its byte +8 set, a shader
/// without its own bone constants still uploads them) and `"BoneMatrix3"` (`010a1cac`).
const BONE_SWITCH: u32 = 0x0120_1090;
const BONE_MATRIX_NAME: u32 = 0x010a_1cac;

// Typed from 00bbd000
/// Lighting 3.0's partition set-up (slot 0x64), `thiscall(shader, eight words: an unread word, the skin instance,
/// the partition, three unread words, the world transform, an unread word)`: only with a partition, no +0x38 and
/// either the switch object (`01201090`) present with its byte +8 set or the shader's +0x34 its own bone constants
/// (+0x70): the renderer's bone matrices for the skin with the world transform (3 registers a bone, the world left
/// out), then the current pass's constants (+0x5c) get the partition's bones through their slot 0x78: under the
/// shader's "BoneMatrix3" handle (slot 0x70 of +0x70) when they are its own, else under the switch, the skin's
/// buffer (+0x28), the partition's 16-bit count (+0x20), the skin's word +0x20 and the partition's bone indices
/// (+4). Answers 0.
#[allow(clippy::too_many_arguments)]
pub fn bone_setup(
    e: &mut Engine,
    this: Ptr,
    _unused_0: u32,
    skin: u32,
    partition: u32,
    _unused_3: u32,
    _unused_4: u32,
    _unused_5: u32,
    world: u32,
    _unused_7: u32,
) -> u32 {
    let t = this.addr();
    if partition == 0 || e.global::<u32>(t.wrapping_add(0x38)) != 0 {
        return 0;
    }
    let switch = capture::own_at::<u32>(e, BONE_SWITCH);
    let switched = switch != 0 && e.global::<u8>(switch.wrapping_add(8)) != 0;
    if !switched && e.global::<u32>(t.wrapping_add(0x34)) != e.global::<u32>(t.wrapping_add(0x70)) {
        return 0;
    }
    let renderer = e.global::<u32>(packet::RENDERER);
    typed_call!(
        super::FAMILY,
        0x00e6_fe30,
        super::skin::bone_matrices(
            e,
            Ptr::new(renderer),
            Ptr::new(skin),
            Ptr::new(world),
            0u8,
            3u32,
            1u8
        )
    );
    let map = e.global::<u32>(t.wrapping_add(0x70));
    let matrices = e.global::<u32>(skin.wrapping_add(0x28));
    let rows = e.global::<u32>(skin.wrapping_add(0x20));
    let indices = e.global::<u32>(partition.wrapping_add(4));
    let count = u32::from(e.global::<u16>(partition.wrapping_add(0x20)));
    let constants = e.global::<u32>(packet::current_pass(e).wrapping_add(0x5c));
    let entry = if e.global::<u32>(t.wrapping_add(0x34)) == map {
        e.vcall(map, 0x70, &args![BONE_MATRIX_NAME]).u32()
    } else {
        capture::own_at::<u32>(e, BONE_SWITCH)
    };
    e.vcall(
        constants,
        0x78,
        &args![entry, matrices, count, rows, indices],
    );
    0
}

/// What lighting 3.0's stage set-ups call by address (none reads or writes the draw's own state; each works on the
/// pass handed to it): a stage's set-up after its texture (`00bb9640`, `thiscall(shader, stage, property)`), the
/// rendered texture's `GetTexture` (`004bc320`), the pass constants' setters (`00c0bae0`, `00c0bb20`, `00c0bb50`,
/// `00c0bba0`, `00c0bbd0`, `00c0bc00`, `00c0bcf0`, `thiscall(shader, pass, ...)`) and the two set-ups of their own
/// (`00bbdda0`, `00bbdcd0`, `thiscall(shader, property, pass)`).
const STAGE_SETUP: u32 = 0x00bb_9640;
const RENDERED_TEXTURE: u32 = 0x004b_c320;
const CONSTANTS_BAE0: u32 = 0x00c0_bae0;
const CONSTANTS_BB20: u32 = 0x00c0_bb20;
const CONSTANTS_BB50: u32 = 0x00c0_bb50;
const CONSTANTS_BBA0: u32 = 0x00c0_bba0;
const CONSTANTS_BBD0: u32 = 0x00c0_bbd0;
const CONSTANTS_BC00: u32 = 0x00c0_bc00;
const CONSTANTS_BCF0: u32 = 0x00c0_bcf0;
const SETUP_DDA0: u32 = 0x00bb_dda0;
const SETUP_DCD0: u32 = 0x00bb_dcd0;
/// The fallback texture (`011f9510`), the texture filter setting (`011f91b0`: 3 asks for filter mode 1), the
/// word the pass type 0x181 puts in entry +0x10 (`011f9530`), the shadow flag constant (`01201c7c`).
const DEFAULT_TEXTURE: u32 = 0x011f_9510;
const FILTER_SETTING: u32 = 0x011f_91b0;
const ENTRY_181_WORD: u32 = 0x011f_9530;
const SHADOW_FLAG: u32 = 0x0120_1c7c;
/// A lighting property's light count getter (`00b704b0`, `fastcall(property)`: its 16 bits at +0x68), by address.
const LIGHT_COUNT: u32 = 0x00b7_04b0;

/// Stage `index` of `pass` (its stages +0x24: the pass's contents, read in place).
fn stage_of(e: &Engine, pass: u32, index: u32) -> u32 {
    let stages = e.global::<u32>(pass.wrapping_add(0x24));
    e.global::<u32>(stages.wrapping_add(index.wrapping_mul(4)))
}

/// Stage 0 of `pass` takes the word at the property's `[+0xac]`, then the stage's set-up (`00bb9640`).
fn first_stage(e: &mut Engine, this: u32, pass: u32, property: u32) {
    let stage = stage_of(e, pass, 0);
    let source = e.global::<u32>(property.wrapping_add(0xac));
    let texture = e.global::<u32>(source);
    own::set_stage_texture(e, stage, texture);
    e.call(STAGE_SETUP, &args![this, stage, property]);
}

// Typed from 00c0cd80
/// Lighting 3.0's stage textures (its slot 0x13c), `thiscall(shader, pass type, property)` (not a Ghidra function:
/// typed from its bytes). With the current pass read once: unless the type is in 0x1bd to 0x1e1 outside 0x1c4 to
/// 0x1cb, or in 0x2d8 to 0x2ef, stage 0 takes the property's `[+0xac]` word and stage 1 (when there is one) its
/// `[+0xb0]` word or the exe's fallback (`011f9510`), each followed by the stage's set-up (`00bb9640`); then the
/// shadow stage ([`shadow_stage`]). For the types 0x2e0, 0x2e1, 0x2ec, 0x2ed and 0x1d3 to 0x1d6, stage 0 is set
/// again as at the start.
pub fn stage_textures(e: &mut Engine, this: Ptr, pass_type: u32, property: u32) {
    let base = this.addr();
    let pass = packet::current_pass(e);
    let skipped = (pass_type.wrapping_sub(0x1bd) <= 0x24
        && !(0x1c4..=0x1c7).contains(&pass_type)
        && pass_type.wrapping_sub(0x1c8) > 3)
        || (0x2d8..=0x2ef).contains(&pass_type);
    if !skipped {
        first_stage(e, base, pass, property);
        let second = stage_of(e, pass, 1);
        if second != 0 {
            let source = e.global::<u32>(property.wrapping_add(0xb0));
            let mut texture = e.global::<u32>(source);
            if texture == 0 {
                texture = e.global::<u32>(DEFAULT_TEXTURE);
            }
            own::set_stage_texture(e, second, texture);
            e.call(STAGE_SETUP, &args![base, second, property]);
        }
        shadow_stage(e, pass, property);
    }
    if matches!(pass_type, 0x2e0 | 0x2e1 | 0x2ec | 0x2ed) || pass_type.wrapping_sub(0x1d3) <= 3 {
        first_stage(e, base, pass, property);
    }
}

/// [`stage_textures`]'s third stage: when the rendered pass's type is 0x1c4 to 0x1c7, the property's light count
/// (`00b704b0`, by address: its 16 bits at +0x68) is not 0, the rendered pass's first scene light has its
/// byte +0xec and a shadow map (+0x10c) and the rendered pass its byte 9: stage 2 takes the shadow map's texture
/// (`GetTexture(0)`), filter mode 1 when the filter setting (`011f91b0`) is 3 else 0, address mode 0, and the shadow
/// flag constant (`01201c7c`) becomes 1.0 or 0.0 by the property's flag 8 (`004a2020`; `FILD`/`FSTP`).
fn shadow_stage(e: &mut Engine, pass: u32, property: u32) {
    if packet::rendering_pass_type(e).wrapping_sub(0x1c4) > 3 {
        return;
    }
    if e.call(LIGHT_COUNT, &args![property]).u16() == 0 {
        return;
    }
    let rendering = packet::rendering_pass(e);
    let light = packet::rendering_first_light(e);
    if packet::pass_light_byte(e, light, 0xec) == 0 {
        return;
    }
    let shadow_map = packet::scene_light_word(e, light, 0x10c);
    if shadow_map == 0 || packet::rendering_pass_byte9(e, rendering) == 0 {
        return;
    }
    let third = stage_of(e, pass, 2);
    let texture = e.call(RENDERED_TEXTURE, &args![shadow_map, 0u32]).u32();
    own::set_stage_texture(e, third, texture);
    let mode = u32::from(e.global::<u32>(FILTER_SETTING) == 3);
    typed_call!(
        shader::FAMILY,
        0x00e7_def0,
        crate::shader::state::set_filter_mode(e, Ptr::new(third), mode)
    );
    typed_call!(
        shader::FAMILY,
        0x00e7_de90,
        crate::shader::state::set_address_modes(e, Ptr::new(third), 0)
    );
    let flagged: bool = typed_call!(
        crate::render::FAMILY,
        0x004a_2020,
        crate::render::passes::has_flag(e, property, 8)
    );
    let value = if flagged { ONE } else { 0 };
    own::set_constant_word(e, SHADOW_FLAG, value);
}

/// Entry `slot` (a byte offset into the pass's entries +0x24) of `pass` given `value` at +8.
fn set_entry(e: &mut Engine, pass: u32, slot: u32, value: u32) {
    let entries = e.global::<u32>(pass.wrapping_add(0x24));
    let entry = e.global::<u32>(entries.wrapping_add(slot));
    e.set_global(entry.wrapping_add(8), value);
}

/// The word at `at` of the object at the property's `offset` (its texture sources).
fn source_word(e: &Engine, property: u32, offset: u32, at: u32) -> u32 {
    let source = e.global::<u32>(property.wrapping_add(offset));
    e.global::<u32>(source.wrapping_add(at))
}

/// One step of [`stage_constants`]'s lists.
#[derive(Clone, Copy)]
enum ConstantStep {
    Bae0,
    Bb20,
    Bb50,
    Bbd0,
    Mapped,
    Entry18,
}

/// [`stage_constants`]'s lists for the types 0x182 to 0x1b8 (the exe's jump table `00c0d0cc` with the bytes
/// `00c0d110`, read from the exe).
fn constant_steps(pass_type: u32) -> &'static [ConstantStep] {
    use ConstantStep::{Bae0, Bb20, Bb50, Bbd0, Entry18, Mapped};
    match pass_type {
        0x182 | 0x183 | 0x18a | 0x18b | 0x191 | 0x192 | 0x199 | 0x19a => &[Bae0],
        0x186 | 0x187 | 0x18c | 0x18d | 0x195 | 0x196 | 0x19b | 0x19c => &[Entry18],
        0x1a0 | 0x1a1 | 0x1a8 | 0x1a9 | 0x1af | 0x1b0 | 0x1b7 | 0x1b8 => &[Bbd0, Bae0],
        0x1a4 | 0x1a5 | 0x1aa | 0x1ab | 0x1b3 | 0x1b4 | 0x1b9 | 0x1ba => &[Entry18, Bbd0],
        0x19d | 0x19e | 0x1a6 | 0x1a7 | 0x1ac | 0x1ad | 0x1b5 | 0x1b6 => &[Bbd0],
        0x190 => &[Bb20],
        0x19f | 0x1ae => &[Bb20, Bbd0],
        0x1a2 | 0x1b1 => &[Bbd0, Mapped],
        0x184 | 0x193 => &[Mapped],
        0x1a3 | 0x1b2 => &[Bbd0, Bb50],
        0x185 | 0x194 => &[Bb50],
        _ => &[],
    }
}

// Typed from 00c0cf10
/// Lighting 3.0's stage constants (its slot 0x140), `thiscall(shader, pass type, property)` (not a Ghidra function:
/// typed from its bytes). With the current pass read once: types 0xe and 0xf run `00bbdda0(property, pass)`; 0x181
/// sets the pass's entries +0x10 (the exe's word `011f9530`) and +0x14 (the property's `[+0xb4]` word); 0x182 to
/// 0x1ba run the setters of the exe's jump table ([`constant_steps`]: the entry +0x10 and +0x14 from the property's
/// `[+0xac]` and `[+0xb0]` objects' word +4, the entry +0x18 from its `[+0xb4]` word); 0x1bd to 0x1c3 the setters
/// `00c0bc00` (with the type), the emittance stage ([`emittance_stage`]) and `00c0bcf0`; 0x1cc, 0x1ce and 0x1d0
/// `00c0bba0`; 0x1e0 and 0x1e1 `00bbdcd0(property, pass)`; other types: nothing.
pub fn stage_constants(e: &mut Engine, this: Ptr, pass_type: u32, property: u32) {
    let base = this.addr();
    let pass = packet::current_pass(e);
    if pass_type <= 0x181 {
        if pass_type == 0x181 {
            let word = e.global::<u32>(ENTRY_181_WORD);
            set_entry(e, pass, 0x10, word);
            let value = source_word(e, property, 0xb4, 0);
            set_entry(e, pass, 0x14, value);
        } else if pass_type == 0xe || pass_type == 0xf {
            e.call(SETUP_DDA0, &args![base, property, pass]);
        }
        return;
    }
    match pass_type {
        0x1bd..=0x1c3 => {
            e.call(CONSTANTS_BC00, &args![base, pass, property, pass_type]);
            typed_call!(
                super::FAMILY,
                0x00c0_bc90,
                emittance_stage(e, this, pass, property)
            );
            e.call(CONSTANTS_BCF0, &args![base, pass, property, 0u32]);
            return;
        }
        0x1e0 | 0x1e1 => {
            e.call(SETUP_DCD0, &args![base, property, pass]);
            return;
        }
        0x1cc | 0x1ce | 0x1d0 => {
            e.call(CONSTANTS_BBA0, &args![base, pass, property]);
            return;
        }
        _ => {}
    }
    for step in constant_steps(pass_type) {
        match step {
            ConstantStep::Bae0 => {
                e.call(CONSTANTS_BAE0, &args![base, pass, property, 4u32]);
            }
            ConstantStep::Bb20 => {
                e.call(CONSTANTS_BB20, &args![base, pass, property]);
            }
            ConstantStep::Bb50 => {
                e.call(CONSTANTS_BB50, &args![base, pass, property, 1u32]);
            }
            ConstantStep::Bbd0 => {
                e.call(CONSTANTS_BBD0, &args![base, pass]);
            }
            ConstantStep::Mapped => {
                let first = source_word(e, property, 0xac, 4);
                set_entry(e, pass, 0x10, first);
                let second = source_word(e, property, 0xb0, 4);
                set_entry(e, pass, 0x14, second);
            }
            ConstantStep::Entry18 => {
                let value = source_word(e, property, 0xb4, 0);
                set_entry(e, pass, 0x18, value);
            }
        }
    }
}

/// The emittance stage's constants (`01201bac`: 1.0 with the property's emittance texture, 0.0 with the fallback;
/// `01201ba8`: the property's +0x30, through `FLD`/`FSTP`, so quieted) and the fallback texture for it (`011f9514`).
const EMITTANCE_ON: u32 = 0x0120_1bac;
const EMITTANCE_VALUE: u32 = 0x0120_1ba8;
const EMITTANCE_FALLBACK: u32 = 0x011f_9514;

// Typed from 00c0bc90
/// Lighting 3.0's emittance stage, `thiscall(shader, pass, property)`: stage 3 of the pass takes what the property's
/// slot 0xf4 answers for (5, 0), the emittance constant 1.0; without one the exe's fallback (`011f9514`) and 0.0;
/// then the stage's set-up (`00bb9640`) and the emittance value the property's +0x30 (the packet's).
pub fn emittance_stage(e: &mut Engine, this: Ptr, pass: u32, property: u32) {
    let texture = e.vcall(property, 0xf4, &args![5u32, 0u32]).u32();
    let stage = stage_of(e, pass, 3);
    own::set_constant_word(e, EMITTANCE_ON, ONE);
    let texture = if texture == 0 {
        own::set_constant_word(e, EMITTANCE_ON, 0);
        e.global::<u32>(EMITTANCE_FALLBACK)
    } else {
        texture
    };
    own::set_stage_texture(e, stage, texture);
    e.call(STAGE_SETUP, &args![this.addr(), stage, property]);
    let value = engine_core::x87::x87_quiet_bits(capture::word(e, property, 0x30));
    own::set_constant_word(e, EMITTANCE_VALUE, value);
}

/// Lighting 3.0's pass table (`01200ec0` on, a pass a word, in Group C's statics: the draw's own state, read through
/// the packet).
const PASS_TABLE: u32 = 0x0120_0ec0;

/// What [`pass_for_type`] does for a pass type (the exe's switch: `0xa` to `0xf` through the table `00c0b7a8`, 0x178
/// its own case, `0x17f` to `0x2ed` through the bytes `00c0b970` into the table `00c0b7c0`; decoded from the exe):
/// the shader's field whose word goes into its counted pointer +0x28, the pass's place in the pass table, and
/// whether the pass is read after the assignment rather than before it.
fn pass_case(pass_type: u32) -> Option<(u32, u32, bool)> {
    Some(match pass_type {
        0xa => (0x7c, 0x4, false),
        0xb => (0x8c, 0x8, false),
        0xc => (0x84, 0xc, false),
        0xd => (0x94, 0x10, false),
        0xe => (0x7c, 0x14, false),
        0xf => (0x8c, 0x18, false),
        0x178 => (0x7c, 0x0, false),
        0x17f => (0x7c, 0x1c, false),
        0x180 => (0x8c, 0x20, false),
        0x181 => (0x84, 0x24, false),
        0x182 => (0x7c, 0x28, false),
        0x183 => (0x7c, 0x2c, false),
        0x184 => (0x7c, 0x30, false),
        0x185 => (0x8c, 0x34, false),
        0x186 => (0x7c, 0x38, false),
        0x187 => (0x8c, 0x3c, false),
        0x188 => (0x84, 0x40, false),
        0x189 => (0x8c, 0x44, false),
        0x18a => (0x84, 0x48, false),
        0x18b => (0x84, 0x4c, false),
        0x18c => (0x84, 0x50, false),
        0x18d => (0x94, 0x54, false),
        0x18e => (0x7c, 0x58, false),
        0x18f => (0x8c, 0x5c, false),
        0x190 => (0x84, 0x60, false),
        0x191 => (0x7c, 0x64, false),
        0x192 => (0x7c, 0x68, false),
        0x193 => (0x7c, 0x6c, false),
        0x194 => (0x8c, 0x70, false),
        0x195 => (0x7c, 0x74, false),
        0x196 => (0x8c, 0x78, false),
        0x197 => (0x84, 0x7c, false),
        0x198 => (0x94, 0x80, false),
        0x199 => (0x84, 0x84, false),
        0x19a => (0x84, 0x88, false),
        0x19b => (0x84, 0x8c, false),
        0x19c => (0x94, 0x90, false),
        0x19d => (0x7c, 0x94, false),
        0x19e => (0x8c, 0x98, false),
        0x19f => (0x84, 0x9c, false),
        0x1a0 => (0x7c, 0xa0, false),
        0x1a1 => (0x7c, 0xa4, false),
        0x1a2 => (0x7c, 0xa8, false),
        0x1a3 => (0x8c, 0xac, false),
        0x1a4 => (0x7c, 0xb0, false),
        0x1a5 => (0x8c, 0xb4, false),
        0x1a6 => (0x84, 0xb8, false),
        0x1a7 => (0x94, 0xbc, false),
        0x1a8 => (0x84, 0xc0, false),
        0x1a9 => (0x84, 0xc4, false),
        0x1aa => (0x84, 0xc8, false),
        0x1ab => (0x94, 0xcc, false),
        0x1ac => (0x7c, 0xd0, false),
        0x1ad => (0x8c, 0xd4, false),
        0x1ae => (0x84, 0xd8, false),
        0x1af => (0x7c, 0xdc, false),
        0x1b0 => (0x7c, 0xe0, false),
        0x1b1 => (0x7c, 0xe4, false),
        0x1b2 => (0x8c, 0xe8, false),
        0x1b3 => (0x7c, 0xec, false),
        0x1b4 => (0x8c, 0xf0, false),
        0x1b5 => (0x84, 0xf4, false),
        0x1b6 => (0x94, 0xf8, false),
        0x1b7 => (0x84, 0xfc, false),
        0x1b8 => (0x84, 0x100, false),
        0x1b9 => (0x84, 0x104, false),
        0x1ba => (0x94, 0x108, false),
        0x1bb => (0x7c, 0x10c, false),
        0x1bc => (0x84, 0x110, false),
        0x1bd => (0x7c, 0x114, false),
        0x1be => (0x7c, 0x118, false),
        0x1bf => (0x84, 0x11c, false),
        0x1c0 => (0x84, 0x120, false),
        0x1c1 => (0x8c, 0x124, false),
        0x1c2 => (0x94, 0x128, false),
        0x1c3 => (0x7c, 0x12c, false),
        0x1c4 => (0x7c, 0x130, false),
        0x1c5 => (0x84, 0x134, false),
        0x1c6 => (0x8c, 0x138, false),
        0x1c7 => (0x8c, 0x13c, false),
        0x1c8 => (0x7c, 0x140, true),
        0x1c9 => (0x84, 0x144, true),
        0x1ca => (0x8c, 0x148, false),
        0x1cb => (0x94, 0x14c, false),
        0x1cc => (0x7c, 0x150, true),
        0x1cd => (0x84, 0x150, true),
        0x1ce => (0x8c, 0x154, false),
        0x1cf => (0x8c, 0x154, false),
        0x1d0 => (0x7c, 0x158, true),
        0x1d1 => (0x7c, 0x15c, true),
        0x1d2 => (0x8c, 0x160, false),
        0x1d3 => (0x7c, 0x164, false),
        0x1d4 => (0x84, 0x168, false),
        0x1d5 => (0x8c, 0x16c, false),
        0x1d6 => (0x94, 0x170, false),
        0x1d7 => (0x7c, 0x174, true),
        0x1d8 => (0x84, 0x178, true),
        0x1d9 => (0x8c, 0x17c, false),
        0x1da => (0x94, 0x180, false),
        0x1db => (0x7c, 0x184, false),
        0x1dc => (0x8c, 0x188, false),
        0x1dd => (0x7c, 0x18c, true),
        0x1de => (0x8c, 0x190, false),
        0x1df => (0x7c, 0x194, true),
        0x1e0 => (0x7c, 0x198, false),
        0x1e1 => (0x8c, 0x19c, false),
        0x2da => (0x7c, 0x1a0, false),
        0x2db => (0x8c, 0x1a4, false),
        0x2e0 => (0x7c, 0x1a8, false),
        0x2e1 => (0x8c, 0x1ac, false),
        0x2e6 => (0x84, 0x1b0, false),
        0x2e7 => (0x94, 0x1b4, false),
        0x2ec => (0x84, 0x1b8, false),
        0x2ed => (0x94, 0x1bc, false),
        _ => return None,
    })
}

// Typed from 00c0abd0
/// Lighting 3.0's pass for a pass type, `thiscall(shader, pass type)`: for a type of its switch ([`pass_case`]) the
/// shader's field goes into its counted pointer +0x28 (`00a5f860`) and the type's pass word is answered (read before
/// or after the assignment, as the exe reads it); 0 for any other type.
pub fn pass_for_type(e: &mut Engine, this: Ptr, pass_type: u32) -> u32 {
    let Some((field, place, after)) = pass_case(pass_type) else {
        return 0;
    };
    let base = this.addr();
    let word = e.global::<u32>(base.wrapping_add(field));
    let before = if after {
        0
    } else {
        capture::own_at::<u32>(e, PASS_TABLE + place)
    };
    own::set_counted(e, base.wrapping_add(0x28), word);
    if after {
        capture::own_at::<u32>(e, PASS_TABLE + place)
    } else {
        before
    }
}
