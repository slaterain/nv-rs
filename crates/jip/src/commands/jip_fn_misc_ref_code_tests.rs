//! `ToggleNoZPosReset` and `ToggleObjectCollision`, called by their opcodes, the two seams the
//! first answers and JIP's collision toggle. Expectations from JIP's bodies
//! (`Cmd_ToggleNoZPosReset_Execute` and its bytes, `ToggleObjectCollision`,
//! `TESObjectREFR::ToggleCollision`, `NiNode::ToggleCollision`).

use super::*;
use crate::commands::test_support::*;

const REF: u32 = FORMS + 0x1000;
const REF_VT: u32 = FORMS + 0x1400;
const NODE_VT: u32 = FORMS + 0x1800;
const LOADED: u32 = FORMS + 0x1c00;
const ROOT: u32 = FORMS + 0x2000;
const CHILD: u32 = FORMS + 0x2200;
const LEAF: u32 = FORMS + 0x2400;
const CHILDREN: u32 = FORMS + 0x2600;
const WORLD_OBJ: u32 = FORMS + 0x2800;
const WORLD_VT: u32 = FORMS + 0x2a00;
const HAVOK: u32 = FORMS + 0x2c00;
const COLLISION: u32 = FORMS + 0x2e00;
const PROC: u32 = FORMS + 0x3000;
const WORDS: u32 = FORMS + 0x3200;
const CONTROLLER: u32 = FORMS + 0x3400;
const CTRL_OBJ: u32 = FORMS + 0x3600;
const UPDATE_FN: u32 = 0x0dea_8000;

/// A reference (not an actor) whose 3D is a node with one child node (with the collision
/// object) and one child that is not a node; the world object's slot `0xC4` records.
fn world() -> Engine {
    let mut e = engine();
    e.mem.set_u32(REF, REF_VT);
    e.mem.set_u32(REF + LOADED_DATA, LOADED);
    e.mem.set_u32(LOADED + LOADED_3D, ROOT);
    for node in [ROOT, CHILD] {
        e.mem.set_u32(node, NODE_VT);
    }
    e.mem.set_u32(NODE_VT + 0xc, RETURN_THIS);
    e.mem.set_u32(LEAF, NODE_VT + 0x40);
    e.mem.set_u16(ROOT + 0xa6, 3);
    e.mem.set_u32(ROOT + 0xa0, CHILDREN);
    e.mem.set_u32(CHILDREN, CHILD);
    e.mem.set_u32(CHILDREN + 4, 0);
    e.mem.set_u32(CHILDREN + 8, LEAF);
    e.mem.set_u32(CHILD + 0x1c, COLLISION);
    e.mem.set_u32(COLLISION + 0x10, WORLD_OBJ);
    e.mem.set_u32(WORLD_OBJ, WORLD_VT);
    e.mem.set_u32(WORLD_OBJ + 8, HAVOK);
    e.mem.set_u32(WORLD_VT + 0xc4, UPDATE_FN);
    e.mem.set_u8(HAVOK + 0x2d, 0x81);
    e.register(UPDATE_FN, |_, w| {
        CALLS.with(|c| c.borrow_mut().push((UPDATE_FN, w.to_vec())));
        Ret::default()
    });
    e
}

#[test]
fn no_z_pos_reset_flags_and_the_seams() {
    let mut e = world();
    // Before the first "on": nothing at either seam.
    e.mem.set_u8(REF + 0x5f, 0x10);
    assert_eq!(e.site(AREA_RESET.addr(), &[REF]), None);
    assert_eq!(e.site(WARP_ACTOR.addr(), &[REF]), None);
    e.mem.set_u8(REF + 0x5f, 0x01);
    call_on(&mut e, 0x26db, REF, &[n(1)]);
    assert_eq!(e.mem.u8(REF + 0x5f), 0x11, "the flag, the others kept");
    assert_eq!(e.site(AREA_RESET.addr(), &[REF]), Some(1));
    assert_eq!(e.site(WARP_ACTOR.addr(), &[REF]), Some(1));
    assert_eq!(
        e.site(AREA_RESET.addr(), &[0]),
        Some(0),
        "no collidable reference"
    );
    call_on(&mut e, 0x26db, REF, &[n(0)]);
    assert_eq!(e.mem.u8(REF + 0x5f), 0x01);
    assert_eq!(
        e.site(AREA_RESET.addr(), &[REF]),
        Some(0),
        "the code stays written"
    );
    assert_eq!(e.site(WARP_ACTOR.addr(), &[REF]), Some(0));
}

/// Off: JIP's flag set, the filter bit 0x40 set in every node's collision (child nodes walked,
/// a child that is not a node and a null child passed over) and the world object updated; off
/// again does nothing; on clears both.
#[test]
fn object_collision_off_and_on() {
    let mut e = world();
    call_on(&mut e, 0x2789, REF, &[n(0)]);
    assert_eq!(e.mem.u8(REF + 0x5f), REF_FLAG_DISABLE_COLLISION);
    assert_eq!(e.mem.u8(HAVOK + 0x2d), 0xc1);
    assert_eq!(calls_to(UPDATE_FN), vec![vec![WORLD_OBJ]]);
    call_on(&mut e, 0x2789, REF, &[n(0)]);
    assert_eq!(calls_to(UPDATE_FN).len(), 1, "already off");
    call_on(&mut e, 0x2789, REF, &[n(3)]);
    assert_eq!(e.mem.u8(REF + 0x5f), 0);
    assert_eq!(e.mem.u8(HAVOK + 0x2d), 0x81);
    assert_eq!(calls_to(UPDATE_FN).len(), 2);
}

/// An actor (slot `0x100` the shared `return true`) with a middle or high process: its
/// character controller's object gets the bit too; a low process's does not.
#[test]
fn an_actors_controller_too() {
    let mut e = world();
    e.mem.set_u32(REF_VT + 0x100, RETURN_TRUE);
    e.mem.set_u32(REF + PROCESS, PROC);
    e.mem.set_u32(PROC + 0x138, WORDS);
    e.mem.set_u32(WORDS + 0x594, CONTROLLER);
    e.mem.set_u32(CONTROLLER + 8, CTRL_OBJ);
    e.mem.set_u8(CTRL_OBJ + 0x2d, 0x02);
    e.mem.set_u8(PROC + PROCESS_LEVEL, 2);
    toggle_collision(&mut e, REF, false);
    assert_eq!(e.mem.u8(CTRL_OBJ + 0x2d), 0x02, "low process");
    e.mem.set_u8(PROC + PROCESS_LEVEL, 1);
    toggle_collision(&mut e, REF, false);
    assert_eq!(e.mem.u8(CTRL_OBJ + 0x2d), 0x42);
    toggle_collision(&mut e, REF, true);
    assert_eq!(e.mem.u8(CTRL_OBJ + 0x2d), 0x02);
    // No 3D: the controller only.
    e.mem.set_u32(REF + LOADED_DATA, 0);
    toggle_collision(&mut e, REF, false);
    assert_eq!(e.mem.u8(CTRL_OBJ + 0x2d), 0x42);
}
