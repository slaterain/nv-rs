//! Two `functions_jip/jip_fn_misc_ref.h` commands (JIP LN NVSE) that work through JIP's flags
//! byte in a reference (`+0x5F`): `ToggleNoZPosReset` (bit 0x10: the game stops moving the
//! reference back onto the ground when its area loads or it is warped) and
//! `ToggleObjectCollision` (bit 2: the reference's collision turned off, again whenever its 3D
//! loads).
//!
//! `ToggleNoZPosReset` rewrites two pieces of the game's code the first time it is turned on;
//! the port has a seam at each (engine-fallout-unplaced-a's `run_006286f0.rs` at `0x629006`,
//! engine-fallout-env's `loadedareabound.rs` at `0x6295E9`), put in only while JIP answers
//! "installed" ([`AREA`]), answering nothing until the command has run.
//! `ToggleObjectCollision` changes no code: JIP's attach of a queued reference (group GP's
//! `queued_reference`) turns a marked reference's collision off again after its 3D loads
//! ([`toggle_collision`]).

use super::Handlers;
use crate::game_patches::Area;
use engine_core::extend::{Owner, SiteSlot};
use engine_core::prelude::*;
use nvse::commands::Read;
use nvse::{handled, CommandArgs};
use std::sync::atomic::{AtomicBool, Ordering};

#[rustfmt::skip]
pub static COMMANDS: Handlers = &[
    (0x26db, toggle_no_z_pos_reset),
    (0x2789, toggle_object_collision),
];

/// The two places JIP's ToggleNoZPosReset rewrites.
pub const AREA_RESET: SiteSlot = SiteSlot::new(Owner::Jip, 0x0062_9006);
pub const WARP_ACTOR: SiteSlot = SiteSlot::new(Owner::Jip, 0x0062_95e9);

/// JIP's flags byte in a reference and its bits (`kHookRefFlag5F_DisableCollision`,
/// `kHookRefFlag5F_NoZPosReset`).
const REF_FLAGS_5F: u32 = 0x5f;
pub const REF_FLAG_DISABLE_COLLISION: u8 = 1 << 1;
const REF_FLAG_NO_Z_POS_RESET: u8 = 1 << 4;

/// Whether JIP has written ToggleNoZPosReset's code (`*(UInt32*)0x629006 == 0x0C74C085`).
#[derive(Default)]
pub struct NoZPosReset {
    written: AtomicBool,
}

fn written(e: &mut Engine) -> bool {
    e.ext_free::<NoZPosReset>().written.load(Ordering::Relaxed)
}

/// The slot (made before the threads start).
pub fn install(e: &mut Engine) {
    e.ext_free::<NoZPosReset>();
}

/// The two seams: in with JIP's `INSTALLED`, as JIP's other patches.
pub const AREA: Area = Area {
    name: "no height reset",
    fix_sites: &[],
    fix_wraps: &[],
    install_fixes: crate::game_patches::nothing,
    jip_sites: &[AREA_RESET.addr(), WARP_ACTOR.addr()],
    jip_wraps: &[],
    install_jip,
};

fn install_jip(e: &mut Engine) {
    e.on_site(AREA_RESET, area_reset);
    e.on_site(WARP_ACTOR, warp_actor);
}

fn no_z_pos_reset(e: &Engine, refr: u32) -> bool {
    e.mem.u8(refr.wrapping_add(REF_FLAGS_5F)) & REF_FLAG_NO_Z_POS_RESET != 0
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_misc_ref.h:
// Cmd_ToggleNoZPosReset_Execute (its bytes at 0x629006).
/// The area's reset of a reference (`a`: the collidable reference, 0 for none), once JIP's code
/// is in: 1 (skip) when the reference has JIP's flag, else 0. Before, nothing.
fn area_reset(e: &mut Engine, a: &[u32]) -> Option<u32> {
    if !written(e) {
        return None;
    }
    let refr = a[0];
    Some(u32::from(refr != 0 && no_z_pos_reset(e, refr)))
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_misc_ref.h:
// Cmd_ToggleNoZPosReset_Execute (its bytes at 0x6295E9).
/// The warp of an actor (`a`: the actor), once JIP's code is in: 1 (skip) when the actor has
/// JIP's flag, else 0. Before, nothing.
fn warp_actor(e: &mut Engine, a: &[u32]) -> Option<u32> {
    if !written(e) {
        return None;
    }
    Some(u32::from(no_z_pos_reset(e, a[0])))
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_misc_ref.h:
// Cmd_ToggleNoZPosReset_Execute.
/// `ref.ToggleNoZPosReset toggle`: with a toggle other than 0 the reference keeps its height
/// when its area loads or it is warped (JIP's code written the first time); with 0 it is reset
/// again.
fn toggle_no_z_pos_reset(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if !read.ok || a.this_obj == 0 {
        return handled(true);
    }
    let at = a.this_obj.wrapping_add(REF_FLAGS_5F);
    let flags = e.mem.u8(at);
    if read.int(0, 0) != 0 {
        e.ext_free::<NoZPosReset>()
            .written
            .store(true, Ordering::Relaxed);
        e.mem.set_u8(at, flags | REF_FLAG_NO_Z_POS_RESET);
    } else {
        e.mem.set_u8(at, flags & !REF_FLAG_NO_Z_POS_RESET);
    }
    handled(true)
}

/// The shared `return true` and `return this` functions (an actor's slot `0x100`, a node's
/// `0xC`); an actor's process (`+0x68`), its level (`+0x28`), the middle-high process's
/// character controller words (`+0x138`, then `+0x594`, then `+8`); a reference's loaded data
/// (`+0x64`) and its 3D (`+0x14`).
const RETURN_TRUE: u32 = 0x008d_0360;
const RETURN_THIS: u32 = 0x0068_15c0;
const PROCESS: u32 = 0x68;
const PROCESS_LEVEL: u32 = 0x28;
const LOADED_DATA: u32 = 0x64;
const LOADED_3D: u32 = 0x14;
/// A Havok object's collision filter byte (`+0x2D`, bit 0x40 "no collision").
const FILTER_BYTE: u32 = 0x2d;

/// The filter byte at `object + 0x2D` given `flag` in bit 0x40.
fn set_filter(e: &mut Engine, object: u32, flag: u8) {
    let at = object.wrapping_add(FILTER_BYTE);
    let byte = e.mem.u8(at);
    e.mem.set_u8(at, (byte & 0xbf) | flag);
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/netimmerse.cpp: NiNode::ToggleCollision
// (asm).
/// The node's collision object's world object (`+0x1C`, then `+0x10`): its Havok object's
/// (`+8`) filter byte given `flag` and the world object updated (slot `0xC4`); then every child
/// node (`+0xA0`, `0xA6` of them; a child whose slot `0xC` is `return this`) the same.
fn node_toggle_collision(e: &mut Engine, node: u32, flag: u8) {
    let collision = e.mem.u32(node.wrapping_add(0x1c));
    if collision != 0 {
        let world_object = e.mem.u32(collision.wrapping_add(0x10));
        if world_object != 0 {
            let havok = e.mem.u32(world_object.wrapping_add(8));
            set_filter(e, havok, flag);
            e.vcall(world_object, 0xc4, &[]);
        }
    }
    let count = u32::from(e.mem.u16(node.wrapping_add(0xa6)));
    let children = e.mem.u32(node.wrapping_add(0xa0));
    for i in 0..count {
        let child = e.mem.u32(children.wrapping_add(4 * i));
        if child != 0 && e.mem.u32(e.mem.u32(child).wrapping_add(0xc)) == RETURN_THIS {
            node_toggle_collision(e, child, flag);
        }
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), nvse/GameObjects.cpp:
// TESObjectREFR::ToggleCollision (asm).
/// The reference's collision on (`toggle`) or off: an actor's character controller (a middle or
/// high process's, `+0x138`, `+0x594`, `+8`) gets the filter bit, then every collision object of
/// the reference's 3D ([`node_toggle_collision`]).
pub fn toggle_collision(e: &mut Engine, refr: u32, toggle: bool) {
    let flag = if toggle { 0 } else { 0x40 };
    let vtable = e.mem.u32(refr);
    if e.mem.u32(vtable.wrapping_add(0x100)) == RETURN_TRUE {
        let process = e.mem.u32(refr.wrapping_add(PROCESS));
        if process != 0 && e.mem.u8(process.wrapping_add(PROCESS_LEVEL)) <= 1 {
            let words = e.mem.u32(process.wrapping_add(0x138));
            let controller = if words != 0 {
                e.mem.u32(words.wrapping_add(0x594))
            } else {
                0
            };
            let object = if controller != 0 {
                e.mem.u32(controller.wrapping_add(8))
            } else {
                0
            };
            if object != 0 {
                set_filter(e, object, flag);
            }
        }
    }
    let loaded = e.mem.u32(refr.wrapping_add(LOADED_DATA));
    if loaded == 0 {
        return;
    }
    let node = e.mem.u32(loaded.wrapping_add(LOADED_3D));
    if node != 0 {
        node_toggle_collision(e, node, flag);
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_misc_ref.h:
// ToggleObjectCollision (asm) and Cmd_ToggleObjectCollision_Execute.
/// `ref.ToggleObjectCollision enable`: the reference's collision off (0) or on (any other
/// value); nothing when it already is. JIP's flag keeps it off when the 3D loads again.
fn toggle_object_collision(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let refr = a.this_obj;
    if !read.ok || refr == 0 {
        return handled(true);
    }
    let enable = read.int(0, 0) != 0;
    let at = refr.wrapping_add(REF_FLAGS_5F);
    let flags = e.mem.u8(at);
    let enabled = flags & REF_FLAG_DISABLE_COLLISION == 0;
    if enabled != enable {
        e.mem.set_u8(at, flags ^ REF_FLAG_DISABLE_COLLISION);
        toggle_collision(e, refr, enable);
    }
    handled(true)
}

#[cfg(test)]
#[path = "jip_fn_misc_ref_code_tests.rs"]
mod tests;
