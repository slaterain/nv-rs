//! Group C of the drawing thread's plan (`measure/typed-draw/GROUP-C.md`): what the world draw still
//! reached as translations, typed so a drawing thread can run it. First the render states' lock
//! counters the passes' state restore calls by address (`00714a40`, `00714a60`, `00714aa0`, `00714ae0`,
//! `004eced0`): each gives back `count` locks on its state
//! (through [`super::own`], the draw's own writes) and sets the state (the shader family's setters).

use super::{own, packet};
use crate::shader::{self, state};
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
    let word = e.global::<u32>(table.wrapping_add(pass_type.wrapping_mul(4)));
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
        let entry = e.global::<u32>(FLAG_ENTRIES + 4 * index);
        if entry != 0 {
            own::set_entry_enabled(e, entry, true);
        }
        let value = table_bit(e, PASS_FLAGS_FIRST, pass_type, bit);
        own::set_constant_byte(e, FIRST_FLAG_BYTES + index, value);
    }
    let entry = e.global::<u32>(FLAG_ENTRIES + 4 * 3);
    if entry != 0 {
        let on = table_bit(e, PASS_FLAGS_SECOND, pass_type, 1) != 0;
        own::set_entry_enabled(e, entry, on);
    }
    for index in 0..6u32 {
        let value = table_bit(e, PASS_FLAGS_SECOND, pass_type, index + 1);
        own::set_constant_byte(e, SECOND_FLAG_BYTES + index, value);
        if index < 5 {
            let entry = e.global::<u32>(FLAG_ENTRIES + 4 * (4 + index));
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
    let entry = LIGHT_TABLE.wrapping_add(index.wrapping_shl(5));
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

/// The flag byte the pass end clears (`01200863`) and the entry it switches off (the pointer at
/// `012010bc`, made at load).
const PASS_END_FLAG: u32 = 0x0120_0863;
const PASS_END_ENTRY: u32 = 0x0120_10bc;

// Typed from 00bb96d0
/// After a pass, `thiscall(shader, unread word)` (neither read): the entry at `012010bc` switched off
/// and the flag byte `01200863` cleared.
pub fn pass_end_flags(e: &mut Engine, _this: Ptr, _unused: u32) {
    let entry = e.global::<u32>(PASS_END_ENTRY);
    own::set_constant_byte(e, PASS_END_FLAG, 0);
    own::set_entry_enabled(e, entry, false);
}
