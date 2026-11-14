//! The variables JIP LN NVSE adds to scripts (`functions_jip/jip_fn_script.h`:
//! `AddScriptVariable`, `HasVariableAdded`, `RemoveScriptVariable`, `RemoveAllAddedVariables`),
//! JIP's `Script::AddVariable` (`internal/jip_core.cpp`) and what the co-save's `JPSV` record does
//! with them (step 3.3, its application): a load makes each saved variable in its owner's script
//! and event list and gives it its value ([`add_variable`], [`set_loaded_value`], called from
//! `cosave::load_script_vars`), and a save writes each variable's value as it is then
//! ([`refresh_saved_values`]). Kept apart from `jip_fn_script.rs` so the co-save's application
//! comes together.
//!
//! JIP's two maps: `s_scriptVariablesBuffer` (owner id, name: the variable and the mod that added
//! it; saved) is the co-save's [`crate::cosave::Saved::script_vars`], its entries' variables in
//! [`State::live`]; `s_addedVariables` (script id, names JIP added to the script this session;
//! never saved, never cleared) is [`State::added`].
//!
//! The game's layouts (Ghidra :8090): `Script` variable list `+0x4C` (`Script::GetVariableName`,
//! `005ABED0`, through `005D43E0`), `VariableInfo` id `+0`, value `+8`, integer flag `+0x10`
//! (`ScriptCompileData::GetVariable` `005AE7A0` reads it), name `BSStringT` `+0x18` (its text read
//! through `00559450`), 0x20 bytes (JIP's `sizeof(VariableInfo)`); `ScriptLocals` variable list
//! pointer `+0xC` (`005ABF60` stores the list `005AC020` builds there), each `ScriptVar` 0x18
//! bytes (`005AC020` allocates 0x18 per variable; `005AC150` copies id `+0`, value `+8` and the
//! integer flag `+0x10` from the `VariableInfo`). The script's last variable id `+0x24`
//! (`info.varCount`, JIP's header; the Xbox PDB's `SCRIPT_HEADER::m_uiLastID` at the header's
//! `+0xC`, the header at `+0x18`).

use super::jip_fn_aux_variable::overriding_mod;
use super::{buffer_arg, heap_alloc, list_prepend, list_step, ref_id, string_set, Handlers};
use crate::commands::forms::ListNode;
use crate::containers::{Name, UnorderedMap};
use crate::cosave::ScriptVar;
use crate::Jip;
use engine_core::prelude::*;
use nvse::commands::Read;
use nvse::{handled, CommandArgs};

#[rustfmt::skip]
pub static COMMANDS: Handlers = &[
    (0x2621, add_script_variable),
    (0x268c, has_variable_added),
    (0x268d, remove_script_variable),
    (0x2690, remove_all_added_variables),
];

/// `Script::info.varCount` (JIP's header): the last variable id given.
const SCRIPT_LAST_VAR_ID: u32 = 0x24;
/// `Script::varList`: a `tList<VariableInfo>`, first node inline.
const SCRIPT_VAR_LIST: u32 = 0x4c;
/// `VariableInfo`: 0x20 bytes, its id `+0`, its name (`BSStringT`, text first) `+0x18`.
pub const VARIABLE_INFO_SIZE: u32 = 0x20;
const VARIABLE_INFO_NAME: u32 = 0x18;
/// `ScriptLocals::m_vars`: a pointer to the `tList<ScriptVar>`.
const EVENT_LIST_VARS: u32 = 0xc;
/// `ScriptVar`: 0x18 bytes, id `+0`, value (a double) `+8`, integer flag `+0x10`.
pub const SCRIPT_VAR_SIZE: u32 = 0x18;
const SCRIPT_VAR_VALUE: u32 = 8;
const SCRIPT_VAR_INTEGER: u32 = 0x10;
/// JIP's buffer of the variable name (`char varName[0x50]`).
const NAME_BUFFER: usize = 0x50;

/// What JIP keeps of the variables it added.
#[derive(Clone, Debug, Default)]
pub struct State {
    /// `s_addedVariables`: by script id, the names JIP added to the script this session.
    pub added: UnorderedMap<u32, UnorderedMap<Name, ()>>,
    /// `ScriptVariableEntry::value`: by owner id and name, the `ScriptVar` each saved entry
    /// (`Saved::script_vars`) holds.
    pub live: UnorderedMap<u32, UnorderedMap<Name, u32>>,
}

fn state(e: &mut Engine) -> &mut State {
    &mut e.ext::<Jip>().script_vars
}

/// Whether the C string at `text` is `name`, without case (`StrCompareCI(text, name) == 0`; a
/// null text is never equal). Read only as far as decides it (the name and its 0), each byte
/// checked: a byte that cannot be read is not equal.
fn text_is(e: &Engine, text: u32, name: &[u8]) -> bool {
    if text == 0 {
        return false;
    }
    let mut read = Vec::with_capacity(name.len() + 1);
    for i in 0..=name.len() as u32 {
        let mut b = [0u8];
        if e.mem.try_read(text.wrapping_add(i), &mut b).is_err() {
            return false;
        }
        read.push(b[0]);
        if b[0] == 0 {
            break;
        }
    }
    crate::utility::str_compare_ci(&read, name).0 == 0
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), nvse/GameScript.cpp: Script::GetVariableByName.
/// The script's `VariableInfo` named `name` (without case), 0 for none: the variable list walked
/// from its first node, empty nodes passed over.
pub fn variable_by_name(e: &Engine, script: u32, name: &[u8]) -> u32 {
    let mut node = script + SCRIPT_VAR_LIST;
    let mut walked = 0usize;
    while node != 0 && list_step(e, &mut walked) {
        let info = e.get(Ptr::<ListNode>::new(node), ListNode::data);
        if info != 0 && text_is(e, e.mem.u32(info + VARIABLE_INFO_NAME), name) {
            return info;
        }
        node = e.get(Ptr::<ListNode>::new(node), ListNode::next);
    }
    0
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), nvse/GameAPI.cpp: ScriptLocals::GetVariable (asm).
/// The event list's `ScriptVar` of id `id`, 0 for none (its variable list walked from the node
/// `m_vars` points to; none when that is null).
pub fn event_variable(e: &Engine, events: u32, id: u32) -> u32 {
    let mut node = e.mem.u32(events + EVENT_LIST_VARS);
    let mut walked = 0usize;
    while node != 0 && list_step(e, &mut walked) {
        let var = e.get(Ptr::<ListNode>::new(node), ListNode::data);
        node = e.get(Ptr::<ListNode>::new(node), ListNode::next);
        if var != 0 && e.mem.u32(var) == id {
            return var;
        }
    }
    0
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/jip_core.cpp: GetVariableAdded.
/// Whether JIP added `name` to the script of id `script_id` this session.
pub fn variable_added(e: &mut Engine, script_id: u32, name: &[u8]) -> bool {
    state(e)
        .added
        .get(&script_id)
        .is_some_and(|names| names.get(&Name(name.to_vec())).is_some())
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/jip_core.cpp: Script::AddVariable.
/// `name` added to the script and its event list for owner `owner_id` by mod `mod_index`; the
/// event list's variable, or `None`. A name the script does not have gets a new `VariableInfo`
/// (0x20 bytes of the game's heap, zeroed; the script's next variable id; the name set; first in
/// the script's list) and is noted as added this session; a name it has is used only when JIP
/// added it this session. The event list gets a `ScriptVar` of that id when it has none (0x18
/// bytes, the value 0, first in its list). Not a temporary name (`*` first): the variable is kept
/// for the co-save under its owner (a map of 4 buckets made for a new owner), with the mod.
pub fn add_variable(
    e: &mut Engine,
    script: u32,
    events: u32,
    owner_id: u32,
    name: &[u8],
    mod_index: u8,
) -> Option<u32> {
    let script_id = ref_id(e, script);
    let mut info = variable_by_name(e, script, name);
    if info == 0 {
        info = heap_alloc(e, VARIABLE_INFO_SIZE);
        for i in (0..VARIABLE_INFO_SIZE).step_by(4) {
            e.mem.set_u32(info + i, 0);
        }
        let id = e.mem.u32(script + SCRIPT_LAST_VAR_ID).wrapping_add(1);
        e.mem.set_u32(script + SCRIPT_LAST_VAR_ID, id);
        e.mem.set_u32(info, id);
        string_set(e, info + VARIABLE_INFO_NAME, name);
        list_prepend(e, Ptr::new(script + SCRIPT_VAR_LIST), info);
        state(e)
            .added
            .emplace_with(script_id, UnorderedMap::default)
            .0
            .emplace(Name(name.to_vec()));
    } else if !variable_added(e, script_id, name) {
        return None;
    }
    let id = e.mem.u32(info);
    let mut var = event_variable(e, events, id);
    if var == 0 {
        // nv-rs's own: an event list with no variable list (the game's always has one, `005AC020`)
        // would be written through a null pointer; the variable is not made.
        let list = e.mem.u32(events + EVENT_LIST_VARS);
        if list == 0 {
            return None;
        }
        var = heap_alloc(e, SCRIPT_VAR_SIZE);
        e.mem.set_u32(var, id);
        e.mem.set_f64(var + SCRIPT_VAR_VALUE, 0.0);
        // nv-rs's own: JIP leaves the integer flag as the heap gave it; the game's own variables
        // copy it from their `VariableInfo` (`005AC150`), JIP's zeroed one: a float variable.
        e.mem.set_u8(var + SCRIPT_VAR_INTEGER, 0);
        list_prepend(e, Ptr::new(list), var);
    }
    if name.first() != Some(&b'*') {
        let value = e.mem.u64(var + SCRIPT_VAR_VALUE);
        let jip = e.ext::<Jip>();
        // `s_scriptVariablesBuffer()[ownerID][varName].Set(var, modIdx)`.
        let entry = jip
            .saved
            .script_vars
            .emplace_with(owner_id, || UnorderedMap::new(4))
            .0
            .emplace_with(Name(name.to_vec()), ScriptVar::default)
            .0;
        *entry = ScriptVar { mod_index, value };
        *jip.script_vars
            .live
            .emplace_with(owner_id, || UnorderedMap::new(4))
            .0
            .emplace(Name(name.to_vec())) = var;
    }
    Some(var)
}

/// The load's `var->data = varData->num`: the value read (a form's id already resolved) into the
/// variable [`add_variable`] gave, and into its saved entry.
pub fn set_loaded_value(e: &mut Engine, owner_id: u32, name: &[u8], var: u32, value: u64) {
    e.mem.set_u64(var + SCRIPT_VAR_VALUE, value);
    if let Some(entry) = e
        .ext::<Jip>()
        .saved
        .script_vars
        .get_mut(&owner_id)
        .and_then(|vars| vars.get_mut(&Name(name.to_vec())))
    {
        entry.value = value;
    }
}

/// Before a save: each saved entry's value read again from its variable (JIP writes
/// `value->data` at the save). A variable whose memory cannot be read keeps the value last known
/// (nv-rs's own: JIP reads it whatever it is).
pub fn refresh_saved_values(e: &mut Engine) {
    let jip = e.ext::<Jip>();
    let mut vars = Vec::new();
    for (owner, names) in jip.saved.script_vars.iter() {
        for (name, _) in names.iter() {
            let var = jip
                .script_vars
                .live
                .get(owner)
                .and_then(|live| live.get(name))
                .copied();
            if let Some(var) = var {
                vars.push((*owner, name.clone(), var));
            }
        }
    }
    for (owner, name, var) in vars {
        let mut bytes = [0u8; 8];
        if e.mem
            .try_read(var.wrapping_add(SCRIPT_VAR_VALUE), &mut bytes)
            .is_err()
        {
            continue;
        }
        if let Some(entry) = e
            .ext::<Jip>()
            .saved
            .script_vars
            .get_mut(&owner)
            .and_then(|v| v.get_mut(&name))
        {
            entry.value = u64::from_le_bytes(bytes);
        }
    }
}

/// The owner a command names: the form argument, else the calling reference (none: `None`).
fn owner_of(form: u32, this_obj: u32) -> Option<u32> {
    match (form, this_obj) {
        (0, 0) => None,
        (0, this) => Some(this),
        (form, _) => Some(form),
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_script.h:
// ScriptVariableAction_Execute.
/// The three commands' body (`actionType` 0 has, 1 add, 2 remove): a name (JIP's 0x50-byte
/// buffer; empty: nothing) and an optional form, else the calling reference. Remove: the saved
/// entry of the owner's id and the name taken out (an emptied owner too), 1 when it was there; the
/// script's variable itself stays. Add and has need the owner's script and event list: add makes
/// the variable for the calling script's mod (1 when made); has answers 1 when JIP added the name
/// to that script this session, the owner has a saved entry of it, and the script's variable of
/// that name is in the event list.
fn script_variable_action(e: &mut Engine, w: &[u32], action: u8) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if !read.ok {
        return handled(true);
    }
    let name = buffer_arg(e, &read, 0, NAME_BUFFER, "ScriptVariableAction");
    if name.is_empty() {
        return handled(true);
    }
    let Some(form) = owner_of(read.form(1), a.this_obj) else {
        return handled(true);
    };
    let owner_id = ref_id(e, form);
    let key = Name(name.clone());
    if action == 2 {
        let jip = e.ext::<Jip>();
        let Some(vars) = jip.saved.script_vars.get_mut(&owner_id) else {
            return handled(true);
        };
        if vars.erase(&key) {
            if vars.is_empty() {
                jip.saved.script_vars.erase(&owner_id);
            }
            if let Some(live) = jip.script_vars.live.get_mut(&owner_id) {
                live.erase(&key);
            }
            a.set_result(e, 1.0);
        }
        return handled(true);
    }
    let Some((script, events)) = crate::cosave::script_and_event_list(e, form) else {
        return handled(true);
    };
    if action == 1 {
        let mod_index = overriding_mod(e, a.script) as u8;
        if add_variable(e, script, events, owner_id, &name, mod_index).is_some() {
            a.set_result(e, 1.0);
        }
    } else {
        let script_id = ref_id(e, script);
        if !variable_added(e, script_id, &name) {
            return handled(true);
        }
        let saved = e
            .ext::<Jip>()
            .saved
            .script_vars
            .get(&owner_id)
            .is_some_and(|vars| vars.get(&key).is_some());
        if !saved {
            return handled(true);
        }
        let info = variable_by_name(e, script, &name);
        if info != 0 && event_variable(e, events, e.mem.u32(info)) != 0 {
            a.set_result(e, 1.0);
        }
    }
    handled(true)
}

/// `HasVariableAdded name [form]`.
fn has_variable_added(e: &mut Engine, w: &[u32]) -> Ret {
    script_variable_action(e, w, 0)
}

/// `AddScriptVariable name [form]`.
fn add_script_variable(e: &mut Engine, w: &[u32]) -> Ret {
    script_variable_action(e, w, 1)
}

/// `RemoveScriptVariable name [form]`.
fn remove_script_variable(e: &mut Engine, w: &[u32]) -> Ret {
    script_variable_action(e, w, 2)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_script.h:
// Cmd_RemoveAllAddedVariables_Execute.
/// `RemoveAllAddedVariables [form]`: the owner's (the form, else the calling reference) saved
/// entries added by the calling script's mod taken out (an emptied owner too); the scripts'
/// variables stay. No result.
fn remove_all_added_variables(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if !read.ok {
        return handled(true);
    }
    let Some(form) = owner_of(read.form(0), a.this_obj) else {
        return handled(true);
    };
    let owner_id = ref_id(e, form);
    let mod_index = overriding_mod(e, a.script);
    let jip = e.ext::<Jip>();
    let Some(vars) = jip.saved.script_vars.get_mut(&owner_id) else {
        return handled(true);
    };
    let mut gone = Vec::new();
    vars.retain(|name, v| {
        let keep = u32::from(v.mod_index) != mod_index;
        if !keep {
            gone.push(name.clone());
        }
        keep
    });
    if vars.is_empty() {
        jip.saved.script_vars.erase(&owner_id);
    }
    if let Some(live) = jip.script_vars.live.get_mut(&owner_id) {
        for name in &gone {
            live.erase(name);
        }
    }
    handled(true)
}

#[cfg(test)]
#[path = "jip_fn_script_vars_tests.rs"]
mod tests;
