//! `functions_jip/jip_fn_global_var.h` (JIP LN NVSE): a global's value, and a form id kept in a
//! global. All 4 of its commands. The global is read as `kParamType_Global`, whose type check
//! xNVSE's reader makes.
//!
//! A global's value is a float at +0x24 (the global loader `005A64B0` reads `FLTV` there, the type
//! byte `FNAM` at +0x20); JIP's `GetGlobalRef`/`SetGlobalRef` keep a form id in the same word and
//! mark it with bit 0x200 of the form's flags (a bit the game leaves alone in globals), so the id
//! is mapped to today's load order once, the first time it is read after a load
//! (`GetResolvedRefID`: an id of a file no longer loaded stays as it was).

use super::forms::{TESForm, TESGlobal};
use super::{ref_id, Handlers};
use engine_core::prelude::*;
use nvse::commands::Read;
use nvse::{handled, CommandArgs};

#[rustfmt::skip]
pub static COMMANDS: Handlers = &[
    (0x2626, get_global_variable),
    (0x2627, set_global_variable),
    (0x27d5, get_global_ref),
    (0x27d6, set_global_ref),
];

/// JIP's mark on a global holding a resolved form id (`global->flags & 0x200`).
const REF_RESOLVED: u32 = 0x200;

fn global_arg(read: &Read) -> Option<Ptr<TESGlobal>> {
    read.ok.then(|| Ptr::new(read.form(0)))
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_global_var.h:
// Cmd_GetGlobalVariable_Execute.
fn get_global_variable(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if let Some(g) = global_arg(&read) {
        let v = f32::from_bits(e.get(g, TESGlobal::data));
        a.set_result(e, f64::from(v));
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_global_var.h:
// Cmd_SetGlobalVariable_Execute.
fn set_global_variable(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if let Some(g) = global_arg(&read) {
        e.set(g, TESGlobal::data, read.float(1, 0.0).to_bits());
    }
    handled(true)
}

fn form_flags(g: Ptr<TESGlobal>) -> Ptr<TESForm> {
    g.cast()
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_global_var.h:
// Cmd_GetGlobalRef_Execute.
/// The id in the global's word; the first read after a load maps it to today's load order (and
/// marks the global).
fn get_global_ref(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if let Some(g) = global_arg(&read) {
        let flags = e.get(form_flags(g), TESForm::iFormFlags);
        if flags & REF_RESOLVED == 0 {
            e.set(form_flags(g), TESForm::iFormFlags, flags | REF_RESOLVED);
            // `GetResolvedRefID(&uRefID)` rewrites the id's top byte only when its file is
            // still loaded; otherwise the word stays.
            let id = e.get(g, TESGlobal::data);
            let now = crate::cosave::resolved_ref_id(e, id);
            if now != 0 {
                e.set(g, TESGlobal::data, now);
            }
        }
        let id = e.get(g, TESGlobal::data);
        a.set_form_result(e, id);
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_global_var.h:
// Cmd_SetGlobalRef_Execute.
/// The form's id (0 without one) put in the global's word, marked as resolved.
fn set_global_ref(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if let Some(g) = global_arg(&read) {
        let form = read.form(1);
        let id = if form != 0 { ref_id(e, form) } else { 0 };
        e.set(g, TESGlobal::data, id);
        let flags = e.get(form_flags(g), TESForm::iFormFlags);
        e.set(form_flags(g), TESForm::iFormFlags, flags | REF_RESOLVED);
    }
    handled(true)
}

#[cfg(test)]
#[path = "jip_fn_global_var_tests.rs"]
mod tests;
