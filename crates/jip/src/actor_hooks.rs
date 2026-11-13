//! JIP LN NVSE's actor event hooks (step 3.5b; `internal/hooks.h` (JIP LN NVSE)): an actor's
//! animation actions and play groups, health damage and crippled limbs, weapons fired and
//! reloaded, hits and critical hits, ragdolls, aid items used and projectile impacts, with the
//! eleven commands that set those events, and JIP's hit-data copy in the processes' vtables.
//!
//! JIP patches a hook's sites in when its first user arrives (`HookInfo::ModUsers`); here each
//! site is a seam in its translation (step 3.5b's first half) that reads the hook's user count
//! and then asks the handler registered below. JIP's hit-data routines are written into the two
//! processes' vtables at its start, as JIP writes them (always, not on demand): the exe reaches
//! `MiddleHighProcess::SetLastHitData` (`0092c400`) and `ClearLastHitData` (`0092c4a0`) only
//! through those slots.
//!
//! Nothing here runs while JIP answers "not installed": the handlers, the routines and the
//! commands are put in only when it answers "installed".

use crate::game::{
    ActorHitData, Animation, BSFadeNode, BhkRagdollController, JipActorFlags, JipFormFlags,
    MiddleHighProcess, NiAVObject, TESForm, TESObjectREFR,
};
use crate::Jip;
use engine_core::extend::{Hook, Owner, SiteSlot};
use engine_core::prelude::*;
use nvse::commands::Read;
use nvse::{handled, CommandArgs};
use std::collections::{BTreeMap, BTreeSet};

/// The commands' opcodes.
pub const SET_ON_ANIM_ACTION: u32 = 0x279c;
pub const SET_ON_PLAY_GROUP: u32 = 0x279d;
pub const SET_ON_HEALTH_DAMAGE: u32 = 0x279e;
pub const SET_ON_CRIPPLED_LIMB: u32 = 0x279f;
pub const SET_ON_FIRE_WEAPON: u32 = 0x27a0;
pub const SET_ON_CRITICAL_HIT: u32 = 0x2801;
pub const SET_ON_HIT: u32 = 0x2880;
pub const SET_ON_PROJECTILE_IMPACT: u32 = 0x28b0;
pub const SET_ON_USE_AID_ITEM: u32 = 0x28b3;
pub const SET_ON_RELOAD_WEAPON: u32 = 0x28c5;
pub const SET_ON_RAGDOLL: u32 = 0x28c9;
/// The eleven.
pub const COMMANDS: [u32; 11] = [
    SET_ON_ANIM_ACTION,
    SET_ON_PLAY_GROUP,
    SET_ON_HEALTH_DAMAGE,
    SET_ON_CRIPPLED_LIMB,
    SET_ON_FIRE_WEAPON,
    SET_ON_CRITICAL_HIT,
    SET_ON_HIT,
    SET_ON_PROJECTILE_IMPACT,
    SET_ON_USE_AID_ITEM,
    SET_ON_RELOAD_WEAPON,
    SET_ON_RAGDOLL,
];

/// The seams whose handlers are here (engine-core `SITES`, owner JIP, step 3.5b).
pub const SITES: [u32; 11] = [
    0x0049_4e19,
    0x005a_c78a,
    0x0066_ee72,
    0x0088_c74e,
    0x008a_7549,
    0x008a_7551,
    0x008a_86b0,
    0x008a_8dd0,
    0x008c_49b7,
    0x009c_20bf,
    0x00c7_c151,
];

/// Each command's hook and every site JIP patches for it (`InitJIPHooks`, the caller markers
/// included): a command may add users only while all of them have seams.
pub const COMMAND_SITES: [(u32, Hook, &[u32]); 11] = [
    (SET_ON_ANIM_ACTION, Hook::SetAnimAction, &[0x008a_7551]),
    (SET_ON_PLAY_GROUP, Hook::SetAnimGroup, &[0x0049_4e19]),
    (SET_ON_HEALTH_DAMAGE, Hook::DamageActorValue, &[0x0066_ee72]),
    (SET_ON_CRIPPLED_LIMB, Hook::DamageActorValue, &[0x0066_ee72]),
    (
        SET_ON_FIRE_WEAPON,
        Hook::RemoveAmmo,
        &[0x008a_86b0, 0x008a_8dd0],
    ),
    (SET_ON_CRITICAL_HIT, Hook::OnHitEvent, &[]),
    (SET_ON_HIT, Hook::OnHitEvent, &[0x005a_c78a, 0x008c_49b7]),
    (
        SET_ON_PROJECTILE_IMPACT,
        Hook::ProjectileImpact,
        &[0x009c_20bf],
    ),
    (SET_ON_USE_AID_ITEM, Hook::EquipAidItem, &[0x0088_c74e]),
    (SET_ON_RELOAD_WEAPON, Hook::ReloadWeapon, &[0x008a_7549]),
    (SET_ON_RAGDOLL, Hook::OnRagdoll, &[0x00c7_c151]),
];

/// JIP's hit-data routines (`CopyHitDataHook`, and its `ClearLastHitData` bytes written at
/// `0x92C4A0`), in JIP's routines past the click routine's.
pub const COPY_HIT_DATA_ROUTINE: u32 = crate::identity::ROUTINES + 0x500;
pub const CLEAR_HIT_DATA_ROUTINE: u32 = crate::identity::ROUTINES + 0x504;
const _: () = assert!(CLEAR_HIT_DATA_ROUTINE < engine_core::mem::EXTENDER_CODE.end);
/// `MiddleHighProcess::SetLastHitData`'s slot (`+0x774`) in HighProcess's (`01087864`) and
/// MiddleHighProcess's (`0108904c`) vtables, and `ClearLastHitData`'s right after (`+0x778`).
pub const COPY_HIT_DATA_SLOTS: [u32; 2] = [0x0108_7fd8, 0x0108_97c0];
pub const CLEAR_HIT_DATA_SLOTS: [u32; 2] = [0x0108_7fdc, 0x0108_97c4];
/// The size JIP allocates for a process's hit data: the game's 0x60 bytes and JIP's location.
const HIT_DATA_SIZE: u32 = 0x64;

/// `kHookActorFlag1_InfiniteAmmo`, `kHookActorFlag3_On*`, `kHookFormFlag6_OnEquipHandlers` and
/// `_ProjectileImpact` (`internal/hooks.h` (JIP LN NVSE)).
const INFINITE_AMMO: u8 = 1 << 2;
const ON_ANIM_ACTION: u8 = 1 << 0;
const ON_PLAY_GROUP: u8 = 1 << 1;
const ON_HEALTH_DAMAGE: u8 = 1 << 2;
const ON_CRIPPLED_LIMB: u8 = 1 << 3;
const ON_FIRE_WEAPON: u8 = 1 << 4;
const ON_HIT: u8 = 1 << 5;
const FORM_EVENT_HANDLERS: u8 = 1 << 0;
/// `ActorHitData::kFlag_IsCritical`.
const IS_CRITICAL: u8 = 4;
/// `kAVCode_Health`, `kAVCode_PerceptionCondition`, `kAVCode_BrainCondition`.
const HEALTH: u8 = 0x10;
const PERCEPTION_CONDITION: u8 = 0x19;
const BRAIN_CONDITION: u8 = 0x1f;
/// `kAnimAction_Reload`, `kAnimAction_ReloadLoop`.
const RELOAD: u8 = 9;
const RELOAD_LOOP: u8 = 0x11;
/// `ScriptEventList::kEvent_OnHit`.
const ON_HIT_EVENT: u32 = 0x80;
/// The player's form id, `kVtbl_BSFadeNode`, `kFormType_*`.
const PLAYER_ID: u32 = 0x14;
const BS_FADE_NODE: u32 = 0x010a_8f90;
const SCRIPT: u8 = 0x11;
const WEAPON: u8 = 0x28;
const ALCHEMY_ITEM: u8 = 0x2f;
const PROJECTILE: u8 = 0x33;
const LIST_FORM: u8 = 0x55;
/// `ExtraDataList`'s place in a reference (`TESObjectREFR::extraDataList`, `+0x44`), which
/// `Script::SetActionFlag`'s list stands for; an actor's vtables are at `0x0108xxxx` (JIP's
/// `OnHitEventHook` tests the high word and bit 3 of the low byte).
const EXTRA_LIST_OFFSET: u32 = 0x44;
const ACTOR_VTABLE_HIGH: u16 = 0x0108;
/// An actor value owner's place in an actor (`Actor::avOwner`, `+0xA4`).
const AV_OWNER_OFFSET: u32 = 0xa4;
/// A projectile base's type (`BGSProjectile::type`, `+0x62`; 2 is a beam), a projectile's
/// position (`+0x30`) and its impact data (`+0x88`; a position at `+4`).
const BEAM: u16 = 2;

/// Caller marks (a hook that reads a return address, step 3.5b's first half), one bit each.
const MARK_RELOAD_AMMO: u8 = 1;
const MARK_MAGIC_HIT: u8 = 2;

/// A critical hit's event (`CriticalHitEventData`): filters (0 for any) and the script.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CriticalHit {
    pub target: u32,
    pub source: u32,
    pub weapon: u32,
    pub script: u32,
}

/// Scripts by key (`UnorderedMap<K, EventCallbackScripts>`).
pub type ScriptsBy = BTreeMap<u32, BTreeSet<u32>>;

/// JIP's actor hook state.
#[derive(Clone, Debug, Default)]
pub struct State {
    /// The caller marks, per game thread (a return address is each thread's own).
    pub marks: BTreeMap<u32, u8>,
    /// `s_animActionEventMap`, `s_playGroupEventMap`: by action or group type.
    pub anim_action: ScriptsBy,
    pub play_group: ScriptsBy,
    /// `s_animActionEventMapFl`, `s_playGroupEventMapFl`: by actor, then action or group type.
    pub anim_action_actors: BTreeMap<u32, ScriptsBy>,
    pub play_group_actors: BTreeMap<u32, ScriptsBy>,
    /// `s_healthDamageEventMap`, `s_crippledLimbEventMap`, `s_fireWeaponEventMap`,
    /// `s_onHitEventMap`: by actor.
    pub health_damage: ScriptsBy,
    pub crippled_limb: ScriptsBy,
    pub fire_weapon: ScriptsBy,
    pub on_hit: ScriptsBy,
    /// `s_fireWeaponEventScripts`, `s_onHitEventScripts`: for every actor.
    pub fire_weapon_scripts: BTreeSet<u32>,
    pub on_hit_scripts: BTreeSet<u32>,
    /// `s_reloadWeaponEventScripts`, `s_onRagdollEventScripts`.
    pub reload_weapon: BTreeSet<u32>,
    pub ragdoll: BTreeSet<u32>,
    /// `s_useAidItemEventMap` by aid item, `s_projectileImpactEventMap` by projectile base.
    pub use_aid_item: ScriptsBy,
    pub projectile_impact: ScriptsBy,
    /// `s_criticalHitEvents`, in order.
    pub critical_hit: Vec<CriticalHit>,
}

fn state(e: &mut Engine) -> &mut State {
    &mut e.ext::<Jip>().actor_hooks
}

/// Puts JIP's actor hooks in, when it answers "installed": the seams' handlers, the hit-data
/// routines in the processes' vtables, and the eleven commands.
pub fn install(e: &mut Engine, answers_installed: bool) {
    if !answers_installed {
        return;
    }
    e.on_site(SiteSlot::new(Owner::Jip, 0x0049_4e19), set_anim_group_site);
    e.on_site(SiteSlot::new(Owner::Jip, 0x005a_c78a), on_hit_site);
    e.on_site(
        SiteSlot::new(Owner::Jip, 0x0066_ee72),
        damage_actor_value_site,
    );
    e.on_site(SiteSlot::new(Owner::Jip, 0x0088_c74e), equip_aid_item_site);
    e.on_site(SiteSlot::new(Owner::Jip, 0x008a_7549), reload_weapon_site);
    e.on_site(SiteSlot::new(Owner::Jip, 0x008a_7551), set_anim_action_site);
    e.on_site(SiteSlot::new(Owner::Jip, 0x008a_86b0), reload_ammo_mark);
    e.on_site(SiteSlot::new(Owner::Jip, 0x008a_8dd0), remove_ammo_site);
    e.on_site(SiteSlot::new(Owner::Jip, 0x008c_49b7), magic_hit_mark);
    e.on_site(
        SiteSlot::new(Owner::Jip, 0x009c_20bf),
        projectile_impact_site,
    );
    e.on_site(SiteSlot::new(Owner::Jip, 0x00c7_c151), ragdoll_site);
    e.register(COPY_HIT_DATA_ROUTINE, copy_hit_data_routine);
    e.register(CLEAR_HIT_DATA_ROUTINE, clear_hit_data_routine);
    for slot in COPY_HIT_DATA_SLOTS {
        e.mem.set_u32(slot, COPY_HIT_DATA_ROUTINE);
    }
    for slot in CLEAR_HIT_DATA_SLOTS {
        e.mem.set_u32(slot, CLEAR_HIT_DATA_ROUTINE);
    }
    use crate::table::set_handlers;
    set_handlers(e, SET_ON_ANIM_ACTION, Some(set_on_anim_action), None);
    set_handlers(e, SET_ON_PLAY_GROUP, Some(set_on_play_group), None);
    set_handlers(e, SET_ON_HEALTH_DAMAGE, Some(set_on_health_damage), None);
    set_handlers(e, SET_ON_CRIPPLED_LIMB, Some(set_on_crippled_limb), None);
    set_handlers(e, SET_ON_FIRE_WEAPON, Some(set_on_fire_weapon), None);
    set_handlers(e, SET_ON_CRITICAL_HIT, Some(set_on_critical_hit), None);
    set_handlers(e, SET_ON_HIT, Some(set_on_hit), None);
    set_handlers(
        e,
        SET_ON_PROJECTILE_IMPACT,
        Some(set_on_projectile_impact),
        None,
    );
    set_handlers(e, SET_ON_USE_AID_ITEM, Some(set_on_use_aid_item), None);
    set_handlers(e, SET_ON_RELOAD_WEAPON, Some(set_on_reload_weapon), None);
    set_handlers(e, SET_ON_RAGDOLL, Some(set_on_ragdoll), None);
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/serialization.h:
// DoPreLoadGameHousekeeping (this step's part).
/// Before a game loads: the fire-weapon hook keeps one user per scripted actor and per script
/// for every actor (the infinite-ammo users go), and the ragdoll hook loses its users and
/// scripts. The other events stay, as in JIP.
pub fn pre_load(e: &mut Engine) {
    let s = state(e);
    let size = (s.fire_weapon.len() + s.fire_weapon_scripts.len()) as u32;
    let ragdoll = std::mem::take(&mut s.ragdoll);
    if size != 0 {
        e.set_hook_users(Hook::RemoveAmmo, size);
    } else {
        e.set_hook(Hook::RemoveAmmo, false);
    }
    e.set_hook(Hook::OnRagdoll, false);
    for script in ragdoll {
        crate::ln_events::release(e, script);
    }
}

// ---- reads ----

fn form_type(e: &Engine, form: u32) -> u8 {
    e.get(Ptr::<TESForm>::new(form), TESForm::cFormType)
}

fn form_id(e: &Engine, form: u32) -> u32 {
    e.get(Ptr::<TESForm>::new(form), TESForm::iFormID)
}

fn form_flags6(e: &Engine, form: u32) -> u8 {
    e.get(Ptr::<JipFormFlags>::new(form), JipFormFlags::jipFormFlags6)
}

fn actor_flags1(e: &Engine, actor: u32) -> u8 {
    e.get(
        Ptr::<JipActorFlags>::new(actor),
        JipActorFlags::jipActorFlags1,
    )
}

fn actor_flags3(e: &Engine, actor: u32) -> u8 {
    e.get(
        Ptr::<JipActorFlags>::new(actor),
        JipActorFlags::jipActorFlags3,
    )
}

fn set_actor_flags3(e: &mut Engine, actor: u32, flags: u8) {
    e.set(
        Ptr::<JipActorFlags>::new(actor),
        JipActorFlags::jipActorFlags3,
        flags,
    );
}

fn set_form_flag6(e: &mut Engine, form: u32, flag: u8, on: bool) {
    let flags = form_flags6(e, form);
    let flags = if on { flags | flag } else { flags & !flag };
    e.set(
        Ptr::<JipFormFlags>::new(form),
        JipFormFlags::jipFormFlags6,
        flags,
    );
}

/// `EventCallbackScripts::InvokeEventsThis*`: each script of a copy of `scripts` called on
/// `this_obj` with `args`.
fn invoke(e: &mut Engine, scripts: &BTreeSet<u32>, this_obj: u32, args: &[u32]) {
    for script in scripts.clone() {
        crate::script_call::call(e, script, this_obj, args.to_vec());
    }
}

/// The scripts of `key` in `map`, copied (none: empty).
fn scripts_of(map: &ScriptsBy, key: u32) -> Option<BTreeSet<u32>> {
    map.get(&key).cloned()
}

/// This game thread's caller marks.
fn marks(e: &mut Engine) -> &mut u8 {
    let thread = engine_core::thread_host::current_id(e);
    state(e).marks.entry(thread).or_default()
}

/// A caller mark set (1) or cleared (0) by its caller's seam.
fn set_mark(e: &mut Engine, mark: u8, on: bool) {
    let m = marks(e);
    if on {
        *m |= mark;
    } else {
        *m &= !mark;
    }
}

/// Whether the call this handler runs for was marked by its caller; the mark goes (a call made
/// inside this one has another return address).
fn take_mark(e: &mut Engine, mark: u8) -> bool {
    let m = marks(e);
    let had = *m & mark != 0;
    *m &= !mark;
    had
}

// ---- the seams ----

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/hooks.h: SetAnimGroupHook.
/// The seam after a slot's group and sequence are stored (`0x494E19`: the animation, the
/// group, its type, the actor): unless the group is 0xFF or the animation is the player's
/// first-person one, the scripts of the group's type for every actor, then those for this actor
/// (with its play-group flag), each called on the actor with the type.
fn set_anim_group_site(e: &mut Engine, a: &[u32]) -> Option<u32> {
    let (animation, group, kind, actor) = (a[0], a[1] as u16, a[2], a[3]);
    if group == 0xff {
        return None;
    }
    let reference = e.get(Ptr::<Animation>::new(animation), Animation::pActorRef);
    if form_id(e, reference) == PLAYER_ID {
        let first_person = e.get(
            Ptr::<crate::game::PlayerCharacter>::new(reference),
            crate::game::PlayerCharacter::animData1stPerson,
        );
        if first_person == animation {
            return None;
        }
    }
    if !state(e).play_group.is_empty() {
        if let Some(scripts) = scripts_of(&state(e).play_group, kind) {
            invoke(e, &scripts, actor, &[kind]);
        }
    }
    if actor_flags3(e, actor) & ON_PLAY_GROUP != 0 {
        let scripts = state(e)
            .play_group_actors
            .get(&actor)
            .and_then(|m| scripts_of(m, kind));
        if let Some(scripts) = scripts {
            invoke(e, &scripts, actor, &[kind]);
        }
    }
    None
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/hooks.h: OnHitEventHook.
/// The seam before `Script::SetActionFlag` returns (`0x5AC78A`: the form, the list, the
/// flags, the result): for the OnHit flag on an actor's list (not for the magic target's call,
/// `0x8C49BC`), the hit scripts for every actor, then those for this actor (with its hit flag),
/// each called on the actor.
fn on_hit_site(e: &mut Engine, a: &[u32]) -> Option<u32> {
    let (list, flags) = (a[1], a[2]);
    let from_magic = take_mark(e, MARK_MAGIC_HIT);
    if flags & ON_HIT_EVENT == 0 || from_magic || list == 0 {
        return None;
    }
    let actor = list.wrapping_sub(EXTRA_LIST_OFFSET);
    if e.mem.u8(actor) & 8 == 0 || e.mem.u16(actor + 2) != ACTOR_VTABLE_HIGH {
        return None;
    }
    if !state(e).on_hit_scripts.is_empty() {
        let scripts = state(e).on_hit_scripts.clone();
        invoke(e, &scripts, actor, &[]);
    }
    if actor_flags3(e, actor) & ON_HIT != 0 {
        if let Some(scripts) = scripts_of(&state(e).on_hit, actor) {
            invoke(e, &scripts, actor, &[]);
        }
    }
    None
}

/// The magic target's call of `Script::SetActionFlag` (`0x8C49B7`): marked (1) and cleared (0).
fn magic_hit_mark(e: &mut Engine, a: &[u32]) -> Option<u32> {
    set_mark(e, MARK_MAGIC_HIT, a[0] != 0);
    None
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/hooks.h: DamageActorValueHook.
/// The seam before an actor value's callback (`0x66EE72`: the owner, the value, the old value's
/// and the change's bits, the source's owner): for health lost (a change below 0) the actor's
/// health scripts with the change and the source actor; for a condition (perception to brain)
/// that was not zero and is now at most zero, the actor's limb scripts with the limb (the value
/// less perception's); each on the actor, only with the actor's flag. Then the callback is called
/// with the words as they are (the answer).
fn damage_actor_value_site(e: &mut Engine, a: &[u32]) -> Option<u32> {
    let (owner, code, old, delta, source) = (a[0], a[1], a[2], a[3], a[4]);
    if owner == 0 {
        return Some(0);
    }
    let actor = owner.wrapping_sub(AV_OWNER_OFFSET);
    let low = code as u8;
    if low == HEALTH {
        if actor_flags3(e, actor) & ON_HEALTH_DAMAGE == 0 || !(0.0 > f32::from_bits(delta)) {
            return Some(0);
        }
        if let Some(scripts) = scripts_of(&state(e).health_damage, actor) {
            let source = if source != 0 {
                source.wrapping_sub(AV_OWNER_OFFSET)
            } else {
                0
            };
            invoke(e, &scripts, actor, &[delta, source]);
        }
        return Some(0);
    }
    if actor_flags3(e, actor) & ON_CRIPPLED_LIMB == 0
        || (low as i8) < PERCEPTION_CONDITION as i8
        || (low as i8) > BRAIN_CONDITION as i8
        || old == 0
    {
        return Some(0);
    }
    let now = f32::from_bits(old) + f32::from_bits(delta);
    if now > 0.0 {
        return Some(0);
    }
    if let Some(scripts) = scripts_of(&state(e).crippled_limb, actor) {
        let limb = (code & !0xff) | u32::from(low.wrapping_sub(PERCEPTION_CONDITION));
        invoke(e, &scripts, actor, &[limb]);
    }
    Some(0)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/hooks.h: EquipAidItemHook.
/// The seam after an aid item's equip (`0x88C74E`: the item, the actor): the item's scripts
/// (with its handlers flag), each called on the actor with the item.
fn equip_aid_item_site(e: &mut Engine, a: &[u32]) -> Option<u32> {
    let (item, actor) = (a[0], a[1]);
    if form_flags6(e, item) & FORM_EVENT_HANDLERS != 0 {
        if let Some(scripts) = scripts_of(&state(e).use_aid_item, item) {
            invoke(e, &scripts, actor, &[item]);
        }
    }
    None
}

fn process_of(e: &Engine, actor: u32) -> u32 {
    e.get(
        Ptr::<crate::game::Actor>::new(actor),
        crate::game::Actor::pCurrentProcess,
    )
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/hooks.h: ReloadWeaponHook.
/// The seam before the process is told an action (`0x8A7549`: the actor, the action): a new
/// action (the process's current one, a 16-bit word, differs) whose low byte is a reload or a
/// reload loop calls the reload scripts on the actor with the held weapon (0 for none).
fn reload_weapon_site(e: &mut Engine, a: &[u32]) -> Option<u32> {
    let (actor, action) = (a[0], a[1]);
    let process = Ptr::<MiddleHighProcess>::new(process_of(e, actor));
    if e.get(process, MiddleHighProcess::currentAction) as u16 == action as u16 {
        return None;
    }
    let low = action as u8;
    if low != RELOAD && low != RELOAD_LOOP {
        return None;
    }
    let entry = e.get(process, MiddleHighProcess::weaponInfo);
    let weapon = if entry != 0 {
        e.get(
            Ptr::<crate::game::ContChangesEntry>::new(entry),
            crate::game::ContChangesEntry::form,
        )
    } else {
        0
    };
    let scripts = state(e).reload_weapon.clone();
    invoke(e, &scripts, actor, &[weapon]);
    None
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/hooks.h: SetAnimActionHook.
/// The seam at the process's action (`0x8A7551`: the actor, the action, the sequence), in place
/// of its virtual call: the sequence stored; a new action (as a 16-bit word) stored and, when
/// not negative, its scripts for every actor, then those for this actor (with its flag), each
/// called on the actor with the action. Answers that the call is made.
fn set_anim_action_site(e: &mut Engine, a: &[u32]) -> Option<u32> {
    let (actor, action, sequence) = (a[0], a[1], a[2]);
    let process = Ptr::<MiddleHighProcess>::new(process_of(e, actor));
    e.set(process, MiddleHighProcess::currentSequence, sequence);
    if e.get(process, MiddleHighProcess::currentAction) as u16 == action as u16 {
        return Some(0);
    }
    e.set(
        process,
        MiddleHighProcess::currentAction,
        action as u16 as i16,
    );
    if (action as i32) < 0 {
        return Some(0);
    }
    if !state(e).anim_action.is_empty() {
        if let Some(scripts) = scripts_of(&state(e).anim_action, action) {
            invoke(e, &scripts, actor, &[action]);
        }
    }
    if actor_flags3(e, actor) & ON_ANIM_ACTION != 0 {
        let scripts = state(e)
            .anim_action_actors
            .get(&actor)
            .and_then(|m| scripts_of(m, action));
        if let Some(scripts) = scripts {
            invoke(e, &scripts, actor, &[action]);
        }
    }
    Some(0)
}

/// The reload's call of `Actor::ShouldUseAmmo` (`0x8A86B0`): marked (1) and cleared (0).
fn reload_ammo_mark(e: &mut Engine, a: &[u32]) -> Option<u32> {
    set_mark(e, MARK_RELOAD_AMMO, a[0] != 0);
    None
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/hooks.h: RemoveAmmoHook.
/// `Actor::ShouldUseAmmo` replaced (`0x8A8DD0`: the actor, the weapon): unless the reload asks
/// (`0x8A86B5`), the fire-weapon scripts for every actor, then those for this actor (with its
/// flag), each called on the actor with the weapon. Then the answer: true for the player (form
/// id 0x14); false with infinite ammo or no weapon; true for a weapon with flag 2 at `+0x12C`;
/// false for one with bit 7 at `+0x100`; else the actor's teammate byte.
fn remove_ammo_site(e: &mut Engine, a: &[u32]) -> Option<u32> {
    let (actor, weapon) = (a[0], a[1]);
    if !take_mark(e, MARK_RELOAD_AMMO) {
        if !state(e).fire_weapon_scripts.is_empty() {
            let scripts = state(e).fire_weapon_scripts.clone();
            invoke(e, &scripts, actor, &[weapon]);
        }
        if actor_flags3(e, actor) & ON_FIRE_WEAPON != 0 {
            if let Some(scripts) = scripts_of(&state(e).fire_weapon, actor) {
                invoke(e, &scripts, actor, &[weapon]);
            }
        }
    }
    let answer = if form_id(e, actor) == PLAYER_ID {
        true
    } else if actor_flags1(e, actor) & INFINITE_AMMO != 0 || weapon == 0 {
        false
    } else if e.get(
        Ptr::<crate::game::TESObjectWEAP>::new(weapon),
        crate::game::TESObjectWEAP::weaponFlags2,
    ) & 2
        != 0
    {
        true
    } else if e.get(
        Ptr::<crate::game::TESObjectWEAP>::new(weapon),
        crate::game::TESObjectWEAP::weaponFlags1,
    ) & 0x80
        != 0
    {
        false
    } else {
        e.get(
            Ptr::<crate::game::Actor>::new(actor),
            crate::game::Actor::bPlayerTeammate,
        ) != 0
    };
    Some(u32::from(answer))
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/hooks.h: ProjectileImpactHook.
/// The seam after a projectile's impacts are processed (`0x9C20BF`: the projectile): for a base
/// with impact scripts (and its flag), a projectile that is not a beam and has impact data
/// moves to the impact's position, and the scripts are called on it.
fn projectile_impact_site(e: &mut Engine, a: &[u32]) -> Option<u32> {
    let projectile = a[0];
    let base = e.get(
        Ptr::<TESObjectREFR>::new(projectile),
        TESObjectREFR::pObjectReference,
    );
    if form_flags6(e, base) & FORM_EVENT_HANDLERS == 0 {
        return None;
    }
    let scripts = scripts_of(&state(e).projectile_impact, base)?;
    let p = Ptr::<crate::game::Projectile>::new(projectile);
    let kind = e.get(
        Ptr::<crate::game::BGSProjectile>::new(base),
        crate::game::BGSProjectile::projectileType,
    );
    let impact = e.get(p, crate::game::Projectile::impactDataList);
    if kind != BEAM && impact != 0 {
        let position = p.at(crate::game::Projectile::position).addr();
        for i in 0..3 {
            let word = e.mem.u32(impact + 4 + 4 * i);
            e.mem.set_u32(position + 4 * i, word);
        }
    }
    invoke(e, &scripts, projectile, &[]);
    None
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/hooks.h: OnRagdollHook.
/// The seam at a ragdoll controller's `DisableRagdollAnim` (`0xC7C151`: the controller): when
/// its Bip01 node's parent is a fade node with a reference, the ragdoll scripts are called on
/// that reference.
fn ragdoll_site(e: &mut Engine, a: &[u32]) -> Option<u32> {
    let bip01 = e.get(
        Ptr::<BhkRagdollController>::new(a[0]),
        BhkRagdollController::bip01Node,
    );
    if bip01 == 0 {
        return None;
    }
    let parent = e.get(Ptr::<NiAVObject>::new(bip01), NiAVObject::m_pkParent);
    if parent == 0 || e.mem.u32(parent) != BS_FADE_NODE {
        return None;
    }
    let reference = e.get(Ptr::<BSFadeNode>::new(parent), BSFadeNode::linkedObj);
    if reference == 0 {
        return None;
    }
    let scripts = state(e).ragdoll.clone();
    invoke(e, &scripts, reference, &[]);
    None
}

// ---- the hit data ----

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/hooks.h: CopyHitDataHook,
// InvokeActorHitEvents.
/// `MiddleHighProcess::SetLastHitData` as JIP writes it into the vtables (`thiscall`: the
/// process, the hit data): the process's hit data (0x64 bytes from the game's heap at the first
/// hit) gets the hit's 0x60 bytes and, at `+0x60`, its location; a critical hit calls each
/// critical-hit script whose filters match (a copy of the list, in order) on the target with the
/// source and the weapon.
fn copy_hit_data_routine(e: &mut Engine, a: &[u32]) -> Ret {
    let (process, hit) = (Ptr::<MiddleHighProcess>::new(a[0]), a[1]);
    let mut data = e.get(process, MiddleHighProcess::lastHitData);
    if data == 0 {
        data = crate::commands::heap_alloc(e, HIT_DATA_SIZE);
        e.set(process, MiddleHighProcess::lastHitData, data);
    }
    for i in 0..0x18 {
        let word = e.mem.u32(hit + 4 * i);
        e.mem.set_u32(data + 4 * i, word);
    }
    let h = Ptr::<ActorHitData>::new(data);
    let location = e.get(Ptr::<ActorHitData>::new(hit), ActorHitData::eDamageLimb);
    e.set(h, ActorHitData::hitLocation, location as i32);
    if e.get(h, ActorHitData::uiFlags) as u8 & IS_CRITICAL != 0 && !state(e).critical_hit.is_empty()
    {
        let (target, source, weapon) = (
            e.get(h, ActorHitData::pTarget),
            e.get(h, ActorHitData::pAggressor),
            e.get(h, ActorHitData::pWeapon),
        );
        for event in state(e).critical_hit.clone() {
            if (event.target == 0 || event.target == target)
                && (event.source == 0 || event.source == source)
                && (event.weapon == 0 || event.weapon == weapon)
            {
                crate::script_call::call(e, event.script, target, vec![source, weapon]);
            }
        }
    }
    Ret::default()
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/hooks.h: InitJIPHooks (the bytes written
// at `0x92C4A0`).
/// `MiddleHighProcess::ClearLastHitData` as JIP writes it (`thiscall`: the process): the hit
/// data, when there is one, keeps everything but its location, which becomes -1.
fn clear_hit_data_routine(e: &mut Engine, a: &[u32]) -> Ret {
    let data = e.get(
        Ptr::<MiddleHighProcess>::new(a[0]),
        MiddleHighProcess::lastHitData,
    );
    if data != 0 {
        e.set(
            Ptr::<ActorHitData>::new(data),
            ActorHitData::eDamageLimb,
            u32::MAX,
        );
    }
    Ret::default()
}

// ---- the commands ----

/// The script argument read as a script form, or `None` (the command does nothing).
fn script_arg(e: &Engine, read: &Read) -> Option<u32> {
    let script = read.form(0);
    (read.ok && script != 0 && form_type(e, script) == SCRIPT).then_some(script)
}

/// `tList<TESForm> tempList(form)`, or the form list's own list for a list form: the items of
/// its nodes from the first (the first one's even when empty, as JIP's `do ... while` walks).
fn form_or_list(e: &Engine, form: u32) -> Vec<u32> {
    if form != 0 && form_type(e, form) == LIST_FORM {
        let mut items = Vec::new();
        let mut node = Ptr::<crate::game::BGSListForm>::new(form)
            .at(crate::game::BGSListForm::list)
            .addr();
        while node != 0 {
            let n = Ptr::<crate::game::ListNode>::new(node);
            items.push(e.get(n, crate::game::ListNode::data));
            node = e.get(n, crate::game::ListNode::next);
        }
        items
    } else {
        vec![form]
    }
}

fn insert(e: &mut Engine, set: impl Fn(&mut State) -> &mut BTreeSet<u32>, script: u32) -> bool {
    let added = set(state(e)).insert(script);
    if added {
        crate::ln_events::capture(e, script);
    }
    added
}

fn erase(e: &mut Engine, set: impl Fn(&mut State) -> &mut BTreeSet<u32>, script: u32) -> bool {
    let removed = set(state(e)).remove(&script);
    if removed {
        crate::ln_events::release(e, script);
    }
    removed
}

/// The animation commands' maps: the action ones or the play-group ones.
fn anim_maps(s: &mut State, action: bool) -> (&mut ScriptsBy, &mut BTreeMap<u32, ScriptsBy>) {
    if action {
        (&mut s.anim_action, &mut s.anim_action_actors)
    } else {
        (&mut s.play_group, &mut s.play_group_actors)
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_actor.h:
// SetOnAnimationEventHandler_Execute.
/// Adds or removes a script's event for an action or group type: for every actor (no actor or
/// list: a type's first script is one user of the hook), or for each actor of the form or list
/// (an actor's first type is one user, with its flag; its last type gives both back).
fn set_on_animation(e: &mut Engine, w: &[u32], action: bool) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let Some(script) = script_arg(e, &read) else {
        return handled(true);
    };
    let add = read.int(1, 0) != 0;
    set_animation_event(e, script, add, read.form(2), read.int(3, 0), action);
    handled(true)
}

/// [`set_on_animation`] once its arguments are read: `target` the actor or list, `id` the
/// action or group type.
fn set_animation_event(e: &mut Engine, script: u32, add: bool, target: u32, id: u32, action: bool) {
    let (hook, flag) = if action {
        (Hook::SetAnimAction, ON_ANIM_ACTION)
    } else {
        (Hook::SetAnimGroup, ON_PLAY_GROUP)
    };
    let items = form_or_list(e, target);
    if items[0] == 0 {
        // `tempList.Empty()`: the list's first node holds nothing.
        if add {
            let new = !anim_maps(state(e), action).0.contains_key(&id);
            if new {
                anim_maps(state(e), action).0.insert(id, BTreeSet::new());
                e.add_hook_user(hook, true);
            }
            insert(
                e,
                |s| anim_maps(s, action).0.get_mut(&id).expect("made above"),
                script,
            );
        } else if anim_maps(state(e), action).0.contains_key(&id)
            && erase(
                e,
                |s| anim_maps(s, action).0.get_mut(&id).expect("found"),
                script,
            )
            && anim_maps(state(e), action).0[&id].is_empty()
        {
            anim_maps(state(e), action).0.remove(&id);
            e.add_hook_user(hook, false);
        }
        return;
    }
    for actor in items {
        if actor == 0 {
            continue;
        }
        if add {
            if !crate::commands::is_actor(e, actor) {
                continue;
            }
            if !anim_maps(state(e), action).1.contains_key(&actor) {
                anim_maps(state(e), action)
                    .1
                    .insert(actor, ScriptsBy::new());
                let flags = actor_flags3(e, actor);
                set_actor_flags3(e, actor, flags | flag);
                e.add_hook_user(hook, true);
            }
            insert(
                e,
                |s| {
                    anim_maps(s, action)
                        .1
                        .get_mut(&actor)
                        .expect("made above")
                        .entry(id)
                        .or_default()
                },
                script,
            );
        } else if anim_maps(state(e), action)
            .1
            .get(&actor)
            .is_some_and(|m| m.contains_key(&id))
            && erase(
                e,
                |s| {
                    anim_maps(s, action)
                        .1
                        .get_mut(&actor)
                        .and_then(|m| m.get_mut(&id))
                        .expect("found")
                },
                script,
            )
            && anim_maps(state(e), action).1[&actor][&id].is_empty()
        {
            let maps = anim_maps(state(e), action).1;
            let actor_map = maps.get_mut(&actor).expect("found");
            actor_map.remove(&id);
            if actor_map.is_empty() {
                maps.remove(&actor);
                let flags = actor_flags3(e, actor);
                set_actor_flags3(e, actor, flags & !flag);
                e.add_hook_user(hook, false);
            }
        }
    }
}

fn set_on_anim_action(e: &mut Engine, w: &[u32]) -> Ret {
    set_on_animation(e, w, true)
}

fn set_on_play_group(e: &mut Engine, w: &[u32]) -> Ret {
    set_on_animation(e, w, false)
}

/// `s_actorEventInfos`: the four actor events' maps, hooks and flags.
#[derive(Clone, Copy)]
enum ActorEvent {
    HealthDamage,
    CrippledLimb,
    FireWeapon,
    Hit,
}

impl ActorEvent {
    fn filtered(self, s: &mut State) -> &mut ScriptsBy {
        match self {
            ActorEvent::HealthDamage => &mut s.health_damage,
            ActorEvent::CrippledLimb => &mut s.crippled_limb,
            ActorEvent::FireWeapon => &mut s.fire_weapon,
            ActorEvent::Hit => &mut s.on_hit,
        }
    }
    /// The scripts for every actor (none for the damage events: their commands need a form).
    fn unfiltered(self, s: &mut State) -> Option<&mut BTreeSet<u32>> {
        match self {
            ActorEvent::FireWeapon => Some(&mut s.fire_weapon_scripts),
            ActorEvent::Hit => Some(&mut s.on_hit_scripts),
            _ => None,
        }
    }
    fn hook(self) -> Hook {
        match self {
            ActorEvent::HealthDamage | ActorEvent::CrippledLimb => Hook::DamageActorValue,
            ActorEvent::FireWeapon => Hook::RemoveAmmo,
            ActorEvent::Hit => Hook::OnHitEvent,
        }
    }
    fn flag(self) -> u8 {
        match self {
            ActorEvent::HealthDamage => ON_HEALTH_DAMAGE,
            ActorEvent::CrippledLimb => ON_CRIPPLED_LIMB,
            ActorEvent::FireWeapon => ON_FIRE_WEAPON,
            ActorEvent::Hit => ON_HIT,
        }
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_actor.h:
// SetActorEventHandler_Execute.
/// Adds or removes a script's actor event: for every actor without a form (each script one user
/// of the hook), or for each actor of the form or list (an actor's first script one user; the
/// actor's flag set with each script added and cleared with each one removed, as JIP does; its
/// last script gives the user back).
fn set_actor_event(e: &mut Engine, w: &[u32], event: ActorEvent) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let Some(script) = script_arg(e, &read) else {
        return handled(true);
    };
    let add = read.int(1, 0) != 0;
    let target = read.form(2);
    let hook = event.hook();
    if target == 0 {
        // JIP's damage events have no list for every actor (their form is not optional).
        if event.unfiltered(state(e)).is_none() {
            return handled(true);
        }
        if add {
            if insert(e, move |s| event.unfiltered(s).expect("checked"), script) {
                e.add_hook_user(hook, true);
            }
        } else if erase(e, move |s| event.unfiltered(s).expect("checked"), script) {
            e.add_hook_user(hook, false);
        }
        return handled(true);
    }
    let flag = event.flag();
    for actor in form_or_list(e, target) {
        if actor == 0 {
            continue;
        }
        if add {
            if !crate::commands::is_actor(e, actor) {
                continue;
            }
            if !event.filtered(state(e)).contains_key(&actor) {
                event.filtered(state(e)).insert(actor, BTreeSet::new());
                e.add_hook_user(hook, true);
            }
            insert(
                e,
                move |s| event.filtered(s).get_mut(&actor).expect("made above"),
                script,
            );
            let flags = actor_flags3(e, actor);
            set_actor_flags3(e, actor, flags | flag);
        } else if event.filtered(state(e)).contains_key(&actor)
            && erase(
                e,
                move |s| event.filtered(s).get_mut(&actor).expect("found"),
                script,
            )
        {
            if crate::commands::is_actor(e, actor) {
                let flags = actor_flags3(e, actor);
                set_actor_flags3(e, actor, flags & !flag);
            }
            if event.filtered(state(e))[&actor].is_empty() {
                event.filtered(state(e)).remove(&actor);
                e.add_hook_user(hook, false);
            }
        }
    }
    handled(true)
}

fn set_on_health_damage(e: &mut Engine, w: &[u32]) -> Ret {
    set_actor_event(e, w, ActorEvent::HealthDamage)
}

fn set_on_crippled_limb(e: &mut Engine, w: &[u32]) -> Ret {
    set_actor_event(e, w, ActorEvent::CrippledLimb)
}

fn set_on_fire_weapon(e: &mut Engine, w: &[u32]) -> Ret {
    set_actor_event(e, w, ActorEvent::FireWeapon)
}

fn set_on_hit(e: &mut Engine, w: &[u32]) -> Ret {
    set_actor_event(e, w, ActorEvent::Hit)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_actor.h:
// Cmd_SetOnReloadWeaponEventHandler_Execute, Cmd_SetOnRagdollEventHandler_Execute.
/// Adds or removes a script of a list event; each script is one user of the event's hook.
fn set_on_list_event(
    e: &mut Engine,
    w: &[u32],
    hook: Hook,
    set: fn(&mut State) -> &mut BTreeSet<u32>,
) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let Some(script) = script_arg(e, &read) else {
        return handled(true);
    };
    if read.int(1, 0) != 0 {
        if insert(e, set, script) {
            e.add_hook_user(hook, true);
        }
    } else if erase(e, set, script) {
        e.add_hook_user(hook, false);
    }
    handled(true)
}

fn set_on_reload_weapon(e: &mut Engine, w: &[u32]) -> Ret {
    set_on_list_event(e, w, Hook::ReloadWeapon, |s| &mut s.reload_weapon)
}

fn set_on_ragdoll(e: &mut Engine, w: &[u32]) -> Ret {
    set_on_list_event(e, w, Hook::OnRagdoll, |s| &mut s.ragdoll)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_inventory.h:
// Cmd_SetOnUseAidItemEventHandler_Execute; functions_jip/jip_fn_projectile.h:
// Cmd_SetOnProjectileImpactEventHandler_Execute.
/// Adds or removes a script's event for each form of `kind` in the form or list: a form's
/// first script is one user of the hook and sets the form's flag; its last gives both back.
fn set_on_form_event(
    e: &mut Engine,
    w: &[u32],
    kind: u8,
    hook: Hook,
    map: fn(&mut State) -> &mut ScriptsBy,
) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let Some(script) = script_arg(e, &read) else {
        return handled(true);
    };
    let add = read.int(1, 0) != 0;
    for form in form_or_list(e, read.form(2)) {
        if form == 0 || form_type(e, form) != kind {
            continue;
        }
        if add {
            if !map(state(e)).contains_key(&form) {
                map(state(e)).insert(form, BTreeSet::new());
                e.add_hook_user(hook, true);
            }
            insert(
                e,
                move |s| map(s).get_mut(&form).expect("made above"),
                script,
            );
            set_form_flag6(e, form, FORM_EVENT_HANDLERS, true);
        } else if map(state(e)).contains_key(&form)
            && erase(e, move |s| map(s).get_mut(&form).expect("found"), script)
            && map(state(e))[&form].is_empty()
        {
            map(state(e)).remove(&form);
            e.add_hook_user(hook, false);
            set_form_flag6(e, form, FORM_EVENT_HANDLERS, false);
        }
    }
    handled(true)
}

fn set_on_use_aid_item(e: &mut Engine, w: &[u32]) -> Ret {
    set_on_form_event(e, w, ALCHEMY_ITEM, Hook::EquipAidItem, |s| {
        &mut s.use_aid_item
    })
}

fn set_on_projectile_impact(e: &mut Engine, w: &[u32]) -> Ret {
    set_on_form_event(e, w, PROJECTILE, Hook::ProjectileImpact, |s| {
        &mut s.projectile_impact
    })
}

/// `IsReference` (`TESForm`'s vtable `+0xF0`) answers true (`008D0360`).
fn is_reference(e: &Engine, form: u32) -> bool {
    e.mem.u32(e.mem.u32(form) + 0xf0) == 0x008d_0360
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_misc_ref.h:
// Cmd_SetOnCriticalHitEventHandler_Execute (CriticalHitEventFind, CriticalHitEventRemove,
// internal/jip_core.h).
/// Adds or removes a critical-hit event with its filters (a target that is not an actor, a
/// source that is not a reference, or a weapon that is not one, as JIP sorts them out). Adding
/// what an event already covers does nothing; otherwise every event the new filters cover is
/// removed first, and the new one appended. No hook user: the hit-data routine is always in.
fn set_on_critical_hit(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let Some(script) = script_arg(e, &read) else {
        return handled(true);
    };
    let add = read.int(1, 0) != 0;
    let (mut target, mut source, weapon) = (read.form(2), read.form(3), read.form(4));
    if target != 0 {
        if !crate::commands::is_actor(e, target) {
            if source == 0 {
                return handled(true);
            }
            target = 0;
        }
        if source != 0 {
            if !is_reference(e, source) {
                if weapon == 0 {
                    return handled(true);
                }
                source = 0;
            }
            if weapon != 0 && form_type(e, weapon) != WEAPON {
                return handled(true);
            }
        }
    }
    let new = CriticalHit {
        target,
        source,
        weapon,
        script,
    };
    let covers = |old: &CriticalHit, by: &CriticalHit| {
        old.script == by.script
            && (by.target == 0 || old.target == by.target)
            && (by.source == 0 || old.source == by.source)
            && (by.weapon == 0 || old.weapon == by.weapon)
    };
    if add && state(e).critical_hit.iter().any(|old| covers(&new, old)) {
        return handled(true);
    }
    let mut i = state(e).critical_hit.len();
    while i > 0 {
        i -= 1;
        let old = state(e).critical_hit[i];
        if covers(&old, &new) {
            state(e).critical_hit.remove(i);
            crate::ln_events::release(e, old.script);
        }
    }
    if add {
        state(e).critical_hit.push(new);
        crate::ln_events::capture(e, script);
    }
    handled(true)
}

#[cfg(test)]
#[path = "actor_hooks_tests.rs"]
mod tests;
