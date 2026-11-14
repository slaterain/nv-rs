//! `functions_jip/jip_fn_misc_ref.h` (JIP LN NVSE): references' interaction state. 2 of its 104
//! commands: the getters of the player's activation a script disabled, which the core track's
//! `SetInteractionDisabled` and `SetInteractionDisabledType` set (`world_hooks.rs`: the form's
//! `jipFormFlags6` bit, the form types in `ui_hooks.rs`' `activation_disabled_types`).
//!
//! The rest wait for their own batches: most read or change a reference's scene graph and
//! physics (JIP's node lookup `GetNode2`, `NiAVObject` blocks, Havok bodies).

use super::Handlers;
use crate::game::JipFormFlags;
use crate::Jip;
use engine_core::prelude::*;
use nvse::commands::Read;
use nvse::{handled, CommandArgs};

#[rustfmt::skip]
pub static COMMANDS: Handlers = &[
    (0x27c8, get_interaction_disabled),
    (0x27ca, get_interaction_disabled_type),
];

/// `kHookFormFlag6_NoPCActivation` (`internal/hooks.h` (JIP LN NVSE), the bit `world_hooks.rs`
/// sets).
const NO_PC_ACTIVATION: u8 = 1 << 1;

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_misc_ref.h:
// Cmd_GetInteractionDisabled_Execute.
/// 1 when the form (or the calling reference) is one the player may not activate.
fn get_interaction_disabled(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if !read.ok {
        return handled(true);
    }
    let mut form = read.form(0);
    if form == 0 {
        form = a.this_obj;
    }
    if form != 0
        && e.get(Ptr::<JipFormFlags>::new(form), JipFormFlags::jipFormFlags6) & NO_PC_ACTIVATION
            != 0
    {
        a.set_result(e, 1.0);
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_misc_ref.h:
// Cmd_GetInteractionDisabledType_Execute.
/// 1 when the form type (below `kFormType_Max`, one of the filter types) is one the player may
/// not activate.
fn get_interaction_disabled_type(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let kind = read.int(0, 0) as usize;
    if read.ok
        && crate::ln_events::VALID_FILTER_FORM
            .get(kind)
            .copied()
            .unwrap_or(false)
        && e.ext::<Jip>()
            .ui_hooks
            .activation_disabled_types
            .get(kind)
            .copied()
            .unwrap_or(false)
    {
        a.set_result(e, 1.0);
    }
    handled(true)
}

#[cfg(test)]
#[path = "jip_fn_misc_ref_tests.rs"]
mod tests;
