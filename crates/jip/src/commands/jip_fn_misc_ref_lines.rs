//! `functions_jip/jip_fn_misc_ref.h` (JIP LN NVSE): `AttachLine`, a coloured line of a given
//! length put under a node of a reference's model (for the player also under the same node of
//! its first-person model). The rest of the file's commands are in `jip_fn_misc_ref.rs` and the
//! `jip_fn_misc_ref_*` modules.
//!
//! What it does, as JIP's command body and `NiLines::Create` (internal/netimmerse.cpp), every
//! call checked in the exe (Ghidra, FalloutNV.exe 1.4.0.525):
//! - the node: JIP's `TESObjectREFR::GetNode2` (nvse/GameObjects.cpp): the loaded root (+0x64,
//!   +0x14; never the first-person node), for a non-empty name the block of that name below it
//!   when it is a node (none otherwise: the result 0);
//! - the colour's four channels times `1 / 255.0F` (single precision);
//! - the line (`NiLines::Create`): from the game's heap a connection array of two bytes (1, 0),
//!   two colours (the colour twice, 0x20 bytes) and two vertices ((0, 0, 0) and (0, 1, 0), 0x18
//!   bytes); an `NiLines` of 0xC4 bytes (`00AA13E0`, `cdecl`) made by its constructor
//!   (`00A746E0`, `thiscall` (lines, 2, vertices, colours, 0, 0, 0, connections), `RET 0x1C`);
//!   JIP's alpha property added (`NiGeometry::AddProperty`: its count up, a list node from
//!   `0043A010` holding it put first in the property list +0x24, the count +0x2C up, the tail
//!   +0x28 set for an empty list), then a `BSShaderNoLightingProperty` (0x80 bytes, its
//!   constructor `00B6FC90`, `fastcall`) added the same way; JIP's `AssignGeometryProps`
//!   (`NiAVObject::UpdateProperties` `00A5A040`, `this` the line in `ecx` and no stack word, `RET`;
//!   then `BSShaderManager::PrepareObjectRecurse` `00B57BD0`, `cdecl` (lines, 0, 0, 0), the four
//!   words JIP pushed before the first call); named (JIP's
//!   `NiObjectNET::SetName`, [`super::jip_fn_misc_ref_nodes_w::set_name`]); its local scale
//!   (+0x64) the length;
//! - added under the node (`AddObject`, vtable +0xDC, `00A5ED10`, `thiscall` (node, line, 1));
//! - for the player, when JIP's first-person node exists (`s_pc1stPersonNode`, kept by the
//!   G38 patch, JIP's own, in whenever JIP answers "installed":
//!   [`crate::game_patches::gp::first_person_node`]) and has a node of that name
//!   (`NiNode::GetNode`), a copy of the line ([`crate::game_patches::gp::create_copy`]) added
//!   under it; the result 1.
//!
//! JIP's alpha property (`s_alphaProperty`): made by JIP at its start-up once the game's data is
//! loaded (`NiAlphaProperty::Create`, `00A5CEB0`, then its flags (+0x18) set to 0x10ED), kept
//! for the run. The port makes it the first time a line needs it (the same object and flags)
//! and keeps it for the run.
//!
//! Deliberate difference: before the player's first-person model is first made (G38 keeps JIP's
//! copy of it) JIP would read through its null node; the port then adds the line under the
//! third-person node only.
//!
//! Threads: on the script's thread, as JIP's (no lock: JIP takes none); the alpha property's
//! word is shared by every thread's engine (made once, compare-and-swap).

use super::jip_fn_misc_ref_nodes::block_name;
use super::jip_fn_misc_ref_nodes_w::set_name;
use super::jip_fn_ui_nodes::ADD_OBJECT_SLOT;
use super::nif_nodes as nif;
use super::Handlers;
use crate::game_patches::gp;
use crate::game_patches::gx::refs_add;
use engine_core::prelude::*;
use nvse::commands::Read;
use nvse::{handled, CommandArgs};
use std::sync::atomic::{AtomicU32, Ordering};

#[rustfmt::skip]
pub static COMMANDS: Handlers = &[
    (0x2919, attach_line),
];

/// The game's `NiNew` allocation (`00AA13E0`, `cdecl` (size)).
const NI_ALLOCATE: u32 = 0x00aa_13e0;
/// `NiLines`' size and constructor; `BSShaderNoLightingProperty`'s size and constructor.
const LINES_SIZE: u32 = 0xc4;
const LINES_CONSTRUCT: u32 = 0x00a7_46e0;
const NO_LIGHTING_SIZE: u32 = 0x80;
const NO_LIGHTING_CONSTRUCT: u32 = 0x00b6_fc90;
/// `NiAlphaProperty::Create` (`00A5CEB0`) and its flags (+0x18, `u16`): JIP's 0x10ED.
const ALPHA_CREATE: u32 = 0x00a5_ceb0;
const ALPHA_FLAGS: u32 = 0x18;
const JIP_ALPHA_FLAGS: u16 = 0x10ed;
/// The property list's node allocator (`0043A010`, the receiver unused).
const LIST_NODE_NEW: u32 = 0x0043_a010;
/// `NiAVObject`'s property list: first node (+0x24), last (+0x28), count (+0x2C); a node's next
/// (+0), previous (+4), property (+8).
const PROPERTIES_HEAD: u32 = 0x24;
const PROPERTIES_TAIL: u32 = 0x28;
const PROPERTIES_COUNT: u32 = 0x2c;
const LIST_PREV: u32 = 0x04;
const LIST_ITEM: u32 = 0x08;
/// `AssignGeometryProps`' two calls: `NiAVObject::UpdateProperties` (`thiscall`, the object
/// alone) and `BSShaderManager::PrepareObjectRecurse` (`cdecl`, four words).
const ASSIGN_PROPS_1: u32 = 0x00a5_a040;
const ASSIGN_PROPS_2: u32 = 0x00b5_7bd0;
/// `NiAVObject`'s local scale (+0x64).
const LOCAL_SCALE: u32 = 0x64;
/// `1 / 255.0F`.
const COLOUR_SCALE: f32 = 1.0 / 255.0;

/// JIP's `s_alphaProperty`, made at the first line ([`alpha_property`]).
#[derive(Default)]
pub struct AlphaProperty(AtomicU32);

/// The alpha property's word made before the threads start, so every thread's engine shares it.
pub fn install(e: &mut Engine) {
    e.ext_free::<AlphaProperty>();
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_game.h (DeferredInit):
// s_alphaProperty.
/// JIP's alpha property: made the first time (`NiAlphaProperty::Create`, flags 0x10ED).
pub(crate) fn alpha_property(e: &mut Engine) -> u32 {
    let made = e.ext_free::<AlphaProperty>().0.load(Ordering::Acquire);
    if made != 0 {
        return made;
    }
    let property = e.call(ALPHA_CREATE, &[]).u32();
    if property == 0 {
        return 0;
    }
    e.mem
        .set_u16(property.wrapping_add(ALPHA_FLAGS), JIP_ALPHA_FLAGS);
    match e.ext_free::<AlphaProperty>().0.compare_exchange(
        0,
        property,
        Ordering::AcqRel,
        Ordering::Acquire,
    ) {
        Ok(_) => property,
        // Another thread's engine made it first: theirs is kept (this one is not used).
        Err(theirs) => theirs,
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/netimmerse.cpp: NiGeometry::AddProperty
// (asm).
/// `property` put first in `geometry`'s property list, counted up.
pub(crate) fn add_property(e: &mut Engine, geometry: u32, property: u32) {
    refs_add(e, property, 1);
    let node = e.call(LIST_NODE_NEW, &args![geometry]).u32();
    e.mem.set_u32(node.wrapping_add(LIST_ITEM), property);
    let count = geometry.wrapping_add(PROPERTIES_COUNT);
    e.mem.set_u32(count, e.mem.u32(count).wrapping_add(1));
    let head = e.mem.u32(geometry.wrapping_add(PROPERTIES_HEAD));
    e.mem.set_u32(geometry.wrapping_add(PROPERTIES_HEAD), node);
    if head != 0 {
        e.mem.set_u32(node, head);
        e.mem.set_u32(head.wrapping_add(LIST_PREV), node);
    } else {
        e.mem.set_u32(geometry.wrapping_add(PROPERTIES_TAIL), node);
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/netimmerse.cpp:
// NiAVObject::AssignGeometryProps (asm).
/// The object's properties updated and its geometry prepared for the shaders.
pub(crate) fn assign_geometry_props(e: &mut Engine, object: u32) {
    e.call(ASSIGN_PROPS_1, &args![object]);
    e.call(ASSIGN_PROPS_2, &args![object, 0u32, 0u32, 0u32]);
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/netimmerse.cpp: NiLines::Create (asm).
/// A line of `length` in `colour` (already scaled to 0..1) named `name`.
fn create_lines(e: &mut Engine, length: f32, colour: [f32; 4], name: &[u8]) -> u32 {
    let connections = super::heap_alloc(e, 2);
    e.mem.set_u16(connections, 1);
    let colours = super::heap_alloc(e, 0x20);
    for (i, c) in colour.into_iter().enumerate() {
        e.mem.set_f32(colours + 4 * i as u32, c);
        e.mem.set_f32(colours + 0x10 + 4 * i as u32, c);
    }
    let vertices = super::heap_alloc(e, 0x18);
    for i in 0..4 {
        e.mem.set_u32(vertices + 4 * i, 0);
    }
    e.mem.set_f32(vertices + 0x10, 1.0);
    e.mem.set_u32(vertices + 0x14, 0);
    let block = e.call(NI_ALLOCATE, &args![LINES_SIZE]).u32();
    let lines = e
        .call(
            LINES_CONSTRUCT,
            &args![
                block,
                2u32,
                vertices,
                colours,
                0u32,
                0u32,
                0u32,
                connections
            ],
        )
        .u32();
    let alpha = alpha_property(e);
    if alpha != 0 {
        add_property(e, lines, alpha);
    }
    let block = e.call(NI_ALLOCATE, &args![NO_LIGHTING_SIZE]).u32();
    let shader = e.call(NO_LIGHTING_CONSTRUCT, &args![block]).u32();
    add_property(e, lines, shader);
    assign_geometry_props(e, lines);
    set_name(e, lines, name);
    e.mem.set_f32(lines.wrapping_add(LOCAL_SCALE), length);
    lines
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), nvse/GameObjects.cpp: TESObjectREFR::GetNode2
// (asm).
/// The loaded root (never the first-person node); for a non-empty name the block below it, kept
/// only when it is a node (its vtable +0xC answers `006815C0`).
fn ref_get_node2(e: &Engine, refr: u32, name: &[u8]) -> u32 {
    let root = nif::ref_ni_node(e, refr);
    if root == 0 || name.is_empty() {
        return root;
    }
    nif::get_node(e, root, name)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_misc_ref.h:
// Cmd_AttachLine_Execute.
/// A node's name, the line's name, its length and colour (red, green, blue, alpha, 0 to 255): the
/// line added under the node (and the player's first-person node of that name); 1 when added.
fn attach_line(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if !read.ok || a.this_obj == 0 {
        return handled(true);
    }
    let node_name = block_name(e, &read, 0, "AttachLine");
    let line_name = block_name(e, &read, 1, "AttachLine");
    let length = read.float(2, 0.0);
    let colour: [f32; 4] = std::array::from_fn(|i| read.float(3 + i, 0.0) * COLOUR_SCALE);
    let target = ref_get_node2(e, a.this_obj, &node_name);
    if target == 0 {
        return handled(true);
    }
    let lines = create_lines(e, length, colour, &line_name);
    e.vcall(target, ADD_OBJECT_SLOT, &args![lines, 1u32]);
    if nif::is_player(e, a.this_obj) {
        let first = gp::first_person_node(e);
        if first != 0 {
            let node = nif::get_node(e, first, &node_name);
            if node != 0 {
                let copy = gp::create_copy(e, lines);
                e.vcall(node, ADD_OBJECT_SLOT, &args![copy, 1u32]);
            }
        }
    }
    a.set_result(e, 1.0);
    handled(true)
}

#[cfg(test)]
#[path = "jip_fn_misc_ref_lines_tests.rs"]
mod tests;
