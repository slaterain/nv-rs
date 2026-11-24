//! Seven AuxStringMap commands used by MCM Extender, ShowOff f45d069:
//! SO_fn_AuxVars.cpp:38-82,173-271,295-365; internal/jip_nvse.h;
//! internal/serialization.h and ShowOffNVSE.cpp::MessageHandler.
//! Names retain their prefixes: `*` is session storage, `_` after it shares
//! storage between mods. Permanent maps use ShowOff's actual SMSO co-save record.
use engine_core::prelude::*;
use nvse::{
    commands::{arg_in_buffer, Read},
    cosave::{self, Chunk, OutOfRange, Reader},
    handled,
    plugins::{self, BuiltIn, Message, Records},
    table::{self, PluginRecord, Returns},
    vars::{self, Container, Data, Key, Value},
    CommandArgs,
};
use std::{
    collections::BTreeMap,
    sync::{Mutex, MutexGuard},
};

const RECORD: u32 = cosave::tag(b"SMSO");
const LOOKUP_FORM: u32 = 0x0048_39c0;

#[derive(Clone, Debug, PartialEq)]
enum Scalar {
    Number(f64),
    Form(u32),
    Text(u8, Vec<u8>),
}
#[derive(Clone, Default)]
struct Map {
    name: Vec<u8>,
    entries: BTreeMap<u32, (Vec<u8>, Scalar)>,
}
#[derive(Default)]
struct State {
    maps: BTreeMap<(bool, u8, u32), Map>,
    changed: bool,
    last_path: Vec<u8>,
    // Other ShowOff records are not implemented by this command slice.
    other: Vec<Chunk>,
}
#[derive(Default)]
struct Store(Mutex<State>);
/// A source static global, shared by real-thread engine views. No guard may
/// survive an engine call, NVSE array operation or string assignment.
fn state(e: &mut Engine) -> MutexGuard<'_, State> {
    e.ext_free::<Store>()
        .0
        .lock()
        .unwrap_or_else(|p| p.into_inner())
}

/// HashedKey<char*>::Match compares hashes, including collisions, not strings.
/// internal/containers.h and containers.cpp::StrHashCI (same JIP-derived hash).
fn hash(text: &[u8]) -> u32 {
    let mut h = 0xd39c_a509u32;
    for (i, b) in text.iter().take_while(|&&b| b != 0).enumerate() {
        h = h
            .wrapping_sub(u32::from(b.to_ascii_lowercase()) << [4, 12, 20, 0][i % 4])
            .wrapping_mul(31);
    }
    h
}
fn scope(e: &Engine, script: u32, name: &[u8]) -> (bool, u8, u32) {
    let temporary = name.first() == Some(&b'*');
    let owner = if name.get(usize::from(temporary)) == Some(&b'_') || script == 0 {
        0xff
    } else {
        // TESForm::GetOverridingModIdx, the last file in the form's mods list.
        let mut node = script + 0x10;
        while e.mem.u32(node + 4) != 0 {
            node = e.mem.u32(node + 4);
        }
        let file = e.mem.u32(node);
        if file == 0 {
            0xff
        } else {
            e.mem.u8(file + 0x40c)
        }
    };
    (temporary, owner, hash(name))
}
fn name(e: &Engine, read: &Read, i: usize) -> Vec<u8> {
    arg_in_buffer(e, read, i, 256, "AuxStringMap")
}
fn text_value(v: &[u8]) -> Scalar {
    let text = v.split(|&b| b == 0).next().unwrap_or_default();
    // AuxVariableValue::length is uint16_t before the copy.
    Scalar::Text(4, text[..usize::from(text.len() as u16)].to_vec())
}
fn put(e: &mut Engine, at: (bool, u8, u32), name: &[u8], key: &[u8], value: Scalar) {
    let key = key.split(|&b| b == 0).next().unwrap_or_default();
    if name.is_empty() || key.is_empty() {
        return;
    }
    let mut s = state(e);
    if !at.0 {
        s.changed = true;
    }
    let map = s.maps.entry(at).or_insert_with(|| Map {
        name: name.to_vec(),
        ..Map::default()
    });
    // Emplace keeps the spelling with which a hash was first inserted.
    map.entries
        .entry(hash(key))
        .and_modify(|(_, old)| *old = value.clone())
        .or_insert_with(|| (key.to_vec(), value));
}
fn find(e: &mut Engine, script: u32, name: &[u8], key: &[u8]) -> Option<Scalar> {
    if name.is_empty() {
        return None;
    }
    let at = scope(e, script, name);
    state(e)
        .maps
        .get(&at)?
        .entries
        .get(&hash(key))
        .map(|(_, v)| v.clone())
}
fn get_float(e: &mut Engine, words: &[u32]) -> Ret {
    let a = CommandArgs::new(words);
    a.set_result(e, 0.0);
    let r = Read::of(e, &a);
    if r.ok {
        let (n, k) = (name(e, &r, 0), name(e, &r, 1));
        if let Some(Scalar::Number(v)) = find(e, a.script, &n, &k) {
            a.set_result(e, v);
        }
    }
    handled(true)
}
fn get_string(e: &mut Engine, words: &[u32]) -> Ret {
    let a = CommandArgs::new(words);
    let r = Read::of(e, &a);
    let value = if r.ok {
        let (n, k) = (name(e, &r, 0), name(e, &r, 1));
        find(e, a.script, &n, &k)
    } else {
        None
    };
    let text = match value {
        Some(Scalar::Text(_, v)) => v,
        _ => Vec::new(),
    };
    // AssignString is called even when extraction fails.
    nvse::commands::string::assign_to_string_var(e, &a, &text);
    handled(true)
}
fn get_array(e: &mut Engine, words: &[u32]) -> Ret {
    let a = CommandArgs::new(words);
    a.set_result(e, 0.0);
    let r = Read::of(e, &a);
    if !r.ok {
        return handled(true);
    }
    let n = name(e, &r, 0);
    if n.is_empty() {
        return handled(true);
    }
    let at = scope(e, a.script, &n);
    let Some(map) = state(e).maps.get(&at).cloned() else {
        return handled(true);
    };
    let owner = vars::script_mod_index(e, a.script);
    let id = vars::create_array(e, vars::KEY_STRING, false, owner);
    for (_, (key, value)) in map.entries {
        let value = match value {
            Scalar::Number(v) => Value::Number(v),
            Scalar::Text(4, v) => Value::Str(v),
            Scalar::Text(_, _) => Value::Number(0.0),
            Scalar::Form(form) => {
                let ptr = e.call(LOOKUP_FORM, &args![form]).u32();
                Value::Form(if ptr == 0 { 0 } else { e.mem.u32(ptr + 0xc) })
            }
        };
        vars::set_element(e, id, &Key::Str(key), value);
    }
    // ArrayAPI::AssignArrayCommandResult casts the array handle through int.
    a.set_result(e, f64::from(id as i32));
    handled(true)
}
fn set_float(e: &mut Engine, words: &[u32]) -> Ret {
    let a = CommandArgs::new(words);
    let r = Read::of(e, &a);
    if r.ok {
        let (n, k) = (name(e, &r, 0), name(e, &r, 1));
        let at = scope(e, a.script, &n);
        let value = r
            .args
            .get(2)
            .and_then(nvse::args::Arg::double)
            .unwrap_or(0.0);
        put(e, at, &n, &k, Scalar::Number(value));
    }
    // The original setter leaves result untouched.
    handled(true)
}
fn set_array(e: &mut Engine, words: &[u32]) -> Ret {
    let a = CommandArgs::new(words);
    a.set_result(e, 0.0);
    let r = Read::of(e, &a);
    if !r.ok {
        return handled(true);
    }
    let n = name(e, &r, 0);
    let at = scope(e, a.script, &n);
    if r.int(2, 0) == 0 {
        state(e).maps.remove(&at);
    }
    // Clearing precedes array validation, including its changed-flag update.
    let Some(array) = vars::array(e, r.int(1, 0)).cloned() else {
        return handled(true);
    };
    if let Container::StrMap(entries) = array.elements {
        for (key, element) in entries {
            let value = match element.data {
                Data::Number(v) => Scalar::Number(v),
                Data::Str(v) => text_value(&v),
                Data::Form(id) => {
                    let ptr = e.call(LOOKUP_FORM, &args![id]).u32();
                    Scalar::Form(if ptr == 0 { 0 } else { e.mem.u32(ptr + 0xc) })
                }
                // SetElem's fallback reads the ArrayElement's double union.
                // C++ leaves the upper word indeterminate for an array handle;
                // use zero, as the existing JIP aux-variable conversion does.
                Data::Array(id) => Scalar::Number(f64::from_bits(u64::from(id))),
                Data::Invalid => Scalar::Number(0.0),
            };
            put(e, at, &n, &key, value);
        }
    }
    // The source indexes keys[0] for an empty input; avoid that undefined read.
    if !at.0 {
        state(e).changed = true;
    }
    handled(true)
}
fn erase(e: &mut Engine, words: &[u32]) -> Ret {
    let a = CommandArgs::new(words);
    a.set_result(e, 0.0);
    let r = Read::of(e, &a);
    if r.ok {
        let (n, k) = (name(e, &r, 0), name(e, &r, 1));
        let at = scope(e, a.script, &n);
        if !n.is_empty() {
            let mut s = state(e);
            if let Some(map) = s.maps.get_mut(&at) {
                if map.entries.remove(&hash(&k)).is_some() {
                    let left = map.entries.len();
                    if left == 0 {
                        s.maps.remove(&at);
                    }
                    if !at.0 {
                        s.changed = true;
                    }
                    drop(s);
                    a.set_result(e, left as f64);
                }
            }
        }
    }
    handled(true)
}
fn destroy(e: &mut Engine, words: &[u32]) -> Ret {
    let a = CommandArgs::new(words);
    let r = Read::of(e, &a);
    if r.ok {
        let n = name(e, &r, 0);
        let at = scope(e, a.script, &n);
        if !n.is_empty() {
            let mut s = state(e);
            if s.maps.remove(&at).is_some() && !at.0 {
                s.changed = true;
            }
        }
    }
    handled(true)
}

fn message(e: &mut Engine, m: &Message) {
    crate::timers::message(e, m);
    use plugins::msg;
    let path = if matches!(m.kind, msg::SAVE_GAME | msg::PRE_LOAD_GAME) {
        table::guest_text(e, m.data)
    } else {
        Vec::new()
    };
    let mut s = state(e);
    match m.kind {
        msg::SAVE_GAME => {
            s.last_path = path;
            s.changed = false;
        }
        msg::PRE_LOAD_GAME => {
            if s.last_path != path {
                s.last_path = path;
                s.changed = true;
            }
        }
        msg::POST_LOAD_GAME if m.data != 0 => s.changed = false,
        msg::EXIT_TO_MAIN_MENU => {
            s.maps.retain(|at, _| at.0);
            s.other.clear();
            s.last_path.clear();
        }
        _ => {}
    }
}
fn save(e: &mut Engine, out: &mut Records) {
    let s = state(e);
    let mut mods: BTreeMap<u8, Vec<&Map>> = BTreeMap::new();
    for ((temp, owner, _), map) in &s.maps {
        if !temp {
            mods.entry(*owner).or_default().push(map);
        }
    }
    if !mods.is_empty() {
        out.open(RECORD, 10);
        out.u16(mods.len() as u16);
        for (owner, maps) in mods {
            out.u8(owner);
            out.u16(maps.len() as u16);
            for map in maps {
                out.u8(map.name.len() as u8);
                out.data(&map.name[..usize::from(map.name.len() as u8)]);
                out.u16(map.entries.len() as u16);
                for (key, value) in map.entries.values() {
                    out.u8(key.len() as u8);
                    out.data(&key[..usize::from(key.len() as u8)]);
                    match value {
                        Scalar::Number(v) => {
                            out.u8(1);
                            out.u64(v.to_bits());
                        }
                        Scalar::Form(v) => {
                            out.u8(2);
                            out.u32(*v);
                        }
                        Scalar::Text(kind, v) => {
                            out.u8(*kind);
                            out.u16(v.len() as u16);
                            out.data(&v[..usize::from(v.len() as u16)]);
                        }
                    }
                }
            }
        }
    }
    out.chunks.extend(s.other.clone());
}
fn read_text(r: &mut Reader, width: u8) -> Result<Vec<u8>, OutOfRange> {
    let n = if width == 1 {
        u32::from(r.read8()?)
    } else {
        u32::from(r.read16()?)
    };
    if n > r.left() {
        return Err(OutOfRange);
    }
    r.read_data(n)
}
fn load(e: &mut Engine, r: &mut Reader) -> Result<(), OutOfRange> {
    let changed = state(e).changed;
    if changed {
        state(e).maps.retain(|at, _| at.0);
    }
    state(e).other.clear();
    while let Some((kind, version, length)) = r.next_record()? {
        if kind != RECORD {
            let data = r.read_data(length)?;
            state(e).other.push(Chunk {
                kind,
                version,
                data,
            });
            continue;
        }
        if !changed {
            continue;
        }
        let owners = r.read16()?;
        for _ in 0..owners {
            let old = r.read8()?;
            let owner =
                nvse::saveload::resolve_ref_id(e, u32::from(old) << 24).map(|id| (id >> 24) as u8);
            let count = r.read16()?;
            for _ in 0..count {
                let name = read_text(r, 1)?;
                let count = r.read16()?;
                for _ in 0..count {
                    let key = read_text(r, 1)?;
                    let kind = r.read8()?;
                    let value = match kind {
                        1 => Scalar::Number(r.read64()?.ok_or(OutOfRange)?),
                        2 => {
                            let id = r.read32()?;
                            Scalar::Form(nvse::saveload::resolve_ref_id(e, id).unwrap_or(0))
                        }
                        _ => Scalar::Text(kind, read_text(r, 2)?),
                    };
                    // Unlike the C++ loader's missing-mod continue, consume its payload
                    // before dropping it, so subsequent loaded mods remain readable.
                    if let Some(owner) = owner {
                        put(e, (false, owner, hash(&name)), &name, &key, value);
                    }
                }
            }
        }
    }
    Ok(())
}

fn new_game(e: &mut Engine) {
    // xNVSE uses this callback when the .fos has no co-save. Opaque records
    // belong to the loaded co-save and cannot be carried into that game.
    // Original ShowOff has no new-game handling for its typed map state.
    state(e).other.clear();
}

pub(crate) fn install(e: &mut Engine) {
    e.ext_free::<Store>();
    plugins::register(
        e,
        BuiltIn {
            file: "ShowOffNVSE.dll",
            name: "ShowOffNVSE Plugin",
            version: 184,
            opcode_base: 0x3c93,
            listed: false,
            listener: Some(message),
            save: Some(save),
            load: Some(load),
            pre_load: None,
            new_game: Some(new_game),
        },
    );
    table::grow_to(e, 0x3cc1);
    for (op, name, short, spec, returns, handler) in [
        (
            0x3cb2,
            "AuxStringMapArrayGetFloat",
            "AuxStringMapGetFlt",
            &[(0, false), (0, false)][..],
            Returns::Default,
            get_float as AbiFn,
        ),
        (
            0x3cb4,
            "AuxStringMapArrayGetString",
            "AuxStringMapGetStr",
            &[(0, false), (0, false)][..],
            Returns::String,
            get_string,
        ),
        (
            0x3cb9,
            "AuxStringMapArrayGetAsArray",
            "AuxStringMapGetAsArr",
            &[(0, false)][..],
            Returns::Array,
            get_array,
        ),
        (
            0x3cba,
            "AuxStringMapArraySetFromArray",
            "AuxStringMapSetFromArr",
            &[(0, false), (1, false), (1, true)][..],
            Returns::Default,
            set_array,
        ),
        (
            0x3cbb,
            "AuxStringMapArraySetFloat",
            "AuxStringMapSetFlt",
            &[(0, false), (0, false), (0x2c, false)][..],
            Returns::Default,
            set_float,
        ),
        (
            0x3cbe,
            "AuxStringMapArrayEraseKey",
            "AuxStringMapEraseKey",
            &[(0, false), (0, false)][..],
            Returns::Default,
            erase,
        ),
        (
            0x3cc0,
            "AuxStringMapArrayDestroy",
            "AuxStringMapDestroy",
            &[(0, false)][..],
            Returns::Default,
            destroy,
        ),
    ] {
        let params = table::alloc(e, 12 * spec.len() as u32);
        for (i, &(kind, optional)) in spec.iter().enumerate() {
            let at = params + 12 * i as u32;
            let text = table::guest_str(e, if kind == 0 { b"string" } else { b"number" });
            e.mem.set_u32(at, text);
            e.mem.set_u32(at + 4, kind);
            e.mem.set_u32(at + 8, u32::from(optional));
        }
        let execute = table::execute_address(op);
        e.register(execute, handler);
        table::add_plugin_record(
            e,
            op,
            &PluginRecord {
                name,
                short,
                needs_ref: false,
                param_count: spec.len() as u16,
                params,
                execute,
                eval: 0,
                flags: 0,
                expression: false,
                returns,
            },
        );
    }
}

#[cfg(test)]
mod tests;
