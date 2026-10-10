//! `fallout shared/sky/sky.cpp` (Xbox PDB source unit), subsystem `fallout shared/sky`: its functions in
//! FalloutNV.exe 1.4.0.525, translated (docs/ENGINE_CRATE.md).
//!
//! `Sky` (the singleton at `pInstance`) owns the sky's scene-graph roots, the
//! sky components (atmosphere, stars, sun, clouds, the two moons, the
//! precipitation), the current climate and weather and the colours and fog
//! values the weather blend produces each frame. The file also holds the
//! `SkySound` helpers and a few weather and clouds accessors the linker
//! placed between the `Sky` methods; each says whose method it is.
//!
//! Layouts and helpers are at the top; the functions follow in address
//! order. The next session continues at `0063d060` (the next function the queue lists).
//!
//! x87 note: the game computes in extended precision and stores `float`
//! results. The translations compute in `f64` and round to `f32` at every
//! `float` store (docs/ENGINE_CRATE.md); the comparisons the game makes with
//! `FCOMPP` keep their NaN behaviour (a comparison with NaN is false).

#[allow(unused_imports)]
use crate::prelude::*;

// ---- callees outside this file ----

/// `NiPointer<T>::get` (`__thiscall`, no arguments): the pointer stored at
/// `this`.
pub(crate) const NI_POINTER_GET: u32 = 0x0055_9450;
/// `NiPointer<T>::operator=(T*)` (`RET 4`): releases the old object, stores
/// the new one and adds a reference to it.
pub(crate) const NI_POINTER_ASSIGN: u32 = 0x0066_b0d0;
/// `NiPointer<T>::operator!=(T*)` (`RET 4`): `*this != pointer`.
const NI_POINTER_NOT_EQUAL: u32 = 0x0052_aa80;
/// `NiPointer<T>::~NiPointer`: releases the held object.
const NI_POINTER_DESTRUCTOR: u32 = 0x0045_cec0;
/// `BSSimpleList` node accessors (`__thiscall`): the address of the node's
/// item (`this`) and the next node (`this + 4`); `IsEmpty` tests both words
/// of the head node, `RemoveHead` unlinks and frees the first node, `Clear`
/// empties the list and `0047 02f0` is its scalar deleting destructor.
const LIST_NODE_ITEM: u32 = 0x0068_15c0;
const LIST_NODE_NEXT: u32 = 0x0072_6070;
const LIST_IS_EMPTY: u32 = 0x0082_56d0;
const LIST_REMOVE_HEAD: u32 = 0x0063_f7b0;
const LIST_CLEAR: u32 = 0x0047_0470;
const LIST_SCALAR_DELETING_DESTRUCTOR: u32 = 0x0047_02f0;
/// `operator new` (`cdecl`, size), `operator delete` (`cdecl`, block) and the
/// `NiObject` allocation (`cdecl`, size).
const OPERATOR_NEW: u32 = 0x0040_1000;
const OPERATOR_DELETE: u32 = 0x0040_1030;
const NI_OBJECT_NEW: u32 = 0x00aa_13e0;
/// `memset` (`cdecl`: destination, value, size).
const MEMSET: u32 = 0x0040_3d30;
/// Scope timer constructor (`ECX` = timer, `RET 0x10`: kind, 1, file, line)
/// and destructor.
const TIMER_SCOPE_CONSTRUCT: u32 = 0x0040_4eb0;
const TIMER_SCOPE_DESTROY: u32 = 0x0040_4ee0;
/// `"D:\_Fallout3\Platforms\Common\Code\Fallout Shared\Sky\Sky.cpp"`.
const SKY_SOURCE_FILE_NAME: u32 = 0x0104_edf0;
/// `Sky::vftable`.
const SKY_VTABLE: u32 = 0x0104_edec;

/// `BSSoundHandle` constructor (`{ -1, 0, 0 }`), copy assignment (`RET 4`)
/// and (empty) destructor; `IsPlaying`, `Stop` and `Release`.
const SOUND_HANDLE_CONSTRUCT: u32 = 0x0041_a250;
const SOUND_HANDLE_ASSIGN: u32 = 0x0041_8900;
const SOUND_HANDLE_DESTRUCT: u32 = 0x0048_3710;
const SOUND_HANDLE_IS_PLAYING: u32 = 0x00ad_8930;
const SOUND_HANDLE_STOP: u32 = 0x00ad_88f0;
const SOUND_HANDLE_RELEASE: u32 = 0x00ad_8d10;
/// The body of `SkySound`'s destructor (a `BSSoundHandle` destructor).
const SKY_SOUND_BASE_DESTRUCT: u32 = 0x0052_d240;

/// Scene-graph node visibility setter (`__thiscall`, `RET 4`, one byte; the
/// underwater code passes 1 to hide the sky nodes).
const SET_NODE_HIDDEN: u32 = 0x0045_0f90;
/// The scene-graph node of a `Sun` (`this + 0xc`, `__thiscall`).
const SUN_GET_NODE: u32 = 0x0043_b230;
/// The number of layers of a `Clouds` (`__thiscall`).
const CLOUDS_LAYER_COUNT: u32 = 0x0063_4830;

/// `Sky::UnloadAllTextures` and the function that runs when mode 2 or 3 is entered (both `__thiscall`, no arguments); `Sky::UpdateHDRValues`.
const SKY_UNLOAD_ALL_TEXTURES: u32 = 0x0063_e210;
const SKY_ENTER_VISIBLE_MODE: u32 = 0x0063_e2f0;
const SKY_UPDATE_HDR_VALUES: u32 = 0x0063_ef20;
/// Precipitation: update (`__thiscall`, `float`), constructor (0x18 bytes)
/// and `Initialize`.
const PRECIPITATION_UPDATE: u32 = 0x0063_6ed0;
const PRECIPITATION_CONSTRUCT: u32 = 0x0063_6a40;
const PRECIPITATION_INITIALIZE: u32 = 0x0063_6bb0;
/// Constructors of `Atmosphere` (0x1c bytes), `Stars` (0x10), `Sun` (0x2c)
/// and `Clouds` (0x5c).
const ATMOSPHERE_CONSTRUCT: u32 = 0x0063_3080;
const STARS_CONSTRUCT: u32 = 0x0063_fc70;
const SUN_CONSTRUCT: u32 = 0x0064_04f0;
const CLOUDS_CONSTRUCT: u32 = 0x0063_3cc0;
/// The moons' root `NiNode` constructor (0xac bytes, one argument) and the
/// `BSMultiBoundNode` constructor (0xb4 bytes).
const MOONS_ROOT_CONSTRUCT: u32 = 0x00a5_ecb0;
const MULTIBOUND_NODE_CONSTRUCT: u32 = 0x00c4_6970;
/// Calls on the sky's root node in `Initialize` (all `__thiscall`): the two
/// that take one flag word (`1`), the one taking the root's name argument,
/// `AttachProperty`, `UpdateProperties`, and the object whose virtual slot
/// `0xe8` takes the root away from its scene graph.
const ROOT_SET_FIRST_FLAG: u32 = 0x0054_68d0;
const ROOT_SET_SECOND_FLAG: u32 = 0x0049_02f0;
const ROOT_SET_NAME: u32 = 0x0044_0460;
const ROOT_ATTACH_PROPERTY: u32 = 0x0043_9410;
const ROOT_UPDATE_PROPERTIES: u32 = 0x00a5_a040;
const ROOT_GET_SCENE: u32 = 0x0096_11e0;
/// The string `Initialize` passes to [`ROOT_SET_NAME`].
const ROOT_NAME_ARGUMENT: u32 = 0x011f_426c;
/// The sky's property object: constructor (0x1c bytes, `NiObject`
/// allocation) and four setters called with `6`, `7`, `1` and `1`.
const SKY_PROPERTY_CONSTRUCT: u32 = 0x0043_91c0;
const SKY_PROPERTY_SET_A: u32 = 0x0043_9340;
const SKY_PROPERTY_SET_B: u32 = 0x0043_9390;
const SKY_PROPERTY_SET_C: u32 = 0x0049_ed90;
const SKY_PROPERTY_SET_D: u32 = 0x0063_6000;
/// `NiPointer` constructor from a raw pointer (`00633c90`, translated in
/// `run_00633c90.rs`).
const NI_POINTER_FROM_RAW: u32 = 0x0063_3c90;
/// A setting object (`ECX` = [`SETTING_PRECIPITATION`]) whose accessor
/// returns a pointer to its byte value.
const SETTING_PRECIPITATION: u32 = 0x011c_cb8c;
const SETTING_BYTE_VALUE: u32 = 0x0040_8d60;
/// The accessor of a setting object holding a `float` (`ECX` = the setting,
/// returns a pointer to the value).
const SETTING_FLOAT_VALUE: u32 = 0x0040_3e20;
/// A global byte; when set `Initialize` stores `1.0` in the two floats.
const INITIAL_FLAG: u32 = 0x011f_941e;
const INITIAL_FLOAT_A: u32 = 0x011a_d87c;
const INITIAL_FLOAT_B: u32 = 0x011a_d880;
/// `SetMode` clears this byte when it enters mode 0 or 1 from 2, 3 or 4.
const SET_MODE_FLAG: u32 = 0x011c_cb74;

// ---- callees and data of the colour, fog and update code ----

/// `_ftol2_sse` (`00ec62c0`): truncates the float in `ST0`, passed here as an
/// `f64` argument (two words).
const FTOL: u32 = 0x00ec_62c0;
/// The player (a pointer global) and its `parentCell` accessor (`__thiscall`,
/// returns the word at `+0x40`).
const PLAYER_CHARACTER: u32 = 0x011d_ea3c;
const PLAYER_PARENT_CELL: u32 = 0x008d_6f30;
/// `TESObjectREFR::GetRelevantWaterHeight` (`ECX` = the player, `float` in
/// `ST0`) and the cell's water height (`ECX` = cell, `float` in `ST0`).
const RELEVANT_WATER_HEIGHT: u32 = 0x0057_b0a0;
const CELL_WATER_HEIGHT: u32 = 0x0054_71e0;
/// The `NiPointer` global holding the camera and its helpers: the camera
/// (no arguments), the camera's scene node, and a node's world position
/// (`node + 0x8c`).
const CAMERA_HOLDER_GET: u32 = 0x0045_c670;
const CAMERA_NODE: u32 = 0x0055_8310;
const NODE_WORLD_POSITION: u32 = 0x0045_bb80;
/// The calendar (an object at a fixed address): `Calendar::GetHour` (`float`
/// in `ST0`) and the day count accessor (`ECX` = calendar, returns a word).
const CALENDAR: u32 = 0x011d_e7b8;
const CALENDAR_GET_HOUR: u32 = 0x0086_7da0;
const CALENDAR_GET_DAYS: u32 = 0x0086_7de0;
/// A global pointer to an object whose accessor `0042ce10` returns bit 1 of
/// the word at `+0x244`.
const WORLD_STATE_POINTER: u32 = 0x011d_df38;
const WORLD_STATE_FLAG: u32 = 0x0042_ce10;
/// Steps `Sky::Update` calls on other units' code: `0063d1d0`, `0063d5e0`
/// (`__thiscall`, no arguments), the moon-phase test `006368f0` (`cdecl`,
/// the sky) and `004dce50` (`cdecl`, the colour at `+0x48`).
const SKY_UPDATE_TRANSITION: u32 = 0x0063_d1d0;
const SKY_UPDATE_UNKNOWN: u32 = 0x0063_d5e0;
const MOON_PHASE_APPLIES: u32 = 0x0063_68f0;
const FOG_COLOR_UPDATE: u32 = 0x004d_ce50;
/// `Sky::ResetWeather`.
const SKY_RESET_WEATHER: u32 = 0x0063_d060;
/// The tick counter object (`ECX`) and its reader (returns the word at
/// `+0x14`); the lightning flash duration setting.
const TICK_COUNTER: u32 = 0x011f_6394;
const TICK_COUNT_GET: u32 = 0x0082_5c00;
const FLASH_DURATION_SETTING: u32 = 0x011c_cc34;
/// Doubles in the exe's data: `0.0`, `1.0`, `0.5`, `1000.0`.
const DOUBLE_ZERO: u32 = 0x0101_2060;
const DOUBLE_ONE: u32 = 0x0101_2070;
const DOUBLE_HALF: u32 = 0x0101_1588;
const THOUSAND_DOUBLE: u32 = 0x0101_7b70;

/// `Sky` methods other units define: the sunrise begin and end and the
/// sunset begin and end hours of the current climate (`__thiscall`, `float`
/// in `ST0`). The sky caches the first and last in the globals below.
const SKY_SUNRISE_BEGIN: u32 = 0x0059_5ea0;
const SKY_SUNRISE_END: u32 = 0x0059_5f50;
const SKY_SUNSET_BEGIN: u32 = 0x0059_5fc0;
const SKY_SUNSET_END: u32 = 0x0059_6030;
const SUNRISE_BEGIN_CACHE: u32 = 0x011c_ccfc;
const SUNSET_END_CACHE: u32 = 0x011c_cd00;
/// The time-transition setting.
const TIME_SETTING: u32 = 0x011c_cbd8;
/// `max(a, b)` and `min(a, b)` of two `float`s (`cdecl`; the second argument
/// is the result when the comparison is false or unordered).
const MAX_FLOAT: u32 = 0x0040_4010;
const MIN_FLOAT: u32 = 0x0040_ebd0;
/// `clamp(&value, low, high)` (`cdecl`) and the colour clamp to at most 1
/// (`ECX` = colour).
const CLAMP_FLOAT: u32 = 0x0053_30e0;
const COLOR_CLAMP_TO_ONE: u32 = 0x00a6_9690;
/// `NiColor::NiColor(r, g, b)` (`__thiscall`, returns `this`).
const NI_COLOR_CONSTRUCT: u32 = 0x0041_6870;
/// A weather's byte table fraction (`ECX` = weather; index, high, low):
/// `table[index] / 255 * (high - low) + low`.
const WEATHER_BYTE_FRACTION: u32 = 0x004e_d230;
/// The log call for invalid climate data (`cdecl`, one string) and its
/// message.
const LOG_MASTERFILE_ERROR: u32 = 0x005b_5e40;
const TRANSITION_TIMES_MESSAGE: u32 = 0x0104_ee38;
/// The interior-cell holder global, the accessor `005f36f0` that returns its
/// interior cell (the engine map names it `ActorMover::GetPreferredMoveMode`
/// because the linker folded identical code) and the interior cell's
/// lighting colour and fog getters.
const INTERIOR_CELL_HOLDER: u32 = 0x011d_ea10;
const GET_INTERIOR_CELL: u32 = 0x005f_36f0;
const CELL_COLOR_FIRST: u32 = 0x0054_4890;
const CELL_COLOR_SECOND: u32 = 0x0054_47b0;
const CELL_COLOR_THIRD: u32 = 0x0054_4a30;
const CELL_FOG_FAR: u32 = 0x0054_4b10;
const CELL_FOG_NEAR: u32 = 0x0054_4ab0;
const CELL_FOG_POWER: u32 = 0x0054_4b70;
/// The fog default (a `float`), the interior far-distance limit (a double),
/// the factor for the near distance (a double) and the camera-relative fog
/// setting.
const DEFAULT_FOG_DISTANCE: u32 = 0x0103_17c4;
const FOG_FAR_LIMIT: u32 = 0x0103_a480;
const FOG_NEAR_FACTOR: u32 = 0x0104_ee90;
const FOG_SETTING: u32 = 0x0120_3144;
/// The water volume's fog near and far distances (`ECX` = volume), its two
/// colours, and the setting that scales the depth.
const WATER_FOG_NEAR: u32 = 0x004e_3d00;
const WATER_FOG_FAR: u32 = 0x0050_7b20;
const WATER_COLOR_A: u32 = 0x004f_9bf0;
const WATER_COLOR_B: u32 = 0x009e_e040;
const WATER_DEPTH_SCALE: u32 = 0x011c_cc60;
/// The two global flag bytes the fog code tests (getters without arguments).
const FLAG_UNDERWATER_A: u32 = 0x004e_2180;
const FLAG_UNDERWATER_B: u32 = 0x004e_3c40;
/// Lightning scale settings for sky colours 3 and 4.
const LIGHTNING_SCALE_COLOR_3: u32 = 0x011c_cbc0;
const LIGHTNING_SCALE_COLOR_4: u32 = 0x011c_cccc;
/// `Sky` accessors (`__thiscall`): the clouds (`+0x2c`), the atmosphere
/// (`+0x20`) and the sun (`+0x28`); and the `NiPointer` at `+8` of a
/// component.
const SKY_GET_CLOUDS: u32 = 0x0055_b980;
const SKY_GET_ATMOSPHERE: u32 = 0x007a_f430;
const SKY_GET_SUN: u32 = 0x0045_cd60;
const HOLDER_NODE: u32 = 0x0055_85e0;
/// `TESForm` lookup by id (`cdecl`), `__RTDynamicCast` (`cdecl`: object,
/// vfdelta, source type, target type, reference flag) and the two type
/// descriptors `Sky::SetCurrentClimate` casts between.
const LOOKUP_FORM: u32 = 0x0048_39c0;
const DYNAMIC_CAST: u32 = 0x00ec_43fb;
const TYPE_DESCRIPTOR_FORM: u32 = 0x0118_3028;
const TYPE_DESCRIPTOR_CLIMATE: u32 = 0x0118_4170;
/// `Stars::LoadGeometry` (Xbox PDB).
const STARS_LOAD_GEOMETRY: u32 = 0x0063_fdb0;
/// Sun texture loading: the node that holds the glare property (`+0x10` of
/// the sun's holder, `__thiscall`), `NiAVObject::GetProperty(type)`
/// (`RET 4`), the climate texture entry's validity test and name, the
/// `BSStringT` constructor, destructor, assignment from a `char*` and append,
/// a case-insensitive comparison (`cdecl`), the texture loader (`cdecl`:
/// path, 1, holder, 1, 0), the property's texture setter and the node flag
/// setter (`ECX` = node, one byte).
const SUN_FIRST_NODE: u32 = 0x006f_a820;
const NODE_GET_PROPERTY: u32 = 0x00a5_9d30;
const TEXTURE_ENTRY_VALID: u32 = 0x0048_cee0;
const TEXTURE_ENTRY_NAME: u32 = 0x0040_8da0;
const STRING_CONSTRUCT: u32 = 0x0040_37b0;
const STRING_DESTRUCT: u32 = 0x0040_37d0;
const STRING_ASSIGN: u32 = 0x0043_8390;
const STRING_APPEND: u32 = 0x0040_4820;
const STRING_COMPARE: u32 = 0x0040_4dc0;
const TEXTURE_LOAD: u32 = 0x00b5_5840;
const PROPERTY_SET_TEXTURE: u32 = 0x0063_48e0;
const NODE_SET_FLAG_BIT_20: u32 = 0x0063_5fe0;
/// `"Sky\SunGlare.dds"` and `"Textures\Sky\SunGlareNonHDR.dds"`.
const SUN_GLARE_NAME: u32 = 0x0102_d7c4;
const SUN_GLARE_NON_HDR_PATH: u32 = 0x0104_eeb8;
/// `NiUpdateData(time, flag, flag)` constructor (`RET 0xc`) and the node call
/// that takes it (`RET 4`).
const NI_UPDATE_DATA_CONSTRUCT: u32 = 0x0043_d410;
const NODE_UPDATE_CONTROLLERS: u32 = 0x00a5_9c60;
/// A setting object holding a word (`ECX` = setting, returns a pointer to
/// the value) and the `Moon` constructor (0x7c bytes; `RET 0x1c`: name,
/// five floats, one word).
const SETTING_WORD_VALUE: u32 = 0x0043_d4d0;
const MOON_CONSTRUCT: u32 = 0x0063_4a70;

layout! {
    /// `Sky` (Xbox PDB), 0x138 bytes; the offsets used here match the PC
    /// code. `Sky::pInstance` is the singleton.
    pub struct Sky: 0x138 {
        /// `spRoot` (Xbox PDB): `NiPointer<BSMultiBoundNode>`.
        0x04 spRoot: u32,
        /// `spMoonsRoot` (Xbox PDB): `NiPointer<NiNode>`.
        0x08 spMoonsRoot: u32,
        /// `pCurrentClimate` (Xbox PDB): `TESClimate*`.
        0x0C pCurrentClimate: Ptr,
        /// `pCurrentWeather` (Xbox PDB): `TESWeather*`.
        0x10 pCurrentWeather: Ptr,
        /// `pLastWeather` (Xbox PDB): `TESWeather*`.
        0x14 pLastWeather: Ptr,
        /// `pAtmosphere` (Xbox PDB).
        0x20 pAtmosphere: Ptr,
        /// `pStars` (Xbox PDB).
        0x24 pStars: Ptr,
        /// `pSun` (Xbox PDB).
        0x28 pSun: Ptr,
        /// `pClouds` (Xbox PDB).
        0x2C pClouds: Ptr,
        /// `pMasser` (Xbox PDB): a `Moon`.
        0x30 pMasser: Ptr,
        /// `pSecunda` (Xbox PDB): a `Moon`.
        0x34 pSecunda: Ptr,
        /// `pPrecip` (Xbox PDB): the `Precipitation`.
        0x38 pPrecip: Ptr,
        /// `SkyColor` (Xbox PDB): ten `NiColor`s of 12 bytes from here
        /// ([`SKY_COLOR_STRIDE`]).
        0x3C SkyColor: u32,
        /// `WaterFogColor` (Xbox PDB).
        0xB4 WaterFogColor: u32,
        /// `SunSpecularColor` (Xbox PDB).
        0xC0 SunSpecularColor: u32,
        /// `fWindSpeed` (Xbox PDB).
        0xCC fWindSpeed: f32,
        /// `fFogDistances` (Xbox PDB): the near distance.
        0xD4 fFogNear: f32,
        /// `fFogDistances` (Xbox PDB): the far distance.
        0xD8 fFogFar: f32,
        /// `fFogPower` (Xbox PDB).
        0xE8 fFogPower: f32,
        /// `fCurrentGameHour` (Xbox PDB).
        0xEC fCurrentGameHour: f32,
        /// `fCurrentWeatherPct` (Xbox PDB).
        0xF4 fCurrentWeatherPct: f32,
        /// `eMode` (Xbox PDB): `Sky::SKY_MODE`.
        0xF8 eMode: u32,
        /// `pSkySoundList` (Xbox PDB): `BSSimpleList<SkySound*>*`.
        0xFC pSkySoundList: Ptr,
        /// `fFlash` (Xbox PDB).
        0x100 fFlash: f32,
        /// `uiFlashTime` (Xbox PDB).
        0x104 uiFlashTime: u32,
        /// `uiLastMoonPhaseUpdate` (Xbox PDB).
        0x108 uiLastMoonPhaseUpdate: u32,
        /// `uiFlags` (Xbox PDB).
        0x118 uiFlags: u32,
        /// `fHighNoon` (Xbox PDB).
        0x12C fHighNoon: f32,
    }

    /// `SkySound` (Xbox PDB), 0x1c bytes: a `BSSoundHandle` (id, flag byte,
    /// state word) and the weather sound's description.
    pub struct SkySound: 0x1C {
        /// `SndHandle` (Xbox PDB): the sound id (`-1` = none).
        0x00 SndHandle: u32,
        /// `pWeather` (Xbox PDB).
        0x0C pWeather: Ptr,
        /// `eSoundType` (Xbox PDB).
        0x10 eSoundType: u32,
        /// `uiFormID` (Xbox PDB).
        0x14 uiFormID: u32,
        /// `uiData` (Xbox PDB).
        0x18 uiData: u32,
    }
}

/// Size of one `NiColor` (three `float`s) in `Sky::SkyColor`.
pub(crate) const SKY_COLOR_STRIDE: u32 = 12;

/// Copies one `NiColor` (three words) from `from` to `to`.
fn copy_color(e: &mut Engine, to: u32, from: u32) {
    for word in 0..3 {
        let value = e.mem.u32(from + 4 * word);
        e.mem.set_u32(to + 4 * word, value);
    }
}

/// `NiPointer<T>::get` on the smart pointer stored at `slot`.
fn ni_pointer_get(e: &mut Engine, slot: u32) -> Ptr {
    e.call(NI_POINTER_GET, &args![slot]).ptr()
}

/// `NiPointer<T>::operator=` on the smart pointer stored at `slot`.
fn ni_pointer_assign(e: &mut Engine, slot: u32, value: Ptr) {
    e.call(NI_POINTER_ASSIGN, &args![slot, value]);
}

/// `delete component`: calls the object's scalar deleting destructor
/// (virtual slot 0, flag 1) when it is not null.
fn delete_component(e: &mut Engine, component: Ptr) {
    if !component.is_null() {
        e.vcall(component.addr(), 0, &args![1u32]);
    }
}

/// Runs `body` between the scope timer's constructor and destructor (`line`
/// is the source line the game passes).
fn with_timer<R>(e: &mut Engine, line: u32, body: impl FnOnce(&mut Engine) -> R) -> R {
    e.with_stack(8, |e, guard| {
        e.call(
            TIMER_SCOPE_CONSTRUCT,
            &args![guard, 0x21u32, 1u32, SKY_SOURCE_FILE_NAME, line],
        );
        let result = body(e);
        e.call(TIMER_SCOPE_DESTROY, &args![guard]);
        result
    })
}

/// `new` of a class with a constructor: allocates `size` bytes with
/// `allocator`, runs `construct` on the block unless the allocation failed
/// and returns the object (null when the block is null).
fn construct_with(
    e: &mut Engine,
    allocator: u32,
    size: u32,
    construct: u32,
    extra_arguments: &[u32],
) -> Ptr {
    let block: Ptr = e.call(allocator, &args![size]).ptr();
    if block.is_null() {
        return Ptr::NULL;
    }
    let mut words = vec![block.addr()];
    words.extend_from_slice(extra_arguments);
    e.call(construct, &words).ptr()
}

// ---- functions ----

// Translated from 00639bf0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `SkySound::SkySound` (constructor by its body; the engine map has no
/// name): makes `SndHandle` a fresh `BSSoundHandle`, copies in the handle
/// passed by value (three words), stores the weather, the sound type and the
/// form id and clears `uiData`. The compiler's exception frame is not
/// translated.
#[allow(clippy::too_many_arguments)]
pub fn fn_00639bf0(
    e: &mut Engine,
    this: Ptr<SkySound>,
    handle_id: u32,
    handle_flag: u32,
    handle_state: u32,
    weather: Ptr,
    sound_type: u32,
    form_id: u32,
) -> Ptr<SkySound> {
    e.call(SOUND_HANDLE_CONSTRUCT, &args![this]);
    e.with_stack(12, |e, copy| {
        e.mem.set_u32(copy.addr(), handle_id);
        e.mem.set_u32(copy.addr() + 4, handle_flag);
        e.mem.set_u32(copy.addr() + 8, handle_state);
        e.call(SOUND_HANDLE_ASSIGN, &args![this, copy]);
        e.call(SOUND_HANDLE_DESTRUCT, &args![copy]);
    });
    e.set(this, SkySound::pWeather, weather);
    e.set(this, SkySound::eSoundType, sound_type);
    e.set(this, SkySound::uiFormID, form_id);
    let data = this.addr() + 0x18;
    e.call(MEMSET, &args![data, 0u32, 4u32]);
    this
}

// Translated from 00639c90 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed method of another class (its owner is not identified): when the
/// object's `00457fe0` value exceeds the limit `0042f5a0(0x80000000,
/// 00639ce0())`, calls `00639d00` with that value plus `extra` and returns
/// true; otherwise returns false.
pub fn fn_00639c90(e: &mut Engine, this: Ptr, extra: u32) -> bool {
    let size = e.call(0x0045_7fe0, &args![this]).u32();
    let limit_source = e.call(0x0063_9ce0, &args![this]).u32();
    let limit = e
        .call(0x0042_f5a0, &args![0x8000_0000u32, limit_source])
        .u32();
    if limit < size {
        e.call(0x0063_9d00, &args![this, size.wrapping_add(extra)]);
        true
    } else {
        false
    }
}

// Translated from 0063a080 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Sky::_scalar_deleting_destructor_` (Xbox PDB): runs the destructor and,
/// when bit 0 of `flags` is set, frees the object. Returns `this`.
pub fn sky_scalar_deleting_destructor(e: &mut Engine, this: Ptr<Sky>, flags: u32) -> Ptr<Sky> {
    fn_0063a0b0(e, this);
    if flags & 1 != 0 {
        e.call(OPERATOR_DELETE, &args![this]);
    }
    this
}

// Translated from 0063a0b0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Sky::~Sky` (by its body; the engine map names only the deleting
/// version): stores the vtable, deletes atmosphere, stars, clouds, sun, both
/// moons and the precipitation (virtual deleting destructors), releases the
/// moons root, takes the root out of its scene graph and releases it, deletes
/// every `SkySound` of the sound list, then the list. The compiler's
/// exception frame is not translated.
pub fn fn_0063a0b0(e: &mut Engine, this: Ptr<Sky>) {
    e.mem.set_u32(this.addr(), SKY_VTABLE);
    for field in [
        Sky::pAtmosphere,
        Sky::pStars,
        Sky::pClouds,
        Sky::pSun,
        Sky::pMasser,
        Sky::pSecunda,
        Sky::pPrecip,
    ] {
        let component = e.get(this, field);
        delete_component(e, component);
    }
    ni_pointer_assign(e, this.addr() + 8, Ptr::NULL);
    let root_slot = this.addr() + 4;
    if e.call(NI_POINTER_NOT_EQUAL, &args![root_slot, 0u32]).bool() {
        let root = ni_pointer_get(e, root_slot);
        if !e.call(ROOT_GET_SCENE, &args![root]).ptr::<()>().is_null() {
            let root = ni_pointer_get(e, root_slot);
            let scene: Ptr = e.call(ROOT_GET_SCENE, &args![root]).ptr();
            let root = ni_pointer_get(e, root_slot);
            e.vcall(scene.addr(), 0xe8, &args![root]);
        }
    }
    ni_pointer_assign(e, root_slot, Ptr::NULL);
    let mut node = e.get(this, Sky::pSkySoundList);
    while !node.is_null() {
        let item_slot: Ptr = e.call(LIST_NODE_ITEM, &args![node]).ptr();
        if e.mem.u32(item_slot.addr()) == 0 {
            break;
        }
        let item_slot: Ptr = e.call(LIST_NODE_ITEM, &args![node]).ptr();
        let item: Ptr<SkySound> = Ptr::new(e.mem.u32(item_slot.addr()));
        if !item.is_null() {
            fn_0063a3c0(e, item, 1);
        }
        node = e.call(LIST_NODE_NEXT, &args![node]).ptr();
    }
    let list = e.get(this, Sky::pSkySoundList);
    e.call(LIST_CLEAR, &args![list]);
    let list = e.get(this, Sky::pSkySoundList);
    if !list.is_null() {
        e.call(LIST_SCALAR_DELETING_DESTRUCTOR, &args![list, 1u32]);
    }
    e.call(NI_POINTER_DESTRUCTOR, &args![this.addr() + 8]);
    e.call(NI_POINTER_DESTRUCTOR, &args![root_slot]);
}

// Translated from 0063a3c0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Scalar deleting destructor of a `SkySound` (by its use in the sky's sound
/// list): runs the `BSSoundHandle` destructor body and, when bit 0 of
/// `flags` is set, frees the object. Returns `this`.
pub fn fn_0063a3c0(e: &mut Engine, this: Ptr<SkySound>, flags: u32) -> Ptr<SkySound> {
    e.call(SKY_SOUND_BASE_DESTRUCT, &args![this]);
    if flags & 1 != 0 {
        e.call(OPERATOR_DELETE, &args![this]);
    }
    this
}

// Translated from 0063a3f0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Sky::SetMode` (Xbox PDB): changes the sky mode (`Sky::SKY_MODE`: 0
/// `SM_NONE`, 1 `SM_INTERIOR`, 2 `SM_FAKEEXTERIOR`, 3 `SM_FULL`, 4
/// `SM_COUNT`). Entering mode 2 or 3 from 0, 1 or 4 calls `0063e2f0`, flags
/// the clouds (`0063a610`) and clears the visibility flag of the root and
/// the sun node. Entering mode 0 or 1 from 2, 3 or 4 unloads the textures,
/// sets that flag on both nodes, stops, releases and deletes every sound of
/// the list, clears the byte at `011ccb74`, stores the mode, resets the
/// precipitation and the flash. Every path ends by storing the mode and
/// calling `Sky::UpdateHDRValues`.
pub fn sky_set_mode(e: &mut Engine, this: Ptr<Sky>, mode: u32) {
    let current = e.get(this, Sky::eMode);
    if matches!(current, 0 | 1 | 4) && matches!(mode, 2 | 3) {
        e.call(SKY_ENTER_VISIBLE_MODE, &args![this]);
        let clouds = e.get(this, Sky::pClouds);
        if !clouds.is_null() {
            fn_0063a610(e, clouds);
        }
        set_root_and_sun_hidden(e, this, 0);
    } else if matches!(current, 2..=4) && matches!(mode, 0 | 1) {
        e.call(SKY_UNLOAD_ALL_TEXTURES, &args![this]);
        set_root_and_sun_hidden(e, this, 1);
        let list = e.get(this, Sky::pSkySoundList);
        while !e.call(LIST_IS_EMPTY, &args![list]).bool() {
            let slot: Ptr = e.call(LIST_NODE_ITEM, &args![list]).ptr();
            let sound = e.mem.u32(slot.addr());
            if e.call(SOUND_HANDLE_IS_PLAYING, &args![sound]).bool() {
                let slot: Ptr = e.call(LIST_NODE_ITEM, &args![list]).ptr();
                let sound = e.mem.u32(slot.addr());
                e.call(SOUND_HANDLE_STOP, &args![sound]);
            }
            let slot: Ptr = e.call(LIST_NODE_ITEM, &args![list]).ptr();
            let sound = e.mem.u32(slot.addr());
            e.call(SOUND_HANDLE_RELEASE, &args![sound]);
            let slot: Ptr = e.call(LIST_NODE_ITEM, &args![list]).ptr();
            let sound: Ptr<SkySound> = Ptr::new(e.mem.u32(slot.addr()));
            if !sound.is_null() {
                fn_0063a3c0(e, sound, 1);
            }
            e.call(LIST_REMOVE_HEAD, &args![list]);
        }
        e.set_global(SET_MODE_FLAG, 0u8);
        e.set(this, Sky::eMode, mode);
        let precipitation = e.get(this, Sky::pPrecip);
        if !precipitation.is_null() {
            e.call(PRECIPITATION_UPDATE, &args![precipitation, 0.0f32]);
        }
        e.set(this, Sky::fFlash, 0.0);
    }
    e.set(this, Sky::eMode, mode);
    e.call(SKY_UPDATE_HDR_VALUES, &args![this]);
}

/// Calls the visibility setter with `hidden` (0 or 1) on the root node and the
/// sun's node, as `SetMode` does on both of its branches.
fn set_root_and_sun_hidden(e: &mut Engine, this: Ptr<Sky>, hidden: u32) {
    let root_slot = this.addr() + 4;
    if !ni_pointer_get(e, root_slot).is_null() {
        let root = ni_pointer_get(e, root_slot);
        e.call(SET_NODE_HIDDEN, &args![root, hidden]);
    }
    let sun = e.get(this, Sky::pSun);
    if !sun.is_null() {
        let node: Ptr = e.call(SUN_GET_NODE, &args![sun]).ptr();
        e.call(SET_NODE_HIDDEN, &args![node, hidden]);
    }
}

// Translated from 0063a610 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Clouds` method (by its caller `Sky::SetMode`): sets the byte at `+0x5a`.
pub fn fn_0063a610(e: &mut Engine, this: Ptr) {
    e.mem.set_u8(this.addr() + 0x5a, 1);
}

// Translated from 0063a630 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Sky::Initialize` (Xbox PDB): builds the sky. Takes the root node from
/// `root`, or makes a `BSMultiBoundNode` when `root` is null, names it, then
/// replaces the atmosphere (initialized with the root and `atmosphere_argument`),
/// the stars, the sun, the moons root (attached to the root), the
/// precipitation (only when the setting at `011ccb8c` is on), and the clouds,
/// and gives the root the sky's property object. The compiler's exception
/// frame is not translated.
pub fn sky_initialize(e: &mut Engine, this: Ptr<Sky>, root: Ptr, atmosphere_argument: u32) {
    with_timer(e, 0x152, |e| {
        let root_slot = this.addr() + 4;
        if e.call(NI_POINTER_NOT_EQUAL, &args![root_slot, 0u32]).bool() {
            let current = ni_pointer_get(e, root_slot);
            if !e
                .call(ROOT_GET_SCENE, &args![current])
                .ptr::<()>()
                .is_null()
            {
                let current = ni_pointer_get(e, root_slot);
                let scene: Ptr = e.call(ROOT_GET_SCENE, &args![current]).ptr();
                let current = ni_pointer_get(e, root_slot);
                e.vcall(scene.addr(), 0xe8, &args![current]);
            }
            ni_pointer_assign(e, root_slot, Ptr::NULL);
        }
        if !root.is_null() {
            ni_pointer_assign(e, root_slot, root);
        } else {
            let node = construct_with(e, NI_OBJECT_NEW, 0xb4, MULTIBOUND_NODE_CONSTRUCT, &[]);
            ni_pointer_assign(e, root_slot, node);
            let current = ni_pointer_get(e, root_slot);
            e.call(ROOT_SET_FIRST_FLAG, &args![current, 1u32]);
            let current = ni_pointer_get(e, root_slot);
            e.call(ROOT_SET_SECOND_FLAG, &args![current, 1u32]);
        }
        let current = ni_pointer_get(e, root_slot);
        e.call(ROOT_SET_NAME, &args![current, ROOT_NAME_ARGUMENT]);

        // Atmosphere: initialized with the root and the caller's argument.
        let old = e.get(this, Sky::pAtmosphere);
        delete_component(e, old);
        let atmosphere = construct_with(e, OPERATOR_NEW, 0x1c, ATMOSPHERE_CONSTRUCT, &[]);
        e.set(this, Sky::pAtmosphere, atmosphere);
        let current = ni_pointer_get(e, root_slot);
        e.vcall(
            atmosphere.addr(),
            0x10,
            &args![current, atmosphere_argument],
        );

        // Stars, sun: initialized with the root.
        let old = e.get(this, Sky::pStars);
        delete_component(e, old);
        let stars = construct_with(e, OPERATOR_NEW, 0x10, STARS_CONSTRUCT, &[]);
        e.set(this, Sky::pStars, stars);
        let current = ni_pointer_get(e, root_slot);
        e.vcall(stars.addr(), 8, &args![current]);

        let old = e.get(this, Sky::pSun);
        delete_component(e, old);
        let sun = construct_with(e, OPERATOR_NEW, 0x2c, SUN_CONSTRUCT, &[]);
        e.set(this, Sky::pSun, sun);
        let current = ni_pointer_get(e, root_slot);
        e.vcall(sun.addr(), 8, &args![current]);

        // The moons' root node, attached to the root (slot 0xdc).
        let moons = construct_with(e, NI_OBJECT_NEW, 0xac, MOONS_ROOT_CONSTRUCT, &[0]);
        ni_pointer_assign(e, this.addr() + 8, moons);
        let current = ni_pointer_get(e, root_slot);
        let moons = ni_pointer_get(e, this.addr() + 8);
        e.vcall(current.addr(), 0xdc, &args![moons, 1u32]);

        // Precipitation exists only when the setting is on.
        let old = e.get(this, Sky::pPrecip);
        delete_component(e, old);
        let setting: Ptr = e
            .call(SETTING_BYTE_VALUE, &args![SETTING_PRECIPITATION])
            .ptr();
        if e.mem.u8(setting.addr()) != 0 {
            let precipitation = construct_with(e, OPERATOR_NEW, 0x18, PRECIPITATION_CONSTRUCT, &[]);
            e.set(this, Sky::pPrecip, precipitation);
            e.call(PRECIPITATION_INITIALIZE, &args![precipitation]);
        } else {
            e.set(this, Sky::pPrecip, Ptr::NULL);
        }

        let old = e.get(this, Sky::pClouds);
        delete_component(e, old);
        let clouds = construct_with(e, OPERATOR_NEW, 0x5c, CLOUDS_CONSTRUCT, &[]);
        e.set(this, Sky::pClouds, clouds);
        let current = ni_pointer_get(e, root_slot);
        e.vcall(clouds.addr(), 8, &args![current]);

        // The property object, held in a `NiPointer` on the stack, is
        // configured and attached to the root.
        e.with_stack(4, |e, holder| {
            let property = construct_with(e, NI_OBJECT_NEW, 0x1c, SKY_PROPERTY_CONSTRUCT, &[]);
            e.call(NI_POINTER_FROM_RAW, &args![holder, property]);
            for (setter, value) in [
                (SKY_PROPERTY_SET_A, 6u32),
                (SKY_PROPERTY_SET_B, 7),
                (SKY_PROPERTY_SET_C, 1),
                (SKY_PROPERTY_SET_D, 1),
            ] {
                let held = ni_pointer_get(e, holder.addr());
                e.call(setter, &args![held, value]);
            }
            let held = ni_pointer_get(e, holder.addr());
            let current = ni_pointer_get(e, root_slot);
            e.call(ROOT_ATTACH_PROPERTY, &args![current, held]);
            ni_pointer_assign(e, holder.addr(), Ptr::NULL);
            e.call(NI_POINTER_DESTRUCTOR, &args![holder]);
        });

        if e.global::<u8>(INITIAL_FLAG) != 0 {
            e.set_global(INITIAL_FLOAT_A, 1.0f32);
            e.set_global(INITIAL_FLOAT_B, 1.0f32);
        }
        let current = ni_pointer_get(e, root_slot);
        e.call(ROOT_UPDATE_PROPERTIES, &args![current]);
    });
}

// Translated from 0063ac70 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Sky::Update` (Xbox PDB): the per-frame update. Does nothing when the
/// player has no parent cell. Otherwise reads the hour from the calendar,
/// sets flag 4 when the camera is below the water, and in mode 1 without the
/// world flag runs the short update `0063b0a0`. Else it updates the weather
/// (`0063ca20`, `0063d1d0`), the colours (`0063b120`), the fog (`0063bce0`),
/// the wind (`0063c490`) and `0063d5e0`, updates atmosphere, stars, clouds
/// and sun, the moon phases when the day changed or the world flag asks for
/// it, both moons and the precipitation, fades the lightning flash, calls
/// `004dce50` on the colour at `+0x48` and clears flags 1 and 2.
pub fn sky_update(e: &mut Engine, this: Ptr<Sky>, delta: f32) {
    with_timer(e, 0x1d8, |e| {
        let player: u32 = e.global(PLAYER_CHARACTER);
        if e.call(PLAYER_PARENT_CELL, &args![player]).u32() == 0 {
            return;
        }
        let hour = e.call(CALENDAR_GET_HOUR, &args![CALENDAR]).f32();
        e.set(this, Sky::fCurrentGameHour, hour);

        // Flag 4: the camera is below the water of the player's cell.
        let player: u32 = e.global(PLAYER_CHARACTER);
        let mut flags = e.get(this, Sky::uiFlags);
        let mut underwater = false;
        if player != 0 && e.call(PLAYER_PARENT_CELL, &args![player]).u32() != 0 {
            let camera: Ptr = e.call(CAMERA_HOLDER_GET, &[]).ptr();
            let node: Ptr = e.call(CAMERA_NODE, &args![camera]).ptr();
            let position: Ptr = e.call(NODE_WORLD_POSITION, &args![node]).ptr();
            let water = e.call(RELEVANT_WATER_HEIGHT, &args![player]).f64();
            let camera_height = e.mem.f32(position.addr() + 8) as f64;
            underwater = camera_height < water;
        }
        flags = if underwater { flags | 4 } else { flags & !4 };
        e.set(this, Sky::uiFlags, flags);

        let world_flag: u32 = e.global(WORLD_STATE_POINTER);
        if e.get(this, Sky::eMode) == 1 && !e.call(WORLD_STATE_FLAG, &args![world_flag]).bool() {
            fn_0063b0a0(e, this, delta);
            return;
        }
        fn_0063ca20(e, this);
        let world_flag: u32 = e.global(WORLD_STATE_POINTER);
        if e.call(WORLD_STATE_FLAG, &args![world_flag]).bool() {
            let flags = e.get(this, Sky::uiFlags);
            e.set(this, Sky::uiFlags, flags | 1);
        }
        e.call(SKY_UPDATE_TRANSITION, &args![this]);
        fn_0063b120(e, this);
        fn_0063bce0(e, this);
        fn_0063c490(e, this);
        e.call(SKY_UPDATE_UNKNOWN, &args![this]);
        for field in [Sky::pAtmosphere, Sky::pStars, Sky::pClouds, Sky::pSun] {
            update_component(e, this, field, delta);
        }

        // Moon phases: when the day count moved on (and flag 0x20 is set),
        // or the world flag is set.
        let flags = e.get(this, Sky::uiFlags);
        let day_changed = flags & 0x20 != 0 && {
            let days = e.call(CALENDAR_GET_DAYS, &args![CALENDAR]).u32();
            e.get(this, Sky::uiLastMoonPhaseUpdate) < days
        };
        let world_flag: u32 = e.global(WORLD_STATE_POINTER);
        if day_changed || e.call(WORLD_STATE_FLAG, &args![world_flag]).bool() {
            let phase_applies = e.call(MOON_PHASE_APPLIES, &args![this]).bool();
            for field in [Sky::pMasser, Sky::pSecunda] {
                let moon = e.get(this, field);
                if !moon.is_null() && phase_applies && e.mem.u32(moon.addr() + 0x70) == 0 {
                    let world_flag: u32 = e.global(WORLD_STATE_POINTER);
                    let on = e.call(WORLD_STATE_FLAG, &args![world_flag]).bool();
                    e.mem.set_u32(moon.addr() + 0x70, on as u32 + 1);
                }
            }
            let days = e.call(CALENDAR_GET_DAYS, &args![CALENDAR]).u32();
            e.set(this, Sky::uiLastMoonPhaseUpdate, days);
        }
        for field in [Sky::pMasser, Sky::pSecunda] {
            update_component(e, this, field, delta);
        }
        let precipitation = e.get(this, Sky::pPrecip);
        if !precipitation.is_null() {
            e.call(PRECIPITATION_UPDATE, &args![precipitation, delta]);
        }

        // The lightning flash fades over its duration (a setting, seconds).
        let zero: f64 = e.global(DOUBLE_ZERO);
        if e.get(this, Sky::fFlash) as f64 > zero {
            let now = e.call(TICK_COUNT_GET, &args![TICK_COUNTER]).u32();
            let elapsed = now.wrapping_sub(e.get(this, Sky::uiFlashTime)) as f64;
            let duration: Ptr = e
                .call(SETTING_FLOAT_VALUE, &args![FLASH_DURATION_SETTING])
                .ptr();
            let duration = e.mem.f32(duration.addr()) as f64;
            let thousand: f64 = e.global(THOUSAND_DOUBLE);
            let remaining = 1.0 - elapsed / (duration * thousand);
            e.set(this, Sky::fFlash, remaining as f32);
        }
        if (e.get(this, Sky::fFlash) as f64) < zero {
            e.set(this, Sky::fFlash, 0.0);
        }
        e.call(FOG_COLOR_UPDATE, &args![this.addr() + 0x48]);
        let flags = e.get(this, Sky::uiFlags);
        e.set(this, Sky::uiFlags, flags & !3);
    });
}

/// Calls virtual slot `0xc` (the per-frame update, `Sky*` and `float`) of the
/// component stored in `field`, when there is one.
fn update_component(e: &mut Engine, this: Ptr<Sky>, field: Field<Sky, Ptr>, delta: f32) {
    let component = e.get(this, field);
    if !component.is_null() {
        e.vcall(component.addr(), 0xc, &args![this, delta]);
    }
}

// Translated from 0063b0a0 (decompiled, FalloutNV.exe 1.4.0.525)
/// The short update `Sky::Update` runs in mode 1 without the world flag:
/// colours (`0063b120`), fog (`0063bce0`), then the atmosphere and the sun
/// (virtual slot `0xc`), and `004dce50` on the colour at `+0x48`.
pub fn fn_0063b0a0(e: &mut Engine, this: Ptr<Sky>, delta: f32) {
    fn_0063b120(e, this);
    fn_0063bce0(e, this);
    for field in [Sky::pAtmosphere, Sky::pSun] {
        update_component(e, this, field, delta);
    }
    e.call(FOG_COLOR_UPDATE, &args![this.addr() + 0x48]);
}

// Translated from 0063b120 (decompiled, FalloutNV.exe 1.4.0.525)
/// Computes the ten sky colours (`SkyColor`, 12 bytes each from `+0x3c`) and
/// the sun specular colour at `+0xc0` for the current mode.
///
/// Mode 0 (`SM_NONE`): fixed colours. Mode 1 (`SM_INTERIOR`) with an
/// interior cell (`011dea10` is the cell holder, `005f36f0` returns its
/// interior cell): the cell's three lighting colours (`00544890`,
/// `005447b0`, `00544a30`). Otherwise the weather blend: `0063b630` picks
/// the two time-of-day sets and their weights, and every colour is mixed
/// by `0063c690` from the weather's colours (`0063bab0`; for the clouds
/// `0063bb70`), with lightning added for colours 3 and 4 when there is no
/// last weather. The colours are then copied down the chain the game keeps
/// them in (`+0xc0`, `+0x9c`, `+0x78`, `+0x3c`, `+0x48`, `+0x90`).
pub fn fn_0063b120(e: &mut Engine, this: Ptr<Sky>) {
    let mode = e.get(this, Sky::eMode);
    let clouds = e.get(this, Sky::pClouds);
    let sky = this.addr();
    let interior_cell = |e: &mut Engine| -> Ptr {
        let holder: u32 = e.global(INTERIOR_CELL_HOLDER);
        if holder == 0 {
            return Ptr::NULL;
        }
        e.call(GET_INTERIOR_CELL, &args![holder]).ptr()
    };
    if mode == 0 {
        let temp = e.mem.alloc(12);
        let (a, b): (f32, f32) = (e.global(0x0101_ffc0), e.global(0x0101_9de0));
        e.call(NI_COLOR_CONSTRUCT, &args![temp, a, b, b]);
        copy_color(e, sky + 0x6c, temp);
        let (a, c): (f32, f32) = (e.global(0x0102_1f44), e.global(0x0104_ee30));
        e.call(NI_COLOR_CONSTRUCT, &args![temp, a, a, c]);
        copy_color(e, sky + 0x60, temp);
        if !clouds.is_null() {
            let mut layer = 0;
            while layer < e.call(CLOUDS_LAYER_COUNT, &args![clouds]).i32() {
                let value: f32 = e.global(0x0101_e2bc);
                e.call(NI_COLOR_CONSTRUCT, &args![temp, value, value, value]);
                fn_0063b600(e, clouds, layer, Ptr::new(temp));
                layer += 1;
            }
        }
        e.mem.free(temp);
        copy_color(e, sky + 0xc0, 0x011f_4998);
        copy_color(e, sky + 0x9c, sky + 0xc0);
        copy_color(e, sky + 0x78, sky + 0x9c);
        copy_color(e, sky + 0x3c, sky + 0x78);
        copy_color(e, sky + 0x48, sky + 0x3c);
        copy_color(e, sky + 0x90, sky + 0x48);
    } else if mode == 1 && !interior_cell(e).is_null() {
        let cell = interior_cell(e);
        e.call(CELL_COLOR_FIRST, &args![cell, sky + 0x6c]);
        let cell = interior_cell(e);
        e.call(CELL_COLOR_SECOND, &args![cell, sky + 0x60]);
        let cell = interior_cell(e);
        e.call(CELL_COLOR_THIRD, &args![cell, sky + 0x48]);
        copy_color(e, sky + 0xc0, sky + 0x48);
        copy_color(e, sky + 0x9c, sky + 0xc0);
        copy_color(e, sky + 0x78, sky + 0x9c);
        copy_color(e, sky + 0x3c, sky + 0x78);
        copy_color(e, sky + 0x90, sky + 0x3c);
        if !clouds.is_null() {
            let mut layer = 0;
            while layer < e.call(CLOUDS_LAYER_COUNT, &args![clouds]).i32() {
                fn_0063b600(e, clouds, layer, Ptr::new(sky + 0x48));
                layer += 1;
            }
        }
    } else {
        let weather = e.get(this, Sky::pCurrentWeather);
        let last_weather = e.get(this, Sky::pLastWeather);
        let percent = e.get(this, Sky::fCurrentWeatherPct);
        let flash = e.get(this, Sky::fFlash);
        let sample = e.mem.alloc(0x20);
        let first_index = e.mem.alloc(4);
        let second_index = e.mem.alloc(4);
        fn_0063b630(
            e,
            this,
            Ptr::new(sample),
            weather,
            percent.to_bits(),
            Ptr::new(first_index),
            Ptr::new(second_index),
        );
        for color in 0..10u32 {
            fn_0063bab0(
                e,
                this,
                Ptr::new(sample),
                weather,
                last_weather,
                color as i32,
                Ptr::new(first_index),
                Ptr::new(second_index),
            );
            let mut lightning = 0.0f32;
            if last_weather.is_null() {
                if color == 4 {
                    lightning = lightning_scaled(e, flash, LIGHTNING_SCALE_COLOR_4);
                } else if color == 3 {
                    lightning = lightning_scaled(e, flash, LIGHTNING_SCALE_COLOR_3);
                }
            }
            fn_0063c690(
                e,
                this,
                Ptr::new(sky + 0x3c + color * SKY_COLOR_STRIDE),
                Ptr::new(sample),
                lightning,
            );
        }
        if !clouds.is_null() {
            let temp = e.mem.alloc(12);
            let mut layer = 0;
            while layer < e.call(CLOUDS_LAYER_COUNT, &args![clouds]).i32() {
                fn_0063bb70(
                    e,
                    this,
                    Ptr::new(sample),
                    weather,
                    last_weather,
                    layer,
                    Ptr::new(first_index),
                    Ptr::new(second_index),
                );
                let mut lightning = 0.0f32;
                if last_weather.is_null() && layer == 0 {
                    lightning = flash;
                }
                e.call(NI_COLOR_CONSTRUCT, &args![temp, 0.0f32, 0.0f32, 0.0f32]);
                fn_0063c690(e, this, Ptr::new(temp), Ptr::new(sample), lightning);
                fn_0063b600(e, clouds, layer, Ptr::new(temp));
                layer += 1;
            }
            e.mem.free(temp);
        }
        e.mem.free(sample);
        e.mem.free(first_index);
        e.mem.free(second_index);
        copy_color(e, sky + 0xc0, sky + 0x6c);
    }
}

/// `flash` times the `float` setting object at `setting` (the lightning's
/// contribution to colour 3 or 4).
fn lightning_scaled(e: &mut Engine, flash: f32, setting: u32) -> f32 {
    let value: Ptr = e.call(SETTING_FLOAT_VALUE, &args![setting]).ptr();
    (flash as f64 * e.mem.f32(value.addr()) as f64) as f32
}

// Translated from 0063b600 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Clouds` method (by its caller `Sky::fn_0063b120`): copies the colour at
/// `color` into the layer's slot at `+0x28 + 12 * index`.
pub fn fn_0063b600(e: &mut Engine, this: Ptr, index: i32, color: Ptr) {
    let slot = this
        .addr()
        .wrapping_add(0x28)
        .wrapping_add((index as u32).wrapping_mul(SKY_COLOR_STRIDE));
    copy_color(e, slot, color.addr());
}

// Translated from 0063b630 (decompiled, FalloutNV.exe 1.4.0.525)
/// Picks the two time-of-day colour sets for the current hour and their
/// weights. `sunrise_begin`, `sunrise_end`, `sunset_begin` and `sunset_end`
/// (`0063b9b0`, `00595f50`, `00595fc0`, `0063ba30`) divide the day into
/// night (index 3), sunrise (0 to 1), day (1) and sunset (4 to 2); `high_noon`
/// (`+0x12c`) splits the day. Writes the indices into `first_index_out` and
/// `second_index_out`, and into `sample` (`+0x10` to `+0x1c`) the four
/// weights: the blend towards the second set and its complement, each scaled
/// by the weather percentage (`+0xf4`) and its complement. Does nothing when
/// `weather` is null. An hour outside every range logs the master file data
/// error `"MASTERFILE: Data error detected--Transition times stored in
/// climate data are invalid."` (`005b5e40`) and uses set 1 for both. The
/// third word is passed but not read.
pub fn fn_0063b630(
    e: &mut Engine,
    this: Ptr<Sky>,
    sample: Ptr,
    weather: Ptr,
    _unused_2: u32,
    first_index_out: Ptr,
    second_index_out: Ptr,
) {
    if weather.is_null() {
        return;
    }
    let sunrise_begin = fn_0063b9b0(e, this);
    let sunrise_end = e.call(SKY_SUNRISE_END, &args![this]).f32();
    let sunset_begin = e.call(SKY_SUNSET_BEGIN, &args![this]).f32();
    let sunset_end = fn_0063ba30(e, this);
    let hour = e.get(this, Sky::fCurrentGameHour);
    let high_noon = e.get(this, Sky::fHighNoon);
    let (a, b, c, d) = (sunrise_begin, sunrise_end, sunset_begin, sunset_end);
    let first = first_index_out.addr();
    let second = second_index_out.addr();
    let blend: f32;
    if a < hour && hour < b {
        e.mem.set_u32(first, 0);
        let half = ((b as f64 - a as f64) * e.global::<f64>(DOUBLE_HALF)) as f32;
        let middle = (a as f64 + half as f64) as f32;
        if middle > hour {
            blend = (1.0 - (middle as f64 - hour as f64) / half as f64) as f32;
            e.mem.set_u32(second, 3);
        } else {
            blend = (1.0 - (hour as f64 - middle as f64) / half as f64) as f32;
            e.mem.set_u32(second, 1);
        }
    } else if b < hour && hour <= high_noon {
        e.mem.set_u32(second, 1);
        e.mem.set_u32(first, 4);
        let span = (high_noon as f64 - b as f64) as f32;
        blend = (1.0 - (high_noon as f64 - hour as f64) / span as f64) as f32;
    } else if high_noon <= hour && hour <= c {
        e.mem.set_u32(second, 4);
        e.mem.set_u32(first, 1);
        let span = (c as f64 - high_noon as f64) as f32;
        blend = (1.0 - (c as f64 - hour as f64) / span as f64) as f32;
    } else if c < hour && hour < d {
        e.mem.set_u32(first, 2);
        let half = ((d as f64 - c as f64) * e.global::<f64>(DOUBLE_HALF)) as f32;
        let middle = (c as f64 + half as f64) as f32;
        if middle > hour {
            blend = (1.0 - (middle as f64 - hour as f64) / half as f64) as f32;
            e.mem.set_u32(second, 1);
        } else {
            blend = (1.0 - (hour as f64 - middle as f64) / half as f64) as f32;
            e.mem.set_u32(second, 3);
        }
    } else if d <= hour || a >= hour {
        // Night.
        e.mem.set_u32(second, 3);
        e.mem.set_u32(first, 3);
        blend = 1.0;
    } else {
        e.call(LOG_MASTERFILE_ERROR, &args![TRANSITION_TIMES_MESSAGE]);
        e.mem.set_u32(second, 1);
        e.mem.set_u32(first, 1);
        blend = 1.0;
    }
    let percent = e.get(this, Sky::fCurrentWeatherPct);
    let complement = (1.0 - blend as f64) as f32;
    e.mem.set_f32(sample.addr() + 0x10, blend);
    e.mem.set_f32(sample.addr() + 0x14, complement);
    e.mem.set_f32(sample.addr() + 0x18, blend);
    e.mem.set_f32(sample.addr() + 0x1c, complement);
    let first_scaled = (blend as f64 * percent as f64) as f32;
    let second_scaled = (complement as f64 * percent as f64) as f32;
    e.mem.set_f32(sample.addr() + 0x10, first_scaled);
    e.mem.set_f32(sample.addr() + 0x14, second_scaled);
    let rest = 1.0 - percent as f64;
    let third = (rest * e.mem.f32(sample.addr() + 0x18) as f64) as f32;
    e.mem.set_f32(sample.addr() + 0x18, third);
    let fourth = (rest * e.mem.f32(sample.addr() + 0x1c) as f64) as f32;
    e.mem.set_f32(sample.addr() + 0x1c, fourth);
}

// Translated from 0063b9b0 (decompiled, FalloutNV.exe 1.4.0.525)
/// The sunrise begin hour as the sky uses it: when flag `0x1000` is set,
/// recomputes it as `max(0, Sky::GetSunriseBegin (00595ea0) - fTimeTransition)`
/// into the cache at `011cccfc` and clears the flag; returns the cache.
pub fn fn_0063b9b0(e: &mut Engine, this: Ptr<Sky>) -> f32 {
    let flags = e.get(this, Sky::uiFlags);
    if flags & 0x1000 != 0 {
        let begin = e.call(SKY_SUNRISE_BEGIN, &args![this]).f64();
        let setting: Ptr = e.call(SETTING_FLOAT_VALUE, &args![TIME_SETTING]).ptr();
        let margin = e.mem.f32(setting.addr()) as f64;
        let value = (begin - margin) as f32;
        let clamped = e.call(MAX_FLOAT, &args![0.0f32, value]).f32();
        e.set_global(SUNRISE_BEGIN_CACHE, clamped);
        let flags = e.get(this, Sky::uiFlags);
        e.set(this, Sky::uiFlags, flags & 0xffff_efff);
    }
    e.global(SUNRISE_BEGIN_CACHE)
}

// Translated from 0063ba30 (decompiled, FalloutNV.exe 1.4.0.525)
/// The sunset end hour as the sky uses it: when flag `0x2000` is set,
/// recomputes it as `min(Sky sunset end (00596030) + fTimeTransition,
/// [0104ede4])` into the cache at `011ccd00` and clears the flag; returns the
/// cache.
pub fn fn_0063ba30(e: &mut Engine, this: Ptr<Sky>) -> f32 {
    let flags = e.get(this, Sky::uiFlags);
    if flags & 0x2000 != 0 {
        let end = e.call(SKY_SUNSET_END, &args![this]).f64();
        let setting: Ptr = e.call(SETTING_FLOAT_VALUE, &args![TIME_SETTING]).ptr();
        let margin = e.mem.f32(setting.addr()) as f64;
        let value = (margin + end) as f32;
        let upper: f32 = e.global(0x0104_ede4);
        let clamped = e.call(MIN_FLOAT, &args![upper, value]).f32();
        e.set_global(SUNSET_END_CACHE, clamped);
        let flags = e.get(this, Sky::uiFlags);
        e.set(this, Sky::uiFlags, flags & 0xffff_dfff);
    }
    e.global(SUNSET_END_CACHE)
}

// Translated from 0063bab0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Reads four colours of the weathers `weather_a` and `weather_b` for the
/// time-of-day `index`: `out[0]`/`out[1]` from `weather_a` at the indices
/// stored at `first_index`/`second_index`, `out[2]`/`out[3]` the same from
/// `weather_b` (0 when there is none). Does nothing when `weather_a` is null.
/// The `__thiscall` receiver is not read.
#[allow(clippy::too_many_arguments)]
pub fn fn_0063bab0(
    e: &mut Engine,
    _unused_this: Ptr<Sky>,
    out: Ptr,
    weather_a: Ptr,
    weather_b: Ptr,
    index: i32,
    first_index: Ptr,
    second_index: Ptr,
) {
    if weather_a.is_null() {
        return;
    }
    let first = e.mem.i32(first_index.addr());
    let value = fn_0063bb40(e, weather_a, index, first);
    e.mem.set_u32(out.addr(), value);
    let second = e.mem.i32(second_index.addr());
    let value = fn_0063bb40(e, weather_a, index, second);
    e.mem.set_u32(out.addr() + 4, value);
    if weather_b.is_null() {
        e.mem.set_u32(out.addr() + 0xc, 0);
        e.mem.set_u32(out.addr() + 8, 0);
    } else {
        let first = e.mem.i32(first_index.addr());
        let value = fn_0063bb40(e, weather_b, index, first);
        e.mem.set_u32(out.addr() + 8, value);
        let second = e.mem.i32(second_index.addr());
        let value = fn_0063bb40(e, weather_b, index, second);
        e.mem.set_u32(out.addr() + 0xc, value);
    }
}

// Translated from 0063bb40 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESWeather` colour accessor (by its caller): the packed colour at
/// `weather + 0x108 + 0x18 * index + 4 * time_of_day`.
pub fn fn_0063bb40(e: &mut Engine, this: Ptr, index: i32, time_of_day: i32) -> u32 {
    let row = this
        .addr()
        .wrapping_add((index as u32).wrapping_mul(0x18))
        .wrapping_add(0x108);
    e.mem
        .u32(row.wrapping_add((time_of_day as u32).wrapping_mul(4)))
}

// Translated from 0063bb70 (decompiled, FalloutNV.exe 1.4.0.525)
/// Like [`fn_0063bab0`] for the cloud layers (colours through
/// [`fn_0063bc60`]); when there is a `weather_b` a colour that is 0 is
/// replaced by the other weather's colour at the same position. The
/// `__thiscall` receiver is not read.
#[allow(clippy::too_many_arguments)]
pub fn fn_0063bb70(
    e: &mut Engine,
    _unused_this: Ptr<Sky>,
    out: Ptr,
    weather_a: Ptr,
    weather_b: Ptr,
    layer: i32,
    first_index: Ptr,
    second_index: Ptr,
) {
    if weather_a.is_null() {
        return;
    }
    let first = e.mem.i32(first_index.addr());
    let value = fn_0063bc60(e, weather_a, layer, first);
    e.mem.set_u32(out.addr(), value);
    let second = e.mem.i32(second_index.addr());
    let value = fn_0063bc60(e, weather_a, layer, second);
    e.mem.set_u32(out.addr() + 4, value);
    if weather_b.is_null() {
        e.mem.set_u32(out.addr() + 0xc, 0);
        e.mem.set_u32(out.addr() + 8, 0);
        return;
    }
    let first = e.mem.i32(first_index.addr());
    let value = fn_0063bc60(e, weather_b, layer, first);
    e.mem.set_u32(out.addr() + 8, value);
    let second = e.mem.i32(second_index.addr());
    let value = fn_0063bc60(e, weather_b, layer, second);
    e.mem.set_u32(out.addr() + 0xc, value);
    for (target, other) in [(0u32, 8u32), (4, 0xc), (8, 0), (0xc, 4)] {
        if e.mem.u32(out.addr() + target) == 0 {
            let value = e.mem.u32(out.addr() + other);
            e.mem.set_u32(out.addr() + target, value);
        }
    }
}

// Translated from 0063bc60 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESWeather` cloud colour accessor (by its caller): the packed colour of
/// cloud layer `layer` for `time_of_day` (the layer count is at `+0x368`,
/// the colours at `+0x68 + 0x18 * layer`; a layer past the count uses layer
/// 0). Without layers, or before the colours are known, the built-in
/// colours: `0x9b9b9b` for time 1, `0x1c140f` for 3, `0x5b4b46` for 0 and
/// `0x6c5953` otherwise.
pub fn fn_0063bc60(e: &mut Engine, this: Ptr, layer: i32, time_of_day: i32) -> u32 {
    let mut color = match time_of_day {
        1 => 0x9b9b9b,
        3 => 0x1c140f,
        0 => 0x5b4b46,
        _ => 0x6c5953,
    };
    let count = e.mem.i32(this.addr() + 0x368);
    if count > 0 {
        let layer = if layer >= count { 0 } else { layer };
        let row = this
            .addr()
            .wrapping_add((layer as u32).wrapping_mul(0x18))
            .wrapping_add(0x68);
        color = e
            .mem
            .u32(row.wrapping_add((time_of_day as u32).wrapping_mul(4)));
    }
    color
}

// Translated from 0063bce0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Computes the fog near and far distances (`+0xd4`, `+0xd8`) and the fog
/// power (`+0xe8`).
///
/// When one of the underwater flags (`004e2180`, `004e3c40`) is set and the
/// camera is below the water (or the second flag is set) and a water volume
/// is known (`0063c470`), the fog comes from the water volume
/// (`004e3d00`, `00507b20`), the water colour feeds `0063c690` into the colour
/// at `+0x48`, and in modes 2 and 3 the sky nodes are hidden once the camera
/// is more than half way below the surface. Otherwise: with a current weather
/// in mode 2 or 3, the fog is the hour-weighted mix of the weather's fog
/// values (`0063c420`; plus the last weather's, scaled by the weather
/// percentage); in mode 1 with an interior cell the cell's fog (`00544b10`,
/// `00544ab0`, `00544b70`, with the far distance clamped to 163840.0); else
/// the default `[010317c4]` for both distances and power 1.0. Unless
/// `0063c460` is set the far distance is then pulled to the camera's far
/// plane (virtual slot `0x100` of the camera, `fFogSetting`) and the near
/// distance rescaled with it.
pub fn fn_0063bce0(e: &mut Engine, this: Ptr<Sky>) {
    let water = fn_0063c470(e);
    if e.call(FLAG_UNDERWATER_A, &[]).bool() || e.call(FLAG_UNDERWATER_B, &[]).bool() {
        let camera: Ptr = e.call(CAMERA_HOLDER_GET, &[]).ptr();
        let node: Ptr = e.call(CAMERA_NODE, &args![camera]).ptr();
        let position: Ptr = e.call(NODE_WORLD_POSITION, &args![node]).ptr();
        let camera_height = e.mem.f32(position.addr() + 8);
        if water != 0 {
            let mut below = true;
            if !e.call(FLAG_UNDERWATER_B, &[]).bool() {
                let player: u32 = e.global(PLAYER_CHARACTER);
                let cell: Ptr = e.call(PLAYER_PARENT_CELL, &args![player]).ptr();
                let surface = e.call(CELL_WATER_HEIGHT, &args![cell]).f64();
                below = (camera_height as f64) < surface;
            }
            if below {
                fog_from_water(e, this, water, camera_height);
                return;
            }
        }
    }
    let weather = e.get(this, Sky::pCurrentWeather);
    let mode = e.get(this, Sky::eMode);
    if !weather.is_null() && (mode == 3 || mode == 2) {
        fog_from_weather(e, this, weather);
    } else {
        let holder: u32 = e.global(INTERIOR_CELL_HOLDER);
        if mode == 1
            && holder != 0
            && !e
                .call(GET_INTERIOR_CELL, &args![holder])
                .ptr::<()>()
                .is_null()
        {
            let far_default: f32 = e.global(DEFAULT_FOG_DISTANCE);
            let cell: Ptr = e.call(GET_INTERIOR_CELL, &args![holder]).ptr();
            let mut far = e.call(CELL_FOG_FAR, &args![cell]).f32();
            let zero: f64 = e.global(DOUBLE_ZERO);
            let limit: f64 = e.global(FOG_FAR_LIMIT);
            if far as f64 <= zero || far as f64 > limit {
                far = far_default;
            }
            let cell: Ptr = e.call(GET_INTERIOR_CELL, &args![holder]).ptr();
            let mut near = e.call(CELL_FOG_NEAR, &args![cell]).f32();
            if near as f64 <= zero || far < near {
                let factor: f64 = e.global(FOG_NEAR_FACTOR);
                near = (far as f64 * factor) as f32;
            }
            e.set(this, Sky::fFogNear, near);
            e.set(this, Sky::fFogFar, far);
            let cell: Ptr = e.call(GET_INTERIOR_CELL, &args![holder]).ptr();
            let power = e.call(CELL_FOG_POWER, &args![cell]).f32();
            e.set(this, Sky::fFogPower, power);
        } else {
            let default: f32 = e.global(DEFAULT_FOG_DISTANCE);
            e.set(this, Sky::fFogNear, default);
            e.set(this, Sky::fFogFar, default);
            e.set(this, Sky::fFogPower, 1.0);
        }
    }
    if !fn_0063c460(e) {
        let camera: Ptr = e.call(CAMERA_HOLDER_GET, &[]).ptr();
        let camera_far = e.vcall(camera.addr(), 0x100, &[]).f32();
        let setting: Ptr = e.call(SETTING_FLOAT_VALUE, &args![FOG_SETTING]).ptr();
        let near_limit = e.mem.f32(setting.addr());
        let far = e.get(this, Sky::fFogFar);
        let near = e.get(this, Sky::fFogNear);
        let ratio = ((far as f64 - camera_far as f64) / (far as f64 - near_limit as f64)) as f32;
        let pulled = (near as f64 - (near as f64 - near_limit as f64) * ratio as f64) as f32;
        e.set(this, Sky::fFogNear, pulled);
        e.set(this, Sky::fFogFar, camera_far);
        let result = e.call(MIN_FLOAT, &args![camera_far, pulled]).f32();
        e.set(this, Sky::fFogNear, result);
    }
}

/// The underwater branch of [`fn_0063bce0`]: the fog of the water volume at
/// `water`, the water colour fed to `0063c690`, and (modes 2 and 3) the sky
/// nodes hidden when the camera is more than half way below the surface.
fn fog_from_water(e: &mut Engine, this: Ptr<Sky>, water: u32, camera_height: f32) {
    let near = e.call(WATER_FOG_NEAR, &args![water]).f32();
    e.set(this, Sky::fFogNear, near);
    let far = e.call(WATER_FOG_FAR, &args![water]).f32();
    e.set(this, Sky::fFogFar, far);
    e.set(this, Sky::fFogPower, 1.0);
    let mut surface = fn_0063c480(e);
    let player: u32 = e.global(PLAYER_CHARACTER);
    let cell: Ptr = e.call(PLAYER_PARENT_CELL, &args![player]).ptr();
    let cell_surface = e.call(CELL_WATER_HEIGHT, &args![cell]).f64();
    if (camera_height as f64) < cell_surface {
        let cell: Ptr = e.call(PLAYER_PARENT_CELL, &args![player]).ptr();
        surface = e.call(CELL_WATER_HEIGHT, &args![cell]).f32();
    }
    let depth_scale: f32 = e.global(WATER_DEPTH_SCALE);
    let mut depth = ((surface as f64 - camera_height as f64) * depth_scale as f64) as f32;
    let limit: f64 = e.global(DOUBLE_ONE);
    if depth as f64 > limit {
        depth = 1.0;
    }
    e.with_stack(0x20, |e, sample| {
        let first = e.call(WATER_COLOR_A, &args![water]).u32();
        let second = e.call(WATER_COLOR_B, &args![water]).u32();
        e.mem.set_u32(sample.addr(), first);
        e.mem.set_u32(sample.addr() + 4, second);
        e.mem.set_u32(sample.addr() + 8, 0);
        e.mem.set_u32(sample.addr() + 12, 0);
        e.mem
            .set_f32(sample.addr() + 0x10, (1.0 - depth as f64) as f32);
        e.mem.set_f32(sample.addr() + 0x14, depth);
        e.mem.set_f32(sample.addr() + 0x18, 0.0);
        e.mem.set_f32(sample.addr() + 0x1c, 0.0);
        fn_0063c690(e, this, Ptr::new(this.addr() + 0x48), sample, 0.0);
    });
    let mode = e.get(this, Sky::eMode);
    if mode == 3 || mode == 2 {
        let half: f64 = e.global(DOUBLE_HALF);
        let hide = depth as f64 > half;
        let clouds = ni_pointer_get_clouds(e, this);
        let layers = e.call(CLOUDS_LAYER_COUNT, &args![clouds]).i32();
        for layer in 0..layers {
            let clouds = ni_pointer_get_clouds(e, this);
            let node = fn_0063c440(e, clouds, layer);
            e.call(SET_NODE_HIDDEN, &args![node, hide as u32]);
        }
        let atmosphere: Ptr = e.call(SKY_GET_ATMOSPHERE, &args![this]).ptr();
        let node: Ptr = e.call(HOLDER_NODE, &args![atmosphere]).ptr();
        e.call(SET_NODE_HIDDEN, &args![node, hide as u32]);
        let sun: Ptr = e.call(SKY_GET_SUN, &args![this]).ptr();
        let node: Ptr = e.call(HOLDER_NODE, &args![sun]).ptr();
        e.call(SET_NODE_HIDDEN, &args![node, hide as u32]);
        let sun: Ptr = e.call(SKY_GET_SUN, &args![this]).ptr();
        let node: Ptr = e.call(SUN_GET_NODE, &args![sun]).ptr();
        e.call(SET_NODE_HIDDEN, &args![node, hide as u32]);
        let root = ni_pointer_get(e, this.addr() + 4);
        e.call(SET_NODE_HIDDEN, &args![root, hide as u32]);
    }
}

/// `Sky::GetClouds` (`0055b980`, returns `+0x2c`).
fn ni_pointer_get_clouds(e: &mut Engine, this: Ptr<Sky>) -> Ptr {
    e.call(SKY_GET_CLOUDS, &args![this]).ptr()
}

/// The weather branch of [`fn_0063bce0`]: the fog values mixed from the
/// weather's fog table by the hour, and by the last weather and the weather
/// percentage.
fn fog_from_weather(e: &mut Engine, this: Ptr<Sky>, weather: Ptr) {
    let sunrise_begin = fn_0063b9b0(e, this);
    let sunrise_end = e.call(SKY_SUNRISE_END, &args![this]).f32();
    let sunset_begin = e.call(SKY_SUNSET_BEGIN, &args![this]).f32();
    let sunset_end = fn_0063ba30(e, this);
    let hour = e.get(this, Sky::fCurrentGameHour);
    let blend: f32 = if sunrise_begin < hour && hour < sunrise_end {
        ((hour as f64 - sunrise_begin as f64) / (sunrise_end as f64 - sunrise_begin as f64)) as f32
    } else if sunrise_end <= hour && hour <= sunset_begin {
        1.0
    } else if sunset_begin < hour && hour < sunset_end {
        ((sunset_end as f64 - hour as f64) / (sunset_end as f64 - sunset_begin as f64)) as f32
    } else {
        0.0
    };
    let complement = (1.0 - blend as f64) as f32;
    let mix = |e: &mut Engine, from: Ptr, day: i32, night: i32| -> f64 {
        let first = fn_0063c420(e, from, day);
        let first = first as f64 * blend as f64;
        let second = fn_0063c420(e, from, night);
        second as f64 * complement as f64 + first
    };
    let fields = [
        (Sky::fFogNear, 0, 2),
        (Sky::fFogFar, 1, 3),
        (Sky::fFogPower, 4, 5),
    ];
    for (field, day, night) in fields {
        let value = mix(e, weather, day, night) as f32;
        e.set(this, field, value);
    }
    let last_weather = e.get(this, Sky::pLastWeather);
    if !last_weather.is_null() {
        for (field, day, night) in fields {
            let percent = e.get(this, Sky::fCurrentWeatherPct);
            let current = e.get(this, field);
            let scaled = (current as f64 * percent as f64) as f32;
            e.set(this, field, scaled);
            let rest = 1.0 - e.get(this, Sky::fCurrentWeatherPct) as f64;
            let last_value = mix(e, last_weather, day, night);
            let result = last_value * rest + e.get(this, field) as f64;
            e.set(this, field, result as f32);
        }
    }
}

// Translated from 0063c420 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TESWeather` fog accessor (by its caller): the `float` at
/// `weather + 0xf0 + 4 * index`.
pub fn fn_0063c420(e: &mut Engine, this: Ptr, index: i32) -> f32 {
    e.mem.f32(
        this.addr()
            .wrapping_add((index as u32).wrapping_mul(4))
            .wrapping_add(0xf0),
    )
}

// Translated from 0063c440 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Clouds` accessor (by its caller): the node pointer stored at
/// `clouds + 8 + 4 * index` (`NiPointer::get`).
pub fn fn_0063c440(e: &mut Engine, this: Ptr, index: i32) -> Ptr {
    let slot = this
        .addr()
        .wrapping_add((index as u32).wrapping_mul(4))
        .wrapping_add(8);
    ni_pointer_get(e, slot)
}

// Translated from 0063c460 (decompiled, FalloutNV.exe 1.4.0.525)
/// Returns the byte at `0119f188`.
pub fn fn_0063c460(e: &mut Engine) -> bool {
    e.global(0x0119_f188)
}

// Translated from 0063c470 (decompiled, FalloutNV.exe 1.4.0.525)
/// Returns the word at `011c7a3c` (the water volume the fog code reads).
pub fn fn_0063c470(e: &mut Engine) -> u32 {
    e.global(0x011c_7a3c)
}

// Translated from 0063c480 (decompiled, FalloutNV.exe 1.4.0.525)
/// Returns the `float` at `011c7a5c`.
pub fn fn_0063c480(e: &mut Engine) -> f32 {
    e.global(0x011c_7a5c)
}

// Translated from 0063c490 (decompiled, FalloutNV.exe 1.4.0.525)
/// The wind: in modes 2 and 3 with a weather, `fWindSpeed` is the weather's
/// wind speed (`004ed230`, byte 0 of its table scaled to 0..1), mixed with
/// the last weather's by the weather percentage; then it publishes the speed
/// (`0063c660`) and two values from the time of day (`0063c640` gets the
/// words at `011a9b50`; `0063c670` gets 15 times the lookup sine of the
/// phase and 15 times the sine of 0.7 of it). The compiler's no-op call of
/// `006815c0` on a local is left out.
pub fn fn_0063c490(e: &mut Engine, this: Ptr<Sky>) {
    let weather = e.get(this, Sky::pCurrentWeather);
    let mode = e.get(this, Sky::eMode);
    if weather.is_null() || !(mode == 3 || mode == 2) {
        return;
    }
    let speed = e
        .call(WEATHER_BYTE_FRACTION, &args![weather, 0u32, 1.0f32, 0.0f32])
        .f32();
    e.set(this, Sky::fWindSpeed, speed);
    let last_weather = e.get(this, Sky::pLastWeather);
    if !last_weather.is_null() {
        let percent = e.get(this, Sky::fCurrentWeatherPct);
        let scaled = (speed as f64 * percent as f64) as f32;
        e.set(this, Sky::fWindSpeed, scaled);
        let rest = 1.0 - e.get(this, Sky::fCurrentWeatherPct) as f64;
        let last_speed = e
            .call(
                WEATHER_BYTE_FRACTION,
                &args![last_weather, 0u32, 1.0f32, 0.0f32],
            )
            .f64();
        let mixed = last_speed * rest + scaled as f64;
        e.set(this, Sky::fWindSpeed, mixed as f32);
    }
    let (first, second): (u32, u32) = (e.global(0x011a_9b50), e.global(0x011a_9b54));
    fn_0063c640(e, first, second);
    let speed = e.get(this, Sky::fWindSpeed);
    fn_0063c660(e, speed);
    let fifteen: f32 = e.global(0x0101_e580);
    let seconds = e.call(0x0045_2e70, &args![0u32]).f64();
    let hour_seconds: f64 = e.global(0x0101_2640);
    let two_pi: f64 = e.global(0x0104_eea0);
    let phase = (seconds / hour_seconds * two_pi * fifteen as f64) as f32;
    let sine = fn_0063c600(e, phase);
    let x = (sine as f64 * fifteen as f64) as f32;
    let factor: f64 = e.global(0x0104_ee98);
    let shifted = (phase as f64 * factor) as f32;
    let other = e.call(0x0057_e960, &args![shifted]).f32();
    let y = (other as f64 * fifteen as f64) as f32;
    fn_0063c670(e, x, y);
}

// Translated from 0063c600 (decompiled, FalloutNV.exe 1.4.0.525)
/// Sine lookup (by its callers): `table[(int)(k / [011ccc50] * angle) & 0x1ff]`
/// with `k = [01028338]` and the table of 512 `float`s at `011f52e0`.
pub fn fn_0063c600(e: &mut Engine, angle: f32) -> f32 {
    let divisor: f32 = e.global(0x011c_cc50);
    let numerator: f64 = e.global(0x0102_8338);
    let scaled = numerator / divisor as f64 * angle as f64;
    let index = e.call(FTOL, &args![scaled]).u32() & 0x1ff;
    e.global(0x011f_52e0 + 4 * index)
}

// Translated from 0063c640 (decompiled, FalloutNV.exe 1.4.0.525)
/// Stores its two words in the globals `011f95dc` and `011f95e0`.
pub fn fn_0063c640(e: &mut Engine, first: u32, second: u32) {
    e.set_global(0x011f_95dc, first);
    e.set_global(0x011f_95e0, second);
}

// Translated from 0063c660 (decompiled, FalloutNV.exe 1.4.0.525)
/// Stores its `float` in the global `011ad824`.
pub fn fn_0063c660(e: &mut Engine, value: f32) {
    e.set_global(0x011a_d824, value);
}

// Translated from 0063c670 (decompiled, FalloutNV.exe 1.4.0.525)
/// Stores its two `float`s in the globals `011fd884` and `011fd888`.
pub fn fn_0063c670(e: &mut Engine, first: f32, second: f32) {
    e.set_global(0x011f_d884, first);
    e.set_global(0x011f_d888, second);
}

// Translated from 0063c690 (decompiled, FalloutNV.exe 1.4.0.525)
/// Mixes a colour (`Sky` method; `this` supplies the current weather at
/// `+0x10`). `source` holds four packed colours (`0xBBGGRR`, one per word)
/// followed by their four `float` weights: each output channel of `dest` is
/// the weighted sum of that channel's bytes divided by 255. `lightning` is
/// clamped to at most 1.0; when it is positive it is added to every channel
/// and, if there is a weather, each channel is then clamped to
/// `[0, weather byte fraction (table bytes 0xc, 0xd, 0xe)]`; otherwise (or
/// without a weather) every channel is clamped to at most 1.0 (`00a69690`).
pub fn fn_0063c690(e: &mut Engine, this: Ptr<Sky>, dest: Ptr, source: Ptr, lightning: f32) {
    let scale: f64 = e.global(0x0102_31e8);
    for channel in 0..3u32 {
        let mut sum = 0.0f32;
        for j in 0..4u32 {
            let packed = e.mem.u32(source.addr() + 4 * j);
            let byte = (packed >> (8 * channel)) & 0xff;
            let weight = e.mem.f32(source.addr() + 0x10 + 4 * j);
            sum = (byte as f64 * weight as f64 + sum as f64) as f32;
        }
        e.mem
            .set_f32(dest.addr() + 4 * channel, (sum as f64 * scale) as f32);
    }
    let lightning = e.call(MIN_FLOAT, &args![1.0f32, lightning]).f32();
    let zero: f64 = e.global(DOUBLE_ZERO);
    if lightning as f64 > zero {
        e.with_stack(12, |e, tint| {
            e.call(
                NI_COLOR_CONSTRUCT,
                &args![tint, lightning, lightning, lightning],
            );
            fn_0063c8a0(e, dest, tint);
        });
        let weather = e.get(this, Sky::pCurrentWeather);
        if !weather.is_null() {
            for channel in 0..3u32 {
                let limit = e
                    .call(
                        WEATHER_BYTE_FRACTION,
                        &args![weather, 0xcu32 + channel, 1.0f32, 0.0f32],
                    )
                    .f32();
                e.call(
                    CLAMP_FLOAT,
                    &args![dest.addr() + 4 * channel, 0.0f32, limit],
                );
            }
            return;
        }
    }
    e.call(COLOR_CLAMP_TO_ONE, &args![dest]);
}

// Translated from 0063c8a0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `NiColor::operator+=` (by its use): adds `other`'s three `float`s to
/// `this` and returns `this`.
pub fn fn_0063c8a0(e: &mut Engine, this: Ptr, other: Ptr) -> Ptr {
    for channel in 0..3u32 {
        let sum = e.mem.f32(this.addr() + 4 * channel) as f64
            + e.mem.f32(other.addr() + 4 * channel) as f64;
        e.mem.set_f32(this.addr() + 4 * channel, sum as f32);
    }
    this
}

// Translated from 0063c8f0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Sky::SetCurrentClimate` (Xbox PDB): when `climate` is null and there is
/// no current climate (or `force` is set), looks up the form with id `0x15f`
/// (`004839c0`) and casts it to a climate with `__RTDynamicCast`. A non-null
/// climate that differs from the current one (or any with `force`) becomes
/// current: flags `0x3f00` are set (`0063c9f0`), the weather is reset
/// (`Sky::ResetWeather`, `0063d060`) and flag `0x40` is set. The compiler's
/// exception frame is not translated.
pub fn sky_set_current_climate(e: &mut Engine, this: Ptr<Sky>, climate: Ptr, force: bool) {
    with_timer(e, 0x5b8, |e| {
        let mut climate = climate;
        if climate.is_null() && (e.get(this, Sky::pCurrentClimate).is_null() || force) {
            let form: Ptr = e.call(LOOKUP_FORM, &args![0x15fu32]).ptr();
            climate = e
                .call(
                    DYNAMIC_CAST,
                    &args![
                        form,
                        0u32,
                        TYPE_DESCRIPTOR_FORM,
                        TYPE_DESCRIPTOR_CLIMATE,
                        0u32
                    ],
                )
                .ptr();
        }
        if climate.is_null() {
            return;
        }
        if e.get(this, Sky::pCurrentClimate) != climate || force {
            e.set(this, Sky::pCurrentClimate, climate);
            fn_0063c9f0(e, this);
            e.call(SKY_RESET_WEATHER, &args![this]);
            let flags = e.get(this, Sky::uiFlags);
            e.set(this, Sky::uiFlags, flags | 0x40);
        }
    });
}

// Translated from 0063c9f0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Sets flags `0x3f00` of the sky (marks every cached climate time stale).
pub fn fn_0063c9f0(e: &mut Engine, this: Ptr<Sky>) {
    let flags = e.get(this, Sky::uiFlags);
    e.set(this, Sky::uiFlags, flags | 0x3f00);
}

// Translated from 0063ca20 (decompiled, FalloutNV.exe 1.4.0.525)
/// Rebuilds the climate-dependent parts of the sky when flag `0x40` is set
/// (`Sky::SetCurrentClimate` sets it), then clears the flag. In modes 2 and
/// 3: loads the stars' geometry (`Stars::LoadGeometry`, `0063fdb0`, with the
/// value of `0063cfe0` on the climate); loads the sun's two textures (the
/// climate's entries 0 and 1 of `0063cfa0`; a name equal to
/// `"Sky\SunGlare.dds"` with the flag byte `011f941e` clear is replaced by
/// `"Textures\Sky\SunGlareNonHDR.dds"`) into the properties of the sun's
/// nodes and marks both nodes by whether a texture loaded (`00635fe0`). Then
/// creates the "Masser" and "Secunda" moons (`Moon::Moon`, `00634a70`) when
/// the climate's byte at `+0x55` has bit `0x80` / `0x40` set and deletes them
/// when it has not; updates the root's properties and its controllers
/// (`00a59c60` with a zeroed `NiUpdateData`). The compiler's exception frame
/// is not translated.
pub fn fn_0063ca20(e: &mut Engine, this: Ptr<Sky>) {
    if e.get(this, Sky::uiFlags) & 0x40 == 0 {
        return;
    }
    let mode = e.get(this, Sky::eMode);
    let climate = e.get(this, Sky::pCurrentClimate);
    let stars = e.get(this, Sky::pStars);
    if !stars.is_null() && (mode == 3 || mode == 2) {
        let geometry = fn_0063cfe0(e, climate);
        e.call(STARS_LOAD_GEOMETRY, &args![stars, geometry]);
    }
    let sun = e.get(this, Sky::pSun);
    if !sun.is_null() && (mode == 3 || mode == 2) {
        load_sun_textures(e, sun, climate);
    }
    let wanted = fn_0063d000(e, climate);
    update_moon(e, this, Sky::pMasser, wanted, MASSER_MOON);
    let wanted = fn_0063d020(e, climate);
    update_moon(e, this, Sky::pSecunda, wanted, SECUNDA_MOON);
    let root = ni_pointer_get(e, this.addr() + 4);
    e.call(ROOT_UPDATE_PROPERTIES, &args![root]);
    e.with_stack(12, |e, update_data| {
        e.call(
            NI_UPDATE_DATA_CONSTRUCT,
            &args![update_data, 0.0f32, 0u32, 0u32],
        );
        let root = ni_pointer_get(e, this.addr() + 4);
        e.call(NODE_UPDATE_CONTROLLERS, &args![root, update_data]);
    });
    let flags = e.get(this, Sky::uiFlags);
    e.set(this, Sky::uiFlags, flags & 0xffff_ffbf);
}

/// Builds the sun's two textures from the climate's entries 0 and 1 and
/// attaches them to the properties of the sun's nodes (the node at `+0x10`
/// and the node at `+0x14` of the sun's holder), then marks the sun's nodes
/// by whether a texture was found. `texture` is not reset between the two
/// loads, as in the game.
fn load_sun_textures(e: &mut Engine, sun: Ptr, climate: Ptr) {
    e.with_stack(8, |e, text| {
        e.call(STRING_CONSTRUCT, &args![text]);
        let node: Ptr = e.call(SUN_FIRST_NODE, &args![sun]).ptr();
        let property: Ptr = e.call(NODE_GET_PROPERTY, &args![node, 3u32]).ptr();
        let mut texture = Ptr::NULL;
        let entry = fn_0063cfa0(e, climate, 0);
        if !entry.is_null() && e.call(TEXTURE_ENTRY_VALID, &args![entry]).u32() != 0 {
            e.with_stack(4, |e, holder| {
                e.call(NI_POINTER_FROM_RAW, &args![holder, 0u32]);
                let prefix = e.vcall(entry.addr(), 0x18, &[]).u32();
                e.call(STRING_ASSIGN, &args![text, prefix]);
                let name = e.call(TEXTURE_ENTRY_NAME, &args![entry]).u32();
                e.call(STRING_APPEND, &args![text, name]);
                texture = load_texture(e, text, holder, property);
                e.call(NI_POINTER_DESTRUCTOR, &args![holder]);
            });
        }
        let found = (!texture.is_null()) as u32;
        let node: Ptr = e.call(HOLDER_NODE, &args![sun]).ptr();
        e.call(NODE_SET_FLAG_BIT_20, &args![node, found]);

        let node = fn_0063d040(e, sun);
        let property: Ptr = e.call(NODE_GET_PROPERTY, &args![node, 3u32]).ptr();
        let entry = fn_0063cfa0(e, climate, 1);
        if !entry.is_null() && e.call(TEXTURE_ENTRY_VALID, &args![entry]).u32() != 0 {
            let hdr_off = e.global::<u8>(INITIAL_FLAG) == 0;
            let mut use_default = false;
            if hdr_off {
                let name = e.call(TEXTURE_ENTRY_NAME, &args![entry]).u32();
                use_default = e.call(STRING_COMPARE, &args![name, SUN_GLARE_NAME]).u32() == 0;
            }
            if use_default {
                e.call(STRING_ASSIGN, &args![text, SUN_GLARE_NON_HDR_PATH]);
            } else {
                let prefix = e.vcall(entry.addr(), 0x18, &[]).u32();
                e.call(STRING_ASSIGN, &args![text, prefix]);
                let name = e.call(TEXTURE_ENTRY_NAME, &args![entry]).u32();
                e.call(STRING_APPEND, &args![text, name]);
            }
            e.with_stack(4, |e, holder| {
                e.call(NI_POINTER_FROM_RAW, &args![holder, 0u32]);
                texture = load_texture(e, text, holder, property);
                e.call(NI_POINTER_DESTRUCTOR, &args![holder]);
            });
        }
        let found = (!texture.is_null()) as u32;
        let node: Ptr = e.call(SUN_GET_NODE, &args![sun]).ptr();
        e.call(NODE_SET_FLAG_BIT_20, &args![node, found]);
        e.call(STRING_DESTRUCT, &args![text]);
    });
}

/// Loads the texture named by the string at `text` into the `NiPointer` at
/// `holder` (`BSShaderManager::GetTexture`-style loader `00b55840`) and, when
/// one was found, sets it in `property` (`006348e0`). Returns the texture.
fn load_texture(e: &mut Engine, text: Ptr, holder: Ptr, property: Ptr) -> Ptr {
    let path = ni_pointer_get(e, text.addr());
    e.call(TEXTURE_LOAD, &args![path, 1u32, holder, 1u32, 0u32]);
    let texture = ni_pointer_get(e, holder.addr());
    if !texture.is_null() {
        e.call(PROPERTY_SET_TEXTURE, &args![property, texture]);
    }
    texture
}

/// How `fn_0063ca20` builds one moon: its name and the six settings
/// (a word setting and five `float` settings, in the order the game reads
/// them).
struct MoonSettings {
    name: u32,
    word_setting: u32,
    float_settings: [u32; 5],
}

const MASSER_MOON: MoonSettings = MoonSettings {
    name: 0x0104_eeb0,
    word_setting: 0x011c_cc18,
    float_settings: [
        0x011c_cc54,
        0x011c_cbfc,
        0x011c_cba4,
        0x011c_cc94,
        0x011c_ccb8,
    ],
};

const SECUNDA_MOON: MoonSettings = MoonSettings {
    name: 0x0104_eea8,
    word_setting: 0x011c_ccec,
    float_settings: [
        0x011c_cc7c,
        0x011c_cbcc,
        0x011c_cc64,
        0x011c_cbe8,
        0x011c_cb98,
    ],
};

/// Creates the moon stored in `field` (when `wanted` and there is none: a
/// `Moon` of 0x7c bytes built with the settings, then initialized through
/// virtual slot `0x10` with the moons root and its name) or deletes it (when
/// not `wanted`).
fn update_moon(
    e: &mut Engine,
    this: Ptr<Sky>,
    field: Field<Sky, Ptr>,
    wanted: bool,
    settings: MoonSettings,
) {
    if wanted && e.get(this, field).is_null() {
        let block: Ptr = e.call(OPERATOR_NEW, &args![0x7cu32]).ptr();
        let moon = if block.is_null() {
            Ptr::NULL
        } else {
            let word_slot: Ptr = e
                .call(SETTING_WORD_VALUE, &args![settings.word_setting])
                .ptr();
            let word = e.mem.u32(word_slot.addr());
            let mut values = [0.0f32; 5];
            for (value, setting) in values.iter_mut().zip(settings.float_settings) {
                let slot: Ptr = e.call(SETTING_FLOAT_VALUE, &args![setting]).ptr();
                *value = e.mem.f32(slot.addr());
            }
            e.call(
                MOON_CONSTRUCT,
                &args![
                    block,
                    settings.name,
                    values[4],
                    values[3],
                    values[2],
                    values[1],
                    values[0],
                    word
                ],
            )
            .ptr()
        };
        e.set(this, field, moon);
        let moons_root = ni_pointer_get(e, this.addr() + 8);
        e.vcall(moon.addr(), 0x10, &args![moons_root, settings.name]);
    } else if !wanted {
        let moon = e.get(this, field);
        delete_component(e, moon);
        e.set(this, field, Ptr::NULL);
    }
}

// Translated from 0063cfa0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Climate accessor (by its caller): the address of entry `index` (12 bytes
/// each from `+0x38`) when `index < 2`, else null.
pub fn fn_0063cfa0(_e: &mut Engine, this: Ptr, index: i32) -> Ptr {
    if index < 2 {
        Ptr::new(
            this.addr()
                .wrapping_add((index as u32).wrapping_mul(12))
                .wrapping_add(0x38),
        )
    } else {
        Ptr::NULL
    }
}

// Translated from 0063cfe0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Climate method (by its caller): calls virtual slot `0x14` of the object
/// embedded at `+0x18` and returns its result.
pub fn fn_0063cfe0(e: &mut Engine, this: Ptr) -> u32 {
    e.vcall(this.addr() + 0x18, 0x14, &[]).u32()
}

// Translated from 0063d000 (decompiled, FalloutNV.exe 1.4.0.525)
/// Climate accessor: whether bit `0x80` of the byte at `+0x55` is set (the
/// climate has Masser).
pub fn fn_0063d000(e: &mut Engine, this: Ptr) -> bool {
    e.mem.u8(this.addr() + 0x55) & 0x80 != 0
}

// Translated from 0063d020 (decompiled, FalloutNV.exe 1.4.0.525)
/// Climate accessor: whether bit `0x40` of the byte at `+0x55` is set (the
/// climate has Secunda).
pub fn fn_0063d020(e: &mut Engine, this: Ptr) -> bool {
    e.mem.u8(this.addr() + 0x55) & 0x40 != 0
}

// Translated from 0063d040 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Sun` accessor (by its caller): the `NiPointer` stored at `+0x14`.
pub fn fn_0063d040(e: &mut Engine, this: Ptr) -> Ptr {
    ni_pointer_get(e, this.addr() + 0x14)
}

// @@FUNCS

/// This unit's translated functions, by exe address.
pub fn funcs() -> Vec<(u32, AbiFn)> {
    vec![
        entry!(
            0x00639bf0,
            fn_00639bf0(Ptr<SkySound>, u32, u32, u32, Ptr, u32, u32) -> Ptr<SkySound>
        ),
        entry!(0x00639c90, fn_00639c90(Ptr, u32) -> bool),
        entry!(
            0x0063a080,
            sky_scalar_deleting_destructor(Ptr<Sky>, u32) -> Ptr<Sky>
        ),
        entry!(0x0063a0b0, fn_0063a0b0(Ptr<Sky>)),
        entry!(0x0063a3c0, fn_0063a3c0(Ptr<SkySound>, u32) -> Ptr<SkySound>),
        entry!(0x0063a3f0, sky_set_mode(Ptr<Sky>, u32)),
        entry!(0x0063a610, fn_0063a610(Ptr)),
        entry!(0x0063a630, sky_initialize(Ptr<Sky>, Ptr, u32)),
        entry!(0x0063ac70, sky_update(Ptr<Sky>, f32)),
        entry!(0x0063b0a0, fn_0063b0a0(Ptr<Sky>, f32)),
        entry!(0x0063b120, fn_0063b120(Ptr<Sky>)),
        entry!(0x0063b600, fn_0063b600(Ptr, i32, Ptr)),
        entry!(0x0063b630, fn_0063b630(Ptr<Sky>, Ptr, Ptr, u32, Ptr, Ptr)),
        entry!(0x0063b9b0, fn_0063b9b0(Ptr<Sky>) -> f32),
        entry!(0x0063ba30, fn_0063ba30(Ptr<Sky>) -> f32),
        entry!(
            0x0063bab0,
            fn_0063bab0(Ptr<Sky>, Ptr, Ptr, Ptr, i32, Ptr, Ptr)
        ),
        entry!(0x0063bb40, fn_0063bb40(Ptr, i32, i32) -> u32),
        entry!(
            0x0063bb70,
            fn_0063bb70(Ptr<Sky>, Ptr, Ptr, Ptr, i32, Ptr, Ptr)
        ),
        entry!(0x0063bc60, fn_0063bc60(Ptr, i32, i32) -> u32),
        entry!(0x0063bce0, fn_0063bce0(Ptr<Sky>)),
        entry!(0x0063c420, fn_0063c420(Ptr, i32) -> f32),
        entry!(0x0063c440, fn_0063c440(Ptr, i32) -> Ptr),
        entry!(0x0063c460, fn_0063c460() -> bool),
        entry!(0x0063c470, fn_0063c470() -> u32),
        entry!(0x0063c480, fn_0063c480() -> f32),
        entry!(0x0063c490, fn_0063c490(Ptr<Sky>)),
        entry!(0x0063c600, fn_0063c600(f32) -> f32),
        entry!(0x0063c640, fn_0063c640(u32, u32)),
        entry!(0x0063c660, fn_0063c660(f32)),
        entry!(0x0063c670, fn_0063c670(f32, f32)),
        entry!(0x0063c690, fn_0063c690(Ptr<Sky>, Ptr, Ptr, f32)),
        entry!(0x0063c8a0, fn_0063c8a0(Ptr, Ptr) -> Ptr),
        entry!(0x0063c8f0, sky_set_current_climate(Ptr<Sky>, Ptr, bool)),
        entry!(0x0063c9f0, fn_0063c9f0(Ptr<Sky>)),
        entry!(0x0063ca20, fn_0063ca20(Ptr<Sky>)),
        entry!(0x0063cfa0, fn_0063cfa0(Ptr, i32) -> Ptr),
        entry!(0x0063cfe0, fn_0063cfe0(Ptr) -> u32),
        entry!(0x0063d000, fn_0063d000(Ptr) -> bool),
        entry!(0x0063d020, fn_0063d020(Ptr) -> bool),
        entry!(0x0063d040, fn_0063d040(Ptr) -> Ptr),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ret(value: u32) -> Ret {
        Ret {
            eax: value,
            ..Ret::default()
        }
    }

    fn ret_float(value: f32) -> Ret {
        Ret {
            st0: value as f64,
            ..Ret::default()
        }
    }

    /// An engine with test doubles for the smart-pointer, list and timer
    /// helpers every sky function uses: `NiPointer::get` reads the word,
    /// `operator=` stores it (no reference counting), `operator!=` compares,
    /// list nodes are `{ item, next }`, `operator new` is the heap.
    fn engine() -> Engine {
        let mut e = Engine::new();
        e.register(NI_POINTER_GET, |e, a| ret(e.mem.u32(a[0])));
        e.register(NI_POINTER_ASSIGN, |e, a| {
            e.mem.set_u32(a[0], a[1]);
            ret(a[0])
        });
        e.register(NI_POINTER_NOT_EQUAL, |e, a| {
            ret((e.mem.u32(a[0]) != a[1]) as u32)
        });
        e.register(NI_POINTER_DESTRUCTOR, |_, _| Ret::default());
        e.register(LIST_NODE_ITEM, |_, a| ret(a[0]));
        e.register(LIST_NODE_NEXT, |e, a| ret(e.mem.u32(a[0] + 4)));
        e.register(TIMER_SCOPE_CONSTRUCT, |_, _| Ret::default());
        e.register(TIMER_SCOPE_DESTROY, |_, _| Ret::default());
        e.register(OPERATOR_NEW, |e, a| ret(e.mem.alloc(a[0])));
        e.register(NI_OBJECT_NEW, |e, a| ret(e.mem.alloc(a[0])));
        e.register(OPERATOR_DELETE, |_, _| Ret::default());
        e
    }

    /// Maps the pages of the exe's data the sky code reads and sets the
    /// constants it loads.
    fn with_data(mut e: Engine) -> Engine {
        e.map(0x0100_0000, 0x0010_0000);
        e.map(0x0118_0000, 0x0009_0000);
        e.set_global(DOUBLE_ONE, 1.0f64);
        e.set_global(DOUBLE_HALF, 0.5f64);
        e.set_global(THOUSAND_DOUBLE, 1000.0f64);
        e
    }

    const VTABLE: u32 = 0x00ac_0000;
    const SLOT_BASE: u32 = 0x00aa_0000;

    /// A vtable at [`VTABLE`] whose slot at byte offset `k` is a no-op
    /// double at `SLOT_BASE + k`, so `call_log` shows which slot was called.
    fn install_vtable(e: &mut Engine) {
        let slots: Vec<u32> = (0..0x48).map(|i| SLOT_BASE + 4 * i).collect();
        e.put_vtable(VTABLE, &slots);
        for i in 0..0x48 {
            e.register(SLOT_BASE + 4 * i, |_, _| Ret::default());
        }
    }

    fn object_with_vtable(e: &mut Engine, size: u32) -> Ptr {
        let object = e.mem.alloc(size);
        e.mem.set_u32(object, VTABLE);
        Ptr::new(object)
    }

    fn addresses(log: &[(u32, Vec<u32>)]) -> Vec<u32> {
        log.iter().map(|(address, _)| *address).collect()
    }

    fn new_sky(e: &mut Engine) -> Ptr<Sky> {
        e.new_object()
    }

    #[test]
    fn sky_sound_constructor_copies_the_handle_and_stores_the_description() {
        let mut e = engine();
        e.register(SOUND_HANDLE_CONSTRUCT, |e, a| {
            e.mem.set_u32(a[0], u32::MAX);
            ret(a[0])
        });
        e.register(SOUND_HANDLE_ASSIGN, |e, a| {
            for word in 0..3 {
                let value = e.mem.u32(a[1] + 4 * word);
                e.mem.set_u32(a[0] + 4 * word, value);
            }
            ret(a[0])
        });
        e.register(SOUND_HANDLE_DESTRUCT, |_, _| Ret::default());
        e.register(MEMSET, |e, a| {
            e.mem.set_u32(a[0], a[1]);
            ret(a[0])
        });
        let sound: Ptr<SkySound> = e.new_object();
        e.mem.set_u32(sound.addr() + 0x18, 0x1234);
        e.call_log = Some(vec![]);
        let back = e
            .call(
                0x0063_9bf0,
                &args![sound, 7u32, 1u32, 2u32, 0x5000u32, 3u32, 0xabcu32],
            )
            .ptr::<SkySound>();
        assert_eq!(back, sound);
        let log = e.call_log.take().unwrap();
        assert_eq!(
            addresses(&log),
            [
                0x0063_9bf0,
                SOUND_HANDLE_CONSTRUCT,
                SOUND_HANDLE_ASSIGN,
                SOUND_HANDLE_DESTRUCT,
                MEMSET
            ]
        );
        assert_eq!(log[4].1, [sound.addr() + 0x18, 0, 4]);
        // The handle passed by value ends up in the sound.
        assert_eq!(e.get(sound, SkySound::SndHandle), 7);
        assert_eq!(e.mem.u32(sound.addr() + 4), 1);
        assert_eq!(e.mem.u32(sound.addr() + 8), 2);
        assert_eq!(e.get(sound, SkySound::pWeather), Ptr::new(0x5000));
        assert_eq!(e.get(sound, SkySound::eSoundType), 3);
        assert_eq!(e.get(sound, SkySound::uiFormID), 0xabc);
        assert_eq!(e.get(sound, SkySound::uiData), 0);
    }

    #[test]
    fn fn_00639c90_grows_only_past_the_limit() {
        let mut e = engine();
        e.register(0x0045_7fe0, |_, _| ret(100));
        e.register(0x0063_9ce0, |_, _| ret(0x1111));
        e.register(0x0042_f5a0, |_, a| {
            assert_eq!(a, [0x8000_0000, 0x1111]);
            ret(40)
        });
        e.register(0x0063_9d00, |_, _| Ret::default());
        e.call_log = Some(vec![]);
        assert!(e.call(0x0063_9c90, &args![0x6000u32, 5u32]).bool());
        let log = e.call_log.take().unwrap();
        assert_eq!(log.last().unwrap(), &(0x0063_9d00, vec![0x6000, 105]));
        // A limit at or above the value: nothing happens.
        e.register(0x0042_f5a0, |_, _| ret(100));
        e.call_log = Some(vec![]);
        assert!(!e.call(0x0063_9c90, &args![0x6000u32, 5u32]).bool());
        assert_eq!(e.call_log.take().unwrap().len(), 4);
    }

    #[test]
    fn deleting_destructor_frees_only_when_asked() {
        let mut e = engine();
        e.register(LIST_CLEAR, |_, _| Ret::default());
        let sky = new_sky(&mut e);
        e.call_log = Some(vec![]);
        let back = e.call(0x0063_a080, &args![sky, 0u32]).ptr::<Sky>();
        assert_eq!(back, sky);
        let log = e.call_log.take().unwrap();
        assert!(!addresses(&log).contains(&OPERATOR_DELETE));
        e.call_log = Some(vec![]);
        e.call(0x0063_a080, &args![sky, 1u32]);
        let log = e.call_log.take().unwrap();
        assert_eq!(log.last().unwrap(), &(OPERATOR_DELETE, vec![sky.addr()]));
    }

    #[test]
    fn destructor_deletes_components_in_order_and_empties_the_sound_list() {
        let mut e = engine();
        install_vtable(&mut e);
        e.register(LIST_CLEAR, |_, _| Ret::default());
        e.register(LIST_SCALAR_DELETING_DESTRUCTOR, |_, _| Ret::default());
        e.register(SKY_SOUND_BASE_DESTRUCT, |_, _| Ret::default());
        e.register(ROOT_GET_SCENE, |e, _| {
            let scene = object_with_vtable(e, 8);
            ret(scene.addr())
        });
        let sky = new_sky(&mut e);
        let fields = [
            Sky::pAtmosphere,
            Sky::pStars,
            Sky::pSun,
            Sky::pClouds,
            Sky::pMasser,
            Sky::pSecunda,
            Sky::pPrecip,
        ];
        let mut components = vec![];
        for field in fields {
            let component = object_with_vtable(&mut e, 0x10);
            e.set(sky, field, component);
            components.push(component.addr());
        }
        e.mem.set_u32(sky.addr() + 4, 0x7777);
        e.mem.set_u32(sky.addr() + 8, 0x8888);
        // Two sounds in the list: node = { item, next }.
        let second = e.mem.alloc(8);
        let first = e.mem.alloc(8);
        let (sound_a, sound_b) = (e.mem.alloc(0x1c), e.mem.alloc(0x1c));
        e.mem.set_u32(first, sound_a);
        e.mem.set_u32(first + 4, second);
        e.mem.set_u32(second, sound_b);
        e.set(sky, Sky::pSkySoundList, Ptr::new(first));
        e.call_log = Some(vec![]);
        fn_0063a0b0(&mut e, sky);
        let log = e.call_log.take().unwrap();
        assert_eq!(e.mem.u32(sky.addr()), SKY_VTABLE);
        // Slot 0 of atmosphere, stars, clouds, sun, masser, secunda, precip.
        let deletions: Vec<u32> = log
            .iter()
            .filter(|(address, _)| *address == SLOT_BASE)
            .map(|(_, args)| args[0])
            .collect();
        let expected: Vec<u32> = [0, 1, 3, 2, 4, 5, 6]
            .iter()
            .map(|index| components[*index])
            .collect();
        assert_eq!(deletions, expected);
        assert!(log
            .iter()
            .all(|(address, args)| *address != SLOT_BASE || args[1] == 1));
        // The root is taken out of its scene (slot 0xe8) and both smart
        // pointers are released.
        assert!(log
            .iter()
            .any(|(address, args)| *address == SLOT_BASE + 0xe8 && args[1] == 0x7777));
        assert_eq!(e.mem.u32(sky.addr() + 4), 0);
        assert_eq!(e.mem.u32(sky.addr() + 8), 0);
        // Both sounds are deleted, then the list is cleared and destroyed.
        let sounds: Vec<&Vec<u32>> = log
            .iter()
            .filter(|(address, _)| *address == OPERATOR_DELETE)
            .map(|(_, args)| args)
            .collect();
        assert_eq!(sounds, [&vec![sound_a], &vec![sound_b]]);
        let tail = addresses(&log);
        assert_eq!(
            tail[tail.len() - 4..],
            [
                LIST_CLEAR,
                LIST_SCALAR_DELETING_DESTRUCTOR,
                NI_POINTER_DESTRUCTOR,
                NI_POINTER_DESTRUCTOR
            ]
        );
    }

    #[test]
    fn sky_sound_deleting_destructor_runs_the_base_and_frees() {
        let mut e = engine();
        e.register(SKY_SOUND_BASE_DESTRUCT, |_, _| Ret::default());
        let sound: Ptr<SkySound> = e.new_object();
        e.call_log = Some(vec![]);
        e.call(0x0063_a3c0, &args![sound, 1u32]);
        let log = e.call_log.take().unwrap();
        assert_eq!(
            log[1..],
            [
                (SKY_SOUND_BASE_DESTRUCT, vec![sound.addr()]),
                (OPERATOR_DELETE, vec![sound.addr()])
            ]
        );
        e.call_log = Some(vec![]);
        let back = e.call(0x0063_a3c0, &args![sound, 0u32]).ptr::<SkySound>();
        assert_eq!(back, sound);
        assert_eq!(e.call_log.take().unwrap().len(), 2);
    }

    #[test]
    fn set_mode_entering_a_visible_mode_clears_the_visibility_flags() {
        let mut e = engine();
        e.register(SKY_ENTER_VISIBLE_MODE, |_, _| Ret::default());
        e.register(SET_NODE_HIDDEN, |_, _| Ret::default());
        e.register(SUN_GET_NODE, |_, _| ret(0x4444));
        e.register(SKY_UPDATE_HDR_VALUES, |_, _| Ret::default());
        let sky = new_sky(&mut e);
        e.set(sky, Sky::eMode, 1);
        let clouds = e.mem.alloc(0x5c);
        e.set(sky, Sky::pClouds, Ptr::new(clouds));
        e.mem.set_u32(sky.addr() + 4, 0x3333);
        e.set(sky, Sky::pSun, Ptr::new(0x5555));
        e.call_log = Some(vec![]);
        e.call(0x0063_a3f0, &args![sky, 3u32]);
        let log = e.call_log.take().unwrap();
        assert_eq!(e.get(sky, Sky::eMode), 3);
        assert_eq!(e.mem.u8(clouds + 0x5a), 1);
        assert!(log.contains(&(SET_NODE_HIDDEN, vec![0x3333, 0])));
        assert!(log.contains(&(SET_NODE_HIDDEN, vec![0x4444, 0])));
        assert_eq!(
            log.last().unwrap(),
            &(SKY_UPDATE_HDR_VALUES, vec![sky.addr()])
        );
    }

    #[test]
    fn set_mode_leaving_a_visible_mode_stops_every_sound() {
        let mut e = with_data(engine());
        e.register(SKY_UNLOAD_ALL_TEXTURES, |_, _| Ret::default());
        e.register(SET_NODE_HIDDEN, |_, _| Ret::default());
        e.register(SUN_GET_NODE, |_, _| ret(0x4444));
        e.register(SKY_UPDATE_HDR_VALUES, |_, _| Ret::default());
        e.register(SOUND_HANDLE_IS_PLAYING, |_, _| ret(1));
        e.register(SOUND_HANDLE_STOP, |_, _| Ret::default());
        e.register(SOUND_HANDLE_RELEASE, |_, _| Ret::default());
        e.register(SKY_SOUND_BASE_DESTRUCT, |_, _| Ret::default());
        e.register(PRECIPITATION_UPDATE, |_, _| Ret::default());
        // One sound in the list; the list is empty after `RemoveHead`.
        let list = e.mem.alloc(8);
        let sound = e.mem.alloc(0x1c);
        e.mem.set_u32(list, sound);
        e.register(LIST_IS_EMPTY, |e, a| ret((e.mem.u32(a[0]) == 0) as u32));
        e.register(LIST_REMOVE_HEAD, |e, a| {
            e.mem.set_u32(a[0], 0);
            Ret::default()
        });
        let sky = new_sky(&mut e);
        e.set(sky, Sky::eMode, 3);
        e.set(sky, Sky::pSkySoundList, Ptr::new(list));
        e.set(sky, Sky::pPrecip, Ptr::new(0x6666));
        e.set(sky, Sky::fFlash, 0.75);
        e.mem.set_u32(sky.addr() + 4, 0x3333);
        e.set_global(SET_MODE_FLAG, 1u8);
        e.call_log = Some(vec![]);
        e.call(0x0063_a3f0, &args![sky, 0u32]);
        let log = e.call_log.take().unwrap();
        assert_eq!(addresses(&log)[1], SKY_UNLOAD_ALL_TEXTURES);
        assert!(log.contains(&(SET_NODE_HIDDEN, vec![0x3333, 1])));
        assert!(log.contains(&(SOUND_HANDLE_STOP, vec![sound])));
        assert!(log.contains(&(SOUND_HANDLE_RELEASE, vec![sound])));
        assert!(log.contains(&(OPERATOR_DELETE, vec![sound])));
        assert!(log.contains(&(LIST_REMOVE_HEAD, vec![list])));
        assert!(log.contains(&(PRECIPITATION_UPDATE, vec![0x6666, 0])));
        assert_eq!(e.get(sky, Sky::eMode), 0);
        assert_eq!(e.get(sky, Sky::fFlash), 0.0);
        assert_eq!(e.global::<u8>(SET_MODE_FLAG), 0);
    }

    #[test]
    fn set_mode_between_hidden_modes_only_stores_the_mode() {
        let mut e = engine();
        e.register(SKY_UPDATE_HDR_VALUES, |_, _| Ret::default());
        let sky = new_sky(&mut e);
        e.set(sky, Sky::eMode, 0);
        e.call_log = Some(vec![]);
        e.call(0x0063_a3f0, &args![sky, 1u32]);
        let log = e.call_log.take().unwrap();
        assert_eq!(addresses(&log), [0x0063_a3f0, SKY_UPDATE_HDR_VALUES]);
        assert_eq!(e.get(sky, Sky::eMode), 1);
    }

    #[test]
    fn clouds_flag_setter_sets_the_byte() {
        let mut e = engine();
        let clouds: Ptr = Ptr::new(e.mem.alloc(0x5c));
        e.call(0x0063_a610, &args![clouds]);
        assert_eq!(e.mem.u8(clouds.addr() + 0x5a), 1);
    }

    fn min_float(_: &mut Engine, a: &[u32]) -> Ret {
        let (first, second) = (f32::from_bits(a[0]), f32::from_bits(a[1]));
        ret_float(if second > first { first } else { second })
    }

    fn max_float(_: &mut Engine, a: &[u32]) -> Ret {
        let (first, second) = (f32::from_bits(a[0]), f32::from_bits(a[1]));
        ret_float(if second < first { first } else { second })
    }

    /// `NiColor::NiColor(r, g, b)`: stores the three floats, returns `this`.
    fn color_construct(e: &mut Engine, a: &[u32]) -> Ret {
        for channel in 0..3 {
            e.mem.set_u32(a[0] + 4 * channel, a[1 + channel as usize]);
        }
        ret(a[0])
    }

    /// Doubles for the helpers the colour mixing of `0063c690` uses.
    fn with_color_helpers(mut e: Engine) -> Engine {
        e.register(MIN_FLOAT, min_float);
        e.register(MAX_FLOAT, max_float);
        e.register(NI_COLOR_CONSTRUCT, color_construct);
        e.register(COLOR_CLAMP_TO_ONE, |e, a| {
            for channel in 0..3 {
                let value = e.mem.f32(a[0] + 4 * channel);
                if value > 1.0 {
                    e.mem.set_f32(a[0] + 4 * channel, 1.0);
                }
            }
            Ret::default()
        });
        e.register(CLAMP_FLOAT, |e, a| {
            let (low, high) = (f32::from_bits(a[1]), f32::from_bits(a[2]));
            let value = e.mem.f32(a[0]);
            if high < value {
                e.mem.set_f32(a[0], high);
            } else if value < low {
                e.mem.set_f32(a[0], low);
            }
            Ret::default()
        });
        // A float setting is its own value: the "pointer" is the address.
        e.register(SETTING_FLOAT_VALUE, |_, a| ret(a[0]));
        e
    }

    fn initialize_engine() -> Engine {
        let mut e = with_data(engine());
        install_vtable(&mut e);
        for constructor in [
            MULTIBOUND_NODE_CONSTRUCT,
            ATMOSPHERE_CONSTRUCT,
            STARS_CONSTRUCT,
            SUN_CONSTRUCT,
            CLOUDS_CONSTRUCT,
            MOONS_ROOT_CONSTRUCT,
            PRECIPITATION_CONSTRUCT,
            SKY_PROPERTY_CONSTRUCT,
        ] {
            e.register(constructor, |e, a| {
                e.mem.set_u32(a[0], VTABLE);
                ret(a[0])
            });
        }
        for plain in [
            ROOT_SET_FIRST_FLAG,
            ROOT_SET_SECOND_FLAG,
            ROOT_SET_NAME,
            ROOT_ATTACH_PROPERTY,
            ROOT_UPDATE_PROPERTIES,
            PRECIPITATION_INITIALIZE,
            SKY_PROPERTY_SET_A,
            SKY_PROPERTY_SET_B,
            SKY_PROPERTY_SET_C,
            SKY_PROPERTY_SET_D,
        ] {
            e.register(plain, |_, _| Ret::default());
        }
        e.register(ROOT_GET_SCENE, |_, _| ret(0));
        e.register(NI_POINTER_FROM_RAW, |e, a| {
            e.mem.set_u32(a[0], a[1]);
            ret(a[0])
        });
        // The setting is its own value: the "pointer" is the address.
        e.register(SETTING_BYTE_VALUE, |_, a| ret(a[0]));
        e
    }

    #[test]
    fn initialize_builds_every_component_and_attaches_the_moons() {
        let mut e = initialize_engine();
        e.mem.set_u8(SETTING_PRECIPITATION, 1);
        let sky = new_sky(&mut e);
        e.call_log = Some(vec![]);
        e.call(0x0063_a630, &args![sky, 0u32, 0x77u32]);
        let log = e.call_log.take().unwrap();
        let root = e.mem.u32(sky.addr() + 4);
        let moons = e.mem.u32(sky.addr() + 8);
        let atmosphere = e.get(sky, Sky::pAtmosphere).addr();
        let stars = e.get(sky, Sky::pStars).addr();
        let sun = e.get(sky, Sky::pSun).addr();
        let clouds = e.get(sky, Sky::pClouds).addr();
        let precipitation = e.get(sky, Sky::pPrecip).addr();
        for value in [root, moons, atmosphere, stars, sun, clouds, precipitation] {
            assert_ne!(value, 0);
        }
        // Sizes the game allocates.
        assert_eq!(e.mem.block_size(root), Some(0xb8));
        assert_eq!(e.mem.block_size(atmosphere), Some(0x20));
        assert_eq!(e.mem.block_size(sun), Some(0x30));
        // The root is flagged and named; the components are initialized
        // with the root (slots 8 and 0x10) and the moons root is attached.
        assert!(log.contains(&(ROOT_SET_FIRST_FLAG, vec![root, 1])));
        assert!(log.contains(&(ROOT_SET_SECOND_FLAG, vec![root, 1])));
        assert!(log.contains(&(ROOT_SET_NAME, vec![root, ROOT_NAME_ARGUMENT])));
        assert!(log.contains(&(SLOT_BASE + 0x10, vec![atmosphere, root, 0x77])));
        assert!(log.contains(&(SLOT_BASE + 8, vec![stars, root])));
        assert!(log.contains(&(SLOT_BASE + 8, vec![sun, root])));
        assert!(log.contains(&(SLOT_BASE + 8, vec![clouds, root])));
        assert!(log.contains(&(SLOT_BASE + 0xdc, vec![root, moons, 1])));
        assert!(log.contains(&(PRECIPITATION_INITIALIZE, vec![precipitation])));
        // The property object gets its four settings, then the root.
        let property = log
            .iter()
            .find(|(address, _)| *address == SKY_PROPERTY_SET_A)
            .unwrap()
            .1[0];
        assert!(log.contains(&(SKY_PROPERTY_SET_A, vec![property, 6])));
        assert!(log.contains(&(SKY_PROPERTY_SET_B, vec![property, 7])));
        assert!(log.contains(&(SKY_PROPERTY_SET_C, vec![property, 1])));
        assert!(log.contains(&(SKY_PROPERTY_SET_D, vec![property, 1])));
        assert!(log.contains(&(ROOT_ATTACH_PROPERTY, vec![root, property])));
        // The last call is the scope timer's destructor.
        assert_eq!(log[log.len() - 2], (ROOT_UPDATE_PROPERTIES, vec![root]));
    }

    #[test]
    fn initialize_replaces_old_components_and_uses_the_given_root() {
        let mut e = initialize_engine();
        e.mem.set_u8(SETTING_PRECIPITATION, 0);
        e.set_global(INITIAL_FLAG, 1u8);
        let sky = new_sky(&mut e);
        let given_root = object_with_vtable(&mut e, 0xb4);
        // An old root with a scene, and an old atmosphere and precipitation.
        e.mem.set_u32(sky.addr() + 4, 0x1357);
        e.register(ROOT_GET_SCENE, |e, _| {
            let scene = object_with_vtable(e, 8);
            ret(scene.addr())
        });
        let old_atmosphere = object_with_vtable(&mut e, 0x20);
        e.set(sky, Sky::pAtmosphere, old_atmosphere);
        let old_precipitation = object_with_vtable(&mut e, 0x20);
        e.set(sky, Sky::pPrecip, old_precipitation);
        e.call_log = Some(vec![]);
        e.call(0x0063_a630, &args![sky, given_root, 0u32]);
        let log = e.call_log.take().unwrap();
        // The old root is taken out of its scene (slot 0xe8) and replaced.
        assert!(log
            .iter()
            .any(|(address, args)| *address == SLOT_BASE + 0xe8 && args[1] == 0x1357));
        assert_eq!(e.mem.u32(sky.addr() + 4), given_root.addr());
        assert!(!addresses(&log).contains(&MULTIBOUND_NODE_CONSTRUCT));
        // The old components are deleted with flag 1; no precipitation now.
        assert!(log.contains(&(SLOT_BASE, vec![old_atmosphere.addr(), 1])));
        assert!(log.contains(&(SLOT_BASE, vec![old_precipitation.addr(), 1])));
        assert!(e.get(sky, Sky::pPrecip).is_null());
        assert!(!addresses(&log).contains(&PRECIPITATION_INITIALIZE));
        assert_eq!(e.global::<f32>(INITIAL_FLOAT_A), 1.0);
        assert_eq!(e.global::<f32>(INITIAL_FLOAT_B), 1.0);
    }

    /// Doubles for everything `Sky::Update` touches outside this file, for a
    /// sky in mode 0 (fixed colours) with a player in a cell.
    fn update_engine() -> Engine {
        let mut e = with_color_helpers(with_data(engine()));
        install_vtable(&mut e);
        e.set_global(PLAYER_CHARACTER, 0x1000u32);
        e.register(PLAYER_PARENT_CELL, |_, _| ret(0x2000));
        e.register(CALENDAR_GET_HOUR, |_, _| ret_float(13.5));
        e.register(CAMERA_HOLDER_GET, |_, _| ret(0x3000));
        e.register(CAMERA_NODE, |_, _| ret(0x3100));
        e.register(NODE_WORLD_POSITION, |e, _| {
            let position = e.mem.alloc(16);
            e.mem.set_f32(position + 8, 50.0);
            ret(position)
        });
        e.register(RELEVANT_WATER_HEIGHT, |_, _| ret_float(10.0));
        e.register(WORLD_STATE_FLAG, |_, _| ret(0));
        e.register(FLAG_UNDERWATER_A, |_, _| ret(0));
        e.register(FLAG_UNDERWATER_B, |_, _| ret(0));
        e.register(SKY_UPDATE_TRANSITION, |_, _| Ret::default());
        e.register(SKY_UPDATE_UNKNOWN, |_, _| Ret::default());
        e.register(MOON_PHASE_APPLIES, |_, _| ret(1));
        e.register(FOG_COLOR_UPDATE, |_, _| Ret::default());
        e.register(CALENDAR_GET_DAYS, |_, _| ret(5));
        e.register(PRECIPITATION_UPDATE, |_, _| Ret::default());
        e.register(TICK_COUNT_GET, |_, _| ret(1500));
        // The flag byte at `0119f188` skips the camera-relative fog tail.
        e.set_global(0x0119_f188, 1u8);
        e
    }

    #[test]
    fn update_does_nothing_without_a_parent_cell() {
        let mut e = update_engine();
        e.register(PLAYER_PARENT_CELL, |_, _| ret(0));
        let sky = new_sky(&mut e);
        e.set(sky, Sky::uiFlags, 3);
        e.call_log = Some(vec![]);
        e.call(0x0063_ac70, &args![sky, 0.5f32]);
        let log = e.call_log.take().unwrap();
        assert_eq!(e.get(sky, Sky::uiFlags), 3);
        assert_eq!(e.get(sky, Sky::fCurrentGameHour), 0.0);
        assert!(!addresses(&log).contains(&CALENDAR_GET_HOUR));
    }

    #[test]
    fn update_runs_the_components_and_the_moon_phases() {
        let mut e = update_engine();
        let sky = new_sky(&mut e);
        e.set(sky, Sky::eMode, 0);
        e.set(sky, Sky::uiFlags, 0x23);
        let atmosphere = object_with_vtable(&mut e, 0x20);
        let masser = object_with_vtable(&mut e, 0x80);
        let secunda = object_with_vtable(&mut e, 0x80);
        let precipitation = object_with_vtable(&mut e, 0x20);
        e.set(sky, Sky::pAtmosphere, atmosphere);
        e.set(sky, Sky::pMasser, masser);
        e.set(sky, Sky::pSecunda, secunda);
        e.set(sky, Sky::pPrecip, precipitation);
        // Secunda already has a phase; Masser does not.
        e.mem.set_u32(secunda.addr() + 0x70, 7);
        e.set(sky, Sky::uiLastMoonPhaseUpdate, 4);
        e.call_log = Some(vec![]);
        e.call(0x0063_ac70, &args![sky, 0.5f32]);
        let log = e.call_log.take().unwrap();
        assert_eq!(e.get(sky, Sky::fCurrentGameHour), 13.5);
        // Camera at 50 above the water at 10: flag 4 is clear. Flags 1 and 2
        // are cleared at the end; the others stay.
        assert_eq!(e.get(sky, Sky::uiFlags), 0x20);
        // The day count (5) is newer than the last update (4).
        assert_eq!(e.get(sky, Sky::uiLastMoonPhaseUpdate), 5);
        assert_eq!(e.mem.u32(masser.addr() + 0x70), 1);
        assert_eq!(e.mem.u32(secunda.addr() + 0x70), 7);
        let dt = 0.5f32.to_bits();
        assert!(log.contains(&(SLOT_BASE + 0xc, vec![atmosphere.addr(), sky.addr(), dt])));
        assert!(log.contains(&(SLOT_BASE + 0xc, vec![masser.addr(), sky.addr(), dt])));
        assert!(log.contains(&(SLOT_BASE + 0xc, vec![secunda.addr(), sky.addr(), dt])));
        assert!(log.contains(&(PRECIPITATION_UPDATE, vec![precipitation.addr(), dt])));
        assert!(log.contains(&(FOG_COLOR_UPDATE, vec![sky.addr() + 0x48])));
    }

    #[test]
    fn update_sets_the_underwater_flag_and_fades_the_flash() {
        let mut e = update_engine();
        e.register(RELEVANT_WATER_HEIGHT, |_, _| ret_float(80.0));
        // The flash lasts 2 seconds; 1500 - 500 ms have passed.
        e.set_global(FLASH_DURATION_SETTING, 2.0f32);
        let sky = new_sky(&mut e);
        e.set(sky, Sky::fFlash, 1.0);
        e.set(sky, Sky::uiFlashTime, 500);
        e.call(0x0063_ac70, &args![sky, 0.1f32]);
        assert_eq!(e.get(sky, Sky::uiFlags) & 4, 4);
        assert_eq!(e.get(sky, Sky::fFlash), 0.5);
        // Once the flash has run out it is cleared.
        e.set(sky, Sky::fFlash, 1.0);
        e.set(sky, Sky::uiFlashTime, 1500u32.wrapping_sub(2500));
        e.call(0x0063_ac70, &args![sky, 0.1f32]);
        assert_eq!(e.get(sky, Sky::fFlash), 0.0);
    }

    #[test]
    fn update_in_mode_1_without_the_world_flag_takes_the_short_path() {
        let mut e = update_engine();
        let sky = new_sky(&mut e);
        e.set(sky, Sky::eMode, 1);
        let atmosphere = object_with_vtable(&mut e, 0x20);
        let sun = object_with_vtable(&mut e, 0x30);
        let stars = object_with_vtable(&mut e, 0x20);
        e.set(sky, Sky::pAtmosphere, atmosphere);
        e.set(sky, Sky::pSun, sun);
        e.set(sky, Sky::pStars, stars);
        e.call_log = Some(vec![]);
        e.call(0x0063_ac70, &args![sky, 0.25f32]);
        let log = e.call_log.take().unwrap();
        let dt = 0.25f32.to_bits();
        // Only the atmosphere and the sun are updated.
        assert!(log.contains(&(SLOT_BASE + 0xc, vec![atmosphere.addr(), sky.addr(), dt])));
        assert!(log.contains(&(SLOT_BASE + 0xc, vec![sun.addr(), sky.addr(), dt])));
        assert!(!log
            .iter()
            .any(|(_, args)| args.first() == Some(&stars.addr())));
        assert!(!addresses(&log).contains(&SKY_UPDATE_TRANSITION));
        assert!(log.contains(&(FOG_COLOR_UPDATE, vec![sky.addr() + 0x48])));
    }

    #[test]
    fn short_update_updates_colours_fog_atmosphere_and_sun() {
        let mut e = update_engine();
        let sky = new_sky(&mut e);
        e.set(sky, Sky::eMode, 0);
        let sun = object_with_vtable(&mut e, 0x30);
        e.set(sky, Sky::pSun, sun);
        e.call_log = Some(vec![]);
        e.call(0x0063_b0a0, &args![sky, 1.5f32]);
        let log = e.call_log.take().unwrap();
        assert!(log.contains(&(
            SLOT_BASE + 0xc,
            vec![sun.addr(), sky.addr(), 1.5f32.to_bits()]
        )));
        // Colours and fog were computed: mode 0 copies the fixed colour and
        // the fog is the default (power 1.0).
        assert_eq!(e.get(sky, Sky::fFogPower), 1.0);
        assert_eq!(
            log.last().unwrap(),
            &(FOG_COLOR_UPDATE, vec![sky.addr() + 0x48])
        );
    }

    #[test]
    fn clouds_layer_colour_is_copied_into_its_slot() {
        let mut e = engine();
        let clouds: Ptr = Ptr::new(e.mem.alloc(0x5c));
        let color = e.mem.alloc(12);
        for (i, value) in [0.25f32, 0.5, 0.75].iter().enumerate() {
            e.mem.set_f32(color + 4 * i as u32, *value);
        }
        e.call(0x0063_b600, &args![clouds, 2i32, color]);
        let slot = clouds.addr() + 0x28 + 2 * 12;
        assert_eq!(e.mem.f32(slot), 0.25);
        assert_eq!(e.mem.f32(slot + 4), 0.5);
        assert_eq!(e.mem.f32(slot + 8), 0.75);
    }

    type CallLog = Vec<(u32, Vec<u32>)>;

    /// Runs `fn_0063b630` for `hour` with sunrise 6..10, sunset 18..22, high
    /// noon 13 and a weather percentage of 0.5; returns the two indices, the
    /// four weights and the engine's call log.
    fn transition_at(hour: f32) -> (u32, u32, [f32; 4], CallLog) {
        let mut e = with_data(engine());
        e.register(SKY_SUNRISE_END, |_, _| ret_float(10.0));
        e.register(SKY_SUNSET_BEGIN, |_, _| ret_float(18.0));
        e.register(LOG_MASTERFILE_ERROR, |_, _| Ret::default());
        e.set_global(SUNRISE_BEGIN_CACHE, 6.0f32);
        e.set_global(SUNSET_END_CACHE, 22.0f32);
        let sky = new_sky(&mut e);
        e.set(sky, Sky::fCurrentGameHour, hour);
        e.set(sky, Sky::fHighNoon, 13.0);
        e.set(sky, Sky::fCurrentWeatherPct, 0.5);
        let sample = e.mem.alloc(0x20);
        let first = e.mem.alloc(4);
        let second = e.mem.alloc(4);
        e.call_log = Some(vec![]);
        e.call(
            0x0063_b630,
            &args![sky, sample, 0x1000u32, 0u32, first, second],
        );
        let log = e.call_log.take().unwrap();
        let weights = [
            e.mem.f32(sample + 0x10),
            e.mem.f32(sample + 0x14),
            e.mem.f32(sample + 0x18),
            e.mem.f32(sample + 0x1c),
        ];
        (e.mem.u32(first), e.mem.u32(second), weights, log)
    }

    #[test]
    fn transition_sunrise_first_half_blends_from_night() {
        // Hour 6.5 is a quarter of the way through the sunrise (6..10): the
        // middle is 8, so the blend is 1 - (8 - 6.5) / 2 = 0.25 towards set 3.
        let (first, second, weights, _) = transition_at(6.5);
        assert_eq!((first, second), (0, 3));
        // Weights: blend and its complement scaled by 0.5, then by 1 - 0.5.
        assert_eq!(weights, [0.125, 0.375, 0.125, 0.375]);
    }

    #[test]
    fn transition_sunrise_second_half_blends_to_day() {
        let (first, second, weights, _) = transition_at(9.0);
        assert_eq!((first, second), (0, 1));
        assert_eq!(weights, [0.25, 0.25, 0.25, 0.25]);
    }

    #[test]
    fn transition_morning_runs_from_set_4_to_set_1() {
        // 11 is between sunrise end (10) and high noon (13): 1 - 2/3.
        let (first, second, weights, _) = transition_at(11.0);
        assert_eq!((first, second), (4, 1));
        let blend = (1.0f64 - 2.0 / 3.0) as f32;
        assert_eq!(weights[0], (blend as f64 * 0.5) as f32);
    }

    #[test]
    fn transition_afternoon_runs_from_set_1_to_set_4() {
        // 15 is between high noon (13) and sunset begin (18): 1 - 3/5.
        let (first, second, weights, _) = transition_at(15.0);
        assert_eq!((first, second), (1, 4));
        assert_eq!(weights[0], (0.4f64 * 0.5) as f32);
    }

    #[test]
    fn transition_sunset_blends_towards_night() {
        let (first, second, _, _) = transition_at(19.0);
        assert_eq!((first, second), (2, 1));
        let (first, second, weights, _) = transition_at(20.0);
        assert_eq!((first, second), (2, 3));
        // The middle of the sunset is 20: the blend is exactly 1.
        assert_eq!(weights, [0.5, 0.0, 0.5, 0.0]);
    }

    #[test]
    fn transition_at_night_uses_set_3_for_both() {
        for hour in [23.0f32, 3.0] {
            let (first, second, weights, log) = transition_at(hour);
            assert_eq!((first, second), (3, 3));
            assert_eq!(weights, [0.5, 0.0, 0.5, 0.0]);
            assert!(!addresses(&log).contains(&LOG_MASTERFILE_ERROR));
        }
    }

    #[test]
    fn transition_with_an_invalid_hour_logs_the_data_error() {
        let (first, second, weights, log) = transition_at(f32::NAN);
        assert_eq!((first, second), (1, 1));
        assert_eq!(weights, [0.5, 0.0, 0.5, 0.0]);
        assert!(log.contains(&(LOG_MASTERFILE_ERROR, vec![TRANSITION_TIMES_MESSAGE])));
    }

    #[test]
    fn transition_without_a_weather_does_nothing() {
        let mut e = with_data(engine());
        let sky = new_sky(&mut e);
        let sample = e.mem.alloc(0x20);
        let first = e.mem.alloc(4);
        e.mem.set_u32(first, 99);
        e.call_log = Some(vec![]);
        e.call(
            0x0063_b630,
            &args![sky, sample, 0u32, 0u32, first, 0x8000_0000u32],
        );
        assert_eq!(e.call_log.take().unwrap().len(), 1);
        assert_eq!(e.mem.u32(first), 99);
    }

    #[test]
    fn sunrise_begin_is_cached_until_the_climate_changes() {
        let mut e = with_data(engine());
        e.register(MAX_FLOAT, max_float);
        e.register(SETTING_FLOAT_VALUE, |_, a| ret(a[0]));
        e.register(SKY_SUNRISE_BEGIN, |_, _| ret_float(7.0));
        e.set_global(TIME_SETTING, 1.0f32);
        let sky = new_sky(&mut e);
        e.set(sky, Sky::uiFlags, 0x1000 | 0x20);
        assert_eq!(e.call(0x0063_b9b0, &args![sky]).f32(), 6.0);
        // The flag is cleared and the value cached.
        assert_eq!(e.get(sky, Sky::uiFlags), 0x20);
        assert_eq!(e.global::<f32>(SUNRISE_BEGIN_CACHE), 6.0);
        e.register(SKY_SUNRISE_BEGIN, |_, _| panic!("recomputed"));
        assert_eq!(e.call(0x0063_b9b0, &args![sky]).f32(), 6.0);
        // A start before the transition margin is clamped to 0.
        e.register(SKY_SUNRISE_BEGIN, |_, _| ret_float(0.5));
        e.set(sky, Sky::uiFlags, 0x1000);
        assert_eq!(e.call(0x0063_b9b0, &args![sky]).f32(), 0.0);
    }

    #[test]
    fn sunset_end_is_cached_and_limited() {
        let mut e = with_data(engine());
        e.register(MIN_FLOAT, min_float);
        e.register(SETTING_FLOAT_VALUE, |_, a| ret(a[0]));
        e.register(SKY_SUNSET_END, |_, _| ret_float(21.0));
        e.set_global(TIME_SETTING, 2.0f32);
        e.set_global(0x0104_ede4, 22.0f32);
        let sky = new_sky(&mut e);
        e.set(sky, Sky::uiFlags, 0x2000);
        // 21 + 2 is limited to 22.
        assert_eq!(e.call(0x0063_ba30, &args![sky]).f32(), 22.0);
        assert_eq!(e.get(sky, Sky::uiFlags), 0);
        e.register(SKY_SUNSET_END, |_, _| ret_float(17.0));
        e.set(sky, Sky::uiFlags, 0x2000);
        assert_eq!(e.call(0x0063_ba30, &args![sky]).f32(), 19.0);
        // Without the flag the cache is returned.
        e.register(SKY_SUNSET_END, |_, _| panic!("recomputed"));
        assert_eq!(e.call(0x0063_ba30, &args![sky]).f32(), 19.0);
    }

    /// A weather-sized block whose colour rows hold `row * 100 + column`.
    fn weather_with_colors(e: &mut Engine) -> Ptr {
        let weather = e.mem.alloc(0x400);
        for row in 0..6u32 {
            for column in 0..6u32 {
                e.mem.set_u32(
                    weather + 0x108 + row * 0x18 + column * 4,
                    row * 100 + column,
                );
            }
        }
        Ptr::new(weather)
    }

    #[test]
    fn weather_colour_accessor_indexes_rows_and_columns() {
        let mut e = engine();
        let weather = weather_with_colors(&mut e);
        assert_eq!(e.call(0x0063_bb40, &args![weather, 3i32, 2i32]).u32(), 302);
        assert_eq!(e.call(0x0063_bb40, &args![weather, 0i32, 5i32]).u32(), 5);
    }

    #[test]
    fn weather_colours_are_read_for_both_weathers() {
        let mut e = engine();
        let weather_a = weather_with_colors(&mut e);
        let weather_b = weather_with_colors(&mut e);
        e.mem.set_u32(weather_b.addr() + 0x108 + 2 * 0x18 + 4, 999);
        let out = e.mem.alloc(16);
        let indices = e.mem.alloc(8);
        e.mem.set_u32(indices, 1);
        e.mem.set_u32(indices + 4, 4);
        let call = |e: &mut Engine, a: Ptr, b: Ptr| {
            e.call(
                0x0063_bab0,
                &args![0x1234u32, out, a, b, 2i32, indices, indices + 4],
            );
            [
                e.mem.u32(out),
                e.mem.u32(out + 4),
                e.mem.u32(out + 8),
                e.mem.u32(out + 12),
            ]
        };
        // Row 2, columns 1 and 4 of both weathers (weather B differs at column 1).
        assert_eq!(call(&mut e, weather_a, weather_b), [201, 204, 999, 204]);
        // Without the second weather its half is zero.
        e.mem.set_u32(out + 8, 5);
        assert_eq!(call(&mut e, weather_a, Ptr::NULL), [201, 204, 0, 0]);
        // Without the first weather nothing is written.
        e.mem.set_u32(out, 77);
        call(&mut e, Ptr::NULL, weather_b);
        assert_eq!(e.mem.u32(out), 77);
    }

    #[test]
    fn cloud_colour_accessor_uses_defaults_and_layers() {
        let mut e = engine();
        let weather: u32 = e.mem.alloc(0x400);
        // No layers: the built-in colours.
        for (time, expected) in [(1, 0x9b9b9b), (3, 0x1c140f), (0, 0x5b4b46), (2, 0x6c5953)] {
            assert_eq!(
                e.call(0x0063_bc60, &args![weather, 0i32, time]).u32(),
                expected
            );
        }
        // Two layers of colours at +0x68, 0x18 apart.
        e.mem.set_i32(weather + 0x368, 2);
        e.mem.set_u32(weather + 0x68 + 4, 0x111111);
        e.mem.set_u32(weather + 0x68 + 0x18 + 8, 0x222222);
        assert_eq!(
            e.call(0x0063_bc60, &args![weather, 0i32, 1i32]).u32(),
            0x111111
        );
        assert_eq!(
            e.call(0x0063_bc60, &args![weather, 1i32, 2i32]).u32(),
            0x222222
        );
        // A layer past the count uses layer 0.
        assert_eq!(
            e.call(0x0063_bc60, &args![weather, 7i32, 1i32]).u32(),
            0x111111
        );
    }

    #[test]
    fn cloud_colours_fall_back_to_the_other_weather() {
        let mut e = engine();
        let weather_a = e.mem.alloc(0x400);
        let weather_b = e.mem.alloc(0x400);
        for weather in [weather_a, weather_b] {
            e.mem.set_i32(weather + 0x368, 1);
        }
        // Weather A's colours: [1] = 0x10, rest 0; weather B's: [0] = 0x20.
        e.mem.set_u32(weather_a + 0x68 + 4, 0x10);
        e.mem.set_u32(weather_b + 0x68, 0x20);
        let out = e.mem.alloc(16);
        let indices = e.mem.alloc(8);
        e.mem.set_u32(indices, 0);
        e.mem.set_u32(indices + 4, 1);
        e.call(
            0x0063_bb70,
            &args![0u32, out, weather_a, weather_b, 0i32, indices, indices + 4],
        );
        // out = [A0, A1, B0, B1] = [0, 0x10, 0x20, 0]; zero entries are
        // replaced by the entry of the other weather at the same slot.
        assert_eq!(
            [
                e.mem.u32(out),
                e.mem.u32(out + 4),
                e.mem.u32(out + 8),
                e.mem.u32(out + 12)
            ],
            [0x20, 0x10, 0x20, 0x10]
        );
        // Without the second weather its half is zero and nothing is replaced.
        e.call(
            0x0063_bb70,
            &args![0u32, out, weather_a, 0u32, 0i32, indices, indices + 4],
        );
        assert_eq!(
            [
                e.mem.u32(out),
                e.mem.u32(out + 4),
                e.mem.u32(out + 8),
                e.mem.u32(out + 12)
            ],
            [0, 0x10, 0, 0]
        );
        // Without the first weather nothing is written.
        e.mem.set_u32(out, 77);
        e.call(
            0x0063_bb70,
            &args![0u32, out, 0u32, weather_b, 0i32, indices, indices + 4],
        );
        assert_eq!(e.mem.u32(out), 77);
    }

    fn color_at(e: &Engine, address: u32) -> [f32; 3] {
        [
            e.mem.f32(address),
            e.mem.f32(address + 4),
            e.mem.f32(address + 8),
        ]
    }

    fn set_color(e: &mut Engine, address: u32, color: [f32; 3]) {
        for (i, value) in color.iter().enumerate() {
            e.mem.set_f32(address + 4 * i as u32, *value);
        }
    }

    #[test]
    fn colours_in_mode_0_are_the_fixed_ones_and_chain_down() {
        let mut e = with_color_helpers(with_data(engine()));
        e.register(CLOUDS_LAYER_COUNT, |_, _| ret(2));
        for (address, value) in [
            (0x0101_ffc0, 0.9f32),
            (0x0101_9de0, 0.8),
            (0x0102_1f44, 0.3),
            (0x0104_ee30, 0.4),
            (0x0101_e2bc, 0.1),
        ] {
            e.set_global(address, value);
        }
        e.set_global(0x011f_4998, 0.25f32);
        e.set_global(0x011f_499c, 0.5f32);
        e.set_global(0x011f_49a0, 0.75f32);
        let sky = new_sky(&mut e);
        let clouds = e.mem.alloc(0x5c);
        e.set(sky, Sky::pClouds, Ptr::new(clouds));
        e.call(0x0063_b120, &args![sky]);
        let base = sky.addr();
        assert_eq!(color_at(&e, base + 0x6c), [0.9, 0.8, 0.8]);
        assert_eq!(color_at(&e, base + 0x60), [0.3, 0.3, 0.4]);
        assert_eq!(color_at(&e, clouds + 0x28), [0.1, 0.1, 0.1]);
        assert_eq!(color_at(&e, clouds + 0x34), [0.1, 0.1, 0.1]);
        // The exe's colour is copied down the chain.
        for offset in [0xc0, 0x9c, 0x78, 0x3c, 0x48, 0x90] {
            assert_eq!(
                color_at(&e, base + offset),
                [0.25, 0.5, 0.75],
                "{offset:#x}"
            );
        }
    }

    #[test]
    fn colours_in_an_interior_come_from_the_cell() {
        let mut e = with_color_helpers(with_data(engine()));
        e.register(CLOUDS_LAYER_COUNT, |_, _| ret(1));
        e.set_global(INTERIOR_CELL_HOLDER, 0x5000u32);
        e.register(GET_INTERIOR_CELL, |_, _| ret(0x6000));
        e.register(CELL_COLOR_FIRST, |e, a| {
            set_color(e, a[1], [1.0, 2.0, 3.0]);
            Ret::default()
        });
        e.register(CELL_COLOR_SECOND, |e, a| {
            set_color(e, a[1], [4.0, 5.0, 6.0]);
            Ret::default()
        });
        e.register(CELL_COLOR_THIRD, |e, a| {
            set_color(e, a[1], [7.0, 8.0, 9.0]);
            Ret::default()
        });
        let sky = new_sky(&mut e);
        e.set(sky, Sky::eMode, 1);
        let clouds = e.mem.alloc(0x5c);
        e.set(sky, Sky::pClouds, Ptr::new(clouds));
        e.call(0x0063_b120, &args![sky]);
        let base = sky.addr();
        assert_eq!(color_at(&e, base + 0x6c), [1.0, 2.0, 3.0]);
        assert_eq!(color_at(&e, base + 0x60), [4.0, 5.0, 6.0]);
        for offset in [0x48, 0xc0, 0x9c, 0x78, 0x3c, 0x90] {
            assert_eq!(color_at(&e, base + offset), [7.0, 8.0, 9.0], "{offset:#x}");
        }
        assert_eq!(color_at(&e, clouds + 0x28), [7.0, 8.0, 9.0]);
    }

    #[test]
    fn colours_with_a_weather_mix_the_weather_table() {
        let mut e = with_color_helpers(with_data(engine()));
        e.register(SKY_SUNRISE_END, |_, _| ret_float(10.0));
        e.register(SKY_SUNSET_BEGIN, |_, _| ret_float(18.0));
        e.set_global(SUNRISE_BEGIN_CACHE, 6.0f32);
        e.set_global(SUNSET_END_CACHE, 22.0f32);
        e.set_global(0x0102_31e8, 1.0f64 / 255.0);
        // Every colour of the weather is white.
        let weather = e.mem.alloc(0x400);
        for row in 0..10 {
            for column in 0..6 {
                e.mem
                    .set_u32(weather + 0x108 + row * 0x18 + column * 4, 0x00ff_ffff);
            }
        }
        let sky = new_sky(&mut e);
        e.set(sky, Sky::pCurrentWeather, Ptr::new(weather));
        e.set(sky, Sky::eMode, 3);
        e.set(sky, Sky::fCurrentGameHour, 12.0);
        e.set(sky, Sky::fHighNoon, 13.0);
        e.set(sky, Sky::fCurrentWeatherPct, 1.0);
        e.call(0x0063_b120, &args![sky]);
        for color in 0..10 {
            let value = color_at(&e, sky.addr() + 0x3c + color * 12);
            for channel in value {
                assert!((channel - 1.0).abs() < 1e-5, "{color}: {value:?}");
            }
        }
        // The specular colour is a copy of colour 3.
        assert_eq!(
            color_at(&e, sky.addr() + 0xc0),
            color_at(&e, sky.addr() + 0x6c)
        );
    }

    #[test]
    fn colours_with_a_weather_add_lightning_to_colour_4_and_the_clouds() {
        let mut e = with_color_helpers(with_data(engine()));
        e.register(SKY_SUNRISE_END, |_, _| ret_float(10.0));
        e.register(SKY_SUNSET_BEGIN, |_, _| ret_float(18.0));
        e.register(CLOUDS_LAYER_COUNT, |_, _| ret(1));
        e.register(WEATHER_BYTE_FRACTION, |_, _| ret_float(2.0));
        e.set_global(SUNRISE_BEGIN_CACHE, 6.0f32);
        e.set_global(SUNSET_END_CACHE, 22.0f32);
        e.set_global(0x0102_31e8, 0.0f64);
        e.set_global(LIGHTNING_SCALE_COLOR_3, 0.5f32);
        e.set_global(LIGHTNING_SCALE_COLOR_4, 0.25f32);
        let weather = e.mem.alloc(0x400);
        let clouds = e.mem.alloc(0x5c);
        let sky = new_sky(&mut e);
        e.set(sky, Sky::pClouds, Ptr::new(clouds));
        e.set(sky, Sky::pCurrentWeather, Ptr::new(weather));
        e.set(sky, Sky::eMode, 3);
        e.set(sky, Sky::fCurrentGameHour, 12.0);
        e.set(sky, Sky::fHighNoon, 13.0);
        e.set(sky, Sky::fFlash, 0.5);
        e.call(0x0063_b120, &args![sky]);
        // The channels are zero before lightning; colour 3 gets 0.5 * 0.5,
        // colour 4 gets 0.5 * 0.25, the first cloud layer gets the flash.
        assert_eq!(color_at(&e, sky.addr() + 0x3c + 3 * 12), [0.25; 3]);
        assert_eq!(color_at(&e, sky.addr() + 0x3c + 4 * 12), [0.125; 3]);
        assert_eq!(color_at(&e, sky.addr() + 0x3c + 2 * 12), [0.0; 3]);
        assert_eq!(color_at(&e, clouds + 0x28), [0.5; 3]);
    }

    /// Doubles for `Sky::fn_0063bce0` and a sky in mode 3 whose camera is at
    /// height 50 and whose water volume is absent.
    fn fog_engine() -> Engine {
        let mut e = with_color_helpers(with_data(engine()));
        install_vtable(&mut e);
        e.set_global(PLAYER_CHARACTER, 0x1000u32);
        e.register(PLAYER_PARENT_CELL, |_, _| ret(0x2000));
        e.register(FLAG_UNDERWATER_A, |_, _| ret(0));
        e.register(FLAG_UNDERWATER_B, |_, _| ret(0));
        e.register(CAMERA_HOLDER_GET, |e, _| {
            let camera = object_with_vtable(e, 0x20);
            ret(camera.addr())
        });
        e.register(CAMERA_NODE, |_, _| ret(0x3100));
        e.register(NODE_WORLD_POSITION, |e, _| {
            let position = e.mem.alloc(16);
            e.mem.set_f32(position + 8, 50.0);
            ret(position)
        });
        // The camera-relative tail is skipped by default.
        e.set_global(0x0119_f188, 1u8);
        e
    }

    #[test]
    fn fog_underwater_comes_from_the_water_volume() {
        let mut e = fog_engine();
        e.register(FLAG_UNDERWATER_A, |_, _| ret(1));
        e.register(CELL_WATER_HEIGHT, |_, _| ret_float(80.0));
        e.register(WATER_FOG_NEAR, |_, _| ret_float(10.0));
        e.register(WATER_FOG_FAR, |_, _| ret_float(500.0));
        e.register(WATER_COLOR_A, |_, _| ret(0x0000_00ff));
        e.register(WATER_COLOR_B, |_, _| ret(0));
        e.register(CLOUDS_LAYER_COUNT, |_, _| ret(1));
        e.register(SKY_GET_CLOUDS, |e, a| ret(e.mem.u32(a[0] + 0x2c)));
        e.register(SKY_GET_ATMOSPHERE, |e, a| ret(e.mem.u32(a[0] + 0x20)));
        e.register(SKY_GET_SUN, |e, a| ret(e.mem.u32(a[0] + 0x28)));
        e.register(HOLDER_NODE, |e, a| ret(e.mem.u32(a[0] + 8)));
        e.register(SUN_GET_NODE, |e, a| ret(e.mem.u32(a[0] + 0xc)));
        e.register(SET_NODE_HIDDEN, |_, _| Ret::default());
        e.set_global(0x011c_7a3c, 0x7000u32);
        e.set_global(0x011c_7a5c, 100.0f32);
        e.set_global(WATER_DEPTH_SCALE, 0.01f32);
        e.set_global(0x0102_31e8, 1.0f64 / 255.0);
        e.register(WATER_FOG_NEAR, |_, a| {
            assert_eq!(a[0], 0x7000);
            ret_float(10.0)
        });
        let sky = new_sky(&mut e);
        e.set(sky, Sky::eMode, 3);
        // Components with the node pointer at +8 (+0xc for the sun).
        let (clouds, atmosphere, sun) = (e.mem.alloc(0x60), e.mem.alloc(0x30), e.mem.alloc(0x30));
        e.mem.set_u32(clouds + 8, 0xc100);
        e.mem.set_u32(atmosphere + 8, 0xa100);
        e.mem.set_u32(sun + 8, 0x5100);
        e.mem.set_u32(sun + 0xc, 0x5200);
        e.set(sky, Sky::pClouds, Ptr::new(clouds));
        e.set(sky, Sky::pSun, Ptr::new(sun));
        e.mem.set_u32(sky.addr() + 0x20, atmosphere);
        e.mem.set_u32(sky.addr() + 4, 0x4100);
        e.call_log = Some(vec![]);
        e.call(0x0063_bce0, &args![sky]);
        let log = e.call_log.take().unwrap();
        assert_eq!(e.get(sky, Sky::fFogNear), 10.0);
        assert_eq!(e.get(sky, Sky::fFogFar), 500.0);
        assert_eq!(e.get(sky, Sky::fFogPower), 1.0);
        // The camera is 30 below the cell's water: depth 0.3, so colour
        // channel 0 is 0.7 (the water colour is red) and nothing is hidden.
        let color = color_at(&e, sky.addr() + 0x48);
        assert!((color[0] - 0.7).abs() < 1e-6 && color[1] == 0.0 && color[2] == 0.0);
        let hidden: Vec<_> = log
            .iter()
            .filter(|(address, _)| *address == SET_NODE_HIDDEN)
            .map(|(_, args)| args.clone())
            .collect();
        assert_eq!(
            hidden,
            [
                vec![0xc100, 0],
                vec![0xa100, 0],
                vec![0x5100, 0],
                vec![0x5200, 0],
                vec![0x4100, 0]
            ]
        );
        // The same, deeper than half way: everything is hidden.
        e.set_global(WATER_DEPTH_SCALE, 0.02f32);
        e.call_log = Some(vec![]);
        e.call(0x0063_bce0, &args![sky]);
        let log = e.call_log.take().unwrap();
        assert!(log
            .iter()
            .filter(|(address, _)| *address == SET_NODE_HIDDEN)
            .all(|(_, args)| args[1] == 1));
    }

    #[test]
    fn fog_from_the_weather_mixes_by_the_hour_and_the_last_weather() {
        let mut e = fog_engine();
        e.register(SKY_SUNRISE_END, |_, _| ret_float(10.0));
        e.register(SKY_SUNSET_BEGIN, |_, _| ret_float(18.0));
        e.set_global(SUNRISE_BEGIN_CACHE, 6.0f32);
        e.set_global(SUNSET_END_CACHE, 22.0f32);
        let weather = e.mem.alloc(0x400);
        let last = e.mem.alloc(0x400);
        for (index, value) in [100.0f32, 200.0, 300.0, 400.0, 2.0, 4.0].iter().enumerate() {
            e.mem.set_f32(weather + 0xf0 + 4 * index as u32, *value);
        }
        for (index, value) in [40.0f32, 0.0, 40.0, 0.0, 0.0, 0.0].iter().enumerate() {
            e.mem.set_f32(last + 0xf0 + 4 * index as u32, *value);
        }
        let sky = new_sky(&mut e);
        e.set(sky, Sky::pCurrentWeather, Ptr::new(weather));
        e.set(sky, Sky::eMode, 3);
        // Midday: the daytime values (indices 0, 1, 4).
        e.set(sky, Sky::fCurrentGameHour, 12.0);
        e.call(0x0063_bce0, &args![sky]);
        assert_eq!(e.get(sky, Sky::fFogNear), 100.0);
        assert_eq!(e.get(sky, Sky::fFogFar), 200.0);
        assert_eq!(e.get(sky, Sky::fFogPower), 2.0);
        // 8 o'clock is half way through the sunrise: the mean of both.
        e.set(sky, Sky::fCurrentGameHour, 8.0);
        e.call(0x0063_bce0, &args![sky]);
        assert_eq!(e.get(sky, Sky::fFogNear), 200.0);
        assert_eq!(e.get(sky, Sky::fFogFar), 300.0);
        assert_eq!(e.get(sky, Sky::fFogPower), 3.0);
        // Half way into a change of weather the last weather's mix (near 40,
        // others 0) is added with weight 0.5, the new weather's halved.
        e.set(sky, Sky::pLastWeather, Ptr::new(last));
        e.set(sky, Sky::fCurrentWeatherPct, 0.5);
        e.call(0x0063_bce0, &args![sky]);
        assert_eq!(e.get(sky, Sky::fFogNear), 120.0);
        assert_eq!(e.get(sky, Sky::fFogFar), 150.0);
        assert_eq!(e.get(sky, Sky::fFogPower), 1.5);
        // At night the nighttime values apply.
        e.set(sky, Sky::pLastWeather, Ptr::NULL);
        e.set(sky, Sky::fCurrentGameHour, 23.0);
        e.call(0x0063_bce0, &args![sky]);
        assert_eq!(e.get(sky, Sky::fFogNear), 300.0);
        assert_eq!(e.get(sky, Sky::fFogPower), 4.0);
    }

    #[test]
    fn fog_in_an_interior_comes_from_the_cell_with_limits() {
        let mut e = fog_engine();
        e.set_global(INTERIOR_CELL_HOLDER, 0x5000u32);
        e.set_global(DEFAULT_FOG_DISTANCE, 163840.0f32);
        e.set_global(FOG_FAR_LIMIT, 163840.0f64);
        e.set_global(FOG_NEAR_FACTOR, 0.5f64);
        e.register(GET_INTERIOR_CELL, |_, _| ret(0x6000));
        e.register(CELL_FOG_FAR, |_, _| ret_float(300.0));
        e.register(CELL_FOG_NEAR, |_, _| ret_float(50.0));
        e.register(CELL_FOG_POWER, |_, _| ret_float(1.5));
        let sky = new_sky(&mut e);
        e.set(sky, Sky::eMode, 1);
        e.call(0x0063_bce0, &args![sky]);
        assert_eq!(e.get(sky, Sky::fFogNear), 50.0);
        assert_eq!(e.get(sky, Sky::fFogFar), 300.0);
        assert_eq!(e.get(sky, Sky::fFogPower), 1.5);
        // A far distance of 0 becomes the default; a near distance of 0, or
        // past the far one, becomes the far one times the factor.
        e.register(CELL_FOG_FAR, |_, _| ret_float(0.0));
        e.register(CELL_FOG_NEAR, |_, _| ret_float(0.0));
        e.call(0x0063_bce0, &args![sky]);
        assert_eq!(e.get(sky, Sky::fFogFar), 163840.0);
        assert_eq!(e.get(sky, Sky::fFogNear), 81920.0);
        // Past the limit also becomes the default.
        e.register(CELL_FOG_FAR, |_, _| ret_float(200000.0));
        e.register(CELL_FOG_NEAR, |_, _| ret_float(1000.0));
        e.call(0x0063_bce0, &args![sky]);
        assert_eq!(e.get(sky, Sky::fFogFar), 163840.0);
        assert_eq!(e.get(sky, Sky::fFogNear), 1000.0);
    }

    #[test]
    fn fog_defaults_and_the_camera_tail() {
        let mut e = fog_engine();
        e.set_global(DEFAULT_FOG_DISTANCE, 1000.0f32);
        e.set_global(FOG_SETTING, 10.0f32);
        let sky = new_sky(&mut e);
        e.set(sky, Sky::eMode, 2);
        e.call(0x0063_bce0, &args![sky]);
        assert_eq!(
            (e.get(sky, Sky::fFogNear), e.get(sky, Sky::fFogFar)),
            (1000.0, 1000.0)
        );
        assert_eq!(e.get(sky, Sky::fFogPower), 1.0);
        // Without the flag byte the far distance is pulled to the camera's
        // far plane (slot 0x100 returns 400) and the near one rescaled.
        e.set_global(0x0119_f188, 0u8);
        e.register(SLOT_BASE + 0x100, |_, _| ret_float(400.0));
        e.call(0x0063_bce0, &args![sky]);
        let ratio = ((1000.0f64 - 400.0) / (1000.0 - 10.0)) as f32;
        let pulled = (1000.0f64 - (1000.0 - 10.0) * ratio as f64) as f32;
        assert_eq!(e.get(sky, Sky::fFogFar), 400.0);
        assert_eq!(e.get(sky, Sky::fFogNear), pulled.min(400.0));
    }

    #[test]
    fn weather_fog_accessor_reads_the_table() {
        let mut e = engine();
        let weather = e.mem.alloc(0x400);
        e.mem.set_f32(weather + 0xf0 + 4 * 3, 7.5);
        assert_eq!(e.call(0x0063_c420, &args![weather, 3i32]).f32(), 7.5);
    }

    #[test]
    fn clouds_layer_node_accessor_reads_the_pointer() {
        let mut e = engine();
        let clouds = e.mem.alloc(0x5c);
        e.mem.set_u32(clouds + 8 + 4 * 2, 0xbeef);
        assert_eq!(e.call(0x0063_c440, &args![clouds, 2i32]).u32(), 0xbeef);
    }

    #[test]
    fn global_accessors_return_their_globals() {
        let mut e = with_data(engine());
        e.set_global(0x0119_f188, 1u8);
        assert!(e.call(0x0063_c460, &[]).bool());
        e.set_global(0x0119_f188, 0u8);
        assert!(!e.call(0x0063_c460, &[]).bool());
        e.set_global(0x011c_7a3c, 0x1264u32);
        assert_eq!(e.call(0x0063_c470, &[]).u32(), 0x1264);
        e.set_global(0x011c_7a5c, 2.5f32);
        assert_eq!(e.call(0x0063_c480, &[]).f32(), 2.5);
    }

    fn wind_engine() -> Engine {
        let mut e = with_data(engine());
        e.register(WEATHER_BYTE_FRACTION, |_, a| {
            ret_float(if a[0] == 0x1000 { 0.6 } else { 0.2 })
        });
        e.register(0x0045_2e70, |_, _| ret_float(3600.0));
        e.register(0x0057_e960, |_, _| ret_float(0.5));
        e.register(FTOL, |_, a| ret(f64::take(a, &mut 0).trunc() as i32 as u32));
        e.set_global(0x0101_2640, 3600.0f64);
        e.set_global(0x0104_eea0, std::f64::consts::TAU);
        e.set_global(0x0101_e580, 15.0f32);
        e.set_global(0x0104_ee98, 0.7f64);
        e.set_global(0x0102_8338, 512.0f64);
        e.set_global(0x011c_cc50, 1.0f32);
        e.set_global(0x011a_9b50, 11u32);
        e.set_global(0x011a_9b54, 22u32);
        e
    }

    #[test]
    fn wind_follows_the_weather_and_publishes_the_gusts() {
        let mut e = wind_engine();
        let sky = new_sky(&mut e);
        e.set(sky, Sky::pCurrentWeather, Ptr::new(0x1000));
        e.set(sky, Sky::eMode, 3);
        // The phase is 2 pi * 15, which indexes the table at 126.
        e.set_global(0x011f_52e0 + 4 * 126, 0.25f32);
        e.call(0x0063_c490, &args![sky]);
        assert_eq!(e.get(sky, Sky::fWindSpeed), 0.6);
        assert_eq!(e.global::<f32>(0x011a_d824), 0.6);
        assert_eq!(e.global::<u32>(0x011f_95dc), 11);
        assert_eq!(e.global::<u32>(0x011f_95e0), 22);
        assert_eq!(e.global::<f32>(0x011f_d884), 3.75);
        assert_eq!(e.global::<f32>(0x011f_d888), 7.5);
        // With a last weather the speeds are mixed by the percentage:
        // 0.6 * 0.5 + 0.2 * 0.5.
        e.set(sky, Sky::pLastWeather, Ptr::new(0x2000));
        e.set(sky, Sky::fCurrentWeatherPct, 0.5);
        e.call(0x0063_c490, &args![sky]);
        assert!((e.get(sky, Sky::fWindSpeed) - 0.4).abs() < 1e-6);
    }

    #[test]
    fn wind_does_nothing_without_a_weather_or_in_the_wrong_mode() {
        let mut e = wind_engine();
        let sky = new_sky(&mut e);
        e.set(sky, Sky::eMode, 3);
        e.call_log = Some(vec![]);
        e.call(0x0063_c490, &args![sky]);
        e.set(sky, Sky::pCurrentWeather, Ptr::new(0x1000));
        e.set(sky, Sky::eMode, 1);
        e.call(0x0063_c490, &args![sky]);
        assert_eq!(e.call_log.take().unwrap().len(), 2);
        assert_eq!(e.get(sky, Sky::fWindSpeed), 0.0);
    }

    #[test]
    fn sine_lookup_indexes_the_table() {
        let mut e = wind_engine();
        e.set_global(0x011c_cc50, 2.0f32);
        e.register(FTOL, |_, a| ret(f64::take(a, &mut 0).trunc() as i32 as u32));
        e.set_global(0x011f_52e0 + 4 * 256, 0.75f32);
        // 512 / 2 * 1.0 = 256.
        assert_eq!(e.call(0x0063_c600, &args![1.0f32]).f32(), 0.75);
        // The index wraps at 512.
        e.set_global(0x011f_52e0 + 4 * 6, 0.5f32);
        assert_eq!(e.call(0x0063_c600, &args![2.0f32 + 3.0 / 128.0]).f32(), 0.5);
    }

    #[test]
    fn wind_helpers_store_their_arguments() {
        let mut e = with_data(engine());
        e.call(0x0063_c640, &args![5u32, 6u32]);
        assert_eq!(e.global::<u32>(0x011f_95dc), 5);
        assert_eq!(e.global::<u32>(0x011f_95e0), 6);
        e.call(0x0063_c660, &args![1.5f32]);
        assert_eq!(e.global::<f32>(0x011a_d824), 1.5);
        e.call(0x0063_c670, &args![2.5f32, 3.5f32]);
        assert_eq!(e.global::<f32>(0x011f_d884), 2.5);
        assert_eq!(e.global::<f32>(0x011f_d888), 3.5);
    }

    /// A mixing source: four packed colours and four weights.
    fn mixing_source(e: &mut Engine, colors: [u32; 4], weights: [f32; 4]) -> u32 {
        let source = e.mem.alloc(0x20);
        for i in 0..4 {
            e.mem.set_u32(source + 4 * i, colors[i as usize]);
            e.mem.set_f32(source + 0x10 + 4 * i, weights[i as usize]);
        }
        source
    }

    #[test]
    fn colour_mixing_weights_each_channel_and_clamps_to_one() {
        let mut e = with_color_helpers(with_data(engine()));
        e.set_global(0x0102_31e8, 0.5f64);
        let source = mixing_source(
            &mut e,
            [0x0003_0201, 0x0006_0504, 0, 0],
            [0.5, 0.25, 0.0, 0.0],
        );
        let sky = new_sky(&mut e);
        let dest = e.mem.alloc(12);
        e.call_log = Some(vec![]);
        e.call(0x0063_c690, &args![sky, dest, source, 0.0f32]);
        let log = e.call_log.take().unwrap();
        // Red: (1 * 0.5 + 4 * 0.25) * 0.5 = 0.75; green 1.125 and blue 1.5
        // are clamped to 1.
        assert_eq!(color_at(&e, dest), [0.75, 1.0, 1.0]);
        assert!(log.contains(&(COLOR_CLAMP_TO_ONE, vec![dest])));
    }

    #[test]
    fn colour_mixing_adds_lightning_and_clamps_to_the_weather() {
        let mut e = with_color_helpers(with_data(engine()));
        e.set_global(0x0102_31e8, 0.5f64);
        e.register(WEATHER_BYTE_FRACTION, |_, a| {
            assert_eq!((a[2], a[3]), (1.0f32.to_bits(), 0));
            ret_float(match a[1] {
                0xc => 0.9,
                0xd => 1.2,
                _ => 2.0,
            })
        });
        let source = mixing_source(
            &mut e,
            [0x0003_0201, 0x0006_0504, 0, 0],
            [0.5, 0.25, 0.0, 0.0],
        );
        let sky = new_sky(&mut e);
        e.set(sky, Sky::pCurrentWeather, Ptr::new(0x1000));
        let dest = e.mem.alloc(12);
        e.call_log = Some(vec![]);
        // Lightning above 1 is limited to 1: [1.75, 2.125, 2.5], then the
        // channels are limited to the weather's 0.9, 1.2 and 2.0.
        e.call(0x0063_c690, &args![sky, dest, source, 3.0f32]);
        let log = e.call_log.take().unwrap();
        assert_eq!(color_at(&e, dest), [0.9, 1.2, 2.0]);
        assert!(!addresses(&log).contains(&COLOR_CLAMP_TO_ONE));
        // Without a weather the lightning is added and clamped to one.
        e.set(sky, Sky::pCurrentWeather, Ptr::NULL);
        e.call(0x0063_c690, &args![sky, dest, source, 0.25f32]);
        assert_eq!(color_at(&e, dest), [1.0, 1.0, 1.0]);
    }

    #[test]
    fn colour_addition_adds_each_channel_and_returns_this() {
        let mut e = engine();
        let (first, second) = (e.mem.alloc(12), e.mem.alloc(12));
        set_color(&mut e, first, [0.5, 1.0, 1.5]);
        set_color(&mut e, second, [0.25, 0.25, 0.25]);
        let back = e.call(0x0063_c8a0, &args![first, second]).u32();
        assert_eq!(back, first);
        assert_eq!(color_at(&e, first), [0.75, 1.25, 1.75]);
        assert_eq!(color_at(&e, second), [0.25, 0.25, 0.25]);
    }

    fn climate_engine() -> Engine {
        let mut e = with_data(engine());
        e.register(LOOKUP_FORM, |_, a| {
            assert_eq!(a, [0x15f]);
            ret(0x9999)
        });
        e.register(DYNAMIC_CAST, |_, a| {
            assert_eq!(
                a,
                [0x9999, 0, TYPE_DESCRIPTOR_FORM, TYPE_DESCRIPTOR_CLIMATE, 0]
            );
            ret(0x4444)
        });
        e.register(SKY_RESET_WEATHER, |_, _| Ret::default());
        e
    }

    #[test]
    fn set_current_climate_looks_up_the_default_climate() {
        let mut e = climate_engine();
        let sky = new_sky(&mut e);
        e.call_log = Some(vec![]);
        e.call(0x0063_c8f0, &args![sky, 0u32, false]);
        let log = e.call_log.take().unwrap();
        assert_eq!(e.get(sky, Sky::pCurrentClimate), Ptr::new(0x4444));
        assert_eq!(e.get(sky, Sky::uiFlags), 0x3f40);
        assert!(log.contains(&(SKY_RESET_WEATHER, vec![sky.addr()])));
    }

    #[test]
    fn set_current_climate_keeps_the_same_climate_unless_forced() {
        let mut e = climate_engine();
        let sky = new_sky(&mut e);
        e.set(sky, Sky::pCurrentClimate, Ptr::new(0x1234));
        e.call_log = Some(vec![]);
        e.call(0x0063_c8f0, &args![sky, 0x1234u32, false]);
        assert_eq!(e.call_log.take().unwrap().len(), 3);
        assert_eq!(e.get(sky, Sky::uiFlags), 0);
        // A different climate is taken over.
        e.call(0x0063_c8f0, &args![sky, 0x5678u32, false]);
        assert_eq!(e.get(sky, Sky::pCurrentClimate), Ptr::new(0x5678));
        assert_eq!(e.get(sky, Sky::uiFlags), 0x3f40);
        // Forcing with no climate looks the default up again.
        e.set(sky, Sky::uiFlags, 0);
        e.call_log = Some(vec![]);
        e.call(0x0063_c8f0, &args![sky, 0u32, true]);
        let log = e.call_log.take().unwrap();
        assert_eq!(e.get(sky, Sky::pCurrentClimate), Ptr::new(0x4444));
        assert!(addresses(&log).contains(&LOOKUP_FORM));
        assert_eq!(e.get(sky, Sky::uiFlags), 0x3f40);
    }

    #[test]
    fn set_current_climate_does_nothing_when_no_climate_is_found() {
        let mut e = climate_engine();
        e.register(DYNAMIC_CAST, |_, _| ret(0));
        let sky = new_sky(&mut e);
        e.call(0x0063_c8f0, &args![sky, 0u32, false]);
        assert!(e.get(sky, Sky::pCurrentClimate).is_null());
        assert_eq!(e.get(sky, Sky::uiFlags), 0);
        // An existing climate and no argument: nothing is looked up.
        e.set(sky, Sky::pCurrentClimate, Ptr::new(0x1234));
        e.call_log = Some(vec![]);
        e.call(0x0063_c8f0, &args![sky, 0u32, false]);
        assert!(!addresses(&e.call_log.take().unwrap()).contains(&LOOKUP_FORM));
        assert_eq!(e.get(sky, Sky::pCurrentClimate), Ptr::new(0x1234));
    }

    #[test]
    fn climate_stale_flags_are_set() {
        let mut e = engine();
        let sky = new_sky(&mut e);
        e.set(sky, Sky::uiFlags, 0x41);
        e.call(0x0063_c9f0, &args![sky]);
        assert_eq!(e.get(sky, Sky::uiFlags), 0x3f41);
    }

    /// A sky in mode 3 with a sun, and a climate with two texture entries,
    /// with doubles for the texture loading.
    fn climate_update_engine() -> Engine {
        let mut e = with_color_helpers(with_data(engine()));
        install_vtable(&mut e);
        e.register(SLOT_BASE + 0x14, |_, _| ret(0x5150));
        e.register(SLOT_BASE + 0x18, |_, _| ret(0x6100));
        e.register(STARS_LOAD_GEOMETRY, |_, _| Ret::default());
        e.register(STRING_CONSTRUCT, |_, _| Ret::default());
        e.register(STRING_DESTRUCT, |_, _| Ret::default());
        e.register(STRING_ASSIGN, |_, _| Ret::default());
        e.register(STRING_APPEND, |_, _| Ret::default());
        e.register(STRING_COMPARE, |_, _| ret(1));
        e.register(SUN_FIRST_NODE, |_, _| ret(0x7100));
        e.register(NODE_GET_PROPERTY, |_, a| {
            assert_eq!(a[1], 3);
            ret(0x8100 + a[0])
        });
        e.register(TEXTURE_ENTRY_VALID, |_, _| ret(1));
        e.register(TEXTURE_ENTRY_NAME, |_, _| ret(0x6200));
        e.register(NI_POINTER_FROM_RAW, |e, a| {
            e.mem.set_u32(a[0], a[1]);
            ret(a[0])
        });
        e.register(TEXTURE_LOAD, |e, a| {
            e.mem.set_u32(a[2], 0x7700);
            Ret::default()
        });
        e.register(PROPERTY_SET_TEXTURE, |_, _| Ret::default());
        e.register(NODE_SET_FLAG_BIT_20, |_, _| Ret::default());
        e.register(HOLDER_NODE, |e, a| ret(e.mem.u32(a[0] + 8)));
        e.register(SUN_GET_NODE, |e, a| ret(e.mem.u32(a[0] + 0xc)));
        e.register(ROOT_UPDATE_PROPERTIES, |_, _| Ret::default());
        e.register(NI_UPDATE_DATA_CONSTRUCT, |_, _| Ret::default());
        e.register(NODE_UPDATE_CONTROLLERS, |_, _| Ret::default());
        e.register(SETTING_WORD_VALUE, |_, a| ret(a[0]));
        e.register(MOON_CONSTRUCT, |e, a| {
            e.mem.set_u32(a[0], VTABLE);
            ret(a[0])
        });
        e
    }

    #[test]
    fn climate_update_does_nothing_without_the_flag() {
        let mut e = climate_update_engine();
        let sky = new_sky(&mut e);
        e.call_log = Some(vec![]);
        e.call(0x0063_ca20, &args![sky]);
        assert_eq!(e.call_log.take().unwrap().len(), 1);
    }

    #[test]
    fn climate_update_loads_stars_sun_textures_and_creates_the_moons() {
        let mut e = climate_update_engine();
        let sky = new_sky(&mut e);
        e.set(sky, Sky::uiFlags, 0x40 | 0x100);
        e.set(sky, Sky::eMode, 3);
        // Climate: an embedded object at +0x18 with a vtable, the two texture
        // entries (objects with a vtable) at +0x38 and +0x44, and both moons.
        let climate = e.mem.alloc(0x80);
        e.mem.set_u32(climate + 0x18, VTABLE);
        e.mem.set_u32(climate + 0x38, VTABLE);
        e.mem.set_u32(climate + 0x44, VTABLE);
        e.mem.set_u8(climate + 0x55, 0xc0);
        e.set(sky, Sky::pCurrentClimate, Ptr::new(climate));
        let stars = e.mem.alloc(0x10);
        let sun = e.mem.alloc(0x30);
        e.mem.set_u32(sun + 8, 0x5100);
        e.mem.set_u32(sun + 0xc, 0x5200);
        e.mem.set_u32(sun + 0x14, 0x5300);
        e.set(sky, Sky::pStars, Ptr::new(stars));
        e.set(sky, Sky::pSun, Ptr::new(sun));
        e.mem.set_u32(sky.addr() + 4, 0x4100);
        e.mem.set_u32(sky.addr() + 8, 0x4200);
        e.set_global(0x011c_cc54, 1.0f32);
        e.call_log = Some(vec![]);
        e.call(0x0063_ca20, &args![sky]);
        let log = e.call_log.take().unwrap();
        // The stars get the climate's geometry value.
        assert!(log.contains(&(STARS_LOAD_GEOMETRY, vec![stars, 0x5150])));
        // The first texture (entry 0, climate + 0x38) is built from the
        // entry's prefix and name, loaded and set on the sun's property.
        assert!(log.contains(&(
            STRING_ASSIGN,
            vec![
                log.iter().find(|(a, _)| *a == STRING_CONSTRUCT).unwrap().1[0],
                0x6100
            ]
        )));
        assert!(log.contains(&(
            STRING_APPEND,
            vec![
                log.iter().find(|(a, _)| *a == STRING_CONSTRUCT).unwrap().1[0],
                0x6200
            ]
        )));
        let loads = log.iter().filter(|(a, _)| *a == TEXTURE_LOAD).count();
        assert_eq!(loads, 2);
        let sets: Vec<_> = log
            .iter()
            .filter(|(a, _)| *a == PROPERTY_SET_TEXTURE)
            .collect();
        assert_eq!(sets.len(), 2);
        assert_eq!(sets[0].1[1], 0x7700);
        // Both sun nodes are flagged as having a texture.
        assert!(log.contains(&(NODE_SET_FLAG_BIT_20, vec![0x5100, 1])));
        assert!(log.contains(&(NODE_SET_FLAG_BIT_20, vec![0x5200, 1])));
        // Both moons are created with the settings and initialized with the
        // moons root and their names (slot 0x10).
        let masser = e.get(sky, Sky::pMasser);
        let secunda = e.get(sky, Sky::pSecunda);
        assert!(!masser.is_null() && !secunda.is_null());
        assert_eq!(e.mem.block_size(masser.addr()), Some(0x80));
        assert!(log.contains(&(SLOT_BASE + 0x10, vec![masser.addr(), 0x4200, 0x0104_eeb0])));
        assert!(log.contains(&(SLOT_BASE + 0x10, vec![secunda.addr(), 0x4200, 0x0104_eea8])));
        let masser_construct = log
            .iter()
            .find(|(a, args)| *a == MOON_CONSTRUCT && args[1] == 0x0104_eeb0)
            .unwrap();
        // The name, the five float settings in reverse order and the word.
        assert_eq!(masser_construct.1.len(), 8);
        assert_eq!(
            masser_construct.1[2..7].iter().filter(|w| **w == 0).count(),
            4
        );
        assert_eq!(masser_construct.1[6], 1.0f32.to_bits());
        // The flag is cleared and the root's controllers updated.
        assert_eq!(e.get(sky, Sky::uiFlags), 0x100);
        assert!(log.contains(&(ROOT_UPDATE_PROPERTIES, vec![0x4100])));
        assert!(addresses(&log).contains(&NODE_UPDATE_CONTROLLERS));
    }

    #[test]
    fn climate_update_uses_the_non_hdr_glare_texture_and_deletes_old_moons() {
        let mut e = climate_update_engine();
        e.register(STRING_COMPARE, |_, a| {
            assert_eq!((a[0], a[1]), (0x6200, SUN_GLARE_NAME));
            ret(0)
        });
        let sky = new_sky(&mut e);
        e.set(sky, Sky::uiFlags, 0x40);
        e.set(sky, Sky::eMode, 2);
        let climate = e.mem.alloc(0x80);
        e.mem.set_u32(climate + 0x38, VTABLE);
        e.mem.set_u32(climate + 0x44, VTABLE);
        e.mem.set_u8(climate + 0x55, 0);
        e.set(sky, Sky::pCurrentClimate, Ptr::new(climate));
        let sun = e.mem.alloc(0x30);
        e.mem.set_u32(sun + 8, 0x5100);
        e.mem.set_u32(sun + 0xc, 0x5200);
        e.mem.set_u32(sun + 0x14, 0x5300);
        e.set(sky, Sky::pSun, Ptr::new(sun));
        let masser = object_with_vtable(&mut e, 0x80);
        let secunda = object_with_vtable(&mut e, 0x80);
        e.set(sky, Sky::pMasser, masser);
        e.set(sky, Sky::pSecunda, secunda);
        e.mem.set_u32(sky.addr() + 4, 0x4100);
        e.call_log = Some(vec![]);
        e.call(0x0063_ca20, &args![sky]);
        let log = e.call_log.take().unwrap();
        // Entry 1 is the glare texture: with the HDR byte clear and its name
        // equal to the glare name, the non-HDR path is used.
        assert!(log
            .iter()
            .any(|(a, args)| *a == STRING_ASSIGN && args[1] == SUN_GLARE_NON_HDR_PATH));
        // The moons are deleted (slot 0, flag 1) and cleared.
        assert!(log.contains(&(SLOT_BASE, vec![masser.addr(), 1])));
        assert!(log.contains(&(SLOT_BASE, vec![secunda.addr(), 1])));
        assert!(e.get(sky, Sky::pMasser).is_null());
        assert!(e.get(sky, Sky::pSecunda).is_null());
    }

    #[test]
    fn climate_entry_accessor_returns_the_first_two_entries() {
        let mut e = engine();
        assert_eq!(e.call(0x0063_cfa0, &args![0x1000u32, 0i32]).u32(), 0x1038);
        assert_eq!(e.call(0x0063_cfa0, &args![0x1000u32, 1i32]).u32(), 0x1044);
        assert_eq!(e.call(0x0063_cfa0, &args![0x1000u32, 2i32]).u32(), 0);
        assert_eq!(
            e.call(0x0063_cfa0, &args![0x1000u32, -1i32]).u32(),
            0x1000 + 0x38 - 12
        );
    }

    #[test]
    fn climate_virtual_accessor_calls_slot_0x14_of_the_embedded_object() {
        let mut e = engine();
        install_vtable(&mut e);
        e.register(SLOT_BASE + 0x14, |_, a| ret(a[0] + 1));
        let climate = e.mem.alloc(0x40);
        e.mem.set_u32(climate + 0x18, VTABLE);
        assert_eq!(
            e.call(0x0063_cfe0, &args![climate]).u32(),
            climate + 0x18 + 1
        );
    }

    #[test]
    fn climate_moon_flags_read_bits_of_the_byte_at_0x55() {
        let mut e = engine();
        let climate = e.mem.alloc(0x80);
        for (byte, masser, secunda) in [
            (0u8, false, false),
            (0x80, true, false),
            (0x40, false, true),
            (0xc0, true, true),
        ] {
            e.mem.set_u8(climate + 0x55, byte);
            assert_eq!(e.call(0x0063_d000, &args![climate]).bool(), masser);
            assert_eq!(e.call(0x0063_d020, &args![climate]).bool(), secunda);
        }
    }

    #[test]
    fn sun_node_accessor_reads_the_pointer_at_0x14() {
        let mut e = engine();
        let sun = e.mem.alloc(0x30);
        e.mem.set_u32(sun + 0x14, 0xabcd);
        assert_eq!(e.call(0x0063_d040, &args![sun]).u32(), 0xabcd);
    }

    // @@TESTS
}
