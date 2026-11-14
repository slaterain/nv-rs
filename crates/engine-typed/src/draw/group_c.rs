//! Group C of the drawing thread's plan (`measure/typed-draw/GROUP-C.md`): what the world draw still
//! reached as translations, typed so a drawing thread can run it. First the render states' lock
//! counters the passes' state restore calls by address (`00714a40`, `00714a60`, `00714aa0`, `00714ae0`,
//! `004eced0`): each gives back `count` locks on its state
//! (through [`super::own`], the draw's own writes) and sets the state (the shader family's setters).

use super::capture;
use super::{own, packet};
use crate::shader::{self, lighting, state};
use engine_core::ptr::Ptr;
use engine_core::{args, typed_call, Engine};

// Typed from 00714a40
/// `count` z-test locks given back (`011ff9d8`), then the z test on, unlocked (`00b97de0(1, 0)`).
pub fn release_z_enable(e: &mut Engine, count: u32) {
    own::release_state_locks(e, own::Z_ENABLE_LOCKS, count);
    typed_call!(
        shader::FAMILY,
        0x00b9_7de0,
        state::set_z_enable(e, 1u8, 0u32)
    );
}

// Typed from 00714a60
/// `count` z-write locks given back (`011ff9dc`), then z writes on, unlocked
/// (`BSRenderState::SetZWriteEnable`, `00b97e30(1, 0)`).
pub fn release_z_write(e: &mut Engine, count: u32) {
    own::release_state_locks(e, own::Z_WRITE_LOCKS, count);
    typed_call!(
        shader::FAMILY,
        0x00b9_7e30,
        state::set_z_write_enable(e, 1u8, 0u32)
    );
}

// Typed from 00714aa0
/// `count` alpha-test locks given back (`011ff9e8`), then the alpha test off, unlocked
/// (`00b97ed0(0, 0)`).
pub fn release_alpha_test(e: &mut Engine, count: u32) {
    own::release_state_locks(e, own::ALPHA_TEST_LOCKS, count);
    typed_call!(
        shader::FAMILY,
        0x00b9_7ed0,
        state::set_alpha_test_enable(e, 0u8, 0u32)
    );
}

// Typed from 00714ae0
/// `count` alpha-blend locks given back (`011ff9f0`), then alpha blending off, unlocked
/// (`00b97fa0(0, 0)`).
pub fn release_alpha_blend(e: &mut Engine, count: u32) {
    own::release_state_locks(e, own::ALPHA_BLEND_LOCKS, count);
    typed_call!(
        shader::FAMILY,
        0x00b9_7fa0,
        state::set_alpha_blend_enable(e, 0u8, 0u32)
    );
}

// Typed from 004eced0
/// `count` colour-write locks given back (`011ffa18`), then every colour channel written, unlocked
/// (`00b98380(7, 0)`).
pub fn release_color_write(e: &mut Engine, count: u32) {
    own::release_state_locks(e, own::COLOR_WRITE_LOCKS, count);
    typed_call!(
        shader::FAMILY,
        0x00b9_8380,
        state::set_color_write(e, 7u32, 0u32)
    );
}

// ---- What the set-ups share. ----

/// Bits of 1.0.
const ONE: u32 = 0x3f80_0000;

/// The current pass's stage `index` (the pass's stages, +0x24, are its contents: filled at load and
/// read in place).
pub(super) fn pass_stage(e: &Engine, index: u32) -> u32 {
    let pass = packet::current_pass(e);
    let stages = e.global::<u32>(pass.wrapping_add(0x24));
    e.global::<u32>(stages.wrapping_add(index.wrapping_mul(4)))
}

/// A stage given a texture, as the set-ups leave it: the texture in (+8), the stage's address modes
/// from the property's clamp mode (its slot 0xcc; `00e7de90`, the shader family's) and the stage bound
/// (`00e7eb00(0)`, the render family's).
pub(super) fn set_stage(e: &mut Engine, stage: u32, texture: u32, property: u32) {
    own::set_stage_texture(e, stage, texture);
    let mode = e.vcall(property, 0xcc, &[]).u32();
    typed_call!(
        shader::FAMILY,
        0x00e7_de90,
        state::set_address_modes(e, Ptr::new(stage), mode)
    );
    let _: bool = typed_call!(
        crate::render::FAMILY,
        0x00e7_eb00,
        crate::render::stage::bind(e, Ptr::new(stage), 0u8)
    );
}

/// A constant map's `SetShaderConstants` (its slot 0x78; `NiD3DShaderConstantMap`'s, `00e826d0`, is the
/// shader family's).
const SLOT_SET_SHADER_CONSTANTS: u32 = 0x78;
const SET_SHADER_CONSTANTS: u32 = 0x00e8_26d0;

/// The shader's pixel (+0x30) and vertex (+0x34) constant maps set their constants for the current
/// pass's pixel (+0x44) and vertex (+0x5c) programs, in that order ([`map_constants`]).
pub(super) fn set_map_constants(e: &mut Engine, shader: u32, words: [u32; 10]) {
    map_constants(e, shader, 0x30, 0x44, words);
    map_constants(e, shader, 0x34, 0x5c, words);
}

/// The shader's constant map at `map_offset` sets its constants for the current pass's program at
/// `program_offset` (`SetShaderConstants`, with the ten words a set-up hands on), skipped without the
/// map or the program. The maps are the shader's (made at load); the programs the pass's contents.
pub(super) fn map_constants(
    e: &mut Engine,
    shader: u32,
    map_offset: u32,
    program_offset: u32,
    words: [u32; 10],
) {
    let map = own::constant_map(e, shader, map_offset);
    if map == 0 {
        return;
    }
    let program = e.global::<u32>(packet::current_pass(e).wrapping_add(program_offset));
    if program == 0 {
        return;
    }
    set_constants_of(e, map, program, words);
}

/// The constant map `map` sets its constants for `program` (`SetShaderConstants`, slot 0x78, with the
/// ten words a set-up hands on; direct when the slot holds the shader family's typed one).
pub(super) fn set_constants_of(e: &mut Engine, map: u32, program: u32, words: [u32; 10]) {
    let [w2, w3, w4, w5, w6, w7, w8, w9, w10, w11] = words;
    if crate::slot_target(e, map, SLOT_SET_SHADER_CONSTANTS) == SET_SHADER_CONSTANTS {
        let _: bool = typed_call!(
            shader::FAMILY,
            0x00e8_26d0,
            shader::constants::set_shader_constants(
                e,
                Ptr::new(map),
                program,
                w2,
                w3,
                w4,
                w5,
                w6,
                w7,
                w8,
                w9,
                w10,
                w11
            )
        );
    } else {
        e.vcall(
            map,
            SLOT_SET_SHADER_CONSTANTS,
            &args![program, w2, w3, w4, w5, w6, w7, w8, w9, w10, w11],
        );
    }
}

/// A float truncated to a 64-bit integer as `FISTP qword` does under a chopping control word
/// (`x87::fistp_chop64`), its low word: a NaN or a magnitude from 2^63 up gives the integer indefinite,
/// whose low word is 0.
pub(super) fn chop_low_word(x: f32) -> u32 {
    let [b0, b1, b2, b3, ..] = engine_core::x87::fistp_chop64(f64::from(x)).to_le_bytes();
    u32::from_le_bytes([b0, b1, b2, b3])
}

/// The pass types' flag tables `00b815e0` reads (a word each, filled at load: `011ff248`, `011ff120`),
/// the flag bytes it writes (`011ff118` from the first, `011ff110` from the second) and its entries (the
/// pointers at `011ff4a0`, nine, made at load).
const PASS_FLAGS_FIRST: u32 = 0x011f_f248;
const PASS_FLAGS_SECOND: u32 = 0x011f_f120;
const FIRST_FLAG_BYTES: u32 = 0x011f_f118;
const SECOND_FLAG_BYTES: u32 = 0x011f_f110;
const FLAG_ENTRIES: u32 = 0x011f_f4a0;

/// Bit `bit` of pass type `pass_type`'s word in `table`.
fn table_bit(e: &Engine, table: u32, pass_type: u32, bit: u32) -> u8 {
    let word = capture::own_at::<u32>(e, table.wrapping_add(pass_type.wrapping_mul(4)));
    u8::from((word >> bit) & 1 != 0)
}

// Typed from 00b815e0
/// A pass type's flag bits into the flag bytes and entries, `cdecl(pass type)`: the first three entries
/// switched on (each when there is one, before its byte), the bytes `011ff118` to `011ff11a` bits 1 to 3
/// of the first table; the fourth entry bit 1 of the second table; the bytes `011ff110` to `011ff115` bits
/// 1 to 6 of the second, the entries five to nine (when there) bits 2 to 6. Every bit read from its table
/// again, as the exe does.
pub fn pass_flags(e: &mut Engine, pass_type: u32) {
    for (index, bit) in [(0u32, 1u32), (1, 2), (2, 3)] {
        let entry = capture::own_at::<u32>(e, FLAG_ENTRIES + 4 * index);
        if entry != 0 {
            own::set_entry_enabled(e, entry, true);
        }
        let value = table_bit(e, PASS_FLAGS_FIRST, pass_type, bit);
        own::set_constant_byte(e, FIRST_FLAG_BYTES + index, value);
    }
    let entry = capture::own_at::<u32>(e, FLAG_ENTRIES + 4 * 3);
    if entry != 0 {
        let on = table_bit(e, PASS_FLAGS_SECOND, pass_type, 1) != 0;
        own::set_entry_enabled(e, entry, on);
    }
    for index in 0..6u32 {
        let value = table_bit(e, PASS_FLAGS_SECOND, pass_type, index + 1);
        own::set_constant_byte(e, SECOND_FLAG_BYTES + index, value);
        if index < 5 {
            let entry = capture::own_at::<u32>(e, FLAG_ENTRIES + 4 * (4 + index));
            if entry != 0 {
                let on = table_bit(e, PASS_FLAGS_SECOND, pass_type, index + 2) != 0;
                own::set_entry_enabled(e, entry, on);
            }
        }
    }
}

/// The colour constant a lighting set-up fills (`011fa270`, four floats).
const LIGHT_COLOUR: u32 = 0x011f_a270;

// Typed from 00c03180
/// A material's colour as a constant, `stdcall(material)`: its colour (its colour pointer +0x34, or its
/// own colour +0x28), with HDR on each component times the HDR scale (the image space's effect 1's
/// scale times the material's +0x40, a float), to `011fa270` with 1.0.
pub fn material_colour_constant(e: &mut Engine, material: u32) {
    let mut source = packet::material_colour(e, material);
    if source == 0 {
        source = material.wrapping_add(0x28);
    }
    let mut colour = [0u32, 1, 2].map(|k| packet::colour_word(e, source, k));
    if packet::hdr_on(e) != 0 {
        let scale = f32::from_bits(packet::hdr_effect_scale(e))
            * f32::from_bits(packet::material_scale(e, material));
        colour = colour.map(|c| (scale * f32::from_bits(c)).to_bits());
    }
    for (k, word) in (0u32..).zip(colour) {
        own::set_constant_word(e, LIGHT_COLOUR + 4 * k, word);
    }
    own::set_constant_word(e, LIGHT_COLOUR + 0xc, ONE);
}

/// The light table the lighting set-ups fill (`01201c80`, 0x20 bytes an entry).
const LIGHT_TABLE: u32 = 0x0120_1c80;

// Typed from 00bb95e0
/// A light table entry set, `cdecl(index, four words, four words)`: entry `index` of the table at
/// `01201c80` takes the first group in its second half (+0x10 to +0x1c) and the second group in its
/// first half (+0 to +0xc), in that order.
#[allow(clippy::too_many_arguments)]
pub fn set_light_entry(
    e: &mut Engine,
    index: u32,
    first_0: u32,
    first_1: u32,
    first_2: u32,
    first_3: u32,
    second_0: u32,
    second_1: u32,
    second_2: u32,
    second_3: u32,
) {
    let entry = LIGHT_TABLE.wrapping_add(index.wrapping_shl(5)); // own: an address, written through set_constant_word
    for (offset, word) in [
        (0x10u32, first_0),
        (0x14, first_1),
        (0x18, first_2),
        (0x1c, first_3),
        (0, second_0),
        (4, second_1),
        (8, second_2),
        (0xc, second_3),
    ] {
        own::set_constant_word(e, entry.wrapping_add(offset), word);
    }
}

// Typed from 00bb95b0
/// A light table entry's first half set, `cdecl(index, four words)`: entry `index` of the table at `01201c80` takes
/// the words at +0 to +0xc.
pub fn set_light_half(
    e: &mut Engine,
    index: u32,
    word_0: u32,
    word_1: u32,
    word_2: u32,
    word_3: u32,
) {
    let entry = LIGHT_TABLE.wrapping_add(index.wrapping_shl(5)); // own: an address, written through set_constant_word
    for (offset, word) in (0u32..).zip([word_0, word_1, word_2, word_3]) {
        own::set_constant_word(e, entry.wrapping_add(4 * offset), word);
    }
}

/// The lighting shaders' ambient constant (`01201b30`, four words), its entry (the pointer at `012010c4`, made at load)
/// and the byte that says the ambient is set (`01200bf4`).
const AMBIENT: u32 = 0x0120_1b30;
const AMBIENT_ENTRY: u32 = 0x0120_10c4;
const AMBIENT_SET: u32 = 0x0120_0bf4;

// Typed from 00bbb790
/// The ambient set, `cdecl(colour, byte)`: the four words at `colour` into the ambient constant; the ambient's entry
/// switched off when the byte and the set byte are both clear, else on; the set byte takes the byte.
pub fn set_ambient(e: &mut Engine, colour: Ptr, byte: u8) {
    for k in 0..4u32 {
        let word = e.global::<u32>(colour.addr().wrapping_add(4 * k)); // own: the caller's colour, a local
        own::set_constant_word(e, AMBIENT + 4 * k, word);
    }
    let entry = capture::own_at::<u32>(e, AMBIENT_ENTRY);
    let on = byte != 0 || own::constant_byte(e, AMBIENT_SET) != 0;
    own::set_entry_enabled(e, entry, on);
    own::set_constant_byte(e, AMBIENT_SET, byte);
}

/// The flag byte the pass end clears (`01200863`) and the entry it switches off (the pointer at
/// `012010bc`, made at load).
const PASS_END_FLAG: u32 = 0x0120_0863;
const PASS_END_ENTRY: u32 = 0x0120_10bc;

// Typed from 00bb96d0
/// After a pass, `thiscall(shader, unread word)` (neither read): the entry at `012010bc` switched off
/// and the flag byte `01200863` cleared.
pub fn pass_end_flags(e: &mut Engine, _this: Ptr, _unused: u32) {
    let entry = capture::own_at::<u32>(e, PASS_END_ENTRY);
    own::set_constant_byte(e, PASS_END_FLAG, 0);
    own::set_entry_enabled(e, entry, false);
}

/// The distance fades of a lighting property's slot 0x110 (`00b66c40`): the second fade's near (`011f945c`) and far
/// (`011f9460`) distances, the first's (`011f9464`, `011f9468`).
const SECOND_FADE_NEAR: u32 = 0x011f_945c;
const SECOND_FADE_FAR: u32 = 0x011f_9460;
const FIRST_FADE_NEAR: u32 = 0x011f_9464;
const FIRST_FADE_FAR: u32 = 0x011f_9468;
/// A lighting property's slot 0x110 and the exe's target there on every lighting property class.
pub(crate) const SLOT_DISTANCE_FADE: u32 = 0x110;
pub(crate) const DISTANCE_FADE: u32 = 0x00b6_6c40;

/// The fade at `distance` between `near` and `far` (main-thread `f32`): 1.0 when `near` is above the distance
/// (ordered), 0.0 when the distance is at least `far`, else `1 - min(ratio, 1)` with `ratio = (distance - near) /
/// (far - near)` stored as a float.
fn distance_fade_between(distance: f32, near: f32, far: f32) -> f32 {
    if near > distance {
        return 1.0;
    }
    if distance >= far {
        return 0.0;
    }
    let ratio = (distance - near) / (far - near);
    if 1.0 < ratio {
        return 0.0;
    }
    1.0 - ratio
}

// Typed from 00b66c40
/// A lighting property's distance fade (its vtable's slot 0x110, unnamed in the engine map), `thiscall(property)`,
/// a float in `ST0`. With bit 0x800000 of its second flags (+0x24) clear: when its flags (+0x20) have bit 0x20000
/// and the first far distance (`011f9468`) is above 0, the fade between `011f9464` and `011f9468` at its camera
/// distance (+0x34); otherwise when they have bit 0x80 and the second far distance (`011f9460`) is above 0, 1.0
/// when `011f945c` is above the distance, 0.0 at `011f9460` or past it, else 1.0 less the smaller of 1.0 and the
/// ratio (`0040ebd0`, the draw helpers'); otherwise 1.0. The second flags and the distance are the packet's (value
/// packet B: the cut copies +0x24..+0x37); main-thread `f32`.
pub fn distance_fade(e: &mut Engine, this: Ptr) -> f32 {
    let base = this.addr();
    let flags = packet::property_flags(e, base);
    let unlocked = |e: &Engine| packet::property_flags_24(e, base) & 0x0080_0000 == 0;
    let second_far = f32::from_bits(e.global::<u32>(SECOND_FADE_FAR));
    let second = flags & 0x80 != 0 && unlocked(e) && 0.0 < second_far;
    let first_far = f32::from_bits(e.global::<u32>(FIRST_FADE_FAR));
    let first = flags & 0x0002_0000 != 0 && unlocked(e) && first_far > 0.0;
    let distance = f32::from_bits(packet::property_distance(e, base));
    if first {
        let near = f32::from_bits(e.global::<u32>(FIRST_FADE_NEAR));
        return distance_fade_between(distance, near, first_far);
    }
    if !second {
        return 1.0;
    }
    let near = f32::from_bits(e.global::<u32>(SECOND_FADE_NEAR));
    if near > distance {
        return 1.0;
    }
    if distance >= second_far {
        return 0.0;
    }
    let ratio = (distance - near) / (second_far - near);
    let picked: f32 = typed_call!(
        crate::draw_f::FAMILY,
        0x0040_ebd0,
        crate::draw_f::helpers::float_min(e, 1.0, ratio)
    );
    1.0 - picked
}

/// A lighting property's slot 0x110 factor: [`distance_fade`] directly when the slot holds the exe's
/// (`00b66c40`, every lighting property class), else through the vtable.
pub(crate) fn property_distance_fade(e: &mut Engine, property: u32) -> f64 {
    if crate::slot_target(e, property, SLOT_DISTANCE_FADE) == DISTANCE_FADE {
        let fade: f32 = typed_call!(
            super::FAMILY,
            0x00b6_6c40,
            distance_fade(e, Ptr::new(property))
        );
        f64::from(fade)
    } else {
        e.vcall(property, SLOT_DISTANCE_FADE, &[]).f64()
    }
}
/// The zero colour's four words (a constant of the exe).
fn zero_colour(e: &Engine) -> [u32; 4] {
    std::array::from_fn(|i| e.global::<u32>(lighting::ZERO_COLOUR + 4 * lighting::idx(i)))
    // own: a constant of the exe
}

/// Three floats of a pass light's light from `offset` times `strength`, in the exe's operand order (the first two
/// loaded and multiplied, the third the memory operand).
fn scaled_colour(e: &Engine, data: u32, offset: u32, strength: f32) -> [f32; 3] {
    [
        lighting::light_value(e, data, offset) * strength,
        lighting::light_value(e, data, offset + 4) * strength,
        strength * lighting::light_value(e, data, offset + 8),
    ]
}

/// The light table's first half (`00bb95b0`, the draw family's) set to the zero colour for light `index`.
fn zero_light(e: &mut Engine, index: u32) {
    let zero = zero_colour(e);
    typed_call!(
        super::FAMILY,
        0x00bb_95b0,
        set_light_half(e, index, zero[0], zero[1], zero[2], zero[3])
    );
}

/// A light table entry (`00bb95e0`, the draw family's): the first four words in its second half, the second four
/// in its first.
fn light_entry(e: &mut Engine, index: u32, first: [u32; 4], second: [u32; 4]) {
    typed_call!(
        super::FAMILY,
        0x00bb_95e0,
        set_light_entry(
            e, index, first[0], first[1], first[2], first[3], second[0], second[1], second[2],
            second[3]
        )
    );
}

// Typed from 00b70de0
/// Sets the lighting shader's light `index` from a light with a direction (unnamed in the engine map; `cdecl (index,
/// light, scale)`): without a light, the light table's first half (`00bb95b0`) the zero colour. Otherwise its light's
/// direction (+0xf0, w the shader property's vtable +0x10c or 1.0) and colour go to the light table (`00bb95e0`): the
/// colour is the light's +0xd4 times its capped strength, times `scale`, times the pass light's +0xd0 (and times
/// `011f9190` with the HDR switch and not `011f9427`), w the pass light's +0xd4. On the way the light's +0xc8 colour
/// times the capped strength, brightened and times `scale` (w 1.0), goes to the ambient (`00bbb790`) with whether
/// `scale` is below 1.0. The pass light and its light are read through the packet. Main thread only: `f32` maths.
pub fn direction_light(e: &mut Engine, index: u32, light: Ptr, scale: f32) {
    let light = light.addr();
    if light == 0 {
        zero_light(e, index);
        return;
    }
    let object = lighting::shader_object(e);
    let data = packet::pass_light_word(e, light, 0xf8);
    let direction: [u32; 3] =
        std::array::from_fn(|i| packet::light_value_word(e, data, 0xf0 + 4 * lighting::idx(i)));
    let w = if object != 0 {
        crate::anim_keys::narrow(e.vcall(object, 0x10c, &args![]).f64())
    } else {
        1.0
    };
    let threshold = e.global::<f32>(lighting::THRESHOLD); // own: main's frame value, value packet B
    let strength = lighting::capped_strength(e, data);
    let mut colour = scaled_colour(e, data, 0xc8, strength);
    if let Some(bright) = lighting::brighten(e, colour, threshold) {
        colour = bright;
    }
    let scaled = colour.map(|component| component * scale);
    let below_full = u8::from(1.0 > scale);
    e.with_stack(0x10, |e, adjusted| {
        let adjusted = adjusted.addr();
        for (i, component) in scaled.iter().enumerate() {
            capture::set_own_at::<f32>(e, adjusted + 4 * lighting::idx(i), *component);
        }
        capture::set_own_at::<f32>(e, adjusted + 0xc, 1.0);
        typed_call!(
            super::FAMILY,
            0x00bb_b790,
            set_ambient(e, Ptr::new(adjusted), below_full)
        );
    });
    let mut strength = strength * scale * f32::from_bits(packet::pass_light_word(e, light, 0xd0));
    if e.global::<u8>(lighting::HDR) != 0 && e.global::<u8>(lighting::NO_EXTRA_SCALE) == 0 {
        // own: main's frame value, value packet B
        strength *= e.global::<f32>(lighting::EXTRA_SCALE); // own: main's frame value, value packet B
    }
    let colour = scaled_colour(e, data, 0xd4, strength);
    let alpha = packet::pass_light_word(e, light, 0xd4);
    light_entry(
        e,
        index,
        [direction[0], direction[1], direction[2], w.to_bits()],
        [
            colour[0].to_bits(),
            colour[1].to_bits(),
            colour[2].to_bits(),
            alpha,
        ],
    );
}

// Typed from 00b71130
/// Sets the lighting shader's light `index` (below 20) of its other set (unnamed in the engine map; `cdecl (index,
/// light, scale)`): without a light, the light table's first half (`00bb95b0`) the zero colour; otherwise the light
/// table (`00bb95e0`) gets its light's position (+0x8c, w its radius +0xe0) and, for `scale` below 1.0, the zero
/// colour, else the light's +0xd4 colour times its strength (held to 1.0 unless the HDR switch: `0040ebd0`), `scale`
/// and the pass light's +0xd0; w the pass light's +0xd4. The pass light and its light are read through the packet.
/// Main thread only: `f32` maths.
pub fn other_light(e: &mut Engine, index: u32, light: Ptr, scale: f32) {
    if index >= 0x14 {
        return;
    }
    let light = light.addr();
    if light == 0 {
        zero_light(e, index);
        return;
    }
    let data = packet::pass_light_word(e, light, 0xf8);
    let first: [u32; 4] =
        [0x8cu32, 0x90, 0x94, 0xe0].map(|at| packet::light_value_word(e, data, at));
    // The exe also reads +0xe4 and +0xe8 into locals it never uses.
    let _ = (
        packet::light_value_word(e, data, 0xe4),
        packet::light_value_word(e, data, 0xe8),
    );
    let colour = if 1.0 > scale {
        let zero = zero_colour(e);
        [zero[0], zero[1], zero[2]]
    } else {
        let mut strength = lighting::light_value(e, data, 0xc4);
        if e.global::<u8>(lighting::HDR) == 0 {
            // own: main's frame value, value packet B
            strength = typed_call!(
                crate::draw_f::FAMILY,
                0x0040_ebd0,
                crate::draw_f::helpers::float_min(e, 1.0, strength)
            );
        }
        let strength = strength * scale * f32::from_bits(packet::pass_light_word(e, light, 0xd0));
        scaled_colour(e, data, 0xd4, strength).map(f32::to_bits)
    };
    let alpha = packet::pass_light_word(e, light, 0xd4);
    light_entry(e, index, first, [colour[0], colour[1], colour[2], alpha]);
}
