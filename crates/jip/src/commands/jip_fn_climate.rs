//! `functions_jip/jip_fn_climate.h` (JIP LN NVSE): a climate's textures, night sky, times and
//! weather list, and the sky's current climate. 17 of its 18 commands.
//!
//! Not built: `ForceClimate` (`0x28D8`): it only sets JIP's `s_forcedClimate`, which JIP's
//! climate hook reads (hook sites, steps 3.5/3.9); until that hook exists nothing forces a
//! climate, so `SetCurrentClimate`'s clearing of the forced climate has nothing to clear.
//!
//! The game keeps its own copies of some of these values: the sky caches the current climate's
//! sunrise and sunset hours (its flags `0x3F00` mark them stale, set on a climate change), so a
//! new time set on the current climate is seen at its next change or refresh, as with JIP, which
//! writes the climate only.

use super::forms::{form_type, Sky, TESClimate, TESTexture, WeatherEntry, SKY};
use super::{
    assign_string, buffer_arg, heap_alloc, heap_free, model_path, set_form_result, set_model_path,
    string_set, string_text, type_of, Handlers,
};
use crate::console::print_form;
use crate::game::ListNode;
use engine_core::prelude::*;
use engine_core::ptr::Layout;
use nvse::commands::Read;
use nvse::{handled, CommandArgs};

#[rustfmt::skip]
pub static COMMANDS: Handlers = &[
    (0x266b, get_climate_sun_texture),
    (0x266c, set_climate_sun_texture),
    (0x266d, get_climate_sun_glare_texture),
    (0x266e, set_climate_sun_glare_texture),
    (0x266f, get_climate_night_sky_model),
    (0x2670, set_climate_night_sky_model),
    (0x2671, get_climate_trait_numeric),
    (0x2672, set_climate_trait_numeric),
    (0x2673, get_climate_num_weather_types),
    (0x2674, get_climate_nth_weather_type),
    (0x2675, get_climate_nth_weather_chance),
    (0x2676, get_climate_nth_weather_global),
    (0x2677, climate_add_weather_type),
    (0x2678, climate_remove_weather_type),
    (0x26d0, get_current_climate),
    (0x26d1, set_current_climate),
    (0x26d2, refresh_current_climate),
];

/// JIP's path buffers (`char path[0x80]`).
const PATH_BUFFER: usize = 0x80;
/// `Sky::RefreshClimate` (JIP LN NVSE): the game's `Sky::SetCurrentClimate` (`0063C8F0`,
/// `thiscall`: the climate and whether to change at once), which sets the climate, resets the
/// weather and marks the cached times stale when the climate differs or `immediate` is set.
const SKY_SET_CURRENT_CLIMATE: u32 = 0x0063_c8f0;

fn sky(e: &Engine) -> Ptr<Sky> {
    Ptr::new(e.global::<u32>(SKY))
}

/// The climate argument 0 when the read succeeded and it is a climate (`IS_ID(climate,
/// TESClimate)`).
fn climate_arg(e: &Engine, read: &Read) -> Option<Ptr<TESClimate>> {
    let form = read.form(0);
    (read.ok && type_of(e, form) == form_type::TES_CLIMATE).then(|| Ptr::new(form))
}

/// A climate's texture path as a string (none for a failed read or another form).
fn get_texture(e: &mut Engine, w: &[u32], texture: fn(Ptr<TESClimate>) -> Ptr<TESTexture>) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let text = climate_arg(e, &read).and_then(|c| {
        let path = texture(c).at(TESTexture::ddsPath);
        string_text(e, path.addr())
    });
    assign_string(e, &a, text);
    handled(true)
}

/// A climate's texture path set (`String::Set` from JIP's 128-byte buffer).
fn set_texture(
    e: &mut Engine,
    w: &[u32],
    command: &str,
    texture: fn(Ptr<TESClimate>) -> Ptr<TESTexture>,
) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let path = buffer_arg(e, &read, 1, PATH_BUFFER, command);
    if let Some(c) = climate_arg(e, &read) {
        let string = texture(c).at(TESTexture::ddsPath);
        string_set(e, string.addr(), &path);
    }
    handled(true)
}

fn sun(c: Ptr<TESClimate>) -> Ptr<TESTexture> {
    c.at(TESClimate::sunTexture)
}
fn sun_glare(c: Ptr<TESClimate>) -> Ptr<TESTexture> {
    c.at(TESClimate::sunGlareTexture)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_climate.h:
// Cmd_GetClimateSunTexture_Execute.
fn get_climate_sun_texture(e: &mut Engine, w: &[u32]) -> Ret {
    get_texture(e, w, sun)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_climate.h:
// Cmd_SetClimateSunTexture_Execute.
fn set_climate_sun_texture(e: &mut Engine, w: &[u32]) -> Ret {
    set_texture(e, w, "SetClimateSunTexture", sun)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_climate.h:
// Cmd_GetClimateSunGlareTexture_Execute.
fn get_climate_sun_glare_texture(e: &mut Engine, w: &[u32]) -> Ret {
    get_texture(e, w, sun_glare)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_climate.h:
// Cmd_SetClimateSunGlareTexture_Execute.
fn set_climate_sun_glare_texture(e: &mut Engine, w: &[u32]) -> Ret {
    set_texture(e, w, "SetClimateSunGlareTexture", sun_glare)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_climate.h:
// Cmd_GetClimateNightSkyModel_Execute.
/// The night sky model's path, through `TESModel`'s virtual `GetModelPath`.
fn get_climate_night_sky_model(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let text = match climate_arg(e, &read) {
        Some(c) => model_path(e, c.at(TESClimate::nightSkyModel).addr()),
        None => None,
    };
    assign_string(e, &a, text);
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_climate.h:
// Cmd_SetClimateNightSkyModel_Execute.
fn set_climate_night_sky_model(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let path = buffer_arg(e, &read, 1, PATH_BUFFER, "SetClimateNightSkyModel");
    if let Some(c) = climate_arg(e, &read) {
        set_model_path(e, c.at(TESClimate::nightSkyModel).addr(), &path);
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_climate.h:
// Cmd_GetClimateTraitNumeric_Execute.
/// Trait 0-3: sunrise begin, sunrise end, sunset begin, sunset end in minutes (the stored
/// ten-minute units times 10); 4: volatility; 5: the moons (the top two bits of the phase byte);
/// 6: the phase length (its low six). Any other trait leaves the result.
fn get_climate_trait_numeric(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if let Some(c) = climate_arg(e, &read) {
        let value = match read.int(1, 0) {
            0 => Some(u32::from(e.get(c, TESClimate::sunriseBegin)) * 10),
            1 => Some(u32::from(e.get(c, TESClimate::sunriseEnd)) * 10),
            2 => Some(u32::from(e.get(c, TESClimate::sunsetBegin)) * 10),
            3 => Some(u32::from(e.get(c, TESClimate::sunsetEnd)) * 10),
            4 => Some(u32::from(e.get(c, TESClimate::volatility))),
            5 => Some(u32::from(e.get(c, TESClimate::phaseLength) >> 6)),
            6 => Some(u32::from(e.get(c, TESClimate::phaseLength) & 63)),
            _ => None,
        };
        if let Some(v) = value {
            a.set_result(e, f64::from(v));
        }
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_climate.h:
// Cmd_SetClimateTraitNumeric_Execute.
/// The same traits set: times as minutes (above 1430, 143; else divided by 10), volatility up
/// to 255, the moons up to 3 (in the top two bits), the phase length up to 63 (the low six).
fn set_climate_trait_numeric(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if let Some(c) = climate_arg(e, &read) {
        let val = read.int(2, 0);
        let time = if val > 1430 { 143 } else { (val / 10) as u8 };
        match read.int(1, 0) {
            0 => e.set(c, TESClimate::sunriseBegin, time),
            1 => e.set(c, TESClimate::sunriseEnd, time),
            2 => e.set(c, TESClimate::sunsetBegin, time),
            3 => e.set(c, TESClimate::sunsetEnd, time),
            4 => e.set(c, TESClimate::volatility, val.min(255) as u8),
            5 => {
                let low = e.get(c, TESClimate::phaseLength) & 63;
                let moons = if val > 3 { 192 } else { (val << 6) as u8 };
                e.set(c, TESClimate::phaseLength, low.wrapping_add(moons));
            }
            6 => {
                let high = e.get(c, TESClimate::phaseLength) & 192;
                e.set(c, TESClimate::phaseLength, high + val.min(63) as u8);
            }
            _ => {}
        }
    }
    handled(true)
}

/// `tList::Count` (JIP LN NVSE): 0 for an empty first node, else every node.
fn list_count(e: &Engine, head: Ptr<ListNode>) -> u32 {
    if e.get(head, ListNode::data) == 0 {
        return 0;
    }
    let mut count = 1;
    let mut node = e.get(head, ListNode::next);
    while node != 0 {
        count += 1;
        node = e.get(Ptr::<ListNode>::new(node), ListNode::next);
    }
    count
}

/// `tList::GetNthItem` (JIP LN NVSE): the item of node `index` (a negative index none), 0 past
/// the end.
fn nth_item(e: &Engine, head: Ptr<ListNode>, index: i32) -> u32 {
    if index < 0 {
        return 0;
    }
    let mut node = head;
    for _ in 0..index {
        node = Ptr::new(e.get(node, ListNode::next));
        if node.is_null() {
            return 0;
        }
    }
    e.get(node, ListNode::data)
}

fn weathers(c: Ptr<TESClimate>) -> Ptr<ListNode> {
    c.at(TESClimate::weatherTypes)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_climate.h:
// Cmd_GetClimateNumWeatherTypes_Execute.
fn get_climate_num_weather_types(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if let Some(c) = climate_arg(e, &read) {
        let count = list_count(e, weathers(c));
        a.set_result(e, f64::from(count as i32));
    }
    handled(true)
}

/// The weather list's entry at argument 1, for a climate argument 0.
fn nth_entry(e: &Engine, read: &Read) -> Option<Ptr<WeatherEntry>> {
    let c = climate_arg(e, read)?;
    let entry = nth_item(e, weathers(c), read.int(1, 0) as i32);
    (entry != 0).then(|| Ptr::new(entry))
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_climate.h:
// Cmd_GetClimateNthWeatherType_Execute.
fn get_climate_nth_weather_type(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if let Some(entry) = nth_entry(e, &read) {
        let weather = e.get(entry, WeatherEntry::weather);
        if weather != 0 {
            set_form_result(e, &a, weather);
        }
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_climate.h:
// Cmd_GetClimateNthWeatherChance_Execute.
fn get_climate_nth_weather_chance(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if let Some(entry) = nth_entry(e, &read) {
        let chance = e.get(entry, WeatherEntry::chance);
        a.set_result(e, f64::from(chance as i32));
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_climate.h:
// Cmd_GetClimateNthWeatherGlobal_Execute.
fn get_climate_nth_weather_global(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if let Some(entry) = nth_entry(e, &read) {
        let global = e.get(entry, WeatherEntry::global);
        if global != 0 {
            set_form_result(e, &a, global);
        }
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), nvse/GameForms.cpp: TESClimate::GetWeatherEntry,
// with tList's ListNode::RemoveMe, RemoveNext and Prepend (nvse/GameTypes.h).
/// The climate's entry for `weather` (the first node whose entry has it). With `remove` its
/// node is taken out (the next node's item and link copied into the first node, its node freed;
/// a lone first node only emptied; a later node unlinked and freed) and the entry answered, or
/// none when there is no such entry. Without `remove`, a missing entry is made: a new 12-byte
/// block of the game's heap (not cleared), put first in the list.
fn weather_entry(e: &mut Engine, c: Ptr<TESClimate>, weather: u32, remove: bool) -> Option<u32> {
    let head = weathers(c);
    let mut node = head;
    let mut prev = Ptr::<ListNode>::NULL;
    loop {
        let entry = e.get(node, ListNode::data);
        if entry != 0 && e.get(Ptr::<WeatherEntry>::new(entry), WeatherEntry::weather) == weather {
            if remove {
                if !prev.is_null() {
                    let gone = e.get(prev, ListNode::next);
                    let after = e.get(Ptr::<ListNode>::new(gone), ListNode::next);
                    e.set(prev, ListNode::next, after);
                    heap_free(e, gone);
                } else {
                    let next = e.get(node, ListNode::next);
                    if next != 0 {
                        let next_node = Ptr::<ListNode>::new(next);
                        let item = e.get(next_node, ListNode::data);
                        let after = e.get(next_node, ListNode::next);
                        e.set(node, ListNode::data, item);
                        e.set(node, ListNode::next, after);
                        heap_free(e, next);
                    } else {
                        e.set(node, ListNode::data, 0);
                    }
                }
            }
            return Some(entry);
        }
        prev = node;
        node = Ptr::new(e.get(node, ListNode::next));
        if node.is_null() {
            break;
        }
    }
    if remove {
        return None;
    }
    let entry = heap_alloc(e, WeatherEntry::SIZE);
    // `tList::Prepend`: into an empty first node, else `ListNode::Insert` (the first item moves
    // to a new node after the first).
    if e.get(head, ListNode::data) != 0 {
        let added = heap_alloc(e, ListNode::SIZE);
        let added_node = Ptr::<ListNode>::new(added);
        let first = e.get(head, ListNode::data);
        let next = e.get(head, ListNode::next);
        e.set(added_node, ListNode::data, first);
        e.set(head, ListNode::data, entry);
        e.set(added_node, ListNode::next, next);
        e.set(head, ListNode::next, added);
    } else {
        e.set(head, ListNode::data, entry);
    }
    Some(entry)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_climate.h:
// Cmd_ClimateAddWeatherType_Execute.
/// The weather's entry (found or made) given the weather, the chance modulo 101 and the global
/// (none when not given); the result 1.
fn climate_add_weather_type(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if let Some(c) = climate_arg(e, &read) {
        let weather = read.form(1);
        if let Some(entry) = weather_entry(e, c, weather, false) {
            let entry = Ptr::<WeatherEntry>::new(entry);
            e.set(entry, WeatherEntry::weather, weather);
            e.set(entry, WeatherEntry::chance, read.int(2, 0) % 101);
            e.set(entry, WeatherEntry::global, read.form(3));
            a.set_result(e, 1.0);
        }
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_climate.h:
// Cmd_ClimateRemoveWeatherType_Execute.
/// The weather's entry taken out of the list and freed; the result 1 when there was one.
fn climate_remove_weather_type(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if let Some(c) = climate_arg(e, &read) {
        if let Some(entry) = weather_entry(e, c, read.form(1), true) {
            heap_free(e, entry);
            a.set_result(e, 1.0);
        }
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_climate.h:
// Cmd_GetCurrentClimate_Execute.
/// The sky's current climate, echoed to the console as JIP echoes a form.
fn get_current_climate(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let climate = e.get(sky(e), Sky::currClimate);
    if climate != 0 {
        set_form_result(e, &a, climate);
    }
    print_form(e, &a, climate);
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_climate.h:
// Cmd_SetCurrentClimate_Execute.
/// The sky's current climate set, nothing else (the game sees it at its next climate update).
/// JIP first clears its forced climate when it is another one; `ForceClimate` is not built, so
/// there is none (see the module's notes).
fn set_current_climate(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if let Some(c) = climate_arg(e, &read) {
        let s = sky(e);
        e.set(s, Sky::currClimate, c.addr());
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_climate.h:
// Cmd_RefreshCurrentClimate_Execute.
/// The game's `Sky::SetCurrentClimate` with the current climate, at once.
fn refresh_current_climate(e: &mut Engine, w: &[u32]) -> Ret {
    let _ = w;
    let s = sky(e);
    let climate = e.get(s, Sky::currClimate);
    e.call(SKY_SET_CURRENT_CLIMATE, &args![s.addr(), climate, 1u32]);
    handled(true)
}

#[cfg(test)]
#[path = "jip_fn_climate_tests.rs"]
mod tests;
