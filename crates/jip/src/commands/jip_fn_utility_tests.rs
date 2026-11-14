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
