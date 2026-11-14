//! The lighting property's light list walks (`BSShaderLightingProperty`'s first and next active light, with and
//! without the shadow lights, and the two counts), a pass light's clip-plane store and the shadow scene node's camera
//! position, typed in the draw family so that the world draw's light set-ups read the property's light list, its
//! lights and the node through the value packet (the cut copies them: `cut::take_geometry`, `cut::take_frame`) and
//! write the clip-plane cache as the draw's own state. Without an open packet every read and write is guest memory's,
//! as the translation's.

use super::{capture, packet};
use engine_core::ptr::Ptr;
use engine_core::{typed_call, Engine};

/// Whether a light is active: its index (`u16` +0x110) is not 0xff and its light's (+0xf8) app-culled bit (byte
/// +0x30, bit 0) is clear; for the non-shadow form also its byte +0xec is not 1.
fn active(e: &Engine, light: u32, non_shadow: bool) -> bool {
    packet::pass_light_half(e, light, 0x110) != 0xff
        && packet::light_value_byte(e, packet::pass_light_word(e, light, 0xf8), 0x30) & 1 == 0
        && (!non_shadow || packet::pass_light_byte(e, light, 0xec) != 1)
}

/// The walk both getters share: the list (+0x60) from its head, the iterator `iterator` left on each node it
/// passes, as the exe stores it; the first active light, or 0.
fn first(e: &mut Engine, property: u32, iterator: u32, non_shadow: bool) -> u32 {
    let head = packet::property_light_list(e, property);
    e.set_global(iterator, head); // the caller's iterator, a local of its frame
    if head == 0 {
        return 0;
    }
    let mut next = packet::light_node_word(e, head, 0);
    e.set_global(iterator, next);
    let mut light = packet::light_node_word(e, head, 8);
    while next != 0 {
        if light == 0 {
            return 0;
        }
        if active(e, light, non_shadow) {
            break;
        }
        let node = e.global::<u32>(iterator);
        next = packet::light_node_word(e, node, 0);
        e.set_global(iterator, next);
        light = packet::light_node_word(e, node, 8);
    }
    if light != 0 && active(e, light, non_shadow) {
        light
    } else {
        0
    }
}

// Typed from 00b70590
/// `BSShaderLightingProperty::GetFirstActiveLight` (engine map), `thiscall(property, iterator)`: the first light
/// of the property's list (+0x60; nodes: next +0, light +8) whose index (+0x110) is not 0xff and whose light (+0xf8)
/// is not app-culled (its byte +0x30, bit 0), or 0; the iterator is left on the node after the one examined last.
pub fn first_active_light(e: &mut Engine, property: Ptr, iterator: Ptr) -> u32 {
    first(e, property.addr(), iterator.addr(), false)
}

// Typed from 00b70600
/// `BSShaderLightingProperty::GetFirstActiveNonShadowLight` (engine map), `thiscall(property, iterator)`: as
/// [`first_active_light`], skipping also the lights whose byte +0xec is 1.
pub fn first_active_non_shadow_light(e: &mut Engine, property: Ptr, iterator: Ptr) -> u32 {
    first(e, property.addr(), iterator.addr(), true)
}

/// The walk both next-light getters share, from the iterator's node (the word at `iterator`, a local of the caller's
/// frame): that node's light, then on while the list goes on and the light is not active, the iterator left on the
/// node after the one examined last, as the exe stores it; the light when active, else 0 (also for a node without a
/// light).
fn next_light(e: &mut Engine, iterator: u32, non_shadow: bool) -> u32 {
    let node = e.global::<u32>(iterator);
    if node == 0 {
        return 0;
    }
    let mut next = packet::light_node_word(e, node, 0);
    e.set_global(iterator, next);
    let mut light = packet::light_node_word(e, node, 8);
    if light == 0 {
        return 0;
    }
    while next != 0 && !active(e, light, non_shadow) {
        let node = next;
        next = packet::light_node_word(e, node, 0);
        e.set_global(iterator, next);
        light = packet::light_node_word(e, node, 8);
        if light == 0 {
            return 0;
        }
    }
    if active(e, light, non_shadow) {
        light
    } else {
        0
    }
}

// Typed from 00b70680
/// `BSShaderLightingProperty::GetNextActiveLight` (engine map), `thiscall(property, iterator)` (the property not
/// read): from the node the iterator holds, the next light whose index (+0x110) is not 0xff and whose light (+0xf8) is
/// not app-culled, or 0; the iterator is left on the node after the one examined last.
pub fn next_active_light(e: &mut Engine, _property: Ptr, iterator: Ptr) -> u32 {
    next_light(e, iterator.addr(), false)
}

// Typed from 00b70700
/// `BSShaderLightingProperty::GetNextActiveNonShadowLight` (engine map), `thiscall(property, iterator)` (the property
/// not read): as [`next_active_light`], skipping also the lights whose byte +0xec is 1.
pub fn next_active_non_shadow_light(e: &mut Engine, _property: Ptr, iterator: Ptr) -> u32 {
    next_light(e, iterator.addr(), true)
}

// Typed from 00b707d0
/// `BSShaderLightingProperty::GetNumberofActiveNonShadowLights` (engine map), `thiscall(property)`: how many lights
/// of the property's list (+0x60) are present, of an index (+0x110) other than 0xff, not app-culled (their light's
/// +0x30, bit 0) and of byte +0xec 0 (this one tests 0, where the getters skip 1).
pub fn active_non_shadow_light_count(e: &mut Engine, property: Ptr) -> u32 {
    let mut node = packet::property_light_list(e, property.addr());
    let mut count = 0u32;
    while node != 0 {
        let light = packet::light_node_word(e, node, 8);
        node = packet::light_node_word(e, node, 0);
        if light != 0
            && packet::pass_light_half(e, light, 0x110) != 0xff
            && packet::light_value_byte(e, packet::pass_light_word(e, light, 0xf8), 0x30) & 1 == 0
            && packet::pass_light_byte(e, light, 0xec) == 0
        {
            count = count.wrapping_add(1);
        }
    }
    count
}

/// A pass light's clip-plane cache (+0x1a8, six planes: 0x60 bytes) and the byte that says it is made (+0x208).
const CLIP_PLANE_CACHE: u32 = crate::shader::geometry::CLIP_PLANE_CACHE;
const CLIP_PLANES_SIZE: u32 = crate::shader::geometry::CLIP_PLANES_SIZE;
const CLIP_PLANES_MADE: u32 = crate::shader::geometry::CLIP_PLANES_MADE;

// Typed from 00b71ad0
/// A pass light's clip planes stored, `thiscall(light, planes)`: with planes, their 0x60 bytes copied into the light's
/// cache (+0x1a8, `_memcpy`) and its byte +0x208 set; without, the byte cleared. The cache is the draw's own state (the
/// cut owns it per pass light), written through the packet while it is open.
pub fn store_clip_planes(e: &mut Engine, light: Ptr, planes: u32) {
    let cache = light.addr().wrapping_add(CLIP_PLANE_CACHE);
    if planes != 0 {
        capture::own_image(e, cache, CLIP_PLANES_SIZE, |e, at| {
            let _: u32 = typed_call!(
                crate::crt::FAMILY,
                0x00ec_44d0,
                crate::crt::mem::memcpy(e, at, planes, CLIP_PLANES_SIZE)
            );
        });
        capture::set_own_at::<u8>(e, cache.wrapping_add(CLIP_PLANES_MADE), 1);
    } else {
        capture::set_own_at::<u8>(e, cache.wrapping_add(CLIP_PLANES_MADE), 0);
    }
}

// Typed from 00b71b40
/// A shadow scene node's camera position, `thiscall(node, out)`: its three floats at +0x1e4 copied to `out` (+0x1e4,
/// then +0x1ec, then +0x1e8, as the exe reads them), the packet's (the cut copies the scene node's).
pub fn camera_position(e: &mut Engine, node: Ptr, out: Ptr) {
    let (node, out) = (node.addr(), out.addr());
    let x = capture::word(e, node, 0x1e4);
    e.set_global(out, x);
    let z = capture::word(e, node, 0x1ec);
    let y = capture::word(e, node, 0x1e8);
    e.set_global(out.wrapping_add(4), y);
    e.set_global(out.wrapping_add(8), z);
}

// Typed from 00b70790
/// `BSShaderLightingProperty::GetNumberofActiveLights` (engine map), `thiscall(property)`: how many lights of the
/// property's list (+0x60) are present, of an index (+0x110) other than 0xff and not app-culled (their light's +0x30,
/// bit 0).
pub fn active_light_count(e: &mut Engine, property: Ptr) -> u32 {
    let mut node = packet::property_light_list(e, property.addr());
    let mut count = 0u32;
    while node != 0 {
        let light = packet::light_node_word(e, node, 8);
        node = packet::light_node_word(e, node, 0);
        if light != 0 && active(e, light, false) {
            count = count.wrapping_add(1);
        }
    }
    count
}
