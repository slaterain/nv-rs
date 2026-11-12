//! Tests of the climate commands. The climates are built at `TESClimate`'s offsets as JIP's
//! header and `TESClimate::Load` (`0052AC90`) place them (night sky model +0x18, weather list
//! +0x30, sun +0x38, glare +0x44, times +0x50..+0x55); the expected answers are worked out from
//! JIP's C++.

use super::*;
use crate::commands::test_support::*;
use std::cell::RefCell;

const CLIMATE: u32 = FORMS + 0x1000;
const WEATHER_A: u32 = FORMS + 0x2000;
const WEATHER_B: u32 = FORMS + 0x2100;
const WEATHER_C: u32 = FORMS + 0x2200;
const GLOBAL: u32 = FORMS + 0x2300;
const ENTRIES: u32 = FORMS + 0x3000;
const NODES: u32 = FORMS + 0x3800;
const SKY_OBJECT: u32 = FORMS + 0x8000;

fn setup() -> Engine {
    let mut e = engine();
    form(&mut e, CLIMATE, 0x36, 0x0001_2345);
    form(&mut e, WEATHER_A, 0x35, 0x0000_aaaa);
    form(&mut e, WEATHER_B, 0x35, 0x0000_bbbb);
    form(&mut e, WEATHER_C, 0x35, 0x0000_cccc);
    form(&mut e, GLOBAL, 0x06, 0x0000_0606);
    model(&mut e, CLIMATE + 0x18, Some(b"sky\\night.nif"));
    string(&mut e, CLIMATE + 0x3c, Some(b"sky\\sun.dds"));
    string(&mut e, CLIMATE + 0x48, Some(b"sky\\glare.dds"));
    set_ref(&mut e, 1, CLIMATE);
    set_ref(&mut e, 2, WEATHER_A);
    set_ref(&mut e, 3, WEATHER_B);
    set_ref(&mut e, 4, WEATHER_C);
    set_ref(&mut e, 5, GLOBAL);
    e.mem.set_u32(0x011d_ea20, SKY_OBJECT);
    CALLS.with(|c| c.borrow_mut().clear());
    e
}

/// A reference index whose variable holds no form: the read fails.
const NO_FORM: u16 = 9;

/// The weather list as entries (weather, chance, global) in nodes from the inline first node.
fn set_weathers(e: &mut Engine, entries: &[(u32, u32, u32)]) {
    e.mem.set_u32(CLIMATE + 0x30, 0);
    e.mem.set_u32(CLIMATE + 0x34, 0);
    for (i, &(weather, chance, global)) in entries.iter().enumerate() {
        let entry = ENTRIES + 0x10 * i as u32;
        e.mem.set_u32(entry, weather);
        e.mem.set_u32(entry + 4, chance);
        e.mem.set_u32(entry + 8, global);
        let node = if i == 0 {
            CLIMATE + 0x30
        } else {
            NODES + 8 * i as u32
        };
        e.mem.set_u32(node, entry);
        e.mem.set_u32(node + 4, 0);
        if i > 0 {
            let prev = if i == 1 {
                CLIMATE + 0x30
            } else {
                NODES + 8 * (i as u32 - 1)
            };
            e.mem.set_u32(prev + 4, node);
        }
    }
}

/// The list's items, from the first node.
fn items(e: &Engine) -> Vec<u32> {
    let mut out = Vec::new();
    let mut node = CLIMATE + 0x30;
    while node != 0 {
        out.push(e.mem.u32(node));
        node = e.mem.u32(node + 4);
    }
    out
}

#[test]
fn the_sun_textures() {
    let mut e = setup();
    let (ok, res) = call(&mut e, 0x266b, &[r(1)]);
    assert_eq!(ok, 1);
    assert_eq!(string_result(&e, res), b"sky\\sun.dds");
    let (_, res) = call(&mut e, 0x266d, &[r(1)]);
    assert_eq!(string_result(&e, res), b"sky\\glare.dds");
    // Not a climate, no form, or no buffer: "".
    let (_, res) = call(&mut e, 0x266b, &[r(2)]);
    assert_eq!(string_result(&e, res), b"");
    let (ok, res) = call(&mut e, 0x266d, &[r(NO_FORM)]);
    assert_eq!((ok, string_result(&e, res)), (1, b"".to_vec()));
    string(&mut e, CLIMATE + 0x3c, None);
    let (_, res) = call(&mut e, 0x266b, &[r(1)]);
    assert_eq!(string_result(&e, res), b"");
}

/// `String::Set`: a text that fits stays in the buffer; a longer one gets a new buffer (the old
/// freed); an empty one empties it.
#[test]
fn the_sun_textures_set() {
    let mut e = setup();
    let old = e.mem.u32(CLIMATE + 0x3c);
    call(&mut e, 0x266c, &[r(1), s(b"a.dds")]);
    assert_eq!(e.mem.u32(CLIMATE + 0x3c), old, "fits: the same buffer");
    assert_eq!(text_at(&e, CLIMATE + 0x3c), b"a.dds");
    assert_eq!(e.mem.u16(CLIMATE + 0x40), 5);
    assert_eq!(e.mem.u16(CLIMATE + 0x42), 11, "the buffer's size stays");
    assert!(freed().is_empty());
    call(&mut e, 0x266c, &[r(1), s(b"sky\\a_longer_name.dds")]);
    assert_ne!(e.mem.u32(CLIMATE + 0x3c), old);
    assert_eq!(freed(), vec![old]);
    assert_eq!(text_at(&e, CLIMATE + 0x3c), b"sky\\a_longer_name.dds");
    assert_eq!(
        (e.mem.u16(CLIMATE + 0x40), e.mem.u16(CLIMATE + 0x42)),
        (21, 21)
    );
    let glare = e.mem.u32(CLIMATE + 0x48);
    call(&mut e, 0x266e, &[r(1), s(b"")]);
    assert_eq!(e.mem.u32(CLIMATE + 0x48), glare);
    assert_eq!(e.mem.u8(glare), 0);
    assert_eq!(e.mem.u16(CLIMATE + 0x4c), 0);
    // Not a climate: nothing.
    call(&mut e, 0x266e, &[r(2), s(b"x")]);
    assert_eq!(e.mem.u16(WEATHER_A + 0x4c), 0);
}

/// An empty text into a string with no buffer leaves it without one; a text into no buffer gets
/// a new one of its length plus one.
#[test]
fn an_empty_text_needs_no_buffer() {
    let mut e = setup();
    string(&mut e, CLIMATE + 0x3c, None);
    call(&mut e, 0x266c, &[r(1), s(b"")]);
    assert_eq!(e.mem.u32(CLIMATE + 0x3c), 0);
    call(&mut e, 0x266c, &[r(1), s(b"abc")]);
    assert_eq!(text_at(&e, CLIMATE + 0x3c), b"abc");
    assert!(calls().iter().any(|c| c.0 == 0x00aa_3e40 && c.1[1] == 4));
    assert!(freed().is_empty());
}

#[test]
fn the_night_sky_model_through_the_virtual_functions() {
    let mut e = setup();
    let (_, res) = call(&mut e, 0x266f, &[r(1)]);
    assert_eq!(string_result(&e, res), b"sky\\night.nif");
    assert!(calls()
        .iter()
        .any(|c| c.0 == 0x0040_8da0 && c.1[0] == CLIMATE + 0x18));
    call(&mut e, 0x2670, &[r(1), s(b"sky\\other.nif")]);
    let set = calls()
        .into_iter()
        .find(|c| c.0 == 0x0048_9100)
        .expect("SetModelPath");
    assert_eq!(set.1[0], CLIMATE + 0x18);
    assert_eq!(text_at(&e, CLIMATE + 0x1c), b"sky\\other.nif");
    // Not a climate: neither function is called.
    let mut e = setup();
    let (_, res) = call(&mut e, 0x266f, &[r(2)]);
    assert_eq!(string_result(&e, res), b"");
    call(&mut e, 0x2670, &[r(2), s(b"x")]);
    assert!(calls().is_empty());
}

#[test]
#[should_panic(expected = "runs past JIP's 128-byte buffer")]
fn a_path_longer_than_the_buffer_stops() {
    let mut e = setup();
    call(&mut e, 0x2670, &[r(1), s(&[b'a'; 128])]);
}

#[test]
fn the_numeric_traits() {
    let mut e = setup();
    for (i, v) in [36u8, 60, 108, 132, 75, 0xc5].iter().enumerate() {
        e.mem.set_u8(CLIMATE + 0x50 + i as u32, *v);
    }
    let get = |e: &mut Engine, t: i32| call(e, 0x2671, &[r(1), n(t)]).1;
    assert_eq!(get(&mut e, 0), 360.0);
    assert_eq!(get(&mut e, 1), 600.0);
    assert_eq!(get(&mut e, 2), 1080.0);
    assert_eq!(get(&mut e, 3), 1320.0);
    assert_eq!(get(&mut e, 4), 75.0);
    assert_eq!(get(&mut e, 5), 3.0, "0xC5 >> 6");
    assert_eq!(get(&mut e, 6), 5.0, "0xC5 & 63");
    assert_eq!(get(&mut e, 7), 0.0);
    assert_eq!(get(&mut e, -1), 0.0);
    assert_eq!(call(&mut e, 0x2671, &[r(2), n(4)]).1, 0.0, "not a climate");
}

#[test]
fn the_numeric_traits_set() {
    let mut e = setup();
    e.mem.set_u8(CLIMATE + 0x55, 0xc5);
    let set = |e: &mut Engine, t: i32, v: i32| {
        call(e, 0x2672, &[r(1), n(t), n(v)]);
    };
    set(&mut e, 0, 1431);
    assert_eq!(e.mem.u8(CLIMATE + 0x50), 143);
    set(&mut e, 1, 1430);
    assert_eq!(e.mem.u8(CLIMATE + 0x51), 143);
    set(&mut e, 2, 655);
    assert_eq!(e.mem.u8(CLIMATE + 0x52), 65);
    set(&mut e, 3, -1);
    assert_eq!(
        e.mem.u8(CLIMATE + 0x53),
        143,
        "an unsigned value above 1430"
    );
    set(&mut e, 4, 300);
    assert_eq!(e.mem.u8(CLIMATE + 0x54), 255);
    set(&mut e, 4, 200);
    assert_eq!(e.mem.u8(CLIMATE + 0x54), 200);
    set(&mut e, 5, 2);
    assert_eq!(e.mem.u8(CLIMATE + 0x55), 5 + 128);
    set(&mut e, 5, 9);
    assert_eq!(e.mem.u8(CLIMATE + 0x55), 5 + 192);
    set(&mut e, 6, 70);
    assert_eq!(e.mem.u8(CLIMATE + 0x55), 192 + 63);
    set(&mut e, 6, 7);
    assert_eq!(e.mem.u8(CLIMATE + 0x55), 192 + 7);
    let before: Vec<u8> = (0..6).map(|i| e.mem.u8(CLIMATE + 0x50 + i)).collect();
    set(&mut e, 7, 10);
    let after: Vec<u8> = (0..6).map(|i| e.mem.u8(CLIMATE + 0x50 + i)).collect();
    assert_eq!(before, after, "trait 7 writes nothing");
    call(&mut e, 0x2672, &[r(2), n(4), n(9)]);
    assert_eq!(e.mem.u8(WEATHER_A + 0x54), 0, "not a climate");
}

#[test]
fn the_weather_list_read() {
    let mut e = setup();
    set_weathers(&mut e, &[]);
    assert_eq!(call(&mut e, 0x2673, &[r(1)]).1, 0.0);
    set_weathers(
        &mut e,
        &[(WEATHER_A, 30, 0), (WEATHER_B, 50, GLOBAL), (0, 20, 0)],
    );
    assert_eq!(call(&mut e, 0x2673, &[r(1)]).1, 3.0);
    let type_of_nth = |e: &mut Engine, i: i32| {
        call(e, 0x2674, &[r(1), n(i)]);
        form_result(e)
    };
    assert_eq!(type_of_nth(&mut e, 0), 0xaaaa);
    assert_eq!(type_of_nth(&mut e, 1), 0xbbbb);
    assert_eq!(type_of_nth(&mut e, 2), 0, "an entry with no weather");
    assert_eq!(type_of_nth(&mut e, 3), 0, "past the end");
    assert_eq!(type_of_nth(&mut e, -1), 0, "a negative index");
    assert_eq!(call(&mut e, 0x2675, &[r(1), n(1)]).1, 50.0);
    assert_eq!(call(&mut e, 0x2675, &[r(1), n(2)]).1, 20.0);
    assert_eq!(call(&mut e, 0x2675, &[r(1), n(3)]).1, 0.0);
    call(&mut e, 0x2676, &[r(1), n(1)]);
    assert_eq!(form_result(&e), 0x0606);
    call(&mut e, 0x2676, &[r(1), n(0)]);
    assert_eq!(form_result(&e), 0, "no global");
    assert_eq!(call(&mut e, 0x2673, &[r(2)]).1, 0.0, "not a climate");
    assert_eq!(call(&mut e, 0x2675, &[r(2), n(0)]).1, 0.0);
}

#[test]
fn a_weather_added() {
    let mut e = setup();
    set_weathers(&mut e, &[(WEATHER_A, 30, 0), (WEATHER_B, 50, GLOBAL)]);
    // An entry it has: updated in place.
    let (_, res) = call(&mut e, 0x2677, &[r(1), r(2), n(150), r(5)]);
    assert_eq!(res, 1.0);
    assert_eq!(
        (e.mem.u32(ENTRIES + 4), e.mem.u32(ENTRIES + 8)),
        (150 % 101, GLOBAL)
    );
    // No global given: cleared.
    call(&mut e, 0x2677, &[r(1), r(3), n(10)]);
    assert_eq!(e.mem.u32(ENTRIES + 0x10 + 8), 0);
    assert_eq!(items(&e), vec![ENTRIES, ENTRIES + 0x10]);
    // A new one: a 12-byte entry first, the old first item in a new node after it.
    let (_, res) = call(&mut e, 0x2677, &[r(1), r(4), n(5)]);
    assert_eq!(res, 1.0);
    let list = items(&e);
    assert_eq!(list.len(), 3);
    assert_eq!(&list[1..], &[ENTRIES, ENTRIES + 0x10]);
    let new = list[0];
    assert_eq!(
        (e.mem.u32(new), e.mem.u32(new + 4), e.mem.u32(new + 8)),
        (WEATHER_C, 5, 0)
    );
    let allocs: Vec<u32> = calls()
        .iter()
        .filter(|c| c.0 == 0x00aa_3e40)
        .map(|c| c.1[1])
        .collect();
    assert_eq!(allocs, vec![12, 8], "the entry, then the node");
    // Into an empty list: the first node takes it.
    set_weathers(&mut e, &[]);
    call(&mut e, 0x2677, &[r(1), r(2), n(1)]);
    assert_eq!(items(&e).len(), 1);
    assert_eq!(e.mem.u32(e.mem.u32(CLIMATE + 0x30)), WEATHER_A);
    // Not a climate: nothing, result 0.
    assert_eq!(call(&mut e, 0x2677, &[r(3), r(2), n(1)]).1, 0.0);
}

#[test]
fn a_weather_removed() {
    // The first of several: the next node's item and link come into the first node, the node
    // is freed, then the entry.
    let mut e = setup();
    set_weathers(
        &mut e,
        &[(WEATHER_A, 1, 0), (WEATHER_B, 2, 0), (WEATHER_C, 3, 0)],
    );
    let (_, res) = call(&mut e, 0x2678, &[r(1), r(2)]);
    assert_eq!(res, 1.0);
    assert_eq!(items(&e), vec![ENTRIES + 0x10, ENTRIES + 0x20]);
    assert_eq!(freed(), vec![NODES + 8, ENTRIES]);
    // A later one: unlinked.
    let mut e = setup();
    set_weathers(
        &mut e,
        &[(WEATHER_A, 1, 0), (WEATHER_B, 2, 0), (WEATHER_C, 3, 0)],
    );
    call(&mut e, 0x2678, &[r(1), r(3)]);
    assert_eq!(items(&e), vec![ENTRIES, ENTRIES + 0x20]);
    assert_eq!(freed(), vec![NODES + 8, ENTRIES + 0x10]);
    // A lone first node: emptied.
    let mut e = setup();
    set_weathers(&mut e, &[(WEATHER_A, 1, 0)]);
    call(&mut e, 0x2678, &[r(1), r(2)]);
    assert_eq!(items(&e), vec![0]);
    assert_eq!(freed(), vec![ENTRIES]);
    // Not there: nothing, result 0.
    let mut e = setup();
    set_weathers(&mut e, &[(WEATHER_A, 1, 0)]);
    assert_eq!(call(&mut e, 0x2678, &[r(1), r(3)]).1, 0.0);
    assert!(freed().is_empty());
    assert_eq!(call(&mut e, 0x2678, &[r(2), r(2)]).1, 0.0, "not a climate");
}

#[test]
fn the_current_climate() {
    let mut e = setup();
    call(&mut e, 0x26d0, &[]);
    assert_eq!(form_result(&e), 0, "none");
    e.mem.set_u32(SKY_OBJECT + 0xc, CLIMATE);
    call(&mut e, 0x26d0, &[]);
    assert_eq!(form_result(&e), 0x0001_2345);
    // Set: only to a climate.
    call(&mut e, 0x26d1, &[r(2)]);
    assert_eq!(e.mem.u32(SKY_OBJECT + 0xc), CLIMATE);
    let other = FORMS + 0x4000;
    form(&mut e, other, 0x36, 0x77);
    set_ref(&mut e, 6, other);
    call(&mut e, 0x26d1, &[r(6)]);
    assert_eq!(e.mem.u32(SKY_OBJECT + 0xc), other);
    call(&mut e, 0x26d1, &[r(NO_FORM)]);
    assert_eq!(e.mem.u32(SKY_OBJECT + 0xc), other);
    // Refresh: the game's Sky::SetCurrentClimate(sky, current, true).
    call(&mut e, 0x26d2, &[]);
    let refresh = calls()
        .into_iter()
        .find(|c| c.0 == 0x0063_c8f0)
        .expect("SetCurrentClimate");
    assert_eq!(refresh.1, vec![SKY_OBJECT, other, 1]);
}

thread_local! {
    static LINES: RefCell<Vec<Vec<u8>>> = const { RefCell::new(Vec::new()) };
}

/// JIP's form echo: `GetCurrentClimate >> 12345 "Name"`, the id alone without an editor ID,
/// `0` with no climate; nothing outside the console.
#[test]
fn the_current_climate_echoed() {
    const CONSOLE: u32 = 0x0200_f000;
    const NAME: u32 = FORMS + 0x5000;
    let mut e = setup();
    LINES.with(|l| l.borrow_mut().clear());
    e.register(0x0071_b160, |_, _| Ret {
        eax: CONSOLE,
        ..Ret::default()
    });
    e.register(0x0071_d0a0, |e, a| {
        LINES.with(|l| l.borrow_mut().push(nvse::table::guest_text(e, a[1])));
        Ret::default()
    });
    // EvaluateLine as the echo sees it: the handler called from it.
    e.register(0x005e_1550, |e, w| e.call(w[0], &w[1..9]));
    // The climate's vtable: its editor ID at +0x130.
    let vtbl = FORMS + 0x6000;
    e.mem.set_u32(CLIMATE, vtbl);
    e.mem.set_u32(vtbl + 0x130, 0x0050_0000);
    e.register(0x0050_0000, |_, _| Ret {
        eax: NAME,
        ..Ret::default()
    });
    e.mem.write(NAME, b"WastelandClimate\0");
    console(&mut e, true);
    let run = |e: &mut Engine| {
        let words = crate::test_support::line(e, 0x26d0, &[]);
        let execute = crate::test_support::execute_of(e, 0x26d0);
        let mut all = vec![execute];
        all.extend_from_slice(&words);
        e.call(0x005e_1550, &all);
    };
    run(&mut e);
    e.mem.set_u32(SKY_OBJECT + 0xc, CLIMATE);
    run(&mut e);
    e.mem.write(NAME, b"\0");
    run(&mut e);
    console(&mut e, false);
    run(&mut e);
    let lines: Vec<String> = LINES
        .with(|l| l.borrow().clone())
        .into_iter()
        .map(|l| String::from_utf8(l).unwrap())
        .collect();
    assert_eq!(
        lines,
        vec![
            "GetCurrentClimate >> 0",
            "GetCurrentClimate >> 12345 \"WastelandClimate\"",
            "GetCurrentClimate >> 12345",
        ]
    );
}
