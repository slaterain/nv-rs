//! `UpdateMiniMap` on part 1's stand-in HUD, every expectation from JIP's
//! `Cmd_UpdateMiniMap_Execute` and the helpers it calls (functions_jip/jip_fn_minimap.h).

use super::*;

/// Door markers inside the local map only for 0 <= x < 1 and 0 <= y < 1 (a NaN outside).
#[test]
fn door_markers_outside_the_map_are_left_out() {
    assert!(!off_bounds([0.0, 0.0]));
    assert!(!off_bounds([0.999, 0.5]));
    assert!(off_bounds([1.0, 0.5]));
    assert!(off_bounds([0.5, 1.0]));
    assert!(off_bounds([-0.001, 0.5]));
    assert!(off_bounds([0.5, -0.001]));
    assert!(off_bounds([f32::NAN, 0.5]));
    assert!(!off_bounds([-0.0, 0.5]), "-0 is not below 0");
}
