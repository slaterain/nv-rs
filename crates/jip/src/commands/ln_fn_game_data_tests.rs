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

const SEARCH_VT: u32 = FORMS + 0x9000;
const EDID_FN: u32 = 0x0090_0200;
const PRINT_FN: u32 = 0x0071_d0a0;
const TEXTS: u32 = FORMS + 0x9400;

/// The searched forms: every form of the map gets a vtable whose `GetEditorID` (+0x130) answers
/// the text at the form's +0xF8 (else `""`); WEAP1 is named "Varmint Rifle" (its full name, the
/// component at +0x30 by `kBaseComponentOffsets`), WEAP2's editor ID is "DocsRifle"; the type
/// signatures at `0101C2AC` (reversed by type) and the console's print as stand-ins.
fn search_setup(e: &mut Engine) {
    for f in [LIST, WEAP1, WEAP2, GLOB1, GLOB2, CELL1, WS, KEYF] {
        e.mem.set_u32(f, SEARCH_VT);
    }
    e.mem.set_u32(SEARCH_VT + 0x130, EDID_FN);
    e.register(EDID_FN, |e, w| {
        let t = e.mem.u32(w[0] + 0xf8);
        Ret {
            eax: if t == 0 { EMPTY_TEXT } else { t },
            ..Ret::default()
        }
    });
    e.mem.write(TEXTS, b"Varmint Rifle\0DocsRifle\0");
    e.mem.set_u32(WEAP1 + 0x34, TEXTS);
    e.mem.set_u16(WEAP1 + 0x38, 13);
    e.mem.set_u32(WEAP2 + 0xf8, TEXTS + 14);
    for (t, sig) in [(0x28u32, b"WEAP"), (0x06, b"GLOB"), (0x38, b"KEYF")] {
        e.mem.write(0x0101_c2ac + 8 * (120 - t), sig);
    }
    e.register(0x0071_b160, |_, _| Ret {
        eax: 0x0300_f000,
        ..Ret::default()
    });
    e.register(PRINT_FN, |e, w| {
        let mut words = vec![w[0]];
        words.push(w[1]);
        for i in 0..4 {
            words.push(e.mem.u32(w[2] + 4 * i));
        }
        // The name is passed in a stack frame: its text kept as it is at the print.
        let name = nvse::table::guest_text(e, words[5]);
        NAMES.with(|n| n.borrow_mut().push(name));
        CALLS.with(|c| c.borrow_mut().push((PRINT_FN, words)));
        Ret::default()
    });
    console(e, true);
}

thread_local! {
    /// The name text of each printed line, as it was at the print.
    static NAMES: std::cell::RefCell<Vec<Vec<u8>>> = const { std::cell::RefCell::new(Vec::new()) };
}

/// A printed line: id, signature, editor ID, name.
type Line = (u32, Vec<u8>, Vec<u8>, Vec<u8>);

/// The printed lines (since the last clear of `CALLS`).
fn printed(e: &Engine) -> Vec<Line> {
    let calls = calls_to(PRINT_FN);
    let names = NAMES.with(|n| n.borrow().clone());
    let names = &names[names.len() - calls.len()..];
    calls
        .into_iter()
        .zip(names)
        .map(|(w, name)| {
            assert_eq!(nvse::table::guest_text(e, w[1]), b"[%08X]  %s  %s  %s");
            let sig = e.mem.bytes(w[3], 4);
            (w[2], sig, nvse::table::guest_text(e, w[4]), name.clone())
        })
        .collect()
}

/// The search's names are a form's, which a script decides: passed in a stack frame, never in
/// xNVSE's built-ins data. With that data full a search still prints (its format was stored by
/// the first search) and nothing stops.
#[test]
fn search_with_the_built_ins_data_full() {
    let mut e = setup();
    search_setup(&mut e);
    call(&mut e, 0x2246, &[s(b"varmint")]);
    fill_builtins(&mut e);
    CALLS.with(|c| c.borrow_mut().clear());
    e.mem.write(TEXTS + 0x40, b"Varmint Rifle Mk II\0");
    e.mem.set_u32(WEAP1 + 0x34, TEXTS + 0x40);
    e.mem.set_u16(WEAP1 + 0x38, 19);
    call(&mut e, 0x2246, &[s(b"varmint")]);
    assert_eq!(
        printed(&e),
        vec![(
            2,
            b"WEAP".to_vec(),
            b"".to_vec(),
            b"Varmint Rifle Mk II".to_vec()
        )]
    );
    assert_eq!(nvse::table::try_alloc(&mut e, 4), None, "nothing taken");
}

#[test]
fn search_in_the_console() {
    let mut e = setup();
    search_setup(&mut e);
    // A name match, case blind.
    call(&mut e, 0x2246, &[s(b"varmint")]);
    assert_eq!(
        printed(&e),
        vec![(2, b"WEAP".to_vec(), b"".to_vec(), b"Varmint Rifle".to_vec())]
    );
    // An editor ID match; "rifle" matches both.
    CALLS.with(|c| c.borrow_mut().clear());
    call(&mut e, 0x2246, &[s(b"RIFLE")]);
    let ids: Vec<u32> = printed(&e).iter().map(|p| p.0).collect();
    assert_eq!(ids.len(), 2);
    assert!(ids.contains(&2) && ids.contains(&0x0100_0003));
    // No match.
    CALLS.with(|c| c.borrow_mut().clear());
    call(&mut e, 0x2246, &[s(b"zzz")]);
    assert!(printed(&e).is_empty());
}

#[test]
fn search_type_filters() {
    let mut e = setup();
    search_setup(&mut e);
    let lines = |e: &mut Engine, ty: &[u8]| {
        CALLS.with(|c| c.borrow_mut().clear());
        call(e, 0x2246, &[s(b"rifle"), s(ty)]);
        printed(e).len()
    };
    assert_eq!(lines(&mut e, b"40"), 2, "by number: weapons");
    assert_eq!(lines(&mut e, b"6"), 0, "globals: none named so");
    assert_eq!(lines(&mut e, b"56"), 0, "an unsearchable type");
    assert_eq!(lines(&mut e, b"121"), 0, "past 120");
    assert_eq!(lines(&mut e, b"weap"), 2, "a signature, case folded");
    assert_eq!(lines(&mut e, b"GLOB"), 0);
    assert_eq!(lines(&mut e, b"ZZZZ"), 0, "an unknown signature");
    assert_eq!(
        lines(&mut e, b"KEYF"),
        0,
        "a known signature of an unsearchable type"
    );
}

#[test]
fn regions_radios_and_file_filters() {
    let mut e = setup();
    let dh = DATA_HANDLER_AT;
    // Regions: the region list (+0x1D8) holds its tList at +4.
    let (regs, r1, r2) = (FORMS + 0xa000, FORMS + 0xa100, FORMS + 0xa200);
    form(&mut e, r1, 0x37, 0x0000_0020);
    form(&mut e, r2, 0x37, 0x0100_0021);
    e.mem.set_u32(dh + 0x1d8, regs);
    node(&mut e, regs + 4, r1, NODES + 0x50);
    node(&mut e, NODES + 0x50, r2, 0);
    let (_, res) = call(&mut e, 0x224e, &[n(0x37)]);
    assert_eq!(array_data(&e, res), forms(&[0x20, 0x0100_0021]));
    let (_, res) = call(&mut e, 0x224e, &[n(0x37), n(1)]);
    assert_eq!(array_data(&e, res), forms(&[0x0100_0021]));
    // Radios (302): a reference in the interior cell with the radio data's presence bit; the
    // map marker reference there is not one.
    let (radio, marker) = (FORMS + 0xa300, FORMS + 0xa400);
    form(&mut e, radio, 0x3a, 0x0000_0030);
    form(&mut e, marker, 0x3a, 0x0100_0031);
    e.mem.set_u8(radio + 0x44 + 8 + 13, 1);
    e.mem.set_u8(marker + 0x44 + 8 + 5, 0x10);
    node(&mut e, NODES + 8, REF2, NODES + 0x58);
    node(&mut e, NODES + 0x58, radio, NODES + 0x60);
    node(&mut e, NODES + 0x60, marker, 0);
    let (_, res) = call(&mut e, 0x224e, &[n(302)]);
    assert_eq!(array_data(&e, res), forms(&[0x30]));
    let (_, res) = call(&mut e, 0x224e, &[n(302), n(1)]);
    assert!(array_data(&e, res).is_empty(), "the radio is of file 0");
    // The file filter on cells, bound objects, every form and map markers.
    let (_, res) = call(&mut e, 0x224e, &[n(0x39), n(0)]);
    assert!(array_data(&e, res).is_empty(), "the cell is of file 1");
    let (_, res) = call(&mut e, 0x224e, &[n(0x28), n(1)]);
    assert_eq!(array_data(&e, res), forms(&[0x0100_0003]));
    let (_, res) = call(&mut e, 0x224e, &[n(0x38), n(1)]);
    assert!(array_data(&e, res).is_empty());
    let (_, res) = call(&mut e, 0x224e, &[n(0x38), n(0)]);
    assert_eq!(array_data(&e, res), forms(&[0xc]));
    let (_, res) = call(&mut e, 0x224e, &[n(301), n(0)]);
    assert!(array_data(&e, res).is_empty());
    let (_, res) = call(&mut e, 0x224e, &[n(301), n(1)]);
    assert_eq!(array_data(&e, res), forms(&[0x0100_000b]));
}

#[test]
fn not_overridden_without_files() {
    let mut e = setup();
    // WEAP1 with no file at all: its last file is 0xFF, never "after".
    e.mem.set_u32(SCRIPT + 0xc, 0x0000_0f00);
    assert_eq!(call(&mut e, 0x223a, &[r(2)]).1, 0.0);
}

#[test]
fn a_negative_integer_setting() {
    let mut e = setup();
    let (_, _, s3) = settings_setup(&mut e);
    setting(&mut e, s3, b"iLow", u32::MAX);
    let (_, res) = call(&mut e, 0x2337, &[n(1)]);
    assert_eq!(
        array_data(&e, res),
        vec![Data::Number(9.5), Data::Number(-1.0)]
    );
}
