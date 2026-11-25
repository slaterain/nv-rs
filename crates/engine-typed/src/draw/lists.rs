//! The list draws the world draw still made through translations by address (the drawing thread's "still live"
//! list, `measure/typed-draw/DRAW-THREAD.md`): a register group holder's own pass list (`00b99990`) and a batch
//! renderer's pass types (`BSBatchRenderer::RenderBatches`, `00b99e20`, slot 0x90). Typed, they read and write the
//! lists' words through the drawing thread's view ([`super::detach`]), so with `NV_DRAW_DETACH=1` they need no
//! words handed back around them. Each list's spare chain stays in guest memory, as everywhere.

use super::detach;
use crate::shader::layout::{BSBatchRenderer, PassGroup, PassList, RenderPass};
use crate::shader::{self, batch, state};
use engine_core::ptr::Ptr;
use engine_core::{args, typed_call, Engine};

/// Whether the card has alpha to coverage (`011f9421`).
const COVERAGE_AVAILABLE: u32 = 0x011f_9421;
/// The pass map's `GetAt` (`00853130`, `thiscall(map, key, out)`, by address).
const MAP_GET_AT: u32 = 0x0085_3130;

// Typed from 00b630f0
/// `BSShaderAccumulator::IsShadowPass` (engine map), `cdecl(type)`, the answer in `AL`: whether the pass type is a
/// shadow pass, 0x1c4 to 0x1c7, 0x231 to 0x234 or 0x284 to 0x287.
pub fn is_shadow_pass(_e: &mut Engine, pass_type: u32) -> bool {
    matches!(pass_type, 0x1c4..=0x1c7 | 0x231..=0x234 | 0x284..=0x287)
}

/// [`is_shadow_pass`] as the draw and shader families' code asks it (a typed call).
#[inline(always)]
pub(crate) fn shadow_pass(e: &mut Engine, pass_type: u32) -> bool {
    typed_call!(super::FAMILY, 0x00b6_30f0, is_shadow_pass(e, pass_type))
}

// Typed from 00b99990
/// A register group holder's own pass list drawn, `cdecl(list, second)`: when the list counts passes (+0x10),
/// the pass ended, each listed pass with a geometry drawn (`RenderPassImmediately(pass, its type, 1, second,
/// 1)`; the walk stops at the first null pass), the list's spare chain given back to the pool (`00b63e90`), its
/// nodes kept as the new spare chain and the list cleared, and the pass ended again.
pub fn own_list_draw(e: &mut Engine, list: Ptr<PassList>, second: u32) {
    let at = list.addr();
    if detach::word(e, at.wrapping_add(PassList::uiCount.off)) == 0 {
        return;
    }
    typed_call!(shader::FAMILY, 0x00b9_88e0, batch::end_pass(e));
    let node = detach::word(e, at.wrapping_add(PassList::pHead.off));
    if node != 0 {
        let mut next = e.global::<u32>(node);
        let mut pass = e.global::<u32>(node.wrapping_add(8));
        while pass != 0 {
            if e.global::<u32>(pass) != 0 {
                let pass_type =
                    u32::from(e.get(Ptr::<RenderPass>::new(pass), RenderPass::usPassEnum));
                typed_call!(
                    shader::FAMILY,
                    0x00b9_94f0,
                    batch::render_pass(e, Ptr::new(pass), pass_type, 1u32, second, 1u32)
                );
            }
            if next == 0 {
                break;
            }
            pass = e.global::<u32>(next.wrapping_add(8));
            next = e.global::<u32>(next);
        }
    }
    let _: u32 = typed_call!(shader::FAMILY, 0x00b6_3e90, batch::release_spare(e, list));
    let head = detach::word(e, at.wrapping_add(PassList::pHead.off));
    e.set(list, PassList::pSpare, head);
    detach::set_word(e, at.wrapping_add(PassList::pHead.off), 0);
    detach::set_word(e, at.wrapping_add(PassList::pTail.off), 0);
    detach::set_word(e, at.wrapping_add(PassList::uiCount.off), 0);
    typed_call!(shader::FAMILY, 0x00b9_88e0, batch::end_pass(e));
}

/// The cull mode, the alpha test, alpha to coverage and the blend flag a group's list `index` is drawn with.
fn list_state(index: u32) -> (u32, u8, u8, u8) {
    match index {
        0 => (1, 0, 0, 0),
        1 => (3, 0, 0, 0),
        2 => (3, 1, 1, 1),
        3 => (1, 1, 1, 1),
        _ => (1, 1, 0, 1),
    }
}

// Typed from 00b99e20
/// `BSBatchRenderer::RenderBatches` (Xbox PDB), `thiscall(renderer, first, last, third)`: the pass types
/// `first..=last` (1..=0x2f1 when `first` is 0) of the renderer's active list, in its order, without taking them
/// off it. The fog and ambient copy (`00bbb840`) first when `011f94a6` is set. For each listed type: its group
/// (the pass map, `00853130`); a shadow pass while the accumulator's +0x164 or +0x165 is set only has the group
/// cleared. Otherwise each of the group's five lists with a head is drawn with its cull mode, alpha test and
/// alpha to coverage (where the card has it): every pass through `RenderPassImmediately(pass, type, flag,
/// third, 0)`, the flag the blend byte in the low byte of `last` as passed; then alpha to coverage off (twice
/// where the card has it, which also counts the coverage counter down) and the two list counters down when not
/// zero. The group is cleared after (`00b99a50`). Last: the cull mode 1, the alpha test off and the pass ended.
pub fn render_batches(
    e: &mut Engine,
    this: Ptr<BSBatchRenderer>,
    first: u32,
    last: u32,
    third: u32,
) {
    let base = this.addr();
    let (low, high) = if first == 0 {
        (1, 0x2f1)
    } else {
        (first, last)
    };
    if super::packet::fog_ambient(e) {
        typed_call!(super::FAMILY, 0x00bb_b840, super::own::fog_ambient(e));
    }
    let head = base.wrapping_add(BSBatchRenderer::kActivePassIndexList.off);
    if detach::word(e, head.wrapping_add(4)) != 0 || detach::half(e, head) != 0 {
        let mut node = head;
        while node != 0 {
            let pass_type = u32::from(detach::half(e, node));
            if pass_type > high || pass_type < low {
                node = detach::word(e, node.wrapping_add(4));
                continue;
            }
            let group = e.with_stack(4, |e, slot| {
                e.set_global(slot.addr(), 0u32);
                e.call(MAP_GET_AT, &args![base.wrapping_add(0xc), pass_type, slot]);
                e.global::<u32>(slot.addr())
            });
            let skip = shadow_pass(e, pass_type) && {
                let acc = typed_call!(
                    crate::render::FAMILY,
                    0x00b4_f5c0,
                    crate::render::passes::shader_accumulator(e)
                );
                super::packet::skip_type_13(e, Ptr::new(acc)) || {
                    let acc = typed_call!(
                        crate::render::FAMILY,
                        0x00b4_f5c0,
                        crate::render::passes::shader_accumulator(e)
                    );
                    super::packet::shadow_groups_set(e, Ptr::new(acc))
                }
            };
            let offset = group.wrapping_mul(0x68);
            if !skip {
                for index in 0..5u32 {
                    let groups = e.get(this, BSBatchRenderer::pRenderPassA);
                    let list = index
                        .wrapping_mul(0x14)
                        .wrapping_add(offset)
                        .wrapping_add(groups);
                    if detach::word(e, list.wrapping_add(PassList::pHead.off)) == 0 {
                        continue;
                    }
                    let (cull, test, coverage, blend) = list_state(index);
                    typed_call!(
                        shader::FAMILY,
                        0x00b9_84d0,
                        state::set_cull_mode_of_renderer(e, cull, u32::from(index != 0))
                    );
                    typed_call!(
                        shader::FAMILY,
                        0x00b9_7ed0,
                        state::set_alpha_test_enable(e, test, u32::from(test))
                    );
                    if e.global::<u8>(COVERAGE_AVAILABLE) != 0 {
                        typed_call!(
                            shader::FAMILY,
                            0x00b9_8540,
                            state::set_alpha_to_coverage(e, coverage, 1u32)
                        );
                    }
                    // The exe keeps the blend byte in the low byte of `last`'s stack word and passes the word.
                    let flag = (last & 0xffff_ff00) | u32::from(blend);
                    let groups = e.get(this, BSBatchRenderer::pRenderPassA);
                    let list = index
                        .wrapping_mul(0x14)
                        .wrapping_add(offset)
                        .wrapping_add(groups);
                    let mut node = detach::word(e, list.wrapping_add(PassList::pHead.off));
                    while node != 0 {
                        let pass = e.global::<u32>(node.wrapping_add(8));
                        node = e.global::<u32>(node);
                        typed_call!(
                            shader::FAMILY,
                            0x00b9_94f0,
                            batch::render_pass(e, Ptr::new(pass), pass_type, flag, third, 0u32)
                        );
                    }
                    if e.global::<u8>(COVERAGE_AVAILABLE) != 0 {
                        super::own::coverage_count_down(e);
                        typed_call!(
                            shader::FAMILY,
                            0x00b9_8540,
                            state::set_alpha_to_coverage(e, 0u8, 0u32)
                        );
                    }
                    typed_call!(
                        shader::FAMILY,
                        0x00b9_8540,
                        state::set_alpha_to_coverage(e, 0u8, 0u32)
                    );
                    super::own::list_counters_down(e);
                }
            }
            let groups = e.get(this, BSBatchRenderer::pRenderPassA);
            typed_call!(
                shader::FAMILY,
                0x00b9_9a50,
                batch::clear_group(e, Ptr::<PassGroup>::new(groups.wrapping_add(offset)), 0u32)
            );
            node = detach::word(e, node.wrapping_add(4));
        }
    }
    typed_call!(
        shader::FAMILY,
        0x00b9_84d0,
        state::set_cull_mode_of_renderer(e, 1u32, 0u32)
    );
    typed_call!(
        shader::FAMILY,
        0x00b9_7ed0,
        state::set_alpha_test_enable(e, 0u8, 0u32)
    );
    typed_call!(shader::FAMILY, 0x00b9_88e0, batch::end_pass(e));
}

/// The memory manager object its free takes as `this` (`011f6238`), and the free (`00aa4060`, `thiscall(manager,
/// block)`, by address).
const MEMORY_MANAGER: u32 = 0x011f_6238;
const MEMORY_FREE: u32 = 0x00aa_4060;

// Typed from 00b64ec0
/// A `BSSimpleList<unsigned short>` cleared (`BSBatchRenderer::kActivePassIndexList`), `fastcall(list)`: while the
/// list has a next node, the node is unlinked (its own next kept), its own list cleared (this again: its next is
/// already cleared, so only its item) and freed through the memory manager; then the list's 16-bit item cleared. The
/// words go through the drawing thread's view ([`detach`]), so the world draw's active lists need no words handed
/// back around it.
pub fn clear_simple_list(e: &mut Engine, list: u32) {
    let next_at = list.wrapping_add(4);
    if detach::word(e, next_at) != 0 {
        loop {
            let node = detach::word(e, next_at);
            let after = detach::word(e, node.wrapping_add(4));
            detach::set_word(e, node.wrapping_add(4), 0);
            let node = detach::word(e, next_at);
            if node != 0 {
                clear_simple_list(e, node);
                e.call(MEMORY_FREE, &args![MEMORY_MANAGER, node]);
            }
            detach::set_word(e, next_at, after);
            if after == 0 {
                break;
            }
        }
    }
    detach::set_half(e, list, 0);
}

/// The 33 pass types `00b98970` answers for (the immediates of its code), all within `0x28a..=0x2cd`.
fn is_listed_pass(pass_type: u32) -> bool {
    matches!(
        pass_type,
        0x28a
            | 0x28d
            | 0x28e
            | 0x291
            | 0x292
            | 0x295
            | 0x296
            | 0x299
            | 0x29b
            | 0x29c
            | 0x29f
            | 0x2a0
            | 0x2a3
            | 0x2a4
            | 0x2a7
            | 0x2a8
            | 0x2ac
            | 0x2af
            | 0x2b0
            | 0x2b3
            | 0x2b4
            | 0x2b5
            | 0x2b8
            | 0x2bb
            | 0x2bd
            | 0x2be
            | 0x2c1
            | 0x2c2
            | 0x2c5
            | 0x2c6
            | 0x2c7
            | 0x2ca
            | 0x2cd
    )
}

// Typed from 00b98970
/// `thiscall(renderer, low, high)`: whether the renderer's active pass type list (+0x2c: its item, then the next
/// node at +4 of each) holds a type in `low..=high` that is one of [`is_listed_pass`]'s. The walk reads the list's
/// words through the drawing thread's view ([`detach`]).
pub fn lists_pass_in(e: &mut Engine, this: Ptr<BSBatchRenderer>, low: u32, high: u32) -> bool {
    let mut node = this
        .addr()
        .wrapping_add(BSBatchRenderer::kActivePassIndexList.off);
    while node != 0 {
        let pass_type = u32::from(detach::half(e, node));
        if pass_type <= high && pass_type >= low && is_listed_pass(pass_type) {
            return true;
        }
        node = detach::word(e, node.wrapping_add(4));
    }
    false
}
