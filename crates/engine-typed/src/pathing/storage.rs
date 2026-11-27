//! Path manager free-list and task-stack base destructors.

use super::FAMILY;
use engine_core::{args, ptr::Ptr, typed_call, Engine};

// Typed from 006ebda0
pub fn free_list_base_destruct(e: &mut Engine, this: Ptr) {
    e.mem.set_u32(this.addr(), 0x0106_d63c);
}

// Typed from 006ebe30
pub fn free_list_base_delete(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    typed_call!(FAMILY, 0x006e_bda0, free_list_base_destruct(e, this));
    if flags & 1 != 0 {
        e.call(0x0040_1030, &args![this]);
    }
    this
}

// Typed from 006ebdc0
pub fn task_stack_base_destruct(e: &mut Engine, this: Ptr) {
    e.mem.set_u32(this.addr(), 0x0106_d644);
}

// Typed from 006ebe60
pub fn task_stack_base_delete(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    typed_call!(FAMILY, 0x006e_bdc0, task_stack_base_destruct(e, this));
    if flags & 1 != 0 {
        e.call(0x0040_1030, &args![this]);
    }
    this
}
