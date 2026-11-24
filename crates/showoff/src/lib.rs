//! Selected ShowOff commands. Translated from f45d069,
//! SO_fn_Files.cpp: Cmd_GetINIFloatOrDefault_Cached_Execute,
//! Cmd_ClearFileCacheShowOff_Execute, IniToNVSE::{GetINIPath,Call_GetOrDefault}.
//! Nothing is advertised as a complete ShowOff plugin; the remaining slots stay absent.
use engine_core::prelude::*;
#[cfg(test)]
use engine_core::simple_ini;
use nvse::{
    commands::Read,
    expr::{Eval, Tok, Ty},
    handled,
    table::{self, PluginRecord, Returns},
    CommandArgs,
};
use std::{collections::HashMap, sync::Mutex};
mod actor_events;
mod actor_flags;
mod aux_map;
mod corner_messages;
mod easing;
mod encumbrance;
mod equipment;
mod form_helpers;
mod ini_write;
mod item_values;
mod json;
mod key_text;
mod on_add;
mod perk_menu;
mod pre_activate;
mod projectile_impact;
mod spell_usage;
mod timers;

type Values = HashMap<(Vec<u8>, Vec<u8>), Vec<u8>>;
#[derive(Default)]
struct Cache(
    Mutex<HashMap<Vec<u8>, Values>>,
    Mutex<HashMap<Vec<u8>, ini_write::Document>>,
);

#[derive(Default)]
struct EventChoice(std::sync::atomic::AtomicBool);

/// Startup-only choice for the original game's ShowOff event hooks. Off by
/// default: command/query helpers alone do not change vanilla activation or
/// projectile behavior. Set before install and before real thread views exist.
pub fn set_events_on(e: &mut Engine, on: bool) {
    e.ext_free::<EventChoice>()
        .0
        .store(on, std::sync::atomic::Ordering::Release);
}
pub fn events_on(e: &Engine) -> bool {
    e.ext_get::<EventChoice>()
        .is_some_and(|choice| choice.0.load(std::sync::atomic::Ordering::Acquire))
}

pub fn install(e: &mut Engine) {
    e.ext_free::<Cache>();
    form_helpers::install(e);
    actor_flags::install(e);
    encumbrance::install(e);
    easing::install(e);
    equipment::install(e);
    item_values::install(e);
    spell_usage::install(e);
    perk_menu::install(e);
    if events_on(e) {
        on_add::install(e);
        actor_events::install(e);
        projectile_impact::install(e);
        pre_activate::install(e);
        corner_messages::install(e);
    }
    aux_map::install(e);
    ini_write::install(e);
    json::install(e);
    timers::install(e);
    key_text::install(e);
    table::grow_to(e, 0x3d2d);
    for (op, name, short, spec, expression, handler) in [
        (
            0x3d10,
            "SetOwnershipTemp",
            "ImGoingToBorrowThis",
            &[(0x3d, true), (0x3d, true)][..],
            false,
            set_ownership_temp as AbiFn,
        ),
        (
            0x3d29,
            "GetINIFloatOrDefault_Cached",
            "GetINIFloat_Cached",
            &[(0x8004, false), (0x8004, true), (0x8001, true)][..],
            true,
            read_ini,
        ),
        (
            0x3d2c,
            "ClearFileCacheShowOff",
            "CloseFileSO",
            &[(0x8004, false), (0x8001, false)][..],
            true,
            clear_cache,
        ),
    ] {
        let params = table::alloc(e, 12 * spec.len() as u32);
        for (i, &(kind, optional)) in spec.iter().enumerate() {
            let at = params + 12 * i as u32;
            let text = table::guest_str(
                e,
                match kind {
                    0x8004 => b"string",
                    0x3d => b"form",
                    _ => b"number",
                },
            );
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
                expression,
                returns: Returns::Default,
            },
        );
    }
}

/// f45d069 GameForms.cpp::TryGetREFRParent: a reference without a base stays
/// unchanged. IsActorAlt applies this once more when validating the owner.
fn reference_parent(e: &mut Engine, form: u32) -> u32 {
    if e.vcall(form, 0xf0, &[]).bool() {
        let base = e.mem.u32(form + 0x20);
        if base != 0 {
            return base;
        }
    }
    form
}

/// f45d069 SO_fn_Gameplay.cpp::Cmd_SetOwnershipTemp_Execute, opcode 3d10.
/// Unlike vanilla SetOwnership/SetCellOwnership, it updates the extra list
/// directly without calling MarkModified (reference 40 / cell 8).
fn set_ownership_temp(e: &mut Engine, words: &[u32]) -> Ret {
    let a = CommandArgs::new(words);
    a.set_result(e, 0.0);
    let player = e.mem.u32(0x011d_ea3c);
    let default_owner = e.mem.u32(player + 0x20);
    let r = Read::of(e, &a);
    if !r.ok {
        return handled(true);
    }
    let owner = if r.args.is_empty() {
        default_owner
    } else {
        r.form(0)
    };
    if owner == 0 {
        return handled(true);
    }
    let owner = reference_parent(e, owner);
    let actor_base = reference_parent(e, owner);
    if !matches!(e.mem.u8(actor_base + 4), 0x2a | 0x2b) && e.mem.u8(owner + 4) != 8 {
        return handled(true);
    }
    let target = r.form(1);
    // Original IS_TYPE tests the exact TESObjectCELL vtable, not its type byte.
    // Cell+28 / reference+44: 004610d0 / 005d43c0 in FalloutNV.exe 1.4.0.525.
    let list = if target != 0 && e.mem.u32(target) == 0x0102_e9b4 {
        target + 0x28
    } else if a.this_obj != 0 {
        a.this_obj + 0x44
    } else {
        return handled(true);
    };
    e.call(0x0041_9700, &args![list, owner]);
    a.set_result(e, 1.0);
    handled(true)
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
    let Some(ini) = ini_write::values(e, &name) else {
        return default;
    };
    number(ini.get(&lookup), default)
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
        let mode = nvse::args::f64_to_i32(ev.get_number(e, &args[1])) as u8;
        let given = ev.get_string(e, &args[0]);
        let removed = match mode {
            0 => relative_name(e, a.script, given).is_some_and(|name| ini_write::clear(e, &name)),
            1 => json::clear(e, &given),
            _ => false,
        };
        a.set_result(e, f64::from(removed));
    });
    handled(true)
}
#[cfg(test)]
engine_core::assemble!();
#[cfg(test)]
mod tests;
