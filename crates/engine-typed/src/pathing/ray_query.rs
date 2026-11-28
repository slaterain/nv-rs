//! Ray-cast queries on the path manager that owns the implementation pointer.

use engine_core::{args, ptr::Ptr, Engine};

// Typed from 006ebc20
pub fn get_ray_path(
    e: &mut Engine,
    owner: Ptr,
    key: u32,
    start: Ptr,
    goal: Ptr,
    output: u32,
) -> i32 {
    let manager = e.mem.u32(owner.addr());
    e.call(0x006e_b2b0, &args![manager, key, start, goal, output])
        .i32()
}

// Typed from 006ebbf0
pub fn get_ray_path_to_navmesh(
    e: &mut Engine,
    owner: Ptr,
    key: u32,
    start: Ptr,
    goal: Ptr,
    output: u32,
    mode: u8,
) -> i32 {
    let manager = e.mem.u32(owner.addr());
    e.call(0x006e_b250, &args![manager, key, start, goal, output, mode])
        .i32()
}
