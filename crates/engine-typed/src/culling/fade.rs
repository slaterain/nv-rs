//! `BSFadeNode::OnVisible` (`00b4e490`): a fade node met by the culling walk decides how far it has
//! faded in or out (by the camera's distance, its type's multiplier, the frame time) and walks its
//! children only while its fade and its most alpha are not 0. About 700 of them in a street frame,
//! on the main thread and the parallel walk's helpers.
//!
//! What stays a call: the alpha set down the subtree (`RecursiveSetPropertyFadeAlpha`, `00b6bb30`),
//! the accumulator's kind test and its alpha grouping (a parallel walk logs those for main), and the
//! most-alpha callback. The float getters it calls (`0040ebd0`, `00404010`) are done in place, and the
//! square root of a stored float, stored again, is `f32::sqrt`.
//!
//! **Floats.** Both threads' control words: every other operation keeps the exe's x87 steps through
//! the exact helpers, each stored where the exe stores it.

use super::FAMILY;
use engine_core::ptr::Ptr;
use engine_core::x87::{
    x87_add, x87_div, x87_load32, x87_m32, x87_mul, x87_quiet_bits, x87_store32, x87_sub,
};
use engine_core::{args, typed_call, Engine};

/// `BSFadeNode` fields (PC offsets): the world bound (+0x20), the flags (+0x30), the world
/// translation (+0x8c), the near and far distances (+0xac, +0xb0), `fLastFade` (+0xb4),
/// `fCurrentFade` (+0xb8), `fBoundRadius` (+0xbc), `eMultType` (+0xc4), `iFrameCounter` (+0xc8).
const WORLD_BOUND: u32 = 0x20;
const FLAGS: u32 = 0x30;
const WORLD_TRANSLATE: u32 = 0x8c;
const NEAR: u32 = 0xac;
const FAR: u32 = 0xb0;
const LAST_FADE: u32 = 0xb4;
const CURRENT_FADE: u32 = 0xb8;
const BOUND_RADIUS: u32 = 0xbc;
const MULT_TYPE: u32 = 0xc4;
const FRAME_COUNTER: u32 = 0xc8;
/// The camera's `m_fLODAdjust` (+0x110) and the process's camera (+0xc).
const CAMERA_LOD_ADJUST: u32 = 0x110;
const PROCESS_CAMERA: u32 = 0xc;
/// The statics: fading on, the multipliers, the times, thresholds, distance multiplier and most
/// increment, the frame time, the fade counter, the most-alpha callback.
const FADE_ENABLED: u32 = 0x011a_d7b4;
const MULTIPLIERS: u32 = 0x011a_d7b8;
const FADE_IN_TIME: u32 = 0x011a_d7e4;
const FADE_OUT_TIME: u32 = 0x011a_d7e8;
const FADE_IN_THRESHOLD: u32 = 0x011a_d7ec;
const FADE_OUT_THRESHOLD: u32 = 0x011a_d7f0;
const DISTANCE_MULTIPLIER: u32 = 0x011a_d7f4;
const MAX_FADE_INCREMENT: u32 = 0x011a_d7f8;
const FRAME_TIME: u32 = 0x011f_63a0;
const FADE_COUNTER: u32 = 0x011f_9138;
const MAX_ALPHA_ACCESS: u32 = 0x011f_9134;
/// 0.2 (the scale for a type whose multiplier is not positive) and 600.0 (the snap radius).
const UNSCALED_DISTANCE: u32 = 0x0101_df44;
const SNAP_RADIUS: u32 = 0x0101_786c;
const ONE: u32 = 0x3f80_0000;
/// The scene global (its +8 the accumulator), the accumulator's kind test and its argument, the
/// default bound, the alpha grouping, the subtree's fade alpha.
const SCENE_GLOBAL: u32 = 0x011f_4748;
const ACCUMULATOR_CHECK: u32 = 0x0043_b300;
const ACCUMULATOR_CHECK_ARGUMENT: u32 = 0x011f_a000;
const DEFAULT_BOUND: u32 = 0x011f_4288;
const START_GROUPING_ALPHAS: u32 = 0x00b6_3500;
const STOP_GROUPING_ALPHAS: u32 = 0x00b6_3090;
const SET_FADE_ALPHA: u32 = 0x00b6_bb30;

fn get(e: &Engine, node: u32, field: u32) -> u32 {
    e.mem.u32(node.wrapping_add(field))
}

fn put(e: &mut Engine, node: u32, field: u32, value: u32) {
    e.mem.set_u32(node.wrapping_add(field), value);
}

/// `0040ebd0`: the smaller of two floats (`b` when `b <= a`, so `a` on an unordered compare),
/// quieted as its `FLD` does.
fn float_min(a: u32, b: u32) -> f64 {
    let (a, b) = (x87_load32(a), x87_load32(b));
    if b <= a {
        b
    } else {
        a
    }
}

/// `00404010`: the larger of two floats (`a` when `b < a`, else `b`).
fn float_max(a: u32, b: u32) -> f64 {
    let (a, b) = (x87_load32(a), x87_load32(b));
    if b < a {
        a
    } else {
        b
    }
}

fn set_fade_alpha(e: &mut Engine, node: u32, bits: u32) {
    e.call(SET_FADE_ALPHA, &args![node, bits]);
}

fn set_fades(e: &mut Engine, node: u32, bits: u32) {
    put(e, node, CURRENT_FADE, bits);
    put(e, node, LAST_FADE, bits);
}

/// The fade-in step: the fade grows by the frame time over the fade-in time (at most the most
/// increment), up to 1; the alpha goes down the subtree, `fLastFade` follows, and flag 0x4000 is
/// set once the fade reaches 1.
fn fade_in_step(e: &mut Engine, node: u32) {
    let step = x87_store32(x87_div(
        x87_load32(e.global::<u32>(FRAME_TIME)),
        x87_m32(e.global::<u32>(FADE_IN_TIME)),
    ));
    let increment = x87_quiet_bits(e.global::<u32>(MAX_FADE_INCREMENT));
    let first = float_min(step, increment);
    let sum = x87_store32(x87_add(first, x87_m32(get(e, node, CURRENT_FADE))));
    let value = x87_quiet_bits(x87_store32(float_min(ONE, sum)));
    put(e, node, CURRENT_FADE, value);
    set_fade_alpha(e, node, value);
    let current = x87_quiet_bits(get(e, node, CURRENT_FADE));
    put(e, node, LAST_FADE, current);
    if 1.0 <= x87_m32(get(e, node, CURRENT_FADE)) {
        let flags = get(e, node, FLAGS);
        put(e, node, FLAGS, flags | 0x4000);
    }
}

/// The fade-out step: the fade shrinks by the frame time over the fade-out time (at most the most
/// increment), down to 0; the alpha goes down the subtree, `fLastFade` follows, and flag 0x4000 is
/// cleared once the fade is 0.
fn fade_out_step(e: &mut Engine, node: u32) {
    let current = x87_load32(get(e, node, CURRENT_FADE));
    let step = x87_store32(x87_div(
        x87_load32(e.global::<u32>(FRAME_TIME)),
        x87_m32(e.global::<u32>(FADE_OUT_TIME)),
    ));
    let increment = x87_quiet_bits(e.global::<u32>(MAX_FADE_INCREMENT));
    let decrement = float_min(step, increment);
    let lowered = x87_store32(x87_sub(current, decrement));
    let value = x87_quiet_bits(x87_store32(float_max(0, lowered)));
    put(e, node, CURRENT_FADE, value);
    set_fade_alpha(e, node, value);
    let current = x87_quiet_bits(get(e, node, CURRENT_FADE));
    put(e, node, LAST_FADE, current);
    if x87_m32(get(e, node, CURRENT_FADE)) <= 0.0 {
        let flags = get(e, node, FLAGS);
        put(e, node, FLAGS, flags & 0xffff_bfff);
    }
}

/// `(first² + second²) + third²` in the exe's operand order.
fn sum_of_squares(first: f64, second: f64, third: f64) -> f64 {
    let first_square = x87_mul(first, first);
    let second_square = x87_mul(second, second);
    let sum = x87_add(second_square, first_square);
    x87_add(sum, x87_mul(third, third))
}

/// The node's most alpha (`00b4df80`): the callback's answer rounded to a float, else 1.0.
fn most_alpha(e: &mut Engine, node: u32) -> u32 {
    let access = e.global::<u32>(MAX_ALPHA_ACCESS);
    if access != 0 {
        let value = e.call(access, &args![node]).st0;
        x87_quiet_bits(x87_store32(value))
    } else {
        ONE
    }
}

fn node_on_visible(e: &mut Engine, node: u32, process: u32) {
    typed_call!(
        FAMILY,
        0x00a5_dbe0,
        super::visible::node_on_visible(e, Ptr::new(node), Ptr::new(process))
    );
}

// Typed from 00b4e490
/// `BSFadeNode::OnVisible` (`thiscall(node, process)`), as the translation describes it: with fading
/// off or flag 0x8000 simply visible; type 6 only fades in; otherwise the fade target from the
/// camera distance (scaled by the camera's LOD adjust over the type's multiplier, or 0.2 with the node
/// marked faded in when the multiplier is not positive), 1 up to the near distance and falling to the
/// far one (type 10: flag 0x10000000); a small node unseen for 20 fade frames snaps; the counter is
/// stored; the most alpha groups the node's alphas or the fade steps in or out; visible while the
/// fade and the most alpha are not 0.
pub fn on_visible(e: &mut Engine, this: Ptr, process: Ptr) {
    let (t, p) = (this.addr(), process.addr());
    let decision = decide(e, t, p);
    super::compact::note_fade(t, decision.visible);
    if decision.visible {
        node_on_visible(e, t, p);
    }
    finish(e, decision);
}

/// What [`decide`] found: whether the children are walked, and whether [`finish`] stops an alpha
/// grouping it started.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Decision {
    pub(crate) visible: bool,
    pub(crate) grouping: bool,
}

/// After the children: the alpha grouping [`decide`] started is stopped.
pub(crate) fn finish(e: &mut Engine, decision: Decision) {
    if decision.grouping {
        // The accumulator read again, as the exe reads it after the children.
        let accumulator = e.mem.u32(e.global::<u32>(SCENE_GLOBAL).wrapping_add(8));
        e.call(STOP_GROUPING_ALPHAS, &args![accumulator]);
    }
}

/// Everything `OnVisible` does before it walks the children (every write and call of it, in order):
/// whether it walks them, and the grouping it started.
pub(crate) fn decide(e: &mut Engine, t: u32, p: u32) -> Decision {
    let plain = Decision {
        visible: true,
        grouping: false,
    };
    let flags = get(e, t, FLAGS);
    if e.global::<u8>(FADE_ENABLED) == 0 || (flags >> 15) & 1 != 0 {
        return plain;
    }
    let mult_type = get(e, t, MULT_TYPE);
    if mult_type == 6 {
        if (flags >> 14) & 1 == 0 {
            fade_in_step(e, t);
        }
        return plain;
    }
    let multiplier = x87_quiet_bits(
        e.mem
            .u32(MULTIPLIERS.wrapping_add(mult_type.wrapping_mul(4))),
    );
    let camera = e.mem.u32(p.wrapping_add(PROCESS_CAMERA));
    let scale = if x87_load32(multiplier) > 0.0 {
        let adjust = x87_m32(e.mem.u32(camera.wrapping_add(CAMERA_LOD_ADJUST)));
        x87_store32(x87_div(adjust, x87_load32(multiplier)))
    } else {
        put(e, t, FLAGS, flags | 0x4000);
        set_fades(e, t, ONE);
        let current = x87_quiet_bits(get(e, t, CURRENT_FADE));
        set_fade_alpha(e, t, current);
        x87_quiet_bits(e.global::<u32>(UNSCALED_DISTANCE))
    };
    let delta = [0u32, 4, 8].map(|i| {
        let own = x87_load32(e.mem.u32(t.wrapping_add(WORLD_TRANSLATE + i)));
        let other = x87_m32(e.mem.u32(camera.wrapping_add(WORLD_TRANSLATE + i)));
        x87_store32(x87_sub(own, other))
    });
    let squared = x87_store32(sum_of_squares(
        x87_load32(delta[0]),
        x87_load32(delta[1]),
        x87_load32(delta[2]),
    ));
    let root = f32::from_bits(squared).sqrt().to_bits();
    let distance = x87_store32(x87_mul(x87_load32(root), x87_m32(scale)));
    let mut fade = ONE;
    let factor = x87_load32(e.global::<u32>(DISTANCE_MULTIPLIER));
    let near = x87_store32(x87_mul(x87_load32(get(e, t, NEAR)), factor));
    let near_value = x87_load32(near);
    let distance_value = x87_load32(distance);
    if near_value < distance_value {
        let beyond = x87_sub(distance_value, near_value);
        let far = x87_store32(x87_mul(factor, x87_load32(get(e, t, FAR))));
        let range = x87_sub(x87_load32(far), near_value);
        fade = x87_store32(x87_sub(1.0, x87_div(beyond, range)));
    }
    let mult_type = get(e, t, MULT_TYPE);
    if mult_type == 10 {
        fade = if (get(e, t, FLAGS) >> 28) & 1 != 0 {
            ONE
        } else {
            x87_store32(0.0)
        };
    }
    let fade_value = x87_load32(fade);
    let counter = i32::from_ne_bytes(e.global::<u32>(FADE_COUNTER).to_ne_bytes());
    let last_seen = i32::from_ne_bytes(get(e, t, FRAME_COUNTER).to_ne_bytes());
    if mult_type != 8
        && last_seen < counter.wrapping_sub(0x14)
        && x87_load32(e.global::<u32>(SNAP_RADIUS)) > x87_m32(get(e, t, BOUND_RADIUS))
    {
        let fade_in = x87_load32(e.global::<u32>(FADE_IN_THRESHOLD));
        let snap_in = fade_in < fade_value
            || ((get(e, t, FLAGS) >> 14) & 1 != 0
                && x87_load32(e.global::<u32>(FADE_OUT_THRESHOLD)) < fade_value);
        if snap_in {
            set_fade_alpha(e, t, ONE);
            put(e, t, CURRENT_FADE, ONE);
            let flags = get(e, t, FLAGS);
            put(e, t, FLAGS, flags | 0x4000);
            put(e, t, LAST_FADE, ONE);
        } else {
            set_fade_alpha(e, t, 0);
            put(e, t, CURRENT_FADE, 0);
            let flags = get(e, t, FLAGS);
            put(e, t, FLAGS, flags & 0xffff_bfff);
            put(e, t, LAST_FADE, 0);
        }
    }
    let counter = e.global::<u32>(FADE_COUNTER);
    put(e, t, FRAME_COUNTER, counter);
    let most = most_alpha(e, t);
    let most_value = x87_load32(most);
    let faded_in = (get(e, t, FLAGS) >> 14) & 1 != 0;
    let mut grouping = false;
    if faded_in && 0.0 < most_value && 1.0 > most_value {
        let accumulator = e.mem.u32(e.global::<u32>(SCENE_GLOBAL).wrapping_add(8));
        if e.call(
            ACCUMULATOR_CHECK,
            &args![ACCUMULATOR_CHECK_ARGUMENT, accumulator],
        )
        .u8()
            != 0
        {
            let bound = match get(e, t, WORLD_BOUND) {
                0 => DEFAULT_BOUND,
                bound => bound,
            };
            e.call(START_GROUPING_ALPHAS, &args![accumulator, bound, 1u32]);
            grouping = true;
            set_fade_alpha(e, t, most);
            set_fades(e, t, most);
        }
    } else {
        let fade_in = x87_load32(e.global::<u32>(FADE_IN_THRESHOLD));
        let current = x87_m32(get(e, t, CURRENT_FADE));
        let step_in = (!faded_in && fade_value > fade_in)
            || (0.0 < current && 1.0 > current && fade_in < fade_value);
        if step_in {
            fade_in_step(e, t);
        } else {
            let fade_out = x87_load32(e.global::<u32>(FADE_OUT_THRESHOLD));
            let step_out = (faded_in && fade_out > fade_value) || {
                let current = x87_m32(get(e, t, CURRENT_FADE));
                1.0 > current && 0.0 < current && fade_value < fade_out
            };
            if step_out {
                fade_out_step(e, t);
            }
        }
    }
    Decision {
        visible: 0.0 < x87_m32(get(e, t, CURRENT_FADE)) && x87_m32(most) != 0.0,
        grouping,
    }
}
