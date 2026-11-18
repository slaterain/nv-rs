//! `functions_jip/jip_fn_utility.h` (JIP LN NVSE): `GetOptionalPatch` and `SetOptionalPatch`,
//! JIP's `[GamePatches]` switches asked and turned on or off by name while the game runs, on
//! [`crate::game_patches::switches`] (what each switch does when it changes is there). The rest
//! of the file's commands are in `jip_fn_utility.rs`.

use super::Handlers;
use crate::game_patches::switches;
use engine_core::prelude::*;
use nvse::commands::Read;
use nvse::{handled, CommandArgs};

#[rustfmt::skip]
pub static COMMANDS: Handlers = &[
    (0x27ee, get_optional_patch),
    (0x27ef, set_optional_patch),
];

/// JIP's `patchName[0x40]`.
const NAME_BUFFER: usize = 0x40;

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_utility.h:
// Cmd_GetOptionalPatch_Execute.
/// A switch's name (`jip_nvse.ini`'s key): 1 when it is in, else 0; printed in the console.
fn get_optional_patch(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if read.ok {
        let name = super::buffer_arg(e, &read, 0, NAME_BUFFER, "GetOptionalPatch");
        if switches::is_in(e, switches::number_of(&name)) {
            a.set_result(e, 1.0);
        }
    }
    crate::console::print_result(e, &a);
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_utility.h:
// Cmd_SetOptionalPatch_Execute.
/// A switch's name and 0 or not: the switch turned off or on ([`switches::set`]). No result.
fn set_optional_patch(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if read.ok {
        let name = super::buffer_arg(e, &read, 0, NAME_BUFFER, "SetOptionalPatch");
        let enable = read.int(1, 0) != 0;
        switches::set(e, switches::number_of(&name), enable);
    }
    handled(true)
}

#[cfg(test)]
#[path = "jip_fn_utility_patches_tests.rs"]
mod tests;
