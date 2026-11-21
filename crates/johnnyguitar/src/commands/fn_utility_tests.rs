//! Tests of `fn_utility`'s commands, called through their records.

use super::*;
use crate::test_support::*;

/// A form with JohnnyGuitar's getter in its class's +0x130 and a name in the store.
fn named_form(e: &mut Engine, id: u32, name: &[u8]) -> u32 {
    let vtable = e.mem.alloc(0x200);
    e.mem.set_u32(vtable + 0x130, crate::GETTER);
    let form = e.mem.alloc(0x40);
    e.mem.set_u32(form, vtable);
    e.mem.set_u32(form + 0xc, id);
    engine_core::editor_ids::store(e).add_name(form, name);
    engine_core::editor_ids::store(e).map_insert(form, name);
    form
}

/// EditorIDToFormID: the id of the form the name names (case-blind, through the wrap at
/// 0x483A00), in the result's low word; 0 for an unknown name.
#[test]
fn editor_id_to_form_id_finds_the_form() {
    let mut e = engine(true);
    named_form(&mut e, 0x0001_5a3b, b"DocMitchellRef");
    let (_, r) = call(&mut e, EDITOR_ID_TO_FORM_ID, &[s(b"docmitchellref")]);
    assert_eq!(r.to_bits() as u32, 0x0001_5a3b);
    let (_, r) = call(&mut e, EDITOR_ID_TO_FORM_ID, &[s(b"NoSuchForm")]);
    assert_eq!(r.to_bits() as u32, 0);
}

/// GameGetSecondsPassed: the game timer's +0xC over the global at 011AC3A0 (the exe's 007013E0,
/// stood in here by its body).
#[test]
fn seconds_passed_reads_the_game_timer() {
    let mut e = engine(true);
    e.register(SECONDS_PASSED, |e, a| {
        (e.mem.f32(a[0] + 0xc) / 2.0).into_ret()
    });
    if !e.mem.is_mapped(GAME_TIMER) {
        e.map(GAME_TIMER & !0xfff, 0x1000);
    }
    e.mem.set_f32(GAME_TIMER + 0xc, 3.0);
    let (_, r) = call(&mut e, GAME_GET_SECONDS_PASSED, &[]);
    assert_eq!(r, 1.5);
}

/// GetTimePlayed: the clock less the deferred start's, as milliseconds, seconds or minutes.
#[test]
fn time_played_counts_from_the_deferred_start() {
    let mut e = engine(true);
    let clock = std::rc::Rc::new(std::cell::Cell::new(1_000u32));
    let c = clock.clone();
    e.register_double(TICK_COUNT, move |_, _| c.get().into_ret());
    deferred_init(&mut e);
    clock.set(121_000);
    assert_eq!(call(&mut e, GET_TIME_PLAYED, &[int(0)]).1, 120_000.0);
    assert_eq!(call(&mut e, GET_TIME_PLAYED, &[int(1)]).1, 120.0);
    assert_eq!(call(&mut e, GET_TIME_PLAYED, &[int(2)]).1, 2.0);
    assert_eq!(call(&mut e, GET_TIME_PLAYED, &[int(3)]).1, 0.0);
}
