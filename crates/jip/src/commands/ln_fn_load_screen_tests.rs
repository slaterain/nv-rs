//! Tests of the load screen commands at the raw offsets (load screen: texture path +0x1C, type
//! +0x34; load screen type: first font colour +0x34), with a form of another type left alone.

use super::*;
use crate::commands::test_support::*;
use nvse::vars::Data;

const LSCR: u32 = FORMS + 0x1000;
const LSCT: u32 = FORMS + 0x2000;
const LSCT2: u32 = FORMS + 0x2100;
const MISC: u32 = FORMS + 0x3000;

fn setup() -> Engine {
    let mut e = engine();
    form(&mut e, LSCR, 0x4b, 0x0000_0100);
    form(&mut e, LSCT, 0x6e, 0x0000_0200);
    form(&mut e, LSCT2, 0x6e, 0x0000_0201);
    form(&mut e, MISC, 0x1f, 0x0000_0300);
    for (i, f) in [LSCR, LSCT, LSCT2, MISC].iter().enumerate() {
        set_ref(&mut e, i as u16 + 1, *f);
    }
    e
}

#[test]
fn the_texture() {
    let mut e = setup();
    let (_, res) = call(&mut e, 0x229b, &[r(1)]);
    assert_eq!(string_result(&e, res), b"", "no buffer");
    call(&mut e, 0x229c, &[r(1), s(b"textures\\ls.dds")]);
    assert_eq!(text_at(&e, LSCR + 0x1c), b"textures\\ls.dds");
    assert_eq!(e.mem.u16(LSCR + 0x20), 15, "the length");
    let (_, res) = call(&mut e, 0x229b, &[r(1)]);
    assert_eq!(string_result(&e, res), b"textures\\ls.dds");
    // Another form type: left alone, "".
    let before = e.mem.bytes(MISC, 0x40);
    call(&mut e, 0x229c, &[r(4), s(b"x.dds")]);
    assert_eq!(e.mem.bytes(MISC, 0x40), before);
    let (_, res) = call(&mut e, 0x229b, &[r(4)]);
    assert_eq!(string_result(&e, res), b"");
}

#[test]
fn the_type() {
    let mut e = setup();
    call(&mut e, 0x229d, &[r(1)]);
    assert_eq!(form_result(&e), 0, "none");
    call(&mut e, 0x229e, &[r(1), r(2)]);
    assert_eq!(e.mem.u32(LSCR + 0x34), LSCT);
    call(&mut e, 0x229d, &[r(1)]);
    assert_eq!(form_result(&e), 0x200);
    // The second form must be a load screen type, the first a load screen.
    call(&mut e, 0x229e, &[r(1), r(4)]);
    assert_eq!(e.mem.u32(LSCR + 0x34), LSCT);
    let before = e.mem.bytes(MISC, 0x40);
    call(&mut e, 0x229e, &[r(4), r(3)]);
    assert_eq!(e.mem.bytes(MISC, 0x40), before);
    call(&mut e, 0x229d, &[r(4)]);
    assert_eq!(form_result(&e), 0);
}

#[test]
fn the_text_colour() {
    let mut e = setup();
    call(&mut e, 0x22a0, &[r(2), z(0.25), z(0.5), z(1.1)]);
    assert_eq!(
        [0, 4, 8].map(|o| e.mem.f32(LSCT + 0x34 + o)),
        [0.25, 0.5, 1.1f32]
    );
    assert_eq!(e.mem.f32(LSCT + 0x40), 0.0, "the next field untouched");
    let (_, res) = call(&mut e, 0x229f, &[r(2)]);
    let a = nvse::vars::array(&e, res as u32).unwrap();
    assert_eq!(a.key_type, nvse::vars::KEY_STRING);
    assert_eq!(
        array_data(&e, res),
        // A string map sorts its keys: b, g, r.
        vec![
            Data::Number(f64::from(1.1f32)),
            Data::Number(0.5),
            Data::Number(0.25)
        ]
    );
    let before = e.mem.bytes(MISC, 0x48);
    call(&mut e, 0x22a0, &[r(4), z(1.0), z(1.0), z(1.0)]);
    assert_eq!(e.mem.bytes(MISC, 0x48), before);
    assert_eq!(call(&mut e, 0x229f, &[r(1)]).1, 0.0);
}
