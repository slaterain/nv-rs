//! `BSShaderLightingProperty::ResortLights` (`00b70390`): a lit geometry's light list put in order of
//! each light's luminance to the geometry's bound, as the render passes ask before they are built.
//!
//! The exe asks `ShadowSceneLight::GetLuminanceToBound` (`00b9dbe0`) again for every earlier node of
//! the list at every node it places, so a list of `n` lights costs about `n * n / 2` luminances, each
//! a square root. The luminance depends only on the light, its `NiLight`'s position and radius, the
//! scene node's lighting offset and the bound, none of which the sort changes, so here each node's
//! luminance is computed once, when the walk reaches it, and kept in a local list in the order of
//! the nodes already reached (exactly the nodes a scan passes, in list order, since a placed node
//! only ever moves among them): the same comparisons in the same order, the same moves, and each
//! light's `fLuminance` written with the same value. A list of `n` lights now costs `n` square
//! roots instead of about `n * n / 2`.
//!
//! The values compared are never read back from `fLuminance`: that field belongs to the light,
//! which many objects' lists share, and other code writes it (`00b9bdf0`, and this sort for other
//! objects on the loading thread), so only what this call worked out may decide its order, as in
//! the exe, which compares the values its own calls answered.
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

/// The value of the `at`th reached node `candidate`: kept from when the walk reached it. Should the
/// list ever not match what was kept (nothing in this call changes it but the moves mirrored in
/// `reached`), the exe's own way: its luminance worked out again.
fn kept(e: &mut Engine, reached: &[(u32, f32)], at: usize, candidate: u32, bound: u32) -> f32 {
    match reached.get(at) {
        Some(&(node, value)) if node == candidate => value,
        _ => {
            let light = e.mem.u32(candidate.wrapping_add(8));
            f32::from_bits(luminance(e, light, bound))
        }
    }
}

// Typed from 00b70390
/// `thiscall(property, bound)`: for each node of the light list (its next taken first), its light's
/// luminance is compared with each node's from the head up to it; the first with a greater one (an
/// ordered `>`) gets the node moved before it and ends that scan. When anything moved,
/// `iLastRenderPassState` is cleared.
///
/// The nodes a scan passes are the ones the walk reached before; their values, worked out when the
/// walk reached them, are kept in `reached` in the list's order (a node moved before the `i`th
/// reached node takes place `i` there).
pub fn resort_lights(e: &mut Engine, this: Ptr, bound: u32) {
    resort_with(e, this, bound, &mut |_| {});
}

/// [`resort_lights`] with `between` run after each node's luminance is worked out (the tests stand
/// in another thread's writes there).
fn resort_with(e: &mut Engine, this: Ptr, bound: u32, between: &mut dyn FnMut(&mut Engine)) {
    let list = this.addr().wrapping_add(LIGHT_LIST);
    let mut node = e.mem.u32(list);
    if node == 0 {
        return;
    }
    let mut changed = false;
    let mut reached: Vec<(u32, f32)> = Vec::new();
    loop {
        let mut other = e.mem.u32(list);
        let next = e.mem.u32(node);
        let light = e.mem.u32(node.wrapping_add(8));
        let reference = f32::from_bits(luminance(e, light, bound));
        between(e);
        let mut place = reached.len();
        let mut at = 0usize;
        while other != node && other != 0 {
            let candidate = other;
            other = e.mem.u32(other);
            let value = kept(e, &reached, at, candidate, bound);
            if value > reference {
                move_before(e, list, node, candidate);
                other = 0;
                changed = true;
                place = at;
            }
            at += 1;
        }
        reached.insert(place.min(reached.len()), (node, reference));
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

#[cfg(test)]
mod tests {
    use super::*;

    /// A property whose list holds lights 1, 2 and 3 units from the bound's centre (radius 10, the
    /// bound's radius 0): luminances 0.1, 0.2, 0.3, already in order. Answers the property, the
    /// bound and the lights.
    fn world(e: &mut Engine) -> (u32, u32, [u32; 3]) {
        e.map(0x011f_9000, 0x1000);
        let node = e.mem.alloc(0x200);
        e.mem.set_u32(SCENE_NODE, node);
        let lights = [1.0f32, 2.0, 3.0].map(|x| {
            let ni = e.mem.alloc(0x100);
            e.mem.set_f32(ni + WORLD_TRANSLATE, x);
            e.mem.set_f32(ni + LIGHT_RADIUS, 10.0);
            let light = e.mem.alloc(0x100);
            e.mem.set_u32(light + LIGHT_SP_LIGHT, ni);
            light
        });
        let items = lights.map(|_| e.mem.alloc(0xc));
        for i in 0..3 {
            let next = if i < 2 { items[i + 1] } else { 0 };
            let previous = if i > 0 { items[i - 1] } else { 0 };
            e.mem.set_u32(items[i], next);
            e.mem.set_u32(items[i] + 4, previous);
            e.mem.set_u32(items[i] + 8, lights[i]);
        }
        let property = e.mem.alloc(0x80);
        e.mem.set_u32(property + LIGHT_LIST, items[0]);
        e.mem.set_u32(property + LIGHT_LIST + 4, items[2]);
        e.mem.set_u32(property + LIGHT_LIST + 8, 3);
        e.mem.set_u32(property + LAST_RENDER_PASS_STATE, 7);
        let bound = e.mem.alloc(0x10);
        (property, bound, lights)
    }

    fn order(e: &Engine, property: u32) -> Vec<u32> {
        let mut out = Vec::new();
        let mut item = e.mem.u32(property + LIGHT_LIST);
        while item != 0 {
            out.push(e.mem.u32(item + 8));
            item = e.mem.u32(item);
        }
        out
    }

    /// Another thread writing the lights' shared `fLuminance` between the scans (here: a value
    /// above every luminance, which read back would move each light to the front) changes nothing:
    /// the order comes only from the values this call worked out.
    #[test]
    fn writes_to_the_shared_luminance_between_scans_do_not_change_the_order() {
        let mut e = Engine::new();
        let (property, bound, lights) = world(&mut e);
        resort_with(&mut e, Ptr::new(property), bound, &mut |e| {
            for light in lights {
                e.mem.set_f32(light + LIGHT_LUMINANCE, 1000.0);
            }
        });
        assert_eq!(order(&e, property), lights.to_vec());
        assert_eq!(e.mem.u32(property + LAST_RENDER_PASS_STATE), 7);
    }

    /// A list out of order is sorted, nearest first, and the pass state cleared; each light's
    /// `fLuminance` is left with its value.
    #[test]
    fn a_list_out_of_order_is_sorted() {
        let mut e = Engine::new();
        let (property, bound, lights) = world(&mut e);
        // Swap the first two lights' places: luminances 0.2, 0.1, 0.3.
        let first = e.mem.u32(lights[0] + LIGHT_SP_LIGHT);
        let second = e.mem.u32(lights[1] + LIGHT_SP_LIGHT);
        e.mem.set_f32(first + WORLD_TRANSLATE, 2.0);
        e.mem.set_f32(second + WORLD_TRANSLATE, 1.0);
        resort_lights(&mut e, Ptr::new(property), bound);
        assert_eq!(order(&e, property), vec![lights[1], lights[0], lights[2]]);
        assert_eq!(e.mem.u32(property + LAST_RENDER_PASS_STATE), 0);
        assert_eq!(e.mem.f32(lights[1] + LIGHT_LUMINANCE), 0.1);
    }
}
