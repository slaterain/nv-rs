//! Tests of the recipe commands, at raw offsets (the recipe loader `005A8110` and save
//! `005A8490`): skill +0x24, skill level +0x28, category id +0x2C, subcategory id +0x30,
//! conditions +0x34, inputs +0x3C, outputs +0x44, category +0x54, subcategory +0x58; a
//! component's count +0, item +4; a condition's 0x1C bytes. The data handler's recipes at +0xA0.

use super::*;
use crate::commands::test_support::*;
use nvse::vars::{Container, Data};

const RCPE: u32 = FORMS + 0x1000;
const RCPE2: u32 = FORMS + 0x1100;
const CAT: u32 = FORMS + 0x1200;
const SUB: u32 = FORMS + 0x1300;
const MISC: u32 = FORMS + 0x1400;
const AMMO: u32 = FORMS + 0x1500;
const WEAP: u32 = FORMS + 0x1600;
const OTHER: u32 = FORMS + 0x1700;
const COMP: u32 = FORMS + 0x4000;
const NODES: u32 = FORMS + 0x5000;
const DH: u32 = FORMS + 0x6000;
const DATA_HANDLER: u32 = 0x011c_3f2c;
/// The recipe's size plus one word.
const SIZE: u32 = 0x60;

/// Recipe 1: inputs misc x2, ammo x5 (a second node); output a weapon x1; recipe 2 (category
/// SUB): input misc x3, outputs ammo and the weapon. The data handler lists recipe 1, an empty
/// node, recipe 2.
fn setup() -> Engine {
    let mut e = engine();
    e.map(0x011c_3000, 0x1000);
    form(&mut e, RCPE, 0x6a, 0x0000_1a01);
    form(&mut e, RCPE2, 0x6a, 0x0000_1a02);
    form(&mut e, CAT, 0x6b, 0x0000_ca01);
    form(&mut e, SUB, 0x6b, 0x0000_ca02);
    form(&mut e, MISC, 0x1f, 0x0000_0999);
    form(&mut e, AMMO, 0x29, 0x0000_0a33);
    form(&mut e, WEAP, 0x28, 0x0000_0e44);
    form(&mut e, OTHER, 0x1f, 0x0000_0555);
    for (i, f) in [RCPE, RCPE2, CAT, SUB, MISC, AMMO, WEAP, OTHER]
        .iter()
        .enumerate()
    {
        set_ref(&mut e, i as u16 + 1, *f);
    }
    let comp = |i: u32| COMP + 8 * i;
    for (i, (count, item)) in [
        (2, MISC),
        (5, AMMO),
        (1, WEAP),
        (3, MISC),
        (4, AMMO),
        (6, WEAP),
    ]
    .iter()
    .enumerate()
    {
        e.mem.set_u32(comp(i as u32), *count);
        e.mem.set_u32(comp(i as u32) + 4, *item);
    }
    e.mem.set_u32(RCPE + 0x3c, comp(0));
    e.mem.set_u32(RCPE + 0x40, NODES);
    e.mem.set_u32(NODES, comp(1));
    e.mem.set_u32(RCPE + 0x44, comp(2));
    e.mem.set_u32(RCPE2 + 0x3c, comp(3));
    e.mem.set_u32(RCPE2 + 0x44, comp(4));
    e.mem.set_u32(RCPE2 + 0x48, NODES + 8);
    e.mem.set_u32(NODES + 8, comp(5));
    e.mem.set_u32(RCPE2 + 0x58, SUB);
    e.mem.set_u32(DATA_HANDLER, DH);
    e.mem.set_u32(DH + 0xa0, RCPE);
    e.mem.set_u32(DH + 0xa4, NODES + 0x10);
    e.mem.set_u32(NODES + 0x14, NODES + 0x18);
    e.mem.set_u32(NODES + 0x18, RCPE2);
    e.mem.set_u32(SCRIPT + 0xc, 0x0700_0001);
    CALLS.with(|c| c.borrow_mut().clear());
    e
}

/// The recipe's bytes (plus one word) with the word at `off` set to `value`.
fn with_word(e: &Engine, at: u32, off: usize, value: u32) -> Vec<u8> {
    let mut want = e.mem.bytes(at, SIZE);
    want[off..off + 4].copy_from_slice(&value.to_le_bytes());
    want
}

/// The form ids of a result's array (`None` for no array).
fn ids(e: &Engine, result: f64) -> Option<Vec<u32>> {
    let a = nvse::vars::array(e, result as u32)?;
    assert_eq!(
        (a.packed, a.owning_mod),
        (true, 7),
        "packed, the script's mod"
    );
    let Container::Array(v) = &a.elements else {
        panic!("a packed array")
    };
    Some(
        v.iter()
            .map(|el| match el.data {
                Data::Form(id) => id,
                ref d => panic!("a form element, not {d:?}"),
            })
            .collect(),
    )
}

#[test]
fn the_skill_and_its_level() {
    let mut e = setup();
    e.mem.set_u32(RCPE + 0x24, 0xffff_fffe);
    e.mem.set_u32(RCPE + 0x28, 45);
    assert_eq!(call(&mut e, 0x225a, &[r(1)]).1, -2.0, "(int)");
    assert_eq!(call(&mut e, 0x225c, &[r(1)]).1, 45.0);
    assert_eq!(call(&mut e, 0x225a, &[r(5)]).1, -1.0, "a misc item: -1");
    assert_eq!(call(&mut e, 0x225c, &[r(5)]).1, -1.0, "a misc item: -1");
    let want = with_word(&e, RCPE, 0x24, 41);
    call(&mut e, 0x225b, &[r(1), n(41)]);
    assert_eq!(e.mem.bytes(RCPE, SIZE), want, "the skill alone");
    let want = with_word(&e, RCPE, 0x28, 0xffff_ffff);
    call(&mut e, 0x225d, &[r(1), n(-1)]);
    assert_eq!(e.mem.bytes(RCPE, SIZE), want, "the level alone");
}

#[test]
fn the_category_and_subcategory() {
    let mut e = setup();
    call(&mut e, 0x22d1, &[r(1)]);
    assert_eq!(form_result(&e), 0, "none");
    let mut want = with_word(&e, RCPE, 0x54, CAT);
    want[0x2c..0x30].copy_from_slice(&0xca01u32.to_le_bytes());
    call(&mut e, 0x22d2, &[r(1), r(3)]);
    assert_eq!(e.mem.bytes(RCPE, SIZE), want, "the category and its id");
    call(&mut e, 0x22d1, &[r(1)]);
    assert_eq!(form_result(&e), 0xca01);
    let mut want = with_word(&e, RCPE, 0x58, SUB);
    want[0x30..0x34].copy_from_slice(&0xca02u32.to_le_bytes());
    call(&mut e, 0x22d4, &[r(1), r(4)]);
    assert_eq!(e.mem.bytes(RCPE, SIZE), want, "the subcategory and its id");
    call(&mut e, 0x22d3, &[r(1)]);
    assert_eq!(form_result(&e), 0xca02);
    let before = e.mem.bytes(RCPE, SIZE);
    call(&mut e, 0x22d2, &[r(1), r(5)]);
    call(&mut e, 0x22d4, &[r(1), r(2)]);
    assert_eq!(
        e.mem.bytes(RCPE, SIZE),
        before,
        "a misc item, a recipe: no category"
    );
}

#[test]
fn the_component_forms() {
    let mut e = setup();
    let (_, res) = call(&mut e, 0x2254, &[r(1)]);
    assert_eq!(ids(&e, res), Some(vec![0x0999, 0x0a33]), "the inputs");
    let (_, res) = call(&mut e, 0x2255, &[r(1)]);
    assert_eq!(ids(&e, res), Some(vec![0x0e44]), "the outputs");
    // An empty list: an empty array all the same.
    e.mem.set_u32(RCPE + 0x44, 0);
    let (_, res) = call(&mut e, 0x2255, &[r(1)]);
    assert_eq!(ids(&e, res), Some(vec![]));
    let (_, res) = call(&mut e, 0x2254, &[r(5)]);
    assert_eq!(res, 0.0, "a misc item: no array");
}

#[test]
fn the_counts() {
    let mut e = setup();
    assert_eq!(call(&mut e, 0x2256, &[r(1), r(6)]).1, 5.0);
    assert_eq!(call(&mut e, 0x2258, &[r(1), r(7)]).1, 1.0);
    assert_eq!(call(&mut e, 0x2256, &[r(1), r(7)]).1, 0.0, "not an input");
    e.mem.set_u32(COMP + 8, 0xffff_ffff);
    assert_eq!(call(&mut e, 0x2256, &[r(1), r(6)]).1, -1.0, "(int)");
    let comps = |e: &Engine| e.mem.bytes(COMP, 0x30);
    let mut want = comps(&e);
    want[8..12].copy_from_slice(&9u32.to_le_bytes());
    call(&mut e, 0x2257, &[r(1), r(6), n(9)]);
    assert_eq!(comps(&e), want, "the ammo's count alone");
    want[0x10..0x14].copy_from_slice(&7u32.to_le_bytes());
    call(&mut e, 0x2259, &[r(1), r(7), n(7)]);
    assert_eq!(comps(&e), want, "the weapon's count alone");
    call(&mut e, 0x2257, &[r(1), r(7), n(1)]);
    assert_eq!(comps(&e), want, "not an input: nothing");
}

#[test]
fn replacing_a_component() {
    let mut e = setup();
    let comps = |e: &Engine| e.mem.bytes(COMP, 0x30);
    let mut want = comps(&e);
    want[0xc..0x10].copy_from_slice(&OTHER.to_le_bytes());
    call(&mut e, 0x225e, &[r(1), r(6), r(8)]);
    assert_eq!(comps(&e), want, "the ammo's item alone");
    want[0x14..0x18].copy_from_slice(&MISC.to_le_bytes());
    call(&mut e, 0x225f, &[r(1), r(7), r(5)]);
    assert_eq!(comps(&e), want, "the weapon's item alone");
    call(&mut e, 0x225e, &[r(1), r(7), r(5)]);
    assert_eq!(comps(&e), want, "not an input: nothing");
}

#[test]
fn adding_a_component() {
    let mut e = setup();
    let block = heap_next(&e);
    call(&mut e, 0x22d8, &[r(1), r(8), n(4)]);
    assert_eq!(
        allocations(),
        vec![8, 8],
        "the component, then the moved first item's node"
    );
    assert_eq!(e.mem.u32(block), 4, "the count");
    assert_eq!(e.mem.u32(block + 4), OTHER, "the item");
    assert_eq!(e.mem.u32(RCPE + 0x3c), block, "first");
    let node = e.mem.u32(RCPE + 0x40);
    assert_eq!((e.mem.u32(node), e.mem.u32(node + 4)), (COMP, NODES));
    // Into an empty output list: the first node takes it, no node made.
    e.mem.set_u32(RCPE + 0x44, 0);
    CALLS.with(|c| c.borrow_mut().clear());
    let block = heap_next(&e);
    call(&mut e, 0x22d9, &[r(1), r(5), n(2)]);
    assert_eq!(allocations(), vec![8]);
    assert_eq!(e.mem.u32(RCPE + 0x44), block);
    assert_eq!((e.mem.u32(block), e.mem.u32(block + 4)), (2, MISC));
}

#[test]
fn removing_a_component() {
    let mut e = setup();
    // The second node: its component and the node freed.
    assert_eq!(call(&mut e, 0x22da, &[r(1), r(6)]).1, 5.0);
    assert_eq!(freed(), vec![COMP + 8, NODES]);
    assert_eq!((e.mem.u32(RCPE + 0x3c), e.mem.u32(RCPE + 0x40)), (COMP, 0));
    // The lone first node: emptied, only its component freed.
    CALLS.with(|c| c.borrow_mut().clear());
    assert_eq!(call(&mut e, 0x22da, &[r(1), r(5)]).1, 2.0);
    assert_eq!(freed(), vec![COMP]);
    assert_eq!(e.mem.u32(RCPE + 0x3c), 0);
    // Now empty: nothing found, nothing freed (JIP would read through null here).
    CALLS.with(|c| c.borrow_mut().clear());
    assert_eq!(call(&mut e, 0x22da, &[r(1), r(5)]).1, 0.0);
    assert!(freed().is_empty());
    // An output of recipe 2 in the first node: the next node's item moves up, that node freed.
    assert_eq!(call(&mut e, 0x22db, &[r(2), r(6)]).1, 4.0);
    assert_eq!(freed(), vec![COMP + 0x20, NODES + 8]);
    assert_eq!(
        (e.mem.u32(RCPE2 + 0x44), e.mem.u32(RCPE2 + 0x48)),
        (COMP + 0x28, 0)
    );
}

#[test]
fn the_recipes_of_a_form() {
    let mut e = setup();
    let (_, res) = call(&mut e, 0x2260, &[r(5)]);
    assert_eq!(
        ids(&e, res),
        Some(vec![0x1a01, 0x1a02]),
        "the misc item's recipes"
    );
    // A recipe with the form twice is listed twice.
    e.mem.set_u32(COMP + 0xc, MISC);
    let (_, res) = call(&mut e, 0x2260, &[r(5)]);
    assert_eq!(ids(&e, res), Some(vec![0x1a01, 0x1a01, 0x1a02]));
    let (_, res) = call(&mut e, 0x2260, &[r(5), r(4)]);
    assert_eq!(ids(&e, res), Some(vec![0x1a02]), "by subcategory");
    let (_, res) = call(&mut e, 0x2260, &[r(5), r(7)]);
    assert_eq!(
        ids(&e, res),
        Some(vec![0x1a01, 0x1a01, 0x1a02]),
        "a weapon is no category: no filter"
    );
    let (_, res) = call(&mut e, 0x2260, &[r(8)]);
    assert_eq!(ids(&e, res), Some(vec![]), "no recipe: an empty array");
}

#[test]
fn the_outputs_of_a_form() {
    let mut e = setup();
    // Recipe 1 gives the weapon, recipe 2 the ammo and the weapon: each once, by address.
    let (_, res) = call(&mut e, 0x2261, &[r(5)]);
    assert_eq!(ids(&e, res), Some(vec![0x0a33, 0x0e44]));
    let (_, res) = call(&mut e, 0x2261, &[r(5), r(4)]);
    assert_eq!(ids(&e, res), Some(vec![0x0a33, 0x0e44]), "recipe 2 alone");
    // By address, not by id: the weapon (the higher address) gets the lower id.
    e.mem.set_u32(WEAP + 0xc, 0x0100);
    let (_, res) = call(&mut e, 0x2261, &[r(5)]);
    assert_eq!(ids(&e, res), Some(vec![0x0a33, 0x0100]));
    e.mem.set_u32(WEAP + 0xc, 0x0e44);
    e.mem.set_u32(RCPE + 0x58, CAT);
    let (_, res) = call(&mut e, 0x2261, &[r(5), r(3)]);
    assert_eq!(ids(&e, res), Some(vec![0x0e44]), "recipe 1 alone");
    let (_, res) = call(&mut e, 0x2261, &[r(8)]);
    assert_eq!(res, 0.0, "nothing: no array");
}

#[test]
fn adding_a_condition() {
    let mut e = setup();
    let first = heap_next(&e);
    call(&mut e, 0x22dc, &[r(1), n(0x48), r(5), n(0x60), z(2.5)]);
    assert_eq!(allocations(), vec![0x1c], "into the empty first node");
    assert_eq!(e.mem.u32(RCPE + 0x34), first);
    assert_eq!(e.mem.u8(first), 0x60, "the comparison");
    assert_eq!(
        e.mem.bytes(first + 1, 3),
        vec![0xcd; 3],
        "the pad as the heap gave it"
    );
    assert_eq!(e.mem.f32(first + 4), 2.5);
    assert_eq!(e.mem.u32(first + 8), 0x48, "the function");
    assert_eq!(e.mem.u32(first + 0xc), MISC);
    assert_eq!(
        e.mem.bytes(first + 0x10, 0xc),
        vec![0; 0xc],
        "the second parameter, run on, reference"
    );
    CALLS.with(|c| c.borrow_mut().clear());
    let second = heap_next(&e);
    call(&mut e, 0x22dc, &[r(1), n(1), r(6), n(0x1ff), z(-1.0)]);
    assert_eq!(allocations(), vec![0x1c, 8], "then a node at the end");
    assert_eq!(e.mem.u8(second), 0xff, "the comparison's low byte");
    let node = e.mem.u32(RCPE + 0x38);
    assert_eq!((e.mem.u32(node), e.mem.u32(node + 4)), (second, 0));
    assert_eq!(e.mem.u32(RCPE + 0x34), first, "the first kept");
}

/// Every command reads its recipe as any form: a misc item filled with a non-zero pattern
/// answers -1, 0 or no array and keeps its bytes; no heap block is touched.
#[test]
fn another_form_type_is_left_alone() {
    let mut e = setup();
    for off in 0x10..SIZE {
        e.mem.set_u8(OTHER + off, 0xa5);
    }
    let before = e.mem.bytes(OTHER, SIZE);
    assert_eq!(call(&mut e, 0x225a, &[r(8)]).1, -1.0);
    assert_eq!(call(&mut e, 0x225c, &[r(8)]).1, -1.0);
    for get in [0x2254, 0x2255] {
        assert_eq!(call(&mut e, get, &[r(8)]).1, 0.0, "{get:04x}");
    }
    for get in [0x2256, 0x2258] {
        assert_eq!(call(&mut e, get, &[r(8), r(5)]).1, 0.0, "{get:04x}");
    }
    for get in [0x22d1, 0x22d3] {
        call(&mut e, get, &[r(8)]);
        assert_eq!(form_result(&e), 0, "{get:04x}");
    }
    call(&mut e, 0x225b, &[r(8), n(1)]);
    call(&mut e, 0x225d, &[r(8), n(1)]);
    call(&mut e, 0x2257, &[r(8), r(5), n(1)]);
    call(&mut e, 0x2259, &[r(8), r(5), n(1)]);
    call(&mut e, 0x225e, &[r(8), r(5), r(6)]);
    call(&mut e, 0x225f, &[r(8), r(5), r(6)]);
    call(&mut e, 0x22d2, &[r(8), r(3)]);
    call(&mut e, 0x22d4, &[r(8), r(3)]);
    call(&mut e, 0x22d8, &[r(8), r(5), n(1)]);
    call(&mut e, 0x22d9, &[r(8), r(5), n(1)]);
    call(&mut e, 0x22da, &[r(8), r(5)]);
    call(&mut e, 0x22db, &[r(8), r(5)]);
    call(&mut e, 0x22dc, &[r(8), n(1), r(5), n(0), z(1.0)]);
    assert_eq!(e.mem.bytes(OTHER, SIZE), before);
    assert!(allocations().is_empty() && freed().is_empty());
}
