//! `functions_jip/jip_fn_faction.h` (JIP LN NVSE): a faction's reputation. All 2 of its commands.
//! The faction and the reputation are read as `kParamType_Faction` and `kParamType_Reputation`,
//! whose type checks xNVSE's reader makes. The reputation is the word at +0x38 (the faction loader
//! `005FCB70` stores the record's reputation id there).

use super::forms::TESFaction;
use super::{set_form_result, Handlers};
use engine_core::prelude::*;
use nvse::commands::Read;
use nvse::{handled, CommandArgs};

#[rustfmt::skip]
pub static COMMANDS: Handlers = &[
    (0x2628, get_faction_reputation_type),
    (0x2629, set_faction_reputation_type),
];

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_faction.h:
// Cmd_GetFactionReputationType_Execute.
fn get_faction_reputation_type(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if read.ok {
        let f = Ptr::<TESFaction>::new(read.form(0));
        let reputation = e.get(f, TESFaction::reputation);
        if reputation != 0 {
            set_form_result(e, &a, reputation);
        }
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_faction.h:
// Cmd_SetFactionReputationType_Execute.
/// The reputation set, or cleared without one.
fn set_faction_reputation_type(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if read.ok {
        let f = Ptr::<TESFaction>::new(read.form(0));
        e.set(f, TESFaction::reputation, read.form(1));
    }
    handled(true)
}

#[cfg(test)]
#[path = "jip_fn_faction_tests.rs"]
mod tests;
