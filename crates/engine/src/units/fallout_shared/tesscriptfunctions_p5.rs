//! `fallout shared/tesscriptfunctions.cpp` (Xbox PDB source unit), part 5: its functions from `005cd990` up to
//! (not including) `005d21e0` in FalloutNV.exe 1.4.0.525, translated
//! (docs/ENGINE_CRATE.md). The unit's shared layouts and helpers are in
//! [`super::tesscriptfunctions`]; anything public there may be used here.
//!
//! Progress: the first 40 queue entries of the range (`005cd990` to
//! `005ceb70`) are translated. The next session continues at `005ceb90`
//! (`Script::GetDefaultOpenFunction`).
//!
//! The bodies follow the conventions of the main file: `cdecl`, the eight
//! stack words as [`ScriptArgs`], `AL` as the result. The members of
//! `PlayerCharacter`, `Actor` and the factions are read at the PC offsets
//! (the PC build differs from the Xbox PDB's layout), with a comment, instead
//! of through a `layout!`.

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
/// RTTI type descriptor of `TESObjectREFR` (`.?AVTESObjectREFR@@`).
const RTTI_TES_OBJECT_REFR: u32 = 0x0118_41cc;
/// RTTI type descriptor of `Actor` (`.?AVActor@@`).
const RTTI_ACTOR: u32 = 0x0118_46d4;

/// `Script::GetDetectionLevelConditionFunction` (Xbox PDB), `cdecl`
/// (`thisObj, actor, 0, result`); the other condition functions below take
/// the same four words.
const GET_DETECTION_LEVEL_CONDITION: u32 = 0x005a_1ee0;
/// `Script::IsSwimmingConditionFunction` (Xbox PDB).
const IS_SWIMMING_CONDITION: u32 = 0x005a_1fa0;
/// `Script::GetAmountStolenSoldConditionFunction` (Xbox PDB).
const GET_AMOUNT_STOLEN_SOLD_CONDITION: u32 = 0x005a_2050;
/// `Script::GetPCExpelledConditionFunction` (Xbox PDB).
const GET_PC_EXPELLED_CONDITION: u32 = 0x005a_20d0;
/// `Script::GetPCFactionMurderConditionFunction` (Xbox PDB).
const GET_PC_FACTION_MURDER_CONDITION: u32 = 0x005a_2150;
/// `Script::GetPlayerEnemyofFactionConditionFunction` (Xbox PDB).
const GET_PLAYER_ENEMY_OF_FACTION_CONDITION: u32 = 0x005a_21f0;
/// `Script::GetPCFactionAttackConditionFunction` (Xbox PDB).
const GET_PC_FACTION_ATTACK_CONDITION: u32 = 0x005a_2290;
/// `Script::GetDestroyedConditionFunction` (Xbox PDB).
const GET_DESTROYED_CONDITION: u32 = 0x005a_2330;
/// `Script::HasMagicEffectConditionFunction` (Xbox PDB).
const HAS_MAGIC_EFFECT_CONDITION: u32 = 0x005a_2390;
/// `Script::IsSpellTargetConditionFunction` (Xbox PDB).
const IS_SPELL_TARGET_CONDITION: u32 = 0x005a_2440;
/// `Script::GetSpellUsageNumberConditionFunction` (Xbox PDB).
const GET_SPELL_USAGE_NUMBER_CONDITION: u32 = 0x005a_24f0;
/// `Script::GetVATSModeConditionFunction` (Xbox PDB).
const GET_VATS_MODE_CONDITION: u32 = 0x005a_2590;
/// `Script::GetVATSTargetHeightConditionFunction` (Xbox PDB).
const GET_VATS_TARGET_HEIGHT_CONDITION: u32 = 0x005a_25f0;

/// `Script::PutNumericIDInDouble` (Xbox PDB), `cdecl` (`address of the id,
/// double* result`): stores the 4-byte id into the double.
const PUT_NUMERIC_ID_IN_DOUBLE: u32 = 0x005a_cc70;
/// `thiscall`: `*(this + 0x0c)`, the form id of a form.
const GET_FORM_ID: u32 = 0x0084_e3a0;
/// `thiscall` on a reference: the name of the reference (the full name of
/// its base form).
const GET_REFERENCE_NAME: u32 = 0x0055_d520;
/// `TESObjectREFR::GetActionRef` (Xbox PDB).
const GET_ACTION_REF: u32 = 0x0057_2e30;
/// `TESObjectREFR::GetRefPersists` (Xbox PDB).
const GET_REF_PERSISTS: u32 = 0x0056_53d0;
/// `*(this + 0x20)`: the base form of a reference (the map calls it
/// `BGSSaveFormBuffer::GetForm`).
const GET_BASE_FORM: u32 = 0x007a_f430;
/// Form type byte (`this + 4`).
const FORM_TYPE: u32 = 0x0040_1170;
/// `TESContainer::ContainerCanHoldType` (Xbox PDB), `cdecl` (`form type`).
const CONTAINER_CAN_HOLD_TYPE: u32 = 0x0048_1f30;
/// `thiscall` on a reference: a reference read from its extra data list
/// through `0041da10` (what `GetParentRef` returns).
const GET_PARENT_REF: u32 = 0x0056_a9f0;
/// `thiscall` on a reference: a reference read from its extra data list
/// through `0041e410` (what `GetLinkedRef` returns).
const GET_LINKED_REF: u32 = 0x0056_9b80;
/// `this + 0x44`: the extra data list of a reference.
const EXTRA_DATA_LIST: u32 = 0x005d_43c0;
/// `ExtraDataList::GetPackageExtra` (Xbox PDB): the package of the list.
const GET_PACKAGE_EXTRA: u32 = 0x0041_cb10;
/// `Actor::GetPackageSetAsPcurrent` (Xbox PDB).
const GET_PACKAGE_SET_AS_CURRENT: u32 = 0x0088_1510;
/// `Actor::GetCurrentPackageTarget` (Xbox PDB).
const GET_CURRENT_PACKAGE_TARGET: u32 = 0x0088_1650;
/// Package type: the sign-extended byte at `package + 0x20`.
const GET_PACKAGE_TYPE: u32 = 0x0041_ca90;
/// `thiscall` on a package (`flag`): sets or clears bit `0x10000` of its
/// flags word (`this + 0x1c`) and notifies the data handler.
const SET_PACKAGE_FLAG: u32 = 0x0067_4f30;
/// `MiddleHighProcess::GetSavedAcquireObject` (Xbox PDB): `*(this + 0x68)`,
/// the actor's process.
const GET_PROCESS: u32 = 0x008d_8520;
/// `PlayerCharacter` method (`00962720`, `thiscall`, `actor`): whether the
/// actor is among the player's teammates.
const PLAYER_HAS_TEAMMATE: u32 = 0x0096_2720;
/// `PlayerCharacter` method (`00962620`, `thiscall`): the number of
/// teammates.
const PLAYER_TEAMMATE_COUNT: u32 = 0x0096_2620;
/// `thiscall` on a game setting (`011cdad0`): the address of its integer
/// value.
const GET_SETTING_INTEGER: u32 = 0x0043_d4d0;
/// `*(this + 4)` of a string global: its text.
const BS_STRING_TEXT: u32 = 0x0040_3df0;
/// Interface message with icon (`cdecl`: `text, 0, 0, 0, float, 0`).
const SHOW_MESSAGE: u32 = 0x0070_52f0;
/// `ProcessLists` method (`00973460`, `thiscall` on the process lists
/// singleton, `actor`): a byte, non-zero when the actor detects the player.
const PROCESS_LISTS_IS_ACTOR_DETECTED: u32 = 0x0097_3460;
/// `thiscall` on a faction (`mask, flag`): sets or clears the bits of the
/// mask in the faction's flags word (`this + 0x34`) and notifies it.
const FACTION_SET_FLAG_BITS: u32 = 0x005f_c970;
/// The faction flag `005ce040` sets or clears.
const FACTION_FLAG_EXPELLED: u32 = 0x08;
/// `thiscall` on the player: the player's parent cell, `*(this + 0x40)`.
const GET_PARENT_CELL: u32 = 0x008d_6f30;
/// `TESObjectCELL::GetOwner` (Xbox PDB).
const CELL_GET_OWNER: u32 = 0x0054_6a40;
/// `thiscall` on a faction (`flag`): sets or clears the faction flag `0x40`
/// (it calls `005fc970` with that mask).
const FACTION_SET_MURDER_FLAG: u32 = 0x0047_ebb0;
/// Same shape as [`FACTION_SET_MURDER_FLAG`] (`0047eb90`).
const FACTION_SET_ENEMY_FLAG: u32 = 0x0047_eb90;
/// Same shape as [`FACTION_SET_MURDER_FLAG`] (`0047ebd0`).
const FACTION_SET_ATTACK_FLAG: u32 = 0x0047_ebd0;
/// `thiscall` on a form (`flag`): sets or clears form flag `0x800000`
/// (`this + 8`) and notifies the form.
const FORM_SET_FLAG_800000: u32 = 0x0048_4650;
/// `thiscall` on the player (`value, flag`): stores a `dword` at `+0x654` and
/// a byte at `+0x658` (`005c1a00`, another part of the unit).
const PLAYER_SET_FIELD_654: u32 = 0x005c_1a00;
/// `thiscall` on an actor (`flag`): stores the force-run byte at `+0x124`
/// (`005bf800`, another part of the unit).
const ACTOR_SET_FORCE_RUN: u32 = 0x005b_f800;
/// `thiscall` on an actor: the byte at `+0x124` (the force-run flag).
const ACTOR_GET_FORCE_RUN: u32 = 0x008d_8220;
/// `thiscall` on an actor: the byte at `+0x125` (the force-sneak flag).
const ACTOR_GET_FORCE_SNEAK: u32 = 0x005c_e8f0;
/// `thiscall` on the player: the player's level (`u16`).
const PLAYER_GET_LEVEL: u32 = 0x0087_f9f0;
/// `thiscall` on a reference: its base form, `*(this + 0x20)`.
const GET_BASE_FORM_OF_REFERENCE: u32 = 0x0041_81e0;
/// `thiscall` on the actor base data at `form + 0x30` (`level`): stores the
/// level word at `+0x0c` and notifies.
const ACTOR_BASE_DATA_SET_LEVEL: u32 = 0x0047_dfe0;
/// `Interface::CreateLevelUpMenu` (Xbox PDB).
const CREATE_LEVEL_UP_MENU: u32 = 0x0070_6270;

// ---- Globals and constants ---------------------------------------------------

/// The `PlayerCharacter` singleton pointer.
const PLAYER: u32 = 0x011d_ea3c;
/// The process lists singleton (the `this` of
/// [`PROCESS_LISTS_IS_ACTOR_DETECTED`]).
const PROCESS_LISTS: u32 = 0x011e_0e80;
/// The follower-limit game setting.
const FOLLOWER_LIMIT_SETTING: u32 = 0x011c_dad0;
/// The string global whose text is the "too many followers" message.
const TOO_MANY_FOLLOWERS_MESSAGE: u32 = 0x011d_3d54;
/// `float` the message is shown with.
const MESSAGE_DURATION: u32 = 0x0101_62c0;
/// Byte at `+0x268` of the TLS block: commands echo to the console when set.
const TLS_ECHO: u32 = 0x268;

/// Virtual slot `0x130` of a package: its name.
const PACKAGE_NAME_SLOT: u32 = 0x130;
/// Virtual slot `0x284` of the actor's process: sets the package stage.
const PROCESS_SET_STAGE_SLOT: u32 = 0x284;
/// Virtual slot `0x128` of the actor's process: the actor's target.
const PROCESS_GET_TARGET_SLOT: u32 = 0x128;
/// Virtual slot `0x42c` of an actor: its combat target.
const ACTOR_GET_COMBAT_TARGET_SLOT: u32 = 0x42c;

// ---- String literals (addresses in the exe's data) ---------------------------

/// `" %s is not detected"`
const MSG_NOT_DETECTED: u32 = 0x0103_b6b8;
/// `" %s is detected"`
const MSG_DETECTED: u32 = 0x0103_b6cc;
/// `"PACKAGES: Package %s is not  %s current package"`
const MSG_PACKAGE_NOT_CURRENT: u32 = 0x0103_b6dc;
/// `"PACKAGES: Package %s is not a follow or an escort. "`
const MSG_NOT_FOLLOW_OR_ESCORT: u32 = 0x0103_b70c;
/// `"GetActionRef >> (%08x)"`
const MSG_GET_ACTION_REF: u32 = 0x0103_b740;
/// `"GetSelf >> (%08x)"`
const MSG_GET_SELF: u32 = 0x0103_b758;
/// `"GetCombatTarget >> (%08x)"`
const MSG_GET_COMBAT_TARGET: u32 = 0x0103_b76c;
/// `"GetPackageTarget >> (%08x)"`
const MSG_GET_PACKAGE_TARGET: u32 = 0x0103_b788;
/// `"GetContainer >>(%08x)"`
const MSG_GET_CONTAINER: u32 = 0x0103_b7a4;
/// `"GetParentRef >> (%08x)"`
const MSG_GET_PARENT_REF: u32 = 0x0103_b7bc;
/// `"GetLinkedRef >> (%08x)"`
const MSG_GET_LINKED_REF: u32 = 0x0103_b7d4;
/// `"GetForceRun >> %0.2f"`
const MSG_GET_FORCE_RUN: u32 = 0x0103_b7ec;
/// `"SetForceRun >> %0.2f"`
const MSG_SET_FORCE_RUN: u32 = 0x0103_b804;
/// `"GetForceSneak >> %0.2f"`
const MSG_GET_FORCE_SNEAK: u32 = 0x0103_b81c;
/// `"SetForceSneak >> %0.2f"`
const MSG_SET_FORCE_SNEAK: u32 = 0x0103_b834;

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

/// `__RTDynamicCast` of `object` from `TESObjectREFR` to `Actor`.
fn actor_of(e: &mut Engine, object: u32) -> u32 {
    e.call(
        DYNAMIC_CAST,
        &args![object, 0u32, RTTI_TES_OBJECT_REFR, RTTI_ACTOR, 0u32],
    )
    .u32()
}

/// Calls a condition function with the command's `thisObj`, `argument`, 0 and
/// the result double: its `AL`.
fn call_condition(e: &mut Engine, function: u32, a: ScriptArgs, argument: u32) -> bool {
    e.call(function, &args![a.this_obj, argument, 0u32, a.result])
        .bool()
}

/// Stores the form id of `form` into the result double through
/// `Script::PutNumericIDInDouble` and returns the id (the game keeps it in a
/// local for the echo).
fn store_form_id(e: &mut Engine, form: u32, result: Ptr) -> u32 {
    let id = e.call(GET_FORM_ID, &args![form]).u32();
    e.with_stack(4, |e, cell| {
        e.mem.set_u32(cell.addr(), id);
        e.call(PUT_NUMERIC_ID_IN_DOUBLE, &args![cell, result]);
    });
    id
}

/// The echo of the reference-returning commands: `"<name> >> (%08x)"`.
fn echo_id(e: &mut Engine, format: u32, id: u32) {
    if echo_enabled(e) {
        console_print(e, &args![format, id]);
    }
}

/// Echo of the commands that return a flag in the result double.
fn echo_result(e: &mut Engine, format: u32, result: Ptr) {
    if echo_enabled(e) {
        let value = e.mem.f64(result.addr());
        console_print(e, &args![format, value]);
    }
}

/// The check both package commands start with: the package must be the
/// actor's current package, or the one in its extra data. Otherwise the
/// logging stub gets the message and this returns false.
fn package_is_current(e: &mut Engine, actor: u32, package: u32) -> bool {
    if e.call(GET_PACKAGE_SET_AS_CURRENT, &args![actor]).u32() == package {
        return true;
    }
    let extra_data = e.call(EXTRA_DATA_LIST, &args![actor]).u32();
    if e.call(GET_PACKAGE_EXTRA, &args![extra_data]).u32() == package {
        return true;
    }
    let actor_name = e.call(GET_REFERENCE_NAME, &args![actor]).u32();
    let package_name = e.vcall(package, PACKAGE_NAME_SLOT, &args![]).u32();
    e.call(
        LOG_STUB,
        &args![MSG_PACKAGE_NOT_CURRENT, package_name, actor_name],
    );
    false
}

/// The setter commands of a faction flag: parse a faction and a flag, call
/// `setter(faction, flag != 0)`.
fn set_faction_flag(e: &mut Engine, a: ScriptArgs, setter: u32) -> bool {
    let Some([faction, flag]) = parse_params(e, a, [0, 0]) else {
        return false;
    };
    e.call(setter, &args![faction, (flag != 0) as u32]);
    true
}

// ---- Translated functions --------------------------------------------------------

// Translated from 005cd990 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Script::GetDetectionLevelFunction` (Xbox PDB): parses one actor argument
/// and returns `GetDetectionLevelConditionFunction(thisObj, actor, 0,
/// result)`.
pub fn script_get_detection_level_function(e: &mut Engine, a: ScriptArgs) -> bool {
    let Some([actor]) = parse_params(e, a, [0]) else {
        return false;
    };
    call_condition(e, GET_DETECTION_LEVEL_CONDITION, a, actor)
}

// Translated from 005cd9f0 (decompiled, FalloutNV.exe 1.4.0.525)
/// The `IsSwimming` body: false without a `thisObj`, otherwise
/// `IsSwimmingConditionFunction(thisObj, 0, 0, result)`.
pub fn fn_005cd9f0(e: &mut Engine, a: ScriptArgs) -> bool {
    if a.this_obj.is_null() {
        return false;
    }
    call_condition(e, IS_SWIMMING_CONDITION, a, 0)
}

// Translated from 005cda20 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Script::IsActorDetected` (Xbox PDB): when `thisObj` is an actor, the
/// result is 1.0 if the process lists say that it detects the player, else
/// 0.0; with the TLS echo flag set the console says whether the actor is
/// detected. Always succeeds.
pub fn script_is_actor_detected(e: &mut Engine, a: ScriptArgs) -> bool {
    let actor = actor_of(e, a.this_obj.addr());
    if actor != 0 {
        let detected = e
            .call(
                PROCESS_LISTS_IS_ACTOR_DETECTED,
                &args![PROCESS_LISTS, actor],
            )
            .u8();
        e.mem.set_f64(a.result.addr(), detected as f64);
        if echo_enabled(e) {
            let format = if e.mem.f64(a.result.addr()) == 0.0 {
                MSG_NOT_DETECTED
            } else {
                MSG_DETECTED
            };
            let name = e.call(GET_REFERENCE_NAME, &args![actor]).u32();
            console_print(e, &args![format, name]);
        }
    }
    true
}

// Translated from 005cdad0 (decompiled, FalloutNV.exe 1.4.0.525)
/// A package command (argument: a package): when `thisObj` is an actor and
/// the package is its current one, a package of type 2 gets its flag set and
/// the actor's process is told stage 2 (virtual slot `0x284`); a package of
/// type 7 or 1 only gets the flag set; any other type is reported through
/// the logging stub.
pub fn fn_005cdad0(e: &mut Engine, a: ScriptArgs) -> bool {
    let Some([package]) = parse_params(e, a, [0]) else {
        return false;
    };
    let actor = actor_of(e, a.this_obj.addr());
    if actor != 0 && package != 0 {
        if !package_is_current(e, actor, package) {
            return true;
        }
        if e.call(GET_PACKAGE_TYPE, &args![package]).i32() == 2 {
            e.call(SET_PACKAGE_FLAG, &args![package, 1u32]);
            let process = e.call(GET_PROCESS, &args![actor]).u32();
            e.vcall(process, PROCESS_SET_STAGE_SLOT, &args![2u32]);
        } else if e.call(GET_PACKAGE_TYPE, &args![package]).i32() == 7
            || e.call(GET_PACKAGE_TYPE, &args![package]).i32() == 1
        {
            e.call(SET_PACKAGE_FLAG, &args![package, 1u32]);
        } else {
            let package_name = e.vcall(package, PACKAGE_NAME_SLOT, &args![]).u32();
            e.call(LOG_STUB, &args![MSG_NOT_FOLLOW_OR_ESCORT, package_name]);
        }
    }
    true
}

// Translated from 005cdc10 (decompiled, FalloutNV.exe 1.4.0.525)
/// The counterpart of [`fn_005cdad0`] (argument: a package): after the same
/// check, a package of type 2 gets its flag cleared and the actor's process
/// is told stage 3; a package of type 7 or 1 has its flag cleared, unless the
/// actor's target (process virtual slot `0x128`, cast to an actor) is the
/// player, the player does not have the actor as a teammate yet and has more
/// teammates than the setting at `011cdad0` allows: then the "too many
/// followers" message is shown and the flag stays. Any other type is
/// reported through the logging stub.
pub fn fn_005cdc10(e: &mut Engine, a: ScriptArgs) -> bool {
    let Some([package]) = parse_params(e, a, [0]) else {
        return false;
    };
    let actor = actor_of(e, a.this_obj.addr());
    if actor != 0 && package != 0 {
        if !package_is_current(e, actor, package) {
            return true;
        }
        if e.call(GET_PACKAGE_TYPE, &args![package]).i32() == 2 {
            e.call(SET_PACKAGE_FLAG, &args![package, 0u32]);
            let process = e.call(GET_PROCESS, &args![actor]).u32();
            e.vcall(process, PROCESS_SET_STAGE_SLOT, &args![3u32]);
        } else if e.call(GET_PACKAGE_TYPE, &args![package]).i32() == 7
            || e.call(GET_PACKAGE_TYPE, &args![package]).i32() == 1
        {
            let process = e.call(GET_PROCESS, &args![actor]).u32();
            let target = e.vcall(process, PROCESS_GET_TARGET_SLOT, &args![]).u32();
            let target = actor_of(e, target);
            let player = e.global::<u32>(PLAYER);
            if target == player && !e.call(PLAYER_HAS_TEAMMATE, &args![player, actor]).bool() {
                let teammates = e.call(PLAYER_TEAMMATE_COUNT, &args![player]).i32();
                let limit_address = e
                    .call(GET_SETTING_INTEGER, &args![FOLLOWER_LIMIT_SETTING])
                    .u32();
                if teammates > e.mem.i32(limit_address) {
                    let text = e
                        .call(BS_STRING_TEXT, &args![TOO_MANY_FOLLOWERS_MESSAGE])
                        .u32();
                    let duration: f32 = e.global(MESSAGE_DURATION);
                    e.call(SHOW_MESSAGE, &args![text, 0u32, 0u32, 0u32, duration, 0u32]);
                    return true;
                }
            }
            e.call(SET_PACKAGE_FLAG, &args![package, 0u32]);
        } else {
            let package_name = e.vcall(package, PACKAGE_NAME_SLOT, &args![]).u32();
            e.call(LOG_STUB, &args![MSG_NOT_FOLLOW_OR_ESCORT, package_name]);
        }
    }
    true
}

// Translated from 005cde00 (decompiled, FalloutNV.exe 1.4.0.525)
/// Writes the player's `dword` at `+0x654` ([`fn_005cde20`]) into the result
/// double as an integer. Always succeeds.
pub fn fn_005cde00(e: &mut Engine, a: ScriptArgs) -> bool {
    let player = e.global::<u32>(PLAYER);
    let value = fn_005cde20(e, Ptr::new(player));
    e.mem.set_f64(a.result.addr(), value as f64);
    true
}

// Translated from 005cde20 (decompiled, FalloutNV.exe 1.4.0.525)
/// `PlayerCharacter`'s `dword` at `+0x654` (PC offset).
pub fn fn_005cde20(e: &mut Engine, this: Ptr) -> i32 {
    e.mem.i32(this.addr() + 0x654)
}

// Translated from 005cde40 (decompiled, FalloutNV.exe 1.4.0.525)
/// Parses one integer and stores it, with the flag 1, in the player's members
/// at `+0x654` / `+0x658` (`005c1a00`).
pub fn fn_005cde40(e: &mut Engine, a: ScriptArgs) -> bool {
    let Some([value]) = parse_params(e, a, [0]) else {
        return false;
    };
    let player = e.global::<u32>(PLAYER);
    e.call(PLAYER_SET_FIELD_654, &args![player, value, 1u32]);
    true
}

// Translated from 005cdea0 (decompiled, FalloutNV.exe 1.4.0.525)
/// The `GetAmountStolenSold` body: calls
/// `GetAmountStolenSoldConditionFunction(thisObj, 0, 0, result)`. Always
/// succeeds.
pub fn fn_005cdea0(e: &mut Engine, a: ScriptArgs) -> bool {
    call_condition(e, GET_AMOUNT_STOLEN_SOLD_CONDITION, a, 0);
    true
}

// Translated from 005cdec0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Parses one integer and adds it to the player's `dword` at `+0x6dc`
/// ([`fn_005cdf20`]).
pub fn fn_005cdec0(e: &mut Engine, a: ScriptArgs) -> bool {
    let Some([amount]) = parse_params(e, a, [0]) else {
        return false;
    };
    let player = e.global::<u32>(PLAYER);
    fn_005cdf20(e, Ptr::new(player), amount as i32);
    true
}

// Translated from 005cdf20 (decompiled, FalloutNV.exe 1.4.0.525)
/// Adds `amount` to the `dword` at `this + 0x6dc` of the player (PC offset;
/// the Xbox PDB has `iAmountStolenSold` at `+0x6ec`).
pub fn fn_005cdf20(e: &mut Engine, this: Ptr, amount: i32) {
    let address = this.addr() + 0x6dc;
    let sum = e.mem.i32(address).wrapping_add(amount);
    e.mem.set_u32(address, sum as u32);
}

// Translated from 005cdf50 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Script::GetPCExpelledFunction` (Xbox PDB): parses one faction and calls
/// `GetPCExpelledConditionFunction(thisObj, faction, 0, result)`. Succeeds
/// whenever the arguments parse.
pub fn script_get_pc_expelled_function(e: &mut Engine, a: ScriptArgs) -> bool {
    let Some([faction]) = parse_params(e, a, [0]) else {
        return false;
    };
    call_condition(e, GET_PC_EXPELLED_CONDITION, a, faction);
    true
}

// Translated from 005cdfb0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Script::SetPCExpelledFunction` (Xbox PDB): parses a faction and a flag
/// and sets or clears the faction's expelled flag ([`fn_005ce040`]). When
/// setting it, the owner of the player's parent cell is looked up
/// (`TESObjectCELL::GetOwner`, result unused).
pub fn script_set_pc_expelled_function(e: &mut Engine, a: ScriptArgs) -> bool {
    let Some([faction, flag]) = parse_params(e, a, [0, 0]) else {
        return false;
    };
    if flag != 0 {
        fn_005ce040(e, Ptr::new(faction), 1);
        let player = e.global::<u32>(PLAYER);
        let cell = e.call(GET_PARENT_CELL, &args![player]).u32();
        if cell != 0 {
            e.call(CELL_GET_OWNER, &args![cell]);
        }
    } else {
        fn_005ce040(e, Ptr::new(faction), 0);
    }
    true
}

// Translated from 005ce040 (decompiled, FalloutNV.exe 1.4.0.525)
/// Sets or clears the faction flag `0x08` (the expelled flag).
pub fn fn_005ce040(e: &mut Engine, this: Ptr, flag: u8) {
    e.call(
        FACTION_SET_FLAG_BITS,
        &args![this, FACTION_FLAG_EXPELLED, flag as u32],
    );
}

// Translated from 005ce060 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Script::GetPCFactionMurderFunction` (Xbox PDB): parses one faction and
/// calls `GetPCFactionMurderConditionFunction(thisObj, faction, 0, result)`.
pub fn script_get_pc_faction_murder_function(e: &mut Engine, a: ScriptArgs) -> bool {
    let Some([faction]) = parse_params(e, a, [0]) else {
        return false;
    };
    call_condition(e, GET_PC_FACTION_MURDER_CONDITION, a, faction);
    true
}

// Translated from 005ce0c0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Parses a faction and a flag and sets or clears the faction flag `0x40`
/// (`0047ebb0`).
pub fn fn_005ce0c0(e: &mut Engine, a: ScriptArgs) -> bool {
    set_faction_flag(e, a, FACTION_SET_MURDER_FLAG)
}

// Translated from 005ce130 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Script::GetPlayerEnemyofFactionFunction` (Xbox PDB): parses one faction
/// and calls `GetPlayerEnemyofFactionConditionFunction(thisObj, faction, 0,
/// result)`.
pub fn script_get_player_enemyof_faction_function(e: &mut Engine, a: ScriptArgs) -> bool {
    let Some([faction]) = parse_params(e, a, [0]) else {
        return false;
    };
    call_condition(e, GET_PLAYER_ENEMY_OF_FACTION_CONDITION, a, faction);
    true
}

// Translated from 005ce190 (decompiled, FalloutNV.exe 1.4.0.525)
/// Parses a faction and a flag and sets or clears a faction flag
/// (`0047eb90`).
pub fn fn_005ce190(e: &mut Engine, a: ScriptArgs) -> bool {
    set_faction_flag(e, a, FACTION_SET_ENEMY_FLAG)
}

// Translated from 005ce200 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Script::GetPCFactionAttackFunction` (Xbox PDB): parses one faction and
/// calls `GetPCFactionAttackConditionFunction(thisObj, faction, 0, result)`.
pub fn script_get_pc_faction_attack_function(e: &mut Engine, a: ScriptArgs) -> bool {
    let Some([faction]) = parse_params(e, a, [0]) else {
        return false;
    };
    call_condition(e, GET_PC_FACTION_ATTACK_CONDITION, a, faction);
    true
}

// Translated from 005ce260 (decompiled, FalloutNV.exe 1.4.0.525)
/// Parses a faction and a flag and sets or clears a faction flag
/// (`0047ebd0`).
pub fn fn_005ce260(e: &mut Engine, a: ScriptArgs) -> bool {
    set_faction_flag(e, a, FACTION_SET_ATTACK_FLAG)
}

// Translated from 005ce2d0 (decompiled, FalloutNV.exe 1.4.0.525)
/// The `GetDestroyed` body: calls `GetDestroyedConditionFunction(thisObj, 0,
/// 0, result)`. Always succeeds.
pub fn fn_005ce2d0(e: &mut Engine, a: ScriptArgs) -> bool {
    call_condition(e, GET_DESTROYED_CONDITION, a, 0);
    true
}

// Translated from 005ce2f0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Parses a flag; with a `thisObj`, sets or clears form flag `0x800000` on it
/// (`00484650`).
pub fn fn_005ce2f0(e: &mut Engine, a: ScriptArgs) -> bool {
    let Some([flag]) = parse_params(e, a, [0]) else {
        return false;
    };
    if !a.this_obj.is_null() {
        e.call(FORM_SET_FLAG_800000, &args![a.this_obj, (flag != 0) as u32]);
    }
    true
}

// Translated from 005ce360 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Script::GetActionRefFunction` (Xbox PDB): the result is the form id of
/// `thisObj`'s action reference (0 when it has none), echoed to the console
/// when the TLS flag is set. Always succeeds.
pub fn script_get_action_ref_function(e: &mut Engine, a: ScriptArgs) -> bool {
    e.mem.set_f64(a.result.addr(), 0.0);
    let mut id = 0;
    if !a.this_obj.is_null() && e.call(GET_ACTION_REF, &args![a.this_obj]).u32() != 0 {
        let action_ref = e.call(GET_ACTION_REF, &args![a.this_obj]).u32();
        id = store_form_id(e, action_ref, a.result);
    }
    echo_id(e, MSG_GET_ACTION_REF, id);
    true
}

// Translated from 005ce3e0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Script::GetSelfFunction` (Xbox PDB): the result is the form id of
/// `thisObj`, except for a reference that does not persist and whose base
/// form's type can be held by a container (then it stays 0). Echoed when the
/// TLS flag is set. Always succeeds.
pub fn script_get_self_function(e: &mut Engine, a: ScriptArgs) -> bool {
    e.mem.set_f64(a.result.addr(), 0.0);
    let mut id = 0;
    if !a.this_obj.is_null() {
        let mut report = true;
        if !e.call(GET_REF_PERSISTS, &args![a.this_obj]).bool() {
            let base = e.call(GET_BASE_FORM, &args![a.this_obj]).u32();
            let form_type = e.call(FORM_TYPE, &args![base]).u8();
            report = !e
                .call(CONTAINER_CAN_HOLD_TYPE, &args![form_type as u32])
                .bool();
        }
        if report {
            id = store_form_id(e, a.this_obj.addr(), a.result);
        }
    }
    echo_id(e, MSG_GET_SELF, id);
    true
}

// Translated from 005ce480 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Script::GetCombatTargetFunction` (Xbox PDB): when `thisObj` is an actor
/// with a combat target (virtual slot `0x42c`), the result is the target's
/// form id. Echoed when the TLS flag is set. Always succeeds.
pub fn script_get_combat_target_function(e: &mut Engine, a: ScriptArgs) -> bool {
    e.mem.set_f64(a.result.addr(), 0.0);
    let actor = actor_of(e, a.this_obj.addr());
    let mut id = 0;
    if actor != 0 {
        let target = e.vcall(actor, ACTOR_GET_COMBAT_TARGET_SLOT, &args![]).u32();
        if target != 0 {
            id = store_form_id(e, target, a.result);
        }
    }
    echo_id(e, MSG_GET_COMBAT_TARGET, id);
    true
}

// Translated from 005ce520 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Script::GetPackageTargetFunction` (Xbox PDB): when `thisObj` is an actor
/// with a current package target, the result is the target's form id. Echoed
/// when the TLS flag is set. Always succeeds.
pub fn script_get_package_target_function(e: &mut Engine, a: ScriptArgs) -> bool {
    e.mem.set_f64(a.result.addr(), 0.0);
    let actor = actor_of(e, a.this_obj.addr());
    let mut id = 0;
    if actor != 0 {
        let target = e.call(GET_CURRENT_PACKAGE_TARGET, &args![actor]).u32();
        if target != 0 {
            id = store_form_id(e, target, a.result);
        }
    }
    echo_id(e, MSG_GET_PACKAGE_TARGET, id);
    true
}

// Translated from 005ce5c0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Script::GetContainerFunction` (Xbox PDB): with both `thisObj` and the
/// containing object, the result is the containing object's form id. Echoed
/// when the TLS flag is set. Always succeeds.
pub fn script_get_container_function(e: &mut Engine, a: ScriptArgs) -> bool {
    e.mem.set_f64(a.result.addr(), 0.0);
    let mut id = 0;
    if !a.this_obj.is_null() && !a.containing_obj.is_null() {
        id = store_form_id(e, a.containing_obj.addr(), a.result);
    }
    echo_id(e, MSG_GET_CONTAINER, id);
    true
}

// Translated from 005ce630 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Script::GetParentRefFunction` (Xbox PDB): the result is the form id of
/// the reference `0056a9f0` finds for `thisObj` (0 when none). Echoed when
/// the TLS flag is set. Always succeeds.
pub fn script_get_parent_ref_function(e: &mut Engine, a: ScriptArgs) -> bool {
    e.mem.set_f64(a.result.addr(), 0.0);
    let mut id = 0;
    if !a.this_obj.is_null() && e.call(GET_PARENT_REF, &args![a.this_obj]).u32() != 0 {
        let parent = e.call(GET_PARENT_REF, &args![a.this_obj]).u32();
        id = store_form_id(e, parent, a.result);
    }
    echo_id(e, MSG_GET_PARENT_REF, id);
    true
}

// Translated from 005ce6b0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Script::GetLinkedRefFunction` (Xbox PDB): the result is the form id of
/// the reference `00569b80` finds for `thisObj` (0 when none). Echoed when
/// the TLS flag is set. Always succeeds.
pub fn script_get_linked_ref_function(e: &mut Engine, a: ScriptArgs) -> bool {
    e.mem.set_f64(a.result.addr(), 0.0);
    let mut id = 0;
    if !a.this_obj.is_null() && e.call(GET_LINKED_REF, &args![a.this_obj]).u32() != 0 {
        let linked = e.call(GET_LINKED_REF, &args![a.this_obj]).u32();
        id = store_form_id(e, linked, a.result);
    }
    echo_id(e, MSG_GET_LINKED_REF, id);
    true
}

// Translated from 005ce730 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Script::GetForceRun` (Xbox PDB): the result is 1.0 when `thisObj` is an
/// actor whose force-run byte (`+0x124`) is set, else 0.0. Echoed when the
/// TLS flag is set. Always succeeds.
pub fn script_get_force_run(e: &mut Engine, a: ScriptArgs) -> bool {
    e.mem.set_f64(a.result.addr(), 0.0);
    let actor = actor_of(e, a.this_obj.addr());
    if actor != 0 && e.call(ACTOR_GET_FORCE_RUN, &args![actor]).bool() {
        e.mem.set_f64(a.result.addr(), 1.0);
    }
    echo_result(e, MSG_GET_FORCE_RUN, a.result);
    true
}

// Translated from 005ce7b0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Script::SetForceRun` (Xbox PDB): when `thisObj` is an actor, the result is
/// set to 1.0, one flag is parsed and stored as the actor's force-run byte
/// (`005bf800`); echoed when the TLS flag is set (the parsed integer is
/// passed as it is to the `%0.2f` format, as the game does). Fails when the
/// flag does not parse; succeeds without an actor.
pub fn script_set_force_run(e: &mut Engine, a: ScriptArgs) -> bool {
    let actor = actor_of(e, a.this_obj.addr());
    if actor != 0 {
        e.mem.set_f64(a.result.addr(), 1.0);
        let Some([flag]) = parse_params(e, a, [0]) else {
            return false;
        };
        e.call(ACTOR_SET_FORCE_RUN, &args![actor, (flag != 0) as u32]);
        if echo_enabled(e) {
            console_print(e, &args![MSG_SET_FORCE_RUN, flag]);
        }
    }
    true
}

// Translated from 005ce870 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Script::GetForceSneak` (Xbox PDB): the result is 1.0 when `thisObj` is an
/// actor whose force-sneak byte (`+0x125`) is set, else 0.0. Echoed when the
/// TLS flag is set. Always succeeds.
pub fn script_get_force_sneak(e: &mut Engine, a: ScriptArgs) -> bool {
    e.mem.set_f64(a.result.addr(), 0.0);
    let actor = actor_of(e, a.this_obj.addr());
    if actor != 0 && e.call(ACTOR_GET_FORCE_SNEAK, &args![actor]).bool() {
        e.mem.set_f64(a.result.addr(), 1.0);
    }
    echo_result(e, MSG_GET_FORCE_SNEAK, a.result);
    true
}

// Translated from 005ce910 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Script::SetForceSneak` (Xbox PDB): like [`script_set_force_run`] for the
/// force-sneak byte (`+0x125`, [`fn_005ce9d0`]).
pub fn script_set_force_sneak(e: &mut Engine, a: ScriptArgs) -> bool {
    let actor = actor_of(e, a.this_obj.addr());
    if actor != 0 {
        e.mem.set_f64(a.result.addr(), 1.0);
        let Some([flag]) = parse_params(e, a, [0]) else {
            return false;
        };
        fn_005ce9d0(e, Ptr::new(actor), (flag != 0) as u8);
        if echo_enabled(e) {
            console_print(e, &args![MSG_SET_FORCE_SNEAK, flag]);
        }
    }
    true
}

// Translated from 005ce9d0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Stores the force-sneak byte at `this + 0x125` of an actor (PC offset).
pub fn fn_005ce9d0(e: &mut Engine, this: Ptr, flag: u8) {
    e.mem.set_u8(this.addr() + 0x125, flag);
}

// Translated from 005ce9f0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Raises the player's base level by one and opens the level-up menu
/// (`Interface::CreateLevelUpMenu`); nothing without a player. The new level
/// is written to the base data at `base form + 0x30` (`0047dfe0`).
pub fn fn_005ce9f0(e: &mut Engine) -> bool {
    let player = e.global::<u32>(PLAYER);
    if player != 0 {
        let level = e.call(PLAYER_GET_LEVEL, &args![player]).u16() as u32;
        let base = e.call(GET_BASE_FORM_OF_REFERENCE, &args![player]).u32();
        e.call(ACTOR_BASE_DATA_SET_LEVEL, &args![base + 0x30, level + 1]);
        e.call(CREATE_LEVEL_UP_MENU, &args![]);
    }
    true
}

// Translated from 005cea30 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Script::HasMagicEffectFunction` (Xbox PDB): parses one magic effect and
/// returns `HasMagicEffectConditionFunction(thisObj, effect, 0, result)`.
pub fn script_has_magic_effect_function(e: &mut Engine, a: ScriptArgs) -> bool {
    let Some([effect]) = parse_params(e, a, [0]) else {
        return false;
    };
    call_condition(e, HAS_MAGIC_EFFECT_CONDITION, a, effect)
}

// Translated from 005cea90 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Script::IsSpellTargetFunction` (Xbox PDB): parses one spell and returns
/// `IsSpellTargetConditionFunction(thisObj, spell, 0, result)`.
pub fn script_is_spell_target_function(e: &mut Engine, a: ScriptArgs) -> bool {
    let Some([spell]) = parse_params(e, a, [0]) else {
        return false;
    };
    call_condition(e, IS_SPELL_TARGET_CONDITION, a, spell)
}

// Translated from 005ceaf0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Script::GetSpellUsageNumberFunction` (Xbox PDB): parses one spell and
/// returns `GetSpellUsageNumberConditionFunction(thisObj, spell, 0, result)`.
pub fn script_get_spell_usage_number_function(e: &mut Engine, a: ScriptArgs) -> bool {
    let Some([spell]) = parse_params(e, a, [0]) else {
        return false;
    };
    call_condition(e, GET_SPELL_USAGE_NUMBER_CONDITION, a, spell)
}

// Translated from 005ceb50 (decompiled, FalloutNV.exe 1.4.0.525)
/// The `GetVATSMode` body: returns `GetVATSModeConditionFunction(thisObj, 0,
/// 0, result)`.
pub fn fn_005ceb50(e: &mut Engine, a: ScriptArgs) -> bool {
    call_condition(e, GET_VATS_MODE_CONDITION, a, 0)
}

// Translated from 005ceb70 (decompiled, FalloutNV.exe 1.4.0.525)
/// The `GetVATSTargetHeight` body: returns
/// `GetVATSTargetHeightConditionFunction(thisObj, 0, 0, result)`.
pub fn fn_005ceb70(e: &mut Engine, a: ScriptArgs) -> bool {
    call_condition(e, GET_VATS_TARGET_HEIGHT_CONDITION, a, 0)
}

/// This part's translated functions, by exe address.
pub fn funcs() -> Vec<(u32, AbiFn)> {
    vec![
        entry!(0x005cd990, script_get_detection_level_function(ScriptArgs) -> bool),
        entry!(0x005cd9f0, fn_005cd9f0(ScriptArgs) -> bool),
        entry!(0x005cda20, script_is_actor_detected(ScriptArgs) -> bool),
        entry!(0x005cdad0, fn_005cdad0(ScriptArgs) -> bool),
        entry!(0x005cdc10, fn_005cdc10(ScriptArgs) -> bool),
        entry!(0x005cde00, fn_005cde00(ScriptArgs) -> bool),
        entry!(0x005cde20, fn_005cde20(Ptr) -> i32),
        entry!(0x005cde40, fn_005cde40(ScriptArgs) -> bool),
        entry!(0x005cdea0, fn_005cdea0(ScriptArgs) -> bool),
        entry!(0x005cdec0, fn_005cdec0(ScriptArgs) -> bool),
        entry!(0x005cdf20, fn_005cdf20(Ptr, i32)),
        entry!(0x005cdf50, script_get_pc_expelled_function(ScriptArgs) -> bool),
        entry!(0x005cdfb0, script_set_pc_expelled_function(ScriptArgs) -> bool),
        entry!(0x005ce040, fn_005ce040(Ptr, u8)),
        entry!(0x005ce060, script_get_pc_faction_murder_function(ScriptArgs) -> bool),
        entry!(0x005ce0c0, fn_005ce0c0(ScriptArgs) -> bool),
        entry!(0x005ce130, script_get_player_enemyof_faction_function(ScriptArgs) -> bool),
        entry!(0x005ce190, fn_005ce190(ScriptArgs) -> bool),
        entry!(0x005ce200, script_get_pc_faction_attack_function(ScriptArgs) -> bool),
        entry!(0x005ce260, fn_005ce260(ScriptArgs) -> bool),
        entry!(0x005ce2d0, fn_005ce2d0(ScriptArgs) -> bool),
        entry!(0x005ce2f0, fn_005ce2f0(ScriptArgs) -> bool),
        entry!(0x005ce360, script_get_action_ref_function(ScriptArgs) -> bool),
        entry!(0x005ce3e0, script_get_self_function(ScriptArgs) -> bool),
        entry!(0x005ce480, script_get_combat_target_function(ScriptArgs) -> bool),
        entry!(0x005ce520, script_get_package_target_function(ScriptArgs) -> bool),
        entry!(0x005ce5c0, script_get_container_function(ScriptArgs) -> bool),
        entry!(0x005ce630, script_get_parent_ref_function(ScriptArgs) -> bool),
        entry!(0x005ce6b0, script_get_linked_ref_function(ScriptArgs) -> bool),
        entry!(0x005ce730, script_get_force_run(ScriptArgs) -> bool),
        entry!(0x005ce7b0, script_set_force_run(ScriptArgs) -> bool),
        entry!(0x005ce870, script_get_force_sneak(ScriptArgs) -> bool),
        entry!(0x005ce910, script_set_force_sneak(ScriptArgs) -> bool),
        entry!(0x005ce9d0, fn_005ce9d0(Ptr, u8)),
        entry!(0x005ce9f0, fn_005ce9f0() -> bool),
        entry!(0x005cea30, script_has_magic_effect_function(ScriptArgs) -> bool),
        entry!(0x005cea90, script_is_spell_target_function(ScriptArgs) -> bool),
        entry!(0x005ceaf0, script_get_spell_usage_number_function(ScriptArgs) -> bool),
        entry!(0x005ceb50, fn_005ceb50(ScriptArgs) -> bool),
        entry!(0x005ceb70, fn_005ceb70(ScriptArgs) -> bool),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    // Fake virtual functions the tests put into vtables.
    const V_NAME: u32 = 0x0900_0001;
    const V_STAGE: u32 = 0x0900_0002;
    const V_TARGET: u32 = 0x0900_0003;
    const V_COMBAT_TARGET: u32 = 0x0900_0004;

    /// An engine with the pages of the globals these commands touch mapped
    /// and the accessors every command uses replaced by doubles that behave
    /// like the exe's code (see the constants' documentation).
    fn engine() -> Engine {
        let mut e = Engine::new();
        e.map(0x0101_6000, 0x1000);
        e.map(0x011d_e000, 0x1000);
        e.register(CONSOLE_PRINT, |_, _| Ret::default());
        // `__RTDynamicCast`: every object in these tests is an actor.
        e.register(DYNAMIC_CAST, |_, a| a[0].into_ret());
        e.register(GET_FORM_ID, |e, a| e.mem.u32(a[0] + 0x0c).into_ret());
        e.register(PUT_NUMERIC_ID_IN_DOUBLE, |e, a| {
            let id = e.mem.u32(a[0]);
            e.mem.set_f64(a[1], id as f64);
            Ret::default()
        });
        e.register(EXTRA_DATA_LIST, |_, a| (a[0] + 0x44).into_ret());
        e.register(GET_REFERENCE_NAME, |_, _| 0xaaaa.into_ret());
        e.register(V_NAME, |_, _| 0xbbbb.into_ret());
        e.register(V_STAGE, |_, _| Ret::default());
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
        let player = object(e);
        e.set_global(PLAYER, player);
        player
    }

    /// `ParseParameters` gets: info, data, opcode offset, thisObj,
    /// containing, script, event list, then the address of the local.
    fn assert_parsed(e: &Engine, this_obj: u32) {
        assert_eq!(
            calls(e, PARSE_PARAMETERS)[0][..7],
            [1, 2, 8, this_obj, 0, 5, 6]
        );
    }

    /// The test of a "parse one value, hand it to a condition function"
    /// command: the condition function gets `(thisObj, value, 0, result)`.
    fn check_parse_then_condition(command_address: u32, condition: u32, returns_condition: bool) {
        let mut e = engine();
        let this_obj = object(&mut e);
        let a = command(&mut e, this_obj);
        parse_gives(&mut e, true, &[0x77]);
        e.register(condition, |_, _| true.into_ret());
        start_log(&mut e);
        assert!(e.call(command_address, &args![a]).bool());
        assert_parsed(&e, this_obj);
        assert_eq!(
            calls(&e, condition),
            vec![vec![this_obj, 0x77, 0, a.result.addr()]]
        );
        if returns_condition {
            // The condition function's AL is the command's.
            e.register(condition, |_, _| false.into_ret());
            assert!(!e.call(command_address, &args![a]).bool());
        }
        // Parameters that do not parse: false, nothing called.
        parse_gives(&mut e, false, &[]);
        start_log(&mut e);
        assert!(!e.call(command_address, &args![a]).bool());
        assert!(calls(&e, condition).is_empty());
    }

    /// The test of a "no arguments, call a condition function" command.
    fn check_condition_only(command_address: u32, condition: u32, returns_condition: bool) {
        let mut e = engine();
        let this_obj = object(&mut e);
        let a = command(&mut e, this_obj);
        e.register(condition, |_, _| false.into_ret());
        start_log(&mut e);
        let result = e.call(command_address, &args![a]).bool();
        assert_eq!(result, !returns_condition);
        assert_eq!(
            calls(&e, condition),
            vec![vec![this_obj, 0, 0, a.result.addr()]]
        );
        e.register(condition, |_, _| true.into_ret());
        assert!(e.call(command_address, &args![a]).bool());
    }

    #[test]
    fn get_detection_level_passes_the_parsed_actor_to_the_condition_function() {
        check_parse_then_condition(0x005c_d990, GET_DETECTION_LEVEL_CONDITION, true);
    }

    #[test]
    fn fn_005cd9f0_needs_a_reference_and_returns_the_condition_function_s_result() {
        let mut e = engine();
        let a = command(&mut e, 0);
        e.register(IS_SWIMMING_CONDITION, |_, _| true.into_ret());
        start_log(&mut e);
        assert!(!e.call(0x005c_d9f0, &args![a]).bool());
        assert!(calls(&e, IS_SWIMMING_CONDITION).is_empty());
        let this_obj = object(&mut e);
        let a = command(&mut e, this_obj);
        assert!(e.call(0x005c_d9f0, &args![a]).bool());
        assert_eq!(
            calls(&e, IS_SWIMMING_CONDITION),
            vec![vec![this_obj, 0, 0, a.result.addr()]]
        );
        e.register(IS_SWIMMING_CONDITION, |_, _| false.into_ret());
        assert!(!e.call(0x005c_d9f0, &args![a]).bool());
    }

    #[test]
    fn is_actor_detected_reports_the_process_lists_answer() {
        let mut e = engine();
        let actor = object(&mut e);
        e.register(PROCESS_LISTS_IS_ACTOR_DETECTED, |_, _| 1u32.into_ret());
        let a = command(&mut e, actor);
        start_log(&mut e);
        assert!(e.call(0x005c_da20, &args![a]).bool());
        assert_eq!(e.mem.f64(a.result.addr()), 1.0);
        assert_eq!(
            calls(&e, PROCESS_LISTS_IS_ACTOR_DETECTED),
            vec![vec![PROCESS_LISTS, actor]]
        );
        // No echo flag: nothing printed.
        assert!(calls(&e, CONSOLE_PRINT).is_empty());

        // Echo: the name and the verdict are printed.
        set_echo(&mut e, true);
        assert!(e.call(0x005c_da20, &args![a]).bool());
        assert_eq!(calls(&e, CONSOLE_PRINT), vec![vec![MSG_DETECTED, 0xaaaa]]);
        e.register(PROCESS_LISTS_IS_ACTOR_DETECTED, |_, _| 0u32.into_ret());
        assert!(e.call(0x005c_da20, &args![a]).bool());
        assert_eq!(e.mem.f64(a.result.addr()), 0.0);
        assert_eq!(
            calls(&e, CONSOLE_PRINT).last().unwrap(),
            &vec![MSG_NOT_DETECTED, 0xaaaa]
        );

        // Not an actor (the cast gives null): nothing happens, the command
        // still succeeds.
        e.register(DYNAMIC_CAST, |_, _| Ret::default());
        start_log(&mut e);
        assert!(e.call(0x005c_da20, &args![a]).bool());
        assert!(calls(&e, PROCESS_LISTS_IS_ACTOR_DETECTED).is_empty());
    }

    /// An actor with a package of the given type, the stubs for the package
    /// commands, and the current package either the package or another one.
    /// Returns `(actor, package, process)`.
    fn package_scene(e: &mut Engine, package_type: i8, current: bool) -> (u32, u32, u32) {
        let package = object_with(e, &[(PACKAGE_NAME_SLOT, V_NAME)]);
        e.mem.set_u8(package + 0x20, package_type as u8);
        let process = object_with(e, &[(PROCESS_SET_STAGE_SLOT, V_STAGE)]);
        let actor = object(e);
        e.mem.set_u32(actor + 0x68, process);
        let current_package = if current { package } else { 0x1234 };
        e.register_double(GET_PACKAGE_SET_AS_CURRENT, move |_, _| {
            current_package.into_ret()
        });
        e.register(GET_PACKAGE_EXTRA, |_, _| Ret::default());
        e.register(GET_PACKAGE_TYPE, |e, a| {
            (e.mem.u8(a[0] + 0x20) as i8 as i32 as u32).into_ret()
        });
        e.register(GET_PROCESS, |e, a| e.mem.u32(a[0] + 0x68).into_ret());
        e.register(SET_PACKAGE_FLAG, |_, _| Ret::default());
        (actor, package, process)
    }

    #[test]
    fn fn_005cdad0_sets_the_package_flag_by_package_type() {
        // Type 2: flag set, the process told stage 2.
        let mut e = engine();
        let (actor, package, process) = package_scene(&mut e, 2, true);
        parse_gives(&mut e, true, &[package]);
        let a = command(&mut e, actor);
        start_log(&mut e);
        assert!(e.call(0x005c_dad0, &args![a]).bool());
        assert_parsed(&e, actor);
        assert_eq!(calls(&e, SET_PACKAGE_FLAG), vec![vec![package, 1]]);
        assert_eq!(calls(&e, V_STAGE), vec![vec![process, 2]]);

        // Types 7 and 1: only the flag.
        for kind in [7, 1] {
            let (actor, package, _) = package_scene(&mut e, kind, true);
            parse_gives(&mut e, true, &[package]);
            let a = command(&mut e, actor);
            start_log(&mut e);
            assert!(e.call(0x005c_dad0, &args![a]).bool());
            assert_eq!(calls(&e, SET_PACKAGE_FLAG), vec![vec![package, 1]]);
            assert!(calls(&e, V_STAGE).is_empty());
        }

        // Another type: reported, nothing set.
        let (actor, package, _) = package_scene(&mut e, 5, true);
        parse_gives(&mut e, true, &[package]);
        let a = command(&mut e, actor);
        start_log(&mut e);
        assert!(e.call(0x005c_dad0, &args![a]).bool());
        assert!(calls(&e, SET_PACKAGE_FLAG).is_empty());
        assert_eq!(
            calls(&e, LOG_STUB),
            vec![vec![MSG_NOT_FOLLOW_OR_ESCORT, 0xbbbb]]
        );

        // Not the actor's current package, nor the one in its extra data:
        // reported with both names, nothing set.
        let (actor, package, _) = package_scene(&mut e, 2, false);
        parse_gives(&mut e, true, &[package]);
        let a = command(&mut e, actor);
        start_log(&mut e);
        assert!(e.call(0x005c_dad0, &args![a]).bool());
        assert!(calls(&e, SET_PACKAGE_FLAG).is_empty());
        assert_eq!(
            calls(&e, LOG_STUB),
            vec![vec![MSG_PACKAGE_NOT_CURRENT, 0xbbbb, 0xaaaa]]
        );

        // The package in the actor's extra data counts as current.
        let (actor, package, _) = package_scene(&mut e, 2, false);
        e.register_double(GET_PACKAGE_EXTRA, move |_, _| package.into_ret());
        parse_gives(&mut e, true, &[package]);
        let a = command(&mut e, actor);
        start_log(&mut e);
        assert!(e.call(0x005c_dad0, &args![a]).bool());
        assert_eq!(calls(&e, SET_PACKAGE_FLAG), vec![vec![package, 1]]);

        // No package, or parameters that do not parse.
        parse_gives(&mut e, true, &[0]);
        start_log(&mut e);
        assert!(e.call(0x005c_dad0, &args![a]).bool());
        assert!(calls(&e, SET_PACKAGE_FLAG).is_empty());
        parse_gives(&mut e, false, &[]);
        assert!(!e.call(0x005c_dad0, &args![a]).bool());
    }

    /// The doubles `fn_005cdc10` needs for its follower check; the actor's
    /// target is `target`.
    fn follower_scene(e: &mut Engine, target: u32, teammate: bool, count: i32, limit: i32) -> u32 {
        let player = set_player(e);
        e.register_double(V_TARGET, move |_, _| target.into_ret());
        e.register_double(PLAYER_HAS_TEAMMATE, move |_, _| teammate.into_ret());
        e.register_double(PLAYER_TEAMMATE_COUNT, move |_, _| (count as u32).into_ret());
        let cell = e.mem.alloc(4);
        e.mem.set_u32(cell, limit as u32);
        e.register_double(GET_SETTING_INTEGER, move |_, _| cell.into_ret());
        e.register(BS_STRING_TEXT, |_, _| 0x7777.into_ret());
        e.register(SHOW_MESSAGE, |_, _| Ret::default());
        e.set_global(MESSAGE_DURATION, 2.5f32);
        player
    }

    #[test]
    fn fn_005cdc10_clears_the_package_flag_unless_the_player_has_too_many_followers() {
        // Type 2: flag cleared, the process told stage 3.
        let mut e = engine();
        let (actor, package, process) = package_scene(&mut e, 2, true);
        parse_gives(&mut e, true, &[package]);
        let a = command(&mut e, actor);
        start_log(&mut e);
        assert!(e.call(0x005c_dc10, &args![a]).bool());
        assert_parsed(&e, actor);
        assert_eq!(calls(&e, SET_PACKAGE_FLAG), vec![vec![package, 0]]);
        assert_eq!(calls(&e, V_STAGE), vec![vec![process, 3]]);

        // Type 7 with another target: the flag is cleared.
        let (actor, package, process) = package_scene(&mut e, 7, true);
        let vtable = e.mem.alloc(0x800);
        e.mem.set_u32(vtable + PROCESS_GET_TARGET_SLOT, V_TARGET);
        e.mem.set_u32(process, vtable);
        let player = follower_scene(&mut e, 0x5555, false, 9, 3);
        parse_gives(&mut e, true, &[package]);
        let a = command(&mut e, actor);
        start_log(&mut e);
        assert!(e.call(0x005c_dc10, &args![a]).bool());
        assert_eq!(calls(&e, SET_PACKAGE_FLAG), vec![vec![package, 0]]);
        assert!(calls(&e, SHOW_MESSAGE).is_empty());

        // The target is the player, who has no room for another follower:
        // the message is shown, the flag stays.
        let player_target = player;
        follower_scene(&mut e, player_target, false, 4, 3);
        start_log(&mut e);
        assert!(e.call(0x005c_dc10, &args![a]).bool());
        assert!(calls(&e, SET_PACKAGE_FLAG).is_empty());
        assert_eq!(
            calls(&e, PLAYER_HAS_TEAMMATE),
            vec![vec![player_target, actor]]
        );
        assert_eq!(
            calls(&e, SHOW_MESSAGE),
            vec![vec![0x7777, 0, 0, 0, 2.5f32.to_bits(), 0]]
        );
        assert_eq!(
            calls(&e, GET_SETTING_INTEGER),
            vec![vec![FOLLOWER_LIMIT_SETTING]]
        );

        // The count is not above the limit: cleared.
        follower_scene(&mut e, player_target, false, 3, 3);
        start_log(&mut e);
        assert!(e.call(0x005c_dc10, &args![a]).bool());
        assert_eq!(calls(&e, SET_PACKAGE_FLAG), vec![vec![package, 0]]);
        assert!(calls(&e, SHOW_MESSAGE).is_empty());

        // The actor is already a teammate: cleared, no count asked.
        follower_scene(&mut e, player_target, true, 9, 3);
        start_log(&mut e);
        assert!(e.call(0x005c_dc10, &args![a]).bool());
        assert_eq!(calls(&e, SET_PACKAGE_FLAG), vec![vec![package, 0]]);
        assert!(calls(&e, PLAYER_TEAMMATE_COUNT).is_empty());

        // Another type is reported; a package that is not current stops.
        let (actor, package, _) = package_scene(&mut e, 4, true);
        parse_gives(&mut e, true, &[package]);
        let a = command(&mut e, actor);
        start_log(&mut e);
        assert!(e.call(0x005c_dc10, &args![a]).bool());
        assert_eq!(
            calls(&e, LOG_STUB),
            vec![vec![MSG_NOT_FOLLOW_OR_ESCORT, 0xbbbb]]
        );
        let (actor, package, _) = package_scene(&mut e, 2, false);
        parse_gives(&mut e, true, &[package]);
        let a = command(&mut e, actor);
        start_log(&mut e);
        assert!(e.call(0x005c_dc10, &args![a]).bool());
        assert!(calls(&e, SET_PACKAGE_FLAG).is_empty());
        assert_eq!(
            calls(&e, LOG_STUB),
            vec![vec![MSG_PACKAGE_NOT_CURRENT, 0xbbbb, 0xaaaa]]
        );
        parse_gives(&mut e, false, &[]);
        assert!(!e.call(0x005c_dc10, &args![a]).bool());
    }

    #[test]
    fn fn_005cde00_stores_the_players_member_as_an_integer() {
        let mut e = engine();
        let player = set_player(&mut e);
        e.mem.set_u32(player + 0x654, (-5i32) as u32);
        let a = command(&mut e, 0);
        assert!(e.call(0x005c_de00, &args![a]).bool());
        assert_eq!(e.mem.f64(a.result.addr()), -5.0);
    }

    #[test]
    fn fn_005cde20_reads_the_member_at_0x654() {
        let mut e = engine();
        let player = object(&mut e);
        e.mem.set_u32(player + 0x654, 41);
        assert_eq!(e.call(0x005c_de20, &args![player]).i32(), 41);
    }

    #[test]
    fn fn_005cde40_stores_the_parsed_value_in_the_player() {
        let mut e = engine();
        let player = set_player(&mut e);
        let a = command(&mut e, 0);
        parse_gives(&mut e, true, &[7]);
        e.register(PLAYER_SET_FIELD_654, |_, _| Ret::default());
        start_log(&mut e);
        assert!(e.call(0x005c_de40, &args![a]).bool());
        assert_eq!(calls(&e, PLAYER_SET_FIELD_654), vec![vec![player, 7, 1]]);
        parse_gives(&mut e, false, &[]);
        start_log(&mut e);
        assert!(!e.call(0x005c_de40, &args![a]).bool());
        assert!(calls(&e, PLAYER_SET_FIELD_654).is_empty());
    }

    #[test]
    fn fn_005cdea0_calls_the_condition_function_and_succeeds() {
        check_condition_only(0x005c_dea0, GET_AMOUNT_STOLEN_SOLD_CONDITION, false);
    }

    #[test]
    fn fn_005cdec0_adds_the_parsed_amount_to_the_players_member() {
        let mut e = engine();
        let player = set_player(&mut e);
        e.mem.set_u32(player + 0x6dc, 10);
        let a = command(&mut e, 0);
        parse_gives(&mut e, true, &[5]);
        assert!(e.call(0x005c_dec0, &args![a]).bool());
        assert_eq!(e.mem.i32(player + 0x6dc), 15);
        parse_gives(&mut e, true, &[(-20i32) as u32]);
        assert!(e.call(0x005c_dec0, &args![a]).bool());
        assert_eq!(e.mem.i32(player + 0x6dc), -5);
        parse_gives(&mut e, false, &[]);
        assert!(!e.call(0x005c_dec0, &args![a]).bool());
        assert_eq!(e.mem.i32(player + 0x6dc), -5);
    }

    #[test]
    fn fn_005cdf20_adds_to_the_member_at_0x6dc() {
        let mut e = engine();
        let player = object(&mut e);
        e.call(0x005c_df20, &args![player, 3i32]);
        e.call(0x005c_df20, &args![player, -1i32]);
        assert_eq!(e.mem.i32(player + 0x6dc), 2);
    }

    #[test]
    fn get_pc_expelled_passes_the_parsed_faction_to_the_condition_function() {
        check_parse_then_condition(0x005c_df50, GET_PC_EXPELLED_CONDITION, false);
    }

    #[test]
    fn set_pc_expelled_sets_the_flag_and_looks_up_the_cell_owner() {
        let mut e = engine();
        let player = set_player(&mut e);
        let a = command(&mut e, 0);
        e.register(FACTION_SET_FLAG_BITS, |_, _| Ret::default());
        e.register(GET_PARENT_CELL, |_, _| 0x3000.into_ret());
        e.register(CELL_GET_OWNER, |_, _| Ret::default());
        // Flag set: the expelled bit set, the owner of the cell asked.
        parse_gives(&mut e, true, &[0x66, 1]);
        start_log(&mut e);
        assert!(e.call(0x005c_dfb0, &args![a]).bool());
        assert_eq!(calls(&e, FACTION_SET_FLAG_BITS), vec![vec![0x66, 8, 1]]);
        assert_eq!(calls(&e, GET_PARENT_CELL), vec![vec![player]]);
        assert_eq!(calls(&e, CELL_GET_OWNER), vec![vec![0x3000]]);
        // No parent cell: no owner lookup.
        e.register(GET_PARENT_CELL, |_, _| Ret::default());
        start_log(&mut e);
        assert!(e.call(0x005c_dfb0, &args![a]).bool());
        assert!(calls(&e, CELL_GET_OWNER).is_empty());
        // Flag clear: the bit cleared, no cell lookup.
        parse_gives(&mut e, true, &[0x66, 0]);
        start_log(&mut e);
        assert!(e.call(0x005c_dfb0, &args![a]).bool());
        assert_eq!(calls(&e, FACTION_SET_FLAG_BITS), vec![vec![0x66, 8, 0]]);
        assert!(calls(&e, GET_PARENT_CELL).is_empty());
        // Parameters that do not parse.
        parse_gives(&mut e, false, &[]);
        start_log(&mut e);
        assert!(!e.call(0x005c_dfb0, &args![a]).bool());
        assert!(calls(&e, FACTION_SET_FLAG_BITS).is_empty());
    }

    #[test]
    fn fn_005ce040_sets_the_expelled_bit_of_the_faction() {
        let mut e = engine();
        e.register(FACTION_SET_FLAG_BITS, |_, _| Ret::default());
        start_log(&mut e);
        e.call(0x005c_e040, &args![0x66u32, 1u8]);
        e.call(0x005c_e040, &args![0x66u32, 0u8]);
        assert_eq!(
            calls(&e, FACTION_SET_FLAG_BITS),
            vec![vec![0x66, 8, 1], vec![0x66, 8, 0]]
        );
    }

    #[test]
    fn get_pc_faction_murder_passes_the_parsed_faction_to_the_condition_function() {
        check_parse_then_condition(0x005c_e060, GET_PC_FACTION_MURDER_CONDITION, false);
    }

    /// The test of a faction flag setter command: a faction and a flag are
    /// parsed and the setter gets `(faction, flag != 0)`.
    fn check_faction_flag_setter(command_address: u32, setter: u32) {
        let mut e = engine();
        let a = command(&mut e, 0);
        e.register(setter, |_, _| Ret::default());
        parse_gives(&mut e, true, &[0x66, 9]);
        start_log(&mut e);
        assert!(e.call(command_address, &args![a]).bool());
        assert_parsed(&e, 0);
        assert_eq!(calls(&e, setter), vec![vec![0x66, 1]]);
        parse_gives(&mut e, true, &[0x66, 0]);
        start_log(&mut e);
        assert!(e.call(command_address, &args![a]).bool());
        assert_eq!(calls(&e, setter), vec![vec![0x66, 0]]);
        parse_gives(&mut e, false, &[]);
        start_log(&mut e);
        assert!(!e.call(command_address, &args![a]).bool());
        assert!(calls(&e, setter).is_empty());
    }

    #[test]
    fn fn_005ce0c0_sets_a_faction_flag() {
        check_faction_flag_setter(0x005c_e0c0, FACTION_SET_MURDER_FLAG);
    }

    #[test]
    fn get_player_enemyof_faction_passes_the_parsed_faction_to_the_condition_function() {
        check_parse_then_condition(0x005c_e130, GET_PLAYER_ENEMY_OF_FACTION_CONDITION, false);
    }

    #[test]
    fn fn_005ce190_sets_a_faction_flag() {
        check_faction_flag_setter(0x005c_e190, FACTION_SET_ENEMY_FLAG);
    }

    #[test]
    fn get_pc_faction_attack_passes_the_parsed_faction_to_the_condition_function() {
        check_parse_then_condition(0x005c_e200, GET_PC_FACTION_ATTACK_CONDITION, false);
    }

    #[test]
    fn fn_005ce260_sets_a_faction_flag() {
        check_faction_flag_setter(0x005c_e260, FACTION_SET_ATTACK_FLAG);
    }

    #[test]
    fn fn_005ce2d0_calls_the_condition_function_and_succeeds() {
        check_condition_only(0x005c_e2d0, GET_DESTROYED_CONDITION, false);
    }

    #[test]
    fn fn_005ce2f0_sets_form_flag_0x800000_on_the_reference() {
        let mut e = engine();
        let this_obj = object(&mut e);
        let a = command(&mut e, this_obj);
        e.register(FORM_SET_FLAG_800000, |_, _| Ret::default());
        parse_gives(&mut e, true, &[3]);
        start_log(&mut e);
        assert!(e.call(0x005c_e2f0, &args![a]).bool());
        assert_eq!(calls(&e, FORM_SET_FLAG_800000), vec![vec![this_obj, 1]]);
        parse_gives(&mut e, true, &[0]);
        start_log(&mut e);
        assert!(e.call(0x005c_e2f0, &args![a]).bool());
        assert_eq!(calls(&e, FORM_SET_FLAG_800000), vec![vec![this_obj, 0]]);
        // Without a reference: succeeds, nothing set.
        let none = command(&mut e, 0);
        start_log(&mut e);
        assert!(e.call(0x005c_e2f0, &args![none]).bool());
        assert!(calls(&e, FORM_SET_FLAG_800000).is_empty());
        parse_gives(&mut e, false, &[]);
        assert!(!e.call(0x005c_e2f0, &args![a]).bool());
    }

    /// A form with the given id at `+0x0c`.
    fn form_with_id(e: &mut Engine, id: u32) -> u32 {
        let form = object(e);
        e.mem.set_u32(form + 0x0c, id);
        form
    }

    #[test]
    fn get_action_ref_returns_the_id_of_the_action_reference() {
        let mut e = engine();
        let this_obj = object(&mut e);
        let a = command(&mut e, this_obj);
        // No action reference: 0.
        e.register(GET_ACTION_REF, |_, _| Ret::default());
        assert!(e.call(0x005c_e360, &args![a]).bool());
        assert_eq!(e.mem.f64(a.result.addr()), 0.0);
        // With one: its id, echoed when the flag is set.
        let action = form_with_id(&mut e, 0x1234);
        e.register_double(GET_ACTION_REF, move |_, _| action.into_ret());
        start_log(&mut e);
        assert!(e.call(0x005c_e360, &args![a]).bool());
        assert_eq!(e.mem.f64(a.result.addr()), 0x1234 as f64);
        assert!(calls(&e, CONSOLE_PRINT).is_empty());
        set_echo(&mut e, true);
        assert!(e.call(0x005c_e360, &args![a]).bool());
        assert_eq!(
            calls(&e, CONSOLE_PRINT),
            vec![vec![MSG_GET_ACTION_REF, 0x1234]]
        );
        // No reference: result 0, echo of 0, success.
        let none = command(&mut e, 0);
        e.mem.set_f64(none.result.addr(), 5.0);
        start_log(&mut e);
        assert!(e.call(0x005c_e360, &args![none]).bool());
        assert_eq!(e.mem.f64(none.result.addr()), 0.0);
        assert_eq!(calls(&e, CONSOLE_PRINT), vec![vec![MSG_GET_ACTION_REF, 0]]);
    }

    #[test]
    fn get_self_returns_the_id_unless_a_non_persistent_container_item() {
        let mut e = engine();
        let base = object(&mut e);
        let this_obj = form_with_id(&mut e, 0x4321);
        e.mem.set_u32(this_obj + 0x20, base);
        e.mem.set_u8(base + 4, 0x28);
        e.register(GET_REF_PERSISTS, |_, _| true.into_ret());
        e.register(GET_BASE_FORM, |e, a| e.mem.u32(a[0] + 0x20).into_ret());
        e.register(FORM_TYPE, |e, a| (e.mem.u8(a[0] + 4) as u32).into_ret());
        e.register(CONTAINER_CAN_HOLD_TYPE, |_, a| (a[0] == 0x28).into_ret());
        let a = command(&mut e, this_obj);
        // Persistent: always reported.
        assert!(e.call(0x005c_e3e0, &args![a]).bool());
        assert_eq!(e.mem.f64(a.result.addr()), 0x4321 as f64);
        // Not persistent and a container item type: stays 0.
        e.register(GET_REF_PERSISTS, |_, _| false.into_ret());
        start_log(&mut e);
        assert!(e.call(0x005c_e3e0, &args![a]).bool());
        assert_eq!(e.mem.f64(a.result.addr()), 0.0);
        assert_eq!(calls(&e, CONTAINER_CAN_HOLD_TYPE), vec![vec![0x28]]);
        // Not persistent, not a container item type: reported and echoed.
        e.mem.set_u8(base + 4, 0x10);
        set_echo(&mut e, true);
        assert!(e.call(0x005c_e3e0, &args![a]).bool());
        assert_eq!(e.mem.f64(a.result.addr()), 0x4321 as f64);
        assert_eq!(
            calls(&e, CONSOLE_PRINT).last().unwrap(),
            &vec![MSG_GET_SELF, 0x4321]
        );
        // No reference.
        let none = command(&mut e, 0);
        assert!(e.call(0x005c_e3e0, &args![none]).bool());
        assert_eq!(e.mem.f64(none.result.addr()), 0.0);
    }

    #[test]
    fn get_combat_target_returns_the_id_of_the_actors_target() {
        let mut e = engine();
        let target = form_with_id(&mut e, 0x99);
        let actor = object_with(&mut e, &[(ACTOR_GET_COMBAT_TARGET_SLOT, V_COMBAT_TARGET)]);
        e.register_double(V_COMBAT_TARGET, move |_, _| target.into_ret());
        let a = command(&mut e, actor);
        start_log(&mut e);
        set_echo(&mut e, true);
        assert!(e.call(0x005c_e480, &args![a]).bool());
        assert_eq!(e.mem.f64(a.result.addr()), 0x99 as f64);
        assert_eq!(
            calls(&e, CONSOLE_PRINT),
            vec![vec![MSG_GET_COMBAT_TARGET, 0x99]]
        );
        // No target: 0.
        e.register(V_COMBAT_TARGET, |_, _| Ret::default());
        assert!(e.call(0x005c_e480, &args![a]).bool());
        assert_eq!(e.mem.f64(a.result.addr()), 0.0);
        // Not an actor: 0, success.
        e.register(DYNAMIC_CAST, |_, _| Ret::default());
        e.mem.set_f64(a.result.addr(), 3.0);
        assert!(e.call(0x005c_e480, &args![a]).bool());
        assert_eq!(e.mem.f64(a.result.addr()), 0.0);
    }

    #[test]
    fn get_package_target_returns_the_id_of_the_current_package_target() {
        let mut e = engine();
        let target = form_with_id(&mut e, 0x55);
        let actor = object(&mut e);
        e.register_double(GET_CURRENT_PACKAGE_TARGET, move |_, _| target.into_ret());
        let a = command(&mut e, actor);
        start_log(&mut e);
        set_echo(&mut e, true);
        assert!(e.call(0x005c_e520, &args![a]).bool());
        assert_eq!(e.mem.f64(a.result.addr()), 0x55 as f64);
        assert_eq!(calls(&e, GET_CURRENT_PACKAGE_TARGET), vec![vec![actor]]);
        assert_eq!(
            calls(&e, CONSOLE_PRINT),
            vec![vec![MSG_GET_PACKAGE_TARGET, 0x55]]
        );
        e.register(GET_CURRENT_PACKAGE_TARGET, |_, _| Ret::default());
        assert!(e.call(0x005c_e520, &args![a]).bool());
        assert_eq!(e.mem.f64(a.result.addr()), 0.0);
        e.register(DYNAMIC_CAST, |_, _| Ret::default());
        assert!(e.call(0x005c_e520, &args![a]).bool());
        assert_eq!(e.mem.f64(a.result.addr()), 0.0);
    }

    #[test]
    fn get_container_returns_the_id_of_the_containing_object() {
        let mut e = engine();
        let this_obj = object(&mut e);
        let container = form_with_id(&mut e, 0x77);
        let a = ScriptArgs {
            containing_obj: Ptr::new(container),
            ..command(&mut e, this_obj)
        };
        start_log(&mut e);
        set_echo(&mut e, true);
        assert!(e.call(0x005c_e5c0, &args![a]).bool());
        assert_eq!(e.mem.f64(a.result.addr()), 0x77 as f64);
        assert_eq!(
            calls(&e, CONSOLE_PRINT),
            vec![vec![MSG_GET_CONTAINER, 0x77]]
        );
        // Without a containing object, or without a reference: 0.
        let a = command(&mut e, this_obj);
        e.mem.set_f64(a.result.addr(), 7.0);
        assert!(e.call(0x005c_e5c0, &args![a]).bool());
        assert_eq!(e.mem.f64(a.result.addr()), 0.0);
        let a = ScriptArgs {
            containing_obj: Ptr::new(container),
            ..command(&mut e, 0)
        };
        assert!(e.call(0x005c_e5c0, &args![a]).bool());
        assert_eq!(e.mem.f64(a.result.addr()), 0.0);
    }

    #[test]
    fn get_parent_ref_returns_the_id_of_the_parent_reference() {
        let mut e = engine();
        let parent = form_with_id(&mut e, 0x31);
        let this_obj = object(&mut e);
        e.register_double(GET_PARENT_REF, move |_, _| parent.into_ret());
        let a = command(&mut e, this_obj);
        start_log(&mut e);
        set_echo(&mut e, true);
        assert!(e.call(0x005c_e630, &args![a]).bool());
        assert_eq!(e.mem.f64(a.result.addr()), 0x31 as f64);
        assert_eq!(
            calls(&e, CONSOLE_PRINT),
            vec![vec![MSG_GET_PARENT_REF, 0x31]]
        );
        // The lookup is made twice, as the game does.
        assert_eq!(calls(&e, GET_PARENT_REF).len(), 2);
        e.register(GET_PARENT_REF, |_, _| Ret::default());
        assert!(e.call(0x005c_e630, &args![a]).bool());
        assert_eq!(e.mem.f64(a.result.addr()), 0.0);
        let none = command(&mut e, 0);
        assert!(e.call(0x005c_e630, &args![none]).bool());
        assert_eq!(e.mem.f64(none.result.addr()), 0.0);
    }

    #[test]
    fn get_linked_ref_returns_the_id_of_the_linked_reference() {
        let mut e = engine();
        let linked = form_with_id(&mut e, 0x41);
        let this_obj = object(&mut e);
        e.register_double(GET_LINKED_REF, move |_, _| linked.into_ret());
        let a = command(&mut e, this_obj);
        start_log(&mut e);
        set_echo(&mut e, true);
        assert!(e.call(0x005c_e6b0, &args![a]).bool());
        assert_eq!(e.mem.f64(a.result.addr()), 0x41 as f64);
        assert_eq!(
            calls(&e, CONSOLE_PRINT),
            vec![vec![MSG_GET_LINKED_REF, 0x41]]
        );
        e.register(GET_LINKED_REF, |_, _| Ret::default());
        assert!(e.call(0x005c_e6b0, &args![a]).bool());
        assert_eq!(e.mem.f64(a.result.addr()), 0.0);
        let none = command(&mut e, 0);
        assert!(e.call(0x005c_e6b0, &args![none]).bool());
        assert_eq!(e.mem.f64(none.result.addr()), 0.0);
    }

    #[test]
    fn get_force_run_reports_the_actors_flag() {
        let mut e = engine();
        let actor = object(&mut e);
        e.register(ACTOR_GET_FORCE_RUN, |_, _| true.into_ret());
        let a = command(&mut e, actor);
        start_log(&mut e);
        set_echo(&mut e, true);
        assert!(e.call(0x005c_e730, &args![a]).bool());
        assert_eq!(e.mem.f64(a.result.addr()), 1.0);
        // The echo passes the double as two words.
        assert_eq!(
            calls(&e, CONSOLE_PRINT),
            vec![vec![MSG_GET_FORCE_RUN, 0, 0x3ff0_0000]]
        );
        e.register(ACTOR_GET_FORCE_RUN, |_, _| false.into_ret());
        assert!(e.call(0x005c_e730, &args![a]).bool());
        assert_eq!(e.mem.f64(a.result.addr()), 0.0);
        // Not an actor: 0.
        e.register(DYNAMIC_CAST, |_, _| Ret::default());
        e.mem.set_f64(a.result.addr(), 4.0);
        set_echo(&mut e, false);
        assert!(e.call(0x005c_e730, &args![a]).bool());
        assert_eq!(e.mem.f64(a.result.addr()), 0.0);
    }

    #[test]
    fn set_force_run_stores_the_parsed_flag() {
        let mut e = engine();
        let actor = object(&mut e);
        e.register(ACTOR_SET_FORCE_RUN, |_, _| Ret::default());
        let a = command(&mut e, actor);
        parse_gives(&mut e, true, &[5]);
        start_log(&mut e);
        set_echo(&mut e, true);
        assert!(e.call(0x005c_e7b0, &args![a]).bool());
        assert_parsed(&e, actor);
        assert_eq!(e.mem.f64(a.result.addr()), 1.0);
        assert_eq!(calls(&e, ACTOR_SET_FORCE_RUN), vec![vec![actor, 1]]);
        // The raw integer goes to the echo.
        assert_eq!(calls(&e, CONSOLE_PRINT), vec![vec![MSG_SET_FORCE_RUN, 5]]);
        parse_gives(&mut e, true, &[0]);
        start_log(&mut e);
        set_echo(&mut e, false);
        assert!(e.call(0x005c_e7b0, &args![a]).bool());
        assert_eq!(calls(&e, ACTOR_SET_FORCE_RUN), vec![vec![actor, 0]]);
        assert!(calls(&e, CONSOLE_PRINT).is_empty());
        // Parameters that do not parse: false, nothing stored.
        parse_gives(&mut e, false, &[]);
        start_log(&mut e);
        assert!(!e.call(0x005c_e7b0, &args![a]).bool());
        assert!(calls(&e, ACTOR_SET_FORCE_RUN).is_empty());
        // Not an actor: success without parsing.
        e.register(DYNAMIC_CAST, |_, _| Ret::default());
        start_log(&mut e);
        assert!(e.call(0x005c_e7b0, &args![a]).bool());
        assert!(calls(&e, PARSE_PARAMETERS).is_empty());
    }

    #[test]
    fn get_force_sneak_reports_the_actors_flag() {
        let mut e = engine();
        let actor = object(&mut e);
        e.register(ACTOR_GET_FORCE_SNEAK, |_, _| true.into_ret());
        let a = command(&mut e, actor);
        start_log(&mut e);
        set_echo(&mut e, true);
        assert!(e.call(0x005c_e870, &args![a]).bool());
        assert_eq!(e.mem.f64(a.result.addr()), 1.0);
        assert_eq!(
            calls(&e, CONSOLE_PRINT),
            vec![vec![MSG_GET_FORCE_SNEAK, 0, 0x3ff0_0000]]
        );
        e.register(ACTOR_GET_FORCE_SNEAK, |_, _| false.into_ret());
        assert!(e.call(0x005c_e870, &args![a]).bool());
        assert_eq!(e.mem.f64(a.result.addr()), 0.0);
        e.register(DYNAMIC_CAST, |_, _| Ret::default());
        e.mem.set_f64(a.result.addr(), 4.0);
        assert!(e.call(0x005c_e870, &args![a]).bool());
        assert_eq!(e.mem.f64(a.result.addr()), 0.0);
    }

    #[test]
    fn set_force_sneak_stores_the_parsed_flag_in_the_actor() {
        let mut e = engine();
        let actor = object(&mut e);
        let a = command(&mut e, actor);
        parse_gives(&mut e, true, &[5]);
        start_log(&mut e);
        set_echo(&mut e, true);
        assert!(e.call(0x005c_e910, &args![a]).bool());
        assert_parsed(&e, actor);
        assert_eq!(e.mem.f64(a.result.addr()), 1.0);
        assert_eq!(e.mem.u8(actor + 0x125), 1);
        assert_eq!(calls(&e, CONSOLE_PRINT), vec![vec![MSG_SET_FORCE_SNEAK, 5]]);
        parse_gives(&mut e, true, &[0]);
        assert!(e.call(0x005c_e910, &args![a]).bool());
        assert_eq!(e.mem.u8(actor + 0x125), 0);
        parse_gives(&mut e, false, &[]);
        e.mem.set_u8(actor + 0x125, 1);
        assert!(!e.call(0x005c_e910, &args![a]).bool());
        assert_eq!(e.mem.u8(actor + 0x125), 1);
        // Not an actor: success without parsing.
        e.register(DYNAMIC_CAST, |_, _| Ret::default());
        start_log(&mut e);
        assert!(e.call(0x005c_e910, &args![a]).bool());
        assert!(calls(&e, PARSE_PARAMETERS).is_empty());
    }

    #[test]
    fn fn_005ce9d0_stores_the_byte_at_0x125() {
        let mut e = engine();
        let actor = object(&mut e);
        e.call(0x005c_e9d0, &args![actor, 1u8]);
        assert_eq!(e.mem.u8(actor + 0x125), 1);
        e.call(0x005c_e9d0, &args![actor, 0u8]);
        assert_eq!(e.mem.u8(actor + 0x125), 0);
    }

    #[test]
    fn fn_005ce9f0_raises_the_players_level_and_opens_the_level_up_menu() {
        let mut e = engine();
        // Without a player: nothing, but success.
        e.register(PLAYER_GET_LEVEL, |_, _| 0xffff_0004u32.into_ret());
        e.register(GET_BASE_FORM_OF_REFERENCE, |_, _| 0x6000.into_ret());
        e.register(ACTOR_BASE_DATA_SET_LEVEL, |_, _| Ret::default());
        e.register(CREATE_LEVEL_UP_MENU, |_, _| Ret::default());
        start_log(&mut e);
        assert!(e.call(0x005c_e9f0, &args![]).bool());
        assert!(calls(&e, CREATE_LEVEL_UP_MENU).is_empty());
        // With one: the 16-bit level plus one goes to the base data.
        let player = set_player(&mut e);
        assert!(e.call(0x005c_e9f0, &args![]).bool());
        assert_eq!(calls(&e, PLAYER_GET_LEVEL), vec![vec![player]]);
        assert_eq!(calls(&e, ACTOR_BASE_DATA_SET_LEVEL), vec![vec![0x6030, 5]]);
        assert_eq!(calls(&e, CREATE_LEVEL_UP_MENU).len(), 1);
    }

    #[test]
    fn has_magic_effect_passes_the_parsed_effect_to_the_condition_function() {
        check_parse_then_condition(0x005c_ea30, HAS_MAGIC_EFFECT_CONDITION, true);
    }

    #[test]
    fn is_spell_target_passes_the_parsed_spell_to_the_condition_function() {
        check_parse_then_condition(0x005c_ea90, IS_SPELL_TARGET_CONDITION, true);
    }

    #[test]
    fn get_spell_usage_number_passes_the_parsed_spell_to_the_condition_function() {
        check_parse_then_condition(0x005c_eaf0, GET_SPELL_USAGE_NUMBER_CONDITION, true);
    }

    #[test]
    fn fn_005ceb50_returns_the_condition_function_s_result() {
        check_condition_only(0x005c_eb50, GET_VATS_MODE_CONDITION, true);
    }

    #[test]
    fn fn_005ceb70_returns_the_condition_function_s_result() {
        check_condition_only(0x005c_eb70, GET_VATS_TARGET_HEIGHT_CONDITION, true);
    }
}
