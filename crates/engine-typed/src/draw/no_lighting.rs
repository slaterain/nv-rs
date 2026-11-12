//! The no-lighting shader's set-ups (`BSShaderNoLighting`, vtable `010bb230`; Group C of the drawing
//! thread's plan, `measure/typed-draw/GROUP-C.md`) and the helpers they call: what the world draw
//! reached as translations. Live reads go through [`super::packet`], the constants and states they
//! leave through [`super::own`]. Main thread only (the drawing thread replays main's draw with main's
//! control word): `f32` maths. Read in place, not routed: the shader's own words (its renderer +0x18,
//! its passes and programs, its entries and constant maps, made at load) and the exe's constants.

use super::{group_c, own, packet};
use crate::shader::{self, accumulator, state};
use engine_core::ptr::Ptr;
use engine_core::{args, typed_call, Engine};

/// Bits of 1.0.
const ONE: u32 = 0x3f80_0000;

// Typed from 00bc6970
/// The no-lighting shader's pass choice (slot 0xf0), `thiscall(shader, pass type)`: for its pass types
/// (0x42 to 0x57, 0x2dc, and 0x2dd, 0x2e2, 0x2e3, 0x2e8, 0x2e9, 0x2ee, 0x2ef: the exe's jump tables) the
/// shader's pass for the type becomes the current pass, and its counted pointer +0x28 is set to the
/// type's declaration (+0x184 to +0x1b0; `00a5f860`) or, for 0x52 to 0x57, released and cleared. Other
/// types: nothing.
pub fn select_pass(e: &mut Engine, this: Ptr, pass_type: u32) {
    let Some((pass, declaration)) = pass_words(pass_type) else {
        return;
    };
    let shader = this.addr();
    let word = e.global::<u32>(shader.wrapping_add(pass));
    own::set_current_pass(e, word);
    match declaration {
        Some(offset) => {
            let object = e.global::<u32>(shader.wrapping_add(offset));
            own::set_counted(e, shader.wrapping_add(0x28), object);
        }
        None => own::clear_counted(e, shader.wrapping_add(0x28)),
    }
}

/// Where [`select_pass`] finds a pass type's pass and declaration in the shader, as the exe's jump
/// tables (`00bc6d50` for 0x42 to 0x57; the bytes `00bc6dc8` into `00bc6da8` for 0x2dd to 0x2ef) lead.
fn pass_words(pass_type: u32) -> Option<(u32, Option<u32>)> {
    let words = match pass_type {
        0x42 => (0x6c, Some(0x184)),
        0x43 => (0x74, Some(0x188)),
        0x44 => (0x70, Some(0x18c)),
        0x45 => (0x78, Some(0x184)),
        0x46 => (0x8c, Some(0x19c)),
        0x47 => (0x94, Some(0x1a0)),
        0x48 => (0x90, Some(0x1a4)),
        0x49 => (0x98, Some(0x19c)),
        0x4a => (0x7c, Some(0x190)),
        0x4b => (0x84, Some(0x194)),
        0x4c => (0x80, Some(0x198)),
        0x4d => (0x88, Some(0x190)),
        0x4e => (0x9c, Some(0x1a8)),
        0x4f => (0xa4, Some(0x1ac)),
        0x50 => (0xa0, Some(0x1b0)),
        0x51 => (0xa8, Some(0x1a8)),
        0x52 => (0xac, None),
        0x53 => (0xb0, None),
        0x54 => (0xb4, None),
        0x55 => (0xb8, None),
        0x56 => (0xbc, None),
        0x57 => (0xc0, None),
        0x2dc => (0xc4, Some(0x184)),
        0x2dd => (0xd4, Some(0x19c)),
        0x2e2 => (0xc8, Some(0x184)),
        0x2e3 => (0xd8, Some(0x19c)),
        0x2e8 => (0xcc, Some(0x188)),
        0x2e9 => (0xdc, Some(0x1a0)),
        0x2ee => (0xd0, Some(0x18c)),
        0x2ef => (0xe0, Some(0x1a4)),
        _ => return None,
    };
    Some(words)
}

/// The fog constants the set-ups fill (`01202148`, eight words) and the far distance used when a fog
/// property's two distances are 0 (`010ad6e0`, the exe's).
const FOG: u32 = 0x0120_2148;
const FOG_FAR_DEFAULT: u32 = 0x010a_d6e0;

// Typed from 00bc6de0
/// The no-lighting shader's slot 0xfc, `thiscall(shader, pass type)`: its entry +0x170 switched on for
/// the pass types 0x52 to 0x57, its entry +0x178 for 0x4a to 0x51, 0x56 and 0x57 (each off otherwise);
/// then with the scene's fog property (`GetFogProperty`, the draw family's): when its far (+0x30) and
/// near (+0x2c) distances are both 0.0 (a NaN is not), the fog constants are the default far distance,
/// 0.0, its power (+0x60), 0.0 and the fog colour (`011a9bd0`); otherwise its far distance, far less
/// near, its power, 0.0, its colour (+0x20 to +0x28) and 1.0.
pub fn setup_fog(e: &mut Engine, this: Ptr, pass_type: u32) {
    let shader = this.addr();
    let skinned = e.global::<u32>(shader.wrapping_add(0x170));
    own::set_entry_enabled(e, skinned, pass_type.wrapping_sub(0x52) <= 5);
    let other = e.global::<u32>(shader.wrapping_add(0x178));
    let on = (0x4a..=0x51).contains(&pass_type) || pass_type == 0x56 || pass_type == 0x57;
    own::set_entry_enabled(e, other, on);
    let index = u32::from(packet::fog_index(e));
    let fog: u32 = typed_call!(
        super::FAMILY,
        0x00b5_5520,
        super::passes::fog_property(e, index)
    );
    if fog == 0 {
        return;
    }
    let near = packet::fog_field(e, fog, 0x2c);
    let colour = [0x20u32, 0x24, 0x28].map(|offset| packet::fog_field(e, fog, offset));
    let far = packet::fog_field(e, fog, 0x30);
    let power = packet::fog_field(e, fog, 0x60);
    let (far_value, near_value) = (f32::from_bits(far), f32::from_bits(near));
    let words = if far_value == 0.0 && near_value == 0.0 {
        let default = e.global::<u32>(FOG_FAR_DEFAULT);
        let c = [0u32, 1, 2, 3].map(|k| packet::fog_colour_word(e, k));
        [default, 0, power, 0, c[0], c[1], c[2], c[3]]
    } else {
        let range = (far_value - near_value).to_bits();
        [far, range, power, 0, colour[0], colour[1], colour[2], ONE]
    };
    for (k, word) in (0u32..).zip(words) {
        own::set_constant_word(e, FOG + 4 * k, word);
    }
}

/// The sub-texture constants (`01202048`, 0x10 bytes each).
const SUBTEXTURES: u32 = 0x0120_2048;

// Typed from 00bc6fa0
/// The sub-textures copied, `thiscall(shader, geometry, unread word)`: the geometry's model data has a
/// byte count of 0x10-byte entries; when not 0, they are copied to `01202048` (`_memcpy`, the CRT
/// family's) and the count stored in the word +0x20 of the shader's entry +0x170.
pub fn copy_subtextures(e: &mut Engine, this: Ptr, geometry: Ptr, _unused: u32) {
    let data = packet::model_data(e, geometry.addr());
    let count = packet::subtexture_count(e, data);
    if count != 0 {
        let source = packet::subtextures(e, data);
        own::copy_constants(e, SUBTEXTURES, source, u32::from(count) << 4);
        let entry = e.global::<u32>(this.addr().wrapping_add(0x170));
        own::set_shader_global(e, entry.wrapping_add(0x20), u32::from(count));
    }
}

/// The material constants: the colour (`01202010`, three floats) and the alpha (`0120201c`); the
/// `double` 0.1 below which a colour is scaled (`0101ffa0`, the exe's).
const MATERIAL_COLOUR: u32 = 0x0120_2010;
const MATERIAL_ALPHA: u32 = 0x0120_201c;
const TENTH: u32 = 0x0101_ffa0;

// Typed from 00bc6fe0
/// The material constants, `thiscall(shader, property, material)` (the shader not read): the alpha
/// the property's (+0x28) when below 1.0, else 1.0; the material's colour (its colour pointer +0x34, or
/// its own colour +0x28), times its scale (+0x40) when HDR is on or all three are below 0.1, each
/// product a float; the replacement colour (`011a9b7c`) instead when it then equals the neutral colour
/// (`011f4998`).
pub fn material_constants(e: &mut Engine, _this: Ptr, property: Ptr, material: Ptr) {
    let alpha = f32::from_bits(packet::property_alpha(e, property.addr()));
    let stored = if 1.0 > alpha { alpha.to_bits() } else { ONE };
    own::set_constant_word(e, MATERIAL_ALPHA, stored);
    let material = material.addr();
    let mut source = packet::material_colour(e, material);
    if source == 0 {
        source = material.wrapping_add(0x28);
    }
    let mut colour = [0u32, 1, 2].map(|k| f32::from_bits(packet::colour_word(e, source, k)));
    let scale = packet::hdr_on(e) != 0 || {
        let tenth: f64 = e.global(TENTH);
        colour.iter().all(|c| tenth > f64::from(*c))
    };
    if scale {
        let factor = f32::from_bits(packet::material_scale(e, material));
        colour = colour.map(|c| c * factor);
    }
    let neutral = [0u32, 1, 2].map(|k| f32::from_bits(packet::neutral_colour_word(e, k)));
    if neutral == colour {
        colour = [0u32, 1, 2].map(|k| f32::from_bits(packet::replacement_colour_word(e, k)));
    }
    for (k, c) in (0u32..).zip(colour) {
        own::set_constant_word(e, MATERIAL_COLOUR + 4 * k, c.to_bits());
    }
}

// Typed from 00bc7100
/// The light fog constants, `thiscall(shader)`: for the pass types 0x2d8 to 0x2ef, the shader's entry
/// +0x174 switched on and the fog constants the fog's far distance, three zeros, the fog plane and its
/// dot product with the sun's vector (in the exe's order) less the plane's offset.
pub fn light_fog(e: &mut Engine, this: Ptr) {
    let pass_type = packet::rendering_pass_type(e);
    if pass_type.wrapping_sub(0x2d8) > 0x17 {
        return;
    }
    let entry = e.global::<u32>(this.addr().wrapping_add(0x174));
    own::set_entry_enabled(e, entry, true);
    let sun = [0u32, 1, 2].map(|k| f32::from_bits(packet::sun_word(e, k)));
    let plane = [0u32, 1, 2].map(|k| packet::fog_plane_word(e, k));
    let far = packet::fog_far(e);
    let offset = f32::from_bits(packet::fog_plane_offset(e));
    let c = plane.map(f32::from_bits);
    let dot = (c[0] * sun[0] + c[1] * sun[1]) + c[2] * sun[2];
    let words = [
        far,
        0,
        0,
        0,
        plane[0],
        plane[1],
        plane[2],
        (dot - offset).to_bits(),
    ];
    for (k, word) in (0u32..).zip(words) {
        own::set_constant_word(e, FOG + 4 * k, word);
    }
}

/// The texture matrix constants (`01201fe0`: three rows of three floats, 0x10 bytes apart).
const TEXTURE_MATRIX: u32 = 0x0120_1fe0;

// Typed from 00bc8e00
/// The texture matrix, `thiscall(shader, state)` (the shader not read): the identity, unless the
/// state's texturing property (its sixth) has a first map with a transform: then, the transform
/// updated first when it is marked changed (`00a88cd0`), its nine floats (+0x20). Its rows go to
/// `01201fe0`, `01201ff0` and `01202000`.
pub fn texture_matrix(e: &mut Engine, _this: Ptr, state: Ptr) {
    let mut matrix = [ONE, 0, 0, 0, ONE, 0, 0, 0, ONE];
    let texturing = packet::state_property(e, state.addr(), 5);
    if texturing != 0 {
        let map = packet::first_texture_map(e, texturing);
        if map != 0 {
            let transform = packet::map_transform(e, map);
            if transform != 0 {
                if packet::texture_transform_changed(e, transform) != 0 {
                    own::update_texture_transform(e, transform);
                }
                for (k, word) in (0u32..).zip(matrix.iter_mut()) {
                    *word = packet::texture_transform_word(e, transform, k);
                }
            }
        }
    }
    for (k, word) in (0u32..).zip(matrix) {
        let at = TEXTURE_MATRIX + 0x10 * (k / 3) + 4 * (k % 3);
        own::set_constant_word(e, at, word);
    }
}

// Typed from 00bc8d40
/// The no-lighting shader's slot 0x78, `thiscall(shader, state)` (the shader not read): with the
/// current pass's stage 0, the state's shader property's texture (its slot 0xb4), when it has one, on
/// the stage ([`group_c::set_stage`]).
pub fn setup_textures(e: &mut Engine, _this: Ptr, state: Ptr) {
    let stage = group_c::pass_stage(e, 0);
    if stage == 0 {
        return;
    }
    let property = packet::state_property(e, state.addr(), 3);
    let texture = e.vcall(property, 0xb4, &[]).u32();
    if texture != 0 {
        group_c::set_stage(e, stage, texture, property);
    }
}

/// The falloff constants (`01202030`, four words) and the alpha flags (`01202020`, two floats);
/// `NiTransform::MakeIdentity` (`00a86bf0`, by address).
const FALLOFF: u32 = 0x0120_2030;
const ALPHA_FLAGS: u32 = 0x0120_2020;
const MAKE_IDENTITY: u32 = 0x00a8_6bf0;

// Typed from 00bc9a50
/// `BSShaderNoLighting::SetupGeometryConstants` (slot 0x7c), `thiscall(shader, state)`. With the
/// rendered geometry: the renderer's world transform (`00e6fbb0`, the shader family's) from its
/// transform (+0x68), or with a skin from an identity (`NiTransform::MakeIdentity` in a 0x34-byte frame)
/// with the skinned origin's translation; for the pass types 0x52 to 0x57 the sub-textures; the
/// texture matrix, the light fog and the material constants (this family's); the falloff the
/// property's +0x70 to +0x7c, or zeros for the pass types 0x56 and 0x57 without its bit 0x40; the vertex
/// constant map's constants, the alpha flags (1.0 in the first for the alpha property's flags field
/// 0, in the second for 2), the pixel map's ([`group_c::map_constants`]); z writes off without the
/// property's bit 0 of +0x24 and the z test off without bit 31 of +0x20 (the shader family's); last,
/// with the state's fifth property's flag 0, the shader's +0x1c object's slot 0x24 with it.
pub fn setup_constants(e: &mut Engine, this: Ptr, state: Ptr) {
    let base = this.addr();
    let block = state.addr();
    let pass_type = packet::rendering_pass_type(e);
    let geometry = packet::rendering_geometry(e);
    let bound = packet::bound_of(e, geometry);
    let buffers = packet::buffer_data(e, packet::model_data(e, geometry));
    let property = packet::state_property(e, block, 3);
    let skin = packet::skin_instance(e, geometry);
    let material = packet::state_property(e, block, 2);
    e.with_stack(0x34, |e, frame| {
        let frame = frame.addr();
        let transform = if skin != 0 {
            e.call(MAKE_IDENTITY, &args![frame]);
            for k in 0..3u32 {
                let word = packet::origin_word(e, k);
                e.set_global(frame + 0x24 + 4 * k, word);
            }
            frame
        } else {
            geometry.wrapping_add(0x68)
        };
        let renderer = e.global::<u32>(base.wrapping_add(0x18));
        typed_call!(
            shader::FAMILY,
            0x00e6_fbb0,
            accumulator::set_world_transform(e, Ptr::new(renderer), Ptr::new(transform), 0u8)
        );
    });
    if pass_type.wrapping_sub(0x52) <= 5 {
        typed_call!(
            super::FAMILY,
            0x00bc_6fa0,
            copy_subtextures(e, this, Ptr::new(geometry), geometry.wrapping_add(0x68))
        );
    }
    typed_call!(super::FAMILY, 0x00bc_8e00, texture_matrix(e, this, state));
    typed_call!(super::FAMILY, 0x00bc_7100, light_fog(e, this));
    typed_call!(
        super::FAMILY,
        0x00bc_6fe0,
        material_constants(e, this, Ptr::new(property), Ptr::new(material))
    );
    let falloff =
        if pass_type.wrapping_sub(0x56) <= 1 && packet::property_flags(e, property) & 0x40 == 0 {
            [0u32; 4]
        } else {
            [0u32, 1, 2, 3].map(|k| packet::property_falloff_word(e, property, k))
        };
    for (k, word) in (0u32..).zip(falloff) {
        own::set_constant_word(e, FALLOFF + 4 * k, word);
    }
    let words = [
        geometry,
        skin,
        0,
        buffers,
        block,
        0,
        geometry.wrapping_add(0x68),
        bound,
        0,
        1,
    ];
    group_c::map_constants(e, base, 0x34, 0x5c, words);
    own::set_constant_word(e, ALPHA_FLAGS, 0);
    own::set_constant_word(e, ALPHA_FLAGS + 4, 0);
    let alpha = packet::state_property(e, block, 0);
    if alpha != 0 {
        match (u32::from(packet::ni_property_flags(e, alpha)) >> 5) & 0xf {
            0 => own::set_constant_word(e, ALPHA_FLAGS, ONE),
            2 => own::set_constant_word(e, ALPHA_FLAGS + 4, ONE),
            _ => {}
        }
    }
    group_c::map_constants(e, base, 0x30, 0x44, words);
    if packet::property_flags_24(e, property) & 1 == 0 {
        typed_call!(
            shader::FAMILY,
            0x00b9_7e30,
            state::set_z_write_enable(e, 0u8, 0u32)
        );
    }
    if packet::property_flags(e, property) & 0x8000_0000 == 0 {
        typed_call!(
            shader::FAMILY,
            0x00b9_7de0,
            state::set_z_enable(e, 0u8, 0u32)
        );
    }
    let other = packet::state_property(e, block, 4);
    if other != 0 && packet::ni_property_flags(e, other) & 1 != 0 {
        let target = e.global::<u32>(base.wrapping_add(0x1c));
        e.vcall(target, 0x24, &args![other]);
    }
}
