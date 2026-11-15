//! `functions_jip/jip_fn_misc_ref.h` (JIP LN NVSE): the commands that WRITE a reference's scene
//! graph, on the node lookup of `nif_nodes.rs` (the read-only ones are in
//! `jip_fn_misc_ref_nodes.rs`): a block's translation, rotation, scale and flags; a geometry
//! block's material values (one block, or every one below the root with `*`); a texture
//! transform controller's key; a block renamed or removed from its parent; the controller
//! manager's sequences (their frequency read and set, one played).
//!
//! What each writes and calls, as JIP does (every game call's words in AUDIT-lane-F2.md):
//! - `SetNifBlockTranslation`, `SetNifBlockRotation`, `SetNifBlockScale`: the block's local
//!   transform (+0x34 rotation, +0x58 translation, +0x64 scale); then for a node, unless the
//!   reference is an actor (vtable +0x100 answering `008D0360`), JIP's `NiNode::ResetCollision`
//!   (lane H's `ref_move::reset_collision`); then the block's own `UpdateDownwardPass` (vtable
//!   +0xA4, `thiscall` (block, update data, 0), `RET 8` in every class: `NiNode` `00A5DD70`,
//!   `NiAVObject` and the geometry classes `00A59F90`, `BSFadeNode` `00B4DED0` (a jump to
//!   `00A5DD70`), `NiBillboardNode` `00A7DAE0`, `NiSwitchNode`/`NiLODNode` `00A94040`,
//!   `BSMultiBoundNode` `00C3A5B0`, `BSFaceGenNiNode` `00663A70`, `BSTreeNode` `0066BF80`,
//!   `NiParticleSystem` `00C1AD90`) with JIP's `kNiUpdateData` (12 zero bytes).
//! - `SetNifBlockFlag`: one bit (0-26) of the flags (+0x30); no update (JIP calls none).
//! - `SetMaterialPropertyValue`: JIP's `NiMaterialProperty::SetTraitValue` (specular +0x1C,
//!   emissive +0x28, glossiness +0x38, alpha +0x3C, emit multiplier +0x40) and its recursive
//!   `NiNode::SetMaterialPropValueRecurse`; no game call.
//! - `SetTextureTransformKey`: the key's value in the controller's float data, then the
//!   controller's `Update` (vtable +0x94; the class is checked to be
//!   `NiTextureTransformController` `01098184`, whose +0x94 is `00A46490`, `RET 4`).
//! - `RenameNifBlock`: JIP's `NiObjectNET::SetName`: the new name's fixed string from the
//!   string table (JIP's `GetNiFixedString`; JIP replaces the game's `00A5B690` with its own
//!   pool, stage 3.9 G43, which waits: the port calls the game's own `00A5B690`, `cdecl` (text),
//!   which interns the text and counts it up exactly as JIP's pool does, the count at -8), the
//!   old name's count taken down (`lock dec [name-8]`, as JIP; neither table frees it there).
//! - `RemoveNifBlock`: the parent's `RemoveObject` (vtable +0xE8, `00A5D8F0` in every node
//!   class, `thiscall` (parent, block), `RET 4`).
//! - `PlayAnimSequence`: each other sequence not named `Idle...` deactivated (vtable +0x8C,
//!   `NiControllerSequence` `01096584`: `00A35030`, `thiscall` (sequence, 0.0, 0), `RET 8`), then
//!   JIP's `NiControllerSequence::Play`: `00A34F20` (`thiscall` (sequence, 0x64, 1, 1.0, 0, 0,
//!   0), `RET 0x18`), on success the offset (+0x48) set to `-FLT_MAX` and the manager's (+0x40)
//!   active bit (8 in the byte +8) set; then `MarkModified(0x10000000)`.
//!
//! Threads: every command runs on the thread that runs the script (the main thread), as JIP's
//! do, with no lock and no task: JIP takes none (it writes the nodes and calls the update in
//! place). With real threads on, the drawing and loading threads may read these nodes at the
//! same time; JIP has that race in the original too, and the port does exactly what JIP does.
//! `RemoveNifBlock` may free the block (the parent drops its reference), as JIP's does.
//!
//! Left out, each waiting for what it names (also in AUDIT-lane-F2.md):
//! - `SetNifBlockTranslation`'s point light branch (an `NiPointLight` whose JIP-only flag byte
//!   +0x9F has bit 1 gets its vector +0x100 set): the flag and the vector are JIP's own fields
//!   past the game's object, there only once every point light is allocated at 0x110 bytes
//!   (stage 3.9 H03, with JIP's light commands); no light the port makes has them, and writing
//!   +0x100 of a game-sized light would write past it.
//! - `ModelHasBlock`: reads the nodes `InsertNode`/`AttachModel` record (`s_insertNodeMap`) and
//!   loads a base form's model (`ModelLoader::LoadModel`): waits for those commands and JIP's
//!   3D-load hooks (stage 3.9 H09/H10).
//! - `InsertNode`, `AttachModel`, `AttachLight`, `RemoveLight`, `AttachLine`,
//!   `AttachExtraCamera`, `ProjectExtraCamera`: JIP's per-reference lists re-applied when 3D
//!   reloads (H09/H10), its light records and hooks (H03-H07, H12), its camera list.
//!
//! Safety: as `nif_nodes.rs`: every guest word read checked, every walk bounded, every address
//! sum wrapping; a write to memory that cannot be written is skipped, and the update after it
//! with it. Where JIP would follow a null pointer (a root's missing parent, a null sequence in
//! the list) the port does nothing there (listed as deliberate differences in the audit).

use super::jip_fn_misc_ref::{from_euler_pry, multiply_vector};
use super::jip_fn_misc_ref_nodes::{
    block_name, ALPHA, EMISSIVE, EMIT_MULT, GLOSSINESS, MATERIAL_PROPERTY, SPECULAR,
};
use super::ni_matrix::{from_2_points, from_euler_pry_inv, multiply_matrices};
use super::nif_nodes as nif;
use super::{extra, havok_refs, is_actor, Handlers, FLT_PI_D_180};
use engine_core::prelude::*;
use nvse::commands::Read;
use nvse::{handled, CommandArgs};
use std::cmp::Ordering;

#[rustfmt::skip]
pub static COMMANDS: Handlers = &[
    (0x278d, set_material_property_value),
    (0x284c, set_nif_block_translation),
    (0x284e, set_nif_block_rotation),
    (0x2850, set_nif_block_scale),
    (0x2852, set_nif_block_flag),
    (0x285f, get_anim_sequence_frequency),
    (0x2860, set_anim_sequence_frequency),
    (0x28fb, set_texture_transform_key),
    (0x2900, rename_nif_block),
    (0x2901, remove_nif_block),
    (0x2904, play_anim_sequence),
];

/// `NiAVObject::UpdateDownwardPass` (vtable +0xA4) and `NiNode::RemoveObject` (+0xE8).
pub(crate) const UPDATE_DOWNWARD_PASS_SLOT: u32 = 0xa4;
pub(crate) const REMOVE_OBJECT_SLOT: u32 = 0xe8;
/// JIP's `kNiUpdateData`: a static of 12 bytes its constructor leaves zero.
const UPDATE_DATA_SIZE: u32 = 0xc;
/// The game's fixed-string maker (`NiGlobalStringTable::AddString`, `00A5B690`, `cdecl` (text)).
pub(crate) const ADD_STRING: u32 = 0x00a5_b690;
/// A fixed string's count, the word before its length, 8 bytes before the text (`00A5B690`
/// writes 1 there for a new string and counts a found one up with `InterlockedIncrement`).
const STRING_COUNT: u32 = 8;
/// `NiObjectNET::m_controller` (+0xC: `NiTimeController::SetTarget` `00A6D450` walks the
/// target's controllers from +0xC); `NiTimeController::nextCtrl` (+0x30, the same walk).
const CONTROLLER: u32 = 0x0c;
const NEXT_CONTROLLER: u32 = 0x30;
/// `kVtbl_NiControllerManager` (`0109619C`), `kVtbl_NiControllerSequence` (`01096584`),
/// `kVtbl_NiTextureTransformController` (`01098184`), `kVtbl_NiFloatInterpolator`
/// (`01096F34`), `kVtbl_NiFloatData` (`0102DABC`): each confirmed by its RTTI name in the exe.
pub(crate) const CONTROLLER_MANAGER_VTBL: u32 = 0x0109_619c;
pub(crate) const TEXTURE_TRANSFORM_CONTROLLER_VTBL: u32 = 0x0109_8184;
pub(crate) const FLOAT_INTERPOLATOR_VTBL: u32 = 0x0109_6f34;
pub(crate) const FLOAT_DATA_VTBL: u32 = 0x0102_dabc;
/// `NiControllerManager::sequences` (an `NiTArray` at +0x34: data +0x38, its size, the first
/// free entry, u16 +0x3E: `NiControllerManager::LinkObject` `00A2F6E0` passes `this+0x34` to the
/// array's set-size and add; the add `00A2EC90` reads data +4 and the size +0xA).
const SEQUENCES: u32 = 0x38;
const SEQUENCE_COUNT: u32 = 0x3e;
/// `NiControllerSequence`: name +8 (`LoadBinary` `00A33E40` loads it first), frequency +0x28
/// (the same), manager +0x40 (`NiControllerManager::LinkObject` sets it), offset +0x48 (JIP's
/// `Play` writes it; JIP LN NVSE's layout only); `Deactivate` vtable +0x8C.
const SEQUENCE_NAME: u32 = 0x08;
const SEQUENCE_FREQUENCY: u32 = 0x28;
const SEQUENCE_MANAGER: u32 = 0x40;
const SEQUENCE_OFFSET: u32 = 0x48;
const DEACTIVATE_SLOT: u32 = 0x8c;
/// `NiControllerSequence::Activate` (`00A34F20`), as JIP's `Play` calls it.
pub(crate) const ACTIVATE: u32 = 0x00a3_4f20;
/// `NiTimeController::flags` (u16 +8; JIP sets the active bit 8 in its low byte).
const CONTROLLER_FLAGS: u32 = 0x08;
/// `-FLT_MAX` (`0xFF7FFFFF`), JIP's `Play`'s offset.
const NEG_FLT_MAX: u32 = 0xff7f_ffff;
/// The four bytes `Idle` as one word (JIP's `'eldI'`).
const IDLE: u32 = u32::from_le_bytes(*b"Idle");
/// `MarkModified`'s flag in `PlayAnimSequence` (JIP's `0x10000000`).
const CHANGED_ANIMATION: u32 = 0x1000_0000;
/// `NiGeometry`'s texturing property (type 5: `NiTexturingProperty`'s type function `00E9E670`
/// answers 5; `UpdatePropertiesDownward` `00A80820` keeps type t at +0x9C + 4t): +0xB0.
const TEXTURING_PROPERTY: u32 = 0xb0;
/// `NiSingleInterpController::interpolator` (+0x34: the texture transform controller's
/// `LinkObject` `00A42D00` links it); `NiFloatInterpolator::data` (+0x10, its `LoadBinary`
/// `00A3C800`); `NiFloatData` key count +8, keys +0xC (`NiFloatData::ReplaceAnim` `00A3CDC0`).
const INTERPOLATOR: u32 = 0x34;
const FLOAT_DATA: u32 = 0x10;
const KEY_COUNT: u32 = 0x08;
const KEYS: u32 = 0x0c;
/// JIP's `InterpKey<float>` (time, value: 8 bytes; JIP indexes with it whatever the key type)
/// and the value's place in it.
const KEY_SIZE: u32 = 8;
const KEY_VALUE: u32 = 4;
/// `NiTimeController::Update` (vtable +0x94).
const UPDATE_SLOT: u32 = 0x94;
/// `Actor::lifeState` (+0x108; 1 dying).
const LIFE_STATE: u32 = 0x108;
const DYING: u32 = 1;
/// JIP's sequence name buffers (`char seqName[0x40]`, `PlayAnimSequence`'s `[0x60]`).
const NAME_BUFFER: usize = 0x40;
const SEQUENCE_BUFFER: usize = 0x60;

// ---- checked writes and calls ----

/// The word written at `at`; false when it cannot be written.
fn wr32(e: &Engine, at: u32, v: u32) -> bool {
    e.mem.try_write(at, &v.to_le_bytes()).is_ok()
}

/// The floats written from `at` (all or none: the span is checked by one write).
fn wr_floats(e: &Engine, at: u32, v: &[f32]) -> bool {
    let bytes: Vec<u8> = v.iter().flat_map(|x| x.to_le_bytes()).collect();
    e.mem.try_write(at, &bytes).is_ok()
}

/// The virtual function at `slot` of `object`'s vtable called with `args` after `this`; `None`
/// (nothing called) when the vtable or the slot cannot be read or holds 0.
fn vcall(e: &mut Engine, object: u32, slot: u32, args: &[u32]) -> Option<Ret> {
    let vtbl = nif::ptr(e, object)?;
    nif::ptr(e, vtbl.wrapping_add(slot))?;
    Some(e.vcall(object, slot, args))
}

/// `block->UpdateDownwardPass(kNiUpdateData, 0)` (vtable +0xA4), the update data's 12 zero bytes
/// in a frame of the stack.
fn update_downward(e: &mut Engine, block: u32) {
    e.with_stack(UPDATE_DATA_SIZE, |e, data| {
        let data = data.addr();
        for i in 0..3 {
            e.mem.set_u32(data + 4 * i, 0);
        }
        vcall(e, block, UPDATE_DOWNWARD_PASS_SLOT, &[data, 0]);
    });
}

/// After a transform write: JIP's `ResetCollision` for a node unless the reference is an actor,
/// then the block's update.
fn after_transform(e: &mut Engine, refr: u32, block: u32) {
    if nif::is_node(e, block) && !is_actor(e, refr) {
        super::ref_move::reset_collision(e, block);
    }
    update_downward(e, block);
}

/// The named block of a `SetNifBlock*` command: argument 0 (empty: none, JIP's `blockName[0]`
/// test), under the root `pc_node` chooses ([`nif::nif_block`]).
fn set_block(e: &Engine, a: &CommandArgs, read: &Read, pc_node: u32, command: &str) -> u32 {
    let name = block_name(e, read, 0, command);
    if name.is_empty() {
        return 0;
    }
    nif::nif_block(e, a.this_obj, pc_node, &name)
}

// ---- transforms ----

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_misc_ref.h:
// Cmd_SetNifBlockTranslation_Execute.
/// The block's local translation: transform 0 set to the three floats, 1 moved by them, any
/// other moved by them turned by the block's world rotation (`MultiplyVector`, the word after
/// the matrix the world translation's x); then [`after_transform`].
fn set_nif_block_translation(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if !read.ok || a.this_obj == 0 {
        return handled(true);
    }
    let t = [read.float(1, 0.0), read.float(2, 0.0), read.float(3, 0.0)];
    let (pc_node, transform) = (read.int(4, 0), read.int(5, 0));
    let block = set_block(e, &a, &read, pc_node, "SetNifBlockTranslation");
    if block == 0 {
        return handled(true);
    }
    let at = block.wrapping_add(nif::LOCAL_TRANSLATE);
    let v = match transform {
        0 => t,
        _ => {
            let Some(local) = nif::floats3(e, at) else {
                return handled(true);
            };
            let add = if transform == 1 {
                t
            } else {
                let world = block.wrapping_add(nif::WORLD_ROTATE);
                let (Some(m), Some(after)) = (
                    nif::matrix(e, world),
                    nif::rdf32(e, block.wrapping_add(nif::WORLD_TRANSLATE)),
                ) else {
                    return handled(true);
                };
                multiply_vector(&m, after, t)
            };
            [local[0] + add[0], local[1] + add[1], local[2] + add[2]]
        }
    };
    if wr_floats(e, at, &v) {
        after_transform(e, a.this_obj, block);
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/Ni_types.cpp:
// NiMatrix33::MultiplyMatricesInv (asm).
/// `a` replaced by `b` transposed times `a`: each row i of the result is, lane by lane as the asm's
/// `mulps`/`addps`, `(b[i]*a row 0 + b[3+i]*a row 1) + b[6+i]*a row 2`.
pub(crate) fn multiply_matrices_inv(a: &[f32; 9], b: &[f32; 9]) -> [f32; 9] {
    std::array::from_fn(|k| {
        let (i, j) = (k / 3, k % 3);
        (b[i] * a[j] + b[3 + i] * a[3 + j]) + b[6 + i] * a[6 + j]
    })
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_misc_ref.h:
// Cmd_SetNifBlockRotation_Execute.
/// The block's local rotation from the three angles (degrees, each times `FltPId180` as
/// `GET_PS(8)`'s lanes): transform 0 (and any above 5) `FromEulerPRY`; 1 the rotation times
/// `FromEulerPRYInv` (`Rotate`); 2 and 3 `FromEulerPRYInv`; 4 `FromEulerPRYInv` of the angles
/// plus the world rotation's `ToEulerPRYInv`, then `MultiplyMatricesInv` by the parent's world
/// rotation; 5 `From2Points` from the world translation to the three floats as a point (not
/// degrees), then the same. Then [`after_transform`].
fn set_nif_block_rotation(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if !read.ok || a.this_obj == 0 {
        return handled(true);
    }
    let rot = [read.float(1, 0.0), read.float(2, 0.0), read.float(3, 0.0)];
    let (transform, pc_node) = (read.int(4, 0), read.int(5, 0));
    let block = set_block(e, &a, &read, pc_node, "SetNifBlockRotation");
    if block == 0 {
        return handled(true);
    }
    let rad = rot.map(|v| v * FLT_PI_D_180);
    let local_at = block.wrapping_add(nif::LOCAL_ROTATE);
    let m = match transform {
        1 => {
            let Some(local) = nif::matrix(e, local_at) else {
                return handled(true);
            };
            multiply_matrices(&local, &from_euler_pry_inv(rad))
        }
        2 | 3 => from_euler_pry_inv(rad),
        4 | 5 => {
            // JIP reads the parent's world rotation unchecked (a root has none: the game
            // stops); the port leaves the block as it is.
            let parent_world = nif::ptr(e, block.wrapping_add(nif::PARENT))
                .and_then(|p| nif::matrix(e, p.wrapping_add(nif::WORLD_ROTATE)));
            let Some(parent_world) = parent_world else {
                return handled(true);
            };
            let m = if transform == 4 {
                let Some(world) = nif::matrix(e, block.wrapping_add(nif::WORLD_ROTATE)) else {
                    return handled(true);
                };
                let now = nif::to_euler_pry_inv(&world);
                from_euler_pry_inv([rad[0] + now[0], rad[1] + now[1], rad[2] + now[2]])
            } else {
                let Some(at) = nif::floats3(e, block.wrapping_add(nif::WORLD_TRANSLATE)) else {
                    return handled(true);
                };
                from_2_points(at, rot)
            };
            multiply_matrices_inv(&m, &parent_world)
        }
        _ => from_euler_pry(rad),
    };
    if wr_floats(e, local_at, &m) {
        after_transform(e, a.this_obj, block);
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_misc_ref.h:
// Cmd_SetNifBlockScale_Execute.
/// The block's local scale (+0x64), then [`after_transform`].
fn set_nif_block_scale(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if !read.ok || a.this_obj == 0 {
        return handled(true);
    }
    let scale = read.float(1, 0.0);
    let block = set_block(e, &a, &read, read.int(2, 0), "SetNifBlockScale");
    if block != 0 && wr32(e, block.wrapping_add(nif::LOCAL_SCALE), scale.to_bits()) {
        after_transform(e, a.this_obj, block);
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_misc_ref.h:
// Cmd_SetNifBlockFlag_Execute.
/// Flag bit `flagID` (0-26) of the block's flags (+0x30) set (third argument not 0) or cleared.
/// The empty name is the root here (JIP does not test it).
fn set_nif_block_flag(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if !read.ok || a.this_obj == 0 {
        return handled(true);
    }
    let (flag, set) = (read.int(1, 0), read.int(2, 0) != 0);
    if flag > 26 {
        return handled(true);
    }
    let name = block_name(e, &read, 0, "SetNifBlockFlag");
    let block = nif::nif_block(e, a.this_obj, read.int(3, 0), &name);
    if block == 0 {
        return handled(true);
    }
    let at = block.wrapping_add(nif::FLAGS);
    if let Some(flags) = nif::rd32(e, at) {
        let flags = if set {
            flags | (1 << flag)
        } else {
            flags & !(1 << flag)
        };
        wr32(e, at, flags);
    }
    handled(true)
}

// ---- materials ----

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/netimmerse.cpp:
// NiMaterialProperty::SetTraitValue.
/// Trait 0-2 specular, 3-5 emissive (red, green, blue), 6 glossiness, 7 alpha, any other the
/// emit multiplier, set to `value`.
fn set_trait_value(e: &Engine, material: u32, trait_id: u32, value: f32) {
    let at = match trait_id {
        0..=2 => SPECULAR + 4 * trait_id,
        3..=5 => EMISSIVE + 4 * (trait_id - 3),
        6 => GLOSSINESS,
        7 => ALPHA,
        _ => EMIT_MULT,
    };
    wr32(e, material.wrapping_add(at), value.to_bits());
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/netimmerse.cpp:
// NiNode::SetMaterialPropValueRecurse (asm).
/// Each child of `node` in order: a node searched below first (depth first), else a geometry
/// block's material (+0xA4) given the value. `depth` the children's level below the walk's
/// start; bounded as `nif_nodes.rs`' walks.
fn set_material_recurse(
    e: &Engine,
    node: u32,
    trait_id: u32,
    value: f32,
    depth: u32,
    walk: &mut nif::Walk,
) {
    if nif::too_deep(depth) {
        return;
    }
    nif::each_child(e, node, walk, |walk, child| {
        if nif::is_node(e, child) {
            set_material_recurse(e, child, trait_id, value, depth + 1, walk);
        } else if nif::is_geometry(e, child) {
            if let Some(material) = nif::ptr(e, child.wrapping_add(MATERIAL_PROPERTY)) {
                set_trait_value(e, material, trait_id, value);
            }
        }
        true
    });
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_misc_ref.h:
// Cmd_SetMaterialPropertyValue_Execute.
/// A trait (0-8) of the named geometry block's material set to the value (not below 0; a NaN is
/// not taken), or with the name `*` of every geometry block below the root (`GetNiNode`). The
/// name goes to `NiNode::GetBlock` (an empty one finds nothing).
fn set_material_property_value(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if !read.ok || a.this_obj == 0 {
        return handled(true);
    }
    let name = block_name(e, &read, 0, "SetMaterialPropertyValue");
    let (trait_id, value) = (read.int(1, 0), read.float(2, 0.0));
    // `value >= 0` as JIP's float compare: false for a NaN, true for -0.
    let at_least_0 = matches!(
        value.partial_cmp(&0.0),
        Some(Ordering::Greater | Ordering::Equal)
    );
    if trait_id > 8 || !at_least_0 {
        return handled(true);
    }
    let root = nif::ni_node(e, a.this_obj);
    if root == 0 {
        return handled(true);
    }
    if name == b"*" {
        set_material_recurse(e, root, trait_id, value, 1, &mut nif::Walk::new());
        return handled(true);
    }
    let block = nif::get_block(e, root, &name);
    if block != 0 && nif::is_geometry(e, block) {
        if let Some(material) = nif::ptr(e, block.wrapping_add(MATERIAL_PROPERTY)) {
            set_trait_value(e, material, trait_id, value);
        }
    }
    handled(true)
}

// ---- texture transform keys ----

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_misc_ref.h:
// Cmd_SetTextureTransformKey_Execute.
/// The value of key `keyIndex` of the float data behind the named geometry block's texturing
/// property's controller number `ctrlIndex` (counted along the controller list), when that is a
/// texture transform controller with a float interpolator and float data; then the controller's
/// `Update(kNiUpdateData)`. The list walk is bounded ([`nif::MAX_VISITS`] steps).
fn set_texture_transform_key(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if !read.ok || a.this_obj == 0 {
        return handled(true);
    }
    let name = block_name(e, &read, 0, "SetTextureTransformKey");
    let (mut index, key, value) = (read.int(1, 0), read.int(2, 0), read.float(3, 0.0));
    let block = nif::ni_block(e, a.this_obj, &name);
    if block == 0 || !nif::is_geometry(e, block) {
        return handled(true);
    }
    let Some(texturing) = nif::ptr(e, block.wrapping_add(TEXTURING_PROPERTY)) else {
        return handled(true);
    };
    let mut ctrl = nif::rd32(e, texturing.wrapping_add(CONTROLLER)).unwrap_or(0);
    let mut walk = nif::Walk::new();
    while ctrl != 0 && index != 0 {
        if !walk.step() {
            return handled(true);
        }
        index -= 1;
        ctrl = nif::rd32(e, ctrl.wrapping_add(NEXT_CONTROLLER)).unwrap_or(0);
    }
    if ctrl == 0 || nif::rd32(e, ctrl) != Some(TEXTURE_TRANSFORM_CONTROLLER_VTBL) {
        return handled(true);
    }
    let Some(interp) = nif::ptr(e, ctrl.wrapping_add(INTERPOLATOR)) else {
        return handled(true);
    };
    if nif::rd32(e, interp) != Some(FLOAT_INTERPOLATOR_VTBL) {
        return handled(true);
    }
    let Some(data) = nif::ptr(e, interp.wrapping_add(FLOAT_DATA)) else {
        return handled(true);
    };
    if nif::rd32(e, data) != Some(FLOAT_DATA_VTBL)
        || nif::rd32(e, data.wrapping_add(KEY_COUNT)).map_or(true, |n| key >= n)
    {
        return handled(true);
    }
    let Some(keys) = nif::rd32(e, data.wrapping_add(KEYS)) else {
        return handled(true);
    };
    let at = keys
        .wrapping_add(key.wrapping_mul(KEY_SIZE))
        .wrapping_add(KEY_VALUE);
    if wr32(e, at, value.to_bits()) {
        e.with_stack(UPDATE_DATA_SIZE, |e, upd| {
            let upd = upd.addr();
            for i in 0..3 {
                e.mem.set_u32(upd + 4 * i, 0);
            }
            vcall(e, ctrl, UPDATE_SLOT, &[upd]);
        });
    }
    handled(true)
}

// ---- names and removal ----

/// The fixed string at `s` counted down (`lock dec [s-8]`); nothing for 0 or an unreadable
/// count.
fn release_name(e: &Engine, s: u32) {
    if s == 0 {
        return;
    }
    if let Some(count) = e.mem.atomic_u32(s.wrapping_sub(STRING_COUNT)) {
        count.fetch_sub(1, std::sync::atomic::Ordering::SeqCst);
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_misc_ref.h:
// Cmd_RenameNifBlock_Execute, and internal/netimmerse.cpp: NiObjectNET::SetName (asm).
/// The block (`GetNifBlock`; the empty name the root) renamed: the new name's fixed string from
/// the game's string table; when it is the block's name already, the count it was given back;
/// else the old name's count down and the new one stored. Result 1.
fn rename_nif_block(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if !read.ok || a.this_obj == 0 {
        return handled(true);
    }
    let name = block_name(e, &read, 0, "RenameNifBlock");
    let new_name = block_name(e, &read, 1, "RenameNifBlock");
    let block = nif::nif_block(e, a.this_obj, read.int(2, 0), &name);
    if block == 0 {
        return handled(true);
    }
    let name_at = block.wrapping_add(nif::NAME);
    let Some(old) = nif::rd32(e, name_at) else {
        return handled(true);
    };
    let s = super::with_guest_text(e, &new_name, |e, text| e.call(ADD_STRING, &[text]).eax);
    if s == old {
        release_name(e, s);
    } else if wr32(e, name_at, s) {
        release_name(e, old);
    } else {
        // nv-rs's own: the name could not be stored; the new string's count goes back.
        release_name(e, s);
        return handled(true);
    }
    a.set_result(e, 1.0);
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_misc_ref.h:
// Cmd_RemoveNifBlock_Execute.
/// The block (`GetNifBlock`) taken from its parent (`RemoveObject`, vtable +0xE8). Result 1.
/// JIP calls through the parent unchecked (a root has none: the game stops); the port does
/// nothing without a parent that is a node.
fn remove_nif_block(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if !read.ok || a.this_obj == 0 {
        return handled(true);
    }
    let name = block_name(e, &read, 0, "RemoveNifBlock");
    let block = nif::nif_block(e, a.this_obj, read.int(1, 0), &name);
    if block == 0 {
        return handled(true);
    }
    let Some(parent) = nif::ptr(e, block.wrapping_add(nif::PARENT)) else {
        return handled(true);
    };
    if !nif::is_node(e, parent) {
        return handled(true);
    }
    if vcall(e, parent, REMOVE_OBJECT_SLOT, &[block]).is_some() {
        a.set_result(e, 1.0);
    }
    handled(true)
}

// ---- animation sequences ----

/// The controller manager of `node` (its first controller, +0xC, when that is an
/// `NiControllerManager`), for a reference that is not a dying actor; 0 for none.
fn controller_manager(e: &Engine, refr: u32, node: u32) -> u32 {
    if node == 0
        || (is_actor(e, refr) && nif::rd32(e, refr.wrapping_add(LIFE_STATE)) == Some(DYING))
    {
        return 0;
    }
    match nif::ptr(e, node.wrapping_add(CONTROLLER)) {
        Some(m) if nif::rd32(e, m) == Some(CONTROLLER_MANAGER_VTBL) => m,
        _ => 0,
    }
}

/// The manager's sequence slots in order (`sequences`, up to its first free entry), empty ones
/// included as 0; none when the array cannot be read (a stop at the first unreadable slot).
fn sequences(e: &Engine, manager: u32) -> Vec<u32> {
    let (Some(data), Some(count)) = (
        nif::rd32(e, manager.wrapping_add(SEQUENCES)),
        nif::rd16(e, manager.wrapping_add(SEQUENCE_COUNT)),
    ) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for i in 0..u32::from(count) {
        match nif::rd32(e, data.wrapping_add(4 * i)) {
            Some(s) => out.push(s),
            None => break,
        }
    }
    out
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/netimmerse.cpp:
// NiControllerManager::FindSequence.
/// The first sequence whose name equals `name` ignoring case (JIP's `StrCompareCI`; a sequence
/// without a name never does); 0 for none.
fn find_sequence(e: &Engine, manager: u32, name: &[u8]) -> u32 {
    sequences(e, manager)
        .into_iter()
        .filter(|&s| s != 0)
        .find(|&s| {
            nif::ptr(e, s.wrapping_add(SEQUENCE_NAME))
                .and_then(|p| nif::c_text(e, p))
                .is_some_and(|t| crate::utility::str_compare_ci(&t, name).0 == 0)
        })
        .unwrap_or(0)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_misc_ref.h:
// Cmd_GetAnimSequenceFrequency_Execute.
/// The named sequence's frequency (+0x28) of the root's (`GetNiNode`) controller manager; -1
/// (set first, whatever follows) for none.
fn get_anim_sequence_frequency(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    a.set_result(e, -1.0);
    let read = Read::of(e, &a);
    if !read.ok || a.this_obj == 0 {
        return handled(true);
    }
    let name = super::buffer_arg(e, &read, 0, NAME_BUFFER, "GetAnimSequenceFrequency");
    let manager = controller_manager(e, a.this_obj, nif::ni_node(e, a.this_obj));
    if manager == 0 {
        return handled(true);
    }
    let sequence = find_sequence(e, manager, &name);
    if sequence != 0 {
        if let Some(f) = nif::rdf32(e, sequence.wrapping_add(SEQUENCE_FREQUENCY)) {
            a.set_result(e, f64::from(f));
        }
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_misc_ref.h:
// Cmd_SetAnimSequenceFrequency_Execute.
/// The named sequence's frequency set, or with the name `*` every sequence's. JIP writes
/// through an empty slot too (the game stops); the port passes it over.
fn set_anim_sequence_frequency(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if !read.ok || a.this_obj == 0 {
        return handled(true);
    }
    let name = super::buffer_arg(e, &read, 0, NAME_BUFFER, "SetAnimSequenceFrequency");
    let frequency = read.float(1, 0.0).to_bits();
    let manager = controller_manager(e, a.this_obj, nif::ni_node(e, a.this_obj));
    if manager == 0 {
        return handled(true);
    }
    if name == b"*" {
        for s in sequences(e, manager).into_iter().filter(|&s| s != 0) {
            wr32(e, s.wrapping_add(SEQUENCE_FREQUENCY), frequency);
        }
    } else {
        let sequence = find_sequence(e, manager, &name);
        if sequence != 0 {
            wr32(e, sequence.wrapping_add(SEQUENCE_FREQUENCY), frequency);
        }
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/netimmerse.cpp:
// NiControllerSequence::Play (asm).
/// `Activate(0x64, true, 1.0, 0, null, null)`; when it answers true, the offset set to
/// `-FLT_MAX` and the manager's active bit set. Its answer.
fn play(e: &mut Engine, sequence: u32) -> bool {
    let ok = e.call(
        ACTIVATE,
        &args![sequence, 0x64u32, 1u32, 1.0f32, 0u32, 0u32, 0u32],
    );
    if ok.eax & 0xff == 0 {
        return false;
    }
    wr32(e, sequence.wrapping_add(SEQUENCE_OFFSET), NEG_FLT_MAX);
    if let Some(manager) = nif::ptr(e, sequence.wrapping_add(SEQUENCE_MANAGER)) {
        let at = manager.wrapping_add(CONTROLLER_FLAGS);
        if let Some(flags) = nif::rd8(e, at) {
            let _ = e.mem.try_write(at, &[flags | 8]);
        }
    }
    true
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_misc_ref.h:
// Cmd_PlayAnimSequence_Execute.
/// The named sequence of the named node's (`GetNode`: the seen root for an empty name)
/// controller manager played: first every sequence whose name does not begin with `Idle`
/// (exactly those four bytes) deactivated (`Deactivate(0, false)`), the named one too; then
/// [`play`]; when it plays, `MarkModified(0x10000000)` and result 1.
fn play_anim_sequence(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if !read.ok || a.this_obj == 0 {
        return handled(true);
    }
    let sequence_name = super::buffer_arg(e, &read, 0, SEQUENCE_BUFFER, "PlayAnimSequence");
    let node_name = super::buffer_arg(e, &read, 1, NAME_BUFFER, "PlayAnimSequence");
    let node = havok_refs::node(e, a.this_obj, &node_name);
    let manager = controller_manager(e, a.this_obj, node);
    if manager == 0 {
        return handled(true);
    }
    let sequence = find_sequence(e, manager, &sequence_name);
    if sequence == 0 {
        return handled(true);
    }
    for s in sequences(e, manager) {
        if s == 0 {
            continue;
        }
        // JIP compares the name's first word (`*(UInt32*)name != 'eldI'`); an unreadable one
        // is passed over here.
        let idle = match nif::ptr(e, s.wrapping_add(SEQUENCE_NAME)) {
            None => continue,
            Some(p) => match nif::rd32(e, p) {
                None => continue,
                Some(word) => word == IDLE,
            },
        };
        if !idle {
            vcall(e, s, DEACTIVATE_SLOT, &args![0.0f32, 0u32]);
        }
    }
    if play(e, sequence) {
        extra::mark_modified(e, a.this_obj, CHANGED_ANIMATION);
        a.set_result(e, 1.0);
    }
    handled(true)
}

#[cfg(test)]
#[path = "jip_fn_misc_ref_nodes_w_tests.rs"]
mod tests;
