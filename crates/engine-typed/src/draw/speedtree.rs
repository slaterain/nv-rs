//! The SpeedTree leaf shader's set-ups (vtable `010b9190`; Group C of the drawing thread's plan,
//! `measure/typed-draw/GROUP-C.md`): what the world draw reached as translations. Live reads go through
//! [`super::packet`], the constants they leave through [`super::own`]. Main thread only (the drawing
//! thread replays main's draw with main's control word): `f32` maths. Read in place, not routed: the
//! shader's own words (its renderer +0x18, its constant maps, made at load), the passes' contents and
//! the exe's constants.

use super::{group_c, own, packet, thunks};
use crate::shader::{self, accumulator};
use engine_core::ptr::Ptr;
use engine_core::{args, typed_call, Engine};

/// Bits of 1.0.
const ONE: u32 = 0x3f80_0000;
/// What these call by address: the leaf lights' set-up (`00bb1960`, `thiscall(shader, property,
/// inverse, scale)`), the camera rows (`00bb1a90`, no words) and a scene light's constants (`00b71360`,
/// `cdecl(index, light, value)`).
const LIGHT_SETUP: u32 = 0x00bb_1960;
const CAMERA_ROWS: u32 = 0x00bb_1a90;
const SCENE_LIGHT: u32 = 0x00b7_1360;
/// The shader's `SetupGeometry` slot (+0x60, [`setup_geometry`] in the exe's vtable); a leaf
/// property's leaves (+0x100, 0x300 bytes) and data (+0xfc).
const SLOT_SETUP_GEOMETRY: u32 = 0x60;
const SETUP_GEOMETRY: u32 = 0x00bb_25d0;
const SLOT_LEAVES: u32 = 0x100;
const SLOT_LEAF_DATA: u32 = 0xfc;

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
        e.call(LIGHT_SETUP, &args![base, property, inverse, scale]);
    });
    e.call(CAMERA_ROWS, &[]);
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
    let base = this.addr();
    let block = state.addr();
    let record = packet::rendering_pass(e);
    let geometry = packet::rendering_geometry(e);
    let bound = packet::bound_of(e, geometry);
    let buffers = packet::buffer_data(e, packet::model_data(e, geometry));
    let skin = packet::skin_instance(e, geometry);
    let property = packet::state_property(e, block, 3);
    let light_value = packet::leaf_light_value(e, property);
    let mut index = 0u8;
    while index < packet::rendering_pass_byte9(e, record) {
        let light = packet::rendering_light(e, record, u32::from(index));
        e.call(SCENE_LIGHT, &args![u32::from(index), light, light_value]);
        index = index.wrapping_add(1);
    }
    let transform = geometry.wrapping_add(0x68);
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
    let leaves = e.vcall(property, SLOT_LEAVES, &[]).u32();
    own::copy_constants(e, base.wrapping_add(0x6c), leaves, 0x300);
    let data = e.vcall(property, SLOT_LEAF_DATA, &[]).u32();
    let x = f32::from_bits(packet::leaf_data_word(e, data, 0x18))
        * f32::from_bits(packet::leaf_scale_word(e, 0));
    own::set_constant_word(e, LEAF_X, x.to_bits());
    let data = e.vcall(property, SLOT_LEAF_DATA, &[]).u32();
    let y = f32::from_bits(packet::leaf_data_word(e, data, 0x1c))
        * f32::from_bits(packet::leaf_scale_word(e, 1));
    own::set_constant_word(e, LEAF_Y, y.to_bits());
    let data = e.vcall(property, SLOT_LEAF_DATA, &[]).u32();
    let word = packet::leaf_data_word(e, data, 0x14);
    own::set_constant_word(e, LEAF_Z, word);
    let data = e.vcall(property, SLOT_LEAF_DATA, &[]).u32();
    let word = packet::leaf_data_word(e, data, 0x10);
    own::set_constant_word(e, LEAF_W, word);
    for k in 0..4u32 {
        own::set_constant_word(e, TREE + 4 * k, 0);
    }
    let data = e.vcall(property, SLOT_LEAF_DATA, &[]).u32();
    let word = packet::leaf_data_word(e, data, 0xc);
    own::set_constant_word(e, TREE + 4, word);
    let words = [geometry, skin, 0, buffers, block, 0, transform, bound, 0, 1];
    group_c::map_constants(e, base, 0x34, 0x5c, words);
    group_c::map_constants(e, base, 0x30, 0x44, words);
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
