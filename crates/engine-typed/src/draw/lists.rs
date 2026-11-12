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
/// `IsShadowPass` (`00b630f0`, `cdecl(type)`, by address).
const IS_SHADOW_PASS: u32 = 0x00b6_30f0;
/// The pass map's `GetAt` (`00853130`, `thiscall(map, key, out)`, by address).
const MAP_GET_AT: u32 = 0x0085_3130;

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
            let skip = e.call(IS_SHADOW_PASS, &args![pass_type]).u8() != 0 && {
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
