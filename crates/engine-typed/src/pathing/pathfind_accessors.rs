//! The small accessors of `pathfind`: the LOS search's ignored-goal, last-valid and closest
//! nodes and its LOS map pointer, the location assignment, the flag setter, and the request
//! family's minimum/maximum distance and max-cost limit words.

use super::layout::PathingLocation;
use super::{location, FAMILY};
use engine_core::ptr::Ptr;
use engine_core::typed::leaf;
use engine_core::x87::x87_quiet;
use engine_core::{typed_call, Engine};

// Typed from 006d1bc0
/// Assigns `location` to the location at `this + 0x34` ([`location::assign`]).
pub fn set_embedded_location(e: &mut Engine, this: Ptr, location_: Ptr) {
    let _: Ptr<PathingLocation> = typed_call!(
        FAMILY,
        0x006d_cce0,
        location::assign(e, Ptr::new(this.addr() + 0x34), location_.cast())
    );
}

// Typed from 006d1be0
/// The word at `+0x2090` of the LOS search (`pFirstIgnoredGoal`).
pub fn first_ignored_goal(e: &mut Engine, this: Ptr) -> u32 {
    e.mem.u32(this.addr() + 0x2090)
}

// Typed from 006d1c00
/// The word at `+0x2094` of the LOS search (`pLastValidNode`).
pub fn last_valid_node(e: &mut Engine, this: Ptr) -> u32 {
    e.mem.u32(this.addr() + 0x2094)
}

// Typed from 006d1c20
/// The word at `+0x2098` of the LOS search (`pClosestNode`).
pub fn closest_node(e: &mut Engine, this: Ptr) -> u32 {
    e.mem.u32(this.addr() + 0x2098)
}

// Typed from 006d1c40
/// The pointer held by the `NiPointer` at `+0x2070` of the LOS search (`spLOSMap`).
pub fn los_map(e: &mut Engine, this: Ptr) -> u32 {
    typed_call!(
        leaf::FAMILY,
        0x0055_9450,
        leaf::word_at_0(e, this.addr() + 0x2070)
    )
}

// Typed from 006d2c00
/// Sets bit 1 (`2`) of the pathing location's flags (`uiFlags`, +0x26).
pub fn location_set_flag_2(e: &mut Engine, this: Ptr<PathingLocation>) {
    let flags = e.get(this, PathingLocation::flags);
    e.set(this, PathingLocation::flags, flags | 2);
}

// Typed from 006d2c20
/// The `float` at `+0xB0` (`fMinFleeDistance` of a flee request, `fMinDistance` of the
/// close-point family), as `FLD` leaves it.
pub fn min_distance(e: &mut Engine, this: Ptr) -> f32 {
    e.mem.f32(this.addr() + 0xb0)
}

// Typed from 006d2c40
/// The `float` at `+0xB4` (`fMaxDistance` of the close-point family), as `FLD` leaves it.
pub fn max_distance(e: &mut Engine, this: Ptr) -> f32 {
    e.mem.f32(this.addr() + 0xb4)
}

// Typed from 006d2c60
/// Stores the `float` at `+0x20A4` of the max-cost search (its cost limit), a signalling NaN
/// quieted as the x87 stack does.
pub fn set_cost_limit(e: &mut Engine, this: Ptr, value: f32) {
    e.mem.set_f32(this.addr() + 0x20a4, x87_quiet(value));
}
