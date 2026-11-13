//! The shadow light shader's per-geometry constants (`ShadowLightShader::SetupGeometryConstants`,
//! `00b7dab0`), the set-up the batch renderer runs for nearly every lit geometry of the crowded street
//! (about 600 times a frame), with the constant set-ups it calls in the exe's order: the lights, the
//! ambient and emittance colours, the object-to-cube and shadow projection matrices, the eye position,
//! the toggles, the clip planes and the smaller entries. Then the pixel and vertex constant maps upload
//! their entries (`SetShaderConstants`, [`super::constants`], made direct when the map's slot holds it).
//!
//! `SetupGeometryConstants` and each of the sixteen set-ups it calls are registered at their own exe
//! addresses (the family's list in `shader/mod.rs`), so another caller by address reaches the typed one.
//! Inside `SetupGeometryConstants` they are plain Rust calls, as in the translation. Everything they
//! write to the device goes through the same calls in the same order: the D3D9 stream is the
//! translation's.
//!
//! **Floats.** Only the main thread runs these (the thread census; each set-up's one caller is
//! `SetupGeometryConstants`), so their maths is plain `f32` in the exe's order (the family's float
//! rule: [`super`]): the exe's own rounding at the main thread's 24-bit precision. A float the exe
//! copies through the x87 stack is copied as it is.

use super::{accumulator, constants, lighting, state, FAMILY};
use super::{div_d, mul_d, sub_d};
use crate::anim_keys::narrow;
use crate::draw::{capture, packet};
use engine_core::prelude::*;

/// The rendering object and its pass type (`011f91e0`, `011f91e4`), the current pass (`0126f74c`) and
/// the position used without a geometry bound (`011f4288`).
const RENDERING_PASS: u32 = 0x011f_91e0;
const RENDERING_PASS_TYPE: u32 = 0x011f_91e4;
const CURRENT_PASS: u32 = 0x0126_f74c;
const DEFAULT_POSITION: u32 = 0x011f_4288;

/// A loop's small index as a word.
#[allow(clippy::cast_possible_truncation)]
fn idx(i: usize) -> u32 {
    i as u32
}

/// The first word of the rendering pass's (`011f91e0`) +0xc: its first light.
fn rendering_object(e: &Engine) -> u32 {
    let rendering = capture::own_at::<u32>(e, RENDERING_PASS);
    let holder = packet::rendering_pass_lights(e, rendering);
    packet::pass_light_at(e, holder, 0)
}

/// The renderer (`011f4748`), the HDR switch (`011f941e`) and the colour row the lights scale (`011fa0d0`);
/// the D3DX import thunks these set-ups call are the draw family's (`crate::draw::thunks`).
const RENDERER: u32 = 0x011f_4748;
const HDR: u32 = 0x011f_941e;
const COLOUR: u32 = 0x011f_a0d0;

/// The shader's vertex and pixel constant-map entry pointers (`ShadowLightShader::
/// pVertexConstantMapEntry` / `pPixelConstantMapEntry` (Xbox PDB), 22 words each from
/// `011febd8` and `011fec30` on PC).
pub(crate) const VERTEX_ENTRIES: u32 = 0x011f_ebd8;
pub(crate) const PIXEL_ENTRIES: u32 = 0x011f_ec30;

/// Whether the constant-map entry the pointer at `slot` holds is enabled (`m_bEnabled`, +8).
pub(crate) fn entry_enabled(e: &Engine, slot: u32) -> bool {
    let entry = capture::own_at::<u32>(e, slot); // own: an own slot handed in (its callers name the block)
    e.global::<u8>(entry.wrapping_add(8)) != 0 // own: main's object read in place (value packet B, piece 4)
}

/// A float the exe copies through the x87 stack (`FLD dword`, `FSTP dword`): its bits, as they are.
#[inline(always)]
fn copy_float(e: &Engine, addr: u32) -> u32 {
    capture::own_at::<u32>(e, addr) // own: a generic accessor, its callers' addresses are checked
}

/// The float at `addr`.
#[inline(always)]
fn float_at(e: &Engine, addr: u32) -> f32 {
    capture::own_at::<f32>(e, addr) // own: a generic accessor, its callers' addresses are checked
}

/// CRT `_memcpy` (`00ec44d0`, `cdecl`).
const MEMCPY: u32 = 0x00ec_44d0;
/// The 4x4 matrix `00b79050` makes the identity (`011fec88`).
const MATRIX: u32 = 0x011f_ec88;

// Typed from 00b79050
/// Unnamed in the engine map, `thiscall` with one word. With vertex
/// entry 12 enabled and the word not null: an identity matrix built on the stack (zeros from
/// `FLDZ`, ones from `FLD1`) is copied to `011fec88` (`memcpy`, 0x40 bytes).
/// `this` (`ECX`) is not read, but `SetupGeometryConstants` (`00b7dab0`) calls it with `ECX` set.
pub fn vertex_entry_12(e: &mut Engine, _this: Ptr, object: u32) {
    if !entry_enabled(e, VERTEX_ENTRIES + 4 * 12) || object == 0 {
        return;
    }
    e.with_stack(0x40, |e, matrix| {
        let matrix = matrix.addr();
        for index in 0..16u32 {
            let value = if index % 5 == 0 { 1.0f32 } else { 0.0 };
            capture::set_own_at::<u32>(e, matrix + 4 * index, value.to_bits());
        }
        capture::own_image(e, MATRIX, 0x40, |e, at| {
            e.call(MEMCPY, &args![at, matrix, 0x40u32]);
        });
    });
}

/// The game time the shaders animate with (`011f940c`, the packet's: `packet::time_value`), `3600.0` (`double`, `01012640`), the
/// `double` `6.28318` (`0104eea0`) and the angle `00b790d0` stores (`011fd868`).
const SECONDS_PER_HOUR: u32 = 0x0101_2640;
const TWO_PI: u32 = 0x0104_eea0;
const ANGLE: u32 = 0x011f_d868;

// Typed from 00b790d0
/// Unnamed in the engine map, `thiscall` with one word, an object
/// with an integer at +0xe4. With vertex entry 15 enabled: `011fd868` = game time (`011f940c`)
/// / 3600.0 * 6.28318 * the integer (in the exe's order, each step rounded to a float).
/// `this` (`ECX`) is not read, but `SetupGeometryConstants` (`00b7dab0`) calls it with `ECX` set.
pub fn vertex_entry_15(e: &mut Engine, _this: Ptr, object: Ptr) {
    if !entry_enabled(e, VERTEX_ENTRIES + 4 * 15) {
        return;
    }
    let time = f32::from_bits(packet::time_value(e, 0));
    let value = div_d(time, e.global::<f64>(SECONDS_PER_HOUR)); // own: a constant of the exe
    let value = mul_d(value, e.global::<f64>(TWO_PI)); // own: a constant of the exe
    let count = e.global::<u32>(object.addr() + 0xe4) as i32; // own: main's object read in place (value packet B, piece 4)
    let value = mul_d(value, f64::from(count));
    capture::set_own_at(e, ANGLE, value);
}

/// The camera position the shaders subtract (`011f474c`, three floats), the `double` 15.0
/// (`01015a38`) and the four floats `00b79100` reads (`011f95f4`) and writes (`011fa2e0`).
const CAMERA: u32 = 0x011f_474c;
const FIFTEEN: u32 = 0x0101_5a38;
const SOURCE: u32 = 0x011f_95f4;
const TARGET: u32 = 0x011f_a2e0;

// Typed from 00b79100
/// Unnamed in the engine map, `thiscall` with one word it does not
/// read. With vertex entry 16 enabled: the four floats at `011fa2e0` become those at
/// `011f95f4` less the camera's x and y (`011f474c`) and less 15.0 twice (the last as `FSUBR`).
/// `this` (`ECX`) is not read, but `SetupGeometryConstants` (`00b7dab0`) calls it with `ECX` set.
pub fn vertex_entry_16(e: &mut Engine, _this: Ptr, _unused_0: u32) {
    if !entry_enabled(e, VERTEX_ENTRIES + 4 * 16) {
        return;
    }
    for index in 0..2u32 {
        let source = e.global::<f32>(SOURCE + 4 * index); // own: main's frame value, value packet B
        let camera = e.global::<f32>(CAMERA + 4 * index); // own: main's frame value, value packet B
        capture::set_own_at(e, TARGET + 4 * index, source - camera);
    }
    let third = e.global::<f32>(SOURCE + 8); // own: main's frame value, value packet B
    let fifteen = e.global::<f64>(FIFTEEN); // own: a constant of the exe
    capture::set_own_at(e, TARGET + 8, sub_d(third, fifteen));
    let fourth = e.global::<f32>(SOURCE + 0xc); // own: main's frame value, value packet B
    capture::set_own_at(e, TARGET + 0xc, sub_d(fourth, fifteen));
}

/// The three floats `00b79180` writes (`011fd8b4`).
const OFFSET: u32 = 0x011f_d8b4;

// Typed from 00b79180
/// Unnamed in the engine map, `thiscall` with one word, an object
/// whose +0x20 points at a position (`011f4288` when null). With vertex entry 17 enabled: the
/// position less the camera (`011f474c`), each rounded to a float on the stack and copied to
/// `011fd8b4`.
/// `this` (`ECX`) is not read, but `SetupGeometryConstants` (`00b7dab0`) calls it with `ECX` set.
pub fn vertex_entry_17(e: &mut Engine, _this: Ptr, object: Ptr) {
    if !entry_enabled(e, VERTEX_ENTRIES + 4 * 17) {
        return;
    }
    let mut position = e.global::<u32>(object.addr() + 0x20); // own: main's object read in place (value packet B, piece 4)
    if position == 0 {
        position = DEFAULT_POSITION;
    }
    let mut values = [0f32; 3];
    for (index, value) in values.iter_mut().enumerate() {
        let offset = 4 * idx(index);
        *value = float_at(e, position + offset) - e.global::<f32>(CAMERA + offset);
        // own: main's frame value, value packet B
    }
    for (index, value) in values.iter().enumerate() {
        capture::set_own_at(e, OFFSET + 4 * idx(index), *value);
    }
}

/// The four floats `00b791f0` writes (`011fa0b0`), the float it uses for type 0xfe
/// (`011ad808`), the times and scale of its type 0x230 (`011f94ac`, `011ad838`; `double` 5.0
/// at `01020998`), the two pairs it lerps between (`011f962c`/`011f9634` and
/// `011f9630`/`011f9638`), the byte it clears when the factor is 1.0 (`011f94aa`), and the
/// float max / min helpers (`00404010`, `0040ebd0`; `cdecl`, two floats, the result in ST0).
const VECTOR: u32 = 0x011f_a0b0;
const VALUE_DEFAULT: u32 = 0x011a_d808;
const DURATION: u32 = 0x011a_d838;
const FIVE: u32 = 0x0102_0998;
const TO_X: u32 = 0x011f_962c;
const TO_Y: u32 = 0x011f_9630;
const FROM_X: u32 = 0x011f_9634;
const FROM_Y: u32 = 0x011f_9638;
const RUNNING: u32 = 0x011f_94aa;

// Typed from 00b791f0
/// Unnamed in the engine map, `thiscall` with three words: the pass
/// type and two objects. With vertex entry 21 enabled, `011fa0b0` (four floats) becomes for
/// type 0xfe (the first object's +0x80, `011ad808`, 0, 0); for 0x61 (1, 0, 0, 0); for 0x230 the
/// first object's +0x94 to +0xa0 with the last two replaced by the lerps between the pairs at
/// `011f9634`/`011f962c` and `011f9638`/`011f9630` by the factor min(1, max(0, (time -
/// `011f94ac`) / (`011ad838` * 5))) less the second object's +0x8c and +0x90 (the factor 1.0
/// clears `011f94aa`). Every step and copy is in the exe's order.
/// `this` (`ECX`) is not read, but `SetupGeometryConstants` (`00b7dab0`) calls it with `ECX` set.
pub fn vertex_entry_21(e: &mut Engine, _this: Ptr, pass_type: u32, first: Ptr, second: Ptr) {
    if !entry_enabled(e, VERTEX_ENTRIES + 4 * 21) {
        return;
    }
    let first = first.addr();
    let second = second.addr();
    match pass_type {
        0xfe => {
            let x = e.global::<u32>(first + 0x80); // own: main's object read in place (value packet B, piece 4)
            capture::set_own_at(e, VECTOR, x);
            let y = e.global::<u32>(VALUE_DEFAULT); // own: a constant of the exe
            capture::set_own_at(e, VECTOR + 4, y);
            capture::set_own_at(e, VECTOR + 8, 0u32);
            capture::set_own_at(e, VECTOR + 0xc, 0u32);
        }
        0x61 => {
            capture::set_own_at(e, VECTOR, 1.0f32.to_bits());
            for offset in [4u32, 8, 0xc] {
                capture::set_own_at(e, VECTOR + offset, 0u32);
            }
        }
        0x230 => {
            let elapsed =
                f32::from_bits(packet::time_value(e, 0)) - f32::from_bits(packet::start_time(e));
            let scale = mul_d(
                e.global::<f32>(DURATION), // own: main's frame value, value packet B
                e.global::<f64>(FIVE),     // own: a constant of the exe
            );
            let x_offset = e.global::<f32>(second + 0x8c); // own: main's object read in place (value packet B, piece 4)
            let y_offset = e.global::<f32>(second + 0x90); // own: main's object read in place (value packet B, piece 4)
                                                           // The exe also reads +0x94 into a stack slot nothing reads.
            let _unread = e.global::<u32>(second + 0x94); // own: main's object read in place (value packet B, piece 4)
            let ratio = elapsed / scale;
            // The larger of 0 and the ratio (`00404010`), then the smaller of 1 and that (`0040ebd0`):
            // `cdecl(a, b)`, each answering in ST0.
            let raised: f32 = typed_call!(
                crate::draw_f::FAMILY,
                0x0040_4010,
                crate::draw_f::helpers::float_max(e, 0.0, ratio)
            );
            let factor: f32 = typed_call!(
                crate::draw_f::FAMILY,
                0x0040_ebd0,
                crate::draw_f::helpers::float_min(e, 1.0, raised)
            );
            let span_x = e.global::<f32>(TO_X) - e.global::<f32>(FROM_X); // own: main's frame value, value packet B
            let span_y = e.global::<f32>(TO_Y) - e.global::<f32>(FROM_Y); // own: main's frame value, value packet B
            let scaled_x = span_x * factor;
            let scaled_y = span_y * factor;
            let lerp_x = scaled_x + e.global::<f32>(FROM_X); // own: main's frame value, value packet B
            let lerp_y = scaled_y + e.global::<f32>(FROM_Y); // own: main's frame value, value packet B
            if factor == 1.0 {
                capture::set_own_at(e, RUNNING, 0u8);
            }
            for (index, offset) in [0x94u32, 0x98, 0x9c, 0xa0].iter().enumerate() {
                let word = e.global::<u32>(first + offset); // own: main's object read in place (value packet B, piece 4)
                capture::set_own_at(e, VECTOR + 4 * idx(index), word);
            }
            capture::set_own_at(e, VECTOR + 8, lerp_x - x_offset);
            capture::set_own_at(e, VECTOR + 0xc, lerp_y - y_offset);
        }
        _ => {}
    }
}

/// The rows of four floats `00b79410` and `00b7c580` write (`011fa0c0`, 0x10 bytes each).
const ROWS: u32 = 0x011f_a0c0;

// Typed from 00b79410
/// Unnamed in the engine map, `thiscall` with two words: the pass
/// type and an object (bytes at +9 and +0xb). Types 0x93 and 0x1f1 to 0x1f5 use rows 2 and 3
/// of `011fa0c0` and set pixel entry 2's register count (+0x20) to max(3, byte +9 + 2,
/// signed); type 0x1f6 uses rows 0 and 0x1b; others leave everything. With the selector byte
/// +0xb the first row becomes (1 for 1, 1 for 2, 1 for 3, 1 for 4, else 0 each) and the second
/// (selector 5, 6, 7, 8), as the x87 stack juggling of 0.0 and 1.0 in the exe leaves them.
/// `this` (`ECX`) is not read, but `SetupGeometryConstants` (`00b7dab0`) calls it with `ECX` set.
pub fn pixel_entry_2(e: &mut Engine, _this: Ptr, pass_type: u32, object: Ptr) {
    let object = object.addr();
    let (first_row, second_row);
    if pass_type > 0x1f5 {
        if pass_type != 0x1f6 {
            return;
        }
        first_row = 0u32;
        second_row = 0x1bu32;
    } else {
        if pass_type < 0x1f1 && pass_type != 0x93 {
            return;
        }
        first_row = 2;
        second_row = 3;
        let mut count = u32::from(packet::rendering_pass_byte9(e, object)) + 2;
        if (count as i32) <= 3 {
            count = 3;
        }
        let entry = capture::own_at::<u32>(e, PIXEL_ENTRIES + 4 * 2);
        capture::set_own_at::<u32>(e, entry + 0x20, count);
    }
    let selector = u32::from(e.global::<u8>(object + 0xb)); // own: main's object read in place (value packet B, piece 4)
    let flag = |wanted: u32| {
        if selector == wanted {
            1.0f32.to_bits()
        } else {
            0
        }
    };
    let first = ROWS + first_row * 0x10;
    for (index, wanted) in [1u32, 2, 3, 4].iter().enumerate() {
        capture::set_own_at::<u32>(e, first + 4 * idx(index), flag(*wanted));
    }
    let second = ROWS.wrapping_add(second_row * 0x10);
    for (index, wanted) in [5u32, 6, 7, 8].iter().enumerate() {
        capture::set_own_at::<u32>(e, second + 4 * idx(index), flag(*wanted));
    }
}

/// `UpdateTogglesConstant` as the shadow light shader's vtable +0x154 holds it (`00c02d90`, this
/// family's `set_tint`), the fog property index byte (`011f91c4`), the three floats
/// `SetupGeometryConstants_Toggles` writes (`011fa2d4`, `011fa2d8`, `011fa2dc`), `1.0` and `255.0`
/// (`double`, `01012070`, `0101e568`) and the float it uses for bit 0 (`010af2f0`). The land toggles
/// (`00c02c30`) and the fog property (`00b55520`) are the draw family's.
const UPDATE_TOGGLES: u32 = 0x00c0_2d90;
const TOGGLE_FIRST: u32 = 0x011f_a2d4;
const TOGGLE_SECOND: u32 = 0x011f_a2d8;
const TOGGLE_THIRD: u32 = 0x011f_a2dc;
const ONE: u32 = 0x0101_2070;
const BYTE_SCALE: u32 = 0x0101_e568;
const TOGGLE_VALUE: u32 = 0x010a_f2f0;

// Typed from 00b795b0
/// `ShadowLightShader::SetupGeometryConstants_Toggles` (engine map), `thiscall` with six stack
/// words: the pass type, a geometry word, the shader property, a fog word, an object with a
/// byte at +8 and a property with flags at +0x18. With pixel entry 4 enabled:
/// `UpdateTogglesConstantLand(geometry, 0, property, 0)` for types 0x1f7 to 0x22e, else the
/// shader's `UpdateTogglesConstant` (vtable +0x154) with the geometry word, the fog property
/// (`GetFogProperty` of the byte at `011f91c4`) and the fog word. A fifth word with its byte +8
/// clear zeroes `011fa2d4`. Without bit 0x4000 of the shader property's +0x20 and outside the
/// types 0x248 to 0x24b, `011fa2dc` becomes max(1, byte +0x1a) / 255 when bit 9 of the flags is
/// set, else `010af2f0` with bit 0 and 0.0 without; otherwise, for the types 0x248 to 0x24b,
/// 1.0 or 0.0 by the property's vtable +0xf4 (5, 0), and `011fa2d8` its +0x30.
#[allow(clippy::too_many_arguments)]
pub fn toggles(
    e: &mut Engine,
    this: Ptr,
    pass_type: u32,
    geometry: u32,
    property: Ptr,
    fog: u32,
    object: u32,
    flags_property: Ptr,
) {
    if !entry_enabled(e, PIXEL_ENTRIES + 4 * 4) {
        return;
    }
    let base = this.addr();
    let property = property.addr();
    if pass_type.wrapping_sub(0x1f7) <= 0x37 {
        typed_call!(
            crate::draw::FAMILY,
            0x00c0_2c30,
            crate::draw::setups::land_toggles(e, Ptr::new(base), geometry, 0u32, property, 0u32)
        );
    } else {
        let vtable = e.global::<u32>(base); // own: main's object read in place (value packet B, piece 4)
        let index = u32::from(packet::fog_index(e));
        let fog_property: u32 = typed_call!(
            crate::draw::FAMILY,
            0x00b5_5520,
            crate::draw::passes::fog_property(e, index)
        );
        let target = e.global::<u32>(vtable + 0x154); // own: main's object read in place (value packet B, piece 4)
        if target == UPDATE_TOGGLES {
            typed_call!(
                FAMILY,
                0x00c0_2d90,
                accumulator::set_tint(e, base, Ptr::new(geometry), fog_property, fog)
            );
        } else {
            e.call(target, &args![base, geometry, fog_property, fog]);
        }
    }
    if object != 0 && e.global::<u8>(object + 8) == 0 {
        // own: main's object read in place (value packet B, piece 4)
        capture::set_own_at(e, TOGGLE_FIRST, 0u32);
    }
    let land = e.global::<u32>(property + 0x20) & 0x4000 != 0; // own: main's object read in place (value packet B, piece 4)
    if !land && !(0x248..=0x24b).contains(&pass_type) {
        let flags_property = flags_property.addr();
        let flags = e.global::<u16>(flags_property + 0x18); // own: main's object read in place (value packet B, piece 4)
        if (flags >> 9) & 1 != 0 {
            let mut value = f32::from(e.global::<u8>(flags_property + 0x1a)); // own: main's object read in place (value packet B, piece 4)
            if f64::from(value) < e.global::<f64>(ONE) {
                // own: a constant of the exe
                value = 1.0;
            }
            capture::set_own_at(
                e,
                TOGGLE_THIRD,
                div_d(value, e.global::<f64>(BYTE_SCALE)), // own: a constant of the exe
            );
        } else if e.global::<u8>(flags_property + 0x18) & 1 != 0 {
            // own: main's object read in place (value packet B, piece 4)
            let value = e.global::<u32>(TOGGLE_VALUE); // own: a constant of the exe
            capture::set_own_at(e, TOGGLE_THIRD, value);
        } else {
            capture::set_own_at(e, TOGGLE_THIRD, 0u32);
        }
        return;
    }
    if pass_type.wrapping_sub(0x248) > 3 {
        return;
    }
    let lit = e.vcall(property, 0xf4, &args![5u32, 0u32]).u32();
    let value = if lit != 0 { 1.0f32.to_bits() } else { 0 };
    capture::set_own_at(e, TOGGLE_THIRD, value);
    let word = packet::property_word(e, property, 0x30);
    capture::set_own_at(e, TOGGLE_SECOND, word);
}

/// The two floats `00b796f0` writes (`011fd944`, `011fd948`) and its float for bit 4
/// (`010177e8`).
const VALUE_FIRST: u32 = 0x011f_d944;
const VALUE_SECOND: u32 = 0x011f_d948;
const VALUE_CONSTANT: u32 = 0x0101_77e8;

// Typed from 00b796f0
/// Unnamed in the engine map, `thiscall` with one word, an object.
/// With pixel entry 16 enabled: `011fd944` takes its +0xe0, `011fd948` the float at `010177e8`
/// when bit 4 of its byte +0x24 is set, else 1.0.
/// `this` (`ECX`) is not read, but `SetupGeometryConstants` (`00b7dab0`) calls it with `ECX` set.
pub fn pixel_entry_16(e: &mut Engine, _this: Ptr, object: Ptr) {
    if !entry_enabled(e, PIXEL_ENTRIES + 4 * 16) {
        return;
    }
    let object = object.addr();
    let value = e.global::<u32>(object + 0xe0); // own: main's object read in place (value packet B, piece 4)
    capture::set_own_at(e, VALUE_FIRST, value);
    let value = if e.global::<u8>(object + 0x24) & 0x10 != 0 {
        // own: main's object read in place (value packet B, piece 4)
        e.global::<u32>(VALUE_CONSTANT) // own: a constant of the exe
    } else {
        1.0f32.to_bits()
    };
    capture::set_own_at(e, VALUE_SECOND, value);
}

/// The plane `00b79730` reads (`011f9604`, four floats), the vector it dots it with
/// (`011fa2a0`), the two distances it compares (`011f9614`, `011f9618`), its `-1.0`
/// (`01012054`) and what it writes (`011fd8e4`, four floats; `011fd8c4`, four floats).
const PLANE: u32 = 0x011f_9604;
const EYE_POSITION: u32 = 0x011f_a2a0;
const NEAR: u32 = 0x011f_9614;
const FAR: u32 = 0x011f_9618;
const MINUS_ONE: u32 = 0x0101_2054;
const PLANE_CONSTANT: u32 = 0x011f_d8e4;
const RANGE: u32 = 0x011f_d8c4;

// Typed from 00b79730
/// Unnamed in the engine map, no arguments. With pixel entry 19
/// enabled: `011fd8e4` takes the plane's first three floats (`011f9604`) and as the fourth the
/// dot product with `011fa2a0` ((y y' + x x') + z z', in that order) less the plane's fourth;
/// `011fd8c4` becomes (0, 0, -1.0, 0) when both distances (`011f9618`, `011f9614`) are 0, else
/// (far, far - near, 1.0, 1.0).
pub fn pixel_entry_19(e: &mut Engine) {
    if !entry_enabled(e, PIXEL_ENTRIES + 4 * 19) {
        return;
    }
    let vector: [f32; 3] = std::array::from_fn(|index| float_at(e, EYE_POSITION + 4 * idx(index)));
    let plane: [u32; 4] = std::array::from_fn(|index| e.global::<u32>(PLANE + 4 * idx(index))); // own: main's frame value, value packet B
    let y = f32::from_bits(plane[1]) * vector[1];
    let x = f32::from_bits(plane[0]) * vector[0];
    for (index, word) in plane.iter().take(3).enumerate() {
        capture::set_own_at(e, PLANE_CONSTANT + 4 * idx(index), *word);
    }
    let sum = y + x;
    let z = f32::from_bits(plane[2]) * vector[2];
    let dot = sum + z;
    capture::set_own_at(e, PLANE_CONSTANT + 0xc, dot - f32::from_bits(plane[3]));
    let far = e.global::<u32>(FAR); // own: a constant of the exe
    let near = e.global::<u32>(NEAR); // own: a constant of the exe
    let far_value = f32::from_bits(far);
    let near_value = f32::from_bits(near);
    if far_value == 0.0 && near_value == 0.0 {
        capture::set_own_at(e, RANGE, 0u32);
        capture::set_own_at(e, RANGE + 4, 0u32);
        let minus_one = e.global::<u32>(MINUS_ONE); // own: a constant of the exe
        capture::set_own_at(e, RANGE + 8, minus_one);
        capture::set_own_at(e, RANGE + 0xc, 0u32);
        return;
    }
    capture::set_own_at(e, RANGE, far);
    capture::set_own_at(e, RANGE + 4, far_value - near_value);
    capture::set_own_at(e, RANGE + 8, 1.0f32.to_bits());
    capture::set_own_at(e, RANGE + 0xc, 1.0f32.to_bits());
}

/// The four floats `00b798c0` writes (`011fa300`).
const VECTOR_LAST: u32 = 0x011f_a300;

// Typed from 00b798c0
/// Unnamed in the engine map, `thiscall` with one word, an object.
/// With pixel entry 20 enabled: `011fa300` takes its floats +0xf0, +0xf4, +0xfc, +0xf8 in that
/// order (each copied through the x87 stack).
/// `this` (`ECX`) is not read, but `SetupGeometryConstants` (`00b7dab0`) calls it with `ECX` set.
pub fn pixel_entry_20(e: &mut Engine, _this: Ptr, object: Ptr) {
    if !entry_enabled(e, PIXEL_ENTRIES + 4 * 20) {
        return;
    }
    let object = object.addr();
    for (index, offset) in [0xf0u32, 0xf4, 0xfc, 0xf8].iter().enumerate() {
        let value = e.global::<u32>(object + offset); // own: main's object read in place (value packet B, piece 4)
        capture::set_own_at(e, VECTOR_LAST + 4 * idx(index), value);
    }
}

/// The eye position the shaders use (`011fa2a0`, three floats), the two vectors
/// `SetupGeometryConstants_EyePos` writes (`011fd894`, `011fd8a4`), the point unitize
/// `004a0c10` (unnamed, `thiscall` on the point).
const EYE: u32 = 0x011f_a2a0;
const EYE_VECTOR: u32 = 0x011f_d894;
const EYE_DIRECTION: u32 = 0x011f_d8a4;
const POINT_UNITIZE: u32 = 0x004a_0c10;

// Typed from 00b78ef0
/// `eye_pos` as the exe calls it (`stdcall`, eighteen words: the pass type, the object, the matrix).
pub fn eye_pos_words(e: &mut Engine, words: &[u32]) -> Ret {
    let mut matrix = [0u32; 16];
    matrix.copy_from_slice(&words[2..18]);
    eye_pos(e, words[0], Ptr::new(words[1]), matrix);
    Ret::default()
}

/// `ShadowLightShader::SetupGeometryConstants_EyePos` (engine map), `stdcall` with eighteen
/// words: the pass type, an object (position at +0x24, a float at +0x30) and a 4x4 matrix by
/// value. With vertex entry 9 or pixel entry 5 enabled: for pass types 0x244 to 0x24b the eye
/// (`011fa2a0`) less the object's position, unitized (`004a0c10`), goes to `011fd8a4` with 0, and
/// the eye less the camera (`011f474c`) to `011fd894` with 1.0; for other types the eye
/// transformed by the matrix (`D3DXVec3TransformCoord`) goes to `011fd894` with the object's
/// +0x30.
pub fn eye_pos(e: &mut Engine, pass_type: u32, object: Ptr, matrix: [u32; 16]) {
    if !entry_enabled(e, VERTEX_ENTRIES + 4 * 9) && !entry_enabled(e, PIXEL_ENTRIES + 4 * 5) {
        return;
    }
    let object = object.addr();
    if (0x244..=0x24b).contains(&pass_type) {
        let eye: [f32; 3] = std::array::from_fn(|index| float_at(e, EYE + 4 * idx(index)));
        e.with_stack(0xc, |e, point| {
            let point = point.addr();
            for (index, eye) in (0u32..).zip(eye) {
                let value = eye - e.global::<f32>(object + 0x24 + 4 * index); // own: main's object read in place (value packet B, piece 4)
                capture::set_own_at::<f32>(e, point + 4 * index, value);
            }
            e.call(POINT_UNITIZE, &args![point]);
            for index in 0..3u32 {
                let value = copy_float(e, point + 4 * index);
                capture::set_own_at(e, EYE_DIRECTION + 4 * index, value);
            }
        });
        capture::set_own_at(e, EYE_DIRECTION + 0xc, 0u32);
        let mut shifted = [0f32; 3];
        for (index, value) in shifted.iter_mut().enumerate() {
            *value = eye[index] - e.global::<f32>(CAMERA + 4 * idx(index)); // own: main's frame value, value packet B
        }
        for (index, value) in shifted.iter().enumerate() {
            capture::set_own_at(e, EYE_VECTOR + 4 * idx(index), *value);
        }
        capture::set_own_at(e, EYE_VECTOR + 0xc, 1.0f32.to_bits());
        return;
    }
    e.with_stack(0x58, |e, frame| {
        let frame = frame.addr();
        let vector = frame;
        let transformed = frame + 0xc;
        let by_value = frame + 0x18;
        for (index, word) in matrix.iter().enumerate() {
            capture::set_own_at::<u32>(e, by_value + 4 * idx(index), *word);
        }
        for index in 0..3u32 {
            let value = copy_float(e, EYE + 4 * index);
            capture::set_own_at::<u32>(e, vector + 4 * index, value);
        }
        crate::draw::thunks::call_vec3_transform_coord(e, transformed, vector, by_value);
        for index in 0..3u32 {
            let value = copy_float(e, transformed + 4 * index);
            capture::set_own_at(e, EYE_VECTOR + 4 * index, value);
        }
    });
    let value = e.global::<u32>(object + 0x30); // own: main's object read in place (value packet B, piece 4)
    capture::set_own_at(e, EYE_VECTOR + 0xc, value);
}

/// The matrix `SetupGeometryConstants_ShadowProj` writes (`011fd968`, 16 floats).
/// `BSShaderManager::GetAccumulator` (`00b4f5c0`, engine map; no arguments) is the render family's
/// `shader_accumulator`.
const SHADOW_PROJECTION: u32 = 0x011f_d968;

// Typed from 00b7b930
/// `ShadowLightShader::SetupGeometryConstants_ShadowProj` (engine map), `thiscall` with three
/// words: a block whose +0xc is an object with a register at +0x1c, a flag and a transform (a
/// 3x3 rotation, a translation and a scale, 13 floats). With vertex entry 3 enabled, the object
/// set with a register other than -1, and the accumulator's +0x194 set: `011fd968` becomes,
/// without the flag, the rows (rotation row with translation x, y, z in the fourth column) and
/// (0, 0, 0, scale), each float copied through the x87 stack; with it, the identity with the
/// camera position (`011f474c`) in the fourth column.
/// `this` (`ECX`) is not read, but `SetupGeometryConstants` (`00b7dab0`) calls it with `ECX` set.
pub fn shadow_proj(e: &mut Engine, _this: Ptr, block: Ptr, flag: u32, transform: Ptr) {
    if !entry_enabled(e, VERTEX_ENTRIES + 4 * 3) {
        return;
    }
    let object = e.global::<u32>(block.addr() + 0xc); // own: main's object read in place (value packet B, piece 4)
    if object == 0 || e.global::<u32>(object + 0x1c) == 0xffff_ffff {
        // own: main's object read in place (value packet B, piece 4)
        return;
    }
    let accumulator: u32 = typed_call!(
        crate::render::FAMILY,
        0x00b4_f5c0,
        crate::render::passes::shader_accumulator(e)
    );
    if e.global::<u32>(accumulator + 0x194) == 0 {
        // own: main's object read in place (value packet B, piece 4)
        return;
    }
    let mut matrix = [0u32; 16];
    if flag == 0 {
        let transform = transform.addr();
        let sources = [0u32, 1, 2, 9, 3, 4, 5, 10, 6, 7, 8, 11];
        for (index, source) in sources.iter().enumerate() {
            matrix[index] = e.global::<u32>(transform + 4 * source); // own: the geometry's world transform (cut-taken), read in place: value packet B (#542)
        }
        matrix[15] = e.global::<u32>(transform + 0x30); // own: the geometry's world transform (cut-taken), read in place: value packet B (#542)
    } else {
        let one = 1.0f32.to_bits();
        for index in [0usize, 5, 10, 15] {
            matrix[index] = one;
        }
        for (row, index) in [3usize, 7, 11].iter().enumerate() {
            matrix[*index] = e.global::<u32>(CAMERA + 4 * idx(row)); // own: main's frame value, value packet B
        }
    }
    for (index, word) in matrix.iter().enumerate() {
        capture::set_own_at(e, SHADOW_PROJECTION + 4 * idx(index), *word);
    }
}

/// `NiD3DUtility::GetD3DFromNi` (`00b799a0`, engine map; `cdecl`: matrix, transform) and the matrix
/// `SetupGeometryConstants_ObjToCube` writes (`011fecc8`).
const GET_D3D_FROM_NI: u32 = 0x00b7_99a0;
const OBJECT_TO_CUBE: u32 = 0x011f_ecc8;

// Typed from 00b7b730
/// `ShadowLightShader::SetupGeometryConstants_ObjToCube` (engine map), `thiscall` with four
/// stack words: a block (a byte at +9, an object at +0xc), the pass type, a flag and a
/// transform. With vertex entry 1 enabled: without the block's object, with the byte set and the
/// object's first word's byte +0xed set, or for the pass types 0x244 to 0x24b, 0x2d8 to 0x2ef,
/// 0xfe and 0x10b, `011fecc8` becomes the transpose of the renderer's (+0x18) matrix at +0x940
/// (copied to the stack first). Otherwise, with the rendering pass's +0xc set and the byte set:
/// an identity matrix (or the transform's, `GetD3DFromNi`, without the flag) gets the camera
/// (`011f474c`) added to its translation, is multiplied by the first light's matrix (+0x10 of
/// the first word of the rendering pass's +0xc), transposed in place and copied to `011fecc8`.
pub fn obj_to_cube(
    e: &mut Engine,
    this: Ptr,
    block: Ptr,
    pass_type: u32,
    flag: u32,
    transform: u32,
) {
    if !entry_enabled(e, VERTEX_ENTRIES + 4) {
        return;
    }
    let block = block.addr();
    let object = packet::rendering_pass_lights(e, block);
    let byte = packet::rendering_pass_byte9(e, block);
    let mut direct = object == 0;
    if !direct && byte != 0 && packet::pass_light_at(e, object, 0) != 0 {
        let first = packet::pass_light_at(e, object, 0);
        direct = packet::pass_light_byte(e, first, 0xed) != 0;
    }
    if !direct {
        direct = (0x244..=0x24b).contains(&pass_type)
            || (0x2d8..=0x2ef).contains(&pass_type)
            || pass_type == 0xfe
            || pass_type == 0x10b;
    }
    e.with_stack(0x100, |e, frame| {
        let frame = frame.addr();
        let first = frame;
        let second = frame + 0x40;
        let product = frame + 0x80;
        if direct {
            let renderer = e.global::<u32>(this.addr() + 0x18); // own: main's object read in place (value packet B, piece 4)
            let bytes = e.mem.bytes(renderer + 0x940, 0x40); // own: the renderer's matrix (piece 4) into a local
            e.mem.write(product, &bytes); // own: a local
            capture::own_image(e, OBJECT_TO_CUBE, 0x40, |e, at| {
                crate::draw::thunks::call_matrix_transpose(e, at, product);
            });
            return;
        }
        let rendering = capture::own_at::<u32>(e, RENDERING_PASS);
        if packet::rendering_pass_lights(e, rendering) == 0 || byte == 0 {
            return;
        }
        for matrix in [first, second] {
            for index in 0..16u32 {
                let value = if index % 5 == 0 { 1.0f32 } else { 0.0 };
                capture::set_own_at::<u32>(e, matrix + 4 * index, value.to_bits());
            }
        }
        // GetD3DFromNi reads the geometry's world transform (cut-taken) in place: value packet B (#542).
        if flag == 0 {
            e.call(GET_D3D_FROM_NI, &args![first, transform]);
        }
        for index in 0..3u32 {
            let slot = first + 0x30 + 4 * index;
            let value = e.global::<f32>(CAMERA + 4 * index) + float_at(e, slot); // own: main's frame value, value packet B
            capture::set_own_at::<f32>(e, slot, value);
        }
        let lights = packet::rendering_pass_lights(e, rendering);
        let light = packet::pass_light_at(e, lights, 0);
        // The light's matrix (+0x10), the packet's.
        capture::image(e, light, 0x10, 0x50, |e, base| {
            crate::draw::thunks::call_matrix_multiply(e, product, first, base + 0x10);
        });
        let bytes = e.mem.bytes(product, 0x40); // own: a local
        e.mem.write(second, &bytes); // own: a local
        crate::draw::thunks::call_matrix_transpose(e, second, second);
        capture::own_image(e, OBJECT_TO_CUBE, 0x40, |e, at| {
            e.call(MEMCPY, &args![at, second, 0x40u32]);
        });
    });
}

/// The camera object (`011f91c8`; its position at +0x1e4) and the light vectors the light
/// loops write and upload (`011fd9a8`, 0x10 bytes per light).
const CAMERA_OBJECT: u32 = 0x011f_91c8;
const LIGHT_VECTORS: u32 = 0x011f_d9a8;

/// The six clip planes at `planes` set on the device (renderer `011f4748` +0x288, read for each;
/// its vtable +0xdc with the index and the plane).
fn set_clip_planes(e: &mut Engine, planes: u32) {
    for index in 0..6u32 {
        let renderer = e.global::<u32>(RENDERER); // own: main's frame value, value packet B
        let device = e.global::<u32>(renderer + 0x288); // own: main's object read in place (value packet B, piece 4)
        e.vcall(device, 0xdc, &args![index, planes + 0x10 * index]);
    }
}

// Typed from 00b7be00
/// `ShadowLightShader::SetupGeometryConstants_SimpleShadowClipPlanes` (engine map), `thiscall`
/// with one stack word, the pass type; only the types 0x231 to 0x234 do anything. The first
/// light (the first word of the rendering pass's +0xc) keeps transformed clip planes at +0x1a8
/// and a byte at +0x208 that says they are made; when it is set the device gets them. Else: the
/// renderer's (+0x18) view matrix (+0x980) on the stack gets the translation -(camera . column)
/// for its three columns (the camera `011f474c`, in the exe's order), is multiplied by the
/// renderer's matrix at +0x9c0, inverted and transposed; each of the light's six planes (+0x144;
/// the fourth word negated) is normalized and transformed by it; the device gets the six, they
/// are copied to +0x1a8 and the byte is set.
pub fn simple_shadow_clip_planes(e: &mut Engine, this: Ptr, pass_type: u32) {
    if pass_type.wrapping_sub(0x231) > 3 {
        return;
    }
    let light = rendering_object(e);
    // The light's clip-plane cache (+0x1a8, made +0x208) is draw-written in main's light: piece 4.
    if e.global::<u8>(light + 0x208) != 0 {
        // own: the light's clip-plane cache, piece 4
        set_clip_planes(e, light + 0x1a8);
        return;
    }
    let renderer = e.global::<u32>(this.addr() + 0x18); // own: main's object read in place (value packet B, piece 4)
    e.with_stack(0x1a0, |e, frame| {
        let frame = frame.addr();
        let view = frame;
        let product = frame + 0x40;
        let inverse = frame + 0x80;
        let planes = frame + 0xc0;
        let plane = frame + 0x120;
        let moved = frame + 0x130;
        let bytes = e.mem.bytes(renderer + 0x980, 0x40); // own: the renderer's matrix (piece 4) into a local
        e.mem.write(view, &bytes); // own: a local
        let m = |e: &Engine, index: u32| float_at(e, view + 4 * index);
        let x = e.global::<f32>(CAMERA); // own: main's frame value, value packet B
        let y = e.global::<f32>(CAMERA + 4); // own: main's frame value, value packet B
        let z = e.global::<f32>(CAMERA + 8); // own: main's frame value, value packet B
        let first = y * m(e, 4) + x * m(e, 0);
        let first = first + z * m(e, 8);
        capture::set_own_at::<f32>(e, view + 0x30, -first);
        let second = y * m(e, 5) + x * m(e, 1);
        let second = second + z * m(e, 9);
        capture::set_own_at::<f32>(e, view + 0x34, -second);
        let third = y * m(e, 6) + x * m(e, 2);
        let third = third + z * m(e, 10);
        capture::set_own_at::<f32>(e, view + 0x38, -third);
        let bytes = e.mem.bytes(renderer + 0x9c0, 0x40); // own: the renderer's matrix (piece 4) into a local
        e.mem.write(moved, &bytes); // own: a local
        crate::draw::thunks::call_matrix_multiply(e, product, view, moved);
        crate::draw::thunks::call_matrix_inverse(e, inverse, 0u32, product);
        crate::draw::thunks::call_matrix_transpose(e, product, inverse);
        for index in 0..6u32 {
            let source = 0x144 + 0x10 * index;
            for word in 0..3u32 {
                let value = packet::pass_light_word(e, light, source + 4 * word);
                capture::set_own_at::<u32>(e, plane + 4 * word, value);
            }
            let fourth = f32::from_bits(packet::pass_light_word(e, light, source + 0xc));
            capture::set_own_at::<f32>(e, plane + 0xc, -fourth);
            crate::draw::thunks::call_plane_normalize(e, plane, plane);
            let out = frame + 0x170;
            crate::draw::thunks::call_plane_transform(e, out, plane, product);
            for word in 0..4u32 {
                let value = copy_float(e, out + 4 * word);
                capture::set_own_at::<u32>(e, planes + 0x10 * index + 4 * word, value);
            }
        }
        set_clip_planes(e, planes);
        e.call(MEMCPY, &args![light + 0x1a8, planes, 0x60u32]);
    });
    e.set_global::<u8>(light + 0x208, 1); // own: the light's clip-plane cache, piece 4
}

/// The shader mode the shaders switch on (`011f91b8`), the shader manager's object `00b4f5d0`
/// answers (unnamed, no arguments; its colour at +0x154), the per-light set-up `00b70820`
/// ([`lighting::light_setup`]; `cdecl`: index, light, a float), `BSShaderLightingProperty::GetFirstActiveLight`
/// (`00b70590`, engine map; `thiscall`, one word), the light positions and directions the
/// set-up leaves (`011fa170` and `011fa1f0`, 0x10 bytes per light), `0.5` (`01016248`), the
/// HDR light scale (`011f9184`) and the colour rows `SetupGeometryConstants_Lights` writes
/// (`011fa0d0`).
const SHADER_MODE: u32 = 0x011f_91b8;
const GET_FIRST_ACTIVE_LIGHT: u32 = 0x00b7_0590;
const LIGHT_POSITIONS: u32 = 0x011f_a170;
const LIGHT_DIRECTIONS: u32 = 0x011f_a1f0;
const HALF: u32 = 0x0101_6248;
const HDR_LIGHT_SCALE: u32 = 0x011f_9184;

// Typed from 00b78a90
/// `lights` as the exe calls it (`stdcall`, 21 words: the property, the rendering object, the
/// matrix, the scale object, the pass type, the flag).
pub fn lights_words(e: &mut Engine, words: &[u32]) -> Ret {
    let mut matrix = [0u32; 16];
    matrix.copy_from_slice(&words[2..18]);
    lights(
        e,
        Ptr::new(words[0]),
        Ptr::new(words[1]),
        matrix,
        Ptr::new(words[18]),
        words[19],
        words[20],
    );
    Ret::default()
}

/// `ShadowLightShader::SetupGeometryConstants_Lights` (engine map), `stdcall` with 21 words: the
/// shader property (a float at +0x6c, flags at +0x20), the rendering object (type +4, light count
/// +9, lights +0xc), a 4x4 matrix by value, an object with a scale at +0x30, the pass type and a
/// flag. In shader mode 0xf (`011f91b8`) row 0 of `011fa0c0` takes the manager object's colour
/// (`00b4f5d0` +0x154) and that is all. Else the register counts of vertex entry 6 (1 for types
/// 0x10 to 0x13, else the light count) and pixel entries 2 (the count) and 7 (count - 1) are set
/// when enabled. For each light up to 10, after its set-up (`00b70820` with the property's
/// float): for types 0x10 to 0x13 `011fd9a8` row i takes the position row and as its fourth word
/// the direction row's first; else, for a light with its byte +0xf4 set (a point light), the
/// position plus the camera object's (`011f91c8` +0x1e4) transformed by the matrix, its fourth
/// word the direction row's first, scaled by the object's +0x30 for types 0x231 to 0x233 or the
/// fourth divided by it otherwise (neither with the flag); for another light the negated
/// direction transformed as a normal and normalized, with the direction row's fourth word. With
/// HDR (`011f941e`), bit 0x400 of the property and a first light that is not a point light, row
/// 1 of the colours (`011fa0d0`) is multiplied by `011f9184`. For pass type 0x24b without an
/// active point light (`GetFirstActiveLight`) row 1 becomes (0.5, 0.5, 0.5, 1.0) in shader mode 10,
/// else a copy of row 0.
pub fn lights(
    e: &mut Engine,
    property: Ptr,
    rendering: Ptr,
    matrix: [u32; 16],
    scale_object: Ptr,
    pass_type: u32,
    flag: u32,
) {
    let property = property.addr();
    let rendering = rendering.addr();
    let light_float = e.global::<u32>(property + 0x6c); // own: main's object read in place (value packet B, piece 4)
    if capture::own_at::<u32>(e, SHADER_MODE) == 0xf {
        // The getter `00b4f5d0` reads the held accumulator (`011f95f0`, own state): done here, through the packet.
        let object = packet::held_accumulator(e);
        for index in 0..4u32 {
            let word = e.global::<u32>(object + 0x154 + 4 * index); // own: main's object read in place (value packet B, piece 4)
            capture::set_own_at(e, ROWS + 4 * index, word);
        }
        return;
    }
    let count = u32::from(packet::rendering_pass_byte9(e, rendering));
    let kind = e.global::<u16>(rendering + 4); // own: main's object read in place (value packet B, piece 4)
    let entry = capture::own_at::<u32>(e, VERTEX_ENTRIES + 4 * 6);
    if capture::own_at::<u8>(e, entry + 8) != 0 {
        let registers = if (0x10..=0x13).contains(&kind) {
            1
        } else {
            count
        };
        capture::set_own_at::<u32>(e, entry + 0x20, registers);
    }
    let entry = capture::own_at::<u32>(e, PIXEL_ENTRIES + 4 * 2);
    if capture::own_at::<u8>(e, entry + 8) != 0 {
        capture::set_own_at::<u32>(e, entry + 0x20, count);
    }
    let entry = capture::own_at::<u32>(e, PIXEL_ENTRIES + 4 * 7);
    if capture::own_at::<u8>(e, entry + 8) != 0 {
        capture::set_own_at::<u32>(e, entry + 0x20, count.wrapping_sub(1));
    }
    let mut index: u8 = 0;
    if packet::rendering_pass_byte9(e, rendering) > 0 {
        e.with_stack(0x80, |e, frame| {
            let frame = frame.addr();
            let by_value = frame;
            let vector = frame + 0x40;
            let out = frame + 0x50;
            for (word, value) in matrix.iter().enumerate() {
                capture::set_own_at::<u32>(e, by_value + 4 * idx(word), *value);
            }
            loop {
                if index >= 10 {
                    break;
                }
                let i = u32::from(index);
                let lights = packet::rendering_pass_lights(e, rendering);
                let light = packet::pass_light_at(e, lights, i);
                typed_call!(
                    FAMILY,
                    0x00b7_0820,
                    lighting::light_setup(e, i, Ptr::new(light), f32::from_bits(light_float))
                );
                let row = LIGHT_VECTORS + 0x10 * i;
                let position = LIGHT_POSITIONS + 0x10 * i;
                let direction = LIGHT_DIRECTIONS + 0x10 * i;
                let kind = e.global::<u16>(rendering + 4); // own: main's object read in place (value packet B, piece 4)
                if (0x10..=0x13).contains(&kind) {
                    for word in 0..4u32 {
                        let value = capture::own_at::<u32>(e, position + 4 * word);
                        capture::set_own_at::<u32>(e, row + 4 * word, value);
                    }
                    let fourth = copy_float(e, direction);
                    capture::set_own_at::<u32>(e, row + 0xc, fourth);
                } else if light != 0 {
                    if packet::pass_light_byte(e, light, 0xf4) != 0 {
                        for word in 0..3u32 {
                            let value = copy_float(e, position + 4 * word);
                            capture::set_own_at::<u32>(e, vector + 4 * word, value);
                        }
                        let camera = e.global::<u32>(CAMERA_OBJECT); // own: main's frame value, value packet B
                        for word in 0..3u32 {
                            let offset = e.global::<f32>(camera + 0x1e4 + 4 * word); // own: main's object read in place (value packet B, piece 4)
                            let value = offset + float_at(e, vector + 4 * word);
                            capture::set_own_at::<f32>(e, vector + 4 * word, value);
                        }
                        crate::draw::thunks::call_vec3_transform_coord(e, out, vector, by_value);
                        let mut fourth = copy_float(e, direction);
                        if flag == 0 {
                            let scale_object = scale_object.addr();
                            let kind = e.global::<u16>(rendering + 4); // own: main's object read in place (value packet B, piece 4)
                            if (0x231..=0x233).contains(&kind) {
                                let scale = e.global::<f32>(scale_object + 0x30); // own: main's object read in place (value packet B, piece 4)
                                for word in 0..3u32 {
                                    let slot = out + 4 * word;
                                    let value = scale * float_at(e, slot);
                                    capture::set_own_at::<f32>(e, slot, value);
                                }
                            } else {
                                let value =
                                    f32::from_bits(fourth) / e.global::<f32>(scale_object + 0x30); // own: main's object read in place (value packet B, piece 4)
                                fourth = value.to_bits();
                            }
                        }
                        for word in 0..3u32 {
                            let value = copy_float(e, out + 4 * word);
                            capture::set_own_at::<u32>(e, row + 4 * word, value);
                        }
                        capture::set_own_at::<u32>(e, row + 0xc, fourth);
                    } else {
                        for word in 0..3u32 {
                            let value = float_at(e, direction + 4 * word);
                            capture::set_own_at::<f32>(e, vector + 4 * word, -value);
                        }
                        crate::draw::thunks::call_vec3_transform_normal(e, out, vector, by_value);
                        crate::draw::thunks::call_vec3_normalize(e, vector, out);
                        for word in 0..3u32 {
                            let value = copy_float(e, vector + 4 * word);
                            capture::set_own_at::<u32>(e, row + 4 * word, value);
                        }
                        let fourth = copy_float(e, direction + 0xc);
                        capture::set_own_at::<u32>(e, row + 0xc, fourth);
                    }
                }
                index = index.wrapping_add(1);
                if index >= packet::rendering_pass_byte9(e, rendering) {
                    break;
                }
            }
        });
    }
    let mut scale_colour = e.global::<u8>(HDR) != 0 // own: main's frame value, value packet B
        && e.global::<u32>(property + 0x20) & 0x400 != 0 // own: main's object read in place (value packet B, piece 4)
        && packet::rendering_pass_byte9(e, rendering) != 0;
    if scale_colour {
        let lights = packet::rendering_pass_lights(e, rendering);
        let first = packet::pass_light_at(e, lights, 0);
        scale_colour = packet::pass_light_byte(e, first, 0xf4) == 0;
    }
    if scale_colour {
        let scale = e.global::<f32>(HDR_LIGHT_SCALE); // own: main's frame value, value packet B
        let red = capture::own_at::<f32>(e, COLOUR);
        let green = capture::own_at::<f32>(e, COLOUR + 4);
        let blue = capture::own_at::<f32>(e, COLOUR + 8);
        let alpha = capture::own_at::<f32>(e, COLOUR + 0xc);
        capture::set_own_at(e, COLOUR, red * scale);
        capture::set_own_at(e, COLOUR + 4, green * scale);
        capture::set_own_at(e, COLOUR + 8, blue * scale);
        capture::set_own_at(e, COLOUR + 0xc, scale * alpha);
    }
    if pass_type != 0x24b {
        return;
    }
    let mode = capture::own_at::<u32>(e, SHADER_MODE);
    e.with_stack(4, |e, slot| {
        let light = e.call(GET_FIRST_ACTIVE_LIGHT, &args![property, slot]).u32();
        if light != 0 && e.global::<u8>(light + 0xf4) != 0 {
            // own: the first active light, read in place: an open packet route (the drawing agent's next PR)
            return;
        }
        if mode == 10 {
            let half = e.global::<u32>(HALF); // own: a constant of the exe
            for offset in [0u32, 4, 8] {
                capture::set_own_at(e, COLOUR + offset, half);
            }
            capture::set_own_at(e, COLOUR + 0xc, 1.0f32.to_bits());
        } else {
            for offset in [0u32, 4, 8, 0xc] {
                let word = capture::own_at::<u32>(e, ROWS + offset);
                capture::set_own_at(e, COLOUR + offset, word);
            }
        }
    });
}

/// The ambient helpers (`stdcall`, one word: the shader's word): `00c03180` with pixel entry 3
/// enabled outside types 0xc3 to 0xd6, else `00c03010` (both unnamed).
const AMBIENT_SET_UP: u32 = 0x00c0_3180;
const ACCUMULATE_LIGHT_COLOR: u32 = 0x00c0_3010;
/// The two floats `SetupGeometryConstants_AmbientEmittance` sets from the light count for
/// types 0xc3 to 0xd6 (`011fd95c`, `011fa27c`), the byte that makes the ambient row white
/// (`011f91a7`), `BSShaderLightingProperty::GetFirstActiveNonShadowLight` (`00b70600`, engine
/// map; `thiscall`, one word), the three luminance weights (`double`s at `010ae8c0`, `010ae8b8`,
/// `010ae8b0` for the second, first and third channel) and the helpers it calls (unnamed):
/// `00439ef0` (`thiscall` on a point: out, other point), `00633b40` (`thiscall` on a colour:
/// out, a float), `00457990` (`thiscall` on a point, the result in ST0), `0068b050` (`cdecl`: a
/// `double` and a float, the result in ST0), `00b719c0` (`cdecl`: two `double`s, the result in
/// ST0) and `00b719e0` (`thiscall` on a colour, the result in ST0).
const LIGHT_COUNT: u32 = 0x011f_d95c;
const LIGHT_COUNT_CONSTANT: u32 = 0x011f_a27c;
const WHITE_AMBIENT: u32 = 0x011f_91a7;
const GET_FIRST_ACTIVE_NON_SHADOW_LIGHT: u32 = 0x00b7_0600;
const WEIGHT_SECOND: u32 = 0x010a_e8c0;
const WEIGHT_FIRST: u32 = 0x010a_e8b8;
const WEIGHT_THIRD: u32 = 0x010a_e8b0;
const POINT_DIFFERENCE: u32 = 0x0043_9ef0;
const COLOUR_SCALE: u32 = 0x0063_3b40;
const POINT_LENGTH: u32 = 0x0045_7990;
const FLOAT_HELPER: u32 = 0x0068_b050;
const DOUBLE_HELPER: u32 = 0x00b7_19c0;
const COLOUR_LUMINANCE: u32 = 0x00b7_19e0;

// Typed from 00b7ba70
/// `ShadowLightShader::SetupGeometryConstants_AmbientEmittance` (engine map), `thiscall` with
/// four words: the shader property (flags at +0x20 and +0x24, a float at +0x28, its vtable
/// +0x110 answering a float), the rendering object (type +4, light count +9), a block whose
/// +0x9c is a property with flags at +0x18, and a word for the ambient helpers. Row 0 of
/// `011fa0c0` keeps its colour (copied) with the property's +0x28 as the fourth word; the ambient
/// helper runs (`00c03180` or `00c03010`); types 0xc3 to 0xd6 put the light count into
/// `011fd95c` and `011fa27c`; `011f91a7` makes the row white. With bit 0x8000 of +0x24, without
/// bit 0x20000 of +0x20 and for rendering types 0x244 to 0x24b, the fourth word is multiplied by
/// the larger of the camera light luminance and, for the first active non-shadow light, its
/// luminance scaled by 1 - min(1, max(0, distance / radius)). In shader mode 8 the row becomes
/// (0, 0, 1 or 0 by the block property bits 0 and 9, 1 or 0 by bit 0x20000 of +0x24). For pass
/// types 0x244 to 0x24b outside mode 10 the fourth word is multiplied by the property vtable
/// +0x110. Every step, copy and comparison follows the exe.
/// `this` (`ECX`) is not read, but `SetupGeometryConstants` (`00b7dab0`) calls it with `ECX` set.
pub fn ambient_emittance(
    e: &mut Engine,
    _this: Ptr,
    property: Ptr,
    rendering: Ptr,
    block: Ptr,
    word: u32,
) {
    let property = property.addr();
    let rendering = rendering.addr();
    let current = capture::own_at::<u32>(e, RENDERING_PASS);
    let colour: [u32; 3] = std::array::from_fn(|index| copy_float(e, ROWS + 4 * idx(index)));
    let alpha = packet::property_word(e, property, 0x28);
    for (index, value) in colour.iter().enumerate() {
        capture::set_own_at(e, ROWS + 4 * idx(index), *value);
    }
    capture::set_own_at(e, ROWS + 0xc, alpha);
    let kind = e.global::<u16>(rendering + 4); // own: main's object read in place (value packet B, piece 4)
    let helper = if entry_enabled(e, PIXEL_ENTRIES + 4 * 3) && !(0xc3..=0xd6).contains(&kind) {
        AMBIENT_SET_UP
    } else {
        ACCUMULATE_LIGHT_COLOR
    };
    if helper == ACCUMULATE_LIGHT_COLOR {
        typed_call!(
            FAMILY,
            0x00c0_3010,
            accumulator::accumulate_light_color(e, Ptr::new(word))
        );
    } else {
        e.call(helper, &args![word]);
    }
    let kind = e.global::<u16>(rendering + 4); // own: main's object read in place (value packet B, piece 4)
    if (0xc3..=0xd6).contains(&kind) {
        let count = f32::from(packet::rendering_pass_byte9(e, rendering));
        capture::set_own_at(e, LIGHT_COUNT, count);
        let count = f32::from(packet::rendering_pass_byte9(e, rendering));
        capture::set_own_at(e, LIGHT_COUNT_CONSTANT, count);
    }
    if e.global::<u8>(WHITE_AMBIENT) != 0 {
        // own: main's frame value, value packet B
        for offset in [0u32, 4, 8, 0xc] {
            capture::set_own_at(e, ROWS + offset, 1.0f32.to_bits());
        }
    }
    let current_kind = capture::own_at::<u16>(e, current + 4);
    if packet::property_word(e, property, 0x24) & 0x8000 != 0
        && e.global::<u32>(property + 0x20) & 0x20000 == 0 // own: main's object read in place (value packet B, piece 4)
        && (0x244..=0x24b).contains(&current_kind)
    {
        e.with_stack(0x24, |e, frame| {
            let frame = frame.addr();
            let slot = frame;
            let difference = frame + 4;
            let scaled = frame + 0x10;
            let light = e
                .call(GET_FIRST_ACTIVE_NON_SHADOW_LIGHT, &args![property, slot])
                .u32();
            let camera = e.global::<u32>(CAMERA_OBJECT); // own: main's frame value, value packet B
            let camera_light = e.global::<u32>(camera + 0xe0); // own: main's object read in place (value packet B, piece 4)
            let values = e.global::<u32>(camera_light + 0xf8); // own: the first active (or camera) light, read in place: an open packet route (the drawing agent's next PR)
            let second = mul_d(
                e.global::<f32>(values + 0xd8), // own: the first active (or camera) light, read in place: an open packet route (the drawing agent's next PR)
                e.global::<f64>(WEIGHT_SECOND), // own: a constant of the exe
            );
            let first = mul_d(
                e.global::<f32>(values + 0xd4), // own: the first active (or camera) light, read in place: an open packet route (the drawing agent's next PR)
                e.global::<f64>(WEIGHT_FIRST),  // own: a constant of the exe
            );
            let sum = second + first;
            let third = mul_d(
                e.global::<f32>(values + 0xdc), // own: the first active (or camera) light, read in place: an open packet route (the drawing agent's next PR)
                e.global::<f64>(WEIGHT_THIRD),  // own: a constant of the exe
            );
            let luminance = sum + third;
            let mut light_part = 0.0f32;
            if light != 0 {
                let current = capture::own_at::<u32>(e, RENDERING_PASS);
                let first_word = capture::own_at::<u32>(e, current);
                let mut position = capture::own_at::<u32>(e, first_word + 0x20);
                if position == 0 {
                    position = DEFAULT_POSITION;
                }
                let values = e.global::<u32>(light + 0xf8); // own: the first active (or camera) light, read in place: an open packet route (the drawing agent's next PR)
                e.call(
                    POINT_DIFFERENCE,
                    &args![values + 0x8c, difference, position],
                );
                let strength = e.global::<u32>(values + 0xc4); // own: the first active (or camera) light, read in place: an open packet route (the drawing agent's next PR)
                e.call(COLOUR_SCALE, &args![values + 0xd4, scaled, strength]);
                let length = e.call(POINT_LENGTH, &args![difference]).f64();
                let ratio = narrow(length / f64::from(e.global::<f32>(values + 0xe0))); // own: the first active (or camera) light, read in place: an open packet route (the drawing agent's next PR)
                let raised = e.call(FLOAT_HELPER, &args![0.0f64, ratio.to_bits()]).f64();
                let limited = e.call(DOUBLE_HELPER, &args![1.0f64, raised]).f64();
                let falloff = 1.0 - narrow(limited);
                let brightness = e.call(COLOUR_LUMINANCE, &args![scaled]).f64();
                light_part = mul_d(falloff, brightness);
            }
            let chosen = if luminance < light_part {
                light_part
            } else {
                luminance
            };
            let fourth = capture::own_at::<f32>(e, ROWS + 0xc);
            capture::set_own_at(e, ROWS + 0xc, chosen * fourth);
        });
    }
    let mode = capture::own_at::<u32>(e, SHADER_MODE);
    if mode == 8 {
        let one = 1.0f32.to_bits();
        let alpha = if packet::property_word(e, property, 0x24) & 0x20000 != 0 {
            0
        } else {
            one
        };
        let flags_property = e.global::<u32>(block.addr() + 0x9c); // own: main's object read in place (value packet B, piece 4)
        let flags = e.global::<u16>(flags_property + 0x18); // own: main's object read in place (value packet B, piece 4)
        let blue = if flags & 1 != 0 || (flags >> 9) & 1 != 0 {
            0
        } else {
            one
        };
        capture::set_own_at(e, ROWS, 0u32);
        capture::set_own_at(e, ROWS + 4, 0u32);
        capture::set_own_at(e, ROWS + 8, blue);
        capture::set_own_at(e, ROWS + 0xc, alpha);
    }
    let pass_type = capture::own_at::<u32>(e, RENDERING_PASS_TYPE);
    if pass_type.wrapping_sub(0x244) > 7 || mode == 10 {
        return;
    }
    let factor = e.vcall(property, 0x110, &args![]).f64();
    let fourth = capture::own_at::<f32>(e, ROWS + 0xc);
    capture::set_own_at(e, ROWS + 0xc, mul_d(fourth, factor));
}

/// The two pass set-ups `SetupGeometryConstants` runs for types 4 and 5 (`00b7ad00`) and 0x250 and
/// 0x251 (`00b7ce90`) (`thiscall`: geometry, property, rendering object). The model-to-world helper
/// `00c4c2d0` (`cdecl`: transform, skin, matrix out) is the dispatch family's `inverse_4x4`.
const PASS_SETUP_SHADOW: u32 = 0x00b7_ad00;
const PASS_SETUP_SPECIAL: u32 = 0x00b7_ce90;

/// A constant map's `SetShaderConstants` (its vtable +0x78, `00e826d0`).
const SLOT_SET_SHADER_CONSTANTS: u32 = 0x78;
const SET_SHADER_CONSTANTS: u32 = 0x00e8_26d0;

// Typed from 00b7dab0
/// `ShadowLightShader::SetupGeometryConstants` (engine map), `thiscall` with one stack word, the
/// block (its first word a property, +8 a word, +0xc the shader property, +0x10 another
/// property). With the rendering object (`011f91e0`), its geometry (first word; transform at
/// +0x68, position at +0x20 or `011f4288`, +0xb8 and skin +0xbc) and the pass type (`011f91e4`):
/// pass types 6 and 8 with bit 0x40000 of the shader property and bit 0 of the property set
/// `SetZWriteEnable(0, 1)` and `00b98380(0, 1)`; the model matrix is made (`00c4c2d0`) and
/// without a skin uploaded (`00b7c750`). Types 0x250 and 0x251 run `00b7ce90`, types 4 and 5
/// `00b7ad00`; every other type the constant set-ups in the exe's order (lights, ambient,
/// object-to-cube, shadow projection, eye, `00b79050`, `00b790d0`, `00b79180`, `00b79100`,
/// `00b791f0`, `00b79410`, toggles, `00b796f0`, `00b79730`, clip planes, `00b798c0`). The pixel
/// (+0x30) and vertex (+0x34) constant maps then set their constants (`SetShaderConstants`,
/// vtable +0x78) for the pass's pixel (+0x44) and vertex (+0x5c) program; with the block's +0x10
/// property's bit 0, the shader's +0x1c gets it through its vtable +0x24.
pub fn setup_geometry_constants(e: &mut Engine, this: Ptr, block: Ptr) {
    let base = this.addr();
    let block = block.addr();
    let rendering = capture::own_at::<u32>(e, RENDERING_PASS);
    let geometry = capture::own_at::<u32>(e, rendering);
    let pass_type = capture::own_at::<u32>(e, RENDERING_PASS_TYPE);
    let mut position = capture::own_at::<u32>(e, geometry + 0x20);
    if position == 0 {
        position = DEFAULT_POSITION;
    }
    let data = capture::own_at::<u32>(e, geometry + 0xb8);
    let geometry_data = capture::own_at::<u32>(e, data + 0x34);
    let fog_word = e.global::<u32>(block + 8); // own: main's object read in place (value packet B, piece 4)
    let skin = capture::own_at::<u32>(e, geometry + 0xbc);
    let property = e.global::<u32>(block + 0xc); // own: main's object read in place (value packet B, piece 4)
    let first_property = e.global::<u32>(block); // own: main's object read in place (value packet B, piece 4)
    if (pass_type == 6 || pass_type == 8)
        && e.global::<u32>(property + 0x20) & 0x40000 != 0 // own: main's object read in place (value packet B, piece 4)
        && e.global::<u8>(first_property + 0x18) & 1 != 0
    // own: main's object read in place (value packet B, piece 4)
    {
        typed_call!(FAMILY, 0x00b9_7e30, state::set_z_write_enable(e, 0u8, 1u32));
        typed_call!(FAMILY, 0x00b9_8380, state::set_color_write(e, 0u32, 1u32));
    }
    let transform = geometry + 0x68;
    e.with_stack(0x40, |e, matrix| {
        let matrix_addr = matrix.addr();
        typed_call!(
            crate::dispatch::FAMILY,
            0x00c4_c2d0,
            crate::dispatch::math::inverse_4x4(e, Ptr::new(transform), skin, Ptr::new(matrix_addr))
        );
        if skin == 0 {
            typed_call!(
                FAMILY,
                0x00b7_c750,
                accumulator::set_world_view_constants(e, base, geometry, transform)
            );
        }
        let mut words = [0u32; 16];
        for (index, word) in words.iter_mut().enumerate() {
            *word = capture::own_at::<u32>(e, matrix_addr + 4 * idx(index));
        }
        if pass_type == 0x250 || pass_type == 0x251 {
            e.call(
                PASS_SETUP_SPECIAL,
                &args![base, geometry, property, rendering],
            );
        } else if pass_type == 4 || pass_type == 5 {
            e.call(
                PASS_SETUP_SHADOW,
                &args![base, geometry, property, rendering],
            );
        } else {
            let p = Ptr::new(property);
            let r = Ptr::new(rendering);
            lights(e, p, r, words, Ptr::new(transform), pass_type, skin);
            ambient_emittance(e, this, p, r, Ptr::new(geometry), fog_word);
            obj_to_cube(e, this, r, pass_type, skin, transform);
            shadow_proj(e, this, Ptr::new(block), skin, Ptr::new(transform));
            eye_pos(e, pass_type, Ptr::new(transform), words);
            vertex_entry_12(e, this, skin);
            vertex_entry_15(e, this, p);
            vertex_entry_17(e, this, Ptr::new(geometry));
            vertex_entry_16(e, this, geometry);
            vertex_entry_21(e, this, pass_type, p, Ptr::new(geometry));
            pixel_entry_2(e, this, pass_type, r);
            toggles(
                e,
                this,
                pass_type,
                geometry,
                p,
                fog_word,
                rendering,
                Ptr::new(first_property),
            );
            pixel_entry_16(e, this, p);
            pixel_entry_19(e);
            simple_shadow_clip_planes(e, this, pass_type);
            pixel_entry_20(e, this, p);
        }
    });
    for (map_offset, program_offset) in [(0x30u32, 0x44u32), (0x34, 0x5c)] {
        let map = e.global::<u32>(base + map_offset); // own: main's object read in place (value packet B, piece 4)
        if map == 0 {
            continue;
        }
        let pass = capture::own_at::<u32>(e, CURRENT_PASS);
        let program = capture::own_at::<u32>(e, pass + program_offset);
        if program == 0 {
            continue;
        }
        if crate::slot_target(e, map, SLOT_SET_SHADER_CONSTANTS) == SET_SHADER_CONSTANTS {
            let _: bool = typed_call!(
                FAMILY,
                0x00e8_26d0,
                constants::set_shader_constants(
                    e,
                    Ptr::new(map),
                    program,
                    geometry,
                    skin,
                    0,
                    geometry_data,
                    block,
                    0,
                    transform,
                    position,
                    0,
                    1
                )
            );
        } else {
            let words = args![
                program,
                geometry,
                skin,
                0u32,
                geometry_data,
                block,
                0u32,
                transform,
                position,
                0u32,
                1u32
            ];
            e.vcall(map, SLOT_SET_SHADER_CONSTANTS, &words);
        }
    }
    let other = e.global::<u32>(block + 0x10); // own: main's object read in place (value packet B, piece 4)
    if other != 0 && e.global::<u8>(other + 0x18) & 1 != 0 {
        // own: main's object read in place (value packet B, piece 4)
        let object = e.global::<u32>(base + 0x1c); // own: main's object read in place (value packet B, piece 4)
        e.vcall(object, 0x24, &args![other]);
    }
}
