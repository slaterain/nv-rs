//! The reference's form id given to the geometry under a node: `SetReferenceIDOnScenegraph`
//! (`004b6dc0`, Xbox PDB), a walk of the scene graph from the node that stops at each geometry and
//! stores the id in the geometry's shader property, and `NiAVObject::GetProperty` (`00a59d30`, Xbox
//! PDB), which the walk asks of each geometry for that property. The game walks an actor's 3D this
//! way whenever it is loaded: `TES`'s load of a reference into the scene (`00451ef0`, every NPC and
//! creature but the player), the head's set-up (`0043ed90`: the biped's node and the face's) and an
//! actor's model update (`Update3dModel`, `008f69e0`). In the exe every object the walk visits
//! costs one or two virtual calls (as a geometry, then as a node) and calls by address: a node's
//! child count and child at each step, a geometry's `GetProperty` (with a virtual call for each
//! property it asks), `004a2c20` with its `00441110`, and `0057bd60`.
//!
//! **Native.** The walk keeps the exe's order: depth first, the children of a node in their order
//! from 0, the child count read again before each step, a null child or root visited and left at
//! once, and no depth limit of its own (the exe has none: it recurses with a plain call). Its
//! recursion is a plain Rust call, as the translation's is. A virtual call is answered in place
//! only when its slot holds the exe's function for that slot in the scene graph's classes, a body
//! with no calls of its own; any other target is called through the vtable as the exe calls it:
//!
//! - slot 0x18 (the object as a geometry): `00acbb70` (0) in the nodes, the lights and the cameras,
//!   `00e68810` (the object itself) in every geometry class (`NiTriShape`, `NiTriStrips`,
//!   `BSSegmentedTriShape`, `NiLines`, the particle systems);
//! - slot 0xc (the object as a node): `006815c0` (the object itself) in `NiNode` and the classes
//!   built on it (the fade node, the switch, billboard, BSP, LOD and multi-bound nodes, the scene
//!   graph), `00acbb70` (0) in the others;
//! - slot 0x8c of a property (its type), in `GetProperty`: the exe's eleven constant bodies (alpha
//!   0 at `00acbb70`, culling 1, material 2, shade 3 in every `BSShaderProperty` class, stencil 4,
//!   texturing 5, dither 8, specular 9, vertex colour 0xa, z-buffer 0xb, fog 0xd).
//!
//! Done in place, as they have no calls: `004a2c20` (whether the property's word at +0x1c, read by
//! `00441110`, is between 1 and 0xc, signed; false for no property) and `0057bd60` (the store at
//! +0x70). Called with `typed_call!`: `GetProperty` (this family's), the node's child count and
//! child (`0043b480`, `0043b4a0`, the dispatch family's).
//!
//! **JIP LN NVSE.** Nothing of JIP's replaces these functions or the slots they call. With JIP's
//! `bNPCWeaponMods` or `uNPCPerks` on, its handler at `0045234e` in `00451ef0` answers the node the
//! walk starts from (its render state's node, or none) in place of the reference's 3D; the walk is
//! the same for either root, and for none.
//!
//! **Which thread.** Main for the scene's load and the model update; the head's set-up runs where
//! its task runs. Integers only.

use super::FAMILY;
use crate::{dispatch, slot_target};
use engine_core::ptr::Ptr;
use engine_core::{args, typed_call, Engine};

/// The scene graph object's slots: the object as a node (+0xc) and as a geometry (+0x18).
const SLOT_AS_NODE: u32 = 0xc;
const SLOT_AS_GEOMETRY: u32 = 0x18;
/// The exe's bodies found in those slots: 0 (`00acbb70`), and the object itself (`006815c0` at +0xc
/// in the nodes, `00e68810` at +0x18 in the geometries).
const ANSWERS_ZERO: u32 = 0x00ac_bb70;
const NODE_ANSWERS_ITSELF: u32 = 0x0068_15c0;
const GEOMETRY_ANSWERS_ITSELF: u32 = 0x00e6_8810;
/// The property kind the walk asks for (the shade property, which every `BSShaderProperty` is).
const PROPERTY_SHADE: i32 = 3;
/// An object's list of properties (its head at +0x24; a list node holds the next node at +0 and the
/// property at +8), and the kinds `GetProperty` answers for (below 7, signed).
const PROPERTY_LIST_HEAD: u32 = 0x24;
const LIST_NODE_NEXT: u32 = 0;
const LIST_NODE_PROPERTY: u32 = 8;
const PROPERTY_KINDS: i32 = 7;
/// A property's slot that answers its kind, and the exe's constant bodies found there.
const SLOT_PROPERTY_KIND: u32 = 0x8c;
const KIND_BODIES: [(u32, u32); 11] = [
    (ANSWERS_ZERO, 0),  // NiAlphaProperty
    (0x00b6_f470, 1),   // the culling property
    (0x00c1_ba40, 2),   // NiMaterialProperty
    (0x00a6_a000, 3),   // NiShadeProperty and every BSShaderProperty class
    (0x0081_b6c0, 4),   // NiStencilProperty
    (0x00e9_e670, 5),   // NiTexturingProperty
    (0x00dc_9bc0, 8),   // NiDitherProperty
    (0x00a9_50e0, 9),   // NiSpecularProperty
    (0x00dc_9e60, 0xa), // NiVertexColorProperty
    (0x00a5_f500, 0xb), // NiZBufferProperty
    (0x00e0_9620, 0xd), // NiFogProperty and BSFogProperty
];
/// The property's word `004a2c20` tests (+0x1c, read by `00441110`) and the range it accepts.
const PROPERTY_WORD_1C: u32 = 0x1c;
const ACCEPTED_FIRST: i32 = 1;
const ACCEPTED_LAST: i32 = 0xc;
/// Where `0057bd60` stores the id in the property.
const PROPERTY_REFERENCE_ID: u32 = 0x70;

/// The object's slot 0x18 (the object as a geometry, 0 for any other class).
#[inline]
fn as_geometry(e: &mut Engine, object: u32) -> u32 {
    match slot_target(e, object, SLOT_AS_GEOMETRY) {
        ANSWERS_ZERO => 0,
        GEOMETRY_ANSWERS_ITSELF => object,
        _ => e.vcall(object, SLOT_AS_GEOMETRY, &args![]).u32(),
    }
}

/// The object's slot 0xc (the object as a node, 0 for any other class).
#[inline]
fn as_node(e: &mut Engine, object: u32) -> u32 {
    match slot_target(e, object, SLOT_AS_NODE) {
        ANSWERS_ZERO => 0,
        NODE_ANSWERS_ITSELF => object,
        _ => e.vcall(object, SLOT_AS_NODE, &args![]).u32(),
    }
}

/// The property's slot 0x8c (its kind).
#[inline]
fn kind_of_property(e: &mut Engine, property: u32) -> u32 {
    let target = slot_target(e, property, SLOT_PROPERTY_KIND);
    match KIND_BODIES.iter().find(|(body, _)| *body == target) {
        Some(&(_, kind)) => kind,
        None => e.vcall(property, SLOT_PROPERTY_KIND, &args![]).u32(),
    }
}

/// `004a2c20`: whether `property` is not null and its word at +0x1c (`00441110`) is between 1 and
/// 0xc (signed).
#[inline]
fn accepts(e: &Engine, property: u32) -> bool {
    if property == 0 {
        return false;
    }
    let word = e.mem.u32(property.wrapping_add(PROPERTY_WORD_1C)) as i32;
    (ACCEPTED_FIRST..=ACCEPTED_LAST).contains(&word)
}

/// The node's child count (`0043b480`), read again before every step.
#[inline]
fn children_count(e: &mut Engine, node: u32) -> u32 {
    u32::from(typed_call!(
        dispatch::FAMILY,
        0x0043_b480,
        dispatch::scene::child_count(e, Ptr::new(node))
    ))
}

/// The node's child `index` (`0043b4a0`).
#[inline]
fn child_at(e: &mut Engine, node: u32, index: u32) -> u32 {
    typed_call!(
        dispatch::FAMILY,
        0x0043_b4a0,
        dispatch::scene::child(e, Ptr::new(node), index)
    )
    .addr()
}

// Typed from 004b6dc0
/// `SetReferenceIDOnScenegraph` (Xbox PDB), `cdecl(object, id)`: nothing for a null object. An
/// object that is a geometry (its slot 0x18 answers non-zero) has its shade property asked for
/// (`GetProperty(object, 3)`, on the object itself) and, when `004a2c20` accepts that property, the
/// id stored in it (`0057bd60`); the walk ends there. Any other object that is a node (its slot
/// 0xc) has the walk run on each of its children in order, the count read again before each step.
pub fn set_reference_id_on_scenegraph(e: &mut Engine, object: u32, id: u32) {
    if object == 0 {
        return;
    }
    if as_geometry(e, object) != 0 {
        let property = typed_call!(FAMILY, 0x00a5_9d30, get_property(e, object, PROPERTY_SHADE));
        if accepts(e, property) {
            e.mem
                .set_u32(property.wrapping_add(PROPERTY_REFERENCE_ID), id);
        }
        return;
    }
    let node = as_node(e, object);
    if node != 0 {
        let mut index = 0u32;
        while index < children_count(e, node) {
            let child = child_at(e, node, index);
            set_reference_id_on_scenegraph(e, child, id);
            index += 1;
        }
    }
}

// Typed from 00a59d30
/// `NiAVObject::GetProperty` (Xbox PDB), `thiscall(this, kind)`, `ret 4`: 0 for a kind of 7 or more
/// (signed); else the first property of the object's list whose kind (its slot 0x8c) is `kind`, the
/// list's null items passed over, or 0 when none is. The exe reads each node's property and its
/// next node before it asks the property.
pub fn get_property(e: &mut Engine, this: u32, kind: i32) -> u32 {
    if kind >= PROPERTY_KINDS {
        return 0;
    }
    let mut node = e.mem.u32(this.wrapping_add(PROPERTY_LIST_HEAD));
    while node != 0 {
        let property = e.mem.u32(node.wrapping_add(LIST_NODE_PROPERTY));
        node = e.mem.u32(node.wrapping_add(LIST_NODE_NEXT));
        if property != 0 && kind_of_property(e, property) == kind as u32 {
            return property;
        }
    }
    0
}
