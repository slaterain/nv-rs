//! Tests of the ref map commands, on the aux variable tests' world (the form map, REF of BASE,
//! ACTOR of OTHER, the script's mod 7, `MarkModified` recorded): where a map goes, the values,
//! the walk, the keys and maps as arrays, erasing, validating and destroying, the bucket counts
//! and the `JPRM` record the co-save writes.

use super::*;
use crate::commands::jip_fn_aux_variable::tests::{
    clear_calls, marks, saved, setup, text, MISSING_ID,
};
use crate::commands::test_support::*;
use crate::cosave::kind;
use nvse::plugins::Records;
use nvse::vars::{self, Container, Data, Value as VarValue, KEY_NUMBER};

const REF_ID: u32 = 0xff00_0001;
const BASE_ID: u32 = 0x0100_0b00;
const OTHER_ID: u32 = 0x0200_0333;
const SL: u32 = FORMS + 0x7800;
const REF: u32 = FORMS + 0x1100;
const ACTOR: u32 = FORMS + 0x1200;

/// The map named `name` (temporary or saved, mod `m`).
fn ids(e: &mut Engine, temp: bool, m: u32, name: &[u8]) -> Option<UnorderedMap<u32, Value>> {
    maps(e, temp).get(&m)?.get(&Name(name.to_vec())).cloned()
}

fn set_flt(e: &mut Engine, this: u32, name: &[u8], v: f64, form: Option<u16>) {
    let mut a = vec![s(name), z(v)];
    a.extend(form.map(r));
    call_on(e, 0x26a5, this, &a);
}

fn get_flt(e: &mut Engine, this: u32, name: &[u8], form: Option<u16>) -> f64 {
    let mut a = vec![s(name)];
    a.extend(form.map(r));
    call_on(e, 0x26a4, this, &a).1
}

/// The elements of array `id`.
fn elements(e: &Engine, id: f64) -> Option<Vec<Data>> {
    array_values(e, id as u32)
}

#[test]
fn where_a_map_goes_and_its_values() {
    let mut e = setup();
    set_flt(&mut e, REF, b"Map", 1.5, None);
    set_flt(&mut e, 0, b"map", 2.5, Some(3));
    set_flt(&mut e, 0, b"MAP", 3.5, Some(1));
    let m = ids(&mut e, false, 7, b"map").expect("mod 7");
    assert_eq!(
        m.get(&REF_ID),
        Some(&Value::Number(1.5)),
        "the calling reference"
    );
    assert_eq!(m.get(&OTHER_ID), Some(&Value::Number(2.5)), "a form");
    assert_eq!(
        m.get(&BASE_ID),
        Some(&Value::Number(3.5)),
        "a reference: its base"
    );
    assert_eq!(m.len(), 3);
    assert_eq!(call_on(&mut e, 0x269c, 0, &[s(b"mAp")]).1, 3.0, "the size");
    assert_eq!(call_on(&mut e, 0x269c, 0, &[s(b"none")]).1, 0.0);
    assert_eq!(get_flt(&mut e, REF, b"map", None), 1.5);
    assert_eq!(
        get_flt(&mut e, ACTOR, b"map", Some(4)),
        2.5,
        "ACTOR: its base OTHER"
    );
    assert_eq!(get_flt(&mut e, 0, b"map", None), 0.0, "no key");
    // Scope: `*` temporary, `_` shared.
    set_flt(&mut e, REF, b"*t", 1.0, None);
    set_flt(&mut e, REF, b"_s", 2.0, None);
    set_flt(&mut e, REF, b"*_ts", 3.0, None);
    assert!(ids(&mut e, true, 7, b"*t").is_some());
    assert!(ids(&mut e, false, 7, b"*t").is_none(), "not saved");
    assert!(ids(&mut e, false, 0xff, b"_s").is_some());
    assert!(ids(&mut e, true, 0xff, b"*_ts").is_some());
    // No name, no key: nothing.
    set_flt(&mut e, REF, b"", 1.0, None);
    set_flt(&mut e, 0, b"nokey", 1.0, None);
    assert!(ids(&mut e, false, 7, b"").is_none());
    assert!(ids(&mut e, false, 7, b"nokey").is_none());
    // Types, forms, texts, the one-element array.
    call_on(&mut e, 0x26a7, REF, &[s(b"v"), r(3)]);
    call_on(&mut e, 0x26a9, 0, &[s(b"v"), s(b"text"), r(3)]);
    call_on(&mut e, 0x26a5, 0, &[s(b"v"), z(4.0), r(2)]);
    assert_eq!(call_on(&mut e, 0x269d, REF, &[s(b"v")]).1, 2.0);
    assert_eq!(call_on(&mut e, 0x269d, 0, &[s(b"v"), r(3)]).1, 4.0);
    assert_eq!(call_on(&mut e, 0x269d, 0, &[s(b"v"), r(2)]).1, 1.0);
    assert_eq!(
        call_on(&mut e, 0x269d, ACTOR, &[s(b"v")]).1,
        0.0,
        "no entry"
    );
    call_on(&mut e, 0x26a6, REF, &[s(b"v")]);
    assert_eq!(form_result(&e), OTHER_ID);
    call_on(&mut e, 0x26a6, 0, &[s(b"v"), r(3)]);
    assert_eq!(form_result(&e), 0, "a text: no form");
    let t = call_on(&mut e, 0x26a8, 0, &[s(b"v"), r(3)]).1;
    assert_eq!(string_result(&e, t), b"text");
    let t = call_on(&mut e, 0x26a8, REF, &[s(b"v")]).1;
    assert_eq!(string_result(&e, t), b"", "a form: no text");
    assert_eq!(get_flt(&mut e, REF, b"v", None), 0.0, "a form: no number");
    let one = call_on(&mut e, 0x2764, 0, &[s(b"v"), r(3)]).1;
    assert_eq!(elements(&e, one), Some(vec![Data::Str(b"text".to_vec())]));
    let one = call_on(&mut e, 0x2764, REF, &[s(b"v")]).1;
    assert_eq!(elements(&e, one), Some(vec![Data::Form(OTHER_ID)]));
    assert_eq!(
        call_on(&mut e, 0x2764, ACTOR, &[s(b"v")]).1,
        0.0,
        "no entry: no array"
    );
}

#[test]
fn the_changed_flag_and_mark_modified() {
    let mut e = setup();
    saved(&mut e).changed = 0;
    set_flt(&mut e, REF, b"*t", 1.0, None);
    call_on(&mut e, 0x26a7, REF, &[s(b"*t"), r(3)]);
    assert_eq!(saved(&mut e).changed, 0, "temporary: no flag");
    assert!(marks().is_empty());
    let arr = vars::create_array(&mut e, KEY_NUMBER, true, 6);
    vars::set_element(&mut e, arr, &Key::Number(0.0), VarValue::Number(1.0));
    for (op, args) in [
        (0x26a5, vec![s(b"a"), z(1.0)]),
        (0x26a7, vec![s(b"b"), r(3)]),
        (0x26a9, vec![s(b"c"), s(b"t")]),
        (0x2765, vec![s(b"d"), n(arr as i32)]),
    ] {
        saved(&mut e).changed = 0x10;
        clear_calls();
        call_on(&mut e, op, REF, &args);
        assert_eq!(
            saved(&mut e).changed,
            0x12,
            "{op:04x}: the ref map flag added"
        );
        assert_eq!(marks(), vec![vec![SL, REF, 0, 0]], "{op:04x}");
    }
    clear_calls();
    set_flt(&mut e, 0, b"a", 1.0, Some(3));
    assert!(marks().is_empty(), "no calling reference");
    saved(&mut e).changed = 0;
    get_flt(&mut e, REF, b"a", None);
    assert_eq!(saved(&mut e).changed, 0, "a read");
}

#[test]
fn set_from_an_array() {
    let mut e = setup();
    let set = |e: &mut Engine, values: &[VarValue]| {
        let arr = vars::create_array(e, KEY_NUMBER, true, 6);
        for (i, v) in values.iter().enumerate() {
            vars::set_element(e, arr, &Key::Number(i as f64), v.clone());
        }
        call_on(e, 0x2765, REF, &[s(b"v"), n(arr as i32)]);
        ids(e, false, 7, b"v").and_then(|m| m.get(&REF_ID).cloned())
    };
    assert_eq!(
        set(&mut e, &[VarValue::Number(2.5)]),
        Some(Value::Number(2.5))
    );
    assert_eq!(
        set(&mut e, &[VarValue::Form(OTHER_ID)]),
        Some(Value::Form(OTHER_ID))
    );
    assert_eq!(
        set(&mut e, &[VarValue::Form(MISSING_ID)]),
        Some(Value::Form(0)),
        "gone: 0"
    );
    assert_eq!(
        set(&mut e, &[VarValue::Str(b"s".to_vec())]),
        Some(text(b"s"))
    );
    assert_eq!(
        set(&mut e, &[VarValue::Number(7.0), VarValue::Number(8.0)]),
        Some(text(b"s")),
        "two elements: nothing"
    );
    assert_eq!(set(&mut e, &[]), Some(text(b"s")), "none: nothing");
    call_on(&mut e, 0x2765, REF, &[s(b"v"), n(0x7777)]);
    assert_eq!(
        ids(&mut e, false, 7, b"v").unwrap().get(&REF_ID),
        Some(&text(b"s"))
    );
}

/// Three entries of a map, the expected walking order taken from the map itself (JIP's order:
/// bucket by bucket, each chain from its newest).
fn three(e: &mut Engine) -> Vec<u32> {
    set_flt(e, REF, b"w", 1.0, None);
    set_flt(e, 0, b"w", 2.0, Some(3));
    set_flt(e, 0, b"w", 3.0, Some(2));
    ids(e, false, 7, b"w")
        .unwrap()
        .iter()
        .map(|(k, _)| *k)
        .collect()
}

fn value_of(id: u32) -> f64 {
    match id {
        REF_ID => 1.0,
        OTHER_ID => 2.0,
        _ => 3.0,
    }
}

#[test]
fn the_walk() {
    let mut e = setup();
    let order = three(&mut e);
    assert_eq!(order.len(), 3);
    let step = |e: &mut Engine, op: u32, name: &[u8]| {
        let v = call_on(e, op, 0, &[s(name)]).1;
        elements(e, v)
    };
    let pair = |id: u32| Some(vec![Data::Form(id), Data::Number(value_of(id))]);
    assert_eq!(step(&mut e, 0x269e, b"w"), pair(order[0]), "the first");
    assert_eq!(
        step(&mut e, 0x269f, b"W"),
        pair(order[1]),
        "the next, any case"
    );
    assert_eq!(step(&mut e, 0x269f, b"w"), pair(order[2]), "the last");
    assert_eq!(step(&mut e, 0x269f, b"w"), None, "past the end");
    assert_eq!(step(&mut e, 0x269f, b"w"), None);
    assert_eq!(
        step(&mut e, 0x269e, b"w"),
        pair(order[0]),
        "GetFirst starts again"
    );
    // Another map: GetNext starts it from its first.
    set_flt(&mut e, REF, b"x", 9.0, None);
    let x = step(&mut e, 0x269f, b"x");
    assert_eq!(x, Some(vec![Data::Form(REF_ID), Data::Number(9.0)]));
    assert_eq!(
        step(&mut e, 0x269f, b"w"),
        pair(order[0]),
        "back to w: from its first"
    );
    // The next entry erased: the walk ends.
    let next = order[1];
    let form = match next {
        REF_ID => None,
        OTHER_ID => Some(3),
        _ => Some(2),
    };
    let mut args = vec![s(b"w")];
    args.extend(form.map(r));
    call_on(&mut e, 0x26a2, if form.is_none() { REF } else { 0 }, &args);
    assert_eq!(step(&mut e, 0x269f, b"w"), None, "the next one is gone");
    assert_eq!(step(&mut e, 0x269e, b"none"), None, "no map");
    assert_eq!(step(&mut e, 0x269e, b""), None, "no name");
    // A key whose form is gone gives a null form.
    maps(&mut e, false)
        .get_mut(&7)
        .unwrap()
        .get_mut(&Name(b"x".to_vec()))
        .unwrap()
        .erase(&REF_ID);
    maps(&mut e, false)
        .get_mut(&7)
        .unwrap()
        .get_mut(&Name(b"x".to_vec()))
        .unwrap()
        .emplace_with(MISSING_ID, || Value::Number(4.0));
    assert_eq!(
        step(&mut e, 0x269e, b"x"),
        Some(vec![Data::Form(0), Data::Number(4.0)])
    );
}

#[test]
fn keys_and_all_maps() {
    let mut e = setup();
    let order = three(&mut e);
    let keys = call_on(&mut e, 0x26a0, 0, &[s(b"w")]).1;
    assert_eq!(
        elements(&e, keys),
        Some(order.iter().map(|&k| Data::Form(k)).collect()),
        "the keys' forms in the map's order"
    );
    assert_eq!(call_on(&mut e, 0x26a0, 0, &[s(b"none")]).1, 0.0);
    set_flt(&mut e, 0, b"y", 1.0, Some(3));
    let all = call_on(&mut e, 0x26a1, 0, &[n(0)]).1 as u32;
    let a = vars::array(&e, all).expect("a map");
    let Container::StrMap(entries) = &a.elements else {
        panic!("a map by text")
    };
    let entries = entries.clone();
    assert_eq!(entries.len(), 2);
    for (name, el) in entries {
        let Data::Array(inner) = el.data else {
            panic!("an array")
        };
        assert!(inner > all, "the map made before its arrays");
        let n = vars::array(&e, inner).unwrap().len();
        assert_eq!(n, if name == b"w" { 3 } else { 1 }, "{name:?}");
    }
    assert_eq!(
        call_on(&mut e, 0x26a1, 0, &[n(1)]).1,
        0.0,
        "no temporary maps"
    );
    set_flt(&mut e, REF, b"*_t", 1.0, None);
    let t = call_on(&mut e, 0x26a1, 0, &[n(3)]).1 as u32;
    assert_eq!(
        vars::array(&e, t).unwrap().len(),
        1,
        "type 3: shared temporary"
    );
}

#[test]
fn erasing_validating_destroying() {
    let mut e = setup();
    three(&mut e);
    saved(&mut e).changed = 0;
    clear_calls();
    assert_eq!(
        call_on(&mut e, 0x26a2, 0, &[s(b"w"), r(3)]).1,
        2.0,
        "2 left"
    );
    assert_eq!(saved(&mut e).changed, 2);
    assert!(marks().is_empty(), "erasing does not MarkModified");
    saved(&mut e).changed = 0;
    assert_eq!(
        call_on(&mut e, 0x26a2, 0, &[s(b"w"), r(3)]).1,
        0.0,
        "gone already"
    );
    assert_eq!(saved(&mut e).changed, 0, "nothing erased: no flag");
    assert_eq!(call_on(&mut e, 0x26a2, REF, &[s(b"w")]).1, 1.0);
    assert_eq!(
        call_on(&mut e, 0x26a2, 0, &[s(b"w"), r(2)]).1,
        0.0,
        "the last: no count"
    );
    assert!(
        maps(&mut e, false).get(&7).is_none(),
        "the map and the mod gone"
    );
    // Validate: keys whose form is gone erased.
    set_flt(&mut e, REF, b"v", 1.0, None);
    set_flt(&mut e, 0, b"v", 2.0, Some(3));
    maps(&mut e, false)
        .get_mut(&7)
        .unwrap()
        .get_mut(&Name(b"v".to_vec()))
        .unwrap()
        .emplace_with(MISSING_ID, || Value::Number(3.0));
    saved(&mut e).changed = 0;
    assert_eq!(call_on(&mut e, 0x26f0, 0, &[s(b"V")]).1, 2.0);
    assert_eq!(saved(&mut e).changed, 2);
    assert!(ids(&mut e, false, 7, b"v")
        .unwrap()
        .get(&MISSING_ID)
        .is_none());
    saved(&mut e).changed = 0;
    assert_eq!(call_on(&mut e, 0x26f0, 0, &[s(b"v")]).1, 2.0);
    assert_eq!(saved(&mut e).changed, 0, "nothing gone: no flag");
    // A map of gone forms only: it goes.
    maps(&mut e, false)
        .get_mut(&7)
        .unwrap()
        .emplace_with(Name(b"g".to_vec()), || UnorderedMap::new(4))
        .0
        .emplace_with(MISSING_ID, || Value::Number(1.0));
    assert_eq!(call_on(&mut e, 0x26f0, 0, &[s(b"g")]).1, 0.0);
    assert!(ids(&mut e, false, 7, b"g").is_none());
    assert!(ids(&mut e, false, 7, b"v").is_some(), "the mod stays");
    // Destroy.
    saved(&mut e).changed = 0;
    call_on(&mut e, 0x26a3, 0, &[s(b"v")]);
    assert!(
        maps(&mut e, false).get(&7).is_none(),
        "the last map: the mod goes"
    );
    assert_eq!(saved(&mut e).changed, 2);
    saved(&mut e).changed = 0;
    call_on(&mut e, 0x26a3, 0, &[s(b"v")]);
    assert_eq!(saved(&mut e).changed, 0, "none: no flag");
    // Temporary maps: no flag.
    set_flt(&mut e, REF, b"*t", 1.0, None);
    set_flt(&mut e, 0, b"*t", 1.0, Some(3));
    call_on(&mut e, 0x26a2, REF, &[s(b"*t")]);
    maps(&mut e, true)
        .get_mut(&7)
        .unwrap()
        .get_mut(&Name(b"*t".to_vec()))
        .unwrap()
        .emplace_with(MISSING_ID, || Value::Number(1.0));
    call_on(&mut e, 0x26f0, 0, &[s(b"*t")]);
    call_on(&mut e, 0x26a3, 0, &[s(b"*t")]);
    assert!(maps(&mut e, true).get(&7).is_none());
    assert_eq!(saved(&mut e).changed, 0);
    // A second map keeps the mod.
    set_flt(&mut e, REF, b"a", 1.0, None);
    set_flt(&mut e, REF, b"b", 1.0, None);
    call_on(&mut e, 0x26a2, REF, &[s(b"a")]);
    assert!(ids(&mut e, false, 7, b"b").is_some());
}

/// The maps a command makes have JIP's bucket counts (a mod's names 4, a name's ids 4), and the
/// co-save writes them as JIP's `JPRM` record.
#[test]
fn the_maps_and_the_record() {
    let mut e = setup();
    set_flt(&mut e, REF, b"Rm", 1.5, None);
    call_on(&mut e, 0x26a9, REF, &[s(b"rm"), s(b"hi")]);
    let names = saved(&mut e).ref_maps.get(&7).unwrap();
    assert_eq!(names.bucket_count(), 4, "a mod's names: 4");
    assert_eq!(
        names.get(&Name(b"rm".to_vec())).unwrap().bucket_count(),
        4,
        "ids: 4"
    );
    let mut r = Records::default();
    crate::cosave::save(&mut e, &mut r);
    let chunk = r
        .chunks
        .iter()
        .find(|c| c.kind == kind::REF_MAPS)
        .expect("JPRM");
    assert_eq!(chunk.version, 10);
    let mut want = vec![1, 0, 7, 1, 0, 2, b'R', b'm', 1, 0];
    want.extend_from_slice(&REF_ID.to_le_bytes());
    want.extend_from_slice(&[4, 2, 0, b'h', b'i']);
    assert_eq!(chunk.data, want, "the first spelling, the last value");
}

/// The ref map setters' key argument, with no calling reference: REF (reference 1) stands for its
/// base BASE, as BASE (reference 2) does.
#[test]
fn the_key_argument_of_the_setters() {
    let mut e = setup();
    call_on(&mut e, 0x26a7, 0, &[s(b"m"), r(3), r(1)]);
    assert_eq!(
        ids(&mut e, false, 7, b"m").unwrap().get(&BASE_ID),
        Some(&Value::Form(OTHER_ID))
    );
    call_on(&mut e, 0x26a9, 0, &[s(b"n"), s(b"t"), r(2)]);
    assert_eq!(
        ids(&mut e, false, 7, b"n").unwrap().get(&BASE_ID),
        Some(&text(b"t"))
    );
    let arr = vars::create_array(&mut e, KEY_NUMBER, true, 6);
    vars::set_element(&mut e, arr, &Key::Number(0.0), VarValue::Number(5.0));
    call_on(&mut e, 0x2765, 0, &[s(b"o"), n(arr as i32), r(1)]);
    assert_eq!(
        ids(&mut e, false, 7, b"o").unwrap().get(&BASE_ID),
        Some(&Value::Number(5.0))
    );
    for name in [b"m", b"n", b"o"] {
        assert_eq!(ids(&mut e, false, 7, name).unwrap().len(), 1, "only BASE");
    }
}

/// Walk rules: the same name in another mod is another map (`GetNext` starts it again); erasing
/// the entry just returned or adding a key to a later bucket leaves the walk going.
#[test]
fn walk_rules() {
    let mut e = setup();
    // The three keys fall in buckets 0 (BASE), 1 (REF) and 3 (OTHER) of 4: mixed hash
    // `(k * 13) ^ (k >> 15)`, its low two bits.
    let order = three(&mut e);
    assert_eq!(order, vec![BASE_ID, REF_ID, OTHER_ID], "worked by hand");
    let step = |e: &mut Engine, op: u32| {
        let v = call_on(e, op, 0, &[s(b"w")]).1;
        elements(e, v)
    };
    let pair = |id: u32| Some(vec![Data::Form(id), Data::Number(value_of(id))]);
    assert_eq!(step(&mut e, 0x269e), pair(BASE_ID));
    // The entry just returned erased: the walk goes on.
    call_on(&mut e, 0x26a2, 0, &[s(b"w"), r(2)]);
    // A key added to bucket 2 (key 2: `2 * 13 = 26`, low bits 10), after the place of the walk.
    maps(&mut e, false)
        .get_mut(&7)
        .unwrap()
        .get_mut(&Name(b"w".to_vec()))
        .unwrap()
        .emplace_with(2, || Value::Number(7.0));
    assert_eq!(step(&mut e, 0x269f), pair(REF_ID));
    assert_eq!(
        step(&mut e, 0x269f),
        Some(vec![Data::Form(0), Data::Number(7.0)]),
        "the new key"
    );
    assert_eq!(step(&mut e, 0x269f), pair(OTHER_ID));
    assert_eq!(step(&mut e, 0x269f), None);
    // The same name in mod 3: GetNext starts that map.
    assert_eq!(step(&mut e, 0x269e), pair(REF_ID), "w now: REF first");
    e.mem.set_u32(SCRIPT + 0x14, 0);
    set_flt(&mut e, 0, b"w", 8.0, Some(3));
    assert_eq!(
        step(&mut e, 0x269f),
        Some(vec![Data::Form(OTHER_ID), Data::Number(8.0)]),
        "mod 3's map, from its first"
    );
    assert_eq!(step(&mut e, 0x269f), None, "its only entry");
}

/// A walk over 64 entries (the map resized on the way) gives every entry once, in the map's own
/// walking order (the order the co-save writes).
#[test]
fn a_long_walk() {
    let mut e = setup();
    let ids_map = maps(&mut e, false)
        .emplace_with(7, || UnorderedMap::new(4))
        .0
        .emplace_with(Name(b"big".to_vec()), || UnorderedMap::new(4))
        .0;
    for i in 0..64u32 {
        ids_map.emplace_with(0x0100_0000 + i * 0x101, || Value::Number(f64::from(i)));
    }
    let want: Vec<f64> = ids(&mut e, false, 7, b"big")
        .unwrap()
        .iter()
        .map(|(_, v)| value_float(v))
        .collect();
    let mut got = Vec::new();
    let mut op = 0x269e;
    loop {
        let v = call_on(&mut e, op, 0, &[s(b"big")]).1;
        op = 0x269f;
        let Some(pair) = elements(&e, v) else {
            break;
        };
        let Data::Number(n) = pair[1] else {
            panic!("a number")
        };
        got.push(n);
    }
    assert_eq!(got.len(), 64);
    assert_eq!(got, want);
}

/// A co-save written by hand in JIP's layout (`JPAV`, `JPRM`: mod 7, owners REF and BASE, names in
/// mixed case, the three value types and a text of type 3) loads into what the commands see.
#[test]
fn a_loaded_co_save() {
    let mut e = setup();
    let sl = e.mem.u32(0x011d_df38);
    for i in 0..0xffu32 {
        e.mem.set_u8(sl + crate::game::save_mod(i as u8), i as u8);
    }
    // The save's changes map: empty.
    let chg = FORMS + 0x8000;
    e.mem.set_u32(chg + 4, 16);
    e.mem.set_u32(chg + 8, chg + 0x100);
    e.mem.set_u32(sl, chg);
    saved(&mut e).changed = 0x1f;
    let mut av = vec![1, 0, 7, 2, 0];
    av.extend_from_slice(&REF_ID.to_le_bytes());
    av.extend_from_slice(&[1, 0, 5]);
    av.extend_from_slice(b"MiXed");
    av.extend_from_slice(&[3, 0, 1]);
    av.extend_from_slice(&2.5f64.to_le_bytes());
    av.push(2);
    av.extend_from_slice(&OTHER_ID.to_le_bytes());
    av.extend_from_slice(&[3, 2, 0, b'a', b'b']);
    av.extend_from_slice(&BASE_ID.to_le_bytes());
    av.extend_from_slice(&[1, 0, 5]);
    av.extend_from_slice(b"Other");
    av.extend_from_slice(&[1, 0, 4, 2, 0, b'h', b'i']);
    let mut rm = vec![1, 0, 7, 1, 0, 4];
    rm.extend_from_slice(b"RmAp");
    rm.extend_from_slice(&[3, 0]);
    rm.extend_from_slice(&OTHER_ID.to_le_bytes());
    rm.push(1);
    rm.extend_from_slice(&3.0f64.to_le_bytes());
    rm.extend_from_slice(&REF_ID.to_le_bytes());
    rm.extend_from_slice(&[4, 1, 0, b'x']);
    rm.extend_from_slice(&BASE_ID.to_le_bytes());
    rm.push(2);
    rm.extend_from_slice(&REF_ID.to_le_bytes());
    let chunks = vec![
        nvse::cosave::Chunk {
            kind: kind::AUX_VARS,
            version: 10,
            data: av,
        },
        nvse::cosave::Chunk {
            kind: kind::REF_MAPS,
            version: 10,
            data: rm,
        },
    ];
    let file = nvse::cosave::write(&[nvse::cosave::Plugin {
        opcode_base: 0x2200,
        chunks,
    }]);
    let mut reader = nvse::cosave::Reader::open(&file).unwrap();
    reader.next_plugin().unwrap().unwrap();
    crate::cosave::load(&mut e, &mut reader).unwrap();
    // The aux variables, through the commands.
    assert_eq!(call_on(&mut e, 0x2693, REF, &[s(b"mixed")]).1, 2.5);
    call_on(&mut e, 0x2695, REF, &[s(b"MIXED"), n(1)]);
    assert_eq!(form_result(&e), OTHER_ID);
    assert_eq!(
        call_on(&mut e, 0x2692, REF, &[s(b"mixed"), n(2)]).1,
        3.0,
        "type 3 kept"
    );
    let t = call_on(&mut e, 0x2697, REF, &[s(b"mixed"), n(2)]).1;
    assert_eq!(string_result(&e, t), b"ab");
    let a = call_on(&mut e, 0x26bd, REF, &[s(b"mixed")]).1 as u32;
    assert_eq!(
        array_values(&e, a),
        Some(vec![
            Data::Number(2.5),
            Data::Form(OTHER_ID),
            Data::Number(0.0)
        ]),
        "a text of type 3: 0"
    );
    let all = call_on(&mut e, 0x2699, REF, &[n(0)]).1 as u32;
    let names: Vec<Vec<u8>> = match &vars::array(&e, all).unwrap().elements {
        Container::StrMap(entries) => entries.iter().map(|(k, _)| k.clone()).collect(),
        _ => panic!("a map by text"),
    };
    assert_eq!(names, vec![b"MiXed".to_vec()], "the file's spelling");
    let t = call_on(&mut e, 0x2697, 0, &[s(b"other"), n(0), r(2)]).1;
    assert_eq!(string_result(&e, t), b"hi");
    // The ref map, walked in its buckets' order: BASE (0), REF (1), OTHER (3).
    let step = |e: &mut Engine, op: u32| {
        let v = call_on(e, op, 0, &[s(b"rmap")]).1;
        elements(e, v)
    };
    assert_eq!(
        step(&mut e, 0x269e),
        Some(vec![Data::Form(BASE_ID), Data::Form(REF_ID)])
    );
    assert_eq!(
        step(&mut e, 0x269f),
        Some(vec![Data::Form(REF_ID), Data::Str(b"x".to_vec())])
    );
    assert_eq!(
        step(&mut e, 0x269f),
        Some(vec![Data::Form(OTHER_ID), Data::Number(3.0)])
    );
    assert_eq!(step(&mut e, 0x269f), None);
    assert_eq!(call_on(&mut e, 0x269c, 0, &[s(b"RMAP")]).1, 3.0);
}
