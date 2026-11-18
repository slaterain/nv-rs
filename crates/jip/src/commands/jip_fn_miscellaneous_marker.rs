//! `functions_jip/jip_fn_miscellaneous.h` (JIP LN NVSE): `SetInternalMarker`, a form asked about,
//! put in or taken out of JIP's set of forms the game takes for markers (`s_internalMarkerIDs`),
//! which JIP's `IsMarker` (H22, `crate::game_patches::gf`, in whenever JIP answers "installed")
//! reads: a marker is not drawn or shown in the game. The rest of the file's commands are in
//! `jip_fn_miscellaneous.rs` and `jip_fn_miscellaneous_scene.rs`.
//!
//! The set is JIP's ids at the start (35, `InitContainers`) and lives for the run (JIP keeps it
//! in memory, not in the co-save). Threads: changed from the script's thread under the set's
//! write lock; `IsMarker` reads it from any thread.

use super::Handlers;
use crate::game_patches::gf::InternalMarkers;
use engine_core::prelude::*;
use nvse::commands::Read;
use nvse::{handled, CommandArgs};

#[rustfmt::skip]
pub static COMMANDS: Handlers = &[
    (0x2902, set_internal_marker),
];

/// A form's vtable slot `IsReference` (+0xF0) and its true answer (`008D0360`); a reference's
/// base form (+0x20); a form's id (+0xC).
const VT_IS_REFERENCE: u32 = 0xf0;
const RETURN_TRUE: u32 = 0x008d_0360;
const BASE_FORM: u32 = 0x20;
const FORM_ID: u32 = 0x0c;

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_miscellaneous.h:
// Cmd_SetInternalMarker_Execute.
/// A form (a reference: its base form) and, optionally, 1 or 0: with no number (or one below 0)
/// 1 when the form's id is in JIP's marker set; else the id put in (not 0) or taken out (0).
fn set_internal_marker(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if !read.ok {
        return handled(true);
    }
    let mut form = read.form(0);
    if form == 0 {
        return handled(true);
    }
    let toggle = read.int(1, u32::MAX) as i32;
    if e.mem.u32(e.mem.u32(form).wrapping_add(VT_IS_REFERENCE)) == RETURN_TRUE {
        form = e.mem.u32(form.wrapping_add(BASE_FORM));
        if form == 0 {
            return handled(true);
        }
    }
    let id = e.mem.u32(form.wrapping_add(FORM_ID));
    let markers = e.ext_free::<InternalMarkers>();
    if toggle < 0 {
        if markers.has(id) {
            a.set_result(e, 1.0);
        }
    } else {
        markers.set(id, toggle != 0);
    }
    handled(true)
}

#[cfg(test)]
#[path = "jip_fn_miscellaneous_marker_tests.rs"]
mod tests;
