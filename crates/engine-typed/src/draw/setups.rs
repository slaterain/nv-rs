//! The shadow light shader's pass set-ups of the four pass type groups with their own (group B): the
//! two `begin_pass` runs for the types 0xfc to 0x102 and 0xca to 0xd1 ([`begin_types_fc`],
//! [`begin_types_ca`]) and the three per-geometry set-ups the standard draw runs for 0xca, 0xd1 and 0x102
//! ([`setup_type_ca`], [`setup_type_d1`], [`setup_type_102`]); the plain geometry pass (0xfc, 0xfd) is
//! the render family's. Each fills the current pass's stages from the geometry's property and sets the
//! world-view constants.

use super::capture;
use super::{own, packet, passes, FAMILY};
use crate::render::pass::property_clamp_mode;
use crate::shader::{self, accumulator, state};
use engine_core::ptr::Ptr;
use engine_core::x87::{
    x87_add, x87_div, x87_load32, x87_m32, x87_quiet_bits, x87_store32, x87_sub,
};
use engine_core::{args, typed_call, Engine};

/// The two switch objects whose word +0x20 the first begin sets (`011febf0`, `011fec38`).
const SWITCH_A: u32 = 0x011f_ebf0;
const SWITCH_B: u32 = 0x011f_ec38;
/// The word stage 3 takes when the geometry answers none (`011f951c`).
const DEFAULT_STAGE_3: u32 = 0x011f_951c;
/// The point the passes transform (`011fa2a0`) and 1.0 (a double, `01012070`).
const ORIGIN_POINT: u32 = 0x011f_a2a0;
const ONE_DOUBLE: u32 = 0x0101_2070;
/// The shader's own set-ups these call: the fog upload, the pixel entries, the ambient and the light colour
/// (`00b7c3a0`, `00b7c510`, `00b7c580`, `00b7c600`, `thiscall(shader)`: this file's) and by address `00b7c850` (`stdcall(matrix, scale)`), `00b7cb00` (`stdcall(matrix, scale,
/// word)`); `D3DXVec3TransformCoord` (`00ee6df8`) and `D3DXMatrixTranspose` (`00ee6dd4`) are [`super::thunks`].
const SCALED_LIGHT: u32 = 0x00b7_c850;
const SCALED_LIGHT_WORD: u32 = 0x00b7_cb00;

/// An array index as a word (0..4 here).
#[inline(always)]
#[allow(clippy::cast_possible_truncation)]
fn idx(i: usize) -> u32 {
    i as u32
}

/// The current pass's stage `index`, read afresh each time.
#[inline(always)]
fn pass_stage(e: &Engine, index: u32) -> u32 {
    let pass = packet::current_pass(e);
    let stages = e.global::<u32>(pass.wrapping_add(0x24));
    e.global::<u32>(stages.wrapping_add(index.wrapping_mul(4)))
}

/// Stage `stage`'s address modes from the property's clamp mode.
#[inline(always)]
fn stage_modes(e: &mut Engine, stage: u32, property: u32) {
    let mode = property_clamp_mode(e, property);
    typed_call!(
        shader::FAMILY,
        0x00e7_de90,
        state::set_address_modes(e, Ptr::new(stage), mode)
    );
}

/// Stage 0 the word at the property's `[+0xac]`, stage 1 the property's texture 0, each with the
/// property's address modes.
fn property_stages(e: &mut Engine, property: u32) {
    let source = e.global::<u32>(property.wrapping_add(0xac));
    let stage0 = pass_stage(e, 0);
    let texture = e.global::<u32>(source);
    own::set_stage_texture(e, stage0, texture);
    stage_modes(e, stage0, property);
    let stage1 = pass_stage(e, 1);
    let texture: u32 = typed_call!(
        shader::FAMILY,
        0x00c0_3230,
        accumulator::texture_by_index(e, Ptr::new(property), 0u32)
    );
    own::set_stage_texture(e, stage1, texture);
    stage_modes(e, stage1, property);
}

/// Stage `index` bound (the render family's `00e7eb00`, not forced).
#[inline(always)]
fn bind_stage(e: &mut Engine, index: u32) {
    let stage = pass_stage(e, index);
    let _: bool = typed_call!(
        crate::render::FAMILY,
        0x00e7_eb00,
        crate::render::stage::bind(e, Ptr::new(stage), 0u8)
    );
}

/// The rendered geometry's world transform (+0x68) inverted into `out` and the shader's world-view
/// constants set from it; answers the geometry (the first word of the pass being rendered).
fn view_matrix(e: &mut Engine, shader: u32, out: u32) -> u32 {
    let object = packet::rendering_geometry(e);
    // The transform as the value packet holds it ([`super::capture::image`]); the unused word stays the object.
    let (from, to) = super::cut::WORLD;
    super::capture::image(e, object, from, to, |e, image| {
        let matrix = image.wrapping_add(0x68);
        typed_call!(
            crate::dispatch::FAMILY,
            0x00c4_c2d0,
            crate::dispatch::math::inverse_4x4(e, Ptr::new(matrix), 0u32, Ptr::new(out))
        );
        typed_call!(
            shader::FAMILY,
            0x00b7_c750,
            accumulator::set_world_view_constants(e, shader, object, matrix)
        );
    });
    object
}

/// The shader's object at +0x30 gets the pass's word +0x44 through its slot 0x78 with six zeros, the
/// locals `big` (0x34 bytes) and `small` (0x10), 0 and 1: only with both.
fn upload_last(e: &mut Engine, shader: u32, big: u32, small: u32) {
    let object = own::constant_map(e, shader, 0x30);
    if object == 0 {
        return;
    }
    let pass = packet::current_pass(e);
    let word = e.global::<u32>(pass.wrapping_add(0x44));
    if word == 0 {
        return;
    }
    e.vcall(
        object,
        0x78,
        &args![word, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, big, small, 0u32, 1u32],
    );
}

/// The vertex shader constant 0x10 (one register) from `vector` on the pass's device.
#[inline(always)]
fn light_constant(e: &mut Engine, vector: u32) {
    let device = packet::pass_device(e);
    own::device_vertex_constants(e, device, 0x10, vector, 1);
}

// Typed from 00b7dd50
/// The begin of the types 0xfc to 0x102, `thiscall(shader)`: stage 5 bound, the fog constants, the
/// two switches' word +0x20 set, the three set-ups after them, and the last upload with a frame of 0x44
/// bytes (the small local at +0, the big at +0x10).
pub fn begin_types_fc(e: &mut Engine, this: Ptr) {
    let shader = this.addr();
    bind_stage(e, 5);
    fog_upload_call(e, shader);
    let first = capture::own_at::<u32>(e, SWITCH_A);
    own::set_switch_word(e, first, 1);
    let second = capture::own_at::<u32>(e, SWITCH_B);
    own::set_switch_word(e, second, 1);
    light_setups_call(e, shader);
    e.with_stack(0x44, |e, frame| {
        let frame = frame.addr();
        upload_last(e, shader, frame + 0x10, frame);
    });
}

// Typed from 00b7deb0
/// The begin of the types 0xca to 0xd1, `thiscall(shader)`: the fog constants and the three set-ups
/// after them.
pub fn begin_types_ca(e: &mut Engine, this: Ptr) {
    let shader = this.addr();
    fog_upload_call(e, shader);
    light_setups_call(e, shader);
}

// Typed from 00b7ded0
/// The set-up of type 0xca, `thiscall(shader, property state)`, the property `[state + 0xc]`: the property's
/// stages, both bound, the view matrix into a local (frame +0x10 of 0x84 bytes), the scaled light
/// (`00b7c850`, the geometry's world scale +0x98) and the last upload (big +0x50, small +0).
pub fn setup_type_ca(e: &mut Engine, this: Ptr, state: Ptr) {
    let shader = this.addr();
    let property = packet::state_property(e, state.addr(), 3);
    property_stages(e, property);
    bind_stage(e, 0);
    bind_stage(e, 1);
    e.with_stack(0x84, |e, frame| {
        let frame = frame.addr();
        let local = frame + 0x10;
        let object = view_matrix(e, shader, local);
        let scale = x87_quiet_bits(packet::bone_scale(e, object));
        e.call(SCALED_LIGHT, &args![local, scale]);
        upload_last(e, shader, frame + 0x50, frame);
    });
}

// Typed from 00b7dfe0
/// The set-up of type 0xd1, `thiscall(shader, property state)`: as [`setup_type_ca`] up to the view matrix
/// (frame +0x14 of 0x88 bytes), then the scaled light with the state block's word +8 (`00b7cb00`); the rendered pass's
/// byte +9 less 1.0 as the fourth float of the point `011fa2a0` transformed by the local, which goes to
/// the vertex constant 0x10; the last upload (big +0x54, small +4).
pub fn setup_type_d1(e: &mut Engine, this: Ptr, state: Ptr) {
    let shader = this.addr();
    let property = packet::state_property(e, state.addr(), 3);
    property_stages(e, property);
    bind_stage(e, 0);
    bind_stage(e, 1);
    e.with_stack(0x88, |e, frame| {
        let frame = frame.addr();
        let local = frame + 0x14;
        let holder = packet::rendering_pass(e);
        let object = view_matrix(e, shader, local);
        let scale = x87_quiet_bits(packet::bone_scale(e, object));
        let word = packet::state_property(e, state.addr(), 2);
        e.call(SCALED_LIGHT_WORD, &args![local, scale, word]);
        let byte = packet::rendering_pass_byte9(e, holder);
        let one: f64 = e.global(ONE_DOUBLE);
        let w = x87_store32(x87_sub(f64::from(byte), one));
        e.set_global(frame, w);
        let vector = frame + 4;
        capture::own_image(e, ORIGIN_POINT, 0xc, |e, origin| {
            crate::draw::thunks::call_vec3_transform_coord(e, vector, origin, local);
        });
        let w = x87_quiet_bits(e.global::<u32>(frame));
        e.set_global(frame + 0x10, w);
        light_constant(e, vector);
        upload_last(e, shader, frame + 0x54, frame + 4);
    });
}

// Typed from 00b7e150
/// The set-up of type 0x102, `thiscall(shader, property state)`: the property's stages; stage 3 the property's
/// slot 0xf4 answer for (3, 0) (asked again when not 0) or `011f951c`'s word; stages 0, 1 and 3 bound;
/// the view matrix (frame +0x10 of 0x50 bytes) with the light direction; the geometry's world scale as the
/// fourth float of the transformed point, the vertex constant 0x10.
pub fn setup_type_102(e: &mut Engine, this: Ptr, state: Ptr) {
    let shader = this.addr();
    let property = packet::state_property(e, state.addr(), 3);
    property_stages(e, property);
    let stage3 = pass_stage(e, 3);
    let value = if e.vcall(property, 0xf4, &args![3u32, 0u32]).u32() != 0 {
        e.vcall(property, 0xf4, &args![3u32, 0u32]).u32()
    } else {
        e.global::<u32>(DEFAULT_STAGE_3)
    };
    own::set_stage_texture(e, stage3, value);
    bind_stage(e, 0);
    bind_stage(e, 1);
    bind_stage(e, 3);
    e.with_stack(0x50, |e, frame| {
        let frame = frame.addr();
        let local = frame + 0x10;
        let object = view_matrix(e, shader, local);
        typed_call!(
            shader::FAMILY,
            0x00b7_c7b0,
            accumulator::set_light_direction(e, local)
        );
        // The float is kept in the dead argument slot over the call.
        let w = x87_quiet_bits(packet::bone_scale(e, object));
        capture::own_image(e, ORIGIN_POINT, 0xc, |e, origin| {
            crate::draw::thunks::call_vec3_transform_coord(e, frame, origin, local);
        });
        e.set_global(frame + 0xc, w);
        light_constant(e, frame);
    });
}

/// The two pass masks (`011fb4c0`, `011fa8e0`, a word a pass) and the two constant switch lists
/// (`011febd8..011fec30` and `011fec30..011fec88`, objects with an enabled byte at +8).
const MASKS_FIRST: u32 = 0x011f_b4c0;
const MASKS_SECOND: u32 = 0x011f_a8e0;
const FIRST_LIST: (u32, u32) = (0x011f_ebd8, 0x011f_ec30);
const SECOND_LIST: (u32, u32) = (0x011f_ec30, 0x011f_ec88);

/// Each switch of `from..to` that is on and whose bit (from bit 1, two switches a step) is set in the
/// mask word at `mask` (read for each) is turned off.
fn apply_mask(e: &mut Engine, mask: u32, from: u32, to: u32) {
    let mut bit: u32 = 1;
    let mut at = from + 4;
    while at < to {
        let first = capture::own_at::<u32>(e, at - 4);
        if first != 0 && own::entry_is_enabled(e, first) {
            let on = capture::own_at::<u32>(e, mask) & bit.rotate_left(1) == 0;
            own::set_entry_enabled(e, first, on);
        }
        let second = capture::own_at::<u32>(e, at);
        bit = bit.rotate_left(1);
        if second != 0 && own::entry_is_enabled(e, second) {
            let on = capture::own_at::<u32>(e, mask) & bit.rotate_left(1) == 0;
            own::set_entry_enabled(e, second, on);
        }
        at += 8;
        bit = bit.rotate_left(1);
    }
}

// Typed from 00b7ac20
/// The tail of the pass kinds 1 to 5 (`thiscall(shader, pass index)`): with a first-mask word for the
/// pass, the first switch list follows it and the vertex constant map (+0x34) is sorted (its slot 0x24);
/// with a second-mask word, the second list and the pixel constant map (+0x30).
pub fn tail_kinds_1_to_5(e: &mut Engine, this: Ptr, pass: u32) {
    let first = MASKS_FIRST.wrapping_add(pass.wrapping_mul(4)); // own: read through own_at
    if capture::own_at::<u32>(e, first) != 0 {
        apply_mask(e, first, FIRST_LIST.0, FIRST_LIST.1);
        let map = own::constant_map(e, this.addr(), 0x34);
        e.vcall(map, 0x24, &args![]);
    }
    let second = MASKS_SECOND.wrapping_add(pass.wrapping_mul(4)); // own: read through own_at
    if capture::own_at::<u32>(e, second) != 0 {
        apply_mask(e, second, SECOND_LIST.0, SECOND_LIST.1);
        let map = own::constant_map(e, this.addr(), 0x30);
        e.vcall(map, 0x24, &args![]);
    }
}

/// The constant entries of the vertex (`011febd8`) and pixel (`011fec30`) maps, 22 each, and the pass
/// types' enable words for them (`011fcc80`, `011fc0a0`; entry k: bit k + 1).
const VERTEX_ENTRIES: u32 = 0x011f_ebd8;
const PIXEL_ENTRIES: u32 = 0x011f_ec30;
const VERTEX_FLAGS: u32 = 0x011f_cc80;
const PIXEL_FLAGS: u32 = 0x011f_c0a0;
/// The fog constants (`011fa280`), the far distance with no fog distances (`010ad6e0`) and the fog
/// colour's three copies (`011fd9b8`); the fog colour and the fog index byte come through [`packet`].
const FOG: u32 = 0x011f_a280;
const FAR_DISTANCE: u32 = 0x010a_d6e0;
const FOG_COLOURS: u32 = 0x011f_d9b8;

/// The 22 entries at `entries` enabled by the pass type's word of `flags` (entry k by bit k + 1, the
/// word read for each).
fn enable_entries(e: &mut Engine, entries: u32, flags: u32, pass_type: u32) {
    let mut mask = 1u32;
    for index in 0..22u32 {
        let entry = capture::own_at::<u32>(e, entries + 4 * index);
        if entry != 0 {
            let word = capture::own_at::<u32>(e, flags.wrapping_add(pass_type.wrapping_mul(4)));
            let enabled = word & mask.rotate_left(1) != 0;
            own::set_entry_enabled(e, entry, enabled);
        }
        mask = mask.rotate_left(1);
    }
}

/// Whether the entry the pointer at `slot` holds is enabled (+8).
#[inline(always)]
fn entry_enabled(e: &Engine, slot: u32) -> bool {
    let entry = capture::own_at::<u32>(e, slot);
    own::entry_is_enabled(e, entry)
}

/// The fog constants (`011fa280`): with a fog property, (500000.0, 0, 0, 0) and the fog colour when its
/// far (+0x30) and near (+0x2c) distances are both 0, else (far, far - near, its +0x60, 0) and its
/// colour (+0x20, +0x24, +0x28, 0). Each float copied through x87 (a signalling NaN quieted).
fn fog_constants(e: &mut Engine) {
    let index = u32::from(packet::fog_index(e));
    let fog: u32 = typed_call!(FAMILY, 0x00b5_5520, passes::fog_property(e, index));
    if fog == 0 {
        return;
    }
    let near = x87_quiet_bits(packet::fog_field(e, fog, 0x2c));
    let far = x87_quiet_bits(packet::fog_field(e, fog, 0x30));
    let power = x87_quiet_bits(packet::fog_field(e, fog, 0x60));
    let far_value = x87_load32(far);
    let near_value = x87_load32(near);
    if far_value == 0.0 && near_value == 0.0 {
        let distance = x87_quiet_bits(e.global::<u32>(FAR_DISTANCE));
        own::set_constant_word(e, FOG, distance);
        for offset in [4u32, 8, 0xc] {
            own::set_constant_word(e, FOG + offset, 0u32);
        }
        let colour: [u32; 4] = std::array::from_fn(|i| packet::fog_colour_word(e, idx(i)));
        own::set_constant_word(e, FOG + 0x10, colour[0]);
        own::set_constant_word(e, FOG + 0x1c, colour[3]);
        own::set_constant_word(e, FOG + 0x18, colour[2]);
        own::set_constant_word(e, FOG + 0x14, colour[1]);
    } else {
        let red = packet::fog_field(e, fog, 0x20);
        let green = packet::fog_field(e, fog, 0x24);
        let span = x87_store32(x87_sub(far_value, near_value));
        let blue = packet::fog_field(e, fog, 0x28);
        own::set_constant_word(e, FOG, far);
        own::set_constant_word(e, FOG + 4, span);
        own::set_constant_word(e, FOG + 8, x87_quiet_bits(power));
        own::set_constant_word(e, FOG + 0xc, 0u32);
        own::set_constant_word(e, FOG + 0x10, x87_quiet_bits(red));
        own::set_constant_word(e, FOG + 0x1c, 0u32);
        own::set_constant_word(e, FOG + 0x18, x87_quiet_bits(blue));
        own::set_constant_word(e, FOG + 0x14, x87_quiet_bits(green));
    }
}

// Typed from 00b7b450
/// `ShadowLightShader::SetupPassConstants` (engine map; a shader's slot 0xfc), `thiscall(shader, pass
/// type)`: nothing for 0x250 and 0x251. The vertex and pixel entries are enabled by the type's words,
/// the vertex (+0x34) and pixel (+0x30) constant maps sorted (their slot 0x24); with vertex entry 8 the
/// fog constants; when the rendered geometry has a skin instance (+0xbc), the renderer's (+0x18) view
/// matrix (+0xa00) transposed into a local and set as vertex constants 1 to 4; with vertex entry 8 the fog
/// colour copied three times.
pub fn setup_pass_constants(e: &mut Engine, this: Ptr, pass_type: u32) {
    if pass_type == 0x250 || pass_type == 0x251 {
        return;
    }
    let base = this.addr();
    enable_entries(e, VERTEX_ENTRIES, VERTEX_FLAGS, pass_type);
    enable_entries(e, PIXEL_ENTRIES, PIXEL_FLAGS, pass_type);
    let vertex_map = own::constant_map(e, base, 0x34);
    e.vcall(vertex_map, 0x24, &args![]);
    let pixel_map = own::constant_map(e, base, 0x30);
    e.vcall(pixel_map, 0x24, &args![]);
    if entry_enabled(e, VERTEX_ENTRIES + 4 * 8) {
        fog_constants(e);
    }
    let geometry = packet::rendering_geometry(e);
    if packet::skin_instance(e, geometry) != 0 {
        let renderer = e.global::<u32>(base.wrapping_add(0x18));
        e.with_stack(0x40, |e, matrix| {
            crate::draw::thunks::call_matrix_transpose(e, matrix, renderer.wrapping_add(0xa00));
            let device = packet::pass_device(e);
            own::device_vertex_constants(e, device, 1, matrix.addr(), 4);
        });
    }
    if entry_enabled(e, VERTEX_ENTRIES + 4 * 8) {
        let colour: [u32; 4] = std::array::from_fn(|i| packet::fog_colour_word(e, idx(i)));
        for copy in 0..3u32 {
            for (index, word) in (0u32..).zip(colour) {
                own::set_constant_word(e, FOG_COLOURS + 0x10 * copy + 4 * index, word);
            }
        }
    }
}

/// The pass table (`011fdff8`, a pass for each pass type).
const PASSES: u32 = 0x011f_dff8;

/// Where the pass begin takes the shader's counted pointer +0x28 from for a pass type: `None` for
/// +0x70 (the exe's inline assignment), else the word at that offset (through `00a5f860`). The exe's
/// four switch tables (byte maps at `00b7b144`, `00b7b24c`, `00b7b2a4` and `00b7b390`) as match arms.
fn first_source(pass_type: u32) -> Option<u32> {
    match pass_type {
        0x3
        | 0x5
        | 0x8
        | 0x12..=0x13
        | 0x15
        | 0x5c..=0x5f
        | 0x6f..=0x79
        | 0x7d
        | 0x89..=0x92
        | 0x9b..=0x9d
        | 0xa6..=0xa8
        | 0xb1..=0xb3
        | 0xbc..=0xbe
        | 0xc6..=0xc7
        | 0xcd..=0xce
        | 0xd4..=0xd5
        | 0xdc..=0xdd
        | 0xe5..=0xe6
        | 0xee..=0xef
        | 0xf7..=0xf8
        | 0x105..=0x106
        | 0x109
        | 0x111..=0x112
        | 0x115
        | 0x11c..=0x11d
        | 0x120
        | 0x127..=0x128
        | 0x12b
        | 0x12e..=0x12f
        | 0x135..=0x136
        | 0x13b..=0x13c
        | 0x141..=0x142
        | 0x147..=0x148
        | 0x14d
        | 0x14f..=0x150
        | 0x152
        | 0x156..=0x157
        | 0x15d..=0x15e
        | 0x163..=0x164
        | 0x169..=0x16a
        | 0x16f..=0x170
        | 0x175..=0x176
        | 0x1e3
        | 0x1e6
        | 0x1e8
        | 0x1eb
        | 0x1ed
        | 0x232
        | 0x246..=0x247
        | 0x249
        | 0x24f
        | 0x251
        | 0x254..=0x255
        | 0x2d7
        | 0x2d9
        | 0x2df
        | 0x2e5
        | 0x2eb => None,
        0x61 | 0xfe | 0x10b | 0x1e9 => Some(0x74),
        0x93..=0x94 | 0x1ef..=0x230 | 0x233 => Some(0x78),
        _ => Some(0x6c),
    }
}

// Typed from 00b7af80
/// The pass begin's choice (a shader's slot 0xf0), `thiscall(shader, pass type)`: the type's pass
/// (the table `011fdff8`) becomes the current pass; the shader's counted pointer +0x28 is set from the
/// word +0x70, +0x74, +0x78 or +0x6c by the type ([`first_source`]); for 0x250 and 0x251 the pixel and
/// vertex constant maps (+0x30, +0x34) are set from +0x7c and +0x80 (the exe's inline assignment, the
/// source read again for the store), for every other type from +0x84 and +0x88 through `00c663d0`.
pub fn select_pass(e: &mut Engine, this: Ptr, pass_type: u32) {
    let shader = this.addr();
    let pass = capture::own_at::<u32>(e, PASSES.wrapping_add(pass_type.wrapping_mul(4)));
    own::set_current_pass(e, pass);
    match first_source(pass_type) {
        None => own::assign_counted(e, shader + 0x28, shader + 0x70, false),
        Some(offset) => {
            let object = e.global::<u32>(shader.wrapping_add(offset));
            own::set_counted(e, shader + 0x28, object);
        }
    }
    if pass_type == 0x250 || pass_type == 0x251 {
        own::assign_counted(e, shader + 0x30, shader + 0x7c, true);
        own::assign_counted(e, shader + 0x34, shader + 0x80, true);
    } else {
        own::copy_counted(e, shader + 0x30, shader + 0x84);
        own::copy_counted(e, shader + 0x34, shader + 0x88);
    }
}

/// The land toggles (`011fa2d0`, four floats) and the property's count slot (0xec, 16 bits); the
/// defaults' second word is the fog colour's green (`011a9bd4`, through [`packet`]).
const LAND_TOGGLES: u32 = 0x011f_a2d0;
const PROPERTY_COUNT: u32 = 0xec;

// Typed from 00c02c30
/// `ShadowLightShader::UpdateTogglesConstantLand` (Xbox PDB), `thiscall(shader, unread word, flag,
/// shader property, index)`: the land toggles 1.0; the default's second word, or 1.0 with `flag`; the
/// byte `index` of the property's table (+0xc4) as a float; the property's count (its slot 0xec) plus
/// 1.0 (the toggles set-up passes the shader property, the translation calls it the geometry).
/// Stored second, first, third, fourth, as the exe does.
pub fn land_toggles(
    e: &mut Engine,
    _this: Ptr,
    _unused: u32,
    flag: u32,
    property: u32,
    index: u32,
) {
    let second = if flag != 0 {
        x87_store32(1.0)
    } else {
        packet::fog_colour_word(e, 1)
    };
    let table = packet::land_table(e, property);
    let byte = e.mem.u8(table.wrapping_add(index));
    let third = x87_store32(f64::from(byte));
    let count = e.vcall(property, PROPERTY_COUNT, &args![]).u16();
    own::set_constant_word(e, LAND_TOGGLES + 4, second);
    own::set_constant_word(e, LAND_TOGGLES, x87_store32(1.0));
    own::set_constant_word(e, LAND_TOGGLES + 8, third);
    let one: f64 = e.global(ONE_DOUBLE);
    let fourth = x87_store32(x87_add(f64::from(count), one));
    own::set_constant_word(e, LAND_TOGGLES + 0xc, fourth);
}

/// The lighting properties' fade distances (`011f9454` near, `011f9458` far).
const FADE_NEAR: u32 = 0x011f_9454;
const FADE_FAR: u32 = 0x011f_9458;

/// The fade at `distance` between `near` and `far`: 1.0 when `near` is above the distance (ordered),
/// 0.0 when the distance is at least `far`, else `1 − min(ratio, 1)` with `ratio = (distance − near) /
/// (far − near)` stored as a float (the clamped 1.0 stored too, then subtracted from 1.0).
fn fade_between(distance: u32, near: u32, far: u32) -> f32 {
    let (distance_value, near_value, far_value) = (
        f32::from_bits(distance),
        f32::from_bits(near),
        f32::from_bits(far),
    );
    if near_value > distance_value {
        return 1.0;
    }
    if distance_value >= far_value {
        return 0.0;
    }
    let ratio = x87_store32(x87_div(
        x87_sub(x87_load32(distance), x87_load32(near)),
        x87_sub(x87_load32(far), x87_load32(near)),
    ));
    if 1.0 < f32::from_bits(ratio) {
        return f32::from_bits(x87_store32(x87_sub(1.0, 1.0)));
    }
    f32::from_bits(x87_store32(x87_sub(1.0, x87_m32(ratio))))
}

// Typed from 00b66b80
/// A lighting property's fade (its vtable's, unnamed in the engine map), `thiscall(property)`, a float
/// in `ST0`: 1.0 unless bit 0 of its flags (+0x20) is set, bit 0x800000 of +0x24 clear and the far
/// distance (`011f9458`) above 0 (ordered); then the fade between `011f9454` and `011f9458` at its
/// camera distance (+0x34).
pub fn property_fade(e: &mut Engine, this: Ptr) -> f32 {
    let base = this.addr();
    let far = e.global::<u32>(FADE_FAR);
    let enabled = packet::property_flags(e, base) & 1 != 0
        && packet::property_flags_24(e, base) & 0x0080_0000 == 0
        && 0.0 < f32::from_bits(far);
    if !enabled {
        return 1.0;
    }
    let near = e.global::<u32>(FADE_NEAR);
    fade_between(packet::property_distance(e, base), near, far)
}

/// The fog upload ([`fog_upload`], `00b7c3a0`) as the type groups' begins call it.
fn fog_upload_call(e: &mut Engine, shader: u32) {
    typed_call!(FAMILY, 0x00b7_c3a0, fog_upload(e, Ptr::new(shader)));
}

/// The three set-ups after the fog ([`pixel_entries`], [`light_ambient`], [`light_colour`]) as the type groups'
/// begins call them, in the exe's order.
fn light_setups_call(e: &mut Engine, shader: u32) {
    let _: u32 = typed_call!(FAMILY, 0x00b7_c510, pixel_entries(e, Ptr::new(shader)));
    typed_call!(FAMILY, 0x00b7_c580, light_ambient(e, Ptr::new(shader)));
    typed_call!(FAMILY, 0x00b7_c600, light_colour(e, Ptr::new(shader)));
}

// Typed from 00b7c3a0
/// The shadow light shader's fog upload, `thiscall(shader)` (the shader not read): the fog constants (`011fa280`, as
/// [`fog_constants`] makes them from the fog property of the fog index, both through the packet), then the current
/// pass's device (its constants holder +0x5c, that holder's +0x20) gets the eight floats as vertex constants 0xe and
/// 0xf (its slot 0x178; an image of the draw's own copy while the packet owns them).
pub fn fog_upload(e: &mut Engine, _this: Ptr) {
    fog_constants(e);
    let device = packet::pass_device(e);
    own::device_vertex_constants(e, device, 0xe, FOG, 2); // own: uploaded through own_image
}

// Typed from 00b7c510
/// The shadow light shader's pixel entries, `thiscall(shader)`: each of the 22 pixel entries (`011fec30`) that is
/// set is switched on when its bit (entry k: bit k + 1) is set in the rendered pass type's word of `011fc0a0` (read
/// for each), then the pixel constant map (+0x30) is sorted (its slot 0x24, a tail call: its answer is this one's).
pub fn pixel_entries(e: &mut Engine, this: Ptr) -> u32 {
    let pass_type = packet::rendering_pass_type(e);
    enable_entries(e, PIXEL_ENTRIES, PIXEL_FLAGS, pass_type);
    let map = own::constant_map(e, this.addr(), 0x30);
    e.vcall(map, 0x24, &args![]).u32()
}

/// The ambient row (`011fa0c0`) and the light colour row (`011fa0d0`), four floats each.
const AMBIENT_ROW: u32 = 0x011f_a0c0;
const LIGHT_ROW: u32 = 0x011f_a0d0;
/// The HDR switch (`011f941e`), the byte that keeps the extra scale off (`011f9427`) and the extra scale
/// (`011f9190`).
const HDR_ON: u32 = 0x011f_941e;
const NO_EXTRA_SCALE: u32 = 0x011f_9427;
const EXTRA_SCALE: u32 = 0x011f_9190;

// Typed from 00b7c580
/// The shadow light shader's ambient, `thiscall(shader)` (the shader not read): the rendered pass's first light's
/// light (+0xf8) gives its ambient colour (+0xc8, three floats) with 1.0 to `011fa0c0`, through the packet.
pub fn light_ambient(e: &mut Engine, _this: Ptr) {
    let shadow = packet::rendering_first_light(e);
    let light = packet::pass_light_word(e, shadow, 0xf8);
    for k in 0..3u32 {
        // Through an `FLD` and `FSTP` in the exe: a signalling NaN quieted.
        let word = x87_quiet_bits(packet::light_value_word(e, light, 0xc8 + 4 * k));
        own::set_constant_word(e, AMBIENT_ROW + 4 * k, word);
    }
    own::set_constant_word(e, AMBIENT_ROW + 0xc, 1.0f32.to_bits());
}

// Typed from 00b7c600
/// The shadow light shader's light colour, `thiscall(shader)` (the shader not read): the rendered pass's first light
/// (its weight +0xd0, its alpha +0xd4) and that light's light (+0xf8: its strength +0xc4, held to 1.0 unless HDR, and
/// colour +0xd4): each component the colour times the strength, then times the weight (the exe's order), times the extra scale with HDR on and `011f9427` clear, to `011fa0d0` with the alpha; through
/// the packet. Main thread only: `f32` maths.
#[allow(clippy::neg_cmp_op_on_partial_ord)] // NaN-aware as the exe: held only when above 1.0.
pub fn light_colour(e: &mut Engine, _this: Ptr) {
    let shadow = packet::rendering_first_light(e);
    let light = packet::pass_light_word(e, shadow, 0xf8);
    let value = |e: &Engine, at: u32| f32::from_bits(packet::light_value_word(e, light, at));
    let mut strength = value(e, 0xc4);
    let hdr = e.global::<u8>(HDR_ON) != 0; // own: main's frame value, value packet B
    if !hdr && 1.0 < strength {
        strength = 1.0;
    }
    let weight = f32::from_bits(packet::pass_light_word(e, shadow, 0xd0));
    let mut colour = [
        value(e, 0xd4) * strength * weight,
        weight * (value(e, 0xd8) * strength),
        weight * (strength * value(e, 0xdc)),
    ];
    if hdr && e.global::<u8>(NO_EXTRA_SCALE) == 0 {
        // own: main's frame value, value packet B
        let extra = e.global::<f32>(EXTRA_SCALE); // own: main's frame value, value packet B
        colour = colour.map(|c| extra * c);
    }
    for (k, c) in (0u32..).zip(colour) {
        own::set_constant_word(e, LIGHT_ROW + 4 * k, c.to_bits());
    }
    // Through an `FLD` and `FSTP` in the exe: a signalling NaN quieted.
    let alpha = x87_quiet_bits(packet::pass_light_word(e, shadow, 0xd4));
    own::set_constant_word(e, LIGHT_ROW + 0xc, alpha);
}

/// The land values `00b7ad80` fills from a shader property's +0xdc block (`011fd8f4` to `011fd910`, `011fd85c`,
/// `011fd860`, `011fd914`, 1.0 at `011fd918`) and the fog values (`011fd924` to `011fd940`).
const LAND_WORDS: u32 = 0x011f_d8f4;
const LAND_VALUE_A: u32 = 0x011f_d85c;
const LAND_VALUE_B: u32 = 0x011f_d860;
const LAND_VALUE_C: u32 = 0x011f_d914;
const LAND_ONE: u32 = 0x011f_d918;
const LAND_FOG_COLOUR: u32 = 0x011f_d924;
const LAND_FOG_FAR: u32 = 0x011f_d934;
const LAND_FOG_RANGE: u32 = 0x011f_d938;
const LAND_FOG_POWER: u32 = 0x011f_d93c;
const LAND_FOG_LAST: u32 = 0x011f_d940;

// Typed from 00b7ad80
/// The shadow light shader's land values, `thiscall(shader, word, property)` (the shader not read): with a property,
/// its +0xdc block (when set) gives the words +0xc to +0x28 to `011fd8f4`, the floats +0x4c, +0x50 and +0x54 to
/// `011fd85c`, `011fd860` and `011fd914`, and 1.0 to `011fd918`; the fog values `011fd924` to `011fd940` are cleared;
/// with the word set and the fog property of the fog index (both through the packet) whose far (+0x30) or near (+0x2c)
/// distance is not 0.0 (a NaN counts as not equal), they become its colour (+0x20 to +0x28, then 0.0), the far
/// distance, far less near, its +0x60 and 0.0. Main thread only: `f32` maths.
pub fn land_values(e: &mut Engine, _this: Ptr, enabled: u32, property: u32) {
    if property == 0 {
        return;
    }
    let block = e.global::<u32>(property.wrapping_add(0xdc)); // own: main's object read in place (set at load)
    if block != 0 {
        for k in 0..8u32 {
            let word = e.global::<u32>(block.wrapping_add(0xc + 4 * k)); // own: main's object read in place (set at load)
            own::set_constant_word(e, LAND_WORDS + 4 * k, word);
        }
        // The three floats through an `FLD` and `FSTP` in the exe (a signalling NaN quieted); the words above plain.
        let word = x87_quiet_bits(e.global::<u32>(block.wrapping_add(0x4c))); // own: main's object read in place (set at load)
        own::set_constant_word(e, LAND_VALUE_A, word);
        let word = x87_quiet_bits(e.global::<u32>(block.wrapping_add(0x50))); // own: main's object read in place (set at load)
        own::set_constant_word(e, LAND_VALUE_B, word);
        let word = x87_quiet_bits(e.global::<u32>(block.wrapping_add(0x54))); // own: main's object read in place (set at load)
        own::set_constant_word(e, LAND_VALUE_C, word);
        own::set_constant_word(e, LAND_ONE, 1.0f32.to_bits());
    }
    for k in 0..8u32 {
        own::set_constant_word(e, LAND_FOG_COLOUR + 4 * k, 0);
    }
    if enabled == 0 {
        return;
    }
    let index = u32::from(packet::fog_index(e));
    let fog: u32 = typed_call!(FAMILY, 0x00b5_5520, passes::fog_property(e, index));
    if fog == 0 {
        return;
    }
    // Each through an `FLD` and `FSTP` in the exe (a signalling NaN quieted), the colour too.
    let near = x87_quiet_bits(packet::fog_field(e, fog, 0x2c));
    let far = x87_quiet_bits(packet::fog_field(e, fog, 0x30));
    let power = x87_quiet_bits(packet::fog_field(e, fog, 0x60));
    let (far_value, near_value) = (f32::from_bits(far), f32::from_bits(near));
    if far_value == 0.0 && near_value == 0.0 {
        return;
    }
    let range = far_value - near_value;
    let colour = [0x20u32, 0x24, 0x28].map(|at| x87_quiet_bits(packet::fog_field(e, fog, at)));
    own::set_constant_word(e, LAND_FOG_FAR, far);
    own::set_constant_word(e, LAND_FOG_RANGE, range.to_bits());
    own::set_constant_word(e, LAND_FOG_POWER, power);
    own::set_constant_word(e, LAND_FOG_LAST, 0);
    for (k, word) in (0u32..).zip(colour) {
        own::set_constant_word(e, LAND_FOG_COLOUR + 4 * k, word);
    }
    own::set_constant_word(e, LAND_FOG_COLOUR + 0xc, 0);
}

// ---- The shadow light shader's subclasses' own slots (vtables `010bb7a8` and `010bb980`). ----

/// The pair of textures the stage setters `00b7c150`/`00b7c170` put into a stage (`011f95e4`, `011f95e8`: main's,
/// set before the draw) and the word a stage takes when a property answers no texture (`011f951c`).
const PAIR_FIRST: u32 = 0x011f_95e4;
const PAIR_SECOND: u32 = 0x011f_95e8;
const NO_TEXTURE: u32 = 0x011f_951c;

// Typed from 00b7c150
/// `thiscall(shader, stage)` (the shader not read): the current pass's stage takes the pair's first texture.
pub fn stage_pair_first(e: &mut Engine, _this: Ptr, stage: u32) {
    let at = pass_stage(e, stage);
    let texture = e.global::<u32>(PAIR_FIRST);
    own::set_stage_texture(e, at, texture);
}

// Typed from 00b7c170
/// `thiscall(shader, stage)` (the shader not read): the current pass's stage takes the pair's second texture.
pub fn stage_pair_second(e: &mut Engine, _this: Ptr, stage: u32) {
    let at = pass_stage(e, stage);
    let texture = e.global::<u32>(PAIR_SECOND);
    own::set_stage_texture(e, at, texture);
}

/// A stage's fallback: the exe's `011f951c` with the flag, else none.
fn no_texture(e: &Engine, fallback: bool) -> u32 {
    if fallback {
        e.global::<u32>(NO_TEXTURE)
    } else {
        0
    }
}

// Typed from 00b7c1f0
/// `stdcall(property, stage, index, flag)`: the current pass's stage takes the property's texture `index` (its
/// array +0xb4), or without one the fallback ([`no_texture`]).
pub fn stage_indexed_texture(
    e: &mut Engine,
    property: Ptr,
    stage: u32,
    index: u32,
    fallback: bool,
) {
    let at = pass_stage(e, stage);
    let textures = e.global::<u32>(property.addr().wrapping_add(0xb4));
    let texture = e.global::<u32>(textures.wrapping_add(index.wrapping_mul(4)));
    let texture = if texture != 0 {
        texture
    } else {
        no_texture(e, fallback)
    };
    own::set_stage_texture(e, at, texture);
}

// Typed from 00b7c230
/// `stdcall(property, stage, word, flag)`: the current pass's stage takes what the property's slot 0xf4 answers
/// for (3, word), asked a second time for the store when it is not null; else the fallback ([`no_texture`]).
pub fn stage_slot_texture(e: &mut Engine, property: Ptr, stage: u32, word: u32, fallback: bool) {
    let at = pass_stage(e, stage);
    let object = property.addr();
    let texture = if e.vcall(object, 0xf4, &args![3u32, word]).u32() != 0 {
        e.vcall(object, 0xf4, &args![3u32, word]).u32()
    } else {
        no_texture(e, fallback)
    };
    own::set_stage_texture(e, at, texture);
}

/// One stage setter call of the first subclass's textures ([`subclass_textures`]): `00b7c0a0(property, stage,
/// texture)`, `00b7c0e0(property, stage, word)`, `00b7c1f0(property, stage, index, flag)`, `00b7c230(property,
/// stage, word, flag)`, `00b7c120(stage)`. The exe also calls an empty function (`00d3dd00`) in three lists: left
/// out.
#[derive(Clone, Copy)]
enum StageCall {
    A(u32, u32),
    B(u32, u32),
    F(u32, u32, bool),
    T(u32, u32, bool),
    R(u32),
}

/// The first subclass's stage lists by pass type: 0x231 to 0x233, and the types 0x99 to 0x174 through the exe's
/// jump table (`00bce6c0` with the bytes `00bce6e8`, read from the exe). `None`: the shadow light shader's own.
fn subclass_stage_calls(pass_type: u32) -> Option<&'static [StageCall]> {
    use StageCall::{A, B, F, R, T};
    if (0x231..=0x233).contains(&pass_type) {
        return Some(&[A(0, 0), R(6)]);
    }
    Some(match pass_type {
        0xda | 0xe3 | 0xec | 0xf5 => &[F(3, 0, true), A(0, 0), B(1, 0), T(2, 0, true)],
        0xd9 | 0xe2 | 0xeb | 0xf4 => &[A(0, 0), B(1, 0), T(2, 0, true)],
        0x103 | 0x10f | 0x11a | 0x125 => &[F(4, 0, true), A(0, 0), B(1, 0), T(3, 0, true)],
        0x101 | 0x10e | 0x119 | 0x124 => &[A(0, 0), B(1, 0), T(3, 0, true)],
        0x9a | 0xa5 | 0xb0 | 0xbb => &[F(4, 0, true), A(0, 0), B(1, 0), T(3, 0, true)],
        0x99 | 0xa4 | 0xaf | 0xba => &[A(0, 0), B(1, 0), T(3, 0, true)],
        0x134 | 0x140 => &[A(1, 0), B(0, 0), T(2, 0, true)],
        0x153 => &[A(0, 0), T(2, 0, true)],
        0x15c | 0x162 | 0x168 | 0x16e | 0x174 => &[A(1, 0), B(0, 0), T(2, 0, true)],
        _ => return None,
    })
}

// Typed from 00bce500
/// The first subclass's `SetupGeometryTextures` (vtable `010bb7a8` slot 0x78), `thiscall(shader, state)` (not a
/// Ghidra function: typed from its bytes): for the pass types with a stage list ([`subclass_stage_calls`]) its
/// setters with the state's shader property, then the shader's slot 0x108; every other type the shadow light
/// shader's own (`00b7d200`, the shader family's) with the state.
pub fn subclass_textures(e: &mut Engine, this: Ptr, state: Ptr) {
    let pass_type = packet::rendering_pass_type(e);
    let property = packet::state_property(e, state.addr(), 3);
    let Some(calls) = subclass_stage_calls(pass_type) else {
        typed_call!(
            shader::FAMILY,
            0x00b7_d200,
            shader::textures::setup_geometry_textures(e, this, state)
        );
        return;
    };
    for call in calls {
        match *call {
            StageCall::A(stage, texture) => typed_call!(
                shader::FAMILY,
                0x00b7_c0a0,
                shader::textures::stage_texture(e, Ptr::new(property), stage, texture)
            ),
            StageCall::B(stage, word) => typed_call!(
                shader::FAMILY,
                0x00b7_c0e0,
                shader::textures::stage_second_texture(e, Ptr::new(property), stage, word)
            ),
            StageCall::F(stage, index, fallback) => typed_call!(
                FAMILY,
                0x00b7_c1f0,
                stage_indexed_texture(e, Ptr::new(property), stage, index, fallback)
            ),
            StageCall::T(stage, word, fallback) => typed_call!(
                FAMILY,
                0x00b7_c230,
                stage_slot_texture(e, Ptr::new(property), stage, word, fallback)
            ),
            StageCall::R(stage) => typed_call!(
                shader::FAMILY,
                0x00b7_c120,
                shader::textures::stage_light_texture(e, stage)
            ),
        }
    }
    super::speedtree::after_textures(e, this.addr());
}

/// The pair textures' stages of the subclasses' slot 0xf8 ([`subclass_pair_textures`]): `None` for the types that
/// set none, `Some(None)` for the shadow light shader's own (`00b7cfb0`), else the two stages.
type PairStages = Option<Option<(u32, u32)>>;

/// The first subclass's (`00bcc590`, the exe's jump table `00bcc600` with the bytes `00bcc614`).
fn first_pair_stages(pass_type: u32) -> PairStages {
    match pass_type {
        0x99 | 0x9a | 0xaf | 0xb0 | 0xda | 0xec | 0x101 | 0x103 | 0x119 | 0x11a | 0x134 | 0x140
        | 0x153 | 0x15c | 0x168 | 0x16e | 0x174 => None,
        0xa4 | 0xa5 | 0xba | 0xbb | 0x10e | 0x10f | 0x124 | 0x125 => Some(Some((6, 7))),
        0xe2 | 0xe3 | 0xf4 | 0xf5 => Some(Some((5, 6))),
        0x162 => Some(Some((4, 5))),
        _ => Some(None),
    }
}

/// The second subclass's (`00bce800`, the exe's jump table `00bce858` with the bytes `00bce868`).
fn second_pair_stages(pass_type: u32) -> PairStages {
    match pass_type {
        0xc5 | 0xc8 | 0xcc | 0xcf | 0xd3 | 0xd6 | 0xdb | 0xde | 0xed | 0xf0 | 0x100 | 0x107
        | 0x118 | 0x11e | 0x133 | 0x136 | 0x13f | 0x142 => None,
        0xe4 | 0xe7 | 0xf6 | 0xf9 => Some(Some((5, 6))),
        0x10d | 0x113 | 0x123 | 0x129 => Some(Some((6, 7))),
        _ => Some(None),
    }
}

/// The subclasses' pair textures by their stages ([`PairStages`]).
fn pair_textures_by(e: &mut Engine, this: Ptr, pass_type: u32, stages: PairStages) {
    match stages {
        None => {}
        Some(Some((first, second))) => {
            typed_call!(FAMILY, 0x00b7_c150, stage_pair_first(e, this, first));
            typed_call!(FAMILY, 0x00b7_c170, stage_pair_second(e, this, second));
        }
        Some(None) => typed_call!(
            shader::FAMILY,
            0x00b7_cfb0,
            accumulator::set_pair_textures(e, this.addr(), pass_type)
        ),
    }
}

// Typed from 00bcc590
/// The first subclass's pair textures (slot 0xf8), `thiscall(shader, pass type)`: by [`first_pair_stages`].
pub fn subclass_pair_textures(e: &mut Engine, this: Ptr, pass_type: u32) {
    pair_textures_by(e, this, pass_type, first_pair_stages(pass_type));
}

// Typed from 00bce800
/// The second subclass's pair textures (slot 0xf8), `thiscall(shader, pass type)`: by [`second_pair_stages`].
pub fn second_subclass_pair_textures(e: &mut Engine, this: Ptr, pass_type: u32) {
    pair_textures_by(e, this, pass_type, second_pair_stages(pass_type));
}

/// `BSShaderManager::GetShader` (`00b55560`, by address: it reads the shader table, made at load).
const GET_SHADER: u32 = 0x00b5_5560;

/// The fallback both subclasses' pass choices share: the shadow light shader's (`00b7af80`) with the type, then
/// shader table entry 1's slot 0xf0 with it.
fn other_pass(e: &mut Engine, this: Ptr, pass_type: u32) {
    typed_call!(FAMILY, 0x00b7_af80, select_pass(e, this, pass_type));
    let shader = e.call(GET_SHADER, &args![1u32]).u32();
    e.vcall(shader, 0xf0, &args![pass_type]);
}

/// The first subclass's pass words (`01202230` to `012022b0`), by pass type (the exe's jump tables `00bcd184` with
/// the bytes `00bcd1cc` for 1 to 0xf5 and `00bcd2c4` with `00bcd308` for 0x102 to 0x174, and its own case 0x101).
fn first_pass_word(pass_type: u32) -> Option<u32> {
    Some(match pass_type {
        0xd9 => 0x0120_2230,
        0xda => 0x0120_2234,
        0xe2 => 0x0120_2238,
        0xe3 => 0x0120_223c,
        0xeb => 0x0120_2240,
        0xec => 0x0120_2244,
        0xf4 => 0x0120_2248,
        0xf5 => 0x0120_224c,
        0x101 => 0x0120_2250,
        0x102 => 0x0120_2254,
        0x103 => 0x0120_2258,
        0x10e => 0x0120_225c,
        0x10f => 0x0120_2260,
        0x119 => 0x0120_2264,
        0x11a => 0x0120_2268,
        0x124 => 0x0120_226c,
        0x125 => 0x0120_2270,
        0x99 => 0x0120_2274,
        0x9a => 0x0120_2278,
        0xa4 => 0x0120_227c,
        0xa5 => 0x0120_2280,
        0xaf => 0x0120_2284,
        0xb0 => 0x0120_2288,
        0xba => 0x0120_228c,
        0xbb => 0x0120_2290,
        0x134 => 0x0120_2294,
        0x140 => 0x0120_2298,
        0x153 => 0x0120_229c,
        0x15c => 0x0120_22a0,
        0x162 => 0x0120_22a4,
        0x168 => 0x0120_22a8,
        0x16e => 0x0120_22ac,
        0x174 => 0x0120_22b0,
        _ => return None,
    })
}

// Typed from 00bcce40
/// The first subclass's pass choice (slot 0xf0), `thiscall(shader, pass type)`: the counted pointer +0x28 takes
/// the declaration +0x6c (the exe's inlined assignment, the new one as first read); a type of its table
/// ([`first_pass_word`]) makes that pass current; types 1 and 2 run the fallback ([`other_pass`]) and set +0x28 from
/// +0x6c again (`00a5f860`); any other type runs the fallback.
pub fn first_subclass_select_pass(e: &mut Engine, this: Ptr, pass_type: u32) {
    let base = this.addr();
    own::assign_counted(e, base + 0x28, base + 0x6c, false);
    if let Some(word) = first_pass_word(pass_type) {
        let pass = e.global::<u32>(word);
        own::set_current_pass(e, pass);
        return;
    }
    other_pass(e, this, pass_type);
    if pass_type == 1 || pass_type == 2 {
        let declaration = e.global::<u32>(base + 0x6c);
        own::set_counted(e, base + 0x28, declaration);
    }
}

/// The second subclass's pass words (`012022c0` to `01202324`) and whether the declaration after it is the second
/// (+0x70) rather than the first (+0x6c), by pass type (the exe's jump tables `00bd0128` with the bytes `00bd016c`
/// for 1 to 0xf9 and `00bd0268` with `00bd0298` for 0x107 to 0x142, and its own case 0x100).
fn second_pass_word(pass_type: u32) -> Option<(u32, bool)> {
    Some(match pass_type {
        0xdb => (0x0120_22c0, false),
        0xde => (0x0120_22c4, true),
        0xe4 => (0x0120_22c8, false),
        0xe7 => (0x0120_22cc, true),
        0xed => (0x0120_22d0, false),
        0xf0 => (0x0120_22d4, true),
        0xf6 => (0x0120_22d8, false),
        0xf9 => (0x0120_22dc, true),
        0x100 => (0x0120_22e0, false),
        0x107 => (0x0120_22e4, true),
        0x10d => (0x0120_22e8, false),
        0x113 => (0x0120_22ec, true),
        0x118 => (0x0120_22f0, false),
        0x11e => (0x0120_22f4, true),
        0x123 => (0x0120_22f8, false),
        0x129 => (0x0120_22fc, true),
        0x133 => (0x0120_2300, false),
        0x136 => (0x0120_2304, true),
        0x13f => (0x0120_2308, false),
        0x142 => (0x0120_230c, true),
        0xc5 => (0x0120_2310, false),
        0xc8 => (0x0120_2314, true),
        0xcc => (0x0120_2318, false),
        0xcf => (0x0120_231c, true),
        0xd3 => (0x0120_2320, false),
        0xd6 => (0x0120_2324, true),
        _ => return None,
    })
}

// Typed from 00bcfd20
/// The second subclass's pass choice (slot 0xf0), `thiscall(shader, pass type)`: a type of its table
/// ([`second_pass_word`]) makes that pass current and sets the counted pointer +0x28 from its declaration (+0x6c or
/// +0x70, `00a5f860`); types 1 and 2 run the fallback ([`other_pass`]) then set it from +0x6c, type 3 from +0x70;
/// any other type runs the fallback only. Then for 0x250 and 0x251 the counted pointers +0x30 and +0x34 take +0x7c
/// and +0x80 (the exe's inlined assignment, the source read again after the old one's count went down), for every
/// other type `00c663d0` copies +0x84 into +0x30 and +0x88 into +0x34.
pub fn second_subclass_select_pass(e: &mut Engine, this: Ptr, pass_type: u32) {
    let base = this.addr();
    let declaration = match second_pass_word(pass_type) {
        Some((word, second)) => {
            let pass = e.global::<u32>(word);
            own::set_current_pass(e, pass);
            Some(second)
        }
        None => {
            other_pass(e, this, pass_type);
            match pass_type {
                1 | 2 => Some(false),
                3 => Some(true),
                _ => None,
            }
        }
    };
    if let Some(second) = declaration {
        let object = e.global::<u32>(base + if second { 0x70 } else { 0x6c });
        own::set_counted(e, base + 0x28, object);
    }
    if pass_type == 0x250 || pass_type == 0x251 {
        own::assign_counted(e, base + 0x30, base + 0x7c, true);
        own::assign_counted(e, base + 0x34, base + 0x80, true);
    } else {
        own::copy_counted(e, base + 0x30, base + 0x84);
        own::copy_counted(e, base + 0x34, base + 0x88);
    }
}
