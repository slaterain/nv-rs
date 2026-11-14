//! Tests of `ln_fn_cell` at the cell loader's offsets (flags byte +0x24, water height +0x50,
//! worldspace +0xC0, lighting template +0xD8, inherited flags +0xDC; `ExtraCellWaterType` 0x03 and
//! `ExtraCellImageSpace` 0x59 at +0xC; a worldspace's first water form +0x74), the player's parent
//! cell (+0x40) standing in for a missing cell.

use super::*;
use crate::commands::test_support::*;

const CELL: u32 = FORMS + 0x1000;
const CELL2: u32 = FORMS + 0x1100;
const WS: u32 = FORMS + 0x2000;
const WATER: u32 = FORMS + 0x2100;
const WATER_WS: u32 = FORMS + 0x2200;
const IMGS: u32 = FORMS + 0x2300;
const LGTM: u32 = FORMS + 0x2400;
const MISC: u32 = FORMS + 0x2500;
const PC: u32 = FORMS + 0x2600;
const XW: u32 = FORMS + 0x3000;
const XI: u32 = FORMS + 0x3100;

fn setup() -> Engine {
    let mut e = engine();
    extra_stand_ins(&mut e);
    form(&mut e, CELL, 0x39, 0x0000_0100);
    form(&mut e, CELL2, 0x39, 0x0000_0101);
    form(&mut e, WS, 0x41, 0x0000_0102);
    form(&mut e, WATER, 0x4e, 0x0000_0103);
    form(&mut e, WATER_WS, 0x4e, 0x0000_0104);
    form(&mut e, IMGS, 0x53, 0x0000_0105);
    form(&mut e, LGTM, 0x65, 0x0000_0106);
    form(&mut e, MISC, 0x1f, 0x0000_0107);
    e.mem.set_u32(0x011d_ea3c, PC);
    e.mem.set_u32(PC + 0x40, CELL2);
    for (i, f) in [CELL, CELL2, LGTM, MISC].iter().enumerate() {
        set_ref(&mut e, i as u16 + 1, *f);
    }
    e
}

#[test]
fn water_height() {
    let mut e = setup();
    e.mem.set_f32(CELL + 0x50, 12.5);
    assert_eq!(call(&mut e, 0x22f1, &[r(1)]).1, 12.5);
    call(&mut e, 0x22f2, &[z(-3.25), r(1)]);
    assert_eq!(e.mem.f32(CELL + 0x50), -3.25);
    // No cell: the player's.
    call(&mut e, 0x22f2, &[z(7.0)]);
    assert_eq!(e.mem.f32(CELL2 + 0x50), 7.0);
    assert_eq!(call(&mut e, 0x22f1, &[]).1, 7.0);
    // Another form: nothing.
    let before = e.mem.bytes(MISC, 0xe0);
    call(&mut e, 0x22f2, &[z(1.0), r(4)]);
    assert_eq!(e.mem.bytes(MISC, 0xe0), before);
    assert_eq!(call(&mut e, 0x22f1, &[r(4)]).1, 0.0);
    // The player in no cell: nothing.
    e.mem.set_u32(PC + 0x40, 0);
    assert_eq!(call(&mut e, 0x22f1, &[]).1, 0.0);
}

#[test]
fn water_form() {
    let mut e = setup();
    call(&mut e, 0x22be, &[r(1)]);
    assert_eq!(form_result(&e), 0, "no extra, no worldspace");
    e.mem.set_u32(CELL + 0xc0, WS);
    e.mem.set_u32(WS + 0x74, WATER_WS);
    call(&mut e, 0x22be, &[r(1)]);
    assert_eq!(form_result(&e), 0x104, "the worldspace's");
    put_extra(&mut e, CELL + 0x28, XW, 0x03, WATER);
    call(&mut e, 0x22be, &[r(1)]);
    assert_eq!(form_result(&e), 0x103, "the cell's own first");
    // The extra with no form: the worldspace's again.
    e.mem.set_u32(XW + 0xc, 0);
    call(&mut e, 0x22be, &[r(1)]);
    assert_eq!(form_result(&e), 0x104);
    e.mem.set_f64(RESULT, 0.0);
    call(&mut e, 0x22be, &[r(4)]);
    assert_eq!(form_result(&e), 0);
}

#[test]
fn image_space_and_lighting_template() {
    let mut e = setup();
    call(&mut e, 0x22ea, &[r(1)]);
    assert_eq!(form_result(&e), 0);
    put_extra(&mut e, CELL + 0x28, XI, 0x59, IMGS);
    call(&mut e, 0x22ea, &[r(1)]);
    assert_eq!(form_result(&e), 0x105);
    call(&mut e, 0x2253, &[r(1), r(3)]);
    assert_eq!(e.mem.u32(CELL + 0xd8), LGTM);
    call(&mut e, 0x2252, &[r(1)]);
    assert_eq!(form_result(&e), 0x106);
    call(&mut e, 0x2253, &[r(1), r(4)]);
    assert_eq!(
        e.mem.u32(CELL + 0xd8),
        LGTM,
        "a misc item is not a template"
    );
    call(&mut e, 0x2253, &[r(1)]);
    assert_eq!(e.mem.u32(CELL + 0xd8), 0, "none clears it");
    e.mem.set_f64(RESULT, 0.0);
    call(&mut e, 0x2252, &[r(1)]);
    assert_eq!(form_result(&e), 0);
    let before = e.mem.bytes(MISC, 0xe0);
    call(&mut e, 0x2253, &[r(4), r(3)]);
    assert_eq!(e.mem.bytes(MISC, 0xe0), before);
}

#[test]
fn flags() {
    let mut e = setup();
    e.mem.set_u8(CELL + 0x24, 0x81);
    assert_eq!(call(&mut e, 0x22ec, &[r(1), n(0)]).1, 1.0);
    assert_eq!(call(&mut e, 0x22ec, &[r(1), n(7)]).1, 1.0);
    assert_eq!(call(&mut e, 0x22ec, &[r(1), n(1)]).1, 0.0);
    call(&mut e, 0x22ed, &[r(1), n(1), n(1)]);
    call(&mut e, 0x22ed, &[r(1), n(7), n(0)]);
    assert_eq!(e.mem.u8(CELL + 0x24), 0x03);
    assert_eq!(e.mem.u8(CELL + 0x25), 0, "the byte after untouched");
    // 8 to 16: the inherited flags' bits 0 to 8.
    call(&mut e, 0x22ed, &[r(1), n(8), n(1)]);
    call(&mut e, 0x22ed, &[r(1), n(16), n(1)]);
    assert_eq!(e.mem.u32(CELL + 0xdc), 0x101);
    assert_eq!(call(&mut e, 0x22ec, &[r(1), n(16)]).1, 1.0);
    assert_eq!(call(&mut e, 0x22ec, &[r(1), n(9)]).1, 0.0);
    call(&mut e, 0x22ed, &[r(1), n(8), n(0)]);
    assert_eq!(e.mem.u32(CELL + 0xdc), 0x100);
    // Above 16: nothing.
    call(&mut e, 0x22ed, &[r(1), n(17), n(1)]);
    assert_eq!(e.mem.u32(CELL + 0xdc), 0x100);
    e.mem.set_u32(CELL + 0xdc, u32::MAX);
    assert_eq!(call(&mut e, 0x22ec, &[r(1), n(17)]).1, 0.0);
    let before = e.mem.bytes(MISC, 0xe0);
    call(&mut e, 0x22ed, &[r(4), n(1), n(1)]);
    assert_eq!(e.mem.bytes(MISC, 0xe0), before);
}
