//! Look up pending ray-cast task data in the path manager's request map.

use engine_core::{args, ptr::Ptr, Engine};

fn with_request(
    e: &mut Engine,
    manager: Ptr,
    key: u32,
    found: impl FnOnce(&mut Engine, u32) -> i32,
) -> i32 {
    e.with_stack(4, |e, slot| {
        e.mem.set_u32(slot.addr(), 0);
        if e.vcall(manager.addr() + 0x1480, 8, &args![key, slot])
            .bool()
        {
            let task = e.mem.u32(slot.addr());
            found(e, task)
        } else {
            2
        }
    })
}

// Typed from 006eb250
pub fn ray_to_navmesh(
    e: &mut Engine,
    manager: Ptr,
    key: u32,
    start: Ptr,
    goal: Ptr,
    output: u32,
    mode: u8,
) -> i32 {
    with_request(e, manager, key, |e, task| {
        e.call(0x006e_a780, &args![task, start, goal, output, mode])
            .i32()
    })
}

// Typed from 006eb2b0
pub fn ray_path(e: &mut Engine, manager: Ptr, key: u32, start: Ptr, goal: Ptr, output: u32) -> i32 {
    with_request(e, manager, key, |e, task| {
        e.call(0x006e_a510, &args![task, start, goal, output]).i32()
    })
}
