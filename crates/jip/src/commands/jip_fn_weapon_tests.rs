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
