//! Tests of the rest of the MiscRef batch through the commands' records (and the condition evals
//! through theirs), with the real game's answers of run `inv6r1` (`real-tests/gr_u4_misc.txt`)
//! where it gave them. Extra data lists are answered by a stand-in of `BaseExtraList::GetByType`
//! (a list's data of type `t` at `+0x100 + 4t`).

use super::super::test_support::*;
use super::*;
use crate::vars::{self, Key, Value};
use crate::Nvse;

const ITEM: u32 = 0x0200_0000;
const NPC_REF: u32 = 0x0200_0400;
const NPC: u32 = 0x0200_0800;
const OTHER_NPC: u32 = 0x0200_0c00;
const FACTION: u32 = 0x0200_1000;
const CELL: u32 = 0x0200_1400;
const EXTRA: u32 = 0x0200_1800;
const NODES: u32 = 0x0200_1c00;
const PC: u32 = 0x0200_2000;
const MOVER: u32 = 0x0200_2400;
const VTBL: u32 = 0x0200_2800;
const RAGDOLL_AT: u32 = 0x0200_3000;
const GLOBALS: u32 = 0x0200_3400;
const SHADER: u32 = 0x0200_3800;
const EFFECTS: u32 = 0x0200_4000;
const FORM: u32 = 0x0200_5000;
const FORM_VTBL: u32 = 0x0200_5800;
const HEAP: u32 = 0x0200_8000;
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

fn put_extra(e: &mut Engine, list: u32, kind: u32, data: u32) {
    e.mem.set_u32(list + 0x100 + 4 * kind, data);
}

/// An item reference and an NPC reference (base `NPC`, an actor base whose data is at `+0x30`)
/// in `CELL`, a faction, `GetByType`, a bump heap, prints and calls kept.
fn setup() -> Engine {
    let mut e = engine_with_table();
    e.map(0x011c_3000, 0x1000);
    e.map(0x011e_0000, 0x1000);
    e.register(0x0041_0220, |e, a| eax(e.mem.u32(a[0] + 0x100 + 4 * a[1])));
    e.register(0x0040_1000, |e, a| {
        let at = e.mem.u32(HEAP).max(HEAP + 0x100);
        e.mem.set_u32(HEAP, at + a[0].next_multiple_of(0x100));
        eax(at)
    });
    e.register(0x0071_b160, |_, _| eax(CONSOLE));
    e.register(0x0071_d0a0, |e, a| {
        let text = crate::table::guest_text(e, a[1]);
        let words = (0..2).map(|i| e.mem.u32(a[2] + 4 * i)).collect();
        SEEN.with(|p| p.borrow_mut().push((text, words)));
        eax(0)
    });
    for (form, kind, id) in [
        (ITEM, 0x3a, 0x10),
        (NPC_REF, 0x3b, 0x11),
        (NPC, 0x2a, 0x12),
        (OTHER_NPC, 0x2a, 0x13),
        (FACTION, 0x08, 0x14),
        (CELL, 0x39, 0x15),
    ] {
        e.mem.set_u8(form + 4, kind);
        e.mem.set_u32(form + 0xc, 0x0001_0000 + id);
    }
    set_classes(&mut e, ITEM, 1);
    set_classes(&mut e, NPC_REF, 1 | 1 << 21);
    set_classes(&mut e, NPC, 1 << 33);
    set_classes(&mut e, OTHER_NPC, 1 << 33);
    set_classes(&mut e, FACTION, 1 << 32);
    e.mem.set_u32(ITEM + 0x20, OTHER_NPC);
    e.mem.set_u32(ITEM + 0x40, CELL);
    e.mem.set_u32(NPC_REF + 0x20, NPC);
    e.mem.set_u32(NPC_REF + 0x40, CELL);
    e.mem.set_u32(SCRIPT + 0xc, 0x0500_0d00);
    e
}

/// The console open and running a line.
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

const HAS_OWNERSHIP: u32 = 0x14f7;
const IS_OWNED: u32 = 0x14f8;
const IS_PLAYER_SWIMMING: u32 = 0x15a0;
const GET_TFC: u32 = 0x15a1;
const GET_FIK: u32 = 0x1605;
const SET_FIK: u32 = 0x1606;
const HAS_EFFECT_SHADER: u32 = 0x1608;
const SET_EDITOR_ID_OP: u32 = 0x1645;
const CREATE_FORM_LIST: u32 = 0x164c;
const GET_HEADING_ANGLE_X: u32 = 0x165e;
const EVALUATE_INVENTORY: u32 = 0x1674;

/// An `ExtraOwnership` (and an `ExtraRank`) on `list`.
fn own(e: &mut Engine, list: u32, owner: u32, rank: Option<u32>, at: u32) {
    e.mem.set_u32(at + 0xc, owner);
    put_extra(e, list, 0x21, at);
    if let Some(rank) = rank {
        e.mem.set_u32(at + 0x2c, rank);
        put_extra(e, list, 0x23, at + 0x20);
    }
}

/// The item owned by the NPC's base: `HasOwnership` (the NPC asks about the item) and `IsOwned`
/// (the item asks about the NPC) answer 1, as command and as condition; another base 0; no
/// argument 0 with nothing printed; a wrong-typed argument (an item reference where `IsOwned`
/// takes an actor) fails the read.
#[test]
fn direct_ownership() {
    let mut e = setup();
    set_script_ref(&mut e, 1, ITEM);
    set_script_ref(&mut e, 2, NPC_REF);
    assert_eq!(
        run(&mut e, HAS_OWNERSHIP, NPC_REF, &[r(1)]),
        0.0,
        "no owner"
    );
    own(&mut e, ITEM + 0x44, NPC, None, EXTRA);
    assert_eq!(run(&mut e, HAS_OWNERSHIP, NPC_REF, &[r(1)]), 1.0);
    assert_eq!(run(&mut e, IS_OWNED, ITEM, &[r(2)]), 1.0);
    assert_eq!(eval(&mut e, HAS_OWNERSHIP, NPC_REF, ITEM, 0), 1.0);
    assert_eq!(eval(&mut e, IS_OWNED, ITEM, NPC_REF, 0), 1.0);
    assert_eq!(eval(&mut e, IS_OWNED, ITEM, 0, 0), 0.0);
    assert_eq!(eval(&mut e, IS_OWNED, 0, NPC_REF, 0), 0.0);
    own(&mut e, ITEM + 0x44, OTHER_NPC, None, EXTRA);
    assert_eq!(run(&mut e, HAS_OWNERSHIP, NPC_REF, &[r(1)]), 0.0);
    console(&mut e);
    assert_eq!(run(&mut e, HAS_OWNERSHIP, NPC_REF, &[]), 0.0);
    assert!(seen().is_empty(), "nothing printed without an argument");
    assert_eq!(
        run(&mut e, IS_OWNED, ITEM, &[r(1)]),
        0.0,
        "an item is no actor"
    );
    assert!(seen().is_empty());
    own(&mut e, ITEM + 0x44, NPC, None, EXTRA);
    assert_eq!(run(&mut e, IS_OWNED, ITEM, &[r(2)]), 1.0);
    let lines = seen();
    assert_eq!(lines[0].0, b"IsOwned >> %.2f");
    assert_eq!(lines[0].1, double_words(1.0));
    run(&mut e, HAS_OWNERSHIP, NPC_REF, &[r(1)]);
    assert_eq!(seen()[0].0, b"HasOwnership >> %.2f");
    assert_eq!(run(&mut e, HAS_OWNERSHIP, 0, &[r(1)]), 0.0);
}

/// A faction owner: the NPC's rank in it (its own `ExtraFactionChanges` first, else its base's
/// faction list) against the required rank (the item's `ExtraRank`; with the cell's owner the
/// cell's). A rank byte of 255 is -1; no rank anywhere is -1, under a required 0.
#[test]
fn faction_ownership_and_ranks() {
    let mut e = setup();
    own(&mut e, ITEM + 0x44, FACTION, Some(2), EXTRA);
    let base_data = NPC + 0x30;
    let entry = NODES;
    e.mem.set_u32(entry, FACTION);
    e.mem.set_u8(entry + 4, 2);
    e.mem.set_u32(base_data + 0x2c, entry);
    assert_eq!(
        eval(&mut e, HAS_OWNERSHIP, NPC_REF, ITEM, 0),
        1.0,
        "the base's rank 2"
    );
    e.mem.set_u8(entry + 4, 1);
    assert_eq!(eval(&mut e, HAS_OWNERSHIP, NPC_REF, ITEM, 0), 0.0);
    // The reference's own changes win: rank 3.
    let changes = EXTRA + 0x100;
    let node = NODES + 0x40;
    let data = NODES + 0x80;
    e.mem.set_u32(node, 0);
    e.mem.set_u32(node + 4, node + 8);
    e.mem.set_u32(node + 8, data);
    e.mem.set_u32(data, FACTION);
    e.mem.set_u8(data + 4, 3);
    e.mem.set_u32(changes + 0xc, node);
    put_extra(&mut e, NPC_REF + 0x44, 0x5e, changes);
    assert_eq!(eval(&mut e, HAS_OWNERSHIP, NPC_REF, ITEM, 0), 1.0);
    e.mem.set_u8(data + 4, 255);
    assert_eq!(
        eval(&mut e, HAS_OWNERSHIP, NPC_REF, ITEM, 0),
        0.0,
        "255 is -1"
    );
    own(&mut e, ITEM + 0x44, FACTION, Some(0x1ff), EXTRA);
    assert_eq!(
        eval(&mut e, HAS_OWNERSHIP, NPC_REF, ITEM, 0),
        1.0,
        "0x1FF is -1 too"
    );
    // The cell's owner and rank when the item has no owner.
    put_extra(&mut e, ITEM + 0x44, 0x21, 0);
    put_extra(&mut e, ITEM + 0x44, 0x23, 0);
    own(&mut e, CELL + 0x28, FACTION, Some(0), EXTRA + 0x200);
    e.mem.set_u8(data + 4, 0);
    assert_eq!(eval(&mut e, HAS_OWNERSHIP, NPC_REF, ITEM, 0), 1.0);
    put_extra(&mut e, NPC_REF + 0x44, 0x5e, 0);
    e.mem.set_u32(base_data + 0x2c, 0);
    assert_eq!(
        eval(&mut e, HAS_OWNERSHIP, NPC_REF, ITEM, 0),
        0.0,
        "-1 under 0"
    );
}

/// A reference in no cell (an inventory reference's) with no owner of its own: xNVSE reads the
/// cell's list through nothing and the game stops; with the script reference checks row, 0.
#[test]
fn a_reference_in_no_cell() {
    let mut e = setup();
    e.mem.set_u32(ITEM + 0x40, 0);
    let stop = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        eval(&mut e, HAS_OWNERSHIP, NPC_REF, ITEM, 0)
    }));
    assert!(stop.is_err());
    let mut e = setup();
    e.mem.set_u32(ITEM + 0x40, 0);
    crate::invref::set_checks(&mut e, true);
    assert_eq!(eval(&mut e, HAS_OWNERSHIP, NPC_REF, ITEM, 0), 0.0);
    // And an asker without a base form, with an owner to compare.
    own(&mut e, ITEM + 0x44, NPC, None, EXTRA);
    e.mem.set_u32(NPC_REF + 0x20, 0);
    assert_eq!(eval(&mut e, HAS_OWNERSHIP, NPC_REF, ITEM, 0), 0.0);
}

/// The player mover's ninth entry gives the flags (bit 11 swimming); its third (`+8`, which
/// clears move modes) is never called. The free camera is the OS globals' byte `+6`. The real
/// game answered 0 and 0 (run `inv6r1`); the console prints `%f`.
#[test]
fn swimming_and_the_free_camera() {
    let mut e = setup();
    e.set_global::<u32>(0x011d_ea3c, PC);
    e.mem.set_u32(PC + 0x190, MOVER);
    e.mem.set_u32(MOVER, VTBL);
    e.mem.set_u32(VTBL + 8, 0x0300_9000);
    e.register(0x0300_9000, |_, _| panic!("the mover's +8 entry"));
    e.mem.set_u32(VTBL + 0x20, 0x0300_9100);
    e.register(0x0300_9100, |e, _| eax(e.mem.u32(MOVER + 0x94)));
    assert_eq!(run(&mut e, IS_PLAYER_SWIMMING, 0, &[]), 0.0);
    e.mem.set_u32(MOVER + 0x94, 0x800 | 0x200);
    assert_eq!(run(&mut e, IS_PLAYER_SWIMMING, 0, &[]), 1.0);
    assert_eq!(eval(&mut e, IS_PLAYER_SWIMMING, 0, 0, 0), 1.0);
    e.mem.set_u32(MOVER + 0x94, 0x7ff);
    assert_eq!(eval(&mut e, IS_PLAYER_SWIMMING, 0, 0, 0), 0.0);
    e.set_global::<u32>(0x011d_ea0c, GLOBALS);
    assert_eq!(run(&mut e, GET_TFC, 0, &[]), 0.0);
    e.mem.set_u8(GLOBALS + 6, 1);
    assert_eq!(run(&mut e, GET_TFC, 0, &[]), 1.0);
    assert_eq!(eval(&mut e, GET_TFC, 0, 0, 0), 1.0);
    console(&mut e);
    run(&mut e, GET_TFC, 0, &[]);
    run(&mut e, IS_PLAYER_SWIMMING, 0, &[]);
    let lines = seen();
    assert_eq!(
        lines[0],
        (b"GetTFC: %f".to_vec(), double_words(1.0).to_vec())
    );
    assert_eq!(lines[1].0, b"Player is swimming? %f");
}

/// Foot IK: the actor's ragdoll byte `+0x220`, set only where `+0x21F` allows (1 for any
/// non-zero); a non-actor answers 0, no calling reference makes the handler answer false. The
/// real game: the player 1, Doc 1, `SetActorFIKstatus 0` on Doc reads 0 (run `inv6r1`).
#[test]
fn foot_ik() {
    let mut e = setup();
    e.mem.set_u32(NPC_REF + 0xac, RAGDOLL_AT);
    e.mem.set_u8(RAGDOLL_AT + 0x220, 1);
    assert_eq!(run(&mut e, GET_FIK, NPC_REF, &[]), 1.0);
    assert_eq!(eval(&mut e, GET_FIK, NPC_REF, 0, 0), 1.0);
    assert_eq!(run(&mut e, GET_FIK, ITEM, &[]), 0.0, "no actor");
    assert_eq!(call(&mut e, GET_FIK, 0, &[]), (0, 0.0));
    assert_eq!(run(&mut e, SET_FIK, NPC_REF, &[n(0)]), 0.0);
    assert_eq!(e.mem.u8(RAGDOLL_AT + 0x220), 1, "+0x21F is 0: not set");
    e.mem.set_u8(RAGDOLL_AT + 0x21f, 1);
    run(&mut e, SET_FIK, NPC_REF, &[n(0)]);
    assert_eq!(run(&mut e, GET_FIK, NPC_REF, &[]), 0.0);
    run(&mut e, SET_FIK, NPC_REF, &[n(7)]);
    assert_eq!(e.mem.u8(RAGDOLL_AT + 0x220), 1);
    assert_eq!(call(&mut e, SET_FIK, 0, &[n(1)]), (0, 0.0));
    assert_eq!(
        call(&mut e, SET_FIK, 0, &[s("x")]),
        (1, 0.0),
        "a failed read first"
    );
}

/// The shader hit effects of the process manager's temporary effects on the calling reference
/// with that shader, counted; other effects, targets and shaders are not. The real game answered
/// 0 (run `inv6r1`). A form that is no effect shader fails the read.
#[test]
fn effect_shaders() {
    let mut e = setup();
    e.mem.set_u8(SHADER + 4, 0x4f);
    e.mem.set_u32(SHADER + 0xc, 0x0001_0020);
    set_script_ref(&mut e, 1, SHADER);
    set_script_ref(&mut e, 2, ITEM);
    assert_eq!(run(&mut e, HAS_EFFECT_SHADER, NPC_REF, &[r(1)]), 0.0);
    let list = 0x011e_0e80 + 0x60;
    let mut node = list;
    for (i, (target, shader, hit)) in [
        (NPC_REF, SHADER, true),
        (ITEM, SHADER, true),
        (NPC_REF, ITEM, true),
        (NPC_REF, SHADER, false),
        (NPC_REF, SHADER, true),
    ]
    .into_iter()
    .enumerate()
    {
        let effect = EFFECTS + 0x100 * i as u32;
        e.mem.set_u32(effect + 0x1c, target);
        e.mem.set_u32(effect + 0x30, shader);
        if hit {
            set_classes(&mut e, effect, 1 << 35);
        }
        if i > 0 {
            let next = EFFECTS + 0x800 + 8 * i as u32;
            e.mem.set_u32(node + 4, next);
            node = next;
        }
        e.mem.set_u32(node, effect);
    }
    assert_eq!(run(&mut e, HAS_EFFECT_SHADER, NPC_REF, &[r(1)]), 2.0);
    assert_eq!(run(&mut e, HAS_EFFECT_SHADER, ITEM, &[r(1)]), 1.0);
    assert_eq!(run(&mut e, HAS_EFFECT_SHADER, NPC_REF, &[r(2)]), 0.0);
    assert_eq!(run(&mut e, HAS_EFFECT_SHADER, 0, &[r(1)]), 0.0);
}

/// A form whose vtable answers `SetEditorID_AtRuntime` at `+0x134` and stops at `+0x130`.
fn editable_form(e: &mut Engine) {
    e.mem.set_u8(FORM + 4, 0x55);
    e.mem.set_u32(FORM + 0xc, 0xff00_1341);
    e.mem.set_u32(FORM, FORM_VTBL);
    e.mem.set_u32(FORM_VTBL + 0x130, 0x0300_9200);
    e.register(0x0300_9200, |_, _| panic!("the editor id's getter"));
    e.mem.set_u32(FORM_VTBL + 0x134, 0x0300_9300);
    e.register(0x0300_9300, |e, a| {
        let text = crate::table::guest_text(e, a[1]);
        SEEN.with(|p| p.borrow_mut().push((text, vec![a[0]])));
        eax(1)
    });
}

/// `SetEditorID` hands the text to the form's own function and answers 1 (the real game: 1, run
/// `inv6r1`); a string in the form's place fails the read: 0.
#[test]
fn editor_ids() {
    let mut e = setup();
    editable_form(&mut e);
    set_script_ref(&mut e, 1, FORM);
    assert_eq!(
        run(&mut e, SET_EDITOR_ID_OP, 0, &[r(1), s("nvrsTestList")]),
        1.0
    );
    assert_eq!(seen(), [(b"nvrsTestList".to_vec(), vec![FORM])]);
    assert_eq!(run(&mut e, SET_EDITOR_ID_OP, 0, &[s("x"), s("y")]), 0.0);
    assert!(seen().is_empty());
}

fn num(v: f64) -> Vec<u8> {
    let mut b = vec![b'Z'];
    b.extend_from_slice(&v.to_le_bytes());
    b
}
fn text(t: &str) -> Vec<u8> {
    let mut b = vec![b'S'];
    b.extend_from_slice(&(t.len() as u16).to_le_bytes());
    b.extend_from_slice(t.as_bytes());
    b
}
fn array_var(idx: u16) -> Vec<u8> {
    let mut b = vec![b'V', 3, 0, 0];
    b.extend_from_slice(&idx.to_le_bytes());
    b
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

/// `CreateFormList`: a new list (the game's constructor, its id the result) named with the text
/// given (a number gives ""), filled at its end with the array's forms in order, numbers and
/// unknown ids passed over; `numAddedObjects` stays 0. The real game: `CreateFormList
/// "nvrsTestList" (Ar_List pistol, player)` gave FF001341 with 2 entries, the second 00000014
/// (run `inv6r1`).
#[test]
fn new_form_lists() {
    let mut e = setup();
    e.register(0x00ec_a7f4, |_, a| {
        eax(u32::from((a[0] as u8).to_ascii_uppercase()))
    });
    e.register(0x0058_f9d0, |e, a| {
        e.mem.write(a[0], &[0u8; 0x24]);
        e.mem.set_u32(a[0], FORM_VTBL);
        e.mem.set_u8(a[0] + 4, 0x55);
        e.mem.set_u32(a[0] + 0xc, 0xff00_1341);
        eax(a[0])
    });
    editable_form(&mut e);
    e.register(0x0048_39c0, |_, a| {
        eax(match a[0] {
            0x0e3778 => ITEM,
            0x14 => NPC_REF,
            _ => 0,
        })
    });
    let id = vars::create_array(&mut e, vars::KEY_NUMBER, true, 5);
    for (i, v) in [
        Value::Form(0x0e3778),
        Value::Number(5.0),
        Value::Form(0x0bad),
        Value::Form(0x14),
    ]
    .into_iter()
    .enumerate()
    {
        vars::set_element(&mut e, id, &Key::Number(i as f64), v);
    }
    add_local(&mut e, 1, "a1", f64::from(id));
    let made = run_expr(
        &mut e,
        CREATE_FORM_LIST,
        &[text("nvrsTestList"), array_var(1)],
    );
    assert_eq!(made.to_bits() as u32, 0xff00_1341);
    let list = HEAP + 0x100;
    assert_eq!(e.mem.u32(list + 0x18), ITEM);
    let second = e.mem.u32(list + 0x1c);
    assert_eq!(e.mem.u32(second), NPC_REF);
    assert_eq!(e.mem.u32(second + 4), 0);
    assert_eq!(e.mem.u32(list + 0x20), 0);
    assert_eq!(seen(), [(b"nvrsTestList".to_vec(), vec![list])]);
    // No arguments: an empty list and no editor id; a number for the name: "".
    run_expr(&mut e, CREATE_FORM_LIST, &[]);
    assert!(seen().is_empty());
    run_expr(&mut e, CREATE_FORM_LIST, &[num(3.0)]);
    assert_eq!(seen()[0].0, b"");
}

/// The up/down angle: asin of the height difference over the distance, less the x rotation, in
/// degrees, all in `float`s; the distance is the largest float for references in different cells
/// of no common worldspace. Pinned against the real game's numbers (run `inv6r1`): the player
/// (z 7360.774902) to a door (z 7358) 1123.048218 away answered -0.530697 (exactly
/// -583506984960 / 2^40, a `float`); the player's x rotation was not printed, so the one that
/// gives that answer is used.
#[test]
fn heading_angle_x() {
    let mut e = setup();
    let (me, door) = (NPC_REF, ITEM);
    e.mem.set_f32(me + 0x38, 7360.775);
    e.mem.set_f32(door + 0x38, 7358.0);
    e.mem.set_f32(door + 0x30, 1_123.048_2);
    let rot = 0.002_470_8_f32 + 0.530_697 / 57.295_78;
    e.mem.set_f32(me + 0x24, rot);
    set_script_ref(&mut e, 1, door);
    let v = run(&mut e, GET_HEADING_ANGLE_X, me, &[r(1)]);
    assert_eq!(v, f64::from(v as f32), "a float");
    assert!((v - scaled(-583_506_984_960.0)).abs() < 2e-5, "{v}");
    let opposite = 7360.775f32 - 7358.0;
    let hypotenuse = point_distance(&e, me + 0x30, door + 0x30);
    assert_eq!(v as f32, ((opposite / hypotenuse).asin() - rot) * 57.295_78);
    // Different cells of no worldspace: the largest float, so about the x rotation alone.
    let other_cell = CELL + 0x200;
    e.mem.set_u32(door + 0x40, other_cell);
    let v = run(&mut e, GET_HEADING_ANGLE_X, me, &[r(1)]);
    assert_eq!(v as f32, (0.0f32 - rot) * 57.295_78);
    assert_eq!(run(&mut e, GET_HEADING_ANGLE_X, 0, &[r(1)]), 0.0);
    assert_eq!(run(&mut e, GET_HEADING_ANGLE_X, me, &[s("x")]), 0.0);
}

/// JIP's distance: a Newton step on the hardware estimate, within a few units in the last place
/// of the true distance; 0 is 0.
#[test]
fn point_distances() {
    let e = setup();
    for (i, v) in [3.0f32, 4.0, 12.0].into_iter().enumerate() {
        e.mem.set_f32(EXTRA + 4 * i as u32, v);
    }
    let d = point_distance(&e, EXTRA, EXTRA + 0x40);
    assert!((d - 13.0).abs() <= 13.0 * 4.0 * f32::EPSILON, "{d}");
    assert_eq!(point_distance(&e, EXTRA, EXTRA), 0.0);
}

/// `EvaluateInventory` calls the game's re-evaluation for a character (`IsCharacter` at
/// `+0x218`, the bip names at `+0x1E8`) with the base form, the actor, the names and 1; a
/// non-character actor and a non-actor call nothing. The real game raised OnActorEquip for Doc
/// (run `inv6r1`): the game's own code does that, through the call.
#[test]
fn evaluate_inventory_calls_the_game() {
    let mut e = setup();
    e.mem.set_u32(NPC_REF, VTBL);
    e.mem.set_u32(VTBL + 0x218, 0x0300_9400);
    e.mem.set_u32(VTBL + 0x214, 0x0300_9500);
    e.mem.set_u32(VTBL + 0x1e8, 0x0300_9600);
    e.register(0x0300_9500, |_, _| panic!("the entry before IsCharacter"));
    e.register(0x0300_9400, |e, _| eax(e.mem.u32(PC)));
    e.register(0x0300_9600, |_, _| eax(0x0200_7000));
    e.register(0x0060_6540, |e, a| {
        SEEN.with(|p| p.borrow_mut().push((Vec::new(), a[..4].to_vec())));
        let _ = e;
        eax(0)
    });
    e.mem.set_u32(PC, 1);
    let (ok, result) = call(&mut e, EVALUATE_INVENTORY, NPC_REF, &[]);
    assert_eq!((ok, result), (1, 0.0));
    assert_eq!(seen(), [(Vec::new(), vec![NPC, NPC_REF, 0x0200_7000, 1])]);
    e.mem.set_u32(PC, 0);
    run(&mut e, EVALUATE_INVENTORY, NPC_REF, &[]);
    run(&mut e, EVALUATE_INVENTORY, ITEM, &[]);
    run(&mut e, EVALUATE_INVENTORY, 0, &[]);
    assert!(seen().is_empty());
    let _ = e.ext::<Nvse>();
}
