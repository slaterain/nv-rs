//! ShowOff f45d069 SO_fn_Debug.cpp:58-66 (`IsBaseForm`),
//! SO_fn_Array.cpp:13-39,60-74 (`ListAddArray`) and nvse/GameForms.h::BGSListForm::AddAt.
use engine_core::prelude::*;
use nvse::{
    commands::{dynamic_cast, misc_forms::add_at, rtti, Read},
    expr::{Eval, Ty},
    handled,
    table::{self, PluginRecord, Returns},
    vars::{self, Container, Data},
    CommandArgs,
};

const RETURN_TRUE: u32 = 0x008d_0360;
const LOOKUP_FORM: u32 = 0x0048_39c0;
fn is_base(e: &mut Engine, words: &[u32]) -> Ret {
    let a = CommandArgs::new(words);
    a.set_result(e, 0.0);
    let read = Read::of(e, &a);
    if read.ok {
        let form = read.form(0);
        // IS_REFERENCE compares the virtual function address; it does not call
        // the function, inspect a type-byte range, or replace refs with bases.
        if form != 0 && e.mem.u32(e.mem.u32(form) + 0xf0) != RETURN_TRUE {
            a.set_result(e, 1.0);
        }
    }
    handled(true)
}
fn list_add_at(e: &mut Engine, list: u32, form: u32, index: i32) -> i32 {
    let result = add_at(e, list + 0x18, form, index);
    // Source IsAddedObject takes UInt32. Test the requested index, not the
    // returned index, and do not apply xNVSE AddAtTemp's max/duplicate policy.
    let added = e.mem.u32(list + 0x20);
    if result >= 0 && (index as u32) < added {
        e.mem.set_u32(list + 0x20, added.wrapping_add(1));
    }
    result
}
fn list_add_array(e: &mut Engine, words: &[u32]) -> Ret {
    let a = CommandArgs::new(words);
    a.set_result(e, 1.0);
    let mut ev = Eval::new(e, &a);
    nvse::expr::with_evaluator(e, |e| {
        let Some(args) = ev.extract_args(e) else {
            return;
        };
        if !(2..=3).contains(&args.len())
            || !ev.can_convert(e, &args[0], Ty::Form)
            || !ev.can_convert(e, &args[1], Ty::Array)
            || args
                .get(2)
                .is_some_and(|arg| !ev.can_convert(e, arg, Ty::Number))
        {
            return;
        }
        let form = ev.get_form(e, &args[0]);
        let list = dynamic_cast(e, form, rtti::TES_FORM, rtti::BGS_LIST_FORM);
        let id = ev.get_array_id(e, &args[1]);
        let index = args
            .get(2)
            .map_or(-2, |arg| nvse::args::f64_to_i32(ev.get_number(e, arg)));
        let Some(array) = vars::array(e, id) else {
            return;
        };
        if list == 0 {
            return;
        }
        // GetElements resolves every Form to a pointer before any insertion.
        let values = match &array.elements {
            Container::Array(v) => v.iter().map(|e| e.data.clone()).collect::<Vec<_>>(),
            Container::NumMap(v) => v.iter().map(|(_, e)| e.data.clone()).collect(),
            Container::StrMap(v) => v.iter().map(|(_, e)| e.data.clone()).collect(),
        };
        let mut forms = values
            .into_iter()
            .filter_map(|data| {
                if let Data::Form(id) = data {
                    let ptr = e.call(LOOKUP_FORM, &args![id]).u32();
                    (ptr != 0).then_some(ptr)
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();
        if index != -2 {
            forms.reverse();
        }
        for form in forms {
            if list_add_at(e, list, form, index) == -1 {
                a.set_result(e, 0.0);
                break;
            }
        }
    });
    nvse::expr::report(e, &ev);
    handled(true)
}
pub(crate) fn install(e: &mut Engine) {
    table::grow_to(e, 0x3d1d);
    for (op, name, short, spec, expression, handler) in [
        (
            0x3cd4,
            "IsBaseForm",
            "IsBasedForm",
            &[(0x3d, false)][..],
            false,
            is_base as AbiFn,
        ),
        (
            0x3d1c,
            "ListAddArray",
            "AddArrayToFormList",
            &[(0x8008, false), (0x8040, false), (0x8001, true)][..],
            true,
            list_add_array,
        ),
    ] {
        let params = table::alloc(e, 12 * spec.len() as u32);
        for (i, &(kind, optional)) in spec.iter().enumerate() {
            let at = params + 12 * i as u32;
            let text = table::guest_str(
                e,
                if kind == 0x8040 {
                    b"array"
                } else if kind == 0x8001 {
                    b"index"
                } else {
                    b"form"
                },
            );
            e.mem.set_u32(at, text);
            e.mem.set_u32(at + 4, kind);
            e.mem.set_u32(at + 8, u32::from(optional));
        }
        let execute = table::execute_address(op);
        e.register(execute, handler);
        table::add_plugin_record(
            e,
            op,
            &PluginRecord {
                name,
                short,
                needs_ref: false,
                param_count: spec.len() as u16,
                params,
                execute,
                eval: 0,
                flags: 0,
                expression,
                returns: Returns::Default,
            },
        );
    }
}
#[cfg(test)]
mod tests;
