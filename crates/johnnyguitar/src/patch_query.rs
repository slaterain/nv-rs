//! GetJohnnyPatch, JG59049a0 fn_utility.cpp:416-456 / JohnnyCommands.cpp ordinal21.
//! Explicit native partial adaptation: flag1 reports the actual shared editor-ID
//! store state, whereas the original DLL always reports true. Flags3-8 report
//! false: their complete hook families are not installed by this partial port.
//! In particular partial JIP serialization fixes do not constitute bFixJIP.
//! This query never advertises a complete plugin version.
use engine_core::prelude::*;
use nvse::{
    commands::Read,
    handled,
    table::{self, PluginRecord, Returns},
    CommandArgs, EvalArgs,
};
const OPCODE: u32 = 0x3115;
const CONSOLE_ECHO: u32 = 0x005e_23f0;
const PRINT_LINE: u32 = 0x0070_3c00;
fn enabled(e: &mut Engine, id: u32) -> f64 {
    f64::from(id == 1 && engine_core::editor_ids::on(e))
}
fn eval(e: &mut Engine, words: &[u32]) -> Ret {
    let args = EvalArgs::new(words);
    let value = enabled(e, args.arg1);
    e.mem.set_f64(args.result, value);
    handled(true)
}
fn execute(e: &mut Engine, words: &[u32]) -> Ret {
    let args = CommandArgs::new(words);
    args.set_result(e, 0.0);
    let read = Read::of(e, &args);
    if read.ok {
        let id = read.int(0, 0);
        let value = enabled(e, id);
        args.set_result(e, value);
        if e.call(CONSOLE_ECHO, &[]).bool() {
            let format = table::guest_str(e, b"GetJohnnyPatch %d >> %d");
            // Preserve the source's PrintLine varargs: *result is a double even
            // though its format says %d. Do not silently repair this source bug.
            e.call(PRINT_LINE, &args![format, id, value]);
        }
    }
    handled(true)
}
pub(crate) fn install(e: &mut Engine) {
    engine_core::editor_ids::store(e);
    table::grow_to(e, OPCODE + 1);
    let params = table::alloc(e, 12);
    let name = table::guest_str(e, b"int");
    e.mem.set_u32(params, name);
    e.mem.set_u32(params + 4, 1);
    e.mem.set_u32(params + 8, 0);
    let execute_addr = table::execute_address(OPCODE);
    let eval_addr = table::eval_address(OPCODE);
    e.register(execute_addr, execute);
    e.register(eval_addr, eval);
    table::add_plugin_record(
        e,
        OPCODE,
        &PluginRecord {
            name: "GetJohnnyPatch",
            short: "",
            needs_ref: false,
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
