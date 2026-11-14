//! The water shader's set-ups (`WaterShader`, vtable `010af9e0`; Group C of the drawing thread's plan,
//! `measure/typed-draw/GROUP-C.md`): what the world draw reached as translations, the largest of the
//! frame's live reads (the water property and the water system). Live reads go through
//! [`super::packet`], the constants, textures and states they leave through [`super::own`]. Main thread
//! only (the drawing thread replays main's draw with main's control word): `f32` maths. Read in place,
//! not routed: the shader's own words (its renderer +0x18, its constant maps, made at load), the passes'
//! contents and the exe's constants.

use super::{group_c, own, packet};
use crate::shader::{self, accumulator, div_d};
use engine_core::ptr::Ptr;
use engine_core::{args, typed_call, Engine};

/// Bits of 1.0.
const ONE: u32 = 0x3f80_0000;
/// `BSRenderedTexture::GetTexture` (`004bc320`, `thiscall(texture, 0)`), by address; the shader's slot
/// after the textures (+0x108).
const RENDERED_TEXTURE: u32 = 0x004b_c320;
const SLOT_AFTER_TEXTURES: u32 = 0x108;

/// A rendered texture's texture (`BSRenderedTexture::GetTexture(0)`).
fn rendered(e: &mut Engine, texture: u32) -> u32 {
    e.call(RENDERED_TEXTURE, &args![texture, 0u32]).u32()
}

// Typed from 00b863a0
/// The water shader's slot 0x78, `thiscall(shader, state)`: the current pass's first five stages'
/// textures cleared; then, but in the pass types 0x288 and 0x289, with the state's water property:
/// stage 0 the texture of its reflection (+0x13c) when it has one and its byte +0x80 is set, else the
/// default texture; stage 1 the texture of the water's second rendered texture when there is one and
/// the property's byte +0x81 is set; stage 2 the texture of its +0x138, else the default; stage 3 its
/// +0x148 (a texture) when its byte +0x60 is set and that is not null; stage 4 the texture of its
/// +0x144 when it has one and its byte +0x63 is set. Then the shader's slot 0x108.
pub fn setup_textures(e: &mut Engine, this: Ptr, state: Ptr) {
    let pass_type = packet::rendering_pass_type(e);
    let property = packet::state_property(e, state.addr(), 3);
    for k in 0..5u32 {
        let stage = group_c::pass_stage(e, k);
        own::set_stage_texture(e, stage, 0);
    }
    if pass_type == 0x288 || pass_type == 0x289 {
        return;
    }
    let reflection = packet::water_word(e, property, 0x13c);
    let texture = if reflection != 0 && packet::water_byte(e, property, 0x80) != 0 {
        rendered(e, reflection)
    } else {
        packet::default_texture(e)
    };
    own::set_stage_texture(e, group_c::pass_stage(e, 0), texture);
    let second = packet::water_rendered_texture(e);
    if second != 0 && packet::water_byte(e, property, 0x81) != 0 {
        let texture = rendered(e, second);
        own::set_stage_texture(e, group_c::pass_stage(e, 1), texture);
    }
    let third = packet::water_word(e, property, 0x138);
    let texture = if third != 0 {
        rendered(e, third)
    } else {
        packet::default_texture(e)
    };
    own::set_stage_texture(e, group_c::pass_stage(e, 2), texture);
    if packet::water_byte(e, property, 0x60) != 0 {
        let texture = packet::water_word(e, property, 0x148);
        if texture != 0 {
            own::set_stage_texture(e, group_c::pass_stage(e, 3), texture);
        }
    }
    let fifth = packet::water_word(e, property, 0x144);
    if fifth != 0 && packet::water_byte(e, property, 0x63) != 0 {
        let texture = rendered(e, fifth);
        own::set_stage_texture(e, group_c::pass_stage(e, 4), texture);
    }
    e.vcall(this.addr(), SLOT_AFTER_TEXTURES, &[]);
}

/// The water constants (`011ff588`, 0x50 words), the origin's copy (`011ff558`), the four copies of
/// the property's +0x124 (`011ff548`), the shade (`011ff568`, four floats), the light count
/// (`011ff658`), the lights' positions (`011ff668`) and colours (`011ff6e8`), 0x10 bytes each.
const WATER: u32 = 0x011f_f588;
const ORIGIN_COPY: u32 = 0x011f_f558;
const FOUR_COPIES: u32 = 0x011f_f548;
const SHADE: u32 = 0x011f_f568;
const LIGHT_COUNT: u32 = 0x011f_f658;
const LIGHT_COLOURS: u32 = 0x011f_f6e8;
/// The exe's 10.0 (`double`, `01020758`) and 1.0 (`double`, `01012070`).
const TEN: u32 = 0x0102_0758;
const ONE_DOUBLE: u32 = 0x0101_2070;
/// The stencil state setters this calls by address (`cdecl`, the render state's): the stencil test on or
/// off (`00b98070`, `D3DRS_STENCILENABLE`), its function, reference and mask (`00b98180`), its operations
/// (`00b980c0`: fail, z fail, pass) and its write mask (`00b98230`).
const STENCIL_ENABLE: u32 = 0x00b9_8070;
const STENCIL_FUNCTION: u32 = 0x00b9_8180;
const STENCIL_OPERATIONS: u32 = 0x00b9_80c0;
const STENCIL_WRITE_MASK: u32 = 0x00b9_8230;

// Typed from 00b864d0
/// The water shader's slot 0x7c, `thiscall(shader, state)`. With the rendered geometry and the state's
/// water property: the renderer's world transform (`00e6fbb0`, the shader family's); the water
/// constants: the sun less the origin and 0.0, the property's twelve colour words (+0x88), its +0x118
/// and +0x100, its +0x104 over 10.0 (1.0 when below the exe's 1.0), its +0x6c, +0x70 and +0xd0, the
/// water system's value (`011ff108`), the origin (also copied to `011ff558`) and the property's +0xb8 to
/// +0xc4; with the scene's fog property its far distance, far less near, the property's +0x74 and +0x78,
/// the fog's colour and the property's +0x120, and the fog's power; the property's +0xd8 to +0xe4 and the
/// water system's eight words (`011ff398`); its +0x124 four times; the stencil states (the pass types 0x288
/// and 0x289: `00b98230` with its +0x84; else with its byte +0x60: `00b98070(1, 0)`, `00b98180(2, 0xff, +0x84,
/// 0)`, `00b980c0(0, 0, 3 or 0 for 0x29a and 0x2ab, 0)`, `00b98230(0xff, 0)`); its byte +0x82 as a float four
/// times; its light count as a float; for each light of its list the position and +0xe0, and the colour
/// times the light's +0xc4 and the property's +0xd4 with 1.0; last the constant maps' constants.
pub fn setup_constants(e: &mut Engine, this: Ptr, state: Ptr) {
    let geometry = packet::rendering_geometry(e);
    // The geometry's world transform as the value packet holds it (value packet B).
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
    let renderer = e.global::<u32>(base.wrapping_add(0x18));
    typed_call!(
        shader::FAMILY,
        0x00e6_fbb0,
        accumulator::set_world_transform(e, Ptr::new(renderer), Ptr::new(world), 0u8)
    );
    for k in 0..3u32 {
        let sun = f32::from_bits(packet::sun_word(e, k));
        let value = sun - f32::from_bits(packet::origin_word(e, k));
        own::set_constant_word(e, WATER + 4 * k, value.to_bits());
    }
    own::set_constant_word(e, WATER + 0xc, 0);
    let property = packet::state_property(e, block, 3);
    for k in 0..12u32 {
        let word = packet::water_word(e, property, 0x88 + 4 * k);
        own::set_constant_word(e, WATER + 0x10 + 4 * k, word);
    }
    let word = packet::water_word(e, property, 0x118);
    own::set_constant_word(e, WATER + 0x40, word);
    let word = packet::water_word(e, property, 0x100);
    own::set_constant_word(e, WATER + 0x48, word);
    let scaled = div_d(
        f32::from_bits(packet::water_word(e, property, 0x104)),
        e.global(TEN),
    );
    // `FCOM` with the double 1.0: below it the value becomes 1.0; equal, above or unordered keeps it.
    let one: f64 = e.global(ONE_DOUBLE);
    let scaled = if f64::from(scaled) < one {
        ONE
    } else {
        scaled.to_bits()
    };
    let origin = [0u32, 1, 2].map(|k| packet::origin_word(e, k));
    own::set_constant_word(e, WATER + 0x4c, scaled);
    for (k, offset) in (0u32..).zip([0x6cu32, 0x70, 0xd0]) {
        let word = packet::water_word(e, property, offset);
        own::set_constant_word(e, WATER + 0x50 + 4 * k, word);
    }
    let value = packet::water_value(e);
    own::set_constant_word(e, WATER + 0x5c, value);
    for (k, word) in (0u32..).zip(origin) {
        own::set_constant_word(e, WATER + 0x60 + 4 * k, word);
        own::set_constant_word(e, ORIGIN_COPY + 4 * k, word);
    }
    for k in 0..4u32 {
        let word = packet::water_word(e, property, 0xb8 + 4 * k);
        own::set_constant_word(e, WATER + 0x70 + 4 * k, word);
    }
    let index = u32::from(packet::fog_index(e));
    let fog: u32 = typed_call!(
        super::FAMILY,
        0x00b5_5520,
        super::passes::fog_property(e, index)
    );
    if fog != 0 {
        let far = packet::fog_field(e, fog, 0x30);
        let colour = [0x20u32, 0x24, 0x28].map(|offset| packet::fog_field(e, fog, offset));
        let power = packet::fog_field(e, fog, 0x60);
        let near = packet::fog_field(e, fog, 0x2c);
        let range = (f32::from_bits(far) - f32::from_bits(near)).to_bits();
        let first = packet::water_word(e, property, 0x74);
        let second = packet::water_word(e, property, 0x78);
        let last = packet::water_word(e, property, 0x120);
        let words = [
            far, range, first, second, colour[0], colour[1], colour[2], last,
        ];
        for (k, word) in (0u32..).zip(words) {
            own::set_constant_word(e, WATER + 0x80 + 4 * k, word);
        }
        own::set_constant_word(e, WATER + 0x44, power);
    }
    for k in 0..4u32 {
        let word = packet::water_word(e, property, 0xd8 + 4 * k);
        own::set_constant_word(e, WATER + 0xa0 + 4 * k, word);
    }
    for k in 0..8u32 {
        let word = packet::water_extra_word(e, k);
        own::set_constant_word(e, WATER + 0xb0 + 4 * k, word);
    }
    let word = packet::water_word(e, property, 0x124);
    for k in 0..4u32 {
        own::set_constant_word(e, FOUR_COPIES + 4 * k, word);
    }
    let stencil = packet::water_word(e, property, 0x84);
    if pass_type == 0x288 || pass_type == 0x289 {
        e.call(STENCIL_WRITE_MASK, &args![stencil, 0u32]);
    } else if packet::water_byte(e, property, 0x60) != 0 {
        e.call(STENCIL_ENABLE, &args![1u32, 0u32]);
        let stencil = packet::water_word(e, property, 0x84);
        e.call(STENCIL_FUNCTION, &args![2u32, 0xffu32, stencil, 0u32]);
        let third = if pass_type == 0x29a || pass_type == 0x2ab {
            0u32
        } else {
            3
        };
        e.call(STENCIL_OPERATIONS, &args![0u32, 0u32, third, 0u32]);
        e.call(STENCIL_WRITE_MASK, &args![0xffu32, 0u32]);
    }
    let shade = f32::from(packet::water_byte(e, property, 0x82)).to_bits();
    for k in 0..4u32 {
        own::set_constant_word(e, SHADE + 4 * k, shade);
    }
    // The count as an unsigned integer (`FILD` and 2^32 added to a negative one): one rounding to a
    // float.
    #[allow(clippy::cast_precision_loss)]
    let count = packet::water_light_count(e, property) as f32;
    own::set_constant_word(e, LIGHT_COUNT, count.to_bits());
    let mut node = packet::water_lights(e, property);
    let mut cursor = LIGHT_COLOURS; // own: written through set_constant_word
    while node != 0 {
        let light = packet::light_node_light(e, node);
        node = packet::light_node_next(e, node);
        if light == 0 {
            continue;
        }
        for (k, offset) in (0u32..).zip([0x8cu32, 0x90, 0x94, 0xe0]) {
            let word = packet::light_value_word(e, light, offset);
            own::set_constant_word(e, cursor - 0x80 + 4 * k, word);
        }
        let scale = f32::from_bits(packet::light_value_word(e, light, 0xc4));
        let red = f32::from_bits(packet::light_value_word(e, light, 0xd4)) * scale;
        let green = f32::from_bits(packet::light_value_word(e, light, 0xd8)) * scale;
        let blue = scale * f32::from_bits(packet::light_value_word(e, light, 0xdc));
        let factor = f32::from_bits(packet::water_word(e, property, 0xd4));
        for (k, value) in (0u32..).zip([red * factor, factor * green, factor * blue]) {
            own::set_constant_word(e, cursor + 4 * k, value.to_bits());
        }
        own::set_constant_word(e, cursor + 0xc, ONE);
        cursor += 0x10;
    }
    group_c::set_map_constants(
        e,
        base,
        [geometry, skin, 0, buffers, block, 0, world, bound, 0, 1],
    );
}

/// Where the water shader's pass choice finds a pass type's pass in the shader (+0x6c to +0x190): the types 0x23f
/// to 0x2ce with a case in the exe's jump table (`00b81ba4` with the bytes `00b81ccc`).
fn water_pass_offset(pass_type: u32) -> Option<u32> {
    Some(match pass_type {
        0x23f => 0xf0,
        0x240 => 0xf4,
        0x288 => 0xf8,
        0x289 => 0x188,
        0x28a => 0xb0,
        0x28b => 0xbc,
        0x28c => 0xb8,
        0x28d => 0xb4,
        0x28e => 0xc0,
        0x28f => 0xcc,
        0x290 => 0xc8,
        0x291 => 0xc4,
        0x292 => 0xd0,
        0x293 => 0xdc,
        0x294 => 0xd8,
        0x295 => 0xd4,
        0x296 => 0xe0,
        0x297 => 0xec,
        0x298 => 0xe8,
        0x299 => 0xe4,
        0x29a => 0x100,
        0x29b => 0x148,
        0x29c => 0x14c,
        0x29d => 0x150,
        0x29e => 0x154,
        0x29f => 0x158,
        0x2a0 => 0x15c,
        0x2a1 => 0x160,
        0x2a2 => 0x164,
        0x2a3 => 0x168,
        0x2a4 => 0x16c,
        0x2a5 => 0x170,
        0x2a6 => 0x174,
        0x2a7 => 0x178,
        0x2a8 => 0x17c,
        0x2a9 => 0x180,
        0x2aa => 0x184,
        0x2ab => 0x190,
        0x2ac => 0x6c,
        0x2ad => 0x78,
        0x2ae => 0x74,
        0x2af => 0x70,
        0x2b0 => 0x8c,
        0x2b1 => 0x98,
        0x2b2 => 0x94,
        0x2b3 => 0x90,
        0x2b4 => 0x7c,
        0x2b5 => 0x80,
        0x2b6 => 0x84,
        0x2b7 => 0x88,
        0x2b8 => 0x9c,
        0x2b9 => 0xa8,
        0x2ba => 0xa4,
        0x2bb => 0xa0,
        0x2bc => 0xfc,
        0x2bd => 0xac,
        0x2be => 0x104,
        0x2bf => 0x110,
        0x2c0 => 0x10c,
        0x2c1 => 0x108,
        0x2c2 => 0x124,
        0x2c3 => 0x130,
        0x2c4 => 0x12c,
        0x2c5 => 0x128,
        0x2c6 => 0x114,
        0x2c7 => 0x118,
        0x2c8 => 0x11c,
        0x2c9 => 0x120,
        0x2ca => 0x134,
        0x2cb => 0x140,
        0x2cc => 0x13c,
        0x2cd => 0x138,
        0x2ce => 0x18c,
        _ => return None,
    })
}

// Typed from 00b81760
/// The water shader's pass choice (slot 0xf0), `thiscall(shader, pass type)` (not a Ghidra function: typed from
/// its bytes): for its pass types the shader's pass for the type ([`water_pass_offset`]) becomes the current pass;
/// other types: nothing.
pub fn select_pass(e: &mut Engine, this: Ptr, pass_type: u32) {
    if let Some(offset) = water_pass_offset(pass_type) {
        let pass = e.global::<u32>(this.addr().wrapping_add(offset));
        own::set_current_pass(e, pass);
    }
}

/// The pass index the water shader's pass constants hand to the pass flags (`00b815e0`): 1 to 0x48 for the types
/// 0x23f to 0x2ce with a case in the exe's jump table (`00b822f0` with the bytes `00b82418`), else 0.
fn water_pass_index(pass_type: u32) -> u32 {
    match pass_type {
        0x23f => 0x21,
        0x240 => 0x22,
        0x288 => 0x23,
        0x289 => 0x47,
        0x28a => 0x11,
        0x28b => 0x14,
        0x28c => 0x13,
        0x28d => 0x12,
        0x28e => 0x15,
        0x28f => 0x18,
        0x290 => 0x17,
        0x291 => 0x16,
        0x292 => 0x19,
        0x293 => 0x1c,
        0x294 => 0x1b,
        0x295 => 0x1a,
        0x296 => 0x1d,
        0x297 => 0x20,
        0x298 => 0x1f,
        0x299 => 0x1e,
        0x29a => 0x25,
        0x29b => 0x37,
        0x29c => 0x38,
        0x29d => 0x39,
        0x29e => 0x3a,
        0x29f => 0x3b,
        0x2a0 => 0x3c,
        0x2a1 => 0x3d,
        0x2a2 => 0x3e,
        0x2a3 => 0x3f,
        0x2a4 => 0x40,
        0x2a5 => 0x41,
        0x2a6 => 0x42,
        0x2a7 => 0x43,
        0x2a8 => 0x44,
        0x2a9 => 0x45,
        0x2aa => 0x46,
        0x2ab => 0x49,
        0x2ad => 0x3,
        0x2ae => 0x2,
        0x2af => 0x1,
        0x2b0 => 0x8,
        0x2b1 => 0xb,
        0x2b2 => 0xa,
        0x2b3 => 0x9,
        0x2b4 => 0x4,
        0x2b5 => 0x5,
        0x2b6 => 0x6,
        0x2b7 => 0x7,
        0x2b8 => 0xc,
        0x2b9 => 0xf,
        0x2ba => 0xe,
        0x2bb => 0xd,
        0x2bc => 0x24,
        0x2bd => 0x10,
        0x2be => 0x26,
        0x2bf => 0x29,
        0x2c0 => 0x28,
        0x2c1 => 0x27,
        0x2c2 => 0x2e,
        0x2c3 => 0x31,
        0x2c4 => 0x30,
        0x2c5 => 0x2f,
        0x2c6 => 0x2a,
        0x2c7 => 0x2b,
        0x2c8 => 0x2c,
        0x2c9 => 0x2d,
        0x2ca => 0x32,
        0x2cb => 0x35,
        0x2cc => 0x34,
        0x2cd => 0x33,
        0x2ce => 0x48,
        _ => 0,
    }
}

// Typed from 00b81d60
/// The water shader's pass constants (slot 0xfc), `thiscall(shader, pass type)` (the shader not read; not a Ghidra
/// function: typed from its bytes): the pass flags (`00b815e0`, this family's) of the type's pass index
/// ([`water_pass_index`]).
pub fn pass_constants(e: &mut Engine, _this: Ptr, pass_type: u32) {
    let index = water_pass_index(pass_type);
    typed_call!(super::FAMILY, 0x00b8_15e0, group_c::pass_flags(e, index));
}

// Typed from 00b86b80
/// The water shader's geometry end (slot 0x8c), `thiscall(shader, state)` (the shader not read; not a Ghidra
/// function): wireframe off (`00b98480(0, 0)`) when the state's water property is set and has bit 0x10000 of its
/// flags +0x24; the stencil test off (`00b98070(0, 0)`) outside the pass types 0x288 and 0x289 when the property's
/// byte +0x60 is set (read with no null check, as the exe).
pub fn geometry_done(e: &mut Engine, _this: Ptr, state: Ptr) {
    let property = packet::state_property(e, state.addr(), 3);
    let pass_type = packet::rendering_pass_type(e);
    if property != 0 && packet::property_flags_24(e, property) & 0x1_0000 != 0 {
        typed_call!(
            shader::FAMILY,
            0x00b9_8480,
            crate::shader::state::set_fill_mode(e, 0u8, 0)
        );
    }
    if pass_type != 0x288 && pass_type != 0x289 && e.global::<u8>(property.wrapping_add(0x60)) != 0
    {
        typed_call!(
            shader::FAMILY,
            0x00b9_8070,
            crate::shader::state::set_stencil_enable(e, 0u8, 0)
        );
    }
}
