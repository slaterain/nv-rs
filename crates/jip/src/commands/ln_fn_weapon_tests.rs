//! Tests of the LN weapon commands, at the raw offsets of the weapon loader (`0051F630`): kill
//! impulse +0x1A8 and impulse distance +0x1B8 (in `DNAM`), critical flags +0x1C8 (in `CRDT`),
//! the VATS effect +0x370 (`VATS`).

use super::*;
use crate::commands::test_support::*;

const WEAP: u32 = FORMS + 0x1000;
const SPEL: u32 = FORMS + 0x2000;
const MISC: u32 = FORMS + 0x3000;

fn setup() -> Engine {
    let mut e = engine();
    form(&mut e, WEAP, 0x28, 0x0000_3ea9);
    form(&mut e, SPEL, 0x14, 0x0000_05e1);
    form(&mut e, MISC, 0x1f, 0x0000_0999);
    for (i, f) in [WEAP, SPEL, MISC].iter().enumerate() {
        set_ref(&mut e, i as u16 + 1, *f);
    }
    e
}

#[test]
fn the_impulses_each_in_place() {
    let mut e = setup();
    for (get, set, off) in [(0x232b, 0x232c, 0x1a8usize), (0x232d, 0x232e, 0x1b8)] {
        e.mem.set_f32(WEAP + off as u32, 4.5);
        assert_eq!(call(&mut e, get, &[r(1)]).1, 4.5, "{get:04x}");
        let mut want = e.mem.bytes(WEAP, 0x388);
        call(&mut e, set, &[r(1), z(-12.25)]);
        want[off..off + 4].copy_from_slice(&(-12.25f32).to_le_bytes());
        assert_eq!(e.mem.bytes(WEAP, 0x388), want, "{set:04x}: only its float");
    }
}

#[test]
fn the_vats_effect() {
    let mut e = setup();
    call(&mut e, 0x232f, &[r(1)]);
    assert_eq!(form_result(&e), 0, "none");
    call(&mut e, 0x2330, &[r(1), r(2)]);
    assert_eq!(e.mem.u32(WEAP + 0x370), SPEL);
    call(&mut e, 0x232f, &[r(1)]);
    assert_eq!(form_result(&e), 0x05e1);
    call(&mut e, 0x2330, &[r(1), r(3)]);
    assert_eq!(e.mem.u32(WEAP + 0x370), SPEL, "a misc item is no spell");
}

#[test]
fn the_critical_flags() {
    let mut e = setup();
    e.mem.set_u32(WEAP + 0x1c8, 0xaaaa_aa05);
    assert_eq!(call(&mut e, 0x2331, &[r(1)]).1, 5.0, "the byte");
    // critDamageFlags = flags: the UInt32's low byte.
    call(&mut e, 0x2332, &[r(1), n(0x1ff)]);
    assert_eq!(e.mem.u32(WEAP + 0x1c8), 0xaaaa_aaff, "one byte written");
    call(&mut e, 0x2332, &[r(1), n(-2)]);
    assert_eq!(e.mem.u8(WEAP + 0x1c8), 0xfe);
}

/// Another inventory form (a misc item: the reader's `kParamType_ObjectID` passes it, `IS_ID`
/// refuses it) holding the same bytes answers 0 and keeps them through every command.
#[test]
fn another_form_type_is_left_alone() {
    let mut e = setup();
    e.mem.set_f32(MISC + 0x1a8, 1.0);
    e.mem.set_f32(MISC + 0x1b8, 1.0);
    e.mem.set_u8(MISC + 0x1c8, 1);
    e.mem.set_u32(MISC + 0x370, SPEL);
    let before = e.mem.bytes(MISC, 0x388);
    for get in [0x232b, 0x232d, 0x2331] {
        assert_eq!(call(&mut e, get, &[r(3)]).1, 0.0, "{get:04x}");
    }
    call(&mut e, 0x232f, &[r(3)]);
    assert_eq!(form_result(&e), 0);
    call(&mut e, 0x232c, &[r(3), z(2.0)]);
    call(&mut e, 0x232e, &[r(3), z(2.0)]);
    call(&mut e, 0x2330, &[r(3), r(2)]);
    call(&mut e, 0x2332, &[r(3), n(7)]);
    assert_eq!(e.mem.bytes(MISC, 0x388), before);
}
