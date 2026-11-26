//! The set-ups of a run where the player cannot switch the new lighting (`want_scene_light`,
//! `want_sun_shadows`: the real Direct3D 9, or no backend) record in a frame, for each Settings
//! state, what that state recorded before the switch existed ([`lighting_frames::before_the_switch`]).
//! `lighting_fresh_start` checks a run where the player can switch it against the same table. Its
//! own test binary: those set-ups install the frame's points for the whole run, so the states go
//! in the order the points come: neither, the sun shadows alone, both, the ambient occlusion alone.

mod lighting_frames;

use d3d9_wgpu::scene_light::Parts;
use engine_platform::d3d9real::{want_scene_light, want_sun_shadows};
use lighting_frames::{before_the_switch, engine, frame, reader};

// A program of its own: the platform layer and no engine crates above it (its engines have the
// platform's device state).
engine_core::assemble!(engine_platform::units::funcs);

#[test]
fn the_set_ups_without_the_switch_record_what_they_did_before_it() {
    let (mut e, seen) = engine();
    let [off, all, sun, ao] = Parts::EVERY;
    assert_eq!(frame(&mut e, &seen), before_the_switch(off), "neither");
    // The sun shadows alone: the host's `want_sun_shadows` only.
    want_sun_shadows(reader);
    assert_eq!(
        frame(&mut e, &seen),
        before_the_switch(sun),
        "the sun shadows"
    );
    // Both: `want_scene_light` too.
    want_scene_light(reader, 1.0);
    assert_eq!(frame(&mut e, &seen), before_the_switch(all), "both");
    // The ambient occlusion alone (`want_scene_light`, which installs both points, alone).
    d3d9_wgpu::sun_shadow::enable(false);
    assert_eq!(
        frame(&mut e, &seen),
        before_the_switch(ao),
        "the ambient occlusion"
    );
    d3d9_wgpu::scene_light::enable(false);
}
