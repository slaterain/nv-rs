//! The path manager's task-data scalar delete (`006e9c90`), pause-event accessors and message
//! queue (`006eb790`, `006eb7b0`, `006eb7d0`, `006eb7f0`) and the message/task lifecycle
//! (`006eb820`, `006eb850`).

use engine_core::ptr::Ptr;
use engine_core::{args, Engine};

/// Callees.
const TASK_DATA_DTOR: u32 = 0x006e_9e20;
const OPERATOR_DELETE: u32 = 0x0040_1030;
const EVENT_SIGNALLED: u32 = 0x00aa_6120;
const EVENT_SIGNAL: u32 = 0x00aa_62c0;
const MESSAGE_CTOR: u32 = 0x0045_d260;
const STOP_TASK: u32 = 0x006e_b890;

/// The impl's offsets (`PATH_MANAGER_IMPL_*`).
const PAUSE_EVENT: u32 = 0x0c2c;
const CANCEL_PATHS: u32 = 0x0c34;
/// The task queue member's offset and its enqueue vtable slot (byte offset).
const TASK_QUEUE: u32 = 0x0c10;

/// The build-path message's class id and vtable.
const MESSAGE_CLASS: u32 = 0x0103_a1bc;
const MESSAGE_VTABLE: u32 = 0x0106_d624;

// Typed from 006e9c90
/// Destroys the pathing task data (`006e9e20`), then frees it only when the scalar delete flag
/// (bit 0 of `flags`) is set. Returns `this`.
pub fn task_data_scalar_delete(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    e.call(TASK_DATA_DTOR, &args![this]);
    if flags & 1 != 0 {
        e.call(OPERATOR_DELETE, &args![this]);
    }
    this
}

// Typed from 006eb790
/// Whether the manager's pause event is signalled.
pub fn pause_event_signalled(e: &mut Engine, this: Ptr) -> bool {
    e.call(EVENT_SIGNALLED, &args![this.addr() + PAUSE_EVENT])
        .bool()
}

// Typed from 006eb7b0
/// Signals the manager's pause event.
pub fn signal_pause_event(e: &mut Engine, this: Ptr) {
    e.call(EVENT_SIGNAL, &args![this.addr() + PAUSE_EVENT]);
}

// Typed from 006eb7d0
/// Returns the path cancellation flag.
pub fn cancel_paths_flag(e: &mut Engine, this: Ptr) -> u8 {
    e.mem.u8(this.addr() + CANCEL_PATHS)
}

// Typed from 006eb7f0
/// Enqueues a message on the path manager's task queue (vtable slot 4 of the queue at +0x0c10).
pub fn enqueue_message(e: &mut Engine, this: Ptr, message: u32) {
    e.vcall(this.addr() + TASK_QUEUE, 4, &args![message]);
}

// Typed from 006eb820
/// Constructs the named Pathing/BuildPath message in the object and clears its state word.
/// Returns `this`.
pub fn construct_build_path_message(e: &mut Engine, this: Ptr) -> Ptr {
    e.call(
        MESSAGE_CTOR,
        &args![this.addr() + 4, MESSAGE_CLASS, MESSAGE_VTABLE, 0u32, 1u32],
    );
    e.mem.set_u32(this.addr(), 0);
    this
}

// Typed from 006eb850
/// Stops the owned task with flag 1 when the task word is not null.
pub fn stop_owned_task(e: &mut Engine, this: Ptr) {
    let task = e.mem.u32(this.addr());
    if task != 0 {
        e.call(STOP_TASK, &args![task, 1u32]);
    }
}
