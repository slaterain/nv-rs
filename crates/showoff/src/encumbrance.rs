//! ShowOfff45d069 SO_fn_Actors.cpp463–482 / SO_fn_Actors.h22;
//! ShowOffNVSE.cpp214,645 and GameObjects.h767. PipBoy UI Tweaks invokes
//! IsPCOverencumbered before its requirement gates, then on inventory changes.
use engine_core::prelude::*;
use nvse::{
    commands::Read,
    handled,
    table::{self, PluginRecord, Returns},
    CommandArgs, EvalArgs,
};
const OPCODE: u32 = 0x3d3c;
const PLAYER: u32 = 0x011d_ea3c;
pub(crate) fn install(e: &mut Engine) {
    table::grow_to(e, OPCODE + 1);
    let params = table::alloc(e, 12);
    let name = table::guest_str(e, b"int");
    e.mem.set_u32(params, name);
    e.mem.set_u32(params + 4, 1);
    e.mem.set_u32(params + 8, 1);
    let execute = table::execute_address(OPCODE);
    let eval = table::eval_address(OPCODE);
    e.register(execute, command);
    e.register(eval, condition);
    table::add_plugin_record(
        e,
        OPCODE,
        &PluginRecord {
            name: "GetIsPlayerOverencumbered",
            short: "IsPCOverencumbered",
            needs_ref: false,
            param_count: 1,
            params,
            execute,
            eval,
            flags: 0,
            expression: false,
            returns: Returns::Default,
        },
    );
}
fn overencumbered(e: &mut Engine, ignore_god_mode: bool) -> bool {
    if !ignore_god_mode && e.call(0x0095_26b0, &[]).bool() {
        return false;
    }
    // Source re-fetches the singleton separately for these calls.
    let player = e.mem.u32(PLAYER);
    let weight = e.vcall(player + 0xa4, 8, &args![0x2eu32]).i32();
    let player = e.mem.u32(PLAYER);
    let max = e.call(0x008a_0c20, &args![player]).f32();
    // GetMaxCarryWeight returns float; usual C++ conversions compare weight
    // as f32 too (large signed weights can round), with NaN/equality false.
    max < weight as f32
}
fn command(e: &mut Engine, words: &[u32]) -> Ret {
    let a = CommandArgs::new(words);
    let r = Read::of(e, &a);
    let value = overencumbered(e, r.int(0, 0) != 0);
    a.set_result(e, f64::from(value));
    handled(true)
}
fn condition(e: &mut Engine, words: &[u32]) -> Ret {
    let a = EvalArgs::new(words);
    let value = overencumbered(e, a.arg1 != 0);
    e.mem.set_f64(a.result, f64::from(value));
    handled(true)
}
#[cfg(test)]
mod tests;
