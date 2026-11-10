//! The world draw's alpha pass and the batch renderer's group draws it shares with the opaque passes:
//! the alpha group draw that alternates two batch renderers back to front, the distance of an alpha
//! group, the clear of a drawn alpha group, a register group holder's draw, and the shadow types' draw
//! that empties a type's group before drawing it.

use super::{own, packet, FAMILY};
use crate::shader::layout::{BSBatchRenderer, BSShaderAccumulator, PassGroup};
use crate::shader::{self, batch};
use engine_core::ptr::Ptr;
use engine_core::x87::{x87_add, x87_load32, x87_m32, x87_mul, x87_quiet_bits, x87_store32, x87_sub};
use engine_core::{args, typed_call, Engine};

/// `-FLT_MAX` as the float an empty group's distance is (`01015f5c`), and as the double the alpha
/// draw's loop compares with (`010241b0`).
const NO_DISTANCE: u32 = 0x0101_5f5c;
const NO_DISTANCE_DOUBLE: u32 = 0x0102_41b0;
/// The larger of two floats (`00404010`, `cdecl(a, b)`, answering in `ST0`).
const FLOAT_MAX: u32 = 0x0040_4010;
/// A holder's own pass list draw (`00b99990`, `thiscall(list, flag)`), and a batch renderer's slot
/// that draws its pass types (0x90, `(1, 0x2f1, flag)`).
const OWN_LIST_DRAW: u32 = 0x00b9_9990;
const SLOT_DRAW_TYPES: u32 = 0x90;
/// The pass map's `GetAt` (`00853130`, `thiscall(map, key, out)`).
const MAP_GET_AT: u32 = 0x0085_3130;

// Typed from 00b650c0
/// `BSShaderAccumulator::RenderAlphaGeometry` (Xbox PDB), `thiscall(accumulator, group)`: draws alpha
/// group `group` of the one or two batch renderers back to front. Each renderer's group distance
/// ([`alpha_distance`], `-FLT_MAX` for a missing second); while either is above `-FLT_MAX` (the double
/// at `010241b0`), the renderer with the farther one (the second on a tie) draws its next alphas down
/// to the other's distance (`RenderNextAlpha`) and its distance is taken again. Then each renderer's
/// group is cleared ([`clear_alpha_group`]) and the pass ends.
///
/// The radius byte goes out as a word whose upper bytes are a stack slot's: an uninitialised local
/// for the first distance (0 here, as the translation) and the `group` argument's for the others, as
/// the exe pushes them.
pub fn draw_alpha_group(e: &mut Engine, acc: Ptr<BSShaderAccumulator>, group: u32) {
    own::end_pass(e);
    let word = |e: &Engine, upper: u32| upper | u32::from(packet::subtract_radius(e, acc));
    let flag = word(e, 0);
    let camera = packet::camera(e, acc);
    let first = packet::renderer(e, acc, 0);
    let mut distances = [0u32; 2];
    distances[0] = x87_store32(typed_call!(
        FAMILY,
        0x00b9_8c80,
        alpha_distance(e, first, group, camera, flag)
    ));
    let second = if packet::renderer_count(e, acc) == 2 {
        let flag = word(e, group & 0xffff_ff00);
        let camera = packet::camera(e, acc);
        let renderer = packet::renderer(e, acc, 1);
        typed_call!(
            FAMILY,
            0x00b9_8c80,
            alpha_distance(e, renderer, group, camera, flag)
        )
    } else {
        x87_load32(e.global::<u32>(NO_DISTANCE))
    };
    distances[1] = x87_store32(second);
    loop {
        let floor = e.global::<f64>(NO_DISTANCE_DOUBLE);
        let a = x87_load32(distances[0]);
        let b = x87_load32(distances[1]);
        if !(a > floor) && !(b > floor) {
            break;
        }
        // `FCOMPP`: the second renderer when the first is not farther.
        let which = usize::from(!(a > b));
        let limit = x87_quiet_bits(distances[1 - which]);
        let flag = word(e, 0);
        let camera = packet::camera(e, acc);
        let renderer = packet::renderer(e, acc, which as u32);
        typed_call!(
            FAMILY,
            0x00b9_af60,
            render_next_alpha(e, renderer, group, camera, flag, f32::from_bits(limit))
        );
        let flag = word(e, group & 0xffff_ff00);
        let camera = packet::camera(e, acc);
        let renderer = packet::renderer(e, acc, which as u32);
        distances[which] = x87_store32(typed_call!(
            FAMILY,
            0x00b9_8c80,
            alpha_distance(e, renderer, group, camera, flag)
        ));
    }
    let first = packet::renderer(e, acc, 0);
    typed_call!(FAMILY, 0x00b6_4f20, clear_alpha_group(e, first, group));
    if packet::renderer_count(e, acc) == 2 {
        let second = packet::renderer(e, acc, 1);
        typed_call!(FAMILY, 0x00b6_4f20, clear_alpha_group(e, second, group));
    }
    own::end_pass(e);
}

/// The camera direction dotted with a bound's centre in the exe's order for this function: y first,
/// then x, then z.
#[inline(always)]
fn centre_distance(e: &Engine, bound: u32, d: [f64; 3], x_by_load: bool) -> f64 {
    let p = |i: u32| x87_load32(e.global::<u32>(bound.wrapping_add(i * 4)));
    let x = if x_by_load {
        x87_mul(p(0), d[0])
    } else {
        x87_mul(d[0], x87_m32(e.global::<u32>(bound)))
    };
    let sum = x87_add(x87_mul(p(1), d[1]), x);
    x87_add(sum, x87_mul(p(2), d[2]))
}

/// The distance stored as a float, with the radius taken off (and stored again) when `subtract`.
#[inline(always)]
fn stored_distance(e: &Engine, bound: u32, d: [f64; 3], subtract: bool) -> u32 {
    let dot = x87_store32(centre_distance(e, bound, d, true));
    if subtract {
        let radius = x87_m32(e.global::<u32>(bound.wrapping_add(0xc)));
        x87_store32(x87_sub(x87_load32(dot), radius))
    } else {
        dot
    }
}

// Typed from 00b98c80
/// `thiscall(renderer, group, camera, subtract)`, answering in `ST0`: the distance of alpha group
/// `group` along the camera's direction. With an empty holder, the first sorted group's distance (+0x20)
/// when the group list has one, else `-FLT_MAX`. Otherwise the holder's first pass's bound: the camera
/// direction dotted with its centre (less the radius when `subtract`); with sorted groups too, the
/// larger of that and the first group's (`00404010`).
pub fn alpha_distance(
    e: &mut Engine,
    renderer: Ptr<BSBatchRenderer>,
    group: u32,
    camera: u32,
    subtract: u32,
) -> f64 {
    let subtract = shader::low8(subtract) != 0;
    let d = packet::camera_direction(e, camera);
    let holder = packet::alpha_holder(e, renderer, group);
    let count = packet::holder_count(e, holder);
    let list = packet::alpha_list(renderer, group);
    let listed = e.global::<u32>(list.wrapping_add(8));
    let first_group_distance = |e: &Engine| {
        let head = e.global::<u32>(list);
        let item = e.global::<u32>(head.wrapping_add(8));
        e.global::<u32>(item.wrapping_add(0x20))
    };
    if count == 0 {
        if listed == 0 {
            return x87_load32(e.global::<u32>(NO_DISTANCE));
        }
        return x87_load32(first_group_distance(e));
    }
    let first_bound = |e: &Engine| {
        let node = e.global::<u32>(holder.wrapping_add(0x10));
        let pass = e.global::<u32>(node.wrapping_add(8));
        packet::bound_of(e, e.global::<u32>(pass))
    };
    if listed == 0 {
        let bound = first_bound(e);
        return x87_load32(stored_distance(e, bound, d, subtract));
    }
    let list_distance = x87_quiet_bits(first_group_distance(e));
    let bound = first_bound(e);
    let distance = if subtract {
        stored_distance(e, bound, d, true)
    } else {
        x87_store32(centre_distance(e, bound, d, false))
    };
    e.call(FLOAT_MAX, &args![list_distance, distance]).st0
}

// Typed from 00b64f20
/// `thiscall(renderer, group)`: after alpha group `group` is drawn: its holder emptied, each sorted
/// group of its list destroyed and freed, the list emptied and the current grouped holder cleared.
pub fn clear_alpha_group(e: &mut Engine, renderer: Ptr<BSBatchRenderer>, group: u32) {
    let holder = packet::alpha_holder(e, renderer, group);
    own::reset_holder(e, holder);
    let mut node = e.global::<u32>(packet::alpha_list(renderer, group));
    while node != 0 {
        let item = e.global::<u32>(node.wrapping_add(8));
        node = e.global::<u32>(node);
        if item != 0 {
            own::free_alpha_group(e, item);
        }
    }
    own::empty_alpha_list(e, renderer, group);
}

// Typed from 00b9a3c0
/// `thiscall(renderer, type, list, flag, unused)`: the shadow pass types' draw: the group of the type
/// at `pass_type` (the pass map's value, the out word cleared first) is emptied, then the next list is
/// made ready as [`batch::next_group`] does, and its answer passed on.
pub fn draw_and_clear_type(
    e: &mut Engine,
    renderer: Ptr<BSBatchRenderer>,
    pass_type: u32,
    element: u32,
    flag: u32,
    _unused: u32,
) -> bool {
    let key = e.global::<u32>(pass_type);
    let index = e.with_stack(4, |e, slot| {
        e.set_global(slot.addr(), 0u32);
        e.call(
            MAP_GET_AT,
            &args![renderer.addr().wrapping_add(0xc), key, slot],
        );
        e.global::<u32>(slot.addr())
    });
    let groups = e.get(renderer, BSBatchRenderer::pRenderPassA);
    let group = Ptr::<PassGroup>::new(index.wrapping_mul(0x68).wrapping_add(groups));
    own::clear_pass_group(e, group);
    typed_call!(
        shader::FAMILY,
        0x00b9_a090,
        batch::next_group(e, renderer, pass_type, element, flag)
    )
}

// Typed from 00b9a4d0
/// `thiscall(holder, flag)`: draws a register group holder: its own pass list (`00b99990`) when it
/// keeps one, else its batch renderer's pass types (slot 0x90 with 1, 0x2f1, `flag`); then the
/// renderer's active list cleared (with a renderer) and the holder's count.
pub fn draw_holder(e: &mut Engine, holder: u32, flag: u32) {
    if packet::keeps_own(e, holder) {
        e.call(OWN_LIST_DRAW, &args![holder.wrapping_add(0xc), flag]);
    } else {
        let renderer = packet::holder_renderer(e, holder);
        if renderer != 0 {
            e.vcall(renderer, SLOT_DRAW_TYPES, &args![1u32, 0x2f1u32, flag]);
        }
    }
    let renderer = packet::holder_renderer(e, holder);
    if renderer != 0 {
        own::clear_active_list(e, renderer);
    }
    own::clear_holder_count(e, holder);
}

/// The double `RenderNextAlpha` takes off its limit (`01016978`, 0.001).
const LIMIT_STEP: u32 = 0x0101_6978;
/// `_isnan` (CRT, `cdecl(double)`).
const ISNAN: u32 = 0x00ec_75b1;

/// The distance of `pass` along `d` as `RenderNextAlpha` first takes it (the direction kept on the x87
/// stack: each product the direction first, x then y, z last).
#[inline(always)]
fn next_alpha_first(e: &Engine, pass: u32, d: [f64; 3], subtract: bool) -> u32 {
    let bound = packet::bound_of(e, e.global::<u32>(pass));
    let p = |i: u32| x87_load32(e.global::<u32>(bound.wrapping_add(i * 4)));
    let y = x87_mul(d[1], p(1));
    let x = x87_mul(d[0], x87_m32(e.global::<u32>(bound)));
    let dot = x87_add(x87_mul(d[2], p(2)), x87_add(y, x));
    radius_off(e, bound, dot, subtract)
}

/// The same for the following passes (the direction read from its locals).
#[inline(always)]
fn next_alpha_next(e: &Engine, pass: u32, d: [u32; 3], subtract: bool) -> u32 {
    let bound = packet::bound_of(e, e.global::<u32>(pass));
    let p = |i: u32| x87_load32(e.global::<u32>(bound.wrapping_add(i * 4)));
    let sum = x87_add(x87_mul(p(1), x87_m32(d[1])), x87_mul(p(0), x87_m32(d[0])));
    let dot = x87_add(sum, x87_mul(p(2), x87_m32(d[2])));
    radius_off(e, bound, dot, subtract)
}

/// A distance stored as a float, the radius taken off (and stored again) when `subtract`.
#[inline(always)]
fn radius_off(e: &Engine, bound: u32, dot: f64, subtract: bool) -> u32 {
    if subtract {
        let dot = x87_store32(dot);
        x87_store32(x87_sub(
            x87_load32(dot),
            x87_m32(e.global::<u32>(bound.wrapping_add(0xc))),
        ))
    } else {
        x87_store32(dot)
    }
}

/// Whether `limit` is above `value` or either is NaN (the exe's `FCOMP` with `JP`/`JA` taken).
#[inline(always)]
fn beyond(limit: f64, value: f64) -> bool {
    limit > value || limit.is_nan() || value.is_nan()
}

// Typed from 00b9af60
/// `BSBatchRenderer::RenderNextAlpha` (Xbox PDB), `thiscall(renderer, group, camera, subtract,
/// limit)`: draws, back to front, alpha group `group`'s passes (its holder's list) and its sorted
/// groups (the group list) down to `limit` less 0.001 (the double at `01016978`). The current pass's
/// distance is the camera direction dotted with its bound's centre (less the radius when `subtract`).
/// While a sorted group is not nearer than the current pass (or there is no pass) and within the limit,
/// it is drawn (its own passes through `00b99990` with 1 when it keeps them, else its batch renderer's
/// slot 0x90 with 1, 0x2f1, 1), its renderer's active list cleared, the group destroyed and freed and
/// its node given back to the pool. While the pass is within the limit it is drawn
/// ([`draw_alpha_pass`]) and taken off the holder's list. The loop ends when neither is left within
/// the limit, or the current distance is NaN (`_isnan`).
pub fn render_next_alpha(
    e: &mut Engine,
    renderer: Ptr<BSBatchRenderer>,
    group: u32,
    camera: u32,
    subtract: u32,
    limit: f32,
) {
    let locals = packet::camera_direction_words(e, camera);
    let d = locals.map(x87_load32);
    let subtract = shader::low8(subtract) != 0;
    let head = e.global::<u32>(packet::alpha_holder(e, renderer, group).wrapping_add(0x10));
    let mut pass = if head != 0 {
        e.global::<u32>(head.wrapping_add(8))
    } else {
        0
    };
    let mut current = x87_store32(0.0);
    if pass != 0 {
        current = next_alpha_first(e, pass, d, subtract);
    }
    let list = packet::alpha_list(renderer, group);
    let first_sorted = |e: &Engine| {
        if e.global::<u32>(list.wrapping_add(8)) != 0 {
            e.global::<u32>(e.global::<u32>(list).wrapping_add(8))
        } else {
            0
        }
    };
    let mut sorted = first_sorted(e);
    let limit = x87_store32(x87_sub(
        x87_load32(limit.to_bits()),
        e.global::<f64>(LIMIT_STEP),
    ));
    let limit_value = x87_load32(limit);
    loop {
        // The sorted groups.
        let mut pass_turn = false;
        while sorted != 0 {
            let distance = x87_load32(e.global::<u32>(sorted.wrapping_add(0x20)));
            if !(x87_load32(current) < distance) && pass != 0 {
                pass_turn = true;
                break;
            }
            if beyond(limit_value, distance) {
                break;
            }
            let own_renderer = packet::holder_renderer(e, sorted);
            if packet::keeps_own(e, sorted) {
                e.call(OWN_LIST_DRAW, &args![sorted.wrapping_add(0xc), 1u32]);
            } else if own_renderer != 0 {
                e.vcall(own_renderer, SLOT_DRAW_TYPES, &args![1u32, 0x2f1u32, 1u32]);
            }
            let own_renderer = packet::holder_renderer(e, sorted);
            if own_renderer != 0 {
                own::clear_active_list(e, own_renderer);
            }
            own::clear_holder_count(e, sorted);
            own::free_alpha_group(e, sorted);
            own::take_sorted_node(e, list);
            sorted = first_sorted(e);
        }
        // The pass.
        let mut check_pass = pass_turn || pass != 0;
        if check_pass {
            if !beyond(limit_value, x87_load32(current)) {
                draw_alpha_pass(e, pass);
                let holder = packet::alpha_holder(e, renderer, group);
                own::take_first_pass(e, holder);
                let holder = packet::alpha_holder(e, renderer, group);
                let head = e.global::<u32>(holder.wrapping_add(0x10));
                if head == 0 {
                    pass = 0;
                    check_pass = false;
                } else {
                    pass = e.global::<u32>(head.wrapping_add(8));
                    if pass == 0 {
                        check_pass = false;
                    } else {
                        current = next_alpha_next(e, pass, locals, subtract);
                    }
                }
            }
            if check_pass && limit_value <= x87_load32(current) {
                if e.call(ISNAN, &args![x87_load32(current)]).u32() != 0 {
                    return;
                }
                continue;
            }
        }
        if sorted == 0 {
            return;
        }
        let distance = x87_load32(e.global::<u32>(sorted.wrapping_add(0x20)));
        if beyond(limit_value, distance) {
            return;
        }
        if e.call(ISNAN, &args![x87_load32(current)]).u32() != 0 {
            return;
        }
    }
}

/// The render mode word whose value 9 skips the property kinds 0x15 and 0x11 (`011f91b8`).
const SHADER_MODE: u32 = 0x011f_91b8;

/// One pass of [`render_next_alpha`]: skipped without a shader property, or in shader mode 9 with
/// a property of kind 0x15 or 0x11; drawn (`RenderPassImmediately(pass, type, 1, 1, 1)`, the shader
/// family's), with z-writes off and the decal depth bias around it when the property's flags have bit
/// 26 or 27.
fn draw_alpha_pass(e: &mut Engine, pass: u32) {
    let geometry = e.global::<u32>(pass);
    let property = e.global::<u32>(geometry.wrapping_add(0xa8));
    if property == 0 {
        return;
    }
    if e.global::<u32>(SHADER_MODE) == 9 {
        let kind = e.global::<u32>(property.wrapping_add(0x1c));
        if kind == 0x15 || kind == 0x11 {
            return;
        }
    }
    let flags = e.global::<u32>(property.wrapping_add(0x20));
    let biased = flags & 0x0c00_0000 != 0;
    if biased {
        own::decal_state_on(e);
    }
    let pass_type = u32::from(e.global::<u16>(pass.wrapping_add(4)));
    typed_call!(
        shader::FAMILY,
        0x00b9_94f0,
        batch::render_pass(e, Ptr::new(pass), pass_type, 1u32, 1u32, 1u32)
    );
    if biased {
        own::decal_state_off(e);
    }
}
