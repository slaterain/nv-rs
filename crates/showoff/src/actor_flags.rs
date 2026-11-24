//! ShowOff f45d069 SO_fn_Gameplay.cpp:248-275 and GameForms.h:
//! GetActorHasBaseFlag/ActorHasBaseFlag. The selected base's stored baseData
//! flags are queried directly; no additional template/leveled-base traversal.
use engine_core::prelude::*;
use nvse::{
    commands::Read,
    handled,
    table::{self, PluginRecord, Returns},
    CommandArgs, EvalArgs,
};
const OPCODE: u32 = 0x3ce0;
const IS_ACTOR_BASE: u32 = 0xf8;
fn flag(e: &mut Engine, this: u32, bit: u32, given: u32) -> f64 {
    if bit > 31 {
        return 0.0;
    }
    let base = if given != 0 {
        given
    } else if this != 0 {
        e.mem.u32(this + 0x20)
    } else {
        0
    };
    if base == 0 || !e.vcall(base, IS_ACTOR_BASE, &[]).bool() {
        return 0.0;
    }
    f64::from((e.mem.u32(base + 0x34) & (1u32 << bit)) != 0)
}
fn execute(e: &mut Engine, words: &[u32]) -> Ret {
    let a = CommandArgs::new(words);
    a.set_result(e, 0.0);
    let r = Read::of(e, &a);
    if r.ok {
        let value = flag(e, a.this_obj, r.int(0, 0), r.form(1));
        a.set_result(e, value);
    }
    handled(true)
}
fn eval(e: &mut Engine, words: &[u32]) -> Ret {
    let a = EvalArgs::new(words);
    e.mem.set_f64(a.result, 0.0);
    let value = flag(e, a.this_obj, a.arg1, a.arg2);
    e.mem.set_f64(a.result, value);
    handled(true)
}
pub(crate) fn install(e: &mut Engine) {
    table::grow_to(e, OPCODE + 1);
    let params = table::alloc(e, 24);
    for (i, (kind, optional, label)) in [(1, false, "int"), (0x19, true, "base actor")]
        .into_iter()
        .enumerate()
    {
        let at = params + 12 * i as u32;
        let name = table::guest_str(e, label.as_bytes());
        e.mem.set_u32(at, name);
        e.mem.set_u32(at + 4, kind);
        e.mem.set_u32(at + 8, u32::from(optional));
    }
    let execute_addr = table::execute_address(OPCODE);
    let eval_addr = table::eval_address(OPCODE);
    e.register(execute_addr, execute);
    e.register(eval_addr, eval);
    table::add_plugin_record(
        e,
        OPCODE,
        &PluginRecord {
            name: "GetActorHasBaseFlag",
            short: "ActorHasBaseFlag",
            needs_ref: false,
            param_count: 2,
            params,
            execute: execute_addr,
            eval: eval_addr,
            flags: 0,
            expression: false,
            returns: Returns::Default,
        },
    );
}
#[cfg(test)]
mod tests;
