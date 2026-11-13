//! Tests of the perk commands, at raw offsets: the perk's flags (+0x38 trait, +0x39 level), its
//! entry list's first node at +0x48; a quest entry (vtable `01046B84`: quest +8, stage +0xC), an
//! ability entry (`01046C44`: spell +8), entry point entries (`01046D0C`: entry point +8, data
//! +0xC) with one value (`010462C0`, +4), two values (`01046300`, +4/+8), a leveled list
//! (`01046320`, +4) and an activation choice (`01046340`, its label string at +4).

use super::*;
use crate::commands::test_support::*;

const PERK: u32 = FORMS + 0x1000;
const QUEST: u32 = FORMS + 0x2000;
const SPELL: u32 = FORMS + 0x2100;
const LVLI: u32 = FORMS + 0x2200;
const MISC: u32 = FORMS + 0x2300;
const ACTOR: u32 = FORMS + 0x2400;
const ENTRIES: u32 = FORMS + 0x4000;
const NODES: u32 = FORMS + 0x5000;
const DATA: u32 = FORMS + 0x6000;
const ACTOR_VTBL: u32 = FORMS + 0x7000;

/// Entries 0-6: quest, ability, one value, two values, leveled list, activation choice, an entry
/// point with the base data class.
fn setup() -> Engine {
    let mut e = engine();
    form(&mut e, PERK, 0x56, 0x0000_9e4c);
    form(&mut e, QUEST, 0x47, 0x0000_0a11);
    form(&mut e, SPELL, 0x14, 0x0000_0b22);
    form(&mut e, LVLI, 0x34, 0x0000_0c33);
    form(&mut e, MISC, 0x1f, 0x0000_0999);
    for (i, f) in [PERK, QUEST, SPELL, LVLI, MISC].iter().enumerate() {
        set_ref(&mut e, i as u16 + 1, *f);
    }
    let entry = |i: u32| ENTRIES + 0x20 * i;
    let data = |i: u32| DATA + 0x20 * i;
    e.mem.set_u32(entry(0), 0x0104_6b84);
    e.mem.set_u32(entry(0) + 8, QUEST);
    e.mem.set_u8(entry(0) + 0xc, 20);
    e.mem.set_u32(entry(1), 0x0104_6c44);
    e.mem.set_u32(entry(1) + 8, SPELL);
    let kinds = [
        (2, 0x0104_62c0u32),
        (3, 0x0104_6300),
        (4, 0x0104_6320),
        (5, 0x0104_6340),
        (6, 0x0104_62e0),
    ];
    for (i, vtbl) in kinds {
        e.mem.set_u32(entry(i), 0x0104_6d0c);
        e.mem.set_u8(entry(i) + 8, 10 + i as u8);
        e.mem.set_u32(entry(i) + 0xc, data(i));
        e.mem.set_u32(data(i), vtbl);
    }
    e.mem.set_f32(data(2) + 4, 1.5);
    e.mem.set_f32(data(3) + 4, 2.5);
    e.mem.set_f32(data(3) + 8, 3.5);
    e.mem.set_u32(data(4) + 4, LVLI);
    string(&mut e, data(5) + 4, Some(b"Choose"));
    // The list: the first node inline in the perk, the rest at NODES.
    let mut node = PERK + 0x48;
    for i in 0..7 {
        e.mem.set_u32(node, entry(i));
        let next = if i < 6 { NODES + 8 * i } else { 0 };
        e.mem.set_u32(node + 4, next);
        node = next;
    }
    CALLS.with(|c| c.borrow_mut().clear());
    e
}

#[test]
fn entry_count_and_types() {
    let mut e = setup();
    assert_eq!(call(&mut e, 0x2267, &[r(1)]).1, 7.0);
    let kind = |e: &mut Engine, i: i32| call(e, 0x2268, &[r(1), n(i)]).1;
    let want = [6.0, 5.0, 1.0, 2.0, 3.0, 4.0, 0.0];
    for (i, k) in want.iter().enumerate() {
        assert_eq!(kind(&mut e, i as i32), *k, "entry {i}");
    }
    assert_eq!(kind(&mut e, 7), 0.0, "past the end");
}

#[test]
fn entry_points() {
    let mut e = setup();
    let get = |e: &mut Engine, i: i32| call(e, 0x2269, &[r(1), n(i)]).1;
    assert_eq!(get(&mut e, 2), 12.0);
    assert_eq!(get(&mut e, 0), -1.0, "a quest entry: -1");
    assert_eq!(get(&mut e, 9), -1.0, "past the end: -1");
    call(&mut e, 0x226a, &[r(1), n(2), n(0x1ff)]);
    assert_eq!(e.mem.u8(ENTRIES + 0x40 + 8), 0xff, "the low byte");
    assert_eq!(e.mem.u8(ENTRIES + 0x40 + 9), 0, "the next byte untouched");
    let before = e.mem.bytes(ENTRIES, 0x20);
    call(&mut e, 0x226a, &[r(1), n(0), n(5)]);
    assert_eq!(e.mem.bytes(ENTRIES, 0x20), before, "a quest entry: nothing");
}

#[test]
fn entry_forms() {
    let mut e = setup();
    let get = |e: &mut Engine, i: i32| {
        call(e, 0x226b, &[r(1), n(i)]);
        form_result(e)
    };
    assert_eq!(get(&mut e, 0), 0x0a11, "the quest");
    assert_eq!(get(&mut e, 1), 0x0b22, "the spell");
    assert_eq!(get(&mut e, 4), 0x0c33, "the leveled list");
    assert_eq!(get(&mut e, 2), 0, "one value data: none");
    call(&mut e, 0x226c, &[r(1), n(0), r(3)]);
    assert_eq!(e.mem.u32(ENTRIES + 8), QUEST, "a spell is no quest");
    call(&mut e, 0x226c, &[r(1), n(1), r(2)]);
    assert_eq!(e.mem.u32(ENTRIES + 0x28), SPELL, "a quest is no spell");
    e.mem.set_u32(ENTRIES + 0x28, 0);
    call(&mut e, 0x226c, &[r(1), n(1), r(3)]);
    assert_eq!(e.mem.u32(ENTRIES + 0x28), SPELL);
    e.mem.set_u32(ENTRIES + 8, 0);
    call(&mut e, 0x226c, &[r(1), n(0), r(2)]);
    assert_eq!(e.mem.u32(ENTRIES + 8), QUEST);
    let before = e.mem.bytes(ENTRIES + 0x40, 0x20);
    call(&mut e, 0x226c, &[r(1), n(2), r(2)]);
    assert_eq!(
        e.mem.bytes(ENTRIES + 0x40, 0x20),
        before,
        "an entry point: nothing"
    );
}

#[test]
fn entry_values() {
    let mut e = setup();
    let v1 = |e: &mut Engine, i: i32| call(e, 0x226d, &[r(1), n(i)]).1;
    let v2 = |e: &mut Engine, i: i32| call(e, 0x226f, &[r(1), n(i)]).1;
    assert_eq!(v1(&mut e, 2), 1.5);
    assert_eq!(v1(&mut e, 3), 2.5);
    assert_eq!(v1(&mut e, 0), 20.0, "the quest stage");
    assert_eq!(v1(&mut e, 4), -1.0, "leveled list data: -1");
    assert_eq!(v1(&mut e, 1), -1.0, "an ability entry: -1");
    assert_eq!(v2(&mut e, 3), 3.5);
    assert_eq!(v2(&mut e, 2), -1.0, "one value: -1");
    call(&mut e, 0x226e, &[r(1), n(3), z(-4.25)]);
    assert_eq!(e.mem.f32(DATA + 0x60 + 4), -4.25);
    call(&mut e, 0x226e, &[r(1), n(2), z(6.75)]);
    assert_eq!(e.mem.f32(DATA + 0x40 + 4), 6.75);
    call(&mut e, 0x2270, &[r(1), n(3), z(9.5)]);
    assert_eq!(e.mem.f32(DATA + 0x60 + 8), 9.5);
    call(&mut e, 0x2270, &[r(1), n(2), z(9.5)]);
    assert_eq!(
        e.mem.f32(DATA + 0x40 + 8),
        0.0,
        "one value data: no second value"
    );
    let before = e.mem.bytes(DATA + 0x80, 0x20);
    call(&mut e, 0x226e, &[r(1), n(4), z(1.0)]);
    assert_eq!(e.mem.bytes(DATA + 0x80, 0x20), before, "leveled list data");
    // The stage: a float into a byte, (int) then its low byte.
    call(&mut e, 0x226e, &[r(1), n(0), z(300.9)]);
    assert_eq!(e.mem.u8(ENTRIES + 0xc), 44);
    call(&mut e, 0x226e, &[r(1), n(0), z(-1.5)]);
    assert_eq!(e.mem.u8(ENTRIES + 0xc), 0xff, "-1's low byte");
    call(&mut e, 0x226e, &[r(1), n(0), z(f64::NAN)]);
    assert_eq!(e.mem.u8(ENTRIES + 0xc), 0, "NaN: INT_MIN's low byte");
    call(&mut e, 0x226e, &[r(1), n(0), z(3e9)]);
    assert_eq!(
        e.mem.u8(ENTRIES + 0xc),
        0,
        "past the range: INT_MIN's low byte"
    );
}

#[test]
fn the_label() {
    let mut e = setup();
    let (_, res) = call(&mut e, 0x2271, &[r(1), n(5)]);
    assert_eq!(string_result(&e, res), b"Choose");
    let (_, res) = call(&mut e, 0x2271, &[r(1), n(2)]);
    assert_eq!(string_result(&e, res), b"", "not a choice");
    call(&mut e, 0x2272, &[r(1), n(5), s(b"Pick")]);
    assert_eq!(text_at(&e, DATA + 0xa0 + 4), b"Pick");
    assert!(
        allocations().is_empty() && freed().is_empty(),
        "shorter: the same buffer"
    );
}

#[test]
fn trait_and_level() {
    let mut e = setup();
    e.mem.set_u8(PERK + 0x38, 2);
    e.mem.set_u8(PERK + 0x39, 14);
    assert_eq!(call(&mut e, 0x229a, &[r(1)]).1, 1.0);
    assert_eq!(call(&mut e, 0x230c, &[r(1)]).1, 14.0);
    call(&mut e, 0x28ae, &[r(1), n(0x102)]);
    assert_eq!(e.mem.u8(PERK + 0x39), 2, "the low byte");
    assert_eq!(e.mem.u8(PERK + 0x3a), 0, "the ranks untouched");
    e.mem.set_u8(PERK + 0x38, 0);
    assert_eq!(call(&mut e, 0x229a, &[r(1)]).1, 0.0);
}

#[test]
fn an_actors_rank() {
    let mut e = setup();
    e.mem.set_u32(ACTOR, ACTOR_VTBL);
    e.mem.set_u32(ACTOR_VTBL + 0x100, 0x008d_0360);
    e.mem.set_u32(ACTOR_VTBL + 0x4a0, 0x0050_2000);
    e.register(0x0050_2000, |_, w| {
        CALLS.with(|c| c.borrow_mut().push((0x0050_2000, w.to_vec())));
        Ret {
            eax: 0xffff_ff03,
            ..Ret::default()
        }
    });
    assert_eq!(
        call_on(&mut e, 0x221b, ACTOR, &[r(1), n(5)]).1,
        3.0,
        "the rank's byte"
    );
    let asked: Vec<_> = calls()
        .into_iter()
        .filter(|c| c.0 == 0x0050_2000)
        .map(|c| c.1)
        .collect();
    assert_eq!(asked, vec![vec![ACTOR, PERK, 1]]);
    assert_eq!(
        call_on(&mut e, 0x221b, ACTOR, &[r(5)]).1,
        0.0,
        "a misc item is no perk"
    );
    e.mem.set_u32(ACTOR_VTBL + 0x100, 0);
    assert_eq!(
        call_on(&mut e, 0x221b, ACTOR, &[r(1)]).1,
        0.0,
        "not an actor"
    );
}

/// The perk is read by type (`kParamType_Perk`) by the getters, as any form by the setters (then
/// `IS_ID`): a misc item holding the same list answers 0, -1 or "" and changes nothing.
#[test]
fn another_form_type_is_left_alone() {
    let mut e = setup();
    let list = e.mem.bytes(PERK + 0x48, 8);
    e.mem.write(MISC + 0x48, &list);
    e.mem.set_u8(MISC + 0x38, 1);
    e.mem.set_u8(MISC + 0x39, 7);
    let snap = |e: &Engine| {
        (
            e.mem.bytes(MISC, 0x50),
            e.mem.bytes(ENTRIES, 0xe0),
            e.mem.bytes(DATA, 0xe0),
        )
    };
    let before = snap(&e);
    assert_eq!(call(&mut e, 0x2267, &[r(5)]).1, 0.0);
    for i in 0..7 {
        assert_eq!(call(&mut e, 0x2268, &[r(5), n(i)]).1, 0.0, "type {i}");
        assert_eq!(call(&mut e, 0x2269, &[r(5), n(i)]).1, -1.0, "function {i}");
        call(&mut e, 0x226b, &[r(5), n(i)]);
        assert_eq!(form_result(&e), 0, "form {i}");
        assert_eq!(call(&mut e, 0x226d, &[r(5), n(i)]).1, -1.0, "value1 {i}");
        assert_eq!(call(&mut e, 0x226f, &[r(5), n(i)]).1, -1.0, "value2 {i}");
        let (_, res) = call(&mut e, 0x2271, &[r(5), n(i)]);
        assert_eq!(string_result(&e, res), b"", "label {i}");
        call(&mut e, 0x226a, &[r(5), n(i), n(1)]);
        call(&mut e, 0x226c, &[r(5), n(i), r(2)]);
        call(&mut e, 0x226e, &[r(5), n(i), z(1.0)]);
        call(&mut e, 0x2270, &[r(5), n(i), z(1.0)]);
        call(&mut e, 0x2272, &[r(5), n(i), s(b"x")]);
    }
    assert_eq!(call(&mut e, 0x229a, &[r(5)]).1, 0.0);
    assert_eq!(call(&mut e, 0x230c, &[r(5)]).1, 0.0);
    call(&mut e, 0x28ae, &[r(5), n(3)]);
    assert_eq!(snap(&e), before);
}
