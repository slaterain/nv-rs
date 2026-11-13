//! Tests of the armour commands, at the raw offsets of the armour loader (`00512FD0`): flags
//! +0x78, the four models +0x7C/+0x9C/+0xBC/+0xDC (their paths at +4, texture lists at +0x18),
//! the ragdoll constraint template's path +0x138, the template +0x188, override sounds +0x18C.

use super::*;
use crate::commands::test_support::*;

const ARMOR: u32 = FORMS + 0x1000;
const TEMPLATE: u32 = FORMS + 0x2000;
const ADDON: u32 = FORMS + 0x3000;
const MISC: u32 = FORMS + 0x4000;

fn setup() -> Engine {
    let mut e = engine();
    form(&mut e, ARMOR, 0x18, 0x0000_a4a0);
    e.mem.set_u32(ARMOR, 0x0102_a62c);
    form(&mut e, TEMPLATE, 0x18, 0x0000_7e30);
    e.mem.set_u32(TEMPLATE, 0x0102_a62c);
    form(&mut e, ADDON, 0x60, 0x0000_add0);
    e.mem.set_u32(ADDON, 0x0102_a31c);
    form(&mut e, MISC, 0x1f, 0x0000_01f1);
    for (i, f) in [ARMOR, TEMPLATE, ADDON, MISC].iter().enumerate() {
        set_ref(&mut e, i as u16 + 1, *f);
    }
    e
}

#[test]
fn the_ragdoll_constraint_template() {
    let mut e = setup();
    let (_, res) = call(&mut e, 0x27bc, &[r(1)]);
    assert_eq!(string_result(&e, res), b"", "no buffer");
    string(&mut e, ARMOR + 0x138, Some(b"rct.rdt"));
    let (_, res) = call(&mut e, 0x27bc, &[r(1)]);
    assert_eq!(string_result(&e, res), b"rct.rdt");
    call(&mut e, 0x27bd, &[r(1), s(b"other\\x.rdt")]);
    assert_eq!(text_at(&e, ARMOR + 0x138), b"other\\x.rdt");
    assert_eq!(e.mem.u16(ARMOR + 0x13c), 11, "length");
    // A misc item is read (any form) and refused by IS_TYPE.
    call(&mut e, 0x27bd, &[r(4), s(b"y")]);
    assert_eq!(e.mem.u32(MISC + 0x138), 0);
    let (_, res) = call(&mut e, 0x27bc, &[r(4)]);
    assert_eq!(string_result(&e, res), b"");
    // The vtable decides: an armour form type with the addon's vtable is no armour.
    e.mem.set_u32(ARMOR, 0x0102_a31c);
    let (_, res) = call(&mut e, 0x27bc, &[r(1)]);
    assert_eq!(string_result(&e, res), b"");
}

#[test]
fn the_biped_model_paths() {
    let mut e = setup();
    for which in 0..4u32 {
        let text = format!("m{which}.nif");
        call(&mut e, 0x2808, &[r(1), n(which as i32), s(text.as_bytes())]);
        assert_eq!(
            text_at(&e, ARMOR + 0x7c + 0x20 * which + 4),
            text.as_bytes(),
            "model {which}"
        );
    }
    let before = e.mem.bytes(ARMOR, 0x190);
    call(&mut e, 0x2808, &[r(1), n(4), s(b"m4.nif")]);
    assert_eq!(e.mem.bytes(ARMOR, 0x190), before, "index 4: nothing");
    call(&mut e, 0x2808, &[r(1), n(-1), s(b"m.nif")]);
    assert_eq!(
        e.mem.bytes(ARMOR, 0x190),
        before,
        "-1 is 0xFFFFFFFF: nothing"
    );
}

#[test]
fn a_path_longer_than_jips_buffer_stops_without_the_list_guard() {
    let mut e = setup();
    let long = vec![b'a'; 0x80];
    let stopped = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        call(&mut e, 0x27bd, &[r(1), s(&long)]);
    }));
    assert!(stopped.is_err());
}

#[test]
fn the_audio_template() {
    let mut e = setup();
    e.mem.set_u32(ARMOR + 0x188, TEMPLATE);
    call(&mut e, 0x281f, &[r(1)]);
    assert_eq!(form_result(&e), 0x7e30);
    e.mem.set_u8(ARMOR + 0x18c, 1);
    e.mem.set_f64(RESULT, 0.0);
    call(&mut e, 0x281f, &[r(1)]);
    assert_eq!(form_result(&e), 0, "own sounds: the word is a list");
    e.mem.set_u8(ARMOR + 0x18c, 0);
    e.mem.set_u32(ARMOR + 0x188, 0);
    call(&mut e, 0x281f, &[r(1)]);
    assert_eq!(form_result(&e), 0, "no template");

    // The template must be an armour with its own sounds.
    call(&mut e, 0x2820, &[r(1), r(2)]);
    assert_eq!(e.mem.u32(ARMOR + 0x188), 0, "template without own sounds");
    e.mem.set_u8(TEMPLATE + 0x18c, 1);
    call(&mut e, 0x2820, &[r(1), r(2)]);
    assert_eq!(e.mem.u32(ARMOR + 0x188), TEMPLATE);
    call(&mut e, 0x2820, &[r(1), r(4)]);
    assert_eq!(e.mem.u32(ARMOR + 0x188), TEMPLATE, "a misc item: nothing");
    call(&mut e, 0x2820, &[r(1)]);
    assert_eq!(e.mem.u32(ARMOR + 0x188), 0, "cleared");
    // An armour with its own sounds keeps its list.
    e.mem.set_u8(ARMOR + 0x18c, 1);
    e.mem.set_u32(ARMOR + 0x188, 0x0400_1234);
    call(&mut e, 0x2820, &[r(1), r(2)]);
    assert_eq!(e.mem.u32(ARMOR + 0x188), 0x0400_1234);
}

#[test]
fn armour_addons() {
    let mut e = setup();
    // A real addon (type 0x60) is no inventory object: xNVSE's reader refuses it for
    // `kParamType_ObjectID`, so the answer stays 0 (as with the DLL).
    assert_eq!(call(&mut e, 0x2835, &[r(3)]).1, 0.0);
    assert_eq!(call(&mut e, 0x2835, &[r(1)]).1, 0.0, "an armour");
    // The command's own test is the vtable: an inventory form with the addon's vtable.
    e.mem.set_u32(MISC, 0x0102_a31c);
    assert_eq!(call(&mut e, 0x2835, &[r(4)]).1, 1.0);
}

#[test]
fn the_armour_class() {
    let mut e = setup();
    let set = |e: &mut Engine, c: i32| {
        call(e, 0x2896, &[r(1), n(c)]);
    };
    e.mem.set_u32(ARMOR + 0x78, 0x8000_0001);
    set(&mut e, 2);
    assert_eq!(e.mem.u32(ARMOR + 0x78), 0x8000_0009, "medium");
    set(&mut e, 3);
    assert_eq!(e.mem.u32(ARMOR + 0x78), 0x8000_0081, "heavy");
    set(&mut e, 1);
    assert_eq!(e.mem.u32(ARMOR + 0x78), 0x8000_0001, "light: both cleared");
    e.mem.set_u32(ARMOR + 0x78, 0x88);
    set(&mut e, 0);
    assert_eq!(e.mem.u32(ARMOR + 0x78), 0x88, "0: nothing");
    set(&mut e, 4);
    assert_eq!(e.mem.u32(ARMOR + 0x78), 0x88, "4: nothing");
}

#[test]
fn alternate_textures_cleared() {
    let mut e = setup();
    // Model 0: three nodes (the first inline); model 3: one item; models 1, 2 empty.
    let list0 = ARMOR + 0x7c + 0x18;
    let (n1, n2) = (HEAP + 0x8000, HEAP + 0x8010);
    e.mem.set_u32(list0, 0x0500_0001);
    e.mem.set_u32(list0 + 4, n1);
    e.mem.set_u32(n1, 0x0500_0002);
    e.mem.set_u32(n1 + 4, n2);
    e.mem.set_u32(n2, 0x0500_0003);
    e.mem.set_u32(n2 + 4, 0);
    let list3 = ARMOR + 0xdc + 0x18;
    e.mem.set_u32(list3, 0x0500_0004);
    call(&mut e, 0x28ba, &[r(1)]);
    for list in [list0, list3] {
        assert_eq!(e.mem.u32(list), 0);
        assert_eq!(e.mem.u32(list + 4), 0);
    }
    assert_eq!(
        freed(),
        vec![n1, n2],
        "the nodes after the first, items kept"
    );
    // Not an armour: nothing.
    e.mem.set_u32(MISC + 0x7c + 0x18, 7);
    call(&mut e, 0x28ba, &[r(4)]);
    assert_eq!(e.mem.u32(MISC + 0x7c + 0x18), 7);
}
