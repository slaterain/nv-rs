//! A port-added point in the frame's draw (not in the original): the world is in the scene target
//! and nothing after it is yet. The world frames (`00870bd0`, and `00870a00`, its variant with the
//! shadow scene node's byte +0x131 set) reach it right after their call of the world pass
//! (`00873200`: the opaque and alpha-tested world, the player's body in third person, the sky),
//! before the water and refraction passes and the rest of the draw (`00875110`: the first-person
//! arms and weapon), the image-space (HDR) step, the HUD and the menus. It is there in first and
//! third person alike: in third person the first-person node is culled and the frame skips the rest
//! of the draw, and the game's depth-only clear before the arms with it (a recording has none in
//! third person, measure/newlight/zclears-views.txt).
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

static DEPTH_CLEAR_SKIPPED: OnceLock<fn(&mut Engine)> = OnceLock::new();

/// Runs `f` at [`depth_clear_skipped`] from now on, in every engine of the run (the first call
/// wins).
pub fn install_depth_clear_skipped(f: fn(&mut Engine)) {
    let _ = DEPTH_CLEAR_SKIPPED.set(f);
}

/// A second port-added point: the game's depth-only clear before the first-person arms (the
/// rest of the draw's `0x7148C0` call at `0x8751C6`) is skipped, as JIP's `ToggleDepthClear 0`
/// asks. The clear is where the platform marks the world as drawn for the new lighting and the
/// sun shadows (the frame's camera and sun recorded; draws after it neither cast nor take the
/// sun's shadows), so the frame calls this in its place and the mark stays. Not installed (the
/// lighting and the shadows off): one load and a branch.
#[inline]
pub fn depth_clear_skipped(e: &mut Engine) {
    if let Some(f) = DEPTH_CLEAR_SKIPPED.get() {
        f(e);
    }
}
