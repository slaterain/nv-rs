//! `functions_jip/jip_fn_actor.h` (JIP LN NVSE), its first 70 commands in the file's order (the
//! rest is `jip_fn_actor_p2`): an actor base's template, voice type, leveling data and base actor
//! values, a creature's damage, reach and model list, an actor's process (level, light, alpha,
//! dive breath, idle, weapon, ammo, package, knocked state, last hit), its combat targets and
//! allies, what it detects, its followers and dropped items, its active effects' modifiers and
//! its perk modifiers, the turn calls, a name set on a base form or a cell, the character
//! controller's fall time, velocity, air state and the body under its feet, and the backup
//! package away from the player, and (lane C3) the radiation an actor takes. 56 built here
//! (`ToggleDetectionFix` with its patch).
//!
//! Built by the core track inside its hook files (not again here): `GetCombatDisabled`,
//! `SetCombatDisabled`, `ToggleNoHealthReset`, `GetTeammateUsingAmmo`, `SetTeammateUsingAmmo`,
//! `ForceActorDetectionValue` (`ai_hooks`), `ToggleIgnoreLockedDoors` (`world_hooks`), and the
//! five `SetOn...EventHandler` commands (`actor_hooks`).
//!
//! Left for later, with what each waits for:
//! - `GetWheelDisabled`/`SetWheelDisabled` (flag 0x80, read only by the companion-wheel patch,
//!   row G54): the flag alone changes nothing in the game until that patch exists.
//!   (`ToggleDetectionFix` is listed here; its handler is with its patch, G53, in
//!   `game_patches::gc`.)
//!
//! An actor base (`TESActorBase`): its base data at +0x30 (`ACBS`, saved by
//! `TESActorBaseData::Save` `0047CAC0` as 0x18 bytes from +4: flags +0x34, level +0x3C (signed
//! 16 bits), calc min +0x3E, calc max +0x40 (16 bits each); the voice type `VTCK` +0x50, the
//! template `TPLT` +0x54), its `ActorValueOwner` at +0x100 (`GetActorValue` its vtable +0xC, read
//! by the exe's `ModActorValue` `005F7520`, the actor base's vtable +0x198). A creature: its
//! attack damage +0x110 (the component at +0x10C, `0047FF50` stores +4 of it; `005FA200` calls it
//! on +0x10C), the model list +0x114 (`TESCreature::Save` `005F7C20`, its names a list at +4 of
//! it), the hand reach `RNAM` +0x130 (same save).
//!
//! Array walks sized by indexes or a count read from the game's memory (the process lists, a
//! combat group's targets and members, an actor's target and ally arrays) go through at most the
//! crate's list bound ([`list_step`]: [`nvse::invref::MAX_LIST_NODES`] entries, where real arrays
//! hold a few hundred): a count word a bad save or mod made huge ends the walk there with the
//! list guard row, as a list that runs back into itself does. The element addresses wrap.

use super::arrays::{create_array, Element};
use super::extra;
use super::forms::{
    ActiveEffect as A, EffectSetting, ListNode, ProcessManager, ACTOR_MAGIC_TARGET,
};
use super::havok_refs as h;
use super::ln_fn_actor::actor_base;
use super::ln_fn_inventory::container_of;
use super::{
    buffer_arg, cvttss2si, full_name, heap_alloc, heap_free, is_actor, list_items, list_prepend,
    list_step, list_unlink, ref_id, set_form_result, string_set, type_of, Conditions, Handlers,
};
use crate::utility::str_compare_ci;
use engine_core::prelude::*;
use nvse::commands::Read;
use nvse::{handled, CommandArgs, EvalArgs};

#[rustfmt::skip]
pub static COMMANDS: Handlers = &[
    (0x2601, get_actor_template),
    (0x2602, get_leveled_actor_base),
    (0x2603, get_creature_damage),
    (0x2604, set_creature_damage),
    (0x2637, get_followers),
    (0x2689, get_actor_leveling_data),
    (0x268a, set_actor_leveling_data),
    (0x26d5, get_actor_voice_type),
    (0x26d6, set_actor_voice_type),
    (0x26d7, get_creature_reach),
    (0x26d8, get_is_immobile),
    (0x26d9, pick_from_list),
    (0x26ea, toggle_creature_model),
    (0x26eb, creature_has_model),
    (0x26ec, get_creature_models),
    (0x2723, mod_base_actor_value),
    (0x2734, get_actor_processing_level),
    (0x2735, get_actors_by_processing_level),
    (0x273c, get_actor_alpha),
    (0x273d, get_actor_dive_breath),
    (0x273e, set_actor_dive_breath),
    (0x2742, get_combat_targets),
    (0x2743, get_combat_allies),
    (0x2744, get_detected_actors),
    (0x2745, get_detecting_actors),
    (0x2757, get_current_stable_package),
    (0x275c, crate::game_patches::gc::toggle_detection_fix),
    (0x2761, is_in_combat_with),
    (0x2741, stop_idle),
    (0x276b, holster_weapon),
    (0x2780, get_dropped_refs),
    (0x2782, face_object),
    (0x2783, turn),
    (0x2786, get_ash_pile_source),
    (0x27a1, get_current_ammo),
    (0x27a2, get_current_ammo_rounds),
    (0x27a5, set_full_name_alt),
    (0x27a7, get_detection_value),
    (0x27a8, get_base_actor_value_alt),
    (0x27ac, set_speed_mult),
    (0x27f5, get_actor_value_modifier),
    (0x27f6, get_perk_modifier),
    (0x2802, get_hit_health_damage),
    (0x276c, get_fall_time_elapsed),
    (0x276d, get_fall_time_remaining),
    (0x276e, reset_fall_time),
    (0x27b4, get_actor_velocity),
    (0x27ba, get_object_under_feet),
    (0x2781, move_away_from_player),
];

#[rustfmt::skip]
pub static CONDITIONS: Conditions = &[
    (0x2605, get_is_poisoned, get_is_poisoned_eval),
    (0x273b, get_actor_light_amount, get_actor_light_amount_eval),
    (0x2769, is_attacking, is_attacking_eval),
    (0x277d, is_in_water, is_in_water_eval),
    (0x27ad, get_is_ragdolled, get_is_ragdolled_eval),
    (0x27b5, is_in_air, is_in_air_eval),
    (0x277c, get_radiation_level_alt, get_radiation_level_alt_eval),
];

/// A reference's base form (+0x20) and an actor's process (`pCurrentProcess`, +0x68).
const BASE: u32 = 0x20;
const PROCESS: u32 = 0x68;
/// The actor base data's fields (see the module's doc).
const BD_FLAGS: u32 = 0x34;
const BD_LEVEL: u32 = 0x3c;
const BD_CALC_MIN: u32 = 0x3e;
const BD_CALC_MAX: u32 = 0x40;
const BD_VOICE_TYPE: u32 = 0x50;
const BD_TEMPLATE: u32 = 0x54;
/// `ACBS` flag 0x80: the level is a multiplier of the player's (thousandths).
const PC_LEVEL_MULT: u32 = 0x80;
/// `ACBS` flag 0x800000, JIP's immobile test on a creature base.
const IMMOBILE_FLAG: u32 = 0x0080_0000;
const ACTOR_BASE_AV_OWNER: u32 = 0x100;
const CREATURE_DAMAGE: u32 = 0x110;
const CREATURE_MODELS: u32 = 0x118;
const CREATURE_REACH: u32 = 0x130;
/// An actor (Xbox PDB names in engine-fallout-ai `actor.rs`): its `ActorValueOwner` +0xA4,
/// `eLifeState` +0x108, the combat target and member arrays +0x12C and +0x130 (each a
/// `BSSimpleArray*`: its data +4, its size +8, as JIP's `IsInCombatWith` reads them), `bInWater`
/// +0x14C, `bTurretBehavior` +0x1B0 (JIP's `isImmobileCreature`).
const ACTOR_AV_OWNER: u32 = 0xa4;
const LIFE_STATE: u32 = 0x108;
const COMBAT_TARGETS: u32 = 0x12c;
const COMBAT_ALLIES: u32 = 0x130;
const IN_WATER: u32 = 0x14c;
const IMMOBILE_CREATURE: u32 = 0x1b0;
/// A reference's rotation about z (+0x2C) and position (+0x30, +0x34, +0x38).
const ROT_Z: u32 = 0x2c;
const POS: u32 = 0x30;
/// `g_thePlayer`; the player's combat group (`pCombatGroup` +0xD64) and difficulty
/// (`eDifficultyLevel` +0x7B8), engine-fallout-ai `playercharacter.rs`.
const PLAYER: u32 = 0x011d_ea3c;
const PLAYER_ID: u32 = 0x14;
const PLAYER_COMBAT_GROUP: u32 = 0xd64;
const PLAYER_DIFFICULTY: u32 = 0x7b8;
/// A combat group's target array (`BSSimpleArray<CombatTarget>` at +8: data +0xC, size +0x10,
/// 0x68-byte elements) and member array (+0x18: data +0x1C, size +0x20, 0x14-byte elements),
/// each element's actor first (engine-fallout-ai `combatgroup.rs`).
const GROUP_TARGETS: u32 = 0x0c;
const GROUP_TARGET_COUNT: u32 = 0x10;
const TARGET_SIZE: u32 = 0x68;
const GROUP_ALLIES: u32 = 0x1c;
const GROUP_ALLY_COUNT: u32 = 0x20;
const ALLY_SIZE: u32 = 0x14;
/// A process: its running package (`currentPackage.package` +4), its level (+0x28: 0 high, 1
/// middle-high, 2 middle-low, 3 low), its cached values (+0x2C, their flags +0x44: walk and run
/// speed cached 0x1000, 0x2000; engine-fallout-ai `actor_p2.rs`). A middle-high process: its
/// ammo entry +0x118 (`MiddleHighProcess::GetCurrentAmmo` `005F7590`), its knocked state +0x13C
/// (`GetKnockState` `008D8540`), its last hit +0x240 (`game::MiddleHighProcess`). A high
/// process: its detected and detecting lists +0x25C and +0x260 (`GetDetectionState`
/// `008F6650`), its light amount +0x3C4 (`SetActorLightLevel` `008D94D0`).
const P_PACKAGE: u32 = 0x04;
const P_LEVEL: u32 = 0x28;
const P_CACHED: u32 = 0x2c;
const CACHED_FLAGS: u32 = 0x44;
const CACHED_SPEEDS: u32 = 0x3000;
const P_AMMO: u32 = 0x118;
const P_KNOCKED: u32 = 0x13c;
const P_LAST_HIT: u32 = 0x240;
const P_DETECTED: u32 = 0x25c;
const P_DETECTING: u32 = 0x260;
const P_LIGHT: u32 = 0x3c4;
/// The process's virtual functions (HighProcess vtable `01087864`): `PostAnimFreeUpSpecialIdle`
/// +0xD8 (JIP's `StopIdle`), `SetBreathTimer` +0x2FC, `GetBreathTimer` +0x300, `GetAnimAction`
/// +0x3E4, `GetWeaponDrawn` +0x454, `SetWeaponDrawn` +0x458, `GetDetectionState` +0x504
/// (JIP's `GetDetectionData`), `GetAlphaMult` +0x5AC.
const STOP_IDLE: u32 = 0xd8;
const SET_DIVE_BREATH: u32 = 0x2fc;
const GET_DIVE_BREATH: u32 = 0x300;
const GET_CURRENT_ACTION: u32 = 0x3e4;
const IS_WEAPON_OUT: u32 = 0x454;
const SET_WEAPON_OUT: u32 = 0x458;
const GET_DETECTION_DATA: u32 = 0x504;
const GET_ACTOR_ALPHA: u32 = 0x5ac;
/// `DetectionData` (`DetectionState`, Xbox PDB, engine-fallout-ai `highprocess.rs`): the actor
/// +0, `iLevel` +8 (JIP's `detectionValue`), `bLineofSight` +0x1E.
const DETECTION_VALUE: u32 = 0x08;
const IN_LOS: u32 = 0x1e;
/// A package's type byte (+0x20).
const PACKAGE_TYPE: u32 = 0x20;
/// `ContChangesEntry`: its count +4, its form +8.
const ENTRY_COUNT: u32 = 0x04;
const ENTRY_TYPE: u32 = 0x08;
/// `ActorHitData`: its target +4, its health damage +0x14.
const HIT_TARGET: u32 = 0x04;
const HIT_HEALTH: u32 = 0x14;
/// The difficulty multipliers the exe's `00648CB0` reads: five game settings for hits on the
/// player, then five for the rest (+0x14), by difficulty; a setting's value at +4.
const DIFFICULTY_MULTS: u32 = 0x0119_b310;
/// Virtual functions: the actor base's `ModActorValue` (+0x198), `ActorValueOwner`'s
/// `GetBaseActorValue` (+4) and `GetActorValue` (+0xC), the actor's `SetActorValueInt` (+0x398,
/// `008806D0` on a character), `MagicTarget::GetEffectList` (+8).
const MOD_ACTOR_VALUE: u32 = 0x198;
const GET_BASE_ACTOR_VALUE: u32 = 0x04;
const GET_ACTOR_VALUE: u32 = 0x0c;
const SET_ACTOR_VALUE_INT: u32 = 0x398;
const GET_EFFECT_LIST: u32 = 0x08;
/// `Actor::TurnToFaceObject`'s and `Actor::TurnAngle`'s game calls (`thiscall`: x, y, z, 0;
/// angle, 0).
const TURN_TO_POINT: u32 = 0x008b_b520;
const TURN_TO_ANGLE: u32 = 0x008b_b5c0;
/// `ApplyPerkModifiers` (the exe's `HandleEntryPoint`, `cdecl`), the entry points' table (16
/// bytes each, the tab count at +4) and `kPerkEntry_Max`.
const APPLY_PERK_MODIFIERS: u32 = 0x005e_58f0;
const ENTRY_POINT_INFO: u32 = 0x0119_6ee0;
const PERK_ENTRY_MAX: u32 = 0x4a;
/// `ProcessManager` (the exe's `ProcessLists`).
const PROCESS_MANAGER: u32 = 0x011e_0e80;
/// Extra data: `ExtraPackage` 0x19 (+0xC), `ExtraFollower` 0x1D (a list pointer at +0xC,
/// `ExtraFollower::ExtraFollower` `00430DD0`), `ExtraDroppedItemList` 0x3A (a list at +0xC),
/// `ExtraAshPileRef` 0x89 (+0xC, `ExtraDataList::GetAshPileRef` `0041E310`).
const EXTRA_PACKAGE: u32 = 0x19;
const EXTRA_FOLLOWER: u32 = 0x1d;
const EXTRA_DROPPED_ITEM_LIST: u32 = 0x3a;
const EXTRA_ASH_PILE_REF: u32 = 0x89;
const EXTRA_WORD: u32 = 0x0c;
/// `kFormType_*` (JIP LN NVSE).
const TES_OBJECT_ACTI: u8 = 0x15;
const BGS_TALKING_ACTIVATOR: u8 = 0x16;
const TES_NPC: u8 = 0x2a;
const TES_CREATURE: u8 = 0x2b;
const TES_OBJECT_CELL: u8 = 0x39;
const BGS_VOICE_TYPE: u8 = 0x5d;

// ---- JIP's helpers ----

/// `actorBase || (thisObj && IS_ACTOR(thisObj) && (actorBase = thisObj->GetActorBase()))`.
fn base_or_actor(e: &mut Engine, form: u32, this: u32) -> Option<u32> {
    if form != 0 {
        return Some(form);
    }
    if is_actor(e, this) {
        let b = actor_base(e, this);
        return (b != 0).then_some(b);
    }
    None
}

/// The actor's process, 0 for none (or for a reference that is not an actor).
fn process(e: &Engine, actor: u32) -> u32 {
    if is_actor(e, actor) {
        e.mem.u32(actor + PROCESS)
    } else {
        0
    }
}

fn level(e: &Engine, process: u32) -> u32 {
    e.mem.u32(process + P_LEVEL)
}

/// The items of a `tList` from its first node at `head` (0: none).
fn items(e: &Engine, head: u32) -> Vec<u32> {
    if head == 0 {
        Vec::new()
    } else {
        list_items(e, Ptr::<ListNode>::new(head))
    }
}

/// `TESObjectREFR::IsPlayer`: the reference's id is 0x14.
fn is_player(e: &Engine, refr: u32) -> bool {
    ref_id(e, refr) == PLAYER_ID
}

/// `Actor::GetDead`: life state 1 (dying) or 2 (dead).
fn dead(e: &Engine, actor: u32) -> bool {
    matches!(e.mem.u32(actor + LIFE_STATE), 1 | 2)
}

/// The array result of `forms`, made only when there is one (`CreateArray`, `(int)` id).
fn array_result(e: &mut Engine, a: &CommandArgs, forms: &[u32]) {
    if !forms.is_empty() {
        let els: Vec<Element> = forms.iter().map(|&f| Element::Form(f)).collect();
        let id = create_array(e, a.script, &els);
        a.set_result(e, f64::from(id as i32));
    }
}

/// The actor's active effects (`magicTarget.GetEffectList()`), from the first node; empty for
/// no list.
fn effects(e: &mut Engine, actor: u32) -> Vec<u32> {
    let first = e
        .vcall(actor + ACTOR_MAGIC_TARGET, GET_EFFECT_LIST, &[])
        .u32();
    items(e, first)
}

/// `BaseProcess::GetDetectionData(target, 0)`, 0 for none.
fn detection_data(e: &mut Engine, process: u32, target: u32) -> u32 {
    e.vcall(process, GET_DETECTION_DATA, &[target, 0]).u32()
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), nvse/GameObjects.cpp: Actor::GetDetectionValue.
/// The value of the actor's detection data for `detected` (a high process only), else -100.
fn detection_value(e: &mut Engine, actor: u32, detected: u32) -> i32 {
    let p = e.mem.u32(actor + PROCESS);
    if p != 0 && level(e, p) == 0 {
        let d = detection_data(e, p, detected);
        if d != 0 {
            return e.mem.u32(d + DETECTION_VALUE) as i32;
        }
    }
    -100
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), nvse/GameObjects.cpp: Actor::GetLOS.
fn has_los(e: &mut Engine, actor: u32, target: u32) -> bool {
    let p = e.mem.u32(actor + PROCESS);
    if p == 0 || level(e, p) != 0 {
        return false;
    }
    let d = detection_data(e, p, target);
    d != 0 && e.mem.u8(d + IN_LOS) != 0
}

// ---- actor bases ----

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_actor.h:
// Cmd_GetActorTemplate_Execute.
fn get_actor_template(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if let Some(b) = read
        .ok
        .then(|| base_or_actor(e, read.form(0), a.this_obj))
        .flatten()
    {
        let t = e.mem.u32(b + BD_TEMPLATE);
        if t != 0 {
            set_form_result(e, &a, t);
        }
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_actor.h:
// Cmd_GetLeveledActorBase_Execute.
/// The calling actor's base (its leveled base for a base made in the game).
fn get_leveled_actor_base(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    if is_actor(e, a.this_obj) {
        let b = actor_base(e, a.this_obj);
        if b != 0 {
            set_form_result(e, &a, b);
        }
    }
    handled(true)
}

/// The creature argument, else the calling actor's base, when it is a creature.
fn creature(e: &mut Engine, read: &Read, i: usize, this: u32) -> Option<u32> {
    if !read.ok {
        return None;
    }
    base_or_actor(e, read.form(i), this).filter(|&c| type_of(e, c) == TES_CREATURE)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_actor.h:
// Cmd_GetCreatureDamage_Execute.
fn get_creature_damage(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if let Some(c) = creature(e, &read, 0, a.this_obj) {
        let d = e.mem.u16(c + CREATURE_DAMAGE);
        a.set_result(e, f64::from(d));
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_actor.h:
// Cmd_SetCreatureDamage_Execute.
/// The damage held to 0..0xFFFF.
fn set_creature_damage(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if let Some(c) = creature(e, &read, 1, a.this_obj) {
        let d = (read.int(0, 0) as i32).clamp(0, 0xffff);
        e.mem.set_u16(c + CREATURE_DAMAGE, d as u16);
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_actor.h:
// Cmd_GetActorLevelingData_Execute.
/// 0: the level (thousandths of the player's when the base's flag 0x80 says so), 1: calc min,
/// 2: calc max; any other number nothing.
fn get_actor_leveling_data(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let which = read.int(0, 0);
    if !read.ok || which > 2 {
        return handled(true);
    }
    if let Some(b) = base_or_actor(e, read.form(1), a.this_obj) {
        let v = match which {
            1 => f64::from(e.mem.u16(b + BD_CALC_MIN)),
            2 => f64::from(e.mem.u16(b + BD_CALC_MAX)),
            _ => {
                let lvl = f64::from(e.mem.u16(b + BD_LEVEL) as i16);
                if e.mem.u32(b + BD_FLAGS) & PC_LEVEL_MULT != 0 {
                    lvl * 0.001
                } else {
                    lvl
                }
            }
        };
        a.set_result(e, v);
    }
    handled(true)
}

/// `GetMax(val, 1.0F)`: `val > 1 ? val : 1` (NaN gives 1).
fn at_least_one(val: f32) -> f32 {
    if val > 1.0 {
        val
    } else {
        1.0
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_actor.h:
// Cmd_SetActorLevelingData_Execute.
/// 0: the level (with flag 0x80: below 0.01 gives 10, else the value times 1000; else at least
/// 1), 1: calc min (at least 1; calc max raised to it), 2: calc max (at least 1; calc min lowered
/// to it). The float goes into the 16-bit field as the compiled code converts it (`cvttss2si`).
fn set_actor_leveling_data(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let which = read.int(0, 0);
    if !read.ok || which > 2 {
        return handled(true);
    }
    let Some(b) = base_or_actor(e, read.form(2), a.this_obj) else {
        return handled(true);
    };
    let mut val = read.float(1, 0.0);
    match which {
        0 => {
            if e.mem.u32(b + BD_FLAGS) & PC_LEVEL_MULT != 0 {
                val = if val < 0.01 { 10.0 } else { val * 1000.0 };
            } else {
                val = at_least_one(val);
            }
            e.mem.set_u16(b + BD_LEVEL, cvttss2si(val) as u16);
        }
        1 => {
            val = at_least_one(val);
            e.mem.set_u16(b + BD_CALC_MIN, cvttss2si(val) as u16);
            if f32::from(e.mem.u16(b + BD_CALC_MAX)) < val {
                e.mem.set_u16(b + BD_CALC_MAX, cvttss2si(val) as u16);
            }
        }
        _ => {
            val = at_least_one(val);
            e.mem.set_u16(b + BD_CALC_MAX, cvttss2si(val) as u16);
            if f32::from(e.mem.u16(b + BD_CALC_MIN)) > val {
                e.mem.set_u16(b + BD_CALC_MIN, cvttss2si(val) as u16);
            }
        }
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_actor.h:
// Cmd_GetActorVoiceType_Execute.
fn get_actor_voice_type(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if let Some(b) = read
        .ok
        .then(|| base_or_actor(e, read.form(0), a.this_obj))
        .flatten()
    {
        let v = e.mem.u32(b + BD_VOICE_TYPE);
        if v != 0 {
            set_form_result(e, &a, v);
        }
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_actor.h:
// Cmd_SetActorVoiceType_Execute.
/// A voice type form only.
fn set_actor_voice_type(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let form = read.form(0);
    if read.ok && form != 0 && type_of(e, form) == BGS_VOICE_TYPE {
        if let Some(b) = base_or_actor(e, read.form(1), a.this_obj) {
            e.mem.set_u32(b + BD_VOICE_TYPE, form);
        }
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_actor.h:
// Cmd_GetCreatureReach_Execute.
fn get_creature_reach(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if let Some(c) = creature(e, &read, 0, a.this_obj) {
        let r = e.mem.u8(c + CREATURE_REACH);
        a.set_result(e, f64::from(r));
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_actor.h:
// Cmd_GetIsImmobile_Execute.
/// No creature argument: the calling actor's immobile byte; a creature argument: its base flag
/// 0x800000 (another form type: 0).
fn get_is_immobile(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if !read.ok {
        return handled(true);
    }
    let c = read.form(0);
    let yes = if c == 0 {
        is_actor(e, a.this_obj) && e.mem.u8(a.this_obj + IMMOBILE_CREATURE) != 0
    } else {
        type_of(e, c) == TES_CREATURE && e.mem.u32(c + BD_FLAGS) & IMMOBILE_FLAG != 0
    };
    if yes {
        a.set_result(e, 1.0);
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_actor.h:
// Cmd_PickFromList_Execute.
/// The first form of the list, from `start` and for at most `len` forms (-1: no end), that the
/// calling reference holds (`GetFormCount` above 0); an empty item ends the walk. Needs a base
/// form with a container and the reference's container changes.
fn pick_from_list(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let (mut start, mut len) = (read.int(1, 0) as i32, read.int(2, u32::MAX) as i32);
    if !read.ok || start < 0 || a.this_obj == 0 {
        return handled(true);
    }
    let container = container_of(e, e.mem.u32(a.this_obj + BASE));
    if container == 0 {
        return handled(true);
    }
    let changes = crate::overrides::changes_list(e, a.this_obj);
    if changes == 0 {
        return handled(true);
    }
    for item in items(e, read.form(0) + 0x18) {
        let skip = start > 0;
        start = start.wrapping_sub(1);
        if skip {
            continue;
        }
        let ended = len == 0;
        len = len.wrapping_sub(1);
        if ended || item == 0 {
            break;
        }
        if crate::overrides::form_count(e, container + 4, changes, item) > 0 {
            set_form_result(e, &a, item);
            break;
        }
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), nvse/GameForms.cpp: TESModelList::ModelListAction.
/// The creature's model list and `path` (compared without case): an entry that matches is
/// removed (its text freed, its node unlinked) for a negative `action`; the answer is then
/// `action <= 0`. No match: false for `action <= 0`, else a copy of the path (the game's heap,
/// its length plus one) put first, true.
///
/// A list that runs back into itself takes the crate's bound ([`list_step`]): with the list
/// guard row the walk ends there as at the list's end, but with nothing added (false); JIP would
/// walk it for ever.
fn model_list_action(e: &mut Engine, creature: u32, path: &[u8], action: i8) -> bool {
    let head = Ptr::<ListNode>::new(creature + CREATURE_MODELS);
    let (mut node, mut prev) = (head, Ptr::<ListNode>::NULL);
    let mut walked = 0usize;
    loop {
        if !list_step(e, &mut walked) {
            return false;
        }
        let text = e.get(node, ListNode::data);
        if text != 0 && str_compare_ci(&nvse::table::guest_text(e, text), path).0 == 0 {
            if action < 0 {
                heap_free(e, text);
                list_unlink(e, node, prev);
            }
            return action <= 0;
        }
        prev = node;
        node = Ptr::new(e.get(node, ListNode::next));
        if node.is_null() {
            break;
        }
    }
    if action <= 0 {
        return false;
    }
    let copy = heap_alloc(e, path.len() as u32 + 1);
    e.mem.write(copy, path);
    e.mem.set_u8(copy + path.len() as u32, 0);
    list_prepend(e, head, copy);
    true
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_actor.h:
// Cmd_ToggleCreatureModel_Execute.
fn toggle_creature_model(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if let Some(c) = creature(e, &read, 2, a.this_obj) {
        let path = buffer_arg(e, &read, 0, 0x100, "ToggleCreatureModel");
        let action = if read.int(1, 0) != 0 { 1 } else { -1 };
        if model_list_action(e, c, &path, action) {
            a.set_result(e, 1.0);
        }
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_actor.h:
// Cmd_CreatureHasModel_Execute.
fn creature_has_model(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if let Some(c) = creature(e, &read, 1, a.this_obj) {
        let path = buffer_arg(e, &read, 0, 0x100, "CreatureHasModel");
        if model_list_action(e, c, &path, 0) {
            a.set_result(e, 1.0);
        }
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_actor.h:
// Cmd_GetCreatureModels_Execute.
/// An array of the model paths (strings), none for an empty list.
fn get_creature_models(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if let Some(c) = creature(e, &read, 0, a.this_obj) {
        let els: Vec<Element> = items(e, c + CREATURE_MODELS)
            .into_iter()
            .filter(|&t| t != 0)
            .map(|t| Element::Str(nvse::table::guest_text(e, t)))
            .collect();
        if !els.is_empty() {
            let id = create_array(e, a.script, &els);
            a.set_result(e, f64::from(id as i32));
        }
    }
    handled(true)
}

/// The actor base argument, else the calling actor's base form itself (not its leveled base).
fn base_or_this_base(e: &Engine, form: u32, this: u32) -> Option<u32> {
    if form != 0 {
        Some(form)
    } else if is_actor(e, this) {
        let b = e.mem.u32(this + BASE);
        (b != 0).then_some(b)
    } else {
        None
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_actor.h:
// Cmd_ModBaseActorValue_Execute.
/// The actor base's `ModActorValue` (its virtual function: the value it has plus the amount).
fn mod_base_actor_value(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if read.ok {
        if let Some(b) = base_or_this_base(e, read.form(2), a.this_obj) {
            let (av, value) = (read.short(0, 0), read.float(1, 0.0));
            e.vcall(b, MOD_ACTOR_VALUE, &[av, value.to_bits()]);
        }
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_actor.h:
// Cmd_GetBaseActorValueAlt_Execute.
/// The actor base's actor value (a float), else the calling actor's base actor value as a whole
/// number (toward zero).
fn get_base_actor_value_alt(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if !read.ok {
        return handled(true);
    }
    let (av, base) = (read.short(0, 0), read.form(1));
    if base != 0 {
        let v = e
            .vcall(base + ACTOR_BASE_AV_OWNER, GET_ACTOR_VALUE, &[av])
            .f32();
        a.set_result(e, f64::from(v));
    } else if is_actor(e, a.this_obj) {
        let v = e
            .vcall(a.this_obj + ACTOR_AV_OWNER, GET_BASE_ACTOR_VALUE, &[av])
            .f32();
        a.set_result(e, f64::from(cvttss2si(v)));
    }
    handled(true)
}

// ---- processes ----

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_actor.h:
// Cmd_GetActorProcessingLevel_Execute.
/// The process level, -1 without a process.
fn get_actor_processing_level(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let p = process(e, a.this_obj);
    let lvl = if p != 0 { level(e, p) as i32 } else { -1 };
    a.set_result(e, f64::from(lvl));
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_actor.h:
// Cmd_GetActorsByProcessingLevel_Execute.
/// The actors of process level 0 to 3 in the process lists' order.
fn get_actors_by_processing_level(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let lvl = read.int(0, 0);
    if !read.ok || lvl > 3 {
        return handled(true);
    }
    let data = e.get(
        Ptr::<ProcessManager>::new(PROCESS_MANAGER),
        ProcessManager::objects,
    );
    let first = e
        .mem
        .u32(PROCESS_MANAGER + ProcessManager::beginOffsets.off + lvl.wrapping_mul(4));
    let past = e
        .mem
        .u32(PROCESS_MANAGER + ProcessManager::endOffsets.off + lvl.wrapping_mul(4));
    // Indexes read from the game's memory: at most the crate's list bound (the module's notes).
    let mut walked = 0usize;
    let actors: Vec<u32> = (first..past)
        .take_while(|_| list_step(e, &mut walked))
        .map(|i| e.mem.u32(data.wrapping_add(i.wrapping_mul(4))))
        .filter(|&actor| is_actor(e, actor))
        .collect();
    array_result(e, &a, &actors);
    handled(true)
}

/// The light amount of a high process.
fn light_amount(e: &Engine, actor: u32) -> Option<f32> {
    let p = process(e, actor);
    (p != 0 && level(e, p) == 0).then(|| e.mem.f32(p + P_LIGHT))
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_actor.h:
// Cmd_GetActorLightAmount_Execute.
fn get_actor_light_amount(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    if let Some(v) = light_amount(e, a.this_obj) {
        a.set_result(e, f64::from(v));
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_actor.h:
// Cmd_GetActorLightAmount_Eval.
fn get_actor_light_amount_eval(e: &mut Engine, w: &[u32]) -> Ret {
    let a = EvalArgs::new(w);
    if let Some(v) = light_amount(e, a.this_obj) {
        e.mem.set_f64(a.result, f64::from(v));
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_actor.h:
// Cmd_GetActorAlpha_Execute.
fn get_actor_alpha(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let p = process(e, a.this_obj);
    if p != 0 {
        let v = e.vcall(p, GET_ACTOR_ALPHA, &[]).f32();
        a.set_result(e, f64::from(v));
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_actor.h:
// Cmd_GetActorDiveBreath_Execute.
fn get_actor_dive_breath(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let p = process(e, a.this_obj);
    if p != 0 {
        let v = e.vcall(p, GET_DIVE_BREATH, &[]).f32();
        a.set_result(e, f64::from(v));
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_actor.h:
// Cmd_SetActorDiveBreath_Execute.
/// `GetMax(breath, 0.0F)` (NaN gives 0).
fn set_actor_dive_breath(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let p = process(e, a.this_obj);
    if read.ok && p != 0 {
        let b = read.float(0, 0.0);
        let b = if b > 0.0 { b } else { 0.0 };
        e.vcall(p, SET_DIVE_BREATH, &[b.to_bits()]);
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_actor.h: GetCombatActors.
/// The player: its combat group's targets, or its members other than itself (the group of
/// `g_thePlayer`, whichever player-id reference calls). Another actor: its own target or member
/// array, itself left out.
fn combat_actors(e: &mut Engine, a: &CommandArgs, allies: bool) {
    let this = a.this_obj;
    if this == 0 {
        return;
    }
    let mut found = Vec::new();
    if is_player(e, this) {
        let group = e.mem.u32(e.mem.u32(PLAYER) + PLAYER_COMBAT_GROUP);
        if group == 0 {
            return;
        }
        let (data, count, size) = if allies {
            (GROUP_ALLIES, GROUP_ALLY_COUNT, ALLY_SIZE)
        } else {
            (GROUP_TARGETS, GROUP_TARGET_COUNT, TARGET_SIZE)
        };
        let (data, count) = (e.mem.u32(group + data), e.mem.u32(group + count));
        let mut walked = 0usize;
        for i in 0..count {
            if !list_step(e, &mut walked) {
                break;
            }
            let actor = e.mem.u32(data.wrapping_add(i.wrapping_mul(size)));
            if actor != 0 && (!allies || actor != this) {
                found.push(actor);
            }
        }
    } else {
        if !is_actor(e, this) {
            return;
        }
        let arr = e.mem.u32(
            this + if allies {
                COMBAT_ALLIES
            } else {
                COMBAT_TARGETS
            },
        );
        if arr == 0 {
            return;
        }
        let (data, count) = (e.mem.u32(arr + 4), e.mem.u32(arr + 8));
        if data == 0 {
            return;
        }
        let mut walked = 0usize;
        for i in 0..count {
            if !list_step(e, &mut walked) {
                break;
            }
            let actor = e.mem.u32(data.wrapping_add(i.wrapping_mul(4)));
            if actor != 0 && actor != this {
                found.push(actor);
            }
        }
    }
    array_result(e, a, &found);
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_actor.h:
// Cmd_GetCombatTargets_Execute.
fn get_combat_targets(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    combat_actors(e, &a, false);
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_actor.h:
// Cmd_GetCombatAllies_Execute.
fn get_combat_allies(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    combat_actors(e, &a, true);
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_actor.h: GetDetectionData.
/// The player: the living high-process actors that detect it (`detecting`: their detection
/// value of the player above 0) or that it sees (its line of sight to them). Another actor with
/// a high process: the living actors of its detecting or detected list with a value above 0.
fn detection_actors(e: &mut Engine, a: &CommandArgs, detecting: bool) {
    let actor = a.this_obj;
    if !is_actor(e, actor) {
        return;
    }
    let mut found = Vec::new();
    if is_player(e, actor) {
        let data = e.get(
            Ptr::<ProcessManager>::new(PROCESS_MANAGER),
            ProcessManager::objects,
        );
        let first = e
            .mem
            .u32(PROCESS_MANAGER + ProcessManager::beginOffsets.off);
        let past = e.mem.u32(PROCESS_MANAGER + ProcessManager::endOffsets.off);
        let mut walked = 0usize;
        for i in first..past {
            if !list_step(e, &mut walked) {
                break;
            }
            let target = e.mem.u32(data.wrapping_add(i.wrapping_mul(4)));
            if target == 0 || !is_actor(e, target) || dead(e, target) {
                continue;
            }
            let yes = if detecting {
                detection_value(e, target, actor) > 0
            } else {
                has_los(e, actor, target)
            };
            if yes {
                found.push(target);
            }
        }
    } else {
        let p = e.mem.u32(actor + PROCESS);
        if p == 0 || level(e, p) != 0 {
            return;
        }
        let list = e
            .mem
            .u32(p + if detecting { P_DETECTING } else { P_DETECTED });
        for d in items(e, list) {
            if d == 0 {
                continue;
            }
            let other = e.mem.u32(d);
            if e.mem.u32(d + DETECTION_VALUE) as i32 > 0 && other != 0 && !dead(e, other) {
                found.push(other);
            }
        }
    }
    array_result(e, a, &found);
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_actor.h:
// Cmd_GetDetectedActors_Execute.
fn get_detected_actors(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    detection_actors(e, &a, false);
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_actor.h:
// Cmd_GetDetectingActors_Execute.
fn get_detecting_actors(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    detection_actors(e, &a, true);
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), nvse/GameObjects.cpp: Actor::GetStablePackage.
/// The running package when its type is below 18, 26 or 30; else the reference's
/// `ExtraPackage` package; 0 without a process.
fn stable_package(e: &mut Engine, actor: u32) -> u32 {
    let p = e.mem.u32(actor + PROCESS);
    if p == 0 {
        return 0;
    }
    let package = e.mem.u32(p + P_PACKAGE);
    if package == 0 {
        return 0;
    }
    let t = e.mem.u8(package + PACKAGE_TYPE);
    if t < 18 || t == 26 || t == 30 {
        return package;
    }
    match extra::refr_extra(e, actor, EXTRA_PACKAGE) {
        0 => 0,
        x => e.mem.u32(x + EXTRA_WORD),
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_actor.h:
// Cmd_GetCurrentStablePackage_Execute.
/// The package, echoed in the console (none: `0`).
fn get_current_stable_package(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let package = if is_actor(e, a.this_obj) {
        stable_package(e, a.this_obj)
    } else {
        0
    };
    if package != 0 {
        set_form_result(e, &a, package);
    }
    crate::console::print_form(e, &a, package);
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_actor.h:
// Cmd_IsInCombatWith_Execute; nvse/GameObjects.cpp: Actor::IsInCombatWith (asm).
/// The player: the target is one of its combat group's targets. Another actor: the target is in
/// its own target array.
fn is_in_combat_with(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if !is_actor(e, a.this_obj) || !read.ok {
        return handled(true);
    }
    let target = read.form(0);
    let mut walked = 0usize;
    let yes = if !is_player(e, a.this_obj) {
        let arr = e.mem.u32(a.this_obj + COMBAT_TARGETS);
        arr != 0 && {
            let (data, count) = (e.mem.u32(arr + 4), e.mem.u32(arr + 8));
            (0..count)
                .take_while(|_| list_step(e, &mut walked))
                .any(|i| e.mem.u32(data.wrapping_add(i.wrapping_mul(4))) == target)
        }
    } else {
        let group = e.mem.u32(e.mem.u32(PLAYER) + PLAYER_COMBAT_GROUP);
        group != 0 && {
            let data = e.mem.u32(group + GROUP_TARGETS);
            let count = e.mem.u32(group + GROUP_TARGET_COUNT);
            (0..count)
                .take_while(|_| list_step(e, &mut walked))
                .any(|i| e.mem.u32(data.wrapping_add(i.wrapping_mul(TARGET_SIZE))) == target)
        }
    };
    if yes {
        a.set_result(e, 1.0);
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_actor.h: IsAttacking.
/// The process's current action (16 bits, signed) is 2 to 5.
fn attacking(e: &mut Engine, actor: u32) -> bool {
    let p = process(e, actor);
    p != 0 && {
        let action = e.vcall(p, GET_CURRENT_ACTION, &[]).u16() as i16;
        action > 1 && action < 6
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_actor.h:
// Cmd_IsAttacking_Execute.
fn is_attacking(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    if attacking(e, a.this_obj) {
        a.set_result(e, 1.0);
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_actor.h:
// Cmd_IsAttacking_Eval.
fn is_attacking_eval(e: &mut Engine, w: &[u32]) -> Ret {
    let a = EvalArgs::new(w);
    if attacking(e, a.this_obj) {
        e.mem.set_f64(a.result, 1.0);
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_actor.h:
// Cmd_StopIdle_Execute.
fn stop_idle(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let p = process(e, a.this_obj);
    if p != 0 {
        e.vcall(p, STOP_IDLE, &[a.this_obj]);
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_actor.h:
// Cmd_HolsterWeapon_Execute.
fn holster_weapon(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let p = process(e, a.this_obj);
    if p != 0 && e.vcall(p, IS_WEAPON_OUT, &[]).u8() != 0 {
        e.vcall(p, SET_WEAPON_OUT, &[a.this_obj, 0]);
    }
    handled(true)
}

/// `IS_ACTOR(thisObj) && inWater`.
fn in_water(e: &Engine, actor: u32) -> bool {
    is_actor(e, actor) && e.mem.u8(actor + IN_WATER) != 0
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_actor.h:
// Cmd_IsInWater_Execute.
fn is_in_water(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    if in_water(e, a.this_obj) {
        a.set_result(e, 1.0);
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_actor.h:
// Cmd_IsInWater_Eval.
fn is_in_water_eval(e: &mut Engine, w: &[u32]) -> Ret {
    let a = EvalArgs::new(w);
    if in_water(e, a.this_obj) {
        e.mem.set_f64(a.result, 1.0);
    }
    handled(true)
}

// ---- radiation (lane C3) ----

/// An actor's `bSwimming` +0x14D (Xbox PDB; `IsSwimming`'s condition reads it) and its loaded
/// data +0x64 (`LOADED_REF_DATA`, its water reference `pCurrentWaterObject` +0); a reference's cell
/// +0x40 (`REFR_CELL`, below); a placeable water's water form +0x4C (`BGSPlaceableWater`'s
/// loader casts the form it reads there); a water form's other water form +0x80 (`004FB070`,
/// which the cell's own lookup
/// `00547770` follows) and its radiation +0x188 (JIP reads the byte; the exe's `00778950` reads
/// the word); `TESObjectCELL::GetWaterType` `00547770` (`thiscall`).
const SWIMMING: u32 = 0x14d;
const LOADED_DATA: u32 = 0x64;
const LOADED_WATER_REF: u32 = 0x00;
const PLACEABLE_WATER_FORM: u32 = 0x4c;
const WATER_OTHER: u32 = 0x80;
const WATER_RADIATION: u32 = 0x188;
const CELL_WATER_TYPE: u32 = 0x0054_7770;
/// The game settings `fSwimRadiationDamageMult` and `fWadeRadiationDamageMult` (a setting's
/// value at +4, as `00403E20` reads it; the game's own water radiation in `Actor::Update`,
/// `0088A98E`, reads the two).
const SWIM_RADIATION_MULT: u32 = 0x011d_0460;
const WADE_RADIATION_MULT: u32 = 0x011d_1464;
const SETTING_VALUE: u32 = 0x04;
/// The loaded references (`*0x11C95C8`, made once by `InitTES`; JIP keeps the pointer from its
/// start-up) and their radiation emitters' map +0x90 (`009C1A50`, walked by the game's own
/// `009777A0`): an `NiTPtrMap` (bucket count +4, buckets +8; an entry: next +0, key +4, the
/// reference +8). An emitter's `ExtraRadius` (0x5C) and `ExtraRadiation` (0x5D, `00422450`),
/// each its value at +0xC.
const LOADED_REFERENCES: u32 = 0x011c_95c8;
const RADIATION_EMITTERS: u32 = 0x90;
const MAP_BUCKET_COUNT: u32 = 0x04;
const MAP_BUCKETS: u32 = 0x08;
const MAP_ENTRY_NEXT: u32 = 0x00;
const MAP_ENTRY_DATA: u32 = 0x08;
const EXTRA_RADIUS: u32 = 0x5c;
const EXTRA_RADIATION: u32 = 0x5d;
const EXTRA_FLOAT: u32 = 0x0c;
/// `kAVCode_RadResist` (0x14): `Actor::GetRadiationResistanceMult` `008C4330` reads it through
/// the actor's `ActorValueOwner` (+0xA4, its `GetActorValue` +0xC).
const AV_RAD_RESIST: u32 = 0x14;
/// A high process's radiation (HighProcess vtable `01087864`): its +0x75C (`008D89C0`) answers
/// the float +0x238, its +0x764 (`008D8A00`) the float +0x23C, and `GetRadsSec` +0x76C
/// (`008D97A0`); the game's `GetRadiationLevel` condition (`005A50F0`) adds the three.
const P_RADS: u32 = 0x238;
const P_WATER_RADS: u32 = 0x23c;
const GET_RADS_SEC: u32 = 0x76c;

// Translated from JIP LN NVSE 57.30 (5a30ac4), nvse/GameObjects.cpp: Actor::GetRadiationLevel.
/// The radiation an actor takes where it stands: in water, the water form's radiation times the
/// swim or wade setting (the water form from the actor's water reference, else its cell's;
/// its other water form when it has one); plus each loaded emitter's radiation, scaled by how
/// far inside its radius the actor is; all of it less the actor's radiation resistance (in
/// percent). Single-precision floats in JIP's order. The emitters' map walk is bounded
/// ([`list_step`]).
fn actor_radiation_level(e: &mut Engine, actor: u32) -> f32 {
    let mut result = 0.0f32;
    if e.mem.u8(actor + IN_WATER) != 0 {
        let loaded = e.mem.u32(actor + LOADED_DATA);
        let water_ref = if loaded != 0 {
            e.mem.u32(loaded + LOADED_WATER_REF)
        } else {
            0
        };
        let water = if water_ref != 0 {
            let base = e.mem.u32(water_ref + BASE);
            let water = h::rd32(e, base.wrapping_add(PLACEABLE_WATER_FORM)).unwrap_or(0);
            if water != 0 && e.mem.u32(water + WATER_OTHER) != 0 {
                e.mem.u32(water + WATER_OTHER)
            } else {
                water
            }
        } else {
            let cell = e.mem.u32(actor + REFR_CELL);
            e.call(CELL_WATER_TYPE, &args![cell]).u32()
        };
        if water != 0 && e.mem.u8(water + WATER_RADIATION) != 0 {
            let setting = if e.mem.u8(actor + SWIMMING) != 0 {
                SWIM_RADIATION_MULT
            } else {
                WADE_RADIATION_MULT
            };
            let mult = e.mem.f32(setting + SETTING_VALUE);
            result = f32::from(e.mem.u8(water + WATER_RADIATION)) * mult;
        }
    }
    let collection = e.mem.u32(LOADED_REFERENCES);
    if collection == 0 {
        // Before `InitTES` makes it: JIP's null pointer would stop the game; nothing to walk.
        return finish_radiation(e, actor, result);
    }
    let map = collection + RADIATION_EMITTERS;
    let buckets = e.mem.u32(map + MAP_BUCKETS);
    let count = e.mem.u32(map + MAP_BUCKET_COUNT);
    let mut walked = 0usize;
    for b in 0..count {
        if !list_step(e, &mut walked) {
            break;
        }
        let Some(mut entry) = h::rd32(e, buckets.wrapping_add(4u32.wrapping_mul(b))) else {
            break;
        };
        while entry != 0 {
            if !list_step(e, &mut walked) {
                return finish_radiation(e, actor, result);
            }
            let (Some(refr), Some(next)) = (
                h::rd32(e, entry.wrapping_add(MAP_ENTRY_DATA)),
                h::rd32(e, entry.wrapping_add(MAP_ENTRY_NEXT)),
            ) else {
                return finish_radiation(e, actor, result);
            };
            entry = next;
            if refr == 0 {
                continue;
            }
            let radius = extra::refr_extra(e, refr, EXTRA_RADIUS);
            if radius == 0 {
                continue;
            }
            let r = e.mem.f32(radius + EXTRA_FLOAT);
            let distance = r - super::ln_fn_math::distance_3d(e, actor, refr);
            if distance > 0.0 {
                let radiation = extra::refr_extra(e, refr, EXTRA_RADIATION);
                if radiation != 0 {
                    result += e.mem.f32(radiation + EXTRA_FLOAT) * distance / r;
                }
            }
        }
    }
    finish_radiation(e, actor, result)
}

/// JIP's last step of `GetRadiationLevel`: a non-zero result times one less the actor's
/// radiation resistance in hundredths.
fn finish_radiation(e: &mut Engine, actor: u32, result: f32) -> f32 {
    if result != 0.0 {
        let resist = e
            .vcall(actor + ACTOR_AV_OWNER, GET_ACTOR_VALUE, &[AV_RAD_RESIST])
            .f32();
        result * (1.0 - resist * 0.01)
    } else {
        result
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_actor.h: GetRadiationLevelAlt.
/// An actor with a high process (level 0): for the player, the process's water radiation plus
/// its radiation plus `GetRadsSec`; for another actor, [`actor_radiation_level`]; else 0.
fn radiation_level_alt(e: &mut Engine, actor: u32) -> f32 {
    let p = process(e, actor);
    if p == 0 || level(e, p) != 0 {
        return 0.0;
    }
    if ref_id(e, actor) == PLAYER_ID {
        let sum = e.mem.f32(p + P_WATER_RADS) + e.mem.f32(p + P_RADS);
        sum + e.vcall(p, GET_RADS_SEC, &[]).f32()
    } else {
        actor_radiation_level(e, actor)
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_actor.h:
// Cmd_GetRadiationLevelAlt_Execute.
fn get_radiation_level_alt(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let v = radiation_level_alt(e, a.this_obj);
    a.set_result(e, f64::from(v));
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_actor.h:
// Cmd_GetRadiationLevelAlt_Eval.
fn get_radiation_level_alt_eval(e: &mut Engine, w: &[u32]) -> Ret {
    let a = EvalArgs::new(w);
    let v = radiation_level_alt(e, a.this_obj);
    e.mem.set_f64(a.result, f64::from(v));
    handled(true)
}

// ---- references ----

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_actor.h:
// Cmd_GetFollowers_Execute.
fn get_followers(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    if is_actor(e, a.this_obj) {
        let x = extra::refr_extra(e, a.this_obj, EXTRA_FOLLOWER);
        if x != 0 {
            let list = e.mem.u32(x + EXTRA_WORD);
            if list != 0 {
                let found: Vec<u32> = items(e, list).into_iter().filter(|&f| f != 0).collect();
                array_result(e, &a, &found);
            }
        }
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_actor.h:
// Cmd_GetDroppedRefs_Execute.
fn get_dropped_refs(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    if is_actor(e, a.this_obj) {
        let x = extra::refr_extra(e, a.this_obj, EXTRA_DROPPED_ITEM_LIST);
        if x != 0 {
            let found: Vec<u32> = items(e, x + EXTRA_WORD)
                .into_iter()
                .filter(|&f| f != 0)
                .collect();
            array_result(e, &a, &found);
        }
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_actor.h:
// Cmd_FaceObject_Execute; nvse/GameObjects.cpp: Actor::TurnToFaceObject (asm).
/// The game's turn toward the target's position.
fn face_object(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if is_actor(e, a.this_obj) && read.ok {
        let t = read.form(0);
        let (x, y, z) = (
            e.mem.u32(t + POS),
            e.mem.u32(t + POS + 4),
            e.mem.u32(t + POS + 8),
        );
        e.call(TURN_TO_POINT, &args![a.this_obj, x, y, z, 0u32]);
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_actor.h: Cmd_Turn_Execute;
// nvse/GameObjects.cpp: Actor::TurnAngle (asm).
/// The game's turn to the actor's z rotation plus the angle in degrees (`mulss` by
/// `PS_V3_PId180`, then `addss`: two float roundings).
fn turn(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if is_actor(e, a.this_obj) && read.ok {
        let angle = read.float(0, 0.0) * super::FLT_PI_D_180 + e.mem.f32(a.this_obj + ROT_Z);
        e.call(TURN_TO_ANGLE, &args![a.this_obj, angle.to_bits(), 0u32]);
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_actor.h:
// Cmd_GetAshPileSource_Execute.
/// An activator reference's `ExtraAshPileRef` source.
fn get_ash_pile_source(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    if a.this_obj != 0 {
        let base = e.mem.u32(a.this_obj + BASE);
        if base != 0 && type_of(e, base) == TES_OBJECT_ACTI {
            let x = extra::refr_extra(e, a.this_obj, EXTRA_ASH_PILE_REF);
            if x != 0 {
                let src = e.mem.u32(x + EXTRA_WORD);
                if src != 0 {
                    set_form_result(e, &a, src);
                }
            }
        }
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), nvse/GameObjects.cpp: Actor::GetAmmoInfo (asm).
/// A middle-high or high process's ammo entry, 0 otherwise.
fn ammo_info(e: &Engine, actor: u32) -> u32 {
    let p = process(e, actor);
    if p != 0 && level(e, p) <= 1 {
        e.mem.u32(p + P_AMMO)
    } else {
        0
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_actor.h:
// Cmd_GetCurrentAmmo_Execute.
fn get_current_ammo(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let entry = ammo_info(e, a.this_obj);
    if entry != 0 {
        let t = e.mem.u32(entry + ENTRY_TYPE);
        if t != 0 {
            set_form_result(e, &a, t);
        }
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_actor.h:
// Cmd_GetCurrentAmmoRounds_Execute.
fn get_current_ammo_rounds(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let entry = ammo_info(e, a.this_obj);
    if entry != 0 {
        let n = e.mem.u32(entry + ENTRY_COUNT) as i32;
        a.set_result(e, f64::from(n));
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_actor.h:
// Cmd_SetFullNameAlt_Execute.
/// The name of an NPC or creature (its full name +0xD0; marked changed with 0x20), a talking
/// activator (its full name component) or a cell (+0x18; both marked with 4); the form argument,
/// else the calling reference's base form.
fn set_full_name_alt(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if !read.ok {
        return handled(true);
    }
    let mut form = read.form(1);
    if form == 0 {
        if a.this_obj == 0 {
            return handled(true);
        }
        form = e.mem.u32(a.this_obj + BASE);
        if form == 0 {
            return handled(true);
        }
    }
    let (name, flag) = match type_of(e, form) {
        TES_NPC | TES_CREATURE => (form + 0xd0, 0x20),
        BGS_TALKING_ACTIVATOR => (full_name(e, form), 4),
        TES_OBJECT_CELL => (form + 0x18, 4),
        _ => return handled(true),
    };
    if name == 0 {
        return handled(true);
    }
    let text = buffer_arg(e, &read, 0, 0x100, "SetFullNameAlt");
    string_set(e, name + 4, &text);
    extra::mark_modified(e, form, flag);
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_actor.h:
// Cmd_GetDetectionValue_Execute.
fn get_detection_value(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if is_actor(e, a.this_obj) && read.ok {
        let v = detection_value(e, a.this_obj, read.form(0));
        a.set_result(e, f64::from(v));
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_actor.h:
// Cmd_SetSpeedMult_Execute.
/// A speed multiplier above 0 set (`SetActorValueInt(0x15, ...)`); then, in any case, the
/// process's cached walk and run speeds dropped.
fn set_speed_mult(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if !is_actor(e, a.this_obj) || !read.ok {
        return handled(true);
    }
    let mult = read.int(0, 0);
    if mult != 0 {
        e.vcall(a.this_obj, SET_ACTOR_VALUE_INT, &[0x15, mult]);
    }
    let p = e.mem.u32(a.this_obj + PROCESS);
    if p != 0 {
        let cached = e.mem.u32(p + P_CACHED);
        if cached != 0 {
            let f = e.mem.u32(cached + CACHED_FLAGS);
            e.mem.set_u32(cached + CACHED_FLAGS, f & !CACHED_SPEEDS);
        }
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), nvse/GameObjects.cpp: Actor::GetKnockedState.
fn ragdolled(e: &Engine, actor: u32) -> bool {
    let p = process(e, actor);
    p != 0 && level(e, p) <= 1 && e.mem.u8(p + P_KNOCKED) as i8 == 1
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_actor.h:
// Cmd_GetIsRagdolled_Execute.
fn get_is_ragdolled(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    if ragdolled(e, a.this_obj) {
        a.set_result(e, 1.0);
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_actor.h:
// Cmd_GetIsRagdolled_Eval.
fn get_is_ragdolled_eval(e: &mut Engine, w: &[u32]) -> Ret {
    let a = EvalArgs::new(w);
    if ragdolled(e, a.this_obj) {
        e.mem.set_f64(a.result, 1.0);
    }
    handled(true)
}

// ---- effects, perks, hits ----

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_actor.h: GetIsPoisoned.
/// An active effect of spell type 5 (`ActiveEffect +0x2C`).
fn poisoned(e: &mut Engine, actor: u32) -> bool {
    is_actor(e, actor)
        && effects(e, actor)
            .into_iter()
            .any(|fx| fx != 0 && e.mem.u32(fx + ACTIVE_EFFECT_SPELL_TYPE) == 5)
}

/// `ActiveEffect::spellType` (JIP LN NVSE, `// 2C`).
const ACTIVE_EFFECT_SPELL_TYPE: u32 = 0x2c;

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_actor.h:
// Cmd_GetIsPoisoned_Execute.
fn get_is_poisoned(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    if poisoned(e, a.this_obj) {
        a.set_result(e, 1.0);
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_actor.h:
// Cmd_GetIsPoisoned_Eval.
fn get_is_poisoned_eval(e: &mut Engine, w: &[u32]) -> Ret {
    let a = EvalArgs::new(w);
    if poisoned(e, a.this_obj) {
        e.mem.set_f64(a.result, 1.0);
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_actor.h:
// Cmd_GetActorValueModifier_Execute.
/// The magnitudes (a float sum) of the active, not terminated effects with an effect item whose
/// base effect is of archetype 0, flag 2, and the actor value; `duration` filters by bits (1:
/// effects with no duration, 2: with one; default 3). Echoed in the console.
fn get_actor_value_modifier(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if !is_actor(e, a.this_obj) || !read.ok {
        return handled(true);
    }
    let (av, duration) = (read.short(0, 0), read.int(1, 3));
    let first = e
        .vcall(a.this_obj + ACTOR_MAGIC_TARGET, GET_EFFECT_LIST, &[])
        .u32();
    if first == 0 {
        return handled(true);
    }
    let mut modifier = 0.0f32;
    for fx in items(e, first) {
        let f = Ptr::<A>::new(fx);
        if fx == 0
            || e.get(f, A::bActive) == 0
            || e.get(f, A::bTerminated) != 0
            || e.get(f, A::effectItem) == 0
        {
            continue;
        }
        let setting = e.mem.u32(e.get(f, A::effectItem) + 0x14);
        if setting == 0 {
            continue;
        }
        let s = Ptr::<EffectSetting>::new(setting);
        let bits = if e.get(f, A::duration) != 0.0 { 2 } else { 1 };
        if e.get(s, EffectSetting::archtype) != 0
            || u32::from(e.get(s, EffectSetting::actorVal)) != av
            || e.get(s, EffectSetting::effectFlags) & 2 == 0
            || bits & duration == 0
        {
            continue;
        }
        modifier += e.get(f, A::magnitude);
    }
    a.set_result(e, f64::from(modifier));
    crate::console::print_result(e, &a);
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_actor.h:
// Cmd_GetPerkModifier_Execute.
/// The base value through the game's `ApplyPerkModifiers` for the entry point, with as many
/// filter forms as the entry point's tab count asks (1: none, 2: one, 3: two; any other: no
/// call); the value echoed in the console.
fn get_perk_modifier(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let entry = read.int(0, 0);
    if !is_actor(e, a.this_obj) || !read.ok || entry >= PERK_ENTRY_MAX {
        return handled(true);
    }
    let base = read.float(1, 0.0);
    let (f1, f2) = (read.form(2), read.form(3));
    let tabs = e.mem.u8(ENTRY_POINT_INFO + 0x10 * entry + 4);
    let this = a.this_obj;
    let value = e.with_stack(4, |e, at| {
        let v = at.addr();
        e.mem.set_f32(v, base);
        match tabs {
            1 => {
                e.call(APPLY_PERK_MODIFIERS, &args![entry, this, v]);
            }
            2 => {
                e.call(APPLY_PERK_MODIFIERS, &args![entry, this, f1, v]);
            }
            3 => {
                e.call(APPLY_PERK_MODIFIERS, &args![entry, this, f1, f2, v]);
            }
            _ => {}
        }
        e.mem.f32(v)
    });
    a.set_result(e, f64::from(value));
    crate::console::print_result(e, &a);
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_actor.h:
// Cmd_GetHitHealthDamage_Execute; nvse/GameObjects.cpp: Actor::GetHitDataValue (case 0),
// AdjustDmgByDifficulty (asm).
/// A middle-high or high process's last hit: its health damage times the difficulty's
/// multiplier (the player's row when the player was hit, else the other row), as the x87 does it
/// (`fld`, `fmul` of two floats) at the running thread's precision, as `flt_to_str` follows it:
/// on the game's main thread (24-bit precision) the product rounded to a float, the exact double
/// under 53- or 64-bit precision.
fn get_hit_health_damage(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let p = process(e, a.this_obj);
    if p == 0 || level(e, p) > 1 {
        return handled(true);
    }
    let hit = e.mem.u32(p + P_LAST_HIT);
    if hit != 0 {
        let player = e.mem.u32(PLAYER);
        let row = if e.mem.u32(hit + HIT_TARGET) == player {
            DIFFICULTY_MULTS
        } else {
            DIFFICULTY_MULTS + 0x14
        };
        let difficulty = e.mem.u32(player + PLAYER_DIFFICULTY);
        let setting = e.mem.u32(row.wrapping_add(difficulty.wrapping_mul(4)));
        let mult = e.mem.f32(setting + 4);
        let damage = e.mem.f32(hit + HIT_HEALTH);
        let value = engine_core::x87::x87_mul(f64::from(damage), f64::from(mult));
        a.set_result(e, value);
    }
    handled(true)
}

// ---- the backup package (lane C1) ----

/// `Actor::AddBackUpPackage`'s pieces, each confirmed in the exe: the process's
/// `GetCurrentPackage` (vtable +0x27C, `0091EEC0`) and a package's type byte (+0x20; 0x1A ends
/// it); the game's own backup packages are 0x8C bytes (`CreatePackage` `00670DC5` allocates 0x8C
/// for its `009ED030`); the constructor `BackUpPackage::BackUpPackage` (`009ED030`, no words) and
/// `InitializeBackUpPackage` (`009ED0D0`, `(actor, cell, target)`, `RET 0xC`); its fields: the
/// procedure +0x18 (0x30), the flags +0x1C, the backup point +0x80 (`backuppackage.rs`); then the
/// process's vtable +0x28 (`HighProcess`' `00907650`, no words) and the actor's vtable +0x2F4
/// (`0087EAC0`, `(package, 1, 1)`, `RET 0xC`: the actor put on the package).
const GET_CURRENT_PACKAGE: u32 = 0x27c;
const PACKAGE_TYPE_BYTE: u32 = 0x20;
const BACKUP_SIZE: u32 = 0x8c;
const BACKUP_CTOR: u32 = 0x009e_d030;
const BACKUP_INIT: u32 = 0x009e_d0d0;
const PACKAGE_PROCEDURE: u32 = 0x18;
const PACKAGE_FLAGS: u32 = 0x1c;
const BACKUP_POINT: u32 = 0x80;
const PROCESS_SLOT_28: u32 = 0x28;
const ACTOR_SET_PACKAGE: u32 = 0x2f4;
/// `kPackageFlag_ContinueDuringCombat` (`1 << 26`), a reference's cell (+0x40).
const CONTINUE_DURING_COMBAT: u32 = 1 << 26;
const REFR_CELL: u32 = 0x40;

// Translated from JIP LN NVSE 57.30 (5a30ac4), nvse/GameObjects.cpp: Actor::AddBackUpPackage
// (asm).
/// For an actor not in life state 3 or 5, with a high process (level 0) not running a package of
/// type 0x1A: a new backup package (0x8C bytes) away from `target` (its point the target's
/// position when there is no cell), procedure 0x30, `flags` added, the process told
/// (+0x28) and the actor put on it (+0x2F4). JIP then sets `s_moveAwayDistance` back to 300,
/// which only its move-away patch (row G42, not built) reads: the exe's own distance is that
/// same 300, so nothing is owed here.
fn add_back_up_package(e: &mut Engine, actor: u32, target: u32, cell: u32, flags: u32) {
    let life = e.mem.u32(actor + LIFE_STATE);
    if life == 3 || life == 5 {
        return;
    }
    let p = process(e, actor);
    if p == 0 || e.mem.u8(p + P_LEVEL) != 0 {
        return;
    }
    let running = e.vcall(p, GET_CURRENT_PACKAGE, &args![]).u32();
    if running != 0 && e.mem.u8(running + PACKAGE_TYPE_BYTE) == 0x1a {
        return;
    }
    let block = heap_alloc(e, BACKUP_SIZE);
    let package = e.call(BACKUP_CTOR, &args![block]).u32();
    e.call(BACKUP_INIT, &args![package, actor, cell, target]);
    if cell == 0 {
        for i in 0..3 {
            let v = e.mem.u32(target + POS + 4 * i);
            e.mem.set_u32(package + BACKUP_POINT + 4 * i, v);
        }
    }
    e.mem.set_u32(package + PACKAGE_PROCEDURE, 0x30);
    let old = e.mem.u32(package + PACKAGE_FLAGS);
    e.mem.set_u32(package + PACKAGE_FLAGS, old | flags);
    e.vcall(p, PROCESS_SLOT_28, &args![]);
    e.vcall(actor, ACTOR_SET_PACKAGE, &args![package, 1u32, 1u32]);
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_actor.h:
// Cmd_MoveAwayFromPlayer_Execute.
/// An actor backs away from the player ([`add_back_up_package`] with its own cell and "continue
/// during combat"). No result.
fn move_away_from_player(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    if !is_actor(e, a.this_obj) {
        return handled(true);
    }
    let player = e.mem.u32(PLAYER);
    if player == 0 {
        return handled(true);
    }
    let cell = e.mem.u32(a.this_obj + REFR_CELL);
    add_back_up_package(e, a.this_obj, player, cell, CONTINUE_DURING_COMBAT);
    handled(true)
}

// ---- the character controller (lane C1) ----

/// `bhkCharacterController` fields (Xbox PDB names, engine-havok `bhkcharactercontroller.rs`),
/// each confirmed in the exe: the state `m_currentState` +0x3F0 (`UpdateState` `00C6CBA0` stores
/// the wanted state there; JIP's `chrContext.hkState`), the velocity `OutVelocity` +0x4F0 (four
/// floats; `bhkCharacterStateInAir::Update` `00CD3FB0` loads and stores it with `MOVAPS`), the
/// pitch timer `fCalculatePitchTimer` +0x530 (`00C70060` tests it), the fall's starting height
/// `fFallStartHeight` +0x544 (`GetFallDistance` `00C70550` reads it), the fall time `fFallTime`
/// +0x548 (`00C6E2E0` adds the step time to it), the body under the feet `spSupportBody` +0x60C
/// (lane G's `havok_refs::SUPPORT_BODY`).
pub(crate) const CC_STATE: u32 = 0x3f0;
pub(crate) const CC_VELOCITY: u32 = 0x4f0;
const CC_PITCH_TIMER: u32 = 0x530;
const CC_START_HEIGHT: u32 = 0x544;
const CC_FALL_TIME: u32 = 0x548;
/// A reference's position z (+0x38).
const POS_Z: u32 = 0x38;

/// The controller of the reference (`TESObjectREFR::GetCharacterController`), 0 for none.
pub(crate) fn controller(e: &Engine, refr: u32) -> u32 {
    if refr == 0 {
        return 0;
    }
    h::character_controller(e, refr)
}

/// The float at `at` of the reference's controller, `None` without a controller (or when it does
/// not read).
fn controller_float(e: &Engine, refr: u32, at: u32) -> Option<f32> {
    let ctrl = controller(e, refr);
    (ctrl != 0)
        .then(|| h::rdf32(e, ctrl.wrapping_add(at)))
        .flatten()
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_actor.h:
// Cmd_GetFallTimeElapsed_Execute.
/// The controller's fall time (+0x548); nothing without a controller.
fn get_fall_time_elapsed(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    if let Some(v) = controller_float(e, a.this_obj, CC_FALL_TIME) {
        a.set_result(e, f64::from(v));
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_actor.h:
// Cmd_GetFallTimeRemaining_Execute.
/// The controller's pitch timer (+0x530, JIP's `calculatePitchTimer`); nothing without a
/// controller.
fn get_fall_time_remaining(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    if let Some(v) = controller_float(e, a.this_obj, CC_PITCH_TIMER) {
        a.set_result(e, f64::from(v));
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_actor.h:
// Cmd_ResetFallTime_Execute.
/// The fall's starting height set to the reference's z and the fall time to 0 (two plain stores
/// the controller's own step reads; nothing is woken). No result.
fn reset_fall_time(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let ctrl = controller(e, a.this_obj);
    if ctrl != 0 {
        if let Some(z) = h::rd32(e, a.this_obj.wrapping_add(POS_Z)) {
            let at = ctrl.wrapping_add(CC_START_HEIGHT);
            if e.mem.try_write(at, &z.to_le_bytes()).is_ok() {
                let _ = e
                    .mem
                    .try_write(ctrl.wrapping_add(CC_FALL_TIME), &0f32.to_le_bytes());
            }
        }
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_actor.h:
// Cmd_GetActorVelocity_Execute.
/// The controller's velocity on the axis (X, Y, Z), or with no axis its length (`Length_V4` of
/// all four lanes: `(x*x + y*y) + (z*z + w*w)`, then the estimate's Newton step).
fn get_actor_velocity(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if !read.ok {
        return handled(true);
    }
    let axis = super::jip_fn_misc_ref_havok::axis_arg(&read, 0);
    let ctrl = controller(e, a.this_obj);
    let Some(v) = (ctrl != 0)
        .then(|| h::vec4(e, ctrl.wrapping_add(CC_VELOCITY)))
        .flatten()
    else {
        return handled(true);
    };
    if axis == 0 {
        let s = (v[0] * v[0] + v[1] * v[1]) + (v[2] * v[2] + v[3] * v[3]);
        a.set_result(e, f64::from(super::ln_fn_math::newton_sqrt(s)));
    } else if let Some(i) = axis.checked_sub(b'X').filter(|&i| i < 3) {
        a.set_result(e, f64::from(v[usize::from(i)]));
    }
    handled(true)
}

/// Whether the reference's controller state has bit 1 (JIP's `chrContext.hkState & 2`).
fn in_air(e: &Engine, refr: u32) -> bool {
    let ctrl = controller(e, refr);
    ctrl != 0 && h::rd32(e, ctrl.wrapping_add(CC_STATE)).is_some_and(|s| s & 2 != 0)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_actor.h:
// Cmd_IsInAir_Execute.
fn is_in_air(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    if in_air(e, a.this_obj) {
        a.set_result(e, 1.0);
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_actor.h: Cmd_IsInAir_Eval.
fn is_in_air_eval(e: &mut Engine, w: &[u32]) -> Ret {
    let a = EvalArgs::new(w);
    if in_air(e, a.this_obj) {
        e.mem.set_f64(a.result, 1.0);
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_actor.h:
// Cmd_GetObjectUnderFeet_Execute.
/// The reference of the body under the controller's feet (`hkpWorldObject::GetParentRef`; the
/// support count is not consulted here, unlike the contact commands); nothing for none.
fn get_object_under_feet(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let ctrl = controller(e, a.this_obj);
    let Some(body) = (ctrl != 0)
        .then(|| h::ptr(e, ctrl.wrapping_add(h::SUPPORT_BODY)))
        .flatten()
    else {
        return handled(true);
    };
    let refr = h::object_parent_ref(e, body, &mut h::Walk::new());
    if let Some(id) = (refr != 0)
        .then(|| h::rd32(e, refr.wrapping_add(h::REF_ID)))
        .flatten()
    {
        a.set_form_result(e, id);
    }
    handled(true)
}

#[cfg(test)]
#[path = "jip_fn_actor_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "jip_fn_actor_c3_tests.rs"]
mod c3_tests;
