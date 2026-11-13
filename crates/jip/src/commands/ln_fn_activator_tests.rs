//! Tests of the activator commands, at the raw offsets of the activator loader (`00510A90`:
//! looping sound +0x74, activation sound +0x78, water +0x80, radio station +0x84; the prompt
//! string at +0x88) and a placeable water's water (+0x4C). Setters compare the whole form
//! (activator 0x94 bytes, placeable water 0x54) with only the expected word changed.

use super::*;
use crate::commands::test_support::*;

const ACTI: u32 = FORMS + 0x1000;
const PWAT: u32 = FORMS + 0x1100;
const RADIO: u32 = FORMS + 0x2000;
const SOUND: u32 = FORMS + 0x2100;
const WATER: u32 = FORMS + 0x2200;
const MISC: u32 = FORMS + 0x2300;
const REFR: u32 = FORMS + 0x3000;
const ACTI_SIZE: u32 = 0x94;
const PWAT_SIZE: u32 = 0x54;

fn setup() -> Engine {
    let mut e = engine();
    form(&mut e, ACTI, 0x15, 0x0000_ac71);
    form(&mut e, PWAT, 0x23, 0x0000_9a7e);
    form(&mut e, RADIO, 0x16, 0x0000_4ad1);
    form(&mut e, SOUND, 0x0d, 0x0000_50d0);
    form(&mut e, WATER, 0x4e, 0x0000_3a7e);
    form(&mut e, MISC, 0x1f, 0x0000_0999);
    for (i, f) in [ACTI, PWAT, RADIO, SOUND, WATER, MISC].iter().enumerate() {
        set_ref(&mut e, i as u16 + 1, *f);
    }
    e.mem.set_u32(REFR + 0x20, ACTI);
    e
}

/// The `size` bytes of the form at `at` with the word at `off` set to `value`.
fn with_word(e: &Engine, at: u32, size: u32, off: u32, value: u32) -> Vec<u8> {
    let mut want = e.mem.bytes(at, size);
    let off = off as usize;
    want[off..off + 4].copy_from_slice(&value.to_le_bytes());
    want
}

#[test]
fn the_forms_by_argument_and_by_caller() {
    let mut e = setup();
    for (get, set, off, rf, value, id) in [
        (0x2273, 0x2274, 0x84, 3u16, RADIO, 0x4ad1u32),
        (0x2290, 0x2291, 0x74, 4, SOUND, 0x50d0),
        (0x2292, 0x2293, 0x78, 4, SOUND, 0x50d0),
    ] {
        call(&mut e, get, &[r(1)]);
        assert_eq!(form_result(&e), 0, "{get:04x}: none");
        let want = with_word(&e, ACTI, ACTI_SIZE, off, value);
        call(&mut e, set, &[r(rf), r(1)]);
        assert_eq!(e.mem.bytes(ACTI, ACTI_SIZE), want, "{set:04x}: by argument");
        call(&mut e, get, &[r(1)]);
        assert_eq!(form_result(&e), id);
        // The caller's base form when no activator is given.
        e.mem.set_u32(ACTI + off, 0);
        call_on(&mut e, set, REFR, &[r(rf)]);
        assert_eq!(e.mem.bytes(ACTI, ACTI_SIZE), want, "{set:04x}: by caller");
        call_on(&mut e, get, REFR, &[]);
        assert_eq!(form_result(&e), id, "{get:04x}: by caller");
        // A value of another type: nothing.
        call(&mut e, set, &[r(6), r(1)]);
        assert_eq!(e.mem.bytes(ACTI, ACTI_SIZE), want, "{set:04x}: a misc item");
    }
    // Neither an argument nor a caller: nothing.
    call(&mut e, 0x2273, &[]);
    assert_eq!(form_result(&e), 0);
}

#[test]
fn the_prompt() {
    let mut e = setup();
    let (_, res) = call(&mut e, 0x2275, &[r(1)]);
    assert_eq!(string_result(&e, res), b"", "no buffer");
    let first = heap_next(&e);
    call(&mut e, 0x2276, &[s(b"Open"), r(1)]);
    assert_eq!(text_at(&e, ACTI + 0x88), b"Open");
    assert_eq!(allocations(), vec![5], "the text and its 0");
    assert!(freed().is_empty(), "no buffer to free");
    assert_eq!(e.mem.u32(ACTI + 0x88), first);
    call_on(&mut e, 0x2276, REFR, &[s(b"Use")]);
    assert_eq!(text_at(&e, ACTI + 0x88), b"Use");
    assert_eq!(allocations(), vec![5], "a shorter text keeps the buffer");
    assert!(freed().is_empty(), "a shorter text frees nothing");
    let (_, res) = call_on(&mut e, 0x2275, REFR, &[]);
    assert_eq!(string_result(&e, res), b"Use");
    let second = heap_next(&e);
    call(&mut e, 0x2276, &[s(b"Search"), r(1)]);
    assert_eq!(text_at(&e, ACTI + 0x88), b"Search");
    assert_eq!(allocations(), vec![5, 7], "longer: a new buffer");
    assert_eq!(freed(), vec![first], "the old buffer freed");
    assert_eq!(e.mem.u32(ACTI + 0x88), second);
    assert_eq!(e.mem.u16(ACTI + 0x8c), 6, "the length");
    assert_eq!(e.mem.u16(ACTI + 0x8e), 6, "the buffer's size");
}

#[test]
fn the_water_of_an_activator_or_a_placeable_water() {
    let mut e = setup();
    let want = with_word(&e, ACTI, ACTI_SIZE, 0x80, WATER);
    call(&mut e, 0x2278, &[r(5), r(1)]);
    assert_eq!(e.mem.bytes(ACTI, ACTI_SIZE), want, "the activator's water");
    let want = with_word(&e, PWAT, PWAT_SIZE, 0x4c, WATER);
    call(&mut e, 0x2278, &[r(5), r(2)]);
    assert_eq!(
        e.mem.bytes(PWAT, PWAT_SIZE),
        want,
        "the placeable water's water"
    );
    for rf in [1, 2] {
        call(&mut e, 0x2277, &[r(rf)]);
        assert_eq!(form_result(&e), 0x3a7e, "form {rf}");
    }
    let before = (e.mem.bytes(ACTI, ACTI_SIZE), e.mem.bytes(PWAT, PWAT_SIZE));
    call(&mut e, 0x2278, &[r(6), r(1)]);
    call(&mut e, 0x2278, &[r(6), r(2)]);
    assert_eq!(
        (e.mem.bytes(ACTI, ACTI_SIZE), e.mem.bytes(PWAT, PWAT_SIZE)),
        before,
        "a misc item is no water"
    );
}

/// Every command takes its activator as any form: a misc item filled with a non-zero pattern
/// answers 0 or "" and keeps its bytes; every setter writes a value that differs from the fill.
#[test]
fn another_form_type_is_left_alone() {
    let mut e = setup();
    for off in (0x10..0x88).chain(0x90..ACTI_SIZE) {
        e.mem.set_u8(MISC + off, 0xa5);
    }
    string(&mut e, MISC + 0x88, Some(b"x"));
    let before = e.mem.bytes(MISC, ACTI_SIZE);
    for get in [0x2273, 0x2277, 0x2290, 0x2292] {
        call(&mut e, get, &[r(6)]);
        assert_eq!(form_result(&e), 0, "{get:04x}");
    }
    let (_, res) = call(&mut e, 0x2275, &[r(6)]);
    assert_eq!(string_result(&e, res), b"");
    call(&mut e, 0x2274, &[r(3), r(6)]);
    call(&mut e, 0x2278, &[r(5), r(6)]);
    call(&mut e, 0x2291, &[r(4), r(6)]);
    call(&mut e, 0x2293, &[r(4), r(6)]);
    call(&mut e, 0x2276, &[s(b"y"), r(6)]);
    call(&mut e, 0x2276, &[s(b"longer"), r(6)]);
    assert_eq!(e.mem.bytes(MISC, ACTI_SIZE), before);
}
