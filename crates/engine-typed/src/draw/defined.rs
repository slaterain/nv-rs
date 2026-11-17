//! The shader constant map's defined constants (`00e83db0`), typed so that the renderer's matrices it hands the
//! shaders (world, view, projection, view-projection, inverse view) are read through the draw's own state
//! ([`super::statics::RENDERER_BLOCKS`]): on the drawing thread the renderer's words are the drawing side's copy,
//! while main draws its own scenes with its own. The rest is the translation's behaviour unchanged.

use super::capture;
use engine_core::ptr::Ptr;
use engine_core::x87::{x87_add, x87_load32, x87_m32, x87_quiet_bits, x87_store32};
use engine_core::{args, Engine};

/// The D3DX thunks (the draw family's): `D3DXMatrixInverse`, `D3DXMatrixTranspose`, `D3DXMatrixMultiply` and the
/// multiply-and-transpose `00ee6dfe`.
const MATRIX_INVERSE: u32 = 0x00ee_6dda;
const MATRIX_TRANSPOSE: u32 = 0x00ee_6dd4;
const MATRIX_MULTIPLY: u32 = 0x00ee_6de0;
const MATRIX_PRODUCT: u32 = 0x00ee_6dfe;
/// `SetupTextureTransformMatrix` (`00e83550`), a texture transform's `UpdateMatrix` (`00bc5c20`), the texturing
/// property's map for an index (`00a463e0`).
const SETUP_TEXTURE_TRANSFORM: u32 = 0x00e8_3550;
const UPDATE_MATRIX: u32 = 0x00bc_5c20;
const MAP_AT: u32 = 0x00a4_63e0;
/// The extra data's name handle (`011bf65c`), `NiFixedString`'s constructor and destructor, `GetExtraData`.
const EXTRA_DATA_NAME: u32 = 0x011b_f65c;
const FIXED_STRING_CONSTRUCT: u32 = 0x0043_8170;
const FIXED_STRING_DESTRUCT: u32 = 0x0043_81b0;
const GET_EXTRA_DATA: u32 = 0x00a5_bdd0;
/// The CRT's x87 helpers the time constants go through.
const CRT_HELPER_A: u32 = 0x00ec_a0a0;
const CRT_HELPER_B: u32 = 0x00ec_9f70;
const CRT_HELPER_C: u32 = 0x00ec_a5d0;
/// The light cast (`00653290`) with its record (`011f5f6c`) and the light offset (`00633b40`).
const LIGHT_CAST: u32 = 0x0065_3290;
const LIGHT_RECORD: u32 = 0x011f_5f6c;
const LIGHT_OFFSET: u32 = 0x0063_3b40;
/// Three floats read for the constants 73 and 74 (`011f4998`) and 2^32 as a float (`01099378`).
const VECTOR: u32 = 0x011f_4998;
const TWO_TO_32: u32 = 0x0109_9378;
/// The results: the extra data not found (0x10), the program refusing the constant (0x50).
const NO_EXTRA_DATA: u32 = 0x10;
const NOT_SET: u32 = 0x50;
const ONE: u32 = 0x3f80_0000;
/// The renderer's world (+0x940), view (+0x980), projection (+0x9c0), view-projection (+0xa00) and inverse view
/// (+0xa40) matrices.
const WORLD: u32 = 0x940;
const VIEW: u32 = 0x980;
const PROJECTION: u32 = 0x9c0;
const VIEW_PROJECTION: u32 = 0xa00;
const INVERSE_VIEW: u32 = 0xa40;

/// `FLD dword` then `FSTP dword` of the float at `at`: its bits, a signalling NaN quieted.
fn copy_float(e: &Engine, at: u32) -> u32 {
    x87_quiet_bits(e.global::<u32>(at))
}

/// The renderer's matrix at `offset` copied to `to` (`REP MOVSD`), as the draw's own state holds it.
fn copy_renderer_matrix(e: &mut Engine, renderer: u32, offset: u32, to: u32) {
    let bytes = capture::own_bytes(e, renderer.wrapping_add(offset), 0x40);
    e.mem.write(to, &bytes);
}

/// The matrix at `matrix` inverted in place (no determinant) when `inverse`, then transposed in place when
/// `transpose`.
fn invert_transpose(e: &mut Engine, matrix: u32, inverse: bool, transpose: bool) {
    if inverse {
        e.call(MATRIX_INVERSE, &args![matrix, 0u32, matrix]);
    }
    if transpose {
        e.call(MATRIX_TRANSPOSE, &args![matrix, matrix]);
    }
}

/// Four float bit patterns at `at`.
fn store_vector(e: &mut Engine, at: u32, values: [u32; 4]) {
    for (i, value) in (0u32..).zip(values) {
        e.set_global(at.wrapping_add(4 * i), value);
    }
}

/// A 32-bit dimension as `FILD` takes it (2^32 added, `FADD` of the float at `01099378`, when negative as a signed
/// word), stored as a float.
fn unsigned_to_float(e: &Engine, word: u32) -> u32 {
    #[allow(clippy::cast_possible_wrap)]
    let signed = word as i32;
    let mut value = f64::from(signed);
    if signed < 0 {
        value = x87_add(value, x87_m32(e.global::<u32>(TWO_TO_32)));
    }
    x87_store32(value)
}

/// A CRT helper on the float `time` (`FLD`, the call with the value in ST0, its result stored as a float, loaded and
/// stored again): the result's bits.
fn crt_on(e: &mut Engine, helper: u32, time: u32) -> u32 {
    let result = e.call(helper, &args![x87_load32(time)]).f64();
    x87_quiet_bits(x87_store32(result))
}

// Typed from 00e83db0
/// `NiD3DShaderConstantMap::SetDefinedConstant`, `thiscall` with eleven words (the sixth and the last three not read):
/// the shader program, the constant map entry, an object with extra data, two words for the skinning constants, the
/// property state and the dynamic effect state. The entry's predefined constant (+0x10) picks the data handed to the
/// program's `SetShaderConstant` (its slot 0x74: the entry, the data, 0); answers 0, or 0x50 when the program answers
/// false. The renderer (`this` +0x3c) gives the matrices: 1/2/13/14 its projection, 3/4/15/16 its view, 5/6/17/18
/// its world, 7/8/19/20 world times view, 9/10/21/22 view times projection (the second and fourth of each four
/// inverted, the third and fourth transposed), 11/12/23/24 `00ee6dfe` of world and view-projection, 25 to 28 the
/// view and 29 to 32 view times projection (with the fourth word), 78/79 the last and third rows of the inverse view;
/// those reads go through the draw's own state. The other constants: the skinning array (33 to 36, slot 0x78), the
/// texture transforms (37 to 68, 88 to 95), the property state's +8 object's floats (73 to 77), the extra data's
/// time through the CRT helpers (82 to 86), the lights' sum (87), the identity (96 to 99), map values (103 to 105),
/// texture sizes (106 to 115); any other answers 0.
#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
pub fn set_defined_constant(
    e: &mut Engine,
    this: Ptr,
    program: u32,
    entry: u32,
    object: u32,
    fourth: u32,
    fifth: u32,
    _sixth: u32,
    properties: u32,
    effects: u32,
    _ninth: u32,
    _tenth: u32,
    _eleventh: u32,
) -> u32 {
    let base = this.addr();
    let id = e.global::<u32>(entry.wrapping_add(0x10));
    // The inverted and transposed members of the group of four from `first`.
    let flags = |first: u32| {
        (
            id == first + 1 || id == first + 3,
            id == first + 2 || id == first + 3,
        )
    };
    let renderer = |e: &Engine| e.global::<u32>(base.wrapping_add(0x3c));
    let texturing = |e: &Engine| e.global::<u32>(properties.wrapping_add(0x14));
    let map_word = |e: &Engine, texturing: u32, offset: u32| {
        let maps = e.global::<u32>(texturing.wrapping_add(0x20));
        e.global::<u32>(maps.wrapping_add(offset))
    };
    let entry_index = |e: &Engine| e.global::<u32>(entry.wrapping_add(0x18));
    e.with_stack(0x110, |e, frame| {
        let frame = frame.addr();
        let pair = frame + 0x1c;
        let single = frame + 0x24;
        let vector = frame + 0x30;
        let first_matrix = frame + 0x40;
        let name = frame + 0x8c;
        let second_matrix = frame + 0x90;
        let third_matrix = frame + 0xd0;
        let mut result = 0u32;
        let renderer_matrix = |e: &mut Engine, offset: u32, inverse: bool, transpose: bool| {
            let r = renderer(e);
            copy_renderer_matrix(e, r, offset, first_matrix);
            invert_transpose(e, first_matrix, inverse, transpose);
            first_matrix
        };
        let product = |e: &mut Engine,
                       left: (u32, u32),
                       right: (u32, u32),
                       inverse: bool,
                       transpose: bool| {
            let r = renderer(e);
            copy_renderer_matrix(e, r, left.0, left.1);
            copy_renderer_matrix(e, r, right.0, right.1);
            e.call(MATRIX_MULTIPLY, &args![first_matrix, left.1, right.1]);
            invert_transpose(e, first_matrix, inverse, transpose);
            first_matrix
        };
        let texture_transform = |e: &mut Engine, map: u32, first: u32| {
            let mut matrix = 0;
            let (mut inverse, mut transpose) = (false, false);
            if map != 0 {
                let transform = e.global::<u32>(map.wrapping_add(0xc));
                if transform != 0 {
                    matrix = e.call(UPDATE_MATRIX, &args![transform]).u32();
                    (inverse, transpose) = flags(first);
                }
            }
            e.call(
                SETUP_TEXTURE_TRANSFORM,
                &args![
                    second_matrix,
                    matrix,
                    u32::from(inverse),
                    u32::from(transpose),
                    4u32
                ],
            );
            second_matrix
        };
        let slot_map = |e: &Engine, offset: u32| {
            let texturing = texturing(e);
            if texturing == 0 {
                0
            } else {
                map_word(e, texturing, offset)
            }
        };
        let data = match id {
            1 | 2 | 13 | 14 => renderer_matrix(e, PROJECTION, id == 2 || id == 14, id >= 13),
            3 | 4 | 15 | 16 => renderer_matrix(e, VIEW, id == 4 || id == 16, id >= 15),
            5 | 6 | 17 | 18 => renderer_matrix(e, WORLD, id == 6 || id == 18, id >= 17),
            7 | 8 | 19 | 20 => product(
                e,
                (WORLD, second_matrix),
                (VIEW, third_matrix),
                id == 8 || id == 20,
                id >= 19,
            ),
            9 | 10 | 21 | 22 => product(
                e,
                (VIEW, third_matrix),
                (PROJECTION, second_matrix),
                id == 10 || id == 22,
                id >= 21,
            ),
            11 | 12 | 23 | 24 => {
                let r = renderer(e);
                // World to view-projection's end: one image of the renderer's matrices for the call.
                capture::own_image_read(
                    e,
                    r.wrapping_add(WORLD),
                    VIEW_PROJECTION + 0x40 - WORLD,
                    |e, at| {
                        e.call(
                            MATRIX_PRODUCT,
                            &args![second_matrix, at, at + (VIEW_PROJECTION - WORLD)],
                        );
                    },
                );
                second_matrix
            }
            25..=28 => {
                if fourth == 0 {
                    return 0;
                }
                let (inverse, transpose) = flags(0x19);
                renderer_matrix(e, VIEW, inverse, transpose)
            }
            29..=32 => {
                if fourth == 0 {
                    return 0;
                }
                let (inverse, transpose) = flags(0x1d);
                product(
                    e,
                    (VIEW, third_matrix),
                    (PROJECTION, second_matrix),
                    inverse,
                    transpose,
                )
            }
            33..=36 => {
                if fourth == 0 || ((id == 33 || id == 35) && fifth == 0) {
                    return 0;
                }
                let matrices = e.global::<u32>(fourth.wrapping_add(0x28));
                let bones = u32::from(e.global::<u16>(fifth.wrapping_add(0x20)));
                let registers = e.global::<u32>(fourth.wrapping_add(0x20));
                let indices = e.global::<u32>(fifth.wrapping_add(4));
                let set = e
                    .vcall(
                        program,
                        0x78,
                        &args![entry, matrices, bones, registers, indices],
                    )
                    .bool();
                return if set { result } else { NOT_SET };
            }
            37..=40 => texture_transform(e, slot_map(e, 0), 0x25),
            41..=44 => texture_transform(e, slot_map(e, 4), 0x29),
            45..=48 => texture_transform(e, slot_map(e, 8), 0x2d),
            49..=52 => texture_transform(e, slot_map(e, 0xc), 0x31),
            53..=56 => texture_transform(e, slot_map(e, 0x10), 0x35),
            57..=60 => texture_transform(e, slot_map(e, 0x14), 0x39),
            88..=91 => texture_transform(e, slot_map(e, 0x18), 0x58),
            92..=95 => texture_transform(e, slot_map(e, 0x1c), 0x5c),
            61..=64 => {
                let index = entry_index(e);
                let map = slot_map(e, index.wrapping_mul(4).wrapping_add(0x20));
                texture_transform(e, map, 0x3d)
            }
            65..=68 => {
                let texturing = texturing(e);
                let map = if texturing != 0 {
                    let index = entry_index(e);
                    e.call(MAP_AT, &args![texturing, index]).u32()
                } else {
                    0
                };
                texture_transform(e, map, 0x41)
            }
            73..=77 => {
                let source = e.global::<u32>(properties.wrapping_add(8));
                if source == 0 {
                    return 0;
                }
                let at = |offset: u32| source.wrapping_add(offset);
                let values = match id {
                    73 | 74 => [
                        copy_float(e, VECTOR),
                        copy_float(e, VECTOR + 4),
                        copy_float(e, VECTOR + 8),
                        copy_float(e, at(0x3c)),
                    ],
                    75 => [
                        copy_float(e, at(0x1c)),
                        copy_float(e, at(0x20)),
                        copy_float(e, at(0x24)),
                        copy_float(e, at(0x3c)),
                    ],
                    76 => {
                        let held = e.global::<u32>(at(0x34));
                        let from = if held != 0 { held } else { at(0x28) };
                        [
                            copy_float(e, from),
                            copy_float(e, from.wrapping_add(4)),
                            copy_float(e, from.wrapping_add(8)),
                            copy_float(e, at(0x3c)),
                        ]
                    }
                    _ => [copy_float(e, at(0x38)); 4],
                };
                store_vector(e, vector, values);
                vector
            }
            78 | 79 => {
                let r = renderer(e);
                copy_renderer_matrix(e, r, INVERSE_VIEW, first_matrix);
                let row = first_matrix + if id == 78 { 0x30 } else { 0x20 };
                let values = [0u32, 4, 8, 0xc].map(|offset| copy_float(e, row + offset));
                store_vector(e, vector, values);
                vector
            }
            80 | 81 => 0,
            82..=86 => {
                let handle = e.global::<u32>(EXTRA_DATA_NAME);
                let held = e
                    .call(FIXED_STRING_CONSTRUCT, &args![name, handle])
                    .u32();
                let extra = e.call(GET_EXTRA_DATA, &args![object, held]).u32();
                e.call(FIXED_STRING_DESTRUCT, &args![name]);
                if extra == 0 {
                    result = NO_EXTRA_DATA;
                }
                let time = copy_float(e, extra.wrapping_add(0xc));
                e.set_global(pair, time);
                let values = match id - 0x52 {
                    0 => [time; 4],
                    1 => [crt_on(e, CRT_HELPER_A, time); 4],
                    2 => [crt_on(e, CRT_HELPER_B, time); 4],
                    3 => [crt_on(e, CRT_HELPER_C, time); 4],
                    4 => [
                        time,
                        crt_on(e, CRT_HELPER_A, time),
                        crt_on(e, CRT_HELPER_B, time),
                        crt_on(e, CRT_HELPER_C, time),
                    ],
                    _ => return NO_EXTRA_DATA,
                };
                store_vector(e, vector, values);
                vector
            }
            87 => {
                store_vector(e, vector, [0, 0, 0, ONE]);
                let mut node = if effects != 0 {
                    e.global::<u32>(effects.wrapping_add(0xc))
                } else {
                    0
                };
                while node != 0 {
                    let light = e.global::<u32>(node.wrapping_add(4));
                    node = e.global::<u32>(node);
                    if light == 0 {
                        continue;
                    }
                    let found = e.call(LIGHT_CAST, &args![light, LIGHT_RECORD]).u32();
                    if found == 0 {
                        continue;
                    }
                    let strength = copy_float(e, found.wrapping_add(0xc4));
                    e.call(
                        LIGHT_OFFSET,
                        &args![found.wrapping_add(0xc8), single, strength],
                    );
                    for component in 0..3u32 {
                        let at = vector + 4 * component;
                        let sum = x87_add(
                            x87_load32(e.global::<u32>(at)),
                            x87_m32(e.global::<u32>(single + 4 * component)),
                        );
                        e.set_global(at, x87_store32(sum));
                    }
                }
                vector
            }
            96..=99 => {
                if fourth == 0 {
                    return 0;
                }
                for word in 0..16u32 {
                    let value = if word % 5 == 0 { ONE } else { 0 };
                    e.set_global(first_matrix + 4 * word, value);
                }
                let (inverse, transpose) = flags(0x60);
                invert_transpose(e, first_matrix, inverse, transpose);
                first_matrix
            }
            103 => {
                store_vector(e, vector, [ONE; 4]);
                let map = slot_map(e, 0x14);
                if map != 0 {
                    let values = [0x18u32, 0x1c, 0x20, 0x24]
                        .map(|offset| copy_float(e, map.wrapping_add(offset)));
                    store_vector(e, vector, values);
                }
                vector
            }
            104 => {
                e.set_global(single, 0u32);
                e.set_global(single + 4, ONE);
                let map = slot_map(e, 0x14);
                if map != 0 {
                    let first = copy_float(e, map.wrapping_add(0x14));
                    e.set_global(single, first);
                    let second = copy_float(e, map.wrapping_add(0x10));
                    e.set_global(single + 4, second);
                }
                single
            }
            105 => {
                e.set_global(single, 0u32);
                let map = slot_map(e, 0x1c);
                if map != 0 {
                    let value = copy_float(e, map.wrapping_add(0x10));
                    e.set_global(single, value);
                }
                single
            }
            106..=115 => {
                let texturing = texturing(e);
                if texturing == 0 {
                    return 0;
                }
                let map = match id - 0x6a {
                    slot @ 0..=7 => map_word(e, texturing, 4 * slot),
                    8 => {
                        let index = entry_index(e);
                        map_word(e, texturing, index.wrapping_mul(4).wrapping_add(0x20))
                    }
                    9 => {
                        let index = entry_index(e);
                        e.call(MAP_AT, &args![texturing, index]).u32()
                    }
                    _ => 0,
                };
                e.set_global(pair, 0u32);
                e.set_global(pair + 4, 0u32);
                if map != 0 {
                    let texture = e.global::<u32>(map.wrapping_add(8));
                    if texture != 0 {
                        for (slot, at) in [(0x94u32, pair), (0x98, pair + 4)] {
                            let size = e.vcall(texture, slot, &args![]).u32();
                            e.set_global(single, size);
                            let value = unsigned_to_float(e, size);
                            e.set_global(at, value);
                        }
                    }
                }
                pair
            }
            _ => return 0,
        };
        if !e.vcall(program, 0x74, &args![entry, data, 0u32]).bool() {
            result = NOT_SET;
        }
        result
    })
}
