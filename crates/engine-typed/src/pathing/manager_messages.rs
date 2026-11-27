//! The path manager's small message and flag helpers: the pause-event reset, the cancellation
//! flag, the task-queue enqueue, the named message constructor, the task stop, and the owned
//! implementation's forwarders.

use engine_core::ptr::Ptr;
use engine_core::{args, Engine};

/// Callees.
const EVENT_RESET: u32 = 0x00aa_62c0;
const MESSAGE_CTOR: u32 = 0x0045_d260;
const TASK_STOP: u32 = 0x006e_b890;

/// The impl's pause-event offset.
const PAUSE_EVENT: u32 = 0x0c2c;
/// The impl's task queue offset.
const TASK_QUEUE: u32 = 0x0c10;
/// The impl's cancel-paths flag offset.
const CANCEL_PATHS: u32 = 0x0c34;

// Typed from 006eb7b0
/// Resets the pause event (`00aa62c0` on the event at +0x0c2c).
pub fn reset_pause_event(e: &mut Engine, this: Ptr) {
    e.call(EVENT_RESET, &args![this.addr() + PAUSE_EVENT]);
}

// Typed from 006eb7d0
/// The path cancellation flag byte (+0x0c34).
pub fn cancel_flag(e: &mut Engine, this: Ptr) -> u8 {
    e.mem.u8(this.addr() + CANCEL_PATHS)
}

// Typed from 006eb7f0
/// Enqueues a message on the task queue (vtable slot 4 of the queue at +0x0c10).
pub fn enqueue_message(e: &mut Engine, this: Ptr, message: u32) {
    e.vcall(this.addr() + TASK_QUEUE, 4, &args![message]);
}

// Typed from 006eb820
/// Constructs the named Pathing/BuildPath message (`0045d260` on +4) and clears the state word.
/// Returns `this`.
pub fn build_path_message_new(e: &mut Engine, this: Ptr) -> Ptr {
    e.call(
        MESSAGE_CTOR,
        &args![this.addr() + 4, 0x0103_a1bcu32, 0x0106_d624u32, 0u32, 1u32],
    );
    e.mem.set_u32(this.addr(), 0);
    this
}

// Typed from 006eb850
/// Stops the owned task with flag 1 when non-null.
pub fn stop_task(e: &mut Engine, this: Ptr) {
    let task = e.mem.u32(this.addr());
    if task != 0 {
        e.call(TASK_STOP, &args![task, 1u32]);
    }
}
