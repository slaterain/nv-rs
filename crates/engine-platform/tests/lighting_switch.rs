//! The new lighting switchable while playing (`want_lighting_switch`, the runs wgpu draws): both of
//! the frame's points installed whatever the start, the switch allowed from the start, and the
//! frame's parts the start's until the player's switch is taken at a `Present`: its own test
//! binary, so the run-wide hooks and switches start empty.

use d3d9_wgpu::scene_light::{self, Parts};
use engine_core::world_drawn::{depth_clear_skipped_installed, world_drawn_installed};

// A program of its own: no engine crates above the core (it makes no engine).
engine_core::assemble!();

#[test]
fn the_switch_installs_both_points_and_takes_effect_at_a_present() {
    assert!(!depth_clear_skipped_installed());
    assert!(!world_drawn_installed());
    assert!(!scene_light::switchable());
    let start = Parts {
        ambient_occlusion: false,
        sun_shadows: true,
    };
    engine_platform::d3d9real::want_lighting_switch(|_| None, start, 1.0);
    assert!(depth_clear_skipped_installed());
    assert!(world_drawn_installed());
    assert!(scene_light::switchable());
    assert_eq!(scene_light::wanted(), start);
    assert_eq!(Parts::now(), start, "the frames start as the run's choice");
    assert_eq!(scene_light::switch_due(), None, "nothing switched yet");
    // The player's key: the frames keep their parts until the next Present takes the switch.
    let next = scene_light::switch_key().expect("allowed");
    assert_eq!(next, start.next());
    assert_eq!(Parts::now(), start, "not before the Present");
    assert_eq!(scene_light::switch_due(), Some(next));
    assert_eq!(Parts::now(), next);
    assert_eq!(scene_light::switch_due(), None, "taken once");
    // Pressed twice before a Present: the frames go to where the presses ended.
    scene_light::switch_key();
    let last = scene_light::switch_key().expect("allowed");
    assert_eq!(last, next.next().next());
    assert_eq!(scene_light::switch_due(), Some(last));
    assert_eq!(Parts::now(), last);
}
