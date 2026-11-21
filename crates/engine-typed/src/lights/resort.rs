//! `BSShaderLightingProperty::ResortLights` (`00b70390`): a lit geometry's light list put in order of
//! each light's luminance to the geometry's bound, as the render passes ask before they are built.
//!
//! The exe asks `ShadowSceneLight::GetLuminanceToBound` (`00b9dbe0`) again for every earlier node of
//! the list at every node it places, so a list of `n` lights costs about `n * n / 2` luminances, each
//! a square root. The luminance depends only on the light, its `NiLight`'s position and radius, the
//! scene node's lighting offset and the bound, none of which the sort changes, so here each node's
//! luminance is computed once, when the walk reaches it (the nodes before it are exactly those it
//! reached already, and their lights' `fLuminance` holds it), and the scans compare those stored
//! values: the same comparisons in the same order, the same moves, and each light's `fLuminance`
//! left with the same value. A list of `n` lights now costs `n` square roots instead of about
//! `n * n / 2`.
//!
//! **Floats.** The render passes that ask for the sort run on the main thread and now and then on
//! the loading thread, so the luminance keeps the exe's x87 steps with the exact helpers (each sum,
//! difference and quotient stored as the exe stores it, right under either control word; the Diff
//! test runs both); its square root of a stored float, stored again as a float, is `f32::sqrt` (a
//! correctly rounded root, as `FSQRT` then the float store gives under either control word).

use engine_core::ptr::Ptr;
use engine_core::x87::{
    x87_add, x87_div, x87_load32, x87_m32, x87_mul, x87_quiet_bits, x87_store32, x87_sub,
};
use engine_core::Engine;

/// The property's light list (`lLightList`, +0x60: head, then tail at +0x64) and
/// `iLastRenderPassState` (+0x38).
const LIGHT_LIST: u32 = 0x60;
const LAST_RENDER_PASS_STATE: u32 = 0x38;
/// The scene node pointer (`011f91c8`) and its lighting offset (+0x1e4).
const SCENE_NODE: u32 = 0x011f_91c8;
const LIGHTING_OFFSET: u32 = 0x1e4;
/// A `ShadowSceneLight`'s `fLuminance` (+0xc) and `spLight` (+0xf8); an `NiLight`'s world
/// translation (+0x8c) and radius (+0xe0).
const LIGHT_LUMINANCE: u32 = 0xc;
const LIGHT_SP_LIGHT: u32 = 0xf8;
const WORLD_TRANSLATE: u32 = 0x8c;
const LIGHT_RADIUS: u32 = 0xe0;

/// `(y * y + x * x) + z * z` with the exact helpers, in the exe's order.
fn squared_length(x: f64, y: f64, z: f64) -> f64 {
    x87_add(x87_add(x87_mul(y, y), x87_mul(x, x)), x87_mul(z, z))
}

/// `00b9dbe0` in place: `(distance - bound radius) / NiLight radius` as a float (a signalling NaN
/// quieted), the distance from the `NiLight`'s position moved by the lighting offset to the bound's
/// centre; stored in the light's `fLuminance` and answered as bits.
fn luminance(e: &mut Engine, light: u32, bound: u32) -> u32 {
    let node = e.global::<u32>(SCENE_NODE);
    let offset = [0u32, 4, 8].map(|i| e.mem.u32(node.wrapping_add(LIGHTING_OFFSET + i)));
    let ni = e.mem.u32(light.wrapping_add(LIGHT_SP_LIGHT));
    let at = |i: u32| ni.wrapping_add(WORLD_TRANSLATE + i);
    let px = x87_store32(x87_add(x87_load32(offset[0]), x87_m32(e.mem.u32(at(0)))));
    let py = x87_store32(x87_add(x87_load32(e.mem.u32(at(4))), x87_m32(offset[1])));
    let pz = x87_store32(x87_add(x87_load32(e.mem.u32(at(8))), x87_m32(offset[2])));
    let dx = x87_store32(x87_sub(x87_load32(px), x87_m32(e.mem.u32(bound))));
    let dy = x87_store32(x87_sub(
        x87_load32(py),
        x87_m32(e.mem.u32(bound.wrapping_add(4))),
    ));
    let dz = x87_store32(x87_sub(
        x87_load32(pz),
        x87_m32(e.mem.u32(bound.wrapping_add(8))),
    ));
    let squared = x87_store32(squared_length(
        x87_load32(dx),
        x87_load32(dy),
        x87_load32(dz),
    ));
    let distance = f32::from_bits(squared).sqrt().to_bits();
    let value = x87_div(
        x87_sub(
            x87_load32(distance),
            x87_m32(e.mem.u32(bound.wrapping_add(0xc))),
        ),
        x87_m32(e.mem.u32(ni.wrapping_add(LIGHT_RADIUS))),
    );
    let value = x87_quiet_bits(x87_store32(value));
    e.mem.set_u32(light.wrapping_add(LIGHT_LUMINANCE), value);
    value
}

/// The list move the exe inlines: `node` goes just before `before`.
fn move_before(e: &mut Engine, list: u32, node: u32, before: u32) {
    if e.mem.u32(list) == node {
        let next = e.mem.u32(node);
        e.mem.set_u32(list, next);
    }
    if e.mem.u32(list) == before {
        e.mem.set_u32(list, node);
    }
    if e.mem.u32(list.wrapping_add(4)) == node {
        let previous = e.mem.u32(node.wrapping_add(4));
        e.mem.set_u32(list.wrapping_add(4), previous);
    }
    let next = e.mem.u32(node);
    if next != 0 {
        let previous = e.mem.u32(node.wrapping_add(4));
        e.mem.set_u32(next.wrapping_add(4), previous);
    }
    let previous = e.mem.u32(node.wrapping_add(4));
    if previous != 0 {
        let next = e.mem.u32(node);
        e.mem.set_u32(previous, next);
    }
    let previous = e.mem.u32(before.wrapping_add(4));
    e.mem.set_u32(node.wrapping_add(4), previous);
    e.mem.set_u32(node, before);
    if previous != 0 {
        e.mem.set_u32(previous, node);
    }
    e.mem.set_u32(before.wrapping_add(4), node);
}

// Typed from 00b70390
/// `thiscall(property, bound)`: for each node of the light list (its next taken first), its light's
/// luminance is compared with each node's from the head up to it; the first with a greater one (an
/// ordered `>`) gets the node moved before it and ends that scan. When anything moved,
/// `iLastRenderPassState` is cleared.
///
/// The nodes a scan passes are the ones the walk reached before, so their lights' `fLuminance`
/// already holds this call's value (the exe writes the same value again at each comparison): it is
/// read back instead of being worked out again.
pub fn resort_lights(e: &mut Engine, this: Ptr, bound: u32) {
    let list = this.addr().wrapping_add(LIGHT_LIST);
    let mut node = e.mem.u32(list);
    if node == 0 {
        return;
    }
    let mut changed = false;
    loop {
        let mut other = e.mem.u32(list);
        let next = e.mem.u32(node);
        let light = e.mem.u32(node.wrapping_add(8));
        let reference = f32::from_bits(luminance(e, light, bound));
        while other != node && other != 0 {
            let candidate = other;
            other = e.mem.u32(other);
            let candidate_light = e.mem.u32(candidate.wrapping_add(8));
            let value = e.mem.f32(candidate_light.wrapping_add(LIGHT_LUMINANCE));
            if value > reference {
                move_before(e, list, node, candidate);
                other = 0;
                changed = true;
            }
        }
        if next == 0 {
            break;
        }
        node = next;
    }
    if changed {
        e.mem
            .set_u32(this.addr().wrapping_add(LAST_RENDER_PASS_STATE), 0);
    }
}
