//! Tests of the utility commands: the data handler's file slots (+0x21C, names at +0x20, the
//! index at +0x40C, the header version at +0x3DC), the save's file tables (`saveMods` +0x44,
//! `loadedMods` +0x143), the editor ID map's own bucket and compare functions as stand-ins, and
//! the game's random generator as a stand-in that records its bound.

use super::*;
use crate::commands::test_support::*;
use crate::containers::Name;
use crate::cosave::{LinkedRef, ScriptVar};
use nvse::vars::Value as V;

const MOD_A: u32 = FORMS + 0x1_0000;
const MOD_B: u32 = FORMS + 0x1_1000;
const FORM_A: u32 = FORMS + 0x2000;
const REFR: u32 = FORMS + 0x2100;
const FORM_X: u32 = FORMS + 0x2200;
const REF_VT: u32 = FORMS + 0x3000;
const PLAIN_VT: u32 = FORMS + 0x3100;
const EMAP: u32 = FORMS + 0x4000;
const EMAP_VT: u32 = FORMS + 0x4100;
const ENTRY: u32 = FORMS + 0x4200;
const EDID_TEXT: u32 = FORMS + 0x4300;
const CALC_FN: u32 = 0x0090_0000;
const EQUAL_FN: u32 = 0x0090_0010;
const FILE_7: u32 = FORMS + 0x5000;

fn setup() -> Engine {
    let mut e = engine();
    extra_stand_ins(&mut e);
    game_globals(&mut e);
    loaded_mod(&mut e, 0, MOD_A, b"FalloutNV.esm");
    loaded_mod(&mut e, 1, MOD_B, b"My Mod.esp");
    e.mem.set_f32(MOD_B + 0x3dc, 1.34);
    e.mem.set_u32(REF_VT + 0xf0, 0x008d_0360);
    form(&mut e, FORM_A, 0x1f, 0x0100_0abc);
    e.mem.set_u32(FORM_A, PLAIN_VT);
    form(&mut e, REFR, 0x3a, 0xff00_0010);
    e.mem.set_u32(REFR, REF_VT);
    e.mem.set_u32(REFR + 0x20, FORM_A);
    form(&mut e, FORM_X, 0x1f, 0x0000_0020);
    e.mem.set_u32(FORM_X, PLAIN_VT);
    in_form_map(&mut e, FORM_A, 0x0100_0abc, 0);
    in_form_map(&mut e, FORM_X, 0x0000_0020, 1);
    set_ref(&mut e, 1, FORM_A);
    set_ref(&mut e, 2, REFR);
    for i in 0..255 {
        e.mem.set_u8(SAVE_LOAD + 0x44 + i, 0xff);
        e.mem.set_u8(SAVE_LOAD + 0x143 + i, 0xff);
    }
    // The editor ID map: one bucket, one entry "DocMitchell" -> FORM_X.
    e.mem.set_u32(0x011c_54c8, EMAP);
    e.mem.set_u32(EMAP, EMAP_VT);
    e.mem.set_u32(EMAP_VT + 4, CALC_FN);
    e.mem.set_u32(EMAP_VT + 8, EQUAL_FN);
    e.mem.set_u32(EMAP + 8, EMAP + 0x20);
    e.mem.set_u32(EMAP + 0x20, ENTRY);
    e.mem.write(EDID_TEXT, b"DocMitchell\0");
    e.mem.set_u32(ENTRY + 4, EDID_TEXT);
    e.mem.set_u32(ENTRY + 8, FORM_X);
    e.register(CALC_FN, |_, _| Ret::default());
    e.register(EQUAL_FN, |e, w| {
        let (a, b) = (
            nvse::table::guest_text(e, w[1]),
            nvse::table::guest_text(e, w[2]),
        );
        Ret {
            eax: u32::from(a.eq_ignore_ascii_case(&b)),
            ..Ret::default()
        }
    });
    e
}

#[test]
fn hex_from_the_end() {
    assert_eq!(hex_to_uint(b"1A2b"), 0x1a2b);
    assert_eq!(
        hex_to_uint(b"zz12"),
        0x12,
        "stops at the first non-digit from the end"
    );
    assert_eq!(hex_to_uint(b"12:"), 0);
    assert_eq!(hex_to_uint(b""), 0);
    assert_eq!(hex_to_uint(b"FFFFFFFF"), u32::MAX);
    // A ninth digit: the shift wraps to 0 and the digit adds into the low bits.
    assert_eq!(hex_to_uint(b"123456789"), 0x2345_678a);
    assert_eq!(hex_to_uint(b"g1"), 1);
    assert_eq!(hex_to_uint(b"`1"), 1);
    assert_eq!(hex_to_uint(b"@1"), 1);
}

#[test]
fn a_form_as_text() {
    let mut e = setup();
    let (_, res) = call(&mut e, 0x262a, &[r(1)]);
    assert_eq!(string_result(&e, res), b"My Mod.esp:ABC");
    // A reference: its base form's.
    let (_, res) = call(&mut e, 0x262a, &[r(2)]);
    assert_eq!(string_result(&e, res), b"My Mod.esp:ABC");
    // No argument: the calling reference itself (file 0xFF has no name).
    let (_, res) = call_on(&mut e, 0x262a, REFR, &[]);
    assert_eq!(string_result(&e, res), b":10");
    // No argument and no reference: "".
    let (_, res) = call(&mut e, 0x262a, &[]);
    assert_eq!(string_result(&e, res), b"");
}

#[test]
fn text_to_a_form_id() {
    let mut e = setup();
    call(&mut e, 0x262b, &[s(b"My Mod.esp:ABC")]);
    assert_eq!(form_result(&e), 0x0100_0abc);
    call(&mut e, 0x262b, &[s(b"Other.esp:1")]);
    assert_eq!(form_result(&e), 0, "a file not loaded");
    call(&mut e, 0x262b, &[s(b":12")]);
    assert_eq!(
        form_result(&e),
        0xff00_0012,
        "nothing before the colon: 0xFF"
    );
    call(&mut e, 0x262b, &[s(b"docmitchell")]);
    assert_eq!(form_result(&e), 0x20, "by editor ID (the map's compare)");
    // A hex id of file 7 of the save, loaded today as 1.
    e.mem.set_u8(SAVE_LOAD + 0x44 + 7, 1);
    call(&mut e, 0x262b, &[s(b"07000123")]);
    assert_eq!(form_result(&e), 0x0100_0123);
    call(&mut e, 0x262b, &[s(b"05000123")]);
    assert_eq!(form_result(&e), 0, "file 5 not loaded");
}

/// `s_strRefs`: an answer is kept by the text's case-blind hash for the session.
#[test]
fn text_to_a_form_id_is_kept() {
    let mut e = setup();
    e.mem.set_u8(SAVE_LOAD + 0x44 + 7, 1);
    call(&mut e, 0x262b, &[s(b"07000123")]);
    e.mem.set_u8(SAVE_LOAD + 0x44 + 7, 2);
    call(&mut e, 0x262b, &[s(b"07000123")]);
    assert_eq!(form_result(&e), 0x0100_0123, "the first answer kept");
    // The file renamed after a first answer: another case of the same text keeps it.
    call(&mut e, 0x262b, &[s(b"MY MOD.ESP:abc")]);
    e.mem.write(MOD_B + 0x20, b"Gone.esp\0");
    call(&mut e, 0x262b, &[s(b"my mod.esp:ABC")]);
    assert_eq!(form_result(&e), 0x0100_0abc);
}

#[test]
fn smallest_and_largest() {
    let mut e = setup();
    assert_eq!(call(&mut e, 0x262e, &[z(3.0), z(-2.0)]).1, -2.0);
    assert_eq!(call(&mut e, 0x262f, &[z(3.0), z(-2.0)]).1, 3.0);
    let five = [z(4.0), z(9.0), z(-1.5), z(7.0), z(0.0)];
    assert_eq!(call(&mut e, 0x262e, &five).1, -1.5);
    assert_eq!(call(&mut e, 0x262f, &five).1, 9.0);
    let five = [z(4.0), z(9.0), z(1.5), z(7.0), z(-8.0)];
    assert_eq!(call(&mut e, 0x262e, &five).1, -8.0, "the fifth reached");
    // Only the given ones: three arguments do not reach a fourth.
    assert_eq!(call(&mut e, 0x262e, &[z(4.0), z(9.0), z(5.0)]).1, 4.0);
    // minsd keeps the running value only when it compares less: a NaN gives the next value.
    assert_eq!(call(&mut e, 0x262e, &[z(f64::NAN), z(2.0)]).1, 2.0);
    assert!(call(&mut e, 0x262e, &[z(2.0), z(f64::NAN)]).1.is_nan());
    assert_eq!(call(&mut e, 0x262f, &[z(f64::NAN), z(1.0), z(5.0)]).1, 5.0);
    // Equal values: the next one (-0 after 0).
    let r = call(&mut e, 0x262e, &[z(0.0), z(-0.0)]).1;
    assert_eq!(r.to_bits(), (-0.0f64).to_bits());
}

#[test]
fn load_order_changed() {
    let mut e = setup();
    assert_eq!(call(&mut e, 0x268e, &[]).1, 0.0);
    // The same index today: no change.
    e.mem.set_u8(SAVE_LOAD + 0x44 + 3, 3);
    e.mem.set_u8(SAVE_LOAD + 0x143 + 3, 3);
    assert_eq!(call(&mut e, 0x268e, &[]).1, 0.0);
    // Index 0 is not looked at.
    e.mem.set_u8(SAVE_LOAD + 0x44, 4);
    assert_eq!(call(&mut e, 0x268e, &[]).1, 0.0);
    e.mem.set_u8(SAVE_LOAD + 0x44 + 254, 5);
    assert_eq!(call(&mut e, 0x268e, &[]).1, 1.0);
    e.mem.set_u8(SAVE_LOAD + 0x44 + 254, 0xff);
    e.mem.set_u8(SAVE_LOAD + 0x44 + 1, 2);
    assert_eq!(call(&mut e, 0x268e, &[]).1, 1.0, "index 1 is");
}

#[test]
fn mod_index_validated() {
    let mut e = setup();
    e.mem.set_u8(SAVE_LOAD + 0x44 + 7, 2);
    assert_eq!(call(&mut e, 0x268f, &[n(7)]).1, 2.0);
    assert_eq!(call(&mut e, 0x268f, &[n(6)]).1, -1.0, "not loaded");
    assert_eq!(call(&mut e, 0x268f, &[n(0xff)]).1, 255.0);
    assert_eq!(call(&mut e, 0x268f, &[n(0x100)]).1, -1.0);
    assert_eq!(call(&mut e, 0x268f, &[n(-1)]).1, -1.0);
}

#[test]
fn a_mods_saved_data_cleared() {
    let mut e = setup();
    // The calling script's last file is file 7.
    e.mem.set_u8(FILE_7 + 0x40c, 7);
    e.mem.set_u32(SCRIPT + 0x10, FILE_7);
    e.mem.set_u32(SCRIPT + 0x14, 0);
    {
        let saved = &mut e.ext::<crate::Jip>().saved;
        let vars = saved.script_vars.emplace(0x10);
        vars.emplace_with(Name(b"a".to_vec()), || ScriptVar {
            mod_index: 7,
            value: 1,
        });
        vars.emplace_with(Name(b"b".to_vec()), || ScriptVar {
            mod_index: 3,
            value: 2,
        });
        saved
            .script_vars
            .emplace(0x11)
            .emplace_with(Name(b"c".to_vec()), || ScriptVar {
                mod_index: 7,
                value: 3,
            });
        saved.linked_refs.emplace_with(1, || LinkedRef {
            link_id: 2,
            mod_index: 7,
        });
        saved.linked_refs.emplace_with(3, || LinkedRef {
            link_id: 4,
            mod_index: 1,
        });
        saved.aux_vars.emplace(7).emplace(0x10);
        saved.aux_vars.emplace(0xff).emplace(0x10);
        saved.ref_maps.emplace(7);
        saved.changed = 0;
    }
    call(&mut e, 0x26aa, &[n(1), n(0), n(0), n(0)]);
    let saved = &mut e.ext::<crate::Jip>().saved;
    assert_eq!(saved.script_vars.len(), 1, "owner 0x11 emptied and gone");
    assert_eq!(saved.script_vars.get(&0x10).unwrap().len(), 1);
    assert!(saved
        .script_vars
        .get(&0x10)
        .unwrap()
        .get(&Name(b"b".to_vec()))
        .is_some());
    assert_eq!(saved.changed, 0, "no flag for script variables");
    call(&mut e, 0x26aa, &[n(0), n(1), n(0), n(0)]);
    let saved = &mut e.ext::<crate::Jip>().saved;
    assert_eq!(saved.linked_refs.len(), 1);
    assert!(saved.linked_refs.get(&3).is_some());
    assert_eq!(saved.changed, crate::cosave::changed::LINKED_REFS);
    // The aux variables of the shared mod (2): 0xFF's go, 7's stay.
    saved.changed = 0;
    call(&mut e, 0x26aa, &[n(0), n(0), n(2), n(0)]);
    let saved = &mut e.ext::<crate::Jip>().saved;
    assert!(saved.aux_vars.get(&0xff).is_none() && saved.aux_vars.get(&7).is_some());
    assert_eq!(saved.changed, crate::cosave::changed::AUX_VARS);
    // Again: nothing left for 0xFF, no flag; the mod's own (1) go.
    saved.changed = 0;
    call(&mut e, 0x26aa, &[n(0), n(0), n(2), n(0)]);
    assert_eq!(e.ext::<crate::Jip>().saved.changed, 0);
    call(&mut e, 0x26aa, &[n(0), n(0), n(1), n(1)]);
    let saved = &mut e.ext::<crate::Jip>().saved;
    assert_eq!(saved.aux_vars.len(), 0);
    assert_eq!(
        saved.changed,
        crate::cosave::changed::AUX_VARS | crate::cosave::changed::REF_MAPS
    );
    assert_eq!(saved.ref_maps.len(), 0);
    // Linked refs empty: no flag.
    saved.linked_refs.clear();
    saved.changed = 0;
    call(&mut e, 0x26aa, &[n(0), n(1), n(0), n(0)]);
    assert_eq!(e.ext::<crate::Jip>().saved.changed, 0);
}

#[test]
fn plugin_header_version() {
    let mut e = setup();
    assert_eq!(
        call(&mut e, 0x287f, &[s(b"MY MOD.ESP")]).1,
        f64::from(1.34f32)
    );
    assert_eq!(call(&mut e, 0x287f, &[s(b"Nope.esp")]).1, 0.0);
    assert_eq!(lookup_mod_by_name(&e, b"falloutnv.esm"), MOD_A);
}

#[test]
fn an_array_value() {
    let mut e = setup();
    let inner = array_of(&mut e, vec![]);
    let id = array_of(
        &mut e,
        vec![
            V::Number(2.5),
            V::Form(0x0100_0abc),
            V::Str(b"x".to_vec()),
            V::Array(inner),
            V::Form(0x0700_0001),
        ],
    );
    assert_eq!(call(&mut e, 0x28c8, &[n(id as i32), n(0)]).1, 2.5);
    call(&mut e, 0x28c8, &[n(id as i32), n(1)]);
    assert_eq!(form_result(&e), 0x0100_0abc);
    assert_eq!(
        call(&mut e, 0x28c8, &[n(id as i32), n(2)]).1,
        0.0,
        "a text: nothing"
    );
    assert_eq!(
        call(&mut e, 0x28c8, &[n(id as i32), n(3)]).1,
        f64::from(inner)
    );
    call(&mut e, 0x28c8, &[n(id as i32), n(4)]);
    assert_eq!(form_result(&e), 0, "a form no longer loaded: nothing");
    assert_eq!(call(&mut e, 0x28c8, &[n(id as i32), n(9)]).1, 0.0);
    // Negative indexes count from the end (xNVSE's array).
    assert_eq!(call(&mut e, 0x28c8, &[n(id as i32), n(-5)]).1, 2.5);
    assert_eq!(call(&mut e, 0x28c8, &[n(0), n(0)]).1, 0.0);
}

#[test]
fn a_random_integer() {
    let mut e = setup();
    e.register(0x00aa_5230, |_, w| {
        assert_eq!(w[0], 0x011c_4180, "the game's generator");
        Ret {
            eax: w[1] / 2,
            ..Ret::default()
        }
    });
    assert_eq!(call(&mut e, 0x28de, &[n(10), n(20)]).1, 15.0);
    assert_eq!(call(&mut e, 0x28de, &[n(-10), n(-4)]).1, -7.0);
    // The bound wraps: max - min is 0xFFFFFFFF.
    assert_eq!(
        call(&mut e, 0x28de, &[n(i32::MIN), n(i32::MAX)]).1,
        f64::from(((u32::MAX / 2) as i32).wrapping_add(i32::MIN))
    );
}

/// `refMaps == 2`: the shared ref maps (mod 0xFF) go, the mod's own stay; a second call with none
/// left sets no flag.
#[test]
fn shared_ref_maps_cleared() {
    let mut e = setup();
    e.mem.set_u8(FILE_7 + 0x40c, 7);
    e.mem.set_u32(SCRIPT + 0x10, FILE_7);
    e.mem.set_u32(SCRIPT + 0x14, 0);
    {
        let saved = &mut e.ext::<crate::Jip>().saved;
        saved.ref_maps.emplace(7);
        saved.ref_maps.emplace(0xff);
        saved.changed = 0;
    }
    call(&mut e, 0x26aa, &[n(0), n(0), n(0), n(2)]);
    let saved = &mut e.ext::<crate::Jip>().saved;
    assert!(saved.ref_maps.get(&0xff).is_none() && saved.ref_maps.get(&7).is_some());
    assert_eq!(saved.changed, crate::cosave::changed::REF_MAPS);
    saved.changed = 0;
    call(&mut e, 0x26aa, &[n(0), n(0), n(0), n(2)]);
    assert_eq!(e.ext::<crate::Jip>().saved.changed, 0);
}

/// GetIsLAA: 2, the answer JIP gives a patched game that reaches memory above 2 GB.
#[test]
fn large_address_aware() {
    let mut e = setup();
    assert_eq!(call(&mut e, 0x2892, &[]).1, 2.0);
    // Typed in the console: echoed (JIP's DoConsolePrint).
    let (res, lines) = echoed(&mut e, 0x2892, 0, &[]);
    assert_eq!((res, lines), (2.0, vec![b"GetIsLAA >> 2".to_vec()]));
}

// ---- Mods' files (lane files): arrays and texts in files, mod logs, GetStringHash.txt ----

fn res_of(e: &mut Engine, opcode: u32, args: &[Vec<u8>]) -> f64 {
    e.mem.set_f64(RESULT, 0.0);
    call(e, opcode, args).1
}

fn files_setup() -> Engine {
    let e = setup();
    files::fresh();
    e
}

/// `ReadArrayFromFile`: one line as an array; more as columns, or lines with `transpose`; `@`
/// forms, `$` texts, numbers; short lines filled with 0; CR, LF and empty lines skipped.
#[test]
fn an_array_read_from_a_file() {
    let mut e = files_setup();
    files::put(r"data\one.txt", b"1\t$a b\t2.5\t@My Mod.esp:ABC\r\n");
    let res = res_of(&mut e, 0x263a, &[s(b"data/one.txt")]);
    assert_eq!(
        array_data(&e, res),
        vec![
            Data::Number(1.0),
            Data::Str(b"a b".to_vec()),
            Data::Number(2.5),
            Data::Form(0x0100_0abc)
        ]
    );
    files::put(r"data\two.txt", b"1\t2\r\n\r\n3\n4\t\t5\t6\n");
    let columns = |e: &Engine, res: f64| -> Vec<Vec<Data>> {
        array_data(e, res)
            .iter()
            .map(|d| match d {
                Data::Array(id) => array_data(e, f64::from(*id)),
                _ => panic!("an array"),
            })
            .collect()
    };
    let res = res_of(&mut e, 0x263a, &[s(b"data\\two.txt")]);
    assert_eq!(
        columns(&e, res),
        vec![
            vec![Data::Number(1.0), Data::Number(3.0), Data::Number(4.0)],
            vec![Data::Number(2.0), Data::Number(0.0), Data::Number(5.0)],
        ]
    );
    let res = res_of(&mut e, 0x263a, &[s(b"data\\two.txt"), n(1)]);
    assert_eq!(
        columns(&e, res),
        vec![
            vec![Data::Number(1.0), Data::Number(2.0)],
            vec![Data::Number(3.0), Data::Number(0.0)],
            vec![Data::Number(4.0), Data::Number(5.0)],
        ]
    );
    // One line with transpose: an array of that line's array.
    let res = res_of(&mut e, 0x263a, &[s(b"data\\one.txt"), n(1)]);
    assert_eq!(columns(&e, res).len(), 1);
    // A first line starting with a tab has no columns; a missing file: nothing.
    files::put(r"data\tab.txt", b"\t1\r\n");
    assert_eq!(res_of(&mut e, 0x263a, &[s(b"data\\tab.txt")]), 0.0);
    assert_eq!(res_of(&mut e, 0x263a, &[s(b"data\\none.txt")]), 0.0);
}

/// `WriteArrayToFile`: columns of the top array's arrays (other values a column of one), tabs
/// between and LF after all but the last line; flag 1 a column a line; `0` past a column's end;
/// append after a LF; the folders made for a new file.
#[test]
fn an_array_written_to_a_file() {
    let mut e = files_setup();
    let c1 = array_of(&mut e, vec![V::Number(1.0), V::Number(2.0)]);
    let top = array_of(
        &mut e,
        vec![V::Array(c1), V::Str(b"x".to_vec()), V::Form(0x0100_0abc)],
    );
    assert_eq!(
        res_of(&mut e, 0x263c, &[s(b"out/a.txt"), n(0), n(top as i32)]),
        1.0
    );
    assert_eq!(
        files::get(r"out\a.txt").unwrap(),
        b"1\t$x\t@My Mod.esp:ABC\n2\t0\t0"
    );
    assert!(files::folders().contains(&b"out".to_vec()));
    assert_eq!(
        res_of(
            &mut e,
            0x263c,
            &[s(b"out/a.txt"), n(1), n(top as i32), n(1)]
        ),
        1.0
    );
    assert_eq!(
        files::get(r"out\a.txt").unwrap(),
        b"1\t$x\t@My Mod.esp:ABC\n2\t0\t0\n1\t2\n$x\t0\n@My Mod.esp:ABC\t0"
    );
    // Not appended: written anew.
    let one = array_of(&mut e, vec![V::Number(0.5)]);
    assert_eq!(
        res_of(&mut e, 0x263c, &[s(b"out/a.txt"), n(0), n(one as i32)]),
        1.0
    );
    assert_eq!(files::get(r"out\a.txt").unwrap(), b"0.5");
    let empty = array_of(&mut e, vec![]);
    assert_eq!(
        res_of(&mut e, 0x263c, &[s(b"out/b.txt"), n(0), n(empty as i32)]),
        0.0
    );
    assert_eq!(files::get(r"out\b.txt"), None);
    // A refused name: nothing written.
    assert_eq!(
        res_of(&mut e, 0x263c, &[s(b"../x.txt"), n(0), n(one as i32)]),
        0.0
    );
}

/// `ReadStringFromFile`: from line `start`, `count` lines (the last one's line end left out);
/// the whole file by default; `""` for a count of 0 or a missing file.
#[test]
fn a_text_read_from_a_file() {
    let mut e = files_setup();
    files::put(r"data\t.txt", b"one\r\ntwo\r\nthree\nfour");
    let text = |e: &mut Engine, args: &[Vec<u8>]| -> Vec<u8> {
        let res = res_of(e, 0x263b, args);
        string_result(e, res)
    };
    assert_eq!(
        text(&mut e, &[s(b"data/t.txt")]),
        b"one\r\ntwo\r\nthree\nfour"
    );
    assert_eq!(
        text(&mut e, &[s(b"data/t.txt"), n(2)]),
        b"two\r\nthree\nfour"
    );
    assert_eq!(text(&mut e, &[s(b"data/t.txt"), n(2), n(1)]), b"two");
    assert_eq!(
        text(&mut e, &[s(b"data/t.txt"), n(1), n(3)]),
        b"one\r\ntwo\r\nthree"
    );
    assert_eq!(text(&mut e, &[s(b"data/t.txt"), n(0), n(1)]), b"one");
    assert_eq!(text(&mut e, &[s(b"data/t.txt"), n(4), n(5)]), b"four");
    assert_eq!(text(&mut e, &[s(b"data/t.txt"), n(1), n(0)]), b"");
    assert_eq!(text(&mut e, &[s(b"data/none.txt")]), b"");
}

/// `ModLogPrint`: `Mod Logs\<file>.log`, the first print of the session anew, later ones after a
/// LF, indented by tabs (40 at most); nothing for a script of no file.
#[test]
fn mod_log_lines() {
    let mut e = files_setup();
    files::put(r"Mod Logs\My Mod.esp.log", b"old session");
    let script = crate::test_support::SCRIPT;
    e.mem.set_u32(script + 0x10, MOD_B);
    e.mem.set_u32(script + 0x14, 0);
    call(&mut e, 0x27c0, &[n(0), s(b"first")]);
    assert_eq!(files::get(r"Mod Logs\My Mod.esp.log").unwrap(), b"first");
    call(&mut e, 0x27c0, &[n(2), s(b"second")]);
    call(&mut e, 0x27c0, &[n(45), s(b"x")]);
    let mut want = b"first\n\t\tsecond\n".to_vec();
    want.extend([b'\t'; 40]);
    want.push(b'x');
    assert_eq!(files::get(r"Mod Logs\My Mod.esp.log").unwrap(), want);
    // A script of no file: nothing.
    e.mem.set_u32(script + 0x10, 0);
    let before = files::writes();
    call(&mut e, 0x27c0, &[n(0), s(b"none")]);
    assert_eq!(files::writes(), before);
}

/// `WriteStringToFile`: the text written (appended after a LF when asked and the file is there).
#[test]
fn a_text_written_to_a_file() {
    let mut e = files_setup();
    assert_eq!(
        res_of(&mut e, 0x263d, &[s(b"out/s.txt"), n(1), s(b"hello")]),
        1.0
    );
    assert_eq!(files::get(r"out\s.txt").unwrap(), b"hello");
    assert_eq!(
        res_of(&mut e, 0x263d, &[s(b"out/s.txt"), n(1), s(b"again")]),
        1.0
    );
    assert_eq!(files::get(r"out\s.txt").unwrap(), b"hello\nagain");
    assert_eq!(
        res_of(&mut e, 0x263d, &[s(b"out/s.txt"), n(0), s(b"new")]),
        1.0
    );
    assert_eq!(files::get(r"out\s.txt").unwrap(), b"new");
}

/// `GetStringHash`: JIP's hash without case, or with case when asked; with `split`, the parts and
/// their hashes in `GetStringHash.txt` and no result.
#[test]
fn string_hashes() {
    let mut e = files_setup();
    let ci = f64::from(crate::containers::str_hash_ci(b"Hello") as i32);
    let cs = f64::from(crate::containers::str_hash_cs(b"Hello") as i32);
    assert_ne!(ci, cs);
    assert_eq!(res_of(&mut e, 0x293f, &[s(b"Hello")]), ci);
    assert_eq!(res_of(&mut e, 0x293f, &[s(b"hello")]), ci);
    assert_eq!(res_of(&mut e, 0x293f, &[s(b"Hello"), n(0), n(1)]), cs);
    assert_eq!(res_of(&mut e, 0x293f, &[s(b"a||B|"), n(1)]), 0.0);
    let want = format!(
        "\"a\"\n{}\n\n\"B\"\n{}\n\n",
        crate::containers::str_hash_ci(b"a") as i32,
        crate::containers::str_hash_ci(b"B") as i32
    );
    assert_eq!(files::get("GetStringHash.txt").unwrap(), want.as_bytes());
    // StrHashCS: the same steps without folding.
    assert_eq!(
        crate::containers::str_hash_cs(b"abcde"),
        crate::containers::str_hash_ci(b"ABCDE")
    );
}
