//! JohnnyGuitar59049a0 fn_utility.cpp459–476, fn_gameplay.cpp270–287,
//! 740–750,949–960; PlayerCharacter/ActorMover/PlayerMover layouts. These
//! selected APIs unblock actual Enhanced Movement paths without plugin claims.
use engine_core::prelude::*;
use nvse::{
    commands::{console_print, double_words, string::assign_to_string_var, Read},
    handled,
    table::{self, PluginRecord, Returns},
    CommandArgs, EvalArgs,
};
const PLAYER: u32 = 0x011d_ea3c;
pub(crate) fn install(e: &mut Engine) {
    table::grow_to(e, 0x320c);
    for (op, name, short, specs, execute, eval, returns, needs_ref) in [
        (
            0x3114,
            "GetEditorID",
            "GetEdID",
            vec![("form", 0x3d, true)],
            editor_id as AbiFn,
            None,
            Returns::String,
            false,
        ),
        (
            0x320b,
            "SetAlwaysRun",
            "",
            vec![("int", 1, false), ("int", 1, true)],
            always_run as AbiFn,
            None,
            Returns::Default,
            false,
        ),
        (
            0x3187,
            "IsCompassHostile",
            "",
            vec![],
            compass as AbiFn,
            Some(compass_eval as AbiFn),
            Returns::Default,
            true,
        ),
    ] {
        let count = specs.len();
        let params = if count == 0 {
            0
        } else {
            table::alloc(e, count as u32 * 12)
        };
        for (i, (name, kind, optional)) in specs.into_iter().enumerate() {
            let at = params + i as u32 * 12;
            let name = table::guest_str(e, name.as_bytes());
            e.mem.set_u32(at, name);
            e.mem.set_u32(at + 4, kind);
            e.mem.set_u32(at + 8, u32::from(optional));
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
                short,
                needs_ref,
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
fn editor_id(e: &mut Engine, words: &[u32]) -> Ret {
    let a = CommandArgs::new(words);
    let r = Read::of(e, &a);
    if r.ok {
        let form = if r.form(0) == 0 {
            a.this_obj
        } else {
            r.form(0)
        };
        let pointer = if form == 0 {
            0
        } else {
            e.vcall(form, 0x130, &[]).u32()
        };
        let text = table::guest_text(e, pointer);
        assign_to_string_var(e, &a, &text);
        if e.call(0x005e_23f0, &[]).bool() {
            console_print(e, "GetEditorID >> %s", &[pointer]);
        }
    }
    handled(true)
}
fn always_run(e: &mut Engine, words: &[u32]) -> Ret {
    let a = CommandArgs::new(words);
    a.set_result(e, 0.0);
    let r = Read::of(e, &a);
    let value = r.int(0, u32::MAX) as i32; // source ignores extraction status and initializes -1.
    if value > -1 {
        let enabled = value > 0;
        let player = e.mem.u32(PLAYER);
        e.mem.set_u8(player + 0x651, u8::from(enabled));
        if r.int(1, 0) != 0 {
            let player = e.mem.u32(PLAYER);
            let mover = e.mem.u32(player + 0x190);
            let flags = (e.mem.u32(mover + 0x94) & !0x200) | if enabled { 0x200 } else { 0 };
            e.vcall(mover, 0xc, &args![flags]);
        }
        a.set_result(e, 1.0);
    }
    handled(true)
}
fn hostile(e: &Engine, target: u32) -> bool {
    let player = e.mem.u32(PLAYER);
    let mut node = e.mem.u32(player + 0xd48);
    while node != 0 && (e.mem.u32(node) != 0 || e.mem.u32(node + 4) != 0) {
        let perceived = e.mem.u32(node);
        if e.mem.u8(perceived + 4) != 0 && e.mem.u32(perceived) == target {
            return true;
        }
        node = e.mem.u32(node + 4);
    }
    false
}
fn compass_eval(e: &mut Engine, words: &[u32]) -> Ret {
    let a = EvalArgs::new(words);
    e.mem.set_f64(a.result, f64::from(hostile(e, a.this_obj)));
    handled(true)
}
fn compass(e: &mut Engine, words: &[u32]) -> Ret {
    let a = CommandArgs::new(words);
    let value = f64::from(hostile(e, a.this_obj));
    a.set_result(e, value);
    if e.call(0x005e_23f0, &[]).bool() {
        console_print(e, "IsCompassHostile >> %.f", &double_words(value));
    }
    handled(true)
}
#[cfg(test)]
mod tests;
