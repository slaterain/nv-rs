//! The path manager's linked and static message queue constructors.

use crate::image;
use engine_core::{args, ptr::Ptr, Engine};

image! {
    pub struct LinkedQueue / LinkedQueueImage: 0x8 {
        0x0 vtable: u32,
        0x4 head: u32,
    }

    pub struct StaticQueue / StaticQueueImage: 0x8 {
        0x0 vtable: u32,
        0x4 count: u32,
    }
}

// Typed from 006ecb40
pub fn linked_queue_new(e: &mut Engine, this: Ptr<LinkedQueue>) -> Ptr<LinkedQueue> {
    e.call(0x006e_cc00, &args![this]);
    e.set(this, LinkedQueue::vtable, 0x0106_d698);
    e.set(this, LinkedQueue::head, 0);
    this
}

// Typed from 006ecb70
pub fn static_queue_new(e: &mut Engine, this: Ptr<StaticQueue>) -> Ptr<StaticQueue> {
    e.call(0x006e_cc20, &args![this]);
    e.set(this, StaticQueue::vtable, 0x0106_d6f0);
    e.set(this, StaticQueue::count, 0);
    this
}
