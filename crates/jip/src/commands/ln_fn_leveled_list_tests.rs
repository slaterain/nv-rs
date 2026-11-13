//! Tests of the leveled list commands, at raw offsets: the component at +0x30 of the form, its
//! entries' list at +0x34 (+4), chance none +0x3C, flags +0x3D, global +0x40; an entry's form +0,
//! count +4, level +6, extra +8; an extra's owner +0, global +4, health +8.

use super::*;
use crate::commands::test_support::*;

const LVLI: u32 = FORMS + 0x1000;
const LVLC: u32 = FORMS + 0x1100;
const MISC: u32 = FORMS + 0x1200;
const AMMO: u32 = FORMS + 0x1300;
const WEAP: u32 = FORMS + 0x1400;
const GLOB: u32 = FORMS + 0x1500;
const OTHER: u32 = FORMS + 0x1600;
const ENTRIES: u32 = FORMS + 0x3000;
const EXTRAS: u32 = FORMS + 0x3800;
const NODES: u32 = FORMS + 0x4000;
/// The form up to one word past the component (+0x30 + 0x1C).
const SIZE: u32 = 0x50;
const CONSOLE: u32 = 0x0200_f000;

thread_local! {
    /// The console lines: the format's text and the first six words of its arguments.
    static LINES: std::cell::RefCell<Vec<(Vec<u8>, Vec<u32>)>> =
        const { std::cell::RefCell::new(Vec::new()) };
}

fn entry(i: u32) -> u32 {
    ENTRIES + 0x10 * i
}

fn node(i: u32) -> u32 {
    NODES + 8 * i
}

/// LVLI: entries 0 (MISC, level 1, count 2, health 0.5), 1 (AMMO, level 5, count 3, no extra),
/// 2 (the creature list LVLC, level 10, count 1). LVLC: entry 3 (WEAP, level 2), entry 4 (LVLI:
/// the lists hold each other).
fn setup() -> Engine {
    let mut e = engine();
    form(&mut e, LVLI, 0x34, 0x0000_0c01);
    form(&mut e, LVLC, 0x2c, 0x0000_0c02);
    form(&mut e, MISC, 0x1f, 0x0000_0999);
    form(&mut e, AMMO, 0x29, 0x0000_0a33);
    form(&mut e, WEAP, 0x28, 0x0000_0e44);
    form(&mut e, GLOB, 0x06, 0x0000_0606);
    form(&mut e, OTHER, 0x1f, 0x0000_0555);
    for (i, f) in [LVLI, LVLC, MISC, AMMO, WEAP, GLOB, OTHER]
        .iter()
        .enumerate()
    {
        set_ref(&mut e, i as u16 + 1, *f);
    }
    for (i, (f, level, count, extra)) in [
        (MISC, 1i16, 2i16, EXTRAS),
        (AMMO, 5, 3, 0),
        (LVLC, 10, 1, 0),
        (WEAP, 2, 1, 0),
        (LVLI, 3, 1, 0),
    ]
    .iter()
    .enumerate()
    {
        let at = entry(i as u32);
        e.mem.set_u32(at, *f);
        e.mem.set_u16(at + 4, *count as u16);
        e.mem.set_u16(at + 6, *level as u16);
        e.mem.set_u32(at + 8, *extra);
    }
    e.mem.set_f32(EXTRAS + 8, 0.5);
    e.mem.set_u32(LVLI + 0x34, entry(0));
    e.mem.set_u32(LVLI + 0x38, node(0));
    e.mem.set_u32(node(0), entry(1));
    e.mem.set_u32(node(0) + 4, node(1));
    e.mem.set_u32(node(1), entry(2));
    e.mem.set_u32(LVLC + 0x34, entry(3));
    e.mem.set_u32(LVLC + 0x38, node(2));
    e.mem.set_u32(node(2), entry(4));
    LINES.with(|l| l.borrow_mut().clear());
    e.register(0x0071_b160, |_, _| Ret {
        eax: CONSOLE,
        ..Ret::default()
    });
    e.register(0x0071_d0a0, |e, a| {
        let words = (0..6).map(|i| e.mem.u32(a[2] + 4 * i)).collect();
        LINES.with(|l| {
            l.borrow_mut()
                .push((nvse::table::guest_text(e, a[1]), words))
        });
        Ret::default()
    });
    CALLS.with(|c| c.borrow_mut().clear());
    e
}

fn with_bytes(e: &Engine, at: u32, size: u32, off: usize, bytes: &[u8]) -> Vec<u8> {
    let mut want = e.mem.bytes(at, size);
    want[off..off + bytes.len()].copy_from_slice(bytes);
    want
}

#[test]
fn chance_none_flags_and_global() {
    let mut e = setup();
    e.mem.set_u8(LVLI + 0x3c, 25);
    e.mem.set_u8(LVLI + 0x3d, 0x85);
    assert_eq!(call(&mut e, 0x2222, &[r(1)]).1, 25.0);
    assert_eq!(call(&mut e, 0x2222, &[r(3)]).1, -1.0, "a misc item: -1");
    assert_eq!(
        call(&mut e, 0x228e, &[r(1)]).1,
        f64::from(0x85u8),
        "unsigned"
    );
    let want = with_bytes(&e, LVLI, SIZE, 0x3c, &[100]);
    call(&mut e, 0x2223, &[r(1), n(100)]);
    assert_eq!(e.mem.bytes(LVLI, SIZE), want, "chance none alone");
    call(&mut e, 0x2223, &[r(1), n(101)]);
    call(&mut e, 0x2223, &[r(1), n(-1)]);
    assert_eq!(e.mem.bytes(LVLI, SIZE), want, "above 100: nothing");
    let want = with_bytes(&e, LVLI, SIZE, 0x3d, &[0x07]);
    call(&mut e, 0x228f, &[r(1), n(0x107)]);
    assert_eq!(e.mem.bytes(LVLI, SIZE), want, "the flags' low byte alone");
    call(&mut e, 0x222b, &[r(1)]);
    assert_eq!(form_result(&e), 0, "no global");
    e.mem.set_u32(LVLI + 0x40, GLOB);
    call(&mut e, 0x222b, &[r(1)]);
    assert_eq!(form_result(&e), 0x0606);
}

#[test]
fn the_entries() {
    let mut e = setup();
    assert_eq!(call(&mut e, 0x2224, &[r(1)]).1, 3.0);
    assert_eq!(call(&mut e, 0x2224, &[r(2)]).1, 2.0);
    call(&mut e, 0x2225, &[r(1), n(1)]);
    assert_eq!(form_result(&e), 0x0a33);
    call(&mut e, 0x2225, &[r(1), n(3)]);
    assert_eq!(form_result(&e), 0, "past the end");
    assert_eq!(call(&mut e, 0x2226, &[r(1), n(2)]).1, 10.0);
    assert_eq!(call(&mut e, 0x2227, &[r(1), n(1)]).1, 3.0);
    let entries = |e: &Engine| e.mem.bytes(ENTRIES, 0x60);
    let want = with_bytes(&e, ENTRIES, 0x60, 0x16, &0x8000u16.to_le_bytes());
    call(&mut e, 0x2312, &[r(1), n(1), n(0x1_8000)]);
    assert_eq!(entries(&e), want, "entry 1's level: the low 16 bits");
    assert_eq!(call(&mut e, 0x2226, &[r(1), n(1)]).1, -32768.0, "signed");
    let want = with_bytes(&e, ENTRIES, 0x60, 0x24, &7u16.to_le_bytes());
    call(&mut e, 0x2313, &[r(1), n(2), n(7)]);
    assert_eq!(entries(&e), want, "entry 2's count alone");
    let want = with_bytes(&e, ENTRIES, 0x60, 0, &OTHER.to_le_bytes());
    call(&mut e, 0x2311, &[r(1), n(0), r(7)]);
    assert_eq!(entries(&e), want, "entry 0's form alone");
    call(&mut e, 0x2311, &[r(1), n(5), r(7)]);
    call(&mut e, 0x2312, &[r(1), n(-1), n(1)]);
    assert_eq!(entries(&e), want, "past the end, negative: nothing");
}

#[test]
fn indexes() {
    let mut e = setup();
    assert_eq!(call(&mut e, 0x2221, &[r(1), n(5)]).1, 1.0);
    assert_eq!(call(&mut e, 0x2221, &[r(1), n(4)]).1, -1.0);
    e.mem.set_u16(entry(1) + 6, 0xffff);
    assert_eq!(
        call(&mut e, 0x2221, &[r(1), n(-1)]).1,
        1.0,
        "level -1 as an int"
    );
    assert_eq!(call(&mut e, 0x2221, &[r(1), n(0xffff)]).1, -1.0);
    assert_eq!(call(&mut e, 0x2220, &[r(1), r(2)]).1, 2.0);
    assert_eq!(call(&mut e, 0x2220, &[r(1), r(5)]).1, -1.0);
    assert_eq!(
        call(&mut e, 0x2220, &[r(3), r(3)]).1,
        -1.0,
        "not a list: -1"
    );
}

#[test]
fn adding_a_form() {
    let mut e = setup();
    // Level 3: before entry 1 (level 5), into node 0, entry 1 moved to a new node after it.
    let (block, extra) = (heap_next(&e), heap_next(&e) + 0x10);
    call(&mut e, 0x221e, &[r(1), r(7), n(0x1_0003), n(4), z(50.0)]);
    assert_eq!(
        allocations(),
        vec![0xc, 0xc, 8],
        "the entry, its extra, a node"
    );
    assert!(freed().is_empty());
    assert_eq!(e.mem.u32(block), OTHER);
    assert_eq!(
        (e.mem.u16(block + 4), e.mem.u16(block + 6)),
        (4, 3),
        "count, level"
    );
    assert_eq!(e.mem.u32(block + 8), extra);
    assert_eq!((e.mem.u32(extra), e.mem.u32(extra + 4)), (0, 0));
    assert_eq!(e.mem.f32(extra + 8), 0.5, "50 is a percentage");
    assert_eq!(e.mem.u32(node(0)), block);
    let moved = e.mem.u32(node(0) + 4);
    assert_eq!(
        (e.mem.u32(moved), e.mem.u32(moved + 4)),
        (entry(1), node(1))
    );
    // Level 1, equal to entry 0's: first, entry 0 moved after it.
    CALLS.with(|c| c.borrow_mut().clear());
    let block = heap_next(&e);
    call(&mut e, 0x221e, &[r(1), r(7), n(1), n(1), z(-2.0)]);
    assert_eq!(e.mem.u32(LVLI + 0x34), block);
    assert_eq!(e.mem.f32(block + 0x10 + 8), 0.0, "below 0: 0");
    // Above every level: nothing inserted, both blocks freed.
    CALLS.with(|c| c.borrow_mut().clear());
    let list = e.mem.bytes(LVLI, SIZE);
    let (block, extra) = (heap_next(&e), heap_next(&e) + 0x10);
    call(&mut e, 0x221e, &[r(1), r(7), n(20), n(1), z(1.0)]);
    assert_eq!(allocations(), vec![0xc, 0xc]);
    assert_eq!(
        freed(),
        vec![block, extra],
        "JIP leaves them; the port frees them"
    );
    assert_eq!(e.mem.bytes(LVLI, SIZE), list, "nothing inserted");
    assert_eq!(call(&mut e, 0x2224, &[r(1)]).1, 5.0);
    // An empty list: the first node takes it.
    e.mem.set_u32(LVLC + 0x34, 0);
    e.mem.set_u32(LVLC + 0x38, 0);
    CALLS.with(|c| c.borrow_mut().clear());
    let block = heap_next(&e);
    call(&mut e, 0x221e, &[r(2), r(5), n(9), n(1), z(0.75)]);
    assert_eq!(allocations(), vec![0xc, 0xc]);
    assert_eq!(e.mem.u32(LVLC + 0x34), block);
    assert_eq!(e.mem.f32(block + 0x10 + 8), 0.75, "up to 1: kept");
}

#[test]
fn removing_a_form() {
    let mut e = setup();
    e.mem.set_u32(entry(2), MISC);
    // Entry 0 (the first node: entry 1's node moves up) and entry 2 (node 1).
    assert_eq!(call(&mut e, 0x221f, &[r(1), r(3)]).1, 2.0);
    assert_eq!(freed(), vec![entry(0), node(0), entry(2), node(1)]);
    assert_eq!(
        (e.mem.u32(LVLI + 0x34), e.mem.u32(LVLI + 0x38)),
        (entry(1), 0)
    );
    CALLS.with(|c| c.borrow_mut().clear());
    assert_eq!(call(&mut e, 0x221f, &[r(1), r(3)]).1, 0.0);
    assert!(freed().is_empty());
    // The lone entry: the first node emptied.
    assert_eq!(call(&mut e, 0x221f, &[r(1), r(4)]).1, 1.0);
    assert_eq!(freed(), vec![entry(1)]);
    assert_eq!(e.mem.u32(LVLI + 0x34), 0);
}

#[test]
fn replacing_a_form() {
    let mut e = setup();
    e.mem.set_u32(entry(2), MISC);
    let mut want = e.mem.bytes(ENTRIES, 0x60);
    want[0..4].copy_from_slice(&WEAP.to_le_bytes());
    want[0x20..0x24].copy_from_slice(&WEAP.to_le_bytes());
    call(&mut e, 0x22e8, &[r(1), r(3), r(5)]);
    assert_eq!(e.mem.bytes(ENTRIES, 0x60), want, "every entry of the form");
}

#[test]
fn removing_the_nth_and_clearing() {
    let mut e = setup();
    call(&mut e, 0x2228, &[r(1), n(1)]);
    assert_eq!(freed(), vec![node(0)], "the node; the entry left");
    assert_eq!(e.mem.u32(LVLI + 0x38), node(1));
    CALLS.with(|c| c.borrow_mut().clear());
    call(&mut e, 0x2228, &[r(1), n(5)]);
    assert!(freed().is_empty(), "past the end: nothing");
    call(&mut e, 0x2228, &[r(1), n(-1)]);
    assert_eq!(
        freed(),
        vec![node(1)],
        "negative: the first, the next node moving up"
    );
    assert_eq!(
        (e.mem.u32(LVLI + 0x34), e.mem.u32(LVLI + 0x38)),
        (entry(2), 0)
    );
    CALLS.with(|c| c.borrow_mut().clear());
    call(&mut e, 0x2229, &[r(2)]);
    assert_eq!(freed(), vec![node(2)], "the later nodes");
    assert_eq!(e.mem.u32(LVLC + 0x34), 0);
}

#[test]
fn a_form_deep_inside() {
    let mut e = setup();
    assert_eq!(
        call(&mut e, 0x28da, &[r(1), r(5)]).1,
        1.0,
        "in the nested list"
    );
    assert_eq!(call(&mut e, 0x28da, &[r(1), r(3)]).1, 1.0);
    assert_eq!(call(&mut e, 0x28da, &[r(1), r(7)]).1, 0.0, "the loop ends");
    e.mem.set_u32(entry(3), 0);
    assert_eq!(
        call(&mut e, 0x28da, &[r(2), r(5)]).1,
        0.0,
        "a null form: no list"
    );
}

#[test]
fn the_dump() {
    let mut e = setup();
    e.mem.set_u32(LVLC + 0x34, entry(3));
    e.mem.set_u32(LVLC + 0x38, 0);
    // The forms named need a vtable (not a reference's: +0xF0 is no "true"); MISC's name is
    // set in its full name component (`kBaseComponentOffsets` byte 0).
    for f in [MISC, AMMO, WEAP] {
        e.mem.set_u32(f, FORMS + 0x7800);
    }
    let name = super::super::full_name(&e, MISC);
    e.mem.write(FORMS + 0x7000, b"Wrench\0");
    e.mem.set_u32(name + 4, FORMS + 0x7000);
    e.mem.set_u16(name + 8, 6);
    call(&mut e, 0x222a, &[r(1)]);
    assert!(
        LINES.with(|l| l.borrow().is_empty()),
        "console closed: nothing"
    );
    console(&mut e, true);
    call(&mut e, 0x222a, &[r(1)]);
    let lines = LINES.with(|l| l.borrow().clone());
    let text = |w: u32| String::from_utf8(nvse::table::guest_text(&e, w)).unwrap();
    assert_eq!(lines.len(), 5);
    assert_eq!(lines[0].0, b"Dumping LeveledList [%08X]");
    assert_eq!(lines[0].1[0], 0x0c01);
    let row = b"%s [%08X] Level: %d Count: %d Health: %.2f".to_vec();
    let h = |v: f64| nvse::commands::double_words(v);
    assert_eq!(lines[1].0, row);
    assert_eq!(text(lines[1].1[0]), "Wrench");
    assert_eq!(lines[1].1[1..], [0x0999, 1, 2, h(0.5)[0], h(0.5)[1]]);
    assert_eq!(text(lines[2].1[0]), "", "no name");
    assert_eq!(lines[2].1[1..], [0x0a33, 5, 3, 0, 0], "no extra: health 0");
    assert_eq!(text(lines[3].1[0]), "(LeveledList)");
    assert_eq!(lines[3].1[1], 0x0c02);
    assert_eq!(
        text(lines[4].1[0]),
        "   ",
        "three spaces in, the name empty"
    );
    assert_eq!(lines[4].1[1..3], [0x0e44, 2]);
    console(&mut e, false);
}

/// Every command reads its list as any form: a misc item filled with a non-zero pattern answers
/// -1 or 0 and keeps its bytes; no heap block is touched.
#[test]
fn another_form_type_is_left_alone() {
    let mut e = setup();
    for off in 0x10..SIZE {
        e.mem.set_u8(OTHER + off, 0xa5);
    }
    let before = e.mem.bytes(OTHER, SIZE);
    console(&mut e, true);
    assert_eq!(call(&mut e, 0x2222, &[r(7)]).1, -1.0);
    assert_eq!(call(&mut e, 0x2221, &[r(7), n(1)]).1, -1.0);
    assert_eq!(call(&mut e, 0x2220, &[r(7), r(3)]).1, -1.0);
    for get in [0x2224, 0x228e] {
        assert_eq!(call(&mut e, get, &[r(7)]).1, 0.0, "{get:04x}");
    }
    for get in [0x2226, 0x2227] {
        assert_eq!(call(&mut e, get, &[r(7), n(0)]).1, 0.0, "{get:04x}");
    }
    call(&mut e, 0x2225, &[r(7), n(0)]);
    assert_eq!(form_result(&e), 0);
    call(&mut e, 0x222b, &[r(7)]);
    assert_eq!(form_result(&e), 0);
    assert_eq!(call(&mut e, 0x28da, &[r(7), r(3)]).1, 0.0);
    call(&mut e, 0x221e, &[r(7), r(3), n(1), n(1), z(1.0)]);
    call(&mut e, 0x221f, &[r(7), r(3)]);
    call(&mut e, 0x22e8, &[r(7), r(3), r(4)]);
    call(&mut e, 0x2223, &[r(7), n(5)]);
    call(&mut e, 0x228f, &[r(7), n(5)]);
    call(&mut e, 0x2311, &[r(7), n(0), r(3)]);
    call(&mut e, 0x2312, &[r(7), n(0), n(9)]);
    call(&mut e, 0x2313, &[r(7), n(0), n(9)]);
    call(&mut e, 0x2228, &[r(7), n(0)]);
    call(&mut e, 0x2229, &[r(7)]);
    call(&mut e, 0x222a, &[r(7)]);
    assert_eq!(e.mem.bytes(OTHER, SIZE), before);
    assert!(allocations().is_empty() && freed().is_empty());
    assert!(LINES.with(|l| l.borrow().is_empty()), "no dump");
    console(&mut e, false);
}
