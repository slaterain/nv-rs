//! The world draw's drivers: the two halves the world pass calls (and the secondary scenes' draw that
//! does both), the accumulator's render-mode draw and finish, the render mode 0 halves, the pass type
//! range draw that interleaves the batch renderers by pass type, the register group draw, the alpha
//! sort, and the batch renderers' "has a pass of these types" and active list clears.

use super::own::{self, hold_accumulator};
use super::packet::{self, finish_accumulating, set_camera};
use super::{alpha, FAMILY};
use crate::shader::layout::{BSBatchRenderer, BSShaderAccumulator};
use crate::shader::{self, batch};
use engine_core::ptr::Ptr;
use engine_core::{args, typed_call, Engine};

/// The render mode's preparation (`00b4f450`, `cdecl(mode)`).
const RENDER_MODE_PREPARE: u32 = 0x00b4_f450;
/// Whether a batch renderer's active list holds a pass type in a range (`00b98970`,
/// `thiscall(renderer, low, high)`).
const HAS_LISTED_PASS: u32 = 0x00b9_8970;
/// The accumulator's slots: the camera (0x8c), the draw (0xa8), the finish (0xac).
const SLOT_SET_CAMERA: u32 = 0x8c;
const SLOT_DRAW: u32 = 0xa8;
const SLOT_FINISH: u32 = 0xac;

/// The camera into the accumulator (its slot 0x8c, [`set_camera`] when it holds that).
#[inline(always)]
fn camera_into(e: &mut Engine, acc: u32, camera: u32) {
    if crate::slot_target(e, acc, SLOT_SET_CAMERA) == 0x00d4_7a40 {
        typed_call!(FAMILY, 0x00d4_7a40, set_camera(e, Ptr::new(acc), camera));
    } else {
        e.vcall(acc, SLOT_SET_CAMERA, &args![camera]);
    }
}

/// What both the world pass's first half and the secondary scenes' draw do first: the camera to the
/// scene renderer, noted, the accumulator held (condition 7), the camera into the accumulator.
/// False (nothing done) without a scene renderer or a camera.
#[inline(always)]
fn start(e: &mut Engine, camera: u32, acc: u32) -> bool {
    let scene = packet::scene_renderer(e);
    if scene == 0 || camera == 0 {
        return false;
    }
    typed_call!(FAMILY, 0x004e_9bb0, packet::hand_camera(e, scene, camera));
    packet::note_camera(e, camera);
    typed_call!(FAMILY, 0x00b5_4b10, hold_accumulator(e, acc));
    camera_into(e, acc, camera);
    true
}

// Typed from 00b6ba20
/// The world pass's first half, `cdecl(camera, accumulator, third)`: with a scene renderer and a
/// camera, [`start`], then a jump to the accumulator's slot 0xa8 (the render mode's draw,
/// [`accumulator_draw`]) with the three words still on the stack. The jump stays a call through the
/// slot: a direct call could not hand on those words.
pub fn world_draw_first(e: &mut Engine, camera: u32, acc: Ptr<BSShaderAccumulator>, third: u32) {
    if !start(e, camera, acc.addr()) {
        return;
    }
    if super::detach::switch_on() {
        // The drawing thread's step 1: the lists the draw walks leave guest memory for the draw's view.
        let mut words = Vec::new();
        for i in 0..packet::renderer_count(e, acc) {
            super::detach::list_words(e, packet::renderer(e, acc, i).addr(), &mut words);
        }
        super::detach::open(e, &words);
    }
    e.vcall(acc.addr(), SLOT_DRAW, &args![camera, acc, third]);
}

// Typed from 00b6b930
/// The world pass's second half, `cdecl(camera, accumulator, third)`: a jump to the accumulator's slot
/// 0xac (the render mode's finish, [`accumulator_finish`]) with the three words still on the stack,
/// kept a call through the slot for them.
pub fn world_draw_second(e: &mut Engine, camera: u32, acc: Ptr<BSShaderAccumulator>, third: u32) {
    e.vcall(acc.addr(), SLOT_FINISH, &args![camera, acc, third]);
    // Step 1 of the drawing thread: the view's words back to guest memory as the draw ends.
    super::detach::close(e);
}

// Typed from 00b6c0d0
/// The secondary scenes' draw (water reflections, menus, the world pass's extra scenes),
/// `cdecl(camera, accumulator)`: [`start`] and the accumulator's slot 0xa8 when there is a scene
/// renderer and a camera; always its slot 0xac, whose answer it passes on. Both slots are direct when
/// they hold the accumulator's own ([`accumulator_finish`] answers nothing: 0, as its translation's
/// uniform form does).
pub fn scene_draw(e: &mut Engine, camera: u32, acc: Ptr<BSShaderAccumulator>) -> u32 {
    let base = acc.addr();
    if start(e, camera, base) {
        if crate::slot_target(e, base, SLOT_DRAW) == 0x00b6_6520 {
            typed_call!(FAMILY, 0x00b6_6520, accumulator_draw(e, acc));
        } else {
            e.vcall(base, SLOT_DRAW, &[]);
        }
    }
    if crate::slot_target(e, base, SLOT_FINISH) == 0x00b6_6570 {
        typed_call!(FAMILY, 0x00b6_6570, accumulator_finish(e, acc));
        0
    } else {
        e.vcall(base, SLOT_FINISH, &[]).u32()
    }
}

// Typed from 00b66520
/// The accumulator's slot 0xa8 (the render mode's draw), `thiscall(accumulator)`: the mode to its
/// preparation, then mode 0's [`first_half`], or the render mode table's entry (entry 0 for an empty
/// one), each with the accumulator.
pub fn accumulator_draw(e: &mut Engine, acc: Ptr<BSShaderAccumulator>) {
    let mode = packet::render_mode(e, acc);
    e.call(RENDER_MODE_PREPARE, &args![mode]);
    let mode = packet::render_mode(e, acc);
    if mode == 0 {
        typed_call!(FAMILY, 0x00b6_5ae0, first_half(e, acc));
        return;
    }
    let mut function = packet::render_mode_draw(e, mode);
    if function == 0 {
        function = packet::render_mode_draw(e, 0);
    }
    super::detach::suspended(e, |e| e.call(function, &args![acc]));
}

// Typed from 00b66570
/// The accumulator's slot 0xac (the render mode's finish), `thiscall(accumulator)`: mode 0's
/// [`second_half`], the scene renderer's word +0xc cleared, and `FinishAccumulating` (the camera
/// cleared).
pub fn accumulator_finish(e: &mut Engine, acc: Ptr<BSShaderAccumulator>) {
    if packet::render_mode(e, acc) == 0 {
        typed_call!(FAMILY, 0x00b6_5c60, second_half(e, acc));
    }
    packet::clear_scene_word(e);
    typed_call!(FAMILY, 0x00aa_0fe0, finish_accumulating(e, acc));
}

// Typed from 00b65ae0
/// Render mode 0's draw, `cdecl(accumulator)`: with a camera, mode 8 draws the pass types 6..8 and
/// 0x1d3..0x1d6 and mode 7 the types 0x10..0x13 and 0x1c8..0x1cb, both then clearing the active lists.
/// Any other mode draws register group 0xa, makes the render pass tree when asked (not in mode 7), the
/// pass types 1..0x25b (split at 0x237 with clip planes off for the rest when +0x164 is set), group 0
/// with `011a9608` set to 1, the decal pass, the types 0x25c..0x287, sorts the alphas and draws the
/// alpha group the second half does not.
pub fn first_half(e: &mut Engine, acc: Ptr<BSShaderAccumulator>) {
    if packet::camera(e, acc) == 0 {
        return;
    }
    let mode = packet::render_mode(e, acc);
    if mode == 8 || mode == 7 {
        let (a, b): ((u32, u32), (u32, u32)) = if mode == 8 {
            ((6, 8), (0x1d3, 0x1d6))
        } else {
            ((0x10, 0x13), (0x1c8, 0x1cb))
        };
        typed_call!(
            FAMILY,
            0x00b6_3770,
            draw_types(e, acc, a.0, a.1, 0u32, -1i32)
        );
        typed_call!(
            FAMILY,
            0x00b6_3770,
            draw_types(e, acc, b.0, b.1, 0u32, -1i32)
        );
        typed_call!(FAMILY, 0x00b6_5580, clear_active_lists(e, acc));
        return;
    }
    typed_call!(FAMILY, 0x00b6_39e0, draw_group(e, acc, 0xau32, 0u32));
    if packet::pass_tree_asked(e) && packet::render_mode(e, acc) != 7 {
        own::make_pass_tree(e, acc.addr());
    }
    if packet::split_clip(e, acc) {
        typed_call!(
            FAMILY,
            0x00b6_3770,
            draw_types(e, acc, 1u32, 0x237u32, 0u32, -1i32)
        );
        own::clip_planes(e, 0);
        typed_call!(
            FAMILY,
            0x00b6_3770,
            draw_types(e, acc, 0x238u32, 0x25bu32, 0u32, -1i32)
        );
        own::clip_planes(e, 1);
    } else {
        typed_call!(
            FAMILY,
            0x00b6_3770,
            draw_types(e, acc, 1u32, 0x25bu32, 0u32, -1i32)
        );
    }
    let saved = own::swap_group_zero_flag(e, 1);
    typed_call!(FAMILY, 0x00b6_39e0, draw_group(e, acc, 0u32, 0u32));
    own::swap_group_zero_flag(e, saved);
    typed_call!(FAMILY, 0x00b6_4e30, decal_pass(e, acc));
    typed_call!(
        FAMILY,
        0x00b6_3770,
        draw_types(e, acc, 0x25cu32, 0x287u32, 0u32, -1i32)
    );
    typed_call!(FAMILY, 0x00b6_3a50, sort_alphas(e, acc));
    let group = u32::from(!packet::late_alpha_group(e, acc));
    typed_call!(FAMILY, 0x00b6_50c0, alpha::draw_alpha_group(e, acc, group));
}

// Typed from 00b64e30
/// The decal pass, `thiscall(accumulator)` (named `BSRenderState::SetRenderState` in the engine map,
/// but an accumulator method): with decals on and a render mode other than 8, register groups 2 and 3
/// (flag 1) drawn in the decal state ([`own::decal_state_on`], [`own::decal_state_off`]).
pub fn decal_pass(e: &mut Engine, acc: Ptr<BSShaderAccumulator>) {
    if !packet::render_decals(e, acc) || packet::render_mode(e, acc) == 8 {
        return;
    }
    own::decal_state_on(e);
    typed_call!(FAMILY, 0x00b6_39e0, draw_group(e, acc, 2u32, 1u32));
    typed_call!(FAMILY, 0x00b6_39e0, draw_group(e, acc, 3u32, 1u32));
    own::decal_state_off(e);
}

// Typed from 00b65c60
/// Render mode 0's finish, `cdecl(accumulator)`: with +0x38 set and a pass of the types 0x288..0x2ce
/// listed, the water reflection is copied when there is one and the camera goes to the renderer. Then
/// the pass types 0x288..0x289, 0x28a..0x2ab and 0x2ac..0x2ce, the borrowed copy given back (with
/// +0x38), register group 9 (flag 1), the other alpha group, alpha blending off twice, the types
/// 0x2cf..0x2d7 and register group 1.
pub fn second_half(e: &mut Engine, acc: Ptr<BSShaderAccumulator>) {
    if packet::copy_reflection(e, acc)
        && typed_call!(
            FAMILY,
            0x00b6_3120,
            has_listed_pass(e, acc, 0x288u32, 0x2ceu32)
        )
    {
        if packet::reflection_source(e) != 0 {
            own::copy_reflection(e);
        }
        let camera = packet::camera(e, acc);
        let renderer = packet::renderer_singleton(e);
        typed_call!(
            FAMILY,
            0x004e_9bb0,
            packet::hand_camera(e, renderer, camera)
        );
    }
    typed_call!(
        FAMILY,
        0x00b6_3770,
        draw_types(e, acc, 0x288u32, 0x289u32, 0u32, -1i32)
    );
    typed_call!(
        FAMILY,
        0x00b6_3770,
        draw_types(e, acc, 0x28au32, 0x2abu32, 0u32, -1i32)
    );
    typed_call!(
        FAMILY,
        0x00b6_3770,
        draw_types(e, acc, 0x2acu32, 0x2ceu32, 0u32, -1i32)
    );
    if packet::copy_reflection(e, acc) {
        own::return_reflection(e);
    }
    typed_call!(FAMILY, 0x00b6_39e0, draw_group(e, acc, 9u32, 1u32));
    let group = u32::from(packet::late_alpha_group(e, acc));
    typed_call!(FAMILY, 0x00b6_50c0, alpha::draw_alpha_group(e, acc, group));
    own::alpha_blend_off(e);
    own::alpha_blend_off(e);
    typed_call!(
        FAMILY,
        0x00b6_3770,
        draw_types(e, acc, 0x2cfu32, 0x2d7u32, 0u32, -1i32)
    );
    typed_call!(FAMILY, 0x00b6_39e0, draw_group(e, acc, 1u32, 0u32));
}

/// The shadow pass types (`BSShaderAccumulator::IsShadowPass`, inlined in the exe here).
#[inline(always)]
fn is_shadow_pass(pass_type: u32) -> bool {
    (0x1c4..=0x1c7).contains(&pass_type)
        || (0x231..=0x234).contains(&pass_type)
        || pass_type.wrapping_sub(0x284) <= 3
}

/// The most batch renderers the accumulator holds (its array at +0x174, two words, as the exe has it).
const MAX_RENDERERS: u32 = 2;

// Typed from 00b63770
/// `thiscall(accumulator, low, high, second, group)`: draws the pass types `low..=high` of the batch
/// renderers (or, for `group` >= 0, of each one's register group `group`'s own renderer), interleaved
/// by pass type. Each renderer gets the range and its first list ready (`00b9a090`); then while any is
/// active, the one with the lowest pass type (then bucket, then the one whose first-pass byte is set)
/// draws its list: a shadow pass type with +0x164 or +0x165 set through the group-clearing draw
/// (`00b9a3c0`), any other through `00b9a120` with `second`. Then the register groups are emptied and
/// the pass ends. With the fog-coloured ambient flag set, the ambient is set up first.
///
/// The exe keeps the renderers and the groups after them in a four-word stack array: two renderers,
/// the most the accumulator holds; this keeps them in one array the same way.
pub fn draw_types(
    e: &mut Engine,
    acc: Ptr<BSShaderAccumulator>,
    low: u32,
    high: u32,
    second: u32,
    group: i32,
) {
    if packet::fog_ambient(e) {
        typed_call!(FAMILY, 0x00bb_b840, own::fog_ambient(e));
    }
    let count = packet::renderer_count(e, acc);
    let n = count.min(MAX_RENDERERS);
    // Renderers at 0.., groups at 2.., as the exe's array.
    let mut local = [0u32; MAX_RENDERERS as usize + 2];
    if let Ok(g) = u32::try_from(group) {
        for i in 0..n {
            let renderer = packet::renderer(e, acc, i);
            let holder = packet::group_holder(e, renderer, g);
            local[i as usize + 2] = holder;
            local[i as usize] = packet::holder_renderer(e, holder);
        }
    } else if count != 0 {
        for i in 0..n {
            local[i as usize + 2] = 0;
        }
        for i in 0..n {
            local[i as usize] = packet::renderer(e, acc, i).addr();
        }
    }
    if count != 0 {
        let mut i = 0u32;
        loop {
            let renderer = local[(i as usize).min(MAX_RENDERERS as usize + 1)];
            own::clear_cursor_pass(e, acc, i);
            if renderer != 0 {
                own::set_pass_range(e, renderer, low, high);
                let pass = own::cursor_pass_at(e, acc, i);
                let bucket = own::cursor_bucket_at(e, acc, i);
                let first = own::cursor_first_at(e, acc, i);
                let active = typed_call!(
                    shader::FAMILY,
                    0x00b9_a090,
                    batch::next_group(
                        e,
                        Ptr::<BSBatchRenderer>::new(renderer),
                        pass,
                        bucket,
                        first
                    )
                );
                own::set_cursor_active(e, acc, i, active);
            } else {
                own::set_cursor_active(e, acc, i, false);
            }
            i += 1;
            if i >= packet::renderer_count(e, acc) {
                break;
            }
        }
    }
    loop {
        let mut any = own::cursor_active(e, acc, 0);
        let mut best = 0u32;
        let mut index = 1u32;
        while index < packet::renderer_count(e, acc) {
            let active = own::cursor_active(e, acc, index);
            let better = if !any && active {
                any = true;
                true
            } else if active {
                let pass_of = |e: &Engine, k: u32| own::cursor_pass(e, acc, k);
                let bucket_of = |e: &Engine, k: u32| own::cursor_bucket(e, acc, k);
                let (p, q) = (pass_of(e, index), pass_of(e, best));
                p < q
                    || (p == q
                        && (bucket_of(e, index) < bucket_of(e, best)
                            || (bucket_of(e, index) == bucket_of(e, best)
                                && own::cursor_first(e, acc, index))))
            } else {
                false
            };
            if better {
                best = index;
            }
            index += 1;
        }
        if !any {
            break;
        }
        let pass = own::cursor_pass_at(e, acc, best);
        let pass_type = own::cursor_pass(e, acc, best);
        let renderer =
            Ptr::<BSBatchRenderer>::new(local[(best as usize).min(MAX_RENDERERS as usize + 1)]);
        let bucket = own::cursor_bucket_at(e, acc, best);
        let first = own::cursor_first_at(e, acc, best);
        let active = if is_shadow_pass(pass_type) && packet::shadow_groups(e, acc) {
            typed_call!(
                FAMILY,
                0x00b9_a3c0,
                alpha::draw_and_clear_type(e, renderer, pass, bucket, first, 0u32)
            )
        } else {
            typed_call!(
                shader::FAMILY,
                0x00b9_a120,
                batch::render_group(e, renderer, pass, bucket, first, second)
            )
        };
        own::set_cursor_active(e, acc, best, active);
    }
    let mut i = 0u32;
    while i < packet::renderer_count(e, acc) {
        let holder = local[(i as usize + 2).min(MAX_RENDERERS as usize + 1)];
        if holder != 0 {
            own::reset_holder(e, holder);
        }
        i += 1;
    }
    own::end_pass(e);
}

// Typed from 00b639e0
/// `thiscall(accumulator, group, flag)`: draws register group `group`: when the first renderer's
/// holder keeps its passes itself, each renderer's holder draws them (`00b9a4d0` with `flag`);
/// otherwise the pass types 1..0x2f1 of the group through [`draw_types`]. Nothing without a holder.
pub fn draw_group(e: &mut Engine, acc: Ptr<BSShaderAccumulator>, group: u32, flag: u32) {
    let first = packet::group_holder(e, packet::renderer(e, acc, 0), group);
    if first == 0 {
        return;
    }
    if !packet::keeps_own(e, first) {
        typed_call!(
            FAMILY,
            0x00b6_3770,
            draw_types(e, acc, 1u32, 0x2f1u32, flag, group as i32)
        );
        return;
    }
    let mut i = 0u32;
    while i < packet::renderer_count(e, acc) {
        let holder = packet::group_holder(e, packet::renderer(e, acc, i), group);
        typed_call!(FAMILY, 0x00b9_a4d0, alpha::draw_holder(e, holder, flag));
        i += 1;
    }
}

// Typed from 00b63a50
/// `thiscall(accumulator)`: sorts every batch renderer's alpha lists along the camera, with the
/// radius byte (`00b99a10`, the shader family's).
pub fn sort_alphas(e: &mut Engine, acc: Ptr<BSShaderAccumulator>) {
    let mut i = 0u32;
    while i < packet::renderer_count(e, acc) {
        let subtract = packet::subtract_radius(e, acc);
        let camera = packet::camera(e, acc);
        let renderer = packet::renderer(e, acc, i);
        typed_call!(
            shader::FAMILY,
            0x00b9_9a10,
            batch::sort_alphas(e, renderer, camera, subtract)
        );
        i += 1;
    }
}

// Typed from 00b63120
/// `thiscall(accumulator, low, high)`: whether any batch renderer's active list holds a pass type in
/// `low..=high` (`00b98970`); the walk stops at the first that does.
pub fn has_listed_pass(e: &mut Engine, acc: Ptr<BSShaderAccumulator>, low: u32, high: u32) -> bool {
    let mut found = false;
    let mut i = 0u32;
    while i < packet::renderer_count(e, acc) && !found {
        let renderer = packet::renderer(e, acc, i);
        let mut words = Vec::new();
        super::detach::candidates(e, renderer.addr(), &mut words);
        found = super::detach::suspended_for(e, &words, |e| {
            e.call(HAS_LISTED_PASS, &args![renderer, low, high]).u8()
        }) != 0;
        i += 1;
    }
    found
}

// Typed from 00b65580
/// `thiscall(accumulator)`: clears every batch renderer's active pass type list.
pub fn clear_active_lists(e: &mut Engine, acc: Ptr<BSShaderAccumulator>) {
    let mut i = 0u32;
    while i < packet::renderer_count(e, acc) {
        let renderer = packet::renderer(e, acc, i);
        own::clear_active_list(e, renderer.addr());
        i += 1;
    }
}
