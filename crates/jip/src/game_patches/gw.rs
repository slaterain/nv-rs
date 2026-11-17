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

use super::gh1::{GAME_RNG, GET_RANDOM_INT};
use super::Area;
use crate::commands::forms::form_type;
use crate::commands::type_of;
use engine_core::extend::{jip_patches, Owner, SiteSlot, WrapSlot};
use engine_core::prelude::*;
use nvse::commands::Read;
use nvse::{handled, CommandArgs};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::sync::Mutex;

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
/// G113's seams: the region's incidental (`RDSI`) and battle (`RDSB`) media set ids read.
pub const MEDIA_SET_ID: SiteSlot = SiteSlot::new(Owner::Jip, 0x004f_49ab);
pub const BATTLE_MEDIA_SET_ID: SiteSlot = SiteSlot::new(Owner::Jip, 0x004f_49d2);
/// G113's wraps: `MediaLocationController`'s list pick (`00595560`) and
/// `TESRegionDataSound::GetMediaSetBattle` (`004f4f90`, the vtable word `0x1023DD8` holds).
pub const PICK_MEDIA_SET: WrapSlot = WrapSlot::new(Owner::Jip, 0x0059_5560);
pub const PICK_BATTLE_MEDIA_SET: WrapSlot = WrapSlot::new(Owner::Jip, 0x004f_4f90);
/// D05's wrap: the sound file picker `BSWin32Audio::PickSoundFileFromFolder` holds
/// (`005e3630`).
pub const PICK_SOUND_FILE: WrapSlot = WrapSlot::new(Owner::Jip, 0x005e_3630);
/// G115's seam: `main`'s launcher test (`0x86AC60`), where JIP reads `FalloutCustom.ini`.
pub const CUSTOM_INI: SiteSlot = SiteSlot::new(Owner::Jip, 0x0086_ac60);
/// G108's seams: `ApplySettingsFromMasterFile`'s request (`0x82D416`), the audio manager's message
/// kind 54 (its jump table word `0xAE5648`, group GA's seam) and the game sound's constructor
/// (`0xAEACAA`).
pub const SOUND_REQUEST: SiteSlot = SiteSlot::new(Owner::Jip, 0x0082_d416);
pub const SOURCE_SOUND: SiteSlot = SiteSlot::new(Owner::Jip, 0x00ae_5648);
pub const FREQUENCY_FIELDS: SiteSlot = SiteSlot::new(Owner::Jip, 0x00ae_acaa);
/// G108's wraps: `BSWin32GameSound`'s frequency set and read (vtable slots `+0x64`, `+0x68`, the
/// words `0x10A3C58`, `0x10A3C5C` hold).
pub const SET_FREQUENCY: WrapSlot = WrapSlot::new(Owner::Jip, 0x00af_01e0);
pub const GET_FREQUENCY: WrapSlot = WrapSlot::new(Owner::Jip, 0x00af_0290);

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
        MEDIA_SET_ID.addr(),
        BATTLE_MEDIA_SET_ID.addr(),
        CUSTOM_INI.addr(),
        SOUND_REQUEST.addr(),
        SOURCE_SOUND.addr(),
        FREQUENCY_FIELDS.addr(),
    ],
    fix_wraps: &[
        UPDATE_TIME_GLOBALS.addr(),
        PICK_MEDIA_SET.addr(),
        PICK_BATTLE_MEDIA_SET.addr(),
        PICK_SOUND_FILE.addr(),
        SET_FREQUENCY.addr(),
        GET_FREQUENCY.addr(),
    ],
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
    /// `s_pickMediaSetMap` (G113): each list's media sets still to play, by the list's first
    /// node.
    media_set_bags: Mutex<HashMap<u32, Vec<u32>>>,
}

impl Default for GwState {
    fn default() -> Self {
        GwState {
            wind_speed_mult: AtomicU32::new((1.0f32 / 255.0).to_bits()),
            forced_climate: AtomicU32::new(0),
            hour_and_days: [AtomicU64::new(0), AtomicU64::new(0)],
            media_set_bags: Mutex::new(HashMap::new()),
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
    e.on_site(MEDIA_SET_ID, media_set_id);
    e.on_site(BATTLE_MEDIA_SET_ID, media_set_id);
    e.wrap(PICK_MEDIA_SET, pick_media_set);
    e.wrap(PICK_BATTLE_MEDIA_SET, pick_battle_media_set);
    e.wrap(PICK_SOUND_FILE, pick_sound_file);
    e.on_site(CUSTOM_INI, custom_ini);
    e.on_site(SOUND_REQUEST, sound_request);
    e.on_site(SOURCE_SOUND, source_sound);
    e.on_site(FREQUENCY_FIELDS, frequency_fields);
    e.wrap(SET_FREQUENCY, set_frequency);
    e.wrap(GET_FREQUENCY, get_frequency);
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

/// `TESForm::AddCompileIndex` (`00485d50`, cdecl: the form id's slot, the file).
const ADD_COMPILE_INDEX: u32 = 0x0048_5d50;

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_game.h: LoadMediaSetIDHook
// (credited there to lStewieAl and TrueCourierSix).
/// G113 (`a`: the file, the id's slot), after the exe's `GetChunkData` read the id: the id resolved
/// through the plugin's masters (`AddCompileIndex(slot, file)`), so a region's music from a plugin
/// finds its media sets. Answers 0 (not read).
fn media_set_id(e: &mut Engine, a: &[u32]) -> Option<u32> {
    e.call(ADD_COMPILE_INDEX, &args![a[1], a[0]]);
    Some(0)
}

/// JIP's `GetRandomUInt(bound)`: `0..bound` from the game's generator.
fn random_uint(e: &mut Engine, bound: u32) -> u32 {
    e.call(GET_RANDOM_INT, &args![GAME_RNG, bound]).u32()
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_game.h: PickMediaSetHook;
// internal/utility.h: ArrayUtils::Shuffle; internal/containers.h: Vector::Pop.
/// G113, in place of `00595560` (`w`: the controller, not read; the list's first node, whose
/// data `+0` and next `+4`): a list of one node answers its media set. Otherwise JIP keeps a bag
/// per list (by its first node): an empty bag is filled with the list's media sets (nulls
/// skipped) in order and shuffled (`GetRandomUInt(count)` for each place from the first, swapped
/// when not 0); the bag's last one is taken out and answered (0 when the bag stays empty). So the
/// sets play in a random order with no repeat until each has played; the list is not changed
/// (the exe moved its pick to the end). Without the fixes: the exe's function. Difference from
/// JIP: no list (0) answers 0, as the exe does (JIP would read address 0).
fn pick_media_set(e: &mut Engine, w: &[u32]) -> Ret {
    if e.jip_patches() & jip_patches::FIXES == 0 {
        return e.call_next(PICK_MEDIA_SET.addr(), w);
    }
    let head = w[1];
    if head == 0 {
        return Ret::default();
    }
    if e.mem.u32(head + 4) == 0 {
        return e.mem.u32(head).into_ret();
    }
    let mut bag = state(e)
        .media_set_bags
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .remove(&head)
        .unwrap_or_default();
    if bag.is_empty() {
        let mut node = head;
        while node != 0 {
            let data = e.mem.u32(node);
            if data != 0 {
                bag.push(data);
            }
            node = e.mem.u32(node + 4);
        }
        let mut count = bag.len() as u32;
        let mut at = 0usize;
        while count > 1 {
            let rand = random_uint(e, count);
            if rand != 0 {
                bag.swap(at, at + rand as usize);
            }
            count -= 1;
            at += 1;
        }
    }
    let picked = bag.pop().unwrap_or(0);
    state(e)
        .media_set_bags
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .insert(head, bag);
    picked.into_ret()
}

/// `kFormType_MediaSet` (`0x6F`), a form's type byte (`+4`).
const MEDIA_SET_TYPE: u8 = 0x6f;

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_game.h: PickBattleMediaSetHook.
/// G113, in place of `TESRegionDataSound::GetMediaSetBattle` (`w`: the region's sound data; its
/// battle media set ids are the list at `+0x18`): no first id answers 0. With more than one id,
/// one of the others (the second with two; else `GetRandomUInt(count - 1)` picks among the second
/// to the last) swaps places with the first, so the one played last is never picked next. The
/// first id's form answers when it is a media set, else 0. Without the fixes: the exe's function.
fn pick_battle_media_set(e: &mut Engine, w: &[u32]) -> Ret {
    if e.jip_patches() & jip_patches::FIXES == 0 {
        return e.call_next(PICK_BATTLE_MEDIA_SET.addr(), w);
    }
    let head = w[0].wrapping_add(0x18);
    if e.mem.u32(head) == 0 {
        return Ret::default();
    }
    let mut others = 0u32;
    let mut node = e.mem.u32(head + 4);
    while node != 0 {
        others += 1;
        node = e.mem.u32(node + 4);
    }
    if others != 0 {
        let mut picked = e.mem.u32(head + 4);
        if others != 1 {
            let rand = random_uint(e, others);
            picked = head;
            for _ in 0..=rand {
                picked = e.mem.u32(picked + 4);
            }
        }
        let first = e.mem.u32(head);
        let id = e.mem.u32(picked);
        e.mem.set_u32(head, id);
        e.mem.set_u32(picked, first);
    }
    let form = crate::cosave::lookup_form(e, e.mem.u32(head));
    if form == 0 || e.mem.u8(form + 4) != MEDIA_SET_TYPE {
        return Ret::default();
    }
    form.into_ret()
}

/// The game's flag that the folder lists are to be built (`011acdf9`, JIP's "invalidate"); the
/// archive's file test (`00af7400`, cdecl: the search path, the out path, the archive types) and
/// `FileFinder::BuildFileList` (`00afe420`, cdecl: the search path, the out path, the types, 0;
/// a list of nodes, the name `+0` and the next `+4`, on the game's heap).
const USE_FILE_LISTS: u32 = 0x011a_cdf9;
const ARCHIVE_FIND: u32 = 0x00af_7400;
const BUILD_FILE_LIST: u32 = 0x00af_e420;
/// The archive types JIP asks for (8: sounds).
const SOUND_ARCHIVES: u32 = 8;
/// JIP's search path buffer (`char[0x100]`), here with room for the longest path the caller's
/// buffer holds (`0x104`) and the pattern.
const SEARCH_PATH: u32 = 0x110;

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_game.h: PickSoundFileFromFolderHook.
/// D05, in place of `005e3630` (`w`: the folder path, `char[0x104]`, which gets the file picked):
/// without the game's flag `011acdf9`, the archive's file test of `path*.wav` (the found name
/// into `path`) answers. With it, the lists of `path*.wav` and `path*.ogg` are built and joined
/// (the `.ogg` list after the `.wav` one): none answers false; one file is taken, of more,
/// `GetRandomUInt(count)` picks; that name goes into `path`, every name and node is freed, and the
/// answer is true. So a folder of `.ogg` files plays with `.wav` ones (the exe took the `.ogg`
/// files only when there were any, and kept a pick from repeating). Without the fixes: the exe's
/// function.
fn pick_sound_file(e: &mut Engine, w: &[u32]) -> Ret {
    if e.jip_patches() & jip_patches::FIXES == 0 {
        return e.call_next(PICK_SOUND_FILE.addr(), w);
    }
    let path = w[0];
    let found = e.with_stack(SEARCH_PATH, |e, buffer| {
        let buffer = buffer.addr();
        let folder = e.mem.cstr(path);
        let end = buffer + folder.len() as u32;
        e.mem.set_cstr(buffer, &folder);
        e.mem.set_cstr(end, b"*.wav");
        if e.mem.u8(USE_FILE_LISTS) == 0 {
            return e
                .call(ARCHIVE_FIND, &args![buffer, path, SOUND_ARCHIVES])
                .u8();
        }
        let wavs = e
            .call(BUILD_FILE_LIST, &args![buffer, path, SOUND_ARCHIVES, 0u32])
            .u32();
        e.mem.write(end + 1, b".ogg");
        let oggs = e
            .call(BUILD_FILE_LIST, &args![buffer, path, SOUND_ARCHIVES, 0u32])
            .u32();
        let list = match (wavs, oggs) {
            (0, 0) => return 0,
            (0, oggs) => oggs,
            (wavs, 0) => wavs,
            (wavs, oggs) => {
                let mut last = wavs;
                while e.mem.u32(last + 4) != 0 {
                    last = e.mem.u32(last + 4);
                }
                e.mem.set_u32(last + 4, oggs);
                wavs
            }
        };
        let mut others = 0u32;
        let mut node = list;
        while e.mem.u32(node + 4) != 0 {
            node = e.mem.u32(node + 4);
            others += 1;
        }
        let mut pick = if others == 0 {
            0
        } else {
            random_uint(e, others + 1)
        };
        let mut node = list;
        while node != 0 {
            let name = e.mem.u32(node);
            if pick == 0 {
                let text = if name == 0 { vec![] } else { e.mem.cstr(name) };
                e.mem.set_cstr(path, &text);
            }
            pick = pick.wrapping_sub(1);
            let next = e.mem.u32(node + 4);
            crate::commands::heap_free(e, name);
            crate::commands::heap_free(e, node);
            node = next;
        }
        1
    });
    u32::from(found).into_ret()
}

/// `FalloutCustom.ini`: JIP reads it from the game's settings folder (`My Games\FalloutNV`); the
/// port reads it as a mod's file (`engine_core::mod_files`): from the port's own folder beside the
/// saves, else the game's folder. Never written.
const CUSTOM_INI_FILE: &[u8] = b"FalloutCustom.ini";

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/utility.cpp: StrToUInt.
/// A decimal number at the start of `text` as an unsigned word: an optional minus sign (the
/// value then negated), the digits up to the first other byte; when a digit makes the value fall
/// (unsigned) below the one before, 0xFFFFFFFF, or 0 with the sign.
fn str_to_uint(text: &[u8]) -> u32 {
    let at = |i: usize| text.get(i).copied().unwrap_or(0);
    let negative = at(0) == b'-';
    let mut i = usize::from(negative);
    let (mut value, mut last) = (0u32, 0u32);
    loop {
        let digit = at(i).wrapping_sub(b'0');
        if digit > 9 {
            break;
        }
        value = value.wrapping_mul(10).wrapping_add(u32::from(digit));
        if value < last {
            return u32::from(negative).wrapping_sub(1);
        }
        last = value;
        i += 1;
    }
    if negative {
        value.wrapping_neg()
    } else {
        value
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_game.h: ProcessCustomINI;
// nvse/GameSettings.cpp: Setting::Set(const char*, bool).
/// G115, before `main`'s launcher test (`0x86AC60`; the exe's call follows): when
/// `FalloutCustom.ini` is there, each `key=value` of each of its sections sets the INI setting
/// `key:section` (the settings map JIP's `GetNumericGameSetting` uses, made now when it is not
/// yet), by the key's first letter: `b` 1 for a value starting `1` else 0 (the word), `i`
/// `StrToInt`, `u` `StrToUInt`, `f` `StrToDbl` as a `float`, `s` a copy of the text on the game's
/// heap (the old text not freed, as JIP's `Set(.., false)`); another letter, or no such setting,
/// is left. The sections and their lines as Windows' profile calls give them (each list in JIP's
/// `0x4000`-byte buffer). Answers 0 (not read). Difference from JIP: the file is the port's (see
/// [`CUSTOM_INI_FILE`]), and a name longer than JIP's `0x100`-byte buffer is taken whole.
fn custom_ini(e: &mut Engine, _: &[u32]) -> Option<u32> {
    use crate::commands::ln_fn_utility::{walked, MAX_MESSAGE_LENGTH};
    use engine_core::mod_files;
    if !mod_files::exists(e, CUSTOM_INI_FILE) {
        return Some(0);
    }
    let names = mod_files::profile_section_names(e, CUSTOM_INI_FILE).unwrap_or_default();
    let (buffer, count) = mod_files::profile_list(&names, MAX_MESSAGE_LENGTH);
    for section in walked(&buffer, count) {
        let lines = mod_files::profile_section(e, CUSTOM_INI_FILE, &section).unwrap_or_default();
        let (buffer, count) = mod_files::profile_list(&lines, MAX_MESSAGE_LENGTH);
        for pair in walked(&buffer, count) {
            let (key, value) = crate::commands::jip_fn_ui::next_token(&pair, b'=');
            let name = [key, b":", &section[..]].concat();
            let Some(setting) = crate::overrides::game_setting(e, &name) else {
                continue;
            };
            let data = setting + 4;
            match name.first().copied().unwrap_or(0) | 0x20 {
                b'b' => e.mem.set_u32(data, u32::from(value.first() == Some(&b'1'))),
                b'i' => e.mem.set_i32(data, crate::utility::str_to_int(value)),
                b'u' => e.mem.set_u32(data, str_to_uint(value)),
                b'f' => e
                    .mem
                    .set_f32(data, crate::utility::str_to_dbl(value) as f32),
                b's' => {
                    let copy = crate::commands::heap_alloc(e, value.len() as u32 + 1);
                    e.mem.set_cstr(copy, value);
                    e.mem.set_u32(data, copy);
                }
                _ => {}
            }
        }
    }
    Some(0)
}

/// A game sound's (`BSWin32GameSound`) last bytes of its name buffer, where JIP keeps the sound's
/// form (`+0x134`) and its frequency factor (`+0x138`, a `float`); its base rate
/// (`iModSamplesPerSecond`, `+0x15A`, a 16-bit word), the device (`+0x198`) and the buffer
/// (`+0x19C`, an `IDirectSoundBuffer`: `GetFrequency` slot `0x20`, `SetFrequency` slot `0x44`).
const SOUND_FORM: u32 = 0x134;
const FREQUENCY_FACTOR: u32 = 0x138;
const SAMPLES_PER_SECOND: u32 = 0x15a;
const DEVICE: u32 = 0x198;
const BUFFER: u32 = 0x19c;
/// The audio manager (`0x11F6EF0`) and its message post (`00adb1a0`, thiscall: the kind, three
/// words, the smart pointer, three floats); its sounds' map (`+0x58` the bucket count, `+0x5C`
/// the buckets; an entry's next `+0`, key `+4`, sound `+8`).
const AUDIO_MANAGER: u32 = 0x011f_6ef0;
const POST_MESSAGE: u32 = 0x00ad_b1a0;
const SOURCE_SOUND_MESSAGE: u32 = 0x36;
const SOUND_MAP_SIZE: u32 = 0x58;
const SOUND_MAP_BUCKETS: u32 = 0x5c;
/// JIP's `kFreqMult` (0.01 for a raised pitch, 0.005 for a lowered one) and its limits
/// (`SS_100`, `kFlt200K`).
const FREQUENCY_UP: u32 = 0x3c23_d70a;
const FREQUENCY_DOWN: u32 = 0x3ba3_d70a;
const LOWEST_FREQUENCY: f32 = 100.0;
const HIGHEST_FREQUENCY: f32 = 200_000.0;

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_game.h: InitGamePatches
// (SAFE_WRITE_BUF(0xAEACAA, ..)).
/// G108 (`a`: the game sound being made): JIP's form field cleared and its factor 1.0 (the exe's
/// three clears JIP's bytes also make are the translation's). Answers 0 (not read).
fn frequency_fields(e: &mut Engine, a: &[u32]) -> Option<u32> {
    e.mem.set_u32(a[0].wrapping_add(SOUND_FORM), 0);
    e.mem.set_f32(a[0].wrapping_add(FREQUENCY_FACTOR), 1.0);
    Some(0)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_game.h: FillGameSoundPropsHook.
/// G108 (`a`: the sound handle, the sound form), at the start of `ApplySettingsFromMasterFile`
/// (the form is not null): for a handle whose sound id (its first word) is not negative, a message
/// of kind 54 with the id and the form is posted to the audio manager (the audio thread then sets
/// the game sound's factor, [`source_sound`]). Answers 0 (not read).
fn sound_request(e: &mut Engine, a: &[u32]) -> Option<u32> {
    let id = e.mem.u32(a[0]);
    if (id as i32) >= 0 {
        e.call(
            POST_MESSAGE,
            &args![
                AUDIO_MANAGER,
                SOURCE_SOUND_MESSAGE,
                id,
                a[1],
                0u32,
                0u32,
                0u32,
                0u32,
                0u32
            ],
        );
    }
    Some(0)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_game.h: GetFrequencyModifier.
/// A sound form's frequency factor: 1.0, or with its flag `+0x48` bit 0 clear and its signed
/// percentage byte `+0x46` not 0, `1 + percent * 0.01` (raised) or `1 + percent * 0.005`
/// (lowered), in `float`.
fn frequency_modifier(e: &Engine, form: u32) -> f32 {
    if e.mem.u8(form.wrapping_add(0x48)) & 1 != 0 {
        return 1.0;
    }
    let percent = i32::from(e.mem.u8(form.wrapping_add(0x46)) as i8);
    if percent == 0 {
        return 1.0;
    }
    let mult = f32::from_bits(if percent < 0 {
        FREQUENCY_DOWN
    } else {
        FREQUENCY_UP
    });
    1.0 + percent as f32 * mult
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_game.h: SetSourceSoundRequestHook.
/// G108 (`a`: the audio manager, the message), for the message kind 54 (the exe has no case for
/// it): the game sound with the message's id (`+4`) in the manager's map gets the message's form
/// (`+8`) at `+0x134` and its frequency factor at `+0x138`; no such sound, nothing. Answers 0 (not
/// read; the exe's end follows). Differences from JIP: a map of no buckets finds nothing (JIP
/// would divide by 0), and a null form gets the factor 1.0 (JIP would read address 0x46).
fn source_sound(e: &mut Engine, a: &[u32]) -> Option<u32> {
    let (manager, message) = (a[0], a[1]);
    let count = e.mem.u32(manager.wrapping_add(SOUND_MAP_SIZE));
    if count == 0 {
        return Some(0);
    }
    let id = e.mem.u32(message.wrapping_add(4));
    let buckets = e.mem.u32(manager.wrapping_add(SOUND_MAP_BUCKETS));
    let mut entry = e.mem.u32(buckets.wrapping_add(4 * (id % count)));
    while entry != 0 {
        if e.mem.u32(entry + 4) == id {
            let sound = e.mem.u32(entry + 8);
            let form = e.mem.u32(message.wrapping_add(8));
            e.mem.set_u32(sound.wrapping_add(SOUND_FORM), form);
            let factor = if form == 0 {
                1.0
            } else {
                frequency_modifier(e, form)
            };
            e.mem.set_f32(sound.wrapping_add(FREQUENCY_FACTOR), factor);
            break;
        }
        entry = e.mem.u32(entry);
    }
    Some(0)
}

/// `cvtss2si` under the default rounding: to nearest, ties to even (the value is within 100 to
/// 200000 here).
fn cvtss2si(v: f32) -> i32 {
    v.round_ties_even() as i32
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_game.h: AdjustSoundFrequencyHook.
/// G108, in place of `BSWin32GameSound::SetFrequency` (`w`: the sound, the ratio as `float`
/// bits): with a buffer and a device, the base rate times the sound's factor times the ratio (in
/// `float`), kept within 100 to 200000 (`maxss`, `minss`: a NaN is 100), rounded to nearest, goes
/// to the buffer's `SetFrequency`. Answers 0 always (the exe answered whether the call worked).
/// Without the fixes: the exe's function.
fn set_frequency(e: &mut Engine, w: &[u32]) -> Ret {
    if e.jip_patches() & jip_patches::FIXES == 0 {
        return e.call_next(SET_FREQUENCY.addr(), w);
    }
    let this = w[0];
    let buffer = e.mem.u32(this.wrapping_add(BUFFER));
    if buffer == 0 || e.mem.u32(this.wrapping_add(DEVICE)) == 0 {
        return Ret::default();
    }
    let rate = f32::from(e.mem.u16(this.wrapping_add(SAMPLES_PER_SECOND)));
    let factor = e.mem.f32(this.wrapping_add(FREQUENCY_FACTOR));
    let v = rate * factor * f32::from_bits(w[1]);
    let v = if v > LOWEST_FREQUENCY {
        v
    } else {
        LOWEST_FREQUENCY
    };
    let v = if v < HIGHEST_FREQUENCY {
        v
    } else {
        HIGHEST_FREQUENCY
    };
    e.vcall(buffer, 0x44, &[cvtss2si(v) as u32]);
    Ret::default()
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_game.h: GetSoundFrequencyPercHook.
/// G108, in place of `BSWin32GameSound::GetFrequency` (`w`: the sound): with a buffer and a device
/// whose `GetFrequency` works, that frequency over the base rate times the sound's factor (in
/// `float`, so a pitched sound reads its ratio as the game set it); else 1.0. Without the fixes:
/// the exe's function.
fn get_frequency(e: &mut Engine, w: &[u32]) -> Ret {
    if e.jip_patches() & jip_patches::FIXES == 0 {
        return e.call_next(GET_FREQUENCY.addr(), w);
    }
    let this = w[0];
    let buffer = e.mem.u32(this.wrapping_add(BUFFER));
    if buffer == 0 || e.mem.u32(this.wrapping_add(DEVICE)) == 0 {
        return 1.0f32.into_ret();
    }
    let read = e.with_stack(4, |e, frequency| {
        let at = frequency.addr();
        e.mem.set_u32(at, 0);
        let result = e.vcall(buffer, 0x20, &[at]).u32();
        ((result as i32) >= 0).then(|| e.mem.i32(at))
    });
    let Some(frequency) = read else {
        return 1.0f32.into_ret();
    };
    let rate = f32::from(e.mem.u16(this.wrapping_add(SAMPLES_PER_SECOND)));
    let base = rate * e.mem.f32(this.wrapping_add(FREQUENCY_FACTOR));
    (frequency as f32 / base).into_ret()
}

#[cfg(test)]
#[path = "gw_tests.rs"]
mod tests;
