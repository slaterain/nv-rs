//! Pause, resume and state queries on the path manager owner.

use engine_core::{args, ptr::Ptr, Engine};

// Typed from 006ebc70
pub fn pause(e: &mut Engine, owner: Ptr) {
    let manager = e.mem.u32(owner.addr());
    e.call(0x006e_b310, &args![manager]);
}

// Typed from 006ebc90
pub fn is_paused(e: &mut Engine, owner: Ptr) -> bool {
    let manager = e.mem.u32(owner.addr());
    e.call(0x006e_b390, &args![manager]).bool()
}

// Typed from 006ebcb0
pub fn is_running(e: &mut Engine, owner: Ptr) -> bool {
    let manager = e.mem.u32(owner.addr());
    e.call(0x006e_b420, &args![manager]).bool()
}

// Typed from 006ebcd0
pub fn resume(e: &mut Engine, owner: Ptr) {
    let manager = e.mem.u32(owner.addr());
    e.call(0x006e_b3b0, &args![manager]);
}
