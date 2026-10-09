//! `fallout shared/tesscriptfunctions.cpp` (Xbox PDB source unit), part 2: its functions from `005bc430` up to
//! (not including) `005c4240` in FalloutNV.exe 1.4.0.525, translated
//! (docs/ENGINE_CRATE.md). The unit's shared layouts and helpers are in
//! [`super::tesscriptfunctions`]; anything public there may be used here.
//!
//! Progress: the first 40 queue entries of this part (`005bc430` to
//! `005be870`) are translated. The next session continues at `005be900`
//! (the next entry of the queue after `005be870`).
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
}
