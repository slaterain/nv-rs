//! Tests of the second `Commands_Misc.cpp` batch through the commands' records, on lists laid out
//! in the test's memory and a bump heap standing in for the game's. The answers pinned are
//! xNVSE's as its source and the shipped DLL (`nvse_1_4.dll`, addresses in the notes) give them;
//! none of these commands has been run in the real game yet.

use super::super::test_support::*;
use super::*;

const NPC_REF: u32 = 0x0200_0000;
const NPC: u32 = 0x0200_0800;
const RACE: u32 = 0x0200_1000;
const HAIR_A: u32 = 0x0200_1400;
const HAIR_B: u32 = 0x0200_1800;
const EYES: u32 = 0x0200_1c00;
const FORM_LIST: u32 = 0x0200_2000;
const DOOR: u32 = 0x0200_2400;
const SOUND: u32 = 0x0200_2800;
const MANAGER: u32 = 0x0200_3000;
const NODES: u32 = 0x0200_4000;
const TEXT: u32 = 0x0200_5000;
const VTBL: u32 = 0x0200_6000;
const TARGETS: u32 = 0x0200_7000;
const HEAP: u32 = 0x0200_8000;
const CONSOLE: u32 = 0x0200_f000;
const LOOKUP_FORM: u32 = 0x0048_39c0;

thread_local! {
    static SEEN: std::cell::RefCell<Vec<(Vec<u8>, Vec<u32>)>> = const { std::cell::RefCell::new(Vec::new()) };
    static FREED: std::cell::RefCell<Vec<u32>> = const { std::cell::RefCell::new(Vec::new()) };
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

fn freed() -> Vec<u32> {
    FREED.with(|p| p.borrow_mut().drain(..).collect())
}

/// An NPC reference (its base `NPC`, an NPC and actor base); a race with two hairs; eyes; a form
/// list; a door with an open sound; the default objects; a bump heap whose frees are kept.
fn setup() -> Engine {
    let mut e = engine_with_table();
    e.map(0x011c_3000, 0x1000);
    e.map(0x011c_a000, 0x1000);
    e.register(0x00ec_a7f4, |_, a| eax(u32::from((a[0] as u8).to_ascii_uppercase())));
    e.register(0x0071_b160, |_, _| eax(CONSOLE));
    e.register(0x0071_d0a0, |e, a| {
        let t = crate::table::guest_text(e, a[1]);
        let words = (0..2).map(|i| e.mem.u32(a[2] + 4 * i)).collect();
        SEEN.with(|p| p.borrow_mut().push((t, words)));
        eax(0)
    });
    e.register(FORM_HEAP_ALLOCATE, |e, a| {
        let at = e.mem.u32(HEAP).max(HEAP + 0x10);
        e.mem.set_u32(HEAP, at + a[0].next_multiple_of(0x10));
        eax(at)
    });
    e.register(FORM_HEAP_FREE, |_, a| {
        FREED.with(|p| p.borrow_mut().push(a[0]));
        eax(0)
    });
    for (form, kind, id) in [
        (NPC_REF, 0x3b, 0x10),
        (NPC, 0x2a, 0x11),
        (RACE, 0x0c, 0x12),
        (HAIR_A, 0x0b, 0x13),
        (HAIR_B, 0x0b, 0x14),
        (EYES, 0x0a, 0x15),
        (FORM_LIST, 0x55, 0x16),
        (DOOR, 0x1c, 0x17),
        (SOUND, 0x0d, 0x18),
    ] {
        e.mem.set_u8(form + 4, kind);
        e.mem.set_u32(form + 0xc, 0x0001_0000 + id);
    }
    set_classes(&mut e, NPC_REF, 1 | 1 << 21);
    set_classes(&mut e, NPC, 1 << 29 | 1 << 26);
    set_classes(&mut e, RACE, 1 << 18);
    set_classes(&mut e, HAIR_A, 1 << 31);
    set_classes(&mut e, HAIR_B, 1 << 31);
    set_classes(&mut e, EYES, 1 << 30);
    set_classes(&mut e, FORM_LIST, 1 << 14);
    set_classes(&mut e, DOOR, 1 << 39);
    e.mem.set_u32(NPC_REF + 0x20, NPC);
    // The race's hairs: A then B.
    e.mem.set_u32(RACE + 0x8c, HAIR_A);
    e.mem.set_u32(RACE + 0x90, NODES);
    e.mem.set_u32(NODES, HAIR_B);
    e.set_global::<u32>(DEFAULT_OBJECTS, MANAGER);
    e.mem.set_u32(SCRIPT + 0xc, 0x0500_0d00);
    // References answer `GetIsReference` (+0xF0) with 1, `+0xF4` stops.
    for form in [NPC_REF] {
        e.mem.set_u32(form, VTBL);
    }
    e.mem.set_u32(VTBL + 0xf0, 0x0300_9000);
    e.register(0x0300_9000, |_, _| eax(1));
    e.mem.set_u32(VTBL + 0xf4, 0x0300_9100);
    e.register(0x0300_9100, |_, _| panic!("the entry after GetIsReference"));
    // Base forms answer 0 there.
    e.mem.set_u32(NPC, VTBL + 0x400);
    e.mem.set_u32(VTBL + 0x400 + 0xf0, 0x0300_9200);
    e.register(0x0300_9200, |_, _| eax(0));
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

/// The list at `head` as its items' ids (0 for an empty node), up to its end.
fn list_ids(e: &Engine, head: u32) -> Vec<u32> {
    tlist_nodes(e, head)
        .into_iter()
        .map(|f| if f == 0 { 0 } else { e.mem.u32(f + 0xc) & 0xff })
        .collect()
}

fn low(v: f64) -> u32 {
    v.to_bits() as u32
}

const ADD: u32 = 0x15dc;
const REPLACE: u32 = 0x15dd;
const DELETE: u32 = 0x15de;
const GET: u32 = 0x15e1;
const CHECK: u32 = 0x15ff;

/// The race's hairs (kind 0): add at the head, the end (the index answered is the old last's),
/// in the middle; get, replace, check and delete by index; a form of another kind is refused
/// (the result stays -1); an unknown kind makes the handler answer false.
#[test]
fn generic_lists_of_a_race() {
    let mut e = setup();
    for (i, f) in [RACE, HAIR_A, HAIR_B, EYES, NPC, FORM_LIST].into_iter().enumerate() {
        set_script_ref(&mut e, i as u16 + 1, f);
    }
    let race = RACE + 0x8c;
    assert_eq!(list_ids(&e, race), [0x13, 0x14]);
    assert_eq!(run(&mut e, ADD, 0, &[n(0), r(1), r(3), n(-2)]), 1.0, "the old last index");
    assert_eq!(list_ids(&e, race), [0x13, 0x14, 0x14]);
    assert_eq!(run(&mut e, ADD, 0, &[n(0), r(1), r(2), n(0)]), 0.0);
    assert_eq!(list_ids(&e, race), [0x13, 0x13, 0x14, 0x14]);
    assert_eq!(run(&mut e, ADD, 0, &[n(0), r(1), r(3), n(9)]), -1.0, "no such node");
    assert_eq!(run(&mut e, ADD, 0, &[n(0), r(1), r(4), n(0)]), -1.0, "eyes are no hair");
    assert_eq!(list_ids(&e, race), [0x13, 0x13, 0x14, 0x14]);
    assert_eq!(low(run(&mut e, GET, 0, &[n(0), r(1), n(2)])), 0x0001_0014);
    assert_eq!(low(run(&mut e, GET, 0, &[n(0), r(1), n(-2)])), 0x0001_0014, "-2 is the last");
    let v = run(&mut e, GET, 0, &[n(0), r(1), n(9)]);
    assert_eq!(v, -1.0);
    // A form answered goes into the low word over -1's bits.
    let v = run(&mut e, REPLACE, 0, &[n(0), r(1), r(3), n(0)]);
    assert_eq!(v.to_bits(), (-1.0f64).to_bits() & !0xffff_ffff | 0x0001_0013);
    assert_eq!(list_ids(&e, race), [0x14, 0x13, 0x14, 0x14]);
    assert_eq!(run(&mut e, CHECK, 0, &[n(0), r(1), r(2)]), 1.0);
    assert_eq!(run(&mut e, CHECK, 0, &[n(0), r(1), r(4)]), -1.0, "eyes: no list");
    freed();
    assert_eq!(low(run(&mut e, DELETE, 0, &[n(0), r(1), n(1)])), 0x0001_0013);
    assert_eq!(list_ids(&e, race), [0x14, 0x14, 0x14]);
    assert_eq!(freed().len(), 1);
    assert_eq!(run(&mut e, CHECK, 0, &[n(0), r(1), r(2)]), -2.0, "not found");
    assert_eq!(low(run(&mut e, DELETE, 0, &[n(0), r(1), n(-2)])), 0x0001_0014, "-2: the head");
    assert_eq!(run(&mut e, DELETE, 0, &[n(0), r(1), n(5)]), -1.0, "past the end");
    // Kinds 2, 3, 4, 8, 9, 11, 12 and above 13 are no list: false.
    for which in [2, 3, 4, 8, 9, 11, 12, 14] {
        assert_eq!(call(&mut e, GET, 0, &[n(which), r(1), n(0)]).0, 0, "{which}");
    }
    assert_eq!(call(&mut e, GET, 0, &[s("x"), r(1), n(0)]).0, 0, "a failed read");
    // Eyes (kind 1): an empty list takes the first at its head.
    assert_eq!(run(&mut e, ADD, 0, &[n(1), r(1), r(4), n(0)]), 0.0);
    assert_eq!(list_ids(&e, RACE + 0xa8), [0x15]);
    console(&mut e);
    run(&mut e, ADD, 0, &[n(1), r(1), r(4), n(-2)]);
    run(&mut e, CHECK, 0, &[n(1), r(1), r(4)]);
    let lines = seen();
    assert_eq!(lines[0], (b"GenericAddForm: Which: %d, Index: %d".to_vec(), vec![1, 0]));
    assert_eq!(lines[1], (b"GenericCheckForm: Which: %d, Returned: %d.".to_vec(), vec![1, 0]));
    // The other lines, each with the form's id in hex.
    run(&mut e, GET, 0, &[n(1), r(1), n(0)]);
    run(&mut e, REPLACE, 0, &[n(1), r(1), r(4), n(0)]);
    run(&mut e, CHECK, 0, &[n(0), r(1), r(4)]);
    run(&mut e, ADD, 0, &[n(1), r(1), r(4), n(7)]);
    run(&mut e, DELETE, 0, &[n(1), r(1), n(0)]);
    let lines = seen();
    let texts: Vec<&[u8]> = lines.iter().map(|l| l.0.as_slice()).collect();
    assert_eq!(
        texts,
        [
            &b"GenericGetForm: Which: %d, Returned: %08X."[..],
            b"GenericChangeForm: Which: %d, Replaced: %08X.",
            b"GenericAddForm: Which: %d, Index: %d",
            b"GenericDeleteForm: Which: %d, Deleted: %08X.",
        ],
        "a check of a list it cannot take prints nothing"
    );
    assert_eq!(lines[0].1, [1, 0x0001_0015]);
    assert_eq!(lines[2].1, [1, u32::MAX], "an index past the end prints -1");
}

/// `GenericCheckForm` over a list with an empty node in it: xNVSE reads the id of the node's
/// missing form (`100354C6`) and the game stops; the script reference checks row passes over it.
/// An empty head counts as an empty list in both presets (`Count` is 0: "Not found!", -2).
#[test]
fn generic_check_over_an_empty_node() {
    for checks in [false, true] {
        let mut e = setup();
        crate::invref::set_checks(&mut e, checks);
        set_script_ref(&mut e, 1, RACE);
        set_script_ref(&mut e, 2, HAIR_B);
        e.mem.set_u32(NODES, 0);
        e.mem.set_u32(NODES + 4, NODES + 8);
        e.mem.set_u32(NODES + 8, HAIR_B);
        let found = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            run(&mut e, CHECK, 0, &[n(0), r(1), r(2)])
        }));
        match found {
            Ok(v) => assert!(checks && v == 2.0, "{checks} {v}"),
            Err(_) => assert!(!checks),
        }
        e.mem.set_u32(RACE + 0x8c, 0);
        assert_eq!(run(&mut e, CHECK, 0, &[n(0), r(1), r(2)]), -2.0, "{checks}");
    }
}

/// A form list (kind 13) takes any form, its count of added forms untouched; an actor base's
/// packages (kind 5) through the calling reference's base form.
#[test]
fn generic_lists_of_form_lists_and_actor_bases() {
    let mut e = setup();
    set_script_ref(&mut e, 1, FORM_LIST);
    set_script_ref(&mut e, 2, DOOR);
    set_script_ref(&mut e, 3, NPC_REF);
    assert_eq!(run(&mut e, ADD, 0, &[n(13), r(1), r(2), n(-2)]), 0.0);
    assert_eq!(list_ids(&e, FORM_LIST + 0x18), [0x17]);
    assert_eq!(e.mem.u32(FORM_LIST + 0x20), 0);
    assert_eq!(run(&mut e, CHECK, 0, &[n(13), r(1), r(2)]), 0.0);
    // Packages: the package must be a package.
    set_classes(&mut e, SOUND, 1 << 40);
    set_script_ref(&mut e, 4, SOUND);
    assert_eq!(run(&mut e, ADD, NPC_REF, &[n(5), r(3), r(4), n(0)]), 0.0);
    assert_eq!(list_ids(&e, NPC + 0xa8), [0x18]);
    assert_eq!(run(&mut e, ADD, NPC_REF, &[n(5), r(3), r(2), n(0)]), -1.0);
}

/// The default objects: the nth (0 to 33) and all 34 as an array (0 for an empty one); setting
/// one touches no result.
#[test]
fn default_objects() {
    let mut e = setup();
    e.mem.set_u32(MANAGER + 0x18 + 4 * 3, DOOR);
    assert_eq!(low(run(&mut e, 0x15e3, 0, &[n(3)])), 0x0001_0017);
    assert_eq!(run(&mut e, 0x15e3, 0, &[n(34)]), 0.0);
    assert_eq!(run(&mut e, 0x15e3, 0, &[n(-1)]), 0.0);
    set_script_ref(&mut e, 1, SOUND);
    assert_eq!(run(&mut e, 0x15e4, 0, &[n(33), r(1)]), 0.0);
    assert_eq!(e.mem.u32(MANAGER + 0x18 + 4 * 33), SOUND);
    run(&mut e, 0x15e4, 0, &[n(34), r(1)]);
    assert_eq!(e.mem.u32(MANAGER + 0x18 + 4 * 34), 0, "past the end: nothing");
    let array = run(&mut e, 0x15e5, 0, &[]) as u32;
    assert_eq!(vars::array(&e, array).unwrap().len(), 34);
    let at = |e: &mut Engine, i: f64| vars::get_element(e, array, &Key::Number(i)).unwrap().data;
    assert_eq!(at(&mut e, 3.0), vars::Data::Form(0x0001_0017));
    assert_eq!(at(&mut e, 0.0), vars::Data::Form(0));
    assert_eq!(at(&mut e, 33.0), vars::Data::Form(0x0001_0018));
}

/// The current objective's targets (the game's `00952BA0` on the player): one array each, its
/// doors and then its target; an empty node still makes an (empty) array.
#[test]
fn quest_objective_teleport_links() {
    let mut e = setup();
    e.set_global::<u32>(0x011d_ea3c, NPC_REF);
    e.register(CURRENT_QUEST_TARGETS, |_, _| eax(TARGETS));
    let (t1, links) = (TARGETS + 0x100, TARGETS + 0x200);
    e.mem.set_u32(TARGETS, t1);
    e.mem.set_u32(TARGETS + 4, TARGETS + 0x40);
    e.mem.set_u32(TARGETS + 0x44, TARGETS + 0x48);
    e.mem.set_u32(TARGETS + 0x48, TARGETS + 0x300);
    e.mem.set_u32(t1 + 0xc, SOUND);
    e.mem.set_u32(t1 + 0x24, links);
    e.mem.set_u32(t1 + 0x28, 3);
    e.mem.set_u32(links, DOOR);
    e.mem.set_u32(links + 0x20, RACE);
    let array = run(&mut e, 0x1609, 0, &[]) as u32;
    let subs: Vec<u32> = (0..3)
        .map(|i| match vars::get_element(&mut e, array, &Key::Number(f64::from(i))).unwrap().data {
            vars::Data::Array(a) => a,
            other => panic!("{other:?}"),
        })
        .collect();
    let forms = |e: &mut Engine, a: u32| -> Vec<vars::Data> {
        let len = vars::array(e, a).unwrap().len();
        (0..len)
            .map(|i| vars::get_element(e, a, &Key::Number(i as f64)).unwrap().data)
            .collect()
    };
    assert_eq!(
        forms(&mut e, subs[0]),
        [vars::Data::Form(0x0001_0017), vars::Data::Form(0x0001_0012), vars::Data::Form(0x0001_0018)]
    );
    assert!(forms(&mut e, subs[1]).is_empty(), "an empty node");
    assert!(forms(&mut e, subs[2]).is_empty(), "a target without data");
    assert!(subs[0] > array && subs[1] > subs[0]);
}

/// Special idles: the nth name (every node counted), added at the end as a copy, deleted by
/// name without case (the name freed), all deleted (the nodes freed). For a reference xNVSE takes
/// the calling reference's base form, so a reference given with no calling reference finds none.
#[test]
fn special_idles() {
    let mut e = setup();
    let names = NPC + 0xc8;
    set_script_ref(&mut e, 1, NPC);
    set_script_ref(&mut e, 2, NPC_REF);
    run(&mut e, 0x1619, NPC_REF, &[s("Idle1")]);
    run(&mut e, 0x1619, 0, &[s("Idle2"), r(1)]);
    run(&mut e, 0x1619, 0, &[s(""), r(1)]);
    run(&mut e, 0x1619, 0, &[s("Idle3"), r(2)]);
    let texts = |e: &Engine| -> Vec<String> {
        tlist_nodes(e, names)
            .into_iter()
            .map(|t| String::from_utf8_lossy(&crate::table::guest_text(e, t)).into_owned())
            .collect()
    };
    assert_eq!(texts(&e), ["Idle1", "Idle2"]);
    let name = run(&mut e, 0x1618, NPC_REF, &[n(1)]);
    assert_eq!(vars::string(&e, name as u32).unwrap(), b"Idle2");
    let name = run(&mut e, 0x1618, NPC_REF, &[n(5)]);
    assert_eq!(vars::string(&e, name as u32).unwrap(), b"");
    let name = run(&mut e, 0x1618, NPC_REF, &[s("x")]);
    assert_eq!(vars::string(&e, name as u32).unwrap(), b"", "a failed read: \"\"");
    freed();
    let idle1 = tlist_nodes(&e, names)[0];
    run(&mut e, 0x161a, NPC_REF, &[s("IDLE1")]);
    assert_eq!(texts(&e), ["Idle2"]);
    let gone = freed();
    assert_eq!((gone.len(), gone[1]), (2, idle1), "the next node, then the name");
    run(&mut e, 0x1619, NPC_REF, &[s("Idle4")]);
    run(&mut e, 0x161b, NPC_REF, &[]);
    assert!(texts(&e).is_empty());
    assert_eq!(freed().len(), 1, "one node after the head");
}

/// A creature's special idles: added, read and deleted by name as an NPC's, but `DelAnimations`
/// leaves them: its second test asks for the NPC again (`else if (npc)`), and the DLL tests only
/// the NPC (`10035EDD`).
#[test]
fn a_creatures_idles_survive_del_animations() {
    let mut e = setup();
    let creature = NPC;
    set_classes(&mut e, creature, 1 << 41 | 1 << 26);
    run(&mut e, 0x1619, NPC_REF, &[s("Idle1")]);
    run(&mut e, 0x1619, NPC_REF, &[s("Idle2")]);
    let name = run(&mut e, 0x1618, NPC_REF, &[n(0)]);
    assert_eq!(vars::string(&e, name as u32).unwrap(), b"Idle1");
    run(&mut e, 0x161b, NPC_REF, &[]);
    assert_eq!(tlist_nodes(&e, creature + 0xc8).len(), 2);
    run(&mut e, 0x161a, NPC_REF, &[s("idle2")]);
    assert_eq!(tlist_nodes(&e, creature + 0xc8).len(), 1);
}

/// A name of 256 bytes or more overflows `AddAnimation`'s and `DelAnimation`'s 256-byte buffer:
/// without the script list and scan guard row the port stops the game; with it the whole name is
/// used. 255 bytes and the terminator fit.
#[test]
fn a_long_idle_name() {
    let long = "i".repeat(256);
    for op in [0x1619, 0x161a] {
        let mut e = setup();
        let stop = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            run(&mut e, op, NPC_REF, &[s(&long)])
        }));
        assert!(stop.is_err(), "{op:x}");
    }
    let mut e = setup();
    run(&mut e, 0x1619, NPC_REF, &[s(&long[1..])]);
    super::super::array::set_fix(&mut e, super::super::array::LIST_GUARD, true);
    run(&mut e, 0x1619, NPC_REF, &[s(&long)]);
    let name = run(&mut e, 0x1618, NPC_REF, &[n(1)]);
    assert_eq!(vars::string(&e, name as u32).unwrap().len(), 256);
    run(&mut e, 0x161a, NPC_REF, &[s(&long)]);
    let name = run(&mut e, 0x1618, NPC_REF, &[n(1)]);
    assert_eq!(vars::string(&e, name as u32).unwrap(), b"", "the 256-byte one deleted");
}

/// Runs an expression command with these expressions as its arguments.
fn run_expr(e: &mut Engine, opcode: u32, exprs: &[Vec<u8>]) -> f64 {
    let mut bytes = vec![exprs.len() as u8];
    for x in exprs {
        bytes.extend_from_slice(&(x.len() as u16 + 2).to_le_bytes());
        bytes.extend_from_slice(x);
    }
    e.mem.set_u16(DATA, opcode as u16);
    e.mem.set_u16(DATA + 2, bytes.len() as u16);
    e.mem.write(DATA + 4, &bytes);
    e.mem.set_u32(OFFSET, 4);
    e.mem.set_f64(RESULT, 0.0);
    let record = crate::table::record_of(e, opcode);
    let (params, execute) = (e.mem.u32(record + 0x14), e.mem.u32(record + 0x18));
    e.call(execute, &[params, DATA, 0, 0, SCRIPT, LIST, RESULT, OFFSET]);
    e.mem.f64(RESULT)
}

/// A form of the script's reference list `index` (`R`) and a number (`Z`), as expressions.
fn form_tok(index: u16) -> Vec<u8> {
    let mut b = vec![b'R'];
    b.extend_from_slice(&index.to_le_bytes());
    b
}
fn num(v: f64) -> Vec<u8> {
    let mut b = vec![b'Z'];
    b.extend_from_slice(&v.to_le_bytes());
    b
}

/// A door's open (0) and close (any other) sound; a form that is no door: 0, and the console says
/// so; a door without the sound is read through without the script reference checks row, 0 with
/// it.
#[test]
fn door_sounds() {
    let mut e = setup();
    e.register(LOOKUP_FORM, |_, a| {
        eax(match a[0] {
            0x0001_0017 => DOOR,
            0x0001_0012 => RACE,
            _ => 0,
        })
    });
    e.mem.set_u32(DOOR + 0x78, SOUND);
    set_script_ref(&mut e, 1, DOOR);
    set_script_ref(&mut e, 2, RACE);
    assert_eq!(low(run_expr(&mut e, 0x1677, &[form_tok(1), num(0.0)])), 0x0001_0018);
    console(&mut e);
    assert_eq!(run_expr(&mut e, 0x1677, &[form_tok(2), num(0.0)]), 0.0);
    let lines = seen();
    assert_eq!((lines[0].0.as_slice(), lines[0].1[0]), (&b"Form %X is not a TESObjectDOOR"[..], 0x0001_0012));
    let stop = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        run_expr(&mut e, 0x1677, &[form_tok(1), num(1.0)])
    }));
    assert!(stop.is_err(), "no close sound");
    let mut e = setup();
    e.register(LOOKUP_FORM, |_, _| eax(DOOR));
    crate::invref::set_checks(&mut e, true);
    set_script_ref(&mut e, 1, DOOR);
    assert_eq!(run_expr(&mut e, 0x1677, &[form_tok(1), num(1.0)]), 0.0);
}

/// `FireChallenge` with six arguments calls the game's increment (numbers truncated to `int`s as
/// `cvttsd2si` does, 0, the weapon as a form) and answers its `eax` as an unsigned 32-bit number
/// (`100361BB`); five arguments do nothing.
#[test]
fn fire_challenge_calls_the_game() {
    let mut e = setup();
    e.register(LOOKUP_FORM, |_, _| eax(DOOR));
    e.register(INCREMENT_CHALLENGE, |_, a| {
        SEEN.with(|p| p.borrow_mut().push((Vec::new(), a[..7].to_vec())));
        eax(0x8000_0001)
    });
    set_script_ref(&mut e, 1, DOOR);
    let args = [num(3.9), num(-2.5), form_tok(1), num(1.0), num(2.0), num(7.0)];
    assert_eq!(run_expr(&mut e, 0x1678, &args), 2_147_483_649.0);
    assert_eq!(seen(), [(Vec::new(), vec![3, (-2i32) as u32, 0, DOOR, 1, 2, 7])]);
    assert_eq!(run_expr(&mut e, 0x1678, &args[..5]), 0.0);
    assert!(seen().is_empty());
}
