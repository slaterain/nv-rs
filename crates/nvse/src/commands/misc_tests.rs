//! Tests of the first `Commands_Misc.cpp` batch through the commands' records (and the condition
//! evals through theirs), on forms laid out where the test's cast puts their parts. The answers
//! pinned are xNVSE's as its source and the shipped DLL (`nvse_1_4.dll`, addresses in the tests'
//! notes) give them; none of these commands has been run in the real game yet.

use super::super::test_support::*;
use super::*;

const NPC_REF: u32 = 0x0200_0000;
const NPC: u32 = 0x0200_0800;
const RACE: u32 = 0x0200_1000;
const CLASS: u32 = 0x0200_1400;
const FACTION_A: u32 = 0x0200_1800;
const FACTION_B: u32 = 0x0200_1c00;
const PERK_FORM: u32 = 0x0200_2000;
const ITEM: u32 = 0x0200_2400;
const ITEM_BASE: u32 = 0x0200_2800;
const NODES: u32 = 0x0200_3000;
const TEXT: u32 = 0x0200_4000;
const VTBL: u32 = 0x0200_5000;
const LEVELED: u32 = 0x0200_6000;
const LIST: u32 = 0x0200_6800;
const CONSOLE: u32 = 0x0200_f000;

thread_local! {
    static SEEN: std::cell::RefCell<Vec<(Vec<u8>, Vec<u32>)>> = const { std::cell::RefCell::new(Vec::new()) };
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

/// A text at `TEXT + at`.
fn text(e: &mut Engine, at: u32, t: &str) -> u32 {
    e.mem.write(TEXT + at, t.as_bytes());
    e.mem.set_u8(TEXT + at + t.len() as u32, 0);
    TEXT + at
}

/// A `String` at `string` holding `t` (its text at `TEXT + at`, its length at `+4`).
fn string(e: &mut Engine, string: u32, at: u32, t: &str) {
    let data = text(e, at, t);
    e.mem.set_u32(string, data);
    e.mem.set_u16(string + 4, t.len() as u16);
    e.mem.set_u16(string + 6, t.len() as u16);
}

/// A list node at `NODES + 8 * i` holding `item`, followed by node `next` (none for 0).
fn node(e: &mut Engine, i: u32, item: u32, next: u32) -> u32 {
    let at = NODES + 8 * i;
    e.mem.set_u32(at, item);
    e.mem
        .set_u32(at + 4, if next == 0 { 0 } else { NODES + 8 * next });
    at
}

/// An NPC reference (an actor; base `NPC`, an NPC and actor base whose data is at `+0x30`) of
/// race `RACE` ("Hispanic Old") and class `CLASS`; an item reference; ASCII `toupper`; prints kept.
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
    e.register(0x0041_0220, |e, a| eax(e.mem.u32(a[0] + 0x100 + 4 * a[1])));
    for (form, kind, id) in [
        (NPC_REF, 0x3b, 0x10),
        (NPC, 0x2a, 0x11),
        (RACE, 0x0c, 0x12),
        (CLASS, 0x07, 0x13),
        (FACTION_A, 0x08, 0x14),
        (FACTION_B, 0x08, 0x15),
        (PERK_FORM, 0x56, 0x16),
        (ITEM, 0x3a, 0x17),
        (ITEM_BASE, 0x1f, 0x18),
        (LEVELED, 0x2c, 0x19),
        (LIST, 0x55, 0x1a),
    ] {
        e.mem.set_u8(form + 4, kind);
        e.mem.set_u32(form + 0xc, 0x0001_0000 + id);
    }
    set_classes(&mut e, NPC_REF, 1 | 1 << 21);
    set_classes(&mut e, ITEM, 1);
    set_classes(&mut e, NPC, 1 << 29 | 1 << 26 | 1 << 33);
    set_classes(&mut e, PERK_FORM, 1 << 36);
    set_classes(&mut e, LEVELED, 1 << 37);
    e.mem.set_u32(NPC_REF + 0x20, NPC);
    e.mem.set_u32(ITEM + 0x20, ITEM_BASE);
    e.mem.set_u32(NPC + 0x110, RACE);
    e.mem.set_u32(NPC + 0x130, CLASS);
    string(&mut e, RACE + 0x1c, 0, "Hispanic Old");
    string(&mut e, CLASS + 0x1c, 0x40, "Doctor");
    e.mem.set_u32(SCRIPT + 0xc, 0x0500_0d00);
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

fn ids(e: &mut Engine, array: f64) -> Vec<String> {
    let id = array as u32;
    let len = vars::array(e, id).map_or(0, |a| a.len());
    (0..len)
        .map(|i| match vars::get_element(e, id, &Key::Number(i as f64)) {
            Some(vars::Element { data, .. }) => match data {
                vars::Data::Form(f) => format!("{f:08x}"),
                vars::Data::Number(n) => format!("{n}"),
                vars::Data::Str(s) => String::from_utf8_lossy(&s).into_owned(),
                other => format!("{other:?}"),
            },
            None => "none".into(),
        })
        .collect()
}

/// The age class from the race: its child flag, then its name in capitals ("CHILD" 0, "OLDAGED"
/// 3, "OLD" 2, else 1); -1 for no NPC or no race. The argument comes first, then the calling
/// reference; the console prints `%.0f`.
#[test]
fn age_classes() {
    let mut e = setup();
    set_script_ref(&mut e, 1, NPC_REF);
    assert_eq!(run(&mut e, 0x14fe, NPC_REF, &[]), 2.0);
    assert_eq!(run(&mut e, 0x14fe, 0, &[r(1)]), 2.0);
    assert_eq!(eval(&mut e, 0x14fe, 0, NPC_REF, 0), 2.0);
    assert_eq!(run(&mut e, 0x14fe, ITEM, &[]), -1.0);
    assert_eq!(run(&mut e, 0x14fe, 0, &[]), -1.0);
    for (name, class) in [
        ("Old Aged", 2.0),
        ("OldAged", 3.0),
        ("child of atom", 0.0),
        ("Caucasian", 1.0),
        ("", 1.0),
    ] {
        let at = text(&mut e, 0x100, name);
        e.mem.set_u32(RACE + 0x1c, at);
        assert_eq!(run(&mut e, 0x14fe, NPC_REF, &[]), class, "{name}");
    }
    e.mem.set_u32(RACE + 0x1c, 0);
    assert_eq!(run(&mut e, 0x14fe, NPC_REF, &[]), 1.0, "no name");
    e.mem.set_u32(RACE + 0x70, 4);
    assert_eq!(run(&mut e, 0x14fe, NPC_REF, &[]), 0.0, "the child flag");
    e.mem.set_u32(NPC + 0x110, 0);
    assert_eq!(run(&mut e, 0x14fe, NPC_REF, &[]), -1.0, "no race");
    console(&mut e);
    run(&mut e, 0x14fe, NPC_REF, &[]);
    assert_eq!(
        seen(),
        [(b"GetAgeClass: %.0f".to_vec(), double_words(-1.0).to_vec())]
    );
}

/// Each byte goes to the game's `toupper` sign-extended (`movsx` at `10033123`), the terminator
/// too (the loop runs to `i <= strlen`), in order; the name is read through its text pointer
/// alone (no length test, unlike `CStr`).
#[test]
fn age_class_upper_cases_signed_bytes() {
    thread_local! {
        static ASKED: std::cell::RefCell<Vec<u32>> = const { std::cell::RefCell::new(Vec::new()) };
    }
    let mut e = setup();
    e.register(0x00ec_a7f4, |_, a| {
        ASKED.with(|p| p.borrow_mut().push(a[0]));
        eax(u32::from((a[0] as u8).to_ascii_uppercase()))
    });
    let at = text(&mut e, 0x100, "\u{e9}old");
    e.mem.set_u32(RACE + 0x1c, at);
    e.mem.set_u16(RACE + 0x20, 0);
    assert_eq!(run(&mut e, 0x14fe, NPC_REF, &[]), 2.0);
    let asked = ASKED.with(|p| p.borrow_mut().drain(..).collect::<Vec<_>>());
    // "é" is two bytes in UTF-8, both above 0x7F.
    assert_eq!(asked, [0xffff_ffc3, 0xffff_ffa9, 0x6f, 0x6c, 0x64, 0]);
}

/// A race name of 256 characters or more overflows xNVSE's 256-byte buffer (`upperName` at
/// `ebp-0x100`, the saved frame right above it): without the script list and scan guard row the
/// port stops the game; with it the name is read whole. 255 characters and the terminator fit.
#[test]
fn a_long_race_name() {
    let long = format!("{}OLD", "x".repeat(253));
    let mut e = setup();
    let at = text(&mut e, 0x100, &long);
    e.mem.set_u32(RACE + 0x1c, at);
    let stop = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        run(&mut e, 0x14fe, NPC_REF, &[])
    }));
    assert!(stop.is_err());
    let at = text(&mut e, 0x100, &long[1..]);
    e.mem.set_u32(RACE + 0x1c, at);
    assert_eq!(run(&mut e, 0x14fe, NPC_REF, &[]), 2.0, "255 characters fit");
    let mut e = setup();
    super::super::array::set_fix(&mut e, super::super::array::LIST_GUARD, true);
    let at = text(&mut e, 0x100, &long);
    e.mem.set_u32(RACE + 0x1c, at);
    assert_eq!(run(&mut e, 0x14fe, NPC_REF, &[]), 2.0);
}

/// Respawn: the actor base data's flag 8, set from any non-zero integer; permanent: `GetPermanent`
/// reads the base form's quest item flag, `SetPermanent` writes the reference's own and marks it
/// changed (`+0x48` with 1; the next entry, `+0x4C`, is not called).
#[test]
fn respawn_and_permanent_flags() {
    let mut e = setup();
    let data = NPC + 0x30;
    assert_eq!(run(&mut e, 0x150c, NPC_REF, &[]), 0.0);
    run(&mut e, 0x150d, NPC_REF, &[n(-1)]);
    assert_eq!(e.mem.u32(data + 4), 8);
    assert_eq!(eval(&mut e, 0x150c, NPC_REF, 0, 0), 1.0);
    set_script_ref(&mut e, 1, NPC_REF);
    run(&mut e, 0x150d, 0, &[n(0), r(1)]);
    assert_eq!(e.mem.u32(data + 4), 0);
    assert_eq!(run(&mut e, 0x150c, 0, &[r(1)]), 0.0);
    assert_eq!(run(&mut e, 0x150c, ITEM, &[]), 0.0, "no actor");
    // Permanent.
    e.mem.set_u32(ITEM, VTBL);
    e.mem.set_u32(VTBL + 0x48, 0x0300_9000);
    e.register(0x0300_9000, |e, a| {
        SEEN.with(|p| p.borrow_mut().push((Vec::new(), a[..2].to_vec())));
        let _ = e;
        eax(0)
    });
    e.mem.set_u32(VTBL + 0x4c, 0x0300_9100);
    e.register(0x0300_9100, |_, _| panic!("the entry after MarkAsModified"));
    assert_eq!(run(&mut e, 0x150e, ITEM, &[]), 0.0);
    run(&mut e, 0x150f, ITEM, &[n(1)]);
    assert_eq!(e.mem.u32(ITEM + 8), 0x400);
    assert_eq!(seen(), [(Vec::new(), vec![ITEM, 1])]);
    assert_eq!(
        run(&mut e, 0x150e, ITEM, &[]),
        0.0,
        "the base form's flag is read"
    );
    e.mem.set_u32(ITEM_BASE + 8, 0x400);
    assert_eq!(run(&mut e, 0x150e, ITEM, &[]), 1.0);
    assert_eq!(eval(&mut e, 0x150e, 0, ITEM, 0), 1.0);
    run(&mut e, 0x150f, ITEM, &[n(0)]);
    assert_eq!(e.mem.u32(ITEM + 8), 0);
    assert_eq!(
        run(&mut e, 0x150f, ITEM, &[s("x")]),
        0.0,
        "a failed read sets nothing"
    );
    assert!(seen().len() == 1);
}

/// Race and class (a form given, a reference's base form, or the calling reference's), and their
/// names as strings; a non-NPC answers 0 and makes no string. A name is `CStr`'s: "" without a
/// text pointer or with a length of 0 (the DLL tests both, `100337ED`).
#[test]
fn race_and_class() {
    let mut e = setup();
    assert_eq!(
        run(&mut e, 0x159c, NPC_REF, &[]).to_bits() as u32,
        0x0001_0012
    );
    assert_eq!(
        run(&mut e, 0x161c, NPC_REF, &[]).to_bits() as u32,
        0x0001_0013
    );
    set_script_ref(&mut e, 1, NPC);
    set_script_ref(&mut e, 2, NPC_REF);
    set_script_ref(&mut e, 3, ITEM);
    assert_eq!(
        run(&mut e, 0x159c, 0, &[r(1)]).to_bits() as u32,
        0x0001_0012
    );
    assert_eq!(
        run(&mut e, 0x159c, 0, &[r(2)]).to_bits() as u32,
        0x0001_0012
    );
    assert_eq!(
        run(&mut e, 0x159c, NPC_REF, &[r(3)]),
        0.0,
        "the argument wins"
    );
    let name = run(&mut e, 0x159d, NPC_REF, &[]);
    assert_eq!(vars::string(&e, name as u32).unwrap(), b"Hispanic Old");
    let name = run(&mut e, 0x161d, NPC_REF, &[]);
    assert_eq!(vars::string(&e, name as u32).unwrap(), b"Doctor");
    assert_eq!(run(&mut e, 0x159d, ITEM, &[]), 0.0);
    e.mem.set_u16(CLASS + 0x20, 0);
    let name = run(&mut e, 0x161d, NPC_REF, &[]);
    assert_eq!(vars::string(&e, name as u32).unwrap(), b"", "a length of 0");
    e.mem.set_u16(CLASS + 0x20, 6);
    e.mem.set_u32(CLASS + 0x1c, 0);
    let name = run(&mut e, 0x161d, NPC_REF, &[]);
    assert_eq!(vars::string(&e, name as u32).unwrap(), b"", "no text");
}

/// Perk ranks through the actor's `+0x4A0` entry (the perk and whether the alternate ranks), a
/// byte; -1 without a perk (a form that is no perk) or without an actor.
#[test]
fn perk_ranks() {
    let mut e = setup();
    e.mem.set_u32(NPC_REF, VTBL);
    e.mem.set_u32(VTBL + 0x4a0, 0x0300_9200);
    e.register(0x0300_9200, |_, a| eax(if a[2] != 0 { 0x1ff } else { 2 }));
    e.mem.set_u32(VTBL + 0x49c, 0x0300_9300);
    e.register(0x0300_9300, |_, _| panic!("the entry before GetPerkRank"));
    set_script_ref(&mut e, 1, PERK_FORM);
    set_script_ref(&mut e, 2, NPC_REF);
    set_script_ref(&mut e, 3, NPC);
    assert_eq!(run(&mut e, 0x15c3, NPC_REF, &[r(1)]), 2.0);
    assert_eq!(run(&mut e, 0x15c4, 0, &[r(1), r(2)]), 255.0);
    assert_eq!(eval(&mut e, 0x15c3, 0, PERK_FORM, NPC_REF), 2.0);
    assert_eq!(
        eval(&mut e, 0x15c3, NPC_REF, NPC, 0),
        -1.0,
        "an NPC base is no perk"
    );
    assert_eq!(eval(&mut e, 0x15c3, 0, PERK_FORM, 0), -1.0);
    assert_eq!(run(&mut e, 0x15c3, 0, &[r(1)]), -1.0);
    console(&mut e);
    run(&mut e, 0x15c4, NPC_REF, &[r(1)]);
    assert_eq!(seen()[0].0, b"GetAltPerkRank: %.0f");
}

/// The list commands: each makes an array (empty when there is nothing to list), in list order,
/// empty nodes passed over.
#[test]
fn lists_to_arrays() {
    let mut e = setup();
    // The race's hairs (an empty node between) and no eyes.
    let first = node(&mut e, 0, FACTION_A, 1);
    node(&mut e, 1, 0, 2);
    node(&mut e, 2, FACTION_B, 0);
    e.mem.set_u32(RACE + 0x8c, FACTION_A);
    e.mem.set_u32(RACE + 0x90, first + 8);
    set_script_ref(&mut e, 1, RACE);
    let a = run(&mut e, 0x15cd, 0, &[r(1)]);
    assert_eq!(ids(&mut e, a), ["00010014", "00010015"]);
    let a = run(&mut e, 0x15ce, 0, &[r(1)]);
    assert!(ids(&mut e, a).is_empty());
    let a = run(&mut e, 0x15cd, 0, &[r(2)]);
    assert!(
        ids(&mut e, a).is_empty(),
        "no argument read: an empty array"
    );
    // The actor base's factions and ranks (255 stays 255).
    let (fa, fb) = (NODES + 0x200, NODES + 0x210);
    e.mem.set_u32(fa, FACTION_A);
    e.mem.set_u8(fa + 4, 3);
    e.mem.set_u32(fb, FACTION_B);
    e.mem.set_u8(fb + 4, 255);
    e.mem.set_u32(NPC + 0x5c, fa);
    let more = node(&mut e, 3, fb, 0);
    e.mem.set_u32(NPC + 0x60, more);
    let a = run(&mut e, 0x15d2, NPC_REF, &[]);
    assert_eq!(ids(&mut e, a), ["00010014", "00010015"]);
    let a = run(&mut e, 0x15d3, NPC_REF, &[]);
    assert_eq!(ids(&mut e, a), ["3", "255"]);
    set_script_ref(&mut e, 2, NPC);
    let a = run(&mut e, 0x15d3, 0, &[r(2)]);
    assert_eq!(ids(&mut e, a), ["3", "255"]);
    // Active factions: a change of faction B (rank 0xFF, listed: the byte is unsigned).
    let changes = NODES + 0x300;
    let cb = NODES + 0x310;
    e.mem.set_u32(cb, FACTION_B);
    e.mem.set_u8(cb + 4, 0xff);
    let head = node(&mut e, 4, cb, 0);
    e.mem.set_u32(changes + 0xc, head);
    e.mem.set_u32(NPC_REF + 0x44 + 0x100 + 4 * 0x5e, changes);
    let a = run(&mut e, 0x15d4, NPC_REF, &[]);
    assert_eq!(ids(&mut e, a), ["00010014", "00010015"]);
    let a = run(&mut e, 0x15d5, NPC_REF, &[]);
    assert_eq!(ids(&mut e, a), ["3", "255"]);
    e.mem.set_u32(cb, FACTION_A);
    e.mem.set_u8(cb + 4, 7);
    let a = run(&mut e, 0x15d5, NPC_REF, &[]);
    assert_eq!(
        ids(&mut e, a),
        ["255", "7"],
        "the base's B, then the change of A"
    );
    let a = run(&mut e, 0x15d4, ITEM, &[]);
    assert!(ids(&mut e, a).is_empty(), "no actor");
    // The faction's rank names.
    let (r1, r2, r3) = (NODES + 0x400, NODES + 0x420, NODES + 0x440);
    string(&mut e, r1, 0x200, "Recruit");
    string(&mut e, r2, 0x220, "Sergeant");
    string(&mut e, r2 + 8, 0x240, "Sergeant Ma'am");
    string(&mut e, r3, 0x260, "Lieutenant");
    e.mem.set_u16(r3 + 4, 0);
    e.mem.set_u32(FACTION_A + 0x3c, r1);
    let next = node(&mut e, 5, r2, 7);
    node(&mut e, 7, r3, 0);
    e.mem.set_u32(FACTION_A + 0x40, next);
    set_script_ref(&mut e, 3, FACTION_A);
    let a = run(&mut e, 0x15d6, 0, &[r(3)]);
    assert_eq!(
        ids(&mut e, a),
        ["Recruit", "Sergeant", ""],
        "a length of 0: \"\""
    );
    let a = run(&mut e, 0x15d7, 0, &[r(3)]);
    assert_eq!(ids(&mut e, a), ["", "Sergeant Ma'am", ""]);
    let a = run(&mut e, 0x15d6, 0, &[r(1)]);
    assert!(
        ids(&mut e, a).is_empty(),
        "a race is no faction: the read fails"
    );
    // Spells, leveled spells, packages, head parts.
    for (at, op) in [
        (0x80, 0x15cf),
        (0x88, 0x15d0),
        (0xa8, 0x15d1),
        (0x1dc, 0x15d8),
    ] {
        e.mem.set_u32(NPC + at, CLASS);
        let a = run(&mut e, op, NPC_REF, &[]);
        assert_eq!(ids(&mut e, a), ["00010013"], "{op:x}");
        let a = run(&mut e, op, ITEM, &[]);
        assert!(ids(&mut e, a).is_empty(), "{op:x}");
    }
    // A leveled creature's entries; a form list's forms.
    let (l1, l2) = (NODES + 0x500, NODES + 0x510);
    e.mem.set_u32(l1, NPC);
    e.mem.set_u32(l2, CLASS);
    e.mem.set_u32(LEVELED + 0x34, l1);
    let next = node(&mut e, 6, l2, 0);
    e.mem.set_u32(LEVELED + 0x38, next);
    set_script_ref(&mut e, 4, LEVELED);
    let a = run(&mut e, 0x15d9, 0, &[r(4)]);
    assert_eq!(ids(&mut e, a), ["00010011", "00010013"]);
    let a = run(&mut e, 0x15da, 0, &[r(4)]);
    assert!(
        ids(&mut e, a).is_empty(),
        "a leveled creature is no leveled character"
    );
    set_classes(&mut e, LEVELED, 1 << 38);
    let a = run(&mut e, 0x15da, 0, &[r(4)]);
    assert_eq!(
        ids(&mut e, a),
        ["00010011", "00010013"],
        "as a leveled character"
    );
    let a = run(&mut e, 0x15d9, 0, &[r(4)]);
    assert!(ids(&mut e, a).is_empty(), "and no leveled creature");
    set_classes(&mut e, LEVELED, 1 << 37);
    e.mem.set_u32(LIST + 0x18, ITEM_BASE);
    set_script_ref(&mut e, 5, LIST);
    let a = run(&mut e, 0x15db, 0, &[r(5)]);
    assert_eq!(ids(&mut e, a), ["00010018"]);
    let a = run(&mut e, 0x15db, 0, &[r(4)]);
    assert!(ids(&mut e, a).is_empty(), "a leveled list is no form list");
}

/// A leveled list entry without a form: read through without the script reference checks row,
/// passed over with it.
#[test]
fn a_leveled_entry_without_a_form() {
    let mut e = setup();
    let entry = NODES + 0x500;
    e.mem.set_u32(LEVELED + 0x34, entry);
    set_script_ref(&mut e, 1, LEVELED);
    let stop = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        run(&mut e, 0x15d9, 0, &[r(1)])
    }));
    assert!(stop.is_err());
    let mut e = setup();
    crate::invref::set_checks(&mut e, true);
    e.mem.set_u32(LEVELED + 0x34, entry);
    set_script_ref(&mut e, 1, LEVELED);
    let a = run(&mut e, 0x15d9, 0, &[r(1)]);
    assert!(ids(&mut e, a).is_empty());
}
