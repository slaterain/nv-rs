//! xNVSE's per-mod player controls (`Commands_Game.cpp` and `Hooks_Gameplay.cpp`'s
//! `TogglePlayerControlsAlt`, xNVSE): `DisablePlayerControlsAlt`, `EnablePlayerControlsAlt`,
//! `GetPlayerControlsDisabledAlt` and their `Ex` forms. Each mod keeps its own set of disabled
//! controls; their union is xNVSE's `g_disabledControls`, kept here in the engine's extender
//! state ([`Engine::controls_alt`]), where the translated functions xNVSE patches read it with
//! one load: `PlayerCharacter::Update` (V.A.T.S., jumping, aiming, running, waiting), the attack
//! handling `00948310` (attacking), `005a03f0` (the game's own seven flags), `ShowSleepWaitMenu`,
//! the furniture's `Activate` (sleeping) and the map menu's `DoClick` (fast travel). The 14th
//! place, `PlayerCharacter::Reload` (xNVSE patches its vtable slot `0108ae28`), is a wrap of the
//! function the slot holds, `0095d3f0` ([`RELOAD_ENTRY`]): the exe calls it only through the slot.
//!
//! Read from xNVSE's source (xNVSE 6.4.9, `0ccd23a`). The per-mod map is keyed as xNVSE's
//! `UnorderedMap<const char*>` is, by the mod name's case-insensitive hash; the name is
//! `GetModName(scriptObj, true)` ([`crate::errors::mod_name`]: the script's first file, which every
//! script loaded from a file has).

use super::Read;
use crate::table::{Command, Param, Returns};
use crate::{handled, CommandArgs, Nvse};
use engine_core::extend::controls_alt::*;
use engine_core::extend::{Owner, WrapSlot};
use engine_core::prelude::*;
use std::collections::HashMap;

const fn p(name: &'static str, kind: u32, optional: bool) -> Param {
    Param {
        name,
        kind,
        optional,
    }
}
/// `kParamType_Integer`; `kParams_SevenOptionalInts`, `kParams_EightOptionalInts`,
/// `kParams_OneOptionalInt`, `kParams_TwoOptionalInts`.
const INT: u32 = 0x01;
const SEVEN_INTS: &[Param] = &[
    p("int", INT, true),
    p("int", INT, true),
    p("int", INT, true),
    p("int", INT, true),
    p("int", INT, true),
    p("int", INT, true),
    p("int", INT, true),
];
const EIGHT_INTS: &[Param] = &[
    p("int", INT, true),
    p("int", INT, true),
    p("int", INT, true),
    p("int", INT, true),
    p("int", INT, true),
    p("int", INT, true),
    p("int", INT, true),
    p("int", INT, true),
];
const ONE_INT: &[Param] = &[p("int", INT, true)];
const TWO_INTS: &[Param] = &[p("int", INT, true), p("int", INT, true)];

const fn cmd(
    opcode: u32,
    name: &'static str,
    short: &'static str,
    help: &'static str,
    params: &'static [Param],
    execute: AbiFn,
) -> Command {
    Command {
        opcode,
        name,
        short,
        help,
        needs_ref: false,
        params,
        execute,
        eval: None,
        returns: Returns::Default,
    }
}

const HELP: &str = "Per-mod version with added args";

#[rustfmt::skip]
pub static COMMANDS: &[Command] = &[
    cmd(0x1661, "DisablePlayerControlsAlt", "DPCAlt", HELP, SEVEN_INTS, disable_alt),
    cmd(0x1662, "EnablePlayerControlsAlt", "EPCAlt", HELP, SEVEN_INTS, enable_alt),
    cmd(0x1663, "GetPlayerControlsDisabledAlt", "GPCDAlt", "", EIGHT_INTS, get_disabled_alt),
    cmd(0x1664, "DisablePlayerControlsAltEx", "DPCEx", HELP, ONE_INT, disable_alt_ex),
    cmd(0x1665, "EnablePlayerControlsAltEx", "EPCEx", HELP, ONE_INT, enable_alt_ex),
    cmd(0x1666, "GetPlayerControlsDisabledAltEx", "GPCDEx", "", TWO_INTS, get_disabled_alt_ex),
];

/// `kVanillaDefaultDisableFlags`: what `DisablePlayerControls` with no arguments disables.
const VANILLA_DEFAULT: u32 = MOVEMENT | PIPBOY | POV | FIGHTING;

/// The game's player (`g_thePlayer`), its disabled-controls byte (`+0x680`), `bThirdPerson`
/// (`+0x64C`), `playerNode` (`+0x694`), `actorMover` (`+0x190`, `Actor`), the third-person zoom
/// height (`011e0768`).
const PLAYER: u32 = 0x011d_ea3c;
const PLAYER_DISABLED: u32 = 0x680;
const PLAYER_THIRD_PERSON: u32 = 0x64c;
const PLAYER_NODE: u32 = 0x694;
const ACTOR_MOVER: u32 = 0x190;
const THIRD_PERSON_ZOOM: u32 = 0x011e_0768;
/// `HUDMainMenu::UpdateVisibilityState` (`cdecl`) and its states; `Actor::SetWantsWeaponOut`,
/// `PlayerCharacter::UpdateCamera`, the mover's flags setter `009ea3e0`, `Actor::AimWeapon`,
/// `Actor::SetBlocking` (`thiscall`); the mover's `GetMovementFlags` (virtual `+0x20`).
const UPDATE_HUD_STATE: u32 = 0x0077_1700;
const HUD_RECALCULATE: u32 = 0x1;
const HUD_PLAYER_DISABLED_CONTROLS: u32 = 0xc;
const SET_WANTS_WEAPON_OUT: u32 = 0x008a_6840;
const UPDATE_CAMERA: u32 = 0x0094_ae40;
const SET_MOVE_FLAGS: u32 = 0x009e_a3e0;
const AIM_WEAPON: u32 = 0x008b_b650;
const SET_BLOCKING: u32 = 0x0089_4cc0;
const MOVER_GET_FLAGS: u32 = 0x20;
/// The sneak move flag (`0x400`).
const MOVE_SNEAK: u32 = 0x400;

/// `PlayerCharacter::Reload` (`0095d3f0`, the function the player's vtable slot `0108ae28`
/// holds), wrapped for the reload flag; the V.A.T.S. camera's data (`011f2250`) and its mode
/// (`+4`, 4 the playback).
pub const RELOAD_FN: u32 = 0x0095_d3f0;
pub const RELOAD_ENTRY: WrapSlot = WrapSlot::new(Owner::Nvse, RELOAD_FN);
const VATS_CAMERA: u32 = 0x011f_2250;
const VATS_MODE: u32 = 4;
const VATS_PLAYBACK: u32 = 4;

/// The disabled controls of each mod (`g_disabledFlagsPerMod`), by the mod name's
/// case-insensitive hash.
#[derive(Default)]
pub struct State {
    per_mod: HashMap<u32, u32>,
}

fn state(e: &mut Engine) -> &mut State {
    &mut e.ext::<Nvse>().controls
}

/// The calling script's mod, as the map keys it.
fn mod_key(e: &Engine, script: u32) -> u32 {
    crate::events::hash_name(&crate::errors::mod_name(e, script))
}

/// Registers the reload wrap.
pub fn install(e: &mut Engine) {
    e.wrap(RELOAD_ENTRY, reload);
}

fn player(e: &Engine) -> u32 {
    e.mem.u32(PLAYER)
}

fn vanilla_flags(e: &Engine) -> u32 {
    u32::from(e.mem.u8(player(e) + PLAYER_DISABLED))
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Hooks_Gameplay.cpp:
// TogglePlayerControlsAlt::ApplyImmediateDisablingEffects.
/// What disabling a control does at once, for the controls not disabled already (by a mod or the
/// game): the HUD told, the weapon put away, first person, out of sneaking, out of aiming and
/// blocking.
fn apply_disabling(e: &mut Engine, changed: u32) {
    let pc = player(e);
    let real = changed & !(e.controls_alt() | vanilla_flags(e));
    if real == 0 {
        return;
    }
    let hud = if real & MOVEMENT != 0 {
        HUD_PLAYER_DISABLED_CONTROLS
    } else {
        HUD_RECALCULATE
    };
    e.call(UPDATE_HUD_STATE, &args![hud]);
    if real & FIGHTING != 0 {
        e.call(SET_WANTS_WEAPON_OUT, &args![pc, 0u32]);
    }
    if real & POV != 0 {
        e.mem.set_u8(pc + PLAYER_THIRD_PERSON, 0);
        if e.mem.u32(pc + PLAYER_NODE) != 0 {
            e.call(UPDATE_CAMERA, &args![pc, 0u32, 0u32]);
        } else {
            e.mem.set_f32(THIRD_PERSON_ZOOM, 0.0);
        }
    }
    if real & SNEAKING != 0 {
        let mover = e.mem.u32(pc + ACTOR_MOVER);
        let flags = e.vcall(mover, MOVER_GET_FLAGS, &args![]).u32();
        e.call(SET_MOVE_FLAGS, &args![mover, flags & !MOVE_SNEAK]);
    }
    if real & AIMING_OR_BLOCKING != 0 {
        e.call(AIM_WEAPON, &args![pc, 0u32, 0u32, 0u32]);
        e.call(SET_BLOCKING, &args![pc, 0u32]);
    }
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Hooks_Gameplay.cpp:
// TogglePlayerControlsAlt::ApplyImmediateEnablingEffects.
/// What enabling a control does at once: the HUD recalculated when movement comes back (and the
/// game itself does not keep it disabled).
fn apply_enabling(e: &mut Engine, changed: u32) {
    // Nothing of the mods' to take back (the game's byte is read only when there is).
    let ours = changed & e.controls_alt();
    if ours == 0 {
        return;
    }
    let real = ours & !vanilla_flags(e);
    if real & MOVEMENT != 0 {
        e.call(UPDATE_HUD_STATE, &args![HUD_RECALCULATE]);
    }
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Hooks_Gameplay.cpp:
// TogglePlayerControlsAlt::DisablePlayerControlsAlt.
/// The flags added to the mod's set (the immediate effects for those it did not have yet), and
/// to the union.
pub fn disable(e: &mut Engine, flags: u32, key: u32) {
    match state(e).per_mod.get(&key).copied() {
        Some(had) => {
            apply_disabling(e, flags & !had);
            state(e).per_mod.insert(key, had | flags);
        }
        None => {
            apply_disabling(e, flags);
            state(e).per_mod.insert(key, flags);
        }
    }
    let all = e.controls_alt() | flags;
    e.set_controls_alt(all);
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Hooks_Gameplay.cpp:
// TogglePlayerControlsAlt::EnablePlayerControlsAlt.
/// The flags taken from the mod's set (a set left empty goes); the union made again from every
/// mod's set. The immediate effects are told the mod's whole set as it was, as xNVSE tells them.
pub fn enable(e: &mut Engine, flags: u32, key: u32) {
    let Some(had) = state(e).per_mod.get(&key).copied() else {
        return;
    };
    apply_enabling(e, had);
    let left = had & !flags;
    if left == 0 {
        state(e).per_mod.remove(&key);
    } else {
        state(e).per_mod.insert(key, left);
    }
    let all = state(e).per_mod.values().fold(0, |a, f| a | f);
    e.set_controls_alt(all);
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Hooks_Gameplay.cpp:
// TogglePlayerControlsAlt::ResetOnLoad.
/// A game loaded or started: no mod's controls disabled any more, the enabling effects for all
/// (told after the word is cleared, as xNVSE does, so they find nothing to take back).
pub fn reset_on_load(e: &mut Engine) {
    state(e).per_mod.clear();
    e.set_controls_alt(0);
    apply_enabling(e, ALL);
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Hooks_Gameplay.cpp:
// TogglePlayerControlsAlt::GetPlayerControlsDisabledAlt.
/// Whether any of `flags` is disabled: by the calling mod (0), by any mod (1), by any mod or the
/// game's own byte (2), by the game only (3); false for another way.
fn disabled(e: &mut Engine, how: u32, flags: u32, key: u32) -> bool {
    match how {
        0 => state(e).per_mod.get(&key).is_some_and(|f| f & flags != 0),
        1 => e.controls_alt() & flags != 0,
        2 => (e.controls_alt() | vanilla_flags(e)) & flags != 0,
        3 => vanilla_flags(e) & flags != 0,
        _ => false,
    }
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Hooks_Gameplay.cpp:
// TogglePlayerControlsAlt::CondenseVanillaFlagArgs.
/// The seven vanilla arguments, in `DisablePlayerControls`' order, as the game's bits.
fn condense(args: [u32; 7]) -> u32 {
    let [movement, pipboy, fighting, pov, looking, rollover, sneaking] = args;
    movement
        | (looking << 1)
        | (pipboy << 2)
        | (fighting << 3)
        | (pov << 4)
        | (rollover << 5)
        | (sneaking << 6)
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Hooks_Gameplay.h:
// ExtractArgsForEnableOrDisablePlayerControls.
/// The seven arguments (movement, Pip-Boy, fighting, POV default 1; looking, rollover text,
/// sneaking default 0 to disable, 1 to enable) as flags; `None` for a failed read.
fn vanilla_args(e: &mut Engine, a: &CommandArgs, enable: bool) -> Option<u32> {
    let read = Read::of(e, a);
    if !read.ok {
        return None;
    }
    let late = u32::from(enable);
    let defaults = [1, 1, 1, 1, late, late, late];
    let mut values = [0u32; 7];
    for (i, v) in values.iter_mut().enumerate() {
        *v = read.int(i, defaults[i]);
    }
    Some(condense(values))
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_Game.cpp:
// Cmd_DisablePlayerControlsAlt_Execute.
fn disable_alt(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    if let Some(flags) = vanilla_args(e, &a, false).filter(|&f| f != 0) {
        let key = mod_key(e, a.script);
        disable(e, flags, key);
    }
    handled(true)
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_Game.cpp:
// Cmd_EnablePlayerControlsAlt_Execute.
fn enable_alt(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    if let Some(flags) = vanilla_args(e, &a, true).filter(|&f| f != 0) {
        let key = mod_key(e, a.script);
        enable(e, flags, key);
    }
    handled(true)
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_Game.cpp:
// Cmd_DisablePlayerControlsAltEx_Execute.
/// The flags as one bit field (default `kVanillaDefaultDisableFlags`).
fn disable_alt_ex(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if read.ok {
        let flags = read.int(0, VANILLA_DEFAULT);
        let key = mod_key(e, a.script);
        disable(e, flags, key);
    }
    handled(true)
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_Game.cpp:
// Cmd_EnablePlayerControlsAltEx_Execute.
/// The flags as one bit field (default all).
fn enable_alt_ex(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if read.ok {
        let flags = read.int(0, ALL);
        let key = mod_key(e, a.script);
        enable(e, flags, key);
    }
    handled(true)
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_Game.cpp:
// Cmd_GetPlayerControlsDisabledAlt_Execute.
/// How (the first argument, default 0), then the seven vanilla arguments (default 1 each).
fn get_disabled_alt(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    a.set_result(e, 0.0);
    let read = Read::of(e, &a);
    if read.ok {
        let how = read.int(0, 0);
        let mut values = [0u32; 7];
        for (i, v) in values.iter_mut().enumerate() {
            *v = read.int(i + 1, 1);
        }
        let flags = condense(values);
        let key = mod_key(e, a.script);
        let on = disabled(e, how, flags, key);
        a.set_result(e, f64::from(u8::from(on)));
    }
    handled(true)
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_Game.cpp:
// Cmd_GetPlayerControlsDisabledAltEx_Execute.
/// How (default 0) and a bit field (default all).
fn get_disabled_alt_ex(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    a.set_result(e, 0.0);
    let read = Read::of(e, &a);
    if read.ok {
        let how = read.int(0, 0);
        let flags = read.int(1, ALL);
        let key = mod_key(e, a.script);
        let on = disabled(e, how, flags, key);
        a.set_result(e, f64::from(u8::from(on)));
    }
    handled(true)
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Hooks_Gameplay.cpp:
// MaybePreventReload::PlayerCharacter_Reload_Hook.
/// `PlayerCharacter::Reload(weapon, animType, hasExtendedClip, isInstantSwapHotkey)`: with the
/// mods' reloading disabled, a reload that plays its animation (types 1 and 2) outside V.A.T.S.
/// playback answers false and does nothing; anything else goes on to the game's.
fn reload(e: &mut Engine, a: &[u32]) -> Ret {
    let alt = e.controls_alt();
    if alt != 0 && alt & RELOAD != 0 {
        let playback = e.mem.u32(VATS_CAMERA + VATS_MODE) == VATS_PLAYBACK;
        let anim_type = a.get(2).copied().unwrap_or(0);
        if !playback && (anim_type == 1 || anim_type == 2) {
            return Ret::default();
        }
    }
    e.call_next(RELOAD_FN, a)
}

#[cfg(test)]
#[path = "controls_tests.rs"]
mod tests;
