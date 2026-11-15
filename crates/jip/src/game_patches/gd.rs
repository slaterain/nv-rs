//! Group GD's game patches (step 3.9: projectiles, explosions, combat AI, packages): its handlers, wraps and slot routines. Only group
//! GD's PR edits this file (and its own tests file beside it).
//!
//! **G117, a projectile's ammo, count and split beams** (`InitGamePatches`,
//! `internal/patches_game.h` (JIP LN NVSE): `WriteRelJump(0x9BC241, InitProjectileSetAmmoHook)`).
//! The launch constructor (`009bbef0`) divides the weapon's damage by the exe's projectile count
//! (`TESObjectWEAP::GetNumProjectiles` `00525b20`) and scales it by the double 1.3 at `01051678`
//! when the held weapon's mod effect 0xC (split beam) is active. JIP's hook replaces that block
//! (up to `0x9BC293`): it records the weapon's equipped ammo, the split-beam flag and its own
//! count in the projectile, scales by the float 1.3 at `01022004` first, then divides by its
//! count. A JIP fix (with the row `NV_JIP_GAME_PATCHES`): split-beam and multi-projectile damage
//! change for every player. Seam in `engine-fallout-ai` `projectile.rs` (`fn_009bbef0`).
//!
//! **The record** (`measure/jip/GROUP-GD-NOTES.md`, "G117 record for GC's O1"): kept where JIP keeps
//! it, in the projectile's own bytes ([`AMMO`], [`HAS_SPLIT_BEAMS`], [`NUM_PROJECTILES`]; the
//! exe never touches them). G117 is the only writer; GC's O1 reads it through [`projectile_ammo`]
//! and the two bytes, GC's G119 reads the [`AMMO`] word directly. A projectile G117 did not set up
//! (JIP not installed, the plain constructor of a loaded projectile, no weapon) reads all zero:
//! the port's guest heap zero-fills every block (on the game's heap JIP reads whatever the two
//! bytes held for a loaded projectile).
//!
//! Threads: the constructor runs wherever a weapon fires (the main thread, the AI threads with real
//! threads on). The handler writes only the new projectile and reads forms and the shooter's
//! process; it keeps no state of JIP's and takes no slot.
//!
//! **Part 2, fixes of `InitGamePatches` (with the row) and the switch `bProjImpactDmgFix`:**
//! - G27 (`SAFE_WRITE_BUF(0x47667D / 0x9C35F7, "\x89\xC8\x0F\x1F\x00")`): `mov eax, ecx` in place of
//!   the shooter call where a destroyed projectile-like reference (`fn_00475b20`) or a projectile
//!   (its explosion) makes an explosion: the explosion's source is the projectile, which the
//!   explosion factory already reads its shooter from (so the shooter's perk entry sees the
//!   projectile's weapon).
//! - G30 (`SAFE_WRITE_BUF(0x9ACA23, ...)`, 53 bytes): in the explosion factory (`009ac9c0`), for a
//!   projectile source, the shooter is read at `+0xFC` and the "adjust explosion radius" entry
//!   point (0x48) runs for any actor shooter with the projectile's weapon (`+0xF8`), not only for
//!   the player.
//! - G62 (`PackageSetRunHook`, 0x8DAA6F): the move mode honours a package's Always Walk flag
//!   (JIP's bit 0x20000000) before the run tests.
//! - G91 (`ReactionCooldownCheckHook`, 0x9D10F3): ranged combat's reaction line waits the
//!   setting's seconds of real time (`GetTickCount`) since the last line of any actor.
//! - G92 (`IsValidAITargetHook`, 0x9F50C4): sandbox never picks a disabled (or pending
//!   disabled) reference.
//! - G102 (`SAFE_WRITE_BUF(0x9EE367, ...)`): a dialogue package's speaker ends its line only when
//!   dead (life state 2).
//! - O4 (`InitMissileFlagsHook`, 0x9B7E14, switch `bProjImpactDmgFix`): a missile's flag 0x8000
//!   (impact damage not taken again from the weapon) is set only for a projectile base with flag
//!   0x100, and never cleared.
//!
//! Threads: G27, G30, G62, G92, G102 and O4 read and write only the objects they are given (their
//! functions run on the AI threads too). G91 keeps JIP's one static, the last line's tick count:
//! a run-wide atomic ([`ReactionTick`], lock-free, the shared slot is never taken), read and
//! written as JIP's plain loads and stores are (two threads may both pass in one tick, as with JIP).

use super::Area;
use crate::commands::forms::TES_AMMO_VTBL;
use engine_core::extend::{Owner, SiteSlot};
use engine_core::prelude::*;
use std::sync::atomic::{AtomicU32, Ordering};

/// G117's seam: the launch constructor's count block (words: the projectile, the weapon
/// argument, the actor shooter or 0, the mod-0xC flag).
pub const PROJECTILE_SET_AMMO: SiteSlot = SiteSlot::new(Owner::Jip, 0x009b_c241);
/// G27's seams: the explosion source in `fn_00475b20` (`0x47667D`) and in the projectile's own
/// explosion (`0x9C35F7`); words: the reference.
pub const EXPLOSION_SOURCE: [SiteSlot; 2] = [
    SiteSlot::new(Owner::Jip, 0x0047_667d),
    SiteSlot::new(Owner::Jip, 0x009c_35f7),
];
/// G30's seam: the explosion factory's projectile source (words: the source, the multiplier's
/// bits; answers the multiplier's bits).
pub const EXPLOSION_PERK: SiteSlot = SiteSlot::new(Owner::Jip, 0x009a_ca23);
/// G62's seam: the move mode's run tests (words: the actor, the running package; answers the mode
/// or 0 to go on).
pub const MOVE_MODE: SiteSlot = SiteSlot::new(Owner::Jip, 0x008d_aa6f);
/// G91's seam: ranged combat's reaction cooldown test (words: the combat state).
pub const REACTION_COOLDOWN: SiteSlot = SiteSlot::new(Owner::Jip, 0x009d_10f3);
/// G92's seam: the sandbox target's flag tests (words: the reference; answers the skip byte).
pub const AI_TARGET: SiteSlot = SiteSlot::new(Owner::Jip, 0x009f_50c4);
/// G102's seam: the speaker's state calls (words: the speaker; answers "dead").
pub const SPEAKER_DONE: SiteSlot = SiteSlot::new(Owner::Jip, 0x009e_e367);
/// O4's seam: the missile's flag 0x8000 call (words: the projectile, the value).
pub const MISSILE_FLAGS: SiteSlot = SiteSlot::new(Owner::Jip, 0x009b_7e14);

/// The group's rows and installers.
pub const AREA: Area = Area {
    name: "GD",
    fix_sites: &[
        PROJECTILE_SET_AMMO.addr(),
        EXPLOSION_SOURCE[0].addr(),
        EXPLOSION_SOURCE[1].addr(),
        EXPLOSION_PERK.addr(),
        MOVE_MODE.addr(),
        REACTION_COOLDOWN.addr(),
        AI_TARGET.addr(),
        SPEAKER_DONE.addr(),
        MISSILE_FLAGS.addr(),
    ],
    fix_wraps: &[],
    install_fixes,
    jip_sites: &[],
    jip_wraps: &[],
    install_jip: super::nothing,
};

/// The record's three places in a projectile (JIP LN NVSE's `Projectile`, GameObjects.h): the
/// ammo word in the reference's extra data list (`extraDataList.ammo`, list `+0x44` + `0x1C`), JIP's
/// `hasSplitBeams` and `numProjectiles` bytes (padding after the exe's `bSoundFired` `+0x148`).
pub const AMMO: u32 = 0x60;
pub const HAS_SPLIT_BEAMS: u32 = 0x149;
pub const NUM_PROJECTILES: u32 = 0x14a;
/// The projectile's damage (`fDamage`, JIP's `hitDamage`).
const HIT_DAMAGE: u32 = 0xdc;

/// `TESObjectWEAP` (JIP LN NVSE, GameForms.h): its ammo form (`BGSAmmoForm::ammo`, a `TESAmmo` or a
/// form list), `ammoUse`, `numProjectiles`, the three mod effects and their first values.
const WEAPON_AMMO: u32 = 0xa8;
const WEAPON_AMMO_USE: u32 = 0x102;
const WEAPON_NUM_PROJECTILES: u32 = 0x11e;
const WEAPON_EFFECT_MODS: u32 = 0x180;
const WEAPON_VALUE1_MODS: u32 = 0x18c;
/// The split-beam mod effect.
const SPLIT_BEAM_EFFECT: u32 = 0xc;
/// `TESAmmo::projPerShot` (JIP LN NVSE), read as a signed byte.
const AMMO_PROJ_PER_SHOT: u32 = 0xb0;
/// `kFormType_BGSListForm`; a form list's first node (`+0x18`) and the count JIP walks (`+0x20`).
const LIST_FORM_TYPE: u8 = 0x55;
const LIST_HEAD: u32 = 0x18;
const LIST_WALK: u32 = 0x20;
/// The float JIP scales split-beam damage by (1.3, in the exe's constants).
pub const SPLIT_BEAM_SCALE: u32 = 0x0102_2004;
/// `Actor::baseProcess` and, in a middle or high process (level 0 or 1 at `+0x28`), the weapon
/// and ammo entries (`ContChangesEntry*`, their form at `+8`).
const BASE_PROCESS: u32 = 0x68;
const PROCESS_LEVEL: u32 = 0x28;
const PROCESS_WEAPON_INFO: u32 = 0x114;
const PROCESS_AMMO_INFO: u32 = 0x118;
const ENTRY_FORM: u32 = 8;

fn install_fixes(e: &mut Engine) {
    e.on_site(PROJECTILE_SET_AMMO, projectile_set_ammo);
    for slot in EXPLOSION_SOURCE {
        e.on_site(slot, explosion_source);
    }
    e.on_site(EXPLOSION_PERK, explosion_perk);
    e.on_site(MOVE_MODE, move_mode);
    // G91's run-wide tick, made before the threads start (`ext_free`'s rule).
    e.ext_free::<ReactionTick>();
    e.on_site(REACTION_COOLDOWN, reaction_cooldown);
    e.on_site(AI_TARGET, ai_target);
    e.on_site(SPEAKER_DONE, speaker_done);
    e.on_site(MISSILE_FLAGS, missile_flags);
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_game.h: InitGamePatches
// (SAFE_WRITE_BUF(0x47667D, "\x89\xC8\x0F\x1F\x00"), the same at 0x9C35F7).
/// G27: `mov eax, ecx` in place of the shooter call: the reference itself.
fn explosion_source(_: &mut Engine, a: &[u32]) -> Option<u32> {
    Some(a[0])
}

/// The function an actor's vtable holds at `+0x100` (`IsActor`, answering true), which JIP's G30
/// compares the shooter's slot with.
pub const ACTOR_IS_ACTOR: u32 = 0x008d_0360;
/// `BGSEntryPointPerkEntry::HandleEntryPoint` (`ADDR_ApplyPerkModifiers`; cdecl: the entry point,
/// the actor, a form, the float it adjusts).
const APPLY_PERK_MODIFIERS: u32 = 0x005e_58f0;
/// `kEntryPoint_AdjustExplosionRadius`.
const ADJUST_EXPLOSION_RADIUS: u32 = 0x48;

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_game.h: InitGamePatches
// (SAFE_WRITE_BUF(0x9ACA23, "\x8B\x8A\xFC\x00\x00\x00\x89\x4D\xE8...")).
/// G30: the projectile source's shooter (`+0xFC`); for an actor shooter (its vtable's `+0x100` is
/// `Actor::IsActor`) the "adjust explosion radius" entry point adjusts the multiplier with the
/// projectile's weapon (`+0xF8`). Answers the multiplier's bits (the factory reads the shooter
/// at `+0xFC` itself, JIP's `[ebp-0x18]`).
fn explosion_perk(e: &mut Engine, a: &[u32]) -> Option<u32> {
    let (source, mut multiplier) = (a[0], a[1]);
    let shooter = e.mem.u32(source.wrapping_add(0xfc));
    if shooter != 0 && e.mem.u32(e.mem.u32(shooter).wrapping_add(0x100)) == ACTOR_IS_ACTOR {
        let weapon = e.mem.u32(source.wrapping_add(0xf8));
        e.with_stack(4, |e, out| {
            e.mem.set_u32(out.addr(), multiplier);
            e.call(
                APPLY_PERK_MODIFIERS,
                &args![ADJUST_EXPLOSION_RADIUS, shooter, weapon, out],
            );
            multiplier = e.mem.u32(out.addr());
        });
    }
    Some(multiplier)
}

/// A package's flag word (`+0x1C`): `kPackageFlag_AlwaysRun` and JIP's `kPackageFlag_AlwaysWalk`.
const ALWAYS_RUN: u32 = 1 << 13;
const ALWAYS_WALK: u32 = 1 << 29;
/// The actor's bytes the exe's `008d8220` and `00493bb0` answer (`+0x124`, `+0x104`).
const ACTOR_BYTE_124: u32 = 0x124;
const ACTOR_BYTE_104: u32 = 0x104;

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_game.h: PackageSetRunHook.
/// G62: walk (0x101) for a package with the Always Walk flag; run (0x201) for one with Always
/// Run, else when the actor's byte `+0x124` or `+0x104` is set; else 0 (go on at `0x8DAAA9`).
fn move_mode(e: &mut Engine, a: &[u32]) -> Option<u32> {
    let (actor, package) = (a[0], a[1]);
    if package != 0 {
        let flags = e.mem.u32(package.wrapping_add(0x1c));
        if flags & ALWAYS_WALK != 0 {
            return Some(0x101);
        }
        if flags & ALWAYS_RUN != 0 {
            return Some(0x201);
        }
    }
    if e.mem.u8(actor.wrapping_add(ACTOR_BYTE_124)) != 0
        || e.mem.u8(actor.wrapping_add(ACTOR_BYTE_104)) != 0
    {
        return Some(0x201);
    }
    Some(0)
}

/// G91's `lastTickCount` (JIP's static in `ReactionCooldownCheckHook`): one for the run, every
/// thread's engine reading the same word without a lock.
#[derive(Default)]
pub struct ReactionTick(pub AtomicU32);

/// The exe's `GetTickCount` wrapper (`00457fe0`, no argument).
const TICK_COUNT: u32 = 0x0045_7fe0;
/// The setting JIP compares with (`0x11CE9B4`'s value).
const REACTION_COOLDOWN_SETTING: u32 = 0x011c_e9b8;

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_game.h: ReactionCooldownCheckHook.
/// G91: whether more than the setting's seconds passed since the last line (the tick difference
/// as a signed int, times 0.001, `comiss`/`seta`); when so the tick is kept.
fn reaction_cooldown(e: &mut Engine, _: &[u32]) -> Option<u32> {
    let now = e.call(TICK_COUNT, &[]).u32();
    let last = e.ext_free::<ReactionTick>().0.load(Ordering::Relaxed);
    let seconds = now.wrapping_sub(last) as i32 as f32 * 0.001f32;
    let ready = seconds > e.mem.f32(REACTION_COOLDOWN_SETTING);
    if ready {
        e.ext_free::<ReactionTick>().0.store(now, Ordering::Relaxed);
    }
    Some(u32::from(ready))
}

/// `Script::IsPendingEnabledReference` / `IsPendingDisabledReference` (cdecl, the reference).
const PENDING_ENABLED: u32 = 0x005a_a680;
const PENDING_DISABLED: u32 = 0x005a_a630;

// Translated from JIP LN NVSE 57.30 (5a30ac4), nvse/GameObjects.cpp: TESObjectREFR::GetDisabled.
/// JIP's disabled test: with the disabled flag (`+8` bit 0x800), disabled unless pending enabled;
/// else pending disabled.
pub fn get_disabled(e: &mut Engine, reference: u32) -> bool {
    if e.mem.u8(reference.wrapping_add(9)) & 8 != 0
        && !e.call(PENDING_ENABLED, &args![reference]).bool()
    {
        return true;
    }
    e.call(PENDING_DISABLED, &args![reference]).bool()
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_game.h: IsValidAITargetHook.
/// G92: skip a reference with a flag of 0x4820 (deleted 0x20, 0x800, 0x4000), else a disabled
/// one ([`get_disabled`]).
fn ai_target(e: &mut Engine, a: &[u32]) -> Option<u32> {
    let reference = a[0];
    if e.mem.u32(reference.wrapping_add(8)) & 0x4820 != 0 {
        return Some(1);
    }
    Some(u32::from(get_disabled(e, reference)))
}

/// `Actor::lifeState` (`+0x108`) and the dead state.
const LIFE_STATE: u32 = 0x108;
const DEAD: u32 = 2;

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_game.h: InitGamePatches
// (SAFE_WRITE_BUF(0x9EE367, "\x8B\x8A\xB0\x00\x00\x00\x83\xB9\x08\x01\x00\x00\x02...")).
/// G102: whether the speaker's life state is dead (the exe's two state calls not made).
fn speaker_done(e: &mut Engine, a: &[u32]) -> Option<u32> {
    Some(u32::from(e.mem.u32(a[0].wrapping_add(LIFE_STATE)) == DEAD))
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_game.h: InitMissileFlagsHook.
/// O4: with the value set and the projectile's base (`+0x20`) flag byte `+0x61` bit 1 (flag
/// 0x100), the projectile's flag 0x8000 (`or byte [ecx+0xC9], 0x80`); nothing cleared.
fn missile_flags(e: &mut Engine, a: &[u32]) -> Option<u32> {
    let (projectile, value) = (a[0], a[1]);
    if value & 0xff != 0 {
        let base = e.mem.u32(projectile.wrapping_add(0x20));
        if e.mem.u8(base.wrapping_add(0x61)) & 1 != 0 {
            let flags = projectile.wrapping_add(0xc9);
            let byte = e.mem.u8(flags);
            e.mem.set_u8(flags, byte | 0x80);
        }
    }
    Some(0)
}

/// JIP's `cvttss2si`: truncated toward zero; NaN or out of range gives `0x80000000`.
fn cvttss2si(value: f32) -> i32 {
    if value.is_nan() || !(-2_147_483_648.0..2_147_483_648.0).contains(&value) {
        i32::MIN
    } else {
        value as i32
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), nvse/GameForms.cpp: TESObjectWEAP::GetAmmo.
/// The weapon's ammo: its ammo form, or for a form list the item at the list's `+0x20` count
/// from the head when the list is that long, else the first item.
pub fn weapon_ammo(e: &Engine, weapon: u32) -> u32 {
    let form = e.mem.u32(weapon.wrapping_add(WEAPON_AMMO));
    if form == 0 || e.mem.u8(form.wrapping_add(4)) != LIST_FORM_TYPE {
        return form;
    }
    let mut node = form.wrapping_add(LIST_HEAD);
    let mut count = e.mem.u32(form.wrapping_add(LIST_WALK)) as i32;
    loop {
        count = count.wrapping_sub(1);
        if count < 0 {
            return e.mem.u32(node);
        }
        node = e.mem.u32(node.wrapping_add(4));
        if node == 0 {
            return e.mem.u32(form.wrapping_add(LIST_HEAD));
        }
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), nvse/GameForms.cpp: TESObjectWEAP::GetEquippedAmmo.
/// The ammo `actor` has loaded in `weapon` (its middle or high process's ammo entry, when its
/// weapon entry is this weapon and the entry's form is a `TESAmmo`), else [`weapon_ammo`]. JIP
/// first answers 0 for its Rock-It Launcher (`g_rockItLauncher`, set only by
/// `InitRockItLauncher`, not built: null in the port, so never this weapon).
pub fn equipped_ammo(e: &Engine, weapon: u32, actor: u32) -> u32 {
    if actor != 0 {
        let process = e.mem.u32(actor.wrapping_add(BASE_PROCESS));
        if process != 0 && e.mem.u8(process.wrapping_add(PROCESS_LEVEL)) <= 1 {
            let weapon_info = e.mem.u32(process.wrapping_add(PROCESS_WEAPON_INFO));
            if weapon_info != 0 && e.mem.u32(weapon_info.wrapping_add(ENTRY_FORM)) == weapon {
                let ammo_info = e.mem.u32(process.wrapping_add(PROCESS_AMMO_INFO));
                if ammo_info != 0 {
                    let ammo = e.mem.u32(ammo_info.wrapping_add(ENTRY_FORM));
                    if ammo != 0 && e.mem.u32(ammo) == TES_AMMO_VTBL {
                        return ammo;
                    }
                }
            }
        }
    }
    weapon_ammo(e, weapon)
}

/// JIP's `Projectile::GetAmmo` (GameObjects.h): the recorded ammo when the recorded count is not
/// 0, else 0. For GC's O1.
pub fn projectile_ammo(e: &Engine, projectile: u32) -> u32 {
    if e.mem.u8(projectile.wrapping_add(NUM_PROJECTILES)) != 0 {
        e.mem.u32(projectile.wrapping_add(AMMO))
    } else {
        0
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_game.h: InitProjectileSetAmmoHook.
/// The launch constructor's count block (`0x9BC241`; the projectile and the weapon never 0 here,
/// the block is inside the constructor's `weapon != 0` branch): records the equipped ammo and the
/// split-beam flag, counts the projectiles (the ammo's projectiles per shot times the weapon's
/// ammo use when that signed byte is above 0, else the weapon's count; plus the split-beam mod's
/// value), scales the damage by 1.3 with split beams and divides it by the count when the count's
/// low byte is above 1. Answers 0: the exe's block is skipped.
fn projectile_set_ammo(e: &mut Engine, a: &[u32]) -> Option<u32> {
    let (projectile, weapon, actor, split) = (a[0], a[1], a[2], a[3] & 0xff != 0);
    let ammo = equipped_ammo(e, weapon, actor);
    e.mem.set_u32(projectile.wrapping_add(AMMO), ammo);
    let mut damage = e.mem.f32(projectile.wrapping_add(HIT_DAMAGE));
    let mut count = u32::from(e.mem.u8(weapon.wrapping_add(WEAPON_NUM_PROJECTILES)));
    if ammo != 0 {
        let per_shot = i32::from(e.mem.u8(ammo.wrapping_add(AMMO_PROJ_PER_SHOT)) as i8);
        if per_shot > 0 {
            let ammo_use = i32::from(e.mem.u8(weapon.wrapping_add(WEAPON_AMMO_USE)));
            count = ammo_use.wrapping_mul(per_shot) as u32;
        }
    }
    e.mem
        .set_u8(projectile.wrapping_add(HAS_SPLIT_BEAMS), u8::from(split));
    if split {
        damage *= e.mem.f32(SPLIT_BEAM_SCALE);
        let effects = weapon.wrapping_add(WEAPON_EFFECT_MODS);
        if let Some(slot) =
            (0..3u32).find(|i| e.mem.u32(effects.wrapping_add(4 * i)) == SPLIT_BEAM_EFFECT)
        {
            let value = e
                .mem
                .f32(weapon.wrapping_add(WEAPON_VALUE1_MODS + 4 * slot));
            count = count.wrapping_add(cvttss2si(value) as u32);
        }
    }
    e.mem
        .set_u8(projectile.wrapping_add(NUM_PROJECTILES), count as u8);
    if count as u8 > 1 {
        damage /= count as i32 as f32;
    }
    e.mem.set_f32(projectile.wrapping_add(HIT_DAMAGE), damage);
    Some(0)
}

#[cfg(test)]
#[path = "gd_tests.rs"]
mod tests;
