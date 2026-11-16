//! Tests of `jip_fn_miscellaneous`: the player, the HUD, `TES`, the sky, the OS and time globals
//! at their addresses, forms at the offsets of the headers and the exe, and the game functions
//! the commands call as recording stand-ins. Expectations from JIP's source.

use super::*;
use crate::commands::test_support::*;
use nvse::vars::Data;
use std::collections::BTreeMap;

const PC: u32 = FORMS + 0x1_0000;
const PROCESS: u32 = FORMS + 0x1_1000;
const HUD_AT: u32 = FORMS + 0x1_2000;
const TES_AT: u32 = FORMS + 0x1_3000;
const SKY_AT: u32 = FORMS + 0x1_4000;
const MOON: u32 = FORMS + 0x1_4400;
const OSG: u32 = FORMS + 0x1_5000;
const RADIO: u32 = FORMS + 0x1_5100;
const GLOBALS: u32 = FORMS + 0x1_5200;
const MAP: u32 = FORMS + 0x1_7000;
const AV_INFOS: u32 = FORMS + 0x1_8000;
/// A plain form's vtable (+0xF0 not `ReturnTrue`) and a reference's.
const VT_FORM: u32 = FORMS + 0x6000;
const VT_REFR: u32 = FORMS + 0x6400;
const VT_PROCESS: u32 = FORMS + 0x6800;

const IDLE: u32 = FORMS + 0x1000;
const CONT: u32 = FORMS + 0x1100;
const MISC: u32 = FORMS + 0x1200;
const NAVM: u32 = FORMS + 0x1300;
const WRLD: u32 = FORMS + 0x1400;
const CELL_EX: u32 = FORMS + 0x1500;
const CELL_IN: u32 = FORMS + 0x1600;
const CELL_X: u32 = FORMS + 0x1700;
const REFR: u32 = FORMS + 0x1800;
const NOTE: u32 = FORMS + 0x1900;
const RADS: u32 = FORMS + 0x1a00;
const HUNGER: u32 = FORMS + 0x1b00;
const SPEL: u32 = FORMS + 0x1c00;
const LIST: u32 = FORMS + 0x1d00;
const LIST2: u32 = FORMS + 0x1e00;
const DOOR: u32 = FORMS + 0x1f00;
const ACTOR: u32 = FORMS + 0x2000;

const SET_FIRST_PERSON_FN: u32 = 0x0095_0110;
const DETECTION_FN: u32 = 0x0097_3710;
const IS_WEAPON_OUT_FN: u32 = 0x0060_0454;
const TERRAIN_FN: u32 = 0x0045_72e0;
const MANAGER_FN: u32 = 0x006c_0720;
const DISABLE_FN: u32 = 0x006c_1150;
const ENABLE_FN: u32 = 0x006c_1130;
const MOD_AV_FN: u32 = 0x0093_b0f0;
const PRECACHE_FN: u32 = 0x00ad_7fa0;
const MANAGER: u32 = 0x011d_71e0;

fn rec(addr: u32, w: &[u32]) {
    CALLS.with(|c| c.borrow_mut().push((addr, w.to_vec())));
}

fn eax(v: u32) -> Ret {
    Ret {
        eax: v,
        ..Ret::default()
    }
}

fn plain(e: &mut Engine, at: u32, t: u8, id: u32) -> u32 {
    form(e, at, t, id);
    e.mem.set_u32(at, VT_FORM);
    at
}

fn setup() -> Engine {
    let mut e = engine();
    for page in [
        0x011a_4000,
        0x011c_3000,
        0x011c_a000,
        0x011d_6000,
        0x011d_7000,
        0x011d_9000,
        0x011d_d000,
        0x011d_e000,
        0x011e_0000,
        0x011f_1000,
        0x011f_2000,
        0x011f_6000,
    ] {
        e.map(page, 0x1000);
    }
    e.mem.set_u32(VT_REFR + 0xf0, 0x008d_0360);
    e.mem.set_u32(VT_PROCESS + 0x454, IS_WEAPON_OUT_FN);
    e.mem.set_u32(0x011d_ea3c, PC);
    e.mem.set_u32(0x011d_96c0, HUD_AT);
    e.mem.set_u32(0x011d_ea10, TES_AT);
    e.mem.set_u32(0x011d_ea20, SKY_AT);
    e.mem.set_u32(0x011d_ea0c, OSG);
    e.mem.set_u32(0x011d_d42c, RADIO);
    e.mem.set_u32(PC, VT_REFR);
    e.mem.set_u32(PC + 0x68, PROCESS);
    e.mem.set_u32(PROCESS, VT_PROCESS);
    // Forms.
    plain(&mut e, IDLE, 0x48, 0x101);
    plain(&mut e, CONT, 0x1b, 0x102);
    plain(&mut e, MISC, 0x1f, 0x103);
    plain(&mut e, NAVM, 0x43, 0x104);
    plain(&mut e, WRLD, 0x41, 0x105);
    plain(&mut e, CELL_EX, 0x39, 0x106);
    plain(&mut e, CELL_IN, 0x39, 0x107);
    plain(&mut e, CELL_X, 0x39, 0x108);
    form(&mut e, REFR, 0x3a, 0x109);
    e.mem.set_u32(REFR, VT_REFR);
    e.mem.set_u32(REFR + 0x20, CONT);
    plain(&mut e, NOTE, 0x31, 0x10a);
    plain(&mut e, RADS, 0x5a, 0x10b);
    plain(&mut e, HUNGER, 0x77, 0x10c);
    plain(&mut e, SPEL, 0x14, 0x10d);
    plain(&mut e, LIST, 0x55, 0x10e);
    plain(&mut e, LIST2, 0x55, 0x10f);
    plain(&mut e, DOOR, 0x1c, 0x110);
    form(&mut e, ACTOR, 0x3b, 0x111);
    e.mem.set_u32(ACTOR, VT_REFR);
    e.mem.set_u32(ACTOR + 0x20, MISC);
    e.mem.set_u32(CELL_EX + 0xc0, WRLD);
    for (i, f) in [
        IDLE, CONT, MISC, NAVM, WRLD, CELL_EX, CELL_IN, CELL_X, REFR, NOTE, RADS, HUNGER, SPEL,
        LIST, LIST2, DOOR, ACTOR,
    ]
    .iter()
    .enumerate()
    {
        set_ref(&mut e, i as u16 + 1, *f);
    }
    e
}

/// Script reference indexes of the forms above.
const R_IDLE: u16 = 1;
const R_CONT: u16 = 2;
const R_MISC: u16 = 3;
const R_NAVM: u16 = 4;
const R_WRLD: u16 = 5;
const R_CELL_EX: u16 = 6;
const R_CELL_IN: u16 = 7;
const R_CELL_X: u16 = 8;
const R_REFR: u16 = 9;
const R_NOTE: u16 = 10;
const R_RADS: u16 = 11;
const R_HUNGER: u16 = 12;
const R_SPEL: u16 = 13;
const R_LIST: u16 = 14;
const R_LIST2: u16 = 15;
const R_ACTOR: u16 = 17;

/// A condition's eval through its record (+0x20) with `this` and the first parameter.
fn eval(e: &mut Engine, opcode: u32, this: u32, arg1: u32) -> f64 {
    let at = e.mem.u32(nvse::table::record_of(e, opcode) + 0x20);
    assert_ne!(at, 0, "{opcode:04x} has an eval");
    e.mem.set_f64(RESULT, 0.0);
    e.call(at, &[this, arg1, 0, RESULT]);
    e.mem.f64(RESULT)
}

/// The command and its eval agree.
fn both(e: &mut Engine, opcode: u32) -> f64 {
    let c = call(e, opcode, &[]).1;
    assert_eq!(eval(e, opcode, 0, 0), c, "{opcode:04x}: eval and command");
    c
}

#[test]
fn idle_loop_times() {
    let mut e = setup();
    e.mem.set_u8(IDLE + 0x39, 3);
    e.mem.set_u8(IDLE + 0x3a, 9);
    assert_eq!(call(&mut e, 0x26e9, &[r(R_IDLE), n(0)]).1, 3.0);
    assert_eq!(call(&mut e, 0x26e9, &[r(R_IDLE), n(-1)]).1, 9.0);
    // Another form type: nothing.
    assert_eq!(call(&mut e, 0x26e9, &[r(R_MISC), n(1)]).1, 0.0);
    call(&mut e, 0x2770, &[r(R_IDLE), n(4), n(7)]);
    assert_eq!((e.mem.u8(IDLE + 0x39), e.mem.u8(IDLE + 0x3a)), (4, 7));
    // The minimum capped at 255 (unsigned: -1 too), the maximum raised to it.
    call(&mut e, 0x2770, &[r(R_IDLE), n(300), n(2)]);
    assert_eq!((e.mem.u8(IDLE + 0x39), e.mem.u8(IDLE + 0x3a)), (255, 255));
    call(&mut e, 0x2770, &[r(R_IDLE), n(-1), n(0)]);
    assert_eq!((e.mem.u8(IDLE + 0x39), e.mem.u8(IDLE + 0x3a)), (255, 255));
    // -2 compared as signed would stay -2 (bytes 0xFE): unsigned, it is capped at 255.
    call(&mut e, 0x2770, &[r(R_IDLE), n(-2), n(0)]);
    assert_eq!((e.mem.u8(IDLE + 0x39), e.mem.u8(IDLE + 0x3a)), (255, 255));
    // A maximum above 255 keeps its low byte (the UInt8 store); the max equal to the min stays.
    call(&mut e, 0x2770, &[r(R_IDLE), n(5), n(0x105)]);
    assert_eq!((e.mem.u8(IDLE + 0x39), e.mem.u8(IDLE + 0x3a)), (5, 5));
    call(&mut e, 0x2770, &[r(R_IDLE), n(6), n(6)]);
    assert_eq!((e.mem.u8(IDLE + 0x39), e.mem.u8(IDLE + 0x3a)), (6, 6));
    // A wrong-typed form: its bytes unchanged.
    let before = e.mem.bytes(MISC, 0x50);
    call(&mut e, 0x2770, &[r(R_MISC), n(1), n(2)]);
    assert_eq!(e.mem.bytes(MISC, 0x50), before);
}

#[test]
fn nav_meshes() {
    let mut e = setup();
    e.register(MANAGER_FN, |_, w| {
        rec(MANAGER_FN, w);
        eax(MANAGER)
    });
    e.register(DISABLE_FN, |_, w| {
        rec(DISABLE_FN, w);
        eax(0)
    });
    e.register(ENABLE_FN, |_, w| {
        rec(ENABLE_FN, w);
        eax(0)
    });
    call(&mut e, 0x26ed, &[r(R_NAVM)]);
    call(&mut e, 0x26ee, &[r(R_NAVM)]);
    assert_eq!(calls_to(DISABLE_FN), vec![vec![MANAGER, 0x104]]);
    assert_eq!(calls_to(ENABLE_FN), vec![vec![MANAGER, 0x104]]);
    assert_eq!(calls_to(MANAGER_FN).len(), 2);
    // Not a nav mesh: no call.
    call(&mut e, 0x26ed, &[r(R_MISC)]);
    call(&mut e, 0x26ee, &[r(R_MISC)]);
    assert_eq!(calls_to(MANAGER_FN).len(), 2);
}

#[test]
fn terrain_height() {
    let mut e = setup();
    e.register(TERRAIN_FN, |e, w| {
        rec(TERRAIN_FN, w);
        assert_eq!(e.mem.f32(w[2]), 0.0, "the result starts at 0");
        let (x, y) = (e.mem.f32(w[1]), e.mem.f32(w[1] + 4));
        e.mem.set_f32(w[2], x * 2.0 + y);
        eax(1)
    });
    assert_eq!(call(&mut e, 0x26ef, &[z(10.5), z(-3.0)]).1, 18.0);
    assert_eq!(calls_to(TERRAIN_FN)[0][0], TES_AT);
    // The float result: 0.1 as a float.
    assert_eq!(
        call(&mut e, 0x26ef, &[z(0.05), z(0.0)]).1,
        f64::from(0.1f32)
    );
}

#[test]
fn container_respawns() {
    let mut e = setup();
    e.mem.set_u8(CONT + 0x98, 0x02);
    assert_eq!(call(&mut e, 0x2721, &[r(R_CONT)]).1, 1.0);
    // The caller's base form when no form is given.
    assert_eq!(call_on(&mut e, 0x2721, REFR, &[]).1, 1.0);
    assert_eq!(call(&mut e, 0x2721, &[]).1, 0.0, "no form, no caller");
    call(&mut e, 0x2722, &[n(0), r(R_CONT)]);
    assert_eq!(e.mem.u8(CONT + 0x98), 0);
    assert_eq!(call(&mut e, 0x2721, &[r(R_CONT)]).1, 0.0);
    e.mem.set_u8(CONT + 0x98, 0xfd);
    call_on(&mut e, 0x2722, REFR, &[n(5)]);
    assert_eq!(e.mem.u8(CONT + 0x98), 0xff);
    call_on(&mut e, 0x2722, REFR, &[n(0)]);
    assert_eq!(e.mem.u8(CONT + 0x98), 0xfd, "only bit 1 cleared");
    // Not a container: nothing read or written.
    e.mem.set_u8(MISC + 0x98, 2);
    assert_eq!(call(&mut e, 0x2721, &[r(R_MISC)]).1, 0.0);
    call(&mut e, 0x2722, &[n(0), r(R_MISC)]);
    assert_eq!(e.mem.u8(MISC + 0x98), 2);
}

/// A cell map of 4 buckets: (3, -2) and (7, -2) share bucket 2 (keys 0x0003FFFE, 0x0007FFFE);
/// the chain holds the second first.
fn cell_map(e: &mut Engine) {
    e.mem.set_u32(WRLD + 0x30, MAP);
    e.mem.set_u32(MAP + 4, 4);
    e.mem.set_u32(MAP + 8, MAP + 0x100);
    let (a, b) = (MAP + 0x200, MAP + 0x210);
    e.mem.set_u32(MAP + 0x100 + 4 * (0x0003_fffe % 4), b);
    e.mem.set_u32(b, a);
    e.mem.set_u32(b + 4, 0x0007_fffe);
    e.mem.set_u32(b + 8, CELL_X);
    e.mem.set_u32(a, 0);
    e.mem.set_u32(a + 4, 0x0003_fffe);
    e.mem.set_u32(a + 8, CELL_EX);
}

#[test]
fn exterior_cells() {
    let mut e = setup();
    assert_eq!(
        call(&mut e, 0x2736, &[r(R_WRLD), n(3), n(-2)]).1,
        0.0,
        "no map"
    );
    cell_map(&mut e);
    call(&mut e, 0x2736, &[r(R_WRLD), n(3), n(-2)]);
    assert_eq!(form_result(&e), 0x106);
    call(&mut e, 0x2736, &[r(R_WRLD), n(7), n(-2)]);
    assert_eq!(form_result(&e), 0x108);
    // The key's x in the high half: (-2, 3) is another key.
    assert_eq!(call(&mut e, 0x2736, &[r(R_WRLD), n(-2), n(3)]).1, 0.0);
    // A map with no buckets: nothing (JIP divides by 0).
    e.mem.set_u32(MAP + 4, 0);
    assert_eq!(call(&mut e, 0x2736, &[r(R_WRLD), n(3), n(-2)]).1, 0.0);
}

/// The buffers as `TES::TES` makes them: blocks of the game's heap, `setting * 4` bytes (the
/// heap rounds a block up to its size class). Answers (interiors, exteriors).
fn buffers(e: &mut Engine) -> (u32, u32) {
    let (buf_in, buf_ex) = (e.mem.alloc(2 * 4), e.mem.alloc(3 * 4));
    e.mem.set_u32(TES_AT + 0x38, buf_in);
    e.mem.set_u32(TES_AT + 0x3c, buf_ex);
    e.mem.set_u32(0x011c_3e38 + 4, 2);
    e.mem.set_u32(0x011c_3c90 + 4, 3);
    // Interiors: CELL_IN, MISC (not a cell: left out), then CELL_X past the 2 slots.
    for (i, c) in [CELL_IN, MISC, CELL_X].iter().enumerate() {
        e.mem.set_u32(buf_in + 4 * i as u32, *c);
    }
    // Exteriors: CELL_EX, an empty slot, CELL_X after it.
    for (i, c) in [CELL_EX, 0, CELL_X].iter().enumerate() {
        e.mem.set_u32(buf_ex + 4 * i as u32, *c);
    }
    (buf_in, buf_ex)
}

#[test]
fn buffered_cells() {
    let mut e = setup();
    let (_, buf_ex) = buffers(&mut e);
    assert_eq!(call(&mut e, 0x2737, &[r(R_CELL_EX)]).1, 1.0);
    assert_eq!(call(&mut e, 0x2737, &[r(R_CELL_IN)]).1, 1.0);
    // Past the setting's slot count, after an empty slot, in the other buffer: no.
    assert_eq!(call(&mut e, 0x2737, &[r(R_CELL_X)]).1, 0.0);
    e.mem.set_u32(CELL_X + 0xc0, WRLD);
    assert_eq!(call(&mut e, 0x2737, &[r(R_CELL_X)]).1, 0.0);
    e.mem.set_u32(buf_ex + 4, CELL_IN);
    assert_eq!(
        call(&mut e, 0x2737, &[r(R_CELL_X)]).1,
        1.0,
        "the empty slot filled"
    );
    // A form that is not a cell: nothing; the caller's parent cell without a form.
    assert_eq!(call(&mut e, 0x2737, &[r(R_MISC)]).1, 0.0);
    e.mem.set_u32(REFR + 0x40, CELL_IN);
    assert_eq!(call_on(&mut e, 0x2737, REFR, &[]).1, 1.0);
    e.mem.set_u32(REFR + 0x40, 0);
    assert_eq!(call_on(&mut e, 0x2737, REFR, &[]).1, 0.0);
    assert_eq!(call(&mut e, 0x2737, &[]).1, 0.0);
    // The arrays.
    let id = call(&mut e, 0x2853, &[n(1)]).1;
    assert_eq!(array_data(&e, id), vec![Data::Form(0x107)], "MISC left out");
    let id = call(&mut e, 0x2853, &[n(0)]).1;
    assert_eq!(
        array_data(&e, id),
        vec![Data::Form(0x106), Data::Form(0x107), Data::Form(0x108)]
    );
    // No buffer: an empty array.
    e.mem.set_u32(TES_AT + 0x38, 0);
    let id = call(&mut e, 0x2853, &[n(1)]).1;
    assert!(array_data(&e, id).is_empty());
    assert_eq!(call(&mut e, 0x2737, &[r(R_CELL_IN)]).1, 0.0);
}

#[test]
fn difficulty() {
    let mut e = setup();
    e.mem.set_u32(PC + 0x7b8, 3);
    assert_eq!(both(&mut e, 0x273f), 3.0);
    call(&mut e, 0x2838, &[n(4)]);
    assert_eq!(e.mem.u32(PC + 0x7b8), 4);
    call(&mut e, 0x2838, &[n(5)]);
    call(&mut e, 0x2838, &[n(-1)]);
    assert_eq!(e.mem.u32(PC + 0x7b8), 4, "above 4 (unsigned): kept");
    call(&mut e, 0x2838, &[n(0)]);
    assert_eq!(both(&mut e, 0x273f), 0.0);
    e.mem.set_u32(PC + 0x7b8, 0xffff_fffe);
    assert_eq!(both(&mut e, 0x273f), -2.0, "a signed word");
}

#[test]
fn first_person() {
    let mut e = setup();
    e.register(SET_FIRST_PERSON_FN, |_, w| {
        rec(SET_FIRST_PERSON_FN, w);
        eax(1)
    });
    // Third person: ToggleFirstPerson 1 switches, 0 does not.
    e.mem.set_u8(PC + 0x64a, 1);
    call(&mut e, 0x2740, &[n(0)]);
    assert!(calls_to(SET_FIRST_PERSON_FN).is_empty());
    call(&mut e, 0x2740, &[n(7)]);
    assert_eq!(calls_to(SET_FIRST_PERSON_FN), vec![vec![PC, 1]]);
    e.mem.set_u8(PC + 0x64a, 0);
    call(&mut e, 0x2740, &[n(1)]);
    call(&mut e, 0x2740, &[n(0)]);
    assert_eq!(
        calls_to(SET_FIRST_PERSON_FN),
        vec![vec![PC, 1], vec![PC, 0]]
    );
}

#[test]
fn detection_state() {
    let mut e = setup();
    e.register(DETECTION_FN, |e, w| {
        rec(DETECTION_FN, w);
        eax(e.mem.u32(FORMS + 0x7000))
    });
    assert_eq!(both(&mut e, 0x274a), -1.0, "no parent cell");
    e.mem.set_u32(PC + 0x40, CELL_IN);
    e.mem.set_u8(PC + 0xdf1, 1);
    e.mem.set_u8(PC + 0xdf0, 1);
    assert_eq!(both(&mut e, 0x274a), 1.0, "unseen first");
    e.mem.set_u8(PC + 0xdf1, 0);
    assert_eq!(both(&mut e, 0x274a), 3.0);
    e.mem.set_u8(PC + 0xdf0, 0);
    assert!(calls_to(DETECTION_FN).is_empty());
    e.mem.set_u32(FORMS + 0x7000, 0);
    assert_eq!(both(&mut e, 0x274a), 0.0);
    e.mem.set_u32(FORMS + 0x7000, (-5i32) as u32);
    assert_eq!(both(&mut e, 0x274a), 0.0, "signed");
    e.mem.set_u32(FORMS + 0x7000, 1);
    assert_eq!(both(&mut e, 0x274a), 2.0);
    assert_eq!(calls_to(DETECTION_FN)[0], vec![0x011e_0e80, PC, 0]);
}

#[test]
fn combat() {
    let mut e = setup();
    assert_eq!(both(&mut e, 0x26fc), 0.0);
    e.mem.set_u8(PC + 0xdf0, 1);
    assert_eq!(both(&mut e, 0x26fc), 1.0);
    e.mem.set_u8(PC + 0xdf1, 1);
    assert_eq!(both(&mut e, 0x26fc), 0.0, "unseen");
}

#[test]
fn player_flags() {
    let mut e = setup();
    assert_eq!(both(&mut e, 0x27aa), 0.0);
    assert_eq!(both(&mut e, 0x27ab), 0.0);
    e.mem.set_u8(PC + 0x651, 1);
    assert_eq!(both(&mut e, 0x27aa), 1.0);
    assert_eq!(both(&mut e, 0x27ab), 0.0);
    e.mem.set_u8(PC + 0x652, 2);
    assert_eq!(both(&mut e, 0x27ab), 1.0);
    e.mem.set_u8(PC + 0x651, 0);
    assert_eq!(both(&mut e, 0x27aa), 0.0);
}

#[test]
fn iron_sights() {
    let mut e = setup();
    e.register(IS_WEAPON_OUT_FN, |e, w| {
        rec(IS_WEAPON_OUT_FN, w);
        eax(0xffff_ff00 | e.mem.u32(FORMS + 0x7004))
    });
    e.mem.set_u32(FORMS + 0x7004, 1);
    assert_eq!(both(&mut e, 0x277a), 0.0, "no node");
    assert!(calls_to(IS_WEAPON_OUT_FN).is_empty());
    e.mem.set_u32(PC + 0xe34, FORMS + 0x9000);
    assert_eq!(both(&mut e, 0x277a), 1.0);
    assert_eq!(calls_to(IS_WEAPON_OUT_FN)[0], vec![PROCESS]);
    e.mem.set_u32(FORMS + 0x7004, 0);
    assert_eq!(both(&mut e, 0x277a), 0.0, "the bool in al");
    e.mem.set_u32(PC + 0x68, 0);
    assert_eq!(both(&mut e, 0x277a), 0.0, "no process");
}

#[test]
fn kill_cam() {
    let mut e = setup();
    e.mem.set_u32(0x011f_2258, 4);
    assert_eq!(both(&mut e, 0x287c), 0.0, "timer 0");
    e.mem.set_f32(PC + 0xe18, 0.5);
    assert_eq!(both(&mut e, 0x287c), 1.0);
    e.mem.set_u32(0x011f_2258, 3);
    assert_eq!(both(&mut e, 0x287c), 0.0);
    e.mem.set_u32(0x011f_2258, 4);
    e.mem.set_f32(PC + 0xe18, -1.0);
    assert_eq!(both(&mut e, 0x287c), 0.0);
    e.mem.set_f32(PC + 0xe18, f32::NAN);
    assert_eq!(both(&mut e, 0x287c), 0.0);
}

#[test]
fn last_exterior_door_and_health_target() {
    let mut e = setup();
    assert_eq!(call(&mut e, 0x277b, &[]).1, 0.0);
    e.mem.set_u32(PC + 0x604, DOOR);
    call(&mut e, 0x277b, &[]);
    assert_eq!(form_result(&e), 0x110);
    assert_eq!(call(&mut e, 0x283d, &[]).1, 0.0);
    e.mem.set_u32(HUD_AT + 0x21c, ACTOR);
    call(&mut e, 0x283d, &[]);
    assert_eq!(form_result(&e), 0x111);
    // No HUD: nothing.
    e.mem.set_u32(0x011d_96c0, 0);
    assert_eq!(call(&mut e, 0x283d, &[]).1, 0.0);
}

#[test]
fn scope() {
    let mut e = setup();
    assert_eq!(both(&mut e, 0x276a), 0.0);
    e.mem.set_u8(HUD_AT + 0x1fc, 1);
    // A byte: the next ones do not count.
    e.mem.set_u8(HUD_AT + 0x1fd, 0);
    assert_eq!(both(&mut e, 0x276a), 1.0);
    e.mem.set_u8(HUD_AT + 0x1fc, 0);
    e.mem.set_u8(HUD_AT + 0x1fd, 1);
    assert_eq!(both(&mut e, 0x276a), 0.0);
    e.mem.set_u8(HUD_AT + 0x1fc, 1);
    e.mem.set_u32(0x011d_96c0, 0);
    assert_eq!(both(&mut e, 0x276a), 0.0, "no HUD");
}

#[test]
fn pipboy_radio() {
    let mut e = setup();
    assert_eq!(call(&mut e, 0x2751, &[]).1, 0.0);
    // No reference: the eval matches no subject.
    assert_eq!(eval(&mut e, 0x2751, 0, 0), 1.0);
    assert_eq!(eval(&mut e, 0x2751, REFR, 0), 0.0);
    e.mem.set_u32(RADIO, REFR);
    call(&mut e, 0x2751, &[]);
    assert_eq!(form_result(&e), 0x109);
    assert_eq!(eval(&mut e, 0x2751, REFR, 0), 1.0);
    assert_eq!(eval(&mut e, 0x2751, ACTOR, 0), 0.0);
    e.mem.set_u32(0x011d_d42c, 0);
    assert_eq!(call(&mut e, 0x2751, &[]).1, 0.0);
    assert_eq!(eval(&mut e, 0x2751, 0, 0), 0.0, "no station");
}

#[test]
fn moon_textures() {
    let mut e = setup();
    let get = |e: &mut Engine, id: i32| {
        let res = call(e, 0x27a3, &[n(id)]).1;
        string_result(e, res)
    };
    assert_eq!(get(&mut e, 0), b"", "no moon");
    e.mem.set_u32(SKY_AT + 0x30, MOON);
    string(&mut e, MOON + 0x18, Some(b"textures\\sky\\masser_full.dds"));
    string(&mut e, MOON + 0x18 + 8 * 7, Some(b"seven.dds"));
    string(&mut e, MOON + 0x18 + 8 * 4, None);
    assert_eq!(get(&mut e, 0), b"textures\\sky\\masser_full.dds");
    assert_eq!(get(&mut e, 7), b"seven.dds");
    assert_eq!(get(&mut e, 4), b"", "no buffer");
    string(&mut e, MOON + 0x18 + 8 * 8, Some(b"past"));
    assert_eq!(get(&mut e, 8), b"", "8 and above");
    assert_eq!(get(&mut e, -1), b"");
    e.mem.set_u32(0x011d_ea20, 0);
    assert_eq!(get(&mut e, 0), b"", "no sky");
}

#[test]
fn freeze_time() {
    let mut e = setup();
    assert_eq!(call(&mut e, 0x28bf, &[]).1, 0.0);
    assert_eq!(call(&mut e, 0x28bf, &[n(1)]).1, 0.0, "the state before");
    assert_eq!(e.mem.u8(OSG + 7), 1);
    assert_eq!(call(&mut e, 0x28bf, &[]).1, 1.0);
    assert_eq!(e.mem.u8(OSG + 7), 1, "no argument: unchanged");
    call(&mut e, 0x28bf, &[n(5)]);
    assert_eq!(e.mem.u8(OSG + 7), 1);
    assert_eq!(call(&mut e, 0x28bf, &[n(0)]).1, 1.0);
    assert_eq!(e.mem.u8(OSG + 7), 0);
    call(&mut e, 0x28bf, &[n(0)]);
    assert_eq!(e.mem.u8(OSG + 7), 0);
    // A byte other than 0 and 1 matches neither `!toggle`: unchanged.
    e.mem.set_u8(OSG + 7, 2);
    call(&mut e, 0x28bf, &[n(0)]);
    call(&mut e, 0x28bf, &[n(1)]);
    assert_eq!(e.mem.u8(OSG + 7), 2);
}

#[test]
fn debug_modes() {
    let mut e = setup();
    let bytes = [
        0x011e_07ba,
        0x011f_1220,
        0x011a_4880,
        0x011c_3c0d,
        0x011c_ae44,
    ];
    for (i, b) in bytes.iter().enumerate() {
        assert_eq!(call(&mut e, 0x28bd, &[n(i as i32)]).1, 0.0);
        e.mem.set_u8(*b, 1);
        assert_eq!(call(&mut e, 0x28bd, &[n(i as i32)]).1, 1.0, "{i}");
        e.mem.set_u8(*b, 0);
    }
    for b in bytes {
        e.mem.set_u8(b, 1);
    }
    assert_eq!(call(&mut e, 0x28bd, &[n(5)]).1, 0.0);
    assert_eq!(call(&mut e, 0x28bd, &[n(-1)]).1, 0.0);
}

/// The time globals: year, month (from 0), day, hour.
fn date(e: &mut Engine, year: f32, month: f32, day: f32, hour: f32) {
    for (i, v) in [year, month, day, hour].iter().enumerate() {
        let g = GLOBALS + 0x40 * i as u32;
        e.mem.set_u32(0x011d_e7b8 + 4 * i as u32, g);
        e.mem.set_f32(g + 0x24, *v);
    }
}

#[test]
fn days_passed() {
    let mut e = setup();
    // The default date, 2281-10-13 (month 9 from 0): the game on 2281-10-19 at 6:00.
    date(&mut e, 2281.0, 9.0, 19.0, 6.0);
    assert_eq!(call(&mut e, 0x282f, &[]).1, f64::from(6.0f32 + 0.25));
    // A later month of the same year: the rest of October (31-13), November, then 4 days.
    date(&mut e, 2281.0, 11.0, 5.0, 0.0);
    assert_eq!(call(&mut e, 0x282f, &[]).1, 18.0 + 30.0 + 4.0);
    // A later year: the rest of 2281, 2282 whole, then January and 9 days of February 2284.
    date(&mut e, 2284.0, 1.0, 10.0, 12.0);
    let want = (18 + 30 + 31) + 2 * 365 + 31 + 9;
    assert_eq!(call(&mut e, 0x282f, &[]).1, f64::from(want as f32 + 0.5));
    // The year right after: no whole years.
    date(&mut e, 2282.0, 0.0, 1.0, 0.0);
    assert_eq!(call(&mut e, 0x282f, &[]).1, f64::from(18 + 30 + 31));
    // Before the date: negative days apart.
    date(&mut e, 2281.0, 9.0, 3.0, 0.0);
    assert_eq!(call(&mut e, 0x282f, &[]).1, -10.0);
    // A date given: 2280-12-31 to 2281-01-02.
    date(&mut e, 2281.0, 0.0, 2.0, 0.0);
    assert_eq!(call(&mut e, 0x282f, &[n(2280), n(12), n(31)]).1, 1.0);
    // Month 13: nothing; month 0 or below: deliberately nothing either (on a date where any count
    // would not be 0: 2281-01-10).
    date(&mut e, 2281.0, 0.0, 10.0, 0.0);
    assert_eq!(call(&mut e, 0x282f, &[n(2280), n(12), n(31)]).1, 9.0);
    assert_eq!(call(&mut e, 0x282f, &[n(2280), n(13), n(1)]).1, 0.0);
    assert_eq!(call(&mut e, 0x282f, &[n(2280), n(0), n(1)]).1, 0.0);
    assert_eq!(call(&mut e, 0x282f, &[n(2280), n(-3), n(1)]).1, 0.0);
    // The hour in single precision: 1/24 as a float times the hour.
    date(&mut e, 2281.0, 9.0, 13.0, 7.0);
    assert_eq!(
        call(&mut e, 0x282f, &[]).1,
        f64::from(7.0f32 * (1.0f32 / 24.0))
    );
}

#[test]
fn days_passed_echoes() {
    let mut e = setup();
    date(&mut e, 2281.0, 9.0, 14.0, 0.0);
    let (res, lines) = echoed(&mut e, 0x282f, 0, &[]);
    assert_eq!(res, 1.0);
    assert_eq!(lines.len(), 1);
    assert!(lines[0].starts_with(b"GetGameDaysPassed >> "), "{lines:?}");
    // No result (month 13): still echoed.
    let (_, lines) = echoed(&mut e, 0x282f, 0, &[n(2280), n(13), n(1)]);
    assert_eq!(lines.len(), 1);
}

#[test]
fn xp_rewards() {
    let mut e = setup();
    e.mem.set_u32(VT_REFR + 0x3a8, MOD_AV_FN);
    e.mem.set_u32(0x011f_6d98, FORMS + 0xa000);
    e.register(MOD_AV_FN, |_, w| {
        rec(MOD_AV_FN, w);
        eax(0)
    });
    e.register(PRECACHE_FN, |_, w| {
        rec(PRECACHE_FN, w);
        eax(0)
    });
    call(&mut e, 0x28ed, &[n(-25)]);
    assert_eq!(
        calls_to(MOD_AV_FN),
        vec![vec![PC, 0x18, (-25i32) as u32, 0]]
    );
    assert_eq!(
        calls_to(PRECACHE_FN),
        vec![vec![FORMS + 0xa000, 0x0106_f354, 0x121]]
    );
    assert_eq!(
        calls().iter().map(|c| c.0).collect::<Vec<_>>(),
        vec![MOD_AV_FN, PRECACHE_FN],
        "the value first, then the sound"
    );
}

#[test]
fn free_player_does_nothing() {
    let mut e = setup();
    let before = e.mem.bytes(PC, 0x100);
    assert_eq!(call(&mut e, 0x27b0, &[]), (1, 0.0));
    assert_eq!(e.mem.bytes(PC, 0x100), before);
    assert!(freed().is_empty());
}

/// Three actor value infos: 0 "Aggression", 1 none, 2 "Confidence"; 76 "XP".
fn av_infos(e: &mut Engine) {
    e.mem.set_u32(0x011d_61c8, 0);
    for (code, name) in [(0u32, &b"Aggression"[..]), (2, b"Confidence"), (76, b"XP")] {
        let info = AV_INFOS + 0x100 * code;
        let text = AV_INFOS + 0x80 + 0x100 * code;
        e.mem.write(text, name);
        e.mem.set_u8(text + name.len() as u32, 0);
        e.mem.set_u32(info + 0x38, text);
        e.mem.set_u32(0x011d_61c8 + 4 * code, info);
    }
}

#[test]
fn actor_values_by_name() {
    let mut e = setup();
    av_infos(&mut e);
    let code = |e: &mut Engine, t: &[u8]| call(e, 0x28a5, &[s(t)]).1;
    assert_eq!(code(&mut e, b"Aggression"), 0.0);
    assert_eq!(code(&mut e, b"confidence"), 2.0, "case-insensitive");
    assert_eq!(code(&mut e, b"XP"), 76.0);
    assert_eq!(code(&mut e, b"Karma"), -1.0);
    assert_eq!(code(&mut e, b""), -1.0);
    // Built once: a name changed later is not seen.
    e.mem.write(AV_INFOS + 0x80, b"Bggression");
    assert_eq!(code(&mut e, b"Aggression"), 0.0);
    assert_eq!(code(&mut e, b"Bggression"), -1.0);
    // Info 77 is past the table.
    e.mem.set_u32(0x011d_61c8 + 4 * 77, AV_INFOS + 0x200);
    assert_eq!(code(&mut e, b"Confidence"), 2.0);
}

#[test]
fn actor_values_built_lazily() {
    let mut e = setup();
    av_infos(&mut e);
    // A later info of the same hash replaces the code (operator[] assigns).
    let info = AV_INFOS + 0x100 * 5;
    e.mem.set_u32(info + 0x38, AV_INFOS + 0x80 + 0x200);
    e.mem.set_u32(0x011d_61c8 + 4 * 5, info);
    assert_eq!(call(&mut e, 0x28a5, &[s(b"CONFIDENCE")]).1, 5.0);
}

/// An info with no name: JIP's `StrHashCI(nullptr)` is 0, so it is kept under 0 (an empty
/// name hashes to the seed, not 0: not found).
#[test]
fn an_actor_value_without_a_name_is_kept_under_0() {
    let mut e = setup();
    av_infos(&mut e);
    let info = AV_INFOS + 0x100 * 6;
    e.mem.set_u32(info + 0x38, 0);
    e.mem.set_u32(0x011d_61c8 + 4 * 6, info);
    assert_eq!(call(&mut e, 0x28a5, &[s(b"")]).1, -1.0);
    assert_eq!(e.ext::<Jip>().misc.actor_value_ids.get(&0), Some(&6));
}

#[test]
fn hardcore_stages() {
    let mut e = setup();
    e.mem.set_u32(RADS + 0x18, 0xffff_fff6);
    assert_eq!(call(&mut e, 0x28b6, &[r(R_RADS)]).1, -10.0, "signed");
    call(&mut e, 0x28b7, &[r(R_HUNGER), n(600)]);
    assert_eq!(e.mem.u32(HUNGER + 0x18), 600);
    assert_eq!(call(&mut e, 0x28b6, &[r(R_HUNGER)]).1, 600.0);
    // Not a stage (0x76 and up, or 0x5A): nothing.
    let before = e.mem.bytes(MISC, 0x20);
    call(&mut e, 0x28b7, &[r(R_MISC), n(5)]);
    call(&mut e, 0x28b9, &[r(R_MISC), r(R_SPEL)]);
    assert_eq!(e.mem.bytes(MISC, 0x20), before);
    e.mem.set_u32(MISC + 0x18, 9);
    assert_eq!(call(&mut e, 0x28b6, &[r(R_MISC)]).1, 0.0);
    // A type just below the dehydration stage (0x75) is not one.
    e.mem.set_u8(MISC + 4, 0x75);
    assert_eq!(call(&mut e, 0x28b6, &[r(R_MISC)]).1, 0.0);
    e.mem.set_u8(MISC + 4, 0x78);
    assert_eq!(call(&mut e, 0x28b6, &[r(R_MISC)]).1, 9.0);
    // The dehydration stage itself (0x76) is one.
    e.mem.set_u8(MISC + 4, 0x76);
    assert_eq!(call(&mut e, 0x28b6, &[r(R_MISC)]).1, 9.0);
    // Effects.
    assert_eq!(call(&mut e, 0x28b8, &[r(R_RADS)]).1, 0.0);
    call(&mut e, 0x28b9, &[r(R_RADS), r(R_SPEL)]);
    assert_eq!(e.mem.u32(RADS + 0x1c), SPEL);
    call(&mut e, 0x28b8, &[r(R_RADS)]);
    assert_eq!(form_result(&e), 0x10d);
    call(&mut e, 0x28b9, &[r(R_RADS)]);
    assert_eq!(e.mem.u32(RADS + 0x1c), 0, "no spell: none");
}

#[test]
fn hardcore_tracking() {
    let mut e = setup();
    assert_eq!(both(&mut e, 0x28ad), 0.0);
    e.ext::<Jip>().saved.serialized.flags = 3;
    assert_eq!(
        both(&mut e, 0x28ad),
        0.0,
        "the copy changes when the flags are published"
    );
    publish_serialized_flags(&mut e);
    assert_eq!(both(&mut e, 0x28ad), 1.0);
    e.ext::<Jip>().saved.serialized.flags = 2;
    publish_serialized_flags(&mut e);
    assert_eq!(both(&mut e, 0x28ad), 0.0, "bit 0 only");
}

#[test]
fn notes_read() {
    let mut e = setup();
    assert_eq!(call(&mut e, 0x28b1, &[r(R_NOTE)]).1, 0.0);
    assert_eq!(eval(&mut e, 0x28b1, 0, NOTE), 0.0);
    e.mem.set_u8(NOTE + 0x7d, 1);
    assert_eq!(call(&mut e, 0x28b1, &[r(R_NOTE)]).1, 1.0);
    assert_eq!(eval(&mut e, 0x28b1, 0, NOTE), 1.0);
    e.mem.set_u8(NOTE + 0x7d, 0);
    e.mem.set_u8(NOTE + 0x7c, 1);
    assert_eq!(eval(&mut e, 0x28b1, 0, NOTE), 0.0, "the byte at +0x7D");
    assert_eq!(eval(&mut e, 0x28b1, 0, 0), 0.0, "no note");
}

#[test]
fn keywords() {
    let mut e = setup();
    let has = |e: &mut Engine, f: u16, k: &[u8]| call(e, 0x293b, &[r(f), s(k)]).1;
    assert_eq!(has(&mut e, R_MISC, b"metal"), 0.0);
    call(&mut e, 0x293a, &[r(R_MISC), s(b"Metal")]);
    assert_eq!(has(&mut e, R_MISC, b"METAL"), 1.0, "case-insensitive");
    assert_eq!(has(&mut e, R_CONT, b"metal"), 0.0);
    // A reference stands for its base form, both ways.
    assert_eq!(has(&mut e, R_ACTOR, b"metal"), 1.0);
    call(&mut e, 0x293a, &[r(R_REFR), s(b"metal")]);
    assert_eq!(has(&mut e, R_CONT, b"metal"), 1.0);
    assert!(!keyword_forms(&mut e, str_hash_ci(b"metal"))
        .unwrap()
        .contains(&REFR));
    // In address order.
    let id = call(&mut e, 0x293c, &[s(b"metal")]).1;
    assert_eq!(
        array_data(&e, id),
        vec![Data::Form(0x102), Data::Form(0x103)]
    );
    // No such keyword: an empty array.
    let id = call(&mut e, 0x293c, &[s(b"wood")]).1;
    assert!(array_data(&e, id).is_empty());
    // The condition: the hash as its parameter, the subject's base form.
    let hash = str_hash_ci(b"metal");
    let cond = |e: &mut Engine, this: u32, h: u32| {
        let at = e.mem.u32(nvse::table::record_of(e, 0x293e) + 0x20);
        e.mem.set_f64(RESULT, 0.0);
        e.call(at, &[this, h, 0, RESULT]);
        e.mem.f64(RESULT)
    };
    assert_eq!(cond(&mut e, ACTOR, hash), 1.0);
    assert_eq!(cond(&mut e, REFR, hash), 1.0);
    assert_eq!(cond(&mut e, ACTOR, hash ^ 1), 0.0);
    assert_eq!(cond(&mut e, 0, hash), 0.0, "no subject");
    e.mem.set_u32(REFR + 0x20, IDLE);
    assert_eq!(cond(&mut e, REFR, hash), 0.0);
}

#[test]
fn keywords_from_form_lists() {
    let mut e = setup();
    // LIST: MISC, an empty item, REFR (its base CONT), LIST2; LIST2: IDLE, LIST (a loop).
    e.mem.set_u32(LIST + 0x18, MISC);
    let (n1, n2, n3) = (FORMS + 0xb000, FORMS + 0xb008, FORMS + 0xb010);
    e.mem.set_u32(LIST + 0x1c, n1);
    e.mem.set_u32(n1, 0);
    e.mem.set_u32(n1 + 4, n2);
    e.mem.set_u32(n2, REFR);
    e.mem.set_u32(n2 + 4, n3);
    e.mem.set_u32(n3, LIST2);
    e.mem.set_u32(n3 + 4, 0);
    e.mem.set_u32(LIST2 + 0x18, IDLE);
    let n4 = FORMS + 0xb018;
    e.mem.set_u32(LIST2 + 0x1c, n4);
    e.mem.set_u32(n4, LIST);
    e.mem.set_u32(n4 + 4, 0);
    call(&mut e, 0x293a, &[r(R_LIST), s(b"loot")]);
    let set = keyword_forms(&mut e, str_hash_ci(b"loot")).unwrap();
    assert_eq!(
        set,
        vec![IDLE, CONT, MISC],
        "no list in the set, references as bases"
    );
    assert_eq!(call(&mut e, 0x293b, &[r(R_LIST), s(b"loot")]).1, 0.0);
    // An empty list still makes the keyword's set.
    e.mem.set_u32(LIST2 + 0x18, 0);
    e.mem.set_u32(LIST2 + 0x1c, 0);
    call(&mut e, 0x293a, &[r(R_LIST2), s(b"none")]);
    assert_eq!(keyword_forms(&mut e, str_hash_ci(b"none")), Some(vec![]));
    assert_eq!(keyword_forms(&mut e, str_hash_ci(b"never")), None);
}

/// A keyword of 0x80 bytes runs past JIP's buffer: the port stops without the list guard row.
#[test]
#[should_panic(expected = "runs past JIP's 128-byte buffer")]
fn a_keyword_past_the_buffer() {
    let mut e = setup();
    call(&mut e, 0x293a, &[r(R_MISC), s(&[b'k'; 0x80])]);
}

#[test]
fn a_keyword_one_short_of_the_buffer() {
    let mut e = setup();
    call(&mut e, 0x293a, &[r(R_MISC), s(&[b'k'; 0x7f])]);
    assert_eq!(call(&mut e, 0x293b, &[r(R_MISC), s(&[b'K'; 0x7f])]).1, 1.0);
}

/// An actor value name of 0x40 bytes runs past JIP's `avStr[0x40]`.
#[test]
#[should_panic(expected = "runs past JIP's 64-byte buffer")]
fn an_actor_value_name_past_the_buffer() {
    let mut e = setup();
    call(&mut e, 0x28a5, &[s(&[b'a'; 0x40])]);
}

/// No `TES`: no call, the result 0 written.
#[test]
fn terrain_height_without_tes() {
    let mut e = setup();
    e.register(TERRAIN_FN, |_, w| {
        rec(TERRAIN_FN, w);
        eax(1)
    });
    e.mem.set_u32(0x011d_ea10, 0);
    e.mem.set_f64(RESULT, 7.0);
    let mut words = crate::test_support::line(&mut e, 0x26ef, &[z(1.0), z(2.0)]);
    words[2] = 0;
    e.mem.set_f64(RESULT, 7.0);
    let exec = crate::test_support::execute_of(&e, 0x26ef);
    e.call(exec, &words);
    assert_eq!(e.mem.f64(RESULT), 0.0, "written always");
    assert!(calls_to(TERRAIN_FN).is_empty());
}

/// A cell map entry holding no cell: no result.
#[test]
fn an_exterior_cell_entry_without_a_cell() {
    let mut e = setup();
    cell_map(&mut e);
    e.mem.set_u32(MAP + 0x210 + 8, 0);
    assert_eq!(call(&mut e, 0x2736, &[r(R_WRLD), n(7), n(-2)]).1, 0.0);
    // The walk goes on past it to the key only: (3, -2) is still found.
    call(&mut e, 0x2736, &[r(R_WRLD), n(3), n(-2)]);
    assert_eq!(form_result(&e), 0x106);
}

/// A cell map of 701 buckets, the count `TESWorldSpace::Load` (`00583560`, at `00583FF5`) gives
/// a worldspace that is not large (7001 for a large one): odd, so `key % count` and a
/// power-of-two mask (`key & (count - 1)`) pick different buckets. (-3, 5) has a negative x
/// (key 0xFFFD0005), (2, -1) a negative y whose high half must not reach x's (0x0002FFFF).
#[test]
fn an_odd_bucket_cell_map() {
    let mut e = setup();
    let map = FORMS + 0x4_0000;
    let buckets = map + 0x100;
    e.mem.set_u32(WRLD + 0x30, map);
    e.mem.set_u32(map + 4, 701);
    e.mem.set_u32(map + 8, buckets);
    for (i, (key, cell)) in [(0xfffd_0005u32, CELL_EX), (0x0002_ffff, CELL_X)]
        .into_iter()
        .enumerate()
    {
        assert_ne!(key % 701, key & 700, "{key:08x}: the buckets differ");
        let entry = map + 0x1000 + 0x10 * i as u32;
        e.mem.set_u32(buckets + 4 * (key % 701), entry);
        e.mem.set_u32(entry, 0);
        e.mem.set_u32(entry + 4, key);
        e.mem.set_u32(entry + 8, cell);
    }
    call(&mut e, 0x2736, &[r(R_WRLD), n(-3), n(5)]);
    assert_eq!(form_result(&e), 0x106);
    call(&mut e, 0x2736, &[r(R_WRLD), n(2), n(-1)]);
    assert_eq!(form_result(&e), 0x108);
    // x and y swapped: other keys, nothing.
    assert_eq!(call(&mut e, 0x2736, &[r(R_WRLD), n(5), n(-3)]).1, 0.0);
    assert_eq!(call(&mut e, 0x2736, &[r(R_WRLD), n(-1), n(2)]).1, 0.0);
}

/// A script raises `uExterior Cell Buffer` past the slots `TES::TES` made: the walk stops at the
/// heap block's end (non-zero words after it are not read) and leaves out the words in the
/// block's rounding that are not cells.
#[test]
fn buffered_cells_stop_at_the_block_the_game_made() {
    let mut e = setup();
    let (_, buf_ex) = buffers(&mut e);
    let far = plain(&mut e, FORMS + 0x2100, 0x39, 0x120);
    let words = e.mem.block_size(buf_ex).unwrap() / 4;
    assert!(words >= 3);
    for (i, c) in [CELL_EX, CELL_IN, CELL_X].iter().enumerate() {
        e.mem.set_u32(buf_ex + 4 * i as u32, *c);
    }
    for i in 3..words {
        e.mem.set_u32(buf_ex + 4 * i, MISC);
    }
    let after = buf_ex + 4 * words;
    let kept = (e.mem.u32(after), e.mem.u32(after + 4));
    e.mem.set_u32(after, far);
    e.mem.set_u32(after + 4, far);
    e.mem.set_u32(0x011c_3c90 + 4, 1000);
    let id = call(&mut e, 0x2853, &[n(0)]).1;
    assert_eq!(
        array_data(&e, id),
        vec![Data::Form(0x106), Data::Form(0x107), Data::Form(0x108)]
    );
    e.mem.set_u32(far + 0xc0, WRLD);
    set_ref(&mut e, 19, far);
    assert_eq!(call(&mut e, 0x2737, &[r(19)]).1, 0.0, "after the block");
    // A lowered setting still bounds the walk (JIP's bound).
    e.mem.set_u32(0x011c_3c90 + 4, 1);
    let id = call(&mut e, 0x2853, &[n(0)]).1;
    assert_eq!(array_data(&e, id), vec![Data::Form(0x106)]);
    e.mem.set_u32(after, kept.0);
    e.mem.set_u32(after + 4, kept.1);
    // A buffer that is not a heap block: no slots known, no cells.
    e.mem.set_u32(TES_AT + 0x3c, FORMS + 0x4_0000);
    e.mem.set_u32(FORMS + 0x4_0000, CELL_EX);
    e.mem.set_u32(0x011c_3c90 + 4, 3);
    assert_eq!(call(&mut e, 0x2737, &[r(R_CELL_EX)]).1, 0.0);
}

/// The conditions read the lock-free copies: while a second engine of the run (a real thread's)
/// holds JIP's shared slot, both answer on this one (asking for the slot here would be refused:
/// two engines on one thread), and the second engine reads the same keyword map and flags.
#[test]
fn the_conditions_answer_while_another_engine_holds_jips_state() {
    let mut e = setup();
    call(&mut e, 0x293a, &[r(R_MISC), s(b"metal")]);
    e.ext::<Jip>().saved.serialized.flags = 1;
    publish_serialized_flags(&mut e);
    let mut other = Engine::new_untyped_for_test();
    e.share_slots_for_test(&mut other);
    engine_core::extend::release_held_for_test();
    assert!(other.ext::<Jip>().installed, "the other engine holds it");
    let hash = str_hash_ci(b"metal");
    assert_eq!(eval(&mut e, 0x293e, ACTOR, hash), 1.0, "ACTOR's base");
    assert_eq!(eval(&mut e, 0x293e, ACTOR, hash ^ 1), 0.0);
    assert_eq!(eval(&mut e, 0x28ad, 0, 0), 1.0);
    assert!(keyword_has(&mut other, hash, MISC), "one map for the run");
    assert_eq!(serialized_flags(&mut other), 1, "one copy for the run");
    engine_core::extend::release_held_for_test();
}

/// A form list at `at` holding `items`: its first node inline (+0x18), the others from +0x20 (8
/// bytes each).
fn form_list(e: &mut Engine, at: u32, items: &[u32]) -> u32 {
    plain(e, at, 0x55, 0);
    let mut node = at + 0x18;
    for (i, &item) in items.iter().enumerate() {
        let next = if i + 1 < items.len() {
            at + 0x20 + 8 * i as u32
        } else {
            0
        };
        e.mem.set_u32(node, item);
        e.mem.set_u32(node + 4, next);
        node = next;
    }
    at
}

/// The keyword walk of `list` with each list's reads counted.
fn walk_counted(e: &mut Engine, list: u32) -> (BTreeSet<u32>, BTreeMap<u32, u32>) {
    let mut set = BTreeSet::new();
    let mut reads = BTreeMap::new();
    assign_list_reading(e, &mut set, list, &mut |e, l| {
        *reads.entry(l).or_insert(0) += 1;
        list_items(e, Ptr::new(l + 0x18))
    });
    (set, reads)
}

/// A diamond: TOP holds LEFT and RIGHT, both hold SHARED (MISC, REFR): SHARED's items are read
/// once; the forms as JIP's walk gives them (REFR as its base CONT).
#[test]
fn a_form_list_diamond_reads_each_list_once() {
    let mut e = setup();
    let lists = FORMS + 0x5_0000;
    let shared = form_list(&mut e, lists, &[MISC, REFR]);
    let left = form_list(&mut e, lists + 0x40, &[shared, IDLE]);
    let right = form_list(&mut e, lists + 0x80, &[shared]);
    let top = form_list(&mut e, lists + 0xc0, &[left, right]);
    let (set, reads) = walk_counted(&mut e, top);
    assert_eq!(set, BTreeSet::from([IDLE, CONT, MISC]));
    assert_eq!(
        reads,
        BTreeMap::from([(top, 1), (left, 1), (right, 1), (shared, 1)])
    );
    // A list that holds itself: read once too, the first list included.
    let own = form_list(&mut e, lists + 0x100, &[lists + 0x100, MISC]);
    let (set, reads) = walk_counted(&mut e, own);
    assert_eq!(set, BTreeSet::from([MISC]));
    assert_eq!(reads, BTreeMap::from([(own, 1)]));
    // Through the command: the same set.
    set_ref(&mut e, 18, top);
    call(&mut e, 0x293a, &[r(18), s(b"diamond")]);
    assert_eq!(
        keyword_forms(&mut e, str_hash_ci(b"diamond")),
        Some(vec![IDLE, CONT, MISC])
    );
}

/// 40 diamonds in a row (81 lists; JIP walks 2^40 paths): each list read once, at once.
#[test]
fn a_chain_of_40_diamonds_finishes() {
    let mut e = setup();
    let lists = FORMS + 0x5_0000;
    let mut below = form_list(&mut e, lists, &[MISC]);
    let mut all = vec![below];
    for level in 0..40u32 {
        let a = form_list(&mut e, lists + 0x40 + 0x80 * level, &[below, IDLE]);
        let b = form_list(&mut e, lists + 0x80 + 0x80 * level, &[below]);
        below = form_list(&mut e, lists + 0x4000 + 0x40 * level, &[a, b]);
        all.extend([a, b, below]);
    }
    let (set, reads) = walk_counted(&mut e, below);
    assert_eq!(set, BTreeSet::from([IDLE, MISC]));
    assert_eq!(reads.len(), all.len());
    assert!(reads.values().all(|&n| n == 1));
}

// ---- package C2 ----

const C2: u32 = FORMS + 0x8_0000;
/// The event list's locals (as lane G's tests make them): variable `i` (1 to 6) at
/// `LOCALS + 0x100 + 0x10 * (i - 1)`, its value +8.
const LOCALS: u32 = crate::test_support::LIST + 0x100;
const DESC_VT: u32 = C2 + 0x100;
const DESC_GET_FN: u32 = 0x00de_c210;
const CAST_FN: u32 = 0x00ec_43fb;
const DESCRIBED: u32 = C2 + 0x400;
const DESC: u32 = C2 + 0x500;
const NOTE_TEXT_DESC: u32 = C2 + 0x540;
const LSCR: u32 = C2 + 0x600;
const TEXTS: u32 = C2 + 0x800;

/// A script local variable argument (`f` and its id).
fn var(id: u16) -> Vec<u8> {
    let mut b = vec![b'f'];
    b.extend_from_slice(&id.to_le_bytes());
    b
}

/// `n` locals with ids 1 to n, each -1.
fn c2_locals(e: &mut Engine, n: u32) {
    for i in 0..n {
        let local = LOCALS + 0x100 + i * 0x10;
        e.mem.set_u32(local, i + 1);
        e.mem.set_f64(local + 8, -1.0);
        let node = LOCALS + i * 8;
        e.mem.set_u32(node, local);
        e.mem
            .set_u32(node + 4, if i + 1 < n { node + 8 } else { 0 });
    }
    e.mem.set_u32(crate::test_support::LIST + 0xc, LOCALS);
}

fn local_value(e: &Engine, id: u32) -> f64 {
    e.mem.f64(LOCALS + 0x100 + (id - 1) * 0x10 + 8)
}

/// A text at `TEXTS + 0x40 * n`.
fn text(e: &mut Engine, n: u32, t: &[u8]) -> u32 {
    let at = TEXTS + 0x40 * n;
    e.mem.write(at, t);
    e.mem.set_u8(at + t.len() as u32, 0);
    at
}

/// The descriptions: `DESCRIBED` (a misc item) has its `TESDescription` at `DESC` (the cast
/// answers it), the note its text `NOTE_TEXT_DESC`; their `Get` (slot +0x10) records `(this,
/// form, chunk)` and answers the text at this +4.
fn descriptions(e: &mut Engine) {
    plain(e, DESCRIBED, 0x1f, 0x201);
    plain(e, LSCR, 0x4b, 0x202);
    set_ref(e, 30, DESCRIBED);
    set_ref(e, 31, LSCR);
    for d in [DESC, NOTE_TEXT_DESC] {
        e.mem.set_u32(d, DESC_VT);
    }
    e.mem.set_u32(DESC_VT + 0x10, DESC_GET_FN);
    let t = text(e, 0, b"a pistol");
    e.mem.set_u32(DESC + 4, t);
    let t = text(e, 1, b"a note's words");
    e.mem.set_u32(NOTE_TEXT_DESC + 4, t);
    e.register(DESC_GET_FN, |e, w| {
        rec(DESC_GET_FN, w);
        eax(e.mem.u32(w[0] + 4))
    });
    e.register(CAST_FN, |_, w| {
        rec(CAST_FN, w);
        let yes = w[0] == DESCRIBED && w[2] == 0x0118_3028 && w[3] == 0x0118_38f8;
        eax(if yes { DESC } else { 0 })
    });
}

fn description(e: &mut Engine, index: u16) -> Vec<u8> {
    let res = call(e, 0x271b, &[r(index)]).1;
    string_result(e, res)
}

#[test]
fn form_descriptions() {
    let mut e = setup();
    descriptions(&mut e);
    // A form's TESDescription: Get(form, 'DESC').
    assert_eq!(description(&mut e, 30), b"a pistol");
    assert_eq!(
        calls_to(DESC_GET_FN),
        vec![vec![DESC, DESCRIBED, 0x4353_4544]]
    );
    // A form without one: "" (no Get).
    assert_eq!(description(&mut e, R_MISC), b"");
    assert_eq!(calls_to(DESC_GET_FN).len(), 1);
    // A note of type 1: its text's Get(note, 'TNAM'), with no cast.
    e.mem.set_u32(NOTE + 0x6c, NOTE_TEXT_DESC);
    e.mem.set_u8(NOTE + 0x7c, 1);
    // The read flag next to the type (+0x7D) set: the type is one byte.
    e.mem.set_u8(NOTE + 0x7d, 1);
    CALLS.with(|c| c.borrow_mut().clear());
    assert_eq!(description(&mut e, R_NOTE), b"a note's words");
    assert_eq!(
        calls_to(DESC_GET_FN),
        vec![vec![NOTE_TEXT_DESC, NOTE, 0x4d41_4e54]]
    );
    assert!(calls_to(CAST_FN).is_empty());
    // A note of another type, or of type 1 without its text: "".
    e.mem.set_u8(NOTE + 0x7c, 2);
    assert_eq!(description(&mut e, R_NOTE), b"");
    e.mem.set_u8(NOTE + 0x7c, 1);
    e.mem.set_u32(NOTE + 0x6c, 0);
    assert_eq!(description(&mut e, R_NOTE), b"");
    assert_eq!(calls_to(DESC_GET_FN).len(), 1);
    // A load screen: its text when its length is not 0.
    let t = text(&mut e, 2, b"Welcome");
    e.mem.set_u32(LSCR + 0x38, t);
    e.mem.set_u16(LSCR + 0x3c, 7);
    assert_eq!(description(&mut e, 31), b"Welcome");
    // Length 0 with a buffer size (+0x3E) set: still "" (the length is the 16 bits at +0x3C).
    e.mem.set_u16(LSCR + 0x3c, 0);
    e.mem.set_u16(LSCR + 0x3e, 0x20);
    assert_eq!(description(&mut e, 31), b"");
    // Get answering no text: "".
    e.mem.set_u32(DESC + 4, 0);
    assert_eq!(description(&mut e, 30), b"");
    // No argument: "".
    let res = call(&mut e, 0x271b, &[]).1;
    assert_eq!(string_result(&e, res), b"");
}

/// The worlds: the exterior `bhkWorldM` (`*011CA0D8`) with its `hkpWorld` at +8, and an interior
/// cell whose `ExtraHavok` holds another world.
const EXT_BHK: u32 = C2 + 0x1000;
const EXT_HKP: u32 = C2 + 0x1100;
const INT_CELL: u32 = C2 + 0x1200;
const INT_XHAVOK: u32 = C2 + 0x1300;
const INT_BHK: u32 = C2 + 0x1400;
const INT_HKP: u32 = C2 + 0x1500;

fn worlds(e: &mut Engine) {
    extra_stand_ins(e);
    e.mem.set_u32(0x011c_a0d8, EXT_BHK);
    e.mem.set_u32(EXT_BHK + 8, EXT_HKP);
    for (i, g) in [1.0f32, 2.0, -9.8].iter().enumerate() {
        e.mem.set_f32(EXT_HKP + 0x10 + 4 * i as u32, *g);
    }
    put_extra(e, INT_CELL + 0x28, INT_XHAVOK, 1, INT_BHK);
    e.mem.set_u32(INT_BHK + 8, INT_HKP);
    for (i, g) in [4.0f32, 5.0, -6.0].iter().enumerate() {
        e.mem.set_f32(INT_HKP + 0x10 + 4 * i as u32, *g);
    }
}

fn axis(a: u8) -> Vec<u8> {
    vec![a]
}

#[test]
fn local_gravity() {
    let mut e = setup();
    worlds(&mut e);
    // The exterior world's gravity, each axis.
    assert_eq!(call(&mut e, 0x27b2, &[axis(b'X')]).1, 1.0);
    assert_eq!(call(&mut e, 0x27b2, &[axis(b'Y')]).1, 2.0);
    assert_eq!(call(&mut e, 0x27b2, &[axis(b'Z')]).1, f64::from(-9.8f32));
    // In an interior: its ExtraHavok's world.
    e.mem.set_u32(TES_AT + 0x34, INT_CELL);
    assert_eq!(call(&mut e, 0x27b2, &[axis(b'Z')]).1, -6.0);
    assert_eq!(call(&mut e, 0x27b2, &[axis(b'X')]).1, 4.0);
    // Echoed in the console.
    let (res, lines) = echoed(&mut e, 0x27b2, 0, &[axis(b'Y')]);
    assert_eq!((res, lines.len()), (5.0, 1));
    // An interior without ExtraHavok: no world, nothing.
    e.mem.set_u32(INT_CELL + 0x28 + 4, 0);
    assert_eq!(call(&mut e, 0x27b2, &[axis(b'X')]).1, 0.0);
    // The setter: three floats into the world's gravity; no result.
    e.mem.set_u32(TES_AT + 0x34, 0);
    assert_eq!(call(&mut e, 0x27b3, &[z(0.5), z(-1.5), z(-20.0)]).1, 0.0);
    let g = [0, 4, 8].map(|o| e.mem.f32(EXT_HKP + 0x10 + o));
    assert_eq!(g, [0.5, -1.5, -20.0]);
    assert_eq!(call(&mut e, 0x27b2, &[axis(b'Y')]).1, -1.5);
    // The interior's world gets it there.
    e.mem.set_u32(TES_AT + 0x34, INT_CELL);
    put_extra(&mut e, INT_CELL + 0x28, INT_XHAVOK, 1, INT_BHK);
    call(&mut e, 0x27b3, &[z(7.0), z(8.0), z(9.0)]);
    let g = [0, 4, 8].map(|o| e.mem.f32(INT_HKP + 0x10 + o));
    assert_eq!(g, [7.0, 8.0, 9.0]);
    assert_eq!(e.mem.f32(EXT_HKP + 0x14), -1.5, "the exterior's untouched");
    // No world (no exterior world and not inside): nothing written or read.
    e.mem.set_u32(TES_AT + 0x34, 0);
    e.mem.set_u32(0x011c_a0d8, 0);
    assert_eq!(call(&mut e, 0x27b2, &[axis(b'X')]).1, 0.0);
    call(&mut e, 0x27b3, &[z(1.0), z(1.0), z(1.0)]);
    assert_eq!(e.mem.f32(EXT_HKP + 0x10), 0.5);
}

/// The ray cast's scene: the player's controller filter (`0x12340042`), the main camera at
/// (100, 200, 300) whose rotation [0,1,0, 0,0,1, 1,0,0] has column 0 +Z (its view), column 1 +X
/// and row 0 +Y (not symmetric: a row read for column 0 casts elsewhere), the player's camera
/// position (1, 2, 3), and a pick stand-in that copies the record into `PICKED` and answers the
/// object and fraction set at `PICK_ANSWER`.
const CTRL: u32 = C2 + 0x2000;
const PHANTOM: u32 = C2 + 0x2800;
const HKOBJ: u32 = C2 + 0x2900;
const SG: u32 = C2 + 0x3000;
const CAM: u32 = C2 + 0x3100;
const PICK_ANSWER: u32 = C2 + 0x3400;
const PICKED: u32 = C2 + 0x3500;
const HIT: u32 = C2 + 0x3800;
const FADE: u32 = C2 + 0x3a00;
const NOT_ACTOR_VT: u32 = C2 + 0x3c00;
const NOT_ACTOR: u32 = C2 + 0x3e00;
const PICK_FN: u32 = 0x0045_8440;
const LIMB_FN: u32 = 0x008b_3ef0;
const K: u32 = 0x3e12_4dd2;
const K_BACK: u32 = 0x40df_f8d6;

fn scene(e: &mut Engine) {
    e.mem.set_u32(PROCESS + 0x138, CTRL);
    e.mem.set_u32(CTRL + 0x594, PHANTOM);
    e.mem.set_u32(PHANTOM + 8, HKOBJ);
    e.mem.set_u32(HKOBJ + 0x2c, 0x1234_0042);
    e.mem.set_u32(PC + 0x40, CELL_IN);
    e.mem.set_u32(0x011d_eb7c, SG);
    e.mem.set_u32(SG + 0xac, CAM);
    let rot = [0.0f32, 1.0, 0.0, 0.0, 0.0, 1.0, 1.0, 0.0, 0.0];
    for (i, v) in rot.iter().enumerate() {
        e.mem.set_f32(CAM + 0x68 + 4 * i as u32, *v);
    }
    for (i, v) in [100.0f32, 200.0, 300.0, 1.0].iter().enumerate() {
        e.mem.set_f32(CAM + 0x8c + 4 * i as u32, *v);
    }
    for (i, v) in [1.0f32, 2.0, 3.0, 0.0].iter().enumerate() {
        e.mem.set_f32(PC + 0xde0 + 4 * i as u32, *v);
    }
    // In an interior: a cast counts as found even without a hit.
    e.mem.set_u32(TES_AT + 0x34, CELL_IN);
    e.mem.set_u32(PICK_ANSWER, 0);
    e.mem.set_f32(PICK_ANSWER + 4, 0.5);
    e.register(PICK_FN, |e, w| {
        rec(PICK_FN, w);
        for i in 0..0x30 {
            e.mem.set_u32(PICKED + 4 * i, e.mem.u32(w[1] + 4 * i));
        }
        e.mem.set_f32(w[1] + 0x40, e.mem.f32(PICK_ANSWER + 4));
        eax(e.mem.u32(PICK_ANSWER))
    });
}

/// No hit and not inside: the cast does not count as found.
fn not_found(e: &mut Engine) {
    e.mem.set_u32(TES_AT + 0x34, 0);
    e.mem.set_u32(TES_AT + 8, 0);
    e.mem.set_u32(PICK_ANSWER, 0);
}

fn picked(e: &Engine) -> ([f32; 4], [f32; 4], u32) {
    let v = |at: u32| [0, 4, 8, 12].map(|o| e.mem.f32(at + o));
    (v(PICKED), v(PICKED + 0x10), e.mem.u32(PICKED + 0x24))
}

/// JIP's `_GetRayCastObject` ends: `from = pos * k` (three lanes), `to = (dir * range) * k +
/// from`.
fn cast_ends(pos: [f32; 3], dir: [f32; 3], range: f32) -> ([f32; 3], [f32; 3]) {
    let k = f32::from_bits(K);
    let from = pos.map(|p| p * k);
    let to = [0, 1, 2].map(|i| (dir[i] * range) * k + from[i]);
    (from, to)
}

/// `RayCastCoords`'s point `(from + (to - from) * f) * k'`.
fn cast_point(from: [f32; 3], to: [f32; 3], f: f32) -> [f32; 3] {
    let kb = f32::from_bits(K_BACK);
    [0, 1, 2].map(|i| (from[i] + (to[i] - from[i]) * f) * kb)
}

fn first3(v: [f32; 4]) -> [f32; 3] {
    [v[0], v[1], v[2]]
}

#[test]
fn reticle_pos() {
    let mut e = setup();
    scene(&mut e);
    // From the main camera along its rotation's column 0 (+Z), 50000 units, layer -1 (6).
    let id = call(&mut e, 0x27f1, &[]).1;
    let (from, to, filter) = picked(&e);
    let (f3, t3) = cast_ends([100.0, 200.0, 300.0], [0.0, 0.0, 1.0], 50000.0);
    assert_eq!((first3(from), first3(to)), (f3, t3));
    assert_eq!(filter, 0x1234_0006);
    let p = cast_point(f3, t3, 0.5);
    let got: Vec<f64> = array_data(&e, id)
        .into_iter()
        .map(|d| match d {
            Data::Number(n) => n,
            other => panic!("{other:?}"),
        })
        .collect();
    assert_eq!(got, p.map(f64::from).to_vec());
    // The layer and the range: argument 0 and argument 1.
    call(&mut e, 0x27f1, &[n(3), z(10.0)]);
    let (_, to, filter) = picked(&e);
    assert_eq!(filter, 0x1234_0003);
    let (_, t3) = cast_ends([100.0, 200.0, 300.0], [0.0, 0.0, 1.0], 10.0);
    assert_eq!(first3(to), t3);
    // Not found: no array, unless both arguments were given.
    not_found(&mut e);
    assert_eq!(call(&mut e, 0x27f1, &[]).1, 0.0);
    assert_eq!(call(&mut e, 0x27f1, &[n(6)]).1, 0.0);
    assert_ne!(call(&mut e, 0x27f1, &[n(6), z(100.0)]).1, 0.0);
    // A hit counts as found.
    e.mem.set_u32(PICK_ANSWER, HIT);
    assert_ne!(call(&mut e, 0x27f1, &[]).1, 0.0);
    // The player not in a cell: no cast.
    e.mem.set_u32(PC + 0x40, 0);
    let before = calls_to(PICK_FN).len();
    assert_eq!(call(&mut e, 0x27f1, &[]).1, 0.0);
    assert_eq!(calls_to(PICK_FN).len(), before);
}

#[test]
fn reticle_range() {
    let mut e = setup();
    scene(&mut e);
    for (i, v) in [10.0f32, 20.0, 30.0].iter().enumerate() {
        e.mem.set_f32(PC + 0x30 + 4 * i as u32, *v);
    }
    let (f3, t3) = cast_ends([100.0, 200.0, 300.0], [0.0, 0.0, 1.0], 50000.0);
    let p = cast_point(f3, t3, 0.5);
    // From the player's position.
    let d = crate::commands::ln_fn_math::point3_distance([10.0, 20.0, 30.0], p);
    assert_eq!(call(&mut e, 0x27f2, &[]).1, f64::from(d));
    // From the camera with fromCam (argument 2); the range is argument 1, the layer 0.
    let (f3, t3) = cast_ends([100.0, 200.0, 300.0], [0.0, 0.0, 1.0], 400.0);
    let p = cast_point(f3, t3, 0.5);
    let d = crate::commands::ln_fn_math::point3_distance([100.0, 200.0, 300.0], p);
    assert_eq!(
        call(&mut e, 0x27f2, &[n(5), z(400.0), n(1)]).1,
        f64::from(d)
    );
    assert_eq!(picked(&e).2, 0x1234_0005);
    // Not found: -1, unless two arguments or more.
    not_found(&mut e);
    assert_eq!(call(&mut e, 0x27f2, &[]).1, -1.0);
    assert_eq!(call(&mut e, 0x27f2, &[n(6)]).1, -1.0);
    assert_ne!(call(&mut e, 0x27f2, &[n(6), z(400.0)]).1, -1.0);
    // Not in a cell: -1.
    e.mem.set_u32(PC + 0x40, 0);
    assert_eq!(call(&mut e, 0x27f2, &[n(6), z(400.0)]).1, -1.0);
}

#[test]
fn reticle_pos_alt() {
    let mut e = setup();
    scene(&mut e);
    c2_locals(&mut e, 3);
    let (f3, t3) = cast_ends([100.0, 200.0, 300.0], [0.0, 0.0, 1.0], 50000.0);
    let p = cast_point(f3, t3, 0.5);
    assert_eq!(call(&mut e, 0x291f, &[var(1), var(2), var(3)]).1, 1.0);
    assert_eq!([1, 2, 3].map(|i| local_value(&e, i)), p.map(f64::from));
    assert_eq!(picked(&e).2, 0x1234_0006, "layer 6");
    // The variables in their order.
    call(&mut e, 0x291f, &[var(3), var(1), var(2)]);
    assert_eq!([3, 1, 2].map(|i| local_value(&e, i)), p.map(f64::from));
    // The layer (argument 3) and range (argument 4).
    call(&mut e, 0x291f, &[var(1), var(2), var(3), n(9), z(5.0)]);
    let (_, to, filter) = picked(&e);
    assert_eq!(filter, 0x1234_0009);
    let (_, t3) = cast_ends([100.0, 200.0, 300.0], [0.0, 0.0, 1.0], 5.0);
    assert_eq!(first3(to), t3);
    // Not found: nothing unless all five arguments.
    not_found(&mut e);
    c2_locals(&mut e, 3);
    assert_eq!(call(&mut e, 0x291f, &[var(1), var(2), var(3), n(6)]).1, 0.0);
    assert_eq!(local_value(&e, 1), -1.0);
    assert_eq!(
        call(&mut e, 0x291f, &[var(1), var(2), var(3), n(6), z(5.0)]).1,
        1.0
    );
    // Not in a cell: nothing.
    e.mem.set_u32(PC + 0x40, 0);
    c2_locals(&mut e, 3);
    assert_eq!(
        call(&mut e, 0x291f, &[var(1), var(2), var(3), n(6), z(5.0)]).1,
        0.0
    );
    assert_eq!(local_value(&e, 1), -1.0);
}

#[test]
fn reticle_node() {
    let mut e = setup();
    scene(&mut e);
    let node = |e: &mut Engine, args: &[Vec<u8>]| {
        let res = call(e, 0x28f5, args).1;
        string_result(e, res)
    };
    // No hit: "".
    assert_eq!(node(&mut e, &[]), b"");
    // From the player's camera position (1, 2, 3) along the camera's column 0, layer 6.
    let (from, to, filter) = picked(&e);
    let (f3, t3) = cast_ends([1.0, 2.0, 3.0], [0.0, 0.0, 1.0], 50000.0);
    assert_eq!((first3(from), first3(to)), (f3, t3));
    assert_eq!(filter, 0x1234_0006);
    // A hit: its name; the range (argument 0) and layer (argument 1).
    let name = text(&mut e, 3, b"Bip01 Head");
    e.mem.set_u32(HIT + 8, name);
    e.mem.set_u32(PICK_ANSWER, HIT);
    assert_eq!(node(&mut e, &[z(20.0), n(4)]), b"Bip01 Head");
    let (_, to, filter) = picked(&e);
    assert_eq!(filter, 0x1234_0004);
    assert_eq!(
        first3(to),
        cast_ends([1.0, 2.0, 3.0], [0.0, 0.0, 1.0], 20.0).1
    );
    // A block without a name: "NULL".
    e.mem.set_u32(HIT + 8, 0);
    assert_eq!(node(&mut e, &[]), b"NULL");
    // Without the camera: no cast, "".
    e.mem.set_u32(SG + 0xac, 0);
    assert_eq!(node(&mut e, &[]), b"");
}

#[test]
fn reticle_target_limb() {
    let mut e = setup();
    scene(&mut e);
    e.register(LIMB_FN, |_, w| {
        rec(LIMB_FN, w);
        eax(7)
    });
    // No hit: -1, layer 54.
    assert_eq!(call(&mut e, 0x2942, &[]).1, -1.0);
    assert_eq!(picked(&e).2, 0x1234_0036);
    // A hit under an actor's fade node: the game's limb for the block; the range argument 0.
    e.mem.set_u32(PICK_ANSWER, HIT);
    e.mem.set_u32(HIT + 0x18, FADE);
    e.mem.set_u32(FADE, 0x010a_8f90);
    e.mem.set_u32(FADE + 0xcc, ACTOR);
    e.mem.set_u32(VT_REFR + 0x100, 0x008d_0360);
    assert_eq!(call(&mut e, 0x2942, &[z(30.0)]).1, 7.0);
    assert_eq!(calls_to(LIMB_FN), vec![vec![ACTOR, HIT]]);
    assert_eq!(
        first3(picked(&e).1),
        cast_ends([1.0, 2.0, 3.0], [0.0, 0.0, 1.0], 30.0).1
    );
    // Not an actor: -1, no call.
    e.mem.set_u32(NOT_ACTOR, NOT_ACTOR_VT);
    e.mem.set_u32(FADE + 0xcc, NOT_ACTOR);
    assert_eq!(call(&mut e, 0x2942, &[]).1, -1.0);
    assert_eq!(calls_to(LIMB_FN).len(), 1);
    // No reference above the block: -1.
    e.mem.set_u32(HIT + 0x18, 0);
    assert_eq!(call(&mut e, 0x2942, &[]).1, -1.0);
    assert_eq!(calls_to(LIMB_FN).len(), 1);
}

#[test]
fn point_ray_cast_pos() {
    let mut e = setup();
    scene(&mut e);
    c2_locals(&mut e, 3);
    // Angles 0: about +Y (JIP's sine and cosine of 0 are its approximations, so column 1 of
    // FromEulerPRYInv is within 1e-6 of it); from the position given, layer 6, range 50000.
    let args = [
        z(5.0),
        z(6.0),
        z(7.0),
        z(0.0),
        z(0.0),
        var(1),
        var(2),
        var(3),
    ];
    assert_eq!(call(&mut e, 0x2906, &args).1, 1.0);
    let (from, to, filter) = picked(&e);
    let m0 = crate::commands::ni_matrix::from_euler_pry_inv([0.0; 3]);
    assert!((m0[4] - 1.0).abs() < 1e-6 && m0[1].abs() < 1e-6 && m0[7].abs() < 1e-6);
    let (f3, t3) = cast_ends([5.0, 6.0, 7.0], [m0[1], m0[4], m0[7]], 50000.0);
    assert_eq!((first3(from), first3(to)), (f3, t3));
    assert_eq!(filter, 0x1234_0006);
    let p = cast_point(f3, t3, 0.5);
    assert_eq!([1, 2, 3].map(|i| local_value(&e, i)), p.map(f64::from));
    // The angles (degrees; pitch argument 3, yaw argument 4, roll 0): column 1 of
    // FromEulerPRYInv; layer and range are arguments 8 and 9.
    let k = crate::commands::FLT_PI_D_180;
    let m = crate::commands::ni_matrix::from_euler_pry_inv([30.0 * k, 0.0, 60.0 * k]);
    let args = [
        z(5.0),
        z(6.0),
        z(7.0),
        z(30.0),
        z(60.0),
        var(1),
        var(2),
        var(3),
        n(2),
        z(10.0),
    ];
    call(&mut e, 0x2906, &args);
    let (_, to, filter) = picked(&e);
    let (_, t3) = cast_ends([5.0, 6.0, 7.0], [m[1], m[4], m[7]], 10.0);
    assert_eq!(first3(to), t3);
    assert_eq!(filter, 0x1234_0002);
    // Not found: nothing unless all ten arguments.
    not_found(&mut e);
    c2_locals(&mut e, 3);
    assert_eq!(call(&mut e, 0x2906, &args[..9]).1, 0.0);
    assert_eq!(local_value(&e, 2), -1.0);
    assert_eq!(call(&mut e, 0x2906, &args).1, 1.0);
}

#[test]
fn from_2_points_lengths() {
    // Apart: the squared length times its reciprocal square root (one Newton step).
    let l = from_2_points_length([0.0, 0.0, 0.0], [3.0, 4.0, 12.0]);
    assert!((l - 13.0).abs() < 1e-4, "{l}");
    // The same point, or a NaN: 0.
    assert_eq!(from_2_points_length([1.0, 2.0, 3.0], [1.0, 2.0, 3.0]), 0.0);
    assert_eq!(from_2_points_length([f32::NAN, 0.0, 0.0], [0.0; 3]), 0.0);
    // Only z apart: |z| exactly.
    assert_eq!(from_2_points_length([1.0, 2.0, 3.0], [1.0, 2.0, 10.5]), 7.5);
    assert_eq!(from_2_points_length([1.0, 2.0, 10.5], [1.0, 2.0, 3.0]), 7.5);
}

#[test]
fn p2p_ray_cast_range() {
    let mut e = setup();
    scene(&mut e);
    // From (0, 0, 0) toward (0, 100, 0): From2Points' column 1 is +Y (signs of zero aside); the
    // cast's length the distance; the result the hit's distance to the second point.
    let args = [z(0.0), z(0.0), z(0.0), z(0.0), z(100.0), z(0.0)];
    // The hit a quarter of the way: 75 from the second point (25 from the first).
    e.mem.set_f32(PICK_ANSWER + 4, 0.25);
    let res = call(&mut e, 0x2923, &args).1;
    let (from, to, filter) = picked(&e);
    let len = from_2_points_length([0.0; 3], [0.0, 100.0, 0.0]);
    let (f3, t3) = cast_ends([0.0; 3], [0.0, 1.0, 0.0], len);
    assert_eq!(first3(from), f3);
    assert_eq!([to[0].abs(), to[1], to[2].abs()], t3);
    assert_eq!(filter, 0x1234_0006);
    let p = cast_point(f3, t3, 0.25);
    let d = crate::commands::ln_fn_math::point3_distance(p, [0.0, 100.0, 0.0]);
    assert_eq!(res, f64::from(d));
    assert!((res - 75.0).abs() < 0.01, "{res}");
    // The second point is arguments 3 to 5: toward (0, 0, 40) the cast goes up 40.
    call(
        &mut e,
        0x2923,
        &[z(0.0), z(0.0), z(0.0), z(0.0), z(0.0), z(40.0)],
    );
    let (_, to, _) = picked(&e);
    assert_eq!(to[2], cast_ends([0.0; 3], [0.0, 0.0, 1.0], 40.0).1[2]);
    // The layer is argument 6.
    call(
        &mut e,
        0x2923,
        &[z(0.0), z(0.0), z(0.0), z(0.0), z(100.0), z(0.0), n(11)],
    );
    assert_eq!(picked(&e).2, 0x1234_000b);
    // Not found: no result.
    not_found(&mut e);
    assert_eq!(call(&mut e, 0x2923, &args).1, 0.0);
}

/// JIP's `ATan2` is an approximation: within 1e-3 degrees of the true angle.
fn near(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-3
}

#[test]
fn angles_between_points_cmd() {
    let mut e = setup();
    c2_locals(&mut e, 2);
    let run = |e: &mut Engine, p: [f64; 6], vars: [u16; 2]| {
        let mut args: Vec<Vec<u8>> = p.iter().map(|&x| z(x)).collect();
        args.extend([var(vars[0]), var(vars[1])]);
        assert_eq!(call(e, 0x2922, &args).1, 0.0, "no result");
    };
    // d = (10, 0, 0): heading ATan2(10, -0) = 90, pitch -ATan2(0, 10) = -0.
    run(&mut e, [10.0, 0.0, 0.0, 0.0, 0.0, 0.0], [1, 2]);
    assert!(near(local_value(&e, 2), 90.0), "{}", local_value(&e, 2));
    assert_eq!(local_value(&e, 1), 0.0);
    // d = (0, 10, 10): heading ATan2(0, -10) = 180, pitch -ATan2(10, 10) = -45.
    run(&mut e, [0.0, 10.0, 10.0, 0.0, 0.0, 0.0], [1, 2]);
    assert!(near(local_value(&e, 2), 180.0), "{}", local_value(&e, 2));
    assert!(near(local_value(&e, 1), -45.0), "{}", local_value(&e, 1));
    // The second point is subtracted: d = (0, -10, -10) gives heading 0, pitch 45; the
    // variables swapped (x into 2, z into 1).
    run(&mut e, [0.0, 0.0, 0.0, 0.0, 10.0, 10.0], [2, 1]);
    assert!(near(local_value(&e, 1), 0.0), "{}", local_value(&e, 1));
    assert!(near(local_value(&e, 2), 45.0), "{}", local_value(&e, 2));
    // The values are floats times Flt180dPI in floats, widened.
    let (a1, a2) = angles_between_points([3.0, 4.0, 5.0], [0.5, 0.25, 0.0]);
    run(&mut e, [3.0, 4.0, 5.0, 0.5, 0.25, 0.0], [1, 2]);
    let k = crate::commands::nif_nodes::FLT_180_D_PI;
    assert_eq!(local_value(&e, 2), f64::from(a1 * k));
    assert_eq!(local_value(&e, 1), f64::from(flip(a2) * k));
}

#[test]
fn transform_world_to_local_angles() {
    let mut e = setup();
    c2_locals(&mut e, 2);
    let run = |e: &mut Engine, v: [f64; 9]| {
        let mut args: Vec<Vec<u8>> = v.iter().map(|&x| z(x)).collect();
        args.extend([var(1), var(2)]);
        assert_eq!(call(e, 0x2921, &args).1, 0.0, "no result");
        (local_value(e, 1), local_value(e, 2))
    };
    // No rotation: r = target - origin; (0, 10, 0) is heading 0, (10, 0, 0) heading 90.
    let (x, zz) = run(&mut e, [1.0, 1.0, 1.0, 1.0, 11.0, 1.0, 0.0, 0.0, 0.0]);
    assert!(near(zz, 0.0) && near(x, 0.0), "{x} {zz}");
    let (x, zz) = run(&mut e, [1.0, 1.0, 1.0, 11.0, 1.0, 1.0, 0.0, 0.0, 0.0]);
    assert!(near(zz, 90.0) && near(x, 0.0), "{x} {zz}");
    // Above: pitch -45 for (0, 10, 10); the origin is subtracted.
    let (x, zz) = run(&mut e, [0.0, -10.0, -10.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]);
    assert!(near(zz, 0.0) && near(x, -45.0), "{x} {zz}");
    // A rotation: the offset turned by FromEulerPRYInv of the angles (each row times it,
    // `(a0 + a1) + (a2 + a3)`).
    let k = crate::commands::FLT_PI_D_180;
    let m = crate::commands::ni_matrix::from_euler_pry_inv([10.0 * k, 20.0 * k, 30.0 * k]);
    let v = [3.0f32, -4.0, 5.0];
    let r = |i: usize, w: f32| (m[i] * v[0] + m[i + 1] * v[1]) + (m[i + 2] * v[2] + w * 0.0);
    let (r0, r1, r2) = (r(0, m[3]), r(3, m[6]), r(6, 0.0));
    let a1 = crate::commands::ln_fn_math::atan2(r0, r1);
    let a2 = crate::commands::ln_fn_math::atan2(r2, (r0 * r0 + r1 * r1).sqrt());
    let f = crate::commands::nif_nodes::FLT_180_D_PI;
    let (x, zz) = run(&mut e, [0.0, 0.0, 0.0, 3.0, -4.0, 5.0, 10.0, 20.0, 30.0]);
    assert_eq!(zz, f64::from(a1 * f));
    assert_eq!(x, f64::from(flip(a2) * f));
}

#[test]
fn camera_movement() {
    let mut e = setup();
    c2_locals(&mut e, 2);
    let globals = C2 + 0x4000;
    e.map(0x011f_3000, 0x1000);
    e.mem.set_u32(0x011f_35cc, globals);
    e.mem.set_f32(0x011e_0a6c + 4, 0.5);
    let run = |e: &mut Engine| {
        let r = call(e, 0x28f0, &[var(1), var(2)]).1;
        (r, local_value(e, 1), local_value(e, 2))
    };
    // The mouse: its movement times the sensitivity.
    e.mem.set_u32(globals + 0x1b24, 5);
    e.mem.set_u32(globals + 0x1b28, (-3i32) as u32);
    assert_eq!(run(&mut e), (1.0, 2.5, -1.5));
    // The variables in their order.
    call(&mut e, 0x28f0, &[var(2), var(1)]);
    assert_eq!((local_value(&e, 2), local_value(&e, 1)), (2.5, -1.5));
    // Y inverted.
    e.mem.set_u32(0x011e_0a5c + 4, 1);
    assert_eq!(run(&mut e), (1.0, 2.5, 1.5));
    // Only x moved: y stays 0.
    e.mem.set_u32(globals + 0x1b28, 0);
    assert_eq!(run(&mut e), (1.0, 2.5, 0.0));
    // Nothing moved: both 0, no result.
    e.mem.set_u32(globals + 0x1b24, 0);
    assert_eq!(run(&mut e), (0.0, 0.0, 0.0));
    // A controller: the right stick, inside the dead zone 0, times sensitivity / 1500, y
    // negated (after the inversion).
    e.mem.set_u32(0x011e_0a5c + 4, 0);
    {
        let mut s = crate::input::state(&mut e);
        s.ready = true;
        s.dead_zone_rs = 8000;
    }
    e.mem.set_u16(0x011f_35b4, 9000);
    e.mem.set_u16(0x011f_35b6, (-12000i16) as u16);
    let (r, x, y) = run(&mut e);
    assert_eq!(r, 1.0);
    assert_eq!(x, 9000.0 * 0.5 * (1.0 / 1500.0));
    assert_eq!(y, -12000.0 * 0.5 * (-1.0 / 1500.0));
    e.mem.set_u32(0x011e_0a5c + 4, 1);
    let (_, _, y) = run(&mut e);
    assert_eq!(y, 12000.0 * 0.5 * (-1.0 / 1500.0));
    // Inside the dead zone (its edges too): 0; the mouse is not read.
    e.mem.set_u16(0x011f_35b4, 8000);
    e.mem.set_u16(0x011f_35b6, (-8000i16) as u16);
    e.mem.set_u32(globals + 0x1b24, 50);
    assert_eq!(run(&mut e), (0.0, 0.0, 0.0));
    e.mem.set_u16(0x011f_35b4, 8001);
    let (_, x, _) = run(&mut e);
    assert_eq!(x, 8001.0 * 0.5 * (1.0 / 1500.0));
}

#[test]
fn game_hour() {
    let mut e = setup();
    let hour = C2 + 0x5000;
    e.mem.set_u32(0x011d_e7b8 + 0xc, hour);
    e.mem.set_f32(hour + 0x24, 10.0);
    // Later today: set; earlier: a day later. No result.
    assert_eq!(call(&mut e, 0x288b, &[z(12.5)]).1, 0.0);
    assert_eq!(e.mem.f32(hour + 0x24), 12.5);
    call(&mut e, 0x288b, &[z(12.5)]);
    assert_eq!(e.mem.f32(hour + 0x24), 12.5, "equal: set as it is");
    call(&mut e, 0x288b, &[z(5.0)]);
    assert_eq!(e.mem.f32(hour + 0x24), 29.0);
    // Negative: nothing.
    e.mem.set_f32(hour + 0x24, 3.0);
    call(&mut e, 0x288b, &[z(-1.0)]);
    assert_eq!(e.mem.f32(hour + 0x24), 3.0);
    // The hardcore flag: in hardcore with tracking on.
    use crate::game_patches::gp::force_hardcore_needs_update as forced;
    assert!(!forced(&mut e));
    e.mem.set_u32(PC + 0x7bc, 1);
    call(&mut e, 0x288b, &[z(4.0)]);
    assert!(forced(&mut e));
    // Tracking off (JIP's kept flag 1): cleared.
    e.ext::<crate::Jip>().saved.serialized.flags = 1;
    publish_serialized_flags(&mut e);
    call(&mut e, 0x288b, &[z(4.0)]);
    assert!(!forced(&mut e));
    e.ext::<crate::Jip>().saved.serialized.flags = 0;
    publish_serialized_flags(&mut e);
    call(&mut e, 0x288b, &[z(4.0)]);
    assert!(forced(&mut e));
    // Not hardcore: cleared.
    e.mem.set_u32(PC + 0x7bc, 0);
    call(&mut e, 0x288b, &[z(4.0)]);
    assert!(!forced(&mut e));
    // A negative hour leaves the flag.
    e.mem.set_u32(PC + 0x7bc, 1);
    call(&mut e, 0x288b, &[z(4.0)]);
    call(&mut e, 0x288b, &[z(-4.0)]);
    assert!(forced(&mut e));
}

#[test]
fn mark_activator_ash_pile_cmd() {
    let mut e = setup();
    let (acti, acti2) = (C2 + 0x6000, C2 + 0x6100);
    plain(&mut e, acti, 0x15, 0x301);
    plain(&mut e, acti2, 0x15, 0x302);
    set_ref(&mut e, 40, acti);
    let flags6 = |e: &Engine, f: u32| crate::world_hooks::flags6(e, f);
    // An activator: the ash pile bit (0x20) and form flag bit 0. No result.
    e.mem.set_u32(acti + 8, 0x40);
    assert_eq!(call(&mut e, 0x2841, &[r(40)]).1, 0.0);
    assert_eq!((flags6(&e, acti), e.mem.u32(acti + 8)), (0x20, 0x41));
    // A list: its activators only.
    let list = form_list(&mut e, C2 + 0x6200, &[acti2, MISC, 0]);
    set_ref(&mut e, 41, list);
    let misc = e.mem.bytes(MISC, 0x20);
    call(&mut e, 0x2841, &[r(41)]);
    assert_eq!((flags6(&e, acti2), e.mem.u32(acti2 + 8) & 1), (0x20, 1));
    assert_eq!(e.mem.bytes(MISC, 0x20), misc);
    // Another form: nothing.
    call(&mut e, 0x2841, &[r(R_MISC)]);
    assert_eq!(e.mem.bytes(MISC, 0x20), misc);
    // A list inside the list is not walked.
    let (acti3, inner, outer) = (C2 + 0x6400, C2 + 0x6500, C2 + 0x6600);
    plain(&mut e, acti3, 0x15, 0x303);
    form_list(&mut e, inner, &[acti3]);
    form_list(&mut e, outer, &[inner]);
    set_ref(&mut e, 42, outer);
    call(&mut e, 0x2841, &[r(42)]);
    assert_eq!(flags6(&e, acti3), 0);
}

/// The process manager's actor slots.
const PM_DATA: u32 = C2 + 0x7000;
const KEEPS_FN: u32 = 0x0057_7de0;
const FADE_FN: u32 = 0x008f_eb60;

/// A dead actor in slot `i` that passes every test, its process at +0x800.
fn dead(e: &mut Engine, i: u32) -> (u32, u32) {
    let actor = C2 + 0x8000 + 0x1000 * i;
    let process = actor + 0x800;
    form(e, actor, 0x3b, 0x400 + i);
    e.mem.set_u32(actor, VT_REFR);
    e.mem.set_u32(actor + 8, 0);
    e.mem.set_u32(actor + 0x20, MISC);
    e.mem.set_u32(actor + 0x40, 0);
    e.mem.set_u32(actor + 0x108, 2);
    e.mem.set_u32(actor + 0x68, process);
    e.mem.set_f32(process + 0x330, -1.0);
    e.mem.set_u32(PM_DATA + 4 * i, actor);
    (actor, process)
}

#[test]
fn clear_dead_actors_cmd() {
    let mut e = setup();
    e.mem.set_u32(VT_REFR + 0x100, 0x008d_0360);
    e.register(KEEPS_FN, |e, w| {
        rec(KEEPS_FN, w);
        // The answer is the byte in al: the upper bytes are set for every actor.
        eax(0xabcd_ef00 | u32::from(e.mem.u32(w[0] + 0x40) == 0x77))
    });
    e.register(FADE_FN, |_, w| {
        rec(FADE_FN, w);
        eax(0)
    });
    let pm = 0x011e_0e80;
    e.mem.set_u32(pm + 8, PM_DATA);
    e.mem.set_u16(pm + 0xc, 16);
    e.mem.set_u32(pm + 0x14, 1);
    e.mem.set_u32(pm + 0x24, 13);
    let actors: Vec<(u32, u32)> = (0..14).map(|i| dead(&mut e, i)).collect();
    // Each rejected for one reason: 2 alive, 3 flagged 0x200000, 4 a quest item base, 5 no
    // process, 6 a process above level 0, 7 fading, 8 its timer not below 0, 9 a NaN timer,
    // 10 ExtraEnableStateChildren, 11 kept by 00577DE0, 12 not an actor; 0 and 13 outside.
    e.mem.set_u32(actors[2].0 + 0x108, 1);
    e.mem.set_u32(actors[3].0 + 8, 0x0020_0000);
    let quest_base = C2 + 0x7800;
    plain(&mut e, quest_base, 0x1f, 0x3ff);
    e.mem.set_u32(quest_base + 8, 0x400);
    e.mem.set_u32(actors[4].0 + 0x20, quest_base);
    e.mem.set_u32(actors[5].0 + 0x68, 0);
    e.mem.set_u32(actors[6].1 + 0x28, 1);
    e.mem.set_u32(actors[7].1 + 0x3e8, 2);
    e.mem.set_f32(actors[8].1 + 0x330, 0.0);
    e.mem.set_f32(actors[9].1 + 0x330, f32::NAN);
    e.mem.set_u8(actors[10].0 + 0x44 + 8 + 7, 1);
    e.mem.set_u32(actors[11].0 + 0x40, 0x77);
    e.mem.set_u32(actors[12].0, NOT_ACTOR_VT);
    assert_eq!(call(&mut e, 0x28ef, &[]).1, 0.0, "no result");
    // Faded: 1 only.
    assert_eq!(calls_to(FADE_FN), vec![vec![actors[1].1, actors[1].0]]);
    // 00577DE0 asked only of those that passed the rest (11 and 1), last first.
    assert_eq!(
        calls_to(KEEPS_FN),
        vec![vec![actors[11].0], vec![actors[1].0]]
    );
    // Life states other than 2 (dead) are left: 3 (unconscious) and 5 (restrained) too.
    for (i, life) in [(14, 3), (15, 5)] {
        let (actor, _) = dead(&mut e, i);
        e.mem.set_u32(actor + 0x108, life);
    }
    e.mem.set_u32(actors[13].0 + 0x108, 1);
    e.mem.set_u32(pm + 0x24, 16);
    CALLS.with(|c| c.borrow_mut().clear());
    call(&mut e, 0x28ef, &[]);
    assert_eq!(calls_to(FADE_FN), vec![vec![actors[1].1, actors[1].0]]);
    e.mem.set_u32(pm + 0x24, 13);
    // Two that pass, faded last to first (12 then 2); the end capped at the capacity (13).
    dead(&mut e, 2);
    dead(&mut e, 12);
    dead(&mut e, 13);
    e.mem.set_u32(actors[1].0 + 0x108, 1);
    e.mem.set_u32(pm + 0x24, 100);
    e.mem.set_u16(pm + 0xc, 13);
    CALLS.with(|c| c.borrow_mut().clear());
    call(&mut e, 0x28ef, &[]);
    assert_eq!(
        calls_to(FADE_FN),
        vec![
            vec![actors[12].1, actors[12].0],
            vec![actors[2].1, actors[2].0]
        ]
    );
    // The begin: slot 0 is not walked even when it passes; an end not past the begin: nothing.
    dead(&mut e, 0);
    e.mem.set_u32(pm + 0x24, 1);
    CALLS.with(|c| c.borrow_mut().clear());
    call(&mut e, 0x28ef, &[]);
    assert!(calls_to(FADE_FN).is_empty());
    e.mem.set_u32(pm + 0x24, 3);
    call(&mut e, 0x28ef, &[]);
    assert_eq!(calls_to(FADE_FN), vec![vec![actors[2].1, actors[2].0]]);
}

/// A plain chain of 50,000 nested lists (each holding the next, the last MISC): no recursion on
/// the Rust stack (a test thread's 2 MB would run out), no search along the path per level.
#[test]
fn a_chain_of_50000_nested_lists() {
    let mut e = setup();
    const DEEP: u32 = 50_000;
    let lists = 0x0a00_0000;
    e.map(lists, 0x20 * DEEP + 0x1000);
    for i in 0..DEEP {
        let item = if i + 1 < DEEP {
            lists + 0x20 * (i + 1)
        } else {
            MISC
        };
        form_list(&mut e, lists + 0x20 * i, &[item]);
    }
    let mut set = BTreeSet::new();
    assign_list(&mut e, &mut set, lists);
    assert_eq!(set, BTreeSet::from([MISC]));
}

/// `NUM_ARGS_EX`: a line with no argument data (length 0, as a statement compiled without
/// arguments is) has no count; the byte after it (5 here: another line's) is not read as one.
#[test]
fn reticle_lines_without_argument_data() {
    let mut e = setup();
    scene(&mut e);
    not_found(&mut e);
    let run = |e: &mut Engine, opcode: u32| {
        let words = crate::test_support::line(e, opcode, &[]);
        e.mem.set_u16(crate::test_support::DATA + 2, 0);
        e.mem.set_u8(crate::test_support::DATA + 4, 5);
        let execute = crate::test_support::execute_of(e, opcode);
        e.call(execute, &words);
        e.mem.f64(RESULT)
    };
    // Not found and no arguments: no array, -1.
    assert_eq!(run(&mut e, 0x27f1), 0.0);
    assert_eq!(run(&mut e, 0x27f2), -1.0);
    // The defaults were cast: layer 6 (from -1), range 50000.
    let (_, to, filter) = picked(&e);
    assert_eq!(filter, 0x1234_0006);
    let (_, t3) = cast_ends([100.0, 200.0, 300.0], [0.0, 0.0, 1.0], 50000.0);
    assert_eq!(first3(to), t3);
}

/// A global argument with no global behind it: the read fails.
fn bad_arg() -> Vec<u8> {
    vec![b'G', 9, 0]
}

/// Arguments that do not read: no cast, no variable written, the result as the command starts
/// it (0, or -1 for GetReticleRange and GetReticleTargetLimb).
#[test]
fn unreadable_arguments_are_refused() {
    let mut e = setup();
    scene(&mut e);
    worlds(&mut e);
    c2_locals(&mut e, 3);
    let globals = C2 + 0x4000;
    e.map(0x011f_3000, 0x1000);
    e.mem.set_u32(0x011f_35cc, globals);
    e.mem.set_u32(globals + 0x1b24, 5);
    e.mem.set_f32(0x011e_0a6c + 4, 1.0);
    let hour = C2 + 0x5000;
    e.mem.set_u32(0x011d_e7b8 + 0xc, hour);
    e.mem.set_f32(hour + 0x24, 10.0);
    let gravity = [0, 4, 8].map(|o| e.mem.u32(EXT_HKP + 0x10 + o));
    let b = bad_arg;
    let cases: Vec<(u32, Vec<Vec<u8>>, f64)> = vec![
        (0x27f1, vec![b(), z(10.0)], 0.0),
        (0x27f2, vec![n(6), b()], -1.0),
        (0x291f, vec![var(1), var(2), var(3), n(6), b()], 0.0),
        (0x28f5, vec![b()], 0.0),
        (0x2942, vec![b()], -1.0),
        (
            0x2906,
            vec![z(1.0), z(2.0), b(), z(0.0), z(0.0), var(1), var(2), var(3)],
            0.0,
        ),
        (
            0x2923,
            vec![z(0.0), z(0.0), z(0.0), z(0.0), b(), z(0.0)],
            0.0,
        ),
        (
            0x2921,
            vec![
                z(0.0),
                z(0.0),
                z(0.0),
                z(1.0),
                z(0.0),
                z(0.0),
                z(0.0),
                z(0.0),
                b(),
                var(1),
                var(2),
            ],
            0.0,
        ),
        (
            0x2922,
            vec![z(1.0), z(0.0), z(0.0), b(), z(0.0), z(0.0), var(1), var(2)],
            0.0,
        ),
        (0x28f0, vec![var(1), var(9)], 0.0),
        (0x27b2, vec![], 0.0),
        (0x27b3, vec![z(1.0), b(), z(1.0)], 0.0),
        (0x288b, vec![b()], 0.0),
    ];
    for (opcode, args, start) in cases {
        CALLS.with(|c| c.borrow_mut().clear());
        let r = call(&mut e, opcode, &args).1;
        let r = if opcode == 0x28f5 { 0.0 } else { r };
        assert_eq!(r, start, "{opcode:04x}");
        assert!(calls_to(PICK_FN).is_empty(), "{opcode:04x} cast");
        assert_eq!(
            [1, 2, 3].map(|i| local_value(&e, i)),
            [-1.0; 3],
            "{opcode:04x} wrote a variable"
        );
    }
    assert_eq!(gravity, [0, 4, 8].map(|o| e.mem.u32(EXT_HKP + 0x10 + o)));
    assert_eq!(e.mem.f32(hour + 0x24), 10.0);
}

const SNEAK_MOVER: u32 = C2 + 0x2_0000;
const MOVER_VT: u32 = C2 + 0x2_0100;
const CACHED: u32 = C2 + 0x2_0200;
const FORCE_FN: u32 = 0x00de_c220;
const PARALYZED_FN: u32 = 0x00de_c230;
const SNEAK_CHANGED_FN: u32 = 0x0093_ab30;
const IRON_SIGHTS_FN: u32 = 0x008b_b650;
const BLOCK_FN: u32 = 0x0089_4cc0;

/// The player standing, able to sneak: mover flags 0x81 (+0x400 when sneaking), action -1, the
/// exe's action table at `0x94423C` (its bytes, then the jump table's first bytes), cached
/// flags 0xFFFF; `GetIsParalyzed` answers the byte at `C2 + 0x2_0300` with upper bytes set.
fn sneak_setup() -> Engine {
    let mut e = setup();
    e.map(0x0094_4000, 0x1000);
    let table = [
        0u8, 1, 1, 0, 0, 0, 0, 0, 0, 1, 0, 1, 1, 1, 1, 1, 1, 1, 0, 0x90, 0x05, 0x31, 0x94,
    ];
    e.mem.write(0x0094_423c, &table);
    e.mem.set_u32(PC + 0x190, SNEAK_MOVER);
    e.mem.set_u32(SNEAK_MOVER, MOVER_VT);
    e.mem.set_u32(MOVER_VT + 0xc, FORCE_FN);
    e.mem.set_u32(SNEAK_MOVER + 0x94, 0x81);
    e.mem.set_u16(PROCESS + 0x2ec, 0xffff);
    e.mem.set_u32(PROCESS + 0x2c, CACHED);
    e.mem.set_u32(CACHED + 0x44, 0xffff);
    e.mem.set_u32(VT_REFR + 0x234, PARALYZED_FN);
    e.register(PARALYZED_FN, |e, w| {
        rec(PARALYZED_FN, w);
        eax(0xabcd_ef00 | u32::from(e.mem.u8(C2 + 0x2_0300)))
    });
    e.register(FORCE_FN, |e, w| {
        rec(FORCE_FN, w);
        e.mem.set_u32(SNEAK_MOVER + 0x94, w[1]);
        eax(0)
    });
    e.register(SNEAK_CHANGED_FN, |_, w| {
        rec(SNEAK_CHANGED_FN, w);
        eax(0)
    });
    e.register(IRON_SIGHTS_FN, |_, w| {
        rec(IRON_SIGHTS_FN, w);
        eax(0)
    });
    e.register(BLOCK_FN, |_, w| {
        rec(BLOCK_FN, w);
        eax(0)
    });
    e
}

fn changes() -> usize {
    calls_to(FORCE_FN).len()
}

#[test]
fn toggle_player_sneaking_sneaks_and_stands() {
    let mut e = sneak_setup();
    // Sneak: the flag set through the mover, 0093AB30(0xA), iron sights and block ended, the
    // cached speeds marked stale. No result.
    assert_eq!(call(&mut e, 0x2907, &[n(1)]).1, 0.0);
    assert_eq!(calls_to(FORCE_FN), vec![vec![SNEAK_MOVER, 0x481]]);
    assert_eq!(calls_to(SNEAK_CHANGED_FN), vec![vec![PC, 0xa]]);
    assert_eq!(calls_to(IRON_SIGHTS_FN), vec![vec![PC, 0, 0, 0]]);
    assert_eq!(calls_to(BLOCK_FN), vec![vec![PC, 0]]);
    assert_eq!(e.mem.u32(CACHED + 0x44), 0xcfff);
    // Already sneaking: nothing; standing: 0x400 cleared, 0093AB30(0xB). With the scope
    // (+0x64F) no iron sights or block calls.
    CALLS.with(|c| c.borrow_mut().clear());
    call(&mut e, 0x2907, &[n(2)]);
    assert_eq!(changes(), 0);
    e.mem.set_u8(PC + 0x64f, 1);
    call(&mut e, 0x2907, &[n(0)]);
    assert_eq!(calls_to(FORCE_FN), vec![vec![SNEAK_MOVER, 0x81]]);
    assert_eq!(calls_to(SNEAK_CHANGED_FN), vec![vec![PC, 0xb]]);
    assert!(calls_to(IRON_SIGHTS_FN).is_empty() && calls_to(BLOCK_FN).is_empty());
    // No cached values: the rest still done.
    e.mem.set_u32(PROCESS + 0x2c, 0);
    CALLS.with(|c| c.borrow_mut().clear());
    call(&mut e, 0x2907, &[n(1)]);
    assert_eq!(changes(), 1);
}

#[test]
fn toggle_player_sneaking_refusals() {
    // Each blocks the change on its own; the neighbouring bytes are set so a wider read differs.
    type Setup = fn(&mut Engine);
    let cases: [(&str, Setup); 10] = [
        ("alive state", |e| e.mem.set_u32(PC + 0x108, 1)),
        ("sitting", |e| e.mem.set_u32(PC + 0x1ac, 3)),
        ("controls 0x40", |e| e.mem.set_u8(PC + 0x680, 0x40)),
        ("knocked", |e| e.mem.set_u8(PROCESS + 0x13c, 0xff)),
        ("action 0", |e| e.mem.set_u16(PROCESS + 0x2ec, 0)),
        ("action 16", |e| e.mem.set_u16(PROCESS + 0x2ec, 16)),
        ("action 18 (past the table)", |e| {
            e.mem.set_u16(PROCESS + 0x2ec, 18)
        }),
        ("paralysed", |e| e.mem.set_u8(C2 + 0x2_0300, 1)),
        ("a lower process", |e| e.mem.set_u32(PROCESS + 0x28, 1)),
        ("no mover", |e| e.mem.set_u32(PC + 0x190, 0)),
    ];
    for (what, f) in cases {
        let mut e = sneak_setup();
        f(&mut e);
        call(&mut e, 0x2907, &[n(1)]);
        assert_eq!(changes(), 0, "{what}");
        assert!(calls_to(SNEAK_CHANGED_FN).is_empty(), "{what}");
        assert_eq!(e.mem.u32(CACHED + 0x44), 0xffff, "{what}");
    }
    // Bits other than 0x40 of the controls byte, actions whose byte is 0 (2, 9, 17): allowed.
    type Allow = fn(&mut Engine);
    let allowed: [Allow; 4] = [
        |e| e.mem.set_u8(PC + 0x680, 0xbf),
        |e| e.mem.set_u16(PROCESS + 0x2ec, 2),
        |e| e.mem.set_u16(PROCESS + 0x2ec, 9),
        |e| e.mem.set_u16(PROCESS + 0x2ec, 17),
    ];
    for (i, f) in allowed.into_iter().enumerate() {
        let mut e = sneak_setup();
        f(&mut e);
        call(&mut e, 0x2907, &[n(1)]);
        assert_eq!(changes(), 1, "case {i}");
    }
    // An argument that does not read: nothing.
    let mut e = sneak_setup();
    call(&mut e, 0x2907, &[vec![b'G', 9, 0]]);
    assert_eq!(changes(), 0);
    assert!(calls_to(PARALYZED_FN).is_empty());
}
