//! Route path manager messages to the regular or ray-cast queue.

use super::FAMILY;
use engine_core::{args, ptr::Ptr, typed_call, Engine};

const REGULAR_QUEUE: u32 = 0x1064;
const RAY_CAST_QUEUE: u32 = 0xc50;

// Typed from 006eb200
pub fn enqueue(e: &mut Engine, manager: Ptr, message: u32, mode: u8) -> bool {
    let queue = manager.byte_add(if mode == 0 {
        REGULAR_QUEUE
    } else {
        RAY_CAST_QUEUE
    });
    e.vcall(queue.addr(), 4, &args![message]);
    true
}

// Typed from 006ebbd0
pub fn enqueue_from_owner(e: &mut Engine, owner: Ptr, message: u32, mode: u8) -> bool {
    let manager = Ptr::new(e.mem.u32(owner.addr()));
    typed_call!(FAMILY, 0x006e_b200, enqueue(e, manager, message, mode))
}
