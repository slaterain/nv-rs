//! xNVSE's `Commands_Quest.cpp` (xNVSE): a quest's objectives, the player's current objective
//! and current quest.
//!
//! Read from xNVSE's source and checked in the shipped DLL (xNVSE 6.4.9's `nvse_1_4.dll`,
//! executes `1003B020` to `1003B225`).

use super::misc_forms::{count, nth_item};
use super::{console_print, is_console_mode, Read};
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
/// Parameter types (xNVSE `CommandTable.h`): `kParamType_Integer`, `_Quest`.
const INT: u32 = 0x01;
const QUEST: u32 = 0x0e;
/// `kParams_OneQuest`, `kParams_QuestInteger` (xNVSE).
const ONE_QUEST: &[Param] = &[p("quest", QUEST, false)];
const QUEST_INT: &[Param] = &[p("quest", QUEST, false), p("index", INT, false)];

/// `TESQuest::lVarOrObjectives` (`+0x4C`, a `tList`; xNVSE reads each item as a
/// `BGSQuestObjective`, its `objectiveId` at `+4`). `PlayerCharacter`'s quest objective list
/// (`+0x6BC`, its first item) and current quest (`+0x6B8`); `PlayerCharacter::GetSingleton`
/// (`*0x011DEA3C`).
const QUEST_OBJECTIVES: u32 = 0x4c;
const OBJECTIVE_ID: u32 = 4;
const PLAYER: u32 = 0x011d_ea3c;
const PLAYER_OBJECTIVES: u32 = 0x6bc;
const PLAYER_QUEST: u32 = 0x6b8;

const fn cmd(
    opcode: u32,
    name: &'static str,
    short: &'static str,
    help: &'static str,
    params: &'static [Param],
    execute: AbiFn,
) -> Command {
    Command {
        opcode,
        name,
        short,
        help,
        needs_ref: false,
        params,
        execute,
        eval: None,
        returns: Returns::Default,
    }
}

#[rustfmt::skip]
pub static COMMANDS: &[Command] = &[
    cmd(0x163b, "GetQuestObjectiveCount", "GetObjectiveCount", "returns the number of objectives for the specified quest", ONE_QUEST, get_quest_objective_count),
    cmd(0x163c, "GetNthQuestObjective", "GetNthObjective", "returns the objectiveID of the Nth objective of the quest", QUEST_INT, get_nth_quest_objective),
    cmd(0x163d, "GetCurrentObjective", "GetCurObjID", "returns the objectiveID of the current objective", &[], get_current_objective),
    cmd(0x163f, "SetCurrentQuest", "", "sets the current quest to the specified quest", ONE_QUEST, set_current_quest),
];

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_Quest.cpp:
// Cmd_GetQuestObjectiveCount_Execute.
/// The count of the quest's list (`tList::Count`), unsigned; the console prints "%s has %d
/// objectives" with the quest's name (`GetFullName`: "<no name>" without one). xNVSE does not
/// look for the quest after a read that answered (`1003B058`): its reader refuses a missing form.
fn get_quest_objective_count(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    a.set_result(e, 0.0);
    let read = Read::of(e, &a);
    if !read.ok {
        return handled(true);
    }
    let quest = read.form(0);
    let n = count(e, quest + QUEST_OBJECTIVES);
    a.set_result(e, f64::from(n));
    if is_console_mode(e) {
        let name = super::misc_ref::full_name_text(e, quest);
        console_print(e, "%s has %d objectives", &[name, n]);
    }
    handled(true)
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_Quest.cpp:
// Cmd_GetNthQuestObjective_Execute.
/// The id of the item at the index (-2 the last) of the quest's list, unsigned; 0 for none.
fn get_nth_quest_objective(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    a.set_result(e, 0.0);
    let read = Read::of(e, &a);
    let quest = read.form(0);
    if read.ok && quest != 0 {
        let objective = nth_item(e, quest + QUEST_OBJECTIVES, read.int(1, 0) as i32);
        if objective != 0 {
            let id = e.mem.u32(objective + OBJECTIVE_ID);
            a.set_result(e, f64::from(id));
        }
    }
    handled(true)
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_Quest.cpp:
// Cmd_GetCurrentObjective_Execute.
/// The id of the first of the player's quest objectives, unsigned; without one the result is not
/// written (xNVSE sets no 0 first).
fn get_current_objective(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let player = e.global::<u32>(PLAYER);
    let objective = e.mem.u32(player + PLAYER_OBJECTIVES);
    if objective != 0 {
        let id = e.mem.u32(objective + OBJECTIVE_ID);
        a.set_result(e, f64::from(id));
    }
    handled(true)
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_Quest.cpp:
// Cmd_SetCurrentQuest_Execute.
/// The player's current quest set to the quest read.
fn set_current_quest(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    a.set_result(e, 0.0);
    let read = Read::of(e, &a);
    if read.ok {
        let player = e.global::<u32>(PLAYER);
        e.mem.set_u32(player + PLAYER_QUEST, read.form(0));
    }
    handled(true)
}

#[cfg(test)]
#[path = "quest_tests.rs"]
mod tests;
