//! `functions_jip/jip_fn_actor.h` (JIP LN NVSE), second half (lane A2: the file's 71st `DEFINE`
//! line to its end, `GetHitLimbDamage` to `GetAllPerks`): the last hit's data, actor bases'
//! spells, skeletons, death items, weapon lists, impact materials, sound templates, VATS targets
//! and factions, and an actor's killer and kill XP, packages, fleeing, ammo, path, movement,
//! equipped items, idles, water depth, weapon drawn and crippled limbs; and (lane C1) the
//! character controller's tilt, gravity and velocity, the ground's material, the last hit's
//! node, VATS targetability, `AttachAshPileEx`, the player's 3D, the push away from a point and
//! a limb dismembered. 53 of its 71 commands; the first 70 lines are lane A1's module.
//!
//! Built elsewhere already (the core track's hook files): `SetOnHitEventHandler`,
//! `SetOnReloadWeaponEventHandler`, `SetOnRagdollEventHandler` (`actor_hooks.rs`) and
//! `PushActorNoRagdoll` (`world_hooks.rs`, `world_push.rs`).
//!
//! Left for later, each with what it waits for:
//! - a game patch of stage 3.9 (`measure/jip/PLAN-3.9.md`): `MoveAwayFrom` (G42: the backup package reads
//!   `s_moveAwayDistance`; H16: JIP's 0x8C package), `TravelToRef` (H16: `AddBackUpPackage`
//!   allocates JIP's enlarged 0x8C package), `ToggleNoGunWobble` (H19), `ToggleTeammateKillable`
//!   (the killable-teammates patch of 3.9o), `Get/SetExcludedCombatActions` (H16: the combat
//!   controller's added mask and JIP's per-reference map), `RemoveAllPerks` and `GetAllPerks` (O18
//!   NPC perks: their NPC half reads the patch's state and JIP's perk maps), `GetArmourPenetrated`
//!   (hit flag 0x80000000 is set only by JIP's own hit-damage patch, `patches_game.h`, G77);
//! - a hook: `FireWeaponEx` (writes the exe's code at `009BAD76` and `01087F1C` and queues a
//!   call);
//! - the 3D scene and physics (not ported to this layer): `ReloadEquippedModels`,
//!   `RefreshAnimData`, `ReloadCharController`;
//! - not reached by lane C1 (nothing missing; the next agent): `PlayIdleEx` (the idle directory's
//!   pick `00600950`, `TESIdleForm::FindIdle`'s condition walk, `AnimData::PlayIdle` `00497F20`).
//!
//! Layouts (JIP's headers; the exe where the audit table says so): a reference's base form +0x20,
//! position z +0x38, cell +0x40, extra data list +0x44; an actor's process +0x68, killer +0xC0,
//! life state +0x108, the byte +0x14C JIP tests before the water depth, its mover +0x190 (path
//! request +0x1C, path solution +0x20; `GetMovementFlags` its vtable +0x20); a process's level
//! +0x28, ammo entry +0x118, last hit data +0x240 (`MiddleHighProcess::GetLastHitData` `00521630`,
//! the process's vtable +0x770), `GetInterruptPackage` +0x20C, `GetCurrentPackage` +0x27C; a
//! package's type +0x20; the hit data's attacker +0, projectile +8, base weapon damage +0x18,
//! fatigue damage +0x1C, limb damage +0x20, weapon +0x30, impact position +0x38, impact angle
//! +0x44, flags +0x58; an actor base's death item +0x4C (`INAM` in `TESCreature`'s loader
//! `005F7EC0`), template flags +0x4A, template actor +0x54, factions +0x5C (8-byte `{faction,
//! rank}`), spells +0x80, model +0xDC (`MODL` in both loaders); a creature's sounds template
//! +0x128 (its setter `005F9140`), body part data +0x144, material +0x148 (`NAM4`), weapon list
//! +0x158; an NPC's impact material +0x1E4 (`NAM4` in `TESNPC`'s loader `00602190`); a biped's
//! slots from +0x2C (20 of 0x10 bytes, the item at +0); an animation data's played idles +0x124
//! and +0x128 (each with its idle form at +0x2C).

use super::arrays::{create_array, Element};
use super::extra;
use super::forms::{ListNode, TES_OBJECT_ARMO_VTBL};
use super::havok_refs as h;
use super::ln_fn_actor::{actor_base, changes_list, equipped_extra};
use super::{
    heap_alloc, is_actor, is_type, list_contains, list_items, list_prepend, list_remove,
    model_path, set_form_result, string_set, type_of, Handlers,
};
use crate::console::{print_form, print_result};
use engine_core::prelude::*;
use nvse::commands::Read;
use nvse::{handled, CommandArgs, EvalArgs};
use std::collections::BTreeSet;

#[rustfmt::skip]
pub static COMMANDS: Handlers = &[
    (0x2803, get_hit_limb_damage),
    (0x2804, cripple_limb),
    (0x2809, get_kill_xp),
    (0x280b, get_killer),
    (0x280c, kill_actor_alt),
    (0x2816, get_played_idle),
    (0x2821, set_weapon_out),
    (0x2823, add_base_effect_list_effect),
    (0x2824, remove_base_effect_list_effect),
    (0x282c, get_hit_attacker),
    (0x282d, get_hit_projectile),
    (0x282e, get_hit_weapon),
    (0x272a, get_interrupt_package),
    (0x283e, set_current_ammo_rounds),
    (0x2845, get_skeleton_model),
    (0x2846, set_npc_skeleton_model),
    (0x2847, set_cre_skeleton_model),
    (0x2857, get_creature_weapon_list),
    (0x2868, get_death_item),
    (0x2869, set_death_item),
    (0x286a, get_actor_leveled_list),
    (0x2884, get_impact_material_type),
    (0x2885, set_impact_material_type),
    (0x2895, donner_reed_kuru_party),
    (0x2897, get_equipped_ex),
    (0x2898, test_equipped_slots),
    (0x28a6, get_factions),
    (0x28a8, get_hit_3d_data),
    (0x28bb, get_creature_sounds_template),
    (0x28bc, set_creature_sounds_template),
    (0x28cf, get_body_part_vats_target),
    (0x28ee, get_hit_extended_flag),
    (0x2908, get_actor_movement_flags),
    (0x290a, get_hit_base_weapon_damage),
    (0x290b, get_hit_fatigue_damage),
    (0x28db, get_actor_tilt_angle),
    (0x28dc, set_actor_tilt_angle),
    (0x28e7, get_actor_gravity_mult),
    (0x28e8, set_actor_gravity_mult),
    (0x2842, set_actor_velocity),
    (0x2912, get_actor_velocity_alt),
    (0x28d2, get_hit_node),
    (0x2854, set_vats_targetable),
    (0x2840, attach_ash_pile_ex),
    (0x28c7, update_player_3d),
    (0x288a, push_actor_away_alt),
    (0x2833, dismember_limb),
];

#[rustfmt::skip]
pub static CONDITIONS: super::Conditions = &[
    (0x2817, is_idle_playing_ex, is_idle_playing_ex_eval),
    (0x2830, get_water_immersion_perc, get_water_immersion_perc_eval),
    (0x283c, is_fleeing, is_fleeing_eval),
    (0x2849, get_target_unreachable, get_target_unreachable_eval),
    (0x28d9, get_in_faction_list, get_in_faction_list_eval),
    (0x28e0, get_ground_material, get_ground_material_eval),
];

// ---- layouts ----

const REFR_BASE: u32 = 0x20;
const REFR_POS_Z: u32 = 0x38;
const REFR_CELL: u32 = 0x40;
const ACTOR_PROCESS: u32 = 0x68;
const ACTOR_KILLER: u32 = 0xc0;
const ACTOR_LIFE_STATE: u32 = 0x108;
/// The byte JIP's `GetWaterImmersionPerc` tests before the depth (`[ecx+0x14C]`).
const ACTOR_BYTE_14C: u32 = 0x14c;
const ACTOR_MOVER: u32 = 0x190;
const MOVER_PATH_REQUEST: u32 = 0x1c;
const MOVER_PATH_SOLUTION: u32 = 0x20;
/// `ActorMover::GetMovementFlags` (its vtable +0x20; `009DDB70` in `ActorMover`'s `0109223C`).
const MOVER_GET_MOVEMENT_FLAGS: u32 = 0x20;
const PROCESS_LEVEL: u32 = 0x28;
const PROCESS_AMMO_INFO: u32 = 0x118;
const PROCESS_LAST_HIT_DATA: u32 = 0x240;
/// `BaseProcess::GetInterruptPackage`, `GetCurrentPackage`, `GetHitData` (vtable slots;
/// `HighProcess`' `01087864`: +0x770 is `MiddleHighProcess::GetLastHitData` `00521630`, +0x27C
/// `GetPackageThatIsRunning` `0091EEC0`, which asks +0x20C first).
const PROCESS_GET_INTERRUPT_PACKAGE: u32 = 0x20c;
const PROCESS_GET_CURRENT_PACKAGE: u32 = 0x27c;
const PROCESS_GET_HIT_DATA: u32 = 0x770;
const PACKAGE_TYPE: u32 = 0x20;
const HIT_SOURCE: u32 = 0x00;
const HIT_PROJECTILE: u32 = 0x08;
const HIT_WPN_BASE_DMG: u32 = 0x18;
const HIT_FATIGUE_DMG: u32 = 0x1c;
const HIT_LIMB_DMG: u32 = 0x20;
const HIT_WEAPON: u32 = 0x30;
const HIT_IMPACT_POS: u32 = 0x38;
const HIT_IMPACT_ANGLE: u32 = 0x44;
const HIT_FLAGS: u32 = 0x58;
const BASE_DEATH_ITEM: u32 = 0x4c;
const BASE_TEMPLATE_FLAGS: u32 = 0x4a;
const BASE_TEMPLATE_ACTOR: u32 = 0x54;
const BASE_FACTIONS: u32 = 0x5c;
const BASE_SPELLS: u32 = 0x80;
const BASE_MODEL: u32 = 0xdc;
/// `TESActorBase::GetBodyPartData` (vtable +0x180).
const BASE_GET_BODY_PART_DATA: u32 = 0x180;
const CREATURE_AUDIO_TEMPLATE: u32 = 0x128;
const CREATURE_MATERIAL: u32 = 0x148;
const CREATURE_WEAPON_LIST: u32 = 0x158;
const NPC_IMPACT_MATERIAL: u32 = 0x1e4;
const BODY_PARTS: u32 = 0x34;
const BODY_PART_VATS_TARGET: u32 = 0x0c;
const LIST_FORM_LIST: u32 = 0x18;
const ENTRY_COUNT_DELTA: u32 = 0x04;
const ENTRY_TYPE: u32 = 0x08;
const ARMO_PART_MASK: u32 = 0x74;
const BIPED_SLOTS: u32 = 0x2c;
const ANIM_IDLE_PLAYING: u32 = 0x124;
const ANIM_IDLE_QUEUED: u32 = 0x128;
const IDLE_OF_ANIM: u32 = 0x2c;
/// `ExtraLeveledCreature` (0x2E): the original leveled base at +0xC (batch 11: its setter
/// `00421750`); `ExtraFactionChanges` (0x5E): its list (a `tList*`) at +0xC.
const EXTRA_LEVELED_CREATURE: u32 = 0x2e;
const EXTRA_FACTION_CHANGES: u32 = 0x5e;
/// `ExtraDismemberedLimbs` (0x5F): 0x30 bytes (the exe's `AddDismembermentExtra` asks that much
/// before its constructor `00430200`), the mask +0xC (16 bits), eaten +0x1C.
const EXTRA_DISMEMBERED_LIMBS: u32 = 0x5f;
const DISMEMBERED_SIZE: u32 = 0x30;
const DISMEMBERED_MASK: u32 = 0x0c;
const DISMEMBERED_EATEN: u32 = 0x1c;
const DISMEMBERED_CTOR: u32 = 0x0043_0200;
/// `kChangedFlag` 0x20000 JIP passes to `MarkModified` for the dismembered limbs.
const CHANGED_DISMEMBERED: u32 = 0x2_0000;
/// `TESObjectREFR` virtual functions: `GetAnimData` +0x1E4 (`Actor::GetAnimation` `008B70D0` in
/// `Character`'s `01086A6C`), `GetBipedAnim` +0x1E8 (`005D9F90`: +0x1B4), `IsCharacter` +0x218;
/// `Actor::DamageActorValue` +0x3AC (`00881130`: the value, the damage, the attacker).
const REFR_GET_ANIM_DATA: u32 = 0x1e4;
const REFR_GET_BIPED_ANIM: u32 = 0x1e8;
const REFR_IS_CHARACTER: u32 = 0x218;
const ACTOR_DAMAGE_ACTOR_VALUE: u32 = 0x3ac;
/// `kAVCode_PerceptionCondition`.
const AV_PERCEPTION_CONDITION: u32 = 0x19;
/// The game functions called: `CombatController::IsFleeing` `00981990` (`thiscall`),
/// `Animation::SpecialIdleDonePlaying` `004985F0` (`thiscall`), the water depth `00885560`
/// (`thiscall`: the reference, its z, its cell), `Actor::SetWeaponOut` `008A6840` (`thiscall`:
/// the actor, the bool), the `KillActor` command's execute `005BE2A0`, and for the kill XP:
/// `TESActorBaseData::GetLevel` `0047DED0` (`thiscall`), `ExperiencePoints::GetExperiencePoints`
/// `006705B0` (`cdecl`: 1 for not a creature, the level), the difficulty multiplier `00648C80`
/// (`cdecl`: the XP, the difficulty) and `floor` as a float `00406CE0`.
const COMBAT_IS_FLEEING: u32 = 0x0098_1990;
const SPECIAL_IDLE_DONE_PLAYING: u32 = 0x0049_85f0;
const WATER_IMMERSION: u32 = 0x0088_5560;
const SET_WEAPON_OUT: u32 = 0x008a_6840;
const KILL_ACTOR_EXECUTE: u32 = 0x005b_e2a0;
const GET_LEVEL: u32 = 0x0047_ded0;
const GET_EXPERIENCE_POINTS: u32 = 0x0067_05b0;
const DIFFICULTY_MULT: u32 = 0x0064_8c80;
const FLOOR_FLOAT: u32 = 0x0040_6ce0;
/// `g_thePlayer` and its difficulty (+0x7B8).
const THE_PLAYER: u32 = 0x011d_ea3c;
const PLAYER_DIFFICULTY: u32 = 0x7b8;
/// `kFormType_*`.
const TES_NPC: u8 = 0x2a;
const TES_CREATURE: u8 = 0x2b;
const TES_LEV_CREATURE: u8 = 0x2c;
const TES_LEV_CHARACTER: u8 = 0x2d;
/// `kFormType_Creature`: a creature reference.
const CREATURE_REF: u8 = 0x3c;
/// `GetHitExtendedFlag`'s table (`ActorHitData::kFlag_*`).
const HIT_FLAG_TABLE: [u32; 9] = [1, 4, 0x10, 0x20, 0x40, 0x80, 0x300, 0x400, 0x2000];
/// The most template steps `GetActorModel` takes: JIP walks without end, so a template chain a
/// mod made circular would hang it (a deliberate difference: the walk stops there).
const TEMPLATE_STEPS: u32 = 0x1_0000;

// ---- shared ----

/// The reference's process when it is an actor, else 0.
fn process(e: &Engine, actor: u32) -> u32 {
    if is_actor(e, actor) {
        e.mem.u32(actor + ACTOR_PROCESS)
    } else {
        0
    }
}

/// The process's hit data by `GetHitData` (the virtual function), 0 for none.
fn virtual_hit_data(e: &mut Engine, process: u32) -> u32 {
    e.vcall(process, PROCESS_GET_HIT_DATA, &[]).u32()
}

/// `thisObj->GetAnimData()` (the virtual function), 0 for none or no reference.
fn anim_data(e: &mut Engine, refr: u32) -> u32 {
    if refr == 0 {
        return 0;
    }
    e.vcall(refr, REFR_GET_ANIM_DATA, &[]).u32()
}

/// `IS_ID(form, type)`: a form (not null) of the type.
fn is_id(e: &Engine, form: u32, type_id: u8) -> bool {
    form != 0 && type_of(e, form) == type_id
}

/// The actor base argument `i`, or the calling actor's (`thisObj->baseForm` when `own_base`,
/// else `GetActorBase`); none without an actor.
fn base_arg(e: &mut Engine, a: &CommandArgs, form: u32, own_base: bool) -> Option<u32> {
    if form != 0 {
        return Some(form);
    }
    if a.this_obj == 0 || !is_actor(e, a.this_obj) {
        return None;
    }
    let base = if own_base {
        e.mem.u32(a.this_obj + REFR_BASE)
    } else {
        actor_base(e, a.this_obj)
    };
    (base != 0).then_some(base)
}

// ---- the last hit ----

// Translated from JIP LN NVSE 57.30 (5a30ac4), nvse/GameObjects.cpp: Actor::GetHitDataValue.
/// An actor with a process of level 0 or 1 and last hit data: value `kind` of it (1 the limb
/// damage, 2 the attacker, 3 the projectile, 4 the weapon, 6 the base weapon damage, 7 the
/// fatigue damage); a form only when there is one.
fn hit_data_value(e: &mut Engine, a: &CommandArgs, kind: u32) {
    let p = process(e, a.this_obj);
    if p == 0 || e.mem.u32(p + PROCESS_LEVEL) > 1 {
        return;
    }
    let hit = e.mem.u32(p + PROCESS_LAST_HIT_DATA);
    if hit == 0 {
        return;
    }
    let float = |e: &Engine, at: u32| f64::from(e.mem.f32(hit + at));
    match kind {
        1 => a.set_result(e, float(e, HIT_LIMB_DMG)),
        2..=4 => {
            let at = [HIT_SOURCE, HIT_PROJECTILE, HIT_WEAPON][(kind - 2) as usize];
            let form = e.mem.u32(hit + at);
            if form != 0 {
                set_form_result(e, a, form);
            }
        }
        6 => a.set_result(e, float(e, HIT_WPN_BASE_DMG)),
        7 => a.set_result(e, float(e, HIT_FATIGUE_DMG)),
        _ => {}
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_actor.h:
// Cmd_GetHitLimbDamage_Execute.
fn get_hit_limb_damage(e: &mut Engine, w: &[u32]) -> Ret {
    hit_data_value(e, &CommandArgs::new(w), 1);
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_actor.h:
// Cmd_GetHitAttacker_Execute.
fn get_hit_attacker(e: &mut Engine, w: &[u32]) -> Ret {
    hit_data_value(e, &CommandArgs::new(w), 2);
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_actor.h:
// Cmd_GetHitProjectile_Execute.
fn get_hit_projectile(e: &mut Engine, w: &[u32]) -> Ret {
    hit_data_value(e, &CommandArgs::new(w), 3);
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_actor.h:
// Cmd_GetHitWeapon_Execute.
fn get_hit_weapon(e: &mut Engine, w: &[u32]) -> Ret {
    hit_data_value(e, &CommandArgs::new(w), 4);
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_actor.h:
// Cmd_GetHitBaseWeaponDamage_Execute.
fn get_hit_base_weapon_damage(e: &mut Engine, w: &[u32]) -> Ret {
    hit_data_value(e, &CommandArgs::new(w), 6);
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_actor.h:
// Cmd_GetHitFatigueDamage_Execute.
fn get_hit_fatigue_damage(e: &mut Engine, w: &[u32]) -> Ret {
    hit_data_value(e, &CommandArgs::new(w), 7);
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_actor.h:
// Cmd_GetHitExtendedFlag_Execute.
/// 1 when the process's hit data (the virtual function) has flag `flagID` (0 to 8) of the table.
fn get_hit_extended_flag(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let p = process(e, a.this_obj);
    if p == 0 {
        return handled(true);
    }
    let read = Read::of(e, &a);
    let flag_id = read.int(0, 0);
    if read.ok && flag_id <= 8 {
        let hit = virtual_hit_data(e, p);
        if hit != 0 && e.mem.u32(hit + HIT_FLAGS) & HIT_FLAG_TABLE[flag_id as usize] != 0 {
            a.set_result(e, 1.0);
        }
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_actor.h:
// Cmd_GetHit3DData_Execute.
/// An array of the impact position (type not 1) and the impact angle (type not 0): three
/// numbers for type 0 or 1, six for a higher type.
fn get_hit_3d_data(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let p = process(e, a.this_obj);
    if p == 0 {
        return handled(true);
    }
    let read = Read::of(e, &a);
    if !read.ok {
        return handled(true);
    }
    let kind = read.int(0, 0);
    let hit = virtual_hit_data(e, p);
    if hit == 0 {
        return handled(true);
    }
    let mut elements = Vec::new();
    if kind != 1 {
        for i in 0..3 {
            let v = e.mem.f32(hit + HIT_IMPACT_POS + 4 * i);
            elements.push(Element::Number(f64::from(v)));
        }
    }
    if kind != 0 {
        for i in 0..3 {
            let v = e.mem.f32(hit + HIT_IMPACT_ANGLE + 4 * i);
            elements.push(Element::Number(f64::from(v)));
        }
    }
    let id = create_array(e, a.script, &elements);
    a.set_result(e, f64::from(id as i32));
    handled(true)
}

// ---- actor bases ----

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_actor.h:
// Cmd_AddBaseEffectListEffect_Execute.
/// The spell first in the base's spell list unless it is there.
fn add_base_effect_list_effect(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if !read.ok {
        return handled(true);
    }
    let spell = read.form(0);
    if let Some(base) = base_arg(e, &a, read.form(1), true) {
        let list = Ptr::<ListNode>::new(base + BASE_SPELLS);
        if !list_contains(e, list, spell) {
            list_prepend(e, list, spell);
        }
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_actor.h:
// Cmd_RemoveBaseEffectListEffect_Execute.
fn remove_base_effect_list_effect(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if !read.ok {
        return handled(true);
    }
    let spell = read.form(0);
    if let Some(base) = base_arg(e, &a, read.form(1), true) {
        list_remove(e, Ptr::<ListNode>::new(base + BASE_SPELLS), spell);
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_actor.h: GetActorModel (asm).
/// The model of the base, or of the template it takes its model from: while the base has a
/// template and template flag 0x40, the template unless that is a leveled creature or character.
fn actor_model(e: &Engine, base: u32) -> u32 {
    let mut base = base;
    for _ in 0..TEMPLATE_STEPS {
        let template = e.mem.u32(base + BASE_TEMPLATE_ACTOR);
        if template == 0 || e.mem.u8(base + BASE_TEMPLATE_FLAGS) & 0x40 == 0 {
            break;
        }
        let t = type_of(e, template);
        if t == TES_LEV_CREATURE || t == TES_LEV_CHARACTER {
            break;
        }
        base = template;
    }
    base + BASE_MODEL
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_actor.h:
// Cmd_GetSkeletonModel_Execute.
fn get_skeleton_model(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let base = read.form(0);
    let text = if read.ok && base != 0 {
        model_path(e, actor_model(e, base))
    } else {
        None
    };
    super::assign_string(e, &a, text);
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_actor.h:
// Cmd_SetNPCSkeletonModel_Execute.
/// An NPC's skeleton path set (`String::Set`; left out: emptied).
fn set_npc_skeleton_model(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let npc = read.form(0);
    let path = super::buffer_arg(e, &read, 1, 0x100, "SetNPCSkeletonModel");
    if read.ok && is_id(e, npc, TES_NPC) {
        let model = actor_model(e, npc);
        string_set(e, model + 4, &path);
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_actor.h:
// Cmd_SetCRESkeletonModel_Execute.
fn set_cre_skeleton_model(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let creature = read.form(0);
    let path = super::buffer_arg(e, &read, 1, 0x100, "SetCRESkeletonModel");
    if read.ok && is_id(e, creature, TES_CREATURE) {
        let model = actor_model(e, creature);
        string_set(e, model + 4, &path);
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_actor.h:
// Cmd_GetCreatureWeaponList_Execute.
fn get_creature_weapon_list(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if !read.ok {
        return handled(true);
    }
    if let Some(c) = base_arg(e, &a, read.form(0), false) {
        if is_id(e, c, TES_CREATURE) {
            let list = e.mem.u32(c + CREATURE_WEAPON_LIST);
            if list != 0 {
                set_form_result(e, &a, list);
            }
        }
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_actor.h:
// Cmd_GetDeathItem_Execute.
fn get_death_item(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if !read.ok {
        return handled(true);
    }
    if let Some(b) = base_arg(e, &a, read.form(0), false) {
        let item = e.mem.u32(b + BASE_DEATH_ITEM);
        if item != 0 {
            set_form_result(e, &a, item);
        }
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_actor.h:
// Cmd_SetDeathItem_Execute.
fn set_death_item(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if !read.ok {
        return handled(true);
    }
    let item = read.form(0);
    if let Some(b) = base_arg(e, &a, read.form(1), false) {
        e.mem.set_u32(b + BASE_DEATH_ITEM, item);
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_actor.h:
// Cmd_GetActorLeveledList_Execute.
/// The leveled creature or character list an actor was made from: its `ExtraLeveledCreature`'s
/// original base's template.
fn get_actor_leveled_list(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    if !is_actor(e, a.this_obj) {
        return handled(true);
    }
    let x = extra::refr_extra(e, a.this_obj, EXTRA_LEVELED_CREATURE);
    let base = if x == 0 { 0 } else { e.mem.u32(x + 0xc) };
    if base != 0 {
        let t = e.mem.u32(base + BASE_TEMPLATE_ACTOR);
        if is_id(e, t, TES_LEV_CREATURE) || is_id(e, t, TES_LEV_CHARACTER) {
            set_form_result(e, &a, t);
        }
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_actor.h:
// Cmd_GetImpactMaterialType_Execute.
/// An NPC's impact material, any other base's creature material field.
fn get_impact_material_type(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if !read.ok {
        return handled(true);
    }
    if let Some(b) = base_arg(e, &a, read.form(0), false) {
        let at = if type_of(e, b) == TES_NPC {
            NPC_IMPACT_MATERIAL
        } else {
            CREATURE_MATERIAL
        };
        let v = e.mem.u32(b + at) as i32;
        a.set_result(e, f64::from(v));
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_actor.h:
// Cmd_SetImpactMaterialType_Execute.
/// A type 0 to 11 into an NPC's impact material or any other base's creature material field.
fn set_impact_material_type(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let kind = read.int(0, 0);
    if !read.ok || kind > 11 {
        return handled(true);
    }
    if let Some(b) = base_arg(e, &a, read.form(1), false) {
        let at = if type_of(e, b) == TES_NPC {
            NPC_IMPACT_MATERIAL
        } else {
            CREATURE_MATERIAL
        };
        e.mem.set_u32(b + at, kind);
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_actor.h:
// Cmd_GetCreatureSoundsTemplate_Execute.
fn get_creature_sounds_template(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let c = read.form(0);
    if read.ok && is_id(e, c, TES_CREATURE) {
        let t = e.mem.u32(c + CREATURE_AUDIO_TEMPLATE);
        if t != 0 {
            set_form_result(e, &a, t);
        }
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_actor.h:
// Cmd_SetCreatureSoundsTemplate_Execute.
/// A creature's sounds template set to another creature or none (left out).
fn set_creature_sounds_template(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let (c, t) = (read.form(0), read.form(1));
    if read.ok && is_id(e, c, TES_CREATURE) && (t == 0 || is_id(e, t, TES_CREATURE)) {
        e.mem.set_u32(c + CREATURE_AUDIO_TEMPLATE, t);
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_actor.h:
// Cmd_GetBodyPartVATSTarget_Execute.
/// Body part 0 to 14's VATS target text of the base's body part data, `""` for none or an empty
/// one.
fn get_body_part_vats_target(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let part = read.int(0, 0);
    let mut text = Vec::new();
    if read.ok && part <= 14 {
        if let Some(b) = base_arg(e, &a, read.form(1), false) {
            let bpd = e.vcall(b, BASE_GET_BODY_PART_DATA, &[]).u32();
            let bp = if bpd == 0 {
                0
            } else {
                e.mem.u32(bpd + BODY_PARTS + 4 * part)
            };
            let s = bp + BODY_PART_VATS_TARGET;
            if bp != 0 && e.mem.u16(s + 4) != 0 {
                text = super::string_text(e, s).unwrap_or_default();
            }
        }
    }
    super::assign_string(e, &a, Some(text));
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_actor.h: GetFactionList.
/// JIP's sorted set of the factions: the base's (an actor's own base form when no base is given),
/// then for an actor its faction changes: a rank of 0 or more adds the faction, a lower one takes
/// it out. `None` without a base or actor.
fn faction_list(e: &mut Engine, actor: u32, base: u32) -> Option<BTreeSet<u32>> {
    let (actor, base) = if base != 0 {
        (0, base)
    } else {
        if actor == 0 || !is_actor(e, actor) {
            return None;
        }
        (actor, e.mem.u32(actor + REFR_BASE))
    };
    let mut set = BTreeSet::new();
    for d in list_items(e, Ptr::<ListNode>::new(base + BASE_FACTIONS)) {
        if d != 0 {
            set.insert(e.mem.u32(d));
        }
    }
    if actor != 0 {
        let x = extra::refr_extra(e, actor, EXTRA_FACTION_CHANGES);
        let list = if x == 0 { 0 } else { e.mem.u32(x + 0xc) };
        if list != 0 {
            for d in list_items(e, Ptr::<ListNode>::new(list)) {
                if d == 0 {
                    continue;
                }
                let faction = e.mem.u32(d);
                if e.mem.u8(d + 4) as i8 >= 0 {
                    set.insert(faction);
                } else {
                    set.remove(&faction);
                }
            }
        }
    }
    Some(set)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_actor.h:
// Cmd_GetFactions_Execute.
/// An array of the factions in address order; none (result 0) when there are none.
fn get_factions(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if !read.ok {
        return handled(true);
    }
    if let Some(set) = faction_list(e, a.this_obj, read.form(0)).filter(|s| !s.is_empty()) {
        let elements: Vec<Element> = set.into_iter().map(Element::Form).collect();
        let id = create_array(e, a.script, &elements);
        a.set_result(e, f64::from(id as i32));
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_actor.h: GetInFactionList.
/// Some item of the form list is one of the actor's factions.
fn in_faction_list(e: &mut Engine, actor: u32, list: u32) -> bool {
    match faction_list(e, actor, 0) {
        Some(set) if !set.is_empty() => list_items(e, Ptr::<ListNode>::new(list + LIST_FORM_LIST))
            .iter()
            .any(|f| set.contains(f)),
        _ => false,
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_actor.h:
// Cmd_GetInFactionList_Execute.
fn get_in_faction_list(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let list = read.form(0);
    if read.ok && list != 0 && in_faction_list(e, a.this_obj, list) {
        a.set_result(e, 1.0);
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_actor.h:
// Cmd_GetInFactionList_Eval.
fn get_in_faction_list_eval(e: &mut Engine, w: &[u32]) -> Ret {
    let a = EvalArgs::new(w);
    if a.arg1 != 0 && in_faction_list(e, a.this_obj, a.arg1) {
        e.mem.set_f64(a.result, 1.0);
    }
    handled(true)
}

// ---- actors ----

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_actor.h:
// Cmd_CrippleLimb_Execute.
/// Limb 0 to 6's condition (perception condition and on) damaged by -100 through
/// `DamageActorValue`, with the attacker given.
fn cripple_limb(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    if !is_actor(e, a.this_obj) {
        return handled(true);
    }
    let read = Read::of(e, &a);
    let limb = read.int(0, 0);
    if read.ok && limb <= 6 {
        let attacker = read.form(1);
        e.vcall(
            a.this_obj,
            ACTOR_DAMAGE_ACTOR_VALUE,
            &[
                AV_PERCEPTION_CONDITION + limb,
                (-100f32).to_bits(),
                attacker,
            ],
        );
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_actor.h:
// Cmd_GetKillXP_Execute; nvse/GameObjects.cpp: Actor::GetKillXP (asm).
/// The XP for killing the actor: the experience for its base's level (a creature's table or an
/// NPC's) times the difficulty's multiplier, floored (echoed in the console).
fn get_kill_xp(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    if is_actor(e, a.this_obj) {
        let base = e.mem.u32(a.this_obj + REFR_BASE);
        let level = e.call(GET_LEVEL, &args![base + 0x30]).u32() as u16 as i16 as i32;
        let not_creature = u32::from(type_of(e, a.this_obj) != CREATURE_REF);
        let xp = e
            .call(GET_EXPERIENCE_POINTS, &args![not_creature, level as u32])
            .u32() as i32;
        let player = e.mem.u32(THE_PLAYER);
        let difficulty = e.mem.u32(player + PLAYER_DIFFICULTY);
        let xp_bits = (xp as f32).to_bits();
        let scaled = e.call(DIFFICULTY_MULT, &args![xp_bits, difficulty]).st0 as f32;
        // 00406CE0 is cdecl with one stack float: the scaled XP JIP leaves on the stack.
        let r = e.call(FLOOR_FLOAT, &args![scaled.to_bits()]).st0;
        a.set_result(e, r);
    }
    print_result(e, &a);
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_actor.h:
// Cmd_GetKiller_Execute.
/// A dead actor's killer (life state 1 or 2), echoed in the console (0 for none).
fn get_killer(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let mut killer = 0;
    if is_actor(e, a.this_obj) {
        let state = e.mem.u32(a.this_obj + ACTOR_LIFE_STATE);
        if state == 1 || state == 2 {
            killer = e.mem.u32(a.this_obj + ACTOR_KILLER);
            if killer != 0 {
                set_form_result(e, &a, killer);
            }
        }
    }
    print_form(e, &a, killer);
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_actor.h:
// Cmd_KillActorAlt_Execute.
/// The game's own `KillActor` command (`005BE2A0`) with this command's words, its answer.
fn kill_actor_alt(e: &mut Engine, w: &[u32]) -> Ret {
    e.call(KILL_ACTOR_EXECUTE, w)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/jip_core.cpp: AnimData::GetPlayedIdle
// (asm).
/// The queued idle's form when it has one, else the playing idle's form unless its special idle
/// is done playing (`004985F0` answers 1), else none.
fn played_idle(e: &mut Engine, anim: u32) -> u32 {
    let queued = e.mem.u32(anim + ANIM_IDLE_QUEUED);
    if queued != 0 {
        let idle = e.mem.u32(queued + IDLE_OF_ANIM);
        if idle != 0 {
            return idle;
        }
    }
    let playing = e.mem.u32(anim + ANIM_IDLE_PLAYING);
    if playing == 0 {
        return 0;
    }
    let idle = e.mem.u32(playing + IDLE_OF_ANIM);
    if idle == 0 {
        return 0;
    }
    let done = e.call(SPECIAL_IDLE_DONE_PLAYING, &args![anim]).u32() & 0xff;
    done.wrapping_sub(1) & idle
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_actor.h:
// Cmd_GetPlayedIdle_Execute.
fn get_played_idle(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let anim = anim_data(e, a.this_obj);
    if anim != 0 {
        let idle = played_idle(e, anim);
        if idle != 0 {
            set_form_result(e, &a, idle);
        }
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_actor.h: IsIdlePlaying.
fn is_idle_playing(e: &mut Engine, refr: u32, idle: u32) -> bool {
    let anim = anim_data(e, refr);
    anim != 0 && played_idle(e, anim) == idle
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_actor.h:
// Cmd_IsIdlePlayingEx_Execute.
fn is_idle_playing_ex(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if read.ok && is_idle_playing(e, a.this_obj, read.form(0)) {
        a.set_result(e, 1.0);
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_actor.h:
// Cmd_IsIdlePlayingEx_Eval.
fn is_idle_playing_ex_eval(e: &mut Engine, w: &[u32]) -> Ret {
    let a = EvalArgs::new(w);
    if is_idle_playing(e, a.this_obj, a.arg1) {
        e.mem.set_f64(a.result, 1.0);
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_actor.h:
// Cmd_SetWeaponOut_Execute.
fn set_weapon_out(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    if !is_actor(e, a.this_obj) {
        return handled(true);
    }
    let read = Read::of(e, &a);
    if read.ok {
        let out = u32::from(read.int(0, 0) != 0);
        e.call(SET_WEAPON_OUT, &args![a.this_obj, out]);
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), nvse/GameObjects.cpp:
// TESObjectREFR::GetWaterImmersionPerc (asm).
/// For an actor whose byte +0x14C is set and whose process is high (level 0): the game's depth
/// (`00885560` with its z and cell); else 0.
fn water_immersion(e: &mut Engine, refr: u32) -> f64 {
    if !is_actor(e, refr) || e.mem.u8(refr + ACTOR_BYTE_14C) == 0 {
        return 0.0;
    }
    let p = e.mem.u32(refr + ACTOR_PROCESS);
    if p == 0 || e.mem.u32(p + PROCESS_LEVEL) != 0 {
        return 0.0;
    }
    let z = e.mem.u32(refr + REFR_POS_Z);
    let cell = e.mem.u32(refr + REFR_CELL);
    e.call(WATER_IMMERSION, &args![refr, z, cell]).st0
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_actor.h:
// Cmd_GetWaterImmersionPerc_Execute.
fn get_water_immersion_perc(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let v = water_immersion(e, a.this_obj);
    a.set_result(e, v);
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_actor.h:
// Cmd_GetWaterImmersionPerc_Eval.
fn get_water_immersion_perc_eval(e: &mut Engine, w: &[u32]) -> Ret {
    let a = EvalArgs::new(w);
    let v = water_immersion(e, a.this_obj);
    e.mem.set_f64(a.result, v);
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_actor.h:
// Cmd_GetInterruptPackage_Execute.
/// -1, or the type of the actor's interrupt package.
fn get_interrupt_package(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    a.set_result(e, -1.0);
    let p = process(e, a.this_obj);
    if p != 0 {
        let package = e.vcall(p, PROCESS_GET_INTERRUPT_PACKAGE, &[]).u32();
        if package != 0 {
            let t = e.mem.u8(package + PACKAGE_TYPE);
            a.set_result(e, f64::from(t));
        }
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), nvse/GameObjects.cpp: Actor::IsFleeing (asm).
/// The actor's current package is a flee package (type 0xA or 0x16), or a combat controller
/// (type 0x12) that `CombatController::IsFleeing` answers for.
fn fleeing(e: &mut Engine, actor: u32) -> bool {
    let p = process(e, actor);
    if p == 0 {
        return false;
    }
    let package = e.vcall(p, PROCESS_GET_CURRENT_PACKAGE, &[]).u32();
    if package == 0 {
        return false;
    }
    match e.mem.u8(package + PACKAGE_TYPE) {
        0x0a | 0x16 => true,
        0x12 => e.call(COMBAT_IS_FLEEING, &args![package]).u32() & 0xff != 0,
        _ => false,
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_actor.h:
// Cmd_IsFleeing_Execute.
fn is_fleeing(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    if fleeing(e, a.this_obj) {
        a.set_result(e, 1.0);
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_actor.h:
// Cmd_IsFleeing_Eval.
fn is_fleeing_eval(e: &mut Engine, w: &[u32]) -> Ret {
    let a = EvalArgs::new(w);
    if fleeing(e, a.this_obj) {
        e.mem.set_f64(a.result, 1.0);
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_actor.h:
// Cmd_SetCurrentAmmoRounds_Execute; nvse/GameObjects.cpp: Actor::GetAmmoInfo (asm).
/// A count of 0 or more into the ammo entry's count of an actor whose process is level 0 or 1.
fn set_current_ammo_rounds(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    if !is_actor(e, a.this_obj) {
        return handled(true);
    }
    let read = Read::of(e, &a);
    let rounds = read.int(0, 0) as i32;
    if !read.ok || rounds < 0 {
        return handled(true);
    }
    let p = e.mem.u32(a.this_obj + ACTOR_PROCESS);
    if p == 0 || e.mem.u8(p + PROCESS_LEVEL) > 1 {
        return handled(true);
    }
    let entry = e.mem.u32(p + PROCESS_AMMO_INFO);
    if entry != 0 {
        e.mem.set_u32(entry + ENTRY_COUNT_DELTA, rounds as u32);
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), nvse/GameObjects.cpp: Actor::HasNoPath.
/// The actor's mover has a path request and no path solution.
fn has_no_path(e: &Engine, actor: u32) -> bool {
    if !is_actor(e, actor) {
        return false;
    }
    let mover = e.mem.u32(actor + ACTOR_MOVER);
    mover != 0
        && e.mem.u32(mover + MOVER_PATH_REQUEST) != 0
        && e.mem.u32(mover + MOVER_PATH_SOLUTION) == 0
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_actor.h:
// Cmd_GetTargetUnreachable_Execute.
/// 1 when the actor has no path (echoed in the console).
fn get_target_unreachable(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    if has_no_path(e, a.this_obj) {
        a.set_result(e, 1.0);
    }
    print_result(e, &a);
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_actor.h:
// Cmd_GetTargetUnreachable_Eval.
fn get_target_unreachable_eval(e: &mut Engine, w: &[u32]) -> Ret {
    let a = EvalArgs::new(w);
    if has_no_path(e, a.this_obj) {
        e.mem.set_f64(a.result, 1.0);
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_actor.h:
// Cmd_GetActorMovementFlags_Execute.
/// The actor's mover's movement flags (echoed in the console).
fn get_actor_movement_flags(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    if is_actor(e, a.this_obj) {
        let mover = e.mem.u32(a.this_obj + ACTOR_MOVER);
        if mover != 0 {
            let flags = e.vcall(mover, MOVER_GET_MOVEMENT_FLAGS, &[]).u32() as i32;
            a.set_result(e, f64::from(flags));
        }
    }
    print_result(e, &a);
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), nvse/GameExtraData.cpp:
// ContChangesEntryList::FindForItem.
/// The first entry of the list for `item`, 0 for none.
fn find_for_item(e: &Engine, list: u32, item: u32) -> u32 {
    list_items(e, Ptr::<ListNode>::new(list))
        .into_iter()
        .find(|&entry| entry != 0 && e.mem.u32(entry + ENTRY_TYPE) == item)
        .unwrap_or(0)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_actor.h:
// Cmd_GetEquippedEx_Execute.
/// The first item of the form list the actor has equipped.
fn get_equipped_ex(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    if !is_actor(e, a.this_obj) {
        return handled(true);
    }
    let read = Read::of(e, &a);
    let form_list = read.form(0);
    if !read.ok || form_list == 0 {
        return handled(true);
    }
    let list = changes_list(e, a.this_obj);
    if list == 0 {
        return handled(true);
    }
    for item in list_items(e, Ptr::<ListNode>::new(form_list + LIST_FORM_LIST)) {
        if item == 0 {
            continue;
        }
        let entry = find_for_item(e, list, item);
        if entry != 0 && equipped_extra(e, entry) != 0 {
            set_form_result(e, &a, item);
            break;
        }
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_actor.h:
// Cmd_TestEquippedSlots_Execute.
/// 1 when an armour in some biped slot of the character shares a slot bit with the armour given.
fn test_equipped_slots(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    if a.this_obj == 0 || e.vcall(a.this_obj, REFR_IS_CHARACTER, &[]).u32() & 0xff == 0 {
        return handled(true);
    }
    let read = Read::of(e, &a);
    let armor = read.form(0);
    if !read.ok || armor == 0 || !is_type(e, armor, TES_OBJECT_ARMO_VTBL) {
        return handled(true);
    }
    let biped = e.vcall(a.this_obj, REFR_GET_BIPED_ANIM, &[]).u32();
    if biped == 0 {
        return handled(true);
    }
    let mask = e.mem.u32(armor + ARMO_PART_MASK);
    for slot in 0..20 {
        let worn = e.mem.u32(biped + BIPED_SLOTS + 0x10 * slot);
        if worn != 0
            && is_type(e, worn, TES_OBJECT_ARMO_VTBL)
            && e.mem.u32(worn + ARMO_PART_MASK) & mask != 0
        {
            a.set_result(e, 1.0);
            break;
        }
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_actor.h:
// Cmd_DonnerReedKuruParty_Execute.
/// The actor's "was eaten" mark: changed in its dismembered limbs extra (the extra removed when
/// cleared with no limb gone), or a new extra made to set it; each change marks the reference
/// changed (0x20000).
fn donner_reed_kuru_party(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    if !is_actor(e, a.this_obj) {
        return handled(true);
    }
    let read = Read::of(e, &a);
    if !read.ok {
        return handled(true);
    }
    let set = read.int(0, 0) != 0;
    let x = extra::refr_extra(e, a.this_obj, EXTRA_DISMEMBERED_LIMBS);
    let list = a.this_obj + extra::REFR_EXTRA_LIST;
    if x != 0 {
        if (e.mem.u8(x + DISMEMBERED_EATEN) != 0) != set {
            if !set && e.mem.u16(x + DISMEMBERED_MASK) == 0 {
                extra::remove_by_type(e, list, EXTRA_DISMEMBERED_LIMBS);
            } else {
                e.mem.set_u8(x + DISMEMBERED_EATEN, u8::from(set));
            }
            extra::mark_modified(e, a.this_obj, CHANGED_DISMEMBERED);
        }
    } else if set {
        let block = heap_alloc(e, DISMEMBERED_SIZE);
        let made = e.call(DISMEMBERED_CTOR, &args![block]).u32();
        e.mem.set_u8(made + DISMEMBERED_EATEN, 1);
        extra::add_extra(e, list, made);
        extra::mark_modified(e, a.this_obj, CHANGED_DISMEMBERED);
    }
    handled(true)
}

// ---- VATS, ash piles, the player's 3D (lane C1) ----

/// `kHookActorFlag2_NonTargetable` in the actor's `jipActorFlags2` (+0x106): the bit
/// `ui_hooks`' `AddVATSTarget` seam reads.
const NON_TARGETABLE: u8 = 2;

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_actor.h:
// Cmd_SetVATSTargetable_Execute.
/// An actor made targetable in VATS or not: when the asked state differs from the flag's, the
/// flag is flipped and the `AddVATSTarget` hook gains a user (not targetable) or loses one
/// (`HOOK_MOD`). No result.
fn set_vats_targetable(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    if !is_actor(e, a.this_obj) {
        return handled(true);
    }
    let read = Read::of(e, &a);
    if !read.ok {
        return handled(true);
    }
    let targetable = read.int(0, 0);
    let actor = Ptr::<crate::game::JipActorFlags>::new(a.this_obj);
    let flags = e.get(actor, crate::game::JipActorFlags::jipActorFlags2);
    if (targetable == 0) == (flags & NON_TARGETABLE == 0) {
        e.set(
            actor,
            crate::game::JipActorFlags::jipActorFlags2,
            flags ^ NON_TARGETABLE,
        );
        e.add_hook_user(engine_core::extend::Hook::AddVATSTarget, targetable == 0);
    }
    handled(true)
}

/// The game's `AttachAshPile` command (`005DB870`: one parameter parsed with the caller's
/// parameter list into its mode word, then the ash pile placed under the actor; the port's
/// `tesscriptfunctions.rs`), which JIP's `AttachAshPileEx` jumps to with its own arguments.
const ATTACH_ASH_PILE: u32 = 0x005d_b870;

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_actor.h:
// Cmd_AttachAshPileEx_Execute (asm).
/// For an actor and a form JIP marks as an ash pile (`kHookFormFlag6_IsAshPile`): the form kept
/// for the next ash pile (`s_altAshPile`, which G73's seam in the game's command takes once), then
/// the game's `AttachAshPile` run with this command's own words (JIP's `JMP 0x5DB870`), its
/// answer this command's. Otherwise nothing (true).
fn attach_ash_pile_ex(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    if !is_actor(e, a.this_obj) {
        return handled(true);
    }
    let read = Read::of(e, &a);
    let form = read.form(0);
    if !read.ok || form == 0 {
        return handled(true);
    }
    if crate::world_hooks::flags6(e, form) & crate::game_patches::gh2::IS_ASH_PILE == 0 {
        return handled(true);
    }
    e.ext_free::<crate::game_patches::gh2::AltAshPile>()
        .0
        .store(form, std::sync::atomic::Ordering::Relaxed);
    e.call(ATTACH_ASH_PILE, w)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_actor.h:
// Cmd_UpdatePlayer3D_Execute.
/// `g_thePlayer->Update3D()` (`overrides::ref_update_3d`: the player with 3D, not already marked,
/// marked for JIP's model loader hook and queued to load again). Nothing without the player. No
/// result.
fn update_player_3d(e: &mut Engine, w: &[u32]) -> Ret {
    let _ = CommandArgs::new(w);
    let player = e.mem.u32(THE_PLAYER);
    if player != 0 {
        crate::overrides::ref_update_3d(e, player);
    }
    handled(true)
}

/// `BaseProcess::PushActorAway` (vtable +0x418: `HighProcess`' `01087864` holds `0091FEE0`,
/// `RET 0x14`: the pushed actor, x, y, z, the force).
const PUSH_ACTOR_AWAY: u32 = 0x418;
/// A reference's position (+0x30).
const REFR_POS: u32 = 0x30;

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_actor.h:
// Cmd_PushActorAwayAlt_Execute.
/// An actor with a high process (level 0) pushed away from the point (x, y, z; each 0 when not
/// given), the point taken relative to the actor (its position added, in floats) unless the flag
/// says absolute, with the force `Actor::AdjustPushForce` makes of the given one. No result.
fn push_actor_away_alt(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    if !is_actor(e, a.this_obj) {
        return handled(true);
    }
    let read = Read::of(e, &a);
    if !read.ok {
        return handled(true);
    }
    let force = read.float(0, 0.0);
    let mut pos = [read.float(1, 0.0), read.float(2, 0.0), read.float(3, 0.0)];
    let absolute = read.int(4, 0);
    let p = process(e, a.this_obj);
    if p == 0 || e.mem.u8(p + PROCESS_LEVEL) != 0 {
        return handled(true);
    }
    if absolute == 0 {
        for (i, v) in pos.iter_mut().enumerate() {
            *v += e.mem.f32(a.this_obj + REFR_POS + 4 * i as u32);
        }
    }
    let force = crate::world_push::adjust_push_force(e, a.this_obj, force);
    e.vcall(
        p,
        PUSH_ACTOR_AWAY,
        &args![a.this_obj, pos[0], pos[1], pos[2], force],
    );
    handled(true)
}

/// `Actor::DismemberLimb`'s game calls (JIP's asm, each confirmed in the exe with its `RET`):
/// `TESObjectREFR::GetDismembered` (`00573090`, `(part)`, `RET 4`), `Actor::GetEssential`
/// (`0087F3D0`, no words), the actor's vtable +0x1A0 (`(0)`; `Actor`'s `01084254` holds
/// `008D0370`, which answers false), the base form's vtable +0x180 (its body part data), the
/// limb word's flag test (`00456630`, `(1)`, `RET 4`: the flag word at +0x30), then
/// `TESObjectREFR::SetDismembered` (`00572FC0`, `(part, 0, 1, part, explode)`, `RET 0x14`) and
/// the loose piece (`008B52A0`, `(actor value, part, explode, 1, 1, 0)`, `RET 0x18`).
const GET_DISMEMBERED: u32 = 0x0057_3090;
const GET_ESSENTIAL: u32 = 0x0087_f3d0;
const ACTOR_SLOT_1A0: u32 = 0x1a0;
const BASE_BODY_PART_DATA: u32 = 0x180;
const LIMB_FLAG_TEST: u32 = 0x0045_6630;
const SET_DISMEMBERED: u32 = 0x0057_2fc0;
const DISMEMBER_PIECE: u32 = 0x008b_52a0;
/// `BGSBodyPartData::bodyParts` (+0x34, 15 words), a body part's flags (+0x60: 1 severable, 8
/// explodable) and actor value (+0x63) (`commands::forms`); a high process's limb words from
/// +0x1DC (`MiddleHighProcess`' constructor clears the 0x3C bytes there).
const PART_FLAGS: u32 = 0x60;
const PART_ACTOR_VALUE: u32 = 0x63;
const PROCESS_LIMBS: u32 = 0x1dc;

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_actor.h:
// Cmd_DismemberLimb_Execute, and nvse/GameObjects.cpp: Actor::DismemberLimb (asm).
/// For an actor and a body part up to 13: when the part is not dismembered yet, the actor is not
/// essential, its vtable +0x1A0 answers false, its base's body part data has the part with flag 1
/// or 8, it has a high process whose limb word for the part is set and has no flag 1: the part
/// set dismembered and its loose piece made (exploded when asked). No result.
fn dismember_limb(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let actor = a.this_obj;
    if !is_actor(e, actor) {
        return handled(true);
    }
    let read = Read::of(e, &a);
    if !read.ok {
        return handled(true);
    }
    let part = read.int(0, 0);
    let explode = u32::from(read.int(1, 0) != 0);
    if part > 0xd {
        return handled(true);
    }
    if e.call(GET_DISMEMBERED, &args![actor, part]).u32() & 0xff != 0
        || e.call(GET_ESSENTIAL, &args![actor]).u32() & 0xff != 0
        || e.vcall(actor, ACTOR_SLOT_1A0, &args![0u32]).u32() & 0xff != 0
    {
        return handled(true);
    }
    let base = e.mem.u32(actor + REFR_BASE);
    if base == 0 {
        return handled(true);
    }
    let data = e.vcall(base, BASE_BODY_PART_DATA, &args![]).u32();
    let Some(body_part) = (data != 0)
        .then(|| h::ptr(e, data.wrapping_add(BODY_PARTS + 4 * part)))
        .flatten()
    else {
        return handled(true);
    };
    let (Some(flags), Some(value)) = (
        h::rd8(e, body_part.wrapping_add(PART_FLAGS)),
        h::rd8(e, body_part.wrapping_add(PART_ACTOR_VALUE)),
    ) else {
        return handled(true);
    };
    if flags & 9 == 0 {
        return handled(true);
    }
    let p = process(e, actor);
    if p == 0 || e.mem.u8(p + PROCESS_LEVEL) != 0 {
        return handled(true);
    }
    let limb = e.mem.u32(p + PROCESS_LIMBS + 4 * part);
    if limb == 0 || e.call(LIMB_FLAG_TEST, &args![limb, 1u32]).u32() & 0xff != 0 {
        return handled(true);
    }
    e.call(
        SET_DISMEMBERED,
        &args![actor, part, 0u32, 1u32, part, explode],
    );
    e.call(
        DISMEMBER_PIECE,
        &args![actor, u32::from(value), part, explode, 1u32, 1u32, 0u32],
    );
    handled(true)
}

/// `GetGroundMaterial`'s reads and calls (each confirmed in the exe): the controller's support
/// material `eSupportMaterial` +0x434 (`00C6E980` compares it with 0xF and 0x24 and stores it),
/// 0x24 meaning the land; the reference's cell (+0x40); `TESObjectCELL::GetLand` (`00546FB0`,
/// no words); `TESObjectLAND::GetCoordData` (`0053B550`, `(out, position, 1)`, `RET 0xC`: the
/// 0x50-byte record whose quadrant is at +0x18 and nearest vertex at +0x3C); the texture of a
/// vertex (`0053A630`, `(quadrant byte, vertex word)`, `RET 8`), its material byte at +0x1C.
const CC_SUPPORT_MATERIAL: u32 = 0x434;
const MATERIAL_LAND: u32 = 0x24;
const CELL_GET_LAND: u32 = 0x0054_6fb0;
const LAND_COORD_DATA: u32 = 0x0053_b550;
const LAND_TEXTURE_AT: u32 = 0x0053_a630;
const COORD_QUADRANT: u32 = 0x18;
const COORD_VERTEX: u32 = 0x3c;
const TEXTURE_MATERIAL: u32 = 0x1c;

// Translated from JIP LN NVSE 57.30 (5a30ac4), nvse/GameObjects.cpp: Actor::GetGroundMaterial
// (asm).
/// The material under an actor that is not in the air: the controller's support material
/// converted (`kMaterialConvert`, -1 past 31), or for the land (0x24) the texture under the
/// actor's position in its cell's land. -1 for anything else.
fn ground_material(e: &mut Engine, actor: u32) -> i32 {
    let ctrl = super::jip_fn_actor::controller(e, actor);
    if ctrl == 0
        || h::rd32(e, ctrl.wrapping_add(super::jip_fn_actor::CC_STATE)).map_or(true, |s| s & 2 != 0)
    {
        return -1;
    }
    let Some(mut material) = h::rd32(e, ctrl.wrapping_add(CC_SUPPORT_MATERIAL)) else {
        return -1;
    };
    if material == MATERIAL_LAND {
        let cell = e.mem.u32(actor + REFR_CELL);
        if cell == 0 {
            return -1;
        }
        let land = e.call(CELL_GET_LAND, &args![cell]).u32();
        if land == 0 {
            return -1;
        }
        let texture = e.with_stack(0x50, |e, frame| {
            let out = frame.addr();
            e.call(LAND_COORD_DATA, &args![land, out, actor + REFR_POS, 1u32]);
            let quadrant = u32::from(e.mem.u8(out + COORD_QUADRANT));
            let vertex = u32::from(e.mem.u16(out + COORD_VERTEX));
            e.call(LAND_TEXTURE_AT, &args![land, quadrant, vertex])
                .u32()
        });
        let Some(m) = (texture != 0)
            .then(|| h::rd8(e, texture.wrapping_add(TEXTURE_MATERIAL)))
            .flatten()
        else {
            return -1;
        };
        material = u32::from(m);
    }
    super::jip_fn_projectile::MATERIAL_CONVERT
        .get(material as usize)
        .map_or(-1, |&c| i32::from(c))
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_actor.h:
// Cmd_GetGroundMaterial_Execute.
fn get_ground_material(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let m = ground_material(e, a.this_obj);
    a.set_result(e, f64::from(m));
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_actor.h:
// Cmd_GetGroundMaterial_Eval.
fn get_ground_material_eval(e: &mut Engine, w: &[u32]) -> Ret {
    let a = EvalArgs::new(w);
    let m = ground_material(e, a.this_obj);
    e.mem.set_f64(a.result, f64::from(m));
    handled(true)
}

// ---- the character controller (lane C1) ----

/// `bhkCharacterController` fields, confirmed in the exe: the gravity `fGravity` +0x54C
/// (`00C73ED0` copies the construction info's +0x90 into it), the tilt angles `fPitchAngle`
/// +0x550 and `fRollAngle` +0x554 (`ApplyPitchAndRoll` `00C6E420` reads both; JIP's
/// `NiPoint2 tiltAngle`), the character proxy `phkObject` +8 (the `bhkRefObject`'s Havok object,
/// `havok_refs::HAVOK_OBJECT`) whose velocity is at +0x10 (`hkpCharacterProxy::getLinearVelocity`
/// `00CAC950`: `LEA EAX,[ECX+0x10]`). The state and velocity offsets are `jip_fn_actor`'s.
const CC_GRAVITY: u32 = 0x54c;
const CC_TILT: u32 = 0x550;
const PROXY_VELOCITY: u32 = 0x10;

/// The X or Y index of an axis byte (`axis - 'X'` where JIP accepts `axis < 'Z'`).
fn tilt_index(axis: u8) -> Option<u32> {
    axis.checked_sub(b'X').filter(|&i| i < 2).map(u32::from)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_actor.h:
// Cmd_GetActorTiltAngle_Execute.
/// The controller's tilt angle about the axis (X: pitch +0x550, Y: roll +0x554) in degrees (the
/// float times `Dbl180dPI`, in doubles); nothing for Z or without a controller.
fn get_actor_tilt_angle(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if !read.ok {
        return handled(true);
    }
    let Some(i) = tilt_index(super::jip_fn_misc_ref_havok::axis_arg(&read, 0)) else {
        return handled(true);
    };
    let ctrl = super::jip_fn_actor::controller(e, a.this_obj);
    if let Some(v) = (ctrl != 0)
        .then(|| h::rdf32(e, ctrl.wrapping_add(CC_TILT + 4 * i)))
        .flatten()
    {
        a.set_result(e, f64::from(v) * super::DBL_180_D_PI);
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_actor.h:
// Cmd_SetActorTiltAngle_Execute.
/// The controller's tilt angle about the axis (X or Y) set to the degrees times `FltPId180` (in
/// floats): a plain store the controller's `ApplyPitchAndRoll` reads. No result.
fn set_actor_tilt_angle(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if !read.ok {
        return handled(true);
    }
    let axis = super::jip_fn_misc_ref_havok::axis_arg(&read, 0);
    let angle = read.float(1, 0.0);
    let Some(i) = tilt_index(axis) else {
        return handled(true);
    };
    let ctrl = super::jip_fn_actor::controller(e, a.this_obj);
    if ctrl != 0 {
        let v = angle * super::FLT_PI_D_180;
        let _ = e
            .mem
            .try_write(ctrl.wrapping_add(CC_TILT + 4 * i), &v.to_le_bytes());
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_actor.h:
// Cmd_GetActorGravityMult_Execute.
/// The controller's gravity (+0x54C); nothing without a controller.
fn get_actor_gravity_mult(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let ctrl = super::jip_fn_actor::controller(e, a.this_obj);
    if let Some(v) = (ctrl != 0)
        .then(|| h::rdf32(e, ctrl.wrapping_add(CC_GRAVITY)))
        .flatten()
    {
        a.set_result(e, f64::from(v));
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_actor.h:
// Cmd_SetActorGravityMult_Execute.
/// The controller's gravity set (a plain store the in-air state reads each step). No result.
fn set_actor_gravity_mult(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if !read.ok {
        return handled(true);
    }
    let mult = read.float(0, 0.0);
    let ctrl = super::jip_fn_actor::controller(e, a.this_obj);
    if ctrl != 0 {
        let _ = e
            .mem
            .try_write(ctrl.wrapping_add(CC_GRAVITY), &mult.to_le_bytes());
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_actor.h:
// Cmd_SetActorVelocity_Execute.
/// The controller's character proxy's (+8, when not null) velocity on the axis set (+0x10, a
/// plain store; JIP wakes nothing). No result.
fn set_actor_velocity(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if !read.ok {
        return handled(true);
    }
    let axis = super::jip_fn_misc_ref_havok::axis_arg(&read, 0);
    let velocity = read.float(1, 0.0);
    let ctrl = super::jip_fn_actor::controller(e, a.this_obj);
    let Some(proxy) = (ctrl != 0)
        .then(|| h::ptr(e, ctrl.wrapping_add(h::HAVOK_OBJECT)))
        .flatten()
    else {
        return handled(true);
    };
    if let Some(i) = axis.checked_sub(b'X').filter(|&i| i < 3) {
        let at = proxy.wrapping_add(PROXY_VELOCITY + 4 * u32::from(i));
        let _ = e.mem.try_write(at, &velocity.to_le_bytes());
    }
    handled(true)
}

/// `Projectile::ImpactList` (+0x88, a `BSSimpleList` of `ImpactData*`) and an impact data's
/// `pCollidee` (+0) and `pTargetWorldObject` (+0x1C): `009C3EE0` walks the list from +0x88
/// comparing each item's +0; `ImpactData::SaveGame` `009BB970` hands +0x1C to the collidee's
/// vtable +0x1D0.
const PROJ_IMPACTS: u32 = 0x88;
const IMPACT_REFR: u32 = 0x00;
const IMPACT_BODY: u32 = 0x1c;

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_actor.h:
// Cmd_GetHitNode_Execute.
/// The name of the node of the body the last hit's projectile hit on this actor: the projectile
/// (hit data +8, when it is a projectile) and its impact list, the first impact on this actor with
/// a body (+0x1C) whose node (`hkpWorldObject::GetParentNode`) is found; the node's name, `"NULL"`
/// for a null name. `"NULL"` when nothing is found.
fn get_hit_node(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let name = hit_node_name(e, a.this_obj).unwrap_or_else(|| b"NULL".to_vec());
    super::assign_string(e, &a, Some(name));
    handled(true)
}

fn hit_node_name(e: &mut Engine, actor: u32) -> Option<Vec<u8>> {
    let p = process(e, actor);
    if p == 0 {
        return None;
    }
    let hit = virtual_hit_data(e, p);
    let projectile = (hit != 0)
        .then(|| h::ptr(e, hit.wrapping_add(HIT_PROJECTILE)))
        .flatten()?;
    if !h::is_projectile(e, projectile) {
        return None;
    }
    let mut walk = h::Walk::new();
    let head = Ptr::<ListNode>::new(projectile.wrapping_add(PROJ_IMPACTS));
    for impact in list_items(e, head) {
        if impact == 0 || h::rd32(e, impact.wrapping_add(IMPACT_REFR)) != Some(actor) {
            continue;
        }
        let Some(body) = h::ptr(e, impact.wrapping_add(IMPACT_BODY)) else {
            continue;
        };
        let node = h::parent_node(e, body, &mut walk);
        if node != 0 {
            // A null name is JIP's "NULL" (`NiFixedString::Get`); an unreadable one too.
            let text = h::ptr(e, node.wrapping_add(h::NAME))
                .and_then(|p| super::nif_nodes::c_text(e, p))
                .unwrap_or_else(|| b"NULL".to_vec());
            return Some(text);
        }
    }
    None
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/Ni_types.cpp:
// NiMatrix33::MultiplyVectorInv (asm).
/// The vector's x, y, z (its fourth lane cleared) times the matrix's rows (`NiMatrix33`, nine
/// floats by rows), lane by lane: `(m[i]*x + m[3+i]*y) + m[6+i]*z` for the three lanes kept.
fn multiply_vector_inv(m: &[f32; 9], v: [f32; 4]) -> [f32; 3] {
    std::array::from_fn(|i| (m[i] * v[0] + m[3 + i] * v[1]) + m[6 + i] * v[2])
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_actor.h:
// Cmd_GetActorVelocityAlt_Execute.
/// The controller's velocity into the three variables (each lane as a double), with the local
/// flag turned by the root node's world rotation ([`multiply_vector_inv`]); 1 when written. A
/// reference without 3D or a controller answers nothing.
fn get_actor_velocity_alt(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if !read.ok {
        return handled(true);
    }
    let vars = [read.local(0), read.local(1), read.local(2)];
    if vars.contains(&0) {
        return handled(true);
    }
    let local = read.int(3, 0);
    let ctrl = super::jip_fn_actor::controller(e, a.this_obj);
    if ctrl == 0 {
        return handled(true);
    }
    let root = h::ref_ni_node(e, a.this_obj);
    if root == 0 {
        return handled(true);
    }
    let Some(v) = h::vec4(e, ctrl.wrapping_add(super::jip_fn_actor::CC_VELOCITY)) else {
        return handled(true);
    };
    let out = if local == 0 {
        [v[0], v[1], v[2]]
    } else {
        let mut m = [0f32; 9];
        for (i, x) in m.iter_mut().enumerate() {
            match h::rdf32(e, root.wrapping_add(h::WORLD_ROTATE + 4 * i as u32)) {
                Some(f) => *x = f,
                None => return handled(true),
            }
        }
        multiply_vector_inv(&m, v)
    };
    for (var, x) in vars.into_iter().zip(out) {
        e.mem.set_f64(var.wrapping_add(8), f64::from(x));
    }
    a.set_result(e, 1.0);
    handled(true)
}

#[cfg(test)]
#[path = "jip_fn_actor_p2_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "jip_fn_actor_p2_c1_tests.rs"]
mod c1_tests;
