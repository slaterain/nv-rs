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
//!   row), its frustum at +0xdc (left, right, top, bottom, near, far as floats, the orthographic
//!   byte at +0xf4);
//! - `TES` `*011dea10`: the sun light (`NiDirectionalLight`) at +0x1c, its direction at +0xf0; the
//!   current interior cell at +0x34 (none outdoors);
//! - `Sky` `*011dea20`: the sun light's colour (`sunDirectional`) at +0x6c.

use engine_platform::d3d9real::SceneView;
use engine_platform::Engine;

const SCENE_GRAPH: u32 = 0x011d_eb7c;
const TES: u32 = 0x011d_ea10;
const SKY: u32 = 0x011d_ea20;
const CAMERA: u32 = 0xac;
const ROTATE: u32 = 0x68;
const FRUSTUM: u32 = 0xdc;
const ORTHO: u32 = 0xf4;
const SUN_LIGHT: u32 = 0x1c;
const LIGHT_DIRECTION: u32 = 0xf0;
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

/// The frame's camera and sun; none before the game has its world camera (the main menu).
pub fn read(e: &Engine) -> Option<SceneView> {
    let tes = object(e, TES)?;
    let camera = object(e, object(e, SCENE_GRAPH)? + CAMERA)?;
    let sun_direction = object(e, tes + SUN_LIGHT).map(|l| floats(e, l + LIGHT_DIRECTION));
    let sun_colour = object(e, SKY).map_or([0.0; 3], |s| floats(e, s + SUN_COLOUR));
    Some(SceneView {
        frustum: floats(e, camera + FRUSTUM),
        ortho: e.mem.u8(camera + ORTHO) != 0,
        rotate: floats(e, camera + ROTATE),
        sun_direction,
        sun_colour,
        interior: e.mem.u32(tes + INTERIOR) != 0,
    })
}
