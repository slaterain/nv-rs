//! The shadow light shader's pass set-ups of the four pass type groups with their own (group B): the
//! two `begin_pass` runs for the types 0xfc to 0x102 and 0xca to 0xd1 ([`begin_types_fc`],
//! [`begin_types_ca`]) and the three per-geometry set-ups the standard draw runs for 0xca, 0xd1 and 0x102
//! ([`setup_type_ca`], [`setup_type_d1`], [`setup_type_102`]); the plain geometry pass (0xfc, 0xfd) is
//! the render family's. Each fills the current pass's stages from the geometry's property and sets the
//! world-view constants.

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
/// The shader's own set-ups these call by address: `00b7c3a0`, `00b7c510`, `00b7c580`, `00b7c600`
/// (`thiscall(shader)`), `00b7c850` (`stdcall(matrix, scale)`), `00b7cb00` (`stdcall(matrix, scale,
/// word)`); `D3DXVec3TransformCoord` (`00ee6df8`) and `D3DXMatrixTranspose` (`00ee6dd4`) are [`super::thunks`].
const FOG_CONSTANTS: u32 = 0x00b7_c3a0;
const SETUP_C510: u32 = 0x00b7_c510;
const SETUP_C580: u32 = 0x00b7_c580;
const SETUP_C600: u32 = 0x00b7_c600;
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
    let matrix = object.wrapping_add(0x68);
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
    e.call(FOG_CONSTANTS, &args![shader]);
    let first = e.global::<u32>(SWITCH_A);
    own::set_switch_word(e, first, 1);
    let second = e.global::<u32>(SWITCH_B);
    own::set_switch_word(e, second, 1);
    e.call(SETUP_C510, &args![shader]);
    e.call(SETUP_C580, &args![shader]);
    e.call(SETUP_C600, &args![shader]);
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
    e.call(FOG_CONSTANTS, &args![shader]);
    e.call(SETUP_C510, &args![shader]);
    e.call(SETUP_C580, &args![shader]);
    e.call(SETUP_C600, &args![shader]);
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
        crate::draw::thunks::call_vec3_transform_coord(e, vector, ORIGIN_POINT, local);
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
        crate::draw::thunks::call_vec3_transform_coord(e, frame, ORIGIN_POINT, local);
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
        let first = e.global::<u32>(at - 4);
        if first != 0 && own::entry_is_enabled(e, first) {
            let on = e.global::<u32>(mask) & bit.rotate_left(1) == 0;
            own::set_entry_enabled(e, first, on);
        }
        let second = e.global::<u32>(at);
        bit = bit.rotate_left(1);
        if second != 0 && own::entry_is_enabled(e, second) {
            let on = e.global::<u32>(mask) & bit.rotate_left(1) == 0;
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
    let first = MASKS_FIRST.wrapping_add(pass.wrapping_mul(4));
    if e.global::<u32>(first) != 0 {
        apply_mask(e, first, FIRST_LIST.0, FIRST_LIST.1);
        let map = own::constant_map(e, this.addr(), 0x34);
        e.vcall(map, 0x24, &args![]);
    }
    let second = MASKS_SECOND.wrapping_add(pass.wrapping_mul(4));
    if e.global::<u32>(second) != 0 {
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
        let entry = e.global::<u32>(entries + 4 * index);
        if entry != 0 {
            let word = e.global::<u32>(flags.wrapping_add(pass_type.wrapping_mul(4)));
            let enabled = word & mask.rotate_left(1) != 0;
            own::set_entry_enabled(e, entry, enabled);
        }
        mask = mask.rotate_left(1);
    }
}

/// Whether the entry the pointer at `slot` holds is enabled (+8).
#[inline(always)]
fn entry_enabled(e: &Engine, slot: u32) -> bool {
    let entry = e.global::<u32>(slot);
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
    let pass = e.global::<u32>(PASSES.wrapping_add(pass_type.wrapping_mul(4)));
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
