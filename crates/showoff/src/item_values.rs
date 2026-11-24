//! ShowOfff45d069 SO_fn_Items.cpp492–539; utility.cpp1779–1790;
//! decoding.h1853–1871. Native condition/mod-aware value, including source
//! BarterMenu's selected-tile quirk, with real xNVSE inventory-reference data.
use engine_core::prelude::*;
use nvse::{
    commands::Read,
    handled, invref,
    table::{self, PluginRecord, Returns},
    CommandArgs, EvalArgs,
};
const OP: u32 = 0x3d25;
pub(crate) fn install(e: &mut Engine) {
    table::grow_to(e, OP + 1);
    let p = table::alloc(e, 24);
    for (i, (name, kind)) in [("int", 1), ("target item", 0x15)].into_iter().enumerate() {
        let at = p + i as u32 * 12;
        let n = table::guest_str(e, name.as_bytes());
        e.mem.set_u32(at, n);
        e.mem.set_u32(at + 4, kind);
        e.mem.set_u32(at + 8, 1);
    }
    let execute = table::execute_address(OP);
    let eval = table::eval_address(OP);
    e.register(execute, command);
    e.register(eval, condition);
    table::add_plugin_record(
        e,
        OP,
        &PluginRecord {
            name: "GetCalculatedItemValue",
            short: "",
            needs_ref: false,
            param_count: 2,
            params: p,
            execute,
            eval,
            flags: 0,
            expression: false,
            returns: Returns::Default,
        },
    );
}
fn base_form(e: &mut Engine, mut form: u32, reference: u32) -> u32 {
    if form != 0 && e.vcall(form, 0xf0, &[]).bool() {
        let base = e.mem.u32(form + 0x20);
        if base != 0 {
            form = base;
        }
    }
    if form == 0 && reference != 0 {
        form = e.mem.u32(reference + 0x20);
    }
    form
}
fn selected_tile(e: &Engine, menu: u32) -> u32 {
    let list = e.mem.u32(menu + 0x108);
    if list == 0 {
        return 0;
    }
    let selected = e.mem.u32(list + 0x10);
    let mut node = list + 4;
    let mut object = 0;
    while node != 0 {
        let item = e.mem.u32(node);
        if item != 0 && e.mem.u32(item) == selected {
            object = e.mem.u32(item + 4);
            break;
        }
        node = e.mem.u32(node + 4);
    }
    if object == 0 {
        return 0;
    }
    let mut node = list + 4;
    while node != 0 {
        let item = e.mem.u32(node);
        if item != 0 && e.mem.u32(item + 4) == object {
            return e.mem.u32(item);
        }
        node = e.mem.u32(node + 4);
    }
    0
}
fn value(e: &mut Engine, reference: u32, barter: bool, optional: u32) -> f64 {
    let base = base_form(e, optional, reference);
    if base == 0
        || !e
            .call(0x0048_1f30, &args![u32::from(e.mem.u8(base + 4))])
            .bool()
    {
        return -1.0;
    }
    let inventory = if reference == 0 {
        None
    } else {
        let id = e.mem.u32(reference + 0xc);
        if invref::for_ref_id(e, id) {
            invref::get(e, id).map(|r| r.data)
        } else {
            None
        }
    };
    let menu = e.mem.u32(0x011d_8fa4);
    if barter && menu != 0 {
        if let Some(data) = inventory {
            let tile = selected_tile(e, menu);
            return if tile == 0 {
                -1.0
            } else {
                e.call(0x0072_ed00, &args![tile, data.entry]).f64()
            };
        }
    }
    e.with_stack(0x14, |e, frame| {
        let entry = frame.addr();
        e.mem.set_u32(entry, 0);
        e.mem.set_u32(entry + 4, 1);
        e.mem.set_u32(entry + 8, base);
        if let Some(data) = inventory {
            let mut bytes = [0; 12];
            e.mem.read(data.entry, &mut bytes);
            e.mem.write(entry, &bytes);
        } else if reference != 0 {
            e.mem.set_u32(entry + 12, reference + 0x44);
            e.mem.set_u32(entry + 16, 0);
            e.mem.set_u32(entry, entry + 12);
        }
        e.call(0x004b_d400, &args![entry]).f64()
    })
}
fn command(e: &mut Engine, words: &[u32]) -> Ret {
    let a = CommandArgs::new(words);
    a.set_result(e, -1.0);
    let r = Read::of(e, &a);
    if r.ok {
        let answer = value(e, a.this_obj, r.int(0, 0) != 0, r.form(1));
        a.set_result(e, answer);
    }
    handled(true)
}
fn condition(e: &mut Engine, words: &[u32]) -> Ret {
    let a = EvalArgs::new(words);
    let answer = value(e, a.this_obj, a.arg1 != 0, a.arg2);
    e.mem.set_f64(a.result, answer);
    handled(true)
}
#[cfg(test)]
mod tests;
