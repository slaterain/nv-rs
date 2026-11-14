//! JIP LN NVSE's scene-graph lookup, read only: a reference's root node (`GetRefNiNode`,
//! `GetNiNode`, JIP's first/third person switch `GetNifBlock`), a block by name under a node
//! (`NiNode::GetBlock` / `GetBlockByName` / `GetNode`), a block's children and parents, an
//! object's extra data by class (`NiObjectNET::GetExtraData(vtbl)`), a reference's bounding box
//! (`TESObjectREFR::GetBoundingBox`) and its scale (the game's `TESObjectREFR::GetScale`). The
//! commands built on it are in `jip_fn_misc_ref_nodes.rs`; nothing here writes guest memory.
//!
//! Layouts, every one confirmed in the exe (Ghidra, FalloutNV.exe 1.4.0.525):
//! - `NiObjectNET`: name (`NiFixedString`) +8, extra data array +0x10 with its count (u16) +0x14
//!   (`NiObjectNET::GetExtraData` by name `00A5BDD0`; an extra data's name +8, `00A62660`).
//! - `NiAVObject`: parent +0x18, collision object +0x1C (`NiAVObject::UpdateWorldData`
//!   `00A68C60`), flags +0x30, local transform +0x34 and world transform +0x68, each a
//!   `NiTransform` of a rotation (nine floats by rows) +0, a translation +0x24 and a scale +0x30
//!   (`00A68BF0` copies 13 words local to world; `NiTransform::operator*` `0062C250`).
//! - `NiNode`: children `NiTArray` data +0xA0, its size (u16, the high-water mark the game's own
//!   walks stop at) +0xA6 (`NiNode::GetObjectByName` `00A5E560`, `UpdateDownwardPass`
//!   `00A5DD70`).
//! - A node is an object whose vtable slot +0xC (`GetAsNiNode`) is `006815C0` (returns `this`;
//!   `NiNode` `0109B5AC`, `BSFadeNode` `010A8F90`; every other class `00ACBB70`, returns 0); a
//!   geometry block one whose slot +0x18 is `00E68810` (returns `this`; `NiTriShape` `0109D454`,
//!   `NiTriStrips` `0109CD44`, `NiLines` `0109D55C`, `NiGeometry` `0109E04C`): JIP's `IS_NODE` and
//!   `IS_GEOMETRY`.
//! - `TESObjectREFR`: loaded data +0x64 with its root node +0x14 (the game's `Get3D` `0043FCD0`);
//!   the player (`refID` 0x14) its first person node +0x694 and its third person flag (byte)
//!   +0x64A (`PlayerCharacter::Get3D` `00950B60`).
//! - Actors: the vtable answers true (`008D0360`) at +0xFC (`Actor` `01084254`, `Character`,
//!   `Creature`, `PlayerCharacter`; `TESObjectREFR` `0102F55C` answers false); the process +0x68,
//!   its level (byte) +0x28, `MiddleHighProcess`' bound +0x224 (`GetBSBound` `009EE020`).
//! - `BSBound` (vtable `010C2B64`): centre +0xC, half extents +0x18 (`BSBound::LoadBinary`
//!   `00C54E30`).
//!
//! How JIP finds a name: its `GetBlock` turns the text into the game's fixed string (JIP replaces
//! the game's string table, `GetNiFixedString` patched over `00A5B690`, with its own, keyed by a
//! case-sensitive hash) and compares each block's name pointer with it, as the game's own
//! `GetObjectByName` (`00A59E20`) does; a text not in the table finds nothing. The port compares
//! the texts byte for byte instead (it never adds to the string table): the same blocks, since
//! fixed strings are unique per text. JIP's table takes two texts of one hash as the same string;
//! the port does not (a deliberate difference: a hash collision is not a name).
//!
//! Safety: JIP walks the children recursively without a bound and reads every pointer as it is,
//! so a model whose children run back into themselves overflows its stack and a damaged pointer
//! stops the game. Here every guest word is read checked (an unreadable one counts as none),
//! every walk is bounded ([`MAX_DEPTH`] levels, [`MAX_VISITS`] blocks, [`MAX_NAME`] bytes of a
//! name), and what lies past a bound counts as not found: a half-loaded reference or a damaged
//! model answers nothing rather than stopping the game.

use engine_core::prelude::*;

/// JIP's `ADDR_ReturnThis` (`006815C0`) and `ADDR_ReturnThis2` (`00E68810`), the answers of a
/// node's vtable slot +0xC and a geometry block's slot +0x18.
pub(crate) const RETURN_THIS: u32 = 0x0068_15c0;
pub(crate) const RETURN_THIS2: u32 = 0x00e6_8810;
/// `ADDR_ReturnTrue` (`008D0360`).
pub(crate) const RETURN_TRUE: u32 = 0x008d_0360;

pub(crate) const NAME: u32 = 0x08;
pub(crate) const EXTRA_LIST: u32 = 0x10;
pub(crate) const EXTRA_COUNT: u32 = 0x14;
pub(crate) const PARENT: u32 = 0x18;
pub(crate) const COLLISION_OBJECT: u32 = 0x1c;
pub(crate) const FLAGS: u32 = 0x30;
pub(crate) const LOCAL_ROTATE: u32 = 0x34;
pub(crate) const LOCAL_TRANSLATE: u32 = 0x58;
pub(crate) const LOCAL_SCALE: u32 = 0x64;
pub(crate) const WORLD_ROTATE: u32 = 0x68;
pub(crate) const WORLD_TRANSLATE: u32 = 0x8c;
const CHILDREN: u32 = 0xa0;
const CHILDREN_SIZE: u32 = 0xa6;

const REF_ID: u32 = 0x0c;
const LOADED_DATA: u32 = 0x64;
const ROOT_NODE: u32 = 0x14;
const PROCESS: u32 = 0x68;
const PROCESS_LEVEL: u32 = 0x28;
const PROCESS_BOUND: u32 = 0x224;
const IS_ACTOR_SLOT: u32 = 0xfc;
/// The player's `refID`, its first person node and third person flag.
pub(crate) const PLAYER_ID: u32 = 0x14;
const NODE_1ST_PERSON: u32 = 0x694;
const IS_3RD_PERSON: u32 = 0x64a;
/// `kVtbl_BSBound` (JIP LN NVSE), confirmed: `010C2B64` holds `BSBound::LoadBinary` at +0x4C.
pub(crate) const BS_BOUND_VTBL: u32 = 0x010c_2b64;
/// The game's `TESObjectREFR::GetScale` (`00567400`, thiscall, the scale in `ST0`).
pub(crate) const GET_SCALE: u32 = 0x0056_7400;

/// The deepest a walk goes below its start node.
pub(crate) const MAX_DEPTH: u32 = 512;
/// The most blocks one walk looks at.
pub(crate) const MAX_VISITS: u32 = 1 << 16;
/// The longest name read (a fixed string longer than this, or with no end in readable memory,
/// counts as unreadable).
pub(crate) const MAX_NAME: u32 = 0x1000;

// ---- checked reads ----

/// The word at `at`, `None` when it is not readable.
pub(crate) fn rd32(e: &Engine, at: u32) -> Option<u32> {
    let mut b = [0u8; 4];
    e.mem
        .try_read(at, &mut b)
        .ok()
        .map(|()| u32::from_le_bytes(b))
}

/// The 16-bit word at `at`, `None` when it is not readable.
pub(crate) fn rd16(e: &Engine, at: u32) -> Option<u16> {
    let mut b = [0u8; 2];
    e.mem
        .try_read(at, &mut b)
        .ok()
        .map(|()| u16::from_le_bytes(b))
}

/// The byte at `at`, `None` when it is not readable.
pub(crate) fn rd8(e: &Engine, at: u32) -> Option<u8> {
    let mut b = [0u8; 1];
    e.mem.try_read(at, &mut b).ok().map(|()| b[0])
}

/// The float at `at`, `None` when it is not readable.
pub(crate) fn rdf32(e: &Engine, at: u32) -> Option<f32> {
    rd32(e, at).map(f32::from_bits)
}

/// The pointer at `at`: `None` for a null or unreadable one.
pub(crate) fn ptr(e: &Engine, at: u32) -> Option<u32> {
    rd32(e, at).filter(|&p| p != 0)
}

/// Three floats from `at`.
pub(crate) fn floats3(e: &Engine, at: u32) -> Option<[f32; 3]> {
    Some([rdf32(e, at)?, rdf32(e, at + 4)?, rdf32(e, at + 8)?])
}

/// Nine floats (a rotation by rows) from `at`.
pub(crate) fn matrix(e: &Engine, at: u32) -> Option<[f32; 9]> {
    let mut m = [0f32; 9];
    for (i, v) in m.iter_mut().enumerate() {
        *v = rdf32(e, at + 4 * i as u32)?;
    }
    Some(m)
}

/// The C text at `text` (without its 0), `None` when it has no end within [`MAX_NAME`] bytes of
/// readable memory.
pub(crate) fn c_text(e: &Engine, text: u32) -> Option<Vec<u8>> {
    let mut out = Vec::new();
    for i in 0..MAX_NAME {
        match rd8(e, text.wrapping_add(i))? {
            0 => return Some(out),
            b => out.push(b),
        }
    }
    None
}

// ---- blocks ----

/// The word at the object's vtable `slot`, `None` when unreadable.
fn vtable_slot(e: &Engine, object: u32, slot: u32) -> Option<u32> {
    let vtbl = rd32(e, object)?;
    rd32(e, vtbl.wrapping_add(slot))
}

/// `IS_NODE(object)` (JIP LN NVSE): the vtable's `GetAsNiNode` (+0xC) returns `this`.
pub(crate) fn is_node(e: &Engine, object: u32) -> bool {
    vtable_slot(e, object, 0xc) == Some(RETURN_THIS)
}

/// `IS_GEOMETRY(object)` (JIP LN NVSE): the vtable's slot +0x18 returns `this`.
pub(crate) fn is_geometry(e: &Engine, object: u32) -> bool {
    vtable_slot(e, object, 0x18) == Some(RETURN_THIS2)
}

/// The block's name pointer (`m_blockName`), 0 for none or unreadable.
fn name_ptr(e: &Engine, object: u32) -> u32 {
    rd32(e, object.wrapping_add(NAME)).unwrap_or(0)
}

/// `NiObjectNET::GetName` (JIP LN NVSE) as an array element takes it: the name's text, empty for
/// a block without one (xNVSE's `SetStr` stores a null text as no text, `ArrayData::SetStr`) or
/// whose text is unreadable.
pub(crate) fn name_text(e: &Engine, object: u32) -> Vec<u8> {
    match name_ptr(e, object) {
        0 => Vec::new(),
        p => c_text(e, p).unwrap_or_default(),
    }
}

/// Whether the block's name is the fixed string of `name` (the text, byte for byte; a block
/// without a name or with an unreadable one has none).
fn name_is(e: &Engine, object: u32, name: &[u8]) -> bool {
    match name_ptr(e, object) {
        0 => false,
        p => c_text(e, p).is_some_and(|t| t == name),
    }
}

/// The node's children slots in order (`m_children`, up to its size), empty ones left out, as
/// JIP's walks skip them.
pub(crate) fn children(e: &Engine, node: u32) -> Vec<u32> {
    let (Some(data), Some(size)) = (
        rd32(e, node.wrapping_add(CHILDREN)),
        rd16(e, node.wrapping_add(CHILDREN_SIZE)),
    ) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for i in 0..u32::from(size) {
        match rd32(e, data.wrapping_add(4 * i)) {
            Some(0) => {}
            Some(child) => out.push(child),
            None => break,
        }
    }
    out
}

/// A walk's budget: blocks looked at so far.
pub(crate) struct Walk {
    visits: u32,
}

impl Walk {
    pub(crate) fn new() -> Walk {
        Walk { visits: 0 }
    }

    /// One more block; false past [`MAX_VISITS`].
    pub(crate) fn step(&mut self) -> bool {
        self.visits += 1;
        self.visits <= MAX_VISITS
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/netimmerse.cpp: NiNode::GetBlockByName
// (asm).
/// The first block under `node` named `name`, depth first: each child in order is compared, then
/// (a node) searched below before the next child. 0 for none.
fn block_by_name(e: &Engine, node: u32, name: &[u8], depth: u32, walk: &mut Walk) -> u32 {
    if depth >= MAX_DEPTH {
        return 0;
    }
    for child in children(e, node) {
        if !walk.step() {
            return 0;
        }
        if name_is(e, child, name) {
            return child;
        }
        if is_node(e, child) {
            let found = block_by_name(e, child, name, depth + 1, walk);
            if found != 0 {
                return found;
            }
        }
    }
    0
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/netimmerse.cpp: NiNode::GetBlock (asm).
/// The block named `name`: 0 for an empty name, the node itself when it has the name, else the
/// first below it ([`block_by_name`]).
pub(crate) fn get_block(e: &Engine, node: u32, name: &[u8]) -> u32 {
    if name.is_empty() {
        return 0;
    }
    if name_is(e, node, name) {
        return node;
    }
    block_by_name(e, node, name, 0, &mut Walk::new())
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/netimmerse.cpp: NiNode::GetNode (asm).
/// [`get_block`], kept only when the block is a node. No built command calls it yet: JIP's
/// `GetNode`/`GetNode2` callers (`MoveToNode`, `AttachLight`, the ray casts) wait for the write and
/// Havok lanes; translated and tested here with the lookup it belongs to.
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) fn get_node(e: &Engine, node: u32, name: &[u8]) -> u32 {
    let block = get_block(e, node, name);
    if block != 0 && is_node(e, block) {
        block
    } else {
        0
    }
}

// ---- a reference's root ----

/// `TESObjectREFR::GetRefNiNode` (JIP LN NVSE): the loaded data's root node, 0 without 3D.
pub(crate) fn ref_ni_node(e: &Engine, refr: u32) -> u32 {
    ptr(e, refr.wrapping_add(LOADED_DATA))
        .and_then(|loaded| rd32(e, loaded.wrapping_add(ROOT_NODE)))
        .unwrap_or(0)
}

/// Whether `refr` is the player (`refID` 0x14).
pub(crate) fn is_player(e: &Engine, refr: u32) -> bool {
    rd32(e, refr.wrapping_add(REF_ID)) == Some(PLAYER_ID)
}

/// The player's first person node (+0x694).
pub(crate) fn node_1st_person(e: &Engine, player: u32) -> u32 {
    rd32(e, player.wrapping_add(NODE_1ST_PERSON)).unwrap_or(0)
}

/// The player's third person flag (byte +0x64A) is set.
pub(crate) fn is_3rd_person(e: &Engine, player: u32) -> bool {
    rd8(e, player.wrapping_add(IS_3RD_PERSON)).is_some_and(|b| b != 0)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), nvse/GameObjects.cpp: TESObjectREFR::GetNiNode
// (asm).
/// The root node as seen: [`ref_ni_node`], but for the player in first person (flag clear) its
/// first person node; 0 without loaded data (the player's too).
pub(crate) fn ni_node(e: &Engine, refr: u32) -> u32 {
    let Some(loaded) = ptr(e, refr.wrapping_add(LOADED_DATA)) else {
        return 0;
    };
    let root = rd32(e, loaded.wrapping_add(ROOT_NODE)).unwrap_or(0);
    if is_player(e, refr) && !is_3rd_person(e, refr) {
        node_1st_person(e, refr)
    } else {
        root
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), nvse/GameObjects.cpp: TESObjectREFR::GetNiBlock
// (asm).
/// The block named `name` under [`ni_node`]; the root itself for an empty name.
pub(crate) fn ni_block(e: &Engine, refr: u32, name: &[u8]) -> u32 {
    let root = ni_node(e, refr);
    if root == 0 || name.is_empty() {
        return root;
    }
    get_block(e, root, name)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_misc_ref.h: GetNifBlock
// (asm).
/// The block named `name` under the reference's root; the root itself for an empty name. For the
/// player with `pc_node`'s low byte not 0 the root is chosen by it: odd the third person root
/// ([`ref_ni_node`]), even the first person node, whatever the camera; otherwise [`ni_node`].
pub(crate) fn nif_block(e: &Engine, refr: u32, pc_node: u32, name: &[u8]) -> u32 {
    let pc = pc_node as u8;
    let root = if pc != 0 && is_player(e, refr) {
        if pc & 1 != 0 {
            ref_ni_node(e, refr)
        } else {
            node_1st_person(e, refr)
        }
    } else {
        ni_node(e, refr)
    };
    if root == 0 || name.is_empty() {
        return root;
    }
    get_block(e, root, name)
}

// ---- extra data, bounds, scale ----

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/netimmerse.cpp: NiObjectNET::GetExtraData
// (asm).
/// The object's last extra data (from the end of its array) whose vtable is `vtbl`; 0 for none.
pub(crate) fn extra_data_of_class(e: &Engine, object: u32, vtbl: u32) -> u32 {
    let (Some(list), Some(count)) = (
        rd32(e, object.wrapping_add(EXTRA_LIST)),
        rd16(e, object.wrapping_add(EXTRA_COUNT)),
    ) else {
        return 0;
    };
    for i in (0..u32::from(count)).rev() {
        match rd32(e, list.wrapping_add(4 * i)) {
            None => return 0,
            Some(0) => {}
            Some(x) => {
                if rd32(e, x) == Some(vtbl) {
                    return x;
                }
            }
        }
    }
    0
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), nvse/GameObjects.cpp:
// TESObjectREFR::GetBoundingBox (asm).
/// The reference's `BSBound`: an actor's with a high or middle-high process (level byte 0 or 1)
/// is its process' (+0x224, possibly none); any other's the `BSBound` extra data of its loaded
/// root ([`ref_ni_node`]). 0 for none.
pub(crate) fn bounding_box(e: &Engine, refr: u32) -> u32 {
    if vtable_slot(e, refr, IS_ACTOR_SLOT) == Some(RETURN_TRUE) {
        if let Some(process) = ptr(e, refr.wrapping_add(PROCESS)) {
            if rd8(e, process.wrapping_add(PROCESS_LEVEL)).is_some_and(|l| l <= 1) {
                return rd32(e, process.wrapping_add(PROCESS_BOUND)).unwrap_or(0);
            }
        }
    }
    match ref_ni_node(e, refr) {
        0 => 0,
        root => extra_data_of_class(e, root, BS_BOUND_VTBL),
    }
}

/// `TESObjectREFR::GetScale` (JIP LN NVSE `ThisCall<float>(0x567400, this)`): the game's own.
pub(crate) fn scale(e: &mut Engine, refr: u32) -> f32 {
    e.call(GET_SCALE, &[refr]).f32()
}

// ---- rotations (JIP's matrix maths) ----

/// `Flt180dPI` (JIP LN NVSE `57.29578018F`).
pub(crate) const FLT_180_D_PI: f32 = 57.295_780_18_f64 as f32;

fn neg(v: f32) -> f32 {
    f32::from_bits(v.to_bits() ^ 0x8000_0000)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/Ni_types.cpp: NiMatrix33::ToEulerPRY
// (asm).
/// `[ATan2(m5, m8), ASin(-m2), ATan2(m1, m0)]` (JIP's routines, `ln_fn_math`).
pub(crate) fn to_euler_pry(m: &[f32; 9]) -> [f32; 3] {
    use super::ln_fn_math::{asin, atan2};
    [atan2(m[5], m[8]), asin(neg(m[2])), atan2(m[1], m[0])]
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/Ni_types.cpp: NiMatrix33::ToEulerPRYInv
// (asm).
/// `[ATan2(m7, m8), ASin(-m6), ATan2(m3, m0)]`.
pub(crate) fn to_euler_pry_inv(m: &[f32; 9]) -> [f32; 3] {
    use super::ln_fn_math::{asin, atan2};
    [atan2(m[7], m[8]), asin(neg(m[6])), atan2(m[3], m[0])]
}

#[cfg(test)]
#[path = "nif_nodes_tests.rs"]
pub(crate) mod tests;
