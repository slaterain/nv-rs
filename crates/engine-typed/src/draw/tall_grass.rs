//! The tall grass shader's set-ups (`TallGrassShader`, vtable `010b8980`; Group C of the drawing
//! thread's plan, `measure/typed-draw/GROUP-C.md`): what the world draw reached as translations. Live
//! reads go through [`super::packet`], the shader constants it fills through [`super::own`]. Main
//! thread only (the drawing thread replays main's draw with main's control word): `f32` maths.

use super::{group_c, own, packet, thunks};
use crate::anim_keys::narrow;
use crate::shader::{self, accumulator, div_d, mul_d};
use engine_core::ptr::Ptr;
use engine_core::{args, typed_call, Engine};

/// Bits of 1.0.
const ONE: u32 = 0x3f80_0000;
/// The constants this fills: the grass's up direction (two floats), the light's direction, its
/// ambient colour and its colour (four floats each).
const UP: u32 = 0x0120_0550;
const LIGHT_DIRECTION: u32 = 0x0120_0510;
const LIGHT_AMBIENT: u32 = 0x0120_0520;
const LIGHT_COLOUR: u32 = 0x0120_0570;
/// What it calls by address: whether a pass is a shadow pass (`00b630f0`, `cdecl(pass)`), a property's
/// active non-shadow lights (`00b707d0`, `thiscall(property)`, a count in AX), the point lights' set-up
/// (`00baa720`, `thiscall(shader, property, inverse, scale, pass)`), the shadow pass's
/// (`00baa850`, `thiscall(shader, word, transform)`) and the base `SetupGeometry` (`00e807d0`).
const IS_SHADOW_PASS: u32 = 0x00b6_30f0;
const ACTIVE_NON_SHADOW_LIGHTS: u32 = 0x00b7_07d0;
const UPDATE_POINT_LIGHTS: u32 = 0x00ba_a720;
const SHADOW_SETUP: u32 = 0x00ba_a850;
const BASE_SETUP_GEOMETRY: u32 = 0x00e8_07d0;

/// The rotation's entries and where the matrix keeps each scaled one (the first entry is stored first,
/// with the pass between).
const SCALED_ENTRIES: [(u32, u32); 8] = [
    (3, 0x48),
    (6, 0x4c),
    (1, 0x54),
    (4, 0x58),
    (7, 0x5c),
    (2, 0x64),
    (5, 0x68),
    (8, 0x6c),
];

// Typed from 00baac10
/// `TallGrassShader::SetupGeometry` (slot 0x60), `thiscall(shader, geometry, w2, w3, w4, state, w6,
/// transform, bound)`: the renderer's world transform from `transform` (`00e6fbb0`, the shader
/// family's); the rotation scaled, the translation and 1.0 as a 4x4 matrix in the frame, inverted
/// (`D3DXMatrixInverse`); the up axis (0, 1, 0) through the inverse (`D3DXVec3TransformNormal`) and
/// normalised (`D3DXVec3Normalize`), its x and y the grass's up constant (`01200550`). With the state's
/// property of kind 4: the scene's first light, if any, its direction normalised (`004a0c10`), its two
/// colours (+0xc8 to `01200570`, +0xd4 to `01200520`, each with 1.0) and the negated direction with 1.0
/// (`01200510`); then a shadow pass's set-up (`00baa850`), or with the point lights on and the
/// property's active lights (`00b707d0`) the point lights (`00baa720`, the inverse and the scale). Then
/// the base set-up (`00e807d0`), whose answer this answers. The frame's layout is the exe's (0xd0 bytes:
/// the matrix +0x44, its inverse +0x90, the axis +0x34, its image +0x84, the direction +0x28, the pass
/// +0x40), so the D3DX calls see the same locals.
#[allow(clippy::too_many_arguments)]
pub fn setup_geometry(
    e: &mut Engine,
    this: Ptr,
    geometry: u32,
    word_2: u32,
    word_3: u32,
    word_4: u32,
    state: u32,
    word_6: u32,
    transform: u32,
    bound: u32,
) -> u32 {
    let this = this.addr();
    let renderer = e.global::<u32>(this.wrapping_add(0x18));
    typed_call!(
        shader::FAMILY,
        0x00e6_fbb0,
        accumulator::set_world_transform(e, Ptr::new(renderer), Ptr::new(transform), 0u8)
    );
    e.with_stack(0xd0, |e, frame| {
        let frame = frame.addr();
        let matrix = frame + 0x44;
        let inverse = frame + 0x90;
        let up = frame + 0x34;
        let transformed = frame + 0x84;
        let entry = |e: &Engine, index: u32| {
            f32::from_bits(packet::transform_word(e, transform, 4 * index))
        };
        let scale = f32::from_bits(packet::transform_word(e, transform, 0x30));
        e.set_global(matrix, scale * entry(e, 0));
        let pass = packet::rendering_pass_type(e);
        e.set_global(frame + 0x40, pass);
        for (index, place) in SCALED_ENTRIES {
            let value = entry(e, index) * scale;
            e.set_global(frame + place, value);
        }
        for (k, place) in (0u32..).zip([0x74u32, 0x78, 0x7c]) {
            let value = packet::transform_word(e, transform, 0x24 + 4 * k);
            e.set_global(frame + place, value);
        }
        for place in [0x50u32, 0x60, 0x70] {
            e.set_global(frame + place, 0u32);
        }
        e.set_global(frame + 0x80, ONE);
        thunks::call_matrix_inverse(e, inverse, 0u32, matrix);
        e.set_global(up, 0u32);
        e.set_global(up + 4, ONE);
        e.set_global(up + 8, 0u32);
        thunks::call_vec3_transform_normal(e, transformed, up, inverse);
        thunks::call_vec3_normalize(e, up, transformed);
        let x = e.global::<u32>(up);
        own::set_shader_global(e, UP, x);
        let y = e.global::<u32>(up + 4);
        own::set_shader_global(e, UP + 4, y);

        let property = packet::state_property(e, state, 3);
        if property == 0 || packet::property_kind(e, property) != 4 {
            return;
        }
        // `BSShaderManager::GetAccumulator` (`00b4f5c0`): a read of `011f95ec`.
        let accumulator = packet::shader_accumulator(e);
        let light = packet::first_light(e, packet::accumulator_lights(e, accumulator));
        if light != 0 {
            let direction = frame + 0x28;
            for (k, at) in [(2u32, 0xf8u32), (0, 0xf0), (1, 0xf4)] {
                let word = packet::light_word(e, light, at);
                e.set_global(direction + 4 * k, word);
            }
            typed_call!(
                crate::dispatch::FAMILY,
                0x004a_0c10,
                crate::dispatch::math::normalize(e, Ptr::new(direction))
            );
            for (source, target) in [(0xc8u32, LIGHT_COLOUR), (0xd4, LIGHT_AMBIENT)] {
                // own: the targets are written through set_shader_global
                for k in 0..3u32 {
                    let value = packet::light_word(e, light, source + 4 * k);
                    own::set_shader_global(e, target + 4 * k, value);
                }
                own::set_shader_global(e, target + 0xc, ONE);
            }
            for k in 0..3u32 {
                let value = e.global::<u32>(direction + 4 * k) ^ 0x8000_0000;
                own::set_shader_global(e, LIGHT_DIRECTION + 4 * k, value);
            }
            own::set_shader_global(e, LIGHT_DIRECTION + 0xc, ONE);
        }
        let pass = e.global::<u32>(frame + 0x40);
        if e.call(IS_SHADOW_PASS, &args![pass]).bool() {
            e.call(SHADOW_SETUP, &args![this, word_2, transform]);
        } else if packet::point_lights_on(e) != 0
            && e.call(ACTIVE_NON_SHADOW_LIGHTS, &args![property]).u16() != 0
        {
            let scale = packet::transform_word(e, transform, 0x30);
            e.call(
                UPDATE_POINT_LIGHTS,
                &args![this, property, inverse, scale, pass],
            );
        }
    });
    e.call(
        BASE_SETUP_GEOMETRY,
        &args![this, geometry, word_2, word_3, word_4, state, word_6, transform, bound],
    )
    .u32()
}

/// The shader's slots these reach: its `SetupGeometry` (+0x60, [`setup_geometry`] in the exe's
/// vtable) and the one after the textures (+0x108); a shader property's texture (+0xb4).
const SLOT_SETUP_GEOMETRY: u32 = 0x60;
const SETUP_GEOMETRY: u32 = 0x00ba_ac10;
const SLOT_AFTER_TEXTURES: u32 = 0x108;
const SLOT_PROPERTY_TEXTURE: u32 = 0xb4;
/// `BSRenderedTexture::GetTexture` (`004bc320`, `thiscall(texture, 0)`), by address.
const RENDERED_TEXTURE: u32 = 0x004b_c320;

// Typed from 00bac950
/// The tall grass shader's slot 0x78, `thiscall(shader, state)`: the current pass's stage 0 takes the
/// texture of the state's shader property (its slot 0xb4); in the pass types 0x284 to 0x287 stage 1
/// takes the shadow map of the rendered pass's first scene light (`BSRenderedTexture::GetTexture(0)`
/// of its +0x10c); then the shader's slot 0x108.
pub fn setup_textures(e: &mut Engine, this: Ptr, state: Ptr) {
    let pass_type = packet::rendering_pass_type(e);
    let property = packet::state_property(e, state.addr(), 3);
    let stage = group_c::pass_stage(e, 0);
    let texture = e.vcall(property, SLOT_PROPERTY_TEXTURE, &[]).u32();
    own::set_stage_texture(e, stage, texture);
    if pass_type.wrapping_sub(0x284) <= 3 {
        let stage = group_c::pass_stage(e, 1);
        let light = packet::rendering_first_light(e);
        let shadow_map = packet::scene_light_word(e, light, 0x10c);
        let texture = e.call(RENDERED_TEXTURE, &args![shadow_map, 0u32]).u32();
        own::set_stage_texture(e, stage, texture);
    }
    e.vcall(this.addr(), SLOT_AFTER_TEXTURES, &[]);
}

// Typed from 00baaf40
/// The grass's instances into the shader, `thiscall(shader, unused, property)`: the property's instance
/// data has a 16-bit count of 0x10-byte instances, copied to the shader's array (+0x1a4; `_memcpy`, the
/// CRT family's); the data's geometry's model data gets a 16-bit count through its slot 0xa4: its
/// triangle count (`u16` +0x40) when every instance is active, else the 16-bit product of one
/// instance's triangles and the active count; the active count, read again, goes to the word +0x20
/// of the shader's instance entry (+0x1a8) when there is one.
pub fn copy_instances(e: &mut Engine, this: Ptr, _unused: u32, property: Ptr) {
    let base = this.addr();
    let data = packet::grass_instances(e, property.addr());
    let count = packet::grass_instance_count(e, data);
    let target = e.global::<u32>(base.wrapping_add(0x1a4));
    packet::copy_grass_instances(e, target, data, count);
    let active = packet::grass_active_instances(e, data);
    let model = packet::model_data(e, packet::grass_geometry(e, data));
    let triangles = if active == count {
        packet::data_triangles(e, model)
    } else {
        packet::grass_instance_triangles(e, data).wrapping_mul(active)
    };
    own::set_draw_count(e, model, triangles);
    let entry = e.global::<u32>(base.wrapping_add(0x1a8));
    if entry != 0 {
        let active = packet::grass_active_instances(e, data);
        own::set_shader_global(e, entry.wrapping_add(0x20), u32::from(active));
    }
}

/// The constants [`setup_constants`] fills: the diffuse row (`011fa0c0`, white under full bright), the
/// ambient's copy (`01200560`), the wind phase (`0120055c`), the flags (`01200530`, three floats), the
/// HDR value (`01200580`) and the shadow's four words (`01200590`); the shader's own alpha row (+0x1d0)
/// and phase (+0x11c).
const DIFFUSE: u32 = 0x011f_a0c0;
const AMBIENT: u32 = 0x0120_0560;
const WIND_PHASE: u32 = 0x0120_055c;
const FLAGS: u32 = 0x0120_0530;
const HDR_VALUE: u32 = 0x0120_0580;
const SHADOW: u32 = 0x0120_0590;
const ALPHA_ROW: u32 = 0x1d0;
const PHASE: u32 = 0x11c;
/// `255.0` (`double`, `0101e568`), `3600.0` (`double`, `01012640`) and the wind's cycle (`double`,
/// `0104eea0`, about 2 pi).
const BYTE_SCALE: u32 = 0x0101_e568;
const SECONDS_PER_HOUR: u32 = 0x0101_2640;
const CYCLE: u32 = 0x0104_eea0;

// Typed from 00bac9c0
/// The tall grass shader's slot 0x7c, `thiscall(shader, state)`. With the rendered geometry: the
/// shader's `SetupGeometry` (slot 0x60: the geometry, its skin, 0, its data's buffers, the state, 0,
/// its transform +0x68 and its bound); under full bright a white diffuse row; the ambient copied; the
/// shader's alpha row the alpha property's reference over 255 (then three zeros), or the fog colour
/// without one; the wind phase: the game time in hours times the cycle times the grass's wind speed,
/// taken modulo the cycle (`_CIfmod`) when not below it, kept in the shader (+0x11c) and copied out;
/// the flags (1.0 twice with the property's bit 0x800 of +0x24, else 0.0, then 1.0); the instances
/// ([`copy_instances`]); with HDR its value; at shader level 2 or more with the shadows on and their
/// fade truncated to a nonzero integer, the shadow's words; below level 2 or without HDR the alpha
/// function from the alpha property (bits 10 to 12 of its flags, its reference); last the constant
/// maps' constants ([`group_c::set_map_constants`]). Main thread only: `f32` maths (the steps with a
/// `double` constant in `f64`, rounded once, as [`div_d`]/[`mul_d`]).
pub fn setup_constants(e: &mut Engine, this: Ptr, state: Ptr) {
    let geometry = packet::rendering_geometry(e);
    // The geometry's world transform as the value packet holds it (value packet B).
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
    let state = state.addr();
    let buffers = packet::buffer_data(e, packet::model_data(e, geometry));
    let skin = packet::skin_instance(e, geometry);
    let property = packet::state_property(e, state, 3);
    if crate::slot_target(e, base, SLOT_SETUP_GEOMETRY) == SETUP_GEOMETRY {
        let _: u32 = typed_call!(
            super::FAMILY,
            0x00ba_ac10,
            setup_geometry(e, this, geometry, skin, 0, buffers, state, 0, transform, bound)
        );
    } else {
        e.vcall(
            base,
            SLOT_SETUP_GEOMETRY,
            &args![geometry, skin, 0u32, buffers, state, 0u32, transform, bound],
        );
    }
    if packet::full_bright(e) != 0 {
        for k in 0..4u32 {
            own::set_shader_global(e, DIFFUSE + 4 * k, ONE);
        }
    }
    for k in 0..4u32 {
        let word = packet::ambient_word(e, k);
        own::set_shader_global(e, AMBIENT + 4 * k, word);
    }
    let alpha = packet::state_property(e, state, 0);
    if alpha != 0 {
        let scale: f64 = e.global(BYTE_SCALE);
        let value = div_d(f32::from(packet::alpha_reference(e, alpha)), scale);
        own::set_shader_global(e, base.wrapping_add(ALPHA_ROW), value.to_bits());
        for k in 1..4u32 {
            own::set_shader_global(e, base.wrapping_add(ALPHA_ROW + 4 * k), 0);
        }
    } else {
        for k in 0..4u32 {
            let word = packet::fog_colour_word(e, k);
            own::set_shader_global(e, base.wrapping_add(ALPHA_ROW + 4 * k), word);
        }
    }
    let seconds = f32::from_bits(packet::game_time(e));
    let hours = div_d(seconds, e.global(SECONDS_PER_HOUR));
    let cycle: f64 = e.global(CYCLE);
    let speed = packet::grass_wind_speed(e, packet::grass_instances(e, property));
    let phase = mul_d(hours, cycle) * f32::from_bits(speed);
    own::set_shader_global(e, base.wrapping_add(PHASE), phase.to_bits());
    // `FCOM` with the cycle: below it, or unordered, keeps the phase.
    if f64::from(phase) >= cycle {
        let remainder: f64 = typed_call!(
            crate::crt::FAMILY,
            0x00ec_913a,
            crate::crt::math::ci_fmod(e, f64::from(phase), cycle)
        );
        own::set_shader_global(e, base.wrapping_add(PHASE), narrow(remainder).to_bits());
    }
    let phase = own::shader_word(e, base.wrapping_add(PHASE));
    own::set_shader_global(e, WIND_PHASE, phase);
    let flag = if packet::property_flags_24(e, property) & 0x800 != 0 {
        ONE
    } else {
        0
    };
    own::set_shader_global(e, FLAGS, flag);
    own::set_shader_global(e, FLAGS + 4, flag);
    own::set_shader_global(e, FLAGS + 8, ONE);
    typed_call!(
        super::FAMILY,
        0x00ba_af40,
        copy_instances(e, this, geometry, Ptr::new(property))
    );
    let hdr = packet::hdr_on(e) != 0;
    if hdr {
        let value = packet::hdr_value(e);
        own::set_shader_global(e, HDR_VALUE, value);
    }
    let level = packet::shader_level(e);
    let shadows = packet::shadows_skipped(e) == 0
        && packet::shadow_textures(e) != 0
        && packet::shader_flags_byte(e) & 0x20 != 0;
    if shadows && level >= 2 && group_c::chop_low_word(f32::from_bits(packet::shadow_fade(e))) != 0
    {
        for k in 0..4u32 {
            let word = packet::shadow_word(e, k);
            own::set_shader_global(e, SHADOW + 4 * k, word);
        }
    }
    if level < 2 || !hdr {
        let alpha = packet::state_property(e, state, 0);
        let flags = u32::from(packet::ni_property_flags(e, alpha));
        let reference = u32::from(packet::alpha_reference(e, alpha));
        typed_call!(
            shader::FAMILY,
            0x00b9_7f20,
            shader::state::set_alpha_function(e, (flags >> 10) & 7, reference, 0u32)
        );
    }
    group_c::set_map_constants(
        e,
        base,
        [geometry, skin, 0, buffers, state, 0, transform, bound, 0, 1],
    );
}
