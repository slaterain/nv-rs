//! Group GC part 3: O1 (JIP's hit damage) with O12, G116, G119. Expected values follow
//! `CalculateHitDamageHook`'s C++ (JIP LN NVSE 57.30) step by step in single floats; the game's
//! functions are stand-ins whose answers are chosen (not powers of two) so each step shows.

use super::*;
use crate::test_support::engine;

const B: u32 = 0x0560_0000;
const VT_ACTOR: u32 = B;
const VT_MAGIC: u32 = B + 0x400;
const VT_AV: u32 = B + 0x600;
const VT_PROJECTILE: u32 = B + 0x800;
const TARGET: u32 = B + 0x1000;
const SOURCE: u32 = B + 0x2000;
const PLAYER: u32 = B + 0x3000;
const HIT: u32 = B + 0x4000;
const WEAPON: u32 = B + 0x4400;
const PROJ: u32 = B + 0x4800;
const AMMO: u32 = B + 0x4c00;
const PROCESS: u32 = B + 0x5000;
const INFO: u32 = B + 0x5800;
const HELD: u32 = B + 0x5c00;
const ARMOR: u32 = B + 0x6000;
const BIPED: u32 = B + 0x6400;
const CHANGES: u32 = B + 0x6800;
const XLIST: u32 = B + 0x6c00;
const XHEALTH: u32 = B + 0x7000;
const FISTS: u32 = B + 0x7400;
/// Stand-ins' switches and their log.
const CANNOT: u32 = B + 0xf00;
const PARALYZED: u32 = B + 0xf04;
const BLOCKS: u32 = B + 0xf08;
const LOG: u32 = B + 0x8000;
const AMMO_KINDS: u32 = LOG + 0x100;
const PERKS: u32 = LOG + 0x200;
/// How often the exe's `ReduceDamage` (the wrap's next link) ran.
const NEXT_CALLS: u32 = LOG + 0x60;
/// Stand-in functions (vtable slots).
const CANNOT_FN: u32 = B + 0xfe00;
const PARALYZED_FN: u32 = B + 0xfe10;
const AV_FN: u32 = B + 0xfe20;

fn av(e: &mut Engine, actor: u32, code: u32, value: f32) {
    e.mem.set_f32(actor + 0x600 + code * 4, value);
}

fn count(e: &Engine, at: u32) -> u32 {
    e.mem.u32(at)
}

fn bump(e: &mut Engine, at: u32) -> u32 {
    let n = e.mem.u32(at);
    e.mem.set_u32(at, n + 1);
    n
}

/// The world: a target NPC hit by the player with a gun (type 3, no resistance), DR 25, DT 7.5,
/// health damage 37.3; the settings; the stand-ins: `ApplyAmmoEffects` = value x 1.25 + kind,
/// perk entry 0 x 1.3, 0x38 + 2.5, 0x3A + 1.5, the Charisma 6, the base DT 12 and DR 30.
fn world() -> Engine {
    world_as(false)
}

/// [`world`], with JIP installed and the row on first (its installers run before the stand-ins).
fn world_as(install: bool) -> Engine {
    let mut e = engine();
    // The exe's `ReduceDamage` as the wrap's next link: a marker (health damage 11.1, a count).
    e.register(REDUCE_DAMAGE.addr(), |e, a| {
        bump(e, NEXT_CALLS);
        e.mem.set_f32(a[0] + 0x14, 11.1);
        Ret::default()
    });
    if install {
        crate::install_as(&mut e, true);
        // The group's installer, what the row puts in (the row's settings read needs the ini).
        super::super::install_fixes(&mut e);
    }
    e.map(B, 0x1_0000);
    for page in [
        0x011d_1000,
        0x011d_0000,
        0x011c_e000,
        0x011c_f000,
        0x011d_e000,
        0x011c_a000,
        0x011f_2000,
    ] {
        if !e.mem.is_mapped(page) {
            e.map(page, 0x1000);
        }
    }
    e.mem.set_u32(VT_ACTOR + 0x100, 0x008d_0360);
    e.mem.set_u32(VT_ACTOR + 0x234, PARALYZED_FN);
    e.mem.set_u32(VT_MAGIC + 0x10, CANNOT_FN);
    e.mem.set_u32(VT_AV + 0x0c, AV_FN);
    e.mem.set_u32(VT_PROJECTILE + 0x224, 0x008d_0360);
    for actor in [TARGET, SOURCE, PLAYER] {
        e.mem.set_u32(actor, VT_ACTOR);
        e.mem.set_u8(actor + 4, 0x2a);
        e.mem.set_u32(actor + 0xc, 0x0100_0000 + actor);
        e.mem.set_u32(actor + 0x94, VT_MAGIC);
        e.mem.set_u32(actor + 0xa4, VT_AV);
    }
    e.mem.set_u32(PLAYER + 0xc, 0x14);
    e.mem.set_u32(SOURCE + 0xc, 0x14);
    e.mem.set_u32(0x011d_ea3c, PLAYER);
    e.mem.set_u32(0x011c_a278, FISTS);
    e.mem.set_f32(MAX_ARMOR_RATING, 85.0);
    e.mem.set_f32(MIN_DAM_MULTIPLIER, 0.2);
    e.mem.set_f32(BLOCK_SKILL_BASE, 20.0);
    e.mem.set_f32(BLOCK_SKILL_MULT, 0.37);
    e.mem.set_f32(COUNTER_ATTACK_TIMER_SETTING, 1.9);
    e.mem.set_f32(DAMAGE_TO_ARMOR_PERCENTAGE, 0.15);
    e.mem.set_f32(VATS_PLAYER_DAMAGE_MULT, 0.35);
    e.mem.set_f32(VATS_LIMB_DAMAGE_MULT, 1.7);
    av(&mut e, TARGET, 0x12, 25.0);
    av(&mut e, TARGET, 0x4c, 7.5);
    e.mem.set_u32(HIT, SOURCE);
    e.mem.set_u32(HIT + 4, TARGET);
    e.mem.set_u32(HIT + 0x10, 3);
    e.mem.set_f32(HIT + 0x14, 37.3);
    e.mem.set_u32(HIT + 0x30, WEAPON);
    e.mem.set_u8(WEAPON + 0xf4, 3);
    e.mem.set_u32(WEAPON + 0x16c, u32::MAX);
    e.mem.set_u32(FISTS + 0x16c, u32::MAX);
    e.register(CANNOT_FN, |e, _| u32::from(e.mem.u8(CANNOT)).into_ret());
    e.register(PARALYZED_FN, |e, _| {
        u32::from(e.mem.u8(PARALYZED)).into_ret()
    });
    e.register(AV_FN, |e, a| {
        f64::from(e.mem.f32(a[0] - 0xa4 + 0x600 + a[1] * 4)).into_ret()
    });
    e.register(APPLY_AMMO_EFFECTS, |e, a| {
        let n = bump(e, LOG + 0x20);
        e.mem.set_u32(AMMO_KINDS + n * 4, a[0]);
        assert_eq!(a[1], AMMO + 0xd4);
        f64::from(f32::from_bits(a[2]) * 1.25 + a[0] as f32).into_ret()
    });
    e.register(APPLY_PERK_MODIFIERS, |e, a| {
        let n = bump(e, LOG + 0x24);
        for (i, word) in a[..4].iter().enumerate() {
            e.mem.set_u32(PERKS + n * 16 + i as u32 * 4, *word);
        }
        let v = e.mem.f32(a[4]);
        let v = match a[0] {
            0x00 => v * 1.3,
            0x38 => v + 2.5,
            _ => v + 1.5,
        };
        e.mem.set_f32(a[4], v);
        Ret::default()
    });
    e.register(GET_THRESHOLDED_AV, |e, a| {
        e.mem.set_u32(LOG + 0x40, a[0]);
        e.mem.set_u32(LOG + 0x44, a[1]);
        6.0f64.into_ret()
    });
    e.register(CHARACTER_BASE_DT, |_, _| 12.0f64.into_ret());
    e.register(CHARACTER_BASE_DR, |_, _| 30.0f64.into_ret());
    e.register(BLOCK_TEST, |e, a| {
        e.mem.set_u32(LOG + 0x30, a[0]);
        e.mem.set_u32(LOG + 0x34, a[1]);
        e.mem.set_u32(LOG + 0x38, a[2]);
        u32::from(e.mem.u8(BLOCKS)).into_ret()
    });
    e.register(HUD_PLAYER_HIT, |e, a| {
        bump(e, LOG);
        e.mem.set_u32(LOG + 4, a[0]);
        Ret::default()
    });
    e.register(HUD_TARGET_HIT, |e, a| {
        bump(e, LOG + 8);
        e.mem.set_u32(LOG + 0xc, a[0]);
        Ret::default()
    });
    e.register(FOLLOWER_BARK, |e, a| {
        bump(e, LOG + 0x10);
        e.mem.set_u32(LOG + 0x14, a[0]);
        e.mem.set_u32(LOG + 0x18, a[1]);
        Ret::default()
    });
    e.register(0x0041_0220, |e, a| {
        e.mem.u32(a[0] + 0x200 + a[1] * 4).into_ret()
    });
    e.register(RUN_SCRIPT_EFFECT, |e, a| {
        bump(e, LOG + 0x50);
        e.mem.set_u32(LOG + 0x54, a[0]);
        e.mem.set_u32(LOG + 0x58, a[1]);
        e.mem.set_u32(LOG + 0x5c, a[2]);
        Ret::default()
    });
    e.set_jip_patches(jip_patches::INSTALLED | jip_patches::FIXES | jip_patches::IGNORE_DTDR_FIX);
    e
}

fn health(e: &Engine) -> f32 {
    e.mem.f32(HIT + 0x14)
}

/// The resistance factor JIP makes of a DR (`1 - min(DR, fMaxArmorRating) * 0.01`, DR <= 0: 1).
fn resist(dr: f32) -> f32 {
    let dr = if dr < 100.0 { dr } else { 100.0 };
    if dr <= 0.0 {
        1.0
    } else {
        1.0 - (if dr < 85.0 { dr } else { 85.0 }) * 0.01
    }
}

/// The player's gun on an NPC: perk entry 0 on the damage, DR, the attacker's DT perk, DT x DR
/// off the damage, above the minimum: the armour-penetrated flag, no marker, nothing else.
#[test]
fn o1_players_hit_on_an_npc() {
    let mut e = world();
    e.mem.set_u32(HIT + 0x58, 0x0000_0040);
    calculate_hit_damage(&mut e, HIT, 0);
    let dr = resist(25.0);
    let dt = (7.5f32 - 1.5) * dr;
    let expected = 37.3f32 * 1.3 * dr - dt;
    assert_eq!(health(&e).to_bits(), expected.to_bits());
    assert_eq!(e.mem.f32(HIT + 0x18), 37.3, "the base damage kept");
    assert_eq!(e.mem.u32(HIT + 0x58), 0x8000_0040);
    assert_eq!(vats_hit_dt(&mut e), 6.0);
    assert_eq!(count(&e, LOG + 0x24), 2);
    assert_eq!(
        [
            e.mem.u32(PERKS),
            e.mem.u32(PERKS + 4),
            e.mem.u32(PERKS + 8),
            e.mem.u32(PERKS + 12)
        ],
        [0x00, SOURCE, WEAPON, TARGET]
    );
    assert_eq!(
        [
            e.mem.u32(PERKS + 16),
            e.mem.u32(PERKS + 20),
            e.mem.u32(PERKS + 24),
            e.mem.u32(PERKS + 28)
        ],
        [0x3a, SOURCE, WEAPON, TARGET]
    );
    assert_eq!(
        count(&e, LOG) + count(&e, LOG + 8) + count(&e, LOG + 0x10),
        0
    );
    assert_eq!(e.mem.f32(HIT + 0x28), 0.0, "no armour damage for an NPC");
}

/// A target that is not an actor: nothing; one that cannot be hit: health, base and fatigue 0.
#[test]
fn o1_not_an_actor_or_cannot_be_hit() {
    let mut e = world();
    e.mem.set_u32(TARGET, VT_PROJECTILE);
    calculate_hit_damage(&mut e, HIT, 0);
    assert_eq!(health(&e), 37.3);
    e.mem.set_u32(TARGET, VT_ACTOR);
    e.mem.set_u8(CANNOT, 1);
    e.mem.set_f32(HIT + 0x18, 5.5);
    e.mem.set_f32(HIT + 0x1c, 2.5);
    calculate_hit_damage(&mut e, HIT, 0);
    assert_eq!(
        [health(&e), e.mem.f32(HIT + 0x18), e.mem.f32(HIT + 0x1c)],
        [0.0, 0.0, 0.0]
    );
    e.mem.set_u32(HIT + 4, 0);
    e.mem.set_f32(HIT + 0x14, 37.3);
    calculate_hit_damage(&mut e, HIT, 0);
    assert_eq!(health(&e), 37.3, "no target");
}

/// The player hit by a teammate: the defender's and the attacker's DT perks, damage at the
/// minimum (the player's marker with 0, the teammate's bark), the player's armour damage, and
/// in VATS every damage field times `fVATSPlayerDamageMult`.
#[test]
fn o1_player_hit_at_the_minimum_in_vats() {
    let mut e = world();
    e.mem.set_u32(HIT + 4, PLAYER);
    e.mem.set_u32(SOURCE + 0xc, 0x0100_2000);
    e.mem.set_u8(SOURCE + 0x18d, 1);
    av(&mut e, PLAYER, 0x12, 10.0);
    av(&mut e, PLAYER, 0x4c, 50.0);
    e.mem.set_f32(HIT + 0x14, 20.7);
    e.mem.set_f32(HIT + 0x20, 3.3);
    e.mem.set_f32(HIT + 0x2c, 1.7);
    e.mem.set_u32(VATS_MODE, 2);
    calculate_hit_damage(&mut e, HIT, 0);
    let dr = resist(10.0);
    let dt = (50.0f32 + 2.5 - 1.5) * dr;
    let minimum = 0.2f32 * 20.7;
    let armor = (minimum * (1.0 - dr) + if minimum < dt { minimum } else { dt }) * 0.15;
    let m = 0.35f32;
    assert_eq!(health(&e).to_bits(), (minimum * m).to_bits());
    assert_eq!(e.mem.f32(HIT + 0x18).to_bits(), (20.7f32 * m).to_bits());
    assert_eq!(e.mem.f32(HIT + 0x28).to_bits(), (armor * m).to_bits());
    assert_eq!(e.mem.f32(HIT + 0x20).to_bits(), (3.3f32 * m).to_bits());
    assert_eq!(e.mem.f32(HIT + 0x2c).to_bits(), (1.7f32 * m).to_bits());
    assert_eq!(e.mem.u32(HIT + 0x58) & 0x8000_0000, 0);
    assert_eq!((count(&e, LOG), e.mem.u32(LOG + 4)), (1, 0));
    assert_eq!(count(&e, LOG + 8), 0);
    assert_eq!(
        (
            count(&e, LOG + 0x10),
            e.mem.u32(LOG + 0x14),
            e.mem.u32(LOG + 0x18)
        ),
        (1, SOURCE, 9)
    );
    assert_eq!(e.mem.u32(PERKS + 16), 0x38, "the defender's entry");
    assert_eq!(
        [
            e.mem.u32(PERKS + 20),
            e.mem.u32(PERKS + 24),
            e.mem.u32(PERKS + 28)
        ],
        [PLAYER, SOURCE, WEAPON]
    );
}

/// A DT spent by the attacker's perk: no threshold left, the marker's word is "the target had
/// DT" (1); the target's marker only for a living target.
#[test]
fn o1_threshold_spent_shows_the_marker() {
    let mut e = world();
    av(&mut e, TARGET, 0x4c, 1.0);
    calculate_hit_damage(&mut e, HIT, 0);
    let dr = resist(25.0);
    assert_eq!(health(&e).to_bits(), (37.3f32 * 1.3 * dr).to_bits());
    assert_eq!((count(&e, LOG + 8), e.mem.u32(LOG + 0xc)), (1, 1));
    assert_eq!(vats_hit_dt(&mut e), -0.5);
    let mut e = world();
    av(&mut e, TARGET, 0x4c, 1.0);
    e.mem.set_u32(TARGET + 0x108, 2);
    calculate_hit_damage(&mut e, HIT, 0);
    assert_eq!(count(&e, LOG + 8), 0, "a dead target: no marker");
}

/// Ammo effects through the projectile's own ammo (G117's record): fatigue, DR, DT and damage in
/// that order; split beams divide the DT by the count; the weapon's resistance (40%) unless the
/// ammo ignores it; the fatigue follows the damage's share of the base.
#[test]
fn o1_ammo_split_beams_and_resistance() {
    let mut e = world();
    e.mem.set_u32(PROJ, VT_PROJECTILE);
    e.mem.set_u32(HIT + 8, PROJ);
    e.mem.set_u32(PROJ + 0x60, AMMO);
    e.mem.set_u8(PROJ + 0x149, 1);
    e.mem.set_u8(PROJ + 0x14a, 3);
    e.mem.set_u32(AMMO + 0xd4, 1);
    e.mem.set_f32(HIT + 0x1c, 4.4);
    e.mem.set_u32(WEAPON + 0x16c, 0x2b);
    av(&mut e, TARGET, 0x2b, 40.0);
    calculate_hit_damage(&mut e, HIT, 0);
    let fatigue = 4.4f32 * 1.25 + 5.0;
    let dr = resist(25.0f32 * 1.25 + 1.0);
    let dt = (7.5f32 * 1.25 + 2.0 - 1.5) / 3.0 * dr;
    let damage = (37.3f32 * 1.25 + 0.0) * 1.3 * dr - dt;
    let damage = damage * (1.0 - 40.0 * 0.01);
    assert_eq!(health(&e).to_bits(), damage.to_bits());
    assert_eq!(
        e.mem.f32(HIT + 0x1c).to_bits(),
        (fatigue * (damage / 37.3)).to_bits()
    );
    let kinds: Vec<u32> = (0..count(&e, LOG + 0x20))
        .map(|i| e.mem.u32(AMMO_KINDS + 4 * i))
        .collect();
    assert_eq!(kinds, [5, 1, 2, 0]);
    // The ammo ignores resistances (flag 1): no 40% off.
    let mut e2 = world();
    for (at, v) in [(PROJ, VT_PROJECTILE), (HIT + 8, PROJ), (PROJ + 0x60, AMMO)] {
        e2.mem.set_u32(at, v);
    }
    e2.mem.set_u8(PROJ + 0x14a, 1);
    e2.mem.set_u32(AMMO + 0xd4, 1);
    e2.mem.set_u8(AMMO + 0xac, 1);
    e2.mem.set_u32(WEAPON + 0x16c, 0x2b);
    av(&mut e2, TARGET, 0x2b, 40.0);
    calculate_hit_damage(&mut e2, HIT, 0);
    let dr = resist(25.0f32 * 1.25 + 1.0);
    let dt = (7.5f32 * 1.25 + 2.0 - 1.5) * dr;
    assert_eq!(
        health(&e2).to_bits(),
        ((37.3f32 * 1.25) * 1.3 * dr - dt).to_bits()
    );
    // No count recorded: no ammo (`GetAmmo`), no effects asked.
    let mut e3 = world();
    for (at, v) in [(PROJ, VT_PROJECTILE), (HIT + 8, PROJ), (PROJ + 0x60, AMMO)] {
        e3.mem.set_u32(at, v);
    }
    e3.mem.set_u32(AMMO + 0xd4, 1);
    calculate_hit_damage(&mut e3, HIT, 0);
    assert_eq!(count(&e3, LOG + 0x20), 0);
}

/// A resistance of 100 or more takes all the damage.
#[test]
fn o1_full_resistance() {
    let mut e = world();
    e.mem.set_u32(WEAPON + 0x16c, 0x2b);
    av(&mut e, TARGET, 0x2b, 100.0);
    calculate_hit_damage(&mut e, HIT, 0);
    assert_eq!(health(&e), 0.0);
}

/// An automatic melee weapon (flag 2, type <= 2) loses the share DT / attack damage (an attack
/// damage of 0 counts as 1); an NPC attacker without NPC perks gets no perk entries.
#[test]
fn o1_automatic_melee() {
    for (attack, divisor) in [(12u16, 12.0f32), (0, 1.0)] {
        let mut e = world();
        e.mem.set_u32(SOURCE + 0xc, 0x0100_2000);
        e.mem.set_u8(WEAPON + 0x100, 2);
        e.mem.set_u8(WEAPON + 0xf4, 1);
        e.mem.set_u16(WEAPON + 0xa0, attack);
        av(&mut e, TARGET, 0x4c, 0.3);
        av(&mut e, TARGET, 0x12, 10.0);
        calculate_hit_damage(&mut e, HIT, 0);
        let dr = resist(10.0);
        let share = 1.0 - 0.3f32 * dr / divisor;
        let expected = if share > 0.0 {
            37.3f32 * dr * share
        } else {
            37.3f32 * dr
        };
        assert_eq!(health(&e).to_bits(), expected.to_bits(), "{attack}");
        assert_eq!(count(&e, LOG + 0x24), 0, "no perks");
    }
}

/// Weapon flag 1 ignores DT and DR; no weapon: the fists (and their DT entries' filter).
#[test]
fn o1_weapon_ignoring_armor_and_fists() {
    let mut e = world();
    e.mem.set_u8(WEAPON + 0x100, 1);
    calculate_hit_damage(&mut e, HIT, 0);
    assert_eq!(health(&e).to_bits(), (37.3f32 * 1.3 + 1.5 * 0.0).to_bits());
    let mut e = world();
    e.mem.set_u32(HIT + 0x30, 0);
    calculate_hit_damage(&mut e, HIT, 0);
    assert_eq!(e.mem.u32(HIT + 0x30), FISTS);
    assert_eq!(
        e.mem.u32(PERKS + 8),
        FISTS,
        "the attacker's DT entry's weapon"
    );
    let dr = resist(25.0);
    assert_eq!(
        health(&e).to_bits(),
        (37.3f32 * dr - (7.5 - 1.5) * dr).to_bits()
    );
}

/// The player's teammate as target: DR and DT times `1 + Charisma x 0.05` (the player's owner).
#[test]
fn o1_teammate_target_takes_the_players_charisma() {
    let mut e = world();
    e.mem.set_u8(TARGET + 0x18d, 1);
    calculate_hit_damage(&mut e, HIT, 0);
    let m = 1.0f32 + 6.0 * 0.05;
    let dr = resist(25.0 * m);
    let dt = (7.5 * m + 2.5 - 1.5) * dr;
    assert_eq!(health(&e).to_bits(), (37.3f32 * 1.3 * dr - dt).to_bits());
    assert_eq!(
        (e.mem.u32(LOG + 0x40), e.mem.u32(LOG + 0x44)),
        (PLAYER + 0xa4, 8)
    );
}

/// NPC perks (`uNPCPerks`): an NPC attacker's and NPC target's perk entries.
#[test]
fn o1_npc_perks_switch() {
    let mut e = world();
    e.mem.set_u32(SOURCE + 0xc, 0x0100_2000);
    e.set_jip_patches(e.jip_patches() | jip_patches::NPC_PERKS);
    calculate_hit_damage(&mut e, HIT, 0);
    assert_eq!(count(&e, LOG + 0x24), 3);
    assert_eq!(
        [
            e.mem.u32(PERKS),
            e.mem.u32(PERKS + 16),
            e.mem.u32(PERKS + 32)
        ],
        [0x00, 0x38, 0x3a]
    );
}

/// A blocking high process: flag 1, the block DT bonus (base + mult x Melee Weapons 55 for a
/// held melee weapon), flag 2 for a drawn weapon; the block test gets the projectile or the
/// source; `noBlock` and a paralyzed target skip it; the player's counter-attack timer.
#[test]
fn o1_block() {
    let set_block = |e: &mut Engine| {
        e.mem.set_u32(TARGET + 0x68, PROCESS);
        e.mem.set_i16(PROCESS + 0x2ec, 7);
        e.mem.set_u8(BLOCKS, 1);
        e.mem.set_u32(PROCESS + 0x114, INFO);
        e.mem.set_u32(INFO + 8, HELD);
        e.mem.set_u8(HELD + 0xf4, 1);
        e.mem.set_u8(PROCESS + 0x135, 1);
        av(e, TARGET, 0x26, 55.0);
    };
    let mut e = world();
    set_block(&mut e);
    calculate_hit_damage(&mut e, HIT, 0);
    let bonus = 20.0f32 + 0.37 * 55.0;
    let dr = resist(25.0);
    let dt = (7.5 + bonus - 1.5) * dr;
    assert_eq!(e.mem.f32(HIT + 0x24).to_bits(), bonus.to_bits());
    let left = 37.3f32 * 1.3 * dr - dt;
    let minimum = 0.2f32 * 37.3;
    assert!(left <= minimum, "the block leaves the minimum");
    assert_eq!(health(&e).to_bits(), minimum.to_bits());
    assert_eq!(e.mem.u32(HIT + 0x58) & 3, 3);
    assert_eq!(
        [
            e.mem.u32(LOG + 0x30),
            e.mem.u32(LOG + 0x34),
            e.mem.u32(LOG + 0x38)
        ],
        [TARGET, SOURCE, 0]
    );
    // An empty hand against a melee hit: Unarmed; weapon not drawn: flag 2 cleared.
    let mut e = world();
    set_block(&mut e);
    e.mem.set_u32(PROCESS + 0x114, 0);
    e.mem.set_u8(WEAPON + 0xf4, 1);
    e.mem.set_u32(HIT + 0x58, 2);
    av(&mut e, TARGET, 0x2d, 33.0);
    calculate_hit_damage(&mut e, HIT, 0);
    assert_eq!(
        e.mem.f32(HIT + 0x24).to_bits(),
        (20.0f32 + 0.37 * 33.0).to_bits()
    );
    assert_eq!(e.mem.u32(HIT + 0x58) & 3, 1);
    // An empty hand against a ranged hit: no bonus, DT kept.
    let mut e = world();
    set_block(&mut e);
    e.mem.set_u32(PROCESS + 0x114, 0);
    e.mem.set_f32(HIT + 0x24, 9.5);
    calculate_hit_damage(&mut e, HIT, 0);
    assert_eq!(e.mem.f32(HIT + 0x24), 0.0);
    let dr = resist(25.0);
    assert_eq!(
        health(&e).to_bits(),
        (37.3f32 * 1.3 * dr - 6.0 * dr).to_bits()
    );
    // noBlock, a paralyzed target, a process not high, the block test failing: no block.
    for case in 0..4 {
        let mut e = world();
        set_block(&mut e);
        let no_block = u32::from(case == 0);
        match case {
            1 => e.mem.set_u8(PARALYZED, 1),
            2 => e.mem.set_u32(PROCESS + 0x28, 1),
            3 => e.mem.set_u8(BLOCKS, 0),
            _ => {}
        }
        calculate_hit_damage(&mut e, HIT, no_block);
        assert_eq!(e.mem.u32(HIT + 0x58) & 1, 0, "case {case}");
    }
    // The player blocking: the counter-attack timer.
    let mut e = world();
    e.mem.set_u32(HIT + 4, PLAYER);
    e.mem.set_u32(PLAYER + 0x68, PROCESS);
    e.mem.set_i16(PROCESS + 0x2ec, 7);
    e.mem.set_u8(BLOCKS, 1);
    calculate_hit_damage(&mut e, HIT, 0);
    assert_eq!(e.mem.f32(PLAYER + 0xe28), 1.9);
}

/// O12: the hit body part's armour (slot 2, equipped, condition 30 of 100 with the penalty 1):
/// its DT 6 and DR 0.4 x 0.8; for limb 1 taken off directly, for other limbs the character's
/// base DT 12 / DR 30 less those (at least 0) taken off.
#[test]
fn o12_localized_dt_dr() {
    let set = |e: &mut Engine, location: u32| {
        e.set_jip_patches(e.jip_patches() | jip_patches::LOCALIZED_DTDR);
        e.mem.set_u32(HIT + 0x10, location);
        e.mem.set_u32(TARGET + 0x1b4, BIPED);
        e.mem.set_u32(BIPED + 0x4c, ARMOR);
        e.mem.set_u32(ARMOR, VTBL_ARMOR);
        e.mem.set_f32(ARMOR + 0x17c, 6.0);
        e.mem.set_u16(ARMOR + 0x178, 40);
        e.mem.set_u32(ARMOR + 0x6c, 100);
        // The container changes extra (kind 0x15) -> data -> its entry for the armour.
        e.mem.set_u32(TARGET + 0x44 + 0x200 + 0x15 * 4, CHANGES);
        e.mem.set_u32(CHANGES + 0xc, CHANGES + 0x100);
        e.mem.set_u32(CHANGES + 0x100, CHANGES + 0x200);
        e.mem.set_u32(CHANGES + 0x200, CHANGES + 0x300);
        e.mem.set_u32(CHANGES + 0x308, ARMOR);
        e.mem.set_u32(CHANGES + 0x300, CHANGES + 0x380);
        e.mem.set_u32(CHANGES + 0x380, XLIST);
        e.mem.set_u8(XLIST + 8 + (0x16 >> 3), 1 << (0x16 & 7));
        e.mem.set_u32(XLIST + 0x200 + 0x25 * 4, XHEALTH);
        e.mem.set_f32(XHEALTH + 0xc, 30.0);
    };
    let m = 1.0f32 - (0.5 - 30.0f32 / 100.0) * 1.0;
    let (armor_dt, armor_dr) = (6.0f32 * m, 40.0f32 * 0.01 * m);
    let mut e = world();
    set(&mut e, 1);
    calculate_hit_damage(&mut e, HIT, 0);
    let dr = resist(25.0 - armor_dr);
    let dt = (7.5 - armor_dt - 1.5) * dr;
    assert_eq!(health(&e).to_bits(), (37.3f32 * 1.3 * dr - dt).to_bits());
    let mut e = world();
    set(&mut e, 3);
    calculate_hit_damage(&mut e, HIT, 0);
    let (loc_dt, loc_dr) = (12.0f32 - armor_dt, 30.0f32 - armor_dr);
    let dr = resist(25.0 - loc_dr);
    let dt = 7.5 - loc_dt - 1.5;
    let dt_left = if dt > 0.0 { dt * dr } else { 0.0 };
    assert_eq!(
        health(&e).to_bits(),
        (37.3f32 * 1.3 * dr - dt_left).to_bits()
    );
    // Without the switch, for a creature or limb 13: nothing taken off.
    for case in 0..3 {
        let mut e = world();
        set(&mut e, 3);
        match case {
            0 => e.set_jip_patches(e.jip_patches() & !jip_patches::LOCALIZED_DTDR),
            1 => e.mem.set_u8(TARGET + 4, 0x3c),
            _ => e.mem.set_u32(HIT + 0x10, 13),
        }
        calculate_hit_damage(&mut e, HIT, 0);
        let dr = resist(25.0);
        assert_eq!(
            health(&e).to_bits(),
            (37.3f32 * 1.3 * dr - 6.0 * dr).to_bits(),
            "case {case}"
        );
    }
}

/// O1's wrap at `009b5a30`: switch off, the next link (the exe's, a marker here: health 11.1,
/// once); on, JIP's damage with the call's word and the next link not run; a pending deferred
/// word takes the place of the call's.
#[test]
fn o1_wrap_by_its_switch_and_the_pending_word() {
    let mut e = world_as(true);
    e.set_jip_patches(jip_patches::INSTALLED | jip_patches::FIXES);
    e.mem.set_u32(TARGET + 0x68, PROCESS);
    e.mem.set_i16(PROCESS + 0x2ec, 7);
    e.mem.set_u8(BLOCKS, 1);
    e.call(REDUCE_DAMAGE.addr(), &[HIT, 0]);
    assert_eq!(
        (health(&e), count(&e, NEXT_CALLS)),
        (11.1, 1),
        "switch off: the exe's"
    );
    assert_eq!(e.mem.u32(HIT + 0x58), 0, "JIP's block not tried");
    e.mem.set_f32(HIT + 0x14, 37.3);
    e.set_jip_patches(jip_patches::INSTALLED | jip_patches::FIXES | jip_patches::IGNORE_DTDR_FIX);
    e.call(REDUCE_DAMAGE.addr(), &[HIT, 1]);
    assert_eq!(count(&e, NEXT_CALLS), 1, "switch on: the exe's not run");
    assert_eq!(e.mem.u32(HIT + 0x58) & 1, 0, "word 1: no block");
    let dr = resist(25.0);
    assert_eq!(
        health(&e).to_bits(),
        (37.3f32 * 1.3 * dr - 6.0 * dr).to_bits()
    );
    // Deferred at ov3 (word 0): the next call blocks although it passes 1.
    e.mem.set_f32(HIT + 0x14, 37.3);
    assert_eq!(e.site(DEFER_OV3.addr(), &[HIT, 0]), Some(0));
    assert_eq!(e.site(DEFER_OV4.addr(), &[HIT, 1]), None, "one pending");
    e.call(REDUCE_DAMAGE.addr(), &[HIT, 1]);
    assert_eq!(e.mem.u32(HIT + 0x58) & 1, 1, "the pending word 0");
    assert_eq!(
        e.site(DEFER_OV4.addr(), &[HIT, 1]),
        Some(0),
        "taken: deferred again"
    );
}

/// G116 at `0x9B6E81`: the deferred calculation runs (with the pending word); limb 0xE moves the
/// health damage to the limb damage (x the VATS limb factor with a command) and answers 1; other
/// limbs answer 0 with nothing moved.
#[test]
fn g116_runs_the_deferred_damage_and_limb_0xe() {
    let mut e = world_as(true);
    e.set_jip_patches(jip_patches::INSTALLED | jip_patches::FIXES | jip_patches::IGNORE_DTDR_FIX);
    assert_eq!(e.site(HIT_LOCATION.addr(), &[HIT]), Some(0));
    assert_eq!(health(&e), 37.3, "nothing pending");
    e.site(DEFER_OV4.addr(), &[HIT, 1]);
    assert_eq!(e.site(HIT_LOCATION.addr(), &[HIT]), Some(0));
    let dr = resist(25.0);
    assert_eq!(
        health(&e).to_bits(),
        (37.3f32 * 1.3 * dr - 6.0 * dr).to_bits()
    );
    assert_eq!(e.site(HIT_LOCATION.addr(), &[HIT]), Some(0), "ran once");
    e.mem.set_u32(HIT + 0x10, 0xe);
    e.mem.set_f32(HIT + 0x14, 8.9);
    assert_eq!(e.site(HIT_LOCATION.addr(), &[HIT]), Some(1));
    assert_eq!((health(&e), e.mem.f32(HIT + 0x20)), (0.0, 8.9));
    e.mem.set_f32(HIT + 0x14, 8.9);
    e.mem.set_u32(HIT + 0x54, 0x77);
    assert_eq!(e.site(HIT_LOCATION.addr(), &[HIT]), Some(1));
    assert_eq!(e.mem.f32(HIT + 0x20).to_bits(), (8.9f32 * 1.7).to_bits());
}

/// G119 at `0x89ABDC`: the projectile's recorded ammo's script runs on the actor hit (script,
/// actor, 0); none for no projectile, no ammo, not a `TESAmmo` or no script; always answers.
#[test]
fn g119_runs_the_projectiles_ammo_script() {
    let mut e = world_as(true);
    let site = AMMO_IMPACT_SCRIPT.addr();
    assert_eq!(e.site(site, &[HIT, TARGET]), Some(0));
    e.mem.set_u32(HIT + 8, PROJ);
    assert_eq!(e.site(site, &[HIT, TARGET]), Some(0));
    e.mem.set_u32(PROJ + 0x60, AMMO);
    e.mem.set_u32(AMMO + 0xa0, 0x0bad_5c00);
    assert_eq!(e.site(site, &[HIT, TARGET]), Some(0));
    assert_eq!(count(&e, LOG + 0x50), 0, "not a TESAmmo");
    e.mem.set_u32(AMMO, VTBL_AMMO);
    assert_eq!(e.site(site, &[HIT, TARGET]), Some(0));
    assert_eq!(
        [
            count(&e, LOG + 0x50),
            e.mem.u32(LOG + 0x54),
            e.mem.u32(LOG + 0x58),
            e.mem.u32(LOG + 0x5c)
        ],
        [1, 0x0bad_5c00, TARGET, 0]
    );
    e.mem.set_u32(AMMO + 0xa0, 0);
    assert_eq!(e.site(site, &[HIT, TARGET]), Some(0));
    assert_eq!(count(&e, LOG + 0x50), 1, "no script");
}

/// Not installed: the word stays 0 and none of part 3's seams or the wrap has JIP's handler (the
/// exe's `ReduceDamage` runs); installed with the row off: the word has no `FIXES` bit, and
/// neither. Started from a new engine, the word untouched.
#[test]
fn part3_only_with_the_row() {
    for (installed, row) in [(false, true), (true, false)] {
        let mut e = engine();
        e.register(REDUCE_DAMAGE.addr(), |e, _| {
            bump(e, NEXT_CALLS);
            Ret::default()
        });
        e.map(B, 0x1_0000);
        super::super::super::set_row(&mut e, row);
        crate::install_as(&mut e, installed);
        if installed {
            assert_eq!(e.jip_patches(), jip_patches::INSTALLED, "row off");
        } else {
            assert_eq!(e.jip_patches(), 0, "not installed");
        }
        for slot in [DEFER_OV3, DEFER_OV4, HIT_LOCATION, AMMO_IMPACT_SCRIPT] {
            assert_eq!(e.site(slot.addr(), &[HIT, 0]), None, "{installed} {row}");
        }
        assert!(e.wraps_at(REDUCE_DAMAGE.addr()).is_empty());
        e.call(REDUCE_DAMAGE.addr(), &[HIT, 0]);
        assert_eq!(count(&e, NEXT_CALLS), 1, "the exe's");
        assert_eq!(pending_word(&mut e), 0);
    }
}

/// The world with the target's high process blocking (Melee Weapons 55, Unarmed 33) and a held
/// weapon of type `held` drawn.
fn blocking(e: &mut Engine, held: u8) {
    e.mem.set_u32(TARGET + 0x68, PROCESS);
    e.mem.set_i16(PROCESS + 0x2ec, 7);
    e.mem.set_u8(BLOCKS, 1);
    e.mem.set_u32(PROCESS + 0x114, INFO);
    e.mem.set_u32(INFO + 8, HELD);
    e.mem.set_u8(HELD + 0xf4, held);
    e.mem.set_u8(PROCESS + 0x135, 1);
    av(e, TARGET, 0x26, 55.0);
    av(e, TARGET, 0x2d, 33.0);
}

/// `1 - min(23, 85) x 0.01` (`0x3F451EB8`): a factor whose bits are not short.
fn dr23() -> f32 {
    let dr = resist(23.0);
    assert_eq!(dr.to_bits(), 0x3f45_1eb8);
    dr
}

/// Split beams without the flag: a projectile of 7 pellets whose split-beam flag is clear keeps
/// its whole DT (JIP: `hasSplitBeams && numProjectiles > 1` divides it).
#[test]
fn o1_pellets_without_the_split_beam_flag_keep_the_whole_dt() {
    let mut e = world();
    av(&mut e, TARGET, 0x12, 23.0);
    e.mem.set_u32(PROJ, VT_PROJECTILE);
    e.mem.set_u32(HIT + 8, PROJ);
    e.mem.set_u8(PROJ + 0x14a, 7);
    calculate_hit_damage(&mut e, HIT, 0);
    let dr = dr23();
    let expected = 37.3f32 * 1.3 * dr - 6.0 * dr;
    assert_eq!(expected.to_bits(), 0x4202_de83);
    assert_eq!(health(&e).to_bits(), expected.to_bits());
    assert_eq!(count(&e, LOG + 0x20), 0, "no ammo recorded: no effects");
}

/// An automatic gun (flag 2, type 5, attack 12) takes the DT off as any gun: JIP's share rule is
/// for automatic weapons of type 2 or less.
#[test]
fn o1_automatic_gun_takes_the_dt_off() {
    let mut e = world();
    av(&mut e, TARGET, 0x12, 23.0);
    e.mem.set_u8(WEAPON + 0x100, 2);
    e.mem.set_u8(WEAPON + 0xf4, 5);
    e.mem.set_u16(WEAPON + 0xa0, 12);
    calculate_hit_damage(&mut e, HIT, 0);
    let dr = dr23();
    assert_eq!(health(&e).to_bits(), 0x4202_de83);
    assert_eq!(
        health(&e).to_bits(),
        (37.3f32 * 1.3 * dr - 6.0 * dr).to_bits()
    );
}

/// VATS (`mode` 2) scales only a hit on the player: the player's hit on an NPC is as outside VATS.
#[test]
fn o1_vats_scales_only_hits_on_the_player() {
    let mut e = world();
    e.mem.set_u32(VATS_MODE, 2);
    e.mem.set_f32(HIT + 0x1c, 2.5);
    e.mem.set_f32(HIT + 0x2c, 1.7);
    calculate_hit_damage(&mut e, HIT, 0);
    let dr = resist(25.0);
    let expected = 37.3f32 * 1.3 * dr - 6.0 * dr;
    assert_eq!(health(&e).to_bits(), expected.to_bits());
    assert_eq!(e.mem.f32(HIT + 0x18), 37.3, "the base damage, not x 0.35");
    assert_eq!(e.mem.f32(HIT + 0x2c), 1.7, "the weapon damage, not x 0.35");
}

/// A DR above `fMaxArmorRating` (93 against 85) counts as 85; with `fMinDamMultiplier` 0.1 the
/// damage stays above the minimum.
#[test]
fn o1_dr_above_the_armour_rating_cap() {
    let mut e = world();
    av(&mut e, TARGET, 0x12, 93.0);
    e.mem.set_f32(MIN_DAM_MULTIPLIER, 0.1);
    calculate_hit_damage(&mut e, HIT, 0);
    let dr = 1.0f32 - 85.0 * 0.01;
    let expected = 37.3f32 * 1.3 * dr - 6.0 * dr;
    assert_eq!(expected.to_bits(), 0x40cb_f3b9);
    assert!(expected > 0.1f32 * 37.3);
    assert_eq!(health(&e).to_bits(), expected.to_bits());
}

/// An automatic melee weapon whose share `1 - DT x DR / attack` is 0 or less (DT 7.5, DR 23,
/// attack 3: -0.925) leaves the damage after DR as it is.
#[test]
fn o1_automatic_melee_share_at_or_below_zero() {
    let mut e = world();
    e.mem.set_u32(SOURCE + 0xc, 0x0100_2000);
    av(&mut e, TARGET, 0x12, 23.0);
    e.mem.set_u8(WEAPON + 0x100, 2);
    e.mem.set_u8(WEAPON + 0xf4, 1);
    e.mem.set_u16(WEAPON + 0xa0, 3);
    calculate_hit_damage(&mut e, HIT, 0);
    let dr = dr23();
    assert!(1.0 - 7.5 * dr / 3.0 < 0.0);
    let expected = 37.3f32 * dr;
    assert_eq!(expected.to_bits(), 0x41e5_c49b);
    assert!(expected > 0.2f32 * 37.3);
    assert_eq!(health(&e).to_bits(), expected.to_bits());
}

/// The player hit while taking fatigue damage (4.4): no armour damage; the fatigue follows the
/// health damage's share of the base.
#[test]
fn o1_players_armour_damage_only_without_fatigue_damage() {
    let mut e = world();
    e.mem.set_u32(HIT + 4, PLAYER);
    e.mem.set_u32(SOURCE + 0xc, 0x0100_2000);
    av(&mut e, PLAYER, 0x12, 10.0);
    av(&mut e, PLAYER, 0x4c, 5.5);
    e.mem.set_f32(HIT + 0x1c, 4.4);
    calculate_hit_damage(&mut e, HIT, 0);
    let dr = resist(10.0);
    let expected = 37.3f32 * dr - (5.5 + 2.5) * dr;
    assert_eq!(health(&e).to_bits(), expected.to_bits());
    assert_eq!(e.mem.u32(HIT + 0x28), 0, "no armour damage");
    assert_eq!(
        e.mem.f32(HIT + 0x1c).to_bits(),
        (4.4f32 * (expected / 37.3)).to_bits()
    );
}

/// A block while holding a gun (type 5): the base bonus alone (no skill); holding a type-0 weapon:
/// Unarmed; an NPC blocking leaves the player's counter-attack timer alone.
#[test]
fn o1_block_skill_by_the_held_weapon() {
    let mut e = world();
    blocking(&mut e, 5);
    calculate_hit_damage(&mut e, HIT, 0);
    assert_eq!(e.mem.f32(HIT + 0x24), 20.0, "a gun: no skill");
    assert_eq!(e.mem.u32(HIT + 0x58) & 3, 3);
    assert_eq!(e.mem.u32(PLAYER + 0xe28), 0, "an NPC blocked: no timer");
    let mut e = world();
    blocking(&mut e, 0);
    calculate_hit_damage(&mut e, HIT, 0);
    assert_eq!(
        e.mem.f32(HIT + 0x24).to_bits(),
        (20.0f32 + 0.37 * 33.0).to_bits(),
        "type 0: Unarmed"
    );
}

/// O12's armour in biped slot 2 (DT 6, rating 40), equipped, at `condition` of 100.
fn o12_armour(e: &mut Engine, location: u32, condition: f32) {
    e.set_jip_patches(e.jip_patches() | jip_patches::LOCALIZED_DTDR);
    e.mem.set_u32(HIT + 0x10, location);
    e.mem.set_u32(TARGET + 0x1b4, BIPED);
    e.mem.set_u32(BIPED + 0x4c, ARMOR);
    e.mem.set_u32(ARMOR, VTBL_ARMOR);
    e.mem.set_f32(ARMOR + 0x17c, 6.0);
    e.mem.set_u16(ARMOR + 0x178, 40);
    e.mem.set_u32(ARMOR + 0x6c, 100);
    e.mem.set_u32(TARGET + 0x44 + 0x200 + 0x15 * 4, CHANGES);
    e.mem.set_u32(CHANGES + 0xc, CHANGES + 0x100);
    e.mem.set_u32(CHANGES + 0x100, CHANGES + 0x200);
    e.mem.set_u32(CHANGES + 0x200, CHANGES + 0x300);
    e.mem.set_u32(CHANGES + 0x308, ARMOR);
    e.mem.set_u32(CHANGES + 0x300, CHANGES + 0x380);
    e.mem.set_u32(CHANGES + 0x380, XLIST);
    e.mem.set_u8(XLIST + 8 + (0x16 >> 3), 1 << (0x16 & 7));
    e.mem.set_u32(XLIST + 0x200 + 0x25 * 4, XHEALTH);
    e.mem.set_f32(XHEALTH + 0xc, condition);
}

/// The health damage for a limb-1 hit with the armour's DT and DR taken off the target's 7.5 / 25.
fn limb1_damage(armor_dt: f32, armor_dr: f32) -> f32 {
    let dr = resist(25.0 - armor_dr);
    37.3f32 * 1.3 * dr - (7.5 - armor_dt - 1.5) * dr
}

/// O12 limb 1: the condition penalty (0.6) scales the armour below half condition (30 of 100:
/// `1 - 0.2 x 0.6`); at half (50) and above (70) the armour's whole DT 6 and DR 0.4 are taken
/// off, and with no `ExtraHealth` too.
#[test]
fn o12_condition_and_its_penalty() {
    let mut e = world();
    set_condition_penalty(&mut e, 0.6);
    o12_armour(&mut e, 1, 30.0);
    calculate_hit_damage(&mut e, HIT, 0);
    let m = 1.0f32 - (0.5 - 30.0f32 / 100.0) * 0.6;
    let expected = limb1_damage(6.0 * m, 40.0f32 * 0.01 * m);
    assert_ne!(expected, limb1_damage(6.0 * 0.8, 0.4 * 0.8));
    assert_eq!(health(&e).to_bits(), expected.to_bits());
    for condition in [50.0, 70.0] {
        let mut e = world();
        o12_armour(&mut e, 1, condition);
        calculate_hit_damage(&mut e, HIT, 0);
        assert_eq!(
            health(&e).to_bits(),
            limb1_damage(6.0, 40.0f32 * 0.01).to_bits(),
            "{condition}"
        );
    }
    let mut e = world();
    o12_armour(&mut e, 1, 30.0);
    e.mem.set_u32(XLIST + 0x200 + 0x25 * 4, 0);
    calculate_hit_damage(&mut e, HIT, 0);
    assert_eq!(
        health(&e).to_bits(),
        limb1_damage(6.0, 40.0f32 * 0.01).to_bits(),
        "no ExtraHealth: full values"
    );
}

/// O12 limbs other than 1 (3 and 0): the character's base DT 12 / DR 30 less the armour's (at 30
/// of 100: DT 4.8, DR 0.32) are taken off a target of DT 13.3 and DR 41 (DT 6.1, DR 11.32 left);
/// an armour above the base (DT 15, rating 3500) leaves 0 to take off (the target's whole DT 7.5
/// and DR 25); limb -1, an armour that is not a `TESObjectARMO` or one not equipped: nothing.
#[test]
fn o12_other_limbs_the_clamp_and_no_armour() {
    let m = 1.0f32 - (0.5 - 30.0f32 / 100.0) * 1.0;
    let (loc_dt, loc_dr) = (12.0f32 - 6.0 * m, 30.0f32 - 40.0f32 * 0.01 * m);
    assert!(13.3 - loc_dt - 1.5 > 0.0 && 41.0 - loc_dr > 0.0);
    let dr = resist(41.0 - loc_dr);
    let expected = 37.3f32 * 1.3 * dr - (13.3 - loc_dt - 1.5) * dr;
    for limb in [3, 0] {
        let mut e = world();
        av(&mut e, TARGET, 0x12, 41.0);
        av(&mut e, TARGET, 0x4c, 13.3);
        o12_armour(&mut e, limb, 30.0);
        calculate_hit_damage(&mut e, HIT, 0);
        assert_eq!(health(&e).to_bits(), expected.to_bits(), "limb {limb}");
    }
    let dr = resist(25.0);
    let whole = 37.3f32 * 1.3 * dr - 6.0 * dr;
    let mut e = world();
    o12_armour(&mut e, 3, 100.0);
    e.mem.set_f32(ARMOR + 0x17c, 15.0);
    e.mem.set_u16(ARMOR + 0x178, 3500);
    calculate_hit_damage(&mut e, HIT, 0);
    assert_eq!(health(&e).to_bits(), whole.to_bits(), "clamped at 0");
    for case in 0..3 {
        let mut e = world();
        o12_armour(&mut e, 1, 30.0);
        match case {
            0 => e.mem.set_u32(HIT + 0x10, u32::MAX),
            1 => e.mem.set_u32(ARMOR, VT_PROJECTILE),
            _ => e.mem.set_u8(XLIST + 8 + (0x16 >> 3), 0),
        }
        calculate_hit_damage(&mut e, HIT, 0);
        assert_eq!(health(&e).to_bits(), whole.to_bits(), "case {case}");
    }
}

/// G116 runs the pending calculation with the pending word: deferred at `_ov3` (word 0) the
/// block is tried (flag 1, the bonus); deferred at `_ov4` (word 1) it is not.
#[test]
fn g116_runs_the_pending_word() {
    let mut e = world_as(true);
    blocking(&mut e, 1);
    assert_eq!(e.site(DEFER_OV3.addr(), &[HIT, 0]), Some(0));
    assert_eq!(e.site(HIT_LOCATION.addr(), &[HIT]), Some(0));
    assert_eq!(e.mem.u32(HIT + 0x58) & 1, 1, "word 0: blocked");
    assert_eq!(
        e.mem.f32(HIT + 0x24).to_bits(),
        (20.0f32 + 0.37 * 55.0).to_bits()
    );
    let mut e = world_as(true);
    blocking(&mut e, 1);
    assert_eq!(e.site(DEFER_OV4.addr(), &[HIT, 1]), Some(0));
    assert_eq!(e.site(HIT_LOCATION.addr(), &[HIT]), Some(0));
    assert_eq!(e.mem.u32(HIT + 0x58) & 1, 0, "word 1: not blocked");
    assert_ne!(health(&e), 37.3, "it ran");
}

/// G116 with something pending and limb 0xE: the calculation runs first, so the limb damage is
/// the reduced health damage and the health damage 0.
#[test]
fn g116_pending_damage_before_the_limb_0xe_move() {
    let mut e = world_as(true);
    e.mem.set_u32(HIT + 0x10, 0xe);
    assert_eq!(e.site(DEFER_OV4.addr(), &[HIT, 1]), Some(0));
    assert_eq!(e.site(HIT_LOCATION.addr(), &[HIT]), Some(1));
    let dr = resist(25.0);
    assert_eq!(
        e.mem.u32(HIT + 0x20),
        (37.3f32 * 1.3 * dr - 6.0 * dr).to_bits()
    );
    assert_eq!(health(&e), 0.0);
    assert_eq!(pending_word(&mut e), 0);
}

/// A word left pending (a `SetTargetedLimb` that left before its limb step) is JIP's on one
/// thread: the next deferral site does not defer, and its call takes the stale word (0: the block
/// tried although the call passes 1), once.
#[test]
fn o1_a_word_left_pending_is_taken_by_the_next_call() {
    let mut e = world_as(true);
    blocking(&mut e, 1);
    assert_eq!(e.site(DEFER_OV3.addr(), &[HIT, 0]), Some(0));
    assert_eq!(pending_word(&mut e), 1);
    assert_eq!(e.site(DEFER_OV4.addr(), &[HIT, 1]), None, "not deferred");
    assert_eq!(pending_word(&mut e), 1, "a deferral site does not take it");
    e.call(REDUCE_DAMAGE.addr(), &[HIT, 1]);
    assert_eq!(e.mem.u32(HIT + 0x58) & 1, 1, "the stale word 0");
    assert_eq!(pending_word(&mut e), 0, "taken once");
    e.mem.set_u32(HIT + 0x58, 0);
    e.mem.set_f32(HIT + 0x14, 37.3);
    assert_eq!(e.site(HIT_LOCATION.addr(), &[HIT]), Some(0));
    assert_eq!(health(&e), 37.3, "nothing left for G116");
}

/// The pending word is the run's: another run's engine on this thread (its own slots, so its own
/// run id) neither takes nor overwrites it.
#[test]
fn o1_pending_word_is_the_runs() {
    let mut e = world_as(true);
    blocking(&mut e, 1);
    let mut other = world_as(true);
    blocking(&mut other, 1);
    assert_eq!(e.site(DEFER_OV3.addr(), &[HIT, 0]), Some(0));
    other.call(REDUCE_DAMAGE.addr(), &[HIT, 1]);
    assert_eq!(other.mem.u32(HIT + 0x58) & 1, 0, "its own word 1");
    assert_eq!(other.site(DEFER_OV4.addr(), &[HIT, 1]), Some(0));
    assert_eq!((pending_word(&mut e), pending_word(&mut other)), (1, 2));
    assert_eq!(e.site(HIT_LOCATION.addr(), &[HIT]), Some(0));
    assert_eq!(e.mem.u32(HIT + 0x58) & 1, 1, "word 0 kept for this run");
    assert_eq!((pending_word(&mut e), pending_word(&mut other)), (0, 2));
}

/// The engines of one run share the run's slots and so its word (a real thread's engine and the
/// main thread's, on one OS thread here): what one defers the other's G116 takes.
#[test]
fn o1_pending_word_is_shared_by_a_runs_engines() {
    let mut e = world_as(true);
    blocking(&mut e, 1);
    let mut other = world_as(false);
    e.share_slots_for_test(&mut other);
    assert_eq!(e.site(DEFER_OV3.addr(), &[HIT, 0]), Some(0));
    assert_eq!(pending_word(&mut other), 1);
}

/// A run's engine dropped with a word pending leaves it to no later run on this thread, even one
/// whose kernel has the dropped one's address (the run id is a process-wide count, never used
/// again): each new engine's G116 runs with its own word.
#[test]
fn o1_a_dropped_runs_word_is_not_the_next_runs() {
    for _ in 0..8 {
        let mut e = world_as(true);
        blocking(&mut e, 1);
        assert_eq!(e.site(DEFER_OV3.addr(), &[HIT, 0]), Some(0));
        assert_eq!(pending_word(&mut e), 1);
        let kernel = e.kernel.id();
        drop(e);
        let mut next = world_as(true);
        assert_eq!(
            pending_word(&mut next),
            0,
            "kernel reused: {}",
            next.kernel.id() == kernel
        );
    }
}

/// The test engine moved to a second OS thread behind a mutex, as stepped threads hand one engine
/// between their OS threads (`Engine` is not `Send`: a real thread's engine is one OS thread's).
struct Stepped(Engine);
// SAFETY: reached only through the `Mutex` below, by one thread at a time (the stepped threads'
// hand-over), and nothing of it is kept outside the lock.
unsafe impl Send for Stepped {}

/// The pending word is the thread's: deferred on this thread, a hit on another OS thread (the same
/// engine, as a stepped thread runs it; the same run, as a real thread's engine is) works out its
/// own word and defers its own, and this thread's G116 then runs with word 0.
#[test]
fn o1_pending_word_is_the_threads() {
    let mut e = world_as(true);
    blocking(&mut e, 1);
    assert_eq!(e.site(DEFER_OV3.addr(), &[HIT, 0]), Some(0));
    let engine = std::sync::Mutex::new(Stepped(e));
    let lock = || {
        engine
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    };
    let seen = std::thread::scope(|scope| {
        scope
            .spawn(|| {
                let mut g = lock();
                let e = &mut g.0;
                e.call(REDUCE_DAMAGE.addr(), &[HIT, 1]);
                let blocked = e.mem.u32(HIT + 0x58) & 1;
                let deferred = e.site(DEFER_OV4.addr(), &[HIT, 1]);
                let mine = pending_word(e);
                e.mem.set_u32(HIT + 0x58, 0);
                e.site(HIT_LOCATION.addr(), &[HIT]);
                (blocked, deferred, mine, e.mem.u32(HIT + 0x58) & 1)
            })
            .join()
            .expect("the other thread")
    });
    assert_eq!(seen, (0, Some(0), 2, 0), "the other thread's own word 1");
    let mut g = lock();
    let e = &mut g.0;
    assert_eq!(pending_word(e), 1, "this thread's kept");
    e.mem.set_u32(HIT + 0x58, 0);
    assert_eq!(e.site(HIT_LOCATION.addr(), &[HIT]), Some(0));
    assert_eq!(e.mem.u32(HIT + 0x58) & 1, 1, "word 0");
    assert_eq!(pending_word(e), 0);
}
