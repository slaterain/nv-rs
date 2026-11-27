//! The `PathBuilder`'s virtual-node run finder (`006c9fc0`) and the node/request flag
//! accessors (`006ca830`, `006caa20`, `006cd1d0`).

use engine_core::ptr::Ptr;
use engine_core::{args, typed_call, Engine};

use super::{location, search, FAMILY};

/// Callees.
const NODE_LOCATION: u32 = 0x0071_7e50;
const CELL_TEST: u32 = 0x0045_0ff0;
const SOLUTION_VIRTUAL_NODE_COUNT: u32 = 0x005a_4320;
const VIRTUAL_NODE_IS_DOOR: u32 = 0x006c_a0c0;

/// The `PathBuilder`'s `pSolution` offset (Xbox PDB).
const P_SOLUTION: u32 = 0x04;

/// The request's `+0xA1` and `+0xA3` byte offsets.
const B_0XA1: u32 = 0xa1;
const B_0XA3: u32 = 0xa3;

/// The `VirtualNode`'s `iFlags` navmesh bit.
const FLAG_NAVMESH: u32 = 1;

/// The `PathingSolution` of the builder.
fn solution(e: &mut Engine, this: Ptr) -> u32 {
    e.mem.u32(this.addr() + P_SOLUTION)
}

/// The `VirtualNode` at `index` of the solution.
fn virtual_node(e: &mut Engine, sol: u32, index: u32) -> u32 {
    typed_call!(
        FAMILY,
        0x006e_78b0,
        search::solution_node(e, Ptr::new(sol), index)
    )
}

/// Whether the node at `index` has a cell that passes `00450ff0`.
fn node_cell_passes(e: &mut Engine, sol: u32, index: u32) -> bool {
    let node = virtual_node(e, sol, index);
    let location = e.call(NODE_LOCATION, &args![node]).u32();
    let cell: u32 = typed_call!(FAMILY, 0x006d_d4f0, location::cell(e, Ptr::new(location)));
    cell != 0 && e.call(CELL_TEST, &args![cell]).bool()
}

// Typed from 006c9fc0
/// Finds the run of virtual nodes from `index` whose cells pass `00450ff0`: false when node
/// `index` fails; otherwise `*first = index` and `*last` the last node of the run, which also
/// ends after the first door node (`006ca0c0`).
pub fn find_virtual_node_run(e: &mut Engine, this: Ptr, index: i32, first: u32, last: u32) -> bool {
    let sol = solution(e, this);
    if !node_cell_passes(e, sol, index as u32) {
        return false;
    }
    e.mem.set_i32(first, index);
    e.mem.set_i32(last, e.mem.i32(first).wrapping_add(1));
    loop {
        let count = e.call(SOLUTION_VIRTUAL_NODE_COUNT, &args![sol]).u32() as i32;
        if e.mem.i32(last) >= count {
            break;
        }
        let current = e.mem.u32(last);
        if !node_cell_passes(e, sol, current) {
            break;
        }
        let current = e.mem.u32(last);
        let node = virtual_node(e, sol, current);
        if e.call(VIRTUAL_NODE_IS_DOOR, &args![node]).bool() {
            e.mem.set_i32(last, e.mem.i32(last).wrapping_add(1));
            break;
        }
        e.mem.set_i32(last, e.mem.i32(last).wrapping_add(1));
    }
    e.mem.set_i32(last, e.mem.i32(last).wrapping_sub(1));
    true
}

// Typed from 006ca830
/// Whether bit 0 of the virtual node's `iFlags` (Xbox PDB, +0) is set.
pub fn virtual_node_has_navmesh(e: &mut Engine, this: u32) -> bool {
    e.mem.u32(this) & FLAG_NAVMESH != 0
}

// Typed from 006caa20
/// The request's byte at +0xA1.
pub fn request_byte_0xa1(e: &mut Engine, this: u32) -> bool {
    e.mem.u8(this + B_0XA1) != 0
}

// Typed from 006cd1d0
/// The request's byte at +0xA3.
pub fn request_byte_0xa3(e: &mut Engine, this: u32) -> bool {
    e.mem.u8(this + B_0XA3) != 0
}
