//! JIP LN NVSE's game patches of group GC (step 3.9) in this file: G98's, G33's and G18's seams.

use super::*;
use engine_core::extend::{jip_patches, Owner, SiteSlot};

const BASE: u32 = 0x0530_0000;
const ACTOR: Ptr<Actor> = Ptr::new(BASE + 0x100);
/// Where the test handlers write the actor they were asked with, one word per seam.
const ASKED: u32 = BASE + 0x800;
/// How often the exe's teammate test ran.
const TEAMMATE_CALLS: u32 = BASE + 0x900;

fn world() -> Engine {
    let mut e = Engine::new();
    e.map(BASE, 0x1000);
    e.register(PROCESS_GET_FORCE_NEXT_UPDATE, |e, _| {
        let n = e.mem.u32(TEAMMATE_CALLS);
        e.mem.set_u32(TEAMMATE_CALLS, n + 1);
        1u32.into_ret()
    });
    e.on_site(SiteSlot::new(Owner::Jip, 0x0089_dc0e), |e, a| {
        e.mem.set_u32(ASKED, a[0]);
        Some(0x100)
    });
    e.on_site(SiteSlot::new(Owner::Jip, 0x0089_e790), |e, a| {
        e.mem.set_u32(ASKED + 4, a[0]);
        Some(0)
    });
    e.on_site(SiteSlot::new(Owner::Jip, 0x0089_f351), |e, a| {
        e.mem.set_u32(ASKED + 8, a[0]);
        Some(0)
    });
    e.on_site(SiteSlot::new(Owner::Jip, 0x008a_19cf), |e, a| {
        e.mem.set_u32(ASKED + 12, a[0]);
        Some(0x404)
    });
    e
}

fn nothing_asked(e: &Engine) -> bool {
    (0..4).all(|i| e.mem.u32(ASKED + 4 * i) == 0)
}

/// With JIP not installed (word 0) or installed with the row off (`INSTALLED` only), the fixes'
/// seams (G98, G18) ask nothing: the exe's teammate call runs, the mark is `0x400`. G33 (JIP's
/// own) asks nothing with the word 0 or with only other bits: both jumps as the exe's.
#[test]
fn gc_seams_do_nothing_while_off() {
    for word in [0, jip_patches::INSTALLED] {
        let mut e = world();
        e.set_jip_patches(word);
        assert!(kill_challenge_teammate(&mut e, ACTOR), "{word:08x}");
        assert_eq!(e.mem.u32(TEAMMATE_CALLS), 1);
        assert_eq!(life_state_mark(&mut e, ACTOR), 0x400);
        if word == 0 {
            assert!(!player_killer_patched(&mut e, 0x0089_e790, ACTOR));
            assert!(!player_killer_patched(&mut e, 0x0089_f351, ACTOR));
        }
        assert!(nothing_asked(&e), "{word:08x}");
    }
    let mut e = world();
    e.set_jip_patches(jip_patches::FIXES | jip_patches::CREATURE_SPREAD_FIX);
    assert!(!player_killer_patched(&mut e, 0x0089_e790, ACTOR));
    assert!(!player_killer_patched(&mut e, 0x0089_f351, ACTOR));
    assert_eq!(e.mem.u32(ASKED + 4) | e.mem.u32(ASKED + 8), 0);
}

/// With the row on: G98's answer's low byte is the test (`0x100` is false; no exe call), G18's
/// answer is the mark; each asked with the actor. With `INSTALLED`: both G33 seams asked with the
/// actor, each at its own address.
#[test]
fn gc_seams_take_the_answers_when_on() {
    let mut e = world();
    e.set_jip_patches(jip_patches::FIXES);
    assert!(!kill_challenge_teammate(&mut e, ACTOR));
    assert_eq!(e.mem.u32(TEAMMATE_CALLS), 0);
    assert_eq!(e.mem.u32(ASKED), ACTOR.addr());
    assert_eq!(life_state_mark(&mut e, ACTOR), 0x404);
    assert_eq!(e.mem.u32(ASKED + 12), ACTOR.addr());
    e.set_jip_patches(jip_patches::INSTALLED);
    assert!(player_killer_patched(&mut e, 0x0089_e790, ACTOR));
    assert_eq!(e.mem.u32(ASKED + 4), ACTOR.addr());
    assert_eq!(e.mem.u32(ASKED + 8), 0);
    assert!(player_killer_patched(&mut e, 0x0089_f351, ACTOR));
    assert_eq!(e.mem.u32(ASKED + 8), ACTOR.addr());
}

// ---- Part 3: G119's seam in `Actor::HitMe` (the ammo's script effect).

const G119_HIT: Ptr<HitData> = Ptr::new(BASE + 0x200);
const G119_WEAPON: u32 = BASE + 0x300;
/// The weapon's current ammo (the exe's) and the ammo G117 recorded in the projectile (JIP's).
const G119_AMMO: u32 = BASE + 0x400;
const G119_PROJECTILE: u32 = BASE + 0x500;
/// The ammo's script (`TESScriptableForm::script`, `+0xA0`: the exe's effect and JIP's script).
const G119_SCRIPT: u32 = 0x0bad_5c00;
/// Where the logging handler writes its words.
const G119_ASKED: u32 = BASE + 0xa00;

/// A hit with `weapon` (no aggressor) on [`ACTOR`] whose projectile carries an ammo with a
/// script, the exe's calls as stand-ins, with JIP LN NVSE installed and its row on (`jip`) or a
/// logging handler at `0x89ABDC` (answers 0); the word `word` unless `jip`.
fn g119_world(word: u32, jip: bool, weapon: u32) -> Engine {
    let mut e = Engine::new();
    e.map(BASE, 0x1000);
    // The default weapon form (`0x11CA278`), which the branch without a weapon reads.
    e.map(0x011c_a000, 0x1000);
    e.register(REFR_EXTRA_DATA_LIST, |_, _| 0x77u32.into_ret());
    e.register(SCRIPT_SET_ACTION_FLAG, |_, _| Ret::default());
    e.register(WEAPON_GET_CURRENT_AMMO, |_, _| G119_AMMO.into_ret());
    e.register(0x0072_6070, |e, a| {
        e.mem.u32(a[0].wrapping_add(4)).into_ret()
    });
    e.register(SCRIPT_RUN_EFFECT_START, |_, _| Ret::default());
    e.mem.set_u32(G119_AMMO, 0x0102_6064);
    e.mem.set_u32(G119_AMMO + 0xa0, G119_SCRIPT);
    e.mem.set_u32(G119_PROJECTILE + 0x60, G119_AMMO);
    e.set(G119_HIT, HitData::pWeapon, Ptr::new(weapon));
    e.set(G119_HIT, HitData::pSourceRef, Ptr::new(G119_PROJECTILE));
    if jip {
        jip::test_support::install_into(&mut e);
        // `CreateFileA` (the settings file): "no file", so JIP's defaults; `TESTopic::GetOwnerTopic`,
        // which 3.9a's fix wraps with the row on.
        e.register(0x00fd_f088, |_, _| Ret {
            eax: 0xffff_ffff,
            ..Ret::default()
        });
        e.register(0x0061_a310, |_, _| Ret::default());
        jip::game_patches::set_row(&mut e, true);
    } else {
        e.on_site(SiteSlot::new(Owner::Jip, 0x0089_abdc), |e, a| {
            e.mem.set_u32(G119_ASKED, a[0]);
            e.mem.set_u32(G119_ASKED + 4, a[1]);
            Some(0)
        });
        e.set_jip_patches(word);
    }
    e
}

/// Runs `hit_me_report_attack` on [`ACTOR`] and [`G119_HIT`]: the calls made.
fn g119_report(e: &mut Engine) -> Vec<(u32, Vec<u32>)> {
    e.call_log = Some(vec![]);
    let mut s = HitMe {
        this: ACTOR,
        hit: G119_HIT,
        kind: 0,
        player: 0,
        ebp: BASE + 0xf00,
        moved: false,
        flag_1a: false,
        arrows: 0,
        weapon_type: 1,
        crippled: false,
        kind_one: false,
        flag_63: false,
        flag_64: false,
        health: 0.0,
        fatigue: 0.0,
        total: 0.0,
        blocked: 0.0,
        not_fleeing: false,
        dead: false,
        limb: -1,
        cripple_roll: false,
        torso_hit: false,
    };
    hit_me_report_attack(e, &mut s);
    e.call_log.take().unwrap_or_default()
}

fn g119_calls(log: &[(u32, Vec<u32>)], addr: u32) -> Vec<Vec<u32>> {
    log.iter()
        .filter(|(a, _)| *a == addr)
        .map(|(_, w)| w.clone())
        .collect()
}

/// With JIP not installed or the row off: the seam is not asked; the exe's three current-ammo
/// calls and its script effect `(effect, actor, 0)` run.
#[test]
fn g119_exe_ammo_script_while_off() {
    for word in [0, jip_patches::INSTALLED] {
        let mut e = g119_world(word, false, G119_WEAPON);
        let log = g119_report(&mut e);
        assert_eq!(e.mem.u32(G119_ASKED), 0, "{word:08x}");
        assert_eq!(
            g119_calls(&log, WEAPON_GET_CURRENT_AMMO),
            vec![vec![G119_WEAPON, 0]; 3]
        );
        assert_eq!(
            g119_calls(&log, SCRIPT_RUN_EFFECT_START),
            vec![vec![G119_SCRIPT, ACTOR.addr(), 0]]
        );
    }
}

/// The row on: inside the weapon branch the seam is asked with the hit and the actor hit, and its
/// answer skips the exe's current-ammo calls and script effect; with no weapon it is not asked.
#[test]
fn g119_seam_in_the_weapon_branch() {
    let mut e = g119_world(jip_patches::FIXES, false, G119_WEAPON);
    let log = g119_report(&mut e);
    assert_eq!(
        (e.mem.u32(G119_ASKED), e.mem.u32(G119_ASKED + 4)),
        (G119_HIT.addr(), ACTOR.addr())
    );
    assert!(g119_calls(&log, WEAPON_GET_CURRENT_AMMO).is_empty());
    assert!(g119_calls(&log, SCRIPT_RUN_EFFECT_START).is_empty());
    let mut e = g119_world(jip_patches::FIXES, false, 0);
    let log = g119_report(&mut e);
    assert_eq!(e.mem.u32(G119_ASKED), 0, "no weapon: not asked");
    assert!(g119_calls(&log, SCRIPT_RUN_EFFECT_START).is_empty());
}

/// JIP's handler through `hit_me_report_attack`: the script of the ammo recorded in the
/// projectile runs on the actor hit `(script, actor, 0)` (the exe's current-ammo calls skipped);
/// with no ammo recorded or an ammo whose vtable is not `TESAmmo`'s, nothing runs.
#[test]
fn g119_jips_handler_runs_the_projectiles_ammo_script() {
    let mut e = g119_world(0, true, G119_WEAPON);
    assert_ne!(e.jip_patches() & jip_patches::FIXES, 0);
    let log = g119_report(&mut e);
    assert!(g119_calls(&log, WEAPON_GET_CURRENT_AMMO).is_empty());
    assert_eq!(
        g119_calls(&log, SCRIPT_RUN_EFFECT_START),
        vec![vec![G119_SCRIPT, ACTOR.addr(), 0]]
    );
    for case in 0..2 {
        let mut e = g119_world(0, true, G119_WEAPON);
        match case {
            0 => e.mem.set_u32(G119_PROJECTILE + 0x60, 0),
            _ => e.mem.set_u32(G119_AMMO, 0x0102_6000),
        }
        let log = g119_report(&mut e);
        assert!(
            g119_calls(&log, SCRIPT_RUN_EFFECT_START).is_empty(),
            "case {case}"
        );
        assert!(g119_calls(&log, WEAPON_GET_CURRENT_AMMO).is_empty());
    }
}

// ---- Part 4: G99's seam in `Actor::HitMe` (the worn armour's damage) and G126's in
// `fn_008a03a0` (the player's control flag test).

const G99_ITEM: u32 = BASE + 0x600;
const G99_ARMOR: u32 = BASE + 0x680;
const G99_HIT: Ptr<HitData> = Ptr::new(BASE + 0x700);
const G99_VTABLE: u32 = BASE + 0xb00;
const G99_DAMAGE_SLOT: u32 = 0x00c9_93c8;
const G99_ASKED: u32 = BASE + 0xd00;

/// [`ACTOR`] with a vtable whose `0x3C8` is a stand-in, a worn armour entry ([`G99_ITEM`], its
/// form of health 150) and a hit of armour damage 70.25; the exe's calls logged. `jip`: JIP LN
/// NVSE installed (its own patches; the row off); else a logging handler at `0x89D2FF` answering
/// `answer`, and the word `word`.
fn g99_world(word: u32, jip: bool, answer: Option<u32>) -> Engine {
    let mut e = Engine::new();
    e.map(BASE, 0x1000);
    e.mem.set_u32(ACTOR.addr(), G99_VTABLE);
    e.mem.set_u32(G99_VTABLE + 0x3c8, G99_DAMAGE_SLOT);
    e.register(G99_DAMAGE_SLOT, |_, _| Ret::default());
    e.register(ENTRY_DELETE, |_, _| Ret::default());
    e.register(0x0095_08b0, |_, _| Ret::default());
    e.mem.set_u32(G99_ITEM + 8, G99_ARMOR);
    e.mem.set_u32(G99_ARMOR + 0x6c, 150);
    e.set(G99_HIT, HitData::fArmorDamage, 70.25f32);
    if jip {
        jip::test_support::install_into(&mut e);
        e.register(0x00fd_f088, |_, _| Ret {
            eax: 0xffff_ffff,
            ..Ret::default()
        });
        jip::game_patches::set_row(&mut e, false);
    } else {
        e.mem.set_u32(G99_ASKED + 0x10, answer.map_or(0, |a| a + 1));
        e.on_site(SiteSlot::new(Owner::Jip, 0x0089_d2ff), |e, a| {
            for (i, w) in a.iter().enumerate() {
                e.mem.set_u32(G99_ASKED + 4 * i as u32, *w);
            }
            e.mem.u32(G99_ASKED + 0x10).checked_sub(1)
        });
        e.set_jip_patches(word);
    }
    e
}

fn g99_run(e: &mut Engine, item: u32) -> Vec<(u32, Vec<u32>)> {
    e.call_log = Some(vec![]);
    damage_worn_armor(e, ACTOR, item, G99_HIT);
    e.call_log.take().unwrap_or_default()
}

/// With JIP not installed, or only the row's bit: the seam is not asked; the actor's `0x3C8`
/// takes `(item, 70.25, 0)`, then the entry is deleted with 1. No item: nothing.
#[test]
fn g99_exe_armour_damage_while_off() {
    for word in [0, jip_patches::FIXES] {
        let mut e = g99_world(word, false, Some(0));
        let log = g99_run(&mut e, G99_ITEM);
        assert_eq!(e.mem.u32(G99_ASKED), 0, "{word:08x}");
        assert_eq!(
            g119_calls(&log, G99_DAMAGE_SLOT),
            vec![vec![ACTOR.addr(), G99_ITEM, 70.25f32.to_bits(), 0]]
        );
        assert_eq!(g119_calls(&log, ENTRY_DELETE), vec![vec![G99_ITEM, 1]]);
        assert!(g99_run(&mut e, 0).is_empty(), "no item");
    }
}

/// With `INSTALLED`: the seam is asked with the actor, the item and the hit; an answer skips the
/// exe's damage and delete, no answer leaves them. No item: not asked.
#[test]
fn g99_seam_takes_the_span() {
    let mut e = g99_world(jip_patches::INSTALLED, false, Some(0));
    let log = g99_run(&mut e, G99_ITEM);
    assert_eq!(
        (0..3)
            .map(|i| e.mem.u32(G99_ASKED + 4 * i))
            .collect::<Vec<_>>(),
        vec![ACTOR.addr(), G99_ITEM, G99_HIT.addr()]
    );
    assert!(g119_calls(&log, G99_DAMAGE_SLOT).is_empty());
    assert!(g119_calls(&log, ENTRY_DELETE).is_empty());
    let mut e = g99_world(jip_patches::INSTALLED, false, None);
    let log = g99_run(&mut e, G99_ITEM);
    assert_eq!(g119_calls(&log, G99_DAMAGE_SLOT).len(), 1, "no answer");
    assert_eq!(g119_calls(&log, ENTRY_DELETE).len(), 1);
    let mut e = g99_world(jip_patches::INSTALLED, false, Some(0));
    g99_run(&mut e, 0);
    assert_eq!(e.mem.u32(G99_ASKED), 0, "no item: not asked");
}

/// JIP's handler through `damage_worn_armor`: `PlayerCharacter::DamageEquipment` by address with
/// the smaller of 70.25 and the cap (1.0) times 150, then the delete; not the actor's `0x3C8`.
#[test]
fn g99_jips_handler_through_the_span() {
    let mut e = g99_world(0, true, None);
    assert_ne!(e.jip_patches() & jip_patches::INSTALLED, 0);
    let log = g99_run(&mut e, G99_ITEM);
    assert!(g119_calls(&log, G99_DAMAGE_SLOT).is_empty());
    assert_eq!(
        g119_calls(&log, 0x0095_08b0),
        vec![vec![ACTOR.addr(), G99_ITEM, 70.25f32.to_bits(), 0]]
    );
    assert_eq!(g119_calls(&log, ENTRY_DELETE), vec![vec![G99_ITEM, 1]]);
}

const G126_PLAYER: u32 = BASE + 0x800;

/// G126: with the word 0 the exe's `005a03f0` answers (flag 1); with `INSTALLED` the seam's
/// answer's low byte is the test (the call not made), no answer the exe's call.
#[test]
fn g126_seam_replaces_the_control_flag_call() {
    let mut e = Engine::new();
    e.map(BASE, 0x1000);
    e.register(PLAYER_TEST_5A03F0, |_, a| u32::from(a[1] == 1).into_ret());
    e.mem.set_u32(G99_ASKED + 0x10, 0);
    e.on_site(SiteSlot::new(Owner::Jip, 0x008a_04b2), |e, a| {
        e.mem.set_u32(G99_ASKED, a[0]);
        e.mem.u32(G99_ASKED + 0x10).checked_sub(1)
    });
    e.call_log = Some(vec![]);
    assert!(player_controls_test(&mut e, G126_PLAYER));
    assert_eq!(e.mem.u32(G99_ASKED), 0, "word 0: not asked");
    e.set_jip_patches(jip_patches::INSTALLED);
    e.mem.set_u32(G99_ASKED + 0x10, 0x101);
    assert!(!player_controls_test(&mut e, G126_PLAYER), "0x100: false");
    assert_eq!(e.mem.u32(G99_ASKED), G126_PLAYER);
    e.mem.set_u32(G99_ASKED + 0x10, 2);
    assert!(player_controls_test(&mut e, G126_PLAYER));
    let log = e.call_log.take().unwrap_or_default();
    assert_eq!(
        g119_calls(&log, PLAYER_TEST_5A03F0),
        vec![vec![G126_PLAYER, 1]]
    );
    e.mem.set_u32(G99_ASKED + 0x10, 0);
    assert!(
        player_controls_test(&mut e, G126_PLAYER),
        "no answer: the exe's"
    );
}

/// JIP's handler: the player's `+0x680` bit 0, then false once `ToggleNoMovementCombat` is on.
#[test]
fn g126_jips_handler() {
    let mut e = g99_world(0, true, None);
    e.mem.set_u8(G126_PLAYER + 0x680, 1);
    assert!(player_controls_test(&mut e, G126_PLAYER));
    jip::game_patches::gc::damage::set_no_movement_combat(&mut e, true);
    assert!(!player_controls_test(&mut e, G126_PLAYER));
}
