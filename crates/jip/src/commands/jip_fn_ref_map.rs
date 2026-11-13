//! `functions_jip/jip_fn_ref_map.h` (JIP LN NVSE): ref maps, named maps from a form's id (the
//! form argument, a reference's base form, or else the calling reference) to a number, a form or
//! a text, per mod; all 17 of its commands.
//!
//! As with the aux variables ([`super::jip_fn_aux_variable`]): the saved maps are JIP's
//! `s_refMapArrays[0]`, the co-save's own ([`crate::cosave::Saved::ref_maps`], JIP's `JPRM`
//! record byte for byte), and a change to them sets JIP's changed flag for them
//! (`kChangedFlag_RefMaps`); a name beginning `*` is temporary (`s_refMapArrays[1]`, for the
//! session); a name beginning (after `*`) with `_` is shared (mod 0xFF), any other belongs to the
//! calling script's mod. A mod's names map and a name's map are made with 4 buckets each, as
//! JIP's (`RefMapVarsMap`, `RefMapIDsMap`).
//!
//! `RefMapArrayGetFirst`/`GetNext` walk with JIP's one iterator (`s_refMapIterator`, [`Walk`]):
//! the map it walks (a different map or `GetFirst` starts again) and the entry it gives next,
//! which must still be in the bucket it was found in (`Iterator::IsValid`), else the walk ends.

use super::arrays::{create_array, create_string_map, set_element, Element};
use super::jip_fn_aux_variable::{
    array_values, buffer_arg_name, element_value, mark_changed, mark_modified, name_scope,
    subject_id, text_value, type_scope, value_element, value_float, value_ref, value_text, RefMods,
    Temp, STR_BUFFER_SIZE,
};
use super::{assign_string, buffer_arg, Handlers};
use crate::containers::{mixed_hash, str_hash_ci, Name, UnorderedMap};
use crate::cosave::{changed, lookup_form, Value};
use crate::Jip;
use engine_core::prelude::*;
use nvse::commands::Read;
use nvse::vars::Key;
use nvse::{handled, CommandArgs};

#[rustfmt::skip]
pub static COMMANDS: Handlers = &[
    (0x269c, ref_map_get_size),
    (0x269d, ref_map_get_type),
    (0x269e, ref_map_get_first),
    (0x269f, ref_map_get_next),
    (0x26a0, ref_map_get_keys),
    (0x26a1, ref_map_get_all),
    (0x26a2, ref_map_erase),
    (0x26a3, ref_map_destroy),
    (0x26a4, ref_map_get_float),
    (0x26a5, ref_map_set_float),
    (0x26a6, ref_map_get_ref),
    (0x26a7, ref_map_set_ref),
    (0x26a8, ref_map_get_string),
    (0x26a9, ref_map_set_string),
    (0x26f0, ref_map_validate),
    (0x2764, ref_map_get_value),
    (0x2765, ref_map_set_value),
];

/// A mod's names (`RefMapVarsMap`) and a name's ids (`RefMapIDsMap`): 4 buckets each.
const NAMES_BUCKETS: u32 = 4;
const IDS_BUCKETS: u32 = 4;

/// The ref maps: the saved ones or the temporary ones.
fn maps(e: &mut Engine, temp: bool) -> &mut RefMods {
    if temp {
        &mut e.ext::<Temp>().ref_maps
    } else {
        &mut e.ext::<Jip>().saved.ref_maps
    }
}

/// `s_refMapIterator`: the map walked (temporary, mod, name) and the entry it gives next (its
/// id, its bucket and the map's bucket count then).
#[derive(Clone, Debug)]
pub struct Walk {
    temp: bool,
    mod_index: u32,
    name: Name,
    next: Option<(u32, u32, u32)>,
}

/// The map named by a non-empty name (`RMFind`), with its place.
fn find<'a>(
    e: &'a mut Engine,
    script: u32,
    name: &[u8],
) -> Option<&'a mut UnorderedMap<u32, Value>> {
    if name.is_empty() {
        return None;
    }
    let (temp, m) = name_scope(e, script, name);
    maps(e, temp).get_mut(&m)?.get_mut(&Name(name.to_vec()))
}

/// The name argument (JIP's 0x50-byte buffer).
fn name_arg(e: &Engine, read: &Read, command: &str) -> Vec<u8> {
    buffer_arg_name(e, read, command)
}

/// A copy of the value of the key the arguments give (name, then an optional form).
fn read_value(e: &mut Engine, w: &[u32], command: &str) -> (CommandArgs, Option<Value>) {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let name = name_arg(e, &read, command);
    let mut found = None;
    if read.ok {
        let key = subject_id(e, read.form(1), a.this_obj);
        if let Some(ids) = find(e, a.script, &name) {
            found = ids.get(&key).cloned();
        }
    }
    (a, found)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_ref_map.h:
// Cmd_RefMapArrayGetSize_Execute.
fn ref_map_get_size(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let name = name_arg(e, &read, "RefMapArrayGetSize");
    if read.ok {
        if let Some(ids) = find(e, a.script, &name) {
            let n = f64::from(ids.len());
            a.set_result(e, n);
        }
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_ref_map.h:
// Cmd_RefMapArrayGetType_Execute.
fn ref_map_get_type(e: &mut Engine, w: &[u32]) -> Ret {
    let (a, v) = read_value(e, w, "RefMapArrayGetType");
    if let Some(v) = v {
        a.set_result(e, f64::from(v.kind()));
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_ref_map.h:
// Cmd_RefMapArrayGetFloat_Execute.
fn ref_map_get_float(e: &mut Engine, w: &[u32]) -> Ret {
    let (a, v) = read_value(e, w, "RefMapArrayGetFloat");
    if let Some(v) = v {
        a.set_result(e, value_float(&v));
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_ref_map.h:
// Cmd_RefMapArrayGetRef_Execute.
fn ref_map_get_ref(e: &mut Engine, w: &[u32]) -> Ret {
    let (a, v) = read_value(e, w, "RefMapArrayGetRef");
    if let Some(v) = v {
        a.set_form_result(e, value_ref(&v));
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_ref_map.h:
// Cmd_RefMapArrayGetString_Execute.
fn ref_map_get_string(e: &mut Engine, w: &[u32]) -> Ret {
    let (a, v) = read_value(e, w, "RefMapArrayGetString");
    let text = v.as_ref().and_then(value_text);
    assign_string(e, &a, text);
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_ref_map.h:
// Cmd_RefMapArrayGetValue_Execute.
/// The value as a one-element array.
fn ref_map_get_value(e: &mut Engine, w: &[u32]) -> Ret {
    let (a, v) = read_value(e, w, "RefMapArrayGetValue");
    if let Some(v) = v {
        let element = value_element(e, &v);
        let id = create_array(e, a.script, &[element]);
        a.set_result(e, f64::from(id));
    }
    handled(true)
}

/// The bucket an id falls in with `count` buckets.
fn bucket_of(id: u32, count: u32) -> u32 {
    mixed_hash(id) & (count - 1)
}

/// The entry after `id` in the map's walking order, with its bucket.
fn after(ids: &UnorderedMap<u32, Value>, id: Option<u32>) -> Option<(u32, u32, u32)> {
    let count = ids.bucket_count();
    let mut keys = ids.iter().map(|(k, _)| *k);
    let next = match id {
        None => keys.next(),
        Some(id) => {
            let mut keys = keys.skip_while(|&k| k != id);
            keys.next();
            keys.next()
        }
    }?;
    Some((next, bucket_of(next, count), count))
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_ref_map.h:
// RefMapArrayIterator.
/// The walk's next entry as an array of the form (looked up by its id) and the value; 0 at the
/// end or for no map.
fn walk(e: &mut Engine, a: &CommandArgs, name: &[u8], first: bool) -> u32 {
    if name.is_empty() {
        return 0;
    }
    let (temp, m) = name_scope(e, a.script, name);
    let key = Name(name.to_vec());
    let Some(ids) = maps(e, temp)
        .get(&m)
        .and_then(|names| names.get(&key))
        .cloned()
    else {
        return 0;
    };
    let mut state = e.ext::<Temp>().ref_map_walk.clone();
    let same = state.as_ref().is_some_and(|w| {
        w.temp == temp && w.mod_index == m && str_hash_ci(&w.name.0) == str_hash_ci(name)
    });
    if first || !same {
        state = Some(Walk {
            temp,
            mod_index: m,
            name: key,
            next: after(&ids, None),
        });
    } else if let Some(w) = state.as_mut() {
        // `IsValid`: the entry is still in the bucket it was found in.
        let valid = w.next.is_some_and(|(id, bucket, count)| {
            ids.get(&id).is_some()
                && ids.bucket_count() == count
                && bucket_of(id, ids.bucket_count()) == bucket
        });
        if !valid {
            w.next = None;
        }
    }
    let Some(w) = state.as_mut() else {
        return 0;
    };
    let Some((id, _, _)) = w.next else {
        e.ext::<Temp>().ref_map_walk = state;
        return 0;
    };
    let value = ids.get(&id).cloned().unwrap_or(Value::Number(0.0));
    w.next = after(&ids, Some(id));
    e.ext::<Temp>().ref_map_walk = state;
    let elements = [Element::Form(lookup_form(e, id)), value_element(e, &value)];
    create_array(e, a.script, &elements)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_ref_map.h:
// Cmd_RefMapArrayGetFirst_Execute.
fn ref_map_get_first(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let name = name_arg(e, &read, "RefMapArrayGetFirst");
    if read.ok {
        let id = walk(e, &a, &name, true);
        a.set_result(e, f64::from(id));
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_ref_map.h:
// Cmd_RefMapArrayGetNext_Execute.
fn ref_map_get_next(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let name = name_arg(e, &read, "RefMapArrayGetNext");
    if read.ok {
        let id = walk(e, &a, &name, false);
        a.set_result(e, f64::from(id));
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_ref_map.h:
// Cmd_RefMapArrayGetKeys_Execute.
/// The keys' forms (each looked up by its id: 0 when gone) in the map's order.
fn ref_map_get_keys(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let name = name_arg(e, &read, "RefMapArrayGetKeys");
    if !read.ok {
        return handled(true);
    }
    let Some(keys) =
        find(e, a.script, &name).map(|ids| ids.iter().map(|(k, _)| *k).collect::<Vec<_>>())
    else {
        return handled(true);
    };
    let elements: Vec<Element> = keys
        .iter()
        .map(|&k| Element::Form(lookup_form(e, k)))
        .collect();
    if !elements.is_empty() {
        let id = create_array(e, a.script, &elements);
        a.set_result(e, f64::from(id));
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_ref_map.h:
// Cmd_RefMapArrayGetAll_Execute.
/// The mod's maps (temporary or saved, the script's mod or shared, by the type) as a map by name
/// of arrays of their keys' forms; the map made first, then each array.
fn ref_map_get_all(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if !read.ok {
        return handled(true);
    }
    let (temp, m) = type_scope(e, a.script, read.int(0, 0));
    let Some(all) = maps(e, temp)
        .get(&m)
        .filter(|names| !names.is_empty())
        .map(|names| {
            names
                .iter()
                .map(|(n, ids)| (n.0.clone(), ids.iter().map(|(k, _)| *k).collect::<Vec<_>>()))
                .collect::<Vec<_>>()
        })
    else {
        return handled(true);
    };
    let map = create_string_map(e, a.script, &[], &[]);
    for (name, keys) in all {
        let elements: Vec<Element> = keys
            .iter()
            .map(|&k| Element::Form(lookup_form(e, k)))
            .collect();
        let inner = create_array(e, a.script, &elements);
        set_element(e, map, &Key::Str(name), &Element::Array(inner));
    }
    a.set_result(e, f64::from(map));
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_ref_map.h: RefMapAddValue.
/// The value of the key in the named map, made (with its maps) when new; JIP's changed flag and
/// `MarkModified` for a saved map. `None` for an empty name or no key.
fn add_value(e: &mut Engine, a: &CommandArgs, form: u32, name: &[u8], value: Value) {
    if name.is_empty() {
        return;
    }
    let key = subject_id(e, form, a.this_obj);
    if key == 0 {
        return;
    }
    let (temp, m) = name_scope(e, a.script, name);
    if !temp {
        mark_changed(e, changed::REF_MAPS);
        mark_modified(e, a.this_obj);
    }
    let slot = maps(e, temp)
        .emplace_with(m, || UnorderedMap::new(NAMES_BUCKETS))
        .0
        .emplace_with(Name(name.to_vec()), || UnorderedMap::new(IDS_BUCKETS))
        .0
        .emplace_with(key, || Value::Number(0.0))
        .0;
    *slot = value;
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_ref_map.h:
// Cmd_RefMapArraySetFloat_Execute.
fn ref_map_set_float(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let name = name_arg(e, &read, "RefMapArraySetFloat");
    if read.ok {
        let v = read.args.get(1).and_then(|x| x.double()).unwrap_or(0.0);
        add_value(e, &a, read.form(2), &name, Value::Number(v));
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_ref_map.h:
// Cmd_RefMapArraySetRef_Execute.
fn ref_map_set_ref(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let name = name_arg(e, &read, "RefMapArraySetRef");
    if read.ok {
        let form = read.form(1);
        let id = if form == 0 { 0 } else { e.mem.u32(form + 0xc) };
        add_value(e, &a, read.form(2), &name, Value::Form(id));
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_ref_map.h:
// Cmd_RefMapArraySetString_Execute.
fn ref_map_set_string(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let name = name_arg(e, &read, "RefMapArraySetString");
    let text = buffer_arg(e, &read, 1, STR_BUFFER_SIZE, "RefMapArraySetString");
    if read.ok {
        add_value(e, &a, read.form(2), &name, text_value(&text));
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_ref_map.h:
// Cmd_RefMapArraySetValue_Execute.
/// The one value of a one-element array.
fn ref_map_set_value(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let name = name_arg(e, &read, "RefMapArraySetValue");
    if !read.ok {
        return handled(true);
    }
    let Some(data) = array_values(e, read.int(1, 0)).filter(|v| v.len() == 1) else {
        return handled(true);
    };
    let value = element_value(e, &data[0]);
    add_value(e, &a, read.form(2), &name, value);
    handled(true)
}

/// The named map's place: temporary, mod, name; `None` for an empty name.
fn place(e: &Engine, script: u32, name: &[u8]) -> Option<(bool, u32, Name)> {
    if name.is_empty() {
        return None;
    }
    let (temp, m) = name_scope(e, script, name);
    Some((temp, m, Name(name.to_vec())))
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_ref_map.h:
// Cmd_RefMapArrayErase_Execute.
/// The key's entry erased; the count left, or the map (and an emptied mod) gone when it was the
/// last.
fn ref_map_erase(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let name = name_arg(e, &read, "RefMapArrayErase");
    if !read.ok {
        return handled(true);
    }
    let Some((temp, m, key)) = place(e, a.script, &name) else {
        return handled(true);
    };
    let id = subject_id(e, read.form(1), a.this_obj);
    let all = maps(e, temp);
    let Some(names) = all.get_mut(&m) else {
        return handled(true);
    };
    let Some(ids) = names.get_mut(&key) else {
        return handled(true);
    };
    if !ids.erase(&id) {
        return handled(true);
    }
    let mut left = None;
    if ids.is_empty() {
        names.erase(&key);
        if names.is_empty() {
            all.erase(&m);
        }
    } else {
        left = Some(ids.len());
    }
    if let Some(n) = left {
        a.set_result(e, f64::from(n));
    }
    if !temp {
        mark_changed(e, changed::REF_MAPS);
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_ref_map.h:
// Cmd_RefMapArrayValidate_Execute.
/// The entries whose form is gone erased; the count left, or the map gone when emptied. JIP's
/// changed flag only when one was erased.
fn ref_map_validate(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let name = name_arg(e, &read, "RefMapArrayValidate");
    if !read.ok {
        return handled(true);
    }
    let Some((temp, m, key)) = place(e, a.script, &name) else {
        return handled(true);
    };
    let Some(keys) = maps(e, temp)
        .get(&m)
        .and_then(|names| names.get(&key))
        .map(|ids| ids.iter().map(|(k, _)| *k).collect::<Vec<_>>())
    else {
        return handled(true);
    };
    let gone: Vec<u32> = keys
        .into_iter()
        .filter(|&k| lookup_form(e, k) == 0)
        .collect();
    let all = maps(e, temp);
    let names = all.get_mut(&m).expect("found above");
    let ids = names.get_mut(&key).expect("found above");
    ids.retain(|k, _| !gone.contains(k));
    let mut left = None;
    if ids.is_empty() {
        names.erase(&key);
        if names.is_empty() {
            all.erase(&m);
        }
    } else {
        left = Some(ids.len());
    }
    if let Some(n) = left {
        a.set_result(e, f64::from(n));
    }
    if !gone.is_empty() && !temp {
        mark_changed(e, changed::REF_MAPS);
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_ref_map.h:
// Cmd_RefMapArrayDestroy_Execute.
/// The named map gone (and an emptied mod).
fn ref_map_destroy(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let name = name_arg(e, &read, "RefMapArrayDestroy");
    if !read.ok {
        return handled(true);
    }
    let Some((temp, m, key)) = place(e, a.script, &name) else {
        return handled(true);
    };
    let all = maps(e, temp);
    let Some(names) = all.get_mut(&m) else {
        return handled(true);
    };
    if !names.erase(&key) {
        return handled(true);
    }
    if names.is_empty() {
        all.erase(&m);
    }
    if !temp {
        mark_changed(e, changed::REF_MAPS);
    }
    handled(true)
}

#[cfg(test)]
#[path = "jip_fn_ref_map_tests.rs"]
mod tests;
