//! Group GD's game patches: G117 (a projectile's ammo, count and split beams) and part 2 (G27, G30,
//! G62, G91, G92, G102, O4), case by case from
//! JIP's `InitProjectileSetAmmoHook`, `TESObjectWEAP::GetEquippedAmmo` and `GetAmmo`.

use super::*;
use crate::game_patches::set_row;
use crate::test_support::engine;

/// `CreateFileA`'s import slot (the settings file read): answered "no file", so JIP's defaults.
const CREATE_FILE_A: u32 = 0x00fd_f088;
/// JIP's 1.3 as the exe holds it at `01022004`.
const SCALE: f32 = 1.3;

/// An engine with JIP installed as `installed` answers and the host's row `row`.
fn jip(installed: bool, row: bool) -> Engine {
    let mut e = engine();
    e.register(CREATE_FILE_A, |_, _| Ret {
        eax: 0xffff_ffff,
        ..Ret::default()
    });
    // `TESTopic::GetOwnerTopic`, which 3.9a's fix wraps with the row on.
    e.register(0x0061_a310, |_, _| Ret::default());
    crate::install_as(&mut e, installed);
    set_row(&mut e, row);
    e.map(SPLIT_BEAM_SCALE & !0xfff, 0x1000);
    e.mem.set_f32(SPLIT_BEAM_SCALE, SCALE);
    e
}

/// A projectile (damage `damage`, the record's bytes filled with junk the hook must overwrite) and
/// a weapon (count `count`, ammo use `ammo_use`, ammo form `ammo`).
struct Shot {
    projectile: u32,
    weapon: u32,
}

fn shot(e: &mut Engine, damage: f32, count: u8, ammo_use: u8, ammo: u32) -> Shot {
    let projectile = e.mem.alloc(0x150);
    e.mem.set_f32(projectile + 0xdc, damage);
    e.mem.set_u32(projectile + AMMO, 0xdead_beef);
    e.mem.set_u32(projectile + 0x148, 0x5a5a_5a01);
    let weapon = e.mem.alloc(0x388);
    e.mem.set_u8(weapon + 0x11e, count);
    e.mem.set_u8(weapon + 0x102, ammo_use);
    e.mem.set_u32(weapon + 0xa8, ammo);
    Shot { projectile, weapon }
}

/// A `TESAmmo` whose projectiles-per-shot word is `per_shot`.
fn ammo(e: &mut Engine, per_shot: u32) -> u32 {
    let ammo = e.mem.alloc(0xdc);
    e.mem.set_u32(ammo, TES_AMMO_VTBL);
    e.mem.set_u8(ammo + 4, 0x29);
    e.mem.set_u32(ammo + 0xb0, per_shot);
    ammo
}

fn fire(e: &mut Engine, s: &Shot, actor: u32, split: u32) -> Option<u32> {
    e.site(
        PROJECTILE_SET_AMMO.addr(),
        &[s.projectile, s.weapon, actor, split],
    )
}

/// The record and the damage: (ammo word, split byte, count byte, damage, the byte after).
fn record(e: &Engine, s: &Shot) -> (u32, u8, u8, f32, u8) {
    let p = s.projectile;
    (
        e.mem.u32(p + AMMO),
        e.mem.u8(p + HAS_SPLIT_BEAMS),
        e.mem.u8(p + NUM_PROJECTILES),
        e.mem.f32(p + 0xdc),
        e.mem.u8(p + 0x14b),
    )
}

/// Not installed (today), with the row on: nothing at the seam; asking it changes nothing.
#[test]
fn projectile_set_ammo_not_installed_nothing_is_registered() {
    let mut e = jip(false, true);
    assert!(e.site_owners(PROJECTILE_SET_AMMO.addr()).is_empty());
    let a = ammo(&mut e, 2);
    let s = shot(&mut e, 30.0, 3, 1, a);
    assert_eq!(fire(&mut e, &s, 0, 1), None);
    assert_eq!(record(&e, &s), (0xdead_beef, 0x5a, 0x5a, 30.0, 0x5a));
}

/// Installed with the row off (vanilla): G117 is a fix, so nothing at the seam either.
#[test]
fn projectile_set_ammo_installed_row_off_nothing_is_registered() {
    let mut e = jip(true, false);
    assert!(e.site_owners(PROJECTILE_SET_AMMO.addr()).is_empty());
    let s = shot(&mut e, 30.0, 3, 1, 0);
    assert_eq!(fire(&mut e, &s, 0, 0), None);
    assert_eq!(record(&e, &s), (0xdead_beef, 0x5a, 0x5a, 30.0, 0x5a));
}

/// The count: the weapon's own unless the ammo's per-shot byte (signed) is above 0, then that
/// times the weapon's ammo use; the low byte recorded and tested (`cmp dl, 1`), the whole word
/// divided by (`cvtsi2ss edx`); the next byte never written.
#[test]
fn projectile_set_ammo_counts_as_jip() {
    let mut e = jip(true, true);
    assert_eq!(e.site_owners(PROJECTILE_SET_AMMO.addr()), [Owner::Jip]);
    // (weapon count, ammo use, per-shot word or no ammo, recorded count, damage after)
    let none = u32::MAX;
    for (count, ammo_use, per_shot, recorded, after) in [
        (3u8, 1u8, 0u32, 3u8, 10.0f32), // per shot 0: the weapon's count
        (3, 4, 2, 8, 3.75),             // 4 x 2
        (3, 4, 0x80, 3, 10.0),          // per-shot byte -128: the weapon's count
        (3, 4, 0x0100, 3, 10.0),        // the word's low byte only (0)
        (3, 4, none, 3, 10.0),          // no ammo
        (1, 4, 0, 1, 30.0),             // one: no division
        (0, 4, 0, 0, 30.0),             // none: no division
        (3, 0x81, 2, 2, 30.0 / 258.0),  // 0x102: low byte 2, divided by 258
        (3, 0x80, 2, 0, 30.0),          // 0x100: low byte 0, no division
    ] {
        let a = if per_shot == none {
            0
        } else {
            ammo(&mut e, per_shot)
        };
        let s = shot(&mut e, 30.0, count, ammo_use, a);
        assert_eq!(fire(&mut e, &s, 0, 0), Some(0));
        assert_eq!(
            record(&e, &s),
            (a, 0, recorded, after, 0x5a),
            "{count} {ammo_use} {per_shot:x}"
        );
        assert_eq!(
            projectile_ammo(&e, s.projectile),
            if recorded != 0 { a } else { 0 }
        );
    }
}

/// Split beams (the flag's low byte): the damage times JIP's float 1.3 first, the first mod slot
/// with effect 0xC adds its value truncated (`cvttss2si`; NaN adds 0x80000000), then the
/// division; no 0xC slot: the scale alone.
#[test]
fn projectile_set_ammo_split_beams_as_jip() {
    let mut e = jip(true, true);
    // (effects, values, recorded count, divisor or 0 for none)
    for (effects, values, recorded, divisor) in [
        ([1u32, 5, 0xc], [9.0f32, 9.0, 2.7], 5u8, 5.0f32),
        ([0xc, 0xc, 0], [1.9, 7.0, 0.0], 4, 4.0),
        ([1, 2, 3], [9.0, 9.0, 9.0], 3, 3.0),
        ([0xc, 0, 0], [-1.5, 0.0, 0.0], 2, 2.0),
        ([0xc, 0, 0], [-2.0, 0.0, 0.0], 1, 0.0),
        ([0xc, 0, 0], [f32::NAN, 0.0, 0.0], 3, -2_147_483_645.0),
        ([0xc, 0, 0], [3.0e9, 0.0, 0.0], 3, -2_147_483_645.0),
    ] {
        let s = shot(&mut e, 10.0, 3, 1, 0);
        for i in 0..3 {
            e.mem.set_u32(s.weapon + 0x180 + 4 * i, effects[i as usize]);
            e.mem.set_f32(s.weapon + 0x18c + 4 * i, values[i as usize]);
        }
        assert_eq!(fire(&mut e, &s, 0, 0x0000_0001), Some(0));
        let scaled = 10.0f32 * SCALE;
        let after = if divisor == 0.0 {
            scaled
        } else {
            scaled / divisor
        };
        assert_eq!(record(&e, &s), (0, 1, recorded, after, 0x5a), "{values:?}");
    }
    // The flag word's low byte only: 0x100 is no split.
    let s = shot(&mut e, 10.0, 2, 1, 0);
    e.mem.set_u32(s.weapon + 0x180, 0xc);
    e.mem.set_f32(s.weapon + 0x18c, 4.0);
    assert_eq!(fire(&mut e, &s, 0, 0x100), Some(0));
    assert_eq!(record(&e, &s), (0, 0, 2, 5.0, 0x5a));
}

/// A middle or high process (level 0 or 1) whose weapon entry is this weapon and whose ammo
/// entry's form is a `TESAmmo` gives the ammo; anything else falls back to the weapon's.
#[test]
fn projectile_set_ammo_takes_the_loaded_ammo() {
    let mut e = jip(true, true);
    let own = ammo(&mut e, 0);
    let loaded = ammo(&mut e, 2);
    let not_ammo = e.mem.alloc(0x20);
    e.mem.set_u32(not_ammo, TES_AMMO_VTBL + 4);
    // (process level, weapon entry is ours, ammo entry form, expected ammo)
    for (level, ours, form, expected) in [
        (0u8, true, loaded, loaded),
        (1, true, loaded, loaded),
        (2, true, loaded, own),
        (1, false, loaded, own),
        (1, true, not_ammo, own),
        (1, true, 0, own),
    ] {
        let s = shot(&mut e, 30.0, 3, 5, own);
        let actor = e.mem.alloc(0x1b4);
        let process = e.mem.alloc(0x200);
        e.mem.set_u32(actor + 0x68, process);
        e.mem.set_u8(process + 0x28, level);
        let weapon_entry = e.mem.alloc(0x10);
        e.mem
            .set_u32(weapon_entry + 8, if ours { s.weapon } else { s.weapon + 4 });
        e.mem.set_u32(process + 0x114, weapon_entry);
        let ammo_entry = e.mem.alloc(0x10);
        e.mem.set_u32(ammo_entry + 8, form);
        e.mem.set_u32(process + 0x118, ammo_entry);
        assert_eq!(fire(&mut e, &s, actor, 0), Some(0));
        let (recorded, after) = if expected == loaded {
            (10u8, 3.0f32)
        } else {
            (3, 10.0)
        };
        assert_eq!(
            record(&e, &s),
            (expected, 0, recorded, after, 0x5a),
            "{level} {ours} {form:x}"
        );
    }
    // No process, no weapon entry, no ammo entry: the weapon's ammo.
    for case in 0..3 {
        let s = shot(&mut e, 30.0, 3, 5, own);
        let actor = e.mem.alloc(0x1b4);
        if case > 0 {
            let process = e.mem.alloc(0x200);
            e.mem.set_u32(actor + 0x68, process);
            if case == 2 {
                let weapon_entry = e.mem.alloc(0x10);
                e.mem.set_u32(weapon_entry + 8, s.weapon);
                e.mem.set_u32(process + 0x114, weapon_entry);
            }
        }
        assert_eq!(fire(&mut e, &s, actor, 0), Some(0));
        assert_eq!(record(&e, &s).0, own, "{case}");
    }
}

/// A form list as the weapon's ammo: JIP's walk answers the item `+0x20` nodes from the head
/// when the list is that long, else the first item.
#[test]
fn weapon_ammo_walks_a_form_list_as_jip() {
    let mut e = jip(true, true);
    let items = [0x0100_0001u32, 0x0100_0002, 0x0100_0003];
    let list = e.mem.alloc(0x24);
    e.mem.set_u8(list + 4, 0x55);
    let mut node = list + 0x18;
    for (i, item) in items.iter().enumerate() {
        e.mem.set_u32(node, *item);
        if i + 1 < items.len() {
            let next = e.mem.alloc(8);
            e.mem.set_u32(node + 4, next);
            node = next;
        }
    }
    let weapon = e.mem.alloc(0x388);
    e.mem.set_u32(weapon + 0xa8, list);
    for (walk, expected) in [
        (0u32, items[0]),
        (1, items[1]),
        (2, items[2]),
        (3, items[0]),
        (0x8000_0000, items[0]),
    ] {
        e.mem.set_u32(list + 0x20, walk);
        assert_eq!(weapon_ammo(&e, weapon), expected, "{walk:x}");
    }
    // Not a list: the form itself; none: 0.
    e.mem.set_u8(list + 4, 0x29);
    assert_eq!(weapon_ammo(&e, weapon), list);
    e.mem.set_u32(weapon + 0xa8, 0);
    assert_eq!(weapon_ammo(&e, weapon), 0);
}

/// Part 2's seams.
const PART2: [SiteSlot; 8] = [
    EXPLOSION_SOURCE[0],
    EXPLOSION_SOURCE[1],
    EXPLOSION_PERK,
    MOVE_MODE,
    REACTION_COOLDOWN,
    AI_TARGET,
    SPEAKER_DONE,
    MISSILE_FLAGS,
];

/// Not installed (row on) and installed with the row off: none of part 2's seams has a handler,
/// and asking them answers nothing; installed with the row: each has JIP's.
#[test]
fn part2_registered_only_with_the_fixes() {
    for (installed, row) in [(false, true), (true, false)] {
        let mut e = jip(installed, row);
        for slot in PART2 {
            assert!(e.site_owners(slot.addr()).is_empty(), "{:x}", slot.addr());
            assert_eq!(e.site(slot.addr(), &[0x0300_0000, 0]), None);
        }
    }
    let e = jip(true, true);
    for slot in PART2 {
        assert_eq!(
            e.site_owners(slot.addr()),
            [Owner::Jip],
            "{:x}",
            slot.addr()
        );
    }
}

/// Each part-2 seam at the exe address JIP patches (internal/patches_game.h: 5233 and 5234
/// G27, 5245 G30, 5337 G62, 5377 G91, 5378 G92, 5394 G102, 4803 O4), and each handler answers
/// at its own literal address: two constants swapped would put each handler at the other's
/// seam.
#[test]
fn part2_seams_at_jip_addresses() {
    assert_eq!(
        PART2.map(|slot| slot.addr()),
        [
            0x0047_667d,
            0x009c_35f7,
            0x009a_ca23,
            0x008d_aa6f,
            0x009d_10f3,
            0x009f_50c4,
            0x009e_e367,
            0x009b_7e14,
        ]
    );
    // The game addresses and values the handlers use, as JIP's code names them: G30's
    // HandleEntryPoint (`call 0x5E58F0`), entry 0x48, `Actor::IsActor` 0x8D0360; G62's
    // kPackageFlag_AlwaysWalk / AlwaysRun and the bytes +0x124 / +0x104; G91's 0x457FE0 and
    // 0x11CE9B8; GetDisabled's 0x5AA680 / 0x5AA630; G102's +0x108 == 2.
    assert_eq!(
        [
            APPLY_PERK_MODIFIERS,
            ADJUST_EXPLOSION_RADIUS,
            ACTOR_IS_ACTOR,
            ALWAYS_WALK,
            ALWAYS_RUN,
            ACTOR_BYTE_124,
            ACTOR_BYTE_104,
            TICK_COUNT,
            REACTION_COOLDOWN_SETTING,
            PENDING_ENABLED,
            PENDING_DISABLED,
            LIFE_STATE,
            DEAD,
        ],
        [
            0x005e_58f0,
            0x48,
            0x008d_0360,
            0x2000_0000,
            0x2000,
            0x124,
            0x104,
            0x0045_7fe0,
            0x011c_e9b8,
            0x005a_a680,
            0x005a_a630,
            0x108,
            2,
        ]
    );
    let mut e = jip(true, true);
    e.register(PENDING_ENABLED, |_, _| 0u32.into_ret());
    e.register(PENDING_DISABLED, |_, _| 0u32.into_ret());
    // G92 at 0x9F50C4: a reference with flag 0x20 and life state 0 is skipped (G102 there
    // would answer 0).
    let reference = e.mem.alloc(0x200);
    e.mem.set_u32(reference + 8, 0x20);
    assert_eq!(e.site(0x009f_50c4, &[reference]), Some(1));
    // G102 at 0x9EE367: a dead speaker with no flags is done (G92 there would answer 0).
    let speaker = e.mem.alloc(0x200);
    e.mem.set_u32(speaker + 0x108, DEAD);
    assert_eq!(e.site(0x009e_e367, &[speaker]), Some(1));
    // G27 at both: the reference itself.
    for address in [0x0047_667d, 0x009c_35f7] {
        assert_eq!(e.site(address, &[speaker]), Some(speaker));
    }
    // G30 at 0x9ACA23: no shooter keeps the multiplier.
    assert_eq!(
        e.site(0x009a_ca23, &[reference, 0x3fc0_0000]),
        Some(0x3fc0_0000)
    );
    // G62 at 0x8DAA6F: Always Walk walks.
    let package = e.mem.alloc(0x80);
    e.mem.set_u32(package + 0x1c, ALWAYS_WALK);
    assert_eq!(e.site(0x008d_aa6f, &[speaker, package]), Some(0x101));
    // G91 at 0x9D10F3: the tick read, the cooldown passed.
    tick_engine(&mut e);
    TICK.with(|t| t.set(10_000));
    assert_eq!(e.site(0x009d_10f3, &[reference]), Some(1));
    assert_eq!(
        e.ext_free::<ReactionTick>().0.load(Ordering::Relaxed),
        10_000
    );
    // O4 at 0x9B7E14: the flag byte set for a flagged base.
    let projectile = e.mem.alloc(0x150);
    let base = e.mem.alloc(0xd0);
    e.mem.set_u32(projectile + 0x20, base);
    e.mem.set_u8(base + 0x61, 1);
    assert_eq!(e.site(0x009b_7e14, &[projectile, 1]), Some(0));
    assert_eq!(e.mem.u8(projectile + 0xc9), 0x80);
}

/// G27: the reference itself, whatever it is.
#[test]
fn explosion_source_is_the_reference() {
    let mut e = jip(true, true);
    for slot in EXPLOSION_SOURCE {
        for reference in [0x0300_1000u32, 0, 0xffff_fff0] {
            assert_eq!(e.site(slot.addr(), &[reference]), Some(reference));
        }
    }
}

thread_local! {
    static ENTRY_CALLS: std::cell::RefCell<Vec<(Vec<u32>, f32)>> = const { std::cell::RefCell::new(Vec::new()) };
}

/// G30: no shooter or a shooter that is not an actor keeps the multiplier (no entry point); an
/// actor shooter runs entry point 0x48 with the projectile's weapon on the multiplier.
#[test]
fn explosion_perk_runs_the_entry_point_for_actor_shooters() {
    let mut e = jip(true, true);
    e.register(APPLY_PERK_MODIFIERS, |e, a| {
        let before = e.mem.f32(a[3]);
        ENTRY_CALLS.with(|s| s.borrow_mut().push((a[..3].to_vec(), before)));
        e.mem.set_f32(a[3], before * 2.0);
        Ret::default()
    });
    let actor_vtable = e.mem.alloc(0x400);
    e.mem.set_u32(actor_vtable + 0x100, ACTOR_IS_ACTOR);
    let other_vtable = e.mem.alloc(0x400);
    e.mem.set_u32(other_vtable + 0x100, ACTOR_IS_ACTOR + 0x10);
    let actor = e.mem.alloc(0x1b4);
    e.mem.set_u32(actor, actor_vtable);
    let other = e.mem.alloc(0x1b4);
    e.mem.set_u32(other, other_vtable);
    let source = e.mem.alloc(0x150);
    e.mem.set_u32(source + 0xf8, 0x0100_0abc);
    // (shooter, the answer)
    for (shooter, answer) in [(0, 1.5f32), (other, 1.5), (actor, 3.0)] {
        e.mem.set_u32(source + 0xfc, shooter);
        ENTRY_CALLS.with(|s| s.borrow_mut().clear());
        let got = e.site(EXPLOSION_PERK.addr(), &[source, 1.5f32.to_bits()]);
        assert_eq!(got, Some(answer.to_bits()), "{shooter:x}");
        let calls = ENTRY_CALLS.with(|s| s.take());
        if shooter == actor {
            assert_eq!(calls, vec![(vec![0x48, actor, 0x0100_0abc], 1.5)]);
        } else {
            assert!(calls.is_empty());
        }
    }
}

/// G62: Always Walk first (walk even for a running actor), then Always Run, then the actor's
/// bytes `+0x124` and `+0x104` (run), else 0; no package skips the flags.
#[test]
fn move_mode_honours_always_walk() {
    let mut e = jip(true, true);
    let actor = e.mem.alloc(0x200);
    let package = e.mem.alloc(0x80);
    let none = u32::MAX;
    // (package flags or none, byte +0x124, byte +0x104, the answer)
    for (flags, b124, b104, mode) in [
        (ALWAYS_WALK, 1u8, 1u8, 0x101u32),
        (ALWAYS_WALK | ALWAYS_RUN, 0, 0, 0x101),
        (ALWAYS_RUN, 0, 0, 0x201),
        (0, 1, 0, 0x201),
        (0, 0, 1, 0x201),
        (0, 0, 0, 0),
        (!(ALWAYS_WALK | ALWAYS_RUN), 0, 0, 0),
        (none, 1, 0, 0x201),
        (none, 0, 0, 0),
    ] {
        e.mem.set_u8(actor + 0x124, b124);
        e.mem.set_u8(actor + 0x104, b104);
        let pkg = if flags == none {
            0
        } else {
            e.mem.set_u32(package + 0x1c, flags);
            package
        };
        assert_eq!(
            e.site(MOVE_MODE.addr(), &[actor, pkg]),
            Some(mode),
            "{flags:x} {b124} {b104}"
        );
    }
    // JIP compares single bytes (`cmp byte ptr [ecx+0x124]`, `[ecx+0x104]`): the neighbours
    // (forceSneak +0x125, searchingForEnemies +0x127, jipActorFlags1-3 +0x105..+0x107, and the
    // bytes before) set, the two bytes clear, give 0.
    e.mem.set_u8(actor + 0x124, 0);
    e.mem.set_u8(actor + 0x104, 0);
    for offset in [0x103u32, 0x105, 0x106, 0x107, 0x123, 0x125, 0x126, 0x127] {
        e.mem.set_u8(actor + offset, 0xff);
    }
    e.mem.set_u32(package + 0x1c, 0);
    for pkg in [package, 0] {
        assert_eq!(e.site(MOVE_MODE.addr(), &[actor, pkg]), Some(0), "{pkg:x}");
    }
}

thread_local! {
    static TICK: std::cell::Cell<u32> = const { std::cell::Cell::new(0) };
}

/// An engine for G91: the tick wrapper answers [`TICK`], the setting is 2 s.
fn tick_engine(e: &mut Engine) {
    e.register(TICK_COUNT, |_, _| TICK.with(|t| t.get()).into_ret());
    e.map(REACTION_COOLDOWN_SETTING & !0xfff, 0x1000);
    e.mem.set_f32(REACTION_COOLDOWN_SETTING, 2.0);
}

/// G91: more than the setting's seconds of ticks since the last line passes and keeps the tick;
/// exactly the setting, less, or a tick before the last does not; the difference wraps as a
/// signed int.
#[test]
fn reaction_cooldown_in_real_milliseconds() {
    let mut e = jip(true, true);
    tick_engine(&mut e);
    // (now, the answer, the kept tick after)
    for (now, ready, kept) in [
        (1000u32, 0u32, 0u32),
        (2001, 1, 2001),
        (4001, 0, 2001),
        (4002, 1, 4002),
        (1000, 0, 4002),
        (0x8000_0000, 1, 0x8000_0000),
        (0x8000_07d0, 0, 0x8000_0000),
    ] {
        TICK.with(|t| t.set(now));
        assert_eq!(e.site(REACTION_COOLDOWN.addr(), &[0]), Some(ready), "{now}");
        assert_eq!(e.ext_free::<ReactionTick>().0.load(Ordering::Relaxed), kept);
    }
    // Across the tick counter's wrap: 0xFFFFF000 then 0x00000800 is 6.144 s.
    e.ext_free::<ReactionTick>()
        .0
        .store(0xffff_f000, Ordering::Relaxed);
    TICK.with(|t| t.set(0x0000_0800));
    assert_eq!(e.site(REACTION_COOLDOWN.addr(), &[0]), Some(1));
}

/// G91's tick is one for the run: an engine sharing the slots (a real thread's) reads and
/// writes the same word.
#[test]
fn reaction_cooldown_tick_is_run_wide() {
    let mut e = jip(true, true);
    tick_engine(&mut e);
    let mut other = engine();
    e.share_slots_for_test(&mut other);
    tick_engine(&mut other);
    TICK.with(|t| t.set(5000));
    assert_eq!(reaction_cooldown(&mut other, &[0]), Some(1));
    assert_eq!(e.ext_free::<ReactionTick>().0.load(Ordering::Relaxed), 5000);
    TICK.with(|t| t.set(6000));
    assert_eq!(e.site(REACTION_COOLDOWN.addr(), &[0]), Some(0));
}

thread_local! {
    /// The pending lists' answers (enabled, disabled) and their calls (enabled + 0x100 x
    /// disabled).
    static PENDING: std::cell::Cell<(u32, u32, u32)> = const { std::cell::Cell::new((0, 0, 0)) };
    /// The words each pending-list call was given (JIP's GetDisabled pushes the reference:
    /// one cdecl word for both).
    static PENDING_WORDS: std::cell::RefCell<Vec<Vec<u32>>> = const { std::cell::RefCell::new(Vec::new()) };
}

/// G92: a reference with any flag of 0x4820 is skipped without asking the script's pending
/// lists; else it is skipped when pending disabled.
#[test]
fn ai_target_skips_flagged_and_disabled_references() {
    let mut e = jip(true, true);
    e.register(PENDING_ENABLED, |_, a| {
        PENDING_WORDS.with(|s| s.borrow_mut().push(a.to_vec()));
        PENDING.with(|p| {
            let (en, dis, n) = p.get();
            p.set((en, dis, n + 1));
            en.into_ret()
        })
    });
    e.register(PENDING_DISABLED, |_, a| {
        PENDING_WORDS.with(|s| s.borrow_mut().push(a.to_vec()));
        PENDING.with(|p| {
            let (en, dis, n) = p.get();
            p.set((en, dis, n + 0x100));
            dis.into_ret()
        })
    });
    let reference = e.mem.alloc(0x100);
    // (flags, pending enabled, pending disabled, the answer, the calls)
    for (flags, en, dis, skip, calls) in [
        (0x20u32, 0u32, 1u32, 1u32, 0u32),
        (0x800, 0, 1, 1, 0),
        (0x4000, 0, 1, 1, 0),
        (0x4820, 0, 1, 1, 0),
        (0, 0, 1, 1, 0x100),
        (0, 1, 0, 0, 0x100),
        (0x400, 0, 0, 0, 0x100),
    ] {
        e.mem.set_u32(reference + 8, flags);
        PENDING.with(|p| p.set((en, dis, 0)));
        PENDING_WORDS.with(|s| s.borrow_mut().clear());
        assert_eq!(
            e.site(AI_TARGET.addr(), &[reference]),
            Some(skip),
            "{flags:x}"
        );
        assert_eq!(PENDING.with(|p| p.get().2), calls, "{flags:x}");
        let words = PENDING_WORDS.with(|s| s.take());
        assert_eq!(words, vec![vec![reference]; calls.count_ones() as usize]);
    }
    // JIP's GetDisabled with the disabled flag: disabled unless pending enabled.
    e.mem.set_u32(reference + 8, 0x800);
    for (en, dis, disabled, calls) in [(0, 0, true, 1usize), (1, 0, false, 2), (1, 1, true, 2)] {
        PENDING.with(|p| p.set((en, dis, 0)));
        PENDING_WORDS.with(|s| s.borrow_mut().clear());
        assert_eq!(get_disabled(&mut e, reference), disabled, "{en} {dis}");
        let words = PENDING_WORDS.with(|s| s.take());
        assert_eq!(words, vec![vec![reference]; calls], "{en} {dis}");
    }
}

/// G102: only life state 2 (dead) as a whole word; dying (1) and state 6 are not.
#[test]
fn speaker_done_only_when_dead() {
    let mut e = jip(true, true);
    let actor = e.mem.alloc(0x1b4);
    for (state, dead) in [(2u32, 1u32), (0, 0), (1, 0), (6, 0), (0x102, 0), (3, 0)] {
        e.mem.set_u32(actor + 0x108, state);
        assert_eq!(
            e.site(SPEAKER_DONE.addr(), &[actor]),
            Some(dead),
            "{state:x}"
        );
    }
}

/// O4: the flag 0x8000 (byte `+0xC9` bit 0x80) only with the value's low byte set and the base's
/// byte `+0x61` bit 1; the other bits kept; never cleared.
#[test]
fn missile_flags_set_only_for_flagged_bases() {
    let mut e = jip(true, true);
    let projectile = e.mem.alloc(0x150);
    let base = e.mem.alloc(0xd0);
    e.mem.set_u32(projectile + 0x20, base);
    // The neighbouring words point at a form whose byte +0x61 is clear: only `+0x20` (JIP's
    // `[ecx+0x20]`) is the base.
    let other = e.mem.alloc(0xd0);
    for offset in [0x1cu32, 0x24] {
        e.mem.set_u32(projectile + offset, other);
    }
    // (value, base byte, flag byte before, after)
    for (value, base_byte, before, after) in [
        (1u32, 0x01u8, 0x01u8, 0x81u8),
        (1, 0xff, 0x00, 0x80),
        (1, 0xfe, 0x00, 0x00),
        (0, 0x01, 0x00, 0x00),
        (0x100, 0x01, 0x00, 0x00),
        (0, 0x01, 0x80, 0x80),
        (1, 0x00, 0x80, 0x80),
    ] {
        e.mem.set_u8(base + 0x61, base_byte);
        e.mem.set_u8(projectile + 0xc9, before);
        e.mem.set_u8(projectile + 0xc8, 0x5a);
        assert_eq!(e.site(MISSILE_FLAGS.addr(), &[projectile, value]), Some(0));
        assert_eq!(
            e.mem.u8(projectile + 0xc9),
            after,
            "{value:x} {base_byte:x}"
        );
        assert_eq!(e.mem.u8(projectile + 0xc8), 0x5a);
    }
}

/// Part 3's seams: the fixes (G101, G127) and JIP's own (G42 x2, G124, G125).
const PART3_FIXES: [SiteSlot; 2] = [DEATH_RESPONSE, MINE_EXPLODE_CHANCE];
const PART3_JIP: [SiteSlot; 4] = [
    MOVE_AWAY_DISTANCE[0],
    MOVE_AWAY_DISTANCE[1],
    EXPLOSION_IN_FACE,
    EXPLOSION_SHAKE,
];

/// Each part-3 seam at the exe address JIP patches (internal/patches_game.h: 5393 G101, 5430
/// G127, 5280 and 5281 G42, 5427 G124, 5428 G125), the game values the handlers use pinned to
/// JIP's (entry point 4, `GetRandomInt` 0xAA5230 on 0x11C4180, `kMaxDist` 1024.0, the actor's
/// +0x130 / +0x7D / +0x30 and the process slot +0x4B8, G42's 300.0), and each handler answering
/// at its own literal address.
#[test]
fn part3_seams_at_jip_addresses() {
    assert_eq!(
        PART3_FIXES.map(|slot| slot.addr()),
        [0x0098_4156, 0x009c_3bd6]
    );
    assert_eq!(
        PART3_JIP.map(|slot| slot.addr()),
        [0x009e_d3f8, 0x009e_d528, 0x009a_cec6, 0x009a_d2c5]
    );
    assert_eq!(
        [
            ADJUST_MINE_EXPLODE_CHANCE,
            GET_RANDOM_INT,
            GAME_RNG,
            COMBAT_ALLIES,
            IS_TALKING,
            POSITION,
            IS_ON_DIALOGUE_PACKAGE,
            COMBAT_DIALOGUE,
            DEATH_RESPONSE_SUBTYPE,
        ],
        [4, 0x00aa_5230, 0x011c_4180, 0x130, 0x7d, 0x30, 0x4b8, 2, 6]
    );
    assert_eq!(MAX_ALLY_DISTANCE.to_bits(), 0x4480_0000);
    assert_eq!(MINE_CHANCE.to_bits(), 0x42c8_0000);
    assert_eq!(
        MoveAwayDistance::default().0.load(Ordering::Relaxed),
        0x4396_0000
    );
    let mut e = jip(true, true);
    // G42 at both: JIP's distance.
    e.ext_free::<MoveAwayDistance>()
        .0
        .store(123.0f32.to_bits(), Ordering::Relaxed);
    for address in [0x009e_d3f8, 0x009e_d528] {
        assert_eq!(e.site(address, &[]), Some(123.0f32.to_bits()));
    }
    // G124 / G125 with the hit effects off: none / skip.
    super::super::gp::toggle_hit_effects(&mut e, false);
    assert_eq!(e.site(0x009a_cec6, &[]), Some(0));
    assert_eq!(e.site(0x009a_d2c5, &[0x0300_0000]), Some(1));
    // G101: a line that is no death response keeps the exe's speaker.
    assert_eq!(e.site(0x0098_4156, &[0x0300_0000, 7, 2, 2, 1]), Some(0));
    // G127: the roll (the entry point stub leaves 100; roll 99 explodes).
    e.register(APPLY_PERK_MODIFIERS, |_, _| Ret::default());
    e.register(GET_RANDOM_INT, |_, _| 99u32.into_ret());
    let mine = e.mem.alloc(0x100);
    assert_eq!(e.site(0x009c_3bd6, &[mine, 0x0300_0000]), Some(1));
}

/// Not installed (row on): no part-3 handler. Installed, row off: JIP's own (G42, G124, G125)
/// only. Installed, row on: all.
#[test]
fn part3_registered_by_their_bits() {
    let e = jip(false, true);
    for slot in PART3_FIXES.iter().chain(&PART3_JIP) {
        assert!(e.site_owners(slot.addr()).is_empty(), "{:x}", slot.addr());
    }
    let e = jip(true, false);
    for slot in PART3_FIXES {
        assert!(e.site_owners(slot.addr()).is_empty(), "{:x}", slot.addr());
    }
    for slot in PART3_JIP {
        assert_eq!(
            e.site_owners(slot.addr()),
            [Owner::Jip],
            "{:x}",
            slot.addr()
        );
    }
    let e = jip(true, true);
    for slot in PART3_FIXES.iter().chain(&PART3_JIP) {
        assert_eq!(
            e.site_owners(slot.addr()),
            [Owner::Jip],
            "{:x}",
            slot.addr()
        );
    }
}

thread_local! {
    /// The process slot +0x4B8's calls (the process, then its words) and the allies it says are
    /// on a dialogue package.
    static DIALOGUE_PACKAGE: std::cell::RefCell<(Vec<Vec<u32>>, Vec<u32>)> =
        const { std::cell::RefCell::new((Vec::new(), Vec::new())) };
}

/// The slot `IsOnDialoguePackage` points at in the test vtable.
const ON_DIALOGUE_PACKAGE_STUB: u32 = 0x0700_04b8;

/// An ally at `at` with a process (`+0x68`) whose vtable answers slot +0x4B8 from
/// [`DIALOGUE_PACKAGE`]; the neighbours of its life state and talking bytes set.
fn ally(e: &mut Engine, vtable: u32, at: [f32; 3]) -> u32 {
    let actor = e.mem.alloc(0x200);
    let process = e.mem.alloc(0x40);
    e.mem.set_u32(process, vtable);
    e.mem.set_u32(actor + 0x68, process);
    for (i, v) in at.into_iter().enumerate() {
        e.mem.set_f32(actor + POSITION + 4 * i as u32, v);
    }
    // Neighbours of the bytes JIP reads, set: a wider read would see them.
    e.mem.set_u8(actor + 0x109, 1);
    e.mem.set_u8(actor + 0x10a, 2);
    e.mem.set_u8(actor + 0x7c, 1);
    e.mem.set_u8(actor + 0x7e, 1);
    actor
}

/// An engine for G101: the dialogue-package slot stub and a vtable for the processes.
fn ally_engine() -> (Engine, u32) {
    let mut e = jip(true, true);
    let vtable = e.mem.alloc(0x500);
    e.mem
        .set_u32(vtable + IS_ON_DIALOGUE_PACKAGE, ON_DIALOGUE_PACKAGE_STUB);
    e.register(ON_DIALOGUE_PACKAGE_STUB, |_, a| {
        DIALOGUE_PACKAGE.with(|s| {
            let mut s = s.borrow_mut();
            s.0.push(a.to_vec());
            u32::from(s.1.contains(&a[1])).into_ret()
        })
    });
    DIALOGUE_PACKAGE.with(|s| *s.borrow_mut() = (Vec::new(), Vec::new()));
    (e, vtable)
}

/// The actor's combat allies list (`+0x130`: data `+4`, count `+8`).
fn allies(e: &mut Engine, actor: u32, members: &[u32], count: u32) {
    let list = e.mem.alloc(0x10);
    let data = e.mem.alloc(4 * members.len().max(1) as u32);
    for (i, m) in members.iter().enumerate() {
        e.mem.set_u32(data + 4 * i as u32, *m);
    }
    e.mem.set_u32(list + 4, data);
    e.mem.set_u32(list + 8, count);
    e.mem.set_u32(actor + COMBAT_ALLIES, list);
}

/// G101's test: only a death response (type 2, subtype 6, flag byte 1) with no controller and
/// an actor asks for an ally; anything else answers 0 without reading the allies.
#[test]
fn death_response_only_for_a_death_response_without_controller() {
    let (mut e, vtable) = ally_engine();
    let actor = ally(&mut e, vtable, [0.0; 3]);
    let near = ally(&mut e, vtable, [10.0, 0.0, 0.0]);
    allies(&mut e, actor, &[near], 1);
    // (actor, controller, type, subtype, flag, the answer)
    for (who, controller, kind, subtype, flag, answer) in [
        (actor, 0u32, 2u32, 6u32, 1u32, near),
        (actor, 0x0300_0000, 2, 6, 1, 0),
        (actor, 0, 4, 6, 1, 0),
        (actor, 0, 0x102, 6, 1, 0),
        (actor, 0, 2, 5, 1, 0),
        (actor, 0, 2, 0x106, 1, 0),
        (actor, 0, 2, 6, 0, 0),
        (actor, 0, 2, 6, 2, 0),
        (actor, 0, 2, 6, 0x101, near),
        (0, 0, 2, 6, 1, 0),
    ] {
        DIALOGUE_PACKAGE.with(|s| s.borrow_mut().0.clear());
        assert_eq!(
            e.site(
                DEATH_RESPONSE.addr(),
                &[who, controller, kind, subtype, flag]
            ),
            Some(answer),
            "{controller:x} {kind:x} {subtype:x} {flag:x} {who:x}"
        );
        let asked = DIALOGUE_PACKAGE.with(|s| s.borrow().0.len());
        assert_eq!(asked, usize::from(answer != 0));
    }
}

/// JIP's GetNearestLivingAlly: the nearest ally under 1024 units that is not the actor, has a
/// process, is not dying or dead (life state byte), not talking, not on a dialogue package
/// (the process's slot +0x4B8 asked with the ally); an equal distance keeps the first; an unordered
/// distance is taken; the count is signed.
#[test]
fn nearest_living_ally_as_jip() {
    let (mut e, vtable) = ally_engine();
    let actor = ally(&mut e, vtable, [100.0, 200.0, 0.0]);
    let at = |x: f32| [100.0 + x, 200.0, 0.0];
    let far = ally(&mut e, vtable, at(30.0));
    let nearer = ally(&mut e, vtable, at(-20.0));
    let no_process = ally(&mut e, vtable, at(1.0));
    e.mem.set_u32(no_process + 0x68, 0);
    let dying = ally(&mut e, vtable, at(2.0));
    e.mem.set_u8(dying + 0x108, 1);
    let dead = ally(&mut e, vtable, at(3.0));
    e.mem.set_u8(dead + 0x108, 2);
    let talking = ally(&mut e, vtable, at(4.0));
    e.mem.set_u8(talking + IS_TALKING, 1);
    let busy = ally(&mut e, vtable, at(5.0));
    DIALOGUE_PACKAGE.with(|s| s.borrow_mut().1.push(busy));
    let members = [
        0, actor, far, no_process, dying, dead, talking, busy, nearer,
    ];
    allies(&mut e, actor, &members, members.len() as u32);
    assert_eq!(nearest_living_ally(&mut e, actor), nearer);
    // Asked for those with a process, alive and not talking, each with itself.
    let asked = DIALOGUE_PACKAGE.with(|s| s.borrow().0.clone());
    let process = |e: &Engine, a: u32| e.mem.u32(a + 0x68);
    assert_eq!(
        asked,
        vec![
            vec![process(&e, far), far],
            vec![process(&e, busy), busy],
            vec![process(&e, nearer), nearer],
        ]
    );
    // A life state word with a set high byte is still alive (JIP reads the byte).
    e.mem.set_u32(dying + 0x108, 0x0100);
    assert_eq!(nearest_living_ally(&mut e, actor), dying);
    // The count reaches only the first entries; 0 and a negative count find none.
    allies(&mut e, actor, &members, 3);
    assert_eq!(nearest_living_ally(&mut e, actor), far);
    for count in [0, 0xffff_ffff, 0x8000_0001] {
        allies(&mut e, actor, &members, count);
        assert_eq!(nearest_living_ally(&mut e, actor), 0, "{count:x}");
    }
    // No list: none.
    e.mem.set_u32(actor + COMBAT_ALLIES, 0);
    assert_eq!(nearest_living_ally(&mut e, actor), 0);
    // Beyond 1024 none; of two at the same distance the first stays (`jnb`: equal is skipped).
    let beyond = ally(&mut e, vtable, at(2000.0));
    allies(&mut e, actor, &[beyond], 1);
    assert_eq!(nearest_living_ally(&mut e, actor), 0);
    let left = ally(&mut e, vtable, at(-30.0));
    allies(&mut e, actor, &[far, left], 2);
    assert_eq!(nearest_living_ally(&mut e, actor), far);
    allies(&mut e, actor, &[left, far], 2);
    assert_eq!(nearest_living_ally(&mut e, actor), left);
    // A NaN position is taken (`jnb` after an unordered `comiss`), and so is every later one.
    let lost = ally(&mut e, vtable, [f32::NAN, 0.0, 0.0]);
    let later = ally(&mut e, vtable, at(900.0));
    allies(&mut e, actor, &[nearer, lost, later], 3);
    assert_eq!(nearest_living_ally(&mut e, actor), later);
}

/// JIP's Point3Distance: 0 and NaN as they are; otherwise the square root through `rsqrtss` and
/// one Newton step (within 2^-21 relative of the exact root), from `(dx² + dy²) + dz²`, the
/// fourth word ignored.
#[test]
fn point3_distance_as_jip() {
    let mut e = jip(true, true);
    let a = e.mem.alloc(0x20);
    let b = e.mem.alloc(0x20);
    let put = |e: &mut Engine, at: u32, p: [f32; 4]| {
        for (i, v) in p.into_iter().enumerate() {
            e.mem.set_f32(at + 4 * i as u32, v);
        }
    };
    put(&mut e, a, [1.0, 2.0, 3.0, 99.0]);
    put(&mut e, b, [1.0, 2.0, 3.0, -7.0]);
    assert_eq!(point3_distance(&e, a, b).to_bits(), 0);
    put(&mut e, b, [1.0, f32::NAN, 3.0, 0.0]);
    assert!(point3_distance(&e, a, b).is_nan());
    for (p, q, exact) in [
        ([0.0, 0.0, 0.0], [3.0, 4.0, 0.0], 5.0f64),
        ([10.0, -2.0, 7.0], [1.0, 2.0, 3.0], 113.0f64.sqrt()),
        ([0.0, 0.0, 0.0], [0.0, 0.0, 1000.0], 1000.0),
    ] {
        put(&mut e, a, [p[0], p[1], p[2], 5.0]);
        put(&mut e, b, [q[0], q[1], q[2], 9.0]);
        let d = f64::from(point3_distance(&e, a, b));
        assert!(
            (d - exact).abs() <= exact * 2f64.powi(-21),
            "{p:?} {q:?} {d}"
        );
        // The order of the points does not matter.
        assert_eq!(point3_distance(&e, a, b), point3_distance(&e, b, a));
        // Bit for bit JIP's SSE sequence on this processor: `subps`, `mulps`, the two `haddps`
        // as (x + y) + (z + 0), `rsqrtss`, `mulss` x2, `subss` from 3, `mulss` r, `mulss` 0.5,
        // `mulss` by the sum (scalar SSE intrinsics, one rounding each).
        #[cfg(target_arch = "x86_64")]
        {
            use std::arch::x86_64::*;
            // SAFETY: SSE and SSE2 are part of every x86-64 processor.
            let reference = unsafe {
                let ss = |v: f32| _mm_set_ss(v);
                let d = |i: usize| _mm_sub_ss(ss(p[i]), ss(q[i]));
                let sq = |i: usize| _mm_mul_ss(d(i), d(i));
                let s = _mm_add_ss(_mm_add_ss(sq(0), sq(1)), _mm_add_ss(sq(2), ss(0.0)));
                let r = _mm_rsqrt_ss(s);
                let x1 = _mm_mul_ss(_mm_mul_ss(s, r), r);
                let x3 = _mm_mul_ss(_mm_mul_ss(_mm_sub_ss(ss(3.0), x1), r), ss(0.5));
                _mm_cvtss_f32(_mm_mul_ss(s, x3))
            };
            assert_eq!(
                point3_distance(&e, a, b).to_bits(),
                reference.to_bits(),
                "{p:?} {q:?}"
            );
        }
    }
}

type MineLog = (Vec<Vec<u32>>, f32, Vec<Vec<u32>>, u32);

thread_local! {
    /// G127's entry point calls (words and the chance before), the chance it leaves; the roll's
    /// calls and answer.
    static MINE: std::cell::RefCell<MineLog> =
        const { std::cell::RefCell::new((Vec::new(), 0.0, Vec::new(), 0)) };
}

/// G127: the chance 100 through entry point 4 with the actor and the mine's base (`+0x20`),
/// then `GetRandomInt(GAME_RNG, 100)`: below the chance rounded to nearest-even (unsigned
/// compare: a negative or NaN chance is huge) explodes.
#[test]
fn mine_explode_chance_rolls_as_jip() {
    let mut e = jip(true, true);
    e.register(APPLY_PERK_MODIFIERS, |e, a| {
        MINE.with(|m| {
            let mut m = m.borrow_mut();
            m.0.push(vec![a[0], a[1], a[2], e.mem.u32(a[3])]);
            e.mem.set_f32(a[3], m.1);
        });
        Ret::default()
    });
    e.register(GET_RANDOM_INT, |_, a| {
        MINE.with(|m| {
            let mut m = m.borrow_mut();
            m.2.push(a.to_vec());
            m.3.into_ret()
        })
    });
    let mine = e.mem.alloc(0x100);
    e.mem.set_u32(mine + 0x1c, 0x0500_0001);
    e.mem.set_u32(mine + 0x20, 0x0500_0002);
    e.mem.set_u32(mine + 0x24, 0x0500_0003);
    let actor = 0x0300_1234;
    // (the chance the entry point leaves, the roll, explodes)
    for (chance, roll, explode) in [
        (100.0f32, 99u32, 1u32),
        (100.0, 100, 0),
        (50.0, 49, 1),
        (50.0, 50, 0),
        (50.5, 50, 0),
        (51.5, 51, 1),
        (51.5, 52, 0),
        (49.6, 49, 1),
        (0.0, 0, 0),
        (-1.0, 99, 1),
        (f32::NAN, 99, 1),
    ] {
        MINE.with(|m| *m.borrow_mut() = (Vec::new(), chance, Vec::new(), roll));
        assert_eq!(
            e.site(MINE_EXPLODE_CHANCE.addr(), &[mine, actor]),
            Some(explode),
            "{chance} {roll}"
        );
        let (entry, _, rolls, _) = MINE.with(|m| m.take());
        assert_eq!(entry, vec![vec![4, actor, 0x0500_0002, 100.0f32.to_bits()]]);
        assert_eq!(rolls, vec![vec![0x011c_4180, 100]]);
    }
}

/// G42: JIP's distance (300.0 until a command changes it).
#[test]
fn move_away_distance_is_jips_variable() {
    let mut e = jip(true, false);
    for slot in MOVE_AWAY_DISTANCE {
        assert_eq!(e.site(slot.addr(), &[]), Some(300.0f32.to_bits()));
    }
    e.ext_free::<MoveAwayDistance>()
        .0
        .store(55.5f32.to_bits(), Ordering::Relaxed);
    for slot in MOVE_AWAY_DISTANCE {
        assert_eq!(e.site(slot.addr(), &[]), Some(55.5f32.to_bits()));
    }
}

/// G42's distance is one for the run: an engine sharing the slots (a real thread's) reads what
/// this one wrote through the handler, and the word it holds, read and written on a second OS
/// thread (an engine itself stays on its thread), is the one this engine's handler reads.
#[test]
fn move_away_distance_is_run_wide() {
    let mut e = jip(true, false);
    let mut other = engine();
    e.share_slots_for_test(&mut other);
    e.ext_free::<MoveAwayDistance>()
        .0
        .store(77.0f32.to_bits(), Ordering::Relaxed);
    assert_eq!(move_away_distance(&mut other, &[]), Some(77.0f32.to_bits()));
    let word = &other.ext_free::<MoveAwayDistance>().0;
    let seen = std::thread::scope(|s| {
        s.spawn(|| {
            let seen = word.load(Ordering::Relaxed);
            word.store(88.0f32.to_bits(), Ordering::Relaxed);
            seen
        })
        .join()
        .unwrap()
    });
    assert_eq!(seen, 77.0f32.to_bits());
    assert_eq!(move_away_distance(&mut e, &[]), Some(88.0f32.to_bits()));
}

/// G124 and G125 read `s_disableHitEffects` (byte): 0 gives the exe's image space and shake;
/// ToggleHitEffects 0 (bit 1) turns both off; ToggleHitEffects 1 keeps only bit 2.
#[test]
fn hit_effect_patches_follow_the_word() {
    let mut e = jip(true, false);
    let explosion = 0x0300_0040;
    assert_eq!(e.site(EXPLOSION_IN_FACE.addr(), &[]), None);
    assert_eq!(e.site(EXPLOSION_SHAKE.addr(), &[explosion]), Some(0));
    super::super::gp::toggle_hit_effects(&mut e, false);
    assert_eq!(super::super::gp::disable_hit_effects(&mut e), 1);
    assert_eq!(e.site(EXPLOSION_IN_FACE.addr(), &[]), Some(0));
    assert_eq!(e.site(EXPLOSION_SHAKE.addr(), &[explosion]), Some(1));
    super::super::gp::toggle_hit_effects(&mut e, true);
    assert_eq!(e.site(EXPLOSION_IN_FACE.addr(), &[]), None);
    assert_eq!(e.site(EXPLOSION_SHAKE.addr(), &[explosion]), Some(0));
}

/// With immortal mode 2 (`ToggleImmortalMode 2`: bit 2), the hit effects stay off whatever
/// ToggleHitEffects does: `ToggleHitEffects 0` gives 3 (`|= 1`), `ToggleHitEffects 1` gives 2
/// (`&= 2`), and G124 / G125 answer off for both.
#[test]
fn hit_effects_stay_off_in_immortal_mode_two() {
    use super::super::gp::{disable_hit_effects, toggle_hit_effects, TOGGLE_IMMORTAL_MODE};
    use crate::commands::test_support::{call_on, engine, n};
    let mut e = engine();
    e.map(0x011e_0000, 0x1000);
    let explosion = 0x0300_0040;
    call_on(&mut e, TOGGLE_IMMORTAL_MODE, 0, &[n(2)]);
    assert_eq!(disable_hit_effects(&mut e), 2);
    for (on, word) in [(false, 3), (true, 2), (true, 2), (false, 3)] {
        toggle_hit_effects(&mut e, on);
        assert_eq!(disable_hit_effects(&mut e), word, "{on}");
        assert_eq!(e.site(EXPLOSION_IN_FACE.addr(), &[]), Some(0), "{on}");
        assert_eq!(
            e.site(EXPLOSION_SHAKE.addr(), &[explosion]),
            Some(1),
            "{on}"
        );
    }
}
