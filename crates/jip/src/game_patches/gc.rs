//! Group GC's game patches (step 3.9: hit damage, detection, combat formulas): its handlers, wraps and slot routines. Only group
//! GC's PR edits this file (and its own tests file beside it).
//!
//! **G53, detection** (`InitGamePatches`, `internal/patches_game.h` (JIP LN NVSE):
//! `DetectionTeammateHook`, `DetectionGetLevelHook`): part of JIP once it answers "installed"
//! (it only reads the actor flags JIP's commands set, and with none set answers as the exe).
//! `Actor::GetDetectionLevelAgainstActor` (`008a0d10`) asks three times whether the detected actor
//! is the player's teammate (`0056_6950`: the byte `+0x18D`; a teammate takes the player's stealth
//! and sneak values, and one out of combat while the player moves is not detected at all). JIP
//! answers "no" for an actor whose detection it changes: `ForceActorDetectionValue`'s flag
//! (`+0x106` bit 0) or `ToggleDetectionFix`'s (`+0x105` bit 4); otherwise the byte, read directly.
//! The level word of the detected actor (its actor value owner's virtual `+0x28`, a formula input)
//! is read from the player's owner instead for an actor with the detection fix flag.
//! Seams in `engine-fallout-ai` `actor_p3.rs` (`detection_teammate`, `detection_level_source`).
//!
//! **The group's fixes** (the row `NV_JIP_GAME_PATCHES`; `InitGamePatches`): G14 (no un-equip
//! notice for the worn item equipping replaces), G18 (a change of life state is saved, so the dead
//! do not load in their bind pose), G77 (the armour-penetrated flag on a hit), G98 (kill challenges
//! skip teammates, flagged and essential actors), G109 + G120 (impact data with JIP's fallback rule,
//! the hit blood from the hit's own weapon), G111 + O11 (the damage a shot does to the weapon: the
//! weapon's own value, ammo effects, and with `bFO3WpnDegradation` Fallout 3's skill and rate
//! scaling), G118 (on-hit weapon effects from the projectile's own weapon), O19
//! (`bCreatureSpreadFix`). **G33** is JIP's own (`GetKiller`): the player's killer is kept, and
//! cleared before a load.

use super::Area;
use crate::game::JipActorFlags;
use engine_core::extend::{jip_patches, Owner, SiteSlot, WrapSlot};
use engine_core::prelude::*;
use nvse::commands::Read;
use nvse::{handled, CommandArgs};

/// G53's seams: the three teammate tests and the level word.
pub const DETECTION_TEAMMATE: [SiteSlot; 3] = [
    SiteSlot::new(Owner::Jip, 0x008a_0dab),
    SiteSlot::new(Owner::Jip, 0x008a_1291),
    SiteSlot::new(Owner::Jip, 0x008a_131d),
];
pub const DETECTION_LEVEL: SiteSlot = SiteSlot::new(Owner::Jip, 0x008a_147c);

/// The group's rows and installers.
pub const AREA: Area = Area {
    name: "GC",
    fix_sites: &[
        EQUIP_NO_SOUND.addr(),
        HIT_BLOOD_IMPACT.addr(),
        KILL_CHALLENGE.addr(),
        LIFE_STATE_MARK.addr(),
        CREATURE_SPREAD.addr(),
        PROJECTILE_WEAPON.addr(),
        ARMOR_PENETRATED.addr(),
    ],
    fix_wraps: &[GET_IMPACT_DATA.addr(), GET_DAMAGE_TO_WEAPON.addr()],
    install_fixes,
    jip_sites: &[
        PLAYER_KILLER_SET.addr(),
        PLAYER_KILLER_KEPT.addr(),
        DETECTION_TEAMMATE[0].addr(),
        DETECTION_TEAMMATE[1].addr(),
        DETECTION_TEAMMATE[2].addr(),
        DETECTION_LEVEL.addr(),
    ],
    jip_wraps: &[],
    install_jip,
};

/// `kHookActorFlags_DetectionModified` (`internal/hooks.h` (JIP LN NVSE)), tested as JIP's
/// `test word ptr [ecx+0x105]`: `kHookActorFlag1_DetectionFix` (`+0x105` bit 4) and
/// `kHookActorFlag2_ForceDetectionVal` (`+0x106` bit 0).
pub const DETECTION_MODIFIED: u16 = 0x0110;
/// `kHookActorFlag1_DetectionFix`.
pub const DETECTION_FIX: u8 = 1 << 4;
/// `g_thePlayer`'s word in the exe, and `Actor::bIsTeammate` (the byte `0056_6950` answers).
const THE_PLAYER: u32 = 0x011d_ea3c;
const IS_TEAMMATE: u32 = 0x18d;
/// The actor value owner inside an actor (`Actor`'s `ActorValueOwner` base).
const AV_OWNER: u32 = 0xa4;

fn install_jip(e: &mut Engine) {
    for slot in DETECTION_TEAMMATE {
        e.on_site(slot, detection_teammate);
    }
    e.on_site(DETECTION_LEVEL, detection_level);
    e.on_site(PLAYER_KILLER_SET, player_killer);
    e.on_site(PLAYER_KILLER_KEPT, player_killer);
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_game.h: DetectionTeammateHook.
/// A teammate test (the detected actor): 0 when JIP changes the actor's detection, else the
/// actor's teammate byte (JIP reads it in place of the call).
fn detection_teammate(e: &mut Engine, a: &[u32]) -> Option<u32> {
    let actor = a[0];
    let flags = u16::from(e.get(
        Ptr::<JipActorFlags>::new(actor),
        JipActorFlags::jipActorFlags1,
    )) | u16::from(e.get(
        Ptr::<JipActorFlags>::new(actor),
        JipActorFlags::jipActorFlags2,
    )) << 8;
    if flags & DETECTION_MODIFIED != 0 {
        return Some(0);
    }
    Some(u32::from(e.mem.u8(actor.wrapping_add(IS_TEAMMATE))))
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_game.h: DetectionGetLevelHook.
/// The level word's owner (the detected actor): the player's actor value owner when the actor
/// has the detection fix flag (the function still read from the actor's own table), else the
/// exe's call. JIP takes `g_thePlayer + 0xA4` with no test; with no player (never while actors
/// detect) the port keeps the exe's call.
fn detection_level(e: &mut Engine, a: &[u32]) -> Option<u32> {
    let actor = a[0];
    let flags = e.get(
        Ptr::<JipActorFlags>::new(actor),
        JipActorFlags::jipActorFlags1,
    );
    let player = e.mem.u32(THE_PLAYER);
    (flags & DETECTION_FIX != 0 && player != 0).then(|| player.wrapping_add(AV_OWNER))
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_actor.h:
// Cmd_ToggleDetectionFix_Execute.
/// `ToggleDetectionFix enable` (opcode `0x275C`, listed in `commands::jip_fn_actor`): on an actor,
/// sets (any value but 0) or clears its detection fix flag when that differs; read only by G53.
pub fn toggle_detection_fix(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let actor = a.this_obj;
    if !crate::commands::is_actor(e, actor) {
        return handled(true);
    }
    let read = Read::of(e, &a);
    if !read.ok {
        return handled(true);
    }
    let enable = read.int(0, 0) != 0;
    let flags = e.get(
        Ptr::<JipActorFlags>::new(actor),
        JipActorFlags::jipActorFlags1,
    );
    if enable != (flags & DETECTION_FIX != 0) {
        e.set(
            Ptr::<JipActorFlags>::new(actor),
            JipActorFlags::jipActorFlags1,
            flags ^ DETECTION_FIX,
        );
    }
    handled(true)
}

// ---- PR 2: the group's fixes (the row `NV_JIP_GAME_PATCHES`), O19 (its switch) and G33 (JIP's own).

/// G14's seam: `fn_0088db20`'s `UnEquipObject` of a worn item it takes off (`0x88DCDC`).
pub const EQUIP_NO_SOUND: SiteSlot = SiteSlot::new(Owner::Jip, 0x0088_dcdc);
/// G120's seam: the hit blood's impact data in `fn_0088e8d0` (`0x88EBAA`).
pub const HIT_BLOOD_IMPACT: SiteSlot = SiteSlot::new(Owner::Jip, 0x0088_ebaa);
/// G98's seam: `Actor::Kill`'s teammate test before the kill challenges (`0x89DC0E`).
pub const KILL_CHALLENGE: SiteSlot = SiteSlot::new(Owner::Jip, 0x0089_dc0e);
/// G33's seams: `Actor::Kill` keeps the player's killer (`0x89E790`, `0x89F351`).
pub const PLAYER_KILLER_SET: SiteSlot = SiteSlot::new(Owner::Jip, 0x0089_e790);
pub const PLAYER_KILLER_KEPT: SiteSlot = SiteSlot::new(Owner::Jip, 0x0089_f351);
/// G18's seam: `Actor::SetLifeState`'s change mark (`0x8A19CF`).
pub const LIFE_STATE_MARK: SiteSlot = SiteSlot::new(Owner::Jip, 0x008a_19cf);
/// O19's seam: the gun skill's iron sights word in `fn_008b0dd0` (`0x8B0FBF`).
pub const CREATURE_SPREAD: SiteSlot = SiteSlot::new(Owner::Jip, 0x008b_0fbf);
/// G118's seam: the weapon test in `fn_008c2190` (`0x8C227F`).
pub const PROJECTILE_WEAPON: SiteSlot = SiteSlot::new(Owner::Jip, 0x008c_227f);
/// G77's seam: the minimum damage store in `HitData::ReduceDamage` (`0x9B6335`).
pub const ARMOR_PENETRATED: SiteSlot = SiteSlot::new(Owner::Jip, 0x009b_6335);
/// G109's wrap: `BGSImpactDataSet`'s impact data of a material (`0058e9d0`).
pub const GET_IMPACT_DATA: WrapSlot = WrapSlot::new(Owner::Jip, 0x0058_e9d0);
/// G111's wrap: the damage a shot does to the weapon (`00646260`).
pub const GET_DAMAGE_TO_WEAPON: WrapSlot = WrapSlot::new(Owner::Jip, 0x0064_6260);

fn install_fixes(e: &mut Engine) {
    e.on_site(EQUIP_NO_SOUND, equip_no_sound);
    e.on_site(HIT_BLOOD_IMPACT, hit_blood_impact);
    e.on_site(KILL_CHALLENGE, kill_challenge_skips);
    e.on_site(LIFE_STATE_MARK, life_state_mark);
    e.on_site(CREATURE_SPREAD, creature_spread);
    e.on_site(PROJECTILE_WEAPON, projectile_weapon_first);
    e.on_site(ARMOR_PENETRATED, armor_penetrated);
    e.wrap(GET_IMPACT_DATA, get_impact_data);
    e.wrap(GET_DAMAGE_TO_WEAPON, get_damage_to_weapon);
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_game.h: InitGamePatches
// (SafeWrite8(0x88DCDC, 0)).
/// G14: the worn item taken off for the new one is taken off with `UnEquipObject`'s last word 0
/// (the exe's `push 1`): no un-equip notice (`0x8ADED0`) for it.
fn equip_no_sound(_: &mut Engine, _: &[u32]) -> Option<u32> {
    Some(0)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_game.h: InitGamePatches
// (SafeWrite8(0x8A19CF, 4)).
/// G18: a change of life state marks the actor with `0x404` (the exe's `push 0x400`), so the
/// state is saved with it and a dead actor does not load standing in its bind pose.
fn life_state_mark(_: &mut Engine, _: &[u32]) -> Option<u32> {
    Some(0x404)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_game.h: InitGamePatches
// (SafeWrite16(0x8C227F, 0x9066)).
/// G118: the `jnz` over the projectile's weapon is gone: any answer makes the translation take
/// the projectile's own weapon (`Projectile::sourceWeap`, `0x476C90`) whenever it has one.
fn projectile_weapon_first(_: &mut Engine, _: &[u32]) -> Option<u32> {
    Some(1)
}

/// `HitData::uiFlags`' top byte (`+0x5B`) and JIP's `kFlag_ArmorPenetrated` in it.
const HIT_FLAGS_TOP: u32 = 0x5b;
const ARMOR_PENETRATED_BIT: u8 = 0x80;

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_game.h: InitGamePatches
// (SAFE_WRITE_BUF(0x9B6335, ...): `or byte ptr [ecx+0x5B], 0x80` before the store).
/// G77: when the minimum damage replaces the hit's health damage, the hit (`a[0]`) is marked
/// armour-penetrated (bit 31 of its flags). The compare and the store stay the translation's
/// (JIP's SSE `comiss`/`jbe` takes the same branch as the exe's `fcompp`/`test ah, 0x41`).
fn armor_penetrated(e: &mut Engine, a: &[u32]) -> Option<u32> {
    let at = a[0].wrapping_add(HIT_FLAGS_TOP);
    let top = e.mem.u8(at);
    e.mem.set_u8(at, top | ARMOR_PENETRATED_BIT);
    Some(0)
}

/// `Actor`'s byte JIP's `KillChallengeFixHook` tests at `+0x143` (bit 7) and the actor base's
/// flags byte (`+0x34`, bit 1: essential).
const KILL_FLAGS: u32 = 0x143;
const BASE_FLAGS: u32 = 0x34;

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_game.h: KillChallengeFixHook.
/// G98: in place of the killed actor's (`a[0]`) teammate test before the kill challenges: 1 (no
/// challenge counted) for a teammate, for an actor with `+0x143` bit 7 and for one whose actor
/// base (`GetActorBase`) is essential; else 0. JIP reads the base with no null test; with no
/// base the port answers 0.
fn kill_challenge_skips(e: &mut Engine, a: &[u32]) -> Option<u32> {
    let killed = a[0];
    if e.mem.u8(killed.wrapping_add(IS_TEAMMATE)) != 0
        || e.mem.u8(killed.wrapping_add(KILL_FLAGS)) & 0x80 != 0
    {
        return Some(1);
    }
    let base = crate::commands::ln_fn_actor::actor_base(e, killed);
    if base == 0 {
        return Some(0);
    }
    Some(u32::from(e.mem.u8(base + BASE_FLAGS) & 2 != 0))
}

/// `kFormType_Creature` (a creature reference), `Actor::baseProcess`, the process level byte and
/// the high process's byte JIP's `CreatureSpreadFixHook` answers.
const FORM_TYPE_CREATURE: u8 = 0x3c;
const PROCESS: u32 = 0x68;
const PROCESS_LEVEL: u32 = 0x28;
const PROCESS_AIMING: u32 = 0x228;

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_game.h: CreatureSpreadFixHook.
/// O19 (`bCreatureSpreadFix`): in place of the iron sights test (`008bbc10`) in the gun skill
/// value: 1 for a creature; 0 without a process or for a process that is not high; else the high
/// process's byte `+0x228`.
fn creature_spread(e: &mut Engine, a: &[u32]) -> Option<u32> {
    let actor = a[0];
    if e.mem.u8(actor.wrapping_add(4)) == FORM_TYPE_CREATURE {
        return Some(1);
    }
    let process = e.mem.u32(actor.wrapping_add(PROCESS));
    if process == 0 || e.mem.u8(process + PROCESS_LEVEL) != 0 {
        return Some(0);
    }
    Some(u32::from(e.mem.u8(process + PROCESS_AIMING)))
}

/// The impact data set's slots (`+0x1C`, one per material) and JIP's fallback set (`0x11CA834`,
/// the exe's override set) with its lookup (`0058f6f0`).
const IMPACT_SLOTS: u32 = 0x1c;
const IMPACT_FALLBACK: u32 = 0x011c_a834;
const IMPACT_LOOKUP: u32 = 0x0058_f6f0;

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_game.h: GetImpactDataHook.
/// G109: the set's impact data for `material`: none for an empty slot (the exe asks the fallback
/// set even then); with the fallback set, its answer, or the slot's own data when it answers
/// none (the exe returns its answer as is).
pub fn impact_data(e: &mut Engine, set: u32, material: u32) -> u32 {
    let slot = set
        .wrapping_add(material.wrapping_mul(4))
        .wrapping_add(IMPACT_SLOTS);
    let data = e.mem.u32(slot);
    if data == 0 {
        return 0;
    }
    let fallback = e.mem.u32(IMPACT_FALLBACK);
    if fallback == 0 {
        return data;
    }
    match e.call(IMPACT_LOOKUP, &[fallback, material, data]).u32() {
        0 => data,
        found => found,
    }
}

/// `0058e9d0` (`ECX` the set, one word: the material) rewritten as [`impact_data`].
fn get_impact_data(e: &mut Engine, w: &[u32]) -> Ret {
    Ret {
        eax: impact_data(e, w[0], w[1]),
        ..Ret::default()
    }
}

/// `HitData::pWeapon` and the weapon's impact data set (`0x8D8AC0` reads `+0x24C`).
const HIT_WEAPON: u32 = 0x30;
const WEAPON_IMPACT_SET: u32 = 0x24c;

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_game.h: CreateHitBloodHook.
/// G120: the hit blood's impact data from the hit's (`a[0]`) own weapon's impact data set and
/// [`impact_data`] for the material (`a[1]`), in place of the aggressor's current weapon's and
/// `0058e9d0`; 0 with no weapon or no set. The translation reads the texture set (`+0x48`) of an
/// answer that is not 0, as JIP does.
fn hit_blood_impact(e: &mut Engine, a: &[u32]) -> Option<u32> {
    let weapon = e.mem.u32(a[0].wrapping_add(HIT_WEAPON));
    if weapon == 0 {
        return Some(0);
    }
    let set = e.mem.u32(weapon + WEAPON_IMPACT_SET);
    if set == 0 {
        return Some(0);
    }
    Some(impact_data(e, set, a[1]))
}

/// `fDamageToWeaponValue`'s value, the process's weapon and ammo entries, the weapon's words
/// JIP reads (flags `+0x12C` bit 7: its own value at `+0x148`; skill `+0x15C`; `+0xA0`; ammo
/// `+0xA8`), `TESAmmo`'s vtable and its effects list (`+0xD4`), `ApplyAmmoEffects`.
const DAMAGE_TO_WEAPON_VALUE: u32 = 0x011c_f154;
const PROCESS_WEAPON: u32 = 0x114;
const PROCESS_AMMO: u32 = 0x118;
const WEAPON_FLAGS: u32 = 0x12c;
const WEAPON_OWN_VALUE: u32 = 0x148;
const WEAPON_SKILL: u32 = 0x15c;
const WEAPON_FIRE_WORD: u32 = 0xa0;
const WEAPON_AMMO: u32 = 0xa8;
const TES_AMMO_VTABLE: u32 = 0x0102_6064;
const AMMO_EFFECTS: u32 = 0xd4;
const APPLY_AMMO_EFFECTS: u32 = 0x0059_a030;
/// `kAVCode_BigGuns`, and JIP's `kSkillMatcher` (by skill from Big Guns) and `kDegrMults`.
const AV_BIG_GUNS: u8 = 0x21;
const SKILL_MATCHER: [i8; 13] = [3, 1, 3, -1, -1, 2, -1, -1, 0, -1, -1, -1, 2];
const DEGRADATION: [u32; 4] = [0x3e19_999a, 0x3e4c_cccd, 0x3e80_0000, 0x3e99_999a];

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_game.h: GetDamageToWeaponHook.
/// G111 (and O11, `bFO3WpnDegradation`): `00646260` (cdecl: the actor) rewritten. Without a
/// process of level 0 or 1 or without its weapon entry: `fDamageToWeaponValue`. Else the
/// weapon's own value when its flag `+0x12C` bit 7 is set, else the setting; with O11 on, times
/// the skill's multiplier (Guns 0.15, Energy Weapons 0.2, Melee and Unarmed 0.25, Big Guns and
/// Explosives 0.3) and the weapon's word `+0xA0`, in single floats; then, for a weapon with ammo
/// and a `TESAmmo` in the process's ammo entry that has effects, `ApplyAmmoEffects(4, effects,
/// value)`. JIP reads a weapon entry's form with no null test; with none the port answers the
/// setting.
fn get_damage_to_weapon(e: &mut Engine, w: &[u32]) -> Ret {
    let setting = e.mem.f32(DAMAGE_TO_WEAPON_VALUE);
    let process = e.mem.u32(w[0].wrapping_add(PROCESS));
    let entry = if process != 0 && e.mem.u8(process + PROCESS_LEVEL) <= 1 {
        e.mem.u32(process + PROCESS_WEAPON)
    } else {
        0
    };
    let weapon = if entry != 0 { e.mem.u32(entry + 8) } else { 0 };
    if weapon == 0 {
        return f32::into_ret(setting);
    }
    let mut value = if e.mem.u8(weapon + WEAPON_FLAGS) & 0x80 != 0 {
        e.mem.f32(weapon + WEAPON_OWN_VALUE)
    } else {
        setting
    };
    if e.jip_patches() & jip_patches::FO3_WPN_DEGRADATION != 0 {
        let skill = e.mem.u8(weapon + WEAPON_SKILL).wrapping_sub(AV_BIG_GUNS) as i8;
        if (0..=12).contains(&skill) {
            let matched = SKILL_MATCHER[skill as usize];
            if matched >= 0 {
                value *= f32::from_bits(DEGRADATION[matched as usize]);
                value *= f32::from(e.mem.u16(weapon + WEAPON_FIRE_WORD));
            }
        }
    }
    if e.mem.u32(weapon + WEAPON_AMMO) != 0 {
        let ammo_entry = e.mem.u32(process + PROCESS_AMMO);
        let ammo = if ammo_entry != 0 {
            e.mem.u32(ammo_entry + 8)
        } else {
            0
        };
        if ammo != 0 && e.mem.u32(ammo) == TES_AMMO_VTABLE && e.mem.u32(ammo + AMMO_EFFECTS) != 0 {
            return e.call(
                APPLY_AMMO_EFFECTS,
                &[4, ammo + AMMO_EFFECTS, value.to_bits()],
            );
        }
    }
    f32::into_ret(value)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_game.h: InitGamePatches
// (SafeWrite8(0x89E790, 8), SafeWrite16(0x89F351, 0x0BEB)).
/// G33 (JIP's own; `GetKiller` reads it): `Actor::Kill` stores the player's killer too (the jump
/// lands on the store, past `0087eef0`) and no longer clears it after exporting the progress
/// data. Any answer takes JIP's way.
fn player_killer(_: &mut Engine, _: &[u32]) -> Option<u32> {
    Some(0)
}

/// `Actor::killer` (`+0xC0`).
const KILLER: u32 = 0xc0;

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/serialization.h:
// DoPreLoadGameHousekeeping (`g_thePlayer->killer = nullptr`).
/// G33's part of the housekeeping before a load: the player's killer cleared.
pub(crate) fn pre_load(e: &mut Engine) {
    let player = e.mem.u32(THE_PLAYER);
    if player != 0 {
        e.mem.set_u32(player + KILLER, 0);
    }
}

#[cfg(test)]
#[path = "gc_tests.rs"]
mod tests;
