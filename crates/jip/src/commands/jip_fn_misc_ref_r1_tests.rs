//! `SynchronizePosition`, called by its opcode. Expectations from JIP's command body
//! (functions_jip/jip_fn_misc_ref.h), its state (`internal/hooks.h`: `s_syncPositionRef`,
//! `s_syncPositionNode`, `s_syncPositionMods`, `s_syncPositionPos`, `s_syncPositionFlags`) and
//! its load housekeeping (`internal/serialization.h`: `s_syncPositionRef = nullptr`), never from
//! the port's output. The player's move goes through JIP's `MoveToCell` (the marker and the
//! game's `MoveToMarker`, a stand-in that records, as the player here has no 3D).

use super::*;
use crate::commands::test_support::*;
use std::cell::RefCell;

const OP: u32 = 0x28d5;
const PC: u32 = FORMS + 0x1000;
const NEAR: u32 = FORMS + 0x2000;
const FAR: u32 = FORMS + 0x3000;
const OTHER: u32 = FORMS + 0x4000;
const LOST: u32 = FORMS + 0x5000;
const CELL1: u32 = FORMS + 0x6000;
const CELL2: u32 = FORMS + 0x7000;
const MARKER: u32 = FORMS + 0x8000;
const STRINGS: u32 = FORMS + 0x9000;
const MOVE_TO_MARKER: u32 = 0x005c_cb20;
const ADD_STRING: u32 = 0x00a5_b690;
const R_NEAR: u16 = 1;
const R_FAR: u16 = 2;
const R_OTHER: u16 = 3;
const R_LOST: u16 = 4;

/// A recorded move: `MoveToMarker`'s words, the marker's cell and position then.
type Move = (Vec<u32>, u32, [f32; 3]);

thread_local! {
    static STRINGS_MADE: RefCell<Vec<Vec<u8>>> = const { RefCell::new(Vec::new()) };
    static MOVES: RefCell<Vec<Move>> = const { RefCell::new(Vec::new()) };
}

/// The fixed string of `text`: 0x40 bytes apart in the order first asked, its count at -8.
fn string_of(text: &[u8]) -> u32 {
    STRINGS_MADE.with(|s| {
        let s = s.borrow();
        let i = s.iter().position(|t| t == text).expect("made");
        STRINGS + 8 + 0x40 * i as u32
    })
}

fn count(e: &Engine, s: u32) -> u32 {
    e.mem.u32(s - 8)
}

fn place(e: &mut Engine, refr: u32, id: u32, cell: u32, pos: [f32; 3]) {
    form(e, refr, 0x3a, id);
    e.mem.set_u32(refr + 0x40, cell);
    for (i, v) in pos.into_iter().enumerate() {
        e.mem.set_f32(refr + 0x30 + 4 * i as u32, v);
    }
}

fn moves() -> Vec<Move> {
    MOVES.with(|m| m.borrow().clone())
}

fn setup() -> Engine {
    STRINGS_MADE.with(|s| s.borrow_mut().clear());
    MOVES.with(|m| m.borrow_mut().clear());
    let mut e = engine();
    e.map(0x011d_e000, 0x1000);
    e.mem.set_u32(0x011d_ea3c, PC);
    place(&mut e, PC, 0x14, CELL1, [0.0, 0.0, 0.0]);
    place(&mut e, NEAR, 0x101, CELL1, [600.0, 800.0, 10.0]);
    place(&mut e, FAR, 0x102, CELL1, [600.0, 800.0, 1024.0]);
    place(&mut e, OTHER, 0x103, CELL2, [1.0, 2.0, 3.0]);
    place(&mut e, LOST, 0x104, 0, [4.0, 5.0, 6.0]);
    set_ref(&mut e, R_NEAR, NEAR);
    set_ref(&mut e, R_FAR, FAR);
    set_ref(&mut e, R_OTHER, OTHER);
    set_ref(&mut e, R_LOST, LOST);
    extra_stand_ins(&mut e);
    e.ext::<crate::Jip>().pos_marker.marker = MARKER;
    register_awaiting(&mut e, AWAITING_H13);
    e.register(MOVE_TO_MARKER, |e, w| {
        let pos = floats3(e, MARKER + 0x30);
        let cell = e.mem.u32(MARKER + 0x40);
        MOVES.with(|m| m.borrow_mut().push((w.to_vec(), cell, pos)));
        Ret::default()
    });
    e.register(ADD_STRING, |e, w| {
        let text = nvse::table::guest_text(e, w[0]);
        let at = STRINGS_MADE.with(|s| {
            let mut s = s.borrow_mut();
            let i = s.iter().position(|t| *t == text).unwrap_or_else(|| {
                s.push(text.clone());
                s.len() - 1
            });
            STRINGS + 8 + 0x40 * i as u32
        });
        let c = e.mem.u32(at - 8);
        e.mem.set_u32(at - 8, c + 1);
        e.mem.write(at, &text);
        Ret {
            eax: at,
            ..Ret::default()
        }
    });
    e
}

#[test]
fn a_near_reference_is_kept_without_a_move() {
    let mut e = setup();
    // 600, 800, 10 away: about 1000.05, not past 1024.
    assert_eq!(
        call_on(&mut e, OP, 0, &[r(R_NEAR), n(1), z(5.5), s(b"Bip01 Head")]),
        (1, 0.0),
        "no result"
    );
    assert!(moves().is_empty());
    let node = string_of(b"Bip01 Head");
    assert_eq!(count(&e, node), 1, "one count held");
    assert_eq!(
        sync_position(&mut e),
        SyncState {
            refr: NEAR,
            node,
            mods: [0.0, 0.0, 5.5, 0.0],
            pos: [600.0, 800.0, 10.0, 0.0],
            flags: 1,
        }
    );
}

#[test]
fn a_far_reference_moves_the_player_to_it_first() {
    let mut e = setup();
    // 600, 800, 1024 away: past 1024.
    call_on(&mut e, OP, 0, &[r(R_FAR)]);
    assert_eq!(
        moves(),
        vec![(vec![PC, MARKER, 0, 0, 0], CELL1, [600.0, 800.0, 1024.0])]
    );
    let s = sync_position(&mut e);
    assert_eq!((s.refr, s.flags, s.mods[2]), (FAR, 0, 0.0));
    assert_eq!(s.node, string_of(b""), "the empty name's fixed string");
    // Another cell with no worldspace: `GetDistance` answers the largest float.
    call_on(&mut e, OP, 0, &[r(R_OTHER), n(-4)]);
    assert_eq!(
        moves()[1],
        (vec![PC, MARKER, 0, 0, 0], CELL2, [1.0, 2.0, 3.0])
    );
    assert_eq!(sync_position(&mut e).flags, 1, "any value but 0 rotates");
    // No cell of either kind: kept, never moved to.
    call_on(&mut e, OP, 0, &[r(R_LOST)]);
    assert_eq!(moves().len(), 2);
    assert_eq!(sync_position(&mut e).refr, LOST);
}

#[test]
fn the_old_names_count_is_given_back() {
    let mut e = setup();
    call_on(&mut e, OP, 0, &[r(R_NEAR), n(0), z(0.0), s(b"A")]);
    let a = string_of(b"A");
    call_on(&mut e, OP, 0, &[r(R_NEAR), n(0), z(0.0), s(b"B")]);
    let b = string_of(b"B");
    assert_eq!((count(&e, a), count(&e, b)), (0, 1));
    call_on(&mut e, OP, 0, &[r(R_NEAR), n(0), z(0.0), s(b"B")]);
    assert_eq!(count(&e, b), 1, "the same name: still one");
    assert_eq!(sync_position(&mut e).node, b);
}

#[test]
fn no_reference_ends_and_moves_back() {
    let mut e = setup();
    call_on(&mut e, OP, 0, &[r(R_FAR), n(1), z(2.0), s(b"N")]);
    // The reference moves on; the player goes back to where it was at the start.
    place(&mut e, FAR, 0x102, CELL2, [9.0, 9.0, 9.0]);
    call_on(&mut e, OP, 0, &[]);
    let s = sync_position(&mut e);
    assert_eq!(s.refr, 0, "forgotten");
    assert_eq!(s.mods[2], 0.0, "the offset's z set to 0 first");
    assert_eq!(s.node, string_of(b"N"), "the name kept");
    assert_eq!(s.pos, [600.0, 800.0, 1024.0, 0.0]);
    assert_eq!(
        moves()[1],
        (vec![PC, MARKER, 0, 0, 0], CELL2, [600.0, 800.0, 1024.0]),
        "into the kept reference's cell now, at the kept position"
    );
    // Nothing kept: no move.
    call_on(&mut e, OP, 0, &[]);
    assert_eq!(moves().len(), 2);
}

#[test]
fn a_load_forgets_the_reference() {
    let mut e = setup();
    call_on(&mut e, OP, 0, &[r(R_NEAR), n(1), z(3.0), s(b"X")]);
    pre_load(&mut e);
    let s = sync_position(&mut e);
    assert_eq!(s.refr, 0);
    assert_eq!((s.flags, s.mods[2]), (1, 3.0), "the rest kept");
}
