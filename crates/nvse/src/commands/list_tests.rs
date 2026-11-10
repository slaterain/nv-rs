//! Tests of `Commands_List.cpp` through the commands' records (and the condition evals through
//! theirs), on form lists laid out in the test's memory and a bump heap standing in for the
//! game's. The answers pinned are xNVSE's as its source and the shipped DLL (`nvse_1_4.dll`,
//! addresses in the notes) give them; none of these commands has been run in the real game yet.

use super::super::test_support::*;
use super::*;
use crate::functions::{CREATE_EVENT_LIST, EVENT_LIST_DESTROY, SCRIPT_EXECUTE};

const LIST: u32 = 0x0200_0000;
const A: u32 = 0x0200_0400;
const B: u32 = 0x0200_0800;
const C: u32 = 0x0200_0c00;
const REF: u32 = 0x0200_1000;
const REF_BASE: u32 = 0x0200_1400;
const OTHER_LIST: u32 = 0x0200_1800;
const FUNCTION: u32 = 0x0200_2000;
const LEVELED: u32 = 0x0200_3000;
const NODES: u32 = 0x0200_4000;
const HEAP: u32 = 0x0200_8000;
const CONSOLE: u32 = 0x0200_f000;
const LOOKUP_FORM: u32 = 0x0048_39c0;

thread_local! {
    static SEEN: std::cell::RefCell<Vec<(Vec<u8>, Vec<u32>)>> = const { std::cell::RefCell::new(Vec::new()) };
    static RUNS: std::cell::RefCell<Vec<u32>> = const { std::cell::RefCell::new(Vec::new()) };
}

fn eax(v: u32) -> Ret {
    Ret {
        eax: v,
        ..Ret::default()
    }
}

fn seen() -> Vec<(Vec<u8>, Vec<u32>)> {
    SEEN.with(|p| p.borrow_mut().drain(..).collect())
}

fn alloc(e: &mut Engine, size: u32) -> u32 {
    let at = e.mem.u32(HEAP).max(HEAP + 0x10);
    e.mem.set_u32(HEAP, at + size.next_multiple_of(0x10));
    for i in 0..size {
        e.mem.set_u8(at + i, 0);
    }
    at
}

/// A form list (its forms A, B), three forms, a reference (its base `REF_BASE`), a bump heap,
/// prints kept.
fn setup() -> Engine {
    let mut e = engine_with_table();
    e.map(0x011c_3000, 0x1000);
    e.register(0x00ec_a7f4, |_, a| {
        eax(u32::from((a[0] as u8).to_ascii_uppercase()))
    });
    e.register(0x0071_b160, |_, _| eax(CONSOLE));
    e.register(0x0071_d0a0, |e, a| {
        let t = crate::table::guest_text(e, a[1]);
        let words = (0..2).map(|i| e.mem.u32(a[2] + 4 * i)).collect();
        SEEN.with(|p| p.borrow_mut().push((t, words)));
        eax(0)
    });
    e.register(0x0040_1000, |e, a| eax(alloc(e, a[0])));
    e.register(0x0040_1030, |_, _| eax(0));
    e.register(0x0041_0220, |e, a| eax(e.mem.u32(a[0] + 0x100 + 4 * a[1])));
    for (form, kind, id) in [
        (LIST, 0x55, 0x10),
        (A, 0x1f, 0x11),
        (B, 0x1f, 0x12),
        (C, 0x1f, 0x13),
        (REF, 0x3a, 0x14),
        (REF_BASE, 0x1f, 0x15),
        (OTHER_LIST, 0x55, 0x16),
        (LEVELED, 0x2c, 0x17),
    ] {
        e.mem.set_u8(form + 4, kind);
        e.mem.set_u32(form + 0xc, 0x0001_0000 + id);
    }
    set_classes(&mut e, REF, 1);
    e.mem.set_u32(REF + 0x20, REF_BASE);
    e.mem.set_u32(LIST + 0x18, A);
    e.mem.set_u32(LIST + 0x1c, NODES);
    e.mem.set_u32(NODES, B);
    e.mem.set_u32(SCRIPT + 0xc, 0x0500_0d00);
    for (i, f) in [LIST, A, B, C, REF, REF_BASE, OTHER_LIST]
        .into_iter()
        .enumerate()
    {
        set_script_ref(&mut e, i as u16 + 1, f);
    }
    e
}

fn console(e: &mut Engine) {
    e.mem.set_u8(0x011d_ea2e, 1);
    let tls = e.tls();
    e.mem.set_u8(tls + 0x268, 1);
}

fn run(e: &mut Engine, opcode: u32, this: u32, args: &[Vec<u8>]) -> f64 {
    let (ok, result) = call(e, opcode, this, args);
    assert_eq!(ok, 1);
    result
}

fn low(v: f64) -> u32 {
    v.to_bits() as u32
}

/// The list's forms as their ids' low bytes (0 for an empty node), every node.
fn ids(e: &Engine, list: u32) -> Vec<u32> {
    let mut out = Vec::new();
    let mut node = list + 0x18;
    while node != 0 {
        let f = e.mem.u32(node);
        if !(f == 0 && e.mem.u32(node + 4) == 0 && node != list + 0x18) {
            out.push(if f == 0 { 0 } else { e.mem.u32(f + 0xc) & 0xff });
        }
        node = e.mem.u32(node + 4);
    }
    if out == [0] {
        out.clear();
    }
    out
}

const COUNT: u32 = 0x143f;
const NTH: u32 = 0x1440;
const INDEX: u32 = 0x1441;
const ADD: u32 = 0x1442;
const ADD_REF: u32 = 0x1443;
const REMOVE_NTH: u32 = 0x1444;
const REMOVE: u32 = 0x1445;
const REPLACE_NTH: u32 = 0x1446;
const REPLACE: u32 = 0x1447;
const CLEAR: u32 = 0x1448;
const IN_LIST: u32 = 0x1511;
const FOR_EACH: u32 = 0x1638;
const BAKED: u32 = 0x168c;
const LEV_BAKED: u32 = 0x168d;

/// Count, nth form (-2 the last; past the end, a negative index: 0) and index (-1 for none, the
/// form optional: no form answers -1 with no console line); a form that is no list is refused by
/// the read (its type is checked).
#[test]
fn count_nth_and_index() {
    let mut e = setup();
    assert_eq!(run(&mut e, COUNT, 0, &[r(1)]), 2.0);
    assert_eq!(
        run(&mut e, COUNT, 0, &[r(2)]),
        0.0,
        "a form that is no list"
    );
    assert_eq!(low(run(&mut e, NTH, 0, &[r(1), n(1)])), 0x0001_0012);
    assert_eq!(low(run(&mut e, NTH, 0, &[r(1), n(-2)])), 0x0001_0012);
    assert_eq!(run(&mut e, NTH, 0, &[r(1), n(2)]), 0.0);
    assert_eq!(run(&mut e, NTH, 0, &[r(1), n(-1)]), 0.0);
    assert_eq!(run(&mut e, INDEX, 0, &[r(1), r(3)]), 1.0);
    assert_eq!(run(&mut e, INDEX, 0, &[r(1), r(4)]), -1.0);
    assert_eq!(eval(&mut e, INDEX, 0, LIST, B), 1.0);
    console(&mut e);
    assert_eq!(run(&mut e, INDEX, 0, &[r(1)]), -1.0);
    assert!(seen().is_empty(), "no form: no line");
    run(&mut e, INDEX, 0, &[r(1), r(2)]);
    let lines = seen();
    assert_eq!(
        (lines[0].0.as_slice(), lines[0].1[0]),
        (&b"Index: %d"[..], 0)
    );
    // An empty node is counted towards the index (`10031920`).
    e.mem.set_u32(NODES, 0);
    e.mem.set_u32(NODES + 4, NODES + 8);
    e.mem.set_u32(NODES + 8, C);
    assert_eq!(run(&mut e, INDEX, 0, &[r(1), r(4)]), 2.0);
    assert_eq!(run(&mut e, COUNT, 0, &[r(1)]), 3.0);
}

/// `ListAddForm`: at the end by default (the index answered is the old last node's, the
/// `tList::AddAt` answer); at an index; with the duplicate check, a form already there adds nothing
/// and answers -1; no list or no form: -1 and nothing; an index among the forms scripts added goes
/// after them (`max` compared unsigned) and counts one more.
#[test]
fn adding_forms() {
    let mut e = setup();
    assert_eq!(
        run(&mut e, ADD, 0, &[r(1), r(4)]),
        1.0,
        "the old last index"
    );
    assert_eq!(ids(&e, LIST), [0x11, 0x12, 0x13]);
    assert_eq!(
        run(&mut e, ADD, 0, &[r(1), r(4), n(-2), n(1)]),
        -1.0,
        "a duplicate"
    );
    assert_eq!(
        run(&mut e, ADD, 0, &[r(1), r(4), n(-2), n(0)]),
        2.0,
        "duplicates allowed"
    );
    assert_eq!(ids(&e, LIST), [0x11, 0x12, 0x13, 0x13]);
    assert_eq!(run(&mut e, ADD, 0, &[r(1), r(3), n(0)]), 0.0);
    assert_eq!(ids(&e, LIST), [0x12, 0x11, 0x12, 0x13, 0x13]);
    assert_eq!(e.mem.u32(LIST + 0x20), 0, "0 is not below 0: no more");
    // With one added form, index 0 becomes 1 (after it).
    e.mem.set_u32(LIST + 0x20, 1);
    assert_eq!(run(&mut e, ADD, 0, &[r(1), r(2), n(0)]), 1.0);
    assert_eq!(ids(&e, LIST), [0x12, 0x11, 0x11, 0x12, 0x13, 0x13]);
    assert_eq!(e.mem.u32(LIST + 0x20), 1, "index 1 is not below 1");
    assert_eq!(
        run(&mut e, ADD, 0, &[r(1), r(2), n(99)]),
        -1.0,
        "no such node"
    );
    // The end (-2) stays -2; the old last index below the count of added forms counts one more.
    e.mem.set_u32(LIST + 0x20, 10);
    assert_eq!(run(&mut e, ADD, 0, &[r(1), r(4)]), 5.0);
    assert_eq!(e.mem.u32(LIST + 0x20), 11);
    assert_eq!(
        run(&mut e, ADD, 0, &[r(2), r(2)]),
        -1.0,
        "no list: the read fails"
    );
    console(&mut e);
    run(&mut e, ADD, 0, &[r(1), r(4), n(-2), n(1)]);
    let lines = seen();
    assert_eq!(
        (lines[0].0.as_slice(), lines[0].1[0]),
        (&b"Index: %d"[..], u32::MAX)
    );
}

/// `ListAddReference`: the calling reference added as `ListAddForm` adds; no calling reference:
/// -1, nothing added.
#[test]
fn adding_the_calling_reference() {
    let mut e = setup();
    assert_eq!(run(&mut e, ADD_REF, REF, &[r(1)]), 1.0);
    assert_eq!(ids(&e, LIST), [0x11, 0x12, 0x14]);
    assert_eq!(run(&mut e, ADD_REF, REF, &[r(1), n(-2), n(1)]), -1.0);
    assert_eq!(run(&mut e, ADD_REF, 0, &[r(1)]), -1.0);
    assert_eq!(ids(&e, LIST), [0x11, 0x12, 0x14]);
}

/// Removing and replacing: `ListRemoveNthForm` with no index takes the FIRST form (-2 is "0 or
/// less" to `RemoveNth`), the id in the low word over 0; one fewer added form for an added one;
/// `ListReplaceNthForm` with no index the last; by form, the index of the first match, -1 for
/// none.
#[test]
fn removing_and_replacing() {
    let mut e = setup();
    e.mem.set_u32(LIST + 0x20, 1);
    assert_eq!(
        run(&mut e, REMOVE_NTH, 0, &[r(1)]),
        f64::from_bits(0x0001_0011)
    );
    assert_eq!(ids(&e, LIST), [0x12]);
    assert_eq!(e.mem.u32(LIST + 0x20), 1, "-2 is no added form's index");
    run(&mut e, ADD, 0, &[r(1), r(4)]);
    assert_eq!(low(run(&mut e, REMOVE_NTH, 0, &[r(1), n(0)])), 0x0001_0012);
    assert_eq!(
        e.mem.u32(LIST + 0x20),
        1,
        "index 0 was an added form (the add at the end, old last 0, made it 2)"
    );
    assert_eq!(
        run(&mut e, REMOVE_NTH, 0, &[r(1), n(5)]),
        0.0,
        "past the end"
    );
    assert_eq!(ids(&e, LIST), [0x13]);
    run(&mut e, ADD, 0, &[r(1), r(2)]);
    assert_eq!(
        low(run(&mut e, REPLACE_NTH, 0, &[r(1), r(3)])),
        0x0001_0011,
        "the last"
    );
    assert_eq!(ids(&e, LIST), [0x13, 0x12]);
    assert_eq!(
        low(run(&mut e, REPLACE_NTH, 0, &[r(1), r(2), n(0)])),
        0x0001_0013
    );
    assert_eq!(run(&mut e, REPLACE_NTH, 0, &[r(1), r(2), n(-1)]), 0.0);
    assert_eq!(ids(&e, LIST), [0x11, 0x12]);
    assert_eq!(run(&mut e, REPLACE, 0, &[r(1), r(4), r(3)]), 1.0);
    assert_eq!(ids(&e, LIST), [0x11, 0x13]);
    assert_eq!(
        run(&mut e, REPLACE, 0, &[r(1), r(4), r(3)]),
        -1.0,
        "B is gone"
    );
    assert_eq!(run(&mut e, REMOVE, 0, &[r(1), r(4)]), 1.0);
    assert_eq!(ids(&e, LIST), [0x11]);
    assert_eq!(run(&mut e, REMOVE, 0, &[r(1), r(4)]), -1.0);
    assert_eq!(
        run(&mut e, REMOVE, 0, &[r(1)]),
        0.0,
        "no form: 0, nothing done"
    );
}

/// `ListClear` frees the nodes after the head and zeroes the count of added forms.
#[test]
fn clearing() {
    thread_local! {
        static FREED: std::cell::RefCell<Vec<u32>> = const { std::cell::RefCell::new(Vec::new()) };
    }
    let mut e = setup();
    e.register(0x0040_1030, |_, a| {
        FREED.with(|f| f.borrow_mut().push(a[0]));
        eax(0)
    });
    e.mem.set_u32(LIST + 0x20, 2);
    run(&mut e, CLEAR, 0, &[r(1)]);
    assert!(ids(&e, LIST).is_empty());
    assert_eq!(e.mem.u32(LIST + 0x20), 0);
    assert_eq!(
        FREED.with(|f| f.borrow_mut().drain(..).collect::<Vec<_>>()),
        [NODES]
    );
}

/// `IsRefInList`: the form, else its base form (here also its permanent base form); with no
/// form, the calling reference's base form; without a list, -1 and no console line.
#[test]
fn references_in_a_list() {
    let mut e = setup();
    assert_eq!(run(&mut e, IN_LIST, 0, &[r(1), r(3)]), 1.0);
    assert_eq!(
        run(&mut e, IN_LIST, 0, &[r(1), r(5)]),
        -1.0,
        "its base is in no list"
    );
    run(&mut e, ADD, 0, &[r(1), r(6)]);
    assert_eq!(run(&mut e, IN_LIST, 0, &[r(1), r(5)]), 2.0, "the base form");
    assert_eq!(
        run(&mut e, IN_LIST, REF, &[r(1)]),
        2.0,
        "the caller's base form"
    );
    assert_eq!(run(&mut e, IN_LIST, 0, &[r(1)]), -1.0);
    assert_eq!(eval(&mut e, IN_LIST, REF, LIST, 0), 2.0);
    assert_eq!(eval(&mut e, IN_LIST, REF, 0, 0), -1.0);
    console(&mut e);
    eval(&mut e, IN_LIST, REF, 0, 0);
    assert!(seen().is_empty(), "no list: no line");
    run(&mut e, IN_LIST, 0, &[r(1)]);
    let lines = seen();
    assert_eq!(
        (lines[0].0.as_slice(), lines[0].1[0]),
        (&b"Index: %d"[..], u32::MAX)
    );
}

/// A reference without a base form looked up in a list: xNVSE reads the missing form's id and
/// the game stops; the script reference checks row finds nothing there (-1).
#[test]
fn a_reference_without_a_base_form() {
    for checks in [false, true] {
        let mut e = setup();
        crate::invref::set_checks(&mut e, checks);
        e.mem.set_u32(REF + 0x20, 0);
        let found = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            run(&mut e, IN_LIST, REF, &[r(1)])
        }));
        match found {
            Ok(v) => assert!(checks && v == -1.0, "{checks} {v}"),
            Err(_) => assert!(!checks),
        }
    }
}

/// `ForEachInList`: the function runs once per node with its form (an empty node before the
/// last passes 0); 1 when it ran; a list or function of the wrong type: 0, nothing run.
#[test]
fn a_function_on_each_form() {
    let mut e = setup();
    e.map(0x011c_3000, 0x1000);
    e.register(SCRIPT_EXECUTE, |e, a| {
        let local = crate::args::event_list_variable(e, a[2], 1);
        let v = e.mem.u32(local + 8);
        RUNS.with(|r| r.borrow_mut().push(v));
        eax(0)
    });
    e.register(CREATE_EVENT_LIST, |e, a| {
        let list = alloc(e, 0x14);
        e.mem.set_u32(list, a[0]);
        let local = alloc(e, 0x10);
        e.mem.set_u32(local, 1);
        let node = alloc(e, 8);
        e.mem.set_u32(node, local);
        e.mem.set_u32(list + 0xc, node);
        eax(list)
    });
    e.register(EVENT_LIST_DESTROY, |_, _| eax(0));
    e.register(LOOKUP_FORM, |e, a| {
        eax([A, B, C, FUNCTION]
            .into_iter()
            .find(|&f| a[0] != 0 && e.mem.u32(f + 0xc) == a[0])
            .unwrap_or(0))
    });
    crate::functions::install(&mut e);
    // A function script taking one reference (local 1).
    let data = FUNCTION + 0x100;
    let mut d = vec![0x1d, 0, 0, 0, 0x10, 0];
    let block = vec![0x0d, 0, 0, 0, 0, 0, 1, 1, 1, 0, 4];
    d.extend_from_slice(&(block.len() as u16).to_le_bytes());
    d.extend(block);
    d.resize(0x40, 0);
    e.mem.write(data, &d);
    e.mem.set_u32(FUNCTION + 0xc, 0x0500_0a00);
    e.mem.set_u8(FUNCTION + 4, 0x11);
    e.mem.set_u32(FUNCTION + 0x30, data);
    e.mem.set_u32(FUNCTION + 0x20, d.len() as u32);
    set_script_ref(&mut e, 8, FUNCTION);
    e.mem.set_u32(NODES, 0);
    e.mem.set_u32(NODES + 4, NODES + 8);
    e.mem.set_u32(NODES + 8, B);
    assert_eq!(run(&mut e, FOR_EACH, 0, &[r(1), r(8)]), 1.0);
    let runs = RUNS.with(|r| r.borrow_mut().drain(..).collect::<Vec<_>>());
    assert_eq!(runs.len(), 3, "{runs:x?}");
    assert_eq!(run(&mut e, FOR_EACH, 0, &[r(1), r(2)]), 0.0, "no script");
    assert!(RUNS.with(|r| r.borrow().is_empty()));
}

/// The counts of forms scripts added: a form list's `numAddedObjects` (unsigned), a leveled
/// list's `scriptAddedObjects` (through the game's lookup `00487BE0`); the console prints the
/// result after any read.
#[test]
fn save_baked_counts() {
    let mut e = setup();
    e.mem.set_u32(LIST + 0x20, 0x8000_0000);
    assert_eq!(run(&mut e, BAKED, 0, &[r(1)]), 2_147_483_648.0);
    e.register(AS_LEVELED_LIST, |_, a| {
        eax(if a[0] == LEVELED { LEVELED + 0x30 } else { 0 })
    });
    set_script_ref(&mut e, 9, LEVELED);
    assert_eq!(run(&mut e, LEV_BAKED, 0, &[r(9)]), 0.0);
    e.mem.set_u32(LEVELED + 0x30 + 0x14, A);
    e.mem.set_u32(LEVELED + 0x30 + 0x18, NODES + 0x100);
    assert_eq!(run(&mut e, LEV_BAKED, 0, &[r(9)]), 2.0);
    assert_eq!(run(&mut e, LEV_BAKED, 0, &[r(2)]), 0.0, "no leveled list");
    console(&mut e);
    run(&mut e, BAKED, 0, &[r(2)]);
    run(&mut e, LEV_BAKED, 0, &[r(9)]);
    let lines = seen();
    assert_eq!(
        lines[0],
        (
            b"ListGetSaveBakedObjectCount >> %f".to_vec(),
            double_words(0.0).to_vec()
        )
    );
    assert_eq!(
        lines[1],
        (
            b"GetNumLevSaveBakedItems >> %f".to_vec(),
            double_words(2.0).to_vec()
        )
    );
}
