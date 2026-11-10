//! Tests of the batch through the commands' records, on references and forms laid out where the
//! test's cast puts their components and extra data lists answered by a stand-in of
//! `BaseExtraList::GetByType` (a list's data of type `t` at `+0x100 + 4t`).

use super::super::test_support::*;
use super::*;

const DOOR: u32 = 0x0200_1000;
const DOOR_BASE: u32 = 0x0200_1800;
const OTHER: u32 = 0x0200_2000;
const NPC: u32 = 0x0200_3000;
const KEY: u32 = 0x0200_3800;
const CELL: u32 = 0x0200_4000;
const WORLD: u32 = 0x0200_4800;
const EXTRA: u32 = 0x0200_5000;
const HEAP: u32 = 0x0300_c000;

fn eax(v: u32) -> Ret {
    Ret {
        eax: v,
        ..Ret::default()
    }
}

fn get_by_type(e: &mut Engine, a: &[u32]) -> Ret {
    eax(e.mem.u32(a[0] + 0x100 + 4 * a[1]))
}

fn put_extra(e: &mut Engine, list: u32, kind: u32, data: u32) {
    e.mem.set_u32(list + 0x100 + 4 * kind, data);
}

/// A door reference (base a door, in `CELL` of `WORLD`, persistent) and another persistent
/// reference `OTHER` at (10, 20, 30) turned 1.5 radians, an NPC base (an actor base, flags
/// 0x12345678), a key; the script's references 1-4 naming the door, `OTHER`, the NPC and the key.
fn setup() -> Engine {
    let mut e = engine_with_table();
    e.register(0x0041_0220, get_by_type);
    e.register(0x0040_1000, |e, a| {
        let at = e.mem.u32(HEAP).max(HEAP + 0x400);
        e.mem.set_u32(HEAP, at + a[0].next_multiple_of(0x400));
        eax(at)
    });
    e.register(0x0040_1030, |_, _| Ret::default());
    e.register(0x0040_ff60, |e, a| {
        let kind = u32::from(e.mem.u8(a[1] + 4));
        put_extra(e, a[0], kind, a[1]);
        Ret::default()
    });
    e.register(0x0041_0020, |e, a| {
        let kind = u32::from(e.mem.u8(a[1] + 4));
        put_extra(e, a[0], kind, 0);
        Ret::default()
    });
    for (form, kind, id) in [
        (DOOR, 0x3a, 0x10),
        (DOOR_BASE, 0x1c, 0x11),
        (OTHER, 0x3a, 0x12),
        (NPC, 0x2a, 0x13),
        (KEY, 0x2e, 0x14),
        (CELL, 0x39, 0x15),
        (WORLD, 0x41, 0x16),
    ] {
        e.mem.set_u8(form + 4, kind);
        e.mem.set_u32(form + 0xc, 0x0001_0000 + id);
    }
    e.mem.set_u32(DOOR + 0x7f0, 1);
    e.mem.set_u32(DOOR + 0x20, DOOR_BASE);
    e.mem.set_u32(DOOR + 0x40, CELL);
    e.mem.set_u32(DOOR + 8, 0x400);
    e.mem.set_u32(OTHER + 0x7f0, 1);
    e.mem.set_u32(OTHER + 8, 0x400);
    e.mem.set_u32(OTHER + 0x20, NPC);
    e.mem.set_f32(OTHER + 0x2c, 1.5);
    for (i, v) in [10.0f32, 20.0, 30.0].into_iter().enumerate() {
        e.mem.set_f32(OTHER + 0x30 + 4 * i as u32, v);
    }
    e.mem.set_u32(NPC + 0x7f0, 1 << 26);
    e.mem.set_u32(NPC + 0x34, 0x1234_5678);
    e.mem.set_u32(NPC + 8, 0xabcd_0001);
    e.mem.set_u32(KEY + 0x7f0, 1 << 27);
    e.mem.set_u32(CELL + 0xc0, WORLD);
    for (i, form) in [DOOR, OTHER, NPC, KEY].into_iter().enumerate() {
        set_script_ref(&mut e, i as u16 + 1, form);
    }
    e
}

fn run(e: &mut Engine, opcode: u32, this: u32, args: &[Vec<u8>]) -> f64 {
    let (ok, result) = call(e, opcode, this, args);
    assert_eq!(ok, 1);
    result
}

fn form_of(v: f64) -> u32 {
    v.to_bits() as u32
}

/// The base form, the persistent flag, the parent cell and worldspace (the persistent cell's
/// first), `GetBaseForm`'s leveled creature for a form made in game.
#[test]
fn base_forms_cells_and_persistence() {
    let mut e = setup();
    assert_eq!(form_of(run(&mut e, 0x1403, DOOR, &[])), 0x0001_0011);
    assert_eq!(run(&mut e, 0x1403, 0, &[]), 0.0);
    assert_eq!(run(&mut e, 0x146c, DOOR, &[]), 1.0);
    e.mem.set_u32(DOOR + 8, 0);
    assert_eq!(run(&mut e, 0x146c, DOOR, &[]), 0.0);
    assert_eq!(form_of(run(&mut e, 0x146d, DOOR, &[])), 0x0001_0015);
    assert_eq!(form_of(run(&mut e, 0x146e, DOOR, &[])), 0x0001_0016);
    e.mem.set_u32(CELL + 0xc0, 0);
    assert_eq!(run(&mut e, 0x146e, DOOR, &[]), 0.0, "an interior");
    put_extra(&mut e, DOOR + 0x44, 0x0c, EXTRA);
    e.mem.set_u32(EXTRA + 0xc, CELL + 0x400);
    e.mem.set_u32(CELL + 0x400 + 0xc0, WORLD);
    assert_eq!(form_of(run(&mut e, 0x146e, DOOR, &[])), 0x0001_0016);
    assert_eq!(form_of(run(&mut e, 0x1510, DOOR, &[])), 0x0001_0011);
    e.mem.set_u32(DOOR_BASE + 0xc, 0xff00_0001);
    put_extra(&mut e, DOOR + 0x44, 0x2e, EXTRA + 0x40);
    e.mem.set_u32(EXTRA + 0x4c, NPC);
    assert_eq!(form_of(run(&mut e, 0x1510, DOOR, &[])), 0x0001_0013);
}

/// A door's teleport: the linked door and its cell, a load door, the coordinates and the rotation
/// in degrees; `SetDoorTeleport` makes the teleport with the given place, or the linked
/// reference's (the rotation given in degrees), and refuses a non-persistent target.
#[test]
fn door_teleports() {
    let mut e = setup();
    assert_eq!(run(&mut e, 0x15f9, DOOR, &[]), 0.0);
    assert_eq!(run(&mut e, 0x15fe, DOOR, &[r(2)]), 1.0);
    let tele = e.mem.u32(DOOR + 0x44 + 0x100 + 4 * 0x2b);
    assert_eq!(e.mem.u32(tele), 0x0101_58a8);
    assert_eq!(run(&mut e, 0x15f9, DOOR, &[]), 1.0);
    assert_eq!(form_of(run(&mut e, 0x1470, DOOR, &[])), 0x0001_0012);
    assert_eq!(
        run(&mut e, 0x146f, DOOR, &[]),
        0.0,
        "the other's cell is not loaded"
    );
    e.mem.set_u32(OTHER + 0x40, CELL);
    assert_eq!(form_of(run(&mut e, 0x146f, DOOR, &[])), 0x0001_0015);
    assert_eq!(run(&mut e, 0x15fa, DOOR, &[]), 10.0);
    assert_eq!(run(&mut e, 0x15fc, DOOR, &[]), 30.0);
    assert_eq!(
        run(&mut e, 0x15fd, DOOR, &[]),
        f64::from(1.5f32 * 57.295_78)
    );
    run(
        &mut e,
        0x15fe,
        DOOR,
        &[r(2), z(1.0), z(2.0), z(3.0), z(90.0)],
    );
    assert_eq!(run(&mut e, 0x15fb, DOOR, &[]), 2.0);
    let data = e.mem.u32(tele + 0xc);
    assert_eq!(e.mem.f32(data + 0x18), 90.0 * 0.017_453_292);
    assert_eq!(run(&mut e, 0x15fa, OTHER, &[]), 0.0, "not a door");
    e.mem.set_u32(OTHER + 8, 0);
    assert_eq!(run(&mut e, 0x15fe, DOOR, &[r(2)]), 0.0);
}

/// `SetOpenKey` makes the lock (zeroed data) and sets a key, refusing another form;
/// `GetOpenKey` reads it; `ClearOpenKey` removes the lock.
#[test]
fn open_keys() {
    let mut e = setup();
    assert_eq!(run(&mut e, 0x14c8, DOOR, &[]), 0.0);
    assert_eq!(run(&mut e, 0x1512, DOOR, &[r(3)]), 0.0, "not a key");
    assert_eq!(run(&mut e, 0x1512, DOOR, &[r(4)]), 1.0);
    let lock = e.mem.u32(DOOR + 0x44 + 0x100 + 4 * 0x2a);
    assert_eq!(e.mem.u32(lock), 0x0101_589c);
    assert_eq!(e.mem.u32(e.mem.u32(lock + 0xc)), 0, "the lock level");
    assert_eq!(form_of(run(&mut e, 0x14c8, DOOR, &[])), 0x0001_0014);
    assert_eq!(run(&mut e, 0x151b, DOOR, &[]), 1.0);
    assert_eq!(run(&mut e, 0x14c8, DOOR, &[]), 0.0);
    assert_eq!(run(&mut e, 0x151b, DOOR, &[]), 0.0);
}

/// Owners and ranks of the reference and of its cell; `SetOwningFactionRequiredRank` adds an
/// `ExtraRank` (none for 0) and answers the rank.
#[test]
fn owners_and_ranks() {
    let mut e = setup();
    assert_eq!(run(&mut e, 0x14d7, DOOR, &[]), 0.0);
    put_extra(&mut e, DOOR + 0x44, 0x21, EXTRA);
    e.mem.set_u32(EXTRA + 0xc, NPC);
    put_extra(&mut e, CELL + 0x28, 0x21, EXTRA + 0x40);
    e.mem.set_u32(EXTRA + 0x4c, KEY);
    put_extra(&mut e, CELL + 0x28, 0x23, EXTRA + 0x80);
    e.mem.set_u32(EXTRA + 0x8c, 4);
    assert_eq!(form_of(run(&mut e, 0x14d7, DOOR, &[])), 0x0001_0013);
    assert_eq!(form_of(run(&mut e, 0x14df, DOOR, &[])), 0x0001_0014);
    assert_eq!(run(&mut e, 0x14e1, DOOR, &[]), 4.0);
    assert_eq!(run(&mut e, 0x14e0, DOOR, &[]), 0.0);
    assert_eq!(run(&mut e, 0x14f9, DOOR, &[n(3)]), 3.0);
    assert_eq!(run(&mut e, 0x14e0, DOOR, &[]), 3.0);
    let rank = e.mem.u32(DOOR + 0x44 + 0x100 + 4 * 0x23);
    assert_eq!(e.mem.u32(rank), 0x0101_58cc);
    assert_eq!(run(&mut e, 0x14f9, DOOR, &[n(0)]), 0.0);
    assert_eq!(
        e.mem.u32(DOOR + 0x44 + 0x100 + 4 * 0x23),
        rank,
        "0 adds nothing"
    );
}

/// The actor base flag halves (the argument, else the caller's base as an actor base) and the
/// form flag halves (the argument, else the calling reference itself; `SetFlagsHigh` without one
/// writes the caller's actor base).
#[test]
fn flag_words() {
    let mut e = setup();
    assert_eq!(run(&mut e, 0x14ec, 0, &[r(3)]), f64::from(0x5678));
    assert_eq!(run(&mut e, 0x14ee, OTHER, &[]), f64::from(0x1234));
    run(&mut e, 0x14ed, OTHER, &[n(0x0001_9999)]);
    assert_eq!(e.mem.u32(NPC + 0x34), 0x1234_9999);
    run(&mut e, 0x14ef, 0, &[n(0x7777), r(3)]);
    assert_eq!(e.mem.u32(NPC + 0x34), 0x7777_9999);
    assert_eq!(run(&mut e, 0x14ec, DOOR, &[]), 0.0, "not an actor");
    assert_eq!(run(&mut e, 0x15f0, OTHER, &[]), f64::from(0x400));
    assert_eq!(run(&mut e, 0x15f2, 0, &[r(3)]), f64::from(0xabcd));
    run(&mut e, 0x15f1, OTHER, &[n(0x21)]);
    assert_eq!(e.mem.u32(OTHER + 8), 0x21);
    run(&mut e, 0x15f3, OTHER, &[n(0x0102)]);
    assert_eq!(e.mem.u32(NPC + 8), 0x0102_0001, "the caller's actor base");
    assert_eq!(e.mem.u32(OTHER + 8), 0x21);
    run(&mut e, 0x15f3, 0, &[n(0x0304), r(2)]);
    assert_eq!(e.mem.u32(OTHER + 8), 0x0304_0021);
}
