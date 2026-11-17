//! A port-added point in the frame's draw (not in the original): the world is in the scene target
//! and nothing after it is yet. The world frames (`00870bd0`, and `00870a00`, its variant with the
//! shadow scene node's byte +0x131 set) reach it just before their call of `00875110`, the rest of
//! the draw: after the world pass (`00873200`: the opaque and alpha-tested world, the player's
//! body in third person), the water and refraction passes and the scene target made current again
//! (`00874b50`), and before the first-person arms and weapon, the image-space (HDR) step, the HUD
//! and the menus. It is there in first and third person alike (the game's depth-only clear before
//! the arms is not: a recording has none in third person, measure/newlight/zclears-views.txt).
//!
//! The new lighting (`d3d9_wgpu::scene_light`, off by default) is the one user:
//! `engine_platform::d3d9real::want_scene_light` installs [`install`]'s function, which records the
//! frame's camera for the backend. Not installed (the lighting off): [`world_drawn`] is one load
//! and a branch, and nothing is read or recorded.

use crate::Engine;
use std::sync::OnceLock;

static HOOK: OnceLock<fn(&mut Engine)> = OnceLock::new();

/// Runs `f` at [`world_drawn`] from now on, in every engine of the run (the first call wins).
pub fn install(f: fn(&mut Engine)) {
    let _ = HOOK.set(f);
}

/// The world is drawn (see the module): the installed function, if any.
#[inline]
pub fn world_drawn(e: &mut Engine) {
    if let Some(f) = HOOK.get() {
        f(e);
    }
}
