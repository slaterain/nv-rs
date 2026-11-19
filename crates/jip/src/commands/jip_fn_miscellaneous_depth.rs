//! `functions_jip/jip_fn_miscellaneous.h` (JIP LN NVSE): `ToggleDepthClear`, whether the game
//! clears the depth buffer before it draws the first-person view and the interface over the world
//! (JIP's patch G128). The rest of the file's commands are in `jip_fn_miscellaneous.rs` and the
//! `jip_fn_miscellaneous_*` modules.
//!
//! JIP puts its `RendererClearBufferHook` on the call at `0x8751C6` (in the frame's draw after the
//! world, `00875110`: the renderer's `ClearBuffer(nullptr, 4)`, the depth buffer) whenever JIP is
//! in: while JIP's `s_clearDepthBuffer` is 0 the hook returns without the call. The port has no
//! call to rewrite: the translated `00875110` asks the seam [`DEPTH_CLEAR`] there while JIP is
//! installed, and the handler answers (the call skipped) while the switch is 0. A skipped clear still
//! reaches the port-added point `engine_core::world_drawn::depth_clear_skipped` in its place, where
//! the platform marks the world as drawn as it does at the clear (the new lighting's camera, the sun
//! shadows' end of casters): nv-rs's own, since those marks have no other place.
//!
//! Threads: the switch is JIP's static: run-wide, lock-free, never saved (1 at start, as JIP's).

use super::Handlers;
use crate::game_patches::Area;
use engine_core::extend::{Owner, SiteSlot};
use engine_core::prelude::*;
use nvse::commands::Read;
use nvse::{handled, CommandArgs};
use std::sync::atomic::{AtomicU32, Ordering};

#[rustfmt::skip]
pub static COMMANDS: Handlers = &[
    (0x293d, toggle_depth_clear),
];

/// The depth buffer's clear in `00875110` (`CALL 0x007148C0` at `0x8751C6`; no words; answered:
/// the clear skipped).
pub const DEPTH_CLEAR: SiteSlot = SiteSlot::new(Owner::Jip, 0x0087_51c6);

/// JIP's `s_clearDepthBuffer` (1 at start).
pub struct DepthClear(AtomicU32);

impl Default for DepthClear {
    fn default() -> Self {
        DepthClear(AtomicU32::new(1))
    }
}

/// The switch's slot (made before the threads start).
pub fn install(e: &mut Engine) {
    e.ext_free::<DepthClear>();
}

/// The switch, for the tests.
pub fn depth_clear(e: &mut Engine) -> u32 {
    e.ext_free::<DepthClear>().0.load(Ordering::Relaxed)
}

/// The seam: in with JIP's `INSTALLED`, as JIP's hook (written at JIP's start, not by the command).
pub const AREA: Area = Area {
    name: "depth clear",
    fix_sites: &[],
    fix_wraps: &[],
    install_fixes: crate::game_patches::nothing,
    jip_sites: &[DEPTH_CLEAR.addr()],
    jip_wraps: &[],
    install_jip,
};

fn install_jip(e: &mut Engine) {
    e.on_site(DEPTH_CLEAR, depth_clear_site);
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_game.h: RendererClearBufferHook
// (asm).
/// The clear's answer: skipped (`retn 8` without the call) while the switch is 0; else no answer
/// and the game clears (JIP's jump on to `0x7148C0`).
fn depth_clear_site(e: &mut Engine, _: &[u32]) -> Option<u32> {
    (depth_clear(e) == 0).then_some(0)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_miscellaneous.h:
// Cmd_ToggleDepthClear_Execute.
/// `ToggleDepthClear clear`: the switch set to the number (0: the game no longer clears the depth
/// buffer before the first-person view; any other value: it clears again). No result; nothing
/// changes when the argument does not read.
fn toggle_depth_clear(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if read.ok {
        let value = read.int(0, depth_clear(e));
        e.ext_free::<DepthClear>().0.store(value, Ordering::Relaxed);
    }
    handled(true)
}

#[cfg(test)]
#[path = "jip_fn_miscellaneous_depth_tests.rs"]
mod tests;
