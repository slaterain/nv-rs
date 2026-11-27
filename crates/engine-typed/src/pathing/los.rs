//! The LOS (line-of-sight) search's small functions: the LOS map's flag byte, the position and
//! point copies the request and search use, and the per-triangle LOS score. The search itself
//! (`BuildNodePath`, `GetNodeConnections`, `IsGoalNode`, `SetNodeData`) stays translated.
//!
//! No floats here: the copies move words and the score is a byte, so plain memory reads and
//! writes, with the callees the translations call by address.

use super::copy_point;
use engine_core::ptr::Ptr;
use engine_core::{args, Engine};

/// `005c3420` (unnamed in the engine map; `RET 4`: out): a location's position into `out`.
const LOCATION_POSITION: u32 = 0x005c_3420;
/// `00853130` (unnamed in the engine map; `RET 8`: key, out): looks a key up in the map at `this+8`
/// (AL: found; the value to `out`).
const MAP_LOOKUP: u32 = 0x0085_3130;
/// The score an entry without one gets: `0x7f`.
const NO_SCORE: u8 = 0x7f;

// Typed from 006aabf0
/// A getter (unnamed in the engine map): the byte at +0x39 of `this`.
pub fn flag_byte(e: &mut Engine, this: Ptr) -> u8 {
    e.mem.u8(this.addr() + 0x39)
}

// Typed from 006aabd0
/// Unnamed in the engine map (`RET 4`): the position of the location at +0x1c into `out`;
/// returns `out`.
pub fn location_position(e: &mut Engine, this: Ptr, out: Ptr) -> Ptr {
    e.call(LOCATION_POSITION, &args![this.addr() + 0x1c, out]);
    out
}

// Typed from 006ab150
/// Unnamed in the engine map (`RET 4`): the three words at +0xc8 copied into `out`; returns `out`.
pub fn copy_member_c8(e: &mut Engine, this: Ptr, out: Ptr) -> Ptr {
    copy_point(e, this.byte_add(0xc8).cast(), out.cast());
    out
}

// Typed from 006ab3b0
/// Unnamed in the engine map (`RET 8`): the LOS score of a navmesh key and `triangle`: the key's
/// base (the map lookup at +8) plus the 16-bit `triangle` indexes the byte buffer at +0x18 when
/// below the count at +0x2c (unsigned); otherwise, or for an unknown key, `0x7f`.
pub fn los_score(e: &mut Engine, this: Ptr, key: u32, triangle: u32) -> u8 {
    let base = this.addr();
    e.with_stack(4, |e, found| {
        if e.call(MAP_LOOKUP, &args![base + 8, key, found]).u8() == 0 {
            return NO_SCORE;
        }
        let index = (triangle & 0xffff).wrapping_add(e.mem.u32(found.addr()));
        if index >= e.mem.u32(base + 0x2c) {
            return NO_SCORE;
        }
        let buffer = e.mem.u32(base + 0x18);
        e.mem.u8(index.wrapping_add(buffer))
    })
}
