//! The shadow light shader's pass set-ups of the four pass type groups with their own (group B): the
//! two `begin_pass` runs for the types 0xfc to 0x102 and 0xca to 0xd1 ([`begin_types_fc`],
//! [`begin_types_ca`]) and the three per-geometry set-ups the standard draw runs for 0xca, 0xd1 and 0x102
//! ([`setup_type_ca`], [`setup_type_d1`], [`setup_type_102`]); the plain geometry pass (0xfc, 0xfd) is
//! the render family's. Each fills the current pass's stages from the geometry's property and sets the
//! world-view constants.

use crate::render::pass::property_clamp_mode;
use crate::shader::{self, accumulator, state};
use engine_core::ptr::Ptr;
use engine_core::x87::{x87_load32, x87_quiet_bits, x87_store32, x87_sub};
use engine_core::{args, typed_call, Engine};

/// The current shader pass (`0126f74c`): its stages (+0x24), its constants holder (+0x5c, the device
/// at its +0x20), its word +0x44.
const CURRENT_PASS: u32 = 0x0126_f74c;
/// The two switch objects whose word +0x20 the first begin sets (`011febf0`, `011fec38`).
const SWITCH_A: u32 = 0x011f_ebf0;
const SWITCH_B: u32 = 0x011f_ec38;
/// The holder of the view object whose matrix (+0x68) the passes invert (`011f91e0`).
const VIEW_HOLDER: u32 = 0x011f_91e0;
/// The word stage 3 takes when the geometry answers none (`011f951c`).
const DEFAULT_STAGE_3: u32 = 0x011f_951c;
/// The point the passes transform (`011fa2a0`) and 1.0 (a double, `01012070`).
const ORIGIN_POINT: u32 = 0x011f_a2a0;
const ONE_DOUBLE: u32 = 0x0101_2070;
/// The shader's own set-ups these call by address: `00b7c3a0`, `00b7c510`, `00b7c580`, `00b7c600`
/// (`thiscall(shader)`), `00b7c850` (`stdcall(matrix, scale)`), `00b7cb00` (`stdcall(matrix, scale,
/// word)`), and `D3DXVec3TransformCoord` (`00ee6df8`, `stdcall(out, point, matrix)`).
const FOG_CONSTANTS: u32 = 0x00b7_c3a0;
const SETUP_C510: u32 = 0x00b7_c510;
const SETUP_C580: u32 = 0x00b7_c580;
const SETUP_C600: u32 = 0x00b7_c600;
const SCALED_LIGHT: u32 = 0x00b7_c850;
const SCALED_LIGHT_WORD: u32 = 0x00b7_cb00;
const TRANSFORM_COORD: u32 = 0x00ee_6df8;
/// `IDirect3DDevice9::SetVertexShaderConstantF` (slot 94).
const DEVICE_SET_VERTEX_CONSTANTS: u32 = 0x178;

/// An array index as a word (0..4 here).
#[inline(always)]
#[allow(clippy::cast_possible_truncation)]
fn idx(i: usize) -> u32 {
    i as u32
}

/// The current pass's stage `index`, read afresh each time.
#[inline(always)]
fn pass_stage(e: &Engine, index: u32) -> u32 {
    let pass = e.global::<u32>(CURRENT_PASS);
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
    e.set_global(stage0.wrapping_add(8), texture);
    stage_modes(e, stage0, property);
    let stage1 = pass_stage(e, 1);
    let texture: u32 = typed_call!(
        shader::FAMILY,
        0x00c0_3230,
        accumulator::texture_by_index(e, Ptr::new(property), 0u32)
    );
    e.set_global(stage1.wrapping_add(8), texture);
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

/// The view object's matrix inverted into `out` and the shader's world-view constants set from it;
/// answers the view object.
fn view_matrix(e: &mut Engine, shader: u32, out: u32) -> u32 {
    let holder = e.global::<u32>(VIEW_HOLDER);
    let object = e.global::<u32>(holder);
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
    let object = e.global::<u32>(shader.wrapping_add(0x30));
    if object == 0 {
        return;
    }
    let pass = e.global::<u32>(CURRENT_PASS);
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
    let pass = e.global::<u32>(CURRENT_PASS);
    let holder = e.global::<u32>(pass.wrapping_add(0x5c));
    let device = e.global::<u32>(holder.wrapping_add(0x20));
    e.vcall(
        device,
        DEVICE_SET_VERTEX_CONSTANTS,
        &args![0x10u32, vector, 1u32],
    );
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
    e.set_global(first.wrapping_add(0x20), 1u32);
    let second = e.global::<u32>(SWITCH_B);
    e.set_global(second.wrapping_add(0x20), 1u32);
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
/// The set-up of type 0xca, `thiscall(shader, pass)`, the property `[pass + 0xc]`: the property's
/// stages, both bound, the view matrix into a local (frame +0x10 of 0x84 bytes), the scaled light
/// (`00b7c850`, the view object's scale +0x98) and the last upload (big +0x50, small +0).
pub fn setup_type_ca(e: &mut Engine, this: Ptr, pass: Ptr) {
    let shader = this.addr();
    let property = e.global::<u32>(pass.addr().wrapping_add(0xc));
    property_stages(e, property);
    bind_stage(e, 0);
    bind_stage(e, 1);
    e.with_stack(0x84, |e, frame| {
        let frame = frame.addr();
        let local = frame + 0x10;
        let object = view_matrix(e, shader, local);
        let scale = x87_quiet_bits(e.global::<u32>(object.wrapping_add(0x98)));
        e.call(SCALED_LIGHT, &args![local, scale]);
        upload_last(e, shader, frame + 0x50, frame);
    });
}

// Typed from 00b7dfe0
/// The set-up of type 0xd1, `thiscall(shader, pass)`: as [`setup_type_ca`] up to the view matrix (frame
/// +0x14 of 0x88 bytes), then the scaled light with the pass's word +8 (`00b7cb00`); the view holder's
/// byte +9 less 1.0 as the fourth float of the point `011fa2a0` transformed by the local, which goes to
/// the vertex constant 0x10; the last upload (big +0x54, small +4).
pub fn setup_type_d1(e: &mut Engine, this: Ptr, pass: Ptr) {
    let shader = this.addr();
    let property = e.global::<u32>(pass.addr().wrapping_add(0xc));
    property_stages(e, property);
    bind_stage(e, 0);
    bind_stage(e, 1);
    e.with_stack(0x88, |e, frame| {
        let frame = frame.addr();
        let local = frame + 0x14;
        let holder = e.global::<u32>(VIEW_HOLDER);
        let object = view_matrix(e, shader, local);
        let scale = x87_quiet_bits(e.global::<u32>(object.wrapping_add(0x98)));
        let word = e.global::<u32>(pass.addr().wrapping_add(8));
        e.call(SCALED_LIGHT_WORD, &args![local, scale, word]);
        let byte = e.global::<u8>(holder.wrapping_add(9));
        let one: f64 = e.global(ONE_DOUBLE);
        let w = x87_store32(x87_sub(f64::from(byte), one));
        e.set_global(frame, w);
        let vector = frame + 4;
        e.call(TRANSFORM_COORD, &args![vector, ORIGIN_POINT, local]);
        let w = x87_quiet_bits(e.global::<u32>(frame));
        e.set_global(frame + 0x10, w);
        light_constant(e, vector);
        upload_last(e, shader, frame + 0x54, frame + 4);
    });
}

// Typed from 00b7e150
/// The set-up of type 0x102, `thiscall(shader, pass)`: the property's stages; stage 3 the property's
/// slot 0xf4 answer for (3, 0) (asked again when not 0) or `011f951c`'s word; stages 0, 1 and 3 bound;
/// the view matrix (frame +0x10 of 0x50 bytes) with the light direction; the view object's scale as the
/// fourth float of the transformed point, the vertex constant 0x10.
pub fn setup_type_102(e: &mut Engine, this: Ptr, pass: Ptr) {
    let shader = this.addr();
    let property = e.global::<u32>(pass.addr().wrapping_add(0xc));
    property_stages(e, property);
    let stage3 = pass_stage(e, 3);
    let value = if e.vcall(property, 0xf4, &args![3u32, 0u32]).u32() != 0 {
        e.vcall(property, 0xf4, &args![3u32, 0u32]).u32()
    } else {
        e.global::<u32>(DEFAULT_STAGE_3)
    };
    e.set_global(stage3.wrapping_add(8), value);
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
        let w = x87_quiet_bits(e.global::<u32>(object.wrapping_add(0x98)));
        e.call(TRANSFORM_COORD, &args![frame, ORIGIN_POINT, local]);
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
        if first != 0 && e.global::<u8>(first.wrapping_add(8)) != 0 {
            let on = e.global::<u32>(mask) & bit.rotate_left(1) == 0;
            e.set_global(first.wrapping_add(8), u8::from(on));
        }
        let second = e.global::<u32>(at);
        bit = bit.rotate_left(1);
        if second != 0 && e.global::<u8>(second.wrapping_add(8)) != 0 {
            let on = e.global::<u32>(mask) & bit.rotate_left(1) == 0;
            e.set_global(second.wrapping_add(8), u8::from(on));
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
        let map = e.global::<u32>(this.addr().wrapping_add(0x34));
        e.vcall(map, 0x24, &args![]);
    }
    let second = MASKS_SECOND.wrapping_add(pass.wrapping_mul(4));
    if e.global::<u32>(second) != 0 {
        apply_mask(e, second, SECOND_LIST.0, SECOND_LIST.1);
        let map = e.global::<u32>(this.addr().wrapping_add(0x30));
        e.vcall(map, 0x24, &args![]);
    }
}

/// The constant entries of the vertex (`011febd8`) and pixel (`011fec30`) maps, 22 each, and the pass
/// types' enable words for them (`011fcc80`, `011fc0a0`; entry k: bit k + 1).
const VERTEX_ENTRIES: u32 = 0x011f_ebd8;
const PIXEL_ENTRIES: u32 = 0x011f_ec30;
const VERTEX_FLAGS: u32 = 0x011f_cc80;
const PIXEL_FLAGS: u32 = 0x011f_c0a0;
/// The rendering pass holder (`011f91e0`), the fog constants (`011fa280`), the far distance with no fog
/// distances (`010ad6e0`), the fog colour (`011a9bd0`) and its three copies (`011fd9b8`), the fog
/// index byte (`011f91c4`), `GetFogProperty` (`00b55520`) and the matrix transpose (`00ee6dd4`).
const RENDERING_PASS: u32 = 0x011f_91e0;
const FOG: u32 = 0x011f_a280;
const FAR_DISTANCE: u32 = 0x010a_d6e0;
const FOG_COLOUR: u32 = 0x011a_9bd0;
const FOG_COLOURS: u32 = 0x011f_d9b8;
const FOG_INDEX: u32 = 0x011f_91c4;
const GET_FOG_PROPERTY: u32 = 0x00b5_5520;
const MATRIX_TRANSPOSE: u32 = 0x00ee_6dd4;

/// The 22 entries at `entries` enabled by the pass type's word of `flags` (entry k by bit k + 1, the
/// word read for each).
fn enable_entries(e: &mut Engine, entries: u32, flags: u32, pass_type: u32) {
    let mut mask = 1u32;
    for index in 0..22u32 {
        let entry = e.global::<u32>(entries + 4 * index);
        if entry != 0 {
            let word = e.global::<u32>(flags.wrapping_add(pass_type.wrapping_mul(4)));
            let enabled = word & mask.rotate_left(1) != 0;
            e.set_global(entry.wrapping_add(8), u8::from(enabled));
        }
        mask = mask.rotate_left(1);
    }
}

/// Whether the entry the pointer at `slot` holds is enabled (+8).
#[inline(always)]
fn entry_enabled(e: &Engine, slot: u32) -> bool {
    let entry = e.global::<u32>(slot);
    e.global::<u8>(entry.wrapping_add(8)) != 0
}

/// The fog constants (`011fa280`): with a fog property, (500000.0, 0, 0, 0) and the fog colour when its
/// far (+0x30) and near (+0x2c) distances are both 0, else (far, far - near, its +0x60, 0) and its
/// colour (+0x20, +0x24, +0x28, 0). Each float copied through x87 (a signalling NaN quieted).
fn fog_constants(e: &mut Engine) {
    let index = u32::from(e.global::<u8>(FOG_INDEX));
    let fog = e.call(GET_FOG_PROPERTY, &args![index]).u32();
    if fog == 0 {
        return;
    }
    let near = x87_quiet_bits(e.global::<u32>(fog.wrapping_add(0x2c)));
    let far = x87_quiet_bits(e.global::<u32>(fog.wrapping_add(0x30)));
    let power = x87_quiet_bits(e.global::<u32>(fog.wrapping_add(0x60)));
    let far_value = x87_load32(far);
    let near_value = x87_load32(near);
    if far_value == 0.0 && near_value == 0.0 {
        let distance = x87_quiet_bits(e.global::<u32>(FAR_DISTANCE));
        e.set_global(FOG, distance);
        for offset in [4u32, 8, 0xc] {
            e.set_global(FOG + offset, 0u32);
        }
        let colour: [u32; 4] = std::array::from_fn(|i| e.global::<u32>(FOG_COLOUR + 4 * idx(i)));
        e.set_global(FOG + 0x10, colour[0]);
        e.set_global(FOG + 0x1c, colour[3]);
        e.set_global(FOG + 0x18, colour[2]);
        e.set_global(FOG + 0x14, colour[1]);
    } else {
        let red = e.global::<u32>(fog.wrapping_add(0x20));
        let green = e.global::<u32>(fog.wrapping_add(0x24));
        let span = x87_store32(x87_sub(far_value, near_value));
        let blue = e.global::<u32>(fog.wrapping_add(0x28));
        e.set_global(FOG, far);
        e.set_global(FOG + 4, span);
        e.set_global(FOG + 8, x87_quiet_bits(power));
        e.set_global(FOG + 0xc, 0u32);
        e.set_global(FOG + 0x10, x87_quiet_bits(red));
        e.set_global(FOG + 0x1c, 0u32);
        e.set_global(FOG + 0x18, x87_quiet_bits(blue));
        e.set_global(FOG + 0x14, x87_quiet_bits(green));
    }
}

// Typed from 00b7b450
/// `ShadowLightShader::SetupPassConstants` (engine map; a shader's slot 0xfc), `thiscall(shader, pass
/// type)`: nothing for 0x250 and 0x251. The vertex and pixel entries are enabled by the type's words,
/// the vertex (+0x34) and pixel (+0x30) constant maps sorted (their slot 0x24); with vertex entry 8 the
/// fog constants; when the rendering pass's first word has +0xbc, the renderer's (+0x18) view matrix
/// (+0xa00) transposed into a local and set as vertex constants 1 to 4; with vertex entry 8 the fog
/// colour copied three times.
pub fn setup_pass_constants(e: &mut Engine, this: Ptr, pass_type: u32) {
    if pass_type == 0x250 || pass_type == 0x251 {
        return;
    }
    let base = this.addr();
    enable_entries(e, VERTEX_ENTRIES, VERTEX_FLAGS, pass_type);
    enable_entries(e, PIXEL_ENTRIES, PIXEL_FLAGS, pass_type);
    let vertex_map = e.global::<u32>(base.wrapping_add(0x34));
    e.vcall(vertex_map, 0x24, &args![]);
    let pixel_map = e.global::<u32>(base.wrapping_add(0x30));
    e.vcall(pixel_map, 0x24, &args![]);
    if entry_enabled(e, VERTEX_ENTRIES + 4 * 8) {
        fog_constants(e);
    }
    let rendering = e.global::<u32>(RENDERING_PASS);
    let first = e.global::<u32>(rendering);
    if e.global::<u32>(first.wrapping_add(0xbc)) != 0 {
        let renderer = e.global::<u32>(base.wrapping_add(0x18));
        e.with_stack(0x40, |e, matrix| {
            e.call(MATRIX_TRANSPOSE, &args![matrix, renderer.wrapping_add(0xa00)]);
            let pass = e.global::<u32>(CURRENT_PASS);
            let holder = e.global::<u32>(pass.wrapping_add(0x5c));
            let device = e.global::<u32>(holder.wrapping_add(0x20));
            e.vcall(device, DEVICE_SET_VERTEX_CONSTANTS, &args![1u32, matrix, 4u32]);
        });
    }
    if entry_enabled(e, VERTEX_ENTRIES + 4 * 8) {
        let colour: [u32; 4] = std::array::from_fn(|i| e.global::<u32>(FOG_COLOUR + 4 * idx(i)));
        for copy in 0..3u32 {
            for (index, word) in (0u32..).zip(colour) {
                e.set_global(FOG_COLOURS + 0x10 * copy + 4 * index, word);
            }
        }
    }
}
