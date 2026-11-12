//! xNVSE's `Commands_Script.cpp` (xNVSE), the rest after `script.rs`: a form's script read, set
//! and taken away (`IsScripted`, `GetScript`, `RemoveScript`, `SetScript`, `RunScript`), another
//! script's variables by name (`GetVariable` and its kin, `SetVariable` and its kin), scripts
//! compared, a script's variables reset, its literal references counted and read, and the calling
//! script.
//!
//! Read from xNVSE's source (xNVSE 6.4.9, `0ccd23a`). Kept as xNVSE has them:
//! `GetNthExplicitRef` looks the FIRST literal reference up again on every pass of its loop, so
//! every index answers the first one (the loop never moves on). `CompareScripts` reads its scripts
//! as `kParamType_ObjectID`, which only takes inventory objects, so a script never passes the read
//! and the answer is always 0. `SetScript`'s check for a script of unknown type compares the 16-bit
//! type with 0x10000, which never holds, so it is left out.
//!
//! One difference: `GetNthExplicitRef` with the index -1 never ends in xNVSE (its count wraps); here
//! it answers nothing.

use super::string::assign_to_string_var;
use super::{base_form, console_print, dynamic_cast, is_console_mode, rtti, try_get_refr_parent};
use super::{double_words, Read};
use crate::args::{event_list_variable, reference_event_list, script_variable_by_name};
use crate::table::{Command, Param, Returns};
use crate::{handled, vars, CommandArgs};
use engine_core::prelude::*;

const fn p(name: &'static str, kind: u32, optional: bool) -> Param {
    Param {
        name,
        kind,
        optional,
    }
}
/// `kParamType_String`, `_Integer`, `_Float`, `_ObjectID`, `_Quest` and `_AnyForm` (xNVSE
/// `CommandTable.h`).
const STRING: u32 = 0x00;
const INT: u32 = 0x01;
const FLOAT: u32 = 0x02;
const OBJECT_ID: u32 = 0x03;
const QUEST: u32 = 0x0e;
const ANY_FORM: u32 = 0x3d;
/// `kParams_OneOptionalForm`, `kParams_OneForm_OneOptionalForm`, `kParams_OneForm`,
/// `kParams_OneInt_OneOptionalForm`, and `Commands_Script.h`'s own lists.
const ONE_OPTIONAL_FORM: &[Param] = &[p("form", ANY_FORM, true)];
const FORM_OPTIONAL_FORM: &[Param] = &[p("form", ANY_FORM, false), p("form", ANY_FORM, true)];
const ONE_FORM: &[Param] = &[p("form", ANY_FORM, false)];
const INT_OPTIONAL_FORM: &[Param] = &[p("int", INT, false), p("form", ANY_FORM, true)];
const GET_VARIABLE: &[Param] = &[p("variable name", STRING, false), p("quest", QUEST, true)];
const SET_NUM_VARIABLE: &[Param] = &[
    p("variable name", STRING, false),
    p("variable value", FLOAT, false),
    p("quest", QUEST, true),
];
const SET_REF_VARIABLE: &[Param] = &[
    p("variable name", STRING, false),
    p("variable value", ANY_FORM, false),
    p("quest", QUEST, true),
];
const SET_STR_VARIABLE: &[Param] = &[
    p("variable name", STRING, false),
    p("variable value", STRING, false),
    p("quest", QUEST, true),
];
const COMPARE_SCRIPTS: &[Param] = &[p("script", OBJECT_ID, false), p("script", OBJECT_ID, false)];

const fn cmd(
    opcode: u32,
    name: &'static str,
    short: &'static str,
    help: &'static str,
    params: &'static [Param],
    execute: AbiFn,
    returns: Returns,
) -> Command {
    Command {
        opcode,
        name,
        short,
        help,
        needs_ref: false,
        params,
        execute,
        eval: None,
        returns,
    }
}

const RD: Returns = Returns::Default;
const RF: Returns = Returns::Form;

#[rustfmt::skip]
pub static COMMANDS: &[Command] = &[
    cmd(0x14b6, "IsScripted", "", "returns 1 if the object or reference has a script attached to it.", ONE_OPTIONAL_FORM, is_scripted, RD),
    cmd(0x14b7, "GetScript", "", "returns the script of the reference or passed object.", ONE_OPTIONAL_FORM, get_script, RF),
    cmd(0x14b8, "RemoveScript", "", "removes the script of the reference or passed object.", ONE_OPTIONAL_FORM, remove_script, RD),
    cmd(0x14b9, "SetScript", "", "sets the script of the reference or passed object.", FORM_OPTIONAL_FORM, set_script, RD),
    cmd(0x152e, "GetVariable", "", "looks up the value of a variable by name", GET_VARIABLE, get_variable, RD),
    cmd(0x152f, "HasVariable", "", "returns true if the script has a variable with the specified name", GET_VARIABLE, has_variable, RD),
    cmd(0x1530, "GetRefVariable", "", "looks up the value of a ref variable by name", GET_VARIABLE, get_ref_variable, RF),
    cmd(0x1531, "GetArrayVariable", "GetArrayVar", "looks up an array variable by name on the calling object or specified quest", GET_VARIABLE, get_array_variable, Returns::Array),
    cmd(0x1532, "CompareScripts", "", "returns true if the compiled scripts are identical", COMPARE_SCRIPTS, compare_scripts, RD),
    cmd(0x1533, "ResetAllVariables", "", "sets all variables in a script to zero", &[], reset_all_variables, RD),
    cmd(0x1534, "GetNumExplicitRefs", "", "returns the number of literal references in a script", ONE_OPTIONAL_FORM, get_num_explicit_refs, RD),
    cmd(0x1535, "GetNthExplicitRef", "", "returns the nth literal reference in a script", INT_OPTIONAL_FORM, get_nth_explicit_ref, RF),
    cmd(0x1536, "RunScript", "", "debug", ONE_FORM, run_script, RD),
    cmd(0x1537, "GetCurrentScript", "", "returns the calling script", &[], get_current_script, RF),
    cmd(0x15be, "SetVariable", "", "sets the value of a variable by name", SET_NUM_VARIABLE, set_variable, RD),
    cmd(0x15bf, "SetRefVariable", "", "sets the value of a variable by name", SET_REF_VARIABLE, set_ref_variable, RD),
    cmd(0x167c, "GetStringVariable", "", "looks up a string variable by name on the calling object or specified quest", GET_VARIABLE, get_string_variable, Returns::String),
    cmd(0x167d, "SetStringVariable", "", "sets the value of a variable by name", SET_STR_VARIABLE, set_string_variable, RD),
];

/// `TESForm::refID` (`+0xC`), `typeID` (the byte at `+4`) and `kFormType_TESQuest`.
const FORM_ID: u32 = 0xc;
const FORM_TYPE: u32 = 4;
const TYPE_QUEST: u8 = 0x47;
/// `TESScriptableForm::script` (`+4`).
const SCRIPTABLE_SCRIPT: u32 = 4;
/// `EffectSetting::associatedItem` (`+0x60`) and `archtype` (`+0x98`; 1 is a script effect).
const EFFECT_ITEM: u32 = 0x60;
const EFFECT_ARCHTYPE: u32 = 0x98;
const ARCHTYPE_SCRIPT: u32 = 1;
/// `Script::info.dataLength` (`+0x20`), `info.type` (`+0x28`), `data` (`+0x30`), `quest`
/// (`+0x40`), `refList` (`+0x44`); the script types (`eType_*`).
const SCRIPT_DATA_LENGTH: u32 = 0x20;
const SCRIPT_TYPE: u32 = 0x28;
const SCRIPT_DATA: u32 = 0x30;
const SCRIPT_QUEST: u32 = 0x40;
const SCRIPT_REF_LIST: u32 = 0x44;
const TYPE_OBJECT: u16 = 0;
const TYPE_QUEST_SCRIPT: u16 = 1;
const TYPE_MAGIC: u16 = 0x100;
/// `RefVariable::form` (`+8`) and `varIdx` (`+0xC`).
const REF_VAR_FORM: u32 = 8;
const REF_VAR_INDEX: u32 = 0xc;
/// `TESQuest::scriptEventList` (`+0x5C`).
const QUEST_EVENT_LIST: u32 = 0x5c;
/// `ScriptLocal::data` (`+8`).
const LOCAL_DATA: u32 = 8;
/// `TESObjectREFR::extraDataList` (`+0x44`); `kExtraData_Script`, `ExtraScript`'s size and vtable,
/// `script` (`+0xC`) and `eventList` (`+0x10`).
const REFR_EXTRA: u32 = 0x44;
const EXTRA_SCRIPT: u32 = 0x0d;
const EXTRA_SCRIPT_SIZE: u32 = 0x14;
const EXTRA_SCRIPT_VTBL: u32 = 0x0101_5914;
const EXTRA_SCRIPT_SCRIPT: u32 = 0xc;
const EXTRA_SCRIPT_EVENT_LIST: u32 = 0x10;
/// `BaseExtraList::RemoveByType` (`thiscall`), `Script::CreateEventList` (`thiscall`) and
/// `Script::Execute` (`thiscall`: the reference, an event list, the container, a flag).
const EXTRA_REMOVE_BY_TYPE: u32 = 0x0041_0140;
const CREATE_EVENT_LIST: u32 = crate::functions::CREATE_EVENT_LIST;
const SCRIPT_EXECUTE: u32 = crate::functions::SCRIPT_EXECUTE;

fn form_id(e: &Engine, form: u32) -> u32 {
    e.mem.u32(form + FORM_ID)
}

fn script_type(e: &Engine, script: u32) -> u16 {
    e.mem.u16(script + SCRIPT_TYPE)
}

/// `EffectSetting::GetScript`: the effect's script when it is a script effect, 0 otherwise.
fn effect_script(e: &Engine, effect: u32) -> u32 {
    if e.mem.u32(effect + EFFECT_ARCHTYPE) == ARCHTYPE_SCRIPT {
        e.mem.u32(effect + EFFECT_ITEM)
    } else {
        0
    }
}

/// `EffectSetting::SetScript`: the script set when it is a script effect.
fn set_effect_script(e: &mut Engine, effect: u32, script: u32) {
    if e.mem.u32(effect + EFFECT_ARCHTYPE) == ARCHTYPE_SCRIPT {
        e.mem.set_u32(effect + EFFECT_ITEM, script);
    }
}

/// The form a script command works on: the argument's base form when it is a reference
/// (`TryGetREFRParent`), else the calling reference's base form; `None` with neither.
fn form_or_caller_base(e: &mut Engine, form: u32, this_obj: u32) -> Option<u32> {
    let form = try_get_refr_parent(e, form);
    if form != 0 {
        return Some(form);
    }
    (this_obj != 0).then(|| base_form(e, this_obj))
}

#[derive(Clone, Copy, PartialEq)]
enum ScriptMode {
    Has,
    Get,
    Remove,
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_Script.cpp: GetScript_Execute.
/// The script of the form (the argument, a reference's base form, else the caller's base form):
/// a scriptable form's, else a script effect's. `Has` answers whether there is one, `Get` its id;
/// `Remove` answers its id and takes it away, and without an argument also takes the calling
/// reference's `ExtraScript` away, so the script stops at once.
fn script_command(e: &mut Engine, w: &[u32], mode: ScriptMode) -> Ret {
    let a = CommandArgs::new(w);
    a.set_result(e, 0.0);
    let read = Read::of(e, &a);
    let given = read.form(0);
    let Some(form) = form_or_caller_base(e, given, a.this_obj) else {
        return handled(true);
    };
    let scriptable = dynamic_cast(e, form, rtti::TES_FORM, rtti::TES_SCRIPTABLE_FORM);
    let mut effect = 0;
    let script = if scriptable == 0 {
        effect = dynamic_cast(e, form, rtti::TES_FORM, rtti::EFFECT_SETTING);
        if effect != 0 {
            effect_script(e, effect)
        } else {
            0
        }
    } else {
        e.mem.u32(scriptable + SCRIPTABLE_SCRIPT)
    };
    match mode {
        ScriptMode::Has => a.set_result(e, if script != 0 { 1.0 } else { 0.0 }),
        ScriptMode::Get => {
            if script != 0 {
                a.set_form_result(e, form_id(e, script));
            }
        }
        ScriptMode::Remove => {
            if script != 0 {
                a.set_form_result(e, form_id(e, script));
            }
            if scriptable != 0 {
                e.mem.set_u32(scriptable + SCRIPTABLE_SCRIPT, 0);
            } else if effect != 0 {
                set_effect_script(e, effect, 0);
            }
            if given == 0 && a.this_obj != 0 {
                e.call(
                    EXTRA_REMOVE_BY_TYPE,
                    &args![a.this_obj + REFR_EXTRA, EXTRA_SCRIPT],
                );
            }
        }
    }
    handled(true)
}

fn is_scripted(e: &mut Engine, w: &[u32]) -> Ret {
    script_command(e, w, ScriptMode::Has)
}

fn get_script(e: &mut Engine, w: &[u32]) -> Ret {
    script_command(e, w, ScriptMode::Get)
}

fn remove_script(e: &mut Engine, w: &[u32]) -> Ret {
    script_command(e, w, ScriptMode::Remove)
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_Script.cpp: Cmd_SetScript_Execute;
// GameExtraData.cpp: ExtraScript::Create.
/// The form's script (the second argument, a reference's base form, else the caller's base form)
/// set to the script given; the old script's id answered. A magic script goes only on a script
/// effect, an effect takes only a magic script. A quest script is
/// set only on a quest, an object script on any scriptable form, with the script's quest set to
/// the form when it is a quest. An object script set without the second argument also gives the
/// calling reference a new `ExtraScript` with its own event list, so it runs at once.
fn set_script(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    a.set_result(e, 0.0);
    let read = Read::of(e, &a);
    let given = read.form(1);
    let Some(form) = form_or_caller_base(e, given, a.this_obj) else {
        return handled(true);
    };
    let scriptable = dynamic_cast(e, form, rtti::TES_FORM, rtti::TES_SCRIPTABLE_FORM);
    let mut effect = 0;
    let old = if scriptable == 0 {
        effect = dynamic_cast(e, form, rtti::TES_FORM, rtti::EFFECT_SETTING);
        if effect == 0 {
            return handled(true);
        }
        effect_script(e, effect)
    } else {
        e.mem.u32(scriptable + SCRIPTABLE_SCRIPT)
    };
    let script = dynamic_cast(e, read.form(0), rtti::TES_FORM, rtti::SCRIPT);
    if script == 0 {
        return handled(true);
    }
    let kind = script_type(e, script);
    if kind == TYPE_MAGIC && effect == 0 {
        return handled(true);
    }
    if kind == TYPE_MAGIC {
        set_effect_script(e, effect, script);
    } else if effect != 0 {
        return handled(true);
    }
    if old != 0 {
        a.set_form_result(e, form_id(e, old));
    }
    let quest_on_quest = kind == TYPE_QUEST_SCRIPT && e.mem.u8(form + FORM_TYPE) == TYPE_QUEST;
    if quest_on_quest || kind == TYPE_OBJECT {
        e.mem.set_u32(scriptable + SCRIPTABLE_SCRIPT, script);
        let quest = dynamic_cast(e, form, rtti::TES_FORM, rtti::TES_QUEST);
        e.mem.set_u32(script + SCRIPT_QUEST, quest);
    }
    if kind == TYPE_OBJECT && given == 0 && a.this_obj != 0 {
        let list = a.this_obj + REFR_EXTRA;
        e.call(EXTRA_REMOVE_BY_TYPE, &args![list, EXTRA_SCRIPT]);
        let extra = extra_script_create(e, form);
        crate::invref::extra_add(e, list, extra);
    }
    handled(true)
}

/// `ExtraScript::Create(baseForm, true)`: a new `ExtraScript` holding the form's script and a new
/// event list for it (none without a script).
fn extra_script_create(e: &mut Engine, form: u32) -> u32 {
    let extra = crate::invref::create_extra(e, EXTRA_SCRIPT, EXTRA_SCRIPT_SIZE, EXTRA_SCRIPT_VTBL);
    if extra != 0 && form != 0 {
        let scriptable = dynamic_cast(e, form, rtti::TES_FORM, rtti::TES_SCRIPTABLE_FORM);
        if scriptable != 0 {
            let script = e.mem.u32(scriptable + SCRIPTABLE_SCRIPT);
            if script != 0 {
                e.mem.set_u32(extra + EXTRA_SCRIPT_SCRIPT, script);
                let events = e.call(CREATE_EVENT_LIST, &args![script]).u32();
                e.mem.set_u32(extra + EXTRA_SCRIPT_EVENT_LIST, events);
            }
        }
    }
    extra
}

// ---- variables by name ----

/// The script and event list a variable command reads: the quest's (the last argument) when
/// given, else the calling reference's base form's script and the reference's event list; `None`
/// when either is missing.
fn target_script(e: &mut Engine, quest: u32, this_obj: u32) -> Option<(u32, u32)> {
    let (script, events) = if quest != 0 {
        let scriptable = dynamic_cast(e, quest, rtti::TES_QUEST, rtti::TES_SCRIPTABLE_FORM);
        (
            e.mem.u32(scriptable + SCRIPTABLE_SCRIPT),
            e.mem.u32(quest + QUEST_EVENT_LIST),
        )
    } else if this_obj != 0 {
        let base = base_form(e, this_obj);
        let scriptable = dynamic_cast(e, base, rtti::TES_FORM, rtti::TES_SCRIPTABLE_FORM);
        if scriptable == 0 {
            return None;
        }
        let script = e.mem.u32(scriptable + SCRIPTABLE_SCRIPT);
        (script, reference_event_list(e, this_obj))
    } else {
        return None;
    };
    (script != 0 && events != 0).then_some((script, events))
}

/// What `GetVariable_Execute` does.
#[derive(Clone, Copy, PartialEq)]
enum VarAction {
    Get,
    GetRef,
    Has,
    GetStr,
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_Script.cpp: GetVariable_Execute.
/// The variable named (without case) of the quest's or the calling reference's script: `Has`
/// whether the script declares it; `Get` its number; `GetRef` the form id in its low word;
/// `GetStr` its string variable's text as a new string (empty when the variable is not there or
/// holds no string, but nothing assigned when there is no script or event list).
fn get_variable_execute(e: &mut Engine, w: &[u32], action: VarAction) -> Ret {
    let a = CommandArgs::new(w);
    a.set_result(e, 0.0);
    let read = Read::of(e, &a);
    if !read.ok {
        return handled(true);
    }
    let name = super::arg_in_buffer(e, &read, 0, 0x100, "GetVariable");
    let Some((script, events)) = target_script(e, read.form(1), a.this_obj) else {
        return handled(true);
    };
    let var = script_variable_by_name(e, script, &name);
    if action == VarAction::Has {
        a.set_result(e, if var.is_some() { 1.0 } else { 0.0 });
        return handled(true);
    }
    if let Some(index) = var {
        let local = event_list_variable(e, events, index);
        if local != 0 {
            match action {
                VarAction::Get => a.set_result(e, e.mem.f64(local + LOCAL_DATA)),
                VarAction::GetRef => {
                    let id = e.mem.u32(local + LOCAL_DATA);
                    a.set_form_result(e, id);
                }
                VarAction::GetStr => {
                    let id = super::string::dtoui3(e.mem.f64(local + LOCAL_DATA));
                    let text = vars::string(e, id).unwrap_or_default();
                    assign_to_string_var(e, &a, &text);
                    return handled(true);
                }
                VarAction::Has => {}
            }
        }
    }
    if action == VarAction::GetStr {
        assign_to_string_var(e, &a, b"");
    }
    handled(true)
}

fn get_variable(e: &mut Engine, w: &[u32]) -> Ret {
    get_variable_execute(e, w, VarAction::Get)
}

fn has_variable(e: &mut Engine, w: &[u32]) -> Ret {
    get_variable_execute(e, w, VarAction::Has)
}

fn get_ref_variable(e: &mut Engine, w: &[u32]) -> Ret {
    get_variable_execute(e, w, VarAction::GetRef)
}

fn get_string_variable(e: &mut Engine, w: &[u32]) -> Ret {
    get_variable_execute(e, w, VarAction::GetStr)
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_Script.cpp:
// Cmd_GetArrayVariable_Execute.
/// `GetVariable` inside an expression; outside one a runtime error and the line stops.
fn get_array_variable(e: &mut Engine, w: &[u32]) -> Ret {
    if !crate::expr::active(e) {
        let a = CommandArgs::new(w);
        crate::functions::runtime_error(
            e,
            a.script,
            "GetArrayVariable must be called within the context of an NVSE expression",
        );
        return handled(false);
    }
    get_variable_execute(e, w, VarAction::Get)
}

/// The local named (without case) in the quest's or the calling reference's script, 0 for none.
fn named_local(e: &mut Engine, read: &Read, this_obj: u32, quest_arg: usize) -> u32 {
    let name = super::arg_in_buffer(e, read, 0, 0x100, "SetVariable");
    let Some((script, events)) = target_script(e, read.form(quest_arg), this_obj) else {
        return 0;
    };
    match script_variable_by_name(e, script, &name) {
        Some(index) => event_list_variable(e, events, index),
        None => 0,
    }
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_Script.cpp:
// Cmd_SetVariable_Execute.
/// The variable named set to the number (a `float` argument, widened).
fn set_variable(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    a.set_result(e, 0.0);
    let read = Read::of(e, &a);
    if !read.ok {
        return handled(true);
    }
    let local = named_local(e, &read, a.this_obj, 2);
    if local != 0 {
        let value = read.float(1, 0.0);
        e.mem.set_f64(local + LOCAL_DATA, f64::from(value));
    }
    handled(true)
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_Script.cpp:
// Cmd_SetRefVariable_Execute.
/// The variable named set to the form's id (its eight bytes: the id, then 0).
fn set_ref_variable(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    a.set_result(e, 0.0);
    let read = Read::of(e, &a);
    if !read.ok {
        return handled(true);
    }
    let local = named_local(e, &read, a.this_obj, 2);
    if local != 0 {
        let form = read.form(1);
        let id = if form != 0 { form_id(e, form) } else { 0 };
        e.mem.set_u32(local + LOCAL_DATA, id);
        e.mem.set_u32(local + LOCAL_DATA + 4, 0);
    }
    handled(true)
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_Script.cpp:
// Cmd_SetStringVariable_Execute.
/// The string the variable named holds set to the text; nothing when it holds no string.
fn set_string_variable(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    a.set_result(e, 0.0);
    let read = Read::of(e, &a);
    if !read.ok {
        return handled(true);
    }
    let local = named_local(e, &read, a.this_obj, 2);
    if local != 0 {
        let id = super::string::dtoui3(e.mem.f64(local + LOCAL_DATA));
        if vars::string(e, id).is_some() {
            let text = super::string::c_str(&read.text(1)).to_vec();
            vars::set_string(e, id, &text);
        }
    }
    handled(true)
}

// ---- whole scripts ----

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_Script.cpp:
// Cmd_CompareScripts_Execute.
/// 1 when both forms are scripts of the same compiled length whose bytes are the same (or the same
/// script).
fn compare_scripts(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    a.set_result(e, 0.0);
    let read = Read::of(e, &a);
    if !read.ok {
        return handled(true);
    }
    let first = dynamic_cast(e, read.form(0), rtti::TES_FORM, rtti::SCRIPT);
    let second = dynamic_cast(e, read.form(1), rtti::TES_FORM, rtti::SCRIPT);
    if first == 0 || second == 0 {
        return handled(true);
    }
    let len = e.mem.u32(first + SCRIPT_DATA_LENGTH);
    if len != e.mem.u32(second + SCRIPT_DATA_LENGTH) {
        return handled(true);
    }
    let same = first == second || {
        let l = e.mem.bytes(e.mem.u32(first + SCRIPT_DATA), len);
        let r = e.mem.bytes(e.mem.u32(second + SCRIPT_DATA), len);
        l == r
    };
    if same {
        a.set_result(e, 1.0);
    }
    handled(true)
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_Script.cpp:
// Cmd_ResetAllVariables_Execute; GameScript.cpp: ScriptEventList::ResetAllVariables.
/// Every variable of the calling reference's script (else the calling script's) set to 0; the
/// number of them.
fn reset_all_variables(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    a.set_result(e, 0.0);
    let list = if a.this_obj != 0 {
        reference_event_list(e, a.this_obj)
    } else {
        a.event_list
    };
    if list != 0 {
        let n = crate::functions::reset_all_variables(e, list);
        a.set_result(e, f64::from(n));
    }
    handled(true)
}

/// `GetScriptArg`: the form as a script when given, else the calling reference's base form's
/// script; 0 for none.
fn script_arg(e: &mut Engine, this_obj: u32, form: u32) -> u32 {
    if form != 0 {
        return dynamic_cast(e, form, rtti::TES_FORM, rtti::SCRIPT);
    }
    if this_obj != 0 {
        let base = base_form(e, this_obj);
        let scriptable = dynamic_cast(e, base, rtti::TES_FORM, rtti::TES_SCRIPTABLE_FORM);
        if scriptable != 0 {
            return e.mem.u32(scriptable + SCRIPTABLE_SCRIPT);
        }
    }
    0
}

/// The script's literal references (`ExplicitRefFinder`: entries whose `varIdx` is 0), in the
/// list's order (empty nodes passed over).
fn explicit_refs(e: &Engine, script: u32) -> Vec<u32> {
    let mut out = Vec::new();
    let mut node = script + SCRIPT_REF_LIST;
    let mut walked = 0;
    while node != 0 && crate::invref::list_step(e, &mut walked) {
        let var = e.mem.u32(node);
        if var != 0 && e.mem.u32(var + REF_VAR_INDEX) == 0 {
            out.push(var);
        }
        node = e.mem.u32(node + 4);
    }
    out
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_Script.cpp:
// Cmd_GetNumExplicitRefs_Execute.
/// The number of literal references in the script (the form, else the caller's); the console
/// prints it, also after a failed read.
fn get_num_explicit_refs(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    a.set_result(e, 0.0);
    let read = Read::of(e, &a);
    if read.ok {
        let script = script_arg(e, a.this_obj, read.form(0));
        if script != 0 {
            let n = explicit_refs(e, script).len() as u32;
            a.set_result(e, f64::from(n));
        }
    }
    if is_console_mode(e) {
        let words = double_words(e.mem.f64(a.result));
        console_print(e, "GetNumExplicitRefs >> %.0f", &words);
    }
    handled(true)
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_Script.cpp:
// Cmd_GetNthExplicitRef_Execute.
/// The first literal reference's form id for any index (xNVSE's loop finds the first again each
/// pass), 0 without literal references or for one without a form; the console names it.
fn get_nth_explicit_ref(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    // `*refResult = 0`: only the low word.
    e.mem.set_u32(a.result, 0);
    let read = Read::of(e, &a);
    if !read.ok {
        return handled(true);
    }
    let script = script_arg(e, a.this_obj, read.form(1));
    if script == 0 {
        return handled(true);
    }
    let index = read.int(0, 0);
    let first = explicit_refs(e, script).first().copied();
    let Some(entry) = first else {
        return handled(true);
    };
    // `while (count <= refIdx) { entry = Find(); count++; }`: with an entry the loop always runs
    // to refIdx + 1 (an index of 0xFFFFFFFF wraps the count and never ends in xNVSE: the loop is
    // capped here at the list walk's limit instead).
    if index == u32::MAX {
        return handled(true);
    }
    let form = e.mem.u32(entry + REF_VAR_FORM);
    if form != 0 {
        let id = form_id(e, form);
        e.mem.set_u32(a.result, id);
        if is_console_mode(e) {
            let name = super::string::full_name(e, form);
            crate::events::with_guest_text(e, &name, |e, at| {
                console_print(e, "GetNthExplicitRef >> %s (%08x)", &[at, id]);
            });
        }
    }
    handled(true)
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_Script.cpp: Cmd_RunScript_Execute.
/// The form's script (a reference's base form, else the caller's base form; a scriptable form's,
/// a script effect's, or the form itself as a script) run once on the calling reference with no
/// event list; what `Script::Execute` answers.
fn run_script(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    a.set_result(e, 0.0);
    let read = Read::of(e, &a);
    if !read.ok {
        return handled(true);
    }
    let Some(form) = form_or_caller_base(e, read.form(0), a.this_obj) else {
        return handled(true);
    };
    let scriptable = dynamic_cast(e, form, rtti::TES_FORM, rtti::TES_SCRIPTABLE_FORM);
    let script = if scriptable == 0 {
        let effect = dynamic_cast(e, form, rtti::TES_FORM, rtti::EFFECT_SETTING);
        if effect != 0 {
            effect_script(e, effect)
        } else {
            dynamic_cast(e, form, rtti::TES_FORM, rtti::SCRIPT)
        }
    } else {
        e.mem.u32(scriptable + SCRIPTABLE_SCRIPT)
    };
    if script != 0 {
        let ran = e.call(SCRIPT_EXECUTE, &args![script, a.this_obj, 0u32, 0u32, 0u32]);
        let ok = ran.u32() & 0xff != 0;
        a.set_result(e, if ok { 1.0 } else { 0.0 });
    }
    handled(true)
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_Script.cpp:
// Cmd_GetCurrentScript_Execute.
/// The calling script's id in the result's low word.
fn get_current_script(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let id = form_id(e, a.script);
    e.mem.set_u32(a.result, id);
    handled(true)
}

#[cfg(test)]
#[path = "script_vars_tests.rs"]
mod tests;
