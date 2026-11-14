//! Tests of `jip_fn_miscellaneous`: the player, the HUD, `TES`, the sky, the OS and time globals
//! at their addresses, forms at the offsets of the headers and the exe, and the game functions
//! the commands call as recording stand-ins. Expectations from JIP's source.

use super::*;
use crate::commands::test_support::*;
use nvse::vars::Data;

const PC: u32 = FORMS + 0x1_0000;
const PROCESS: u32 = FORMS + 0x1_1000;
const HUD_AT: u32 = FORMS + 0x1_2000;
const TES_AT: u32 = FORMS + 0x1_3000;
const SKY_AT: u32 = FORMS + 0x1_4000;
const MOON: u32 = FORMS + 0x1_4400;
const OSG: u32 = FORMS + 0x1_5000;
const RADIO: u32 = FORMS + 0x1_5100;
const GLOBALS: u32 = FORMS + 0x1_5200;
const BUF_IN: u32 = FORMS + 0x1_6000;
const BUF_EX: u32 = FORMS + 0x1_6100;
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

fn buffers(e: &mut Engine) {
    e.mem.set_u32(TES_AT + 0x38, BUF_IN);
    e.mem.set_u32(TES_AT + 0x3c, BUF_EX);
    e.mem.set_u32(0x011c_3e38 + 4, 2);
    e.mem.set_u32(0x011c_3c90 + 4, 3);
    // Interiors: CELL_IN, then CELL_X past the 2 slots.
    for (i, c) in [CELL_IN, MISC, CELL_X].iter().enumerate() {
        e.mem.set_u32(BUF_IN + 4 * i as u32, *c);
    }
    // Exteriors: CELL_EX, an empty slot, CELL_X after it.
    for (i, c) in [CELL_EX, 0, CELL_X].iter().enumerate() {
        e.mem.set_u32(BUF_EX + 4 * i as u32, *c);
    }
}

#[test]
fn buffered_cells() {
    let mut e = setup();
    buffers(&mut e);
    assert_eq!(call(&mut e, 0x2737, &[r(R_CELL_EX)]).1, 1.0);
    assert_eq!(call(&mut e, 0x2737, &[r(R_CELL_IN)]).1, 1.0);
    // Past the setting's slot count, after an empty slot, in the other buffer: no.
    assert_eq!(call(&mut e, 0x2737, &[r(R_CELL_X)]).1, 0.0);
    e.mem.set_u32(CELL_X + 0xc0, WRLD);
    assert_eq!(call(&mut e, 0x2737, &[r(R_CELL_X)]).1, 0.0);
    e.mem.set_u32(BUF_EX + 4, CELL_IN);
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
    assert_eq!(
        array_data(&e, id),
        vec![Data::Form(0x107), Data::Form(0x103)]
    );
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
    // Month 13: nothing; month 0 or below: deliberately nothing either.
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
    assert_eq!(both(&mut e, 0x28ad), 1.0);
    e.ext::<Jip>().saved.serialized.flags = 2;
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
    assert!(!e.ext::<Jip>().misc.keywords[&str_hash_ci(b"metal")].contains(&REFR));
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
    let set: Vec<u32> = e.ext::<Jip>().misc.keywords[&str_hash_ci(b"loot")]
        .iter()
        .copied()
        .collect();
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
    assert!(e.ext::<Jip>().misc.keywords[&str_hash_ci(b"none")].is_empty());
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
