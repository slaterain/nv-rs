//! `SetLinkedReference`, JIP's `SetLinkedRef`, `SetLinkedRefID` and `RestoreLinkedRefs`:
//! expectations from `internal/jip_core.cpp`, `internal/serialization.h` and
//! `functions_jip/jip_fn_misc_ref.h` (the extra made or changed, the maps, the result).

use super::*;
use crate::commands::test_support::{
    self as ts, calls_to, game_globals, in_form_map, r, set_ref, ADD_EXTRA_FN, FORMS,
    REMOVE_BY_TYPE_FN,
};

const REFR: u32 = FORMS + 0x1000;
const LINK_A: u32 = FORMS + 0x1200;
const LINK_B: u32 = FORMS + 0x1400;
const NOT_REF: u32 = FORMS + 0x1600;
const XLINK: u32 = FORMS + 0x1800;
const VT_REF: u32 = FORMS + 0x8000;
const VT_OTHER: u32 = FORMS + 0x8400;
const ID: u32 = 0x0001_0001;
const ID_A: u32 = 0x0001_000a;
const ID_B: u32 = 0x0001_000b;
const ID_NOT: u32 = 0x0001_000c;

fn engine() -> Engine {
    let mut e = ts::engine();
    ts::extra_stand_ins(&mut e);
    game_globals(&mut e);
    e.mem.set_u32(VT_REF + 0xf0, 0x008d_0360);
    for (at, id, n) in [(REFR, ID, 0), (LINK_A, ID_A, 1), (LINK_B, ID_B, 2)] {
        ts::form(&mut e, at, 0x3a, id);
        e.mem.set_u32(at, VT_REF);
        in_form_map(&mut e, at, id, n);
    }
    ts::form(&mut e, NOT_REF, 0x26, ID_NOT);
    e.mem.set_u32(NOT_REF, VT_OTHER);
    in_form_map(&mut e, NOT_REF, ID_NOT, 3);
    e
}

fn list() -> u32 {
    REFR + extra::REFR_EXTRA_LIST
}

fn give_link(e: &mut Engine, link: u32) {
    ts::put_extra(e, list(), XLINK, 0x51, link);
}

fn s(e: &mut Engine) -> &mut Saved {
    &mut e.ext::<Jip>().saved
}

fn default_of(e: &mut Engine) -> Option<u32> {
    s(e).linked_ref_defaults.get(&ID).copied()
}

fn change_of(e: &mut Engine) -> Option<(u32, u8)> {
    s(e).linked_refs.get(&ID).map(|l| (l.link_id, l.mod_index))
}

#[test]
fn set_without_extra_makes_one() {
    let mut e = engine();
    assert!(set_linked_ref(&mut e, REFR, LINK_A, 5));
    let x = extra::get_by_type(&mut e, list(), 0x51);
    assert_ne!(x, 0);
    assert_eq!(e.mem.u32(x), 0x0101_5cc0);
    assert_eq!(e.mem.u32(x + 4), 0x51);
    assert_eq!(e.mem.u32(x + 8), 0);
    assert_eq!(e.mem.u32(x + 0xc), LINK_A);
    assert_eq!(calls_to(ADD_EXTRA_FN), [vec![list(), x]]);
    assert_eq!(default_of(&mut e), Some(0));
    assert_eq!(change_of(&mut e), Some((ID_A, 5)));
}

#[test]
fn set_with_extra_notes_the_old_link() {
    let mut e = engine();
    give_link(&mut e, LINK_A);
    assert!(set_linked_ref(&mut e, REFR, LINK_B, 7));
    assert_eq!(e.mem.u32(XLINK + 0xc), LINK_B);
    assert!(calls_to(ADD_EXTRA_FN).is_empty());
    assert_eq!(default_of(&mut e), Some(ID_A));
    assert_eq!(change_of(&mut e), Some((ID_B, 7)));
}

/// JIP's `s_linkedRefDefault()[refID] = ...` on every change: a second change notes the first
/// change's link as the default.
#[test]
fn second_set_overwrites_the_default() {
    let mut e = engine();
    give_link(&mut e, LINK_A);
    set_linked_ref(&mut e, REFR, LINK_B, 7);
    set_linked_ref(&mut e, REFR, LINK_A, 7);
    assert_eq!(default_of(&mut e), Some(ID_B));
}

#[test]
fn set_with_an_empty_extra_is_refused() {
    let mut e = engine();
    give_link(&mut e, 0);
    assert!(!set_linked_ref(&mut e, REFR, LINK_B, 7));
    assert_eq!(e.mem.u32(XLINK + 0xc), 0);
    assert_eq!(default_of(&mut e), None);
    assert_eq!(change_of(&mut e), None);
}

#[test]
fn clear_with_default_none_removes_the_extra() {
    let mut e = engine();
    set_linked_ref(&mut e, REFR, LINK_A, 5);
    assert!(set_linked_ref(&mut e, REFR, 0, 0xff));
    assert_eq!(calls_to(REMOVE_BY_TYPE_FN), [vec![list(), 0x51]]);
    assert_eq!(default_of(&mut e), None);
    assert_eq!(change_of(&mut e), None);
}

#[test]
fn clear_sets_the_default_link_back() {
    let mut e = engine();
    give_link(&mut e, LINK_A);
    set_linked_ref(&mut e, REFR, LINK_B, 5);
    assert!(set_linked_ref(&mut e, REFR, 0, 0xff));
    assert_eq!(e.mem.u32(XLINK + 0xc), LINK_A);
    assert!(calls_to(REMOVE_BY_TYPE_FN).is_empty());
    assert_eq!(default_of(&mut e), None);
    assert_eq!(change_of(&mut e), None);
}

/// A default that is no longer a reference: the extra keeps its link; both entries go.
#[test]
fn clear_with_a_default_not_a_reference_leaves_the_link() {
    let mut e = engine();
    give_link(&mut e, LINK_B);
    *s(&mut e).linked_ref_defaults.emplace(ID) = ID_NOT;
    *s(&mut e).linked_refs.emplace(ID) = crate::cosave::LinkedRef {
        link_id: ID_B,
        mod_index: 1,
    };
    assert!(set_linked_ref(&mut e, REFR, 0, 0xff));
    assert_eq!(e.mem.u32(XLINK + 0xc), LINK_B);
    assert_eq!(default_of(&mut e), None);
    assert_eq!(change_of(&mut e), None);
}

/// No default: only the change goes (the extra untouched), true.
#[test]
fn clear_without_default_forgets_the_change() {
    let mut e = engine();
    give_link(&mut e, LINK_B);
    *s(&mut e).linked_refs.emplace(ID) = crate::cosave::LinkedRef {
        link_id: ID_B,
        mod_index: 1,
    };
    assert!(set_linked_ref(&mut e, REFR, 0, 0xff));
    assert_eq!(e.mem.u32(XLINK + 0xc), LINK_B);
    assert_eq!(change_of(&mut e), None);
    assert!(calls_to(REMOVE_BY_TYPE_FN).is_empty());
}

#[test]
fn set_by_id_needs_two_references() {
    let mut e = engine();
    assert!(!set_linked_ref_id(&mut e, ID_NOT, ID_A, 1));
    assert!(!set_linked_ref_id(&mut e, ID, ID_NOT, 1));
    assert!(!set_linked_ref_id(&mut e, ID, 0x0001_0fff, 1));
    assert_eq!(change_of(&mut e), None);
    assert!(set_linked_ref_id(&mut e, ID, ID_A, 1));
    assert_eq!(change_of(&mut e), Some((ID_A, 1)));
    // A link id of 0 sets the default back.
    assert!(set_linked_ref_id(&mut e, ID, 0, 9));
    assert_eq!(change_of(&mut e), None);
}

#[test]
fn restore_skips_what_the_load_set_again() {
    let mut e = engine();
    give_link(&mut e, LINK_A);
    set_linked_ref(&mut e, REFR, LINK_B, 5);
    *s(&mut e).linked_refs_temp.emplace(ID) = ID_B;
    restore_linked_refs(&mut e, true);
    assert_eq!(e.mem.u32(XLINK + 0xc), LINK_B);
    restore_linked_refs(&mut e, false);
    assert_eq!(e.mem.u32(XLINK + 0xc), LINK_A);
    assert_eq!(default_of(&mut e), None);
}

#[test]
fn after_load_restores_only_when_links_changed() {
    let mut e = engine();
    give_link(&mut e, LINK_A);
    set_linked_ref(&mut e, REFR, LINK_B, 5);
    crate::cosave::set_changed(&mut e, 0);
    after_load(&mut e);
    assert_eq!(e.mem.u32(XLINK + 0xc), LINK_B);
    crate::cosave::set_changed(&mut e, changed::LINKED_REFS);
    after_load(&mut e);
    assert_eq!(e.mem.u32(XLINK + 0xc), LINK_A);
}

#[test]
fn command_sets_and_marks_the_change() {
    for opcode in [0x2281, 0x2634] {
        let mut e = engine();
        give_link(&mut e, LINK_A);
        set_ref(&mut e, 1, LINK_B);
        let (ok, result) = ts::call_on(&mut e, opcode, REFR, &[r(1)]);
        assert_eq!((ok, result), (1, 1.0));
        assert_eq!(e.mem.u32(XLINK + 0xc), LINK_B);
        assert_eq!(
            crate::cosave::changed_flags(&mut e) & changed::LINKED_REFS,
            changed::LINKED_REFS
        );
    }
}

#[test]
fn command_refused_has_no_result_and_no_change_mark() {
    let mut e = engine();
    give_link(&mut e, 0);
    set_ref(&mut e, 1, LINK_B);
    crate::cosave::set_changed(&mut e, 0);
    let (_, result) = ts::call_on(&mut e, 0x2281, REFR, &[r(1)]);
    assert_eq!(result, 0.0);
    assert_eq!(crate::cosave::changed_flags(&mut e), 0);
}

#[test]
fn command_without_a_link_sets_the_default_back() {
    let mut e = engine();
    give_link(&mut e, LINK_A);
    set_linked_ref(&mut e, REFR, LINK_B, 5);
    let (_, result) = ts::call_on(&mut e, 0x2634, REFR, &[]);
    assert_eq!(result, 1.0);
    assert_eq!(e.mem.u32(XLINK + 0xc), LINK_A);
}

/// Without a calling reference, or with a link argument whose variable holds no form (the read
/// fails): nothing changed, result 0.
#[test]
fn command_without_reference_or_with_a_failed_read_does_nothing() {
    for (this, index) in [(0, 1u16), (REFR, 9)] {
        let mut e = engine();
        give_link(&mut e, LINK_A);
        set_ref(&mut e, 1, LINK_B);
        crate::cosave::set_changed(&mut e, 0);
        let (ok, result) = ts::call_on(&mut e, 0x2281, this, &[r(index)]);
        assert_eq!((ok, result), (1, 0.0), "this {this:x}");
        assert_eq!(e.mem.u32(XLINK + 0xc), LINK_A);
        assert_eq!(crate::cosave::changed_flags(&mut e), 0);
        assert_eq!(change_of(&mut e), None);
    }
}
