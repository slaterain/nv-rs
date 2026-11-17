//! Group GW's game patches (step 3.9: world, sky, sound, saves, start-up): its handlers, wraps and slot routines. Only group
//! GW's PR edits this file (and its own tests file beside it).
//!
//! `internal/patches_game.h` and `internal/hooks.h` (JIP LN NVSE):
//! - **G15** (a fix; "Fix sun glare texture", `SafeWrite8(0x63E705, 1)`): the sun's glare is
//!   loaded from the climate's glare texture (entry 1), not the sun's (entry 0).
//! - **G19** (a fix; "High-noon fix", `SafeWrite8` at `0x63F385`, `0x63F38C`, `0x63F3F7`,
//!   `0x63F3FE`): the morning's and the afternoon's two image-space forms swapped, so the
//!   high-noon one weighs most at noon and the day one at the ends.
//! - **G74** (a fix; `GetWindSpeedHook` at `0x63C4D6`, `0x63C52C`): the wind speed is the weather's
//!   byte times `SetWindSpeedMult`'s factor (1/255 until a mod sets it); with the command built
//!   here.
//! - **G89** (a fix; `SAFE_WRITE_BUF(0x87A12A, ..)`): an interior's seen-data section is made with
//!   its word `+0x26` (JIP's `fullySeen`, read by JIP's mini-map) cleared.
//! - **G94** (a fix; `PlayAttackSoundHook` at `0x83AD97`): any actor whose held weapon has a
//!   silencer mod plays the silenced attack sound (the exe: only the player in first person).
//! - **G104** (a fix; `SetAcousticSpaceFixHook` at `0x82D8CA`): a pending acoustic space change is
//!   applied once and cleared (the exe applied it every frame and never cleared it).
//! - **H17** (JIP's own, `InitJIPHooks`; `SkyRefreshClimateHook` at `0x63C918`): the climate
//!   `ForceClimate` forced replaces the one the sky is given; with the command built here.
//!
//! G45's three seams in this group's files (`0x8497CD`, `0x845558`, `0x846FFB`) are group GF's
//! row: GF writes their handler.
//!
//! **Threads.** The sky and the sound update run on the main thread, the seen data where a cell's
//! map is made; the commands on the script thread. Their state is one run-wide lock-free slot
//! ([`GwState`]), as JIP's statics are one per process.

use super::Area;
use crate::commands::forms::form_type;
use crate::commands::type_of;
use engine_core::extend::{Owner, SiteSlot};
use engine_core::prelude::*;
use nvse::commands::Read;
use nvse::{handled, CommandArgs};
use std::sync::atomic::{AtomicU32, Ordering};

/// G15's seam: the climate entry of the sun's glare.
pub const SUN_GLARE: SiteSlot = SiteSlot::new(Owner::Jip, 0x0063_e705);
/// G19's seams: the morning's and the afternoon's image-space indexes.
pub const NOON_MORNING: SiteSlot = SiteSlot::new(Owner::Jip, 0x0063_f385);
pub const NOON_AFTERNOON: SiteSlot = SiteSlot::new(Owner::Jip, 0x0063_f3f7);
/// G74's seams: the current and the last weather's wind speed.
pub const WIND_SPEED: SiteSlot = SiteSlot::new(Owner::Jip, 0x0063_c4d6);
pub const WIND_SPEED_LAST: SiteSlot = SiteSlot::new(Owner::Jip, 0x0063_c52c);
/// G89's seam: the `IntSeenData` constructor's end.
pub const FULLY_SEEN: SiteSlot = SiteSlot::new(Owner::Jip, 0x0087_a12a);
/// G94's seam: the attack sound's silenced test.
pub const SILENCED_ATTACK: SiteSlot = SiteSlot::new(Owner::Jip, 0x0083_ad97);
/// G104's seam: the pending acoustic space change.
pub const PENDING_SPACE: SiteSlot = SiteSlot::new(Owner::Jip, 0x0082_d8ca);
/// H17's seam: `Sky::SetCurrentClimate`'s climate.
pub const FORCED_CLIMATE: SiteSlot = SiteSlot::new(Owner::Jip, 0x0063_c918);

/// `SetWindSpeedMult` (`functions_jip/jip_fn_weather.h`), built with G74, and `ForceClimate`
/// (`functions_jip/jip_fn_climate.h`), built with H17: in those files' command lists, their
/// handlers here.
pub const SET_WIND_SPEED_MULT: u32 = 0x2858;
pub const FORCE_CLIMATE: u32 = 0x28d8;

/// The group's rows and installers.
pub const AREA: Area = Area {
    name: "GW",
    fix_sites: &[
        WIND_SPEED.addr(),
        WIND_SPEED_LAST.addr(),
        SUN_GLARE.addr(),
        NOON_MORNING.addr(),
        NOON_AFTERNOON.addr(),
        PENDING_SPACE.addr(),
        SILENCED_ATTACK.addr(),
        FULLY_SEEN.addr(),
    ],
    fix_wraps: &[],
    install_fixes,
    jip_sites: &[FORCED_CLIMATE.addr()],
    jip_wraps: &[],
    install_jip,
};

/// JIP's statics of this group, run-wide and lock-free: `s_windSpeedMult` (G74; its `f32` bits,
/// 1/255 at first) and `s_forcedClimate` (H17; 0 for none).
pub struct GwState {
    wind_speed_mult: AtomicU32,
    forced_climate: AtomicU32,
}

impl Default for GwState {
    fn default() -> Self {
        GwState {
            wind_speed_mult: AtomicU32::new((1.0f32 / 255.0).to_bits()),
            forced_climate: AtomicU32::new(0),
        }
    }
}

fn state(e: &mut Engine) -> &GwState {
    e.ext_free::<GwState>()
}

fn install_fixes(e: &mut Engine) {
    e.ext_free::<GwState>();
    e.on_site(SUN_GLARE, sun_glare);
    e.on_site(NOON_MORNING, noon_morning);
    e.on_site(NOON_AFTERNOON, noon_afternoon);
    e.on_site(WIND_SPEED, wind_speed);
    e.on_site(WIND_SPEED_LAST, wind_speed);
    e.on_site(FULLY_SEEN, fully_seen);
    e.on_site(SILENCED_ATTACK, silenced_attack);
    e.on_site(PENDING_SPACE, pending_space);
}

fn install_jip(e: &mut Engine) {
    e.ext_free::<GwState>();
    e.on_site(FORCED_CLIMATE, forced_climate);
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_game.h: InitGamePatches
// (SafeWrite8(0x63E705, 1): `push 0` becomes `push 1`).
/// G15: the sun's glare from the climate's entry 1.
fn sun_glare(_: &mut Engine, _: &[u32]) -> Option<u32> {
    Some(1)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_game.h: InitGamePatches
// (SafeWrite8(0x63F385, 4); SafeWrite8(0x63F38C, 1)).
/// G19's morning: the first form 4 (high noon), the second 1 (day); the exe's 1 then 4.
fn noon_morning(_: &mut Engine, _: &[u32]) -> Option<u32> {
    Some(4 | 1 << 8)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_game.h: InitGamePatches
// (SafeWrite8(0x63F3F7, 1); SafeWrite8(0x63F3FE, 4)).
/// G19's afternoon: the first form 1 (day), the second 4 (high noon); the exe's 4 then 1.
fn noon_afternoon(_: &mut Engine, _: &[u32]) -> Option<u32> {
    Some(1 | 4 << 8)
}

/// A weather's wind speed byte (`TESWeather` `+0xE0`, the first of its table `004ed230` reads).
const WEATHER_WIND_SPEED: u32 = 0xe0;

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_game.h: GetWindSpeedHook.
/// G74 (`a`: the weather): the byte `+0xE0` times `s_windSpeedMult` (`fild`, `fmul` of the
/// `float`), as the main thread's x87 rounds it (to 24 bits; the product of a byte and a `float`
/// is exact in `f64`, so one rounding to `f32` is the same). The exe's `004ed230(0, 1.0, 0.0)`
/// multiplies by the double 1/255 instead. Answers the speed's bits.
fn wind_speed(e: &mut Engine, a: &[u32]) -> Option<u32> {
    let byte = e.mem.u8(a[0].wrapping_add(WEATHER_WIND_SPEED));
    let mult = f32::from_bits(state(e).wind_speed_mult.load(Ordering::Relaxed));
    Some(((f64::from(byte) * f64::from(mult)) as f32).to_bits())
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_game.h: InitGamePatches
// (SAFE_WRITE_BUF(0x87A12A, "\x31\xD2\x66\x89\x50\x26\x89\x50\x28\xC9\xC2\x08\x00")).
/// G89 (`a`: the section): the word `+0x26` cleared (the `+0x28` store stays the translation's).
fn fully_seen(e: &mut Engine, a: &[u32]) -> Option<u32> {
    e.mem.set_u16(a[0].wrapping_add(0x26), 0);
    Some(0)
}

/// `ADDR_ReturnTrue` (`0x8D0360`), what `TESForm`'s slot `+0x100` (`IsActor`) holds for an actor;
/// an actor's process (`+0x68`), its level byte (`+0x28`) and the held weapon's entry (`+0x114`:
/// its extra lists `+0`, its form `+8`); `kXData_ExtraWeaponModFlags` (`0x8D`) and its flags byte
/// (`+0xC`); a weapon's three mod effects (`+0x180`, `+0x184`, `+0x188`); the silencer effects
/// (`0xB`, `0x10`); `BaseExtraList::GetByType` (`0040a320`, thiscall: the list, the type).
const RETURN_TRUE: u32 = 0x008d_0360;
const IS_ACTOR_SLOT: u32 = 0x100;
const PROCESS: u32 = 0x68;
const PROCESS_LEVEL: u32 = 0x28;
const HELD_WEAPON: u32 = 0x114;
const EXTRA_WEAPON_MOD_FLAGS: u32 = 0x8d;
const GET_BY_TYPE: u32 = 0x0040_a320;
const MOD_EFFECTS: u32 = 0x180;

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_game.h: PlayAttackSoundHook.
/// G94 (`a`: the actor), in place of the exe's test from `0x83AD97` to `0x83AE4F`: 1 (silenced)
/// when the actor is an actor (`IsActor` is `ReturnTrue`) with a high or middle-high process
/// (level 0 or 1) holding a weapon whose `ExtraWeaponModFlags` has a mod (bit 0, 1, 2) whose effect
/// is a silencer (`0xB`) or `0x10`; else 0. JIP reads the entry's first extra list and the
/// weapon without a null test: none of them answers 0 here (JIP would read address 0).
fn silenced_attack(e: &mut Engine, a: &[u32]) -> Option<u32> {
    let actor = a[0];
    let vtable = e.mem.u32(actor);
    if e.mem.u32(vtable.wrapping_add(IS_ACTOR_SLOT)) != RETURN_TRUE {
        return Some(0);
    }
    let process = e.mem.u32(actor.wrapping_add(PROCESS));
    if process == 0 || e.mem.u8(process.wrapping_add(PROCESS_LEVEL)) > 1 {
        return Some(0);
    }
    let entry = e.mem.u32(process.wrapping_add(HELD_WEAPON));
    if entry == 0 {
        return Some(0);
    }
    let weapon = e.mem.u32(entry.wrapping_add(8));
    let lists = e.mem.u32(entry);
    let list = if lists == 0 { 0 } else { e.mem.u32(lists) };
    if list == 0 {
        return Some(0);
    }
    let extra = e
        .call(GET_BY_TYPE, &args![list, EXTRA_WEAPON_MOD_FLAGS])
        .u32();
    if extra == 0 {
        return Some(0);
    }
    let flags = e.mem.u8(extra.wrapping_add(0xc));
    for slot in 0..3u32 {
        if flags & (1 << slot) != 0 && weapon != 0 {
            let effect = e.mem.u8(weapon.wrapping_add(MOD_EFFECTS + 4 * slot));
            if effect == 0xb || effect == 0x10 {
                return Some(1);
            }
        }
    }
    Some(0)
}

/// `bUpdateAcousticSpace`, the current and the next acoustic space, `FalloutAudio`'s space change
/// (`0082eae0`, cdecl: the space).
const UPDATE_ACOUSTIC_SPACE: u32 = 0x011d_cfb1;
const CURRENT_SPACE: u32 = 0x011d_cfb4;
const NEXT_SPACE: u32 = 0x011d_cfb8;
const SET_ACOUSTIC_SPACE: u32 = 0x0082_eae0;

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_game.h: SetAcousticSpaceFixHook.
/// G104 (`a`: the next space), in place of the exe's block up to `0x82D8F4`: the update flag
/// cleared; a next space that is set is cleared and, unless it is the current one, applied
/// (`0082eae0`). Answers 0 (handled).
fn pending_space(e: &mut Engine, a: &[u32]) -> Option<u32> {
    let next = a[0];
    e.mem.set_u8(UPDATE_ACOUSTIC_SPACE, 0);
    if next != 0 {
        e.mem.set_u32(NEXT_SPACE, 0);
        if next != e.mem.u32(CURRENT_SPACE) {
            e.call(SET_ACOUSTIC_SPACE, &args![next]);
        }
    }
    Some(0)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/hooks.h: SkyRefreshClimateHook.
/// H17 (`a`: the climate asked for): the forced climate when one is set (it replaces the
/// argument), else no answer (the argument stays).
fn forced_climate(e: &mut Engine, _: &[u32]) -> Option<u32> {
    let forced = state(e).forced_climate.load(Ordering::Relaxed);
    (forced != 0).then_some(forced)
}

/// The forced climate (`SetCurrentClimate` clears it when it sets another one).
pub(crate) fn forced(e: &mut Engine) -> u32 {
    state(e).forced_climate.load(Ordering::Relaxed)
}

/// Clears the forced climate.
pub(crate) fn clear_forced(e: &mut Engine) {
    state(e).forced_climate.store(0, Ordering::Relaxed);
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_weather.h:
// Cmd_SetWindSpeedMult_Execute.
/// `SetWindSpeedMult mult`: G74's factor becomes `mult / 255` (`speedMult * (1 / 255.0F)`, in
/// `float`). A failed read leaves it.
pub(crate) fn set_wind_speed_mult(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if read.ok {
        let mult = read.float(0, 0.0) * (1.0f32 / 255.0);
        state(e)
            .wind_speed_mult
            .store(mult.to_bits(), Ordering::Relaxed);
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_climate.h:
// Cmd_ForceClimate_Execute.
/// `ForceClimate [climate]`: forces the climate (H17), or clears the forced one with none; another
/// kind of form or a failed read leaves it. The sky takes it at its next climate change.
pub(crate) fn force_climate(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if read.ok {
        let form = read.form(0);
        if form == 0 || type_of(e, form) == form_type::TES_CLIMATE {
            state(e).forced_climate.store(form, Ordering::Relaxed);
        }
    }
    handled(true)
}

#[cfg(test)]
#[path = "gw_tests.rs"]
mod tests;
