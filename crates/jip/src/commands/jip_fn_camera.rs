//! Two JIP LN NVSE commands that switch a piece of the player's camera off by rewriting one byte
//! of the game's code: `ToggleVanityWheel` (`functions_jip/jip_fn_input.h`: the mouse wheel's
//! zoom) and `ToggleCameraCollision` (`functions_jip/jip_fn_miscellaneous.h`: the third-person
//! camera pulled in front of walls).
//!
//! The port has no code bytes to write: each byte is a seam in the translated function
//! (engine-fallout-ai's `playercharacter.rs`), asked while JIP is installed, and the handler
//! answers JIP's switch. Both switches are JIP's statics: run-wide, lock-free, never saved (the
//! code bytes stay as written until the game ends, as in JIP).

use super::Handlers;
use crate::game_patches::Area;
use engine_core::extend::{Owner, SiteSlot};
use engine_core::prelude::*;
use nvse::commands::Read;
use nvse::{handled, CommandArgs};
use std::sync::atomic::{AtomicBool, Ordering};

/// `jip_fn_input.h`'s command.
#[rustfmt::skip]
pub static INPUT_COMMANDS: Handlers = &[
    (0x28b4, toggle_vanity_wheel),
];
/// `jip_fn_miscellaneous.h`'s command.
#[rustfmt::skip]
pub static MISC_COMMANDS: Handlers = &[
    (0x28d6, toggle_camera_collision),
];

/// The zoom test in the player's look handling (`009445b0`): JIP's byte at `0x945A29` is the
/// `JNP`'s second opcode byte at `0x945A28` (`0x8B`; `0x89`, `JNS`, always taken, while the wheel
/// is off).
pub const VANITY_WHEEL: SiteSlot = SiteSlot::new(Owner::Jip, 0x0094_5a28);
/// The `JZ` after the camera pick in the player's camera distance (`0094a0c0`): `0x74`; `0xEB`,
/// `JMP`, while collision is off.
pub const CAMERA_COLLISION: SiteSlot = SiteSlot::new(Owner::Jip, 0x0094_a34e);

/// JIP's `s_vanityEnabled` and `ToggleCameraCollision`'s `cameraCollision` (both true at start).
pub struct CameraSwitches {
    vanity: AtomicBool,
    collision: AtomicBool,
}

impl Default for CameraSwitches {
    fn default() -> Self {
        CameraSwitches {
            vanity: AtomicBool::new(true),
            collision: AtomicBool::new(true),
        }
    }
}

fn switches(e: &mut Engine) -> &CameraSwitches {
    e.ext_free::<CameraSwitches>()
}

/// The switches' slot (made before the threads start).
pub fn install(e: &mut Engine) {
    e.ext_free::<CameraSwitches>();
}

/// The two seams: in with JIP's `INSTALLED`, as JIP's other patches.
pub const AREA: Area = Area {
    name: "camera",
    fix_sites: &[],
    fix_wraps: &[],
    install_fixes: crate::game_patches::nothing,
    jip_sites: &[VANITY_WHEEL.addr(), CAMERA_COLLISION.addr()],
    jip_wraps: &[],
    install_jip,
};

fn install_jip(e: &mut Engine) {
    e.on_site(VANITY_WHEEL, vanity_wheel);
    e.on_site(CAMERA_COLLISION, camera_collision);
}

/// The zoom test's answer: 1 (skip the zoom) while the wheel is off.
fn vanity_wheel(e: &mut Engine, _: &[u32]) -> Option<u32> {
    Some(u32::from(!switches(e).vanity.load(Ordering::Relaxed)))
}

/// The pick's answer: 1 (as if nothing were hit) while collision is off.
fn camera_collision(e: &mut Engine, _: &[u32]) -> Option<u32> {
    Some(u32::from(!switches(e).collision.load(Ordering::Relaxed)))
}

/// JIP's toggle: answers 1 while the switch is on; with an argument read, a call that asks for
/// the other state flips it (`toggle` 0 off, any other value on).
fn toggle(e: &mut Engine, w: &[u32], pick: fn(&CameraSwitches) -> &AtomicBool) -> Ret {
    let a = CommandArgs::new(w);
    let on = pick(switches(e)).load(Ordering::Relaxed);
    if on {
        a.set_result(e, 1.0);
    }
    if super::jip_fn_utility::num_args(e, &a) != 0 {
        let read = Read::of(e, &a);
        if read.ok && on == (read.int(0, 0) == 0) {
            pick(switches(e)).store(!on, Ordering::Relaxed);
        }
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_input.h:
// Cmd_ToggleVanityWheel_Execute.
/// `ToggleVanityWheel [toggle]`: whether the mouse wheel zooms the camera (1 on); with 0 the
/// wheel stops zooming, with any other value it zooms again.
fn toggle_vanity_wheel(e: &mut Engine, w: &[u32]) -> Ret {
    toggle(e, w, |s| &s.vanity)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_miscellaneous.h:
// Cmd_ToggleCameraCollision_Execute.
/// `ToggleCameraCollision [toggle]`: whether the third-person camera is pulled in front of what it
/// hits (1 on); with 0 it passes through walls, with any other value it is pulled in again.
fn toggle_camera_collision(e: &mut Engine, w: &[u32]) -> Ret {
    toggle(e, w, |s| &s.collision)
}

#[cfg(test)]
#[path = "jip_fn_camera_tests.rs"]
mod tests;
