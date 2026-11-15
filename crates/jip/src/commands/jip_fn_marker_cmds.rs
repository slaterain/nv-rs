//! The commands of JIP LN NVSE that use its temporary position marker ([`super::pos_marker`],
//! `s_tempPosMarker`): `PlaceAtCell` (`functions_jip/jip_fn_misc_ref.h`). Kept apart from
//! `jip_fn_misc_ref.rs` so the marker's commands come together.
//!
//! Left for later: `MoveToCell`, `MoveToEditorPosition` (`jip_fn_misc_ref.h`) and `MoveToPosStr`
//! (`jip_fn_ccc.h`), and also `MoveToNode` and the positions JIP's `SetPosEx` and its player sync
//! move to, go through JIP's `TESObjectREFR::MoveToCell` (`nvse/GameObjects.cpp`). Its marker path
//! is ready here (the marker's cell, the reference's rotation and the position written into it,
//! then the game's `MoveToMarker`, `005CCB20`), but a reference with its 3D, in the same cell or
//! in a cell of the same worldspace, is moved by JIP's own `TESObjectREFR::SetPos` instead: the
//! game's `SetPos` (`00575830`), the scene node's translation, then for an actor with a
//! character controller the Havok character proxy's position (`00C9E990`, from the controller's
//! matrices in Havok units), else JIP's `NiNode::ResetCollision` walk, then the node's update
//! (`00A5DD70` with JIP's `kNiUpdateData`); `MoveToEditorPosition` also turns it with JIP's
//! `SetAngle` (Euler matrix maths, the same collision reset and update). That is 3D and Havok
//! work, the same that `SetPosEx`, `SetAngleEx` and `MoveToNode` wait for (lanes E and F): those
//! three commands come with it, with no part built here on its own.

use super::pos_marker::{self, PARENT_CELL, POSITION};
use super::{set_form_result, type_of, Handlers};
use crate::commands::forms::TESWorldSpace;
use engine_core::prelude::*;
use nvse::commands::Read;
use nvse::{handled, CommandArgs};

#[rustfmt::skip]
pub static COMMANDS: Handlers = &[
    (0x285d, place_at_cell),
];

/// `kFormType_TESObjectCELL`, `kFormType_TESWorldSpace`.
const CELL: u8 = 0x39;
const WORLD_SPACE: u8 = 0x41;
/// The game's `PlaceAtMe` (`005C4B30`, `cdecl`: the reference to place at, the form, the count,
/// the distance, the direction, the health as a float), JIP's `PlaceAtMe` helper.
pub const PLACE_AT_ME: u32 = 0x005c_4b30;

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_misc_ref.h:
// Cmd_PlaceAtCell_Execute.
/// `PlaceAtCell form count worldOrCell x y z`: `count` of `form` placed at the position in the
/// cell (a worldspace: its persistent cell, `+0x34`), through the marker: the three floats are
/// read straight into the marker's position (as `ExtractArgsEx` writes each, so a read that
/// fails part way leaves those it read), its `parentCell` set to the cell, then the game's
/// `PlaceAtMe(marker, form, count, 0, 0, 1.0)`; the placed reference's id is the result. Another
/// type than a cell or a worldspace: nothing placed (the position written all the same).
fn place_at_cell(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let marker = pos_marker::marker(e);
    // nv-rs's own: JIP reads the arguments into the marker before it exists (before the end of
    // the first frame) and writes through a null reference; here nothing is done.
    if marker == 0 {
        return handled(true);
    }
    for (i, axis) in (3..6).zip(0u32..) {
        let at = marker + POSITION + 4 * axis;
        let value = read.float(i, e.mem.f32(at));
        e.mem.set_f32(at, value);
    }
    if !read.ok {
        return handled(true);
    }
    let (form, count, mut cell) = (read.form(0), read.int(1, 0), read.form(2));
    if type_of(e, cell) != CELL {
        if type_of(e, cell) != WORLD_SPACE {
            return handled(true);
        }
        cell = e.get(Ptr::<TESWorldSpace>::new(cell), TESWorldSpace::cell);
    }
    e.mem.set_u32(marker + PARENT_CELL, cell);
    let placed = e
        .call(PLACE_AT_ME, &args![marker, form, count, 0u32, 0u32, 1.0f32])
        .u32();
    if placed != 0 {
        set_form_result(e, &a, placed);
    }
    handled(true)
}

#[cfg(test)]
#[path = "jip_fn_marker_cmds_tests.rs"]
mod tests;
