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
//! - **O5** (`bGameDaysPassedFix`, on by default; `UpdateTimeGlobalsHook` at `0x867A40`, a wrap):
//!   the calendar keeps the hours and days passed in doubles between frames, so `GameDaysPassed`
//!   no longer drifts over a long game.
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
use engine_core::extend::{jip_patches, Owner, SiteSlot, WrapSlot};
use engine_core::prelude::*;
use nvse::commands::Read;
use nvse::{handled, CommandArgs};
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};

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
/// O5's wrap: `GameTimeGlobals::Update` (`bGameDaysPassedFix`).
pub const UPDATE_TIME_GLOBALS: WrapSlot = WrapSlot::new(Owner::Jip, 0x0086_7a40);

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
    fix_wraps: &[UPDATE_TIME_GLOBALS.addr()],
    install_fixes,
    jip_sites: &[FORCED_CLIMATE.addr()],
    jip_wraps: &[],
    install_jip,
};

/// JIP's statics of this group, run-wide and lock-free: `s_windSpeedMult` (G74; its `f32` bits,
/// 1/255 at first), `s_forcedClimate` (H17; 0 for none) and `UpdateTimeGlobalsHook`'s
/// `hourAndDays` (O5; the two doubles' bits, 0 at first).
pub struct GwState {
    wind_speed_mult: AtomicU32,
    forced_climate: AtomicU32,
    hour_and_days: [AtomicU64; 2],
}

impl Default for GwState {
    fn default() -> Self {
        GwState {
            wind_speed_mult: AtomicU32::new((1.0f32 / 255.0).to_bits()),
            forced_climate: AtomicU32::new(0),
            hour_and_days: [AtomicU64::new(0), AtomicU64::new(0)],
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
    e.wrap(UPDATE_TIME_GLOBALS, update_time_globals);
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

/// `GameTimeGlobals` (the calendar, `0x11DE7B8`): its globals (year `+0`, month `+4`, day `+8`, hour
/// `+0xC`, days passed `+0x10`, time scale `+0x14`; a global's value at `+0x24`), the midnights
/// passed (`+0x18`) and the "days passed stale" byte (`+0x1C`). The player (`0x11DEA3C`) and its
/// hardcore needs update byte (`+0xE39`). JIP's `kMults`: 24, 1/3600 (`0x3F323456789ABCDF`) and
/// the hour's jump limits +0.05 and -0.05.
const YEAR: u32 = 0;
const MONTH: u32 = 4;
const DAY: u32 = 8;
const HOUR: u32 = 0xc;
const DAYS_PASSED: u32 = 0x10;
const TIME_SCALE: u32 = 0x14;
const MIDNIGHTS: u32 = 0x18;
const RECALC: u32 = 0x1c;
const GLOBAL_VALUE: u32 = 0x24;
const PLAYER: u32 = 0x011d_ea3c;
const HARDCORE_UPDATE: u32 = 0xe39;
const DAY_HOURS: f64 = 24.0;
const PER_SECOND: u64 = 0x3f32_3456_789a_bcdf;
const JUMP_AHEAD: u64 = 0x3fa9_9999_9999_999a;
const JUMP_BACK: u64 = 0xbfa9_9999_9999_999a;
/// `kDaysPerMonth` (`internal/jip_core.cpp`).
const DAYS_PER_MONTH: [u32; 12] = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];

fn global(e: &Engine, calendar: u32, at: u32) -> u32 {
    e.mem
        .u32(calendar.wrapping_add(at))
        .wrapping_add(GLOBAL_VALUE)
}

/// `cvttss2si`/`cvttps2dq`: a float cut to a whole number, `i32::MIN` where it does not fit (NaN).
fn cut(v: f32) -> i32 {
    if v.is_nan() || v >= 2_147_483_648.0 || v < -2_147_483_648.0 {
        i32::MIN
    } else {
        v as i32
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_game.h: UpdateTimeGlobalsHook.
/// O5 (`bGameDaysPassedFix`, on by default; `HOOK_INIT_JUMP(0x867A40, ..)`), in place of
/// `GameTimeGlobals::Update` (`w`: the calendar, the seconds passed as `float` bits): the hours
/// passed (`seconds * time scale / 3600`) and the days passed are kept in doubles between calls
/// (`hourAndDays`), so they no longer drift. The hour is taken from the double JIP kept unless the
/// global moved more than 0.05 hours from it (a script set it: the days are worked out again from
/// the hour and the whole days, back over midnight counting as a day on) or the calendar's stale
/// byte asks; a recount sets the player's hardcore update byte (`+0xE39`) unless `SetGameHour` asked
/// for the needs to take the jump (then clear). Past 24 hours the day, month (JIP's table, no leap
/// years) and year roll on, and the midnights passed count up. Without the switch, the exe's
/// function. Difference from JIP: with no player the byte is not written (JIP would write at
/// address 0xE39); a month outside 0 to 11 reads 31 days (JIP reads past its table).
fn update_time_globals(e: &mut Engine, w: &[u32]) -> Ret {
    if e.jip_patches() & jip_patches::GAME_DAYS_PASSED_FIX == 0 {
        return e.call_next(UPDATE_TIME_GLOBALS.addr(), w);
    }
    let calendar = w[0];
    let seconds = f32::from_bits(w[1]);
    let per_second = f64::from_bits(PER_SECOND);
    let scale = e.mem.f32(global(e, calendar, TIME_SCALE));
    let passed = f64::from(seconds) * f64::from(scale) * per_second;
    let mut hour = f64::from(e.mem.f32(global(e, calendar, HOUR)));
    let cached = [0, 1].map(|i| f64::from_bits(state(e).hour_and_days[i].load(Ordering::Relaxed)));
    let mut days;
    let mut recount = e.mem.u8(calendar.wrapping_add(RECALC)) != 0;
    if !recount {
        let moved = hour - cached[0];
        if moved > f64::from_bits(JUMP_AHEAD) {
            recount = true;
        } else if moved < f64::from_bits(JUMP_BACK) || moved.is_nan() {
            // `comisd; jb`: back past the limit, or NaN.
            hour += DAY_HOURS;
            recount = true;
        }
    }
    if recount {
        e.mem.set_u8(calendar.wrapping_add(RECALC), 0);
        let forced = super::gp::force_hardcore_needs_update(e);
        super::gp::set_force_hardcore_needs_update(e, false);
        let player = e.mem.u32(PLAYER);
        if player != 0 {
            e.mem
                .set_u8(player.wrapping_add(HARDCORE_UPDATE), u8::from(!forced));
        }
        let whole = cut(e.mem.f32(global(e, calendar, DAYS_PASSED)));
        days = hour / DAY_HOURS + f64::from(whole);
    } else {
        hour = cached[0];
        days = cached[1];
    }
    hour += passed;
    if hour > DAY_HOURS {
        let mut month = cut(e.mem.f32(global(e, calendar, MONTH))) as u32;
        let mut day = cut(e.mem.f32(global(e, calendar, DAY))) as u32;
        let mut month_days = DAYS_PER_MONTH.get(month as usize).copied().unwrap_or(31);
        while hour > DAY_HOURS {
            hour -= DAY_HOURS;
            if day < month_days {
                day += 1;
            } else {
                day = 1;
                if month < 0xb {
                    month += 1;
                    month_days = DAYS_PER_MONTH[month as usize];
                } else {
                    month = 0;
                    month_days = 31;
                    let year = global(e, calendar, YEAR);
                    let value = e.mem.f32(year) + 1.0;
                    e.mem.set_f32(year, value);
                }
            }
        }
        e.mem
            .set_f32(global(e, calendar, MONTH), month as i32 as f32);
        e.mem.set_f32(global(e, calendar, DAY), day as i32 as f32);
        let at = calendar.wrapping_add(MIDNIGHTS);
        let midnights = e.mem.u32(at);
        e.mem.set_u32(at, midnights.wrapping_add(1));
    }
    e.mem.set_f32(global(e, calendar, HOUR), hour as f32);
    days += passed / DAY_HOURS;
    e.mem.set_f32(global(e, calendar, DAYS_PASSED), days as f32);
    let s = state(e);
    s.hour_and_days[0].store(hour.to_bits(), Ordering::Relaxed);
    s.hour_and_days[1].store(days.to_bits(), Ordering::Relaxed);
    Ret::default()
}

#[cfg(test)]
#[path = "gw_tests.rs"]
mod tests;
