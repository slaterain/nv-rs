//! `fallout shared/tesscriptfunctions.cpp` (Xbox PDB source unit), part 4: its functions from `005c8450` up to
//! (not including) `005cd990` in FalloutNV.exe 1.4.0.525, translated
//! (docs/ENGINE_CRATE.md). The unit's shared layouts and helpers are in
//! [`super::tesscriptfunctions`]; anything public there may be used here.
//!
//! Progress: the first 40 queue entries of this part (`005c8450` to
//! `005ca510`) are translated. The next session continues at `005ca540`.
//!
//! Notes on the exe's code that the translations rely on:
//! - Script command bodies are `cdecl` and take the eight words of
//!   [`ScriptArgs`]; the parameters are read through
//!   `Script::ParseParameters` into locals passed by address
//!   ([`parse_into`]). The condition-function twins
//!   (`Script::Get...ConditionFunction`, `tesconditionfunctions.cpp`) are
//!   called by address.
//! - `005ca1c0` is the shared body of the "script event flag" commands: it
//!   asks the script event list (the sixth word) whether an event is set.
//! - The HDR commands work on the image space manager's HDR effect
//!   (`ImageSpaceEffectHDR`, Xbox PDB), whose `pfData` array (+0x6c) holds
//!   the 16 `HDRDataType` floats (`HDR_EYE_ADAPT_SPEED = 0` ...
//!   `HDR_TREE_DIMMER = 13`).
//! - The compiler's exception-unwinding frame (`__CxxFrameHandler` states,
//!   the `FS:[0]` chain) of `005c9cf0` is not translated.

#[allow(unused_imports)]
use super::tesscriptfunctions::*;
#[allow(unused_imports)]
use crate::prelude::*;

// ---- Callees outside the unit (by exe address) -----------------------------

/// `Script::ParseParameters` (Xbox PDB), `cdecl`, varargs.
const PARSE_PARAMETERS: u32 = 0x005a_ccb0;
/// The console print: format address first, then the arguments (`cdecl`);
/// a `double` argument takes two words.
const CONSOLE_PRINT: u32 = 0x0070_3c00;
/// `__RTDynamicCast` (`object, 0, source type, target type, 0`).
const DYNAMIC_CAST: u32 = 0x00ec_43fb;
/// RTTI type descriptor of `TESObjectREFR` (`.?AVTESObjectREFR@@`).
const RTTI_TES_OBJECT_REFR: u32 = 0x0118_41cc;
/// RTTI type descriptor of `Actor` (`.?AVActor@@`).
const RTTI_ACTOR: u32 = 0x0118_46d4;

/// `Script::GetDispositionConditionFunction` (Xbox PDB), `cdecl`
/// (`thisObj, actor, 0, result`).
const GET_DISPOSITION_CONDITION: u32 = 0x0059_fdb0;
/// `Script::GetRandomPercentConditionFunction` (Xbox PDB), `cdecl`
/// (`thisObj, 0, 0, result`).
const GET_RANDOM_PERCENT_CONDITION: u32 = 0x005a_0040;
/// `Script::GetLevelConditionFunction` (Xbox PDB), same arguments.
const GET_LEVEL_CONDITION: u32 = 0x005a_00c0;
/// `Script::GetArmorRatingConditionFunction` (Xbox PDB), same arguments.
const GET_ARMOR_RATING_CONDITION: u32 = 0x005a_0150;
/// `Script::GetDeadCountConditionFunction` (Xbox PDB), `cdecl`
/// (`thisObj, form, 0, result`).
const GET_DEAD_COUNT_CONDITION: u32 = 0x005a_01f0;
/// `Script::GetAlertConditionFunction` (Xbox PDB), same arguments as
/// [`GET_RANDOM_PERCENT_CONDITION`].
const GET_ALERT_CONDITION: u32 = 0x005a_0260;
/// Condition function of `tesconditionfunctions.cpp` (`thisObj, 0, 0,
/// result`) that stores a whole number as a `double`.
const FN_005A4340: u32 = 0x005a_4340;

/// `Actor::SetAlert` (Xbox PDB), `thiscall` (`bool`).
const ACTOR_SET_ALERT: u32 = 0x008a_5e40;
/// `Actor::SetLookAtTarget` (Xbox PDB), `thiscall` (the three words of a
/// point passed by value).
const ACTOR_SET_LOOK_AT_TARGET: u32 = 0x008b_3cd0;
/// `Actor::ClearLookAtTarget` (Xbox PDB), `thiscall`.
const ACTOR_CLEAR_LOOK_AT_TARGET: u32 = 0x008b_3d30;
/// `MiddleHighProcess::GetSavedAcquireObject` (Xbox PDB), `thiscall` on the
/// actor (the name is the engine map's; it returns the actor's process).
const GET_SAVED_ACQUIRE_OBJECT: u32 = 0x008d_8520;
/// `this + 0x44`: the extra data list of a reference.
const EXTRA_DATA_LIST: u32 = 0x005d_43c0;
/// `ExtraDataList::SetItemDropper` (Xbox PDB), `thiscall` (`reference`).
const SET_ITEM_DROPPER: u32 = 0x0041_9dc0;
/// `TESObjectREFR::SetTargeted` (Xbox PDB), `thiscall` (`bool`).
const SET_TARGETED: u32 = 0x0056_4db0;
/// `TESObjectREFR::GetMapMarkerData` (Xbox PDB), `thiscall`.
const GET_MAP_MARKER_DATA: u32 = 0x0056_9060;
/// `MapMarkerData::GetVisible` (Xbox PDB), `thiscall`.
const MAP_MARKER_GET_VISIBLE: u32 = 0x0043_8ed0;
/// `MapMarkerData::GetTravelLoc` (Xbox PDB), `thiscall`.
const MAP_MARKER_GET_TRAVEL_LOC: u32 = 0x0043_8ef0;
/// `MapMarkerData::SetVisible` (Xbox PDB), `thiscall` (`bool`).
const MAP_MARKER_SET_VISIBLE: u32 = 0x0044_de40;
/// `MapMarkerData::SetTravelLoc` (Xbox PDB), `thiscall` (`bool`).
const MAP_MARKER_SET_TRAVEL_LOC: u32 = 0x0044_de80;
/// `*(this + 4)` of a string global: its text (`thiscall`).
const BS_STRING_TEXT: u32 = 0x0040_3df0;
/// Interface message with icon (`cdecl`: `text, 0, icon path, sound, float,
/// 0`).
const SHOW_MESSAGE: u32 = 0x0070_52f0;
/// `thiscall` on a reference (`action`): whether the action is set.
const HAS_ACTION: u32 = 0x0057_2d30;
/// `TESObjectREFR::ClearAction` (Xbox PDB), `thiscall` (`action`).
const CLEAR_ACTION: u32 = 0x0057_2db0;
/// `NiPointer::operator T*`: `*this`.
const NI_POINTER_GET: u32 = 0x0055_9450;
/// `thiscall` on the object `005c9740` parsed (`1`); it walks a list at
/// `+8` of the object and updates each entry (unnamed, no unit).
const FN_0060C950: u32 = 0x0060_c950;
/// `thiscall` on a script event list (`event, mask`): whether the event is
/// in the list with one of the mask's bits set (`tesscript.cpp`).
const SCRIPT_EVENT_LIST_HAS_EVENT: u32 = 0x005a_8ef0;
/// `fabs` of a `float` argument, result in `ST0` (`effectsetting.cpp`).
const FLOAT_ABS: u32 = 0x0040_8840;

/// The image space manager singleton getter (loads the global `011f91ac`).
const GET_IMAGE_SPACE_MANAGER: u32 = 0x004e_3270;
/// `thiscall` on the image space manager (`index`): the effect with that
/// index; the HDR commands ask for index 1, the HDR effect.
const IMAGE_SPACE_GET_EFFECT: u32 = 0x004e_bbc0;

/// `operator new` (`size`), `cdecl`.
const OPERATOR_NEW: u32 = 0x0040_1000;
/// `Ni` allocator (`size`), `cdecl`.
const NI_ALLOC: u32 = 0x00aa_13e0;
/// `_vector_constructor_iterator_` (`array, element size, count,
/// constructor`), `stdcall`; the constructor is called on each element.
const VECTOR_CONSTRUCT_ITERATOR: u32 = 0x0040_1050;
/// The default `NiPoint3` constructor passed to the iterator (a function
/// returning `this`).
const NI_POINT3_DEFAULT_CONSTRUCT: u32 = 0x0068_15c0;
/// The default `NiColorA` constructor passed to the iterator.
const NI_COLOR_A_DEFAULT_CONSTRUCT: u32 = 0x004a_7800;
/// `NiColorA::NiColorA(r, g, b, a)`, `thiscall`: returns `this`.
const NI_COLOR_A_CONSTRUCT: u32 = 0x0041_4430;
/// `NiPoint3::NiPoint3(x, y, z)`, `thiscall`: returns `this`.
const NI_POINT3_CONSTRUCT: u32 = 0x0041_6870;
/// `NiLines::NiLines` (Xbox PDB), `thiscall` (`vertex count, vertices,
/// colors, texture coordinates, texture sets, NBT method, connections`):
/// returns `this`.
const NI_LINES_CONSTRUCT: u32 = 0x00a7_46e0;
/// `NiAVObject::GetWorldBound` (Xbox PDB), `thiscall`: the bound (a default
/// one when the object has none).
const GET_WORLD_BOUND: u32 = 0x0043_d450;
/// The radius (`float` at `+0xc`) of a bound, in `ST0`.
const BOUND_RADIUS: u32 = 0x0084_d030;
/// Copies an `NiPoint3` into `this + 0x58` (the local translation of an
/// `NiAVObject`; `thiscall`, `RET 4`).
const SET_TRANSLATE: u32 = 0x0044_0460;
/// `TES::AddTempDebugObject` (Xbox PDB), `thiscall` on the `TES` singleton
/// (`object, seconds`).
const ADD_TEMP_DEBUG_OBJECT: u32 = 0x0045_8e20;

// ---- Globals ---------------------------------------------------------------

/// Pointer to the `TES` singleton.
const TES_SINGLETON: u32 = 0x011d_ea10;
/// Global `float`: the sunlight dimmer HDR value (`fSunlightDimmer`).
const HDR_SUNLIGHT_DIMMER: u32 = 0x011f_9190;
/// Global `float`: the luminance ramp (`Lum Ramp`).
const HDR_LUM_RAMP: u32 = 0x011a_d87c;
/// Global `float`: `fGrassDimmer`.
const HDR_GRASS_DIMMER: u32 = 0x011f_9188;
/// Global `float`: `fTreeDimmer`.
const HDR_TREE_DIMMER: u32 = 0x011f_918c;
/// `float` the message commands pass as the sound volume.
const MESSAGE_VOLUME: u32 = 0x0101_62c0;
/// `10.0f`: the default half length of the debug axes.
const DEFAULT_AXIS_LENGTH: u32 = 0x0101_7b78;
/// `20.0f`: seconds a debug object stays.
const DEBUG_OBJECT_SECONDS: u32 = 0x0101_7868;
/// `0.5` (`double`).
const HALF: u32 = 0x0101_1588;
/// The string global whose text the map marker message shows.
const MAP_MARKER_MESSAGE: u32 = 0x011d_4768;

// ---- String literals (addresses in the exe's data) ---------------------------

/// `"Interface\Icons\Message Icons\glow_message_map.dds"`
const ICON_MAP: u32 = 0x0103_b30c;
/// `"UIPopUpMapMarkerAdded"`
const SOUND_MAP_MARKER_ADDED: u32 = 0x0103_b340;
/// `"Current HDR Params:"`
const MSG_HDR_PARAMS: u32 = 0x0103_b464;
/// `"SISG:"`
const MSG_SISG: u32 = 0x0103_b45c;
/// `"    iNumBlurpasses: %d fBlurRadius: %f"`
const FORMAT_BLUR: u32 = 0x0103_b434;
/// `"    fBrightClamp: %f fBrightScale: %f"`
const FORMAT_BRIGHT: u32 = 0x0103_b40c;
/// `"SSP:"`
const MSG_SSP: u32 = 0x0103_b404;
/// `"    fSunlightDimmer: %f Lum Ramp: %f"`
const FORMAT_SUNLIGHT: u32 = 0x0103_b3dc;
/// `"SHP:"`
const MSG_SHP: u32 = 0x0103_b3d4;
/// `"    fEyeAdaptSpeed: %f fEmissiveHDRMult: %f"`
const FORMAT_EYE_ADAPT: u32 = 0x0103_b3a8;
/// `"    fTreeDimmer: %f fGrassDimmer: %f"`
const FORMAT_DIMMERS: u32 = 0x0103_b380;
/// `"    fUpperLUMClamp: %f fTargetLUM: %f"`
const FORMAT_LUM: u32 = 0x0103_b358;

// ---- Small helpers -------------------------------------------------------------

/// `Script::ParseParameters` with `N` word-sized locals (the stack slots the
/// game passes by address) initialised to `init`: `None` when the
/// parameters do not parse, otherwise the values left in the locals.
fn parse_into<const N: usize>(e: &mut Engine, a: ScriptArgs, init: [u32; N]) -> Option<[u32; N]> {
    let block = e.mem.alloc(4 * N as u32);
    let mut outs = [0u32; N];
    for (i, value) in init.iter().enumerate() {
        outs[i] = block + 4 * i as u32;
        e.mem.set_u32(outs[i], *value);
    }
    let mut words = args![
        a.param_info,
        a.script_data,
        a.opcode_offset,
        a.this_obj,
        a.containing_obj,
        a.script_obj,
        a.event_list
    ];
    words.extend_from_slice(&outs);
    let ok = e.call(PARSE_PARAMETERS, &words).bool();
    let mut values = init;
    for (i, value) in values.iter_mut().enumerate() {
        *value = e.mem.u32(outs[i]);
    }
    e.mem.free(block);
    ok.then_some(values)
}

/// `__RTDynamicCast(object, TESObjectREFR -> Actor)`: the actor, or 0.
fn cast_to_actor(e: &mut Engine, object: Ptr) -> u32 {
    e.call(
        DYNAMIC_CAST,
        &args![object, 0u32, RTTI_TES_OBJECT_REFR, RTTI_ACTOR, 0u32],
    )
    .u32()
}

fn console_print(e: &mut Engine, words: &[u32]) {
    e.call(CONSOLE_PRINT, words);
}

/// The actor's process (`MiddleHighProcess::GetSavedAcquireObject`).
fn saved_acquire_object(e: &mut Engine, actor: u32) -> u32 {
    e.call(GET_SAVED_ACQUIRE_OBJECT, &args![actor]).u32()
}

/// Copies `words` 32-bit words from `from` to `to`.
fn copy_words(e: &mut Engine, to: u32, from: u32, words: u32) {
    for i in 0..words {
        let word = e.mem.u32(from + 4 * i);
        e.mem.set_u32(to + 4 * i, word);
    }
}

/// The twin of a condition function without arguments: calls
/// `callee(thisObj, 0, 0, result)` and returns its `AL`.
fn plain_condition(e: &mut Engine, a: ScriptArgs, callee: u32) -> bool {
    e.call(callee, &args![a.this_obj, 0u32, 0u32, a.result])
        .bool()
}

/// Parses one argument and returns `callee(thisObj, argument, 0, result)`.
fn one_argument_condition(e: &mut Engine, a: ScriptArgs, callee: u32) -> bool {
    let Some([argument]) = parse_into(e, a, [0]) else {
        return false;
    };
    e.call(callee, &args![a.this_obj, argument, 0u32, a.result])
        .bool()
}

// ---- Translated functions --------------------------------------------------------

// Translated from 005c8450 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Script::GetDisposition` (Xbox PDB): parses one actor argument and
/// returns `Script::GetDispositionConditionFunction(thisObj, actor, 0,
/// result)`.
pub fn script_get_disposition(e: &mut Engine, a: ScriptArgs) -> bool {
    one_argument_condition(e, a, GET_DISPOSITION_CONDITION)
}

// Translated from 005c84b0 (decompiled, FalloutNV.exe 1.4.0.525)
/// The `GetRandomPercent` command: `Script::GetRandomPercentConditionFunction
/// (thisObj, 0, 0, result)`.
pub fn fn_005c84b0(e: &mut Engine, a: ScriptArgs) -> bool {
    plain_condition(e, a, GET_RANDOM_PERCENT_CONDITION)
}

// Translated from 005c84d0 (decompiled, FalloutNV.exe 1.4.0.525)
/// The `GetLevel` command: `Script::GetLevelConditionFunction(thisObj, 0, 0,
/// result)`.
pub fn fn_005c84d0(e: &mut Engine, a: ScriptArgs) -> bool {
    plain_condition(e, a, GET_LEVEL_CONDITION)
}

// Translated from 005c84f0 (decompiled, FalloutNV.exe 1.4.0.525)
/// The `GetArmorRating` command: `Script::GetArmorRatingConditionFunction
/// (thisObj, 0, 0, result)`.
pub fn fn_005c84f0(e: &mut Engine, a: ScriptArgs) -> bool {
    plain_condition(e, a, GET_ARMOR_RATING_CONDITION)
}

// Translated from 005c8510 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Script::ModDisposition` (Xbox PDB): parses an actor argument and an
/// integer; when `thisObj` is an actor and the argument is given, calls
/// virtual slot `0x460` of the actor (`actor argument, (float) amount`) and
/// returns `Script::GetDispositionConditionFunction(thisObj, actor, 0,
/// result)`. False when the arguments do not parse or either is missing.
pub fn script_mod_disposition(e: &mut Engine, a: ScriptArgs) -> bool {
    let Some([target, amount]) = parse_into(e, a, [0, 0]) else {
        return false;
    };
    let actor = cast_to_actor(e, a.this_obj);
    if actor == 0 || target == 0 {
        return false;
    }
    e.vcall(actor, 0x460, &args![target, amount as i32 as f32]);
    e.call(
        GET_DISPOSITION_CONDITION,
        &args![a.this_obj, target, 0u32, a.result],
    )
    .bool()
}

// Translated from 005c85c0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Script::GetDeadCount` (Xbox PDB): parses one form argument and returns
/// `Script::GetDeadCountConditionFunction(thisObj, form, 0, result)`.
pub fn script_get_dead_count(e: &mut Engine, a: ScriptArgs) -> bool {
    one_argument_condition(e, a, GET_DEAD_COUNT_CONDITION)
}

// Translated from 005c8620 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Script::ShowMap` (Xbox PDB): parses a map marker reference and a flag.
/// A reference that has map marker data is made visible (and marked
/// changed when it was not), its virtual slot `0x48` called with
/// `0x80000000`; with the flag set its travel location is enabled the same
/// way. When anything changed the "map marker added" message is shown with
/// its icon and sound. `*result` is 1.0.
pub fn script_show_map(e: &mut Engine, a: ScriptArgs) -> bool {
    let Some([marker, travel]) = parse_into(e, a, [0, 0]) else {
        return false;
    };
    let mut changed = false;
    if marker != 0 && e.call(GET_MAP_MARKER_DATA, &args![marker]).u32() != 0 {
        let data = e.call(GET_MAP_MARKER_DATA, &args![marker]).u32();
        if !e.call(MAP_MARKER_GET_VISIBLE, &args![data]).bool() {
            changed = true;
        }
        let data = e.call(GET_MAP_MARKER_DATA, &args![marker]).u32();
        e.call(MAP_MARKER_SET_VISIBLE, &args![data, 1u32]);
        e.vcall(marker, 0x48, &args![0x8000_0000u32]);
        if travel != 0 {
            let data = e.call(GET_MAP_MARKER_DATA, &args![marker]).u32();
            if !e.call(MAP_MARKER_GET_TRAVEL_LOC, &args![data]).bool() {
                changed = true;
            }
            let data = e.call(GET_MAP_MARKER_DATA, &args![marker]).u32();
            e.call(MAP_MARKER_SET_TRAVEL_LOC, &args![data, 1u32]);
        }
    }
    if changed {
        let text = e.call(BS_STRING_TEXT, &args![MAP_MARKER_MESSAGE]).u32();
        let volume: f32 = e.global(MESSAGE_VOLUME);
        e.call(
            SHOW_MESSAGE,
            &args![text, 0u32, ICON_MAP, SOUND_MAP_MARKER_ADDED, volume, 0u32],
        );
    }
    e.mem.set_f64(a.result.addr(), 1.0);
    true
}

// Translated from 005c9670 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Script::SetAlertFunction` (Xbox PDB): parses an integer and, when
/// `thisObj` is an actor, sets its alert state to "the integer is positive"
/// (`Actor::SetAlert`).
pub fn script_set_alert_function(e: &mut Engine, a: ScriptArgs) -> bool {
    let Some([alert]) = parse_into(e, a, [0]) else {
        return false;
    };
    if !a.this_obj.is_null() {
        let actor = cast_to_actor(e, a.this_obj);
        if actor != 0 {
            e.call(ACTOR_SET_ALERT, &args![actor, (alert as i32) > 0]);
        }
    }
    true
}

// Translated from 005c9700 (decompiled, FalloutNV.exe 1.4.0.525)
/// The `GetAlert` command: `Script::GetAlertConditionFunction(thisObj, 0, 0,
/// result)`.
pub fn fn_005c9700(e: &mut Engine, a: ScriptArgs) -> bool {
    plain_condition(e, a, GET_ALERT_CONDITION)
}

// Translated from 005c9720 (decompiled, FalloutNV.exe 1.4.0.525)
/// A command that is `005a4340(thisObj, 0, 0, result)` of
/// `tesconditionfunctions.cpp`.
pub fn fn_005c9720(e: &mut Engine, a: ScriptArgs) -> bool {
    plain_condition(e, a, FN_005A4340)
}

// Translated from 005c9740 (decompiled, FalloutNV.exe 1.4.0.525)
/// Parses one object argument and calls `0060c950(object, 1)` on it, without
/// a null check.
pub fn fn_005c9740(e: &mut Engine, a: ScriptArgs) -> bool {
    let Some([object]) = parse_into(e, a, [0]) else {
        return false;
    };
    e.call(FN_0060C950, &args![object, 1u32]);
    true
}

// Translated from 005c9790 (decompiled, FalloutNV.exe 1.4.0.525)
/// Parses a target reference and a flag. When `thisObj` is an actor with a
/// process (`GetSavedAcquireObject`) and the target is given: with the flag
/// zero the process is told about the target (slot `0x62c`) and the actor's
/// extra data list records it as item dropper; otherwise the actor looks at
/// the target's position (virtual slot `0x1f4`, `Actor::SetLookAtTarget`),
/// the process is told (slot `0x7ac`) and the target is marked targeted.
/// Either way the actor's virtual slot `0x48` is then called with
/// `0x80000000`.
pub fn fn_005c9790(e: &mut Engine, a: ScriptArgs) -> bool {
    let Some([target, look]) = parse_into(e, a, [0, 0]) else {
        return false;
    };
    if a.this_obj.is_null() || target == 0 {
        return true;
    }
    let actor = cast_to_actor(e, a.this_obj);
    if actor == 0 || saved_acquire_object(e, actor) == 0 {
        return true;
    }
    if look == 0 {
        let process = saved_acquire_object(e, actor);
        e.vcall(process, 0x62c, &args![target]);
        let extra_list = e.call(EXTRA_DATA_LIST, &args![actor]).u32();
        e.call(SET_ITEM_DROPPER, &args![extra_list, target]);
    } else {
        let position = e.vcall(target, 0x1f4, &args![]).u32();
        let (x, y, z) = (
            e.mem.u32(position),
            e.mem.u32(position + 4),
            e.mem.u32(position + 8),
        );
        e.call(ACTOR_SET_LOOK_AT_TARGET, &args![actor, x, y, z]);
        let process = saved_acquire_object(e, actor);
        e.vcall(process, 0x7ac, &args![target]);
        e.call(SET_TARGETED, &args![target, 1u32]);
    }
    e.vcall(actor, 0x48, &args![0x8000_0000u32]);
    true
}

// Translated from 005c98e0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Undoes [`fn_005c9790`] for `thisObj` when it is an actor: clears the item
/// dropper of its extra data list and, when it has a process, tells it
/// (slot `0x648` with 1), clears the look-at target and tells the process
/// again (slot `0x7ac` with 0).
pub fn fn_005c98e0(e: &mut Engine, a: ScriptArgs) -> bool {
    if a.this_obj.is_null() {
        return true;
    }
    let actor = cast_to_actor(e, a.this_obj);
    if actor == 0 {
        return true;
    }
    let extra_list = e.call(EXTRA_DATA_LIST, &args![actor]).u32();
    e.call(SET_ITEM_DROPPER, &args![extra_list, 0u32]);
    if saved_acquire_object(e, actor) != 0 {
        let process = saved_acquire_object(e, actor);
        e.vcall(process, 0x648, &args![1u32]);
        e.call(ACTOR_CLEAR_LOOK_AT_TARGET, &args![actor]);
        let process = saved_acquire_object(e, actor);
        e.vcall(process, 0x7ac, &args![0u32]);
    }
    true
}

// Translated from 005c9980 (decompiled, FalloutNV.exe 1.4.0.525)
/// Parses two floats and stores their absolute values in the HDR globals
/// `fSunlightDimmer` (`011f9190`) and the luminance ramp (`011ad87c`).
pub fn fn_005c9980(e: &mut Engine, a: ScriptArgs) -> bool {
    let Some([sunlight, ramp]) = parse_into(e, a, [0, 0]) else {
        return false;
    };
    let value = e.call(FLOAT_ABS, &args![f32::from_bits(sunlight)]).f32();
    e.set_global(HDR_SUNLIGHT_DIMMER, value);
    let value = e.call(FLOAT_ABS, &args![f32::from_bits(ramp)]).f32();
    e.set_global(HDR_LUM_RAMP, value);
    true
}

// Translated from 005c9a00 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Script::PrintHDRParamFunction` (Xbox PDB): prints the current HDR
/// parameters to the console, one line per group, reading the effect's
/// values through the accessors below and the four globals.
///
/// The blur line is printed with one `double` for a format that has `%d`
/// and `%f` (the game's own quirk); it is passed on unchanged.
pub fn script_print_hdr_param_function(e: &mut Engine) -> bool {
    let manager = e.call(GET_IMAGE_SPACE_MANAGER, &args![]).ptr::<()>();
    console_print(e, &args![MSG_HDR_PARAMS]);
    console_print(e, &args![MSG_SISG]);
    let blur_radius = fn_005c9b70(e, manager);
    console_print(e, &args![FORMAT_BLUR, blur_radius as f64]);
    let bright_scale = fn_005c9bf0(e, manager);
    let bright_clamp = fn_005c9bb0(e, manager);
    console_print(
        e,
        &args![FORMAT_BRIGHT, bright_clamp as f64, bright_scale as f64],
    );
    console_print(e, &args![MSG_SSP]);
    let sunlight_dimmer: f32 = e.global(HDR_SUNLIGHT_DIMMER);
    let lum_ramp: f32 = e.global(HDR_LUM_RAMP);
    console_print(
        e,
        &args![FORMAT_SUNLIGHT, sunlight_dimmer as f64, lum_ramp as f64],
    );
    console_print(e, &args![MSG_SHP]);
    let emissive_mult = fn_005c9b30(e, manager);
    let eye_adapt_speed = fn_005c9c30(e, manager);
    console_print(
        e,
        &args![
            FORMAT_EYE_ADAPT,
            eye_adapt_speed as f64,
            emissive_mult as f64
        ],
    );
    let grass_dimmer: f32 = e.global(HDR_GRASS_DIMMER);
    let tree_dimmer: f32 = e.global(HDR_TREE_DIMMER);
    console_print(
        e,
        &args![FORMAT_DIMMERS, tree_dimmer as f64, grass_dimmer as f64],
    );
    let target_lum = fn_005c9c70(e, manager);
    let upper_lum_clamp = fn_005c9cb0(e, manager);
    console_print(
        e,
        &args![FORMAT_LUM, upper_lum_clamp as f64, target_lum as f64],
    );
    true
}

/// The HDR effect of the image space manager (`005c9b30`'s and its
/// siblings' first step).
fn hdr_effect(e: &mut Engine, manager: Ptr) -> Ptr {
    e.call(IMAGE_SPACE_GET_EFFECT, &args![manager, 1u32]).ptr()
}

/// `ImageSpaceEffectHDR::pfData[index]` (`pfData` at +0x6c, Xbox PDB).
fn hdr_data(e: &mut Engine, effect: Ptr, index: u32) -> f32 {
    let data = e.mem.u32(effect.addr() + 0x6c);
    e.mem.f32(data + 4 * index)
}

// Translated from 005c9b30 (decompiled, FalloutNV.exe 1.4.0.525)
/// The emissive HDR multiplier (`HDR_EMISSIVE_MULT`) of the manager's HDR
/// effect.
pub fn fn_005c9b30(e: &mut Engine, this: Ptr) -> f32 {
    let effect = hdr_effect(e, this);
    fn_005c9b50(e, effect)
}

// Translated from 005c9b50 (decompiled, FalloutNV.exe 1.4.0.525)
/// `ImageSpaceEffectHDR::pfData[HDR_EMISSIVE_MULT]` (index 3).
pub fn fn_005c9b50(e: &mut Engine, this: Ptr) -> f32 {
    hdr_data(e, this, 3)
}

// Translated from 005c9b70 (decompiled, FalloutNV.exe 1.4.0.525)
/// The HDR blur radius (`HDR_BLUR_RADIUS`) of the manager's HDR effect.
pub fn fn_005c9b70(e: &mut Engine, this: Ptr) -> f32 {
    let effect = hdr_effect(e, this);
    fn_005c9b90(e, effect)
}

// Translated from 005c9b90 (decompiled, FalloutNV.exe 1.4.0.525)
/// `ImageSpaceEffectHDR::pfData[HDR_BLUR_RADIUS]` (index 1).
pub fn fn_005c9b90(e: &mut Engine, this: Ptr) -> f32 {
    hdr_data(e, this, 1)
}

// Translated from 005c9bb0 (decompiled, FalloutNV.exe 1.4.0.525)
/// The HDR bright clamp (`HDR_BRIGHT_CLAMP`) of the manager's HDR effect.
pub fn fn_005c9bb0(e: &mut Engine, this: Ptr) -> f32 {
    let effect = hdr_effect(e, this);
    fn_005c9bd0(e, effect)
}

// Translated from 005c9bd0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `ImageSpaceEffectHDR::pfData[HDR_BRIGHT_CLAMP]` (index 7).
pub fn fn_005c9bd0(e: &mut Engine, this: Ptr) -> f32 {
    hdr_data(e, this, 7)
}

// Translated from 005c9bf0 (decompiled, FalloutNV.exe 1.4.0.525)
/// The HDR bright scale (`HDR_BRIGHT_SCALE`) of the manager's HDR effect.
pub fn fn_005c9bf0(e: &mut Engine, this: Ptr) -> f32 {
    let effect = hdr_effect(e, this);
    fn_005c9c10(e, effect)
}

// Translated from 005c9c10 (decompiled, FalloutNV.exe 1.4.0.525)
/// `ImageSpaceEffectHDR::pfData[HDR_BRIGHT_SCALE]` (index 6).
pub fn fn_005c9c10(e: &mut Engine, this: Ptr) -> f32 {
    hdr_data(e, this, 6)
}

// Translated from 005c9c30 (decompiled, FalloutNV.exe 1.4.0.525)
/// The HDR eye adapt speed (`HDR_EYE_ADAPT_SPEED`) of the manager's HDR
/// effect.
pub fn fn_005c9c30(e: &mut Engine, this: Ptr) -> f32 {
    let effect = hdr_effect(e, this);
    fn_005c9c50(e, effect)
}

// Translated from 005c9c50 (decompiled, FalloutNV.exe 1.4.0.525)
/// `ImageSpaceEffectHDR::pfData[HDR_EYE_ADAPT_SPEED]` (index 0).
pub fn fn_005c9c50(e: &mut Engine, this: Ptr) -> f32 {
    hdr_data(e, this, 0)
}

// Translated from 005c9c70 (decompiled, FalloutNV.exe 1.4.0.525)
/// The HDR target luminance (`HDR_TARGET_LUM`) of the manager's HDR effect.
pub fn fn_005c9c70(e: &mut Engine, this: Ptr) -> f32 {
    let effect = hdr_effect(e, this);
    fn_005c9c90(e, effect)
}

// Translated from 005c9c90 (decompiled, FalloutNV.exe 1.4.0.525)
/// `ImageSpaceEffectHDR::pfData[HDR_TARGET_LUM]` (index 4).
pub fn fn_005c9c90(e: &mut Engine, this: Ptr) -> f32 {
    hdr_data(e, this, 4)
}

// Translated from 005c9cb0 (decompiled, FalloutNV.exe 1.4.0.525)
/// The HDR upper luminance clamp (`HDR_UPPER_LUM_CLAMP`) of the manager's
/// HDR effect.
pub fn fn_005c9cb0(e: &mut Engine, this: Ptr) -> f32 {
    let effect = hdr_effect(e, this);
    fn_005c9cd0(e, effect)
}

// Translated from 005c9cd0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `ImageSpaceEffectHDR::pfData[HDR_UPPER_LUM_CLAMP]` (index 5).
pub fn fn_005c9cd0(e: &mut Engine, this: Ptr) -> f32 {
    hdr_data(e, this, 5)
}

// Translated from 005c9cf0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Debug command: draws three yellow axis lines (an `NiLines` of six
/// vertices, +-length along each axis, with the connection flags
/// `1 0 1 0 1 0`) at the position of `thisObj` (virtual slot `0x1f4`) for
/// 20 seconds (`TES::AddTempDebugObject`). The half length is 10.0, or half
/// the radius of the world bound of the reference's 3D (virtual slot
/// `0x1d0`) when that is smaller. Does nothing without a reference.
pub fn fn_005c9cf0(e: &mut Engine, a: ScriptArgs) -> bool {
    if a.this_obj.is_null() {
        return true;
    }
    let this = a.this_obj.addr();
    let mut length: f32 = e.global(DEFAULT_AXIS_LENGTH);
    if e.vcall(this, 0x1d0, &args![]).u32() != 0 {
        let node = e.vcall(this, 0x1d0, &args![]).u32();
        let bound = e.call(GET_WORLD_BOUND, &args![node]).u32();
        let radius = e.call(BOUND_RADIUS, &args![bound]).f64();
        let half: f64 = e.global(HALF);
        let half_radius = (radius * half) as f32;
        // `FCOMPP`/`TEST AH,0x41`: the default is kept unless it is greater
        // (or the comparison is unordered).
        if !matches!(
            length.partial_cmp(&half_radius),
            Some(std::cmp::Ordering::Less | std::cmp::Ordering::Equal)
        ) {
            length = half_radius;
        }
    }

    // Six `NiPoint3` vertices, six `NiColorA` colors and six connection
    // bytes.
    let vertices = e.call(OPERATOR_NEW, &args![0x48u32]).u32();
    if vertices != 0 {
        e.call(
            VECTOR_CONSTRUCT_ITERATOR,
            &args![vertices, 0xcu32, 6u32, NI_POINT3_DEFAULT_CONSTRUCT],
        );
    }
    let colors = e.call(OPERATOR_NEW, &args![0x60u32]).u32();
    if colors != 0 {
        e.call(
            VECTOR_CONSTRUCT_ITERATOR,
            &args![colors, 0x10u32, 6u32, NI_COLOR_A_DEFAULT_CONSTRUCT],
        );
    }
    let connections = e.call(OPERATOR_NEW, &args![6u32]).u32();
    for (i, flag) in [1u8, 0, 1, 0, 1, 0].into_iter().enumerate() {
        e.mem.set_u8(connections + i as u32, flag);
    }

    for i in 0..6 {
        e.with_stack(0x10, |e, temp| {
            let color = e
                .call(
                    NI_COLOR_A_CONSTRUCT,
                    &args![temp, 1.0f32, 1.0f32, 0.0f32, 1.0f32],
                )
                .u32();
            copy_words(e, colors + 0x10 * i, color, 4);
        });
    }

    let points = [
        (-length, 0.0f32, 0.0f32),
        (length, 0.0, 0.0),
        (0.0, -length, 0.0),
        (0.0, length, 0.0),
        (0.0, 0.0, -length),
        (0.0, 0.0, length),
    ];
    for (i, (x, y, z)) in points.into_iter().enumerate() {
        e.with_stack(0xc, |e, temp| {
            let point = e.call(NI_POINT3_CONSTRUCT, &args![temp, x, y, z]).u32();
            copy_words(e, vertices + 0xc * i as u32, point, 3);
        });
    }

    let storage = e.call(NI_ALLOC, &args![0xc4u32]).u32();
    let lines = if storage == 0 {
        0
    } else {
        e.call(
            NI_LINES_CONSTRUCT,
            &args![
                storage,
                6u32,
                vertices,
                colors,
                0u32,
                1u32,
                0u32,
                connections
            ],
        )
        .u32()
    };
    let position = e.vcall(this, 0x1f4, &args![]).u32();
    e.call(SET_TRANSLATE, &args![lines, position]);
    let seconds: f32 = e.global(DEBUG_OBJECT_SECONDS);
    let tes = e.global::<u32>(TES_SINGLETON);
    e.call(ADD_TEMP_DEBUG_OBJECT, &args![tes, lines, seconds]);
    true
}

// Translated from 005ca1c0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Shared body of the script event flag commands: `*result` is 1.0 when
/// `thisObj` and the script event list are given and the event list
/// reports `event` with one of the bits of `mask` set (`005a8ef0`), else
/// 0.0.
pub fn fn_005ca1c0(
    e: &mut Engine,
    mask: u32,
    this_obj: Ptr,
    event: u32,
    event_list: u32,
    result: Ptr,
) {
    e.mem.set_f64(result.addr(), 0.0);
    if !this_obj.is_null()
        && event_list != 0
        && e.call(SCRIPT_EVENT_LIST_HAS_EVENT, &args![event_list, event, mask])
            .bool()
    {
        e.mem.set_f64(result.addr(), 1.0);
    }
}

/// A command that parses one argument and passes it to [`fn_005ca1c0`]
/// with `mask`.
fn event_flag_command(e: &mut Engine, a: ScriptArgs, mask: u32) -> bool {
    let Some([event]) = parse_into(e, a, [0]) else {
        return false;
    };
    fn_005ca1c0(e, mask, a.this_obj, event, a.event_list, a.result);
    true
}

// Translated from 005ca200 (decompiled, FalloutNV.exe 1.4.0.525)
/// An event flag command with mask `1` (see [`fn_005ca1c0`]).
pub fn fn_005ca200(e: &mut Engine, a: ScriptArgs) -> bool {
    event_flag_command(e, a, 1)
}

// Translated from 005ca260 (decompiled, FalloutNV.exe 1.4.0.525)
/// An event flag command with mask `0x4000` (see [`fn_005ca1c0`]).
pub fn fn_005ca260(e: &mut Engine, a: ScriptArgs) -> bool {
    event_flag_command(e, a, 0x4000)
}

// Translated from 005ca2d0 (decompiled, FalloutNV.exe 1.4.0.525)
/// An event flag command with mask `2` (see [`fn_005ca1c0`]).
pub fn fn_005ca2d0(e: &mut Engine, a: ScriptArgs) -> bool {
    event_flag_command(e, a, 2)
}

// Translated from 005ca330 (decompiled, FalloutNV.exe 1.4.0.525)
/// An event flag command with mask `8` (see [`fn_005ca1c0`]).
pub fn fn_005ca330(e: &mut Engine, a: ScriptArgs) -> bool {
    event_flag_command(e, a, 8)
}

// Translated from 005ca390 (decompiled, FalloutNV.exe 1.4.0.525)
/// An event flag command with mask `4` (see [`fn_005ca1c0`]).
pub fn fn_005ca390(e: &mut Engine, a: ScriptArgs) -> bool {
    event_flag_command(e, a, 4)
}

// Translated from 005ca3f0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Script::OnActivateFunction` (Xbox PDB): parses one (unused) argument;
/// `*result` is 1.0 when `thisObj` has action 2 set, and both actions 1 and
/// 2 are cleared afterwards.
pub fn script_on_activate_function(e: &mut Engine, a: ScriptArgs) -> bool {
    e.mem.set_f64(a.result.addr(), 0.0);
    if parse_into(e, a, [0]).is_none() {
        return false;
    }
    if !a.this_obj.is_null() {
        if e.call(HAS_ACTION, &args![a.this_obj, 2u32]).bool() {
            e.mem.set_f64(a.result.addr(), 1.0);
        }
        e.call(CLEAR_ACTION, &args![a.this_obj, 1u32]);
        e.call(CLEAR_ACTION, &args![a.this_obj, 2u32]);
    }
    true
}

// Translated from 005ca470 (decompiled, FalloutNV.exe 1.4.0.525)
/// `*result` is 1.0 unless `thisObj`'s virtual slot `0x168` gives a
/// non-null pointer to a non-null pointer; the slot is asked afresh at each
/// step, as the game does. The game dereferences `thisObj` without a null
/// check (its later check never matters).
pub fn fn_005ca470(e: &mut Engine, a: ScriptArgs) -> bool {
    e.mem.set_f64(a.result.addr(), 0.0);
    let this = a.this_obj.addr();
    let mut value = 0u32;
    if e.vcall(this, 0x168, &args![]).u32() != 0 {
        let slot = e.vcall(this, 0x168, &args![]).u32();
        if fn_005ca4f0(e, Ptr::new(slot)) {
            let slot = e.vcall(this, 0x168, &args![]).u32();
            value = e.call(NI_POINTER_GET, &args![slot]).u32();
        }
    }
    if !a.this_obj.is_null() && value == 0 {
        e.mem.set_f64(a.result.addr(), 1.0);
    }
    true
}

// Translated from 005ca4f0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Whether the pointer stored at `this` is non-null.
pub fn fn_005ca4f0(e: &mut Engine, this: Ptr) -> bool {
    e.mem.u32(this.addr()) != 0
}

// Translated from 005ca510 (decompiled, FalloutNV.exe 1.4.0.525)
/// An event flag command with mask `0x400000` that reads no argument and
/// asks for event 0 (see [`fn_005ca1c0`]).
pub fn fn_005ca510(e: &mut Engine, a: ScriptArgs) -> bool {
    e.mem.set_f64(a.result.addr(), 0.0);
    fn_005ca1c0(e, 0x40_0000, a.this_obj, 0, a.event_list, a.result);
    true
}

/// This part's translated functions, by exe address.
pub fn funcs() -> Vec<(u32, AbiFn)> {
    vec![
        entry!(0x005c8450, script_get_disposition(ScriptArgs) -> bool),
        entry!(0x005c84b0, fn_005c84b0(ScriptArgs) -> bool),
        entry!(0x005c84d0, fn_005c84d0(ScriptArgs) -> bool),
        entry!(0x005c84f0, fn_005c84f0(ScriptArgs) -> bool),
        entry!(0x005c8510, script_mod_disposition(ScriptArgs) -> bool),
        entry!(0x005c85c0, script_get_dead_count(ScriptArgs) -> bool),
        entry!(0x005c8620, script_show_map(ScriptArgs) -> bool),
        entry!(0x005c9670, script_set_alert_function(ScriptArgs) -> bool),
        entry!(0x005c9700, fn_005c9700(ScriptArgs) -> bool),
        entry!(0x005c9720, fn_005c9720(ScriptArgs) -> bool),
        entry!(0x005c9740, fn_005c9740(ScriptArgs) -> bool),
        entry!(0x005c9790, fn_005c9790(ScriptArgs) -> bool),
        entry!(0x005c98e0, fn_005c98e0(ScriptArgs) -> bool),
        entry!(0x005c9980, fn_005c9980(ScriptArgs) -> bool),
        entry!(0x005c9a00, script_print_hdr_param_function() -> bool),
        entry!(0x005c9b30, fn_005c9b30(Ptr) -> f32),
        entry!(0x005c9b50, fn_005c9b50(Ptr) -> f32),
        entry!(0x005c9b70, fn_005c9b70(Ptr) -> f32),
        entry!(0x005c9b90, fn_005c9b90(Ptr) -> f32),
        entry!(0x005c9bb0, fn_005c9bb0(Ptr) -> f32),
        entry!(0x005c9bd0, fn_005c9bd0(Ptr) -> f32),
        entry!(0x005c9bf0, fn_005c9bf0(Ptr) -> f32),
        entry!(0x005c9c10, fn_005c9c10(Ptr) -> f32),
        entry!(0x005c9c30, fn_005c9c30(Ptr) -> f32),
        entry!(0x005c9c50, fn_005c9c50(Ptr) -> f32),
        entry!(0x005c9c70, fn_005c9c70(Ptr) -> f32),
        entry!(0x005c9c90, fn_005c9c90(Ptr) -> f32),
        entry!(0x005c9cb0, fn_005c9cb0(Ptr) -> f32),
        entry!(0x005c9cd0, fn_005c9cd0(Ptr) -> f32),
        entry!(0x005c9cf0, fn_005c9cf0(ScriptArgs) -> bool),
        entry!(0x005ca1c0, fn_005ca1c0(u32, Ptr, u32, u32, Ptr)),
        entry!(0x005ca200, fn_005ca200(ScriptArgs) -> bool),
        entry!(0x005ca260, fn_005ca260(ScriptArgs) -> bool),
        entry!(0x005ca2d0, fn_005ca2d0(ScriptArgs) -> bool),
        entry!(0x005ca330, fn_005ca330(ScriptArgs) -> bool),
        entry!(0x005ca390, fn_005ca390(ScriptArgs) -> bool),
        entry!(0x005ca3f0, script_on_activate_function(ScriptArgs) -> bool),
        entry!(0x005ca470, fn_005ca470(ScriptArgs) -> bool),
        entry!(0x005ca4f0, fn_005ca4f0(Ptr) -> bool),
        entry!(0x005ca510, fn_005ca510(ScriptArgs) -> bool),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    // Fake virtual functions the tests put into vtables.
    /// Returns the address `object + 0xe4`.
    const V_ADDRESS: u32 = 0x0900_0002;

    /// An engine with the pages of the globals these commands touch mapped
    /// and the dynamic cast replaced by a double: an object is an actor when
    /// its byte at `+0xff` is 1.
    fn engine() -> Engine {
        let mut e = Engine::new();
        for page in [
            0x0101_1000,
            0x0101_6000,
            0x0101_7000,
            0x011a_d000,
            0x011d_e000,
            0x011f_9000,
        ] {
            e.map(page, 0x1000);
        }
        e.register(DYNAMIC_CAST, |e, a| {
            assert_eq!(a[2..], [RTTI_TES_OBJECT_REFR, RTTI_ACTOR, 0]);
            let actor = a[0] != 0 && e.mem.u8(a[0] + 0xff) == 1;
            (if actor { a[0] } else { 0 }).into_ret()
        });
        e.register(V_ADDRESS, |_, a| (a[0] + 0xe4).into_ret());
        e
    }

    /// An object whose vtable (in the heap) has the given `(byte offset,
    /// function)` slots; `actor` marks it for the cast double.
    fn object_with(e: &mut Engine, actor: bool, slots: &[(u32, u32)]) -> u32 {
        let vtable = e.mem.alloc(0x800);
        for (offset, function) in slots {
            e.mem.set_u32(vtable + offset, *function);
        }
        let object = e.mem.alloc(0x100);
        e.mem.set_u32(object, vtable);
        e.mem.set_u8(object + 0xff, actor as u8);
        object
    }

    /// The standard eight words with `this_obj` and `result` set (the result
    /// is a heap `double` preset to 7.0).
    fn script(e: &mut Engine, this_obj: u32) -> ScriptArgs {
        let result = e.mem.alloc(8);
        e.mem.set_f64(result, 7.0);
        ScriptArgs {
            param_info: 1,
            script_data: 2,
            this_obj: Ptr::new(this_obj),
            containing_obj: Ptr::new(4),
            script_obj: Ptr::new(5),
            event_list: 6,
            result: Ptr::new(result),
            opcode_offset: 8,
        }
    }

    /// `ParseParameters` double: returns `ok` and stores `outs` through its
    /// output pointers (after the seven fixed words).
    fn parse_gives(e: &mut Engine, ok: bool, outs: &[u32]) {
        let outs = outs.to_vec();
        e.register_double(PARSE_PARAMETERS, move |e, a| {
            for (i, value) in outs.iter().enumerate() {
                e.mem.set_u32(a[7 + i], *value);
            }
            ok.into_ret()
        });
    }

    fn start_log(e: &mut Engine) {
        e.call_log = Some(vec![]);
    }

    /// The argument words of every logged call to `addr`.
    fn calls(e: &Engine, addr: u32) -> Vec<Vec<u32>> {
        e.call_log
            .as_ref()
            .unwrap()
            .iter()
            .filter(|(a, _)| *a == addr)
            .map(|(_, words)| words.clone())
            .collect()
    }

    /// Registers doubles that just accept the call.
    fn accept(e: &mut Engine, addrs: &[u32]) {
        for addr in addrs {
            e.register(*addr, |_, _| Ret::default());
        }
    }

    /// The command-body entry for `addr` called with `a`.
    fn run(e: &mut Engine, addr: u32, a: ScriptArgs) -> bool {
        e.call(addr, &args![a]).bool()
    }

    fn result_of(e: &Engine, a: ScriptArgs) -> f64 {
        e.mem.f64(a.result.addr())
    }

    // ---- the condition-function twins --------------------------------------

    /// A command that only forwards to `callee(thisObj, 0, 0, result)`.
    fn check_plain_condition(addr: u32, callee: u32) {
        let mut e = engine();
        e.register(callee, |_, a| (a[0] == 0x40).into_ret());
        let a = script(&mut e, 0x40);
        start_log(&mut e);
        assert!(run(&mut e, addr, a));
        assert_eq!(calls(&e, callee), vec![vec![0x40, 0, 0, a.result.addr()]]);
        // The callee's answer is the command's answer.
        let other = script(&mut e, 0x41);
        assert!(!run(&mut e, addr, other));
    }

    /// A command that parses one argument and forwards to `callee(thisObj,
    /// argument, 0, result)`.
    fn check_one_argument_condition(addr: u32, callee: u32) {
        let mut e = engine();
        parse_gives(&mut e, true, &[0x77]);
        e.register(callee, |_, _| true.into_ret());
        let a = script(&mut e, 0x40);
        start_log(&mut e);
        assert!(run(&mut e, addr, a));
        // ParseParameters gets: info, data, opcode offset, thisObj,
        // containing, script, event list, then the address of the local.
        assert_eq!(
            calls(&e, PARSE_PARAMETERS)[0][..7],
            [1, 2, 8, 0x40, 4, 5, 6]
        );
        assert_eq!(
            calls(&e, callee),
            vec![vec![0x40, 0x77, 0, a.result.addr()]]
        );
        // Parameters that do not parse: false, nothing called.
        parse_gives(&mut e, false, &[]);
        start_log(&mut e);
        assert!(!run(&mut e, addr, a));
        assert!(calls(&e, callee).is_empty());
    }

    #[test]
    fn get_disposition_passes_the_parsed_actor() {
        check_one_argument_condition(0x005c_8450, GET_DISPOSITION_CONDITION);
    }

    #[test]
    fn get_dead_count_passes_the_parsed_form() {
        check_one_argument_condition(0x005c_85c0, GET_DEAD_COUNT_CONDITION);
    }

    #[test]
    fn get_random_percent_forwards_this_obj_and_result() {
        check_plain_condition(0x005c_84b0, GET_RANDOM_PERCENT_CONDITION);
    }

    #[test]
    fn get_level_forwards_this_obj_and_result() {
        check_plain_condition(0x005c_84d0, GET_LEVEL_CONDITION);
    }

    #[test]
    fn get_armor_rating_forwards_this_obj_and_result() {
        check_plain_condition(0x005c_84f0, GET_ARMOR_RATING_CONDITION);
    }

    #[test]
    fn get_alert_forwards_this_obj_and_result() {
        check_plain_condition(0x005c_9700, GET_ALERT_CONDITION);
    }

    #[test]
    fn the_005a4340_command_forwards_this_obj_and_result() {
        check_plain_condition(0x005c_9720, FN_005A4340);
    }

    #[test]
    fn mod_disposition_changes_the_actor_then_reads_it_back() {
        let mut e = engine();
        // The actor argument is 0x77, the amount -5.
        parse_gives(&mut e, true, &[0x77, (-5i32) as u32]);
        e.register(GET_DISPOSITION_CONDITION, |_, _| true.into_ret());
        e.register(0x0900_0100, |_, _| Ret::default());
        let actor = object_with(&mut e, true, &[(0x460, 0x0900_0100)]);
        let a = script(&mut e, actor);
        start_log(&mut e);
        assert!(run(&mut e, 0x005c_8510, a));
        // Slot 0x460 gets the argument and the amount as a float.
        assert_eq!(
            calls(&e, 0x0900_0100),
            vec![vec![actor, 0x77, (-5.0f32).to_bits()]]
        );
        assert_eq!(
            calls(&e, GET_DISPOSITION_CONDITION),
            vec![vec![actor, 0x77, 0, a.result.addr()]]
        );

        // Not an actor: false, no slot call.
        let thing = object_with(&mut e, false, &[(0x460, 0x0900_0100)]);
        let a = script(&mut e, thing);
        start_log(&mut e);
        assert!(!run(&mut e, 0x005c_8510, a));
        assert!(calls(&e, 0x0900_0100).is_empty());

        // No actor argument: false.
        parse_gives(&mut e, true, &[0, 3]);
        let a = script(&mut e, actor);
        start_log(&mut e);
        assert!(!run(&mut e, 0x005c_8510, a));
        assert!(calls(&e, 0x0900_0100).is_empty());

        // Bad parameters: false, no cast.
        parse_gives(&mut e, false, &[]);
        start_log(&mut e);
        assert!(!run(&mut e, 0x005c_8510, a));
        assert!(calls(&e, DYNAMIC_CAST).is_empty());
    }

    // ---- map markers ---------------------------------------------------------

    /// A marker reference whose map marker data (at `+0xd0`, bytes: `+0`
    /// visible, `+1` has a travel location) is as given, with slot `0x48`
    /// recorded at `0x0900_0100`.
    fn marker_ref(e: &mut Engine, visible: bool, travel: bool) -> u32 {
        let marker = object_with(e, false, &[(0x48, 0x0900_0100)]);
        e.mem.set_u8(marker + 0xd0, visible as u8);
        e.mem.set_u8(marker + 0xd1, travel as u8);
        marker
    }

    /// Doubles for the map marker data accessors; the reference `0x66` has
    /// no map marker data.
    fn map_engine() -> Engine {
        let mut e = engine();
        e.register(GET_MAP_MARKER_DATA, |_, a| {
            (if a[0] == 0x66 { 0 } else { a[0] + 0xd0 }).into_ret()
        });
        e.register(MAP_MARKER_GET_VISIBLE, |e, a| {
            (e.mem.u8(a[0]) != 0).into_ret()
        });
        e.register(MAP_MARKER_GET_TRAVEL_LOC, |e, a| {
            (e.mem.u8(a[0] + 1) != 0).into_ret()
        });
        e.register(MAP_MARKER_SET_VISIBLE, |e, a| {
            e.mem.set_u8(a[0], a[1] as u8);
            Ret::default()
        });
        e.register(MAP_MARKER_SET_TRAVEL_LOC, |e, a| {
            e.mem.set_u8(a[0] + 1, a[1] as u8);
            Ret::default()
        });
        e.register(0x0900_0100, |_, _| Ret::default());
        e.register(BS_STRING_TEXT, |_, a| (a[0] + 1).into_ret());
        accept(&mut e, &[SHOW_MESSAGE]);
        e.set_global(MESSAGE_VOLUME, 2.0f32);
        e
    }

    #[test]
    fn show_map_makes_a_hidden_marker_visible_and_announces_it() {
        let mut e = map_engine();
        let marker = marker_ref(&mut e, false, false);
        parse_gives(&mut e, true, &[marker, 1]);
        let a = script(&mut e, 0);
        start_log(&mut e);
        assert!(run(&mut e, 0x005c_8620, a));
        // Visible and with a travel location afterwards.
        assert_eq!(e.mem.u8(marker + 0xd0), 1);
        assert_eq!(e.mem.u8(marker + 0xd1), 1);
        assert_eq!(calls(&e, 0x0900_0100), vec![vec![marker, 0x8000_0000]]);
        assert_eq!(
            calls(&e, SHOW_MESSAGE),
            vec![vec![
                MAP_MARKER_MESSAGE + 1,
                0,
                ICON_MAP,
                SOUND_MAP_MARKER_ADDED,
                2.0f32.to_bits(),
                0
            ]]
        );
        assert_eq!(result_of(&e, a), 1.0);
    }

    #[test]
    fn show_map_announces_only_what_changed() {
        let mut e = map_engine();
        // Already visible, travel location already set: silent.
        let marker = marker_ref(&mut e, true, true);
        parse_gives(&mut e, true, &[marker, 1]);
        let a = script(&mut e, 0);
        start_log(&mut e);
        assert!(run(&mut e, 0x005c_8620, a));
        assert!(calls(&e, SHOW_MESSAGE).is_empty());
        // Visible but no travel location: without the flag it is left alone
        // and nothing is announced; with the flag it is set and announced.
        let marker = marker_ref(&mut e, true, false);
        parse_gives(&mut e, true, &[marker, 0]);
        start_log(&mut e);
        assert!(run(&mut e, 0x005c_8620, a));
        assert_eq!(e.mem.u8(marker + 0xd1), 0);
        assert!(calls(&e, SHOW_MESSAGE).is_empty());
        parse_gives(&mut e, true, &[marker, 1]);
        start_log(&mut e);
        assert!(run(&mut e, 0x005c_8620, a));
        assert_eq!(e.mem.u8(marker + 0xd1), 1);
        assert_eq!(calls(&e, SHOW_MESSAGE).len(), 1);
    }

    #[test]
    fn show_map_without_marker_data_or_reference_only_sets_the_result() {
        let mut e = map_engine();
        for reference in [0x66u32, 0] {
            parse_gives(&mut e, true, &[reference, 1]);
            let a = script(&mut e, 0);
            start_log(&mut e);
            assert!(run(&mut e, 0x005c_8620, a));
            assert!(calls(&e, MAP_MARKER_SET_VISIBLE).is_empty());
            assert!(calls(&e, SHOW_MESSAGE).is_empty());
            assert_eq!(result_of(&e, a), 1.0);
        }
        // Bad parameters: false and the result untouched.
        parse_gives(&mut e, false, &[]);
        let a = script(&mut e, 0);
        assert!(!run(&mut e, 0x005c_8620, a));
        assert_eq!(result_of(&e, a), 7.0);
    }

    // ---- actors ------------------------------------------------------------------

    #[test]
    fn set_alert_passes_whether_the_integer_is_positive() {
        let mut e = engine();
        accept(&mut e, &[ACTOR_SET_ALERT]);
        let actor = object_with(&mut e, true, &[]);
        for (value, alert) in [(3i32, 1), (0, 0), (-1, 0)] {
            parse_gives(&mut e, true, &[value as u32]);
            let a = script(&mut e, actor);
            start_log(&mut e);
            assert!(run(&mut e, 0x005c_9670, a));
            assert_eq!(calls(&e, ACTOR_SET_ALERT), vec![vec![actor, alert]]);
        }
        // Not an actor, or no reference: nothing set, still true.
        parse_gives(&mut e, true, &[1]);
        let thing = object_with(&mut e, false, &[]);
        for this_obj in [thing, 0] {
            let a = script(&mut e, this_obj);
            start_log(&mut e);
            assert!(run(&mut e, 0x005c_9670, a));
            assert!(calls(&e, ACTOR_SET_ALERT).is_empty());
        }
        // Bad parameters: false.
        parse_gives(&mut e, false, &[]);
        let a = script(&mut e, actor);
        assert!(!run(&mut e, 0x005c_9670, a));
    }

    #[test]
    fn the_005c9740_command_calls_the_parsed_object() {
        let mut e = engine();
        accept(&mut e, &[FN_0060C950]);
        parse_gives(&mut e, true, &[0x1234]);
        let a = script(&mut e, 0);
        start_log(&mut e);
        assert!(run(&mut e, 0x005c_9740, a));
        assert_eq!(calls(&e, FN_0060C950), vec![vec![0x1234, 1]]);
        parse_gives(&mut e, false, &[]);
        start_log(&mut e);
        assert!(!run(&mut e, 0x005c_9740, a));
        assert!(calls(&e, FN_0060C950).is_empty());
    }

    /// Doubles for the process/extra data callees of the target commands:
    /// returns the engine, an actor with a process and the process. The
    /// actor's slot `0x48` is `0x0900_0100`; the process's slots `0x62c`,
    /// `0x648` and `0x7ac` are `0x0900_0101` to `0x0900_0103`.
    fn target_engine() -> (Engine, u32, u32) {
        let mut e = engine();
        accept(
            &mut e,
            &[
                SET_ITEM_DROPPER,
                SET_TARGETED,
                ACTOR_SET_LOOK_AT_TARGET,
                ACTOR_CLEAR_LOOK_AT_TARGET,
                0x0900_0100,
                0x0900_0101,
                0x0900_0102,
                0x0900_0103,
            ],
        );
        e.register(EXTRA_DATA_LIST, |_, a| (a[0] + 0x44).into_ret());
        let process = object_with(
            &mut e,
            false,
            &[
                (0x62c, 0x0900_0101),
                (0x648, 0x0900_0102),
                (0x7ac, 0x0900_0103),
            ],
        );
        // Actors flagged at `+0xfe` have a process.
        e.register_double(GET_SAVED_ACQUIRE_OBJECT, move |e, a| {
            (if e.mem.u8(a[0] + 0xfe) != 0 {
                process
            } else {
                0
            })
            .into_ret()
        });
        let actor = object_with(&mut e, true, &[(0x48, 0x0900_0100)]);
        e.mem.set_u8(actor + 0xfe, 1);
        (e, actor, process)
    }

    #[test]
    fn target_command_without_the_flag_sets_the_item_dropper() {
        let (mut e, actor, process) = target_engine();
        let target = object_with(&mut e, false, &[]);
        parse_gives(&mut e, true, &[target, 0]);
        let a = script(&mut e, actor);
        start_log(&mut e);
        assert!(run(&mut e, 0x005c_9790, a));
        assert_eq!(calls(&e, 0x0900_0101), vec![vec![process, target]]);
        assert_eq!(
            calls(&e, SET_ITEM_DROPPER),
            vec![vec![actor + 0x44, target]]
        );
        assert!(calls(&e, ACTOR_SET_LOOK_AT_TARGET).is_empty());
        assert_eq!(calls(&e, 0x0900_0100), vec![vec![actor, 0x8000_0000]]);
    }

    #[test]
    fn target_command_with_the_flag_looks_at_the_target() {
        let (mut e, actor, process) = target_engine();
        // The target's position (slot 0x1f4) is three floats.
        let target = object_with(&mut e, false, &[(0x1f4, V_ADDRESS)]);
        e.mem.set_f32(target + 0xe4, 1.0);
        e.mem.set_f32(target + 0xe8, 2.0);
        e.mem.set_f32(target + 0xec, 3.0);
        parse_gives(&mut e, true, &[target, 1]);
        let a = script(&mut e, actor);
        start_log(&mut e);
        assert!(run(&mut e, 0x005c_9790, a));
        assert_eq!(
            calls(&e, ACTOR_SET_LOOK_AT_TARGET),
            vec![vec![
                actor,
                1.0f32.to_bits(),
                2.0f32.to_bits(),
                3.0f32.to_bits()
            ]]
        );
        assert_eq!(calls(&e, 0x0900_0103), vec![vec![process, target]]);
        assert_eq!(calls(&e, SET_TARGETED), vec![vec![target, 1]]);
        assert!(calls(&e, SET_ITEM_DROPPER).is_empty());
        assert_eq!(calls(&e, 0x0900_0100), vec![vec![actor, 0x8000_0000]]);
    }

    #[test]
    fn target_command_needs_an_actor_with_a_process_and_a_target() {
        let (mut e, actor, _) = target_engine();
        let target = object_with(&mut e, false, &[]);
        let idle = object_with(&mut e, true, &[(0x48, 0x0900_0100)]);
        let thing = object_with(&mut e, false, &[]);
        for (this_obj, argument) in [(idle, target), (thing, target), (0, target), (actor, 0)] {
            parse_gives(&mut e, true, &[argument, 0]);
            let a = script(&mut e, this_obj);
            start_log(&mut e);
            assert!(run(&mut e, 0x005c_9790, a));
            assert!(calls(&e, 0x0900_0100).is_empty());
            assert!(calls(&e, SET_ITEM_DROPPER).is_empty());
        }
        parse_gives(&mut e, false, &[]);
        let a = script(&mut e, actor);
        assert!(!run(&mut e, 0x005c_9790, a));
    }

    #[test]
    fn release_command_clears_the_target_state_of_an_actor_with_a_process() {
        let (mut e, actor, process) = target_engine();
        let a = script(&mut e, actor);
        start_log(&mut e);
        assert!(run(&mut e, 0x005c_98e0, a));
        let wanted = [
            SET_ITEM_DROPPER,
            0x0900_0102,
            ACTOR_CLEAR_LOOK_AT_TARGET,
            0x0900_0103,
        ];
        let order: Vec<u32> = e
            .call_log
            .as_ref()
            .unwrap()
            .iter()
            .map(|(addr, _)| *addr)
            .filter(|addr| wanted.contains(addr))
            .collect();
        assert_eq!(order, wanted);
        assert_eq!(calls(&e, SET_ITEM_DROPPER), vec![vec![actor + 0x44, 0]]);
        assert_eq!(calls(&e, 0x0900_0102), vec![vec![process, 1]]);
        assert_eq!(calls(&e, ACTOR_CLEAR_LOOK_AT_TARGET), vec![vec![actor]]);
        assert_eq!(calls(&e, 0x0900_0103), vec![vec![process, 0]]);

        // An actor without a process only loses its item dropper; other
        // references, and none, do nothing.
        let idle = object_with(&mut e, true, &[]);
        let thing = object_with(&mut e, false, &[]);
        let a = script(&mut e, idle);
        start_log(&mut e);
        assert!(run(&mut e, 0x005c_98e0, a));
        assert_eq!(calls(&e, SET_ITEM_DROPPER).len(), 1);
        assert!(calls(&e, ACTOR_CLEAR_LOOK_AT_TARGET).is_empty());
        for this_obj in [thing, 0] {
            let a = script(&mut e, this_obj);
            start_log(&mut e);
            assert!(run(&mut e, 0x005c_98e0, a));
            assert!(calls(&e, SET_ITEM_DROPPER).is_empty());
        }
    }

    // ---- HDR ------------------------------------------------------------------------

    #[test]
    fn set_hdr_values_stores_the_absolute_values() {
        let mut e = engine();
        e.register(FLOAT_ABS, |_, a| f32::from_bits(a[0]).abs().into_ret());
        parse_gives(&mut e, true, &[(-1.5f32).to_bits(), 2.5f32.to_bits()]);
        let a = script(&mut e, 0);
        assert!(run(&mut e, 0x005c_9980, a));
        assert_eq!(e.global::<f32>(HDR_SUNLIGHT_DIMMER), 1.5);
        assert_eq!(e.global::<f32>(HDR_LUM_RAMP), 2.5);
        // Bad parameters: false and the globals untouched.
        e.set_global(HDR_SUNLIGHT_DIMMER, 9.0f32);
        parse_gives(&mut e, false, &[]);
        assert!(!run(&mut e, 0x005c_9980, a));
        assert_eq!(e.global::<f32>(HDR_SUNLIGHT_DIMMER), 9.0);
    }

    /// An image space manager double: effect 1 is an HDR effect whose
    /// `pfData` (at `+0x6c`) holds the numbers 10.0, 11.0, ... 25.0, so that
    /// index `i` reads `10 + i`.
    fn hdr_engine() -> (Engine, u32, u32) {
        let mut e = engine();
        let effect = e.mem.alloc(0x80);
        let data = e.mem.alloc(0x40);
        for i in 0..16 {
            e.mem.set_f32(data + 4 * i, 10.0 + i as f32);
        }
        e.mem.set_u32(effect + 0x6c, data);
        let manager = e.mem.alloc(0x10);
        e.register_double(GET_IMAGE_SPACE_MANAGER, move |_, _| manager.into_ret());
        e.register_double(IMAGE_SPACE_GET_EFFECT, move |_, a| {
            assert_eq!((a[0], a[1]), (manager, 1));
            effect.into_ret()
        });
        (e, manager, effect)
    }

    /// `via_manager` reads index `index` of the HDR data through the
    /// manager; `direct` reads it from the effect.
    fn check_hdr_accessors(via_manager: u32, direct: u32, index: u32) {
        let (mut e, manager, effect) = hdr_engine();
        let expected = 10.0 + index as f32;
        assert_eq!(e.call(direct, &args![effect]).f32(), expected);
        assert_eq!(e.call(via_manager, &args![manager]).f32(), expected);
    }

    #[test]
    fn emissive_mult_accessors_read_index_3() {
        check_hdr_accessors(0x005c_9b30, 0x005c_9b50, 3);
    }

    #[test]
    fn blur_radius_accessors_read_index_1() {
        check_hdr_accessors(0x005c_9b70, 0x005c_9b90, 1);
    }

    #[test]
    fn bright_clamp_accessors_read_index_7() {
        check_hdr_accessors(0x005c_9bb0, 0x005c_9bd0, 7);
    }

    #[test]
    fn bright_scale_accessors_read_index_6() {
        check_hdr_accessors(0x005c_9bf0, 0x005c_9c10, 6);
    }

    #[test]
    fn eye_adapt_speed_accessors_read_index_0() {
        check_hdr_accessors(0x005c_9c30, 0x005c_9c50, 0);
    }

    #[test]
    fn target_lum_accessors_read_index_4() {
        check_hdr_accessors(0x005c_9c70, 0x005c_9c90, 4);
    }

    #[test]
    fn upper_lum_clamp_accessors_read_index_5() {
        check_hdr_accessors(0x005c_9cb0, 0x005c_9cd0, 5);
    }

    /// The `double`s in a list of argument words.
    fn doubles(words: &[u32]) -> Vec<f64> {
        words
            .chunks(2)
            .map(|pair| f64::from_bits(pair[0] as u64 | (pair[1] as u64) << 32))
            .collect()
    }

    #[test]
    fn print_hdr_params_prints_the_lines_in_order() {
        let (mut e, _, _) = hdr_engine();
        e.set_global(HDR_SUNLIGHT_DIMMER, 0.5f32);
        e.set_global(HDR_LUM_RAMP, 0.25f32);
        e.set_global(HDR_GRASS_DIMMER, 0.75f32);
        e.set_global(HDR_TREE_DIMMER, 0.125f32);
        accept(&mut e, &[CONSOLE_PRINT]);
        start_log(&mut e);
        assert!(e.call(0x005c_9a00, &args![]).bool());
        let prints = calls(&e, CONSOLE_PRINT);
        let formats: Vec<u32> = prints.iter().map(|words| words[0]).collect();
        assert_eq!(
            formats,
            vec![
                MSG_HDR_PARAMS,
                MSG_SISG,
                FORMAT_BLUR,
                FORMAT_BRIGHT,
                MSG_SSP,
                FORMAT_SUNLIGHT,
                MSG_SHP,
                FORMAT_EYE_ADAPT,
                FORMAT_DIMMERS,
                FORMAT_LUM
            ]
        );
        // The values (index i of the HDR data is 10 + i): blur radius (1);
        // bright clamp (7) and scale (6); sunlight dimmer and ramp; eye
        // adapt speed (0) and emissive multiplier (3); tree and grass
        // dimmers; upper luminance clamp (5) and target (4).
        assert_eq!(doubles(&prints[2][1..]), vec![11.0]);
        assert_eq!(doubles(&prints[3][1..]), vec![17.0, 16.0]);
        assert_eq!(doubles(&prints[5][1..]), vec![0.5, 0.25]);
        assert_eq!(doubles(&prints[7][1..]), vec![10.0, 13.0]);
        assert_eq!(doubles(&prints[8][1..]), vec![0.125, 0.75]);
        assert_eq!(doubles(&prints[9][1..]), vec![15.0, 14.0]);
    }

    // ---- debug axes -------------------------------------------------------------------

    /// Doubles for the callees of `005c9cf0`. The reference has a 3D when
    /// `bound_radius` is given (slot `0x1d0`), and its position (slot
    /// `0x1f4`) is at `reference + 0xe4`.
    fn axes_engine(bound_radius: Option<f32>) -> (Engine, u32) {
        let mut e = engine();
        e.register(OPERATOR_NEW, |e, a| e.mem.alloc(a[0]).into_ret());
        e.register(NI_ALLOC, |e, a| e.mem.alloc(a[0]).into_ret());
        accept(
            &mut e,
            &[
                VECTOR_CONSTRUCT_ITERATOR,
                SET_TRANSLATE,
                ADD_TEMP_DEBUG_OBJECT,
            ],
        );
        e.register(NI_COLOR_A_CONSTRUCT, |e, a| {
            for i in 0..4 {
                e.mem.set_u32(a[0] + 4 * i, a[1 + i as usize]);
            }
            a[0].into_ret()
        });
        e.register(NI_POINT3_CONSTRUCT, |e, a| {
            for i in 0..3 {
                e.mem.set_u32(a[0] + 4 * i, a[1 + i as usize]);
            }
            a[0].into_ret()
        });
        e.register(NI_LINES_CONSTRUCT, |_, a| a[0].into_ret());
        e.register(GET_WORLD_BOUND, |_, a| (a[0] + 0x20).into_ret());
        e.register(BOUND_RADIUS, |e, a| e.mem.f32(a[0] + 0xc).into_ret());
        let node = e.mem.alloc(0x40);
        if let Some(radius) = bound_radius {
            e.mem.set_f32(node + 0x20 + 0xc, radius);
        }
        let has_3d = bound_radius.is_some();
        e.register_double(0x0900_0200, move |_, _| {
            (if has_3d { node } else { 0 }).into_ret()
        });
        e.set_global(DEFAULT_AXIS_LENGTH, 10.0f32);
        e.set_global(DEBUG_OBJECT_SECONDS, 20.0f32);
        e.set_global(HALF, 0.5f64);
        e.set_global(TES_SINGLETON, 0x5555u32);
        let reference = object_with(&mut e, false, &[(0x1d0, 0x0900_0200), (0x1f4, V_ADDRESS)]);
        (e, reference)
    }

    /// The lines the command made: its vertices, colors, connections and
    /// the `NiLines` object.
    fn lines_made(e: &Engine) -> ([f32; 18], [[f32; 4]; 6], Vec<u8>, u32) {
        let constructed = calls(e, NI_LINES_CONSTRUCT);
        assert_eq!(constructed.len(), 1);
        let words = &constructed[0];
        // storage, count, vertices, colors, texture coordinates, texture
        // sets, NBT method, connections.
        assert_eq!(words[1], 6);
        assert_eq!(words[4..7], [0, 1, 0]);
        let mut vertices = [0.0; 18];
        for (i, v) in vertices.iter_mut().enumerate() {
            *v = e.mem.f32(words[2] + 4 * i as u32);
        }
        let mut colors = [[0.0; 4]; 6];
        for (i, color) in colors.iter_mut().enumerate() {
            for (j, c) in color.iter_mut().enumerate() {
                *c = e.mem.f32(words[3] + 16 * i as u32 + 4 * j as u32);
            }
        }
        let connections = (0..6).map(|i| e.mem.u8(words[7] + i)).collect();
        (vertices, colors, connections, words[0])
    }

    #[test]
    fn debug_axes_use_the_default_length_when_the_bound_is_large() {
        let (mut e, reference) = axes_engine(Some(100.0));
        let a = script(&mut e, reference);
        start_log(&mut e);
        assert!(run(&mut e, 0x005c_9cf0, a));
        let (vertices, colors, connections, lines) = lines_made(&e);
        assert_eq!(
            vertices,
            [
                -10.0, 0.0, 0.0, 10.0, 0.0, 0.0, 0.0, -10.0, 0.0, 0.0, 10.0, 0.0, 0.0, 0.0, -10.0,
                0.0, 0.0, 10.0
            ]
        );
        assert_eq!(colors, [[1.0, 1.0, 0.0, 1.0]; 6]);
        assert_eq!(connections, vec![1, 0, 1, 0, 1, 0]);
        // Placed at the reference's position and kept for 20 seconds.
        assert_eq!(
            calls(&e, SET_TRANSLATE),
            vec![vec![lines, reference + 0xe4]]
        );
        assert_eq!(
            calls(&e, ADD_TEMP_DEBUG_OBJECT),
            vec![vec![0x5555, lines, 20.0f32.to_bits()]]
        );
        assert_eq!(calls(&e, VECTOR_CONSTRUCT_ITERATOR).len(), 2);
    }

    #[test]
    fn debug_axes_shrink_to_half_the_bound_radius() {
        let (mut e, reference) = axes_engine(Some(8.0));
        let a = script(&mut e, reference);
        start_log(&mut e);
        assert!(run(&mut e, 0x005c_9cf0, a));
        let (vertices, ..) = lines_made(&e);
        assert_eq!(vertices[..6], [-4.0, 0.0, 0.0, 4.0, 0.0, 0.0]);
        assert_eq!(vertices[12..], [0.0, 0.0, -4.0, 0.0, 0.0, 4.0]);
        // A bound exactly as large as the default keeps the default.
        let (mut e, reference) = axes_engine(Some(20.0));
        let a = script(&mut e, reference);
        start_log(&mut e);
        assert!(run(&mut e, 0x005c_9cf0, a));
        assert_eq!(lines_made(&e).0[0], -10.0);
    }

    #[test]
    fn debug_axes_without_3d_or_reference() {
        let (mut e, reference) = axes_engine(None);
        let a = script(&mut e, reference);
        start_log(&mut e);
        assert!(run(&mut e, 0x005c_9cf0, a));
        assert_eq!(lines_made(&e).0[0], -10.0);
        assert!(calls(&e, GET_WORLD_BOUND).is_empty());
        // No reference: nothing made.
        let a = script(&mut e, 0);
        start_log(&mut e);
        assert!(run(&mut e, 0x005c_9cf0, a));
        assert!(calls(&e, OPERATOR_NEW).is_empty());
        assert!(calls(&e, ADD_TEMP_DEBUG_OBJECT).is_empty());
    }

    // ---- script event flags ----------------------------------------------------------

    #[test]
    fn event_flag_body_asks_the_event_list_for_the_event_and_mask() {
        let mut e = engine();
        e.register(SCRIPT_EVENT_LIST_HAS_EVENT, |_, a| {
            (a[1] == 0x77).into_ret()
        });
        let a = script(&mut e, 0x40);
        start_log(&mut e);
        e.call(
            0x005c_a1c0,
            &args![0x20u32, 0x40u32, 0x77u32, 0x99u32, a.result],
        );
        assert_eq!(
            calls(&e, SCRIPT_EVENT_LIST_HAS_EVENT),
            vec![vec![0x99, 0x77, 0x20]]
        );
        assert_eq!(result_of(&e, a), 1.0);
        // Event not in the list: 0.0.
        e.call(
            0x005c_a1c0,
            &args![0x20u32, 0x40u32, 0x78u32, 0x99u32, a.result],
        );
        assert_eq!(result_of(&e, a), 0.0);
        // No reference or no event list: 0.0 and no question asked.
        start_log(&mut e);
        e.mem.set_f64(a.result.addr(), 7.0);
        e.call(
            0x005c_a1c0,
            &args![0x20u32, 0u32, 0x77u32, 0x99u32, a.result],
        );
        assert_eq!(result_of(&e, a), 0.0);
        e.mem.set_f64(a.result.addr(), 7.0);
        e.call(
            0x005c_a1c0,
            &args![0x20u32, 0x40u32, 0x77u32, 0u32, a.result],
        );
        assert_eq!(result_of(&e, a), 0.0);
        assert!(calls(&e, SCRIPT_EVENT_LIST_HAS_EVENT).is_empty());
    }

    /// An event flag command parses one argument and asks for it with `mask`.
    fn check_event_flag_command(addr: u32, mask: u32) {
        let mut e = engine();
        e.register(SCRIPT_EVENT_LIST_HAS_EVENT, |_, _| true.into_ret());
        parse_gives(&mut e, true, &[0x77]);
        let a = script(&mut e, 0x40);
        start_log(&mut e);
        assert!(run(&mut e, addr, a));
        // The event list is the sixth word (6 in `script`).
        assert_eq!(
            calls(&e, SCRIPT_EVENT_LIST_HAS_EVENT),
            vec![vec![6, 0x77, mask]]
        );
        assert_eq!(result_of(&e, a), 1.0);
        // Bad parameters: false and the result untouched.
        parse_gives(&mut e, false, &[]);
        let a = script(&mut e, 0x40);
        assert!(!run(&mut e, addr, a));
        assert_eq!(result_of(&e, a), 7.0);
    }

    #[test]
    fn event_flag_command_005ca200_uses_mask_1() {
        check_event_flag_command(0x005c_a200, 1);
    }

    #[test]
    fn event_flag_command_005ca260_uses_mask_0x4000() {
        check_event_flag_command(0x005c_a260, 0x4000);
    }

    #[test]
    fn event_flag_command_005ca2d0_uses_mask_2() {
        check_event_flag_command(0x005c_a2d0, 2);
    }

    #[test]
    fn event_flag_command_005ca330_uses_mask_8() {
        check_event_flag_command(0x005c_a330, 8);
    }

    #[test]
    fn event_flag_command_005ca390_uses_mask_4() {
        check_event_flag_command(0x005c_a390, 4);
    }

    #[test]
    fn the_argumentless_event_flag_command_asks_event_0_with_mask_0x400000() {
        let mut e = engine();
        e.register(SCRIPT_EVENT_LIST_HAS_EVENT, |_, _| false.into_ret());
        let a = script(&mut e, 0x40);
        start_log(&mut e);
        assert!(run(&mut e, 0x005c_a510, a));
        assert_eq!(
            calls(&e, SCRIPT_EVENT_LIST_HAS_EVENT),
            vec![vec![6, 0, 0x40_0000]]
        );
        assert_eq!(result_of(&e, a), 0.0);
        e.register(SCRIPT_EVENT_LIST_HAS_EVENT, |_, _| true.into_ret());
        assert!(run(&mut e, 0x005c_a510, a));
        assert_eq!(result_of(&e, a), 1.0);
        // No parameters are read.
        assert!(calls(&e, PARSE_PARAMETERS).is_empty());
    }

    // ---- actions and the 0x168 slot ---------------------------------------------------

    #[test]
    fn on_activate_reports_action_2_and_clears_both_actions() {
        let mut e = engine();
        e.register(HAS_ACTION, |_, a| (a[1] == 2 && a[0] == 0x40).into_ret());
        accept(&mut e, &[CLEAR_ACTION]);
        parse_gives(&mut e, true, &[0]);
        let a = script(&mut e, 0x40);
        start_log(&mut e);
        assert!(run(&mut e, 0x005c_a3f0, a));
        assert_eq!(result_of(&e, a), 1.0);
        assert_eq!(calls(&e, CLEAR_ACTION), vec![vec![0x40, 1], vec![0x40, 2]]);
        // Action 2 not set (a different reference): 0.0, still cleared.
        let a = script(&mut e, 0x41);
        start_log(&mut e);
        assert!(run(&mut e, 0x005c_a3f0, a));
        assert_eq!(result_of(&e, a), 0.0);
        assert_eq!(calls(&e, CLEAR_ACTION).len(), 2);
        // No reference: 0.0, nothing touched.
        let a = script(&mut e, 0);
        start_log(&mut e);
        assert!(run(&mut e, 0x005c_a3f0, a));
        assert_eq!(result_of(&e, a), 0.0);
        assert!(calls(&e, CLEAR_ACTION).is_empty());
        // Bad parameters: false, result already zeroed.
        parse_gives(&mut e, false, &[]);
        let a = script(&mut e, 0x40);
        assert!(!run(&mut e, 0x005c_a3f0, a));
        assert_eq!(result_of(&e, a), 0.0);
    }

    #[test]
    fn pointer_test_is_true_for_a_non_null_pointer() {
        let mut e = engine();
        let cell = e.mem.alloc(4);
        assert!(!e.call(0x005c_a4f0, &args![cell]).bool());
        e.mem.set_u32(cell, 0x1234);
        assert!(e.call(0x005c_a4f0, &args![cell]).bool());
    }

    /// An object whose slot `0x168` gives `pointer`, a pointer to a cell
    /// holding `inner` (or null).
    fn slot_168_object(e: &mut Engine, pointer: bool, inner: u32) -> u32 {
        let object = object_with(e, false, &[(0x168, 0x0900_0300)]);
        let cell = e.mem.alloc(4);
        e.mem.set_u32(cell, inner);
        e.mem.set_u32(object + 0xe0, if pointer { cell } else { 0 });
        object
    }

    #[test]
    fn slot_168_command_is_1_unless_the_double_pointer_is_set() {
        let mut e = engine();
        e.register(0x0900_0300, |e, a| e.mem.u32(a[0] + 0xe0).into_ret());
        e.register(NI_POINTER_GET, |e, a| e.mem.u32(a[0]).into_ret());
        // Slot gives nothing: 1.0.
        let object = slot_168_object(&mut e, false, 0);
        let a = script(&mut e, object);
        assert!(run(&mut e, 0x005c_a470, a));
        assert_eq!(result_of(&e, a), 1.0);
        // Slot gives a pointer to null: 1.0.
        let object = slot_168_object(&mut e, true, 0);
        let a = script(&mut e, object);
        start_log(&mut e);
        assert!(run(&mut e, 0x005c_a470, a));
        assert_eq!(result_of(&e, a), 1.0);
        // The slot is asked twice before the null cell is found.
        assert_eq!(calls(&e, 0x0900_0300).len(), 2);
        // Slot gives a pointer to a non-null pointer: 0.0.
        let object = slot_168_object(&mut e, true, 0x4444);
        let a = script(&mut e, object);
        start_log(&mut e);
        assert!(run(&mut e, 0x005c_a470, a));
        assert_eq!(result_of(&e, a), 0.0);
        assert_eq!(calls(&e, 0x0900_0300).len(), 3);
    }
}
