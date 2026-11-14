//! Tests of `ln_fn_utility`: the time stamp (the `GetLocalTime` import as a
//! stand-in filling a `SYSTEMTIME`: hour +8, minute +10, second +12), form lists (forms at
//! +0x18, a `tList`) and form arrays.

use super::*;
use crate::commands::test_support::*;
use nvse::vars::{Data, Value as V};

const LIST: u32 = FORMS + 0x1000;
const NODE2: u32 = FORMS + 0x1100;
const NODE3: u32 = FORMS + 0x1110;
const WEAP: u32 = FORMS + 0x2000;
const ARMO: u32 = FORMS + 0x2100;
const MISC: u32 = FORMS + 0x2200;
const REFR: u32 = FORMS + 0x2300;
const REF_VT: u32 = FORMS + 0x3000;

fn setup() -> Engine {
    let mut e = engine();
    game_globals(&mut e);
    e.mem.set_u32(REF_VT + 0xf0, 0x008d_0360);
    form(&mut e, LIST, 0x55, 0x0000_0100);
    form(&mut e, WEAP, 0x28, 0x0000_0201);
    form(&mut e, ARMO, 0x18, 0x0000_0202);
    form(&mut e, MISC, 0x1f, 0x0000_0203);
    form(&mut e, REFR, 0x3a, 0x0000_0204);
    e.mem.set_u32(REFR, REF_VT);
    e.mem.set_u32(REFR + 0x20, ARMO);
    for f in [WEAP, ARMO, MISC] {
        e.mem.set_u32(f, REF_VT + 0x200);
    }
    for (i, (f, id)) in [(WEAP, 0x201), (ARMO, 0x202), (MISC, 0x203), (REFR, 0x204)]
        .iter()
        .enumerate()
    {
        in_form_map(&mut e, *f, *id, i as u32);
    }
    set_ref(&mut e, 1, LIST);
    set_ref(&mut e, 2, WEAP);
    e
}

#[test]
fn the_time_stamp() {
    let mut e = setup();
    e.register(0x00fd_f0d8, |e, w| {
        e.mem.set_u16(w[0] + 8, 7);
        e.mem.set_u16(w[0] + 10, 5);
        e.mem.set_u16(w[0] + 12, 32);
        Ret::default()
    });
    let (_, res) = call(&mut e, 0x2202, &[]);
    assert_eq!(string_result(&e, res), b"07-05-32");
}

#[test]
fn a_form_list_as_an_array() {
    let mut e = setup();
    // WEAP, an empty node, MISC.
    e.mem.set_u32(LIST + 0x18, WEAP);
    e.mem.set_u32(LIST + 0x1c, NODE2);
    e.mem.set_u32(NODE2 + 4, NODE3);
    e.mem.set_u32(NODE3, MISC);
    let (_, res) = call(&mut e, 0x2212, &[r(1)]);
    assert_eq!(
        array_data(&e, res),
        vec![Data::Form(0x201), Data::Form(0x203)]
    );
    // An empty list: an empty array.
    e.mem.set_u32(LIST + 0x18, 0);
    e.mem.set_u32(LIST + 0x1c, 0);
    let (_, res) = call(&mut e, 0x2212, &[r(1)]);
    assert!(res != 0.0 && array_data(&e, res).is_empty());
    // Not a form list: refused by the reader.
    assert_eq!(call(&mut e, 0x2212, &[r(2)]).1, 0.0);
}

#[test]
fn forms_sorted_by_type() {
    let mut e = setup();
    let forms = array_of(
        &mut e,
        vec![
            V::Form(0x201),
            V::Form(0x203),
            V::Number(5.0),
            V::Form(0x204),
            V::Form(0x202),
            V::Form(0x0700_0000),
        ],
    );
    // Armour (0x18, also the reference by its base), then weapons (0x28).
    let types = array_of(&mut e, vec![V::Number(24.9), V::Number(40.0)]);
    let (_, res) = call(&mut e, 0x22ff, &[n(forms as i32), n(types as i32)]);
    // Each type's forms from the end back: ARMO before REFR; MISC left over at the end.
    assert_eq!(
        array_data(&e, res),
        vec![
            Data::Form(0x202),
            Data::Form(0x204),
            Data::Form(0x201),
            Data::Form(0x203)
        ]
    );
    // An empty type array: nothing.
    let none = array_of(&mut e, vec![]);
    assert_eq!(
        call(&mut e, 0x22ff, &[n(forms as i32), n(none as i32)]).1,
        0.0
    );
    assert_eq!(call(&mut e, 0x22ff, &[n(forms as i32), n(9999)]).1, 0.0);
}

#[test]
fn forms_counted_by_type() {
    let mut e = setup();
    let forms = array_of(
        &mut e,
        vec![
            V::Form(0x201),
            V::Form(0x204),
            V::Form(0x202),
            V::Number(40.0),
        ],
    );
    assert_eq!(call(&mut e, 0x2300, &[n(forms as i32), n(0x18)]).1, 1.0);
    assert_eq!(call(&mut e, 0x2300, &[n(forms as i32), n(0x3a)]).1, 1.0);
    assert_eq!(call(&mut e, 0x2300, &[n(forms as i32), n(0x29)]).1, 0.0);
    // A map is refused.
    let map = nvse::vars::create_array(&mut e, nvse::vars::KEY_NUMBER, false, 0);
    nvse::vars::set_element(&mut e, map, &nvse::vars::Key::Number(3.0), V::Form(0x202));
    e.mem.set_f64(RESULT, 0.0);
    assert_eq!(call(&mut e, 0x2300, &[n(map as i32), n(0x18)]).1, 0.0);
}
