//! `MoveToCell`, `MoveToEditorPosition`, `SetPosEx` and `MoveToPosStr`, expectations from JIP's
//! `Cmd_*_Execute` (`functions_jip/jip_fn_misc_ref.h`, `jip_fn_ccc.h`): the cell (a worldspace's
//! persistent cell), the position and angles passed to JIP's `MoveToCell`, `SetPos` and
//! `SetAngle` (whose own expectations are in `ref_move_tests`), the result.

use super::*;
use crate::commands::ref_move::tests::{
    engine as move_engine, give_3d, make_actor, CELL_A, CELL_B, MARKER, NODE, REFR, WORLD_1,
};
use crate::commands::ref_move::{MOVE_TO_MARKER, SET_LOCATION};
use crate::commands::test_support::{
    self as ts, calls_to, game_globals, in_form_map, n, put_extra, r, s, set_ref, z, FORMS,
    SAVE_LOAD,
};
use crate::Jip;

const QUEST: u32 = FORMS + 0xb000;
const XPOS: u32 = FORMS + 0xb400;
const XCELL: u32 = FORMS + 0xb480;
const START_CELL: u32 = FORMS + 0xb800;
/// JIP's `Flt180dPI` and `FltPId180`.
const K180: f32 = 57.295_78;
const KPI: f32 = 0.017_453_292_38;

/// The ref_move stand-ins, the cells as forms, the marker, `WORLD_1`'s persistent cell
/// `CELL_A`, the reference in no cell and without 3D (so moves go through the marker).
fn engine() -> Engine {
    let mut e = move_engine();
    ts::form(&mut e, CELL_A, 0x39, 0x0000_aaaa);
    ts::form(&mut e, CELL_B, 0x39, 0x0000_1a2b);
    ts::form(&mut e, START_CELL, 0x39, 0x0000_cccc);
    ts::form(&mut e, WORLD_1, 0x41, 0x0000_dddd);
    e.mem.set_u32(WORLD_1 + 0x34, CELL_A);
    ts::form(&mut e, QUEST, 0x47, 0x0000_eeee);
    e.ext::<Jip>().pos_marker.marker = MARKER;
    e.mem.set_u32(MARKER + 0x40, 0xdead_0000);
    e
}

fn marker_pos(e: &Engine) -> [f32; 3] {
    [0, 1, 2].map(|i| e.mem.f32(MARKER + 0x30 + 4 * i))
}

fn move_to_cell(e: &mut Engine, form: u32) -> (u32, f64) {
    set_ref(e, 1, form);
    ts::call_on(e, 0x2758, REFR, &[r(1), z(1.5), z(-2.5), z(64.0)])
}

#[test]
fn move_to_cell_moves_into_the_cell() {
    let mut e = engine();
    let (ok, result) = move_to_cell(&mut e, CELL_B);
    assert_eq!((ok, result), (1, 1.0));
    assert_eq!(e.mem.u32(MARKER + 0x40), CELL_B);
    assert_eq!(marker_pos(&e), [1.5, -2.5, 64.0]);
    assert_eq!(calls_to(MOVE_TO_MARKER), [vec![REFR, MARKER, 0, 0, 0]]);
}

#[test]
fn move_to_cell_in_a_worldspace_uses_its_persistent_cell() {
    let mut e = engine();
    let (_, result) = move_to_cell(&mut e, WORLD_1);
    assert_eq!(result, 1.0);
    assert_eq!(e.mem.u32(MARKER + 0x40), CELL_A);
}

/// Another type, or a worldspace with no persistent cell (nv-rs's own): nothing, no result.
#[test]
fn move_to_cell_other_type_does_nothing() {
    for setup in [false, true] {
        let mut e = engine();
        let form = if setup {
            e.mem.set_u32(WORLD_1 + 0x34, 0);
            WORLD_1
        } else {
            QUEST
        };
        let (ok, result) = move_to_cell(&mut e, form);
        assert_eq!((ok, result), (1, 0.0));
        assert!(calls_to(MOVE_TO_MARKER).is_empty());
        assert_eq!(e.mem.u32(MARKER + 0x40), 0xdead_0000);
    }
}

/// With 3D in the same cell: JIP's `SetPos` (the game's call with the position).
#[test]
fn move_to_cell_with_3d_in_the_cell_sets_the_position() {
    let mut e = engine();
    give_3d(&mut e);
    e.mem.set_u32(REFR + 0x40, CELL_B);
    move_to_cell(&mut e, CELL_B);
    assert_eq!(calls_to(SET_LOCATION).len(), 1);
    assert_eq!(e.mem.f32(NODE + 0x60), 64.0);
    assert!(calls_to(MOVE_TO_MARKER).is_empty());
}

fn actor_place(e: &mut Engine) {
    make_actor(e);
    e.mem.set_u32(REFR + 0x170, CELL_B);
    for (i, v) in [100.0f32, 200.0, -300.0].into_iter().enumerate() {
        e.mem.set_f32(REFR + 0x160 + 4 * i as u32, v);
    }
    e.mem.set_f32(REFR + 0x16c, 1.3);
}

/// An actor: its own place; with `resetRot`, the z angle in degrees and back (two roundings),
/// x and y 0, through `SetAngle` (`MarkModified(2)`); result 1.
#[test]
fn move_to_editor_position_actor_with_rotation() {
    let mut e = engine();
    actor_place(&mut e);
    let (ok, result) = ts::call_on(&mut e, 0x2759, REFR, &[n(1)]);
    assert_eq!((ok, result), (1, 1.0));
    assert_eq!(e.mem.u32(MARKER + 0x40), CELL_B);
    assert_eq!(marker_pos(&e), [100.0, 200.0, -300.0]);
    let z = (1.3f32 * K180) * KPI;
    assert_eq!(
        [0, 1, 2].map(|i| e.mem.u32(REFR + 0x24 + 4 * i)),
        [0, 0, z.to_bits()]
    );
    assert_eq!(calls_to(ts::ADD_CHANGE_FN), [vec![SAVE_LOAD, REFR, 2, 0]]);
}

/// No argument, or 0: moved, the angles untouched, nothing marked.
#[test]
fn move_to_editor_position_without_reset_keeps_the_angles() {
    for args in [vec![], vec![n(0)]] {
        let mut e = engine();
        actor_place(&mut e);
        e.mem.set_f32(REFR + 0x2c, 9.0);
        let (_, result) = ts::call_on(&mut e, 0x2759, REFR, &args);
        assert_eq!(result, 1.0);
        assert_eq!(calls_to(MOVE_TO_MARKER).len(), 1);
        assert_eq!(e.mem.f32(REFR + 0x2c), 9.0);
        assert!(calls_to(ts::ADD_CHANGE_FN).is_empty());
    }
}

fn starting_position(e: &mut Engine) {
    put_extra(e, REFR + 0x44, XPOS, 0x0f, 0);
    for (i, v) in [5.0f32, 6.0, 7.0, 0.25, -0.5, 1.0].into_iter().enumerate() {
        e.mem.set_f32(XPOS + 0xc + 4 * i as u32, v);
    }
}

/// Another reference: the starting position extra's position (and angles, each in degrees and
/// back), the starting world or cell extra's cell (a worldspace: its persistent cell).
#[test]
fn move_to_editor_position_reference_uses_its_extras() {
    let mut e = engine();
    starting_position(&mut e);
    put_extra(&mut e, REFR + 0x44, XCELL, 0x49, WORLD_1);
    e.mem.set_u32(REFR + 0x40, START_CELL);
    let (_, result) = ts::call_on(&mut e, 0x2759, REFR, &[n(1)]);
    assert_eq!(result, 1.0);
    assert_eq!(e.mem.u32(MARKER + 0x40), CELL_A);
    assert_eq!(marker_pos(&e), [5.0, 6.0, 7.0]);
    let want = [0.25f32, -0.5, 1.0].map(|v| ((v * K180) * KPI).to_bits());
    assert_eq!([0, 1, 2].map(|i| e.mem.u32(REFR + 0x24 + 4 * i)), want);
}

/// Without the starting world or cell extra: the reference's own cell.
#[test]
fn move_to_editor_position_reference_without_cell_extra_uses_its_cell() {
    let mut e = engine();
    starting_position(&mut e);
    e.mem.set_u32(REFR + 0x40, START_CELL);
    ts::call_on(&mut e, 0x2759, REFR, &[]);
    assert_eq!(e.mem.u32(MARKER + 0x40), START_CELL);
}

/// No starting position extra, or no cell at all, or a cell of another type: nothing.
#[test]
fn move_to_editor_position_without_a_place_does_nothing() {
    for case in 0..3 {
        let mut e = engine();
        match case {
            0 => e.mem.set_u32(REFR + 0x40, START_CELL),
            1 => starting_position(&mut e),
            _ => {
                starting_position(&mut e);
                e.mem.set_u32(REFR + 0x40, QUEST);
            }
        }
        let (_, result) = ts::call_on(&mut e, 0x2759, REFR, &[n(1)]);
        assert_eq!(result, 0.0, "case {case}");
        assert!(calls_to(MOVE_TO_MARKER).is_empty(), "case {case}");
        assert!(calls_to(ts::ADD_CHANGE_FN).is_empty(), "case {case}");
    }
}

fn set_pos_ex(e: &mut Engine, mode: Option<i32>) -> [f32; 3] {
    let mut args = vec![z(10.0), z(-4.0), z(0.5)];
    if let Some(m) = mode {
        args.push(n(m));
    }
    let (ok, result) = ts::call_on(e, 0x27f0, REFR, &args);
    assert_eq!((ok, result), (1, 0.0));
    [0, 1, 2].map(|i| e.mem.f32(REFR + 0x30 + 4 * i))
}

#[test]
fn set_pos_ex_absolute() {
    for mode in [None, Some(0)] {
        let mut e = engine();
        assert_eq!(set_pos_ex(&mut e, mode), [10.0, -4.0, 0.5]);
        assert_eq!(calls_to(SET_LOCATION).len(), 1);
    }
}

#[test]
fn set_pos_ex_relative_adds_the_position() {
    let mut e = engine();
    for (i, v) in [1.0f32, 2.0, 3.25].into_iter().enumerate() {
        e.mem.set_f32(REFR + 0x30 + 4 * i as u32, v);
    }
    assert_eq!(set_pos_ex(&mut e, Some(2)), [11.0, -2.0, 3.75]);
}

/// Mode 1 with 3D: each world rotation row times the offsets (`(a0 + a1) + (a2 + a3)`, a3 the
/// next float times 0) plus the world translation.
#[test]
fn set_pos_ex_translated_by_the_node() {
    let mut e = engine();
    give_3d(&mut e);
    let m = [0.0f32, -1.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0];
    for (i, v) in m.into_iter().enumerate() {
        e.mem.set_f32(NODE + 0x68 + 4 * i as u32, v);
    }
    let t = [100.0f32, 200.0, 300.0];
    for (i, v) in t.into_iter().enumerate() {
        e.mem.set_f32(NODE + 0x8c + 4 * i as u32, v);
    }
    let o = [10.0f32, -4.0, 0.5];
    let next = [m[3], m[6], t[0]];
    let want: [f32; 3] = std::array::from_fn(|i| {
        let row = (m[3 * i] * o[0] + m[3 * i + 1] * o[1]) + (m[3 * i + 2] * o[2] + next[i] * 0.0);
        row + t[i]
    });
    assert_eq!(set_pos_ex(&mut e, Some(1)), want);
    assert_eq!(want, [104.0, 210.0, 300.5]);
}

/// Mode 1 without 3D: turned by the matrix of the reference's angles, plus its position.
#[test]
fn set_pos_ex_translated_without_3d() {
    let mut e = engine();
    let rot = [0.0f32, 0.0, 1.0];
    for (i, v) in rot.into_iter().enumerate() {
        e.mem.set_f32(REFR + 0x24 + 4 * i as u32, v);
    }
    e.mem.set_f32(REFR + 0x30, 1000.0);
    let m = crate::commands::jip_fn_misc_ref::from_euler_pry(rot);
    let o = [10.0f32, -4.0, 0.5];
    let next = [m[3], m[6], 0.0];
    let p = [1000.0f32, 0.0, 0.0];
    let want: [f32; 3] = std::array::from_fn(|i| {
        let row = (m[3 * i] * o[0] + m[3 * i + 1] * o[1]) + (m[3 * i + 2] * o[2] + next[i] * 0.0);
        row + p[i]
    });
    assert_eq!(set_pos_ex(&mut e, Some(1)), want);
}

/// Any other mode: nothing.
#[test]
fn set_pos_ex_other_mode_does_nothing() {
    let mut e = engine();
    set_pos_ex(&mut e, Some(3));
    assert!(calls_to(SET_LOCATION).is_empty());
}

fn pos_str_engine() -> Engine {
    let mut e = engine();
    game_globals(&mut e);
    in_form_map(&mut e, CELL_B, 0x0000_1a2b, 0);
    in_form_map(&mut e, WORLD_1, 0x0100_0ddd, 1);
    in_form_map(&mut e, QUEST, 0x0000_0eee, 2);
    e.mem.set_u8(SAVE_LOAD + 0x44 + 7, 1);
    e.mem.set_u8(SAVE_LOAD + 0x44 + 5, 0xff);
    e
}

fn pos_str(e: &mut Engine, text: &[u8]) -> f64 {
    ts::call_on(e, 0x2763, REFR, &[s(text)]).1
}

#[test]
fn move_to_pos_str_moves_to_the_cell_and_position() {
    let mut e = pos_str_engine();
    assert_eq!(pos_str(&mut e, b"1A2B 10 -20 30"), 1.0);
    assert_eq!(e.mem.u32(MARKER + 0x40), CELL_B);
    assert_eq!(marker_pos(&e), [10.0, -20.0, 30.0]);
}

/// The id resolved through the load order (file 7 loaded as 1) to a worldspace: its persistent
/// cell; runs of spaces between the parts.
#[test]
fn move_to_pos_str_resolves_the_id() {
    let mut e = pos_str_engine();
    assert_eq!(pos_str(&mut e, b"07000ddd  1   2 3"), 1.0);
    assert_eq!(e.mem.u32(MARKER + 0x40), CELL_A);
    assert_eq!(marker_pos(&e), [1.0, 2.0, 3.0]);
}

/// An id of 0, of a file not loaded, of no form, of another type; fewer than four parts:
/// nothing.
#[test]
fn move_to_pos_str_bad_text_does_nothing() {
    for text in [
        &b"0 1 2 3"[..],
        b"05001A2B 1 2 3",
        b"9999 1 2 3",
        b"0eee 1 2 3",
        b"1A2B",
        b"1A2B 1",
        b"1A2B 1 2",
    ] {
        let mut e = pos_str_engine();
        assert_eq!(
            pos_str(&mut e, text),
            0.0,
            "{}",
            String::from_utf8_lossy(text)
        );
        assert!(calls_to(MOVE_TO_MARKER).is_empty());
    }
}
