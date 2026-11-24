//! JohnnyGuitar59049a0 fn_form.cpp1117–1148 / BGSPerk.cpp and
//! fn_utility.cpp290–302. UI Improvements' available-perk array and Pip-Boy state.
use engine_core::prelude::*;
use nvse::{
    commands::{console_print, double_words},
    handled,
    table::{self, PluginRecord, Returns},
    vars::{self, Key, Value},
    CommandArgs, EvalArgs,
};
const PLAYER: u32 = 0x011d_ea3c;
const HANDLER: u32 = 0x011c_3f2c;
const INTERFACE: u32 = 0x011d_8a80;
pub(crate) fn install(e: &mut Engine) {
    table::grow_to(e, 0x31a7);
    for (op, name, execute, eval, returns) in [
        (
            0x3168,
            "GetPipBoyMode",
            pipboy as AbiFn,
            Some(pipboy_eval as AbiFn),
            Returns::Default,
        ),
        (
            0x31a6,
            "GetAvailablePerks",
            available as AbiFn,
            None,
            Returns::Array,
        ),
    ] {
        let address = table::execute_address(op);
        e.register(address, execute);
        let eval_address = eval.map_or(0, |f| {
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
                needs_ref: false,
                param_count: 0,
                params: 0,
                execute: address,
                eval: eval_address,
                flags: 0,
                expression: false,
                returns,
            },
        );
    }
}
fn mode(e: &Engine) -> f64 {
    let manager = e.mem.u32(INTERFACE);
    if manager == 0 {
        0.0
    } else {
        f64::from(e.mem.u32(manager + 0x4bc))
    }
}
fn pipboy_eval(e: &mut Engine, words: &[u32]) -> Ret {
    let a = EvalArgs::new(words);
    e.mem.set_f64(a.result, mode(e));
    handled(true)
}
fn pipboy(e: &mut Engine, words: &[u32]) -> Ret {
    let a = CommandArgs::new(words);
    let value = mode(e);
    a.set_result(e, value);
    if e.call(0x005e_23f0, &[]).bool() {
        console_print(e, "GetPipBoyMode >> %.2f", &double_words(value));
    }
    handled(true)
}
fn available(e: &mut Engine, words: &[u32]) -> Ret {
    let a = CommandArgs::new(words);
    a.set_result(e, 0.0);
    let mut actor = e.mem.u32(PLAYER);
    if a.this_obj != 0 && e.vcall(a.this_obj, 0x100, &[]).bool() {
        actor = a.this_obj;
    }
    let owner = if a.script == 0 {
        0xff
    } else {
        e.mem.u8(a.script + 0xf)
    };
    let array = vars::create_array(e, vars::KEY_NUMBER, true, owner);
    let mut index = 0;
    if actor != 0 {
        // Actor inherits ActorValueOwner at+a4. This source GetActorLevel is
        // its virtual+28, not Actor::GetLevel87F9F0.
        let level = i32::from(e.vcall(actor + 0xa4, 0x28, &[]).u16());
        let mut node = e.mem.u32(HANDLER) + 0x178;
        while node != 0 && (e.mem.u32(node) != 0 || e.mem.u32(node + 4) != 0) {
            let perk = e.mem.u32(node);
            node = e.mem.u32(node + 4);
            // BGSPerk.cpp wrappers directly read signed level/ranks/trait.
            if e.mem.i8(perk + 0x39) <= 0 {
                continue;
            }
            let rank = e.vcall(actor, 0x4a0, &args![perk, 0u32]).u8();
            if rank < e.mem.u8(perk + 0x3a)
                && e.mem.u8(perk + 0x38) == 0
                && e.call(0x005e_ba00, &args![perk, actor]).bool()
                && e.call(0x005e_b5e0, &args![perk]).bool()
                && e.call(0x0078_5150, &args![perk, actor]).bool()
                && i32::from(e.mem.i8(perk + 0x39)) <= level
            {
                vars::set_element(
                    e,
                    array,
                    &Key::Number(f64::from(index)),
                    Value::Form(e.mem.u32(perk + 0xc)),
                );
                index += 1;
            }
        }
    }
    a.set_result(e, f64::from(array));
    handled(true)
}
#[cfg(test)]
mod tests;
