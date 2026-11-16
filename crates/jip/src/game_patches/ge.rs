//! Group GE's game patches (step 3.9: actors: movement, life, NPC perks): its handlers, wraps and slot routines. Only group
//! GE's PR edits this file (and its own tests file beside it).
//!
//! **G63, actor flags on load** (`InitGamePatches`, `internal/patches_game.h` (JIP LN NVSE):
//! `SAFE_WRITE_BUF(0x8AB6D9, ...)`, 26 bytes). The actor's load-game routine (`008ab3a0`, also
//! reached from `008d33f0` and the creature's `008d4ff0`) reads the flag word `+0x140` from the
//! save and then calls `008c1ac0(0)` and `008c1b50(0)`, which clear its bits 0x80000000 (the
//! runtime "essential" bit) and 0x40000000. JIP's bytes in place of the two calls:
//! `mov ecx, [ebp-0xAC]` (the actor); `mov word [ecx+0x105], 0` (JIP's flag bytes
//! `jipActorFlags1`/`jipActorFlags2`: combat disabled, forced detection values and the rest its
//! commands set); `and byte [ecx+0x143], 0xBF` (bit 0x40000000 only); `nop`. So the essential bit a
//! save holds survives the load for an NPC, and JIP's per-actor flags start clear on every load.
//! Not for a creature: the creature's load `008d4ff0` sets the whole flag word to 0 right after
//! this routine; G04 (`0x8D5003`, below) skips that with the fixes row on.
//!
//! Two kinds in one patch: the reset of JIP's own bytes answers JIP's commands (in whenever JIP
//! answers "installed", every preset; nothing of the game reads those bytes), while keeping the
//! essential bit changes the game, so it is a fix (with the row `NV_JIP_GAME_PATCHES` on). With
//! the row off the handler clears JIP's bytes and lets the exe's two calls run: a script's
//! essential flag is still lost on load, a mix real JIP never produces (its bytes always replace
//! the calls), kept so the row off leaves the game's own behaviour as on main.
//!
//! **The other rows** (each handler below names its JIP source): G04 a creature keeps its loaded
//! flag word; G22 the "AI Z fix" (three direction flattenings removed); G23 `SetLevel`'s
//! experience of the level itself; G28 Agility clamped for reload and equip speeds; H11 the
//! player's ragdoll not built twice on one root node; O13 `bVoiceModulationFix` (switch 13);
//! these are fixes. H18 (a teammate `ToggleTeammateKillable` marked is not essential) is JIP's
//! own. O13's seams at `0x578E16` and `0x7974CC` are in groups GH2's and GB's files.

use super::Area;
use crate::game::{JipActorFlags, TESForm, TESObjectREFR};
use engine_core::extend::{jip_patches, Owner, SiteSlot};
use engine_core::layout;
use engine_core::prelude::*;
use std::sync::atomic::{AtomicU32, Ordering};

layout! {
    /// `Actor` (0x1B4 bytes on PC), the field G63 writes (kept here, in the group's own file).
    pub struct ActorFlags: 0x1B4 {
        /// The actor's flag word (JIP LN NVSE's `actorFlags`, `// 140`: "0x80000000 -
        /// IsEssential"); `008c1ac0`/`008c1b50` set and clear its bits 0x80000000 and 0x40000000.
        0x140 actorFlags: u32,
    }
}

/// G63's seam: the load routine's two flag calls (words: the actor).
pub const ACTOR_FLAGS_LOADED: SiteSlot = SiteSlot::new(Owner::Jip, 0x008a_b6d9);
/// H18's seam: the essential test's player check for a teammate (words: the actor).
pub const TEAMMATE_ESSENTIAL: SiteSlot = SiteSlot::new(Owner::Jip, 0x0087_f427);
/// G04's seam: the creature's load routine after `008ab3a0`, before its flag word is cleared
/// (words: the creature).
pub const CREATURE_FLAGS_KEPT: SiteSlot = SiteSlot::new(Owner::Jip, 0x008d_5003);
/// G22's seams: the three flattenings of a direction (words: the vector, its z at `+8`).
pub const PATH_START_HEIGHT: SiteSlot = SiteSlot::new(Owner::Jip, 0x006b_10a3);
pub const MOVE_HEIGHT: SiteSlot = SiteSlot::new(Owner::Jip, 0x0088_1a9c);
pub const AVOID_HEIGHT: SiteSlot = SiteSlot::new(Owner::Jip, 0x009e_6a82);
/// G23's seam: `SetLevel`'s required-experience level (words: the level).
pub const SET_LEVEL_EXPERIENCE: SiteSlot = SiteSlot::new(Owner::Jip, 0x008d_535f);
/// G28's seams: the Agility read for the reload and the equip speed (words: the actor's value
/// owner, `actor + 0xA4`).
pub const RELOAD_AGILITY: SiteSlot = SiteSlot::new(Owner::Jip, 0x008c_17d9);
pub const EQUIP_AGILITY: SiteSlot = SiteSlot::new(Owner::Jip, 0x008c_1959);
/// H11's seam: the ragdoll build at Havok init (words: the actor, the root node).
pub const RAGDOLL_BUILD: SiteSlot = SiteSlot::new(Owner::Jip, 0x0093_1384);
/// O13's seams (switch `bVoiceModulationFix`): the dialogue sound's flags (words: the object;
/// answer: the flags) and the two other sound flag words (in group GH2's `fn_00578a80` and group
/// GB's `map_menu_do_click`; words: the flags word the exe passes; answer: that word with
/// 0x80000, JIP's byte 8 at `+2` of the immediate).
pub const VOICE_FLAGS: SiteSlot = SiteSlot::new(Owner::Jip, 0x0093_4ac8);
pub const REF_SOUND_FLAGS: SiteSlot = SiteSlot::new(Owner::Jip, 0x0057_8e16);
pub const MAP_SOUND_FLAGS: SiteSlot = SiteSlot::new(Owner::Jip, 0x0079_74cc);

/// The group's rows and installers.
pub const AREA: Area = Area {
    name: "GE",
    fix_sites: &[
        CREATURE_FLAGS_KEPT.addr(),
        PATH_START_HEIGHT.addr(),
        MOVE_HEIGHT.addr(),
        AVOID_HEIGHT.addr(),
        SET_LEVEL_EXPERIENCE.addr(),
        RELOAD_AGILITY.addr(),
        EQUIP_AGILITY.addr(),
        RAGDOLL_BUILD.addr(),
        VOICE_FLAGS.addr(),
        REF_SOUND_FLAGS.addr(),
        MAP_SOUND_FLAGS.addr(),
    ],
    fix_wraps: &[],
    install_fixes,
    jip_sites: &[ACTOR_FLAGS_LOADED.addr(), TEAMMATE_ESSENTIAL.addr()],
    jip_wraps: &[],
    install_jip,
};

fn install_jip(e: &mut Engine) {
    e.on_site(ACTOR_FLAGS_LOADED, actor_flags_loaded);
    e.on_site(TEAMMATE_ESSENTIAL, teammate_essential);
}

fn install_fixes(e: &mut Engine) {
    // Made before the threads start, so every thread's engine shares it.
    e.ext_free::<LastPlayerRoot>();
    e.on_site(CREATURE_FLAGS_KEPT, replaced);
    e.on_site(PATH_START_HEIGHT, replaced);
    e.on_site(MOVE_HEIGHT, replaced);
    e.on_site(AVOID_HEIGHT, replaced);
    e.on_site(SET_LEVEL_EXPERIENCE, set_level_experience);
    e.on_site(RELOAD_AGILITY, capped_agility);
    e.on_site(EQUIP_AGILITY, capped_agility);
    e.on_site(RAGDOLL_BUILD, ragdoll_build);
    e.on_site(VOICE_FLAGS, voice_flags);
    e.on_site(REF_SOUND_FLAGS, voice_flag_added);
    e.on_site(MAP_SOUND_FLAGS, voice_flag_added);
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_game.h: InitGamePatches
// (SAFE_WRITE_BUF(0x8AB6D9, "\x8B\x8D\x54\xFF\xFF\xFF\x66\xC7\x81\x05\x01...")).
/// The load routine's flag calls (`0x8AB6D9`: the actor, the routine's `this`, never 0): JIP's two
/// flag bytes cleared; with the fixes, bit 0x40000000 of the flag word cleared and the calls
/// replaced (answered), else the calls run (nothing answered).
fn actor_flags_loaded(e: &mut Engine, a: &[u32]) -> Option<u32> {
    let actor = Ptr::<JipActorFlags>::new(a[0]);
    e.set(actor, JipActorFlags::jipActorFlags1, 0);
    e.set(actor, JipActorFlags::jipActorFlags2, 0);
    if e.jip_patches() & jip_patches::FIXES == 0 {
        return None;
    }
    // JIP's `and byte [ecx+0x143], 0xBF`: the word's top byte, the other bytes as they are.
    let actor = Ptr::<ActorFlags>::new(a[0]);
    let flags = e.get(actor, ActorFlags::actorFlags);
    e.set(actor, ActorFlags::actorFlags, flags & !0x4000_0000);
    Some(0)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/hooks.h: IsActorEssentialHook.
/// The essential test's player check for a teammate (`0x87F427`: the actor, never 0): a
/// teammate with JIP's `kHookActorFlag2_TeammateKillable` (bit 2, the value 4, of `jipActorFlags2`) answers 1,
/// as if the player's test were true (not essential by that test); else nothing, and the exe's
/// test runs. JIP reads the test's field itself (`cmp byte [player+0x7BC], 0`, where the exe's
/// `004d1360` compares the dword with 1): the same for the field's values 0 and 1.
fn teammate_essential(e: &mut Engine, a: &[u32]) -> Option<u32> {
    let flags = e.get(
        Ptr::<JipActorFlags>::new(a[0]),
        JipActorFlags::jipActorFlags2,
    );
    (flags & TEAMMATE_KILLABLE != 0).then_some(1)
}

/// `kHookActorFlag2_TeammateKillable` (`internal/hooks.h` (JIP LN NVSE)).
pub const TEAMMATE_KILLABLE: u8 = 1 << 2;

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_game.h: InitGamePatches
// (SafeWrite32(0x8D5003, 0x0004C2C9); SAFE_WRITE_BUF(0x6B10A3 / 0x881A9C / 0x9E6A82, nop5)).
/// A store JIP removes: the creature's flag word clear (G04: `leave; ret 4` in its place) and the
/// three direction flattenings (G22, the "AI Z fix", credits Stewie: `fldz; fstp` made a nop).
/// Answered, so the store is skipped.
fn replaced(_: &mut Engine, _: &[u32]) -> Option<u32> {
    Some(0)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_game.h: InitGamePatches
// (SafeWrite32(0x8D535F, 0x001F0F50)).
/// `SetLevel`'s required experience (`0x8D535F`: the level): JIP pushes the level itself where
/// the exe pushes the level minus 1 (`push eax` for `sub eax, 1; push eax`).
fn set_level_experience(_: &mut Engine, a: &[u32]) -> Option<u32> {
    Some(a[0])
}

/// `ActorValueOwner::GetClampedActorFloatValue` (thiscall: the owner, the actor value).
const GET_CLAMPED_ACTOR_VALUE: u32 = 0x0066_ef50;
/// `kAVCode_Agility`.
const AGILITY: u32 = 10;

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_game.h: InitGamePatches
// (SAFE_WRITE_BUF(0x8C17D9 / 0x8C1959, "\xB8\x50\xEF\x66\x00\xFF\xD0...")).
/// The Agility for the reload and equip speeds (`0x8C17D9`, `0x8C1959`: the actor's value
/// owner): JIP calls `GetClampedActorFloatValue(Agility)` (`0066ef50`, by address) in place of
/// the owner's virtual `+0xC`; answered as the value's `f32` bits.
fn capped_agility(e: &mut Engine, a: &[u32]) -> Option<u32> {
    let value = e.call(GET_CLAMPED_ACTOR_VALUE, &args![a[0], AGILITY]).f32();
    Some(value.to_bits())
}

/// JIP's `pc3rdPNode`: the root node of the player's last ragdoll build (0 before the first).
#[derive(Default)]
pub struct LastPlayerRoot(AtomicU32);

/// The player's reference id (`refID` 0x14).
const PLAYER_REF_ID: u32 = 0x14;

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/hooks.h: InitRagdollControllerHook.
/// The ragdoll build at Havok init (`0x931384`: the actor, the root node): for the player
/// (`refID` 0x14) the root node is kept as the last one; when it is the same as the last and the
/// actor has a ragdoll controller (`+0xAC`), answered (`retn 0x10`: no build); else nothing, and
/// `Actor::CreateRagdollInstance` (`0087e130`) runs.
fn ragdoll_build(e: &mut Engine, a: &[u32]) -> Option<u32> {
    let (actor, root) = (a[0], a[1]);
    let id = e.get(Ptr::<TESForm>::new(actor), TESForm::iFormID);
    if id != PLAYER_REF_ID {
        return None;
    }
    let last = e
        .ext_free::<LastPlayerRoot>()
        .0
        .swap(root, Ordering::Relaxed);
    if last != root {
        return None;
    }
    let controller = e.get(
        Ptr::<RagdollOwner>::new(actor),
        RagdollOwner::pRagdollController,
    );
    (controller != 0).then_some(0)
}

layout! {
    /// `Actor` (PC), the field H11 reads (kept here, in the group's own file).
    pub struct RagdollOwner: 0x1B4 {
        /// The actor's ragdoll controller (`+0xAC`, JIP's `ragDollController`; the exe's Havok
        /// init reads it after the build).
        0xAC pRagdollController: u32,
    }
}

/// The exe's dialogue sound flags (`mov [ebp-0x248], 0x104`).
const VOICE_FLAGS_EXE: u32 = 0x104;
/// The flag JIP adds (the sound's "voice modulation" bit, as at the two other places).
const MODULATED: u32 = 0x8_0000;
/// The base form flag JIP tests (`test byte [base+9], 0x20`).
const BASE_FLAG_NO_MODULATION: u32 = 0x2000;

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_game.h: VoiceModulationFixHook.
/// The dialogue sound's flags (`0x934AC8`: the object): 0x104 when the object's base form has
/// flag 0x2000, else 0x80104. JIP reads the base form with no null test (a reference with none
/// would stop the game there); the port answers nothing then, leaving the exe's 0x104.
fn voice_flags(e: &mut Engine, a: &[u32]) -> Option<u32> {
    let base = e.get(
        Ptr::<TESObjectREFR>::new(a[0]),
        TESObjectREFR::pObjectReference,
    );
    if base == 0 {
        return None;
    }
    let flags = e.get(Ptr::<TESForm>::new(base), TESForm::iFormFlags);
    Some(if flags & BASE_FLAG_NO_MODULATION != 0 {
        VOICE_FLAGS_EXE
    } else {
        VOICE_FLAGS_EXE | MODULATED
    })
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_game.h: HOOK_INIT_LIST
// (VoiceModulationFix, {0x578E16, 8UL, 1, 0}, {0x7974CC, 8UL, 1, 0}).
/// The two other sound flag words (`0x578E16`: `0x106 | ..`; `0x7974CC`: `0x105`; the word the
/// exe passes): JIP writes byte 8 at `+2` of each immediate, so 0x80000 is added.
fn voice_flag_added(_: &mut Engine, a: &[u32]) -> Option<u32> {
    Some(a[0] | MODULATED)
}

#[cfg(test)]
#[path = "ge_tests.rs"]
mod tests;
