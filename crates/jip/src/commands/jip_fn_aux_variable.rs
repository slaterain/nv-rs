//! `functions_jip/jip_fn_aux_variable.h` (JIP LN NVSE): auxiliary variables, named arrays of
//! numbers, forms and texts kept per mod and per owner (a form or a reference), all 14 of its
//! commands (`AuxVarGetFltCond` is a condition only).
//!
//! The saved variables are JIP's `s_auxVariables[0]`, the co-save's own map
//! ([`crate::cosave::Saved::aux_vars`], read and written there as JIP's `JPAV` record, byte for
//! byte); a command changing them sets JIP's changed flag for them (`kChangedFlag_AuxVars`), so a
//! load of the same save reads them again. A name beginning `*` is temporary
//! (`s_auxVariables[1]`): never saved, kept for the session ([`Temp`]). A name whose first
//! character after an optional `*` is `_` is shared by every mod (mod index 0xFF); any other
//! belongs to the calling script's mod (`GetOverridingModIdx`: the last file of the script form's
//! files). The owner is the form argument (a reference's base form, as JIP's `GetSubjectID`), or
//! else the calling reference. Maps made by a command get JIP's bucket counts (a mod's owners 8,
//! an owner's variables 4), since the save walks them in their order.
//!
//! JIP clears a deleted temporary reference's variables in its reference-destruction hook
//! (`DestroyRefrHook`, `ClearRefAuxVars`): a hook site of the core track.
//!
//! Where JIP would read through a null script (none in a script line) the port takes mod 0xFF.

use super::arrays::{create_array, create_string_map, set_element, Element};
use super::{assign_string, buffer_arg, cvttsd2si, Handlers};
use crate::containers::{Name, UnorderedMap};
use crate::cosave::{changed, lookup_form, Value};
use crate::Jip;
use engine_core::prelude::*;
use nvse::commands::Read;
use nvse::vars::{self, Container, Data, Key};
use nvse::{handled, CommandArgs, EvalArgs};

#[rustfmt::skip]
pub static COMMANDS: Handlers = &[
    (0x2691, aux_get_size),
    (0x2692, aux_get_type),
    (0x2693, aux_get_float),
    (0x2694, aux_set_float),
    (0x2695, aux_get_ref),
    (0x2696, aux_set_ref),
    (0x2697, aux_get_string),
    (0x2698, aux_set_string),
    (0x2699, aux_get_all),
    (0x269a, aux_erase),
    (0x269b, aux_erase_all),
    (0x26bd, aux_get_as_array),
    (0x26be, aux_set_from_array),
];

/// The condition-only command: its eval.
pub static CONDITIONS_ONLY: super::ConditionsOnly = &[(0x27b1, aux_get_flt_cond_eval)];

/// `char varName[0x50]`.
pub const NAME_BUFFER: usize = 0x50;
/// JIP's string argument buffer (`GetStrArgBuffer`, `STR_BUFFER_SIZE`).
pub const STR_BUFFER_SIZE: usize = 0x4_0000;
/// The longest text a value keeps (`AuxVariableValue::operator=(const char*)`).
pub const TEXT_CAP: usize = 0x1fff;
/// `g_BGSSaveLoadGame` and `BGSSaveLoadGame::AddChange` (`TESForm::MarkModified`, `thiscall`:
/// the form, the change flags, 0).
const SAVE_LOAD_GAME: u32 = 0x011d_df38;
const ADD_CHANGE: u32 = 0x0084_a690;
/// `ADDR_ReturnTrue` in a form's vtable at +0xF0: a reference (`IS_REFERENCE`).
const RETURN_TRUE: u32 = 0x008d_0360;
const IS_REFERENCE: u32 = 0xf0;
/// JIP's map sizes: a mod's owners (`AuxVarOwnersMap`, the default 8), an owner's variables
/// (`AuxVarVarsMap`, 4).
const OWNERS_BUCKETS: u32 = 8;
const VARS_BUCKETS: u32 = 4;

/// An owner's variables: each name's values.
pub type Vars = UnorderedMap<Name, Vec<Value>>;
/// JIP's `AuxVarModsMap`: by mod, by owner id.
pub type AuxMods = UnorderedMap<u32, UnorderedMap<u32, Vars>>;
/// JIP's `RefMapModsMap`: by mod, by name, by reference id.
pub type RefMods = UnorderedMap<u32, UnorderedMap<Name, UnorderedMap<u32, Value>>>;

/// The temporary data JIP keeps for the session (`s_auxVariables[1]`, `s_refMapArrays[1]`,
/// both made with 8 buckets) and its ref map walk (`s_refMapIterator`).
#[derive(Default)]
pub struct Temp {
    pub aux_vars: AuxMods,
    pub ref_maps: RefMods,
    pub ref_map_walk: Option<super::jip_fn_ref_map::Walk>,
}

/// The aux variables: the saved ones or the temporary ones.
pub fn aux_maps(e: &mut Engine, temp: bool) -> &mut AuxMods {
    if temp {
        &mut e.ext::<Temp>().aux_vars
    } else {
        &mut e.ext::<Jip>().saved.aux_vars
    }
}

/// Sets JIP's changed flag `flag` (`s_dataChangedFlags |= flag`).
pub fn mark_changed(e: &mut Engine, flag: u8) {
    e.ext::<Jip>().saved.changed |= flag;
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), nvse/GameForms.h: TESForm::MarkModified.
/// `thisObj->MarkModified(0)` when there is a calling reference (`MarkVarModified`).
pub fn mark_modified(e: &mut Engine, r: u32) {
    if r != 0 {
        let sl = e.mem.u32(SAVE_LOAD_GAME);
        e.call(ADD_CHANGE, &args![sl, r, 0u32, 0u32]);
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/jip_core.cpp: GetSubjectID.
/// The form argument's id (a reference's base form's), else the calling reference's; 0 for
/// neither.
pub fn subject_id(e: &Engine, form: u32, this: u32) -> u32 {
    if form != 0 {
        let vtbl = e.mem.u32(form);
        let f = if e.mem.u32(vtbl + IS_REFERENCE) == RETURN_TRUE {
            e.mem.u32(form + 0x20)
        } else {
            form
        };
        return e.mem.u32(f + 0xc);
    }
    if this != 0 {
        return e.mem.u32(this + 0xc);
    }
    0
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), nvse/GameForms.cpp: TESForm::GetOverridingModIdx.
/// The mod index of the last file in the form's list of files (`+0x10`, the file's index at
/// `+0x40C`), 0xFF for none.
pub fn overriding_mod(e: &Engine, form: u32) -> u32 {
    if form == 0 {
        return 0xff;
    }
    let mut node = form + 0x10;
    loop {
        let next = e.mem.u32(node + 4);
        if next == 0 {
            break;
        }
        node = next;
    }
    let file = e.mem.u32(node);
    if file == 0 {
        0xff
    } else {
        u32::from(e.mem.u8(file + 0x40c))
    }
}

/// `pVarName[isTemp] == '_' ? 0xFF : scriptObj->GetOverridingModIdx()`, and whether the name is
/// temporary (`*`).
pub fn name_scope(e: &Engine, script: u32, name: &[u8]) -> (bool, u32) {
    let temp = name.first() == Some(&b'*');
    let shared = name.get(usize::from(temp)) == Some(&b'_');
    (
        temp,
        if shared {
            0xff
        } else {
            overriding_mod(e, script)
        },
    )
}

/// `type & 1` temporary, `type > 1` shared (0xFF), else the script's mod.
pub fn type_scope(e: &Engine, script: u32, kind: u32) -> (bool, u32) {
    (
        kind & 1 != 0,
        if kind > 1 {
            0xff
        } else {
            overriding_mod(e, script)
        },
    )
}

// ---- values (`AuxVariableValue`) ----

/// The text of a value (`GetStr`), up to its first 0; none for a number or a form.
pub fn value_text(v: &Value) -> Option<Vec<u8>> {
    match v {
        Value::Text { bytes, .. } => {
            let end = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
            Some(bytes[..end].to_vec())
        }
        _ => None,
    }
}

/// `GetFlt`: a number's value, else 0.
pub fn value_float(v: &Value) -> f64 {
    match v {
        Value::Number(n) => *n,
        _ => 0.0,
    }
}

/// `GetRef`: a form's id, else 0.
pub fn value_ref(v: &Value) -> u32 {
    match v {
        Value::Form(id) => *id,
        _ => 0,
    }
}

/// `operator=(const char*)`: a text of type 4, cut at 0x1FFF bytes.
pub fn text_value(text: &[u8]) -> Value {
    let end = text.iter().position(|&b| b == 0).unwrap_or(text.len());
    Value::Text {
        kind: 4,
        bytes: text[..end.min(TEXT_CAP)].to_vec(),
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/jip_core.h: AuxVariableValue::GetAsElement.
/// The value as an array element: a form (looked up by its id: 0 when gone), a text of type 4,
/// else a number. A text of another type (only from a co-save) is read as a number by JIP (its
/// string pointer's bits): the port gives 0 there.
pub fn value_element(e: &Engine, v: &Value) -> Element {
    match v {
        Value::Form(id) => Element::Form(lookup_form(e, *id)),
        Value::Text { kind: 4, .. } => Element::Str(value_text(v).unwrap_or_default()),
        Value::Number(n) => Element::Number(*n),
        Value::Text { .. } => Element::Number(0.0),
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/jip_core.h:
// AuxVariableValue::operator=(const NVSEArrayElement&).
/// An xNVSE array element as a value: a form element (looked up by its id: none when gone) its
/// id, a string its text, a number itself. An array element is read as a number by JIP (the
/// element's 8 bytes, the array's id in the low word): the port takes the high word as 0.
pub fn element_value(e: &Engine, d: &Data) -> Value {
    match d {
        Data::Form(id) => Value::Form(if lookup_form(e, *id) != 0 { *id } else { 0 }),
        Data::Str(s) => text_value(s),
        Data::Number(n) => Value::Number(*n),
        Data::Array(id) => Value::Number(f64::from_bits(u64::from(*id))),
        Data::Invalid => Value::Number(0.0),
    }
}

/// The values of xNVSE array `id` in its order (`GetElements`); `None` for no array.
pub fn array_values(e: &Engine, id: u32) -> Option<Vec<Data>> {
    let a = vars::array(e, id)?;
    Some(match &a.elements {
        Container::Array(v) => v.iter().map(|el| el.data.clone()).collect(),
        Container::NumMap(v) => v.iter().map(|(_, el)| el.data.clone()).collect(),
        Container::StrMap(v) => v.iter().map(|(_, el)| el.data.clone()).collect(),
    })
}

// ---- `AuxVarInfo` ----

/// A variable as JIP's `AuxVarInfo` names it.
struct Info {
    owner: u32,
    mod_index: u32,
    name: Vec<u8>,
    temp: bool,
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/jip_core.h: AuxVarInfo (by name).
/// A non-empty name with an owner; `None` otherwise.
fn info_named(e: &Engine, a: &CommandArgs, form: u32, name: Vec<u8>) -> Option<Info> {
    if name.is_empty() {
        return None;
    }
    let owner = subject_id(e, form, a.this_obj);
    if owner == 0 {
        return None;
    }
    let (temp, mod_index) = name_scope(e, a.script, &name);
    Some(Info {
        owner,
        mod_index,
        name,
        temp,
    })
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/jip_core.h: AuxVarInfo (by type).
fn info_typed(e: &Engine, a: &CommandArgs, form: u32, kind: u32) -> Option<Info> {
    let owner = subject_id(e, form, a.this_obj);
    if owner == 0 {
        return None;
    }
    let (temp, mod_index) = type_scope(e, a.script, kind);
    Some(Info {
        owner,
        mod_index,
        name: Vec::new(),
        temp,
    })
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_aux_variable.h:
// AuxVarInfo::GetArray.
/// The variable's values, made (with its mod's and owner's maps) when `add`.
fn values_of<'a>(maps: &'a mut AuxMods, info: &Info, add: bool) -> Option<&'a mut Vec<Value>> {
    if add {
        let owners = maps
            .emplace_with(info.mod_index, || UnorderedMap::new(OWNERS_BUCKETS))
            .0;
        let vars = owners
            .emplace_with(info.owner, || UnorderedMap::new(VARS_BUCKETS))
            .0;
        return Some(vars.emplace_with(Name(info.name.clone()), Vec::new).0);
    }
    maps.get_mut(&info.mod_index)?
        .get_mut(&info.owner)?
        .get_mut(&Name(info.name.clone()))
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_aux_variable.h:
// AuxVarInfo::GetValue.
/// Value `idx` of the variable (a negative index the last); with `add`, a new value at the end
/// for a negative index or the size (the variable made for an index up to 0).
fn value_at<'a>(maps: &'a mut AuxMods, info: &Info, idx: i32, add: bool) -> Option<&'a mut Value> {
    let vals = values_of(maps, info, add && idx <= 0)?;
    let size = vals.len() as i32;
    if add && (idx < 0 || idx == size) {
        vals.push(Value::Number(0.0));
        return vals.last_mut();
    }
    if size != 0 && idx < size {
        let at = if idx < 0 { size - 1 } else { idx };
        return vals.get_mut(at as usize);
    }
    None
}

/// The name argument (JIP's 0x50-byte buffer).
pub fn buffer_arg_name(e: &Engine, read: &Read, command: &str) -> Vec<u8> {
    buffer_arg(e, read, 0, NAME_BUFFER, command)
}

fn name_arg(e: &Engine, read: &Read, command: &str) -> Vec<u8> {
    buffer_arg_name(e, read, command)
}

/// A copy of value `idx` of the variable the arguments name (name, then an optional index and
/// form at `idx_at`).
fn read_value(e: &mut Engine, w: &[u32], command: &str) -> (CommandArgs, Option<Value>) {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let name = name_arg(e, &read, command);
    let mut found = None;
    if read.ok {
        if let Some(info) = info_named(e, &a, read.form(2), name) {
            let idx = read.int(1, 0) as i32;
            found = value_at(aux_maps(e, info.temp), &info, idx, false).cloned();
        }
    }
    (a, found)
}

// ---- the commands ----

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_aux_variable.h:
// Cmd_AuxiliaryVariableGetSize_Execute.
fn aux_get_size(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let name = name_arg(e, &read, "AuxiliaryVariableGetSize");
    if read.ok {
        if let Some(info) = info_named(e, &a, read.form(1), name) {
            if let Some(vals) = values_of(aux_maps(e, info.temp), &info, false) {
                let size = vals.len() as f64;
                a.set_result(e, size);
            }
        }
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_aux_variable.h:
// Cmd_AuxiliaryVariableGetType_Execute.
fn aux_get_type(e: &mut Engine, w: &[u32]) -> Ret {
    let (a, v) = read_value(e, w, "AuxiliaryVariableGetType");
    if let Some(v) = v {
        a.set_result(e, f64::from(v.kind()));
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_aux_variable.h:
// Cmd_AuxiliaryVariableGetFloat_Execute.
fn aux_get_float(e: &mut Engine, w: &[u32]) -> Ret {
    let (a, v) = read_value(e, w, "AuxiliaryVariableGetFloat");
    if let Some(v) = v {
        a.set_result(e, value_float(&v));
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_aux_variable.h:
// Cmd_AuxiliaryVariableGetRef_Execute.
/// The form's id in the result (`REFR_RES`), as stored, the form there or not.
fn aux_get_ref(e: &mut Engine, w: &[u32]) -> Ret {
    let (a, v) = read_value(e, w, "AuxiliaryVariableGetRef");
    if let Some(v) = v {
        a.set_form_result(e, value_ref(&v));
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_aux_variable.h:
// Cmd_AuxiliaryVariableGetString_Execute.
fn aux_get_string(e: &mut Engine, w: &[u32]) -> Ret {
    let (a, v) = read_value(e, w, "AuxiliaryVariableGetString");
    let text = v.as_ref().and_then(value_text);
    assign_string(e, &a, text);
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_aux_variable.h:
// Cmd_AuxiliaryVariableGetAsArray_Execute.
/// The values as an array; no array for none.
fn aux_get_as_array(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let name = name_arg(e, &read, "AuxiliaryVariableGetAsArray");
    if !read.ok {
        return handled(true);
    }
    let Some(info) = info_named(e, &a, read.form(1), name) else {
        return handled(true);
    };
    let Some(vals) = values_of(aux_maps(e, info.temp), &info, false).cloned() else {
        return handled(true);
    };
    let elements: Vec<Element> = vals.iter().map(|v| value_element(e, v)).collect();
    if !elements.is_empty() {
        let id = create_array(e, a.script, &elements);
        a.set_result(e, f64::from(id));
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_aux_variable.h:
// Cmd_AuxiliaryVariableGetAll_Execute.
/// The owner's variables as a map by name (the first spelling) of arrays of their values; the
/// map is made first, then each array in the owner's order.
fn aux_get_all(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if !read.ok {
        return handled(true);
    }
    let Some(info) = info_typed(e, &a, read.form(1), read.int(0, 0)) else {
        return handled(true);
    };
    let Some(vars) = aux_maps(e, info.temp)
        .get(&info.mod_index)
        .and_then(|owners| owners.get(&info.owner))
        .map(|vars| {
            vars.iter()
                .map(|(n, v)| (n.0.clone(), v.clone()))
                .collect::<Vec<_>>()
        })
    else {
        return handled(true);
    };
    let map = create_string_map(e, a.script, &[], &[]);
    for (name, vals) in vars {
        let elements: Vec<Element> = vals.iter().map(|v| value_element(e, v)).collect();
        let inner = create_array(e, a.script, &elements);
        set_element(e, map, &Key::Str(name), &Element::Array(inner));
    }
    a.set_result(e, f64::from(map));
    handled(true)
}

/// A setter: the value at the index (made when it is new), set by `make`; JIP's changed flag
/// and `MarkModified` for a saved variable; 1.
fn aux_set(e: &mut Engine, a: &CommandArgs, info: Info, idx: i32, value: Value) {
    let Some(slot) = value_at(aux_maps(e, info.temp), &info, idx, true) else {
        return;
    };
    *slot = value;
    if !info.temp {
        mark_changed(e, changed::AUX_VARS);
        mark_modified(e, a.this_obj);
    }
    a.set_result(e, 1.0);
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_aux_variable.h:
// Cmd_AuxiliaryVariableSetFloat_Execute.
fn aux_set_float(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let name = name_arg(e, &read, "AuxiliaryVariableSetFloat");
    if read.ok {
        if let Some(info) = info_named(e, &a, read.form(3), name) {
            let v = read.args.get(1).and_then(|x| x.double()).unwrap_or(0.0);
            aux_set(e, &a, info, read.int(2, 0) as i32, Value::Number(v));
        }
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_aux_variable.h:
// Cmd_AuxiliaryVariableSetRef_Execute.
/// The form's id (`refVal ? refVal->refID : 0`).
fn aux_set_ref(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let name = name_arg(e, &read, "AuxiliaryVariableSetRef");
    if read.ok {
        if let Some(info) = info_named(e, &a, read.form(3), name) {
            let form = read.form(1);
            let id = if form == 0 { 0 } else { e.mem.u32(form + 0xc) };
            aux_set(e, &a, info, read.int(2, 0) as i32, Value::Form(id));
        }
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_aux_variable.h:
// Cmd_AuxiliaryVariableSetString_Execute.
fn aux_set_string(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let name = name_arg(e, &read, "AuxiliaryVariableSetString");
    let text = buffer_arg(e, &read, 1, STR_BUFFER_SIZE, "AuxiliaryVariableSetString");
    if read.ok {
        if let Some(info) = info_named(e, &a, read.form(3), name) {
            aux_set(e, &a, info, read.int(2, 0) as i32, text_value(&text));
        }
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_aux_variable.h:
// Cmd_AuxiliaryVariableSetFromArray_Execute.
/// The variable's values replaced by a non-empty array's (its values in order); their count.
fn aux_set_from_array(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let name = name_arg(e, &read, "AuxiliaryVariableSetFromArray");
    let id = read.int(1, 0);
    if !read.ok || id == 0 {
        return handled(true);
    }
    let Some(data) = array_values(e, id).filter(|v| !v.is_empty()) else {
        return handled(true);
    };
    let Some(info) = info_named(e, &a, read.form(2), name) else {
        return handled(true);
    };
    let values: Vec<Value> = data.iter().map(|d| element_value(e, d)).collect();
    let count = values.len();
    if let Some(vals) = values_of(aux_maps(e, info.temp), &info, true) {
        *vals = values;
    }
    if !info.temp {
        mark_changed(e, changed::AUX_VARS);
        mark_modified(e, a.this_obj);
    }
    a.set_result(e, count as f64);
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_aux_variable.h:
// Cmd_AuxiliaryVariableErase_Execute.
/// -1 unless erased. A negative index (the default) erases the variable; another erases that
/// value (nothing past the end) and answers the count left. An emptied variable, owner and mod
/// go.
fn aux_erase(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    a.set_result(e, -1.0);
    let read = Read::of(e, &a);
    let name = name_arg(e, &read, "AuxiliaryVariableErase");
    if !read.ok {
        return handled(true);
    }
    let Some(info) = info_named(e, &a, read.form(2), name) else {
        return handled(true);
    };
    let idx = read.int(1, u32::MAX) as i32;
    let key = Name(info.name.clone());
    let maps = aux_maps(e, info.temp);
    let Some(owners) = maps.get_mut(&info.mod_index) else {
        return handled(true);
    };
    let Some(vars) = owners.get_mut(&info.owner) else {
        return handled(true);
    };
    let Some(vals) = vars.get_mut(&key) else {
        return handled(true);
    };
    let mut left = None;
    if idx >= 0 {
        if idx as usize >= vals.len() {
            return handled(true);
        }
        vals.remove(idx as usize);
        left = Some(vals.len());
        if vals.is_empty() {
            vars.erase(&key);
        }
    } else {
        vars.erase(&key);
    }
    if vars.is_empty() {
        owners.erase(&info.owner);
        if owners.is_empty() {
            maps.erase(&info.mod_index);
        }
    }
    if let Some(n) = left {
        a.set_result(e, n as f64);
    }
    if !info.temp {
        mark_changed(e, changed::AUX_VARS);
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_aux_variable.h:
// Cmd_AuxiliaryVariableEraseAll_Execute.
/// Every variable of the owner (temporary or saved, the script's mod or shared, by the type);
/// an emptied mod goes.
fn aux_erase_all(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if !read.ok {
        return handled(true);
    }
    let Some(info) = info_typed(e, &a, read.form(1), read.int(0, 0)) else {
        return handled(true);
    };
    let maps = aux_maps(e, info.temp);
    let Some(owners) = maps.get_mut(&info.mod_index) else {
        return handled(true);
    };
    if !owners.erase(&info.owner) {
        return handled(true);
    }
    if owners.is_empty() {
        maps.erase(&info.mod_index);
    }
    if !info.temp {
        mark_changed(e, changed::AUX_VARS);
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_aux_variable.h:
// Cmd_AuxVarGetFltCond_Eval.
/// The condition's quest (`arg1`) and its variable (`arg2`, an id in the quest's event list
/// +0x5C) holding a string variable's id, which names the variable; the calling reference owns
/// it, the quest's script (+0x1C) gives its mod. Value 0's number written when there is one;
/// the result is left alone otherwise (the port: no quest, nothing).
fn aux_get_flt_cond_eval(e: &mut Engine, w: &[u32]) -> Ret {
    let a = EvalArgs::new(w);
    if a.arg1 == 0 {
        return handled(true);
    }
    let list = e.mem.u32(a.arg1 + 0x5c);
    if list == 0 {
        return handled(true);
    }
    let var = nvse::args::event_list_variable(e, list, a.arg2);
    if var == 0 {
        return handled(true);
    }
    let id = cvttsd2si(e.mem.f64(var + 8)) as u32;
    let Some(name) = vars::string(e, id).filter(|n| !n.is_empty()) else {
        return handled(true);
    };
    let script = e.mem.u32(a.arg1 + 0x1c);
    let (temp, mod_index) = name_scope(e, script, &name);
    let info = Info {
        owner: subject_id(e, 0, a.this_obj),
        mod_index,
        name,
        temp,
    };
    if let Some(v) = value_at(aux_maps(e, temp), &info, 0, false) {
        let f = value_float(v);
        e.mem.set_f64(a.result, f);
    }
    handled(true)
}

#[cfg(test)]
#[path = "jip_fn_aux_variable_tests.rs"]
pub(crate) mod tests;
