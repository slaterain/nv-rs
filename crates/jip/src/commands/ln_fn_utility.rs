//! `functions_ln/ln_fn_utility.h` (JIP LN NVSE): JIP's version, a form list as an array, sorting
//! and counting forms by type, and the time of day as text. 5 of its 20 commands.
//!
//! Left for later, each for its reason: `FileExists`, `GetINIFloat`, `SetINIFloat`,
//! `GetINIString`, `SetINIString`, `GetINISection`, `SetINISection`, `GetINISectionNames`,
//! `RemoveINIKey`, `RemoveINISection`, `GetFilesInFolder`, `GetFoldersInFolder` read and write the
//! player's files through Windows' profile and directory calls (the platform keeps every file the
//! game writes in memory, so an INI a mod writes would be gone next session, and it has no
//! `GetPrivateProfileSectionA`, `WritePrivateProfileSectionA`, `GetPrivateProfileSectionNamesA`
//! or `WritePrivateProfileStructA`: one file seam for all of them, with `jip_fn_utility`'s file
//! commands); `Console` compiles and runs a script line (`JIPScriptRunner`, not built yet);
//! `GetDefaultMessageTime`/`SetDefaultMessageTime` hold the time JIP's `QueueUIMessageHook` gives
//! messages (the hook is the core track's; without it the setting would change nothing).

use super::arrays::{create_array, Element};
use super::forms::ListNode;
use super::jip_fn_aux_variable::array_values;
use super::{list_items, type_of, Handlers};
use crate::cosave::lookup_form;
use engine_core::prelude::*;
use nvse::commands::Read;
use nvse::vars::{Container, Data};
use nvse::{handled, CommandArgs};

#[rustfmt::skip]
pub static COMMANDS: Handlers = &[
    (0x2200, get_ln_version),
    (0x2202, get_time_stamp),
    (0x2212, list_to_array),
    (0x22ff, sort_forms_by_type),
    (0x2300, get_form_count_type),
];

/// `JIP_LN_VERSION` as the shipped DLL holds it: 57.30, the double `0x404CA66666666666`.
pub const JIP_LN_VERSION: f64 = f64::from_bits(0x404c_a666_6666_6666);
/// A form list's forms (`BGSListForm::list`, a `tList` at +0x18).
const LIST_FORMS: u32 = 0x18;
/// The game's import slot of `GetLocalTime` (`00FDF0D8`; the save code calls it there).
const GET_LOCAL_TIME_IMPORT: u32 = 0x00fd_f0d8;
/// `IS_REFERENCE`: the vtable answers true (`008D0360`) at +0xF0.
const RETURN_TRUE: u32 = 0x008d_0360;

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_utility.h:
// Cmd_GetLNVersion_Execute.
/// 57.30, echoed in the console.
fn get_ln_version(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    a.set_result(e, JIP_LN_VERSION);
    crate::console::print_result(e, &a);
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_utility.h:
// Cmd_GetTimeStamp_Execute; internal/utility.cpp: GetTimeStamp.
/// The local time as `HH-MM-SS` (JIP's `localtime_s` of the clock: the hour, minute and second
/// `GetLocalTime` gives), printed in the console when it is open (`IsConsoleOpen`) and given as
/// the string result.
fn get_time_stamp(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let (h, m, s) = e.with_stack(16, |e, st| {
        e.call(GET_LOCAL_TIME_IMPORT, &args![st]);
        let at = st.addr();
        (e.mem.u16(at + 8), e.mem.u16(at + 10), e.mem.u16(at + 12))
    });
    let text = format!("{h:02}-{m:02}-{s:02}").into_bytes();
    if nvse::commands::is_console_mode(e) {
        crate::console::console_print_line(e, &text);
    }
    super::assign_string(e, &a, Some(text));
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_utility.h:
// Cmd_ListToArray_Execute.
/// The list's forms in order (empty nodes left out) as a new array.
fn list_to_array(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if read.ok {
        let list = read.form(0);
        let forms: Vec<Element> = list_items(e, Ptr::<ListNode>::new(list + LIST_FORMS))
            .into_iter()
            .filter(|&f| f != 0)
            .map(Element::Form)
            .collect();
        let id = create_array(e, a.script, &forms);
        a.set_result(e, f64::from(id as i32));
    }
    handled(true)
}

/// `GetElements` of a plugin: each value of the array in its order, a form looked up (0 when
/// gone), a number, else nothing a form or number reader takes.
fn plugin_forms(e: &Engine, values: &[Data]) -> Vec<u32> {
    values
        .iter()
        .map(|d| match d {
            Data::Form(id) => lookup_form(e, *id),
            _ => 0,
        })
        .collect()
}

fn plugin_number(d: &Data) -> f64 {
    match d {
        Data::Number(n) => *n,
        _ => 0.0,
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_utility.h:
// Cmd_SortFormsByType_Execute.
/// The forms of the first array (non-null, in order), grouped by the types of the second: for
/// each type (its number as a byte), the forms of that type (a reference also by its base
/// form's type) taken from the end of what is left back to the start; then what is left in
/// order. Nothing when either array is missing or empty.
fn sort_forms_by_type(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if !read.ok {
        return handled(true);
    }
    let (Some(forms), Some(types)) = (
        array_values(e, read.int(0, 0)),
        array_values(e, read.int(1, 0)),
    ) else {
        return handled(true);
    };
    if forms.is_empty() || types.is_empty() {
        return handled(true);
    }
    let mut left: Vec<u32> = plugin_forms(e, &forms)
        .into_iter()
        .filter(|&f| f != 0)
        .collect();
    let mut out = Vec::new();
    for t in &types {
        let kind = super::cvttsd2si(plugin_number(t)) as u8;
        let mut i = left.len();
        while i > 0 {
            i -= 1;
            let f = left[i];
            let matches = type_of(e, f) == kind
                || (e.mem.u32(e.mem.u32(f) + 0xf0) == RETURN_TRUE
                    && type_of(e, e.mem.u32(f + 0x20)) == kind);
            if matches {
                out.push(Element::Form(f));
                left.remove(i);
            }
        }
        if left.is_empty() {
            break;
        }
    }
    out.extend(left.into_iter().map(Element::Form));
    let id = create_array(e, a.script, &out);
    a.set_result(e, f64::from(id as i32));
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_utility.h:
// Cmd_GetFormCountType_Execute.
/// How many forms of the type a packed array holds (a map, a missing or an empty array:
/// nothing).
fn get_form_count_type(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if !read.ok {
        return handled(true);
    }
    let (id, kind) = (read.int(0, 0), read.int(1, 0));
    let packed = nvse::vars::array(e, id).is_some_and(|v| matches!(v.elements, Container::Array(_)));
    let Some(values) = array_values(e, id).filter(|v| packed && !v.is_empty()) else {
        return handled(true);
    };
    let count = plugin_forms(e, &values)
        .into_iter()
        .filter(|&f| f != 0 && u32::from(type_of(e, f)) == kind)
        .count();
    a.set_result(e, count as f64);
    handled(true)
}

#[cfg(test)]
#[path = "ln_fn_utility_tests.rs"]
mod tests;
