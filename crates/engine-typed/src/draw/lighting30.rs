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
    let factor = narrow(e.vcall(property, 0x110, &[]).f64());
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
            let data = packet::light_data(e, light);
            e.call(TRANSFORM_POINT, &args![data + 0x8c, vector, fallback]);
            let radius = packet::light_word(e, data, 0xc4);
            e.call(POINT_TIMES_SCALAR, &args![data + 0xd4, other, radius]);
            let distance = e.call(LENGTH, &args![vector]).f64();
            let ratio =
                narrow(distance / f64::from(f32::from_bits(packet::light_word(e, data, 0xe0))));
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
/// value)`, by address), the light colour (`011fa0d0`, four floats), the light data (`01201c70`: the
/// directional and point counts, the material's word, the fourth; then the 0x20-byte entries from
/// `01201c90`) and its entry (`012010e8`, its count word +0x20), the exe's limit on the light count
/// (`011ae60c`) and 0.5 (`01016248`), `GetNumberofActiveLights` (`00b70790`, by address).
const FIRST_LIGHT: u32 = 0x00b7_0de0;
const OTHER_LIGHT: u32 = 0x00b7_1130;
const LIGHT_COLOUR_ROW: u32 = 0x011f_a0d0;
const LIGHT_DATA: u32 = 0x0120_1c70;
const LIGHT_ENTRIES: u32 = 0x0120_1c90;
const LIGHT_DATA_ENTRY: u32 = 0x0120_10e8;
const LIGHT_LIMIT: u32 = 0x011a_e60c;
const HALF: u32 = 0x0101_6248;
const ACTIVE_LIGHTS: u32 = 0x00b7_0790;

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
        e.call(OTHER_LIGHT, &args![0u32, light, value]);
    } else {
        if count > 0 {
            let light = packet::rendering_light(e, pass, 0);
            e.call(FIRST_LIGHT, &args![0u32, light, value]);
        }
        let mut index = 1u32;
        while index < count {
            let light = packet::rendering_light(e, pass, index);
            e.call(OTHER_LIGHT, &args![index, light, value]);
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
        #[allow(clippy::cast_possible_truncation)]
        let active = e.call(ACTIVE_LIGHTS, &args![property]).u32() as u16;
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
/// `00bbde20`), the camera's position (`00b71b40`), the light's plane store (`00b71ad0`).
const SET_UP_DDA0: u32 = 0x00bb_dda0;
const SET_UP_DCD0: u32 = 0x00bb_dcd0;
const SET_UP_DE20: u32 = 0x00bb_de20;
const CAMERA_POSITION: u32 = 0x00b7_1b40;
const STORE_PLANES: u32 = 0x00b7_1ad0;
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
        e.call(CAMERA_POSITION, &args![camera, position]);
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
        e.call(STORE_PLANES, &args![light, planes]);
    });
}

/// The light's stored planes set on the device (when the light or the shader accumulator refuses the
/// computed ones): the six planes at +0x1a8.
fn stored_clip_planes(e: &mut Engine, light: u32) {
    for index in 0..6u32 {
        let device = packet::scene_renderer_device(e);
        own::device_clip_plane(e, device, index, light.wrapping_add(0x1a8 + 0x10 * index));
    }
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
            // own: the pass light's clip-plane cache byte (+0x208, written by the draw), piece 4; in place.
            let refused = packet::light_byte(e, light, 0x208) != 0 || {
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
