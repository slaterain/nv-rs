//! Tests of `Commands_Factions.cpp` through the commands' records. The answers pinned are
//! xNVSE's as its source and the shipped DLL (`nvse_1_4.dll`, executes `10024F00` to `10025246`)
//! give them; none of these commands has been run in the real game yet.

use super::super::test_support::*;
use super::*;

const NPC_REF: u32 = 0x0200_0000;
const NPC: u32 = 0x0200_0400;
const FACTION_A: u32 = 0x0200_0800;
const FACTION_B: u32 = 0x0200_0c00;
const ITEM: u32 = 0x0200_1000;
const NODES: u32 = 0x0200_2000;

/// An NPC reference (its base `NPC`, an actor base) in factions A (rank 2) and B (rank 0xFF),
/// faction A with three ranks.
fn setup() -> Engine {
    let mut e = engine_with_table();
    for (form, kind, id) in [
        (NPC_REF, 0x3b, 0x10),
        (NPC, 0x2a, 0x11),
        (FACTION_A, 0x08, 0x12),
        (FACTION_B, 0x08, 0x13),
        (ITEM, 0x1f, 0x14),
    ] {
        e.mem.set_u8(form + 4, kind);
        e.mem.set_u32(form + 0xc, 0x0001_0000 + id);
    }
    set_classes(&mut e, NPC_REF, 1 | 1 << 21);
    set_classes(&mut e, NPC, 1 << 26 | 1 << 29);
    e.mem.set_u32(NPC_REF + 0x20, NPC);
    let (ea, eb) = (NODES + 0x100, NODES + 0x110);
    e.mem.set_u32(ea, FACTION_A);
    e.mem.set_u8(ea + 4, 2);
    e.mem.set_u32(eb, FACTION_B);
    e.mem.set_u8(eb + 4, 0xff);
    e.mem.set_u32(NPC + 0x5c, ea);
    e.mem.set_u32(NPC + 0x60, NODES);
    e.mem.set_u32(NODES, eb);
    e.mem.set_u32(FACTION_A + 0x3c, NODES + 0x200);
    e.mem.set_u32(FACTION_A + 0x40, NODES + 0x20);
    e.mem.set_u32(NODES + 0x20, NODES + 0x210);
    e.mem.set_u32(NODES + 0x24, NODES + 0x28);
    e.mem.set_u32(NODES + 0x28, NODES + 0x220);
    e.mem.set_u32(SCRIPT + 0xc, 0x0500_0d00);
    for (i, f) in [NPC_REF, NPC, FACTION_A, FACTION_B, ITEM]
        .into_iter()
        .enumerate()
    {
        set_script_ref(&mut e, i as u16 + 1, f);
    }
    e
}

fn run(e: &mut Engine, opcode: u32, this: u32, args: &[Vec<u8>]) -> f64 {
    let (ok, result) = call(e, opcode, this, args);
    assert_eq!(ok, 1);
    result
}

const NUM: u32 = 0x15c9;
const NTH: u32 = 0x15ca;
const RANK: u32 = 0x15cb;
const RANKS: u32 = 0x15cc;

/// The count, nth faction (its id in the low word; -2 the last) and nth rank (the byte unsigned)
/// of the actor base given (a reference's base form) or the calling reference's; the index
/// comes first; nothing past the end, for a negative index other than -2, or without an actor
/// base.
#[test]
fn an_actor_bases_factions() {
    let mut e = setup();
    assert_eq!(run(&mut e, NUM, NPC_REF, &[]), 2.0);
    assert_eq!(run(&mut e, NUM, 0, &[r(2)]), 2.0);
    assert_eq!(run(&mut e, NUM, 0, &[]), 0.0);
    assert_eq!(
        run(&mut e, NTH, NPC_REF, &[n(1)]).to_bits() as u32,
        0x0001_0013
    );
    assert_eq!(
        run(&mut e, NTH, 0, &[n(0), r(2)]).to_bits() as u32,
        0x0001_0012
    );
    assert_eq!(
        run(&mut e, NTH, 0, &[n(0), r(1)]),
        0.0,
        "a reference is no actor base: the read stops"
    );
    assert_eq!(
        run(&mut e, NTH, NPC_REF, &[n(-2)]).to_bits() as u32,
        0x0001_0013
    );
    assert_eq!(run(&mut e, NTH, NPC_REF, &[n(2)]), 0.0);
    assert_eq!(run(&mut e, NTH, NPC_REF, &[n(-1)]), 0.0);
    assert_eq!(run(&mut e, RANK, NPC_REF, &[n(0)]), 2.0);
    assert_eq!(
        run(&mut e, RANK, NPC_REF, &[n(1)]),
        255.0,
        "the byte is unsigned"
    );
    assert_eq!(run(&mut e, RANK, NPC_REF, &[n(2)]), 0.0);
    // An entry without a faction answers nothing.
    e.mem.set_u32(NODES + 0x110, 0);
    assert_eq!(run(&mut e, NTH, NPC_REF, &[n(1)]), 0.0);
    assert_eq!(run(&mut e, RANK, NPC_REF, &[n(1)]), 0.0);
    assert_eq!(run(&mut e, NUM, NPC_REF, &[]), 2.0, "the node still counts");
}

/// A faction's count of ranks (every node; 0 for an empty head); a form that is no faction is
/// refused by the read (0).
#[test]
fn a_factions_ranks() {
    let mut e = setup();
    assert_eq!(run(&mut e, RANKS, 0, &[r(3)]), 3.0);
    assert_eq!(run(&mut e, RANKS, 0, &[r(4)]), 0.0);
    assert_eq!(run(&mut e, RANKS, 0, &[r(5)]), 0.0, "an item is no faction");
}
