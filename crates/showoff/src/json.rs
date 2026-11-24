//! Partial ShowOff f45d069 ReadFromJSONFile (SO_fn_Files.cpp:32-367,516-594).
//! Parser 0 is standard JSON. Parser 1 (JAXN) and 2 (TaoConfig) deliberately
//! report an unsupported-parser runtime error; they are not parsed as JSON.
//! This does not advertise a complete ShowOff plugin or implement WriteToJSON.
//! Tao's first duplicate object member wins (events/to_value.hpp::member).
use engine_core::{mod_files, prelude::*};
use nvse::{
    expr::{Eval, Tok, Ty},
    handled,
    table::{self, PluginRecord, Returns},
    vars::{self, Key, Value},
    CommandArgs,
};
use serde::de::{self, Deserialize, Deserializer, MapAccess, SeqAccess, Visitor};
use std::{
    collections::{BTreeMap, HashMap},
    fmt,
    sync::Mutex,
};

const OPCODE: u32 = 0x3d1a;
const LOOKUP_FORM: u32 = 0x0048_39c0;
const GET_LIST_FILE: u32 = 0x0046_2f40;
const WIDE_TO_ANSI: u32 = 0x00fd_f170;

#[derive(Clone, Debug, PartialEq)]
enum Json {
    Null,
    Bool(bool),
    Number(f64),
    String(String),
    Array(Vec<Json>),
    Object(BTreeMap<String, Json>),
}
impl<'de> Deserialize<'de> for Json {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct Build;
        impl<'de> Visitor<'de> for Build {
            type Value = Json;
            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("a standard JSON value")
            }
            fn visit_unit<E: de::Error>(self) -> Result<Json, E> {
                Ok(Json::Null)
            }
            fn visit_bool<E: de::Error>(self, v: bool) -> Result<Json, E> {
                Ok(Json::Bool(v))
            }
            fn visit_i64<E: de::Error>(self, v: i64) -> Result<Json, E> {
                Ok(Json::Number(v as f64))
            }
            fn visit_u64<E: de::Error>(self, v: u64) -> Result<Json, E> {
                Ok(Json::Number(v as f64))
            }
            fn visit_f64<E: de::Error>(self, v: f64) -> Result<Json, E> {
                Ok(Json::Number(v))
            }
            fn visit_str<E: de::Error>(self, v: &str) -> Result<Json, E> {
                Ok(Json::String(v.to_owned()))
            }
            fn visit_string<E: de::Error>(self, v: String) -> Result<Json, E> {
                Ok(Json::String(v))
            }
            fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Json, A::Error> {
                let mut out = Vec::new();
                while let Some(v) = seq.next_element()? {
                    out.push(v);
                }
                Ok(Json::Array(out))
            }
            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Json, A::Error> {
                let mut out = BTreeMap::new();
                while let Some((k, v)) = map.next_entry::<String, Json>()? {
                    out.entry(k).or_insert(v);
                }
                Ok(Json::Object(out))
            }
        }
        deserializer.deserialize_any(Build)
    }
}
#[derive(Default)]
struct State {
    trees: HashMap<u32, Json>,
    refs: HashMap<u32, u32>,
}
#[derive(Default)]
struct Cache(Mutex<State>);
/// ShowOff internal/containers.cpp::StrHashCI, hash-only const-char* keys.
fn hash(text: &[u8]) -> u32 {
    let mut h = 0xd39c_a509u32;
    for (i, b) in text.iter().take_while(|&&b| b != 0).enumerate() {
        h = h
            .wrapping_sub(u32::from(b.to_ascii_lowercase()) << [4, 12, 20, 0][i % 4])
            .wrapping_mul(31);
    }
    h
}
fn cstr(bytes: &[u8]) -> &[u8] {
    bytes.split(|&b| b == 0).next().unwrap_or_default()
}
fn normalized(path: &[u8]) -> Vec<u8> {
    cstr(path)
        .iter()
        .map(|&b| if b == b'/' { b'\\' } else { b })
        .collect()
}
pub(crate) fn clear(e: &mut Engine, path: &[u8]) -> bool {
    if cstr(path).is_empty() {
        return false;
    }
    e.ext_free::<Cache>()
        .0
        .lock()
        .unwrap()
        .trees
        .remove(&hash(&normalized(path)))
        .is_some()
}
fn parse(bytes: &[u8]) -> Result<Json, String> {
    // Tao's from_file does not impose serde_json's default 128-level limit.
    let mut parser = serde_json::Deserializer::from_slice(bytes);
    parser.disable_recursion_limit();
    let value = Json::deserialize(&mut parser).map_err(|err| err.to_string())?;
    parser.end().map_err(|err| err.to_string())?;
    Ok(value)
}
fn tree(e: &mut Engine, path: &[u8], cache: bool) -> Result<Json, String> {
    let path = normalized(path);
    let key = hash(&path);
    if let Some(value) = e
        .ext_free::<Cache>()
        .0
        .lock()
        .unwrap()
        .trees
        .get(&key)
        .cloned()
    {
        return Ok(value);
    }
    let bytes = mod_files::read_or_error(e, &path)
        .map_err(|err| format!("File not found or unreadable (error {err})"))?;
    let value = parse(&bytes).map_err(|err| format!("Failed to parse file ({err})"))?;
    if cache {
        let cache = e.ext_free::<Cache>();
        let mut state = cache.0.lock().unwrap();
        return Ok(state.trees.entry(key).or_insert(value).clone());
    }
    Ok(value)
}
/// RFC6901 / Tao pointer.hpp (32-bit source index, excluding UINT32_MAX's
/// std::string::npos sentinel). Object keys are case-sensitive UTF8 strings.
fn pointer<'a>(mut value: &'a Json, path: &[u8]) -> Result<&'a Json, String> {
    if path.is_empty() {
        return Ok(value);
    }
    if path[0] != b'/' {
        return Err("pointer must be empty or begin with '/'".into());
    }
    for part in path[1..].split(|&b| b == b'/') {
        let mut key = Vec::new();
        let mut at = 0;
        while at < part.len() {
            if part[at] == b'~' {
                at += 1;
                key.push(match part.get(at) {
                    Some(b'0') => b'~',
                    Some(b'1') => b'/',
                    _ => return Err("invalid '~' escape".into()),
                });
            } else {
                key.push(part[at]);
            }
            at += 1;
        }
        value = match value {
            Json::Object(map) => std::str::from_utf8(&key).ok().and_then(|s| map.get(s)),
            Json::Array(array) => {
                let index = if key == b"0" {
                    Some(0)
                } else if !key.is_empty() && key[0] != b'0' && key.iter().all(u8::is_ascii_digit) {
                    std::str::from_utf8(&key)
                        .ok()
                        .and_then(|s| s.parse::<u32>().ok())
                        .filter(|&v| v != u32::MAX)
                } else {
                    None
                };
                index.and_then(|i| array.get(i as usize))
            }
            _ => None,
        }
        .ok_or_else(|| "pointer target does not exist or has an invalid array index".to_owned())?;
    }
    Ok(value)
}
/// SO internal/utility.cpp::HexToUInt: first up-to-eight bytes, read backward
/// until the first nonhex byte; unlike JIP's all-suffix converter.
fn hex(bytes: &[u8]) -> u32 {
    let bytes = cstr(bytes);
    let mut out = 0u32;
    let mut shift = 0;
    for &byte in bytes[..bytes.len().min(8)].iter().rev() {
        let byte = byte.to_ascii_lowercase();
        let digit = match byte {
            b'0'..=b'9' => byte - b'0',
            b'a'..=b'f' => byte - b'a' + 10,
            _ => break,
        };
        out = out.wrapping_add(u32::from(digit) << shift);
        shift += 4;
    }
    out
}
fn string_to_ref(e: &mut Engine, text: &[u8]) -> u32 {
    let text = cstr(text);
    let key = hash(text);
    {
        let cache = e.ext_free::<Cache>();
        let mut state = cache.0.lock().unwrap();
        match state.refs.entry(key) {
            std::collections::hash_map::Entry::Occupied(entry) => return *entry.get(),
            std::collections::hash_map::Entry::Vacant(entry) => {
                // Source Insert publishes zero before looking up the mod or
                // resolving the saved ID; a reentrant lookup sees that zero.
                entry.insert(0);
            }
        }
    }
    let id = if let Some(colon) = text.iter().position(|&b| b == b':') {
        let owner = if colon == 0 {
            0xff
        } else {
            let handler = e.mem.u32(0x011c_3f2c);
            let file = nvse::events::with_guest_text(e, &text[..colon], |e, name| {
                e.call(GET_LIST_FILE, &args![handler, name]).u32()
            });
            if file == 0 {
                0xff
            } else {
                e.mem.u8(file + 0x40c)
            }
        };
        if owner == 0xff && colon != 0 {
            0
        } else {
            (u32::from(owner) << 24) | hex(&text[colon + 1..])
        }
    } else {
        nvse::saveload::resolve_ref_id(e, hex(text)).unwrap_or(0)
    };
    e.ext_free::<Cache>().0.lock().unwrap().refs.insert(key, id);
    id
}
/// The source uses UTF8 -> UTF16 -> WideCharToMultiByte(CP_ACP,0,...).
/// Invoke the existing portable import so its ACP/best-fit behavior applies.
fn ansi(e: &mut Engine, text: &str) -> Vec<u8> {
    let text = text.split('\0').next().unwrap_or_default();
    let wide = text
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect::<Vec<_>>();
    e.with_stack((wide.len() * 2) as u32, |e, buf| {
        for (i, c) in wide.iter().enumerate() {
            e.mem.set_u16(buf.addr() + 2 * i as u32, *c);
        }
        let needed = e
            .call(
                WIDE_TO_ANSI,
                &args![0u32, 0u32, buf.addr(), u32::MAX, 0u32, 0u32, 0u32, 0u32],
            )
            .u32();
        if needed == 0 {
            return Vec::new();
        }
        e.with_stack(needed, |e, out| {
            let written = e
                .call(
                    WIDE_TO_ANSI,
                    &args![
                        0u32,
                        0u32,
                        buf.addr(),
                        u32::MAX,
                        out.addr(),
                        needed,
                        0u32,
                        0u32
                    ],
                )
                .u32();
            if written == 0 {
                Vec::new()
            } else {
                e.mem.bytes(out.addr(), written - 1)
            }
        })
    })
}
fn convert(e: &mut Engine, value: &Json, owner: u8) -> Value {
    match value {
        Json::Null => Value::Array(0),
        Json::Bool(v) => Value::Number(f64::from(u8::from(*v))),
        Json::Number(n) => Value::Number(*n),
        Json::String(s) if s.starts_with('@') => {
            let id = string_to_ref(e, &s.as_bytes()[1..]);
            let form = e.call(LOOKUP_FORM, &args![id]).u32();
            Value::Form(if form == 0 { 0 } else { e.mem.u32(form + 0xc) })
        }
        Json::String(s) => Value::Str(ansi(e, s)),
        Json::Array(values) => {
            let id = vars::create_array(e, vars::KEY_NUMBER, true, owner);
            for (i, value) in values.iter().enumerate() {
                let value = convert(e, value, owner);
                vars::set_element(e, id, &Key::Number(i as f64), value);
            }
            Value::Array(id)
        }
        Json::Object(values) => {
            let id = vars::create_array(e, vars::KEY_STRING, false, owner);
            for (key, value) in values {
                let value = convert(e, value, owner);
                vars::set_element(e, id, &Key::Str(cstr(key.as_bytes()).to_vec()), value);
            }
            Value::Array(id)
        }
    }
}
fn read_inner(e: &mut Engine, words: &[u32]) -> Eval {
    let a = CommandArgs::new(words);
    a.set_result(e, 0.0);
    let mut ev = Eval::new(e, &a);
    nvse::expr::with_evaluator(e, |e| {
        let Some(args) = ev.extract_args(e) else {
            return;
        };
        if args.is_empty()
            || args.len() > 4
            || !ev.can_convert(e, &args[0], Ty::String)
            || args
                .get(1)
                .is_some_and(|t| !ev.can_convert(e, t, Ty::String))
            || args
                .get(2)
                .is_some_and(|t| !ev.can_convert(e, t, Ty::Number))
            || args
                .get(3)
                .is_some_and(|t| !ev.can_convert(e, t, Ty::Boolean))
        {
            return;
        }
        let path = ev.get_string(e, &args[0]);
        if cstr(&path).is_empty() {
            return;
        }
        let pointer_path = args.get(1).map_or_else(Vec::new, |t| ev.get_string(e, t));
        let mode = args
            .get(2)
            .map_or(0, |t| nvse::args::f64_to_i32(ev.get_number(e, t)) as u32);
        if matches!(mode, 1 | 2) {
            ev.error_text(format!("ReadFromJSON: parser {mode} is not implemented by nv-rs (only standard JSON parser 0 is supported; JAXN/TaoConfig remain unsupported)"));
            return;
        }
        if mode >= 3 {
            return;
        }
        let cache = args.get(3).is_some_and(|t| ev.get_bool(e, t));
        let tree = match tree(e, &path, cache) {
            Ok(tree) => tree,
            Err(err) => {
                ev.error_text(err);
                return;
            }
        };
        let value = match pointer(&tree, cstr(&pointer_path)) {
            Ok(value) => value,
            Err(err) => {
                ev.error_text(format!("Invalid JSON pointer arg ({err})"));
                return;
            }
        };
        let owner = vars::script_mod_index(e, a.script);
        let token = match convert(e, value, owner) {
            Value::Number(v) => Tok::number(v),
            Value::Form(id) => Tok::Form(id),
            Value::Str(s) => Tok::Str(s),
            Value::Array(id) => Tok::Array(id),
        };
        ev.assign_result(e, &token);
    });
    ev
}
fn read(e: &mut Engine, words: &[u32]) -> Ret {
    let ev = read_inner(e, words);
    nvse::expr::report(e, &ev);
    handled(true)
}
pub(crate) fn install(e: &mut Engine) {
    e.ext_free::<Cache>();
    table::grow_to(e, OPCODE + 1);
    let spec = [
        (0x8004, false),
        (0x8004, true),
        (0x8001, true),
        (0x8002, true),
    ];
    let params = table::alloc(e, 48);
    for (i, (kind, optional)) in spec.into_iter().enumerate() {
        let at = params + 12 * i as u32;
        let name = table::guest_str(
            e,
            match i {
                0 | 1 => b"string",
                2 => b"int",
                _ => b"bool",
            },
        );
        e.mem.set_u32(at, name);
        e.mem.set_u32(at + 4, kind);
        e.mem.set_u32(at + 8, u32::from(optional));
    }
    let execute = table::execute_address(OPCODE);
    e.register(execute, read);
    table::add_plugin_record(
        e,
        OPCODE,
        &PluginRecord {
            name: "ReadFromJSONFile",
            short: "ReadFromJSON",
            needs_ref: false,
            param_count: 4,
            params,
            execute,
            eval: 0,
            flags: 0,
            expression: true,
            returns: Returns::Ambiguous,
        },
    );
}
#[cfg(test)]
mod tests;
