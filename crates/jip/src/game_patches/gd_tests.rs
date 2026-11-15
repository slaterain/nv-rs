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
}

/// G92: a reference with any flag of 0x4820 is skipped without asking the script's pending
/// lists; else it is skipped when pending disabled.
#[test]
fn ai_target_skips_flagged_and_disabled_references() {
    let mut e = jip(true, true);
    e.register(PENDING_ENABLED, |_, _| {
        PENDING.with(|p| {
            let (en, dis, n) = p.get();
            p.set((en, dis, n + 1));
            en.into_ret()
        })
    });
    e.register(PENDING_DISABLED, |_, _| {
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
        assert_eq!(
            e.site(AI_TARGET.addr(), &[reference]),
            Some(skip),
            "{flags:x}"
        );
        assert_eq!(PENDING.with(|p| p.get().2), calls, "{flags:x}");
    }
    // JIP's GetDisabled with the disabled flag: disabled unless pending enabled.
    e.mem.set_u32(reference + 8, 0x800);
    for (en, dis, disabled) in [(0, 0, true), (1, 0, false), (1, 1, true)] {
        PENDING.with(|p| p.set((en, dis, 0)));
        assert_eq!(get_disabled(&mut e, reference), disabled, "{en} {dis}");
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
