//! First gameplay commands from JohnnyGuitarNVSE 59049a0, JG/functions/{fn_math,
//! fn_utility,fn_gameplay}.cpp and internal/JG/JohnnyCommands.cpp. Individual commands
//! are available without advertising the still-incomplete plugin as installed.
use engine_core::prelude::*;
use nvse::commands::{arg_in_buffer, console_print, double_words, is_console_mode, Read};
use nvse::table::{self, Param, PluginRecord, Returns};
use nvse::{handled, CommandArgs};

const fn param(name: &'static str, kind: u32, optional: bool) -> Param {
    Param {
        name,
        kind,
        optional,
    }
}
const STRING: &[Param] = &[param("string", 0, false)];
const FLOATS: &[Param] = &[
    param("float", 2, false),
    param("float", 2, false),
    param("float", 2, false),
];
const THEFT: &[Param] = &[param("target", 4, false), param("int", 1, true)];
const FORM: &[Param] = &[param("form", 0x3d, false)];
const ITEM: &[Param] = &[param("item", 3, true)];
const MENU: &[Param] = &[param("int", 1, true)];
const RENDER: &[Param] = &[
    param("setOrRemove", 1, false),
    param("Script", 0x3d, false),
    param("flags", 1, true),
    param("optionalFlags", 1, true),
];

pub fn install(e: &mut Engine) {
    table::grow_to(e, 0x31e1);
    for (op, name, params, needs_ref, returns, execute) in [
        (
            0x3128,
            "EditorIDToFormID",
            STRING,
            false,
            Returns::Form,
            editor_id_to_form_id as AbiFn,
        ),
        (0x3155, "Clamp", FLOATS, false, Returns::Default, clamp),
        (0x3157, "Lerp", FLOATS, false, Returns::Default, lerp),
        (
            0x31e0,
            "GetFactionFlags",
            FORM,
            false,
            Returns::Default,
            get_faction_flags,
        ),
        (
            0x3103,
            "GetBaseEffectAV",
            FORM,
            false,
            Returns::Default,
            crate::gameplay_commands::base_effect_av,
        ),
        (
            0x3104,
            "GetBaseEffectArchetype",
            FORM,
            false,
            Returns::Default,
            crate::gameplay_commands::base_effect_archetype,
        ),
        (
            0x3109,
            "TogglePipBoy",
            MENU,
            false,
            Returns::Default,
            crate::gameplay_commands::toggle_pipboy,
        ),
        (
            0x310a,
            "GetCalculatedWeaponDPS",
            ITEM,
            false,
            Returns::Default,
            crate::weapon_dps::execute,
        ),
        (
            0x319f,
            "SetJohnnyOnRenderUpdateEventHandler",
            RENDER,
            false,
            Returns::Default,
            crate::render::command,
        ),
        (
            0x31a5,
            "IsCrimeOrEnemy",
            &[][..],
            true,
            Returns::Default,
            crate::gameplay_commands::is_crime_or_enemy,
        ),
        (
            0x3199,
            "SendStealingAlarm",
            THEFT,
            true,
            Returns::Default,
            send_stealing_alarm,
        ),
    ] {
        let p = table::alloc(e, 12 * params.len() as u32);
        for (i, a) in params.iter().enumerate() {
            let slot = p + 12 * i as u32;
            let text = table::guest_str(e, a.name.as_bytes());
            e.mem.set_u32(slot, text);
            e.mem.set_u32(slot + 4, a.kind);
            e.mem.set_u32(slot + 8, u32::from(a.optional));
        }
        let address = table::execute_address(op);
        e.register(address, execute);
        table::add_plugin_record(
            e,
            op,
            &PluginRecord {
                name,
                short: if op == 0x319f {
                    "SetOnRenderUpdateEventHandler"
                } else {
                    ""
                },
                needs_ref,
                param_count: params.len() as u16,
                params: p,
                execute: address,
                eval: 0,
                flags: 0,
                expression: false,
                returns,
            },
        );
    }
    crate::render::install(e);
}

fn clamp(e: &mut Engine, words: &[u32]) -> Ret {
    let a = CommandArgs::new(words);
    a.set_result(e, 0.0);
    let r = Read::of(e, &a);
    if r.ok {
        let value = r.float(0, 0.0);
        let min = r.float(1, 0.0);
        let max = r.float(2, 0.0);
        // The original's ordered branches, including reversed limits and NaNs.
        let result = if value < min {
            min
        } else if value > max {
            max
        } else {
            value
        };
        a.set_result(e, f64::from(result));
    }
    handled(true)
}

/// JG59049a0 fn_math.cpp207: three float32 operands and float32 arithmetic,
/// with no clamping of the interpolation value (including extrapolation).
fn lerp(e: &mut Engine, words: &[u32]) -> Ret {
    let a = CommandArgs::new(words);
    a.set_result(e, 0.0);
    let r = Read::of(e, &a);
    if r.ok {
        let start = r.float(0, 0.0);
        let end = r.float(1, 0.0);
        let value = r.float(2, 0.0);
        a.set_result(e, f64::from((1.0f32 - value) * start + value * end));
    }
    handled(true)
}

/// JohnnyGuitar 59049a0 `fn_form.cpp::Cmd_GetFactionFlags_Execute`, registered
/// at ordinal 224 in JohnnyCommands.cpp (opcode 31e0). IS_ID tests the form
/// type byte (+4); TESFaction::kData.uiFlags is unsigned at +34, also read by
/// the owned FalloutNV.exe 1.4.0.525 at 0047d7ca.
fn get_faction_flags(e: &mut Engine, words: &[u32]) -> Ret {
    let a = CommandArgs::new(words);
    a.set_result(e, 0.0);
    let r = Read::of(e, &a);
    let faction = r.form(0);
    if r.ok && faction != 0 && e.mem.u8(faction + 4) == 8 {
        let value = f64::from(e.mem.u32(faction + 0x34));
        a.set_result(e, value);
        // Script::GetConsoleOuput (005e23f0) reads TLS +268 only, without
        // the additional console-open requirement in is_console_mode.
        if e.call(0x005e_23f0, &[]).bool() {
            console_print(e, "GetFactionFlags >> %.f", &double_words(value));
        }
    }
    handled(true)
}

fn editor_id_to_form_id(e: &mut Engine, words: &[u32]) -> Ret {
    let a = CommandArgs::new(words);
    a.set_result(e, 0.0);
    let r = Read::of(e, &a);
    if r.ok {
        let name = arg_in_buffer(e, &r, 0, 260, "EditorIDToFormID");
        if !name.is_empty() {
            // Go through the game's lookup: JG's editor-id row may be disabled.
            let form = e.with_stack(name.len() as u32 + 1, |e, text| {
                e.mem.write(text.addr(), &name);
                e.mem.set_u8(text.addr() + name.len() as u32, 0);
                e.call(0x0048_3a00, &args![text]).u32()
            });
            let id = if form == 0 { 0 } else { e.mem.u32(form + 0xc) };
            e.mem.set_u32(a.result, id);
            if is_console_mode(e) {
                console_print(e, "EditorIDToFormID >> 0x%08X", &[id]);
            }
        }
    }
    handled(true)
}

fn send_stealing_alarm(e: &mut Engine, words: &[u32]) -> Ret {
    let a = CommandArgs::new(words);
    a.set_result(e, 0.0);
    if a.this_obj == 0 || !e.vcall(a.this_obj, 0x100, &[]).bool() {
        return handled(true);
    }
    let r = Read::of(e, &a);
    if !r.ok || r.form(0) == 0 {
        return handled(true);
    }
    let target = r.form(0);
    let owner = e.call(0x0056_7790, &args![target]).u32();
    let mut found = r.int(1, 0) == 0;
    if !found && owner != 0 {
        // JG's InventoryChanges/ItemChange/BSSimpleList layouts, read against the
        // engine's ExtraDataList::GetContainerChanges (00418520) and GetOwner
        // (00418660): REFR's extra list +44, changes' items +0, entry +0/+8.
        let changes = e.call(0x0041_8520, &args![a.this_obj + 0x44]).u32();
        let mut node = if changes == 0 { 0 } else { e.mem.u32(changes) };
        while node != 0 && e.mem.u32(node) != 0 {
            let entry = e.mem.u32(node);
            if e.mem.u32(entry + 8) != 0 {
                let mut extra = e.mem.u32(entry);
                while extra != 0 && e.mem.u32(extra) != 0 {
                    let list = e.mem.u32(extra);
                    if e.call(0x0041_8660, &args![list]).u32() == owner {
                        found = true;
                        break;
                    }
                    extra = e.mem.u32(extra + 4);
                }
            }
            if found {
                break;
            }
            node = e.mem.u32(node + 4);
        }
    }
    if found {
        e.call(
            0x008b_fa40,
            &args![a.this_obj, target, 0u32, 0u32, 1u32, owner],
        );
        a.set_result(e, 1.0);
    }
    handled(true)
}

#[cfg(test)]
mod tests;
