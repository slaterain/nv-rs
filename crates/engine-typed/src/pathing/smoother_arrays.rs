//! The path smoother POV search's arrays: the constructors, destructors, appends and scalar
//! deleting destructors of its `BSSimpleArray` members (the POVs, the nodes, the links, the edge
//! verts, the meshes).

use super::layout::{EdgePair, SimpleArray};
use super::{pov, search, FAMILY};
use engine_core::ptr::Ptr;
use engine_core::{args, typed_call, Engine};

/// The `BSSimpleArray<PathEdgeVert,1024>` vtable (`0106ba58`).
const EDGE_VERT_ARRAY_VTABLE: u32 = 0x0106_ba58;
/// The `BSSimpleArray<NavMesh*,1024>` vtable (`0106ba6c`).
const NAVMESH_ARRAY_VTABLE: u32 = 0x0106_ba6c;

/// Callees.
const ARRAY_CLEAR: u32 = 0x0084_54f0;
const FREE: u32 = 0x0040_1030;
const EDGE_VERT_INIT: u32 = 0x006b_4190;
const LINK_INDEX_OF_NEW: u32 = 0x006b_3fd0;
const LINK_PREPARE: u32 = 0x006a_2630;
const LINK_COPY: u32 = 0x006b_3910;
const POV_ARRAY_DTOR: u32 = 0x006b_35a0;
const NODE_ARRAY_DTOR: u32 = 0x006b_35f0;
const LINK_ARRAY_DTOR: u32 = 0x006b_3640;

/// The shape shared by the arrays' constructors: the vtable, then the array set-up with reserved
/// size and size 0.
fn construct_array(
    e: &mut Engine,
    this: Ptr<SimpleArray>,
    vtable: u32,
    set_up: Option<u32>,
) -> Ptr<SimpleArray> {
    e.set(this, SimpleArray::vtable, vtable);
    match set_up {
        Some(addr) => {
            e.call(addr, &args![this, 0u32, 0u32]);
        }
        None => {
            typed_call!(FAMILY, 0x006b_3eb0, search::init_vertices(e, this, 0, 0));
        }
    }
    this
}

/// The shape shared by the arrays' destructors: the vtable, then `008454f0` with 1.
fn destroy_array(e: &mut Engine, this: Ptr<SimpleArray>, vtable: u32) {
    e.set(this, SimpleArray::vtable, vtable);
    e.call(ARRAY_CLEAR, &args![this, 1u32]);
}

// Typed from 006b36e0
/// The `BSSimpleArray<PathSmootherPOVSearch::PathEdgeVert,1024>` constructor
/// (`thiscall(array)`): the vtable, then the set-up (`006b4190(0, 0)`). Returns `this`.
pub fn edge_vert_array_new(e: &mut Engine, this: Ptr<SimpleArray>) -> Ptr<SimpleArray> {
    construct_array(e, this, EDGE_VERT_ARRAY_VTABLE, Some(EDGE_VERT_INIT))
}

// Typed from 006b3710
/// The `PathEdgeVert` array's destructor (`thiscall(array)`): the vtable, then `008454f0` with 1.
pub fn edge_vert_array_drop(e: &mut Engine, this: Ptr<SimpleArray>) {
    destroy_array(e, this, EDGE_VERT_ARRAY_VTABLE);
}

// Typed from 006b3660
/// Appends a copy of the `PathSmootherPOVLink` at `link` to the link array
/// (`006b3fd0`, `006a2630`, `006b3910`); returns its index.
pub fn link_array_append(e: &mut Engine, this: Ptr<SimpleArray>, link: Ptr) -> u32 {
    let index = e.call(LINK_INDEX_OF_NEW, &args![this]).u32();
    let slot = e.get(this, SimpleArray::data).wrapping_add(index << 6);
    e.call(LINK_PREPARE, &args![this, slot, 1u32]);
    let slot = e.get(this, SimpleArray::data).wrapping_add(index << 6);
    e.call(LINK_COPY, &args![slot, link]);
    index
}

// Typed from 006b3730
/// Appends a copy of the `PathEdgeVert` at `vert` ([`search::next_slot`],
/// [`search::construct_32s`], [`pov::edge_pair_copy`]); returns its index.
pub fn vert_array_append(e: &mut Engine, this: Ptr<SimpleArray>, vert: Ptr) -> u32 {
    let index: u32 = typed_call!(FAMILY, 0x006b_4060, search::next_slot(e, this));
    let slot = e.get(this, SimpleArray::data).wrapping_add(index << 5);
    typed_call!(
        FAMILY,
        0x006b_40f0,
        search::construct_32s(e, this.cast(), slot, 1)
    );
    let slot = e.get(this, SimpleArray::data).wrapping_add(index << 5);
    let _: Ptr<EdgePair> = typed_call!(
        FAMILY,
        0x006b_39a0,
        pov::edge_pair_copy(e, Ptr::new(slot), vert.cast())
    );
    index
}

// Typed from 006b3800
/// The `BSSimpleArray<NavMesh*,1024>` constructor (`thiscall(array)`): the vtable, then empty
/// ([`search::init_vertices`]). Returns `this`.
pub fn navmesh_array_new(e: &mut Engine, this: Ptr<SimpleArray>) -> Ptr<SimpleArray> {
    construct_array(e, this, NAVMESH_ARRAY_VTABLE, None)
}

// Typed from 006b3830
/// The `BSSimpleArray<NavMesh*,1024>` destructor (`thiscall(array)`): the vtable, then `008454f0`
/// with 1.
pub fn navmesh_array_drop(e: &mut Engine, this: Ptr<SimpleArray>) {
    destroy_array(e, this, NAVMESH_ARRAY_VTABLE);
}

/// The scalar deleting destructors' tail: frees `this` (`00401030`) when bit 0 of `flags` is set.
fn free_if_flagged(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    if flags & 1 != 0 {
        e.call(FREE, &args![this]);
    }
    this
}

// Typed from 006b3880
/// `BSSimpleArray<PathSmootherPOV*,1024>::`scalar deleting destructor'`: the destructor `006b35a0`,
/// then frees the object when bit 0 of `flags` is set; returns `this`.
pub fn pov_array_scalar_delete(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    e.call(POV_ARRAY_DTOR, &args![this]);
    free_if_flagged(e, this, flags)
}

// Typed from 006b38b0
/// `BSSimpleArray<PathSmootherPOVNode,1024>::`scalar deleting destructor'`: the destructor
/// `006b35f0`, then frees the object when bit 0 of `flags` is set; returns `this`.
pub fn node_array_scalar_delete(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    e.call(NODE_ARRAY_DTOR, &args![this]);
    free_if_flagged(e, this, flags)
}

// Typed from 006b38e0
/// `BSSimpleArray<PathSmootherPOVLink,1024>::`scalar deleting destructor'`: the destructor
/// `006b3640`, then frees the object when bit 0 of `flags` is set; returns `this`.
pub fn link_array_scalar_delete(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    e.call(LINK_ARRAY_DTOR, &args![this]);
    free_if_flagged(e, this, flags)
}
