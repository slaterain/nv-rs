//! JG59049a0 fn_av.cpp:13-31, ActorValueOwner.cpp::GetClampedActorValueF,
//! ActorValue.cpp::GetActorValueName; ordinal167 in JohnnyCommands.cpp.
//! Calls the original clamped-value getter; no replacement stealing formula.
use engine_core::prelude::*;
use nvse::{
    commands::Read,
    handled,
    table::{self, PluginRecord, Returns},
    CommandArgs, EvalArgs,
};
const OPCODE: u32 = 0x31a7;
const IS_ACTOR: u32 = 0x100;
const GET_CLAMPED: u32 = 0x0066_ef50;
const GET_AV_NAME: u32 = 0x0066_e950;
const ECHO: u32 = 0x005e_23f0;
const PRINT: u32 = 0x0070_3c00;
fn value(e: &mut Engine, this: u32, av: u32) -> f64 {
    if this != 0 && e.vcall(this, IS_ACTOR, &[]).bool() {
        f64::from(e.call(GET_CLAMPED, &args![this + 0xa4, av]).f32())
    } else {
        0.0
    }
}
fn eval(e: &mut Engine, words: &[u32]) -> Ret {
    let a = EvalArgs::new(words);
    e.mem.set_f64(a.result, 0.0);
    let v = value(e, a.this_obj, a.arg1);
    e.mem.set_f64(a.result, v);
    handled(true)
}
fn execute(e: &mut Engine, words: &[u32]) -> Ret {
    let a = CommandArgs::new(words);
    a.set_result(e, 0.0);
    let r = Read::of(e, &a);
    // The source relies on needs_ref for nonnull thisObj on execute, and tests
    // IsActor here and again in Eval. Conditions themselves permit null.
    if r.ok && e.vcall(a.this_obj, IS_ACTOR, &[]).bool() {
        let av = r.short(0, u32::MAX);
        let v = value(e, a.this_obj, av);
        a.set_result(e, v);
        if e.call(ECHO, &[]).bool() {
            let name = e.call(GET_AV_NAME, &args![av]).u32();
            let format = table::guest_str(e, b"GetThresholdAV %s >> %.2f");
            e.call(PRINT, &args![format, name, v]);
        }
    }
    handled(true)
}
pub(crate) fn install(e: &mut Engine) {
    table::grow_to(e, OPCODE + 1);
    let params = table::alloc(e, 12);
    let name = table::guest_str(e, b"actor value");
    e.mem.set_u32(params, name);
    e.mem.set_u32(params + 4, 5);
    e.mem.set_u32(params + 8, 0);
    let execute_addr = table::execute_address(OPCODE);
    let eval_addr = table::eval_address(OPCODE);
    e.register(execute_addr, execute);
    e.register(eval_addr, eval);
    table::add_plugin_record(
        e,
        OPCODE,
        &PluginRecord {
            name: "GetThresholdedActorValue",
            short: "GetThresholdAV",
            needs_ref: true,
            param_count: 1,
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
