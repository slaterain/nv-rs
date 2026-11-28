//! The path manager's fixed queue of 256 NiAVObject smart pointers.

use crate::image;
use engine_core::{args, ptr::Ptr, Engine};

image! {
    pub struct StaticQueue / StaticQueueImage: 0x414 {
        0x0 _base: [u8; 8],
        0x8 slots: [u32; 256],
        0x408 count: u32,
        0x40c write: u32,
        0x410 read: u32,
    }
}

fn slot(this: Ptr<StaticQueue>, index: u32) -> u32 {
    this.byte_add(StaticQueue::slots.off).index(index, 4).addr()
}

// Typed from 006ec4b0
pub fn try_push(e: &mut Engine, this: Ptr<StaticQueue>, source: u32) -> bool {
    if e.get(this, StaticQueue::count) >= 256 {
        return false;
    }
    let destination = slot(this, e.get(this, StaticQueue::write));
    let placed = e.call(0x006e_6da0, &args![4u32, destination]).u32();
    if placed != 0 {
        e.call(0x0055_9a40, &args![placed, source]);
    }
    let next = e.get(this, StaticQueue::write).wrapping_add(1);
    e.set(this, StaticQueue::write, next % 256);
    e.set(
        this,
        StaticQueue::count,
        e.get(this, StaticQueue::count).wrapping_add(1),
    );
    true
}

// Typed from 006ec5b0
pub fn try_pop(e: &mut Engine, this: Ptr<StaticQueue>, output: Ptr) -> bool {
    if e.get(this, StaticQueue::count) == 0 {
        return false;
    }
    let source = slot(this, e.get(this, StaticQueue::read));
    e.call(0x006e_5cc0, &args![output, source]);
    e.call(0x0082_28f0, &args![source, 0u32]);
    let next = e.get(this, StaticQueue::read).wrapping_add(1);
    e.set(this, StaticQueue::read, next);
    e.set(this, StaticQueue::read, next % 256);
    e.set(
        this,
        StaticQueue::count,
        e.get(this, StaticQueue::count).wrapping_sub(1),
    );
    true
}
