//! The pathing tasklet's small message handlers: constructing and copying the pathing closure
//! messages, and the task data's cancel flag setter.

use engine_core::ptr::Ptr;
use engine_core::{args, Engine};

/// Callees.
const POINTER_ASSIGN: u32 = 0x0055_9a40;
const POINTER_ASSIGN_FROM: u32 = 0x006e_5cc0;

// Typed from 006e9c00
/// Constructs a message from a type and a solution pointer: the type at +0, the reference-counted
/// pointer assigned at +4 (`00559a40`). Returns `this`.
pub fn message_new(e: &mut Engine, this: Ptr, message_type: u32, solution: u32) -> Ptr {
    e.mem.set_u32(this.addr(), message_type);
    e.call(POINTER_ASSIGN, &args![this.addr() + 4, solution]);
    this
}

// Typed from 006e9c30
/// Copies the type and the reference-counted pointer in a small pathing closure: the type word,
/// then `00559a40` on the +4 members. Returns `this`.
pub fn message_copy(e: &mut Engine, this: Ptr, source: Ptr) -> Ptr {
    e.mem.set_u32(this.addr(), e.mem.u32(source.addr()));
    e.call(POINTER_ASSIGN, &args![this.addr() + 4, source.addr() + 4]);
    this
}

// Typed from 006e9c60
/// Copies the type and the `NiPointer<PathingDebugGeometryData>` member (`006e5cc0` on the +4
/// members). Returns `this`.
pub fn debug_data_copy(e: &mut Engine, this: Ptr, source: Ptr) -> Ptr {
    e.mem.set_u32(this.addr(), e.mem.u32(source.addr()));
    e.call(
        POINTER_ASSIGN_FROM,
        &args![this.addr() + 4, source.addr() + 4],
    );
    this
}

// Typed from 006ea330
/// Sets the byte at +4 (the task data's cancelled flag) to 1.
pub fn task_data_set_cancelled(e: &mut Engine, this: Ptr) {
    e.mem.set_u8(this.addr() + 4, 1);
}
