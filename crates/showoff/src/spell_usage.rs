//! ShowOfff45d069 SO_fn_Items.cpp606–645 and GameEffects.h:
//! usage-monitor effects are matched by both EffectItem and adjusted MagicItem
//! pointer; terminated/applied flags are deliberately not additional filters.
use engine_core::prelude::*;
use nvse::{
    commands::Read,
    handled,
    table::{self, PluginRecord, Returns},
    CommandArgs, EvalArgs,
};
const OP: u32 = 0x3d69;
pub(crate) fn install(e: &mut Engine) {
    table::grow_to(e, OP + 1);
    let p = table::alloc(e, 12);
    let name = table::guest_str(e, b"Magic Item");
    e.mem.set_u32(p, name);
    e.mem.set_u32(p + 4, 0x0b);
    e.mem.set_u32(p + 8, 0);
    let execute = table::execute_address(OP);
    let eval = table::eval_address(OP);
    e.register(execute, command);
    e.register(eval, condition);
    table::add_plugin_record(
        e,
        OP,
        &PluginRecord {
            name: "GetSpellUsageNumEx",
            short: "",
            needs_ref: true,
            param_count: 1,
            params: p,
            execute,
            eval,
            flags: 0,
            expression: false,
            returns: Returns::Default,
        },
    );
}
fn usage(e: &mut Engine, actor: u32, magic: u32) -> f64 {
    if actor == 0 || !e.vcall(actor, 0x100, &[]).bool() {
        return 0.0;
    }
    let monitor = e.call(0x0040_6200, &args![magic + 0xc]).u32();
    if monitor == 0 {
        return 0.0;
    }
    let mut node = e.vcall(actor + 0x94, 8, &[]).u32();
    let mut magnitude = 0.0f32;
    while node != 0 {
        let active = e.mem.u32(node);
        node = e.mem.u32(node + 4);
        if active != 0 && e.mem.u32(active + 0xc) == monitor && e.mem.u32(active + 8) == magic {
            magnitude += e.mem.f32(active + 0x1c);
        }
    }
    f64::from(magnitude)
}
fn command(e: &mut Engine, words: &[u32]) -> Ret {
    let a = CommandArgs::new(words);
    a.set_result(e, 0.0);
    let r = Read::of(e, &a);
    if r.ok {
        let value = usage(e, a.this_obj, r.form(0));
        a.set_result(e, value);
    }
    handled(true)
}
fn condition(e: &mut Engine, words: &[u32]) -> Ret {
    let a = EvalArgs::new(words);
    let value = usage(e, a.this_obj, a.arg1);
    e.mem.set_f64(a.result, value);
    handled(true)
}
#[cfg(test)]
mod tests;
