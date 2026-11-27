//! The `PathBuilder`'s virtual-node appenders (`006c9920`, `006c99c0`, `006c9a70`), the
//! virtual node's door-flag test (`006ca0c0`) and the request's heading accessors (`006ca4c0`,
//! `006ca4e0`).

use engine_core::ptr::Ptr;
use engine_core::x87::x87_quiet_bits;
use engine_core::{args, Engine};

/// Callees.
const VIRTUAL_NODE_ARRAY_COUNT: u32 = 0x006c_ef10;
const VIRTUAL_NODE_SLOT: u32 = 0x006e_8f40;
const PLACEMENT_NEW: u32 = 0x006e_6da0;
const VIRTUAL_NODE_CONSTRUCT_INFO: u32 = 0x006f_4e40;
const VIRTUAL_NODE_CONSTRUCT_POINT_INFO: u32 = 0x006f_4f90;
const VIRTUAL_NODE_CONSTRUCT_INFO_POINT_DOOR: u32 = 0x006f_4f00;

/// The `PathingSolution`'s `VirtualNodes` array offset.
const VIRTUAL_NODES: u32 = 0x08;
/// The `VirtualNode`'s size (the placement-new argument).
const VIRTUAL_NODE_SIZE: u32 = 0x30;

/// The request's `bFirstTangentUsesHeading` and `fInitialPathHeading` offsets.
const B_FIRST_TANGENT_USES_HEADING: u32 = 0xa0;
const F_INITIAL_PATH_HEADING: u32 = 0x90;

/// Appends a virtual node built by `construct` from `words`: the array's size is the index, the
/// element slot is placement-new'd (`0x30` bytes) and the constructor runs on the placement.
fn add_virtual_node(e: &mut Engine, solution: u32, construct: u32, words: &[u32]) -> u32 {
    let nodes = solution + VIRTUAL_NODES;
    let index = e.call(VIRTUAL_NODE_ARRAY_COUNT, &args![nodes]).u32();
    let slot = e.call(VIRTUAL_NODE_SLOT, &args![nodes, index]).u32();
    let place = e.call(PLACEMENT_NEW, &args![VIRTUAL_NODE_SIZE, slot]).u32();
    if place != 0 {
        let mut arguments = vec![place];
        arguments.extend_from_slice(words);
        e.call(construct, &arguments);
    }
    index
}

// Typed from 006c9920
/// Thiscall on a `PathingSolution`: appends a virtual node built by `006f4e40` from `info` (a
/// navmesh info); returns its index.
pub fn add_virtual_node_info(e: &mut Engine, this: u32, info: u32) -> u32 {
    add_virtual_node(e, this, VIRTUAL_NODE_CONSTRUCT_INFO, &[info])
}

// Typed from 006c99c0
/// Thiscall on a `PathingSolution`: appends a virtual node built by `006f4f90` from `point` and
/// `info`; returns its index.
pub fn add_virtual_node_point_info(e: &mut Engine, this: u32, point: u32, info: u32) -> u32 {
    add_virtual_node(e, this, VIRTUAL_NODE_CONSTRUCT_POINT_INFO, &[point, info])
}

// Typed from 006c9a70
/// Thiscall on a `PathingSolution`: appends a virtual node built by `006f4f00` from `info`,
/// `point` and `door`; returns its index.
pub fn add_virtual_node_info_point_door(
    e: &mut Engine,
    this: u32,
    info: u32,
    point: u32,
    door: u32,
) -> u32 {
    add_virtual_node(
        e,
        this,
        VIRTUAL_NODE_CONSTRUCT_INFO_POINT_DOOR,
        &[info, point, door],
    )
}

// Typed from 006ca0c0
/// Whether bit 2 (`4`) of the virtual node's `iFlags` (Xbox PDB, +0) is set (the door flag).
pub fn virtual_node_is_door(e: &mut Engine, this: u32) -> bool {
    e.mem.u32(this) & 4 == 4
}

// Typed from 006ca4c0
/// The request's `bFirstTangentUsesHeading` (Xbox PDB, +0xA0).
pub fn request_first_tangent_uses_heading(e: &mut Engine, this: Ptr) -> bool {
    e.mem.u8(this.addr() + B_FIRST_TANGENT_USES_HEADING) != 0
}

// Typed from 006ca4e0
/// The request's `fInitialPathHeading` (Xbox PDB, +0x90), as `FLD` loads it.
pub fn request_initial_path_heading(e: &mut Engine, this: Ptr) -> f32 {
    let bits = e.mem.u32(this.addr() + F_INITIAL_PATH_HEADING);
    f32::from_bits(x87_quiet_bits(bits))
}
