//! `functions_jip/jip_fn_weapon.h` (JIP LN NVSE): a weapon form's sounds, mods, reload animation,
//! models, embedded weapon data, on-hit setting and semi-automatic delays. 19 of its 26 commands.
//! The weapon is read as an inventory object (`kParamType_ObjectID`) or any form; each command
//! checks `IS_ID(weapon, TESObjectWEAP)` (type 0x28).
//!
//! Not built (they read the actor's process or equipped data, or are conditions): IsEquippedWeaponSilenced,
//! IsEquippedWeaponScoped, EquippedWeaponHasModType, WeaponHasModType, GetCalculatedWeaponDamage,
//! CalculateShotsPerSec, IsAttackQueued.
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
use nvse::{handled, CommandArgs};

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
    (slot != 0 && slot <= 3).then_some((wp, slot - 1))
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
        // `kParamType_ActorValue` is read as a 16-bit code into JIP's `UInt32`.
        let code = match read.args.get(1) {
            Some(nvse::args::Arg::Short(v)) => *v,
            _ => 0,
        };
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

#[cfg(test)]
#[path = "jip_fn_weapon_tests.rs"]
mod tests;
