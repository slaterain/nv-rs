//! Group GC's game patches: G53 (detection) and `ToggleDetectionFix`.

use super::*;
use crate::test_support::{engine, execute_of, int_arg, line};

const AREA_BASE: u32 = 0x0520_0000;
const ACTOR_VT: u32 = AREA_BASE;
const NOT_ACTOR_VT: u32 = AREA_BASE + 0x800;
const ACTOR: u32 = AREA_BASE + 0x1000;
const PLAYER: u32 = AREA_BASE + 0x1400;
const NOT_ACTOR: u32 = AREA_BASE + 0x1800;
/// `ToggleDetectionFix`'s opcode.
const TOGGLE_DETECTION_FIX: u32 = 0x275c;

fn world_as(installed: bool) -> Engine {
    let mut e = engine();
    e.map(AREA_BASE, 0x1_0000);
    e.map(THE_PLAYER & !0xfff, 0x1000);
    e.mem.set_u32(ACTOR_VT + 0x100, 0x008d_0360);
    for actor in [ACTOR, PLAYER] {
        e.mem.set_u32(actor, ACTOR_VT);
        e.mem.set_u8(actor + 4, 0x2b);
    }
    e.mem.set_u32(NOT_ACTOR, NOT_ACTOR_VT);
    e.mem.set_u32(THE_PLAYER, PLAYER);
    crate::install_as(&mut e, installed);
    e
}

fn set_flags(e: &mut Engine, actor: u32, flags1: u8, flags2: u8, flags3: u8) {
    e.mem.set_u8(actor + 0x105, flags1);
    e.mem.set_u8(actor + 0x106, flags2);
    e.mem.set_u8(actor + 0x107, flags3);
}

/// Runs `opcode` with `args` through its record on `this`; the handler's `al`.
fn run(e: &mut Engine, opcode: u32, this: u32, args: &[Vec<u8>]) -> u32 {
    let mut words = line(e, opcode, args);
    words[2] = this;
    let at = execute_of(e, opcode);
    e.call(at, &words).eax & 0xff
}

/// While JIP answers "not installed": no G53 seam has JIP's handler and every seam answers
/// nothing (the translation calls the exe's functions); the command has no handler.
#[test]
fn g53_not_installed_nothing_is_put_in() {
    let mut e = world_as(false);
    set_flags(&mut e, ACTOR, DETECTION_FIX, 1, 0);
    for addr in AREA.jip_sites {
        assert!(!e.site_owners(*addr).contains(&Owner::Jip), "{addr:08x}");
        assert_eq!(e.site(*addr, &[ACTOR]), None, "{addr:08x}");
    }
    assert_eq!(execute_of(&e, TOGGLE_DETECTION_FIX), 0);
}

/// Installed with the row `NV_JIP_GAME_PATCHES` off (the vanilla preset): G53 is JIP's own, so
/// its four seams have JIP's handler all the same; the row on adds nothing to them.
#[test]
fn g53_installed_row_off_puts_the_seams_in() {
    let mut e = world_as(true);
    super::super::set_row(&mut e, false);
    for addr in AREA.jip_sites {
        assert_eq!(
            e.site_owners(*addr).to_vec(),
            vec![Owner::Jip],
            "{addr:08x}"
        );
    }
    assert!(AREA.fix_sites.is_empty() && AREA.fix_wraps.is_empty());
    assert_ne!(execute_of(&e, TOGGLE_DETECTION_FIX), 0);
}

/// `DetectionTeammateHook` at each of the three teammate seams: the actor's teammate byte (as
/// read: 0, 1, any byte) unless the detection fix (`+0x105` bit 4) or a forced detection value
/// (`+0x106` bit 0) is set: then 0. Other bits of the two bytes and the third byte do not count.
#[test]
fn g53_teammate_answers_as_jip() {
    let mut e = world_as(true);
    for slot in DETECTION_TEAMMATE {
        let at = slot.addr();
        for byte in [0u8, 1, 0x80] {
            e.mem.set_u8(ACTOR + 0x18d, byte);
            set_flags(&mut e, ACTOR, 0, 0, 0);
            assert_eq!(e.site(at, &[ACTOR]), Some(u32::from(byte)), "{at:08x}");
            set_flags(&mut e, ACTOR, !DETECTION_FIX, 0xfe, 0xff);
            assert_eq!(e.site(at, &[ACTOR]), Some(u32::from(byte)), "other bits");
            set_flags(&mut e, ACTOR, DETECTION_FIX, 0, 0);
            assert_eq!(e.site(at, &[ACTOR]), Some(0), "detection fix");
            set_flags(&mut e, ACTOR, 0, 1, 0);
            assert_eq!(e.site(at, &[ACTOR]), Some(0), "forced value");
            set_flags(&mut e, ACTOR, 0xff, 0xff, 0xff);
            assert_eq!(e.site(at, &[ACTOR]), Some(0), "all");
        }
    }
}

/// `DetectionGetLevelHook`: the player's actor value owner for an actor with the detection fix;
/// no answer (the exe's call on the actor's own owner) without it, with only a forced detection
/// value, with every other bit, or with no player.
#[test]
fn g53_level_source_answers_as_jip() {
    let mut e = world_as(true);
    let at = DETECTION_LEVEL.addr();
    set_flags(&mut e, ACTOR, 0, 0, 0);
    assert_eq!(e.site(at, &[ACTOR]), None);
    set_flags(&mut e, ACTOR, DETECTION_FIX, 0, 0);
    assert_eq!(e.site(at, &[ACTOR]), Some(PLAYER + 0xa4));
    set_flags(&mut e, ACTOR, 0xff, 0xff, 0xff);
    assert_eq!(e.site(at, &[ACTOR]), Some(PLAYER + 0xa4), "all bits");
    set_flags(&mut e, ACTOR, !DETECTION_FIX, 0xff, 0xff);
    assert_eq!(e.site(at, &[ACTOR]), None, "every other bit");
    // The player detected with the fix: its own owner (JIP's answer is the same object).
    set_flags(&mut e, PLAYER, DETECTION_FIX, 0, 0);
    assert_eq!(e.site(at, &[PLAYER]), Some(PLAYER + 0xa4));
    // No player: JIP would call on 0xA4; the port keeps the exe's call.
    e.mem.set_u32(THE_PLAYER, 0);
    set_flags(&mut e, ACTOR, DETECTION_FIX, 0, 0);
    assert_eq!(e.site(at, &[ACTOR]), None);
}

/// `ToggleDetectionFix enable` on an actor: sets or clears `+0x105` bit 4 when it differs (any
/// value other than 0 is "on"), leaving every other bit; nothing on a non-actor, with no
/// reference; a missing number reads as 0. Always answers true.
#[test]
fn toggle_detection_fix_sets_the_flag() {
    let mut e = world_as(true);
    set_flags(&mut e, ACTOR, 0x0f, 0x01, 0x20);
    assert_eq!(run(&mut e, TOGGLE_DETECTION_FIX, ACTOR, &[int_arg(1)]), 1);
    assert_eq!(e.mem.u8(ACTOR + 0x105), 0x1f);
    assert_eq!(run(&mut e, TOGGLE_DETECTION_FIX, ACTOR, &[int_arg(5)]), 1);
    assert_eq!(e.mem.u8(ACTOR + 0x105), 0x1f, "already on: not toggled off");
    assert_eq!(run(&mut e, TOGGLE_DETECTION_FIX, ACTOR, &[int_arg(-1)]), 1);
    assert_eq!(e.mem.u8(ACTOR + 0x105), 0x1f, "negative is on");
    assert_eq!(run(&mut e, TOGGLE_DETECTION_FIX, ACTOR, &[int_arg(0)]), 1);
    assert_eq!(e.mem.u8(ACTOR + 0x105), 0x0f);
    assert_eq!(run(&mut e, TOGGLE_DETECTION_FIX, ACTOR, &[int_arg(0)]), 1);
    assert_eq!(e.mem.u8(ACTOR + 0x105), 0x0f, "already off: not toggled on");
    assert_eq!(
        (e.mem.u8(ACTOR + 0x106), e.mem.u8(ACTOR + 0x107)),
        (0x01, 0x20)
    );
    // No argument (a compiled script always has it): read as 0, as the port reads every missing
    // number (JIP's variable would be left unset).
    e.mem.set_u8(ACTOR + 0x105, 0x1f);
    assert_eq!(run(&mut e, TOGGLE_DETECTION_FIX, ACTOR, &[]), 1);
    assert_eq!(e.mem.u8(ACTOR + 0x105), 0x0f);
    // Not an actor, or no reference: nothing.
    e.mem.set_u8(NOT_ACTOR + 0x105, 0);
    assert_eq!(
        run(&mut e, TOGGLE_DETECTION_FIX, NOT_ACTOR, &[int_arg(1)]),
        1
    );
    assert_eq!(e.mem.u8(NOT_ACTOR + 0x105), 0);
    assert_eq!(run(&mut e, TOGGLE_DETECTION_FIX, 0, &[int_arg(1)]), 1);
    // The flag then reaches both G53 handlers.
    assert_eq!(run(&mut e, TOGGLE_DETECTION_FIX, ACTOR, &[int_arg(1)]), 1);
    e.mem.set_u8(ACTOR + 0x18d, 1);
    assert_eq!(e.site(DETECTION_TEAMMATE[0].addr(), &[ACTOR]), Some(0));
    assert_eq!(
        e.site(DETECTION_LEVEL.addr(), &[ACTOR]),
        Some(PLAYER + 0xa4)
    );
}
