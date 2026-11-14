//! Tests of `ln_fn_game_data` on a data handler laid out as JIP's header gives it (bound objects
//! +4, the type lists from +8, worldspaces +0x10, regions +0x1D8, the cell array +0x1DC, the
//! file slots +0x21C), the game's form map, `TES`'s grid (+8: size +0xC, cells +0x10) and the
//! settings collections the core track reads.

use super::*;
use crate::commands::test_support::*;
use nvse::vars::Data;

const MOD_A: u32 = FORMS + 0x1_0000;
const MOD_B: u32 = FORMS + 0x1_1000;
const LIST: u32 = FORMS + 0x1000;
const WEAP1: u32 = FORMS + 0x1100;
const WEAP2: u32 = FORMS + 0x1200;
const GLOB1: u32 = FORMS + 0x1300;
const GLOB2: u32 = FORMS + 0x1400;
const CELL1: u32 = FORMS + 0x1500;
const WS: u32 = FORMS + 0x1600;
const WS_CELL: u32 = FORMS + 0x1700;
const REF1: u32 = FORMS + 0x1800;
const REF2: u32 = FORMS + 0x1900;
const REF3: u32 = FORMS + 0x1a00;
const KEYF: u32 = FORMS + 0x1b00;
const NODES: u32 = FORMS + 0x4000;
const BOUND_HEAD: u32 = FORMS + 0x5000;
const CELLS: u32 = FORMS + 0x5100;
const TES_AT: u32 = FORMS + 0x5200;
const GRID: u32 = FORMS + 0x5300;
const GRID_CELLS: u32 = FORMS + 0x5400;
const GRID_CELL: u32 = FORMS + 0x5500;

fn node(e: &mut Engine, at: u32, data: u32, next: u32) {
    e.mem.set_u32(at, data);
    e.mem.set_u32(at + 4, next);
}

fn setup() -> Engine {
    let mut e = engine();
    extra_stand_ins(&mut e);
    game_globals(&mut e);
    loaded_mod(&mut e, 0, MOD_A, b"FalloutNV.esm");
    loaded_mod(&mut e, 1, MOD_B, b"Mod.Two.esp");
    let dh = DATA_HANDLER_AT;
    form(&mut e, LIST, 0x55, 0x0100_0001);
    form(&mut e, WEAP1, 0x28, 0x0000_0002);
    form(&mut e, WEAP2, 0x28, 0x0100_0003);
    form(&mut e, GLOB1, 0x06, 0x0000_0004);
    form(&mut e, GLOB2, 0x06, 0x0100_0005);
    form(&mut e, CELL1, 0x39, 0x0100_0006);
    form(&mut e, WS, 0x41, 0x0000_0007);
    form(&mut e, WS_CELL, 0x39, 0x0000_0008);
    form(&mut e, REF1, 0x3a, 0x0000_0009);
    form(&mut e, REF2, 0x3a, 0x0100_000a);
    form(&mut e, REF3, 0x3a, 0x0100_000b);
    form(&mut e, KEYF, 0x38, 0x0000_000c);
    for (i, f) in [LIST, WEAP1, WEAP2, GLOB1, GLOB2, CELL1, WS, KEYF]
        .iter()
        .enumerate()
    {
        let id = e.mem.u32(f + 0xc);
        in_form_map(&mut e, *f, id, i as u32);
    }
    // Bound objects: WEAP1 then WEAP2 (next at +0x20).
    e.mem.set_u32(dh + 4, BOUND_HEAD);
    e.mem.set_u32(BOUND_HEAD + 4, WEAP1);
    e.mem.set_u32(WEAP1 + 0x20, WEAP2);
    // The globals' list: `kTypeListJmpTbl[6]` = 28, at +8 + 28 * 8 (`globalList` +0xE8).
    node(&mut e, dh + 0xe8, GLOB1, NODES);
    node(&mut e, NODES, GLOB2, 0);
    // One interior cell (and a null slot) holding REF1 (of WEAP1) and REF2 (of GLOB1).
    e.mem.set_u32(dh + 0x1dc + 4, CELLS);
    e.mem.set_u16(dh + 0x1dc + 0xa, 2);
    e.mem.set_u32(CELLS + 4, CELL1);
    node(&mut e, CELL1 + 0xac, REF1, NODES + 8);
    node(&mut e, NODES + 8, REF2, 0);
    e.mem.set_u32(REF1 + 0x20, WEAP1);
    e.mem.set_u32(REF2 + 0x20, GLOB1);
    // One worldspace whose persistent cell holds REF3 (of WEAP1, a map marker).
    node(&mut e, dh + 0x10, WS, 0);
    e.mem.set_u32(WS + 0x34, WS_CELL);
    node(&mut e, WS_CELL + 0xac, REF3, 0);
    e.mem.set_u32(REF3 + 0x20, WEAP1);
    e.mem.set_u8(REF3 + 0x44 + 8 + 5, 0x10);
    // TES's grid: one cell holding a non-persistent REF of WEAP1 (REF1 again there).
    e.mem.set_u32(0x011d_ea10, TES_AT);
    e.mem.set_u32(TES_AT + 8, GRID);
    e.mem.set_u32(GRID + 0xc, 1);
    e.mem.set_u32(GRID + 0x10, GRID_CELLS);
    e.mem.set_u32(GRID_CELLS, GRID_CELL);
    node(&mut e, GRID_CELL + 0xac, REF1, 0);
    set_ref(&mut e, 1, LIST);
    set_ref(&mut e, 2, WEAP1);
    e
}

fn forms(v: &[u32]) -> Vec<Data> {
    v.iter().map(|&f| Data::Form(f)).collect()
}

#[test]
fn loaded_types_into_an_array() {
    let mut e = setup();
    // A type of the jump table's lists (globals), any file, then file 1 only.
    let (_, res) = call(&mut e, 0x224e, &[n(6)]);
    assert_eq!(array_data(&e, res), forms(&[4, 0x0100_0005]));
    let (_, res) = call(&mut e, 0x224e, &[n(6), n(1)]);
    assert_eq!(array_data(&e, res), forms(&[0x0100_0005]));
    // Index 255 and negative ones: any file.
    let (_, res) = call(&mut e, 0x224e, &[n(6), n(255)]);
    assert_eq!(array_data(&e, res).len(), 2);
    // A bound object type (0x28: 0x80 in the table).
    let (_, res) = call(&mut e, 0x224e, &[n(0x28)]);
    assert_eq!(array_data(&e, res), forms(&[2, 0x0100_0003]));
    // Cells from the cell array.
    let (_, res) = call(&mut e, 0x224e, &[n(0x39)]);
    assert_eq!(array_data(&e, res), forms(&[0x0100_0006]));
    // A type with no list (0x38, 255 in the table): every form of the map.
    let (_, res) = call(&mut e, 0x224e, &[n(0x38)]);
    assert_eq!(array_data(&e, res), forms(&[0xc]));
    // JIP's 301: map markers in the worldspaces' persistent cells.
    let (_, res) = call(&mut e, 0x224e, &[n(301)]);
    assert_eq!(array_data(&e, res), forms(&[0x0100_000b]));
    // Past the types: empty.
    let (_, res) = call(&mut e, 0x224e, &[n(0x79)]);
    assert!(array_data(&e, res).is_empty());
}

#[test]
fn loaded_types_into_a_form_list() {
    let mut e = setup();
    let first = heap_next(&e);
    call(&mut e, 0x2211, &[r(1), n(6)]);
    // Prepended: GLOB2 first.
    assert_eq!(e.mem.u32(LIST + 0x18), GLOB2);
    assert_eq!(e.mem.u32(first), GLOB1);
    assert_eq!(allocations(), vec![8]);
    // Again with no-clear: added to what is there.
    call(&mut e, 0x2211, &[r(1), n(0x28), n(-1), n(1)]);
    assert_eq!(super::super::list_count(&e, Ptr::new(LIST + 0x18)), 4);
    // Cleared first: the old nodes freed.
    call(&mut e, 0x2211, &[r(1), n(0x39)]);
    assert_eq!(super::super::list_count(&e, Ptr::new(LIST + 0x18)), 1);
    assert_eq!(freed().len(), 3);
}

#[test]
fn file_names_and_indexes() {
    let mut e = setup();
    let (_, res) = call(&mut e, 0x221a, &[n(1)]);
    assert_eq!(string_result(&e, res), b"Mod", "cut at the first dot");
    let (_, res) = call(&mut e, 0x221a, &[n(1), n(1)]);
    assert_eq!(string_result(&e, res), b"Mod.Two.esp");
    let (_, res) = call(&mut e, 0x221a, &[n(0xff)]);
    assert_eq!(string_result(&e, res), b"");
    let (_, res) = call(&mut e, 0x221a, &[n(9)]);
    assert_eq!(string_result(&e, res), b"", "an empty slot");
    // The calling script's own index.
    e.mem.set_u32(SCRIPT + 0xc, 0x0100_0f00);
    assert_eq!(call(&mut e, 0x221d, &[]).1, 1.0);
}

#[test]
fn a_forms_files_and_overrides() {
    let mut e = setup();
    node(&mut e, WEAP1 + 0x10, MOD_A, NODES + 0x10);
    node(&mut e, NODES + 0x10, MOD_B, 0);
    let (_, res) = call(&mut e, 0x2239, &[r(2)]);
    assert_eq!(
        array_data(&e, res),
        vec![
            Data::Str(b"FalloutNV.esm".to_vec()),
            Data::Str(b"Mod.Two.esp".to_vec())
        ]
    );
    // No form: the calling reference's base form.
    let (_, res) = call_on(&mut e, 0x2239, REF1, &[]);
    assert_eq!(array_data(&e, res).len(), 2);
    // Overridden by a file after the script's (file 0).
    e.mem.set_u32(SCRIPT + 0xc, 0x0000_0f00);
    assert_eq!(call(&mut e, 0x223a, &[r(2)]).1, 1.0);
    e.mem.set_u32(SCRIPT + 0xc, 0x0100_0f00);
    assert_eq!(call(&mut e, 0x223a, &[r(2)]).1, 0.0, "not after file 1");
}

#[test]
fn a_form_by_file_and_id() {
    let mut e = setup();
    call(&mut e, 0x2266, &[s(b"mod.two.esp"), s(b"FF000003")]);
    assert_eq!(form_result(&e), 0x0100_0003, "the hex's low 24 bits");
    e.mem.set_f64(RESULT, 0.0);
    call(&mut e, 0x2266, &[s(b"mod.two.esp"), s(b"99")]);
    assert_eq!(form_result(&e), 0, "no such form");
    call(&mut e, 0x2266, &[s(b"Nope.esp"), s(b"3")]);
    assert_eq!(form_result(&e), 0);
    in_form_map(&mut e, REF3, 0xff00_0001, 20);
    call(&mut e, 0x2266, &[s(b"none"), s(b"1")]);
    assert_eq!(form_result(&e), 0xff00_0001, "NONE is file 0xFF");
}

#[test]
fn a_forms_references() {
    let mut e = setup();
    let (_, res) = call(&mut e, 0x2251, &[r(2)]);
    assert_eq!(array_data(&e, res), forms(&[9, 0x0100_000b]));
    let (_, res) = call(&mut e, 0x2251, &[r(2), n(1)]);
    assert_eq!(array_data(&e, res), forms(&[9, 9, 0x0100_000b]));
    // A persistent reference in the grid is left out.
    e.mem.set_u32(REF1 + 8, 0x400);
    let (_, res) = call(&mut e, 0x2251, &[r(2), n(1)]);
    assert_eq!(array_data(&e, res), forms(&[9, 0x0100_000b]));
}

#[test]
fn sub_texts() {
    assert!(sub_str_ci(b"Doc Mitchell", b"mitch"));
    assert!(!sub_str_ci(b"Doc", b"Doctor"));
    assert!(!sub_str_ci(b"", b"a"));
    assert!(!sub_str_ci(b"abc", b""));
    assert!(sub_str_ci(b"abc", b"ABC"));
}

const SETTINGS: u32 = FORMS + 0x6000;
const GAMES: u32 = FORMS + 0x7000;
const COLLS: u32 = FORMS + 0x8000;

/// A setting (`Setting`: value +4, name +8) at `at`.
fn setting(e: &mut Engine, at: u32, name: &[u8], value: u32) {
    let text = at + 0x100;
    e.mem.write(text, name);
    e.mem.set_u8(text + name.len() as u32, 0);
    e.mem.set_u32(at + 4, value);
    e.mem.set_u32(at + 8, text);
}

/// The game settings' map (`*011C8048`, its map at +0x10C: buckets +4, table +8) with three
/// settings, and the four INI collections (their lists at +0x10C) empty.
fn settings_setup(e: &mut Engine) -> (u32, u32, u32) {
    e.map(0x011c_8000, 0x1000);
    e.map(0x011c_c000, 0x1000);
    e.map(0x011f_3000, 0x1000);
    e.map(0x011f_9000, 0x1000);
    let (s1, s2, s3) = (SETTINGS, SETTINGS + 0x200, SETTINGS + 0x400);
    let old = heap_alloc(e, 4);
    e.mem.write(old, b"Old\0");
    setting(e, s1, b"sGreeting", old);
    setting(e, s2, b"fGravity", 9.5f32.to_bits());
    setting(e, s3, b"bFlag", 0x0000_0101);
    e.mem.set_u32(0x011c_8048, GAMES);
    e.mem.set_u32(GAMES + 0x110, 1);
    e.mem.set_u32(GAMES + 0x114, GAMES + 0x200);
    e.mem.set_u32(GAMES + 0x200, GAMES + 0x300);
    for (i, s) in [s1, s2, s3].iter().enumerate() {
        let at = GAMES + 0x300 + 0x10 * i as u32;
        e.mem.set_u32(at, if i < 2 { at + 0x10 } else { 0 });
        e.mem.set_u32(at + 8, *s);
    }
    for (i, at) in [0x011f_96a0u32, 0x011f_35a0, 0x011c_c694, 0x011f_35a4]
        .iter()
        .enumerate()
    {
        let coll = COLLS + 0x200 * i as u32;
        e.mem.set_u32(*at, coll);
        e.mem.set_u32(coll + 0x10c, 0);
    }
    (s1, s2, s3)
}

#[test]
fn string_settings() {
    let mut e = setup();
    let (s1, _, _) = settings_setup(&mut e);
    let (_, res) = call(&mut e, 0x2283, &[s(b"SGREETING")]);
    assert_eq!(string_result(&e, res), b"Old");
    // Only names starting with s.
    let (_, res) = call(&mut e, 0x2283, &[s(b"fGravity")]);
    assert_eq!(string_result(&e, res), b"");
    let old = e.mem.u32(s1 + 4);
    let block = heap_next(&e);
    call(&mut e, 0x2284, &[s(b"sgreeting"), s(b"Hello there")]);
    assert_eq!(freed(), vec![old]);
    assert_eq!(e.mem.u32(s1 + 4), block);
    assert_eq!(*allocations().last().unwrap(), 12, "the length and its 0");
    let (_, res) = call(&mut e, 0x2283, &[s(b"sGreeting")]);
    assert_eq!(string_result(&e, res), b"Hello there");
    call(&mut e, 0x2284, &[s(b"fGravity"), s(b"x")]);
    assert_eq!(freed().len(), 1, "not a string setting: nothing");
    call(&mut e, 0x2284, &[s(b"sMissing"), s(b"x")]);
    assert_eq!(freed().len(), 1);
}

#[test]
fn all_settings_as_a_map() {
    let mut e = setup();
    settings_setup(&mut e);
    let (_, res) = call(&mut e, 0x2337, &[]);
    // Sorted by name without case: bFlag (the word as a signed integer), fGravity, sGreeting.
    assert_eq!(
        array_data(&e, res),
        vec![
            Data::Number(257.0),
            Data::Number(9.5),
            Data::Str(b"Old".to_vec())
        ]
    );
    let (_, res) = call(&mut e, 0x2337, &[n(1)]);
    assert_eq!(array_data(&e, res).len(), 2, "numbers only");
    let (_, res) = call(&mut e, 0x2337, &[n(2)]);
    assert_eq!(array_data(&e, res), vec![Data::Str(b"Old".to_vec())]);
    let (_, res) = call(&mut e, 0x2337, &[n(0)]);
    assert!(array_data(&e, res).is_empty());
}

#[test]
fn search_prints_only_in_the_console() {
    let mut e = setup();
    // Not in the console: nothing read, nothing printed.
    assert_eq!(call(&mut e, 0x2246, &[s(b"weap")]).1, 0.0);
    assert!(calls().is_empty());
}
