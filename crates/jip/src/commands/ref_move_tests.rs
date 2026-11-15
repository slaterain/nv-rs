//! JIP's `TESObjectREFR::SetPos`, `SetAngle` (mode 0), `MoveToCell` and `NiNode::ResetCollision`,
//! expectations from JIP's asm (`nvse/GameObjects.cpp`, `internal/netimmerse.cpp`): the game
//! calls with their words, the fields written, the four lanes of the phantom's position.

use super::*;
use crate::commands::test_support::{self as ts, calls_to, CALLS, FORMS};
use crate::Jip;

pub const REFR: u32 = FORMS + 0x1000;
pub const NODE: u32 = FORMS + 0x2000;
pub const CHILD_NODE: u32 = FORMS + 0x2200;
pub const CHILD_LEAF: u32 = FORMS + 0x2400;
pub const GRANDCHILD: u32 = FORMS + 0x2600;
pub const CHILD_ARRAY: u32 = FORMS + 0x2800;
pub const GRAND_ARRAY: u32 = FORMS + 0x2880;
pub const COL_ROOT: u32 = FORMS + 0x2900;
pub const COL_CHILD: u32 = FORMS + 0x2940;
pub const COL_LEAF: u32 = FORMS + 0x2980;
pub const COL_GRAND: u32 = FORMS + 0x29c0;
pub const LOADED: u32 = FORMS + 0x2a00;
pub const PROCESS_AT: u32 = FORMS + 0x3000;
pub const CTRL: u32 = FORMS + 0x4000;
pub const WRAPPER: u32 = FORMS + 0x5000;
pub const SHAPE: u32 = FORMS + 0x5100;
pub const PROXY: u32 = FORMS + 0x5400;
pub const PHANTOM: u32 = FORMS + 0x5600;
pub const VTBL_OBJECT: u32 = FORMS + 0x8000;
pub const VTBL_ACTOR: u32 = FORMS + 0x8400;
pub const VTBL_NODE: u32 = FORMS + 0x8800;
pub const VTBL_LEAF: u32 = FORMS + 0x8c00;
pub const MARKER: u32 = FORMS + 0x9000;
pub const CELL_A: u32 = FORMS + 0xa000;
pub const CELL_B: u32 = FORMS + 0xa400;
pub const WORLD_1: u32 = FORMS + 0xa800;
pub const WORLD_2: u32 = FORMS + 0xac00;

/// The engine with the stand-ins: `SetLocationOnReference` (records, then writes the three
/// floats it is given into the reference's position, as the game's does), the node update
/// (records, and notes the 12 bytes of its update data), the phantom's `setPosition` (records,
/// and notes the four floats at its position argument), `MoveToMarker` (records), `AddChange`.
pub fn engine() -> Engine {
    let mut e = ts::engine();
    ts::extra_stand_ins(&mut e);
    e.mem.set_u32(VTBL_ACTOR + 0x100, 0x008d_0360);
    e.mem.set_u32(VTBL_NODE + 0xc, 0x0068_15c0);
    e.mem.set_u32(REFR, VTBL_OBJECT);
    e.register(SET_LOCATION, |e, w| {
        CALLS.with(|c| c.borrow_mut().push((SET_LOCATION, w.to_vec())));
        for i in 0..3 {
            let v = e.mem.u32(w[1] + 4 * i);
            e.mem.set_u32(w[0] + 0x30 + 4 * i, v);
        }
        Ret::default()
    });
    e.register(UPDATE_DOWNWARD_PASS, |e, w| {
        let mut words = w.to_vec();
        words.extend((0..3).map(|i| e.mem.u32(w[1] + 4 * i)));
        CALLS.with(|c| c.borrow_mut().push((UPDATE_DOWNWARD_PASS, words)));
        Ret::default()
    });
    e.register(PHANTOM_SET_POSITION, |e, w| {
        let mut words = w.to_vec();
        words.extend((0..4).map(|i| e.mem.u32(w[1] + 4 * i)));
        CALLS.with(|c| c.borrow_mut().push((PHANTOM_SET_POSITION, words)));
        Ret::default()
    });
    e.register(MOVE_TO_MARKER, |_, w| {
        CALLS.with(|c| c.borrow_mut().push((MOVE_TO_MARKER, w.to_vec())));
        Ret::default()
    });
    e
}

/// The reference's 3D: the root `NODE` with a collision object, children: a node (with a
/// collision object and one grandchild node with one), an empty slot, a leaf (not a node, with
/// a collision object that must stay untouched).
pub fn give_3d(e: &mut Engine) {
    e.mem.set_u32(REFR + 0x64, LOADED);
    e.mem.set_u32(LOADED + 0x14, NODE);
    for (node, col) in [
        (NODE, COL_ROOT),
        (CHILD_NODE, COL_CHILD),
        (GRANDCHILD, COL_GRAND),
    ] {
        e.mem.set_u32(node, VTBL_NODE);
        e.mem.set_u32(node + 0x1c, col);
        e.mem.set_u16(col + 0xc, 0x0101);
    }
    e.mem.set_u32(CHILD_LEAF, VTBL_LEAF);
    e.mem.set_u32(CHILD_LEAF + 0x1c, COL_LEAF);
    e.mem.set_u16(COL_LEAF + 0xc, 0x0101);
    e.mem.set_u32(NODE + 0xa0, CHILD_ARRAY);
    e.mem.set_u16(NODE + 0xa6, 3);
    e.mem.set_u32(CHILD_ARRAY, CHILD_NODE);
    e.mem.set_u32(CHILD_ARRAY + 4, 0);
    e.mem.set_u32(CHILD_ARRAY + 8, CHILD_LEAF);
    e.mem.set_u32(CHILD_NODE + 0xa0, GRAND_ARRAY);
    e.mem.set_u16(CHILD_NODE + 0xa6, 1);
    e.mem.set_u32(GRAND_ARRAY, GRANDCHILD);
}

fn reset_bits(e: &Engine) -> [u16; 4] {
    [COL_ROOT, COL_CHILD, COL_GRAND, COL_LEAF].map(|c| e.mem.u16(c + 0xc))
}

/// The actor's chain: process (level 1), controller (state 0, `RotCenter`), phantom wrapper,
/// shape phantom (rotation rows with fourth floats), proxy (velocity, tolerances), phantom.
pub fn make_actor(e: &mut Engine) {
    e.mem.set_u32(REFR, VTBL_ACTOR);
    e.mem.set_u32(REFR + 0x68, PROCESS_AT);
    e.mem.set_u8(PROCESS_AT + 0x28, 1);
    e.mem.set_u32(PROCESS_AT + 0x138, CTRL);
    e.mem.set_u32(CTRL + 0x3f0, 0);
    for (i, v) in [0.5f32, -0.25, 2.0, 7.0].into_iter().enumerate() {
        e.mem.set_f32(CTRL + 0x570 + 4 * i as u32, v);
    }
    e.mem.set_u32(CTRL + 0x594, WRAPPER);
    e.mem.set_u32(WRAPPER + 8, SHAPE);
    let rows = [
        [0.0f32, 1.0, 0.0, 0.125],
        [-1.0, 0.0, 0.0, 0.5],
        [0.0, 0.0, 1.0, -2.0],
    ];
    for (r, row) in rows.iter().enumerate() {
        for (i, v) in row.iter().enumerate() {
            e.mem
                .set_f32(SHAPE + 0xb0 + 0x10 * r as u32 + 4 * i as u32, *v);
        }
    }
    e.mem.set_u32(CTRL + 8, PROXY);
    for i in 0..4 {
        e.mem.set_f32(PROXY + 0x20 + 4 * i, 9.0);
    }
    e.mem.set_u32(PROXY + 0x30, PHANTOM);
    e.mem.set_f32(PROXY + 0x58, 0.1);
    e.mem.set_f32(PROXY + 0x5c, 0.05);
}

fn pos(e: &Engine) -> [f32; 3] {
    [0, 1, 2].map(|i| e.mem.f32(REFR + 0x30 + 4 * i))
}

#[test]
fn set_pos_without_3d_only_calls_the_game() {
    let mut e = engine();
    set_pos_to(&mut e, REFR, [1.0, 2.0, 3.0]);
    let set = calls_to(SET_LOCATION);
    assert_eq!(set.len(), 1);
    assert_eq!(set[0][0], REFR);
    assert_eq!(pos(&e), [1.0, 2.0, 3.0]);
    assert!(calls_to(UPDATE_DOWNWARD_PASS).is_empty());
    assert!(calls_to(PHANTOM_SET_POSITION).is_empty());
}

/// A static with 3D: the root's local translation, every node's collision object marked (not
/// the leaf's), then the update with zeroed data and flags 0.
#[test]
fn set_pos_static_translates_resets_and_updates() {
    let mut e = engine();
    give_3d(&mut e);
    set_pos_to(&mut e, REFR, [10.5, -20.0, 300.0]);
    assert_eq!(
        [0, 1, 2].map(|i| e.mem.f32(NODE + 0x58 + 4 * i)),
        [10.5, -20.0, 300.0]
    );
    assert_eq!(reset_bits(&e), [0x0141, 0x0141, 0x0141, 0x0101]);
    let update = calls_to(UPDATE_DOWNWARD_PASS);
    assert_eq!(update.len(), 1);
    assert_eq!(update[0][0], NODE);
    assert_eq!(
        update[0][2..],
        [0, 0, 0, 0],
        "flags 0, the update data zeroed"
    );
    assert!(calls_to(PHANTOM_SET_POSITION).is_empty());
    assert!(
        calls_to(ts::ADD_CHANGE_FN).is_empty(),
        "SetPos marks nothing itself"
    );
}

/// An actor with a controller: the phantom's position in Havok units plus the turned
/// `RotCenter`, all four lanes as JIP's `mulps`/`addps` make them, the proxy's velocity cleared,
/// the tolerance `keepDistance + keepContactTolerance`; no collision reset; the update.
#[test]
fn set_pos_actor_moves_the_phantom() {
    let mut e = engine();
    give_3d(&mut e);
    make_actor(&mut e);
    e.mem.set_f32(REFR + 0x3c, 1.5);
    set_pos_to(&mut e, REFR, [700.0, -1400.0, 35.0]);
    let k = f32::from_bits(0x3e12_4dd2);
    let p = [700.0f32, -1400.0, 35.0, 1.5];
    let kk = [k, k, k, 0.0];
    let c = [0.5f32, -0.25, 2.0];
    let rows = [
        [0.0f32, 1.0, 0.0, 0.125],
        [-1.0, 0.0, 0.0, 0.5],
        [0.0, 0.0, 1.0, -2.0],
    ];
    let want: [u32; 4] = std::array::from_fn(|i| {
        let mut x = p[i] * kk[i];
        x += c[0] * rows[0][i];
        x += c[1] * rows[1][i];
        x += c[2] * rows[2][i];
        x.to_bits()
    });
    let tol = (0.1f32 + 0.05f32).to_bits();
    let calls = calls_to(PHANTOM_SET_POSITION);
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0][..3], [PHANTOM, PHANTOM + 0xe0, tol]);
    assert_eq!(calls[0][3..], want);
    assert_eq!(
        [0, 1, 2, 3].map(|i| e.mem.u32(PROXY + 0x20 + 4 * i)),
        [0; 4]
    );
    assert_eq!(
        reset_bits(&e),
        [0x0101; 4],
        "no collision reset for an actor"
    );
    assert_eq!(calls_to(UPDATE_DOWNWARD_PASS).len(), 1);
    assert_eq!(e.mem.f32(NODE + 0x5c), -1400.0);
}

/// Each link JIP tests: a missing one (or a far process, or the controller in state 4) leaves
/// the phantom and the collision alone, the update still made.
#[test]
fn set_pos_actor_without_a_full_chain_only_updates() {
    let breaks: [fn(&mut Engine); 6] = [
        |e| e.mem.set_u32(REFR + 0x68, 0),
        |e| e.mem.set_u8(PROCESS_AT + 0x28, 2),
        |e| e.mem.set_u32(PROCESS_AT + 0x138, 0),
        |e| e.mem.set_u32(CTRL + 0x3f0, 4),
        |e| e.mem.set_u32(CTRL + 0x594, 0),
        |e| e.mem.set_u32(WRAPPER + 8, 0),
    ];
    for (n, brk) in breaks.into_iter().enumerate() {
        let mut e = engine();
        give_3d(&mut e);
        make_actor(&mut e);
        brk(&mut e);
        set_pos_to(&mut e, REFR, [1.0, 2.0, 3.0]);
        assert!(calls_to(PHANTOM_SET_POSITION).is_empty(), "case {n}");
        assert_eq!(e.mem.f32(PROXY + 0x20), 9.0, "case {n}: velocity kept");
        assert_eq!(reset_bits(&e), [0x0101; 4], "case {n}");
        assert_eq!(calls_to(UPDATE_DOWNWARD_PASS).len(), 1, "case {n}");
    }
}

/// A process of level 0 also qualifies (JIP's `ja` after `cmp 1`).
#[test]
fn set_pos_actor_high_process_moves_the_phantom() {
    let mut e = engine();
    give_3d(&mut e);
    make_actor(&mut e);
    e.mem.set_u8(PROCESS_AT + 0x28, 0);
    set_pos_to(&mut e, REFR, [1.0, 2.0, 3.0]);
    assert_eq!(calls_to(PHANTOM_SET_POSITION).len(), 1);
}

/// `SetAngle` mode 0 on a static: the angles stored, the matrix of `FromEulerPRY` in the local
/// rotation, the collision reset, the update, and `MarkModified(2)`.
#[test]
fn set_angle_static_turns_the_node() {
    let mut e = engine();
    give_3d(&mut e);
    let pry = [0.5f32, -1.25, 3.0];
    set_angle(&mut e, REFR, pry);
    assert_eq!([0, 1, 2].map(|i| e.mem.f32(REFR + 0x24 + 4 * i)), pry);
    let m = from_euler_pry(pry);
    let got: [u32; 9] = std::array::from_fn(|i| e.mem.u32(NODE + 0x34 + 4 * i as u32));
    assert_eq!(got, m.map(f32::to_bits));
    assert_eq!(reset_bits(&e), [0x0141, 0x0141, 0x0141, 0x0101]);
    assert_eq!(calls_to(UPDATE_DOWNWARD_PASS).len(), 1);
    assert_eq!(
        calls_to(ts::ADD_CHANGE_FN),
        [vec![ts::SAVE_LOAD, REFR, 2, 0]]
    );
}

/// An actor, or a reference without 3D: only the angles and `MarkModified(2)`.
#[test]
fn set_angle_actor_or_no_3d_only_stores_the_angles() {
    for actor in [true, false] {
        let mut e = engine();
        if actor {
            give_3d(&mut e);
            make_actor(&mut e);
        }
        set_angle(&mut e, REFR, [0.0, 0.0, 1.0]);
        assert_eq!(e.mem.f32(REFR + 0x2c), 1.0);
        assert_eq!(e.mem.f32(NODE + 0x34), 0.0, "actor {actor}: node untouched");
        assert!(calls_to(UPDATE_DOWNWARD_PASS).is_empty());
        assert_eq!(
            calls_to(ts::ADD_CHANGE_FN),
            [vec![ts::SAVE_LOAD, REFR, 2, 0]]
        );
    }
}

fn cells(e: &mut Engine) {
    e.mem.set_u32(CELL_A + 0xc0, WORLD_1);
    e.mem.set_u32(CELL_B + 0xc0, WORLD_1);
    e.ext::<Jip>().pos_marker.marker = MARKER;
    e.mem.set_u32(MARKER + 0x40, 0xdead_0000);
    for i in 0..4 {
        e.mem.set_f32(MARKER + 0x30 + 4 * i, -1.0);
    }
    for (i, v) in [0.1f32, 0.2, 0.3, 77.0].into_iter().enumerate() {
        e.mem.set_f32(REFR + 0x24 + 4 * i as u32, v);
    }
}

/// With 3D in the destination cell, or in a cell of the same worldspace: `SetPos`.
#[test]
fn move_to_cell_same_cell_or_worldspace_sets_the_position() {
    for (from, to) in [(CELL_A, CELL_A), (CELL_A, CELL_B)] {
        let mut e = engine();
        give_3d(&mut e);
        cells(&mut e);
        e.mem.set_u32(REFR + 0x40, from);
        move_to_cell_at(&mut e, REFR, to, [4.0, 5.0, 6.0]);
        assert_eq!(calls_to(SET_LOCATION).len(), 1);
        assert!(calls_to(MOVE_TO_MARKER).is_empty());
        assert_eq!(pos(&e), [4.0, 5.0, 6.0]);
        assert_eq!(e.mem.u32(MARKER + 0x40), 0xdead_0000);
    }
}

/// No 3D, no cell, another worldspace, or a destination with no worldspace: through the marker:
/// its cell, the reference's rotation, the position; its scale (+0x3C) left; then `MoveToMarker`.
#[test]
fn move_to_cell_elsewhere_goes_through_the_marker() {
    let cases: [fn(&mut Engine); 4] = [
        |e| e.mem.set_u32(REFR + 0x64, 0),
        |e| e.mem.set_u32(REFR + 0x40, 0),
        |e| e.mem.set_u32(CELL_B + 0xc0, WORLD_2),
        |e| {
            e.mem.set_u32(CELL_A + 0xc0, 0);
            e.mem.set_u32(CELL_B + 0xc0, 0);
        },
    ];
    for (n, case) in cases.into_iter().enumerate() {
        let mut e = engine();
        give_3d(&mut e);
        cells(&mut e);
        e.mem.set_u32(REFR + 0x40, CELL_A);
        case(&mut e);
        move_to_cell_at(&mut e, REFR, CELL_B, [4.0, 5.0, 6.0]);
        assert!(calls_to(SET_LOCATION).is_empty(), "case {n}");
        assert_eq!(e.mem.u32(MARKER + 0x40), CELL_B, "case {n}");
        assert_eq!(
            [0, 1, 2].map(|i| e.mem.f32(MARKER + 0x24 + 4 * i)),
            [0.1, 0.2, 0.3],
            "case {n}"
        );
        assert_eq!(
            [0, 1, 2, 3].map(|i| e.mem.f32(MARKER + 0x30 + 4 * i)),
            [4.0, 5.0, 6.0, -1.0],
            "case {n}"
        );
        assert_eq!(
            calls_to(MOVE_TO_MARKER),
            [vec![REFR, MARKER, 0, 0, 0]],
            "case {n}"
        );
    }
}

/// Before the marker exists: nothing (JIP writes through null).
#[test]
fn move_to_cell_before_the_marker_does_nothing() {
    let mut e = engine();
    cells(&mut e);
    e.ext::<Jip>().pos_marker.marker = 0;
    move_to_cell_at(&mut e, REFR, CELL_B, [4.0, 5.0, 6.0]);
    assert!(calls_to(MOVE_TO_MARKER).is_empty());
    assert!(calls_to(SET_LOCATION).is_empty());
}

/// A node that is its own child: the walk ends (JIP's recursion would not).
#[test]
fn reset_collision_ends_on_a_cycle() {
    let mut e = engine();
    e.mem.set_u32(NODE, VTBL_NODE);
    e.mem.set_u32(NODE + 0x1c, COL_ROOT);
    e.mem.set_u32(NODE + 0xa0, CHILD_ARRAY);
    e.mem.set_u16(NODE + 0xa6, 1);
    e.mem.set_u32(CHILD_ARRAY, NODE);
    reset_collision(&mut e, NODE);
    assert_eq!(e.mem.u8(COL_ROOT + 0xc), 0x40);
}

/// A node listing itself in all 0xFFFF slots: visits plus waiting nodes stay within the bound,
/// so the walk ends without the stack growing past it (it grew by 0xFFFF a visit before).
#[test]
fn reset_collision_bounds_the_waiting_nodes() {
    let mut e = engine();
    let array = FORMS + 0x4_0000;
    e.mem.set_u32(NODE, VTBL_NODE);
    e.mem.set_u32(NODE + 0x1c, COL_ROOT);
    e.mem.set_u32(NODE + 0xa0, array);
    e.mem.set_u16(NODE + 0xa6, 0xffff);
    for i in 0..0xffff {
        e.mem.set_u32(array + 4 * i, NODE);
    }
    let peak = reset_walk(&mut e, NODE);
    assert!(peak < MAX_NODES, "peak {peak}");
    assert_eq!(e.mem.u8(COL_ROOT + 0xc), 0x40);
}

/// The root's translation is the reference's position after the game's call (+0x30), not the
/// argument: a stand-in that moves the reference elsewhere shows which is read.
#[test]
fn set_pos_root_takes_the_position_the_game_set() {
    let mut e = engine();
    give_3d(&mut e);
    e.register(SET_LOCATION, |e, w| {
        CALLS.with(|c| c.borrow_mut().push((SET_LOCATION, w.to_vec())));
        for (i, v) in [11.0f32, 22.0, 33.0].into_iter().enumerate() {
            e.mem.set_f32(w[0] + 0x30 + 4 * i as u32, v);
        }
        Ret::default()
    });
    set_pos_to(&mut e, REFR, [1.0, 2.0, 3.0]);
    let t = [0, 1, 2].map(|i| e.mem.f32(NODE + 0x58 + 4 * i));
    assert_eq!(t, [11.0, 22.0, 33.0]);
}

/// nv-rs's own null checks: no proxy (controller +8): its velocity untouched, no `setPosition`;
/// a proxy without a phantom (+0x30): velocity cleared first, no `setPosition`, nothing written
/// at a phantom's +0xE0. The node's update runs once in both.
#[test]
fn set_pos_actor_without_proxy_or_phantom() {
    for no_proxy in [true, false] {
        let mut e = engine();
        give_3d(&mut e);
        make_actor(&mut e);
        if no_proxy {
            e.mem.set_u32(CTRL + 8, 0);
        } else {
            e.mem.set_u32(PROXY + 0x30, 0);
        }
        set_pos_to(&mut e, REFR, [700.0, -1400.0, 35.0]);
        assert!(calls_to(PHANTOM_SET_POSITION).is_empty());
        let vel = [0, 1, 2, 3].map(|i| e.mem.f32(PROXY + 0x20 + 4 * i));
        assert_eq!(vel, if no_proxy { [9.0; 4] } else { [0.0; 4] });
        assert_eq!(
            [0, 1, 2, 3].map(|i| e.mem.u32(PHANTOM + 0xe0 + 4 * i)),
            [0; 4]
        );
        assert_eq!(calls_to(UPDATE_DOWNWARD_PASS).len(), 1);
    }
}
