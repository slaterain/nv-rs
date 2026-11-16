//! Tests of the aux variable commands: where a variable goes (mod, owner, saved or temporary), the
//! values at each index, the arrays, erasing, the condition, the changed flag and
//! `MarkModified`, the bucket counts of the maps a command makes and the `JPAV` record the
//! co-save writes from them. The game's form map (`011C54C0`, an `NiTPointerMap`: the size +4,
//! the buckets +8, items `next`, `key`, `value`) holds the forms that exist; `MarkModified`
//! (`0084A690`) is a stand-in that records.

use super::*;
use crate::commands::test_support::*;
use crate::cosave::kind;
use nvse::plugins::Records;
use nvse::vars::{Value as VarValue, KEY_NUMBER, KEY_STRING};

const BASE: u32 = FORMS + 0x1000;
const REF: u32 = FORMS + 0x1100;
const ACTOR: u32 = FORMS + 0x1200;
const OTHER: u32 = FORMS + 0x1300;
const QUEST: u32 = FORMS + 0x1400;
const FILE_A: u32 = FORMS + 0x2000;
const FILE_B: u32 = FORMS + 0x3000;
const NODE: u32 = FORMS + 0x4000;
const FMAP: u32 = FORMS + 0x5000;
const ITEMS: u32 = FORMS + 0x5400;
const LOCALS: u32 = FORMS + 0x6000;
const REF_VT: u32 = FORMS + 0x7000;
const PLAIN_VT: u32 = FORMS + 0x7400;
const SL: u32 = FORMS + 0x7800;
const ADD_CHANGE_FN: u32 = 0x0084_a690;
pub(crate) const MISSING_ID: u32 = 0x0012_3456;

fn record(addr: u32, w: &[u32]) {
    CALLS.with(|c| c.borrow_mut().push((addr, w.to_vec())));
}

pub(crate) fn marks() -> Vec<Vec<u32>> {
    calls()
        .into_iter()
        .filter(|c| c.0 == ADD_CHANGE_FN)
        .map(|c| c.1)
        .collect()
}

pub(crate) fn clear_calls() {
    CALLS.with(|c| c.borrow_mut().clear());
}

/// Makes `form` exist in the game's form map under `id` (item `n`).
pub(crate) fn exists(e: &mut Engine, form: u32, id: u32, n: u32) {
    let item = ITEMS + 0x10 * n;
    let bucket = FMAP + 0x100 + 4 * (id % 16);
    e.mem.set_u32(item, e.mem.u32(bucket));
    e.mem.set_u32(item + 4, id);
    e.mem.set_u32(item + 8, form);
    e.mem.set_u32(bucket, item);
}

/// BASE (id 0x0100_0b00), REF (a reference of BASE, 0xff00_0001), ACTOR (a reference of OTHER,
/// 0x0000_0014), OTHER (a plain form, 0x0200_0333), QUEST: all in the form map. The calling
/// script's files: FILE_A (index 3), then FILE_B (index 7): the script's mod is 7. Script
/// references: 1 REF, 2 BASE, 3 OTHER, 4 ACTOR.
pub(crate) fn setup() -> Engine {
    let mut e = engine();
    e.map(0x011c_5000, 0x1000);
    e.map(0x011c_3000, 0x1000);
    e.map(0x011d_d000, 0x1000);
    e.map(0x011d_e000, 0x1000);
    e.mem.set_u32(0x011c_54c0, FMAP);
    e.mem.set_u32(FMAP + 4, 16);
    e.mem.set_u32(FMAP + 8, FMAP + 0x100);
    e.mem.set_u32(0x011d_df38, SL);
    // The player (the co-save reads its base form).
    e.mem.set_u32(0x011d_ea3c, ACTOR);
    e.mem.set_u32(REF_VT + 0xf0, 0x008d_0360);
    for (i, (f, id, vt)) in [
        (BASE, 0x0100_0b00, PLAIN_VT),
        (REF, 0xff00_0001, REF_VT),
        (ACTOR, 0x0000_0014, REF_VT),
        (OTHER, 0x0200_0333, PLAIN_VT),
        (QUEST, 0x0000_0047, PLAIN_VT),
    ]
    .iter()
    .enumerate()
    {
        form(&mut e, *f, 0x3a, *id);
        e.mem.set_u32(*f, *vt);
        exists(&mut e, *f, *id, i as u32);
    }
    e.mem.set_u32(REF + 0x20, BASE);
    e.mem.set_u32(ACTOR + 0x20, OTHER);
    e.mem.set_u8(FILE_A + 0x40c, 3);
    e.mem.set_u8(FILE_B + 0x40c, 7);
    e.mem.set_u32(SCRIPT + 0x10, FILE_A);
    e.mem.set_u32(SCRIPT + 0x14, NODE);
    e.mem.set_u32(NODE, FILE_B);
    e.mem.set_u32(NODE + 4, 0);
    e.mem.set_u32(SCRIPT + 0xc, 0x0600_0001);
    for (i, f) in [REF, BASE, OTHER, ACTOR].iter().enumerate() {
        set_ref(&mut e, i as u16 + 1, *f);
    }
    e.register(ADD_CHANGE_FN, |_, w| {
        record(ADD_CHANGE_FN, w);
        Ret::default()
    });
    clear_calls();
    e
}

pub(crate) fn saved(e: &mut Engine) -> &mut crate::cosave::Saved {
    &mut e.ext::<crate::Jip>().saved
}

/// The values of a variable.
fn vals(e: &mut Engine, temp: bool, m: u32, owner: u32, name: &[u8]) -> Option<Vec<Value>> {
    aux_maps(e, temp)
        .get(&m)?
        .get(&owner)?
        .get(&Name(name.to_vec()))
        .cloned()
}

pub(crate) fn text(t: &[u8]) -> Value {
    Value::Text {
        kind: 4,
        bytes: t.to_vec(),
    }
}

fn set_flt(e: &mut Engine, this: u32, name: &[u8], v: f64, args: &[Vec<u8>]) -> f64 {
    let mut a = vec![s(name), z(v)];
    a.extend_from_slice(args);
    call_on(e, 0x2694, this, &a).1
}

fn get_flt(e: &mut Engine, this: u32, name: &[u8], args: &[Vec<u8>]) -> f64 {
    let mut a = vec![s(name)];
    a.extend_from_slice(args);
    call_on(e, 0x2693, this, &a).1
}

fn first(e: &mut Engine, m: u32, owner: u32, name: &[u8]) -> Value {
    vals(e, m >> 8 == 1, m & 0xff, owner, name).expect("the variable")[0].clone()
}

#[test]
fn where_a_variable_goes() {
    let mut e = setup();
    // The calling reference owns it; the script's last file gives the mod.
    assert_eq!(set_flt(&mut e, REF, b"Count", 1.5, &[]), 1.0);
    assert_eq!(
        vals(&mut e, false, 7, 0xff00_0001, b"count"),
        Some(vec![Value::Number(1.5)]),
        "mod 7, the reference's id, any case"
    );
    // A reference argument: its base form owns it.
    assert_eq!(set_flt(&mut e, ACTOR, b"Count", 2.5, &[n(0), r(1)]), 1.0);
    assert_eq!(
        vals(&mut e, false, 7, 0x0100_0b00, b"Count"),
        Some(vec![Value::Number(2.5)])
    );
    assert_eq!(
        get_flt(&mut e, 0, b"COUNT", &[n(0), r(2)]),
        2.5,
        "read through BASE"
    );
    assert_eq!(
        get_flt(&mut e, 0, b"Count", &[n(0), r(3)]),
        0.0,
        "OTHER has none"
    );
    // `_` shared (0xFF), `*` temporary, `*_` both.
    set_flt(&mut e, REF, b"_shared", 3.0, &[]);
    set_flt(&mut e, REF, b"*temp", 4.0, &[]);
    set_flt(&mut e, REF, b"*_both", 5.0, &[]);
    set_flt(&mut e, REF, b"*", 6.0, &[]);
    assert_eq!(
        first(&mut e, 0xff, 0xff00_0001, b"_shared"),
        Value::Number(3.0)
    );
    assert_eq!(
        first(&mut e, 0x107, 0xff00_0001, b"*temp"),
        Value::Number(4.0)
    );
    assert_eq!(
        first(&mut e, 0x1ff, 0xff00_0001, b"*_both"),
        Value::Number(5.0)
    );
    assert_eq!(first(&mut e, 0x107, 0xff00_0001, b"*"), Value::Number(6.0));
    assert!(
        vals(&mut e, false, 7, 0xff00_0001, b"*temp").is_none(),
        "not saved"
    );
    // No owner, no name: nothing.
    let before = aux_maps(&mut e, false).get(&7).unwrap().len();
    assert_eq!(set_flt(&mut e, 0, b"x", 1.0, &[]), 0.0, "no owner");
    assert_eq!(set_flt(&mut e, REF, b"", 1.0, &[]), 0.0, "no name");
    assert_eq!(aux_maps(&mut e, false).get(&7).unwrap().len(), before);
    // A script with one file, and with none.
    e.mem.set_u32(SCRIPT + 0x14, 0);
    set_flt(&mut e, REF, b"one", 1.0, &[]);
    assert!(
        vals(&mut e, false, 3, 0xff00_0001, b"one").is_some(),
        "its only file"
    );
    e.mem.set_u32(SCRIPT + 0x10, 0);
    set_flt(&mut e, REF, b"none", 1.0, &[]);
    assert!(
        vals(&mut e, false, 0xff, 0xff00_0001, b"none").is_some(),
        "no file: 0xFF"
    );
}

#[test]
fn values_by_index() {
    let mut e = setup();
    let get = |e: &mut Engine, idx: i32| get_flt(e, REF, b"v", &[n(idx)]);
    assert_eq!(
        set_flt(&mut e, REF, b"v", 10.0, &[n(1)]),
        0.0,
        "index 1 of nothing"
    );
    assert!(
        vals(&mut e, false, 7, 0xff00_0001, b"v").is_none(),
        "not made"
    );
    assert_eq!(
        set_flt(&mut e, REF, b"v", 10.0, &[]),
        1.0,
        "index 0 of nothing: made"
    );
    assert_eq!(
        set_flt(&mut e, REF, b"v", 11.0, &[n(-1)]),
        1.0,
        "-1 appends"
    );
    assert_eq!(
        set_flt(&mut e, REF, b"v", 12.0, &[n(2)]),
        1.0,
        "the size appends"
    );
    assert_eq!(
        set_flt(&mut e, REF, b"v", 13.0, &[n(4)]),
        0.0,
        "past the size"
    );
    assert_eq!(
        set_flt(&mut e, REF, b"v", 21.0, &[n(1)]),
        1.0,
        "the middle replaced"
    );
    assert_eq!(
        vals(&mut e, false, 7, 0xff00_0001, b"v"),
        Some(vec![
            Value::Number(10.0),
            Value::Number(21.0),
            Value::Number(12.0)
        ])
    );
    assert_eq!(get(&mut e, 0), 10.0, "the head");
    assert_eq!(get(&mut e, 1), 21.0);
    assert_eq!(get(&mut e, 2), 12.0, "the tail");
    assert_eq!(get(&mut e, -1), 12.0, "a negative index: the last");
    assert_eq!(get(&mut e, -5), 12.0);
    assert_eq!(get(&mut e, 3), 0.0, "past the end");
    assert_eq!(call_on(&mut e, 0x2691, REF, &[s(b"v")]).1, 3.0, "the size");
    assert_eq!(call_on(&mut e, 0x2691, REF, &[s(b"w")]).1, 0.0);
    // One value: index 0 and -1 are it.
    set_flt(&mut e, REF, b"one", 5.0, &[]);
    assert_eq!(get_flt(&mut e, REF, b"one", &[n(-1)]), 5.0, "one");
    assert_eq!(get_flt(&mut e, REF, b"one", &[n(1)]), 0.0);
    // Types, forms and texts at their indexes.
    call_on(&mut e, 0x2696, REF, &[s(b"v"), r(3), n(0)]);
    call_on(&mut e, 0x2698, REF, &[s(b"v"), s(b"hello"), n(2)]);
    for (idx, t) in [(0, 2.0), (1, 1.0), (2, 4.0), (3, 0.0)] {
        assert_eq!(
            call_on(&mut e, 0x2692, REF, &[s(b"v"), n(idx)]).1,
            t,
            "type {idx}"
        );
    }
    call_on(&mut e, 0x2695, REF, &[s(b"v"), n(0)]);
    assert_eq!(form_result(&e), 0x0200_0333);
    call_on(&mut e, 0x2695, REF, &[s(b"v"), n(1)]);
    assert_eq!(form_result(&e), 0, "a number: no form");
    assert_eq!(get(&mut e, 0), 0.0, "a form: no number");
    let r2 = call_on(&mut e, 0x2697, REF, &[s(b"v"), n(2)]).1;
    assert_eq!(string_result(&e, r2), b"hello");
    let r1 = call_on(&mut e, 0x2697, REF, &[s(b"v"), n(1)]).1;
    assert_eq!(string_result(&e, r1), b"", "a number: no text");
    let r9 = call_on(&mut e, 0x2697, REF, &[s(b"v"), n(9)]).1;
    assert_eq!(string_result(&e, r9), b"", "none");
    // A form that no longer exists keeps its id.
    aux_maps(&mut e, false)
        .get_mut(&7)
        .unwrap()
        .get_mut(&0xff00_0001)
        .unwrap()
        .get_mut(&Name(b"v".to_vec()))
        .unwrap()[0] = Value::Form(MISSING_ID);
    call_on(&mut e, 0x2695, REF, &[s(b"v")]);
    assert_eq!(form_result(&e), MISSING_ID);
    // A text is cut at 0x1FFF bytes and at a 0 (too long a text for the test's script line:
    // `text_value`, what the setters store, directly); an empty one is a text.
    let long = vec![b'x'; 0x2100];
    assert_eq!(text_value(&long), text(&long[..0x1fff]));
    assert_eq!(text_value(&[b'a', b'b', 0, b'c']), text(b"ab"));
    // JIP keeps the length in 16 bits before its cap: 0x10005 bytes are 5, 0x10000 an empty text.
    assert_eq!(text_value(&vec![b'z'; 0x1_0005]), text(b"zzzzz"));
    assert_eq!(text_value(&vec![b'z'; 0x1_0000]), text(b""));
    call_on(&mut e, 0x2698, REF, &[s(b"t"), s(b"long")]);
    assert_eq!(
        vals(&mut e, false, 7, 0xff00_0001, b"t"),
        Some(vec![text(b"long")])
    );
    call_on(&mut e, 0x2698, REF, &[s(b"t"), s(b"")]);
    assert_eq!(
        vals(&mut e, false, 7, 0xff00_0001, b"t"),
        Some(vec![text(b"")])
    );
    // A form argument after the index.
    assert_eq!(set_flt(&mut e, 0, b"v", 7.0, &[n(0), r(3)]), 1.0);
    assert_eq!(get_flt(&mut e, 0, b"v", &[n(0), r(3)]), 7.0);
}

#[test]
fn the_changed_flag_and_mark_modified() {
    let mut e = setup();
    crate::cosave::set_changed(&mut e, 0);
    set_flt(&mut e, REF, b"*t", 1.0, &[]);
    call_on(&mut e, 0x2696, REF, &[s(b"*t"), r(3)]);
    call_on(&mut e, 0x2698, REF, &[s(b"*t"), s(b"a")]);
    assert_eq!(
        crate::cosave::changed_flags(&mut e),
        0,
        "temporary: no flag"
    );
    assert!(marks().is_empty(), "nor MarkModified");
    for (op, args) in [
        (0x2694, vec![s(b"a"), z(1.0)]),
        (0x2696, vec![s(b"b"), r(3)]),
        (0x2698, vec![s(b"c"), s(b"t")]),
    ] {
        crate::cosave::set_changed(&mut e, 0x10);
        clear_calls();
        call_on(&mut e, op, REF, &args);
        assert_eq!(
            crate::cosave::changed_flags(&mut e),
            0x11,
            "{op:04x}: the aux flag added"
        );
        assert_eq!(marks(), vec![vec![SL, REF, 0, 0]], "{op:04x}");
    }
    clear_calls();
    call_on(&mut e, 0x2694, 0, &[s(b"a"), z(1.0), n(0), r(3)]);
    assert!(marks().is_empty(), "no calling reference: no MarkModified");
    // Reads change nothing.
    crate::cosave::set_changed(&mut e, 0);
    get_flt(&mut e, REF, b"a", &[]);
    call_on(&mut e, 0x2691, REF, &[s(b"a")]);
    assert_eq!(crate::cosave::changed_flags(&mut e), 0);
}

/// The element arrays: the values in order (a form looked up, a text, a number); `GetAll` makes
/// the map first, then each variable's array in the owner's order.
#[test]
fn arrays() {
    let mut e = setup();
    set_flt(&mut e, REF, b"v", 1.5, &[]);
    call_on(&mut e, 0x2696, REF, &[s(b"v"), r(3), n(-1)]);
    call_on(&mut e, 0x2698, REF, &[s(b"v"), s(b"txt"), n(-1)]);
    aux_maps(&mut e, false)
        .get_mut(&7)
        .unwrap()
        .get_mut(&0xff00_0001)
        .unwrap()
        .get_mut(&Name(b"v".to_vec()))
        .unwrap()
        .push(Value::Form(MISSING_ID));
    let id = call_on(&mut e, 0x26bd, REF, &[s(b"v")]).1 as u32;
    assert_eq!(
        array_values(&e, id),
        Some(vec![
            Data::Number(1.5),
            Data::Form(0x0200_0333),
            Data::Str(b"txt".to_vec()),
            Data::Form(0),
        ]),
        "a gone form: 0"
    );
    assert_eq!(call_on(&mut e, 0x26bd, REF, &[s(b"none")]).1, 0.0);
    set_flt(&mut e, REF, b"w", 2.0, &[]);
    let map = call_on(&mut e, 0x2699, REF, &[n(0)]).1 as u32;
    let m = vars::array(&e, map).expect("a map");
    assert_eq!((m.key_type, m.packed), (KEY_STRING, false));
    let Container::StrMap(entries) = &m.elements else {
        panic!("a map by text")
    };
    let entries = entries.clone();
    assert_eq!(entries.len(), 2);
    for (name, el) in entries {
        let Data::Array(inner) = el.data else {
            panic!("an array")
        };
        assert!(inner > map, "the map made before its arrays");
        let n = vars::array(&e, inner).unwrap().len();
        assert_eq!(
            n,
            if name.eq_ignore_ascii_case(b"v") {
                4
            } else {
                1
            }
        );
    }
    assert_eq!(
        call_on(&mut e, 0x2699, REF, &[n(1)]).1,
        0.0,
        "no temporary ones"
    );
    assert_eq!(
        call_on(&mut e, 0x2699, OTHER, &[n(0)]).1,
        0.0,
        "another owner"
    );
    set_flt(&mut e, REF, b"_s", 3.0, &[]);
    let shared = call_on(&mut e, 0x2699, REF, &[n(2)]).1 as u32;
    assert_eq!(vars::array(&e, shared).unwrap().len(), 1, "type 2: shared");
}

#[test]
fn set_from_an_array() {
    let mut e = setup();
    let arr = vars::create_array(&mut e, KEY_NUMBER, true, 6);
    for (i, v) in [
        VarValue::Number(2.5),
        VarValue::Form(0x0200_0333),
        VarValue::Form(MISSING_ID),
        VarValue::Str(b"s".to_vec()),
    ]
    .into_iter()
    .enumerate()
    {
        vars::set_element(&mut e, arr, &Key::Number(i as f64), v);
    }
    set_flt(&mut e, REF, b"v", 9.0, &[]);
    set_flt(&mut e, REF, b"v", 9.0, &[n(-1)]);
    crate::cosave::set_changed(&mut e, 0);
    clear_calls();
    let set =
        |e: &mut Engine, name: &[u8], id: u32| call_on(e, 0x26be, REF, &[s(name), n(id as i32)]).1;
    assert_eq!(set(&mut e, b"v", arr), 4.0);
    assert_eq!(
        vals(&mut e, false, 7, 0xff00_0001, b"v"),
        Some(vec![
            Value::Number(2.5),
            Value::Form(0x0200_0333),
            Value::Form(0),
            text(b"s"),
        ]),
        "replaced; a gone form 0"
    );
    assert_eq!(crate::cosave::changed_flags(&mut e), 1);
    assert_eq!(marks(), vec![vec![SL, REF, 0, 0]]);
    let empty = vars::create_array(&mut e, KEY_NUMBER, true, 6);
    assert_eq!(set(&mut e, b"v", empty), 0.0, "an empty array");
    assert_eq!(set(&mut e, b"v", 0), 0.0, "no array");
    assert_eq!(set(&mut e, b"v", 0x7777), 0.0, "a missing array");
    assert_eq!(vals(&mut e, false, 7, 0xff00_0001, b"v").unwrap().len(), 4);
    // A map gives its values; an array element is a number.
    let map = vars::create_array(&mut e, KEY_STRING, false, 6);
    vars::set_element(
        &mut e,
        map,
        &Key::Str(b"k".to_vec()),
        VarValue::Array(empty),
    );
    crate::cosave::set_changed(&mut e, 0);
    assert_eq!(set(&mut e, b"*m", map), 1.0);
    assert_eq!(
        vals(&mut e, true, 7, 0xff00_0001, b"*m"),
        Some(vec![Value::Number(f64::from_bits(u64::from(empty)))])
    );
    assert_eq!(
        crate::cosave::changed_flags(&mut e),
        0,
        "temporary: no flag"
    );
}

#[test]
fn erasing() {
    let mut e = setup();
    for v in [1.0, 2.0, 3.0] {
        set_flt(&mut e, REF, b"v", v, &[n(-1)]);
    }
    set_flt(&mut e, REF, b"w", 1.0, &[]);
    let erase = |e: &mut Engine, name: &[u8], args: &[Vec<u8>]| {
        let mut a = vec![s(name)];
        a.extend_from_slice(args);
        call_on(e, 0x269a, REF, &a).1
    };
    crate::cosave::set_changed(&mut e, 0);
    clear_calls();
    assert_eq!(erase(&mut e, b"v", &[n(3)]), -1.0, "past the end");
    assert_eq!(
        crate::cosave::changed_flags(&mut e),
        0,
        "nothing erased: no flag"
    );
    assert_eq!(erase(&mut e, b"v", &[n(1)]), 2.0, "the middle: 2 left");
    assert_eq!(crate::cosave::changed_flags(&mut e), 1);
    assert!(marks().is_empty(), "erasing does not MarkModified");
    assert_eq!(
        vals(&mut e, false, 7, 0xff00_0001, b"v"),
        Some(vec![Value::Number(1.0), Value::Number(3.0)])
    );
    assert_eq!(erase(&mut e, b"v", &[n(1)]), 1.0, "the tail");
    assert_eq!(
        erase(&mut e, b"v", &[n(0)]),
        0.0,
        "the last one: the variable goes"
    );
    assert!(vals(&mut e, false, 7, 0xff00_0001, b"v").is_none());
    assert!(
        vals(&mut e, false, 7, 0xff00_0001, b"w").is_some(),
        "the owner stays"
    );
    assert_eq!(erase(&mut e, b"W", &[]), -1.0, "the whole variable: -1");
    assert!(
        aux_maps(&mut e, false).get(&7).is_none(),
        "the owner and the mod gone"
    );
    crate::cosave::set_changed(&mut e, 0);
    assert_eq!(erase(&mut e, b"w", &[]), -1.0, "none");
    assert_eq!(crate::cosave::changed_flags(&mut e), 0);
    // Another owner keeps the mod.
    set_flt(&mut e, REF, b"x", 1.0, &[]);
    set_flt(&mut e, OTHER, b"x", 1.0, &[]);
    erase(&mut e, b"x", &[]);
    assert!(vals(&mut e, false, 7, 0x0200_0333, b"x").is_some());
    // Temporary: no flag.
    set_flt(&mut e, REF, b"*t", 1.0, &[]);
    crate::cosave::set_changed(&mut e, 0);
    erase(&mut e, b"*t", &[n(0)]);
    assert!(aux_maps(&mut e, true).get(&7).is_none());
    assert_eq!(crate::cosave::changed_flags(&mut e), 0);
    // EraseAll by type: 0 the script's saved, 1 temporary, 2 shared, 3 shared temporary.
    set_flt(&mut e, REF, b"a", 1.0, &[]);
    set_flt(&mut e, REF, b"*a", 1.0, &[]);
    set_flt(&mut e, REF, b"_a", 1.0, &[]);
    set_flt(&mut e, REF, b"*_a", 1.0, &[]);
    let all = |e: &mut Engine, t: i32| call_on(e, 0x269b, REF, &[n(t)]);
    crate::cosave::set_changed(&mut e, 0);
    all(&mut e, 1);
    assert!(aux_maps(&mut e, true).get(&7).is_none());
    assert!(aux_maps(&mut e, true).get(&0xff).is_some());
    assert_eq!(crate::cosave::changed_flags(&mut e), 0);
    all(&mut e, 0);
    assert!(vals(&mut e, false, 7, 0xff00_0001, b"a").is_none());
    assert!(
        vals(&mut e, false, 7, 0x0200_0333, b"x").is_some(),
        "another owner kept"
    );
    assert_eq!(crate::cosave::changed_flags(&mut e), 1);
    all(&mut e, 3);
    assert!(aux_maps(&mut e, true).get(&0xff).is_none());
    assert!(aux_maps(&mut e, false).get(&0xff).is_some());
    all(&mut e, 2);
    assert!(aux_maps(&mut e, false).get(&0xff).is_none());
    crate::cosave::set_changed(&mut e, 0);
    all(&mut e, 0);
    assert_eq!(
        crate::cosave::changed_flags(&mut e),
        0,
        "nothing to erase: no flag"
    );
    call_on(&mut e, 0x269b, OTHER, &[n(0)]);
    assert!(
        aux_maps(&mut e, false).get(&7).is_none(),
        "the last owner: the mod goes"
    );
}

/// `AuxVarGetFltCond`: the quest's variable (an id in its event list) holds a string variable
/// naming the aux variable; the calling reference owns it, the quest's script gives the mod.
#[test]
fn the_condition() {
    let mut e = setup();
    set_flt(&mut e, REF, b"Cond", 4.25, &[]);
    set_flt(&mut e, REF, b"Cond", 9.0, &[n(-1)]);
    let id = vars::add_string(&mut e, 6, b"cond", false);
    e.mem.set_u32(QUEST + 0x5c, LOCALS);
    e.mem.set_u32(QUEST + 0x1c, SCRIPT);
    e.mem.set_u32(LOCALS + 0xc, LOCALS + 0x100);
    e.mem.set_u32(LOCALS + 0x100, LOCALS + 0x200);
    e.mem.set_u32(LOCALS + 0x104, 0);
    e.mem.set_u32(LOCALS + 0x200, 5);
    e.mem.set_f64(LOCALS + 0x208, f64::from(id));
    let eval = |e: &mut Engine, this: u32, quest: u32, var: u32| {
        let record = nvse::table::record_of(e, 0x27b1);
        let at = e.mem.u32(record + 0x20);
        assert_eq!(e.mem.u32(record + 0x18), 0, "no execute");
        e.mem.set_f64(RESULT, 0.5);
        e.call(at, &[this, quest, var, RESULT]);
        e.mem.f64(RESULT)
    };
    assert_eq!(eval(&mut e, REF, QUEST, 5), 4.25, "value 0");
    assert_eq!(eval(&mut e, OTHER, QUEST, 5), 0.5, "another owner: left");
    assert_eq!(eval(&mut e, REF, QUEST, 6), 0.5, "no such variable");
    assert_eq!(eval(&mut e, REF, 0, 5), 0.5, "no quest");
    e.mem.set_u32(QUEST + 0x1c, 0);
    assert_eq!(eval(&mut e, REF, QUEST, 5), 0.5, "no script: mod 0xFF");
    e.mem.set_u32(QUEST + 0x1c, SCRIPT);
    call_on(&mut e, 0x2696, REF, &[s(b"Cond"), r(3)]);
    assert_eq!(eval(&mut e, REF, QUEST, 5), 0.0, "a form: 0 written");
    let empty = vars::add_string(&mut e, 6, b"", false);
    e.mem.set_f64(LOCALS + 0x208, f64::from(empty));
    assert_eq!(eval(&mut e, REF, QUEST, 5), 0.5, "an empty name");
    e.mem.set_u32(QUEST + 0x5c, 0);
    assert_eq!(eval(&mut e, REF, QUEST, 5), 0.5, "no event list");
}

/// The maps a command makes have JIP's bucket counts, and the co-save writes them as JIP's
/// `JPAV` record.
#[test]
fn the_maps_and_the_record() {
    let mut e = setup();
    set_flt(&mut e, REF, b"Ab", 1.5, &[]);
    call_on(&mut e, 0x2696, REF, &[s(b"ab"), r(3), n(-1)]);
    call_on(&mut e, 0x2698, REF, &[s(b"AB"), s(b"hi"), n(-1)]);
    {
        let maps = aux_maps(&mut e, false);
        let owners = maps.get(&7).unwrap();
        assert_eq!(owners.bucket_count(), 8, "a mod's owners: 8");
        assert_eq!(
            owners.get(&0xff00_0001).unwrap().bucket_count(),
            4,
            "an owner's variables: 4"
        );
    }
    let mut r = Records::default();
    crate::cosave::save(&mut e, &mut r);
    let chunk = r
        .chunks
        .iter()
        .find(|c| c.kind == kind::AUX_VARS)
        .expect("JPAV");
    assert_eq!(chunk.version, 10);
    let mut want = vec![1, 0, 7, 1, 0];
    want.extend_from_slice(&0xff00_0001u32.to_le_bytes());
    want.extend_from_slice(&[1, 0, 2, b'A', b'b', 3, 0, 1]);
    want.extend_from_slice(&1.5f64.to_le_bytes());
    want.push(2);
    want.extend_from_slice(&0x0200_0333u32.to_le_bytes());
    want.extend_from_slice(&[4, 2, 0, b'h', b'i']);
    assert_eq!(chunk.data, want, "the first spelling, the values in order");
    assert!(
        r.chunks.iter().all(|c| c.kind != kind::REF_MAPS),
        "no ref maps: no record"
    );
}

/// Every command's owner argument, with no calling reference: REF (script reference 1) stands for
/// its base BASE, as BASE itself (reference 2) does.
#[test]
fn the_owner_argument() {
    let mut e = setup();
    const BASE_ID: u32 = 0x0100_0b00;
    // The setters.
    assert_eq!(
        call_on(&mut e, 0x2696, 0, &[s(b"r"), r(3), n(0), r(1)]).1,
        1.0
    );
    assert_eq!(first(&mut e, 7, BASE_ID, b"r"), Value::Form(0x0200_0333));
    assert_eq!(
        call_on(&mut e, 0x2698, 0, &[s(b"t"), s(b"txt"), n(0), r(2)]).1,
        1.0
    );
    assert_eq!(first(&mut e, 7, BASE_ID, b"t"), text(b"txt"));
    let arr = vars::create_array(&mut e, KEY_NUMBER, true, 6);
    vars::set_element(&mut e, arr, &Key::Number(0.0), VarValue::Number(3.0));
    vars::set_element(&mut e, arr, &Key::Number(1.0), VarValue::Number(4.0));
    assert_eq!(
        call_on(&mut e, 0x26be, 0, &[s(b"a"), n(arr as i32), r(1)]).1,
        2.0
    );
    assert_eq!(
        vals(&mut e, false, 7, BASE_ID, b"a"),
        Some(vec![Value::Number(3.0), Value::Number(4.0)])
    );
    assert!(
        aux_maps(&mut e, false)
            .get(&7)
            .unwrap()
            .get(&0xff00_0001)
            .is_none(),
        "not REF's"
    );
    // The getters.
    assert_eq!(
        call_on(&mut e, 0x2691, 0, &[s(b"a"), r(2)]).1,
        2.0,
        "GetSize"
    );
    assert_eq!(
        call_on(&mut e, 0x2692, 0, &[s(b"r"), n(0), r(1)]).1,
        2.0,
        "GetType"
    );
    call_on(&mut e, 0x2695, 0, &[s(b"r"), n(0), r(2)]);
    assert_eq!(form_result(&e), 0x0200_0333, "GetRef");
    let t = call_on(&mut e, 0x2697, 0, &[s(b"t"), n(0), r(1)]).1;
    assert_eq!(string_result(&e, t), b"txt", "GetString");
    let v = call_on(&mut e, 0x26bd, 0, &[s(b"a"), r(2)]).1 as u32;
    assert_eq!(
        array_values(&e, v),
        Some(vec![Data::Number(3.0), Data::Number(4.0)]),
        "GetAsArray"
    );
    let all = call_on(&mut e, 0x2699, 0, &[n(0), r(1)]).1 as u32;
    assert_eq!(vars::array(&e, all).unwrap().len(), 3, "GetAll: r, t, a");
    // Erasing.
    assert_eq!(
        call_on(&mut e, 0x269a, 0, &[s(b"a"), n(0), r(2)]).1,
        1.0,
        "Erase"
    );
    assert_eq!(
        vals(&mut e, false, 7, BASE_ID, b"a"),
        Some(vec![Value::Number(4.0)])
    );
    call_on(&mut e, 0x269b, 0, &[n(0), r(1)]);
    assert!(
        aux_maps(&mut e, false).get(&7).is_none(),
        "EraseAll: BASE's all gone"
    );
}

/// `AuxVarGetFltCond` reads the lock-free copy of the first values ([`Unlocked`]): while a second
/// engine of the run (a real thread's) holds JIP's shared slot it answers on this engine (asking
/// for the slot here would be refused: two engines on one thread), for a saved and a temporary
/// variable; the copy follows every change (a value erased, the variable erased, the owner's
/// variables erased, the saved ones cleared before a load) and is one for the run (the second
/// engine reads the temporary variable this engine set).
#[test]
fn the_condition_reads_the_lock_free_copy() {
    let mut e = setup();
    set_flt(&mut e, REF, b"Cond", 4.25, &[]);
    set_flt(&mut e, REF, b"Cond", 9.0, &[n(-1)]);
    set_flt(&mut e, REF, b"*Tmp", 2.5, &[]);
    let saved = vars::add_string(&mut e, 6, b"cond", false);
    let temp = vars::add_string(&mut e, 6, b"*tmp", false);
    e.mem.set_u32(QUEST + 0x5c, LOCALS);
    e.mem.set_u32(QUEST + 0x1c, SCRIPT);
    e.mem.set_u32(LOCALS + 0xc, LOCALS + 0x100);
    e.mem.set_u32(LOCALS + 0x100, LOCALS + 0x200);
    e.mem.set_u32(LOCALS + 0x104, LOCALS + 0x300);
    e.mem.set_u32(LOCALS + 0x300, LOCALS + 0x400);
    e.mem.set_u32(LOCALS + 0x304, 0);
    e.mem.set_u32(LOCALS + 0x200, 5);
    e.mem.set_f64(LOCALS + 0x208, f64::from(saved));
    e.mem.set_u32(LOCALS + 0x400, 6);
    e.mem.set_f64(LOCALS + 0x408, f64::from(temp));
    let eval = |e: &mut Engine, var: u32| {
        let at = e.mem.u32(nvse::table::record_of(e, 0x27b1) + 0x20);
        e.mem.set_f64(RESULT, 0.5);
        e.call(at, &[REF, QUEST, var, RESULT]);
        e.mem.f64(RESULT)
    };
    let mut other = Engine::new_untyped_for_test();
    e.share_slots_for_test(&mut other);
    engine_core::extend::release_held_for_test();
    assert!(
        other.ext::<crate::Jip>().installed,
        "the other engine holds it"
    );
    assert_eq!(eval(&mut e, 5), 4.25, "the saved variable's value 0");
    assert_eq!(eval(&mut e, 6), 2.5, "the temporary one's");
    let m = overriding_mod(&e, SCRIPT);
    let id = e.mem.u32(REF + 0xc);
    assert_eq!(
        first_value(&mut other, true, m, id, b"*TMP"),
        Some(2.5),
        "one copy for the run"
    );
    engine_core::extend::release_held_for_test();
    call_on(&mut e, 0x269a, REF, &[s(b"Cond"), n(0)]);
    assert_eq!(eval(&mut e, 5), 9.0, "value 0 erased: the next is first");
    call_on(&mut e, 0x269a, REF, &[s(b"Cond")]);
    assert_eq!(eval(&mut e, 5), 0.5, "the variable erased");
    call_on(&mut e, 0x2694, REF, &[s(b"Cond"), z(1.5)]);
    assert_eq!(eval(&mut e, 5), 1.5);
    call_on(&mut e, 0x269b, REF, &[n(0)]);
    assert_eq!(eval(&mut e, 5), 0.5, "the owner's saved variables erased");
    assert_eq!(eval(&mut e, 6), 2.5, "not the temporary ones");
    call_on(&mut e, 0x2694, REF, &[s(b"Cond"), z(7.0)]);
    crate::cosave::process_changed_flags(&mut e, crate::cosave::changed::AUX_VARS);
    assert_eq!(eval(&mut e, 5), 0.5, "the saved ones cleared before a load");
    assert_eq!(eval(&mut e, 6), 2.5, "the temporary ones kept");
}

/// `forget_owner` drops an owner's first values, temporary and saved, and no other owner's.
#[test]
fn forgetting_an_owner() {
    let mut e = setup();
    set_flt(&mut e, REF, b"Cond", 4.25, &[]);
    set_flt(&mut e, REF, b"*Tmp", 2.5, &[]);
    set_flt(&mut e, ACTOR, b"Cond", 1.0, &[]);
    let m = overriding_mod(&e, SCRIPT);
    let (id, other) = (e.mem.u32(REF + 0xc), e.mem.u32(ACTOR + 0xc));
    forget_owner(&mut e, id);
    assert_eq!(first_value(&mut e, false, m, id, b"cond"), None);
    assert_eq!(first_value(&mut e, true, m, id, b"*tmp"), None);
    assert_eq!(first_value(&mut e, false, m, other, b"cond"), Some(1.0));
}

/// QUEST's event list holding variable `5 + i` naming each of `names` (xNVSE string
/// variables), its script SCRIPT (mod 7): `AuxVarGetFltCond`'s quest for [`cond`].
fn cond_quest(e: &mut Engine, names: &[&[u8]]) {
    e.mem.set_u32(QUEST + 0x5c, LOCALS);
    e.mem.set_u32(QUEST + 0x1c, SCRIPT);
    let mut link = LOCALS + 0xc;
    for (i, name) in names.iter().enumerate() {
        let node = LOCALS + 0x100 + 0x20 * i as u32;
        let var = LOCALS + 0x400 + 0x20 * i as u32;
        let id = vars::add_string(e, 6, name, false);
        e.mem.set_u32(link, node);
        e.mem.set_u32(node, var);
        e.mem.set_u32(node + 4, 0);
        e.mem.set_u32(var, 5 + i as u32);
        e.mem.set_f64(var + 8, f64::from(id));
        link = node + 4;
    }
}

/// `AuxVarGetFltCond` on `this` with [`cond_quest`]'s variable `var`; 0.5 is "left alone".
fn cond(e: &mut Engine, this: u32, var: u32) -> f64 {
    let at = e.mem.u32(nvse::table::record_of(e, 0x27b1) + 0x20);
    e.mem.set_f64(RESULT, 0.5);
    e.call(at, &[this, QUEST, var, RESULT]);
    e.mem.f64(RESULT)
}

/// JIP's reference destruction patch (`DestroyRefrHook`, game_patches gh2) erases a destroyed
/// temporary reference's variables, saved and temporary: the condition answers nothing for it
/// afterwards (as JIP, whose maps it reads), not the copy's old first values.
#[test]
fn a_destroyed_reference_answers_nothing() {
    let mut e = setup();
    // The save's changes map: one empty bucket (REF has no changes, so its variables go).
    let changes = FORMS + 0x7a00;
    e.mem.set_u32(SL, changes);
    e.mem.set_u32(changes + 4, 1);
    e.mem.set_u32(changes + 8, changes + 0x10);
    set_flt(&mut e, REF, b"Cond", 4.25, &[]);
    set_flt(&mut e, REF, b"*Tmp", 2.5, &[]);
    cond_quest(&mut e, &[b"cond", b"*tmp"]);
    assert_eq!((cond(&mut e, REF, 5), cond(&mut e, REF, 6)), (4.25, 2.5));
    e.site(0x0055_a488, &[REF]);
    assert_eq!(cond(&mut e, REF, 5), 0.5, "the saved variable went");
    assert_eq!(cond(&mut e, REF, 6), 0.5, "the temporary one went");
}

/// A co-save load: the saved variables read from `JPAV` answer the condition (the copy written
/// after the records are read), after the reset before the load emptied it.
#[test]
fn a_load_publishes_the_saved_variables() {
    let mut e = setup();
    set_flt(&mut e, REF, b"Cond", 4.25, &[]);
    cond_quest(&mut e, &[b"cond"]);
    let mut r = Records::default();
    crate::cosave::save(&mut e, &mut r);
    let file = nvse::cosave::write(&[nvse::cosave::Plugin {
        opcode_base: 0x2200,
        chunks: r.chunks,
    }]);
    crate::cosave::process_changed_flags(&mut e, crate::cosave::changed::AUX_VARS);
    assert_eq!(cond(&mut e, REF, 5), 0.5, "cleared");
    // The save's file 7 is loaded as 7 today.
    e.mem.set_u8(SL + crate::game::save_mod(7), 7);
    crate::cosave::set_changed(&mut e, crate::cosave::changed::ALL);
    let mut reader = nvse::cosave::Reader::open(&file).unwrap();
    reader.next_plugin().unwrap().unwrap();
    crate::cosave::load(&mut e, &mut reader).unwrap();
    assert_eq!(cond(&mut e, REF, 5), 4.25, "loaded");
}

/// `ClearJIPSavedData`'s aux part: `1` clears the script's mod's saved variables, `2` the shared
/// ones (mod 0xFF); the condition answers nothing for them afterwards, the other mod's stay.
#[test]
fn clear_jip_saved_data_clears_the_copy() {
    let mut e = setup();
    set_flt(&mut e, REF, b"Cond", 4.25, &[]);
    set_flt(&mut e, REF, b"_Sh", 1.5, &[]);
    cond_quest(&mut e, &[b"cond", b"_sh"]);
    assert_eq!((cond(&mut e, REF, 5), cond(&mut e, REF, 6)), (4.25, 1.5));
    call_on(&mut e, 0x26aa, REF, &[n(0), n(0), n(1), n(0)]);
    assert_eq!(cond(&mut e, REF, 5), 0.5, "the script's mod's went");
    assert_eq!(cond(&mut e, REF, 6), 1.5, "the shared ones stay");
    call_on(&mut e, 0x26aa, REF, &[n(0), n(0), n(2), n(0)]);
    assert_eq!(cond(&mut e, REF, 6), 0.5, "the shared ones went");
}

/// `AuxiliaryVariableSetFromArray`: the array's first value answers the condition.
#[test]
fn set_from_array_publishes_the_first_value() {
    let mut e = setup();
    cond_quest(&mut e, &[b"cond"]);
    let arr = vars::create_array(&mut e, KEY_NUMBER, true, 6);
    vars::set_element(&mut e, arr, &Key::Number(0.0), VarValue::Number(3.0));
    vars::set_element(&mut e, arr, &Key::Number(1.0), VarValue::Number(4.0));
    call_on(&mut e, 0x26be, REF, &[s(b"Cond"), n(arr as i32)]);
    assert_eq!(cond(&mut e, REF, 5), 3.0);
}
