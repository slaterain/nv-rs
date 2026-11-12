//! Tests of the destruction data commands. The object is a weapon (form type 0x28, whose
//! destructible component JIP's `kBaseComponentOffsets` puts at word 0x2D, `+0xB4`); its data
//! and stages are built at the offsets of JIP's header and the engine's own layouts
//! (`DestructibleObjectData`, `DestructibleObjectStage`). Expected answers come from JIP's C++.

use super::*;
use crate::commands::test_support::*;

const WEAPON: u32 = FORMS + 0x1000;
const DEST: u32 = WEAPON + 0xb4;
const DATA: u32 = FORMS + 0x2000;
const STAGES: u32 = FORMS + 0x2100;
const STAGE0: u32 = FORMS + 0x2200;
const STAGE1: u32 = FORMS + 0x2300;
const EXPLOSION: u32 = FORMS + 0x3000;
const DEBRIS: u32 = FORMS + 0x3100;
const CLIMATE: u32 = FORMS + 0x3200;

fn setup() -> Engine {
    let mut e = engine();
    form(&mut e, WEAPON, 0x28, 0x0000_3eaf);
    form(&mut e, EXPLOSION, 0x51, 0x0000_0e51);
    form(&mut e, DEBRIS, 0x52, 0x0000_0d52);
    form(&mut e, CLIMATE, 0x36, 0x0000_0036);
    e.mem.set_u32(DEST + 4, DATA);
    e.mem.set_u32(DATA, 500);
    e.mem.set_u8(DATA + 4, 2);
    e.mem.set_u8(DATA + 5, 1);
    e.mem.set_u32(DATA + 8, STAGES);
    e.mem.set_u32(STAGES, STAGE0);
    e.mem.set_u32(STAGES + 4, STAGE1);
    // Stage 1: damage stage 3, 40%, flags 5, -2 a second, its forms, 7 debris.
    e.mem.set_u8(STAGE1, 3);
    e.mem.set_u8(STAGE1 + 1, 40);
    e.mem.set_u16(STAGE1 + 2, 5);
    e.mem.set_u32(STAGE1 + 4, (-2i32) as u32);
    e.mem.set_u32(STAGE1 + 8, EXPLOSION);
    e.mem.set_u32(STAGE1 + 0xc, DEBRIS);
    e.mem.set_u32(STAGE1 + 0x10, 7);
    for (i, f) in [WEAPON, EXPLOSION, DEBRIS, CLIMATE].iter().enumerate() {
        set_ref(&mut e, i as u16 + 1, *f);
    }
    CALLS.with(|c| c.borrow_mut().clear());
    e
}

#[test]
fn health_and_targetable() {
    let mut e = setup();
    assert_eq!(call(&mut e, 0x26c1, &[r(1)]).1, 500.0);
    e.mem.set_u32(DATA, 0xffff_ffff);
    assert_eq!(call(&mut e, 0x26c1, &[r(1)]).1, -1.0, "read as an int");
    call(&mut e, 0x26c2, &[r(1), n(77)]);
    assert_eq!(e.mem.u32(DATA), 77);
    assert_eq!(call(&mut e, 0x26c3, &[r(1)]).1, 1.0);
    call(&mut e, 0x26c4, &[r(1), n(0)]);
    assert_eq!(e.mem.u8(DATA + 5), 0);
    assert_eq!(call(&mut e, 0x26c3, &[r(1)]).1, 0.0);
    call(&mut e, 0x26c4, &[r(1), n(9)]);
    assert_eq!(e.mem.u8(DATA + 5), 1);
    assert_eq!(call(&mut e, 0x26c5, &[r(1)]).1, 2.0);
    // A form with no destructible component, or no data: nothing.
    assert_eq!(call(&mut e, 0x26c1, &[r(4)]).1, 0.0);
    call(&mut e, 0x26c2, &[r(4), n(5)]);
    e.mem.set_u32(DEST + 4, 0);
    assert_eq!(call(&mut e, 0x26c5, &[r(1)]).1, 0.0);
    call(&mut e, 0x26c2, &[r(1), n(5)]);
    assert_eq!(e.mem.u32(DATA), 77);
}

#[test]
fn the_stage_traits() {
    let mut e = setup();
    let get = |e: &mut Engine, i: i32, t: i32| call(e, 0x26c6, &[r(1), n(i), n(t)]).1;
    assert_eq!(get(&mut e, 1, 0), 40.0);
    assert_eq!(get(&mut e, 1, 1), -2.0);
    assert_eq!(get(&mut e, 1, 2), 3.0);
    assert_eq!(get(&mut e, 1, 3), 5.0);
    assert_eq!(get(&mut e, 1, 4), 7.0);
    assert_eq!(get(&mut e, 1, 5), 0.0);
    assert_eq!(get(&mut e, 2, 0), 0.0, "past the count");
    e.mem.set_u32(DATA + 8, 0);
    assert_eq!(get(&mut e, 1, 0), 0.0, "no stage array");
}

#[test]
fn the_stage_traits_set() {
    let mut e = setup();
    let set = |e: &mut Engine, i: i32, t: i32, v: i32| {
        call(e, 0x26c7, &[r(1), n(i), n(t), n(v)]);
    };
    set(&mut e, 1, 0, 150);
    assert_eq!(e.mem.u8(STAGE1 + 1), 49);
    set(&mut e, 1, 1, 12);
    assert_eq!(e.mem.u32(STAGE1 + 4), 12);
    set(&mut e, 1, 2, 10);
    assert_eq!(e.mem.u8(STAGE1), 1);
    e.mem.set_u8(STAGE1 + 3, 0xaa);
    set(&mut e, 1, 3, 0xff);
    assert_eq!(e.mem.u16(STAGE1 + 2), 7, "a 16-bit write: the next byte cleared");
    set(&mut e, 1, 4, 99);
    assert_eq!(e.mem.u32(STAGE1 + 0x10), 99);
    set(&mut e, 2, 4, 1);
    assert_eq!(e.mem.u32(STAGE1 + 0x10), 99, "past the count");
}

#[test]
fn the_stage_forms() {
    let mut e = setup();
    call(&mut e, 0x26c8, &[r(1), n(1)]);
    assert_eq!(form_result(&e), 0x0e51);
    call(&mut e, 0x26ca, &[r(1), n(1)]);
    assert_eq!(form_result(&e), 0x0d52);
    call(&mut e, 0x26c8, &[r(1), n(0)]);
    assert_eq!(form_result(&e), 0, "stage 0 has none");
    // Set: of the right type, else cleared.
    call(&mut e, 0x26c9, &[r(1), n(0), r(2)]);
    assert_eq!(e.mem.u32(STAGE0 + 8), EXPLOSION);
    call(&mut e, 0x26c9, &[r(1), n(0), r(3)]);
    assert_eq!(e.mem.u32(STAGE0 + 8), 0);
    call(&mut e, 0x26cb, &[r(1), n(0), r(3)]);
    assert_eq!(e.mem.u32(STAGE0 + 0xc), DEBRIS);
    call(&mut e, 0x26cb, &[r(1), n(0)]);
    assert_eq!(e.mem.u32(STAGE0 + 0xc), 0);
    call(&mut e, 0x26cb, &[r(1), n(2), r(3)]);
    assert_eq!(e.mem.u32(STAGE1 + 0xc), DEBRIS, "past the count: nothing");
}

#[test]
fn the_stage_replacement() {
    let mut e = setup();
    let (_, res) = call(&mut e, 0x26cc, &[r(1), n(1)]);
    assert_eq!(string_result(&e, res), b"", "none");
    // A stage without one gets a zeroed TESModelTextureSwap with its vtable, then the path.
    let (_, res) = call(&mut e, 0x26cd, &[r(1), n(1), s(b"broken.nif")]);
    assert_eq!(res, 1.0);
    let swap = e.mem.u32(STAGE1 + 0x14);
    assert_ne!(swap, 0);
    assert_eq!(e.mem.u32(swap), 0x0101_d124, "TESModelTextureSwap's vtable");
    assert_eq!(e.mem.bytes(swap + 0xc, 0x14), vec![0; 0x14]);
    assert_eq!(text_at(&e, swap + 4), b"broken.nif");
    assert_eq!(calls()[0], (0x00aa_3e40, vec![0x011f_6238, 0x20]));
    let (_, res) = call(&mut e, 0x26cc, &[r(1), n(1)]);
    assert_eq!(string_result(&e, res), b"broken.nif");
    // One it has: only the path.
    CALLS.with(|c| c.borrow_mut().clear());
    call(&mut e, 0x26cd, &[r(1), n(1), s(b"b2.nif")]);
    assert_eq!(e.mem.u32(STAGE1 + 0x14), swap);
    assert_eq!(text_at(&e, swap + 4), b"b2.nif");
    // Past the count: nothing, result 0.
    assert_eq!(call(&mut e, 0x26cd, &[r(1), n(2), s(b"x")]).1, 0.0);
}

/// The stage array's pointers.
fn stages(e: &Engine) -> Vec<u32> {
    let (array, count) = (e.mem.u32(DATA + 8), e.mem.u8(DATA + 4));
    (0..u32::from(count))
        .map(|i| e.mem.u32(array + 4 * i))
        .collect()
}

#[test]
fn a_stage_added() {
    let mut e = setup();
    // In the middle: a new array one longer, the old freed.
    let (_, res) = call(&mut e, 0x26f3, &[r(1), n(1), n(60), n(258)]);
    assert_eq!(res, 1.0);
    let now = stages(&e);
    assert_eq!(now.len(), 3);
    assert_eq!((now[0], now[2]), (STAGE0, STAGE1));
    let new = now[1];
    assert_eq!(e.mem.u8(new + 1), 60);
    assert_eq!(e.mem.u8(new), 2, "258 stored as a byte");
    assert_eq!(e.mem.bytes(new + 2, 0x16), vec![0; 0x16]);
    assert_eq!(freed(), vec![STAGES]);
    assert_eq!(
        e.mem.u32(WEAPON + 8) & 0x0100_0000,
        0x0100_0000,
        "marked destructible"
    );
    // A negative index: the end.
    let (_, res) = call(&mut e, 0x26f3, &[r(1), n(-1), n(1), n(1)]);
    assert_eq!(res, 3.0);
    assert_eq!(stages(&e).len(), 4);
    // Past the count: nothing, -1.
    let (_, res) = call(&mut e, 0x26f3, &[r(1), n(5), n(1), n(1)]);
    assert_eq!(res, -1.0);
    assert_eq!(stages(&e).len(), 4);
    // Not destructible: -1.
    assert_eq!(call(&mut e, 0x26f3, &[r(4), n(0), n(1), n(1)]).1, -1.0);
}

#[test]
fn a_first_stage_makes_the_data() {
    let mut e = setup();
    e.mem.set_u32(DEST + 4, 0);
    let (_, res) = call(&mut e, 0x26f3, &[r(1), n(-1), n(100), n(0)]);
    assert_eq!(res, 0.0);
    let data = e.mem.u32(DEST + 4);
    assert_ne!(data, 0);
    assert_eq!(e.mem.u32(data), 0);
    assert_eq!(e.mem.u8(data + 4), 1, "one stage");
    assert_eq!(e.mem.u32(data + 4) & 0xffff_ff00, 0x160d_0000);
    assert_eq!((e.mem.u32(data + 0xc), e.mem.u32(data + 0x10)), (0, 0));
    let array = e.mem.u32(data + 8);
    let stage = e.mem.u32(array);
    assert_eq!(e.mem.u8(stage + 1), 100);
    let allocs: Vec<u32> = calls()
        .iter()
        .filter(|c| c.0 == 0x00aa_3e40)
        .map(|c| c.1[1])
        .collect();
    assert_eq!(allocs, vec![0x18, 0x14, 4], "the stage, the data, the array");
    // With data but no array, index 1 is past the count of 0.
    e.mem.set_u32(data + 8, 0);
    assert_eq!(call(&mut e, 0x26f3, &[r(1), n(1), n(1), n(1)]).1, -1.0);
}

#[test]
fn a_stage_removed() {
    let mut e = setup();
    e.mem.set_u32(WEAPON + 8, 0x0100_0010);
    let (_, res) = call(&mut e, 0x26f4, &[r(1), n(0)]);
    assert_eq!(res, 0.0);
    assert_eq!(stages(&e), vec![STAGE1]);
    assert_eq!(e.mem.u32(STAGES + 4), 0, "the freed slot cleared");
    assert_eq!(freed(), vec![STAGE0]);
    // The last one: the array freed, the flag cleared.
    let (_, res) = call(&mut e, 0x26f4, &[r(1), n(0)]);
    assert_eq!(res, 0.0);
    assert_eq!(e.mem.u8(DATA + 4), 0);
    assert_eq!(e.mem.u32(DATA + 8), 0);
    assert_eq!(freed(), vec![STAGE0, STAGE1, STAGES]);
    assert_eq!(e.mem.u32(WEAPON + 8), 0x10);
    // Nothing left to remove: -1.
    assert_eq!(call(&mut e, 0x26f4, &[r(1), n(0)]).1, -1.0);
    assert_eq!(call(&mut e, 0x26f4, &[r(4), n(0)]).1, -1.0);
}

#[test]
fn a_later_stage_removed() {
    let mut e = setup();
    let (_, res) = call(&mut e, 0x26f4, &[r(1), n(1)]);
    assert_eq!(res, 1.0);
    assert_eq!(stages(&e), vec![STAGE0]);
    assert_eq!(e.mem.u32(STAGES + 4), 0);
    assert_eq!(call(&mut e, 0x26f4, &[r(1), n(1)]).1, -1.0, "past the count");
    // Count but no array: -1.
    e.mem.set_u32(DATA + 8, 0);
    assert_eq!(call(&mut e, 0x26f4, &[r(1), n(0)]).1, -1.0);
}
