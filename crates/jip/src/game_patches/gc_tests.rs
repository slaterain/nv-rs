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
    // A variable the script does not have: the read fails and the flag is left as it was (a
    // read taken as 0 would clear it).
    e.mem.set_u8(ACTOR + 0x105, 0x1f);
    assert_eq!(
        run(&mut e, TOGGLE_DETECTION_FIX, ACTOR, &[vec![b'f', 9, 0]]),
        1
    );
    assert_eq!(e.mem.u8(ACTOR + 0x105), 0x1f, "failed read: unchanged");
    // The flag then reaches both G53 handlers.
    assert_eq!(run(&mut e, TOGGLE_DETECTION_FIX, ACTOR, &[int_arg(1)]), 1);
    e.mem.set_u8(ACTOR + 0x18d, 1);
    assert_eq!(e.site(DETECTION_TEAMMATE[0].addr(), &[ACTOR]), Some(0));
    assert_eq!(
        e.site(DETECTION_LEVEL.addr(), &[ACTOR]),
        Some(PLAYER + 0xa4)
    );
}

// ---- PR 2: the group's fixes, O19, G33.

const OTHER: u32 = AREA_BASE + 0x2000;
const BASE_FORM: u32 = AREA_BASE + 0x2400;
const PROCESS_AT: u32 = AREA_BASE + 0x2800;
const WEAPON: u32 = AREA_BASE + 0x3000;
const ENTRY: u32 = AREA_BASE + 0x3400;
const AMMO: u32 = AREA_BASE + 0x3800;
const AMMO_ENTRY: u32 = AREA_BASE + 0x3c00;
const SET: u32 = AREA_BASE + 0x4000;
const HIT: u32 = AREA_BASE + 0x4400;
const LOG: u32 = AREA_BASE + 0x5000;

fn fixes_world() -> Engine {
    let mut e = world_as(true);
    e.map(IMPACT_FALLBACK & !0xfff, 0x1000);
    e.map(DAMAGE_TO_WEAPON_VALUE & !0xfff, 0x1000);
    e
}

/// While JIP answers "not installed": none of the group's PR 2 seams has a handler, row or not.
#[test]
fn pr2_not_installed_nothing_is_put_in() {
    let mut e = world_as(false);
    super::super::set_row(&mut e, true);
    for addr in AREA.fix_sites.iter().chain(AREA.jip_sites) {
        assert!(!e.site_owners(*addr).contains(&Owner::Jip), "{addr:08x}");
        assert_eq!(e.site(*addr, &[ACTOR, 0]), None, "{addr:08x}");
    }
}

/// Installed, row off: G33 (JIP's own) is in, the fixes are not; the row on puts them in.
#[test]
fn pr2_fixes_only_with_the_row() {
    let mut e = world_as(true);
    super::super::set_row(&mut e, false);
    for slot in [PLAYER_KILLER_SET, PLAYER_KILLER_KEPT] {
        assert_eq!(e.site(slot.addr(), &[PLAYER]), Some(0));
    }
    for addr in AREA.fix_sites {
        assert_eq!(e.site(*addr, &[HIT, 0]), None, "{addr:08x}");
    }
    // The group's installer (what the row puts in; the framework's tests hold every area to the
    // row): every fix seam then has JIP's handler and both wraps are in.
    install_fixes(&mut e);
    for addr in AREA.fix_sites {
        assert_eq!(
            e.site_owners(*addr).to_vec(),
            vec![Owner::Jip],
            "{addr:08x}"
        );
    }
    for addr in AREA.fix_wraps {
        assert!(e.wraps_at(*addr).contains(&Owner::Jip), "{addr:08x}");
    }
}

/// The plain byte patches: G14's word 0, G18's mark `0x404`, G118 answers (the `jnz` gone),
/// G33 answers (JIP's jumps).
#[test]
fn byte_patches_answer_jips_bytes() {
    let mut e = fixes_world();
    assert_eq!(equip_no_sound(&mut e, &[]), Some(0));
    assert_eq!(life_state_mark(&mut e, &[ACTOR]), Some(0x404));
    assert!(projectile_weapon_first(&mut e, &[]).is_some());
    assert_eq!(player_killer(&mut e, &[PLAYER]), Some(0));
}

/// G77: bit 31 of the hit's flags set (`or byte [+0x5B], 0x80`), the other bits kept.
#[test]
fn g77_marks_the_hit_armor_penetrated() {
    let mut e = fixes_world();
    e.mem.set_u32(HIT + 0x58, 0x4512_3405);
    armor_penetrated(&mut e, &[HIT]);
    assert_eq!(e.mem.u32(HIT + 0x58), 0xc512_3405);
    armor_penetrated(&mut e, &[HIT]);
    assert_eq!(e.mem.u32(HIT + 0x58), 0xc512_3405);
}

/// G98: a teammate, an actor with `+0x143` bit 7, an essential base: 1; else 0; no base: 0.
#[test]
fn g98_skips_teammates_flagged_and_essential() {
    let mut e = fixes_world();
    e.mem.set_u32(OTHER + 0x20, BASE_FORM);
    assert_eq!(kill_challenge_skips(&mut e, &[OTHER]), Some(0));
    e.mem.set_u8(OTHER + 0x18d, 2);
    assert_eq!(kill_challenge_skips(&mut e, &[OTHER]), Some(1));
    e.mem.set_u8(OTHER + 0x18d, 0);
    e.mem.set_u8(OTHER + 0x143, 0x7f);
    assert_eq!(kill_challenge_skips(&mut e, &[OTHER]), Some(0));
    e.mem.set_u8(OTHER + 0x143, 0x80);
    assert_eq!(kill_challenge_skips(&mut e, &[OTHER]), Some(1));
    e.mem.set_u8(OTHER + 0x143, 0);
    e.mem.set_u8(BASE_FORM + 0x34, 0xfd);
    assert_eq!(kill_challenge_skips(&mut e, &[OTHER]), Some(0));
    e.mem.set_u8(BASE_FORM + 0x34, 2);
    assert_eq!(kill_challenge_skips(&mut e, &[OTHER]), Some(1));
    e.mem.set_u32(OTHER + 0x20, 0);
    assert_eq!(kill_challenge_skips(&mut e, &[OTHER]), Some(0));
}

/// O19: a creature reference 1; no process 0; a process not high 0; else the byte `+0x228`.
#[test]
fn o19_answers_as_jip() {
    let mut e = fixes_world();
    e.mem.set_u8(OTHER + 4, 0x3c);
    assert_eq!(creature_spread(&mut e, &[OTHER]), Some(1));
    e.mem.set_u8(OTHER + 4, 0x3b);
    assert_eq!(creature_spread(&mut e, &[OTHER]), Some(0));
    e.mem.set_u32(OTHER + 0x68, PROCESS_AT);
    e.mem.set_u8(PROCESS_AT + 0x228, 1);
    e.mem.set_u8(PROCESS_AT + 0x28, 1);
    assert_eq!(creature_spread(&mut e, &[OTHER]), Some(0));
    e.mem.set_u8(PROCESS_AT + 0x28, 0);
    assert_eq!(creature_spread(&mut e, &[OTHER]), Some(1));
    e.mem.set_u8(PROCESS_AT + 0x228, 0);
    assert_eq!(creature_spread(&mut e, &[OTHER]), Some(0));
}

/// G109: an empty slot answers none (the lookup not asked); no fallback set: the slot; the
/// lookup's answer, or the slot when it answers none. The wrap answers the same.
#[test]
fn g109_impact_data_as_jip() {
    let mut e = fixes_world();
    e.register(IMPACT_LOOKUP, |e, w| {
        e.mem.set_u32(LOG, w[0]);
        e.mem.set_u32(LOG + 4, w[1]);
        e.mem.set_u32(LOG + 8, w[2]);
        e.mem.u32(LOG + 12).into_ret()
    });
    assert_eq!(impact_data(&mut e, SET, 3), 0);
    e.mem.set_u32(SET + 0x1c + 12, 0x0d33);
    assert_eq!(impact_data(&mut e, SET, 3), 0x0d33);
    assert_eq!(e.mem.u32(LOG), 0, "no fallback: nothing asked");
    e.mem.set_u32(IMPACT_FALLBACK, 0x0f00);
    assert_eq!(impact_data(&mut e, SET, 3), 0x0d33);
    assert_eq!(
        [e.mem.u32(LOG), e.mem.u32(LOG + 4), e.mem.u32(LOG + 8)],
        [0x0f00, 3, 0x0d33]
    );
    e.mem.set_u32(LOG + 12, 0x0e44);
    assert_eq!(impact_data(&mut e, SET, 3), 0x0e44);
    assert_eq!(get_impact_data(&mut e, &[SET, 3]).eax, 0x0e44);
    e.mem.set_u32(LOG, 0);
    assert_eq!(impact_data(&mut e, SET, 4), 0);
    assert_eq!(e.mem.u32(LOG), 0, "empty slot: nothing asked");
}

/// G120: no weapon or no set on the weapon: 0; else the set's data for the material.
#[test]
fn g120_hit_blood_from_the_hits_weapon() {
    let mut e = fixes_world();
    e.mem.set_u32(SET + 0x1c + 8, 0x0d22);
    assert_eq!(hit_blood_impact(&mut e, &[HIT, 2]), Some(0));
    e.mem.set_u32(HIT + 0x30, WEAPON);
    assert_eq!(hit_blood_impact(&mut e, &[HIT, 2]), Some(0));
    e.mem.set_u32(WEAPON + 0x24c, SET);
    assert_eq!(hit_blood_impact(&mut e, &[HIT, 2]), Some(0x0d22));
    assert_eq!(hit_blood_impact(&mut e, &[HIT, 3]), Some(0));
}

fn weapon_damage(e: &mut Engine) -> f64 {
    get_damage_to_weapon(e, &[OTHER]).st0
}

/// G111 + O11: the setting without a fitting process or weapon; the weapon's own value with its
/// flag; O11's skill and `+0xA0` scaling only with its switch and a matched skill; ammo effects
/// for a weapon with ammo and a `TESAmmo` with effects.
#[test]
fn g111_damage_to_weapon_as_jip() {
    let mut e = fixes_world();
    e.mem.set_f32(DAMAGE_TO_WEAPON_VALUE, 1.5);
    assert_eq!(weapon_damage(&mut e), 1.5, "no process");
    e.mem.set_u32(OTHER + 0x68, PROCESS_AT);
    e.mem.set_u32(PROCESS_AT + 0x114, ENTRY);
    e.mem.set_u32(ENTRY + 8, WEAPON);
    e.mem.set_u8(PROCESS_AT + 0x28, 2);
    assert_eq!(weapon_damage(&mut e), 1.5, "low process");
    e.mem.set_u8(PROCESS_AT + 0x28, 1);
    e.mem.set_f32(WEAPON + 0x148, 4.0);
    assert_eq!(weapon_damage(&mut e), 1.5, "flag clear: the setting");
    e.mem.set_u8(WEAPON + 0x12c, 0x80);
    assert_eq!(weapon_damage(&mut e), 4.0, "own value");
    e.mem.set_u8(WEAPON + 0x15c, 0x29);
    e.mem.set_u16(WEAPON + 0xa0, 3);
    assert_eq!(weapon_damage(&mut e), 4.0, "O11 off");
    e.set_jip_patches(e.jip_patches() | jip_patches::FO3_WPN_DEGRADATION);
    let guns = 4.0f32 * f32::from_bits(0x3e19_999a) * 3.0;
    assert_eq!(weapon_damage(&mut e), f64::from(guns), "Guns 0.15, times 3");
    e.mem.set_u8(WEAPON + 0x15c, 0x2d);
    assert_eq!(weapon_damage(&mut e), 3.0, "Unarmed 0.25");
    e.mem.set_u8(WEAPON + 0x15c, 0x22);
    let energy = 4.0f32 * f32::from_bits(0x3e4c_cccd) * 3.0;
    assert_eq!(
        weapon_damage(&mut e),
        f64::from(energy),
        "Energy Weapons 0.2"
    );
    e.mem.set_u8(WEAPON + 0x15c, 0x24);
    assert_eq!(weapon_damage(&mut e), 4.0, "Lockpick: no match");
    e.mem.set_u8(WEAPON + 0x15c, 0x20);
    assert_eq!(weapon_damage(&mut e), 4.0, "below Big Guns");
    e.mem.set_u8(WEAPON + 0x15c, 0x2e);
    assert_eq!(weapon_damage(&mut e), 4.0, "past the table");
    e.mem.set_u8(WEAPON + 0x15c, 0x21);
    let big_guns = 4.0f32 * f32::from_bits(0x3e99_999a) * 3.0;
    assert_eq!(weapon_damage(&mut e), f64::from(big_guns), "Big Guns 0.3");

    e.register(APPLY_AMMO_EFFECTS, |e, w| {
        e.mem.set_u32(LOG, w[0]);
        e.mem.set_u32(LOG + 4, w[1]);
        e.mem.set_u32(LOG + 8, w[2]);
        9.25f64.into_ret()
    });
    e.mem.set_u32(PROCESS_AT + 0x118, AMMO_ENTRY);
    e.mem.set_u32(AMMO_ENTRY + 8, AMMO);
    e.mem.set_u32(AMMO, TES_AMMO_VTABLE);
    e.mem.set_u32(AMMO + 0xd4, 1);
    assert_eq!(
        weapon_damage(&mut e),
        f64::from(big_guns),
        "weapon without ammo"
    );
    e.mem.set_u32(WEAPON + 0xa8, 0x77);
    assert_eq!(weapon_damage(&mut e), 9.25, "ammo effects");
    assert_eq!(
        [e.mem.u32(LOG), e.mem.u32(LOG + 4), e.mem.u32(LOG + 8)],
        [4, AMMO + 0xd4, big_guns.to_bits()]
    );
    e.mem.set_u32(AMMO, TES_AMMO_VTABLE + 4);
    assert_eq!(weapon_damage(&mut e), f64::from(big_guns), "not TESAmmo");
    e.mem.set_u32(AMMO, TES_AMMO_VTABLE);
    e.mem.set_u32(AMMO + 0xd4, 0);
    assert_eq!(weapon_damage(&mut e), f64::from(big_guns), "no effects");
    e.mem.set_u32(AMMO + 0xd4, 1);
    e.mem.set_u32(AMMO_ENTRY + 8, 0);
    assert_eq!(weapon_damage(&mut e), f64::from(big_guns), "no ammo form");
}

/// G33's part of the housekeeping before a load: the player's killer cleared.
#[test]
fn g33_pre_load_clears_the_players_killer() {
    let mut e = fixes_world();
    e.mem.set_u32(PLAYER + 0xc0, ACTOR);
    pre_load(&mut e);
    assert_eq!(e.mem.u32(PLAYER + 0xc0), 0);
}
