//! `fallout shared/tesscriptfunctions.cpp` (Xbox PDB source unit), part 7: its functions from `005d7d30` up to
//! (not including) `005db810` in FalloutNV.exe 1.4.0.525, translated
//! (docs/ENGINE_CRATE.md). The unit's shared layouts and helpers are in
//! [`super::tesscriptfunctions`]; anything public there may be used here.
//!
//! Progress: the first 40 queue entries of the range (`005d7d30` to
//! `005d90a0`) are translated. The next session continues at `005d9160`.
//!
//! The bodies follow the conventions of the main file: `cdecl`, the eight
//! stack words as [`ScriptArgs`], `AL` as the result; a body that never reads
//! a word takes no parameters here. Objects of other classes (quests,
//! objectives, the player, topics, ...) are read at the PC offsets with a
//! comment instead of through a `layout!`.
//!
//! Notes on the exe's code that the translations rely on:
//! - Several bodies read the out-parameters of `Script::ParseParameters`
//!   from locals whose type the call does not show (floats, bytes, a text
//!   buffer). The words are passed on as the bits the game stores; a byte
//!   local is read through its low byte.
//! - `005d8a90` is slot 0 of the vtable at `0103cf54`, the one
//!   [`fn_005d8af0`] installs in a small functor that `005d8930` hands to the
//!   process lists (`0096bbc0`).
//! - The compiler's exception-unwinding frames (`005d8bf0`, `005d8e80`) are
//!   not translated, nor are the stack-cookie checks.

#[allow(unused_imports)]
use super::tesscriptfunctions::*;
#[allow(unused_imports)]
use crate::prelude::*;

// ---- Callees outside this part (by exe address) ----------------------------

/// `Script::ParseParameters` (Xbox PDB), `cdecl`, varargs.
const PARSE_PARAMETERS: u32 = 0x005a_ccb0;
/// The console print: format address first, `double` arguments take two
/// words (`cdecl`).
const CONSOLE_PRINT: u32 = 0x0070_3c00;
/// The logging stub of the unit (`005b5e40`): takes a format and its
/// arguments, does nothing and returns 0 in this build.
const LOG_STUB: u32 = 0x005b_5e40;
/// `__RTDynamicCast` (`object, 0, source type, target type, 0`).
const DYNAMIC_CAST: u32 = 0x00ec_43fb;
/// `_memset` (`dest, value, count`).
const MEMSET: u32 = 0x00ec_61c0;
/// `_strlen`.
const STRLEN: u32 = 0x00ec_6130;
/// `__stricmp` through a 21-byte wrapper (`cdecl`, two strings): 0 when
/// equal ignoring case.
const STRING_COMPARE_NO_CASE: u32 = 0x0040_4dc0;

/// `thiscall`: `*(this + 0x0c)`, the form id of a form (also read as the
/// element count of the array `00717e50` returns).
const GET_FORM_ID: u32 = 0x0084_e3a0;
/// `Script::PutNumericIDInDouble` (Xbox PDB), `cdecl` (`address of the id,
/// double* result`).
const PUT_NUMERIC_ID_IN_DOUBLE: u32 = 0x005a_cc70;
/// `this + 0x44`: the extra data list of a reference (`thiscall`).
const EXTRA_DATA_LIST: u32 = 0x005d_43c0;
/// Form type byte (`this + 4`, `thiscall`).
const FORM_TYPE: u32 = 0x0040_1170;
/// `BaseExtraList::RemoveExtra_ov2` (Xbox PDB), `thiscall` (`extra type`).
const REMOVE_EXTRA: u32 = 0x0041_0140;
/// `thiscall` on a reference: its `Process` (`MiddleHighProcess::
/// GetSavedAcquireObject` in the map: `*(this + 0x68)`).
const GET_PROCESS: u32 = 0x008d_8520;
/// `thiscall` on the player: the player's parent cell (`*(this + 0x40)`).
const PLAYER_GET_PARENT_CELL: u32 = 0x008d_6f30;
/// `TESForm::GetFile` (Xbox PDB), `thiscall` (`0`).
const GET_FILE: u32 = 0x0048_4e60;
/// `thiscall` on a form: bit `0x20` of its flags (`this + 8`).
const FORM_IS_DELETED_FLAG: u32 = 0x0044_0d80;
/// `TESObjectREFR::RunScript` (Xbox PDB), `thiscall`.
const RUN_SCRIPT: u32 = 0x0056_5870;
/// `TESObjectREFR::GetCalcLevel` (Xbox PDB), `thiscall` (`0`).
const GET_CALC_LEVEL: u32 = 0x0056_7e10;

/// `thiscall` on a quest (`index`): the objective with that index, or 0.
const QUEST_FIND_OBJECTIVE: u32 = 0x0060_c8e0;
/// `thiscall` on a quest: bit 1 of its flags (`tesconditionfunctions.cpp`).
const QUEST_FLAG_2: u32 = 0x0059_e400;
/// `thiscall` on an objective: `(this + 0x20) & 1`.
const OBJECTIVE_BIT_0: u32 = 0x005a_5e70;
/// `thiscall` on an objective: `1 < *(this + 0x20)`.
const OBJECTIVE_ABOVE_ONE: u32 = 0x005a_5dc0;
/// `thiscall` on an objective (`flag`).
const OBJECTIVE_SET: u32 = 0x005e_c5d0;
/// Virtual slot `0x138` of a quest: its name.
const QUEST_NAME_SLOT: u32 = 0x138;

/// `FalloutRadio::PipboyRadioEnable` (Xbox PDB), `cdecl` (`flag`).
const PIPBOY_RADIO_ENABLE: u32 = 0x0083_24e0;
/// `cdecl` (`value, flag`), `falloutradio.cpp`.
const RADIO_FN_00832240: u32 = 0x0083_2240;
/// `FalloutRadio::EnableNPCRadio` (Xbox PDB), `cdecl` (`reference, value`).
const ENABLE_NPC_RADIO: u32 = 0x0083_5810;
/// `FalloutRadio::DisableNPCRadio` (Xbox PDB), `cdecl` (`reference`).
const DISABLE_NPC_RADIO: u32 = 0x0083_5980;
/// `cdecl` (`reference`): the `AL` of `GetBroadcastState`.
const RADIO_BROADCAST_STATE: u32 = 0x0083_5b90;
/// `cdecl` (`reference, flag`), `falloutradio.cpp`.
const RADIO_FN_008359E0: u32 = 0x0083_59e0;
/// `FalloutRadio::Update` (Xbox PDB), `cdecl` (`1`).
const RADIO_UPDATE: u32 = 0x0083_2ad0;
/// `FalloutRadio::StartRadioConversation` (Xbox PDB), `cdecl`
/// (`reference, value`).
const START_RADIO_CONVERSATION: u32 = 0x0083_5be0;

/// A condition function of `tesconditionfunctions.cpp`, `cdecl`
/// (`thisObj, argument, 0, double* result`): returns its `AL`.
const CONDITION_FN_005A43B0: u32 = 0x005a_43b0;
/// `Script::GetIsObjectTypeConditionFunction` (Xbox PDB), same shape.
const GET_IS_OBJECT_TYPE_CONDITION: u32 = 0x005a_4410;
/// `Script::GetDialogueEmotionConditionFunction` (Xbox PDB), same shape.
const GET_DIALOGUE_EMOTION_CONDITION: u32 = 0x005a_4480;
/// `Script::GetDialogueEmotionValueConditionFunction` (Xbox PDB), same shape.
const GET_DIALOGUE_EMOTION_VALUE_CONDITION: u32 = 0x005a_4540;
/// `Script::IsWaterObjectConditionFunction` (Xbox PDB), same shape.
const IS_WATER_OBJECT_CONDITION: u32 = 0x005a_45c0;

/// `BSAudio::QInstance` (Xbox PDB): the audio singleton.
const AUDIO_INSTANCE: u32 = 0x0045_3a70;
/// `thiscall` on the audio object: the byte at `+6`.
const AUDIO_BYTE_6: u32 = 0x005b_b4d0;
/// `BSAudio::SetMultiThreaded` (Xbox PDB), `thiscall` (`flag`).
const AUDIO_SET_MULTI_THREADED: u32 = 0x00ad_7230;

/// `thiscall` on a reference (`value`), `tesobjectrefr.cpp`: the target of
/// the commands `005d83b0` and `005d83d0`.
const REFERENCE_FN_0056A9B0: u32 = 0x0056_a9b0;

/// Constructor of a three-float point (`thiscall`: `x, y, z`).
const POINT3_CONSTRUCT: u32 = 0x0041_6870;
/// `thiscall` on a cell (`point, text, index address`), `tesobjectcell.cpp`.
const CELL_FN_005578E0: u32 = 0x0055_78e0;
/// `thiscall` on a cell, `tesobjectcell.cpp`.
const CELL_FN_005578F0: u32 = 0x0055_78f0;

/// `thiscall` (`this + 0xf8` handed to [`DEREF`]) on the value
/// [`fn_005d8710`] returns.
const DEREF_AT_F8: u32 = 0x004e_6540;
/// `thiscall`: a flag (`AL`) computed from the object [`DEREF_AT_F8`] returns.
const FN_00456610: u32 = 0x0045_6610;
/// `thiscall` on the object [`DEREF_AT_F8`] returns (`flag`).
const FN_00450F90: u32 = 0x0045_0f90;
/// `cdecl` (`index`): `*(0x011f91c8 + index * 4)`.
const FN_00450B80: u32 = 0x0045_0b80;
/// `thiscall` (`value`) on what [`FN_00450B80`] returns.
const FN_00B5D270: u32 = 0x00b5_d270;
/// `thiscall`: `*this` (the word at the address it is given).
const DEREF: u32 = 0x0055_9450;

/// `cdecl` (`value, 0.0f, 0`).
const FN_004E0330: u32 = 0x004e_0330;
/// `cdecl`, no arguments.
const FN_004E0870: u32 = 0x004e_0870;
/// `cdecl` (`a, b, 0, c, d`), the four parsed words of `005d87a0`.
const FN_004DE8E0: u32 = 0x004d_e8e0;
/// `cdecl` (`float, flag`).
const FN_004DEF00: u32 = 0x004d_ef00;
/// `cdecl` (`text, byte`).
const FN_004DEED0: u32 = 0x004d_eed0;
/// `cdecl`, no arguments.
const FN_004DEFB0: u32 = 0x004d_efb0;

/// `ProcessLists` method (`0096bbc0`, `thiscall` on the process lists, one
/// functor object).
const PROCESS_LISTS_VISIT: u32 = 0x0096_bbc0;
/// `PlayerCharacter::ResetPlayerGreetFlag` (Xbox PDB), `thiscall`.
const RESET_PLAYER_GREET_FLAG: u32 = 0x0095_3ce0;
/// `TESTopic::GetTopic` (Xbox PDB), `cdecl` (`a, b`).
const GET_TOPIC: u32 = 0x0061_a2d0;
/// `thiscall`: the first node of the list the object at `[0x011c3f2c]`
/// keeps (`this + 0x108`).
const LIST_FIRST_NODE: u32 = 0x0046_12e0;
/// `thiscall` on a list node: the next node, or 0.
const LIST_NEXT_NODE: u32 = 0x0072_6070;
/// `thiscall` on a list node: the address of its data word.
const LIST_NODE_DATA: u32 = 0x0068_15c0;
/// `BaseProcess::GetActorPackageThatIsRunning` in the map (identical code
/// folded): `this + 4`, the array of `005d8930`'s inner loop.
const ARRAY_AT_4: u32 = 0x0071_7e50;
/// `thiscall` (`index`): the address of the element, `*(this + 4) +
/// index * 4`.
const ARRAY_ELEMENT_ADDRESS: u32 = 0x0087_7a30;
/// `thiscall` on an element.
const ELEMENT_FN_0061F280: u32 = 0x0061_f280;
/// `DialogMenu::DebugResetSaidOnceFlags` (Xbox PDB), `cdecl`.
const DEBUG_RESET_SAID_ONCE_FLAGS: u32 = 0x0076_4290;
/// `thiscall` on the player.
const PLAYER_FN_00966D70: u32 = 0x0096_6d70;

/// `cdecl` (`form id`): the form with that id (`tesform.cpp`).
const LOOKUP_FORM: u32 = 0x0048_39c0;
/// `TESGlobal` value setter (`float` argument, `thiscall`).
const TES_GLOBAL_SET_VALUE: u32 = 0x0046_dce0;
/// `Interface::CloseConsole` (Xbox PDB).
const CLOSE_CONSOLE: u32 = 0x0070_3da0;
/// `StartMenu::ChooseMainMenu` (Xbox PDB).
const CHOOSE_MAIN_MENU: u32 = 0x007d_0a70;

/// `BSStringT` constructor, `thiscall`.
const STRING_CONSTRUCT: u32 = 0x0040_37b0;
/// `BSStringT` destructor, `thiscall`.
const STRING_DESTRUCT: u32 = 0x0040_37d0;
/// `fallout shared/tesscriptfunctions.cpp` (another part of the unit):
/// `cdecl`, the eight words, a text and a string object.
const SCRIPT_BODY_005B4960: u32 = 0x005b_4960;
/// `Script::InitActionList` (Xbox PDB), `cdecl` (`reference, list`).
const INIT_ACTION_LIST: u32 = 0x005a_c190;
/// `Script::SetActionFlag` (Xbox PDB), `cdecl` (`reference, list, flag`).
const SET_ACTION_FLAG: u32 = 0x005a_c750;
/// `thiscall` on a reference (`action reference`).
const SET_ACTION_REF: u32 = 0x0057_2e10;
/// `thiscall` (`address of a reference word`) on the list at `0x011cacb8`.
const LIST_ADD_REFERENCE: u32 = 0x005a_e3d0;

/// `TESContainer::TESContainer` (Xbox PDB), `thiscall`.
const CONTAINER_CONSTRUCT: u32 = 0x0048_1610;
/// `thiscall` on the container being built (`item, count, 0`).
const CONTAINER_ADD_ITEM: u32 = 0x0048_18e0;
/// `thiscall` (`float`), `tescontainer.cpp`.
const CONTAINER_FN_00482090: u32 = 0x0048_2090;
/// `thiscall` (`reference, flag`), `tescontainer.cpp`.
const CONTAINER_FN_004821A0: u32 = 0x0048_21a0;
/// `thiscall` on the container being built: its destructor.
const CONTAINER_DESTRUCT: u32 = 0x0048_1680;
/// `thiscall` on `form + 0x30` (`level, count, container, 0`).
const FORM_ADD_LEVELED_ITEMS: u32 = 0x0048_7f70;
/// `thiscall`: `this + 0x18`, the first node of a list.
const LIST_AT_18: u32 = 0x0050_0940;
/// `thiscall` on a list node: true when the node is empty.
const NODE_IS_EMPTY: u32 = 0x0082_56d0;
/// `cdecl` (`reference, a, b, c, d, float`): returns a form or 0
/// (`tesscriptfunctions.cpp`, another part of the unit).
const SCRIPT_BODY_005C4B30: u32 = 0x005c_4b30;

// ---- Globals and constants ---------------------------------------------------

/// The `PlayerCharacter` singleton pointer.
const PLAYER: u32 = 0x011d_ea3c;
/// The process lists singleton (the `this` of [`PROCESS_LISTS_VISIT`]).
const PROCESS_LISTS: u32 = 0x011e_0e80;
/// The object `005d8710` is called on (an immediate address in the code).
const OBJECT_AT_11F2250: u32 = 0x011f_2250;
/// Pointer to the object whose list [`LIST_FIRST_NODE`] walks.
const LIST_OWNER: u32 = 0x011c_3f2c;
/// Pointer to the `TESGlobal` `ExitGame` sets.
const EXIT_GAME_GLOBAL: u32 = 0x011c_3f3c;
/// The list `005d8e20` adds the reference to (an immediate address).
const REFERENCE_LIST: u32 = 0x011c_acb8;
/// The form `ExitGame` looks up (a perk).
const EXIT_GAME_FORM_ID: u32 = 0x000e_d568;
/// RTTI type descriptor of `TESForm` (`.?AVTESForm@@`).
const RTTI_TES_FORM: u32 = 0x0118_3028;
/// RTTI type descriptor of `BGSPerk` (`.?AVBGSPerk@@`).
const RTTI_BGS_PERK: u32 = 0x0118_61dc;
/// Virtual slot `0x4a0` of the player: tests a perk (`perk, 0`), `AL`.
const PLAYER_HAS_PERK_SLOT: u32 = 0x4a0;
/// Vtable of the functor built by [`fn_005d8af0`] (slot 0 is `005d8a90`).
const FUNCTOR_VTABLE: u32 = 0x0103_cf54;
/// Vtable [`fn_005d8b10`] installs (the base class, a pure virtual).
const FUNCTOR_BASE_VTABLE: u32 = 0x0103_cf5c;
/// The table `005d8550` indexes (`0x10` bytes per entry, 10 entries).
const TABLE_AT_1203258: u32 = 0x0120_3258;
/// Byte at `+0x268` of the TLS block: commands echo to the console when set.
const TLS_ECHO: u32 = 0x268;
/// Dword at `+0x290` of the TLS block: the depth of nested script runs.
const TLS_SCRIPT_DEPTH: u32 = 0x290;
/// The extra data type `005d8a90` removes.
const EXTRA_TYPE_73: u32 = 0x73;

/// Virtual slot `0x100` of a reference: a test returning `AL`.
const REFERENCE_TEST_SLOT: u32 = 0x100;
/// Virtual slot `0x310` of the process (`0`).
const PROCESS_SLOT_310: u32 = 0x310;
/// Virtual slot `0xc4` of a reference (`1`).
const REFERENCE_SLOT_C4: u32 = 0xc4;
/// Virtual slot `0xe4` of a form: a test returning `AL`.
const FORM_TEST_SLOT: u32 = 0xe4;
/// Virtual slot `0x130` of the running script: its name.
const SCRIPT_NAME_SLOT: u32 = 0x130;

// ---- String literals (addresses in the exe's data) ---------------------------

/// `"Script error: quest %s (%08X) does not exist."`
const MSG_QUEST_MISSING: u32 = 0x0103_ce84;
/// `"Script error: objective %d does not exist in quest %s (%08X)"`
const MSG_OBJECTIVE_MISSING: u32 = 0x0103_ceb4;
/// `"GetObjectiveCompleted >> %0.2f"`
const MSG_GET_OBJECTIVE_COMPLETED: u32 = 0x0103_6458;
/// `"GetObjectiveDisplayed >> %0.2f"`
const MSG_GET_OBJECTIVE_DISPLAYED: u32 = 0x0103_6478;
/// `"enable"`
const TEXT_ENABLE: u32 = 0x0103_cf04;
/// `"on"`
const TEXT_ON: u32 = 0x0101_22c8;
/// `"disable"`
const TEXT_DISABLE: u32 = 0x0103_cefc;
/// `"off"`
const TEXT_OFF: u32 = 0x0103_94c8;
/// `"tune"`
const TEXT_TUNE: u32 = 0x0103_cef4;
/// `"GetBroadcastState >> %0.2f"`
const MSG_GET_BROADCAST_STATE: u32 = 0x0103_cf0c;
/// `"VATS Lighting Off"`
const MSG_VATS_LIGHTING_OFF: u32 = 0x0103_cf28;
/// `"VATS Lighting On"`
const MSG_VATS_LIGHTING_ON: u32 = 0x0103_cf3c;
/// `"SCRIPTS: AddItem in script '%s' failed to generate an item."`
const MSG_ADD_ITEM_FAILED: u32 = 0x0103_9358;

// ---- Small helpers -------------------------------------------------------------

/// `Script::ParseParameters` with the given output addresses after the seven
/// fixed words: its `AL`.
fn parse(e: &mut Engine, a: ScriptArgs, outs: &[u32]) -> bool {
    let mut words = args![
        a.param_info,
        a.script_data,
        a.opcode_offset,
        a.this_obj,
        a.containing_obj,
        a.script_obj,
        a.event_list
    ];
    words.extend_from_slice(outs);
    e.call(PARSE_PARAMETERS, &words).bool()
}

/// [`parse`] with `N` word-sized locals (the stack slots the game passes by
/// address) initialised to `init`. `None` when the parameters do not parse,
/// otherwise the values left in the locals.
fn parse_params<const N: usize>(e: &mut Engine, a: ScriptArgs, init: [u32; N]) -> Option<[u32; N]> {
    let block = e.mem.alloc(4 * N as u32);
    let mut outs = [0u32; N];
    for (i, value) in init.iter().enumerate() {
        outs[i] = block + 4 * i as u32;
        e.mem.set_u32(outs[i], *value);
    }
    let ok = parse(e, a, &outs);
    let mut values = init;
    for (i, value) in values.iter_mut().enumerate() {
        *value = e.mem.u32(outs[i]);
    }
    e.mem.free(block);
    ok.then_some(values)
}

fn console_print(e: &mut Engine, words: &[u32]) {
    e.call(CONSOLE_PRINT, words);
}

/// Whether the TLS echo flag (commands print their result) is set.
fn echo_enabled(e: &mut Engine) -> bool {
    let tls = e.tls();
    e.mem.u8(tls + TLS_ECHO) != 0
}

/// Stores 1.0 or 0.0 into the result double of a function command, then
/// echoes `"<name> >> %0.2f"` when the TLS echo flag is set.
fn store_flag_result(e: &mut Engine, result: Ptr, format: u32, value: bool) {
    e.mem.set_f64(result.addr(), if value { 1.0 } else { 0.0 });
    if echo_enabled(e) {
        let shown = e.mem.f64(result.addr());
        console_print(e, &args![format, shown]);
    }
}

/// Calls the condition function `function` with the command's `thisObj`,
/// `argument`, 0 and the result double: its `AL`.
fn call_condition(e: &mut Engine, function: u32, a: ScriptArgs, argument: u32) -> bool {
    e.call(function, &args![a.this_obj, argument, 0u32, a.result])
        .bool()
}

/// The player singleton (`[0x011dea3c]`).
fn player(e: &mut Engine) -> u32 {
    e.global::<u32>(PLAYER)
}

// ---- Translated functions --------------------------------------------------------

// Translated from 005d7d30 (decompiled, FalloutNV.exe 1.4.0.525)
/// A quest objective command (parameters: quest, objective index, flag).
/// With a missing objective it reports through the logging stub. Otherwise,
/// unless the quest has bit 1 of its flags, a zero flag calls the objective
/// setter (`005ec5d0`) with 0 and a non-zero flag calls it with 1 when bit 0
/// of the objective's word is not yet set.
///
/// A null quest takes the first error branch, which asks the null quest for
/// its form id and name (virtual slot `0x138`), exactly as the game does.
pub fn fn_005d7d30(e: &mut Engine, a: ScriptArgs) -> bool {
    let Some([quest, objective, flag]) = parse_params(e, a, [0, 0, 0]) else {
        return false;
    };
    if quest == 0 {
        let form_id = e.call(GET_FORM_ID, &args![quest]).u32();
        let name = e.vcall(quest, QUEST_NAME_SLOT, &args![]).u32();
        e.call(LOG_STUB, &args![MSG_QUEST_MISSING, name, form_id]);
    } else {
        let found = e.call(QUEST_FIND_OBJECTIVE, &args![quest, objective]).u32();
        if found == 0 {
            let form_id = e.call(GET_FORM_ID, &args![quest]).u32();
            let name = e.vcall(quest, QUEST_NAME_SLOT, &args![]).u32();
            e.call(
                LOG_STUB,
                &args![MSG_OBJECTIVE_MISSING, objective, name, form_id],
            );
        } else if !e.call(QUEST_FLAG_2, &args![quest]).bool() {
            if flag == 0 {
                e.call(OBJECTIVE_SET, &args![found, 0u32]);
            } else if !e.call(OBJECTIVE_BIT_0, &args![found]).bool() {
                e.call(OBJECTIVE_SET, &args![found, 1u32]);
            }
        }
    }
    true
}

/// The two objective getters: parse (quest, objective index, a third value
/// that is not used), find the objective and store its test as 1.0 / 0.0.
fn objective_getter(e: &mut Engine, a: ScriptArgs, test: u32, format: u32) -> bool {
    let Some([quest, objective, _unused]) = parse_params(e, a, [0, 0, 0]) else {
        return false;
    };
    if quest != 0 {
        let found = e.call(QUEST_FIND_OBJECTIVE, &args![quest, objective]).u32();
        if found != 0 {
            let value = e.call(test, &args![found]).bool();
            store_flag_result(e, a.result, format, value);
        }
    }
    true
}

// Translated from 005d7e30 (decompiled, FalloutNV.exe 1.4.0.525)
/// The `GetObjectiveCompleted` body: with a quest and an existing objective
/// the result is 1.0 when `1 < *(objective + 0x20)`, else 0.0 (echoed as
/// `"GetObjectiveCompleted >> %0.2f"`). Always succeeds once the parameters
/// parse.
pub fn fn_005d7e30(e: &mut Engine, a: ScriptArgs) -> bool {
    objective_getter(e, a, OBJECTIVE_ABOVE_ONE, MSG_GET_OBJECTIVE_COMPLETED)
}

// Translated from 005d7ef0 (decompiled, FalloutNV.exe 1.4.0.525)
/// The `GetObjectiveDisplayed` body: like [`fn_005d7e30`] with the test
/// `(*(objective + 0x20) & 1) != 0`.
pub fn fn_005d7ef0(e: &mut Engine, a: ScriptArgs) -> bool {
    objective_getter(e, a, OBJECTIVE_BIT_0, MSG_GET_OBJECTIVE_DISPLAYED)
}

// Translated from 005d7fb0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Script::PipboyRadio` (Xbox PDB): parses a text and an integer. A text
/// that starts with `'1'`, or equals (ignoring case) `"enable"` or `"on"`,
/// enables the Pip-Boy radio and calls `00832240(value, 1)`; one that starts
/// with `'0'`, or equals `"disable"` or `"off"`, disables it; `"tune"` only
/// calls `00832240(value, 1)`. Succeeds once the parameters parse.
pub fn script_pipboy_radio(e: &mut Engine, a: ScriptArgs) -> bool {
    let text = e.mem.alloc(0x200);
    let value = e.mem.alloc(4);
    e.mem.set_u32(value, 0);
    let ok = parse(e, a, &[text, value]);
    if ok {
        let value = e.mem.u32(value);
        let first = e.mem.i8(text);
        let same = |e: &mut Engine, word: u32| {
            e.call(STRING_COMPARE_NO_CASE, &args![text, word]).i32() == 0
        };
        if first == b'1' as i8 || same(e, TEXT_ENABLE) || same(e, TEXT_ON) {
            e.call(PIPBOY_RADIO_ENABLE, &args![1u32]);
            e.call(RADIO_FN_00832240, &args![value, 1u32]);
        } else if first == b'0' as i8 || same(e, TEXT_DISABLE) || same(e, TEXT_OFF) {
            e.call(PIPBOY_RADIO_ENABLE, &args![0u32]);
        } else if same(e, TEXT_TUNE) {
            e.call(RADIO_FN_00832240, &args![value, 1u32]);
        }
    }
    e.mem.free(text);
    e.mem.free(value);
    ok
}

// Translated from 005d8100 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Script::SetNPCRadio` (Xbox PDB): parses a mode and a value. Fails
/// (returns false) unless `thisObj` exists, its virtual test (slot `0x100`)
/// holds and the value is non-zero; then mode 1 enables the NPC radio of
/// `thisObj` with the value, mode 0 disables it, and the command succeeds.
pub fn script_set_npc_radio(e: &mut Engine, a: ScriptArgs) -> bool {
    let Some([mode, value]) = parse_params(e, a, [0, 0]) else {
        return false;
    };
    if a.this_obj.is_null()
        || !e
            .vcall(a.this_obj.addr(), REFERENCE_TEST_SLOT, &args![])
            .bool()
        || value == 0
    {
        return false;
    }
    if mode == 1 {
        e.call(ENABLE_NPC_RADIO, &args![a.this_obj, value]);
    } else if mode == 0 {
        e.call(DISABLE_NPC_RADIO, &args![a.this_obj]);
    }
    true
}

// Translated from 005d81b0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Script::GetBroadcastState` (Xbox PDB): the result is 1.0 when
/// `00835b90(thisObj)` holds, else 0.0 (echoed as `"GetBroadcastState >>
/// %0.2f"`). Always succeeds.
pub fn script_get_broadcast_state(e: &mut Engine, a: ScriptArgs) -> bool {
    let state = e.call(RADIO_BROADCAST_STATE, &args![a.this_obj]).bool();
    store_flag_result(e, a.result, MSG_GET_BROADCAST_STATE, state);
    true
}

// Translated from 005d8220 (decompiled, FalloutNV.exe 1.4.0.525)
/// Radio command: parses an integer and calls `008359e0(thisObj, value > 0)`
/// (signed). Returns the parse result.
pub fn fn_005d8220(e: &mut Engine, a: ScriptArgs) -> bool {
    let Some([value]) = parse_params(e, a, [0]) else {
        return false;
    };
    e.call(
        RADIO_FN_008359E0,
        &args![a.this_obj, ((value as i32) > 0) as u32],
    );
    true
}

// Translated from 005d8280 (decompiled, FalloutNV.exe 1.4.0.525)
/// Radio command: `FalloutRadio::Update(1)`. Always succeeds.
pub fn fn_005d8280(e: &mut Engine) -> bool {
    e.call(RADIO_UPDATE, &args![1u32]);
    true
}

// Translated from 005d82a0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Script::StartRadioConversation` (Xbox PDB): parses an integer and calls
/// `FalloutRadio::StartRadioConversation(thisObj, value)`. Returns the parse
/// result.
pub fn script_start_radio_conversation(e: &mut Engine, a: ScriptArgs) -> bool {
    let Some([value]) = parse_params(e, a, [0]) else {
        return false;
    };
    e.call(START_RADIO_CONVERSATION, &args![a.this_obj, value]);
    true
}

// Translated from 005d8300 (decompiled, FalloutNV.exe 1.4.0.525)
/// A command with no script parameters: `005a43b0(thisObj, 0, 0, result)`;
/// the `AL` of the callee is the command's.
pub fn fn_005d8300(e: &mut Engine, a: ScriptArgs) -> bool {
    call_condition(e, CONDITION_FN_005A43B0, a, 0)
}

// Translated from 005d8320 (decompiled, FalloutNV.exe 1.4.0.525)
/// Audio command: parses an integer. A non-zero value, when the byte at
/// `+6` of the audio singleton is clear, switches the audio to
/// multi-threaded (`SetMultiThreaded(1)`); a zero value, when it is set,
/// switches it off. Succeeds once the parameter parses.
pub fn fn_005d8320(e: &mut Engine, a: ScriptArgs) -> bool {
    let Some([enable]) = parse_params(e, a, [0]) else {
        return false;
    };
    if enable != 0 {
        let audio = e.call(AUDIO_INSTANCE, &args![]).u32();
        if !e.call(AUDIO_BYTE_6, &args![audio]).bool() {
            let audio = e.call(AUDIO_INSTANCE, &args![]).u32();
            e.call(AUDIO_SET_MULTI_THREADED, &args![audio, 1u32]);
            return true;
        }
    }
    if enable == 0 {
        let audio = e.call(AUDIO_INSTANCE, &args![]).u32();
        if e.call(AUDIO_BYTE_6, &args![audio]).bool() {
            let audio = e.call(AUDIO_INSTANCE, &args![]).u32();
            e.call(AUDIO_SET_MULTI_THREADED, &args![audio, 0u32]);
        }
    }
    true
}

// Translated from 005d83b0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Reference command with no script parameters: `0056a9b0(thisObj, 0)`.
/// Always succeeds.
pub fn fn_005d83b0(e: &mut Engine, a: ScriptArgs) -> bool {
    e.call(REFERENCE_FN_0056A9B0, &args![a.this_obj, 0u32]);
    true
}

// Translated from 005d83d0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Reference command: parses an integer and calls `0056a9b0(thisObj,
/// value)`. Returns the parse result.
pub fn fn_005d83d0(e: &mut Engine, a: ScriptArgs) -> bool {
    let Some([value]) = parse_params(e, a, [0]) else {
        return false;
    };
    e.call(REFERENCE_FN_0056A9B0, &args![a.this_obj, value]);
    true
}

// Translated from 005d8420 (decompiled, FalloutNV.exe 1.4.0.525)
/// Cell command: parses three floats (a point), a text (up to 0x200 bytes,
/// zeroed first) and an integer. When the player has a parent cell, builds
/// the point on the stack and calls `005578e0` on the cell with the point,
/// the text (0 when it is empty) and the table entry [`fn_005d8550`] gives
/// for the integer. Returns the parse result.
pub fn fn_005d8420(e: &mut Engine, a: ScriptArgs) -> bool {
    let text = e.mem.alloc(0x200);
    e.mem.set_u8(text, 0);
    e.call(MEMSET, &args![text + 1, 0u32, 0x1ffu32]);
    let words = e.mem.alloc(16);
    e.mem.set_u32(words + 12, 0);
    let ok = parse(e, a, &[words, words + 4, words + 8, text, words + 12]);
    if ok {
        let (x, y, z) = (e.mem.u32(words), e.mem.u32(words + 4), e.mem.u32(words + 8));
        let index = e.mem.u32(words + 12);
        let player_object = player(e);
        if e.call(PLAYER_GET_PARENT_CELL, &args![player_object]).u32() != 0 {
            e.with_stack(12, |e, point| {
                e.call(POINT3_CONSTRUCT, &args![point, x, y, z]);
                let empty = e.call(STRLEN, &args![text]).u32() == 0;
                let entry = fn_005d8550(e, index);
                let player_object = player(e);
                let cell = e.call(PLAYER_GET_PARENT_CELL, &args![player_object]).u32();
                let shown = if empty { 0 } else { text };
                e.call(CELL_FN_005578E0, &args![cell, point, shown, entry]);
            });
        }
    }
    e.mem.free(text);
    e.mem.free(words);
    ok
}

// Translated from 005d8550 (decompiled, FalloutNV.exe 1.4.0.525)
/// The address of entry `index % 10` of the 16-byte-entry table at
/// `0x01203258`.
pub fn fn_005d8550(_e: &mut Engine, index: u32) -> u32 {
    TABLE_AT_1203258 + (index % 10) * 0x10
}

// Translated from 005d8570 (decompiled, FalloutNV.exe 1.4.0.525)
/// Cell command with no script parameters: when the player has a parent
/// cell, calls `005578f0` on it. Always succeeds.
pub fn fn_005d8570(e: &mut Engine) -> bool {
    let player_object = player(e);
    if e.call(PLAYER_GET_PARENT_CELL, &args![player_object]).u32() != 0 {
        let player_object = player(e);
        let cell = e.call(PLAYER_GET_PARENT_CELL, &args![player_object]).u32();
        e.call(CELL_FN_005578F0, &args![cell]);
    }
    true
}

// Translated from 005d85a0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Script::GetIsObjectTypeFunction` (Xbox PDB): parses one byte-sized
/// argument and returns `GetIsObjectTypeConditionFunction(thisObj, type, 0,
/// result)`.
pub fn script_get_is_object_type_function(e: &mut Engine, a: ScriptArgs) -> bool {
    let Some([object_type]) = parse_params(e, a, [0]) else {
        return false;
    };
    call_condition(e, GET_IS_OBJECT_TYPE_CONDITION, a, object_type & 0xff)
}

// Translated from 005d8600 (decompiled, FalloutNV.exe 1.4.0.525)
/// A command with no script parameters:
/// `GetDialogueEmotionConditionFunction(thisObj, 0, 0, result)`; its `AL` is
/// the command's.
pub fn fn_005d8600(e: &mut Engine, a: ScriptArgs) -> bool {
    call_condition(e, GET_DIALOGUE_EMOTION_CONDITION, a, 0)
}

// Translated from 005d8620 (decompiled, FalloutNV.exe 1.4.0.525)
/// A command with no script parameters:
/// `GetDialogueEmotionValueConditionFunction(thisObj, 0, 0, result)`; its
/// `AL` is the command's.
pub fn fn_005d8620(e: &mut Engine, a: ScriptArgs) -> bool {
    call_condition(e, GET_DIALOGUE_EMOTION_VALUE_CONDITION, a, 0)
}

// Translated from 005d8640 (decompiled, FalloutNV.exe 1.4.0.525)
/// A command with no script parameters:
/// `IsWaterObjectConditionFunction(thisObj, 0, 0, result)`; its `AL` is the
/// command's.
pub fn fn_005d8640(e: &mut Engine, a: ScriptArgs) -> bool {
    call_condition(e, IS_WATER_OBJECT_CONDITION, a, 0)
}

// Translated from 005d8660 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Script::ToggleVATSLight` (Xbox PDB): when [`fn_005d8710`] on the object
/// at `0x011f2250` yields a non-zero value, flips a setting. The current
/// state is the `AL` of `00456610` on the object `004e6540` derives. State
/// set: prints `"VATS Lighting On"`, calls `00450f90(0)` on the derived
/// object, then `00b5d270` on the entry `00450b80(0)` returns, with the
/// value of `005d8710`. State clear: prints `"VATS Lighting Off"` and calls
/// `00450f90(1)`. Always succeeds.
pub fn script_toggle_vats_light(e: &mut Engine) -> bool {
    if fn_005d8710(e, OBJECT_AT_11F2250) == 0 {
        return true;
    }
    let first = fn_005d8710(e, OBJECT_AT_11F2250);
    let derived = e.call(DEREF_AT_F8, &args![first]).u32();
    if e.call(FN_00456610, &args![derived]).bool() {
        console_print(e, &args![MSG_VATS_LIGHTING_ON]);
        let value = fn_005d8710(e, OBJECT_AT_11F2250);
        let derived = e.call(DEREF_AT_F8, &args![value]).u32();
        e.call(FN_00450F90, &args![derived, 0u32]);
        let value = fn_005d8710(e, OBJECT_AT_11F2250);
        let entry = e.call(FN_00450B80, &args![0u32]).u32();
        e.call(FN_00B5D270, &args![entry, value]);
    } else {
        console_print(e, &args![MSG_VATS_LIGHTING_OFF]);
        let value = fn_005d8710(e, OBJECT_AT_11F2250);
        let derived = e.call(DEREF_AT_F8, &args![value]).u32();
        e.call(FN_00450F90, &args![derived, 1u32]);
    }
    true
}

// Translated from 005d8710 (decompiled, FalloutNV.exe 1.4.0.525)
/// `thiscall`: what `00559450` returns for the address `this + 0x34` (the
/// word stored there).
pub fn fn_005d8710(e: &mut Engine, this: u32) -> u32 {
    e.call(DEREF, &args![this + 0x34]).u32()
}

// Translated from 005d8730 (decompiled, FalloutNV.exe 1.4.0.525)
/// Parses an integer and calls `004e0330(value, 0.0f, 0)`. Returns the parse
/// result.
pub fn fn_005d8730(e: &mut Engine, a: ScriptArgs) -> bool {
    let Some([value]) = parse_params(e, a, [0]) else {
        return false;
    };
    e.call(FN_004E0330, &args![value, 0.0f32, 0u32]);
    true
}

// Translated from 005d8790 (decompiled, FalloutNV.exe 1.4.0.525)
/// Calls `004e0870()`. Always succeeds.
pub fn fn_005d8790(e: &mut Engine) -> bool {
    e.call(FN_004E0870, &args![]);
    true
}

// Translated from 005d87a0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Parses an integer and three floats and calls `004de8e0(integer, first
/// float, 0, second float, third float)`. Returns the parse result.
pub fn fn_005d87a0(e: &mut Engine, a: ScriptArgs) -> bool {
    let Some([integer, first, second, third]) = parse_params(e, a, [0, 0, 0, 0]) else {
        return false;
    };
    e.call(FN_004DE8E0, &args![integer, first, 0u32, second, third]);
    true
}

// Translated from 005d8830 (decompiled, FalloutNV.exe 1.4.0.525)
/// Parses a float and a byte flag and calls `004def00(float, flag != 0)`.
/// Returns the parse result.
pub fn fn_005d8830(e: &mut Engine, a: ScriptArgs) -> bool {
    let Some([value, flag]) = parse_params(e, a, [0, 0]) else {
        return false;
    };
    e.call(FN_004DEF00, &args![value, ((flag & 0xff) != 0) as u32]);
    true
}

// Translated from 005d88a0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Parses a text (0x100 bytes) and a byte and calls `004deed0(text, byte)`.
/// Returns the parse result.
pub fn fn_005d88a0(e: &mut Engine, a: ScriptArgs) -> bool {
    let text = e.mem.alloc(0x100);
    let byte = e.mem.alloc(4);
    e.mem.set_u32(byte, 0);
    let ok = parse(e, a, &[text, byte]);
    if ok {
        let byte_value = e.mem.u32(byte) & 0xff;
        e.call(FN_004DEED0, &args![text, byte_value]);
    }
    e.mem.free(text);
    e.mem.free(byte);
    ok
}

// Translated from 005d8920 (decompiled, FalloutNV.exe 1.4.0.525)
/// Calls `004defb0()`. Always succeeds.
pub fn fn_005d8920(e: &mut Engine) -> bool {
    e.call(FN_004DEFB0, &args![]);
    true
}

// Translated from 005d8930 (decompiled, FalloutNV.exe 1.4.0.525)
/// A debug command with no script parameters that resets the dialogue state:
/// hands a functor (built by [`fn_005d8af0`], whose slot 0 is
/// [`fn_005d8a90`]) to the process lists, resets the player's greet flag,
/// then walks the list at `[0x011c3f2c]`: for every non-null topic entry it
/// walks the topic's own list (`topic + 0x2c`), and for every non-null item
/// of that list calls `0061f280` on each non-null element of the item's
/// array (`00717e50`/`00877a30`). Ends with
/// `DialogMenu::DebugResetSaidOnceFlags`. Always succeeds.
///
/// The game also looks up two topics (`GetTopic(6, 0xc)` and `GetTopic(1,
/// 0)`) and sets two local flags when an entry equals them; the flags are
/// never read, so only the two lookups are kept.
pub fn fn_005d8930(e: &mut Engine) -> bool {
    e.with_stack(8, |e, functor| {
        fn_005d8af0(e, functor);
        e.call(PROCESS_LISTS_VISIT, &args![PROCESS_LISTS, functor]);
    });
    let player_object = player(e);
    e.call(RESET_PLAYER_GREET_FLAG, &args![player_object]);
    e.call(GET_TOPIC, &args![6u32, 0xcu32]);
    e.call(GET_TOPIC, &args![1u32, 0u32]);
    let owner = e.global::<u32>(LIST_OWNER);
    let mut node = e.call(LIST_FIRST_NODE, &args![owner]).u32();
    while node != 0 {
        let data = e.call(LIST_NODE_DATA, &args![node]).u32();
        let topic = e.mem.u32(data);
        if topic != 0 {
            let mut inner = fn_005d8a70(e, topic);
            while inner != 0 {
                let data = e.call(LIST_NODE_DATA, &args![inner]).u32();
                let item = e.mem.u32(data);
                if item != 0 {
                    let array = e.call(ARRAY_AT_4, &args![item]).u32();
                    let count = e.call(GET_FORM_ID, &args![array]).u32();
                    for index in 0..count {
                        let slot = e.call(ARRAY_ELEMENT_ADDRESS, &args![array, index]).u32();
                        let element = e.mem.u32(slot);
                        if element != 0 {
                            e.call(ELEMENT_FN_0061F280, &args![element]);
                        }
                    }
                }
                inner = e.call(LIST_NEXT_NODE, &args![inner]).u32();
            }
        }
        node = e.call(LIST_NEXT_NODE, &args![node]).u32();
    }
    e.call(DEBUG_RESET_SAID_ONCE_FLAGS, &args![]);
    true
}

// Translated from 005d8a70 (decompiled, FalloutNV.exe 1.4.0.525)
/// `thiscall`: `this + 0x2c`, the list embedded in a topic.
pub fn fn_005d8a70(_e: &mut Engine, this: u32) -> u32 {
    this + 0x2c
}

// Translated from 005d8a90 (decompiled, FalloutNV.exe 1.4.0.525)
/// Slot 0 of the functor's vtable (`thiscall`, one argument, `RET 4`; the
/// functor itself is never read): when the actor's virtual test (slot
/// `0x100`) holds and it has a process, calls the process's slot `0x310`
/// with 0; then removes extra data type `0x73` from the actor's extra data
/// list.
pub fn fn_005d8a90(e: &mut Engine, _unused_this: Ptr, actor: Ptr) {
    if e.vcall(actor.addr(), REFERENCE_TEST_SLOT, &args![]).bool() {
        let process = e.call(GET_PROCESS, &args![actor]).u32();
        if process != 0 {
            e.vcall(process, PROCESS_SLOT_310, &args![0u32]);
        }
    }
    let list = e.call(EXTRA_DATA_LIST, &args![actor]).u32();
    e.call(REMOVE_EXTRA, &args![list, EXTRA_TYPE_73]);
}

// Translated from 005d8af0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Constructor of the 8-byte functor: the base constructor
/// ([`fn_005d8b10`]), then the vtable at `0x0103cf54`. Returns `this`.
pub fn fn_005d8af0(e: &mut Engine, this: Ptr) -> Ptr {
    fn_005d8b10(e, this);
    e.mem.set_u32(this.addr(), FUNCTOR_VTABLE);
    this
}

// Translated from 005d8b10 (decompiled, FalloutNV.exe 1.4.0.525)
/// Base constructor of the functor: vtable `0x0103cf5c` (a pure virtual
/// base), word at `+4` cleared. Returns `this`.
pub fn fn_005d8b10(e: &mut Engine, this: Ptr) -> Ptr {
    e.mem.set_u32(this.addr(), FUNCTOR_BASE_VTABLE);
    e.mem.set_u32(this.addr() + 4, 0);
    this
}

// Translated from 005d8b40 (decompiled, FalloutNV.exe 1.4.0.525)
/// Calls `00966d70` on the player. Always succeeds.
pub fn fn_005d8b40(e: &mut Engine) -> bool {
    let player_object = player(e);
    e.call(PLAYER_FN_00966D70, &args![player_object]);
    true
}

// Translated from 005d8b60 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Script::ExitGame` (Xbox PDB): looks up form `0x000ed568`, casts it from
/// `TESForm` to `BGSPerk`, asks the player (virtual slot `0x4a0`) about that
/// perk, stores 2.0 (perk held) or 1.0 in the `TESGlobal` at `[0x011c3f3c]`,
/// closes the console and goes to the main menu. Always succeeds.
pub fn script_exit_game(e: &mut Engine) -> bool {
    let form = e.call(LOOKUP_FORM, &args![EXIT_GAME_FORM_ID]).u32();
    let perk = e
        .call(
            DYNAMIC_CAST,
            &args![form, 0u32, RTTI_TES_FORM, RTTI_BGS_PERK, 0u32],
        )
        .u32();
    let player_object = player(e);
    let held = e
        .vcall(player_object, PLAYER_HAS_PERK_SLOT, &args![perk, 0u32])
        .bool();
    let global = e.global::<u32>(EXIT_GAME_GLOBAL);
    let value = if held { 2.0f32 } else { 1.0f32 };
    e.call(TES_GLOBAL_SET_VALUE, &args![global, value]);
    e.call(CLOSE_CONSOLE, &args![]);
    e.call(CHOOSE_MAIN_MENU, &args![]);
    true
}

// Translated from 005d8bf0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Parses a text (0x200 bytes), builds an 8-byte `BSStringT` and calls
/// `005b4960(the eight words, text, string)`; its `AL` is the command's. The
/// string is destroyed afterwards. The exception-unwinding frame is not
/// translated.
pub fn fn_005d8bf0(e: &mut Engine, a: ScriptArgs) -> bool {
    let text = e.mem.alloc(0x200);
    if !parse(e, a, &[text]) {
        e.mem.free(text);
        return false;
    }
    let result = e.with_stack(8, |e, string| {
        e.call(STRING_CONSTRUCT, &args![string]);
        let result = e.call(SCRIPT_BODY_005B4960, &args![a, text, string]).bool();
        e.call(STRING_DESTRUCT, &args![string]);
        result
    });
    e.mem.free(text);
    result
}

// Translated from 005d8cf0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Script::TriggerEnterFunction` (Xbox PDB): parses a reference. With a
/// `thisObj`: initialises its action list (`InitActionList(thisObj,
/// extra data list)`), sets action flags `0x20000000` and `0x10000000` for
/// the parsed reference, sets it as the action reference of `thisObj`
/// (`00572e10`) and, while the TLS script depth (`+0x290`) is below 5, runs
/// `thisObj`'s script with the depth raised by one. Returns the parse
/// result.
pub fn script_trigger_enter_function(e: &mut Engine, a: ScriptArgs) -> bool {
    let Some([target]) = parse_params(e, a, [0]) else {
        return false;
    };
    if !a.this_obj.is_null() {
        let list = e.call(EXTRA_DATA_LIST, &args![a.this_obj]).u32();
        e.call(INIT_ACTION_LIST, &args![a.this_obj, list]);
        let list = e.call(EXTRA_DATA_LIST, &args![a.this_obj]).u32();
        e.call(SET_ACTION_FLAG, &args![target, list, 0x2000_0000u32]);
        let list = e.call(EXTRA_DATA_LIST, &args![a.this_obj]).u32();
        e.call(SET_ACTION_FLAG, &args![target, list, 0x1000_0000u32]);
        e.call(SET_ACTION_REF, &args![a.this_obj, target]);
        let tls = e.tls();
        if e.mem.i32(tls + TLS_SCRIPT_DEPTH) < 5 {
            let depth = e.mem.i32(tls + TLS_SCRIPT_DEPTH);
            e.mem.set_i32(tls + TLS_SCRIPT_DEPTH, depth + 1);
            e.call(RUN_SCRIPT, &args![a.this_obj]);
            let depth = e.mem.i32(tls + TLS_SCRIPT_DEPTH);
            e.mem.set_i32(tls + TLS_SCRIPT_DEPTH, depth - 1);
        }
    }
    true
}

// Translated from 005d8e20 (decompiled, FalloutNV.exe 1.4.0.525)
/// A reference command with no script parameters: for a `thisObj` that is
/// not flagged by `00440d80` and not the player, calls its virtual slot
/// `0xc4` with 1 and, when `TESForm::GetFile(0)` gives no file, adds it (by
/// address of a stack word) to the list at `0x011cacb8` (`005ae3d0`).
/// Always succeeds.
pub fn fn_005d8e20(e: &mut Engine, a: ScriptArgs) -> bool {
    let object = a.this_obj;
    if !object.is_null()
        && !e.call(FORM_IS_DELETED_FLAG, &args![object]).bool()
        && object.addr() != player(e)
    {
        e.vcall(object.addr(), REFERENCE_SLOT_C4, &args![1u32]);
        if e.call(GET_FILE, &args![object, 0u32]).u32() == 0 {
            e.with_stack(4, |e, cell| {
                e.mem.set_u32(cell.addr(), object.addr());
                e.call(LIST_ADD_REFERENCE, &args![REFERENCE_LIST, cell]);
            });
        }
    }
    true
}

// Translated from 005d8e80 (decompiled, FalloutNV.exe 1.4.0.525)
/// The `AddItem`-style body (its failure message is `"SCRIPTS: AddItem in
/// script '%s' failed to generate an item."`): parses an item form, a count,
/// a float and an integer. With a `thisObj`, builds a `TESContainer` (12
/// bytes) on the stack and fills it from the item, by form type: type `0x34`
/// adds the leveled items of `item + 0x30` for the reference's calculated
/// level; type `0x55` adds every entry of its list that passes the virtual
/// test `0xe4` (needs a non-zero count); any other item that passes the test
/// is added directly. When nothing applies the logging stub gets the message
/// with the script's name. The container then gets the float
/// (`00482090`) and is applied to `thisObj` (`004821a0(thisObj, integer ==
/// 1)`), then destroyed. Returns the parse result. The exception-unwinding
/// frame is not translated.
pub fn fn_005d8e80(e: &mut Engine, a: ScriptArgs) -> bool {
    let Some([item, count, quality, flag]) = parse_params(e, a, [0, 0, 0, 0]) else {
        return false;
    };
    if a.this_obj.is_null() {
        return true;
    }
    let level = e.call(GET_CALC_LEVEL, &args![a.this_obj, 0u32]).u32();
    e.with_stack(12, |e, container| {
        e.call(CONTAINER_CONSTRUCT, &args![container]);
        let mut leveled = 0;
        let mut list_form = 0;
        let mut single = 0;
        let form_type = e.call(FORM_TYPE, &args![item]).u32();
        if form_type == 0x34 {
            leveled = item;
        } else if form_type == 0x55 {
            list_form = item;
        } else if e.vcall(item, FORM_TEST_SLOT, &args![]).bool() {
            single = item;
        }
        if leveled != 0 {
            e.call(
                FORM_ADD_LEVELED_ITEMS,
                &args![
                    leveled + 0x30,
                    level & 0xffff,
                    count & 0xffff,
                    container,
                    0u32
                ],
            );
        } else if list_form != 0 && count != 0 {
            let mut node = e.call(LIST_AT_18, &args![list_form]).u32();
            while node != 0 && !e.call(NODE_IS_EMPTY, &args![node]).bool() {
                let data = e.call(LIST_NODE_DATA, &args![node]).u32();
                let entry = e.mem.u32(data);
                node = e.call(LIST_NEXT_NODE, &args![node]).u32();
                if e.vcall(entry, FORM_TEST_SLOT, &args![]).bool() {
                    e.call(CONTAINER_ADD_ITEM, &args![container, entry, count, 0u32]);
                }
            }
        } else if single != 0 && count != 0 {
            e.call(CONTAINER_ADD_ITEM, &args![container, single, count, 0u32]);
        } else {
            let name = e
                .vcall(a.script_obj.addr(), SCRIPT_NAME_SLOT, &args![])
                .u32();
            e.call(LOG_STUB, &args![MSG_ADD_ITEM_FAILED, name]);
        }
        e.call(CONTAINER_FN_00482090, &args![container, quality]);
        e.call(
            CONTAINER_FN_004821A0,
            &args![container, a.this_obj, (flag == 1) as u32],
        );
        e.call(CONTAINER_DESTRUCT, &args![container]);
    });
    true
}

// Translated from 005d90a0 (decompiled, FalloutNV.exe 1.4.0.525)
/// A function command: the result double starts at 0.0; parses a form, a
/// float (default 1.0), an integer (default 1) and two more integers, then
/// calls `005c4b30(thisObj, form, integer, integer2, integer3, float)`. When
/// that returns a form, its form id is stored into the result double
/// (`PutNumericIDInDouble`). Returns the parse result.
pub fn fn_005d90a0(e: &mut Engine, a: ScriptArgs) -> bool {
    e.mem.set_f64(a.result.addr(), 0.0);
    let one = 1.0f32.to_bits();
    let Some([form, scale, first, second, third]) = parse_params(e, a, [0, one, 1, 0, 0]) else {
        return false;
    };
    let made = e
        .call(
            SCRIPT_BODY_005C4B30,
            &args![a.this_obj, form, first, second, third, scale],
        )
        .u32();
    if made != 0 {
        let id = e.call(GET_FORM_ID, &args![made]).u32();
        e.with_stack(4, |e, cell| {
            e.mem.set_u32(cell.addr(), id);
            e.call(PUT_NUMERIC_ID_IN_DOUBLE, &args![cell, a.result]);
        });
    }
    true
}

/// This part's translated functions, by exe address.
pub fn funcs() -> Vec<(u32, AbiFn)> {
    vec![
        entry!(0x005d7d30, fn_005d7d30(ScriptArgs) -> bool),
        entry!(0x005d7e30, fn_005d7e30(ScriptArgs) -> bool),
        entry!(0x005d7ef0, fn_005d7ef0(ScriptArgs) -> bool),
        entry!(0x005d7fb0, script_pipboy_radio(ScriptArgs) -> bool),
        entry!(0x005d8100, script_set_npc_radio(ScriptArgs) -> bool),
        entry!(0x005d81b0, script_get_broadcast_state(ScriptArgs) -> bool),
        entry!(0x005d8220, fn_005d8220(ScriptArgs) -> bool),
        entry!(0x005d8280, fn_005d8280() -> bool),
        entry!(0x005d82a0, script_start_radio_conversation(ScriptArgs) -> bool),
        entry!(0x005d8300, fn_005d8300(ScriptArgs) -> bool),
        entry!(0x005d8320, fn_005d8320(ScriptArgs) -> bool),
        entry!(0x005d83b0, fn_005d83b0(ScriptArgs) -> bool),
        entry!(0x005d83d0, fn_005d83d0(ScriptArgs) -> bool),
        entry!(0x005d8420, fn_005d8420(ScriptArgs) -> bool),
        entry!(0x005d8550, fn_005d8550(u32) -> u32),
        entry!(0x005d8570, fn_005d8570() -> bool),
        entry!(0x005d85a0, script_get_is_object_type_function(ScriptArgs) -> bool),
        entry!(0x005d8600, fn_005d8600(ScriptArgs) -> bool),
        entry!(0x005d8620, fn_005d8620(ScriptArgs) -> bool),
        entry!(0x005d8640, fn_005d8640(ScriptArgs) -> bool),
        entry!(0x005d8660, script_toggle_vats_light() -> bool),
        entry!(0x005d8710, fn_005d8710(u32) -> u32),
        entry!(0x005d8730, fn_005d8730(ScriptArgs) -> bool),
        entry!(0x005d8790, fn_005d8790() -> bool),
        entry!(0x005d87a0, fn_005d87a0(ScriptArgs) -> bool),
        entry!(0x005d8830, fn_005d8830(ScriptArgs) -> bool),
        entry!(0x005d88a0, fn_005d88a0(ScriptArgs) -> bool),
        entry!(0x005d8920, fn_005d8920() -> bool),
        entry!(0x005d8930, fn_005d8930() -> bool),
        entry!(0x005d8a70, fn_005d8a70(u32) -> u32),
        entry!(0x005d8a90, fn_005d8a90(Ptr, Ptr)),
        entry!(0x005d8af0, fn_005d8af0(Ptr) -> Ptr),
        entry!(0x005d8b10, fn_005d8b10(Ptr) -> Ptr),
        entry!(0x005d8b40, fn_005d8b40() -> bool),
        entry!(0x005d8b60, script_exit_game() -> bool),
        entry!(0x005d8bf0, fn_005d8bf0(ScriptArgs) -> bool),
        entry!(0x005d8cf0, script_trigger_enter_function(ScriptArgs) -> bool),
        entry!(0x005d8e20, fn_005d8e20(ScriptArgs) -> bool),
        entry!(0x005d8e80, fn_005d8e80(ScriptArgs) -> bool),
        entry!(0x005d90a0, fn_005d90a0(ScriptArgs) -> bool),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::{Cell, RefCell};
    use std::rc::Rc;

    // Fake virtual functions the tests put into vtables.
    const V_NAME: u32 = 0x0900_0001;
    const V_TRUE: u32 = 0x0900_0002;
    const V_FALSE: u32 = 0x0900_0003;
    const V_PERK_HELD: u32 = 0x0900_0004;
    const V_PERK_NOT_HELD: u32 = 0x0900_0005;
    const V_PROCESS: u32 = 0x0900_0006;

    /// An engine with the pages of the globals these commands touch mapped
    /// and the accessors every command uses replaced by doubles that behave
    /// like the exe's code (see the constants' documentation).
    fn engine() -> Engine {
        let mut e = Engine::new();
        e.map(0x011c_3000, 0x1000);
        e.map(0x011d_e000, 0x1000);
        e.register(CONSOLE_PRINT, |_, _| Ret::default());
        e.register(LOG_STUB, |_, _| Ret::default());
        e.register(GET_FORM_ID, |e, a| e.mem.u32(a[0] + 0x0c).into_ret());
        e.register(V_NAME, |_, _| 0xbbbb.into_ret());
        e.register(V_TRUE, |_, _| true.into_ret());
        e.register(V_FALSE, |_, _| false.into_ret());
        e.register(V_PERK_HELD, |_, _| true.into_ret());
        e.register(V_PERK_NOT_HELD, |_, _| false.into_ret());
        e.register(V_PROCESS, |_, _| Ret::default());
        e
    }

    /// A zeroed object of 0x800 bytes with a vtable that has the given
    /// `(byte offset, function)` slots.
    fn object_with(e: &mut Engine, slots: &[(u32, u32)]) -> u32 {
        let vtable = e.mem.alloc(0x800);
        for (offset, function) in slots {
            e.mem.set_u32(vtable + offset, *function);
        }
        let object = e.mem.alloc(0x800);
        e.mem.set_u32(object, vtable);
        object
    }

    fn object(e: &mut Engine) -> u32 {
        object_with(e, &[])
    }

    /// The standard eight words with `this_obj` set and a result double.
    fn command(e: &mut Engine, this_obj: u32) -> ScriptArgs {
        let result = e.mem.alloc(8);
        ScriptArgs {
            param_info: 1,
            script_data: 2,
            this_obj: Ptr::new(this_obj),
            containing_obj: Ptr::NULL,
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

    /// `ParseParameters` double for a command whose first output is a text
    /// and whose second is a word.
    fn parse_gives_text(e: &mut Engine, text: &'static str, word: u32) {
        e.register_double(PARSE_PARAMETERS, move |e, a| {
            e.mem.set_cstr(a[7], text.as_bytes());
            e.mem.set_u32(a[8], word);
            true.into_ret()
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

    fn set_echo(e: &mut Engine, on: bool) {
        let tls = e.tls();
        e.mem.set_u8(tls + TLS_ECHO, on as u8);
    }

    fn set_player(e: &mut Engine) -> u32 {
        let existing: u32 = e.global(PLAYER);
        if existing != 0 {
            return existing;
        }
        let player_object = object(e);
        e.set_global(PLAYER, player_object);
        player_object
    }

    /// `ParseParameters` gets: info, data, opcode offset, thisObj,
    /// containing, script, event list, then the address of the local.
    fn assert_parsed(e: &Engine, this_obj: u32) {
        assert_eq!(
            calls(e, PARSE_PARAMETERS)[0][..7],
            [1, 2, 8, this_obj, 0, 5, 6]
        );
    }

    fn run(e: &mut Engine, address: u32, a: ScriptArgs) -> bool {
        e.call(address, &args![a]).bool()
    }

    // ---- 005d7d30 and the objective getters ----

    #[test]
    fn fn_005d7d30_sets_the_objective_flag_or_reports_a_missing_objective() {
        let mut e = engine();
        let quest = object_with(&mut e, &[(QUEST_NAME_SLOT, V_NAME)]);
        e.mem.set_u32(quest + 0x0c, 0x1234);
        let a = command(&mut e, 0);
        e.register(OBJECTIVE_SET, |_, _| Ret::default());
        e.register(QUEST_FLAG_2, |_, _| false.into_ret());
        e.register(OBJECTIVE_BIT_0, |_, _| false.into_ret());

        // Missing objective: the message gets the index, name and id.
        e.register(QUEST_FIND_OBJECTIVE, |_, _| 0u32.into_ret());
        parse_gives(&mut e, true, &[quest, 3, 1]);
        start_log(&mut e);
        assert!(run(&mut e, 0x005d_7d30, a));
        assert_parsed(&e, 0);
        assert_eq!(
            calls(&e, LOG_STUB),
            vec![vec![MSG_OBJECTIVE_MISSING, 3, 0xbbbb, 0x1234]]
        );
        assert_eq!(calls(&e, QUEST_FIND_OBJECTIVE), vec![vec![quest, 3]]);
        assert!(calls(&e, OBJECTIVE_SET).is_empty());

        // Found, flag 1, bit 0 clear: the setter gets 1.
        e.register(QUEST_FIND_OBJECTIVE, |_, _| 0x7000u32.into_ret());
        start_log(&mut e);
        assert!(run(&mut e, 0x005d_7d30, a));
        assert_eq!(calls(&e, OBJECTIVE_SET), vec![vec![0x7000, 1]]);
        assert!(calls(&e, LOG_STUB).is_empty());

        // Flag 1 with bit 0 already set: nothing.
        e.register(OBJECTIVE_BIT_0, |_, _| true.into_ret());
        start_log(&mut e);
        assert!(run(&mut e, 0x005d_7d30, a));
        assert!(calls(&e, OBJECTIVE_SET).is_empty());

        // Flag 0: the setter gets 0 whatever bit 0 says.
        parse_gives(&mut e, true, &[quest, 3, 0]);
        start_log(&mut e);
        assert!(run(&mut e, 0x005d_7d30, a));
        assert_eq!(calls(&e, OBJECTIVE_SET), vec![vec![0x7000, 0]]);

        // The quest flag set: nothing at all.
        e.register(QUEST_FLAG_2, |_, _| true.into_ret());
        start_log(&mut e);
        assert!(run(&mut e, 0x005d_7d30, a));
        assert!(calls(&e, OBJECTIVE_SET).is_empty());

        // Parameters that do not parse.
        parse_gives(&mut e, false, &[]);
        start_log(&mut e);
        assert!(!run(&mut e, 0x005d_7d30, a));
        assert!(calls(&e, QUEST_FIND_OBJECTIVE).is_empty());
    }

    #[test]
    fn fn_005d7d30_asks_a_null_quest_for_its_name_like_the_game() {
        let mut e = engine();
        // The game reads the vtable at address 0; the page is mapped here.
        e.map(0, 0x1000);
        let vtable = e.mem.alloc(0x800);
        e.mem.set_u32(vtable + QUEST_NAME_SLOT, V_NAME);
        e.mem.set_u32(0, vtable);
        let a = command(&mut e, 0);
        parse_gives(&mut e, true, &[0, 3, 1]);
        start_log(&mut e);
        assert!(run(&mut e, 0x005d_7d30, a));
        assert_eq!(
            calls(&e, LOG_STUB),
            vec![vec![MSG_QUEST_MISSING, 0xbbbb, 0]]
        );
        assert!(calls(&e, QUEST_FIND_OBJECTIVE).is_empty());
    }

    /// The test of an objective getter: `address` is the command, `test` the
    /// objective test it calls.
    fn check_objective_getter(address: u32, test: u32, format: u32) {
        let mut e = engine();
        let a = command(&mut e, 0);
        e.mem.set_f64(a.result.addr(), 7.0);
        e.register(QUEST_FIND_OBJECTIVE, |_, _| 0x7000u32.into_ret());
        e.register(test, |_, _| true.into_ret());

        // No quest: the result stays as it was.
        parse_gives(&mut e, true, &[0, 1, 0]);
        start_log(&mut e);
        assert!(run(&mut e, address, a));
        assert_eq!(e.mem.f64(a.result.addr()), 7.0);
        assert!(calls(&e, QUEST_FIND_OBJECTIVE).is_empty());

        // Objective missing: the result stays as it was.
        parse_gives(&mut e, true, &[0x5000, 2, 0]);
        e.register(QUEST_FIND_OBJECTIVE, |_, _| 0u32.into_ret());
        assert!(run(&mut e, address, a));
        assert_eq!(e.mem.f64(a.result.addr()), 7.0);

        // Found: the test decides, and the echo prints the result.
        e.register(QUEST_FIND_OBJECTIVE, |_, _| 0x7000u32.into_ret());
        start_log(&mut e);
        assert!(run(&mut e, address, a));
        assert_eq!(e.mem.f64(a.result.addr()), 1.0);
        assert_eq!(calls(&e, QUEST_FIND_OBJECTIVE), vec![vec![0x5000, 2]]);
        assert_eq!(calls(&e, test), vec![vec![0x7000]]);
        assert!(calls(&e, CONSOLE_PRINT).is_empty());
        set_echo(&mut e, true);
        e.register(test, |_, _| false.into_ret());
        start_log(&mut e);
        assert!(run(&mut e, address, a));
        assert_eq!(e.mem.f64(a.result.addr()), 0.0);
        assert_eq!(calls(&e, CONSOLE_PRINT), vec![args![format, 0.0f64]]);

        parse_gives(&mut e, false, &[]);
        assert!(!run(&mut e, address, a));
    }

    #[test]
    fn fn_005d7e30_reports_whether_the_objective_is_completed() {
        check_objective_getter(
            0x005d_7e30,
            OBJECTIVE_ABOVE_ONE,
            MSG_GET_OBJECTIVE_COMPLETED,
        );
    }

    #[test]
    fn fn_005d7ef0_reports_whether_the_objective_is_displayed() {
        check_objective_getter(0x005d_7ef0, OBJECTIVE_BIT_0, MSG_GET_OBJECTIVE_DISPLAYED);
    }

    // ---- Radio commands ----

    #[test]
    fn pipboy_radio_reads_the_word_and_enables_disables_or_tunes() {
        let mut e = engine();
        e.map(0x0101_2000, 0x1000);
        e.map(0x0103_9000, 0x1000);
        e.map(0x0103_c000, 0x1000);
        e.mem.set_cstr(TEXT_ENABLE, b"enable");
        e.mem.set_cstr(TEXT_ON, b"on");
        e.mem.set_cstr(TEXT_DISABLE, b"disable");
        e.mem.set_cstr(TEXT_OFF, b"off");
        e.mem.set_cstr(TEXT_TUNE, b"tune");
        e.register(STRING_COMPARE_NO_CASE, |e, a| {
            let (x, y) = (e.mem.cstr(a[0]), e.mem.cstr(a[1]));
            u32::from(!x.eq_ignore_ascii_case(&y)).into_ret()
        });
        e.register(PIPBOY_RADIO_ENABLE, |_, _| Ret::default());
        e.register(RADIO_FN_00832240, |_, _| Ret::default());
        let a = command(&mut e, 0);

        // (text, expected enable calls, expected tune calls)
        let enable = vec![vec![1]];
        let disable = vec![vec![0]];
        let tune = vec![vec![9, 1]];
        type Calls = Vec<Vec<u32>>;
        let cases: [(&'static str, Calls, Calls); 9] = [
            ("1", enable.clone(), tune.clone()),
            ("Enable", enable.clone(), tune.clone()),
            ("ON", enable, tune.clone()),
            ("0", disable.clone(), vec![]),
            ("DISABLE", disable.clone(), vec![]),
            ("Off", disable, vec![]),
            ("tune", vec![], tune),
            ("unknown", vec![], vec![]),
            ("", vec![], vec![]),
        ];
        for (text, enable, tune) in cases {
            parse_gives_text(&mut e, text, 9);
            start_log(&mut e);
            assert!(run(&mut e, 0x005d_7fb0, a), "{text:?}");
            assert_eq!(calls(&e, PIPBOY_RADIO_ENABLE), enable, "{text:?}");
            assert_eq!(calls(&e, RADIO_FN_00832240), tune, "{text:?}");
        }

        parse_gives(&mut e, false, &[]);
        start_log(&mut e);
        assert!(!run(&mut e, 0x005d_7fb0, a));
        assert!(calls(&e, PIPBOY_RADIO_ENABLE).is_empty());
    }

    #[test]
    fn set_npc_radio_enables_or_disables_the_radio_of_an_actor_with_a_value() {
        let mut e = engine();
        let actor = object_with(&mut e, &[(REFERENCE_TEST_SLOT, V_TRUE)]);
        let a = command(&mut e, actor);
        e.register(ENABLE_NPC_RADIO, |_, _| Ret::default());
        e.register(DISABLE_NPC_RADIO, |_, _| Ret::default());

        parse_gives(&mut e, true, &[1, 0x77]);
        start_log(&mut e);
        assert!(run(&mut e, 0x005d_8100, a));
        assert_parsed(&e, actor);
        assert_eq!(calls(&e, ENABLE_NPC_RADIO), vec![vec![actor, 0x77]]);
        assert!(calls(&e, DISABLE_NPC_RADIO).is_empty());

        parse_gives(&mut e, true, &[0, 0x77]);
        start_log(&mut e);
        assert!(run(&mut e, 0x005d_8100, a));
        assert!(calls(&e, ENABLE_NPC_RADIO).is_empty());
        assert_eq!(calls(&e, DISABLE_NPC_RADIO), vec![vec![actor]]);

        // Another mode: succeeds without touching the radio.
        parse_gives(&mut e, true, &[2, 0x77]);
        start_log(&mut e);
        assert!(run(&mut e, 0x005d_8100, a));
        assert!(calls(&e, ENABLE_NPC_RADIO).is_empty());
        assert!(calls(&e, DISABLE_NPC_RADIO).is_empty());

        // A zero value, a failing parse, no reference or a failing test.
        parse_gives(&mut e, true, &[1, 0]);
        assert!(!run(&mut e, 0x005d_8100, a));
        parse_gives(&mut e, false, &[1, 0x77]);
        assert!(!run(&mut e, 0x005d_8100, a));
        parse_gives(&mut e, true, &[1, 0x77]);
        let none = command(&mut e, 0);
        assert!(!run(&mut e, 0x005d_8100, none));
        let refusing = object_with(&mut e, &[(REFERENCE_TEST_SLOT, V_FALSE)]);
        let b = command(&mut e, refusing);
        start_log(&mut e);
        assert!(!run(&mut e, 0x005d_8100, b));
        assert!(calls(&e, ENABLE_NPC_RADIO).is_empty());
    }

    #[test]
    fn get_broadcast_state_stores_the_radio_state_in_the_result() {
        let mut e = engine();
        let a = command(&mut e, 0x4444);
        e.register(RADIO_BROADCAST_STATE, |_, _| true.into_ret());
        start_log(&mut e);
        assert!(run(&mut e, 0x005d_81b0, a));
        assert_eq!(e.mem.f64(a.result.addr()), 1.0);
        assert_eq!(calls(&e, RADIO_BROADCAST_STATE), vec![vec![0x4444]]);
        assert!(calls(&e, CONSOLE_PRINT).is_empty());

        e.register(RADIO_BROADCAST_STATE, |_, _| false.into_ret());
        set_echo(&mut e, true);
        start_log(&mut e);
        assert!(run(&mut e, 0x005d_81b0, a));
        assert_eq!(e.mem.f64(a.result.addr()), 0.0);
        assert_eq!(
            calls(&e, CONSOLE_PRINT),
            vec![args![MSG_GET_BROADCAST_STATE, 0.0f64]]
        );
    }

    #[test]
    fn fn_005d8220_passes_whether_the_value_is_positive() {
        let mut e = engine();
        let a = command(&mut e, 0x4444);
        e.register(RADIO_FN_008359E0, |_, _| Ret::default());
        for (value, flag) in [(5u32, 1u32), (0, 0), (-3i32 as u32, 0)] {
            parse_gives(&mut e, true, &[value]);
            start_log(&mut e);
            assert!(run(&mut e, 0x005d_8220, a));
            assert_parsed(&e, 0x4444);
            assert_eq!(calls(&e, RADIO_FN_008359E0), vec![vec![0x4444, flag]]);
        }
        parse_gives(&mut e, false, &[]);
        start_log(&mut e);
        assert!(!run(&mut e, 0x005d_8220, a));
        assert!(calls(&e, RADIO_FN_008359E0).is_empty());
    }

    #[test]
    fn fn_005d8280_updates_the_radio() {
        let mut e = engine();
        e.register(RADIO_UPDATE, |_, _| Ret::default());
        start_log(&mut e);
        assert!(e.call(0x005d_8280, &args![]).bool());
        assert_eq!(calls(&e, RADIO_UPDATE), vec![vec![1]]);
    }

    #[test]
    fn start_radio_conversation_passes_the_parsed_value() {
        let mut e = engine();
        let a = command(&mut e, 0x4444);
        e.register(START_RADIO_CONVERSATION, |_, _| Ret::default());
        parse_gives(&mut e, true, &[0x66]);
        start_log(&mut e);
        assert!(run(&mut e, 0x005d_82a0, a));
        assert_eq!(
            calls(&e, START_RADIO_CONVERSATION),
            vec![vec![0x4444, 0x66]]
        );
        parse_gives(&mut e, false, &[]);
        start_log(&mut e);
        assert!(!run(&mut e, 0x005d_82a0, a));
        assert!(calls(&e, START_RADIO_CONVERSATION).is_empty());
    }

    /// The test of a "call a condition function with (thisObj, 0, 0,
    /// result)" command.
    fn check_condition_only(address: u32, condition: u32) {
        let mut e = engine();
        let a = command(&mut e, 0x4444);
        e.register(condition, |_, _| false.into_ret());
        start_log(&mut e);
        assert!(!run(&mut e, address, a));
        assert_eq!(
            calls(&e, condition),
            vec![vec![0x4444, 0, 0, a.result.addr()]]
        );
        e.register(condition, |_, _| true.into_ret());
        assert!(run(&mut e, address, a));
    }

    #[test]
    fn fn_005d8300_returns_the_condition_function_s_result() {
        check_condition_only(0x005d_8300, CONDITION_FN_005A43B0);
    }

    #[test]
    fn fn_005d8600_returns_the_condition_function_s_result() {
        check_condition_only(0x005d_8600, GET_DIALOGUE_EMOTION_CONDITION);
    }

    #[test]
    fn fn_005d8620_returns_the_condition_function_s_result() {
        check_condition_only(0x005d_8620, GET_DIALOGUE_EMOTION_VALUE_CONDITION);
    }

    #[test]
    fn fn_005d8640_returns_the_condition_function_s_result() {
        check_condition_only(0x005d_8640, IS_WATER_OBJECT_CONDITION);
    }

    #[test]
    fn fn_005d8320_switches_the_audio_thread_mode_only_when_it_changes() {
        let mut e = engine();
        let a = command(&mut e, 0);
        e.register(AUDIO_INSTANCE, |_, _| 0xa0d10u32.into_ret());
        e.register(AUDIO_SET_MULTI_THREADED, |_, _| Ret::default());
        // (value parsed, current byte, expected SetMultiThreaded calls)
        let cases = [
            (1u32, false, vec![vec![0xa0d10, 1]]),
            (1, true, vec![]),
            (0, true, vec![vec![0xa0d10, 0]]),
            (0, false, vec![]),
        ];
        for (value, current, expected) in cases {
            parse_gives(&mut e, true, &[value]);
            e.register_double(AUDIO_BYTE_6, move |_, _| current.into_ret());
            start_log(&mut e);
            assert!(run(&mut e, 0x005d_8320, a), "{value} {current}");
            assert_eq!(calls(&e, AUDIO_SET_MULTI_THREADED), expected);
        }
        parse_gives(&mut e, false, &[]);
        start_log(&mut e);
        assert!(!run(&mut e, 0x005d_8320, a));
        assert!(calls(&e, AUDIO_INSTANCE).is_empty());
    }

    #[test]
    fn fn_005d83b0_forwards_this_obj_and_zero() {
        let mut e = engine();
        let a = command(&mut e, 0x4444);
        e.register(REFERENCE_FN_0056A9B0, |_, _| Ret::default());
        start_log(&mut e);
        assert!(run(&mut e, 0x005d_83b0, a));
        assert_eq!(calls(&e, REFERENCE_FN_0056A9B0), vec![vec![0x4444, 0]]);
    }

    #[test]
    fn fn_005d83d0_forwards_this_obj_and_the_parsed_value() {
        let mut e = engine();
        let a = command(&mut e, 0x4444);
        e.register(REFERENCE_FN_0056A9B0, |_, _| Ret::default());
        parse_gives(&mut e, true, &[0x21]);
        start_log(&mut e);
        assert!(run(&mut e, 0x005d_83d0, a));
        assert_eq!(calls(&e, REFERENCE_FN_0056A9B0), vec![vec![0x4444, 0x21]]);
        parse_gives(&mut e, false, &[]);
        start_log(&mut e);
        assert!(!run(&mut e, 0x005d_83d0, a));
        assert!(calls(&e, REFERENCE_FN_0056A9B0).is_empty());
    }

    // ---- Cell commands ----

    #[test]
    fn fn_005d8420_builds_the_point_and_hands_it_to_the_cell() {
        let mut e = engine();
        let player_object = set_player(&mut e);
        let a = command(&mut e, 0);
        e.register(MEMSET, |e, a| {
            for i in 0..a[2] {
                e.mem.set_u8(a[0] + i, a[1] as u8);
            }
            a[0].into_ret()
        });
        e.register(STRLEN, |e, a| (e.mem.cstr(a[0]).len() as u32).into_ret());
        e.register(POINT3_CONSTRUCT, |_, a| a[0].into_ret());
        e.register(CELL_FN_005578E0, |_, _| Ret::default());
        e.register(PLAYER_GET_PARENT_CELL, |_, _| 0xce11u32.into_ret());
        let (x, y, z) = (1.0f32.to_bits(), 2.0f32.to_bits(), 3.0f32.to_bits());

        // With a text: the text address is passed on; 13 % 10 = entry 3.
        e.register_double(PARSE_PARAMETERS, move |e, a| {
            // The text buffer was zeroed before the parse.
            assert_eq!(e.mem.u32(a[10]), 0);
            e.mem.set_u32(a[7], x);
            e.mem.set_u32(a[8], y);
            e.mem.set_u32(a[9], z);
            e.mem.set_cstr(a[10], b"Marker");
            e.mem.set_u32(a[11], 13);
            true.into_ret()
        });
        start_log(&mut e);
        assert!(run(&mut e, 0x005d_8420, a));
        let memset = calls(&e, MEMSET)[0].clone();
        assert_eq!(memset[1..], [0, 0x1ff]);
        let point = calls(&e, POINT3_CONSTRUCT)[0][0];
        assert_eq!(calls(&e, POINT3_CONSTRUCT), vec![vec![point, x, y, z]]);
        assert_eq!(calls(&e, PLAYER_GET_PARENT_CELL)[0], vec![player_object]);
        let cell_call = calls(&e, CELL_FN_005578E0)[0].clone();
        assert_eq!(cell_call[0], 0xce11);
        assert_eq!(cell_call[1], point);
        assert_eq!(cell_call[2], memset[0] - 1);
        assert_eq!(cell_call[3], 0x0120_3288);

        // An empty text passes 0; entry 4.
        e.register_double(PARSE_PARAMETERS, move |e, a| {
            e.mem.set_u32(a[11], 4);
            true.into_ret()
        });
        start_log(&mut e);
        assert!(run(&mut e, 0x005d_8420, a));
        let cell_call = calls(&e, CELL_FN_005578E0)[0].clone();
        assert_eq!(cell_call[2..], [0, 0x0120_3298]);

        // No parent cell: nothing is built.
        e.register(PLAYER_GET_PARENT_CELL, |_, _| 0u32.into_ret());
        start_log(&mut e);
        assert!(run(&mut e, 0x005d_8420, a));
        assert!(calls(&e, POINT3_CONSTRUCT).is_empty());

        parse_gives(&mut e, false, &[]);
        assert!(!run(&mut e, 0x005d_8420, a));
    }

    #[test]
    fn fn_005d8550_indexes_the_table_modulo_ten() {
        let mut e = engine();
        assert_eq!(e.call(0x005d_8550, &args![0u32]).u32(), 0x0120_3258);
        assert_eq!(e.call(0x005d_8550, &args![13u32]).u32(), 0x0120_3288);
        assert_eq!(e.call(0x005d_8550, &args![9u32]).u32(), 0x0120_32e8);
        assert_eq!(e.call(0x005d_8550, &args![10u32]).u32(), 0x0120_3258);
        // 0xffffffff % 10 = 5 (unsigned).
        assert_eq!(
            e.call(0x005d_8550, &args![0xffff_ffffu32]).u32(),
            0x0120_3258 + 5 * 0x10
        );
    }

    #[test]
    fn fn_005d8570_calls_the_cell_only_when_the_player_has_one() {
        let mut e = engine();
        let player_object = set_player(&mut e);
        e.register(CELL_FN_005578F0, |_, _| Ret::default());
        e.register(PLAYER_GET_PARENT_CELL, |_, _| 0xce11u32.into_ret());
        start_log(&mut e);
        assert!(e.call(0x005d_8570, &args![]).bool());
        assert_eq!(calls(&e, CELL_FN_005578F0), vec![vec![0xce11]]);
        assert_eq!(calls(&e, PLAYER_GET_PARENT_CELL)[0], vec![player_object]);
        e.register(PLAYER_GET_PARENT_CELL, |_, _| 0u32.into_ret());
        start_log(&mut e);
        assert!(e.call(0x005d_8570, &args![]).bool());
        assert!(calls(&e, CELL_FN_005578F0).is_empty());
    }

    #[test]
    fn get_is_object_type_passes_the_parsed_byte_to_the_condition_function() {
        let mut e = engine();
        let a = command(&mut e, 0x4444);
        e.register(GET_IS_OBJECT_TYPE_CONDITION, |_, _| true.into_ret());
        // Only the low byte of the local counts.
        parse_gives(&mut e, true, &[0x1234_5642]);
        start_log(&mut e);
        assert!(run(&mut e, 0x005d_85a0, a));
        assert_parsed(&e, 0x4444);
        assert_eq!(
            calls(&e, GET_IS_OBJECT_TYPE_CONDITION),
            vec![vec![0x4444, 0x42, 0, a.result.addr()]]
        );
        e.register(GET_IS_OBJECT_TYPE_CONDITION, |_, _| false.into_ret());
        assert!(!run(&mut e, 0x005d_85a0, a));
        parse_gives(&mut e, false, &[]);
        start_log(&mut e);
        assert!(!run(&mut e, 0x005d_85a0, a));
        assert!(calls(&e, GET_IS_OBJECT_TYPE_CONDITION).is_empty());
    }

    // ---- VATS light and the small forwarders ----

    #[test]
    fn toggle_vats_light_flips_the_state_through_the_derived_object() {
        let mut e = engine();
        e.register(DEREF, |_, a| (a[0] + 0x1000).into_ret());
        e.register(DEREF_AT_F8, |_, a| (a[0] + 0x10).into_ret());
        e.register(FN_00456610, |_, _| true.into_ret());
        e.register(FN_00450F90, |_, _| Ret::default());
        e.register(FN_00450B80, |_, _| 0xe0e0u32.into_ret());
        e.register(FN_00B5D270, |_, _| Ret::default());

        // State set: "On", the flag is cleared, the entry gets the value.
        start_log(&mut e);
        assert!(e.call(0x005d_8660, &args![]).bool());
        assert_eq!(calls(&e, CONSOLE_PRINT), vec![vec![MSG_VATS_LIGHTING_ON]]);
        let value = OBJECT_AT_11F2250 + 0x34 + 0x1000;
        assert_eq!(calls(&e, DEREF)[0], vec![OBJECT_AT_11F2250 + 0x34]);
        assert_eq!(calls(&e, FN_00450F90), vec![vec![value + 0x10, 0]]);
        assert_eq!(calls(&e, FN_00450B80), vec![vec![0]]);
        assert_eq!(calls(&e, FN_00B5D270), vec![vec![0xe0e0, value]]);

        // State clear: "Off", the flag is set.
        e.register(FN_00456610, |_, _| false.into_ret());
        start_log(&mut e);
        assert!(e.call(0x005d_8660, &args![]).bool());
        assert_eq!(calls(&e, CONSOLE_PRINT), vec![vec![MSG_VATS_LIGHTING_OFF]]);
        assert_eq!(calls(&e, FN_00450F90), vec![vec![value + 0x10, 1]]);
        assert!(calls(&e, FN_00B5D270).is_empty());

        // Nothing to toggle.
        e.register(DEREF, |_, _| 0u32.into_ret());
        start_log(&mut e);
        assert!(e.call(0x005d_8660, &args![]).bool());
        assert!(calls(&e, CONSOLE_PRINT).is_empty());
        assert!(calls(&e, FN_00450F90).is_empty());
    }

    #[test]
    fn fn_005d8710_reads_the_word_at_offset_0x34() {
        let mut e = engine();
        e.register(DEREF, |e, a| e.mem.u32(a[0]).into_ret());
        let holder = object(&mut e);
        e.mem.set_u32(holder + 0x34, 0x1357);
        assert_eq!(e.call(0x005d_8710, &args![holder]).u32(), 0x1357);
    }

    #[test]
    fn fn_005d8730_passes_the_value_zero_float_and_zero() {
        let mut e = engine();
        let a = command(&mut e, 0);
        e.register(FN_004E0330, |_, _| Ret::default());
        parse_gives(&mut e, true, &[0x44]);
        start_log(&mut e);
        assert!(run(&mut e, 0x005d_8730, a));
        assert_eq!(calls(&e, FN_004E0330), vec![vec![0x44, 0, 0]]);
        parse_gives(&mut e, false, &[]);
        start_log(&mut e);
        assert!(!run(&mut e, 0x005d_8730, a));
        assert!(calls(&e, FN_004E0330).is_empty());
    }

    #[test]
    fn fn_005d8790_and_fn_005d8920_call_their_callee() {
        let mut e = engine();
        e.register(FN_004E0870, |_, _| Ret::default());
        e.register(FN_004DEFB0, |_, _| Ret::default());
        start_log(&mut e);
        assert!(e.call(0x005d_8790, &args![]).bool());
        assert!(e.call(0x005d_8920, &args![]).bool());
        assert_eq!(calls(&e, FN_004E0870).len(), 1);
        assert_eq!(calls(&e, FN_004DEFB0).len(), 1);
    }

    #[test]
    fn fn_005d87a0_reorders_the_parsed_words_for_the_callee() {
        let mut e = engine();
        let a = command(&mut e, 0);
        e.register(FN_004DE8E0, |_, _| Ret::default());
        parse_gives(&mut e, true, &[0xa, 0xb, 0xc, 0xd]);
        start_log(&mut e);
        assert!(run(&mut e, 0x005d_87a0, a));
        assert_eq!(calls(&e, FN_004DE8E0), vec![vec![0xa, 0xb, 0, 0xc, 0xd]]);
        parse_gives(&mut e, false, &[]);
        start_log(&mut e);
        assert!(!run(&mut e, 0x005d_87a0, a));
        assert!(calls(&e, FN_004DE8E0).is_empty());
    }

    #[test]
    fn fn_005d8830_passes_the_float_and_whether_the_byte_is_set() {
        let mut e = engine();
        let a = command(&mut e, 0);
        e.register(FN_004DEF00, |_, _| Ret::default());
        let value = 2.5f32.to_bits();
        for (byte, flag) in [(0u32, 0u32), (1, 1), (0x100, 0), (0x80, 1)] {
            parse_gives(&mut e, true, &[value, byte]);
            start_log(&mut e);
            assert!(run(&mut e, 0x005d_8830, a));
            assert_eq!(calls(&e, FN_004DEF00), vec![vec![value, flag]], "{byte}");
        }
        parse_gives(&mut e, false, &[]);
        start_log(&mut e);
        assert!(!run(&mut e, 0x005d_8830, a));
        assert!(calls(&e, FN_004DEF00).is_empty());
    }

    #[test]
    fn fn_005d88a0_passes_the_text_and_the_byte() {
        let mut e = engine();
        let a = command(&mut e, 0);
        e.register_double(FN_004DEED0, |e, a| {
            assert_eq!(e.mem.cstr(a[0]), b"abc");
            Ret::default()
        });
        e.register_double(PARSE_PARAMETERS, |e, a| {
            e.mem.set_cstr(a[7], b"abc");
            e.mem.set_u32(a[8], 0x177);
            true.into_ret()
        });
        start_log(&mut e);
        assert!(run(&mut e, 0x005d_88a0, a));
        // Only the low byte is passed.
        assert_eq!(calls(&e, FN_004DEED0)[0][1], 0x77);
        parse_gives(&mut e, false, &[]);
        start_log(&mut e);
        assert!(!run(&mut e, 0x005d_88a0, a));
        assert!(calls(&e, FN_004DEED0).is_empty());
    }

    // ---- The dialogue reset and its functor ----

    #[test]
    fn fn_005d8930_resets_every_element_of_every_item_of_every_topic() {
        let mut e = engine();
        let player_object = set_player(&mut e);
        // Lists are chains of nodes `[data][next]`; the owner's first word
        // is the head, a topic's embedded list starts at `topic + 0x2c`.
        fn node(e: &mut Engine, data: u32, next: u32) -> u32 {
            let n = e.mem.alloc(8);
            e.mem.set_u32(n, data);
            e.mem.set_u32(n + 4, next);
            n
        }
        e.register(LIST_FIRST_NODE, |e, a| e.mem.u32(a[0]).into_ret());
        e.register(LIST_NEXT_NODE, |e, a| e.mem.u32(a[0] + 4).into_ret());
        e.register(LIST_NODE_DATA, |_, a| a[0].into_ret());
        // An item's first word is its array; the array has its count at
        // +0x0c (the form id accessor) and the elements from +0x20.
        e.register(ARRAY_AT_4, |e, a| e.mem.u32(a[0]).into_ret());
        e.register(ARRAY_ELEMENT_ADDRESS, |_, a| {
            (a[0] + 0x20 + 4 * a[1]).into_ret()
        });
        for addr in [
            RESET_PLAYER_GREET_FLAG,
            GET_TOPIC,
            ELEMENT_FN_0061F280,
            DEBUG_RESET_SAID_ONCE_FLAGS,
        ] {
            e.register(addr, |_, _| Ret::default());
        }
        let functor_seen = Rc::new(RefCell::new(vec![]));
        let seen = functor_seen.clone();
        e.register_double(PROCESS_LISTS_VISIT, move |e, a| {
            seen.borrow_mut()
                .push((a[0], e.mem.u32(a[1]), e.mem.u32(a[1] + 4)));
            Ret::default()
        });

        let make_array = |e: &mut Engine, elements: &[u32]| {
            let array = e.mem.alloc(0x40);
            e.mem.set_u32(array + 0x0c, elements.len() as u32);
            for (i, element) in elements.iter().enumerate() {
                e.mem.set_u32(array + 0x20 + 4 * i as u32, *element);
            }
            array
        };
        let make_item = |e: &mut Engine, array: u32| {
            let item = e.mem.alloc(8);
            e.mem.set_u32(item, array);
            item
        };
        let array1 = make_array(&mut e, &[0x101, 0, 0x102]);
        let array2 = make_array(&mut e, &[0x103]);
        let item1 = make_item(&mut e, array1);
        let item2 = make_item(&mut e, array2);
        // Topic 1: items `item1`, a null entry, `item2`. Topic 2: empty.
        let topic1 = e.mem.alloc(0x40);
        let last = node(&mut e, item2, 0);
        let null_entry = node(&mut e, 0, last);
        e.mem.set_u32(topic1 + 0x2c, item1);
        e.mem.set_u32(topic1 + 0x30, null_entry);
        let topic2 = e.mem.alloc(0x40);
        let n3 = node(&mut e, topic2, 0);
        let n2 = node(&mut e, 0, n3);
        let n1 = node(&mut e, topic1, n2);
        let owner = e.mem.alloc(0x20);
        e.mem.set_u32(owner, n1);
        e.set_global(LIST_OWNER, owner);

        start_log(&mut e);
        assert!(e.call(0x005d_8930, &args![]).bool());
        // The functor handed to the process lists is the constructed one.
        assert_eq!(
            *functor_seen.borrow(),
            vec![(PROCESS_LISTS, FUNCTOR_VTABLE, 0)]
        );
        assert_eq!(
            calls(&e, RESET_PLAYER_GREET_FLAG),
            vec![vec![player_object]]
        );
        assert_eq!(calls(&e, GET_TOPIC), vec![vec![6, 0xc], vec![1, 0]]);
        assert_eq!(
            calls(&e, ELEMENT_FN_0061F280),
            vec![vec![0x101], vec![0x102], vec![0x103]]
        );
        let log = e.call_log.as_ref().unwrap();
        assert_eq!(log.last().unwrap().0, DEBUG_RESET_SAID_ONCE_FLAGS);
    }

    #[test]
    fn fn_005d8a70_is_the_embedded_list_of_a_topic() {
        let mut e = engine();
        assert_eq!(e.call(0x005d_8a70, &args![0x1000u32]).u32(), 0x102c);
    }

    #[test]
    fn fn_005d8a90_removes_extra_data_and_notifies_the_process_of_a_tested_actor() {
        let mut e = engine();
        let process = object_with(&mut e, &[(PROCESS_SLOT_310, V_PROCESS)]);
        let actor = object_with(&mut e, &[(REFERENCE_TEST_SLOT, V_TRUE)]);
        e.register_double(GET_PROCESS, move |_, _| process.into_ret());
        e.register(EXTRA_DATA_LIST, |_, a| (a[0] + 0x44).into_ret());
        e.register(REMOVE_EXTRA, |_, _| Ret::default());
        start_log(&mut e);
        e.call(0x005d_8a90, &args![0x9999u32, actor]);
        assert_eq!(calls(&e, V_PROCESS), vec![vec![process, 0]]);
        assert_eq!(calls(&e, REMOVE_EXTRA), vec![vec![actor + 0x44, 0x73]]);

        // No process: only the extra data is removed.
        e.register(GET_PROCESS, |_, _| 0u32.into_ret());
        start_log(&mut e);
        e.call(0x005d_8a90, &args![0x9999u32, actor]);
        assert!(calls(&e, V_PROCESS).is_empty());
        assert_eq!(calls(&e, REMOVE_EXTRA).len(), 1);

        // The virtual test fails: the process is not asked at all.
        let other = object_with(&mut e, &[(REFERENCE_TEST_SLOT, V_FALSE)]);
        start_log(&mut e);
        e.call(0x005d_8a90, &args![0x9999u32, other]);
        assert!(calls(&e, GET_PROCESS).is_empty());
        assert_eq!(calls(&e, REMOVE_EXTRA), vec![vec![other + 0x44, 0x73]]);
    }

    #[test]
    fn the_functor_constructors_set_the_vtables_and_clear_the_word() {
        let mut e = engine();
        let functor = e.mem.alloc(8);
        e.mem.set_u32(functor + 4, 0xdead);
        assert_eq!(e.call(0x005d_8b10, &args![functor]).u32(), functor);
        assert_eq!(e.mem.u32(functor), FUNCTOR_BASE_VTABLE);
        assert_eq!(e.mem.u32(functor + 4), 0);
        e.mem.set_u32(functor + 4, 0xdead);
        assert_eq!(e.call(0x005d_8af0, &args![functor]).u32(), functor);
        assert_eq!(e.mem.u32(functor), FUNCTOR_VTABLE);
        assert_eq!(e.mem.u32(functor + 4), 0);
    }

    #[test]
    fn fn_005d8b40_calls_the_player() {
        let mut e = engine();
        let player_object = set_player(&mut e);
        e.register(PLAYER_FN_00966D70, |_, _| Ret::default());
        start_log(&mut e);
        assert!(e.call(0x005d_8b40, &args![]).bool());
        assert_eq!(calls(&e, PLAYER_FN_00966D70), vec![vec![player_object]]);
    }

    #[test]
    fn exit_game_stores_two_when_the_player_has_the_perk_and_one_otherwise() {
        let mut e = engine();
        let player_object = object_with(&mut e, &[(PLAYER_HAS_PERK_SLOT, V_PERK_HELD)]);
        e.set_global(PLAYER, player_object);
        e.set_global(EXIT_GAME_GLOBAL, 0x5050u32);
        e.register(LOOKUP_FORM, |_, _| 0x77u32.into_ret());
        e.register(DYNAMIC_CAST, |_, _| 0x88u32.into_ret());
        e.register(TES_GLOBAL_SET_VALUE, |_, _| Ret::default());
        e.register(CLOSE_CONSOLE, |_, _| Ret::default());
        e.register(CHOOSE_MAIN_MENU, |_, _| Ret::default());

        start_log(&mut e);
        assert!(e.call(0x005d_8b60, &args![]).bool());
        assert_eq!(calls(&e, LOOKUP_FORM), vec![vec![0x000e_d568]]);
        assert_eq!(
            calls(&e, DYNAMIC_CAST),
            vec![vec![0x77, 0, 0x0118_3028, 0x0118_61dc, 0]]
        );
        assert_eq!(calls(&e, V_PERK_HELD), vec![vec![player_object, 0x88, 0]]);
        assert_eq!(
            calls(&e, TES_GLOBAL_SET_VALUE),
            vec![vec![0x5050, 2.0f32.to_bits()]]
        );
        let log = e.call_log.as_ref().unwrap();
        let order: Vec<u32> = log.iter().map(|(a, _)| *a).collect();
        assert_eq!(order[order.len() - 2..], [CLOSE_CONSOLE, CHOOSE_MAIN_MENU]);

        let player_object = object_with(&mut e, &[(PLAYER_HAS_PERK_SLOT, V_PERK_NOT_HELD)]);
        e.set_global(PLAYER, player_object);
        start_log(&mut e);
        assert!(e.call(0x005d_8b60, &args![]).bool());
        assert_eq!(
            calls(&e, TES_GLOBAL_SET_VALUE),
            vec![vec![0x5050, 1.0f32.to_bits()]]
        );
    }

    #[test]
    fn fn_005d8bf0_hands_the_words_the_text_and_a_string_to_the_script_body() {
        let mut e = engine();
        let a = command(&mut e, 0x4444);
        e.register(STRING_CONSTRUCT, |_, _| Ret::default());
        e.register(STRING_DESTRUCT, |_, _| Ret::default());
        e.register_double(PARSE_PARAMETERS, |e, a| {
            e.mem.set_cstr(a[7], b"hello");
            true.into_ret()
        });
        e.register_double(SCRIPT_BODY_005B4960, |e, a| {
            assert_eq!(e.mem.cstr(a[8]), b"hello");
            true.into_ret()
        });
        start_log(&mut e);
        assert!(run(&mut e, 0x005d_8bf0, a));
        let body = calls(&e, SCRIPT_BODY_005B4960)[0].clone();
        assert_eq!(body[..8], args![a][..]);
        let string = body[9];
        assert_eq!(calls(&e, STRING_CONSTRUCT), vec![vec![string]]);
        assert_eq!(calls(&e, STRING_DESTRUCT), vec![vec![string]]);

        // The body's result is the command's.
        e.register(SCRIPT_BODY_005B4960, |_, _| false.into_ret());
        assert!(!run(&mut e, 0x005d_8bf0, a));
        assert_eq!(calls(&e, STRING_DESTRUCT).len(), 2);

        parse_gives(&mut e, false, &[]);
        start_log(&mut e);
        assert!(!run(&mut e, 0x005d_8bf0, a));
        assert!(calls(&e, STRING_CONSTRUCT).is_empty());
    }

    #[test]
    fn trigger_enter_sets_the_action_and_runs_the_script_below_depth_five() {
        let mut e = engine();
        let this_obj = object(&mut e);
        let a = command(&mut e, this_obj);
        e.register(EXTRA_DATA_LIST, |_, a| (a[0] + 0x44).into_ret());
        e.register(INIT_ACTION_LIST, |_, _| Ret::default());
        e.register(SET_ACTION_FLAG, |_, _| Ret::default());
        e.register(SET_ACTION_REF, |_, _| Ret::default());
        let depth_seen = Rc::new(Cell::new(-1));
        let seen = depth_seen.clone();
        e.register_double(RUN_SCRIPT, move |e, _| {
            let tls = e.tls();
            seen.set(e.mem.i32(tls + TLS_SCRIPT_DEPTH));
            Ret::default()
        });
        let tls = e.tls();
        e.mem.set_i32(tls + TLS_SCRIPT_DEPTH, 4);
        parse_gives(&mut e, true, &[0x6666]);
        start_log(&mut e);
        assert!(run(&mut e, 0x005d_8cf0, a));
        let list = this_obj + 0x44;
        assert_eq!(calls(&e, INIT_ACTION_LIST), vec![vec![this_obj, list]]);
        assert_eq!(
            calls(&e, SET_ACTION_FLAG),
            vec![
                vec![0x6666, list, 0x2000_0000],
                vec![0x6666, list, 0x1000_0000]
            ]
        );
        assert_eq!(calls(&e, SET_ACTION_REF), vec![vec![this_obj, 0x6666]]);
        assert_eq!(calls(&e, RUN_SCRIPT), vec![vec![this_obj]]);
        // The depth was raised while the script ran and restored after.
        assert_eq!(depth_seen.get(), 5);
        assert_eq!(e.mem.i32(tls + TLS_SCRIPT_DEPTH), 4);

        // At depth 5 the script is not run (the rest still is).
        e.mem.set_i32(tls + TLS_SCRIPT_DEPTH, 5);
        start_log(&mut e);
        assert!(run(&mut e, 0x005d_8cf0, a));
        assert!(calls(&e, RUN_SCRIPT).is_empty());
        assert_eq!(calls(&e, SET_ACTION_REF).len(), 1);
        assert_eq!(e.mem.i32(tls + TLS_SCRIPT_DEPTH), 5);

        // No thisObj: only the parse. A failing parse returns false.
        let none = command(&mut e, 0);
        start_log(&mut e);
        assert!(run(&mut e, 0x005d_8cf0, none));
        assert!(calls(&e, INIT_ACTION_LIST).is_empty());
        parse_gives(&mut e, false, &[]);
        assert!(!run(&mut e, 0x005d_8cf0, a));
    }

    #[test]
    fn fn_005d8e20_registers_an_ordinary_reference_without_a_file() {
        let mut e = engine();
        let player_object = set_player(&mut e);
        let target = object_with(&mut e, &[(REFERENCE_SLOT_C4, V_TRUE)]);
        e.register(FORM_IS_DELETED_FLAG, |_, _| false.into_ret());
        e.register(GET_FILE, |_, _| 0u32.into_ret());
        e.register_double(LIST_ADD_REFERENCE, move |e, a| {
            // The reference is passed by the address of a word.
            assert_eq!(e.mem.u32(a[1]), target);
            Ret::default()
        });
        let a = command(&mut e, target);
        start_log(&mut e);
        assert!(run(&mut e, 0x005d_8e20, a));
        assert_eq!(calls(&e, V_TRUE), vec![vec![target, 1]]);
        assert_eq!(calls(&e, GET_FILE), vec![vec![target, 0]]);
        assert_eq!(calls(&e, LIST_ADD_REFERENCE).len(), 1);
        assert_eq!(calls(&e, LIST_ADD_REFERENCE)[0][0], REFERENCE_LIST);

        // A reference with a file is not added.
        e.register(GET_FILE, |_, _| 0xf11eu32.into_ret());
        start_log(&mut e);
        assert!(run(&mut e, 0x005d_8e20, a));
        assert!(calls(&e, LIST_ADD_REFERENCE).is_empty());

        // The player, a flagged form and no reference are left alone.
        let for_player = command(&mut e, player_object);
        start_log(&mut e);
        assert!(run(&mut e, 0x005d_8e20, for_player));
        assert!(calls(&e, GET_FILE).is_empty());
        e.register(FORM_IS_DELETED_FLAG, |_, _| true.into_ret());
        assert!(run(&mut e, 0x005d_8e20, a));
        assert!(calls(&e, GET_FILE).is_empty());
        let none = command(&mut e, 0);
        assert!(run(&mut e, 0x005d_8e20, none));
        assert!(calls(&e, GET_FILE).is_empty());
    }

    // ---- The AddItem-style body ----

    /// An engine for `005d8e80` with the container callees stubbed.
    fn engine_for_add_item() -> Engine {
        let mut e = engine();
        for addr in [
            CONTAINER_CONSTRUCT,
            CONTAINER_ADD_ITEM,
            CONTAINER_FN_00482090,
            CONTAINER_FN_004821A0,
            CONTAINER_DESTRUCT,
            FORM_ADD_LEVELED_ITEMS,
        ] {
            e.register(addr, |_, _| Ret::default());
        }
        e.register(GET_CALC_LEVEL, |_, _| 0x0001_0007u32.into_ret());
        e.register(LIST_NODE_DATA, |_, a| a[0].into_ret());
        e.register(LIST_NEXT_NODE, |e, a| e.mem.u32(a[0] + 4).into_ret());
        e.register(NODE_IS_EMPTY, |e, a| {
            (e.mem.u32(a[0]) == 0 && e.mem.u32(a[0] + 4) == 0).into_ret()
        });
        e
    }

    fn add_item_command(e: &mut Engine, this_obj: u32) -> ScriptArgs {
        let script = object_with(e, &[(SCRIPT_NAME_SLOT, V_NAME)]);
        let mut a = command(e, this_obj);
        a.script_obj = Ptr::new(script);
        a
    }

    #[test]
    fn fn_005d8e80_adds_leveled_items_by_level_and_count() {
        let mut e = engine_for_add_item();
        let a = add_item_command(&mut e, 0x4444);
        let item = object(&mut e);
        e.register(FORM_TYPE, |_, _| 0x34u32.into_ret());
        let quality = 0.5f32.to_bits();
        parse_gives(&mut e, true, &[item, 0x0002_0005, quality, 1]);
        start_log(&mut e);
        assert!(run(&mut e, 0x005d_8e80, a));
        assert_eq!(
            calls(&e, PARSE_PARAMETERS)[0][..7],
            [1, 2, 8, 0x4444, 0, a.script_obj.addr(), 6]
        );
        let container = calls(&e, CONTAINER_CONSTRUCT)[0][0];
        assert_eq!(calls(&e, GET_CALC_LEVEL), vec![vec![0x4444, 0]]);
        assert_eq!(
            calls(&e, FORM_ADD_LEVELED_ITEMS),
            vec![vec![item + 0x30, 7, 5, container, 0]]
        );
        assert_eq!(
            calls(&e, CONTAINER_FN_00482090),
            vec![vec![container, quality]]
        );
        assert_eq!(
            calls(&e, CONTAINER_FN_004821A0),
            vec![vec![container, 0x4444, 1]]
        );
        assert_eq!(calls(&e, CONTAINER_DESTRUCT), vec![vec![container]]);
        assert!(calls(&e, LOG_STUB).is_empty());

        // The integer decides the flag: anything but 1 passes 0.
        parse_gives(&mut e, true, &[item, 5, quality, 2]);
        start_log(&mut e);
        assert!(run(&mut e, 0x005d_8e80, a));
        assert_eq!(calls(&e, CONTAINER_FN_004821A0)[0][2], 0);
    }

    #[test]
    fn fn_005d8e80_adds_the_entries_of_a_list_that_pass_the_test() {
        let mut e = engine_for_add_item();
        let a = add_item_command(&mut e, 0x4444);
        let list_form = object(&mut e);
        let passing = object_with(&mut e, &[(FORM_TEST_SLOT, V_TRUE)]);
        let failing = object_with(&mut e, &[(FORM_TEST_SLOT, V_FALSE)]);
        // Nodes `[data][next]`: passing, failing, then an empty node that
        // ends the walk.
        let n3 = e.mem.alloc(8);
        let n2 = e.mem.alloc(8);
        e.mem.set_u32(n2, failing);
        e.mem.set_u32(n2 + 4, n3);
        let n1 = e.mem.alloc(8);
        e.mem.set_u32(n1, passing);
        e.mem.set_u32(n1 + 4, n2);
        // The first node of the item's list: `item + 0x18` in the game.
        e.register_double(LIST_AT_18, move |_, _| n1.into_ret());
        e.register(FORM_TYPE, |_, _| 0x55u32.into_ret());
        parse_gives(&mut e, true, &[list_form, 3, 0, 0]);
        start_log(&mut e);
        assert!(run(&mut e, 0x005d_8e80, a));
        let container = calls(&e, CONTAINER_CONSTRUCT)[0][0];
        assert_eq!(calls(&e, LIST_AT_18), vec![vec![list_form]]);
        assert_eq!(
            calls(&e, CONTAINER_ADD_ITEM),
            vec![vec![container, passing, 3, 0]]
        );
        assert!(calls(&e, LOG_STUB).is_empty());

        // Without a count the list is not used and the failure is logged.
        parse_gives(&mut e, true, &[list_form, 0, 0, 0]);
        start_log(&mut e);
        assert!(run(&mut e, 0x005d_8e80, a));
        assert!(calls(&e, CONTAINER_ADD_ITEM).is_empty());
        assert_eq!(calls(&e, LOG_STUB), vec![vec![MSG_ADD_ITEM_FAILED, 0xbbbb]]);
        // The container is still applied and destroyed.
        assert_eq!(calls(&e, CONTAINER_FN_004821A0).len(), 1);
        assert_eq!(calls(&e, CONTAINER_DESTRUCT).len(), 1);
    }

    #[test]
    fn fn_005d8e80_adds_a_single_item_that_passes_the_test() {
        let mut e = engine_for_add_item();
        let a = add_item_command(&mut e, 0x4444);
        e.register(FORM_TYPE, |_, _| 0x28u32.into_ret());
        let item = object_with(&mut e, &[(FORM_TEST_SLOT, V_TRUE)]);
        parse_gives(&mut e, true, &[item, 4, 0, 0]);
        start_log(&mut e);
        assert!(run(&mut e, 0x005d_8e80, a));
        let container = calls(&e, CONTAINER_CONSTRUCT)[0][0];
        assert_eq!(
            calls(&e, CONTAINER_ADD_ITEM),
            vec![vec![container, item, 4, 0]]
        );
        assert!(calls(&e, LOG_STUB).is_empty());

        // A zero count: logged. An item that fails the test: logged.
        parse_gives(&mut e, true, &[item, 0, 0, 0]);
        start_log(&mut e);
        assert!(run(&mut e, 0x005d_8e80, a));
        assert!(calls(&e, CONTAINER_ADD_ITEM).is_empty());
        assert_eq!(calls(&e, LOG_STUB).len(), 1);
        let refusing = object_with(&mut e, &[(FORM_TEST_SLOT, V_FALSE)]);
        parse_gives(&mut e, true, &[refusing, 4, 0, 0]);
        start_log(&mut e);
        assert!(run(&mut e, 0x005d_8e80, a));
        assert!(calls(&e, CONTAINER_ADD_ITEM).is_empty());
        assert_eq!(calls(&e, LOG_STUB), vec![vec![MSG_ADD_ITEM_FAILED, 0xbbbb]]);
    }

    #[test]
    fn fn_005d8e80_needs_a_reference_and_parsed_parameters() {
        let mut e = engine_for_add_item();
        let none = add_item_command(&mut e, 0);
        parse_gives(&mut e, true, &[1, 1, 0, 0]);
        start_log(&mut e);
        assert!(run(&mut e, 0x005d_8e80, none));
        assert!(calls(&e, CONTAINER_CONSTRUCT).is_empty());
        assert!(calls(&e, GET_CALC_LEVEL).is_empty());
        let a = add_item_command(&mut e, 0x4444);
        parse_gives(&mut e, false, &[]);
        assert!(!run(&mut e, 0x005d_8e80, a));
    }

    #[test]
    fn fn_005d90a0_stores_the_id_of_the_form_the_script_body_makes() {
        let mut e = engine();
        let a = command(&mut e, 0x4444);
        e.mem.set_f64(a.result.addr(), 9.0);
        let made = object(&mut e);
        e.mem.set_u32(made + 0x0c, 0x0abc);
        e.register(PUT_NUMERIC_ID_IN_DOUBLE, |e, a| {
            let id = e.mem.u32(a[0]);
            e.mem.set_f64(a[1], id as f64);
            Ret::default()
        });
        e.register_double(SCRIPT_BODY_005C4B30, move |_, _| made.into_ret());
        let scale = 2.0f32.to_bits();
        e.register_double(PARSE_PARAMETERS, move |e, a| {
            // The documented defaults are in the locals before the parse.
            assert_eq!(e.mem.u32(a[7]), 0);
            assert_eq!(e.mem.u32(a[8]), 1.0f32.to_bits());
            assert_eq!(e.mem.u32(a[9]), 1);
            assert_eq!(e.mem.u32(a[10]), 0);
            assert_eq!(e.mem.u32(a[11]), 0);
            e.mem.set_u32(a[7], 0x11);
            e.mem.set_u32(a[8], scale);
            e.mem.set_u32(a[9], 5);
            e.mem.set_u32(a[10], 6);
            e.mem.set_u32(a[11], 7);
            true.into_ret()
        });
        start_log(&mut e);
        assert!(run(&mut e, 0x005d_90a0, a));
        assert_eq!(
            calls(&e, SCRIPT_BODY_005C4B30),
            vec![vec![0x4444, 0x11, 5, 6, 7, scale]]
        );
        assert_eq!(e.mem.f64(a.result.addr()), 0x0abc as f64);

        // Nothing made: the result stays 0.0.
        e.register(SCRIPT_BODY_005C4B30, |_, _| 0u32.into_ret());
        e.mem.set_f64(a.result.addr(), 9.0);
        assert!(run(&mut e, 0x005d_90a0, a));
        assert_eq!(e.mem.f64(a.result.addr()), 0.0);

        // A failing parse still clears the result.
        e.mem.set_f64(a.result.addr(), 9.0);
        parse_gives(&mut e, false, &[]);
        start_log(&mut e);
        assert!(!run(&mut e, 0x005d_90a0, a));
        assert_eq!(e.mem.f64(a.result.addr()), 0.0);
        assert!(calls(&e, SCRIPT_BODY_005C4B30).is_empty());
    }
}
