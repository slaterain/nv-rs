//! xNVSE's `Commands_Packages.cpp` (xNVSE): the target, subject and speaker of an actor's
//! dialogue package, an actor's current package, a package's location and target, and an actor
//! base's package list (its count, a package by index set, added or taken out, all taken out).
//!
//! Read from xNVSE's source (xNVSE 6.4.9, `0ccd23a`). Kept as xNVSE has them: `SetNthPackage`,
//! `AddPackageAt` and `RemovePackageAt` read the index -1 as the end (`eListEnd`), and for
//! `RemovePackageAt` that takes out the FIRST package (`tList::RemoveNth` takes any index of 0 or
//! less as the head). `AddPackageAt` at the end answers the index of the package it went after,
//! and an index one past the last package adds nothing (-1). `GetNthPackage` reads -1 as no
//! package and -2 as the last. A location's radius is a whole number (a float set is truncated).

use super::misc_forms::{add_at, count, nth_item, remove_nth, replace_nth};
use super::string::dtoui3;
use super::{console_print, double_words, dynamic_cast, is_console_mode, rtti, Read};
use crate::table::{Command, Param, Returns};
use crate::{handled, CommandArgs, EvalArgs};
use engine_core::prelude::*;

const fn p(name: &'static str, kind: u32, optional: bool) -> Param {
    Param {
        name,
        kind,
        optional,
    }
}
/// `kParamType_Integer`, `_Float`, `_ObjectRef`, `_Actor` and `_AnyForm` (xNVSE `CommandTable.h`).
const INT: u32 = 0x01;
const FLOAT: u32 = 0x02;
const OBJECT_REF: u32 = 0x04;
const ACTOR: u32 = 0x06;
const ANY_FORM: u32 = 0x3d;
/// `kParams_OneOptionalActorRef`, `kParams_OneOptionalObjectRef`,
/// `kParams_OneForm_OneOptionalObjectRef`, `kParams_OneForm`, `kParams_OneForm_OneFloat`,
/// `kParams_OneForm_OneInt`, `kParams_OneIndexOneOptionalObjectRef` and
/// `kParams_OnePackageOneIndexOneOptionalObjectRef` (xNVSE `ParamInfos.h`).
const ONE_OPTIONAL_ACTOR_REF: &[Param] = &[p("actor reference", ACTOR, true)];
const ONE_OPTIONAL_OBJECT_REF: &[Param] = &[p("ref", OBJECT_REF, true)];
const FORM_OPTIONAL_REF: &[Param] = &[p("form", ANY_FORM, false), p("ref", OBJECT_REF, true)];
const ONE_FORM: &[Param] = &[p("form", ANY_FORM, false)];
const FORM_FLOAT: &[Param] = &[p("form", ANY_FORM, false), p("float", FLOAT, false)];
const FORM_INT: &[Param] = &[p("form", ANY_FORM, false), p("int", INT, false)];
const INDEX_OPTIONAL_REF: &[Param] = &[p("index", INT, false), p("ref", OBJECT_REF, true)];
const PACKAGE_INDEX_OPTIONAL_REF: &[Param] = &[
    p("package", ANY_FORM, false),
    p("index", INT, false),
    p("ref", OBJECT_REF, true),
];

#[allow(clippy::too_many_arguments)]
const fn cmd(
    opcode: u32,
    name: &'static str,
    short: &'static str,
    help: &'static str,
    needs_ref: bool,
    params: &'static [Param],
    execute: AbiFn,
    eval: Option<AbiFn>,
    returns: Returns,
) -> Command {
    Command {
        opcode,
        name,
        short,
        help,
        needs_ref,
        params,
        execute,
        eval,
        returns,
    }
}

const RD: Returns = Returns::Default;
const RF: Returns = Returns::Form;

#[rustfmt::skip]
pub static COMMANDS: &[Command] = &[
    cmd(0x14fa, "GetDialogueTarget", "GDT", "get the NPC talked to", true, ONE_OPTIONAL_ACTOR_REF, get_dialogue_target, Some(get_dialogue_target_eval), RF),
    cmd(0x14fb, "GetDialogueSubject", "GDS", "get the NPC who started the conversation", true, ONE_OPTIONAL_ACTOR_REF, get_dialogue_subject, Some(get_dialogue_subject_eval), RF),
    cmd(0x14fc, "GetDialogueSpeaker", "GDK", "get the NPC who is currently speaking in the conversation", true, ONE_OPTIONAL_ACTOR_REF, get_dialogue_speaker, Some(get_dialogue_speaker_eval), RF),
    cmd(0x14fd, "SetPackageLocationReference", "", "sets package start location as a reference", false, FORM_OPTIONAL_REF, set_package_location_reference, None, RF),
    cmd(0x1513, "GetCurrentPackage", "", "gets the current package from an actor", false, ONE_OPTIONAL_OBJECT_REF, get_current_package, None, RF),
    cmd(0x1514, "GetPackageLocation", "GetPackageLocationReference", "gets package start location", false, ONE_FORM, get_package_location, None, RD),
    cmd(0x1515, "GetPackageCount", "", "gets the count of packages from an actor base form", false, ONE_OPTIONAL_OBJECT_REF, get_package_count, Some(get_package_count_eval), RD),
    cmd(0x1516, "GetNthPackage", "", "gets the Nth package from an actor base form", false, INDEX_OPTIONAL_REF, get_nth_package, None, RF),
    cmd(0x1517, "SetNthPackage", "", "sets and returns the Nth package to an actor base form", false, PACKAGE_INDEX_OPTIONAL_REF, set_nth_package, None, RD),
    cmd(0x1518, "AddPackageAt", "", "adds the Nth package to an actor base form : 0 at top -1 at end returns index", false, PACKAGE_INDEX_OPTIONAL_REF, add_package_at, None, RD),
    cmd(0x1519, "RemovePackageAt", "", "removes and returns the Nth package from an actor base form", false, INDEX_OPTIONAL_REF, remove_package_at, None, RD),
    cmd(0x151a, "RemoveAllPackages", "", "removes all packages from an actor base form returns count removed", false, ONE_OPTIONAL_OBJECT_REF, remove_all_packages, None, RD),
    cmd(0x151c, "SetPackageTargetReference", "", "sets package target as a reference", false, FORM_OPTIONAL_REF, set_package_target_reference, None, RD),
    cmd(0x151d, "SetPackageTargetCount", "SetPackageTargetDistance", "sets package target count or distance", false, FORM_INT, set_package_target_count, None, RD),
    cmd(0x151e, "GetPackageTargetCount", "GetPackageTargetDistance", "gets package target count or distance", false, ONE_FORM, get_package_target_count, None, RD),
    cmd(0x151f, "SetPackageLocationRadius", "", "sets package start location radius", false, FORM_FLOAT, set_package_location_radius, None, RD),
    cmd(0x1520, "GetPackageLocationRadius", "", "gets package start location radius", false, ONE_FORM, get_package_location_radius, None, RD),
];

/// `TESForm::refID` (`+0xC`) and `TESForm::GetName` (virtual `+0x130`); `TESObjectREFR::baseForm`
/// (`+0x20`).
const FORM_ID: u32 = 0xc;
const GET_NAME: u32 = 0x130;
const BASE_FORM: u32 = 0x20;
/// `Actor::baseProcess` (`+0x68`) and `BaseProcess::GetCurrentPackage` (virtual `+0x27C`, the
/// 160th function of `BaseProcess`'s table).
const BASE_PROCESS: u32 = 0x68;
const GET_CURRENT_PACKAGE: u32 = 0x27c;
/// `DialoguePackage`: `speaker` (`+0x94`), `subject` (`+0xB0`), `target` (`+0xB4`).
const SPEAKER: u32 = 0x94;
const SUBJECT: u32 = 0xb0;
const TARGET: u32 = 0xb4;
/// `TESPackage::location` (`+0x2C`) and `::target` (`+0x30`).
const LOCATION: u32 = 0x2c;
const PACKAGE_TARGET: u32 = 0x30;
/// `TESPackage::LocationData` (`0xC` bytes): `locationType` (the byte at `+0`), `radius`
/// (`+4`), `object` (`+8`), and the kinds the commands read (`kPackLocation_*`).
const LOCATION_SIZE: u32 = 0xc;
const RADIUS: u32 = 4;
const LOCATION_OBJECT: u32 = 8;
const NEAR_REFERENCE: u8 = 0;
const IN_CELL: u8 = 1;
const CURRENT_LOCATION: u8 = 2;
const OBJECT_ID: u8 = 4;
const OBJECT_TYPE: u8 = 5;
/// `TESPackage::TargetData` (`0x10` bytes): `targetType` (the byte at `+0`), `target` (`+4`),
/// `count` (`+8`); `kTargetType_Refr`, `kTargetType_TypeCode`, `kObjectType_Activators`, and the
/// distance `SetTarget` gives a reference.
const TARGET_SIZE: u32 = 0x10;
const TARGET_OBJECT: u32 = 4;
const COUNT: u32 = 8;
const TARGET_REFR: u8 = 0;
const TARGET_TYPE_CODE: u8 = 2;
const ACTIVATORS: u32 = 1;
const DEFAULT_DISTANCE: u32 = 150;
/// `TESAIForm::packageList` (`+0x18`, a `tList`).
const PACKAGE_LIST: u32 = 0x18;
/// `FormHeap_Allocate` and `FormHeap_Free`.
const FORM_HEAP_ALLOCATE: u32 = 0x0040_1000;
const FORM_HEAP_FREE: u32 = 0x0040_1030;
/// `eListEnd` (xNVSE `GameTypes.h`).
const LIST_END: i32 = -2;

fn form_id(e: &Engine, form: u32) -> u32 {
    e.mem.u32(form + FORM_ID)
}

/// The package the actor's process runs (`baseProcess->GetCurrentPackage()`); 0 without a
/// process.
fn current_package(e: &mut Engine, actor: u32) -> u32 {
    let process = e.mem.u32(actor + BASE_PROCESS);
    if process == 0 {
        return 0;
    }
    e.vcall(process, GET_CURRENT_PACKAGE, &args![]).u32()
}

/// The argument's reference, or else the calling reference (`if (!pRefr) if (!thisObj) return
/// true; else pRefr = thisObj;`); `None` with neither.
fn ref_or_caller(form: u32, this_obj: u32) -> Option<u32> {
    match (form, this_obj) {
        (0, 0) => None,
        (0, caller) => Some(caller),
        (form, _) => Some(form),
    }
}

/// The package list of the reference's base form when the reference is an actor and its base
/// form has one (`DYNAMIC_CAST(pActor->baseForm, TESForm, TESAIForm)`); `None` otherwise.
fn package_list(e: &mut Engine, refr: u32) -> Option<u32> {
    let actor = dynamic_cast(e, refr, rtti::TES_FORM, rtti::ACTOR);
    if actor == 0 {
        return None;
    }
    let base = e.mem.u32(actor + BASE_FORM);
    let ai = dynamic_cast(e, base, rtti::TES_FORM, rtti::TES_AI_FORM);
    (ai != 0).then_some(ai + PACKAGE_LIST)
}

/// The form as a package (`DYNAMIC_CAST(pForm, TESForm, TESPackage)`), 0 for any other form.
fn as_package(e: &mut Engine, form: u32) -> u32 {
    dynamic_cast(e, form, rtti::TES_FORM, rtti::TES_PACKAGE)
}

/// `-1` read as the end (`anIndex == -1 ? eListEnd : anIndex`).
fn end_for_minus_one(index: i32) -> i32 {
    if index == -1 {
        LIST_END
    } else {
        index
    }
}

// ---- dialogue ----

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_Packages.cpp:
// Cmd_GetDialogueTarget_Eval, Cmd_GetDialogueSubject_Eval, Cmd_GetDialogueSpeaker_Eval.
/// The id of the actor at `field` of the calling actor's current package, when that is a
/// dialogue package, in the result's low word (0 for none or another package); the console
/// prints it.
fn dialogue_eval(e: &mut Engine, this_obj: u32, result: u32, field: u32, line: &str) {
    e.mem.set_f64(result, 0.0);
    let actor = dynamic_cast(e, this_obj, rtti::TES_FORM, rtti::ACTOR);
    let package = if actor != 0 {
        current_package(e, actor)
    } else {
        0
    };
    if package != 0 {
        let dialogue = dynamic_cast(e, package, rtti::TES_PACKAGE, rtti::DIALOGUE_PACKAGE);
        if dialogue != 0 {
            let who = e.mem.u32(dialogue + field);
            if who != 0 {
                e.mem.set_u32(result, form_id(e, who));
            }
        }
    }
    if is_console_mode(e) {
        let id = e.mem.u32(result);
        console_print(e, line, &[id]);
    }
}

/// The condition's eval (`COMMAND_ARGS_EVAL`): the calling reference only.
fn dialogue_eval_words(e: &mut Engine, w: &[u32], field: u32, line: &str) -> Ret {
    let a = EvalArgs::new(w);
    dialogue_eval(e, a.this_obj, a.result, field, line);
    handled(true)
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_Packages.cpp:
// Cmd_GetDialogueTarget_Execute, Cmd_GetDialogueSubject_Execute, Cmd_GetDialogueSpeaker_Execute.
/// The eval on the calling reference (the argument is not read); 0 without one.
fn dialogue_execute(e: &mut Engine, w: &[u32], field: u32, line: &str) -> Ret {
    let a = CommandArgs::new(w);
    a.set_result(e, 0.0);
    if a.this_obj != 0 {
        dialogue_eval(e, a.this_obj, a.result, field, line);
    }
    handled(true)
}

const TARGET_LINE: &str = "GetDialogueTarget >> %10X";
const SUBJECT_LINE: &str = "GetDialogueSubject >> %10X";
const SPEAKER_LINE: &str = "GetDialogueSpeaker >> %10X";

fn get_dialogue_target_eval(e: &mut Engine, w: &[u32]) -> Ret {
    dialogue_eval_words(e, w, TARGET, TARGET_LINE)
}
fn get_dialogue_target(e: &mut Engine, w: &[u32]) -> Ret {
    dialogue_execute(e, w, TARGET, TARGET_LINE)
}
fn get_dialogue_subject_eval(e: &mut Engine, w: &[u32]) -> Ret {
    dialogue_eval_words(e, w, SUBJECT, SUBJECT_LINE)
}
fn get_dialogue_subject(e: &mut Engine, w: &[u32]) -> Ret {
    dialogue_execute(e, w, SUBJECT, SUBJECT_LINE)
}
fn get_dialogue_speaker_eval(e: &mut Engine, w: &[u32]) -> Ret {
    dialogue_eval_words(e, w, SPEAKER, SPEAKER_LINE)
}
fn get_dialogue_speaker(e: &mut Engine, w: &[u32]) -> Ret {
    dialogue_execute(e, w, SPEAKER, SPEAKER_LINE)
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_Packages.cpp:
// Cmd_GetCurrentPackage_Execute.
/// The package the actor (the argument, else the calling reference) runs, its id in the result's
/// low word; nothing for a reference that is not an actor or has no process. The console prints
/// the result's low word.
fn get_current_package(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    a.set_result(e, 0.0);
    let read = Read::of(e, &a);
    let Some(refr) = ref_or_caller(read.form(0), a.this_obj) else {
        return handled(true);
    };
    let actor = dynamic_cast(e, refr, rtti::TES_OBJECT_REFR, rtti::ACTOR);
    if actor == 0 || e.mem.u32(actor + BASE_PROCESS) == 0 {
        return handled(true);
    }
    let package = current_package(e, actor);
    if package != 0 {
        a.set_form_result(e, form_id(e, package));
    }
    if is_console_mode(e) {
        // The format's `%08X` takes the low word of the double passed.
        let words = double_words(e.mem.f64(a.result));
        console_print(e, "GetCurrentPackage >> [%08X] ", &words);
    }
    handled(true)
}

// ---- location and target ----

/// `TESPackage::GetLocationData`: the package's location, made when it has none
/// (`LocationData::Create`: the current location, no object, radius 0).
fn location_data(e: &mut Engine, package: u32) -> u32 {
    let location = e.mem.u32(package + LOCATION);
    if location != 0 {
        return location;
    }
    let made = e.call(FORM_HEAP_ALLOCATE, &args![LOCATION_SIZE]).u32();
    e.mem.set_u8(made, CURRENT_LOCATION);
    e.mem.set_u32(made + LOCATION_OBJECT, 0);
    e.mem.set_u32(made + RADIUS, 0);
    e.mem.set_u32(package + LOCATION, made);
    made
}

/// `TESPackage::GetTargetData`: the package's target, made when it has none
/// (`TargetData::Create`: count 0, the activators' type code).
fn target_data(e: &mut Engine, package: u32) -> u32 {
    let target = e.mem.u32(package + PACKAGE_TARGET);
    if target != 0 {
        return target;
    }
    let made = e.call(FORM_HEAP_ALLOCATE, &args![TARGET_SIZE]).u32();
    e.mem.set_u32(made + COUNT, 0);
    e.mem.set_u32(made + TARGET_OBJECT, ACTIVATORS);
    e.mem.set_u8(made, TARGET_TYPE_CODE);
    e.mem.set_u32(package + PACKAGE_TARGET, made);
    made
}

/// The form (the first argument) as a package that has a location: the location; 0 otherwise.
fn existing_location(e: &mut Engine, form: u32) -> u32 {
    let package = as_package(e, form);
    if package == 0 {
        return 0;
    }
    e.mem.u32(package + LOCATION)
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_Packages.cpp:
// Cmd_SetPackageLocationReference_Execute.
/// The package's location (made when it has none) set near the reference (the second argument,
/// else the calling reference). Nothing without a reference, a form or for a form that is not a
/// package.
fn set_package_location_reference(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    a.set_result(e, 0.0);
    let read = Read::of(e, &a);
    let Some(refr) = ref_or_caller(read.form(1), a.this_obj) else {
        return handled(true);
    };
    let package = as_package(e, read.form(0));
    if package != 0 {
        let location = location_data(e, package);
        e.mem.set_u8(location, NEAR_REFERENCE);
        e.mem.set_u32(location + LOCATION_OBJECT, refr);
    }
    handled(true)
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_Packages.cpp:
// Cmd_GetPackageLocation_Execute.
/// In the result's low word: the location's form id for a location near a reference, in a cell
/// or at an object, its type code for an object type; 0 for the other kinds, no object, or a
/// package without a location (none is made).
fn get_package_location(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    a.set_result(e, 0.0);
    let read = Read::of(e, &a);
    let location = existing_location(e, read.form(0));
    if location == 0 {
        return handled(true);
    }
    let object = e.mem.u32(location + LOCATION_OBJECT);
    if object != 0 {
        match e.mem.u8(location) {
            NEAR_REFERENCE | IN_CELL | OBJECT_ID => a.set_form_result(e, form_id(e, object)),
            OBJECT_TYPE => a.set_form_result(e, object),
            _ => {}
        }
    }
    handled(true)
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_Packages.cpp:
// Cmd_SetPackageLocationRadius_Execute.
/// The location's radius set to the float, truncated to a whole number (MSVC's `__ftoui3`), when
/// the package has a location (none is made).
fn set_package_location_radius(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    a.set_result(e, 0.0);
    let read = Read::of(e, &a);
    let radius = read.float(1, 0.0);
    let location = existing_location(e, read.form(0));
    if location != 0 {
        e.mem.set_u32(location + RADIUS, dtoui3(f64::from(radius)));
    }
    handled(true)
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_Packages.cpp:
// Cmd_GetPackageLocationRadius_Execute.
/// The location's radius (unsigned); 0 for a package without a location.
fn get_package_location_radius(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    a.set_result(e, 0.0);
    let read = Read::of(e, &a);
    let location = existing_location(e, read.form(0));
    if location != 0 {
        let radius = e.mem.u32(location + RADIUS);
        a.set_result(e, f64::from(radius));
    }
    handled(true)
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_Packages.cpp:
// Cmd_SetPackageTargetReference_Execute; GameForms.cpp: TESPackage::SetTarget(TESObjectREFR*).
/// The package's target (made when it has none) set to the reference (the second argument, else
/// the calling reference) at the default distance, 150.
fn set_package_target_reference(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    a.set_result(e, 0.0);
    let read = Read::of(e, &a);
    let Some(refr) = ref_or_caller(read.form(1), a.this_obj) else {
        return handled(true);
    };
    let package = as_package(e, read.form(0));
    if package != 0 {
        let target = target_data(e, package);
        e.mem.set_u8(target, TARGET_REFR);
        e.mem.set_u32(target + TARGET_OBJECT, refr);
        e.mem.set_u32(target + COUNT, DEFAULT_DISTANCE);
    }
    handled(true)
}

/// The form as a package that has a target: the target; 0 otherwise.
fn existing_target(e: &mut Engine, form: u32) -> u32 {
    let package = as_package(e, form);
    if package == 0 {
        return 0;
    }
    e.mem.u32(package + PACKAGE_TARGET)
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_Packages.cpp:
// Cmd_SetPackageTargetCount_Execute; GameForms.cpp: TESPackage::SetCount.
/// The target's count (or distance) set, when the package has a target (none is made).
fn set_package_target_count(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    a.set_result(e, 0.0);
    let read = Read::of(e, &a);
    let count = read.int(1, 0);
    let target = existing_target(e, read.form(0));
    if target != 0 {
        e.mem.set_u32(target + COUNT, count);
    }
    handled(true)
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_Packages.cpp:
// Cmd_GetPackageTargetCount_Execute.
/// The target's count (or distance, unsigned); 0 for a package without a target.
fn get_package_target_count(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    a.set_result(e, 0.0);
    let read = Read::of(e, &a);
    let target = existing_target(e, read.form(0));
    if target != 0 {
        let count = e.mem.u32(target + COUNT);
        a.set_result(e, f64::from(count));
    }
    handled(true)
}

// ---- an actor base's packages ----

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_Packages.cpp:
// Cmd_GetPackageCount_Eval; GameTypes.h: tList::Count.
/// The number of packages of the actor's base form (0 for an empty list, a reference that is not
/// an actor or a base without packages); the console prints it.
fn package_count_eval(e: &mut Engine, refr: u32, result: u32) {
    e.mem.set_f64(result, 0.0);
    if let Some(list) = package_list(e, refr) {
        let n = count(e, list);
        e.mem.set_f64(result, f64::from(n));
    }
    if is_console_mode(e) {
        let n = dtoui3(e.mem.f64(result));
        console_print(e, "GetPackageCount >> %u", &[n]);
    }
}

fn get_package_count_eval(e: &mut Engine, w: &[u32]) -> Ret {
    let a = EvalArgs::new(w);
    package_count_eval(e, a.this_obj, a.result);
    handled(true)
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_Packages.cpp:
// Cmd_GetPackageCount_Execute.
/// The eval on the argument, else the calling reference; 0 without either.
fn get_package_count(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    a.set_result(e, 0.0);
    let read = Read::of(e, &a);
    if let Some(refr) = ref_or_caller(read.form(0), a.this_obj) {
        package_count_eval(e, refr, a.result);
    }
    handled(true)
}

/// The list commands' reference: the argument at `i`, else the calling reference.
fn list_of(e: &mut Engine, read: &Read, i: usize, this_obj: u32) -> Option<u32> {
    let refr = ref_or_caller(read.form(i), this_obj)?;
    package_list(e, refr)
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_Packages.cpp:
// Cmd_GetNthPackage_Execute; GameTypes.h: tList::GetNthItem.
/// The package at the index (-2 the last, -1 none), its id in the result's low word; the console
/// prints its id and name when there is one.
fn get_nth_package(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    a.set_result(e, 0.0);
    let read = Read::of(e, &a);
    let index = read.int(0, 0) as i32;
    let Some(list) = list_of(e, &read, 1, a.this_obj) else {
        return handled(true);
    };
    let package = nth_item(e, list, index);
    if package == 0 {
        return handled(true);
    }
    let id = form_id(e, package);
    a.set_form_result(e, id);
    if is_console_mode(e) {
        let name = e.vcall(package, GET_NAME, &args![]).u32();
        console_print(e, "GetNthPackage >> %08x (%s)", &[id, name]);
    }
    handled(true)
}

/// The package argument (the first) for the list commands that take one: 0 when the read stopped
/// before it, where xNVSE's pointer is left unset.
fn package_argument(e: &mut Engine, read: &Read) -> u32 {
    as_package(e, read.form(0))
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_Packages.cpp:
// Cmd_SetNthPackage_Execute; GameTypes.h: tList::ReplaceNth.
/// The package at the index (-1 or -2 the last) replaced by the given one; the replaced package's
/// id in the result's low word (0 when there was none there; past the end nothing changes).
fn set_nth_package(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    a.set_result(e, 0.0);
    let read = Read::of(e, &a);
    let index = read.int(1, 0) as i32;
    let Some(list) = list_of(e, &read, 2, a.this_obj) else {
        return handled(true);
    };
    let package = package_argument(e, &read);
    if package != 0 {
        let replaced = replace_nth(e, list, end_for_minus_one(index), package);
        if replaced != 0 {
            a.set_form_result(e, form_id(e, replaced));
        }
    }
    handled(true)
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_Packages.cpp:
// Cmd_AddPackageAt_Execute; GameTypes.h: tList::AddAt.
/// The package put at the index (0 the top, -1 or -2 the end); the index answered (for the end,
/// the index of the package it went after), -1 when nothing was added.
fn add_package_at(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    a.set_result(e, 0.0);
    let read = Read::of(e, &a);
    let index = read.int(1, 0) as i32;
    let Some(list) = list_of(e, &read, 2, a.this_obj) else {
        return handled(true);
    };
    let package = package_argument(e, &read);
    if package != 0 {
        let at = add_at(e, list, package, end_for_minus_one(index));
        a.set_result(e, f64::from(at));
    }
    handled(true)
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_Packages.cpp:
// Cmd_RemovePackageAt_Execute; GameTypes.h: tList::RemoveNth.
/// The package at the index taken out (0, -1, -2 and any lower index: the first), its id in the
/// result's low word; 0 past the end.
fn remove_package_at(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    a.set_result(e, 0.0);
    let read = Read::of(e, &a);
    let index = read.int(0, 0) as i32;
    let Some(list) = list_of(e, &read, 1, a.this_obj) else {
        return handled(true);
    };
    let removed = remove_nth(e, list, end_for_minus_one(index));
    if removed != 0 {
        a.set_form_result(e, form_id(e, removed));
    }
    handled(true)
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), GameTypes.h: tList::RemoveAll.
/// The head emptied and every other node freed (the packages are not).
fn remove_all(e: &mut Engine, head: u32) {
    let mut node = e.mem.u32(head + 4);
    e.mem.set_u32(head, 0);
    e.mem.set_u32(head + 4, 0);
    let mut walked = 0;
    while node != 0 && crate::invref::list_step(e, &mut walked) {
        let next = e.mem.u32(node + 4);
        e.call(FORM_HEAP_FREE, &args![node]);
        node = next;
    }
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_Packages.cpp:
// Cmd_RemoveAllPackages_Execute; GameForms.h: TESAIForm::RemoveAllPackages.
/// Every package taken out; the number there was.
fn remove_all_packages(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    a.set_result(e, 0.0);
    let read = Read::of(e, &a);
    let Some(list) = list_of(e, &read, 0, a.this_obj) else {
        return handled(true);
    };
    let before = count(e, list);
    remove_all(e, list);
    let after = count(e, list);
    a.set_result(e, f64::from(before.wrapping_sub(after)));
    handled(true)
}

#[cfg(test)]
#[path = "packages_tests.rs"]
mod tests;
