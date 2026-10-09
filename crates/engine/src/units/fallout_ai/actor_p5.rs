//! `fallout/ai/actor.cpp` (Xbox PDB source unit), part 5: its functions from `008b00c0` up to
//! (not including) `008b8e90` in FalloutNV.exe 1.4.0.525, translated
//! (docs/ENGINE_CRATE.md). The unit's shared layouts and helpers are in
//! [`super::actor`]; anything public there may be used here.
//!
//! The PC `Actor` is the Xbox PDB's with every field after `TESForm` 0x10 lower
//! (`TESForm` is 0x18 on PC), so `pCurrentProcess` (PDB +0x78) is at +0x68.
//! Several functions here are thunks of the `MagicTarget` base (the `Actor`
//! pointer plus 0xA4): they take that interior pointer as `this` and reach
//! the actor's process at `this - 0x3C`.

#[allow(unused_imports)]
use super::actor::*;
#[allow(unused_imports)]
use crate::prelude::*;

/// `PlayerCharacter::pSingleton`: the global holding the player's pointer.
const PLAYER_CHARACTER: u32 = 0x011d_ea3c;
/// `0.0` as a `double` in the exe's constants (compared against by `FCOMP`).
const ZERO_DOUBLE: u32 = 0x0101_2060;

/// `Actor::bPlayerTeammate` getter (the engine map names it
/// `MiddleHighProcess::GetForceNextUpdate` because the linker folded the
/// identical bodies; it reads the byte at actor +0x18D).
const ACTOR_IS_PLAYER_TEAMMATE: u32 = 0x0056_6950;
/// `Actor::GetActorAggression` (Xbox PDB): the actor's aggression, 3 meaning
/// "never fights".
const ACTOR_GET_ACTOR_AGGRESSION: u32 = 0x008b_ea10;
/// `Actor::GetActorAssistance` (Xbox PDB).
const ACTOR_GET_ACTOR_ASSISTANCE: u32 = 0x008b_eae0;
/// `Actor::IsAngryWithPlayer` (Xbox PDB).
const ACTOR_IS_ANGRY_WITH_PLAYER: u32 = 0x008b_ffc0;
/// `Actor::IsInCombatWithActor` (Xbox PDB): `(this, other)`.
const ACTOR_IS_IN_COMBAT_WITH_ACTOR: u32 = 0x008b_c700;
/// `Actor::GetFactionFightReaction` (Xbox PDB): `(this, other, out flag byte)`.
const ACTOR_GET_FACTION_FIGHT_REACTION: u32 = 0x008b_87a0;
/// `Actor::GetAnimation` (Xbox PDB): the process's animation, or 0.
const ACTOR_GET_ANIMATION: u32 = 0x008b_70d0;
/// `PlayerCharacter::GetAnimation` (Xbox PDB): `(this, first_person)`.
const PLAYER_GET_ANIMATION: u32 = 0x0095_0a60;
/// `Animation::ReloadTargets` (Xbox PDB): `(animation, flag)`.
const ANIMATION_RELOAD_TARGETS: u32 = 0x0049_9240;
/// `MiddleHighProcess::GetSavedAcquireObject` (Xbox PDB): the actor's
/// process (the body is `this->pCurrentProcess`).
const ACTOR_GET_PROCESS: u32 = 0x008d_8520;
/// `BGSEntryPoint::HandleEntryPoint` (Xbox PDB), cdecl.
const HANDLE_ENTRY_POINT: u32 = 0x005e_58f0;
/// Getter of `Actor::bInCombat` (Xbox PDB +0x114, PC +0x104), a byte.
const ACTOR_IS_IN_COMBAT: u32 = 0x0049_3bb0;
/// `PlayerCharacter::IsTargetPerceivedAndHostile` (Xbox PDB): `(player, target)`.
const PLAYER_IS_TARGET_PERCEIVED_AND_HOSTILE: u32 = 0x0093_dee0;
/// `"FollowerSwitchAggressive"` follower setting lookup: `(actor, name)`,
/// returns the value as a float in ST0.
const ACTOR_GET_FOLLOWER_SETTING: u32 = 0x008c_1740;

/// `ProcessLists` instance (`ProcessLists::GetActorRefInHigh`'s `this`).
const PROCESS_LISTS: u32 = 0x011e_0e80;
/// The lip sync background manager instance (`LipSyncBackgroundManager::CancelLipFile`'s `this`).
const LIP_SYNC_MANAGER: u32 = 0x011f_11e0;
/// `Actor::GetIronSights` (Xbox PDB): the process's slot `0x404`, false without a process.
const ACTOR_GET_IRON_SIGHTS: u32 = 0x008b_bc10;
/// `Actor::SetLifeState` (Xbox PDB): `(this, state)`.
const ACTOR_SET_LIFE_STATE: u32 = 0x008a_1800;

layout! {
    /// `Actor` (Xbox PDB), the fields this part uses, at their PC offsets
    /// (the PDB's minus 0x10).
    pub struct Actor: 0x1b4 {
        /// `pCurrentProcess` (Xbox PDB +0x78): `BaseProcess*`.
        0x68 pCurrentProcess: Ptr,
        /// `pMyKiller` (Xbox PDB +0xD0): `Actor*`.
        0xC0 pMyKiller: Ptr,
        /// `bMurderAlarm` (Xbox PDB +0xD4).
        0xC4 bMurderAlarm: u8,
        /// `bReloadTargetQueued` (Xbox PDB +0x101).
        0xF1 bReloadTargetQueued: u8,
        /// `iVisFlags` (Xbox PDB +0x12C).
        0x11C iVisFlags: u32,
        /// `bSetOnDeath` (Xbox PDB +0x184): set once the death was handled.
        0x174 bSetOnDeath: u8,
        /// `fGunSkillGun` (Xbox PDB +0x188): cached value for selector 1 of
        /// the entry-point value at `008b0dd0`.
        0x178 fGunSkillGun: f32,
        /// `fGunSkillHUD` (Xbox PDB +0x18C): cached value for selector 0.
        0x17C fGunSkillHUD: f32,
        /// `fGunSkillActor` (Xbox PDB +0x190): cached value for selector 2.
        0x180 fGunSkillActor: f32,
        /// `fGunSkillVATS` (Xbox PDB +0x194): cached value for selector 3.
        0x184 fGunSkillVATS: f32,
        /// `pActorMover` (Xbox PDB +0x1A0): `ActorMover*`.
        0x190 pActorMover: Ptr,
    }
}

// Translated from 008b00c0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Updates the character controller's `0x4000` state flag from the actor
/// value `0x15`: when the actor has a controller whose state does not have
/// `0x10000000` set, the flag is set exactly when that value is `0.0`
/// (`0087f9c0` returns the value as a float).
pub fn fn_008b00c0(e: &mut Engine, this: Ptr<Actor>) {
    // MobileObject::GetCharController (Xbox PDB)
    let controller = e.call(0x0093_06d0, &args![this]).u32();
    if controller == 0 {
        return;
    }
    let state = controller + 0x410;
    if e.call(0x0062_1270, &args![state, 0x1000_0000u32]).u32() != 0 {
        return;
    }
    let value = e.call(0x0087_f9c0, &args![this]).f64();
    let zero: f64 = e.global(ZERO_DOUBLE);
    let is_zero = value == zero;
    e.call(0x0062_9670, &args![state, 0x4000u32, is_zero as u32]);
}

// Translated from 008b0140 (decompiled, FalloutNV.exe 1.4.0.525)
/// Sets (`set` non-zero) or clears the bits of `mask` in `iVisFlags`.
pub fn fn_008b0140(e: &mut Engine, this: Ptr<Actor>, set: u8, mask: u32) {
    let flags = e.get(this, Actor::iVisFlags);
    let flags = if set != 0 {
        flags | mask
    } else {
        !mask & flags
    };
    e.set(this, Actor::iVisFlags, flags);
}

// Translated from 008b0190 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Actor::IsVisible` (Xbox PDB): true when every bit of `mask` is set in
/// `iVisFlags`.
pub fn actor_is_visible(e: &mut Engine, this: Ptr<Actor>, mask: u32) -> bool {
    let flags = e.get(this, Actor::iVisFlags);
    (!(flags & mask) & mask) == 0
}

// Translated from 008b0520 (decompiled, FalloutNV.exe 1.4.0.525)
/// Decides whether the actor's process should hand its acquired object back
/// (`true` means yes). Virtual slots used are those of the actor (`0x160`,
/// `0x1D0`) and of its process (`0x22C`, `0x280`, `0x5F8`, `0x5FC`).
/// Returns false at once when slot `0x160` of the actor answers true.
pub fn fn_008b0520(e: &mut Engine, this: Ptr<Actor>) -> bool {
    if e.vcall(this.addr(), 0x160, &args![]).bool() {
        return false;
    }
    let process = e.call(ACTOR_GET_PROCESS, &args![this]).u32();
    let object = e.vcall(process, 0x22c, &args![]).u32();
    if object != 0 {
        // The object's kind (`0041ca90`) must be 5 or 6 and the process
        // must answer slot 0x280 for it.
        let mut proceed = false;
        if e.call(0x0041_ca90, &args![object]).i32() == 5 {
            let process = e.call(ACTOR_GET_PROCESS, &args![this]).u32();
            proceed = e.vcall(process, 0x280, &args![]).u32() != 0;
        }
        if !proceed && e.call(0x0041_ca90, &args![object]).i32() == 6 {
            let process = e.call(ACTOR_GET_PROCESS, &args![this]).u32();
            proceed = e.vcall(process, 0x280, &args![]).u32() != 0;
        }
        if !proceed {
            return false;
        }
    }
    let process = e.call(ACTOR_GET_PROCESS, &args![this]).u32();
    if e.vcall(process, 0x5fc, &args![]).bool() {
        let process = e.call(ACTOR_GET_PROCESS, &args![this]).u32();
        e.vcall(process, 0x5f8, &args![0u32]);
        return false;
    }
    let target = e.vcall(this.addr(), 0x1d0, &args![]).u32();
    if target != 0 && e.vcall(target, 0x10, &args![]).u32() != 0 {
        // `00524c90` takes nothing; the 0.8f constant is the second argument.
        let first = e.call(0x0052_4c90, &args![]).u32();
        let limit: f32 = e.global(0x0103_0ff0);
        if !e.call(0x00b4_e030, &args![target, first, limit]).bool() {
            return true;
        }
    }
    false
}

// Translated from 008b0670 (decompiled, FalloutNV.exe 1.4.0.525)
/// True when `other` is not the player and neither is `this` (or `this` is
/// the player's teammate); otherwise asks
/// [`actor_get_should_attack_actor`] with the flag set. The `out` value of
/// that call is discarded.
pub fn fn_008b0670(e: &mut Engine, this: Ptr<Actor>, other: Ptr<Actor>) -> bool {
    let player: u32 = e.global(PLAYER_CHARACTER);
    let must_ask = if other.addr() == player {
        true
    } else if this.addr() == player {
        false
    } else {
        !e.call(ACTOR_IS_PLAYER_TEAMMATE, &args![this]).bool()
    };
    if !must_ask {
        return true;
    }
    e.with_stack(4, |e, out| {
        actor_get_should_attack_actor(e, this, other, true, out, false)
    })
}

// Translated from 008b06d0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Actor::GetShouldAttackActor` (Xbox PDB): whether `this` should attack
/// `target`. `engaged` is the caller's "already fighting" flag (set here when
/// `target` is in combat with `this`, or `this` is a teammate and the target
/// is in combat with the player, unless `ignore_combat`); `out` receives the
/// faction fight reaction. A teammate of the player judges for the player,
/// and with `FollowerSwitchAggressive` set treats its aggression as 1.
pub fn actor_get_should_attack_actor(
    e: &mut Engine,
    this: Ptr<Actor>,
    target: Ptr<Actor>,
    engaged: bool,
    out: Ptr,
    ignore_combat: bool,
) -> bool {
    let player: u32 = e.global(PLAYER_CHARACTER);
    let mut engaged = engaged;
    let mut follower = false;
    let teammate = e.call(ACTOR_IS_PLAYER_TEAMMATE, &args![this]).bool();
    if teammate {
        if target.addr() == player {
            return e.call(ACTOR_IS_ANGRY_WITH_PLAYER, &args![this]).bool();
        }
        if e.vcall(target.addr(), 0x1a0, &args![0u32]).bool() {
            return false;
        }
        let switch_aggressive = e
            .call(ACTOR_GET_FOLLOWER_SETTING, &args![this, 0x0107_1de0u32])
            .f64();
        let zero: f64 = e.global(ZERO_DOUBLE);
        if switch_aggressive == zero {
            if !e.call(ACTOR_IS_IN_COMBAT, &args![target]).bool() {
                return false;
            }
            return e
                .call(ACTOR_IS_IN_COMBAT_WITH_ACTOR, &args![target, player])
                .bool()
                || e.call(ACTOR_IS_IN_COMBAT_WITH_ACTOR, &args![target, this])
                    .bool();
        }
        follower = true;
    } else if e.call(ACTOR_IS_PLAYER_TEAMMATE, &args![target]).bool() {
        return e.with_stack(4, |e, ignored| {
            actor_get_should_attack_actor(e, this, Ptr::new(player), engaged, ignored, false)
        });
    }
    let aggression = if follower {
        1
    } else {
        e.call(ACTOR_GET_ACTOR_AGGRESSION, &args![this]).i32()
    };
    if aggression == 3 {
        return true;
    }
    if (e
        .call(ACTOR_IS_IN_COMBAT_WITH_ACTOR, &args![target, this])
        .bool()
        || (e.call(ACTOR_IS_PLAYER_TEAMMATE, &args![this]).bool()
            && e.call(ACTOR_IS_IN_COMBAT_WITH_ACTOR, &args![target, player])
                .bool()))
        && !ignore_combat
    {
        engaged = true;
    }
    // Eight bytes: the reaction function's flag out value at +0, the verdict
    // byte at +4.
    e.with_stack(8, |e, scratch| {
        let reaction = if follower && !target.is_null() {
            e.call(
                ACTOR_GET_FACTION_FIGHT_REACTION,
                &args![target, player, scratch],
            )
            .i32()
        } else {
            e.call(
                ACTOR_GET_FACTION_FIGHT_REACTION,
                &args![this, target, scratch],
            )
            .i32()
        };
        e.mem.set_i32(out.addr(), reaction);
        let verdict = scratch.byte_add(4);
        e.mem.set_u8(verdict.addr(), 0);
        if target.addr() == player && e.call(ACTOR_IS_ANGRY_WITH_PLAYER, &args![this]).bool() {
            e.mem.set_u8(verdict.addr(), 1);
        } else if engaged {
            if e.call(ACTOR_GET_ACTOR_AGGRESSION, &args![target]).i32() == 3
                || (reaction != 3 && reaction != 2)
            {
                e.mem.set_u8(verdict.addr(), 1);
            }
        } else {
            match aggression {
                1 if reaction == 1
                    && (!teammate
                        || e.call(
                            PLAYER_IS_TARGET_PERCEIVED_AND_HOSTILE,
                            &args![player, target],
                        )
                        .bool()) =>
                {
                    e.mem.set_u8(verdict.addr(), 1);
                }
                2 => {
                    if reaction <= 1 {
                        e.mem.set_u8(verdict.addr(), 1);
                    }
                }
                3 => e.mem.set_u8(verdict.addr(), 1),
                _ => {}
            }
        }
        e.call(HANDLE_ENTRY_POINT, &args![0xfu32, this, target, verdict]);
        e.mem.u8(verdict.addr()) != 0
    })
}

// Translated from 008b0970 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Actor::GetShouldHelp` (Xbox PDB): whether `this` should help `other`.
/// Nobody is never helped, oneself always is. A teammate of the player
/// helps whoever the player is not in combat against, and an actor facing a
/// teammate asks about the player instead; otherwise the answer follows the
/// actor's assistance setting and its faction's reaction to `other`.
pub fn actor_get_should_help(e: &mut Engine, this: Ptr<Actor>, other: Ptr<Actor>) -> bool {
    if other.is_null() {
        return false;
    }
    if other == this {
        return true;
    }
    let player: u32 = e.global(PLAYER_CHARACTER);
    if e.call(ACTOR_IS_PLAYER_TEAMMATE, &args![this]).bool() {
        if other.addr() == player {
            return !e.call(ACTOR_IS_ANGRY_WITH_PLAYER, &args![this]).bool();
        }
        if e.call(ACTOR_IS_IN_COMBAT, &args![other]).bool()
            && (e
                .call(ACTOR_IS_IN_COMBAT_WITH_ACTOR, &args![other, player])
                .bool()
                || e.call(ACTOR_IS_IN_COMBAT_WITH_ACTOR, &args![other, this])
                    .bool())
        {
            return false;
        }
        return actor_get_should_help(e, Ptr::new(player), other);
    }
    if e.call(ACTOR_IS_PLAYER_TEAMMATE, &args![other]).bool() {
        return actor_get_should_help(e, this, Ptr::new(player));
    }
    if e.call(ACTOR_GET_ACTOR_AGGRESSION, &args![other]).i32() == 3 {
        return false;
    }
    let assistance = e.call(ACTOR_GET_ACTOR_ASSISTANCE, &args![this]).i32();
    let reaction = e.with_stack(4, |e, flag| {
        e.call(ACTOR_GET_FACTION_FIGHT_REACTION, &args![this, other, flag])
            .i32()
    });
    match assistance {
        1 => reaction == 2,
        2 => reaction == 3 || reaction == 2,
        _ => false,
    }
}

// Translated from 008b0ac0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Reloads the targets of the actor's animation (virtual slot `0x1E4`) with
/// the queued flag `bReloadTargetQueued`, then clears that flag.
pub fn fn_008b0ac0(e: &mut Engine, this: Ptr<Actor>) {
    let queued = e.get(this, Actor::bReloadTargetQueued);
    let animation = e.vcall(this.addr(), 0x1e4, &args![]).u32();
    e.call(ANIMATION_RELOAD_TARGETS, &args![animation, queued]);
    e.set(this, Actor::bReloadTargetQueued, 0);
}

// Translated from 008b0b00 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Actor::ReloadTargets` (Xbox PDB): reloads the targets of the actor's
/// animation. The player has two (first and third person); any other actor
/// needs both an animation and a process.
pub fn actor_reload_targets(e: &mut Engine, this: Ptr<Actor>, flag: u8) {
    let player: u32 = e.global(PLAYER_CHARACTER);
    if this.addr() == player {
        for first_person in [1u32, 0u32] {
            if e.call(PLAYER_GET_ANIMATION, &args![player, first_person])
                .u32()
                != 0
            {
                let animation = e
                    .call(PLAYER_GET_ANIMATION, &args![player, first_person])
                    .u32();
                e.call(ANIMATION_RELOAD_TARGETS, &args![animation, flag]);
            }
        }
    } else if e.call(ACTOR_GET_ANIMATION, &args![this]).u32() != 0
        && !e.get(this, Actor::pCurrentProcess).is_null()
    {
        let animation = e.call(ACTOR_GET_ANIMATION, &args![this]).u32();
        e.call(ANIMATION_RELOAD_TARGETS, &args![animation, flag]);
    }
}

// Translated from 008b0ba0 (decompiled, FalloutNV.exe 1.4.0.525)
/// The process's virtual slot `0x52C` result, or 0 without a process.
pub fn fn_008b0ba0(e: &mut Engine, this: Ptr<Actor>) -> u32 {
    let process = e.get(this, Actor::pCurrentProcess);
    if process.is_null() {
        return 0;
    }
    e.vcall(process.addr(), 0x52c, &args![]).u32()
}

// Translated from 008b0bd0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Actor::InitLightingPropertyPtr` (Xbox PDB): finds the lighting source
/// among the children of `node`'s first child (virtual slot `0xC` of the
/// node): the child named `"SkinAttachment"` if there is one, otherwise the
/// first child not named `"BIP..."` that has a child list and a lighting
/// property, otherwise the first child itself; then hands the lighting
/// property found to the process (virtual slot `0x4FC`).
pub fn actor_init_lighting_property_ptr(e: &mut Engine, this: Ptr<Actor>, node: Ptr) {
    let process = e.get(this, Actor::pCurrentProcess);
    if process.is_null() || node.is_null() {
        return;
    }
    let first = e.vcall(node.addr(), 0xc, &args![]).u32();
    let mut property = 0u32;
    if first == 0 {
        return;
    }
    // "SkinAttachment" (`0x101f724`)
    let mut chosen = e.call(0x004a_ae30, &args![node, 0x0101_f724u32]).u32();
    if chosen == 0 {
        let mut index = 0u32;
        loop {
            let count = e.call(0x0043_b480, &args![first]).u32();
            if index >= count || chosen != 0 {
                break;
            }
            let child = e.call(0x0043_b4a0, &args![first, index]).u32();
            if child != 0 {
                let name_holder = e.call(0x0041_3f40, &args![child]).u32();
                let name = e.call(0x0043_b1b0, &args![name_holder]).u32();
                // `_strnicmp(name, "BIP", 3)` (`0x1084e60`)
                if name != 0
                    && e.call(0x00ec_7ec0, &args![name, 0x0108_4e60u32, 3u32])
                        .i32()
                        != 0
                {
                    let grandchildren = e.vcall(child, 0xc, &args![]).u32();
                    if grandchildren != 0
                        && e.call(0x0043_b480, &args![grandchildren]).u32() != 0
                        && e.call(0x004b_2960, &args![grandchildren]).u32() != 0
                    {
                        property = e.call(0x004b_5710, &args![grandchildren]).u32();
                        if property != 0 {
                            chosen = grandchildren;
                        }
                    }
                }
            }
            index += 1;
        }
        if chosen == 0 {
            chosen = first;
        }
    }
    if property == 0 {
        property = e.call(0x004b_5710, &args![chosen]).u32();
    }
    e.vcall(process.addr(), 0x4fc, &args![property]);
}

// Translated from 008b0d30 (decompiled, FalloutNV.exe 1.4.0.525)
/// Thunk of the `MagicTarget` base (`this` is the actor + 0xA4): the
/// process's virtual slot `0x5E4` as a float, or `0.0` without a process.
pub fn fn_008b0d30(e: &mut Engine, this: Ptr, argument: u32) -> f32 {
    let process = e.mem.u32(this.addr().wrapping_sub(0x3c));
    if process == 0 {
        return 0.0;
    }
    e.vcall(process, 0x5e4, &args![argument]).f32()
}

// Translated from 008b0d70 (decompiled, FalloutNV.exe 1.4.0.525)
/// Thunk of the `MagicTarget` base (`this` is the actor + 0xA4): the
/// process's virtual slot `0x5EC` as a float, or `0.0` without a process.
pub fn fn_008b0d70(e: &mut Engine, this: Ptr, argument: u32) -> f32 {
    let process = e.mem.u32(this.addr().wrapping_sub(0x3c));
    if process == 0 {
        return 0.0;
    }
    e.vcall(process, 0x5ec, &args![argument]).f32()
}

// Translated from 008b0db0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Thunk of the `MagicTarget` base (`this` is the actor + 0xA4):
/// `ModifierList::GetModifier` (Xbox PDB, `00937730`) of the list embedded at
/// `this + 0x2C` (the actor's permanent modifiers), as a float.
pub fn fn_008b0db0(e: &mut Engine, this: Ptr, modifier: u8) -> f32 {
    e.call(0x0093_7730, &args![this.addr() + 0x2c, modifier])
        .f32()
}

// Translated from 008b0dd0 (decompiled, FalloutNV.exe 1.4.0.525)
/// A cached value from the actor's entry-point calculation, by `selector`
/// (1, 2, 0 or 3: `fGunSkillGun`, `fGunSkillActor`, `fGunSkillHUD`,
/// `fGunSkillVATS`). A non-negative cache is returned as is (a negative one
/// for selector 1 is reset to 0); otherwise the value is computed from the
/// process's base value (virtual slot `0x42C`) with `00646910`, adjusted by
/// entry point `0x22` and, for selector 3, scaled by a game setting and added
/// to the process's base value again, then cached. Returns `0.0` without a
/// process or without the process's item (slot `0x148`).
pub fn fn_008b0dd0(e: &mut Engine, this: Ptr<Actor>, selector: i32) -> f32 {
    let process = e.get(this, Actor::pCurrentProcess);
    if process.is_null() || e.vcall(process.addr(), 0x148, &args![]).u32() == 0 {
        return 0.0;
    }
    let item = e.vcall(process.addr(), 0x148, &args![]).u32();
    let item_value = e.call(0x0044_ddc0, &args![item]).u32();
    match selector {
        1 => {
            let cached = e.get(this, Actor::fGunSkillGun);
            if cached >= 0.0 {
                return cached;
            }
            e.set(this, Actor::fGunSkillGun, 0.0);
            return 0.0;
        }
        2 => {
            let cached = e.get(this, Actor::fGunSkillActor);
            if cached >= 0.0 {
                return cached;
            }
        }
        0 => {
            let cached = e.get(this, Actor::fGunSkillHUD);
            if cached >= 0.0 {
                return cached;
            }
        }
        3 => {
            let cached = e.get(this, Actor::fGunSkillVATS);
            if cached >= 0.0 {
                return cached;
            }
        }
        _ => {}
    }
    let base = e.vcall(process.addr(), 0x42c, &args![]).f32();
    let mut flag_100 = false;
    let mut running = false;
    if selector != 3 {
        let flags = e.call(0x0088_46e0, &args![this]).u32();
        flag_100 = flags & 0x100 != 0;
        running = flags & 0xf != 0 && e.call(0x0088_4730, &args![this]).bool();
    }
    e.vcall(this.addr(), 0x1e4, &args![]);
    let mode = e.call(0x0044_ddc0, &args![0x011f_2250u32]).i32();
    let mut value = if mode == 2 || mode == 3 {
        e.call(
            0x0064_6910,
            &args![this, item_value, base, 1u32, 0u32, flag_100, running],
        )
        .f32()
    } else {
        let flag_400 = e.call(0x0049_97b0, &args![this]).bool();
        let iron_sights = e.call(0x008b_bc10, &args![this]).bool();
        e.call(
            0x0064_6910,
            &args![
                this,
                item_value,
                base,
                iron_sights,
                flag_400,
                flag_100,
                running
            ],
        )
        .f32()
    };
    e.with_stack(4, |e, slot| {
        e.mem.set_f32(slot.addr(), value);
        e.call(HANDLE_ENTRY_POINT, &args![0x22u32, this, item_value, slot]);
        value = e.mem.f32(slot.addr());
    });
    match selector {
        2 => e.set(this, Actor::fGunSkillActor, value),
        0 => e.set(this, Actor::fGunSkillHUD, value),
        3 => {
            let scale = e.call(0x0040_3e20, &args![0x011c_e034u32]).u32();
            let scale = e.mem.f32(scale);
            value = (value as f64 * scale as f64) as f32;
            let process = e.call(ACTOR_GET_PROCESS, &args![this]).u32();
            let base = e.vcall(process, 0x42c, &args![]).f64();
            value = (base + value as f64) as f32;
            e.set(this, Actor::fGunSkillVATS, value);
        }
        _ => {}
    }
    value
}
// Translated from 008b01c0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Actor::DoDeathStuff` (Xbox PDB): what happens when the actor dies.
///
/// Does nothing for an actor that is essential (while the setting at
/// `0x011e0888` is on), or whose animation (virtual slot `0x1E4`) has the
/// death animation kind and a 3D model that is not flagged `0x800`. On the
/// first call it records the death (`bSetOnDeath`), raises the crime alarm
/// for a killer (`pMyKiller`, with `bMurderAlarm` set and a killer that
/// answers slot `0x218`), lets the player's process react, sets the script
/// action flags `0x20` and `0x10` on the killer, stops the process's
/// movement and sets the life state to 2; on later calls it only sets the
/// life state.
pub fn actor_do_death_stuff(e: &mut Engine, this: Ptr<Actor>) {
    let animation = e.vcall(this.addr(), 0x1e4, &args![]).u32();
    if animation != 0
        && e.call(0x0049_1040, &args![animation, 0u32]).u32() != 0
        && !e.call(0x0044_0da0, &args![this]).bool()
    {
        return;
    }
    let essential_only = e.call(0x0040_8d60, &args![0x011e_0888u32]).u32();
    if e.mem.u8(essential_only) != 0 && e.call(0x0087_f3d0, &args![this]).bool() {
        return;
    }
    if e.get(this, Actor::bSetOnDeath) != 0 {
        e.call(ACTOR_SET_LIFE_STATE, &args![this, 2u32]);
        return;
    }
    e.set(this, Actor::bSetOnDeath, 1);
    let player: u32 = e.global(PLAYER_CHARACTER);
    let killer = e.get(this, Actor::pMyKiller);
    if e.get(this, Actor::bMurderAlarm) != 0
        && !killer.is_null()
        && e.vcall(killer.addr(), 0x218, &args![]).bool()
    {
        let mut witness = this.addr();
        let owner = e.call(0x0056_7790, &args![this]).u32();
        let slot_21c = e.vcall(this.addr(), 0x21c, &args![]).bool();
        if !slot_21c && owner == 0 {
            e.call(0x008c_09e0, &args![this, killer]);
        } else {
            if owner != 0 {
                if e.call(0x0040_1170, &args![owner]).u32() == 0x2a {
                    witness = e
                        .call(0x0097_0a20, &args![PROCESS_LISTS, owner, 0u32])
                        .u32();
                } else if e.call(0x0040_1170, &args![owner]).u32() == 8 {
                    witness = e
                        .call(0x0097_0b30, &args![PROCESS_LISTS, owner, 0u32, 0u32])
                        .u32();
                }
            }
            set_action_flag(e, this, killer.addr(), 0x20);
            let killer = e.get(this, Actor::pMyKiller);
            if witness != 0 && !killer.is_null() {
                e.call(0x008c_0460, &args![witness, killer, 0u32, 1u32]);
            }
            e.set(this, Actor::bMurderAlarm, 0);
        }
    }
    let killer = e.get(this, Actor::pMyKiller);
    if killer.addr() == player {
        let process = e.call(ACTOR_GET_PROCESS, &args![killer]).u32();
        let position = e.vcall(this.addr(), 0x1f4, &args![0u32, 4u32, 0u32]).u32();
        let (x, y, z) = (
            e.mem.u32(position),
            e.mem.u32(position + 4),
            e.mem.u32(position + 8),
        );
        e.vcall(process, 0xfc, &args![killer, x, y, z]);
    }
    let killer = e.get(this, Actor::pMyKiller);
    if killer.addr() == player && e.get(this, Actor::bMurderAlarm) != 0 {
        let extra = e.call(0x0041_81e0, &args![this]).u32();
        if extra != 0 {
            e.call(0x0047_ead0, &args![extra + 0x30, 0u32]);
        }
    }
    let killer = e.get(this, Actor::pMyKiller);
    set_action_flag(e, this, killer.addr(), 0x10);
    if e.call(ACTOR_GET_PROCESS, &args![this]).u32() != 0 {
        let process = e.call(ACTOR_GET_PROCESS, &args![this]).u32();
        let limit: f32 = e.global(0x0101_6970);
        e.vcall(process, 0x4b4, &args![limit]);
        let process = e.call(ACTOR_GET_PROCESS, &args![this]).u32();
        e.vcall(process, 0x4c0, &args![this, 0u32, 0u32, 0x7fu32]);
    }
    e.call(ACTOR_SET_LIFE_STATE, &args![this, 2u32]);
    if e.call(ACTOR_IS_IN_COMBAT, &args![this]).bool() {
        e.vcall(this.addr(), 0x434, &args![0u32]);
    }
    let process = e.get(this, Actor::pCurrentProcess);
    if !process.is_null() {
        e.call(0x0088_1680, &args![this, 0u32]);
        e.vcall(process.addr(), 0x214, &args![]);
        e.vcall(process.addr(), 0x234, &args![]);
    }
    e.call(0x008b_ca90, &args![this, 0u32]);
}

/// `Script::SetActionFlag` (Xbox PDB, `005ac750`, cdecl) on `target`'s
/// extra data list (`005d43c0` returns it: the object plus 0x44) with
/// `flag`.
fn set_action_flag(e: &mut Engine, this: Ptr<Actor>, target: u32, flag: u32) {
    let extra = e.call(0x005d_43c0, &args![this]).u32();
    e.call(0x005a_c750, &args![target, extra, flag]);
}
// Translated from 008b1070 (decompiled, FalloutNV.exe 1.4.0.525)
/// Re-applies the actor's dialogue package on its process.
///
/// The package is the process's current package (virtual slot `0x20C`) or,
/// failing that, its next one (slot `0x22C`); it counts as a dialogue
/// package when its kind (`0041ca90`) is `0x1C`. For one, the two speakers
/// (`009ee040` the one who started the conversation, `008d80e0` its target)
/// have their processes pointed at each other, their lip files cancelled and
/// the package re-run or cleared as the code below spells out; the player's
/// `0x208` field is cleared when it names the package. Without a dialogue
/// package the actor's own process is reset (slots `0x674`, `0x650`,
/// `0x624`, `0x71C`). Last, when the process has an item (slot `0x148`) and
/// no slot `0x14C` item, and that item's ammo regeneration rate is not
/// positive, slot `0x3EC` of the actor is called with the item's value.
pub fn fn_008b1070(e: &mut Engine, this: Ptr<Actor>) {
    let process = e.get(this, Actor::pCurrentProcess);
    if process.is_null() {
        return;
    }
    let setting_holder: u32 = e.global(0x011d_e45c);
    if e.call(0x0047_c850, &args![setting_holder]).bool() {
        return;
    }
    let mut current = true;
    let mut target_ok = true;
    let extra = e.call(0x005d_43c0, &args![this]).u32();
    e.call(0x0042_e130, &args![extra]);
    e.vcall(process.addr(), 0x24c, &args![0u32]);
    let mut package = e.vcall(process.addr(), 0x20c, &args![]).u32();
    if package == 0 || e.call(0x0041_ca90, &args![package]).i32() != 0x1c {
        current = false;
        package = e.vcall(process.addr(), 0x22c, &args![]).u32();
    }
    if package != 0 && e.call(0x0041_ca90, &args![package]).i32() == 0x1c {
        let player: u32 = e.global(PLAYER_CHARACTER);
        if package == e.mem.u32(player + 0x208) {
            e.mem.set_u32(player + 0x208, 0);
        }
        let starter = e.call(0x009e_e040, &args![package]).u32();
        let target = e.call(0x008d_80e0, &args![package]).u32();
        let reference = e.call(0x0040_36b0, &args![package]).u32();
        if reference != 0 {
            for _ in 0..2 {
                let holder = e.call(0x005e_3fa0, &args![reference]).u32();
                let extra = e.call(0x005d_43c0, &args![holder]).u32();
                e.call(0x0042_e130, &args![extra]);
            }
            e.call(0x0096_d470, &args![PROCESS_LISTS, reference, 0u32]);
            e.call(0x0096_e870, &args![PROCESS_LISTS, reference]);
        }
        if starter != target && current {
            let other = if this.addr() == starter {
                target
            } else {
                starter
            };
            let wait_type = e.mem.u32(package + 0x8c);
            if wait_type != 0 && e.call(0x0084_e3a0, &args![wait_type]).i32() == 0xef {
                if process_of(e, other) != 0
                    && process_call(e, other, 0x20c, &args![]).u32() == package
                {
                    process_call(e, this.addr(), 0x214, &args![]);
                    return;
                }
            } else if target != 0
                && (process_of(e, target) == 0
                    || process_call(e, target, 0x20c, &args![]).u32() != package)
            {
                target_ok = false;
            }
        }
        // "SoundHandle" of the package (+0x80): stop it when valid.
        if e.call(0x00ad_8ce0, &args![package + 0x80]).bool() {
            e.call(0x009e_f530, &args![package]);
        }
        for speaker in [starter, target] {
            if speaker != 0 && e.vcall(speaker, 0x100, &args![]).bool() {
                e.call(0x0090_6050, &args![LIP_SYNC_MANAGER, speaker]);
                e.mem.set_u8(speaker + 0x80, 1);
            }
        }
        e.call(0x0067_27b0, &args![package]);
        if starter != 0 && process_of(e, starter) != 0 {
            process_call(e, starter, 0x650, &args![1u32]);
            process_call(e, starter, 0x624, &args![target]);
            e.mem.set_f32(starter + 0x74, 1.0);
            process_call(e, starter, 0x374, &args![0u32]);
            process_call(e, starter, 0x2ec, &args![0u32]);
            if current {
                process_call(e, starter, 0x214, &args![]);
            } else {
                process_call(e, starter, 0x714, &args![starter]);
            }
            if !e.vcall(starter, 0x274, &args![1u32]).bool() {
                // (The game takes this actor's process here, not the starter's.)
                process_call(e, this.addr(), 0x71c, &args![0u32]);
                if e.vcall(starter, 0x1e4, &args![]).u32() != 0 && is_idle_kind(e, starter) {
                    process_call(e, starter, 0x614, &args![0x800u32]);
                    process_call(e, starter, 0x71c, &args![0u32]);
                }
            }
        }
        if target_ok
            && target != 0
            && starter != target
            && target != player
            && process_of(e, target) != 0
        {
            if !e.call(0x0067_27b0, &args![package]).bool() {
                process_call(e, target, 0x650, &args![1u32]);
            }
            if !e.call(0x0067_27b0, &args![package]).bool() {
                process_call(e, target, 0x624, &args![starter]);
                e.mem.set_f32(target + 0x74, 1.0);
            }
            process_call(e, target, 0x374, &args![0u32]);
            process_call(e, target, 0x2ec, &args![0u32]);
            if current {
                if e.call(0x0093_44a0, &args![target]).u32() == package {
                    process_call(e, target, 0x214, &args![]);
                }
            } else {
                let target_package = process_call(e, target, 0x20c, &args![]).u32();
                if e.call(0x0093_36c0, &args![target]).bool()
                    && !e.call(0x0067_27b0, &args![package]).bool()
                {
                    if target_package != 0
                        && e.call(0x0041_ca90, &args![target_package]).i32() == 0x1c
                    {
                        process_call(e, target, 0x214, &args![]);
                    } else {
                        process_call(e, target, 0x714, &args![target]);
                    }
                }
            }
            if !e.vcall(target, 0x274, &args![1u32]).bool()
                && e.vcall(target, 0x1e4, &args![]).u32() != 0
                && is_idle_kind(e, target)
            {
                process_call(e, target, 0x614, &args![0x800u32]);
                process_call(e, target, 0x71c, &args![0u32]);
            }
        }
    } else if process_of(e, this.addr()) != 0 {
        let value = process_call(e, this.addr(), 0x674, &args![4u32]).u32();
        process_call(e, this.addr(), 0x650, &args![1u32]);
        process_call(e, this.addr(), 0x624, &args![value]);
        e.mem.set_f32(this.addr() + 0x74, 1.0);
        process_call(e, this.addr(), 0x71c, &args![0u32]);
    }
    let process = e.get(this, Actor::pCurrentProcess);
    if !process.is_null()
        && e.vcall(process.addr(), 0x14c, &args![]).u32() == 0
        && e.vcall(process.addr(), 0x148, &args![]).u32() != 0
    {
        let item = e.vcall(process.addr(), 0x148, &args![]).u32();
        let flag = e.call(0x004b_da70, &args![item, 6u32]).bool();
        let item = e.vcall(process.addr(), 0x148, &args![]).u32();
        let item_value = e.call(0x0044_ddc0, &args![item]).u32();
        let rate = e.call(0x0070_9430, &args![item_value, flag]).f64();
        let zero: f64 = e.global(ZERO_DOUBLE);
        if rate <= zero {
            let item = e.vcall(process.addr(), 0x148, &args![]).u32();
            let second = e.call(0x004b_da70, &args![item, 2u32]).bool();
            let drawn = e.call(0x008a_16d0, &args![this]).bool();
            let item = e.vcall(process.addr(), 0x148, &args![]).u32();
            let item_value = e.call(0x0044_ddc0, &args![item]).u32();
            e.vcall(this.addr(), 0x3ec, &args![item_value, drawn, second, 0u32]);
        }
    }
}

/// `actor->pCurrentProcess` through `008d8520`.
fn process_of(e: &mut Engine, actor: u32) -> u32 {
    e.call(ACTOR_GET_PROCESS, &args![actor]).u32()
}

/// Virtual call on the process of `actor`, fetched through `008d8520` first.
fn process_call(e: &mut Engine, actor: u32, slot: u32, arguments: &[u32]) -> Ret {
    let process = process_of(e, actor);
    e.vcall(process, slot, arguments)
}

/// True when the actor's slot `0x214` answers 0, 9 or 4.
fn is_idle_kind(e: &mut Engine, actor: u32) -> bool {
    e.vcall(actor, 0x214, &args![]).u32() == 0
        || e.vcall(actor, 0x214, &args![]).u32() == 9
        || e.vcall(actor, 0x214, &args![]).u32() == 4
}
// Translated from 008b1910 (decompiled, FalloutNV.exe 1.4.0.525)
/// Records `talker` as the talking actor in its extra data list
/// (`0042e060` with the form `007af430` gives), calls its virtual slot
/// `0x48` with `0x80000000`, and reads the talking-actor extra back
/// (`0042e110`). When that exists, its word at +0xC is stored at `out` and
/// becomes the result, replaced by the form's `0x90` field (`00516bf0`) when
/// that is not null; otherwise the result is `talker`. The result is handed
/// to the process (virtual slot `0x12C`) and returned.
pub fn fn_008b1910(e: &mut Engine, this: Ptr<Actor>, talker: Ptr, out: Ptr) -> u32 {
    let form = e.call(0x007a_f430, &args![talker]).u32();
    let extra = e.call(0x005d_43c0, &args![talker]).u32();
    e.call(0x0042_e060, &args![extra, form, talker]);
    e.vcall(talker.addr(), 0x48, &args![0x8000_0000u32]);
    let extra = e.call(0x005d_43c0, &args![talker]).u32();
    let talking = e.call(0x0042_e110, &args![extra]).u32();
    let mut result = talker.addr();
    if talking != 0 {
        result = e.mem.u32(talking + 0xc);
        e.mem.set_u32(out.addr(), result);
        if e.call(0x0051_6bf0, &args![form]).u32() != 0 {
            result = e.call(0x0051_6bf0, &args![form]).u32();
        }
    }
    let process = e.get(this, Actor::pCurrentProcess);
    e.vcall(process.addr(), 0x12c, &args![result]);
    result
}

// Translated from 008b1ff0 (decompiled, FalloutNV.exe 1.4.0.525)
/// True when bit 1 of the word at `this + 0x1C` is set (the class is not
/// named in the engine map; `008b19c0` calls it on its package argument).
pub fn fn_008b1ff0(e: &mut Engine, this: Ptr) -> bool {
    e.mem.u32(this.addr() + 0x1c) & 2 != 0
}

// Translated from 008b2010 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Actor::CanUseIdle` (Xbox PDB): true when the idle `idle` (or, if that
/// one is a generic location, the end of its chain of `008041a0` links) is
/// among the root idle array or the loose idle array of the idle manager
/// for the directory name of the actor's model. The stack-cookie check of
/// the compiler is not translated.
pub fn actor_can_use_idle(e: &mut Engine, this: Ptr<Actor>, idle: u32) -> bool {
    if idle == 0 {
        return false;
    }
    let mut idle = idle;
    while e.call(0x0080_41a0, &args![idle]).u32() != 0 {
        idle = e.call(0x0080_41a0, &args![idle]).u32();
    }
    let idle_manager: u32 = e.global(0x011c_b6a0);
    e.with_stack(264, |e, directory| {
        // TESObjectREFR::GetModel (Xbox PDB), TESIdleManager::GetIdleDirName
        let model = e.call(0x0057_15d0, &args![this]).u32();
        e.call(0x0060_07f0, &args![model, directory]);
        for finder in [0x0060_0560u32, 0x0060_0630u32] {
            let array = e.call(finder, &args![idle_manager, directory]).u32();
            if array != 0 {
                let mut index = 0u32;
                while index < e.call(0x0084_e3a0, &args![array]).u32() {
                    if e.call(0x004d_6170, &args![array, index]).u32() == idle {
                        return true;
                    }
                    index += 1;
                }
            }
        }
        false
    })
}

/// `DialoguePackage::DialoguePackage` (Xbox PDB).
const DIALOGUE_PACKAGE_CONSTRUCTOR: u32 = 0x009e_dd80;
/// `operator new`.
const OPERATOR_NEW: u32 = 0x0040_1000;

// Translated from 008b19c0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Starts a conversation: builds a dialogue package (`DialoguePackage`,
/// package kind `0x1C`) from `source`, a package whose settings it copies
/// (location, target, topic, flags, secondary location), and runs it.
///
/// Takes the talk partner from the process (slot `0x128`; when there is none
/// the process is first asked through slot `0x8C`), either directly when it
/// is a mobile object (slot `0xFC`) or through the extra talking-actor data
/// when its form kind is `0x16`; returns false without a partner that has a
/// process. The partner and this actor both get the package (slots `0x710`
/// and `0x2F4`) unless it is the player or this actor itself. The
/// compiler's exception frame is not translated. The original also has a
/// block at `008b1afc` (testing a partner found through slot `0x300`) that
/// can never run: the partner is still null at that point, so it is left out.
pub fn fn_008b19c0(e: &mut Engine, this: Ptr<Actor>, source: u32) -> bool {
    let player: u32 = e.global(PLAYER_CHARACTER);
    let process = process_of(e, this.addr());
    e.vcall(process, 0x22c, &args![]);
    let process = e.get(this, Actor::pCurrentProcess);
    let mut talker = e.vcall(process.addr(), 0x128, &args![]).u32();
    let mut partner = 0u32;
    let mut partner_extra = 0u32;
    if talker == 0 {
        let process = process_of(e, this.addr());
        e.vcall(process, 0x8c, &args![this]);
    }
    let process = e.get(this, Actor::pCurrentProcess);
    talker = e.vcall(process.addr(), 0x128, &args![]).u32();
    if talker == 0 {
        return false;
    }
    if e.vcall(talker, 0xfc, &args![]).bool() {
        partner = talker;
    } else {
        let form = e.call(0x007a_f430, &args![talker]).u32();
        if e.call(0x0040_1170, &args![form]).u32() == 0x16 {
            let form = e.call(0x007a_f430, &args![talker]).u32();
            let extra = e.call(0x005d_43c0, &args![talker]).u32();
            e.call(0x0042_e060, &args![extra, form, talker]);
            let extra = e.call(0x005d_43c0, &args![talker]).u32();
            let talking = e.call(0x0042_e110, &args![extra]).u32();
            partner = e.mem.u32(talking + 0xc);
            partner_extra = partner;
        }
    }
    if partner == 0 || process_of(e, partner) == 0 {
        return false;
    }
    if partner != this.addr() {
        e.vcall(partner, 0x288, &args![]);
    }
    let topic = e.call(0x0067_2760, &args![source]).u32();
    let process = e.get(this, Actor::pCurrentProcess);
    e.vcall(process.addr(), 0x214, &args![]);
    if !e.call(0x0067_a690, &args![source]).bool() && !e.call(0x0067_2800, &args![source]).bool() {
        process_call(e, this.addr(), 0x644, &args![1u32]);
        process_call(e, this.addr(), 0x634, &args![partner]);
    }
    let memory = e.call(OPERATOR_NEW, &args![0xd0u32]).u32();
    let package = if memory != 0 {
        // `005f36f0` (named `ActorMover::GetPreferredMoveMode` in the map,
        // but a folded getter) is tested for zero.
        let getter_is_zero = e.call(0x005f_36f0, &args![source]).u32() == 0;
        e.call(
            DIALOGUE_PACKAGE_CONSTRUCTOR,
            &args![
                memory,
                0u32,
                this,
                partner,
                partner_extra,
                0u32,
                getter_is_zero
            ],
        )
        .u32()
    } else {
        0
    };
    if source != 0 {
        e.call(0x0067_a1b0, &args![package, source]);
    }
    // TESPackage::SetPackType (Xbox PDB)
    e.call(0x0067_0fc0, &args![package, 0x1cu32]);
    let flag = fn_008b1ff0(e, Ptr::new(source));
    e.call(0x0082_6b40, &args![package, flag]);
    let flag = e.call(0x0067_efd0, &args![source]).bool();
    e.call(0x0082_6b90, &args![package, flag]);
    e.call(0x0098_4f60, &args![package, 10u32]);
    let location = e.call(0x0055_b980, &args![source]).u32();
    // TESPackage::SetPackageLocation (Xbox PDB)
    e.call(0x0067_1d30, &args![package, location]);
    e.mem.set_u32(package + 0x8c, topic);
    if e.call(0x0067_2dd0, &args![source]).u32() != 0 {
        let second = e.call(0x0067_2dd0, &args![source]).u32();
        // TESPackage::SetPackageSecondLocation (Xbox PDB)
        e.call(0x0067_1e10, &args![package, second]);
    }
    let memory = e.call(OPERATOR_NEW, &args![0x10u32]).u32();
    let target = if memory != 0 {
        // PackageTarget::PackageTarget (Xbox PDB)
        e.call(0x0067_ff70, &args![memory]).u32()
    } else {
        0
    };
    // PackageTarget::SetTargReference, SetTargType (Xbox PDB)
    e.call(0x0068_0110, &args![target, partner]);
    e.call(0x0068_00b0, &args![target, 0u32]);
    let holder = e.call(0x0067_1d10, &args![source]).u32();
    let value = e.call(0x0044_ddc0, &args![holder]).u32();
    e.call(0x0040_3550, &args![target, value]);
    // TESPackage::SetPackageTarget (Xbox PDB)
    e.call(0x0067_2fc0, &args![package, target]);
    if target != 0 {
        e.call(0x007b_3fa0, &args![target, 1u32]);
    }
    let flag = e.call(0x0067_27b0, &args![source]).bool();
    e.call(0x0067_2aa0, &args![package, flag]);
    let seconds = e.call(0x0067_2850, &args![source]).f32();
    e.call(0x0067_2c40, &args![package, seconds]);
    let topic = e.call(0x0067_2760, &args![source]).u32();
    e.call(0x0067_29d0, &args![package, topic]);
    let flag = e.call(0x0067_2800, &args![source]).bool();
    e.call(0x0067_2b70, &args![package, flag]);
    process_call(e, this.addr(), 0x28, &args![]);
    // MiddleHighProcess::SetBSBound (Xbox PDB), on the player singleton
    e.call(0x005c_8a30, &args![player, package]);
    process_call(e, this.addr(), 0x710, &args![this]);
    e.vcall(this.addr(), 0x2f4, &args![package, 0u32, 1u32]);
    if process_call(e, this.addr(), 0x280, &args![]).u32() != 1 {
        process_call(e, this.addr(), 0x238, &args![1u32]);
    }
    let process = e.get(this, Actor::pCurrentProcess);
    e.vcall(process.addr(), 0x12c, &args![partner]);
    let process = e.get(this, Actor::pCurrentProcess);
    e.call(0x0048_3710, &args![process]);
    let process = e.get(this, Actor::pCurrentProcess);
    e.call(0x0048_3710, &args![process]);
    e.call(0x0057_bd60, &args![this, partner]);
    if !e.call(0x0067_27b0, &args![source]).bool() && e.call(0x0067_2dd0, &args![source]).u32() == 0
    {
        if partner != player && partner != this.addr() {
            process_call(e, partner, 0x710, &args![partner]);
            let current = e.call(0x0093_44a0, &args![this]).u32();
            e.vcall(partner, 0x2f4, &args![current, 0u32, 1u32]);
            process_call(e, partner, 0x12c, &args![partner]);
            process_call(e, partner, 0x288, &args![partner, 1u32]);
            e.call(0x0057_bd60, &args![partner, this]);
        } else if !e.vcall(partner, 0x22c, &args![0u32]).bool()
            && partner != player
            && partner != this.addr()
        {
            process_call(e, this.addr(), 0x288, &args![this, 3u32]);
            return true;
        }
    }
    let process = e.get(this, Actor::pCurrentProcess);
    let result = e.vcall(process.addr(), 0x90, &args![this, 1u32]).u32();
    e.mem.set_u32(package + 0x9c, result);
    true
}
/// `PackageLocation::PackageLocation` (Xbox PDB).
const PACKAGE_LOCATION_CONSTRUCTOR: u32 = 0x0067_f030;
/// The destructor of a `PackageLocation` (`0067f110`).
const PACKAGE_LOCATION_DESTRUCTOR: u32 = 0x0067_f110;
/// `Conversation::Conversation_ov2` (Xbox PDB).
const CONVERSATION_CONSTRUCTOR: u32 = 0x0083_b850;

// Translated from 008b2170 (decompiled, FalloutNV.exe 1.4.0.525)
/// Starts a scripted conversation of this actor with `partner`: builds a
/// `Conversation` (unless the partner is the player) and a dialogue package
/// (`DialoguePackage`, kind `0x1C`) configured from the arguments and from
/// the process's current package (slot `0x22C`), makes both actors run it
/// and returns true.
///
/// Returns false without a partner or a partner process, when the partner is
/// a mobile object that is trying to enter furniture, when this actor is
/// dismembered in limb 1 or 2 while slot `0x22C` answers true, and when the
/// partner (not the player) is not given the package. `location` and
/// `second_location` (null or `PackageLocation`s) are copied into the
/// package's locations; the second one gets its radius from `00676280` plus
/// a game setting. `topic` is stored in the package (+0x8C) and set as its
/// dialogue topic. The compiler's exception frame is not translated.
#[allow(clippy::too_many_arguments)]
pub fn fn_008b2170(
    e: &mut Engine,
    this: Ptr<Actor>,
    partner: Ptr<Actor>,
    location: Ptr,
    second_location: Ptr,
    head_track: bool,
    option_a: bool,
    option_b: bool,
    topic: u32,
    option_c: bool,
    option_d: bool,
) -> bool {
    let player: u32 = e.global(PLAYER_CHARACTER);
    if process_of(e, this.addr()) == 0 {
        return false;
    }
    let current_package = process_call(e, this.addr(), 0x22c, &args![]).u32();
    let mut mobile = 0u32;
    if !partner.is_null() && e.vcall(partner.addr(), 0x100, &args![]).bool() {
        mobile = partner.addr();
    }
    if partner.is_null()
        || (mobile != 0 && e.call(0x008c_13d0, &args![mobile]).bool())
        || process_of(e, partner.addr()) == 0
    {
        return false;
    }
    if e.vcall(this.addr(), 0x22c, &args![0u32]).bool()
        && (e.call(0x0057_3090, &args![this, 1u32]).bool()
            || e.call(0x0057_3090, &args![this, 2u32]).bool())
    {
        return false;
    }
    let mut conversation = 0u32;
    if partner.addr() != player {
        let memory = e.call(OPERATOR_NEW, &args![0xcu32]).u32();
        conversation = if memory != 0 {
            e.call(
                CONVERSATION_CONSTRUCTOR,
                &args![memory, this, partner, topic],
            )
            .u32()
        } else {
            0
        };
    }
    // MobileObject::StopCurrentDialogue (Xbox PDB)
    e.call(0x0093_4250, &args![this]);
    let process = e.get(this, Actor::pCurrentProcess);
    e.vcall(process.addr(), 0x214, &args![]);
    if partner.addr() != player && !e.call(0x0083_b9a0, &args![conversation]).bool() {
        // Conversation::FirstItem found nothing: drop the conversation.
        if conversation != 0 {
            e.call(0x0079_8450, &args![conversation, 1u32]);
        }
        return false;
    }
    if head_track {
        process_call(e, this.addr(), 0x644, &args![1u32]);
        process_call(e, this.addr(), 0x634, &args![partner]);
        process_call(e, partner.addr(), 0x644, &args![1u32]);
        process_call(e, partner.addr(), 0x634, &args![this]);
    }
    let memory = e.call(OPERATOR_NEW, &args![0xd0u32]).u32();
    let package = if memory != 0 {
        e.call(
            DIALOGUE_PACKAGE_CONSTRUCTOR,
            &args![memory, conversation, this, partner, 0u32, 1u32, 1u32],
        )
        .u32()
    } else {
        0
    };
    e.mem.set_u32(package + 0x8c, topic);
    if current_package != 0 {
        e.call(0x0067_a1b0, &args![package, current_package]);
    }
    e.call(0x0067_0fc0, &args![package, 0x1cu32]);
    e.call(0x0082_6b40, &args![package, 1u32]);
    e.call(0x0082_6b90, &args![package, 1u32]);
    e.call(0x0067_29d0, &args![package, topic]);
    fn_008b2860(e, Ptr::new(package), option_a as u8);
    fn_008b2880(e, Ptr::new(package), option_c as u8);
    e.call(0x0067_2aa0, &args![package, option_c]);
    e.call(0x0067_a6b0, &args![package, !head_track]);
    fn_008b28a0(e, Ptr::new(package), option_d as u8);
    // Two `PackageLocation`s on the stack, 0xC bytes each.
    let locations = e.mem.alloc(0x18);
    let (first, second) = (locations, locations + 0xc);
    e.call(PACKAGE_LOCATION_CONSTRUCTOR, &args![first]);
    if !location.is_null() {
        e.call(0x0067_f4d0, &args![first, location, 1u32]);
    }
    e.call(0x0067_f3c0, &args![first, this]);
    e.call(0x0067_1d30, &args![package, first]);
    e.call(PACKAGE_LOCATION_CONSTRUCTOR, &args![second]);
    if !second_location.is_null() {
        e.call(0x0067_f4d0, &args![second, second_location, 1u32]);
    }
    let result = 'conversation: {
        let distance = e.call(0x0067_6280, &args![package, this]).u32();
        let setting = e.call(0x0040_3e20, &args![0x011c_cf38u32]).u32();
        let setting = e.mem.f32(setting);
        let radius = (setting as f64 + distance as f64) as i64 as u32;
        e.call(0x0067_f1f0, &args![second, radius]);
        e.call(0x0067_1e10, &args![package, second]);
        let memory = e.call(OPERATOR_NEW, &args![0x10u32]).u32();
        let target = if memory != 0 {
            e.call(0x0067_ff70, &args![memory]).u32()
        } else {
            0
        };
        e.call(0x0067_2fc0, &args![package, target]);
        if target != 0 {
            e.call(0x007b_3fa0, &args![target, 1u32]);
        }
        e.call(0x0098_4f60, &args![package, 10u32]);
        let holder = e.call(0x0067_1d10, &args![package]).u32();
        e.call(0x0068_00b0, &args![holder, 0u32]);
        let holder = e.call(0x0067_1d10, &args![package]).u32();
        e.call(0x0068_0110, &args![holder, partner]);
        let kind = process_call(e, this.addr(), 0x4bc, &args![]).u32();
        let holder = e.call(0x0067_1d10, &args![package]).u32();
        e.call(
            0x0040_3550,
            &args![holder, if kind == 4 { 0xc8u32 } else { 0x5au32 }],
        );
        process_call(e, this.addr(), 0x28, &args![]);
        e.vcall(this.addr(), 0x2f4, &args![package, 1u32, 1u32]);
        e.call(0x0057_bd60, &args![this, partner]);
        let process = e.get(this, Actor::pCurrentProcess);
        if option_a {
            e.vcall(process.addr(), 0x288, &args![this, 2u32]);
        } else if option_b {
            e.vcall(process.addr(), 0x288, &args![this, 1u32]);
        }
        let process = e.get(this, Actor::pCurrentProcess);
        e.call(0x0048_3710, &args![process]);
        let process = e.get(this, Actor::pCurrentProcess);
        e.call(0x0048_3710, &args![process]);
        if partner.addr() != player && partner != this && (!option_c || option_d) {
            e.vcall(partner.addr(), 0x2f4, &args![package, 1u32, 1u32]);
            if e.call(0x0093_44a0, &args![partner]).u32() != package {
                e.vcall(this.addr(), 0x288, &args![]);
                break 'conversation false;
            }
            e.call(0x0057_bd60, &args![partner, this]);
            if option_a {
                process_call(e, partner.addr(), 0x288, &args![partner, 2u32]);
            } else {
                process_call(e, partner.addr(), 0x288, &args![partner, 1u32]);
            }
        }
        true
    };
    e.call(PACKAGE_LOCATION_DESTRUCTOR, &args![second]);
    e.call(PACKAGE_LOCATION_DESTRUCTOR, &args![first]);
    e.mem.free(locations);
    result
}

// Translated from 008b2860 (decompiled, FalloutNV.exe 1.4.0.525)
/// Sets the byte at +0xBE of a `DialoguePackage`.
pub fn fn_008b2860(e: &mut Engine, this: Ptr, value: u8) {
    e.mem.set_u8(this.addr() + 0xbe, value);
}

// Translated from 008b2880 (decompiled, FalloutNV.exe 1.4.0.525)
/// Sets the byte at +0xBF of a `DialoguePackage`.
pub fn fn_008b2880(e: &mut Engine, this: Ptr, value: u8) {
    e.mem.set_u8(this.addr() + 0xbf, value);
}

// Translated from 008b28a0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Sets the byte at +0xC0 of a `DialoguePackage`.
pub fn fn_008b28a0(e: &mut Engine, this: Ptr, value: u8) {
    e.mem.set_u8(this.addr() + 0xc0, value);
}

// Translated from 008b2b60 (decompiled, FalloutNV.exe 1.4.0.525)
/// Adds 3 to `base` when `animation` is (or is not) the player's
/// first-person animation and `item` agrees (`item` null, or for the
/// first-person animation [`fn_008b2bf0`] / otherwise [`fn_008b2c10`] true).
/// Returns `base` unchanged when `item` is set and its value (`00508070`,
/// a float) is not positive. `this` (ECX) is not read; the caller
/// (`008b28c0`) passes the equipped item's value there.
pub fn fn_008b2b60(e: &mut Engine, _unused_this: Ptr, animation: u32, base: i32, item: Ptr) -> i32 {
    if !item.is_null() {
        let value = e.call(0x0050_8070, &args![item]).f64();
        let zero: f64 = e.global(ZERO_DOUBLE);
        if !(value > zero || value.is_nan()) {
            return base;
        }
    }
    let player: u32 = e.global(PLAYER_CHARACTER);
    let first_person = e.call(PLAYER_GET_ANIMATION, &args![player, 1u32]).u32();
    if animation == first_person && (item.is_null() || fn_008b2bf0(e, item)) {
        return base.wrapping_add(3);
    }
    let first_person = e.call(PLAYER_GET_ANIMATION, &args![player, 1u32]).u32();
    if animation != first_person && (item.is_null() || fn_008b2c10(e, item)) {
        return base.wrapping_add(3);
    }
    base
}

// Translated from 008b2bf0 (decompiled, FalloutNV.exe 1.4.0.525)
/// True when bit `0x40` of the byte at `this + 0x100` is clear.
pub fn fn_008b2bf0(e: &mut Engine, this: Ptr) -> bool {
    e.mem.u8(this.addr() + 0x100) & 0x40 == 0
}

// Translated from 008b2c10 (decompiled, FalloutNV.exe 1.4.0.525)
/// True when bit `0x100` of the word at `this + 0x12C` is clear.
pub fn fn_008b2c10(e: &mut Engine, this: Ptr) -> bool {
    e.mem.u32(this.addr() + 0x12c) & 0x100 == 0
}
// Translated from 008b28c0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Plays an animation group (and, for attack and aim actions, up to two
/// follow-up groups) on `animation` and tells the process (slot `0x424`).
///
/// `anim_group` is the requested group; for attack and aim actions
/// (`005f2540`, `005f2630`) three groups are tried (`anim_group + 0`, `+1`,
/// `+2`), otherwise one. With an item that has iron sights the group is
/// adjusted by `008b2b60` first. The first group is looked up with
/// `Actor::GetAnimGroup` (`00897910`); later ones continue from the best
/// found (`00495740`). A group found is played (`00494740`) and, unless
/// `animation` is the player's first-person animation, reported to the
/// process; none found clears group `i + 4` (`00496080`). Finally the process
/// is told it is done (slot `0x428`).
pub fn fn_008b28c0(e: &mut Engine, this: Ptr<Actor>, anim_group: u32, animation: Ptr) {
    let mut anim_group = anim_group;
    let player: u32 = e.global(PLAYER_CHARACTER);
    let mut count = 3i32;
    let mut best: u16 = 0xff;
    let attack_or_aim = e.call(0x005f_2540, &args![anim_group & 0xffff]).bool()
        || e.call(0x005f_2630, &args![anim_group & 0xffff]).bool();
    if !attack_or_aim {
        count = 1;
    }
    let mut i = 0i32;
    while i < count {
        let mut group = anim_group;
        let mut found: u16 = 0xff;
        let process = e.get(this, Actor::pCurrentProcess);
        let item_value = if e.vcall(process.addr(), 0x148, &args![]).u32() != 0 {
            let item = e.vcall(process.addr(), 0x148, &args![]).u32();
            e.call(0x0044_ddc0, &args![item]).u32()
        } else {
            0
        };
        if group != 0xff {
            if attack_or_aim && e.call(ACTOR_GET_IRON_SIGHTS, &args![this]).bool() {
                group = fn_008b2b60(
                    e,
                    this.cast(),
                    animation.addr(),
                    anim_group as i32,
                    Ptr::new(item_value),
                ) as u32;
                group = group.wrapping_add(i as u32);
            } else {
                group = anim_group.wrapping_add(i as u32);
            }
            if i == 0 {
                // Actor::GetAnimGroup (Xbox PDB)
                found = e
                    .call(0x0089_7910, &args![this, group, 0u32, 0u32, animation])
                    .u16();
                best = found;
                if attack_or_aim && e.call(0x005f_2750, &args![group]).bool() {
                    let kind = e.call(0x005f_2440, &args![found as u32]).u32();
                    if kind == group.wrapping_sub(3) {
                        group = group.wrapping_sub(3);
                        anim_group = anim_group.wrapping_sub(3);
                    }
                }
            } else if best != 0xff {
                best = best.wrapping_add(1);
                // Animation::PickBestAnimation (Xbox PDB)
                found = e
                    .call(0x0049_5740, &args![animation, best as u32, 0u32])
                    .u16();
            }
            if e.call(0x005f_2440, &args![found as u32]).u32() != group {
                found = 0xff;
            }
        }
        let first_person =
            |e: &mut Engine| e.call(PLAYER_GET_ANIMATION, &args![player, 1u32]).u32();
        if found != 0xff {
            // Animation::PlayGroup (Xbox PDB)
            let played = e
                .call(
                    0x0049_4740,
                    &args![animation, found as u32, 1u32, -1i32, -1i32],
                )
                .u32();
            if animation.addr() != first_person(e) {
                process_call(e, this.addr(), 0x424, &args![i, played]);
            }
        } else {
            // Animation::ClearGroup (Xbox PDB)
            e.call(0x0049_6080, &args![animation, i + 4, 0.0f32]);
            if animation.addr() != first_person(e) {
                process_call(e, this.addr(), 0x424, &args![i, 0u32]);
            }
        }
        i += 1;
    }
    let process = e.get(this, Actor::pCurrentProcess);
    e.vcall(process.addr(), 0x428, &args![this]);
}

// Translated from 008b2c30 (decompiled, FalloutNV.exe 1.4.0.525)
/// Scales the rotation of the player's weapon node by the weapon's cached
/// value (`008b0dd0` with selector 2, times a game setting unless `00894900`
/// says otherwise). Only runs for the player with an equipped item that
/// answers slot `0x454` of the process: finds the node (`008d6970`,
/// `004aae30`), makes sure it has a blend collision object
/// (`bhkBlendCollisionObjectAddRotation`, copied from an existing one of
/// the type at `0x12681cc`), then takes the Euler angles of the matrix built
/// from the animation's float at +0xD0, multiplies them by the scale and
/// sets the product with the identity-based matrix `0x011a9448` on the
/// collision object (`004f0110`). The compiler's exception frame is not
/// translated.
pub fn fn_008b2c30(e: &mut Engine, this: Ptr<Actor>) {
    let player: u32 = e.global(PLAYER_CHARACTER);
    if this.addr() != player {
        return;
    }
    let process = e.get(this, Actor::pCurrentProcess);
    if process.is_null() {
        return;
    }
    if e.vcall(process.addr(), 0x148, &args![]).u32() == 0 {
        return;
    }
    let process = e.get(this, Actor::pCurrentProcess);
    if !e.vcall(process.addr(), 0x454, &args![]).bool() {
        return;
    }
    let process = e.get(this, Actor::pCurrentProcess);
    let item = e.vcall(process.addr(), 0x148, &args![]).u32();
    let item_value = e.call(0x0044_ddc0, &args![item]).u32();
    // Three matrices of 9 floats (the rotation M, the constant copy R and a
    // product) and three Euler angles.
    e.with_stack(0x78, |e, block| {
        let matrix = block.addr();
        let constant = matrix + 0x24;
        let product = matrix + 0x48;
        let angles = matrix + 0x6c;
        e.call(0x0068_15c0, &args![matrix]);
        let animation = e.call(ACTOR_GET_ANIMATION, &args![this]).u32();
        // Float at animation +0xD0, handed on as an argument (the matrix
        // address stays on the stack for the call after).
        let rotation_value = e.call(0x0045_3700, &args![animation]).f32();
        let index = e.call(0x0044_6390, &args![item_value]).i32();
        let table_entry = e
            .mem
            .u32(0x0118_a838u32.wrapping_add((index as u32).wrapping_mul(4)));
        let node_name = e
            .call(0x008d_6970, &args![table_entry, rotation_value, matrix])
            .u32();
        if node_name == 0 {
            return;
        }
        let root = e.call(0x0043_fcd0, &args![this]).u32();
        let node = e.call(0x004a_ae30, &args![root, node_name]).u32();
        if node == 0 {
            return;
        }
        let mut scale = fn_008b0dd0(e, this, 2);
        if !e.call(0x0089_4900, &args![this]).bool() {
            let setting = e.call(0x0040_3e20, &args![0x011c_ec34u32]).u32();
            scale = (scale as f64 * e.mem.f32(setting) as f64) as f32;
        }
        let collision = e.call(0x0068_38b0, &args![node]).u32();
        if collision == 0 {
            let created = new_blend_collision_object(e);
            e.call(0x0062_bc90, &args![node, created]);
        } else {
            let collision = e.call(0x0068_38b0, &args![node]).u32();
            let source = e.call(0x0065_3270, &args![0x0126_7e64u32, collision]).u32();
            if source != 0 && e.call(0x0065_3270, &args![0x0126_81ccu32, source]).u32() == 0 {
                let created = new_blend_collision_object(e);
                e.call(0x00c9_0090, &args![created, source]);
                e.call(0x0062_bc90, &args![node, created]);
            }
        }
        let collision = e.call(0x0068_38b0, &args![node]).u32();
        let blend = e.call(0x0065_3270, &args![0x0126_81ccu32, collision]).u32();
        if blend == 0 {
            return;
        }
        for word in 0..9 {
            let value = e.mem.u32(0x011a_9448 + 4 * word);
            e.mem.set_u32(constant + 4 * word, value);
        }
        e.call(0x00a5_92c0, &args![matrix, angles, angles + 4, angles + 8]);
        let first = (e.mem.f32(angles) as f64 * scale as f64) as f32;
        let second = (e.mem.f32(angles + 4) as f64 * scale as f64) as f32;
        let third = (e.mem.f32(angles + 8) as f64 * scale as f64) as f32;
        e.call(0x00a5_9540, &args![matrix, first, second, third]);
        let result = e.call(0x0043_f8d0, &args![constant, product, matrix]).u32();
        for word in 0..9 {
            let value = e.mem.u32(result + 4 * word);
            e.mem.set_u32(constant + 4 * word, value);
        }
        e.call(0x004f_0110, &args![blend, constant]);
    });
}

/// A new `bhkBlendCollisionObjectAddRotation`: 0x50 bytes (`00aa13e0`)
/// constructed by `00c8ffd0`, or null when the allocation fails.
fn new_blend_collision_object(e: &mut Engine) -> u32 {
    let memory = e.call(0x00aa_13e0, &args![0x50u32]).u32();
    if memory != 0 {
        e.call(0x00c8_ffd0, &args![memory]).u32()
    } else {
        0
    }
}
// Translated from 008b2f90 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Actor::IsAtPoint` (Xbox PDB): true when `point` is within `radius` of
/// the actor, `00439ef0` taking the vector from the point to the actor's
/// position (virtual slot `0x1F4`) and `004b5470` comparing it with the
/// radius (0 or less is "within").
///
/// The vertical part of the vector is dropped when the point lies within
/// 30 units of the actor's height (its height, `008853a0`, or 128 when that
/// is 0) — unless `force_height_window` is false and the character
/// controller (`005c0880`) says to drop it anyway. With `add_actor_radius`
/// the radius is increased by the controller's radius (`00c6e280`, through
/// `004587d0`; doubled when `008a5170` says so, at least 32) or by 32
/// without a controller.
pub fn actor_is_at_point(
    e: &mut Engine,
    this: Ptr<Actor>,
    point: Ptr,
    radius: f32,
    add_actor_radius: bool,
    force_height_window: bool,
) -> bool {
    let position = e.vcall(this.addr(), 0x1f4, &args![]).u32();
    let controller = e.call(0x0093_06d0, &args![this]).u32();
    let mut radius = radius;
    // The actor's position (12 bytes) and the vector from the point to it.
    e.with_stack(0x18, |e, block| {
        let (actor_position, delta) = (block.addr(), block.addr() + 0xc);
        for word in 0..3 {
            let value = e.mem.u32(position + 4 * word);
            e.mem.set_u32(actor_position + 4 * word, value);
        }
        e.call(0x0043_9ef0, &args![actor_position, delta, point]);
        let zero: f64 = e.global(ZERO_DOUBLE);
        if !force_height_window
            && controller != 0
            && e.call(0x005c_0880, &args![controller]).u32() != 0
        {
            e.mem.set_f32(delta + 8, 0.0);
        } else {
            let mut height = e.call(0x0088_53a0, &args![this]).f32();
            if height as f64 == zero {
                height = e.global(0x0101_e704);
            }
            let margin: f64 = e.global(0x0101_db88);
            let z = e.mem.f32(actor_position + 8);
            let low = (z as f64 - margin) as f32;
            let high = ((z as f64 + height as f64) + margin) as f32;
            let point_z = e.mem.f32(point.addr() + 8);
            if low < point_z && point_z < high {
                e.mem.set_f32(delta + 8, 0.0);
            }
        }
        if add_actor_radius {
            let mut extra: f32 = e.global(0x0101_e340);
            if controller != 0 {
                let controller_radius = e.call(0x00c6_e280, &args![controller]).f32();
                extra = e.call(0x0045_87d0, &args![controller_radius]).f32();
                if e.call(0x008a_5170, &args![controller + 0x410]).bool() {
                    extra = (extra as f64 + extra as f64) as f32;
                }
                let minimum: f64 = e.global(0x0102_f070);
                if (extra as f64) < minimum {
                    extra = e.global(0x0101_e340);
                }
            }
            radius = (radius as f64 + extra as f64) as f32;
        }
        let result = e.call(0x004b_5470, &args![delta, radius]).i32();
        result <= 0
    })
}

// Translated from 008b30f0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Creates the actor's `ActorMover` (0x88 bytes, `009dad00`) and stores it in
/// `pActorMover`. The compiler's exception frame is not translated.
pub fn fn_008b30f0(e: &mut Engine, this: Ptr<Actor>) {
    let memory = e.call(OPERATOR_NEW, &args![0x88u32]).u32();
    let mover = if memory != 0 {
        e.call(0x009d_ad00, &args![memory, this]).u32()
    } else {
        0
    };
    e.set(this, Actor::pActorMover, Ptr::new(mover));
}

// Translated from 008b3180 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Actor::DestroyActorMover` (Xbox PDB): when the actor has extra data
/// (`005d43c0`), tells the path manager to forget the inventory pointers of
/// the actor's container changes (`00418520`, `0047d0b0`, `006ebd10`); then
/// deletes the `ActorMover` (virtual destructor, slot 0, with the delete
/// flag) and clears `pActorMover`.
pub fn actor_destroy_actor_mover(e: &mut Engine, this: Ptr<Actor>) {
    if e.call(0x005d_43c0, &args![this]).u32() != 0 {
        let extra = e.call(0x005d_43c0, &args![this]).u32();
        let changes = e.call(0x0041_8520, &args![extra]).u32();
        let manager = e.call(0x0047_d0b0, &args![changes]).u32();
        e.call(0x006e_bd10, &args![manager]);
    }
    let mover = e.get(this, Actor::pActorMover);
    if !mover.is_null() {
        e.vcall(mover.addr(), 0, &args![1u32]);
        e.set(this, Actor::pActorMover, Ptr::NULL);
    }
}

// Translated from 008b3200 (decompiled, FalloutNV.exe 1.4.0.525)
/// Passes `value` on to the `ActorMover` (`009dc950`) when there is one.
pub fn fn_008b3200(e: &mut Engine, this: Ptr<Actor>, value: u32) {
    let mover = e.get(this, Actor::pActorMover);
    if !mover.is_null() {
        e.call(0x009d_c950, &args![mover, value]);
    }
}

/// The path-finding argument that is 0 means "the process's slot `0x254`
/// for this actor".
fn default_pathing_argument(e: &mut Engine, this: Ptr<Actor>, value: u32) -> u32 {
    if value != 0 {
        return value;
    }
    process_call(e, this.addr(), 0x254, &args![this]).u32()
}

// Translated from 008b3630 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Actor::SetPathfindingGoal` (Xbox PDB): for a goal that fails `0069e390`,
/// passes the process's slot `0x254` result to `006d61e0` of the goal; then
/// hands the goal to the `ActorMover` (`009dba30`).
pub fn actor_set_pathfinding_goal(e: &mut Engine, this: Ptr<Actor>, goal: u32) {
    if goal != 0 && e.call(0x0069_e390, &args![goal]).u32() == 0 {
        let value = process_call(e, this.addr(), 0x254, &args![this]).u32();
        e.call(0x006d_61e0, &args![goal, value]);
    }
    let mover = e.get(this, Actor::pActorMover);
    e.call(0x009d_ba30, &args![mover, goal]);
}

// Translated from 008b3690 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Actor::SetPathfindingGoal_ov2` (Xbox PDB): `ActorMover::SetGoal`
/// (`009dba50`) with `last` defaulting to the process's slot `0x254` result.
pub fn actor_set_pathfinding_goal_ov2(
    e: &mut Engine,
    this: Ptr<Actor>,
    first: u32,
    second: u32,
    third: u32,
    fourth: f32,
    last: u32,
) {
    let last = default_pathing_argument(e, this, last);
    let mover = e.get(this, Actor::pActorMover);
    e.call(
        0x009d_ba50,
        &args![mover, first, second, third, fourth, last],
    );
}

// Translated from 008b36f0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Actor::SetPathfindingGoal_ov3` (Xbox PDB): `009dbb10` of the
/// `ActorMover`, with `last` defaulting to the process's slot `0x254` result.
pub fn actor_set_pathfinding_goal_ov3(
    e: &mut Engine,
    this: Ptr<Actor>,
    first: u32,
    second: f32,
    last: u32,
) {
    let last = default_pathing_argument(e, this, last);
    let mover = e.get(this, Actor::pActorMover);
    e.call(0x009d_bb10, &args![mover, first, second, last]);
}

// Translated from 008b3750 (decompiled, FalloutNV.exe 1.4.0.525)
/// `009dbb60` of the `ActorMover`, with `last` defaulting to the process's
/// slot `0x254` result.
#[allow(clippy::too_many_arguments)]
pub fn fn_008b3750(
    e: &mut Engine,
    this: Ptr<Actor>,
    first: u32,
    second: u32,
    third: u32,
    fourth: f32,
    fifth: f32,
    last: u32,
) {
    let last = default_pathing_argument(e, this, last);
    let mover = e.get(this, Actor::pActorMover);
    e.call(
        0x009d_bb60,
        &args![mover, first, second, third, fourth, fifth, last],
    );
}

// Translated from 008b37c0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `009dbc30` of the `ActorMover`, with `last` defaulting to the process's
/// slot `0x254` result.
pub fn fn_008b37c0(e: &mut Engine, this: Ptr<Actor>, first: u32, second: f32, last: u32) {
    let last = default_pathing_argument(e, this, last);
    let mover = e.get(this, Actor::pActorMover);
    e.call(0x009d_bc30, &args![mover, first, second, last]);
}

// Translated from 008b3230 (decompiled, FalloutNV.exe 1.4.0.525)
/// Per-frame update of the actor's movement speed and creature sound, for
/// every actor but the player (inside a profiling scope guard,
/// `00404eb0`/`00404ee0`, for `Actor.cpp` line `0x5D06`).
///
/// With a process: the elapsed time (`delta`, or when 0 and the process type
/// is 0 the setting at `0x011f6394`, times the VATS update multiplier for
/// the VATS target) is multiplied by the animation's float (`004e3d00`) when
/// slot 8 of the `MagicTarget` base answers a positive value for `0x33`,
/// and handed to the `ActorMover` (slot `0x14`).
///
/// A creature-type actor (virtual slot `0x21C`; form kind 6 or 7) with a
/// creature sound (`TESCreature::PickCreatureSound(10)`, `005f92d0`) and
/// something to follow (slot `0x1D0`) plays that sound while it should (the
/// flag from `004938e0`/`008a3b30` and slot `0x22C`) and stops it when it
/// should not, keeping the `BSSoundHandle` in its extra data (`00418980`
/// loads it, `0041a280` stores it). The compiler's exception frame is not
/// translated.
pub fn fn_008b3230(e: &mut Engine, this: Ptr<Actor>, delta: f32) {
    let guard = e.mem.alloc(8);
    e.call(
        0x0040_4eb0,
        &args![guard, 0x2du32, 1u32, 0x0108_4918u32, 0x5d06u32],
    );
    actor_frame_update(e, this, delta);
    e.call(0x0040_4ee0, &args![guard]);
    e.mem.free(guard);
}

/// The body of [`fn_008b3230`] between the scope guard's calls.
fn actor_frame_update(e: &mut Engine, this: Ptr<Actor>, delta: f32) {
    let player: u32 = e.global(PLAYER_CHARACTER);
    if this.addr() == player {
        return;
    }
    if process_of(e, this.addr()) != 0 {
        let zero: f64 = e.global(ZERO_DOUBLE);
        let mut elapsed = delta;
        if elapsed as f64 == zero && e.call(0x0093_1850, &args![this]).u32() == 0 {
            let mode = e.call(0x0044_ddc0, &args![0x011f_2250u32]).i32();
            let vats_target: u32 = e.global(0x011f_21cc);
            if mode == 4 && this.addr() == vats_target {
                let setting = e.call(0x0084_d030, &args![0x011f_6394u32]).f64();
                let multiplier = e.call(0x009c_8d60, &args![0x011f_2250u32]).f64();
                elapsed = (multiplier * setting) as f32;
            } else {
                elapsed = e.call(0x0084_d030, &args![0x011f_6394u32]).f32();
            }
        }
        if e.vcall(this.addr() + 0xa4, 8, &args![0x33u32]).i32() > 0
            && e.vcall(this.addr(), 0x1e4, &args![]).u32() != 0
        {
            let animation = e.vcall(this.addr(), 0x1e4, &args![]).u32();
            let factor = e.call(0x004e_3d00, &args![animation]).f64();
            elapsed = (factor * elapsed as f64) as f32;
        }
        let mover = e.get(this, Actor::pActorMover);
        e.vcall(mover.addr(), 0x14, &args![elapsed]);
    }
    if !e.vcall(this.addr(), 0x21c, &args![]).bool() {
        return;
    }
    let form = e.call(0x007a_f430, &args![this]).u32();
    let kind: i8 = if form == 0 {
        -1
    } else {
        e.call(0x0059_f3a0, &args![form]).u8() as i8
    };
    if kind != 6 && kind != 7 {
        return;
    }
    let sound = if form != 0 {
        e.call(0x005f_92d0, &args![form, 0xau32]).u32()
    } else {
        0
    };
    if sound == 0 || e.vcall(this.addr(), 0x1d0, &args![]).u32() == 0 {
        return;
    }
    let should_play = (e.call(0x0049_38e0, &args![this]).bool()
        || e.call(0x008a_3b30, &args![this]).bool())
        && !e.vcall(this.addr(), 0x22c, &args![0u32]).bool();
    // The sound handle (16 bytes), a copy of the creature sound's 36-byte
    // data, and the temporary handle the audio system fills.
    let block = e.mem.alloc(0x60);
    let handle = block;
    let sound_data = block + 0x20;
    let temporary = block + 0x44;
    e.call(0x0041_a250, &args![handle]);
    let extra = e.call(0x005d_43c0, &args![this]).u32();
    e.call(0x0041_8980, &args![extra, handle]);
    let store = |e: &mut Engine| {
        let extra = e.call(0x005d_43c0, &args![this]).u32();
        e.call(0x0041_a280, &args![extra, handle]);
    };
    if should_play && !e.call(0x00ad_8930, &args![handle]).bool() {
        if !e.call(0x00ad_8ce0, &args![handle]).bool() {
            let data = e.call(0x004e_75d0, &args![sound, sound_data]).u32();
            let first = e.mem.u32(data + 4);
            let flags = e.call(0x005e_39b0, &args![first]).u32() | 2;
            let file_name = e.call(0x0051_1840, &args![sound]).u32();
            let audio = e.call(0x0045_3a70, &args![]).u32();
            let created = e
                .call(
                    0x00ad_7480,
                    &args![audio, temporary, file_name, flags, sound],
                )
                .u32();
            e.call(0x0041_8900, &args![handle, created]);
            e.call(0x0048_3710, &args![temporary]);
            e.call(0x004f_15a0, &args![handle, 1u32]);
            store(e);
            let follow = e.vcall(this.addr(), 0x1d0, &args![]).u32();
            e.call(0x00ad_8f20, &args![handle, follow]);
        }
        if !e.call(0x00ad_8930, &args![handle]).bool() {
            let position = e.vcall(this.addr(), 0x1f4, &args![]).u32();
            e.call(0x0068_a7d0, &args![handle, position]);
            e.call(0x00ad_8830, &args![handle, 0u32]);
            store(e);
        }
    } else if !should_play && e.call(0x00ad_8930, &args![handle]).bool() {
        e.call(0x00ad_88f0, &args![handle]);
        store(e);
    }
    if e.vcall(this.addr(), 0x22c, &args![0u32]).bool()
        && e.call(0x00ad_8930, &args![handle]).bool()
    {
        e.call(0x00ad_88f0, &args![handle]);
        e.call(0x00ad_8d10, &args![handle]);
        store(e);
    }
    e.call(0x0048_3710, &args![handle]);
    e.mem.free(block);
}

/// `ActorMover` (Xbox PDB) of the actor: `pActorMover` (PC +0x190).
fn actor_mover(e: &mut Engine, this: Ptr<Actor>) -> Ptr {
    e.get(this, Actor::pActorMover)
}

/// `ActorMover::bStopMovementNextUpdate` (Xbox PDB +0x72), `bFaceTargetPoint`
/// (+0x73) and `bWaitingForPath` (+0x74): byte fields of the mover.
const MOVER_STOP_MOVEMENT_NEXT_UPDATE: u32 = 0x72;
const MOVER_FACE_TARGET_POINT: u32 = 0x73;
const MOVER_WAITING_FOR_PATH: u32 = 0x74;

// Translated from 008b3820 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Actor::GetCurrentPathingRequest` (Xbox PDB): `006838b0` of the
/// `ActorMover`, its result passed through.
pub fn actor_get_current_pathing_request(e: &mut Engine, this: Ptr<Actor>) -> u32 {
    let mover = actor_mover(e, this);
    e.call(0x0068_38b0, &args![mover]).u32()
}

// Translated from 008b3840 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Actor::GetCurrentPathfindingGoal` (Xbox PDB): `009de2b0` of the
/// `ActorMover`, its result passed through.
pub fn actor_get_current_pathfinding_goal(e: &mut Engine, this: Ptr<Actor>, goal: u32) -> u32 {
    let mover = actor_mover(e, this);
    e.call(0x009d_e2b0, &args![mover, goal]).u32()
}

// Translated from 008b3860 (decompiled, FalloutNV.exe 1.4.0.525)
/// `009de330` of the `ActorMover`, its result passed through.
pub fn fn_008b3860(e: &mut Engine, this: Ptr<Actor>, argument: u32) -> u32 {
    let mover = actor_mover(e, this);
    e.call(0x009d_e330, &args![mover, argument]).u32()
}

// Translated from 008b3880 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Actor::BuildRequest` (Xbox PDB): `009dbc90` with the actor as its first
/// word; when the actor's in-combat byte (`00493bb0`) is set, `008ad8f0(target, 0)`
/// follows on the first argument.
#[allow(clippy::too_many_arguments)]
pub fn actor_build_request(
    e: &mut Engine,
    this: Ptr<Actor>,
    target: u32,
    second: u32,
    third: u32,
    fourth: u32,
    value: f32,
    last: u32,
) {
    e.call(
        0x009d_bc90,
        &args![this, target, second, third, fourth, value, last],
    );
    if e.call(ACTOR_IS_IN_COMBAT, &args![this]).u8() != 0 {
        e.call(0x008a_d8f0, &args![target, 0u32]);
    }
}

// Translated from 008b38d0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Actor::BuildRequest_ov2` (Xbox PDB): resolves the cell and world space
/// from `place` (the actor's own, `008d6f30` and `TESObjectREFR::GetWorldSpace`
/// `00575d70`, when 0; `place` itself when its form kind (`00401170`) is `0x39`
/// for the first or `0x41` for the second) and builds the request with
/// [`actor_build_request`].
pub fn actor_build_request_ov2(
    e: &mut Engine,
    this: Ptr<Actor>,
    target: u32,
    second: u32,
    place: u32,
    value: f32,
    last: u32,
) {
    let mut first_place = 0;
    let mut second_place = 0;
    if place == 0 {
        first_place = e.call(0x008d_6f30, &args![this]).u32();
        second_place = e.call(0x0057_5d70, &args![this]).u32();
    } else if e.call(0x0040_1170, &args![place]).u32() == 0x39 {
        first_place = place;
    } else if e.call(0x0040_1170, &args![place]).u32() == 0x41 {
        second_place = place;
    }
    actor_build_request(
        e,
        this,
        target,
        second,
        first_place,
        second_place,
        value,
        last,
    );
}

// Translated from 008b3960 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Actor::IsPathValid` (Xbox PDB): `ActorMover::IsPathValid` (`009de3e0`).
pub fn actor_is_path_valid(e: &mut Engine, this: Ptr<Actor>) -> u32 {
    let mover = actor_mover(e, this);
    e.call(0x009d_e3e0, &args![mover]).u32()
}

// Translated from 008b3980 (decompiled, FalloutNV.exe 1.4.0.525)
/// `009dcab0` of the `ActorMover`, its result passed through.
pub fn fn_008b3980(e: &mut Engine, this: Ptr<Actor>) -> u32 {
    let mover = actor_mover(e, this);
    e.call(0x009d_cab0, &args![mover]).u32()
}

// Translated from 008b39a0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `009de110` of the `ActorMover`, its result passed through.
pub fn fn_008b39a0(e: &mut Engine, this: Ptr<Actor>, first: u32, second: u32) -> u32 {
    let mover = actor_mover(e, this);
    e.call(0x009d_e110, &args![mover, first, second]).u32()
}

// Translated from 008b39d0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Actor::GetFailedPathDestination` (Xbox PDB):
/// `ActorMover::GetFailedPathDestination` (`009de460`).
pub fn actor_get_failed_path_destination(e: &mut Engine, this: Ptr<Actor>) -> u32 {
    let mover = actor_mover(e, this);
    e.call(0x009d_e460, &args![mover]).u32()
}

// Translated from 008b39f0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Actor::SetMoveMode` (Xbox PDB): slot 4 of the `ActorMover`
/// (`SetMoveModePreference`).
pub fn actor_set_move_mode(e: &mut Engine, this: Ptr<Actor>, mode: u32) -> u32 {
    let mover = actor_mover(e, this);
    e.vcall(mover.addr(), 0x4, &args![mode]).u32()
}

// Translated from 008b3a20 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Actor::ForceMoveMode` (Xbox PDB): slot `0xC` of the `ActorMover`.
pub fn actor_force_move_mode(e: &mut Engine, this: Ptr<Actor>, mode: u32) -> u32 {
    let mover = actor_mover(e, this);
    e.vcall(mover.addr(), 0xc, &args![mode]).u32()
}

// Translated from 008b3a50 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Actor::ClearForcedMoveMode` (Xbox PDB): slot `0x10` of the `ActorMover`.
pub fn actor_clear_forced_move_mode(e: &mut Engine, this: Ptr<Actor>) -> u32 {
    let mover = actor_mover(e, this);
    e.vcall(mover.addr(), 0x10, &args![]).u32()
}

// Translated from 008b3a80 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Actor::ClearMoveMode` (Xbox PDB): slot 8 of the `ActorMover`
/// (`ClearMoveModePreference`), one stack argument.
pub fn actor_clear_move_mode(e: &mut Engine, this: Ptr<Actor>, argument: u32) -> u32 {
    let mover = actor_mover(e, this);
    e.vcall(mover.addr(), 0x8, &args![argument]).u32()
}

// Translated from 008b3ab0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Actor::StopMoving` (Xbox PDB): `ActorMover::StopMoving` (`009dd0a0`).
pub fn actor_stop_moving(e: &mut Engine, this: Ptr<Actor>) -> u32 {
    let mover = actor_mover(e, this);
    e.call(0x009d_d0a0, &args![mover]).u32()
}

// Translated from 008b3ad0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Actor::ForceStopMoving` (Xbox PDB): `ActorMover::ForceStopMoving`
/// (`009dd0c0`).
pub fn actor_force_stop_moving(e: &mut Engine, this: Ptr<Actor>) -> u32 {
    let mover = actor_mover(e, this);
    e.call(0x009d_d0c0, &args![mover]).u32()
}

// Translated from 008b3af0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Runs [`fn_008b3b10`] on the actor's `ActorMover`.
pub fn fn_008b3af0(e: &mut Engine, this: Ptr<Actor>) {
    let mover = actor_mover(e, this);
    fn_008b3b10(e, mover);
}

// Translated from 008b3b10 (decompiled, FalloutNV.exe 1.4.0.525)
/// `ActorMover` (`this`): clears `bStopMovementNextUpdate` (Xbox PDB +0x72).
pub fn fn_008b3b10(e: &mut Engine, this: Ptr) {
    e.mem
        .set_u8(this.addr() + MOVER_STOP_MOVEMENT_NEXT_UPDATE, 0);
}

// Translated from 008b3b30 (decompiled, FalloutNV.exe 1.4.0.525)
/// Writes a 12-byte vector to `out` and returns `out`: with `value <= 0.0`
/// the 12 bytes `00436aa0` of the actor returns, otherwise the result of
/// `009dcc80(mover, out, value)`.
pub fn fn_008b3b30(e: &mut Engine, this: Ptr<Actor>, out: Ptr, value: f32) -> Ptr {
    let zero: f64 = e.global(ZERO_DOUBLE);
    if f64::from(value) <= zero {
        let source = e.call(0x0043_6aa0, &args![this]).u32();
        for offset in [0, 4, 8] {
            let word = e.mem.u32(source + offset);
            e.mem.set_u32(out.addr() + offset, word);
        }
    } else {
        let mover = actor_mover(e, this);
        e.call(0x009d_cc80, &args![mover, out, value]);
    }
    out
}

// Translated from 008b3b90 (decompiled, FalloutNV.exe 1.4.0.525)
/// `0097f2f0` of the `ActorMover`, its result passed through.
pub fn fn_008b3b90(e: &mut Engine, this: Ptr<Actor>) -> u32 {
    let mover = actor_mover(e, this);
    e.call(0x0097_f2f0, &args![mover]).u32()
}

// Translated from 008b3bb0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Actor::IsPathingComplete` (Xbox PDB): `ActorMover::IsPathingComplete`
/// (`009dcd40`).
pub fn actor_is_pathing_complete(e: &mut Engine, this: Ptr<Actor>) -> u32 {
    let mover = actor_mover(e, this);
    e.call(0x009d_cd40, &args![mover]).u32()
}

// Translated from 008b3bd0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Actor::IsPathing` (Xbox PDB): `ActorMover::IsPathing` (`009dcdb0`).
pub fn actor_is_pathing(e: &mut Engine, this: Ptr<Actor>) -> u32 {
    let mover = actor_mover(e, this);
    e.call(0x009d_cdb0, &args![mover]).u32()
}

// Translated from 008b3bf0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Actor::IsWaitingOnPath` (Xbox PDB): [`fn_008b3c10`] on the actor's
/// `ActorMover`.
pub fn actor_is_waiting_on_path(e: &mut Engine, this: Ptr<Actor>) -> u8 {
    let mover = actor_mover(e, this);
    fn_008b3c10(e, mover)
}

// Translated from 008b3c10 (decompiled, FalloutNV.exe 1.4.0.525)
/// `ActorMover` (`this`): `bWaitingForPath` (Xbox PDB +0x74).
pub fn fn_008b3c10(e: &mut Engine, this: Ptr) -> u8 {
    e.mem.u8(this.addr() + MOVER_WAITING_FOR_PATH)
}

// Translated from 008b3c30 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Actor::IsRotating` (Xbox PDB): `ActorMover::IsRotating` (`009ddb50`).
pub fn actor_is_rotating(e: &mut Engine, this: Ptr<Actor>) -> u32 {
    let mover = actor_mover(e, this);
    e.call(0x009d_db50, &args![mover]).u32()
}

// Translated from 008b3c50 (decompiled, FalloutNV.exe 1.4.0.525)
/// `009dcb30` of the `ActorMover`, its result passed through.
pub fn fn_008b3c50(e: &mut Engine, this: Ptr<Actor>) -> u32 {
    let mover = actor_mover(e, this);
    e.call(0x009d_cb30, &args![mover]).u32()
}

// Translated from 008b3c70 (decompiled, FalloutNV.exe 1.4.0.525)
/// [`fn_008b3c90`] on the actor's `ActorMover`.
pub fn fn_008b3c70(e: &mut Engine, this: Ptr<Actor>) -> u8 {
    let mover = actor_mover(e, this);
    fn_008b3c90(e, mover)
}

// Translated from 008b3c90 (decompiled, FalloutNV.exe 1.4.0.525)
/// `ActorMover` (`this`): `bStopMovementNextUpdate` (Xbox PDB +0x72).
pub fn fn_008b3c90(e: &mut Engine, this: Ptr) -> u8 {
    e.mem.u8(this.addr() + MOVER_STOP_MOVEMENT_NEXT_UPDATE)
}

// Translated from 008b3cb0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `009de3a0` of the `ActorMover`, its result passed through.
pub fn fn_008b3cb0(e: &mut Engine, this: Ptr<Actor>, argument: u32) -> u32 {
    let mover = actor_mover(e, this);
    e.call(0x009d_e3a0, &args![mover, argument]).u32()
}

// Translated from 008b3cd0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Actor::SetLookAtTarget` (Xbox PDB): `ActorMover::SetLookAtTarget`
/// (`009de160`) with the point's three floats.
pub fn actor_set_look_at_target(e: &mut Engine, this: Ptr<Actor>, x: f32, y: f32, z: f32) -> u32 {
    let mover = actor_mover(e, this);
    e.call(0x009d_e160, &args![mover, x, y, z]).u32()
}

// Translated from 008b3d10 (decompiled, FalloutNV.exe 1.4.0.525)
/// `ActorMover` (`this`): `bFaceTargetPoint` (Xbox PDB +0x73).
pub fn fn_008b3d10(e: &mut Engine, this: Ptr) -> u8 {
    e.mem.u8(this.addr() + MOVER_FACE_TARGET_POINT)
}

// Translated from 008b3d30 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Actor::ClearLookAtTarget` (Xbox PDB): `ActorMover::ClearLookAtTarget`
/// (`009de230`).
pub fn actor_clear_look_at_target(e: &mut Engine, this: Ptr<Actor>) -> u32 {
    let mover = actor_mover(e, this);
    e.call(0x009d_e230, &args![mover]).u32()
}

// Translated from 008b3d50 (decompiled, FalloutNV.exe 1.4.0.525)
/// `00460140` of the `ActorMover`, its result passed through.
pub fn fn_008b3d50(e: &mut Engine, this: Ptr<Actor>) -> u32 {
    let mover = actor_mover(e, this);
    e.call(0x0046_0140, &args![mover]).u32()
}

/// `Actor::Dismember` (Xbox PDB): `(this, hit, actor value, body part, first extra,
/// second extra, chance, limit, flag)`.
const ACTOR_DISMEMBER: u32 = 0x008b_4d10;
/// The `ModelLoader` instance (`ModelLoader::LoadFile`'s `this`).
const MODEL_LOADER: u32 = 0x011c_3b3c;

// Translated from 008b3d70 (decompiled, FalloutNV.exe 1.4.0.525)
/// The actor's `0043fcd0` handle is used for a ray test along the direction
/// from the first point to the second (both passed by value, three floats
/// each): the segment is `512.0` long (`010231a0`), starts half of it (times
/// `01016248`) before the second point and goes along the normalised
/// direction (`00439ef0` subtracts, `00525340` normalises, `0045bb20`
/// scales); `008cfbc0` runs it. When that finds nothing a second test
/// (`00c806b0`, with a `006240d0` object configured by `008a50f0` and
/// `008a50d0(0102226c)`) is made and its hit is turned into an object
/// through `004ae750`, `004b5a20` and `0044ddc0`. Returns the result of
/// slot `0xC` of that object (0 without one). The compiler's stack
/// protector is not modelled.
#[allow(clippy::too_many_arguments)]
pub fn fn_008b3d70(
    e: &mut Engine,
    this: Ptr<Actor>,
    first_x: f32,
    first_y: f32,
    first_z: f32,
    second_x: f32,
    second_y: f32,
    second_z: f32,
) -> u32 {
    let world = e.call(0x0043_fcd0, &args![this]).u32();
    e.with_stack(0x80, |e, block| {
        let first = block.addr();
        let second = first + 0x0c;
        let direction = first + 0x18;
        let scaled = first + 0x24;
        let halved = first + 0x30;
        let start = first + 0x3c;
        let length = first + 0x48;
        let query = first + 0x50;
        let point_copy = first + 0x70;
        e.mem.set_f32(first, first_x);
        e.mem.set_f32(first + 4, first_y);
        e.mem.set_f32(first + 8, first_z);
        e.mem.set_f32(second, second_x);
        e.mem.set_f32(second + 4, second_y);
        e.mem.set_f32(second + 8, second_z);
        e.call(0x0043_9ef0, &args![second, direction, first]);
        e.call(0x0052_5340, &args![direction]);
        let segment: f32 = e.global(0x0102_31a0);
        e.mem.set_f32(length, segment);
        e.call(0x0045_bb20, &args![direction, scaled, segment]);
        let half: f32 = e.global(0x0101_6248);
        e.call(0x0045_bb20, &args![scaled, halved, half]);
        e.call(0x0043_9ef0, &args![second, start, halved]);
        let mut ray = vec![world];
        ray.extend((0..3).map(|word| e.mem.u32(start + 4 * word)));
        ray.extend((0..3).map(|word| e.mem.u32(direction + 4 * word)));
        ray.extend([length, 0]);
        let mut found = e.call(0x008c_fbc0, &ray).u32();
        if found == 0 {
            e.call(0x0062_40d0, &args![query]);
            e.call(0x0068_15c0, &args![point_copy]);
            e.call(0x004a_3e00, &args![point_copy, second]);
            e.call(0x008a_50f0, &args![query, point_copy]);
            let radius: f32 = e.global(0x0102_226c);
            e.call(0x008a_50d0, &args![query, radius]);
            let hit = e.call(0x00c8_06b0, &args![world, query, 0u32]).u32();
            if hit != 0 {
                let proxy = e.call(0x004a_e750, &args![hit]).u32();
                let owner = e.call(0x004b_5a20, &args![proxy]).u32();
                if owner != 0 {
                    found = e.call(0x0044_ddc0, &args![owner]).u32();
                }
            }
        }
        if found != 0 {
            e.vcall(found, 0xc, &args![]).u32()
        } else {
            0
        }
    })
}

// Translated from 008b3ef0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Finds which of the process's 15 slots (virtual slot `0x6D0` of the
/// process, index 0 to 14) holds `node` or one of its parents (`009611e0`),
/// walking up until the actor's own slot `0x1D0` object; `0xE` when a parent
/// equals the process's slot `0x190` answer for the actor's slot `0x1E8`;
/// -1 when nothing matches or the actor has no process.
pub fn fn_008b3ef0(e: &mut Engine, this: Ptr<Actor>, node: u32) -> i32 {
    let process = e.get(this, Actor::pCurrentProcess).addr();
    if node == 0 || process == 0 {
        return -1;
    }
    let stop = e.vcall(this.addr(), 0x1d0, &args![]).u32();
    let mut current = node;
    let key = e.vcall(this.addr(), 0x1e8, &args![]).u32();
    let target = e.vcall(process, 0x190, &args![key]).u32();
    while current != 0 && current != stop {
        if e.call(0x0096_11e0, &args![current]).u32() == target {
            return 0xe;
        }
        for index in 0..0xf_u32 {
            let process = e.get(this, Actor::pCurrentProcess).addr();
            if e.vcall(process, 0x6d0, &args![index]).u32() == current {
                return index as i32;
            }
        }
        current = e.call(0x0096_11e0, &args![current]).u32();
    }
    -1
}

// Translated from 008b3fe0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Spawns the model of a destroyed body part (a dismemberment piece).
/// The body part `part_index` of the actor's body part data (`004181e0` then
/// slot `0x180` of the result; `BGSBodyPartData::GetBodyPart` `005e50f0`) is
/// used when the actor has a parent cell (`008d6f30`) and the part passes
/// [`fn_008b4360`].
///
/// With `spawn_effects` set: a part that has `005e3fa0` triggers `009ac9c0`
/// (an explosion; arguments: that value, 0, 0, the actor's `008d6f30`, the
/// three floats at `impact + 0x24`, the nine words at `reference + 0x68`);
/// and a part with an object at `+0x68` (`008d8520`) and a non-zero
/// [`fn_008b4380`] calls `004fa6f0` on that object with the actor's scale
/// (`TESObjectREFR::GetScale`) times `009b0bd0` of the part.
///
/// When the part has a model path (`005d8a70`/`0048cee0`) and `reference` is
/// not 0, the model is loaded (`ModelLoader::LoadFile`, `00447080`), cloned
/// (`NiObject::Clone`, `00a5d2c0`) and placed: its local transform is
/// `inverse(actor reference transform, 008b42e0) * (reference + 0x68)`
/// (`0062c250`), set through `00440460`, `0043fa80` and the scale
/// `00440490(LowProcess::GetTrackedDamage)`. The model is released
/// afterwards (`0045a5e0`). Returns the clone, or 0. The compiler's
/// exception frame is not modelled.
pub fn fn_008b3fe0(
    e: &mut Engine,
    this: Ptr<Actor>,
    part_index: u32,
    reference: u32,
    impact: u32,
    spawn_effects: bool,
) -> u32 {
    let mut clone = 0;
    let base_form = e.call(0x0041_81e0, &args![this]).u32();
    let body_parts = e.vcall(base_form, 0x180, &args![]).u32();
    if body_parts == 0 || e.call(0x008d_6f30, &args![this]).u32() == 0 {
        return clone;
    }
    let part = e.call(0x005e_50f0, &args![body_parts, part_index]).u32();
    if part == 0 || !fn_008b4360(e, Ptr::new(part)) {
        return clone;
    }
    if spawn_effects {
        if e.call(0x005e_3fa0, &args![part]).u32() != 0 {
            let source = e.call(0x0046_1130, &args![reference]).u32();
            let matrix: Vec<u32> = (0..9).map(|word| e.mem.u32(source + 4 * word)).collect();
            let impact_position: Vec<u32> = (0..3)
                .map(|word| e.mem.u32(impact + 0x24 + 4 * word))
                .collect();
            let place = e.call(0x008d_6f30, &args![this]).u32();
            let value = e.call(0x005e_3fa0, &args![part]).u32();
            let mut words = vec![value, 0, 0, place];
            words.extend(impact_position);
            words.extend(matrix);
            e.call(0x009a_c9c0, &words);
        }
        let object = e.call(ACTOR_GET_PROCESS, &args![part]).u32();
        if object != 0 && fn_008b4380(e, Ptr::new(part)) != 0 {
            let scale = e.call(0x0056_7400, &args![this]).f64();
            let factor = e.call(0x009b_0bd0, &args![part]).f64();
            let value = (factor * scale) as f32;
            let count = fn_008b4380(e, Ptr::new(part)) as u32;
            let place = e.call(0x008d_6f30, &args![this]).u32();
            let object = e.call(ACTOR_GET_PROCESS, &args![part]).u32();
            e.call(
                0x004f_a6f0,
                &args![object, place, impact + 0x24, count, value],
            );
        }
    }
    let path_owner = e.call(0x005d_8a70, &args![part]).u32();
    if e.call(0x0048_cee0, &args![path_owner]).u32() == 0 || reference == 0 {
        return clone;
    }
    let path_owner = e.call(0x005d_8a70, &args![part]).u32();
    let path = e
        .vcall(path_owner, 0x14, &args![0u32, 1u32, 0u32, 0u32, 0u32])
        .u32();
    let loader = e.global::<u32>(MODEL_LOADER);
    let model = e.call(0x0044_7080, &args![loader, path]).u32();
    if model != 0 {
        clone = e.with_stack(0x100, |e, block| {
            let first_transform = block.addr();
            let second_transform = first_transform + 0x40;
            let scratch = first_transform + 0x80;
            let cloning = first_transform + 0xc0;
            e.call(0x004a_d050, &args![cloning, 1.0f32]);
            let clone = e.call(0x00a5_d2c0, &args![model, cloning]).u32();
            e.call(0x0047_6a80, &args![first_transform]);
            e.call(0x0047_6a80, &args![second_transform]);
            let owner = e.vcall(this.addr(), 0x1d0, &args![]).u32();
            let owner_transform = e.call(0x0046_1130, &args![owner]).u32();
            fn_008b42e0(e, Ptr::new(owner_transform), Ptr::new(first_transform));
            let reference_transform = e.call(0x0046_1130, &args![reference]).u32();
            let composed = e
                .call(
                    0x0062_c250,
                    &args![first_transform, scratch, reference_transform],
                )
                .u32();
            for word in 0..13 {
                let value = e.mem.u32(composed + 4 * word);
                e.mem.set_u32(second_transform + 4 * word, value);
            }
            e.call(0x0044_0460, &args![clone, second_transform + 0x24]);
            e.call(0x0043_fa80, &args![clone, second_transform]);
            let tracked = e.call(0x009b_88c0, &args![part]).f64() as f32;
            e.call(0x0044_0490, &args![clone, tracked]);
            e.call(0x004a_d270, &args![cloning]);
            clone
        });
    }
    let path_owner = e.call(0x005d_8a70, &args![part]).u32();
    let path = e.vcall(path_owner, 0x14, &args![]).u32();
    let loader = e.global::<u32>(MODEL_LOADER);
    e.call(0x0045_a5e0, &args![loader, path]);
    clone
}

// Translated from 008b42e0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `NiTransform` inverse: `dest` becomes the inverse of `this`. The rotation
/// is `NiMatrix3::Inverse_ov2` (`004b46a0`), the scale (`+0x30`) is
/// `1.0 / this.scale`, and the translation (`+0x24`) is `004a3760` of the
/// new scale and `dest.rotation` (`004b4500`) applied to `004a0bd0` of
/// `this`'s translation.
pub fn fn_008b42e0(e: &mut Engine, this: Ptr, dest: Ptr) {
    e.call(0x004b_46a0, &args![this, dest]);
    let scale = e.mem.f32(this.addr() + 0x30);
    let inverse_scale = (1.0 / f64::from(scale)) as f32;
    e.mem.set_f32(dest.addr() + 0x30, inverse_scale);
    e.with_stack(0x30, |e, scratch| {
        let first = scratch.addr();
        let negated = e.call(0x004a_0bd0, &args![this.addr() + 0x24, first]).u32();
        let rotated = e
            .call(0x004b_4500, &args![dest, first + 0x0c, negated])
            .u32();
        let scaled = e
            .call(0x004a_3760, &args![first + 0x18, inverse_scale, rotated])
            .u32();
        for offset in [0, 4, 8] {
            let word = e.mem.u32(scaled + offset);
            e.mem.set_u32(dest.addr() + 0x24 + offset, word);
        }
    });
}

// Translated from 008b4360 (decompiled, FalloutNV.exe 1.4.0.525)
/// Body part (`BGSBodyPartData` entry, `this`): flag `0x08` of the byte at `+0x60`.
pub fn fn_008b4360(e: &mut Engine, this: Ptr) -> bool {
    e.mem.u8(this.addr() + 0x60) & 0x08 != 0
}

// Translated from 008b4380 (decompiled, FalloutNV.exe 1.4.0.525)
/// Body part (`this`): the byte at `+0x66`.
pub fn fn_008b4380(e: &mut Engine, this: Ptr) -> u8 {
    e.mem.u8(this.addr() + 0x66)
}

// Translated from 008b4cd0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Body part (`this`): flag `0x40` of the byte at `+0x60`.
pub fn fn_008b4cd0(e: &mut Engine, this: Ptr) -> bool {
    e.mem.u8(this.addr() + 0x60) & 0x40 != 0
}

/// Number of dismemberment calls left this frame: the counter at
/// `011e077c` is bumped on every use of [`fn_008b43a0`] and compared with the
/// setting at `011e0a10`.
const DISMEMBER_COUNTER: u32 = 0x011e_077c;
const DISMEMBER_LIMIT_SETTING: u32 = 0x011e_0a10;
/// Setting whose byte, when non-zero, stops humanoid creatures from being dismembered.
const DISMEMBER_BLOCK_SETTING: u32 = 0x011d_f7f8;
const DISMEMBER_SETTING_LIMB_CHANCE: u32 = 0x011c_f18c;
const DISMEMBER_SETTING_FIRST_PART: u32 = 0x011c_e188;
const DISMEMBER_SETTING_OTHER_PART: u32 = 0x011c_fe90;
const DISMEMBER_SETTING_ROLL: u32 = 0x011c_e2b4;
const DISMEMBER_SETTING_ROLLED_PART: u32 = 0x011c_ee3c;

/// The value behind a setting object (`0043d4d0` returns its address).
fn setting_value(e: &mut Engine, setting: u32) -> u32 {
    let address = e.call(0x0043_d4d0, &args![setting]).u32();
    e.mem.u32(address)
}

/// Frame offsets (from the frame pointer) of the four 7-entry arrays of
/// [`fn_008b43a0`]. The original keeps them on the stack side by side and
/// can write one entry past the end; the frame is kept as one block so the
/// overlap is the same.
const DISMEMBER_PARTS: i32 = -0x20;
const DISMEMBER_VALUES: i32 = -0x7c;
const DISMEMBER_CHANCE: i32 = -0x5c;
const DISMEMBER_LIMIT: i32 = -0x40;

fn frame_index(base: i32, entry: u32) -> usize {
    ((0xdc + base + 4 * entry as i32) / 4) as usize
}

/// One list entry of [`fn_008b43a0`]: body part, actor value, two numbers.
fn dismember_push(frame: &mut [u32; 0x37], count: u32, entry: [u32; 4]) {
    frame[frame_index(DISMEMBER_PARTS, count)] = entry[0];
    frame[frame_index(DISMEMBER_VALUES, count)] = entry[1];
    frame[frame_index(DISMEMBER_CHANCE, count)] = entry[2];
    frame[frame_index(DISMEMBER_LIMIT, count)] = entry[3];
}

/// `BGSBodyPartData` entry test of `008b43a0`: the entry has flag `0x08`,
/// flag `0x40` and a byte `008b4cf0` of at least 100.
fn body_part_is_severable(e: &mut Engine, part: u32) -> bool {
    fn_008b4360(e, Ptr::new(part))
        && fn_008b4cd0(e, Ptr::new(part))
        && e.call(0x008b_4cf0, &args![part]).u8() >= 100
}

// Translated from 008b43a0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Picks the body parts to dismember for a hit and calls `Actor::Dismember`
/// (`008b4d10`) for each; `hit` is the hit record (its fields at 0, 8 and
/// `0x10` are the attacker, a part list owner and a body part index).
///
/// Nothing happens for a humanoid creature or an actor whose slot `0x218`
/// answers true while the setting at `011df7f8` is non-zero, without body
/// part data (`004181e0` slot `0x180`), without a hit, or when the per-frame
/// counter `011e077c` has reached the setting at `011e0a10`; otherwise the
/// counter is bumped and up to 7 entries (part, actor value `ToActorValue(4,
/// i)`, and two numbers) are collected: from the hit's body part and the
/// actor values (slot 8 of the `MagicTarget` at `+0xA4`) when the attacker's
/// value `0x37` is positive, from the list owner's list when its slot `0x220`
/// answers true, else from the hit's body part alone. With no entry
/// `Dismember` is called once with -1 for part and value; entries with value
/// `0x1a` are dismembered with flag `0x40` of the hit temporarily set
/// (`00407e00`) when the roll flag was set and the flag was not already.
pub fn fn_008b43a0(
    e: &mut Engine,
    this: Ptr<Actor>,
    hit: u32,
    first_extra: u32,
    second_extra: u32,
) {
    let form = e.call(0x0041_81e0, &args![this]).u32();
    let cast = e
        .call(
            0x00ec_43fb,
            &args![form, 0u32, 0x0118_46e8u32, 0x0118_3a00u32, 0u32],
        )
        .u32();
    let humanoid = cast != 0 && e.call(0x005f_bf20, &args![cast]).bool();
    if humanoid || e.vcall(this.addr(), 0x218, &args![]).bool() {
        let setting = e.call(0x0040_8d60, &args![DISMEMBER_BLOCK_SETTING]).u32();
        if e.mem.u8(setting) != 0 {
            return;
        }
    }
    let form = e.call(0x0041_81e0, &args![this]).u32();
    let body_parts = e.vcall(form, 0x180, &args![]).u32();
    if body_parts == 0 || hit == 0 {
        return;
    }
    let limit = setting_value(e, DISMEMBER_LIMIT_SETTING) as i32;
    let counter = e.global::<u32>(DISMEMBER_COUNTER) as i32;
    if counter >= limit {
        return;
    }
    let counter = e.global::<u32>(DISMEMBER_COUNTER);
    e.set_global::<u32>(DISMEMBER_COUNTER, counter.wrapping_add(1));

    let magic_target = this.addr() + 0xa4;
    let mut frame = [0u32; 0x37];
    for entry in 0..7 {
        frame[frame_index(DISMEMBER_PARTS, entry)] = u32::MAX;
        frame[frame_index(DISMEMBER_VALUES, entry)] = u32::MAX;
        frame[frame_index(DISMEMBER_CHANCE, entry)] = 0;
        frame[frame_index(DISMEMBER_LIMIT, entry)] = 0;
    }
    let mut count: u32 = 0;
    let mut rolled = false;
    let hit_attacker = e.mem.u32(hit);
    let hit_list_owner = e.mem.u32(hit + 8);
    let hit_part = e.mem.u32(hit + 0x10);

    if hit_attacker != 0 && e.vcall(hit_attacker + 0xa4, 8, &args![0x37u32]).i32() > 0 {
        let mut chosen_value: i32 = -1;
        let part = e.call(0x005e_50f0, &args![body_parts, hit_part]).u32();
        e.vcall(magic_target, 8, &args![0x1au32]);
        let mut forced_part: u32 = u32::MAX;
        let mut severable: bool;
        if part != 0 {
            let flag_set = e.call(0x0058_cba0, &args![hit, 0x40u32]).bool();
            severable = if flag_set && fn_008b4360(e, Ptr::new(part)) {
                true
            } else {
                body_part_is_severable(e, part)
            };
            let value = e.call(0x005e_5190, &args![part]).u8() as i8 as i32;
            if value == 0x1a {
                let take = if severable {
                    true
                } else if fn_008b4360(e, Ptr::new(part)) {
                    let roll = e.call(0x0048_7f50, &args![]).u32() % 100;
                    roll < setting_value(e, DISMEMBER_SETTING_LIMB_CHANCE)
                } else {
                    false
                };
                if take {
                    forced_part = e.call(0x005e_5300, &args![part]).u32();
                    rolled = true;
                }
            } else if count == 0 {
                chosen_value = e.call(0x005e_5190, &args![part]).u8() as i8 as i32;
                let limit = if severable {
                    100
                } else {
                    setting_value(e, DISMEMBER_SETTING_FIRST_PART)
                };
                dismember_push(
                    &mut frame,
                    count,
                    [hit_part, chosen_value as u32, 100, limit],
                );
                count += 1;
            }
        }
        let mut index: u32 = 0;
        while index < 7 && count < 7 {
            let value = e.call(0x0066_ec10, &args![4u32, index & 0xff]).u32() as i32;
            if value != chosen_value && value != -1 && value != 0x1a {
                let part = e.call(0x005e_5130, &args![body_parts, value as u32]).u32();
                if part != 0 {
                    severable = e.call(0x0058_cba0, &args![hit, 0x40u32]).bool()
                        || body_part_is_severable(e, part);
                    let current = e.vcall(magic_target, 8, &args![value as u32]).i32();
                    if current <= 0 {
                        let chosen = e.call(0x005e_5300, &args![part]).u32();
                        let limit = if severable {
                            100
                        } else {
                            setting_value(e, DISMEMBER_SETTING_OTHER_PART)
                        };
                        dismember_push(&mut frame, count, [chosen, value as u32, 100, limit]);
                        count += 1;
                    } else if severable
                        || rolled
                        || e.call(0x0048_7f50, &args![]).u32() % 100
                            <= setting_value(e, DISMEMBER_SETTING_ROLL)
                    {
                        let chosen = e.call(0x005e_5300, &args![part]).u32();
                        let limit = if severable {
                            100
                        } else {
                            setting_value(e, DISMEMBER_SETTING_ROLLED_PART)
                        };
                        dismember_push(&mut frame, count, [chosen, value as u32, 100, limit]);
                        count += 1;
                    }
                }
            }
            index += 1;
        }
        if rolled && forced_part != u32::MAX {
            dismember_push(&mut frame, count, [forced_part, 0x1a, 100, 100]);
            count += 1;
        }
    } else if hit_list_owner != 0 && e.vcall(hit_list_owner, 0x220, &args![]).bool() {
        let mut node = e.call(0x009b_1720, &args![hit_list_owner, this]).u32();
        while node != 0 && !e.call(0x0082_56d0, &args![node]).bool() && count < 7 {
            let slot = e.call(0x0068_15c0, &args![node]).u32();
            let entry = e.mem.u32(slot);
            node = e.call(0x0072_6070, &args![node]).u32();
            let part_key = e.mem.u32(entry);
            let part = e.call(0x005e_50f0, &args![body_parts, part_key]).u32();
            if part != 0 && e.mem.u32(entry) != 0 {
                let chosen = e.mem.u32(entry);
                let value = e.call(0x005e_5190, &args![part]).u8() as i8 as i32;
                frame[frame_index(DISMEMBER_PARTS, count)] = chosen;
                frame[frame_index(DISMEMBER_VALUES, count)] = value as u32;
                count += 1;
            }
        }
    } else {
        let part = e.call(0x005e_50f0, &args![body_parts, hit_part]).u32();
        let severable = part != 0 && body_part_is_severable(e, part);
        if part != 0 && (severable || hit_part != 0) {
            let value = e.call(0x005e_5190, &args![part]).u8() as i8 as i32;
            if value != -1 && e.vcall(magic_target, 8, &args![value as u32]).i32() <= 0 {
                frame[frame_index(DISMEMBER_PARTS, count)] = hit_part;
                frame[frame_index(DISMEMBER_VALUES, count)] = value as u32;
                count += 1;
            }
        }
    }

    if count == 0 {
        e.call(
            ACTOR_DISMEMBER,
            &args![
                this,
                hit,
                u32::MAX,
                u32::MAX,
                first_extra,
                second_extra,
                0u32,
                0u32,
                0u32
            ],
        );
    }
    for entry in 0..count {
        let part = frame[frame_index(DISMEMBER_PARTS, entry)];
        let value = frame[frame_index(DISMEMBER_VALUES, entry)];
        if part == u32::MAX || value == u32::MAX {
            continue;
        }
        let call_words = args![
            this,
            hit,
            value,
            part,
            first_extra,
            second_extra,
            frame[frame_index(DISMEMBER_CHANCE, entry)],
            frame[frame_index(DISMEMBER_LIMIT, entry)],
            rolled as u32
        ];
        if rolled && value == 0x1a && !e.call(0x0058_cba0, &args![hit, 0x40u32]).bool() {
            e.call(0x0040_7e00, &args![hit, 0x40u32, 1u32]);
            e.call(ACTOR_DISMEMBER, &call_words);
            e.call(0x0040_7e00, &args![hit, 0x40u32, 0u32]);
        } else {
            e.call(ACTOR_DISMEMBER, &call_words);
        }
    }
}
/// This part's translated functions, by exe address.
pub fn funcs() -> Vec<(u32, AbiFn)> {
    vec![
        entry!(0x008b00c0, fn_008b00c0(Ptr<Actor>)),
        entry!(0x008b0140, fn_008b0140(Ptr<Actor>, u8, u32)),
        entry!(0x008b0190, actor_is_visible(Ptr<Actor>, u32) -> bool),
        entry!(0x008b0520, fn_008b0520(Ptr<Actor>) -> bool),
        entry!(0x008b0670, fn_008b0670(Ptr<Actor>, Ptr<Actor>) -> bool),
        entry!(
            0x008b06d0,
            actor_get_should_attack_actor(Ptr<Actor>, Ptr<Actor>, bool, Ptr, bool) -> bool
        ),
        entry!(
            0x008b0970,
            actor_get_should_help(Ptr<Actor>, Ptr<Actor>) -> bool
        ),
        entry!(0x008b0ac0, fn_008b0ac0(Ptr<Actor>)),
        entry!(0x008b0b00, actor_reload_targets(Ptr<Actor>, u8)),
        entry!(0x008b0ba0, fn_008b0ba0(Ptr<Actor>) -> u32),
        entry!(
            0x008b0bd0,
            actor_init_lighting_property_ptr(Ptr<Actor>, Ptr)
        ),
        entry!(0x008b0d30, fn_008b0d30(Ptr, u32) -> f32),
        entry!(0x008b0d70, fn_008b0d70(Ptr, u32) -> f32),
        entry!(0x008b0db0, fn_008b0db0(Ptr, u8) -> f32),
        entry!(0x008b0dd0, fn_008b0dd0(Ptr<Actor>, i32) -> f32),
        entry!(0x008b01c0, actor_do_death_stuff(Ptr<Actor>)),
        entry!(0x008b1070, fn_008b1070(Ptr<Actor>)),
        entry!(0x008b1910, fn_008b1910(Ptr<Actor>, Ptr, Ptr) -> u32),
        entry!(0x008b19c0, fn_008b19c0(Ptr<Actor>, u32) -> bool),
        entry!(0x008b1ff0, fn_008b1ff0(Ptr) -> bool),
        entry!(0x008b2010, actor_can_use_idle(Ptr<Actor>, u32) -> bool),
        entry!(
            0x008b2170,
            fn_008b2170(
                Ptr<Actor>,
                Ptr<Actor>,
                Ptr,
                Ptr,
                bool,
                bool,
                bool,
                u32,
                bool,
                bool,
            ) -> bool
        ),
        entry!(0x008b2860, fn_008b2860(Ptr, u8)),
        entry!(0x008b2880, fn_008b2880(Ptr, u8)),
        entry!(0x008b28a0, fn_008b28a0(Ptr, u8)),
        entry!(0x008b2b60, fn_008b2b60(Ptr, u32, i32, Ptr) -> i32),
        entry!(0x008b2bf0, fn_008b2bf0(Ptr) -> bool),
        entry!(0x008b2c10, fn_008b2c10(Ptr) -> bool),
        entry!(0x008b28c0, fn_008b28c0(Ptr<Actor>, u32, Ptr)),
        entry!(0x008b2c30, fn_008b2c30(Ptr<Actor>)),
        entry!(
            0x008b2f90,
            actor_is_at_point(Ptr<Actor>, Ptr, f32, bool, bool) -> bool
        ),
        entry!(0x008b30f0, fn_008b30f0(Ptr<Actor>)),
        entry!(0x008b3180, actor_destroy_actor_mover(Ptr<Actor>)),
        entry!(0x008b3200, fn_008b3200(Ptr<Actor>, u32)),
        entry!(0x008b3230, fn_008b3230(Ptr<Actor>, f32)),
        entry!(0x008b3630, actor_set_pathfinding_goal(Ptr<Actor>, u32)),
        entry!(
            0x008b3690,
            actor_set_pathfinding_goal_ov2(Ptr<Actor>, u32, u32, u32, f32, u32)
        ),
        entry!(
            0x008b36f0,
            actor_set_pathfinding_goal_ov3(Ptr<Actor>, u32, f32, u32)
        ),
        entry!(
            0x008b3750,
            fn_008b3750(Ptr<Actor>, u32, u32, u32, f32, f32, u32)
        ),
        entry!(0x008b37c0, fn_008b37c0(Ptr<Actor>, u32, f32, u32)),
        entry!(
            0x008b3820,
            actor_get_current_pathing_request(Ptr<Actor>) -> u32
        ),
        entry!(
            0x008b3840,
            actor_get_current_pathfinding_goal(Ptr<Actor>, u32) -> u32
        ),
        entry!(0x008b3860, fn_008b3860(Ptr<Actor>, u32) -> u32),
        entry!(
            0x008b3880,
            actor_build_request(Ptr<Actor>, u32, u32, u32, u32, f32, u32)
        ),
        entry!(
            0x008b38d0,
            actor_build_request_ov2(Ptr<Actor>, u32, u32, u32, f32, u32)
        ),
        entry!(0x008b3960, actor_is_path_valid(Ptr<Actor>) -> u32),
        entry!(0x008b3980, fn_008b3980(Ptr<Actor>) -> u32),
        entry!(0x008b39a0, fn_008b39a0(Ptr<Actor>, u32, u32) -> u32),
        entry!(
            0x008b39d0,
            actor_get_failed_path_destination(Ptr<Actor>) -> u32
        ),
        entry!(0x008b39f0, actor_set_move_mode(Ptr<Actor>, u32) -> u32),
        entry!(0x008b3a20, actor_force_move_mode(Ptr<Actor>, u32) -> u32),
        entry!(0x008b3a50, actor_clear_forced_move_mode(Ptr<Actor>) -> u32),
        entry!(0x008b3a80, actor_clear_move_mode(Ptr<Actor>, u32) -> u32),
        entry!(0x008b3ab0, actor_stop_moving(Ptr<Actor>) -> u32),
        entry!(0x008b3ad0, actor_force_stop_moving(Ptr<Actor>) -> u32),
        entry!(0x008b3af0, fn_008b3af0(Ptr<Actor>)),
        entry!(0x008b3b10, fn_008b3b10(Ptr)),
        entry!(0x008b3b30, fn_008b3b30(Ptr<Actor>, Ptr, f32) -> Ptr),
        entry!(0x008b3b90, fn_008b3b90(Ptr<Actor>) -> u32),
        entry!(0x008b3bb0, actor_is_pathing_complete(Ptr<Actor>) -> u32),
        entry!(0x008b3bd0, actor_is_pathing(Ptr<Actor>) -> u32),
        entry!(0x008b3bf0, actor_is_waiting_on_path(Ptr<Actor>) -> u8),
        entry!(0x008b3c10, fn_008b3c10(Ptr) -> u8),
        entry!(0x008b3c30, actor_is_rotating(Ptr<Actor>) -> u32),
        entry!(0x008b3c50, fn_008b3c50(Ptr<Actor>) -> u32),
        entry!(0x008b3c70, fn_008b3c70(Ptr<Actor>) -> u8),
        entry!(0x008b3c90, fn_008b3c90(Ptr) -> u8),
        entry!(0x008b3cb0, fn_008b3cb0(Ptr<Actor>, u32) -> u32),
        entry!(
            0x008b3cd0,
            actor_set_look_at_target(Ptr<Actor>, f32, f32, f32) -> u32
        ),
        entry!(0x008b3d10, fn_008b3d10(Ptr) -> u8),
        entry!(0x008b3d30, actor_clear_look_at_target(Ptr<Actor>) -> u32),
        entry!(0x008b3d50, fn_008b3d50(Ptr<Actor>) -> u32),
        entry!(
            0x008b3d70,
            fn_008b3d70(Ptr<Actor>, f32, f32, f32, f32, f32, f32) -> u32
        ),
        entry!(0x008b3ef0, fn_008b3ef0(Ptr<Actor>, u32) -> i32),
        entry!(
            0x008b3fe0,
            fn_008b3fe0(Ptr<Actor>, u32, u32, u32, bool) -> u32
        ),
        entry!(0x008b42e0, fn_008b42e0(Ptr, Ptr)),
        entry!(0x008b4360, fn_008b4360(Ptr) -> bool),
        entry!(0x008b4380, fn_008b4380(Ptr) -> u8),
        entry!(0x008b43a0, fn_008b43a0(Ptr<Actor>, u32, u32, u32)),
        entry!(0x008b4cd0, fn_008b4cd0(Ptr) -> bool),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;
    use std::rc::Rc;

    type SharedLog = Rc<std::cell::RefCell<Vec<(u32, Vec<u32>)>>>;
    type VectorLog = Rc<std::cell::RefCell<Vec<Vec<u32>>>>;
    type ConversationRun = (
        World,
        bool,
        Vec<(u32, Vec<u32>)>,
        Ptr<Actor>,
        Ptr<Actor>,
        Ptr,
        Ptr,
    );

    fn int(value: u32) -> Ret {
        Ret {
            eax: value,
            ..Ret::default()
        }
    }

    fn float(value: f64) -> Ret {
        Ret {
            st0: value,
            ..Ret::default()
        }
    }

    /// An engine with the pages the code reads constants and the player
    /// pointer from, and the player at a fresh block.
    fn engine() -> (Engine, Ptr<Actor>) {
        let mut e = Engine::new();
        for page in [
            0x0101_2000,
            0x0103_0000,
            0x0107_1000,
            0x0108_4000,
            0x011c_e000,
            0x011d_e000,
            0x011f_2000,
        ] {
            e.map(page, 0x1000);
        }
        let player = e.new_object::<Actor>();
        e.set_global(PLAYER_CHARACTER, player.addr());
        (e, player)
    }

    /// An object of `size` bytes whose vtable has the given `(offset,
    /// function address)` slots.
    fn object_with_slots(e: &mut Engine, size: u32, slots: &[(u32, u32)]) -> Ptr {
        let vtable = e.mem.alloc(0x800);
        for (offset, target) in slots {
            e.mem.set_u32(vtable + offset, *target);
        }
        let object = e.mem.alloc(size);
        e.mem.set_u32(object, vtable);
        Ptr::new(object)
    }

    /// A double that counts its calls and returns `value`.
    fn counting_double(e: &mut Engine, address: u32, value: u32) -> Rc<Cell<u32>> {
        let count = Rc::new(Cell::new(0));
        let seen = count.clone();
        e.register_double(address, move |_, _| {
            seen.set(seen.get() + 1);
            int(value)
        });
        count
    }

    /// An actor with the actor-level virtual slots the tests use, at fresh
    /// fake function addresses.
    fn actor_with_slots(e: &mut Engine, slots: &[(u32, u32)]) -> Ptr<Actor> {
        object_with_slots(e, 0x1b4, slots).cast()
    }

    #[test]
    fn controller_flag_follows_the_actor_value() {
        for (value, expected) in [(0.0, 1u32), (3.0, 0)] {
            let (mut e, _) = engine();
            let actor = e.new_object::<Actor>();
            let controller = e.mem.alloc(0x500);
            e.mem.set_u32(actor.addr() + 0x1b0, controller);
            e.register(0x0093_06d0, |e, a| int(e.mem.u32(a[0] + 0x1b0)));
            e.register(0x0062_1270, |_, _| int(0));
            e.register_double(0x0087_f9c0, move |_, _| float(value));
            e.register(0x0062_9670, |_, _| Ret::default());
            e.call_log = Some(vec![]);
            e.call(0x008b_00c0, &args![actor]);
            let log = e.call_log.take().unwrap();
            assert_eq!(
                log.last().unwrap(),
                &(0x0062_9670, vec![controller + 0x410, 0x4000, expected])
            );
        }
    }

    #[test]
    fn controller_flag_is_skipped_without_controller_or_when_state_is_set() {
        let (mut e, _) = engine();
        let actor = e.new_object::<Actor>();
        e.register(0x0093_06d0, |_, _| int(0));
        e.call(0x008b_00c0, &args![actor]);
        let controller = e.mem.alloc(0x500);
        e.register_double(0x0093_06d0, move |_, _| int(controller));
        e.register(0x0062_1270, |_, _| int(1));
        // Any further call (value, flag setter) would panic as untranslated.
        e.call(0x008b_00c0, &args![actor]);
    }

    #[test]
    fn visibility_flags_are_set_cleared_and_tested() {
        let (mut e, _) = engine();
        let actor = e.new_object::<Actor>();
        e.call(0x008b_0140, &args![actor, 1u32, 0b0110u32]);
        assert_eq!(e.get(actor, Actor::iVisFlags), 0b0110);
        e.call(0x008b_0140, &args![actor, 0u32, 0b0010u32]);
        assert_eq!(e.get(actor, Actor::iVisFlags), 0b0100);
        assert!(e.call(0x008b_0190, &args![actor, 0b0100u32]).bool());
        assert!(!e.call(0x008b_0190, &args![actor, 0b0110u32]).bool());
        assert!(e.call(0x008b_0190, &args![actor, 0u32]).bool());
    }

    fn acquire_engine() -> (Engine, Ptr<Actor>, Rc<Cell<u32>>) {
        let (mut e, _) = engine();
        let released = Rc::new(Cell::new(0));
        let seen = released.clone();
        e.register_double(0x0f00_0001, move |_, a| {
            seen.set(seen.get() + 1 + a[1]);
            int(0)
        });
        e.register(0x0f00_0002, |_, _| int(0)); // actor slot 0x160: false
        e.register(0x0f00_0003, |_, _| int(0)); // process slot 0x5fc: false
        e.register(0x0f00_0004, |_, _| int(0)); // process slot 0x22c: nothing
        e.register(0x0f00_0005, |_, _| int(0)); // process slot 0x280
        e.register(0x0f00_0006, |_, _| int(0)); // actor slot 0x1d0: no target
        e.register(0x008d_8520, |e, a| int(e.mem.u32(a[0] + 0x68)));
        let process = object_with_slots(
            &mut e,
            0x600,
            &[
                (0x22c, 0x0f00_0004),
                (0x280, 0x0f00_0005),
                (0x5fc, 0x0f00_0003),
                (0x5f8, 0x0f00_0001),
            ],
        );
        let actor = actor_with_slots(&mut e, &[(0x160, 0x0f00_0002), (0x1d0, 0x0f00_0006)]);
        e.set(actor, Actor::pCurrentProcess, process);
        (e, actor, released)
    }

    #[test]
    fn acquire_check_without_object_or_target_is_false() {
        let (mut e, actor, released) = acquire_engine();
        assert!(!e.call(0x008b_0520, &args![actor]).bool());
        assert_eq!(released.get(), 0);
    }

    #[test]
    fn acquire_check_is_false_when_the_actor_slot_says_so() {
        let (mut e, actor, _) = acquire_engine();
        e.register(0x0f00_0002, |_, _| int(1));
        // Nothing else may run (the process getter would be reached first).
        e.register(0x008d_8520, |_, _| panic!("process must not be read"));
        assert!(!e.call(0x008b_0520, &args![actor]).bool());
    }

    #[test]
    fn acquire_check_releases_the_object_when_the_process_wants_it() {
        let (mut e, actor, released) = acquire_engine();
        e.register(0x0f00_0003, |_, _| int(1));
        assert!(!e.call(0x008b_0520, &args![actor]).bool());
        assert_eq!(released.get(), 1);
    }

    #[test]
    fn acquire_check_needs_the_object_kind_and_slot_0x280() {
        let (mut e, actor, _) = acquire_engine();
        e.register(0x0f00_0004, |_, _| int(0x1234)); // an object exists
        e.register_double(0x0041_ca90, |_, _| int(7));
        assert!(!e.call(0x008b_0520, &args![actor]).bool());
        // Kind 5 with slot 0x280 true proceeds; target is still missing.
        e.register_double(0x0041_ca90, |_, _| int(5));
        e.register(0x0f00_0005, |_, _| int(1));
        assert!(!e.call(0x008b_0520, &args![actor]).bool());
        // Kind 6 with slot 0x280 false stops.
        e.register_double(0x0041_ca90, |_, _| int(6));
        e.register(0x0f00_0005, |_, _| int(0));
        let before = e.mem.u32(actor.addr() + 0x68);
        assert_ne!(before, 0);
        assert!(!e.call(0x008b_0520, &args![actor]).bool());
    }

    #[test]
    fn acquire_check_with_a_target_depends_on_the_second_check() {
        for (second, expected) in [(0u32, true), (1, false)] {
            let (mut e, actor, _) = acquire_engine();
            let target = object_with_slots(&mut e, 0x20, &[(0x10, 0x0f00_0007)]);
            e.register_double(0x0f00_0006, move |_, _| int(target.addr()));
            e.register(0x0f00_0007, |_, _| int(1));
            e.register(0x0052_4c90, |_, _| int(0x55));
            e.set_global(0x0103_0ff0, 0.8f32);
            e.register_double(0x00b4_e030, move |_, a| {
                assert_eq!((a[0], a[1], a[2]), (target.addr(), 0x55, 0.8f32.to_bits()));
                int(second)
            });
            assert_eq!(e.call(0x008b_0520, &args![actor]).bool(), expected);
        }
    }

    /// Answers of the doubles that `GetShouldAttackActor` and
    /// `GetShouldHelp` ask.
    #[derive(Default)]
    struct Knobs {
        angry: Cell<bool>,
        in_combat_with: Cell<bool>,
        reaction: Cell<i32>,
        perceived_hostile: Cell<bool>,
        setting: Cell<f64>,
        verdict_seen: Cell<i32>,
    }

    /// Fields the doubles read from the actors (unused PC bytes of the test
    /// objects): `+0x18d` teammate, `+0x104` in combat, `+0x108`
    /// aggression, `+0x10c` assistance.
    fn relation_engine() -> (Engine, Ptr<Actor>, Ptr<Actor>, Ptr<Actor>, Rc<Knobs>) {
        let (mut e, player) = engine();
        let knobs = Rc::new(Knobs::default());
        e.register(ACTOR_IS_PLAYER_TEAMMATE, |e, a| {
            int(e.mem.u8(a[0] + 0x18d) as u32)
        });
        e.register(
            ACTOR_IS_IN_COMBAT,
            |e, a| int(e.mem.u8(a[0] + 0x104) as u32),
        );
        e.register(ACTOR_GET_ACTOR_AGGRESSION, |e, a| {
            int(e.mem.u32(a[0] + 0x108))
        });
        e.register(ACTOR_GET_ACTOR_ASSISTANCE, |e, a| {
            int(e.mem.u32(a[0] + 0x10c))
        });
        let k = knobs.clone();
        e.register_double(ACTOR_IS_ANGRY_WITH_PLAYER, move |_, _| {
            int(k.angry.get() as u32)
        });
        let k = knobs.clone();
        e.register_double(ACTOR_IS_IN_COMBAT_WITH_ACTOR, move |_, _| {
            int(k.in_combat_with.get() as u32)
        });
        let k = knobs.clone();
        e.register_double(ACTOR_GET_FACTION_FIGHT_REACTION, move |_, _| {
            int(k.reaction.get() as u32)
        });
        let k = knobs.clone();
        e.register_double(PLAYER_IS_TARGET_PERCEIVED_AND_HOSTILE, move |_, _| {
            int(k.perceived_hostile.get() as u32)
        });
        let k = knobs.clone();
        e.register_double(ACTOR_GET_FOLLOWER_SETTING, move |_, _| {
            float(k.setting.get())
        });
        let k = knobs.clone();
        e.register_double(HANDLE_ENTRY_POINT, move |e, a| {
            k.verdict_seen.set(e.mem.u8(a[3]) as i32);
            Ret::default()
        });
        e.register(0x0f00_0010, |_, _| int(0)); // target slot 0x1a0: false
        let this: Ptr<Actor> = actor_with_slots(&mut e, &[(0x1a0, 0x0f00_0010)]);
        let target: Ptr<Actor> = actor_with_slots(&mut e, &[(0x1a0, 0x0f00_0010)]);
        (e, player, this, target, knobs)
    }

    fn attack_ex(
        e: &mut Engine,
        this: Ptr<Actor>,
        target: Ptr<Actor>,
        engaged: bool,
        ignore: bool,
    ) -> bool {
        let out = e.mem.alloc(4);
        e.call(0x008b_06d0, &args![this, target, engaged, out, ignore])
            .bool()
    }

    fn attack(e: &mut Engine, this: Ptr<Actor>, target: Ptr<Actor>, engaged: bool) -> (bool, i32) {
        let out = e.mem.alloc(4);
        e.mem.set_i32(out, -77);
        let verdict = e
            .call(0x008b_06d0, &args![this, target, engaged, out, false])
            .bool();
        (verdict, e.mem.i32(out))
    }

    #[test]
    fn plain_actors_attack_by_aggression_and_reaction() {
        let (mut e, _, this, target, knobs) = relation_engine();
        e.mem.set_u32(this.addr() + 0x108, 1);
        knobs.reaction.set(1);
        assert_eq!(attack(&mut e, this, target, false), (true, 1));
        assert_eq!(knobs.verdict_seen.get(), 1);
        knobs.reaction.set(2);
        assert_eq!(attack(&mut e, this, target, false), (false, 2));
        e.mem.set_u32(this.addr() + 0x108, 2);
        knobs.reaction.set(1);
        assert_eq!(attack(&mut e, this, target, false), (true, 1));
        knobs.reaction.set(2);
        assert_eq!(attack(&mut e, this, target, false), (false, 2));
    }

    #[test]
    fn aggression_three_attacks_without_asking_the_reaction() {
        let (mut e, _, this, target, _) = relation_engine();
        e.mem.set_u32(this.addr() + 0x108, 3);
        e.register(ACTOR_GET_FACTION_FIGHT_REACTION, |_, _| panic!("not asked"));
        e.register(HANDLE_ENTRY_POINT, |_, _| panic!("not asked"));
        assert!(attack_ex(&mut e, this, target, false, false));
    }

    #[test]
    fn engaged_actors_attack_unless_the_reaction_is_friendly() {
        let (mut e, _, this, target, knobs) = relation_engine();
        e.mem.set_u32(this.addr() + 0x108, 1);
        e.mem.set_u32(target.addr() + 0x108, 1);
        knobs.reaction.set(3);
        assert_eq!(attack(&mut e, this, target, true), (false, 3));
        knobs.reaction.set(0);
        assert_eq!(attack(&mut e, this, target, true), (true, 0));
        // A target that never fights (aggression 3) is always attacked back.
        e.mem.set_u32(target.addr() + 0x108, 3);
        knobs.reaction.set(3);
        assert_eq!(attack(&mut e, this, target, true), (true, 3));
    }

    #[test]
    fn combat_with_the_target_engages_unless_ignored() {
        let (mut e, _, this, target, knobs) = relation_engine();
        e.mem.set_u32(this.addr() + 0x108, 1);
        e.mem.set_u32(target.addr() + 0x108, 1);
        knobs.in_combat_with.set(true);
        knobs.reaction.set(3);
        // Engaged by the combat: the friendly reaction does not attack.
        assert!(!attack_ex(&mut e, this, target, false, false));
        // Ignored: the aggression-1 rule with reaction 3 does not attack either,
        // but with reaction 1 it does.
        knobs.reaction.set(1);
        assert!(attack_ex(&mut e, this, target, false, true));
    }

    #[test]
    fn the_player_is_attacked_when_angry() {
        let (mut e, player, this, _, knobs) = relation_engine();
        e.mem.set_u32(this.addr() + 0x108, 1);
        knobs.reaction.set(3);
        knobs.angry.set(true);
        assert_eq!(
            attack(&mut e, this, Ptr::new(player.addr()), false),
            (true, 3)
        );
        knobs.angry.set(false);
        assert_eq!(
            attack(&mut e, this, Ptr::new(player.addr()), false),
            (false, 3)
        );
    }

    #[test]
    fn teammates_follow_the_player_and_the_follower_setting() {
        let (mut e, player, this, target, knobs) = relation_engine();
        e.mem.set_u8(this.addr() + 0x18d, 1);
        // Against the player: only anger matters.
        knobs.angry.set(true);
        assert!(attack_ex(&mut e, this, player, false, false));
        knobs.angry.set(false);
        assert!(!attack_ex(&mut e, this, player, false, false));
        // Setting 0: attack only what is in combat with the player or us.
        knobs.setting.set(0.0);
        assert!(!attack_ex(&mut e, this, target, false, false));
        e.mem.set_u8(target.addr() + 0x104, 1);
        assert!(!attack_ex(&mut e, this, target, false, false));
        knobs.in_combat_with.set(true);
        assert!(attack_ex(&mut e, this, target, false, false));
        // Setting set: aggression 1, hostile reaction needs a perceived target.
        knobs.setting.set(1.0);
        knobs.reaction.set(1);
        knobs.in_combat_with.set(false);
        assert!(!attack_ex(&mut e, this, target, false, false));
        knobs.perceived_hostile.set(true);
        assert!(attack_ex(&mut e, this, target, false, false));
    }

    #[test]
    fn a_target_that_never_fights_is_not_attacked_by_a_teammate() {
        let (mut e, _, this, target, _) = relation_engine();
        e.mem.set_u8(this.addr() + 0x18d, 1);
        e.register(0x0f00_0010, |_, _| int(1));
        assert!(!attack_ex(&mut e, this, target, false, false));
    }

    #[test]
    fn a_teammate_target_is_judged_as_the_player() {
        let (mut e, player, this, target, knobs) = relation_engine();
        e.mem.set_u8(target.addr() + 0x18d, 1);
        e.mem.set_u32(this.addr() + 0x108, 1);
        knobs.reaction.set(1);
        knobs.angry.set(true);
        // The recursion's target is the player: anger decides.
        assert!(attack_ex(&mut e, this, target, false, false));
        knobs.angry.set(false);
        knobs.reaction.set(3);
        assert!(!attack_ex(&mut e, this, target, false, false));
        let _ = player;
    }

    #[test]
    fn the_attack_check_of_008b0670() {
        let (mut e, player, this, target, knobs) = relation_engine();
        e.mem.set_u32(this.addr() + 0x108, 1);
        knobs.reaction.set(3);
        // Neither is the player and `this` is no teammate: asks, answer false.
        assert!(!e.call(0x008b_0670, &args![this, target]).bool());
        // `this` is a teammate: true without asking.
        e.mem.set_u8(this.addr() + 0x18d, 1);
        e.register(ACTOR_GET_FACTION_FIGHT_REACTION, |_, _| panic!("not asked"));
        assert!(e.call(0x008b_0670, &args![this, target]).bool());
        // `this` is the player: true without asking.
        assert!(e.call(0x008b_0670, &args![player, target]).bool());
    }

    #[test]
    fn help_follows_assistance_and_reaction() {
        let (mut e, _, this, other, knobs) = relation_engine();
        assert!(!e.call(0x008b_0970, &args![this, 0u32]).bool());
        assert!(e.call(0x008b_0970, &args![this, this]).bool());
        e.mem.set_u32(this.addr() + 0x10c, 1);
        knobs.reaction.set(2);
        assert!(e.call(0x008b_0970, &args![this, other]).bool());
        knobs.reaction.set(3);
        assert!(!e.call(0x008b_0970, &args![this, other]).bool());
        e.mem.set_u32(this.addr() + 0x10c, 2);
        assert!(e.call(0x008b_0970, &args![this, other]).bool());
        knobs.reaction.set(0);
        assert!(!e.call(0x008b_0970, &args![this, other]).bool());
        // Someone who never fights is not helped.
        e.mem.set_u32(other.addr() + 0x108, 3);
        knobs.reaction.set(3);
        assert!(!e.call(0x008b_0970, &args![this, other]).bool());
    }

    #[test]
    fn teammates_help_unless_the_other_is_in_combat_with_someone_relevant() {
        let (mut e, player, this, other, knobs) = relation_engine();
        e.mem.set_u8(this.addr() + 0x18d, 1);
        knobs.angry.set(true);
        assert!(!e.call(0x008b_0970, &args![this, player]).bool());
        knobs.angry.set(false);
        assert!(e.call(0x008b_0970, &args![this, player]).bool());
        // The other is in combat with the player or us: no help.
        e.mem.set_u8(other.addr() + 0x104, 1);
        knobs.in_combat_with.set(true);
        assert!(!e.call(0x008b_0970, &args![this, other]).bool());
        // Not in combat: asks as the player (the player's assistance is 1).
        e.mem.set_u8(other.addr() + 0x104, 0);
        e.mem.set_u32(player.addr() + 0x10c, 1);
        knobs.reaction.set(2);
        assert!(e.call(0x008b_0970, &args![this, other]).bool());
        // Helping a teammate means helping the player.
        e.mem.set_u8(this.addr() + 0x18d, 0);
        e.mem.set_u8(other.addr() + 0x18d, 1);
        e.mem.set_u32(this.addr() + 0x10c, 1);
        knobs.reaction.set(2);
        assert!(e.call(0x008b_0970, &args![this, other]).bool());
        knobs.reaction.set(3);
        assert!(!e.call(0x008b_0970, &args![this, other]).bool());
    }

    #[test]
    fn queued_reload_uses_the_flag_and_clears_it() {
        let (mut e, _) = engine();
        let actor = actor_with_slots(&mut e, &[(0x1e4, 0x0f00_0020)]);
        e.register(0x0f00_0020, |_, _| int(0xa11));
        e.register(ANIMATION_RELOAD_TARGETS, |_, _| Ret::default());
        e.set(actor, Actor::bReloadTargetQueued, 1);
        e.call_log = Some(vec![]);
        e.call(0x008b_0ac0, &args![actor]);
        let log = e.call_log.take().unwrap();
        assert_eq!(
            log.last().unwrap(),
            &(ANIMATION_RELOAD_TARGETS, vec![0xa11, 1])
        );
        assert_eq!(e.get(actor, Actor::bReloadTargetQueued), 0);
    }

    #[test]
    fn reload_targets_of_the_player_covers_both_animations() {
        let (mut e, player) = engine();
        e.register(PLAYER_GET_ANIMATION, |_, a| {
            int(if a[1] == 1 { 0xf1 } else { 0 })
        });
        let reloads = Rc::new(Cell::new(0));
        let seen = reloads.clone();
        e.register_double(ANIMATION_RELOAD_TARGETS, move |_, a| {
            assert_eq!((a[0], a[1]), (0xf1, 9));
            seen.set(seen.get() + 1);
            Ret::default()
        });
        e.call(0x008b_0b00, &args![player, 9u32]);
        assert_eq!(reloads.get(), 1);
    }

    #[test]
    fn reload_targets_of_another_actor_needs_animation_and_process() {
        let (mut e, _) = engine();
        let actor = e.new_object::<Actor>();
        e.register(ACTOR_GET_ANIMATION, |_, _| int(0xa2));
        let reloads = counting_double(&mut e, ANIMATION_RELOAD_TARGETS, 0);
        e.call(0x008b_0b00, &args![actor, 1u32]);
        assert_eq!(reloads.get(), 0);
        e.set(actor, Actor::pCurrentProcess, Ptr::new(0x1234));
        e.call(0x008b_0b00, &args![actor, 1u32]);
        assert_eq!(reloads.get(), 1);
        e.register(ACTOR_GET_ANIMATION, |_, _| int(0));
        e.call(0x008b_0b00, &args![actor, 1u32]);
        assert_eq!(reloads.get(), 1);
    }

    #[test]
    fn process_slot_0x52c_needs_a_process() {
        let (mut e, _) = engine();
        let actor = e.new_object::<Actor>();
        assert_eq!(e.call(0x008b_0ba0, &args![actor]).u32(), 0);
        e.register(0x0f00_0030, |_, _| int(0x77));
        let process = object_with_slots(&mut e, 0x600, &[(0x52c, 0x0f00_0030)]);
        e.set(actor, Actor::pCurrentProcess, process);
        assert_eq!(e.call(0x008b_0ba0, &args![actor]).u32(), 0x77);
    }

    /// Scene graph doubles: a node's children list is the node itself
    /// (`0043b480` counts, `0043b4a0` indexes) kept in test memory.
    /// A node: +4 count, +8.. children (4 bytes each), +0x30 name pointer,
    /// +0x34 has-lighting flag.
    fn node(e: &mut Engine, name: &[u8], children: &[u32], slot_c_target: u32) -> Ptr {
        let n = object_with_slots(e, 0x60, &[(0xc, slot_c_target)]);
        e.mem.set_u32(n.addr() + 4, children.len() as u32);
        for (i, c) in children.iter().enumerate() {
            e.mem.set_u32(n.addr() + 8 + 4 * i as u32, *c);
        }
        let text = e.mem.alloc(16);
        e.mem.set_cstr(text, name);
        e.mem.set_u32(n.addr() + 0x30, text);
        n
    }

    fn lighting_engine() -> (Engine, Ptr<Actor>, Rc<Cell<u32>>) {
        let (mut e, _) = engine();
        e.register(0x0043_b480, |e, a| int(e.mem.u32(a[0] + 4)));
        e.register(0x0043_b4a0, |e, a| int(e.mem.u32(a[0] + 8 + 4 * a[1])));
        e.register(0x0041_3f40, |_, a| int(a[0]));
        e.register(0x0043_b1b0, |e, a| int(e.mem.u32(a[0] + 0x30)));
        // `_strnicmp(name, "BIP", 3)`: the prefix is compared case-insensitively.
        e.register(0x00ec_7ec0, |e, a| {
            let name = e.mem.cstr(a[0]);
            let prefix = e.mem.cstr(a[1]);
            let same = name.len() >= 3 && name[..3].eq_ignore_ascii_case(&prefix[..3]);
            int(if same { 0 } else { 1 })
        });
        e.mem.set_cstr(0x0108_4e60, b"BIP");
        e.register(0x004b_2960, |e, a| int(e.mem.u32(a[0] + 0x34)));
        // The lighting property of a node is the node's address + 1.
        e.register(0x004b_5710, |_, a| int(a[0] + 1));
        // "SkinAttachment" search: a child whose slot-less name matches, via +0x38.
        e.register(0x004a_ae30, |e, a| int(e.mem.u32(a[0] + 0x38)));
        let received = Rc::new(Cell::new(u32::MAX));
        let seen = received.clone();
        e.register_double(0x0f00_0040, move |_, a| {
            seen.set(a[1]);
            Ret::default()
        });
        let process = object_with_slots(&mut e, 0x600, &[(0x4fc, 0x0f00_0040)]);
        let actor = e.new_object::<Actor>();
        e.set(actor, Actor::pCurrentProcess, process);
        (e, actor, received)
    }

    #[test]
    fn lighting_property_comes_from_the_skin_attachment_when_present() {
        let (mut e, actor, received) = lighting_engine();
        let first = node(&mut e, b"root", &[], 0);
        e.register_double(0x0f00_0041, move |_, _| int(first.addr()));
        let root = node(&mut e, b"Fo", &[], 0x0f00_0041);
        let skin = e.mem.alloc(0x60);
        e.mem.set_u32(root.addr() + 0x38, skin);
        e.call(0x008b_0bd0, &args![actor, root]);
        assert_eq!(received.get(), skin + 1);
    }

    #[test]
    fn lighting_property_skips_bip_children_and_takes_the_first_with_one() {
        let (mut e, actor, received) = lighting_engine();
        let grandchild = node(&mut e, b"g", &[0x1], 0);
        e.mem.set_u32(grandchild.addr() + 0x34, 1);
        let bip_inner = node(&mut e, b"x", &[0x1], 0);
        e.mem.set_u32(bip_inner.addr() + 0x34, 1);
        let g1 = grandchild;
        e.register_double(0x0f00_0042, move |_, _| int(g1.addr()));
        let b1 = bip_inner;
        e.register_double(0x0f00_0043, move |_, _| int(b1.addr()));
        let bip = node(&mut e, b"Bip01 Pelvis", &[], 0x0f00_0043);
        let body = node(&mut e, b"Body", &[], 0x0f00_0042);
        let first = node(&mut e, b"hierarchy", &[bip.addr(), body.addr()], 0);
        e.register_double(0x0f00_0041, move |_, _| int(first.addr()));
        let root = node(&mut e, b"root", &[], 0x0f00_0041);
        e.call(0x008b_0bd0, &args![actor, root]);
        assert_eq!(received.get(), grandchild.addr() + 1);
    }

    #[test]
    fn lighting_property_falls_back_to_the_first_child() {
        let (mut e, actor, received) = lighting_engine();
        let first = node(&mut e, b"hierarchy", &[], 0);
        e.register_double(0x0f00_0041, move |_, _| int(first.addr()));
        let root = node(&mut e, b"root", &[], 0x0f00_0041);
        e.call(0x008b_0bd0, &args![actor, root]);
        assert_eq!(received.get(), first.addr() + 1);
        // Without a process or without a node nothing is handed over.
        received.set(u32::MAX);
        e.set(actor, Actor::pCurrentProcess, Ptr::NULL);
        e.call(0x008b_0bd0, &args![actor, root]);
        assert_eq!(received.get(), u32::MAX);
    }

    #[test]
    fn magic_target_thunks_read_the_process_behind_the_interior_pointer() {
        let (mut e, _) = engine();
        let actor = e.new_object::<Actor>();
        let interior = Ptr::<()>::new(actor.addr() + 0xa4);
        assert_eq!(e.call(0x008b_0d30, &args![interior, 5u32]).f32(), 0.0);
        assert_eq!(e.call(0x008b_0d70, &args![interior, 5u32]).f32(), 0.0);
        e.register(0x0f00_0050, |_, a| float(a[1] as f64 + 0.5));
        e.register(0x0f00_0051, |_, a| float(a[1] as f64 + 0.25));
        let process =
            object_with_slots(&mut e, 0x600, &[(0x5e4, 0x0f00_0050), (0x5ec, 0x0f00_0051)]);
        e.set(actor, Actor::pCurrentProcess, process);
        assert_eq!(e.call(0x008b_0d30, &args![interior, 5u32]).f32(), 5.5);
        assert_eq!(e.call(0x008b_0d70, &args![interior, 5u32]).f32(), 5.25);
    }

    #[test]
    fn modifier_thunk_passes_the_embedded_list() {
        let (mut e, _) = engine();
        let actor = e.new_object::<Actor>();
        e.register(0x0093_7730, |_, a| float((a[0] + a[1]) as f64));
        let interior = Ptr::<()>::new(actor.addr() + 0xa4);
        let got = e.call(0x008b_0db0, &args![interior, 3u8]).f32();
        assert_eq!(got, (interior.addr() + 0x2c + 3) as f32);
    }

    /// An engine for the cached value: the process answers slot `0x148`
    /// with an item whose value (`0044ddc0` reads +8) is `0x99`, slot
    /// `0x42c` with the base 10.0, and `00646910` returns `base * 2`.
    fn cached_value_engine() -> (Engine, Ptr<Actor>, Rc<Cell<u32>>) {
        let (mut e, _) = engine();
        e.register(0x0f00_0060, |_, _| int(0x7000));
        e.register(0x0f00_0061, |_, _| float(10.0));
        let process =
            object_with_slots(&mut e, 0x600, &[(0x148, 0x0f00_0060), (0x42c, 0x0f00_0061)]);
        e.register(0x0f00_0062, |_, _| int(0xb10));
        let actor = actor_with_slots(&mut e, &[(0x1e4, 0x0f00_0062)]);
        e.set(actor, Actor::pCurrentProcess, process);
        e.register(0x0044_ddc0, |e, a| {
            int(if a[0] == 0x7000 {
                0x99
            } else {
                e.mem.u32(a[0] + 8)
            })
        });
        e.register(0x008d_8520, |e, a| int(e.mem.u32(a[0] + 0x68)));
        e.register(0x0088_46e0, |_, _| int(0x103));
        e.register(0x0088_4730, |_, _| int(1));
        e.register(0x0049_97b0, |_, _| int(1));
        e.register(0x008b_bc10, |_, _| int(0));
        let args_seen = Rc::new(Cell::new(0));
        let seen = args_seen.clone();
        e.register_double(0x0064_6910, move |_, a| {
            seen.set(a.len() as u32);
            float(f32::from_bits(a[2]) as f64 * 2.0)
        });
        // Entry point 0x22 adds 1.0 to the value it is handed.
        e.register(HANDLE_ENTRY_POINT, |e, a| {
            assert_eq!((a[0], a[2]), (0x22, 0x99));
            let v = e.mem.f32(a[3]);
            e.mem.set_f32(a[3], v + 1.0);
            Ret::default()
        });
        e.register(0x0040_3e20, |_, a| int(a[0] + 4));
        e.mem.set_f32(0x011c_e038, 0.5);
        (e, actor, args_seen)
    }

    #[test]
    fn cached_value_is_returned_when_not_negative() {
        let (mut e, actor, _) = cached_value_engine();
        e.set(actor, Actor::fGunSkillHUD, 4.0);
        e.set(actor, Actor::fGunSkillGun, 5.0);
        e.set(actor, Actor::fGunSkillActor, 6.0);
        e.set(actor, Actor::fGunSkillVATS, 7.0);
        for (selector, expected) in [(0, 4.0), (1, 5.0), (2, 6.0), (3, 7.0)] {
            assert_eq!(
                e.call(0x008b_0dd0, &args![actor, selector as u32]).f32(),
                expected
            );
        }
    }

    #[test]
    fn negative_selector_one_cache_is_reset_to_zero() {
        let (mut e, actor, _) = cached_value_engine();
        e.set(actor, Actor::fGunSkillGun, -1.0);
        assert_eq!(e.call(0x008b_0dd0, &args![actor, 1u32]).f32(), 0.0);
        assert_eq!(e.get(actor, Actor::fGunSkillGun), 0.0);
    }

    #[test]
    fn missing_process_or_item_gives_zero() {
        let (mut e, actor, _) = cached_value_engine();
        e.register(0x0f00_0060, |_, _| int(0));
        assert_eq!(e.call(0x008b_0dd0, &args![actor, 0u32]).f32(), 0.0);
        e.set(actor, Actor::pCurrentProcess, Ptr::NULL);
        assert_eq!(e.call(0x008b_0dd0, &args![actor, 0u32]).f32(), 0.0);
    }

    #[test]
    fn uncached_values_go_through_the_entry_point_and_are_cached() {
        let (mut e, actor, words) = cached_value_engine();
        e.set(actor, Actor::fGunSkillHUD, -1.0);
        e.set(actor, Actor::fGunSkillActor, -1.0);
        // (10 * 2) + 1
        assert_eq!(e.call(0x008b_0dd0, &args![actor, 0u32]).f32(), 21.0);
        assert_eq!(e.get(actor, Actor::fGunSkillHUD), 21.0);
        assert_eq!(words.get(), 7);
        // Setting 2 or 3 uses the other argument list but gives the same value.
        e.mem.set_u32(0x011f_2250 + 8, 2);
        assert_eq!(e.call(0x008b_0dd0, &args![actor, 2u32]).f32(), 21.0);
        assert_eq!(e.get(actor, Actor::fGunSkillActor), 21.0);
    }

    #[test]
    fn selector_three_scales_and_adds_the_base_again() {
        let (mut e, actor, _) = cached_value_engine();
        e.set(actor, Actor::fGunSkillVATS, -1.0);
        // ((10 * 2) + 1) * 0.5 + 10
        assert_eq!(e.call(0x008b_0dd0, &args![actor, 3u32]).f32(), 20.5);
        assert_eq!(e.get(actor, Actor::fGunSkillVATS), 20.5);
    }

    /// A dying actor (slots `0x1e4` animation, `0x21c`, `0x1f4` position,
    /// `0x434`) with a killer, doubles for everything `DoDeathStuff` calls,
    /// and the call log started.
    fn death_engine() -> (Engine, Ptr<Actor>, Ptr<Actor>) {
        let (mut e, _) = engine();
        for fake in [0x0f00_0070, 0x0f00_0072, 0x0f00_0073] {
            e.register(fake, |_, _| int(0));
        }
        e.register(0x0f00_0071, |_, _| int(1));
        let position = e.mem.alloc(12);
        for (i, v) in [1.0f32, 2.0, 3.0].iter().enumerate() {
            e.mem.set_f32(position + 4 * i as u32, *v);
        }
        e.register_double(0x0f00_0074, move |_, _| int(position));
        let actor = actor_with_slots(
            &mut e,
            &[
                (0x1e4, 0x0f00_0070),
                (0x21c, 0x0f00_0072),
                (0x1f4, 0x0f00_0074),
                (0x434, 0x0f00_0073),
            ],
        );
        let killer = actor_with_slots(&mut e, &[(0x218, 0x0f00_0071)]);
        e.set(actor, Actor::pMyKiller, killer.cast());
        let setting = e.mem.alloc(8);
        e.register_double(0x0040_8d60, move |_, _| int(setting + 4));
        e.register(0x0087_f3d0, |_, _| int(0));
        e.register(ACTOR_SET_LIFE_STATE, |_, _| Ret::default());
        e.register(0x0056_7790, |_, _| int(0));
        e.register(0x008c_09e0, |_, _| Ret::default());
        e.register(0x005d_43c0, |_, a| int(a[0] + 0x44));
        e.register(0x005a_c750, |_, _| int(1));
        e.register(ACTOR_GET_PROCESS, |e, a| int(e.mem.u32(a[0] + 0x68)));
        e.register(
            ACTOR_IS_IN_COMBAT,
            |e, a| int(e.mem.u8(a[0] + 0x104) as u32),
        );
        e.register(0x0088_1680, |_, _| Ret::default());
        e.register(0x008b_ca90, |_, _| Ret::default());
        (e, actor, killer.cast())
    }

    fn calls(e: &mut Engine) -> Vec<(u32, Vec<u32>)> {
        e.call_log.take().unwrap_or_default()
    }

    #[test]
    fn essential_actors_do_not_die() {
        let (mut e, actor, _) = death_engine();
        let setting = e.call(0x0040_8d60, &args![0u32]).u32();
        e.mem.set_u8(setting, 1);
        e.register(0x0087_f3d0, |_, _| int(1));
        e.call_log = Some(vec![]);
        e.call(0x008b_01c0, &args![actor]);
        let log = calls(&mut e);
        assert!(log
            .iter()
            .all(|(address, _)| *address != ACTOR_SET_LIFE_STATE));
        assert_eq!(e.get(actor, Actor::bSetOnDeath), 0);
    }

    #[test]
    fn a_death_animation_without_the_flag_waits() {
        let (mut e, actor, _) = death_engine();
        e.register(0x0f00_0070, |_, _| int(0xa11));
        e.register(0x0049_1040, |_, _| int(1));
        e.register(0x0044_0da0, |_, _| int(0));
        e.call(0x008b_01c0, &args![actor]);
        assert_eq!(e.get(actor, Actor::bSetOnDeath), 0);
        // With the flag set on the model the death goes ahead.
        e.register(0x0044_0da0, |_, _| int(1));
        e.call(0x008b_01c0, &args![actor]);
        assert_eq!(e.get(actor, Actor::bSetOnDeath), 1);
    }

    #[test]
    fn a_second_death_only_sets_the_life_state() {
        let (mut e, actor, _) = death_engine();
        e.set(actor, Actor::bSetOnDeath, 1);
        e.call_log = Some(vec![]);
        e.call(0x008b_01c0, &args![actor]);
        let log = calls(&mut e);
        assert_eq!(
            log,
            vec![
                (0x008b_01c0, vec![actor.addr()]),
                (0x0f00_0070, vec![actor.addr()]),
                (0x0040_8d60, vec![0x011e_0888]),
                (ACTOR_SET_LIFE_STATE, vec![actor.addr(), 2]),
            ]
        );
    }

    #[test]
    fn a_murder_without_witness_reports_through_the_actor() {
        let (mut e, actor, killer) = death_engine();
        e.set(actor, Actor::bMurderAlarm, 1);
        e.call_log = Some(vec![]);
        e.call(0x008b_01c0, &args![actor]);
        let log = calls(&mut e);
        let killer = killer.addr();
        let a = actor.addr();
        assert!(log.contains(&(0x008c_09e0, vec![a, killer])));
        assert!(!log.contains(&(0x005a_c750, vec![killer, a + 0x44, 0x20])));
        assert!(log.contains(&(0x005a_c750, vec![killer, a + 0x44, 0x10])));
        assert_eq!(e.get(actor, Actor::bMurderAlarm), 1);
        assert_eq!(log[log.len() - 3], (ACTOR_SET_LIFE_STATE, vec![a, 2]));
        assert_eq!(log[log.len() - 1], (0x008b_ca90, vec![a, 0]));
    }

    #[test]
    fn a_murder_of_an_owned_actor_raises_the_alarm_for_the_owner() {
        let (mut e, actor, killer) = death_engine();
        e.set(actor, Actor::bMurderAlarm, 1);
        let owner = e.mem.alloc(0x20);
        e.register_double(0x0056_7790, move |_, _| int(owner));
        // Form type 0x2a: the owner is an actor reference in the high process.
        e.register(0x0040_1170, |_, _| int(0x2a));
        e.register(0x0097_0a20, |_, a| int(a[1] + 0x100));
        e.register(0x008c_0460, |_, _| Ret::default());
        e.call_log = Some(vec![]);
        e.call(0x008b_01c0, &args![actor]);
        let log = calls(&mut e);
        let (killer, a) = (killer.addr(), actor.addr());
        assert!(log.contains(&(0x0097_0a20, vec![PROCESS_LISTS, owner, 0])));
        assert!(log.contains(&(0x005a_c750, vec![killer, a + 0x44, 0x20])));
        assert!(log.contains(&(0x008c_0460, vec![owner + 0x100, killer, 0, 1])));
        assert!(!log.iter().any(|(address, _)| *address == 0x008c_09e0));
        assert_eq!(e.get(actor, Actor::bMurderAlarm), 0);
    }

    #[test]
    fn a_murder_by_an_owner_of_another_form_type_uses_the_other_lookup() {
        let (mut e, actor, _) = death_engine();
        e.set(actor, Actor::bMurderAlarm, 1);
        let owner = e.mem.alloc(0x20);
        e.register_double(0x0056_7790, move |_, _| int(owner));
        e.register(0x0040_1170, |_, _| int(8));
        e.register(0x0097_0b30, |_, _| int(0));
        e.call_log = Some(vec![]);
        e.call(0x008b_01c0, &args![actor]);
        let log = calls(&mut e);
        assert!(log.contains(&(0x0097_0b30, vec![PROCESS_LISTS, owner, 0, 0])));
        // No witness, so no attack alarm (it would be an untranslated call).
        assert_eq!(e.get(actor, Actor::bMurderAlarm), 0);
    }

    #[test]
    fn the_players_kill_reaches_the_players_process_and_the_dying_process_stops() {
        let (mut e, actor, _) = death_engine();
        let player = e.global::<u32>(PLAYER_CHARACTER);
        e.set(actor, Actor::pMyKiller, Ptr::<()>::new(player));
        e.mem.set_u8(actor.addr() + 0x104, 1);
        let received = Rc::new(Cell::new((0, 0, 0, 0)));
        let seen = received.clone();
        e.register_double(0x0f00_0080, move |_, a| {
            seen.set((a[1], a[2], a[3], a[4]));
            Ret::default()
        });
        let player_process = object_with_slots(&mut e, 0x600, &[(0xfc, 0x0f00_0080)]);
        e.mem.set_u32(player + 0x68, player_process.addr());
        e.register(0x0f00_0081, |_, _| Ret::default());
        let limit_seen = Rc::new(Cell::new(0));
        let limit = limit_seen.clone();
        e.register_double(0x0f00_0082, move |_, a| {
            limit.set(a[1]);
            Ret::default()
        });
        e.map(0x0101_6000, 0x1000);
        e.set_global(0x0101_6970, 3.0e38f32);
        let process = object_with_slots(
            &mut e,
            0x600,
            &[
                (0x4b4, 0x0f00_0082),
                (0x4c0, 0x0f00_0081),
                (0x214, 0x0f00_0081),
                (0x234, 0x0f00_0081),
            ],
        );
        e.set(actor, Actor::pCurrentProcess, process);
        e.register(0x0f00_0071, |_, _| int(1));
        e.call_log = Some(vec![]);
        e.call(0x008b_01c0, &args![actor]);
        let log = calls(&mut e);
        assert_eq!(
            received.get(),
            (player, 1.0f32.to_bits(), 2.0f32.to_bits(), 3.0f32.to_bits())
        );
        // The player's kill is not an alarm and the process is told to stop:
        // end of interrupt package, then slots 0x4b4/0x4c0/0x214/0x234 ran.
        assert!(log.contains(&(0x0088_1680, vec![actor.addr(), 0])));
        assert_eq!(limit_seen.get(), 3.0e38f32.to_bits());
    }

    type SlotLog = Rc<std::cell::RefCell<Vec<(u32, u32, Vec<u32>)>>>;
    type Answers = Rc<std::cell::RefCell<std::collections::HashMap<(u32, u32), u32>>>;

    /// Every virtual slot used by `fn_008b1070` on actors and processes
    /// points at a recording double: `log` gets `(object, slot, arguments)`
    /// and the result is `answers[(object, slot)]` (default 0).
    struct World {
        e: Engine,
        log: SlotLog,
        answers: Answers,
    }

    const DIALOGUE_SLOTS: [u32; 42] = [
        0x24c, 0x20c, 0x22c, 0x650, 0x624, 0x374, 0x2ec, 0x214, 0x714, 0x71c, 0x614, 0x674, 0x14c,
        0x148, 0x3ec, 0x100, 0x274, 0x1e4, 0x128, 0x8c, 0x644, 0x634, 0x28, 0x710, 0x280, 0x238,
        0x90, 0xfc, 0x288, 0x2f4, 0x48, 0x12c, 0x4bc, 0x424, 0x428, 0x454, 0x14, 0x21c, 0x1d0,
        0x1f4, 0x8, 0x254,
    ];

    fn world() -> World {
        let (mut e, _) = engine();
        let log: SlotLog = Default::default();
        let answers: Answers = Default::default();
        for slot in DIALOGUE_SLOTS {
            let (log, answers) = (log.clone(), answers.clone());
            e.register_double(0x0f20_0000 + slot, move |_, a| {
                log.borrow_mut().push((a[0], slot, a[1..].to_vec()));
                int(answers.borrow().get(&(a[0], slot)).copied().unwrap_or(0))
            });
        }
        e.register(ACTOR_GET_PROCESS, |e, a| int(e.mem.u32(a[0] + 0x68)));
        e.register(0x0047_c850, |_, _| int(0));
        e.register(0x005d_43c0, |_, a| int(a[0] + 0x44));
        e.register(0x0042_e130, |_, _| Ret::default());
        e.register(0x0067_27b0, |_, _| int(0));
        e.register(0x00ad_8ce0, |_, _| int(0));
        e.register(0x0084_e3a0, |e, a| int(e.mem.u32(a[0] + 4)));
        e.register(0x0040_36b0, |_, _| int(0));
        e.register(0x0093_44a0, |e, a| int(e.mem.u32(a[0] + 0x1f0)));
        e.register(0x0093_36c0, |_, _| int(1));
        e.map(0x011f_1000, 0x1000);
        e.map(0x011d_e000, 0x1000);
        World { e, log, answers }
    }

    impl World {
        fn slots() -> Vec<(u32, u32)> {
            DIALOGUE_SLOTS
                .iter()
                .map(|s| (*s, 0x0f20_0000 + s))
                .collect()
        }

        /// An actor with a process; both have every recorded slot.
        fn actor(&mut self) -> (Ptr<Actor>, Ptr) {
            let actor: Ptr<Actor> = object_with_slots(&mut self.e, 0x1b4, &Self::slots()).cast();
            let process = object_with_slots(&mut self.e, 0x800, &Self::slots());
            self.e.set(actor, Actor::pCurrentProcess, process);
            (actor, process)
        }

        fn answer(&self, object: u32, slot: u32, value: u32) {
            self.answers.borrow_mut().insert((object, slot), value);
        }

        fn calls_on(&self, object: u32) -> Vec<(u32, Vec<u32>)> {
            self.log
                .borrow()
                .iter()
                .filter(|(o, _, _)| *o == object)
                .map(|(_, s, a)| (*s, a.clone()))
                .collect()
        }

        /// A package with a kind-0x1c marker (`41ca90`), starter and target.
        fn dialogue(&mut self, starter: Ptr<Actor>, target: Ptr<Actor>) -> u32 {
            let package = self.e.mem.alloc(0x100);
            let (s, t) = (starter.addr(), target.addr());
            self.e.register_double(0x0041_ca90, move |_, a| {
                int(if a[0] == package { 0x1c } else { 0 })
            });
            self.e.register_double(0x009e_e040, move |_, _| int(s));
            self.e.register_double(0x008d_80e0, move |_, _| int(t));
            package
        }
    }

    #[test]
    fn package_change_needs_a_process_and_no_blocking_setting() {
        let mut w = world();
        let actor = w.e.new_object::<Actor>();
        w.e.call(0x008b_1070, &args![actor]);
        let (actor, process) = w.actor();
        w.e.register(0x0047_c850, |_, _| int(1));
        w.e.call(0x008b_1070, &args![actor]);
        assert!(w.log.borrow().is_empty());
        let _ = process;
    }

    #[test]
    fn without_a_dialogue_package_the_process_is_reset() {
        let mut w = world();
        let (actor, process) = w.actor();
        w.answer(process.addr(), 0x674, 0x44);
        w.e.register(0x0041_ca90, |_, _| int(0));
        w.e.call(0x008b_1070, &args![actor]);
        assert_eq!(
            w.calls_on(process.addr()),
            vec![
                (0x24c, vec![0]),
                (0x20c, vec![]),
                (0x22c, vec![]),
                (0x674, vec![4]),
                (0x650, vec![1]),
                (0x624, vec![0x44]),
                (0x71c, vec![0]),
                (0x14c, vec![]),
                (0x148, vec![]),
            ]
        );
        assert_eq!(w.e.mem.f32(actor.addr() + 0x74), 1.0);
    }

    #[test]
    fn the_current_dialogue_package_points_both_speakers_at_each_other() {
        let mut w = world();
        let (starter, starter_process) = w.actor();
        let (target, target_process) = w.actor();
        let package = w.dialogue(starter, target);
        w.answer(starter_process.addr(), 0x20c, package);
        w.answer(target_process.addr(), 0x20c, package);
        w.e.mem.set_u32(target.addr() + 0x1f0, package);
        w.e.call(0x008b_1070, &args![starter]);
        let (s, t) = (starter.addr(), target.addr());
        assert_eq!(
            w.calls_on(starter_process.addr()),
            vec![
                (0x24c, vec![0]),
                (0x20c, vec![]),
                (0x650, vec![1]),
                (0x624, vec![t]),
                (0x374, vec![0]),
                (0x2ec, vec![0]),
                (0x214, vec![]),
                (0x71c, vec![0]),
                (0x14c, vec![]),
                (0x148, vec![]),
            ]
        );
        assert_eq!(
            w.calls_on(target_process.addr()),
            vec![
                (0x20c, vec![]),
                (0x650, vec![1]),
                (0x624, vec![s]),
                (0x374, vec![0]),
                (0x2ec, vec![0]),
                (0x214, vec![]),
            ]
        );
        // Both speakers are asked (slots 0x274, 0x100) and the lip hold is not set.
        assert_eq!(w.e.mem.f32(s + 0x74), 1.0);
        assert_eq!(w.e.mem.f32(t + 0x74), 1.0);
        assert_eq!(w.e.mem.u8(s + 0x80), 0);
    }

    #[test]
    fn the_next_dialogue_package_uses_the_restart_slot_and_cancels_lip_files() {
        let mut w = world();
        let (starter, starter_process) = w.actor();
        let (target, target_process) = w.actor();
        let package = w.dialogue(starter, target);
        w.answer(starter_process.addr(), 0x22c, package);
        // `IsinDialogue` is true (set up) and the target's own package is not a dialogue.
        w.answer(starter.addr(), 0x100, 1);
        let lip = Rc::new(Cell::new(0));
        let seen = lip.clone();
        w.e.register_double(0x0090_6050, move |_, a| {
            assert_eq!(a[0], LIP_SYNC_MANAGER);
            seen.set(seen.get() + 1);
            Ret::default()
        });
        w.e.call(0x008b_1070, &args![starter]);
        let (s, t) = (starter.addr(), target.addr());
        assert_eq!(lip.get(), 1);
        assert_eq!(w.e.mem.u8(s + 0x80), 1);
        assert_eq!(w.e.mem.u8(t + 0x80), 0);
        let on_starter = w.calls_on(starter_process.addr());
        assert!(on_starter.contains(&(0x714, vec![s])));
        assert!(!on_starter.contains(&(0x214, vec![])));
        let on_target = w.calls_on(target_process.addr());
        // The target's package (answer 0) is not a dialogue: slot 0x714 with itself.
        assert!(on_target.contains(&(0x714, vec![t])));
    }

    #[test]
    fn a_target_that_is_already_in_the_package_ends_the_change_early() {
        let mut w = world();
        let (starter, starter_process) = w.actor();
        let (target, target_process) = w.actor();
        let package = w.dialogue(starter, target);
        w.answer(starter_process.addr(), 0x20c, package);
        w.answer(target_process.addr(), 0x20c, package);
        // The package waits on a type-0xef condition.
        let waiting = w.e.mem.alloc(0x20);
        w.e.mem.set_u32(package + 0x8c, waiting);
        w.e.mem.set_u32(waiting + 4, 0xef);
        w.e.call(0x008b_1070, &args![starter]);
        // Only the early slot calls ran, then the starter's process slot 0x214.
        assert_eq!(
            w.calls_on(starter_process.addr()),
            vec![(0x24c, vec![0]), (0x20c, vec![]), (0x214, vec![])]
        );
        assert!(w
            .calls_on(target_process.addr())
            .iter()
            .all(|(slot, _)| *slot == 0x20c));
    }

    #[test]
    fn a_target_busy_elsewhere_is_left_alone() {
        let mut w = world();
        let (starter, starter_process) = w.actor();
        let (target, target_process) = w.actor();
        let package = w.dialogue(starter, target);
        w.answer(starter_process.addr(), 0x20c, package);
        // The target's process runs another package (slot 0x20c answers 0).
        w.e.call(0x008b_1070, &args![starter]);
        assert!(w
            .calls_on(target_process.addr())
            .iter()
            .all(|(slot, _)| *slot == 0x20c));
    }

    #[test]
    fn the_players_conversation_field_is_cleared() {
        let mut w = world();
        let (starter, starter_process) = w.actor();
        let (target, _) = w.actor();
        let package = w.dialogue(starter, target);
        w.answer(starter_process.addr(), 0x20c, package);
        let player = w.e.global::<u32>(PLAYER_CHARACTER);
        w.e.mem.map(player, 0x300);
        w.e.mem.set_u32(player + 0x208, package);
        w.e.call(0x008b_1070, &args![starter]);
        assert_eq!(w.e.mem.u32(player + 0x208), 0);
    }

    #[test]
    fn a_weapon_without_regeneration_updates_the_actor() {
        let mut w = world();
        let (actor, process) = w.actor();
        w.e.register(0x0041_ca90, |_, _| int(0));
        w.answer(process.addr(), 0x148, 0x5000);
        w.e.register(0x004b_da70, |_, a| int(if a[1] == 6 { 1 } else { 0 }));
        w.e.register(0x0044_ddc0, |_, _| int(0x77));
        let rate = Rc::new(Cell::new(0.0));
        let shown = rate.clone();
        w.e.register_double(0x0070_9430, move |_, a| {
            assert_eq!(a, [0x77, 1]);
            float(shown.get())
        });
        w.e.register(0x008a_16d0, |_, _| int(1));
        w.e.call(0x008b_1070, &args![actor]);
        let on_actor = w.calls_on(actor.addr());
        assert_eq!(on_actor, vec![(0x3ec, vec![0x77, 1, 0, 0])]);
        // A positive regeneration rate leaves the actor alone.
        w.log.borrow_mut().clear();
        rate.set(0.5);
        w.e.call(0x008b_1070, &args![actor]);
        assert!(w.calls_on(actor.addr()).is_empty());
        // So does an item in the second slot.
        rate.set(-1.0);
        w.answer(process.addr(), 0x14c, 0x6000);
        w.e.call(0x008b_1070, &args![actor]);
        assert!(w.calls_on(actor.addr()).is_empty());
    }

    #[test]
    fn the_talker_is_recorded_and_handed_to_the_process() {
        let mut w = world();
        let (actor, process) = w.actor();
        let (talker, _) = w.actor();
        let form = w.e.mem.alloc(0x100);
        w.e.register_double(0x007a_f430, move |_, _| int(form));
        w.e.register(0x005d_43c0, |_, a| int(a[0] + 0x44));
        w.e.register(0x0042_e060, |_, _| Ret::default());
        let talking = w.e.mem.alloc(0x20);
        w.e.mem.set_u32(talking + 0xc, 0x4321);
        w.e.register_double(0x0042_e110, move |_, _| int(talking));
        w.e.register(0x0051_6bf0, |e, a| int(e.mem.u32(a[0] + 0x90)));
        let out = w.e.mem.alloc(4);
        w.e.call_log = Some(vec![]);
        let result = w.e.call(0x008b_1910, &args![actor, talker, out]).u32();
        let log = w.e.call_log.take().unwrap();
        assert_eq!(result, 0x4321);
        assert_eq!(w.e.mem.u32(out), 0x4321);
        assert!(log.contains(&(0x0042_e060, vec![talker.addr() + 0x44, form, talker.addr()])));
        assert_eq!(w.calls_on(talker.addr()), vec![(0x48, vec![0x8000_0000])]);
        assert_eq!(w.calls_on(process.addr()), vec![(0x12c, vec![0x4321])]);
        // The form's 0x90 field replaces it.
        w.e.mem.set_u32(form + 0x90, 0x8888);
        assert_eq!(
            w.e.call(0x008b_1910, &args![actor, talker, out]).u32(),
            0x8888
        );
        assert_eq!(w.e.mem.u32(out), 0x4321);
        // Without talking-actor extra the talker itself is handed over.
        w.e.register(0x0042_e110, |_, _| int(0));
        assert_eq!(
            w.e.call(0x008b_1910, &args![actor, talker, out]).u32(),
            talker.addr()
        );
    }

    #[test]
    fn bit_one_of_the_word_at_0x1c() {
        let (mut e, _) = engine();
        let object = e.mem.alloc(0x40);
        assert!(!e.call(0x008b_1ff0, &args![Ptr::<()>::new(object)]).bool());
        e.mem.set_u32(object + 0x1c, 2);
        assert!(e.call(0x008b_1ff0, &args![Ptr::<()>::new(object)]).bool());
        e.mem.set_u32(object + 0x1c, 0xfd);
        assert!(!e.call(0x008b_1ff0, &args![Ptr::<()>::new(object)]).bool());
    }

    /// Idle manager doubles: arrays are `[count at +4, forms from +8]`,
    /// the root array answers for `600560`, the loose one for `600630`.
    fn idle_engine(root: &[u32], loose: &[u32]) -> Engine {
        let (mut e, _) = engine();
        e.map(0x011c_b000, 0x1000);
        e.set_global(0x011c_b6a0, 0x7777u32);
        let mut array = |forms: &[u32]| {
            if forms.is_empty() {
                return 0;
            }
            let a = e.mem.alloc(8 + 4 * forms.len() as u32);
            e.mem.set_u32(a + 4, forms.len() as u32);
            for (i, f) in forms.iter().enumerate() {
                e.mem.set_u32(a + 8 + 4 * i as u32, *f);
            }
            a
        };
        let (root, loose) = (array(root), array(loose));
        e.register(0x0057_15d0, |_, _| int(0xaa));
        e.register(0x0060_07f0, |e, a| {
            assert_eq!(a[0], 0xaa);
            e.mem.set_cstr(a[1], b"meshes\\idles");
            Ret::default()
        });
        e.register_double(0x0060_0560, move |e, a| {
            assert_eq!(a[0], 0x7777);
            assert_eq!(e.mem.cstr(a[1]), b"meshes\\idles");
            int(root)
        });
        e.register_double(0x0060_0630, move |_, a| {
            assert_eq!(a[0], 0x7777);
            int(loose)
        });
        e.register(0x0084_e3a0, |e, a| int(e.mem.u32(a[0] + 4)));
        e.register(0x004d_6170, |e, a| int(e.mem.u32(a[0] + 8 + 4 * a[1])));
        // Idle 0x500 links to 0x600 (a location chain); nothing else links.
        e.register(0x0080_41a0, |_, a| {
            int(if a[0] == 0x500 { 0x600 } else { 0 })
        });
        e
    }

    #[test]
    fn idles_are_usable_when_listed_in_either_array() {
        let mut e = idle_engine(&[0x10, 0x20], &[0x30]);
        let actor = e.new_object::<Actor>();
        let can = |e: &mut Engine, idle: u32| e.call(0x008b_2010, &args![actor, idle]).bool();
        assert!(!can(&mut e, 0));
        assert!(can(&mut e, 0x20));
        assert!(can(&mut e, 0x30));
        assert!(!can(&mut e, 0x40));
    }

    #[test]
    fn idles_follow_their_location_chain_and_survive_empty_arrays() {
        let mut e = idle_engine(&[], &[0x600]);
        let actor = e.new_object::<Actor>();
        assert!(e.call(0x008b_2010, &args![actor, 0x500u32]).bool());
        let mut e = idle_engine(&[], &[]);
        let actor = e.new_object::<Actor>();
        assert!(!e.call(0x008b_2010, &args![actor, 0x500u32]).bool());
    }

    /// A conversation between `this` (with a process answering slot 0x128
    /// with the partner) and a mobile partner, with doubles for every
    /// package setter.
    struct Conversation {
        w: World,
        this: Ptr<Actor>,
        this_process: Ptr,
        partner: Ptr<Actor>,
        partner_process: Ptr,
        source: u32,
    }

    fn conversation() -> Conversation {
        let mut w = world();
        let (this, this_process) = w.actor();
        let (partner, partner_process) = w.actor();
        w.answer(this_process.addr(), 0x128, partner.addr());
        w.answer(partner.addr(), 0xfc, 1);
        w.answer(this_process.addr(), 0x90, 0x9090);
        for address in [
            0x0067_a1b0u32,
            0x0067_0fc0,
            0x0082_6b40,
            0x0082_6b90,
            0x0098_4f60,
            0x0067_1d30,
            0x0068_0110,
            0x0068_00b0,
            0x0040_3550,
            0x0067_2fc0,
            0x007b_3fa0,
            0x0067_2aa0,
            0x0067_2c40,
            0x0067_29d0,
            0x0067_2b70,
            0x005c_8a30,
            0x0048_3710,
            0x0057_bd60,
            0x0067_1e10,
        ] {
            w.e.register(address, |_, _| Ret::default());
        }
        for address in [
            0x0067_a690u32,
            0x0067_2800,
            0x0067_27b0,
            0x0067_2dd0,
            0x005f_36f0,
            0x0067_efd0,
            0x0055_b980,
            0x0067_1d10,
            0x0044_ddc0,
        ] {
            w.e.register(address, |_, _| int(0));
        }
        w.e.register(OPERATOR_NEW, |e, a| int(e.mem.alloc(a[0])));
        w.e.register(DIALOGUE_PACKAGE_CONSTRUCTOR, |_, a| int(a[0]));
        w.e.register(0x0067_ff70, |_, a| int(a[0]));
        w.e.register(0x0067_2760, |_, _| int(0x70));
        w.e.register(0x0067_2850, |_, _| float(2.5));
        w.e.register(0x0093_44a0, |_, _| int(0x1234));
        let source = w.e.mem.alloc(0x100);
        w.e.mem.set_u32(source + 0x1c, 2);
        Conversation {
            w,
            this,
            this_process,
            partner,
            partner_process,
            source,
        }
    }

    #[test]
    fn a_mobile_partner_gets_the_package_and_the_conversation_starts() {
        let mut c = conversation();
        c.w.e.call_log = Some(vec![]);
        let started = c.w.e.call(0x008b_19c0, &args![c.this, c.source]).bool();
        let log = c.w.e.call_log.take().unwrap();
        assert!(started);
        let (this, partner, source) = (c.this.addr(), c.partner.addr(), c.source);
        // The package is built from the source package.
        let build = log
            .iter()
            .find(|(address, _)| *address == DIALOGUE_PACKAGE_CONSTRUCTOR)
            .unwrap();
        let package = build.1[0];
        assert_eq!(build.1[1..], [0, this, partner, 0, 0, 1]);
        assert!(log.contains(&(0x0067_a1b0, vec![package, source])));
        assert!(log.contains(&(0x0067_0fc0, vec![package, 0x1c])));
        // Bit 1 of the source's word at +0x1C, then the other flag getter.
        assert!(log.contains(&(0x0082_6b40, vec![package, 1])));
        assert!(log.contains(&(0x0082_6b90, vec![package, 0])));
        assert!(log.contains(&(0x0098_4f60, vec![package, 10])));
        assert!(log.contains(&(0x0067_2c40, vec![package, 2.5f32.to_bits()])));
        assert!(log.contains(&(0x0067_29d0, vec![package, 0x70])));
        assert_eq!(c.w.e.mem.u32(package + 0x8c), 0x70);
        assert_eq!(c.w.e.mem.u32(package + 0x9c), 0x9090);
        // The package target refers to the partner (allocation is 0x10 bytes).
        let target_set = log
            .iter()
            .find(|(address, _)| *address == 0x0068_0110)
            .unwrap();
        assert_eq!(target_set.1[1], partner);
        assert!(log.contains(&(0x0068_00b0, vec![target_set.1[0], 0])));
        assert!(log.contains(&(0x0067_2fc0, vec![package, target_set.1[0]])));
        assert!(log.contains(&(0x007b_3fa0, vec![target_set.1[0], 1])));
        assert!(!log.iter().any(|(address, _)| *address == 0x0067_1e10));
        // The partner is told to talk back (slots 0x710, 0x2f4, 0x12c, 0x288).
        assert!(log.contains(&(0x0057_bd60, vec![this, partner])));
        assert!(log.contains(&(0x0057_bd60, vec![partner, this])));
        let on_partner = c.w.calls_on(c.partner_process.addr());
        assert_eq!(
            on_partner,
            vec![
                (0x710, vec![partner]),
                (0x12c, vec![partner]),
                (0x288, vec![partner, 1])
            ]
        );
        let on_this = c.w.calls_on(c.this_process.addr());
        assert!(on_this.contains(&(0x710, vec![this])));
        assert!(on_this.contains(&(0x12c, vec![partner])));
        assert!(on_this.contains(&(0x238, vec![1])));
        assert!(on_this.contains(&(0x644, vec![1])));
        assert!(on_this.contains(&(0x634, vec![partner])));
        assert!(c.w.calls_on(partner).contains(&(0x288, vec![])));
        assert!(c.w.calls_on(this).contains(&(0x2f4, vec![package, 0, 1])));
    }

    #[test]
    fn a_source_with_headtracking_off_skips_the_head_calls() {
        let mut c = conversation();
        c.w.e.register(0x0067_2800, |_, _| int(1));
        c.w.e.call(0x008b_19c0, &args![c.this, c.source]);
        let on_this = c.w.calls_on(c.this_process.addr());
        assert!(!on_this
            .iter()
            .any(|(slot, _)| *slot == 0x644 || *slot == 0x634));
        // The process already answering 1 to slot 0x280 skips slot 0x238.
        c.w.answer(c.this_process.addr(), 0x280, 1);
        c.w.log.borrow_mut().clear();
        c.w.e.call(0x008b_19c0, &args![c.this, c.source]);
        assert!(!c
            .w
            .calls_on(c.this_process.addr())
            .iter()
            .any(|(slot, _)| *slot == 0x238));
    }

    #[test]
    fn a_second_location_and_a_busy_target_are_forwarded() {
        let mut c = conversation();
        c.w.e.register(0x0067_2dd0, |_, _| int(0x2222));
        c.w.e.call_log = Some(vec![]);
        c.w.e.call(0x008b_19c0, &args![c.this, c.source]);
        let log = c.w.e.call_log.take().unwrap();
        let package = log
            .iter()
            .find(|(address, _)| *address == DIALOGUE_PACKAGE_CONSTRUCTOR)
            .unwrap()
            .1[0];
        assert!(log.contains(&(0x0067_1e10, vec![package, 0x2222])));
        // A second location suppresses the partner's own package.
        assert!(c.w.calls_on(c.partner_process.addr()).is_empty());
    }

    #[test]
    fn the_player_as_partner_is_not_given_the_package() {
        let mut c = conversation();
        let (player, _) = c.w.actor();
        c.w.e.set_global(PLAYER_CHARACTER, player.addr());
        c.w.answer(c.this_process.addr(), 0x128, player.addr());
        c.w.answer(player.addr(), 0xfc, 1);
        c.w.e.call(0x008b_19c0, &args![c.this, c.source]);
        // Slot 0x22c of the player is asked with 0 and nothing is sent to its process.
        assert!(c.w.calls_on(player.addr()).contains(&(0x22c, vec![0])));
        assert!(c.w.calls_on(c.partner_process.addr()).is_empty());
    }

    #[test]
    fn no_partner_or_a_partner_without_a_process_means_no_conversation() {
        let mut c = conversation();
        c.w.answer(c.this_process.addr(), 0x128, 0);
        assert!(!c.w.e.call(0x008b_19c0, &args![c.this, c.source]).bool());
        // The process was asked to look for one through slot 0x8c.
        assert!(c
            .w
            .calls_on(c.this_process.addr())
            .contains(&(0x8c, vec![c.this.addr()])));
        c.w.answer(c.this_process.addr(), 0x128, c.partner.addr());
        c.w.e.set(c.partner, Actor::pCurrentProcess, Ptr::NULL);
        assert!(!c.w.e.call(0x008b_19c0, &args![c.this, c.source]).bool());
    }

    #[test]
    fn a_non_mobile_talker_of_kind_0x16_yields_the_extra_data_actor() {
        let mut c = conversation();
        let talker = c.w.e.mem.alloc(0x40);
        // A talker object without slot 0xfc support: its vtable answers 0.
        let (not_mobile, _) = c.w.actor();
        c.w.answer(not_mobile.addr(), 0xfc, 0);
        c.w.answer(c.this_process.addr(), 0x128, not_mobile.addr());
        c.w.e.register_double(0x007a_f430, move |_, _| int(talker));
        c.w.e.register(0x0040_1170, |_, _| int(0x16));
        c.w.e.register(0x005d_43c0, |_, a| int(a[0] + 0x44));
        c.w.e.register(0x0042_e060, |_, _| Ret::default());
        let holder = c.w.e.mem.alloc(0x20);
        c.w.e.mem.set_u32(holder + 0xc, c.partner.addr());
        c.w.e.register_double(0x0042_e110, move |_, _| int(holder));
        c.w.e.call_log = Some(vec![]);
        assert!(c.w.e.call(0x008b_19c0, &args![c.this, c.source]).bool());
        let log = c.w.e.call_log.take().unwrap();
        let build = log
            .iter()
            .find(|(address, _)| *address == DIALOGUE_PACKAGE_CONSTRUCTOR)
            .unwrap();
        // Partner and the extra-data actor (argument 5) are the same.
        assert_eq!(build.1[3], c.partner.addr());
        assert_eq!(build.1[4], c.partner.addr());
        // Another kind of form ends the call.
        c.w.e.register(0x0040_1170, |_, _| int(0x17));
        assert!(!c.w.e.call(0x008b_19c0, &args![c.this, c.source]).bool());
    }

    /// `this`, a partner (both with processes) and doubles for everything
    /// `fn_008b2170` calls. Test knobs live in the partner's unused bytes:
    /// `+0x1f4` trying to enter furniture, `+0x1f8`/`+0x1fc` dismembered in
    /// limbs 1/2 (read from `this`), `+0x200` the first-item answer.
    fn talk_world() -> (World, Ptr<Actor>, Ptr, Ptr<Actor>, Ptr) {
        let mut w = world();
        let (this, this_process) = w.actor();
        let (partner, partner_process) = w.actor();
        w.answer(partner.addr(), 0x100, 1);
        w.e.register(0x008c_13d0, |e, a| int(e.mem.u8(a[0] + 0x1f4) as u32));
        w.e.register(
            0x0057_3090,
            |e, a| int(e.mem.u8(a[0] + 0x1f6 + a[1]) as u32),
        );
        w.e.register(OPERATOR_NEW, |e, a| int(e.mem.alloc(a[0])));
        w.e.register(CONVERSATION_CONSTRUCTOR, |_, a| int(a[0]));
        w.e.register(0x0093_4250, |_, _| Ret::default());
        w.e.register(0x0083_b9a0, |e, a| int(e.mem.u8(a[0] + 4) as u32));
        w.e.register(0x0079_8450, |_, _| Ret::default());
        w.e.register(DIALOGUE_PACKAGE_CONSTRUCTOR, |_, a| int(a[0]));
        for address in [
            0x0067_a1b0u32,
            0x0067_0fc0,
            0x0082_6b40,
            0x0082_6b90,
            0x0067_29d0,
            0x0067_2aa0,
            0x0067_a6b0,
            0x0067_f030,
            0x0067_f110,
            0x0067_f4d0,
            0x0067_f3c0,
            0x0067_1d30,
            0x0067_f1f0,
            0x0067_1e10,
            0x0067_2fc0,
            0x007b_3fa0,
            0x0098_4f60,
            0x0068_00b0,
            0x0068_0110,
            0x0040_3550,
            0x0057_bd60,
            0x0048_3710,
        ] {
            w.e.register(address, |_, _| Ret::default());
        }
        w.e.register(0x0067_ff70, |_, a| int(a[0]));
        w.e.register(0x0067_1d10, |_, a| int(a[0] + 0x30));
        w.e.register(0x0067_6280, |_, _| int(40));
        w.e.register(0x0040_3e20, |_, a| int(a[0] + 4));
        w.e.map(0x011c_c000, 0x1000);
        w.e.mem.set_f32(0x011c_cf3c, 2.5);
        w.e.register(0x0093_44a0, |e, a| int(e.mem.u32(a[0] + 0x204)));
        w.answer(this_process.addr(), 0x22c, 0x5151);
        (w, this, this_process, partner, partner_process)
    }

    fn talk(w: &mut World, this: Ptr<Actor>, partner: Ptr<Actor>, flags: [bool; 5]) -> bool {
        w.e.call(
            0x008b_2170,
            &args![
                this, partner, 0u32, 0u32, flags[0], flags[1], flags[2], 0x70u32, flags[3],
                flags[4]
            ],
        )
        .bool()
    }

    #[test]
    fn a_conversation_needs_a_partner_with_a_process() {
        let (mut w, this, _, partner, _) = talk_world();
        // No partner at all.
        assert!(!w
            .e
            .call(
                0x008b_2170,
                &args![this, 0u32, 0u32, 0u32, false, false, false, 0u32, false, false]
            )
            .bool());
        // Partner without a process.
        let (loner, _) = w.actor();
        w.e.set(loner, Actor::pCurrentProcess, Ptr::NULL);
        assert!(!talk(&mut w, this, loner, [false; 5]));
        // This actor without a process.
        let (idle, _) = w.actor();
        w.e.set(idle, Actor::pCurrentProcess, Ptr::NULL);
        assert!(!talk(&mut w, idle, partner, [false; 5]));
    }

    #[test]
    fn a_partner_entering_furniture_or_a_dismembered_speaker_stops_the_conversation() {
        let (mut w, this, _, partner, _) = talk_world();
        w.e.mem.set_u8(partner.addr() + 0x1f4, 1);
        assert!(!talk(&mut w, this, partner, [false; 5]));
        w.e.mem.set_u8(partner.addr() + 0x1f4, 0);
        // Slot 0x22c of this actor true and limb 2 dismembered.
        w.answer(this.addr(), 0x22c, 1);
        w.e.mem.set_u8(this.addr() + 0x1f8, 1);
        assert!(!talk(&mut w, this, partner, [false; 5]));
        // Without the dismemberment the conversation goes on to the item check.
        w.e.mem.set_u8(this.addr() + 0x1f8, 0);
        w.e.mem.set_u8(partner.addr() + 0x204, 0);
        let _ = talk(&mut w, this, partner, [false; 5]);
    }

    #[test]
    fn a_conversation_without_a_first_item_is_dropped() {
        let (mut w, this, _, partner, _) = talk_world();
        w.e.call_log = Some(vec![]);
        // The Conversation object is a fresh block whose byte +4 answers FirstItem: 0.
        assert!(!talk(&mut w, this, partner, [false; 5]));
        let log = w.e.call_log.take().unwrap();
        let conversation = log
            .iter()
            .find(|(address, _)| *address == CONVERSATION_CONSTRUCTOR)
            .unwrap()
            .1[0];
        assert!(log.contains(&(0x0079_8450, vec![conversation, 1])));
        assert!(!log
            .iter()
            .any(|(address, _)| *address == DIALOGUE_PACKAGE_CONSTRUCTOR));
    }

    /// The package the partner has to be running: its address is only
    /// known during the call, so the partner double reads it from the log.
    fn run_conversation(flags: [bool; 5], kind: u32) -> ConversationRun {
        let (mut w, this, this_process, partner, partner_process) = talk_world();
        // FirstItem answers true for every conversation block.
        w.e.register(0x0083_b9a0, |_, _| int(1));
        // The partner accepts the package: 9344a0 answers the package built last.
        let built = Rc::new(Cell::new(0));
        let seen = built.clone();
        w.e.register_double(DIALOGUE_PACKAGE_CONSTRUCTOR, move |_, a| {
            seen.set(a[0]);
            int(a[0])
        });
        let accepted = built.clone();
        w.e.register_double(0x0093_44a0, move |_, _| int(accepted.get()));
        w.answer(this_process.addr(), 0x4bc, kind);
        w.e.call_log = Some(vec![]);
        let result = talk(&mut w, this, partner, flags);
        let log = w.e.call_log.take().unwrap();
        (w, result, log, this, partner, this_process, partner_process)
    }

    #[test]
    fn a_conversation_builds_and_configures_the_package() {
        let (w, result, log, this, partner, this_process, partner_process) =
            run_conversation([true, true, false, false, false], 0);
        assert!(result);
        let (t, p) = (this.addr(), partner.addr());
        let build = log
            .iter()
            .find(|(address, _)| *address == DIALOGUE_PACKAGE_CONSTRUCTOR)
            .unwrap();
        let package = build.1[0];
        let conversation = build.1[1];
        assert_eq!(build.1[2..], [t, p, 0, 1, 1]);
        assert_eq!(w.e.mem.u32(package + 0x8c), 0x70);
        // Current package 0x5151 of the process is copied; kind 0x1c; topic.
        assert!(log.contains(&(0x0067_a1b0, vec![package, 0x5151])));
        assert!(log.contains(&(0x0067_0fc0, vec![package, 0x1c])));
        assert!(log.contains(&(0x0067_29d0, vec![package, 0x70])));
        // Option bytes: +0xbe = option_a (true), +0xbf and +0xc0 false.
        assert_eq!(w.e.mem.u8(package + 0xbe), 1);
        assert_eq!(w.e.mem.u8(package + 0xbf), 0);
        assert_eq!(w.e.mem.u8(package + 0xc0), 0);
        // The head-track flag turns off the dialogue-without-head flag call.
        assert!(log.contains(&(0x0067_a6b0, vec![package, 0])));
        // The second location radius: truncated 2.5 + 40.
        let radius = log
            .iter()
            .find(|(address, _)| *address == 0x0067_f1f0)
            .unwrap();
        assert_eq!(radius.1[1], 42);
        // Both locations are constructed and destroyed, the first one bound to this actor.
        let first = log
            .iter()
            .find(|(address, _)| *address == 0x0067_f3c0)
            .unwrap()
            .1[0];
        assert_eq!(radius.1[0], first + 0xc);
        assert!(log.contains(&(0x0067_f3c0, vec![first, t])));
        assert!(log.contains(&(0x0067_f110, vec![first + 0xc])));
        assert!(log.contains(&(0x0067_f110, vec![first])));
        // Targets: the partner as reference; 0x5a for process kind other than 4.
        assert!(log.contains(&(0x0068_0110, vec![package + 0x30, p])));
        assert!(log.contains(&(0x0068_00b0, vec![package + 0x30, 0])));
        assert!(log.contains(&(0x0040_3550, vec![package + 0x30, 0x5a])));
        // Head tracking: both processes face the other.
        let on_this = w.calls_on(this_process.addr());
        assert!(on_this.contains(&(0x644, vec![1])));
        assert!(on_this.contains(&(0x634, vec![p])));
        assert!(on_this.contains(&(0x288, vec![t, 2])));
        assert_eq!(
            w.calls_on(partner_process.addr()),
            vec![(0x644, vec![1]), (0x634, vec![t]), (0x288, vec![p, 2])]
        );
        assert!(w.calls_on(t).contains(&(0x2f4, vec![package, 1, 1])));
        assert!(w.calls_on(p).contains(&(0x2f4, vec![package, 1, 1])));
        assert!(log.contains(&(0x0057_bd60, vec![t, p])));
        assert!(log.contains(&(0x0057_bd60, vec![p, t])));
        let _ = conversation;
    }

    #[test]
    fn process_kind_four_and_the_second_option_change_the_calls() {
        let (w, result, log, this, _, this_process, _) =
            run_conversation([false, false, true, false, false], 4);
        assert!(result);
        let package = log
            .iter()
            .find(|(address, _)| *address == DIALOGUE_PACKAGE_CONSTRUCTOR)
            .unwrap()
            .1[0];
        assert!(log.contains(&(0x0040_3550, vec![package + 0x30, 0xc8])));
        // No head tracking; option_b gives slot 0x288 the value 1.
        let on_this = w.calls_on(this_process.addr());
        assert!(!on_this.iter().any(|(slot, _)| *slot == 0x644));
        assert!(on_this.contains(&(0x288, vec![this.addr(), 1])));
        assert!(log.contains(&(0x0067_a6b0, vec![package, 1])));
    }

    #[test]
    fn the_package_is_not_given_to_a_partner_when_option_c_without_option_d() {
        let (w, result, log, _, partner, _, partner_process) =
            run_conversation([false, false, false, true, false], 0);
        assert!(result);
        assert!(w.calls_on(partner_process.addr()).is_empty());
        assert!(!w
            .calls_on(partner.addr())
            .iter()
            .any(|(slot, _)| *slot == 0x2f4));
        let _ = log;
    }

    #[test]
    fn a_partner_that_refuses_the_package_fails_the_conversation() {
        let (mut w, this, _, partner, _) = talk_world();
        w.e.register(0x0083_b9a0, |_, _| int(1));
        w.e.mem.set_u32(partner.addr() + 0x204, 0x9999);
        w.e.call_log = Some(vec![]);
        assert!(!talk(&mut w, this, partner, [false; 5]));
        let log = w.e.call_log.take().unwrap();
        // This actor is told to give up (slot 0x288 without arguments), and both
        // locations are destroyed.
        assert!(w.calls_on(this.addr()).contains(&(0x288, vec![])));
        assert_eq!(
            log.iter()
                .filter(|(address, _)| *address == 0x0067_f110)
                .count(),
            2
        );
    }

    #[test]
    fn the_player_as_partner_gets_no_conversation_object() {
        let (mut w, this, _, _, _) = talk_world();
        let (player, player_process) = w.actor();
        w.e.set_global(PLAYER_CHARACTER, player.addr());
        w.e.call_log = Some(vec![]);
        assert!(talk(&mut w, this, player, [false; 5]));
        let log = w.e.call_log.take().unwrap();
        assert!(!log
            .iter()
            .any(|(address, _)| *address == CONVERSATION_CONSTRUCTOR));
        assert!(w.calls_on(player_process.addr()).is_empty());
    }

    #[test]
    fn dialogue_package_byte_setters() {
        let (mut e, _) = engine();
        let package = e.mem.alloc(0x100);
        e.call(0x008b_2860, &args![Ptr::<()>::new(package), 1u8]);
        e.call(0x008b_2880, &args![Ptr::<()>::new(package), 2u8]);
        e.call(0x008b_28a0, &args![Ptr::<()>::new(package), 3u8]);
        assert_eq!(
            [
                e.mem.u8(package + 0xbe),
                e.mem.u8(package + 0xbf),
                e.mem.u8(package + 0xc0)
            ],
            [1, 2, 3]
        );
    }

    #[test]
    fn the_player_animation_bonus_of_008b2b60() {
        let (mut e, _) = engine();
        e.register(PLAYER_GET_ANIMATION, |_, _| int(0xa11));
        let value = Rc::new(Cell::new(1.0));
        let seen = value.clone();
        e.register_double(0x0050_8070, move |_, _| float(seen.get()));
        let actor = e.mem.alloc(0x200);
        let call = |e: &mut Engine, animation: u32, actor: u32| {
            e.call(0x008b_2b60, &args![0u32, animation, 10u32, actor])
                .i32()
        };
        // The player's own animation with no actor, or with bit 0x40 clear.
        assert_eq!(call(&mut e, 0xa11, 0), 13);
        assert_eq!(call(&mut e, 0xa11, actor), 13);
        e.mem.set_u8(actor + 0x100, 0x40);
        assert_eq!(call(&mut e, 0xa11, actor), 10);
        // Another animation: bit 0x100 of the word at +0x12c must be clear.
        assert_eq!(call(&mut e, 0xb22, 0), 13);
        assert_eq!(call(&mut e, 0xb22, actor), 13);
        e.mem.set_u32(actor + 0x12c, 0x100);
        assert_eq!(call(&mut e, 0xb22, actor), 10);
        // A value that is not positive returns the base at once.
        e.mem.set_u32(actor + 0x12c, 0);
        value.set(0.0);
        assert_eq!(call(&mut e, 0xb22, actor), 10);
        value.set(-2.0);
        assert_eq!(call(&mut e, 0xa11, actor), 10);
    }

    #[test]
    fn flag_getters_008b2bf0_and_008b2c10() {
        let (mut e, _) = engine();
        let object = e.mem.alloc(0x200);
        assert!(e.call(0x008b_2bf0, &args![Ptr::<()>::new(object)]).bool());
        assert!(e.call(0x008b_2c10, &args![Ptr::<()>::new(object)]).bool());
        e.mem.set_u8(object + 0x100, 0x40);
        e.mem.set_u32(object + 0x12c, 0x100);
        assert!(!e.call(0x008b_2bf0, &args![Ptr::<()>::new(object)]).bool());
        assert!(!e.call(0x008b_2c10, &args![Ptr::<()>::new(object)]).bool());
        e.mem.set_u8(object + 0x100, 0xbf);
        e.mem.set_u32(object + 0x12c, 0xfffffeff);
        assert!(e.call(0x008b_2bf0, &args![Ptr::<()>::new(object)]).bool());
        assert!(e.call(0x008b_2c10, &args![Ptr::<()>::new(object)]).bool());
    }

    /// `fn_008b28c0` doubles: `GetType(x)` is `x + 0x3b`, `GetAnimGroup`
    /// answers 5, `PickBestAnimation(best)` answers `best`, `PlayGroup`
    /// answers 99; `5f2540` (attack) answers `attack`, aim false.
    fn play_world(attack: bool, iron_sights: bool) -> (World, Ptr<Actor>, Ptr) {
        let mut w = world();
        let (this, _) = w.actor();
        let animation = w.e.mem.alloc(0x40);
        w.e.register_double(0x005f_2540, move |_, _| int(attack as u32));
        w.e.register(0x005f_2630, |_, _| int(0));
        w.e.register(0x005f_2750, |_, _| int(1));
        w.e.register(0x005f_2440, |_, a| int(a[0] + 0x3b));
        w.e.register(0x0089_7910, |_, _| int(5));
        w.e.register(0x0049_5740, |_, a| int(a[1]));
        w.e.register(0x0049_4740, |_, _| int(99));
        w.e.register(0x0049_6080, |_, _| Ret::default());
        w.e.register_double(ACTOR_GET_IRON_SIGHTS, move |_, _| int(iron_sights as u32));
        w.e.register(0x0044_ddc0, |_, _| int(0));
        w.e.register(PLAYER_GET_ANIMATION, |_, _| int(0xa11));
        (w, this, Ptr::new(animation))
    }

    #[test]
    fn a_plain_group_is_played_and_reported() {
        let (mut w, this, animation) = play_world(false, false);
        let process = w.e.get(this, Actor::pCurrentProcess);
        w.e.call_log = Some(vec![]);
        w.e.call(0x008b_28c0, &args![this, 0x40u32, animation]);
        let log = w.e.call_log.take().unwrap();
        assert!(log.contains(&(0x0089_7910, vec![this.addr(), 0x40, 0, 0, animation.addr()])));
        assert!(log.contains(&(
            0x0049_4740,
            vec![animation.addr(), 5, 1, u32::MAX, u32::MAX]
        )));
        assert_eq!(
            w.calls_on(process.addr()),
            vec![
                (0x148, vec![]),
                (0x424, vec![0, 99]),
                (0x428, vec![this.addr()])
            ]
        );
    }

    #[test]
    fn a_group_of_another_type_is_cleared_instead() {
        let (mut w, this, animation) = play_world(false, false);
        let process = w.e.get(this, Actor::pCurrentProcess);
        w.e.call_log = Some(vec![]);
        // GetType(5) = 0x40 is not 0x30.
        w.e.call(0x008b_28c0, &args![this, 0x30u32, animation]);
        let log = w.e.call_log.take().unwrap();
        assert!(log.contains(&(0x0049_6080, vec![animation.addr(), 4, 0.0f32.to_bits()])));
        assert!(!log.iter().any(|(address, _)| *address == 0x0049_4740));
        assert!(w.calls_on(process.addr()).contains(&(0x424, vec![0, 0])));
        // Group 0xff is never played.
        w.e.call_log = Some(vec![]);
        w.e.call(0x008b_28c0, &args![this, 0xffu32, animation]);
        let log = w.e.call_log.take().unwrap();
        assert!(!log.iter().any(|(address, _)| *address == 0x0089_7910));
    }

    #[test]
    fn attack_groups_try_three_followers_from_the_best() {
        let (mut w, this, animation) = play_world(true, false);
        w.e.call_log = Some(vec![]);
        // GetType(5) = 0x40 = the group; the followers 6 and 7 give 0x41 and 0x42.
        w.e.call(0x008b_28c0, &args![this, 0x40u32, animation]);
        let log = w.e.call_log.take().unwrap();
        let played: Vec<u32> = log
            .iter()
            .filter(|(address, _)| *address == 0x0049_4740)
            .map(|(_, a)| a[1])
            .collect();
        assert_eq!(played, vec![5, 6, 7]);
        assert!(log.contains(&(0x0049_5740, vec![animation.addr(), 6, 0])));
        assert!(log.contains(&(0x0049_5740, vec![animation.addr(), 7, 0])));
        let process = w.e.get(this, Actor::pCurrentProcess);
        let reported: Vec<_> = w
            .calls_on(process.addr())
            .into_iter()
            .filter(|(slot, _)| *slot == 0x424)
            .collect();
        assert_eq!(
            reported,
            vec![
                (0x424, vec![0, 99]),
                (0x424, vec![1, 99]),
                (0x424, vec![2, 99])
            ]
        );
    }

    #[test]
    fn the_players_own_animation_is_not_reported_to_the_process() {
        let (mut w, this, _) = play_world(false, false);
        let process = w.e.get(this, Actor::pCurrentProcess);
        w.e.call(0x008b_28c0, &args![this, 0x40u32, Ptr::<()>::new(0xa11)]);
        assert_eq!(
            w.calls_on(process.addr()),
            vec![(0x148, vec![]), (0x428, vec![this.addr()])]
        );
    }

    #[test]
    fn iron_sights_shift_the_group_and_are_undone_for_the_first_action() {
        let (mut w, this, animation) = play_world(true, true);
        // The animation is the player's first-person one and the item is null:
        // 008b2b60 adds 3, and 005f2750 + GetType undo it again for group 0.
        let first_person = Ptr::<()>::new(0xa11);
        let _ = animation;
        w.e.call_log = Some(vec![]);
        w.e.call(0x008b_28c0, &args![this, 0x40u32, first_person]);
        let log = w.e.call_log.take().unwrap();
        // i = 0: GetAnimGroup is asked for 0x40 + 3.
        assert!(log.contains(&(0x0089_7910, vec![this.addr(), 0x43, 0, 0, 0xa11])));
        let played: Vec<u32> = log
            .iter()
            .filter(|(address, _)| *address == 0x0049_4740)
            .map(|(_, a)| a[1])
            .collect();
        assert_eq!(played, vec![5, 6, 7]);
    }

    /// The player's weapon node: node block `+4` holds its collision object.
    fn recoil_world() -> (World, Ptr<Actor>, u32, SharedLog) {
        let mut w = world();
        let (player, process) = w.actor();
        w.e.set_global(PLAYER_CHARACTER, player.addr());
        w.answer(process.addr(), 0x148, 0x7000);
        w.answer(process.addr(), 0x454, 1);
        w.e.set(player, Actor::fGunSkillActor, 2.0);
        w.e.register(0x0044_ddc0, |_, _| int(0x5000));
        w.e.register(0x0068_15c0, |_, _| Ret::default());
        w.e.register(ACTOR_GET_ANIMATION, |_, _| int(0xa1));
        w.e.register(0x0045_3700, |_, _| float(0.5));
        w.e.register(0x0044_6390, |_, _| int(2));
        w.e.map(0x0118_a000, 0x1000);
        w.e.mem.set_u32(0x0118_a838 + 8, 0xdead);
        w.e.register(0x008d_6970, |_, a| {
            assert_eq!((a[0], f32::from_bits(a[1])), (0xdead, 0.5));
            int(0x3000)
        });
        w.e.register(0x0043_fcd0, |_, _| int(0x4000));
        let node = w.e.mem.alloc(0x20);
        w.e.register_double(0x004a_ae30, move |_, a| {
            assert_eq!((a[0], a[1]), (0x4000, 0x3000));
            int(node)
        });
        w.e.register(0x0089_4900, |_, _| int(0));
        w.e.register(0x0040_3e20, |_, a| int(a[0] + 4));
        w.e.map(0x011c_e000, 0x1000);
        w.e.mem.set_f32(0x011c_ec38, 3.0);
        w.e.register(0x0068_38b0, |e, a| int(e.mem.u32(a[0] + 4)));
        w.e.register(0x0062_bc90, |e, a| {
            e.mem.set_u32(a[0] + 4, a[1]);
            Ret::default()
        });
        w.e.register(0x00aa_13e0, |e, a| int(e.mem.alloc(a[0])));
        w.e.register(0x00c8_ffd0, |_, a| int(a[0]));
        w.e.register(0x0065_3270, |_, a| {
            int(if a[0] == 0x0126_81cc { a[1] } else { 0 })
        });
        w.e.map(0x011a_9000, 0x1000);
        for word in 0..9 {
            w.e.mem.set_f32(0x011a_9448 + 4 * word, word as f32);
        }
        w.e.register(0x00a5_92c0, |e, a| {
            e.mem.set_f32(a[1], 0.1);
            e.mem.set_f32(a[2], 0.2);
            e.mem.set_f32(a[3], 0.3);
            Ret::default()
        });
        let euler = Rc::new(std::cell::RefCell::new(vec![]));
        let seen = euler.clone();
        w.e.register_double(0x00a5_9540, move |_, a| {
            seen.borrow_mut().push((0x00a5_9540, a.to_vec()));
            Ret::default()
        });
        w.e.register(0x0043_f8d0, |e, a| {
            for word in 0..9 {
                e.mem.set_f32(a[1] + 4 * word, 100.0 + word as f32);
            }
            int(a[1])
        });
        let seen = euler.clone();
        w.e.register_double(0x004f_0110, move |e, a| {
            let first = e.mem.f32(a[1]);
            seen.borrow_mut()
                .push((0x004f_0110, vec![a[0], first.to_bits()]));
            Ret::default()
        });
        (w, player, node, euler)
    }

    #[test]
    fn the_recoil_rotation_is_scaled_and_set_on_a_new_collision_object() {
        let (mut w, player, node, calls) = recoil_world();
        w.e.call(0x008b_2c30, &args![player]);
        let collision = w.e.mem.u32(node + 4);
        assert_ne!(collision, 0);
        let calls = calls.borrow();
        // scale = 2.0 (cached) * 3.0 (setting); angles 0.1, 0.2, 0.3.
        let (_, euler) = &calls[0];
        assert_eq!(f32::from_bits(euler[1]), 0.1f32 * 6.0);
        assert_eq!(f32::from_bits(euler[2]), 0.2f32 * 6.0);
        assert_eq!(f32::from_bits(euler[3]), 0.3f32 * 6.0);
        // The product (starting at 100.0) is what the collision object receives.
        assert_eq!(calls[1], (0x004f_0110, vec![collision, 100.0f32.to_bits()]));
    }

    #[test]
    fn an_existing_collision_object_without_blend_is_replaced_by_a_copy() {
        let (mut w, player, node, _) = recoil_world();
        let existing = w.e.mem.alloc(0x20);
        w.e.mem.set_u32(node + 4, existing);
        // The first cast (0x1267e64) finds a source, the second (0x12681cc)
        // says it is not a blend object yet; afterwards the copy is one.
        let copied = Rc::new(Cell::new(0));
        let seen = copied.clone();
        w.e.register_double(0x00c9_0090, move |_, a| {
            seen.set(a[0] + 1000);
            assert_eq!(a[1], 0x6000);
            Ret::default()
        });
        w.e.register(0x0065_3270, |_, a| {
            int(if a[0] == 0x0126_7e64 {
                0x6000
            } else if a[1] == 0x6000 {
                0
            } else {
                a[1]
            })
        });
        w.e.call(0x008b_2c30, &args![player]);
        let replaced = w.e.mem.u32(node + 4);
        assert_ne!(replaced, existing);
        assert_eq!(copied.get(), replaced + 1000);
    }

    #[test]
    fn the_recoil_needs_the_player_a_process_an_item_and_slot_0x454() {
        let (mut w, player, node, calls) = recoil_world();
        let (other, _) = w.actor();
        w.e.call(0x008b_2c30, &args![other]);
        w.answer(w.e.get(player, Actor::pCurrentProcess).addr(), 0x454, 0);
        w.e.call(0x008b_2c30, &args![player]);
        w.answer(w.e.get(player, Actor::pCurrentProcess).addr(), 0x454, 1);
        w.answer(w.e.get(player, Actor::pCurrentProcess).addr(), 0x148, 0);
        w.e.call(0x008b_2c30, &args![player]);
        w.e.set(player, Actor::pCurrentProcess, Ptr::NULL);
        w.e.call(0x008b_2c30, &args![player]);
        assert_eq!(w.e.mem.u32(node + 4), 0);
        assert!(calls.borrow().is_empty());
    }

    #[test]
    fn the_recoil_stops_without_a_node_name_or_node() {
        let (mut w, player, node, calls) = recoil_world();
        w.e.register(0x008d_6970, |_, _| int(0));
        w.e.call(0x008b_2c30, &args![player]);
        w.e.register(0x008d_6970, |_, _| int(0x3000));
        w.e.register(0x004a_ae30, |_, _| int(0));
        w.e.call(0x008b_2c30, &args![player]);
        assert_eq!(w.e.mem.u32(node + 4), 0);
        assert!(calls.borrow().is_empty());
    }

    /// An actor whose slot `0x1f4` returns the position `(1, 2, 3)`, with
    /// doubles for `IsAtPoint`.
    fn point_world(height: f32) -> (Engine, Ptr<Actor>, u32, VectorLog) {
        let (mut e, _) = engine();
        e.map(0x0101_d000, 0x1000);
        e.map(0x0101_e000, 0x1000);
        e.map(0x0102_f000, 0x1000);
        e.set_global(0x0101_db88, 30.0f64);
        e.set_global(0x0101_e704, 128.0f32);
        e.set_global(0x0101_e340, 32.0f32);
        e.set_global(0x0102_f070, 32.0f64);
        let position = e.mem.alloc(12);
        for (i, v) in [100.0f32, 200.0, 50.0].iter().enumerate() {
            e.mem.set_f32(position + 4 * i as u32, *v);
        }
        e.register_double(0x0f00_0090, move |_, _| int(position));
        let actor = actor_with_slots(&mut e, &[(0x1f4, 0x0f00_0090)]);
        e.register(0x0093_06d0, |_, _| int(0));
        e.register_double(0x0088_53a0, move |_, _| float(height as f64));
        // The vector from the point to the actor.
        e.register(0x0043_9ef0, |e, a| {
            for word in 0..3 {
                let value = e.mem.f32(a[0] + 4 * word) - e.mem.f32(a[2] + 4 * word);
                e.mem.set_f32(a[1] + 4 * word, value);
            }
            int(a[1])
        });
        let seen = Rc::new(std::cell::RefCell::new(vec![]));
        let log = seen.clone();
        e.register_double(0x004b_5470, move |e, a| {
            log.borrow_mut().push(vec![
                e.mem.f32(a[0]).to_bits(),
                e.mem.f32(a[0] + 4).to_bits(),
                e.mem.f32(a[0] + 8).to_bits(),
                a[1],
            ]);
            int(0)
        });
        let point = e.mem.alloc(12);
        (e, actor, point, seen)
    }

    #[test]
    fn a_point_within_the_height_window_ignores_the_vertical_distance() {
        let (mut e, actor, point, seen) = point_world(0.0);
        for (i, v) in [90.0f32, 195.0, 100.0].iter().enumerate() {
            e.mem.set_f32(point + 4 * i as u32, *v);
        }
        // Height 0 means 128: the window is 50 - 30 .. 50 + 128 + 30.
        assert!(e
            .call(0x008b_2f90, &args![actor, point, 5.0f32, false, true])
            .bool());
        let vector = seen.borrow()[0].clone();
        assert_eq!(f32::from_bits(vector[0]), 10.0);
        assert_eq!(f32::from_bits(vector[1]), 5.0);
        assert_eq!(f32::from_bits(vector[2]), 0.0);
        assert_eq!(f32::from_bits(vector[3]), 5.0);
        // Outside the window the vertical part stays.
        e.mem.set_f32(point + 8, 300.0);
        e.call(0x008b_2f90, &args![actor, point, 5.0f32, false, true]);
        assert_eq!(f32::from_bits(seen.borrow()[1][2]), -250.0);
        // A given height narrows the window.
        e.register(0x0088_53a0, |_, _| float(10.0));
        e.mem.set_f32(point + 8, 95.0);
        e.call(0x008b_2f90, &args![actor, point, 5.0f32, false, true]);
        assert_eq!(f32::from_bits(seen.borrow()[2][2]), -45.0);
        e.mem.set_f32(point + 8, 80.0);
        e.call(0x008b_2f90, &args![actor, point, 5.0f32, false, true]);
        assert_eq!(f32::from_bits(seen.borrow()[3][2]), 0.0);
    }

    #[test]
    fn the_controller_can_drop_the_vertical_part_without_the_window() {
        let (mut e, actor, point, seen) = point_world(0.0);
        let controller = e.mem.alloc(0x500);
        e.register_double(0x0093_06d0, move |_, _| int(controller));
        e.register(0x005c_0880, |_, _| int(1));
        e.mem.set_f32(point + 8, 1000.0);
        e.call(0x008b_2f90, &args![actor, point, 5.0f32, false, false]);
        assert_eq!(f32::from_bits(seen.borrow()[0][2]), 0.0);
        // Forcing the window leaves it (1000 is far above the window).
        e.call(0x008b_2f90, &args![actor, point, 5.0f32, false, true]);
        assert_eq!(f32::from_bits(seen.borrow()[1][2]), -950.0);
    }

    #[test]
    fn the_actors_radius_is_added_to_the_distance() {
        let (mut e, actor, point, seen) = point_world(0.0);
        // No controller: plus 32.
        e.call(0x008b_2f90, &args![actor, point, 5.0f32, true, true]);
        assert_eq!(f32::from_bits(seen.borrow()[0][3]), 37.0);
        let controller = e.mem.alloc(0x500);
        e.register_double(0x0093_06d0, move |_, _| int(controller));
        e.register(0x005c_0880, |_, _| int(0));
        e.register(0x00c6_e280, |_, _| float(40.0));
        e.register(0x0045_87d0, |_, a| float(f32::from_bits(a[0]) as f64 + 1.0));
        e.register_double(0x008a_5170, move |_, a| {
            assert_eq!(a[0], controller + 0x410);
            int(0)
        });
        e.call(0x008b_2f90, &args![actor, point, 5.0f32, true, true]);
        assert_eq!(f32::from_bits(seen.borrow()[1][3]), 46.0);
        e.register(0x008a_5170, |_, _| int(1));
        e.call(0x008b_2f90, &args![actor, point, 5.0f32, true, true]);
        assert_eq!(f32::from_bits(seen.borrow()[2][3]), 87.0);
        // Below 32 the minimum applies.
        e.register(0x00c6_e280, |_, _| float(3.0));
        e.register(0x008a_5170, |_, _| int(0));
        e.call(0x008b_2f90, &args![actor, point, 5.0f32, true, true]);
        assert_eq!(f32::from_bits(seen.borrow()[3][3]), 37.0);
    }

    #[test]
    fn the_distance_comparison_decides_the_result() {
        let (mut e, actor, point, _) = point_world(0.0);
        for (answer, expected) in [(-1i32, true), (0, true), (1, false)] {
            e.register_double(0x004b_5470, move |_, _| int(answer as u32));
            assert_eq!(
                e.call(0x008b_2f90, &args![actor, point, 1.0f32, false, true])
                    .bool(),
                expected
            );
        }
    }

    fn mover_world() -> (World, Ptr<Actor>, Ptr, SharedLog) {
        let mut w = world();
        let (this, _) = w.actor();
        let mover = object_with_slots(&mut w.e, 0x100, &[(0, 0x0f30_0000), (0x14, 0x0f20_0014)]);
        w.e.set(this, Actor::pActorMover, mover);
        w.e.register(0x0f30_0000, |_, _| Ret::default());
        let seen = Rc::new(std::cell::RefCell::new(vec![]));
        let log = seen.clone();
        // Every ActorMover function this part calls records its arguments.
        for address in [
            0x009d_ba30u32,
            0x009d_ba50,
            0x009d_bb10,
            0x009d_bb60,
            0x009d_bc30,
            0x009d_c950,
            0x006d_61e0,
        ] {
            let log = log.clone();
            w.e.register_double(address, move |_, a| {
                log.borrow_mut().push((address, a.to_vec()));
                Ret::default()
            });
        }
        (w, this, mover, seen)
    }

    #[test]
    fn the_actor_mover_is_created_destroyed_and_fed() {
        let (mut w, this, mover, log) = mover_world();
        // Created: the new mover replaces the old pointer.
        w.e.register(OPERATOR_NEW, |e, a| {
            assert_eq!(a[0], 0x88);
            int(e.mem.alloc(a[0]))
        });
        w.e.register(0x009d_ad00, |_, a| int(a[0]));
        w.e.call(0x008b_30f0, &args![this]);
        let created = w.e.get(this, Actor::pActorMover);
        assert_ne!(created, mover);
        assert!(!created.is_null());
        // A failed allocation stores null.
        w.e.register(OPERATOR_NEW, |_, _| int(0));
        w.e.call(0x008b_30f0, &args![this]);
        assert!(w.e.get(this, Actor::pActorMover).is_null());
        // Value passed on.
        w.e.set(this, Actor::pActorMover, mover);
        w.e.call(0x008b_3200, &args![this, 0x1234u32]);
        assert_eq!(log.borrow()[0], (0x009d_c950, vec![mover.addr(), 0x1234]));
        w.e.set(this, Actor::pActorMover, Ptr::NULL);
        w.e.call(0x008b_3200, &args![this, 1u32]);
        assert_eq!(log.borrow().len(), 1);
    }

    #[test]
    fn destroying_the_actor_mover_clears_inventory_references_first() {
        let (mut w, this, mover, _) = mover_world();
        let destroyed = Rc::new(Cell::new(0));
        let seen = destroyed.clone();
        w.e.register_double(0x0f30_0000, move |_, a| {
            assert_eq!(a[1], 1);
            seen.set(a[0]);
            Ret::default()
        });
        w.e.register(0x005d_43c0, |_, a| int(a[0] + 0x44));
        w.e.register(0x0041_8520, |_, a| int(a[0] + 1));
        w.e.register(0x0047_d0b0, |_, a| int(a[0] + 2));
        let cleared = Rc::new(Cell::new(0));
        let seen = cleared.clone();
        w.e.register_double(0x006e_bd10, move |_, a| {
            seen.set(a[0]);
            Ret::default()
        });
        w.e.call(0x008b_3180, &args![this]);
        assert_eq!(cleared.get(), this.addr() + 0x44 + 1 + 2);
        assert_eq!(destroyed.get(), mover.addr());
        assert!(w.e.get(this, Actor::pActorMover).is_null());
        // Without a mover nothing is destroyed.
        destroyed.set(0);
        w.e.call(0x008b_3180, &args![this]);
        assert_eq!(destroyed.get(), 0);
        // Without extra data the path manager is left alone.
        w.e.register(0x005d_43c0, |_, _| int(0));
        w.e.register(0x006e_bd10, |_, _| panic!("no extra data"));
        w.e.call(0x008b_3180, &args![this]);
    }

    #[test]
    fn pathfinding_goals_go_to_the_mover_with_the_default_from_the_process() {
        let (mut w, this, mover, log) = mover_world();
        let (t, m) = (this.addr(), mover.addr());
        let (_, process) = {
            let p = w.e.get(this, Actor::pCurrentProcess);
            ((), p)
        };
        w.answer(process.addr(), 0x254, 0x5555);
        w.e.register(0x0069_e390, |_, a| int((a[0] == 0x77) as u32));
        // A goal that fails 0069e390 first gets the process value.
        w.e.call(0x008b_3630, &args![this, 0x66u32]);
        w.e.call(0x008b_3630, &args![this, 0x77u32]);
        w.e.call(0x008b_3630, &args![this, 0u32]);
        w.e.call(0x008b_3690, &args![this, 1u32, 2u32, 3u32, 4.5f32, 0u32]);
        w.e.call(0x008b_3690, &args![this, 1u32, 2u32, 3u32, 4.5f32, 9u32]);
        w.e.call(0x008b_36f0, &args![this, 1u32, 2.5f32, 0u32]);
        w.e.call(
            0x008b_3750,
            &args![this, 1u32, 2u32, 3u32, 4.5f32, 5.5f32, 0u32],
        );
        w.e.call(0x008b_37c0, &args![this, 1u32, 2.5f32, 8u32]);
        let log = log.borrow();
        assert_eq!(
            *log,
            vec![
                (0x006d_61e0, vec![0x66, 0x5555]),
                (0x009d_ba30, vec![m, 0x66]),
                (0x009d_ba30, vec![m, 0x77]),
                (0x009d_ba30, vec![m, 0]),
                (0x009d_ba50, vec![m, 1, 2, 3, 4.5f32.to_bits(), 0x5555]),
                (0x009d_ba50, vec![m, 1, 2, 3, 4.5f32.to_bits(), 9]),
                (0x009d_bb10, vec![m, 1, 2.5f32.to_bits(), 0x5555]),
                (
                    0x009d_bb60,
                    vec![m, 1, 2, 3, 4.5f32.to_bits(), 5.5f32.to_bits(), 0x5555]
                ),
                (0x009d_bc30, vec![m, 1, 2.5f32.to_bits(), 8]),
            ]
        );
        let _ = t;
    }

    /// An actor and everything `fn_008b3230` calls; slot answers in
    /// the `World`, knobs through cells.
    struct FrameKnobs {
        playing: Cell<bool>,
        valid: Cell<bool>,
        play_flag: Cell<bool>,
    }

    fn frame_world() -> (World, Ptr<Actor>, Ptr, Rc<FrameKnobs>) {
        let mut w = world();
        let (this, _) = w.actor();
        let mover = object_with_slots(&mut w.e, 0x100, &[(0x14, 0x0f20_0014)]);
        w.e.set(this, Actor::pActorMover, mover);
        let magic = w.e.mem.alloc(0x800);
        w.e.mem.set_u32(magic + 8, 0x0f20_0008);
        w.e.mem.set_u32(this.addr() + 0xa4, magic);
        w.e.map(0x011f_6000, 0x1000);
        w.e.mem.set_f32(0x011f_63a0, 0.25);
        for address in [0x0040_4eb0u32, 0x0040_4ee0] {
            w.e.register(address, |_, _| Ret::default());
        }
        w.e.register(0x0093_1850, |_, _| int(0));
        w.e.register(0x0044_ddc0, |e, a| int(e.mem.u32(a[0] + 8)));
        w.e.register(0x0084_d030, |e, a| float(e.mem.f32(a[0] + 0xc) as f64));
        w.e.register(0x009c_8d60, |_, _| float(2.0));
        w.e.register(0x004e_3d00, |_, _| float(3.0));
        let knobs = Rc::new(FrameKnobs {
            playing: Cell::new(false),
            valid: Cell::new(false),
            play_flag: Cell::new(false),
        });
        (w, this, mover, knobs)
    }

    fn mover_calls(w: &World, mover: Ptr) -> Vec<(u32, Vec<u32>)> {
        w.calls_on(mover.addr())
    }

    #[test]
    fn the_player_is_not_updated() {
        let (mut w, _, _, _) = frame_world();
        let (player, _) = w.actor();
        w.e.set_global(PLAYER_CHARACTER, player.addr());
        w.e.call_log = Some(vec![]);
        w.e.call(0x008b_3230, &args![player, 0.5f32]);
        let log = w.e.call_log.take().unwrap();
        let addresses: Vec<u32> = log.iter().map(|(address, _)| *address).collect();
        assert_eq!(addresses, vec![0x008b_3230, 0x0040_4eb0, 0x0040_4ee0]);
        assert_eq!(log[1].1[1..], [0x2d, 1, 0x0108_4918, 0x5d06]);
    }

    #[test]
    fn the_elapsed_time_goes_to_the_mover() {
        let (mut w, this, mover, _) = frame_world();
        w.answer(this.addr(), 0x21c, 0);
        w.e.call(0x008b_3230, &args![this, 0.5f32]);
        assert_eq!(mover_calls(&w, mover), vec![(0x14, vec![0.5f32.to_bits()])]);
        // Slot 8 of the magic target answering 0x33 > 0 with an animation scales it.
        w.log.borrow_mut().clear();
        w.answer(this.addr() + 0xa4, 8, 1);
        w.answer(this.addr(), 0x1e4, 0xa11);
        w.e.call(0x008b_3230, &args![this, 0.5f32]);
        assert_eq!(mover_calls(&w, mover), vec![(0x14, vec![1.5f32.to_bits()])]);
    }

    #[test]
    fn a_zero_elapsed_time_is_replaced_by_the_setting() {
        let (mut w, this, mover, _) = frame_world();
        w.e.call(0x008b_3230, &args![this, 0.0f32]);
        assert_eq!(
            mover_calls(&w, mover),
            vec![(0x14, vec![0.25f32.to_bits()])]
        );
        // In VATS mode (4) the VATS target gets the update multiplier.
        w.log.borrow_mut().clear();
        w.e.mem.set_u32(0x011f_2250 + 8, 4);
        w.e.mem.set_u32(0x011f_21cc, this.addr());
        w.e.call(0x008b_3230, &args![this, 0.0f32]);
        assert_eq!(mover_calls(&w, mover), vec![(0x14, vec![0.5f32.to_bits()])]);
        // A nonzero process type keeps the zero.
        w.log.borrow_mut().clear();
        w.e.register(0x0093_1850, |_, _| int(1));
        w.e.call(0x008b_3230, &args![this, 0.0f32]);
        assert_eq!(mover_calls(&w, mover), vec![(0x14, vec![0])]);
    }

    /// Doubles for the creature sound part; `playing`/`valid` are read from
    /// the knobs, and every handle call is logged by address.
    fn sound_world() -> (World, Ptr<Actor>, Rc<FrameKnobs>) {
        let (mut w, this, _, knobs) = frame_world();
        w.answer(this.addr(), 0x21c, 1);
        w.answer(this.addr(), 0x1d0, 0x1111);
        w.e.register(0x007a_f430, |_, _| int(0x2222));
        w.e.register(0x0059_f3a0, |_, _| int(6));
        w.e.register(0x005f_92d0, |_, a| {
            assert_eq!((a[0], a[1]), (0x2222, 10));
            int(0x3333)
        });
        w.e.register(0x0049_38e0, |_, _| int(1));
        w.e.register(0x008a_3b30, |_, _| int(0));
        w.e.register(0x0041_a250, |_, _| Ret::default());
        w.e.register(0x005d_43c0, |_, a| int(a[0] + 0x44));
        w.e.register(0x0041_8980, |_, _| Ret::default());
        w.e.register(0x0041_a280, |_, _| Ret::default());
        let k = knobs.clone();
        w.e.register_double(0x00ad_8930, move |_, _| int(k.playing.get() as u32));
        let k = knobs.clone();
        w.e.register_double(0x00ad_8ce0, move |_, _| int(k.valid.get() as u32));
        w.e.register(0x004e_75d0, |_, a| int(a[1]));
        w.e.register(0x005e_39b0, |_, _| int(0x10));
        w.e.register(0x0051_1840, |_, _| int(0x4444));
        w.e.register(0x0045_3a70, |_, _| int(0x5555));
        w.e.register(0x00ad_7480, |_, a| int(a[1]));
        for address in [
            0x0041_8900u32,
            0x0048_3710,
            0x004f_15a0,
            0x00ad_8f20,
            0x0068_a7d0,
            0x00ad_8830,
            0x00ad_88f0,
            0x00ad_8d10,
        ] {
            w.e.register(address, |_, _| Ret::default());
        }
        let _ = knobs.play_flag.get();
        (w, this, knobs)
    }

    fn handle_calls(log: &[(u32, Vec<u32>)]) -> Vec<u32> {
        log.iter()
            .map(|(address, _)| *address)
            .filter(|address| {
                [
                    0x0041_8900,
                    0x004f_15a0,
                    0x00ad_8f20,
                    0x0068_a7d0,
                    0x00ad_8830,
                    0x00ad_88f0,
                    0x00ad_8d10,
                    0x0041_a280,
                    0x0041_8980,
                ]
                .contains(address)
            })
            .collect()
    }

    #[test]
    fn a_creature_starts_its_sound_when_it_should_and_nothing_plays() {
        let (mut w, this, knobs) = sound_world();
        knobs.valid.set(false);
        w.e.call_log = Some(vec![]);
        w.e.call(0x008b_3230, &args![this, 0.5f32]);
        let log = w.e.call_log.take().unwrap();
        // Load the handle, create it, store it, follow, play, store it.
        assert_eq!(
            handle_calls(&log),
            vec![
                0x0041_8980,
                0x0041_8900,
                0x004f_15a0,
                0x0041_a280,
                0x00ad_8f20,
                0x0068_a7d0,
                0x00ad_8830,
                0x0041_a280
            ]
        );
        let flags = log
            .iter()
            .find(|(address, _)| *address == 0x00ad_7480)
            .unwrap();
        // (audio, handle out, file name, flags 0x10 | 2, sound)
        assert_eq!(flags.1[0], 0x5555);
        assert_eq!(flags.1[2..], [0x4444, 0x12, 0x3333]);
        let follow = log
            .iter()
            .find(|(address, _)| *address == 0x00ad_8f20)
            .unwrap();
        assert_eq!(follow.1[1], 0x1111);
    }

    #[test]
    fn a_valid_handle_is_only_played() {
        let (mut w, this, knobs) = sound_world();
        knobs.valid.set(true);
        w.e.call_log = Some(vec![]);
        w.e.call(0x008b_3230, &args![this, 0.5f32]);
        let log = w.e.call_log.take().unwrap();
        assert_eq!(
            handle_calls(&log),
            vec![0x0041_8980, 0x0068_a7d0, 0x00ad_8830, 0x0041_a280]
        );
    }

    #[test]
    fn a_playing_sound_is_left_alone_while_it_should_play() {
        let (mut w, this, knobs) = sound_world();
        knobs.playing.set(true);
        w.e.call_log = Some(vec![]);
        w.e.call(0x008b_3230, &args![this, 0.5f32]);
        let log = w.e.call_log.take().unwrap();
        assert_eq!(handle_calls(&log), vec![0x0041_8980]);
    }

    #[test]
    fn a_sound_that_should_not_play_is_stopped() {
        let (mut w, this, knobs) = sound_world();
        w.e.register(0x0049_38e0, |_, _| int(0));
        knobs.playing.set(true);
        w.e.call_log = Some(vec![]);
        w.e.call(0x008b_3230, &args![this, 0.5f32]);
        let log = w.e.call_log.take().unwrap();
        assert_eq!(
            handle_calls(&log),
            vec![0x0041_8980, 0x00ad_88f0, 0x0041_a280]
        );
        // Slot 0x22c answering true while playing stops, releases and stores it.
        let (mut w, this, knobs) = sound_world();
        knobs.playing.set(true);
        w.answer(this.addr(), 0x22c, 1);
        w.e.call_log = Some(vec![]);
        w.e.call(0x008b_3230, &args![this, 0.5f32]);
        let log = w.e.call_log.take().unwrap();
        assert_eq!(
            handle_calls(&log),
            vec![
                0x0041_8980,
                0x00ad_88f0,
                0x0041_a280,
                0x00ad_88f0,
                0x00ad_8d10,
                0x0041_a280
            ]
        );
    }

    #[test]
    fn creatures_need_a_sound_a_follow_target_and_the_right_form_kind() {
        let (mut w, this, _) = sound_world();
        w.e.register(0x0059_f3a0, |_, _| int(5));
        w.e.register(0x0041_a250, |_, _| panic!("no handle for other kinds"));
        w.e.call(0x008b_3230, &args![this, 0.5f32]);
        w.e.register(0x0059_f3a0, |_, _| int(7));
        w.e.register(0x005f_92d0, |_, _| int(0));
        w.e.call(0x008b_3230, &args![this, 0.5f32]);
        w.e.register(0x005f_92d0, |_, _| int(0x3333));
        w.answer(this.addr(), 0x1d0, 0);
        w.e.call(0x008b_3230, &args![this, 0.5f32]);
        w.answer(this.addr(), 0x21c, 0);
        w.e.call(0x008b_3230, &args![this, 0.5f32]);
    }

    // ---- 008b3820 .. 008b4cd0 ------------------------------------------

    type Calls = Vec<(u32, Vec<u32>)>;
    type SlotCalls = Rc<std::cell::RefCell<Vec<(u32, u32, Vec<u32>)>>>;
    type SlotAnswers = Rc<std::cell::RefCell<std::collections::HashMap<(u32, u32), u32>>>;

    /// An engine whose objects share one vtable: each of `slots` points at a
    /// double that logs `(object, slot, arguments)` and answers what was set
    /// with [`Scene::answer`] (default 0).
    struct Scene {
        e: Engine,
        calls: SlotCalls,
        answers: SlotAnswers,
        vtable: u32,
    }

    fn scene(slots: &[u32]) -> Scene {
        let (mut e, _) = engine();
        let calls: SlotCalls = Default::default();
        let answers: SlotAnswers = Default::default();
        let vtable = e.mem.alloc(0x800);
        for &slot in slots {
            let (calls, answers) = (calls.clone(), answers.clone());
            let target = 0x0f30_0000 + slot;
            e.mem.set_u32(vtable + slot, target);
            e.register_double(target, move |_, a| {
                calls.borrow_mut().push((a[0], slot, a[1..].to_vec()));
                int(answers.borrow().get(&(a[0], slot)).copied().unwrap_or(0))
            });
        }
        Scene {
            e,
            calls,
            answers,
            vtable,
        }
    }

    impl Scene {
        fn object(&mut self, size: u32) -> u32 {
            let object = self.e.mem.alloc(size);
            self.e.mem.set_u32(object, self.vtable);
            object
        }

        fn actor(&mut self) -> Ptr<Actor> {
            let actor = self.object(0x1b4);
            self.e.mem.set_u32(actor + 0xa4, self.vtable);
            Ptr::new(actor)
        }

        fn answer(&self, object: u32, slot: u32, value: u32) {
            self.answers.borrow_mut().insert((object, slot), value);
        }

        fn slot_calls(&self) -> Vec<(u32, u32, Vec<u32>)> {
            self.calls.borrow().clone()
        }
    }

    /// Runs `address` on an actor whose mover is a plain block; the
    /// callee `target` answers 0x1234. Checks that `target` receives the
    /// mover and `words` and that its answer is returned.
    fn assert_mover_thunk(address: u32, target: u32, words: &[u32]) {
        let (mut e, _) = engine();
        let actor = e.new_object::<Actor>();
        let mover = e.mem.alloc(0xa8);
        e.mem.set_u32(actor.addr() + 0x190, mover);
        e.register(target, |_, _| int(0x1234));
        e.call_log = Some(vec![]);
        let mut all = vec![actor.addr()];
        all.extend_from_slice(words);
        let result = e.call(address, &all).u32();
        let log = e.call_log.take().unwrap();
        let mut expected = vec![mover];
        expected.extend_from_slice(words);
        assert_eq!(log[1], (target, expected));
        assert_eq!(log.len(), 2);
        assert_eq!(result, 0x1234);
    }

    /// Like [`assert_mover_thunk`] for a thunk of virtual slot `slot` of the mover.
    fn assert_mover_slot_thunk(address: u32, slot: u32, words: &[u32]) {
        let mut s = scene(&[slot]);
        let actor = s.actor();
        let mover = s.object(0xa8);
        s.e.mem.set_u32(actor.addr() + 0x190, mover);
        s.answer(mover, slot, 0x4321);
        let mut all = vec![actor.addr()];
        all.extend_from_slice(words);
        assert_eq!(s.e.call(address, &all).u32(), 0x4321);
        assert_eq!(s.slot_calls(), vec![(mover, slot, words.to_vec())]);
    }

    #[test]
    fn get_current_pathing_request_asks_the_mover() {
        assert_mover_thunk(0x008b_3820, 0x0068_38b0, &[]);
    }

    #[test]
    fn get_current_pathfinding_goal_passes_its_argument() {
        assert_mover_thunk(0x008b_3840, 0x009d_e2b0, &[0x77]);
    }

    #[test]
    fn thunk_008b3860_passes_its_argument() {
        assert_mover_thunk(0x008b_3860, 0x009d_e330, &[0x78]);
    }

    #[test]
    fn build_request_follows_up_only_in_combat() {
        for combat in [0u32, 1] {
            let (mut e, _) = engine();
            let actor = e.new_object::<Actor>();
            e.register(0x009d_bc90, |_, _| int(0));
            e.register_double(ACTOR_IS_IN_COMBAT, move |_, _| int(combat));
            e.register(0x008a_d8f0, |_, _| int(0));
            e.call_log = Some(vec![]);
            e.call(
                0x008b_3880,
                &args![actor, 0x11u32, 0x22u32, 0x33u32, 0x44u32, 1.5f32, 0x66u32],
            );
            let log = e.call_log.take().unwrap();
            assert_eq!(
                log[1],
                (
                    0x009d_bc90,
                    vec![actor.addr(), 0x11, 0x22, 0x33, 0x44, 1.5f32.to_bits(), 0x66]
                )
            );
            assert_eq!(log[2].0, ACTOR_IS_IN_COMBAT);
            if combat == 1 {
                assert_eq!(log[3], (0x008a_d8f0, vec![0x11, 0]));
            } else {
                assert_eq!(log.len(), 3);
            }
        }
    }

    #[test]
    fn build_request_ov2_picks_cell_or_world_space() {
        // (place, expected first place, expected second place)
        for (place, first, second) in [
            (0u32, 0xce11u32, 0x3du32),
            (0x1001, 0x1001, 0),
            (0x1002, 0, 0x1002),
            (0x1003, 0, 0),
        ] {
            let (mut e, _) = engine();
            let actor = e.new_object::<Actor>();
            e.register(0x009d_bc90, |_, _| int(0));
            e.register(ACTOR_IS_IN_COMBAT, |_, _| int(0));
            e.register(0x008d_6f30, |_, _| int(0xce11));
            e.register(0x0057_5d70, |_, _| int(0x3d));
            e.register(0x0040_1170, |_, a| {
                int(match a[0] {
                    0x1001 => 0x39,
                    0x1002 => 0x41,
                    _ => 0x40,
                })
            });
            e.call_log = Some(vec![]);
            e.call(
                0x008b_38d0,
                &args![actor, 0x11u32, 0x22u32, place, 2.5f32, 0x66u32],
            );
            let log = e.call_log.take().unwrap();
            let request = log.iter().find(|c| c.0 == 0x009d_bc90).unwrap();
            assert_eq!(
                request.1,
                vec![
                    actor.addr(),
                    0x11,
                    0x22,
                    first,
                    second,
                    2.5f32.to_bits(),
                    0x66
                ]
            );
        }
    }

    #[test]
    fn is_path_valid_asks_the_mover() {
        assert_mover_thunk(0x008b_3960, 0x009d_e3e0, &[]);
    }

    #[test]
    fn thunk_008b3980_asks_the_mover() {
        assert_mover_thunk(0x008b_3980, 0x009d_cab0, &[]);
    }

    #[test]
    fn thunk_008b39a0_passes_both_arguments() {
        assert_mover_thunk(0x008b_39a0, 0x009d_e110, &[5, 6]);
    }

    #[test]
    fn get_failed_path_destination_asks_the_mover() {
        assert_mover_thunk(0x008b_39d0, 0x009d_e460, &[]);
    }

    #[test]
    fn set_move_mode_uses_slot_4() {
        assert_mover_slot_thunk(0x008b_39f0, 0x4, &[3]);
    }

    #[test]
    fn force_move_mode_uses_slot_c() {
        assert_mover_slot_thunk(0x008b_3a20, 0xc, &[3]);
    }

    #[test]
    fn clear_forced_move_mode_uses_slot_10() {
        assert_mover_slot_thunk(0x008b_3a50, 0x10, &[]);
    }

    #[test]
    fn clear_move_mode_uses_slot_8() {
        assert_mover_slot_thunk(0x008b_3a80, 0x8, &[9]);
    }

    #[test]
    fn stop_moving_asks_the_mover() {
        assert_mover_thunk(0x008b_3ab0, 0x009d_d0a0, &[]);
    }

    #[test]
    fn force_stop_moving_asks_the_mover() {
        assert_mover_thunk(0x008b_3ad0, 0x009d_d0c0, &[]);
    }

    #[test]
    fn mover_stop_flag_is_cleared() {
        let (mut e, _) = engine();
        let actor = e.new_object::<Actor>();
        let mover = e.mem.alloc(0xa8);
        e.mem.set_u8(mover + 0x72, 1);
        e.mem.set_u32(actor.addr() + 0x190, mover);
        e.call(0x008b_3af0, &args![actor]);
        assert_eq!(e.mem.u8(mover + 0x72), 0);
        e.mem.set_u8(mover + 0x72, 1);
        e.call(0x008b_3b10, &args![Ptr::<()>::new(mover)]);
        assert_eq!(e.mem.u8(mover + 0x72), 0);
    }

    #[test]
    fn vector_comes_from_the_actor_or_the_mover() {
        // Zero or negative: the actor's own vector.
        for value in [0.0f32, -2.0] {
            let (mut e, _) = engine();
            let actor = e.new_object::<Actor>();
            let source = e.mem.alloc(12);
            for (i, bits) in [1.0f32, 2.0, 3.0].iter().enumerate() {
                e.mem.set_f32(source + 4 * i as u32, *bits);
            }
            e.register_double(0x0043_6aa0, move |_, _| int(source));
            e.register(0x009d_cc80, |_, _| panic!("mover is not asked"));
            let out = e.mem.alloc(12);
            let result = e.call(0x008b_3b30, &args![actor, out, value]).u32();
            assert_eq!(result, out);
            assert_eq!(
                [e.mem.f32(out), e.mem.f32(out + 4), e.mem.f32(out + 8)],
                [1.0, 2.0, 3.0]
            );
        }
        // Positive: the mover fills it.
        let (mut e, _) = engine();
        let actor = e.new_object::<Actor>();
        let mover = e.mem.alloc(0xa8);
        e.mem.set_u32(actor.addr() + 0x190, mover);
        e.register(0x0043_6aa0, |_, _| panic!("actor is not asked"));
        e.register(0x009d_cc80, |_, _| int(0));
        let out = e.mem.alloc(12);
        e.call_log = Some(vec![]);
        let result = e.call(0x008b_3b30, &args![actor, out, 0.5f32]).u32();
        assert_eq!(result, out);
        let log = e.call_log.take().unwrap();
        assert_eq!(log[1], (0x009d_cc80, vec![mover, out, 0.5f32.to_bits()]));
    }

    #[test]
    fn thunk_008b3b90_asks_the_mover() {
        assert_mover_thunk(0x008b_3b90, 0x0097_f2f0, &[]);
    }

    #[test]
    fn is_pathing_complete_asks_the_mover() {
        assert_mover_thunk(0x008b_3bb0, 0x009d_cd40, &[]);
    }

    #[test]
    fn is_pathing_asks_the_mover() {
        assert_mover_thunk(0x008b_3bd0, 0x009d_cdb0, &[]);
    }

    #[test]
    fn mover_flags_are_bytes() {
        let (mut e, _) = engine();
        let actor = e.new_object::<Actor>();
        let mover = e.mem.alloc(0xa8);
        e.mem.set_u32(actor.addr() + 0x190, mover);
        e.mem.set_u8(mover + 0x72, 0x11);
        e.mem.set_u8(mover + 0x73, 0x22);
        e.mem.set_u8(mover + 0x74, 0x33);
        let mover_ptr = Ptr::<()>::new(mover);
        assert_eq!(e.call(0x008b_3bf0, &args![actor]).u8(), 0x33);
        assert_eq!(e.call(0x008b_3c10, &args![mover_ptr]).u8(), 0x33);
        assert_eq!(e.call(0x008b_3c70, &args![actor]).u8(), 0x11);
        assert_eq!(e.call(0x008b_3c90, &args![mover_ptr]).u8(), 0x11);
        assert_eq!(e.call(0x008b_3d10, &args![mover_ptr]).u8(), 0x22);
    }

    #[test]
    fn is_rotating_asks_the_mover() {
        assert_mover_thunk(0x008b_3c30, 0x009d_db50, &[]);
    }

    #[test]
    fn thunk_008b3c50_asks_the_mover() {
        assert_mover_thunk(0x008b_3c50, 0x009d_cb30, &[]);
    }

    #[test]
    fn thunk_008b3cb0_passes_its_argument() {
        assert_mover_thunk(0x008b_3cb0, 0x009d_e3a0, &[0x44]);
    }

    #[test]
    fn set_look_at_target_passes_the_three_floats() {
        assert_mover_thunk(
            0x008b_3cd0,
            0x009d_e160,
            &[1.0f32.to_bits(), 2.0f32.to_bits(), 3.0f32.to_bits()],
        );
    }

    #[test]
    fn clear_look_at_target_asks_the_mover() {
        assert_mover_thunk(0x008b_3d30, 0x009d_e230, &[]);
    }

    #[test]
    fn thunk_008b3d50_asks_the_mover() {
        assert_mover_thunk(0x008b_3d50, 0x0046_0140, &[]);
    }

    /// Doubles for the vector helpers of `008b3d70`: subtraction, scaling
    /// and normalisation are done on the actual floats.
    fn ray_engine() -> (Engine, Ptr<Actor>) {
        let (mut e, actor) = engine();
        for page in [0x0102_3000, 0x0101_6000, 0x0102_2000] {
            e.map(page, 0x1000);
        }
        e.set_global::<f32>(0x0102_31a0, 512.0);
        e.set_global::<f32>(0x0101_6248, 0.5);
        e.set_global::<f32>(0x0102_226c, 7.0);
        e.register(0x0043_fcd0, |_, _| int(0xa01d));
        e.register(0x0043_9ef0, |e, a| {
            for i in 0..3 {
                let value = e.mem.f32(a[0] + 4 * i) - e.mem.f32(a[2] + 4 * i);
                e.mem.set_f32(a[1] + 4 * i, value);
            }
            int(a[1])
        });
        e.register(0x0052_5340, |e, a| {
            let v: Vec<f32> = (0..3).map(|i| e.mem.f32(a[0] + 4 * i)).collect();
            let length = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
            for i in 0..3 {
                e.mem.set_f32(a[0] + 4 * i, v[i as usize] / length);
            }
            Ret::default()
        });
        e.register(0x0045_bb20, |e, a| {
            let factor = f32::from_bits(a[2]);
            for i in 0..3 {
                let value = e.mem.f32(a[0] + 4 * i) * factor;
                e.mem.set_f32(a[1] + 4 * i, value);
            }
            int(a[1])
        });
        (e, actor)
    }

    #[test]
    fn ray_hit_releases_through_slot_c() {
        let (mut e, actor) = ray_engine();
        let vtable = e.mem.alloc(0x40);
        e.mem.set_u32(vtable + 0xc, 0x0f31_000c);
        let hit = e.mem.alloc(0x10);
        e.mem.set_u32(hit, vtable);
        e.register(0x0f31_000c, |_, _| int(0x55));
        e.register_double(0x008c_fbc0, move |e, a| {
            // world, start (3), direction (3), length pointer, 0
            assert_eq!(a[0], 0xa01d);
            assert_eq!(f32::from_bits(a[1]), -253.0);
            assert_eq!(f32::from_bits(a[4]), 1.0);
            assert_eq!(e.mem.f32(a[7]), 512.0);
            assert_eq!(a[8], 0);
            int(hit)
        });
        e.register(0x0062_40d0, |_, _| panic!("no second test after a hit"));
        let result = e.call(
            0x008b_3d70,
            &args![actor, 0f32, 0f32, 0f32, 3.0f32, 0f32, 0f32],
        );
        assert_eq!(result.u32(), 0x55);
    }

    #[test]
    fn ray_without_hit_tries_the_second_test() {
        let (mut e, actor) = ray_engine();
        let vtable = e.mem.alloc(0x40);
        e.mem.set_u32(vtable + 0xc, 0x0f31_000c);
        let object = e.mem.alloc(0x10);
        e.mem.set_u32(object, vtable);
        e.register(0x0f31_000c, |_, _| int(0x66));
        e.register(0x008c_fbc0, |_, _| int(0));
        e.register(0x0062_40d0, |_, _| Ret::default());
        e.register(0x0068_15c0, |_, _| Ret::default());
        e.register(0x004a_3e00, |_, _| Ret::default());
        e.register(0x008a_50f0, |_, _| Ret::default());
        e.register(0x008a_50d0, |_, a| {
            assert_eq!(f32::from_bits(a[1]), 7.0);
            Ret::default()
        });
        e.register(0x00c8_06b0, |_, a| {
            assert_eq!(a[0], 0xa01d);
            int(0x1111)
        });
        e.register(0x004a_e750, |_, a| {
            assert_eq!(a[0], 0x1111);
            int(0x2222)
        });
        e.register(0x004b_5a20, |_, a| {
            assert_eq!(a[0], 0x2222);
            int(0x3333)
        });
        e.register_double(0x0044_ddc0, move |_, a| {
            assert_eq!(a[0], 0x3333);
            int(object)
        });
        e.call_log = Some(vec![]);
        let result = e.call(
            0x008b_3d70,
            &args![actor, 0f32, 0f32, 0f32, 3.0f32, 0f32, 0f32],
        );
        assert_eq!(result.u32(), 0x66);
        let log = e.call_log.take().unwrap();
        let order: Vec<u32> = log.iter().map(|c| c.0).collect();
        assert_eq!(
            &order[order.len() - 10..],
            [
                0x0062_40d0,
                0x0068_15c0,
                0x004a_3e00,
                0x008a_50f0,
                0x008a_50d0,
                0x00c8_06b0,
                0x004a_e750,
                0x004b_5a20,
                0x0044_ddc0,
                0x0f31_000c
            ]
        );
    }

    #[test]
    fn ray_with_no_object_returns_zero() {
        let (mut e, actor) = ray_engine();
        e.register(0x008c_fbc0, |_, _| int(0));
        e.register(0x0062_40d0, |_, _| Ret::default());
        e.register(0x0068_15c0, |_, _| Ret::default());
        e.register(0x004a_3e00, |_, _| Ret::default());
        e.register(0x008a_50f0, |_, _| Ret::default());
        e.register(0x008a_50d0, |_, _| Ret::default());
        e.register(0x00c8_06b0, |_, _| int(0));
        let result = e.call(
            0x008b_3d70,
            &args![actor, 0f32, 0f32, 0f32, 3.0f32, 0f32, 0f32],
        );
        assert_eq!(result.u32(), 0);
    }

    #[test]
    fn body_part_index_walks_up_the_parents() {
        let mut s = scene(&[0x1d0, 0x1e8, 0x190, 0x6d0]);
        let actor = s.actor();
        let process = s.object(0x800);
        s.e.mem.set_u32(actor.addr() + 0x68, process);
        let stop = 0x5555;
        s.answer(actor.addr(), 0x1d0, stop);
        s.answer(actor.addr(), 0x1e8, 0x9);
        s.answer(process, 0x190, 0x200);
        // Parent chain: 100 -> 150 -> 0.
        s.e.register(0x0096_11e0, |_, a| {
            int(match a[0] {
                100 => 150,
                _ => 0,
            })
        });
        // Index 7 of the process holds node 150.
        s.e.register_double(0x0f30_06d0, move |_, a| {
            int(if a[1] == 7 { 150 } else { 0 })
        });
        assert_eq!(s.e.call(0x008b_3ef0, &args![actor, 100u32]).i32(), 7);
        // A parent equal to the process's answer for the actor's slot 0x1E8 gives 0xE.
        s.answer(process, 0x190, 150);
        assert_eq!(s.e.call(0x008b_3ef0, &args![actor, 100u32]).i32(), 0xe);
        // Reaching the actor's own object, an empty node, or no process: -1.
        assert_eq!(s.e.call(0x008b_3ef0, &args![actor, stop]).i32(), -1);
        assert_eq!(s.e.call(0x008b_3ef0, &args![actor, 0u32]).i32(), -1);
        s.e.mem.set_u32(actor.addr() + 0x68, 0);
        assert_eq!(s.e.call(0x008b_3ef0, &args![actor, 100u32]).i32(), -1);
    }

    #[test]
    fn transform_inverse_inverts_scale_and_translation() {
        let (mut e, _) = engine();
        let this = e.mem.alloc(0x34);
        let dest = e.mem.alloc(0x34);
        e.mem.set_f32(this + 0x30, 4.0);
        let negated = e.mem.alloc(12);
        let rotated = e.mem.alloc(12);
        let scaled = e.mem.alloc(12);
        for (i, v) in [7.0f32, 8.0, 9.0].iter().enumerate() {
            e.mem.set_f32(scaled + 4 * i as u32, *v);
        }
        e.register(0x004b_46a0, |_, _| Ret::default());
        e.register_double(0x004a_0bd0, move |_, a| {
            assert_eq!(a[0], this + 0x24);
            int(negated)
        });
        e.register_double(0x004b_4500, move |_, a| {
            assert_eq!((a[0], a[2]), (dest, negated));
            int(rotated)
        });
        e.register_double(0x004a_3760, move |_, a| {
            assert_eq!(f32::from_bits(a[1]), 0.25);
            assert_eq!(a[2], rotated);
            int(scaled)
        });
        e.call_log = Some(vec![]);
        e.call(
            0x008b_42e0,
            &args![Ptr::<()>::new(this), Ptr::<()>::new(dest)],
        );
        let log = e.call_log.take().unwrap();
        assert_eq!(log[1], (0x004b_46a0, vec![this, dest]));
        assert_eq!(e.mem.f32(dest + 0x30), 0.25);
        assert_eq!(
            [
                e.mem.f32(dest + 0x24),
                e.mem.f32(dest + 0x28),
                e.mem.f32(dest + 0x2c)
            ],
            [7.0, 8.0, 9.0]
        );
    }

    #[test]
    fn body_part_flags_and_byte() {
        let (mut e, _) = engine();
        let part = e.mem.alloc(0x80);
        let part_ptr = Ptr::<()>::new(part);
        assert!(!e.call(0x008b_4360, &args![part_ptr]).bool());
        assert!(!e.call(0x008b_4cd0, &args![part_ptr]).bool());
        e.mem.set_u8(part + 0x60, 0x08);
        assert!(e.call(0x008b_4360, &args![part_ptr]).bool());
        assert!(!e.call(0x008b_4cd0, &args![part_ptr]).bool());
        e.mem.set_u8(part + 0x60, 0x40);
        assert!(!e.call(0x008b_4360, &args![part_ptr]).bool());
        assert!(e.call(0x008b_4cd0, &args![part_ptr]).bool());
        e.mem.set_u8(part + 0x66, 0x5a);
        assert_eq!(e.call(0x008b_4380, &args![part_ptr]).u8(), 0x5a);
    }

    /// The world of the `008b3fe0` tests: every callee is a double.
    struct Spawn {
        s: Scene,
        actor: Ptr<Actor>,
        part: u32,
        reference: u32,
        impact: u32,
        clone: u32,
    }

    fn spawn_world() -> Spawn {
        let mut s = scene(&[0x180, 0x1d0, 0x14]);
        let actor = s.actor();
        let form = s.object(0x20);
        let owner = s.object(0x100);
        let path_owner = s.object(0x20);
        s.answer(form, 0x180, 0xb0d1);
        s.answer(actor.addr(), 0x1d0, owner);
        s.answer(path_owner, 0x14, 0x9a7);
        let part = s.e.mem.alloc(0x80);
        s.e.mem.set_u8(part + 0x60, 0x08);
        s.e.mem.set_u8(part + 0x66, 3);
        let reference = s.e.mem.alloc(0x100);
        let impact = s.e.mem.alloc(0x40);
        for i in 0..9 {
            s.e.mem.set_u32(reference + 0x68 + 4 * i, 0x100 + i);
        }
        for i in 0..3 {
            s.e.mem.set_u32(impact + 0x24 + 4 * i, 0x200 + i);
        }
        let e = &mut s.e;
        e.map(0x011c_3000, 0x1000);
        e.set_global::<u32>(MODEL_LOADER, 0xa0ad);
        e.register_double(0x0041_81e0, move |_, _| int(form));
        e.register(0x008d_6f30, |_, _| int(0xce11));
        e.register_double(0x005e_50f0, move |_, a| {
            assert_eq!(a[0], 0xb0d1);
            int(part)
        });
        e.register(0x005e_3fa0, |_, _| int(0x77));
        e.register(0x0046_1130, |_, a| int(a[0] + 0x68));
        e.register(0x009a_c9c0, |_, _| Ret::default());
        e.register(ACTOR_GET_PROCESS, |_, _| int(0xb1));
        e.register(0x0056_7400, |_, _| float(2.0));
        e.register(0x009b_0bd0, |_, _| float(3.0));
        e.register(0x004f_a6f0, |_, _| Ret::default());
        e.register_double(0x005d_8a70, move |_, _| int(path_owner));
        e.register(0x0048_cee0, |_, _| int(1));
        e.register(0x0044_7080, |_, _| int(0x30d1));
        e.register(0x004a_d050, |_, _| Ret::default());
        e.register(0x00a5_d2c0, |_, _| int(0xc10e));
        e.register(0x0047_6a80, |_, _| Ret::default());
        e.register(0x004b_46a0, |_, _| Ret::default());
        e.register(0x004a_0bd0, |_, _| int(0x1000));
        e.register(0x004b_4500, |_, _| int(0x2000));
        let scaled_buffer = e.mem.alloc(12);
        e.register_double(0x004a_3760, move |_, _| int(scaled_buffer));
        let composed = e.mem.alloc(0x34);
        for i in 0..13 {
            e.mem.set_u32(composed + 4 * i, 0x300 + i);
        }
        e.register_double(0x0062_c250, move |_, _| int(composed));
        e.register(0x0044_0460, |_, _| Ret::default());
        e.register(0x0043_fa80, |_, _| Ret::default());
        e.register(0x009b_88c0, |_, _| float(0.5));
        e.register(0x0044_0490, |_, _| Ret::default());
        e.register(0x004a_d270, |_, _| Ret::default());
        e.register(0x0045_a5e0, |_, _| Ret::default());
        Spawn {
            s,
            actor,
            part,
            reference,
            impact,
            clone: 0xc10e,
        }
    }

    #[test]
    fn spawn_part_model_runs_the_whole_chain() {
        let mut w = spawn_world();
        let e = &mut w.s.e;
        e.call_log = Some(vec![]);
        let result = e
            .call(
                0x008b_3fe0,
                &args![w.actor, 5u32, w.reference, w.impact, true],
            )
            .u32();
        assert_eq!(result, w.clone);
        let log = e.call_log.take().unwrap();
        let order: Vec<u32> = log.iter().map(|c| c.0).collect();
        assert_eq!(
            order,
            vec![
                0x008b_3fe0,
                0x0041_81e0,
                0x0f30_0180,
                0x008d_6f30,
                0x005e_50f0,
                0x005e_3fa0,
                0x0046_1130,
                0x008d_6f30,
                0x005e_3fa0,
                0x009a_c9c0,
                ACTOR_GET_PROCESS,
                0x0056_7400,
                0x009b_0bd0,
                0x008d_6f30,
                ACTOR_GET_PROCESS,
                0x004f_a6f0,
                0x005d_8a70,
                0x0048_cee0,
                0x005d_8a70,
                0x0f30_0014,
                0x0044_7080,
                0x004a_d050,
                0x00a5_d2c0,
                0x0047_6a80,
                0x0047_6a80,
                0x0f30_01d0,
                0x0046_1130,
                0x004b_46a0,
                0x004a_0bd0,
                0x004b_4500,
                0x004a_3760,
                0x0046_1130,
                0x0062_c250,
                0x0044_0460,
                0x0043_fa80,
                0x009b_88c0,
                0x0044_0490,
                0x004a_d270,
                0x005d_8a70,
                0x0f30_0014,
                0x0045_a5e0,
            ]
        );
        let explosion = &log[order.iter().position(|a| *a == 0x009a_c9c0).unwrap()].1;
        let mut expected = vec![0x77, 0, 0, 0xce11, 0x200, 0x201, 0x202];
        expected.extend((0..9).map(|i| 0x100 + i));
        assert_eq!(explosion, &expected);
        let placed = &log[order.iter().position(|a| *a == 0x004f_a6f0).unwrap()].1;
        assert_eq!(
            placed,
            &vec![0xb1, 0xce11, w.impact + 0x24, 3, 6.0f32.to_bits()]
        );
        let clone_scale = &log[order.iter().position(|a| *a == 0x0044_0490).unwrap()].1;
        assert_eq!(clone_scale, &vec![w.clone, 0.5f32.to_bits()]);
        let _ = w.part;
    }

    #[test]
    fn spawn_part_model_stops_early() {
        // No effects and no reference: the model is not touched.
        let mut w = spawn_world();
        let e = &mut w.s.e;
        e.register(0x0044_7080, |_, _| panic!("no model without a reference"));
        e.call_log = Some(vec![]);
        let result = e
            .call(0x008b_3fe0, &args![w.actor, 5u32, 0u32, w.impact, false])
            .u32();
        assert_eq!(result, 0);
        let order: Vec<u32> = e.call_log.take().unwrap().iter().map(|c| c.0).collect();
        assert_eq!(
            order,
            vec![
                0x008b_3fe0,
                0x0041_81e0,
                0x0f30_0180,
                0x008d_6f30,
                0x005e_50f0,
                0x005d_8a70,
                0x0048_cee0
            ]
        );
        // A part without flag 0x08 stops before anything else.
        e.mem.set_u8(w.part + 0x60, 0);
        e.call_log = Some(vec![]);
        e.call(
            0x008b_3fe0,
            &args![w.actor, 5u32, w.reference, w.impact, true],
        );
        assert_eq!(e.call_log.take().unwrap().len(), 5);
        // No cell: nothing after the body part data.
        e.register(0x008d_6f30, |_, _| int(0));
        e.call_log = Some(vec![]);
        e.call(
            0x008b_3fe0,
            &args![w.actor, 5u32, w.reference, w.impact, true],
        );
        assert_eq!(e.call_log.take().unwrap().len(), 4);
    }

    /// The world of the `008b43a0` tests.
    struct Dismember {
        s: Scene,
        actor: Ptr<Actor>,
        hit: u32,
        parts: Rc<std::cell::RefCell<std::collections::HashMap<u32, u32>>>,
    }

    /// Settings (by the object `0043d4d0` is given) and their values.
    const DISMEMBER_SETTINGS: [(u32, u32); 7] = [
        (DISMEMBER_LIMIT_SETTING, 10),
        (DISMEMBER_SETTING_LIMB_CHANCE, 50),
        (DISMEMBER_SETTING_FIRST_PART, 40),
        (DISMEMBER_SETTING_OTHER_PART, 41),
        (DISMEMBER_SETTING_ROLL, 60),
        (DISMEMBER_SETTING_ROLLED_PART, 42),
        (DISMEMBER_BLOCK_SETTING, 0),
    ];

    fn dismember_world() -> Dismember {
        let mut s = scene(&[0x218, 0x180, 0x220, 8, 0x1d0]);
        let actor = s.actor();
        let form = s.object(0x20);
        s.answer(form, 0x180, 0xb0d1);
        let hit = s.e.mem.alloc(0x20);
        let e = &mut s.e;
        e.map(0x011e_0000, 0x1000);
        e.set_global::<u32>(DISMEMBER_COUNTER, 0);
        let mut pointers = std::collections::HashMap::new();
        for (setting, value) in DISMEMBER_SETTINGS {
            let cell = e.mem.alloc(4);
            e.mem.set_u32(cell, value);
            pointers.insert(setting, cell);
        }
        e.register_double(0x0043_d4d0, move |_, a| int(pointers[&a[0]]));
        e.register_double(0x0041_81e0, move |_, _| int(form));
        e.register(0x00ec_43fb, |_, _| int(0));
        e.register(0x005f_bf20, |_, _| int(0));
        e.register(0x0040_8d60, |_, a| int(a[0]));
        let parts: Rc<std::cell::RefCell<std::collections::HashMap<u32, u32>>> = Default::default();
        let by_key = parts.clone();
        e.register_double(0x005e_50f0, move |_, a| {
            int(by_key.borrow().get(&a[1]).copied().unwrap_or(0))
        });
        let by_value = parts.clone();
        e.register_double(0x005e_5130, move |_, a| {
            int(by_value
                .borrow()
                .get(&(0x1000 + a[1]))
                .copied()
                .unwrap_or(0))
        });
        // Part kind: the byte at +0x61 is the actor value, the dword at +0x70 its body part.
        e.register(0x005e_5190, |e, a| int(e.mem.u8(a[0] + 0x61) as u32));
        e.register(0x005e_5300, |e, a| int(e.mem.u32(a[0] + 0x70)));
        e.register(0x008b_4cf0, |e, a| int(e.mem.u8(a[0] + 0x62) as u32));
        e.register(0x0058_cba0, |_, _| int(0));
        e.register(0x0048_7f50, |_, _| int(250));
        e.register(0x0066_ec10, |_, a| {
            assert_eq!(a[0], 4);
            int(10 + a[1])
        });
        e.register(0x0040_7e00, |_, _| Ret::default());
        e.register(ACTOR_DISMEMBER, |_, _| Ret::default());
        Dismember {
            s,
            actor,
            hit,
            parts,
        }
    }

    impl Dismember {
        /// A body part with the given actor value byte, part number and 4cf0 byte.
        fn part(&mut self, value: u8, number: u32, severable: u8, flags: u8) -> u32 {
            let part = self.s.e.mem.alloc(0x80);
            self.s.e.mem.set_u8(part + 0x60, flags);
            self.s.e.mem.set_u8(part + 0x61, value);
            self.s.e.mem.set_u8(part + 0x62, severable);
            self.s.e.mem.set_u32(part + 0x70, number);
            part
        }

        fn run(&mut self) -> Calls {
            self.s.e.call_log = Some(vec![]);
            let actor = self.actor;
            let hit = self.hit;
            self.s
                .e
                .call(0x008b_43a0, &args![actor, hit, 0xaau32, 0xbbu32]);
            self.s.e.call_log.take().unwrap()
        }
    }

    fn dismember_calls(log: &Calls) -> Calls {
        log.iter()
            .filter(|c| c.0 == ACTOR_DISMEMBER || c.0 == 0x0040_7e00)
            .cloned()
            .collect()
    }

    #[test]
    fn dismember_stops_early_and_counts() {
        let mut w = dismember_world();
        // A creature that cannot be dismembered while the setting says so.
        w.s.answer(w.actor.addr(), 0x218, 1);
        let blocked = w.s.e.mem.alloc(4);
        w.s.e.mem.set_u8(blocked, 1);
        w.s.e.register_double(0x0040_8d60, move |_, _| int(blocked));
        let log = w.run();
        assert_eq!(log.last().unwrap().0, 0x0040_8d60);
        assert_eq!(w.s.e.global::<u32>(DISMEMBER_COUNTER), 0);
        // No body part data.
        let mut w = dismember_world();
        let form = w.s.object(0x20);
        w.s.e.register_double(0x0041_81e0, move |_, _| int(form));
        let log = w.run();
        assert_eq!(log.last().unwrap().0, 0x0f30_0180);
        // Without a hit.
        let mut w = dismember_world();
        w.hit = 0;
        let log = w.run();
        assert_eq!(log.last().unwrap().0, 0x0f30_0180);
        // The counter has reached the limit.
        let mut w = dismember_world();
        w.s.e.set_global::<u32>(DISMEMBER_COUNTER, 10);
        let log = w.run();
        assert!(dismember_calls(&log).is_empty());
        assert_eq!(w.s.e.global::<u32>(DISMEMBER_COUNTER), 10);
        // Otherwise it counts up.
        let mut w = dismember_world();
        w.run();
        assert_eq!(w.s.e.global::<u32>(DISMEMBER_COUNTER), 1);
    }

    #[test]
    fn dismember_without_candidates_dismembers_nothing_specific() {
        let mut w = dismember_world();
        let log = w.run();
        assert_eq!(
            dismember_calls(&log),
            vec![(
                ACTOR_DISMEMBER,
                vec![
                    w.actor.addr(),
                    w.hit,
                    u32::MAX,
                    u32::MAX,
                    0xaa,
                    0xbb,
                    0,
                    0,
                    0
                ]
            )]
        );
    }

    #[test]
    fn dismember_single_part_of_the_hit() {
        let mut w = dismember_world();
        let part = w.part(9, 0x33, 0, 0);
        w.parts.borrow_mut().insert(3, part);
        w.s.e.mem.set_u32(w.hit + 0x10, 3);
        let log = w.run();
        assert_eq!(
            dismember_calls(&log),
            vec![(
                ACTOR_DISMEMBER,
                vec![w.actor.addr(), w.hit, 9, 3, 0xaa, 0xbb, 0, 0, 0]
            )]
        );
        // The attacker's value already above zero: nothing is added.
        w.s.answer(w.actor.addr() + 0xa4, 8, 1);
        let log = w.run();
        assert_eq!(
            dismember_calls(&log)[0].1[2..4],
            [u32::MAX, u32::MAX],
            "no candidate, so the general call"
        );
    }

    #[test]
    fn dismember_with_a_strong_attacker_picks_the_hit_part_and_another() {
        let mut w = dismember_world();
        let attacker = w.s.object(0x100);
        w.s.answer(attacker + 0xa4, 8, 5);
        // Slot 8 is also the actor's value lookup; the attacker's and the
        // actor's answers are keyed by object.
        w.s.e.mem.set_u32(attacker + 0xa4, w.s.vtable);
        w.s.e.mem.set_u32(w.hit, attacker);
        // The hit's part: not severable; actor value 5.
        let hit_part = w.part(5, 0x51, 0, 0);
        w.parts.borrow_mut().insert(3, hit_part);
        w.s.e.mem.set_u32(w.hit + 0x10, 3);
        // Another part for the actor value 11 (the second index).
        let other = w.part(0, 0x52, 0, 0);
        w.parts.borrow_mut().insert(0x1000 + 11, other);
        let log = w.run();
        assert_eq!(
            dismember_calls(&log),
            vec![
                (
                    ACTOR_DISMEMBER,
                    vec![w.actor.addr(), w.hit, 5, 3, 0xaa, 0xbb, 100, 40, 0]
                ),
                (
                    ACTOR_DISMEMBER,
                    vec![w.actor.addr(), w.hit, 11, 0x52, 0xaa, 0xbb, 100, 41, 0]
                ),
            ]
        );
    }

    #[test]
    fn dismember_head_roll_sets_the_flag_around_the_call() {
        let mut w = dismember_world();
        let attacker = w.s.object(0x100);
        w.s.answer(attacker + 0xa4, 8, 5);
        let vtable = w.s.vtable;
        w.s.e.mem.set_u32(attacker + 0xa4, vtable);
        w.s.e.mem.set_u32(w.hit, attacker);
        // The hit's part is the head (value 0x1a) and severable (flags 0x08 | 0x40, byte >= 100).
        let head = w.part(0x1a, 0x61, 100, 0x48);
        w.parts.borrow_mut().insert(3, head);
        w.s.e.mem.set_u32(w.hit + 0x10, 3);
        let log = w.run();
        assert_eq!(
            dismember_calls(&log),
            vec![
                (0x0040_7e00, vec![w.hit, 0x40, 1]),
                (
                    ACTOR_DISMEMBER,
                    vec![w.actor.addr(), w.hit, 0x1a, 0x61, 0xaa, 0xbb, 100, 100, 1]
                ),
                (0x0040_7e00, vec![w.hit, 0x40, 0]),
            ]
        );
    }

    #[test]
    fn dismember_from_a_part_list() {
        let mut w = dismember_world();
        let owner = w.s.object(0x40);
        w.s.answer(owner, 0x220, 1);
        w.s.e.mem.set_u32(w.hit + 8, owner);
        let part = w.part(7, 0x71, 0, 0);
        w.parts.borrow_mut().insert(0x1234, part);
        let entry = w.s.e.mem.alloc(4);
        w.s.e.mem.set_u32(entry, 0x1234);
        let slot = w.s.e.mem.alloc(4);
        w.s.e.mem.set_u32(slot, entry);
        w.s.e.register(0x009b_1720, |_, _| int(0xa1));
        w.s.e.register(0x0082_56d0, |_, _| int(0));
        w.s.e.register_double(0x0068_15c0, move |_, _| int(slot));
        w.s.e.register(0x0072_6070, |_, _| int(0));
        let log = w.run();
        assert_eq!(
            dismember_calls(&log),
            vec![(
                ACTOR_DISMEMBER,
                vec![w.actor.addr(), w.hit, 7, 0x1234, 0xaa, 0xbb, 0, 0, 0]
            )]
        );
    }
}
