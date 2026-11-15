//! JIP LN NVSE's own moves of a reference (`nvse/GameObjects.cpp`): `TESObjectREFR::SetPos`,
//! `TESObjectREFR::SetAngle` (every `setMode`), `TESObjectREFR::Rotate`,
//! `TESObjectREFR::RotateAroundPoint`, `TESObjectREFR::MoveToCell`, and the helper they share,
//! `NiNode::ResetCollision` (`internal/netimmerse.cpp`), built once here for the commands that
//! move references (`jip_fn_move_cmds`). Their matrices are JIP's (`ni_matrix`, lane E's
//! `FromEulerPRY`, lane F's `ToEulerPRY`).
//!
//! What each writes, as JIP writes it:
//! - `SetPos`: the game's `SetLocationOnReference` (`00575830`, `thiscall` (reference, position);
//!   `RET 4`). With 3D (`pLoadedData` +0x64, its root +0x14): the root's local translation
//!   (`m_kLocal` +0x34, translation +0x58) set to the reference's new position (+0x30); then for
//!   an actor (its vtable answers true, `008D0360`, at +0x100) with a process (+0x68) of level 0
//!   or 1 (+0x28), a character controller (`MiddleHighProcess` +0x138) not in state 4
//!   (`m_currentState` +0x3F0) and a shape phantom (`spShapePhantom` +0x594, its Havok object
//!   +8): the phantom's position (`hkpShapePhantom::setPosition`, `00C9E990`, `thiscall`
//!   (phantom, position, tolerance), `RET 8`) set to the position in Havok units (times
//!   `1/70`, `0x3E124DD2`) plus the controller's `RotCenter` (+0x570) turned by the phantom's
//!   rotation rows (+0xB0, +0xC0, +0xD0), the character proxy's (`phkObject` +8) velocity
//!   (+0x20) cleared first, the tolerance `m_keepDistance + m_keepContactTolerance` (+0x58,
//!   +0x5C, as the game's own `hkpCharacterProxy::setPosition`, `00CAC920`, sums them), the
//!   phantom being the proxy's `m_shapePhantom` (+0x30, the exe's getter `00CAC980`); any other
//!   reference gets the collision reset below. Then the root's update
//!   (`NiNode::UpdateDownwardPass`, `00A5DD70`, `thiscall` (node, update data, 0), `RET 8`) with
//!   JIP's `kNiUpdateData` (12 zero bytes).
//! - `SetAngle` (mode 0): the angles (radians) into the reference's rotation (+0x24); unless it
//!   is an actor, with 3D: the matrix of the angles (JIP's `NiMatrix33::FromEulerPRY`,
//!   `jip_fn_misc_ref`) into the root's local rotation (+0x34, 0x24 bytes), the collision reset
//!   and the update as above; then always `MarkModified(2)` (`BGSSaveLoadGame::AddChange`,
//!   `0084A690`). Modes 1 (`FromEulerPRYInv`) and 2 and up (`From2Points` toward the point)
//!   store the matrix's `ToEulerPRY` as the angles instead.
//! - `Rotate`: the root's local rotation (or, for the player or without 3D, the angles' matrix)
//!   times `FromEulerPRYInv` of the turn; `RotateAroundPoint`: the root's local transform turned
//!   about a point (`RotateOrigin`), or with `skipAngles` only the position (`GetRotatedPos`),
//!   then `SetPos`.
//! - `MoveToCell`: a reference with 3D whose cell (+0x40) is the destination, or shares its
//!   non-null worldspace (+0xC0), is moved by `SetPos`; any other goes through JIP's temporary
//!   position marker (`pos_marker`): the marker's cell, the reference's rotation and the position
//!   written into it, then the game's `MoveToMarker` (`Script::MoveToFunctionBase`, `005CCB20`,
//!   `cdecl` (reference, destination, three float offsets 0)).
//! - `ResetCollision`: bit 0x40 set in the flags (+0xC) of the node's collision object
//!   (`m_spCollisionObject` +0x1C), then the same for each child that is a node (its vtable's
//!   +0xC is `ReturnThis`, `006815C0`), from the children array (+0xA0, slots in use +0xA6). The
//!   exe's own `bhkNiCollisionObject::ResetSim` (`00C6BD00`) walks the same fields (the flags as
//!   16 bits, the children backwards); JIP's walk is the one done here.
//!
//! Locks: the game's own `SetPos` and `SetAngle` script commands (`Script::SetPosFunction`,
//! `005C05F0`, engine-fallout-script `tesscriptfunctions_p2.rs`) take no lock either: they queue
//! a task (`0087A8B0`, tasks `0x1007`/`0x1009`) when the setting at `011C3EA4` is above 1, else
//! call their base function directly. JIP calls the functions above directly on the script's
//! thread, with no lock and no task; so does the port.

use super::jip_fn_misc_ref::{floats3, from_euler_pry, matrix_at, ref_ni_node as root_node};
use super::ni_matrix::{
    from_2_points, from_euler_pry_inv, from_euler_ypr, get_rotated_pos, multiply_matrices,
    rotate_origin, to_euler_pry_lane3,
};
use super::nif_nodes::{is_player, to_euler_pry};
use super::{extra, is_actor, pos_marker};
use engine_core::prelude::*;

/// `TESObjectREFR` (Xbox PDB): `rotation` +0x24, `position` +0x30, `pParentCell` +0x40 (the 3D root through `jip_fn_misc_ref`'s `GetRefNiNode`).
const ROTATION: u32 = 0x24;
const POSITION: u32 = 0x30;
pub(crate) const PARENT_CELL: u32 = 0x40;
/// `TESObjectCELL::worldSpace` (JIP LN NVSE, `// C0`; the exe's `GetWorldSpace` 00575D70 reads
/// the reference's cell's +0xC0).
pub(crate) const CELL_WORLD_SPACE: u32 = 0xc0;
/// `NiAVObject::m_kLocal` (+0x34: rotation, 0x24 bytes; translation +0x58),
/// `m_spCollisionObject` +0x1C; `NiNode::m_kChildren` (+0xA0 the array, +0xA6 the slots in use).
const LOCAL_ROTATE: u32 = 0x34;
const LOCAL_TRANSLATE: u32 = 0x58;
const COLLISION_OBJECT: u32 = 0x1c;
const CHILDREN: u32 = 0xa0;
const CHILD_COUNT: u32 = 0xa6;
/// A collision object's flags (+0xC) and the bit `ResetCollision` sets.
const COLLISION_FLAGS: u32 = 0x0c;
const RESET_BIT: u8 = 0x40;
/// `NiObject::GetAsNiNode` (vtable +0xC) answering `this` (`006815C0`): the child is a node.
const GET_AS_NI_NODE: u32 = 0x0c;
const RETURN_THIS: u32 = 0x0068_15c0;
/// `Actor::pCurrentProcess` (+0x68), a process's level (+0x28), the character controller of a
/// middle-high process (+0x138: `MiddleHighProcess` constructor `POINTER_INIT(p + 0x138)`,
/// engine-fallout-ai `middlehighprocess.rs`).
const PROCESS: u32 = 0x68;
const PROCESS_LEVEL: u32 = 0x28;
const CHAR_CONTROLLER: u32 = 0x138;
/// `bhkCharacterController` (engine-havok `bhkcharactercontroller.rs`): `phkObject` +8 (the
/// `hkpCharacterProxy`), `m_currentState` +0x3F0, `RotCenter` +0x570, `spShapePhantom` +0x594.
const CTRL_PROXY: u32 = 0x08;
const CTRL_STATE: u32 = 0x3f0;
const CTRL_ROT_CENTER: u32 = 0x570;
const CTRL_PHANTOM: u32 = 0x594;
/// The state `SetPos` leaves alone (JIP's `cmp [eax+0x3F0], 4`).
const STATE_SKIPPED: u32 = 4;
/// A `bhk` wrapper's Havok object (+8), an `hkpShapePhantom`'s rotation rows (+0xB0, as
/// `setPosition` 00C9E990 passes `this + 0xB0` as its transform) and position (+0xE0).
const HK_OBJECT: u32 = 0x08;
const PHANTOM_ROWS: u32 = 0xb0;
const PHANTOM_POSITION: u32 = 0xe0;
/// `hkpCharacterProxy`: `m_velocity` +0x20, `m_shapePhantom` +0x30, `m_keepDistance` +0x58,
/// `m_keepContactTolerance` +0x5C (engine-havok `hkpcharacterproxy.rs`; 00CAC920 sums the two).
const PROXY_VELOCITY: u32 = 0x20;
const PROXY_PHANTOM: u32 = 0x30;
const PROXY_KEEP_DISTANCE: u32 = 0x58;
/// JIP's `kUnitConv` (`EMIT_PS_3(0x3E124DD2)`: three lanes of 1/70, the fourth 0).
const UNIT_CONV: [u32; 4] = [0x3e12_4dd2, 0x3e12_4dd2, 0x3e12_4dd2, 0];

/// `TESObjectREFR::SetLocationOnReference` (`00575830`), `hkpShapePhantom::setPosition`
/// (`00C9E990`), `NiNode::UpdateDownwardPass` (`00A5DD70`), `Script::MoveToFunctionBase`
/// (`005CCB20`, JIP's `ADDR_MoveToMarker`).
pub const SET_LOCATION: u32 = 0x0057_5830;
pub const PHANTOM_SET_POSITION: u32 = 0x00c9_e990;
pub const UPDATE_DOWNWARD_PASS: u32 = 0x00a5_dd70;
pub const MOVE_TO_MARKER: u32 = 0x005c_cb20;
/// `MarkModified`'s flag in `SetAngle` (`kChangeFlag_Refr_Move`'s bit as JIP passes it: 2).
const CHANGED_MOVE: u32 = 2;
/// The most nodes one collision reset visits or holds waiting (JIP's recursion has no bound).
const MAX_NODES: usize = 0x1_0000;

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/netimmerse.cpp: NiNode::ResetCollision
// (asm).
/// Bit 0x40 in the flags of the collision object of `node` and, depth first, of every node
/// below it (empty slots and children that are not nodes skipped).
pub(crate) fn reset_collision(e: &mut Engine, node: u32) {
    reset_walk(e, node);
}

/// [`reset_collision`]'s walk; answers the most nodes it held waiting at once.
fn reset_walk(e: &mut Engine, node: u32) -> usize {
    let mut stack = vec![node];
    let mut visited = 0;
    let mut peak = 1;
    while let Some(n) = stack.pop() {
        visited += 1;
        if visited > MAX_NODES {
            return peak;
        }
        let object = e.mem.u32(n + COLLISION_OBJECT);
        if object != 0 {
            let flags = object + COLLISION_FLAGS;
            e.mem.set_u8(flags, e.mem.u8(flags) | RESET_BIT);
        }
        let count = u32::from(e.mem.u16(n + CHILD_COUNT));
        let children = e.mem.u32(n + CHILDREN);
        if count == 0 || children == 0 {
            continue;
        }
        // Pushed backwards so the first child is walked first, as JIP's recursion does.
        for i in (0..count).rev() {
            // nv-rs's own: visits plus waiting nodes stay within the bound (a damaged or cyclic
            // graph cannot grow the stack without end).
            if visited + stack.len() >= MAX_NODES {
                return peak;
            }
            let child = e.mem.u32(children + 4 * i);
            if child == 0 {
                continue;
            }
            let vtbl = e.mem.u32(child);
            if vtbl != 0 && e.mem.u32(vtbl + GET_AS_NI_NODE) == RETURN_THIS {
                stack.push(child);
                peak = peak.max(stack.len());
            }
        }
    }
    peak
}

/// `node->UpdateDownwardPass(kNiUpdateData, 0)` as JIP calls it (`00A5DD70` directly), the
/// update data (JIP's static, zeroed by its constructor) in a frame of the stack.
fn update_node(e: &mut Engine, node: u32) {
    e.with_stack(0xc, |e, data| {
        let data = data.addr();
        for i in 0..3 {
            e.mem.set_u32(data + 4 * i, 0);
        }
        e.call(UPDATE_DOWNWARD_PASS, &args![node, data, 0u32]);
    });
}

fn f32x4(e: &Engine, at: u32) -> [f32; 4] {
    std::array::from_fn(|i| e.mem.f32(at + 4 * i as u32))
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), nvse/GameObjects.cpp: TESObjectREFR::SetPos (asm).
/// The reference moved to the position at `pos_at` (three floats in guest memory): see the
/// module's list. Nothing beyond the game's call without 3D.
pub(crate) fn set_pos(e: &mut Engine, refr: u32, pos_at: u32) {
    e.call(SET_LOCATION, &args![refr, pos_at]);
    let node = root_node(e, refr);
    if node == 0 {
        return;
    }
    let pos = f32x4(e, refr + POSITION);
    e.mem.set_f32(node + LOCAL_TRANSLATE, pos[0]);
    e.mem.set_f32(node + LOCAL_TRANSLATE + 4, pos[1]);
    e.mem.set_f32(node + LOCAL_TRANSLATE + 8, pos[2]);
    if is_actor(e, refr) {
        sync_phantom(e, refr, pos);
    } else {
        reset_collision(e, node);
    }
    update_node(e, node);
}

/// `SetPos`'s actor part: the character controller's shape phantom moved to the position in
/// Havok units (all four lanes as JIP's `mulps`/`addps` make them: the fourth from the
/// reference's scale times 0 and the rows' fourth floats). Each pointer JIP follows is checked.
fn sync_phantom(e: &mut Engine, refr: u32, pos: [f32; 4]) {
    let process = e.mem.u32(refr + PROCESS);
    if process == 0 || e.mem.u8(process + PROCESS_LEVEL) > 1 {
        return;
    }
    let ctrl = e.mem.u32(process + CHAR_CONTROLLER);
    if ctrl == 0 || e.mem.u32(ctrl + CTRL_STATE) == STATE_SKIPPED {
        return;
    }
    let wrapper = e.mem.u32(ctrl + CTRL_PHANTOM);
    if wrapper == 0 {
        return;
    }
    let shape = e.mem.u32(wrapper + HK_OBJECT);
    if shape == 0 {
        return;
    }
    let rows = shape + PHANTOM_ROWS;
    let center = f32x4(e, ctrl + CTRL_ROT_CENTER);
    let (r0, r1, r2) = (f32x4(e, rows), f32x4(e, rows + 0x10), f32x4(e, rows + 0x20));
    let v: [f32; 4] = std::array::from_fn(|i| {
        let mut x = pos[i] * f32::from_bits(UNIT_CONV[i]);
        x += center[0] * r0[i];
        x += center[1] * r1[i];
        x += center[2] * r2[i];
        x
    });
    // nv-rs's own: JIP follows the proxy and its phantom without a check.
    let proxy = e.mem.u32(ctrl + CTRL_PROXY);
    if proxy == 0 {
        return;
    }
    let tolerance =
        e.mem.f32(proxy + PROXY_KEEP_DISTANCE) + e.mem.f32(proxy + PROXY_KEEP_DISTANCE + 4);
    for i in 0..4 {
        e.mem.set_u32(proxy + PROXY_VELOCITY + 4 * i, 0);
    }
    let phantom = e.mem.u32(proxy + PROXY_PHANTOM);
    if phantom == 0 {
        return;
    }
    let at = phantom + PHANTOM_POSITION;
    for (i, x) in v.into_iter().enumerate() {
        e.mem.set_f32(at + 4 * i as u32, x);
    }
    e.call(PHANTOM_SET_POSITION, &args![phantom, at, tolerance]);
}

/// [`set_pos`] to a position the caller holds: the three floats in a frame of the stack (JIP's
/// callers pass a vector of their own frame).
pub(crate) fn set_pos_to(e: &mut Engine, refr: u32, pos: [f32; 3]) {
    e.with_stack(0xc, |e, at| {
        let at = at.addr();
        for (i, v) in pos.into_iter().enumerate() {
            e.mem.set_f32(at + 4 * i as u32, v);
        }
        set_pos(e, refr, at);
    });
}

/// [`set_angle_mode`] in JIP's mode 0: the reference turned to the angles `pry` (radians).
pub(crate) fn set_angle(e: &mut Engine, refr: u32, pry: [f32; 3]) {
    set_angle_mode(e, refr, pry, 0);
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), nvse/GameObjects.cpp: TESObjectREFR::SetAngle
// (asm), every `setMode`.
/// The reference turned by `v` in JIP's mode `mode` (its `dl`): 0 the angles themselves (stored,
/// then their `FromEulerPRY` matrix); 1 the matrix `FromEulerPRYInv(v)`; 2 and up the matrix
/// pointing from the point `v` to the reference's position (`From2Points(position, v)`); for 1
/// and up the angles stored are the matrix's `ToEulerPRY`. Then, unless an actor, with 3D: that
/// matrix into the root's local rotation (+0x34, 0x24 bytes), the collision reset and the update;
/// always `MarkModified(2)`.
pub(crate) fn set_angle_mode(e: &mut Engine, refr: u32, v: [f32; 3], mode: u8) {
    let m = match mode {
        0 => {
            for (i, x) in v.into_iter().enumerate() {
                e.mem.set_f32(refr + ROTATION + 4 * i as u32, x);
            }
            from_euler_pry(v)
        }
        _ => {
            let m = if mode == 1 {
                from_euler_pry_inv(v)
            } else {
                from_2_points(floats3(e, refr + POSITION), v)
            };
            // `movlps [esi+0x24], xmm0` (pitch, roll) and `movss [esi+0x2C], xmm6` (the yaw).
            for (i, x) in to_euler_pry(&m).into_iter().enumerate() {
                e.mem.set_f32(refr + ROTATION + 4 * i as u32, x);
            }
            m
        }
    };
    if !is_actor(e, refr) {
        let node = root_node(e, refr);
        if node != 0 {
            write_matrix(e, node + LOCAL_ROTATE, &m);
            reset_collision(e, node);
            update_node(e, node);
        }
    }
    extra::mark_modified(e, refr, CHANGED_MOVE);
}

fn write_matrix(e: &mut Engine, at: u32, m: &[f32; 9]) {
    for (i, x) in m.iter().enumerate() {
        e.mem.set_f32(at + 4 * i as u32, *x);
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), nvse/GameObjects.cpp: TESObjectREFR::Rotate (asm).
/// The reference turned further by `pry` (radians): `MarkModified(2)` first; then, unless it is
/// the player (`refID` 0x14) or has no 3D, the root's local rotation times `FromEulerPRYInv(pry)`
/// written back, its `ToEulerPRY` stored as the angles, and unless an actor the collision reset
/// and the update; the player or a reference without 3D: the angles' own matrix times it, its
/// `ToEulerPRY` stored, nothing else.
pub(crate) fn rotate(e: &mut Engine, refr: u32, pry: [f32; 3]) {
    let turn = from_euler_pry_inv(pry);
    extra::mark_modified(e, refr, CHANGED_MOVE);
    let node = if is_player(e, refr) {
        0
    } else {
        root_node(e, refr)
    };
    let m = if node != 0 {
        let m = multiply_matrices(&matrix_at(e, node + LOCAL_ROTATE), &turn);
        write_matrix(e, node + LOCAL_ROTATE, &m);
        m
    } else {
        multiply_matrices(&from_euler_pry(floats3(e, refr + ROTATION)), &turn)
    };
    for (i, x) in to_euler_pry(&m).into_iter().enumerate() {
        e.mem.set_f32(refr + ROTATION + 4 * i as u32, x);
    }
    if node != 0 && !is_actor(e, refr) {
        reset_collision(e, node);
        update_node(e, node);
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), nvse/GameObjects.cpp:
// TESObjectREFR::RotateAroundPoint (asm).
/// The reference turned about `origin` by the yaw, pitch and roll angles `pry` (radians).
/// `skip_angles` 0: needs 3D (else nothing): the root's local transform turned about the origin
/// (`RotateOrigin` with `FromEulerYPR(pry)`), the angles stored as the new rotation's
/// `ToEulerPRY` with its fourth lane over the position's x (`movups [ecx+0x24]`: 16 bytes; the
/// game's `SetLocationOnReference` then writes the position), then `SetPos` to the root's new
/// translation. Otherwise only the position, turned about the origin (`GetRotatedPos`), by
/// `SetPos`.
pub(crate) fn rotate_around_point(
    e: &mut Engine,
    refr: u32,
    pry: [f32; 3],
    origin: [f32; 3],
    skip_angles: u32,
) {
    let m = from_euler_ypr(pry);
    if skip_angles != 0 {
        let pos = get_rotated_pos(&m, floats3(e, refr + POSITION), origin);
        set_pos_to(e, refr, pos);
        return;
    }
    let node = root_node(e, refr);
    if node == 0 {
        return;
    }
    let (rot, trans) = rotate_origin(
        &matrix_at(e, node + LOCAL_ROTATE),
        floats3(e, node + LOCAL_TRANSLATE),
        &m,
        origin,
    );
    write_matrix(e, node + LOCAL_ROTATE, &rot);
    for (i, x) in trans.into_iter().enumerate() {
        e.mem.set_f32(node + LOCAL_TRANSLATE + 4 * i as u32, x);
    }
    let angles = to_euler_pry(&rot);
    for (i, x) in angles.into_iter().enumerate() {
        e.mem.set_f32(refr + ROTATION + 4 * i as u32, x);
    }
    e.mem.set_f32(refr + POSITION, to_euler_pry_lane3(&rot));
    set_pos(e, refr, node + LOCAL_TRANSLATE);
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), nvse/GameObjects.cpp: TESObjectREFR::MoveToCell
// (asm).
/// The reference moved to the position at `pos_at` in `cell` (a cell, never null: the callers
/// check its type): by [`set_pos`] when it has 3D and stays in its cell or its worldspace, else
/// through the marker and the game's `MoveToMarker`.
///
/// The marker path's two 16-byte copies: the first (the reference's rotation and position x into
/// the marker's +0x24) leaves the rotation, its fourth word overwritten by the second; the second
/// (the caller's vector into +0x30) also writes the four bytes after the caller's vector over the
/// marker's scale (+0x3C). Those bytes are JIP's caller's stack for most callers; nothing reads
/// the marker's scale (`MoveToMarker` 005CCB20 reads the destination's cell +0x40, worldspace,
/// rotation +0x24 and its three position floats through the vtable's +0x1F4, `00436AA0`; the
/// game's `PlaceAtMe` 005C4B30, the marker's other user, the same), so the port leaves it.
pub(crate) fn move_to_cell(e: &mut Engine, refr: u32, cell: u32, pos_at: u32) {
    let old = e.mem.u32(refr + PARENT_CELL);
    if old != 0 && root_node(e, refr) != 0 {
        if old == cell {
            set_pos(e, refr, pos_at);
            return;
        }
        let world = e.mem.u32(cell + CELL_WORLD_SPACE);
        if world != 0 && e.mem.u32(old + CELL_WORLD_SPACE) == world {
            set_pos(e, refr, pos_at);
            return;
        }
    }
    // nv-rs's own: before JIP's deferred start made the marker JIP writes through null.
    let marker = pos_marker::marker(e);
    if marker == 0 {
        return;
    }
    e.mem.set_u32(marker + PARENT_CELL, cell);
    for i in 0..3 {
        let rot = e.mem.u32(refr + ROTATION + 4 * i);
        e.mem.set_u32(marker + ROTATION + 4 * i, rot);
    }
    for i in 0..3 {
        let v = e.mem.u32(pos_at + 4 * i);
        e.mem.set_u32(marker + POSITION + 4 * i, v);
    }
    e.call(MOVE_TO_MARKER, &args![refr, marker, 0u32, 0u32, 0u32]);
}

/// [`move_to_cell`] to a position the caller holds (in a frame of the stack).
pub(crate) fn move_to_cell_at(e: &mut Engine, refr: u32, cell: u32, pos: [f32; 3]) {
    e.with_stack(0xc, |e, at| {
        let at = at.addr();
        for (i, v) in pos.into_iter().enumerate() {
            e.mem.set_f32(at + 4 * i as u32, v);
        }
        move_to_cell(e, refr, cell, at);
    });
}

#[cfg(test)]
#[path = "ref_move_tests.rs"]
pub(crate) mod tests;
