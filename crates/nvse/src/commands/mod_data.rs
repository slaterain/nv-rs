//! xNVSE's mod-local data and the small commands of `Commands_Scripting.cpp` and
//! `Commands_Script.cpp` around it (xNVSE): the user's clock as a string map (`GetUserTime`),
//! data a mod keeps in memory across loaded saves (`SetModLocalData` and its kin), the calling
//! reference (`GetSelfAlt`), a command's opcode by name, and a value matched against a list
//! (`MatchesAnyOf`).
//!
//! Read from xNVSE's source (xNVSE 6.4.9, `0ccd23a`). Kept as xNVSE has them: the mod-local data's
//! map finds a key by its case-insensitive hash alone (`UnorderedMap<char*>` keeps the hash and
//! compares only it), so two keys that differ in case are one key, and the key text kept is the
//! one the entry was made with. One difference: `MatchesAnyOf` with an array that does not exist
//! answers "no match" where xNVSE reads through a null array and stops the game.

use super::array::{basic_elem, compare_arrays};
use super::string::{assign_to_string_var, with_eval};
use crate::expr::{expect_return_type, Ty};
use crate::table::{Command, Param, Returns};
use crate::vars::{self, Data, Key, Value, KEY_STRING};
use crate::{handled, CommandArgs};
use engine_core::prelude::*;
use std::collections::HashMap;

const fn p(name: &'static str, kind: u32, optional: bool) -> Param {
    Param {
        name,
        kind,
        optional,
    }
}
/// `kParamType_String` (xNVSE `CommandTable.h`); `kNVSEParamType_String`, the data's
/// `String | Number | Form`, and `BasicType` (xNVSE `ScriptUtils.h`).
const STRING: u32 = 0x00;
const NVSE_STRING: u32 = 0x8004;
const NVSE_DATA: u32 = 0x800d;
const BASIC_TYPE: u32 = 0x804d;
const ONE_STRING: &[Param] = &[p("string", STRING, false)];
const ONE_NVSE_STRING: &[Param] = &[p("string", NVSE_STRING, false)];
const SET_MOD_LOCAL_DATA: &[Param] = &[p("key", NVSE_STRING, false), p("data", NVSE_DATA, false)];
/// `kNVSEParams_MatchesAnyOf`: the value, one value to match, then nineteen optional ones.
const MATCHES_ANY_OF: &[Param] = &[
    p("valueToMatch", BASIC_TYPE, false),
    p("value", BASIC_TYPE, false),
    p("value", BASIC_TYPE, true),
    p("value", BASIC_TYPE, true),
    p("value", BASIC_TYPE, true),
    p("value", BASIC_TYPE, true),
    p("value", BASIC_TYPE, true),
    p("value", BASIC_TYPE, true),
    p("value", BASIC_TYPE, true),
    p("value", BASIC_TYPE, true),
    p("value", BASIC_TYPE, true),
    p("value", BASIC_TYPE, true),
    p("value", BASIC_TYPE, true),
    p("value", BASIC_TYPE, true),
    p("value", BASIC_TYPE, true),
    p("value", BASIC_TYPE, true),
    p("value", BASIC_TYPE, true),
    p("value", BASIC_TYPE, true),
    p("value", BASIC_TYPE, true),
    p("value", BASIC_TYPE, true),
    p("value", BASIC_TYPE, true),
];

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
const SELF_HELP: &str =
    "Unlike GetSelf, will return ThisObj even if it isn't Persistent and is clutter.";

#[rustfmt::skip]
pub static COMMANDS: &[Command] = &[
    cmd(0x1547, "GetUserTime", "", "returns the users local time and date as a stringmap", &[], get_user_time, Returns::Array),
    cmd(0x1548, "GetModLocalData", "", "gets a key-value pair for the mod", ONE_NVSE_STRING, get_mod_local_data, Returns::Ambiguous),
    cmd(0x1549, "SetModLocalData", "", "sets a key-value pair for the mod", SET_MOD_LOCAL_DATA, set_mod_local_data, RD),
    cmd(0x154a, "ModLocalDataExists", "", "returns true if mod local data exists for the specified key", ONE_NVSE_STRING, mod_local_data_exists, RD),
    cmd(0x154b, "RemoveModLocalData", "", "removes the specified entry from the mod's local data", ONE_NVSE_STRING, remove_mod_local_data, RD),
    cmd(0x154c, "GetAllModLocalData", "", "returns a StringMap containing all local data for the calling mod", &[], get_all_mod_local_data, Returns::Array),
    cmd(0x162c, "GetCommandOpcode", "", "gets opcode for command name", ONE_STRING, get_command_opcode, RD),
    cmd(0x164a, "GetSelfAlt_OLD", "ThisAlt_OLD", SELF_HELP, &[], get_self_alt, Returns::Form),
    cmd(0x166f, "MatchesAnyOf", "", "Returns true/false if the first value matches any of the other values.", MATCHES_ANY_OF, matches_any_of, RD),
    cmd(0x1669, "GetSelfAlt", "ThisAlt", SELF_HELP, &[], get_self_alt, Returns::Form),
];

/// `GetLocalTime` (the KERNEL32 import slot, `stdcall`: a `SYSTEMTIME` of eight words).
const GET_LOCAL_TIME: u32 = 0x00fd_f0d8;

/// The mods' local data (`ModLocalDataManager`): per mod index, the entries by the key's
/// case-insensitive hash, each with the key text it was made with and its value (a number, a
/// form id or a string; never an array).
#[derive(Default)]
pub struct State {
    data: HashMap<u8, HashMap<u32, (Vec<u8>, Value)>>,
}

fn state(e: &mut Engine) -> &mut State {
    &mut e.ext::<crate::Nvse>().mod_data
}

fn script_mod(e: &Engine, script: u32) -> u8 {
    vars::script_mod_index(e, script)
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_Scripting.cpp:
// Cmd_GetUserTime_Execute.
/// A new string map (the calling script's mod) of the local clock: `Year`, `Month`, `DayOfWeek`
/// (Sunday 1), `Day`, `Hour`, `Minute`, `Second`, `Millisecond`.
fn get_user_time(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let owner = script_mod(e, a.script);
    let id = vars::create_array(e, KEY_STRING, false, owner);
    a.set_result(e, f64::from(id));
    let words: Vec<u16> = e.with_stack(16, |e, at| {
        let at = at.addr();
        e.call(GET_LOCAL_TIME, &args![at]);
        (0..8).map(|i| e.mem.u16(at + 2 * i)).collect()
    });
    let fields: [(&[u8], u16); 8] = [
        (b"Year", words[0]),
        (b"Month", words[1]),
        (b"DayOfWeek", words[2] + 1),
        (b"Day", words[3]),
        (b"Hour", words[4]),
        (b"Minute", words[5]),
        (b"Second", words[6]),
        (b"Millisecond", words[7]),
    ];
    for (key, v) in fields {
        let key = Key::Str(key.to_vec());
        vars::set_element(e, id, &key, Value::Number(f64::from(v)));
    }
    handled(true)
}

/// The value an element read as `BasicTokenToElem` makes, `None` for an array or nothing.
fn stored(data: Option<Data>) -> Option<Value> {
    match data? {
        Data::Number(v) => Some(Value::Number(v)),
        Data::Str(s) => Some(Value::Str(s)),
        Data::Form(f) => Some(Value::Form(f)),
        _ => None,
    }
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_Scripting.cpp:
// Cmd_SetModLocalData_Execute, ModLocalDataManager::Set.
/// The value (a number, a form or a string) kept under the key for the calling script's mod;
/// 1 when kept, 0 for an empty key, an array, or not two arguments.
fn set_mod_local_data(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    a.set_result(e, 0.0);
    let owner = script_mod(e, a.script);
    with_eval(e, &a, |e, ev, toks| {
        let Some(toks) = toks.filter(|t| t.len() == 2) else {
            return;
        };
        if !ev.can_convert(e, &toks[0], Ty::String) {
            return;
        }
        let Some(value) = stored(basic_elem(e, ev, &toks[1])) else {
            return;
        };
        let key = ev.get_string(e, &toks[0]);
        if key.is_empty() {
            return;
        }
        let hash = crate::events::hash_name(&key);
        let entries = state(e).data.entry(owner).or_default();
        match entries.get_mut(&hash) {
            Some(entry) => entry.1 = value,
            None => {
                entries.insert(hash, (key, value));
            }
        }
        a.set_result(e, 1.0);
    });
    handled(true)
}

/// The one string argument of the mod-local data readers, when there is exactly one and it reads
/// as a string.
fn key_argument(e: &mut Engine, a: &CommandArgs) -> Option<Vec<u8>> {
    let mut out = None;
    with_eval(e, a, |e, ev, toks| {
        if let Some(toks) = toks.filter(|t| t.len() == 1) {
            if ev.can_convert(e, &toks[0], Ty::String) {
                out = Some(ev.get_string(e, &toks[0]));
            }
        }
    });
    out
}

fn lookup(e: &mut Engine, owner: u8, key: &[u8]) -> Option<Value> {
    let hash = crate::events::hash_name(key);
    state(e)
        .data
        .get(&owner)
        .and_then(|m| m.get(&hash))
        .map(|(_, v)| v.clone())
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_Scripting.cpp:
// Cmd_GetModLocalData_Execute.
/// The value kept under the key: a number as it is, a form's id in the low word, a string as a
/// new string variable; the expected return type follows. 0 for no value.
fn get_mod_local_data(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let owner = script_mod(e, a.script);
    expect_return_type(e, Returns::Default);
    a.set_result(e, 0.0);
    let Some(key) = key_argument(e, &a) else {
        return handled(true);
    };
    match lookup(e, owner, &key) {
        Some(Value::Number(v)) => a.set_result(e, v),
        Some(Value::Form(id)) => {
            a.set_form_result(e, id);
            expect_return_type(e, Returns::Form);
        }
        Some(Value::Str(s)) => {
            assign_to_string_var(e, &a, &s);
            expect_return_type(e, Returns::String);
        }
        _ => {}
    }
    handled(true)
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_Scripting.cpp:
// Cmd_ModLocalDataExists_Execute.
/// 1 when a value is kept under the key.
fn mod_local_data_exists(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let owner = script_mod(e, a.script);
    let found = match key_argument(e, &a) {
        Some(key) => lookup(e, owner, &key).is_some(),
        None => false,
    };
    a.set_result(e, if found { 1.0 } else { 0.0 });
    handled(true)
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_Scripting.cpp:
// Cmd_RemoveModLocalData_Execute, ModLocalDataManager::Remove.
/// The value under the key taken away; 1 when there was one.
fn remove_mod_local_data(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let owner = script_mod(e, a.script);
    let removed = match key_argument(e, &a) {
        Some(key) => {
            let hash = crate::events::hash_name(&key);
            state(e)
                .data
                .get_mut(&owner)
                .and_then(|m| m.remove(&hash))
                .is_some()
        }
        None => false,
    };
    a.set_result(e, if removed { 1.0 } else { 0.0 });
    handled(true)
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_Scripting.cpp:
// Cmd_GetAllModLocalData_Execute, ModLocalDataManager::GetAllAsNVSEArray.
/// A new string map (owned by the calling script's mod) of every value the mod keeps, by the
/// keys' texts.
fn get_all_mod_local_data(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let owner = script_mod(e, a.script);
    let id = vars::create_array(e, KEY_STRING, false, owner);
    let entries: Vec<(Vec<u8>, Value)> = state(e)
        .data
        .get(&owner)
        .map(|m| m.values().cloned().collect())
        .unwrap_or_default();
    for (key, value) in entries {
        vars::set_element(e, id, &Key::Str(key), value);
    }
    a.set_result(e, f64::from(id));
    handled(true)
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_Script.cpp:
// Cmd_GetCommandOpcode_Execute.
/// The opcode of the command named (its name or short name, without case), 0 for none.
fn get_command_opcode(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    a.set_result(e, 0.0);
    let read = super::Read::of(e, &a);
    if !read.ok {
        return handled(true);
    }
    let name = super::arg_in_buffer(e, &read, 0, 0x400, "GetCommandOpcode");
    let opcode = crate::table::opcode_by_name(e, &name).unwrap_or(0);
    a.set_result(e, f64::from(opcode));
    handled(true)
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_Scripting.cpp:
// Cmd_GetSelfAlt_Execute (and Cmd_GetSelfAlt_OLD_Execute, which calls it).
/// The calling reference's id in the result's low word (0 without one), even for a reference
/// that is not persistent.
fn get_self_alt(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let id = if a.this_obj != 0 {
        e.mem.u32(a.this_obj + 0xc)
    } else {
        0
    };
    e.mem.set_u32(a.result, id);
    handled(true)
}

/// `FloatEqual` (xNVSE `common/ITypes.h`): both as `float`, closer than `0.0001f`.
fn float_equal(a: f64, b: f64) -> bool {
    (a as f32 - b as f32).abs() < 0.0001
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_Script.cpp:
// Cmd_MatchesAnyOf_Execute; ScriptTokens.cpp: GetTokenTypeAsVariableType.
/// 1 when the first value matches any of the others, compared as the first value's kind: numbers
/// as floats within 0.0001, strings without case, forms by id, arrays by their keys and elements
/// (not deep); 0 otherwise, and 0 at once for a first value of no kind.
fn matches_any_of(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    a.set_result(e, 0.0);
    let mut found = false;
    with_eval(e, &a, |e, ev, toks| {
        let Some(toks) = toks else {
            return;
        };
        let Some(first) = toks.first() else {
            return;
        };
        let kind = [Ty::Number, Ty::String, Ty::Form, Ty::Array]
            .into_iter()
            .find(|&t| ev.can_convert(e, first, t));
        let Some(kind) = kind else {
            return;
        };
        let upper = vars::upper_table(e);
        for other in &toks[1..] {
            let same = match kind {
                Ty::Number => float_equal(ev.get_number(e, first), ev.get_number(e, other)),
                Ty::String => {
                    let (l, r) = (ev.get_string(e, first), ev.get_string(e, other));
                    vars::str_compare(&upper, &l, &r) == std::cmp::Ordering::Equal
                }
                Ty::Form => ev.get_form_id(e, first) == ev.get_form_id(e, other),
                _ => {
                    let (l, r) = (ev.get_array_id(e, first), ev.get_array_id(e, other));
                    vars::array_exists(e, l)
                        && vars::array_exists(e, r)
                        && compare_arrays(e, l, r, false)
                }
            };
            if same {
                found = true;
                return;
            }
        }
    });
    if found {
        a.set_result(e, 1.0);
    }
    handled(true)
}

#[cfg(test)]
#[path = "mod_data_tests.rs"]
mod tests;
