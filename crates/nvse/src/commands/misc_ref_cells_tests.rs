//! Tests of the cell scans through the commands' records: cells, references and base forms laid
//! out in the test's memory, a worldspace whose cell map holds the cells under xNVSE's keys. None
//! of the batch was compared with the real game (its script, `real-tests/gr_u_scan.txt`, did not
//! compile in run `inv6r1`, and this track has no game runs): unit tests of the source and the
//! DLL's reading only.

use super::super::test_support::*;
use super::*;
use crate::Nvse;

const PC: u32 = 0x0200_0000;
const WORLD: u32 = 0x0200_0100;
const MAP: u32 = 0x0200_0200;
const BUCKETS: u32 = 0x0200_0300;
const ENTRIES: u32 = 0x0200_0400;
const CELLS: u32 = 0x0200_1000;
const REFS: u32 = 0x0200_4000;
const BASES: u32 = 0x0200_9000;
const NODES: u32 = 0x0200_a000;
const SCRIPT_VTBL: u32 = 0x0200_f000;
const EMPTY_TEXT: u32 = 0x0200_f800;
const NAME_FN: u32 = 0x0200_f900;
const CONSOLE: u32 = 0x0200_fa00;
const BUCKET_COUNT: u32 = 7;

thread_local! {
    static PRINTED: std::cell::RefCell<Vec<Vec<u8>>> = const { std::cell::RefCell::new(Vec::new()) };
    static ASKED: std::cell::RefCell<Vec<u32>> = const { std::cell::RefCell::new(Vec::new()) };
}

fn eax(v: u32) -> Ret {
    Ret {
        eax: v,
        ..Ret::default()
    }
}

/// `ConsoleManager::Print`: the format with its first word as `%08x` or `%s` would make it.
fn print_fake(e: &mut Engine, a: &[u32]) -> Ret {
    let format = crate::table::guest_text(e, a[1]);
    let word = e.mem.u32(a[2]);
    let line = if format == b"%s" {
        crate::table::guest_text(e, word)
    } else {
        let f = String::from_utf8_lossy(&format).replace("%08x", &format!("{word:08x}"));
        f.into_bytes()
    };
    PRINTED.with(|p| p.borrow_mut().push(line));
    eax(0)
}

fn printed() -> Vec<String> {
    PRINTED.with(|p| {
        p.borrow_mut()
            .drain(..)
            .map(|l| String::from_utf8_lossy(&l).into_owned())
            .collect()
    })
}

/// The player in no cell yet, a worldspace with an empty map, the script of mod 5 (its name ""),
/// errors printed to a console that is not open.
fn setup() -> Engine {
    let mut e = engine_with_table();
    // The data handler's page (none: the error header names no file).
    e.map(0x011c_3000, 0x1000);
    e.set_global::<u32>(PLAYER, PC);
    e.mem.set_u8(PC + 4, 0x3b);
    e.mem.set_u32(PC + 0xc, 0x14);
    e.mem.set_u32(WORLD + WORLD_CELL_MAP, MAP);
    e.mem.set_u32(MAP + 4, BUCKET_COUNT);
    e.mem.set_u32(MAP + 8, BUCKETS);
    e.mem.set_u32(SCRIPT, SCRIPT_VTBL);
    e.mem.set_u32(SCRIPT + 0xc, 0x0500_0d00);
    e.mem.set_u32(SCRIPT_VTBL + 0x130, NAME_FN);
    e.register(NAME_FN, |_, _| eax(EMPTY_TEXT));
    e.register(0x0071_b160, |_, _| eax(CONSOLE));
    e.register(0x0071_d0a0, print_fake);
    e.ext::<Nvse>().main_thread_recorded = true;
    // `IsInventoryObjectType`: the types asked kept; the item types answer 1.
    e.register(IS_INVENTORY_OBJECT_TYPE, |_, a| {
        ASKED.with(|v| v.borrow_mut().push(a[0]));
        eax(u32::from(matches!(a[0], 0x18..=0x2f)))
    });
    e
}

/// Cell `i` at `(x, y)` of `world` (0: an interior), in the map under xNVSE's key when exterior.
fn cell(e: &mut Engine, i: u32, x: i32, y: i32, world: u32) -> u32 {
    let c = CELLS + 0x200 * i;
    e.mem.set_u8(c + 4, 0x39);
    e.mem.set_u32(c + 0xc, 0x0001_0100 + i);
    e.mem.set_u32(c + CELL_DATA, c + 0x180);
    e.mem.set_u32(c + 0x180, x as u32);
    e.mem.set_u32(c + 0x184, y as u32);
    e.mem.set_u32(c + CELL_WORLD, world);
    if world != 0 {
        let k = key(x, y);
        let entry = ENTRIES + 0x10 * i;
        let bucket = BUCKETS + 4 * (k % BUCKET_COUNT);
        e.mem.set_u32(entry, e.mem.u32(bucket));
        e.mem.set_u32(entry + 4, k);
        e.mem.set_u32(entry + 8, c);
        e.mem.set_u32(bucket, entry);
    }
    c
}

/// Base form `i` of type `kind` (its id `0x00020000 + i`, or `id` when given).
fn base(e: &mut Engine, i: u32, kind: u8, id: Option<u32>) -> u32 {
    let b = BASES + 0x40 * i;
    e.mem.set_u8(b + 4, kind);
    e.mem.set_u32(b + 0xc, id.unwrap_or(0x0002_0000 + i));
    b
}

/// Reference `i` (id `0x00030000 + i`) of `base_form` at `pos`, put at the end of `cell`'s list.
fn reference(e: &mut Engine, i: u32, base_form: u32, cell: u32, pos: [f32; 3]) -> u32 {
    let r = REFS + 0x100 * i;
    e.mem.set_u8(r + 4, 0x3a);
    e.mem.set_u32(r + 0xc, 0x0003_0000 + i);
    e.mem.set_u32(r + REFR_BASE_FORM, base_form);
    e.mem.set_u32(r + REFR_PARENT_CELL, cell);
    for (k, v) in pos.iter().enumerate() {
        e.mem.set_f32(r + REFR_POS + 4 * k as u32, *v);
    }
    set_classes(e, r, 1);
    let mut node = cell + CELL_OBJECTS;
    if e.mem.u32(node) == 0 && e.mem.u32(node + 4) == 0 {
        e.mem.set_u32(node, r);
        return r;
    }
    while e.mem.u32(node + 4) != 0 {
        node = e.mem.u32(node + 4);
    }
    let added = NODES + 8 * i;
    e.mem.set_u32(added, r);
    e.mem.set_u32(node + 4, added);
    r
}

fn run(e: &mut Engine, opcode: u32, this: u32, args: &[Vec<u8>]) -> f64 {
    let (ok, result) = call(e, opcode, this, args);
    assert_eq!(ok, 1);
    result
}

fn id_of(v: f64) -> u32 {
    v.to_bits() as u32
}

const GET_FIRST_REF: u32 = 0x1471;
const GET_NEXT_REF: u32 = 0x1472;
const GET_NUM_REFS: u32 = 0x1473;
const GET_FIRST_REF_IN_CELL: u32 = 0x1474;
const GET_NUM_REFS_IN_CELL: u32 = 0x1475;
const GET_REFS: u32 = 0x15c7;
const GET_REFS_IN_CELL: u32 = 0x15c8;
const GET_IN_GRID: u32 = 0x15ed;
const GET_IN_GRID_IN_CELL: u32 = 0x15ee;

/// An interior with two doors (the second taken), an NPC and the player's own base: the type
/// filter finds the doors in order, passes over the taken one unless asked and never answers the
/// player's base; `GetNextRef` goes on from the last answer and ends with 0.
#[test]
fn first_and_next_refs_by_type() {
    let mut e = setup();
    let room = cell(&mut e, 0, 3, 4, 0);
    e.mem.set_u32(PC + REFR_PARENT_CELL, room);
    let door = base(&mut e, 0, 0x1c, None);
    let npc = base(&mut e, 1, 0x2a, None);
    let player = base(&mut e, 2, 0x2a, Some(7));
    reference(&mut e, 0, door, room, [0.0; 3]);
    reference(&mut e, 1, npc, room, [0.0; 3]);
    let taken = reference(&mut e, 2, door, room, [0.0; 3]);
    e.mem.set_u32(taken + FORM_FLAGS, 0x20);
    reference(&mut e, 3, player, room, [0.0; 3]);
    assert_eq!(id_of(run(&mut e, GET_FIRST_REF, 0, &[n(28)])), 0x0003_0000);
    assert_eq!(id_of(run(&mut e, GET_NEXT_REF, 0, &[])), 0);
    assert_eq!(
        id_of(run(&mut e, GET_FIRST_REF, 0, &[n(28), n(0), n(1)])),
        0x0003_0000
    );
    assert_eq!(id_of(run(&mut e, GET_NEXT_REF, 0, &[])), 0x0003_0002);
    assert_eq!(id_of(run(&mut e, GET_NEXT_REF, 0, &[])), 0);
    // The NPC type: the player's base (id 7) is passed over.
    assert_eq!(id_of(run(&mut e, GET_FIRST_REF, 0, &[n(42)])), 0x0003_0001);
    assert_eq!(id_of(run(&mut e, GET_NEXT_REF, 0, &[])), 0);
    // Any type: everything not taken, in order; the type is a byte (256 + 28 is 28).
    assert_eq!(id_of(run(&mut e, GET_FIRST_REF, 0, &[])), 0x0003_0000);
    assert_eq!(id_of(run(&mut e, GET_NEXT_REF, 0, &[])), 0x0003_0001);
    assert_eq!(id_of(run(&mut e, GET_NEXT_REF, 0, &[])), 0x0003_0003);
    assert_eq!(
        id_of(run(&mut e, GET_FIRST_REF, 0, &[n(256 + 28)])),
        0x0003_0000
    );
    // A scan that ended is dropped: the next GetNextRef has none.
    assert_eq!(id_of(run(&mut e, GET_NEXT_REF, 0, &[])), 0);
    printed();
    assert_eq!(id_of(run(&mut e, GET_NEXT_REF, 0, &[])), 0);
    let lines = printed();
    assert_eq!(lines.len(), 1, "{lines:?}");
    assert!(
        lines[0].ends_with("\nGetFirstRef MUST be called before GetNextRef"),
        "{lines:?}"
    );
    // A wrong-typed argument (a string for the form type) fails the read: 0, no scan.
    assert_eq!(id_of(run(&mut e, GET_FIRST_REF, 0, &[s("x")])), 0);
}

/// The second `GetNextRef` without a scan reads the record the first left unbuilt: xNVSE reads its
/// uninitialised memory and the game stops; without the script reference checks row the port stops.
#[test]
#[should_panic(expected = "never built or has ended")]
fn an_unbuilt_scan_stops_the_game_without_the_reference_checks() {
    let mut e = setup();
    let room = cell(&mut e, 0, 0, 0, 0);
    e.mem.set_u32(PC + REFR_PARENT_CELL, room);
    run(&mut e, GET_NEXT_REF, 0, &[]);
    run(&mut e, GET_NEXT_REF, 0, &[]);
}

/// With the row the unbuilt record answers as the first call did: the error again and 0. A
/// console scan that has ended is kept under the console's key (xNVSE drops the script's own id),
/// and the row answers the same for it.
#[test]
fn unbuilt_and_ended_scans_answer_with_the_reference_checks() {
    let mut e = setup();
    invref::set_checks(&mut e, true);
    let room = cell(&mut e, 0, 0, 0, 0);
    e.mem.set_u32(PC + REFR_PARENT_CELL, room);
    let door = base(&mut e, 0, 0x1c, None);
    reference(&mut e, 0, door, room, [0.0; 3]);
    for _ in 0..3 {
        assert_eq!(id_of(run(&mut e, GET_NEXT_REF, 0, &[])), 0);
    }
    let lines = printed();
    assert_eq!(lines.len(), 3, "{lines:?}");
    // A console line: the console open and running, the script of no mod.
    e.mem.set_u8(0x011d_ea2e, 1);
    let tls = e.tls();
    e.mem.set_u8(tls + 0x268, 1);
    e.mem.set_u8(SCRIPT + 4, 0x11);
    e.mem.set_u32(SCRIPT + 0xc, 0xff00_0001);
    assert_eq!(id_of(run(&mut e, GET_FIRST_REF, 0, &[n(28)])), 0x0003_0000);
    assert_eq!(id_of(run(&mut e, GET_NEXT_REF, 0, &[])), 0);
    assert_eq!(
        printed(),
        ["GetFirstRef >> 00030000", "GetNextRef >> 00000000"],
        "the end prints no error"
    );
    assert!(state(&mut e).scans.contains_key(&CONSOLE_KEY));
    assert_eq!(id_of(run(&mut e, GET_NEXT_REF, 0, &[])), 0);
    let lines = printed();
    assert!(lines[0].contains("In-game console") && lines[0].ends_with("before GetNextRef"));
    assert_eq!(lines[1], "GetNextRef >> 00000000");
    // A new GetFirstRef rebuilds the console's scan.
    assert_eq!(id_of(run(&mut e, GET_FIRST_REF, 0, &[n(28)])), 0x0003_0000);
}

/// The console's ended scan without the row: the next `GetNextRef` reads its missing cell.
#[test]
#[should_panic(expected = "never built or has ended")]
fn an_ended_console_scan_stops_the_game_without_the_reference_checks() {
    let mut e = setup();
    let room = cell(&mut e, 0, 0, 0, 0);
    e.mem.set_u32(PC + REFR_PARENT_CELL, room);
    let door = base(&mut e, 0, 0x1c, None);
    reference(&mut e, 0, door, room, [0.0; 3]);
    e.mem.set_u8(0x011d_ea2e, 1);
    let tls = e.tls();
    e.mem.set_u8(tls + 0x268, 1);
    e.mem.set_u32(SCRIPT + 0xc, 0xff00_0001);
    run(&mut e, GET_FIRST_REF, 0, &[n(28)]);
    run(&mut e, GET_NEXT_REF, 0, &[]);
    run(&mut e, GET_NEXT_REF, 0, &[]);
}

/// The cell map's key is `(x << 16) + y` in 32 bits, so a negative y borrows from x (xNVSE's
/// key, not `(x << 16) | (y & 0xFFFF)`).
#[test]
fn the_cell_keys_are_xnvse_sums() {
    assert_eq!(key(1, 2), 0x0001_0002);
    assert_eq!(key(-1, -1), 0xfffe_ffff);
    assert_eq!(key(0, -1), 0xffff_ffff);
    assert_eq!(key(128, 0), 0x0080_0000);
}

/// An exterior area of depth 1 around (0, 0): the cells row by row from (-1, -1), missing ones
/// passed over; `GetNumRefs` counts and `GetRefs` lists in that order, and `GetNextRef` crosses
/// into the next cell of the area.
#[test]
fn an_area_is_walked_row_by_row() {
    let mut e = setup();
    let centre = cell(&mut e, 0, 0, 0, WORLD);
    let corner = cell(&mut e, 1, -1, -1, WORLD);
    let right = cell(&mut e, 2, 1, 0, WORLD);
    let far = cell(&mut e, 3, 2, 2, WORLD);
    e.mem.set_u32(PC + REFR_PARENT_CELL, centre);
    let misc = base(&mut e, 0, 0x1f, None);
    reference(&mut e, 0, misc, centre, [0.0; 3]);
    reference(&mut e, 1, misc, corner, [0.0; 3]);
    reference(&mut e, 2, misc, right, [0.0; 3]);
    reference(&mut e, 3, misc, far, [0.0; 3]);
    assert_eq!(
        run(&mut e, GET_NUM_REFS, 0, &[n(0)]),
        1.0,
        "no depth: the cell"
    );
    assert_eq!(run(&mut e, GET_NUM_REFS, 0, &[n(0), n(1)]), 3.0);
    assert_eq!(run(&mut e, GET_NUM_REFS, 0, &[n(0), n(2)]), 4.0);
    let array = run(&mut e, GET_REFS, 0, &[n(31), n(1)]) as u32;
    let ids: Vec<u32> = (0..3)
        .map(
            |i| match vars::get_element(&mut e, array, &Key::Number(f64::from(i))) {
                Some(vars::Element {
                    data: vars::Data::Form(id),
                    ..
                }) => id,
                other => panic!("{other:?}"),
            },
        )
        .collect();
    assert_eq!(ids, [0x0003_0001, 0x0003_0000, 0x0003_0002]);
    assert_eq!(vars::array(&e, array).unwrap().len(), 3);
    assert_eq!(
        id_of(run(&mut e, GET_FIRST_REF, 0, &[n(31), n(1)])),
        0x0003_0001
    );
    assert_eq!(id_of(run(&mut e, GET_NEXT_REF, 0, &[])), 0x0003_0000);
    assert_eq!(id_of(run(&mut e, GET_NEXT_REF, 0, &[])), 0x0003_0002);
    assert_eq!(id_of(run(&mut e, GET_NEXT_REF, 0, &[])), 0);
    // The InCell forms take the cell first: the far cell alone, or with depth 1 around it.
    set_script_ref(&mut e, 1, far);
    assert_eq!(run(&mut e, GET_NUM_REFS_IN_CELL, 0, &[r(1), n(0)]), 1.0);
    assert_eq!(
        run(&mut e, GET_NUM_REFS_IN_CELL, 0, &[r(1), n(0), n(1)]),
        1.0
    );
    assert_eq!(
        id_of(run(&mut e, GET_FIRST_REF_IN_CELL, 0, &[r(1), n(31)])),
        0x0003_0003
    );
    let array = run(&mut e, GET_REFS_IN_CELL, 0, &[r(1), n(0), n(3)]) as u32;
    assert_eq!(vars::array(&e, array).unwrap().len(), 4);
    // A reference in the cell slot is no cell: the read fails, 0 and an empty array.
    let not_a_cell = REFS + 0x100;
    set_script_ref(&mut e, 2, not_a_cell);
    assert_eq!(run(&mut e, GET_NUM_REFS_IN_CELL, 0, &[r(2), n(0)]), 0.0);
    let array = run(&mut e, GET_REFS_IN_CELL, 0, &[r(2)]) as u32;
    assert_ne!(array, 0);
    assert_eq!(vars::array(&e, array).unwrap().len(), 0);
}

/// Depth -1 is `uGridsToLoad:General` (an INI setting, here 2), -127 or none is 0.
#[test]
fn depth_minus_one_is_the_grid_setting() {
    let mut e = setup();
    e.register(0x00ec_a7f4, |_, a| {
        eax(u32::from((a[0] as u8).to_ascii_uppercase()))
    });
    e.map(0x011f_3000, 0x1000);
    e.map(0x011f_9000, 0x1000);
    let (coll, setting, name) = (0x0200_e000, 0x0200_e200, 0x0200_e300);
    e.mem.set_u32(0x011f_35a0, coll);
    e.mem.set_u32(coll + 0x10c, setting);
    e.mem.set_u32(setting + 4, 2);
    e.mem.set_u32(setting + 8, name);
    e.mem.write(name, b"uGridsToLoad:General\0");
    let centre = cell(&mut e, 0, 0, 0, WORLD);
    let far = cell(&mut e, 1, 2, 2, WORLD);
    e.mem.set_u32(PC + REFR_PARENT_CELL, centre);
    let misc = base(&mut e, 0, 0x1f, None);
    reference(&mut e, 0, misc, centre, [0.0; 3]);
    reference(&mut e, 1, misc, far, [0.0; 3]);
    assert_eq!(run(&mut e, GET_NUM_REFS, 0, &[n(0), n(-1)]), 2.0);
    assert_eq!(run(&mut e, GET_NUM_REFS, 0, &[n(0), n(-127)]), 1.0);
    assert_eq!(run(&mut e, GET_NUM_REFS, 0, &[n(0), n(1)]), 1.0);
}

/// A depth that takes the area's far edge past 127 (x 120, depth 10): the byte never meets the end
/// test. Read from xNVSE's source and the DLL (`NextCell` at `100369C0`: `movsx` of the byte minus
/// the 32-bit x, compared with the depth); the hang itself was not run in the real game. With the
/// list guard row the walk ends where it started: the first row's cells each counted once (the
/// row's x goes round all 256 values), the centre's row never reached.
#[test]
fn a_walk_past_the_grid_ends_with_the_list_guard() {
    let mut e = setup();
    super::super::array::set_fix(&mut e, super::super::array::LIST_GUARD, true);
    let centre = cell(&mut e, 0, 120, 0, WORLD);
    let first_row = cell(&mut e, 1, 125, -10, WORLD);
    let wrapped = cell(&mut e, 2, -100, -10, WORLD);
    e.mem.set_u32(PC + REFR_PARENT_CELL, centre);
    let misc = base(&mut e, 0, 0x1f, None);
    reference(&mut e, 0, misc, centre, [0.0; 3]);
    reference(&mut e, 1, misc, first_row, [0.0; 3]);
    reference(&mut e, 2, misc, wrapped, [0.0; 3]);
    assert_eq!(run(&mut e, GET_NUM_REFS, 0, &[n(0), n(10)]), 2.0);
    assert_eq!(
        id_of(run(&mut e, GET_FIRST_REF, 0, &[n(0), n(10)])),
        0x0003_0001
    );
    // Each GetNextRef walks only until it finds one: round the wrapped row again and again, as
    // xNVSE answers (these calls never reach the guard).
    for expected in [0x0003_0002, 0x0003_0001, 0x0003_0002, 0x0003_0001] {
        assert_eq!(id_of(run(&mut e, GET_NEXT_REF, 0, &[])), expected);
    }
    // Doors are on no cell of that row: one call goes round without a match, the hang, ended.
    assert_eq!(id_of(run(&mut e, GET_FIRST_REF, 0, &[n(28), n(10)])), 0);
    assert!(state(&mut e).scans.is_empty());
    // A depth that ends (x 100, depth 10) never reaches the guard: walked as xNVSE walks it.
    let ends = cell(&mut e, 3, 100, 0, WORLD);
    let mut scan = Scan::new(&mut e, 10, 0, false, ends).unwrap();
    let mut found = Vec::new();
    while scan.next_cell(&mut e) {
        found.push(scan.cur_cell);
    }
    assert_eq!(
        found,
        [ends],
        "the only cell of that area in the map: the centre"
    );
}

#[test]
#[should_panic(expected = "never ends")]
fn a_walk_past_the_grid_stops_the_game_without_the_list_guard() {
    let mut e = setup();
    let centre = cell(&mut e, 0, 120, 0, WORLD);
    e.mem.set_u32(PC + REFR_PARENT_CELL, centre);
    run(&mut e, GET_NUM_REFS, 0, &[n(0), n(10)]);
}

/// The walk's bytes: a step from x 127 keeps -128 but looks up 128 (the DLL's 32-bit sum).
#[test]
fn a_step_from_127_looks_up_128() {
    let mut e = setup();
    let centre = cell(&mut e, 0, 127, 0, WORLD);
    let at_128 = cell(&mut e, 1, 128, -1, WORLD);
    let mut scan = Scan::new(&mut e, 1, 0, false, centre).unwrap();
    assert_eq!((scan.cur_x, scan.cur_y), (126, -1));
    // (127, -1) is not mapped: the step from 127 looks up key(128, -1).
    assert!(scan.next_cell(&mut e));
    assert_eq!(scan.cur_cell, at_128);
    assert_eq!((scan.cur_x, scan.cur_y), (-128, -1));
}

/// The distance and angle filters measure from the calling reference (itself always counted):
/// at most 200 units, at most 45 degrees either side of its heading; a base form narrows.
#[test]
fn distance_angle_and_base_form_filters() {
    let mut e = setup();
    let room = cell(&mut e, 0, 0, 0, 0);
    e.mem.set_u32(PC + REFR_PARENT_CELL, room);
    let misc = base(&mut e, 0, 0x1f, None);
    let other = base(&mut e, 1, 0x1f, None);
    let me = reference(&mut e, 0, misc, room, [0.0; 3]);
    reference(&mut e, 1, misc, room, [0.0, 100.0, 0.0]);
    reference(&mut e, 2, misc, room, [100.0, 0.0, 0.0]);
    reference(&mut e, 3, other, room, [0.0, 300.0, 0.0]);
    let count = |e: &mut Engine, args: &[Vec<u8>]| run(e, GET_NUM_REFS, me, args);
    assert_eq!(count(&mut e, &[]), 4.0);
    assert_eq!(count(&mut e, &[n(0), n(0), n(0), z(200.0)]), 3.0);
    assert_eq!(count(&mut e, &[n(0), n(0), n(0), z(0.0), z(45.0)]), 3.0);
    assert_eq!(count(&mut e, &[n(0), n(0), n(0), z(200.0), z(45.0)]), 2.0);
    set_script_ref(&mut e, 1, other);
    assert_eq!(
        count(&mut e, &[n(0), n(0), n(0), z(0.0), z(-1.0), r(1)]),
        1.0
    );
    // Without a calling reference no distance is measured.
    assert_eq!(
        run(&mut e, GET_NUM_REFS, 0, &[n(0), n(0), n(0), z(1.0)]),
        4.0
    );
    assert_eq!(
        heading_angle(&e, me, REFS + 0x200),
        f64::from(std::f64::consts::FRAC_PI_2 as f32) * 57.295_779_513_082_32,
        "the atan2 rounded to a float first"
    );
    e.mem.set_f32(me + REFR_ROT_Z, 3.0);
    let a = heading_angle(&e, me, REFS + 0x200);
    assert!((-180.0..=180.0).contains(&a));
    assert!((a - (90.0 - 3.0f64.to_degrees())).abs() < 1e-4, "{a}");
}

/// The actor filter (200): creatures and NPCs but not the player's base, taken or not; the item
/// filter (201): the item types and lights that can be carried, not taken unless asked.
#[test]
fn actor_and_item_filters() {
    let mut e = setup();
    let room = cell(&mut e, 0, 0, 0, 0);
    e.mem.set_u32(PC + REFR_PARENT_CELL, room);
    let creature = base(&mut e, 0, 0x2b, None);
    let npc = base(&mut e, 1, 0x2a, None);
    let player = base(&mut e, 2, 0x2a, Some(7));
    let weapon = base(&mut e, 3, 0x28, None);
    let torch = base(&mut e, 4, 0x1e, None);
    let lamp = base(&mut e, 5, 0x1e, None);
    let door = base(&mut e, 6, 0x1c, None);
    set_classes(&mut e, torch, 1 << 34);
    set_classes(&mut e, lamp, 1 << 34);
    e.mem.set_u32(torch + LIGHT_FLAGS, 2);
    let c = reference(&mut e, 0, creature, room, [0.0; 3]);
    e.mem.set_u32(c + FORM_FLAGS, 0x2);
    reference(&mut e, 1, npc, room, [0.0; 3]);
    reference(&mut e, 2, player, room, [0.0; 3]);
    let w = reference(&mut e, 3, weapon, room, [0.0; 3]);
    reference(&mut e, 4, torch, room, [0.0; 3]);
    reference(&mut e, 5, lamp, room, [0.0; 3]);
    reference(&mut e, 6, door, room, [0.0; 3]);
    assert_eq!(
        run(&mut e, GET_NUM_REFS, 0, &[n(200)]),
        2.0,
        "a taken creature counts"
    );
    assert_eq!(run(&mut e, GET_NUM_REFS, 0, &[n(201)]), 2.0);
    e.mem.set_u32(w + FORM_FLAGS, 0x20);
    assert_eq!(run(&mut e, GET_NUM_REFS, 0, &[n(201)]), 1.0);
    assert_eq!(run(&mut e, GET_NUM_REFS, 0, &[n(201), n(0), n(1)]), 2.0);
    assert_eq!(id_of(run(&mut e, GET_FIRST_REF, 0, &[n(201)])), 0x0003_0004);
    // 202 and 203 are no filter of `GetNumRefs`: types nothing has.
    assert_eq!(run(&mut e, GET_NUM_REFS, 0, &[n(202)]), 0.0);
    assert_eq!(run(&mut e, GET_NUM_REFS, 0, &[n(203)]), 0.0);
}

/// `GetInGrid`: 1 when the reference is in the area. The taken test asks the type of the
/// reference itself (not its base's), which is no inventory type, so a taken reference is found.
/// No reference given: 0. `GetInGridInCell` takes the cell first.
#[test]
fn in_grid_looks_for_one_reference() {
    let mut e = setup();
    let room = cell(&mut e, 0, 0, 0, 0);
    let other = cell(&mut e, 1, 0, 0, 0);
    e.mem.set_u32(PC + REFR_PARENT_CELL, room);
    let weapon = base(&mut e, 0, 0x28, None);
    reference(&mut e, 0, weapon, room, [0.0; 3]);
    let taken = reference(&mut e, 1, weapon, room, [0.0; 3]);
    let away = reference(&mut e, 2, weapon, other, [0.0; 3]);
    e.mem.set_u32(taken + FORM_FLAGS, 0x20);
    set_script_ref(&mut e, 1, taken);
    set_script_ref(&mut e, 2, away);
    set_script_ref(&mut e, 3, other);
    ASKED.with(|v| v.borrow_mut().clear());
    assert_eq!(run(&mut e, GET_IN_GRID, 0, &[r(1)]), 1.0);
    assert!(ASKED.with(|v| v.borrow().iter().all(|&t| t == 0x3a)));
    assert_eq!(run(&mut e, GET_IN_GRID, 0, &[r(2)]), 0.0);
    assert_eq!(run(&mut e, GET_IN_GRID, 0, &[]), 0.0);
    assert_eq!(run(&mut e, GET_IN_GRID_IN_CELL, 0, &[r(3), r(2)]), 1.0);
    assert_eq!(run(&mut e, GET_IN_GRID_IN_CELL, 0, &[r(3), r(1)]), 0.0);
    // Its scan is kept as GetFirstRef's: GetNextRef goes on looking for the same reference.
    assert_eq!(run(&mut e, GET_IN_GRID, 0, &[r(1)]), 1.0);
    assert_eq!(id_of(run(&mut e, GET_NEXT_REF, 0, &[])), 0);
}

/// Without the player the scans answer nothing; with the player in no cell `GetFirstRef` answers 0.
#[test]
fn no_player_or_no_cell() {
    let mut e = setup();
    assert_eq!(run(&mut e, GET_FIRST_REF, 0, &[]), 0.0);
    assert_eq!(run(&mut e, GET_NUM_REFS, 0, &[]), 0.0);
    e.set_global::<u32>(PLAYER, 0);
    assert_eq!(run(&mut e, GET_FIRST_REF, 0, &[]), 0.0);
    assert_eq!(run(&mut e, GET_IN_GRID, 0, &[]), 0.0);
    let array = run(&mut e, GET_REFS, 0, &[]) as u32;
    assert_eq!(vars::array(&e, array).unwrap().len(), 0);
}

/// A cell reference without a base form: the type filters read its type through nothing; with the
/// reference checks row it is passed over.
#[test]
fn a_reference_without_a_base_form() {
    let mut e = setup();
    let room = cell(&mut e, 0, 0, 0, 0);
    e.mem.set_u32(PC + REFR_PARENT_CELL, room);
    reference(&mut e, 0, 0, room, [0.0; 3]);
    let misc = base(&mut e, 0, 0x1f, None);
    reference(&mut e, 1, misc, room, [0.0; 3]);
    assert_eq!(
        run(&mut e, GET_NUM_REFS, 0, &[n(0)]),
        2.0,
        "any type reads no base"
    );
    let stop = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        run(&mut e, GET_NUM_REFS, 0, &[n(31)])
    }));
    assert!(stop.is_err());
    let mut e = setup();
    invref::set_checks(&mut e, true);
    let room = cell(&mut e, 0, 0, 0, 0);
    e.mem.set_u32(PC + REFR_PARENT_CELL, room);
    reference(&mut e, 0, 0, room, [0.0; 3]);
    let misc = base(&mut e, 0, 0x1f, None);
    reference(&mut e, 1, misc, room, [0.0; 3]);
    assert_eq!(run(&mut e, GET_NUM_REFS, 0, &[n(31)]), 1.0);
    assert_eq!(run(&mut e, GET_NUM_REFS, 0, &[n(200)]), 0.0);
}

/// An exterior scan whose walk finds no cell of its area in the map (the centre itself not there):
/// xNVSE reads the missing cell's list and the game stops; with the reference checks row the scan
/// answers 0 and is dropped.
#[test]
fn a_walk_that_finds_no_cell() {
    for checks in [false, true] {
        let mut e = setup();
        invref::set_checks(&mut e, checks);
        let centre = cell(&mut e, 0, 5, 5, 0);
        e.mem.set_u32(centre + CELL_WORLD, WORLD);
        e.mem.set_u32(PC + REFR_PARENT_CELL, centre);
        let first = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            run(&mut e, GET_FIRST_REF, 0, &[n(0), n(1)])
        }));
        if checks {
            assert_eq!(first.unwrap(), 0.0);
            assert!(state(&mut e).scans.is_empty());
            assert_eq!(run(&mut e, GET_NUM_REFS, 0, &[n(0), n(1)]), 0.0);
        } else {
            assert!(first.is_err());
        }
    }
}

/// The stepped scan's hang without the list guard: GetFirstRef for a type no cell of the wrapped
/// row holds goes round and round in one call (read from the DLL's walk; the hang was not run).
#[test]
#[should_panic(expected = "never ends")]
fn a_stepped_walk_past_the_grid_without_a_match_stops_without_the_list_guard() {
    let mut e = setup();
    let centre = cell(&mut e, 0, 120, 0, WORLD);
    let first_row = cell(&mut e, 1, 125, -10, WORLD);
    e.mem.set_u32(PC + REFR_PARENT_CELL, centre);
    let misc = base(&mut e, 0, 0x1f, None);
    reference(&mut e, 0, misc, first_row, [0.0; 3]);
    assert_eq!(
        id_of(run(&mut e, GET_FIRST_REF, 0, &[n(31), n(10)])),
        0x0003_0000
    );
    assert_eq!(
        id_of(run(&mut e, GET_NEXT_REF, 0, &[])),
        0x0003_0000,
        "round again"
    );
    run(&mut e, GET_FIRST_REF, 0, &[n(28), n(10)]);
}

/// With the list guard alone (no reference checks), a walk past the grid that finds no cell at all
/// answers 0 instead of stopping.
#[test]
fn a_guard_ended_walk_without_cells_answers_with_the_list_guard_alone() {
    let mut e = setup();
    super::super::array::set_fix(&mut e, super::super::array::LIST_GUARD, true);
    let centre = cell(&mut e, 0, 120, 0, 0);
    e.mem.set_u32(centre + CELL_WORLD, WORLD);
    e.mem.set_u32(PC + REFR_PARENT_CELL, centre);
    assert_eq!(run(&mut e, GET_FIRST_REF, 0, &[n(0), n(10)]), 0.0);
    assert_eq!(run(&mut e, GET_NUM_REFS, 0, &[n(0), n(10)]), 0.0);
}
