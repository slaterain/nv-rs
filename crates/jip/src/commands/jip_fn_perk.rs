//! `functions_jip/jip_fn_perk.h` (JIP LN NVSE): a perk's trait, playable and hidden flags. All 2
//! of its commands. The perk is read as `kParamType_Perk`, whose type check xNVSE's reader
//! makes. The flags are the bools of the perk's `DATA` (the perk loader `005EADA0` reads its five
//! bytes at +0x38: trait, minimum level, ranks, playable, hidden).

use super::Handlers;
use crate::game::BGSPerk;
use engine_core::prelude::*;
use nvse::commands::Read;
use nvse::{handled, CommandArgs};

#[rustfmt::skip]
pub static COMMANDS: Handlers = &[
    (0x27be, get_perk_flag),
    (0x27bf, set_perk_flag),
];

/// Flag 0: trait; 1: playable; 2: hidden.
fn flag_field(flag: u32) -> Option<Field<BGSPerk, u8>> {
    Some(match flag {
        0 => BGSPerk::isTrait,
        1 => BGSPerk::isPlayable,
        2 => BGSPerk::isHidden,
        _ => return None,
    })
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_perk.h:
// Cmd_GetPerkFlag_Execute.
fn get_perk_flag(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if read.ok {
        let perk = Ptr::<BGSPerk>::new(read.form(0));
        if let Some(field) = flag_field(read.int(1, 0)) {
            if e.get(perk, field) != 0 {
                a.set_result(e, 1.0);
            }
        }
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_perk.h:
// Cmd_SetPerkFlag_Execute.
/// The flag set to 1 for a non-zero value, else 0 (`= (value != 0)`).
fn set_perk_flag(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if read.ok {
        let perk = Ptr::<BGSPerk>::new(read.form(0));
        if let Some(field) = flag_field(read.int(1, 0)) {
            e.set(perk, field, u8::from(read.int(2, 0) != 0));
        }
    }
    handled(true)
}

#[cfg(test)]
#[path = "jip_fn_perk_tests.rs"]
mod tests;
