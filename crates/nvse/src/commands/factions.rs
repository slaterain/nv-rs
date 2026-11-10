//! xNVSE's `Commands_Factions.cpp` (xNVSE): an actor base's factions by number, and a faction's
//! count of ranks. (`GetFactionRank` and `ModFactionRank` there are the game's own commands'
//! replacements, not registered by xNVSE 6.4.9.)
//!
//! Read from xNVSE's source and checked in the shipped DLL (xNVSE 6.4.9's `nvse_1_4.dll`,
//! executes `10024F00` to `10025246`).

use super::misc::convert;
use super::misc_forms::{count, nth_item};
use super::{rtti, Read};
use crate::table::{Command, Param, Returns};
use crate::{handled, CommandArgs};
use engine_core::prelude::*;

const fn p(name: &'static str, kind: u32, optional: bool) -> Param {
    Param {
        name,
        kind,
        optional,
    }
}
/// Parameter types (xNVSE `CommandTable.h`): `kParamType_Integer`, `_Faction`, `_ActorBase`.
const INT: u32 = 0x01;
const FACTION: u32 = 0x11;
const ACTOR_BASE: u32 = 0x19;
/// `kParams_OneOptionalActorBase`, `kParams_OneIntOneOptionalActorBase`, `kParams_OneFaction`
/// (xNVSE), as the DLL's records hold them.
const OPTIONAL_ACTOR_BASE: &[Param] = &[p("base actor", ACTOR_BASE, true)];
const INT_OPTIONAL_ACTOR_BASE: &[Param] =
    &[p("bool", INT, false), p("base actor", ACTOR_BASE, true)];
const ONE_FACTION: &[Param] = &[p("faction", FACTION, false)];

/// `TESForm::refID` (`+0xC`). `TESActorBase`'s faction list (`+0x5C`, its actor base data's
/// `+0x2C`: `005D8A70` answers `this + 0x2C`; each entry a faction `+0` and a rank byte `+4`).
/// `TESFaction`'s ranks (`+0x3C`).
const FORM_ID: u32 = 0xc;
const ACTOR_BASE_FACTIONS: u32 = 0x5c;
const FACTION_RANKS: u32 = 0x3c;

const fn cmd(
    opcode: u32,
    name: &'static str,
    help: &'static str,
    params: &'static [Param],
    execute: AbiFn,
    returns: Returns,
) -> Command {
    Command {
        opcode,
        name,
        short: "",
        help,
        needs_ref: false,
        params,
        execute,
        eval: None,
        returns,
    }
}

use Returns::{Default as RD, Form as RF};

#[rustfmt::skip]
pub static COMMANDS: &[Command] = &[
    cmd(0x15c9, "GetBaseNumFactions", "returns the number of factions to which an actor base form belongs", OPTIONAL_ACTOR_BASE, get_base_num_factions, RD),
    cmd(0x15ca, "GetBaseNthFaction", "returns the nth faction to which an actor base form belongs", INT_OPTIONAL_ACTOR_BASE, get_base_nth_faction, RF),
    cmd(0x15cb, "GetBaseNthRank", "returns the nth faction rank to which an actor base form belongs", INT_OPTIONAL_ACTOR_BASE, get_base_nth_rank, RD),
    cmd(0x15cc, "GetNumRanks", "returns the number of rank of a faction", ONE_FACTION, get_num_ranks, RD),
];

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_Factions.cpp:
// Cmd_GetBaseNumFactions_Execute.
/// The actor base's count of factions (`ExtractActorBase`: the form given, a reference's base
/// form, else the calling reference's base form; the read's answer not looked at), unsigned.
fn get_base_num_factions(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    a.set_result(e, 0.0);
    let read = Read::of(e, &a);
    let base = convert(e, a.this_obj, read.form(0), rtti::TES_ACTOR_BASE);
    if base != 0 {
        let n = count(e, base + ACTOR_BASE_FACTIONS);
        a.set_result(e, f64::from(n));
    }
    handled(true)
}

/// `ExtractSetActorBase` (xNVSE `Commands_Misc.cpp`): the index (0 when not read) and the actor
/// base after it, then the base's entry at that index (`tList::GetNthItem`: -2 the last), 0 for
/// none or for an entry without a faction.
fn nth_entry(e: &mut Engine, a: &CommandArgs) -> u32 {
    let read = Read::of(e, a);
    let index = read.int(0, 0) as i32;
    let base = convert(e, a.this_obj, read.form(1), rtti::TES_ACTOR_BASE);
    if base == 0 {
        return 0;
    }
    let entry = nth_item(e, base + ACTOR_BASE_FACTIONS, index);
    if entry != 0 && e.mem.u32(entry) != 0 {
        entry
    } else {
        0
    }
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_Factions.cpp:
// Cmd_GetBaseNthFaction_Execute.
/// The nth faction's id in the low word, 0 for none.
fn get_base_nth_faction(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    a.set_form_result(e, 0);
    let entry = nth_entry(e, &a);
    if entry != 0 {
        let id = e.mem.u32(e.mem.u32(entry) + FORM_ID);
        a.set_form_result(e, id);
    }
    handled(true)
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_Factions.cpp:
// Cmd_GetBaseNthRank_Execute.
/// The nth faction's rank, the byte read unsigned (`100251B3`); the result's low word 0 for none
/// (its high word untouched, as xNVSE writes `*(UInt32 *)result = 0`).
fn get_base_nth_rank(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    a.set_form_result(e, 0);
    let entry = nth_entry(e, &a);
    if entry != 0 {
        let rank = e.mem.u8(entry + 4);
        a.set_result(e, f64::from(rank));
    }
    handled(true)
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_Factions.cpp:
// Cmd_GetNumRanks_Execute.
/// The faction's count of ranks, unsigned. xNVSE does not look for the faction after a read that
/// answered (`10025205`): its reader refuses a missing form, so one is always there.
fn get_num_ranks(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    a.set_result(e, 0.0);
    let read = Read::of(e, &a);
    if read.ok {
        let faction = read.form(0);
        let n = count(e, faction + FACTION_RANKS);
        a.set_result(e, f64::from(n));
    }
    handled(true)
}

#[cfg(test)]
#[path = "factions_tests.rs"]
mod tests;
