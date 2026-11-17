//! The new lighting's view of the game (port-added, off by default: `[nv-rs Display]
//! bAmbientOcclusion`, `NV_AO`; [`crate::fixes::Choice::ambient_occlusion`]): the world camera's
//! frustum and rotation, the sun light's direction and colour, and whether the cell is an
//! interior, read from the game's memory at the depth-only clear before the first-person arms
//! (`engine_platform::d3d9real::want_scene_light`; the step itself is `d3d9_wgpu::scene_light`).
//! Nothing here changes the game.
//!
//! Where they are (layouts from JIP LN NVSE's `netimmerse.h` and `GameData.h`, and Depth Resolve's
//! `TESMain`/`BSSceneGraph`/`NiCamera`; the scene graph's word is written in `InitSceneGraph`):
//! - the world scene graph `*011deb7c`, its camera at +0xac (`BSSceneGraph::spCamera`);
//! - the camera (`NiCamera`): its world rotation at +0x68 (`NiAVObject::m_kWorld`, 3x3 floats row by
//!   row), its place in the world just after (+0x8c, the translation; for the sun shadows,
//!   `d3d9_wgpu::sun_shadow`), its frustum at +0xdc (left, right, top, bottom, near, far as
//!   floats, the orthographic byte at +0xf4);
//! - `TES` `*011dea10`: the current interior cell at +0x34 (none outdoors);
//! - `Sky` `*011dea20`: the sun light's colour (`sunDirectional`) at +0x6c; its `Sun` at +0x28, whose
//!   directional light (+0x1c, the `NiDirectionalLight` the lit shaders take as their light 0) the
//!   sun's update (`Sun::Update`, 00641830, `engine-fallout-env` `sun.rs`) turns: its world rotation's
//!   column 0 (+0x68, row by row) is the way the light goes, `(-height, distance, light height)`
//!   made unit; the way toward the light is its opposite ([`toward_light`]). That, not the sun
//!   disc's place, is what lights the world: the disc (the root node +0x04, local translation +0x58)
//!   is placed from another vector, and the two were measured 33 degrees apart at 09:00, 56 at noon
//!   and 27 at 17:30 (run p-onL1, 2026-11-17), so shadows drawn from the disc would not line up
//!   with the lit sides. At night the same light shines with the weather's night colour (there is no
//!   separate moon light), so the shadows follow it then too, as faint as that colour is. The disc
//!   is the fallback when the light is not there.

use engine_platform::d3d9real::SceneView;
use engine_platform::Engine;

const SCENE_GRAPH: u32 = 0x011d_eb7c;
const TES: u32 = 0x011d_ea10;
const SKY: u32 = 0x011d_ea20;
const CAMERA: u32 = 0xac;
const ROTATE: u32 = 0x68;
const TRANSLATE: u32 = 0x8c;
const FRUSTUM: u32 = 0xdc;
const ORTHO: u32 = 0xf4;
const SUN: u32 = 0x28;
const SUN_ROOT: u32 = 0x04;
const LOCAL_TRANSLATE: u32 = 0x58;
const INTERIOR: u32 = 0x34;
const SUN_COLOUR: u32 = 0x6c;

/// The object the word at `at` points to, when it is in the game's memory.
fn object(e: &Engine, at: u32) -> Option<u32> {
    let p = e.mem.u32(at);
    (p != 0 && e.mem.is_mapped(p)).then_some(p)
}

fn floats<const N: usize>(e: &Engine, at: u32) -> [f32; N] {
    std::array::from_fn(|i| e.mem.f32(at + 4 * i as u32))
}

/// The sun's directional light (`Sun` +0x1c, the `NiDirectionalLight` its update turns).
const SUN_LIGHT: u32 = 0x1c;
/// A node's world translation (+0x8c, after its world rotation at +0x68).
const WORLD_TRANSLATE: u32 = 0x8c;

/// A tool (`NV_SUN_SHADOWS_TRACE=N`, the sun shadows' trace): for the next N reads, the sun disc's
/// local and world place and the sun light's world rotation, in the log, to compare the way the
/// lit shaders' light points with the disc's place, by day and by night. Nothing changes.
fn trace_light(e: &Engine, sky: Option<u32>) {
    use std::sync::atomic::{AtomicI64, Ordering};
    static LEFT: AtomicI64 = AtomicI64::new(-1);
    if LEFT.load(Ordering::Relaxed) < 0 {
        let n = std::env::var("NV_SUN_SHADOWS_TRACE")
            .ok()
            .and_then(|v| v.trim().parse::<i64>().ok())
            .unwrap_or(0);
        LEFT.store(n, Ordering::Relaxed);
    }
    if LEFT.load(Ordering::Relaxed) <= 0 {
        return;
    }
    let Some(sun) = sky.and_then(|s| object(e, s + SUN)) else {
        return;
    };
    let root = object(e, sun + SUN_ROOT);
    let light = object(e, sun + SUN_LIGHT);
    let local: Option<[f32; 3]> = root.map(|n| floats(e, n + LOCAL_TRANSLATE));
    // Indoors (and before the sky is placed) the disc stays at the origin: not counted.
    if local.map_or(true, |p| p == [0.0; 3]) {
        return;
    }
    LEFT.fetch_sub(1, Ordering::Relaxed);
    let world: Option<[f32; 3]> = root.map(|n| floats(e, n + WORLD_TRANSLATE));
    let rotate: Option<[f32; 9]> = light.map(|l| floats(e, l + ROTATE));
    eprintln!(
        "nv-rs: sun light trace: disc local {local:.4?} world {world:.1?}; light world rotation {rotate:.4?}"
    );
}

/// The way toward a directional light from its world rotation (row by row): the opposite of
/// column 0, the way it shines, made unit; none when that is not a direction.
pub fn toward_light(rotate: [f32; 9]) -> Option<[f32; 3]> {
    let d = [-rotate[0], -rotate[3], -rotate[6]];
    let len = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt();
    (len.is_finite() && len > 1e-6).then(|| d.map(|v| v / len))
}

/// The way toward the sun the new lighting is given: with the sun shadows on, the sun light's
/// (`light`), the disc's place (`disc`) as the fallback; with them off, the disc's place alone, as
/// before the sun shadows came (the ambient occlusion's contact shadows look as they did).
fn pick_to_sun(
    sun_shadows: bool,
    light: impl FnOnce() -> Option<[f32; 3]>,
    disc: impl FnOnce() -> Option<[f32; 3]>,
) -> Option<[f32; 3]> {
    if sun_shadows {
        light().or_else(disc)
    } else {
        disc()
    }
}

/// The frame's camera and sun (the sun disc's place); none before the game has its world camera
/// (the main menu). The reader with the sun shadows off.
pub fn read(e: &Engine) -> Option<SceneView> {
    read_with(e, false)
}

/// [`read`] with the sun shadows on: the sun is the way toward the sun light ([`toward_light`]).
pub fn read_lit(e: &Engine) -> Option<SceneView> {
    read_with(e, true)
}

fn read_with(e: &Engine, sun_shadows: bool) -> Option<SceneView> {
    let tes = object(e, TES)?;
    let camera = object(e, object(e, SCENE_GRAPH)? + CAMERA)?;
    let sky = object(e, SKY);
    let sun = sky.and_then(|s| object(e, s + SUN));
    let to_sun = pick_to_sun(
        sun_shadows,
        || {
            sun.and_then(|s| object(e, s + SUN_LIGHT))
                .and_then(|l| toward_light(floats(e, l + ROTATE)))
        },
        || {
            sun.and_then(|s| object(e, s + SUN_ROOT))
                .map(|n| floats(e, n + LOCAL_TRANSLATE))
        },
    );
    let sun_colour = sky.map_or([0.0; 3], |s| floats(e, s + SUN_COLOUR));
    trace_light(e, sky);
    Some(SceneView {
        frustum: floats(e, camera + FRUSTUM),
        ortho: e.mem.u8(camera + ORTHO) != 0,
        rotate: floats(e, camera + ROTATE),
        position: floats(e, camera + TRANSLATE),
        to_sun,
        sun_colour,
        interior: e.mem.u32(tes + INTERIOR) != 0,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The sun light's world rotation as the game left it (run p-onL1, 2026-11-17, the trace):
    /// column 0 `(-height, distance, light height)` made unit, the rest the identity's.
    fn traced(x: f32, yz: f32) -> [f32; 9] {
        [x, 0.0, 0.0, yz, 1.0, 0.0, yz, 0.0, 1.0]
    }

    /// With the sun shadows off the new lighting keeps the sun disc's place (its ambient
    /// occlusion's look does not change); on, the sun light's way, the disc only without it.
    #[test]
    fn the_sun_light_is_taken_only_with_the_sun_shadows_on() {
        let (light, disc) = ([1.0, 0.0, 0.0], [0.0, 0.6, 0.8]);
        assert_eq!(
            pick_to_sun(false, || Some(light), || Some(disc)),
            Some(disc)
        );
        assert_eq!(pick_to_sun(false, || Some(light), || None), None);
        assert_eq!(
            pick_to_sun(true, || Some(light), || Some(disc)),
            Some(light)
        );
        assert_eq!(pick_to_sun(true, || None, || Some(disc)), Some(disc));
    }

    #[test]
    fn the_light_shines_from_where_its_column_0_points_away_from() {
        // 17:30: the light goes east (+x), south and down: it comes from the west, north and above,
        // 11 degrees up (the lit sides face west, as the pictures show).
        let at_dusk = toward_light(traced(0.9644, -0.1869)).unwrap();
        assert!((at_dusk[0] + 0.9644).abs() < 1e-3, "{at_dusk:?}");
        assert!((at_dusk[1] - 0.1869).abs() < 1e-3 && (at_dusk[2] - 0.1869).abs() < 1e-3);
        // 09:00: from the east; noon: from the north-east, 33 degrees up (the light's height
        // setting keeps it under 45 degrees all day).
        let morning = toward_light(traced(-0.9553, -0.2091)).unwrap();
        assert!(morning[0] > 0.9 && morning[2] > 0.0);
        let noon = toward_light(traced(-0.6273, -0.5507)).unwrap();
        assert!((noon[2].asin().to_degrees() - 33.4).abs() < 0.5, "{noon:?}");
        let n = noon.iter().map(|v| v * v).sum::<f32>().sqrt();
        assert!((n - 1.0).abs() < 1e-6);
        assert_eq!(toward_light([0.0; 9]), None);
        assert_eq!(toward_light([f32::NAN; 9]), None);
    }
}
