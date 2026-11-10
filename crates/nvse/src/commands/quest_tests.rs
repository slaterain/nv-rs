//! Tests of `Commands_Quest.cpp` through the commands' records. The answers pinned are xNVSE's
//! as its source and the shipped DLL (`nvse_1_4.dll`, executes `1003B020` to `1003B225`) give
//! them; none of these commands has been run in the real game yet.

use super::super::test_support::*;
use super::*;

const QUEST: u32 = 0x0200_0000;
const OTHER_QUEST: u32 = 0x0200_0400;
const ITEM: u32 = 0x0200_0800;
const PC: u32 = 0x0200_1000;
const NODES: u32 = 0x0200_4000;
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

/// A quest with two objectives (ids 10 and 0x80000000), another with none, an item, the player.
fn setup() -> Engine {
    let mut e = engine_with_table();
    e.register(0x0071_b160, |_, _| eax(CONSOLE));
    e.register(0x0071_d0a0, |e, a| {
        let t = crate::table::guest_text(e, a[1]);
        let words = (0..2).map(|i| e.mem.u32(a[2] + 4 * i)).collect();
        SEEN.with(|p| p.borrow_mut().push((t, words)));
        eax(0)
    });
    for (form, kind, id) in [
        (QUEST, 0x47, 0x10),
        (OTHER_QUEST, 0x47, 0x11),
        (ITEM, 0x1f, 0x12),
    ] {
        e.mem.set_u8(form + 4, kind);
        e.mem.set_u32(form + 0xc, 0x0001_0000 + id);
    }
    let (o1, o2) = (NODES + 0x100, NODES + 0x120);
    e.mem.set_u32(o1 + 4, 10);
    e.mem.set_u32(o2 + 4, 0x8000_0000);
    e.mem.set_u32(QUEST + 0x4c, o1);
    e.mem.set_u32(QUEST + 0x50, NODES);
    e.mem.set_u32(NODES, o2);
    e.set_global::<u32>(PLAYER, PC);
    e.mem.set_u32(SCRIPT + 0xc, 0x0500_0d00);
    for (i, f) in [QUEST, OTHER_QUEST, ITEM].into_iter().enumerate() {
        set_script_ref(&mut e, i as u16 + 1, f);
    }
    e
}

fn run(e: &mut Engine, opcode: u32, this: u32, args: &[Vec<u8>]) -> f64 {
    let (ok, result) = call(e, opcode, this, args);
    assert_eq!(ok, 1);
    result
}

/// The count and nth objective's id (unsigned; -2 the last), 0 for none or a form that is no
/// quest; the console line names the quest ("<no name>" here).
#[test]
fn a_quests_objectives() {
    let mut e = setup();
    assert_eq!(run(&mut e, 0x163b, 0, &[r(1)]), 2.0);
    assert_eq!(run(&mut e, 0x163b, 0, &[r(2)]), 0.0);
    assert_eq!(run(&mut e, 0x163b, 0, &[r(3)]), 0.0, "an item is no quest");
    assert_eq!(run(&mut e, 0x163c, 0, &[r(1), n(0)]), 10.0);
    assert_eq!(run(&mut e, 0x163c, 0, &[r(1), n(-2)]), 2_147_483_648.0);
    assert_eq!(run(&mut e, 0x163c, 0, &[r(1), n(2)]), 0.0);
    assert_eq!(run(&mut e, 0x163c, 0, &[r(2), n(0)]), 0.0);
    e.mem.set_u8(0x011d_ea2e, 1);
    let tls = e.tls();
    e.mem.set_u8(tls + 0x268, 1);
    run(&mut e, 0x163b, 0, &[r(1)]);
    let lines = SEEN.with(|p| p.borrow_mut().drain(..).collect::<Vec<_>>());
    assert_eq!(lines[0].0, b"%s has %d objectives");
    assert_eq!(crate::table::guest_text(&e, lines[0].1[0]), b"<no name>");
    assert_eq!(lines[0].1[1], 2);
}

/// The player's current objective's id (unsigned), the result untouched without one; the current
/// quest set from the read (a failed read sets nothing).
#[test]
fn the_players_objective_and_quest() {
    let mut e = setup();
    assert_eq!(run(&mut e, 0x163d, 0, &[]), 0.0);
    e.mem.set_u32(PC + 0x6bc, NODES + 0x120);
    assert_eq!(run(&mut e, 0x163d, 0, &[]), 2_147_483_648.0);
    run(&mut e, 0x163f, 0, &[r(2)]);
    assert_eq!(e.mem.u32(PC + 0x6b8), OTHER_QUEST);
    run(&mut e, 0x163f, 0, &[r(3)]);
    assert_eq!(e.mem.u32(PC + 0x6b8), OTHER_QUEST, "an item is no quest");
}
