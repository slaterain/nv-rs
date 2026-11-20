//! The new lighting's switch (`want_scene_light`) puts the platform's hooks on both of the frame's
//! points: where the world is drawn (`engine_core::world_drawn`) and the one that stands in for a
//! skipped depth-only clear (`engine_core::world_drawn::depth_clear_skipped`, JIP's
//! `ToggleDepthClear 0`): its own test binary, so the run-wide hooks start empty.

use engine_core::world_drawn::{depth_clear_skipped_installed, world_drawn_installed};

// A program of its own: no engine crates above the core (it makes no engine).
engine_core::assemble!();

#[test]
fn the_new_lighting_installs_both_points() {
    assert!(!depth_clear_skipped_installed());
    assert!(!world_drawn_installed());
    engine_platform::d3d9real::want_scene_light(|_| None, 1.0);
    assert!(depth_clear_skipped_installed());
    assert!(world_drawn_installed());
    d3d9_wgpu::scene_light::enable(false);
}
