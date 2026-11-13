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
