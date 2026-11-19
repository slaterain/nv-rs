//! `functions_jip/jip_fn_misc_ref.h` (JIP LN NVSE): `AttachLight` and `RemoveLight`, a point light
//! of a light form put under a node of a reference's model (or taken out again). The rest of the
//! file's commands are in `jip_fn_misc_ref.rs` and the `jip_fn_misc_ref_*` modules.
//!
//! What they do, as JIP's command bodies:
//! - the node: JIP's `TESObjectREFR::GetNode2` ([`super::havok_refs::node2`]: the loaded root, or
//!   the node of that name below it);
//! - `AttachLight`: JIP's `CreatePointLight` ([`gl_lights::create_point_light`]: the node's first
//!   point light child, else a new point light named `"PtLight "` and the form's id, put under
//!   the node), the given offset as its local translation (+0x58, three floats), JIP's mark of a
//!   script's light (`extraFlags` +0x9F bit 0x80), the result 1;
//! - `RemoveLight`: the node's first child that is a point light with that mark taken out
//!   (`RemoveObject`, vtable +0xE8).
//!
//! The light is one of JIP's (listed in `s_activePtLights`, `game_patches::gl::lights`): JIP's
//! animated-lights step (H12, in with JIP installed) sets it from its form, puts it into the scene's
//! lights at its next pass and frees it once it is out of the model and let go. JIP 57.30 keeps no
//! record of which reference has it: when the reference's 3D is made again the light is gone, and
//! before a game loads and at the exit to the main menu every light with the script mark is taken
//! off its node (`gl::lights::pre_load`, JIP's pre-load housekeeping), as in JIP.
//!
//! Threads: on the script's thread, as JIP's (no lock: JIP takes none); the list has its own lock.

use super::havok_refs::node2;
use super::jip_fn_misc_ref_nodes::block_name;
use super::jip_fn_misc_ref_nodes_w::REMOVE_OBJECT_SLOT;
use super::{type_of, Handlers};
use crate::game_patches::gl::lights as gl_lights;
use engine_core::prelude::*;
use nvse::commands::Read;
use nvse::{handled, CommandArgs};

#[rustfmt::skip]
pub static COMMANDS: Handlers = &[
    (0x2879, attach_light),
    (0x287a, remove_light),
];

/// `kFormType_TESObjectLIGH`.
const LIGHT_TYPE: u8 = 0x1e;
/// A point light's local translation (+0x58), JIP's `extraFlags` (+0x9F) and its script mark.
const LOCAL_TRANSLATE: u32 = 0x58;
const EXTRA_FLAGS: u32 = 0x9f;
const SCRIPT_LIGHT: u8 = 0x80;
/// A node's children (`NiTArray`: data +0xA0, count +0xA6).
const CHILDREN: u32 = 0xa0;
const CHILD_COUNT: u32 = 0xa6;

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_misc_ref.h:
// Cmd_AttachLight_Execute.
/// A node's name, a light form and an offset (x, y, z; 0 when not given): the light put under the
/// node at the offset; 1 when the node is there.
fn attach_light(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if !read.ok || a.this_obj == 0 {
        return handled(true);
    }
    let name = block_name(e, &read, 0, "AttachLight");
    let form = read.form(1);
    if form == 0 || type_of(e, form) != LIGHT_TYPE {
        return handled(true);
    }
    let offset = [read.float(2, 0.0), read.float(3, 0.0), read.float(4, 0.0)];
    let node = node2(e, a.this_obj, &name);
    if node == 0 {
        return handled(true);
    }
    let light = gl_lights::create_point_light(e, form, node);
    if light == 0 {
        return handled(true);
    }
    for (i, v) in offset.iter().enumerate() {
        e.mem
            .set_f32(light.wrapping_add(LOCAL_TRANSLATE + 4 * i as u32), *v);
    }
    let flags = e.mem.u8(light.wrapping_add(EXTRA_FLAGS));
    e.mem
        .set_u8(light.wrapping_add(EXTRA_FLAGS), flags | SCRIPT_LIGHT);
    a.set_result(e, 1.0);
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_misc_ref.h:
// Cmd_RemoveLight_Execute.
/// A node's name: the node's first point light child with the script mark taken out. No result.
fn remove_light(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if !read.ok || a.this_obj == 0 {
        return handled(true);
    }
    let name = block_name(e, &read, 0, "RemoveLight");
    let node = node2(e, a.this_obj, &name);
    if node == 0 {
        return handled(true);
    }
    let count = e.mem.u16(node.wrapping_add(CHILD_COUNT));
    let children = e.mem.u32(node.wrapping_add(CHILDREN));
    for i in 0..u32::from(count) {
        let child = e.mem.u32(children.wrapping_add(4 * i));
        if child == 0
            || e.mem.u32(child) != gl_lights::POINT_LIGHT_VTABLE
            || e.mem.u8(child.wrapping_add(EXTRA_FLAGS)) & SCRIPT_LIGHT == 0
        {
            continue;
        }
        e.vcall(node, REMOVE_OBJECT_SLOT, &[child]);
        break;
    }
    handled(true)
}

#[cfg(test)]
#[path = "jip_fn_misc_ref_lights_tests.rs"]
mod tests;
