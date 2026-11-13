//! `functions_jip/jip_fn_quest.h` (JIP LN NVSE): a quest's stage, flags and objectives, the
//! objectives' targets, texts and teleport links, the player's quests and active quest. All 17 of
//! its commands. The quest is read by type (`kParamType_Quest`), references as object references:
//! the reader refuses another form and the command does nothing.
//!
//! Objectives and targets are checked in their constructors (`forms::BGSQuestObjective`,
//! `forms::ObjectiveTarget`); the quest's flags and stage, the player's active quest, objectives
//! and targets in `crate::game`. The game functions JIP calls: `TESQuest::SetStage`
//! (`0060D510`), `ConditionList::Evaluate` (`00680C60`), the target's path refresh (`00952D60`,
//! `stdcall`), the target's constructor (`0060FF70`) and destructor (`005EC4D0`), the player's
//! target refresh (`0060F110`) and the quest's fail (`0060CAF0`).

use super::arrays::{append_element, create_array, Element, TempElements};
use super::forms::{
    BGSQuestObjective, ListNode, ObjectiveTarget, ObjectiveTargetData, BGS_QUEST_OBJECTIVE_VTBL,
};
use super::{
    assign_string, formatted_in_buffer, heap_alloc, is_type, list_items, list_prepend,
    list_remove_all, list_unlink, nth_item, string_set, string_text, Handlers,
};
use crate::game::{PlayerCharacter, TESQuest};
use crate::Jip;
use engine_core::prelude::*;
use nvse::commands::Read;
use nvse::{handled, CommandArgs};

#[rustfmt::skip]
pub static COMMANDS: Handlers = &[
    (0x26da, set_stage_alt),
    (0x272f, get_quest_targets_changed),
    (0x2746, clear_active_quest),
    (0x2818, get_objective_targets),
    (0x2819, set_objective_nth_target),
    (0x281a, get_objective_has_target),
    (0x281b, add_objective_target),
    (0x281c, remove_objective_target),
    (0x281d, get_objective_text),
    (0x281e, set_objective_text),
    (0x286d, get_quests),
    (0x286e, get_quest_objectives),
    (0x286f, get_active_objectives),
    (0x2870, get_objective_teleport_links),
    (0x2888, set_quest_flag),
    (0x2889, get_quest_flag),
    (0x2903, fail_quest),
];

/// What the quest commands keep between calls: `s_lastQuestTargets`, the targets
/// `GetQuestTargetsChanged` saw last (sorted, as JIP's `Set`).
#[derive(Default)]
pub struct State {
    pub last_targets: Vec<u32>,
}

const PLAYER: u32 = 0x011d_ea3c;
const SET_STAGE: u32 = 0x0060_d510;
const EVALUATE: u32 = 0x0068_0c60;
const REFRESH_PATH: u32 = 0x0095_2d60;
const TARGET_CONSTRUCT: u32 = 0x0060_ff70;
const TARGET_DESTROY: u32 = 0x005e_c4d0;
const REFRESH_TARGETS: u32 = 0x0060_f110;
const FAIL_QUEST: u32 = 0x0060_caf0;
/// `TESQuest::kFlag_Completed`.
const COMPLETED: u8 = 2;
/// `kCommandInfo_SetObjectiveText.numParams`: the quest, the index and xNVSE's 21 format
/// parameters.
const OBJECTIVE_TEXT_PARAMS: u32 = 23;
/// JIP's string argument buffer (`GetStrArgBuffer`, `STR_BUFFER_SIZE`).
const STR_BUFFER_SIZE: usize = 0x4_0000;

fn the_player(e: &Engine) -> Ptr<PlayerCharacter> {
    Ptr::new(e.mem.u32(PLAYER))
}

fn active_quest(e: &Engine) -> u32 {
    e.get(the_player(e), PlayerCharacter::activeQuest)
}

fn objective_ptr(o: u32) -> Ptr<BGSQuestObjective> {
    Ptr::new(o)
}

fn status(e: &Engine, o: u32) -> u32 {
    e.get(objective_ptr(o), BGSQuestObjective::status)
}

fn target_ref(e: &Engine, t: u32) -> u32 {
    e.get(Ptr::<ObjectiveTarget>::new(t), ObjectiveTarget::target)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), nvse/GameForms.cpp: TESQuest::GetObjective.
/// The quest's first objective (vtable `BGSQuestObjective`) with the id, 0 for none.
fn objective(e: &Engine, quest: u32, id: u32) -> u32 {
    let list = Ptr::<TESQuest>::new(quest).at(TESQuest::lVarOrObjectives);
    list_items(e, list)
        .into_iter()
        .find(|&o| {
            o != 0
                && is_type(e, o, BGS_QUEST_OBJECTIVE_VTBL)
                && e.get(objective_ptr(o), BGSQuestObjective::objectiveId) == id
        })
        .unwrap_or(0)
}

/// The quest's objective of argument 1 when the read succeeded.
fn objective_arg(e: &Engine, read: &Read) -> u32 {
    if read.ok {
        objective(e, read.form(0), read.int(1, 0))
    } else {
        0
    }
}

fn targets(o: u32) -> Ptr<ListNode> {
    objective_ptr(o).at(BGSQuestObjective::targets)
}

/// `ConditionList::Evaluate(target->target, NULL, &evalRes, 0)` (`00680C60`, `thiscall`) on the
/// target's conditions: its `bool` answer.
fn evaluate(e: &mut Engine, t: u32) -> bool {
    let conditions = Ptr::<ObjectiveTarget>::new(t).at(ObjectiveTarget::conditions);
    let run_on = target_ref(e, t);
    e.with_stack(4, |e, result| {
        e.call(
            EVALUATE,
            &args![conditions.addr(), run_on, 0u32, result.addr(), 0u32],
        )
        .u32()
            & 0xff
            != 0
    })
}

/// The targets of an objective that have a reference and whose conditions hold, in order.
fn live_targets(e: &mut Engine, o: u32) -> Vec<u32> {
    let mut live = Vec::new();
    for t in list_items(e, targets(o)) {
        if t != 0 && target_ref(e, t) != 0 && evaluate(e, t) {
            live.push(t);
        }
    }
    live
}

/// `StdCall(0x952D60, target->target, &target->data, 1)`: the target's path worked out again.
fn refresh_path(e: &mut Engine, t: u32) {
    let r = target_ref(e, t);
    let data = Ptr::<ObjectiveTarget>::new(t)
        .at(ObjectiveTarget::data)
        .addr();
    e.call(REFRESH_PATH, &args![r, data, 1u32]);
}

/// `ThisCall(0x60F110, quest, &player->questTargetList, &player->questObjectiveList)`.
fn refresh_targets(e: &mut Engine, quest: u32) {
    let p = the_player(e);
    let (targets, objectives) = (
        p.at(PlayerCharacter::questTargetList).addr(),
        p.at(PlayerCharacter::questObjectiveList).addr(),
    );
    e.call(REFRESH_TARGETS, &args![quest, targets, objectives]);
}

// ---- the quest ----

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_quest.h:
// Cmd_SetStageAlt_Execute.
/// `SetStage` with the stage's low byte; when it answers true, the current stage set to it and
/// the result 1.
fn set_stage_alt(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if !read.ok {
        return handled(true);
    }
    let (quest, stage) = (read.form(0), read.int(1, 0) as u8);
    if e.call(SET_STAGE, &args![quest, u32::from(stage)]).u32() & 0xff != 0 {
        e.set(Ptr::<TESQuest>::new(quest), TESQuest::currentStage, stage);
        a.set_result(e, 1.0);
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_quest.h:
// Cmd_GetQuestFlag_Execute.
/// Flag 0-7 of the quest's flag byte.
fn get_quest_flag(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let flag = read.int(1, 0);
    if read.ok && flag <= 7 {
        let flags = e.get(Ptr::<TESQuest>::new(read.form(0)), TESQuest::questFlags);
        if flags & (1 << flag) != 0 {
            a.set_result(e, 1.0);
        }
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_quest.h:
// Cmd_SetQuestFlag_Execute.
/// Flag 0-7 set or cleared.
fn set_quest_flag(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let flag = read.int(1, 0);
    if read.ok && flag <= 7 {
        let q = Ptr::<TESQuest>::new(read.form(0));
        let flags = e.get(q, TESQuest::questFlags);
        let bit = 1u8 << flag;
        let flags = if read.int(2, 0) != 0 {
            flags | bit
        } else {
            flags & !bit
        };
        e.set(q, TESQuest::questFlags, flags);
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_quest.h:
// Cmd_FailQuest_Execute.
/// `ThisCall(0x60CAF0, quest, 1)`.
fn fail_quest(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if read.ok {
        e.call(FAIL_QUEST, &args![read.form(0), 1u32]);
    }
    handled(true)
}

// ---- the player's quests ----

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_quest.h:
// Cmd_ClearActiveQuest_Execute.
/// No active quest, and the player's target list emptied (its later nodes freed).
fn clear_active_quest(e: &mut Engine, _w: &[u32]) -> Ret {
    let p = the_player(e);
    e.set(p, PlayerCharacter::activeQuest, 0);
    list_remove_all(e, p.at(PlayerCharacter::questTargetList));
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_quest.h:
// Cmd_GetQuests_Execute.
/// An array of the quests of the player's displayed objectives (status bit 0) that are completed
/// or not as asked, each once (sorted by address, `InsertUnique`); no array when there are none.
fn get_quests(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if !read.ok {
        return handled(true);
    }
    let completed = if read.int(0, 0) != 0 { COMPLETED } else { 0 };
    let list = the_player(e).at(PlayerCharacter::questObjectiveList);
    let mut tmp = TempElements::new();
    for o in list_items(e, list) {
        if o == 0 || status(e, o) & 1 == 0 {
            continue;
        }
        let quest = e.get(objective_ptr(o), BGSQuestObjective::quest);
        if completed == e.get(Ptr::<TESQuest>::new(quest), TESQuest::questFlags) & COMPLETED {
            tmp.insert_unique(Element::Form(quest));
        }
    }
    if !tmp.is_empty() {
        let id = create_array(e, a.script, &tmp.0);
        a.set_result(e, f64::from(id));
    }
    handled(true)
}

/// The ids (as numbers) of the quest's objectives whose status (its low two bits) is `want`.
fn objective_ids(e: &Engine, quest: u32, want: u32) -> TempElements {
    let list = Ptr::<TESQuest>::new(quest).at(TESQuest::lVarOrObjectives);
    let mut tmp = TempElements::new();
    for o in list_items(e, list) {
        if o != 0 && is_type(e, o, BGS_QUEST_OBJECTIVE_VTBL) && status(e, o) & 3 == want {
            let id = e.get(objective_ptr(o), BGSQuestObjective::objectiveId) as i32;
            tmp.append(Element::Number(f64::from(id)));
        }
    }
    tmp
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_quest.h:
// Cmd_GetQuestObjectives_Execute.
/// An array of the ids of the quest's displayed (status 1) or, asked for completed, displayed
/// and completed (status 3) objectives; no array when there are none.
fn get_quest_objectives(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if !read.ok {
        return handled(true);
    }
    let want = if read.int(1, 0) != 0 { 3 } else { 1 };
    let tmp = objective_ids(e, read.form(0), want);
    if !tmp.is_empty() {
        let id = create_array(e, a.script, &tmp.0);
        a.set_result(e, f64::from(id));
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_quest.h:
// Cmd_GetActiveObjectives_Execute.
/// The same for the player's active quest's displayed objectives; nothing without one.
fn get_active_objectives(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let quest = active_quest(e);
    if quest != 0 {
        let tmp = objective_ids(e, quest, 1);
        if !tmp.is_empty() {
            let id = create_array(e, a.script, &tmp.0);
            a.set_result(e, f64::from(id));
        }
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_quest.h:
// Cmd_GetQuestTargetsChanged_Execute.
/// The live targets' references of the player's displayed, not completed objectives of quests
/// not completed, as a set (sorted, each once); 1 when it differs from the set of the last call,
/// which it then replaces.
fn get_quest_targets_changed(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let list = the_player(e).at(PlayerCharacter::questObjectiveList);
    let mut set: Vec<u32> = Vec::new();
    for o in list_items(e, list) {
        if o == 0 {
            continue;
        }
        let quest = e.get(objective_ptr(o), BGSQuestObjective::quest);
        if e.get(Ptr::<TESQuest>::new(quest), TESQuest::questFlags) & COMPLETED != 0
            || status(e, o) & 3 != 1
            || e.get(targets(o), ListNode::data) == 0
        {
            continue;
        }
        for t in live_targets(e, o) {
            let r = target_ref(e, t);
            if let Err(at) = set.binary_search(&r) {
                set.insert(at, r);
            }
        }
    }
    let last = &mut e.ext::<Jip>().quest.last_targets;
    if *last != set {
        *last = set;
        a.set_result(e, 1.0);
    }
    handled(true)
}

// ---- objectives ----

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_quest.h:
// Cmd_GetObjectiveTargets_Execute.
/// An array of the references of a displayed objective's live targets; no array when there are
/// none.
fn get_objective_targets(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let o = objective_arg(e, &read);
    if o == 0 || status(e, o) & 3 != 1 || e.get(targets(o), ListNode::data) == 0 {
        return handled(true);
    }
    let mut tmp = TempElements::new();
    for t in live_targets(e, o) {
        tmp.append(Element::Form(target_ref(e, t)));
    }
    if !tmp.is_empty() {
        let id = create_array(e, a.script, &tmp.0);
        a.set_result(e, f64::from(id));
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_quest.h:
// Cmd_GetObjectiveTeleportLinks_Execute.
/// For a displayed objective, an array (made first, kept even when empty) with one array per
/// live target: the doors of its teleport links, then the target's reference. A quest other than
/// the active one has each target's path worked out first.
fn get_objective_teleport_links(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let o = objective_arg(e, &read);
    if o == 0 || status(e, o) & 3 != 1 {
        return handled(true);
    }
    let links = create_array(e, a.script, &[]);
    let quest = read.form(0);
    let active = active_quest(e);
    let mut tmp = TempElements::new();
    for t in list_items(e, targets(o)) {
        if t == 0 || target_ref(e, t) == 0 || !evaluate(e, t) {
            continue;
        }
        if quest != active {
            refresh_path(e, t);
        }
        let td = Ptr::<ObjectiveTarget>::new(t).at(ObjectiveTarget::data);
        let (data, size) = (
            e.get(td, ObjectiveTargetData::linksData),
            e.get(td, ObjectiveTargetData::linksSize),
        );
        for i in 0..size {
            tmp.append(Element::Form(e.mem.u32(data + 0x10 * i)));
        }
        tmp.append(Element::Form(target_ref(e, t)));
        let one = create_array(e, a.script, &tmp.0);
        append_element(e, links, &Element::Array(one));
        tmp.clear();
    }
    a.set_result(e, f64::from(links));
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_quest.h:
// Cmd_SetObjectiveNthTarget_Execute.
/// Target `index` (`GetNthItem`) gets the reference; for the active quest its path is worked out
/// again.
fn set_objective_nth_target(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let o = objective_arg(e, &read);
    if o == 0 {
        return handled(true);
    }
    let t = nth_item(e, targets(o), read.int(2, 0) as i32);
    if t != 0 {
        e.set(
            Ptr::<ObjectiveTarget>::new(t),
            ObjectiveTarget::target,
            read.form(3),
        );
        if read.form(0) == active_quest(e) {
            refresh_path(e, t);
        }
    }
    handled(true)
}

/// The first target whose reference is `r` (`Find(ObjTargetFinder(refr))`), 0 for none.
fn find_target(e: &Engine, o: u32, r: u32) -> u32 {
    list_items(e, targets(o))
        .into_iter()
        .find(|&t| t != 0 && target_ref(e, t) == r)
        .unwrap_or(0)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_quest.h:
// Cmd_GetObjectiveHasTarget_Execute.
fn get_objective_has_target(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let o = objective_arg(e, &read);
    if o != 0 && find_target(e, o, read.form(2)) != 0 {
        a.set_result(e, 1.0);
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_quest.h:
// Cmd_AddObjectiveTarget_Execute.
/// A reference not among the objective's targets yet: a new target (0x48 bytes of the game's
/// heap, the game's constructor) with it, put first; for the active quest the player's targets
/// are worked out again.
fn add_objective_target(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let o = objective_arg(e, &read);
    let r = read.form(2);
    if o == 0 || find_target(e, o, r) != 0 {
        return handled(true);
    }
    let block = heap_alloc(e, ObjectiveTarget::SIZE);
    let t = e.call(TARGET_CONSTRUCT, &args![block]).u32();
    e.set(Ptr::<ObjectiveTarget>::new(t), ObjectiveTarget::target, r);
    list_prepend(e, targets(o), t);
    let quest = read.form(0);
    if quest == active_quest(e) {
        refresh_targets(e, quest);
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_quest.h:
// Cmd_RemoveObjectiveTarget_Execute.
/// The first target with the reference taken out of the list (`RemoveIf`: a later node freed,
/// the first takes the next one's item) and destroyed (`ThisCall(0x5EC4D0, target, 1)`); for
/// the active quest the player's targets are worked out again.
fn remove_objective_target(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let o = objective_arg(e, &read);
    if o == 0 {
        return handled(true);
    }
    let r = read.form(2);
    let (mut node, mut prev) = (targets(o), Ptr::<ListNode>::NULL);
    loop {
        let t = e.get(node, ListNode::data);
        if t != 0 && target_ref(e, t) == r {
            list_unlink(e, node, prev);
            e.call(TARGET_DESTROY, &args![t, 1u32]);
            let quest = read.form(0);
            if quest == active_quest(e) {
                refresh_targets(e, quest);
            }
            break;
        }
        prev = node;
        node = Ptr::new(e.get(node, ListNode::next));
        if node.is_null() {
            break;
        }
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_quest.h:
// Cmd_GetObjectiveText_Execute.
fn get_objective_text(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let o = objective_arg(e, &read);
    let text = if o == 0 {
        None
    } else {
        string_text(
            e,
            objective_ptr(o).at(BGSQuestObjective::displayText).addr(),
        )
    };
    assign_string(e, &a, text);
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_quest.h:
// Cmd_SetObjectiveText_Execute.
/// The objective's text set to the formatted text (xNVSE's `ExtractFormatStringArgs`, the
/// format string third, into JIP's string argument buffer).
fn set_objective_text(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let f = nvse::commands::string::format_string_args_at(e, &a, 2, OBJECTIVE_TEXT_PARAMS);
    if !f.ok {
        return handled(true);
    }
    let text = formatted_in_buffer(e, &f.text, STR_BUFFER_SIZE, "SetObjectiveText");
    let quest = f
        .leading
        .first()
        .and_then(nvse::args::Arg::form)
        .unwrap_or(0);
    let id = f
        .leading
        .get(1)
        .and_then(nvse::args::Arg::int)
        .map_or(0, |v| v as u32);
    let o = objective(e, quest, id);
    if o != 0 {
        string_set(
            e,
            objective_ptr(o).at(BGSQuestObjective::displayText).addr(),
            &text,
        );
    }
    handled(true)
}

#[cfg(test)]
#[path = "jip_fn_quest_tests.rs"]
mod tests;
