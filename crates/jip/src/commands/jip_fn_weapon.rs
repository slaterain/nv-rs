//! `functions_jip/jip_fn_weapon.h` (JIP LN NVSE): a weapon form's sounds, mods, reload animation,
//! models, embedded weapon data, on-hit setting and semi-automatic delays; the equipped weapon's
//! silencer, scope and mods, a weapon's mod slots, shots a second and the attack queue. 25 of its
//! 26 commands. The weapon is read as an inventory object (`kParamType_ObjectID`) or any form;
//! each command checks `IS_ID(weapon, TESObjectWEAP)` (type 0x28).
//!
//! Left for later: `GetCalculatedWeaponDamage` reads `s_patchInstallState.NPCPerks` (whether
//! JIP's NPC perks patch is on, `internal/patches_game.h`: the core track's step 3.9) inside its
//! damage chain (`ContChangesEntry::CalculateWeaponDamage`: the game's `0x644CE0`, the ammo
//! effects and the perk entry point), so it comes with the patches.
//!
//! The weapon loader (`0051F630`) reads `DNAM` (0xCC bytes) at +0xF4 (the embedded weapon actor
//! value byte +0x11F, on-hit +0x128, the semi-automatic delays +0x174/+0x178 (defaulted there),
//! the mod effects +0x180, the mod values +0x18C, the mod reload animation byte +0x1A1 inside it),
//! loads the shell casing model at +0x1D0 and the seven mod models from +0x270 (0x20 each), and
//! stores the sound level at +0x364. The twelve sounds (+0x21C), the item mods (+0x350), the
//! embedded node name (+0x35C) and the pickup/putdown sounds (+0xEC/+0xF0) follow JIP's header.
//!
//! Casts: the script's `UInt32` into the reload animation and embedded actor value bytes (low
//! byte); the floats stored as they are.

use super::forms::{form_type_ln, TESObjectWEAP, TES_MODEL_TEXTURE_SWAP_SIZE};
use super::{
    assign_string, buffer_arg, model_path, set_form_result, set_model_path, string_set, type_of,
    Handlers,
};
use engine_core::prelude::*;
use nvse::commands::Read;
use nvse::{handled, CommandArgs, EvalArgs};

#[rustfmt::skip]
pub static COMMANDS: Handlers = &[
    (0x264a, get_weapon_detection_sound_level),
    (0x264b, set_weapon_detection_sound_level),
    (0x264e, get_weapon_sound),
    (0x264f, set_weapon_sound),
    (0x2650, set_weapon_item_mod),
    (0x2651, set_weapon_item_mod_effect),
    (0x2652, set_weapon_item_mod_value),
    (0x26bb, get_weapon_mod_reload_anim),
    (0x26bc, set_weapon_mod_reload_anim),
    (0x26f5, get_weapon_shell_casing_model),
    (0x26f6, set_weapon_shell_casing_model),
    (0x277e, set_embedded_weapon_node),
    (0x277f, set_embedded_weapon_av),
    (0x2899, get_weapon_on_hit_setting),
    (0x289a, set_weapon_on_hit_setting),
    (0x289b, get_weapon_semi_auto_fire_delay),
    (0x289c, set_weapon_semi_auto_fire_delay),
    (0x28a9, get_weapon_model),
    (0x28aa, set_weapon_model),
    (0x2924, weapon_has_mod_type),
    (0x2940, calculate_shots_per_sec),
];

#[rustfmt::skip]
pub static CONDITIONS: super::Conditions = &[
    (0x264c, is_equipped_weapon_silenced, is_equipped_weapon_silenced_eval),
    (0x264d, is_equipped_weapon_scoped, is_equipped_weapon_scoped_eval),
    (0x28f7, equipped_weapon_has_mod_type, equipped_weapon_has_mod_type_eval),
    (0x2941, is_attack_queued, is_attack_queued_eval),
];

/// JIP's path buffers (`char path[0x80]`) and the node name's (`char nodeName[0x40]`).
const PATH_BUFFER: usize = 0x80;
const NODE_BUFFER: usize = 0x40;

fn weapon_arg(e: &Engine, read: &Read) -> Option<Ptr<TESObjectWEAP>> {
    let form = read.form(0);
    (read.ok && type_of(e, form) == form_type_ln::TES_OBJECT_WEAP).then(|| Ptr::new(form))
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_weapon.h:
// Cmd_GetWeaponDetectionSoundLevel_Execute.
fn get_weapon_detection_sound_level(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if let Some(wp) = weapon_arg(e, &read) {
        let v = e.get(wp, TESObjectWEAP::soundLevel) as i32;
        a.set_result(e, f64::from(v));
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_weapon.h:
// Cmd_SetWeaponDetectionSoundLevel_Execute.
/// Levels 0-2 (loud, normal, silent).
fn set_weapon_detection_sound_level(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let level = read.int(1, 0);
    if let Some(wp) = weapon_arg(e, &read).filter(|_| level <= 2) {
        e.set(wp, TESObjectWEAP::soundLevel, level);
    }
    handled(true)
}

/// Sound `kind` 0-11 (`sounds[12]`), 12 the pickup sound, 13 the putdown sound.
fn sound_field(
    wp: Ptr<TESObjectWEAP>,
    kind: u32,
) -> (Ptr<TESObjectWEAP>, Field<TESObjectWEAP, u32>) {
    match kind {
        0..=11 => (wp.index(kind, 4), TESObjectWEAP::sounds),
        12 => (wp, TESObjectWEAP::pickupSound),
        _ => (wp, TESObjectWEAP::putdownSound),
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_weapon.h:
// Cmd_GetWeaponSound_Execute.
fn get_weapon_sound(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let kind = read.int(1, 0);
    if let Some(wp) = weapon_arg(e, &read).filter(|_| kind <= 13) {
        let (p, field) = sound_field(wp, kind);
        let sound = e.get(p, field);
        if sound != 0 {
            set_form_result(e, &a, sound);
        }
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_weapon.h:
// Cmd_SetWeaponSound_Execute.
/// The sound set, or cleared without one (the reader checks it is a sound).
fn set_weapon_sound(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let kind = read.int(1, 0);
    if let Some(wp) = weapon_arg(e, &read).filter(|_| kind <= 13) {
        let (p, field) = sound_field(wp, kind);
        e.set(p, field, read.form(2));
    }
    handled(true)
}

/// Mod slot 1-3 as an index 0-2, with the weapon.
fn mod_slot(e: &Engine, read: &Read) -> Option<(Ptr<TESObjectWEAP>, u32)> {
    let slot = read.int(1, 0);
    let wp = weapon_arg(e, read)?;
    if slot != 0 && slot <= 3 {
        Some((wp, slot - 1))
    } else {
        None
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_weapon.h:
// Cmd_SetWeaponItemMod_Execute.
/// The slot's weapon mod set; a missing form or another type clears it.
fn set_weapon_item_mod(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if let Some((wp, i)) = mod_slot(e, &read) {
        let m = read.form(2);
        let value = if m != 0 && type_of(e, m) == form_type_ln::TES_OBJECT_IMOD {
            m
        } else {
            0
        };
        e.set(wp.index(i, 4), TESObjectWEAP::itemMod, value);
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_weapon.h:
// Cmd_SetWeaponItemModEffect_Execute.
/// Effects 1-14 but 5.
fn set_weapon_item_mod_effect(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let kind = read.int(2, 0);
    if let Some((wp, i)) = mod_slot(e, &read) {
        if kind != 0 && kind <= 14 && kind != 5 {
            e.set(wp.index(i, 4), TESObjectWEAP::effectMods, kind);
        }
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_weapon.h:
// Cmd_SetWeaponItemModValue_Execute.
fn set_weapon_item_mod_value(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if let Some((wp, i)) = mod_slot(e, &read) {
        e.set(wp.index(i, 4), TESObjectWEAP::value1Mod, read.float(2, 0.0));
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_weapon.h:
// Cmd_GetWeaponModReloadAnim_Execute.
fn get_weapon_mod_reload_anim(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if let Some(wp) = weapon_arg(e, &read) {
        let v = e.get(wp, TESObjectWEAP::modReloadAnim);
        a.set_result(e, f64::from(v));
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_weapon.h:
// Cmd_SetWeaponModReloadAnim_Execute.
/// Animations 0-22.
fn set_weapon_mod_reload_anim(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let anim = read.int(1, 0);
    if let Some(wp) = weapon_arg(e, &read).filter(|_| anim <= 22) {
        e.set(wp, TESObjectWEAP::modReloadAnim, anim as u8);
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_weapon.h:
// Cmd_GetWeaponShellCasingModel_Execute.
fn get_weapon_shell_casing_model(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let text = match weapon_arg(e, &read) {
        Some(wp) => model_path(e, wp.at(TESObjectWEAP::shellCasingModel).addr()),
        None => None,
    };
    assign_string(e, &a, text);
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_weapon.h:
// Cmd_SetWeaponShellCasingModel_Execute.
fn set_weapon_shell_casing_model(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let path = buffer_arg(e, &read, 1, PATH_BUFFER, "SetWeaponShellCasingModel");
    if let Some(wp) = weapon_arg(e, &read) {
        set_model_path(e, wp.at(TESObjectWEAP::shellCasingModel).addr(), &path);
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_weapon.h:
// Cmd_SetEmbeddedWeaponNode_Execute.
fn set_embedded_weapon_node(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let name = buffer_arg(e, &read, 1, NODE_BUFFER, "SetEmbeddedWeaponNode");
    if let Some(wp) = weapon_arg(e, &read) {
        string_set(e, wp.at(TESObjectWEAP::embeddedNodeName).addr(), &name);
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_weapon.h:
// Cmd_SetEmbeddedWeaponAV_Execute.
fn set_embedded_weapon_av(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if let Some(wp) = weapon_arg(e, &read) {
        // `kParamType_ActorValue` is read as a 16-bit code into JIP's `UInt32`: from the
        // compiled data or, for a parenthesised argument, from the expression evaluator.
        let code = read.short(1, 0);
        e.set(wp, TESObjectWEAP::embedWeaponAV, code as u8);
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_weapon.h:
// Cmd_GetWeaponOnHitSetting_Execute.
/// The setting (`(int)`); -1 when the read failed or the form is no weapon.
fn get_weapon_on_hit_setting(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let v = match weapon_arg(e, &read) {
        Some(wp) => e.get(wp, TESObjectWEAP::onHit) as i32,
        None => -1,
    };
    a.set_result(e, f64::from(v));
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_weapon.h:
// Cmd_SetWeaponOnHitSetting_Execute.
/// Settings 0-3.
fn set_weapon_on_hit_setting(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let setting = read.int(1, 0);
    if let Some(wp) = weapon_arg(e, &read).filter(|_| setting <= 3) {
        e.set(wp, TESObjectWEAP::onHit, setting);
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_weapon.h:
// Cmd_GetWeaponSemiAutoFireDelay_Execute.
/// The minimum delay, or the maximum for a non-zero second argument.
fn get_weapon_semi_auto_fire_delay(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if let Some(wp) = weapon_arg(e, &read) {
        let i = u32::from(read.int(1, 0) != 0);
        let v = e.get(wp.index(i, 4), TESObjectWEAP::semiAutoFireDelay);
        a.set_result(e, f64::from(v));
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_weapon.h:
// Cmd_SetWeaponSemiAutoFireDelay_Execute.
fn set_weapon_semi_auto_fire_delay(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if let Some(wp) = weapon_arg(e, &read) {
        let i = u32::from(read.int(1, 0) != 0);
        e.set(
            wp.index(i, 4),
            TESObjectWEAP::semiAutoFireDelay,
            read.float(2, 0.0),
        );
    }
    handled(true)
}

/// The model of mod flags 0-7: 0 the weapon's own (`textureSwap`, +0x3C); 3 is `modModels[3]`
/// and 4 `modModels[2]` (JIP's swap); any other `modModels[flags - 1]`.
fn weapon_model(wp: Ptr<TESObjectWEAP>, flags: u32) -> u32 {
    let mods = |i: u32| {
        wp.at(TESObjectWEAP::modModels)
            .index(i, TES_MODEL_TEXTURE_SWAP_SIZE)
            .addr()
    };
    match flags {
        0 => wp.at(TESObjectWEAP::textureSwap).addr(),
        3 => mods(3),
        4 => mods(2),
        _ => mods(flags - 1),
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_weapon.h:
// Cmd_GetWeaponModel_Execute.
fn get_weapon_model(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let flags = read.int(1, 0);
    let text = match weapon_arg(e, &read).filter(|_| flags <= 7) {
        Some(wp) => model_path(e, weapon_model(wp, flags)),
        None => None,
    };
    assign_string(e, &a, text);
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_weapon.h:
// Cmd_SetWeaponModel_Execute.
fn set_weapon_model(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let flags = read.int(1, 0);
    let path = buffer_arg(e, &read, 2, PATH_BUFFER, "SetWeaponModel");
    if let Some(wp) = weapon_arg(e, &read).filter(|_| flags <= 7) {
        set_model_path(e, weapon_model(wp, flags), &path);
    }
    handled(true)
}

// ---- the equipped weapon, its mods, the attack queue (batch 12) ----

/// `Actor::baseProcess` (+0x68), its process level (byte +0x28), `MiddleHighProcess::weaponInfo`
/// (+0x114): JIP's `Actor::GetWeaponInfo` asm.
const PROCESS: u32 = 0x68;
const PROCESS_LEVEL: u32 = 0x28;
const WEAPON_INFO: u32 = 0x114;
/// A changes entry: its extra data lists (+0), its item (+8).
const ENTRY_LISTS: u32 = 0x00;
const ENTRY_ITEM: u32 = 0x08;
/// `TESObjectWEAP`: `eWeaponType` +0xF4, `animMult` +0xF8, `weaponFlags1` +0x100 (2 automatic, 4
/// has a scope), `attackAnim` +0x11D, `weaponFlags2` +0x12C (0x2000: the scope is a mod),
/// `animAttackMult` +0x130, `fireRate` +0x134 (all inside the loader's `DNAM` at +0xF4),
/// `effectMods[3]` +0x180 and `value1Mod[3]` +0x18C, `itemMod[3]` +0x350, `soundLevel` +0x364.
const WEAPON_TYPE: u32 = 0xf4;
const ANIM_MULT: u32 = 0xf8;
const FLAGS1: u32 = 0x100;
const ATTACK_ANIM: u32 = 0x11d;
const FLAGS2: u32 = 0x12c;
const ANIM_ATTACK_MULT: u32 = 0x130;
const FIRE_RATE: u32 = 0x134;
const EFFECT_MODS: u32 = 0x180;
const VALUE1_MODS: u32 = 0x18c;
const ITEM_MODS: u32 = 0x350;
const SOUND_LEVEL: u32 = 0x364;
/// `kXData_ExtraItemDropper` (its reference +0xC), `kXData_ExtraWeaponModFlags` (its byte +0xC).
const EXTRA_ITEM_DROPPER: u32 = 0x39;
const EXTRA_WEAPON_MOD_FLAGS: u32 = 0x8d;
/// The game's attack-queued byte (`0x11E07AC`).
const ATTACK_QUEUED: u32 = 0x011e_07ac;
/// `PickBestAnimation` (`00495740`, `thiscall`: the anim data, the group id, 0) and
/// `GetTESAnimGroup` (`00496500`, `thiscall`: the anim data, the group id); the weapon types'
/// group bases (`0x118A838`, a word each); `TESAnimGroup::numKeys` +0x14 and `keyTimes` +0x18.
const PICK_BEST_ANIMATION: u32 = 0x0049_5740;
const GET_ANIM_GROUP: u32 = 0x0049_6500;
const WEAPON_TYPE_GROUPS: u32 = 0x0118_a838;
const GROUP_NUM_KEYS: u32 = 0x14;
const GROUP_KEY_TIMES: u32 = 0x18;
/// `TESObjectREFR::GetAnimData` (vtable +0x1E4).
const GET_ANIM_DATA: u32 = 0x1e4;

// Translated from JIP LN NVSE 57.30 (5a30ac4), nvse/GameObjects.cpp: Actor::GetWeaponInfo (asm).
/// The actor's equipped weapon's changes entry: its process's when the process is high or
/// middle-high (level 0 or 1); else none.
fn weapon_info(e: &Engine, actor: u32) -> u32 {
    let process = e.mem.u32(actor + PROCESS);
    if process == 0 || e.mem.u8(process + PROCESS_LEVEL) > 1 {
        return 0;
    }
    e.mem.u32(process + WEAPON_INFO)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_game.h: GetEntryDataModFlagsHook
// (asm).
/// A weapon entry's mod flags: its first extra data list's `ExtraWeaponModFlags` byte, or, when
/// that list has an `ExtraItemDropper` with a reference, the dropper's own list's. 0 for no
/// entry, not a weapon, no list or no flags. (JIP's always-on replacement of the game's
/// `0x4BD820`, which it calls directly here.)
pub fn entry_mod_flags(e: &mut Engine, entry: u32) -> u8 {
    if entry == 0 {
        return 0;
    }
    let item = e.mem.u32(entry + ENTRY_ITEM);
    if type_of(e, item) != form_type_ln::TES_OBJECT_WEAP {
        return 0;
    }
    let lists = e.mem.u32(entry + ENTRY_LISTS);
    if lists == 0 {
        return 0;
    }
    let mut list = e.mem.u32(lists);
    if list == 0 {
        return 0;
    }
    let dropper = super::extra::get_by_type(e, list, EXTRA_ITEM_DROPPER);
    if dropper != 0 {
        let refr = e.mem.u32(dropper + 0xc);
        if refr != 0 {
            list = refr + super::extra::REFR_EXTRA_LIST;
        }
    }
    let flags = super::extra::get_by_type(e, list, EXTRA_WEAPON_MOD_FLAGS);
    if flags == 0 {
        return 0;
    }
    e.mem.u8(flags + 0xc)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_game.h: GetEntryDataHasModHook
// (asm).
/// Whether a mod slot the entry has (flag bits 1, 2, 4, in that order) carries the effect
/// `mod_type` (the low byte of the weapon's `effectMods` word against the byte).
pub fn entry_has_mod(e: &mut Engine, entry: u32, mod_type: u8) -> bool {
    let flags = entry_mod_flags(e, entry);
    if flags == 0 {
        return false;
    }
    let weapon = e.mem.u32(entry + ENTRY_ITEM);
    (0..3).any(|i| flags & (1 << i) != 0 && e.mem.u8(weapon + EFFECT_MODS + 4 * i) == mod_type)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), nvse/GameExtraData.cpp:
// ContChangesEntry::GetWeaponModEffectValue (asm).
/// The value of the first mod slot the entry's first list's flags hold whose effect (the whole
/// word) is `effect`, from the weapon's `value1Mod`; 0 for none, no list or no flags.
fn weapon_mod_effect_value(e: &mut Engine, entry: u32, effect: u32) -> f32 {
    let lists = e.mem.u32(entry + ENTRY_LISTS);
    if lists == 0 {
        return 0.0;
    }
    let weapon = e.mem.u32(entry + ENTRY_ITEM);
    let list = e.mem.u32(lists);
    if list == 0 {
        return 0.0;
    }
    let x = super::extra::get_by_type(e, list, EXTRA_WEAPON_MOD_FLAGS);
    if x == 0 {
        return 0.0;
    }
    let flags = e.mem.u8(x + 0xc);
    for i in 0..3 {
        if flags & (1 << i) != 0 && e.mem.u32(weapon + EFFECT_MODS + 4 * i) == effect {
            return e.mem.f32(weapon + VALUE1_MODS + 4 * i);
        }
    }
    0.0
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), nvse/GameObjects.cpp: Actor::EquippedWeaponSilenced.
/// 1 for an equipped weapon of sound level 2 (silent), 2 for one with a silencer mod (effect 11),
/// else 0.
fn equipped_weapon_silenced(e: &mut Engine, actor: u32) -> u8 {
    let info = weapon_info(e, actor);
    if info == 0 {
        return 0;
    }
    let weapon = e.mem.u32(info + ENTRY_ITEM);
    if e.mem.u32(weapon + SOUND_LEVEL) == 2 {
        1
    } else if entry_has_mod(e, info, 11) {
        2
    } else {
        0
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), nvse/GameObjects.cpp: Actor::EquippedWeaponHasScope.
/// For an equipped weapon with a scope (flags1 bit 4): 1 when it is not a mod's (flags2 bit
/// 0x2000 clear), 2 when the scope mod (effect 14) is on; else 0.
fn equipped_weapon_has_scope(e: &mut Engine, actor: u32) -> u8 {
    let info = weapon_info(e, actor);
    if info == 0 {
        return 0;
    }
    let weapon = e.mem.u32(info + ENTRY_ITEM);
    if e.mem.u8(weapon + FLAGS1) & 4 == 0 {
        return 0;
    }
    if e.mem.u32(weapon + FLAGS2) & 0x2000 == 0 {
        1
    } else if entry_has_mod(e, info, 14) {
        2
    } else {
        0
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_weapon.h:
// Cmd_IsEquippedWeaponSilenced_Execute/_Eval.
fn is_equipped_weapon_silenced(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    if super::is_actor(e, a.this_obj) {
        let v = equipped_weapon_silenced(e, a.this_obj);
        a.set_result(e, f64::from(v));
    }
    handled(true)
}

fn is_equipped_weapon_silenced_eval(e: &mut Engine, w: &[u32]) -> Ret {
    let a = EvalArgs::new(w);
    if super::is_actor(e, a.this_obj) {
        let v = equipped_weapon_silenced(e, a.this_obj);
        e.mem.set_f64(a.result, f64::from(v));
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_weapon.h:
// Cmd_IsEquippedWeaponScoped_Execute/_Eval.
fn is_equipped_weapon_scoped(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    if super::is_actor(e, a.this_obj) {
        let v = equipped_weapon_has_scope(e, a.this_obj);
        a.set_result(e, f64::from(v));
    }
    handled(true)
}

fn is_equipped_weapon_scoped_eval(e: &mut Engine, w: &[u32]) -> Ret {
    let a = EvalArgs::new(w);
    if super::is_actor(e, a.this_obj) {
        let v = equipped_weapon_has_scope(e, a.this_obj);
        e.mem.set_f64(a.result, f64::from(v));
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_weapon.h:
// Cmd_EquippedWeaponHasModType_Execute/_Eval; nvse/GameObjects.cpp: Actor::EquippedWeaponHasMod.
/// 1 when the actor's equipped weapon has a mod of the effect type (the argument's low byte, as
/// the hook's `UInt8` takes it).
fn equipped_weapon_has_mod_type(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    if !super::is_actor(e, a.this_obj) {
        return handled(true);
    }
    let read = Read::of(e, &a);
    if read.ok {
        let info = weapon_info(e, a.this_obj);
        if info != 0 && entry_has_mod(e, info, read.int(0, 0) as u8) {
            a.set_result(e, 1.0);
        }
    }
    handled(true)
}

fn equipped_weapon_has_mod_type_eval(e: &mut Engine, w: &[u32]) -> Ret {
    let a = EvalArgs::new(w);
    if super::is_actor(e, a.this_obj) {
        let info = weapon_info(e, a.this_obj);
        if info != 0 && entry_has_mod(e, info, a.arg1 as u8) {
            e.mem.set_f64(a.result, 1.0);
        }
    }
    handled(true)
}

/// `IS_REFERENCE(form)` (JIP LN NVSE): its vtable answers true (`008D0360`) at +0xF0.
fn is_reference(e: &Engine, form: u32) -> bool {
    e.mem.u32(e.mem.u32(form) + 0xf0) == 0x008d_0360
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_weapon.h:
// Cmd_WeaponHasModType_Execute.
/// The slot (1 to 3) of the weapon's (or a reference's base weapon's) first item mod whose effect
/// is the type (the whole word); 0 for none. The result is written on every path, a failed read
/// included.
fn weapon_has_mod_type(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let mut slot = 0u32;
    let mut form = read.form(0);
    if read.ok && form != 0 {
        if is_reference(e, form) {
            form = e.mem.u32(form + 0x20);
        }
        if form != 0 && type_of(e, form) == form_type_ln::TES_OBJECT_WEAP {
            let kind = read.int(1, 0);
            for i in 0..3 {
                if e.mem.u32(form + ITEM_MODS + 4 * i) == 0
                    || e.mem.u32(form + EFFECT_MODS + 4 * i) != kind
                {
                    continue;
                }
                slot = i + 1;
                break;
            }
        }
    }
    a.set_result(e, f64::from(slot));
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_weapon.h:
// Cmd_CalculateShotsPerSec_Execute.
/// The actor's shots a second with the weapon (or an inventory reference's weapon, its stack's
/// attack speed mod (effect 8) added to the attack multiplier): for a non-automatic weapon with
/// anim data, `attack mult * anim mult / the attack group's next-to-last key time` when the
/// best attack animation is the weapon's (0xFF: 0x20) and its group has two keys or more; else
/// (automatic, or no anim data) `attack mult * attack mult * fire rate * anim mult`. Floats.
fn calculate_shots_per_sec(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    if !super::is_actor(e, a.this_obj) {
        return handled(true);
    }
    let read = Read::of(e, &a);
    let mut form = read.form(0);
    if !read.ok || form == 0 {
        return handled(true);
    }
    let mut info = 0;
    if is_reference(e, form) {
        let id = super::ref_id(e, form);
        if nvse::invref::for_ref_id(e, id) {
            if let Some(r) = nvse::invref::get(e, id) {
                info = r.data.entry;
                form = r.data.item;
            }
        }
    }
    if form == 0 || type_of(e, form) != form_type_ln::TES_OBJECT_WEAP {
        return handled(true);
    }
    let weapon = form;
    let anim_data = e.vcall(a.this_obj, GET_ANIM_DATA, &[]).u32();
    let mut mult = e.mem.f32(weapon + ANIM_ATTACK_MULT);
    if info != 0 {
        mult += weapon_mod_effect_value(e, info, 8);
    }
    let anim_mult = e.mem.f32(weapon + ANIM_MULT);
    if anim_data != 0 && e.mem.u8(weapon + FLAGS1) & 2 == 0 {
        let mut attack = u32::from(e.mem.u8(weapon + ATTACK_ANIM));
        if attack == 0xff {
            attack = 0x20;
        }
        let kind = u32::from(e.mem.u8(weapon + WEAPON_TYPE));
        let base = e.mem.u32(WEAPON_TYPE_GROUPS + 4 * kind);
        let group_id = (base << 8) | attack;
        let anim = e
            .call(PICK_BEST_ANIMATION, &args![anim_data, group_id, 0u32])
            .u32()
            & 0xffff;
        if anim & 0xff == attack {
            let group = e.call(GET_ANIM_GROUP, &args![anim_data, anim]).u32();
            if group != 0 {
                let keys = e.mem.u32(group + GROUP_NUM_KEYS);
                if keys >= 2 {
                    let times = e.mem.u32(group + GROUP_KEY_TIMES);
                    let t = e.mem.f32(times + 4 * (keys - 2));
                    a.set_result(e, f64::from((mult * anim_mult) / t));
                }
            }
        }
    } else {
        let rate = e.mem.f32(weapon + FIRE_RATE);
        a.set_result(e, f64::from(mult * mult * rate * anim_mult));
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_weapon.h:
// Cmd_IsAttackQueued_Execute/_Eval.
/// 1 while the game's attack-queued byte (`0x11E07AC`) is set.
fn is_attack_queued(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    if e.mem.u8(ATTACK_QUEUED) != 0 {
        a.set_result(e, 1.0);
    }
    handled(true)
}

fn is_attack_queued_eval(e: &mut Engine, w: &[u32]) -> Ret {
    let a = EvalArgs::new(w);
    if e.mem.u8(ATTACK_QUEUED) != 0 {
        e.mem.set_f64(a.result, 1.0);
    }
    handled(true)
}

#[cfg(test)]
#[path = "jip_fn_weapon_tests.rs"]
mod tests;
