//! The POV search's `BSSimpleArray` destructor bodies (each stores its instance's vtable, then
//! runs the array's release) and the search's member release at +0x30.

use engine_core::ptr::Ptr;
use engine_core::{args, Engine};

/// The vtable of `BSSimpleArray<NavMeshPOV*,1024>` (`0106b3b0`).
const POV_ARRAY_VTABLE: u32 = 0x0106_b3b0;
/// The vtable of `BSSimpleArray<BSSimpleArray<NavMeshPOV*,1024>,1024>` (`0106b3d4`).
const POV_LIST_ARRAY_VTABLE: u32 = 0x0106_b3d4;
/// The vtable of `BSSimpleArray<NavMeshPOVSearchNode,1024>` (`0106b3e8`).
const SEARCH_NODE_ARRAY_VTABLE: u32 = 0x0106_b3e8;
/// The vtable of `BSSimpleArray<NavMeshPOVSearchLink,1024>` (`0106b3fc`).
const SEARCH_LINK_ARRAY_VTABLE: u32 = 0x0106_b3fc;
/// The vtable of `BSSimpleArray<BSSimpleArray<FastNavMeshVertHandle,1024>,1024>` (`0106b410`).
const VERT_HANDLE_LIST_ARRAY_VTABLE: u32 = 0x0106_b410;
/// The vtable of `BSSimpleArray<RemoveIndex,1024>` (`0106b424`).
const REMOVE_INDEX_ARRAY_VTABLE: u32 = 0x0106_b424;

/// Callees: the plain array's release (`008454f0`) and the nested array's (`0049faf0`), both
/// `thiscall` with the word 1.
const ARRAY_RELEASE: u32 = 0x0084_54f0;
const LIST_ARRAY_RELEASE: u32 = 0x0049_faf0;

/// A member object (+0x30).
const MEMBER: u32 = 0x30;

// Typed from 006a14e0
/// The `BSSimpleArray<NavMeshPOV*,1024>` destructor body: vtable `0106b3b0`, then the release
/// (`008454f0(this, 1)`).
pub fn pov_array_dtor(e: &mut Engine, this: Ptr) {
    e.mem.set_u32(this.addr(), POV_ARRAY_VTABLE);
    e.call(ARRAY_RELEASE, &args![this, 1u32]);
}

// Typed from 006a1700
/// The `BSSimpleArray<BSSimpleArray<NavMeshPOV*,1024>,1024>` destructor body: vtable `0106b3d4`,
/// then the release (`0049faf0(this, 1)`).
pub fn pov_list_array_dtor(e: &mut Engine, this: Ptr) {
    e.mem.set_u32(this.addr(), POV_LIST_ARRAY_VTABLE);
    e.call(LIST_ARRAY_RELEASE, &args![this, 1u32]);
}

// Typed from 006a1890
/// The `BSSimpleArray<NavMeshPOVSearchNode,1024>` destructor body: vtable `0106b3e8`, then the
/// release (`008454f0(this, 1)`).
pub fn search_node_array_dtor(e: &mut Engine, this: Ptr) {
    e.mem.set_u32(this.addr(), SEARCH_NODE_ARRAY_VTABLE);
    e.call(ARRAY_RELEASE, &args![this, 1u32]);
}

// Typed from 006a18e0
/// The `BSSimpleArray<NavMeshPOVSearchLink,1024>` destructor body: vtable `0106b3fc`, then the
/// release (`008454f0(this, 1)`).
pub fn search_link_array_dtor(e: &mut Engine, this: Ptr) {
    e.mem.set_u32(this.addr(), SEARCH_LINK_ARRAY_VTABLE);
    e.call(ARRAY_RELEASE, &args![this, 1u32]);
}

// Typed from 006a19e0
/// The `BSSimpleArray<BSSimpleArray<FastNavMeshVertHandle,1024>,1024>` destructor body: vtable
/// `0106b410`, then the release (`0049faf0(this, 1)`).
pub fn vert_handle_list_array_dtor(e: &mut Engine, this: Ptr) {
    e.mem.set_u32(this.addr(), VERT_HANDLE_LIST_ARRAY_VTABLE);
    e.call(LIST_ARRAY_RELEASE, &args![this, 1u32]);
}

// Typed from 006a1a30
/// The `BSSimpleArray<RemoveIndex,1024>` destructor body: vtable `0106b424`, then the release
/// (`008454f0(this, 1)`).
pub fn remove_index_array_dtor(e: &mut Engine, this: Ptr) {
    e.mem.set_u32(this.addr(), REMOVE_INDEX_ARRAY_VTABLE);
    e.call(ARRAY_RELEASE, &args![this, 1u32]);
}

// Typed from 0069efd0
/// When the object at +0x30 is set, its scalar deleting destructor (vtable slot 0) with 1.
pub fn release_member(e: &mut Engine, this: Ptr) {
    let member = e.mem.u32(this.addr() + MEMBER);
    if member != 0 {
        e.vcall(member, 0, &args![1u32]);
    }
}
