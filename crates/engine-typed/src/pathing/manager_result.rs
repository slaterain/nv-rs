//! The path manager's path-data yield flag (`006ea330`) and the two-word completion result's
//! init/copy (`006ea9d0`, `006eaa00`).

use engine_core::ptr::Ptr;
use engine_core::Engine;

// Typed from 006ea330
/// Sets the path-data yield flag (byte at +4).
pub fn set_yield_flag(e: &mut Engine, this: Ptr) {
    e.mem.set_u8(this.addr() + 4, 1);
}

// Typed from 006ea9d0
/// Initializes the two-word completion result used by the path-manager worker: `0xffffffff`,
/// then `0`. Returns `this`.
pub fn completion_init(e: &mut Engine, this: Ptr) -> Ptr {
    e.mem.set_u32(this.addr(), u32::MAX);
    e.mem.set_u32(this.addr() + 4, 0);
    this
}

// Typed from 006eaa00
/// Copies both words of the path-manager completion result from `source`. Returns `this`.
pub fn completion_copy(e: &mut Engine, this: Ptr, source: Ptr) -> Ptr {
    e.mem.set_u32(this.addr(), e.mem.u32(source.addr()));
    e.mem.set_u32(this.addr() + 4, e.mem.u32(source.addr() + 4));
    this
}
