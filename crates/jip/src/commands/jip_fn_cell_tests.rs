//! Tests of `jip_fn_cell` at the cell loader's offsets (flags +0x24, extra data list +0x28, noise
//! texture path +0x5C), the made extras' exact blocks, and forms of the wrong type left alone.

use super::*;
use crate::commands::test_support::*;

const CELL: u32 = FORMS + 0x1000;
const WATER: u32 = FORMS + 0x2000;
const WATER2: u32 = FORMS + 0x2100;
const CLIMATE: u32 = FORMS + 0x2200;
const MISC: u32 = FORMS + 0x2300;
const OWNER: u32 = FORMS + 0x2400;
const XROT: u32 = FORMS + 0x3000;

fn setup() -> Engine {
    let mut e = engine();
    extra_stand_ins(&mut e);
    form(&mut e, CELL, 0x39, 0x0000_0100);
    form(&mut e, WATER, 0x4e, 0x0000_0101);
    form(&mut e, WATER2, 0x4e, 0x0000_0102);
    form(&mut e, CLIMATE, 0x36, 0x0000_0103);
    form(&mut e, MISC, 0x1f, 0x0000_0104);
    form(&mut e, OWNER, 0x08, 0x0000_0105);
    for (i, f) in [CELL, WATER, WATER2, CLIMATE, MISC].iter().enumerate() {
        set_ref(&mut e, i as u16 + 1, *f);
    }
    e
}

#[test]
fn the_water_form() {
    let mut e = setup();
    // None to clear with no extra: nothing.
    call(&mut e, 0x27dd, &[r(1)]);
    assert!(calls().is_empty());
    let block = heap_next(&e);
    call(&mut e, 0x27dd, &[r(1), r(2)]);
    assert_eq!(allocations(), vec![0x10]);
    assert_eq!(
        [0, 4, 8, 0xc].map(|o| e.mem.u32(block + o)),
        [0x0101_4270, 0x03, 0, WATER]
    );
    assert_eq!(calls_to(ADD_EXTRA_FN), vec![vec![CELL + 0x28, block]]);
    // The extra there: its form replaced.
    call(&mut e, 0x27dd, &[r(1), r(3)]);
    assert_eq!(e.mem.u32(block + 0xc), WATER2);
    assert_eq!(allocations().len(), 1);
    // A wrong type: nothing.
    call(&mut e, 0x27dd, &[r(1), r(5)]);
    assert_eq!(e.mem.u32(block + 0xc), WATER2);
    // None: removed by type.
    call(&mut e, 0x27dd, &[r(1)]);
    assert_eq!(calls_to(REMOVE_BY_TYPE_FN), vec![vec![CELL + 0x28, 0x03]]);
    // Not a cell: nothing.
    let before = e.mem.bytes(MISC, 0xe0);
    call(&mut e, 0x27dd, &[r(5), r(2)]);
    assert_eq!(e.mem.bytes(MISC, 0xe0), before);
}

#[test]
fn the_climate() {
    let mut e = setup();
    call(&mut e, 0x27de, &[r(1)]);
    assert_eq!(form_result(&e), 0);
    let block = heap_next(&e);
    call(&mut e, 0x27df, &[r(1), r(4)]);
    assert_eq!(
        [0, 4, 8, 0xc].map(|o| e.mem.u32(block + o)),
        [0x0101_424c, 0x08, 0, CLIMATE]
    );
    call(&mut e, 0x27de, &[r(1)]);
    assert_eq!(form_result(&e), 0x103);
    // A climate extra with no climate: nothing.
    e.mem.set_u32(block + 0xc, 0);
    e.mem.set_f64(RESULT, 0.0);
    call(&mut e, 0x27de, &[r(1)]);
    assert_eq!(form_result(&e), 0);
    call(&mut e, 0x27df, &[r(1), r(2)]);
    assert_eq!(e.mem.u32(block + 0xc), 0, "a water form is not a climate");
    call(&mut e, 0x27df, &[r(1)]);
    assert_eq!(calls_to(REMOVE_BY_TYPE_FN), vec![vec![CELL + 0x28, 0x08]]);
    call(&mut e, 0x27de, &[r(5)]);
    assert_eq!(form_result(&e), 0);
}

#[test]
fn the_noise_texture() {
    let mut e = setup();
    let (_, res) = call(&mut e, 0x27e0, &[r(1)]);
    assert_eq!(string_result(&e, res), b"");
    call(&mut e, 0x27e1, &[r(1), s(b"water\\noise.dds")]);
    assert_eq!(text_at(&e, CELL + 0x5c), b"water\\noise.dds");
    let (_, res) = call(&mut e, 0x27e0, &[r(1)]);
    assert_eq!(string_result(&e, res), b"water\\noise.dds");
    let before = e.mem.bytes(MISC, 0xe0);
    call(&mut e, 0x27e1, &[r(5), s(b"x")]);
    assert_eq!(e.mem.bytes(MISC, 0xe0), before);
}

#[test]
fn the_owner() {
    let mut e = setup();
    e.register(0x0054_6a40, |_, w| {
        assert_eq!(w[0], CELL);
        Ret {
            eax: OWNER,
            ..Ret::default()
        }
    });
    call(&mut e, 0x28ab, &[r(1)]);
    assert_eq!(form_result(&e), 0x105);
    e.register(0x0054_6a40, |_, _| Ret::default());
    e.mem.set_f64(RESULT, 0.0);
    call(&mut e, 0x28ab, &[r(1)]);
    assert_eq!(form_result(&e), 0);
    call(&mut e, 0x28ab, &[r(5)]);
    assert_eq!(form_result(&e), 0);
}

#[test]
fn the_north_rotation() {
    let mut e = setup();
    put_extra(&mut e, CELL + 0x28, XROT, 0x43, 0.5f32.to_bits());
    // An exterior: nothing.
    assert_eq!(call(&mut e, 0x292d, &[r(1)]).1, 0.0);
    e.mem.set_u8(CELL + 0x24, 0x81);
    assert_eq!(
        call(&mut e, 0x292d, &[r(1)]).1,
        0.5 * crate::commands::DBL_180_D_PI
    );
    assert_eq!(call(&mut e, 0x292d, &[r(5)]).1, 0.0);
}
