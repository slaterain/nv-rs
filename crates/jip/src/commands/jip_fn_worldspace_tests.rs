//! Tests of the worldspace commands, at the raw offsets of the worldspace loader (`00583560`:
//! flags byte +0x4C, parent flags +0x4E) and JIP's header (persistent cell +0x34, climate +0x40,
//! image space +0x44, parent +0x70, water +0x74, noise texture path +0xE4), with the sky
//! (`0x11DEA20`, its climate +0xC), the player (`0x11DEA3C`, its cell +0x40) and the cell's
//! worldspace (+0xC0).

use super::*;
use crate::commands::test_support::*;

const ROOT: u32 = FORMS + 0x1000;
const CHILD: u32 = FORMS + 0x2000;
const CLIMATE: u32 = FORMS + 0x3000;
const WATER: u32 = FORMS + 0x3100;
const IMGS: u32 = FORMS + 0x3200;
const CELL: u32 = FORMS + 0x3300;
const MISC: u32 = FORMS + 0x3400;
const SKY_AT: u32 = FORMS + 0x4000;
const PLAYER_AT: u32 = FORMS + 0x5000;

fn setup() -> Engine {
    let mut e = engine();
    form(&mut e, ROOT, 0x41, 0x0000_0a00);
    form(&mut e, CHILD, 0x41, 0x0000_0b00);
    form(&mut e, CLIMATE, 0x36, 0x0000_0c11);
    form(&mut e, WATER, 0x4e, 0x0000_0d11);
    form(&mut e, IMGS, 0x53, 0x0000_0e11);
    form(&mut e, CELL, 0x39, 0x0000_0f11);
    form(&mut e, MISC, 0x1f, 0x0000_0999);
    for (i, f) in [ROOT, CHILD, CLIMATE, WATER, IMGS, CELL, MISC]
        .iter()
        .enumerate()
    {
        set_ref(&mut e, i as u16 + 1, *f);
    }
    e.map(0x011d_e000, 0x1000);
    e
}

#[test]
fn the_climate_through_parents() {
    let mut e = setup();
    e.mem.set_u32(ROOT + 0x40, CLIMATE);
    e.mem.set_u32(CHILD + 0x70, ROOT);
    call(&mut e, 0x26d3, &[r(2)]);
    assert_eq!(form_result(&e), 0, "own climate (none), the flag off");
    e.mem.set_u16(CHILD + 0x4e, 0x10);
    call(&mut e, 0x26d3, &[r(2)]);
    assert_eq!(form_result(&e), 0x0c11, "the parent's");
    // The flag without a parent: its own.
    e.mem.set_u32(CHILD + 0x70, 0);
    call(&mut e, 0x26d3, &[r(2)]);
    assert_eq!(form_result(&e), 0);
}

#[test]
fn the_climate_set() {
    let mut e = setup();
    e.mem.set_u32(CHILD + 0x70, ROOT);
    e.mem.set_u16(CHILD + 0x4e, 0x13);
    // No sky yet: the form changes, nothing else.
    call(&mut e, 0x26d4, &[r(2), r(3)]);
    assert_eq!(e.mem.u32(CHILD + 0x40), CLIMATE);
    assert_eq!(e.mem.u16(CHILD + 0x4e), 0x03, "stops using the parent's");
    call(&mut e, 0x26d4, &[r(2), r(7)]);
    assert_eq!(e.mem.u32(CHILD + 0x40), CLIMATE, "a misc item: nothing");
    call(&mut e, 0x26d4, &[r(2)]);
    assert_eq!(e.mem.u32(CHILD + 0x40), 0, "cleared");
    assert_eq!(e.mem.u16(CHILD + 0x4e), 0x03);

    // The sky and the player in a cell of the child, which uses the root's climate.
    e.mem.set_u32(0x011d_ea20, SKY_AT);
    e.mem.set_u32(0x011d_ea3c, PLAYER_AT);
    e.mem.set_u32(PLAYER_AT + 0x40, CELL);
    e.mem.set_u32(CELL + 0xc0, CHILD);
    e.mem.set_u16(CHILD + 0x4e, 0x10);
    call(&mut e, 0x26d4, &[r(1), r(3)]);
    assert_eq!(
        e.mem.u32(SKY_AT + 0xc),
        CLIMATE,
        "the player is under the root"
    );
    e.mem.set_u32(SKY_AT + 0xc, 0);
    // Another worldspace: the sky keeps its climate.
    e.mem.set_u16(CHILD + 0x4e, 0);
    call(&mut e, 0x26d4, &[r(1), r(3)]);
    assert_eq!(
        e.mem.u32(SKY_AT + 0xc),
        0,
        "the player's worldspace is the child now"
    );
    call(&mut e, 0x26d4, &[r(2), r(3)]);
    assert_eq!(e.mem.u32(SKY_AT + 0xc), CLIMATE);
    // No cell: nothing more.
    e.mem.set_u32(SKY_AT + 0xc, 0);
    e.mem.set_u32(PLAYER_AT + 0x40, 0);
    call(&mut e, 0x26d4, &[r(2), r(3)]);
    assert_eq!(e.mem.u32(SKY_AT + 0xc), 0);
    // A cell outside any worldspace: nothing more.
    e.mem.set_u32(PLAYER_AT + 0x40, CELL);
    e.mem.set_u32(CELL + 0xc0, 0);
    call(&mut e, 0x26d4, &[r(2), r(3)]);
    assert_eq!(e.mem.u32(SKY_AT + 0xc), 0);
}

#[test]
fn the_flags() {
    let mut e = setup();
    let get = |e: &mut Engine, f: i32| call(e, 0x26e7, &[r(1), n(f)]).1;
    let set = |e: &mut Engine, f: i32, v: i32| {
        call(e, 0x26e8, &[r(1), n(f), n(v)]);
    };
    e.mem.set_u32(ROOT + 0x4c, 0xffff_ff81);
    assert_eq!(get(&mut e, 0), 1.0);
    assert_eq!(get(&mut e, 7), 1.0);
    assert_eq!(get(&mut e, 1), 0.0);
    assert_eq!(get(&mut e, 8), 0.0, "flag 8 is past the byte");
    set(&mut e, 2, 1);
    assert_eq!(e.mem.u32(ROOT + 0x4c), 0xffff_ff85, "one byte written");
    set(&mut e, 7, 0);
    assert_eq!(e.mem.u32(ROOT + 0x4c), 0xffff_ff05);
    set(&mut e, 8, 1);
    assert_eq!(e.mem.u32(ROOT + 0x4c), 0xffff_ff05, "flag 8: nothing");
}

#[test]
fn the_noise_texture() {
    let mut e = setup();
    let (_, res) = call(&mut e, 0x27d7, &[r(1)]);
    assert_eq!(string_result(&e, res), b"", "no buffer");
    call(&mut e, 0x27d8, &[r(1), s(b"noise.dds")]);
    assert_eq!(text_at(&e, ROOT + 0xe4), b"noise.dds");
    let (_, res) = call(&mut e, 0x27d7, &[r(1)]);
    assert_eq!(string_result(&e, res), b"noise.dds");
    call(&mut e, 0x27d8, &[r(7), s(b"x.dds")]);
    assert_eq!(e.mem.u32(MISC + 0xe4), 0, "any form read, not a worldspace");
}

#[test]
fn the_forms() {
    let mut e = setup();
    for (get, set, off, rf, id) in [
        (0x27d9, 0x27da, 0x74, 4u16, 0x0d11u32),
        (0x27db, 0x27dc, 0x44, 5, 0x0e11),
    ] {
        call(&mut e, get, &[r(1)]);
        assert_eq!(form_result(&e), 0, "{get:04x}: none");
        call(&mut e, set, &[r(1), r(rf)]);
        assert_eq!(
            e.mem.u32(ROOT + off),
            FORMS + 0x3000 + 0x100 * u32::from(rf - 3)
        );
        call(&mut e, get, &[r(1)]);
        assert_eq!(form_result(&e), id);
        call(&mut e, set, &[r(1), r(7)]);
        assert_ne!(e.mem.u32(ROOT + off), 0, "{set:04x}: a misc item, nothing");
        call(&mut e, set, &[r(1), r(3)]);
        assert_ne!(
            e.mem.u32(ROOT + off),
            CLIMATE,
            "{set:04x}: a climate, nothing"
        );
        call(&mut e, set, &[r(1)]);
        assert_eq!(e.mem.u32(ROOT + off), 0, "{set:04x}: cleared");
    }
    call(&mut e, 0x292b, &[r(1)]);
    assert_eq!(form_result(&e), 0);
    e.mem.set_u32(ROOT + 0x34, CELL);
    call(&mut e, 0x292b, &[r(1)]);
    assert_eq!(form_result(&e), 0x0f11);
}

thread_local! {
    static LINES: std::cell::RefCell<Vec<Vec<u8>>> = const { std::cell::RefCell::new(Vec::new()) };
}

/// The console echo of `GetWorldspaceClimate`: the climate used (the parent's when the flag is
/// on), `0` with none; nothing outside the console.
#[test]
fn the_climate_echoed() {
    const CONSOLE: u32 = 0x0200_f000;
    const NAME: u32 = FORMS + 0x5800;
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
    e.register(0x005e_1550, |e, w| e.call(w[0], &w[1..9]));
    let vtbl = FORMS + 0x6000;
    e.mem.set_u32(CLIMATE, vtbl);
    e.mem.set_u32(vtbl + 0x130, 0x0050_0000);
    e.register(0x0050_0000, |_, _| Ret {
        eax: NAME,
        ..Ret::default()
    });
    e.mem.write(NAME, b"Mojave\0");
    e.mem.set_u32(ROOT + 0x40, CLIMATE);
    e.mem.set_u32(CHILD + 0x70, ROOT);
    console(&mut e, true);
    let run = |e: &mut Engine| {
        let words = crate::test_support::line(e, 0x26d3, &[r(2)]);
        let execute = crate::test_support::execute_of(e, 0x26d3);
        let mut all = vec![execute];
        all.extend_from_slice(&words);
        e.call(0x005e_1550, &all);
    };
    run(&mut e);
    e.mem.set_u16(CHILD + 0x4e, 0x10);
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
            "GetWorldspaceClimate >> 0",
            "GetWorldspaceClimate >> C11 \"Mojave\"",
        ]
    );
}

/// The flag "use the parent's climate" is cleared only when there is a parent.
#[test]
fn the_flag_stays_without_a_parent() {
    let mut e = setup();
    e.mem.set_u16(ROOT + 0x4e, 0x10);
    call(&mut e, 0x26d4, &[r(1), r(3)]);
    assert_eq!(e.mem.u32(ROOT + 0x40), CLIMATE);
    assert_eq!(e.mem.u16(ROOT + 0x4e), 0x10, "no parent: the flag stays");
}

/// The worldspace is read as `kParamType_WorldSpace` (any form for the noise texture's setter,
/// tested above): a misc item holding the same words is refused by xNVSE's reader, so every
/// command answers 0 or "" and leaves its bytes.
#[test]
fn another_form_type_is_left_alone() {
    let mut e = setup();
    for off in (0x34..0xec).step_by(4) {
        e.mem.set_u32(MISC + off, CLIMATE);
    }
    e.mem.set_u16(MISC + 0x4e, 0x10);
    let before = e.mem.bytes(MISC, 0xec);
    for get in [0x26d3, 0x27d9, 0x27db, 0x292b] {
        call(&mut e, get, &[r(7)]);
        assert_eq!(form_result(&e), 0, "{get:04x}");
    }
    let (_, res) = call(&mut e, 0x27d7, &[r(7)]);
    assert_eq!(string_result(&e, res), b"");
    assert_eq!(call(&mut e, 0x26e7, &[r(7), n(0)]).1, 0.0);
    call(&mut e, 0x26e8, &[r(7), n(0), n(0)]);
    call(&mut e, 0x26d4, &[r(7), r(3)]);
    call(&mut e, 0x27da, &[r(7), r(4)]);
    call(&mut e, 0x27dc, &[r(7), r(5)]);
    assert_eq!(e.mem.bytes(MISC, 0xec), before);
}
