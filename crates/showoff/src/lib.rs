//! ShowOff cached INI-read and cache-invalidation commands. Translated from f45d069,
//! SO_fn_Files.cpp: Cmd_GetINIFloatOrDefault_Cached_Execute,
//! Cmd_ClearFileCacheShowOff_Execute, IniToNVSE::{GetINIPath,Call_GetOrDefault}.
//! Nothing is advertised as a complete ShowOff plugin; the remaining slots stay absent.
use engine_core::{mod_files, prelude::*, simple_ini};
use nvse::{
    expr::{Eval, Tok, Ty},
    handled,
    table::{self, PluginRecord, Returns},
    CommandArgs,
};
use std::{collections::HashMap, sync::Mutex};
type Values = HashMap<(Vec<u8>, Vec<u8>), Vec<u8>>;
#[derive(Default)]
struct Cache(Mutex<HashMap<Vec<u8>, Values>>);

pub fn install(e: &mut Engine) {
    e.ext_free::<Cache>();
    table::grow_to(e, 0x3d2d);
    for (op, name, short, spec, handler) in [
        (
            0x3d29,
            "GetINIFloatOrDefault_Cached",
            "GetINIFloat_Cached",
            &[(0x8004, false), (0x8004, true), (0x8001, true)][..],
            read_ini as AbiFn,
        ),
        (
            0x3d2c,
            "ClearFileCacheShowOff",
            "CloseFileSO",
            &[(0x8004, false), (0x8001, false)][..],
            clear_cache,
        ),
    ] {
        let params = table::alloc(e, 12 * spec.len() as u32);
        for (i, &(kind, optional)) in spec.iter().enumerate() {
            let at = params + 12 * i as u32;
            let text = table::guest_str(e, if kind == 0x8004 { b"string" } else { b"number" });
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
                expression: true,
                returns: Returns::Default,
            },
        );
    }
}
fn with_args(e: &mut Engine, a: &CommandArgs, f: impl FnOnce(&mut Engine, &Eval, Vec<Tok>)) {
    let mut ev = Eval::new(e, a);
    nvse::expr::with_evaluator(e, |e| {
        if let Some(args) = ev.extract_args(e) {
            f(e, &ev, args)
        }
    });
    nvse::expr::report(e, &ev);
}
/// GetOverridingModIdx uses the last of TESForm::mods (xNVSE GameForms.cpp).
/// The game's GetFile(-1), 00484e60, returns that file; its name starts at +20.
fn relative_name(e: &mut Engine, script: u32, mut given: Vec<u8>) -> Option<Vec<u8>> {
    given.truncate(given.iter().position(|&b| b == 0).unwrap_or(given.len()));
    if !given.is_empty() {
        return Some(given);
    }
    if script == 0 {
        return None;
    }
    let file = e.call(0x0048_4e60, &args![script, u32::MAX]).u32();
    if file == 0 || e.mem.u8(file + 0x40c) == 0xff {
        return None;
    }
    let mut name = table::guest_text(e, file + 0x20);
    if name.len() < 3 {
        return None;
    }
    name.truncate(name.len() - 3);
    name.extend_from_slice(b"ini");
    Some(name)
}
fn full_path(name: &[u8]) -> Vec<u8> {
    let mut path = b"data\\config\\".to_vec();
    path.extend(name.iter().map(|&b| if b == b'/' { b'\\' } else { b }));
    if !name.contains(&b'.') {
        path.extend_from_slice(b".ini")
    }
    path
}
/// SimpleIni's GetDoubleValue uses a 64-byte conversion buffer and strtod;
/// empty/missing/overlong values use the default, invalid trailing bytes are ignored.
fn number(value: Option<&Vec<u8>>, default: f64) -> f64 {
    match value {
        Some(v) if !v.is_empty() && v.len() < 64 => nvse::cnum::strtod(v).0,
        _ => default,
    }
}
fn cached_value(e: &mut Engine, name: Vec<u8>, section: &[u8], key: &[u8], default: f64) -> f64 {
    let lookup = (section.to_ascii_lowercase(), key.to_ascii_lowercase());
    let cache = e.ext_free::<Cache>();
    if let Some(ini) = cache.0.lock().unwrap().get(&name.to_ascii_lowercase()) {
        return number(ini.get(&lookup), default);
    }
    let Some(bytes) = mod_files::read(e, &full_path(&name)) else {
        return default;
    };
    let ini = simple_ini::values(&bytes);
    let result = number(ini.get(&lookup), default);
    e.ext_free::<Cache>()
        .0
        .lock()
        .unwrap()
        .insert(name.to_ascii_lowercase(), ini);
    result
}
fn read_ini(e: &mut Engine, words: &[u32]) -> Ret {
    let a = CommandArgs::new(words);
    a.set_result(e, 0.);
    with_args(e, &a, |e, ev, args| {
        if args.is_empty() || args.len() > 3 || !ev.can_convert(e, &args[0], Ty::String) {
            return;
        }
        if args
            .get(1)
            .is_some_and(|t| !ev.can_convert(e, t, Ty::String))
            || args
                .get(2)
                .is_some_and(|t| !ev.can_convert(e, t, Ty::Number))
        {
            return;
        }
        let default = args.get(2).map_or(0., |t| ev.get_number(e, t));
        a.set_result(e, default);
        let value = ev.get_string(e, &args[0]);
        let Some(split) = value.iter().position(|&b| matches!(b, b':' | b'/' | b'\\')) else {
            return;
        };
        if split + 1 == value.len() {
            return;
        }
        let given = args.get(1).map_or_else(Vec::new, |t| ev.get_string(e, t));
        let Some(name) = relative_name(e, a.script, given) else {
            return;
        };
        let result = cached_value(e, name, &value[..split], &value[split + 1..], default);
        a.set_result(e, result);
    });
    handled(true)
}
fn clear_cache(e: &mut Engine, words: &[u32]) -> Ret {
    let a = CommandArgs::new(words);
    a.set_result(e, 0.);
    with_args(e, &a, |e, ev, args| {
        if args.len() != 2
            || !ev.can_convert(e, &args[0], Ty::String)
            || !ev.can_convert(e, &args[1], Ty::Number)
        {
            return;
        }
        // No JSON-reading command has been implemented, so no JSON cache can exist yet.
        if nvse::args::f64_to_i32(ev.get_number(e, &args[1])) as u8 != 0 {
            return;
        }
        let given = ev.get_string(e, &args[0]);
        if let Some(name) = relative_name(e, a.script, given) {
            let removed = e
                .ext_free::<Cache>()
                .0
                .lock()
                .unwrap()
                .remove(&name.to_ascii_lowercase())
                .is_some();
            a.set_result(e, f64::from(removed));
        }
    });
    handled(true)
}
#[cfg(test)]
engine_core::assemble!();
#[cfg(test)]
mod tests;
