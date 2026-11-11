//! The shader pass's begin and end as the batch renderer's draws reach them through the shader's
//! vtable (group B, first part): the `BSShader` slots the shadow light shader keeps (the alpha function
//! and test of a geometry's state, the pass's render states set and put back, its constants handed to
//! the shader objects, the end and restore states) and the render state put back by state
//! ([`restore_state`]), with the masked stage binding the pass set-ups use.
//!
//! The render states go through the shader family's setters and caches (condition 2 of the pipeline
//! probe: the render-state cache the next draw on main reads).

use super::{own, packet, FAMILY};
use crate::shader::{self, state};
use engine_core::ptr::Ptr;
use engine_core::{args, typed_call, Engine};

/// The first node of the current pass's render state list (+0x30, its +0x10).
#[inline(always)]
fn pass_states(e: &Engine) -> u32 {
    let pass = packet::current_pass(e);
    let states = e.global::<u32>(pass.wrapping_add(0x30));
    e.global::<u32>(states.wrapping_add(0x10))
}

// Typed from 00be0eb0
/// The pass's render states set (a shader's slot 0x100), `thiscall(shader, unread word)`: for each node
/// of the current pass's state list, the state (+0) and its value (+4) through `00b97d90`, unlocked.
pub fn set_pass_states(e: &mut Engine, _this: u32, _unused: u32) {
    let mut node = pass_states(e);
    while node != 0 {
        let value = e.global::<u32>(node.wrapping_add(4));
        let state = e.global::<u32>(node);
        typed_call!(
            shader::FAMILY,
            0x00b9_7d90,
            state::set_state(e, state, value, 0u32)
        );
        node = e.global::<u32>(node.wrapping_add(8));
    }
}

// Typed from 00be0ee0
/// The pass's render states put back (a shader's slot 0x104), `thiscall(shader, unread word)`: for each
/// node of the current pass's state list, [`restore_state`] of its state.
pub fn restore_pass_states(e: &mut Engine, _this: u32, _unused: u32) {
    let mut node = pass_states(e);
    while node != 0 {
        let state = e.global::<u32>(node);
        typed_call!(FAMILY, 0x00b9_8610, restore_state(e, state, 0u32));
        node = e.global::<u32>(node.wrapping_add(8));
    }
}

// Typed from 00b98610
/// A render state put back, `cdecl(state, value)`: the states with a setter of their own get it with
/// `value` (7, 0xe, 0xf, 0x13 and 0x14, 0x17, 0x18 and 0x19, 0x1b, 0x34, 0x35 to 0x37, 0x38 to 0x3a,
/// 0x3b, 0x98, 0xa8, 0xbe, 0xc3); any other state sends the render-state cache's own stored value for
/// it (+0x124 + 8 · state) back through its slot 0x68, unlocked.
pub fn restore_state(e: &mut Engine, state: u32, value: u32) {
    let setter = match state {
        7 => 0x0071_4a40,
        0xe => 0x0071_4a60,
        0xf => 0x0071_4aa0,
        0x13 | 0x14 => 0x0071_4b00,
        0x17 => 0x0071_4a80,
        0x18 | 0x19 => 0x0071_4ac0,
        0x1b => 0x0071_4ae0,
        0x34 => 0x004e_cb40,
        0x35..=0x37 => 0x0071_4b20,
        0x38..=0x3a => 0x0071_4b50,
        0x3b => 0x0071_4b80,
        0x98 => 0x004e_b510,
        0xa8 => 0x004e_ced0,
        0xbe => 0x00b9_85f0,
        0xc3 => 0x0071_4bf0,
        _ => {
            let renderer = packet::renderer_singleton(e);
            let cache = packet::render_state_cache(e, renderer);
            let stored = packet::stored_render_state(e, cache, state);
            if crate::slot_target(e, cache, 0x68) == 0x00e8_8780 {
                typed_call!(
                    shader::FAMILY,
                    0x00e8_8780,
                    state::set_render_state(e, Ptr::new(cache), state, stored, 0u32, 0u8)
                );
            } else {
                e.vcall(cache, 0x68, &args![state, stored, 0u32, 0u32]);
            }
            return;
        }
    };
    e.call(setter, &args![value]);
}

// Typed from 00be20b0
/// The alpha function of a geometry's state (a shader's slot 0x84), `thiscall(shader, state block)`:
/// the alpha property (the block's first word) gives its function (bits 10 to 12 of +0x18) and
/// reference (+0x1a), unlocked.
pub fn alpha_function_of(e: &mut Engine, _this: u32, block: u32) {
    let property = packet::state_property(e, block, 0);
    let flags = u32::from(packet::ni_property_flags(e, property));
    let reference = u32::from(packet::alpha_reference(e, property));
    typed_call!(
        shader::FAMILY,
        0x00b9_7f20,
        state::set_alpha_function(e, (flags >> 10) & 7, reference, 0u32)
    );
}

/// `00b97ed0` (the alpha test's setter, the shader family's): called by address with the whole word,
/// whose upper bytes the exe leaves as the state block's (the setter reads its low byte).
const SET_ALPHA_TEST_ENABLE: u32 = 0x00b9_7ed0;

// Typed from 00be20e0
/// The alpha test and cull mode of a geometry's state (a shader's slot 0x88), `thiscall(shader, state
/// block, flag)`: the renderer's cull mode 3 when the block's second property (+0x10) has both bits 10
/// and 11 of its flags or the pass type being rendered is 0x56 or 0x57, else 1; then the alpha test
/// from a word whose low byte is 0, or with the flag and an alpha property its flags' bit 9, and whose
/// upper bytes are the block address's.
pub fn alpha_test_of(e: &mut Engine, _this: u32, block: u32, flag: bool) {
    let mut both = false;
    let pass_type = packet::rendering_pass_type(e);
    if block != 0 {
        let second = packet::state_property(e, block, 4);
        if second != 0 && packet::ni_property_flags(e, second) & 0xc00 == 0xc00 {
            both = true;
        }
    }
    if pass_type.wrapping_sub(0x56) <= 1 {
        both = true;
    }
    let mut low = 0u32;
    if flag && block != 0 {
        let first = packet::state_property(e, block, 0);
        if first != 0 {
            low = u32::from(packet::ni_property_flags(e, first) >> 9) & 1;
        }
    }
    typed_call!(
        shader::FAMILY,
        0x00b9_84d0,
        state::set_cull_mode_of_renderer(e, u32::from(both) * 2 + 1, 0u32)
    );
    e.call(
        SET_ALPHA_TEST_ENABLE,
        &args![(block & 0xffff_ff00) | low, 0u32],
    );
}

// Typed from 00be0e50
/// The pass end's states (a shader's slot 0x110), `thiscall(shader, unread word)`: unless the pass type
/// being rendered is 0x252 to 0x256, alpha blending on, z-writes off and the z function 2, unlocked.
pub fn end_states(e: &mut Engine, _this: u32, _unused: u32) {
    let pass_type = packet::rendering_pass_type(e);
    if (0x252..=0x256).contains(&pass_type) {
        return;
    }
    typed_call!(
        shader::FAMILY,
        0x00b9_7fa0,
        state::set_alpha_blend_enable(e, 1u8, 0u32)
    );
    typed_call!(
        shader::FAMILY,
        0x00b9_7e30,
        state::set_z_write_enable(e, 0u8, 0u32)
    );
    typed_call!(
        shader::FAMILY,
        0x00b9_7e80,
        state::set_z_function(e, 2u32, 0u32)
    );
}

// Typed from 00be0e90
/// The pass end's states put back (a shader's slot 0x114), `thiscall(shader)` with no stack words:
/// alpha blending off, z-writes on and the z function 3, unlocked.
pub fn end_states_restored(e: &mut Engine, _this: u32) {
    typed_call!(
        shader::FAMILY,
        0x00b9_7fa0,
        state::set_alpha_blend_enable(e, 0u8, 0u32)
    );
    typed_call!(
        shader::FAMILY,
        0x00b9_7e30,
        state::set_z_write_enable(e, 1u8, 0u32)
    );
    typed_call!(
        shader::FAMILY,
        0x00b9_7e80,
        state::set_z_function(e, 3u32, 0u32)
    );
}

/// The element mask table (`011ae640`, a word an element).
const ELEMENT_BITS: u32 = 0x011a_e640;

// Typed from 00be21b0
/// The current pass's stages chosen by its mask byte (+0x4c, sign-extended), no words: each stage whose
/// word in `011ae640` shares a bit with the mask, up to the first null one, is bound (the render
/// family's `00e7eb00`, not forced) when it has a texture (+8). The count (+0x18) is read once.
pub fn bind_masked_stages(e: &mut Engine) {
    let pass = packet::current_pass(e);
    let count = e.global::<u32>(pass.wrapping_add(0x18));
    let mask = sign_extend_byte(e.global::<u8>(pass.wrapping_add(0x4c)));
    let mut index = 0u32;
    while index < count {
        if e.global::<u32>(ELEMENT_BITS.wrapping_add(index.wrapping_mul(4))) & mask != 0 {
            let stages = e.global::<u32>(pass.wrapping_add(0x24));
            let stage = e.global::<u32>(stages.wrapping_add(index.wrapping_mul(4)));
            if stage == 0 {
                return;
            }
            if own::stage_texture(e, stage) != 0 {
                let _: bool = typed_call!(
                    crate::render::FAMILY,
                    0x00e7_eb00,
                    crate::render::stage::bind(e, Ptr::new(stage), 0u8)
                );
            }
        }
        index += 1;
    }
}

/// A byte sign-extended to a word (`MOVSX`).
#[inline(always)]
fn sign_extend_byte(byte: u8) -> u32 {
    i32::from(i8::from_ne_bytes([byte])) as u32
}

/// The current pass's constants target (`00e7f7c0`, `thiscall(pass)`).
const PASS_TARGET: u32 = 0x00e7_f7c0;

/// A shader object's shader set (its slot 0x8c or 0x7c, `thiscall(object, shader, lock)`): the
/// render-state cache's vertex (`00e90e00`) or pixel (`00e88870`) shader set, the shader family's,
/// directly when the slot holds it.
#[inline(always)]
fn shader_set(e: &mut Engine, target: u32, object: u32, value: u32) {
    match target {
        0x00e9_0e00 => typed_call!(
            shader::FAMILY,
            0x00e9_0e00,
            state::set_vertex_shader(e, Ptr::new(object), value, 0u8)
        ),
        0x00e8_8870 => typed_call!(
            shader::FAMILY,
            0x00e8_8870,
            state::set_pixel_shader(e, Ptr::new(object), value, 0u8)
        ),
        _ => {
            e.call(target, &args![object, value, 0u32]);
        }
    }
}

// Typed from 00be1f90
/// The pass begin's end (a shader's slot 0xf4), `thiscall(shader, unread word)`: the object `00e7f7c0`
/// answers for the current pass gets, through its slot 0x8c, what the pass's +0x5c answers from its slot
/// 0x84; then, with a pass word +0x44, through its slot 0x7c what that answers from its slot 0x7c. Each
/// target slot is read before the call that feeds it, as in the exe; the getters take no word, so the
/// pushed 0 stays as the target's last word.
pub fn begin_done(e: &mut Engine, _this: u32, _unused: u32) {
    let pass = packet::current_pass(e);
    let object = e.call(PASS_TARGET, &args![pass]).u32();
    let pass = packet::current_pass(e);
    let source = e.global::<u32>(pass.wrapping_add(0x5c));
    let vtable = e.global::<u32>(object);
    let value = e.vcall(source, 0x84, &args![]).u32();
    let target = e.global::<u32>(vtable.wrapping_add(0x8c));
    shader_set(e, target, object, value);
    let pass = packet::current_pass(e);
    let source = e.global::<u32>(pass.wrapping_add(0x44));
    if source != 0 {
        let vtable = e.global::<u32>(object);
        let value = e.vcall(source, 0x7c, &args![]).u32();
        let target = e.global::<u32>(vtable.wrapping_add(0x7c));
        shader_set(e, target, object, value);
    }
}

// Typed from 00b55520
/// `BSShaderManager::GetFogProperty` (engine map), `cdecl(index)`: the fog property of shadow scene
/// node `index`.
pub fn fog_property(e: &mut Engine, index: u32) -> u32 {
    packet::scene_fog_property(e, index)
}
