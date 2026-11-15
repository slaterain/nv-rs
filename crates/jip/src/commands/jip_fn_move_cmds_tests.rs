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
const KPI: f32 = 0.017_453_292;

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

/// No cell extra and no cell, but an `ExtraPersistentCell` (0x0C): JIP's `GetParentCell` gives
/// its cell, and the reference moves there.
#[test]
fn move_to_editor_position_reference_in_a_persistent_cell() {
    let mut e = engine();
    starting_position(&mut e);
    put_extra(&mut e, REFR + 0x44, FORMS + 0xb500, 0x0c, START_CELL);
    let (_, result) = ts::call_on(&mut e, 0x2759, REFR, &[]);
    assert_eq!(result, 1.0);
    assert_eq!(e.mem.u32(MARKER + 0x40), START_CELL);
}

/// Every command without a calling reference (its arguments all given): no game call, no
/// result. A required argument left out is not a failed read: the reader reads as many
/// arguments as the compiled line counts (xNVSE's `v_ExtractArgsEx` loops over that count, as
/// nvse's reader does), so that case runs on the command's defaults, not into this guard.
#[test]
fn commands_without_reference_do_nothing() {
    let cases: [(u32, Vec<Vec<u8>>); 5] = [
        (0x2758, vec![r(1), z(1.0), z(2.0), z(3.0)]),
        (0x2759, vec![n(1)]),
        (0x27f0, vec![z(1.0), z(2.0), z(3.0)]),
        (0x2763, vec![s(b"1a2b 1 2 3")]),
        (0x2867, vec![r(1), s(b"Bip01 Head")]),
    ];
    for (opcode, args) in cases {
        let mut e = pos_str_engine();
        target(&mut e, true);
        set_ref(&mut e, 1, if opcode == 0x2867 { TARGET } else { CELL_B });
        starting_position(&mut e);
        let (_, result) = ts::call_on(&mut e, opcode, 0, &args);
        assert_eq!(result, 0.0, "{opcode:04x}");
        assert!(calls_to(MOVE_TO_MARKER).is_empty(), "{opcode:04x}");
        assert!(calls_to(SET_LOCATION).is_empty(), "{opcode:04x}");
    }
}

const TARGET: u32 = FORMS + 0xc000;
const T_LOADED: u32 = FORMS + 0xc200;
const T_ROOT: u32 = FORMS + 0xc400;
const T_CHILD: u32 = FORMS + 0xc600;
const T_ARRAY: u32 = FORMS + 0xc800;
const T_NAMES: u32 = FORMS + 0xc900;
const XPERSIST: u32 = FORMS + 0xca00;
const VTBL_N: u32 = FORMS + 0xcc00;

/// The target (in `CELL_B` or in none): its root "Root" at world (-1, -2, -3) with a child node
/// "Bip01 Head" at world (100, 200, 300).
fn target(e: &mut Engine, in_cell: bool) {
    ts::form(e, TARGET, 0x3a, 0x0000_7777);
    e.mem.set_u32(TARGET, FORMS + 0xce00);
    if in_cell {
        e.mem.set_u32(TARGET + 0x40, CELL_B);
    }
    e.mem.set_u32(TARGET + 0x64, T_LOADED);
    e.mem.set_u32(T_LOADED + 0x14, T_ROOT);
    e.mem.set_u32(VTBL_N + 0xc, 0x0068_15c0);
    for node in [T_ROOT, T_CHILD] {
        e.mem.set_u32(node, VTBL_N);
    }
    e.mem.write(T_NAMES, b"Root\0Bip01 Head\0");
    e.mem.set_u32(T_ROOT + 8, T_NAMES);
    e.mem.set_u32(T_CHILD + 8, T_NAMES + 5);
    e.mem.set_u32(T_ROOT + 0xa0, T_ARRAY);
    e.mem.set_u16(T_ROOT + 0xa6, 1);
    e.mem.set_u32(T_ARRAY, T_CHILD);
    for (i, v) in [100.0f32, 200.0, 300.0].into_iter().enumerate() {
        e.mem.set_f32(T_CHILD + 0x8c + 4 * i as u32, v);
    }
    for (i, v) in [-1.0f32, -2.0, -3.0].into_iter().enumerate() {
        e.mem.set_f32(T_ROOT + 0x8c + 4 * i as u32, v);
    }
}

fn move_to_node(e: &mut Engine, name: &[u8], mods: &[f64]) -> (u32, f64) {
    set_ref(e, 1, TARGET);
    let mut args = vec![r(1), s(name)];
    args.extend(mods.iter().map(|&m| z(m)));
    ts::call_on(e, 0x2867, REFR, &args)
}

#[test]
fn move_to_node_moves_to_the_node_plus_offsets() {
    let mut e = engine();
    target(&mut e, true);
    let (ok, result) = move_to_node(&mut e, b"Bip01 Head", &[1.5, -2.0, 0.25]);
    assert_eq!((ok, result), (1, 1.0));
    assert_eq!(e.mem.u32(MARKER + 0x40), CELL_B);
    assert_eq!(marker_pos(&e), [101.5, 198.0, 300.25]);
    assert_eq!(calls_to(MOVE_TO_MARKER).len(), 1);
}

#[test]
fn move_to_node_empty_name_is_the_root_and_offsets_default_to_0() {
    let mut e = engine();
    target(&mut e, true);
    let (_, result) = move_to_node(&mut e, b"", &[]);
    assert_eq!(result, 1.0);
    assert_eq!(marker_pos(&e), [-1.0, -2.0, -3.0]);
}

#[test]
fn move_to_node_uses_the_persistent_cell() {
    let mut e = engine();
    target(&mut e, false);
    put_extra(&mut e, TARGET + 0x44, XPERSIST, 0x0c, CELL_A);
    let (_, result) = move_to_node(&mut e, b"Bip01 Head", &[]);
    assert_eq!(result, 1.0);
    assert_eq!(e.mem.u32(MARKER + 0x40), CELL_A);
}

#[test]
fn move_to_node_without_cell_or_node_does_nothing() {
    for no_cell in [true, false] {
        let mut e = engine();
        target(&mut e, !no_cell);
        let name: &[u8] = if no_cell {
            b"Bip01 Head"
        } else {
            b"Bip01 Tail"
        };
        let (_, result) = move_to_node(&mut e, name, &[]);
        assert_eq!(result, 0.0);
        assert!(calls_to(MOVE_TO_MARKER).is_empty());
    }
}

// ---- lane H3: SetAngleEx, RotateAroundPoint, MoveToNode ----

use crate::commands::ni_matrix::{from_euler_pry_inv, from_euler_ypr, get_rotated_pos};
use crate::commands::nif_nodes::{to_euler_pry, to_euler_pry_inv};

fn angles(e: &Engine) -> [u32; 3] {
    [0, 1, 2].map(|i| e.mem.u32(REFR + 0x24 + 4 * i))
}

fn set_angle_ex(e: &mut Engine, v: [f64; 3], t: Option<i32>) -> (u32, f64) {
    let mut args = vec![z(v[0]), z(v[1]), z(v[2])];
    if let Some(t) = t {
        args.push(n(t));
    }
    ts::call_on(e, 0x27f8, REFR, &args)
}

fn rad(v: [f32; 3]) -> [f32; 3] {
    v.map(|x| x * KPI)
}

#[test]
fn set_angle_ex_absolute_converts_degrees() {
    for t in [None, Some(0), Some(256)] {
        let mut e = engine();
        let (ok, result) = set_angle_ex(&mut e, [90.0, -45.0, 10.0], t);
        assert_eq!((ok, result), (1, 0.0));
        assert_eq!(
            angles(&e),
            rad([90.0, -45.0, 10.0]).map(f32::to_bits),
            "{t:?}"
        );
    }
}

#[test]
fn set_angle_ex_mode_1_inverse() {
    let mut e = engine();
    set_angle_ex(&mut e, [30.0, 20.0, -100.0], Some(1));
    let want = to_euler_pry(&from_euler_pry_inv(rad([30.0, 20.0, -100.0])));
    assert_eq!(angles(&e), want.map(f32::to_bits));
}

/// Mode 2 is `Rotate`: without 3D the angles' matrix (all 0 here: the identity of `FromEulerPRY`)
/// times the turn.
#[test]
fn set_angle_ex_mode_2_rotates() {
    let mut e = engine();
    set_angle_ex(&mut e, [30.0, 20.0, -100.0], Some(2));
    let m = crate::commands::ni_matrix::multiply_matrices(
        &from_euler_pry([0.0; 3]),
        &from_euler_pry_inv(rad([30.0, 20.0, -100.0])),
    );
    assert_eq!(angles(&e), to_euler_pry(&m).map(f32::to_bits));
}

/// Mode 3 adds the current angles (`ToEulerPRYInv` of the root's world rotation with 3D, of the
/// angles' matrix without, or for the player) and turns by mode 1.
#[test]
fn set_angle_ex_mode_3_adds_the_current_angles() {
    for with_3d in [false, true] {
        let mut e = engine();
        let cur = [0.1f32, 0.2, 0.3];
        for (i, v) in cur.into_iter().enumerate() {
            e.mem.set_f32(REFR + 0x24 + 4 * i as u32, v);
        }
        let m = if with_3d {
            give_3d(&mut e);
            let w = [0.0f32, 1.0, 0.0, -1.0, 0.0, 0.0, 0.0, 0.0, 1.0];
            for (i, v) in w.iter().enumerate() {
                e.mem.set_f32(NODE + 0x68 + 4 * i as u32, *v);
            }
            w
        } else {
            from_euler_pry(cur)
        };
        set_angle_ex(&mut e, [30.0, 20.0, -100.0], Some(3));
        let inv = to_euler_pry_inv(&m);
        let r = rad([30.0, 20.0, -100.0]);
        let sum = [r[0] + inv[0], r[1] + inv[1], r[2] + inv[2]];
        let want = to_euler_pry(&from_euler_pry_inv(sum));
        assert_eq!(angles(&e), want.map(f32::to_bits), "3d {with_3d}");
    }
}

/// Mode 4 points at `x y z` itself (no conversion); a value past 4 is `SetAngle` of its low byte
/// (258: the point mode, of the converted values).
#[test]
fn set_angle_ex_point_modes() {
    let mut e = engine();
    for (i, v) in [5.0f32, -1.0, 2.0].into_iter().enumerate() {
        e.mem.set_f32(REFR + 0x30 + 4 * i as u32, v);
    }
    set_angle_ex(&mut e, [5.0, -1.0, -6.0], Some(4));
    assert_eq!(angles(&e)[0], 0x3fc9_0fdb);
    assert_eq!(angles(&e)[2], 0x8000_0000);
    // 258 (low byte 2): the point is the converted values (not straight below the position):
    // the same as mode 4 given those radians.
    let mut e2 = engine();
    let mut e3 = engine();
    for e in [&mut e2, &mut e3] {
        for (i, v) in [5.0f32, -1.0, 2.0].into_iter().enumerate() {
            e.mem.set_f32(REFR + 0x30 + 4 * i as u32, v);
        }
    }
    set_angle_ex(&mut e2, [5.0, -1.0, -6.0], Some(258));
    let r = rad([5.0, -1.0, -6.0]);
    set_angle_ex(&mut e3, [r[0].into(), r[1].into(), r[2].into()], Some(4));
    assert_eq!(angles(&e2), angles(&e3));
    assert_ne!(angles(&e2), angles(&e));
}

#[test]
fn rotate_around_point_cmd_converts_and_skips() {
    let mut e = engine();
    let p = [10.5f32, -4.25, 3.75];
    for (i, v) in p.into_iter().enumerate() {
        e.mem.set_f32(REFR + 0x30 + 4 * i as u32, v);
    }
    let (ok, result) = ts::call_on(
        &mut e,
        0x2926,
        REFR,
        &[z(2.5), z(1.25), z(-0.75), z(30.0), z(-60.0), z(90.0), n(1)],
    );
    assert_eq!((ok, result), (1, 0.0));
    let want = get_rotated_pos(
        &from_euler_ypr(rad([30.0, -60.0, 90.0])),
        p,
        [2.5, 1.25, -0.75],
    );
    assert_eq!(calls_to(SET_LOCATION).len(), 1);
    let got: [u32; 3] = [0, 1, 2].map(|i| e.mem.u32(REFR + 0x30 + 4 * i));
    assert_eq!(got, want.map(f32::to_bits));
    // skipAngles left out (0) without 3D: nothing.
    let mut e = engine();
    ts::call_on(
        &mut e,
        0x2926,
        REFR,
        &[z(2.5), z(1.25), z(-0.75), z(30.0), z(-60.0), z(90.0)],
    );
    assert!(calls_to(SET_LOCATION).is_empty());
}

// ---- MoveToReticle ----

const R_TES: u32 = FORMS + 0xd000;
const R_PLAYER: u32 = FORMS + 0xd100;
const R_PROCESS: u32 = FORMS + 0xd300;
const R_CTRL: u32 = FORMS + 0xd500;
const R_SHAPE: u32 = FORMS + 0xdd00;
const R_HAVOK: u32 = FORMS + 0xdd40;
const R_GRAPH: u32 = FORMS + 0xde00;
const R_CAMERA: u32 = FORMS + 0xdf00;
const R_PICK_HIT: u32 = FORMS + 0xdfc0;
const R_PICK_FRACTION: u32 = FORMS + 0xdfc4;
const R_PICK_FILTER: u32 = FORMS + 0xdfc8;
/// `PS_HKUnitCnvrt` and `kUnitConv`, as JIP's tables hold them.
const FROM_HK: f32 = f32::from_bits(0x40df_f8d6);
const TO_HK: f32 = f32::from_bits(0x3e12_4dd2);

/// The player in `cell`, with the controller chain the cast reads its filter from; the TES (an
/// interior when `interior`); the world scene graph's camera at (700, -1400, 70) looking along
/// +y (identity rotation); the pick answers `hit` with the fraction `fraction`.
fn reticle_world(e: &mut Engine, cell: u32, interior: bool, hit: u32, fraction: f32) {
    e.mem.set_u32(0x011d_ea10, R_TES);
    e.mem.set_u32(R_TES + 0x34, u32::from(interior));
    e.mem.set_u32(0x011d_ea3c, R_PLAYER);
    e.mem.set_u32(R_PLAYER + 0x40, cell);
    e.mem.set_u32(R_PLAYER + 0x68, R_PROCESS);
    e.mem.set_u32(R_PROCESS + 0x138, R_CTRL);
    e.mem.set_u32(R_CTRL + 0x594, R_SHAPE);
    e.mem.set_u32(R_SHAPE + 8, R_HAVOK);
    e.mem.set_u32(R_HAVOK + 0x2c, 0x1234_5678);
    e.mem.set_u32(0x011d_eb7c, R_GRAPH);
    e.mem.set_u32(R_GRAPH + 0xac, R_CAMERA);
    for (i, v) in [1.0f32, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0]
        .into_iter()
        .enumerate()
    {
        e.mem.set_f32(R_CAMERA + 0x68 + 4 * i as u32, v);
    }
    for (i, v) in [700.0f32, -1400.0, 70.0, 1.0].into_iter().enumerate() {
        e.mem.set_f32(R_CAMERA + 0x8c + 4 * i as u32, v);
    }
    e.mem.set_u32(R_PICK_HIT, hit);
    e.mem.set_f32(R_PICK_FRACTION, fraction);
    e.register(0x0045_8440, |e, w| {
        ts::CALLS.with(|c| c.borrow_mut().push((0x0045_8440, w.to_vec())));
        let fraction = e.mem.f32(R_PICK_FRACTION);
        let filter = e.mem.u32(w[1] + 0x24);
        e.mem.set_u32(R_PICK_FILTER, filter);
        e.mem.set_f32(w[1] + 0x40, fraction);
        Ret {
            eax: e.mem.u32(R_PICK_HIT),
            ..Ret::default()
        }
    });
}

/// `RayCastCoords`' point for the camera above at `fraction` of `range`, as JIP computes it:
/// `(from + (to - from) * fraction) * PS_HKUnitCnvrt`, `from` the camera times `kUnitConv`, `to`
/// the column (0, 1, 0) times `range` times `kUnitConv` plus `from`.
fn reticle_point(range: f32, fraction: f32) -> [f32; 3] {
    let from = [700.0f32 * TO_HK, -1400.0 * TO_HK, 70.0 * TO_HK];
    let dir = [0.0f32, 1.0, 0.0];
    let to: [f32; 3] = std::array::from_fn(|i| (dir[i] * range) * TO_HK + from[i]);
    std::array::from_fn(|i| (from[i] + (to[i] - from[i]) * fraction) * FROM_HK)
}

#[test]
fn move_to_reticle_moves_to_the_hit_plus_offsets() {
    let mut e = engine();
    reticle_world(&mut e, CELL_B, false, FORMS + 0xe000, 0.25);
    let (ok, result) = ts::call_on(&mut e, 0x27f3, REFR, &[z(1000.0), z(1.5), z(-2.0), z(3.0)]);
    assert_eq!((ok, result), (1, 1.0));
    let p = reticle_point(1000.0, 0.25);
    assert_eq!(marker_pos(&e), [p[0] + 1.5, p[1] + -2.0, p[2] + 3.0]);
    assert_eq!(e.mem.u32(MARKER + 0x40), CELL_B);
    assert_eq!(calls_to(MOVE_TO_MARKER).len(), 1);
    assert_eq!(calls_to(0x0045_8440).len(), 1);
}

/// No arguments: the range 50000, no offsets; an exterior cell gives its worldspace's
/// persistent cell.
#[test]
fn move_to_reticle_defaults_and_the_persistent_cell() {
    let mut e = engine();
    e.mem.set_u32(CELL_B + 0xc0, WORLD_1);
    reticle_world(&mut e, CELL_B, true, 0, 1.0);
    let (_, result) = ts::call_on(&mut e, 0x27f3, REFR, &[]);
    assert_eq!(result, 1.0);
    assert_eq!(marker_pos(&e), reticle_point(50000.0, 1.0));
    assert_eq!(e.mem.u32(MARKER + 0x40), CELL_A);
}

/// Nothing hit outside an interior (no grid): only `translate` moves it.
#[test]
fn move_to_reticle_without_a_hit_needs_translate() {
    for translate in [0, 1] {
        let mut e = engine();
        reticle_world(&mut e, CELL_B, false, 0, 0.5);
        let (_, result) = ts::call_on(
            &mut e,
            0x27f3,
            REFR,
            &[z(100.0), z(0.0), z(0.0), z(0.0), n(translate)],
        );
        assert_eq!(result, f64::from(translate), "translate {translate}");
        assert_eq!(calls_to(MOVE_TO_MARKER).len(), translate as usize);
        if translate == 1 {
            assert_eq!(marker_pos(&e), reticle_point(100.0, 0.5));
        }
    }
}

/// The player in no cell: no cast, nothing; a worldspace without a persistent cell (nv-rs's
/// own): nothing.
#[test]
fn move_to_reticle_without_a_cell_does_nothing() {
    let mut e = engine();
    reticle_world(&mut e, 0, true, 1, 0.5);
    let (_, result) = ts::call_on(&mut e, 0x27f3, REFR, &[]);
    assert_eq!(result, 0.0);
    assert!(calls_to(0x0045_8440).is_empty());
    let mut e = engine();
    e.mem.set_u32(CELL_B + 0xc0, WORLD_1);
    e.mem.set_u32(WORLD_1 + 0x34, 0);
    reticle_world(&mut e, CELL_B, true, 1, 0.5);
    let (_, result) = ts::call_on(&mut e, 0x27f3, REFR, &[]);
    assert_eq!(result, 0.0);
    assert!(calls_to(MOVE_TO_MARKER).is_empty());
}

/// The cast's filter word: the player's phantom's filter info with its low 16 bits replaced by
/// JIP's layer 6 (`RayCastCoords(..., 6)`).
#[test]
fn move_to_reticle_casts_on_layer_6() {
    let mut e = engine();
    reticle_world(&mut e, CELL_B, true, 1, 0.5);
    ts::call_on(&mut e, 0x27f3, REFR, &[]);
    assert_eq!(e.mem.u32(R_PICK_FILTER), 0x1234_0006);
}

/// Mode 3 for the player with 3D: JIP uses the stored angles' matrix, not the root's world
/// rotation (`rootNode && !thisObj->IsPlayer()`).
#[test]
fn set_angle_ex_mode_3_player_uses_the_stored_angles() {
    let mut e = engine();
    give_3d(&mut e);
    e.mem.set_u32(REFR + 0xc, 0x14);
    let cur = [0.1f32, 0.2, 0.3];
    for (i, v) in cur.into_iter().enumerate() {
        e.mem.set_f32(REFR + 0x24 + 4 * i as u32, v);
    }
    let w = [0.0f32, 1.0, 0.0, -1.0, 0.0, 0.0, 0.0, 0.0, 1.0];
    for (i, v) in w.iter().enumerate() {
        e.mem.set_f32(NODE + 0x68 + 4 * i as u32, *v);
    }
    set_angle_ex(&mut e, [30.0, 20.0, -100.0], Some(3));
    let inv = to_euler_pry_inv(&from_euler_pry(cur));
    let r = rad([30.0, 20.0, -100.0]);
    let sum = [r[0] + inv[0], r[1] + inv[1], r[2] + inv[2]];
    assert_eq!(
        angles(&e),
        to_euler_pry(&from_euler_pry_inv(sum)).map(f32::to_bits)
    );
    // Not the world rotation's.
    let winv = to_euler_pry_inv(&w);
    let wsum = [r[0] + winv[0], r[1] + winv[1], r[2] + winv[2]];
    assert_ne!(
        angles(&e),
        to_euler_pry(&from_euler_pry_inv(wsum)).map(f32::to_bits)
    );
}

/// A global argument whose reference index holds no form: the read fails.
fn bad_float() -> Vec<u8> {
    vec![b'G', 9, 0]
}

/// An opcode, arguments that read, arguments whose read fails.
type Case = (u32, Vec<Vec<u8>>, Vec<Vec<u8>>);

/// The three new commands without a calling reference, and with a read that fails: no game
/// call, no change, result 0.
#[test]
fn new_commands_without_reference_or_with_a_failed_read_do_nothing() {
    let cases: [Case; 3] = [
        (
            0x27f8,
            vec![z(10.0), z(20.0), z(30.0), n(1)],
            vec![z(10.0), z(20.0), bad_float()],
        ),
        (
            0x2926,
            vec![z(1.0), z(2.0), z(3.0), z(10.0), z(20.0), z(30.0), n(1)],
            vec![z(1.0), z(2.0), z(3.0), z(10.0), z(20.0), bad_float()],
        ),
        (0x27f3, vec![z(100.0)], vec![bad_float()]),
    ];
    for (opcode, good, bad) in cases {
        for (this, args) in [(0, &good), (REFR, &bad)] {
            let mut e = engine();
            give_3d(&mut e);
            reticle_world(&mut e, CELL_B, true, 1, 0.5);
            let (ok, result) = ts::call_on(&mut e, opcode, this, args);
            assert_eq!((ok, result), (1, 0.0), "{opcode:04x} this {this:x}");
            assert_eq!(angles(&e), [0; 3], "{opcode:04x}");
            assert!(calls_to(SET_LOCATION).is_empty(), "{opcode:04x}");
            assert!(calls_to(MOVE_TO_MARKER).is_empty(), "{opcode:04x}");
            assert!(calls_to(0x0045_8440).is_empty(), "{opcode:04x}");
            assert!(calls_to(ts::ADD_CHANGE_FN).is_empty(), "{opcode:04x}");
        }
    }
}
