//! Deletion of the path request and ray-cast path owned by pathing task data.

use engine_core::{args, ptr::Ptr, Engine};

// Typed from 006e9f60
pub fn request_delete(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    e.call(0x006c_b410, &args![this]);
    if flags & 1 != 0 {
        e.call(0x0040_1030, &args![this]);
    }
    this
}

// Typed from 006e9f90
pub fn ray_cast_path_delete(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    e.call(0x006c_b4a0, &args![this]);
    if flags & 1 != 0 {
        e.call(0x0040_1030, &args![this]);
    }
    this
}
