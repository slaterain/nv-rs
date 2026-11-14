//! Tests of the radio commands on a reference with an `ExtraRadioData` (type 0x68: radius +0xC,
//! range type +0x10, static +0x14, position reference +0x18).

use super::*;
use crate::commands::test_support::*;

const RADIO: u32 = FORMS + 0x1000;
const PLAIN: u32 = FORMS + 0x1100;
const POS: u32 = FORMS + 0x1200;
const X: u32 = FORMS + 0x2000;

fn setup() -> Engine {
    let mut e = engine();
    extra_stand_ins(&mut e);
    form(&mut e, RADIO, 0x3a, 0x0000_0100);
    form(&mut e, PLAIN, 0x3a, 0x0000_0101);
    form(&mut e, POS, 0x3a, 0x0000_0102);
    put_extra(&mut e, RADIO + 0x44, X, 0x68, 500.0f32.to_bits());
    e.mem.set_f32(X + 0x14, 0.25);
    e.mem.set_u32(X + 0x18, POS);
    set_ref(&mut e, 1, POS);
    e
}

#[test]
fn is_a_radio() {
    let mut e = setup();
    assert_eq!(call_on(&mut e, 0x2245, RADIO, &[]).1, 1.0);
    assert_eq!(call_on(&mut e, 0x2245, PLAIN, &[]).1, 0.0);
    // The presence bit decides (bit 0 of byte 0x68 >> 3 = 13).
    e.mem.set_u8(PLAIN + 0x44 + 8 + 13, 1);
    assert_eq!(call_on(&mut e, 0x2245, PLAIN, &[]).1, 1.0);
}

#[test]
fn radius_static_and_position_for_range_type_0() {
    let mut e = setup();
    assert_eq!(call_on(&mut e, 0x223d, RADIO, &[]).1, 0.0);
    assert_eq!(call_on(&mut e, 0x223f, RADIO, &[]).1, 500.0);
    assert_eq!(call_on(&mut e, 0x2241, RADIO, &[]).1, 0.25);
    call_on(&mut e, 0x2243, RADIO, &[]);
    assert_eq!(form_result(&e), 0x102);
    call_on(&mut e, 0x2240, RADIO, &[z(800.0)]);
    call_on(&mut e, 0x2242, RADIO, &[z(0.5)]);
    assert_eq!((e.mem.f32(X + 0xc), e.mem.f32(X + 0x14)), (800.0, 0.5));
    // Negative and NaN are refused.
    call_on(&mut e, 0x2240, RADIO, &[z(-1.0)]);
    call_on(&mut e, 0x2242, RADIO, &[z(f64::NAN)]);
    assert_eq!((e.mem.f32(X + 0xc), e.mem.f32(X + 0x14)), (800.0, 0.5));
    call_on(&mut e, 0x2244, RADIO, &[r(1)]);
    assert_eq!(e.mem.u32(X + 0x18), POS);
    // Nothing on a reference with no radio data.
    assert_eq!(call_on(&mut e, 0x223f, PLAIN, &[]).1, 0.0);
    call_on(&mut e, 0x2240, PLAIN, &[z(1.0)]);
}

#[test]
fn another_range_type() {
    let mut e = setup();
    call_on(&mut e, 0x223e, RADIO, &[n(2)]);
    assert_eq!(e.mem.u32(X + 0x10), 2);
    assert_eq!(
        (e.mem.f32(X + 0xc), e.mem.f32(X + 0x14)),
        (0.0, 0.0),
        "zeroed"
    );
    assert_eq!(e.mem.u32(X + 0x18), POS, "the position reference stays");
    assert_eq!(call_on(&mut e, 0x223d, RADIO, &[]).1, 2.0);
    // Radius, static and position only for type 0.
    e.mem.set_f32(X + 0xc, 9.0);
    assert_eq!(call_on(&mut e, 0x223f, RADIO, &[]).1, 0.0);
    assert_eq!(call_on(&mut e, 0x2241, RADIO, &[]).1, 0.0);
    call_on(&mut e, 0x2243, RADIO, &[]);
    assert_eq!(form_result(&e), 0);
    call_on(&mut e, 0x2240, RADIO, &[z(5.0)]);
    assert_eq!(e.mem.f32(X + 0xc), 9.0);
    e.mem.set_u32(X + 0x18, 0);
    call_on(&mut e, 0x2244, RADIO, &[r(1)]);
    assert_eq!(e.mem.u32(X + 0x18), 0);
    // Type 5 is refused; back to 0 keeps the values.
    call_on(&mut e, 0x223e, RADIO, &[n(5)]);
    assert_eq!(e.mem.u32(X + 0x10), 2);
    call_on(&mut e, 0x223e, RADIO, &[n(0)]);
    assert_eq!((e.mem.u32(X + 0x10), e.mem.f32(X + 0xc)), (0, 9.0));
}
