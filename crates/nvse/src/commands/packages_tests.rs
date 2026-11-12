//! Tests of `Commands_Packages.cpp` through the commands' records, on actors, processes and
//! packages laid out as the game's (the package list a `tList` in the base form's `TESAIForm`).
//! The answers pinned are xNVSE's as its source gives them; none of these commands has been run in
//! the real game yet.

use super::super::rtti;
use super::super::test_support::*;
use crate::table::guest_text;
use engine_core::prelude::*;
use std::cell::RefCell;

/// An actor (a character) with a base form that has packages and a process; an actor without a
/// process; a reference that is not an actor; four packages and a dialogue package; the people
/// of the dialogue.
const ACTOR: u32 = 0x0200_0800;
const NPC: u32 = 0x0200_1000;
const PROCESS: u32 = 0x0200_1800;
const IDLE_ACTOR: u32 = 0x0200_2000;
const THING: u32 = 0x0200_2800;
const P: [u32; 4] = [0x0200_3000, 0x0200_3800, 0x0200_4000, 0x0200_4800];
const DIALOGUE: u32 = 0x0200_5000;
const SPEAKER: u32 = 0x0200_5800;
const SUBJECT: u32 = 0x0200_6000;
const TARGET: u32 = 0x0200_6800;
/// The package list's head: the base form's `TESAIForm` (at `+0x640` in the fake cast) `+0x18`.
const LIST: u32 = NPC + 0x640 + 0x18;
/// Where the fake form heap hands out memory, and the list nodes the test makes.
const HEAP: u32 = 0x0200_9000;
const NODES: u32 = 0x0200_8000;
/// The process's `GetCurrentPackage` and the packages' `GetName`.
const CURRENT_PACKAGE: u32 = 0x0060_2000;
const GET_NAME: u32 = 0x0060_1000;

thread_local! {
    static LINES: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
    static FREED: RefCell<Vec<u32>> = const { RefCell::new(Vec::new()) };
}

fn eax(v: u32) -> Ret {
    Ret {
        eax: v,
        ..Ret::default()
    }
}

fn lines() -> Vec<String> {
    LINES.with(|l| std::mem::take(&mut *l.borrow_mut()))
}

fn freed() -> Vec<u32> {
    FREED.with(|f| std::mem::take(&mut *f.borrow_mut()))
}

fn console_on(e: &mut Engine) {
    e.mem.set_u8(0x011d_ea2e, 1);
    let tls = e.tls();
    e.mem.set_u8(tls + 0x268, 1);
}

/// A form at `at` of `kind`, its id its address, answering the casts of `classes`.
fn form(e: &mut Engine, at: u32, kind: u8, classes: u64) {
    e.mem.set_u8(at + 4, kind);
    e.mem.set_u32(at + 0xc, at);
    set_classes(e, at, classes);
}

fn bit(class: u32) -> u64 {
    component(class).0
}

fn setup() -> Engine {
    LINES.with(|l| l.borrow_mut().clear());
    FREED.with(|f| f.borrow_mut().clear());
    let mut e = engine_with_table();
    e.register(0x0071_b160, |_, _| eax(0x0200_f000));
    e.register(0x0071_d0a0, |e, a| {
        let format = guest_text(e, a[1]);
        let list = e.mem.bytes(a[2], 16);
        let line = super::super::printf::ucrt_format(e, &format, &list);
        LINES.with(|l| l.borrow_mut().push(String::from_utf8(line).unwrap()));
        eax(0)
    });
    e.register(0x0040_1000, |e, a| {
        let at = e.mem.u32(HEAP).max(HEAP + 0x10);
        e.mem.set_u32(HEAP, at + a[0].next_multiple_of(16));
        // Fresh memory as the heap hands it out: whatever was there.
        for i in 0..a[0] {
            e.mem.set_u8(at + i, 0xcc);
        }
        eax(at)
    });
    e.register(0x0040_1030, |_, a| {
        FREED.with(|f| f.borrow_mut().push(a[0]));
        eax(0)
    });
    e.register(CURRENT_PACKAGE, |e, a| eax(e.mem.u32(a[0] + 0x100)));
    e.register(GET_NAME, |_, a| eax(a[0] + 0x200));
    let actor = bit(rtti::ACTOR) | bit(rtti::TES_OBJECT_REFR);
    form(&mut e, ACTOR, 0x3b, actor);
    e.mem.set_u32(ACTOR + 0x20, NPC);
    e.mem.set_u32(ACTOR + 0x68, PROCESS);
    form(&mut e, NPC, 0x2a, bit(rtti::TES_AI_FORM));
    e.mem.set_u32(PROCESS, PROCESS + 0x400);
    e.mem.set_u32(PROCESS + 0x400 + 0x27c, CURRENT_PACKAGE);
    form(&mut e, IDLE_ACTOR, 0x3b, actor);
    e.mem.set_u32(IDLE_ACTOR + 0x20, NPC);
    form(&mut e, THING, 0x3a, bit(rtti::TES_OBJECT_REFR));
    for (i, &p) in P.iter().enumerate() {
        form(&mut e, p, 0x49, bit(rtti::TES_PACKAGE));
        e.mem.set_u32(p, p + 0x400);
        e.mem.set_u32(p + 0x400 + 0x130, GET_NAME);
        e.mem.write(p + 0x200, format!("Pack{i}\0").as_bytes());
    }
    form(
        &mut e,
        DIALOGUE,
        0x49,
        bit(rtti::TES_PACKAGE) | bit(rtti::DIALOGUE_PACKAGE),
    );
    for who in [SPEAKER, SUBJECT, TARGET] {
        form(&mut e, who, 0x3b, actor);
    }
    e.ext::<crate::Nvse>().main_thread_recorded = true;
    e
}

/// The package list made of `items` (head node embedded, then nodes at `NODES`).
fn set_list(e: &mut Engine, items: &[u32]) {
    e.mem.set_u32(LIST, items.first().copied().unwrap_or(0));
    e.mem.set_u32(LIST + 4, 0);
    let mut prev = LIST;
    for (i, &item) in items.iter().enumerate().skip(1) {
        let node = NODES + 8 * i as u32;
        e.mem.set_u32(node, item);
        e.mem.set_u32(node + 4, 0);
        e.mem.set_u32(prev + 4, node);
        prev = node;
    }
}

/// The package list's items in order.
fn list(e: &Engine) -> Vec<u32> {
    let mut out = Vec::new();
    let mut node = LIST;
    while node != 0 {
        let item = e.mem.u32(node);
        if item != 0 {
            out.push(item);
        }
        node = e.mem.u32(node + 4);
    }
    out
}

/// A result double's low word (a form id).
fn id(result: f64) -> u32 {
    result.to_bits() as u32
}

/// A condition's eval through the record's eval word, on `this_obj`; the result double.
fn eval(e: &mut Engine, opcode: u32, this_obj: u32) -> f64 {
    let record = crate::table::record_of(e, opcode);
    let eval = e.mem.u32(record + 0x20);
    assert_ne!(eval, 0, "{opcode:04x} has an eval");
    e.mem.set_f64(RESULT, 1.0);
    e.call(eval, &[this_obj, 0, 0, RESULT]);
    e.mem.f64(RESULT)
}

// ---- dialogue ----

/// The dialogue commands: the target (`+0xB4`), subject (`+0xB0`) and speaker (`+0x94`) of the
/// calling actor's current package when it is a dialogue package, as a form id; 0 for another
/// package, an actor without a process, an empty field, or no calling reference (the execute
/// does not read its argument). The console prints the id in `%10X`.
#[test]
fn dialogue_commands_read_the_dialogue_package() {
    let mut e = setup();
    e.mem.set_u32(DIALOGUE + 0xb4, TARGET);
    e.mem.set_u32(DIALOGUE + 0xb0, SUBJECT);
    e.mem.set_u32(DIALOGUE + 0x94, SPEAKER);
    e.mem.set_u32(PROCESS + 0x100, DIALOGUE);
    for (opcode, who) in [(0x14fa, TARGET), (0x14fb, SUBJECT), (0x14fc, SPEAKER)] {
        assert_eq!(id(call(&mut e, opcode, ACTOR, &[]).1), who);
        assert_eq!(id(eval(&mut e, opcode, ACTOR)), who);
        assert_eq!(call(&mut e, opcode, 0, &[]), (1, 0.0));
        assert_eq!(eval(&mut e, opcode, IDLE_ACTOR), 0.0);
        assert_eq!(eval(&mut e, opcode, THING), 0.0);
    }
    e.mem.set_u32(DIALOGUE + 0xb0, 0);
    assert_eq!(eval(&mut e, 0x14fb, ACTOR), 0.0);
    e.mem.set_u32(PROCESS + 0x100, P[0]);
    assert_eq!(eval(&mut e, 0x14fa, ACTOR), 0.0);
    e.mem.set_u32(PROCESS + 0x100, 0);
    assert_eq!(eval(&mut e, 0x14fa, ACTOR), 0.0);
    e.mem.set_u32(PROCESS + 0x100, DIALOGUE);
    console_on(&mut e);
    call(&mut e, 0x14fa, ACTOR, &[]);
    eval(&mut e, 0x14fc, ACTOR);
    eval(&mut e, 0x14fb, THING);
    assert_eq!(
        lines(),
        [
            "GetDialogueTarget >>    2006800",
            "GetDialogueSpeaker >>    2005800",
            "GetDialogueSubject >>          0",
        ]
    );
}

/// `GetCurrentPackage`: the package the actor (the argument, else the caller) runs; nothing for a
/// reference that is not an actor, an actor without a process, or neither; the console prints the
/// result's low word, also when there is no package.
#[test]
fn current_package_of_the_actor() {
    let mut e = setup();
    e.mem.set_u32(PROCESS + 0x100, P[1]);
    set_script_ref(&mut e, 1, ACTOR);
    set_script_ref(&mut e, 2, THING);
    set_script_ref(&mut e, 3, IDLE_ACTOR);
    assert_eq!(id(call(&mut e, 0x1513, 0, &[r(1)]).1), P[1]);
    assert_eq!(id(call(&mut e, 0x1513, ACTOR, &[]).1), P[1]);
    assert_eq!(call(&mut e, 0x1513, ACTOR, &[r(2)]).1, 0.0);
    assert_eq!(call(&mut e, 0x1513, ACTOR, &[r(3)]).1, 0.0);
    assert_eq!(call(&mut e, 0x1513, 0, &[]).1, 0.0);
    console_on(&mut e);
    call(&mut e, 0x1513, ACTOR, &[]);
    // An actor without a process: no line either.
    call(&mut e, 0x1513, IDLE_ACTOR, &[]);
    e.mem.set_u32(PROCESS + 0x100, 0);
    call(&mut e, 0x1513, ACTOR, &[]);
    call(&mut e, 0x1513, THING, &[]);
    assert_eq!(
        lines(),
        [
            "GetCurrentPackage >> [02003800] ",
            "GetCurrentPackage >> [00000000] "
        ]
    );
}

// ---- location and target ----

/// `SetPackageLocationReference` makes the location when the package has none (12 bytes: the
/// current location, no object, radius 0) and sets it near the reference (the second argument,
/// else the caller); nothing without a reference or for a form that is not a package.
/// `GetPackageLocation`: the object's id for near a reference, in a cell and an object, the type
/// code for an object type, 0 for another kind or no object; no location is made by reading.
#[test]
fn package_locations() {
    let mut e = setup();
    set_script_ref(&mut e, 1, P[0]);
    set_script_ref(&mut e, 2, THING);
    set_script_ref(&mut e, 3, P[1]);
    set_script_ref(&mut e, 4, ACTOR);
    assert_eq!(call(&mut e, 0x1514, 0, &[r(1)]).1, 0.0);
    assert_eq!(e.mem.u32(P[0] + 0x2c), 0, "a read makes no location");
    // Neither a reference nor a caller: nothing, even with a package.
    call(&mut e, 0x14fd, 0, &[r(1)]);
    assert_eq!(e.mem.u32(P[0] + 0x2c), 0);
    call(&mut e, 0x14fd, 0, &[r(1), r(2)]);
    let location = e.mem.u32(P[0] + 0x2c);
    assert_ne!(location, 0);
    assert_eq!(e.mem.u8(location), 0);
    assert_eq!(e.mem.u32(location + 4), 0, "radius");
    assert_eq!(e.mem.u32(location + 8), THING);
    assert_eq!(id(call(&mut e, 0x1514, 0, &[r(1)]).1), THING);
    // The caller when there is no second argument.
    call(&mut e, 0x14fd, ACTOR, &[r(3)]);
    assert_eq!(e.mem.u32(e.mem.u32(P[1] + 0x2c) + 8), ACTOR);
    // A form that is not a package.
    call(&mut e, 0x14fd, 0, &[r(4), r(2)]);
    assert_eq!(e.mem.u32(ACTOR + 0x2c), 0);
    for (kind, object, answer) in [
        (1, THING, THING),
        (4, THING, THING),
        (5, 7, 7),
        (2, THING, 0),
        (3, THING, 0),
        (6, THING, 0),
        (0, 0, 0),
    ] {
        e.mem.set_u8(location, kind);
        e.mem.set_u32(location + 8, object);
        assert_eq!(
            id(call(&mut e, 0x1514, 0, &[r(1)]).1),
            answer,
            "kind {kind}"
        );
    }
}

/// The radius: set only when the package has a location (none is made), truncated to a whole
/// number as MSVC's conversion does; read back unsigned, 0 without a location.
#[test]
fn location_radius() {
    let mut e = setup();
    set_script_ref(&mut e, 1, P[0]);
    set_script_ref(&mut e, 2, THING);
    call(&mut e, 0x151f, 0, &[r(1), z(250.0)]);
    assert_eq!(e.mem.u32(P[0] + 0x2c), 0, "no location made");
    assert_eq!(call(&mut e, 0x1520, 0, &[r(1)]).1, 0.0);
    call(&mut e, 0x14fd, 0, &[r(1), r(2)]);
    call(&mut e, 0x151f, 0, &[r(1), z(250.75)]);
    assert_eq!(call(&mut e, 0x1520, 0, &[r(1)]).1, 250.0);
    call(&mut e, 0x151f, 0, &[r(1), z(3e9)]);
    assert_eq!(call(&mut e, 0x1520, 0, &[r(1)]).1, 3e9);
}

/// `SetPackageTargetReference` makes the target when the package has none (16 bytes: count 0,
/// the activators' type code), then a reference target at the distance 150, the second argument
/// else the caller; the count's commands only work on a target that is there (none is made).
#[test]
fn package_targets() {
    let mut e = setup();
    set_script_ref(&mut e, 1, P[0]);
    set_script_ref(&mut e, 2, THING);
    call(&mut e, 0x151d, 0, &[r(1), n(9)]);
    assert_eq!(e.mem.u32(P[0] + 0x30), 0, "no target made");
    assert_eq!(call(&mut e, 0x151e, 0, &[r(1)]).1, 0.0);
    call(&mut e, 0x151c, 0, &[r(1)]);
    assert_eq!(e.mem.u32(P[0] + 0x30), 0, "no reference, no caller");
    call(&mut e, 0x151c, 0, &[r(1), r(2)]);
    let target = e.mem.u32(P[0] + 0x30);
    assert_ne!(target, 0);
    assert_eq!(
        (
            e.mem.u8(target),
            e.mem.u32(target + 4),
            e.mem.u32(target + 8)
        ),
        (0, THING, 150)
    );
    assert_eq!(call(&mut e, 0x151e, 0, &[r(1)]).1, 150.0);
    call(&mut e, 0x151d, 0, &[r(1), n(9)]);
    assert_eq!(call(&mut e, 0x151e, 0, &[r(1)]).1, 9.0);
    call(&mut e, 0x151c, ACTOR, &[r(1)]);
    assert_eq!(e.mem.u32(target + 4), ACTOR);
    assert_eq!(e.mem.u32(target + 8), 150);
    assert_eq!(e.mem.u32(P[0] + 0x30), target, "the target is kept");
}

/// A location made by `SetPackageLocationReference` writes only the fields `LocationData::Create`
/// writes (the kind byte, the radius, the object): the padding after the kind keeps what the heap
/// handed out.
#[test]
fn a_made_location_starts_at_the_current_location() {
    let mut e = setup();
    set_script_ref(&mut e, 1, P[2]);
    set_script_ref(&mut e, 2, THING);
    call(&mut e, 0x14fd, 0, &[r(1), r(2)]);
    let location = e.mem.u32(P[2] + 0x2c);
    // The kind byte was set to near a reference; the bytes after it are the heap's.
    assert_eq!(e.mem.u8(location + 1), 0xcc);
    assert_eq!(e.mem.u32(location + 4), 0);
}

// ---- an actor base's packages ----

/// `GetPackageCount` (execute: the argument else the caller; eval: the caller): the number of
/// packages, 0 for a reference that is not an actor; the console prints it as `%u`.
#[test]
fn package_count() {
    let mut e = setup();
    set_list(&mut e, &[P[0], P[1], P[2]]);
    set_script_ref(&mut e, 1, ACTOR);
    set_script_ref(&mut e, 2, THING);
    assert_eq!(call(&mut e, 0x1515, 0, &[r(1)]).1, 3.0);
    assert_eq!(call(&mut e, 0x1515, ACTOR, &[]).1, 3.0);
    assert_eq!(call(&mut e, 0x1515, ACTOR, &[r(2)]).1, 0.0);
    assert_eq!(call(&mut e, 0x1515, 0, &[]).1, 0.0);
    assert_eq!(eval(&mut e, 0x1515, ACTOR), 3.0);
    assert_eq!(eval(&mut e, 0x1515, THING), 0.0);
    set_list(&mut e, &[]);
    assert_eq!(eval(&mut e, 0x1515, ACTOR), 0.0);
    set_list(&mut e, &[P[3]]);
    console_on(&mut e);
    call(&mut e, 0x1515, ACTOR, &[]);
    eval(&mut e, 0x1515, THING);
    assert_eq!(lines(), ["GetPackageCount >> 1", "GetPackageCount >> 0"]);
}

/// `GetNthPackage`: by index, -2 the last, -1 and past the end none; the console names it.
#[test]
fn nth_package() {
    let mut e = setup();
    set_list(&mut e, &[P[0], P[1], P[2]]);
    let nth = |e: &mut Engine, i: i32| id(call(e, 0x1516, ACTOR, &[n(i)]).1);
    assert_eq!(
        [
            nth(&mut e, 0),
            nth(&mut e, 1),
            nth(&mut e, 2),
            nth(&mut e, -2)
        ],
        [P[0], P[1], P[2], P[2]]
    );
    assert_eq!([nth(&mut e, -1), nth(&mut e, 3)], [0, 0]);
    set_script_ref(&mut e, 1, THING);
    assert_eq!(call(&mut e, 0x1516, ACTOR, &[n(0), r(1)]).1, 0.0);
    console_on(&mut e);
    nth(&mut e, 1);
    nth(&mut e, 5);
    assert_eq!(lines(), ["GetNthPackage >> 02003800 (Pack1)"]);
}

/// `SetNthPackage`: the package at the index replaced, the old one answered; -1 (as -2) the last;
/// past the end nothing; a form that is not a package nothing.
#[test]
fn set_nth_package() {
    let mut e = setup();
    set_list(&mut e, &[P[0], P[1], P[2]]);
    set_script_ref(&mut e, 1, P[3]);
    set_script_ref(&mut e, 2, THING);
    assert_eq!(id(call(&mut e, 0x1517, ACTOR, &[r(1), n(1)]).1), P[1]);
    assert_eq!(list(&e), [P[0], P[3], P[2]]);
    assert_eq!(id(call(&mut e, 0x1517, ACTOR, &[r(1), n(-1)]).1), P[2]);
    assert_eq!(list(&e), [P[0], P[3], P[3]]);
    assert_eq!(call(&mut e, 0x1517, ACTOR, &[r(1), n(3)]).1, 0.0);
    assert_eq!(call(&mut e, 0x1517, ACTOR, &[r(2), n(0)]).1, 0.0);
    assert_eq!(list(&e), [P[0], P[3], P[3]]);
}

/// `AddPackageAt`: 0 the head (answers 0), -1 the end (answers the index of the package it went
/// after), an index inside before the package there (answers it); the index one past the last adds
/// nothing (-1).
#[test]
fn add_package_at() {
    let mut e = setup();
    set_list(&mut e, &[P[0], P[1]]);
    set_script_ref(&mut e, 1, P[2]);
    set_script_ref(&mut e, 2, P[3]);
    set_script_ref(&mut e, 3, THING);
    assert_eq!(call(&mut e, 0x1518, ACTOR, &[r(1), n(-1)]).1, 1.0);
    assert_eq!(list(&e), [P[0], P[1], P[2]]);
    assert_eq!(call(&mut e, 0x1518, ACTOR, &[r(2), n(0)]).1, 0.0);
    assert_eq!(list(&e), [P[3], P[0], P[1], P[2]]);
    assert_eq!(call(&mut e, 0x1518, ACTOR, &[r(2), n(2)]).1, 2.0);
    assert_eq!(list(&e), [P[3], P[0], P[3], P[1], P[2]]);
    assert_eq!(call(&mut e, 0x1518, ACTOR, &[r(2), n(5)]).1, -1.0);
    assert_eq!(list(&e).len(), 5);
    // Not a package: nothing, and the answer stays 0.
    assert_eq!(call(&mut e, 0x1518, ACTOR, &[r(3), n(0)]).1, 0.0);
    assert_eq!(list(&e).len(), 5);
}

/// `RemovePackageAt`: -1 takes out the FIRST package (`RemoveNth` takes any index of 0 or less as
/// the head), 1 the second; past the end 0. `RemoveAllPackages` answers the count and frees every
/// node but the head.
#[test]
fn remove_packages() {
    let mut e = setup();
    set_list(&mut e, &[P[0], P[1], P[2], P[3]]);
    assert_eq!(id(call(&mut e, 0x1519, ACTOR, &[n(-1)]).1), P[0]);
    assert_eq!(list(&e), [P[1], P[2], P[3]]);
    assert_eq!(id(call(&mut e, 0x1519, ACTOR, &[n(1)]).1), P[2]);
    assert_eq!(list(&e), [P[1], P[3]]);
    assert_eq!(call(&mut e, 0x1519, ACTOR, &[n(2)]).1, 0.0);
    freed();
    set_list(&mut e, &[P[0], P[1], P[2]]);
    assert_eq!(call(&mut e, 0x151a, ACTOR, &[]).1, 3.0);
    assert!(list(&e).is_empty());
    assert_eq!(freed(), [NODES + 8, NODES + 16]);
    assert_eq!(call(&mut e, 0x151a, ACTOR, &[]).1, 0.0);
    assert_eq!(call(&mut e, 0x151a, THING, &[]).1, 0.0);
}
