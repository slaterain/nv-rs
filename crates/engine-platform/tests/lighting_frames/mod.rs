//! What a frame records for the new lighting, as the platform records it for the device (the
//! integration binaries `lighting_before_switch` and `lighting_fresh_start`): a frame's four points
//! (the scene's colour and depth clear, the frame draw's world-drawn point, the depth-only clear
//! before the first-person arms, and the point in its place when JIP skips that clear), each with
//! the port's `SetTransform` words recorded there by name; and what each of the four Settings
//! states recorded before the player could switch the new lighting.

#![allow(dead_code)]

use d3d9_stream::{Cmd, Data, Reply};
use d3d9_wgpu::scene_light::{self, Parts};
use d3d9_wgpu::sun_shadow;
use engine_platform::d3d9real::{self, SceneView};
use engine_platform::Engine;
use std::sync::{Arc, Mutex};

pub const DEVICE: u32 = 0x1000;

/// An exterior camera with the sun up (so the sun shadows' words are recorded too).
pub fn reader(_: &Engine) -> Option<SceneView> {
    Some(SceneView {
        frustum: [-0.7, 0.7, 0.4, -0.4, 5.0, 5000.0],
        ortho: false,
        rotate: [0.0, 0.0, 1.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0],
        position: [100.0, 200.0, 300.0],
        to_sun: Some([0.3, 0.2, 0.9]),
        sun_colour: [1.0, 0.9, 0.8],
        interior: false,
        world: 0x1234,
    })
}

/// What a [`Words`] backend recorded.
pub type Seen = Arc<Mutex<Vec<String>>>;

/// A backend that keeps each command's name, and a `SetTransform`'s port state by name.
struct Words(Seen);

/// The port's `SetTransform` states by name.
fn state_name(state: u32) -> String {
    match state {
        s if s == scene_light::STATE => "camera (NVAO)".into(),
        s if s == sun_shadow::STATE => "sun frame (depth clear)".into(),
        s if s == sun_shadow::VIEW_STATE => "sun camera (scene clear)".into(),
        s if s == sun_shadow::BEGIN_STATE => "sun frame (scene clear)".into(),
        s if s == sun_shadow::WORLD_STATE => "sun world (scene clear)".into(),
        s if s == scene_light::SWITCH_STATE => "switch (NVLS)".into(),
        s => format!("SetTransform {s:#x}"),
    }
}

impl d3d9_stream::Backend for Words {
    fn execute(&mut self, cmd: Cmd<Data<'_>>) -> Reply {
        let seen = match &cmd {
            Cmd::SetTransform { state, .. } => state_name(*state),
            Cmd::Clear { flags, .. } => format!("Clear {flags}"),
            _ => cmd.name().to_string(),
        };
        self.0.lock().unwrap().push(seen);
        Reply::None
    }
}

/// An engine whose game device records into a [`Words`] backend, and what it recorded.
pub fn engine() -> (Engine, Seen) {
    let seen: Seen = Arc::default();
    let mut e = Engine::new();
    d3d9real::record_into_for_test(&mut e, DEVICE, Box::new(Words(Arc::clone(&seen))));
    (e, seen)
}

/// One frame's four points ([the module](self)), what each recorded.
pub fn frame(e: &mut Engine, seen: &Seen) -> [Vec<String>; 4] {
    let take = || std::mem::take(&mut *seen.lock().unwrap());
    take();
    d3d9real::clear(e, DEVICE, 0, 0, 3, 0, 0, 0, 0);
    let scene_clear = take();
    engine_core::world_drawn::world_drawn(e);
    let world_drawn = take();
    d3d9real::clear(e, DEVICE, 0, 0, 2, 0, 0, 0, 0);
    let depth_clear = take();
    engine_core::world_drawn::depth_clear_skipped(e);
    let skipped = take();
    [scene_clear, world_drawn, depth_clear, skipped]
}

/// What a frame recorded at its four points with the Settings `parts` (`bAmbientOcclusion`,
/// `bSunShadows`) before the player could switch the new lighting (merge base 79900e075): with the
/// ambient occlusion on, `want_scene_light` installed the world-drawn point and the depth-only
/// clear's point and recorded the camera at both and at the depth-only clear; with the sun shadows
/// on, `want_sun_shadows` installed the depth-only clear's point, and the scene's clear recorded
/// the sun's world, camera and frame, the depth-only clear (and its point) the camera and then the
/// sun's frame; with neither, nothing was installed and the clears recorded themselves alone.
pub fn before_the_switch(parts: Parts) -> [Vec<String>; 4] {
    let (ao, sun) = (parts.ambient_occlusion, parts.sun_shadows);
    let mut scene_clear: Vec<String> = Vec::new();
    if sun {
        scene_clear.extend(
            [
                "sun world (scene clear)",
                "sun camera (scene clear)",
                "sun frame (scene clear)",
            ]
            .map(String::from),
        );
    }
    scene_clear.push("Clear 3".into());
    let world_drawn: Vec<String> = if ao {
        vec!["camera (NVAO)".into()]
    } else {
        Vec::new()
    };
    let mut marks: Vec<String> = Vec::new();
    if ao || sun {
        marks.push("camera (NVAO)".into());
    }
    if sun {
        marks.push("sun frame (depth clear)".into());
    }
    let mut depth_clear = marks.clone();
    depth_clear.push("Clear 2".into());
    [scene_clear, world_drawn, depth_clear, marks]
}
