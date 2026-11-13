//! JIP LN NVSE's actor AI hooks (step 3.5b, third part; `internal/hooks.h` (JIP LN NVSE)): combat
//! a script disabled or a target it forced, teammates' health after a wake-up or `ResetHealth`,
//! teammates told to follow or wait when the player teleports, equipment a script locked, forced
//! detection values, a push without a ragdoll, and disabled scripts' events; with the commands
//! that switch them on.
//!
//! The seams are in the translations (step 3.5b's first part, each behind its hook's user count);
//! the handlers here answer them as JIP's patched code does, reading JIP's actor flag bytes
//! (`+0x105`, `+0x106`) and maps. Nothing here runs while JIP answers "not installed".
//!
//! Not here yet: `PushActorNoRagdoll` (the only user of the ApplyActorVelocity hook: its push
//! force and push are JIP's own SSE routines, a later batch) and `CCCSetEquipped` (with JIP's
//! always-on `GetCannotWear` patch at `0x418B10`, which reads the same `s_forceEquipCall`).

use crate::game::{Actor, JipActorFlags, TESForm, TESObjectREFR};
use crate::Jip;
use engine_core::extend::{Hook, Owner, SiteSlot};
use engine_core::prelude::*;
use engine_core::x87::{x87_mul, x87_sub};
use nvse::commands::Read;
use nvse::{handled, CommandArgs};
use std::collections::BTreeMap;

/// The commands' opcodes.
pub const GET_COMBAT_DISABLED: u32 = 0x2754;
pub const SET_COMBAT_DISABLED: u32 = 0x2755;
pub const TOGGLE_NO_HEALTH_RESET: u32 = 0x2756;
pub const GET_TEAMMATE_USING_AMMO: u32 = 0x275a;
pub const SET_TEAMMATE_USING_AMMO: u32 = 0x275b;
pub const CCC_SET_FOLLOW_STATE: u32 = 0x2760;
pub const LOCK_EQUIPMENT: u32 = 0x2768;
pub const FORCE_ACTOR_DETECTION_VALUE: u32 = 0x27ae;
pub const GET_SCRIPT_DISABLED: u32 = 0x27e3;
pub const SET_SCRIPT_DISABLED: u32 = 0x27e4;
/// The ten.
pub const COMMANDS: [u32; 10] = [
    GET_COMBAT_DISABLED,
    SET_COMBAT_DISABLED,
    TOGGLE_NO_HEALTH_RESET,
    GET_TEAMMATE_USING_AMMO,
    SET_TEAMMATE_USING_AMMO,
    CCC_SET_FOLLOW_STATE,
    LOCK_EQUIPMENT,
    FORCE_ACTOR_DETECTION_VALUE,
    GET_SCRIPT_DISABLED,
    SET_SCRIPT_DISABLED,
];

/// The seams whose handlers are here (engine-core `SITES`, owner JIP, step 3.5b).
pub const SITES: [u32; 13] = [
    0x005a_c782,
    0x005c_6b8b,
    0x0060_47c0,
    0x0088_c87a,
    0x0089_1c86,
    0x008a_0963,
    0x008a_152d,
    0x008a_d471,
    0x0090_01c6,
    0x0098_6c79,
    0x0099_9857,
    0x009d_a8e3,
    0x00c6_d4e4,
];

/// Each command's hooks and every site JIP patches for them (`InitJIPHooks`, the caller markers
/// included): a command may add users only while all of them have seams.
pub const COMMAND_SITES: [(u32, &[u32]); 5] = [
    (SET_COMBAT_DISABLED, &[0x0090_01c6, 0x0098_6c79]),
    (TOGGLE_NO_HEALTH_RESET, &[0x005c_6b8b, 0x008a_0963]),
    (CCC_SET_FOLLOW_STATE, &[0x008a_d471]),
    (
        LOCK_EQUIPMENT,
        &[
            0x0088_c87a,
            0x0060_47c0,
            0x0099_9857,
            0x009d_a8e3,
            0x0089_1c86,
            0x0091_559d,
            0x005c_d736,
            0x0060_678f,
        ],
    ),
    (FORCE_ACTOR_DETECTION_VALUE, &[0x008a_152d]),
];

/// `kHookActorFlag1_*`, `kHookActorFlag2_ForceDetectionVal`, `kHookRefFlag5F_Update3D`
/// (`internal/hooks.h` (JIP LN NVSE)).
const COMBAT_DISABLED: u8 = 1 << 0;
const FORCE_COMBAT_TARGET: u8 = 1 << 1;
const COMBAT_AI_MODIFIED: u8 = COMBAT_DISABLED | FORCE_COMBAT_TARGET;
const INFINITE_AMMO: u8 = 1 << 2;
const LOCKED_EQUIPMENT: u8 = 1 << 3;
const PC_TELEPORT_WAIT: u8 = 1 << 5;
const PC_TELEPORT_FOLLOW: u8 = 1 << 6;
const PC_TELEPORT_AI: u8 = PC_TELEPORT_WAIT | PC_TELEPORT_FOLLOW;
const FORCE_DETECTION_VALUE: u8 = 1 << 0;
const UPDATE_3D: u8 = 1 << 0;
/// `kFormType_*`, `kAVCode_Health`, `kXData_ExtraScript`, `kXData_ExtraCombatStyle`.
const SCRIPT: u8 = 0x11;
const ARMOR: u8 = 0x18;
const WEAPON: u8 = 0x28;
const HEALTH: u32 = 0x10;
const EXTRA_SCRIPT: u32 = 0x0d;
const EXTRA_COMBAT_STYLE: u32 = 0x69;
/// `Character`'s vtable (JIP's `kVtbl_Character`).
const VTBL_CHARACTER: u32 = 0x0108_6a6c;

/// The game's functions JIP calls: `BaseExtraList::GetByType` (`thiscall`: list, type),
/// `Actor::RestoreActorValue` (`thiscall`: actor, code, amount), `Actor::EquipItem` (`thiscall`:
/// actor, item, count, extra list, 1, 0, 1), `ContChangesEntry`'s deleting destructor
/// (`thiscall`: entry, 1); and the vtable entries:
/// `Actor::GetPreferedWeapon` (`+0x3BC`, `00891c80`), `Actor::GetCombatController` (`+0x428`,
/// `008a02d0`), `MagicTarget`'s `+0x14` (an enchantment applied), `ActorValueOwner`'s
/// `GetBaseActorValue` (`+4`) and `GetActorValue` (`+0xC`), both floats.
const GET_BY_TYPE: u32 = 0x0041_0220;
const RESTORE_ACTOR_VALUE: u32 = 0x0088_b740;
const EQUIP_ITEM: u32 = 0x0088_c650;
const DELETE_ENTRY: u32 = 0x0044_59e0;
const GET_PREFERED_WEAPON: u32 = 0x3bc;
const GET_COMBAT_CONTROLLER: u32 = 0x428;
const MAGIC_TARGET_APPLY: u32 = 0x14;
const AV_BASE: u32 = 0x4;
const AV_CURRENT: u32 = 0xc;
/// The console's open byte (`0x11DEA2E`): JIP's `ResetHPCommandHook` lets the reset run while it is
/// set.
const CONSOLE_OPEN: u32 = 0x011d_ea2e;

/// The calls marked for the hooks that read their caller's return address: `ResolveEquipment`'s
/// first `EquipItem` call (`0x9155A2`), `SetCombatStyle`'s call reaching `ReEquipAll` through
/// `0x60678F` (the grand-caller `0x5CD73B`). JIP's own `GetPreferedWeapon` call (its return
/// address in the plugin: `[ebp+7] != 0`) is marked at a JIP routine address.
const RESOLVE_EQUIP_CALL: u32 = 0x0091_559d;
const COMBAT_STYLE_CALL: u32 = 0x0060_678f;
const JIP_PREFERED_WEAPON_CALL: u32 = crate::identity::ROUTINES + 0x508;
const _: () = assert!(JIP_PREFERED_WEAPON_CALL < engine_core::mem::EXTENDER_CODE.end);

/// JIP's AI hook state.
#[derive(Clone, Debug, Default)]
pub struct State {
    /// `s_forceCombatTargetMap`: actor -> the target a script forced.
    pub force_combat_target: BTreeMap<u32, u32>,
    /// `s_forceDetectionValueMap`: actor -> its packed value (bit 16: added to the formula's).
    pub force_detection_value: BTreeMap<u32, u32>,
    /// `s_healthRestoreRatio`.
    pub health_restore_ratio: f32,
    /// `s_forceEquipCall` (set by `CCCSetEquipped`, not built yet).
    pub force_equip_call: bool,
}

fn state(e: &mut Engine) -> &mut State {
    &mut e.ext::<Jip>().ai_hooks
}

/// Puts the handlers and the commands in, when JIP answers "installed".
pub fn install(e: &mut Engine, answers_installed: bool) {
    if !answers_installed {
        return;
    }
    type Handler = fn(&mut Engine, &[u32]) -> Option<u32>;
    let sites: [(u32, Handler); 13] = [
        (0x005a_c782, merge_event_mask_site),
        (0x005c_6b8b, reset_hp_command_site),
        (0x0060_47c0, re_equip_all_site),
        (0x0088_c87a, equip_item_site),
        (0x0089_1c86, get_prefered_weapon_site),
        (0x008a_0963, reset_hp_wake_up_site),
        (0x008a_152d, detection_value_site),
        (0x008a_d471, teleport_with_pc_site),
        (0x0090_01c6, start_combat_site),
        (0x0098_6c79, set_combat_target_site),
        (0x0099_9857, weapon_switch_select_site),
        (0x009d_a8e3, weapon_switch_unequip_site),
        (0x00c6_d4e4, apply_actor_velocity_site),
    ];
    for (addr, handler) in sites {
        e.on_site(SiteSlot::new(Owner::Jip, addr), handler);
    }
    use crate::table::set_handlers;
    set_handlers(e, GET_COMBAT_DISABLED, Some(get_combat_disabled), None);
    set_handlers(e, SET_COMBAT_DISABLED, Some(set_combat_disabled), None);
    set_handlers(
        e,
        TOGGLE_NO_HEALTH_RESET,
        Some(toggle_no_health_reset),
        None,
    );
    set_handlers(
        e,
        GET_TEAMMATE_USING_AMMO,
        Some(get_teammate_using_ammo),
        None,
    );
    set_handlers(
        e,
        SET_TEAMMATE_USING_AMMO,
        Some(set_teammate_using_ammo),
        None,
    );
    set_handlers(e, CCC_SET_FOLLOW_STATE, Some(ccc_set_follow_state), None);
    set_handlers(e, LOCK_EQUIPMENT, Some(lock_equipment), None);
    set_handlers(
        e,
        FORCE_ACTOR_DETECTION_VALUE,
        Some(force_actor_detection_value),
        None,
    );
    set_handlers(e, GET_SCRIPT_DISABLED, Some(get_script_disabled), None);
    set_handlers(e, SET_SCRIPT_DISABLED, Some(set_script_disabled), None);
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/serialization.h:
// DoPreLoadGameHousekeeping (this step's part).
/// Before a game loads: the combat, teleport and equipment hooks lose their users; the forced
/// targets and detection values go, with their hooks' users. The actors' flag bytes stay, as in
/// JIP.
pub fn pre_load(e: &mut Engine) {
    e.set_hook(Hook::StartCombat, false);
    if !state(e).force_combat_target.is_empty() {
        state(e).force_combat_target.clear();
        e.set_hook(Hook::SetCombatTarget, false);
    }
    e.set_hook(Hook::TeleportWithPC, false);
    e.set_hook(Hook::LockEquipment, false);
    if !state(e).force_detection_value.is_empty() {
        state(e).force_detection_value.clear();
        e.set_hook(Hook::GetDetectionValue, false);
    }
}

// ---- reads ----

fn form_type(e: &Engine, form: u32) -> u8 {
    e.get(Ptr::<TESForm>::new(form), TESForm::cFormType)
}

fn flags1(e: &Engine, actor: u32) -> u8 {
    e.get(
        Ptr::<JipActorFlags>::new(actor),
        JipActorFlags::jipActorFlags1,
    )
}

fn set_flags1(e: &mut Engine, actor: u32, flags: u8) {
    e.set(
        Ptr::<JipActorFlags>::new(actor),
        JipActorFlags::jipActorFlags1,
        flags,
    );
}

fn flags2(e: &Engine, actor: u32) -> u8 {
    e.get(
        Ptr::<JipActorFlags>::new(actor),
        JipActorFlags::jipActorFlags2,
    )
}

fn set_flags2(e: &mut Engine, actor: u32, flags: u8) {
    e.set(
        Ptr::<JipActorFlags>::new(actor),
        JipActorFlags::jipActorFlags2,
        flags,
    );
}

fn teammate(e: &Engine, actor: u32) -> bool {
    e.get(Ptr::<Actor>::new(actor), Actor::bPlayerTeammate) != 0
}

/// `BaseExtraList::GetByType` on a reference's extra list (`+0x44`).
fn extra_of(e: &mut Engine, reference: u32, kind: u32) -> u32 {
    e.call(GET_BY_TYPE, &args![reference + 0x44, kind]).u32()
}

// ---- the seams ----

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/hooks.h: StartCombatHook.
/// `HighProcess::EnterCombat`'s entry (`0x9001C6`: the actor): an actor a script disabled combat
/// for answers false at once.
fn start_combat_site(e: &mut Engine, a: &[u32]) -> Option<u32> {
    (flags1(e, a[0]) & COMBAT_DISABLED != 0).then_some(0)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/hooks.h: SetCombatTargetHook;
// nvse/GameObjects.cpp: Actor::IsInCombatWith.
/// The combat group's target pick (`0x986C79`: the actor): an actor whose target a script forced
/// gets it, while that target is among the actor's combat targets.
fn set_combat_target_site(e: &mut Engine, a: &[u32]) -> Option<u32> {
    let actor = a[0];
    if flags1(e, actor) & FORCE_COMBAT_TARGET == 0 {
        return None;
    }
    let target = state(e)
        .force_combat_target
        .get(&actor)
        .copied()
        .unwrap_or(0);
    (target != 0 && is_in_combat_with(e, actor, target)).then_some(target)
}

/// `Actor::IsInCombatWith`: the target is in the actor's combat targets (the array at `+0x12C`:
/// its data `+4`, count `+8`).
fn is_in_combat_with(e: &Engine, actor: u32, target: u32) -> bool {
    let targets = e.mem.u32(actor + 0x12c);
    if targets == 0 {
        return false;
    }
    let (data, count) = (e.mem.u32(targets + 4), e.mem.u32(targets + 8));
    (0..count).any(|i| e.mem.u32(data + i * 4) == target)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/hooks.h: ResetHPCommandHook.
/// `ResetHealth`'s command after its actor test (`0x5C6B8B`: the reference, the event list):
/// for a teammate run by its own object script (the event list is its script's), not while the
/// console is open, the reset is skipped when an event of the list with the
/// `0x40` bit names no object or this one.
fn reset_hp_command_site(e: &mut Engine, a: &[u32]) -> Option<u32> {
    let (reference, event_list) = (a[0], a[1]);
    if !teammate(e, reference) || event_list == 0 || e.mem.u8(CONSOLE_OPEN) != 0 {
        return None;
    }
    let script = extra_of(e, reference, EXTRA_SCRIPT);
    if script == 0 || e.mem.u32(script + 0x10) != event_list {
        return None;
    }
    let mut node = e.mem.u32(event_list + 8);
    while node != 0 {
        let event = e.mem.u32(node);
        if event == 0 {
            return None;
        }
        node = e.mem.u32(node + 4);
        if e.mem.u8(event + 4) & 0x40 == 0 {
            continue;
        }
        let object = e.mem.u32(event);
        if object == 0 || object == reference {
            return Some(1);
        }
    }
    None
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/hooks.h: ResetHPWakeUpHook.
/// `Actor::RestoreFullHealthAndConditions`'s entry (`0x8A0963`: the actor): a teammate gets back
/// `s_healthRestoreRatio` of its base health less its health (x87 maths, stored as a float), and
/// nothing else is restored.
fn reset_hp_wake_up_site(e: &mut Engine, a: &[u32]) -> Option<u32> {
    let actor = a[0];
    if !teammate(e, actor) {
        return None;
    }
    let owner = actor + 0xa4;
    let base = e.vcall(owner, AV_BASE, &args![HEALTH]).f32();
    let ratio = state(e).health_restore_ratio;
    let product = x87_mul(f64::from(base), f64::from(ratio));
    let current = e.vcall(owner, AV_CURRENT, &args![HEALTH]).f32();
    let amount = x87_sub(product, f64::from(current)) as f32;
    e.call(RESTORE_ACTOR_VALUE, &args![actor, HEALTH, amount]);
    Some(1)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/hooks.h: TeleportWithPCHook.
/// The player's teleport, a teammate's waiting test (`0x8AD471`: the actor): not a teammate, or
/// told to follow, moves with the player (0); told to wait stays (1); else the exe's test.
fn teleport_with_pc_site(e: &mut Engine, a: &[u32]) -> Option<u32> {
    let actor = a[0];
    if !teammate(e, actor) {
        return Some(0);
    }
    let flags = flags1(e, actor);
    if flags & PC_TELEPORT_FOLLOW != 0 {
        Some(0)
    } else if flags & PC_TELEPORT_WAIT != 0 {
        Some(1)
    } else {
        None
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/hooks.h: GetDetectionValueHook.
/// The detection formula's call (`0x8A152D`: the actor detected): an actor with a forced value
/// answers it (missing from the map: 0, as `UnorderedMap::Get`).
fn detection_value_site(e: &mut Engine, a: &[u32]) -> Option<u32> {
    let actor = a[0];
    if flags2(e, actor) & FORCE_DETECTION_VALUE == 0 {
        return None;
    }
    Some(
        state(e)
            .force_detection_value
            .get(&actor)
            .copied()
            .unwrap_or(0),
    )
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/hooks.h: MergeEventMaskHook.
/// `Script::SetActionFlag`'s merge of the locals' event mask (`0x5AC782`: the locals): a script
/// a script disabled (data length 0) answers true without merging.
fn merge_event_mask_site(e: &mut Engine, a: &[u32]) -> Option<u32> {
    let script = e.mem.u32(a[0]);
    (e.mem.u32(script + 0x20) == 0).then_some(1)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/hooks.h: ApplyActorVelocityHook.
/// `bhkCharacterController::SetVelocityModifier`'s entry (`0xC6D4E4`: the controller): while its
/// push byte's bit 7 is set and its push time (`+0x524`) is above 0 (or not a number), nothing is
/// changed; else the bit is cleared and the exe's code runs.
fn apply_actor_velocity_site(e: &mut Engine, a: &[u32]) -> Option<u32> {
    let controller = a[0];
    let flags = e.mem.u8(controller + 0x417);
    if flags & 0x80 == 0 {
        return None;
    }
    // `comiss 0, [+0x524]` then `jnb`: the bit goes when 0 >= the time (an ordered compare).
    if 0.0 >= e.mem.f32(controller + 0x524) {
        e.mem.set_u8(controller + 0x417, flags & 0x7f);
        None
    } else {
        Some(1)
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/hooks.h: EquipItemHook.
/// `Actor::EquipObject`'s type test (`0x88C87A`: the item, the actor): a weapon is not equipped
/// on an actor whose equipment a script locked, unless JIP's own equip call or
/// `ResolveEquipment`'s first equip call (marked) made it. The mark is taken first.
fn equip_item_site(e: &mut Engine, a: &[u32]) -> Option<u32> {
    let from_resolve = e.take_caller_mark(RESOLVE_EQUIP_CALL);
    let (item, actor) = (a[0], a[1]);
    if form_type(e, item) != WEAPON || state(e).force_equip_call || from_resolve {
        return None;
    }
    (flags1(e, actor) & LOCKED_EQUIPMENT != 0).then_some(0)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/hooks.h: WeaponSwitchUnequipHook.
/// The switch-weapon procedure's unequip (`0x9DA8E3`: the attacker): a locked attacker keeps
/// its weapon.
fn weapon_switch_unequip_site(e: &mut Engine, a: &[u32]) -> Option<u32> {
    (flags1(e, a[0]) & LOCKED_EQUIPMENT != 0).then_some(0)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/hooks.h: GetPreferedWeaponHook.
/// `Actor::GetPreferedWeapon`'s entry (`0x891C86`: the actor): a locked actor has none (0),
/// except for JIP's own call (marked). The mark is taken first.
fn get_prefered_weapon_site(e: &mut Engine, a: &[u32]) -> Option<u32> {
    let from_jip = e.take_caller_mark(JIP_PREFERED_WEAPON_CALL);
    (!from_jip && flags1(e, a[0]) & LOCKED_EQUIPMENT != 0).then_some(0)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/hooks.h: WeaponSwitchSelectHook.
/// The combat state's weapon rating (`0x999857`: the attacker, the acquired item's entry, the
/// weapons list): a locked attacker with an acquired weapon it has ammo for (or that needs
/// none) rates only that weapon: the list's count is cut after it and the loop starts at it.
fn weapon_switch_select_site(e: &mut Engine, a: &[u32]) -> Option<u32> {
    let (attacker, acquired, weapons) = (a[0], a[1], a[2]);
    if flags1(e, attacker) & LOCKED_EQUIPMENT == 0 || acquired == 0 {
        return None;
    }
    let weapon = e.mem.u32(acquired + 8);
    let ammo = e.mem.u32(weapon + 0xa8);
    if ammo != 0 && crate::overrides::item_count(e, attacker, ammo) <= 0 {
        return None;
    }
    let (data, count) = (e.mem.u32(weapons + 4), e.mem.u32(weapons + 8));
    let index = (0..count).find(|&i| e.mem.u32(data + i * 0x14) == weapon)?;
    e.mem.set_u32(weapons + 8, index + 1);
    Some(index)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/hooks.h: ReEquipAllHook,
// ReApplyEnchantments.
/// `TESNPC::ReEquipAll`'s entry (`0x6047C0`: the base, the actor), for a locked actor only: from
/// `SetCombatStyle` (marked, taken first) the actor equips its preferred weapon unless its held
/// weapon already suits its combat style; from anywhere else, an actor with no 3D and not
/// waiting for an Update3D gets its worn armour's enchantments applied again. Nothing else is
/// re-equipped.
fn re_equip_all_site(e: &mut Engine, a: &[u32]) -> Option<u32> {
    let from_combat_style = e.take_caller_mark(COMBAT_STYLE_CALL);
    let actor = a[1];
    if flags1(e, actor) & LOCKED_EQUIPMENT == 0 {
        return None;
    }
    if !from_combat_style {
        let refr = Ptr::<TESObjectREFR>::new(actor);
        if e.get(refr, TESObjectREFR::pLoadedData) == 0
            && e.get(refr, TESObjectREFR::jipRefFlags5F) & UPDATE_3D == 0
        {
            re_apply_enchantments(e, actor);
        }
        return Some(0);
    }
    let process = e.get(Ptr::<Actor>::new(actor), Actor::pCurrentProcess);
    if process == 0 {
        return Some(0);
    }
    let held = e.mem.u32(process + 0x114);
    if e.mem.u8(process + 0x28) <= 1 && held != 0 {
        let extra = extra_of(e, actor, EXTRA_COMBAT_STYLE);
        if extra == 0 {
            return Some(0);
        }
        let style = e.mem.u32(extra + 0xc);
        if style == 0 {
            return Some(0);
        }
        let restrictions = e.mem.u8(style + 0x40);
        if restrictions == 0 {
            return Some(0);
        }
        let melee = e.mem.u8(e.mem.u32(held + 8) + 0xf4) <= 2;
        if (restrictions & 1 != 0) == melee {
            return Some(0);
        }
    }
    let mark = e.mark_caller(Hook::LockEquipment, JIP_PREFERED_WEAPON_CALL);
    let entry = e.vcall(actor, GET_PREFERED_WEAPON, &args![6u32]).u32();
    drop(mark);
    if entry == 0 {
        return Some(0);
    }
    let weapon = e.mem.u32(entry + 8);
    let mut count = 1u32;
    if e.mem.u8(weapon + 0xf4) > 9 && e.mem.u32(entry + 4) > 1 {
        count = e.mem.u32(entry + 4);
    }
    let lists = e.mem.u32(entry);
    let extra = if lists != 0 { e.mem.u32(lists) } else { 0 };
    e.call(
        EQUIP_ITEM,
        &args![actor, weapon, count, extra, 1u32, 0u32, 1u32],
    );
    e.call(DELETE_ENTRY, &args![entry, 1u32]);
    Some(0)
}

/// `ReApplyEnchantments`: for each worn armour of the actor's inventory changes with an
/// enchantment, the enchantment applied to the actor's magic target again.
fn re_apply_enchantments(e: &mut Engine, actor: u32) {
    let mut node = crate::overrides::changes_list(e, actor);
    while node != 0 {
        let entry = e.mem.u32(node);
        node = e.mem.u32(node + 4);
        if entry == 0 {
            continue;
        }
        let form = e.mem.u32(entry + 8);
        if form_type(e, form) != ARMOR {
            continue;
        }
        let enchantment = e.mem.u32(form + 0x4c);
        if enchantment == 0 || equipped_extra(e, entry) == 0 {
            continue;
        }
        e.vcall(
            actor + 0x88,
            MAGIC_TARGET_APPLY,
            &args![enchantment + 0x18, form, 0u32],
        );
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), nvse/GameExtraData.cpp:
// ContChangesEntry::GetEquippedExtra.
/// The first of an entry's extra lists (its list at `+0`) with the worn bit (`+0xA` bit 6), or 0.
fn equipped_extra(e: &Engine, entry: u32) -> u32 {
    let mut node = e.mem.u32(entry);
    while node != 0 {
        let extra = e.mem.u32(node);
        node = e.mem.u32(node + 4);
        if extra != 0 && e.mem.u8(extra + 0xa) & 0x40 != 0 {
            return extra;
        }
    }
    0
}

// ---- the commands ----

/// The calling reference when it is an actor.
fn actor_this(e: &Engine, a: &CommandArgs) -> Option<u32> {
    crate::commands::is_actor(e, a.this_obj).then_some(a.this_obj)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_actor.h:
// Cmd_GetCombatDisabled_Execute.
fn get_combat_disabled(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    if let Some(actor) = actor_this(e, &a) {
        if flags1(e, actor) & COMBAT_AI_MODIFIED != 0 {
            a.set_result(e, 1.0);
        }
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_actor.h:
// Cmd_SetCombatDisabled_Execute; nvse/GameObjects.cpp: Actor::StopCombat.
/// Off (0 or less): what was set goes, with its user. On: no target disables combat (and stops
/// the actor's), a target forces it; switching between the two moves the user from one hook to
/// the other.
fn set_combat_disabled(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let Some(actor) = actor_this(e, &a) else {
        return handled(true);
    };
    let read = Read::of(e, &a);
    if !read.ok {
        return handled(true);
    }
    let disable = read.int(0, 0) as i32;
    let target = read.form(1);
    let flags = flags1(e, actor);
    if disable <= 0 {
        if flags & COMBAT_AI_MODIFIED == 0 {
            return handled(true);
        }
        if flags & COMBAT_DISABLED != 0 {
            e.add_hook_user(Hook::StartCombat, false);
        } else {
            state(e).force_combat_target.remove(&actor);
            e.add_hook_user(Hook::SetCombatTarget, false);
        }
        set_flags1(e, actor, flags & !COMBAT_AI_MODIFIED);
        return handled(true);
    }
    let flag = if target != 0 {
        FORCE_COMBAT_TARGET
    } else {
        COMBAT_DISABLED
    };
    if flags & COMBAT_AI_MODIFIED == 0 {
        set_flags1(e, actor, flags | flag);
        let hook = if target != 0 {
            Hook::SetCombatTarget
        } else {
            Hook::StartCombat
        };
        e.add_hook_user(hook, true);
    } else if flags & flag == 0 {
        if target == 0 {
            state(e).force_combat_target.remove(&actor);
        }
        e.add_hook_user(Hook::StartCombat, target == 0);
        e.add_hook_user(Hook::SetCombatTarget, target != 0);
        set_flags1(e, actor, flags ^ COMBAT_AI_MODIFIED);
    }
    if target != 0 {
        state(e).force_combat_target.insert(actor, target);
    } else if e.mem.u8(actor + 0x104) != 0 {
        // `Actor::StopCombat`: the combat controller told to stop (`+0x125`).
        let controller = e.vcall(actor, GET_COMBAT_CONTROLLER, &[]).u32();
        if controller != 0 {
            e.mem.set_u8(controller + 0x125, 1);
        }
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_actor.h:
// Cmd_ToggleNoHealthReset_Execute.
/// The ratio is written as the reader stores it (also when the read then fails); the hook is set
/// on or off (`HOOK_SET`) when the read succeeds.
fn toggle_no_health_reset(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if read.args.len() > 1 {
        state(e).health_restore_ratio = read.float(1, 0.0);
    }
    if read.ok {
        e.set_hook(Hook::NoHealthReset, read.int(0, 0) != 0);
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_actor.h:
// Cmd_GetTeammateUsingAmmo_Execute.
fn get_teammate_using_ammo(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    if let Some(actor) = actor_this(e, &a) {
        if flags1(e, actor) & INFINITE_AMMO == 0 {
            a.set_result(e, 1.0);
        }
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_actor.h:
// Cmd_SetTeammateUsingAmmo_Execute.
/// Infinite ammo on (`useAmmo` 0) or off when it changes: one user of the ammo hook more or less.
fn set_teammate_using_ammo(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let Some(actor) = actor_this(e, &a) else {
        return handled(true);
    };
    let read = Read::of(e, &a);
    if !read.ok {
        return handled(true);
    }
    let use_ammo = read.int(0, 0) != 0;
    let flags = flags1(e, actor);
    if !use_ammo == (flags & INFINITE_AMMO == 0) {
        set_flags1(e, actor, flags ^ INFINITE_AMMO);
        e.add_hook_user(Hook::RemoveAmmo, !use_ammo);
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_ccc.h:
// Cmd_CCCSetFollowState_Execute.
/// Below 0: the follow or wait order goes, with its user. Else 1 follow, 0 wait: a first order
/// is one user; switching orders keeps it.
fn ccc_set_follow_state(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let Some(actor) = actor_this(e, &a) else {
        return handled(true);
    };
    if !read.ok {
        return handled(true);
    }
    let order = read.int(0, u32::MAX) as i32;
    let flags = flags1(e, actor);
    if order < 0 {
        if flags & PC_TELEPORT_AI != 0 {
            set_flags1(e, actor, flags & !PC_TELEPORT_AI);
            e.add_hook_user(Hook::TeleportWithPC, false);
        }
        return handled(true);
    }
    let flag = if order != 0 {
        PC_TELEPORT_FOLLOW
    } else {
        PC_TELEPORT_WAIT
    };
    if flags & flag != 0 {
        return handled(true);
    }
    if flags & PC_TELEPORT_AI != 0 {
        set_flags1(e, actor, flags ^ PC_TELEPORT_AI);
    } else {
        set_flags1(e, actor, flags | flag);
        e.add_hook_user(Hook::TeleportWithPC, true);
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_ccc.h:
// Cmd_LockEquipment_Execute.
/// A character's equipment locked or unlocked when it changes: one user of the hook more or less.
fn lock_equipment(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let actor = a.this_obj;
    if !read.ok || actor == 0 || !crate::commands::is_type(e, actor, VTBL_CHARACTER) {
        return handled(true);
    }
    let lock = read.int(0, 0) != 0;
    let flags = flags1(e, actor);
    if lock != (flags & LOCKED_EQUIPMENT != 0) {
        set_flags1(e, actor, flags ^ LOCKED_EQUIPMENT);
        e.add_hook_user(Hook::LockEquipment, lock);
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_actor.h:
// Cmd_ForceActorDetectionValue_Execute.
/// A value other than 0 is forced (packed with the operation in bit 16: added to the formula's
/// result instead of replacing it); the actor's first is one user. 0 removes it.
fn force_actor_detection_value(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let Some(actor) = actor_this(e, &a) else {
        return handled(true);
    };
    let read = Read::of(e, &a);
    if !read.ok {
        return handled(true);
    }
    let value = read.int(0, 0);
    let operation = read.int(1, 0);
    if value != 0 {
        if !state(e).force_detection_value.contains_key(&actor) {
            set_flags2(e, actor, flags2(e, actor) | FORCE_DETECTION_VALUE);
            e.add_hook_user(Hook::GetDetectionValue, true);
        }
        // `Coordinate packedVal(oprType != 0, detectionValue)`: x in the high half, y low.
        let packed = (u32::from(operation != 0) << 16) | (value & 0xffff);
        state(e).force_detection_value.insert(actor, packed);
    } else if state(e).force_detection_value.remove(&actor).is_some() {
        set_flags2(e, actor, flags2(e, actor) & !FORCE_DETECTION_VALUE);
        e.add_hook_user(Hook::GetDetectionValue, false);
    }
    handled(true)
}

/// `IS_ID(form, Script)`.
fn is_script(e: &Engine, form: u32) -> bool {
    form != 0 && form_type(e, form) == SCRIPT
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_script.h:
// Cmd_GetScriptDisabled_Execute.
fn get_script_disabled(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let script = read.form(0);
    if read.ok && is_script(e, script) && e.mem.u32(script + 0x20) == 0 {
        a.set_result(e, 1.0);
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_script.h:
// Cmd_SetScriptDisabled_Execute, SetScriptDisabled.
/// Each script of the form or list disabled (its data length 0) or enabled again (its length
/// counted from its data) when that changes; a script that is not a quest's is one user of the
/// event-mask hook while disabled.
fn set_script_disabled(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if !read.ok {
        return handled(true);
    }
    let disable = read.int(1, 0) != 0;
    for script in crate::actor_hooks::form_or_list(e, read.form(0)) {
        if !is_script(e, script) || disable == (e.mem.u32(script + 0x20) == 0) {
            continue;
        }
        let length = if disable { 0 } else { data_length(e, script) };
        e.mem.set_u32(script + 0x20, length);
        if e.mem.u8(script + 0x28) == 0 {
            e.add_hook_user(Hook::MergeEventMask, disable);
        }
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), nvse/GameScript.cpp: Script::GetDataLength.
/// A compiled script's length: `ScriptName` (0x1D), then each `Begin` block (0x10, its length
/// words at +2 and +6) and its `End` (0x11); 0 for data that does not follow that shape.
fn data_length(e: &Engine, script: u32) -> u32 {
    let data = e.mem.u32(script + 0x30);
    if e.mem.u32(data) != 0x1d {
        return 0;
    }
    let mut at = data + 4;
    while e.mem.u16(at) == 0x10 {
        at += u32::from(e.mem.u16(at + 2)) + u32::from(e.mem.u16(at + 6));
        if e.mem.u32(at) != 0x11 {
            return 0;
        }
        at += 4;
    }
    at - data
}

#[cfg(test)]
#[path = "ai_hooks_tests.rs"]
mod tests;
