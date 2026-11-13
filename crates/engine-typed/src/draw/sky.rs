//! The sky shader's set-ups (`SkyShader`, vtable `010afe18`; Group C of the drawing thread's plan,
//! `measure/typed-draw/GROUP-C.md`): what the world draw reached as translations. Live reads go
//! through [`super::packet`], the constants they leave through [`super::own`]. Main thread only (the
//! drawing thread replays main's draw with main's control word): `f32` maths. Read in place, not
//! routed: the shader's own words (its renderer +0x18, its constant maps, its +0x1c object, made at
//! load) and the exe's constants.

use super::{group_c, own, packet};
use crate::shader::{self, accumulator, div_d, state};
use engine_core::ptr::Ptr;
use engine_core::{args, typed_call, Engine};

/// Bits of 1.0.
const ONE: u32 = 0x3f80_0000;

// Typed from 00b89cc0
/// The sky shader's slot 0x78, `thiscall(shader, state)` (the shader not read): with the state's
/// shader property, its first texture, when set, on the current pass's stage 0; stage 1, when there is
/// one, takes its second texture, or without one in the pass types 0x2d4, 0x23c, 0x23a, 0x2d2, 0x23d and
/// 0x23e its first (when set). Each stage as [`group_c::set_stage`] leaves it.
pub fn setup_textures(e: &mut Engine, _this: Ptr, state: Ptr) {
    let property = packet::state_property(e, state.addr(), 3);
    let first = packet::sky_first_texture(e, property);
    if first != 0 {
        let stage = group_c::pass_stage(e, 0);
        group_c::set_stage(e, stage, first, property);
    }
    let mut texture = packet::sky_second_texture(e, property);
    let pass_type = packet::rendering_pass_type(e);
    if texture == 0 {
        if !matches!(pass_type, 0x2d4 | 0x23c | 0x23a | 0x2d2 | 0x23d | 0x23e) {
            return;
        }
        texture = packet::sky_first_texture(e, property);
        if texture == 0 {
            return;
        }
    }
    let stage = group_c::pass_stage(e, 1);
    if stage != 0 {
        group_c::set_stage(e, stage, texture, property);
    }
}

/// The constants [`setup_constants`] fills: the eye position (`011ff8c8`, four floats) and the colours
/// (`011ff910`, twelve words); the shader's own +0xe4, +0xe8 and +0xec. The exe's 7.0
/// (`double`, `01015a40`). The held accumulator (what its getter `00b4f5d0` answers) is read through
/// [`packet::held_accumulator`].
const EYE: u32 = 0x011f_f8c8;
const COLOURS: u32 = 0x011f_f910;
const SEVEN: u32 = 0x0101_5a40;

// Typed from 00b89d80
/// The sky shader's slot 0x7c, `thiscall(shader, state)`. With the rendered geometry: the renderer's
/// world transform (`00e6fbb0`, the shader family's) from its transform, or, while the held
/// accumulator skips type 13 (+0x164), from a copy in a 0x34-byte frame whose translation is moved by
/// minus the scene camera's (`011f95d8`) and plus the accumulator's camera's when it has one. The eye
/// position the sun less the origin, then 0.0 (in pass type 5 its z minus the sky's height over 7.0).
/// For the property's kind 3 the shader's +0xe8 and +0xe4 take the property's value (+0x84) and the
/// blend value its index (`u16` +0x88) picks, other kinds +0xe8 0.0; with HDR, +0xec the sky's HDR
/// value. The colours by kind (the exe's switch): 0, 1, 3, 5, 6 and 7 the property's four (+0x60), 2 the
/// twelve sky colours (`011ff8e0`), others 0.0, 0.0, 0.0, 1.0 and six more 0.0 (the eighth word left).
/// Then the constant maps' constants ([`group_c::set_map_constants`]), the blend (6, 0) for the kinds 0
/// and 6 (the shader family's), and the shader's +0x1c object's slot 0x24 with the state's fifth
/// property when its flag 0 is set.
pub fn setup_constants(e: &mut Engine, this: Ptr, state: Ptr) {
    let geometry = packet::rendering_geometry(e);
    // The geometry's world transform as the value packet holds it (value packet B): the helpers handed it
    // read the packet's image while the packet is open, the geometry's own otherwise.
    packet::with_drawn(e, geometry, |e, world, bound| {
        constants_with(e, this, state, geometry, world, bound);
    });
}

/// [`setup_constants`] with the transform and the bound (`world`: the geometry's +0x68, `bound`: its bound; or the
/// packet's images of them).
fn constants_with(e: &mut Engine, this: Ptr, state: Ptr, geometry: u32, world: u32, bound: u32) {
    let base = this.addr();
    let block = state.addr();
    let pass_type = packet::rendering_pass_type(e);
    let buffers = packet::buffer_data(e, packet::model_data(e, geometry));
    let skin = packet::skin_instance(e, geometry);
    let property = packet::state_property(e, block, 3);
    let held = packet::held_accumulator(e);
    if packet::skips_type13(e, held) != 0 {
        e.with_stack(0x34, |e, copy| {
            let copy = copy.addr();
            packet::copy_world_transform(e, geometry, copy);
            // The held accumulator's camera is the cut's: its translation is the packet's (value packet B), and the
            // scene camera's too when it is that camera (another camera read in place).
            let camera = packet::scene_camera(e);
            let held = packet::held_accumulator(e);
            let object = packet::camera(e, Ptr::new(held));
            let mut moved = [0f32; 3];
            for (k, value) in (0u32..).zip(moved.iter_mut()) {
                let own = f32::from_bits(e.global::<u32>(copy + 0x24 + 4 * k));
                let translation = if camera == object {
                    packet::camera_translation_word(e, camera, k)
                } else {
                    packet::world_translation_word(e, camera, k)
                };
                *value = own - f32::from_bits(translation);
            }
            if object != 0 {
                for (k, value) in (0u32..).zip(moved.iter_mut()) {
                    *value += f32::from_bits(packet::camera_translation_word(e, object, k));
                }
            }
            for (k, value) in (0u32..).zip(moved) {
                e.set_global(copy + 0x24 + 4 * k, value.to_bits());
            }
            let renderer = e.global::<u32>(base.wrapping_add(0x18));
            typed_call!(
                shader::FAMILY,
                0x00e6_fbb0,
                accumulator::set_world_transform(e, Ptr::new(renderer), Ptr::new(copy), 0u8)
            );
        });
    } else {
        let renderer = e.global::<u32>(base.wrapping_add(0x18));
        typed_call!(
            shader::FAMILY,
            0x00e6_fbb0,
            accumulator::set_world_transform(e, Ptr::new(renderer), Ptr::new(world), 0u8)
        );
    }
    for k in 0..3u32 {
        let sun = f32::from_bits(packet::sun_word(e, k));
        let eye = sun - f32::from_bits(packet::origin_word(e, k));
        own::set_constant_word(e, EYE + 4 * k, eye.to_bits());
    }
    own::set_constant_word(e, EYE + 0xc, 0);
    if pass_type == 5 {
        let height = f32::from_bits(packet::sky_height(e));
        let z = div_d(-height, e.global(SEVEN));
        own::set_constant_word(e, EYE + 8, z.to_bits());
    }
    if packet::sky_kind(e, property) == 3 {
        let value = packet::sky_value(e, property);
        own::set_shader_global(e, base.wrapping_add(0xe8), value);
        let index = u32::from(packet::sky_blend_index(e, property));
        let blend = packet::sky_blend_value(e, index);
        own::set_shader_global(e, base.wrapping_add(0xe4), blend);
    } else {
        own::set_shader_global(e, base.wrapping_add(0xe8), 0);
    }
    if packet::hdr_on(e) != 0 {
        let value = packet::sky_hdr_value(e);
        own::set_shader_global(e, base.wrapping_add(0xec), value);
    }
    match packet::sky_kind(e, property) {
        0 | 1 | 3 | 5 | 6 | 7 => {
            for k in 0..4u32 {
                let word = packet::sky_colour_word(e, property, k);
                own::set_constant_word(e, COLOURS + 4 * k, word);
            }
        }
        2 => {
            for k in 0..12u32 {
                let word = packet::sky_colours_word(e, k);
                own::set_constant_word(e, COLOURS + 4 * k, word);
            }
        }
        _ => {
            for (k, word) in [
                (0u32, 0u32),
                (1, 0),
                (2, 0),
                (3, ONE),
                (4, 0),
                (5, 0),
                (6, 0),
            ] {
                own::set_constant_word(e, COLOURS + 4 * k, word);
            }
            for k in [8u32, 9, 10] {
                own::set_constant_word(e, COLOURS + 4 * k, 0);
            }
        }
    }
    group_c::set_map_constants(
        e,
        base,
        [geometry, skin, 0, buffers, block, 0, world, bound, 0, 1],
    );
    let kind = packet::sky_kind(e, property);
    if kind == 0 || kind == 6 {
        typed_call!(
            shader::FAMILY,
            0x00b9_7ff0,
            state::set_blend(e, 6u32, 0u32, 0u32)
        );
    }
    let other = packet::state_property(e, block, 4);
    if other != 0 && packet::ni_property_flags(e, other) & 1 != 0 {
        let target = e.global::<u32>(base.wrapping_add(0x1c));
        e.vcall(target, 0x24, &args![other]);
    }
}
