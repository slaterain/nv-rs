//! The sun shadows' switch (`want_sun_shadows`) puts the platform's hook on the frame's point that
//! stands in for a skipped depth-only clear (`engine_core::world_drawn::depth_clear_skipped`, where
//! JIP's `ToggleDepthClear 0` leaves the frame): its own test binary, so the run-wide hooks start
//! empty.

use engine_core::world_drawn::{depth_clear_skipped_installed, world_drawn_installed};

// A program of its own: no engine crates above the core (it makes no engine).
engine_core::assemble!();

#[test]
fn the_sun_shadows_install_the_skipped_clears_point() {
    assert!(!depth_clear_skipped_installed());
    engine_platform::d3d9real::want_sun_shadows(|_| None);
    assert!(depth_clear_skipped_installed());
    // The sun shadows alone do not take the new lighting's point.
    assert!(!world_drawn_installed());
    d3d9_wgpu::sun_shadow::enable(false);
}
