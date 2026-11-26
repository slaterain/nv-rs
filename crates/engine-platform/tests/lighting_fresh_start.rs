//! A run where the player can switch the new lighting (`want_lighting_switch`: every wgpu run,
//! whatever Settings says) records in a frame, for each Settings state it starts in, exactly what
//! that state recorded before the switch existed ([`lighting_frames::before_the_switch`], which
//! `lighting_before_switch` checks the set-ups without the switch against), and after a switch at
//! a `Present` what a fresh start in the state switched to records, the switch's word right after
//! the `Present`. Its own test binary: the set-up installs the frame's points for the whole run.

mod lighting_frames;

use d3d9_wgpu::scene_light::{self, Parts};
use engine_platform::d3d9real::{self, want_lighting_switch};
use lighting_frames::{before_the_switch, engine, frame, reader, DEVICE};

// A program of its own: the platform layer and no engine crates above it (its engines have the
// platform's device state).
engine_core::assemble!(engine_platform::units::funcs);

#[test]
fn a_fresh_start_in_each_state_records_what_it_did_before_the_switch() {
    let (mut e, seen) = engine();
    for start in Parts::EVERY {
        want_lighting_switch(reader, start, 1.0);
        assert_eq!(
            frame(&mut e, &seen),
            before_the_switch(start),
            "a fresh start with {start:?}"
        );
    }
    // Switched while playing, round twice from neither: until the Present the frame records what
    // its state did; the switch's word right after the Present; from then on, the next state's.
    want_lighting_switch(reader, Parts::default(), 1.0);
    let mut parts = Parts::default();
    for _ in 0..8 {
        let next = scene_light::switch_key().expect("allowed");
        assert_eq!(
            frame(&mut e, &seen),
            before_the_switch(parts),
            "{parts:?}, switched"
        );
        d3d9real::present(&mut e, DEVICE, 0, 0, 0, 0, 0);
        let got = std::mem::take(&mut *seen.lock().unwrap());
        assert_eq!(got, ["Present", "switch (NVLS)"], "to {next:?}");
        parts = next;
        assert_eq!(
            frame(&mut e, &seen),
            before_the_switch(parts),
            "{parts:?}, switched to"
        );
    }
    scene_light::forbid_switch();
    scene_light::enable(false);
    d3d9_wgpu::sun_shadow::enable(false);
}
