//! The shadow light shader's per-geometry textures (`00b7d200`, its vtable +0x78, the slot the Xbox PDB
//! calls `SetupGeometryTextures`): what the batch renderer runs just before the per-geometry constants
//! ([`super::geometry`]) for nearly every lit geometry of the crowded street (about 600 times a frame).
//! By the rendering object's type it puts the shader property's textures on the current pass's stages
//! and then binds the pass's stages (the shader's vtable +0x108, the render family's
//! `bind_pass_stages` when the slot holds it).
//!
//! The set-up and each of the stage set-ups it calls are registered at their own exe addresses (the
//! family's list in `shader/mod.rs`); inside the set-up they are plain Rust calls, as in the
//! translation. The exe picks a body by a two-level jump table on the type; the match below has one arm
//! per body, in the table's order of first use, and was checked against the exe's table type by type.

use super::{accumulator, state, FAMILY};
use crate::render;
use engine_core::prelude::*;

/// The current shader pass (`0126f74c`; its stages at +0x24) and the rendering object (`011f91e0`;
/// its type at +4, a selector byte at +0xb, its lights at +0xc).
const CURRENT_PASS: u32 = 0x0126_f74c;
const RENDERING_PASS: u32 = 0x011f_91e0;

/// The current pass's stage `index` (the pass and its stages read afresh each time, as the exe does).
#[inline(always)]
fn pass_stage(e: &Engine, index: u32) -> u32 {
    let pass = e.global::<u32>(CURRENT_PASS);
    let stages = e.mem.u32(pass.wrapping_add(0x24));
    e.mem.u32(stages.wrapping_add(index.wrapping_mul(4)))
}

/// A stage's texture (+8).
#[inline(always)]
fn set_stage_texture(e: &mut Engine, stage: u32, texture: u32) {
    e.mem.set_u32(stage.wrapping_add(8), texture);
}

/// The stage takes the address modes of the property's clamp mode (its vtable +0xe0, then
/// `00e7de90`).
#[inline(always)]
fn clamp_stage(e: &mut Engine, stage: u32, property: u32) {
    let mode = render::pass::property_clamp_mode(e, property);
    typed_call!(
        FAMILY,
        0x00e7_de90,
        state::set_address_modes(e, Ptr::new(stage), mode)
    );
}

// Typed from 00b7c0a0
/// Unnamed in the engine map, `stdcall` with three words: the shader property (textures at +0xac, its
/// vtable +0xe0 answers a clamp mode), a stage index and a texture index. The current pass's stage
/// gets that texture and the clamp mode's address modes (`00e7de90`).
pub fn stage_texture(e: &mut Engine, property: Ptr, stage: u32, index: u32) {
    let stage = pass_stage(e, stage);
    let property = property.addr();
    let textures = e.mem.u32(property.wrapping_add(0xac));
    let texture = e.mem.u32(textures.wrapping_add(index.wrapping_mul(4)));
    set_stage_texture(e, stage, texture);
    clamp_stage(e, stage, property);
}

// Typed from 00b7c0e0
/// Unnamed in the engine map, `stdcall` with three words: the shader property, a stage index and a
/// texture index. The stage gets the property's texture of its second array or a default one
/// (`00c03230`) and, with a property, its clamp mode's address modes.
pub fn stage_second_texture(e: &mut Engine, property: Ptr, stage: u32, index: u32) {
    let stage = pass_stage(e, stage);
    let property = property.addr();
    let texture: u32 = typed_call!(
        FAMILY,
        0x00c0_3230,
        accumulator::texture_by_index(e, Ptr::new(property), index)
    );
    set_stage_texture(e, stage, texture);
    if property != 0 {
        clamp_stage(e, stage, property);
    }
}

/// `BSRenderedTexture::GetTexture` (`004bc320`, Xbox PDB; `thiscall`, one word).
const RENDERED_TEXTURE_GET_TEXTURE: u32 = 0x004b_c320;

// Typed from 00b7c120
/// Unnamed in the engine map, `stdcall` with one word, a stage index. The stage gets `GetTexture(0)`
/// of the rendered texture at +0x10c of the rendering object's first light (the first word of its
/// +0xc).
pub fn stage_light_texture(e: &mut Engine, stage: u32) {
    let stage = pass_stage(e, stage);
    let rendering = e.global::<u32>(RENDERING_PASS);
    let lights = e.mem.u32(rendering.wrapping_add(0xc));
    let light = e.mem.u32(lights);
    let rendered = e.mem.u32(light.wrapping_add(0x10c));
    let texture = e
        .call(RENDERED_TEXTURE_GET_TEXTURE, &args![rendered, 0u32])
        .u32();
    set_stage_texture(e, stage, texture);
}

/// `00b7c190`'s choice byte (`011ad80c`), its two textures (`011f953c`, `011f9540`), its filter byte
/// (`011f9425`), the renderer (`011f4748`, its device at +0x288) and the sampler value (`011f9428`).
const CHOICE: u32 = 0x011a_d80c;
const TEXTURE_CHOSEN: u32 = 0x011f_953c;
const TEXTURE_OTHER: u32 = 0x011f_9540;
const FILTER: u32 = 0x011f_9425;
const RENDERER: u32 = 0x011f_4748;
const SAMPLER_VALUE: u32 = 0x011f_9428;

// Typed from 00b7c190
/// Unnamed in the engine map, `stdcall` with one word, a stage index. The stage gets the texture
/// `011f953c` (with the byte `011ad80c` set) or `011f9540`, the filter mode 7 when `011f9425` is set
/// (`NiD3DTextureStage::SetFilterMode`), and the device (the renderer's +0x288) its vtable +0x114 with
/// the index, 8 and `011f9428`.
pub fn stage_shared_texture(e: &mut Engine, index: u32) {
    let stage = pass_stage(e, index);
    let source = if e.global::<u8>(CHOICE) != 0 {
        TEXTURE_CHOSEN
    } else {
        TEXTURE_OTHER
    };
    let texture = e.global::<u32>(source);
    set_stage_texture(e, stage, texture);
    if e.global::<u8>(FILTER) != 0 {
        typed_call!(
            FAMILY,
            0x00e7_def0,
            state::set_filter_mode(e, Ptr::new(stage), 7u32)
        );
    }
    let value = e.global::<u32>(SAMPLER_VALUE);
    let renderer = e.global::<u32>(RENDERER);
    let device = e.mem.u32(renderer.wrapping_add(0x288));
    e.vcall(device, 0x114, &args![index, 8u32, value]);
}

/// The texture getter `00c02f30` (unnamed, `stdcall`, two words: the property and an index).
const TEXTURE_GETTER: u32 = 0x00c0_2f30;

// Typed from 00b7c290
/// Unnamed in the engine map, `stdcall` with three words: the shader property, a stage index and a
/// word. The stage gets `00c02f30(property, word)` and the address modes of clamp mode 0.
pub fn stage_fourth_texture(e: &mut Engine, property: u32, stage: u32, word: u32) {
    let stage = pass_stage(e, stage);
    let texture = e.call(TEXTURE_GETTER, &args![property, word]).u32();
    set_stage_texture(e, stage, texture);
    typed_call!(
        FAMILY,
        0x00e7_de90,
        state::set_address_modes(e, Ptr::new(stage), 0u32)
    );
}

/// The texture `00b7c2c0` falls back to (`011f9514`).
const FALLBACK_FIFTH: u32 = 0x011f_9514;

// Typed from 00b7c2c0
/// Unnamed in the engine map, `stdcall` with three words: the shader property, a stage index and a
/// word. The stage gets what the property's vtable +0xf4 answers for (5, word), asked a second time
/// when it is not null, else `011f9514`.
pub fn stage_fifth_texture(e: &mut Engine, property: Ptr, stage: u32, word: u32) {
    let stage = pass_stage(e, stage);
    let property = property.addr();
    if e.vcall(property, 0xf4, &args![5u32, word]).u32() != 0 {
        let texture = e.vcall(property, 0xf4, &args![5u32, word]).u32();
        set_stage_texture(e, stage, texture);
        return;
    }
    let fallback = e.global::<u32>(FALLBACK_FIFTH);
    set_stage_texture(e, stage, fallback);
}

// Typed from 00b71b10
/// Unnamed in the engine map, `thiscall` with two words: a stage and the shader property. With both,
/// the stage takes the property's clamp mode's address modes.
/// `this` (`ECX`) is not read, but `00b7d200` calls it with `ECX` set.
pub fn stage_clamp(e: &mut Engine, _this: Ptr, stage: u32, property: u32) {
    if stage == 0 || property == 0 {
        return;
    }
    clamp_stage(e, stage, property);
}

/// The texture a stage gets when the property's first texture is null (`011f951c`) and the one
/// types 4 and 5 use without a texture of their own (`011f952c`).
const FALLBACK_FIRST: u32 = 0x011f_951c;
const FALLBACK_SHADOW: u32 = 0x011f_952c;

/// The current pass's stage `index` (the pass read afresh) gets the property's first texture (the
/// first word of its +0xb4 array), or `011f951c` when that is null.
fn first_texture_or_fallback(e: &mut Engine, stages: u32, index: u32, property: u32) {
    let textures = e.mem.u32(property.wrapping_add(0xb4));
    let texture = e.mem.u32(textures);
    let stage = e.mem.u32(stages.wrapping_add(index.wrapping_mul(4)));
    let value = if texture != 0 {
        texture
    } else {
        e.global::<u32>(FALLBACK_FIRST)
    };
    set_stage_texture(e, stage, value);
}

/// The shader's vtable +0x108 (`thiscall`, no stack words): the render family's `bind_pass_stages`
/// (`00be2170`).
const SLOT_BIND_STAGES: u32 = 0x108;
const BIND_PASS_STAGES: u32 = 0x00be_2170;

// Typed from 00b7d200
/// Unnamed in the engine map: the shadow light shader's vtable +0x78 (the slot the Xbox PDB calls
/// `SetupGeometryTextures`), `thiscall` with one stack word, a block whose +0xc is the shader property.
/// By the rendering object's type (`011f91e0`, +4; the exe's two-level jump table on type - 2, one
/// match arm per body) it puts the property's textures on the current pass's stages through
/// `00b7c0a0`, `00b7c0e0`, `00b7c290`, `00b7c2c0`, `00b7c120` and `00b7c190`, stores first textures
/// directly, or copies the property's texture arrays (+0xac, and +0xb0 without bit 0x100000 of its
/// +0x24) onto stages 0 to 6 and 7 to 13; then, for every type in the table, the shader's vtable
/// +0x108 binds the pass's stages. Types outside the table do nothing.
pub fn setup_geometry_textures(e: &mut Engine, this: Ptr, block: Ptr) {
    let base = this.addr();
    let rendering = e.global::<u32>(RENDERING_PASS);
    let kind = u32::from(e.mem.u16(rendering.wrapping_add(4)));
    let property = e.mem.u32(block.addr().wrapping_add(0xc));
    let pass = e.global::<u32>(CURRENT_PASS);
    let second_set = e.mem.u32(property.wrapping_add(0x24)) & 0x100000 == 0;
    let p = Ptr::new(property);
    let body = match kind {
        0x2
        | 0x6..=0x8
        | 0x11
        | 0x13
        | 0x58
        | 0x5a
        | 0x5c
        | 0x5e
        | 0x62..=0x63
        | 0x14a
        | 0x14d..=0x14f
        | 0x253
        | 0x255
        | 0x2de..=0x2df
        | 0x2ea..=0x2eb => 0,
        0x4..=0x5 => 1,
        0x59 | 0x5b | 0x5d | 0x5f | 0x1f1 | 0x1f4 => 2,
        0x64..=0x65
        | 0x69..=0x6a
        | 0x6f..=0x70
        | 0x74..=0x75
        | 0x7f..=0x80
        | 0x83..=0x84
        | 0x89..=0x8a
        | 0x8d..=0x8e
        | 0xc3
        | 0xc6
        | 0xc9
        | 0xcd
        | 0xd7
        | 0xdc
        | 0xe0
        | 0xe5
        | 0xe9
        | 0xee
        | 0xf2
        | 0xf7
        | 0xfb
        | 0x105
        | 0x10a
        | 0x111
        | 0x116
        | 0x11c
        | 0x121
        | 0x127
        | 0x235 => 3,
        0x66..=0x67
        | 0x6b..=0x6c
        | 0x71..=0x72
        | 0x76..=0x77
        | 0x81..=0x82
        | 0x86..=0x87
        | 0x8b..=0x8c
        | 0x90..=0x91 => 4,
        0x68 | 0x6d | 0x73 | 0x78 | 0x85 | 0x8f => 5,
        0x7c..=0x7d
        | 0x12c..=0x12f
        | 0x132
        | 0x135
        | 0x13e
        | 0x141
        | 0x154
        | 0x156
        | 0x1e2..=0x1e4 => 6,
        0x93..=0x94 | 0x1ef..=0x1f0 | 0x1f2..=0x1f3 | 0x1f5..=0x1f6 => 7,
        0x95 | 0x9b | 0xa0 | 0xa6 | 0xab | 0xb1 | 0xb6 | 0xbc | 0xd0 | 0xd4 | 0x10b => 8,
        0x96 | 0x9c | 0xa1 | 0xa7 | 0xac | 0xb2 | 0xb7 | 0xbd | 0xd2 | 0xd5 => 9,
        0x97 | 0x9d | 0xa2 | 0xa8 | 0xad | 0xb3 | 0xb8 | 0xbe | 0x104 | 0x109 | 0x110 | 0x115
        | 0x11b | 0x120 | 0x126 | 0x12b => 10,
        0xc4 | 0xc7 | 0xcb | 0xce | 0xff | 0x106 | 0x10c | 0x112 | 0x117 | 0x11d | 0x122
        | 0x128 => 11,
        0xd8 | 0xdd | 0xe1 | 0xe6 | 0xea | 0xef | 0xf3 | 0xf8 => 12,
        0xfe => 13,
        0x14b | 0x150 => 14,
        0x14c | 0x152 => 15,
        0x155
        | 0x157
        | 0x15a..=0x15b
        | 0x15d..=0x15e
        | 0x160..=0x161
        | 0x163..=0x164
        | 0x166..=0x167
        | 0x169..=0x16a
        | 0x16c..=0x16d
        | 0x16f..=0x170
        | 0x172..=0x173
        | 0x175..=0x176
        | 0x24b => 16,
        0x1f7..=0x1fe => 17,
        0x1ff..=0x206 => 18,
        0x207..=0x20e => 19,
        0x20f..=0x216 => 20,
        0x217..=0x21e => 21,
        0x21f..=0x226 => 22,
        0x227..=0x22e => 23,
        0x230 => 24,
        0x231..=0x233 => 25,
        0x244..=0x247 => 26,
        0x248..=0x24a => 27,
        0x2d6..=0x2d7 => 28,
        _ => return,
    };
    let stages = e.mem.u32(pass.wrapping_add(0x24));
    match body {
        0 => stage_texture(e, p, 0, 0),
        1 => {
            let stage = e.mem.u32(stages);
            let textures = e.mem.u32(property.wrapping_add(0xdc));
            let mut texture = 0;
            if textures != 0 {
                texture = e.mem.u32(textures.wrapping_add(0x6c));
            }
            if texture == 0 {
                texture = e.global::<u32>(FALLBACK_SHADOW);
            }
            set_stage_texture(e, stage, texture);
            // By address in the translation too: `00b71b10` is another unit's.
            typed_call!(
                FAMILY,
                0x00b7_1b10,
                stage_clamp(e, this, stage, property)
            );
        }
        2 => {
            stage_texture(e, p, 0, 0);
            let pass = e.global::<u32>(CURRENT_PASS);
            let stages = e.mem.u32(pass.wrapping_add(0x24));
            first_texture_or_fallback(e, stages, 1, property);
        }
        3 => {
            stage_second_texture(e, p, 1, 0);
            stage_texture(e, p, 0, 0);
        }
        4 => {
            first_texture_or_fallback(e, stages, 2, property);
            stage_second_texture(e, p, 1, 0);
            stage_texture(e, p, 0, 0);
        }
        5 => {
            stage_texture(e, p, 0, 0);
            stage_second_texture(e, p, 1, 0);
            stage_texture(e, p, 2, 1);
            stage_second_texture(e, p, 3, 1);
        }
        6 | 16 => stage_second_texture(e, p, 0, 0),
        7 | 24 => {
            let index = u32::from(e.mem.u8(rendering.wrapping_add(0xb)));
            stage_texture(e, p, 0, index);
            stage_second_texture(e, p, 1, index);
            if body == 24 {
                stage_shared_texture(e, 2);
            }
        }
        8 => {
            stage_texture(e, p, 0, 0);
            stage_second_texture(e, p, 1, 0);
        }
        9 => {
            first_texture_or_fallback(e, stages, 4, property);
            stage_texture(e, p, 0, 0);
            stage_second_texture(e, p, 1, 0);
        }
        10 => {
            stage_texture(e, p, 0, 0);
            stage_second_texture(e, p, 1, 0);
            stage_texture(e, p, 4, 1);
        }
        11 | 12 => {
            stage_texture(e, p, 0, 0);
            stage_second_texture(e, p, 1, 0);
            let pass = e.global::<u32>(CURRENT_PASS);
            let stages = e.mem.u32(pass.wrapping_add(0x24));
            let index = if body == 11 { 4 } else { 3 };
            first_texture_or_fallback(e, stages, index, property);
        }
        13 => {
            stage_texture(e, p, 0, 0);
            stage_second_texture(e, p, 1, 0);
            stage_texture(e, p, 4, 1);
            stage_second_texture(e, p, 6, 1);
            stage_shared_texture(e, 7);
        }
        14 => {
            stage_texture(e, p, 0, 0);
            stage_texture(e, p, 1, 1);
            stage_second_texture(e, p, 2, 1);
        }
        15 => {
            stage_texture(e, p, 0, 0);
            stage_texture(e, p, 1, 1);
        }
        17..=23 => {
            for index in (0..=(body - 17)).rev() {
                let first = e.mem.u32(property.wrapping_add(0xac));
                let stages = e.mem.u32(pass.wrapping_add(0x24));
                let stage = e.mem.u32(stages.wrapping_add(4 * index));
                let texture = e.mem.u32(first.wrapping_add(4 * index));
                set_stage_texture(e, stage, texture);
                if second_set {
                    let second = e.mem.u32(property.wrapping_add(0xb0));
                    let stages = e.mem.u32(pass.wrapping_add(0x24));
                    let stage = e.mem.u32(stages.wrapping_add(4 * (index + 7)));
                    let texture = e.mem.u32(second.wrapping_add(4 * index));
                    set_stage_texture(e, stage, texture);
                }
            }
        }
        25 => {
            stage_texture(e, p, 0, 0);
            stage_light_texture(e, 6);
        }
        26 | 27 => {
            let (first, second) = if body == 26 { (2, 3) } else { (3, 1) };
            stage_fifth_texture(e, p, first, 0);
            stage_fourth_texture(e, property, second, 0);
            stage_second_texture(e, p, 0, 0);
        }
        // Body 28.
        _ => first_texture_or_fallback(e, stages, 0, property),
    }
    if crate::slot_target(e, base, SLOT_BIND_STAGES) == BIND_PASS_STAGES {
        typed_call!(
            render::FAMILY,
            0x00be_2170,
            render::pass::bind_pass_stages(e); words base
        );
    } else {
        e.vcall(base, SLOT_BIND_STAGES, &[]);
    }
}
