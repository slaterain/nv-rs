//! Tests of the map marker commands, on references: the base form at +0x20 (id 0x10 for the
//! marker static), the extra data list at +0x44, an `ExtraMapMarker` (type 0x2C) holding the
//! marker data at +0xC, and the marker data at the raw offsets of `MapMarkerData::Load`
//! (`00438CA0`): name string +4, flags +0xC, type +0xE, reputation +0x10. The game's
//! `GetByType` (`00410220`) is a stand-in written from its walk: the extras from the list's +4,
//! each with its type at +4 and the next at +8.

use super::*;
use crate::commands::test_support::*;

const MARKER_STATIC: u32 = FORMS + 0x1000;
const OTHER_STATIC: u32 = FORMS + 0x1100;
const REFR: u32 = FORMS + 0x2000;
const PLAIN: u32 = FORMS + 0x2100;
const EXTRA: u32 = FORMS + 0x3000;
const OTHER_EXTRA: u32 = FORMS + 0x3100;
const DATA: u32 = FORMS + 0x4000;
const REP: u32 = FORMS + 0x5000;
const MISC: u32 = FORMS + 0x5100;

fn setup() -> Engine {
    let mut e = engine();
    form(&mut e, MARKER_STATIC, 0x20, 0x0000_0010);
    form(&mut e, OTHER_STATIC, 0x20, 0x0000_0011);
    form(&mut e, REP, 0x68, 0x0000_4e90);
    form(&mut e, MISC, 0x1f, 0x0000_0999);
    set_ref(&mut e, 1, REP);
    set_ref(&mut e, 2, MISC);
    e.mem.set_u32(REFR + 0x20, MARKER_STATIC);
    // The list: another extra first, then the marker.
    e.mem.set_u32(REFR + 0x44 + 4, OTHER_EXTRA);
    e.mem.set_u8(OTHER_EXTRA + 4, 0x2b);
    e.mem.set_u32(OTHER_EXTRA + 8, EXTRA);
    e.mem.set_u8(EXTRA + 4, 0x2c);
    e.mem.set_u32(EXTRA + 0xc, DATA);
    e.mem.set_u32(PLAIN + 0x20, OTHER_STATIC);
    e.register(0x0041_0220, |e, w| {
        let mut x = e.mem.u32(w[0] + 4);
        while x != 0 && u32::from(e.mem.u8(x + 4)) != w[1] {
            x = e.mem.u32(x + 8);
        }
        Ret {
            eax: x,
            ..Ret::default()
        }
    });
    e
}

#[test]
fn is_a_map_marker() {
    let mut e = setup();
    assert_eq!(call_on(&mut e, 0x2249, REFR, &[]).1, 1.0);
    assert_eq!(call_on(&mut e, 0x2249, PLAIN, &[]).1, 0.0);
}

#[test]
fn the_name() {
    let mut e = setup();
    let (_, res) = call_on(&mut e, 0x222f, REFR, &[]);
    assert_eq!(string_result(&e, res), b"", "no buffer");
    call_on(&mut e, 0x2230, REFR, &[s(b"Goodsprings")]);
    assert_eq!(text_at(&e, DATA + 4), b"Goodsprings");
    // JIP's `String::Set`: no buffer yet, so one of the text's length and its 0, nothing freed.
    assert_eq!(allocations(), vec![12]);
    assert!(freed().is_empty());
    let first = e.mem.u32(DATA + 4);
    // A longer name: the old buffer freed, one of the new length and its 0.
    call_on(&mut e, 0x2230, REFR, &[s(b"Goodsprings Town")]);
    assert_eq!(allocations(), vec![12, 17]);
    assert_eq!(freed(), vec![first]);
    let second = e.mem.u32(DATA + 4);
    // A shorter one: written into the same buffer, nothing allocated or freed.
    call_on(&mut e, 0x2230, REFR, &[s(b"Sloan")]);
    assert_eq!((allocations(), freed()), (vec![12, 17], vec![first]));
    assert_eq!(e.mem.u32(DATA + 4), second);
    assert_eq!(text_at(&e, DATA + 4), b"Sloan");
    call_on(&mut e, 0x2230, REFR, &[s(b"Goodsprings")]);
    let (_, res) = call_on(&mut e, 0x222f, REFR, &[]);
    assert_eq!(string_result(&e, res), b"Goodsprings");
    // No marker extra: nothing read or written.
    e.mem.set_u8(EXTRA + 4, 0x2d);
    let before = e.mem.bytes(DATA, 0x14);
    call_on(&mut e, 0x2230, REFR, &[s(b"x")]);
    assert_eq!(e.mem.bytes(DATA, 0x14), before);
    let (_, res) = call_on(&mut e, 0x222f, REFR, &[]);
    assert_eq!(string_result(&e, res), b"");
}

#[test]
fn the_flags() {
    let mut e = setup();
    e.mem.set_u32(DATA + 0xc, 0xaaaa_aaf0);
    for (get, set, bit) in [
        (None, 0x2234, 1u8),
        (Some(0x2237), 0x2238, 2),
        (Some(0x2235), 0x2236, 4),
    ] {
        if let Some(g) = get {
            assert_eq!(call_on(&mut e, g, REFR, &[]).1, 0.0, "{g:04x} off");
        }
        call_on(&mut e, set, REFR, &[n(9)]);
        assert_eq!(e.mem.u8(DATA + 0xc) & bit, bit, "{set:04x} on");
        if let Some(g) = get {
            assert_eq!(call_on(&mut e, g, REFR, &[]).1, 1.0, "{g:04x} on");
        }
    }
    assert_eq!(e.mem.u32(DATA + 0xc), 0xaaaa_aaf7, "one byte written");
    call_on(&mut e, 0x2236, REFR, &[n(0)]);
    assert_eq!(e.mem.u8(DATA + 0xc), 0xf3);
    call_on(&mut e, 0x2234, REFR, &[n(0)]);
    assert_eq!(e.mem.u8(DATA + 0xc), 0xf2);
}

#[test]
fn the_type() {
    let mut e = setup();
    e.mem.set_u32(DATA + 0xc, 0xffff_0000);
    assert_eq!(
        call_on(&mut e, 0x2231, REFR, &[]).1,
        65535.0,
        "16 bits, unsigned"
    );
    call_on(&mut e, 0x2232, REFR, &[n(14)]);
    assert_eq!(e.mem.u32(DATA + 0xc), 0x000e_0000, "the flags byte kept");
    call_on(&mut e, 0x2232, REFR, &[n(15)]);
    assert_eq!(e.mem.u16(DATA + 0xe), 14, "15 refused");
    call_on(&mut e, 0x2232, REFR, &[n(-1)]);
    assert_eq!(e.mem.u16(DATA + 0xe), 14, "-1 is 0xFFFFFFFF: refused");
    assert_eq!(call_on(&mut e, 0x2231, PLAIN, &[]).1, -1.0, "no marker: -1");
}

#[test]
fn the_reputation() {
    let mut e = setup();
    call_on(&mut e, 0x222d, REFR, &[]);
    assert_eq!(form_result(&e), 0);
    call_on(&mut e, 0x222e, REFR, &[r(1)]);
    assert_eq!(e.mem.u32(DATA + 0x10), REP);
    call_on(&mut e, 0x222d, REFR, &[]);
    assert_eq!(form_result(&e), 0x4e90);
    call_on(&mut e, 0x222e, REFR, &[r(2)]);
    assert_eq!(e.mem.u32(DATA + 0x10), REP, "a misc item is no reputation");
    call_on(&mut e, 0x222e, REFR, &[]);
    assert_eq!(e.mem.u32(DATA + 0x10), 0, "cleared");
}

/// A reference of another base form (or with no base form) holding the same extra: every
/// command answers as without a marker and leaves the marker data alone.
#[test]
fn another_reference_is_left_alone() {
    let mut e = setup();
    e.mem.set_u32(PLAIN + 0x44 + 4, EXTRA);
    e.mem.set_u32(DATA + 0x10, REP);
    e.mem.set_u32(DATA + 0xc, 0x0003_0007);
    let before = e.mem.bytes(DATA, 0x14);
    for refr in [PLAIN, FORMS + 0x2200] {
        let (_, res) = call_on(&mut e, 0x222f, refr, &[]);
        assert_eq!(string_result(&e, res), b"");
        assert_eq!(call_on(&mut e, 0x2237, refr, &[]).1, 0.0);
        assert_eq!(call_on(&mut e, 0x2235, refr, &[]).1, 0.0);
        assert_eq!(call_on(&mut e, 0x2231, refr, &[]).1, -1.0);
        call_on(&mut e, 0x222d, refr, &[]);
        assert_eq!(form_result(&e), 0);
        call_on(&mut e, 0x2230, refr, &[s(b"x")]);
        for set in [0x2234, 0x2236, 0x2238, 0x2232] {
            call_on(&mut e, set, refr, &[n(0)]);
        }
        call_on(&mut e, 0x222e, refr, &[]);
    }
    assert_eq!(e.mem.bytes(DATA, 0x14), before);
}
