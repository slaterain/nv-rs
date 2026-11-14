//! The SpeedTree leaf shader's set-ups (vtable `010b9190`; Group C of the drawing thread's plan,
//! `measure/typed-draw/GROUP-C.md`): what the world draw reached as translations. Live reads go through
//! [`super::packet`], the constants they leave through [`super::own`]. Main thread only (the drawing
//! thread replays main's draw with main's control word): `f32` maths. Read in place, not routed: the
//! shader's own words (its renderer +0x18, its constant maps, made at load), the passes' contents and
//! the exe's constants.

use super::{capture, group_c, own, packet, thunks};
use crate::shader::{self, accumulator};
use engine_core::ptr::Ptr;
use engine_core::{args, typed_call, Engine};

/// Bits of 1.0.
const ONE: u32 = 0x3f80_0000;
/// The shader's `SetupGeometry` slot (+0x60, [`setup_geometry`] in the exe's vtable); a leaf
/// property's data (slot 0xfc) and leaves (slot 0x100) through [`packet::leaf_data`] and [`packet::copy_leaves`].
const SLOT_SETUP_GEOMETRY: u32 = 0x60;
const SETUP_GEOMETRY: u32 = 0x00bb_25d0;

/// The rotation's entries and where the frame keeps each scaled one (the first entry is stored first).
const SCALED_ENTRIES: [(u32, u32); 8] = [
    (3, 0xc),
    (6, 0x10),
    (1, 0x18),
    (4, 0x1c),
    (7, 0x20),
    (2, 0x28),
    (5, 0x2c),
    (8, 0x30),
];

// Typed from 00bb25d0
/// The leaf shader's `SetupGeometry` (slot 0x60), `thiscall(shader, geometry, w2, w3, w4, state, w6,
/// transform, bound)`, only the state and the transform read: the renderer's world transform
/// (`00e6fbb0`, the shader family's) from the transform; in a 0x88-byte frame laid out as the exe's (the
/// matrix +8, its inverse +0x48) the D3DX matrix of the transform (the rotation transposed and times the
/// scale, the translation, 1.0) and its inverse (`D3DXMatrixInverse`); then the leaf lights' set-up
/// (`00bb1960`) with the state's shader property when it is of kind 6 (else none), the inverse and the
/// scale, and the camera rows (`00bb1a90`). Answers 0.
#[allow(clippy::too_many_arguments)]
pub fn setup_geometry(
    e: &mut Engine,
    this: Ptr,
    _geometry: u32,
    _word_2: u32,
    _word_3: u32,
    _word_4: u32,
    state: u32,
    _word_6: u32,
    transform: u32,
    _bound: u32,
) -> u32 {
    let base = this.addr();
    let renderer = e.global::<u32>(base.wrapping_add(0x18));
    typed_call!(
        shader::FAMILY,
        0x00e6_fbb0,
        accumulator::set_world_transform(e, Ptr::new(renderer), Ptr::new(transform), 0u8)
    );
    e.with_stack(0x88, |e, frame| {
        let frame = frame.addr();
        let matrix = frame + 8;
        let inverse = frame + 0x48;
        let entry = |e: &Engine, index: u32| {
            f32::from_bits(packet::transform_word(e, transform, 4 * index))
        };
        let scale = f32::from_bits(packet::transform_word(e, transform, 0x30));
        e.set_global(matrix, (scale * entry(e, 0)).to_bits());
        for (index, place) in SCALED_ENTRIES {
            let value = entry(e, index) * scale;
            e.set_global(frame + place, value.to_bits());
        }
        for (k, place) in (0u32..).zip([0x38u32, 0x3c, 0x40]) {
            let word = packet::transform_word(e, transform, 0x24 + 4 * k);
            e.set_global(frame + place, word);
        }
        for place in [0x14u32, 0x24, 0x34] {
            e.set_global(frame + place, 0u32);
        }
        e.set_global(frame + 0x44, ONE);
        thunks::call_matrix_inverse(e, inverse, 0u32, matrix);
        let candidate = packet::state_property(e, state, 3);
        let property = if candidate != 0 && packet::property_kind(e, candidate) == 6 {
            candidate
        } else {
            0
        };
        let scale = packet::transform_word(e, transform, 0x30);
        typed_call!(
            super::FAMILY,
            0x00bb_1960,
            light_setup(e, this, Ptr::new(property), inverse, f32::from_bits(scale))
        );
    });
    typed_call!(super::FAMILY, 0x00bb_1a90, camera_rows(e));
    0
}

/// The constants [`setup_constants`] fills: the diffuse row white under full bright (`011fa0c0`) and
/// the row after it (`011fa0d0`); the leaves' values (`0120065c`, `0120066c`, `01200660`, `01200670`)
/// and the tree's (`01200678`, four words); the full bright row's second word in shader mode 9
/// (`010231a8`, the exe's float).
const DIFFUSE: u32 = 0x011f_a0c0;
const ROW_AFTER: u32 = 0x011f_a0d0;
const LEAF_X: u32 = 0x0120_065c;
const LEAF_Y: u32 = 0x0120_066c;
const LEAF_Z: u32 = 0x0120_0660;
const LEAF_W: u32 = 0x0120_0670;
const TREE: u32 = 0x0120_0678;
const MODE_9_WORD: u32 = 0x0102_31a8;

// Typed from 00bb2760
/// The leaf shader's slot 0x7c, `thiscall(shader, state)`. With the rendered pass and its geometry and
/// the state's leaf property: each of the pass's scene lights to `00b71360` with its index and the
/// property's light value (+0x6c); the shader's `SetupGeometry` (slot 0x60: the geometry, 0, 0, its
/// data's buffers, the state, 0, its transform and its bound; direct when it holds the typed one); under
/// full bright the diffuse row white and the next row 0.0, the exe's float, 0.0, 0.0 in shader mode 9,
/// else zeros; the property's leaves (0x300 bytes) copied into the shader (+0x6c, `_memcpy`); from the
/// property's data (its slot 0xfc, asked each time) its +0x18 and +0x1c times the leaf scales, its +0x14
/// and +0x10, four zeros and its +0xc into the tree's second word; last the vertex and then the pixel
/// constant maps' constants.
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
    let block = state.addr();
    let record = packet::rendering_pass(e);
    let buffers = packet::buffer_data(e, packet::model_data(e, geometry));
    let skin = packet::skin_instance(e, geometry);
    let property = packet::state_property(e, block, 3);
    let light_value = packet::leaf_light_value(e, property);
    let mut index = 0u8;
    while index < packet::rendering_pass_byte9(e, record) {
        let light = packet::rendering_light(e, record, u32::from(index));
        typed_call!(
            super::FAMILY,
            0x00b7_1360,
            scene_light(
                e,
                u32::from(index),
                Ptr::new(light),
                f32::from_bits(light_value)
            )
        );
        index = index.wrapping_add(1);
    }
    if crate::slot_target(e, base, SLOT_SETUP_GEOMETRY) == SETUP_GEOMETRY {
        let _: u32 = typed_call!(
            super::FAMILY,
            0x00bb_25d0,
            setup_geometry(e, this, geometry, 0, 0, buffers, block, 0, transform, bound)
        );
    } else {
        e.vcall(
            base,
            SLOT_SETUP_GEOMETRY,
            &args![geometry, 0u32, 0u32, buffers, block, 0u32, transform, bound],
        );
    }
    if packet::full_bright(e) != 0 {
        let nine = packet::shader_mode(e) == 9;
        for k in 0..4u32 {
            own::set_constant_word(e, DIFFUSE + 4 * k, ONE);
        }
        if nine {
            let value = e.global::<u32>(MODE_9_WORD);
            for (k, word) in (0u32..).zip([0, value, 0, 0]) {
                own::set_constant_word(e, ROW_AFTER + 4 * k, word);
            }
        } else {
            for k in 0..4u32 {
                own::set_constant_word(e, ROW_AFTER + 4 * k, 0);
            }
        }
    }
    packet::copy_leaves(e, base.wrapping_add(0x6c), property);
    let data = packet::leaf_data(e, property);
    let x = f32::from_bits(packet::leaf_data_word(e, data, 0x18))
        * f32::from_bits(packet::leaf_scale_word(e, 0));
    own::set_constant_word(e, LEAF_X, x.to_bits());
    let data = packet::leaf_data(e, property);
    let y = f32::from_bits(packet::leaf_data_word(e, data, 0x1c))
        * f32::from_bits(packet::leaf_scale_word(e, 1));
    own::set_constant_word(e, LEAF_Y, y.to_bits());
    let data = packet::leaf_data(e, property);
    let word = packet::leaf_data_word(e, data, 0x14);
    own::set_constant_word(e, LEAF_Z, word);
    let data = packet::leaf_data(e, property);
    let word = packet::leaf_data_word(e, data, 0x10);
    own::set_constant_word(e, LEAF_W, word);
    for k in 0..4u32 {
        own::set_constant_word(e, TREE + 4 * k, 0);
    }
    let data = packet::leaf_data(e, property);
    let word = packet::leaf_data_word(e, data, 0xc);
    own::set_constant_word(e, TREE + 4, word);
    let words = [geometry, skin, 0, buffers, block, 0, transform, bound, 0, 1];
    group_c::map_constants(e, base, 0x34, 0x5c, words);
    group_c::map_constants(e, base, 0x30, 0x44, words);
}

/// What [`light_setup`] reads (the light rows the per-light set-up left: the direction `011fa1f0`, the position
/// `011fa180`, the range `011fa200`) and fills (the leaves' direction `01200618` with 0.0, position `01200628`,
/// range over the scale `01200634`).
const LIGHT_DIRECTION: u32 = 0x011f_a1f0;
const LIGHT_POSITION: u32 = 0x011f_a180;
const LIGHT_RANGE: u32 = 0x011f_a200;
const LEAF_DIRECTION: u32 = 0x0120_0618;
const LEAF_POSITION: u32 = 0x0120_0628;
const LEAF_RANGE: u32 = 0x0120_0634;

// Typed from 00bb1960
/// The leaf lights' set-up, `thiscall(shader, property, matrix, scale)` (the shader not read): in a 0x38-byte
/// frame laid out as the exe's, the light direction negated (each word through `FLD`/`FCHS`/`FSTP`, so quieted)
/// transformed by the matrix (`D3DXVec3TransformNormal` into +0x2c) and normalized back into +4, stored with 0.0;
/// when the property has active lights (`GetNumberofActiveLights`, its low 16 bits), the light position
/// transformed (`D3DXVec3TransformCoord`, +0x10 into +0x1c) and stored, and the range over the scale.
pub fn light_setup(e: &mut Engine, _this: Ptr, property: Ptr, matrix: u32, scale: f32) {
    e.with_stack(0x38, |e, frame| {
        let frame = frame.addr();
        let vector = frame + 4;
        let transformed = frame + 0x2c;
        for k in 0..3u32 {
            let word =
                engine_core::x87::x87_quiet_bits(own::constant_word(e, LIGHT_DIRECTION + 4 * k));
            e.set_global(vector + 4 * k, word ^ 0x8000_0000);
        }
        thunks::call_vec3_transform_normal(e, transformed, vector, matrix);
        thunks::call_vec3_normalize(e, vector, transformed);
        for k in 0..3u32 {
            let word = engine_core::x87::x87_quiet_bits(e.global::<u32>(vector + 4 * k));
            own::set_constant_word(e, LEAF_DIRECTION + 4 * k, word);
        }
        own::set_constant_word(e, LEAF_DIRECTION + 0xc, 0);
        let active: u32 = typed_call!(
            super::FAMILY,
            0x00b7_0790,
            super::lights::active_light_count(e, property)
        );
        if active & 0xffff == 0 {
            return;
        }
        let position = frame + 0x10;
        let moved = frame + 0x1c;
        for k in 0..3u32 {
            let word =
                engine_core::x87::x87_quiet_bits(own::constant_word(e, LIGHT_POSITION + 4 * k));
            e.set_global(position + 4 * k, word);
        }
        thunks::call_vec3_transform_coord(e, moved, position, matrix);
        for k in 0..3u32 {
            let word = engine_core::x87::x87_quiet_bits(e.global::<u32>(moved + 4 * k));
            own::set_constant_word(e, LEAF_POSITION + 4 * k, word);
        }
        let range = f32::from_bits(own::constant_word(e, LIGHT_RANGE)) / scale;
        own::set_constant_word(e, LEAF_RANGE, range.to_bits());
    });
}

/// The camera rows [`camera_rows`] fills (`01200648` and `01200638`, each three words and 0.0).
const CAMERA_ROW_A: u32 = 0x0120_0648;
const CAMERA_ROW_B: u32 = 0x0120_0638;

// Typed from 00bb1a90
/// The leaf shader's camera rows, no words: from the camera the draw started with (`011f917c`) its words +0x6c,
/// +0x78, +0x84 into the first row and +0x70, +0x7c, +0x88 into the second, each with 0.0 (through `FLD`/`FSTP`,
/// so quieted; read through the packet, which holds the camera).
pub fn camera_rows(e: &mut Engine) {
    let camera = packet::last_camera(e);
    let first = [0x6cu32, 0x78, 0x84]
        .map(|offset| engine_core::x87::x87_quiet_bits(capture::word(e, camera, offset)));
    let second = [0x70u32, 0x7c, 0x88]
        .map(|offset| engine_core::x87::x87_quiet_bits(capture::word(e, camera, offset)));
    for (k, word) in (0u32..).zip(first) {
        own::set_constant_word(e, CAMERA_ROW_A + 4 * k, word);
    }
    own::set_constant_word(e, CAMERA_ROW_A + 0xc, 0);
    for (k, word) in (0u32..).zip(second) {
        own::set_constant_word(e, CAMERA_ROW_B + 4 * k, word);
    }
    own::set_constant_word(e, CAMERA_ROW_B + 0xc, 0);
}

/// The light kind [`scene_light`] hands to the general per-light set-up (`00b70820`) for a rendered geometry's
/// shader whose word +0x68 is not lighting 3.0's (0x11), else to lighting 3.0's setters.
const LIGHTING30_KIND: u32 = 0x11;

// Typed from 00b71360
/// A scene light's constants, `cdecl(index, light, value)` (the value a float, through `FLD`/`FSTP`, so
/// quieted): when the rendered geometry's shader (+0xc0) has a word +0x68 other than 0x11 the per-light set-up
/// (`00b70820`, the shader family's); else lighting 3.0's setter for a light with its byte +0xf4 set (`00b71130`)
/// or the direction light's (`00b70de0`), each with the three words.
pub fn scene_light(e: &mut Engine, index: u32, light: Ptr, value: f32) {
    let geometry = packet::rendering_geometry(e);
    let shader = e.global::<u32>(geometry.wrapping_add(0xc0));
    let value = f32::from_bits(engine_core::x87::x87_quiet_bits(value.to_bits()));
    if e.global::<u32>(shader.wrapping_add(0x68)) != LIGHTING30_KIND {
        typed_call!(
            shader::FAMILY,
            0x00b7_0820,
            shader::lighting::light_setup(e, index, light, value)
        );
    } else if packet::pass_light_byte(e, light.addr(), 0xf4) != 0 {
        typed_call!(
            super::FAMILY,
            0x00b7_1130,
            group_c::other_light(e, index, light, value)
        );
    } else {
        typed_call!(
            super::FAMILY,
            0x00b7_0de0,
            group_c::direction_light(e, index, light, value)
        );
    }
}

/// A pass stage's reference count (+0x60) and `00e7ecd0` (`thiscall(stage)`), which deletes one whose count fell
/// to 0; a shader property's texture (its slot 0xb4).
const STAGE_COUNT: u32 = 0x60;
const STAGE_DELETE: u32 = 0x00e7_ecd0;
const SLOT_PROPERTY_TEXTURE: u32 = 0xb4;

// Typed from 00bb1b20
/// The leaf shader's pass choice (slot 0xf0), `thiscall(shader, pass type)`: for type 0x3e the shader's +0x384
/// becomes the current pass, for 0x3f its +0x388; other types leave it.
pub fn select_pass(e: &mut Engine, this: Ptr, pass_type: u32) {
    let offset = match pass_type {
        0x3e => 0x384u32,
        0x3f => 0x388,
        _ => return,
    };
    let pass = e.global::<u32>(this.addr().wrapping_add(offset));
    own::set_current_pass(e, pass);
}

// Typed from 00bb26d0
/// The leaf shader's slot 0x78, `thiscall(shader, state)`: the current pass's stage 0 held (its count +0x60 up
/// when there is one) while it takes the texture of the state's shader property (its slot 0xb4) and the shader's
/// slot 0x108 runs; then its count goes down (no null check, as the exe) and at 0 it is deleted (`00e7ecd0`).
pub fn setup_textures(e: &mut Engine, this: Ptr, state: Ptr) {
    let property = packet::state_property(e, state.addr(), 3);
    let stage = group_c::pass_stage(e, 0);
    if stage != 0 {
        let count = e.global::<u32>(stage.wrapping_add(STAGE_COUNT));
        e.set_global(stage.wrapping_add(STAGE_COUNT), count.wrapping_add(1));
    }
    let texture = e.vcall(property, SLOT_PROPERTY_TEXTURE, &[]).u32();
    own::set_stage_texture(e, stage, texture);
    after_textures(e, this.addr());
    let left = e
        .global::<u32>(stage.wrapping_add(STAGE_COUNT))
        .wrapping_sub(1);
    e.set_global(stage.wrapping_add(STAGE_COUNT), left);
    if left == 0 {
        e.call(STAGE_DELETE, &args![stage]);
    }
}

/// The shader's slot 0x108 (the render family's `00be2170` direct when the slot holds it).
pub(super) fn after_textures(e: &mut Engine, base: u32) {
    if crate::slot_target(e, base, SLOT_AFTER_TEXTURES) == BIND_PASS_STAGES {
        typed_call!(
            crate::render::FAMILY,
            0x00be_2170,
            crate::render::pass::bind_pass_stages(e); words base
        );
    } else {
        e.vcall(base, SLOT_AFTER_TEXTURES, &[]);
    }
}

/// The fog constants the leaf shader fills (`011fa280`: the far distance, the range, the power, 0.0;
/// `011fa290`: the colour, 0.0).
const FOG_PARAMETERS: u32 = 0x011f_a280;
const FOG_COLOUR: u32 = 0x011f_a290;

// Typed from 00bb1b50
/// The leaf shader's slot 0xfc, `thiscall(shader, unread word)` (the shader not read): with the scene's
/// fog property (`GetFogProperty`, the draw family's), the fog parameters its far distance, far less
/// near, its power and 0.0, and the fog colour its colour and 0.0.
pub fn setup_fog(e: &mut Engine, _this: Ptr, _unused: u32) {
    let index = u32::from(packet::fog_index(e));
    let fog: u32 = typed_call!(
        super::FAMILY,
        0x00b5_5520,
        super::passes::fog_property(e, index)
    );
    if fog == 0 {
        return;
    }
    let far = packet::fog_field(e, fog, 0x30);
    let colour = [0x20u32, 0x24, 0x28].map(|offset| packet::fog_field(e, fog, offset));
    let near = packet::fog_field(e, fog, 0x2c);
    let range = (f32::from_bits(far) - f32::from_bits(near)).to_bits();
    let power = packet::fog_field(e, fog, 0x60);
    for (k, word) in (0u32..).zip([far, range, power, 0]) {
        own::set_constant_word(e, FOG_PARAMETERS + 4 * k, word);
    }
    for (k, word) in (0u32..).zip([colour[0], colour[1], colour[2], 0]) {
        own::set_constant_word(e, FOG_COLOUR + 4 * k, word);
    }
}

/// The SpeedTree branch shader (vtable `010bc070`): its fog slot (0x154, `00c1a310` in the exe's vtable), the
/// shader table's entry 1 (`GetShader`, `00b55560`, read by address) whose two constant maps it sets first, and
/// the land toggles its fog slot fills (`011fa2d0`: four words).
const SLOT_BRANCH_FOG: u32 = 0x154;
const BRANCH_FOG: u32 = 0x00c1_a310;
const GET_SHADER: u32 = 0x00b5_5560;
const LAND_TOGGLES: u32 = 0x011f_a2d0;
/// The exe's constants the fog slot reads: 0.0 (`011a9bd4`), 1.0 (`011a9bdc`) and 30.0 (`01018f5c`).
const ZERO_WORD: u32 = 0x011a_9bd4;
const ONE_WORD: u32 = 0x011a_9bdc;
const THIRTY: u32 = 0x0101_8f5c;

// Typed from 00bd4ba0
/// The SpeedTree branch shader's slot 0x7c, `thiscall(shader, state)`. With the rendered pass's geometry, its
/// bound (or the empty bound), its data's buffers, its skin and the state's material (property 2):
/// `SetupGeometryConstants` (`00b7dab0`, the shader family's); the shader's fog slot (0x154: the geometry, the
/// scene's fog property, the material; direct when it holds the typed one); then with the ten words a set-up
/// hands on (the geometry, its skin, 0, the buffers, the state, 0, its transform, its bound, 0, 1) the shader table
/// entry 1's vertex map (+0x30) for the current pass's vertex program (+0x44) and its pixel map (+0x34) for the
/// pixel program (+0x5c), each without a check; last this shader's own two maps, each only when it and its
/// program are set.
pub fn branch_setup_constants(e: &mut Engine, this: Ptr, state: Ptr) {
    let geometry = packet::rendering_geometry(e);
    // The geometry's world transform and bound as the value packet holds them (value packet B).
    packet::with_drawn(e, geometry, |e, transform, bound| {
        branch_constants_with(e, this, state, geometry, transform, bound);
    });
}

/// [`branch_setup_constants`] with the transform and the bound (the geometry's, or the packet's images of them).
fn branch_constants_with(
    e: &mut Engine,
    this: Ptr,
    state: Ptr,
    geometry: u32,
    transform: u32,
    bound: u32,
) {
    let base = this.addr();
    let block = state.addr();
    let buffers = packet::buffer_data(e, packet::model_data(e, geometry));
    let skin = packet::skin_instance(e, geometry);
    let material = packet::state_property(e, block, 2);
    typed_call!(
        shader::FAMILY,
        0x00b7_dab0,
        shader::geometry::setup_geometry_constants(e, this, state)
    );
    let index = u32::from(packet::fog_index(e));
    let fog: u32 = typed_call!(
        super::FAMILY,
        0x00b5_5520,
        super::passes::fog_property(e, index)
    );
    if crate::slot_target(e, base, SLOT_BRANCH_FOG) == BRANCH_FOG {
        typed_call!(
            super::FAMILY,
            0x00c1_a310,
            branch_fog(e, this, geometry, fog, material)
        );
    } else {
        e.vcall(base, SLOT_BRANCH_FOG, &args![geometry, fog, material]);
    }
    let table = e.call(GET_SHADER, &args![1u32]).u32();
    let words = [geometry, skin, 0, buffers, block, 0, transform, bound, 0, 1];
    for (map_offset, program_offset) in [(0x30u32, 0x44u32), (0x34, 0x5c)] {
        let map = own::constant_map(e, table, map_offset);
        let program = e.global::<u32>(packet::current_pass(e).wrapping_add(program_offset));
        group_c::set_constants_of(e, map, program, words);
    }
    group_c::map_constants(e, base, 0x30, 0x44, words);
    group_c::map_constants(e, base, 0x34, 0x5c, words);
}

/// The branch shader's counted pointer (+0x28) and the declaration its pass choice puts there (+0x6c); the slot
/// after the textures (+0x108, `00be2170` in the exe's vtable: the render family's).
const BRANCH_DECLARATION_SLOT: u32 = 0x28;
const BRANCH_DECLARATION: u32 = 0x6c;
const SLOT_AFTER_TEXTURES: u32 = 0x108;
const BIND_PASS_STAGES: u32 = 0x00be_2170;
/// `BSRenderedTexture::GetTexture` (`004bc320`, `thiscall(texture, 0)`), by address; the pair of textures the
/// branch's pass types put into two stages (`011f95e4`, `011f95e8`: main's, set before the draw).
const RENDERED_TEXTURE: u32 = 0x004b_c320;
const PAIR_TEXTURES: [u32; 2] = [0x011f_95e4, 0x011f_95e8];

/// Where the branch shader's pass choice finds a pass type's pass: the exe's word `01202350` to `012023c4` (the
/// jump tables `00bd4254` with the bytes `00bd42b4` for 0x60 to 0x159, and `00bd43b0` with `00bd43cc` for 0x165 to
/// 0x256; 0x15f its own case).
fn branch_pass_word(pass_type: u32) -> Option<u32> {
    Some(match pass_type {
        0x60 => 0x0120_2350,
        0x7a => 0x0120_2354,
        0x7b => 0x0120_2358,
        0x7e => 0x0120_235c,
        0x130 => 0x0120_2360,
        0x131 => 0x0120_2364,
        0x151 => 0x0120_2368,
        0x158 => 0x0120_236c,
        0x159 => 0x0120_2370,
        0x256 => 0x0120_2374,
        0xdf => 0x0120_2378,
        0xe8 => 0x0120_237c,
        0xf1 => 0x0120_2380,
        0xfa => 0x0120_2384,
        0x108 => 0x0120_2388,
        0x114 => 0x0120_238c,
        0x11f => 0x0120_2390,
        0x12a => 0x0120_2394,
        0x9f => 0x0120_2398,
        0xaa => 0x0120_239c,
        0xb5 => 0x0120_23a0,
        0xc0 => 0x0120_23a4,
        0x137 => 0x0120_23a8,
        0x143 => 0x0120_23ac,
        0x15f => 0x0120_23b0,
        0x165 => 0x0120_23b4,
        0x16b => 0x0120_23b8,
        0x171 => 0x0120_23bc,
        0x177 => 0x0120_23c0,
        0x234 => 0x0120_23c4,
        _ => return None,
    })
}

// Typed from 00bd4030
/// The SpeedTree branch shader's pass choice (slot 0xf0), `thiscall(shader, pass type)`: for its pass types the
/// exe's pass word for the type ([`branch_pass_word`]) becomes the current pass; then, for every type, the
/// counted pointer +0x28 takes the shader's +0x6c (the exe's inlined assignment: nothing when equal, the old
/// one's count down, deleted at 0, the new one stored as first read and counted).
pub fn branch_select_pass(e: &mut Engine, this: Ptr, pass_type: u32) {
    if let Some(word) = branch_pass_word(pass_type) {
        let pass = e.global::<u32>(word);
        own::set_current_pass(e, pass);
    }
    let base = this.addr();
    own::assign_counted(
        e,
        base.wrapping_add(BRANCH_DECLARATION_SLOT),
        base.wrapping_add(BRANCH_DECLARATION),
        false,
    );
}

/// What [`branch_setup_textures`] puts into the stages from the state's shader property, by the rendered pass's
/// type (the exe's first switch, the bytes `00bd49c8` into `00bd49b8`): stage 0 the word at the property's
/// `[+0xac]` and stage 1 its texture 0; stage 0 its texture 0; stage 0 the `[+0xac]` word.
enum BranchTextures {
    SourceAndFirst,
    First,
    Source,
}

fn branch_textures(pass_type: u32) -> Option<BranchTextures> {
    Some(match pass_type {
        0x7a | 0x7b | 0x9f | 0xaa | 0xb5 | 0xc0 | 0xdf | 0xe8 | 0xf1 | 0xfa | 0x108 | 0x114
        | 0x11f | 0x12a => BranchTextures::SourceAndFirst,
        0x7e | 0x130 | 0x131 | 0x137 | 0x143 | 0x158 | 0x159 | 0x15f | 0x165 | 0x16b | 0x171
        | 0x177 => BranchTextures::First,
        0x151 | 0x234 => BranchTextures::Source,
        _ => return None,
    })
}

/// The stages the pair of textures (`011f95e4`, `011f95e8`) go to, by type (the exe's second switch, the bytes
/// `00bd4ad8` into `00bd4ac8`).
fn branch_pair_stages(pass_type: u32) -> Option<(u32, u32)> {
    match pass_type {
        0xaa | 0xc0 | 0x114 | 0x12a => Some((6, 7)),
        0xe8 | 0xfa => Some((5, 6)),
        0x165 => Some((1, 2)),
        _ => None,
    }
}

// Typed from 00bd4890
/// The SpeedTree branch shader's slot 0x78, `thiscall(shader, state)` (not a Ghidra function: typed from its
/// bytes). With the current pass (read once) and the rendered pass's type: the stages from the state's shader
/// property ([`branch_textures`]; texture 0 through `00c03230`, the shader family's); the pair of textures into
/// two stages ([`branch_pair_stages`]); for type 0x234 stage 6 the shadow map of the rendered pass's first scene
/// light (`BSRenderedTexture::GetTexture(0)` of its +0x10c); last the shader's slot 0x108 (direct when it holds
/// the render family's).
pub fn branch_setup_textures(e: &mut Engine, this: Ptr, state: Ptr) {
    let property = packet::state_property(e, state.addr(), 3);
    let pass = packet::current_pass(e);
    let pass_type = packet::rendering_pass_type(e);
    let stage = |e: &Engine, index: u32| {
        let stages = e.global::<u32>(pass.wrapping_add(0x24));
        e.global::<u32>(stages.wrapping_add(index.wrapping_mul(4)))
    };
    let source = |e: &Engine| {
        let holder = e.global::<u32>(property.wrapping_add(0xac));
        e.global::<u32>(holder)
    };
    let first = |e: &mut Engine| -> u32 {
        typed_call!(
            shader::FAMILY,
            0x00c0_3230,
            accumulator::texture_by_index(e, Ptr::new(property), 0u32)
        )
    };
    match branch_textures(pass_type) {
        Some(BranchTextures::SourceAndFirst) => {
            let texture = source(e);
            let stage0 = stage(e, 0);
            own::set_stage_texture(e, stage0, texture);
            let stage1 = stage(e, 1);
            let texture = first(e);
            own::set_stage_texture(e, stage1, texture);
        }
        Some(BranchTextures::First) => {
            let stage0 = stage(e, 0);
            let texture = first(e);
            own::set_stage_texture(e, stage0, texture);
        }
        Some(BranchTextures::Source) => {
            let texture = source(e);
            let stage0 = stage(e, 0);
            own::set_stage_texture(e, stage0, texture);
        }
        None => {}
    }
    if let Some(stages) = branch_pair_stages(pass_type) {
        for (index, texture) in [stages.0, stages.1].into_iter().zip(PAIR_TEXTURES) {
            let at = stage(e, index);
            let texture = e.global::<u32>(texture);
            own::set_stage_texture(e, at, texture);
        }
    }
    if pass_type == 0x234 {
        let light = packet::rendering_first_light(e);
        let at = stage(e, 6);
        let shadow_map = packet::scene_light_word(e, light, 0x10c);
        let texture = e.call(RENDERED_TEXTURE, &args![shadow_map, 0u32]).u32();
        own::set_stage_texture(e, at, texture);
    }
    after_textures(e, this.addr());
}

// Typed from 00c1a310
/// The SpeedTree branch shader's fog slot (0x154), `thiscall(shader, geometry, fog, material)` (the shader and
/// the geometry not read): the land toggles 1.0; 1.0 with a fog property, else the exe's 0.0; the material's
/// +0x38 (through an `FLD` and `FSTP`, so quieted), or the exe's 30.0 without one; the exe's 1.0.
pub fn branch_fog(e: &mut Engine, _this: Ptr, _geometry: u32, fog: u32, material: u32) {
    let second = if fog != 0 {
        ONE
    } else {
        e.global::<u32>(ZERO_WORD)
    };
    let third = engine_core::x87::x87_quiet_bits(if material != 0 {
        packet::material_word(e, material, 0x38)
    } else {
        e.global::<u32>(THIRTY)
    });
    let last = e.global::<u32>(ONE_WORD);
    for (k, word) in (0u32..).zip([ONE, second, third, last]) {
        own::set_constant_word(e, LAND_TOGGLES + 4 * k, word);
    }
}
