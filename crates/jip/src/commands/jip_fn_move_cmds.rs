//! JIP LN NVSE's commands that move a reference through its own `TESObjectREFR::SetPos`,
//! `SetAngle` and `MoveToCell` ([`super::ref_move`]): `MoveToCell`, `MoveToEditorPosition`,
//! `SetPosEx`, `MoveToNode` (`functions_jip/jip_fn_misc_ref.h`) and `MoveToPosStr`
//! (`functions_jip/jip_fn_ccc.h`, in its own list for the table's file check).
//!
//! Left for later, with what each waits for:
//! - `SetAngleEx` (`jip_fn_misc_ref.h`): its modes 1 to 4 need JIP's `SetAngle` modes 1 and 2
//!   and `TESObjectREFR::Rotate`, with JIP's matrix maths `NiMatrix33::FromEulerPRYInv`,
//!   `ToEulerPRY`, `From2Points` and `MultiplyMatrices` (`internal/Ni_types.cpp`), not yet
//!   translated; only mode 0 would be possible today, so the command waits whole.
//! - `RotateAroundPoint` (`jip_fn_misc_ref.h`): JIP's `TESObjectREFR::RotateAroundPoint` needs
//!   `NiMatrix33::FromEulerYPR`, `NiTransform::RotateOrigin`, `ToEulerPRY` and
//!   `NiVector3::GetRotatedPos` (the same maths batch).
//!
//! `MoveToNode` (`jip_fn_misc_ref.h`) moves to a node of another reference found by lane G's
//! `TESObjectREFR::GetNode` (`havok_refs::node`).

use super::havok_refs::node as ref_node;
use super::jip_fn_misc_ref::{
    floats3, from_euler_pry, matrix_at, multiply_vector, ref_ni_node, ACTOR_START_CELL,
    ACTOR_START_POS, ACTOR_START_Z_ROT, CELL, EXTRA_STARTING_POSITION,
    EXTRA_STARTING_WORLD_OR_CELL, EXTRA_WORD, FLT_180_D_PI, STARTING_ROTATION, WORLD_CELL,
    WORLD_ROTATE, WORLD_SPACE, WORLD_TRANSLATE,
};
use super::ln_fn_math::cell_or_persistent;
use super::ref_move::{move_to_cell, move_to_cell_at, set_angle, set_pos_to};
use super::{buffer_arg, extra, is_actor, type_of, Handlers, FLT_PI_D_180};
use engine_core::prelude::*;
use nvse::commands::Read;
use nvse::{handled, CommandArgs};

#[rustfmt::skip]
pub static COMMANDS: Handlers = &[
    (0x2758, move_to_cell_cmd),
    (0x2759, move_to_editor_position),
    (0x27f0, set_pos_ex),
    (0x2867, move_to_node),
];

#[rustfmt::skip]
pub static CCC_COMMANDS: Handlers = &[
    (0x2763, move_to_pos_str),
];

/// `TESObjectREFR` `rotation` +0x24, `position` +0x30.
const ROTATION: u32 = 0x24;
const POSITION: u32 = 0x30;
/// JIP's buffer for `MoveToPosStr`'s text (`char posStr[0x40]`) and `MoveToNode`'s node name
/// (`char nodeName[0x40]`).
const POS_STR_SIZE: usize = 0x40;
const NODE_NAME_SIZE: usize = 0x40;

/// `NOT_ID(cell, TESObjectCELL)` then `NOT_ID(cell, TESWorldSpace)`: the cell itself, a
/// worldspace's persistent cell (+0x34), or `None` for another type. nv-rs's own: a
/// worldspace with no persistent cell is `None` too (JIP moves to a null cell).
fn cell_of(e: &Engine, form: u32) -> Option<u32> {
    if form == 0 {
        return None;
    }
    match type_of(e, form) {
        CELL => Some(form),
        WORLD_SPACE => match e.mem.u32(form + WORLD_CELL) {
            0 => None,
            cell => Some(cell),
        },
        _ => None,
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_misc_ref.h:
// Cmd_MoveToCell_Execute.
/// `ref.MoveToCell worldOrCell x y z`: the reference moved to the position in the cell (a
/// worldspace: its persistent cell) by JIP's `MoveToCell`; result 1. Another type: nothing.
fn move_to_cell_cmd(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if !read.ok || a.this_obj == 0 {
        return handled(true);
    }
    let Some(cell) = cell_of(e, read.form(0)) else {
        return handled(true);
    };
    let pos = [read.float(1, 0.0), read.float(2, 0.0), read.float(3, 0.0)];
    move_to_cell_at(e, a.this_obj, cell, pos);
    a.set_result(e, 1.0);
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_misc_ref.h:
// Cmd_MoveToEditorPosition_Execute.
/// `ref.MoveToEditorPosition [resetRot]`: the reference moved to its editor place by JIP's
/// `MoveToCell` (the position passed where it lies: the actor's +0x160, the starting position
/// extra's +0xC); with `resetRot`, turned to its editor angles by `SetAngle` (degrees and back:
/// each angle times `Flt180dPI`, then times `FltPId180`, as JIP's two multiplications round
/// them; an actor's x and y 0); result 1. An actor's place is its own (cell +0x170); another
/// reference's needs its starting position extra, its cell the starting world or cell extra's
/// or else JIP's `GetParentCell` (its cell +0x40, else its `ExtraPersistentCell`'s). No cell, or
/// a form of another type: nothing.
fn move_to_editor_position(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let reset_rot = if super::jip_fn_utility::num_args(e, &a) != 0 {
        Read::of(e, &a).int(0, 0)
    } else {
        0
    };
    let refr = a.this_obj;
    if refr == 0 {
        return handled(true);
    }
    let (cell, pos_at, degrees) = if is_actor(e, refr) {
        let z = e.mem.f32(refr + ACTOR_START_Z_ROT) * FLT_180_D_PI;
        (
            e.mem.u32(refr + ACTOR_START_CELL),
            refr + ACTOR_START_POS,
            [0.0, 0.0, z],
        )
    } else {
        let xpos = extra::refr_extra(e, refr, EXTRA_STARTING_POSITION);
        if xpos == 0 {
            return handled(true);
        }
        let xcell = extra::refr_extra(e, refr, EXTRA_STARTING_WORLD_OR_CELL);
        let cell = if xcell != 0 {
            e.mem.u32(xcell + EXTRA_WORD)
        } else {
            // `GetParentCell`: the cell, else the persistent cell extra's.
            cell_or_persistent(e, refr).unwrap_or(0)
        };
        let rot = floats3(e, xpos + STARTING_ROTATION).map(|v| v * FLT_180_D_PI);
        (cell, xpos + EXTRA_WORD, rot)
    };
    let Some(cell) = cell_of(e, cell) else {
        return handled(true);
    };
    move_to_cell(e, refr, cell, pos_at);
    if reset_rot != 0 {
        set_angle(e, refr, degrees.map(|v| v * FLT_PI_D_180));
    }
    a.set_result(e, 1.0);
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_misc_ref.h:
// Cmd_SetPosEx_Execute, and nvse/GameObjects.cpp: TESObjectREFR::GetTranslatedPos (asm).
/// `ref.SetPosEx x y z [transform]`: JIP's `SetPos` to the position (0), to the offsets turned
/// as `GetTranslatedPos` turns them (1: by the root's world rotation plus its world translation,
/// or without 3D by the matrix of the reference's angles plus its position), or to the offsets
/// added to the position (2); any other value nothing. No result.
fn set_pos_ex(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if !read.ok || a.this_obj == 0 {
        return handled(true);
    }
    let refr = a.this_obj;
    let v = [read.float(0, 0.0), read.float(1, 0.0), read.float(2, 0.0)];
    let pos = match read.int(3, 0) {
        0 => v,
        1 => {
            let node = ref_ni_node(e, refr);
            let (turned, add) = if node != 0 {
                let m = matrix_at(e, node + WORLD_ROTATE);
                let after = e.mem.f32(node + WORLD_TRANSLATE);
                (
                    multiply_vector(&m, after, v),
                    floats3(e, node + WORLD_TRANSLATE),
                )
            } else {
                // JIP's matrix is on its stack, the word past it the return address (finite):
                // its product with the cleared lane is +0, as `jip_fn_misc_ref` takes it.
                let m = from_euler_pry(floats3(e, refr + ROTATION));
                (multiply_vector(&m, 0.0, v), floats3(e, refr + POSITION))
            };
            [turned[0] + add[0], turned[1] + add[1], turned[2] + add[2]]
        }
        2 => {
            let p = floats3(e, refr + POSITION);
            [v[0] + p[0], v[1] + p[1], v[2] + p[2]]
        }
        _ => return handled(true),
    };
    set_pos_to(e, refr, pos);
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_misc_ref.h:
// Cmd_MoveToNode_Execute.
/// `ref.MoveToNode target "node" [x y z]`: the reference moved by JIP's `MoveToCell` into the
/// target's cell (`GetParentCell`: its cell, else its persistent cell extra's) to the node's world
/// translation (+0x8C) plus the offsets (each `offset + translation`, JIP's `NiVector4 +=`);
/// result 1. No cell, or no such node under the target (`GetNode`: the player's first person
/// root when in first person; an empty name the root itself): nothing.
fn move_to_node(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if !read.ok || a.this_obj == 0 {
        return handled(true);
    }
    let target = read.form(0);
    if target == 0 {
        return handled(true);
    }
    let name = buffer_arg(e, &read, 1, NODE_NAME_SIZE, "MoveToNode");
    let mods = [read.float(2, 0.0), read.float(3, 0.0), read.float(4, 0.0)];
    let cell = match cell_or_persistent(e, target) {
        Some(c) if c != 0 => c,
        _ => return handled(true),
    };
    let node = ref_node(e, target, &name);
    if node == 0 {
        return handled(true);
    }
    let t = floats3(e, node + WORLD_TRANSLATE);
    let pos = [mods[0] + t[0], mods[1] + t[1], mods[2] + t[2]];
    move_to_cell_at(e, a.this_obj, cell, pos);
    a.set_result(e, 1.0);
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_ccc.h:
// Cmd_MoveToPosStr_Execute.
/// `ref.MoveToPosStr "cellID x y z"` (the text `GetPosStr` makes): the cell's id in hexadecimal
/// (resolved through the load order as JIP's `GetResolvedRefID`), then three decimal integers,
/// split at spaces (`GetNextToken`); the reference moved there by JIP's `MoveToCell`; result 1.
/// An id of 0, unresolved or of no form, a form of another type, or fewer than four parts:
/// nothing.
fn move_to_pos_str(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if !read.ok || a.this_obj == 0 {
        return handled(true);
    }
    let text = buffer_arg(e, &read, 0, POS_STR_SIZE, "MoveToPosStr");
    let next = super::jip_fn_ui::next_token;
    let (id_text, rest) = next(&text, b' ');
    let id = super::jip_fn_utility::hex_to_uint(id_text);
    if id == 0 || rest.is_empty() {
        return handled(true);
    }
    let id = crate::cosave::resolved_ref_id(e, id);
    if id == 0 {
        return handled(true);
    }
    let Some(cell) = cell_of(e, crate::cosave::lookup_form(e, id)) else {
        return handled(true);
    };
    let (x_text, rest) = next(rest, b' ');
    let x = crate::utility::str_to_int(x_text) as f32;
    if rest.is_empty() {
        return handled(true);
    }
    let (y_text, rest) = next(rest, b' ');
    let y = crate::utility::str_to_int(y_text) as f32;
    if rest.is_empty() {
        return handled(true);
    }
    let z = crate::utility::str_to_int(rest) as f32;
    move_to_cell_at(e, a.this_obj, cell, [x, y, z]);
    a.set_result(e, 1.0);
    handled(true)
}

#[cfg(test)]
#[path = "jip_fn_move_cmds_tests.rs"]
mod tests;
