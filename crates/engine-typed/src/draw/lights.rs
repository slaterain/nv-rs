//! The lighting property's first active light (`BSShaderLightingProperty::GetFirstActiveLight`,
//! `GetFirstActiveNonShadowLight`), typed in the draw family so that the world draw's light set-ups read the
//! property's light list and its lights through the value packet (the cut copies them: `cut::take_geometry`).
//! Without an open packet every read is guest memory's, as the translation's.

use super::packet;
use engine_core::ptr::Ptr;
use engine_core::Engine;

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
