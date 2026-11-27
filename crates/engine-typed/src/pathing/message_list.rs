//! Constructors for the path manager’s message queue bases and free list.

use super::layout::PathFreeList;
use engine_core::{ptr::Ptr, Engine};

// Typed from 006ecc00
pub fn free_list_item_base_new(e: &mut Engine, this: Ptr) -> Ptr {
    e.mem.set_u32(this.addr(), 0x0106_d6b8);
    this
}

// Typed from 006ecc20
pub fn list_node_base_new(e: &mut Engine, this: Ptr) -> Ptr {
    e.mem.set_u32(this.addr(), 0x0106_d710);
    this
}

// Typed from 006ecc40
pub fn free_list_new(e: &mut Engine, this: Ptr<PathFreeList>) -> Ptr<PathFreeList> {
    e.set(this, PathFreeList::vtable, 0x0106_d63c);
    e.set(this, PathFreeList::lock, 0);
    e.set(this, PathFreeList::head, 0);
    this
}
