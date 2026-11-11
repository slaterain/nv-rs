//! The small functions the two passes call for each object, cell and water entry: the exe's getters
//! (several shared by identical code with other classes, so each reads its word whatever the class),
//! the cell's 3D child lists, the grid's cell slots and the setting's value. Each is registered at its
//! address, so every other caller in the game reaches the typed one too.

use super::FAMILY;
use crate::dispatch;
use crate::multibound::{GridCellArray, TES};
use engine_core::ptr::Ptr;
use engine_core::typed::leaf;
use engine_core::{typed_call, Engine};

/// The shadow scene node table (`011f91c8`, one word per scene graph).
const SHADOW_SCENE_NODE_TABLE: u32 = 0x011f_91c8;
/// The zero word `0043d4d0` hands out for a null setting (`01202800`).
const SETTING_ZERO: u32 = 0x0120_2800;

// Typed from 006629f0
/// `thiscall(node)`: the multibound of a multibound node (the pointer at +0xac, through `00559450`).
pub fn multibound_of(e: &mut Engine, this: Ptr) -> u32 {
    typed_call!(
        leaf::FAMILY,
        0x0055_9450,
        leaf::word_at_0(e, this.addr().wrapping_add(0xac))
    )
}

// Typed from 00528820
/// `thiscall(shape)`: the multibound shape's cull result (+8) back to 0, not tested.
pub fn clear_cull_result(e: &mut Engine, this: Ptr) {
    e.mem.set_u32(this.addr().wrapping_add(8), 0);
}

// Typed from 00545cb0
/// `TESObjectCELL::Get3D` (`thiscall(cell)`): with loaded data (+0xc4) its first word, the cell's 3D
/// root (through `00559450`); otherwise null.
pub fn cell_3d(e: &mut Engine, this: Ptr) -> u32 {
    let loaded = e.mem.u32(this.addr().wrapping_add(0xc4));
    if loaded == 0 {
        return 0;
    }
    typed_call!(leaf::FAMILY, 0x0055_9450, leaf::word_at_0(e, loaded))
}

// Typed from 00456fc0
/// `thiscall(cell, index)`: child `index` of the cell's 3D root (its object lists: 2 the land, 7 the
/// multibound nodes), or null without a 3D root.
pub fn cell_child(e: &mut Engine, this: Ptr, index: u32) -> u32 {
    let root = typed_call!(FAMILY, 0x0054_5cb0, cell_3d(e, this));
    if root == 0 {
        return 0;
    }
    typed_call!(
        dispatch::FAMILY,
        0x0043_b4a0,
        dispatch::scene::child(e, Ptr::new(root), index)
    )
    .addr()
}

// Typed from 004ba490
/// `GridCellArray::Get` (`thiscall(grid, x, y)`): the address of the cell slot at (x, y), row `x` of
/// `iDimension` slots; null when either is not below the dimension (read through `0084e3a0`, twice).
pub fn grid_cell(e: &mut Engine, this: Ptr<GridCellArray>, x: u32, y: u32) -> u32 {
    let limit = typed_call!(leaf::FAMILY, 0x0084_e3a0, leaf::word_at_c(e, this));
    if x >= limit || y >= limit {
        return 0;
    }
    let row = typed_call!(leaf::FAMILY, 0x0084_e3a0, leaf::word_at_c(e, this));
    let index = row.wrapping_mul(x).wrapping_add(y);
    e.get(this, GridCellArray::pGridCellA)
        .wrapping_add(index.wrapping_mul(4))
}

// Typed from 0043d4d0
/// `thiscall(setting)`: the address of an integer setting's value (+4); for a null setting the zero
/// word at `01202800`, cleared first.
pub fn setting_value(e: &mut Engine, this: Ptr) -> u32 {
    if this.is_null() {
        e.set_global(SETTING_ZERO, 0u32);
        return SETTING_ZERO;
    }
    this.addr().wrapping_add(4)
}

// Typed from 00450b80
/// `cdecl(index)`: shadow scene node `index` of the table at `011f91c8`.
pub fn shadow_scene_node(e: &mut Engine, index: u32) -> u32 {
    e.global(SHADOW_SCENE_NODE_TABLE.wrapping_add(index.wrapping_mul(4)))
}

// Typed from 00454b30
/// `thiscall`: the word at +0x1e0 (a shadow scene node's portal graph, `pBSPortalGraph`).
pub fn portal_graph(e: &mut Engine, this: Ptr) -> u32 {
    e.mem.u32(this.addr().wrapping_add(0x1e0))
}

// Typed from 0045bc00
/// `thiscall(node, index)`: entry `index` of the node's child array (+0x9c), the count read through
/// `00658930`, the slot through `00877a30` and its pointer through `00559450`; null when `index` is not
/// below the count.
pub fn node_child(e: &mut Engine, this: Ptr, index: u32) -> u32 {
    let array = this.addr().wrapping_add(0x9c);
    let count = typed_call!(
        dispatch::FAMILY,
        0x0065_8930,
        dispatch::scene::array_size(e, Ptr::new(array))
    );
    if u32::from(count) <= index {
        return 0;
    }
    let slot = typed_call!(
        leaf::FAMILY,
        0x0087_7a30,
        leaf::array_element(e, array, index)
    );
    typed_call!(leaf::FAMILY, 0x0055_9450, leaf::word_at_0(e, slot))
}

// Typed from 004fd3e0
/// `thiscall`: the word at +0x88 (`TES::pWorldSpace`).
pub fn world_space(e: &mut Engine, this: Ptr<TES>) -> u32 {
    e.get(this, TES::pWorldSpace)
}

// Typed from 0070ec90
/// `thiscall`: the word at +0x64 (`TES::pWaterSystem`; the same code is other classes' getter).
pub fn word_at_64(e: &mut Engine, this: Ptr) -> u32 {
    e.mem.u32(this.addr().wrapping_add(0x64))
}

// Typed from 005a8080
/// `thiscall`: the address +0x3c (the water system's list of water groups).
pub fn address_3c(_e: &mut Engine, this: Ptr) -> u32 {
    this.addr().wrapping_add(0x3c)
}

// Typed from 006733e0
/// `thiscall`: the word at +0x48 (the water system's LOD water group; the same code is other classes'
/// getter).
pub fn word_at_48(e: &mut Engine, this: Ptr) -> u32 {
    e.mem.u32(this.addr().wrapping_add(0x48))
}

// Typed from 007b52d0
/// `thiscall(list, position)`: the list position after `position` (its first word), or null for a null
/// position; the list is not read.
pub fn next_position(e: &mut Engine, _list: Ptr, position: Ptr) -> u32 {
    if position.is_null() {
        return 0;
    }
    e.mem.u32(position.addr())
}

// Typed from 006317a0
/// `thiscall(list, position)`: the address of a list position's item (+8); the list is not read.
pub fn item_address(_e: &mut Engine, _list: Ptr, position: Ptr) -> u32 {
    position.addr().wrapping_add(8)
}

// Typed from 0045bad0
/// `cdecl(rtti, object)`: false for a null object, otherwise whether its slot 8 (`GetRTTI`) answers
/// `rtti` (`0045baf0`).
pub fn is_exactly(e: &mut Engine, rtti: u32, object: Ptr) -> bool {
    if object.is_null() {
        return false;
    }
    typed_call!(FAMILY, 0x0045_baf0, rtti_is(e, object, rtti))
}

// Typed from 0045baf0
/// `thiscall(object, rtti)`: whether the object's slot 8 (`GetRTTI`) answers `rtti`.
pub fn rtti_is(e: &mut Engine, this: Ptr, rtti: u32) -> bool {
    e.vcall(this.addr(), 8, &[]).u32() == rtti
}
