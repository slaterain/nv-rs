//! The marker's making, from JIP's `DeferredInit` (`internal/patches_game.h`): a reference's
//! block of the game's heap (`0x68`, the exe's `CreateReference` size), the game's constructor
//! on it, then `SetTemporary` on what the constructor returned; once; only while JIP answers
//! "installed".

use super::*;
use crate::commands::test_support::{self as ts, calls, calls_to, CALLS, HEAP};
use nvse::plugins::{dispatch, msg};

fn record(addr: u32, w: &[u32]) {
    CALLS.with(|c| c.borrow_mut().push((addr, w.to_vec())));
}

/// The constructor and `SetTemporary` as stand-ins written from the exe: the constructor stores
/// `TESObjectREFR`'s vtable (`0102F55C`) and returns the reference; `SetTemporary` sets flag
/// `0x4000` in `+8`.
fn stand_ins(e: &mut Engine) {
    e.register(REFR_CONSTRUCTOR, |e, w| {
        record(REFR_CONSTRUCTOR, w);
        e.mem.set_u32(w[0], 0x0102_f55c);
        e.mem.set_u32(w[0] + 8, 0);
        Ret {
            eax: w[0],
            ..Ret::default()
        }
    });
    e.register(SET_TEMPORARY, |e, w| {
        record(SET_TEMPORARY, w);
        let flags = e.mem.u32(w[0] + 8);
        e.mem.set_u32(w[0] + 8, flags | 0x4000);
        Ret::default()
    });
}

#[test]
fn deferred_init_makes_the_temporary_marker() {
    let mut e = ts::engine();
    stand_ins(&mut e);
    assert_eq!(marker(&mut e), 0, "none before the message");
    dispatch(&mut e, msg::DEFERRED_INIT, 0, 0);
    assert_eq!(ts::allocations(), [REFR_SIZE]);
    assert_eq!(calls_to(REFR_CONSTRUCTOR), [vec![HEAP]]);
    assert_eq!(calls_to(SET_TEMPORARY), [vec![HEAP]]);
    // The heap first, then the constructor, then SetTemporary.
    let order: Vec<u32> = calls().iter().map(|c| c.0).collect();
    assert_eq!(order, [0x00aa_3e40, REFR_CONSTRUCTOR, SET_TEMPORARY]);
    assert_eq!(marker(&mut e), HEAP);
    assert_eq!(e.mem.u32(HEAP), 0x0102_f55c);
    assert_eq!(e.mem.u32(HEAP + 8) & 0x4000, 0x4000, "temporary");
}

#[test]
fn a_second_deferred_init_makes_no_second_marker() {
    let mut e = ts::engine();
    stand_ins(&mut e);
    dispatch(&mut e, msg::DEFERRED_INIT, 0, 0);
    dispatch(&mut e, msg::DEFERRED_INIT, 0, 0);
    assert_eq!(ts::allocations(), [REFR_SIZE]);
    assert_eq!(calls_to(REFR_CONSTRUCTOR).len(), 1);
    assert_eq!(marker(&mut e), HEAP);
}

#[test]
fn other_messages_make_no_marker() {
    let mut e = ts::engine();
    stand_ins(&mut e);
    for kind in [msg::MAIN_GAME_LOOP, msg::DEFERRED_INIT + 1, 0xffff] {
        dispatch(&mut e, kind, 0, 0);
    }
    assert!(ts::allocations().is_empty());
    assert_eq!(marker(&mut e), 0);
}

/// JIP answering "not installed" (players' state until step 3.10) touches nothing.
#[test]
fn not_installed_makes_no_marker() {
    let mut e = crate::test_support::engine();
    crate::install_as(&mut e, false);
    CALLS.with(|c| c.borrow_mut().clear());
    e.register(0x00aa_3e40, |_, w| {
        record(0x00aa_3e40, w);
        Ret::default()
    });
    stand_ins(&mut e);
    dispatch(&mut e, msg::DEFERRED_INIT, 0, 0);
    assert!(calls().is_empty());
    assert_eq!(marker(&mut e), 0);
}
