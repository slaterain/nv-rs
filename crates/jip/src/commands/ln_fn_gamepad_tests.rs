//! Tests of `ln_fn_gamepad` on JIP's pad state (`crate::input::State`: the pad's own state, the
//! mods, the dead zones) and its trigger handlers (`crate::ln_events::State`), every expectation
//! worked out from `functions_ln/ln_fn_gamepad.h` and its asm (`GetLeftStick`/`GetRightStick`).

use super::*;
use crate::commands::test_support::*;
use crate::input::Pad;

const SCRIPT_FORM: u32 = FORMS + 0x100;
const SCRIPT_TWO: u32 = FORMS + 0x200;
const NOT_SCRIPT: u32 = FORMS + 0x300;

fn setup() -> Engine {
    let mut e = engine();
    form(&mut e, SCRIPT_FORM, 0x11, 0x0100_0100);
    form(&mut e, SCRIPT_TWO, 0x11, 0x0100_0200);
    form(&mut e, NOT_SCRIPT, 0x47, 0x0100_0300);
    set_ref(&mut e, 1, SCRIPT_FORM);
    set_ref(&mut e, 2, SCRIPT_TWO);
    set_ref(&mut e, 3, NOT_SCRIPT);
    e
}

fn input(e: &mut Engine) -> &mut crate::input::State {
    &mut e.ext::<Jip>().input
}

fn eval(e: &mut Engine, opcode: u32, arg1: u32) -> f64 {
    let record = nvse::table::record_of(e, opcode);
    let at = e.mem.u32(record + 0x20);
    e.mem.set_f64(RESULT, 0.0);
    e.call(at, &[0, arg1, 0, RESULT]);
    e.mem.f64(RESULT)
}

fn run(e: &mut Engine, opcode: u32, args: &[Vec<u8>]) -> f64 {
    e.mem.set_f64(RESULT, 0.0);
    call(e, opcode, args).1
}

/// `GetLeftStick` step by step as the asm runs: `cwde`, zero answers 0; positive: `sub eax, dz`,
/// `jg` else 0; negative: `inc eax`, `add eax, dz`, `jge` 0; then `cvtsi2sd` and `mulsd` by the
/// stored `1.0 / (32767 - dz)`.
fn asm_stick(v: i16, dz: i32) -> f64 {
    let mut eax = i32::from(v);
    if eax == 0 {
        return 0.0;
    }
    if eax > 0 {
        eax -= dz;
        if eax <= 0 {
            return 0.0;
        }
    } else {
        eax += 1;
        eax += dz;
        if eax >= 0 {
            return 0.0;
        }
    }
    let d: f64 = 1.0 / f64::from(32767 - dz);
    f64::from(eax) * d
}

#[test]
fn the_controller_and_its_echo() {
    let mut e = setup();
    assert_eq!(run(&mut e, 0x2203, &[]), 0.0);
    input(&mut e).ready = true;
    assert_eq!(run(&mut e, 0x2203, &[]), 1.0);
    let (res, lines) = echoed(&mut e, 0x2203, 0, &[]);
    assert_eq!((res, lines), (1.0, vec![b"GetController >> 1".to_vec()]));
    input(&mut e).ready = false;
    let (res, lines) = echoed(&mut e, 0x2203, 0, &[]);
    assert_eq!((res, lines), (0.0, vec![b"GetController >> 0".to_vec()]));
}

/// Buttons are read from the pad's own state (`s_gamePad`), not the game's copy, the argument
/// cut to 16 bits; the execute and the eval agree.
#[test]
fn buttons_pressed() {
    let mut e = setup();
    input(&mut e).pad = Pad {
        buttons: 0x1010,
        ..Pad::default()
    };
    assert_eq!(run(&mut e, 0x2204, &[n(0x1000)]), 1.0);
    assert_eq!(run(&mut e, 0x2204, &[n(0x0001)]), 0.0);
    assert_eq!(run(&mut e, 0x2204, &[n(0x1_0010)]), 1.0, "low 16 bits");
    assert_eq!(run(&mut e, 0x2204, &[n(0x1_0000)]), 0.0, "bit 16 cut off");
    assert_eq!(eval(&mut e, 0x2204, 0x0010), 1.0);
    assert_eq!(eval(&mut e, 0x2204, 0x0002), 0.0);
    assert_eq!(eval(&mut e, 0x2204, 0x1_0000), 0.0);
    assert_eq!(run(&mut e, 0x220f, &[]), f64::from(0x1010));
}

/// Both sticks at the dead zone's edges, both directions, and the ends of the range; execute
/// and eval for each axis.
#[test]
fn sticks_past_their_dead_zones() {
    let mut e = setup();
    let cases: [i16; 12] = [
        0, 1, 7849, 7850, -7849, -7850, -7851, 8689, 8690, -8691, 32767, -32768,
    ];
    for v in cases {
        input(&mut e).pad = Pad {
            thumb_lx: v,
            thumb_ly: v,
            thumb_rx: v,
            thumb_ry: v,
            ..Pad::default()
        };
        let l = asm_stick(v, 7849);
        let r = asm_stick(v, 8689);
        for (op, want) in [(0x2205, l), (0x2206, l), (0x2207, r), (0x2208, r)] {
            assert_eq!(run(&mut e, op, &[]).to_bits(), want.to_bits(), "{op:x} {v}");
            assert_eq!(eval(&mut e, op, 0).to_bits(), want.to_bits(), "{op:x} eval {v}");
        }
    }
    // The cases the asm's edges decide.
    assert_eq!(asm_stick(7849, 7849), 0.0);
    assert_eq!(asm_stick(-7850, 7849), 0.0, "inc then add gives 0: jge");
    assert!(asm_stick(-7851, 7849) < 0.0);
    // The axes are read from their own fields.
    input(&mut e).pad = Pad {
        thumb_lx: 10000,
        thumb_ly: -10000,
        thumb_rx: 20000,
        thumb_ry: -20000,
        ..Pad::default()
    };
    assert_eq!(run(&mut e, 0x2205, &[]), asm_stick(10000, 7849));
    assert_eq!(run(&mut e, 0x2206, &[]), asm_stick(-10000, 7849));
    assert_eq!(run(&mut e, 0x2207, &[]), asm_stick(20000, 8689));
    assert_eq!(run(&mut e, 0x2208, &[]), asm_stick(-20000, 8689));
}

#[test]
fn triggers_and_dead_zones() {
    let mut e = setup();
    for (lt, rt) in [(0u8, 255u8), (1, 128), (255, 0)] {
        input(&mut e).pad = Pad {
            left_trigger: lt,
            right_trigger: rt,
            ..Pad::default()
        };
        let l = f64::from(lt) * (1.0 / 255.0);
        let r = f64::from(rt) * (1.0 / 255.0);
        assert_eq!(run(&mut e, 0x2209, &[]).to_bits(), l.to_bits());
        assert_eq!(eval(&mut e, 0x2209, 0).to_bits(), l.to_bits());
        assert_eq!(run(&mut e, 0x220a, &[]).to_bits(), r.to_bits());
        assert_eq!(eval(&mut e, 0x220a, 0).to_bits(), r.to_bits());
    }
    assert_eq!(run(&mut e, 0x220b, &[]), 7849.0 * (1.0 / 32767.0));
    assert_eq!(run(&mut e, 0x220c, &[]), 8689.0 * (1.0 / 32767.0));
    let (_, lines) = echoed(&mut e, 0x220b, 0, &[]);
    assert_eq!(lines, vec![b"GetDeadZoneLS >> 0.23954".to_vec()]);
    let (_, lines) = echoed(&mut e, 0x220c, 0, &[]);
    assert_eq!(lines, vec![b"GetDeadZoneRS >> 0.265175".to_vec()]);
    // A different dead zone in the state is the one read.
    input(&mut e).dead_zone_rs = 3277;
    input(&mut e).pad.thumb_rx = 3278;
    assert_eq!(run(&mut e, 0x2207, &[]), 1.0 * (1.0 / 29490.0));
}

/// The button mods: each command's bit operation on the 16-bit fields (the argument stored as
/// 16 bits; the disabled test compares the whole argument).
#[test]
fn button_mods() {
    let mut e = setup();
    assert_eq!(input(&mut e).mods.button_skip, 0xfbff);
    run(&mut e, 0x2295, &[n(0x1_1000)]);
    assert_eq!(input(&mut e).mods.button_skip, 0xebff, "DisableButton");
    assert_eq!(run(&mut e, 0x2296, &[n(0x1000)]), 1.0);
    assert_eq!(run(&mut e, 0x2296, &[n(0x1001)]), 0.0, "one still passes");
    assert_eq!(run(&mut e, 0x2296, &[n(0x1_0000)]), 1.0, "no bit in 16 passes");
    assert_eq!(eval(&mut e, 0x2296, 0x1000), 1.0);
    assert_eq!(eval(&mut e, 0x2296, 0x0001), 0.0);
    assert_eq!(eval(&mut e, 0x2296, 0x1_0000), 1.0, "(UInt16) 0: nothing passes");
    run(&mut e, 0x2294, &[n(0x1_1400)]);
    assert_eq!(input(&mut e).mods.button_skip, 0xffff, "EnableButton");
    run(&mut e, 0x2297, &[n(0x1_0003)]);
    assert_eq!(input(&mut e).mods.button_hold, 3, "HoldButton");
    assert_eq!(run(&mut e, 0x2299, &[n(2)]), 1.0);
    assert_eq!(run(&mut e, 0x2299, &[n(4)]), 0.0);
    assert_eq!(eval(&mut e, 0x2299, 1), 1.0);
    assert_eq!(eval(&mut e, 0x2299, 0x1_0000), 0.0);
    run(&mut e, 0x2298, &[n(1)]);
    assert_eq!(input(&mut e).mods.button_hold, 2, "ReleaseButton");
    run(&mut e, 0x27fd, &[n(0x1_8000)]);
    run(&mut e, 0x27fd, &[n(0x0040)]);
    assert_eq!(input(&mut e).mods.button_tap, 0x8040, "TapButton adds");
}

#[test]
fn trigger_mods() {
    let mut e = setup();
    let t = |e: &mut Engine| input(e).mods.trigger_mods;
    run(&mut e, 0x22a5, &[n(0)]);
    assert_eq!(t(&mut e), 1, "left disabled");
    run(&mut e, 0x22a5, &[n(2)]);
    assert_eq!(t(&mut e), 3, "any non-zero is the right");
    assert_eq!(run(&mut e, 0x22a6, &[n(0)]), 1.0);
    assert_eq!(eval(&mut e, 0x22a6, 7), 1.0);
    run(&mut e, 0x22a4, &[n(0)]);
    assert_eq!(t(&mut e), 2, "left enabled: & 0xFE");
    assert_eq!(run(&mut e, 0x22a6, &[n(0)]), 0.0);
    assert_eq!(eval(&mut e, 0x22a6, 0), 0.0);
    run(&mut e, 0x22a4, &[n(-1)]);
    assert_eq!(t(&mut e), 0, "right enabled: & 0xFD");
    run(&mut e, 0x22a7, &[n(0)]);
    run(&mut e, 0x22a7, &[n(1)]);
    assert_eq!(t(&mut e), 0xc, "both held");
    assert_eq!(run(&mut e, 0x22a9, &[n(1)]), 1.0);
    assert_eq!(eval(&mut e, 0x22a9, 0), 1.0);
    run(&mut e, 0x22a8, &[n(1)]);
    assert_eq!(t(&mut e), 4, "right released: & 0xF7");
    assert_eq!(run(&mut e, 0x22a9, &[n(1)]), 0.0);
    assert_eq!(eval(&mut e, 0x22a9, 1), 0.0);
    run(&mut e, 0x22a8, &[n(0)]);
    assert_eq!(t(&mut e), 0, "left released: & 0xFB");
    input(&mut e).mods.trigger_mods = 0xc3;
    run(&mut e, 0x22a8, &[n(0)]);
    assert_eq!(t(&mut e), 0xc3 & 0xfb, "only its bit");
    input(&mut e).mods.trigger_mods = 0;
    run(&mut e, 0x27fe, &[n(0)]);
    run(&mut e, 0x27fe, &[n(9)]);
    assert_eq!(t(&mut e), 0x30, "TapTrigger");
}

#[test]
fn stick_mods() {
    let mut e = setup();
    run(&mut e, 0x289e, &[n(0), n(1)]);
    assert_eq!(input(&mut e).mods.stick_skip, 0x0f);
    assert_eq!(run(&mut e, 0x289d, &[n(0)]), 1.0);
    assert_eq!(run(&mut e, 0x289d, &[n(1)]), 0.0);
    assert_eq!(eval(&mut e, 0x289d, 0), 1.0);
    assert_eq!(eval(&mut e, 0x289d, 5), 0.0);
    run(&mut e, 0x289e, &[n(3), n(7)]);
    assert_eq!(input(&mut e).mods.stick_skip, 0xff);
    run(&mut e, 0x289e, &[n(0), n(0)]);
    assert_eq!(input(&mut e).mods.stick_skip, 0xf0);
    input(&mut e).mods.stick_skip = 0x04;
    assert_eq!(run(&mut e, 0x289d, &[n(0)]), 1.0, "one direction is enough");
    run(&mut e, 0x289e, &[n(1), n(0)]);
    assert_eq!(input(&mut e).mods.stick_skip, 0x04, "the right's bits only");
}

/// Trigger handlers: only a Script form while the controller is ready; down/up by the opcode,
/// the trigger by the third argument's low bit; the flag set on an add and cleared when neither
/// trigger has a handler of that kind left; the lambda's variables kept while set.
#[test]
fn trigger_handlers() {
    let mut e = setup();
    let ev = |e: &mut Engine| e.ext::<Jip>().ln_events.triggers.clone();
    let flags = |e: &mut Engine| e.ext::<Jip>().ln_events.flags;
    let down = crate::ln_events::mask(crate::ln_events::id::TRIGGER_DOWN);
    let up = crate::ln_events::mask(crate::ln_events::id::TRIGGER_UP);
    run(&mut e, 0x27ff, &[r(1), n(1), n(0)]);
    assert!(ev(&mut e).iter().all(|s| s.is_empty()), "not ready: nothing");
    input(&mut e).ready = true;
    run(&mut e, 0x27ff, &[r(3), n(1), n(0)]);
    assert!(ev(&mut e).iter().all(|s| s.is_empty()), "not a script");
    nvse::lambda::insert_for_tests(&mut e, SCRIPT_FORM, FORMS + 0x8000);
    run(&mut e, 0x27ff, &[r(1), n(1), n(0)]);
    run(&mut e, 0x27ff, &[r(1), n(1), n(0)]);
    assert_eq!(nvse::lambda::saved_count(&e, SCRIPT_FORM), 1, "kept once");
    run(&mut e, 0x27ff, &[r(2), n(1), n(3)]);
    run(&mut e, 0x2800, &[r(2), n(1), n(2)]);
    let s = ev(&mut e);
    assert_eq!(
        s.iter()
            .map(|x| x.iter().copied().collect::<Vec<_>>())
            .collect::<Vec<_>>(),
        vec![vec![SCRIPT_TWO], vec![SCRIPT_FORM], vec![], vec![SCRIPT_TWO]],
        "left up, left down, right up, right down"
    );
    assert_eq!(flags(&mut e), down | up);
    run(&mut e, 0x27ff, &[r(1), n(0), n(0)]);
    assert_eq!(nvse::lambda::saved_count(&e, SCRIPT_FORM), 0, "given up");
    assert_eq!(flags(&mut e), down | up, "the right trigger still has one");
    run(&mut e, 0x27ff, &[r(2), n(0), n(1)]);
    assert_eq!(flags(&mut e), up, "no down handler left");
    run(&mut e, 0x2800, &[r(1), n(0), n(0)]);
    assert_eq!(flags(&mut e), up, "not there: nothing");
    run(&mut e, 0x2800, &[r(2), n(0), n(0)]);
    assert_eq!(flags(&mut e), 0);
    assert!(ev(&mut e).iter().all(|s| s.is_empty()));
}
