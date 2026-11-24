//! ShowOfff45d069 SO_fn_Items.cpp11–15,146–162,291–369,550–569;
//! GameExtraData.cpp11–56, GameExtraData.h397–478, utility.cpp1726–1763.
//! Real equipped stacks are used; returned refs use xNVSE's actual temporary
//! inventory-reference lifecycle. The C++ unordered_set exposes no order contract;
//! this port retains first encounter order and deduplicates by base pointer.
use engine_core::prelude::*;
use nvse::{
    commands::{console_print, double_words, dynamic_cast, Read},
    handled, invref,
    table::{self, PluginRecord, Returns},
    vars::{self, Key, Value},
    CommandArgs, EvalArgs,
};
use std::collections::HashSet;
const PLAYER: u32 = 0x011d_ea3c;
const WEAPON_VTABLE: u32 = 0x0102_c51c;
pub(crate) fn actor(e: &Engine, reference: u32) -> bool {
    reference != 0 && e.mem.u32(e.mem.u32(reference) + 0x100) == 0x008d_0360
}
pub(crate) fn weapon_info(e: &mut Engine, actor: u32) -> u32 {
    e.vcall(e.mem.u32(actor + 0x68), 0x148, &[]).u32()
}
fn playable(e: &mut Engine, form: u32) -> bool {
    let biped = e.call(0x0048_0db0, &args![form]).u32();
    if biped != 0 {
        e.mem.u32(biped + 8) & 0x40 == 0
    } else if e.mem.u32(form) == WEAPON_VTABLE {
        e.mem.u8(form + 0x100) & 0x80 == 0
    } else {
        true
    }
}
fn slots(e: &mut Engine, form: u32) -> u32 {
    if e.mem.u32(form) == WEAPON_VTABLE {
        32
    } else {
        let biped = e.call(0x0048_0db0, &args![form]).u32();
        if biped == 0 {
            0
        } else {
            e.mem.u32(biped + 4)
        }
    }
}
pub(crate) fn equipped(e: &mut Engine, reference: u32, flags: u32) -> Vec<(u32, u32)> {
    let extra = invref::get_by_type(e, reference + 0x44, 0x15);
    let changes = dynamic_cast(e, extra, 0x0118_3b2c, 0x0118_4bdc);
    if changes == 0 {
        return Vec::new();
    }
    let data = e.mem.u32(changes + 0xc);
    if data == 0 {
        return Vec::new();
    }
    let mut node = e.mem.u32(data);
    let mut seen = HashSet::new();
    let mut result = Vec::new();
    while node != 0 {
        let entry = e.mem.u32(node);
        node = e.mem.u32(node + 4);
        if entry == 0 {
            continue;
        }
        let form = e.mem.u32(entry + 8);
        if form == 0 {
            continue;
        }
        if (flags == 0 || flags & (1 << 20) != 0) && !playable(e, form) {
            continue;
        }
        if flags & (1 << 21) != 0 && e.vcall(form, 0x94, &[]).bool() {
            continue;
        }
        let mask = slots(e, form);
        if flags & (1 << 22) != 0 && mask == 0 {
            continue;
        }
        if mask != 0 && flags != 0 && mask & flags == 0 {
            continue;
        }
        let mut lists = e.mem.u32(entry);
        while lists != 0 {
            let xdata = e.mem.u32(lists);
            lists = e.mem.u32(lists + 4);
            if xdata != 0 && invref::has_type(e, xdata, 0x16) {
                if seen.insert(form) {
                    result.push((form, xdata));
                }
                break;
            }
        }
    }
    result
}
pub(crate) fn install(e: &mut Engine) {
    table::grow_to(e, 0x3d33);
    for (op, name, count, paramspec, execute, eval, returns) in [
        (
            0x3cf7,
            "GetEquippedItemRefs",
            1,
            vec![("int", 1)],
            refs as AbiFn,
            None,
            Returns::Array,
        ),
        (
            0x3d32,
            "GetEquippedWeaponRef",
            0,
            vec![],
            weapon_ref as AbiFn,
            None,
            Returns::Form,
        ),
        (
            0x3cce,
            "GetCalculatedEquippedWeight",
            2,
            vec![("float", 2), ("int", 1)],
            weight as AbiFn,
            Some(weight_eval as AbiFn),
            Returns::Default,
        ),
    ] {
        let params = if count == 0 {
            0
        } else {
            table::alloc(e, count * 12)
        };
        for (i, (name, kind)) in paramspec.into_iter().enumerate() {
            let at = params + i as u32 * 12;
            let name = table::guest_str(e, name.as_bytes());
            e.mem.set_u32(at, name);
            e.mem.set_u32(at + 4, kind);
            e.mem.set_u32(at + 8, 1);
        }
        let address = table::execute_address(op);
        e.register(address, execute);
        let eval = eval.map_or(0, |f| {
            let address = table::eval_address(op);
            e.register(address, f);
            address
        });
        table::add_plugin_record(
            e,
            op,
            &PluginRecord {
                name,
                short: "",
                needs_ref: true,
                param_count: count as u16,
                params,
                execute: address,
                eval,
                flags: 0,
                expression: false,
                returns,
            },
        );
    }
}
fn refs(e: &mut Engine, words: &[u32]) -> Ret {
    let a = CommandArgs::new(words);
    a.set_result(e, 0.0);
    let r = Read::of(e, &a);
    if !r.ok || !actor(e, a.this_obj) {
        return handled(true);
    }
    let items = equipped(e, a.this_obj, r.int(0, 0));
    let owner = if a.script == 0 {
        0xff
    } else {
        e.mem.u8(a.script + 0xf)
    };
    let array = vars::create_array(e, vars::KEY_NUMBER, true, owner);
    for (i, (form, xdata)) in items.into_iter().enumerate() {
        let reference = invref::create_entry(e, a.this_obj, form, 1, xdata);
        vars::set_element(
            e,
            array,
            &Key::Number(i as f64),
            Value::Form(e.mem.u32(reference + 0xc)),
        );
    }
    a.set_result(e, f64::from(array));
    handled(true)
}
fn weapon_ref(e: &mut Engine, words: &[u32]) -> Ret {
    let a = CommandArgs::new(words);
    a.set_result(e, 0.0);
    if !actor(e, a.this_obj) || e.mem.u32(a.this_obj + 0x68) == 0 {
        return handled(true);
    }
    let info = weapon_info(e, a.this_obj);
    if info != 0 {
        let lists = e.mem.u32(info);
        if lists != 0 {
            let reference = invref::create_entry(
                e,
                a.this_obj,
                e.mem.u32(info + 8),
                e.mem.i32(info + 4),
                e.mem.u32(lists),
            );
            if reference != 0 {
                e.mem.set_u32(a.result, e.mem.u32(reference + 0xc));
            }
        }
    }
    handled(true)
}
fn modifier(e: &mut Engine, entry: u32, actor: u32, initial: f32) -> f32 {
    e.with_stack(4, |e, value| {
        e.mem.set_f32(value.addr(), initial);
        e.call(0x005e_58f0, &args![entry, actor, value]);
        e.mem.f32(value.addr())
    })
}
fn calculated_weight(e: &mut Engine, reference: u32, flags: u32, min: f32) -> f32 {
    let player = e.mem.u32(PLAYER);
    let hardcore = u32::from(e.mem.u8(player + 0x7bc));
    if reference == 0 || e.mem.u32(reference + 0x68) == 0 {
        return 0.0;
    }
    let mut total = 0.0f32;
    for (mut item, _) in equipped(e, reference, flags) {
        let mut value = 0.0f32;
        if e.mem.u32(item) == WEAPON_VTABLE {
            let info = weapon_info(e, reference);
            if info == 0 {
                continue;
            }
            item = e.mem.u32(info + 8);
            if item != 0 {
                let lighter = e.call(0x004b_da70, &args![info, 4u32]).bool();
                value = e.call(0x004b_e380, &args![item, lighter]).f64() as f32;
                if value >= 10.0 {
                    let player = e.mem.u32(PLAYER);
                    value *= modifier(e, 73, player, 1.0);
                }
            }
        } else {
            value = e.call(0x0048_ebc0, &args![item, hardcore]).f64() as f32;
        }
        if value > 0.0 {
            if modifier(e, 55, reference, 0.0) > 0.0 {
                let threshold = e.mem.f32(0x011c_647c);
                let factor = e.mem.f32(0x011c_64ac);
                if f64::from(threshold) >= f64::from(value) {
                    value *= factor;
                }
            }
            if value >= min {
                total += value;
            }
        }
    }
    if nvse::commands::is_console_mode(e) {
        console_print(
            e,
            "GetCalculatedEquippedWeight >> %f",
            &double_words(f64::from(total)),
        );
    }
    total
}
fn weight(e: &mut Engine, words: &[u32]) -> Ret {
    let a = CommandArgs::new(words);
    a.set_result(e, 0.0);
    let r = Read::of(e, &a);
    if r.ok {
        let value = calculated_weight(e, a.this_obj, r.int(1, 0), r.float(0, 0.0));
        a.set_result(e, f64::from(value));
    }
    handled(true)
}
fn weight_eval(e: &mut Engine, words: &[u32]) -> Ret {
    let a = EvalArgs::new(words);
    e.mem.set_f64(a.result, 0.0);
    if actor(e, a.this_obj) {
        let value = calculated_weight(e, a.this_obj, a.arg2, f32::from_bits(a.arg1));
        e.mem.set_f64(a.result, f64::from(value));
    }
    handled(true)
}
#[cfg(test)]
pub(crate) mod tests;
