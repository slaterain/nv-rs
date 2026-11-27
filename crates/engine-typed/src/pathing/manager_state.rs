//! The path manager's state machine accessors: pause, resume, the state predicates, the
//! cancellation-object poke, and the pause event's signal state.

use engine_core::ptr::Ptr;
use engine_core::{args, Engine};

/// Callees.
const LOCK_ENTER: u32 = 0x0045_38a0;
const LOCK_LEAVE: u32 = 0x0045_38c0;
const EVENT_SIGNALLED: u32 = 0x00aa_6120;
const CANCELLATION_COMPLETE: u32 = 0x004f_1540;

/// The impl's state word offset (`PATH_MANAGER_IMPL_STATE`).
const STATE: u32 = 0x0c24;
/// The impl's pause-retries word offset.
const PAUSE_RETRIES: u32 = 0x0c28;
/// The impl's lock offset.
const LOCK: u32 = 0x0c38;
/// The impl's pause-event offset.
const PAUSE_EVENT: u32 = 0x0c2c;

/// The lock guard's entry: `004538a0(lock, tag)`.
fn lock_enter(e: &mut Engine, this: Ptr, tag: u32) {
    e.call(LOCK_ENTER, &args![this.addr() + LOCK, tag]);
}

/// The lock guard's exit: `004538c0(lock)`.
fn lock_leave(e: &mut Engine, this: Ptr) {
    e.call(LOCK_LEAVE, &args![this.addr() + LOCK]);
}

// Typed from 006eb310
/// Pauses the manager: under the lock, 0 becomes 1 (with the pause retries cleared) and 3
/// becomes 2.
pub fn pause(e: &mut Engine, this: Ptr) {
    lock_enter(e, this, 0x0106_d5c8);
    match e.mem.u32(this.addr() + STATE) {
        0 => {
            e.mem.set_u32(this.addr() + STATE, 1);
            e.mem.set_u32(this.addr() + PAUSE_RETRIES, 0);
        }
        3 => e.mem.set_u32(this.addr() + STATE, 2),
        _ => {}
    }
    lock_leave(e, this);
}

// Typed from 006eb390
/// Whether the manager is in paused state 2.
pub fn is_paused(e: &mut Engine, this: Ptr) -> bool {
    e.mem.u32(this.addr() + STATE) == 2
}

// Typed from 006eb3b0
/// Resumes a paused manager: under the lock, 1 becomes 0 and 2 becomes 3.
pub fn resume(e: &mut Engine, this: Ptr) {
    lock_enter(e, this, 0x0106_d5e0);
    match e.mem.u32(this.addr() + STATE) {
        1 => e.mem.set_u32(this.addr() + STATE, 0),
        2 => e.mem.set_u32(this.addr() + STATE, 3),
        _ => {}
    }
    lock_leave(e, this);
}

// Typed from 006eb420
/// Whether the manager is in running state 0.
pub fn is_running(e: &mut Engine, this: Ptr) -> bool {
    e.mem.u32(this.addr() + STATE) == 0
}

// Typed from 006eb670
/// Calls the cancellation object's vtable slot 4 with `action` unless the object is null or
/// already complete (`004f1540`).
pub fn cancel_unless_complete(e: &mut Engine, this: Ptr, action: u8) {
    let cancellation = e.mem.u32(this.addr());
    if cancellation != 0 && !e.call(CANCELLATION_COMPLETE, &args![cancellation]).bool() {
        e.vcall(cancellation, 4, &args![u32::from(action)]);
    }
}

// Typed from 006eb790
/// The manager pause event's signal state (`00aa6120` on the event at +0x0c2c).
pub fn pause_event_signalled(e: &mut Engine, this: Ptr) -> bool {
    e.call(EVENT_SIGNALLED, &args![this.addr() + PAUSE_EVENT])
        .bool()
}
