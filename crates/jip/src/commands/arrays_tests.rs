//! Tests of JIP's arrays against xNVSE's rules: a packed array of numbers owned by the calling
//! script's mod, forms stored by id (0 for none), maps by text and by number, and the same
//! clean-up as any array an xNVSE command returns (temporary until a variable refers to it).

use super::*;
use crate::commands::test_support::*;
use nvse::vars::{Container, Data};

const QUEST: u32 = FORMS + 0x100;
const NPC: u32 = FORMS + 0x200;

fn setup() -> Engine {
    let mut e = engine();
    form(&mut e, QUEST, 0x47, 0x0000_0a11);
    form(&mut e, NPC, 0x2a, 0x0000_0b22);
    // The calling script is mod 0x05's.
    e.mem.set_u32(SCRIPT + 0xc, 0x0500_1234);
    e
}

fn data(e: &Engine, id: u32) -> Vec<Data> {
    match &vars::array(e, id).expect("the array").elements {
        Container::Array(v) => v.iter().map(|el| el.data.clone()).collect(),
        Container::NumMap(v) => v.iter().map(|(_, el)| el.data.clone()).collect(),
        Container::StrMap(v) => v.iter().map(|(_, el)| el.data.clone()).collect(),
    }
}

#[test]
fn an_array_as_xnvse_makes_it() {
    let mut e = setup();
    let inner = create_array(&mut e, SCRIPT, &[]);
    let id = create_array(
        &mut e,
        SCRIPT,
        &[
            Element::Form(QUEST),
            Element::Form(0),
            Element::Number(2.5),
            Element::Str(b"text".to_vec()),
            Element::Array(inner),
        ],
    );
    let a = vars::array(&e, id).unwrap();
    assert_eq!(
        (a.key_type, a.packed, a.owning_mod),
        (vars::KEY_NUMBER, true, 5),
        "packed numbers, the script's mod"
    );
    assert_eq!(
        data(&e, id),
        vec![
            Data::Form(0x0a11),
            Data::Form(0),
            Data::Number(2.5),
            Data::Str(b"text".to_vec()),
            Data::Array(inner),
        ]
    );
    // No script: mod 0, as `GetModIndex` of none.
    let id = create_array(&mut e, 0, &[Element::Number(1.0)]);
    assert_eq!(vars::array(&e, id).unwrap().owning_mod, 0);
}

#[test]
fn maps_by_text_and_by_number() {
    let mut e = setup();
    let id = create_string_map(
        &mut e,
        SCRIPT,
        &[b"red".to_vec(), b"green".to_vec()],
        &[Element::Number(1.0), Element::Form(NPC)],
    );
    let a = vars::array(&e, id).unwrap();
    assert_eq!(
        (a.key_type, a.packed, a.owning_mod),
        (vars::KEY_STRING, false, 5)
    );
    assert_eq!(
        a.keys(),
        vec![Key::Str(b"green".to_vec()), Key::Str(b"red".to_vec())]
    );
    assert_eq!(data(&e, id), vec![Data::Form(0x0b22), Data::Number(1.0)]);
    let id = create_map(
        &mut e,
        SCRIPT,
        &[7.0, -1.0],
        &[Element::Number(70.0), Element::Number(-10.0)],
    );
    let a = vars::array(&e, id).unwrap();
    assert_eq!(
        (a.key_type, a.packed, a.owning_mod),
        (vars::KEY_NUMBER, false, 5)
    );
    assert_eq!(a.keys(), vec![Key::Number(-1.0), Key::Number(7.0)]);
}

#[test]
fn append_only_to_a_packed_array() {
    let mut e = setup();
    let id = create_array(&mut e, SCRIPT, &[Element::Number(1.0)]);
    append_element(&mut e, id, &Element::Form(QUEST));
    assert_eq!(data(&e, id), vec![Data::Number(1.0), Data::Form(0x0a11)]);
    let map = create_map(&mut e, SCRIPT, &[3.0], &[Element::Number(1.0)]);
    append_element(&mut e, map, &Element::Number(2.0));
    assert_eq!(data(&e, map), vec![Data::Number(1.0)], "a map: nothing");
    append_element(&mut e, 0x7777, &Element::Number(2.0));
    assert!(!vars::array_exists(&e, 0x7777), "a missing array: nothing");
}

/// The array a command returns is temporary: the tick's clean-up deletes it unless a script
/// variable took a reference; arrays held inside it go with it.
#[test]
fn cleaned_up_as_an_xnvse_array() {
    let mut e = setup();
    let inner = create_array(&mut e, SCRIPT, &[Element::Form(QUEST)]);
    let outer = create_array(&mut e, SCRIPT, &[]);
    append_element(&mut e, outer, &Element::Array(inner));
    let lone = create_array(&mut e, SCRIPT, &[Element::Number(1.0)]);
    vars::clean_arrays(&mut e);
    for id in [inner, outer, lone] {
        assert!(
            !vars::array_exists(&e, id),
            "array {id}: unreferenced, gone"
        );
    }
    // Assigned to a variable: kept with what it holds, until the variable lets go.
    let inner = create_array(&mut e, SCRIPT, &[Element::Form(QUEST)]);
    let outer = create_array(&mut e, SCRIPT, &[]);
    append_element(&mut e, outer, &Element::Array(inner));
    let local = FORMS + 0x8000;
    e.mem.set_f64(local, 0.0);
    vars::add_reference_double(&mut e, local, outer, 5);
    vars::clean_arrays(&mut e);
    assert!(vars::array_exists(&e, outer) && vars::array_exists(&e, inner));
    vars::remove_reference_double(&mut e, local, 5);
    vars::clean_arrays(&mut e);
    assert!(!vars::array_exists(&e, outer) && !vars::array_exists(&e, inner));
}

#[test]
fn insert_unique_sorts_by_the_raw_word() {
    let mut t = TempElements::new();
    for f in [0x300, 0x100, 0x200, 0x100, 0x300] {
        t.insert_unique(Element::Form(f));
    }
    assert_eq!(
        t.0,
        vec![
            Element::Form(0x100),
            Element::Form(0x200),
            Element::Form(0x300)
        ]
    );
    assert!(!t.insert_unique(Element::Form(0x200)));
    // Whole numbers share a raw word of 0: the second is taken for the first.
    let mut t = TempElements::new();
    assert!(t.insert_unique(Element::Number(1.0)));
    assert!(!t.insert_unique(Element::Number(2.0)));
    assert!(t.insert_unique(Element::Number(0.1)));
    assert_eq!(t.len(), 2);
    t.append(Element::Number(9.0));
    assert_eq!(t.0.last(), Some(&Element::Number(9.0)));
    t.clear();
    assert!(t.is_empty());
}
