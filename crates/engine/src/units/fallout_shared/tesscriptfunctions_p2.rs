//! `fallout shared/tesscriptfunctions.cpp` (Xbox PDB source unit), part 2: its functions from `005bc430` up to
//! (not including) `005c4240` in FalloutNV.exe 1.4.0.525, translated
//! (docs/ENGINE_CRATE.md). The unit's shared layouts and helpers are in
//! [`super::tesscriptfunctions`]; anything public there may be used here.
//!
//! Progress: the first 80 queue entries of this part (`005bc430` to
//! `005c0bf0`, in two sessions of 40) are translated. The next session
//! continues at `005c0db0` (the next entry of the queue after `005c0bf0`).
//!
//! Every body is `cdecl` with the eight stack words of [`ScriptArgs`]; most
//! hand them to `Script::ParseParameters` (`005accb0`) to read their
//! arguments into locals passed by address.
//!
//! Notes on the exe's code that the translations rely on:
//! - Several callees are getters that take no argument although the caller
//!   pushed the arguments of the *next* call before it (`push x; call
//!   getter; mov ecx, eax; call method` with `ret 4` in the method): the
//!   getter is called without arguments here and the method gets `x`.
//! - A float returned in `ST0` is read with `.f32()`/`.f64()`; a float or
//!   double the code pushes on the stack is an `f32`/`f64` argument.
//! - The compiler's exception-unwinding frames are not translated.

#[allow(unused_imports)]
use super::tesscriptfunctions::*;
#[allow(unused_imports)]
use crate::prelude::*;

// ---- Callees outside this part (by exe address) ----------------------------

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
/// `_ftol2_sse`: the `double` in `ST0` (a leading `f64` argument) truncated
/// into `EAX`.
const FTOL: u32 = 0x00ec_62c0;
/// `sscanf` (`text, format, ...`).
const SSCANF: u32 = 0x00ec_a4a6;
/// `strlen` through the game's wrapper (`text`).
const STRLEN: u32 = 0x0044_a670;
/// `strcpy_s(destination, size, source)`.
const STRING_COPY: u32 = 0x0040_6d30;
/// `BSstristr(haystack, needle)` (Xbox PDB): case-insensitive search,
/// non-zero when found.
const STRISTR: u32 = 0x004b_75a0;
/// `MemoryManager` deallocation, `cdecl(block)`.
const MEMORY_FREE: u32 = 0x0040_1030;
/// `MemoryManager` allocation, `cdecl(size) -> block`.
const MEMORY_ALLOC: u32 = 0x0040_1000;
/// Form type byte (`this + 4`).
const FORM_TYPE: u32 = 0x0040_1170;
/// `*this`: the text of a `BSStringT` (also an `NiPointer`'s object).
const STRING_TEXT: u32 = 0x0055_9450;
/// `BSStringT` constructor (an 8-byte object).
const BS_STRING_CONSTRUCT: u32 = 0x0040_37b0;
/// `BSStringT` destructor.
const BS_STRING_DESTRUCT: u32 = 0x0040_37d0;
/// `BSStringT` format (`string, format, ...`).
const BS_STRING_FORMAT: u32 = 0x0040_6f60;
/// Constructor of the 12-byte object the speak-sound call fills in (it
/// stores `-1`, `0`, `0`).
const SPEAK_RESULT_CONSTRUCT: u32 = 0x0041_a250;
/// Its (empty) destructor.
const SPEAK_RESULT_DESTRUCT: u32 = 0x0048_3710;
/// `MiddleHighProcess::GetSavedAcquireObject` (Xbox PDB), `thiscall` on the
/// actor: the actor's process.
const ACTOR_PROCESS: u32 = 0x008d_8520;
/// `MobileObject::GetCurrentProcessType` (Xbox PDB), `thiscall` on the
/// actor.
const CURRENT_PROCESS_TYPE: u32 = 0x0093_1850;
/// `thiscall` on a setting object: the address of its value (`this + 4`, or
/// a static zero for a null `this`).
const SETTING_VALUE_ADDRESS: u32 = 0x0043_d4d0;
/// `ActorValue::GetActorValueScriptName` (Xbox PDB), `cdecl(actor value)`.
const ACTOR_VALUE_SCRIPT_NAME: u32 = 0x0066_eac0;
/// `cdecl(actor value, mask)`: whether the actor value's table entry has a
/// flag of `mask` set (`table[value]->flags & mask`).
const ACTOR_VALUE_HAS_FLAG: u32 = 0x0040_6d70;
/// `cdecl(owner, actor value, float*)` (`fallout shared/actorvalue.cpp`):
/// the derived value; `AL` says whether there is one.
const ACTOR_VALUE_DERIVED: u32 = 0x0066_ed60;
/// `thiscall` on a reference: its name (`fallout shared/tesobjectrefr.cpp`).
const OBJECT_NAME: u32 = 0x0055_d520;
/// `thiscall` on an actor: the object whose `+0x100` part is the actor's
/// base-value `ActorValueOwner` (`extradatalist.cpp`).
const REFERENCE_BASE_OWNER: u32 = 0x0041_81e0;
/// `thiscall` on an actor: whether its values are auto-calculated
/// (`fallout/ai/actor.cpp`).
const ACTOR_IS_AUTO_CALCULATED: u32 = 0x0087_f4e0;

// Navigation mesh drawing.
/// Getter of the `NavMeshRender` singleton (loads global `011d6e2c`).
const NAV_MESH_RENDER: u32 = 0x0055_2ba0;
/// `thiscall` on the render object: byte at `+0x210`, "drawing".
const NAV_MESH_RENDER_IS_DRAWING: u32 = 0x0055_2bb0;
/// `NavMeshRender::SetDraw` (Xbox PDB), `thiscall(bool)`.
const NAV_MESH_RENDER_SET_DRAW: u32 = 0x006a_36e0;
/// `NavMeshRender::SetTransparentOverLayMode` (Xbox PDB), `thiscall(bool)`.
const NAV_MESH_RENDER_SET_TRANSPARENT: u32 = 0x006a_5ee0;
/// `NavMeshRender::AddDrawOnlyNavMeshes` (Xbox PDB), `thiscall(cell, 0)`.
const NAV_MESH_RENDER_ADD_CELL: u32 = 0x006a_37a0;
/// `TES::GetCurrentCell` (Xbox PDB), `thiscall` on the `TES` singleton.
const GET_CURRENT_CELL: u32 = 0x0045_7070;
/// `thiscall` on a cell: whether it is an interior (bit 0 of `+0x24`).
const CELL_IS_INTERIOR: u32 = 0x0042_5fd0;
/// `TESObjectCELL::GetDataX` (Xbox PDB).
const CELL_GET_DATA_X: u32 = 0x0054_4c30;
/// `TESObjectCELL::GetDataY` (Xbox PDB).
const CELL_GET_DATA_Y: u32 = 0x0054_4c60;
/// `TESObjectCELL::GetWorldSpace` (Xbox PDB), no argument.
const CELL_GET_WORLD_SPACE: u32 = 0x0054_ddd0;
/// `TESWorldSpace::GetCellFromCellCoord` (Xbox PDB), `thiscall(x, y)`.
const WORLD_SPACE_GET_CELL: u32 = 0x0058_75a0;
/// `TESWorldSpace::LoadCell` (Xbox PDB), `thiscall(x, y)`.
const WORLD_SPACE_LOAD_CELL: u32 = 0x0058_5b30;
/// `TESDataHandler::NewCell` (Xbox PDB), `thiscall(0, x, y, world space)`.
const DATA_HANDLER_NEW_CELL: u32 = 0x0046_1330;
/// `TES::GetWorldSpace` (Xbox PDB), `thiscall` on the `TES` singleton.
const TES_GET_WORLD_SPACE: u32 = 0x004f_d3e0;
/// `this + 0x10` of the data handler: the world space list.
const DATA_HANDLER_WORLD_SPACES: u32 = 0x0046_0140;
/// Returns `this`: the address of a list node's item.
const LIST_ITEM_PTR: u32 = 0x0068_15c0;
/// `ExteriorCellLoader::CancelAllCellLoads` (Xbox PDB), `thiscall`.
const CANCEL_ALL_CELL_LOADS: u32 = 0x0052_8540;
/// `Interface::CloseConsole` (Xbox PDB), `cdecl`.
const CLOSE_CONSOLE: u32 = 0x0070_3da0;
/// `PlayerCharacter::CenterOnCell` (Xbox PDB), `thiscall(cell name, cell)`.
const CENTER_ON_CELL: u32 = 0x0093_db60;

// Pathing and debug text.
/// `Interface::IsDebugTextVisible` (Xbox PDB), `cdecl`.
const IS_DEBUG_TEXT_VISIBLE: u32 = 0x0070_2fc0;
/// `Interface::ToggleDebugTextVisible` (Xbox PDB), `cdecl`.
const TOGGLE_DEBUG_TEXT_VISIBLE: u32 = 0x0070_3000;
/// Getter of the pathing debug object (loads global `011db800`, creating it
/// when it is still null); takes no argument.
const PATHING_DEBUG_OBJECT: u32 = 0x005b_a8d0;
/// `thiscall(text, 0)` on the pathing debug object.
const PATHING_DEBUG_ADD: u32 = 0x0080_29c0;
/// `DebugText::Instance` (Xbox PDB), `cdecl(1)`.
const DEBUG_TEXT_INSTANCE: u32 = 0x00a0_d9e0;
/// `DebugText::Print` (Xbox PDB), `thiscall(text, x, y, 2, -1, float, 0, 0)`.
const DEBUG_TEXT_PRINT: u32 = 0x00a0_f8b0;
/// `cdecl(reference)`: the console's "picked reference" update.
const PICK_REFERENCE: u32 = 0x0070_31e0;
/// `thiscall` on a form: the form id (`this + 0xc`).
const FORM_ID: u32 = 0x0084_e3a0;

// Forms and the navigation mesh obstacle manager.
/// `TESForm::GetFile` (Xbox PDB), `thiscall(index)`.
const FORM_GET_FILE: u32 = 0x0048_4e60;
/// `TESForm::AddCompileIndex` (Xbox PDB), `cdecl(form id*, file)`.
const ADD_COMPILE_INDEX: u32 = 0x0048_5d50;
/// `cdecl(form id)`: the obstacle manager's entry for the form.
const OBSTACLE_ENTRY: u32 = 0x006c_0720;
/// `thiscall` on the entry (`navmeshobstaclemanager.cpp`).
const OBSTACLE_ENTRY_ACTION_A: u32 = 0x006c_1150;
/// `thiscall` on the entry (`navmeshobstaclemanager.cpp`).
const OBSTACLE_ENTRY_ACTION_B: u32 = 0x006c_1130;

// Condition functions the script bodies forward to (`cdecl(object, value,
// 0, result)`).
const COND_FN_0059C4C0: u32 = 0x0059_c4c0;
const COND_FN_0059C4F0: u32 = 0x0059_c4f0;
/// `cdecl(object, value, 0, result)` (`tesconditionfunctions.cpp`).
const COND_FN_0059C680: u32 = 0x0059_c680;
/// `Script::GetFatiguePercentageConditionFunction` (Xbox PDB).
const GET_FATIGUE_PERCENTAGE_CONDITION: u32 = 0x0059_c760;
/// `Script::GetHealthPercentageConditionFunction` (Xbox PDB).
const GET_HEALTH_PERCENTAGE_CONDITION: u32 = 0x0059_c7e0;
/// `Script::GetBaseActorValueConditionFunction` (Xbox PDB).
const GET_BASE_ACTOR_VALUE_CONDITION: u32 = 0x0059_c5d0;
/// `Script::GetCauseofDeathConditionFunction` (Xbox PDB).
const GET_CAUSE_OF_DEATH_CONDITION: u32 = 0x005a_3d30;
/// `Script::IsLimbGoneConditionFunction` (Xbox PDB).
const IS_LIMB_GONE_CONDITION: u32 = 0x005a_3db0;
/// `Script::GetKillingBlowLimbConditionFunction` (Xbox PDB).
const GET_KILLING_BLOW_LIMB_CONDITION: u32 = 0x005a_3e10;
/// `cdecl(object, value, 0, result)` (`tesconditionfunctions.cpp`).
const COND_FN_005A3E90: u32 = 0x005a_3e90;

// Collections and settings (help command).
/// `thiscall` on a name collection: the number of names.
const COLLECTION_COUNT: u32 = 0x0099_38b0;
/// `thiscall(index)` on a name collection: the address of the element.
const COLLECTION_ELEMENT_ADDRESS: u32 = 0x0087_7a30;
/// Constructor of a name collection (0x10 bytes, `thiscall(0, 1)`).
const COLLECTION_CONSTRUCT: u32 = 0x004d_9100;
/// Its destructor.
const COLLECTION_DESTRUCT: u32 = 0x004d_9190;
/// Getter of the game setting collection; takes no argument.
const GAME_SETTINGS: u32 = 0x0040_4a70;
/// Getter of the INI setting collection (loads global `011f96a0`); takes no
/// argument.
const INI_SETTINGS: u32 = 0x0044_f560;

// Actors.
/// `Actor::RestoreActorValue` (Xbox PDB), `thiscall(actor value, amount)`.
const ACTOR_RESTORE_ACTOR_VALUE: u32 = 0x0088_b740;
/// `MobileObject::GetCharController` (Xbox PDB).
const GET_CHAR_CONTROLLER: u32 = 0x0093_06d0;
/// `bhkCharacterController::SetSize` (Xbox PDB), `thiscall(float)`.
const CHAR_CONTROLLER_SET_SIZE: u32 = 0x00c7_08a0;
/// `cdecl(float)` -> float in `ST0`: the magnitude used by the damage and
/// restore commands (`fallout shared/magic/effectsetting.cpp`).
const ABSOLUTE_VALUE: u32 = 0x0040_8840;
/// `cdecl`: whether kills are queued (`fallout/ai`), result in `AL`.
const KILLS_ARE_QUEUED: u32 = 0x008c_7aa0;
/// `thiscall` on the actor: its level (`u16` result).
const ACTOR_LEVEL: u32 = 0x0087_f9f0;
/// `ExperiencePoints::GetExperiencePoints` (Xbox PDB), `cdecl(flag, level)`.
const GET_EXPERIENCE_POINTS: u32 = 0x0067_05b0;
/// `cdecl(float, value)` -> float in `ST0` (`gameplayformulas.cpp`).
const GAMEPLAY_FORMULA: u32 = 0x0064_8c80;
/// `cdecl(float)` -> float in `ST0`: rounds the experience.
const ROUND_EXPERIENCE: u32 = 0x0040_6ce0;
/// `thiscall` on the actor (`xgamesetting.cpp`): the life state.
const ACTOR_LIFE_STATE: u32 = 0x004f_8960;
/// `Actor::SetLifeState` (Xbox PDB), `thiscall(0)`.
const ACTOR_SET_LIFE_STATE: u32 = 0x008a_1800;
/// `Actor::Kill` (Xbox PDB), `thiscall(killer, float)`.
const ACTOR_KILL: u32 = 0x0089_d900;
/// `Actor::ScriptDismember` (Xbox PDB), `thiscall(detail, part, killer)`.
const ACTOR_SCRIPT_DISMEMBER: u32 = 0x008b_51b0;
/// Getter of the task queue interface (loads global `011df1a8`); takes no
/// argument.
const TASK_QUEUE_INTERFACE: u32 = 0x0045_37b0;
/// `TaskQueueInterface::QueueActorKill` (Xbox PDB), `thiscall(actor, float*,
/// dismember data)`.
const QUEUE_ACTOR_KILL: u32 = 0x0087_aff0;
/// `thiscall(index)` on the process lists' array: the actor at the index.
const PROCESS_LISTS_ACTOR: u32 = 0x0096_8670;
/// `this + 4` of the process lists singleton (the engine map calls it
/// `BaseProcess::GetActorPackageThatIsRunning`).
const PROCESS_LISTS_ARRAY: u32 = 0x0071_7e50;
/// `thiscall(0)` on the global `011f11a0`, called before the virtual call of
/// [`fn_005be5e0`].
const SCRIPT_RUNNER_ENTER: u32 = 0x0040_fbf0;
/// Its counterpart called after the virtual call.
const SCRIPT_RUNNER_LEAVE: u32 = 0x0040_fba0;
/// `thiscall` on an actor: a value from the actor (`modelloader.cpp` range).
const ACTOR_FN_0043FCD0: u32 = 0x0043_fcd0;
/// `TESObjectREFR::GetDismembered` (`thiscall(limb)`).
const REFERENCE_IS_LIMB_DISMEMBERED: u32 = 0x0057_3090;

// ---- Globals -----------------------------------------------------------------

/// The `PlayerCharacter` singleton pointer.
const PLAYER: u32 = 0x011d_ea3c;
/// Pointer to the data handler singleton.
const DATA_HANDLER: u32 = 0x011c_3f2c;
/// Pointer to the `TES` singleton.
const TES_SINGLETON: u32 = 0x011d_ea10;
/// Pointer to the exterior cell loader (non-null while one exists).
const EXTERIOR_CELL_LOADER: u32 = 0x011c_9618;
/// The process lists singleton.
const PROCESS_LISTS: u32 = 0x011e_0e80;
/// The setting object whose value is the number of grids to load.
const GRIDS_TO_LOAD_SETTING: u32 = 0x011c_63cc;
/// The setting object `PickRefByID` reads for its line index.
const PICK_REF_LINE_SETTING: u32 = 0x011f_33c8;
/// The setting object [`fn_005be870`] reads.
const SETTING_011D0844: u32 = 0x011d_0844;
/// Base pointer of the array [`fn_005bd5c0`] indexes (read by
/// [`fn_005bd5b0`]).
const DEBUG_LINE_ARRAY: u32 = 0x011f_33f8;
/// The global [`SCRIPT_RUNNER_ENTER`] works on.
const SCRIPT_RUNNER: u32 = 0x011f_11a0;
/// Bytes switched by the four AI toggles.
const AI_LOW_PROCESS: u32 = 0x011f_1229;
const AI_MIDDLE_LOW_PROCESS: u32 = 0x011f_122b;
const AI_MIDDLE_HIGH_PROCESS: u32 = 0x011f_122a;
const AI_SCHEDULES: u32 = 0x011f_122c;
/// Byte of `Script::TogglePathingInfoFunction`.
const PATHING_INFO_ON: u32 = 0x011d_73cc;
/// Bytes and dword of the navigation mesh display modes.
const NAV_MESH_TOGGLE_5: u32 = 0x011d_6e28;
const NAV_MESH_TOGGLE_6: u32 = 0x011d_6e29;
const NAV_MESH_MODE: u32 = 0x011d_6e24;
const NAV_MESH_ENABLED: u32 = 0x011d_6ec4;
/// Table of console commands: [`HELP_COMMAND_COUNT`] entries of
/// [`HELP_ENTRY_SIZE`] bytes (name at +0, short name at +4, help at +0xc).
const COMMAND_TABLE: u32 = 0x0118_e8e0;
/// Table of script functions (same entry layout).
const FUNCTION_TABLE: u32 = 0x0119_0910;
const HELP_COMMAND_COUNT: u32 = 0xcd;
const HELP_FUNCTION_COUNT: u32 = 0x280;
const HELP_ENTRY_SIZE: u32 = 0x28;
/// Byte at `+0x268` of the TLS block: commands echo to the console when set.
const TLS_ECHO: u32 = 0x268;

/// Size of the text parameters `ParseParameters` writes.
const TEXT_PARAMETER_SIZE: u32 = 0x204;

// ---- String literals and float constants (addresses in the exe's data) --------

/// `"Off"`
const TEXT_OFF: u32 = 0x0103_99b8;
/// `"On"`
const TEXT_ON: u32 = 0x0103_99bc;
/// `"%s"`
const FORMAT_STRING: u32 = 0x0101_9f08;
/// `""`
const TEXT_EMPTY: u32 = 0x0101_1584;
/// `"AI Processing for actors in Low is  %s"`
const MSG_AI_LOW: u32 = 0x0103_a00c;
/// `"AI Processing for actors in Middle Low is  %s"`
const MSG_AI_MIDDLE_LOW: u32 = 0x0103_a034;
/// `"AI Processing for actors in Middle High is  %s"`
const MSG_AI_MIDDLE_HIGH: u32 = 0x0103_a064;
/// `"AI Processing of Actor's Editor Schedules is  %s"`
const MSG_AI_SCHEDULES: u32 = 0x0103_a094;
/// `"The NPC will speak the sound now."`
const MSG_NPC_SPEAKS: u32 = 0x0103_a0c8;
/// `"----INI SETTINGS--------------------"`
const HEADER_INI_SETTINGS: u32 = 0x0103_a0ec;
/// `"----GAME SETTINGS--------------------"`
const HEADER_GAME_SETTINGS: u32 = 0x0103_a114;
/// `"----SCRIPT FUNCTIONS--------------------"`
const HEADER_SCRIPT_FUNCTIONS: u32 = 0x0103_a13c;
/// `"----CONSOLE COMMANDS--------------------"`
const HEADER_CONSOLE_COMMANDS: u32 = 0x0103_a18c;
/// `"%s (%s)"`
const FORMAT_NAME_SHORT: u32 = 0x0103_a168;
/// `"%s -> %s"`
const FORMAT_NAME_HELP: u32 = 0x0103_a170;
/// `"%s (%s) -> %s"`
const FORMAT_NAME_SHORT_HELP: u32 = 0x0103_a17c;
/// `"%x"`
const FORMAT_HEX: u32 = 0x0103_a1b8;
/// `"Pathing"`
const TEXT_PATHING: u32 = 0x0103_a1bc;
/// `"\"%s\" (%08x)"`
const FORMAT_PICKED_REFERENCE: u32 = 0x0103_a1c8;
/// `"...Level-up Value: %.2f"`
const MSG_LEVEL_UP_VALUE: u32 = 0x0103_a1d4;
/// `"...Modifiers: Temp: %.2f Perm: %.2f Damage: %.2f"`
const MSG_MODIFIERS: u32 = 0x0103_a1ec;
/// `"......SetAV override: %.2f"`
const MSG_SET_AV_OVERRIDE: u32 = 0x0103_a220;
/// `"......Derived Value: %.2f %s"`
const MSG_DERIVED_VALUE: u32 = 0x0103_a23c;
/// `"(Ignored)"`
const TEXT_IGNORED: u32 = 0x0103_a25c;
/// `"......Reference Base Value: %.2f %s"`
const MSG_REFERENCE_BASE_VALUE: u32 = 0x0103_a268;
/// `"(creature)"`
const TEXT_CREATURE: u32 = 0x0103_a28c;
/// `"(auto-calculated)"`
const TEXT_AUTO_CALCULATED: u32 = 0x0103_a298;
/// `"...Base Value components:"`
const MSG_BASE_VALUE_COMPONENTS: u32 = 0x0103_a2ac;
/// `"...Current Value: %.2f Computed Base: %.2f "`
const MSG_CURRENT_VALUE: u32 = 0x0103_a2c8;
/// `"GetActorValueInfo: %s on %s"`
const MSG_ACTOR_VALUE_INFO: u32 = 0x0103_a2f4;
/// `"Actor Value '%s' cannot be modified in scripts or the console."`
const MSG_CANNOT_MODIFY: u32 = 0x0103_a310;
/// `-1.0f`
const FLOAT_MINUS_ONE: u32 = 0x0101_2054;
/// `640.0f`: the column of the picked reference's debug text.
const PICK_REF_COLUMN: u32 = 0x0103_a1c4;
/// `3.0` (`double`) added to the line's value before it is truncated.
const PICK_REF_ROUNDING: u32 = 0x0102_1928;

// ---- Small helpers -------------------------------------------------------------

/// `Script::ParseParameters` (`005accb0`) with the given output addresses
/// after the seven fixed words: its `AL`.
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
fn parse_into<const N: usize>(e: &mut Engine, a: ScriptArgs, init: [u32; N]) -> Option<[u32; N]> {
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

/// Whether the commands echo to the console (byte `+0x268` of the TLS block).
fn echo_enabled(e: &mut Engine) -> bool {
    let tls = e.tls();
    e.mem.u8(tls + TLS_ECHO) != 0
}

fn console_print(e: &mut Engine, words: &[u32]) {
    e.call(CONSOLE_PRINT, words);
}

/// `__RTDynamicCast` of a reference to `Actor`: the actor or 0.
fn cast_to_actor(e: &mut Engine, object: Ptr) -> u32 {
    e.call(
        DYNAMIC_CAST,
        &args![object, 0u32, RTTI_TES_OBJECT_REFR, RTTI_ACTOR, 0u32],
    )
    .u32()
}

/// Whether the actor value has the flag `mask` (`00406d70`).
fn actor_value_has_flag(e: &mut Engine, actor_value: u32, mask: u32) -> bool {
    e.call(ACTOR_VALUE_HAS_FLAG, &args![actor_value, mask])
        .bool()
}

/// The message of the commands that refuse to change a protected actor
/// value (only when commands echo).
fn print_cannot_modify(e: &mut Engine, actor_value: u32) {
    if echo_enabled(e) {
        let name = e.call(ACTOR_VALUE_SCRIPT_NAME, &args![actor_value]).u32();
        console_print(e, &args![MSG_CANNOT_MODIFY, name]);
    }
}

/// The value of a setting object: the word `00 43d4d0(setting)` points to.
fn setting_value(e: &mut Engine, setting: u32) -> i32 {
    let address = e.call(SETTING_VALUE_ADDRESS, &args![setting]).u32();
    e.mem.i32(address)
}

/// The `NavMeshRender` singleton.
fn nav_mesh_render(e: &mut Engine) -> u32 {
    e.call(NAV_MESH_RENDER, &args![]).u32()
}

fn nav_mesh_set_draw(e: &mut Engine, draw: bool) {
    let render = nav_mesh_render(e);
    e.call(NAV_MESH_RENDER_SET_DRAW, &args![render, draw]);
}

fn nav_mesh_set_transparent(e: &mut Engine, transparent: bool) {
    let render = nav_mesh_render(e);
    e.call(NAV_MESH_RENDER_SET_TRANSPARENT, &args![render, transparent]);
}

fn nav_mesh_add_cell(e: &mut Engine, cell: u32) {
    let render = nav_mesh_render(e);
    e.call(NAV_MESH_RENDER_ADD_CELL, &args![render, cell, 0u32]);
}

// ---- AI toggles ------------------------------------------------------------------

/// The body of the four AI toggles: flips the byte at `flag`, and when the
/// commands echo prints `message` with `"On"` or `"Off"`.
fn toggle_ai_flag(e: &mut Engine, flag: u32, message: u32) -> bool {
    let enabled = e.global::<u8>(flag) == 0;
    e.set_global(flag, enabled as u8);
    if echo_enabled(e) {
        let state = if enabled { TEXT_ON } else { TEXT_OFF };
        console_print(e, &args![message, state]);
    }
    true
}

// Translated from 005bc430 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Script::ToggleLowProcess` (Xbox PDB): flips the AI processing of
/// low-process actors and reports the new state.
pub fn script_toggle_low_process(e: &mut Engine) -> bool {
    toggle_ai_flag(e, AI_LOW_PROCESS, MSG_AI_LOW)
}

// Translated from 005bc4a0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Script::ToggleMiddleLowProcess` (Xbox PDB): the same for the middle-low
/// processes.
pub fn script_toggle_middle_low_process(e: &mut Engine) -> bool {
    toggle_ai_flag(e, AI_MIDDLE_LOW_PROCESS, MSG_AI_MIDDLE_LOW)
}

// Translated from 005bc510 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Script::ToggleMiddleHighProcess` (Xbox PDB): the same for the
/// middle-high processes.
pub fn script_toggle_middle_high_process(e: &mut Engine) -> bool {
    toggle_ai_flag(e, AI_MIDDLE_HIGH_PROCESS, MSG_AI_MIDDLE_HIGH)
}

// Translated from 005bc580 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Script::ToggleAISchedules` (Xbox PDB): the same for the actors' editor
/// schedules.
pub fn script_toggle_ai_schedules(e: &mut Engine) -> bool {
    toggle_ai_flag(e, AI_SCHEDULES, MSG_AI_SCHEDULES)
}

// ---- SpeakSound --------------------------------------------------------------------

// Translated from 005bc5f0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Script::SpeakSoundFunction` (Xbox PDB): parses a sound name (up to 512
/// characters), a number (0 to 7, default 0) and a volume (0 to 100, default
/// 50), and makes the actor the command runs on say the sound: through the
/// actor's virtual slot `0x284` with the name formatted into a `BSStringT`,
/// then the actor's process gets slot `0x310(1)` and slot `0x318(duration)`.
/// Only an actor with a saved acquire object in a process of type 0 takes
/// part; otherwise the command returns false. The 512-byte copy of the name
/// the game makes with `strcpy_s` is unused afterwards but kept, and the
/// check of the always non-null pointer to the name is left out.
pub fn script_speak_sound_function(e: &mut Engine, a: ScriptArgs) -> bool {
    // One block for the game's stack locals: the 12-byte result object, the
    // 8-byte string, the two numbers, the name buffer and its copy.
    let block = e.mem.alloc(0x440);
    let result_object = block;
    let string = block + 0x10;
    let numbers = block + 0x18;
    let name = block + 0x20;
    let name_copy = block + 0x228;
    e.call(SPEAK_RESULT_CONSTRUCT, &args![result_object]);
    e.call(BS_STRING_CONSTRUCT, &args![string]);
    let ok = speak_sound_body(e, a, string, result_object, numbers, name, name_copy);
    e.call(BS_STRING_DESTRUCT, &args![string]);
    e.call(SPEAK_RESULT_DESTRUCT, &args![result_object]);
    e.mem.free(block);
    ok
}

fn speak_sound_body(
    e: &mut Engine,
    a: ScriptArgs,
    string: u32,
    result_object: u32,
    numbers: u32,
    name: u32,
    name_copy: u32,
) -> bool {
    e.mem.set_u32(numbers, 0);
    e.mem.set_u32(numbers + 4, 0x32);
    if !parse(e, a, &[name, numbers, numbers + 4]) {
        return false;
    }
    e.call(STRING_COPY, &args![name_copy, 0x200u32, name]);
    if a.this_obj.is_null() {
        return false;
    }
    let actor = cast_to_actor(e, a.this_obj);
    if actor == 0 {
        return false;
    }
    if e.call(ACTOR_PROCESS, &args![actor]).u32() == 0 {
        return false;
    }
    if e.call(CURRENT_PROCESS_TYPE, &args![actor]).u32() != 0 {
        return false;
    }
    let mut variation = e.mem.i32(numbers);
    if !(0..8).contains(&variation) {
        variation = 0;
    }
    let volume = e.mem.i32(numbers + 4).clamp(0, 100);
    e.call(BS_STRING_FORMAT, &args![string, FORMAT_STRING, name]);
    let text = e.call(STRING_TEXT, &args![string]).u32();
    let duration = e
        .vcall(
            actor,
            0x284,
            &args![
                text,
                result_object,
                variation,
                volume,
                0u32,
                0u32,
                0u32,
                0u32,
                0u32,
                0u32,
                0u32,
                1u32,
                1u32
            ],
        )
        .f32();
    let process = e.call(ACTOR_PROCESS, &args![actor]).u32();
    e.vcall(process, 0x310, &args![1u32]);
    let process = e.call(ACTOR_PROCESS, &args![actor]).u32();
    e.vcall(process, 0x318, &args![duration]);
    if echo_enabled(e) {
        console_print(e, &args![MSG_NPC_SPEAKS]);
    }
    true
}

// ---- Help ------------------------------------------------------------------------------

/// One entry of the help lists: prints it when the filter is empty or occurs
/// (case-insensitive) in the help, name or short name.
fn print_help_entry(e: &mut Engine, entry: u32, filter: u32) {
    let name = e.mem.u32(entry);
    let short_name = e.mem.u32(entry + 4);
    let help = e.mem.u32(entry + 0xc);
    if e.mem.u8(filter) != 0
        && e.call(STRISTR, &args![help, filter]).u32() == 0
        && e.call(STRISTR, &args![name, filter]).u32() == 0
        && e.call(STRISTR, &args![short_name, filter]).u32() == 0
    {
        return;
    }
    let has_help = help != 0 && e.call(STRLEN, &args![help]).u32() != 0;
    if has_help {
        if short_name != 0 && e.call(STRLEN, &args![short_name]).u32() != 0 {
            console_print(e, &args![FORMAT_NAME_SHORT_HELP, name, short_name, help]);
        } else {
            console_print(e, &args![FORMAT_NAME_HELP, name, help]);
        }
    } else if short_name != 0 && e.call(STRLEN, &args![short_name]).u32() != 0 {
        console_print(e, &args![FORMAT_NAME_SHORT, name, short_name]);
    } else {
        console_print(e, &args![FORMAT_STRING, name]);
    }
}

/// Prints the names in the collection `list` (except the first) that contain
/// the filter, releasing each name after it is looked at.
fn print_matching_setting_names(e: &mut Engine, list: u32, filter: u32) {
    let mut index = 0u32;
    while index < e.call(COLLECTION_COUNT, &args![list]).u32() {
        let slot = e
            .call(COLLECTION_ELEMENT_ADDRESS, &args![list, index])
            .u32();
        let name = e.mem.u32(slot);
        if name != 0 {
            if index != 0 && e.call(STRISTR, &args![name, filter]).u32() != 0 {
                console_print(e, &args![FORMAT_STRING, name]);
            }
            e.call(MEMORY_FREE, &args![name]);
        }
        index += 1;
    }
}

// Translated from 005bc950 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Script::HelpFunction` (Xbox PDB): lists the console commands and the
/// script functions whose help, name or short name contains the optional
/// filter argument (everything without one); with a filter it also lists
/// the matching game settings and INI settings (the names the two settings
/// collections hand out, released one by one). The parse result is not
/// looked at.
pub fn script_help_function(e: &mut Engine, a: ScriptArgs) -> bool {
    let block = e.mem.alloc(TEXT_PARAMETER_SIZE + 4 + 0x20);
    let filter = block;
    e.mem.set_u8(filter, 0);
    // The second parameter is a word the command never reads back.
    parse(e, a, &[filter, block + TEXT_PARAMETER_SIZE]);
    console_print(e, &args![HEADER_CONSOLE_COMMANDS]);
    for index in 0..HELP_COMMAND_COUNT {
        print_help_entry(e, COMMAND_TABLE + index * HELP_ENTRY_SIZE, filter);
    }
    console_print(e, &args![HEADER_SCRIPT_FUNCTIONS]);
    for index in 0..HELP_FUNCTION_COUNT {
        print_help_entry(e, FUNCTION_TABLE + index * HELP_ENTRY_SIZE, filter);
    }
    if e.mem.u8(filter) != 0 {
        console_print(e, &args![HEADER_GAME_SETTINGS]);
        let game_names = block + TEXT_PARAMETER_SIZE + 4;
        e.call(COLLECTION_CONSTRUCT, &args![game_names, 0u32, 1u32]);
        let game_settings = e.call(GAME_SETTINGS, &args![]).u32();
        e.vcall(game_settings, 0xc, &args![game_names]);
        print_matching_setting_names(e, game_names, filter);
        console_print(e, &args![HEADER_INI_SETTINGS]);
        let ini_names = game_names + 0x10;
        e.call(COLLECTION_CONSTRUCT, &args![ini_names, 0u32, 1u32]);
        let ini_settings = e.call(INI_SETTINGS, &args![]).u32();
        e.vcall(ini_settings, 0xc, &args![ini_names]);
        print_matching_setting_names(e, ini_names, filter);
        e.call(COLLECTION_DESTRUCT, &args![ini_names]);
        e.call(COLLECTION_DESTRUCT, &args![game_names]);
    }
    e.mem.free(block);
    true
}

// ---- Navigation mesh ------------------------------------------------------------------

// Translated from 005bcfe0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Script::ToggleNavMeshFunction` (Xbox PDB): sets the navigation mesh
/// display mode from an optional argument (default 4, anything above 6 is
/// 4): 4 toggles drawing, 5 and 6 flip the two option bytes (5 turns
/// drawing off when it is on and selects the remembered mode, otherwise mode
/// 0; 6 turns it off and selects mode 3), 0 turns the display off, 2 selects
/// the transparent overlay and 1/3 the opaque one. When drawing was off and
/// the mode is not 0, the meshes of the current cell (interior) or of the
/// grid around it (exterior, `uGridsToLoad` cells wide) are added before
/// drawing is switched on.
pub fn script_toggle_nav_mesh_function(e: &mut Engine, a: ScriptArgs) -> bool {
    let Some([mut mode]) = parse_into(e, a, [4]) else {
        return false;
    };
    if mode > 6 {
        mode = 4;
    }
    let render = nav_mesh_render(e);
    let mut drawing = e.call(NAV_MESH_RENDER_IS_DRAWING, &args![render]).u8();
    if mode == 4 {
        mode = (drawing == 0) as u32;
    }
    if mode == 5 {
        let flag = e.global::<u8>(NAV_MESH_TOGGLE_5);
        e.set_global(NAV_MESH_TOGGLE_5, (flag == 0) as u8);
        if drawing != 0 {
            mode = e.global::<u32>(NAV_MESH_MODE);
            nav_mesh_set_draw(e, false);
            drawing = 0;
        } else {
            mode = 0;
        }
    }
    if mode == 6 {
        let flag = e.global::<u8>(NAV_MESH_TOGGLE_6);
        e.set_global(NAV_MESH_TOGGLE_6, (flag == 0) as u8);
        if drawing != 0 {
            nav_mesh_set_draw(e, false);
            drawing = 0;
        }
        mode = 3;
    }
    if mode == 0 {
        e.set_global(NAV_MESH_ENABLED, 0u8);
        nav_mesh_set_draw(e, false);
        return true;
    }
    e.set_global(NAV_MESH_ENABLED, 1u8);
    if mode == 2 {
        nav_mesh_set_transparent(e, true);
    } else if mode == 1 || mode == 3 {
        nav_mesh_set_transparent(e, false);
    }
    e.set_global(NAV_MESH_MODE, mode);
    if drawing == 0 {
        let tes = e.global::<u32>(TES_SINGLETON);
        let cell = e.call(GET_CURRENT_CELL, &args![tes]).u32();
        if cell != 0 {
            if e.call(CELL_IS_INTERIOR, &args![cell]).bool() {
                nav_mesh_add_cell(e, cell);
            } else {
                let half = (setting_value(e, GRIDS_TO_LOAD_SETTING) - 1) / 2;
                let mut x = e.call(CELL_GET_DATA_X, &args![cell]).i32() - half;
                while x <= e.call(CELL_GET_DATA_X, &args![cell]).i32() + half {
                    let mut y = e.call(CELL_GET_DATA_Y, &args![cell]).i32() - half;
                    while y <= e.call(CELL_GET_DATA_Y, &args![cell]).i32() + half {
                        let world_space = e.call(CELL_GET_WORLD_SPACE, &args![cell]).u32();
                        let neighbour = e
                            .call(WORLD_SPACE_GET_CELL, &args![world_space, x, y])
                            .u32();
                        nav_mesh_add_cell(e, neighbour);
                        y += 1;
                    }
                    x += 1;
                }
            }
        }
        nav_mesh_set_draw(e, true);
    }
    true
}

/// Parses a hexadecimal form id (text of up to 512 characters, read with
/// `sscanf("%x")`) and, when the script object has a file, converts it to
/// the id in that file's numbering (`TESForm::AddCompileIndex`). `None`
/// when the parameter does not parse, `Some(0)` when the text is not a
/// non-zero number.
fn parse_form_id(e: &mut Engine, a: ScriptArgs) -> Option<u32> {
    let text = e.mem.alloc(TEXT_PARAMETER_SIZE);
    let form_id = e.mem.alloc(4);
    let mut result = None;
    if parse(e, a, &[text]) {
        e.mem.set_u32(form_id, 0);
        e.call(SSCANF, &args![text, FORMAT_HEX, form_id]);
        if e.mem.u32(form_id) != 0
            && !a.script_obj.is_null()
            && e.call(FORM_GET_FILE, &args![a.script_obj, 0xffff_ffffu32])
                .u32()
                != 0
        {
            let file = e
                .call(FORM_GET_FILE, &args![a.script_obj, 0xffff_ffffu32])
                .u32();
            e.call(ADD_COMPILE_INDEX, &args![form_id, file]);
        }
        result = Some(e.mem.u32(form_id));
    }
    e.mem.free(form_id);
    e.mem.free(text);
    result
}

// Translated from 005bd240 (decompiled, FalloutNV.exe 1.4.0.525)
/// A navigation mesh command: parses a hexadecimal form id (made relative to
/// the script's file when it has one) and, when it is not zero, looks up the
/// obstacle manager's entry for it (`006c0720`) and calls `006c1150` on it.
pub fn fn_005bd240(e: &mut Engine, a: ScriptArgs) -> bool {
    let Some(form_id) = parse_form_id(e, a) else {
        return false;
    };
    if form_id != 0 {
        let entry = e.call(OBSTACLE_ENTRY, &args![form_id]).u32();
        e.call(OBSTACLE_ENTRY_ACTION_A, &args![entry]);
    }
    true
}

// Translated from 005bd300 (decompiled, FalloutNV.exe 1.4.0.525)
/// The sibling of [`fn_005bd240`] that calls `006c1130` instead of
/// `006c1150`.
pub fn fn_005bd300(e: &mut Engine, a: ScriptArgs) -> bool {
    let Some(form_id) = parse_form_id(e, a) else {
        return false;
    };
    if form_id != 0 {
        let entry = e.call(OBSTACLE_ENTRY, &args![form_id]).u32();
        e.call(OBSTACLE_ENTRY_ACTION_B, &args![entry]);
    }
    true
}

// Translated from 005bd3c0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Script::TogglePathingInfoFunction` (Xbox PDB): switching on makes sure
/// the debug text is visible and adds the `"Pathing"` page to the pathing
/// debug object; switching off only clears the flag.
pub fn script_toggle_pathing_info_function(e: &mut Engine) -> bool {
    if e.global::<u8>(PATHING_INFO_ON) == 0 {
        e.set_global(PATHING_INFO_ON, 1u8);
        if !e.call(IS_DEBUG_TEXT_VISIBLE, &args![]).bool() {
            e.call(TOGGLE_DEBUG_TEXT_VISIBLE, &args![]);
        }
        let debug_object = e.call(PATHING_DEBUG_OBJECT, &args![]).u32();
        e.call(PATHING_DEBUG_ADD, &args![debug_object, TEXT_PATHING, 0u32]);
    } else {
        e.set_global(PATHING_INFO_ON, 0u8);
    }
    true
}

// ---- PickRefByID --------------------------------------------------------------------------

// Translated from 005bd410 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Script::PickRefByIDFunction` (Xbox PDB): parses a reference; when its
/// form type is `0x3a` to `0x40` or `0x69` it prints `"<name>" (<form id>)`
/// as debug text at the row given by the debug line array
/// ([`fn_005bd5b0`], [`fn_005bd5c0`], [`fn_005bd580`]) and hands the
/// reference to `007031e0`.
pub fn script_pick_ref_by_id_function(e: &mut Engine, a: ScriptArgs) -> bool {
    let Some([reference]) = parse_into(e, a, [0]) else {
        return false;
    };
    if reference == 0 {
        return true;
    }
    let form_type = e.call(FORM_TYPE, &args![reference]).i32();
    if form_type < 0x3a || (form_type > 0x40 && form_type != 0x69) {
        return true;
    }
    let string = e.mem.alloc(8);
    e.call(BS_STRING_CONSTRUCT, &args![string]);
    let form_id = e.call(FORM_ID, &args![reference]).u32();
    let name = e.call(OBJECT_NAME, &args![reference]).u32();
    e.call(
        BS_STRING_FORMAT,
        &args![string, FORMAT_PICKED_REFERENCE, name, form_id],
    );
    let line = setting_value(e, PICK_REF_LINE_SETTING) - 1;
    let lines = fn_005bd5b0(e);
    let entry = fn_005bd5c0(e, lines, line as u32);
    let position = fn_005bd580(e, Ptr::new(entry));
    let rounding: f64 = e.global(PICK_REF_ROUNDING);
    let truncated = e.call(FTOL, &args![position as f64 + rounding]).i32();
    let row = (truncated - 5) as f32;
    let column: f32 = e.global(PICK_REF_COLUMN);
    let colour: f32 = e.global(FLOAT_MINUS_ONE);
    let text = e.call(STRING_TEXT, &args![string]).u32();
    let debug_text = e.call(DEBUG_TEXT_INSTANCE, &args![1u32]).u32();
    e.call(
        DEBUG_TEXT_PRINT,
        &args![
            debug_text,
            text,
            column,
            row,
            2u32,
            0xffff_ffffu32,
            colour,
            0u32,
            0u32
        ],
    );
    e.call(PICK_REFERENCE, &args![reference]);
    e.call(BS_STRING_DESTRUCT, &args![string]);
    e.mem.free(string);
    true
}

// Translated from 005bd580 (decompiled, FalloutNV.exe 1.4.0.525)
/// The `float` the object points to at `+0x38`, 0.0 when that pointer is
/// null (returned in `ST0`).
pub fn fn_005bd580(e: &mut Engine, this: Ptr) -> f32 {
    let value = e.mem.u32(this.addr() + 0x38);
    if value == 0 {
        0.0
    } else {
        e.mem.f32(value)
    }
}

// Translated from 005bd5b0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Returns the global `011f33f8` (the base of the debug line array).
pub fn fn_005bd5b0(e: &mut Engine) -> u32 {
    e.global::<u32>(DEBUG_LINE_ARRAY)
}

// Translated from 005bd5c0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Element `index` of the array of words at `this`.
pub fn fn_005bd5c0(e: &mut Engine, this: u32, index: u32) -> u32 {
    e.mem.u32(this.wrapping_add(index.wrapping_mul(4)))
}

// ---- CenterOn ---------------------------------------------------------------------------------

// Translated from 005bd5e0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Script::CenterOnCellFunction` (Xbox PDB): parses a cell name (up to 512
/// characters), closes the console and has the player centre on the cell.
pub fn script_center_on_cell_function(e: &mut Engine, a: ScriptArgs) -> bool {
    let name = e.mem.alloc(TEXT_PARAMETER_SIZE);
    let ok = parse(e, a, &[name]);
    if ok {
        e.call(CLOSE_CONSOLE, &args![]);
        let player = e.global::<u32>(PLAYER);
        e.call(CENTER_ON_CELL, &args![player, name, 0u32]);
    }
    e.mem.free(name);
    ok
}

/// The tail the two exterior commands share: loads (or creates) the cell at
/// `(x, y)` of `world_space` and has the player centre on it, then marks the
/// player (byte `+0x206`).
fn center_on_exterior_cell(e: &mut Engine, world_space: u32, x: u32, y: u32) {
    let loader = e.global::<u32>(EXTERIOR_CELL_LOADER);
    if loader != 0 {
        e.call(CANCEL_ALL_CELL_LOADS, &args![loader]);
    }
    let mut cell = e
        .call(WORLD_SPACE_GET_CELL, &args![world_space, x, y])
        .u32();
    if cell == 0 {
        cell = e
            .call(WORLD_SPACE_LOAD_CELL, &args![world_space, x, y])
            .u32();
    }
    if cell == 0 {
        let data_handler = e.global::<u32>(DATA_HANDLER);
        cell = e
            .call(
                DATA_HANDLER_NEW_CELL,
                &args![data_handler, 0u32, x, y, world_space],
            )
            .u32();
    }
    if cell != 0 {
        e.call(CLOSE_CONSOLE, &args![]);
        let player = e.global::<u32>(PLAYER);
        e.call(CENTER_ON_CELL, &args![player, 0u32, cell]);
    }
    let player = e.global::<u32>(PLAYER);
    e.mem.set_u8(player + 0x206, 1);
}

// Translated from 005bd660 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Script::CenterOnExteriorFunction` (Xbox PDB): parses the cell
/// coordinates `x`, `y`, takes the current world space (or the first one of
/// the data handler when there is none; false when there is no world space
/// at all) and centres the player on that cell.
pub fn script_center_on_exterior_function(e: &mut Engine, a: ScriptArgs) -> bool {
    let Some([x, y]) = parse_into(e, a, [0, 0]) else {
        return false;
    };
    let tes = e.global::<u32>(TES_SINGLETON);
    let mut world_space = e.call(TES_GET_WORLD_SPACE, &args![tes]).u32();
    if world_space == 0 {
        let data_handler = e.global::<u32>(DATA_HANDLER);
        let list = e
            .call(DATA_HANDLER_WORLD_SPACES, &args![data_handler])
            .u32();
        let item = e.call(LIST_ITEM_PTR, &args![list]).u32();
        world_space = e.mem.u32(item);
    }
    if world_space == 0 {
        return false;
    }
    center_on_exterior_cell(e, world_space, x, y);
    true
}

// Translated from 005bd780 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Script::CenterOnWorldFunction` (Xbox PDB): parses a world space and the
/// cell coordinates `x`, `y` and centres the player on that cell; false
/// when the world space is null.
pub fn script_center_on_world_function(e: &mut Engine, a: ScriptArgs) -> bool {
    let Some([world_space, x, y]) = parse_into(e, a, [0, 0, 0]) else {
        return false;
    };
    if world_space == 0 {
        return false;
    }
    center_on_exterior_cell(e, world_space, x, y);
    true
}

// ---- Condition function wrappers ---------------------------------------------------------------

// Translated from 005bd880 (decompiled, FalloutNV.exe 1.4.0.525)
/// Forwards to the condition function `0059c4c0(thisObj, 0, 0, result)` and
/// returns its result.
pub fn fn_005bd880(e: &mut Engine, a: ScriptArgs) -> bool {
    e.call(COND_FN_0059C4C0, &args![a.this_obj, 0u32, 0u32, a.result])
        .bool()
}

// Translated from 005bd8a0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Parses one argument and forwards to `0059c4f0(thisObj, argument, 0,
/// result)`.
pub fn fn_005bd8a0(e: &mut Engine, a: ScriptArgs) -> bool {
    let Some([argument]) = parse_into(e, a, [0]) else {
        return false;
    };
    e.call(
        COND_FN_0059C4F0,
        &args![a.this_obj, argument, 0u32, a.result],
    )
    .bool()
}

// Translated from 005bd900 (decompiled, FalloutNV.exe 1.4.0.525)
/// Parses one argument and forwards to `0059c680(thisObj, argument, 0,
/// result)`.
pub fn fn_005bd900(e: &mut Engine, a: ScriptArgs) -> bool {
    let Some([argument]) = parse_into(e, a, [0]) else {
        return false;
    };
    e.call(
        COND_FN_0059C680,
        &args![a.this_obj, argument, 0u32, a.result],
    )
    .bool()
}

// Translated from 005be6a0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Forwards to `Script::GetFatiguePercentageConditionFunction(thisObj, 0, 0,
/// result)` and returns its result.
pub fn fn_005be6a0(e: &mut Engine, a: ScriptArgs) -> bool {
    e.call(
        GET_FATIGUE_PERCENTAGE_CONDITION,
        &args![a.this_obj, 0u32, 0u32, a.result],
    )
    .bool()
}

// Translated from 005be6c0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Forwards to `Script::GetHealthPercentageConditionFunction(thisObj, 0, 0,
/// result)` and returns its result.
pub fn fn_005be6c0(e: &mut Engine, a: ScriptArgs) -> bool {
    e.call(
        GET_HEALTH_PERCENTAGE_CONDITION,
        &args![a.this_obj, 0u32, 0u32, a.result],
    )
    .bool()
}

// Translated from 005be6e0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Script::GetBaseActorValueFunction` (Xbox PDB): parses an actor value and
/// returns `Script::GetBaseActorValueConditionFunction(thisObj, actor value,
/// 0, result)`.
pub fn script_get_base_actor_value_function(e: &mut Engine, a: ScriptArgs) -> bool {
    let Some([actor_value]) = parse_into(e, a, [0]) else {
        return false;
    };
    e.call(
        GET_BASE_ACTOR_VALUE_CONDITION,
        &args![a.this_obj, actor_value, 0u32, a.result],
    )
    .bool()
}

// Translated from 005be740 (decompiled, FalloutNV.exe 1.4.0.525)
/// Forwards to `Script::GetCauseofDeathConditionFunction(thisObj, 0, 0,
/// result)` and returns its result.
pub fn fn_005be740(e: &mut Engine, a: ScriptArgs) -> bool {
    e.call(
        GET_CAUSE_OF_DEATH_CONDITION,
        &args![a.this_obj, 0u32, 0u32, a.result],
    )
    .bool()
}

// Translated from 005be760 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Script::IsLimbGoneFunction` (Xbox PDB): parses a limb (default 0) and an
/// optional last limb (default -1000). Without a last limb above 0 it
/// returns `Script::IsLimbGoneConditionFunction(thisObj, limb, 0, result)`.
/// Otherwise, for an actor (virtual slot `0x100`), it checks the limbs from
/// `limb` to the last one and writes 1.0 to the result at the first one that
/// is dismembered.
pub fn script_is_limb_gone_function(e: &mut Engine, a: ScriptArgs) -> bool {
    let Some([limb, last_limb]) = parse_into(e, a, [0, -1000i32 as u32]) else {
        return false;
    };
    let last_limb = last_limb as i32;
    if last_limb <= 0 {
        return e
            .call(
                IS_LIMB_GONE_CONDITION,
                &args![a.this_obj, limb, 0u32, a.result],
            )
            .bool();
    }
    if !a.this_obj.is_null() && e.vcall(a.this_obj.addr(), 0x100, &args![]).bool() {
        let mut index = limb as i32;
        while index <= last_limb {
            if e.call(REFERENCE_IS_LIMB_DISMEMBERED, &args![a.this_obj, index])
                .bool()
            {
                e.mem.set_f64(a.result.addr(), 1.0);
                return true;
            }
            index += 1;
        }
    }
    true
}

// Translated from 005be830 (decompiled, FalloutNV.exe 1.4.0.525)
/// Forwards to `Script::GetKillingBlowLimbConditionFunction(thisObj, 0, 0,
/// result)` and returns its result.
pub fn fn_005be830(e: &mut Engine, a: ScriptArgs) -> bool {
    e.call(
        GET_KILLING_BLOW_LIMB_CONDITION,
        &args![a.this_obj, 0u32, 0u32, a.result],
    )
    .bool()
}

// Translated from 005be850 (decompiled, FalloutNV.exe 1.4.0.525)
/// Forwards to the condition function `005a3e90(thisObj, 0, 0, result)` and
/// returns its result.
pub fn fn_005be850(e: &mut Engine, a: ScriptArgs) -> bool {
    e.call(COND_FN_005A3E90, &args![a.this_obj, 0u32, 0u32, a.result])
        .bool()
}

// ---- GetActorValueInfo -----------------------------------------------------------------------------

// Translated from 005bd960 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Script::GetActorValueInfoFunction` (Xbox PDB): parses an actor value and,
/// when commands echo and the command runs on an actor (virtual slot
/// `0x100`) with an actor value below `0x4d`, prints how its value is made
/// up: current and computed base value, reference base value (marked
/// auto-calculated or creature), the derived value, a SetAV override, the
/// modifiers and, for the player, the level-up value. True whenever the
/// parameters parse.
pub fn script_get_actor_value_info_function(e: &mut Engine, a: ScriptArgs) -> bool {
    let Some([actor_value]) = parse_into(e, a, [0]) else {
        return false;
    };
    if !echo_enabled(e)
        || a.this_obj.is_null()
        || !e.vcall(a.this_obj.addr(), 0x100, &args![]).bool()
        || !(0..0x4d).contains(&(actor_value as i32))
    {
        return true;
    }
    let object = a.this_obj.addr();
    // The `ActorValueOwner` part of the actor is at +0xa4.
    let owner = object + 0xa4;
    let computed_base = e.vcall(owner, 0x4, &args![actor_value]).f32();
    let name = e.call(OBJECT_NAME, &args![object]).u32();
    let script_name = e.call(ACTOR_VALUE_SCRIPT_NAME, &args![actor_value]).u32();
    console_print(e, &args![MSG_ACTOR_VALUE_INFO, script_name, name]);
    let current = e.vcall(owner, 0xc, &args![actor_value]).f64();
    console_print(e, &args![MSG_CURRENT_VALUE, current, computed_base as f64]);
    let base_owner = e.call(REFERENCE_BASE_OWNER, &args![object]).u32() + 0x100;
    let reference_base = e.vcall(base_owner, 0xc, &args![actor_value]).f32();
    // A float and a byte the callees fill in.
    let cells = e.mem.alloc(8);
    let derived_cell = cells;
    let override_cell = cells + 4;
    e.mem.set_f32(derived_cell, 0.0);
    e.mem.set_u8(override_cell, 0);
    // `this_obj` is not null here, so the owner address is never replaced by 0.
    let has_derived = e
        .call(
            ACTOR_VALUE_DERIVED,
            &args![owner, actor_value, derived_cell],
        )
        .bool();
    let override_value = e
        .vcall(object, 0x48c, &args![actor_value, override_cell])
        .f32();
    console_print(e, &args![MSG_BASE_VALUE_COMPONENTS]);
    let tag = if e.call(ACTOR_IS_AUTO_CALCULATED, &args![object]).bool() {
        TEXT_AUTO_CALCULATED
    } else if e.vcall(object, 0x21c, &args![]).bool() {
        TEXT_CREATURE
    } else {
        TEXT_EMPTY
    };
    console_print(
        e,
        &args![MSG_REFERENCE_BASE_VALUE, reference_base as f64, tag],
    );
    if has_derived {
        let is_player = e.vcall(object, 0x360, &args![]).bool();
        let mut ignored = !is_player
            && !e.call(ACTOR_IS_AUTO_CALCULATED, &args![object]).bool()
            && actor_value_has_flag(e, actor_value, 0x80);
        if !ignored {
            ignored = e.vcall(object, 0x21c, &args![]).bool();
        }
        let tag = if ignored { TEXT_IGNORED } else { TEXT_EMPTY };
        let derived = e.mem.f32(derived_cell);
        console_print(e, &args![MSG_DERIVED_VALUE, derived as f64, tag]);
    }
    if e.mem.u8(override_cell) != 0 {
        console_print(e, &args![MSG_SET_AV_OVERRIDE, override_value as f64]);
    }
    let damage = e.vcall(owner, 0x14, &args![actor_value]).f64();
    let permanent = e.vcall(owner, 0x18, &args![actor_value]).f64();
    let temporary = e.vcall(owner, 0x10, &args![actor_value]).f64();
    console_print(e, &args![MSG_MODIFIERS, temporary, permanent, damage]);
    if e.vcall(object, 0x360, &args![]).bool() {
        let player = e.global::<u32>(PLAYER);
        let level_up = e.vcall(player + 0xa4, 0x20, &args![actor_value]).f64();
        console_print(e, &args![MSG_LEVEL_UP_VALUE, level_up]);
    }
    e.mem.free(cells);
    true
}

// ---- Actor value commands --------------------------------------------------------------------------

// Translated from 005bdcd0 (decompiled, FalloutNV.exe 1.4.0.525)
/// An actor value command: parses an actor value and a number; for an actor
/// the pair is passed to the actor's virtual slot `0x398(actor value,
/// number)` unless the actor value is protected (flag `0x4000`), in which
/// case the "cannot be modified" message is printed (when commands echo).
pub fn fn_005bdcd0(e: &mut Engine, a: ScriptArgs) -> bool {
    let Some([actor_value, number]) = parse_into(e, a, [0, 0]) else {
        return false;
    };
    let actor = cast_to_actor(e, a.this_obj);
    if actor != 0 {
        if !actor_value_has_flag(e, actor_value, 0x4000) {
            e.vcall(actor, 0x398, &args![actor_value, number]);
        } else {
            print_cannot_modify(e, actor_value);
        }
    }
    true
}

// Translated from 005bddb0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Script::SetSizeFunction` (Xbox PDB): parses a size (default 0.0) and,
/// for an actor with a character controller, sets the controller's size.
pub fn script_set_size_function(e: &mut Engine, a: ScriptArgs) -> bool {
    let Some([size]) = parse_into(e, a, [0.0f32.to_bits()]) else {
        return false;
    };
    let actor = cast_to_actor(e, a.this_obj);
    if actor != 0 {
        let controller = e.call(GET_CHAR_CONTROLLER, &args![actor]).u32();
        if controller != 0 {
            e.call(
                CHAR_CONTROLLER_SET_SIZE,
                &args![controller, f32::from_bits(size)],
            );
        }
    }
    true
}

// Translated from 005bde40 (decompiled, FalloutNV.exe 1.4.0.525)
/// The sibling of [`fn_005bdcd0`] that calls the actor's virtual slot
/// `0x3a8(actor value, number, 0)`.
pub fn fn_005bde40(e: &mut Engine, a: ScriptArgs) -> bool {
    let Some([actor_value, number]) = parse_into(e, a, [0, 0]) else {
        return false;
    };
    let actor = cast_to_actor(e, a.this_obj);
    if actor != 0 {
        if !actor_value_has_flag(e, actor_value, 0x4000) {
            e.vcall(actor, 0x3a8, &args![actor_value, number, 0u32]);
        } else {
            print_cannot_modify(e, actor_value);
        }
    }
    true
}

// Translated from 005bdf20 (decompiled, FalloutNV.exe 1.4.0.525)
/// An actor value command that lowers a value: parses an actor value and an
/// amount (float); for an actor and an actor value that is not protected
/// (flag `0x4000`) it calls the actor's virtual slot `0x3ac(actor value,
/// change, 0)`, where the change is the amount negated for actor values
/// with flag `0x200` and `-|amount|` (`00408840`) otherwise. Actor value
/// `0x10` on an actor that answers true to slot `0x1a0(0)` goes to slot
/// `0x4b8(0, -amount)` instead.
pub fn fn_005bdf20(e: &mut Engine, a: ScriptArgs) -> bool {
    let Some([actor_value, amount]) = parse_into(e, a, [0, 0.0f32.to_bits()]) else {
        return false;
    };
    let amount = f32::from_bits(amount);
    let actor = cast_to_actor(e, a.this_obj);
    if actor != 0 {
        if actor_value_has_flag(e, actor_value, 0x4000) {
            print_cannot_modify(e, actor_value);
        } else if actor_value == 0x10 && e.vcall(actor, 0x1a0, &args![0u32]).bool() {
            e.vcall(actor, 0x4b8, &args![0u32, -amount]);
        } else {
            let mut change = amount;
            if !actor_value_has_flag(e, actor_value, 0x200) {
                change = (-e.call(ABSOLUTE_VALUE, &args![amount]).f64()) as f32;
            }
            e.vcall(actor, 0x3ac, &args![actor_value, change, 0u32]);
        }
    }
    true
}

// Translated from 005be080 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Script::RestoreActorValueFunction` (Xbox PDB): parses an actor value and
/// an amount (float); for an actor and an actor value that is not protected
/// it calls `Actor::RestoreActorValue(actor value, change)` where the change
/// is the amount negated for actor values with flag `0x200` and `|amount|`
/// (`00408840`) otherwise.
pub fn script_restore_actor_value_function(e: &mut Engine, a: ScriptArgs) -> bool {
    let Some([actor_value, amount]) = parse_into(e, a, [0, 0.0f32.to_bits()]) else {
        return false;
    };
    let amount = f32::from_bits(amount);
    let actor = cast_to_actor(e, a.this_obj);
    if actor != 0 {
        if actor_value_has_flag(e, actor_value, 0x4000) {
            print_cannot_modify(e, actor_value);
        } else {
            let change = if actor_value_has_flag(e, actor_value, 0x200) {
                -amount
            } else {
                e.call(ABSOLUTE_VALUE, &args![amount]).f64() as f32
            };
            e.call(
                ACTOR_RESTORE_ACTOR_VALUE,
                &args![actor, actor_value, change],
            );
        }
    }
    true
}

// Translated from 005be190 (decompiled, FalloutNV.exe 1.4.0.525)
/// An actor value command that sets a value: parses an actor value and an
/// integer; for an actor and an actor value that is not protected it calls
/// the actor's virtual slot `0x3a4(actor value, number - current value, 0)`
/// where the current value comes from the `ActorValueOwner` part (+0xa4)
/// slot `0xc`.
pub fn fn_005be190(e: &mut Engine, a: ScriptArgs) -> bool {
    let Some([actor_value, number]) = parse_into(e, a, [0, 0]) else {
        return false;
    };
    let actor = cast_to_actor(e, a.this_obj);
    if actor != 0 {
        if actor_value_has_flag(e, actor_value, 0x4000) {
            print_cannot_modify(e, actor_value);
        } else {
            let wanted = number as i32 as f64;
            let current = e.vcall(actor + 0xa4, 0xc, &args![actor_value]).f64();
            let change = (wanted - current) as f32;
            e.vcall(actor, 0x3a4, &args![actor_value, change, 0u32]);
        }
    }
    true
}

// ---- Kill ----------------------------------------------------------------------------------------------

// Translated from 005be2a0 (decompiled, FalloutNV.exe 1.4.0.525)
/// The kill command: parses the killer (a reference) and two numbers `part`
/// and `detail` (both default -1; the result of the parse is not looked at).
/// For an actor that is not already dead (slot `0x10` of its `+0x94`
/// part):
/// - `part` is dropped when the actor is the player and virtual slot
///   `0x1a0(0)` says so;
/// - when the player is the killer the experience for the actor is given to
///   the player (from the actor's level, creature flag and the player's
///   `+0x7b8` value, rounded, through the player's slot `0x488`);
/// - a life state of 6 is reset to 0;
/// - without a task queue (`008c7aa0` false) the actor is killed directly
///   (`Actor::Kill`) and dismembered when `part` is set; with one, the kill
///   and the dismember data are queued (`TaskQueueInterface::QueueActorKill`)
///   and the killer is stored in the actor (`+0xc0`).
pub fn fn_005be2a0(e: &mut Engine, a: ScriptArgs) -> bool {
    let locals = e.mem.alloc(12);
    e.mem.set_u32(locals, 0);
    e.mem.set_i32(locals + 4, -1);
    e.mem.set_i32(locals + 8, -1);
    parse(e, a, &[locals, locals + 4, locals + 8]);
    let killer = e.mem.u32(locals);
    let mut part = e.mem.i32(locals + 4);
    let detail = e.mem.i32(locals + 8);
    e.mem.free(locals);
    let actor = cast_to_actor(e, a.this_obj);
    if actor == 0 || e.vcall(actor + 0x94, 0x10, &args![]).bool() {
        return true;
    }
    if actor == e.global::<u32>(PLAYER) && e.vcall(actor, 0x1a0, &args![0u32]).bool() {
        part = -1;
    }
    let queued = e.call(KILLS_ARE_QUEUED, &args![]).u8();
    if killer == e.global::<u32>(PLAYER) {
        let level = e.call(ACTOR_LEVEL, &args![actor]).u16() as u32;
        let creature = e.vcall(actor, 0x21c, &args![]).bool();
        let points = e
            .call(GET_EXPERIENCE_POINTS, &args![!creature, level])
            .i32();
        let player = e.global::<u32>(PLAYER);
        let formula_input = fn_005be4d0(e, Ptr::new(player));
        let scaled = e
            .call(GAMEPLAY_FORMULA, &args![points as f32, formula_input])
            .f32();
        let rounded = e.call(ROUND_EXPERIENCE, &args![scaled]).f64();
        let experience = e.call(FTOL, &args![rounded]).u32();
        let player = e.global::<u32>(PLAYER);
        e.vcall(player, 0x488, &args![experience]);
    }
    if e.call(ACTOR_LIFE_STATE, &args![actor]).u32() == 6 {
        e.call(ACTOR_SET_LIFE_STATE, &args![actor, 0u32]);
    }
    if queued == 0 {
        e.call(ACTOR_KILL, &args![actor, killer, 0.0f32]);
        if part != -1 {
            e.call(ACTOR_SCRIPT_DISMEMBER, &args![actor, detail, part, killer]);
        }
    } else {
        let mut dismember = 0u32;
        if part != -1 {
            dismember = e.call(MEMORY_ALLOC, &args![0xcu32]).u32();
            e.mem.set_i32(dismember + 4, detail);
            e.mem.set_i32(dismember, part);
            e.mem.set_u32(dismember + 8, killer);
        }
        let damage = e.call(MEMORY_ALLOC, &args![4u32]).u32();
        e.mem.set_f32(damage, 0.0);
        e.mem.set_u32(actor + 0xc0, killer);
        let queue = e.call(TASK_QUEUE_INTERFACE, &args![]).u32();
        e.call(QUEUE_ACTOR_KILL, &args![queue, actor, damage, dismember]);
    }
    true
}

// ---- Process lists and misc ------------------------------------------------------------------------

// Translated from 005be4d0 (decompiled, FalloutNV.exe 1.4.0.525)
/// The word at `this + 0x7b8`.
pub fn fn_005be4d0(e: &mut Engine, this: Ptr) -> u32 {
    e.mem.u32(this.addr() + 0x7b8)
}

// Translated from 005be4f0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Parses a killer reference and kills every actor in the process lists'
/// array (`Actor::Kill(killer, 0.0)` for each entry that answers true to
/// virtual slot `0x100`). The count is read again before every entry.
pub fn fn_005be4f0(e: &mut Engine, a: ScriptArgs) -> bool {
    let Some([killer]) = parse_into(e, a, [0]) else {
        return false;
    };
    let mut index = 0u32;
    loop {
        let array = e.call(PROCESS_LISTS_ARRAY, &args![PROCESS_LISTS]).u32();
        if index >= fn_005be5c0(e, array, 0) {
            break;
        }
        let array = e.call(PROCESS_LISTS_ARRAY, &args![PROCESS_LISTS]).u32();
        let actor = e.call(PROCESS_LISTS_ACTOR, &args![array, index]).u32();
        if actor != 0 && e.vcall(actor, 0x100, &args![]).bool() {
            e.call(ACTOR_KILL, &args![actor, killer, 0.0f32]);
        }
        index += 1;
    }
    true
}

// Translated from 005be5c0 (decompiled, FalloutNV.exe 1.4.0.525)
/// The word at `this + 0x20 + index * 4`.
pub fn fn_005be5c0(e: &mut Engine, this: u32, index: u32) -> u32 {
    e.mem
        .u32(this.wrapping_add(0x20).wrapping_add(index.wrapping_mul(4)))
}

// Translated from 005be5e0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Parses a flag (default 0); for an actor (cast before the parse) it calls
/// virtual slot `0x324(1, has, flag == 1)` between the two calls on the
/// global `011f11a0` (`0040fbf0(0)` and `0040fba0`), where `has` says
/// whether `0043fcd0(actor)` is non-zero.
pub fn fn_005be5e0(e: &mut Engine, a: ScriptArgs) -> bool {
    let actor = cast_to_actor(e, a.this_obj);
    let Some([flag]) = parse_into(e, a, [0]) else {
        return false;
    };
    if actor != 0 {
        e.call(SCRIPT_RUNNER_ENTER, &args![SCRIPT_RUNNER, 0u32]);
        let has = e.call(ACTOR_FN_0043FCD0, &args![actor]).u32() != 0;
        e.vcall(actor, 0x324, &args![1u32, has, flag == 1]);
        e.call(SCRIPT_RUNNER_LEAVE, &args![SCRIPT_RUNNER]);
    }
    true
}

// Translated from 005be870 (decompiled, FalloutNV.exe 1.4.0.525)
/// For an actor (virtual slot `0x100`): reads its value from slot
/// `0x344(player, 0)` and, when it is below the setting `011d0844`, calls
/// slot `0x460(player, setting - value)` (the difference as a float).
pub fn fn_005be870(e: &mut Engine, a: ScriptArgs) -> bool {
    if a.this_obj.is_null() || !e.vcall(a.this_obj.addr(), 0x100, &args![]).bool() {
        return true;
    }
    let object = a.this_obj.addr();
    let player = e.global::<u32>(PLAYER);
    let value = e.vcall(object, 0x344, &args![player, 0u32]).i32();
    if value < setting_value(e, SETTING_011D0844) {
        let limit = setting_value(e, SETTING_011D0844);
        let player = e.global::<u32>(PLAYER);
        e.vcall(object, 0x460, &args![player, (limit - value) as f32]);
    }
    true
}

// ==== Second batch: `005be900` up to `005c0bf0` ===================================
//
// Constants first (callees outside this part by exe address, globals,
// strings), then the helpers, then the bodies in address order.

// ---- Settings (game settings and INI settings) -------------------------------
/// `thiscall(name)` on the game setting collection ([`GAME_SETTINGS`]): the
/// setting of that name, or 0.
const GAME_SETTING_FIND: u32 = 0x004f_8a30;
/// `thiscall(name)` on an INI setting collection (`fallout shared/
/// tesscriptfunctions.cpp`): the setting of that name, or 0.
const INI_SETTING_FIND: u32 = 0x005e_02b0;
/// Getter of the second INI setting collection (loads global `011f35a0`);
/// takes no argument.
const INI_SETTINGS_FALLBACK: u32 = 0x004d_e490;
/// `thiscall` on a setting: its type code, derived from the first letter of
/// its name (`00c33090`). The commands handle 3, 5 and 6 (and 0 for INI
/// settings).
const SETTING_TYPE: u32 = 0x004f_8940;
/// `thiscall` on a setting: its name, the word at `this + 8`.
const SETTING_NAME: u32 = 0x0044_ddc0;
/// `thiscall`: the word at `this + 8` (the same body as [`SETTING_NAME`]).
const WORD_AT_0X8: u32 = 0x0044_ddc0;
/// `thiscall` on a setting: the address of its float value (`this + 4`, or
/// a static zero for a null `this`).
const SETTING_FLOAT_ADDRESS: u32 = 0x0040_3e20;
/// `thiscall` on a setting: the address of its byte value (`this + 4`, or a
/// static zero for a null `this`).
const SETTING_BOOL_ADDRESS: u32 = 0x0040_8d60;
/// `thiscall` on a setting: its string value, `*(this + 4)` (0 for a null
/// `this`).
const SETTING_STRING_VALUE: u32 = 0x0040_3df0;
/// `thiscall(int)` on a setting: sets its integer value.
const SETTING_SET_INT: u32 = 0x0045_ce80;
/// `thiscall(float)` on a setting: sets its float value.
const SETTING_SET_FLOAT: u32 = 0x004e_d780;
/// `thiscall(text)` on a setting: sets its string value.
const SETTING_SET_STRING: u32 = 0x005e_0190;
/// CRT `atol(text)`.
const ATOL: u32 = 0x00ec_a6d3;
/// CRT `atof(text)`: the value in `ST0`.
const ATOF: u32 = 0x00ec_a573;
/// `"GameSetting %s >> NOT FOUND"`
const MSG_GAME_NOT_FOUND: u32 = 0x0103_a350;
/// `"GameSetting %s >> UNKNOWN TYPE"`
const MSG_GAME_UNKNOWN: u32 = 0x0103_a36c;
/// `"GameSetting %s >> '%s'"`
const MSG_GAME_STRING: u32 = 0x0103_a38c;
/// `"GameSetting %s >> %.2f"`
const MSG_GAME_FLOAT: u32 = 0x0103_a3a4;
/// `"GameSetting %s >> %i"`
const MSG_GAME_INT: u32 = 0x0103_a3bc;
/// `"INISetting %s >> NOT FOUND"`
const MSG_INI_NOT_FOUND: u32 = 0x0103_a3d4;
/// `"INISetting %s >> UNKNOWN TYPE"`
const MSG_INI_UNKNOWN: u32 = 0x0103_a3f0;
/// `"INISetting %s >> '%s'"`
const MSG_INI_STRING: u32 = 0x0103_a410;
/// `"INISetting %s >> %.2f"`
const MSG_INI_FLOAT: u32 = 0x0103_a428;
/// `"INISetting %s >> %i"`
const MSG_INI_INT: u32 = 0x0103_a440;

// ---- Render options command and its helpers ----------------------------------
/// `_strnicmp(first, second, count)` (`cdecl`): 0 when equal.
const STRING_COMPARE_N_NO_CASE: u32 = 0x00ec_7ec0;
/// `cdecl() -> u32`: the render flags word `011f91d8`.
const RENDER_FLAGS_GET: u32 = 0x004b_c3e0;
/// `cdecl(flags)`: stores the render flags word `011f91d8`.
const RENDER_FLAGS_SET: u32 = 0x004b_c3d0;
/// `cdecl(selector) -> int` (`fallout shared/renderer.cpp`): the word
/// `011f91bc` for a non-zero selector, `011f91c0` for 0.
const RENDER_SETTING: u32 = 0x004d_c060;
/// `cdecl(index)` (`BSShader/bsshadermanager.cpp`): selects the shader table
/// entries of that index.
const SHADER_SET_MODE: u32 = 0x00b4_f450;
/// `cdecl()` (`BSShader/bsshadermanager.cpp`): runs a virtual call (slot
/// `0x120`) on each shader of the table at `011f9548`.
const SHADER_REFRESH: u32 = 0x00b5_57d0;
/// `cdecl(a, b)`: the smaller of two signed numbers.
const MIN_I32: u32 = 0x004a_8f20;
/// `cdecl(a, b)`: the larger of two signed numbers.
const MAX_I32: u32 = 0x0064_7b70;
/// `thiscall`, no argument (`fallout shared`, 4812 bytes), called on the
/// word at `+8` of the `TES` singleton by the `psh` option.
const PSH_ENABLE: u32 = 0x004b_af10;
/// `cdecl(value)` (`fallout shared/tes.cpp`): calls `0066b0d0` on the object
/// `011f95e8` with the value.
const PSH_DISABLE: u32 = 0x0045_1570;
/// `thiscall(byte)` on the object [`RENDERER_OBJECT`] (`fallout shared/
/// renderer.cpp`).
const RENDERER_SET_BYTE: u32 = 0x004d_e2d0;
/// The object `005bf840` works on.
const RENDERER_OBJECT: u32 = 0x011c_75a4;
/// `cdecl(index) -> list`: the pointer stored at `011f91c8 + index * 4`
/// (`fallout shared/tes.cpp`).
const SHADOW_LIST_GET: u32 = 0x0045_0b80;
/// `thiscall` on that list (`BSShader/shadowscenenode.cpp`): its first
/// usable node, or 0.
const SHADOW_LIST_FIRST: u32 = 0x00b5_afc0;
/// `thiscall` on that list: the next usable node, or 0.
const SHADOW_LIST_NEXT: u32 = 0x00b5_b010;
/// `thiscall` on a node: its byte at `+0x124` (the field `005bf800` sets;
/// the engine map names the body `MiddleHighProcess::IsCurrentWeaponGrenade`).
const NODE_FLAG: u32 = 0x008d_8220;
/// The render flags word (read by `004bc3e0`, written by `004bc3d0`).
const RENDER_FLAGS: u32 = 0x011f_91d8;
/// The word `005bf7f0` sets.
const GLOBAL_011F91B4: u32 = 0x011f_91b4;
/// The word the `sh` option sets (a number from 0 to 3).
const GLOBAL_011F91B0: u32 = 0x011f_91b0;
/// The byte `005bf8b0` reads and `005bf8c0` writes.
const GLOBAL_011F91DC: u32 = 0x011f_91dc;
/// The byte `005bf860` toggles.
const GLOBAL_0118C000: u32 = 0x0118_c000;
/// The byte the `sc` option toggles.
const GLOBAL_011F9441: u32 = 0x011f_9441;
/// The byte the `opt` option toggles.
const GLOBAL_011AD80D: u32 = 0x011a_d80d;
/// The word the `t<number>` option sets.
const GLOBAL_011AD828: u32 = 0x011a_d828;
/// Option names `_strnicmp` compares with: `"tex"`, `"degrade"`, `"sh"`,
/// `"sc"`, `"psh"`, `"dsh"`, `"opt"`, `"alpha"`.
const OPTION_TEX: u32 = 0x0103_a478;
const OPTION_DEGRADE: u32 = 0x0103_a470;
const OPTION_SH: u32 = 0x0103_a46c;
const OPTION_SC: u32 = 0x0103_a468;
const OPTION_PSH: u32 = 0x0103_a464;
const OPTION_DSH: u32 = 0x0103_a460;
const OPTION_OPT: u32 = 0x0103_a45c;
const OPTION_ALPHA: u32 = 0x0103_a454;

// ---- Camera, tree and decal commands --------------------------------------------
/// `cdecl()`: the word stored at `011deb7c` (an object holding the camera).
const CAMERA_HOLDER: u32 = 0x0045_c670;
/// `thiscall`: the word at `this + 0xac` (the camera of the holder; the
/// engine map names the body `BSFaceGenNiNode::GetAnimationData`).
const WORD_AT_0XAC: u32 = 0x0066_29f0;
/// `thiscall`: the word at `this + 0x68` (the engine map names the body
/// `MiddleHighProcess::GetSavedAcquireObject`).
const WORD_AT_0X68: u32 = 0x008d_8520;
/// `thiscall`: the word at `this + 0x20` (the engine map names the body
/// `BGSSaveFormBuffer::GetForm`).
const WORD_AT_0X20: u32 = 0x007a_f430;
/// `thiscall`: the word at `this + 0xc` (`fallout shared/modelloader.cpp`).
const WORD_AT_0XC: u32 = 0x0043_b230;
/// `thiscall`: `this + 0xdc`, the camera's view frustum (7 words).
const CAMERA_FRUSTUM: u32 = 0x0045_bbe0;
/// `thiscall(float)`: stores the float at `this + 0xfc` (the camera's
/// far-to-near ratio).
const CAMERA_SET_RATIO: u32 = 0x0050_7700;
/// `NiCamera::SetViewFrustum` (Xbox PDB), `thiscall(frustum)`.
const CAMERA_SET_VIEW_FRUSTUM: u32 = 0x00a6_faf0;
/// `thiscall` (`fallout shared/modelloader.cpp`): the word at `this + 0xbc`.
const WORD_AT_0XBC: u32 = 0x0043_fad0;
/// `thiscall`: `this + 0x8c`, the camera's position (3 floats).
const CAMERA_POSITION: u32 = 0x0045_bb80;
/// `thiscall(out)`: writes the camera's direction (3 floats) into `out` and
/// returns `out`.
const CAMERA_DIRECTION: u32 = 0x0045_bba0;
/// `thiscall(out)` on a vector: writes the negated vector into `out` and
/// returns it.
const VECTOR_NEGATE: u32 = 0x004a_0bd0;
/// `thiscall` (`fallout shared/bgsdecalmanager.cpp`) on the 0x74-byte request
/// `005bfe90` fills in: its constructor.
const DECAL_REQUEST_CONSTRUCT: u32 = 0x004a_37b0;
/// `thiscall(request, mode, flag)` (`004a3fe0`) on the world object.
const DECAL_REQUEST_SUBMIT: u32 = 0x004a_3fe0;
/// `cdecl() -> float` in `ST0`.
const DECAL_FLOAT: u32 = 0x004a_4240;
/// `thiscall` -> float in `ST0`: the smaller bound of a range.
const RANGE_LOW: u32 = 0x004a_40a0;
/// `thiscall` -> float in `ST0`: the larger bound of a range.
const RANGE_HIGH: u32 = 0x004a_40c0;
/// `RandomFloat` (Xbox PDB), `cdecl(low, high)` -> float in `ST0`.
const RANDOM_FLOAT: u32 = 0x0047_6b70;
/// `cdecl(float)` -> float in `ST0`: wraps `00406ce0` ([`ROUND_EXPERIENCE`]).
const ROUND_FLOAT: u32 = 0x0040_6cc0;
/// `Actor::GetCurrentWeapon` (Xbox PDB), `thiscall`.
const ACTOR_GET_CURRENT_WEAPON: u32 = 0x008a_1710;
/// `thiscall` on an actor (`fallout/ai/actor.cpp`): asks the word at `+0x68`
/// (slot `0x6ec`), 0 without one.
const ACTOR_FN_008BA410: u32 = 0x008b_a410;
/// `thiscall` on an actor: its `+0x30` part's owner (`extradatalist.cpp`).
const ACTOR_OWNER_PART: u32 = 0x0041_81e0;
/// `MiddleHighProcess::GetFaceSkinnedNode` (Xbox PDB), `thiscall`: the word
/// at `this + 0x24c`.
const WEAPON_NODE: u32 = 0x008d_8ac0;
/// `thiscall(index)` (`fallout shared/bgsimpactdataset.cpp`).
const NODE_LOOKUP_BY_SLOT: u32 = 0x0058_e9d0;
/// `thiscall(0)` on a weapon (`00522ba0`).
const WEAPON_LOOKUP: u32 = 0x0052_2ba0;
/// `thiscall(index)` (`fallout shared/bgsexplosion.cpp`).
const EXPLOSION_LOOKUP: u32 = 0x004f_b7a0;
/// `LowProcess::GetSecondGenericLocation` (Xbox PDB), `thiscall`.
const SECOND_GENERIC_LOCATION: u32 = 0x0067_33e0;
/// `thiscall` on a reference: the word at `this + 0x40`.
const WORD_AT_0X40: u32 = 0x008d_6f30;
/// `ActorMover::GetPreferredMoveMode` (Xbox PDB), `thiscall` on the `TES`
/// singleton: the word at `this + 0x34`.
const WORD_AT_0X34: u32 = 0x005f_36f0;
/// The `RTTI` type descriptor of `MobileObject` (`.?AVMobileObject@@`).
const RTTI_MOBILE_OBJECT: u32 = 0x0118_4920;
/// The setting objects `005bfaf0` reads the defaults of its three numbers
/// from.
const SETTING_011DB200: u32 = 0x011d_b200;
const SETTING_011DB238: u32 = 0x011d_b238;
const SETTING_011DB25C: u32 = 0x011d_b25c;
/// `fallout/interface/interface.cpp`: the object `00460fb0` finds for the
/// interface manager, or 0 without one; takes no argument. `005bfaf0`
/// stores its three numbers into it.
const INTERFACE_OBJECT: u32 = 0x0070_5950;
/// `cdecl()` (`fallout/interface/interface.cpp`): runs `00717e30` on the
/// interface manager when there is one.
const INTERFACE_ACTION: u32 = 0x0070_5a60;
/// `thiscall(float)` on that object (`fallout shared/bgsdecalnode.cpp`).
const INTERFACE_OBJECT_FN_004EE4B0: u32 = 0x004e_e4b0;
/// `Interface::PrintLine` (Xbox PDB), `cdecl(BSStringT by value)`: the
/// 8-byte string is two stack words and the function destroys it.
const PRINT_LINE: u32 = 0x0070_3c80;
/// `BSStringT` copy constructor (`thiscall(source)`, `004047f0`).
const BS_STRING_COPY: u32 = 0x0040_47f0;
/// `"Old values:   [ Local Trees: %f | LOD Trees: %f ]"`
const MSG_OLD_VALUES: u32 = 0x0103_a4bc;
/// `"New values:   [ Local Trees: %f | LOD Trees: %f ]"`
const MSG_NEW_VALUES: u32 = 0x0103_a488;
/// The tree mipmap bias: the local trees' value.
const TREE_MIPMAP_LOCAL: u32 = 0x011f_9484;
/// The tree mipmap bias: the LOD trees' value.
const TREE_MIPMAP_LOD: u32 = 0x011f_9480;
/// `float` `163840.0`: the far distance limit of `005bf9e0`.
const FLOAT_FAR_LIMIT: u32 = 0x0103_17c4;
/// `double` `163840.0`.
const DOUBLE_FAR_LIMIT: u32 = 0x0103_a480;
/// `double` `0.0`.
const DOUBLE_ZERO: u32 = 0x0101_2060;
/// `float` `4.0`: the upper bound of the random number `005bfe90` rounds.
const FLOAT_ROUND_RANGE: u32 = 0x0101_e570;
/// `float` `1000.0`.
const FLOAT_DECAL_1000: u32 = 0x0101_3974;
/// `float` `90.0`.
const FLOAT_DECAL_90: u32 = 0x0102_49d8;

// ---- Files, positions and angles ----------------------------------------------------
/// `TESDataHandler::GetListFile` (Xbox PDB), `thiscall(name)`.
const DATA_HANDLER_GET_LIST_FILE: u32 = 0x0046_2f40;
/// `thiscall(byte)` on the data handler (`fallout shared/tes.cpp`): stores
/// the byte at `this + 0x61d`.
const DATA_HANDLER_SET_BYTE: u32 = 0x0045_0b40;
/// `TESFile::CloseTES` (Xbox PDB), `thiscall`.
const FILE_CLOSE: u32 = 0x0047_1130;
/// `"ERR: No Filename."`
const MSG_NO_FILENAME: u32 = 0x0103_a4f0;
/// `"ERR: Could not find file '%s'."`
const MSG_FILE_NOT_FOUND: u32 = 0x0103_a504;
/// `"Closed file '%s'."`
const MSG_FILE_CLOSED: u32 = 0x0103_a524;
/// `"SCRIPTS: Script '%s' (%08X) is attempting to move reference '%s' (%08X),
/// but it cannot be moved"`
const MSG_SCRIPT_MOVE_LOG: u32 = 0x0103_a538;
/// `"%s cannot be moved."`
const MSG_CANNOT_BE_MOVED: u32 = 0x0103_a598;
/// The logging function of the scripts (`005b5e40`, `cdecl`, format first).
const SCRIPT_LOG: u32 = 0x005b_5e40;
/// `thiscall` on a reference (`fallout shared/tesobjectrefr.cpp`): whether it
/// can be moved.
const REFERENCE_CAN_BE_MOVED: u32 = 0x0057_2c80;
/// `thiscall(form id)` on the data handler: true for ids from `0xff000000`.
const IS_DYNAMIC_FORM_ID: u32 = 0x0046_9860;
/// `cdecl(task, reference, axis, amount: double)` after the queue: the task
/// queue entry point (`fallout/misc`).
const QUEUE_TASK: u32 = 0x0087_a8b0;
/// The setting object whose value (2 or more) sends the move and turn
/// commands through the task queue.
const SETTING_011C3EA4: u32 = 0x011c_3ea4;
/// Task number of "set position" in the queue.
const TASK_SET_POSITION: u32 = 0x1007;
/// Task number of "set angle" in the queue.
const TASK_SET_ANGLE: u32 = 0x1009;
/// `GetPosConditionFunction` (Xbox PDB), `cdecl(object, axis, 0, result)`.
const GET_POS_CONDITION: u32 = 0x0059_c0c0;
/// `GetStartingPosConditionFunction` (Xbox PDB), same arguments.
const GET_STARTING_POS_CONDITION: u32 = 0x0059_c230;
/// `GetAngleConditionFunction` (Xbox PDB), same arguments.
const GET_ANGLE_CONDITION: u32 = 0x0059_c170;
/// `GetStartingAngleConditionFunction` (Xbox PDB), same arguments.
const GET_STARTING_ANGLE_CONDITION: u32 = 0x0059_c2d0;
/// `thiscall` on a reference: `this + 0x24`, its rotation (3 floats).
const REFERENCE_ROTATION: u32 = 0x0043_0830;
/// `double`: the radians in one degree.
const DOUBLE_RADIANS_PER_DEGREE: u32 = 0x0102_3128;
/// `double` `6.2831855` (`2 * pi` as a float).
const DOUBLE_TWO_PI: u32 = 0x0101_ff48;
/// The object whose float at `+0xc` `0084d030` returns.
const FRAME_DATA: u32 = 0x011f_6394;
/// `thiscall`: the float at `this + 0xc` (`ST0`).
const FRAME_DATA_FLOAT: u32 = 0x0084_d030;
/// `TESObjectREFR::SetAngleOnReferenceX`, `thiscall(float)`: stores the float
/// at `this + 0x24` and calls slot `0x48(2)`.
const REFERENCE_SET_ANGLE_X: u32 = 0x0057_5770;
/// `TESObjectREFR::SetAngleOnReferenceY` (Xbox PDB), `thiscall(float)`.
const REFERENCE_SET_ANGLE_Y: u32 = 0x0057_57a0;
/// `TESObjectREFR::SetAngleOnReferenceZ`, `thiscall(float)`.
const REFERENCE_SET_ANGLE_Z: u32 = 0x0057_57d0;
/// `TESObjectREFR::GetOrientation` (Xbox PDB), `thiscall(out)`: writes the
/// 9-float orientation into `out` and returns it.
const REFERENCE_GET_ORIENTATION: u32 = 0x0056_fa00;
/// `thiscall(matrix)` on a node: copies the 9 floats to `this + 0x34`
/// (`fallout shared/modelloader.cpp`).
const NODE_SET_ORIENTATION: u32 = 0x0043_fa80;
/// `thiscall(vector)` on a node: copies the 3 floats to `this + 0x58`.
const NODE_SET_POSITION: u32 = 0x0044_0460;
/// `bhkNiCollisionObject::ResetSim` (Xbox PDB), `cdecl(object, flag)`.
const COLLISION_RESET_SIM: u32 = 0x00c6_bd00;
/// Constructor `thiscall(float, byte, byte)` of the 12-byte update data.
const UPDATE_DATA_CONSTRUCT: u32 = 0x0043_d410;
/// `thiscall(update data)` on a node: updates it.
const NODE_UPDATE: u32 = 0x00a5_9c60;
/// `thiscall(vector)` (`fallout shared/tesobjectrefr.cpp`):
/// `TESObjectREFR::SetLocationOnReference` (Xbox PDB).
const REFERENCE_SET_LOCATION: u32 = 0x0057_5830;
/// `MobileObject::GetCharController` (Xbox PDB) is [`GET_CHAR_CONTROLLER`];
/// `bhkCharacterController::SetPosition` (Xbox PDB), `thiscall(vector)`.
const CONTROLLER_SET_POSITION: u32 = 0x0056_20e0;
/// `thiscall` on a controller: the number `00ca9600` returns for its
/// `this + 0x3e0` part.
const CONTROLLER_PART_VALUE: u32 = 0x00ca_9600;
/// Constructor of a four-float colour: `thiscall(r, g, b, a)`, returns `this`.
const COLOR_CONSTRUCT: u32 = 0x0041_4430;
/// The word `fn_005c02f0` returns.
const GLOBAL_011CA830: u32 = 0x011c_a830;
/// The colour (4 floats) `005c0300` sets.
const GLOBAL_COLOR: u32 = 0x011f_a090;
/// `double` `256.0`.
const DOUBLE_256: u32 = 0x0102_31d8;
/// The 3x3 identity matrix (9 floats).
const IDENTITY_MATRIX: u32 = 0x011a_9448;
/// The zero vector (3 floats).
const ZERO_VECTOR: u32 = 0x011f_426c;
/// `NiMatrix3::MakeXRotation` (Xbox PDB), `thiscall(angle)`.
const MATRIX_MAKE_X_ROTATION: u32 = 0x0052_4ac0;
/// `NiMatrix3::MakeYRotation` (Xbox PDB), `thiscall(angle)`.
const MATRIX_MAKE_Y_ROTATION: u32 = 0x0043_f850;
/// `thiscall(angle)` (`fallout shared/bgsdecalmanager.cpp`): the rotation
/// about Z.
const MATRIX_MAKE_Z_ROTATION: u32 = 0x004a_0c90;
/// `NiMatrix3::operator*` (Xbox PDB), `thiscall(out, right)`: returns `out`.
const MATRIX_MULTIPLY: u32 = 0x0043_f8d0;
/// Empty constructor of the small math types: returns `this`.
const EMPTY_CONSTRUCT: u32 = 0x0068_15c0;

// ---- Helpers ------------------------------------------------------------------------------

/// Whether `_strnicmp(literal, text, count)` says equal.
fn starts_with_no_case(e: &mut Engine, literal: u32, text: u32, count: u32) -> bool {
    e.call(STRING_COMPARE_N_NO_CASE, &args![literal, text, count])
        .i32()
        == 0
}

/// `Interface::PrintLine` with a copy of the `BSStringT` at `string`: the
/// 8-byte copy is passed by value (two words), the callee destroys it.
fn print_line(e: &mut Engine, string: u32) {
    let copy = e.mem.alloc(8);
    e.call(BS_STRING_COPY, &args![copy, string]);
    let (first, second) = (e.mem.u32(copy), e.mem.u32(copy + 4));
    e.call(PRINT_LINE, &args![first, second]);
    e.mem.free(copy);
}

/// The object `005bf8d0` and `005bf9e0` reach through the `TES` singleton:
/// `word(word(tes + 0x68) + 0x20)`.
fn tes_holder(e: &mut Engine) -> u32 {
    let tes = e.global::<u32>(TES_SINGLETON);
    let inner = e.call(WORD_AT_0X68, &args![tes]).u32();
    e.call(WORD_AT_0X20, &args![inner]).u32()
}

/// Copies `words` 32-bit words from `from` to `to`.
fn copy_words(e: &mut Engine, from: u32, to: u32, words: u32) {
    for i in 0..words {
        let value = e.mem.u32(from + 4 * i);
        e.mem.set_u32(to + 4 * i, value);
    }
}

/// The constructor + update the position and angle commands finish with:
/// builds the 12-byte update data (`0.0, 0, 0`) and updates the node.
fn update_node(e: &mut Engine, node: u32) {
    let data = e.mem.alloc(12);
    e.call(UPDATE_DATA_CONSTRUCT, &args![data, 0.0f32, 0u32, 0u32]);
    e.call(NODE_UPDATE, &args![node, data]);
    e.mem.free(data);
}

/// Calls the setter of `REFERENCE_SET_ANGLE_X/Y/Z` for the axis letter
/// (`'X'`, `'Y'`, `'Z'`; nothing for any other).
fn set_reference_angle(e: &mut Engine, reference: Ptr, axis: u8, angle: f32) {
    let setter = match axis {
        b'X' => REFERENCE_SET_ANGLE_X,
        b'Y' => REFERENCE_SET_ANGLE_Y,
        b'Z' => REFERENCE_SET_ANGLE_Z,
        _ => return,
    };
    e.call(setter, &args![reference, angle]);
}

/// The bodies of `GetPos`, `GetStartingPos`, `GetAngle` and
/// `GetStartingAngle`: parse an axis letter and forward to the condition
/// function `condition(thisObj, axis, 0, result)`.
fn get_axis_command(e: &mut Engine, a: ScriptArgs, condition: u32) -> bool {
    let Some([axis]) = parse_into(e, a, [0]) else {
        return false;
    };
    let axis = axis as u8 as i8 as i32;
    e.call(condition, &args![a.this_obj, axis, 0u32, a.result])
        .bool()
}

/// The body of `SetPos` and `SetAngle`: for a reference that cannot be moved
/// prints why (and logs the script that tried), otherwise either queues the
/// task (when the setting `011c3ea4` is above 1) or runs `base` directly.
fn move_command(
    e: &mut Engine,
    a: ScriptArgs,
    task: u32,
    base: fn(&mut Engine, Ptr, u8, f32),
    axis: u8,
    amount: f32,
) -> bool {
    if !a.this_obj.is_null() && !e.call(REFERENCE_CAN_BE_MOVED, &args![a.this_obj]).bool() {
        if echo_enabled(e) {
            let name = e.vcall(a.this_obj.addr(), 0x130, &args![]).u32();
            console_print(e, &args![MSG_CANNOT_BE_MOVED, name]);
        }
        let script_id = e.call(FORM_ID, &args![a.script_obj]).u32();
        let data_handler = e.global::<u32>(DATA_HANDLER);
        if !e
            .call(IS_DYNAMIC_FORM_ID, &args![data_handler, script_id])
            .bool()
        {
            let reference_id = e.call(FORM_ID, &args![a.this_obj]).u32();
            let reference_name = e.vcall(a.this_obj.addr(), 0x130, &args![]).u32();
            let script_id = e.call(FORM_ID, &args![a.script_obj]).u32();
            let script_name = e.vcall(a.script_obj.addr(), 0x130, &args![]).u32();
            e.call(
                SCRIPT_LOG,
                &args![
                    MSG_SCRIPT_MOVE_LOG,
                    script_name,
                    script_id,
                    reference_name,
                    reference_id
                ],
            );
        }
        return true;
    }
    if setting_value(e, SETTING_011C3EA4) > 1 {
        let queue = e.call(TASK_QUEUE_INTERFACE, &args![]).u32();
        e.call(
            QUEUE_TASK,
            &args![queue, task, a.this_obj, axis as i8 as i32, amount as f64],
        );
    } else {
        base(e, a.this_obj, axis, amount);
    }
    true
}

// ---- Settings: reading and writing ------------------------------------------------

// Translated from 005be900 (decompiled, FalloutNV.exe 1.4.0.525)
/// Console command that reads a game setting: parses its name (up to 512
/// characters), stores `0.0` in the result first (also when the parse
/// fails), looks the setting up in the game setting collection and, for
/// integer (3) and float (5) settings, stores the value in the result. When
/// commands echo, prints `GameSetting <name> >> <value>` (or `NOT FOUND`,
/// the string value for type 6, `UNKNOWN TYPE`).
pub fn fn_005be900(e: &mut Engine, a: ScriptArgs) -> bool {
    e.mem.set_f64(a.result.addr(), 0.0);
    let name = e.mem.alloc(TEXT_PARAMETER_SIZE);
    let ok = parse(e, a, &[name]);
    if ok {
        let collection = e.call(GAME_SETTINGS, &args![]).u32();
        let setting = e.call(GAME_SETTING_FIND, &args![collection, name]).u32();
        if setting == 0 {
            if echo_enabled(e) {
                // The extra argument is what the null setting answers.
                let value = e.call(SETTING_STRING_VALUE, &args![setting]).u32();
                console_print(e, &args![MSG_GAME_NOT_FOUND, name, value]);
            }
        } else {
            match e.call(SETTING_TYPE, &args![setting]).u32() {
                3 => {
                    let address = e.call(SETTING_VALUE_ADDRESS, &args![setting]).u32();
                    let value = e.mem.i32(address);
                    e.mem.set_f64(a.result.addr(), value as f64);
                    if echo_enabled(e) {
                        let address = e.call(SETTING_VALUE_ADDRESS, &args![setting]).u32();
                        let value = e.mem.u32(address);
                        let setting_name = e.call(SETTING_NAME, &args![setting]).u32();
                        console_print(e, &args![MSG_GAME_INT, setting_name, value]);
                    }
                }
                5 => {
                    let address = e.call(SETTING_FLOAT_ADDRESS, &args![setting]).u32();
                    let value = e.mem.f32(address);
                    e.mem.set_f64(a.result.addr(), value as f64);
                    if echo_enabled(e) {
                        let address = e.call(SETTING_FLOAT_ADDRESS, &args![setting]).u32();
                        let value = e.mem.f32(address);
                        let setting_name = e.call(SETTING_NAME, &args![setting]).u32();
                        console_print(e, &args![MSG_GAME_FLOAT, setting_name, value as f64]);
                    }
                }
                6 => {
                    if echo_enabled(e) {
                        let value = e.call(SETTING_STRING_VALUE, &args![setting]).u32();
                        let setting_name = e.call(SETTING_NAME, &args![setting]).u32();
                        console_print(e, &args![MSG_GAME_STRING, setting_name, value]);
                    }
                }
                _ => {
                    if echo_enabled(e) {
                        let setting_name = e.call(SETTING_NAME, &args![setting]).u32();
                        console_print(e, &args![MSG_GAME_UNKNOWN, setting_name]);
                    }
                }
            }
        }
    }
    e.mem.free(name);
    ok
}

// Translated from 005beb00 (decompiled, FalloutNV.exe 1.4.0.525)
/// Console command that sets a game setting: parses a name and a value
/// (both texts of up to 512 characters), looks the setting up and stores the
/// value by type (integer 3 through `atol`, float 5 through `atof`, string
/// 6); echoes the new value when commands echo. A missing setting and an
/// unknown type are always reported.
pub fn fn_005beb00(e: &mut Engine, a: ScriptArgs) -> bool {
    let name = e.mem.alloc(TEXT_PARAMETER_SIZE);
    let value_text = e.mem.alloc(TEXT_PARAMETER_SIZE);
    let ok = parse(e, a, &[name, value_text]);
    if ok {
        let collection = e.call(GAME_SETTINGS, &args![]).u32();
        let setting = e.call(GAME_SETTING_FIND, &args![collection, name]).u32();
        if setting == 0 {
            console_print(e, &args![MSG_GAME_NOT_FOUND, name]);
        } else {
            match e.call(SETTING_TYPE, &args![setting]).u32() {
                3 => {
                    let number = e.call(ATOL, &args![value_text]).i32();
                    e.call(SETTING_SET_INT, &args![setting, number]);
                    if echo_enabled(e) {
                        let address = e.call(SETTING_VALUE_ADDRESS, &args![setting]).u32();
                        let value = e.mem.u32(address);
                        let setting_name = e.call(SETTING_NAME, &args![setting]).u32();
                        console_print(e, &args![MSG_GAME_INT, setting_name, value]);
                    }
                }
                5 => {
                    let number = e.call(ATOF, &args![value_text]).f64() as f32;
                    e.call(SETTING_SET_FLOAT, &args![setting, number]);
                    if echo_enabled(e) {
                        let address = e.call(SETTING_FLOAT_ADDRESS, &args![setting]).u32();
                        let value = e.mem.f32(address);
                        let setting_name = e.call(SETTING_NAME, &args![setting]).u32();
                        console_print(e, &args![MSG_GAME_FLOAT, setting_name, value as f64]);
                    }
                }
                6 => {
                    e.call(SETTING_SET_STRING, &args![setting, value_text]);
                    if echo_enabled(e) {
                        let value = e.call(SETTING_STRING_VALUE, &args![setting]).u32();
                        let setting_name = e.call(SETTING_NAME, &args![setting]).u32();
                        console_print(e, &args![MSG_GAME_STRING, setting_name, value]);
                    }
                }
                _ => console_print(e, &args![MSG_GAME_UNKNOWN, name]),
            }
        }
    }
    e.mem.free(value_text);
    e.mem.free(name);
    ok
}

/// The INI setting `name` of the two INI setting collections (the second is
/// asked when the first has none), or 0.
fn find_ini_setting(e: &mut Engine, name: u32) -> u32 {
    let collection = e.call(INI_SETTINGS, &args![]).u32();
    let mut setting = e.call(INI_SETTING_FIND, &args![collection, name]).u32();
    if setting == 0 {
        let fallback = e.call(INI_SETTINGS_FALLBACK, &args![]).u32();
        setting = e.call(INI_SETTING_FIND, &args![fallback, name]).u32();
    }
    setting
}

// Translated from 005becf0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Console command that reads an INI setting: parses its name, looks it up
/// in the INI setting collection and then in the second one, and prints
/// `INISetting <name> >> <value>` by type (0 boolean, 3 integer, 5 float, 6
/// string), `UNKNOWN TYPE` or `NOT FOUND`. Prints whether or not commands
/// echo.
pub fn fn_005becf0(e: &mut Engine, a: ScriptArgs) -> bool {
    let name = e.mem.alloc(TEXT_PARAMETER_SIZE);
    let ok = parse(e, a, &[name]);
    if ok {
        let setting = find_ini_setting(e, name);
        if setting == 0 {
            console_print(e, &args![MSG_INI_NOT_FOUND, name]);
        } else {
            match e.call(SETTING_TYPE, &args![setting]).u32() {
                0 => {
                    let address = e.call(SETTING_BOOL_ADDRESS, &args![setting]).u32();
                    let value = e.mem.u8(address) != 0;
                    console_print(e, &args![MSG_INI_INT, name, value]);
                }
                3 => {
                    let address = e.call(SETTING_VALUE_ADDRESS, &args![setting]).u32();
                    let value = e.mem.u32(address);
                    console_print(e, &args![MSG_INI_INT, name, value]);
                }
                5 => {
                    let address = e.call(SETTING_FLOAT_ADDRESS, &args![setting]).u32();
                    let value = e.mem.f32(address);
                    console_print(e, &args![MSG_INI_FLOAT, name, value as f64]);
                }
                6 => {
                    let address = e.call(SETTING_VALUE_ADDRESS, &args![setting]).u32();
                    let value = e.mem.u32(address);
                    console_print(e, &args![MSG_INI_STRING, name, value]);
                }
                _ => console_print(e, &args![MSG_INI_UNKNOWN, name]),
            }
        }
    }
    e.mem.free(name);
    ok
}

// Translated from 005bee90 (decompiled, FalloutNV.exe 1.4.0.525)
/// Console command that sets an INI setting: parses a name and a value text,
/// finds the setting as `005becf0` does and stores the value by type
/// (boolean 0 and integer 3 through `atol`, float 5 through `atof`, string
/// 6), echoing the new value when commands echo. A missing setting and an
/// unknown type are always reported.
pub fn fn_005bee90(e: &mut Engine, a: ScriptArgs) -> bool {
    let name = e.mem.alloc(TEXT_PARAMETER_SIZE);
    let value_text = e.mem.alloc(TEXT_PARAMETER_SIZE);
    let ok = parse(e, a, &[name, value_text]);
    if ok {
        let setting = find_ini_setting(e, name);
        if setting == 0 {
            console_print(e, &args![MSG_INI_NOT_FOUND, name]);
        } else {
            match e.call(SETTING_TYPE, &args![setting]).u32() {
                0 => {
                    let number = e.call(ATOL, &args![value_text]).i32();
                    e.call(SETTING_SET_INT, &args![setting, number]);
                    if echo_enabled(e) {
                        let address = e.call(SETTING_BOOL_ADDRESS, &args![setting]).u32();
                        let value = e.mem.u8(address) != 0;
                        console_print(e, &args![MSG_INI_INT, name, value]);
                    }
                }
                3 => {
                    let number = e.call(ATOL, &args![value_text]).i32();
                    e.call(SETTING_SET_INT, &args![setting, number]);
                    if echo_enabled(e) {
                        let address = e.call(SETTING_VALUE_ADDRESS, &args![setting]).u32();
                        let value = e.mem.u32(address);
                        console_print(e, &args![MSG_INI_INT, name, value]);
                    }
                }
                5 => {
                    let number = e.call(ATOF, &args![value_text]).f64() as f32;
                    e.call(SETTING_SET_FLOAT, &args![setting, number]);
                    if echo_enabled(e) {
                        let address = e.call(SETTING_FLOAT_ADDRESS, &args![setting]).u32();
                        let value = e.mem.f32(address);
                        console_print(e, &args![MSG_INI_FLOAT, name, value as f64]);
                    }
                }
                6 => {
                    e.call(SETTING_SET_STRING, &args![setting, value_text]);
                    if echo_enabled(e) {
                        let address = e.call(SETTING_VALUE_ADDRESS, &args![setting]).u32();
                        let value = e.mem.u32(address);
                        console_print(e, &args![MSG_INI_STRING, name, value]);
                    }
                }
                _ => console_print(e, &args![MSG_INI_UNKNOWN, name]),
            }
        }
    }
    e.mem.free(value_text);
    e.mem.free(name);
    ok
}

// ---- The render options command and its helpers -----------------------------------------

// Translated from 005bf770 (decompiled, FalloutNV.exe 1.4.0.525)
/// Sets (`on` non-zero) or clears bit `0x10` of the render flags word
/// `011f91d8`.
pub fn fn_005bf770(e: &mut Engine, on: u8) {
    let flags = e.global::<u32>(RENDER_FLAGS);
    let flags = if on != 0 { flags | 0x10 } else { flags & !0x10 };
    e.set_global(RENDER_FLAGS, flags);
}

// Translated from 005bf7b0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Whether `004dc060(0)` is at least 3 and bit `0x10` of the render flags
/// word `011f91d8` is set.
pub fn fn_005bf7b0(e: &mut Engine) -> bool {
    if e.call(RENDER_SETTING, &args![0u32]).i32() < 3 {
        return false;
    }
    e.global::<u32>(RENDER_FLAGS) & 0x10 != 0
}

// Translated from 005bf7f0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Stores `value` in the word `011f91b4`.
pub fn fn_005bf7f0(e: &mut Engine, value: u32) {
    e.set_global(GLOBAL_011F91B4, value);
}

// Translated from 005bf800 (decompiled, FalloutNV.exe 1.4.0.525)
/// Stores `value` in the byte at `this + 0x124`.
pub fn fn_005bf800(e: &mut Engine, this: Ptr, value: u8) {
    e.mem.set_u8(this.addr().wrapping_add(0x124), value);
}

// Translated from 005bf820 (decompiled, FalloutNV.exe 1.4.0.525)
/// The word at `this + 0x128` (`00559450` on that address).
pub fn fn_005bf820(e: &mut Engine, this: Ptr) -> u32 {
    e.call(STRING_TEXT, &args![this.addr().wrapping_add(0x128)])
        .u32()
}

// Translated from 005bf840 (decompiled, FalloutNV.exe 1.4.0.525)
/// Calls `004de2d0(value)` on the renderer object `011c75a4`.
pub fn fn_005bf840(e: &mut Engine, value: u8) {
    e.call(RENDERER_SET_BYTE, &args![RENDERER_OBJECT, value]);
}

// Translated from 005bf860 (decompiled, FalloutNV.exe 1.4.0.525)
/// Toggles the byte `0118c000`; true.
pub fn fn_005bf860(e: &mut Engine) -> bool {
    let value = e.global::<u8>(GLOBAL_0118C000);
    e.set_global(GLOBAL_0118C000, (value == 0) as u8);
    true
}

// Translated from 005bf880 (decompiled, FalloutNV.exe 1.4.0.525)
/// Sets the byte `011f91dc` to 1 unless it already is 1 (then 0); true.
pub fn fn_005bf880(e: &mut Engine) -> bool {
    let value = fn_005bf8b0(e);
    fn_005bf8c0(e, (value != 1) as u8);
    true
}

// Translated from 005bf8b0 (decompiled, FalloutNV.exe 1.4.0.525)
/// The byte `011f91dc`.
pub fn fn_005bf8b0(e: &mut Engine) -> u8 {
    e.global::<u8>(GLOBAL_011F91DC)
}

// Translated from 005bf8c0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Stores `value` in the byte `011f91dc`.
pub fn fn_005bf8c0(e: &mut Engine, value: u8) {
    e.set_global(GLOBAL_011F91DC, value);
}

/// The part of the `1` and `4` options that depends on the text being a
/// single character: marks bits `0xe` and, when `004dc060(0)` is at least 2,
/// `0x20` of `mask`.
fn single_character_flags(e: &mut Engine, text: u32, mask: &mut u16) {
    if e.call(STRLEN, &args![text]).u32() == 1 {
        *mask |= 0x0e;
        if e.call(RENDER_SETTING, &args![0u32]).i32() >= 2 {
            *mask |= 0x20;
        }
    }
}

/// The body of `005bf130` after the parameter parse: acts on the option text
/// and finishes with the flags in `mask` (`004bc3d0`) unless the option
/// returns early.
fn render_option(e: &mut Engine, this_obj: Ptr, text: u32) {
    let mut mask: u16 = 0;
    match e.mem.u8(text) {
        b'1' => {
            mask = 1;
            e.call(SHADER_SET_MODE, &args![0u32]);
            fn_005bf7f0(e, 0);
            single_character_flags(e, text, &mut mask);
        }
        b'2' => {
            mask = 1;
            fn_005bf7f0(e, 1);
        }
        b'3' => {
            mask = 1;
            fn_005bf7f0(e, 2);
        }
        b'4' => {
            mask = 1;
            e.call(SHADER_SET_MODE, &args![3u32]);
            fn_005bf7f0(e, 0);
            single_character_flags(e, text, &mut mask);
        }
        b'5' => {}
        _ => {
            if starts_with_no_case(e, OPTION_TEX, text, 3) {
                mask = 1;
                fn_005bf7f0(e, 4);
            } else if starts_with_no_case(e, OPTION_DEGRADE, text, 7) {
                mask = 1;
                fn_005bf7f0(e, 5);
            } else if starts_with_no_case(e, OPTION_SH, text, 2) {
                if e.mem.u8(text + 2) == 0 {
                    let enabled = fn_005bf7b0(e);
                    fn_005bf770(e, !enabled as u8);
                    mask = e.call(RENDER_FLAGS_GET, &args![]).u16();
                    if mask & 0x80 == 0 {
                        mask |= 0x80;
                    } else {
                        mask &= !0x80;
                    }
                } else {
                    let mut level = e.mem.i8(text + 2) as i32 - 0x30;
                    if level == 1 || level == 2 {
                        level = 3;
                    }
                    let smaller = e.call(MIN_I32, &args![3i32, level]).i32();
                    let level = e.call(MAX_I32, &args![0i32, smaller]).u32();
                    e.set_global(GLOBAL_011F91B0, level);
                    e.call(SHADER_REFRESH, &args![]);
                    mask = e.call(RENDER_FLAGS_GET, &args![]).u16();
                    mask |= 0x80;
                }
            } else if starts_with_no_case(e, OPTION_SC, text, 2) {
                let value = e.global::<u8>(GLOBAL_011F9441);
                e.set_global(GLOBAL_011F9441, (value == 0) as u8);
                mask = e.call(RENDER_FLAGS_GET, &args![]).u16();
            } else if starts_with_no_case(e, OPTION_PSH, text, 2) {
                let flags = e.call(RENDER_FLAGS_GET, &args![]).u32();
                if flags & 0x20 == 0 {
                    fn_005bf840(e, 1);
                    let tes = e.global::<u32>(TES_SINGLETON);
                    let target = e.call(WORD_AT_0X8, &args![tes]).u32();
                    e.call(PSH_ENABLE, &args![target]);
                    let current = e.call(RENDER_FLAGS_GET, &args![]).u16();
                    e.call(RENDER_FLAGS_SET, &args![(current | 0x20) as u32]);
                } else {
                    fn_005bf840(e, 0);
                    e.call(PSH_DISABLE, &args![0u32]);
                    let current = e.call(RENDER_FLAGS_GET, &args![]).u16();
                    e.call(RENDER_FLAGS_SET, &args![(current & !0x20) as u32]);
                }
                return;
            } else if e.mem.u8(text) == b't' {
                let value = e.call(ATOL, &args![text + 1]).u32();
                e.set_global(GLOBAL_011AD828, value);
            } else if starts_with_no_case(e, OPTION_DSH, text, 3) {
                if !this_obj.is_null() {
                    let list = e.call(SHADOW_LIST_GET, &args![0u32]).u32();
                    let mut node = e.call(SHADOW_LIST_FIRST, &args![list]).u32();
                    while node != 0 {
                        let node_value = fn_005bf820(e, Ptr::new(node));
                        let object_value = e.vcall(this_obj.addr(), 0x1d0, &args![]).u32();
                        if node_value == object_value {
                            let flag = e.call(NODE_FLAG, &args![node]).bool();
                            fn_005bf800(e, Ptr::new(node), !flag as u8);
                        }
                        node = e.call(SHADOW_LIST_NEXT, &args![list]).u32();
                    }
                }
                return;
            } else if starts_with_no_case(e, OPTION_OPT, text, 3) {
                let value = e.global::<u8>(GLOBAL_011AD80D);
                e.set_global(GLOBAL_011AD80D, (value == 0) as u8);
                mask = e.call(RENDER_FLAGS_GET, &args![]).u16();
                mask |= 0x100;
            } else if starts_with_no_case(e, OPTION_ALPHA, text, 5) {
                return;
            }
        }
    }
    // The characters after the first select further bits.
    if e.mem.u8(text + 1) == b'1' {
        mask |= 2;
    }
    if e.mem.u8(text + 2) == b'1' {
        mask |= 4;
    }
    if e.mem.u8(text + 3) == b'1' {
        mask |= 8;
    }
    if e.call(RENDER_SETTING, &args![0u32]).i32() >= 2 && e.mem.u8(text + 4) != b'0' {
        mask |= 0x20;
    }
    e.call(RENDER_FLAGS_SET, &args![mask as u32]);
}

// Translated from 005bf130 (decompiled, FalloutNV.exe 1.4.0.525)
/// The render options console command: parses a text (up to 512 characters)
/// and acts on it. A first character `1` to `5` selects a shader mode (`1`
/// and `4` also set the shader table through `00b4f450`), `tex`, `degrade`,
/// `sh` (toggle, or `sh<digit>` for a level 0 to 3), `sc`, `psh`, `t<number>`,
/// `dsh` (clears the flag of the shadow nodes of the command's reference),
/// `opt` and `alpha` are the named options; characters 2 to 5 of the text
/// add bits to the flags word, which is stored through `004bc3d0`
/// (`psh`, `dsh` and `alpha` leave without). True whenever the parameters
/// parse.
pub fn fn_005bf130(e: &mut Engine, a: ScriptArgs) -> bool {
    let text = e.mem.alloc(TEXT_PARAMETER_SIZE);
    let ok = parse(e, a, &[text]);
    if ok {
        render_option(e, a.this_obj, text);
    }
    e.mem.free(text);
    ok
}

// ---- Small accessors and the screen-position commands ---------------------------------

// Translated from 005bf8d0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Parses two integers (default 0). When the object reached through the
/// `TES` singleton (`word(word(word(tes + 0x68) + 0x20) + 0xc)`) exists,
/// stores them as floats at its `+0x2c`/`+0x30` ([`fn_005bf9a0`]) and sets
/// the byte at `+0x18` of the holder to "both zero" ([`fn_005bf9c0`]).
pub fn fn_005bf8d0(e: &mut Engine, a: ScriptArgs) -> bool {
    let Some([x, y]) = parse_into(e, a, [0, 0]) else {
        return false;
    };
    let holder = tes_holder(e);
    let target = e.call(WORD_AT_0XC, &args![holder]).u32();
    if target != 0 {
        let both_zero = x == 0 && y == 0;
        let holder = tes_holder(e);
        fn_005bf9c0(e, Ptr::new(holder), both_zero as u8);
        fn_005bf9a0(e, Ptr::new(target), x as i32 as f32, y as i32 as f32);
    }
    true
}

// Translated from 005bf9a0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Stores the floats `x` and `y` at `this + 0x2c` and `this + 0x30`.
pub fn fn_005bf9a0(e: &mut Engine, this: Ptr, x: f32, y: f32) {
    e.mem.set_f32(this.addr().wrapping_add(0x2c), x);
    e.mem.set_f32(this.addr().wrapping_add(0x30), y);
}

// Translated from 005bf9c0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Stores `value` in the byte at `this + 0x18`.
pub fn fn_005bf9c0(e: &mut Engine, this: Ptr, value: u8) {
    e.mem.set_u8(this.addr().wrapping_add(0x18), value);
}

// Translated from 005bf9e0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Parses a distance (default 0.0) and sets the camera's far plane to it:
/// a distance of 0 or less becomes `163840.0` and sets the holder's flag
/// byte ([`fn_005bf9c0`]) to 1, a distance above `163840.0` is limited to
/// it. When the camera exists and its far distance differs, the far
/// distance, the far-to-near ratio (`0x507700`) and the view frustum
/// (`NiCamera::SetViewFrustum`) are updated and the holder's flag byte is
/// stored. True whenever the parameter parses.
pub fn fn_005bf9e0(e: &mut Engine, a: ScriptArgs) -> bool {
    let Some([bits]) = parse_into(e, a, [0]) else {
        return false;
    };
    let mut distance = f32::from_bits(bits);
    let mut at_limit = 0u8;
    let limit = e.global::<f32>(FLOAT_FAR_LIMIT);
    if distance as f64 <= e.global::<f64>(DOUBLE_ZERO) {
        distance = limit;
        at_limit = 1;
    } else if distance as f64 > e.global::<f64>(DOUBLE_FAR_LIMIT) {
        distance = limit;
    }
    let holder = e.call(CAMERA_HOLDER, &args![]).u32();
    let camera = e.call(WORD_AT_0XAC, &args![holder]).u32();
    if camera != 0 {
        let source = e.call(CAMERA_FRUSTUM, &args![camera]).u32();
        let frustum = e.mem.alloc(0x1c);
        copy_words(e, source, frustum, 7);
        let near = e.mem.f32(frustum + 0x10);
        if e.mem.f32(frustum + 0x14) != distance {
            e.mem.set_f32(frustum + 0x14, distance);
            let ratio = (distance as f64 / near as f64) as f32;
            e.call(CAMERA_SET_RATIO, &args![camera, ratio]);
            e.call(CAMERA_SET_VIEW_FRUSTUM, &args![camera, frustum]);
            let holder = tes_holder(e);
            fn_005bf9c0(e, Ptr::new(holder), at_limit);
        }
        e.mem.free(frustum);
    }
    true
}

// Translated from 005bfaf0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Parses three floats whose defaults are the values of the settings
/// `011db200`, `011db238` and `011db25c` (read before the parse, and a
/// fourth word the command does not use) and hands them to the interface
/// object: the first through `005bfbb0`, the second through `005bfbd0`, the
/// third through `004ee4b0`. Returns whether the parse succeeded.
pub fn fn_005bfaf0(e: &mut Engine, a: ScriptArgs) -> bool {
    let mut defaults = [0u32; 4];
    for (slot, setting) in
        defaults
            .iter_mut()
            .zip([SETTING_011DB200, SETTING_011DB238, SETTING_011DB25C])
    {
        let address = e.call(SETTING_FLOAT_ADDRESS, &args![setting]).u32();
        *slot = e.mem.u32(address);
    }
    let Some([first, second, third, _unused]) = parse_into(e, a, defaults) else {
        return false;
    };
    let object = e.call(INTERFACE_OBJECT, &args![]).u32();
    fn_005bfbb0(e, Ptr::new(object), f32::from_bits(first));
    let object = e.call(INTERFACE_OBJECT, &args![]).u32();
    fn_005bfbd0(e, Ptr::new(object), f32::from_bits(second));
    let object = e.call(INTERFACE_OBJECT, &args![]).u32();
    e.call(
        INTERFACE_OBJECT_FN_004EE4B0,
        &args![object, f32::from_bits(third)],
    );
    true
}

// Translated from 005bfbb0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Stores the float `value` at `this + 0x58`.
pub fn fn_005bfbb0(e: &mut Engine, this: Ptr, value: f32) {
    e.mem.set_f32(this.addr().wrapping_add(0x58), value);
}

// Translated from 005bfbd0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Stores the float `value` at `this + 0x60`.
pub fn fn_005bfbd0(e: &mut Engine, this: Ptr, value: f32) {
    e.mem.set_f32(this.addr().wrapping_add(0x60), value);
}

// Translated from 005bfbf0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Script::SetTreeMipmapBias` (Xbox PDB): parses two floats (defaults: the
/// current values of `011f9484`, the local trees, and `011f9480`, the LOD
/// trees), prints the old values (`Old values:   [ Local Trees: %f | LOD
/// Trees: %f ]`), stores the new ones and prints them. The output goes
/// through a `BSStringT` formatted by `00406f60` and printed by
/// `Interface::PrintLine`. The compiler's exception-unwinding frame is not
/// translated.
pub fn script_set_tree_mipmap_bias(e: &mut Engine, a: ScriptArgs) -> bool {
    let local = e.global::<u32>(TREE_MIPMAP_LOCAL);
    let lod = e.global::<u32>(TREE_MIPMAP_LOD);
    let Some([new_local, new_lod]) = parse_into(e, a, [local, lod]) else {
        return false;
    };
    let old_lod = e.global::<f32>(TREE_MIPMAP_LOD);
    let old_local = e.global::<f32>(TREE_MIPMAP_LOCAL);
    let string = e.mem.alloc(8);
    e.call(BS_STRING_CONSTRUCT, &args![string]);
    e.call(
        BS_STRING_FORMAT,
        &args![string, MSG_OLD_VALUES, old_local as f64, old_lod as f64],
    );
    print_line(e, string);
    e.set_global(TREE_MIPMAP_LOD, new_lod);
    e.set_global(TREE_MIPMAP_LOCAL, new_local);
    e.call(
        BS_STRING_FORMAT,
        &args![
            string,
            MSG_NEW_VALUES,
            f32::from_bits(new_local) as f64,
            f32::from_bits(new_lod) as f64
        ],
    );
    print_line(e, string);
    e.call(BS_STRING_DESTRUCT, &args![string]);
    e.mem.free(string);
    true
}

// Translated from 005bfd40 (decompiled, FalloutNV.exe 1.4.0.525)
/// Calls `00705a60` (`fallout/interface/interface.cpp`); true.
pub fn fn_005bfd40(e: &mut Engine) -> bool {
    e.call(INTERFACE_ACTION, &args![]);
    true
}

// Translated from 005bfd50 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Script::ForceCloseFile` (Xbox PDB): parses a file name (up to 512
/// characters) and, when commands echo, reports `ERR: No Filename.` for an
/// empty one; otherwise looks the file up in the data handler
/// (`TESDataHandler::GetListFile`) and closes it (`TESFile::CloseTES`)
/// between two `0x61d` byte stores of the data handler (1, then 0), or
/// reports that it was not found. True whenever the parameter parses.
pub fn script_force_close_file(e: &mut Engine, a: ScriptArgs) -> bool {
    let name = e.mem.alloc(TEXT_PARAMETER_SIZE);
    e.mem.set_u8(name, 0);
    let ok = parse(e, a, &[name]);
    if ok {
        if e.mem.i8(name) != 0 {
            let data_handler = e.global::<u32>(DATA_HANDLER);
            let file = e
                .call(DATA_HANDLER_GET_LIST_FILE, &args![data_handler, name])
                .u32();
            if file != 0 {
                let data_handler = e.global::<u32>(DATA_HANDLER);
                e.call(DATA_HANDLER_SET_BYTE, &args![data_handler, 1u32]);
                e.call(FILE_CLOSE, &args![file]);
                let data_handler = e.global::<u32>(DATA_HANDLER);
                e.call(DATA_HANDLER_SET_BYTE, &args![data_handler, 0u32]);
                if echo_enabled(e) {
                    console_print(e, &args![MSG_FILE_CLOSED, name]);
                }
            } else if echo_enabled(e) {
                console_print(e, &args![MSG_FILE_NOT_FOUND, name]);
            }
        } else if echo_enabled(e) {
            console_print(e, &args![MSG_NO_FILENAME]);
        }
    }
    e.mem.free(name);
    ok
}

// Translated from 005bfe90 (decompiled, FalloutNV.exe 1.4.0.525)
/// Fills in a 0x74-byte request (`004a37b0`, `fallout shared/
/// bgsdecalmanager.cpp`) from the camera and submits it (`004a3fe0`). The
/// parameters are not parsed. The request starts with the camera position.
/// Without a reference only two lookups happen (the current cell of the
/// `TES` singleton, or the player's `+0x40` word). With a reference that has
/// a `+0x40` word and a slot `0x1d0` object, the request gets: a location
/// found from the player's current weapon (for an actor through its slot and
/// the weapon's node, otherwise through the weapon's `+0x24c` part; falling
/// back to `fn_005c02f0`'s object and index 6), flag bits 1 to 15 for an
/// actor, `-1` at `+0x34`, a random number in `0.0..4.0` rounded into the
/// byte at `+0x70`, a random number between the bounds of the found object
/// at `+0x38`/`+0x3c`, `1000.0` at `+0x40`, `90.0` at `+0x58`, and the
/// direction vector: the camera's for a reference with a `+0xbc` word or an
/// actor (mode 2, the reference at `+0x24`), the camera's negated otherwise
/// (mode 1, the `+0x1d0` object at `+0x28`). True.
pub fn fn_005bfe90(e: &mut Engine, a: ScriptArgs) -> bool {
    let holder = e.call(CAMERA_HOLDER, &args![]).u32();
    let camera = e.call(WORD_AT_0XAC, &args![holder]).u32();
    let request = e.mem.alloc(0x74);
    e.call(DECAL_REQUEST_CONSTRUCT, &args![request]);
    let position = e.call(CAMERA_POSITION, &args![camera]).u32();
    copy_words(e, position, request, 3);
    if a.this_obj.is_null() {
        let tes = e.global::<u32>(TES_SINGLETON);
        if e.call(WORD_AT_0X34, &args![tes]).u32() != 0 {
            e.call(GET_CURRENT_CELL, &args![tes]);
        } else {
            let player = e.global::<u32>(PLAYER);
            e.call(WORD_AT_0X40, &args![player]);
        }
    } else {
        decal_request_for_reference(e, a.this_obj, camera, request);
    }
    e.mem.free(request);
    true
}

/// The part of `005bfe90` for a reference: see there.
fn decal_request_for_reference(e: &mut Engine, reference: Ptr, camera: u32, request: u32) {
    let object = reference.addr();
    let world = e.call(WORD_AT_0X40, &args![object]).u32();
    if e.vcall(object, 0x1d0, &args![]).u32() == 0 {
        return;
    }
    let held = e.vcall(object, 0x1d0, &args![]).u32();
    let player = e.global::<u32>(PLAYER);
    let weapon = e.call(ACTOR_GET_CURRENT_WEAPON, &args![player]).u32();
    let mut found = if e.vcall(object, 0x100, &args![]).bool() {
        let actor = cast_to_actor(e, reference);
        let slot = if e.call(ACTOR_FN_008BA410, &args![actor]).bool() {
            4
        } else {
            let part = e.call(ACTOR_OWNER_PART, &args![actor]).u32() + 0x30;
            e.vcall(part, 0x58, &args![]).u32()
        };
        let node = if weapon != 0 {
            e.call(WEAPON_NODE, &args![weapon]).u32()
        } else {
            0
        };
        if node != 0 {
            e.call(NODE_LOOKUP_BY_SLOT, &args![node, slot]).u32()
        } else {
            0
        }
    } else if weapon != 0 {
        e.call(WEAPON_LOOKUP, &args![weapon, 0u32]).u32()
    } else {
        0
    };
    let mut location = if found != 0 {
        e.call(SECOND_GENERIC_LOCATION, &args![found]).u32()
    } else {
        0
    };
    if location == 0 {
        let owner = fn_005c02f0(e);
        found = e.call(EXPLOSION_LOOKUP, &args![owner, 6u32]).u32();
        location = if found != 0 {
            e.call(SECOND_GENERIC_LOCATION, &args![found]).u32()
        } else {
            0
        };
    }
    e.mem.set_u32(request + 0x30, location);
    if e.vcall(object, 0x100, &args![]).bool() {
        let mut mask = e.mem.u32(request + 0x6c);
        for bit in 0..0xf {
            mask |= 1 << (bit + 1);
        }
        e.mem.set_u32(request + 0x6c, mask);
    }
    e.mem.set_u32(request + 0x34, 0xffff_ffff);
    let value = e.call(DECAL_FLOAT, &args![]).f32();
    e.mem.set_f32(request + 0x44, value);
    let upper = e.global::<f32>(FLOAT_ROUND_RANGE);
    let random = e.call(RANDOM_FLOAT, &args![0.0f32, upper]).f32();
    let rounded = e.call(ROUND_FLOAT, &args![random]).f64();
    let byte = e.call(FTOL, &args![rounded]).u8();
    e.mem.set_u8(request + 0x70, byte);
    let high = e.call(RANGE_HIGH, &args![found]).f32();
    let low = e.call(RANGE_LOW, &args![found]).f32();
    let random = e.call(RANDOM_FLOAT, &args![low, high]).f32();
    e.mem.set_f32(request + 0x3c, random);
    e.mem.set_f32(request + 0x38, random);
    let thousand = e.global::<f32>(FLOAT_DECAL_1000);
    e.mem.set_f32(request + 0x40, thousand);
    let ninety = e.global::<f32>(FLOAT_DECAL_90);
    e.mem.set_f32(request + 0x58, ninety);
    let has_word = e.call(WORD_AT_0XBC, &args![held]).u32() != 0;
    if has_word || e.vcall(object, 0x100, &args![]).bool() {
        let direction = e.mem.alloc(12);
        let result = e.call(CAMERA_DIRECTION, &args![camera, direction]).u32();
        copy_words(e, result, request + 0xc, 3);
        e.mem.free(direction);
        e.mem.set_u32(request + 0x24, object);
        e.call(DECAL_REQUEST_SUBMIT, &args![world, request, 2u32, 1u32]);
    } else {
        let direction = e.mem.alloc(12);
        let negated = e.mem.alloc(12);
        let result = e.call(CAMERA_DIRECTION, &args![camera, direction]).u32();
        let result = e.call(VECTOR_NEGATE, &args![result, negated]).u32();
        copy_words(e, result, request + 0xc, 3);
        e.mem.free(negated);
        e.mem.free(direction);
        let linked = e.vcall(object, 0x1d0, &args![]).u32();
        e.mem.set_u32(request + 0x28, linked);
        e.call(DECAL_REQUEST_SUBMIT, &args![world, request, 1u32, 1u32]);
    }
}

// Translated from 005c02f0 (decompiled, FalloutNV.exe 1.4.0.525)
/// The word `011ca830`.
pub fn fn_005c02f0(e: &mut Engine) -> u32 {
    e.global::<u32>(GLOBAL_011CA830)
}

// Translated from 005c0300 (decompiled, FalloutNV.exe 1.4.0.525)
/// Parses three integers (default 0), builds the colour `(r / 256, g / 256,
/// b / 256, 1.0)` and stores its four floats at `011fa090`. True whenever
/// the parameters parse.
pub fn fn_005c0300(e: &mut Engine, a: ScriptArgs) -> bool {
    let Some([red, green, blue]) = parse_into(e, a, [0, 0, 0]) else {
        return false;
    };
    let scale = e.global::<f64>(DOUBLE_256);
    let channel = |value: u32| (value as i32 as f64 / scale) as f32;
    let color = e.mem.alloc(16);
    let result = e
        .call(
            COLOR_CONSTRUCT,
            &args![color, channel(red), channel(green), channel(blue), 1.0f32],
        )
        .u32();
    copy_words(e, result, GLOBAL_COLOR, 4);
    e.mem.free(color);
    true
}

// Translated from 005c03d0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Script::RotateFunction` (Xbox PDB): parses an axis letter (`X`, `Y` or
/// `Z`) and an integer number of degrees. For a reference with a slot
/// `0x1d0` object, adds `degrees * (pi / 180) * (the float of 0084d030 on
/// 011f6394)` to that axis of the reference's rotation, wraps it into
/// `0..2pi`, stores it (`SetAngleOnReference`), copies the new orientation
/// to the object and, unless slot `0x1e4` says otherwise, updates the node.
/// True whenever the parameters parse.
pub fn script_rotate_function(e: &mut Engine, a: ScriptArgs) -> bool {
    let Some([axis, degrees]) = parse_into(e, a, [0, 0]) else {
        return false;
    };
    let axis = axis as u8;
    if a.this_obj.is_null() {
        return true;
    }
    let object = a.this_obj.addr();
    let node = e.vcall(object, 0x1d0, &args![]).u32();
    if node == 0 {
        return true;
    }
    let mut angle = 0.0f32;
    if matches!(axis, b'X' | b'Y' | b'Z') {
        let rotation = e.call(REFERENCE_ROTATION, &args![a.this_obj]).u32();
        angle = e.mem.f32(rotation + 4 * (axis - b'X') as u32);
    }
    let radians = degrees as i32 as f64 * e.global::<f64>(DOUBLE_RADIANS_PER_DEGREE);
    let frame = e.call(FRAME_DATA_FLOAT, &args![FRAME_DATA]).f32();
    angle = (frame as f64 * radians + angle as f64) as f32;
    let two_pi = e.global::<f64>(DOUBLE_TWO_PI);
    while angle as f64 >= two_pi {
        angle = (angle as f64 - two_pi) as f32;
    }
    while (angle as f64) < e.global::<f64>(DOUBLE_ZERO) {
        angle = (angle as f64 + two_pi) as f32;
    }
    set_reference_angle(e, a.this_obj, axis, angle);
    let orientation = e.mem.alloc(0x24);
    let result = e
        .call(REFERENCE_GET_ORIENTATION, &args![a.this_obj, orientation])
        .u32();
    e.call(NODE_SET_ORIENTATION, &args![node, result]);
    e.mem.free(orientation);
    if e.vcall(object, 0x1e4, &args![]).u32() == 0 {
        update_node(e, node);
    }
    true
}

// Translated from 005c0590 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Script::GetPosFunction` (Xbox PDB): parses an axis letter and forwards
/// to `GetPosConditionFunction(thisObj, axis, 0, result)`.
pub fn script_get_pos_function(e: &mut Engine, a: ScriptArgs) -> bool {
    get_axis_command(e, a, GET_POS_CONDITION)
}

// Translated from 005c05f0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Script::SetPosFunction` (Xbox PDB): parses an axis letter and a float
/// (default 0.0). A reference that cannot be moved (`00572c80`) is reported
/// (see [`move_command`]); otherwise the position is set through the task
/// queue (task `0x1007`) when the setting `011c3ea4` is above 1, or by
/// [`script_set_pos_base`]. True whenever the parameters parse.
pub fn script_set_pos_function(e: &mut Engine, a: ScriptArgs) -> bool {
    let Some([axis, amount]) = parse_into(e, a, [0, 0]) else {
        return false;
    };
    move_command(
        e,
        a,
        TASK_SET_POSITION,
        script_set_pos_base,
        axis as u8,
        f32::from_bits(amount),
    )
}

// Translated from 005c0740 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Script::SetPosBase` (Xbox PDB): sets one coordinate (`X`, `Y` or `Z`)
/// of a reference's position, taken from slot `0x1f4`: the position goes
/// through `SetLocationOnReference`, the character controller of a
/// `MobileObject` (unless `005c0860` says so) is moved too, and the slot
/// `0x1d0` object gets the position, a simulation reset and a node update.
/// Nothing for a null reference.
pub fn script_set_pos_base(e: &mut Engine, reference: Ptr, axis: u8, value: f32) {
    if reference.is_null() {
        return;
    }
    let object = reference.addr();
    let position = e.vcall(object, 0x1f4, &args![]).u32();
    let vector = e.mem.alloc(12);
    copy_words(e, position, vector, 3);
    match axis {
        b'X' => e.mem.set_f32(vector, value),
        b'Y' => e.mem.set_f32(vector + 4, value),
        b'Z' => e.mem.set_f32(vector + 8, value),
        _ => {}
    }
    e.call(REFERENCE_SET_LOCATION, &args![reference, vector]);
    let mobile = e
        .call(
            DYNAMIC_CAST,
            &args![
                reference,
                0u32,
                RTTI_TES_OBJECT_REFR,
                RTTI_MOBILE_OBJECT,
                0u32
            ],
        )
        .u32();
    if mobile != 0 {
        let controller = e.call(GET_CHAR_CONTROLLER, &args![mobile]).u32();
        if controller != 0 && !fn_005c0860(e, Ptr::new(controller)) {
            e.call(CONTROLLER_SET_POSITION, &args![controller, vector]);
        }
    }
    let node = e.vcall(object, 0x1d0, &args![]).u32();
    if node != 0 {
        e.call(NODE_SET_POSITION, &args![node, vector]);
        e.call(COLLISION_RESET_SIM, &args![node, 1u32]);
        update_node(e, node);
    }
    e.mem.free(vector);
}

// Translated from 005c0860 (decompiled, FalloutNV.exe 1.4.0.525)
/// Whether [`fn_005c0880`] answers 4.
pub fn fn_005c0860(e: &mut Engine, this: Ptr) -> bool {
    fn_005c0880(e, this) == 4
}

// Translated from 005c0880 (decompiled, FalloutNV.exe 1.4.0.525)
/// `00ca9600` on the part at `this + 0x3e0` (a character controller's
/// state).
pub fn fn_005c0880(e: &mut Engine, this: Ptr) -> u32 {
    e.call(
        CONTROLLER_PART_VALUE,
        &args![this.addr().wrapping_add(0x3e0)],
    )
    .u32()
}

// Translated from 005c08a0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Script::GetStartingPosFunction` (Xbox PDB): parses an axis letter and
/// forwards to `GetStartingPosConditionFunction(thisObj, axis, 0, result)`.
pub fn script_get_starting_pos_function(e: &mut Engine, a: ScriptArgs) -> bool {
    get_axis_command(e, a, GET_STARTING_POS_CONDITION)
}

// Translated from 005c0900 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Script::GetAngleFunction` (Xbox PDB): parses an axis letter and forwards
/// to `GetAngleConditionFunction(thisObj, axis, 0, result)`.
pub fn script_get_angle_function(e: &mut Engine, a: ScriptArgs) -> bool {
    get_axis_command(e, a, GET_ANGLE_CONDITION)
}

// Translated from 005c0960 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Script::GetStartingAngleFunction` (Xbox PDB): parses an axis letter and
/// forwards to `GetStartingAngleConditionFunction(thisObj, axis, 0,
/// result)`.
pub fn script_get_starting_angle_function(e: &mut Engine, a: ScriptArgs) -> bool {
    get_axis_command(e, a, GET_STARTING_ANGLE_CONDITION)
}

// Translated from 005c09c0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Script::SetAngleFunction` (Xbox PDB): as [`script_set_pos_function`]
/// for an angle in degrees (task `0x1009`, [`script_set_angle_base`]).
pub fn script_set_angle_function(e: &mut Engine, a: ScriptArgs) -> bool {
    let Some([axis, amount]) = parse_into(e, a, [0, 0]) else {
        return false;
    };
    move_command(
        e,
        a,
        TASK_SET_ANGLE,
        script_set_angle_base,
        axis as u8,
        f32::from_bits(amount),
    )
}

// Translated from 005c0b10 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Script::SetAngleBase` (Xbox PDB): converts `degrees` to radians and
/// stores them as one angle (`X`, `Y` or `Z`) of the reference, copies the
/// orientation to its slot `0x1d0` object, resets the simulation and, unless
/// slot `0x1e4` says otherwise, updates the node. Nothing for a null
/// reference.
pub fn script_set_angle_base(e: &mut Engine, reference: Ptr, axis: u8, degrees: f32) {
    if reference.is_null() {
        return;
    }
    let radians = (degrees as f64 * e.global::<f64>(DOUBLE_RADIANS_PER_DEGREE)) as f32;
    set_reference_angle(e, reference, axis, radians);
    let object = reference.addr();
    let node = e.vcall(object, 0x1d0, &args![]).u32();
    if node == 0 {
        return;
    }
    let orientation = e.mem.alloc(0x24);
    let result = e
        .call(REFERENCE_GET_ORIENTATION, &args![reference, orientation])
        .u32();
    e.call(NODE_SET_ORIENTATION, &args![node, result]);
    e.mem.free(orientation);
    e.call(COLLISION_RESET_SIM, &args![node, 1u32]);
    if e.vcall(object, 0x1e4, &args![]).u32() == 0 {
        update_node(e, node);
    }
}

// Translated from 005c0bf0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Script::SetAtStartFunction` (Xbox PDB): puts a reference back at its
/// start. For a reference with a slot `0x1d0` object: calls slot `0x174`
/// with the zero vector, builds the orientation as the product of the
/// rotations about X, Y and Z by the three angles of slot `0x16c` (each new
/// rotation multiplied by the running matrix, which starts as the identity),
/// gives the object the position of slot `0x170` and that orientation, and
/// unless slot `0x1e4` says otherwise updates the node. True.
pub fn script_set_at_start_function(e: &mut Engine, a: ScriptArgs) -> bool {
    if a.this_obj.is_null() {
        return true;
    }
    let object = a.this_obj.addr();
    let node = e.vcall(object, 0x1d0, &args![]).u32();
    if node == 0 {
        return true;
    }
    let rotation = e.mem.alloc(0x24);
    let running = e.mem.alloc(0x24);
    e.call(EMPTY_CONSTRUCT, &args![rotation]);
    e.call(EMPTY_CONSTRUCT, &args![running]);
    copy_words(e, IDENTITY_MATRIX, running, 9);
    let zero = [
        e.mem.u32(ZERO_VECTOR),
        e.mem.u32(ZERO_VECTOR + 4),
        e.mem.u32(ZERO_VECTOR + 8),
    ];
    e.vcall(object, 0x174, &args![zero[0], zero[1], zero[2]]);
    for (index, make_rotation) in [
        MATRIX_MAKE_X_ROTATION,
        MATRIX_MAKE_Y_ROTATION,
        MATRIX_MAKE_Z_ROTATION,
    ]
    .into_iter()
    .enumerate()
    {
        let out = e.mem.alloc(12);
        let angles = e.vcall(object, 0x16c, &args![out]).u32();
        let angle = e.mem.f32(angles + 4 * index as u32);
        e.call(make_rotation, &args![rotation, angle]);
        let product_out = e.mem.alloc(0x24);
        let product = e
            .call(MATRIX_MULTIPLY, &args![rotation, product_out, running])
            .u32();
        copy_words(e, product, running, 9);
        e.mem.free(product_out);
        e.mem.free(out);
    }
    let out = e.mem.alloc(12);
    let position = e.vcall(object, 0x170, &args![out]).u32();
    e.call(NODE_SET_POSITION, &args![node, position]);
    e.mem.free(out);
    e.call(NODE_SET_ORIENTATION, &args![node, running]);
    if e.vcall(object, 0x1e4, &args![]).u32() == 0 {
        update_node(e, node);
    }
    e.mem.free(running);
    e.mem.free(rotation);
    true
}

/// This part's translated functions, by exe address.
pub fn funcs() -> Vec<(u32, AbiFn)> {
    vec![
        entry!(0x005bc430, script_toggle_low_process() -> bool),
        entry!(0x005bc4a0, script_toggle_middle_low_process() -> bool),
        entry!(0x005bc510, script_toggle_middle_high_process() -> bool),
        entry!(0x005bc580, script_toggle_ai_schedules() -> bool),
        entry!(0x005bc5f0, script_speak_sound_function(ScriptArgs) -> bool),
        entry!(0x005bc950, script_help_function(ScriptArgs) -> bool),
        entry!(
            0x005bcfe0,
            script_toggle_nav_mesh_function(ScriptArgs) -> bool
        ),
        entry!(0x005bd240, fn_005bd240(ScriptArgs) -> bool),
        entry!(0x005bd300, fn_005bd300(ScriptArgs) -> bool),
        entry!(
            0x005bd3c0,
            script_toggle_pathing_info_function() -> bool
        ),
        entry!(
            0x005bd410,
            script_pick_ref_by_id_function(ScriptArgs) -> bool
        ),
        entry!(0x005bd580, fn_005bd580(Ptr) -> f32),
        entry!(0x005bd5b0, fn_005bd5b0() -> u32),
        entry!(0x005bd5c0, fn_005bd5c0(u32, u32) -> u32),
        entry!(
            0x005bd5e0,
            script_center_on_cell_function(ScriptArgs) -> bool
        ),
        entry!(
            0x005bd660,
            script_center_on_exterior_function(ScriptArgs) -> bool
        ),
        entry!(
            0x005bd780,
            script_center_on_world_function(ScriptArgs) -> bool
        ),
        entry!(0x005bd880, fn_005bd880(ScriptArgs) -> bool),
        entry!(0x005bd8a0, fn_005bd8a0(ScriptArgs) -> bool),
        entry!(0x005bd900, fn_005bd900(ScriptArgs) -> bool),
        entry!(
            0x005bd960,
            script_get_actor_value_info_function(ScriptArgs) -> bool
        ),
        entry!(0x005bdcd0, fn_005bdcd0(ScriptArgs) -> bool),
        entry!(0x005bddb0, script_set_size_function(ScriptArgs) -> bool),
        entry!(0x005bde40, fn_005bde40(ScriptArgs) -> bool),
        entry!(0x005bdf20, fn_005bdf20(ScriptArgs) -> bool),
        entry!(
            0x005be080,
            script_restore_actor_value_function(ScriptArgs) -> bool
        ),
        entry!(0x005be190, fn_005be190(ScriptArgs) -> bool),
        entry!(0x005be2a0, fn_005be2a0(ScriptArgs) -> bool),
        entry!(0x005be4d0, fn_005be4d0(Ptr) -> u32),
        entry!(0x005be4f0, fn_005be4f0(ScriptArgs) -> bool),
        entry!(0x005be5c0, fn_005be5c0(u32, u32) -> u32),
        entry!(0x005be5e0, fn_005be5e0(ScriptArgs) -> bool),
        entry!(0x005be6a0, fn_005be6a0(ScriptArgs) -> bool),
        entry!(0x005be6c0, fn_005be6c0(ScriptArgs) -> bool),
        entry!(
            0x005be6e0,
            script_get_base_actor_value_function(ScriptArgs) -> bool
        ),
        entry!(0x005be740, fn_005be740(ScriptArgs) -> bool),
        entry!(
            0x005be760,
            script_is_limb_gone_function(ScriptArgs) -> bool
        ),
        entry!(0x005be830, fn_005be830(ScriptArgs) -> bool),
        entry!(0x005be850, fn_005be850(ScriptArgs) -> bool),
        entry!(0x005be870, fn_005be870(ScriptArgs) -> bool),
        entry!(0x005be900, fn_005be900(ScriptArgs) -> bool),
        entry!(0x005beb00, fn_005beb00(ScriptArgs) -> bool),
        entry!(0x005becf0, fn_005becf0(ScriptArgs) -> bool),
        entry!(0x005bee90, fn_005bee90(ScriptArgs) -> bool),
        entry!(0x005bf130, fn_005bf130(ScriptArgs) -> bool),
        entry!(0x005bf770, fn_005bf770(u8)),
        entry!(0x005bf7b0, fn_005bf7b0() -> bool),
        entry!(0x005bf7f0, fn_005bf7f0(u32)),
        entry!(0x005bf800, fn_005bf800(Ptr, u8)),
        entry!(0x005bf820, fn_005bf820(Ptr) -> u32),
        entry!(0x005bf840, fn_005bf840(u8)),
        entry!(0x005bf860, fn_005bf860() -> bool),
        entry!(0x005bf880, fn_005bf880() -> bool),
        entry!(0x005bf8b0, fn_005bf8b0() -> u8),
        entry!(0x005bf8c0, fn_005bf8c0(u8)),
        entry!(0x005bf8d0, fn_005bf8d0(ScriptArgs) -> bool),
        entry!(0x005bf9a0, fn_005bf9a0(Ptr, f32, f32)),
        entry!(0x005bf9c0, fn_005bf9c0(Ptr, u8)),
        entry!(0x005bf9e0, fn_005bf9e0(ScriptArgs) -> bool),
        entry!(0x005bfaf0, fn_005bfaf0(ScriptArgs) -> bool),
        entry!(0x005bfbb0, fn_005bfbb0(Ptr, f32)),
        entry!(0x005bfbd0, fn_005bfbd0(Ptr, f32)),
        entry!(
            0x005bfbf0,
            script_set_tree_mipmap_bias(ScriptArgs) -> bool
        ),
        entry!(0x005bfd40, fn_005bfd40() -> bool),
        entry!(0x005bfd50, script_force_close_file(ScriptArgs) -> bool),
        entry!(0x005bfe90, fn_005bfe90(ScriptArgs) -> bool),
        entry!(0x005c02f0, fn_005c02f0() -> u32),
        entry!(0x005c0300, fn_005c0300(ScriptArgs) -> bool),
        entry!(0x005c03d0, script_rotate_function(ScriptArgs) -> bool),
        entry!(0x005c0590, script_get_pos_function(ScriptArgs) -> bool),
        entry!(0x005c05f0, script_set_pos_function(ScriptArgs) -> bool),
        entry!(0x005c0740, script_set_pos_base(Ptr, u8, f32)),
        entry!(0x005c0860, fn_005c0860(Ptr) -> bool),
        entry!(0x005c0880, fn_005c0880(Ptr) -> u32),
        entry!(
            0x005c08a0,
            script_get_starting_pos_function(ScriptArgs) -> bool
        ),
        entry!(0x005c0900, script_get_angle_function(ScriptArgs) -> bool),
        entry!(
            0x005c0960,
            script_get_starting_angle_function(ScriptArgs) -> bool
        ),
        entry!(0x005c09c0, script_set_angle_function(ScriptArgs) -> bool),
        entry!(0x005c0b10, script_set_angle_base(Ptr, u8, f32)),
        entry!(
            0x005c0bf0,
            script_set_at_start_function(ScriptArgs) -> bool
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::{Cell, RefCell};
    use std::rc::Rc;

    // Fake virtual functions the tests put into vtables.
    const V_TRUE: u32 = 0x0900_0001;
    const V_FALSE: u32 = 0x0900_0002;
    /// Returns nothing; the tests read its calls from the log.
    const V_RECORD: u32 = 0x0900_0003;
    /// `V_FLOAT + k` returns the float `k + 0.5`.
    const V_FLOAT: u32 = 0x0900_0010;

    /// An engine with the pages of the globals these commands touch mapped
    /// and the helpers every command uses replaced by doubles that behave
    /// like the exe's code.
    fn engine() -> Engine {
        let mut e = Engine::new();
        for page in [
            0x0101_2000,
            0x0102_1000,
            0x0103_a000,
            0x011c_3000,
            0x011c_6000,
            0x011c_9000,
            0x011d_0000,
            0x011d_6000,
            0x011d_7000,
            0x011d_e000,
            0x011e_0000,
            0x011f_1000,
            0x011f_3000,
        ] {
            e.map(page, 0x1000);
        }
        e.map(0x0118_e000, 0xa000);
        e.register(V_TRUE, |_, _| true.into_ret());
        e.register(V_FALSE, |_, _| false.into_ret());
        e.register(V_RECORD, |_, _| Ret::default());
        for k in 0..32u32 {
            e.register_double(V_FLOAT + k, move |_, _| (k as f32 + 0.5).into_ret());
        }
        e.register(CONSOLE_PRINT, |_, _| Ret::default());
        // The RTTI cast succeeds for objects whose byte +0x50 is set.
        e.register(DYNAMIC_CAST, |e, a| {
            let actor = if a[0] != 0 && e.mem.u8(a[0] + 0x50) != 0 {
                a[0]
            } else {
                0
            };
            actor.into_ret()
        });
        // Actor value 5 is protected, 6 has flag 0x200, 7 has flag 0x80.
        e.register(ACTOR_VALUE_HAS_FLAG, |_, a| {
            let flags = match a[0] {
                5 => 0x4000,
                6 => 0x200,
                7 => 0x80,
                _ => 0,
            };
            (flags & a[1] != 0).into_ret()
        });
        e.register(ACTOR_VALUE_SCRIPT_NAME, |_, a| {
            (0x0a00_0000 + a[0]).into_ret()
        });
        e.register(ABSOLUTE_VALUE, |_, a| f32::from_bits(a[0]).abs().into_ret());
        e.register(FTOL, |_, a| {
            let value = f64::from_bits(a[0] as u64 | (a[1] as u64) << 32);
            (value as i32).into_ret()
        });
        // The real getter returns the address of the value at `this + 4`.
        e.register(SETTING_VALUE_ADDRESS, |_, a| (a[0] + 4).into_ret());
        e.register(MEMORY_ALLOC, |e, a| e.mem.alloc(a[0]).into_ret());
        e
    }

    fn set_echo(e: &mut Engine, on: bool) {
        let tls = e.tls();
        e.mem.set_u8(tls + TLS_ECHO, on as u8);
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

    /// The console prints, as argument words.
    fn printed(e: &Engine) -> Vec<Vec<u32>> {
        calls(e, CONSOLE_PRINT)
    }

    /// Registers doubles that just accept the call.
    fn accept(e: &mut Engine, addrs: &[u32]) {
        for addr in addrs {
            e.register(*addr, |_, _| Ret::default());
        }
    }

    /// A heap C string.
    fn text(e: &mut Engine, s: &str) -> u32 {
        let address = e.mem.alloc(s.len() as u32 + 1);
        e.mem.set_cstr(address, s.as_bytes());
        address
    }

    /// A vtable (in the heap) with the given `(byte offset, function)` slots.
    fn vtable(e: &mut Engine, slots: &[(u32, u32)]) -> u32 {
        let vtable = e.mem.alloc(0x500);
        for (offset, function) in slots {
            e.mem.set_u32(vtable + offset, *function);
        }
        vtable
    }

    /// An object whose vtable has the given slots.
    fn object_with(e: &mut Engine, slots: &[(u32, u32)]) -> u32 {
        let table = vtable(e, slots);
        let object = e.mem.alloc(0x800);
        e.mem.set_u32(object, table);
        object
    }

    /// An object the RTTI double casts to an actor, answering true to
    /// virtual slot `0x100`.
    fn actor_with(e: &mut Engine, slots: &[(u32, u32)]) -> u32 {
        let mut all = vec![(0x100, V_TRUE)];
        all.extend_from_slice(slots);
        let actor = object_with(e, &all);
        e.mem.set_u8(actor + 0x50, 1);
        actor
    }

    /// The standard eight words with `this_obj` set.
    fn script(this_obj: u32) -> ScriptArgs {
        ScriptArgs {
            param_info: 1,
            script_data: 2,
            this_obj: Ptr::new(this_obj),
            containing_obj: Ptr::NULL,
            script_obj: Ptr::new(5),
            event_list: 6,
            result: Ptr::NULL,
            opcode_offset: 8,
        }
    }

    /// `ParseParameters` double: returns `ok` after calling `fill` with the
    /// argument words (the output pointers start at word 7).
    fn parse_with(e: &mut Engine, ok: bool, fill: impl Fn(&mut Engine, &[u32]) + 'static) {
        e.register_double(PARSE_PARAMETERS, move |e, a| {
            fill(e, a);
            ok.into_ret()
        });
    }

    /// `ParseParameters` double that stores `outs` through its output
    /// pointers (word-sized locals) and returns `ok`.
    fn parse_gives(e: &mut Engine, ok: bool, outs: &[u32]) {
        let outs = outs.to_vec();
        parse_with(e, ok, move |e, a| {
            for (i, value) in outs.iter().enumerate() {
                e.mem.set_u32(a[7 + i], *value);
            }
        });
    }

    fn run(e: &mut Engine, address: u32, a: ScriptArgs) -> bool {
        e.call(address, &args![a]).bool()
    }

    // ---- AI toggles -------------------------------------------------------

    /// Flips the byte twice with echo on and once with echo off.
    fn check_toggle(address: u32, flag: u32, message: u32) {
        let mut e = engine();
        set_echo(&mut e, true);
        start_log(&mut e);
        assert!(e.call(address, &args![]).bool());
        assert_eq!(e.global::<u8>(flag), 1);
        assert!(e.call(address, &args![]).bool());
        assert_eq!(e.global::<u8>(flag), 0);
        assert_eq!(
            printed(&e),
            vec![vec![message, TEXT_ON], vec![message, TEXT_OFF]]
        );
        set_echo(&mut e, false);
        start_log(&mut e);
        assert!(e.call(address, &args![]).bool());
        assert_eq!(e.global::<u8>(flag), 1);
        assert!(printed(&e).is_empty());
    }

    #[test]
    fn toggle_low_process_flips_the_flag_and_reports_it() {
        check_toggle(0x005b_c430, AI_LOW_PROCESS, MSG_AI_LOW);
    }

    #[test]
    fn toggle_middle_low_process_flips_the_flag_and_reports_it() {
        check_toggle(0x005b_c4a0, AI_MIDDLE_LOW_PROCESS, MSG_AI_MIDDLE_LOW);
    }

    #[test]
    fn toggle_middle_high_process_flips_the_flag_and_reports_it() {
        check_toggle(0x005b_c510, AI_MIDDLE_HIGH_PROCESS, MSG_AI_MIDDLE_HIGH);
    }

    #[test]
    fn toggle_ai_schedules_flips_the_flag_and_reports_it() {
        check_toggle(0x005b_c580, AI_SCHEDULES, MSG_AI_SCHEDULES);
    }

    // ---- SpeakSound ---------------------------------------------------------

    /// Doubles for the callees of `005bc5f0`; returns the actor and its
    /// process. The actor's slot `0x284` answers 2.5.
    fn speak_engine(e: &mut Engine) -> (u32, u32) {
        let process = object_with(e, &[(0x310, V_RECORD), (0x318, V_RECORD)]);
        let actor = actor_with(e, &[(0x284, V_FLOAT + 2)]);
        accept(
            e,
            &[
                SPEAK_RESULT_CONSTRUCT,
                SPEAK_RESULT_DESTRUCT,
                BS_STRING_CONSTRUCT,
                BS_STRING_DESTRUCT,
                STRING_COPY,
                BS_STRING_FORMAT,
            ],
        );
        e.register_double(ACTOR_PROCESS, move |_, _| process.into_ret());
        e.register(CURRENT_PROCESS_TYPE, |_, _| 0u32.into_ret());
        e.register(STRING_TEXT, |_, _| 0x7000u32.into_ret());
        (actor, process)
    }

    #[test]
    fn speak_sound_clamps_the_numbers_and_makes_the_actor_speak() {
        let mut e = engine();
        let (actor, process) = speak_engine(&mut e);
        set_echo(&mut e, true);
        // The name, a variation out of range (-> 0) and a volume above 100.
        parse_with(&mut e, true, |e, a| {
            e.mem.set_cstr(a[7], b"boom");
            e.mem.set_i32(a[8], 9);
            e.mem.set_i32(a[9], 200);
        });
        start_log(&mut e);
        assert!(run(&mut e, 0x005b_c5f0, script(actor)));
        let result_object = calls(&e, SPEAK_RESULT_CONSTRUCT)[0][0];
        let string = calls(&e, BS_STRING_CONSTRUCT)[0][0];
        let name = calls(&e, BS_STRING_FORMAT)[0][2];
        assert_eq!(
            calls(&e, BS_STRING_FORMAT),
            vec![vec![string, FORMAT_STRING, name]]
        );
        let copy = calls(&e, STRING_COPY);
        assert_eq!(copy, vec![vec![copy[0][0], 0x200, name]]);
        assert_eq!(
            calls(&e, V_FLOAT + 2),
            vec![vec![
                actor,
                0x7000,
                result_object,
                0,
                100,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                1,
                1
            ]]
        );
        // The process gets slot 0x310(1), then slot 0x318(duration 2.5).
        assert_eq!(
            calls(&e, V_RECORD),
            vec![vec![process, 1], vec![process, 2.5f32.to_bits()]]
        );
        assert_eq!(printed(&e), vec![vec![MSG_NPC_SPEAKS]]);
        assert_eq!(calls(&e, BS_STRING_DESTRUCT), vec![vec![string]]);
        assert_eq!(calls(&e, SPEAK_RESULT_DESTRUCT), vec![vec![result_object]]);
    }

    #[test]
    fn speak_sound_keeps_the_defaults_and_stays_quiet_without_echo() {
        let mut e = engine();
        let (actor, _) = speak_engine(&mut e);
        set_echo(&mut e, false);
        parse_with(&mut e, true, |e, a| e.mem.set_cstr(a[7], b"x"));
        start_log(&mut e);
        assert!(run(&mut e, 0x005b_c5f0, script(actor)));
        let words = &calls(&e, V_FLOAT + 2)[0];
        // Variation 0 and volume 50 (0x32), the defaults.
        assert_eq!(words[3..5], [0, 0x32]);
        assert!(printed(&e).is_empty());
        // A negative variation is replaced by 0 and a negative volume by 0.
        parse_with(&mut e, true, |e, a| {
            e.mem.set_i32(a[8], -3);
            e.mem.set_i32(a[9], -4);
        });
        start_log(&mut e);
        assert!(run(&mut e, 0x005b_c5f0, script(actor)));
        assert_eq!(calls(&e, V_FLOAT + 2)[0][3..5], [0, 0]);
    }

    #[test]
    fn speak_sound_fails_cleanly() {
        let mut e = engine();
        let (actor, _) = speak_engine(&mut e);
        // Parameters that do not parse; both temporaries are still destroyed.
        parse_gives(&mut e, false, &[]);
        start_log(&mut e);
        assert!(!run(&mut e, 0x005b_c5f0, script(actor)));
        assert!(calls(&e, V_FLOAT + 2).is_empty());
        assert_eq!(calls(&e, BS_STRING_DESTRUCT).len(), 1);
        assert_eq!(calls(&e, SPEAK_RESULT_DESTRUCT).len(), 1);
        parse_gives(&mut e, true, &[]);
        // No reference, not an actor.
        start_log(&mut e);
        assert!(!run(&mut e, 0x005b_c5f0, script(0)));
        let plain = object_with(&mut e, &[]);
        assert!(!run(&mut e, 0x005b_c5f0, script(plain)));
        // An actor without a process, and one in a process of another type.
        e.register(ACTOR_PROCESS, |_, _| 0u32.into_ret());
        assert!(!run(&mut e, 0x005b_c5f0, script(actor)));
        e.register(ACTOR_PROCESS, |_, _| 0x1234u32.into_ret());
        e.register(CURRENT_PROCESS_TYPE, |_, _| 2u32.into_ret());
        assert!(!run(&mut e, 0x005b_c5f0, script(actor)));
        assert!(calls(&e, V_FLOAT + 2).is_empty());
        assert_eq!(calls(&e, BS_STRING_DESTRUCT).len(), 4);
    }

    // ---- Help ------------------------------------------------------------------

    /// Doubles for the string functions `HelpFunction` uses.
    fn help_engine() -> Engine {
        let mut e = engine();
        e.register(STRLEN, |e, a| (e.mem.cstr(a[0]).len() as u32).into_ret());
        e.register(STRISTR, |e, a| {
            if a[0] == 0 || a[1] == 0 {
                return 0u32.into_ret();
            }
            let haystack = e.mem.cstr(a[0]).to_ascii_lowercase();
            let needle = e.mem.cstr(a[1]).to_ascii_lowercase();
            haystack
                .windows(needle.len().max(1))
                .any(|w| w == needle.as_slice())
                .into_ret()
        });
        e
    }

    fn set_entry(e: &mut Engine, table: u32, index: u32, name: &str, short: &str, help: &str) {
        let entry = table + index * HELP_ENTRY_SIZE;
        for (offset, value) in [(0, name), (4, short), (0xc, help)] {
            let pointer = if value.is_empty() { 0 } else { text(e, value) };
            e.mem.set_u32(entry + offset, pointer);
        }
    }

    fn entry_word(e: &Engine, table: u32, index: u32, offset: u32) -> u32 {
        e.mem.u32(table + index * HELP_ENTRY_SIZE + offset)
    }

    #[test]
    fn help_without_a_filter_lists_everything_in_the_four_formats() {
        let mut e = help_engine();
        set_entry(&mut e, COMMAND_TABLE, 0, "Alpha", "al", "does a");
        set_entry(&mut e, COMMAND_TABLE, 1, "Beta", "", "");
        set_entry(&mut e, COMMAND_TABLE, 2, "Gamma", "", "does c");
        set_entry(&mut e, COMMAND_TABLE, 3, "Delta", "dl", "");
        parse_gives(&mut e, true, &[]);
        start_log(&mut e);
        assert!(run(&mut e, 0x005b_c950, script(0)));
        let prints = printed(&e);
        let word = |index, offset| entry_word(&e, COMMAND_TABLE, index, offset);
        assert_eq!(prints[0], vec![HEADER_CONSOLE_COMMANDS]);
        assert_eq!(
            prints[1],
            vec![FORMAT_NAME_SHORT_HELP, word(0, 0), word(0, 4), word(0, 0xc)]
        );
        assert_eq!(prints[2], vec![FORMAT_STRING, word(1, 0)]);
        assert_eq!(prints[3], vec![FORMAT_NAME_HELP, word(2, 0), word(2, 0xc)]);
        assert_eq!(prints[4], vec![FORMAT_NAME_SHORT, word(3, 0), word(3, 4)]);
        // 205 commands, a header, 640 script functions; no settings.
        assert_eq!(prints.len(), 1 + 205 + 1 + 640);
        assert_eq!(prints[206], vec![HEADER_SCRIPT_FUNCTIONS]);
        assert!(!prints.iter().any(|p| p[0] == HEADER_GAME_SETTINGS));
    }

    #[test]
    fn help_with_a_filter_lists_the_matches_and_the_settings() {
        let mut e = help_engine();
        set_entry(&mut e, COMMAND_TABLE, 0, "Alpha", "al", "list a");
        set_entry(&mut e, COMMAND_TABLE, 1, "Beta", "", "");
        set_entry(&mut e, COMMAND_TABLE, 2, "Gamma", "g", "ALPine help");
        set_entry(&mut e, FUNCTION_TABLE, 5, "Delta", "", "has alp inside");
        parse_with(&mut e, true, |e, a| e.mem.set_cstr(a[7], b"alp"));
        // The two name collections: which one is being built decides the
        // names the `COLLECTION_*` doubles hand out.
        let stage = Rc::new(Cell::new(0u32));
        let game_names = [
            text(&mut e, "skipped alp"),
            text(&mut e, "my alpha"),
            text(&mut e, "nomatch"),
            0,
        ];
        let ini_names = [text(&mut e, "first alp")];
        let game_cells = e.mem.alloc(16);
        for (i, name) in game_names.iter().enumerate() {
            e.mem.set_u32(game_cells + 4 * i as u32, *name);
        }
        let ini_cells = e.mem.alloc(4);
        e.mem.set_u32(ini_cells, ini_names[0]);
        let counter = stage.clone();
        e.register_double(COLLECTION_CONSTRUCT, move |_, _| {
            counter.set(counter.get() + 1);
            Ret::default()
        });
        let counter = stage.clone();
        e.register_double(COLLECTION_COUNT, move |_, _| {
            (if counter.get() == 1 { 4u32 } else { 1 }).into_ret()
        });
        let counter = stage.clone();
        e.register_double(COLLECTION_ELEMENT_ADDRESS, move |_, a| {
            let base = if counter.get() == 1 {
                game_cells
            } else {
                ini_cells
            };
            (base + 4 * a[1]).into_ret()
        });
        accept(&mut e, &[COLLECTION_DESTRUCT, MEMORY_FREE]);
        let settings = object_with(&mut e, &[(0xc, V_RECORD)]);
        e.register_double(GAME_SETTINGS, move |_, _| settings.into_ret());
        e.register_double(INI_SETTINGS, move |_, _| settings.into_ret());
        start_log(&mut e);
        assert!(run(&mut e, 0x005b_c950, script(0)));
        let prints = printed(&e);
        assert_eq!(prints[0], vec![HEADER_CONSOLE_COMMANDS]);
        assert_eq!(
            prints[1][..2],
            [FORMAT_NAME_SHORT_HELP, entry_word(&e, COMMAND_TABLE, 0, 0)]
        );
        assert_eq!(
            prints[2][..2],
            [FORMAT_NAME_SHORT_HELP, entry_word(&e, COMMAND_TABLE, 2, 0)]
        );
        assert_eq!(prints[3], vec![HEADER_SCRIPT_FUNCTIONS]);
        assert_eq!(
            prints[4][..2],
            [FORMAT_NAME_HELP, entry_word(&e, FUNCTION_TABLE, 5, 0)]
        );
        assert_eq!(prints[5], vec![HEADER_GAME_SETTINGS]);
        // Index 0 is never printed; the matching name after it is.
        assert_eq!(prints[6], vec![FORMAT_STRING, game_names[1]]);
        assert_eq!(prints[7], vec![HEADER_INI_SETTINGS]);
        assert_eq!(prints.len(), 8);
        // Every non-null name is released; the collections are destroyed in
        // reverse order.
        assert_eq!(
            calls(&e, MEMORY_FREE),
            vec![
                vec![game_names[0]],
                vec![game_names[1]],
                vec![game_names[2]],
                vec![ini_names[0]]
            ]
        );
        let destroyed = calls(&e, COLLECTION_DESTRUCT);
        assert_eq!(destroyed.len(), 2);
        assert_eq!(destroyed[0][0], destroyed[1][0] + 0x10);
        assert_eq!(calls(&e, COLLECTION_CONSTRUCT)[0][1..], [0, 1]);
    }

    // ---- Navigation mesh ----------------------------------------------------------

    /// Doubles for the render singleton: a heap object whose byte `+0x210`
    /// is "drawing". Returns the render object.
    fn nav_engine(e: &mut Engine, drawing: bool) -> u32 {
        let render = e.mem.alloc(0x400);
        e.mem.set_u8(render + 0x210, drawing as u8);
        e.register_double(NAV_MESH_RENDER, move |_, _| render.into_ret());
        e.register(NAV_MESH_RENDER_IS_DRAWING, |e, a| {
            e.mem.u8(a[0] + 0x210).into_ret()
        });
        accept(
            e,
            &[
                NAV_MESH_RENDER_SET_DRAW,
                NAV_MESH_RENDER_SET_TRANSPARENT,
                NAV_MESH_RENDER_ADD_CELL,
            ],
        );
        render
    }

    #[test]
    fn nav_mesh_defaults_to_the_toggle_and_adds_the_interior_cell() {
        let mut e = engine();
        let render = nav_engine(&mut e, false);
        e.set_global(TES_SINGLETON, 0x1111u32);
        e.register(GET_CURRENT_CELL, |_, a| {
            assert_eq!(a[0], 0x1111);
            0x2222u32.into_ret()
        });
        e.register(CELL_IS_INTERIOR, |_, _| true.into_ret());
        // The argument keeps its default (4) and drawing is off: mode 1.
        parse_gives(&mut e, true, &[4]);
        start_log(&mut e);
        assert!(run(&mut e, 0x005b_cfe0, script(0)));
        assert_eq!(e.global::<u8>(NAV_MESH_ENABLED), 1);
        assert_eq!(e.global::<u32>(NAV_MESH_MODE), 1);
        assert_eq!(
            calls(&e, NAV_MESH_RENDER_SET_TRANSPARENT),
            vec![vec![render, 0]]
        );
        assert_eq!(
            calls(&e, NAV_MESH_RENDER_ADD_CELL),
            vec![vec![render, 0x2222, 0]]
        );
        assert_eq!(calls(&e, NAV_MESH_RENDER_SET_DRAW), vec![vec![render, 1]]);
        // A mode above 6 is 4 again; with drawing on it becomes 0: off.
        let mut e = engine();
        let render = nav_engine(&mut e, true);
        parse_gives(&mut e, true, &[99]);
        start_log(&mut e);
        assert!(run(&mut e, 0x005b_cfe0, script(0)));
        assert_eq!(e.global::<u8>(NAV_MESH_ENABLED), 0);
        assert_eq!(calls(&e, NAV_MESH_RENDER_SET_DRAW), vec![vec![render, 0]]);
        assert!(calls(&e, NAV_MESH_RENDER_ADD_CELL).is_empty());
        // Parameters that do not parse.
        parse_gives(&mut e, false, &[]);
        assert!(!run(&mut e, 0x005b_cfe0, script(0)));
    }

    #[test]
    fn nav_mesh_adds_the_grid_around_an_exterior_cell() {
        let mut e = engine();
        let render = nav_engine(&mut e, false);
        e.set_global(TES_SINGLETON, 0x1111u32);
        // `uGridsToLoad` is 3: one cell each way.
        e.set_global(GRIDS_TO_LOAD_SETTING + 4, 3u32);
        e.register(GET_CURRENT_CELL, |_, _| 0x2222u32.into_ret());
        e.register(CELL_IS_INTERIOR, |_, _| false.into_ret());
        e.register(CELL_GET_DATA_X, |_, _| 10u32.into_ret());
        e.register(CELL_GET_DATA_Y, |_, _| 20u32.into_ret());
        e.register(CELL_GET_WORLD_SPACE, |_, _| 0x5000u32.into_ret());
        e.register(WORLD_SPACE_GET_CELL, |_, a| {
            assert_eq!(a[0], 0x5000);
            (a[1] * 1000 + a[2]).into_ret()
        });
        parse_gives(&mut e, true, &[2]);
        start_log(&mut e);
        assert!(run(&mut e, 0x005b_cfe0, script(0)));
        // Mode 2 selects the transparent overlay.
        assert_eq!(
            calls(&e, NAV_MESH_RENDER_SET_TRANSPARENT),
            vec![vec![render, 1]]
        );
        let added: Vec<u32> = calls(&e, NAV_MESH_RENDER_ADD_CELL)
            .iter()
            .map(|w| w[1])
            .collect();
        assert_eq!(
            added,
            vec![9019, 9020, 9021, 10019, 10020, 10021, 11019, 11020, 11021]
        );
        let last = e.call_log.as_ref().unwrap().last().unwrap().clone();
        assert_eq!(last.0, NAV_MESH_RENDER_SET_DRAW);
        // No current cell: only drawing is switched on.
        e.register(GET_CURRENT_CELL, |_, _| 0u32.into_ret());
        start_log(&mut e);
        assert!(run(&mut e, 0x005b_cfe0, script(0)));
        assert!(calls(&e, NAV_MESH_RENDER_ADD_CELL).is_empty());
        assert_eq!(calls(&e, NAV_MESH_RENDER_SET_DRAW), vec![vec![render, 1]]);
    }

    #[test]
    fn nav_mesh_modes_5_and_6_flip_their_option_bytes() {
        // Mode 5 with drawing on: back to the remembered mode, drawing off,
        // then re-enabled in that mode.
        let mut e = engine();
        let render = nav_engine(&mut e, true);
        e.set_global(NAV_MESH_MODE, 2u32);
        e.register(GET_CURRENT_CELL, |_, _| 0u32.into_ret());
        parse_gives(&mut e, true, &[5]);
        start_log(&mut e);
        assert!(run(&mut e, 0x005b_cfe0, script(0)));
        assert_eq!(e.global::<u8>(NAV_MESH_TOGGLE_5), 1);
        assert_eq!(e.global::<u32>(NAV_MESH_MODE), 2);
        assert_eq!(
            calls(&e, NAV_MESH_RENDER_SET_DRAW),
            vec![vec![render, 0], vec![render, 1]]
        );
        // Mode 5 with drawing off: mode 0, display off.
        let mut e = engine();
        let render = nav_engine(&mut e, false);
        parse_gives(&mut e, true, &[5]);
        start_log(&mut e);
        assert!(run(&mut e, 0x005b_cfe0, script(0)));
        assert_eq!(e.global::<u8>(NAV_MESH_TOGGLE_5), 1);
        assert_eq!(e.global::<u8>(NAV_MESH_ENABLED), 0);
        assert_eq!(calls(&e, NAV_MESH_RENDER_SET_DRAW), vec![vec![render, 0]]);
        // Mode 6 with drawing on: option byte flipped, drawing off, mode 3.
        let mut e = engine();
        let render = nav_engine(&mut e, true);
        e.register(GET_CURRENT_CELL, |_, _| 0u32.into_ret());
        parse_gives(&mut e, true, &[6]);
        start_log(&mut e);
        assert!(run(&mut e, 0x005b_cfe0, script(0)));
        assert_eq!(e.global::<u8>(NAV_MESH_TOGGLE_6), 1);
        assert_eq!(e.global::<u32>(NAV_MESH_MODE), 3);
        assert_eq!(
            calls(&e, NAV_MESH_RENDER_SET_TRANSPARENT),
            vec![vec![render, 0]]
        );
        assert_eq!(
            calls(&e, NAV_MESH_RENDER_SET_DRAW),
            vec![vec![render, 0], vec![render, 1]]
        );
    }

    /// Doubles for the callees of `005bd240` / `005bd300`.
    fn obstacle_engine() -> Engine {
        let mut e = engine();
        e.register(SSCANF, |e, a| {
            assert_eq!(a[1], FORMAT_HEX);
            let digits = String::from_utf8(e.mem.cstr(a[0])).unwrap();
            let value = u32::from_str_radix(digits.trim(), 16).unwrap_or(0);
            e.mem.set_u32(a[2], value);
            1u32.into_ret()
        });
        // The script has a file; the compile index lands in the top byte.
        e.register(FORM_GET_FILE, |_, a| {
            assert_eq!(a[1], 0xffff_ffff);
            0x1000u32.into_ret()
        });
        e.register(ADD_COMPILE_INDEX, |e, a| {
            assert_eq!(a[1], 0x1000);
            let id = e.mem.u32(a[0]);
            e.mem.set_u32(a[0], id | 0x0100_0000);
            Ret::default()
        });
        accept(&mut e, &[OBSTACLE_ENTRY_ACTION_A, OBSTACLE_ENTRY_ACTION_B]);
        parse_with(&mut e, true, |e, a| e.mem.set_cstr(a[7], b"1A2B"));
        e
    }

    /// Records the form ids the obstacle manager is asked about.
    fn record_obstacle_lookups(e: &mut Engine) -> Rc<RefCell<Vec<u32>>> {
        let seen = Rc::new(RefCell::new(vec![]));
        let log = seen.clone();
        e.register_double(OBSTACLE_ENTRY, move |_, a| {
            log.borrow_mut().push(a[0]);
            0x4444u32.into_ret()
        });
        seen
    }

    #[test]
    fn obstacle_command_a_converts_the_form_id_and_acts_on_its_entry() {
        let mut e = obstacle_engine();
        let seen = record_obstacle_lookups(&mut e);
        start_log(&mut e);
        assert!(run(&mut e, 0x005b_d240, script(0)));
        assert_eq!(*seen.borrow(), vec![0x0100_1a2b]);
        assert_eq!(calls(&e, OBSTACLE_ENTRY_ACTION_A), vec![vec![0x4444]]);
        assert!(calls(&e, OBSTACLE_ENTRY_ACTION_B).is_empty());
        // Without a script object the id is used as typed.
        let a = ScriptArgs {
            script_obj: Ptr::NULL,
            ..script(0)
        };
        assert!(run(&mut e, 0x005b_d240, a));
        assert_eq!(seen.borrow()[1], 0x1a2b);
        // Text that is not a number: nothing is looked up.
        parse_with(&mut e, true, |e, a| e.mem.set_cstr(a[7], b"zz"));
        start_log(&mut e);
        assert!(run(&mut e, 0x005b_d240, script(0)));
        assert!(calls(&e, OBSTACLE_ENTRY_ACTION_A).is_empty());
        // Parameters that do not parse.
        parse_gives(&mut e, false, &[]);
        assert!(!run(&mut e, 0x005b_d240, script(0)));
    }

    #[test]
    fn obstacle_command_b_calls_the_other_action() {
        let mut e = obstacle_engine();
        // A script object whose file is null keeps the typed id.
        e.register(FORM_GET_FILE, |_, _| 0u32.into_ret());
        let seen = record_obstacle_lookups(&mut e);
        start_log(&mut e);
        assert!(run(&mut e, 0x005b_d300, script(0)));
        assert_eq!(*seen.borrow(), vec![0x1a2b]);
        assert_eq!(calls(&e, OBSTACLE_ENTRY_ACTION_B), vec![vec![0x4444]]);
        assert!(calls(&e, OBSTACLE_ENTRY_ACTION_A).is_empty());
        parse_gives(&mut e, false, &[]);
        assert!(!run(&mut e, 0x005b_d300, script(0)));
    }

    // ---- Pathing info ---------------------------------------------------------------

    #[test]
    fn toggle_pathing_info_shows_the_page_once_and_clears_the_flag() {
        let mut e = engine();
        e.register(IS_DEBUG_TEXT_VISIBLE, |_, _| false.into_ret());
        accept(&mut e, &[TOGGLE_DEBUG_TEXT_VISIBLE, PATHING_DEBUG_ADD]);
        e.register(PATHING_DEBUG_OBJECT, |_, a| {
            assert!(a.is_empty());
            0x5555u32.into_ret()
        });
        start_log(&mut e);
        assert!(e.call(0x005b_d3c0, &args![]).bool());
        assert_eq!(e.global::<u8>(PATHING_INFO_ON), 1);
        assert_eq!(calls(&e, TOGGLE_DEBUG_TEXT_VISIBLE).len(), 1);
        assert_eq!(
            calls(&e, PATHING_DEBUG_ADD),
            vec![vec![0x5555, TEXT_PATHING, 0]]
        );
        // Off again: only the flag changes.
        start_log(&mut e);
        assert!(e.call(0x005b_d3c0, &args![]).bool());
        assert_eq!(e.global::<u8>(PATHING_INFO_ON), 0);
        assert!(calls(&e, PATHING_DEBUG_ADD).is_empty());
        // With the debug text already visible it is not toggled.
        e.register(IS_DEBUG_TEXT_VISIBLE, |_, _| true.into_ret());
        start_log(&mut e);
        assert!(e.call(0x005b_d3c0, &args![]).bool());
        assert!(calls(&e, TOGGLE_DEBUG_TEXT_VISIBLE).is_empty());
        assert_eq!(calls(&e, PATHING_DEBUG_ADD).len(), 1);
    }

    // ---- PickRefByID -------------------------------------------------------------------

    /// Doubles for the callees of `005bd410`; returns the form.
    fn pick_engine(e: &mut Engine, form_type: u8) -> u32 {
        let form = e.mem.alloc(0x40);
        e.mem.set_u8(form + 4, form_type);
        e.register(FORM_TYPE, |e, a| (e.mem.u8(a[0] + 4) as u32).into_ret());
        e.register(FORM_ID, |_, _| 0xff00_1234u32.into_ret());
        e.register(OBJECT_NAME, |_, _| 0x6666u32.into_ret());
        accept(
            e,
            &[
                BS_STRING_CONSTRUCT,
                BS_STRING_DESTRUCT,
                BS_STRING_FORMAT,
                DEBUG_TEXT_PRINT,
                PICK_REFERENCE,
            ],
        );
        e.register(STRING_TEXT, |_, _| 0x7000u32.into_ret());
        e.register(DEBUG_TEXT_INSTANCE, |_, a| {
            assert_eq!(a, [1]);
            0x8000u32.into_ret()
        });
        // Setting 011f33c8 says 3 (line 2); line 2 of the array points to an
        // entry whose float is 120.4.
        e.set_global(PICK_REF_LINE_SETTING + 4, 3u32);
        let float = e.mem.alloc(4);
        e.mem.set_f32(float, 120.4);
        let entry = e.mem.alloc(0x40);
        e.mem.set_u32(entry + 0x38, float);
        let array = e.mem.alloc(16);
        e.mem.set_u32(array + 8, entry);
        e.set_global(DEBUG_LINE_ARRAY, array);
        e.set_global(PICK_REF_ROUNDING, 3.0f64);
        e.set_global(PICK_REF_COLUMN, 640.0f32);
        e.set_global(FLOAT_MINUS_ONE, -1.0f32);
        form
    }

    #[test]
    fn pick_ref_by_id_prints_the_reference_on_the_computed_row() {
        let mut e = engine();
        let form = pick_engine(&mut e, 0x3a);
        parse_gives(&mut e, true, &[form]);
        start_log(&mut e);
        assert!(run(&mut e, 0x005b_d410, script(0)));
        let string = calls(&e, BS_STRING_CONSTRUCT)[0][0];
        assert_eq!(
            calls(&e, BS_STRING_FORMAT),
            vec![vec![string, FORMAT_PICKED_REFERENCE, 0x6666, 0xff00_1234]]
        );
        // Row: trunc(120.4 + 3.0) - 5 = 118.
        assert_eq!(
            calls(&e, DEBUG_TEXT_PRINT),
            vec![vec![
                0x8000,
                0x7000,
                640.0f32.to_bits(),
                118.0f32.to_bits(),
                2,
                0xffff_ffff,
                (-1.0f32).to_bits(),
                0,
                0
            ]]
        );
        assert_eq!(calls(&e, PICK_REFERENCE), vec![vec![form]]);
        assert_eq!(calls(&e, BS_STRING_DESTRUCT), vec![vec![string]]);
    }

    #[test]
    fn pick_ref_by_id_only_handles_the_listed_form_types() {
        for (form_type, shown) in [
            (0x39u8, false),
            (0x3a, true),
            (0x40, true),
            (0x41, false),
            (0x69, true),
        ] {
            let mut e = engine();
            let form = pick_engine(&mut e, form_type);
            parse_gives(&mut e, true, &[form]);
            start_log(&mut e);
            assert!(run(&mut e, 0x005b_d410, script(0)));
            assert_eq!(
                calls(&e, DEBUG_TEXT_PRINT).len(),
                shown as usize,
                "{form_type:#x}"
            );
        }
        // No reference: true, nothing printed; bad parameters: false.
        let mut e = engine();
        pick_engine(&mut e, 0x3a);
        parse_gives(&mut e, true, &[0]);
        start_log(&mut e);
        assert!(run(&mut e, 0x005b_d410, script(0)));
        assert!(calls(&e, DEBUG_TEXT_PRINT).is_empty());
        parse_gives(&mut e, false, &[]);
        assert!(!run(&mut e, 0x005b_d410, script(0)));
    }

    #[test]
    fn debug_line_helpers_read_their_fields() {
        let mut e = engine();
        // 005bd580: the float behind +0x38, or 0.0.
        let object = e.mem.alloc(0x40);
        assert_eq!(e.call(0x005b_d580, &args![object]).f32(), 0.0);
        let value = e.mem.alloc(4);
        e.mem.set_f32(value, 7.25);
        e.mem.set_u32(object + 0x38, value);
        assert_eq!(e.call(0x005b_d580, &args![object]).f32(), 7.25);
        // 005bd5b0 returns the global, 005bd5c0 indexes an array of words.
        e.set_global(DEBUG_LINE_ARRAY, 0x1234_5678u32);
        assert_eq!(e.call(0x005b_d5b0, &args![]).u32(), 0x1234_5678);
        let array = e.mem.alloc(16);
        e.mem.set_u32(array + 12, 0xabcd);
        assert_eq!(e.call(0x005b_d5c0, &args![array, 3u32]).u32(), 0xabcd);
        // 005be4d0 and 005be5c0 read +0x7b8 and +0x20 + index * 4.
        let big = e.mem.alloc(0x800);
        e.mem.set_u32(big + 0x7b8, 0x77);
        assert_eq!(e.call(0x005b_e4d0, &args![big]).u32(), 0x77);
        e.mem.set_u32(big + 0x20 + 8, 0x88);
        assert_eq!(e.call(0x005b_e5c0, &args![big, 2u32]).u32(), 0x88);
    }

    // ---- CenterOn --------------------------------------------------------------------------

    #[test]
    fn center_on_cell_closes_the_console_and_moves_the_player() {
        let mut e = engine();
        accept(&mut e, &[CLOSE_CONSOLE, CENTER_ON_CELL]);
        e.set_global(PLAYER, 0x3333u32);
        let name = Rc::new(Cell::new(0u32));
        let slot = name.clone();
        parse_with(&mut e, true, move |_, a| slot.set(a[7]));
        start_log(&mut e);
        assert!(run(&mut e, 0x005b_d5e0, script(0)));
        assert_eq!(calls(&e, CLOSE_CONSOLE).len(), 1);
        assert_eq!(calls(&e, CENTER_ON_CELL), vec![vec![0x3333, name.get(), 0]]);
        parse_gives(&mut e, false, &[]);
        start_log(&mut e);
        assert!(!run(&mut e, 0x005b_d5e0, script(0)));
        assert!(calls(&e, CENTER_ON_CELL).is_empty());
    }

    /// Doubles for the world space and cell functions the exterior commands
    /// use. The player is a heap object.
    fn center_engine(e: &mut Engine) -> u32 {
        accept(e, &[CLOSE_CONSOLE, CENTER_ON_CELL, CANCEL_ALL_CELL_LOADS]);
        let player = e.mem.alloc(0x300);
        e.set_global(PLAYER, player);
        e.set_global(DATA_HANDLER, 0x4000u32);
        e.set_global(TES_SINGLETON, 0x1111u32);
        player
    }

    #[test]
    fn center_on_exterior_creates_the_cell_when_it_is_not_there() {
        let mut e = engine();
        let player = center_engine(&mut e);
        e.set_global(EXTERIOR_CELL_LOADER, 0x9000u32);
        e.register(TES_GET_WORLD_SPACE, |_, a| {
            assert_eq!(a[0], 0x1111);
            0x5000u32.into_ret()
        });
        e.register(WORLD_SPACE_GET_CELL, |_, _| 0u32.into_ret());
        e.register(WORLD_SPACE_LOAD_CELL, |_, _| 0u32.into_ret());
        e.register(DATA_HANDLER_NEW_CELL, |_, _| 0x888u32.into_ret());
        parse_gives(&mut e, true, &[7, 8]);
        start_log(&mut e);
        assert!(run(&mut e, 0x005b_d660, script(0)));
        assert_eq!(calls(&e, CANCEL_ALL_CELL_LOADS), vec![vec![0x9000]]);
        assert_eq!(calls(&e, WORLD_SPACE_GET_CELL), vec![vec![0x5000, 7, 8]]);
        assert_eq!(calls(&e, WORLD_SPACE_LOAD_CELL), vec![vec![0x5000, 7, 8]]);
        assert_eq!(
            calls(&e, DATA_HANDLER_NEW_CELL),
            vec![vec![0x4000, 0, 7, 8, 0x5000]]
        );
        assert_eq!(calls(&e, CENTER_ON_CELL), vec![vec![player, 0, 0x888]]);
        assert_eq!(e.mem.u8(player + 0x206), 1);
    }

    #[test]
    fn center_on_exterior_falls_back_to_the_first_world_space() {
        let mut e = engine();
        let player = center_engine(&mut e);
        e.register(TES_GET_WORLD_SPACE, |_, _| 0u32.into_ret());
        // The data handler's list head holds the world space in its item.
        let node = e.mem.alloc(8);
        e.mem.set_u32(node, 0x5100);
        e.register_double(DATA_HANDLER_WORLD_SPACES, move |_, a| {
            assert_eq!(a[0], 0x4000);
            node.into_ret()
        });
        e.register(LIST_ITEM_PTR, |_, a| a[0].into_ret());
        e.register(WORLD_SPACE_GET_CELL, |_, _| 0x777u32.into_ret());
        parse_gives(&mut e, true, &[1, 2]);
        start_log(&mut e);
        assert!(run(&mut e, 0x005b_d660, script(0)));
        // The cell was found: no load, no new cell, no loader to cancel.
        assert_eq!(calls(&e, WORLD_SPACE_GET_CELL), vec![vec![0x5100, 1, 2]]);
        assert!(calls(&e, CANCEL_ALL_CELL_LOADS).is_empty());
        assert_eq!(calls(&e, CENTER_ON_CELL), vec![vec![player, 0, 0x777]]);
        // No world space anywhere: false.
        e.mem.set_u32(node, 0);
        assert!(!run(&mut e, 0x005b_d660, script(0)));
        parse_gives(&mut e, false, &[]);
        assert!(!run(&mut e, 0x005b_d660, script(0)));
    }

    #[test]
    fn center_on_world_loads_the_cell_of_the_given_world_space() {
        let mut e = engine();
        let player = center_engine(&mut e);
        e.register(WORLD_SPACE_GET_CELL, |_, _| 0u32.into_ret());
        e.register(WORLD_SPACE_LOAD_CELL, |_, _| 0x999u32.into_ret());
        parse_gives(&mut e, true, &[0x5200, 3, 4]);
        start_log(&mut e);
        assert!(run(&mut e, 0x005b_d780, script(0)));
        assert_eq!(calls(&e, WORLD_SPACE_LOAD_CELL), vec![vec![0x5200, 3, 4]]);
        assert!(calls(&e, DATA_HANDLER_NEW_CELL).is_empty());
        assert_eq!(calls(&e, CENTER_ON_CELL), vec![vec![player, 0, 0x999]]);
        assert_eq!(e.mem.u8(player + 0x206), 1);
        // No world space: false and nothing happens.
        parse_gives(&mut e, true, &[0, 3, 4]);
        start_log(&mut e);
        assert!(!run(&mut e, 0x005b_d780, script(0)));
        assert!(calls(&e, WORLD_SPACE_GET_CELL).is_empty());
        parse_gives(&mut e, false, &[]);
        assert!(!run(&mut e, 0x005b_d780, script(0)));
    }

    // ---- Forwarding bodies -------------------------------------------------------------------

    /// Runs the forwarding body at `address` with `result` set, expecting it
    /// to call `callee(thisObj, argument, 0, result)`.
    fn check_forward(address: u32, callee: u32, parsed: Option<u32>) {
        let mut e = engine();
        e.register(callee, |_, _| true.into_ret());
        let a = ScriptArgs {
            result: Ptr::new(0x1234),
            ..script(0x40)
        };
        if let Some(argument) = parsed {
            parse_gives(&mut e, true, &[argument]);
        }
        start_log(&mut e);
        assert!(run(&mut e, address, a));
        assert_eq!(
            calls(&e, callee),
            vec![vec![0x40, parsed.unwrap_or(0), 0, 0x1234]]
        );
        // The callee's answer is the result.
        e.register(callee, |_, _| false.into_ret());
        assert!(!run(&mut e, address, a));
        if parsed.is_some() {
            // Parameters that do not parse: false, callee not called.
            parse_gives(&mut e, false, &[]);
            start_log(&mut e);
            e.register(callee, |_, _| true.into_ret());
            assert!(!run(&mut e, address, a));
            assert!(calls(&e, callee).is_empty());
        }
    }

    #[test]
    fn body_005bd880_forwards_this_obj_and_the_result() {
        check_forward(0x005b_d880, COND_FN_0059C4C0, None);
    }

    #[test]
    fn body_005bd8a0_parses_one_argument_and_forwards_it() {
        check_forward(0x005b_d8a0, COND_FN_0059C4F0, Some(0x77));
    }

    #[test]
    fn body_005bd900_parses_one_argument_and_forwards_it() {
        check_forward(0x005b_d900, COND_FN_0059C680, Some(0x78));
    }

    #[test]
    fn fatigue_percentage_forwards_to_its_condition_function() {
        check_forward(0x005b_e6a0, GET_FATIGUE_PERCENTAGE_CONDITION, None);
    }

    #[test]
    fn health_percentage_forwards_to_its_condition_function() {
        check_forward(0x005b_e6c0, GET_HEALTH_PERCENTAGE_CONDITION, None);
    }

    #[test]
    fn get_base_actor_value_forwards_the_parsed_actor_value() {
        check_forward(0x005b_e6e0, GET_BASE_ACTOR_VALUE_CONDITION, Some(0x15));
    }

    #[test]
    fn cause_of_death_forwards_to_its_condition_function() {
        check_forward(0x005b_e740, GET_CAUSE_OF_DEATH_CONDITION, None);
    }

    #[test]
    fn killing_blow_limb_forwards_to_its_condition_function() {
        check_forward(0x005b_e830, GET_KILLING_BLOW_LIMB_CONDITION, None);
    }

    #[test]
    fn body_005be850_forwards_to_its_condition_function() {
        check_forward(0x005b_e850, COND_FN_005A3E90, None);
    }

    // ---- IsLimbGone -------------------------------------------------------------------------------

    #[test]
    fn is_limb_gone_without_a_range_uses_the_condition_function() {
        let mut e = engine();
        e.register(IS_LIMB_GONE_CONDITION, |_, _| true.into_ret());
        let a = ScriptArgs {
            result: Ptr::new(0x1234),
            ..script(0x40)
        };
        // Last limb stays at the default -1000.
        parse_gives(&mut e, true, &[3, -1000i32 as u32]);
        start_log(&mut e);
        assert!(run(&mut e, 0x005b_e760, a));
        assert_eq!(
            calls(&e, IS_LIMB_GONE_CONDITION),
            vec![vec![0x40, 3, 0, 0x1234]]
        );
        // A last limb of 0 is not a range either.
        parse_gives(&mut e, true, &[3, 0]);
        start_log(&mut e);
        assert!(run(&mut e, 0x005b_e760, a));
        assert_eq!(calls(&e, IS_LIMB_GONE_CONDITION).len(), 1);
        parse_gives(&mut e, false, &[]);
        assert!(!run(&mut e, 0x005b_e760, a));
    }

    #[test]
    fn is_limb_gone_with_a_range_stops_at_the_first_dismembered_limb() {
        let mut e = engine();
        let actor = actor_with(&mut e, &[]);
        let result = e.mem.alloc(8);
        let a = ScriptArgs {
            result: Ptr::new(result),
            ..script(actor)
        };
        e.register(REFERENCE_IS_LIMB_DISMEMBERED, |_, a| (a[1] == 4).into_ret());
        parse_gives(&mut e, true, &[2, 6]);
        start_log(&mut e);
        assert!(run(&mut e, 0x005b_e760, a));
        let asked: Vec<u32> = calls(&e, REFERENCE_IS_LIMB_DISMEMBERED)
            .iter()
            .map(|w| w[1])
            .collect();
        assert_eq!(asked, vec![2, 3, 4]);
        assert_eq!(e.mem.f64(result), 1.0);
        // Nothing dismembered: the result is left alone, still true.
        let other = e.mem.alloc(8);
        let a = ScriptArgs {
            result: Ptr::new(other),
            ..script(actor)
        };
        parse_gives(&mut e, true, &[5, 6]);
        assert!(run(&mut e, 0x005b_e760, a));
        assert_eq!(e.mem.f64(other), 0.0);
        // Not an actor: true, no limb asked about.
        let plain = object_with(&mut e, &[(0x100, V_FALSE)]);
        let a = ScriptArgs {
            result: Ptr::new(other),
            ..script(plain)
        };
        start_log(&mut e);
        assert!(run(&mut e, 0x005b_e760, a));
        assert!(calls(&e, REFERENCE_IS_LIMB_DISMEMBERED).is_empty());
    }

    // ---- GetActorValueInfo ----------------------------------------------------------------------------

    /// What `info_engine` builds.
    struct Info {
        actor: u32,
        player: u32,
        auto: Rc<Cell<bool>>,
    }

    /// An actor whose slots and helpers answer: computed base 11.5, current
    /// 13.5, damage 1.5, permanent 2.5, temporary 3.5, reference base 9.5,
    /// derived 6.25, SetAV override 4.5, level-up value 7.5.
    fn info_engine(e: &mut Engine, creature: bool, is_player: bool, derived: bool) -> Info {
        let player = object_with(e, &[]);
        let owner = vtable(
            e,
            &[
                (0x4, V_FLOAT + 11),
                (0xc, V_FLOAT + 13),
                (0x14, V_FLOAT + 1),
                (0x18, V_FLOAT + 2),
                (0x10, V_FLOAT + 3),
                (0x20, V_FLOAT + 7),
            ],
        );
        e.mem.set_u32(player + 0xa4, owner);
        e.set_global(PLAYER, player);
        let override_slot = 0x0900_0100;
        e.register(override_slot, |e, a| {
            e.mem.set_u8(a[2], 1);
            4.5f32.into_ret()
        });
        let actor = actor_with(
            e,
            &[
                (0x48c, override_slot),
                (0x21c, if creature { V_TRUE } else { V_FALSE }),
                (0x360, if is_player { V_TRUE } else { V_FALSE }),
            ],
        );
        e.mem.set_u32(actor + 0xa4, owner);
        // The base-value owner sits at +0x100 of another object.
        let base = e.mem.alloc(0x200);
        let base_table = vtable(e, &[(0xc, V_FLOAT + 9)]);
        e.mem.set_u32(base + 0x100, base_table);
        e.register_double(REFERENCE_BASE_OWNER, move |_, _| base.into_ret());
        e.register(OBJECT_NAME, |_, _| 0x6666u32.into_ret());
        let auto = Rc::new(Cell::new(false));
        let flag = auto.clone();
        e.register_double(ACTOR_IS_AUTO_CALCULATED, move |_, _| flag.get().into_ret());
        e.register_double(ACTOR_VALUE_DERIVED, move |e, a| {
            e.mem.set_f32(a[2], 6.25);
            derived.into_ret()
        });
        set_echo(e, true);
        Info {
            actor,
            player,
            auto,
        }
    }

    #[test]
    fn actor_value_info_prints_every_component() {
        let mut e = engine();
        let info = info_engine(&mut e, true, true, true);
        parse_gives(&mut e, true, &[3]);
        start_log(&mut e);
        assert!(run(&mut e, 0x005b_d960, script(info.actor)));
        assert_eq!(
            printed(&e),
            vec![
                args![MSG_ACTOR_VALUE_INFO, 0x0a00_0003u32, 0x6666u32],
                args![MSG_CURRENT_VALUE, 13.5f64, 11.5f64],
                args![MSG_BASE_VALUE_COMPONENTS],
                args![MSG_REFERENCE_BASE_VALUE, 9.5f64, TEXT_CREATURE],
                // The player is not ignored through the 0x80 route, but the
                // creature flag still applies.
                args![MSG_DERIVED_VALUE, 6.25f64, TEXT_IGNORED],
                args![MSG_SET_AV_OVERRIDE, 4.5f64],
                args![MSG_MODIFIERS, 3.5f64, 2.5f64, 1.5f64],
                args![MSG_LEVEL_UP_VALUE, 7.5f64],
            ]
        );
        // The override slot gets the actor value and the flag cell.
        let override_calls = calls(&e, 0x0900_0100);
        assert_eq!(override_calls[0][..2], [info.actor, 3]);
        assert_eq!(calls(&e, V_FLOAT + 7), vec![vec![info.player + 0xa4, 3]]);
    }

    #[test]
    fn actor_value_info_marks_auto_calculated_and_ignored_values() {
        // Auto-calculated reference value; the 0x80 flag (actor value 7)
        // does not apply because the actor is auto-calculated.
        let mut e = engine();
        let info = info_engine(&mut e, false, false, true);
        info.auto.set(true);
        parse_gives(&mut e, true, &[7]);
        start_log(&mut e);
        assert!(run(&mut e, 0x005b_d960, script(info.actor)));
        let prints = printed(&e);
        assert_eq!(
            prints[3],
            args![MSG_REFERENCE_BASE_VALUE, 9.5f64, TEXT_AUTO_CALCULATED]
        );
        assert_eq!(prints[4], args![MSG_DERIVED_VALUE, 6.25f64, TEXT_EMPTY]);
        // Not auto, not a player, flag 0x80: "(Ignored)"; no level-up line.
        info.auto.set(false);
        start_log(&mut e);
        assert!(run(&mut e, 0x005b_d960, script(info.actor)));
        let prints = printed(&e);
        assert_eq!(
            prints[3],
            args![MSG_REFERENCE_BASE_VALUE, 9.5f64, TEXT_EMPTY]
        );
        assert_eq!(prints[4], args![MSG_DERIVED_VALUE, 6.25f64, TEXT_IGNORED]);
        assert_eq!(prints.len(), 7);
        // Without a derived value the line is left out.
        let mut e = engine();
        let info = info_engine(&mut e, false, false, false);
        parse_gives(&mut e, true, &[3]);
        start_log(&mut e);
        assert!(run(&mut e, 0x005b_d960, script(info.actor)));
        assert!(!printed(&e).iter().any(|p| p[0] == MSG_DERIVED_VALUE));
    }

    #[test]
    fn actor_value_info_needs_echo_an_actor_and_a_valid_actor_value() {
        let mut e = engine();
        let info = info_engine(&mut e, false, false, true);
        parse_gives(&mut e, true, &[0x4d]);
        start_log(&mut e);
        assert!(run(&mut e, 0x005b_d960, script(info.actor)));
        assert!(printed(&e).is_empty());
        parse_gives(&mut e, true, &[-1i32 as u32]);
        assert!(run(&mut e, 0x005b_d960, script(info.actor)));
        assert!(printed(&e).is_empty());
        parse_gives(&mut e, true, &[3]);
        assert!(run(&mut e, 0x005b_d960, script(0)));
        let not_actor = object_with(&mut e, &[(0x100, V_FALSE)]);
        assert!(run(&mut e, 0x005b_d960, script(not_actor)));
        assert!(printed(&e).is_empty());
        set_echo(&mut e, false);
        assert!(run(&mut e, 0x005b_d960, script(info.actor)));
        assert!(printed(&e).is_empty());
        parse_gives(&mut e, false, &[]);
        assert!(!run(&mut e, 0x005b_d960, script(info.actor)));
    }

    // ---- Actor value commands ----------------------------------------------------------------------------

    /// An actor with the named slots recorded; the engine echoes.
    fn recorder_actor(e: &mut Engine, slots: &[u32]) -> u32 {
        let table: Vec<(u32, u32)> = slots.iter().map(|s| (*s, V_RECORD)).collect();
        let actor = actor_with(e, &table);
        set_echo(e, true);
        actor
    }

    #[test]
    fn body_005bdcd0_passes_the_pair_to_slot_398_unless_protected() {
        let mut e = engine();
        let actor = recorder_actor(&mut e, &[0x398]);
        parse_gives(&mut e, true, &[3, 42]);
        start_log(&mut e);
        assert!(run(&mut e, 0x005b_dcd0, script(actor)));
        assert_eq!(calls(&e, V_RECORD), vec![vec![actor, 3, 42]]);
        // Protected actor value 5: the message, no call.
        parse_gives(&mut e, true, &[5, 42]);
        start_log(&mut e);
        assert!(run(&mut e, 0x005b_dcd0, script(actor)));
        assert!(calls(&e, V_RECORD).is_empty());
        assert_eq!(printed(&e), vec![vec![MSG_CANNOT_MODIFY, 0x0a00_0005]]);
        // The message needs echo.
        set_echo(&mut e, false);
        start_log(&mut e);
        assert!(run(&mut e, 0x005b_dcd0, script(actor)));
        assert!(printed(&e).is_empty());
        // No actor: true, nothing; bad parameters: false.
        assert!(run(&mut e, 0x005b_dcd0, script(0)));
        parse_gives(&mut e, false, &[]);
        assert!(!run(&mut e, 0x005b_dcd0, script(actor)));
    }

    #[test]
    fn body_005bde40_passes_the_pair_and_zero_to_slot_3a8() {
        let mut e = engine();
        let actor = recorder_actor(&mut e, &[0x3a8]);
        parse_gives(&mut e, true, &[3, 42]);
        start_log(&mut e);
        assert!(run(&mut e, 0x005b_de40, script(actor)));
        assert_eq!(calls(&e, V_RECORD), vec![vec![actor, 3, 42, 0]]);
        parse_gives(&mut e, true, &[5, 42]);
        start_log(&mut e);
        assert!(run(&mut e, 0x005b_de40, script(actor)));
        assert!(calls(&e, V_RECORD).is_empty());
        assert_eq!(printed(&e), vec![vec![MSG_CANNOT_MODIFY, 0x0a00_0005]]);
        assert!(run(&mut e, 0x005b_de40, script(0)));
        parse_gives(&mut e, false, &[]);
        assert!(!run(&mut e, 0x005b_de40, script(actor)));
    }

    #[test]
    fn set_size_resizes_the_character_controller() {
        let mut e = engine();
        let actor = recorder_actor(&mut e, &[]);
        e.register(GET_CHAR_CONTROLLER, |_, _| 0x6000u32.into_ret());
        accept(&mut e, &[CHAR_CONTROLLER_SET_SIZE]);
        parse_gives(&mut e, true, &[1.5f32.to_bits()]);
        start_log(&mut e);
        assert!(run(&mut e, 0x005b_ddb0, script(actor)));
        assert_eq!(
            calls(&e, CHAR_CONTROLLER_SET_SIZE),
            vec![vec![0x6000, 1.5f32.to_bits()]]
        );
        // No controller, no actor, bad parameters.
        e.register(GET_CHAR_CONTROLLER, |_, _| 0u32.into_ret());
        start_log(&mut e);
        assert!(run(&mut e, 0x005b_ddb0, script(actor)));
        assert!(run(&mut e, 0x005b_ddb0, script(0)));
        assert!(calls(&e, CHAR_CONTROLLER_SET_SIZE).is_empty());
        parse_gives(&mut e, false, &[]);
        assert!(!run(&mut e, 0x005b_ddb0, script(actor)));
    }

    #[test]
    fn body_005bdf20_lowers_a_value_by_the_negated_magnitude() {
        let mut e = engine();
        let actor = recorder_actor(&mut e, &[0x3ac, 0x4b8]);
        // Slot 0x1a0 answers false until the test changes it.
        let table = e.mem.u32(actor);
        e.mem.set_u32(table + 0x1a0, V_FALSE);
        // Actor value 3 without flag 0x200: -|amount|.
        parse_gives(&mut e, true, &[3, (-2.5f32).to_bits()]);
        start_log(&mut e);
        assert!(run(&mut e, 0x005b_df20, script(actor)));
        assert_eq!(
            calls(&e, V_RECORD),
            vec![vec![actor, 3, (-2.5f32).to_bits(), 0]]
        );
        // Actor value 6 has flag 0x200: the amount unchanged.
        parse_gives(&mut e, true, &[6, 4.0f32.to_bits()]);
        start_log(&mut e);
        assert!(run(&mut e, 0x005b_df20, script(actor)));
        assert_eq!(
            calls(&e, V_RECORD),
            vec![vec![actor, 6, 4.0f32.to_bits(), 0]]
        );
        // Protected: the message only.
        parse_gives(&mut e, true, &[5, 1.0f32.to_bits()]);
        start_log(&mut e);
        assert!(run(&mut e, 0x005b_df20, script(actor)));
        assert!(calls(&e, V_RECORD).is_empty());
        assert_eq!(printed(&e), vec![vec![MSG_CANNOT_MODIFY, 0x0a00_0005]]);
        // Actor value 0x10 on an actor that answers true to slot 0x1a0
        // goes to slot 0x4b8(0, -amount).
        e.mem.set_u32(table + 0x1a0, V_TRUE);
        parse_gives(&mut e, true, &[0x10, 3.0f32.to_bits()]);
        start_log(&mut e);
        assert!(run(&mut e, 0x005b_df20, script(actor)));
        assert_eq!(
            calls(&e, V_RECORD),
            vec![vec![actor, 0, (-3.0f32).to_bits()]]
        );
        assert_eq!(calls(&e, V_TRUE), vec![vec![actor, 0]]);
        // Without that answer the generic route is used.
        e.mem.set_u32(table + 0x1a0, V_FALSE);
        start_log(&mut e);
        assert!(run(&mut e, 0x005b_df20, script(actor)));
        assert_eq!(
            calls(&e, V_RECORD),
            vec![vec![actor, 0x10, (-3.0f32).to_bits(), 0]]
        );
        // No actor: true; bad parameters: false.
        assert!(run(&mut e, 0x005b_df20, script(0)));
        parse_gives(&mut e, false, &[]);
        assert!(!run(&mut e, 0x005b_df20, script(actor)));
    }

    #[test]
    fn restore_actor_value_passes_the_magnitude_or_the_negated_amount() {
        let mut e = engine();
        let actor = recorder_actor(&mut e, &[]);
        accept(&mut e, &[ACTOR_RESTORE_ACTOR_VALUE]);
        parse_gives(&mut e, true, &[3, (-2.5f32).to_bits()]);
        start_log(&mut e);
        assert!(run(&mut e, 0x005b_e080, script(actor)));
        assert_eq!(
            calls(&e, ACTOR_RESTORE_ACTOR_VALUE),
            vec![vec![actor, 3, 2.5f32.to_bits()]]
        );
        // Flag 0x200: the amount negated.
        parse_gives(&mut e, true, &[6, 4.0f32.to_bits()]);
        start_log(&mut e);
        assert!(run(&mut e, 0x005b_e080, script(actor)));
        assert_eq!(
            calls(&e, ACTOR_RESTORE_ACTOR_VALUE),
            vec![vec![actor, 6, (-4.0f32).to_bits()]]
        );
        parse_gives(&mut e, true, &[5, 4.0f32.to_bits()]);
        start_log(&mut e);
        assert!(run(&mut e, 0x005b_e080, script(actor)));
        assert!(calls(&e, ACTOR_RESTORE_ACTOR_VALUE).is_empty());
        assert_eq!(printed(&e), vec![vec![MSG_CANNOT_MODIFY, 0x0a00_0005]]);
        assert!(run(&mut e, 0x005b_e080, script(0)));
        parse_gives(&mut e, false, &[]);
        assert!(!run(&mut e, 0x005b_e080, script(actor)));
    }

    #[test]
    fn body_005be190_sets_a_value_through_the_difference() {
        let mut e = engine();
        let actor = recorder_actor(&mut e, &[0x3a4]);
        // The ActorValueOwner part (+0xa4) answers 13.5 to slot 0xc.
        let owner = vtable(&mut e, &[(0xc, V_FLOAT + 13)]);
        e.mem.set_u32(actor + 0xa4, owner);
        parse_gives(&mut e, true, &[3, 20]);
        start_log(&mut e);
        assert!(run(&mut e, 0x005b_e190, script(actor)));
        assert_eq!(
            calls(&e, V_RECORD),
            vec![vec![actor, 3, 6.5f32.to_bits(), 0]]
        );
        assert_eq!(calls(&e, V_FLOAT + 13), vec![vec![actor + 0xa4, 3]]);
        parse_gives(&mut e, true, &[5, 20]);
        start_log(&mut e);
        assert!(run(&mut e, 0x005b_e190, script(actor)));
        assert!(calls(&e, V_RECORD).is_empty());
        assert_eq!(printed(&e), vec![vec![MSG_CANNOT_MODIFY, 0x0a00_0005]]);
        assert!(run(&mut e, 0x005b_e190, script(0)));
        parse_gives(&mut e, false, &[]);
        assert!(!run(&mut e, 0x005b_e190, script(actor)));
    }

    // ---- Kill ----------------------------------------------------------------------------------------------

    /// Doubles for `005be2a0`: a living actor (its `+0x94` part answers
    /// "not dead") and a player with the experience slot.
    fn kill_engine(e: &mut Engine, creature: bool) -> (u32, u32) {
        let player = object_with(e, &[(0x488, V_RECORD), (0x1a0, V_FALSE)]);
        e.mem.set_u32(player + 0x7b8, 0x55);
        e.set_global(PLAYER, player);
        let actor = actor_with(
            e,
            &[
                (0x1a0, V_FALSE),
                (0x21c, if creature { V_TRUE } else { V_FALSE }),
            ],
        );
        let part = vtable(e, &[(0x10, V_FALSE)]);
        e.mem.set_u32(actor + 0x94, part);
        e.register(KILLS_ARE_QUEUED, |_, _| false.into_ret());
        e.register(ACTOR_LEVEL, |_, _| 12u32.into_ret());
        e.register(GET_EXPERIENCE_POINTS, |_, _| 100u32.into_ret());
        e.register(GAMEPLAY_FORMULA, |_, _| 250.0f32.into_ret());
        e.register(ROUND_EXPERIENCE, |_, a| f32::from_bits(a[0]).into_ret());
        e.register(ACTOR_LIFE_STATE, |_, _| 6u32.into_ret());
        accept(
            e,
            &[
                ACTOR_SET_LIFE_STATE,
                ACTOR_KILL,
                ACTOR_SCRIPT_DISMEMBER,
                QUEUE_ACTOR_KILL,
            ],
        );
        e.register(TASK_QUEUE_INTERFACE, |_, a| {
            assert!(a.is_empty());
            0x4242u32.into_ret()
        });
        (actor, player)
    }

    #[test]
    fn kill_by_the_player_gives_experience_and_dismembers() {
        let mut e = engine();
        let (actor, player) = kill_engine(&mut e, false);
        // Killer = player, part 3, detail 7.
        parse_gives(&mut e, true, &[player, 3, 7]);
        start_log(&mut e);
        assert!(run(&mut e, 0x005b_e2a0, script(actor)));
        // A non-creature (flag true), level 12; formula(100.0, player +0x7b8).
        assert_eq!(calls(&e, GET_EXPERIENCE_POINTS), vec![vec![1, 12]]);
        assert_eq!(
            calls(&e, GAMEPLAY_FORMULA),
            vec![vec![100.0f32.to_bits(), 0x55]]
        );
        assert_eq!(calls(&e, V_RECORD), vec![vec![player, 250]]);
        assert_eq!(calls(&e, ACTOR_SET_LIFE_STATE), vec![vec![actor, 0]]);
        assert_eq!(calls(&e, ACTOR_KILL), vec![vec![actor, player, 0]]);
        assert_eq!(
            calls(&e, ACTOR_SCRIPT_DISMEMBER),
            vec![vec![actor, 7, 3, player]]
        );
        assert!(calls(&e, QUEUE_ACTOR_KILL).is_empty());
        // A creature gives the other experience flag.
        let mut e = engine();
        let (actor, player) = kill_engine(&mut e, true);
        parse_gives(&mut e, true, &[player, -1i32 as u32, -1i32 as u32]);
        start_log(&mut e);
        assert!(run(&mut e, 0x005b_e2a0, script(actor)));
        assert_eq!(calls(&e, GET_EXPERIENCE_POINTS), vec![vec![0, 12]]);
        // Part -1: no dismember.
        assert!(calls(&e, ACTOR_SCRIPT_DISMEMBER).is_empty());
    }

    #[test]
    fn kill_by_someone_else_with_a_task_queue_queues_the_kill() {
        let mut e = engine();
        let (actor, _) = kill_engine(&mut e, false);
        e.register(KILLS_ARE_QUEUED, |_, _| true.into_ret());
        e.register(ACTOR_LIFE_STATE, |_, _| 0u32.into_ret());
        parse_gives(&mut e, true, &[0x999, 2, 5]);
        start_log(&mut e);
        assert!(run(&mut e, 0x005b_e2a0, script(actor)));
        // No experience (the killer is not the player), no life state reset.
        assert!(calls(&e, GET_EXPERIENCE_POINTS).is_empty());
        assert!(calls(&e, ACTOR_SET_LIFE_STATE).is_empty());
        assert!(calls(&e, ACTOR_KILL).is_empty());
        let queued = &calls(&e, QUEUE_ACTOR_KILL)[0];
        assert_eq!(queued[..2], [0x4242, actor]);
        // The damage float is 0.0 and the dismember data is {part, detail,
        // killer}; the killer is also stored in the actor.
        assert_eq!(e.mem.f32(queued[2]), 0.0);
        assert_eq!(e.mem.u32(queued[3]), 2);
        assert_eq!(e.mem.u32(queued[3] + 4), 5);
        assert_eq!(e.mem.u32(queued[3] + 8), 0x999);
        assert_eq!(e.mem.u32(actor + 0xc0), 0x999);
        // Without a part there is no dismember data.
        parse_gives(&mut e, true, &[0x999, -1i32 as u32, 5]);
        start_log(&mut e);
        assert!(run(&mut e, 0x005b_e2a0, script(actor)));
        assert_eq!(calls(&e, QUEUE_ACTOR_KILL)[0][3], 0);
    }

    #[test]
    fn kill_leaves_dead_actors_alone_and_ignores_the_parse_result() {
        let mut e = engine();
        let (actor, player) = kill_engine(&mut e, false);
        // Already dead: slot 0x10 of the +0x94 part answers true.
        let part = vtable(&mut e, &[(0x10, V_TRUE)]);
        e.mem.set_u32(actor + 0x94, part);
        parse_gives(&mut e, true, &[0x999, 1, 1]);
        start_log(&mut e);
        assert!(run(&mut e, 0x005b_e2a0, script(actor)));
        assert!(calls(&e, ACTOR_KILL).is_empty());
        // Not an actor: nothing.
        assert!(run(&mut e, 0x005b_e2a0, script(0)));
        assert!(calls(&e, ACTOR_KILL).is_empty());
        // Bad parameters: the defaults (killer 0, part -1) are used anyway.
        let part = vtable(&mut e, &[(0x10, V_FALSE)]);
        e.mem.set_u32(actor + 0x94, part);
        parse_with(&mut e, false, |_, _| {});
        assert!(run(&mut e, 0x005b_e2a0, script(actor)));
        assert_eq!(calls(&e, ACTOR_KILL), vec![vec![actor, 0, 0]]);
        assert!(calls(&e, ACTOR_SCRIPT_DISMEMBER).is_empty());
        // The player drops the part when slot 0x1a0 says so.
        e.mem.set_u32(player + 0x94, part);
        let table = e.mem.u32(player);
        e.mem.set_u32(table + 0x1a0, V_TRUE);
        e.mem.set_u8(player + 0x50, 1);
        parse_gives(&mut e, true, &[0x999, 4, 6]);
        start_log(&mut e);
        assert!(run(&mut e, 0x005b_e2a0, script(player)));
        assert_eq!(calls(&e, ACTOR_KILL).len(), 1);
        assert!(calls(&e, ACTOR_SCRIPT_DISMEMBER).is_empty());
    }

    // ---- The remaining bodies -----------------------------------------------------------------------------------

    #[test]
    fn kill_all_kills_every_actor_in_the_process_lists() {
        let mut e = engine();
        accept(&mut e, &[ACTOR_KILL]);
        // The array: count in its first word (+0x20), actors by index.
        let array = e.mem.alloc(0x40);
        e.mem.set_u32(array + 0x20, 3);
        e.register_double(PROCESS_LISTS_ARRAY, move |_, a| {
            assert_eq!(a[0], PROCESS_LISTS);
            array.into_ret()
        });
        let live = actor_with(&mut e, &[]);
        let inert = object_with(&mut e, &[(0x100, V_FALSE)]);
        let actors = [live, 0, inert];
        e.register_double(PROCESS_LISTS_ACTOR, move |_, a| {
            assert_eq!(a[0], array);
            actors[a[1] as usize].into_ret()
        });
        parse_gives(&mut e, true, &[0x999]);
        start_log(&mut e);
        assert!(run(&mut e, 0x005b_e4f0, script(0)));
        assert_eq!(calls(&e, ACTOR_KILL), vec![vec![live, 0x999, 0]]);
        assert_eq!(calls(&e, PROCESS_LISTS_ACTOR).len(), 3);
        parse_gives(&mut e, false, &[]);
        start_log(&mut e);
        assert!(!run(&mut e, 0x005b_e4f0, script(0)));
        assert!(calls(&e, ACTOR_KILL).is_empty());
    }

    #[test]
    fn body_005be5e0_calls_slot_324_between_the_runner_calls() {
        let mut e = engine();
        accept(&mut e, &[SCRIPT_RUNNER_ENTER, SCRIPT_RUNNER_LEAVE]);
        e.register(ACTOR_FN_0043FCD0, |_, a| a[0].into_ret());
        let actor = actor_with(&mut e, &[(0x324, V_RECORD)]);
        parse_gives(&mut e, true, &[1]);
        start_log(&mut e);
        assert!(run(&mut e, 0x005b_e5e0, script(actor)));
        assert_eq!(calls(&e, SCRIPT_RUNNER_ENTER), vec![vec![SCRIPT_RUNNER, 0]]);
        assert_eq!(calls(&e, V_RECORD), vec![vec![actor, 1, 1, 1]]);
        assert_eq!(calls(&e, SCRIPT_RUNNER_LEAVE), vec![vec![SCRIPT_RUNNER]]);
        // Flag other than 1, and an actor for which 0043fcd0 says 0.
        e.register(ACTOR_FN_0043FCD0, |_, _| 0u32.into_ret());
        parse_gives(&mut e, true, &[2]);
        start_log(&mut e);
        assert!(run(&mut e, 0x005b_e5e0, script(actor)));
        assert_eq!(calls(&e, V_RECORD), vec![vec![actor, 1, 0, 0]]);
        // Not an actor: nothing but true; bad parameters: false.
        start_log(&mut e);
        assert!(run(&mut e, 0x005b_e5e0, script(0)));
        assert!(calls(&e, SCRIPT_RUNNER_ENTER).is_empty());
        parse_gives(&mut e, false, &[]);
        assert!(!run(&mut e, 0x005b_e5e0, script(actor)));
    }

    #[test]
    fn body_005be870_tops_a_value_up_to_the_setting() {
        let mut e = engine();
        let player = object_with(&mut e, &[]);
        e.set_global(PLAYER, player);
        e.set_global(SETTING_011D0844 + 4, 10u32);
        // Slot 0x344 answers 4 (below the setting of 10): slot 0x460 gets 6.0.
        let four = 0x0900_0200;
        e.register(four, |_, _| 4u32.into_ret());
        let actor = actor_with(&mut e, &[(0x344, four), (0x460, V_RECORD)]);
        start_log(&mut e);
        assert!(run(&mut e, 0x005b_e870, script(actor)));
        assert_eq!(calls(&e, four), vec![vec![actor, player, 0]]);
        assert_eq!(
            calls(&e, V_RECORD),
            vec![vec![actor, player, 6.0f32.to_bits()]]
        );
        // At or above the setting: nothing more.
        e.set_global(SETTING_011D0844 + 4, 4u32);
        start_log(&mut e);
        assert!(run(&mut e, 0x005b_e870, script(actor)));
        assert!(calls(&e, V_RECORD).is_empty());
        // Not an actor / no reference: true, nothing.
        let plain = object_with(&mut e, &[(0x100, V_FALSE)]);
        assert!(run(&mut e, 0x005b_e870, script(plain)));
        assert!(run(&mut e, 0x005b_e870, script(0)));
        assert!(calls(&e, V_RECORD).is_empty());
    }

    // ==== Second batch: 005be900 to 005c0bf0 ===========================================

    /// `engine()` with the pages of the globals and constants of the second
    /// batch mapped and the constants the exe provides set.
    fn engine_b() -> Engine {
        let mut e = engine();
        for page in [
            0x0101_3000,
            0x0101_e000,
            0x0101_f000,
            0x0102_3000,
            0x0102_4000,
            0x0103_1000,
            0x0118_c000,
            0x011a_9000,
            0x011a_d000,
            0x011c_a000,
            0x011d_b000,
            0x011f_4000,
            0x011f_6000,
            0x011f_9000,
            0x011f_a000,
        ] {
            e.map(page, 0x1000);
        }
        e.set_global(DOUBLE_FAR_LIMIT, 163840.0f64);
        e.set_global(FLOAT_FAR_LIMIT, 163840.0f32);
        e.set_global(DOUBLE_256, 256.0f64);
        e.set_global(
            DOUBLE_RADIANS_PER_DEGREE,
            f64::from_bits(0x3f91_df46_a000_0000),
        );
        e.set_global(DOUBLE_TWO_PI, f64::from_bits(0x4019_21fb_6000_0000));
        e.set_global(FLOAT_ROUND_RANGE, 4.0f32);
        e.set_global(FLOAT_DECAL_1000, 1000.0f32);
        e.set_global(FLOAT_DECAL_90, 90.0f32);
        e
    }

    /// The standard script words with `this_obj` and a result slot.
    fn script_result(e: &mut Engine, this_obj: u32) -> ScriptArgs {
        let mut a = script(this_obj);
        a.result = Ptr::new(e.mem.alloc(8));
        a
    }

    fn text_of(e: &Engine, address: u32) -> String {
        String::from_utf8(e.mem.cstr(address)).unwrap()
    }

    // ---- Settings ----------------------------------------------------------------

    /// Doubles for the setting accessors. A setting object keeps its type
    /// code at +0x30 and its value at +4; its "name" is the address +0x40.
    /// The setters store into the value.
    fn setting_doubles(e: &mut Engine) {
        e.register(SETTING_TYPE, |e, a| e.mem.u32(a[0] + 0x30).into_ret());
        e.register(SETTING_NAME, |_, a| (a[0] + 0x40).into_ret());
        e.register(SETTING_FLOAT_ADDRESS, |_, a| (a[0] + 4).into_ret());
        e.register(SETTING_BOOL_ADDRESS, |_, a| (a[0] + 4).into_ret());
        e.register(SETTING_STRING_VALUE, |e, a| {
            let value = if a[0] == 0 { 0 } else { e.mem.u32(a[0] + 4) };
            value.into_ret()
        });
        for setter in [SETTING_SET_INT, SETTING_SET_FLOAT, SETTING_SET_STRING] {
            e.register(setter, |e, a| {
                e.mem.set_u32(a[0] + 4, a[1]);
                Ret::default()
            });
        }
        e.register(ATOL, |e, a| {
            let text = text_of(e, a[0]);
            text.trim().parse::<i32>().unwrap_or(0).into_ret()
        });
        e.register(ATOF, |e, a| {
            let text = text_of(e, a[0]);
            text.trim().parse::<f64>().unwrap_or(0.0).into_ret()
        });
    }

    fn new_setting(e: &mut Engine, kind: u32, value: u32) -> u32 {
        let setting = e.mem.alloc(0x50);
        e.mem.set_u32(setting + 0x30, kind);
        e.mem.set_u32(setting + 4, value);
        setting
    }

    /// The game setting collection answers by the first letter of the name.
    fn game_setting_collection(e: &mut Engine, settings: [(u8, u32); 4]) {
        e.register(GAME_SETTINGS, |_, _| 0x0a00_0000u32.into_ret());
        e.register_double(GAME_SETTING_FIND, move |e, a| {
            assert_eq!(a[0], 0x0a00_0000);
            let first = e.mem.u8(a[1]);
            let found = settings.iter().find(|(letter, _)| *letter == first);
            found.map_or(0, |(_, setting)| *setting).into_ret()
        });
    }

    #[test]
    fn body_005be900_reads_a_game_setting_into_the_result() {
        let mut e = engine_b();
        setting_doubles(&mut e);
        let int_setting = new_setting(&mut e, 3, (-7i32) as u32);
        let float_setting = new_setting(&mut e, 5, 2.5f32.to_bits());
        let string_setting = new_setting(&mut e, 6, 0x0900_1234);
        let other_setting = new_setting(&mut e, 1, 0);
        game_setting_collection(
            &mut e,
            [
                (b'i', int_setting),
                (b'f', float_setting),
                (b's', string_setting),
                (b'o', other_setting),
            ],
        );
        set_echo(&mut e, true);
        let script = script_result(&mut e, 0);
        let result = script.result.addr();
        let case = |e: &mut Engine, name: &'static [u8]| {
            parse_with(e, true, move |e, a| e.mem.set_cstr(a[7], name));
            start_log(e);
            e.mem.set_f64(result, 99.0);
            assert!(run(e, 0x005b_e900, script));
            printed(e)
        };
        let prints = case(&mut e, b"iName");
        assert_eq!(
            prints,
            vec![args![MSG_GAME_INT, int_setting + 0x40, (-7i32) as u32]]
        );
        assert_eq!(e.mem.f64(result), -7.0);
        let prints = case(&mut e, b"fName");
        assert_eq!(
            prints,
            vec![args![MSG_GAME_FLOAT, float_setting + 0x40, 2.5f64]]
        );
        assert_eq!(e.mem.f64(result), 2.5);
        let prints = case(&mut e, b"sName");
        assert_eq!(
            prints,
            vec![args![
                MSG_GAME_STRING,
                string_setting + 0x40,
                0x0900_1234u32
            ]]
        );
        assert_eq!(e.mem.f64(result), 0.0);
        let prints = case(&mut e, b"oName");
        assert_eq!(prints, vec![args![MSG_GAME_UNKNOWN, other_setting + 0x40]]);
        // A missing setting: the not-found message with the null setting's
        // string value as the unused extra argument.
        let prints = case(&mut e, b"zName");
        let name = calls(&e, GAME_SETTING_FIND)[0][1];
        assert_eq!(prints, vec![args![MSG_GAME_NOT_FOUND, name, 0u32]]);
        // Without echo only the result is set.
        set_echo(&mut e, false);
        let prints = case(&mut e, b"iName");
        assert!(prints.is_empty());
        assert_eq!(e.mem.f64(result), -7.0);
        // A failed parse leaves the cleared result and returns false.
        parse_with(&mut e, false, |_, _| {});
        e.mem.set_f64(result, 99.0);
        assert!(!run(&mut e, 0x005b_e900, script));
        assert_eq!(e.mem.f64(result), 0.0);
    }

    #[test]
    fn body_005beb00_sets_a_game_setting_by_type() {
        let mut e = engine_b();
        setting_doubles(&mut e);
        let int_setting = new_setting(&mut e, 3, 0);
        let float_setting = new_setting(&mut e, 5, 0);
        let string_setting = new_setting(&mut e, 6, 0);
        let other_setting = new_setting(&mut e, 1, 0);
        game_setting_collection(
            &mut e,
            [
                (b'i', int_setting),
                (b'f', float_setting),
                (b's', string_setting),
                (b'o', other_setting),
            ],
        );
        set_echo(&mut e, true);
        let script = script(0);
        let case = |e: &mut Engine, name: &'static [u8], value: &'static [u8]| {
            parse_with(e, true, move |e, a| {
                e.mem.set_cstr(a[7], name);
                e.mem.set_cstr(a[8], value);
            });
            start_log(e);
            assert!(run(e, 0x005b_eb00, script));
            printed(e)
        };
        let prints = case(&mut e, b"i1", b"12");
        assert_eq!(calls(&e, SETTING_SET_INT), vec![vec![int_setting, 12]]);
        assert_eq!(prints, vec![args![MSG_GAME_INT, int_setting + 0x40, 12u32]]);
        let prints = case(&mut e, b"f1", b"1.5");
        assert_eq!(
            calls(&e, SETTING_SET_FLOAT),
            vec![vec![float_setting, 1.5f32.to_bits()]]
        );
        assert_eq!(
            prints,
            vec![args![MSG_GAME_FLOAT, float_setting + 0x40, 1.5f64]]
        );
        let prints = case(&mut e, b"s1", b"text");
        let value_text = calls(&e, SETTING_SET_STRING)[0][1];
        assert_eq!(text_of(&e, value_text), "text");
        assert_eq!(
            prints,
            vec![args![MSG_GAME_STRING, string_setting + 0x40, value_text]]
        );
        // Unknown type and missing setting are reported with the typed name
        // buffer, also without echo.
        set_echo(&mut e, false);
        let prints = case(&mut e, b"o1", b"1");
        let name = calls(&e, GAME_SETTING_FIND)[0][1];
        assert_eq!(prints, vec![args![MSG_GAME_UNKNOWN, name]]);
        let prints = case(&mut e, b"z1", b"1");
        let name = calls(&e, GAME_SETTING_FIND)[0][1];
        assert_eq!(prints, vec![args![MSG_GAME_NOT_FOUND, name]]);
        // Without echo a successful change prints nothing.
        let prints = case(&mut e, b"i1", b"5");
        assert!(prints.is_empty());
        assert_eq!(calls(&e, SETTING_SET_INT), vec![vec![int_setting, 5]]);
        parse_with(&mut e, false, |_, _| {});
        assert!(!run(&mut e, 0x005b_eb00, script));
    }

    #[test]
    fn body_005becf0_prints_an_ini_setting_by_type() {
        let mut e = engine_b();
        setting_doubles(&mut e);
        let bool_setting = new_setting(&mut e, 0, 1);
        let int_setting = new_setting(&mut e, 3, 9);
        let float_setting = new_setting(&mut e, 5, 0.25f32.to_bits());
        let string_setting = new_setting(&mut e, 6, 0x0900_4321);
        let other_setting = new_setting(&mut e, 2, 0);
        // `a<digit>` is found in the first collection, `b<digit>` in the
        // second; the digit picks the setting.
        let by_digit = [
            bool_setting,
            int_setting,
            float_setting,
            string_setting,
            other_setting,
        ];
        e.register_double(INI_SETTINGS, |_, _| 0x0a00_0010u32.into_ret());
        e.register_double(INI_SETTINGS_FALLBACK, |_, _| 0x0a00_0020u32.into_ret());
        e.register_double(INI_SETTING_FIND, move |e, a| {
            let (letter, digit) = (e.mem.u8(a[1]), e.mem.u8(a[1] + 1));
            let found = match (a[0], letter) {
                (0x0a00_0010, b'a') | (0x0a00_0020, b'b') => by_digit[(digit - b'0') as usize],
                _ => 0,
            };
            found.into_ret()
        });
        let script = script(0);
        let case = |e: &mut Engine, name: &'static [u8]| {
            parse_with(e, true, move |e, a| e.mem.set_cstr(a[7], name));
            start_log(e);
            assert!(run(e, 0x005b_ecf0, script));
            let name = calls(e, INI_SETTING_FIND)[0][1];
            (printed(e), name)
        };
        // Prints even though commands do not echo.
        set_echo(&mut e, false);
        let (prints, name) = case(&mut e, b"a0");
        assert_eq!(prints, vec![args![MSG_INI_INT, name, 1u32]]);
        assert_eq!(calls(&e, INI_SETTINGS_FALLBACK).len(), 0);
        let (prints, name) = case(&mut e, b"a1");
        assert_eq!(prints, vec![args![MSG_INI_INT, name, 9u32]]);
        let (prints, name) = case(&mut e, b"a2");
        assert_eq!(prints, vec![args![MSG_INI_FLOAT, name, 0.25f64]]);
        let (prints, name) = case(&mut e, b"a3");
        assert_eq!(prints, vec![args![MSG_INI_STRING, name, 0x0900_4321u32]]);
        let (prints, name) = case(&mut e, b"a4");
        assert_eq!(prints, vec![args![MSG_INI_UNKNOWN, name]]);
        // The second collection is asked when the first has none.
        let (prints, name) = case(&mut e, b"b1");
        assert_eq!(calls(&e, INI_SETTING_FIND).len(), 2);
        assert_eq!(calls(&e, INI_SETTING_FIND)[1][0], 0x0a00_0020);
        assert_eq!(prints, vec![args![MSG_INI_INT, name, 9u32]]);
        let (prints, name) = case(&mut e, b"c1");
        assert_eq!(prints, vec![args![MSG_INI_NOT_FOUND, name]]);
        parse_with(&mut e, false, |_, _| {});
        assert!(!run(&mut e, 0x005b_ecf0, script));
    }

    #[test]
    fn body_005bee90_sets_an_ini_setting_by_type() {
        let mut e = engine_b();
        setting_doubles(&mut e);
        let bool_setting = new_setting(&mut e, 0, 0);
        let int_setting = new_setting(&mut e, 3, 0);
        let float_setting = new_setting(&mut e, 5, 0);
        let string_setting = new_setting(&mut e, 6, 0);
        let other_setting = new_setting(&mut e, 2, 0);
        let by_digit = [
            bool_setting,
            int_setting,
            float_setting,
            string_setting,
            other_setting,
        ];
        e.register_double(INI_SETTINGS, |_, _| 0x0a00_0010u32.into_ret());
        e.register_double(INI_SETTINGS_FALLBACK, |_, _| 0x0a00_0020u32.into_ret());
        e.register_double(INI_SETTING_FIND, move |e, a| {
            let (letter, digit) = (e.mem.u8(a[1]), e.mem.u8(a[1] + 1));
            let found = match (a[0], letter) {
                (0x0a00_0010, b'a') | (0x0a00_0020, b'b') => by_digit[(digit - b'0') as usize],
                _ => 0,
            };
            found.into_ret()
        });
        let script = script(0);
        let case = |e: &mut Engine, name: &'static [u8], value: &'static [u8]| {
            parse_with(e, true, move |e, a| {
                e.mem.set_cstr(a[7], name);
                e.mem.set_cstr(a[8], value);
            });
            start_log(e);
            assert!(run(e, 0x005b_ee90, script));
            let name = calls(e, INI_SETTING_FIND)[0][1];
            (printed(e), name)
        };
        set_echo(&mut e, true);
        // The boolean (type 0) goes through the integer setter and echoes the
        // byte at its value as 0 or 1.
        let (prints, name) = case(&mut e, b"a0", b"1");
        assert_eq!(calls(&e, SETTING_SET_INT), vec![vec![bool_setting, 1]]);
        assert_eq!(prints, vec![args![MSG_INI_INT, name, 1u32]]);
        let (prints, name) = case(&mut e, b"a1", b"42");
        assert_eq!(calls(&e, SETTING_SET_INT), vec![vec![int_setting, 42]]);
        assert_eq!(prints, vec![args![MSG_INI_INT, name, 42u32]]);
        let (prints, name) = case(&mut e, b"a2", b"0.5");
        assert_eq!(
            calls(&e, SETTING_SET_FLOAT),
            vec![vec![float_setting, 0.5f32.to_bits()]]
        );
        assert_eq!(prints, vec![args![MSG_INI_FLOAT, name, 0.5f64]]);
        let (prints, name) = case(&mut e, b"b3", b"words");
        let value_text = calls(&e, SETTING_SET_STRING)[0][1];
        assert_eq!(text_of(&e, value_text), "words");
        assert_eq!(calls(&e, INI_SETTING_FIND).len(), 2);
        assert_eq!(prints, vec![args![MSG_INI_STRING, name, value_text]]);
        // Unknown type and missing setting are always reported; no echo of
        // successful changes.
        set_echo(&mut e, false);
        let (prints, name) = case(&mut e, b"a4", b"1");
        assert_eq!(prints, vec![args![MSG_INI_UNKNOWN, name]]);
        let (prints, name) = case(&mut e, b"c1", b"1");
        assert_eq!(prints, vec![args![MSG_INI_NOT_FOUND, name]]);
        let (prints, _) = case(&mut e, b"a1", b"7");
        assert!(prints.is_empty());
        assert_eq!(calls(&e, SETTING_SET_INT), vec![vec![int_setting, 7]]);
        parse_with(&mut e, false, |_, _| {});
        assert!(!run(&mut e, 0x005b_ee90, script));
    }

    // ---- The render options command and its helpers -------------------------

    /// Doubles of the helpers of `005bf130`. The strings the options are
    /// compared with are put into memory, `011f91c0` holds the render setting
    /// `004dc060(0)` answers, the render flags word is `011f91d8`.
    fn render_engine(setting: i32) -> Engine {
        let mut e = engine_b();
        for (address, option) in [
            (OPTION_TEX, &b"tex"[..]),
            (OPTION_DEGRADE, b"degrade"),
            (OPTION_SH, b"sh"),
            (OPTION_SC, b"sc"),
            (OPTION_PSH, b"psh"),
            (OPTION_DSH, b"dsh"),
            (OPTION_OPT, b"opt"),
            (OPTION_ALPHA, b"alpha"),
        ] {
            e.mem.set_cstr(address, option);
        }
        e.register(STRING_COMPARE_N_NO_CASE, |e, a| {
            let n = a[2] as usize;
            let lower = |bytes: Vec<u8>| {
                bytes
                    .into_iter()
                    .take(n)
                    .map(|c| c.to_ascii_lowercase())
                    .collect::<Vec<_>>()
            };
            let (first, second) = (lower(e.mem.cstr(a[0])), lower(e.mem.cstr(a[1])));
            (first != second).into_ret()
        });
        e.register(STRLEN, |e, a| (e.mem.cstr(a[0]).len() as u32).into_ret());
        e.set_global(0x011f_91c0u32, setting);
        e.register(RENDER_SETTING, |e, a| {
            let address = if a[0] == 0 { 0x011f_91c0 } else { 0x011f_91bc };
            e.mem.u32(address).into_ret()
        });
        e.register(RENDER_FLAGS_GET, |e, _| {
            e.global::<u32>(RENDER_FLAGS).into_ret()
        });
        e.register(RENDER_FLAGS_SET, |e, a| {
            e.set_global(RENDER_FLAGS, a[0]);
            Ret::default()
        });
        e.register(MIN_I32, |_, a| (a[0] as i32).min(a[1] as i32).into_ret());
        e.register(MAX_I32, |_, a| (a[0] as i32).max(a[1] as i32).into_ret());
        e.register(WORD_AT_0X8, |e, a| e.mem.u32(a[0] + 8).into_ret());
        e.register(STRING_TEXT, |e, a| e.mem.u32(a[0]).into_ret());
        e.register(NODE_FLAG, |e, a| e.mem.u8(a[0] + 0x124).into_ret());
        e.register(ATOL, |e, a| {
            let text = text_of(e, a[0]);
            text.trim().parse::<i32>().unwrap_or(0).into_ret()
        });
        accept(
            &mut e,
            &[
                SHADER_SET_MODE,
                SHADER_REFRESH,
                RENDERER_SET_BYTE,
                PSH_ENABLE,
                PSH_DISABLE,
            ],
        );
        e
    }

    /// Runs `005bf130` with the option text.
    fn render_option_run(e: &mut Engine, this_obj: u32, option: &'static [u8]) -> bool {
        parse_with(e, true, move |e, a| e.mem.set_cstr(a[7], option));
        start_log(e);
        run(e, 0x005b_f130, script(this_obj))
    }

    #[test]
    fn body_005bf130_digit_options_select_the_mode_and_flags() {
        let mut e = render_engine(1);
        // `1`: mode 0 in the shader manager, 0 in 011f91b4, the single
        // character sets bits 0xe (and not 0x20 at render setting 1).
        assert!(render_option_run(&mut e, 0, b"1"));
        assert_eq!(calls(&e, SHADER_SET_MODE), vec![vec![0]]);
        assert_eq!(e.global::<u32>(GLOBAL_011F91B4), 0);
        assert_eq!(e.global::<u32>(RENDER_FLAGS), 0x0f);
        // At render setting 2 the single character adds 0x20 (and the text
        // after it adds nothing).
        e.set_global(0x011f_91c0u32, 2);
        assert!(render_option_run(&mut e, 0, b"1"));
        assert_eq!(e.global::<u32>(RENDER_FLAGS), 0x2f);
        // `4` selects shader mode 3.
        assert!(render_option_run(&mut e, 0, b"4"));
        assert_eq!(calls(&e, SHADER_SET_MODE), vec![vec![3]]);
        assert_eq!(e.global::<u32>(RENDER_FLAGS), 0x2f);
        // `2` and `3` store 1 and 2 and keep the bits of the characters after
        // them: `2110` sets bits 2 and 4; at render setting 2 the fifth
        // character not being `0` adds 0x20.
        assert!(render_option_run(&mut e, 0, b"21100"));
        assert_eq!(e.global::<u32>(GLOBAL_011F91B4), 1);
        assert_eq!(e.global::<u32>(RENDER_FLAGS), 0x07);
        assert!(render_option_run(&mut e, 0, b"31111"));
        assert_eq!(e.global::<u32>(GLOBAL_011F91B4), 2);
        assert_eq!(e.global::<u32>(RENDER_FLAGS), 0x2f);
        // `5` only stores the bits of the characters after it.
        assert!(render_option_run(&mut e, 0, b"50100"));
        assert_eq!(e.global::<u32>(RENDER_FLAGS), 0x04);
        // Failed parse: false, nothing stored.
        e.set_global(RENDER_FLAGS, 0x1234u32);
        parse_with(&mut e, false, |_, _| {});
        assert!(!run(&mut e, 0x005b_f130, script(0)));
        assert_eq!(e.global::<u32>(RENDER_FLAGS), 0x1234);
    }

    #[test]
    fn body_005bf130_named_options() {
        let mut e = render_engine(1);
        assert!(render_option_run(&mut e, 0, b"tex"));
        assert_eq!(e.global::<u32>(GLOBAL_011F91B4), 4);
        assert_eq!(e.global::<u32>(RENDER_FLAGS), 1);
        assert!(render_option_run(&mut e, 0, b"DEGRADE"));
        assert_eq!(e.global::<u32>(GLOBAL_011F91B4), 5);
        assert_eq!(e.global::<u32>(RENDER_FLAGS), 1);
        // `sh` toggles: at render setting 1 `005bf7b0` is false, so bit 0x10
        // is set, then bit 0x80 flips.
        e.set_global(RENDER_FLAGS, 0u32);
        assert!(render_option_run(&mut e, 0, b"sh"));
        assert_eq!(e.global::<u32>(RENDER_FLAGS), 0x90);
        e.set_global(RENDER_FLAGS, 0x80u32);
        assert!(render_option_run(&mut e, 0, b"sh"));
        assert_eq!(e.global::<u32>(RENDER_FLAGS), 0x10);
        // `sh<digit>`: the level is limited to 0..=3 and 1 and 2 become 3.
        e.set_global(RENDER_FLAGS, 0u32);
        assert!(render_option_run(&mut e, 0, b"sh2"));
        assert_eq!(e.global::<u32>(GLOBAL_011F91B0), 3);
        assert_eq!(calls(&e, SHADER_REFRESH).len(), 1);
        assert_eq!(e.global::<u32>(RENDER_FLAGS), 0x80);
        assert!(render_option_run(&mut e, 0, b"sh0"));
        assert_eq!(e.global::<u32>(GLOBAL_011F91B0), 0);
        assert!(render_option_run(&mut e, 0, b"sh9"));
        assert_eq!(e.global::<u32>(GLOBAL_011F91B0), 3);
        // `sc` and `opt` toggle their bytes.
        e.set_global(RENDER_FLAGS, 0x40u32);
        assert!(render_option_run(&mut e, 0, b"sc"));
        assert_eq!(e.global::<u8>(GLOBAL_011F9441), 1);
        assert_eq!(e.global::<u32>(RENDER_FLAGS), 0x40);
        assert!(render_option_run(&mut e, 0, b"sc"));
        assert_eq!(e.global::<u8>(GLOBAL_011F9441), 0);
        assert!(render_option_run(&mut e, 0, b"opt"));
        assert_eq!(e.global::<u8>(GLOBAL_011AD80D), 1);
        assert_eq!(e.global::<u32>(RENDER_FLAGS), 0x140);
        // `t<number>` stores the number.
        assert!(render_option_run(&mut e, 0, b"t12"));
        assert_eq!(e.global::<u32>(GLOBAL_011AD828), 12);
        // `alpha` and unknown texts: only the latter store the flags.
        e.set_global(RENDER_FLAGS, 0x55u32);
        assert!(render_option_run(&mut e, 0, b"alpha"));
        assert_eq!(e.global::<u32>(RENDER_FLAGS), 0x55);
        assert!(render_option_run(&mut e, 0, b"zzz"));
        assert_eq!(e.global::<u32>(RENDER_FLAGS), 0);
    }

    #[test]
    fn body_005bf130_psh_toggles_and_dsh_flags_the_shadow_nodes() {
        let mut e = render_engine(1);
        let tes = e.mem.alloc(0x20);
        let target = 0x0a00_7777;
        e.mem.set_u32(tes + 8, target);
        e.set_global(TES_SINGLETON, tes);
        // `psh` with bit 0x20 clear: renderer byte 1, the call on the word at
        // +8 of the TES singleton, bit 0x20 set; no further flags.
        e.set_global(RENDER_FLAGS, 0x100u32);
        assert!(render_option_run(&mut e, 0, b"psh11"));
        assert_eq!(calls(&e, RENDERER_SET_BYTE), vec![vec![RENDERER_OBJECT, 1]]);
        assert_eq!(calls(&e, PSH_ENABLE), vec![vec![target]]);
        assert_eq!(e.global::<u32>(RENDER_FLAGS), 0x120);
        // With bit 0x20 set: byte 0, `00451570(0)`, bit cleared.
        assert!(render_option_run(&mut e, 0, b"psh"));
        assert_eq!(calls(&e, RENDERER_SET_BYTE), vec![vec![RENDERER_OBJECT, 0]]);
        assert_eq!(calls(&e, PSH_DISABLE), vec![vec![0]]);
        assert_eq!(e.global::<u32>(RENDER_FLAGS), 0x100);
        // `dsh` toggles the byte of the nodes whose word matches the
        // reference's slot 0x1d0 value.
        let reference = object_with(&mut e, &[(0x1d0, V_FLOAT)]);
        e.register(V_FLOAT, |_, _| 0x7001u32.into_ret());
        let nodes: Vec<u32> = (0..3).map(|_| e.mem.alloc(0x140)).collect();
        for (node, word) in nodes.iter().zip([0x7001u32, 0x7002, 0x7001]) {
            e.mem.set_u32(node + 0x128, word);
        }
        e.mem.set_u8(nodes[2] + 0x124, 1);
        let list = 0x0a00_8888;
        let order = nodes.clone();
        e.register_double(SHADOW_LIST_GET, move |_, a| {
            assert_eq!(a[0], 0);
            list.into_ret()
        });
        e.register_double(SHADOW_LIST_FIRST, {
            let first = order[0];
            move |_, a| {
                assert_eq!(a[0], list);
                first.into_ret()
            }
        });
        let position = Rc::new(Cell::new(0usize));
        e.register_double(SHADOW_LIST_NEXT, move |_, _| {
            position.set(position.get() + 1);
            order.get(position.get()).copied().unwrap_or(0).into_ret()
        });
        e.set_global(RENDER_FLAGS, 0x100u32);
        assert!(render_option_run(&mut e, reference, b"dsh"));
        assert_eq!(e.mem.u8(nodes[0] + 0x124), 1);
        assert_eq!(e.mem.u8(nodes[1] + 0x124), 0);
        assert_eq!(e.mem.u8(nodes[2] + 0x124), 0);
        assert_eq!(e.global::<u32>(RENDER_FLAGS), 0x100);
        // Without a reference nothing happens.
        start_log(&mut e);
        assert!(render_option_run(&mut e, 0, b"dsh"));
        assert!(calls(&e, SHADOW_LIST_GET).is_empty());
    }

    #[test]
    fn body_005bf770_sets_or_clears_bit_0x10() {
        let mut e = engine_b();
        e.set_global(RENDER_FLAGS, 0x0fu32);
        e.call(0x005b_f770, &args![1u32]);
        assert_eq!(e.global::<u32>(RENDER_FLAGS), 0x1f);
        e.call(0x005b_f770, &args![0u32]);
        assert_eq!(e.global::<u32>(RENDER_FLAGS), 0x0f);
    }

    #[test]
    fn body_005bf7b0_needs_render_setting_3_and_bit_0x10() {
        let mut e = engine_b();
        e.register(RENDER_SETTING, |e, _| e.mem.u32(0x011f_91c0).into_ret());
        e.set_global(RENDER_FLAGS, 0x10u32);
        e.set_global(0x011f_91c0u32, 3);
        assert!(e.call(0x005b_f7b0, &args![]).bool());
        e.set_global(RENDER_FLAGS, 0u32);
        assert!(!e.call(0x005b_f7b0, &args![]).bool());
        e.set_global(RENDER_FLAGS, 0x10u32);
        e.set_global(0x011f_91c0u32, 2);
        assert!(!e.call(0x005b_f7b0, &args![]).bool());
    }

    #[test]
    fn body_005bf7f0_stores_the_word() {
        let mut e = engine_b();
        e.call(0x005b_f7f0, &args![5u32]);
        assert_eq!(e.global::<u32>(GLOBAL_011F91B4), 5);
    }

    #[test]
    fn body_005bf800_stores_the_byte_at_0x124() {
        let mut e = engine_b();
        let node = e.mem.alloc(0x140);
        e.call(0x005b_f800, &args![node, 1u32]);
        assert_eq!(e.mem.u8(node + 0x124), 1);
        e.call(0x005b_f800, &args![node, 0u32]);
        assert_eq!(e.mem.u8(node + 0x124), 0);
    }

    #[test]
    fn body_005bf820_reads_the_word_at_0x128() {
        let mut e = engine_b();
        e.register(STRING_TEXT, |e, a| e.mem.u32(a[0]).into_ret());
        let node = e.mem.alloc(0x140);
        e.mem.set_u32(node + 0x128, 0x1234);
        assert_eq!(e.call(0x005b_f820, &args![node]).u32(), 0x1234);
    }

    #[test]
    fn body_005bf840_passes_the_byte_to_the_renderer_object() {
        let mut e = engine_b();
        accept(&mut e, &[RENDERER_SET_BYTE]);
        start_log(&mut e);
        e.call(0x005b_f840, &args![1u32]);
        assert_eq!(calls(&e, RENDERER_SET_BYTE), vec![vec![RENDERER_OBJECT, 1]]);
    }

    #[test]
    fn body_005bf860_toggles_the_byte() {
        let mut e = engine_b();
        assert!(e.call(0x005b_f860, &args![]).bool());
        assert_eq!(e.global::<u8>(GLOBAL_0118C000), 1);
        assert!(e.call(0x005b_f860, &args![]).bool());
        assert_eq!(e.global::<u8>(GLOBAL_0118C000), 0);
    }

    #[test]
    fn body_005bf880_sets_the_byte_to_one_unless_it_is_one() {
        let mut e = engine_b();
        assert!(e.call(0x005b_f880, &args![]).bool());
        assert_eq!(e.global::<u8>(GLOBAL_011F91DC), 1);
        assert!(e.call(0x005b_f880, &args![]).bool());
        assert_eq!(e.global::<u8>(GLOBAL_011F91DC), 0);
        e.set_global(GLOBAL_011F91DC, 7u8);
        assert!(e.call(0x005b_f880, &args![]).bool());
        assert_eq!(e.global::<u8>(GLOBAL_011F91DC), 1);
    }

    #[test]
    fn body_005bf8b0_and_005bf8c0_read_and_write_the_byte() {
        let mut e = engine_b();
        e.call(0x005b_f8c0, &args![9u32]);
        assert_eq!(e.global::<u8>(GLOBAL_011F91DC), 9);
        assert_eq!(e.call(0x005b_f8b0, &args![]).u8(), 9);
    }

    // ---- Screen positions -----------------------------------------------------

    /// The chain `005bf8d0` and `005bf9e0` walk from the `TES` singleton:
    /// returns `(target, holder)` with `word(word(tes + 0x68) + 0x20) =
    /// holder` and `word(holder + 0xc) = target`.
    fn tes_chain(e: &mut Engine) -> (u32, u32) {
        e.register(WORD_AT_0X68, |e, a| e.mem.u32(a[0] + 0x68).into_ret());
        e.register(WORD_AT_0X20, |e, a| e.mem.u32(a[0] + 0x20).into_ret());
        e.register(WORD_AT_0XC, |e, a| e.mem.u32(a[0] + 0xc).into_ret());
        let tes = e.mem.alloc(0x80);
        let band = e.mem.alloc(0x80);
        let holder = e.mem.alloc(0x80);
        let target = e.mem.alloc(0x80);
        e.mem.set_u32(tes + 0x68, band);
        e.mem.set_u32(band + 0x20, holder);
        e.mem.set_u32(holder + 0xc, target);
        e.set_global(TES_SINGLETON, tes);
        (target, holder)
    }

    #[test]
    fn body_005bf8d0_stores_the_numbers_as_floats() {
        let mut e = engine_b();
        let (target, holder) = tes_chain(&mut e);
        parse_gives(&mut e, true, &[3, (-4i32) as u32]);
        assert!(run(&mut e, 0x005b_f8d0, script(0)));
        assert_eq!(e.mem.f32(target + 0x2c), 3.0);
        assert_eq!(e.mem.f32(target + 0x30), -4.0);
        assert_eq!(e.mem.u8(holder + 0x18), 0);
        // Both zero: the holder's byte is 1.
        parse_gives(&mut e, true, &[0, 0]);
        assert!(run(&mut e, 0x005b_f8d0, script(0)));
        assert_eq!(e.mem.f32(target + 0x2c), 0.0);
        assert_eq!(e.mem.u8(holder + 0x18), 1);
        // No target: nothing stored, still true.
        e.mem.set_u32(holder + 0xc, 0);
        e.mem.set_u8(holder + 0x18, 0);
        parse_gives(&mut e, true, &[0, 0]);
        assert!(run(&mut e, 0x005b_f8d0, script(0)));
        assert_eq!(e.mem.u8(holder + 0x18), 0);
        // Failed parse.
        parse_gives(&mut e, false, &[]);
        assert!(!run(&mut e, 0x005b_f8d0, script(0)));
    }

    #[test]
    fn body_005bf9a0_and_005bf9c0_store_into_the_object() {
        let mut e = engine_b();
        let object = e.mem.alloc(0x40);
        e.call(0x005b_f9a0, &args![object, 1.5f32, -2.5f32]);
        assert_eq!(e.mem.f32(object + 0x2c), 1.5);
        assert_eq!(e.mem.f32(object + 0x30), -2.5);
        e.call(0x005b_f9c0, &args![object, 1u32]);
        assert_eq!(e.mem.u8(object + 0x18), 1);
    }

    /// The camera doubles of `005bf9e0`: returns `(camera, camera holder, holder)`.
    /// The frustum has near 2.0 and far 100.0; `NiCamera::SetViewFrustum`
    /// copies the far distance to the camera's word at +0x200.
    fn camera_engine(e: &mut Engine) -> (u32, u32, u32) {
        let (_target, holder) = tes_chain(e);
        let camera_holder = e.mem.alloc(0xc0);
        let camera = e.mem.alloc(0x200);
        e.mem.set_u32(camera_holder + 0xac, camera);
        e.mem.set_f32(camera + 0xdc + 0x10, 2.0);
        e.mem.set_f32(camera + 0xdc + 0x14, 100.0);
        e.register_double(CAMERA_HOLDER, move |_, _| camera_holder.into_ret());
        e.register(WORD_AT_0XAC, |e, a| e.mem.u32(a[0] + 0xac).into_ret());
        e.register(CAMERA_FRUSTUM, |_, a| (a[0] + 0xdc).into_ret());
        e.register(CAMERA_SET_RATIO, |e, a| {
            e.mem.set_u32(a[0] + 0xfc, a[1]);
            Ret::default()
        });
        e.register(CAMERA_SET_VIEW_FRUSTUM, |e, a| {
            let far = e.mem.u32(a[1] + 0x14);
            e.mem.set_u32(a[0] + 0x1fc, far);
            Ret::default()
        });
        (camera, camera_holder, holder)
    }

    #[test]
    fn body_005bf9e0_sets_the_far_distance() {
        let mut e = engine_b();
        let (camera, camera_holder, holder) = camera_engine(&mut e);
        parse_gives(&mut e, true, &[5000.0f32.to_bits()]);
        start_log(&mut e);
        assert!(run(&mut e, 0x005b_f9e0, script(0)));
        // The ratio far / near goes to the camera; the frustum copy is
        // handed to SetViewFrustum with the new far distance.
        assert_eq!(e.mem.f32(camera + 0xfc), 2500.0);
        assert_eq!(e.mem.f32(camera + 0x1fc), 5000.0);
        assert_eq!(calls(&e, CAMERA_SET_VIEW_FRUSTUM).len(), 1);
        assert_eq!(e.mem.u8(holder + 0x18), 0);
        // The same distance again changes nothing.
        e.mem.set_f32(camera + 0xdc + 0x14, 5000.0);
        start_log(&mut e);
        assert!(run(&mut e, 0x005b_f9e0, script(0)));
        assert!(calls(&e, CAMERA_SET_RATIO).is_empty());
        // Zero or less becomes 163840.0 and sets the holder's byte.
        parse_gives(&mut e, true, &[(-1.0f32).to_bits()]);
        assert!(run(&mut e, 0x005b_f9e0, script(0)));
        assert_eq!(e.mem.f32(camera + 0x1fc), 163840.0);
        assert_eq!(e.mem.f32(camera + 0xfc), 81920.0);
        assert_eq!(e.mem.u8(holder + 0x18), 1);
        // Above the limit it is limited without the byte.
        e.mem.set_f32(camera + 0xdc + 0x14, 1.0);
        parse_gives(&mut e, true, &[1.0e6f32.to_bits()]);
        assert!(run(&mut e, 0x005b_f9e0, script(0)));
        assert_eq!(e.mem.f32(camera + 0x1fc), 163840.0);
        assert_eq!(e.mem.u8(holder + 0x18), 0);
        // No camera: nothing; failed parse: false.
        e.mem.set_u32(camera_holder + 0xac, 0);
        start_log(&mut e);
        parse_gives(&mut e, true, &[7.0f32.to_bits()]);
        assert!(run(&mut e, 0x005b_f9e0, script(0)));
        assert!(calls(&e, CAMERA_SET_RATIO).is_empty());
        parse_gives(&mut e, false, &[]);
        assert!(!run(&mut e, 0x005b_f9e0, script(0)));
    }

    #[test]
    fn body_005bfaf0_hands_three_floats_to_the_interface_object() {
        let mut e = engine_b();
        e.register(SETTING_FLOAT_ADDRESS, |_, a| (a[0] + 4).into_ret());
        e.set_global(SETTING_011DB200 + 4, 1.5f32);
        e.set_global(SETTING_011DB238 + 4, 2.5f32);
        e.set_global(SETTING_011DB25C + 4, 3.5f32);
        let object = e.mem.alloc(0x80);
        e.register_double(INTERFACE_OBJECT, move |_, _| object.into_ret());
        accept(&mut e, &[INTERFACE_OBJECT_FN_004EE4B0]);
        // The first number is parsed; the other two keep their defaults,
        // which the outputs hold before the parse.
        let seen = Rc::new(RefCell::new(vec![]));
        let log = seen.clone();
        parse_with(&mut e, true, move |e, a| {
            for i in 0..4 {
                log.borrow_mut().push(e.mem.u32(a[7 + i]));
            }
            e.mem.set_f32(a[7], 9.0);
        });
        start_log(&mut e);
        assert!(run(&mut e, 0x005b_faf0, script(0)));
        assert_eq!(
            seen.borrow()[..3],
            [1.5f32.to_bits(), 2.5f32.to_bits(), 3.5f32.to_bits()]
        );
        assert_eq!(e.mem.f32(object + 0x58), 9.0);
        assert_eq!(e.mem.f32(object + 0x60), 2.5);
        assert_eq!(
            calls(&e, INTERFACE_OBJECT_FN_004EE4B0),
            vec![vec![object, 3.5f32.to_bits()]]
        );
        // The three settings are read before the parse; a failed parse stops
        // before the object is touched.
        parse_with(&mut e, false, |_, _| {});
        start_log(&mut e);
        assert!(!run(&mut e, 0x005b_faf0, script(0)));
        assert_eq!(calls(&e, SETTING_FLOAT_ADDRESS).len(), 3);
        assert!(calls(&e, INTERFACE_OBJECT).is_empty());
    }

    #[test]
    fn body_005bfbb0_and_005bfbd0_store_floats() {
        let mut e = engine_b();
        let object = e.mem.alloc(0x80);
        e.call(0x005b_fbb0, &args![object, 1.25f32]);
        e.call(0x005b_fbd0, &args![object, -3.0f32]);
        assert_eq!(e.mem.f32(object + 0x58), 1.25);
        assert_eq!(e.mem.f32(object + 0x60), -3.0);
    }

    #[test]
    fn script_set_tree_mipmap_bias_prints_the_old_and_new_values() {
        let mut e = engine_b();
        e.set_global(TREE_MIPMAP_LOCAL, 1.5f32);
        e.set_global(TREE_MIPMAP_LOD, 2.5f32);
        accept(
            &mut e,
            &[
                BS_STRING_CONSTRUCT,
                BS_STRING_DESTRUCT,
                BS_STRING_FORMAT,
                PRINT_LINE,
            ],
        );
        // The copy constructor copies the 8-byte string.
        e.register(BS_STRING_COPY, |e, a| {
            let (first, second) = (e.mem.u32(a[1]), e.mem.u32(a[1] + 4));
            e.mem.set_u32(a[0], first);
            e.mem.set_u32(a[0] + 4, second);
            Ret::default()
        });
        let defaults = Rc::new(RefCell::new(vec![]));
        let log = defaults.clone();
        parse_with(&mut e, true, move |e, a| {
            log.borrow_mut().push(e.mem.u32(a[7]));
            log.borrow_mut().push(e.mem.u32(a[8]));
            e.mem.set_f32(a[7], 0.5);
            e.mem.set_f32(a[8], 4.0);
        });
        start_log(&mut e);
        assert!(run(&mut e, 0x005b_fbf0, script(0)));
        // The parameters default to the local trees' then the LOD trees'
        // value.
        assert_eq!(*defaults.borrow(), vec![1.5f32.to_bits(), 2.5f32.to_bits()]);
        let string = calls(&e, BS_STRING_CONSTRUCT)[0][0];
        assert_eq!(
            calls(&e, BS_STRING_FORMAT),
            vec![
                args![string, MSG_OLD_VALUES, 1.5f64, 2.5f64],
                args![string, MSG_NEW_VALUES, 0.5f64, 4.0f64],
            ]
        );
        assert_eq!(calls(&e, PRINT_LINE).len(), 2);
        assert_eq!(calls(&e, BS_STRING_DESTRUCT), vec![vec![string]]);
        assert_eq!(e.global::<f32>(TREE_MIPMAP_LOCAL), 0.5);
        assert_eq!(e.global::<f32>(TREE_MIPMAP_LOD), 4.0);
        // A failed parse changes nothing.
        parse_with(&mut e, false, |_, _| {});
        start_log(&mut e);
        assert!(!run(&mut e, 0x005b_fbf0, script(0)));
        assert!(calls(&e, BS_STRING_CONSTRUCT).is_empty());
        assert_eq!(e.global::<f32>(TREE_MIPMAP_LOCAL), 0.5);
    }

    #[test]
    fn body_005bfd40_calls_the_interface_action() {
        let mut e = engine_b();
        accept(&mut e, &[INTERFACE_ACTION]);
        start_log(&mut e);
        assert!(e.call(0x005b_fd40, &args![]).bool());
        assert_eq!(calls(&e, INTERFACE_ACTION), vec![Vec::<u32>::new()]);
    }

    #[test]
    fn script_force_close_file_closes_a_known_file() {
        let mut e = engine_b();
        let handler = e.mem.alloc(0x40);
        e.set_global(DATA_HANDLER, handler);
        let file = 0x0a00_4444;
        e.register_double(DATA_HANDLER_GET_LIST_FILE, move |e, a| {
            assert_eq!(a[0], handler);
            let known = text_of(e, a[1]) == "good.esp";
            (if known { file } else { 0 }).into_ret()
        });
        accept(&mut e, &[DATA_HANDLER_SET_BYTE, FILE_CLOSE]);
        set_echo(&mut e, true);
        parse_with(&mut e, true, |e, a| e.mem.set_cstr(a[7], b"good.esp"));
        start_log(&mut e);
        assert!(run(&mut e, 0x005b_fd50, script(0)));
        // The file is closed between the two byte stores.
        let order: Vec<u32> = e
            .call_log
            .as_ref()
            .unwrap()
            .iter()
            .map(|(address, _)| *address)
            .filter(|address| [DATA_HANDLER_SET_BYTE, FILE_CLOSE].contains(address))
            .collect();
        assert_eq!(
            order,
            vec![DATA_HANDLER_SET_BYTE, FILE_CLOSE, DATA_HANDLER_SET_BYTE]
        );
        assert_eq!(
            calls(&e, DATA_HANDLER_SET_BYTE),
            vec![vec![handler, 1], vec![handler, 0]]
        );
        assert_eq!(calls(&e, FILE_CLOSE), vec![vec![file]]);
        let name = calls(&e, DATA_HANDLER_GET_LIST_FILE)[0][1];
        assert_eq!(printed(&e), vec![vec![MSG_FILE_CLOSED, name]]);
        // Without echo nothing is printed but the file is still closed.
        set_echo(&mut e, false);
        start_log(&mut e);
        assert!(run(&mut e, 0x005b_fd50, script(0)));
        assert_eq!(calls(&e, FILE_CLOSE).len(), 1);
        assert!(printed(&e).is_empty());
    }

    #[test]
    fn script_force_close_file_reports_missing_files_and_names() {
        let mut e = engine_b();
        e.set_global(DATA_HANDLER, 0x0a00_0100u32);
        e.register(DATA_HANDLER_GET_LIST_FILE, |_, _| 0u32.into_ret());
        accept(&mut e, &[DATA_HANDLER_SET_BYTE, FILE_CLOSE]);
        set_echo(&mut e, true);
        parse_with(&mut e, true, |e, a| e.mem.set_cstr(a[7], b"gone.esp"));
        start_log(&mut e);
        assert!(run(&mut e, 0x005b_fd50, script(0)));
        let name = calls(&e, DATA_HANDLER_GET_LIST_FILE)[0][1];
        assert_eq!(printed(&e), vec![vec![MSG_FILE_NOT_FOUND, name]]);
        assert!(calls(&e, FILE_CLOSE).is_empty());
        // An empty name never reaches the data handler.
        parse_with(&mut e, true, |_, _| {});
        start_log(&mut e);
        assert!(run(&mut e, 0x005b_fd50, script(0)));
        assert_eq!(printed(&e), vec![vec![MSG_NO_FILENAME]]);
        assert!(calls(&e, DATA_HANDLER_GET_LIST_FILE).is_empty());
        // Failed parse.
        parse_with(&mut e, false, |_, _| {});
        assert!(!run(&mut e, 0x005b_fd50, script(0)));
    }

    // ---- 005bfe90 -----------------------------------------------------------------

    /// What `004a3fe0` was handed: the world, the request's words
    /// (`0x74` bytes at the time of the call) and the two flags.
    type Submitted = (u32, Vec<u8>, u32, u32);

    /// Doubles for `005bfe90`. Returns the camera, the player and the list
    /// of submitted requests. The camera sits at (1, 2, 3) and looks along
    /// (4, 5, 6); the request constructor leaves bit 0x10000 in the mask.
    fn decal_engine(e: &mut Engine) -> (u32, u32, Rc<RefCell<Vec<Submitted>>>) {
        let camera_holder = e.mem.alloc(0xc0);
        let camera = e.mem.alloc(0x100);
        e.mem.set_u32(camera_holder + 0xac, camera);
        for (i, value) in [1.0f32, 2.0, 3.0].into_iter().enumerate() {
            e.mem.set_f32(camera + 0x8c + 4 * i as u32, value);
        }
        e.register_double(CAMERA_HOLDER, move |_, _| camera_holder.into_ret());
        e.register(WORD_AT_0XAC, |e, a| e.mem.u32(a[0] + 0xac).into_ret());
        e.register(CAMERA_POSITION, |_, a| (a[0] + 0x8c).into_ret());
        e.register(DECAL_REQUEST_CONSTRUCT, |e, a| {
            e.mem.set_u32(a[0] + 0x6c, 0x1_0000);
            Ret::default()
        });
        e.register(CAMERA_DIRECTION, |e, a| {
            for (i, value) in [4.0f32, 5.0, 6.0].into_iter().enumerate() {
                e.mem.set_f32(a[1] + 4 * i as u32, value);
            }
            a[1].into_ret()
        });
        e.register(VECTOR_NEGATE, |e, a| {
            for i in 0..3 {
                let value = e.mem.f32(a[0] + 4 * i);
                e.mem.set_f32(a[1] + 4 * i, -value);
            }
            a[1].into_ret()
        });
        let submitted = Rc::new(RefCell::new(vec![]));
        let log = submitted.clone();
        e.register_double(DECAL_REQUEST_SUBMIT, move |e, a| {
            let request = e.mem.bytes(a[1], 0x74);
            log.borrow_mut().push((a[0], request, a[2], a[3]));
            Ret::default()
        });
        let player = e.mem.alloc(0x80);
        e.set_global(PLAYER, player);
        let tes = e.mem.alloc(0x80);
        e.set_global(TES_SINGLETON, tes);
        e.register(WORD_AT_0X34, |e, a| e.mem.u32(a[0] + 0x34).into_ret());
        e.register(WORD_AT_0X40, |e, a| e.mem.u32(a[0] + 0x40).into_ret());
        // Floats: decal float 7.0, ranges 1.0 to 5.0, random = their mean
        // (or the range of the call), rounding keeps the value.
        e.register(DECAL_FLOAT, |_, _| 7.0f32.into_ret());
        e.register(RANGE_LOW, |_, _| 1.0f32.into_ret());
        e.register(RANGE_HIGH, |_, _| 5.0f32.into_ret());
        e.register(RANDOM_FLOAT, |_, a| {
            ((f32::from_bits(a[0]) + f32::from_bits(a[1])) / 2.0).into_ret()
        });
        e.register(ROUND_FLOAT, |_, a| f32::from_bits(a[0]).into_ret());
        e.register(WORD_AT_0XBC, |e, a| e.mem.u32(a[0] + 0xbc).into_ret());
        e.register(SECOND_GENERIC_LOCATION, |_, a| (a[0] + 0x1000).into_ret());
        (camera, player, submitted)
    }

    fn word(request: &[u8], offset: usize) -> u32 {
        u32::from_le_bytes(request[offset..offset + 4].try_into().unwrap())
    }

    #[test]
    fn body_005bfe90_without_a_reference_only_looks_up_the_world() {
        let mut e = engine_b();
        let (_camera, player, submitted) = decal_engine(&mut e);
        accept(&mut e, &[GET_CURRENT_CELL]);
        let tes = e.global::<u32>(TES_SINGLETON);
        start_log(&mut e);
        // Without the TES word at +0x34 the player's +0x40 word is asked.
        assert!(run(&mut e, 0x005b_fe90, script(0)));
        assert_eq!(calls(&e, WORD_AT_0X40), vec![vec![player]]);
        assert!(calls(&e, GET_CURRENT_CELL).is_empty());
        // With it the current cell of the TES singleton.
        e.mem.set_u32(tes + 0x34, 1);
        start_log(&mut e);
        assert!(run(&mut e, 0x005b_fe90, script(0)));
        assert_eq!(calls(&e, GET_CURRENT_CELL), vec![vec![tes]]);
        assert!(calls(&e, WORD_AT_0X40).is_empty());
        assert!(submitted.borrow().is_empty());
    }

    #[test]
    fn body_005bfe90_submits_a_negated_direction_for_a_plain_reference() {
        let mut e = engine_b();
        let (camera, player, submitted) = decal_engine(&mut e);
        let linked = e.mem.alloc(0x100);
        e.register_double(V_FLOAT, move |_, _| linked.into_ret());
        let reference = object_with(&mut e, &[(0x100, V_FALSE), (0x1d0, V_FLOAT)]);
        e.mem.set_u32(reference + 0x40, 0x0a00_5555);
        e.register(ACTOR_GET_CURRENT_WEAPON, |_, a| {
            assert_ne!(a[0], 0);
            0x0a00_6000u32.into_ret()
        });
        e.register(WEAPON_LOOKUP, |_, a| {
            assert_eq!(a, [0x0a00_6000, 0]);
            0x0a00_7000u32.into_ret()
        });
        assert!(run(&mut e, 0x005b_fe90, script(reference)));
        let submitted = submitted.borrow();
        assert_eq!(submitted.len(), 1);
        let (world, request, mode, flag) = &submitted[0];
        assert_eq!((*world, *mode, *flag), (0x0a00_5555, 1, 1));
        // Position of the camera, negated direction, the location found
        // through the weapon, -1, the random numbers and constants.
        let floats: Vec<f32> = [0, 4, 8, 0xc, 0x10, 0x14]
            .iter()
            .map(|offset| f32::from_bits(word(request, *offset)))
            .collect();
        assert_eq!(floats, vec![1.0, 2.0, 3.0, -4.0, -5.0, -6.0]);
        assert_eq!(word(request, 0x30), 0x0a00_7000 + 0x1000);
        assert_eq!(word(request, 0x34), 0xffff_ffff);
        assert_eq!(word(request, 0x28), linked);
        assert_eq!(f32::from_bits(word(request, 0x44)), 7.0);
        assert_eq!(f32::from_bits(word(request, 0x38)), 3.0);
        assert_eq!(f32::from_bits(word(request, 0x3c)), 3.0);
        assert_eq!(f32::from_bits(word(request, 0x40)), 1000.0);
        assert_eq!(f32::from_bits(word(request, 0x58)), 90.0);
        // The rounded random number (0.0 to 4.0 -> 2.0) is the byte at 0x70;
        // no actor bits in the mask.
        assert_eq!(request[0x70], 2);
        assert_eq!(word(request, 0x6c), 0x1_0000);
        let _ = (camera, player);
    }

    #[test]
    fn body_005bfe90_an_actor_gets_the_flag_bits_and_the_forward_direction() {
        let mut e = engine_b();
        let (_camera, _player, submitted) = decal_engine(&mut e);
        let linked = e.mem.alloc(0x100);
        e.register_double(V_FLOAT, move |_, _| linked.into_ret());
        let actor = actor_with(&mut e, &[(0x1d0, V_FLOAT)]);
        e.mem.set_u32(actor + 0x40, 0x0a00_5555);
        e.register(ACTOR_GET_CURRENT_WEAPON, |_, _| 0x0a00_6000u32.into_ret());
        // The actor answers false to 008ba410, so its part's slot 0x58
        // chooses the slot (here 9).
        e.register(ACTOR_FN_008BA410, |_, _| false.into_ret());
        let part_owner = e.mem.alloc(0x80);
        e.register(V_TRUE + 0x40, |_, _| 9u32.into_ret());
        let part = vtable(&mut e, &[(0x58, V_TRUE + 0x40)]);
        e.mem.set_u32(part_owner + 0x30, part);
        e.register_double(ACTOR_OWNER_PART, move |_, _| part_owner.into_ret());
        e.register(WEAPON_NODE, |_, a| {
            assert_eq!(a[0], 0x0a00_6000);
            0x0a00_6100u32.into_ret()
        });
        e.register(NODE_LOOKUP_BY_SLOT, |_, a| {
            assert_eq!(a, [0x0a00_6100, 9]);
            0x0a00_7000u32.into_ret()
        });
        assert!(run(&mut e, 0x005b_fe90, script(actor)));
        let submitted = submitted.borrow();
        let (_world, request, mode, flag) = &submitted[0];
        assert_eq!((*mode, *flag), (2, 1));
        // The camera's direction, the actor itself at +0x24, bits 1 to 15.
        let floats: Vec<f32> = [0xc, 0x10, 0x14]
            .iter()
            .map(|offset| f32::from_bits(word(request, *offset)))
            .collect();
        assert_eq!(floats, vec![4.0, 5.0, 6.0]);
        assert_eq!(word(request, 0x24), actor);
        assert_eq!(word(request, 0x6c), 0x1_0000 | 0xfffe);
        assert_eq!(word(request, 0x30), 0x0a00_7000 + 0x1000);
    }

    #[test]
    fn body_005bfe90_falls_back_to_the_object_of_index_6() {
        let mut e = engine_b();
        let (_camera, _player, submitted) = decal_engine(&mut e);
        let linked = e.mem.alloc(0x100);
        e.register_double(V_FLOAT, move |_, _| linked.into_ret());
        let reference = object_with(&mut e, &[(0x100, V_FALSE), (0x1d0, V_FLOAT)]);
        // The player has no weapon: no location, so `fn_005c02f0`'s object
        // and index 6 give the found object.
        e.register(ACTOR_GET_CURRENT_WEAPON, |_, _| 0u32.into_ret());
        e.set_global(GLOBAL_011CA830, 0x0a00_3030u32);
        e.register(EXPLOSION_LOOKUP, |_, a| {
            assert_eq!(a, [0x0a00_3030, 6]);
            0x0a00_8000u32.into_ret()
        });
        assert!(run(&mut e, 0x005b_fe90, script(reference)));
        let submitted = submitted.borrow();
        let (_world, request, mode, _flag) = &submitted[0];
        assert_eq!(*mode, 1);
        assert_eq!(word(request, 0x30), 0x0a00_8000 + 0x1000);
    }

    #[test]
    fn body_005bfe90_stops_without_a_slot_0x1d0_object() {
        let mut e = engine_b();
        let (_camera, _player, submitted) = decal_engine(&mut e);
        let reference = object_with(&mut e, &[(0x1d0, V_FALSE)]);
        assert!(run(&mut e, 0x005b_fe90, script(reference)));
        assert!(submitted.borrow().is_empty());
    }

    #[test]
    fn body_005c02f0_returns_the_global() {
        let mut e = engine_b();
        e.set_global(GLOBAL_011CA830, 0x1234u32);
        assert_eq!(e.call(0x005c_02f0, &args![]).u32(), 0x1234);
    }

    #[test]
    fn body_005c0300_stores_the_colour() {
        let mut e = engine_b();
        // The colour constructor stores its four floats and returns `this`.
        e.register(COLOR_CONSTRUCT, |e, a| {
            for i in 0..4 {
                e.mem.set_u32(a[0] + 4 * i, a[1 + i as usize]);
            }
            a[0].into_ret()
        });
        parse_gives(&mut e, true, &[255, 128, (-256i32) as u32]);
        assert!(run(&mut e, 0x005c_0300, script(0)));
        let colour: Vec<f32> = (0..4).map(|i| e.mem.f32(GLOBAL_COLOR + 4 * i)).collect();
        assert_eq!(colour, vec![255.0 / 256.0, 0.5, -1.0, 1.0]);
        // A failed parse leaves the colour alone.
        parse_gives(&mut e, false, &[]);
        e.mem.set_f32(GLOBAL_COLOR, 7.0);
        assert!(!run(&mut e, 0x005c_0300, script(0)));
        assert_eq!(e.mem.f32(GLOBAL_COLOR), 7.0);
    }

    // ---- Rotation, position and angle commands -----------------------------------------------

    /// Doubles of the rotation and orientation helpers. Returns the node
    /// (the reference's slot `0x1d0` object). The setters `X`/`Y`/`Z` store
    /// the angle in the rotation at `this + 0x24`, `+0x28`, `+0x2c`. The
    /// reference's slot `0x1e4` answers false.
    fn rotation_engine(e: &mut Engine) -> (u32, u32) {
        let node = e.mem.alloc(0x80);
        e.register_double(V_FLOAT + 20, move |_, _| node.into_ret());
        let reference = object_with(
            e,
            &[(0x1d0, V_FLOAT + 20), (0x1e4, V_FALSE), (0x100, V_FALSE)],
        );
        e.register(REFERENCE_ROTATION, |_, a| (a[0] + 0x24).into_ret());
        e.register(FRAME_DATA_FLOAT, |_, _| 1.0f32.into_ret());
        e.register(REFERENCE_SET_ANGLE_X, |e, a| {
            e.mem.set_u32(a[0] + 0x24, a[1]);
            Ret::default()
        });
        e.register(REFERENCE_SET_ANGLE_Y, |e, a| {
            e.mem.set_u32(a[0] + 0x28, a[1]);
            Ret::default()
        });
        e.register(REFERENCE_SET_ANGLE_Z, |e, a| {
            e.mem.set_u32(a[0] + 0x2c, a[1]);
            Ret::default()
        });
        e.register(REFERENCE_GET_ORIENTATION, |_, a| a[1].into_ret());
        accept(
            e,
            &[
                NODE_SET_ORIENTATION,
                UPDATE_DATA_CONSTRUCT,
                NODE_UPDATE,
                COLLISION_RESET_SIM,
            ],
        );
        (reference, node)
    }

    #[test]
    fn script_rotate_function_adds_and_wraps_the_angle() {
        let mut e = engine_b();
        let (reference, node) = rotation_engine(&mut e);
        // 90 degrees about Z from 0.
        parse_gives(&mut e, true, &[b'Z' as u32, 90]);
        start_log(&mut e);
        assert!(run(&mut e, 0x005c_03d0, script(reference)));
        let radians = 90.0 * f64::from_bits(0x3f91_df46_a000_0000);
        assert_eq!(e.mem.f32(reference + 0x2c), radians as f32);
        // The new orientation goes to the node; the node is updated.
        assert_eq!(calls(&e, NODE_SET_ORIENTATION).len(), 1);
        assert_eq!(calls(&e, NODE_SET_ORIENTATION)[0][0], node);
        assert_eq!(calls(&e, NODE_UPDATE).len(), 1);
        assert!(calls(&e, COLLISION_RESET_SIM).is_empty());
        // From 6.0, 90 degrees pass 2 pi and wrap.
        e.mem.set_f32(reference + 0x24, 6.0);
        parse_gives(&mut e, true, &[b'X' as u32, 90]);
        assert!(run(&mut e, 0x005c_03d0, script(reference)));
        let wrapped = ((6.0f32 as f64 + radians) as f32 as f64
            - f64::from_bits(0x4019_21fb_6000_0000)) as f32;
        assert_eq!(e.mem.f32(reference + 0x24), wrapped);
        // A negative angle is brought up into 0..2 pi.
        e.mem.set_f32(reference + 0x28, 0.0);
        parse_gives(&mut e, true, &[b'Y' as u32, (-90i32) as u32]);
        assert!(run(&mut e, 0x005c_03d0, script(reference)));
        let lifted = ((-radians) as f32 as f64 + f64::from_bits(0x4019_21fb_6000_0000)) as f32;
        assert_eq!(e.mem.f32(reference + 0x28), lifted);
    }

    #[test]
    fn script_rotate_function_scales_by_the_frame_float_and_skips_unknown_axes() {
        let mut e = engine_b();
        let (reference, _node) = rotation_engine(&mut e);
        e.register(FRAME_DATA_FLOAT, |_, a| {
            assert_eq!(a[0], FRAME_DATA);
            2.0f32.into_ret()
        });
        parse_gives(&mut e, true, &[b'Z' as u32, 10]);
        assert!(run(&mut e, 0x005c_03d0, script(reference)));
        let radians = 10.0 * f64::from_bits(0x3f91_df46_a000_0000);
        assert_eq!(e.mem.f32(reference + 0x2c), (2.0 * radians) as f32);
        // An unknown axis sets nothing but the orientation is still copied.
        start_log(&mut e);
        parse_gives(&mut e, true, &[b'W' as u32, 10]);
        assert!(run(&mut e, 0x005c_03d0, script(reference)));
        assert!(calls(&e, REFERENCE_SET_ANGLE_X).is_empty());
        assert!(calls(&e, REFERENCE_ROTATION).is_empty());
        assert_eq!(calls(&e, NODE_SET_ORIENTATION).len(), 1);
    }

    #[test]
    fn script_rotate_function_with_slot_0x1e4_true_does_not_update_the_node() {
        let mut e = engine_b();
        let (reference, node) = rotation_engine(&mut e);
        let table = e.mem.u32(reference);
        e.mem.set_u32(table + 0x1e4, V_TRUE);
        parse_gives(&mut e, true, &[b'Y' as u32, 1]);
        start_log(&mut e);
        assert!(run(&mut e, 0x005c_03d0, script(reference)));
        assert_eq!(calls(&e, NODE_SET_ORIENTATION)[0][0], node);
        assert!(calls(&e, NODE_UPDATE).is_empty());
        // No reference or no node: nothing; bad parameters: false.
        start_log(&mut e);
        assert!(run(&mut e, 0x005c_03d0, script(0)));
        let table = e.mem.u32(reference);
        e.mem.set_u32(table + 0x1d0, V_FALSE);
        assert!(run(&mut e, 0x005c_03d0, script(reference)));
        assert!(calls(&e, NODE_SET_ORIENTATION).is_empty());
        parse_with(&mut e, false, |_, _| {});
        assert!(!run(&mut e, 0x005c_03d0, script(reference)));
    }

    /// The condition function is called with the sign-extended axis.
    fn check_axis_command(address: u32, condition: u32) {
        let mut e = engine_b();
        e.register(condition, |_, a| (a[3] == 0x0a00_0abc).into_ret());
        let mut script = script(0x0a00_1000);
        script.result = Ptr::new(0x0a00_0abc);
        parse_gives(&mut e, true, &[b'Y' as u32]);
        start_log(&mut e);
        assert!(run(&mut e, address, script));
        assert_eq!(
            calls(&e, condition),
            vec![vec![0x0a00_1000, b'Y' as u32, 0, 0x0a00_0abc]]
        );
        // A negative character is sign-extended; the condition's answer is
        // the command's.
        e.register(condition, |_, _| false.into_ret());
        parse_gives(&mut e, true, &[0xf0]);
        start_log(&mut e);
        assert!(!run(&mut e, address, script));
        assert_eq!(calls(&e, condition)[0][1], 0xffff_fff0);
        // Failed parse: false without the condition.
        parse_gives(&mut e, false, &[]);
        start_log(&mut e);
        assert!(!run(&mut e, address, script));
        assert!(calls(&e, condition).is_empty());
    }

    #[test]
    fn script_get_pos_function_forwards_to_the_condition_function() {
        check_axis_command(0x005c_0590, GET_POS_CONDITION);
    }

    #[test]
    fn script_get_starting_pos_function_forwards_to_the_condition_function() {
        check_axis_command(0x005c_08a0, GET_STARTING_POS_CONDITION);
    }

    #[test]
    fn script_get_angle_function_forwards_to_the_condition_function() {
        check_axis_command(0x005c_0900, GET_ANGLE_CONDITION);
    }

    #[test]
    fn script_get_starting_angle_function_forwards_to_the_condition_function() {
        check_axis_command(0x005c_0960, GET_STARTING_ANGLE_CONDITION);
    }

    /// Doubles for the move commands: `reference` answers
    /// `REFERENCE_CAN_BE_MOVED` with `movable`, the setting `011c3ea4` holds
    /// `setting`. Slot `0x130` of every object answers its own address plus 1.
    fn move_engine(movable: bool, setting: u32) -> (Engine, u32, u32) {
        let mut e = engine_b();
        e.set_global(DATA_HANDLER, 0x0a00_0200u32);
        e.set_global(SETTING_011C3EA4 + 4, setting);
        e.register_double(REFERENCE_CAN_BE_MOVED, move |_, _| movable.into_ret());
        e.register(FORM_ID, |e, a| e.mem.u32(a[0] + 0xc).into_ret());
        e.register(V_FLOAT + 30, |_, a| (a[0] + 1).into_ret());
        let reference = object_with(&mut e, &[(0x130, V_FLOAT + 30)]);
        let script_object = object_with(&mut e, &[(0x130, V_FLOAT + 30)]);
        e.mem.set_u32(reference + 0xc, 0x0100_0001);
        e.mem.set_u32(script_object + 0xc, 0x0100_0002);
        accept(&mut e, &[SCRIPT_LOG, TASK_QUEUE_INTERFACE, QUEUE_TASK]);
        (e, reference, script_object)
    }

    fn move_script(reference: u32, script_object: u32) -> ScriptArgs {
        let mut a = script(reference);
        a.script_obj = Ptr::new(script_object);
        a
    }

    #[test]
    fn script_set_pos_function_queues_the_task_when_the_setting_is_above_one() {
        let (mut e, reference, script_object) = move_engine(true, 2);
        e.register(TASK_QUEUE_INTERFACE, |_, _| 0x0a00_0300u32.into_ret());
        parse_gives(&mut e, true, &[b'Z' as u32, 1.5f32.to_bits()]);
        start_log(&mut e);
        assert!(run(
            &mut e,
            0x005c_05f0,
            move_script(reference, script_object)
        ));
        assert_eq!(
            calls(&e, QUEUE_TASK),
            vec![args![
                0x0a00_0300u32,
                0x1007u32,
                reference,
                b'Z' as u32,
                1.5f64
            ]]
        );
        // The same for the angle command with the task 0x1009.
        start_log(&mut e);
        assert!(run(
            &mut e,
            0x005c_09c0,
            move_script(reference, script_object)
        ));
        assert_eq!(calls(&e, QUEUE_TASK)[0][1], 0x1009);
        // Failed parse.
        parse_gives(&mut e, false, &[]);
        assert!(!run(
            &mut e,
            0x005c_05f0,
            move_script(reference, script_object)
        ));
    }

    #[test]
    fn script_set_pos_function_runs_the_base_when_the_setting_is_one() {
        let (mut e, reference, script_object) = move_engine(true, 1);
        // The base reads the position through slot 0x1f4 and stores it.
        let position = e.mem.alloc(12);
        e.register_double(V_FLOAT + 31, move |_, _| position.into_ret());
        let table = e.mem.u32(reference);
        e.mem.set_u32(table + 0x1f4, V_FLOAT + 31);
        e.mem.set_u32(table + 0x1d0, V_FALSE);
        accept(&mut e, &[REFERENCE_SET_LOCATION]);
        e.register(DYNAMIC_CAST, |_, _| 0u32.into_ret());
        parse_gives(&mut e, true, &[b'Y' as u32, 4.0f32.to_bits()]);
        start_log(&mut e);
        assert!(run(
            &mut e,
            0x005c_05f0,
            move_script(reference, script_object)
        ));
        assert!(calls(&e, QUEUE_TASK).is_empty());
        let vector = calls(&e, REFERENCE_SET_LOCATION)[0][1];
        assert_eq!(calls(&e, REFERENCE_SET_LOCATION)[0][0], reference);
        assert_eq!(e.mem.f32(vector + 4), 4.0);
    }

    #[test]
    fn script_set_pos_function_reports_a_reference_that_cannot_be_moved() {
        let (mut e, reference, script_object) = move_engine(false, 2);
        set_echo(&mut e, true);
        parse_gives(&mut e, true, &[b'X' as u32, 1.0f32.to_bits()]);
        // A static (non-runtime) form id for the script: the log line.
        e.register(IS_DYNAMIC_FORM_ID, |_, a| (a[1] >= 0xff00_0000).into_ret());
        start_log(&mut e);
        assert!(run(
            &mut e,
            0x005c_05f0,
            move_script(reference, script_object)
        ));
        assert_eq!(printed(&e), vec![vec![MSG_CANNOT_BE_MOVED, reference + 1]]);
        assert_eq!(
            calls(&e, SCRIPT_LOG),
            vec![vec![
                MSG_SCRIPT_MOVE_LOG,
                script_object + 1,
                0x0100_0002,
                reference + 1,
                0x0100_0001
            ]]
        );
        assert!(calls(&e, QUEUE_TASK).is_empty());
        // A dynamic form id for the script: no log line; without echo no
        // message either.
        e.mem.set_u32(script_object + 0xc, 0xff00_0002);
        set_echo(&mut e, false);
        start_log(&mut e);
        assert!(run(
            &mut e,
            0x005c_09c0,
            move_script(reference, script_object)
        ));
        assert!(printed(&e).is_empty());
        assert!(calls(&e, SCRIPT_LOG).is_empty());
    }

    /// A reference for `005c0740`/`005c0b10`: slot 0x1f4 answers a
    /// position (1, 2, 3), slot 0x1d0 the node, slot 0x1e4 false.
    fn base_engine(e: &mut Engine) -> (u32, u32) {
        let (reference, node) = rotation_engine(e);
        let position = e.mem.alloc(12);
        for (i, value) in [1.0f32, 2.0, 3.0].into_iter().enumerate() {
            e.mem.set_f32(position + 4 * i as u32, value);
        }
        e.register_double(V_FLOAT + 31, move |_, _| position.into_ret());
        let table = e.mem.u32(reference);
        e.mem.set_u32(table + 0x1f4, V_FLOAT + 31);
        accept(e, &[REFERENCE_SET_LOCATION, NODE_SET_POSITION]);
        e.register(DYNAMIC_CAST, |_, _| 0u32.into_ret());
        (reference, node)
    }

    fn vector_of(e: &Engine, address: u32) -> [f32; 3] {
        [
            e.mem.f32(address),
            e.mem.f32(address + 4),
            e.mem.f32(address + 8),
        ]
    }

    #[test]
    fn script_set_pos_base_changes_one_coordinate() {
        let mut e = engine_b();
        let (reference, node) = base_engine(&mut e);
        // The vector is copied before being passed: capture it at the call.
        let seen = Rc::new(RefCell::new(vec![]));
        let log = seen.clone();
        e.register_double(REFERENCE_SET_LOCATION, move |e, a| {
            log.borrow_mut().push(vector_of(e, a[1]));
            Ret::default()
        });
        let log = seen.clone();
        e.register_double(NODE_SET_POSITION, move |e, a| {
            assert_eq!(a[0], node);
            log.borrow_mut().push(vector_of(e, a[1]));
            Ret::default()
        });
        for (axis, expected) in [
            (b'X', [9.0, 2.0, 3.0]),
            (b'Y', [1.0, 9.0, 3.0]),
            (b'Z', [1.0, 2.0, 9.0]),
            (b'W', [1.0, 2.0, 3.0]),
        ] {
            seen.borrow_mut().clear();
            start_log(&mut e);
            e.call(0x005c_0740, &args![reference, axis as u32, 9.0f32]);
            assert_eq!(*seen.borrow(), vec![expected, expected]);
            // The node's collision is reset and the node updated.
            assert_eq!(calls(&e, COLLISION_RESET_SIM), vec![vec![node, 1]]);
            assert_eq!(calls(&e, NODE_UPDATE).len(), 1);
        }
        // A null reference does nothing.
        start_log(&mut e);
        e.call(0x005c_0740, &args![0u32, b'X' as u32, 1.0f32]);
        assert!(e.call_log.as_ref().unwrap().len() == 1);
    }

    #[test]
    fn script_set_pos_base_moves_the_character_controller_of_a_mobile_object() {
        let mut e = engine_b();
        let (reference, _node) = base_engine(&mut e);
        // The RTTI cast succeeds (the reference is a mobile object) and it
        // has a controller.
        e.register(DYNAMIC_CAST, |_, a| a[0].into_ret());
        let controller = e.mem.alloc(0x400);
        e.register_double(GET_CHAR_CONTROLLER, move |_, a| {
            assert_ne!(a[0], 0);
            controller.into_ret()
        });
        accept(&mut e, &[CONTROLLER_SET_POSITION]);
        // `00ca9600` answers 4 for the part at +0x3e0: the position is not
        // set; any other answer sets it.
        e.register_double(CONTROLLER_PART_VALUE, move |_, a| {
            assert_eq!(a[0], controller + 0x3e0);
            4u32.into_ret()
        });
        start_log(&mut e);
        e.call(0x005c_0740, &args![reference, b'X' as u32, 5.0f32]);
        assert!(calls(&e, CONTROLLER_SET_POSITION).is_empty());
        e.register(CONTROLLER_PART_VALUE, |_, _| 3u32.into_ret());
        start_log(&mut e);
        e.call(0x005c_0740, &args![reference, b'X' as u32, 5.0f32]);
        assert_eq!(calls(&e, CONTROLLER_SET_POSITION)[0][0], controller);
        // No controller: nothing.
        e.register(GET_CHAR_CONTROLLER, |_, _| 0u32.into_ret());
        start_log(&mut e);
        e.call(0x005c_0740, &args![reference, b'X' as u32, 5.0f32]);
        assert!(calls(&e, CONTROLLER_SET_POSITION).is_empty());
    }

    #[test]
    fn body_005c0860_and_005c0880_ask_the_controller_part() {
        let mut e = engine_b();
        let controller = e.mem.alloc(0x400);
        e.register_double(CONTROLLER_PART_VALUE, move |_, a| {
            assert_eq!(a[0], controller + 0x3e0);
            4u32.into_ret()
        });
        assert_eq!(e.call(0x005c_0880, &args![controller]).u32(), 4);
        assert!(e.call(0x005c_0860, &args![controller]).bool());
        e.register(CONTROLLER_PART_VALUE, |_, _| 2u32.into_ret());
        assert_eq!(e.call(0x005c_0880, &args![controller]).u32(), 2);
        assert!(!e.call(0x005c_0860, &args![controller]).bool());
    }

    #[test]
    fn script_set_angle_base_converts_degrees_and_updates_the_node() {
        let mut e = engine_b();
        let (reference, node) = rotation_engine(&mut e);
        let radians = (90.0f32 as f64 * f64::from_bits(0x3f91_df46_a000_0000)) as f32;
        for (axis, offset) in [(b'X', 0x24), (b'Y', 0x28), (b'Z', 0x2c)] {
            start_log(&mut e);
            e.call(0x005c_0b10, &args![reference, axis as u32, 90.0f32]);
            assert_eq!(e.mem.f32(reference + offset), radians);
            assert_eq!(calls(&e, NODE_SET_ORIENTATION)[0][0], node);
            assert_eq!(calls(&e, COLLISION_RESET_SIM), vec![vec![node, 1]]);
            assert_eq!(calls(&e, NODE_UPDATE).len(), 1);
        }
        // Slot 0x1e4 true: no update.
        let table = e.mem.u32(reference);
        e.mem.set_u32(table + 0x1e4, V_TRUE);
        start_log(&mut e);
        e.call(0x005c_0b10, &args![reference, b'X' as u32, 1.0f32]);
        assert!(calls(&e, NODE_UPDATE).is_empty());
        assert_eq!(calls(&e, COLLISION_RESET_SIM).len(), 1);
        // Null reference and missing node: nothing.
        start_log(&mut e);
        e.call(0x005c_0b10, &args![0u32, b'X' as u32, 1.0f32]);
        e.mem.set_u32(table + 0x1d0, V_FALSE);
        e.call(0x005c_0b10, &args![reference, b'X' as u32, 1.0f32]);
        assert!(calls(&e, NODE_SET_ORIENTATION).is_empty());
        // An unknown axis sets no angle.
        start_log(&mut e);
        e.call(0x005c_0b10, &args![reference, b'W' as u32, 1.0f32]);
        assert!(calls(&e, REFERENCE_SET_ANGLE_X).is_empty());
    }

    #[test]
    fn script_set_angle_function_runs_the_base_when_the_setting_is_one() {
        let (mut e, reference, script_object) = move_engine(true, 1);
        let node = e.mem.alloc(0x80);
        e.register_double(V_FLOAT + 20, move |_, _| node.into_ret());
        let table = e.mem.u32(reference);
        e.mem.set_u32(table + 0x1d0, V_FLOAT + 20);
        e.mem.set_u32(table + 0x1e4, V_TRUE);
        e.register(REFERENCE_SET_ANGLE_Z, |e, a| {
            e.mem.set_u32(a[0] + 0x2c, a[1]);
            Ret::default()
        });
        e.register(REFERENCE_GET_ORIENTATION, |_, a| a[1].into_ret());
        accept(&mut e, &[NODE_SET_ORIENTATION, COLLISION_RESET_SIM]);
        parse_gives(&mut e, true, &[b'Z' as u32, 180.0f32.to_bits()]);
        start_log(&mut e);
        assert!(run(
            &mut e,
            0x005c_09c0,
            move_script(reference, script_object)
        ));
        assert!(calls(&e, QUEUE_TASK).is_empty());
        let radians = (180.0f64 * f64::from_bits(0x3f91_df46_a000_0000)) as f32;
        assert_eq!(e.mem.f32(reference + 0x2c), radians);
    }

    #[test]
    fn script_set_at_start_function_builds_the_orientation_from_three_rotations() {
        let mut e = engine_b();
        // Identity matrix, zero vector.
        for i in 0..9 {
            e.mem
                .set_f32(IDENTITY_MATRIX + 4 * i, if i % 4 == 0 { 1.0 } else { 0.0 });
        }
        let node = e.mem.alloc(0x80);
        e.register_double(V_FLOAT + 20, move |_, _| node.into_ret());
        // Slot 0x16c answers the angles (1, 2, 3) through its out pointer;
        // slot 0x170 the position (7, 8, 9).
        e.register(V_FLOAT + 21, |e, a| {
            for (i, value) in [1.0f32, 2.0, 3.0].into_iter().enumerate() {
                e.mem.set_f32(a[1] + 4 * i as u32, value);
            }
            a[1].into_ret()
        });
        e.register(V_FLOAT + 22, |e, a| {
            for (i, value) in [7.0f32, 8.0, 9.0].into_iter().enumerate() {
                e.mem.set_f32(a[1] + 4 * i as u32, value);
            }
            a[1].into_ret()
        });
        let reference = object_with(
            &mut e,
            &[
                (0x1d0, V_FLOAT + 20),
                (0x1e4, V_FALSE),
                (0x16c, V_FLOAT + 21),
                (0x170, V_FLOAT + 22),
                (0x174, V_RECORD),
            ],
        );
        e.mem.set_u32(ZERO_VECTOR, 1.0f32.to_bits());
        // The matrix doubles: a rotation matrix is filled with the angle in
        // its first word; the product records the order and stores the sum
        // of the first words in the output.
        accept(
            &mut e,
            &[EMPTY_CONSTRUCT, NODE_UPDATE, UPDATE_DATA_CONSTRUCT],
        );
        for (address, tag) in [
            (MATRIX_MAKE_X_ROTATION, 1.0f32),
            (MATRIX_MAKE_Y_ROTATION, 2.0),
            (MATRIX_MAKE_Z_ROTATION, 3.0),
        ] {
            e.register_double(address, move |e, a| {
                let angle = f32::from_bits(a[1]);
                e.mem.set_f32(a[0], tag * 100.0 + angle);
                Ret::default()
            });
        }
        e.register(MATRIX_MULTIPLY, |e, a| {
            // out = rotation word + 1000 * running word
            let rotation = e.mem.f32(a[0]);
            let running = e.mem.f32(a[2]);
            e.mem.set_f32(a[1], rotation + 1000.0 * running);
            copy_words(e, a[2] + 4, a[1] + 4, 8);
            a[1].into_ret()
        });
        accept(&mut e, &[NODE_SET_POSITION]);
        let orientation = Rc::new(RefCell::new(0.0f32));
        let seen = orientation.clone();
        e.register_double(NODE_SET_ORIENTATION, move |e, a| {
            assert_eq!(a[0], node);
            *seen.borrow_mut() = e.mem.f32(a[1]);
            Ret::default()
        });
        start_log(&mut e);
        assert!(run(&mut e, 0x005c_0bf0, script(reference)));
        // The running matrix starts as the identity (first word 1.0); each
        // step: new = rotation + 1000 * running.
        let first = 101.0 + 1000.0 * 1.0;
        let second = 202.0 + 1000.0 * first;
        let third = 303.0 + 1000.0 * second;
        assert_eq!(*orientation.borrow(), third);
        // Slot 0x174 got the three words of the zero vector.
        assert_eq!(
            calls(&e, V_RECORD),
            vec![vec![reference, 1.0f32.to_bits(), 0, 0]]
        );
        assert_eq!(calls(&e, NODE_SET_POSITION)[0][0], node);
        assert_eq!(calls(&e, NODE_UPDATE).len(), 1);
        // Slot 0x1e4 true: no update; a reference without a node: only true.
        let table = e.mem.u32(reference);
        e.mem.set_u32(table + 0x1e4, V_TRUE);
        start_log(&mut e);
        assert!(run(&mut e, 0x005c_0bf0, script(reference)));
        assert!(calls(&e, NODE_UPDATE).is_empty());
        e.mem.set_u32(table + 0x1d0, V_FALSE);
        start_log(&mut e);
        assert!(run(&mut e, 0x005c_0bf0, script(reference)));
        assert!(calls(&e, NODE_SET_POSITION).is_empty());
        assert!(run(&mut e, 0x005c_0bf0, script(0)));
    }
}
