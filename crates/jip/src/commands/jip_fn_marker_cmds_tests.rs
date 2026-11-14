//! `PlaceAtCell`, expectations from JIP's `Cmd_PlaceAtCell_Execute`
//! (`functions_jip/jip_fn_misc_ref.h`): the floats into the marker's position, its cell (a
//! worldspace's persistent cell `+0x34`), the game's `PlaceAtMe(marker, form, count, 0, 0, 1.0)`,
//! the placed reference's id.

use super::*;
use crate::commands::test_support::{self as ts, calls_to, n, r, set_ref, z, CALLS, FORMS};
use crate::Jip;

const MARKER: u32 = FORMS + 0x1000;
const ITEM: u32 = FORMS + 0x2000;
const CELL_FORM: u32 = FORMS + 0x3000;
const WORLD: u32 = FORMS + 0x4000;
const WORLD_CELL: u32 = FORMS + 0x5000;
const QUEST: u32 = FORMS + 0x6000;
const PLACED: u32 = FORMS + 0x7000;
/// What the `PlaceAtMe` stand-in returns.
const PLACE_ANSWER: u32 = FORMS + 0xe000;

/// The engine with the marker made, forms, and `PlaceAtMe` as a stand-in that records its words
/// and returns the word at [`PLACE_ANSWER`].
fn engine() -> Engine {
    let mut e = ts::engine();
    e.ext::<Jip>().pos_marker.marker = MARKER;
    ts::form(&mut e, ITEM, 0x1f, 0x0001_0aaa);
    ts::form(&mut e, CELL_FORM, CELL, 0x0001_0ccc);
    ts::form(&mut e, WORLD, WORLD_SPACE, 0x0001_0ddd);
    e.mem.set_u32(WORLD + 0x34, WORLD_CELL);
    ts::form(&mut e, WORLD_CELL, CELL, 0x0001_0eee);
    ts::form(&mut e, QUEST, 0x47, 0x0001_0fff);
    ts::form(&mut e, PLACED, 0x3a, 0xff00_0801);
    e.mem.set_u32(PLACE_ANSWER, PLACED);
    // The marker's fields before: a cell and a position the command must replace.
    e.mem.set_u32(MARKER + PARENT_CELL, 0xdead_0000);
    for i in 0..3 {
        e.mem.set_f32(MARKER + POSITION + 4 * i, -1.0);
    }
    e.register(PLACE_AT_ME, |e, w| {
        CALLS.with(|c| c.borrow_mut().push((PLACE_AT_ME, w.to_vec())));
        Ret {
            eax: e.mem.u32(PLACE_ANSWER),
            ..Ret::default()
        }
    });
    set_ref(&mut e, 1, ITEM);
    e
}

fn place(e: &mut Engine, where_: u32, count: i32, pos: [f64; 3]) -> (u32, f64) {
    set_ref(e, 2, where_);
    ts::call(e, 0x285d, &[r(1), n(count), r(2), z(pos[0]), z(pos[1]), z(pos[2])])
}

fn position(e: &Engine) -> [f32; 3] {
    [0, 1, 2].map(|i| e.mem.f32(MARKER + POSITION + 4 * i))
}

#[test]
fn place_at_cell_places_through_the_marker() {
    let mut e = engine();
    let (ok, _) = place(&mut e, CELL_FORM, 3, [10.5, -20.25, 4096.0]);
    assert_eq!(ok, 1);
    assert_eq!(position(&e), [10.5, -20.25, 4096.0]);
    assert_eq!(e.mem.u32(MARKER + PARENT_CELL), CELL_FORM);
    assert_eq!(
        calls_to(PLACE_AT_ME),
        [vec![MARKER, ITEM, 3, 0, 0, 1.0f32.to_bits()]]
    );
    assert_eq!(ts::form_result(&e), 0xff00_0801, "the placed reference's id");
}

#[test]
fn place_at_cell_in_a_worldspace_uses_its_persistent_cell() {
    let mut e = engine();
    place(&mut e, WORLD, 1, [1.0, 2.0, 3.0]);
    assert_eq!(e.mem.u32(MARKER + PARENT_CELL), WORLD_CELL);
    assert_eq!(
        calls_to(PLACE_AT_ME),
        [vec![MARKER, ITEM, 1, 0, 0, 1.0f32.to_bits()]]
    );
    assert_eq!(ts::form_result(&e), 0xff00_0801);
}

/// A form that is neither a cell nor a worldspace: nothing placed, the marker's cell kept, but
/// the position read into it all the same (`ExtractArgsEx` wrote it before the type check).
#[test]
fn place_at_cell_other_type_places_nothing() {
    let mut e = engine();
    let (_, result) = place(&mut e, QUEST, 2, [7.0, 8.0, 9.0]);
    assert!(calls_to(PLACE_AT_ME).is_empty());
    assert_eq!(e.mem.u32(MARKER + PARENT_CELL), 0xdead_0000);
    assert_eq!(position(&e), [7.0, 8.0, 9.0]);
    assert_eq!(result, 0.0);
}

/// `PlaceAtMe` placing nothing: no result.
#[test]
fn place_at_cell_nothing_placed_no_result() {
    let mut e = engine();
    e.mem.set_u32(PLACE_ANSWER, 0);
    let (_, result) = place(&mut e, CELL_FORM, 5, [0.0, 0.0, 0.0]);
    assert_eq!(calls_to(PLACE_AT_ME).len(), 1);
    assert_eq!(calls_to(PLACE_AT_ME)[0][2], 5);
    assert_eq!(result, 0.0);
    assert_eq!(ts::form_result(&e), 0);
}

/// Before JIP's deferred start made the marker: nothing (JIP would write through a null one).
#[test]
fn place_at_cell_before_the_marker_does_nothing() {
    let mut e = engine();
    e.ext::<Jip>().pos_marker.marker = 0;
    let (ok, result) = place(&mut e, CELL_FORM, 1, [1.0, 1.0, 1.0]);
    assert_eq!(ok, 1);
    assert!(calls_to(PLACE_AT_ME).is_empty());
    assert_eq!(position(&e), [-1.0, -1.0, -1.0]);
    assert_eq!(result, 0.0);
}

/// A line with only two of the floats (the reader reads the arguments the line has): the third
/// keeps the marker's last value, as `ExtractArgsEx` leaves the field it does not write, and the
/// form is placed there.
#[test]
fn place_at_cell_short_line_keeps_the_marker_value() {
    let mut e = engine();
    set_ref(&mut e, 2, CELL_FORM);
    ts::call(&mut e, 0x285d, &[r(1), n(1), r(2), z(5.0), z(6.0)]);
    assert_eq!(position(&e), [5.0, 6.0, -1.0]);
    assert_eq!(calls_to(PLACE_AT_ME).len(), 1);
    assert_eq!(e.mem.u32(MARKER + PARENT_CELL), CELL_FORM);
}
