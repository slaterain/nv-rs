//! `functions_jip/jip_fn_weather.h` (JIP LN NVSE): a weather's image space modifiers, cloud
//! textures, precipitation model, numbers and colours, and the sky's current weather, wind,
//! lightning and clouds. 16 of its 18 commands.
//!
//! Not built (they only set a value one of JIP's hooks reads; hook sites, steps 3.5/3.9):
//! `SetWeatherTransitionTimeOverride` (`0x27A6`, JIP's `UpdateWeather` hook and
//! `s_weatherTransitionRateOverride`) and `SetWindSpeedMult` (`0x2858`, `s_windSpeedMult`).
//!
//! The weather argument is read as `kParamType_WeatherID`, whose type check xNVSE's reader makes,
//! so the commands check no type themselves. The game reads a weather's fields when it uses the
//! weather (the sky's update), so a set takes effect from then on, as with JIP.
//!
//! The double-to-byte stores (`weather->cloudSpeed[i] = value * 2550` and the rest) are the
//! compiled C++'s conversion: `cvttsd2si` to a 32-bit integer, its low byte stored.

use super::forms::{Clouds, Sky, TESTexture, TESWeather, SKY, SKY_OBJECT_UPDATE};
use super::{
    assign_string, buffer_arg, cvtd2ul, cvttsd2si, model_path, rgb_dec_to_hex, rgb_hex_to_dec,
    set_form_result, set_model_path, string_set, string_text, Handlers, DBL_180_D_PI, FLT_PI_D_180,
};
use crate::console::{print_form, print_result};
use engine_core::prelude::*;
use engine_core::ptr::{Field, Layout};
use nvse::commands::Read;
use nvse::{handled, CommandArgs};

#[rustfmt::skip]
pub static COMMANDS: Handlers = &[
    (0x2653, get_weather_image_space_mod),
    (0x2654, set_weather_image_space_mod),
    (0x2655, get_weather_texture),
    (0x2656, set_weather_texture),
    (0x2657, get_weather_precipitation_model),
    (0x2658, set_weather_precipitation_model),
    (0x2659, get_weather_trait_numeric),
    (0x265a, set_weather_trait_numeric),
    (0x265b, get_weather_rgb_color),
    (0x265c, set_weather_rgb_color),
    (0x26cf, get_current_weather),
    (0x2855, get_wind_direction),
    (0x2856, set_wind_direction),
    (0x2859, trigger_lightning_fx),
    (0x287e, reset_clouds),
    (0x28cd, reload_cloud_textures),
];

/// JIP's path buffers (`char path[0x80]`).
const PATH_BUFFER: usize = 0x80;
/// The 16 bytes `ResetClouds` zeroes (`_mm_storeu_ps((float*)0x11FF8B4, 0)`).
const CLOUD_OFFSETS: u32 = 0x011f_f8b4;

fn sky(e: &Engine) -> Ptr<Sky> {
    Ptr::new(e.global::<u32>(SKY))
}

/// The weather argument 0, when the read succeeded.
fn weather_arg(read: &Read) -> Option<Ptr<TESWeather>> {
    read.ok.then(|| Ptr::new(read.form(0)))
}

/// Element `i` of an array field of `stride`-byte entries (`weather->imageSpaceMods[i]`, ...).
fn nth<T>(p: Ptr<T>, i: u32, stride: u32) -> Ptr<T> {
    p.index(i, stride)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_weather.h:
// Cmd_GetWeatherImageSpaceMod_Execute.
/// The image space modifier of time `0` to `5`.
fn get_weather_image_space_mod(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let time = read.int(1, 0);
    if let Some(weather) = weather_arg(&read).filter(|_| time <= 5) {
        let imod = e.get(nth(weather, time, 4), TESWeather::imageSpaceMods);
        if imod != 0 {
            set_form_result(e, &a, imod);
        }
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_weather.h:
// Cmd_SetWeatherImageSpaceMod_Execute.
/// The image space modifier of a time set, or cleared without one.
fn set_weather_image_space_mod(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let time = read.int(1, 0);
    if let Some(weather) = weather_arg(&read).filter(|_| time <= 5) {
        e.set(
            nth(weather, time, 4),
            TESWeather::imageSpaceMods,
            read.form(2),
        );
    }
    handled(true)
}

fn layer_path(weather: Ptr<TESWeather>, layer: u32) -> u32 {
    nth(weather, layer, TESTexture::SIZE)
        .at(TESWeather::layerTextures)
        .at(TESTexture::ddsPath)
        .addr()
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_weather.h:
// Cmd_GetWeatherTexture_Execute.
/// The cloud texture path of layer `0` to `3`.
fn get_weather_texture(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let layer = read.int(1, 0);
    let text = match weather_arg(&read).filter(|_| layer <= 3) {
        Some(weather) => string_text(e, layer_path(weather, layer)),
        None => None,
    };
    assign_string(e, &a, text);
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_weather.h:
// Cmd_SetWeatherTexture_Execute.
fn set_weather_texture(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let layer = read.int(1, 0);
    let path = buffer_arg(e, &read, 2, PATH_BUFFER, "SetWeatherTexture");
    if let Some(weather) = weather_arg(&read).filter(|_| layer <= 3) {
        string_set(e, layer_path(weather, layer), &path);
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_weather.h:
// Cmd_GetWeatherPrecipitationModel_Execute.
fn get_weather_precipitation_model(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let text = match weather_arg(&read) {
        Some(weather) => model_path(e, weather.at(TESWeather::model).addr()),
        None => None,
    };
    assign_string(e, &a, text);
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_weather.h:
// Cmd_SetWeatherPrecipitationModel_Execute.
fn set_weather_precipitation_model(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let path = buffer_arg(e, &read, 1, PATH_BUFFER, "SetWeatherPrecipitationModel");
    if let Some(weather) = weather_arg(&read) {
        set_model_path(e, weather.at(TESWeather::model).addr(), &path);
    }
    handled(true)
}

/// The byte of traits 6 to 12: `((UInt8*)weather)[0xDE + traitID]` (sun glare at `+0xE4` for
/// trait 6, up to lightning frequency at `+0xEA`).
fn trait_byte(weather: Ptr<TESWeather>, trait_id: u32) -> Ptr<TESWeather> {
    Ptr::new(weather.addr() + 0xde + trait_id - TESWeather::sunGlare.off)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_weather.h:
// Cmd_GetWeatherTraitNumeric_Execute.
/// Traits 0-3: a cloud layer's speed / 2550; 4: wind speed / 255; 5: transition delta / 1000;
/// 6-12: sun glare to lightning frequency / 255; 13: classification; 14: the lightning colour as
/// decimal `RRRGGGBBB`; 15-20: the six fog distances. Above 20: nothing.
fn get_weather_trait_numeric(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let trait_id = read.int(1, 0);
    let Some(weather) = weather_arg(&read).filter(|_| trait_id <= 20) else {
        return handled(true);
    };
    let value = match trait_id {
        0..=3 => {
            f64::from(e.get(nth(weather, trait_id, 1), TESWeather::cloudSpeed)) * (1.0 / 2550.0)
        }
        4 => f64::from(e.get(weather, TESWeather::windSpeed)) * (1.0 / 255.0),
        5 => f64::from(e.get(weather, TESWeather::transDelta)) * 0.001,
        6..=12 => {
            f64::from(e.get(trait_byte(weather, trait_id), TESWeather::sunGlare)) * (1.0 / 255.0)
        }
        13 => f64::from(e.get(weather, TESWeather::weatherClassification)),
        // `cvtul2d`: the unsigned word through the x87's 64-bit load, exact.
        14 => f64::from(rgb_hex_to_dec(e.get(weather, TESWeather::lightningColor))),
        _ => f64::from(e.get(nth(weather, trait_id - 15, 4), TESWeather::fogDistance)),
    };
    a.set_result(e, value);
    handled(true)
}

/// A double stored into a byte as the compiled C++ does it ([`cvttsd2si`], the low byte).
fn to_byte(value: f64) -> u8 {
    cvttsd2si(value) as u8
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_weather.h:
// Cmd_SetWeatherTraitNumeric_Execute.
/// The same traits set: speeds above their top (0.1, 1, 0.25, 1) store 255, else the value
/// scaled back; the classification only when it is 0 or a power of two (as an `int`); the
/// lightning colour only when its decimal form is at most 255255255; the fog distances as floats.
fn set_weather_trait_numeric(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let trait_id = read.int(1, 0);
    let Some(weather) = weather_arg(&read).filter(|_| trait_id <= 20) else {
        return handled(true);
    };
    let value = match read.args.get(2) {
        Some(nvse::args::Arg::Double(v)) => *v,
        _ => 0.0,
    };
    match trait_id {
        0..=3 => {
            let speed = if value > 0.1 {
                255
            } else {
                to_byte(value * 2550.0)
            };
            e.set(nth(weather, trait_id, 1), TESWeather::cloudSpeed, speed);
        }
        4 => {
            let speed = if value > 1.0 {
                255
            } else {
                to_byte(value * 255.0)
            };
            e.set(weather, TESWeather::windSpeed, speed);
        }
        5 => {
            let delta = if value > 0.25 {
                255
            } else {
                to_byte(value * 1000.0)
            };
            e.set(weather, TESWeather::transDelta, delta);
        }
        6..=12 => {
            let byte = if value > 1.0 {
                255
            } else {
                to_byte(value * 255.0)
            };
            e.set(trait_byte(weather, trait_id), TESWeather::sunGlare, byte);
        }
        13 => {
            let int = cvttsd2si(value) as u32;
            if int & int.wrapping_sub(1) == 0 {
                e.set(weather, TESWeather::weatherClassification, int as u8);
            }
        }
        14 => {
            let dec = cvtd2ul(value);
            if dec <= 255_255_255 {
                e.set(weather, TESWeather::lightningColor, rgb_dec_to_hex(dec));
            }
        }
        _ => e.set(
            nth(weather, trait_id - 15, 4),
            TESWeather::fogDistance,
            value as f32,
        ),
    }
    handled(true)
}

/// A colour of `type` (`0` to `9`, the weather's ten colour kinds) and time (`0` to `5`); kind 2
/// is the clouds', per layer (`0` to `3`): the word, none out of range.
fn color_word(
    weather: Ptr<TESWeather>,
    kind: u32,
    time: u32,
    layer: u32,
) -> Option<(Ptr<TESWeather>, Field<TESWeather, u32>)> {
    if kind != 2 {
        Some((nth(weather, kind * 6 + time, 4), TESWeather::colors))
    } else if layer <= 3 {
        Some((nth(weather, layer * 6 + time, 4), TESWeather::cloudColor))
    } else {
        None
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_weather.h:
// Cmd_GetWeatherRGBColor_Execute.
/// A colour as decimal `RRRGGGBBB`; the layer argument (default 0) only for kind 2.
fn get_weather_rgb_color(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let (kind, time, layer) = (read.int(1, 0), read.int(2, 0), read.int(3, 0));
    if let Some(weather) = weather_arg(&read).filter(|_| kind <= 9 && time <= 5) {
        if let Some((at, field)) = color_word(weather, kind, time, layer) {
            let rgb = e.get(at, field);
            a.set_result(e, f64::from(rgb_hex_to_dec(rgb)));
        }
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_weather.h:
// Cmd_SetWeatherRGBColor_Execute.
/// A colour set from decimal `RRRGGGBBB` (at most 255255255).
fn set_weather_rgb_color(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let (kind, time, rgb, layer) = (
        read.int(1, 0),
        read.int(2, 0),
        read.int(3, 0),
        read.int(4, 0),
    );
    if let Some(weather) =
        weather_arg(&read).filter(|_| kind <= 9 && time <= 5 && rgb <= 255_255_255)
    {
        if let Some((at, field)) = color_word(weather, kind, time, layer) {
            e.set(at, field, rgb_dec_to_hex(rgb));
        }
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_weather.h:
// Cmd_GetCurrentWeather_Execute.
fn get_current_weather(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let weather = e.get(sky(e), Sky::currWeather);
    if weather != 0 {
        set_form_result(e, &a, weather);
    }
    print_form(e, &a, weather);
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_weather.h:
// Cmd_GetWindDirection_Execute.
/// The sky's wind direction in degrees (`windDirection * -Dbl180dPI`, in doubles), echoed.
fn get_wind_direction(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let direction = e.get(sky(e), Sky::windDirection);
    a.set_result(e, f64::from(direction) * -DBL_180_D_PI);
    print_result(e, &a);
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_weather.h:
// Cmd_SetWindDirection_Execute.
/// The sky's wind direction set from degrees (`windDirection * -FltPId180`, in floats).
fn set_wind_direction(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if read.ok {
        let s = sky(e);
        e.set(s, Sky::windDirection, read.float(0, 0.0) * -FLT_PI_D_180);
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), nvse/GameData.cpp: Sky::GetIsRaining.
/// The current weather (else the one fading out) is a rain weather (classification 4) and the
/// weather's progress is not below 1 (`comiss 1.0, weatherPercent; setbe`: also true for NaN).
fn is_raining(e: &Engine, s: Ptr<Sky>) -> bool {
    let rainy =
        |w: u32| w != 0 && e.get(Ptr::<TESWeather>::new(w), TESWeather::weatherClassification) == 4;
    if !rainy(e.get(s, Sky::currWeather)) && !rainy(e.get(s, Sky::transWeather)) {
        return false;
    }
    let percent = e.get(s, Sky::weatherPercent);
    percent >= 1.0 || percent.is_nan()
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_weather.h:
// Cmd_TriggerLightningFX_Execute.
/// In rain, the sky's lightning flash set to its full 1.
fn trigger_lightning_fx(e: &mut Engine, w: &[u32]) -> Ret {
    let _ = w;
    let s = sky(e);
    if is_raining(e, s) {
        e.set(s, Sky::lightningFxPerc, 1.0);
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_weather.h:
// Cmd_ResetClouds_Execute.
/// The 16 bytes at `0x11FF8B4` zeroed, as JIP writes them (four floats; `Clouds::Update`
/// itself does not name the address, and nothing here depends on what reads it).
fn reset_clouds(e: &mut Engine, w: &[u32]) -> Ret {
    let _ = w;
    for i in 0..4 {
        e.mem.set_f32(CLOUD_OFFSETS + 4 * i, 0.0);
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_weather.h:
// Cmd_ReloadCloudTextures_Execute.
/// With the sky's clouds made: their forced update set and their `Update` called (the sky, 0).
fn reload_cloud_textures(e: &mut Engine, w: &[u32]) -> Ret {
    let _ = w;
    let s = sky(e);
    let clouds = e.get(s, Sky::clouds);
    if clouds != 0 {
        e.set(Ptr::<Clouds>::new(clouds), Clouds::bForceUpdate, 1);
        e.vcall(clouds, SKY_OBJECT_UPDATE, &[s.addr(), 0f32.to_bits()]);
    }
    handled(true)
}

#[cfg(test)]
#[path = "jip_fn_weather_tests.rs"]
mod tests;
