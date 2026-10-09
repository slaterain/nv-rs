//! `fallout shared/tesconditionfunctions.cpp` (Xbox PDB source unit), subsystem `fallout shared`: its functions in
//! FalloutNV.exe 1.4.0.525, translated (docs/ENGINE_CRATE.md).
//!
//! The unit is the script condition functions: one `bool` function per
//! condition command (`GetDistance`, `GetPos`, `SameFaction`, ...), all with
//! the same shape `bool f(TESObjectREFR* ref, void* param1, void* param2,
//! double* result)` (cdecl). The function writes the condition's value to
//! `*result` and returns `true`; the command table in `.data` points at them.
//! Most start by writing `0.0` to `*result`, and all but a few finish with a
//! debug line through `00703c00` when the byte at `TLS + 0x268` is set.
//!
//! Progress: this file holds the first 40 functions of the unit in address
//! order (`0059bfa0` to `0059dbe0`, `0059c380` excepted: `crates/world`
//! already describes it and the ledger counts it as translated). The next
//! session continues with `0059dc90` (`GetSleeping`); keep new shared
//! helpers and constants in the block below, above the functions.
//!
//! x87 note: the game computes in extended precision and stores `double`
//! results. The translations compute in `f64`, which can differ from the
//! x87 only in the last bit of a result rounded twice.

#[allow(unused_imports)]
use crate::prelude::*;

/// Offset in the TLS block of the byte that turns on the condition
/// functions' debug lines (`*(*(FS:[0x2c] + _tls_index * 4) + 0x268)`).
const TLS_TRACE_FLAG: u32 = 0x268;
/// The debug print (`printf`-like, varargs, `interface.cpp`).
const DEBUG_PRINT: u32 = 0x0070_3c00;
/// `PlayerCharacter*` global.
const PLAYER: u32 = 0x011d_ea3c;
/// `TES*` global (`TES::Pick`'s `this`).
const TES: u32 = 0x011d_ea10;
/// The `Calendar` singleton.
const CALENDAR: u32 = 0x011d_e7b8;
/// The save/load global data object `GetSecondsPassed` falls back on.
const SECONDS_PASSED_SOURCE: u32 = 0x011f_6394;

/// `_ftol2_sse` (`00ec62c0`): truncates the value in ST0 (passed as a leading
/// `f64` argument) to an integer in EAX.
const FTOL: u32 = 0x00ec_62c0;
/// `operator delete` (`00401030`).
const OPERATOR_DELETE: u32 = 0x0040_1030;

/// Value of virtual `IsActor` (vtable `+0x100`, Xbox PDB name).
const VSLOT_IS_ACTOR: u32 = 0x100;
/// Virtual `IsBoundObject` (vtable `+0xe4`, Xbox PDB name).
const VSLOT_IS_BOUND_OBJECT: u32 = 0xe4;
/// Virtual `IsMobileObject` (vtable `+0xfc`, Xbox PDB name).
const VSLOT_IS_MOBILE_OBJECT: u32 = 0xfc;
/// Virtual on `TESObjectREFR` returning a pointer to its position (3 floats).
const VSLOT_GET_POSITION: u32 = 0x1f4;

/// Reads `[this + 0x20]` (`TESObjectREFR`'s base form; the engine map names
/// this body `BGSSaveFormBuffer::GetForm` because the linker folded the
/// identical code).
const REF_GET_BASE_FORM: u32 = 0x007a_f430;
/// Same body as [`REF_GET_BASE_FORM`], in `extradatalist.cpp`: the actor's
/// base form.
const ACTOR_GET_BASE_FORM: u32 = 0x0041_81e0;
/// `TESForm::cFormType` (byte at `+0x04`) of the form in ECX.
const FORM_GET_TYPE: u32 = 0x0040_1170;
/// True when the `TESForm` in ECX has flag `0x800` (`iFormFlags` at `+0x08`).
const FORM_HAS_FLAG_0800: u32 = 0x0044_0da0;
/// `TESObjectREFR::HasContainer` (Xbox PDB): non-zero when the reference's
/// base form holds items.
const REF_HAS_CONTAINER: u32 = 0x0055_d310;
/// Form type of an `NPC_` (`TESNPC`).
const FORM_TYPE_NPC: u32 = 0x2a;
/// Form type of a `FLST` (`BGSListForm`).
const FORM_TYPE_FORM_LIST: u32 = 0x55;
/// Form type of a `TERM` (`BGSTerminal`).
const FORM_TYPE_TERMINAL: u32 = 0x17;

/// `BGSListForm`'s list: returns `this + 0x18`.
const FORM_LIST_GET_LIST: u32 = 0x0050_0940;
/// `BSSimpleList` node helpers (`this` is the node): true for an empty
/// list's head node (item and next both null) ...
const LIST_NODE_IS_EMPTY: u32 = 0x0082_56d0;
/// ... the address of the node's item field (returns `this`) ...
const LIST_NODE_ITEM_ADDRESS: u32 = 0x0068_15c0;
/// ... and the next node (`[this + 4]`).
const LIST_NODE_NEXT: u32 = 0x0072_6070;

/// `InventoryChanges::GetInventoryChanges` (Xbox PDB), cdecl, one reference
/// argument.
const GET_INVENTORY_CHANGES: u32 = 0x004b_f220;
/// `InventoryChanges::GetObjectCount` (Xbox PDB name), `this` the inventory
/// changes, one item form argument.
const INVENTORY_GET_OBJECT_COUNT: u32 = 0x004c_8f30;
/// CRT function the item counts pass through (cdecl, one `int`).
const ITEM_COUNT_FILTER: u32 = 0x00ec_7d40;

/// True when the `Actor`'s middle-high process exists (`this` the actor;
/// the engine map names it `MiddleHighProcess::GetSavedAcquireObject`
/// because of identical code folding).
const ACTOR_GET_PROCESS: u32 = 0x008d_8520;
/// `ActorValue::GetActorValueScriptName` (Xbox PDB), cdecl.
const ACTOR_VALUE_SCRIPT_NAME: u32 = 0x0066_eac0;
/// `TESObjectREFR::GetScale` (Xbox PDB): the reference's scale with its base.
const REF_GET_SCALE: u32 = 0x0056_7400;
/// `[this + 0x3c]` as a `float` (the reference's own scale).
const REF_GET_OWN_SCALE: u32 = 0x0059_8040;
/// `TESObjectREFR::GetDistanceFromReference` (Xbox PDB), `this` and
/// (other, flag, flag).
const REF_GET_DISTANCE_FROM_REFERENCE: u32 = 0x0057_23b0;
/// `TESObjectREFR::GetLock` (Xbox PDB): the reference's `REFR_LOCK`, or null.
const REF_GET_LOCK: u32 = 0x0056_9160;
/// `Actor::LineOfSight` (Xbox PDB): `this` and (0, target, 1, 0, 0), byte
/// result.
const ACTOR_LINE_OF_SIGHT: u32 = 0x0088_b880;
/// The name of a reference (`tesobjectrefr.cpp`), used by `GetLOS`'s debug line.
const REF_GET_NAME: u32 = 0x0055_d520;

/// `57.295776` (`double`): degrees per radian, `GetAngle`'s multiplier.
const DEGREES_PER_RADIAN: u32 = 0x0102_f248;

/// The byte at `TLS + 0x268`: the condition functions print a debug line
/// for their result when it is set.
fn trace_enabled(e: &mut Engine) -> bool {
    let tls = e.tls();
    e.mem.u8(tls + TLS_TRACE_FLAG) != 0
}

/// The debug line `format(*result)`.
fn trace_result(e: &mut Engine, format: u32, result: Ptr) {
    let value = e.mem.f64(result.addr());
    e.call(DEBUG_PRINT, &args![format, value]);
}

/// The debug line `format(label, *result)`.
fn trace_labeled_result(e: &mut Engine, format: u32, label: u32, result: Ptr) {
    let value = e.mem.f64(result.addr());
    e.call(DEBUG_PRINT, &args![format, label, value]);
}

fn set_result(e: &mut Engine, result: Ptr, value: f64) {
    e.mem.set_f64(result.addr(), value);
}

/// `ref` when it is non-null and virtual `IsActor` says so, else null (the
/// game's `DYNAMIC_CAST<Actor*>`-style test every actor condition opens
/// with).
fn actor_of(e: &mut Engine, reference: Ptr) -> Ptr {
    if !reference.is_null() && e.vcall(reference.addr(), VSLOT_IS_ACTOR, &args![]).bool() {
        reference
    } else {
        Ptr::NULL
    }
}

/// The reference's base form when its type is `NPC_`, else null (the test
/// `SameRace` and `SameSex` run on both references).
fn npc_base_form_of(e: &mut Engine, reference: Ptr) -> Ptr {
    if reference.is_null() {
        return Ptr::NULL;
    }
    let base = e.call(REF_GET_BASE_FORM, &args![reference]).ptr::<()>();
    if e.call(FORM_GET_TYPE, &args![base]).u32() == FORM_TYPE_NPC {
        e.call(REF_GET_BASE_FORM, &args![reference]).ptr()
    } else {
        Ptr::NULL
    }
}

// Translated from 0059bfa0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Script::GetDistanceConditionFunction` (Xbox PDB): the distance between
/// the two references. Writes nothing when either is null.
pub fn script_get_distance_condition_function(
    e: &mut Engine,
    reference: Ptr,
    other: Ptr,
    _param2: u32,
    result: Ptr,
) -> bool {
    if !reference.is_null() && !other.is_null() {
        let distance = e
            .call(
                REF_GET_DISTANCE_FROM_REFERENCE,
                &args![reference, other, 1u32, 0u32],
            )
            .f64();
        set_result(e, result, distance);
    }
    if trace_enabled(e) {
        // "GetDistance >> %0.2f"
        trace_result(e, 0x0103_4d0c, result);
    }
    true
}

// Translated from 0059c010 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Script::GetInZoneConditionFunction` (Xbox PDB): 1.0 when the reference's
/// cell (or, failing that, its worldspace's cell) is the given one.
pub fn script_get_in_zone_condition_function(
    e: &mut Engine,
    reference: Ptr,
    zone: Ptr,
    _param2: u32,
    result: Ptr,
) -> bool {
    set_result(e, result, 0.0);
    if !reference.is_null() && !zone.is_null() {
        let mut found = Ptr::NULL;
        let parent_cell = e.call(0x008d_6f30, &args![reference]).ptr::<()>();
        if !parent_cell.is_null() {
            found = e.call(0x0054_6c20, &args![parent_cell]).ptr();
        }
        if found.is_null() {
            // TESObjectREFR::GetWorldSpace (Xbox PDB)
            let worldspace = e.call(0x0057_5d70, &args![reference]).ptr::<()>();
            if !worldspace.is_null() {
                found = e.call(0x0045_8400, &args![worldspace]).ptr();
            }
        }
        if found == zone {
            set_result(e, result, 1.0);
        }
    }
    if trace_enabled(e) {
        // "GetInZone >> %0.2f"
        trace_result(e, 0x0103_4d24, result);
    }
    true
}

// Translated from 0059c0c0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Script::GetPosConditionFunction` (Xbox PDB): one coordinate of the
/// reference's position, selected by the axis letter `'X'` (0x58), `'Y'` or
/// `'Z'`. Any other letter leaves `*result` as it was.
pub fn script_get_pos_condition_function(
    e: &mut Engine,
    reference: Ptr,
    axis: u32,
    _param2: u32,
    result: Ptr,
) -> bool {
    if !reference.is_null() {
        let position = e
            .vcall(reference.addr(), VSLOT_GET_POSITION, &args![])
            .u32();
        let coordinates = [
            e.mem.f32(position),
            e.mem.f32(position + 4),
            e.mem.f32(position + 8),
        ];
        match axis {
            0x58 => set_result(e, result, coordinates[0] as f64),
            0x59 => set_result(e, result, coordinates[1] as f64),
            0x5a => set_result(e, result, coordinates[2] as f64),
            _ => {}
        }
        if trace_enabled(e) {
            // "GetPos: %c >> %0.2f"
            trace_labeled_result(e, 0x0103_4d38, axis, result);
        }
    }
    true
}

// Translated from 0059c170 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Script::GetAngleConditionFunction` (Xbox PDB): one component of the
/// reference's rotation (`00430830` returns the three radians), in degrees.
pub fn script_get_angle_condition_function(
    e: &mut Engine,
    reference: Ptr,
    axis: u32,
    _param2: u32,
    result: Ptr,
) -> bool {
    if !reference.is_null() {
        let rotation = e.call(0x0043_0830, &args![reference]).u32();
        let angles = [
            e.mem.f32(rotation),
            e.mem.f32(rotation + 4),
            e.mem.f32(rotation + 8),
        ];
        let degrees_per_radian: f64 = e.global(DEGREES_PER_RADIAN);
        match axis {
            0x58 => set_result(e, result, angles[0] as f64 * degrees_per_radian),
            0x59 => set_result(e, result, angles[1] as f64 * degrees_per_radian),
            0x5a => set_result(e, result, angles[2] as f64 * degrees_per_radian),
            _ => {}
        }
        if trace_enabled(e) {
            // "GetAngle: %c >> %0.2f"
            trace_labeled_result(e, 0x0103_4d4c, axis, result);
        }
    }
    true
}

// Translated from 0059c230 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Script::GetStartingPosConditionFunction` (Xbox PDB): one coordinate of
/// the position the reference was placed at (virtual `+0x170` fills a
/// 3-float out buffer).
pub fn script_get_starting_pos_condition_function(
    e: &mut Engine,
    reference: Ptr,
    axis: u32,
    _param2: u32,
    result: Ptr,
) -> bool {
    if !reference.is_null() {
        let coordinates = e.with_stack(12, |e, buffer| {
            e.vcall(reference.addr(), 0x170, &args![buffer]);
            [
                e.mem.f32(buffer.addr()),
                e.mem.f32(buffer.addr() + 4),
                e.mem.f32(buffer.addr() + 8),
            ]
        });
        match axis {
            0x58 => set_result(e, result, coordinates[0] as f64),
            0x59 => set_result(e, result, coordinates[1] as f64),
            0x5a => set_result(e, result, coordinates[2] as f64),
            _ => {}
        }
        if trace_enabled(e) {
            // "GetStartingPos: %c >> %0.2f"
            trace_labeled_result(e, 0x0103_4d64, axis, result);
        }
    }
    true
}

// Translated from 0059c2d0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Script::GetStartingAngleConditionFunction` (Xbox PDB): one component of
/// the rotation the reference was placed with (virtual `+0x16c` fills a
/// 3-float out buffer), in degrees.
pub fn script_get_starting_angle_condition_function(
    e: &mut Engine,
    reference: Ptr,
    axis: u32,
    _param2: u32,
    result: Ptr,
) -> bool {
    if !reference.is_null() {
        let angles = e.with_stack(12, |e, buffer| {
            e.vcall(reference.addr(), 0x16c, &args![buffer]);
            [
                e.mem.f32(buffer.addr()),
                e.mem.f32(buffer.addr() + 4),
                e.mem.f32(buffer.addr() + 8),
            ]
        });
        let degrees_per_radian: f64 = e.global(DEGREES_PER_RADIAN);
        match axis {
            0x58 => set_result(e, result, angles[0] as f64 * degrees_per_radian),
            0x59 => set_result(e, result, angles[1] as f64 * degrees_per_radian),
            0x5a => set_result(e, result, angles[2] as f64 * degrees_per_radian),
            _ => {}
        }
        if trace_enabled(e) {
            // "GetStartingAngle: %c >> %0.2f"
            trace_labeled_result(e, 0x0103_4d80, axis, result);
        }
    }
    true
}

// Translated from 0059c430 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Script::GetSecondsPassedConditionFunction` (Xbox PDB): when the second
/// argument (`source`) is set, the byte at `+0x28` of it (`00500940` returns
/// `source + 0x18`, the byte is at `+0x10` of that) is non-zero and the
/// float `00598040` reads from it (`[source + 0x3c]`) is positive, that
/// float; otherwise the value `0084d030` returns for the global data object
/// `011f6394`. The first argument is unused.
pub fn script_get_seconds_passed_condition_function(
    e: &mut Engine,
    _reference: Ptr,
    source: Ptr,
    _param2: u32,
    result: Ptr,
) -> bool {
    let mut from_source = false;
    if !source.is_null() {
        let record = e.call(FORM_LIST_GET_LIST, &args![source]).u32();
        // Byte at +0x10 of the object `00500940` returns (`source + 0x18`).
        if e.mem.u8(record + 0x10) != 0 {
            let value = e.call(REF_GET_OWN_SCALE, &args![source]).f64();
            let zero: f64 = e.global(0x0101_2060);
            if value > zero {
                let value = e.call(REF_GET_OWN_SCALE, &args![source]).f64();
                set_result(e, result, value);
                from_source = true;
            }
        }
    }
    if !from_source {
        let value = e.call(0x0084_d030, &args![SECONDS_PASSED_SOURCE]).f64();
        set_result(e, result, value);
    }
    if trace_enabled(e) {
        // "GetSecondsPassed >> %0.2f"
        trace_result(e, 0x0103_4db8, result);
    }
    true
}

// Translated from 0059c4c0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Condition function without debug line: 1.0 when virtual `+0x1d0` of the
/// reference returns non-null, else 0.0.
pub fn fn_0059c4c0(
    e: &mut Engine,
    reference: Ptr,
    _param1: u32,
    _param2: u32,
    result: Ptr,
) -> bool {
    set_result(e, result, 0.0);
    if !reference.is_null() && e.vcall(reference.addr(), 0x1d0, &args![]).u32() != 0 {
        set_result(e, result, 1.0);
    }
    true
}

// Translated from 0059c4f0 (decompiled, FalloutNV.exe 1.4.0.525)
/// The `GetActorValue` condition function: the actor's current value of
/// actor value `actor_value`, read through the `ActorValueOwner` embedded at
/// `+0xa4` of the actor (virtual `+0xc`), or of its base form's owner at
/// `+0x100` when the actor has form flag `0x800`. Writes nothing for a
/// non-actor.
pub fn fn_0059c4f0(
    e: &mut Engine,
    reference: Ptr,
    actor_value: u32,
    _param2: u32,
    result: Ptr,
) -> bool {
    let actor = actor_of(e, reference);
    if !actor.is_null() {
        let value = if !e.call(FORM_HAS_FLAG_0800, &args![actor]).bool() {
            // ActorValueOwner at Actor+0xa4
            e.vcall(actor.addr() + 0xa4, 0xc, &args![actor_value]).f64()
        } else {
            let base = e.call(ACTOR_GET_BASE_FORM, &args![actor]).u32();
            // ActorValueOwner at TESActorBase+0x100
            e.vcall(base + 0x100, 0xc, &args![actor_value]).f64()
        };
        set_result(e, result, value);
        if trace_enabled(e) {
            let name = e.call(ACTOR_VALUE_SCRIPT_NAME, &args![actor_value]).u32();
            // "GetActorValue: %s >> %0.2f"
            trace_labeled_result(e, 0x0103_4dd4, name, result);
        }
    }
    true
}

// Translated from 0059c5d0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Script::GetBaseActorValueConditionFunction` (Xbox PDB): the actor's base
/// value of an actor value (the integer returned by the `ActorValueOwner` at
/// `+0xa4`, virtual `+0x0`). Writes nothing for a non-actor.
pub fn script_get_base_actor_value_condition_function(
    e: &mut Engine,
    reference: Ptr,
    actor_value: u32,
    _param2: u32,
    result: Ptr,
) -> bool {
    let actor = actor_of(e, reference);
    if !actor.is_null() {
        let value = e.vcall(actor.addr() + 0xa4, 0, &args![actor_value]).i32();
        set_result(e, result, value as f64);
        if trace_enabled(e) {
            let name = e.call(ACTOR_VALUE_SCRIPT_NAME, &args![actor_value]).u32();
            // "GetBaseActorValue: %s >> %0.2f"
            trace_labeled_result(e, 0x0103_4df0, name, result);
        }
    }
    true
}

// Translated from 0059c680 (decompiled, FalloutNV.exe 1.4.0.525)
/// The `GetPermanentActorValue` condition function: the same as
/// [`fn_0059c4f0`] through virtual `+0x20` of the `ActorValueOwner` (and
/// with the same `GetActorValue` debug line).
pub fn fn_0059c680(
    e: &mut Engine,
    reference: Ptr,
    actor_value: u32,
    _param2: u32,
    result: Ptr,
) -> bool {
    let actor = actor_of(e, reference);
    if !actor.is_null() {
        let value = if !e.call(FORM_HAS_FLAG_0800, &args![actor]).bool() {
            e.vcall(actor.addr() + 0xa4, 0x20, &args![actor_value])
                .f64()
        } else {
            let base = e.call(ACTOR_GET_BASE_FORM, &args![actor]).u32();
            e.vcall(base + 0x100, 0x20, &args![actor_value]).f64()
        };
        set_result(e, result, value);
        if trace_enabled(e) {
            let name = e.call(ACTOR_VALUE_SCRIPT_NAME, &args![actor_value]).u32();
            // "GetActorValue: %s >> %0.2f"
            trace_labeled_result(e, 0x0103_4dd4, name, result);
        }
    }
    true
}

// Translated from 0059c760 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Script::GetFatiguePercentageConditionFunction` (Xbox PDB): the actor's
/// fatigue percentage (`Actor::GetFatiguePercentage`). Writes nothing for a
/// non-actor.
pub fn script_get_fatigue_percentage_condition_function(
    e: &mut Engine,
    reference: Ptr,
    _param1: u32,
    _param2: u32,
    result: Ptr,
) -> bool {
    let actor = actor_of(e, reference);
    if !actor.is_null() {
        let value = e.call(0x0089_3530, &args![actor]).f64();
        set_result(e, result, value);
        if trace_enabled(e) {
            // "GetFatiguePercentage >> %0.2f"
            trace_result(e, 0x0103_4e10, result);
        }
    }
    true
}

// Translated from 0059c7e0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Script::GetHealthPercentageConditionFunction` (Xbox PDB): the actor's
/// health percentage (`Actor::GetHealthPercentage`). Writes nothing for a
/// non-actor.
pub fn script_get_health_percentage_condition_function(
    e: &mut Engine,
    reference: Ptr,
    _param1: u32,
    _param2: u32,
    result: Ptr,
) -> bool {
    let actor = actor_of(e, reference);
    if !actor.is_null() {
        let value = e.call(0x0089_3590, &args![actor]).f64();
        set_result(e, result, value);
        if trace_enabled(e) {
            // "GetHealthPercentage >> %0.2f"
            trace_result(e, 0x0103_4e30, result);
        }
    }
    true
}

// Translated from 0059c860 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Script::GetWalkSpeedConditionFunction` (Xbox PDB): the actor's walk
/// speed (`Actor::GetWalkSpeed`). Writes nothing for a non-actor.
pub fn script_get_walk_speed_condition_function(
    e: &mut Engine,
    reference: Ptr,
    _param1: u32,
    _param2: u32,
    result: Ptr,
) -> bool {
    let actor = actor_of(e, reference);
    if !actor.is_null() {
        let value = e.call(0x0088_4dc0, &args![actor]).f64();
        set_result(e, result, value);
        if trace_enabled(e) {
            // "GetWalkSpeed >> %0.2f"
            trace_result(e, 0x0103_4e50, result);
        }
    }
    true
}

// Translated from 0059c8e0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Script::GetCurrentTimeConditionFunction` (Xbox PDB): the game hour
/// (`Calendar::GetHour` on the calendar singleton). Uses no parameter.
pub fn script_get_current_time_condition_function(
    e: &mut Engine,
    _reference: Ptr,
    _param1: u32,
    _param2: u32,
    result: Ptr,
) -> bool {
    let hour = e.call(0x0086_7da0, &args![CALENDAR]).f64();
    set_result(e, result, hour);
    if trace_enabled(e) {
        // "GetCurrentTime >> %0.2f"
        trace_result(e, 0x0103_4e68, result);
    }
    true
}

// Translated from 0059c930 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Script::GetScaleConditionFunction` (Xbox PDB): the reference's own scale
/// (`[ref + 0x3c]`); the debug line also prints `TESObjectREFR::GetScale`.
pub fn script_get_scale_condition_function(
    e: &mut Engine,
    reference: Ptr,
    _param1: u32,
    _param2: u32,
    result: Ptr,
) -> bool {
    if !reference.is_null() {
        let scale = e.call(REF_GET_OWN_SCALE, &args![reference]).f64();
        set_result(e, result, scale);
        if trace_enabled(e) {
            let with_base = e.call(REF_GET_SCALE, &args![reference]).f64();
            let shown = e.mem.f64(result.addr());
            // "GetScale >> %0.2f (with base %0.2f)"
            e.call(DEBUG_PRINT, &args![0x0103_4e80u32, shown, with_base]);
        }
    }
    true
}

/// The player case of `GetLOS` (the body of `if (actor == PlayerCharacter)`
/// in `0059c990`): the player sees `target` when the target is in the
/// player's view (an animation-data test on the target's bound, or on a
/// bound built from its position), and then a ray from the player's eye is
/// not blocked at one of three heights, or `Actor::LineOfSight` holds.
///
/// `frame` is the game's stack frame: the original keeps these locals at
/// fixed `ebp` offsets, and this keeps them there so that the blocks are as
/// far apart as the game lays them out.
fn get_los_player_case(e: &mut Engine, player: Ptr, target: Ptr, result: Ptr) {
    let mut visible = false;
    let tes_singleton = e.call(0x0045_c670, &args![]).u32();
    let tes_cell = e.call(0x0055_8310, &args![tes_singleton]).u32();
    let tes_singleton = e.call(0x0045_c670, &args![]).u32();
    // BSFaceGenNiNode::GetAnimationData (Xbox PDB) on that object.
    let animation_data = e.call(0x0066_29f0, &args![tes_singleton]).u32();

    let bound_owner = if e.vcall(target.addr(), 0x1d0, &args![]).u32() != 0 {
        let owner = e.vcall(target.addr(), 0x1d0, &args![]).u32();
        e.vcall(owner, 0xc, &args![]).u32()
    } else {
        0
    };
    if bound_owner != 0
        && e.call(0x004b_5fc0, &args![bound_owner, animation_data])
            .bool()
    {
        visible = true;
    } else if e.call(0x0044_4ed0, &args![target]).bool() {
        // A 16-byte bound (centre and radius) built on the stack.
        e.with_stack(0x10, |e, bound| {
            e.call(0x0062_40d0, &args![bound]);
            let position = e.vcall(target.addr(), VSLOT_GET_POSITION, &args![]).u32();
            e.call(0x0098_ddd0, &args![bound, position]);
            e.call(0x0063_f790, &args![bound, 1.0f32]);
            if e.call(0x004b_5ff0, &args![bound, animation_data]).bool() {
                visible = true;
            }
        });
    }
    if !visible {
        return;
    }

    let mut found = false;
    e.with_stack(0x200, |e, frame| {
        let ebp = frame.addr() + 0x200;
        let ray_origin = ebp - 0x50;
        let pick_data = ebp - 0x100;
        let filter_target = ebp - 0x104;
        let collector = ebp - 0x190;
        let ray_end = ebp - 0x19c;
        let ray_end_z = ebp - 0x194;
        let filter_bits = ebp - 0x1a0;
        let filter_info = ebp - 0x1a4;
        let bound_max_buffer = ebp - 0x1b0;
        let bound_min_buffer = ebp - 0x1bc;

        let origin = e.call(0x0043_c490, &args![tes_cell]).u32();
        for i in 0..3 {
            let word = e.mem.u32(origin + 4 * i);
            e.mem.set_u32(ray_origin + 4 * i, word);
        }
        // The pick data (a 0xb0-byte object, constructed by 004a3c20).
        e.call(0x004a_3c20, &args![pick_data]);
        e.mem.set_u32(filter_target, 0);
        if e.vcall(target.addr(), VSLOT_IS_MOBILE_OBJECT, &args![])
            .bool()
        {
            e.mem.set_u32(filter_target, target.addr());
        }
        let filter = e.mem.u32(filter_target);
        // The ray-hit collector built for collision layer 0x25 and the filter.
        e.call(0x0062_a190, &args![collector, 0x25u32, filter]);
        fn_0059ceb0(e, Ptr::new(pick_data), Ptr::new(collector));

        let position = e.vcall(target.addr(), VSLOT_GET_POSITION, &args![]).u32();
        for i in 0..3 {
            let word = e.mem.u32(position + 4 * i);
            e.mem.set_u32(ray_end + 4 * i, word);
        }
        e.call(0x004a_3da0, &args![pick_data, ray_origin]);
        e.call(0x0093_1ed0, &args![player, filter_bits]);
        e.call(0x008c_71b0, &args![filter_info, 0u32]);
        e.call(0x004a_39f0, &args![filter_info, 0x25u32]);
        let high_bits = e.call(0x004a_3a20, &args![filter_bits]).u32();
        fn_0059ce80(e, Ptr::new(filter_info), high_bits);
        let info = e.mem.u32(filter_info);
        e.call(0x004a_3f70, &args![pick_data, info]);

        let bound_max = e
            .vcall(target.addr(), 0x1dc, &args![bound_max_buffer])
            .u32();
        let bound_min = e
            .vcall(target.addr(), 0x1d8, &args![bound_min_buffer])
            .u32();
        let height = (e.mem.f32(bound_max + 8) as f64 - e.mem.f32(bound_min + 8) as f64) as f32;
        let eye_z = e.mem.f32(ray_end_z);
        let fractions = [0x0101_de30u32, 0x0101_1588, 0x0102_90b0];
        let mut heights = [0f32; 3];
        for (slot, fraction) in heights.iter_mut().zip(fractions) {
            let fraction: f64 = e.global(fraction);
            *slot = (height as f64 * fraction + eye_z as f64) as f32;
        }

        let tes: u32 = e.global(TES);
        for height in heights {
            e.mem.set_f32(ray_end_z, height);
            e.call(0x004a_3eb0, &args![pick_data, ray_end]);
            let picked = e.call(0x0045_8420, &args![tes, pick_data]).u32();
            let picked_reference = if picked != 0 {
                e.call(0x0056_f930, &args![picked]).u32()
            } else {
                0
            };
            if picked == 0 || picked_reference == target.addr() {
                found = true;
                break;
            }
        }

        if found {
            set_result(e, result, 1.0);
        } else {
            let seen = e
                .call(
                    ACTOR_LINE_OF_SIGHT,
                    &args![player, 0u32, target, 1u32, 0u32, 0u32],
                )
                .u8();
            set_result(e, result, seen as f64);
        }
        fn_0059cee0(e, Ptr::new(collector));
    });
}

// Translated from 0059c990 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Script::GetLOSConditionFunction` (Xbox PDB): 1.0 when the reference (an
/// actor) has line of sight to the target. For the player this is first a
/// view test and ray casts ([`get_los_player_case`]); for any other actor
/// it is `Actor::LineOfSight`. Writes 0.0 otherwise.
///
/// The compiler's exception-unwinding frame and the stack cookie are not
/// translated.
pub fn script_get_los_condition_function(
    e: &mut Engine,
    reference: Ptr,
    target: Ptr,
    _param2: u32,
    result: Ptr,
) -> bool {
    set_result(e, result, 0.0);
    if reference.is_null() {
        return true;
    }
    let viewer = actor_of(e, reference);
    if viewer.is_null() || target.is_null() {
        return true;
    }
    let player: u32 = e.global(PLAYER);
    if viewer.addr() == player {
        get_los_player_case(e, viewer, target, result);
    } else {
        let seen = e
            .call(
                ACTOR_LINE_OF_SIGHT,
                &args![viewer, 0u32, target, 1u32, 0u32, 0u32],
            )
            .u8();
        set_result(e, result, seen as f64);
    }
    if trace_enabled(e) {
        let sees = e.mem.f64(result.addr()) != 0.0;
        let target_name = e.call(REF_GET_NAME, &args![target]).u32();
        let viewer_name = e.call(REF_GET_NAME, &args![viewer]).u32();
        // "%s sees %s" / "%s can't see %s"
        let format = if sees { 0x0103_4eb4u32 } else { 0x0103_4ea4 };
        e.call(DEBUG_PRINT, &args![format, viewer_name, target_name]);
    }
    true
}

// Translated from 0059ce80 (decompiled, FalloutNV.exe 1.4.0.525)
/// Sets the upper 16 bits of the word at `this` (the pick filter's
/// collision-group field), keeping the lower 16.
pub fn fn_0059ce80(e: &mut Engine, this: Ptr, high: u32) {
    let low = e.mem.u32(this.addr()) & 0xffff;
    e.mem.set_u32(this.addr(), low);
    e.mem.set_u32(this.addr(), (high << 16) | low);
}

// Translated from 0059ceb0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Gives the pick data (`this`) its ray-hit collector: stores the collector
/// at `+0xa4` and clears `+0xa8`.
pub fn fn_0059ceb0(e: &mut Engine, this: Ptr, collector: Ptr) {
    e.mem.set_u32(this.addr() + 0xa4, collector.addr());
    e.mem.set_u32(this.addr() + 0xa8, 0);
}

// Translated from 0059cee0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Destructor of the ray-hit collector `GetLOS` builds (calls `0059cf00`).
pub fn fn_0059cee0(e: &mut Engine, this: Ptr) {
    fn_0059cf00(e, this);
}

// Translated from 0059cf00 (decompiled, FalloutNV.exe 1.4.0.525)
/// The collector's destructor body: puts back the vtable of its own class
/// (`01034ec4`, the table with `addRayHit`) and runs the base destructor
/// (`004a3ae0`).
pub fn fn_0059cf00(e: &mut Engine, this: Ptr) {
    e.mem.set_u32(this.addr(), 0x0103_4ec4);
    e.call(0x004a_3ae0, &args![this]);
}

// Translated from 0059cf20 (decompiled, FalloutNV.exe 1.4.0.525)
/// The collector's scalar deleting destructor: destroys it, and frees the
/// memory when bit 0 of `flags` is set. Returns `this`.
pub fn fn_0059cf20(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    fn_0059cf00(e, this);
    if flags & 1 != 0 {
        e.call(OPERATOR_DELETE, &args![this]);
    }
    this
}

// Translated from 0059cf50 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Script::GetDisabledConditionFunction` (Xbox PDB): with a reference, 1.0
/// when it is disabled (form flag `0x800` and not pending enable, or pending
/// disable). Without one, bit 0 of the byte at `+0x04` of the parameter
/// record.
pub fn script_get_disabled_condition_function(
    e: &mut Engine,
    reference: Ptr,
    record: Ptr,
    _param2: u32,
    result: Ptr,
) -> bool {
    if !reference.is_null() {
        set_result(e, result, 0.0);
        let disabled = (e.call(FORM_HAS_FLAG_0800, &args![reference]).bool()
            && !e.call(0x005a_a680, &args![reference]).bool())
            || e.call(0x005a_a630, &args![reference]).bool();
        if disabled {
            set_result(e, result, 1.0);
        }
    } else if !record.is_null() {
        // Byte at +0x04 of the record (where `TESForm::cFormType` sits).
        let bit = e.mem.u8(record.addr() + 4) & 1 != 0;
        set_result(e, result, bit as u8 as f64);
    }
    if trace_enabled(e) {
        // "GetDisabled >> %0.f"
        trace_result(e, 0x0103_4ecc, result);
    }
    true
}

// Translated from 0059d010 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Script::GetLockedConditionFunction` (Xbox PDB): 2.0 for a broken lock
/// (or a terminal the player has fully hacked into), 1.0 for a locked lock
/// (or a terminal that is not unlocked), else 0.0.
pub fn script_get_locked_condition_function(
    e: &mut Engine,
    reference: Ptr,
    _param1: u32,
    _param2: u32,
    result: Ptr,
) -> bool {
    set_result(e, result, 0.0);
    if !reference.is_null() {
        let lock = e.call(REF_GET_LOCK, &args![reference]).ptr::<()>();
        if !lock.is_null() {
            // REFR_LOCK::IsBroken (Xbox PDB)
            if e.call(0x0043_0ae0, &args![lock]).bool() {
                let two: f64 = e.global(0x0101_1590);
                set_result(e, result, two);
            } else if e.call(0x0050_21a0, &args![lock]).bool() {
                set_result(e, result, 1.0);
            }
        } else {
            let base = e.call(REF_GET_BASE_FORM, &args![reference]).ptr::<()>();
            if e.call(FORM_GET_TYPE, &args![base]).u32() == FORM_TYPE_TERMINAL {
                // PlayerCharacter::GetTerminalAccess (Xbox PDB)
                let player: u32 = e.global(PLAYER);
                let access = e.call(0x0096_6c60, &args![player, reference]).u32();
                if access == 2 {
                    let two: f64 = e.global(0x0101_1590);
                    set_result(e, result, two);
                } else {
                    let terminal = e.call(REF_GET_BASE_FORM, &args![reference]).u32();
                    // BGSTerminal::IsUnlocked (Xbox PDB)
                    if !e.call(0x0050_1ae0, &args![terminal, reference]).bool() {
                        set_result(e, result, 1.0);
                    }
                }
            }
        }
    }
    if trace_enabled(e) {
        // "GetLocked >> %0.f"
        trace_result(e, 0x0103_4ee0, result);
    }
    true
}

// Translated from 0059d100 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Script::GetLockLevelConditionFunction` (Xbox PDB): the lock's level
/// (`REFR_LOCK::GetLevel`), or for a terminal -1.0 when unlocked and its hack
/// difficulty's lock level otherwise. 0.0 for anything else.
pub fn script_get_lock_level_condition_function(
    e: &mut Engine,
    reference: Ptr,
    _param1: u32,
    _param2: u32,
    result: Ptr,
) -> bool {
    set_result(e, result, 0.0);
    if !reference.is_null() {
        let lock = e.call(REF_GET_LOCK, &args![reference]).ptr::<()>();
        if !lock.is_null() {
            // REFR_LOCK::GetLevel (Xbox PDB), with the reference
            let level = e.call(0x0043_0a10, &args![lock, reference]).i32();
            set_result(e, result, level as f64);
        } else {
            let base = e.call(REF_GET_BASE_FORM, &args![reference]).ptr::<()>();
            if e.call(FORM_GET_TYPE, &args![base]).u32() == FORM_TYPE_TERMINAL {
                let terminal = e.call(REF_GET_BASE_FORM, &args![reference]).u32();
                // BGSTerminal::IsUnlocked (Xbox PDB)
                if e.call(0x0050_1ae0, &args![terminal, reference]).bool() {
                    let minus_one: f64 = e.global(0x0101_a6b0);
                    set_result(e, result, minus_one);
                } else {
                    // BGSTerminal::GetHackDifficultyLockLevel (Xbox PDB)
                    let level = e.call(0x0050_11a0, &args![terminal, reference]).i32();
                    set_result(e, result, level as f64);
                }
            }
        }
    }
    if trace_enabled(e) {
        // "GetLockLevel >> %0.f"
        trace_result(e, 0x0103_4ef4, result);
    }
    true
}

// Translated from 0059d1d0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Script::GetIsLockBrokenConditionFunction` (Xbox PDB): 1.0 when the
/// reference has a broken lock, else 0.0.
pub fn script_get_is_lock_broken_condition_function(
    e: &mut Engine,
    reference: Ptr,
    _param1: u32,
    _param2: u32,
    result: Ptr,
) -> bool {
    set_result(e, result, 0.0);
    if !reference.is_null() {
        let lock = e.call(REF_GET_LOCK, &args![reference]).ptr::<()>();
        if !lock.is_null() {
            // REFR_LOCK::IsBroken (Xbox PDB)
            let broken = e.call(0x0043_0ae0, &args![lock]).u8();
            set_result(e, result, broken as f64);
        }
    }
    if trace_enabled(e) {
        // "GetIsLockBroken >> %0.f"
        trace_result(e, 0x0103_4f0c, result);
    }
    true
}

// Translated from 0059d250 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Script::GetDiseaseConditionFunction` (Xbox PDB): not implemented in the
/// game, always 0.0 (its debug line says `UNIMPLEMENTED`).
pub fn script_get_disease_condition_function(
    e: &mut Engine,
    _reference: Ptr,
    _param1: u32,
    _param2: u32,
    result: Ptr,
) -> bool {
    set_result(e, result, 0.0);
    if trace_enabled(e) {
        // "UNIMPLEMENTED: GetDisease >> %0.2f"
        trace_result(e, 0x0103_4f24, result);
    }
    true
}

// Translated from 0059d2a0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Script::GetVampireConditionFunction` (Xbox PDB): 1.0 when the actor is a
/// vampire (`0047c850`), else 0.0.
pub fn script_get_vampire_condition_function(
    e: &mut Engine,
    reference: Ptr,
    _param1: u32,
    _param2: u32,
    result: Ptr,
) -> bool {
    set_result(e, result, 0.0);
    let actor = actor_of(e, reference);
    if !actor.is_null() && e.call(0x0047_c850, &args![actor]).bool() {
        set_result(e, result, 1.0);
    }
    if trace_enabled(e) {
        // "GetVampire >> %0.2f"
        trace_result(e, 0x0103_4f48, result);
    }
    true
}

// Translated from 0059d320 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Script::GetClothingValueConditionFunction` (Xbox PDB): the character's
/// clothing value (`Character::GetClothingValue`) when the reference's base
/// form is an `NPC_`; leaves `*result` as it was otherwise.
pub fn script_get_clothing_value_condition_function(
    e: &mut Engine,
    reference: Ptr,
    _param1: u32,
    _param2: u32,
    result: Ptr,
) -> bool {
    let mut character = Ptr::NULL;
    if !reference.is_null() {
        let base = e.call(REF_GET_BASE_FORM, &args![reference]).ptr::<()>();
        if e.call(FORM_GET_TYPE, &args![base]).u32() == FORM_TYPE_NPC {
            character = reference;
        }
    }
    if !character.is_null() {
        let value = e.call(0x008d_3110, &args![character]).f64();
        set_result(e, result, value);
    }
    if trace_enabled(e) {
        // "GetClothingValue >> %0.2f"
        trace_result(e, 0x0103_4f5c, result);
    }
    true
}

// Translated from 0059d3a0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `SameFaction` against the player: calls
/// [`script_same_faction_condition_function`] with the player as the second
/// reference.
pub fn fn_0059d3a0(
    e: &mut Engine,
    reference: Ptr,
    _param1: u32,
    _param2: u32,
    result: Ptr,
) -> bool {
    let player = Ptr::new(e.global::<u32>(PLAYER));
    script_same_faction_condition_function(e, reference, player, 0, result)
}

// Translated from 0059d3c0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `SameRace` against the player (wrapper around
/// [`script_same_race_condition_function`]).
pub fn fn_0059d3c0(
    e: &mut Engine,
    reference: Ptr,
    _param1: u32,
    _param2: u32,
    result: Ptr,
) -> bool {
    let player = Ptr::new(e.global::<u32>(PLAYER));
    script_same_race_condition_function(e, reference, player, 0, result)
}

// Translated from 0059d3e0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `SameSex` against the player (wrapper around
/// [`script_same_sex_condition_function`]).
pub fn fn_0059d3e0(
    e: &mut Engine,
    reference: Ptr,
    _param1: u32,
    _param2: u32,
    result: Ptr,
) -> bool {
    let player = Ptr::new(e.global::<u32>(PLAYER));
    script_same_sex_condition_function(e, reference, player, 0, result)
}

// Translated from 0059d400 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Script::SameFactionConditionFunction` (Xbox PDB): 1.0 when some faction
/// of the first actor's base data has a rank (not -1) in the second
/// actor's base data (`TESActorBaseData::GetFactionRank`, which takes
/// whether the second reference is the player). Returns without a debug
/// line when either reference is not usable.
pub fn script_same_faction_condition_function(
    e: &mut Engine,
    reference: Ptr,
    other: Ptr,
    _param2: u32,
    result: Ptr,
) -> bool {
    set_result(e, result, 0.0);
    let actor = actor_of(e, reference);
    if actor.is_null() || other.is_null() {
        return true;
    }
    let actor_base = e.call(ACTOR_GET_BASE_FORM, &args![actor]).u32();
    let other_base = e.call(ACTOR_GET_BASE_FORM, &args![other]).u32();
    if actor_base != 0 && other_base != 0 {
        // The faction list of the actor's base data (+0x30).
        let mut node = e.call(0x005d_8a70, &args![actor_base + 0x30]).ptr::<()>();
        while !node.is_null()
            && !e.call(LIST_NODE_IS_EMPTY, &args![node]).bool()
            && e.mem.f64(result.addr()) != 1.0
        {
            let item_address = e.call(LIST_NODE_ITEM_ADDRESS, &args![node]).u32();
            // The node's item is a faction rank record whose first field is
            // the faction (read without a null check, as the game does).
            let faction_rank = e.mem.u32(item_address);
            let faction = e.mem.u32(faction_rank);
            if faction != 0 {
                let is_player = other.addr() == e.global::<u32>(PLAYER);
                let rank = e
                    .call(0x0047_d680, &args![other_base + 0x30, faction, is_player])
                    .i32();
                if rank != -1 {
                    set_result(e, result, 1.0);
                }
            }
            node = e.call(LIST_NODE_NEXT, &args![node]).ptr();
        }
    }
    if trace_enabled(e) {
        // "SameFaction >> %0.2f"
        trace_result(e, 0x0103_4f78, result);
    }
    true
}

// Translated from 0059d540 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Script::SameRaceConditionFunction` (Xbox PDB): 1.0 when both references
/// are `NPC_`s of the same race (`004ac110`).
pub fn script_same_race_condition_function(
    e: &mut Engine,
    reference: Ptr,
    other: Ptr,
    _param2: u32,
    result: Ptr,
) -> bool {
    set_result(e, result, 0.0);
    let first = npc_base_form_of(e, reference);
    let second = npc_base_form_of(e, other);
    if !first.is_null() && !second.is_null() {
        let first_race = e.call(0x004a_c110, &args![first]).u32();
        let second_race = e.call(0x004a_c110, &args![second]).u32();
        if first_race == second_race {
            set_result(e, result, 1.0);
        }
    }
    if trace_enabled(e) {
        // "SameRace >> %0.2f"
        trace_result(e, 0x0103_4f90, result);
    }
    true
}

// Translated from 0059d610 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Script::SameSexConditionFunction` (Xbox PDB): 1.0 when both references
/// are `NPC_`s of the same sex (`TESActorBase::GetSex`, `005f0cc0`).
pub fn script_same_sex_condition_function(
    e: &mut Engine,
    reference: Ptr,
    other: Ptr,
    _param2: u32,
    result: Ptr,
) -> bool {
    set_result(e, result, 0.0);
    let first = npc_base_form_of(e, reference);
    let second = npc_base_form_of(e, other);
    if !first.is_null() && !second.is_null() {
        let first_sex = e.call(0x005f_0cc0, &args![first]).i32();
        let second_sex = e.call(0x005f_0cc0, &args![second]).i32();
        if first_sex == second_sex {
            set_result(e, result, 1.0);
        }
    }
    if trace_enabled(e) {
        // "SameSex >> %0.2f"
        trace_result(e, 0x0103_4fa4, result);
    }
    true
}

// Translated from 0059d6e0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Script::GetDetectedConditionFunction` (Xbox PDB): 1.0 when the first
/// actor detects the second (`Actor::GetDetectionLevelAgainstActor` above
/// 0). The debug line also prints the second actor's light level (virtual
/// `+0x734` of its process, truncated).
pub fn script_get_detected_condition_function(
    e: &mut Engine,
    reference: Ptr,
    target: Ptr,
    _param2: u32,
    result: Ptr,
) -> bool {
    set_result(e, result, 0.0);
    let mut detection_level = 0i32;
    let mut light_level = -1i32;
    let detector = actor_of(e, reference);
    let subject = actor_of(e, target);
    if !detector.is_null()
        && !subject.is_null()
        && !e
            .call(ACTOR_GET_PROCESS, &args![detector])
            .ptr::<()>()
            .is_null()
        && !e
            .call(ACTOR_GET_PROCESS, &args![subject])
            .ptr::<()>()
            .is_null()
    {
        let player: u32 = e.global(PLAYER);
        // Two out bytes on the stack: the detection call's flag at +0 and
        // the player's in-combat out flag at +2.
        detection_level = e.with_stack(4, |e, out| {
            let in_combat = if subject.addr() == player {
                e.call(0x0095_3c50, &args![player, out.byte_add(2)]).u8()
            } else {
                e.call(0x0049_3bb0, &args![subject]).u8()
            };
            e.call(
                0x008a_0d10,
                &args![detector, 0u32, subject, out, 0u32, in_combat, 0u32, 0u32],
            )
            .i32()
        });
        if detection_level > 0 {
            set_result(e, result, 1.0);
        }
        let process = e.call(ACTOR_GET_PROCESS, &args![subject]).u32();
        let light = e.vcall(process, 0x734, &args![]).f64();
        light_level = e.call(FTOL, &args![light]).i32();
    }
    if trace_enabled(e) {
        // "GetDetected >> %i and light %i"
        e.call(
            DEBUG_PRINT,
            &args![0x0103_4fb8u32, detection_level, light_level],
        );
    }
    true
}

// Translated from 0059d840 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Script::GetDeadConditionFunction` (Xbox PDB): 1.0 when the actor is
/// dead (virtual `+0x22c` with 1).
pub fn script_get_dead_condition_function(
    e: &mut Engine,
    reference: Ptr,
    _param1: u32,
    _param2: u32,
    result: Ptr,
) -> bool {
    set_result(e, result, 0.0);
    let actor = actor_of(e, reference);
    if !actor.is_null() && e.vcall(actor.addr(), 0x22c, &args![1u32]).bool() {
        set_result(e, result, 1.0);
    }
    if trace_enabled(e) {
        // "GetDead >> %0.2f"
        trace_result(e, 0x0103_4fd8, result);
    }
    true
}

// Translated from 0059d8e0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Script::GetItemCountConditionFunction` (Xbox PDB): how many of an item
/// the container reference holds (`InventoryChanges::GetObjectCount`); for a
/// form list the sum over its items. A reference without a container leaves
/// `*result` at 0.0 and prints "Calling Reference is not a Container Object".
pub fn script_get_item_count_condition_function(
    e: &mut Engine,
    reference: Ptr,
    form: Ptr,
    _param2: u32,
    result: Ptr,
) -> bool {
    set_result(e, result, 0.0);
    if e.call(REF_HAS_CONTAINER, &args![reference]).u32() == 0 {
        if trace_enabled(e) {
            // "Calling Reference is not a Container Object ..."
            e.call(DEBUG_PRINT, &args![0x0103_5004u32]);
        }
        return true;
    }
    let mut item = Ptr::NULL;
    if !form.is_null() && e.vcall(form.addr(), VSLOT_IS_BOUND_OBJECT, &args![]).bool() {
        item = form;
    }
    let inventory = if reference.is_null() {
        Ptr::NULL
    } else {
        e.call(GET_INVENTORY_CHANGES, &args![reference]).ptr::<()>()
    };
    if !inventory.is_null() {
        if !item.is_null() {
            let count = e
                .call(INVENTORY_GET_OBJECT_COUNT, &args![inventory, item])
                .u32();
            let count = e.call(ITEM_COUNT_FILTER, &args![count]).i32();
            set_result(e, result, count as f64);
        } else if !form.is_null()
            && e.call(FORM_GET_TYPE, &args![form]).u32() == FORM_TYPE_FORM_LIST
        {
            let mut node = e.call(FORM_LIST_GET_LIST, &args![form]).ptr::<()>();
            while !node.is_null() && !e.call(LIST_NODE_IS_EMPTY, &args![node]).bool() {
                let item_address = e.call(LIST_NODE_ITEM_ADDRESS, &args![node]).u32();
                let entry = e.mem.u32(item_address);
                node = e.call(LIST_NODE_NEXT, &args![node]).ptr();
                if e.vcall(entry, VSLOT_IS_BOUND_OBJECT, &args![]).bool() {
                    let count = e
                        .call(INVENTORY_GET_OBJECT_COUNT, &args![inventory, entry])
                        .u32();
                    let count = e.call(ITEM_COUNT_FILTER, &args![count]).i32();
                    let total = count as f64 + e.mem.f64(result.addr());
                    set_result(e, result, total);
                }
            }
        }
    }
    if trace_enabled(e) {
        // "GetItemCount >> %0.2f"
        trace_result(e, 0x0103_4fec, result);
    }
    true
}

// Translated from 0059da90 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Script::GetEquippedConditionFunction` (Xbox PDB): 1.0 when the actor
/// wears or wields the item, or any item of the form list
/// (`InventoryChanges::WearingObject`).
pub fn script_get_equipped_condition_function(
    e: &mut Engine,
    reference: Ptr,
    form: Ptr,
    _param2: u32,
    result: Ptr,
) -> bool {
    set_result(e, result, 0.0);
    let actor = actor_of(e, reference);
    let mut item = form;
    if !actor.is_null() && !item.is_null() {
        let mut node: Ptr = Ptr::NULL;
        if e.call(FORM_GET_TYPE, &args![item]).u32() == FORM_TYPE_FORM_LIST {
            node = e.call(FORM_LIST_GET_LIST, &args![item]).ptr();
            let item_address = e.call(LIST_NODE_ITEM_ADDRESS, &args![node]).u32();
            item = Ptr::new(e.mem.u32(item_address));
        }
        while !item.is_null() {
            if e.vcall(item.addr(), VSLOT_IS_BOUND_OBJECT, &args![]).bool() {
                let inventory = e.call(GET_INVENTORY_CHANGES, &args![actor]).ptr::<()>();
                // InventoryChanges::WearingObject (Xbox PDB)
                if !inventory.is_null()
                    && e.call(0x004b_fda0, &args![inventory, item, 0u32]).u32() != 0
                {
                    set_result(e, result, 1.0);
                    break;
                }
            }
            if !node.is_null() {
                node = e.call(LIST_NODE_NEXT, &args![node]).ptr();
            }
            item = if node.is_null() {
                Ptr::NULL
            } else {
                let item_address = e.call(LIST_NODE_ITEM_ADDRESS, &args![node]).u32();
                Ptr::new(e.mem.u32(item_address))
            };
        }
    }
    if trace_enabled(e) {
        // "GetEquipped >> %0.2f"
        trace_result(e, 0x0103_5030, result);
    }
    true
}

// Translated from 0059dbe0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Script::GetGoldConditionFunction` (Xbox PDB): how many caps (the form
/// `004839c0(0xf)` returns) the container reference holds.
pub fn script_get_gold_condition_function(
    e: &mut Engine,
    reference: Ptr,
    _param1: u32,
    _param2: u32,
    result: Ptr,
) -> bool {
    set_result(e, result, 0.0);
    let gold = e.call(0x0048_39c0, &args![0xfu32]).ptr::<()>();
    if !reference.is_null()
        && !gold.is_null()
        && e.call(REF_HAS_CONTAINER, &args![reference]).u32() != 0
    {
        let inventory = e.call(GET_INVENTORY_CHANGES, &args![reference]).ptr::<()>();
        if !inventory.is_null() {
            let count = e
                .call(INVENTORY_GET_OBJECT_COUNT, &args![inventory, gold])
                .i32();
            set_result(e, result, count as f64);
        }
    }
    if trace_enabled(e) {
        // "GetGold >> %0.2f"
        trace_result(e, 0x0103_5048, result);
    }
    true
}

/// This unit's translated functions, by exe address.
pub fn funcs() -> Vec<(u32, AbiFn)> {
    vec![
        entry!(0x0059bfa0, script_get_distance_condition_function(Ptr, Ptr, u32, Ptr) -> bool),
        entry!(0x0059c010, script_get_in_zone_condition_function(Ptr, Ptr, u32, Ptr) -> bool),
        entry!(0x0059c0c0, script_get_pos_condition_function(Ptr, u32, u32, Ptr) -> bool),
        entry!(0x0059c170, script_get_angle_condition_function(Ptr, u32, u32, Ptr) -> bool),
        entry!(0x0059c230, script_get_starting_pos_condition_function(Ptr, u32, u32, Ptr) -> bool),
        entry!(0x0059c2d0, script_get_starting_angle_condition_function(Ptr, u32, u32, Ptr) -> bool),
        entry!(0x0059c430, script_get_seconds_passed_condition_function(Ptr, Ptr, u32, Ptr) -> bool),
        entry!(0x0059c4c0, fn_0059c4c0(Ptr, u32, u32, Ptr) -> bool),
        entry!(0x0059c4f0, fn_0059c4f0(Ptr, u32, u32, Ptr) -> bool),
        entry!(0x0059c5d0, script_get_base_actor_value_condition_function(Ptr, u32, u32, Ptr) -> bool),
        entry!(0x0059c680, fn_0059c680(Ptr, u32, u32, Ptr) -> bool),
        entry!(0x0059c760, script_get_fatigue_percentage_condition_function(Ptr, u32, u32, Ptr) -> bool),
        entry!(0x0059c7e0, script_get_health_percentage_condition_function(Ptr, u32, u32, Ptr) -> bool),
        entry!(0x0059c860, script_get_walk_speed_condition_function(Ptr, u32, u32, Ptr) -> bool),
        entry!(0x0059c8e0, script_get_current_time_condition_function(Ptr, u32, u32, Ptr) -> bool),
        entry!(0x0059c930, script_get_scale_condition_function(Ptr, u32, u32, Ptr) -> bool),
        entry!(0x0059c990, script_get_los_condition_function(Ptr, Ptr, u32, Ptr) -> bool),
        entry!(0x0059ce80, fn_0059ce80(Ptr, u32)),
        entry!(0x0059ceb0, fn_0059ceb0(Ptr, Ptr)),
        entry!(0x0059cee0, fn_0059cee0(Ptr)),
        entry!(0x0059cf00, fn_0059cf00(Ptr)),
        entry!(0x0059cf20, fn_0059cf20(Ptr, u32) -> Ptr),
        entry!(0x0059cf50, script_get_disabled_condition_function(Ptr, Ptr, u32, Ptr) -> bool),
        entry!(0x0059d010, script_get_locked_condition_function(Ptr, u32, u32, Ptr) -> bool),
        entry!(0x0059d100, script_get_lock_level_condition_function(Ptr, u32, u32, Ptr) -> bool),
        entry!(0x0059d1d0, script_get_is_lock_broken_condition_function(Ptr, u32, u32, Ptr) -> bool),
        entry!(0x0059d250, script_get_disease_condition_function(Ptr, u32, u32, Ptr) -> bool),
        entry!(0x0059d2a0, script_get_vampire_condition_function(Ptr, u32, u32, Ptr) -> bool),
        entry!(0x0059d320, script_get_clothing_value_condition_function(Ptr, u32, u32, Ptr) -> bool),
        entry!(0x0059d3a0, fn_0059d3a0(Ptr, u32, u32, Ptr) -> bool),
        entry!(0x0059d3c0, fn_0059d3c0(Ptr, u32, u32, Ptr) -> bool),
        entry!(0x0059d3e0, fn_0059d3e0(Ptr, u32, u32, Ptr) -> bool),
        entry!(0x0059d400, script_same_faction_condition_function(Ptr, Ptr, u32, Ptr) -> bool),
        entry!(0x0059d540, script_same_race_condition_function(Ptr, Ptr, u32, Ptr) -> bool),
        entry!(0x0059d610, script_same_sex_condition_function(Ptr, Ptr, u32, Ptr) -> bool),
        entry!(0x0059d6e0, script_get_detected_condition_function(Ptr, Ptr, u32, Ptr) -> bool),
        entry!(0x0059d840, script_get_dead_condition_function(Ptr, u32, u32, Ptr) -> bool),
        entry!(0x0059d8e0, script_get_item_count_condition_function(Ptr, Ptr, u32, Ptr) -> bool),
        entry!(0x0059da90, script_get_equipped_condition_function(Ptr, Ptr, u32, Ptr) -> bool),
        entry!(0x0059dbe0, script_get_gold_condition_function(Ptr, u32, u32, Ptr) -> bool),
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

    /// An engine with the constants the functions read from the exe's data,
    /// the debug print and the two `IsActor` answers.
    fn engine() -> Engine {
        let mut e = Engine::new();
        for page in [
            0x0101_1000u32,
            0x0101_2000,
            0x0101_a000,
            0x0101_d000,
            0x0102_9000,
            0x0102_f000,
            0x011d_e000,
        ] {
            e.map(page, 0x1000);
        }
        e.set_global(0x0101_1588, 0.5f64);
        e.set_global(0x0101_1590, 2.0f64);
        e.set_global(0x0101_2060, 0.0f64);
        e.set_global(0x0101_a6b0, -1.0f64);
        e.set_global(0x0101_de30, 0.75f64);
        e.set_global(0x0102_90b0, 0.25f64);
        e.set_global(DEGREES_PER_RADIAN, 57.29577951308232f64);
        e.register(DEBUG_PRINT, |_, _| Ret::default());
        e.register(IS_ACTOR_YES, |_, _| true.into_ret());
        e.register(IS_ACTOR_NO, |_, _| false.into_ret());
        // BSSimpleList node helpers.
        e.register(LIST_NODE_IS_EMPTY, |e, a| {
            (e.mem.u32(a[0]) == 0 && e.mem.u32(a[0] + 4) == 0).into_ret()
        });
        e.register(LIST_NODE_ITEM_ADDRESS, |_, a| a[0].into_ret());
        e.register(LIST_NODE_NEXT, |e, a| e.mem.u32(a[0] + 4).into_ret());
        e.register(FORM_LIST_GET_LIST, |_, a| (a[0] + 0x18).into_ret());
        e
    }

    /// Calls the condition function at `addr` and returns what it returned
    /// and what it left in `*result`.
    fn run(e: &mut Engine, addr: u32, reference: u32, param1: u32, param2: u32) -> (bool, f64) {
        let result = e.mem.alloc(8);
        e.mem.set_f64(result, SENTINEL);
        let returned = e
            .call(addr, &args![reference, param1, param2, result])
            .bool();
        (returned, e.mem.f64(result))
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
        let object = e.mem.alloc(0x200);
        e.mem.set_u32(object, vtable);
        object
    }

    fn actor(e: &mut Engine) -> u32 {
        object(e, &[(VSLOT_IS_ACTOR, IS_ACTOR_YES)])
    }

    fn non_actor(e: &mut Engine) -> u32 {
        object(e, &[(VSLOT_IS_ACTOR, IS_ACTOR_NO)])
    }

    /// Three floats in memory.
    fn floats(e: &mut Engine, values: [f32; 3]) -> u32 {
        let block = e.mem.alloc(12);
        for (i, v) in values.iter().enumerate() {
            e.mem.set_f32(block + 4 * i as u32, *v);
        }
        block
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

    #[test]
    fn distance_comes_from_the_reference_distance() {
        let mut e = engine();
        stub_st0(&mut e, REF_GET_DISTANCE_FROM_REFERENCE, 12.5);
        trace_on(&mut e);
        let (returned, value) = run(&mut e, 0x0059_bfa0, 0x2000, 0x3000, 0);
        assert!(returned);
        assert_eq!(value, 12.5);
        let log = take_log(&mut e);
        assert_eq!(
            calls_to(&log, REF_GET_DISTANCE_FROM_REFERENCE),
            vec![vec![0x2000, 0x3000, 1, 0]]
        );
        let w = words(12.5);
        assert_eq!(
            calls_to(&log, DEBUG_PRINT),
            vec![vec![0x0103_4d0c, w[0], w[1]]]
        );
    }

    #[test]
    fn distance_leaves_the_result_when_a_reference_is_null() {
        let mut e = engine();
        assert_eq!(run(&mut e, 0x0059_bfa0, 0x2000, 0, 0), (true, SENTINEL));
        assert_eq!(run(&mut e, 0x0059_bfa0, 0, 0x2000, 0), (true, SENTINEL));
    }

    #[test]
    fn in_zone_matches_the_parent_cell_then_the_worldspace_cell() {
        let mut e = engine();
        // The reference has a parent (00 8d6f30) whose cell (00546c20) is 0x500.
        stub(&mut e, 0x008d_6f30, 0x9000);
        stub(&mut e, 0x0054_6c20, 0x500);
        assert_eq!(run(&mut e, 0x0059_c010, 0x2000, 0x500, 0), (true, 1.0));
        assert_eq!(run(&mut e, 0x0059_c010, 0x2000, 0x600, 0), (true, 0.0));

        // No parent cell: the worldspace's cell (0045 8400) is used.
        stub(&mut e, 0x008d_6f30, 0);
        stub(&mut e, 0x0057_5d70, 0x9100);
        stub(&mut e, 0x0045_8400, 0x700);
        assert_eq!(run(&mut e, 0x0059_c010, 0x2000, 0x700, 0), (true, 1.0));
        // Neither: the cell is null and never equals a non-null zone.
        stub(&mut e, 0x0057_5d70, 0);
        assert_eq!(run(&mut e, 0x0059_c010, 0x2000, 0x700, 0), (true, 0.0));
        // A null zone: nothing is computed, but the result is 0.
        assert_eq!(run(&mut e, 0x0059_c010, 0x2000, 0, 0), (true, 0.0));
    }

    #[test]
    fn pos_picks_the_axis() {
        let mut e = engine();
        let position = floats(&mut e, [1.5, 2.5, 3.5]);
        let reference = object(&mut e, &[(VSLOT_GET_POSITION, 0x0f00_0010)]);
        stub(&mut e, 0x0f00_0010, position);
        assert_eq!(run(&mut e, 0x0059_c0c0, reference, 0x58, 0), (true, 1.5));
        assert_eq!(run(&mut e, 0x0059_c0c0, reference, 0x59, 0), (true, 2.5));
        assert_eq!(run(&mut e, 0x0059_c0c0, reference, 0x5a, 0), (true, 3.5));
        // Any other letter leaves the result alone.
        assert_eq!(
            run(&mut e, 0x0059_c0c0, reference, 0x41, 0),
            (true, SENTINEL)
        );
        // A null reference does nothing at all.
        assert_eq!(run(&mut e, 0x0059_c0c0, 0, 0x58, 0), (true, SENTINEL));

        trace_on(&mut e);
        run(&mut e, 0x0059_c0c0, reference, 0x59, 0);
        let log = take_log(&mut e);
        let w = words(2.5);
        assert_eq!(
            calls_to(&log, DEBUG_PRINT),
            vec![vec![0x0103_4d38, 0x59, w[0], w[1]]]
        );
    }

    #[test]
    fn angle_is_in_degrees() {
        let mut e = engine();
        let angles = floats(&mut e, [1.0, 0.5, 0.0]);
        stub(&mut e, 0x0043_0830, angles);
        let (_, x) = run(&mut e, 0x0059_c170, 0x2000, 0x58, 0);
        assert_eq!(x, 1.0f32 as f64 * 57.29577951308232);
        let (_, y) = run(&mut e, 0x0059_c170, 0x2000, 0x59, 0);
        assert_eq!(y, 0.5f32 as f64 * 57.29577951308232);
        assert_eq!(run(&mut e, 0x0059_c170, 0x2000, 0x5a, 0), (true, 0.0));
        assert_eq!(run(&mut e, 0x0059_c170, 0x2000, 0x41, 0), (true, SENTINEL));
        assert_eq!(run(&mut e, 0x0059_c170, 0, 0x58, 0), (true, SENTINEL));
    }

    #[test]
    fn starting_pos_reads_the_out_buffer_the_virtual_fills() {
        let mut e = engine();
        e.register(0x0f00_0020, |e, a| {
            for (i, v) in [10.0f32, 20.0, 30.0].iter().enumerate() {
                e.mem.set_f32(a[1] + 4 * i as u32, *v);
            }
            Ret::default()
        });
        let reference = object(&mut e, &[(0x170, 0x0f00_0020)]);
        assert_eq!(run(&mut e, 0x0059_c230, reference, 0x58, 0), (true, 10.0));
        assert_eq!(run(&mut e, 0x0059_c230, reference, 0x59, 0), (true, 20.0));
        assert_eq!(run(&mut e, 0x0059_c230, reference, 0x5a, 0), (true, 30.0));
        assert_eq!(
            run(&mut e, 0x0059_c230, reference, 0x41, 0),
            (true, SENTINEL)
        );
        assert_eq!(run(&mut e, 0x0059_c230, 0, 0x58, 0), (true, SENTINEL));
    }

    #[test]
    fn starting_angle_is_in_degrees() {
        let mut e = engine();
        e.register(0x0f00_0021, |e, a| {
            for (i, v) in [0.0f32, 2.0, 0.25].iter().enumerate() {
                e.mem.set_f32(a[1] + 4 * i as u32, *v);
            }
            Ret::default()
        });
        let reference = object(&mut e, &[(0x16c, 0x0f00_0021)]);
        assert_eq!(run(&mut e, 0x0059_c2d0, reference, 0x58, 0), (true, 0.0));
        let (_, y) = run(&mut e, 0x0059_c2d0, reference, 0x59, 0);
        assert_eq!(y, 2.0f32 as f64 * 57.29577951308232);
        let (_, z) = run(&mut e, 0x0059_c2d0, reference, 0x5a, 0);
        assert_eq!(z, 0.25f32 as f64 * 57.29577951308232);
        assert_eq!(run(&mut e, 0x0059_c2d0, 0, 0x58, 0), (true, SENTINEL));
    }

    #[test]
    fn seconds_passed_prefers_the_source_when_flagged_and_positive() {
        let mut e = engine();
        // The source's record (source + 0x18) with the flag byte at +0x10.
        let source = e.mem.alloc(0x40);
        e.mem.set_u8(source + 0x18 + 0x10, 1);
        stub_st0(&mut e, REF_GET_OWN_SCALE, 0.25);
        stub_st0(&mut e, 0x0084_d030, 9.0);
        assert_eq!(run(&mut e, 0x0059_c430, 0, source, 0), (true, 0.25));

        // Not positive: the global data's value.
        stub_st0(&mut e, REF_GET_OWN_SCALE, 0.0);
        assert_eq!(run(&mut e, 0x0059_c430, 0, source, 0), (true, 9.0));
        // Flag clear: also the global data's value.
        stub_st0(&mut e, REF_GET_OWN_SCALE, 3.0);
        e.mem.set_u8(source + 0x18 + 0x10, 0);
        assert_eq!(run(&mut e, 0x0059_c430, 0, source, 0), (true, 9.0));
        // No source.
        assert_eq!(run(&mut e, 0x0059_c430, 0, 0, 0), (true, 9.0));

        trace_on(&mut e);
        run(&mut e, 0x0059_c430, 0, 0, 0);
        let log = take_log(&mut e);
        assert_eq!(
            calls_to(&log, 0x0084_d030),
            vec![vec![SECONDS_PASSED_SOURCE]]
        );
        let w = words(9.0);
        assert_eq!(
            calls_to(&log, DEBUG_PRINT),
            vec![vec![0x0103_4db8, w[0], w[1]]]
        );
    }

    #[test]
    fn unnamed_0059c4c0_is_one_when_the_virtual_answers() {
        let mut e = engine();
        stub(&mut e, 0x0f00_0030, 0x1234);
        stub(&mut e, 0x0f00_0031, 0);
        let yes = object(&mut e, &[(0x1d0, 0x0f00_0030)]);
        let no = object(&mut e, &[(0x1d0, 0x0f00_0031)]);
        assert_eq!(run(&mut e, 0x0059_c4c0, yes, 0, 0), (true, 1.0));
        assert_eq!(run(&mut e, 0x0059_c4c0, no, 0, 0), (true, 0.0));
        assert_eq!(run(&mut e, 0x0059_c4c0, 0, 0, 0), (true, 0.0));
    }

    /// An actor whose embedded `ActorValueOwner` (at +0xa4) has `slots`, and
    /// whose base form (returned by `0041 81e0`) has another one at +0x100.
    fn actor_with_owners(
        e: &mut Engine,
        actor_slots: &[(u32, u32)],
        base_slots: &[(u32, u32)],
    ) -> u32 {
        let actor = actor(e);
        let owner_table = e.mem.alloc(0x100);
        for &(offset, function) in actor_slots {
            e.mem.set_u32(owner_table + offset, function);
        }
        e.mem.set_u32(actor + 0xa4, owner_table);
        let base = e.mem.alloc(0x200);
        let base_table = e.mem.alloc(0x100);
        for &(offset, function) in base_slots {
            e.mem.set_u32(base_table + offset, function);
        }
        e.mem.set_u32(base + 0x100, base_table);
        e.register_double(ACTOR_GET_BASE_FORM, move |_, _| Ret {
            eax: base,
            ..Ret::default()
        });
        actor
    }

    #[test]
    fn actor_value_reads_the_actors_owner_or_the_base_forms() {
        let mut e = engine();
        // Owner virtual +0xc returns 42.5 (actor) / 7.5 (base form); the
        // argument is the actor value index.
        e.register(0x0f00_0040, |e, a| {
            // `this` is the embedded owner (actor + 0xa4), the argument the
            // actor value index.
            assert_eq!(a[1], 8);
            assert!(e.mem.u32(a[0]) != 0);
            Ret {
                st0: 42.5,
                ..Ret::default()
            }
        });
        e.register(0x0f00_0041, |_, _| Ret {
            st0: 7.5,
            ..Ret::default()
        });
        let actor = actor_with_owners(&mut e, &[(0xc, 0x0f00_0040)], &[(0xc, 0x0f00_0041)]);
        stub(&mut e, FORM_HAS_FLAG_0800, 0);
        assert_eq!(run(&mut e, 0x0059_c4f0, actor, 8, 0), (true, 42.5));
        // With form flag 0x800 the base form's owner answers.
        stub(&mut e, FORM_HAS_FLAG_0800, 1);
        assert_eq!(run(&mut e, 0x0059_c4f0, actor, 8, 0), (true, 7.5));
        // A non-actor and a null reference write nothing.
        let other = non_actor(&mut e);
        assert_eq!(run(&mut e, 0x0059_c4f0, other, 8, 0), (true, SENTINEL));
        assert_eq!(run(&mut e, 0x0059_c4f0, 0, 8, 0), (true, SENTINEL));

        // The debug line names the actor value.
        stub(&mut e, ACTOR_VALUE_SCRIPT_NAME, 0x7777);
        trace_on(&mut e);
        run(&mut e, 0x0059_c4f0, actor, 8, 0);
        let log = take_log(&mut e);
        assert_eq!(calls_to(&log, ACTOR_VALUE_SCRIPT_NAME), vec![vec![8]]);
        let w = words(7.5);
        assert_eq!(
            calls_to(&log, DEBUG_PRINT),
            vec![vec![0x0103_4dd4, 0x7777, w[0], w[1]]]
        );
    }

    #[test]
    fn base_actor_value_is_an_integer_from_slot_zero() {
        let mut e = engine();
        stub(&mut e, 0x0f00_0042, (-12i32) as u32);
        let actor = actor_with_owners(&mut e, &[(0, 0x0f00_0042)], &[]);
        assert_eq!(run(&mut e, 0x0059_c5d0, actor, 3, 0), (true, -12.0));
        let other = non_actor(&mut e);
        assert_eq!(run(&mut e, 0x0059_c5d0, other, 3, 0), (true, SENTINEL));
        stub(&mut e, ACTOR_VALUE_SCRIPT_NAME, 0x7778);
        trace_on(&mut e);
        run(&mut e, 0x0059_c5d0, actor, 3, 0);
        let log = take_log(&mut e);
        let w = words(-12.0);
        assert_eq!(
            calls_to(&log, DEBUG_PRINT),
            vec![vec![0x0103_4df0, 0x7778, w[0], w[1]]]
        );
    }

    #[test]
    fn permanent_actor_value_uses_slot_0x20() {
        let mut e = engine();
        e.register(0x0f00_0043, |_, _| Ret {
            st0: 55.0,
            ..Ret::default()
        });
        e.register(0x0f00_0044, |_, _| Ret {
            st0: 66.0,
            ..Ret::default()
        });
        let actor = actor_with_owners(&mut e, &[(0x20, 0x0f00_0043)], &[(0x20, 0x0f00_0044)]);
        stub(&mut e, FORM_HAS_FLAG_0800, 0);
        assert_eq!(run(&mut e, 0x0059_c680, actor, 1, 0), (true, 55.0));
        stub(&mut e, FORM_HAS_FLAG_0800, 1);
        assert_eq!(run(&mut e, 0x0059_c680, actor, 1, 0), (true, 66.0));
        assert_eq!(run(&mut e, 0x0059_c680, 0, 1, 0), (true, SENTINEL));
    }

    #[test]
    fn actor_percentages_and_walk_speed() {
        let mut e = engine();
        let actor = actor(&mut e);
        let other = non_actor(&mut e);
        for (condition, callee, value) in [
            (0x0059_c760u32, 0x0089_3530u32, 0.75),
            (0x0059_c7e0, 0x0089_3590, 0.5),
            (0x0059_c860, 0x0088_4dc0, 135.0),
        ] {
            stub_st0(&mut e, callee, value);
            assert_eq!(run(&mut e, condition, actor, 0, 0), (true, value));
            assert_eq!(run(&mut e, condition, other, 0, 0), (true, SENTINEL));
            assert_eq!(run(&mut e, condition, 0, 0, 0), (true, SENTINEL));
        }
        trace_on(&mut e);
        run(&mut e, 0x0059_c7e0, actor, 0, 0);
        let log = take_log(&mut e);
        let w = words(0.5);
        assert_eq!(
            calls_to(&log, DEBUG_PRINT),
            vec![vec![0x0103_4e30, w[0], w[1]]]
        );
    }

    #[test]
    fn current_time_is_the_calendar_hour() {
        let mut e = engine();
        stub_st0(&mut e, 0x0086_7da0, 13.5);
        trace_on(&mut e);
        assert_eq!(run(&mut e, 0x0059_c8e0, 0, 0, 0), (true, 13.5));
        let log = take_log(&mut e);
        assert_eq!(calls_to(&log, 0x0086_7da0), vec![vec![CALENDAR]]);
    }

    #[test]
    fn scale_prints_the_base_scale_too() {
        let mut e = engine();
        stub_st0(&mut e, REF_GET_OWN_SCALE, 1.25);
        stub_st0(&mut e, REF_GET_SCALE, 2.5);
        assert_eq!(run(&mut e, 0x0059_c930, 0x2000, 0, 0), (true, 1.25));
        assert_eq!(run(&mut e, 0x0059_c930, 0, 0, 0), (true, SENTINEL));
        trace_on(&mut e);
        run(&mut e, 0x0059_c930, 0x2000, 0, 0);
        let log = take_log(&mut e);
        let result = words(1.25);
        let base = words(2.5);
        assert_eq!(
            calls_to(&log, DEBUG_PRINT),
            vec![vec![0x0103_4e80, result[0], result[1], base[0], base[1]]]
        );
    }

    /// Everything `GetLOS` calls for the player, with simple answers. The
    /// target's virtuals: `0x1d0` returns an owner whose `0xc` returns
    /// `0x6000`, `0x1f4` the position (1, 2, 10), `0x1dc` / `0x1d8` bounds
    /// with z 190 and 100, `0xfc` yes.
    fn los_engine() -> (Engine, u32, u32) {
        let mut e = engine();
        let player = actor(&mut e);
        e.set_global(PLAYER, player);
        e.set_global(TES, 0x00ee_0000u32);
        let owner = object(&mut e, &[(0xc, 0x0f00_0051)]);
        stub(&mut e, 0x0f00_0050, owner);
        stub(&mut e, 0x0f00_0051, 0x6000);
        let position = floats(&mut e, [1.0, 2.0, 10.0]);
        stub(&mut e, 0x0f00_0052, position);
        let max_bound = floats(&mut e, [0.0, 0.0, 190.0]);
        let min_bound = floats(&mut e, [0.0, 0.0, 100.0]);
        stub(&mut e, 0x0f00_0053, max_bound);
        stub(&mut e, 0x0f00_0054, min_bound);
        stub(&mut e, 0x0f00_0055, 1);
        let target = object(
            &mut e,
            &[
                (VSLOT_IS_ACTOR, IS_ACTOR_YES),
                (0x1d0, 0x0f00_0050),
                (VSLOT_GET_POSITION, 0x0f00_0052),
                (0x1dc, 0x0f00_0053),
                (0x1d8, 0x0f00_0054),
                (VSLOT_IS_MOBILE_OBJECT, 0x0f00_0055),
            ],
        );
        stub(&mut e, 0x0045_c670, 0x7000);
        stub(&mut e, 0x0055_8310, 0x7100);
        stub(&mut e, 0x0066_29f0, 0x7200);
        stub(&mut e, 0x004b_5fc0, 1);
        stub(&mut e, 0x0044_4ed0, 0);
        let origin = floats(&mut e, [5.0, 6.0, 7.0]);
        stub(&mut e, 0x0043_c490, origin);
        for addr in [
            0x0062_40d0u32,
            0x0098_ddd0,
            0x0063_f790,
            0x004a_3c20,
            0x0062_a190,
            0x004a_3da0,
            0x0093_1ed0,
            0x004a_3f70,
            0x004a_3eb0,
            0x004a_3ae0,
        ] {
            e.register(addr, |_, _| Ret::default());
        }
        stub(&mut e, 0x004b_5ff0, 0);
        e.register(0x008c_71b0, |e, a| {
            e.mem.set_u32(a[0], a[1]);
            Ret::default()
        });
        e.register(0x004a_39f0, |e, a| {
            let word = e.mem.u32(a[0]) & !0x7f | (a[1] & 0x7f);
            e.mem.set_u32(a[0], word);
            Ret::default()
        });
        stub(&mut e, 0x004a_3a20, 0x1234);
        stub(&mut e, 0x0088_b880, 0);
        stub(&mut e, 0x0056_f930, 0);
        stub(&mut e, 0x0055_d520, 0x8000);
        (e, player, target)
    }

    #[test]
    fn los_player_sees_the_target_when_the_first_ray_is_unblocked() {
        let (mut e, player, target) = los_engine();
        // Records the ray end the pick was made with, and finds nothing.
        e.register_double(0x004a_3eb0, |e, a| {
            let z = e.mem.f32(a[1] + 8);
            e.mem.set_f32(0x0101_1ff0, z);
            Ret::default()
        });
        stub(&mut e, 0x0045_8420, 0);
        trace_on(&mut e);
        assert_eq!(run(&mut e, 0x0059_c990, player, target, 0), (true, 1.0));
        let log = take_log(&mut e);
        // Height 90 * 0.75 + the target position's z (10).
        assert_eq!(e.mem.f32(0x0101_1ff0), 77.5);
        assert_eq!(calls_to(&log, 0x0045_8420).len(), 1);
        assert_eq!(calls_to(&log, ACTOR_LINE_OF_SIGHT).len(), 0);
        // The filter: layer 0x25 and the target (it is a mobile object).
        assert_eq!(&calls_to(&log, 0x0062_a190)[0][1..], &[0x25, target]);
        // The collector is destroyed again.
        assert_eq!(calls_to(&log, 0x004a_3ae0).len(), 1);
        // "sees" line: names of the player and the target.
        assert_eq!(
            calls_to(&log, DEBUG_PRINT),
            vec![vec![0x0103_4eb4, 0x8000, 0x8000]]
        );
    }

    #[test]
    fn los_player_tries_three_heights_then_falls_back_to_line_of_sight() {
        let (mut e, player, target) = los_engine();
        e.register(0x004a_3eb0, |e, a| {
            // Keep each ray end's z for the check below.
            let n = e.mem.u32(0x0101_1ff0);
            e.mem.set_u32(0x0101_1ff0, n + 1);
            let z = e.mem.f32(a[1] + 8);
            e.mem.set_f32(0x0101_1ff4 + 4 * n, z);
            Ret::default()
        });
        // Something else is always picked (a reference that is not the target).
        stub(&mut e, 0x0045_8420, 0x9999);
        stub(&mut e, 0x0056_f930, 0x4444);
        stub(&mut e, 0x0088_b880, 1);
        trace_on(&mut e);
        assert_eq!(run(&mut e, 0x0059_c990, player, target, 0), (true, 1.0));
        let log = take_log(&mut e);
        assert_eq!(e.mem.u32(0x0101_1ff0), 3);
        assert_eq!(e.mem.f32(0x0101_1ff4), 77.5);
        assert_eq!(e.mem.f32(0x0101_1ff8), 55.0);
        assert_eq!(e.mem.f32(0x0101_1ffc), 32.5);
        assert_eq!(
            calls_to(&log, ACTOR_LINE_OF_SIGHT),
            vec![vec![player, 0, target, 1, 0, 0]]
        );

        // The line of sight fails too: the player cannot see the target.
        stub(&mut e, 0x0088_b880, 0);
        trace_on(&mut e);
        assert_eq!(run(&mut e, 0x0059_c990, player, target, 0), (true, 0.0));
        let log = take_log(&mut e);
        assert_eq!(
            calls_to(&log, DEBUG_PRINT),
            vec![vec![0x0103_4ea4, 0x8000, 0x8000]]
        );
    }

    #[test]
    fn los_player_sees_when_the_second_pick_is_the_target() {
        let (mut e, player, target) = los_engine();
        e.register_double(0x0045_8420, |_, _| Ret {
            eax: 0x9999,
            ..Ret::default()
        });
        // The first pick resolves to another reference, the second to the
        // target (compared by address).
        let target_address = target;
        let mut calls = 0;
        e.register_double(0x0056_f930, move |_, _| {
            calls += 1;
            Ret {
                eax: if calls == 2 { target_address } else { 0x4444 },
                ..Ret::default()
            }
        });
        trace_on(&mut e);
        assert_eq!(run(&mut e, 0x0059_c990, player, target, 0), (true, 1.0));
        let log = take_log(&mut e);
        assert_eq!(calls_to(&log, 0x0045_8420).len(), 2);
        assert_eq!(calls_to(&log, ACTOR_LINE_OF_SIGHT).len(), 0);
    }

    #[test]
    fn los_player_cannot_see_a_target_outside_the_view() {
        let (mut e, player, target) = los_engine();
        stub(&mut e, 0x004b_5fc0, 0);
        stub(&mut e, 0x0044_4ed0, 0);
        trace_on(&mut e);
        assert_eq!(run(&mut e, 0x0059_c990, player, target, 0), (true, 0.0));
        let log = take_log(&mut e);
        assert_eq!(calls_to(&log, 0x0045_8420).len(), 0);
        assert_eq!(calls_to(&log, ACTOR_LINE_OF_SIGHT).len(), 0);
        assert_eq!(
            calls_to(&log, DEBUG_PRINT),
            vec![vec![0x0103_4ea4, 0x8000, 0x8000]]
        );
    }

    #[test]
    fn los_player_falls_back_on_a_bound_built_from_the_position() {
        let (mut e, player, target) = los_engine();
        // No bound owner test passes, but the target can be tested by a bound.
        stub(&mut e, 0x004b_5fc0, 0);
        stub(&mut e, 0x0044_4ed0, 1);
        stub(&mut e, 0x004b_5ff0, 1);
        stub(&mut e, 0x0045_8420, 0);
        trace_on(&mut e);
        assert_eq!(run(&mut e, 0x0059_c990, player, target, 0), (true, 1.0));
        let log = take_log(&mut e);
        // The bound gets the position and the radius 1.0.
        let bound_position = calls_to(&log, 0x0098_ddd0);
        assert_eq!(bound_position.len(), 1);
        assert_eq!(calls_to(&log, 0x0063_f790)[0][1], 1.0f32.to_bits());
    }

    #[test]
    fn los_other_actors_use_actor_line_of_sight() {
        let (mut e, _, target) = los_engine();
        let viewer = actor(&mut e);
        stub(&mut e, 0x0088_b880, 1);
        trace_on(&mut e);
        assert_eq!(run(&mut e, 0x0059_c990, viewer, target, 0), (true, 1.0));
        let log = take_log(&mut e);
        assert_eq!(
            calls_to(&log, ACTOR_LINE_OF_SIGHT),
            vec![vec![viewer, 0, target, 1, 0, 0]]
        );
        assert_eq!(calls_to(&log, 0x0045_8420).len(), 0);
        stub(&mut e, 0x0088_b880, 0);
        assert_eq!(run(&mut e, 0x0059_c990, viewer, target, 0), (true, 0.0));
        // A viewer that is not an actor, or no target: 0.
        let plain = non_actor(&mut e);
        assert_eq!(run(&mut e, 0x0059_c990, plain, target, 0), (true, 0.0));
        assert_eq!(run(&mut e, 0x0059_c990, viewer, 0, 0), (true, 0.0));
        assert_eq!(run(&mut e, 0x0059_c990, 0, target, 0), (true, 0.0));
    }

    #[test]
    fn collision_group_sets_the_upper_half_word() {
        let mut e = engine();
        let word = e.mem.alloc(4);
        e.mem.set_u32(word, 0x1234_5678);
        e.call(0x0059_ce80, &args![word, 0xabcdu32]);
        assert_eq!(e.mem.u32(word), 0xabcd_5678);
        // Only 16 bits of the argument fit.
        e.call(0x0059_ce80, &args![word, 0x1_0002u32]);
        assert_eq!(e.mem.u32(word), 0x0002_5678);
    }

    #[test]
    fn pick_data_gets_its_collector() {
        let mut e = engine();
        let pick = e.mem.alloc(0xb0);
        e.mem.set_u32(pick + 0xa8, 0xffff);
        e.call(0x0059_ceb0, &args![pick, 0x4321u32]);
        assert_eq!(e.mem.u32(pick + 0xa4), 0x4321);
        assert_eq!(e.mem.u32(pick + 0xa8), 0);
    }

    #[test]
    fn collector_destructors() {
        let mut e = engine();
        let collector = e.mem.alloc(0x90);
        e.register(0x004a_3ae0, |_, _| Ret::default());
        e.register(OPERATOR_DELETE, |_, _| Ret::default());
        e.call_log = Some(vec![]);
        e.call(0x0059_cee0, &args![collector]);
        assert_eq!(e.mem.u32(collector), 0x0103_4ec4);
        let log = take_log(&mut e);
        assert_eq!(calls_to(&log, 0x004a_3ae0), vec![vec![collector]]);

        // The deleting form frees the memory only when bit 0 is set.
        e.call_log = Some(vec![]);
        assert_eq!(
            e.call(0x0059_cf20, &args![collector, 0u32]).u32(),
            collector
        );
        assert_eq!(
            e.call(0x0059_cf20, &args![collector, 3u32]).u32(),
            collector
        );
        let log = take_log(&mut e);
        assert_eq!(calls_to(&log, OPERATOR_DELETE), vec![vec![collector]]);
    }

    #[test]
    fn disabled_of_a_reference_follows_the_pending_flags() {
        let mut e = engine();
        // flag 0x800 set, not pending enable: disabled.
        stub(&mut e, FORM_HAS_FLAG_0800, 1);
        stub(&mut e, 0x005a_a680, 0);
        stub(&mut e, 0x005a_a630, 0);
        assert_eq!(run(&mut e, 0x0059_cf50, 0x2000, 0, 0), (true, 1.0));
        // flag set but pending enable: not disabled unless also pending disable.
        stub(&mut e, 0x005a_a680, 1);
        assert_eq!(run(&mut e, 0x0059_cf50, 0x2000, 0, 0), (true, 0.0));
        stub(&mut e, 0x005a_a630, 1);
        assert_eq!(run(&mut e, 0x0059_cf50, 0x2000, 0, 0), (true, 1.0));
        // flag clear: only pending disable counts.
        stub(&mut e, FORM_HAS_FLAG_0800, 0);
        assert_eq!(run(&mut e, 0x0059_cf50, 0x2000, 0, 0), (true, 1.0));
        stub(&mut e, 0x005a_a630, 0);
        assert_eq!(run(&mut e, 0x0059_cf50, 0x2000, 0, 0), (true, 0.0));
    }

    #[test]
    fn disabled_without_a_reference_reads_bit_zero_of_the_record() {
        let mut e = engine();
        let record = e.mem.alloc(0x10);
        e.mem.set_u8(record + 4, 0x03);
        assert_eq!(run(&mut e, 0x0059_cf50, 0, record, 0), (true, 1.0));
        e.mem.set_u8(record + 4, 0x02);
        assert_eq!(run(&mut e, 0x0059_cf50, 0, record, 0), (true, 0.0));
        assert_eq!(run(&mut e, 0x0059_cf50, 0, 0, 0), (true, SENTINEL));
    }

    #[test]
    fn locked_reports_broken_and_locked_locks_and_terminals() {
        let mut e = engine();
        // A lock: broken is 2, otherwise locked (005021a0) is 1, else 0.
        stub(&mut e, REF_GET_LOCK, 0x5000);
        stub(&mut e, 0x0043_0ae0, 1);
        assert_eq!(run(&mut e, 0x0059_d010, 0x2000, 0, 0), (true, 2.0));
        stub(&mut e, 0x0043_0ae0, 0);
        stub(&mut e, 0x0050_21a0, 1);
        assert_eq!(run(&mut e, 0x0059_d010, 0x2000, 0, 0), (true, 1.0));
        stub(&mut e, 0x0050_21a0, 0);
        assert_eq!(run(&mut e, 0x0059_d010, 0x2000, 0, 0), (true, 0.0));

        // No lock: a terminal depends on the player's access and whether it
        // is unlocked; other forms give 0.
        stub(&mut e, REF_GET_LOCK, 0);
        stub(&mut e, REF_GET_BASE_FORM, 0x6000);
        stub(&mut e, FORM_GET_TYPE, FORM_TYPE_TERMINAL);
        let player = actor(&mut e);
        e.set_global(PLAYER, player);
        stub(&mut e, 0x0096_6c60, 2);
        assert_eq!(run(&mut e, 0x0059_d010, 0x2000, 0, 0), (true, 2.0));
        stub(&mut e, 0x0096_6c60, 0);
        stub(&mut e, 0x0050_1ae0, 0);
        assert_eq!(run(&mut e, 0x0059_d010, 0x2000, 0, 0), (true, 1.0));
        stub(&mut e, 0x0050_1ae0, 1);
        assert_eq!(run(&mut e, 0x0059_d010, 0x2000, 0, 0), (true, 0.0));
        stub(&mut e, FORM_GET_TYPE, 0x28);
        assert_eq!(run(&mut e, 0x0059_d010, 0x2000, 0, 0), (true, 0.0));
        assert_eq!(run(&mut e, 0x0059_d010, 0, 0, 0), (true, 0.0));
    }

    #[test]
    fn lock_level_of_locks_and_terminals() {
        let mut e = engine();
        stub(&mut e, REF_GET_LOCK, 0x5000);
        stub(&mut e, 0x0043_0a10, 75);
        assert_eq!(run(&mut e, 0x0059_d100, 0x2000, 0, 0), (true, 75.0));

        stub(&mut e, REF_GET_LOCK, 0);
        stub(&mut e, REF_GET_BASE_FORM, 0x6000);
        stub(&mut e, FORM_GET_TYPE, FORM_TYPE_TERMINAL);
        stub(&mut e, 0x0050_1ae0, 1);
        assert_eq!(run(&mut e, 0x0059_d100, 0x2000, 0, 0), (true, -1.0));
        stub(&mut e, 0x0050_1ae0, 0);
        stub(&mut e, 0x0050_11a0, 50);
        assert_eq!(run(&mut e, 0x0059_d100, 0x2000, 0, 0), (true, 50.0));
        stub(&mut e, FORM_GET_TYPE, 0x28);
        assert_eq!(run(&mut e, 0x0059_d100, 0x2000, 0, 0), (true, 0.0));
        assert_eq!(run(&mut e, 0x0059_d100, 0, 0, 0), (true, 0.0));
    }

    #[test]
    fn is_lock_broken() {
        let mut e = engine();
        stub(&mut e, REF_GET_LOCK, 0x5000);
        stub(&mut e, 0x0043_0ae0, 1);
        assert_eq!(run(&mut e, 0x0059_d1d0, 0x2000, 0, 0), (true, 1.0));
        stub(&mut e, 0x0043_0ae0, 0);
        assert_eq!(run(&mut e, 0x0059_d1d0, 0x2000, 0, 0), (true, 0.0));
        stub(&mut e, REF_GET_LOCK, 0);
        assert_eq!(run(&mut e, 0x0059_d1d0, 0x2000, 0, 0), (true, 0.0));
        assert_eq!(run(&mut e, 0x0059_d1d0, 0, 0, 0), (true, 0.0));
    }

    #[test]
    fn disease_is_always_zero() {
        let mut e = engine();
        trace_on(&mut e);
        assert_eq!(run(&mut e, 0x0059_d250, 0x2000, 0, 0), (true, 0.0));
        let log = take_log(&mut e);
        let w = words(0.0);
        assert_eq!(
            calls_to(&log, DEBUG_PRINT),
            vec![vec![0x0103_4f24, w[0], w[1]]]
        );
    }

    #[test]
    fn vampire_asks_the_actor() {
        let mut e = engine();
        let actor = actor(&mut e);
        stub(&mut e, 0x0047_c850, 1);
        assert_eq!(run(&mut e, 0x0059_d2a0, actor, 0, 0), (true, 1.0));
        stub(&mut e, 0x0047_c850, 0);
        assert_eq!(run(&mut e, 0x0059_d2a0, actor, 0, 0), (true, 0.0));
        let other = non_actor(&mut e);
        assert_eq!(run(&mut e, 0x0059_d2a0, other, 0, 0), (true, 0.0));
        assert_eq!(run(&mut e, 0x0059_d2a0, 0, 0, 0), (true, 0.0));
    }

    #[test]
    fn clothing_value_needs_an_npc() {
        let mut e = engine();
        stub(&mut e, REF_GET_BASE_FORM, 0x6000);
        stub(&mut e, FORM_GET_TYPE, FORM_TYPE_NPC);
        stub_st0(&mut e, 0x008d_3110, 123.0);
        assert_eq!(run(&mut e, 0x0059_d320, 0x2000, 0, 0), (true, 123.0));
        // Not an NPC: the result is not touched.
        stub(&mut e, FORM_GET_TYPE, 0x28);
        assert_eq!(run(&mut e, 0x0059_d320, 0x2000, 0, 0), (true, SENTINEL));
        assert_eq!(run(&mut e, 0x0059_d320, 0, 0, 0), (true, SENTINEL));
    }

    /// An NPC reference whose base form is `base`; `FORM_GET_TYPE` answers
    /// NPC for every form.
    fn npc_reference(e: &mut Engine, base: u32) -> u32 {
        let reference = e.mem.alloc(0x40);
        e.mem.set_u32(reference + 0x20, base);
        reference
    }

    fn install_base_form_stubs(e: &mut Engine) {
        e.register(REF_GET_BASE_FORM, |e, a| e.mem.u32(a[0] + 0x20).into_ret());
        stub(e, FORM_GET_TYPE, FORM_TYPE_NPC);
    }

    #[test]
    fn same_race_compares_the_races_of_two_npcs() {
        let mut e = engine();
        install_base_form_stubs(&mut e);
        e.register(0x004a_c110, |e, a| e.mem.u32(a[0] + 0x10).into_ret());
        let first_base = e.mem.alloc(0x40);
        let second_base = e.mem.alloc(0x40);
        e.mem.set_u32(first_base + 0x10, 5);
        e.mem.set_u32(second_base + 0x10, 5);
        let first = npc_reference(&mut e, first_base);
        let second = npc_reference(&mut e, second_base);
        assert_eq!(run(&mut e, 0x0059_d540, first, second, 0), (true, 1.0));
        e.mem.set_u32(second_base + 0x10, 6);
        assert_eq!(run(&mut e, 0x0059_d540, first, second, 0), (true, 0.0));
        // A null reference (or one that is not an NPC) gives 0.
        assert_eq!(run(&mut e, 0x0059_d540, first, 0, 0), (true, 0.0));
        stub(&mut e, FORM_GET_TYPE, 0x28);
        assert_eq!(run(&mut e, 0x0059_d540, first, second, 0), (true, 0.0));

        // Against the player (0059d3c0).
        stub(&mut e, FORM_GET_TYPE, FORM_TYPE_NPC);
        e.set_global(PLAYER, first);
        e.mem.set_u32(second_base + 0x10, 5);
        assert_eq!(run(&mut e, 0x0059_d3c0, second, 0, 0), (true, 1.0));
        e.mem.set_u32(second_base + 0x10, 9);
        assert_eq!(run(&mut e, 0x0059_d3c0, second, 0, 0), (true, 0.0));
    }

    #[test]
    fn same_sex_compares_the_sexes_of_two_npcs() {
        let mut e = engine();
        install_base_form_stubs(&mut e);
        e.register(0x005f_0cc0, |e, a| e.mem.u32(a[0] + 0x10).into_ret());
        let first_base = e.mem.alloc(0x40);
        let second_base = e.mem.alloc(0x40);
        e.mem.set_u32(first_base + 0x10, 1);
        e.mem.set_u32(second_base + 0x10, 1);
        let first = npc_reference(&mut e, first_base);
        let second = npc_reference(&mut e, second_base);
        assert_eq!(run(&mut e, 0x0059_d610, first, second, 0), (true, 1.0));
        e.mem.set_u32(second_base + 0x10, 0);
        assert_eq!(run(&mut e, 0x0059_d610, first, second, 0), (true, 0.0));
        assert_eq!(run(&mut e, 0x0059_d610, 0, second, 0), (true, 0.0));

        // Against the player (0059d3e0).
        e.set_global(PLAYER, first);
        assert_eq!(run(&mut e, 0x0059_d3e0, second, 0, 0), (true, 0.0));
        e.mem.set_u32(second_base + 0x10, 1);
        assert_eq!(run(&mut e, 0x0059_d3e0, second, 0, 0), (true, 1.0));
    }

    /// A faction list of `factions` (each wrapped in a rank record whose
    /// first field is the faction) at `base + 0x5c`, the place `005d8a70`
    /// (`this + 0x2c`) of `base + 0x30` returns.
    fn faction_base(e: &mut Engine, factions: &[u32]) -> u32 {
        let base = e.mem.alloc(0x80);
        let mut node = base + 0x5c;
        for (i, &faction) in factions.iter().enumerate() {
            let rank_record = e.mem.alloc(8);
            e.mem.set_u32(rank_record, faction);
            e.mem.set_u32(node, rank_record);
            if i + 1 < factions.len() {
                let next = e.mem.alloc(8);
                e.mem.set_u32(node + 4, next);
                node = next;
            }
        }
        base
    }

    #[test]
    fn same_faction_looks_for_a_shared_rank() {
        let mut e = engine();
        e.register(0x005d_8a70, |_, a| (a[0] + 0x2c).into_ret());
        e.register(ACTOR_GET_BASE_FORM, |e, a| {
            e.mem.u32(a[0] + 0x20).into_ret()
        });
        let first_base = faction_base(&mut e, &[0xa1, 0xa2, 0xa3]);
        let second_base = faction_base(&mut e, &[0xb1]);
        let first = {
            let reference = actor(&mut e);
            e.mem.set_u32(reference + 0x20, first_base);
            reference
        };
        let second = {
            let reference = actor(&mut e);
            e.mem.set_u32(reference + 0x20, second_base);
            reference
        };
        // Rank of faction 0xa3 in the second actor's data is 3, others -1.
        e.register(0x0047_d680, |_, a| {
            (if a[1] == 0xa3 { 3i32 } else { -1 }).into_ret()
        });
        assert_eq!(run(&mut e, 0x0059_d400, first, second, 0), (true, 1.0));
        e.register(0x0047_d680, |_, _| (-1i32).into_ret());
        assert_eq!(run(&mut e, 0x0059_d400, first, second, 0), (true, 0.0));

        // The "is the player" flag is passed on.
        e.set_global(PLAYER, second);
        e.register_double(0x0047_d680, |_, _| (-1i32).into_ret());
        e.call_log = Some(vec![]);
        run(&mut e, 0x0059_d400, first, second, 0);
        let log = take_log(&mut e);
        let calls = calls_to(&log, 0x0047_d680);
        assert_eq!(calls.len(), 3);
        assert!(calls
            .iter()
            .all(|w| w[0] == second_base + 0x30 && w[2] == 1));

        // Not an actor, or no second reference: 0, no debug line needed.
        let plain = non_actor(&mut e);
        assert_eq!(run(&mut e, 0x0059_d400, plain, second, 0), (true, 0.0));
        assert_eq!(run(&mut e, 0x0059_d400, first, 0, 0), (true, 0.0));

        // The wrapper (0059d3a0) compares with the player.
        e.register_double(0x0047_d680, |_, a| {
            (if a[1] == 0xa2 { 1i32 } else { -1 }).into_ret()
        });
        assert_eq!(run(&mut e, 0x0059_d3a0, first, 0, 0), (true, 1.0));
    }

    #[test]
    fn detected_needs_both_actors_and_both_processes() {
        let mut e = engine();
        let detector = actor(&mut e);
        let subject = actor(&mut e);
        e.set_global(PLAYER, 0x00ee_1111u32);
        e.register(ACTOR_GET_PROCESS, |e, a| {
            // The "process" of an actor is the object at +0x30; its vtable
            // holds the light level virtual at +0x734.
            e.mem.u32(a[0] + 0x30).into_ret()
        });
        let process = object(&mut e, &[(0x734, 0x0f00_0070)]);
        e.mem.set_u32(detector + 0x30, 0x8888);
        e.mem.set_u32(subject + 0x30, process);
        e.register(0x0f00_0070, |_, _| Ret {
            st0: 37.9,
            ..Ret::default()
        });
        stub(&mut e, 0x0049_3bb0, 1);
        stub(&mut e, 0x008a_0d10, 25);
        e.register(FTOL, |_, a| (f64::take(a, &mut 0) as i32).into_ret());
        assert_eq!(run(&mut e, 0x0059_d6e0, detector, subject, 0), (true, 1.0));

        // The call: this = detector, (0, subject, &flag, 0, in_combat, 0, 0).
        trace_on(&mut e);
        run(&mut e, 0x0059_d6e0, detector, subject, 0);
        let log = take_log(&mut e);
        let detection = calls_to(&log, 0x008a_0d10);
        assert_eq!(detection.len(), 1);
        assert_eq!(detection[0][0], detector);
        assert_eq!(detection[0][2], subject);
        assert_eq!(detection[0][5], 1);
        // "GetDetected >> %i and light %i": level 25, light 37.
        assert_eq!(calls_to(&log, DEBUG_PRINT), vec![vec![0x0103_4fb8, 25, 37]]);

        // Level 0: not detected, still no error.
        stub(&mut e, 0x008a_0d10, 0);
        assert_eq!(run(&mut e, 0x0059_d6e0, detector, subject, 0), (true, 0.0));

        // The subject being the player uses the combat query with an out flag.
        e.set_global(PLAYER, subject);
        stub(&mut e, 0x0095_3c50, 0);
        e.call_log = Some(vec![]);
        run(&mut e, 0x0059_d6e0, detector, subject, 0);
        let log = take_log(&mut e);
        assert_eq!(calls_to(&log, 0x0095_3c50).len(), 1);
        assert_eq!(calls_to(&log, 0x0049_3bb0).len(), 0);

        // A missing process: nothing is asked, level 0 and light -1 stay.
        e.mem.set_u32(detector + 0x30, 0);
        trace_on(&mut e);
        assert_eq!(run(&mut e, 0x0059_d6e0, detector, subject, 0), (true, 0.0));
        let log = take_log(&mut e);
        assert_eq!(
            calls_to(&log, DEBUG_PRINT),
            vec![vec![0x0103_4fb8, 0, (-1i32) as u32]]
        );
    }

    #[test]
    fn dead_asks_the_actor_virtual_with_one() {
        let mut e = engine();
        e.register(0x0f00_0080, |_, a| (a[1] == 1).into_ret());
        let dead = object(
            &mut e,
            &[(VSLOT_IS_ACTOR, IS_ACTOR_YES), (0x22c, 0x0f00_0080)],
        );
        assert_eq!(run(&mut e, 0x0059_d840, dead, 0, 0), (true, 1.0));
        stub(&mut e, 0x0f00_0081, 0);
        let alive = object(
            &mut e,
            &[(VSLOT_IS_ACTOR, IS_ACTOR_YES), (0x22c, 0x0f00_0081)],
        );
        assert_eq!(run(&mut e, 0x0059_d840, alive, 0, 0), (true, 0.0));
        let other = non_actor(&mut e);
        assert_eq!(run(&mut e, 0x0059_d840, other, 0, 0), (true, 0.0));
    }

    /// A bound object (virtual `IsBoundObject` yes).
    fn bound_object(e: &mut Engine) -> u32 {
        stub(e, 0x0f00_0090, 1);
        object(e, &[(VSLOT_IS_BOUND_OBJECT, 0x0f00_0090)])
    }

    #[test]
    fn item_count_of_one_item_and_of_a_form_list() {
        let mut e = engine();
        stub(&mut e, REF_HAS_CONTAINER, 0x1111);
        stub(&mut e, GET_INVENTORY_CHANGES, 0x3000);
        // Counts by item address: the first item has 4, the second 6.
        let first = bound_object(&mut e);
        let second = bound_object(&mut e);
        e.register_double(INVENTORY_GET_OBJECT_COUNT, move |_, a| {
            assert_eq!(a[0], 0x3000);
            (if a[1] == first { 4u32 } else { 6 }).into_ret()
        });
        // The CRT function passes the count through.
        e.register(ITEM_COUNT_FILTER, |_, a| a[0].into_ret());
        assert_eq!(run(&mut e, 0x0059_d8e0, 0x2000, first, 0), (true, 4.0));

        // A form list (type 0x55; itself not a bound object) of both items:
        // 4 + 6. Its list head node is embedded at +0x18.
        let list_object = object(&mut e, &[(VSLOT_IS_BOUND_OBJECT, IS_ACTOR_NO)]);
        let second_node = e.mem.alloc(8);
        e.mem.set_u32(list_object + 0x18, first);
        e.mem.set_u32(list_object + 0x1c, second_node);
        e.mem.set_u32(second_node, second);
        stub(&mut e, FORM_GET_TYPE, FORM_TYPE_FORM_LIST);
        assert_eq!(
            run(&mut e, 0x0059_d8e0, 0x2000, list_object, 0),
            (true, 10.0)
        );

        // Not a form list and not a bound object: nothing is counted.
        stub(&mut e, FORM_GET_TYPE, 0x28);
        assert_eq!(
            run(&mut e, 0x0059_d8e0, 0x2000, list_object, 0),
            (true, 0.0)
        );
        assert_eq!(run(&mut e, 0x0059_d8e0, 0x2000, 0, 0), (true, 0.0));

        // No inventory changes: 0.
        stub(&mut e, GET_INVENTORY_CHANGES, 0);
        assert_eq!(run(&mut e, 0x0059_d8e0, 0x2000, first, 0), (true, 0.0));
    }

    #[test]
    fn item_count_without_a_container_prints_the_complaint() {
        let mut e = engine();
        stub(&mut e, REF_HAS_CONTAINER, 0);
        trace_on(&mut e);
        assert_eq!(run(&mut e, 0x0059_d8e0, 0x2000, 0x5000, 0), (true, 0.0));
        let log = take_log(&mut e);
        assert_eq!(calls_to(&log, DEBUG_PRINT), vec![vec![0x0103_5004]]);
        assert_eq!(calls_to(&log, GET_INVENTORY_CHANGES).len(), 0);
    }

    #[test]
    fn equipped_checks_one_item_or_a_whole_list() {
        let mut e = engine();
        let actor = actor(&mut e);
        stub(&mut e, GET_INVENTORY_CHANGES, 0x3000);
        let worn = bound_object(&mut e);
        let other_item = bound_object(&mut e);
        // Single items: the form type is not a form list.
        stub(&mut e, FORM_GET_TYPE, 0x28);
        e.register_double(0x004b_fda0, move |_, a| {
            assert_eq!(a[0], 0x3000);
            assert_eq!(a[2], 0);
            (a[1] == worn).into_ret()
        });
        assert_eq!(run(&mut e, 0x0059_da90, actor, worn, 0), (true, 1.0));
        assert_eq!(run(&mut e, 0x0059_da90, actor, other_item, 0), (true, 0.0));

        // A list of [other_item, worn]: found at the second entry.
        let list_object = object(&mut e, &[(VSLOT_IS_BOUND_OBJECT, IS_ACTOR_NO)]);
        let second_node = e.mem.alloc(8);
        e.mem.set_u32(list_object + 0x18, other_item);
        e.mem.set_u32(list_object + 0x1c, second_node);
        e.mem.set_u32(second_node, worn);
        stub(&mut e, FORM_GET_TYPE, FORM_TYPE_FORM_LIST);
        assert_eq!(run(&mut e, 0x0059_da90, actor, list_object, 0), (true, 1.0));
        // Without the worn item in it, nothing is equipped.
        e.mem.set_u32(second_node, other_item);
        assert_eq!(run(&mut e, 0x0059_da90, actor, list_object, 0), (true, 0.0));

        // A non-actor, or no form: 0.
        let plain = non_actor(&mut e);
        assert_eq!(run(&mut e, 0x0059_da90, plain, worn, 0), (true, 0.0));
        assert_eq!(run(&mut e, 0x0059_da90, actor, 0, 0), (true, 0.0));
    }

    #[test]
    fn gold_counts_the_caps_form_in_the_inventory() {
        let mut e = engine();
        stub(&mut e, 0x0048_39c0, 0xca95);
        stub(&mut e, REF_HAS_CONTAINER, 1);
        stub(&mut e, GET_INVENTORY_CHANGES, 0x3000);
        e.register_double(INVENTORY_GET_OBJECT_COUNT, |_, a| {
            assert_eq!((a[0], a[1]), (0x3000, 0xca95));
            250u32.into_ret()
        });
        e.call_log = Some(vec![]);
        assert_eq!(run(&mut e, 0x0059_dbe0, 0x2000, 0, 0), (true, 250.0));
        let log = take_log(&mut e);
        assert_eq!(calls_to(&log, 0x0048_39c0), vec![vec![0xf]]);

        // No container, no inventory, no caps form or no reference: 0.
        stub(&mut e, REF_HAS_CONTAINER, 0);
        assert_eq!(run(&mut e, 0x0059_dbe0, 0x2000, 0, 0), (true, 0.0));
        stub(&mut e, REF_HAS_CONTAINER, 1);
        stub(&mut e, GET_INVENTORY_CHANGES, 0);
        assert_eq!(run(&mut e, 0x0059_dbe0, 0x2000, 0, 0), (true, 0.0));
        stub(&mut e, 0x0048_39c0, 0);
        assert_eq!(run(&mut e, 0x0059_dbe0, 0x2000, 0, 0), (true, 0.0));
        assert_eq!(run(&mut e, 0x0059_dbe0, 0, 0, 0), (true, 0.0));
    }
}
