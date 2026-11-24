//! The shadow scene node's shadow-light list (`lShadowLightList`, +0xc0) as the frame's set-up walks
//! it: its iterator (`00b5afc0` starts it, `00b5b010` goes on), the shadow light of a caster
//! (`00b5cbd0`: found again or queued new), the sort around the two fences (`00b5cde0`), and the two
//! drops of the lights' dynamic shadow maps (`00b5b060`, `00b5c3c0`).
//!
//! The lists stay in guest memory; their items (`NiTListItem`: next, previous, element) are read
//! and linked in place, as the exe does. What stays by address: the renderer's shadow-map release,
//! the lights' own updates, the list's add, move and empty, the allocation and the interlocked words.
//!
//! **Floats** (main thread only): the sort's fade share `elapsed / time` and `1 - share` are each one
//! operation stored to a float, so plain `f32` gives the exe's float under the main thread's word.

use engine_core::ptr::Ptr;
use engine_core::{args, Engine};

/// The node's members: `lShadowLightList` (+0xc0, head at +0xc0, tail at +0xc4), its iterator
/// (+0xcc), the active and fading-in fences (+0xd0, +0xd4) and their fence objects (+0xd8, +0xdc).
const SHADOW_LIGHT_LIST: u32 = 0xc0;
const ITERATOR: u32 = 0xcc;
const ACTIVE_FENCE: u32 = 0xd0;
const FADING_FENCE: u32 = 0xd4;
const ACTIVE_FENCE_OBJECT: u32 = 0xd8;
const FADING_FENCE_OBJECT: u32 = 0xdc;
/// The light queue's add list (+0xe4).
const LIGHT_QUEUE_ADD: u32 = 0xe4;
/// A `ShadowSceneLight`'s members: the LOD dimmer target and elapsed time (+0xd8, +0xdc), `spLight`
/// (+0xf8), `spDynamicShadowMap` (+0x10c), `spShadowScene` (+0x128).
const LIGHT_TARGET: u32 = 0xd8;
const LIGHT_ELAPSED: u32 = 0xdc;
const LIGHT_SP_LIGHT: u32 = 0xf8;
const LIGHT_DYNAMIC_SHADOW_MAP: u32 = 0x10c;
const LIGHT_SHADOW_SCENE: u32 = 0x128;
/// `NiAVObject::m_uFlags`.
const AV_FLAGS: u32 = 0x30;
/// The renderer object (`011f91a8`) and its shadow-map release (`00b6d4c0`).
const RENDERER: u32 = 0x011f_91a8;
const RENDERER_RELEASE_MAP: u32 = 0x00b6_d4c0;
/// The light's update after its map is dropped (`00b9e770`), its fade restart (`00b9bb10`, a
/// `float` and a flag) and its set-up as a shadow caster's light (`00b9dcb0`).
const LIGHT_AFTER_DROP: u32 = 0x00b9_e770;
const LIGHT_RESTART: u32 = 0x00b9_bb10;
const LIGHT_AS_CASTER: u32 = 0x00b9_dcb0;
/// The list's `RemoveAll` (`00e74d40`), add at the head (`00c1b4a0`), move before (`00b702e0`) and
/// add at the tail (`00a5a310`).
const LIST_REMOVE_ALL: u32 = 0x00e7_4d40;
const LIST_ADD_HEAD: u32 = 0x00c1_b4a0;
const LIST_MOVE_BEFORE: u32 = 0x00b7_02e0;
const LIST_ADD_TAIL: u32 = 0x00a5_a310;
/// The light queues' recursive lock (`011f9ea0`, its count at `011f9ea4`) and its acquire.
const QUEUE_LOCK: u32 = 0x011f_9ea0;
const QUEUE_LOCK_COUNT: u32 = 0x011f_9ea4;
const LOCK_ACQUIRE: u32 = 0x0040_fbf0;
/// The allocation (`00aa13e0`, a size) and the `ShadowSceneLight` constructor (`00b9fda0`).
const NEW_OBJECT: u32 = 0x00aa_13e0;
const SHADOW_SCENE_LIGHT_CONSTRUCT: u32 = 0x00b9_fda0;
/// The sort's limits: `low` and `high` (`double`s) and the fade time (`float`).
const FADE_LOW: u32 = 0x0101_ffa0;
const FADE_HIGH: u32 = 0x0106_b9e8;
const FADE_TIME: u32 = 0x011a_d834;

/// An item's next item, previous item and element.
#[inline(always)]
fn next_of(e: &Engine, item: u32) -> u32 {
    e.mem.u32(item)
}
#[inline(always)]
fn previous_of(e: &Engine, item: u32) -> u32 {
    e.mem.u32(item.wrapping_add(4))
}
#[inline(always)]
fn element_of(e: &Engine, item: u32) -> u32 {
    e.mem.u32(item.wrapping_add(8))
}

/// The inlined `NiPointer` release: the count at +4 down, the object deleted (slot +4) at zero.
fn release(e: &mut Engine, object: u32) {
    let left = engine_core::sync_direct::interlocked_decrement(e, object.wrapping_add(4)) as u32;
    if left == 0 {
        e.vcall(object, 4, &args![]);
    }
}

/// The walk of `00b5afc0` / `00b5b010` from `item`: the first element with an `NiLight`, or the
/// element of the last item reached; the iterator left on the item after the one answered.
fn iterate(e: &mut Engine, iterator: u32, item: u32) -> u32 {
    let mut next = next_of(e, item);
    e.mem.set_u32(iterator, next);
    let mut element = element_of(e, item);
    while next != 0 {
        if element != 0 && e.mem.u32(element.wrapping_add(LIGHT_SP_LIGHT)) != 0 {
            break;
        }
        let item = e.mem.u32(iterator);
        next = next_of(e, item);
        e.mem.set_u32(iterator, next);
        element = element_of(e, item);
    }
    element
}

// Typed from 00b5afc0
/// `thiscall(node)`: starts the walk of `lShadowLightList` (the iterator at its head) and answers its
/// first light with an `NiLight`; the last light without that test; 0 for an empty list.
pub fn first_shadow_light(e: &mut Engine, this: Ptr) -> u32 {
    let head = e.mem.u32(this.addr().wrapping_add(SHADOW_LIGHT_LIST));
    let iterator = this.addr().wrapping_add(ITERATOR);
    e.mem.set_u32(iterator, head);
    if head == 0 {
        return 0;
    }
    iterate(e, iterator, head)
}

// Typed from 00b5b010
/// `thiscall(node)`: goes on with the walk [`first_shadow_light`] started (0 once it is done).
pub fn next_shadow_light(e: &mut Engine, this: Ptr) -> u32 {
    let iterator = this.addr().wrapping_add(ITERATOR);
    let item = e.mem.u32(iterator);
    if item == 0 {
        return 0;
    }
    iterate(e, iterator, item)
}

/// The renderer's release of a light's dynamic shadow map, then the inlined `NiPointer` clear.
fn drop_map(e: &mut Engine, light: u32) {
    let slot = light.wrapping_add(LIGHT_DYNAMIC_SHADOW_MAP);
    let map = e.mem.u32(slot);
    let renderer = e.global::<u32>(RENDERER);
    e.call(RENDERER_RELEASE_MAP, &args![renderer, map]);
    let map = e.mem.u32(slot);
    if map != 0 {
        release(e, map);
        e.mem.set_u32(slot, 0);
    }
}

// Typed from 00b5b060
/// `thiscall(node)`: each light of `lShadowLightList` with an `NiLight` has its dynamic shadow map
/// dropped, flag 1 of its `NiLight` set, and its `00b9e770` run.
pub fn drop_lit_shadow_maps(e: &mut Engine, this: Ptr) {
    let mut item = e.mem.u32(this.addr().wrapping_add(SHADOW_LIGHT_LIST));
    while item != 0 {
        let light = element_of(e, item);
        item = next_of(e, item);
        if light == 0 || e.mem.u32(light.wrapping_add(LIGHT_SP_LIGHT)) == 0 {
            continue;
        }
        drop_map(e, light);
        let ni_light = e.mem.u32(light.wrapping_add(LIGHT_SP_LIGHT));
        let flags = e.mem.u32(ni_light.wrapping_add(AV_FLAGS));
        e.mem.set_u32(ni_light.wrapping_add(AV_FLAGS), flags | 1);
        e.call(LIGHT_AFTER_DROP, &args![light]);
    }
}

// Typed from 00b5c3c0
/// `thiscall(node)`: every light of `lShadowLightList` has its dynamic shadow map dropped; then the
/// list is emptied (`00e74d40`) and both fences cleared.
pub fn drop_shadow_maps(e: &mut Engine, this: Ptr) {
    let list = this.addr().wrapping_add(SHADOW_LIGHT_LIST);
    let mut item = e.mem.u32(list);
    while item != 0 {
        let light = element_of(e, item);
        item = next_of(e, item);
        if light != 0 {
            drop_map(e, light);
        }
    }
    e.call(LIST_REMOVE_ALL, &args![list]);
    e.mem.set_u32(this.addr().wrapping_add(ACTIVE_FENCE), 0);
    e.mem.set_u32(this.addr().wrapping_add(FADING_FENCE), 0);
}

// Typed from 00b5cbd0
/// `thiscall(node, object)`: the shadow light whose `spShadowScene` is `object`, with flag 1 of its
/// `NiLight` cleared and its fade restarted (`00b9bb10` with 1.0 and 0). Without one, under the light
/// queues' lock: a new `ShadowSceneLight` (null when the allocation fails) takes the object as its
/// shadow scene (the inlined `NiPointer` assignment), is set up as a caster's light (`00b9dcb0` with
/// 1) and is appended to the queue's add list through a held `NiPointer` temporary. Answers it.
pub fn shadow_light_of(e: &mut Engine, this: Ptr, object: u32) -> u32 {
    let mut item = e.mem.u32(this.addr().wrapping_add(SHADOW_LIGHT_LIST));
    while item != 0 {
        let light = element_of(e, item);
        item = next_of(e, item);
        if light != 0 && e.mem.u32(light.wrapping_add(LIGHT_SHADOW_SCENE)) == object {
            let ni_light = e.mem.u32(light.wrapping_add(LIGHT_SP_LIGHT));
            let flags = e.mem.u32(ni_light.wrapping_add(AV_FLAGS));
            e.mem.set_u32(ni_light.wrapping_add(AV_FLAGS), flags & !1);
            e.call(LIGHT_RESTART, &args![light, 1.0f32, 0u32]);
            return light;
        }
    }
    e.call(LOCK_ACQUIRE, &args![QUEUE_LOCK, 0u32]);
    let memory = e.call(NEW_OBJECT, &args![0x250u32]).u32();
    let light = if memory != 0 {
        e.call(SHADOW_SCENE_LIGHT_CONSTRUCT, &args![memory]).u32()
    } else {
        0
    };
    // `00b5af30`: the `NiPointer` assignment of the object to `spShadowScene`.
    let slot = light.wrapping_add(LIGHT_SHADOW_SCENE);
    let old = e.mem.u32(slot);
    if old != object {
        if old != 0 {
            release(e, old);
        }
        e.mem.set_u32(slot, object);
        if object != 0 {
            engine_core::sync_direct::interlocked_increment(e, object.wrapping_add(4));
        }
    }
    e.call(LIGHT_AS_CASTER, &args![light, 1u32]);
    e.with_stack(4, |e, temporary| {
        e.mem.set_u32(temporary.addr(), light);
        if light != 0 {
            engine_core::sync_direct::interlocked_increment(e, light.wrapping_add(4));
        }
        e.call(
            LIST_ADD_TAIL,
            &args![this.addr().wrapping_add(LIGHT_QUEUE_ADD), temporary],
        );
        if light != 0 {
            release(e, light);
        }
    });
    let count = e.global::<u32>(QUEUE_LOCK_COUNT).wrapping_sub(1);
    e.set_global(QUEUE_LOCK_COUNT, count);
    if count == 0 {
        e.set_global(QUEUE_LOCK, 0u32);
    }
    light
}

/// The list's head and tail words.
fn head_at(list: u32) -> u32 {
    list
}
fn tail_at(list: u32) -> u32 {
    list.wrapping_add(4)
}

/// The sort's inlined move of `item` to just before `before`.
fn move_before(e: &mut Engine, list: u32, item: u32, before: u32) {
    if e.mem.u32(head_at(list)) == item {
        let next = next_of(e, item);
        e.mem.set_u32(head_at(list), next);
    }
    if e.mem.u32(head_at(list)) == before {
        e.mem.set_u32(head_at(list), item);
    }
    if e.mem.u32(tail_at(list)) == item {
        let previous = previous_of(e, item);
        e.mem.set_u32(tail_at(list), previous);
    }
    let next = next_of(e, item);
    if next != 0 {
        let previous = previous_of(e, item);
        e.mem.set_u32(next.wrapping_add(4), previous);
    }
    let previous = previous_of(e, item);
    if previous != 0 {
        let next = next_of(e, item);
        e.mem.set_u32(previous, next);
    }
    let preceding = previous_of(e, before);
    e.mem.set_u32(item.wrapping_add(4), preceding);
    e.mem.set_u32(item, before);
    if preceding != 0 {
        e.mem.set_u32(preceding, item);
    }
    e.mem.set_u32(before.wrapping_add(4), item);
}

/// `00b5a8d0`, which the sort calls: `item` moved to just after `after` (nothing when they are the
/// same item).
fn move_after(e: &mut Engine, list: u32, item: u32, after: u32) {
    if item == after {
        return;
    }
    if e.mem.u32(head_at(list)) == item {
        let next = next_of(e, item);
        e.mem.set_u32(head_at(list), next);
    }
    if e.mem.u32(tail_at(list)) == item {
        let previous = previous_of(e, item);
        e.mem.set_u32(tail_at(list), previous);
    }
    if e.mem.u32(tail_at(list)) == after {
        e.mem.set_u32(tail_at(list), item);
    }
    let next = next_of(e, item);
    if next != 0 {
        let previous = previous_of(e, item);
        e.mem.set_u32(next.wrapping_add(4), previous);
    }
    let previous = previous_of(e, item);
    if previous != 0 {
        let next = next_of(e, item);
        e.mem.set_u32(previous, next);
    }
    let following = next_of(e, after);
    e.mem.set_u32(item, following);
    e.mem.set_u32(item.wrapping_add(4), after);
    if following != 0 {
        e.mem.set_u32(following.wrapping_add(4), item);
    }
    e.mem.set_u32(after, item);
}

// Typed from 00b5cde0
/// `thiscall(node, light)`: sorts `lShadowLightList` around its two fences by each light's LOD dimmer
/// target and elapsed time. The fading-in fence, then the active one, is set first: a missing one is
/// made at the list's head from its fence object (`00c1b4a0`), an existing one moved before the head
/// (`00b702e0`). Then each item after the fading-in fence, with `low`, `high` and the fade `time`: a
/// light at most `low` and not yet faded moves before the active fence (unless it is that fence); one
/// at least `high`, not yet faded, whose share of the fade (`elapsed / time`, or one less that unless
/// its target is above 0) is above `low`, moves after the active fence; otherwise one whose `NiLight`
/// is not culled and that is below `high` or faded goes after the fading-in fence (`00b702e0` for
/// the light given, the inlined move for any other). The time is read again after each move.
pub fn sort_shadow_lights(e: &mut Engine, this: Ptr, argument: u32) {
    let base = this.addr();
    let list = base.wrapping_add(SHADOW_LIGHT_LIST);
    for (fence, object) in [
        (FADING_FENCE, FADING_FENCE_OBJECT),
        (ACTIVE_FENCE, ACTIVE_FENCE_OBJECT),
    ] {
        let current = e.mem.u32(base.wrapping_add(fence));
        if current == 0 {
            e.call(LIST_ADD_HEAD, &args![list, base.wrapping_add(object)]);
            let head = e.mem.u32(list);
            e.mem.set_u32(base.wrapping_add(fence), head);
        } else {
            let head = e.mem.u32(list);
            e.call(LIST_MOVE_BEFORE, &args![list, current, head]);
        }
    }
    let fading = e.mem.u32(base.wrapping_add(FADING_FENCE));
    if fading == 0 {
        return;
    }
    let mut next = next_of(e, fading);
    if next == 0 {
        return;
    }
    let mut time = e.global::<f32>(FADE_TIME);
    while next != 0 {
        let low = e.global::<f64>(FADE_LOW);
        let high = e.global::<f64>(FADE_HIGH);
        let item = next;
        next = next_of(e, item);
        let light = element_of(e, item);
        if light == 0 {
            continue;
        }
        let target = e.mem.f32(light.wrapping_add(LIGHT_TARGET));
        let elapsed = e.mem.f32(light.wrapping_add(LIGHT_ELAPSED));
        if f64::from(target) <= low && elapsed < time {
            let active = e.mem.u32(base.wrapping_add(ACTIVE_FENCE));
            if item != active {
                move_before(e, list, item, active);
                time = e.global::<f32>(FADE_TIME);
            }
            continue;
        }
        if f64::from(target) >= high && elapsed < time {
            let mut share = elapsed / time;
            // Unless the target is above 0 (a NaN target included).
            if target.partial_cmp(&0.0) != Some(std::cmp::Ordering::Greater) {
                share = 1.0 - share;
            }
            if f64::from(share) > low {
                let active = e.mem.u32(base.wrapping_add(ACTIVE_FENCE));
                move_after(e, list, item, active);
                time = e.global::<f32>(FADE_TIME);
                continue;
            }
        }
        let ni_light = e.mem.u32(light.wrapping_add(LIGHT_SP_LIGHT));
        if e.mem.u32(ni_light.wrapping_add(AV_FLAGS)) & 1 != 0 {
            continue;
        }
        let target = e.mem.f32(light.wrapping_add(LIGHT_TARGET));
        if f64::from(target) >= high && e.mem.f32(light.wrapping_add(LIGHT_ELAPSED)) < time {
            continue;
        }
        let fading = e.mem.u32(base.wrapping_add(FADING_FENCE));
        if light == argument {
            e.call(LIST_MOVE_BEFORE, &args![list, item, fading]);
        } else {
            move_after(e, list, item, fading);
        }
        time = e.global::<f32>(FADE_TIME);
    }
}
