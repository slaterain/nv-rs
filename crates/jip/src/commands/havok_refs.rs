//! JIP LN NVSE's lookups of a reference's physics (Havok) state, read only: the root node and a
//! block by name (`GetRefNiNode`, `GetNiNode`, `GetNiBlock2`, `GetNode`, `GetNode2`), the rigid
//! body of a block (`TESObjectREFR::GetRigidBody`), an actor's character controller
//! (`GetCharacterController`), the reference a Havok object or a scene block belongs to
//! (`hkpWorldObject::GetParentRef`, `NiAVObject::GetParentRef`), the objects a reference touches
//! (`GetRefrContactObjects`, `NiNode::GetContactObjects`), and the recursive walks `IsMovable`,
//! `HasPhantom` and `GetBodyMass`; plus the world ray cast JIP builds for the game's pick
//! (`_GetRayCastObject`, `RayCastCoords`). The commands built on them are in
//! `jip_fn_misc_ref_havok.rs`. Nothing here writes game state (the ray cast fills a record on the
//! port's stack for the game's own `TES::Pick_ov2`).
//!
//! Layouts, each confirmed in the exe (Ghidra, FalloutNV.exe 1.4.0.525) unless marked:
//! - `TESObjectREFR`: loaded data +0x64 (`LOADED_REF_DATA`: root node +0x14, `spPhantom` +0x18,
//!   the port's `tesobjectrefr.rs` layout from the Xbox PDB with its PC readers); the player
//!   (`refID` 0x14) its third person flag (byte) +0x64A and first person node +0x694
//!   (`PlayerCharacter::Get3D` `00950B60`).
//! - A reference is an actor when its vtable answers true (`008D0360`) at +0x100 (`Actor`
//!   `01084254` +0x100 = `008D0360`, `TESObjectREFR` `0102F55C` answers `0047C850`); a projectile
//!   at +0x224 (`Projectile` `010900DC` +0x224 = `008D0360`); both slots are the ones the game's
//!   own grab test reads (`CreateMouseSpring` `0095F930`).
//! - An actor's process +0x68, its level (byte) +0x28, `MiddleHighProcess`' character controller
//!   +0x138 (`MiddleHighProcess::GetCharController` `008D8480`, the process' slot +0x28C the game
//!   calls through `MobileObject::GetCharController` `009306D0`).
//! - `bhkCharacterController`: `iSupportCount` +0x608 (JIP reads its low byte), `spSupportBody`
//!   +0x60C (its constructor, the port's `run_00c741e0.rs`), the point collector +0x10 whose
//!   touched objects (`m_collidingObjects`, `hkArray<hkpWorldObject*>`) are at +0x3A4 (data, size
//!   +4), so +0x3B4/+0x3B8 from the controller (`bhkcharacterpointcollector.rs`), the shape
//!   phantom `spShapePhantom` +0x594.
//! - `NiAVObject`: parent +0x18, collision object +0x1C; `NiNode` children data +0xA0, size (u16)
//!   +0xA6; a node's vtable slot +0xC answers `006815C0` (`IS_NODE`); `BSFadeNode` (vtable
//!   `010A8F90`) its reference +0xCC (`FindReferenceFor3D` `0056F930` reads it, `009AD610`).
//! - `bhkNiCollisionObject`: its world object (`bhkWorldObject`) +0x10 (the collision object's
//!   vtable `0103046C` slot +0x90, `00C65890`); a `bhkRefObject`'s Havok object +8
//!   (`bhkWorldObject::GetAVObject` `00C85C80`).
//! - `hkpWorldObject`: the collidable +0x10 whose broadphase type byte is +0x28 (1 an entity, 2 a
//!   phantom: `bhkUtilFunctions::GetNiAVObject` `00C7FA90`), the collision filter info +0x2C
//!   (layer in its low byte, `hkpRigidBody::getCinfo` `00CA8F50`), the properties (`hkArray` of
//!   0x10-byte entries: key +0, value +8) data +0x78 and size +0x7C (`hkpWorldObject::getProperty`
//!   `00C6D840`): key `011F4280` holds the scene object (`bhkWorldObject::GetAVObject`), key
//!   `01267B70` the collision object whose +8 is the node (`GetNiAVObject`).
//! - `hkpRigidBody` (`getCinfo`): friction +0x90, restitution +0x94, the master constraints (0x30
//!   bytes each) data +0xAC and count (u16) +0xB0, the slave constraints data +0xB4 and size +0xB8
//!   (`hkpEntity::sortConstraintsSlavesDeterministically` `00C9C860`; a constraint instance's
//!   bodies +0x14/+0x18), the motion +0xE0 with its type (byte) +0xE8, the motion state's rotation
//!   (three 16-byte columns) +0xF0, linear damping +0x194, angular damping +0x198, the inverse
//!   mass +0x1AC (`hkpMotion::getMass` `00C86430` reads motion +0xCC), linear velocity +0x1B0 and
//!   angular velocity +0x1C0. Header only (JIP's `havok.h`): the inertia floats +0x1A0..+0x1A8
//!   (`getCinfo` reads them through the motion's virtual) and a master constraint's other body
//!   at its +8.
//! - `hkpSimpleShapePhantom` (vtable `010CE15C`, its constructor `00D1F730`): the bodies it
//!   overlaps (`hkArray<hkCdBody*>`) data +0x160, size +0x164; `bhkSimpleShapePhantom` vtable
//!   `010305DC` (its constructor `0056E2D0`). A `hkCdBody` sits at +0x10 of its world object
//!   (JIP's `GetWorldObj`).
//!
//! Names: JIP's `GetBlock` turns the text into the game's fixed string through its own string
//! table and compares name pointers, as the game's `GetObjectByName` (`00A59E20`) does; a text not
//! in the table finds nothing. The port compares the texts byte for byte instead and never adds
//! to the string table: the same blocks, since fixed strings are unique per text. (Lane F's
//! `nif_nodes.rs` has the same lookup; this file keeps a private copy until the two are merged.)
//!
//! Safety: JIP reads every pointer as it finds it and walks children and lists without bounds, so
//! a damaged model or a body without its Havok object stops the game. Here every guest word is
//! read checked (an unreadable or null one counts as none), every walk is bounded ([`MAX_DEPTH`]
//! levels, [`MAX_ITEMS`] items from one list), one command's whole walk shares one budget
//! ([`Walk`]: [`MAX_WORK`] units, one per child slot examined and per Havok array item, property
//! or parent read, so damaged lists inside damaged lists cannot multiply the work), a contact list
//! holds at most [`MAX_ITEMS`] bodies in total, and what lies past a bound counts as nothing: a
//! reference without 3D, without a collision object or in a half-loaded cell answers as JIP
//! answers for "nothing".
//!
//! Threads: the script commands run on the main thread (`ProcessLists::RunActorScripts` and the
//! quest scripts in the frame's world step), while the Havok step runs between `StartThreads` and
//! `WaitForThreads` on an AI thread under `TES::LockHavokUpdateMT` (`00453860`;
//! docs/FRAME_SKELETON.md "The Havok step"). The condition path is the exception: the game
//! evaluates conditions (`GetHasContactType`'s eval) from AI code too, which can run on another
//! thread while the Havok step changes the lists read here. JIP takes no lock on either path and
//! neither do the game's own script commands and conditions, so the port takes none: every read
//! here is checked and bounded, so a list changing under a read gives at worst a wrong answer for
//! that one call, never a stop.

use engine_core::prelude::*;

/// JIP's `ADDR_ReturnThis` (`006815C0`) and `ADDR_ReturnTrue` (`008D0360`).
const RETURN_THIS: u32 = 0x0068_15c0;
pub(crate) const RETURN_TRUE: u32 = 0x008d_0360;
/// `kVtbl_BSFadeNode`, `kVtbl_bhkSimpleShapePhantom`, `kVtbl_hkpSimpleShapePhantom`.
pub(crate) const BS_FADE_NODE_VTBL: u32 = 0x010a_8f90;
pub(crate) const BHK_SIMPLE_SHAPE_PHANTOM_VTBL: u32 = 0x0103_05dc;
pub(crate) const HKP_SIMPLE_SHAPE_PHANTOM_VTBL: u32 = 0x010c_e15c;
/// The property keys (`NiRTTI` objects): `NiAVObject`'s and `bhkNiCollisionObject`'s.
pub(crate) const KEY_AV_OBJECT: u32 = 0x011f_4280;
pub(crate) const KEY_COLLISION_OBJECT: u32 = 0x0126_7b70;

pub(crate) const NAME: u32 = 0x08;
pub(crate) const PARENT: u32 = 0x18;
pub(crate) const COLLISION_OBJECT: u32 = 0x1c;
pub(crate) const WORLD_ROTATE: u32 = 0x68;
pub(crate) const WORLD_TRANSLATE: u32 = 0x8c;
pub(crate) const CHILDREN: u32 = 0xa0;
pub(crate) const CHILDREN_SIZE: u32 = 0xa6;
pub(crate) const FADE_NODE_REF: u32 = 0xcc;
/// `bhkNiCollisionObject::worldObj`, `bhkRefObject::refObject`.
pub(crate) const WORLD_OBJECT: u32 = 0x10;
pub(crate) const HAVOK_OBJECT: u32 = 0x08;

pub(crate) const REF_ID: u32 = 0x0c;
pub(crate) const LOADED_DATA: u32 = 0x64;
pub(crate) const ROOT_NODE: u32 = 0x14;
pub(crate) const PHANTOM: u32 = 0x18;
pub(crate) const PLAYER_ID: u32 = 0x14;
pub(crate) const IS_3RD_PERSON: u32 = 0x64a;
pub(crate) const NODE_1ST_PERSON: u32 = 0x694;
pub(crate) const PROCESS: u32 = 0x68;
pub(crate) const PROCESS_LEVEL: u32 = 0x28;
pub(crate) const CHAR_CONTROLLER: u32 = 0x138;
pub(crate) const IS_ACTOR_SLOT: u32 = 0x100;
pub(crate) const IS_PROJECTILE_SLOT: u32 = 0x224;

pub(crate) const SUPPORT_COUNT: u32 = 0x608;
pub(crate) const SUPPORT_BODY: u32 = 0x60c;
pub(crate) const CONTACT_BODIES: u32 = 0x3b4;
pub(crate) const SHAPE_PHANTOM: u32 = 0x594;

pub(crate) const COLLISION_TYPE: u32 = 0x28;
pub(crate) const FILTER_INFO: u32 = 0x2c;
pub(crate) const PROPERTIES: u32 = 0x78;
pub(crate) const MASTERS: u32 = 0xac;
pub(crate) const MASTERS_COUNT: u32 = 0xb0;
pub(crate) const SLAVES: u32 = 0xb4;
pub(crate) const MOTION_TYPE: u32 = 0xe8;
pub(crate) const ROTATION: u32 = 0xf0;
pub(crate) const MASS_INV: u32 = 0x1ac;
pub(crate) const PHANTOM_BODIES: u32 = 0x160;

/// The deepest a walk goes below its start node.
pub(crate) const MAX_DEPTH: u32 = 512;
/// The most work one command's walk does ([`Walk`]).
pub(crate) const MAX_WORK: u32 = 1 << 20;
/// The most items read from one Havok array (or properties walked), and the most bodies one
/// contact list holds.
pub(crate) const MAX_ITEMS: u32 = 1 << 16;
/// The longest name read.
const MAX_NAME: u32 = 0x1000;

// ---- checked reads ----

/// The word at `at`, `None` when it is not readable.
pub(crate) fn rd32(e: &Engine, at: u32) -> Option<u32> {
    let mut b = [0u8; 4];
    e.mem
        .try_read(at, &mut b)
        .ok()
        .map(|()| u32::from_le_bytes(b))
}

/// The 16-bit word at `at`.
pub(crate) fn rd16(e: &Engine, at: u32) -> Option<u16> {
    let mut b = [0u8; 2];
    e.mem
        .try_read(at, &mut b)
        .ok()
        .map(|()| u16::from_le_bytes(b))
}

/// The byte at `at`.
pub(crate) fn rd8(e: &Engine, at: u32) -> Option<u8> {
    let mut b = [0u8; 1];
    e.mem.try_read(at, &mut b).ok().map(|()| b[0])
}

/// The float at `at`.
pub(crate) fn rdf32(e: &Engine, at: u32) -> Option<f32> {
    rd32(e, at).map(f32::from_bits)
}

/// The pointer at `at`: `None` for a null or unreadable one.
pub(crate) fn ptr(e: &Engine, at: u32) -> Option<u32> {
    rd32(e, at).filter(|&p| p != 0)
}

/// Four floats (a 16-byte vector) from `at`.
pub(crate) fn vec4(e: &Engine, at: u32) -> Option<[f32; 4]> {
    Some([
        rdf32(e, at)?,
        rdf32(e, at.wrapping_add(4))?,
        rdf32(e, at.wrapping_add(8))?,
        rdf32(e, at.wrapping_add(12))?,
    ])
}

/// The word at the object's vtable `slot`.
pub(crate) fn vtable_slot(e: &Engine, object: u32, slot: u32) -> Option<u32> {
    rd32(e, rd32(e, object)?.wrapping_add(slot))
}

/// `IS_ACTOR(form)`: the vtable answers true at +0x100.
pub(crate) fn is_actor(e: &Engine, refr: u32) -> bool {
    vtable_slot(e, refr, IS_ACTOR_SLOT) == Some(RETURN_TRUE)
}

/// `IS_PROJECTILE(form)`: the vtable answers true at +0x224.
pub(crate) fn is_projectile(e: &Engine, refr: u32) -> bool {
    vtable_slot(e, refr, IS_PROJECTILE_SLOT) == Some(RETURN_TRUE)
}

/// `IS_NODE(object)`: the vtable's slot +0xC answers `this`.
pub(crate) fn is_node(e: &Engine, object: u32) -> bool {
    vtable_slot(e, object, 0xc) == Some(RETURN_THIS)
}

// ---- blocks ----

/// One command's walk budget: units of work done so far, one per child slot examined and per
/// Havok array item, property entry or parent read. Shared by everything one command walks.
pub(crate) struct Walk {
    work: u32,
}

impl Walk {
    pub(crate) fn new() -> Walk {
        Walk { work: 0 }
    }

    /// One more unit of work; false (and nothing charged) once [`MAX_WORK`] are done.
    pub(crate) fn charge(&mut self) -> bool {
        if self.work >= MAX_WORK {
            return false;
        }
        self.work += 1;
        true
    }

    /// Whether the budget is used up.
    pub(crate) fn spent(&self) -> bool {
        self.work >= MAX_WORK
    }
}

/// The node's children slots in order (up to its size), empty ones left out as JIP's walks skip
/// them; an unreadable slot or a spent budget ends the list.
pub(crate) fn children(e: &Engine, node: u32, walk: &mut Walk) -> Vec<u32> {
    let (Some(data), Some(size)) = (
        rd32(e, node.wrapping_add(CHILDREN)),
        rd16(e, node.wrapping_add(CHILDREN_SIZE)),
    ) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for i in 0..u32::from(size) {
        if !walk.charge() {
            break;
        }
        match rd32(e, data.wrapping_add(4 * i)) {
            Some(0) => {}
            Some(child) => out.push(child),
            None => break,
        }
    }
    out
}

/// The node children that are nodes (`IS_NODE`), in order: what JIP's recursive walks enter.
pub(crate) fn child_nodes(e: &Engine, node: u32, walk: &mut Walk) -> Vec<u32> {
    children(e, node, walk)
        .into_iter()
        .filter(|&c| is_node(e, c))
        .collect()
}

/// The C text at `text`, `None` without an end within [`MAX_NAME`] readable bytes.
fn c_text(e: &Engine, text: u32) -> Option<Vec<u8>> {
    let mut out = Vec::new();
    for i in 0..MAX_NAME {
        match rd8(e, text.wrapping_add(i))? {
            0 => return Some(out),
            b => out.push(b),
        }
    }
    None
}

/// Whether the block's name is the fixed string of `name`.
fn name_is(e: &Engine, object: u32, name: &[u8]) -> bool {
    match ptr(e, object.wrapping_add(NAME)) {
        None => false,
        Some(p) => c_text(e, p).is_some_and(|t| t == name),
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/netimmerse.cpp: NiNode::GetBlockByName
// (asm).
/// The first block under `node` named `name`, depth first: each child compared, then (a node)
/// searched below before the next child. 0 for none.
fn block_by_name(e: &Engine, node: u32, name: &[u8], depth: u32, walk: &mut Walk) -> u32 {
    if depth >= MAX_DEPTH {
        return 0;
    }
    for child in children(e, node, walk) {
        if walk.spent() {
            return 0;
        }
        if name_is(e, child, name) {
            return child;
        }
        if is_node(e, child) && !walk.spent() {
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
/// first below it.
pub(crate) fn get_block(e: &Engine, node: u32, name: &[u8]) -> u32 {
    if name.is_empty() {
        return 0;
    }
    if name_is(e, node, name) {
        return node;
    }
    block_by_name(e, node, name, 0, &mut Walk::new())
}

/// The block kept only when it is a node (the tail of JIP's `GetNode`/`GetNode2`).
fn only_node(e: &Engine, block: u32) -> u32 {
    if block != 0 && is_node(e, block) {
        block
    } else {
        0
    }
}

/// `TESObjectREFR::GetRefNiNode` (JIP LN NVSE): the loaded data's root node, 0 without 3D.
pub(crate) fn ref_ni_node(e: &Engine, refr: u32) -> u32 {
    ptr(e, refr.wrapping_add(LOADED_DATA))
        .and_then(|loaded| rd32(e, loaded.wrapping_add(ROOT_NODE)))
        .unwrap_or(0)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), nvse/GameObjects.cpp: TESObjectREFR::GetNiNode
// (asm).
/// The root as seen: [`ref_ni_node`], but for the player in first person (flag +0x64A clear)
/// its first person node; 0 without loaded data.
pub(crate) fn ni_node(e: &Engine, refr: u32) -> u32 {
    let Some(loaded) = ptr(e, refr.wrapping_add(LOADED_DATA)) else {
        return 0;
    };
    let root = rd32(e, loaded.wrapping_add(ROOT_NODE)).unwrap_or(0);
    if rd32(e, refr.wrapping_add(REF_ID)) == Some(PLAYER_ID)
        && rd8(e, refr.wrapping_add(IS_3RD_PERSON)) == Some(0)
    {
        rd32(e, refr.wrapping_add(NODE_1ST_PERSON)).unwrap_or(0)
    } else {
        root
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), nvse/GameObjects.cpp: TESObjectREFR::GetNode
// (asm).
/// [`ni_node`] for an empty name, else the node named `name` under it (a block that is not a
/// node counts as none).
pub(crate) fn node(e: &Engine, refr: u32, name: &[u8]) -> u32 {
    let root = ni_node(e, refr);
    if root == 0 || name.is_empty() {
        return root;
    }
    only_node(e, get_block(e, root, name))
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), nvse/GameObjects.cpp: TESObjectREFR::GetNiBlock2
// (asm).
/// [`ref_ni_node`] for an empty name, else the block named `name` under it.
pub(crate) fn ni_block2(e: &Engine, refr: u32, name: &[u8]) -> u32 {
    let root = ref_ni_node(e, refr);
    if root == 0 || name.is_empty() {
        return root;
    }
    get_block(e, root, name)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), nvse/GameObjects.cpp: TESObjectREFR::GetNode2
// (asm).
/// [`ref_ni_node`] for an empty name, else the node named `name` under it.
pub(crate) fn node2(e: &Engine, refr: u32, name: &[u8]) -> u32 {
    let root = ref_ni_node(e, refr);
    if root == 0 || name.is_empty() {
        return root;
    }
    only_node(e, get_block(e, root, name))
}

// ---- Havok objects ----

/// The Havok object (`hkpWorldObject`) of a block's collision object: collision object +0x1C,
/// its world object +0x10, that one's Havok object +8. `None` when any is null.
pub(crate) fn havok_object(e: &Engine, block: u32) -> Option<u32> {
    let collision = ptr(e, block.wrapping_add(COLLISION_OBJECT))?;
    let world = ptr(e, collision.wrapping_add(WORLD_OBJECT))?;
    ptr(e, world.wrapping_add(HAVOK_OBJECT))
}

/// The collidable's broadphase type byte (+0x28): 1 a rigid body, 2 a phantom.
pub(crate) fn collision_type(e: &Engine, object: u32) -> Option<u8> {
    rd8(e, object.wrapping_add(COLLISION_TYPE))
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), nvse/GameObjects.cpp: TESObjectREFR::GetRigidBody
// (asm).
/// The rigid body of the block named `name` ([`ni_block2`]): its Havok object when that is a
/// rigid body (type 1), else 0.
pub(crate) fn rigid_body(e: &Engine, refr: u32, name: &[u8]) -> u32 {
    let block = ni_block2(e, refr, name);
    if block == 0 {
        return 0;
    }
    match havok_object(e, block) {
        Some(body) if collision_type(e, body) == Some(1) => body,
        _ => 0,
    }
}

/// `hkpRigidBody::IsMobile` (JIP LN NVSE): the motion type has bit 1 (types 2, 3, 6, 7).
pub(crate) fn body_is_mobile(e: &Engine, body: u32) -> bool {
    rd8(e, body.wrapping_add(MOTION_TYPE)).is_some_and(|t| t & 2 != 0)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), nvse/GameObjects.cpp:
// TESObjectREFR::GetCharacterController (asm).
/// An actor's character controller: its process (+0x68) at level 0 or 1 (byte +0x28), that
/// process' controller (+0x138); 0 for anything else.
pub(crate) fn character_controller(e: &Engine, refr: u32) -> u32 {
    if !is_actor(e, refr) {
        return 0;
    }
    let Some(process) = ptr(e, refr.wrapping_add(PROCESS)) else {
        return 0;
    };
    match rd8(e, process.wrapping_add(PROCESS_LEVEL)) {
        Some(level) if level <= 1 => rd32(e, process.wrapping_add(CHAR_CONTROLLER)).unwrap_or(0),
        _ => 0,
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/netimmerse.cpp: NiAVObject::GetParentRef
// (asm).
/// The reference of a scene block: the block and then each parent in turn, the first that is a
/// `BSFadeNode` with a reference (+0xCC). 0 for none (or a spent budget).
pub(crate) fn block_parent_ref(e: &Engine, block: u32, walk: &mut Walk) -> u32 {
    let mut object = block;
    for _ in 0..MAX_DEPTH * 2 {
        if object == 0 || !walk.charge() {
            return 0;
        }
        let next = rd32(e, object.wrapping_add(PARENT)).unwrap_or(0);
        if rd32(e, object) == Some(BS_FADE_NODE_VTBL) {
            if let Some(refr) = ptr(e, object.wrapping_add(FADE_NODE_REF)) {
                return refr;
            }
        }
        object = next;
    }
    0
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/havok.cpp: hkpWorldObject::GetParentNode
// (asm).
/// The scene object of a Havok object: the first property whose key is `bhkNiCollisionObject`'s
/// (its collision object's node, +8; none when the collision object is null) or `NiAVObject`'s
/// (the object itself).
fn parent_node(e: &Engine, object: u32, walk: &mut Walk) -> u32 {
    let (Some(data), Some(size)) = (
        rd32(e, object.wrapping_add(PROPERTIES)),
        rd32(e, object.wrapping_add(PROPERTIES + 4)),
    ) else {
        return 0;
    };
    let size = (size as i32).max(0) as u32;
    for i in 0..size.min(MAX_ITEMS) {
        if !walk.charge() {
            return 0;
        }
        let entry = data.wrapping_add(0x10 * i);
        match rd32(e, entry) {
            Some(KEY_COLLISION_OBJECT) => {
                return ptr(e, entry.wrapping_add(8))
                    .and_then(|c| rd32(e, c.wrapping_add(8)))
                    .unwrap_or(0);
            }
            Some(KEY_AV_OBJECT) => return rd32(e, entry.wrapping_add(8)).unwrap_or(0),
            Some(_) => {}
            None => return 0,
        }
    }
    0
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/havok.cpp: hkpWorldObject::GetParentRef
// (asm).
/// The reference a Havok object belongs to: [`block_parent_ref`] from its scene object. 0 for a
/// null object or none.
pub(crate) fn object_parent_ref(e: &Engine, object: u32, walk: &mut Walk) -> u32 {
    if object == 0 {
        return 0;
    }
    let node = parent_node(e, object, walk);
    block_parent_ref(e, node, walk)
}

/// The pointers of an `hkArray` (data at `at`, size at `at + 4`), at most [`MAX_ITEMS`]; an
/// unreadable item or a spent budget ends them.
fn array_items(e: &Engine, at: u32, walk: &mut Walk) -> Vec<u32> {
    let (Some(data), Some(size)) = (rd32(e, at), rd32(e, at.wrapping_add(4))) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for i in 0..((size as i32).max(0) as u32).min(MAX_ITEMS) {
        if !walk.charge() {
            break;
        }
        match rd32(e, data.wrapping_add(4 * i)) {
            Some(p) => out.push(p),
            None => break,
        }
    }
    out
}

/// The world objects of the bodies a simple shape phantom overlaps: each `hkCdBody*` less 0x10
/// (JIP's `hkCdBody::GetWorldObj`).
fn phantom_bodies(e: &Engine, phantom: u32, walk: &mut Walk) -> Vec<u32> {
    array_items(e, phantom.wrapping_add(PHANTOM_BODIES), walk)
        .into_iter()
        .map(|cd| cd.wrapping_sub(0x10))
        .collect()
}

/// Appends `body` to a contact list unless it already holds [`MAX_ITEMS`]; false when full.
fn push_contact(out: &mut Vec<u32>, body: u32) -> bool {
    if out.len() >= MAX_ITEMS as usize {
        return false;
    }
    out.push(body);
    true
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/netimmerse.cpp: NiNode::GetContactObjects.
/// The node's touching objects, then its child nodes' in order: a rigid body's master
/// constraints' other bodies (+8 of each 0x30-byte entry) and, when the slave list's data is not
/// null, each slave constraint's body (+0x14); a simple shape phantom's overlapped bodies.
fn node_contacts(e: &Engine, node: u32, depth: u32, walk: &mut Walk, out: &mut Vec<u32>) {
    if depth >= MAX_DEPTH || walk.spent() || out.len() >= MAX_ITEMS as usize {
        return;
    }
    if let Some(object) = havok_object(e, node) {
        if collision_type(e, object) == Some(1) {
            if let Some(masters) = ptr(e, object.wrapping_add(MASTERS)) {
                let count = rd16(e, object.wrapping_add(MASTERS_COUNT)).unwrap_or(0);
                for i in 0..u32::from(count) {
                    if !walk.charge() {
                        return;
                    }
                    match rd32(e, masters.wrapping_add(0x30 * i + 8)) {
                        Some(body) if push_contact(out, body) => {}
                        _ => break,
                    }
                }
            }
            if ptr(e, object.wrapping_add(SLAVES)).is_some() {
                for instance in array_items(e, object.wrapping_add(SLAVES), walk) {
                    if !push_contact(out, rd32(e, instance.wrapping_add(0x14)).unwrap_or(0)) {
                        return;
                    }
                }
            }
        } else if rd32(e, object) == Some(HKP_SIMPLE_SHAPE_PHANTOM_VTBL) {
            for body in phantom_bodies(e, object, walk) {
                if !push_contact(out, body) {
                    return;
                }
            }
        }
    }
    for child in child_nodes(e, node, walk) {
        if walk.spent() || out.len() >= MAX_ITEMS as usize {
            return;
        }
        node_contacts(e, child, depth + 1, walk, out);
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_misc_ref.h:
// GetRefrContactObjects.
/// What a reference with 3D touches: its trigger phantom's (`spPhantom` +0x18, when it is a
/// `bhkSimpleShapePhantom`) overlapped bodies, then its root node's contacts. Empty without 3D.
pub(crate) fn ref_contacts(e: &Engine, refr: u32, walk: &mut Walk) -> Vec<u32> {
    let mut out = Vec::new();
    let Some(loaded) = ptr(e, refr.wrapping_add(LOADED_DATA)) else {
        return out;
    };
    let Some(root) = ptr(e, loaded.wrapping_add(ROOT_NODE)) else {
        return out;
    };
    if let Some(phantom) = ptr(e, loaded.wrapping_add(PHANTOM)) {
        if rd32(e, phantom) == Some(BHK_SIMPLE_SHAPE_PHANTOM_VTBL) {
            if let Some(object) = ptr(e, phantom.wrapping_add(HAVOK_OBJECT)) {
                // One array: at most MAX_ITEMS, so the list cannot be full yet.
                out.extend(phantom_bodies(e, object, walk));
            }
        }
    }
    node_contacts(e, root, 0, walk, &mut out);
    out
}

/// What the contact commands walk: for an actor its controller's body under its feet (only when
/// the low byte of `iSupportCount` is 0) and its point collector's touched objects; for anything
/// else [`ref_contacts`]. `None` for an actor without a controller (JIP returns at once).
pub(crate) struct Contacts {
    pub(crate) under_feet: u32,
    pub(crate) bodies: Vec<u32>,
}

pub(crate) fn contacts(e: &Engine, refr: u32, walk: &mut Walk) -> Option<Contacts> {
    if is_actor(e, refr) {
        let ctrl = character_controller(e, refr);
        if ctrl == 0 {
            return None;
        }
        let under_feet = if rd8(e, ctrl.wrapping_add(SUPPORT_COUNT)) == Some(0) {
            rd32(e, ctrl.wrapping_add(SUPPORT_BODY)).unwrap_or(0)
        } else {
            0
        };
        Some(Contacts {
            under_feet,
            bodies: array_items(e, ctrl.wrapping_add(CONTACT_BODIES), walk),
        })
    } else {
        Some(Contacts {
            under_feet: 0,
            bodies: ref_contacts(e, refr, walk),
        })
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/netimmerse.cpp: NiNode::IsMovable (asm).
/// Whether the node's Havok object has motion type bit 1 (read whatever the object is), or any
/// child node is movable.
pub(crate) fn is_movable(e: &Engine, node: u32) -> bool {
    fn walk_node(e: &Engine, node: u32, depth: u32, walk: &mut Walk) -> bool {
        if depth >= MAX_DEPTH || walk.spent() {
            return false;
        }
        if havok_object(e, node).is_some_and(|o| body_is_mobile(e, o)) {
            return true;
        }
        child_nodes(e, node, walk)
            .into_iter()
            .any(|c| !walk.spent() && walk_node(e, c, depth + 1, walk))
    }
    walk_node(e, node, 0, &mut Walk::new())
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/netimmerse.cpp: NiNode::HasPhantom.
/// Whether the node's Havok object is a phantom (type 2), or any child node has one.
pub(crate) fn has_phantom(e: &Engine, node: u32) -> bool {
    fn walk_node(e: &Engine, node: u32, depth: u32, walk: &mut Walk) -> bool {
        if depth >= MAX_DEPTH || walk.spent() {
            return false;
        }
        if havok_object(e, node).is_some_and(|o| collision_type(e, o) == Some(2)) {
            return true;
        }
        child_nodes(e, node, walk)
            .into_iter()
            .any(|c| !walk.spent() && walk_node(e, c, depth + 1, walk))
    }
    walk_node(e, node, 0, &mut Walk::new())
}

/// `rcpss`: the processor's estimate of 1/x, as JIP's code gets it.
pub(crate) fn rcp_estimate(v: f32) -> f32 {
    #[cfg(target_arch = "x86_64")]
    {
        use std::arch::x86_64::{_mm_cvtss_f32, _mm_rcp_ss, _mm_set_ss};
        // SAFETY: SSE is part of every x86_64 processor.
        unsafe { _mm_cvtss_f32(_mm_rcp_ss(_mm_set_ss(v))) }
    }
    #[cfg(target_arch = "x86")]
    {
        use std::arch::x86::{_mm_cvtss_f32, _mm_rcp_ss, _mm_set_ss};
        // SAFETY: the port's 32-bit builds require SSE2.
        unsafe { _mm_cvtss_f32(_mm_rcp_ss(_mm_set_ss(v))) }
    }
    #[cfg(not(any(target_arch = "x86_64", target_arch = "x86")))]
    {
        1.0 / v
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/netimmerse.cpp: NiNode::GetBodyMass (asm).
/// `total` plus the node's rigid body's mass (the estimate of 1 over the inverse mass +0x1AC,
/// when its bits are not 0), then each child node's, depth first, summed in floats.
pub(crate) fn body_mass(e: &Engine, node: u32) -> f32 {
    fn walk_node(e: &Engine, node: u32, total: f32, depth: u32, walk: &mut Walk) -> f32 {
        if depth >= MAX_DEPTH || walk.spent() {
            return total;
        }
        let mut total = total;
        if let Some(body) = havok_object(e, node) {
            if collision_type(e, body) == Some(1) {
                if let Some(bits) = rd32(e, body.wrapping_add(MASS_INV)).filter(|&b| b != 0) {
                    total += rcp_estimate(f32::from_bits(bits));
                }
            }
        }
        for child in child_nodes(e, node, walk) {
            if walk.spent() {
                break;
            }
            total = walk_node(e, child, total, depth + 1, walk);
        }
        total
    }
    walk_node(e, node, 0.0, 0, &mut Walk::new())
}

// ---- the world ray cast ----

/// The game's `TES` (`g_TES`) and player (`g_thePlayer`) singletons.
pub(crate) const TES: u32 = 0x011d_ea10;
pub(crate) const PLAYER: u32 = 0x011d_ea3c;
/// `TES::Pick_ov2` (`00458440`, thiscall `(TES, record, byCell)`; the port's `tes_p2.rs`).
pub(crate) const PICK: u32 = 0x0045_8440;
/// `TES::pGridCellA` (+8) and the interior cell (+0x34, the cell `Pick_ov2` picks in).
pub(crate) const TES_GRID: u32 = 0x08;
pub(crate) const TES_INTERIOR: u32 = 0x34;
/// `kUnitConv` (game units to Havok's, 0.142874...) and `PS_HKUnitCnvrt` (back, 6.9991...).
pub(crate) fn to_havok() -> f32 {
    f32::from_bits(0x3e12_4dd2)
}
pub(crate) fn from_havok() -> f32 {
    f32::from_bits(0x40df_f8d6)
}
/// The record's size (JIP's `RayCastData`) and its fields JIP sets or reads.
const RECORD_SIZE: u32 = 0xb0;
const RECORD_FROM: u32 = 0x00;
const RECORD_TO: u32 = 0x10;
const RECORD_FILTER: u32 = 0x24;
const RECORD_FRACTION: u32 = 0x40;

/// A cast's ends in Havok units (four lanes each, as JIP's record holds them) and what the pick
/// answered.
pub(crate) struct Cast {
    pub(crate) from: [f32; 4],
    pub(crate) to: [f32; 4],
    pub(crate) fraction: f32,
    pub(crate) object: u32,
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/jip_core.cpp: _GetRayCastObject and
// GetRayCastObject (asm).
/// The game's pick along a block's forward axis: from `pos` (the block's world translation and
/// the word after it, the scale, as JIP's 16-byte load takes them) times `kUnitConv` in three
/// lanes (the fourth `w * 0`), to that plus the rotation's column 1 (`rot[1]`, `rot[4]`,
/// `rot[7]`) times `range` times `kUnitConv`; the hit fraction set to 1, the words +0x44 and
/// +0x50 to -1, the byte +0x20, words +0x70, +0x80 and 32 bytes from +0x90 to 0; the filter
/// word the player's controller's shape phantom's collision filter info with its low 16 bits
/// replaced by `layer` (6 when negative) and 0x7F. Then `TES::Pick_ov2(record, 1)`.
///
/// JIP reads the player's process, controller and phantom without a check; without any of them
/// (or without the game's `TES`) the port casts nothing and answers `None`. The rest of JIP's
/// record is its stack's leftovers; the port zeroes it.
pub(crate) fn ray_cast(
    e: &mut Engine,
    pos: [f32; 4],
    rot: [f32; 9],
    range: f32,
    layer: i32,
) -> Option<Cast> {
    let tes = ptr(e, TES)?;
    let filter = ptr(e, PLAYER)
        .and_then(|p| ptr(e, p.wrapping_add(PROCESS)))
        .and_then(|p| ptr(e, p.wrapping_add(CHAR_CONTROLLER)))
        .and_then(|c| ptr(e, c.wrapping_add(SHAPE_PHANTOM)))
        .and_then(|s| ptr(e, s.wrapping_add(HAVOK_OBJECT)))
        .and_then(|h| rd32(e, h.wrapping_add(FILTER_INFO)))?;
    let layer = if layer < 0 { 6 } else { layer as u32 };
    let filter = (filter & 0xffff_0000) | (layer & 0x7f);
    let k = [to_havok(), to_havok(), to_havok(), 0.0];
    let from: [f32; 4] = std::array::from_fn(|i| pos[i] * k[i]);
    let dir = [rot[1], rot[4], rot[7], 0.0];
    let r = [range, range, range, 0.0];
    let to: [f32; 4] = std::array::from_fn(|i| (dir[i] * r[i]) * k[i] + from[i]);
    e.with_stack(RECORD_SIZE + 0x10, |e, frame| {
        let record = (frame.addr() + 0xf) & !0xf;
        for i in (0..RECORD_SIZE).step_by(4) {
            e.mem.set_u32(record + i, 0);
        }
        for i in 0..4 {
            e.mem.set_f32(record + RECORD_FROM + 4 * i as u32, from[i]);
            e.mem.set_f32(record + RECORD_TO + 4 * i as u32, to[i]);
        }
        e.mem.set_f32(record + RECORD_FRACTION, 1.0);
        e.mem.set_u32(record + 0x44, u32::MAX);
        e.mem.set_u32(record + 0x50, u32::MAX);
        e.mem.set_u32(record + RECORD_FILTER, filter);
        let object = e.call(PICK, &args![tes, record, 1u8]).u32();
        let fraction = e.mem.f32(record + RECORD_FRACTION);
        Some(Cast {
            from,
            to,
            fraction,
            object,
        })
    })
}

/// The cast's hit point in game units, `(from + (to - from) * fraction) * PS_HKUnitCnvrt` (the
/// fourth lane times 0), as `RayCastCoords` and `GetRayCastMaterial` compute it.
pub(crate) fn hit_point(cast: &Cast) -> [f32; 4] {
    let k = [from_havok(), from_havok(), from_havok(), 0.0];
    std::array::from_fn(|i| (cast.from[i] + (cast.to[i] - cast.from[i]) * cast.fraction) * k[i])
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/jip_core.cpp: NiVector4::RayCastCoords
// (asm) and nvse/GameData.cpp: GridCellArray::GetCellAtCoord (asm).
/// The cast's hit point in game units, `(from + (to - from) * fraction) * PS_HKUnitCnvrt` (the
/// fourth lane times 0), and whether it counts as found: an object was hit, or the game is in an
/// interior (JIP's flag test then answers true), or an exterior grid cell holds the point (its x
/// and y truncated, shifted right 12, then JIP's index into the grid: `y' + x' + (x' << half)`
/// with `x' = x - centreX + half`, `y' = y - centreY + half`, each below the dimension).
pub(crate) fn hit_coords(e: &Engine, cast: &Cast) -> ([f32; 4], bool) {
    let coords = hit_point(cast);
    if cast.object != 0 {
        return (coords, true);
    }
    let Some(tes) = ptr(e, TES) else {
        return (coords, false);
    };
    if rd32(e, tes.wrapping_add(TES_INTERIOR)).is_some_and(|c| c != 0) {
        return (coords, true);
    }
    let Some(grid) = ptr(e, tes.wrapping_add(TES_GRID)) else {
        return (coords, false);
    };
    let truncate = |v: f32| super::cvttss2si(v) >> 12;
    let (x, y) = (truncate(coords[0]) as i16, truncate(coords[1]) as i16);
    let (Some(cx), Some(cy), Some(dim), Some(cells)) = (
        rd32(e, grid.wrapping_add(4)),
        rd32(e, grid.wrapping_add(8)),
        rd32(e, grid.wrapping_add(0xc)),
        rd32(e, grid.wrapping_add(0x10)),
    ) else {
        return (coords, false);
    };
    let half = dim >> 1;
    let gx = (i32::from(x) as u32).wrapping_sub(cx).wrapping_add(half);
    if gx >= dim {
        return (coords, false);
    }
    let gy = (i32::from(y) as u32).wrapping_sub(cy).wrapping_add(half);
    if gy >= dim {
        return (coords, false);
    }
    let index = gy
        .wrapping_add(gx)
        .wrapping_add(gx.wrapping_shl(half & 0xff));
    let cell = rd32(e, cells.wrapping_add(index.wrapping_mul(4))).unwrap_or(0);
    (coords, cell != 0)
}
