//! `fallout shared/tesconditionfunctions.cpp` (Xbox PDB source unit), part 2: its functions from `005a2a50` up to
//! (not including) `ffffffff` in FalloutNV.exe 1.4.0.525, translated
//! (docs/ENGINE_CRATE.md). The unit's shared layouts and helpers are in
//! [`super::tesconditionfunctions`]; anything public there may be used here.
//!
//! Progress: this part holds `005a2a50` to `005a3c90` (the first session,
//! 40 functions); the next session continues with the first function after
//! `005a3c90` (`005a3d00`).
//!
//! The main file keeps its constants and helpers private, so this part
//! declares again the ones it needs. Every condition function has the shape
//! `bool f(TESObjectREFR* ref, void* param1, void* param2, double* result)`
//! (cdecl, four words), which are the parameters below; the value goes to
//! `*result` and the function returns `true`. The game computes `*result` in
//! extended precision; every value here is a whole number or 1.0 / 0.0 / 2.0,
//! so `f64` is exact.

#[allow(unused_imports)]
use super::tesconditionfunctions::*;
#[allow(unused_imports)]
use crate::prelude::*;

/// Offset in the TLS block of the byte that turns on the condition
/// functions' debug lines.
const TLS_TRACE_FLAG: u32 = 0x268;
/// Offset in the TLS block of the form `IsActorVictim` compares with.
const TLS_CRIME_VICTIM: u32 = 0x26c;
/// The debug print (`printf`-like, varargs, `interface.cpp`).
const DEBUG_PRINT: u32 = 0x0070_3c00;
/// `PlayerCharacter*` global.
const PLAYER: u32 = 0x011d_ea3c;
/// The `Calendar` singleton.
const CALENDAR: u32 = 0x011d_e7b8;

/// Virtual `IsActor` (vtable `+0x100`, Xbox PDB name).
const VSLOT_IS_ACTOR: u32 = 0x100;
/// Virtual (vtable `+0x218`) on an actor: a byte test (`IsActorVictim`
/// requires it).
const VSLOT_ACTOR_TEST_218: u32 = 0x218;
/// Virtual (vtable `+0x22c`) on an actor, called with 0: a byte test.
const VSLOT_ACTOR_TEST_22C: u32 = 0x22c;
/// Virtual (vtable `+0x60`) on the actor's middle-high process: a byte test.
const VSLOT_PROCESS_TEST_60: u32 = 0x60;
/// Virtual (vtable `+0x69c`) on the actor's middle-high process: a `float`
/// in ST0.
const VSLOT_PROCESS_FLOAT_69C: u32 = 0x69c;

/// `TESForm::cFormType` (byte at `+0x04`) of the form in ECX.
const FORM_GET_TYPE: u32 = 0x0040_1170;
/// The form type `fn_005a3b90` requires of a reference's base form.
const FORM_TYPE_SPEAKER_SOURCE: u32 = 0x16;
/// The form type `GetArmorRatingUpper` requires of the worn item's form.
const FORM_TYPE_ARMOR: u32 = 0x18;
/// Reads `[this + 0x20]` (a reference's base form).
const REF_GET_BASE_FORM: u32 = 0x007a_f430;
/// Reads `[this + 0x08]` of the object in ECX (the worn `ItemChange`'s item).
const OBJECT_GET_FIELD_08: u32 = 0x0044_ddc0;
/// The name of a form (cdecl, one form argument; a fixed string for null).
const FORM_GET_FULL_NAME: u32 = 0x0048_2720;
/// The name of a reference (`tesobjectrefr.cpp`, `this` the reference).
const REF_GET_NAME: u32 = 0x0055_d520;
/// `this + 0x44`: the reference's `ExtraDataList`.
const REF_GET_EXTRA_DATA_LIST: u32 = 0x005d_43c0;
/// `ExtraDataList` owner getter (`extradatalist.cpp`): the owner form or null.
const EXTRA_LIST_GET_OWNER: u32 = 0x0041_8660;
/// `ExtraDataList::GetFriendHitCount` (Xbox PDB).
const EXTRA_LIST_GET_FRIEND_HIT_COUNT: u32 = 0x0042_2640;
/// `[this + 0x40]` of a reference: its parent cell.
const REF_GET_PARENT_CELL: u32 = 0x008d_6f30;
/// `TESObjectCELL::GetOwner` (Xbox PDB).
const CELL_GET_OWNER: u32 = 0x0054_6a40;
/// True when the cell in ECX is an interior (`extradatalist.cpp`).
const CELL_IS_INTERIOR: u32 = 0x0042_5fd0;
/// `TESObjectCELL::GetWaterType` (Xbox PDB).
const CELL_GET_WATER_TYPE: u32 = 0x0054_7770;
/// `TESWaterForm::GetDangerous` (Xbox PDB).
const WATER_FORM_GET_DANGEROUS: u32 = 0x0058_0060;

/// The actor's middle-high process, or null (the engine map names this body
/// `MiddleHighProcess::GetSavedAcquireObject` because of folded code).
const ACTOR_GET_PROCESS: u32 = 0x008d_8520;
/// `actor.cpp`: the result of virtual `+0x770` of the actor's process, or
/// null without a process.
const ACTOR_GET_PROCESS_PART: u32 = 0x008a_5270;
/// Byte tests of an `Actor` (`actor.cpp`): ghost, offers services, and the
/// one behind `fn_005a35f0`.
const ACTOR_IS_GHOST: u32 = 0x008a_ce90;
const ACTOR_OFFERS_SERVICES: u32 = 0x008a_c680;
const ACTOR_TEST_87F510: u32 = 0x0087_f510;
/// `Actor::GetBarterGoldBase` (Xbox PDB).
const ACTOR_GET_BARTER_GOLD_BASE: u32 = 0x0088_4320;
/// `Actor::IsRunning` (Xbox PDB).
const ACTOR_IS_RUNNING: u32 = 0x0088_4730;
/// Byte test of the actor in ECX (`animation.cpp`): movement flag `0x400`
/// set and `0x800` clear (sneaking).
const ACTOR_IS_SNEAKING: u32 = 0x0049_97b0;
/// Byte at `+0x104` of the actor (`animation.cpp`): behind `IsInCombat`.
const ACTOR_IN_COMBAT_FLAG: u32 = 0x0049_3bb0;
/// `TESObjectREFR::IsPartofEvilFaction` (Xbox PDB).
const REF_IS_PART_OF_EVIL_FACTION: u32 = 0x0056_78a0;
/// `Actor::GetEssential` (Xbox PDB).
const ACTOR_GET_ESSENTIAL: u32 = 0x0087_f3d0;
/// A whole-number value of the calendar (`this` the [`CALENDAR`]; its global
/// at `+0x10` times 24, truncated), the minuend of `fn_005a3a30`.
const CALENDAR_GET_WHOLE_VALUE: u32 = 0x0086_7e30;
/// The table of pointers `fn_005a3370` indexes; the value is at `+4` of the
/// entry.
const MISC_STAT_TABLE: u32 = 0x011c_6d50;
/// The `int` global `fn_005a3650` returns.
const INT_GLOBAL_5A3650: u32 = 0x0119_f344;

/// `InventoryChanges::GetInventoryChanges` (Xbox PDB), cdecl, one reference.
const GET_INVENTORY_CHANGES: u32 = 0x004b_f220;
/// `InventoryChanges::GetWornItem` (Xbox PDB): `this` the changes and (slot,
/// 0); the worn `ItemChange` or null.
const INVENTORY_GET_WORN_ITEM: u32 = 0x004c_8c10;
/// Destructor with a delete flag: `this` the `ItemChange`, flag 1.
const ITEM_CHANGE_DESTROY: u32 = 0x0044_59e0;
/// Armor test (`0x00514410`): `2` for an armor with a flag set, else 0.
const ARMOR_GET_CLASS_FLAG: u32 = 0x0051_4410;
/// `[this + 0x90]` of the base form `fn_005a3b90` tests
/// (`tesobjectcont.cpp`).
const FORM_GET_FIELD_90: u32 = 0x0051_6bf0;

/// `PlayerCharacter` functions (`this` the player): sleeping or resting,
/// `IsPlayerCharacterInCombat` (Xbox PDB, one out byte), `fn_005a3af0`'s
/// value, the test behind `IsActionInList`, and the one behind
/// `fn_005a3bf0`.
const PLAYER_IS_SLEEPING_OR_RESTING: u32 = 0x0094_df60;
const PLAYER_IS_IN_COMBAT: u32 = 0x0095_3c50;
const PLAYER_GET_VALUE_964060: u32 = 0x0096_4060;
const PLAYER_IS_ACTION_ACTIVE: u32 = 0x0096_3ce0;
const PLAYER_TEST_966C10: u32 = 0x0096_6c10;
/// The platform test behind `IsXbox` and `IsPC` (`interface.cpp`).
const PLATFORM_IS_CONSOLE: u32 = 0x0070_9d40;
/// `[this + 0x58] & mask` of the process part (`fn_005a3c90`'s test).
const PROCESS_PART_TEST_FLAGS: u32 = 0x0058_cba0;

/// `-1.0`, `0.0` and `2.0` (`double`s in the exe's data).
const DOUBLE_MINUS_ONE: u32 = 0x0101_a6b0;
const DOUBLE_ZERO: u32 = 0x0101_2060;
const DOUBLE_TWO: u32 = 0x0101_1590;

/// The globals `fn_005a38e0` clears: the caches of other condition
/// functions of the unit (`GetDisposition`, `GetInCell`, `GetInFaction`,
/// `GetIsID`, `GetIsVoiceType`, `GetIsPlayableRace`).
const CACHE_WORDS: [u32; 15] = [
    0x011c_aae8,
    0x011c_aaec,
    0x011c_aaf0,
    0x011c_aaf4,
    0x011c_aaf8,
    0x011c_aafc,
    0x011c_ab00,
    0x011c_ab04,
    0x011c_ab08,
    0x011c_ab0c,
    0x011c_ab10,
    0x011c_ab14,
    0x011c_ab18,
    0x011c_ab1c,
    0x011c_ab20,
];

/// The byte at `TLS + 0x268`: the condition functions print a debug line
/// for their result when it is set.
fn trace_enabled(e: &mut Engine) -> bool {
    let tls = e.tls();
    e.mem.u8(tls + TLS_TRACE_FLAG) != 0
}

fn set_result(e: &mut Engine, result: Ptr, value: f64) {
    e.mem.set_f64(result.addr(), value);
}

/// With the debug lines on, the format the result selects (`zero` for a
/// result equal to 0.0, else `nonzero`); `None` with them off.
fn trace_choice(e: &mut Engine, result: Ptr, zero: u32, nonzero: u32) -> Option<u32> {
    if !trace_enabled(e) {
        return None;
    }
    Some(if e.mem.f64(result.addr()) == 0.0 {
        zero
    } else {
        nonzero
    })
}

/// The debug line `format(*result)`.
fn trace_result(e: &mut Engine, format: u32, result: Ptr) {
    let value = e.mem.f64(result.addr());
    e.call(DEBUG_PRINT, &args![format, value]);
}

/// The name of a form through the cdecl name function.
fn form_name(e: &mut Engine, form: Ptr) -> u32 {
    e.call(FORM_GET_FULL_NAME, &args![form]).u32()
}

/// `ref` when it is non-null and virtual `IsActor` says so, else null.
fn actor_of(e: &mut Engine, reference: Ptr) -> Ptr {
    if !reference.is_null() && e.vcall(reference.addr(), VSLOT_IS_ACTOR, &args![]).bool() {
        reference
    } else {
        Ptr::NULL
    }
}

// Translated from 005a2a50 (decompiled, FalloutNV.exe 1.4.0.525)
/// Condition function without Xbox PDB name: always 0.0, no debug line.
pub fn fn_005a2a50(
    e: &mut Engine,
    _reference: u32,
    _param1: u32,
    _param2: u32,
    result: Ptr,
) -> bool {
    set_result(e, result, 0.0);
    true
}

// Translated from 005a2a60 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Script::GetGhostConditionFunction` (Xbox PDB): 1.0 when the reference is
/// an actor and `008ace90` says it is a ghost.
pub fn script_get_ghost_condition_function(
    e: &mut Engine,
    reference: Ptr,
    _param1: u32,
    _param2: u32,
    result: Ptr,
) -> bool {
    set_result(e, result, 0.0);
    let actor = actor_of(e, reference);
    if !actor.is_null() && e.call(ACTOR_IS_GHOST, &args![actor]).bool() {
        set_result(e, result, 1.0);
    }
    if trace_enabled(e) {
        // "GetGhost >> %0.2f"
        trace_result(e, 0x0103_5bfc, result);
    }
    true
}

// Translated from 005a2af0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Script::GetOffersServiceConditionFunction` (Xbox PDB): 1.0 when the
/// reference is an actor and `008ac680` says it offers services.
pub fn script_get_offers_service_condition_function(
    e: &mut Engine,
    reference: Ptr,
    _param1: u32,
    _param2: u32,
    result: Ptr,
) -> bool {
    set_result(e, result, 0.0);
    let actor = actor_of(e, reference);
    if !actor.is_null() && e.call(ACTOR_OFFERS_SERVICES, &args![actor]).bool() {
        set_result(e, result, 1.0);
    }
    // "%s is not offering services" / "%s  is offering services"
    if let Some(format) = trace_choice(e, result, 0x0103_5c10, 0x0103_5c2c) {
        let name = e.call(REF_GET_NAME, &args![actor]).u32();
        e.call(DEBUG_PRINT, &args![format, name]);
    }
    true
}

// Translated from 005a2ba0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Script::GetBarterGoldConditionFunction` (Xbox PDB): the actor's base
/// barter gold. Leaves the result alone for a reference that is not an
/// actor.
pub fn script_get_barter_gold_condition_function(
    e: &mut Engine,
    reference: Ptr,
    _param1: u32,
    _param2: u32,
    result: Ptr,
) -> bool {
    let actor = actor_of(e, reference);
    if !actor.is_null() {
        let gold = e.call(ACTOR_GET_BARTER_GOLD_BASE, &args![actor]).u32();
        set_result(e, result, gold as f64);
        if trace_enabled(e) {
            let gold = e.call(ACTOR_GET_BARTER_GOLD_BASE, &args![actor]).u32();
            // "%s  has %d barter gold currently"
            e.call(DEBUG_PRINT, &args![0x0103_5c48u32, actor, gold]);
        }
    }
    true
}

// Translated from 005a2c30 (decompiled, FalloutNV.exe 1.4.0.525)
/// Condition function without Xbox PDB name: always 0.0 (it asks the
/// reference whether it is an actor and does nothing with the answer).
pub fn fn_005a2c30(
    e: &mut Engine,
    reference: Ptr,
    _param1: u32,
    _param2: u32,
    result: Ptr,
) -> bool {
    actor_of(e, reference);
    set_result(e, result, 0.0);
    true
}

// Translated from 005a2c70 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Script::IsTimePassingConditionFunction` (Xbox PDB): 1.0 when the player
/// is sleeping or resting. Does not clear the result first.
pub fn script_is_time_passing_condition_function(
    e: &mut Engine,
    _reference: u32,
    _param1: u32,
    _param2: u32,
    result: Ptr,
) -> bool {
    let player = e.global::<u32>(PLAYER);
    if e.call(PLAYER_IS_SLEEPING_OR_RESTING, &args![player]).bool() {
        set_result(e, result, 1.0);
    }
    // "Time is not passing" / "Time is passing"
    if let Some(format) = trace_choice(e, result, 0x0103_5c6c, 0x0103_5c80) {
        e.call(DEBUG_PRINT, &args![format]);
    }
    true
}

// Translated from 005a2ce0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Script::GetArmorRatingUpperConditionFunction` (Xbox PDB): for the item
/// worn in slot 2 (upper body) whose form is of the armor type: 1.0, or 2.0
/// when the armor test `00514410` answers non-zero; 0.0 otherwise. The
/// `ItemChange` obtained from the worn-item lookup is destroyed (flag 1).
pub fn script_get_armor_rating_upper_condition_function(
    e: &mut Engine,
    reference: Ptr,
    _param1: u32,
    _param2: u32,
    result: Ptr,
) -> bool {
    let actor = actor_of(e, reference);
    set_result(e, result, 0.0);
    if !actor.is_null() {
        let changes = e.call(GET_INVENTORY_CHANGES, &args![reference]).ptr::<()>();
        if !changes.is_null() {
            let worn = e
                .call(INVENTORY_GET_WORN_ITEM, &args![changes, 2u32, 0u32])
                .ptr::<()>();
            if !worn.is_null() {
                let mut armor = Ptr::<()>::NULL;
                if !e
                    .call(OBJECT_GET_FIELD_08, &args![worn])
                    .ptr::<()>()
                    .is_null()
                {
                    let form = e.call(OBJECT_GET_FIELD_08, &args![worn]).ptr::<()>();
                    if e.call(FORM_GET_TYPE, &args![form]).u32() == FORM_TYPE_ARMOR {
                        armor = e.call(OBJECT_GET_FIELD_08, &args![worn]).ptr::<()>();
                    }
                }
                if !armor.is_null() {
                    if e.call(ARMOR_GET_CLASS_FLAG, &args![armor]).bool() {
                        let two: f64 = e.global(DOUBLE_TWO);
                        set_result(e, result, two);
                    } else {
                        set_result(e, result, 1.0);
                    }
                }
                e.call(ITEM_CHANGE_DESTROY, &args![worn, 1u32]);
            }
        }
    }
    if trace_enabled(e) {
        // "Armor Rating upper body is %0.2f"
        trace_result(e, 0x0103_5c90, result);
    }
    true
}

// Translated from 005a2e20 (decompiled, FalloutNV.exe 1.4.0.525)
/// Condition function without Xbox PDB name, debug lines "is the owner":
/// 1.0 when the owner of the reference's extra data list is the form
/// parameter (the base form of the player when the parameter is null). Does
/// not clear the result first.
pub fn fn_005a2e20(e: &mut Engine, reference: Ptr, form: Ptr, _param2: u32, result: Ptr) -> bool {
    let mut owner = form;
    if owner.is_null() {
        let player = e.global::<u32>(PLAYER);
        owner = e.call(REF_GET_BASE_FORM, &args![player]).ptr();
    }
    if !reference.is_null() && !owner.is_null() {
        let list = e.call(REF_GET_EXTRA_DATA_LIST, &args![reference]).u32();
        if e.call(EXTRA_LIST_GET_OWNER, &args![list]).ptr::<()>() == owner {
            set_result(e, result, 1.0);
        }
    }
    // "%s is not the owner" / "%s is the owner"
    if let Some(format) = trace_choice(e, result, 0x0103_5cb4, 0x0103_5cc8) {
        let name = form_name(e, owner);
        e.call(DEBUG_PRINT, &args![format, name]);
    }
    true
}

// Translated from 005a2ed0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Script::GetCellOwnershipConditionFunction` (Xbox PDB): 1.0 when the
/// owner of the cell (the first parameter) is the form (the second
/// parameter). Does not clear the result first.
pub fn script_get_cell_ownership_condition_function(
    e: &mut Engine,
    _reference: u32,
    cell: Ptr,
    owner: Ptr,
    result: Ptr,
) -> bool {
    if e.call(CELL_GET_OWNER, &args![cell]).ptr::<()>() == owner {
        set_result(e, result, 1.0);
    }
    // "%s is not the owner" / "%s is the owner"
    if let Some(format) = trace_choice(e, result, 0x0103_5cb4, 0x0103_5cc8) {
        let name = form_name(e, owner);
        e.call(DEBUG_PRINT, &args![format, name]);
    }
    true
}

// Translated from 005a2f60 (decompiled, FalloutNV.exe 1.4.0.525)
/// Condition function without Xbox PDB name, debug lines "is sneaking": 1.0
/// when `004997b0` says the actor is sneaking. Asks it even for a null actor.
pub fn fn_005a2f60(
    e: &mut Engine,
    reference: Ptr,
    _param1: u32,
    _param2: u32,
    result: Ptr,
) -> bool {
    set_result(e, result, 0.0);
    let actor = actor_of(e, reference);
    if e.call(ACTOR_IS_SNEAKING, &args![actor]).bool() {
        set_result(e, result, 1.0);
    }
    // "%s is not sneaking" / "%s is sneaking"
    if let Some(format) = trace_choice(e, result, 0x0103_5cd8, 0x0103_5cec) {
        let name = form_name(e, actor);
        e.call(DEBUG_PRINT, &args![format, name]);
    }
    true
}

// Translated from 005a3010 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Script::IsRunningConditionFunction` (Xbox PDB): 1.0 when
/// `Actor::IsRunning` (Xbox PDB) says so. Asks it even for a null actor.
pub fn script_is_running_condition_function(
    e: &mut Engine,
    reference: Ptr,
    _param1: u32,
    _param2: u32,
    result: Ptr,
) -> bool {
    set_result(e, result, 0.0);
    let actor = actor_of(e, reference);
    if e.call(ACTOR_IS_RUNNING, &args![actor]).bool() {
        set_result(e, result, 1.0);
    }
    // "%s is not running" / "%s is running"
    if let Some(format) = trace_choice(e, result, 0x0103_5cfc, 0x0103_5d10) {
        let name = form_name(e, actor);
        e.call(DEBUG_PRINT, &args![format, name]);
    }
    true
}

// Translated from 005a30c0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Script::GetFriendHitConditionFunction` (Xbox PDB): the friend-hit count
/// of the actor's extra data list; 0.0 and no debug line for a reference
/// that is not an actor. The debug line names the player and the actor.
pub fn script_get_friend_hit_condition_function(
    e: &mut Engine,
    reference: Ptr,
    _param1: u32,
    _param2: u32,
    result: Ptr,
) -> bool {
    set_result(e, result, 0.0);
    let actor = actor_of(e, reference);
    if actor.is_null() {
        return true;
    }
    let list = e.call(REF_GET_EXTRA_DATA_LIST, &args![actor]).u32();
    let count = e.call(EXTRA_LIST_GET_FRIEND_HIT_COUNT, &args![list]).i32();
    set_result(e, result, count as f64);
    if trace_enabled(e) {
        let value = e.mem.f64(result.addr());
        let actor_name = form_name(e, actor);
        let player = Ptr::<()>::new(e.global::<u32>(PLAYER));
        let player_name = form_name(e, player);
        // "%s has hit %s %0.2f times"
        e.call(
            DEBUG_PRINT,
            &args![0x0103_5d20u32, player_name, actor_name, value],
        );
    }
    true
}

// Translated from 005a3180 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Script::IsInCombatConditionFunction` (Xbox PDB): 1.0 when the byte test
/// `00493bb0` says the actor is in combat; for the player the value is
/// instead `PlayerCharacter::IsPlayerCharacterInCombat` (Xbox PDB), called
/// with a local byte that starts as 0.
pub fn script_is_in_combat_condition_function(
    e: &mut Engine,
    reference: Ptr,
    _param1: u32,
    _param2: u32,
    result: Ptr,
) -> bool {
    set_result(e, result, 0.0);
    let actor = actor_of(e, reference);
    if !actor.is_null() && e.call(ACTOR_IN_COMBAT_FLAG, &args![actor]).bool() {
        set_result(e, result, 1.0);
    }
    let player = e.global::<u32>(PLAYER);
    if actor.addr() == player {
        let in_combat = e.with_stack(1, |e, out| {
            e.call(PLAYER_IS_IN_COMBAT, &args![player, out]).u32() & 0xff
        });
        set_result(e, result, in_combat as f64);
    }
    // "%s is not in combat" / "%s is in combat"
    if let Some(format) = trace_choice(e, result, 0x0103_5d3c, 0x0103_5d50) {
        let name = form_name(e, actor);
        e.call(DEBUG_PRINT, &args![format, name]);
    }
    true
}

// Translated from 005a3270 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Script::IsInInteriorConditionFunction` (Xbox PDB): 1.0 when the
/// reference has a parent cell and it is an interior.
pub fn script_is_in_interior_condition_function(
    e: &mut Engine,
    reference: Ptr,
    _param1: u32,
    _param2: u32,
    result: Ptr,
) -> bool {
    set_result(e, result, 0.0);
    if !reference.is_null()
        && !e
            .call(REF_GET_PARENT_CELL, &args![reference])
            .ptr::<()>()
            .is_null()
    {
        let cell = e.call(REF_GET_PARENT_CELL, &args![reference]).u32();
        if e.call(CELL_IS_INTERIOR, &args![cell]).bool() {
            set_result(e, result, 1.0);
        }
    }
    // "%s is not an Interior" / "%s is in an Interior"
    if let Some(format) = trace_choice(e, result, 0x0103_5d60, 0x0103_5d78) {
        let name = form_name(e, reference);
        e.call(DEBUG_PRINT, &args![format, name]);
    }
    true
}

// Translated from 005a3310 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Script::GetPCMiscStatConditionFunction` (Xbox PDB): the player's misc
/// stat whose index is the first parameter ([`fn_005a3370`]).
pub fn script_get_pc_misc_stat_condition_function(
    e: &mut Engine,
    _reference: u32,
    index: u32,
    _param2: u32,
    result: Ptr,
) -> bool {
    let value = fn_005a3370(e, index) as i32;
    set_result(e, result, value as f64);
    if trace_enabled(e) {
        // "Player misc stat value %.02f"
        trace_result(e, 0x0103_5d90, result);
    }
    true
}

// Translated from 005a3370 (decompiled, FalloutNV.exe 1.4.0.525)
/// The value (`+4`) of entry `index` of the table of misc stats at
/// `0x011c6d50` (a table of pointers); cdecl, one word.
pub fn fn_005a3370(e: &mut Engine, index: u32) -> u32 {
    let entry = e.mem.u32(MISC_STAT_TABLE + index.wrapping_mul(4));
    e.mem.u32(entry + 4)
}

// Translated from 005a3390 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Script::IsActorEvilConditionFunction` (Xbox PDB): 1.0 when
/// `TESObjectREFR::IsPartofEvilFaction` (Xbox PDB) says the actor is. A
/// reference that is not an actor gets 0.0 and no debug line.
pub fn script_is_actor_evil_condition_function(
    e: &mut Engine,
    reference: Ptr,
    _param1: u32,
    _param2: u32,
    result: Ptr,
) -> bool {
    let actor = actor_of(e, reference);
    set_result(e, result, 0.0);
    if !actor.is_null() {
        if e.call(REF_IS_PART_OF_EVIL_FACTION, &args![actor]).bool() {
            set_result(e, result, 1.0);
        }
        // "%s  is not evil " / "%s  is evil "
        if let Some(format) = trace_choice(e, result, 0x0103_5db0, 0x0103_5dc4) {
            let name = e.call(REF_GET_NAME, &args![actor]).u32();
            e.call(DEBUG_PRINT, &args![format, name]);
        }
    }
    true
}

// Translated from 005a3460 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Script::IsActorVictimConditionFunction` (Xbox PDB): for an actor whose
/// virtual `+0x218` answers yes, 1.0 when its base form is non-null and is
/// the form kept in the TLS block at `+0x26c` (the current crime victim).
pub fn script_is_actor_victim_condition_function(
    e: &mut Engine,
    reference: Ptr,
    _param1: u32,
    _param2: u32,
    result: Ptr,
) -> bool {
    let actor = actor_of(e, reference);
    set_result(e, result, 0.0);
    if !actor.is_null() && e.vcall(actor.addr(), VSLOT_ACTOR_TEST_218, &args![]).bool() {
        let base = e.call(REF_GET_BASE_FORM, &args![actor]).u32();
        let tls = e.tls();
        if base != 0 && base == e.mem.u32(tls + TLS_CRIME_VICTIM) {
            set_result(e, result, 1.0);
        }
        // "%s  is not a crime victim " / "%s  crime victim "
        if let Some(format) = trace_choice(e, result, 0x0103_5dd4, 0x0103_5df0) {
            let name = e.call(REF_GET_NAME, &args![actor]).u32();
            e.call(DEBUG_PRINT, &args![format, name]);
        }
    }
    true
}

// Translated from 005a3570 (decompiled, FalloutNV.exe 1.4.0.525)
/// Condition function without Xbox PDB name: for an actor with a
/// middle-high process, 1.0 when the process's virtual `+0x60` answers yes.
pub fn fn_005a3570(
    e: &mut Engine,
    reference: Ptr,
    _param1: u32,
    _param2: u32,
    result: Ptr,
) -> bool {
    set_result(e, result, 0.0);
    let actor = actor_of(e, reference);
    if !actor.is_null()
        && !e
            .call(ACTOR_GET_PROCESS, &args![actor])
            .ptr::<()>()
            .is_null()
    {
        let process = e.call(ACTOR_GET_PROCESS, &args![actor]).u32();
        if e.vcall(process, VSLOT_PROCESS_TEST_60, &args![]).bool() {
            set_result(e, result, 1.0);
        }
    }
    true
}

// Translated from 005a35f0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Condition function without Xbox PDB name: for an actor, 1.0 when the byte
/// test `0087f510` answers yes.
pub fn fn_005a35f0(
    e: &mut Engine,
    reference: Ptr,
    _param1: u32,
    _param2: u32,
    result: Ptr,
) -> bool {
    set_result(e, result, 0.0);
    let actor = actor_of(e, reference);
    if !actor.is_null() && e.call(ACTOR_TEST_87F510, &args![actor]).bool() {
        set_result(e, result, 1.0);
    }
    true
}

// Translated from 005a3650 (decompiled, FalloutNV.exe 1.4.0.525)
/// Condition function without Xbox PDB name: the `int` global at
/// `0x0119f344`. (The code first stores -1.0 and overwrites it at once.)
pub fn fn_005a3650(
    e: &mut Engine,
    _reference: u32,
    _param1: u32,
    _param2: u32,
    result: Ptr,
) -> bool {
    let value = e.global::<u32>(INT_GLOBAL_5A3650) as i32;
    set_result(e, result, value as f64);
    true
}

// Translated from 005a3670 (decompiled, FalloutNV.exe 1.4.0.525)
/// Condition function without Xbox PDB name: always 0.0 (it asks a
/// non-null reference whether it is an actor and does nothing with the
/// answer).
pub fn fn_005a3670(
    e: &mut Engine,
    reference: Ptr,
    _param1: u32,
    _param2: u32,
    result: Ptr,
) -> bool {
    set_result(e, result, 0.0);
    if !reference.is_null() {
        actor_of(e, reference);
    }
    true
}

// Translated from 005a36b0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Script::IsInDangerousWaterConditionFunction` (Xbox PDB): for an actor
/// that has the flag byte `fn_005a3740` reads, standing in a cell with a
/// water type whose form says it is dangerous: 1.0.
pub fn script_is_in_dangerous_water_condition_function(
    e: &mut Engine,
    reference: Ptr,
    _param1: u32,
    _param2: u32,
    result: Ptr,
) -> bool {
    set_result(e, result, 0.0);
    let actor = actor_of(e, reference);
    if !actor.is_null()
        && fn_005a3740(e, actor) != 0
        && !e
            .call(REF_GET_PARENT_CELL, &args![actor])
            .ptr::<()>()
            .is_null()
    {
        let cell = e.call(REF_GET_PARENT_CELL, &args![actor]).u32();
        if !e
            .call(CELL_GET_WATER_TYPE, &args![cell])
            .ptr::<()>()
            .is_null()
        {
            let cell = e.call(REF_GET_PARENT_CELL, &args![actor]).u32();
            let water = e.call(CELL_GET_WATER_TYPE, &args![cell]).u32();
            if e.call(WATER_FORM_GET_DANGEROUS, &args![water]).bool() {
                set_result(e, result, 1.0);
            }
        }
    }
    true
}

// Translated from 005a3740 (decompiled, FalloutNV.exe 1.4.0.525)
/// The byte at `+0x14c` of the actor.
pub fn fn_005a3740(e: &mut Engine, this: Ptr) -> u8 {
    e.mem.u8(this.addr() + 0x14c)
}

// Translated from 005a3760 (decompiled, FalloutNV.exe 1.4.0.525)
/// Condition function without Xbox PDB name: 1.0 when [`fn_005a3790`] says
/// the reference has the flag. Does not check for a null reference.
pub fn fn_005a3760(
    e: &mut Engine,
    reference: Ptr,
    _param1: u32,
    _param2: u32,
    result: Ptr,
) -> bool {
    set_result(e, result, 0.0);
    if fn_005a3790(e, reference) {
        set_result(e, result, 1.0);
    }
    true
}

// Translated from 005a3790 (decompiled, FalloutNV.exe 1.4.0.525)
/// True when bit `0x00100000` of the form flags (`this + 0x08`) is set.
pub fn fn_005a3790(e: &mut Engine, this: Ptr) -> bool {
    e.mem.u32(this.addr() + 0x08) & 0x0010_0000 != 0
}

// Translated from 005a37b0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Script::IsXboxConditionFunction` (Xbox PDB): 1.0 when `00709d40` (a
/// platform test with no arguments) says yes, else 0.0.
pub fn script_is_xbox_condition_function(
    e: &mut Engine,
    _reference: u32,
    _param1: u32,
    _param2: u32,
    result: Ptr,
) -> bool {
    set_result(e, result, 0.0);
    let console = e.call(PLATFORM_IS_CONSOLE, &args![]).bool();
    set_result(e, result, if console { 1.0 } else { 0.0 });
    if trace_enabled(e) {
        // "IsXBox >> %1.0f"
        trace_result(e, 0x0103_5e04, result);
    }
    true
}

// Translated from 005a3820 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Script::IsPS3ConditionFunction` (Xbox PDB): always 0.0.
pub fn script_is_ps3_condition_function(
    e: &mut Engine,
    _reference: u32,
    _param1: u32,
    _param2: u32,
    result: Ptr,
) -> bool {
    set_result(e, result, 0.0);
    if trace_enabled(e) {
        // "IsPS3 >> %1.0f"
        trace_result(e, 0x0103_5e14, result);
    }
    true
}

// Translated from 005a3870 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Script::IsPCConditionFunction` (Xbox PDB): 0.0 when `00709d40` says
/// yes, else 1.0 (the opposite of `IsXbox`).
pub fn script_is_pc_condition_function(
    e: &mut Engine,
    _reference: u32,
    _param1: u32,
    _param2: u32,
    result: Ptr,
) -> bool {
    set_result(e, result, 1.0);
    let console = e.call(PLATFORM_IS_CONSOLE, &args![]).bool();
    set_result(e, result, if console { 0.0 } else { 1.0 });
    if trace_enabled(e) {
        // "IsPC >> %1.0f"
        trace_result(e, 0x0103_5e24, result);
    }
    true
}

// Translated from 005a38e0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Clears the fifteen words of the caches the condition functions keep
/// (the reference / form pairs and their `float` results).
pub fn fn_005a38e0(e: &mut Engine) {
    for address in CACHE_WORDS {
        e.set_global(address, 0u32);
    }
}

// Translated from 005a3980 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Script::IsEssentialConditionFunction` (Xbox PDB): 1.0 when the
/// reference is an actor and `Actor::GetEssential` (Xbox PDB) says so. Does
/// not check for a null reference.
pub fn script_is_essential_condition_function(
    e: &mut Engine,
    reference: Ptr,
    _param1: u32,
    _param2: u32,
    result: Ptr,
) -> bool {
    set_result(e, result, 0.0);
    if e.vcall(reference.addr(), VSLOT_IS_ACTOR, &args![]).bool()
        && e.call(ACTOR_GET_ESSENTIAL, &args![reference]).bool()
    {
        set_result(e, result, 1.0);
    }
    true
}

// Translated from 005a39d0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Condition function without Xbox PDB name: 1.0 when the reference is an
/// actor (virtual `IsActor`). Does not check for a null reference.
pub fn fn_005a39d0(
    e: &mut Engine,
    reference: Ptr,
    _param1: u32,
    _param2: u32,
    result: Ptr,
) -> bool {
    set_result(e, result, 0.0);
    if e.vcall(reference.addr(), VSLOT_IS_ACTOR, &args![]).bool() {
        set_result(e, result, 1.0);
    }
    true
}

// Translated from 005a3a00 (decompiled, FalloutNV.exe 1.4.0.525)
/// Condition function without Xbox PDB name: 1.0 when the byte at `+0x20c`
/// of the player is set.
pub fn fn_005a3a00(
    e: &mut Engine,
    _reference: u32,
    _param1: u32,
    _param2: u32,
    result: Ptr,
) -> bool {
    set_result(e, result, 0.0);
    let player = e.global::<u32>(PLAYER);
    if e.mem.u8(player + 0x20c) != 0 {
        set_result(e, result, 1.0);
    }
    true
}

// Translated from 005a3a30 (decompiled, FalloutNV.exe 1.4.0.525)
/// Condition function without Xbox PDB name: for an actor whose virtual
/// `+0x22c` (called with 0) answers yes and that has a middle-high process
/// whose float (virtual `+0x69c`) is positive, the calendar's whole value
/// (`00867e30`, as a `float`) minus that float; else 0.0.
pub fn fn_005a3a30(
    e: &mut Engine,
    reference: Ptr,
    _param1: u32,
    _param2: u32,
    result: Ptr,
) -> bool {
    set_result(e, result, 0.0);
    if !reference.is_null()
        && e.vcall(reference.addr(), VSLOT_ACTOR_TEST_22C, &args![0u32])
            .bool()
        && e.vcall(reference.addr(), VSLOT_IS_ACTOR, &args![]).bool()
        && !e
            .call(ACTOR_GET_PROCESS, &args![reference])
            .ptr::<()>()
            .is_null()
    {
        let process = e.call(ACTOR_GET_PROCESS, &args![reference]).u32();
        let value = e.vcall(process, VSLOT_PROCESS_FLOAT_69C, &args![]).f32();
        let zero: f64 = e.global(DOUBLE_ZERO);
        if value as f64 > zero {
            let whole = e.call(CALENDAR_GET_WHOLE_VALUE, &args![CALENDAR]).u32();
            set_result(e, result, (whole as f32) as f64 - value as f64);
        }
    }
    true
}

// Translated from 005a3af0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Condition function without Xbox PDB name: the `int` that `00964060`
/// returns for the player.
pub fn fn_005a3af0(
    e: &mut Engine,
    _reference: u32,
    _param1: u32,
    _param2: u32,
    result: Ptr,
) -> bool {
    set_result(e, result, 0.0);
    let player = e.global::<u32>(PLAYER);
    let value = e.call(PLAYER_GET_VALUE_964060, &args![player]).i32();
    set_result(e, result, value as f64);
    true
}

// Translated from 005a3b20 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Script::IsPlayerActionActiveConditionFunction` (Xbox PDB): the byte
/// answer of the player's `00963ce0` for the action in the first parameter
/// (0 or 1).
pub fn script_is_player_action_active_condition_function(
    e: &mut Engine,
    _reference: u32,
    action: u32,
    _param2: u32,
    result: Ptr,
) -> bool {
    set_result(e, result, 0.0);
    let player = e.global::<u32>(PLAYER);
    let active = e
        .call(PLAYER_IS_ACTION_ACTIVE, &args![player, action])
        .u32()
        & 0xff;
    set_result(e, result, active as f64);
    if trace_enabled(e) {
        // "IsActionInList >> %0.2f"
        trace_result(e, 0x0103_5e34, result);
    }
    true
}

// Translated from 005a3b90 (decompiled, FalloutNV.exe 1.4.0.525)
/// Condition function without Xbox PDB name: 1.0 when the reference's base
/// form has type `0x16` and its `+0x90` value (`00516bf0`) equals the first
/// parameter.
pub fn fn_005a3b90(e: &mut Engine, reference: Ptr, value: u32, _param2: u32, result: Ptr) -> bool {
    set_result(e, result, 0.0);
    if !reference.is_null() {
        let base = e.call(REF_GET_BASE_FORM, &args![reference]).u32();
        if e.call(FORM_GET_TYPE, &args![base]).u32() == FORM_TYPE_SPEAKER_SOURCE {
            let base = e.call(REF_GET_BASE_FORM, &args![reference]).u32();
            if e.call(FORM_GET_FIELD_90, &args![base]).u32() == value {
                set_result(e, result, 1.0);
            }
        }
    }
    true
}

// Translated from 005a3bf0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Condition function without Xbox PDB name: 1.0 when the player's
/// `00966c10` answers yes for the first parameter.
pub fn fn_005a3bf0(e: &mut Engine, _reference: u32, value: u32, _param2: u32, result: Ptr) -> bool {
    set_result(e, result, 0.0);
    let player = e.global::<u32>(PLAYER);
    if e.call(PLAYER_TEST_966C10, &args![player, value]).bool() {
        set_result(e, result, 1.0);
    }
    true
}

// Translated from 005a3c30 (decompiled, FalloutNV.exe 1.4.0.525)
/// Condition function without Xbox PDB name: for an actor with a part
/// object (`008a5270`), the `int` at its `+0x10`; -1.0 otherwise.
pub fn fn_005a3c30(
    e: &mut Engine,
    reference: Ptr,
    _param1: u32,
    _param2: u32,
    result: Ptr,
) -> bool {
    let minus_one: f64 = e.global(DOUBLE_MINUS_ONE);
    set_result(e, result, minus_one);
    let actor = actor_of(e, reference);
    if !actor.is_null()
        && !e
            .call(ACTOR_GET_PROCESS_PART, &args![actor])
            .ptr::<()>()
            .is_null()
    {
        let part = e.call(ACTOR_GET_PROCESS_PART, &args![actor]).u32();
        let value = e.mem.u32(part + 0x10) as i32;
        set_result(e, result, value as f64);
    }
    true
}

// Translated from 005a3c90 (decompiled, FalloutNV.exe 1.4.0.525)
/// Condition function without Xbox PDB name: for an actor with a part object
/// (`008a5270`), 1.0 when its flag test `0058cba0` with mask 4 answers yes.
pub fn fn_005a3c90(
    e: &mut Engine,
    reference: Ptr,
    _param1: u32,
    _param2: u32,
    result: Ptr,
) -> bool {
    set_result(e, result, 0.0);
    let actor = actor_of(e, reference);
    if !actor.is_null()
        && !e
            .call(ACTOR_GET_PROCESS_PART, &args![actor])
            .ptr::<()>()
            .is_null()
    {
        let part = e.call(ACTOR_GET_PROCESS_PART, &args![actor]).u32();
        let flagged = e.call(PROCESS_PART_TEST_FLAGS, &args![part, 4u32]).bool();
        set_result(e, result, if flagged { 1.0 } else { 0.0 });
    }
    true
}

/// This part's translated functions, by exe address.
pub fn funcs() -> Vec<(u32, AbiFn)> {
    vec![
        entry!(0x005a2a50, fn_005a2a50(u32, u32, u32, Ptr) -> bool),
        entry!(0x005a2a60, script_get_ghost_condition_function(Ptr, u32, u32, Ptr) -> bool),
        entry!(0x005a2af0, script_get_offers_service_condition_function(Ptr, u32, u32, Ptr) -> bool),
        entry!(0x005a2ba0, script_get_barter_gold_condition_function(Ptr, u32, u32, Ptr) -> bool),
        entry!(0x005a2c30, fn_005a2c30(Ptr, u32, u32, Ptr) -> bool),
        entry!(0x005a2c70, script_is_time_passing_condition_function(u32, u32, u32, Ptr) -> bool),
        entry!(0x005a2ce0, script_get_armor_rating_upper_condition_function(Ptr, u32, u32, Ptr) -> bool),
        entry!(0x005a2e20, fn_005a2e20(Ptr, Ptr, u32, Ptr) -> bool),
        entry!(0x005a2ed0, script_get_cell_ownership_condition_function(u32, Ptr, Ptr, Ptr) -> bool),
        entry!(0x005a2f60, fn_005a2f60(Ptr, u32, u32, Ptr) -> bool),
        entry!(0x005a3010, script_is_running_condition_function(Ptr, u32, u32, Ptr) -> bool),
        entry!(0x005a30c0, script_get_friend_hit_condition_function(Ptr, u32, u32, Ptr) -> bool),
        entry!(0x005a3180, script_is_in_combat_condition_function(Ptr, u32, u32, Ptr) -> bool),
        entry!(0x005a3270, script_is_in_interior_condition_function(Ptr, u32, u32, Ptr) -> bool),
        entry!(0x005a3310, script_get_pc_misc_stat_condition_function(u32, u32, u32, Ptr) -> bool),
        entry!(0x005a3370, fn_005a3370(u32) -> u32),
        entry!(0x005a3390, script_is_actor_evil_condition_function(Ptr, u32, u32, Ptr) -> bool),
        entry!(0x005a3460, script_is_actor_victim_condition_function(Ptr, u32, u32, Ptr) -> bool),
        entry!(0x005a3570, fn_005a3570(Ptr, u32, u32, Ptr) -> bool),
        entry!(0x005a35f0, fn_005a35f0(Ptr, u32, u32, Ptr) -> bool),
        entry!(0x005a3650, fn_005a3650(u32, u32, u32, Ptr) -> bool),
        entry!(0x005a3670, fn_005a3670(Ptr, u32, u32, Ptr) -> bool),
        entry!(0x005a36b0, script_is_in_dangerous_water_condition_function(Ptr, u32, u32, Ptr) -> bool),
        entry!(0x005a3740, fn_005a3740(Ptr) -> u8),
        entry!(0x005a3760, fn_005a3760(Ptr, u32, u32, Ptr) -> bool),
        entry!(0x005a3790, fn_005a3790(Ptr) -> bool),
        entry!(0x005a37b0, script_is_xbox_condition_function(u32, u32, u32, Ptr) -> bool),
        entry!(0x005a3820, script_is_ps3_condition_function(u32, u32, u32, Ptr) -> bool),
        entry!(0x005a3870, script_is_pc_condition_function(u32, u32, u32, Ptr) -> bool),
        entry!(0x005a38e0, fn_005a38e0()),
        entry!(0x005a3980, script_is_essential_condition_function(Ptr, u32, u32, Ptr) -> bool),
        entry!(0x005a39d0, fn_005a39d0(Ptr, u32, u32, Ptr) -> bool),
        entry!(0x005a3a00, fn_005a3a00(u32, u32, u32, Ptr) -> bool),
        entry!(0x005a3a30, fn_005a3a30(Ptr, u32, u32, Ptr) -> bool),
        entry!(0x005a3af0, fn_005a3af0(u32, u32, u32, Ptr) -> bool),
        entry!(0x005a3b20, script_is_player_action_active_condition_function(u32, u32, u32, Ptr) -> bool),
        entry!(0x005a3b90, fn_005a3b90(Ptr, u32, u32, Ptr) -> bool),
        entry!(0x005a3bf0, fn_005a3bf0(u32, u32, u32, Ptr) -> bool),
        entry!(0x005a3c30, fn_005a3c30(Ptr, u32, u32, Ptr) -> bool),
        entry!(0x005a3c90, fn_005a3c90(Ptr, u32, u32, Ptr) -> bool),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    /// What `*result` holds before a call; a function that leaves the result
    /// alone leaves this.
    const SENTINEL: f64 = -7.5;
    /// Virtual `IsActor` answering yes / no.
    const IS_ACTOR_YES: u32 = 0x0f00_0001;
    const IS_ACTOR_NO: u32 = 0x0f00_0002;
    /// A virtual that answers yes / no for the other byte-test slots.
    const SLOT_YES: u32 = 0x0f00_0003;
    const SLOT_NO: u32 = 0x0f00_0004;

    /// An engine with the constants the functions read from the exe's data,
    /// the debug print and the two `IsActor` answers.
    fn engine() -> Engine {
        let mut e = Engine::new();
        for page in [
            0x0101_1000u32,
            0x0101_2000,
            0x0101_a000,
            0x0119_f000,
            0x011c_6000,
            0x011c_a000,
            0x011d_e000,
        ] {
            e.map(page, 0x1000);
        }
        e.set_global(DOUBLE_TWO, 2.0f64);
        e.set_global(DOUBLE_ZERO, 0.0f64);
        e.set_global(DOUBLE_MINUS_ONE, -1.0f64);
        e.register(DEBUG_PRINT, |_, _| Ret::default());
        e.register(IS_ACTOR_YES, |_, _| true.into_ret());
        e.register(IS_ACTOR_NO, |_, _| false.into_ret());
        e.register(SLOT_YES, |_, _| true.into_ret());
        e.register(SLOT_NO, |_, _| false.into_ret());
        // The name functions return the argument plus one (or 0x100 for the
        // reference version), so the debug lines show which object was named.
        e.register(FORM_GET_FULL_NAME, |_, a| (a[0] + 1).into_ret());
        e.register(REF_GET_NAME, |_, a| (a[0] + 0x100).into_ret());
        e
    }

    /// Calls the condition function at `addr` with `*result` starting at
    /// `initial`; returns what it returned and what it left in `*result`.
    fn run_from(
        e: &mut Engine,
        initial: f64,
        addr: u32,
        reference: u32,
        param1: u32,
        param2: u32,
    ) -> (bool, f64) {
        let result = e.mem.alloc(8);
        e.mem.set_f64(result, initial);
        let returned = e
            .call(addr, &args![reference, param1, param2, result])
            .bool();
        (returned, e.mem.f64(result))
    }

    fn run(e: &mut Engine, addr: u32, reference: u32, param1: u32, param2: u32) -> (bool, f64) {
        run_from(e, SENTINEL, addr, reference, param1, param2)
    }

    /// A function that returns `eax`.
    fn stub(e: &mut Engine, addr: u32, eax: u32) {
        e.register_double(addr, move |_, _| Ret {
            eax,
            ..Ret::default()
        });
    }

    /// A function that returns `value` in ST0.
    fn stub_st0(e: &mut Engine, addr: u32, value: f64) {
        e.register_double(addr, move |_, _| Ret {
            st0: value,
            ..Ret::default()
        });
    }

    /// An object whose vtable has the given (offset, function) slots.
    fn object(e: &mut Engine, slots: &[(u32, u32)]) -> u32 {
        let vtable = e.mem.alloc(0x800);
        for &(offset, function) in slots {
            e.mem.set_u32(vtable + offset, function);
        }
        let object = e.mem.alloc(0x300);
        e.mem.set_u32(object, vtable);
        object
    }

    fn actor(e: &mut Engine) -> u32 {
        object(e, &[(VSLOT_IS_ACTOR, IS_ACTOR_YES)])
    }

    fn non_actor(e: &mut Engine) -> u32 {
        object(e, &[(VSLOT_IS_ACTOR, IS_ACTOR_NO)])
    }

    /// Turns the debug lines on and starts recording calls.
    fn trace_on(e: &mut Engine) {
        let tls = e.tls();
        e.mem.set_u8(tls + TLS_TRACE_FLAG, 1);
        e.call_log = Some(vec![]);
    }

    type Log = Vec<(u32, Vec<u32>)>;

    fn take_log(e: &mut Engine) -> Log {
        e.call_log.take().unwrap_or_default()
    }

    /// The argument words of every call to `addr`.
    fn calls_to(log: &Log, addr: u32) -> Vec<Vec<u32>> {
        log.iter()
            .filter(|(a, _)| *a == addr)
            .map(|(_, w)| w.clone())
            .collect()
    }

    fn words(value: f64) -> [u32; 2] {
        let bits = value.to_bits();
        [bits as u32, (bits >> 32) as u32]
    }

    /// The debug lines printed by one call with the lines on.
    fn printed(e: &mut Engine, initial: f64, addr: u32, reference: u32, p1: u32, p2: u32) -> Log {
        trace_on(e);
        run_from(e, initial, addr, reference, p1, p2);
        let log = take_log(e);
        log.into_iter().filter(|(a, _)| *a == DEBUG_PRINT).collect()
    }

    #[test]
    fn a2a50_is_always_zero() {
        let mut e = engine();
        assert_eq!(run(&mut e, 0x005a_2a50, 0x1000, 1, 2), (true, 0.0));
    }

    #[test]
    fn ghost_asks_the_actor() {
        let mut e = engine();
        let a = actor(&mut e);
        stub(&mut e, ACTOR_IS_GHOST, 1);
        assert_eq!(run(&mut e, 0x005a_2a60, a, 0, 0), (true, 1.0));
        stub(&mut e, ACTOR_IS_GHOST, 0);
        assert_eq!(run(&mut e, 0x005a_2a60, a, 0, 0), (true, 0.0));
        let plain = non_actor(&mut e);
        assert_eq!(run(&mut e, 0x005a_2a60, plain, 0, 0), (true, 0.0));
        assert_eq!(run(&mut e, 0x005a_2a60, 0, 0, 0), (true, 0.0));
        stub(&mut e, ACTOR_IS_GHOST, 1);
        let lines = printed(&mut e, 0.0, 0x005a_2a60, a, 0, 0);
        let w = words(1.0);
        assert_eq!(lines, vec![(DEBUG_PRINT, vec![0x0103_5bfc, w[0], w[1]])]);
    }

    #[test]
    fn offers_service_names_the_actor_in_the_debug_line() {
        let mut e = engine();
        let a = actor(&mut e);
        stub(&mut e, ACTOR_OFFERS_SERVICES, 1);
        assert_eq!(run(&mut e, 0x005a_2af0, a, 0, 0), (true, 1.0));
        stub(&mut e, ACTOR_OFFERS_SERVICES, 0);
        assert_eq!(run(&mut e, 0x005a_2af0, a, 0, 0), (true, 0.0));
        stub(&mut e, ACTOR_OFFERS_SERVICES, 1);
        let lines = printed(&mut e, 0.0, 0x005a_2af0, a, 0, 0);
        assert_eq!(lines, vec![(DEBUG_PRINT, vec![0x0103_5c2c, a + 0x100])]);
        // Not an actor: result 0, and the "not offering" line names null.
        let plain = non_actor(&mut e);
        let lines = printed(&mut e, 0.0, 0x005a_2af0, plain, 0, 0);
        assert_eq!(lines, vec![(DEBUG_PRINT, vec![0x0103_5c10, 0x100])]);
    }

    #[test]
    fn barter_gold_is_the_base_gold_and_leaves_non_actors_alone() {
        let mut e = engine();
        let a = actor(&mut e);
        stub(&mut e, ACTOR_GET_BARTER_GOLD_BASE, 250);
        assert_eq!(run(&mut e, 0x005a_2ba0, a, 0, 0), (true, 250.0));
        let plain = non_actor(&mut e);
        assert_eq!(run(&mut e, 0x005a_2ba0, plain, 0, 0), (true, SENTINEL));
        let lines = printed(&mut e, 0.0, 0x005a_2ba0, a, 0, 0);
        assert_eq!(lines, vec![(DEBUG_PRINT, vec![0x0103_5c48, a, 250])]);
        assert!(printed(&mut e, 0.0, 0x005a_2ba0, plain, 0, 0).is_empty());
    }

    #[test]
    fn c30_is_always_zero() {
        let mut e = engine();
        let a = actor(&mut e);
        assert_eq!(run(&mut e, 0x005a_2c30, a, 0, 0), (true, 0.0));
        assert_eq!(run(&mut e, 0x005a_2c30, 0, 0, 0), (true, 0.0));
    }

    #[test]
    fn time_passing_follows_the_player_and_keeps_an_unset_result() {
        let mut e = engine();
        let player = actor(&mut e);
        e.set_global(PLAYER, player);
        e.register_double(PLAYER_IS_SLEEPING_OR_RESTING, move |_, a| {
            assert_eq!(a, [player]);
            true.into_ret()
        });
        assert_eq!(run(&mut e, 0x005a_2c70, 0, 0, 0), (true, 1.0));
        stub(&mut e, PLAYER_IS_SLEEPING_OR_RESTING, 0);
        assert_eq!(run(&mut e, 0x005a_2c70, 0, 0, 0), (true, SENTINEL));
        let lines = printed(&mut e, 0.0, 0x005a_2c70, 0, 0, 0);
        assert_eq!(lines, vec![(DEBUG_PRINT, vec![0x0103_5c6c])]);
        stub(&mut e, PLAYER_IS_SLEEPING_OR_RESTING, 1);
        let lines = printed(&mut e, 0.0, 0x005a_2c70, 0, 0, 0);
        assert_eq!(lines, vec![(DEBUG_PRINT, vec![0x0103_5c80])]);
    }

    #[test]
    fn armor_rating_upper_reads_the_worn_armor_and_destroys_the_item() {
        let mut e = engine();
        let a = actor(&mut e);
        stub(&mut e, GET_INVENTORY_CHANGES, 0x7000);
        e.register_double(INVENTORY_GET_WORN_ITEM, |_, args| {
            assert_eq!(args, [0x7000, 2, 0]);
            0x7100u32.into_ret()
        });
        stub(&mut e, OBJECT_GET_FIELD_08, 0x7200);
        stub(&mut e, FORM_GET_TYPE, 0x18);
        stub(&mut e, ARMOR_GET_CLASS_FLAG, 0);
        stub(&mut e, ITEM_CHANGE_DESTROY, 0);
        e.call_log = Some(vec![]);
        assert_eq!(run(&mut e, 0x005a_2ce0, a, 0, 0), (true, 1.0));
        let log = take_log(&mut e);
        assert_eq!(calls_to(&log, ITEM_CHANGE_DESTROY), vec![vec![0x7100, 1]]);
        assert_eq!(calls_to(&log, GET_INVENTORY_CHANGES), vec![vec![a]]);
        // The armor test answers: the exe's 2.0.
        stub(&mut e, ARMOR_GET_CLASS_FLAG, 2);
        assert_eq!(run(&mut e, 0x005a_2ce0, a, 0, 0), (true, 2.0));
        // A form of another type: 0.0, but the item is still destroyed.
        stub(&mut e, FORM_GET_TYPE, 0x28);
        e.call_log = Some(vec![]);
        assert_eq!(run(&mut e, 0x005a_2ce0, a, 0, 0), (true, 0.0));
        let log = take_log(&mut e);
        assert_eq!(calls_to(&log, ITEM_CHANGE_DESTROY).len(), 1);
        // Nothing worn / no changes / not an actor.
        stub(&mut e, INVENTORY_GET_WORN_ITEM, 0);
        e.call_log = Some(vec![]);
        assert_eq!(run(&mut e, 0x005a_2ce0, a, 0, 0), (true, 0.0));
        assert!(calls_to(&take_log(&mut e), ITEM_CHANGE_DESTROY).is_empty());
        stub(&mut e, GET_INVENTORY_CHANGES, 0);
        assert_eq!(run(&mut e, 0x005a_2ce0, a, 0, 0), (true, 0.0));
        let plain = non_actor(&mut e);
        assert_eq!(run(&mut e, 0x005a_2ce0, plain, 0, 0), (true, 0.0));
        let lines = printed(&mut e, 0.0, 0x005a_2ce0, plain, 0, 0);
        let w = words(0.0);
        assert_eq!(lines, vec![(DEBUG_PRINT, vec![0x0103_5c90, w[0], w[1]])]);
    }

    #[test]
    fn e20_compares_the_extra_data_owner_with_the_form() {
        let mut e = engine();
        let player = actor(&mut e);
        e.set_global(PLAYER, player);
        stub(&mut e, REF_GET_EXTRA_DATA_LIST, 0x8000);
        e.register_double(EXTRA_LIST_GET_OWNER, |_, a| {
            assert_eq!(a, [0x8000]);
            0x9000u32.into_ret()
        });
        assert_eq!(run(&mut e, 0x005a_2e20, 0x2000, 0x9000, 0), (true, 1.0));
        // A different form leaves the result as it was.
        assert_eq!(
            run(&mut e, 0x005a_2e20, 0x2000, 0x9100, 0),
            (true, SENTINEL)
        );
        // A null reference too.
        assert_eq!(run(&mut e, 0x005a_2e20, 0, 0x9000, 0), (true, SENTINEL));
        // A null form: the player's base form is used.
        e.register_double(REF_GET_BASE_FORM, move |_, a| {
            assert_eq!(a, [player]);
            0x9000u32.into_ret()
        });
        assert_eq!(run(&mut e, 0x005a_2e20, 0x2000, 0, 0), (true, 1.0));
        let lines = printed(&mut e, 0.0, 0x005a_2e20, 0x2000, 0x9000, 0);
        assert_eq!(lines, vec![(DEBUG_PRINT, vec![0x0103_5cc8, 0x9001])]);
        let lines = printed(&mut e, 0.0, 0x005a_2e20, 0x2000, 0x9100, 0);
        assert_eq!(lines, vec![(DEBUG_PRINT, vec![0x0103_5cb4, 0x9101])]);
    }

    #[test]
    fn cell_ownership_compares_the_cell_owner_with_the_form() {
        let mut e = engine();
        e.register_double(CELL_GET_OWNER, |_, a| {
            assert_eq!(a, [0x3000]);
            0x9000u32.into_ret()
        });
        assert_eq!(run(&mut e, 0x005a_2ed0, 0, 0x3000, 0x9000), (true, 1.0));
        assert_eq!(
            run(&mut e, 0x005a_2ed0, 0, 0x3000, 0x9100),
            (true, SENTINEL)
        );
        let lines = printed(&mut e, 0.0, 0x005a_2ed0, 0, 0x3000, 0x9100);
        assert_eq!(lines, vec![(DEBUG_PRINT, vec![0x0103_5cb4, 0x9101])]);
        let lines = printed(&mut e, 0.0, 0x005a_2ed0, 0, 0x3000, 0x9000);
        assert_eq!(lines, vec![(DEBUG_PRINT, vec![0x0103_5cc8, 0x9001])]);
    }

    #[test]
    fn f60_asks_the_sneaking_test_even_without_an_actor() {
        let mut e = engine();
        let a = actor(&mut e);
        stub(&mut e, ACTOR_IS_SNEAKING, 1);
        assert_eq!(run(&mut e, 0x005a_2f60, a, 0, 0), (true, 1.0));
        stub(&mut e, ACTOR_IS_SNEAKING, 0);
        assert_eq!(run(&mut e, 0x005a_2f60, a, 0, 0), (true, 0.0));
        let plain = non_actor(&mut e);
        e.register_double(ACTOR_IS_SNEAKING, |_, a| {
            assert_eq!(a, [0]);
            true.into_ret()
        });
        assert_eq!(run(&mut e, 0x005a_2f60, plain, 0, 0), (true, 1.0));
        stub(&mut e, ACTOR_IS_SNEAKING, 1);
        let lines = printed(&mut e, 0.0, 0x005a_2f60, a, 0, 0);
        assert_eq!(lines, vec![(DEBUG_PRINT, vec![0x0103_5cec, a + 1])]);
        stub(&mut e, ACTOR_IS_SNEAKING, 0);
        let lines = printed(&mut e, 0.0, 0x005a_2f60, a, 0, 0);
        assert_eq!(lines, vec![(DEBUG_PRINT, vec![0x0103_5cd8, a + 1])]);
    }

    #[test]
    fn is_running_asks_the_actor_even_without_one() {
        let mut e = engine();
        let a = actor(&mut e);
        stub(&mut e, ACTOR_IS_RUNNING, 1);
        assert_eq!(run(&mut e, 0x005a_3010, a, 0, 0), (true, 1.0));
        stub(&mut e, ACTOR_IS_RUNNING, 0);
        assert_eq!(run(&mut e, 0x005a_3010, a, 0, 0), (true, 0.0));
        e.register_double(ACTOR_IS_RUNNING, |_, a| {
            assert_eq!(a, [0]);
            false.into_ret()
        });
        assert_eq!(run(&mut e, 0x005a_3010, 0, 0, 0), (true, 0.0));
        stub(&mut e, ACTOR_IS_RUNNING, 1);
        let lines = printed(&mut e, 0.0, 0x005a_3010, a, 0, 0);
        assert_eq!(lines, vec![(DEBUG_PRINT, vec![0x0103_5d10, a + 1])]);
        stub(&mut e, ACTOR_IS_RUNNING, 0);
        let lines = printed(&mut e, 0.0, 0x005a_3010, a, 0, 0);
        assert_eq!(lines, vec![(DEBUG_PRINT, vec![0x0103_5cfc, a + 1])]);
    }

    #[test]
    fn friend_hit_is_the_extra_data_count_and_names_the_player_and_actor() {
        let mut e = engine();
        let a = actor(&mut e);
        let player = actor(&mut e);
        e.set_global(PLAYER, player);
        stub(&mut e, REF_GET_EXTRA_DATA_LIST, 0x8000);
        e.register_double(EXTRA_LIST_GET_FRIEND_HIT_COUNT, |_, args| {
            assert_eq!(args, [0x8000]);
            3u32.into_ret()
        });
        assert_eq!(run(&mut e, 0x005a_30c0, a, 0, 0), (true, 3.0));
        let lines = printed(&mut e, 0.0, 0x005a_30c0, a, 0, 0);
        let w = words(3.0);
        assert_eq!(
            lines,
            vec![(
                DEBUG_PRINT,
                vec![0x0103_5d20, player + 1, a + 1, w[0], w[1]]
            )]
        );
        // Not an actor: 0.0 and no debug line.
        let plain = non_actor(&mut e);
        assert_eq!(run(&mut e, 0x005a_30c0, plain, 0, 0), (true, 0.0));
        assert!(printed(&mut e, 0.0, 0x005a_30c0, plain, 0, 0).is_empty());
    }

    #[test]
    fn in_combat_uses_the_player_function_for_the_player() {
        let mut e = engine();
        let a = actor(&mut e);
        let player = actor(&mut e);
        e.set_global(PLAYER, player);
        stub(&mut e, ACTOR_IN_COMBAT_FLAG, 1);
        assert_eq!(run(&mut e, 0x005a_3180, a, 0, 0), (true, 1.0));
        stub(&mut e, ACTOR_IN_COMBAT_FLAG, 0);
        assert_eq!(run(&mut e, 0x005a_3180, a, 0, 0), (true, 0.0));
        // The player: the flag says yes, the player's own test says no.
        stub(&mut e, ACTOR_IN_COMBAT_FLAG, 1);
        e.register_double(PLAYER_IS_IN_COMBAT, move |e, args| {
            assert_eq!(args[0], player);
            // The out byte starts as 0.
            assert_eq!(e.mem.u8(args[1]), 0);
            0u32.into_ret()
        });
        assert_eq!(run(&mut e, 0x005a_3180, player, 0, 0), (true, 0.0));
        stub(&mut e, PLAYER_IS_IN_COMBAT, 0x101);
        assert_eq!(run(&mut e, 0x005a_3180, player, 0, 0), (true, 1.0));
        // A non-actor: nothing is asked.
        let plain = non_actor(&mut e);
        assert_eq!(run(&mut e, 0x005a_3180, plain, 0, 0), (true, 0.0));
        let lines = printed(&mut e, 0.0, 0x005a_3180, a, 0, 0);
        assert_eq!(lines, vec![(DEBUG_PRINT, vec![0x0103_5d50, a + 1])]);
        stub(&mut e, ACTOR_IN_COMBAT_FLAG, 0);
        let lines = printed(&mut e, 0.0, 0x005a_3180, a, 0, 0);
        assert_eq!(lines, vec![(DEBUG_PRINT, vec![0x0103_5d3c, a + 1])]);
    }

    #[test]
    fn in_interior_asks_the_parent_cell() {
        let mut e = engine();
        e.register_double(REF_GET_PARENT_CELL, |_, a| {
            assert_eq!(a, [0x2000]);
            0x6000u32.into_ret()
        });
        e.register_double(CELL_IS_INTERIOR, |_, a| {
            assert_eq!(a, [0x6000]);
            true.into_ret()
        });
        assert_eq!(run(&mut e, 0x005a_3270, 0x2000, 0, 0), (true, 1.0));
        stub(&mut e, CELL_IS_INTERIOR, 0);
        assert_eq!(run(&mut e, 0x005a_3270, 0x2000, 0, 0), (true, 0.0));
        stub(&mut e, REF_GET_PARENT_CELL, 0);
        assert_eq!(run(&mut e, 0x005a_3270, 0x2000, 0, 0), (true, 0.0));
        assert_eq!(run(&mut e, 0x005a_3270, 0, 0, 0), (true, 0.0));
        let lines = printed(&mut e, 0.0, 0x005a_3270, 0x2000, 0, 0);
        assert_eq!(lines, vec![(DEBUG_PRINT, vec![0x0103_5d60, 0x2001])]);
        stub(&mut e, REF_GET_PARENT_CELL, 0x6000);
        stub(&mut e, CELL_IS_INTERIOR, 1);
        let lines = printed(&mut e, 0.0, 0x005a_3270, 0x2000, 0, 0);
        assert_eq!(lines, vec![(DEBUG_PRINT, vec![0x0103_5d78, 0x2001])]);
    }

    /// The table of misc stats: entry `index` points at a block whose second
    /// word is the value.
    fn misc_stat(e: &mut Engine, index: u32, value: u32) {
        let block = e.mem.alloc(8);
        e.mem.set_u32(block + 4, value);
        e.mem.set_u32(MISC_STAT_TABLE + 4 * index, block);
    }

    #[test]
    fn pc_misc_stat_reads_the_indexed_entry() {
        let mut e = engine();
        misc_stat(&mut e, 2, 42);
        misc_stat(&mut e, 3, (-1i32) as u32);
        assert_eq!(run(&mut e, 0x005a_3310, 0, 2, 0), (true, 42.0));
        assert_eq!(run(&mut e, 0x005a_3310, 0, 3, 0), (true, -1.0));
        let lines = printed(&mut e, 0.0, 0x005a_3310, 0, 2, 0);
        let w = words(42.0);
        assert_eq!(lines, vec![(DEBUG_PRINT, vec![0x0103_5d90, w[0], w[1]])]);
    }

    #[test]
    fn a3370_reads_the_value_of_the_table_entry() {
        let mut e = engine();
        misc_stat(&mut e, 5, 1234);
        assert_eq!(e.call(0x005a_3370, &args![5u32]).u32(), 1234);
    }

    #[test]
    fn actor_evil_asks_the_faction_test_and_prints_only_for_actors() {
        let mut e = engine();
        let a = actor(&mut e);
        stub(&mut e, REF_IS_PART_OF_EVIL_FACTION, 1);
        assert_eq!(run(&mut e, 0x005a_3390, a, 0, 0), (true, 1.0));
        stub(&mut e, REF_IS_PART_OF_EVIL_FACTION, 0);
        assert_eq!(run(&mut e, 0x005a_3390, a, 0, 0), (true, 0.0));
        let plain = non_actor(&mut e);
        assert_eq!(run(&mut e, 0x005a_3390, plain, 0, 0), (true, 0.0));
        let lines = printed(&mut e, 0.0, 0x005a_3390, a, 0, 0);
        assert_eq!(lines, vec![(DEBUG_PRINT, vec![0x0103_5db0, a + 0x100])]);
        stub(&mut e, REF_IS_PART_OF_EVIL_FACTION, 1);
        let lines = printed(&mut e, 0.0, 0x005a_3390, a, 0, 0);
        assert_eq!(lines, vec![(DEBUG_PRINT, vec![0x0103_5dc4, a + 0x100])]);
        assert!(printed(&mut e, 0.0, 0x005a_3390, plain, 0, 0).is_empty());
    }

    #[test]
    fn actor_victim_compares_the_base_form_with_the_tls_victim() {
        let mut e = engine();
        let a = object(
            &mut e,
            &[
                (VSLOT_IS_ACTOR, IS_ACTOR_YES),
                (VSLOT_ACTOR_TEST_218, SLOT_YES),
            ],
        );
        let tls = e.tls();
        e.mem.set_u32(tls + TLS_CRIME_VICTIM, 0x5000);
        stub(&mut e, REF_GET_BASE_FORM, 0x5000);
        assert_eq!(run(&mut e, 0x005a_3460, a, 0, 0), (true, 1.0));
        stub(&mut e, REF_GET_BASE_FORM, 0x5100);
        assert_eq!(run(&mut e, 0x005a_3460, a, 0, 0), (true, 0.0));
        // A null base form never matches (even a null victim).
        e.mem.set_u32(tls + TLS_CRIME_VICTIM, 0);
        stub(&mut e, REF_GET_BASE_FORM, 0);
        assert_eq!(run(&mut e, 0x005a_3460, a, 0, 0), (true, 0.0));
        // The virtual `+0x218` says no: nothing else runs, no debug line.
        let b = object(
            &mut e,
            &[
                (VSLOT_IS_ACTOR, IS_ACTOR_YES),
                (VSLOT_ACTOR_TEST_218, SLOT_NO),
            ],
        );
        assert_eq!(run(&mut e, 0x005a_3460, b, 0, 0), (true, 0.0));
        assert!(printed(&mut e, 0.0, 0x005a_3460, b, 0, 0).is_empty());
        let plain = non_actor(&mut e);
        assert_eq!(run(&mut e, 0x005a_3460, plain, 0, 0), (true, 0.0));
        e.mem.set_u32(tls + TLS_CRIME_VICTIM, 0x5000);
        stub(&mut e, REF_GET_BASE_FORM, 0x5000);
        let lines = printed(&mut e, 0.0, 0x005a_3460, a, 0, 0);
        assert_eq!(lines, vec![(DEBUG_PRINT, vec![0x0103_5df0, a + 0x100])]);
        stub(&mut e, REF_GET_BASE_FORM, 0x5100);
        let lines = printed(&mut e, 0.0, 0x005a_3460, a, 0, 0);
        assert_eq!(lines, vec![(DEBUG_PRINT, vec![0x0103_5dd4, a + 0x100])]);
    }

    #[test]
    fn a3570_asks_the_process() {
        let mut e = engine();
        let a = actor(&mut e);
        let process = object(&mut e, &[(VSLOT_PROCESS_TEST_60, SLOT_YES)]);
        stub(&mut e, ACTOR_GET_PROCESS, process);
        assert_eq!(run(&mut e, 0x005a_3570, a, 0, 0), (true, 1.0));
        let process_no = object(&mut e, &[(VSLOT_PROCESS_TEST_60, SLOT_NO)]);
        stub(&mut e, ACTOR_GET_PROCESS, process_no);
        assert_eq!(run(&mut e, 0x005a_3570, a, 0, 0), (true, 0.0));
        stub(&mut e, ACTOR_GET_PROCESS, 0);
        assert_eq!(run(&mut e, 0x005a_3570, a, 0, 0), (true, 0.0));
        let plain = non_actor(&mut e);
        assert_eq!(run(&mut e, 0x005a_3570, plain, 0, 0), (true, 0.0));
    }

    #[test]
    fn a35f0_asks_the_actor_test() {
        let mut e = engine();
        let a = actor(&mut e);
        stub(&mut e, ACTOR_TEST_87F510, 1);
        assert_eq!(run(&mut e, 0x005a_35f0, a, 0, 0), (true, 1.0));
        stub(&mut e, ACTOR_TEST_87F510, 0);
        assert_eq!(run(&mut e, 0x005a_35f0, a, 0, 0), (true, 0.0));
        let plain = non_actor(&mut e);
        stub(&mut e, ACTOR_TEST_87F510, 1);
        assert_eq!(run(&mut e, 0x005a_35f0, plain, 0, 0), (true, 0.0));
    }

    #[test]
    fn a3650_returns_the_int_global() {
        let mut e = engine();
        e.set_global(INT_GLOBAL_5A3650, (-5i32) as u32);
        assert_eq!(run(&mut e, 0x005a_3650, 0, 0, 0), (true, -5.0));
        e.set_global(INT_GLOBAL_5A3650, 12u32);
        assert_eq!(run(&mut e, 0x005a_3650, 0, 0, 0), (true, 12.0));
    }

    #[test]
    fn a3670_is_always_zero() {
        let mut e = engine();
        let a = actor(&mut e);
        assert_eq!(run(&mut e, 0x005a_3670, a, 0, 0), (true, 0.0));
        assert_eq!(run(&mut e, 0x005a_3670, 0, 0, 0), (true, 0.0));
    }

    #[test]
    fn dangerous_water_needs_the_flag_the_cell_and_a_dangerous_water_type() {
        let mut e = engine();
        let a = actor(&mut e);
        e.mem.set_u8(a + 0x14c, 1);
        stub(&mut e, REF_GET_PARENT_CELL, 0x6000);
        e.register_double(CELL_GET_WATER_TYPE, |_, args| {
            assert_eq!(args, [0x6000]);
            0x6100u32.into_ret()
        });
        e.register_double(WATER_FORM_GET_DANGEROUS, |_, args| {
            assert_eq!(args, [0x6100]);
            true.into_ret()
        });
        assert_eq!(run(&mut e, 0x005a_36b0, a, 0, 0), (true, 1.0));
        stub(&mut e, WATER_FORM_GET_DANGEROUS, 0);
        assert_eq!(run(&mut e, 0x005a_36b0, a, 0, 0), (true, 0.0));
        stub(&mut e, WATER_FORM_GET_DANGEROUS, 1);
        stub(&mut e, CELL_GET_WATER_TYPE, 0);
        assert_eq!(run(&mut e, 0x005a_36b0, a, 0, 0), (true, 0.0));
        stub(&mut e, CELL_GET_WATER_TYPE, 0x6100);
        stub(&mut e, REF_GET_PARENT_CELL, 0);
        assert_eq!(run(&mut e, 0x005a_36b0, a, 0, 0), (true, 0.0));
        stub(&mut e, REF_GET_PARENT_CELL, 0x6000);
        e.mem.set_u8(a + 0x14c, 0);
        assert_eq!(run(&mut e, 0x005a_36b0, a, 0, 0), (true, 0.0));
        let plain = non_actor(&mut e);
        e.mem.set_u8(plain + 0x14c, 1);
        assert_eq!(run(&mut e, 0x005a_36b0, plain, 0, 0), (true, 0.0));
    }

    #[test]
    fn a3740_reads_the_byte_at_14c() {
        let mut e = engine();
        let block = e.mem.alloc(0x200);
        e.mem.set_u8(block + 0x14c, 0x7f);
        assert_eq!(e.call(0x005a_3740, &args![block]).u32() & 0xff, 0x7f);
    }

    #[test]
    fn a3760_tests_the_reference_flag() {
        let mut e = engine();
        let block = e.mem.alloc(0x20);
        assert_eq!(run(&mut e, 0x005a_3760, block, 0, 0), (true, 0.0));
        e.mem.set_u32(block + 8, 0x0010_0000);
        assert_eq!(run(&mut e, 0x005a_3760, block, 0, 0), (true, 1.0));
    }

    #[test]
    fn a3790_tests_bit_100000_of_the_flags() {
        let mut e = engine();
        let block = e.mem.alloc(0x20);
        e.mem.set_u32(block + 8, 0xffef_ffff);
        assert!(!e.call(0x005a_3790, &args![block]).bool());
        e.mem.set_u32(block + 8, 0x0010_0000);
        assert!(e.call(0x005a_3790, &args![block]).bool());
    }

    #[test]
    fn xbox_follows_the_platform_test() {
        let mut e = engine();
        stub(&mut e, PLATFORM_IS_CONSOLE, 1);
        assert_eq!(run(&mut e, 0x005a_37b0, 0, 0, 0), (true, 1.0));
        stub(&mut e, PLATFORM_IS_CONSOLE, 0);
        assert_eq!(run(&mut e, 0x005a_37b0, 0, 0, 0), (true, 0.0));
        let lines = printed(&mut e, 0.0, 0x005a_37b0, 0, 0, 0);
        let w = words(0.0);
        assert_eq!(lines, vec![(DEBUG_PRINT, vec![0x0103_5e04, w[0], w[1]])]);
    }

    #[test]
    fn ps3_is_always_zero() {
        let mut e = engine();
        assert_eq!(run(&mut e, 0x005a_3820, 0, 0, 0), (true, 0.0));
        let lines = printed(&mut e, 1.0, 0x005a_3820, 0, 0, 0);
        let w = words(0.0);
        assert_eq!(lines, vec![(DEBUG_PRINT, vec![0x0103_5e14, w[0], w[1]])]);
    }

    #[test]
    fn pc_is_the_opposite_of_the_platform_test() {
        let mut e = engine();
        stub(&mut e, PLATFORM_IS_CONSOLE, 1);
        assert_eq!(run(&mut e, 0x005a_3870, 0, 0, 0), (true, 0.0));
        stub(&mut e, PLATFORM_IS_CONSOLE, 0);
        assert_eq!(run(&mut e, 0x005a_3870, 0, 0, 0), (true, 1.0));
        let lines = printed(&mut e, 0.0, 0x005a_3870, 0, 0, 0);
        let w = words(1.0);
        assert_eq!(lines, vec![(DEBUG_PRINT, vec![0x0103_5e24, w[0], w[1]])]);
    }

    #[test]
    fn a38e0_clears_the_caches() {
        let mut e = engine();
        for address in CACHE_WORDS {
            e.set_global(address, 0xdead_beefu32);
        }
        e.call(0x005a_38e0, &args![]);
        for address in CACHE_WORDS {
            assert_eq!(e.global::<u32>(address), 0);
        }
    }

    #[test]
    fn essential_asks_the_actor_without_a_null_check() {
        let mut e = engine();
        let a = actor(&mut e);
        stub(&mut e, ACTOR_GET_ESSENTIAL, 1);
        assert_eq!(run(&mut e, 0x005a_3980, a, 0, 0), (true, 1.0));
        stub(&mut e, ACTOR_GET_ESSENTIAL, 0);
        assert_eq!(run(&mut e, 0x005a_3980, a, 0, 0), (true, 0.0));
        let plain = non_actor(&mut e);
        stub(&mut e, ACTOR_GET_ESSENTIAL, 1);
        assert_eq!(run(&mut e, 0x005a_3980, plain, 0, 0), (true, 0.0));
    }

    #[test]
    fn a39d0_is_the_actor_test() {
        let mut e = engine();
        let a = actor(&mut e);
        let plain = non_actor(&mut e);
        assert_eq!(run(&mut e, 0x005a_39d0, a, 0, 0), (true, 1.0));
        assert_eq!(run(&mut e, 0x005a_39d0, plain, 0, 0), (true, 0.0));
    }

    #[test]
    fn a3a00_reads_the_player_byte_at_20c() {
        let mut e = engine();
        let player = e.mem.alloc(0x300);
        e.set_global(PLAYER, player);
        assert_eq!(run(&mut e, 0x005a_3a00, 0, 0, 0), (true, 0.0));
        e.mem.set_u8(player + 0x20c, 1);
        assert_eq!(run(&mut e, 0x005a_3a00, 0, 0, 0), (true, 1.0));
    }

    #[test]
    fn a3a30_subtracts_the_process_float_from_the_calendar_value() {
        let mut e = engine();
        let a = object(
            &mut e,
            &[
                (VSLOT_IS_ACTOR, IS_ACTOR_YES),
                (VSLOT_ACTOR_TEST_22C, SLOT_YES),
            ],
        );
        let process = object(&mut e, &[(VSLOT_PROCESS_FLOAT_69C, 0x0f00_0010)]);
        stub_st0(&mut e, 0x0f00_0010, 30.5);
        stub(&mut e, ACTOR_GET_PROCESS, process);
        e.register_double(CALENDAR_GET_WHOLE_VALUE, |_, args| {
            assert_eq!(args, [CALENDAR]);
            100u32.into_ret()
        });
        assert_eq!(run(&mut e, 0x005a_3a30, a, 0, 0), (true, 69.5));
        // A process float that is not positive: 0.0.
        stub_st0(&mut e, 0x0f00_0010, 0.0);
        assert_eq!(run(&mut e, 0x005a_3a30, a, 0, 0), (true, 0.0));
        stub_st0(&mut e, 0x0f00_0010, 30.5);
        // No process.
        stub(&mut e, ACTOR_GET_PROCESS, 0);
        assert_eq!(run(&mut e, 0x005a_3a30, a, 0, 0), (true, 0.0));
        stub(&mut e, ACTOR_GET_PROCESS, process);
        // The `+0x22c` test says no.
        let b = object(
            &mut e,
            &[
                (VSLOT_IS_ACTOR, IS_ACTOR_YES),
                (VSLOT_ACTOR_TEST_22C, SLOT_NO),
            ],
        );
        assert_eq!(run(&mut e, 0x005a_3a30, b, 0, 0), (true, 0.0));
        assert_eq!(run(&mut e, 0x005a_3a30, 0, 0, 0), (true, 0.0));
    }

    #[test]
    fn a3af0_returns_the_players_value() {
        let mut e = engine();
        e.set_global(PLAYER, 0x4400u32);
        e.register_double(PLAYER_GET_VALUE_964060, |_, args| {
            assert_eq!(args, [0x4400]);
            (-3i32 as u32).into_ret()
        });
        assert_eq!(run(&mut e, 0x005a_3af0, 0, 0, 0), (true, -3.0));
    }

    #[test]
    fn player_action_active_passes_the_action_and_masks_the_byte() {
        let mut e = engine();
        e.set_global(PLAYER, 0x4400u32);
        e.register_double(PLAYER_IS_ACTION_ACTIVE, |_, args| {
            assert_eq!(args, [0x4400, 9]);
            0x101u32.into_ret()
        });
        assert_eq!(run(&mut e, 0x005a_3b20, 0, 9, 0), (true, 1.0));
        stub(&mut e, PLAYER_IS_ACTION_ACTIVE, 0x100);
        assert_eq!(run(&mut e, 0x005a_3b20, 0, 9, 0), (true, 0.0));
        let lines = printed(&mut e, 0.0, 0x005a_3b20, 0, 9, 0);
        let w = words(0.0);
        assert_eq!(lines, vec![(DEBUG_PRINT, vec![0x0103_5e34, w[0], w[1]])]);
    }

    #[test]
    fn b90_compares_the_form_field_when_the_base_form_has_type_16() {
        let mut e = engine();
        stub(&mut e, REF_GET_BASE_FORM, 0x5000);
        stub(&mut e, FORM_GET_TYPE, 0x16);
        e.register_double(FORM_GET_FIELD_90, |_, args| {
            assert_eq!(args, [0x5000]);
            77u32.into_ret()
        });
        assert_eq!(run(&mut e, 0x005a_3b90, 0x2000, 77, 0), (true, 1.0));
        assert_eq!(run(&mut e, 0x005a_3b90, 0x2000, 78, 0), (true, 0.0));
        stub(&mut e, FORM_GET_TYPE, 0x17);
        assert_eq!(run(&mut e, 0x005a_3b90, 0x2000, 77, 0), (true, 0.0));
        assert_eq!(run(&mut e, 0x005a_3b90, 0, 77, 0), (true, 0.0));
    }

    #[test]
    fn bf0_asks_the_player_with_the_parameter() {
        let mut e = engine();
        e.set_global(PLAYER, 0x4400u32);
        e.register_double(PLAYER_TEST_966C10, |_, args| {
            assert_eq!(args, [0x4400, 0x55]);
            true.into_ret()
        });
        assert_eq!(run(&mut e, 0x005a_3bf0, 0, 0x55, 0), (true, 1.0));
        stub(&mut e, PLAYER_TEST_966C10, 0);
        assert_eq!(run(&mut e, 0x005a_3bf0, 0, 0x55, 0), (true, 0.0));
    }

    #[test]
    fn c30_reads_the_part_value_and_defaults_to_minus_one() {
        let mut e = engine();
        let a = actor(&mut e);
        let part = e.mem.alloc(0x20);
        e.mem.set_u32(part + 0x10, (-4i32) as u32);
        stub(&mut e, ACTOR_GET_PROCESS_PART, part);
        assert_eq!(run(&mut e, 0x005a_3c30, a, 0, 0), (true, -4.0));
        e.mem.set_u32(part + 0x10, 9);
        assert_eq!(run(&mut e, 0x005a_3c30, a, 0, 0), (true, 9.0));
        stub(&mut e, ACTOR_GET_PROCESS_PART, 0);
        assert_eq!(run(&mut e, 0x005a_3c30, a, 0, 0), (true, -1.0));
        let plain = non_actor(&mut e);
        assert_eq!(run(&mut e, 0x005a_3c30, plain, 0, 0), (true, -1.0));
        assert_eq!(run(&mut e, 0x005a_3c30, 0, 0, 0), (true, -1.0));
    }

    #[test]
    fn c90_tests_mask_4_on_the_part() {
        let mut e = engine();
        let a = actor(&mut e);
        stub(&mut e, ACTOR_GET_PROCESS_PART, 0x6600);
        e.register_double(PROCESS_PART_TEST_FLAGS, |_, args| {
            assert_eq!(args, [0x6600, 4]);
            true.into_ret()
        });
        assert_eq!(run(&mut e, 0x005a_3c90, a, 0, 0), (true, 1.0));
        stub(&mut e, PROCESS_PART_TEST_FLAGS, 0);
        assert_eq!(run(&mut e, 0x005a_3c90, a, 0, 0), (true, 0.0));
        stub(&mut e, ACTOR_GET_PROCESS_PART, 0);
        assert_eq!(run(&mut e, 0x005a_3c90, a, 0, 0), (true, 0.0));
        let plain = non_actor(&mut e);
        assert_eq!(run(&mut e, 0x005a_3c90, plain, 0, 0), (true, 0.0));
    }
}
