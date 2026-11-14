//! Tests of the JIP weapon form commands, at raw offsets (the weapon loader `0051F630` and JIP's
//! header): own model +0x3C, pickup/putdown sounds +0xEC/+0xF0, embedded actor value +0x11F,
//! on-hit +0x128, semi-automatic delays +0x174/+0x178, mod effects +0x180, mod values +0x18C,
//! reload animation +0x1A1, shell casing model +0x1D0, sounds +0x21C, mod models +0x270 (0x20
//! each), item mods +0x350, embedded node name +0x35C, sound level +0x364.

use super::*;
use crate::commands::test_support::*;

const WEAP: u32 = FORMS + 0x1000;
const SOUND: u32 = FORMS + 0x2000;
const IMOD: u32 = FORMS + 0x2100;
const MISC: u32 = FORMS + 0x3000;
const SIZE: u32 = 0x388;

fn setup() -> Engine {
    let mut e = engine();
    form(&mut e, WEAP, 0x28, 0x0000_3ea9);
    form(&mut e, SOUND, 0x0d, 0x0000_50d0);
    form(&mut e, IMOD, 0x67, 0x0000_1a0d);
    form(&mut e, MISC, 0x1f, 0x0000_0999);
    for (i, f) in [WEAP, SOUND, IMOD, MISC].iter().enumerate() {
        set_ref(&mut e, i as u16 + 1, *f);
    }
    e
}

/// The weapon's bytes with the word at `off` set to `value`.
fn with_word(e: &Engine, off: usize, value: u32) -> Vec<u8> {
    let mut want = e.mem.bytes(WEAP, SIZE);
    want[off..off + 4].copy_from_slice(&value.to_le_bytes());
    want
}

#[test]
fn the_sound_level() {
    let mut e = setup();
    e.mem.set_u32(WEAP + 0x364, 0xffff_ffff);
    assert_eq!(call(&mut e, 0x264a, &[r(1)]).1, -1.0, "(int)");
    for (v, kept) in [(2, true), (3, false), (0, true), (-1, false)] {
        let before = e.mem.u32(WEAP + 0x364);
        call(&mut e, 0x264b, &[r(1), n(v)]);
        let want = if kept { v as u32 } else { before };
        assert_eq!(e.mem.u32(WEAP + 0x364), want, "level {v}");
    }
}

#[test]
fn the_sounds() {
    let mut e = setup();
    for (kind, off) in [(0u32, 0x21cusize), (11, 0x21c + 44), (12, 0xec), (13, 0xf0)] {
        let want = with_word(&e, off, SOUND);
        call(&mut e, 0x264f, &[r(1), n(kind as i32), r(2)]);
        assert_eq!(e.mem.bytes(WEAP, SIZE), want, "sound {kind}: only its word");
        call(&mut e, 0x264e, &[r(1), n(kind as i32)]);
        assert_eq!(form_result(&e), 0x50d0, "sound {kind}");
        call(&mut e, 0x264f, &[r(1), n(kind as i32)]);
        assert_eq!(e.mem.u32(WEAP + off as u32), 0, "sound {kind}: cleared");
    }
    let before = e.mem.bytes(WEAP, SIZE);
    call(&mut e, 0x264f, &[r(1), n(14), r(2)]);
    assert_eq!(e.mem.bytes(WEAP, SIZE), before, "kind 14: nothing");
    call(&mut e, 0x264e, &[r(1), n(1)]);
    assert_eq!(form_result(&e), 0, "none");
}

#[test]
fn the_mods() {
    let mut e = setup();
    for slot in 1..=3u32 {
        let i = (slot - 1) as usize;
        let want = with_word(&e, 0x350 + 4 * i, IMOD);
        call(&mut e, 0x2650, &[r(1), n(slot as i32), r(3)]);
        assert_eq!(e.mem.bytes(WEAP, SIZE), want, "mod {slot}");
        call(&mut e, 0x2650, &[r(1), n(slot as i32), r(4)]);
        assert_eq!(
            e.mem.u32(WEAP + 0x350 + 4 * i as u32),
            0,
            "a misc item clears it"
        );
        let want = with_word(&e, 0x180 + 4 * i, 14);
        call(&mut e, 0x2651, &[r(1), n(slot as i32), n(14)]);
        assert_eq!(e.mem.bytes(WEAP, SIZE), want, "effect {slot}");
        let want = with_word(&e, 0x18c + 4 * i, (2.5f32 + slot as f32).to_bits());
        call(
            &mut e,
            0x2652,
            &[r(1), n(slot as i32), z(2.5 + f64::from(slot))],
        );
        assert_eq!(e.mem.bytes(WEAP, SIZE), want, "value {slot}");
    }
    let before = e.mem.bytes(WEAP, SIZE);
    for slot in [0, 4] {
        call(&mut e, 0x2650, &[r(1), n(slot), r(3)]);
        call(&mut e, 0x2651, &[r(1), n(slot), n(1)]);
        call(&mut e, 0x2652, &[r(1), n(slot), z(1.0)]);
    }
    for kind in [0, 5, 15] {
        call(&mut e, 0x2651, &[r(1), n(1), n(kind)]);
    }
    assert_eq!(
        e.mem.bytes(WEAP, SIZE),
        before,
        "slots 0, 4 and effects 0, 5, 15: nothing"
    );
}

#[test]
fn the_reload_animation_and_embedded_value() {
    let mut e = setup();
    e.mem.set_u32(WEAP + 0x1a0, 0xaaaa_05aa);
    assert_eq!(call(&mut e, 0x26bb, &[r(1)]).1, 5.0);
    call(&mut e, 0x26bc, &[r(1), n(22)]);
    assert_eq!(e.mem.u32(WEAP + 0x1a0), 0xaaaa_16aa, "one byte");
    call(&mut e, 0x26bc, &[r(1), n(23)]);
    assert_eq!(e.mem.u8(WEAP + 0x1a1), 22, "23 refused");
    e.mem.set_u32(WEAP + 0x11c, 0x00aa_aaaa);
    call(&mut e, 0x277f, &[r(1), vec![0x2b, 0x01]]);
    assert_eq!(
        e.mem.u32(WEAP + 0x11c),
        0x2baa_aaaa,
        "the embedded actor value byte"
    );
    // A parenthesised argument: the evaluator's value (`(44)`, a byte token in an expression).
    let mut arg = vec![0xff, 0xff];
    arg.extend_from_slice(&4u16.to_le_bytes());
    arg.extend_from_slice(&[b'B', 0x2c]);
    call(&mut e, 0x277f, &[r(1), arg]);
    assert_eq!(
        e.mem.u32(WEAP + 0x11c),
        0x2caa_aaaa,
        "through the expression evaluator"
    );
}

#[test]
fn on_hit_and_delays() {
    let mut e = setup();
    e.mem.set_u32(WEAP + 0x128, 2);
    assert_eq!(call(&mut e, 0x2899, &[r(1)]).1, 2.0);
    assert_eq!(call(&mut e, 0x2899, &[r(4)]).1, -1.0, "a misc item: -1");
    call(&mut e, 0x289a, &[r(1), n(3)]);
    assert_eq!(e.mem.u32(WEAP + 0x128), 3);
    call(&mut e, 0x289a, &[r(1), n(4)]);
    assert_eq!(e.mem.u32(WEAP + 0x128), 3, "4 refused");
    e.mem.set_f32(WEAP + 0x174, 0.1);
    e.mem.set_f32(WEAP + 0x178, 0.5);
    assert_eq!(call(&mut e, 0x289b, &[r(1), n(0)]).1, f64::from(0.1f32));
    assert_eq!(
        call(&mut e, 0x289b, &[r(1), n(2)]).1,
        0.5,
        "any non-zero: the maximum"
    );
    call(&mut e, 0x289c, &[r(1), n(7), z(0.75)]);
    assert_eq!(e.mem.f32(WEAP + 0x178), 0.75);
    assert_eq!(e.mem.f32(WEAP + 0x174), 0.1, "the minimum untouched");
    let want = with_word(&e, 0x174, 0.25f32.to_bits());
    call(&mut e, 0x289c, &[r(1), n(0), z(0.25)]);
    assert_eq!(e.mem.bytes(WEAP, SIZE), want, "index 0: only the minimum");
    let want = with_word(&e, 0x178, 1.25f32.to_bits());
    call(&mut e, 0x289c, &[r(1), n(2), z(1.25)]);
    assert_eq!(e.mem.bytes(WEAP, SIZE), want, "index 2: only the maximum");
}

#[test]
fn the_models_and_node_name() {
    let mut e = setup();
    model(&mut e, WEAP + 0x1d0, Some(b"casing.nif"));
    let (_, res) = call(&mut e, 0x26f5, &[r(1)]);
    assert_eq!(string_result(&e, res), b"casing.nif");
    CALLS.with(|c| c.borrow_mut().clear());
    call(&mut e, 0x26f6, &[r(1), s(b"other.nif")]);
    assert_eq!(text_at(&e, WEAP + 0x1d4), b"other.nif");
    assert_eq!(allocations(), vec![10], "the game's setter, once");
    // Mod flags 0-7: the own model, then the mod models with 3 and 4 swapped.
    model(&mut e, WEAP + 0x3c, Some(b"own.nif"));
    for i in 0..7u32 {
        let at = WEAP + 0x270 + 0x20 * i;
        e.mem.set_u32(at, TES_MODEL_TEXTURE_SWAP_VTBL);
        string(&mut e, at + 4, Some(format!("m{i}.nif").as_bytes()));
    }
    let want = [
        "own.nif", "m0.nif", "m1.nif", "m3.nif", "m2.nif", "m4.nif", "m5.nif", "m6.nif",
    ];
    for (flags, name) in want.iter().enumerate() {
        let (_, res) = call(&mut e, 0x28a9, &[r(1), n(flags as i32)]);
        assert_eq!(string_result(&e, res), name.as_bytes(), "flags {flags}");
    }
    let (_, res) = call(&mut e, 0x28a9, &[r(1), n(8)]);
    assert_eq!(string_result(&e, res), b"", "flags 8");
    CALLS.with(|c| c.borrow_mut().clear());
    call(&mut e, 0x28aa, &[r(1), n(4), s(b"new.nif")]);
    assert_eq!(allocations(), vec![8], "the game's setter, once");
    assert_eq!(
        text_at(&e, WEAP + 0x270 + 0x40 + 4),
        b"new.nif",
        "flags 4: model 2"
    );
    let before = e.mem.bytes(WEAP, SIZE);
    call(&mut e, 0x28aa, &[r(1), n(8), s(b"x.nif")]);
    assert_eq!(e.mem.bytes(WEAP, SIZE), before, "flags 8: nothing");
    CALLS.with(|c| c.borrow_mut().clear());
    let first = heap_next(&e);
    call(&mut e, 0x277e, &[r(1), s(b"Bip01 Head")]);
    assert_eq!(text_at(&e, WEAP + 0x35c), b"Bip01 Head");
    assert_eq!(
        allocations(),
        vec![11],
        "no buffer before: its length plus one"
    );
    assert!(freed().is_empty(), "nothing to free");
    assert_eq!(e.mem.u32(WEAP + 0x35c), first);
    let second = heap_next(&e);
    call(&mut e, 0x277e, &[r(1), s(b"Bip01 R Hand2")]);
    assert_eq!(text_at(&e, WEAP + 0x35c), b"Bip01 R Hand2");
    assert_eq!(allocations(), vec![11, 14], "longer: a new buffer");
    assert_eq!(freed(), vec![first], "the first buffer freed");
    assert_eq!(e.mem.u32(WEAP + 0x35c), second);
    let long = vec![b'n'; 0x40];
    let stopped = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        call(&mut e, 0x277e, &[r(1), s(&long)]);
    }));
    assert!(
        stopped.is_err(),
        "0x40 bytes run past JIP's 0x40-byte buffer"
    );
}

/// Another inventory form (a misc item: the reader passes it for `kParamType_ObjectID`, `IS_ID`
/// refuses it) filled with a non-zero pattern answers 0 (-1 for on-hit) and keeps its bytes;
/// every setter writes a value that differs from the fill.
#[test]
fn another_form_type_is_left_alone() {
    let mut e = setup();
    for off in 0x3c..SIZE {
        e.mem.set_u8(MISC + off, 0xa5);
    }
    let before = e.mem.bytes(MISC, SIZE);
    assert_eq!(call(&mut e, 0x264a, &[r(4)]).1, 0.0);
    assert_eq!(call(&mut e, 0x26bb, &[r(4)]).1, 0.0);
    assert_eq!(call(&mut e, 0x289b, &[r(4), n(0)]).1, 0.0);
    call(&mut e, 0x264e, &[r(4), n(0)]);
    assert_eq!(form_result(&e), 0);
    let (_, res) = call(&mut e, 0x26f5, &[r(4)]);
    assert_eq!(string_result(&e, res), b"");
    let (_, res) = call(&mut e, 0x28a9, &[r(4), n(0)]);
    assert_eq!(string_result(&e, res), b"");
    call(&mut e, 0x264b, &[r(4), n(0)]);
    call(&mut e, 0x264f, &[r(4), n(0), r(2)]);
    call(&mut e, 0x2650, &[r(4), n(1), r(3)]);
    call(&mut e, 0x2651, &[r(4), n(1), n(3)]);
    call(&mut e, 0x2652, &[r(4), n(1), z(2.0)]);
    call(&mut e, 0x26bc, &[r(4), n(7)]);
    call(&mut e, 0x26f6, &[r(4), s(b"x.nif")]);
    call(&mut e, 0x277e, &[r(4), s(b"x")]);
    call(&mut e, 0x277f, &[r(4), vec![0x12, 0]]);
    call(&mut e, 0x289a, &[r(4), n(0)]);
    call(&mut e, 0x289c, &[r(4), n(0), z(2.0)]);
    call(&mut e, 0x28aa, &[r(4), n(0), s(b"x.nif")]);
    assert_eq!(e.mem.bytes(MISC, SIZE), before);
}

// ---- batch 12: the equipped weapon, mod slots, shots a second, the attack queue ----

mod equipped {
    use super::super::*;
    use crate::commands::test_support::*;

    const WEAP: u32 = FORMS + 0x1000;
    const MISC: u32 = FORMS + 0x3000;
    const ACTOR: u32 = FORMS + 0x4000;
    const PLAIN: u32 = FORMS + 0x4100;
    const DROPPER: u32 = FORMS + 0x4200;
    const WREF: u32 = FORMS + 0x4300;
    const PROC: u32 = FORMS + 0x5000;
    const ENTRY: u32 = FORMS + 0x5200;
    const LISTS: u32 = FORMS + 0x5300;
    const XDL: u32 = FORMS + 0x5400;
    const X1: u32 = FORMS + 0x5500;
    const X2: u32 = FORMS + 0x5540;
    const X3: u32 = FORMS + 0x5580;
    const GROUP: u32 = FORMS + 0x5600;
    const TIMES: u32 = FORMS + 0x5700;
    const ANIM: u32 = FORMS + 0x5800;
    const TEMP_REF: u32 = FORMS + 0x5900;
    const VTBL: u32 = FORMS + 0x6000;
    const PLAIN_VTBL: u32 = FORMS + 0x6400;

    fn setup() -> Engine {
        let mut e = engine();
        extra_stand_ins(&mut e);
        e.map(0x011e_0000, 0x1000);
        e.map(0x0118_a000, 0x1000);
        form(&mut e, WEAP, 0x28, 0x0000_3ea9);
        form(&mut e, MISC, 0x1f, 0x0000_0999);
        form(&mut e, ACTOR, 0x3b, 0x0000_7000);
        form(&mut e, PLAIN, 0x3a, 0x0000_7100);
        form(&mut e, DROPPER, 0x3a, 0x0000_7200);
        form(&mut e, WREF, 0x3a, 0x0000_7300);
        e.mem.set_u32(VTBL + 0x100, 0x008d_0360);
        e.mem.set_u32(VTBL + 0xf0, 0x008d_0360);
        e.mem.set_u32(PLAIN_VTBL + 0xf0, 0x008d_0360);
        e.mem.set_u32(VTBL + 0x1e4, 0x0050_2000);
        e.register(0x0050_2000, |e, _| Ret {
            eax: e.mem.u32(ANIM + 0x100),
            ..Ret::default()
        });
        e.mem.set_u32(ACTOR, VTBL);
        e.mem.set_u32(WEAP, PLAIN_VTBL + 0x400);
        e.mem.set_u32(MISC, PLAIN_VTBL + 0x400);
        for r in [PLAIN, DROPPER, WREF] {
            e.mem.set_u32(r, PLAIN_VTBL);
        }
        e.mem.set_u32(WREF + 0x20, WEAP);
        e.mem.set_u32(ACTOR + 0x68, PROC);
        e.mem.set_u32(PROC + 0x114, ENTRY);
        e.mem.set_u32(ENTRY + 8, WEAP);
        e.mem.set_u32(ENTRY, LISTS);
        e.mem.set_u32(LISTS, XDL);
        for (i, f) in [WEAP, MISC, WREF].iter().enumerate() {
            set_ref(&mut e, i as u16 + 1, *f);
        }
        e.register(PICK_BEST_ANIMATION, |e, w| {
            CALLS.with(|c| c.borrow_mut().push((PICK_BEST_ANIMATION, w.to_vec())));
            Ret {
                eax: e.mem.u32(ANIM + 0x104),
                ..Ret::default()
            }
        });
        e.register(GET_ANIM_GROUP, |e, w| {
            CALLS.with(|c| c.borrow_mut().push((GET_ANIM_GROUP, w.to_vec())));
            Ret {
                eax: e.mem.u32(ANIM + 0x108),
                ..Ret::default()
            }
        });
        e
    }

    fn run(e: &mut Engine, opcode: u32, this: u32, args: &[Vec<u8>]) -> f64 {
        e.mem.set_f64(RESULT, -5.0);
        call_on(e, opcode, this, args).1
    }

    fn eval(e: &mut Engine, opcode: u32, this: u32, arg1: u32) -> f64 {
        let record = nvse::table::record_of(e, opcode);
        let at = e.mem.u32(record + 0x20);
        e.mem.set_f64(RESULT, -5.0);
        e.call(at, &[this, arg1, 0, RESULT]);
        e.mem.f64(RESULT)
    }

    /// Mod slot `i` (0..3) of the weapon with effect word `effect`.
    fn slot(e: &mut Engine, i: u32, effect: u32) {
        e.mem.set_u32(WEAP + 0x180 + 4 * i, effect);
    }

    #[test]
    fn silenced() {
        let mut e = setup();
        let both = |e: &mut Engine| (run(e, 0x264c, ACTOR, &[]), eval(e, 0x264c, ACTOR, 0));
        assert_eq!(both(&mut e), (0.0, 0.0), "no silencer");
        e.mem.set_u32(WEAP + 0x364, 2);
        assert_eq!(both(&mut e), (1.0, 1.0), "silent sound level");
        e.mem.set_u8(PROC + 0x28, 2);
        assert_eq!(both(&mut e), (0.0, 0.0), "a low process has no weapon info");
        e.mem.set_u8(PROC + 0x28, 1);
        e.mem.set_u32(WEAP + 0x364, 1);
        put_extra(&mut e, XDL, X1, 0x8d, 2);
        slot(&mut e, 1, 0x0000_010b);
        assert_eq!(both(&mut e), (2.0, 2.0), "slot 2's effect byte 11");
        slot(&mut e, 1, 12);
        slot(&mut e, 0, 11);
        assert_eq!(both(&mut e), (0.0, 0.0), "slot 1 has the effect but no mod");
        e.mem.set_u32(X1 + 0xc, 5);
        assert_eq!(both(&mut e), (2.0, 2.0));
        assert_eq!(
            (
                run(&mut e, 0x264c, PLAIN, &[]),
                eval(&mut e, 0x264c, PLAIN, 0)
            ),
            (0.0, -5.0),
            "not an actor: untouched"
        );
        e.mem.set_u32(PROC + 0x114, 0);
        assert_eq!(both(&mut e), (0.0, 0.0), "no weapon info");
        e.mem.set_u32(ACTOR + 0x68, 0);
        assert_eq!(both(&mut e), (0.0, 0.0), "no process");
    }

    /// An `ExtraItemDropper` with a reference: the dropper's own list's flags count; with no
    /// reference the entry's list's.
    #[test]
    fn mod_flags_through_the_dropper() {
        let mut e = setup();
        slot(&mut e, 2, 11);
        put_extra(&mut e, XDL, X1, 0x8d, 1);
        put_extra(&mut e, XDL, X2, 0x39, DROPPER);
        assert_eq!(
            run(&mut e, 0x264c, ACTOR, &[]),
            0.0,
            "the dropper has no flags"
        );
        put_extra(&mut e, DROPPER + 0x44, X3, 0x8d, 4);
        assert_eq!(run(&mut e, 0x264c, ACTOR, &[]), 2.0, "the dropper's slot 3");
        e.mem.set_u32(X2 + 0xc, 0);
        assert_eq!(
            run(&mut e, 0x264c, ACTOR, &[]),
            0.0,
            "own flags: slot 1 only"
        );
        e.mem.set_u32(X1 + 0xc, 4);
        assert_eq!(run(&mut e, 0x264c, ACTOR, &[]), 2.0);
        // Not a weapon in the entry: no flags at all.
        e.mem.set_u32(ENTRY + 8, MISC);
        assert_eq!(entry_mod_flags(&mut e, ENTRY), 0);
        e.mem.set_u32(ENTRY + 8, WEAP);
        assert_eq!(entry_mod_flags(&mut e, ENTRY), 4);
        e.mem.set_u32(LISTS, 0);
        assert_eq!(entry_mod_flags(&mut e, ENTRY), 0, "no first list");
        e.mem.set_u32(ENTRY, 0);
        assert_eq!(entry_mod_flags(&mut e, ENTRY), 0, "no lists");
        assert_eq!(entry_mod_flags(&mut e, 0), 0);
    }

    #[test]
    fn scoped() {
        let mut e = setup();
        let both = |e: &mut Engine| (run(e, 0x264d, ACTOR, &[]), eval(e, 0x264d, ACTOR, 0));
        e.mem.set_u8(WEAP + 0x100, 0xfb);
        assert_eq!(both(&mut e), (0.0, 0.0), "no scope bit");
        e.mem.set_u8(WEAP + 0x100, 4);
        assert_eq!(both(&mut e), (1.0, 1.0), "its own scope");
        e.mem.set_u32(WEAP + 0x12c, 0x2000);
        assert_eq!(both(&mut e), (0.0, 0.0), "a mod's scope, the mod not on");
        put_extra(&mut e, XDL, X1, 0x8d, 1);
        slot(&mut e, 0, 14);
        assert_eq!(both(&mut e), (2.0, 2.0));
        slot(&mut e, 0, 11);
        assert_eq!(both(&mut e), (0.0, 0.0), "another mod");
        assert_eq!(run(&mut e, 0x264d, PLAIN, &[]), 0.0);
    }

    #[test]
    fn equipped_mod_type() {
        let mut e = setup();
        put_extra(&mut e, XDL, X1, 0x8d, 2);
        slot(&mut e, 1, 7);
        assert_eq!(run(&mut e, 0x28f7, ACTOR, &[n(7)]), 1.0);
        assert_eq!(run(&mut e, 0x28f7, ACTOR, &[n(0x107)]), 1.0, "the low byte");
        assert_eq!(run(&mut e, 0x28f7, ACTOR, &[n(8)]), 0.0, "untouched");
        assert_eq!(eval(&mut e, 0x28f7, ACTOR, 7), 1.0);
        assert_eq!(eval(&mut e, 0x28f7, ACTOR, 0x0300_0007), 1.0);
        assert_eq!(eval(&mut e, 0x28f7, ACTOR, 6), -5.0);
        assert_eq!(run(&mut e, 0x28f7, PLAIN, &[n(7)]), 0.0);
        assert_eq!(eval(&mut e, 0x28f7, PLAIN, 7), -5.0);
        e.mem.set_u32(PROC + 0x114, 0);
        assert_eq!(run(&mut e, 0x28f7, ACTOR, &[n(7)]), 0.0);
    }

    /// The first slot with an item mod and the effect (the whole word): its number; 0 written
    /// otherwise (a non-weapon too); a reference's base weapon.
    #[test]
    fn mod_type_slots() {
        let mut e = setup();
        slot(&mut e, 0, 5);
        slot(&mut e, 1, 5);
        slot(&mut e, 2, 5);
        assert_eq!(run(&mut e, 0x2924, 0, &[r(1), n(5)]), 0.0, "no item mods");
        e.mem.set_u32(WEAP + 0x350 + 4, MISC);
        e.mem.set_u32(WEAP + 0x350 + 8, MISC);
        assert_eq!(
            run(&mut e, 0x2924, 0, &[r(1), n(5)]),
            2.0,
            "slot 1 has no item mod"
        );
        assert_eq!(
            run(&mut e, 0x2924, 0, &[r(1), n(0x105)]),
            0.0,
            "the whole word"
        );
        slot(&mut e, 1, 6);
        assert_eq!(run(&mut e, 0x2924, 0, &[r(1), n(5)]), 3.0);
        assert_eq!(
            run(&mut e, 0x2924, 0, &[r(3), n(5)]),
            3.0,
            "the reference's base"
        );
        assert_eq!(
            run(&mut e, 0x2924, 0, &[r(2), n(5)]),
            0.0,
            "not a weapon: 0 written"
        );
    }

    #[test]
    fn attack_queued() {
        let mut e = setup();
        assert_eq!(run(&mut e, 0x2941, 0, &[]), 0.0);
        assert_eq!(eval(&mut e, 0x2941, 0, 0), -5.0);
        e.mem.set_u8(0x011e_07ac, 1);
        assert_eq!(run(&mut e, 0x2941, 0, &[]), 1.0);
        assert_eq!(eval(&mut e, 0x2941, 0, 0), 1.0);
        e.mem.set_u8(0x011e_07ad, 1);
        e.mem.set_u8(0x011e_07ac, 0);
        assert_eq!(run(&mut e, 0x2941, 0, &[]), 0.0, "only that byte");
    }

    fn weapon_numbers(e: &mut Engine, flags1: u8) {
        e.mem.set_u8(WEAP + 0x100, flags1);
        e.mem.set_f32(WEAP + 0x130, 1.25);
        e.mem.set_f32(WEAP + 0xf8, 0.8);
        e.mem.set_f32(WEAP + 0x134, 3.0);
        e.mem.set_u8(WEAP + 0xf4, 3);
        e.mem.set_u8(WEAP + 0x11d, 0xff);
        e.mem.set_u32(0x0118_a838 + 12, 4);
    }

    /// Automatic (or no anim data): `m*m*rate*anim` in floats; otherwise the attack group's
    /// next-to-last key time, when the best animation is the weapon's attack (0xFF: 0x20) and the
    /// group has two keys or more.
    #[test]
    fn shots_per_second() {
        let mut e = setup();
        weapon_numbers(&mut e, 2);
        let auto = f64::from(1.25f32 * 1.25f32 * 3.0f32 * 0.8f32);
        assert_eq!(run(&mut e, 0x2940, ACTOR, &[r(1)]), auto);
        weapon_numbers(&mut e, 0);
        assert_eq!(run(&mut e, 0x2940, ACTOR, &[r(1)]), auto, "no anim data");
        e.mem.set_u32(ANIM + 0x100, ANIM);
        e.mem.set_u32(ANIM + 0x104, 0xab21);
        assert_eq!(
            run(&mut e, 0x2940, ACTOR, &[r(1)]),
            0.0,
            "another animation"
        );
        assert_eq!(
            calls_to(PICK_BEST_ANIMATION),
            vec![vec![ANIM, (4 << 8) | 0x20, 0]]
        );
        e.mem.set_u32(ANIM + 0x104, 0xffff_0420);
        assert_eq!(run(&mut e, 0x2940, ACTOR, &[r(1)]), 0.0, "no group");
        assert_eq!(
            calls_to(GET_ANIM_GROUP),
            vec![vec![ANIM, 0x0420]],
            "16 bits"
        );
        e.mem.set_u32(ANIM + 0x108, GROUP);
        e.mem.set_u32(GROUP + 0x14, 1);
        e.mem.set_u32(GROUP + 0x18, TIMES);
        assert_eq!(run(&mut e, 0x2940, ACTOR, &[r(1)]), 0.0, "one key");
        e.mem.set_u32(GROUP + 0x14, 3);
        e.mem.set_f32(TIMES, 0.1);
        e.mem.set_f32(TIMES + 4, 0.3);
        e.mem.set_f32(TIMES + 8, 0.9);
        let want = f64::from((1.25f32 * 0.8f32) / 0.3f32);
        assert_eq!(run(&mut e, 0x2940, ACTOR, &[r(1)]), want);
        e.mem.set_u8(WEAP + 0x11d, 0x21);
        e.mem.set_u32(ANIM + 0x104, 0x0421);
        assert_eq!(run(&mut e, 0x2940, ACTOR, &[r(1)]), want, "its own attack");
        // Not an actor, not a weapon, a reference that is no inventory reference: nothing.
        assert_eq!(run(&mut e, 0x2940, PLAIN, &[r(1)]), 0.0);
        assert_eq!(run(&mut e, 0x2940, ACTOR, &[r(2)]), 0.0);
        assert_eq!(run(&mut e, 0x2940, ACTOR, &[r(3)]), 0.0);
    }

    /// An inventory reference: its stack's weapon, and its attack speed mod (effect 8, the
    /// whole word) added to the attack multiplier.
    #[test]
    fn shots_per_second_of_an_inventory_reference() {
        let mut e = setup();
        weapon_numbers(&mut e, 2);
        e.register(0x0040_1000, |_, _| Ret {
            eax: TEMP_REF,
            ..Ret::default()
        });
        e.register(0x0055_a2f0, |e, w| {
            e.mem.set_u32(w[0] + 0xc, 0xff00_0900);
            Ret::default()
        });
        let refr = nvse::invref::create(
            &mut e,
            ACTOR,
            nvse::invref::Data {
                item: WEAP,
                entry: ENTRY,
                xdata: XDL,
            },
            false,
        );
        e.mem.set_u32(refr, PLAIN_VTBL);
        set_ref(&mut e, 4, refr);
        put_extra(&mut e, XDL, X1, 0x8d, 6);
        slot(&mut e, 0, 8);
        slot(&mut e, 1, 8);
        e.mem.set_f32(WEAP + 0x18c, 9.0);
        e.mem.set_f32(WEAP + 0x190, 0.5);
        let m = 1.25f32 + 0.5f32;
        let want = f64::from(m * m * 3.0f32 * 0.8f32);
        assert_eq!(
            run(&mut e, 0x2940, ACTOR, &[r(4)]),
            want,
            "slot 2: slot 1 has no mod"
        );
        slot(&mut e, 1, 0x108);
        let plain = f64::from(1.25f32 * 1.25f32 * 3.0f32 * 0.8f32);
        assert_eq!(run(&mut e, 0x2940, ACTOR, &[r(4)]), plain, "the whole word");
    }
}
